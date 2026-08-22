//! wwn-igetty: Linux-shaped VTs on iland DRM after WindowServer is gone.
//! Draw: paint every cell, overlay cursor, one flip. GUI VT is assigned, not hardcoded.
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::io::{self, Write};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

const VT_TEXT_COUNT: usize = 6;
const COLS_MAX: i32 = 512;
const ROWS_MAX: i32 = 256;
const MODEB_MAX_OUTPUTS: usize = 8;
const VT_FILE: &str = "/tmp/libwayland-support/modeb-vt";
const DRM_MODE_CONNECTED: u32 = 1;

static RUN: AtomicBool = AtomicBool::new(true);
static ACTIVE_VT: AtomicI32 = AtomicI32::new(1);
static CURSOR_ON: AtomicBool = AtomicBool::new(true);
static CTRL: AtomicI32 = AtomicI32::new(0);
static ALT: AtomicI32 = AtomicI32::new(0);
static SHIFT: AtomicI32 = AtomicI32::new(0);
static CAPS: AtomicI32 = AtomicI32::new(0);
static GUI_VT: AtomicI32 = AtomicI32::new(0);

struct Output {
    crtc_id: u32,
    conn_id: u32,
    fb_id: u32,
    pitch: u32,
    fb: *mut u32,
    mode: DrmModeModeInfo,
    w: i32,
    h: i32,
}

struct Vt {
    master: i32,
    shell_pid: i32,
    vt: *mut c_void,
    screen: *mut c_void,
    cols: i32,
    rows: i32,
    dirty: bool,
}

struct App {
    drm_fd: i32,
    outs: Vec<Output>,
    vts: [Vt; VT_TEXT_COUNT],
    cell_w: i32,
    cell_h: i32,
    gfx_pid: i32,
    gui_argv: Vec<String>,
    getty: String,
}

unsafe impl Send for App {}
unsafe impl Sync for App {}

static mut APP: *mut App = ptr::null_mut();

#[repr(C)]
#[derive(Clone, Copy)]
struct DrmModeModeInfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    type_: u32,
    name: [u8; 32],
}

#[repr(C)]
struct DrmModeRes {
    count_fbs: i32,
    fbs: *mut u32,
    count_crtcs: i32,
    crtcs: *mut u32,
    count_connectors: i32,
    connectors: *mut u32,
    count_encoders: i32,
    encoders: *mut u32,
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
}

#[repr(C)]
struct DrmModeEncoder {
    encoder_id: u32,
    encoder_type: u32,
    crtc_id: u32,
    possible_crtcs: u32,
    possible_clones: u32,
}

#[repr(C)]
struct DrmModeConnector {
    connector_id: u32,
    encoder_id: u32,
    connector_type: u32,
    connector_type_id: u32,
    connection: u32,
    mm_width: u32,
    mm_height: u32,
    subpixel: u32,
    count_modes: i32,
    modes: *mut DrmModeModeInfo,
    count_props: i32,
    props: *mut u32,
    prop_values: *mut u64,
    count_encoders: i32,
    encoders: *mut u32,
}

#[repr(C)]
struct VTermPos {
    row: i32,
    col: i32,
}

