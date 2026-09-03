//! TrollStore logical sessions over Wawona's in-process PTY substrate.

use std::collections::BTreeMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use wwn_igetty_core::{
    DisplayBackend, InputBackend, Session, SessionKind, SessionProvider, SessionSwitcher,
    SwitchError,
};

#[repr(C)]
struct Winsize {
    rows: u16,
    cols: u16,
    xpixel: u16,
    ypixel: u16,
}

#[repr(C)]
struct PtySession(c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WwnIgettyIosCallbacks {
    pub context: *mut c_void,
    pub present_session: Option<extern "C" fn(*mut c_void, u32, u8, *const c_char) -> c_int>,
}

unsafe impl Send for WwnIgettyIosCallbacks {}

extern "C" {
    fn wwn_pty_session_start(
        shell_path: *const c_char,
        argv: *const *mut c_char,
        envp: *const *mut c_char,
        winsize: *const Winsize,
    ) -> *mut PtySession;
    fn wwn_pty_session_master_fd(session: *const PtySession) -> c_int;
    fn wwn_pty_session_destroy(session: *mut PtySession);
    fn wwn_ios_terminal_set_master(master_fd: c_int);
    fn wwn_ios_terminal_clear_master(master_fd: c_int);
    fn wwn_pty_ios_stop_shell_session();
}

struct IosSession {
    pty: usize,
    master_fd: c_int,
}

struct IosBackend {
    callbacks: WwnIgettyIosCallbacks,
    sessions: BTreeMap<u32, IosSession>,
}

impl SessionProvider for IosBackend {
    fn activate(&mut self, session: &Session) -> Result<(), SwitchError> {
        if session.kind == SessionKind::Text {
            let pty = self
                .sessions
                .get(&session.id)
                .ok_or(SwitchError::ProviderRejected)?;
            unsafe { wwn_ios_terminal_set_master(pty.master_fd) };
        }
        Ok(())
    }

    fn deactivate(&mut self, session: &Session) -> Result<(), SwitchError> {
        if let Some(pty) = self.sessions.get(&session.id) {
            unsafe { wwn_ios_terminal_clear_master(pty.master_fd) };
        }
        Ok(())
    }
}

impl DisplayBackend for IosBackend {
    fn present(&mut self, session: &Session) -> Result<(), SwitchError> {
        let Some(callback) = self.callbacks.present_session else {
            return Err(SwitchError::DisplayRejected);
        };
        let label =
            CString::new(session.label.as_str()).map_err(|_| SwitchError::DisplayRejected)?;
        let rc = callback(
            self.callbacks.context,
            session.id,
            session.kind as u8,
            label.as_ptr(),
        );
        (rc == 0).then_some(()).ok_or(SwitchError::DisplayRejected)
    }
}

struct NoopInput;

impl InputBackend for NoopInput {
    fn focus(&mut self, _session: &Session) -> Result<(), SwitchError> {
        Ok(())
    }
}

struct Broker {
    switcher: SessionSwitcher,
    backend: IosBackend,
    next_id: u32,
}

impl Broker {
    fn new(callbacks: WwnIgettyIosCallbacks) -> Self {
        Self {
            switcher: SessionSwitcher::new("Machines"),
            backend: IosBackend {
                callbacks,
                sessions: BTreeMap::new(),
            },
            next_id: 1,
        }
    }
}

static BROKER: OnceLock<Mutex<Option<Broker>>> = OnceLock::new();

fn broker() -> &'static Mutex<Option<Broker>> {
    BROKER.get_or_init(|| Mutex::new(None))
}

