//! JIP LN NVSE's game patches (step 3.9): the changes JIP makes to the game's own code as it starts
//! (`InitGamePatches`, the `[GamePatches]` switches of `SetOptionalPatch`, the plain patches of
//! `InitJIPHooks` and `DeferredInit`; `internal/patches_game.h`, `internal/hooks.h`,
//! `jip_nvse.cpp` (JIP LN NVSE)). Plan: `measure/jip/PLAN-3.9.md`; design: ADR-0016 §5 (JIP's
//! always-on patches and its switches are fix rows).
//!
//! **Two kinds, one word.** A patch that changes the game for every player once JIP is installed
//! is a fix: it is in only when JIP answers "installed" and the host's row
//! `NV_JIP_GAME_PATCHES` is on (the fixes preset; off in vanilla), and the switches of the
//! player's `jip_nvse.ini` ([`settings`]) decide the optional ones. A patch that only answers
//! JIP's own commands and settings is part of JIP in every preset, in whenever JIP answers
//! "installed". Both kinds are read from one native word, [`Engine::jip_patches`] (its bits in
//! `engine_core::extend::jip_patches`): a seam inside a translation tests its bit first, with one
//! load, and builds its words only after that; whole functions and vtable slots JIP replaces are
//! wraps, put in only when their patch is on, so nothing tests a flag there. While JIP answers
//! "not installed" (today, until step 3.10) the word is 0 and nothing is registered.
//!
//! **Object growth is a port-side record.** Where JIP makes a game object bigger to keep a word of
//! its own in it, the port keeps the object's size and a record keyed by the object's address,
//! filled where JIP writes its word and dropped in the object's destructor (3.9a's
//! [`dialogue`] parent topics are the first).
//!
//! **Thirteen groups at once** (`measure/jip/PLAN-3.9-groups.md`: GA GB GC GD GE GP GH1 GH2 GF GS GW
//! GL GX; every translation file has one owning group). Each group has its own places, so their PRs
//! never edit the same lines:
//! 1. its rows: `crates/engine-core/src/extend/patch_rows/<group>.rs` (seams at addresses no main
//!    row lists, in the file of the group that owns the translation; wraps in the home group's). At
//!    an address a main row of `extend.rs` lists, `Owner::Jip` joins that row instead (rare);
//! 2. its seams in the translations it owns, each `if e.jip_patches() & jip_patches::FIXES != 0 {
//!    e.site(..) }` (or `INSTALLED`, or a switch's bit), the words built inside; with no handler
//!    yet a seam does nothing, so a group adds the inert seam of another group's row in its own file
//!    (with a test handler in its test) and the row's home group writes the handler (3.5b's split);
//! 3. its handlers: `<group>.rs` here (its `AREA`: sites, wraps, installers) and its tests beside it;
//!    `game_patches_tests.rs` holds every area to the word (nothing in while JIP is not installed,
//!    nothing of a fix in vanilla, every fix in with the row) with no edit;
//! 4. a vtable slot JIP writes: [`slots::write_slot`] puts a JIP routine at one of the group's
//!    routine addresses ([`slots::routine`]) into the guest slot, never a wrap of the function the
//!    slot held (most are shared, linker-folded stubs; 3.5a's way);
//! 5. its block in `crates/engine-core/data/overridable.tsv` (`#> 3.9 <group>`), its subsection of
//!    docs/FIXES.md's "JIP LN NVSE's game patches", one row per touched function in its audit
//!    (`measure/jip/AUDIT-TEMPLATE.md`).
//!
//! Seam or wrap: a wrap catches only calls by address; a function some Rust code calls directly
//! (the host's borderless override calls `0xC44280`'s translation, `crates/host/src/full_screen.rs`)
//! needs a seam inside the translation instead.

pub mod dialogue;
pub mod ga;
pub mod gb;
pub mod gc;
pub mod gd;
pub mod ge;
pub mod gf;
pub mod gh1;
pub mod gh2;
pub mod gl;
pub mod gp;
pub mod gs;
pub mod gw;
pub mod gx;
pub mod script;
pub mod settings;
pub mod slots;

use crate::Jip;
use engine_core::extend::jip_patches;
use engine_core::prelude::*;

/// One area's patches: its seams and wraps (exe addresses, for the tests that hold every area to
/// the word) and what puts its handlers in.
pub struct Area {
    /// Its name in the tests' messages.
    pub name: &'static str,
    /// The seams and wraps of its fixes (in with [`jip_patches::FIXES`]).
    pub fix_sites: &'static [u32],
    pub fix_wraps: &'static [u32],
    /// Puts its fixes' handlers and wraps in (once, before the threads start).
    pub install_fixes: fn(&mut Engine),
    /// The seams and wraps of its patches of JIP's own (in with [`jip_patches::INSTALLED`]).
    pub jip_sites: &'static [u32],
    pub jip_wraps: &'static [u32],
    /// Puts those handlers and wraps in (once, as JIP installs).
    pub install_jip: fn(&mut Engine),
}

/// An area with nothing of one kind.
pub fn nothing(_: &mut Engine) {}

/// Every area (3.9a's two, then the groups'), in the order their handlers are put in.
pub const AREAS: &[Area] = &[
    dialogue::AREA,
    script::AREA,
    ga::AREA,
    gb::AREA,
    gc::AREA,
    gd::AREA,
    ge::AREA,
    gf::AREA,
    gh1::AREA,
    gh2::AREA,
    gl::AREA,
    gp::AREA,
    gs::AREA,
    gw::AREA,
    gx::AREA,
];

/// What the game patches keep on JIP's state.
#[derive(Default)]
pub struct State {
    /// The host's row `NV_JIP_GAME_PATCHES` is on.
    pub row: bool,
    /// The fixes' handlers are in.
    pub fixes_in: bool,
    /// What `jip_nvse.ini` said when they went in.
    pub settings: settings::Switches,
}

/// As JIP installs (after the host's rows and xNVSE): the word's `INSTALLED` bit and the patches
/// of JIP's own when JIP answers "installed", and the fixes when the row is on (the host sets it
/// before JIP installs; a test may set it after, [`set_row`]).
pub(crate) fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    e.set_jip_patches(e.jip_patches() | jip_patches::INSTALLED);
    for area in AREAS {
        (area.install_jip)(e);
    }
    if e.ext::<Jip>().game_patches.row {
        put_fixes_in(e);
    }
}

/// The host's row `NV_JIP_GAME_PATCHES` (on in the fixes preset, off in vanilla): JIP's patches
/// that change the game for every player, once JIP answers "installed". Read before the first
/// frame; turning it off later takes nothing out.
pub fn set_row(e: &mut Engine, on: bool) {
    e.ext::<Jip>().game_patches.row = on;
    let jip = e.ext::<Jip>();
    if on && jip.installed && jip.answers_installed {
        put_fixes_in(e);
    }
}

/// Reads the player's `[GamePatches]`, sets the word and puts every area's fixes in, once.
fn put_fixes_in(e: &mut Engine) {
    if e.ext::<Jip>().game_patches.fixes_in {
        return;
    }
    let switches = settings::read(e);
    e.set_jip_patches(e.jip_patches() | jip_patches::FIXES | switches.on);
    e.ext::<Jip>().game_patches.settings = switches;
    e.ext::<Jip>().game_patches.fixes_in = true;
    for area in AREAS {
        (area.install_fixes)(e);
    }
}

#[cfg(test)]
#[path = "game_patches_tests.rs"]
mod tests;