#[repr(C)]
struct VTermRect {
    start_row: i32,
    end_row: i32,
    start_col: i32,
    end_col: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VTermColor {
    data: [u8; 4],
}

#[repr(C)]
struct VTermScreenCallbacks {
    damage: Option<extern "C" fn(VTermRect, *mut c_void) -> i32>,
    moverect: Option<extern "C" fn(VTermRect, VTermRect, *mut c_void) -> i32>,
    movecursor: Option<extern "C" fn(VTermPos, VTermPos, i32, *mut c_void) -> i32>,
    settermprop: Option<extern "C" fn(i32, *mut c_void, *mut c_void) -> i32>,
    bell: Option<extern "C" fn(*mut c_void) -> i32>,
    resize: Option<extern "C" fn(i32, i32, *mut c_void) -> i32>,
    sb_pushline: Option<extern "C" fn(i32, *const c_void, *mut c_void) -> i32>,
    sb_popline: Option<extern "C" fn(i32, *mut c_void, *mut c_void) -> i32>,
    sb_clear: Option<extern "C" fn(*mut c_void) -> i32>,
}

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Pollfd {
    fd: i32,
    events: i16,
    revents: i16,
}

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

extern "C" {
    fn drmOpen(name: *const c_char, busid: *const c_char) -> i32;
    fn drmModeGetResources(fd: i32) -> *mut DrmModeRes;
    fn drmModeFreeResources(ptr: *mut DrmModeRes);
    fn drmModeGetConnector(fd: i32, id: u32) -> *mut DrmModeConnector;
    fn drmModeFreeConnector(ptr: *mut DrmModeConnector);
    fn drmModeGetEncoder(fd: i32, id: u32) -> *mut DrmModeEncoder;
    fn drmModeFreeEncoder(ptr: *mut DrmModeEncoder);
    fn drmModeSetCrtc(
        fd: i32,
        crtc_id: u32,
        fb_id: u32,
        x: u32,
        y: u32,
        connectors: *const u32,
        count: i32,
        mode: *mut DrmModeModeInfo,
    ) -> i32;
    fn drmModePageFlip(fd: i32, crtc_id: u32, fb_id: u32, flags: u32, user: *mut c_void) -> i32;
    fn drmModeCreateDumbBuffer(
        fd: i32,
        width: u32,
        height: u32,
        bpp: u32,
        flags: u32,
        handle: *mut u32,
        pitch: *mut u32,
        size: *mut u64,
    ) -> i32;
    fn drmModeMapDumbBuffer(fd: i32, handle: u32, offset: *mut u64) -> i32;
    fn drmModeAddFB(
        fd: i32,
        width: u32,
        height: u32,
        depth: u8,
        bpp: u8,
        pitch: u32,
        bo_handle: u32,
        buf_id: *mut u32,
    ) -> i32;
    fn vterm_new(rows: i32, cols: i32) -> *mut c_void;
    fn vterm_free(vt: *mut c_void);
    fn vterm_set_utf8(vt: *mut c_void, is_utf8: i32);
    fn vterm_obtain_screen(vt: *mut c_void) -> *mut c_void;
    fn vterm_obtain_state(vt: *mut c_void) -> *mut c_void;
    fn vterm_input_write(vt: *mut c_void, bytes: *const u8, len: usize) -> usize;
    fn vterm_screen_set_callbacks(
        screen: *mut c_void,
        cbs: *const VTermScreenCallbacks,
        user: *mut c_void,
    );
    fn vterm_screen_set_damage_merge(screen: *mut c_void, size: i32);
    fn vterm_screen_enable_altscreen(screen: *mut c_void, altscreen: i32);
    fn vterm_screen_set_default_colors(
        screen: *mut c_void,
        fg: *const VTermColor,
        bg: *const VTermColor,
    );
    fn vterm_screen_reset(screen: *mut c_void, hard: i32);
    fn vterm_screen_flush_damage(screen: *mut c_void);
    fn vterm_state_get_cursorpos(state: *mut c_void, pos: *mut VTermPos);
    fn modeb_ctfont_init(ttf_path: *const c_char, pt_size: f32) -> i32;
    fn modeb_ctfont_cell_size(w: *mut i32, h: *mut i32);
    fn modeb_ctfont_ready() -> i32;
    fn modeb_ctfont_blit(
        fb: *mut u32,
        pitch_bytes: u32,
        fb_w: i32,
        fb_h: i32,
        px: i32,
        py: i32,
        cp: u32,
        fg_bgra: u32,
        bg_bgra: u32,
    );
    fn modeb_vterm_cell(
        screen: *mut c_void,
        row: i32,
        col: i32,
        cp: *mut u32,
        reverse: *mut i32,
        bold: *mut i32,
        fg_rgb: *mut u8,
        bg_rgb: *mut u8,
    ) -> i32;
    fn modeb_input_subscribe() -> i32;
    fn modeb_input_thread(arg: *mut c_void) -> *mut c_void;
    fn openpty(amaster: *mut i32, aslave: *mut i32, name: *mut c_char, termp: *mut c_void, winp: *mut c_void)
        -> i32;
    fn ioctl(fd: i32, req: u64, arg: *mut c_void) -> i32;
    fn fork() -> i32;
    fn setsid() -> i32;
    fn dup2(old: i32, new: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn execl(path: *const c_char, arg0: *const c_char, ...) -> i32;
    fn execvp(file: *const c_char, argv: *const *const c_char) -> i32;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: i32) -> i32;
    fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, n: usize) -> isize;
    fn write(fd: i32, buf: *const u8, n: usize) -> isize;
    fn poll(fds: *mut Pollfd, nfds: u32, timeout: i32) -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
    fn clock_gettime(clk: i32, tp: *mut Timespec) -> i32;
    fn pthread_create(
        thread: *mut usize,
        attr: *const c_void,
        start: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        arg: *mut c_void,
    ) -> i32;
    fn access(path: *const c_char, mode: i32) -> i32;
    fn _NSGetExecutablePath(buf: *mut c_char, bufsize: *mut u32) -> i32;
    fn getenv(name: *const c_char) -> *const c_char;
    fn mkdir(path: *const c_char, mode: u16) -> i32;
    fn signal(sig: i32, handler: usize) -> usize;
    fn open(path: *const c_char, oflag: i32, ...) -> i32;
}

const SIGTERM: i32 = 15;
const SIGINT: i32 = 2;
const SIGHUP: i32 = 1;
const SIGCHLD: i32 = 20;
const SIG_DFL: usize = 0;
const TIOCSWINSZ: u64 = 0x80087467;
const TIOCSCTTY: u64 = 0x20007461;
const F_SETFL: i32 = 4;
const O_NONBLOCK: i32 = 4;
const O_RDWR: i32 = 2;
const O_CLOEXEC: i32 = 0x01000000;
const O_WRONLY: i32 = 1;
const O_CREAT: i32 = 0x0200;
const O_TRUNC: i32 = 0x0400;
const POLLIN: i16 = 0x0001;
const WNOHANG: i32 = 1;
const CLOCK_MONOTONIC: i32 = 6;
const R_OK: i32 = 4;
const X_OK: i32 = 1;
const VTERM_DAMAGE_SCREEN: i32 = 2;

