//! JIP LN NVSE, xNVSE's largest plugin, built in (ADR-0016; step 3 of the plan: the plugins VNV
//! and TTW need, JIP LN first). Translated from JIP LN NVSE 57.30 (`5a30ac4`) with its authors'
//! permission; credits and notices in `NOTICE`. JIP patches the old 32-bit program, so its DLL
//! cannot load into nv-rs: it is rebuilt here, on the built-in xNVSE (`crates/nvse`), the way
//! xNVSE runs it as a plugin.
//!
//! What is here so far (steps 3.1, 3.2 and part of 3.4):
//! - [`table`], [`slots`]: JIP's 1,150 command slots in the script command table at their real
//!   opcodes (`0x2200..=0x233A` and `0x2600..=0x2942`), each record as JIP registers it, with no
//!   handler yet: a script that calls a JIP command fails at that call, as it does without the
//!   DLL.
//! - [`identity`]: the answers a mod asks first (`GetLNVersion`, `GetNVSEVersionFull`, JIP's
//!   versions of `IsPluginInstalled` and `GetPluginVersion`), behind one flag,
//!   [`ANSWERS_INSTALLED`], that keeps JIP "not installed" until the commands mods call are
//!   built.
//! - [`cosave`]: JIP's co-save section (its eight records, in JIP's format and in JohnnyGuitar
//!   NVSE's corrected one) and its listener for xNVSE's messages, registered with xNVSE as a
//!   plugin (`nvse::plugins`); [`containers`]: JIP's hash maps, whose walking order decides the
//!   order of the records it writes.
//! - [`overrides`]: JIP's versions of xNVSE's and the game's commands (`InitCmdPatches`), also
//!   behind [`ANSWERS_INSTALLED`] (the game's own ones also behind a fixes row).
//! - [`game`]: the game's classes JIP reads, as layouts.
//! - [`console`], [`utility`]: JIP's console echo of a command's result and the text helpers it
//!   uses.
//! - [`commands`]: JIP's script commands built so far (step 3.6), one module per JIP source file.
//! - [`ui_hooks`]: JIP's UI hooks at their seams (step 3.5a: menu open, close, mouseover and
//!   click events, location and crafting events, and the hooks behind its interface commands) and
//!   the twelve commands that set the events.
//! - [`actor_hooks`]: JIP's actor event hooks at their seams (step 3.5b: animations, damage, weapons
//!   fired and reloaded, hits, ragdolls, aid items, projectile impacts), its hit-data routines in the
//!   processes' vtables, and the eleven commands that set the events.
//!
//! [`install`] runs once per engine, after `nvse::install` (JIP is xNVSE's plugin): in
//! `host::Host::start_with`, never in the engine crates' own tests.

pub mod actor_hooks;
pub mod commands;
pub mod console;
pub mod containers;
pub mod cosave;
pub mod game;
pub mod identity;
pub mod input;
pub mod ln_events;
pub mod main_loop;
pub mod overrides;
pub mod script_call;
#[rustfmt::skip]
pub mod slots;
#[cfg(test)]
mod saveload_tests;
pub mod table;
#[cfg(test)]
mod test_support;
pub mod ui_hooks;
pub mod utility;

use engine_core::prelude::*;

/// Whether JIP LN NVSE answers "installed": its identity commands work, `IsPluginInstalled`
/// and `GetPluginVersion` answer for its names, its commands' names are in the command table,
/// and its empty slots run as JIP's do. Off until the commands mods call are built (the plan's
/// step 3.10): a mod that checks for JIP then stops with its own "requires JIP" message, as
/// without the DLL, instead of halfway through a script. The one place to turn it on.
pub const ANSWERS_INSTALLED: bool = false;

// Held off in every build until step 3.10, which turns the flag on and removes this line.
const _: () = assert!(
    !ANSWERS_INSTALLED,
    "JIP answers \"installed\" only from the plan's step 3.10"
);

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
    /// The data JIP keeps in its co-save section, and what decides its loads.
    pub saved: cosave::Saved,
    /// JIP's versions of xNVSE's and the game's commands.
    pub overrides: overrides::State,
    /// JIP's gamepad state and its changes to what the game reads.
    pub input: input::State,
    /// JIP's main-loop callbacks.
    pub main_loop: main_loop::State,
    /// JIP's LN events.
    pub ln_events: ln_events::State,
    /// JIP's UI hooks and their events.
    pub ui_hooks: ui_hooks::State,
    /// What JIP's quest commands keep between calls.
    pub quest: commands::jip_fn_quest::State,
    /// JIP's actor hooks and their events.
    pub actor_hooks: actor_hooks::State,
}

/// JIP as xNVSE knows it once loaded (`NVSEPlugin_Query`, `NVSEPlugin_Load`): its DLL, name,
/// version and first opcode base, its listener and its co-save callbacks; in xNVSE's list of
/// plugins only when it answers "installed".
fn plugin(answers_installed: bool) -> nvse::plugins::BuiltIn {
    nvse::plugins::BuiltIn {
        file: "jip_nvse.dll",
        name: "JIP LN NVSE",
        version: identity::VERSION_INT,
        opcode_base: table::FIRST_OPCODE,
        listed: answers_installed,
        listener: Some(cosave::listen),
        save: Some(cosave::save),
        load: Some(cosave::load),
        pre_load: None,
        new_game: None,
    }
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
    // First: a built command's record is named only when JIP answers "installed".
    e.ext::<Jip>().answers_installed = answers_installed;
    table::install(e, answers_installed);
    identity::install(e, answers_installed);
    overrides::install(e, answers_installed);
    // Built commands answer only while JIP answers "installed": `table::set_handlers` writes
    // nothing until then, so every JIP opcode is as without the DLL (no handler, no name).
    commands::install(e);
    input::install(e, answers_installed);
    ln_events::install(e, answers_installed);
    main_loop::install(e);
    ui_hooks::install(e, answers_installed);
    actor_hooks::install(e, answers_installed);
    nvse::plugins::register(e, plugin(answers_installed));
    // `MainGameLoop` only once JIP answers "installed": until then a frame does no JIP work.
    if answers_installed {
        nvse::plugins::listen_per_frame(e);
    }
    e.ext::<Jip>().installed = true;
    // The game's state, not one engine's: real threads' engines share it.
    e.share_ext::<Jip>();
}

// The unit tests' own program: no other crate's functions are linked in.
#[cfg(test)]
engine_core::assemble!();
