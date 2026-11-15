//! UI Organizer (UIO), the NVSE plugin that puts UI mods' additions into the game's menus, built
//! in. Translated from UIO 2.30 (`jazzisparis/UIO-User-Interface-Organizer`, commit `426d963`,
//! the source of the shipped 2.30 DLL), MIT licence, with its author's permission; credits and the
//! licence in `NOTICE`. UIO patches the old 32-bit program, so its DLL cannot load into nv-rs: it
//! is rebuilt here, part by part (docs/FIXES.md, "UI Organizer").
//!
//! What is here (part 1, the organizer; part 2, its XML reader and its export):
//! - [`lists`]: UIO's lists, `Data\uio\supported.txt` and every `Data\uio\public\*.txt`, with
//!   their conditions (the UI mods present, files present, plugins listed in `plugins.txt`): what
//!   goes into which menu file, under which tile.
//! - [`patch`]: each target file's text with its `<include src="..."/>` lines put in, in memory,
//!   exactly where and in the order UIO puts them.
//! - [`reader`]: UIO's file reader, which `Tile::ReadFile` reads through once UIO hooks it: the
//!   patched texts, then UIO's own expansion of `<include>` tags.
//! - [`parse`] and [`build`]: UIO's own XML reader, which replaces the game's (`&-sName;` game
//!   setting values, `&name;` trait constants, `_name` user traits registered as they are met, the
//!   items typed as the game's `AddPair` types them), building the game's parse storage.
//! - [`game`]: the two places in the game (UIO's patches at `0070A049` and `00A01B87`), seams in
//!   the translations that read [`Engine::uio`] first; UIO's parse cache (a menu's parse kept for
//!   every later read, `ReadFile`'s "keep" byte set); [`inject_component`], UIO's export
//!   `UIOInjectComponent`, which the built-in JIP's `InjectUIComponent` calls.
//! - [`hash`]: UIO's hash tables, whose walking order decides the order of the lines it writes.
//!
//! Not yet (part 3): its new tile operators and its code patches (text zoom, the HUD's hardcore
//! texts). Left out: UIO's log files and its developer settings (`settings.ini`: `bDebugMode`
//! and its per-file dumps, `sDoNotCache`).
//!
//! It is a fixes row ("UI Organizer built in", `NV_UIO`): the row switches it on ([`set_on`]) and
//! [`install`] (after JIP, as xNVSE loads `ui_organizer.dll` after `jip_nvse.dll`) puts its seams
//! in and lists it with xNVSE as "UI Organizer Plugin" 230 (what `IsPluginInstalled` and
//! `GetPluginVersion` answer). Off, nothing is installed and no translation changes.

pub mod build;
pub mod game;
pub mod hash;
pub mod lists;
pub mod parse;
pub mod patch;
pub mod reader;

pub use game::inject_component;

use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// `PluginInfo::name` and `::version` (`NVSEPlugin_Query` (UIO)), and the DLL's file name.
pub const NAME: &str = "UI Organizer Plugin";
pub const VERSION: u32 = 230;
pub const FILE: &str = "ui_organizer.dll";

/// UIO's two seams (its patches' addresses).
pub const LOAD_SITE: SiteSlot = SiteSlot::new(Owner::Uio, 0x0070_a049);
pub const READ_FILE_SITE: SiteSlot = SiteSlot::new(Owner::Uio, 0x00a0_1b87);

/// UIO on this engine: one slot, shared by the game's threads once installed.
#[derive(Default)]
pub struct Uio {
    /// The row switched it on ([`set_on`]).
    pub on: bool,
    /// [`install`] ran.
    pub installed: bool,
    /// Its lists were read and its targets patched ([`game::uio_load`]).
    pub loaded: bool,
    /// The patched files.
    pub patched: reader::Patched,
    /// The parse cache (`s_cachedParsedData`), by UIO's hash of the path.
    pub parsed: std::collections::HashMap<u32, Parsed>,
    /// The text [`inject_component`] hands to the next read (`s_externFileData`).
    pub extern_text: Option<Vec<u8>>,
}

/// One file's entry in UIO's parse cache (`ParsedData`): its kept storage (0: none yet) and
/// whether UIO keeps its parse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parsed {
    pub storage: u32,
    pub cached: bool,
}

/// The row's switch: UIO built in for this run (before [`install`]).
pub fn set_on(e: &mut Engine, on: bool) {
    e.ext::<Uio>().on = on;
}

/// Whether UIO is built in and on: what other built-ins ask in place of JIP's and Stewie's
/// `GetModuleHandle("ui_organizer")`.
pub fn on(e: &Engine) -> bool {
    e.uio()
}

/// xNVSE's record of UIO (`NVSEPlugin_Query`): no opcode base (UIO registers no command), no
/// listener, no co-save.
fn plugin() -> nvse::plugins::BuiltIn {
    nvse::plugins::BuiltIn {
        file: FILE,
        name: NAME,
        version: VERSION,
        opcode_base: 0,
        listed: true,
        listener: None,
        save: None,
        load: None,
        pre_load: None,
        new_game: None,
    }
}

/// Builds UIO into the engine when the row switched it on: the word the seams read, its two
/// handlers, its place in xNVSE's list. Once per engine; off, or a second call, does nothing.
pub fn install(e: &mut Engine) {
    let state = e.ext::<Uio>();
    if state.installed || !state.on {
        return;
    }
    state.installed = true;
    e.set_uio(true);
    e.on_site(LOAD_SITE, game::uio_load);
    e.on_site(READ_FILE_SITE, game::read_file);
    nvse::plugins::register(e, plugin());
    // The game's state, not one engine's: real threads' engines share it.
    e.share_ext::<Uio>();
}

// The unit tests' own program: no other crate's functions are linked in.
#[cfg(test)]
engine_core::assemble!();

#[cfg(test)]
mod test_game;

#[cfg(test)]
#[path = "game_tests.rs"]
mod game_tests;