extern "C" fn on_signal(_sig: i32) {
    RUN.store(false, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn modeb_rs_should_run() -> i32 {
    RUN.load(Ordering::SeqCst) as i32
}

extern "C" fn screen_damage(_r: VTermRect, user: *mut c_void) -> i32 {
    unsafe {
        (*(user as *mut Vt)).dirty = true;
    }
    1
}
extern "C" fn screen_moverect(_d: VTermRect, _s: VTermRect, user: *mut c_void) -> i32 {
    unsafe {
        (*(user as *mut Vt)).dirty = true;
    }
    1
}
extern "C" fn screen_movecursor(_p: VTermPos, _o: VTermPos, _v: i32, user: *mut c_void) -> i32 {
    unsafe {
        (*(user as *mut Vt)).dirty = true;
    }
    1
}
extern "C" fn screen_settermprop(_p: i32, _v: *mut c_void, _u: *mut c_void) -> i32 {
    1
}
extern "C" fn screen_bell(_u: *mut c_void) -> i32 {
    1
}
extern "C" fn screen_resize(_r: i32, _c: i32, _u: *mut c_void) -> i32 {
    0
}
extern "C" fn screen_sb_pushline(_c: i32, _cells: *const c_void, _u: *mut c_void) -> i32 {
    1
}
extern "C" fn screen_sb_popline(_c: i32, _cells: *mut c_void, _u: *mut c_void) -> i32 {
    0
}
extern "C" fn screen_sb_clear(_u: *mut c_void) -> i32 {
    1
}

static SCREEN_CBS: VTermScreenCallbacks = VTermScreenCallbacks {
    damage: Some(screen_damage),
    moverect: Some(screen_moverect),
    movecursor: Some(screen_movecursor),
    settermprop: Some(screen_settermprop),
    bell: Some(screen_bell),
    resize: Some(screen_resize),
    sb_pushline: Some(screen_sb_pushline),
    sb_popline: Some(screen_sb_popline),
    sb_clear: Some(screen_sb_clear),
};

fn eprint(s: &str) {
    let _ = io::stderr().write_all(s.as_bytes());
}

fn rgb_bgra(r: u8, g: u8, b: u8) -> u32 {
    0xFF000000 | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}

unsafe fn fill_fb(o: &Output, color: u32) {
    if o.fb.is_null() {
        return;
    }
    let pitch_px = (o.pitch / 4) as isize;
    for y in 0..o.h as isize {
        let row = o.fb.offset(y * pitch_px);
        for x in 0..o.w as isize {
            *row.offset(x) = color;
        }
    }
}

unsafe fn present(app: &App) {
    for o in &app.outs {
        if drmModePageFlip(app.drm_fd, o.crtc_id, o.fb_id, 0, ptr::null_mut()) != 0 {
            let conn = [o.conn_id];
            let mut mode = o.mode;
            drmModeSetCrtc(
                app.drm_fd,
                o.crtc_id,
                o.fb_id,
                0,
                0,
                conn.as_ptr(),
                1,
                &mut mode,
            );
        }
    }
}

unsafe fn bind_text_crtcs(app: &App) {
    for o in &app.outs {
        let conn = [o.conn_id];
        let mut mode = o.mode;
        if drmModeSetCrtc(
            app.drm_fd,
            o.crtc_id,
            o.fb_id,
            0,
            0,
            conn.as_ptr(),
            1,
            &mut mode,
        ) != 0
        {
            eprint("[igettyd] SetCrtc(text) failed\n");
        }
    }
}

unsafe fn draw_cell(app: &App, o: &Output, v: &Vt, row: i32, col: i32) {
    let mut cp = 0u32;
    let mut reverse = 0i32;
    let mut bold = 0i32;
    let mut fg = [0u8; 3];
    let mut bg = [0u8; 3];
    if modeb_vterm_cell(
        v.screen,
        row,
        col,
        &mut cp,
        &mut reverse,
        &mut bold,
        fg.as_mut_ptr(),
        bg.as_mut_ptr(),
    ) != 0
    {
        return;
    }
    if reverse != 0 {
        std::mem::swap(&mut fg, &mut bg);
    }
    let mut fgc = rgb_bgra(fg[0], fg[1], fg[2]);
    let bgc = rgb_bgra(bg[0], bg[1], bg[2]);
    if bold != 0 {
        let bump = |c: u8| -> u8 { if c > 200 { 255 } else { c.saturating_add(55) } };
        fgc = rgb_bgra(bump(fg[0]), bump(fg[1]), bump(fg[2]));
    }
    let px = col * app.cell_w;
    let py = row * app.cell_h;
    if modeb_ctfont_ready() != 0 {
        modeb_ctfont_blit(o.fb, o.pitch, o.w, o.h, px, py, cp, fgc, bgc);
    }
}

unsafe fn overlay_cursor(app: &App, o: &Output, v: &Vt) {
    if !CURSOR_ON.load(Ordering::Relaxed) || v.vt.is_null() {
        return;
    }
    let st = vterm_obtain_state(v.vt);
    let mut cur = VTermPos { row: 0, col: 0 };
    if !st.is_null() {
        vterm_state_get_cursorpos(st, &mut cur);
    }
    if cur.row < 0 || cur.row >= v.rows || cur.col < 0 || cur.col >= v.cols {
        return;
    }
    let px = cur.col * app.cell_w;
    let py = cur.row * app.cell_h;
    let pitch_px = (o.pitch / 4) as i32;
    let block = 0xFFE8E8E8u32;
    for y in 0..app.cell_h {
        let Y = py + y;
        if Y < 0 || Y >= o.h {
            continue;
        }
        for x in 0..app.cell_w {
            let X = px + x;
            if X < 0 || X >= o.w {
                continue;
            }
            *o.fb.offset((Y * pitch_px + X) as isize) = block;
        }
    }
}

unsafe fn render_active_text(app: &mut App) {
    let av = ACTIVE_VT.load(Ordering::SeqCst);
    if av < 1 || av > VT_TEXT_COUNT as i32 {
        return;
    }
    let idx = (av - 1) as usize;
    let v = &app.vts[idx];
    if v.screen.is_null() {
        return;
    }
    for oi in 0..app.outs.len() {
        let o = &app.outs[oi];
        for row in 0..v.rows {
            for col in 0..v.cols {
                draw_cell(app, o, v, row, col);
            }
        }
        overlay_cursor(app, o, v);
    }
    present(app);
    app.vts[idx].dirty = false;
}

unsafe fn add_output(app: &mut App, res: *mut DrmModeRes, conn: *mut DrmModeConnector, mut used: u32) -> u32 {
    if app.outs.len() >= MODEB_MAX_OUTPUTS || conn.is_null() || (*conn).count_modes < 1 {
        return used;
    }
    let mode = *(*conn).modes;
    let mut crtc = 0u32;
    if (*conn).encoder_id != 0 {
        let enc = drmModeGetEncoder(app.drm_fd, (*conn).encoder_id);
        if !enc.is_null() {
            crtc = (*enc).crtc_id;
            drmModeFreeEncoder(enc);
        }
    }
    if crtc != 0 {
        let mut taken = false;
        for i in 0..(*res).count_crtcs {
            let c = *(*res).crtcs.offset(i as isize);
            if c == crtc && (used & (1 << i)) != 0 {
                taken = true;
            }
        }
        if taken {
            crtc = 0;
        }
    }
    if crtc == 0 {
        for i in 0..(*res).count_crtcs {
            if (used & (1 << i)) != 0 {
                continue;
            }
            crtc = *(*res).crtcs.offset(i as isize);
            break;
        }
    }
    if crtc == 0 {
        return used;
    }
    let w = mode.hdisplay as i32;
    let h = mode.vdisplay as i32;
    let mut handle = 0u32;
    let mut pitch = 0u32;
    let mut size = 0u64;
    if drmModeCreateDumbBuffer(app.drm_fd, w as u32, h as u32, 32, 0, &mut handle, &mut pitch, &mut size)
        != 0
    {
        eprint("[igettyd] CreateDumbBuffer failed\n");
        return used;
    }
    let mut map_off = 0u64;
    if drmModeMapDumbBuffer(app.drm_fd, handle, &mut map_off) != 0 {
        eprint("[igettyd] MapDumbBuffer failed\n");
        return used;
    }
    let fb = map_off as *mut u32;
    if fb.is_null() {
        return used;
    }
    let mut fb_id = 0u32;
    if drmModeAddFB(app.drm_fd, w as u32, h as u32, 24, 32, pitch, handle, &mut fb_id) != 0 {
        eprint("[igettyd] AddFB failed\n");
        return used;
    }
    let conn_id = (*conn).connector_id;
    let connectors = [conn_id];
    let mut mode_mut = mode;
    if drmModeSetCrtc(app.drm_fd, crtc, fb_id, 0, 0, connectors.as_ptr(), 1, &mut mode_mut) != 0 {
        eprint("[igettyd] SetCrtc failed\n");
        return used;
    }
    for i in 0..(*res).count_crtcs {
        if *(*res).crtcs.offset(i as isize) == crtc {
            used |= 1 << i;
        }
    }
    eprint(&format!(
        "[igettyd] output {} {}x{} crtc={} conn={}\n",
        app.outs.len(),
        w,
        h,
        crtc,
        conn_id
    ));
    app.outs.push(Output {
        crtc_id: crtc,
        conn_id,
        fb_id,
        pitch,
        fb,
        mode,
        w,
        h,
    });
    used
}

unsafe fn setup_drm(app: &mut App) -> i32 {
    let name = CString::new("card0").unwrap();
    app.drm_fd = drmOpen(name.as_ptr(), ptr::null());
    if app.drm_fd < 0 {
        let path = CString::new("/dev/dri/card0").unwrap();
        app.drm_fd = open(path.as_ptr(), O_RDWR | O_CLOEXEC);
    }
    if app.drm_fd < 0 {
        eprint("[igettyd] drmOpen failed\n");
        return -1;
    }
    let res = drmModeGetResources(app.drm_fd);
    if res.is_null() || (*res).count_connectors < 1 {
        eprint("[igettyd] no DRM connectors\n");
        return -1;
    }
    let mut used = 0u32;
    for i in 0..(*res).count_connectors {
        let id = *(*res).connectors.offset(i as isize);
        let conn = drmModeGetConnector(app.drm_fd, id);
        if conn.is_null() {
            continue;
        }
        if (*conn).connection == DRM_MODE_CONNECTED && (*conn).count_modes > 0 {
            used = add_output(app, res, conn, used);
        }
        drmModeFreeConnector(conn);
    }
    if app.outs.is_empty() {
        let id = *(*res).connectors;
        let conn = drmModeGetConnector(app.drm_fd, id);
        if !conn.is_null() {
            let _ = add_output(app, res, conn, used);
            drmModeFreeConnector(conn);
        }
    }
    drmModeFreeResources(res);
    if app.outs.is_empty() {
        eprint("[igettyd] no modes\n");
        return -1;
    }
    eprint(&format!(
        "[igettyd] DRM outputs={} (clone same VT)\n",
        app.outs.len()
    ));
    for o in &app.outs {
        fill_fb(o, 0xFF101010);
    }
    0
}

unsafe fn spawn_getty(getty: &str, cell_w: i32, cell_h: i32, v: &mut Vt) -> i32 {
    let mut m = -1i32;
    let mut s = -1i32;
    if openpty(&mut m, &mut s, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) < 0 {
        eprint("[igettyd] openpty failed\n");
        return -1;
    }
    let mut ws = Winsize {
        ws_row: v.rows as u16,
        ws_col: v.cols as u16,
        ws_xpixel: (v.cols * cell_w) as u16,
        ws_ypixel: (v.rows * cell_h) as u16,
    };
    ioctl(s, TIOCSWINSZ, &mut ws as *mut _ as *mut c_void);
    let pid = fork();
    if pid < 0 {
        close(m);
        close(s);
        return -1;
    }
    if pid == 0 {
        close(m);
        setsid();
        ioctl(s, TIOCSCTTY, ptr::null_mut());
        dup2(s, 0);
        dup2(s, 1);
        dup2(s, 2);
        if s > 2 {
            close(s);
        }
        let term = CString::new("TERM").unwrap();
        let linux = CString::new("linux").unwrap();
        setenv(term.as_ptr(), linux.as_ptr(), 1);
        let ct = CString::new("COLORTERM").unwrap();
        let tc = CString::new("truecolor").unwrap();
        setenv(ct.as_ptr(), tc.as_ptr(), 1);
        let mt = CString::new("WWN_MODEB_TTY").unwrap();
        let one = CString::new("1").unwrap();
        setenv(mt.as_ptr(), one.as_ptr(), 1);
        if !getty.is_empty() {
            let p = CString::new(getty).unwrap();
            let a = CString::new("igetty").unwrap();
            execl(p.as_ptr(), a.as_ptr(), ptr::null::<c_char>());
        }
        let z = CString::new("/bin/zsh").unwrap();
        let zn = CString::new("zsh").unwrap();
        let dashl = CString::new("-l").unwrap();
        execl(z.as_ptr(), zn.as_ptr(), dashl.as_ptr(), ptr::null::<c_char>());
        libc_exit(127);
    }
    close(s);
    fcntl(m, F_SETFL, O_NONBLOCK);
    v.master = m;
    v.shell_pid = pid;
    0
}

fn libc_exit(code: i32) -> ! {
    extern "C" {
        fn _exit(c: i32) -> !;
    }
    unsafe { _exit(code) }
}

unsafe fn vt_init(getty: &str, cell_w: i32, cell_h: i32, v: &mut Vt, cols: i32, rows: i32) -> i32 {
    *v = Vt {
        master: -1,
        shell_pid: -1,
        vt: ptr::null_mut(),
        screen: ptr::null_mut(),
        cols,
        rows,
        dirty: true,
    };
    v.vt = vterm_new(rows, cols);
    if v.vt.is_null() {
        return -1;
    }
    vterm_set_utf8(v.vt, 1);
    v.screen = vterm_obtain_screen(v.vt);
    vterm_screen_set_callbacks(v.screen, &SCREEN_CBS, v as *mut Vt as *mut c_void);
    vterm_screen_set_damage_merge(v.screen, VTERM_DAMAGE_SCREEN);
    vterm_screen_enable_altscreen(v.screen, 1);
    let fg = VTermColor {
        data: [0, 0xe0, 0xe0, 0xe0],
    };
    let bg = VTermColor {
        data: [0, 0x10, 0x10, 0x10],
    };
    vterm_screen_set_default_colors(v.screen, &fg, &bg);
    vterm_screen_reset(v.screen, 1);
    spawn_getty(getty, cell_w, cell_h, v)
}

unsafe fn vt_feed(v: &mut Vt, buf: &[u8]) {
    if v.vt.is_null() || buf.is_empty() {
        return;
    }
    vterm_input_write(v.vt, buf.as_ptr(), buf.len());
    vterm_screen_flush_damage(v.screen);
}

fn key_to_ascii(key: i32, shift: bool) -> u8 {
    match key {
        28 => b'\n',
        14 => b'\x08',
        15 => b'\t',
        57 => b' ',
        2..=11 => {
            let d = b"1234567890";
            let s = b"!@#$%^&*()";
            if shift {
                s[(key - 2) as usize]
            } else {
                d[(key - 2) as usize]
            }
        }
        16..=25 => {
            let d = b"qwertyuiop";
            let s = b"QWERTYUIOP";
            if shift {
                s[(key - 16) as usize]
            } else {
                d[(key - 16) as usize]
            }
        }
        30..=38 => {
            let d = b"asdfghjkl";
            let s = b"ASDFGHJKL";
            if shift {
                s[(key - 30) as usize]
            } else {
                d[(key - 30) as usize]
            }
        }
        44..=50 => {
            let d = b"zxcvbnm";
            let s = b"ZXCVBNM";
            if shift {
                s[(key - 44) as usize]
            } else {
                d[(key - 44) as usize]
            }
        }
        12 => {
            if shift {
                b'_'
            } else {
                b'-'
            }
        }
        13 => {
            if shift {
                b'+'
            } else {
                b'='
            }
        }
        26 => {
            if shift {
                b'{'
            } else {
                b'['
            }
        }
        27 => {
            if shift {
                b'}'
            } else {
                b']'
            }
        }
        39 => {
            if shift {
                b':'
            } else {
                b';'
            }
        }
        40 => {
            if shift {
                b'"'
            } else {
                b'\''
            }
        }
        41 => {
            if shift {
                b'~'
            } else {
                b'`'
            }
        }
        43 => {
            if shift {
                b'|'
            } else {
                b'\\'
            }
        }
        51 => {
            if shift {
                b'<'
            } else {
                b','
            }
        }
        52 => {
            if shift {
                b'>'
            } else {
                b'.'
            }
        }
        53 => {
            if shift {
                b'?'
            } else {
                b'/'
            }
        }
        _ => 0,
    }
}

unsafe fn app_mut() -> &'static mut App {
    &mut *APP
}

unsafe fn stop_graphics(app: &mut App) {
    if app.gfx_pid > 0 {
        kill(app.gfx_pid, SIGTERM);
        let mut st = 0;
        waitpid(app.gfx_pid, &mut st, WNOHANG);
        app.gfx_pid = -1;
    }
}

unsafe fn start_graphics(app: &mut App) {
    if app.gfx_pid > 0 {
        return;
    }
    if app.gui_argv.is_empty() {
        eprint("[igettyd] no GUI command (WWN_IGETTY_GUI_CMD)\n");
        return;
    }
    let pid = fork();
    if pid == 0 {
        let cstrs: Vec<CString> = app
            .gui_argv
            .iter()
            .map(|s| CString::new(s.as_str()).unwrap())
            .collect();
        let mut ptrs: Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
        ptrs.push(ptr::null());
        execvp(ptrs[0], ptrs.as_ptr());
        libc_exit(127);
    }
    if pid > 0 {
        app.gfx_pid = pid;
        eprint(&format!(
            "[igettyd] GUI VT{} argv={:?} pid={}\n",
            GUI_VT.load(Ordering::SeqCst),
            app.gui_argv,
            pid
        ));
    }
}

fn is_gui_vt(vt: i32) -> bool {
    let g = GUI_VT.load(Ordering::SeqCst);
    g > 0 && vt == g
}

unsafe fn switch_vt(app: &mut App, vt: i32) {
    if vt < 1 || vt > VT_TEXT_COUNT as i32 {
        return;
    }
    let cur = ACTIVE_VT.load(Ordering::SeqCst);
    if vt == cur {
        return;
    }
    eprint(&format!("[igettyd] switch VT {cur} -> {vt}\n"));
    if is_gui_vt(vt) {
        ACTIVE_VT.store(vt, Ordering::SeqCst);
        start_graphics(app);
        return;
    }
    if is_gui_vt(cur) {
        stop_graphics(app);
    }
    ACTIVE_VT.store(vt, Ordering::SeqCst);
    bind_text_crtcs(app);
    app.vts[(vt - 1) as usize].dirty = true;
    render_active_text(app);
}

unsafe fn modeb_request_restore() {
    let path = CString::new("/tmp/libwayland-support/modeb-restore-aqua").unwrap();
    let fd = open(path.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644);
    if fd >= 0 {
        let msg = b"modeb-ttyd-chord\n";
        write(fd, msg.as_ptr(), msg.len());
        close(fd);
    }
    eprint("[igettyd] Ctrl+Option+Backspace -> restore Aqua\n");
    RUN.store(false, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn modeb_rs_handle_key(key: i32, pressed: i32) {
    unsafe {
        if key == 29 || key == 97 {
            CTRL.store(if pressed != 0 { 1 } else { 0 }, Ordering::SeqCst);
            return;
        }
        if key == 56 || key == 100 {
            ALT.store(if pressed != 0 { 1 } else { 0 }, Ordering::SeqCst);
            return;
        }
        if key == 42 || key == 54 {
            SHIFT.store(if pressed != 0 { 1 } else { 0 }, Ordering::SeqCst);
            return;
        }
        if key == 58 && pressed != 0 {
            CAPS.store(1 - CAPS.load(Ordering::SeqCst), Ordering::SeqCst);
            return;
        }
        if pressed == 0 {
            return;
        }
        let ctrl = CTRL.load(Ordering::SeqCst);
        let alt = ALT.load(Ordering::SeqCst);
        eprint(&format!("[igettyd] key={key} ctrl={ctrl} alt={alt}\n"));
        if ctrl != 0 && alt != 0 {
            if key == 14 {
                CTRL.store(0, Ordering::SeqCst);
                ALT.store(0, Ordering::SeqCst);
                modeb_request_restore();
                return;
            }
            if (59..=64).contains(&key) {
                let vt = key - 59 + 1;
                if let Ok(mut f) = std::fs::File::create(VT_FILE) {
                    let _ = write!(f, "{vt}\n");
                }
                CTRL.store(0, Ordering::SeqCst);
                ALT.store(0, Ordering::SeqCst);
                switch_vt(app_mut(), vt);
                return;
            }
        }
        let av = ACTIVE_VT.load(Ordering::SeqCst);
        if av < 1 || av > VT_TEXT_COUNT as i32 {
            return;
        }
        let v = &app_mut().vts[(av - 1) as usize];
        if v.master < 0 {
            return;
        }
        if ctrl != 0 && alt == 0 {
            let c = key_to_ascii(key, false);
            if c.is_ascii_lowercase() {
                let ctrlc = c - b'a' + 1;
                write(v.master, &ctrlc, 1);
            }
            return;
        }
        let shift = (SHIFT.load(Ordering::SeqCst) ^ CAPS.load(Ordering::SeqCst)) != 0;
        let mut c = key_to_ascii(key, shift);
        if key == 14 {
            c = 0x7f;
        }
        if c != 0 {
            write(v.master, &c, 1);
        }
    }
}

fn env_str(key: &str) -> Option<String> {
    unsafe {
        let k = CString::new(key).ok()?;
        let p = getenv(k.as_ptr());
        if p.is_null() {
            return None;
        }
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

fn executable_dir() -> Option<String> {
    unsafe {
        let mut buf = [0u8; 1024];
        let mut sz = buf.len() as u32;
        if _NSGetExecutablePath(buf.as_mut_ptr() as *mut c_char, &mut sz) != 0 {
            return None;
        }
        let s = CStr::from_ptr(buf.as_ptr() as *const c_char)
            .to_string_lossy()
            .into_owned();
        let p = std::path::Path::new(&s);
        Some(p.parent()?.to_string_lossy().into_owned())
    }
}

fn find_sidecar(name: &str, env_key: &str) -> String {
    if let Some(e) = env_str(env_key) {
        let c = CString::new(e.as_str()).unwrap();
        unsafe {
            if access(c.as_ptr(), X_OK) == 0 {
                return e;
            }
        }
    }
    if let Some(dir) = executable_dir() {
        let p = format!("{dir}/{name}");
        let c = CString::new(p.as_str()).unwrap();
        unsafe {
            if access(c.as_ptr(), X_OK) == 0 {
                return p;
            }
        }
    }
    String::new()
}

unsafe fn poll_vt_file(app: &mut App, last: &mut i32) {
    if let Ok(s) = std::fs::read_to_string(VT_FILE) {
        if let Ok(vt) = s.trim().parse::<i32>() {
            if vt != *last {
                *last = vt;
                CTRL.store(0, Ordering::SeqCst);
                ALT.store(0, Ordering::SeqCst);
                switch_vt(app, vt);
            }
        }
    }
}

extern "C" fn input_thread_entry(arg: *mut c_void) -> *mut c_void {
    unsafe { modeb_input_thread(arg) }
}

fn load_gui_argv() -> Vec<String> {
    let Some(cmd) = env_str("WWN_IGETTY_GUI_CMD") else {
        return Vec::new();
    };
    if cmd.is_empty() {
        return Vec::new();
    }
    let mut argv = vec![cmd];
    if let Some(rest) = env_str("WWN_IGETTY_GUI_ARGS") {
        for p in rest.split('\u{1f}') {
            if !p.is_empty() {
                argv.push(p.to_string());
            }
        }
    }
    argv
}

fn load_gui_vt(has_gui: bool) -> i32 {
    if !has_gui {
        return 0;
    }
    env_str("WWN_IGETTY_GUI_VT")
        .and_then(|s| s.trim().parse::<i32>().ok())
        .filter(|n| (1..=VT_TEXT_COUNT as i32).contains(n))
        .unwrap_or(1)
}

fn first_text_vt() -> i32 {
    let g = GUI_VT.load(Ordering::SeqCst);
    for n in 1..=VT_TEXT_COUNT as i32 {
        if n != g {
            return n;
        }
    }
    1
}

fn empty_vt() -> Vt {
    Vt {
        master: -1,
        shell_pid: -1,
        vt: ptr::null_mut(),
        screen: ptr::null_mut(),
        cols: 0,
        rows: 0,
        dirty: true,
    }
}

fn main() {
    unsafe {
        signal(SIGTERM, on_signal as usize);
        signal(SIGINT, on_signal as usize);
        signal(SIGHUP, on_signal as usize);
        signal(SIGCHLD, SIG_DFL);
        let dir = CString::new("/tmp/libwayland-support").unwrap();
        mkdir(dir.as_ptr(), 0o755);

        let mut app = App {
            drm_fd: -1,
            outs: Vec::new(),
            vts: std::array::from_fn(|_| empty_vt()),
            cell_w: 8,
            cell_h: 8,
            gfx_pid: -1,
            gui_argv: load_gui_argv(),
            getty: {
                let g = find_sidecar("igetty", "WWN_IGETTY_GETTY");
                if g.is_empty() {
                    find_sidecar("modeb-getty", "WWN_MODEB_GETTY")
                } else {
                    g
                }
            },
        };
        GUI_VT.store(load_gui_vt(!app.gui_argv.is_empty()), Ordering::SeqCst);
        if app.getty.is_empty() {
            eprint("[igettyd] WARN: igetty missing; fallback zsh -l\n");
        } else {
            eprint(&format!("[igettyd] getty={}\n", app.getty));
        }
        eprint(&format!(
            "[igettyd] GUI VT={} argv={:?}\n",
            GUI_VT.load(Ordering::SeqCst),
            app.gui_argv
        ));
        if setup_drm(&mut app) != 0 {
            std::process::exit(1);
        }
        let mut pt = 12.0f32;
        if let Some(s) = env_str("WWN_MODEB_FONT_PT") {
            if let Ok(v) = s.parse::<f32>() {
                if v >= 8.0 {
                    pt = v;
                }
            }
        }
        if modeb_ctfont_init(ptr::null(), pt) == 0 {
            modeb_ctfont_cell_size(&mut app.cell_w, &mut app.cell_h);
        } else {
            eprint("[igettyd] CoreText font failed\n");
        }

        let mut cols = COLS_MAX;
        let mut rows = ROWS_MAX;
        for o in &app.outs {
            cols = cols.min(o.w / app.cell_w);
            rows = rows.min(o.h / app.cell_h);
        }
        cols = cols.clamp(40, COLS_MAX);
        rows = rows.clamp(10, ROWS_MAX);

        let getty = app.getty.clone();
        let cw = app.cell_w;
        let ch = app.cell_h;
        let gui = GUI_VT.load(Ordering::SeqCst);
        for i in 0..VT_TEXT_COUNT {
            if (i as i32) + 1 == gui {
                continue;
            }
            if vt_init(&getty, cw, ch, &mut app.vts[i], cols, rows) != 0 {
                eprint(&format!("[igettyd] VT{} init failed\n", i + 1));
                std::process::exit(1);
            }
        }

        APP = Box::into_raw(Box::new(app));
        let app = app_mut();

        if modeb_input_subscribe() != 0 {
            eprint("[igettyd] FATAL: no inputd subscribe\n");
            std::process::exit(1);
        }
        let mut thr: usize = 0;
        pthread_create(&mut thr, ptr::null(), input_thread_entry, ptr::null_mut());

        let banner = b"\r\nwwn-igetty (Doorman login + Linux-shaped VTs)\r\n\
Ctrl+Option+F1-F6 switch VTs. Assigned GUI VT runs the Desktop machine.\r\n\
Ctrl+Option+Backspace restores Aqua.\r\n\
(MacBook: hold Fn for F-keys if needed)\r\n\r\n";
        let text0 = first_text_vt();
        if (1..=VT_TEXT_COUNT as i32).contains(&text0) {
            vt_feed(&mut app.vts[(text0 - 1) as usize], banner);
        }

        let boot = if gui > 0 { gui } else { text0 };
        ACTIVE_VT.store(boot, Ordering::SeqCst);
        if is_gui_vt(boot) {
            start_graphics(app);
        } else {
            render_active_text(app);
        }
        eprint(&format!(
            "[igettyd] ready cols={cols} rows={rows} active=VT{boot} gui=VT{gui}\n"
        ));

        let mut last_vt = -1i32;
        while RUN.load(Ordering::SeqCst) {
            poll_vt_file(app, &mut last_vt);

            let mut ts = Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            clock_gettime(CLOCK_MONOTONIC, &mut ts);
            let ms = ts.tv_sec * 1000 + ts.tv_nsec / 1_000_000;
            let on = ((ms / 530) & 1) != 0;
                if on != CURSOR_ON.load(Ordering::Relaxed) {
                CURSOR_ON.store(on, Ordering::Relaxed);
                let av = ACTIVE_VT.load(Ordering::SeqCst);
                if (1..=VT_TEXT_COUNT as i32).contains(&av) && !is_gui_vt(av) {
                    render_active_text(app);
                }
            }

            let mut pfd = [Pollfd {
                fd: 0,
                events: 0,
                revents: 0,
            }; VT_TEXT_COUNT];
            for i in 0..VT_TEXT_COUNT {
                pfd[i].fd = app.vts[i].master;
                pfd[i].events = POLLIN;
            }
            let pr = poll(pfd.as_mut_ptr(), VT_TEXT_COUNT as u32, 50);
            if pr > 0 {
                for i in 0..VT_TEXT_COUNT {
                    if pfd[i].revents & POLLIN == 0 {
                        continue;
                    }
                    let mut buf = [0u8; 4096];
                    let n = read(app.vts[i].master, buf.as_mut_ptr(), buf.len());
                    if n > 0 {
                        vt_feed(&mut app.vts[i], &buf[..n as usize]);
                    }
                }
            }

            for i in 0..VT_TEXT_COUNT {
                if (i as i32) + 1 == GUI_VT.load(Ordering::SeqCst) {
                    continue;
                }
                if app.vts[i].shell_pid <= 0 {
                    continue;
                }
                let mut st = 0;
                let r = waitpid(app.vts[i].shell_pid, &mut st, WNOHANG);
                if r == app.vts[i].shell_pid {
                    eprint(&format!("[igettyd] VT{} session ended; respawn getty\n", i + 1));
                    if app.vts[i].master >= 0 {
                        close(app.vts[i].master);
                        app.vts[i].master = -1;
                    }
                    app.vts[i].shell_pid = -1;
                    if !app.vts[i].vt.is_null() {
                        vterm_screen_reset(app.vts[i].screen, 1);
                        app.vts[i].dirty = true;
                    }
                    let getty = app.getty.clone();
                    let cw = app.cell_w;
                    let ch = app.cell_h;
                    spawn_getty(&getty, cw, ch, &mut app.vts[i]);
                }
            }

            let av = ACTIVE_VT.load(Ordering::SeqCst);
            if (1..=VT_TEXT_COUNT as i32).contains(&av) && !is_gui_vt(av) && app.vts[(av - 1) as usize].dirty {
                render_active_text(app);
            }

            if app.gfx_pid > 0 {
                let mut st = 0;
                let r = waitpid(app.gfx_pid, &mut st, WNOHANG);
                if r == app.gfx_pid {
                    app.gfx_pid = -1;
                    if is_gui_vt(ACTIVE_VT.load(Ordering::SeqCst)) {
                        switch_vt(app, first_text_vt());
                    }
                }
            }
        }

        stop_graphics(app);
        for v in &mut app.vts {
            if v.shell_pid > 0 {
                kill(v.shell_pid, SIGHUP);
            }
            if v.master >= 0 {
                close(v.master);
            }
            if !v.vt.is_null() {
                vterm_free(v.vt);
            }
        }
        eprint("[igettyd] exit\n");
    }
}
