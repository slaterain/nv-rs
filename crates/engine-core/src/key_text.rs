//! Text supplied by the real keyboard's layout, without assuming a US scan-code table.
//!
//! The platform installs the provider. Headless engines have none. The current
//! provider exposes observed key events; arbitrary keys not yet observed are
//! deliberately unavailable, rather than guessed from their physical positions.
use std::sync::OnceLock;

pub type Provider = fn(u32, bool) -> Option<Vec<u16>>;
static PROVIDER: OnceLock<Provider> = OnceLock::new();

pub fn install(provider: Provider) {
    let _ = PROVIDER.set(provider);
}

/// UTF-16 text for a scan code, with Shift/Caps ignored when requested.
/// Unavailable input is distinct from an observed empty character result.
pub fn get(scan: u32, ignore_shift: bool) -> Option<Vec<u16>> {
    PROVIDER
        .get()
        .and_then(|provider| provider(scan, ignore_shift))
}
