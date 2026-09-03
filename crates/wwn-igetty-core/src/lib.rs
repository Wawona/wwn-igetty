//! Platform-neutral logical session switching for Wawona Desktop.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SessionKind {
    Greeter = 0,
    Text = 1,
    Native = 2,
    VirtualMachine = 3,
    Container = 4,
    Compositor = 5,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    pub id: u32,
    pub kind: SessionKind,
    pub label: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwitchPlan {
    pub previous: Option<u32>,
    pub next: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchError {
    UnknownSession,
    ProviderRejected,
    DisplayRejected,
    InputRejected,
}

pub trait SessionProvider {
    fn activate(&mut self, session: &Session) -> Result<(), SwitchError>;
    fn deactivate(&mut self, session: &Session) -> Result<(), SwitchError>;
}

pub trait DisplayBackend {
    fn present(&mut self, session: &Session) -> Result<(), SwitchError>;
}

pub trait InputBackend {
    fn focus(&mut self, session: &Session) -> Result<(), SwitchError>;
}

/// Owns policy only. Platform backends own processes, PTYs, surfaces, and I/O.
pub struct SessionSwitcher {
    sessions: BTreeMap<u32, Session>,
    active: Option<u32>,
}

impl SessionSwitcher {
    pub fn new(greeter_label: impl Into<String>) -> Self {
        let greeter = Session {
            id: 0,
            kind: SessionKind::Greeter,
            label: greeter_label.into(),
        };
        Self {
            sessions: BTreeMap::from([(greeter.id, greeter)]),
            active: Some(0),
        }
    }

    pub fn register(&mut self, session: Session) {
        self.sessions.insert(session.id, session);
    }

    pub fn unregister(&mut self, id: u32) -> Option<Session> {
        if id == 0 {
            return None;
        }
        let removed = self.sessions.remove(&id);
        if self.active == Some(id) {
            self.active = Some(0);
        }
        removed
    }

    pub fn active(&self) -> Option<&Session> {
        self.active.and_then(|id| self.sessions.get(&id))
    }

    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    pub fn plan_switch(&self, next: u32) -> Result<SwitchPlan, SwitchError> {
        if !self.sessions.contains_key(&next) {
            return Err(SwitchError::UnknownSession);
        }
        Ok(SwitchPlan {
            previous: self.active,
            next,
        })
    }

    pub fn commit(&mut self, plan: SwitchPlan) {
        self.active = Some(plan.next);
    }

    pub fn switch_to<P, D, I>(
        &mut self,
        next: u32,
        provider: &mut P,
        display: &mut D,
        input: &mut I,
    ) -> Result<SwitchPlan, SwitchError>
    where
        P: SessionProvider,
        D: DisplayBackend,
        I: InputBackend,
    {
        let plan = self.plan_switch(next)?;
        if plan.previous == Some(next) {
            return Ok(plan);
        }
        if let Some(previous) = plan.previous.and_then(|id| self.sessions.get(&id)) {
            provider.deactivate(previous)?;
        }
        let session = self.sessions.get(&next).expect("validated session");
        provider.activate(session)?;
        display.present(session)?;
        input.focus(session)?;
        self.commit(plan);
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Backend(Vec<(char, u32)>);

    impl SessionProvider for Backend {
        fn activate(&mut self, session: &Session) -> Result<(), SwitchError> {
            self.0.push(('a', session.id));
            Ok(())
        }

        fn deactivate(&mut self, session: &Session) -> Result<(), SwitchError> {
            self.0.push(('d', session.id));
            Ok(())
        }
    }

    impl DisplayBackend for Backend {
        fn present(&mut self, session: &Session) -> Result<(), SwitchError> {
            self.0.push(('p', session.id));
            Ok(())
        }
    }

    impl InputBackend for Backend {
        fn focus(&mut self, session: &Session) -> Result<(), SwitchError> {
            self.0.push(('f', session.id));
            Ok(())
        }
    }

    #[test]
    fn switches_text_and_returns_to_greeter() {
        let mut switcher = SessionSwitcher::new("Machines");
        switcher.register(Session {
            id: 1,
            kind: SessionKind::Text,
            label: "tty01".into(),
        });
        let mut provider = Backend::default();
        let mut display = Backend::default();
        let mut input = Backend::default();

        switcher
            .switch_to(1, &mut provider, &mut display, &mut input)
            .unwrap();
        assert_eq!(switcher.active().map(|s| s.id), Some(1));
        switcher
            .switch_to(0, &mut provider, &mut display, &mut input)
            .unwrap();
        assert_eq!(
            switcher.active().map(|s| s.kind),
            Some(SessionKind::Greeter)
        );
        assert_eq!(provider.0, vec![('d', 0), ('a', 1), ('d', 1), ('a', 0)]);
    }

    #[test]
    fn rejects_unknown_session_without_changing_focus() {
        let switcher = SessionSwitcher::new("Machines");
        assert_eq!(switcher.plan_switch(42), Err(SwitchError::UnknownSession));
        assert_eq!(switcher.active().map(|s| s.id), Some(0));
    }
}