fn label_from_ptr(label: *const c_char) -> Option<String> {
    if label.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(label) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

#[no_mangle]
pub extern "C" fn wwn_igetty_ios_initialize(callbacks: WwnIgettyIosCallbacks) -> c_int {
    let mut guard = broker().lock().expect("igetty broker lock");
    *guard = Some(Broker::new(callbacks));
    0
}

#[no_mangle]
pub extern "C" fn wwn_igetty_ios_register_session(kind: u8, label: *const c_char) -> u32 {
    let Some(kind) = session_kind(kind) else {
        return 0;
    };
    let Some(label) = label_from_ptr(label) else {
        return 0;
    };
    let mut guard = broker().lock().expect("igetty broker lock");
    let Some(broker) = guard.as_mut() else {
        return 0;
    };
    let id = broker.next_id;
    broker.next_id = broker.next_id.saturating_add(1);
    broker.switcher.register(Session { id, kind, label });
    id
}

#[no_mangle]
pub extern "C" fn wwn_igetty_ios_spawn_text_session(
    shell_path: *const c_char,
    label: *const c_char,
    rows: u16,
    cols: u16,
) -> u32 {
    if shell_path.is_null() {
        return 0;
    }
    let Some(label) = label_from_ptr(label) else {
        return 0;
    };
    let argv0 = unsafe { CStr::from_ptr(shell_path) }.as_ptr() as *mut c_char;
    let login = c"-l".as_ptr() as *mut c_char;
    let argv = [argv0, login, ptr::null_mut()];
    let winsize = Winsize {
        rows: rows.max(1),
        cols: cols.max(1),
        xpixel: 0,
        ypixel: 0,
    };
    let pty = unsafe { wwn_pty_session_start(shell_path, argv.as_ptr(), ptr::null(), &winsize) };
    if pty.is_null() {
        return 0;
    }
    let master_fd = unsafe { wwn_pty_session_master_fd(pty) };
    if master_fd < 0 {
        unsafe { wwn_pty_session_destroy(pty) };
        return 0;
    }

    let mut guard = broker().lock().expect("igetty broker lock");
    let Some(broker) = guard.as_mut() else {
        unsafe { wwn_pty_session_destroy(pty) };
        return 0;
    };
    let id = broker.next_id;
    broker.next_id = broker.next_id.saturating_add(1);
    broker.switcher.register(Session {
        id,
        kind: SessionKind::Text,
        label,
    });
    broker.backend.sessions.insert(
        id,
        IosSession {
            pty: pty as usize,
            master_fd,
        },
    );
    id
}

#[no_mangle]
pub extern "C" fn wwn_igetty_ios_switch_to(id: u32) -> c_int {
    let mut guard = broker().lock().expect("igetty broker lock");
    let Some(broker) = guard.as_mut() else {
        return -1;
    };
    let result = (|| {
        let plan = broker.switcher.plan_switch(id)?;
        if plan.previous == Some(id) {
            return Ok(());
        }
        if let Some(previous) = plan
            .previous
            .and_then(|previous| broker.switcher.sessions().find(|s| s.id == previous))
            .cloned()
        {
            broker.backend.deactivate(&previous)?;
        }
        let next = broker
            .switcher
            .sessions()
            .find(|session| session.id == id)
            .cloned()
            .ok_or(SwitchError::UnknownSession)?;
        broker.backend.activate(&next)?;
        broker.backend.present(&next)?;
        let mut input = NoopInput;
        input.focus(&next)?;
        broker.switcher.commit(plan);
        Ok::<(), SwitchError>(())
    })();
    result.map_or_else(|error| -(error as c_int) - 1, |_| 0)
}

#[no_mangle]
pub extern "C" fn wwn_igetty_ios_unregister_session(id: u32) {
    let mut guard = broker().lock().expect("igetty broker lock");
    let Some(broker) = guard.as_mut() else {
        return;
    };
    if let Some(session) = broker.backend.sessions.remove(&id) {
        unsafe { wwn_pty_session_destroy(session.pty as *mut PtySession) };
    }
    broker.switcher.unregister(id);
}

#[no_mangle]
pub extern "C" fn wwn_igetty_ios_active_session() -> u32 {
    broker()
        .lock()
        .expect("igetty broker lock")
        .as_ref()
        .and_then(|broker| broker.switcher.active())
        .map_or(u32::MAX, |session| session.id)
}

#[no_mangle]
pub extern "C" fn wwn_igetty_ios_shutdown() {
    let mut guard = broker().lock().expect("igetty broker lock");
    if let Some(mut broker) = guard.take() {
        for (_, session) in std::mem::take(&mut broker.backend.sessions) {
            unsafe { wwn_pty_session_destroy(session.pty as *mut PtySession) };
        }
    }
    unsafe { wwn_pty_ios_stop_shell_session() };
}

fn session_kind(kind: u8) -> Option<SessionKind> {
    match kind {
        0 => Some(SessionKind::Greeter),
        1 => Some(SessionKind::Text),
        2 => Some(SessionKind::Native),
        3 => Some(SessionKind::VirtualMachine),
        4 => Some(SessionKind::Container),
        5 => Some(SessionKind::Compositor),
        _ => None,
    }
}
