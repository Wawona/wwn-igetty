//! Deferred Sileo session provider contract.
//!
//! The TrollStore product must never link this crate. A later jailbreak build
//! can supply Doorman authentication and Procursus PTYs behind the same core
//! traits without importing macOS WindowServer or IOWatchdog assumptions.

use wwn_igetty_core::{Session, SessionProvider, SwitchError};

pub struct AuthenticatedJailbreakSessionProvider;

impl SessionProvider for AuthenticatedJailbreakSessionProvider {
    fn activate(&mut self, _session: &Session) -> Result<(), SwitchError> {
        Err(SwitchError::ProviderRejected)
    }

    fn deactivate(&mut self, _session: &Session) -> Result<(), SwitchError> {
        Err(SwitchError::ProviderRejected)
    }
}
