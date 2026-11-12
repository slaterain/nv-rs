//! JIP LN NVSE, xNVSE's largest plugin, built in (ADR-0016; step 3 of the plan: the plugins VNV
//! and TTW need, JIP LN first). Translated from JIP LN NVSE 57.30 (`5a30ac4`) with its authors'
//! permission; credits and notices in `NOTICE`. JIP patches the old 32-bit program, so its DLL
//! cannot load into nv-rs: it is rebuilt here, on the built-in xNVSE (`crates/nvse`), the way
//! xNVSE runs it as a plugin.
//!
//! What is here so far (step 3.1, its table and identity):
//! - [`table`], [`slots`]: JIP's 1,150 command slots in the script command table at their real
//!   opcodes (`0x2200..=0x233A` and `0x2600..=0x2942`), each record as JIP registers it, with no
//!   handler yet: a script that calls a JIP command fails at that call, as it does without the
//!   DLL.
//! - [`identity`]: the answers a mod asks first (`GetLNVersion`, `GetNVSEVersionFull`, JIP's
//!   versions of `IsPluginInstalled` and `GetPluginVersion`), behind one flag,
//!   [`ANSWERS_INSTALLED`], that keeps JIP "not installed" until the commands mods call are
//!   built.
//! - [`console`], [`utility`]: JIP's console echo of a command's result and the text helpers it
//!   uses.
//!
//! [`install`] runs once per engine, after `nvse::install` (JIP is xNVSE's plugin): in
//! `host::Host::start_with`, never in the engine crates' own tests.

pub mod console;
pub mod identity;
#[rustfmt::skip]
pub mod slots;
pub mod table;
#[cfg(test)]
mod test_support;
pub mod utility;

use engine_core::prelude::*;

/// Whether JIP LN NVSE answers "installed": its identity commands work, `IsPluginInstalled`
/// and `GetPluginVersion` answer for its names, and its empty slots run as JIP's do. Off until
/// the commands mods call are built (the plan's step 3.10): a mod that checks for JIP then stops
/// with its own "requires JIP" message, as without the DLL, instead of halfway through a
/// script. The one place to turn it on.
pub const ANSWERS_INSTALLED: bool = false;

/// JIP built in on this engine: its state, one slot per engine ([`Engine::ext`]), shared by the
/// game's threads once installed.
#[derive(Default)]
pub struct Jip {
    /// [`install`] ran.
    pub installed: bool,
    /// What [`ANSWERS_INSTALLED`] was when it ran.
    pub answers_installed: bool,
    /// The parameter lists written for JIP's records.
    pub table: table::State,
}

/// Builds JIP into the engine, after xNVSE: its slots in the command table and, when it answers
/// "installed" ([`ANSWERS_INSTALLED`]), its identity. Once per engine; a second call does nothing.
pub fn install(e: &mut Engine) {
    install_as(e, ANSWERS_INSTALLED);
}

/// [`install`] with the flag given (the tests check both answers).
pub fn install_as(e: &mut Engine, answers_installed: bool) {
    if e.ext::<Jip>().installed {
        return;
    }
    assert_eq!(
        e.script_tables().start,
        nvse::table::TABLE_AT,
        "xNVSE is installed before JIP, its plugin"
    );
    table::install(e, answers_installed);
    identity::install(e, answers_installed);
    let jip = e.ext::<Jip>();
    jip.installed = true;
    jip.answers_installed = answers_installed;
    // The game's state, not one engine's: real threads' engines share it.
    e.share_ext::<Jip>();
}

// The unit tests' own program: no other crate's functions are linked in.
#[cfg(test)]
engine_core::assemble!();
