//! The Mod Configuration Menu's NVSE plugin (`NVSE\Plugins\MCM.dll`, MCM 1.5 by Pelinor, with
//! its author's permission; `NOTICE`) built in. MCM's own scripts and the menus of the mods that
//! use it call its nine commands; the DLL is a 32-bit plugin nv-rs cannot load, so they are
//! rebuilt here from its machine code (`build43-scratch/measure/mcm/NOTES.md`).
//!
//! As xNVSE knows it (`NVSEPlugin_Query`, `NVSEPlugin_Load`): "MCM Extensions" version 1, file
//! `MCM.dll`, opcode base `0x21C0`, nine commands registered with `RegisterCommand` (default
//! return, xNVSE's default parser, none a condition), no listener, no co-save. Its commands
//! ([`commands`]):
//! - `GetModINISetting` (`GetModINI`) and `SetModINISetting` (`SetModINI`): a number in
//!   `Data\Config\<mod>.ini`, through the mod files the core keeps (`engine_core::mod_files`: a
//!   mod's writes go to the port's folder beside the saves, never the game's folder);
//! - `GetMCMFloat`, `SetMCMFloat`, `SetMCMString`, `SetMCMFloatMass`, `SetMCMStringMass`,
//!   `SetMCMModList`, `GetMCMListWidth`: MCM's menu traits under `StartMenu/MCM/`, found by the
//!   DLL's own path code ([`menu`]).
//!
//! The commands run on main (script lines); they take no shared slot.
//!
//! A fixes row ("The Mod Configuration Menu's plugin built in", `NV_MCM`, `bMCM`), off in every
//! preset: the row switches it on ([`set_on`]) and [`install`] (after JIP, before UI Organizer:
//! xNVSE loads plugins in the order of their file names) registers the commands and lists the
//! plugin. Off, nothing is registered: the opcodes stay empty, as without the DLL.

pub mod commands;
pub mod menu;

use engine_core::prelude::*;

/// `PluginInfo::name` and `::version` (`NVSEPlugin_Query`), and the DLL's file name.
pub const NAME: &str = "MCM Extensions";
pub const VERSION: u32 = 1;
pub const FILE: &str = "MCM.dll";
/// `SetOpcodeBase(0x21C0)`.
pub const OPCODE_BASE: u32 = 0x21c0;

/// MCM's plugin on this engine.
#[derive(Default)]
pub struct Mcm {
    /// The row switched it on ([`set_on`]).
    pub on: bool,
    /// [`install`] ran.
    pub installed: bool,
}

/// The row's switch: MCM's plugin built in for this run (before [`install`]).
pub fn set_on(e: &mut Engine, on: bool) {
    e.ext::<Mcm>().on = on;
}

/// Whether the running line came from the console, as the DLL asks it: the thread's TLS byte
/// `+0x268` alone (NVSE 2's `IsConsoleMode`, before xNVSE also asked whether the console is open).
pub fn from_console(e: &mut Engine) -> bool {
    let tls = e.tls();
    tls != 0 && e.mem.u8(tls + 0x268) != 0
}

/// xNVSE's record of the plugin (`NVSEPlugin_Query`).
fn plugin() -> nvse::plugins::BuiltIn {
    nvse::plugins::BuiltIn {
        file: FILE,
        name: NAME,
        version: VERSION,
        opcode_base: OPCODE_BASE,
        listed: true,
        listener: None,
        save: None,
        load: None,
        pre_load: None,
        new_game: None,
    }
}

/// Builds MCM's plugin into the engine when the row switched it on: its nine commands at their
/// opcodes, its place in xNVSE's list. Once per engine; off, or a second call, does nothing.
pub fn install(e: &mut Engine) {
    let state = e.ext::<Mcm>();
    if state.installed || !state.on {
        return;
    }
    state.installed = true;
    commands::install(e);
    nvse::plugins::register(e, plugin());
    // The game's state, not one engine's: real threads' engines share it.
    e.share_ext::<Mcm>();
}

// The unit tests' own program: no other crate's functions are linked in.
#[cfg(test)]
engine_core::assemble!();

#[cfg(test)]
#[path = "commands_tests.rs"]
mod commands_tests;
