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
//!   versions of `IsPluginInstalled` and `GetPluginVersion`), behind one run-time
//!   switch, the host's row `NV_JIP_INSTALLED` ([`set_answers_installed`], off in every preset:
//!   step 3.10), that keeps JIP "not installed" unless a run turns it on.
//! - [`cosave`]: JIP's co-save section (its eight records, in JIP's format and in JohnnyGuitar
//!   NVSE's corrected one) and its listener for xNVSE's messages, registered with xNVSE as a
//!   plugin (`nvse::plugins`); [`containers`]: JIP's hash maps, whose walking order decides the
//!   order of the records it writes.
//! - [`overrides`]: JIP's versions of xNVSE's and the game's commands (`InitCmdPatches`), also
//!   behind that switch (the game's own ones also behind a fixes row).
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
//! - [`ai_hooks`]: JIP's actor AI hooks at their seams (step 3.5b: combat disabled or a forced target,
//!   teammates' health resets, follow and wait orders, locked equipment, forced detection values,
//!   a push's velocity, disabled scripts' events) and the ten commands that switch them.
//! - [`world_hooks`]: JIP's world hooks at their seams (step 3.5c: descriptions, model paths and
//!   reference names a script set, unique items, the dialogue topic, fast travel, cell, target,
//!   note and quest stage events, activations and script events a script disabled, teammates at
//!   locked doors, the weather's transition time, swapped LOD blocks, scripts that wait) and the
//!   commands that switch them, with `PushActorNoRagdoll`.
//! - [`game_patches`]: JIP's changes to the game's own code (step 3.9: the parent topic of every
//!   dialogue info, results that start at 0), fix rows behind the host's `NV_JIP_GAME_PATCHES`.
//!
//! [`install`] runs once per engine, after `nvse::install` (JIP is xNVSE's plugin): in
//! `host::Host::start_with`; in the engine crates' own tests only through
//! `test_support::install_into` (the `test-support` feature), for their end-to-end seam tests.

pub mod actor_hooks;
pub mod ai_hooks;
pub mod commands;
pub mod console;
pub mod containers;
pub mod cosave;
pub mod equip_hooks;
pub mod game;
pub mod game_patches;
pub mod identity;
pub mod input;
pub mod ln_events;
pub mod main_loop;
pub mod overrides;
pub mod script_call;
#[rustfmt::skip]
pub mod slots;
#[cfg(test)]
mod condition_locks_tests;
#[cfg(test)]
mod saveload_tests;
#[cfg(test)]
mod switch_tests;
pub mod table;
/// For the crate's tests and, through the `test-support` feature, the engine crates' end-to-end
/// tests of JIP's seams.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod ui_hooks;
pub mod utility;
pub mod world_hooks;
mod world_push;

use engine_core::prelude::*;

/// What JIP LN NVSE answers when nothing switches it on: "not installed". "Installed" means its
/// identity commands work, `IsPluginInstalled` and `GetPluginVersion` answer for its names, it
/// is in xNVSE's list of plugins, its built commands' names and handlers are in the command
/// table, its hooks and its own game patches go in. Not installed, a mod that checks for JIP
/// stops with its own "requires JIP" message, as without the DLL, instead of halfway through a
/// script. A run switches it on only with the host's row `NV_JIP_INSTALLED` (Fallout.ini
/// `bJIPInstalled`, off in every preset: step 3.10), through [`set_answers_installed`] before
/// [`install`].
pub const DEFAULT_ANSWERS_INSTALLED: bool = false;

// "Installed" only by the row, never by default, until the row is proven in the game.
const _: () = assert!(
    !DEFAULT_ANSWERS_INSTALLED,
    "JIP answers \"installed\" only when the host's row NV_JIP_INSTALLED switches it on"
);

/// The host's row `NV_JIP_INSTALLED`: JIP answers "installed" in this run (`on`), or "not
/// installed". Read by [`install`], so it is set before it; after [`install`] it changes
/// nothing.
pub fn set_answers_installed(e: &mut Engine, on: bool) {
    e.ext::<Jip>().row_on = on;
}

/// JIP built in on this engine: its state, one slot per engine ([`Engine::ext`]), shared by the
/// game's threads once installed.
#[derive(Default)]
pub struct Jip {
    /// [`install`] ran.
    pub installed: bool,
    /// Whether JIP answers "installed": what [`install`] was given ([`Jip::row_on`] there).
    pub answers_installed: bool,
    /// The host's row `NV_JIP_INSTALLED` asked for "installed" ([`set_answers_installed`]);
    /// starts at [`DEFAULT_ANSWERS_INSTALLED`].
    pub row_on: bool,
    /// The parameter lists written for JIP's records.
    pub table: table::State,
    /// The data JIP keeps in its co-save section, and what decides its loads.
    pub saved: cosave::Saved,
    /// JIP's versions of xNVSE's and the game's commands.
    pub overrides: overrides::State,
    /// JIP's main-loop callbacks.
    pub main_loop: main_loop::State,
    /// JIP's LN events.
    pub ln_events: ln_events::State,
    /// JIP's UI hooks and their events.
    pub ui_hooks: ui_hooks::State,
    /// What JIP's quest commands keep between calls.
    pub quest: commands::jip_fn_quest::State,
    /// What `jip_fn_miscellaneous`'s commands keep between calls (keywords, actor value names).
    pub misc: commands::jip_fn_miscellaneous::State,
    /// What JIP's script commands keep: the script blocks a script disabled.
    pub script: commands::jip_fn_script::State,
    /// What JIP's CC&C commands keep: the actor bases set to NCCS, the location grids.
    pub ccc: commands::jip_fn_ccc::State,
    /// JIP's temporary position marker (`s_tempPosMarker`).
    pub pos_marker: commands::pos_marker::State,
    /// The live variables JIP added to scripts, and the names it added this session.
    pub script_vars: commands::jip_fn_script_vars::State,
    /// JIP's game patches (step 3.9): the host's row and the switches read.
    pub game_patches: game_patches::State,
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
/// "installed" (the host's row, [`set_answers_installed`]; else [`DEFAULT_ANSWERS_INSTALLED`]),
/// its identity, commands, hooks and patches. Once per engine; a second call does nothing.
pub fn install(e: &mut Engine) {
    let answers_installed = e.ext::<Jip>().row_on;
    install_as(e, answers_installed);
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
    cosave::install_unlocked(e);
    input::install(e, answers_installed);
    ln_events::install(e, answers_installed);
    main_loop::install(e);
    ui_hooks::install(e, answers_installed);
    actor_hooks::install(e, answers_installed);
    ai_hooks::install(e, answers_installed);
    equip_hooks::install(e, answers_installed);
    world_hooks::install(e, answers_installed);
    game_patches::install(e, answers_installed);
    nvse::plugins::register(e, plugin(answers_installed));
    // `MainGameLoop` only once JIP answers "installed": until then a frame does no JIP work.
    if answers_installed {
        nvse::plugins::listen_per_frame(e);
    }
    e.ext::<Jip>().installed = true;
    // The hooks' maps and values, read on the AI and loader threads: lock-free slots, so a
    // seam never takes JIP's shared slot.
    actor_hooks::make_slot(e);
    ai_hooks::make_slot(e);
    world_hooks::make_slot(e);
    // The game's state, not one engine's: real threads' engines share it.
    e.share_ext::<Jip>();
}

// The unit tests' own program: no other crate's functions are linked in.
#[cfg(test)]
engine_core::assemble!();
