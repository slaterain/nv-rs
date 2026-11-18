//! lStewieAl's Tweaks ("lStewieAl's Tweaks and Engine Fixes", `nvse_stewie_tweaks.dll` 10.05), the
//! NVSE plugin of engine fixes and tweaks VNV players run, built in, with its author's permission
//! (credits and VNV's licence in `NOTICE`). Tweaks patches the old 32-bit program, so its DLL cannot
//! load into nv-rs: its changes are rebuilt here row by row (docs/FIXES.md, "lStewieAl's Tweaks";
//! the plan: `measure/stewie/PLAN.md`).
//!
//! - [`rows`]: each change Tweaks makes to the game's code is a row with its own bit
//!   ([`engine_core::extend::stewie`]), its own off key in Fallout.ini's `[nv-rs Stewie]` and, when
//!   Tweaks gives it one, Tweaks' own `[Tweaks]` key. The translation Tweaks patches tests the bit at
//!   the patched place.
//! - [`settings`]: every key Tweaks' code reads, with VNV's values as the port's defaults and the
//!   player's `Data\NVSE\Plugins\nvse_stewie_tweaks.ini` over them, key by key (the port never
//!   writes it).
//! - [`ini`]: the file read by SimpleIni's rules, as Tweaks reads it.
//!
//! It is a fixes row ("lStewieAl's Tweaks built in", `NV_STEWIE`, on in the fixes preset): the row
//! switches it on ([`set_on`]) and [`install`] (after JIP and MCM, before UIO, as xNVSE loads
//! `nvse_stewie_tweaks.dll` between them) reads the settings, sets the rows' bits and puts its record
//! in xNVSE's list as "lStewieAl's Tweaks" 1005, not yet listed: `IsPluginInstalled` and
//! `GetPluginVersion` answer "absent" until its script commands are built ([`ANSWERS_INSTALLED`]; the
//! project's rule: nothing half-working is offered to a script). The fixes work either way. Off, no
//! bit is set and no translation changes.
//!
//! Not yet: Tweaks' eight script commands (opcodes `0x2132` up), its co-save records (kept for the
//! next save by the co-save row while its section is not read here), the `Tweaks\INIs\*.ini` overlay
//! files, and every row not in [`rows::ROWS`].

pub mod ini;
pub mod rows;
pub mod settings;

// The unit tests' own program: no other crate's functions are linked in.
#[cfg(test)]
engine_core::assemble!();

#[cfg(test)]
mod tests;

use engine_core::prelude::*;

/// `PluginInfo::name` and `::version` (`NVSEPlugin_Query` (Tweaks)), and the DLL's file name.
pub const NAME: &str = "lStewieAl's Tweaks";
pub const VERSION: u32 = 1005;
pub const FILE: &str = "nvse_stewie_tweaks.dll";
/// `SetOpcodeBase(0x2132)` (`NVSEPlugin_Load` (Tweaks)): its co-save section's header.
pub const OPCODE_BASE: u32 = 0x2132;

/// Tweaks' INI, as it names it: beside its DLL.
pub const INI: &[u8] = b"Data\\NVSE\\Plugins\\nvse_stewie_tweaks.ini";

/// Fallout.ini's section of the rows' keys.
pub const SECTION: &str = "nv-rs Stewie";

/// Tweaks on this engine: one slot, shared by the game's threads once installed.
#[derive(Default)]
pub struct Stewie {
    /// The host's row switched it on ([`set_on`]).
    pub on: bool,
    /// Fallout.ini's `[nv-rs Stewie]` keys ([`set_keys`]).
    pub keys: rows::Keys,
    /// [`install`] ran with the row on.
    pub installed: bool,
    /// The settings read at [`install`].
    pub settings: settings::Settings,
}

/// The host's row: Tweaks built in for this run (before [`install`]).
pub fn set_on(e: &mut Engine, on: bool) {
    e.ext::<Stewie>().on = on;
}

/// Fallout.ini's `[nv-rs Stewie]` keys (before [`install`]).
pub fn set_keys(e: &mut Engine, keys: rows::Keys) {
    e.ext::<Stewie>().keys = keys;
}

/// Whether Tweaks is built in and on.
pub fn on(e: &Engine) -> bool {
    e.ext_get::<Stewie>().is_some_and(|s| s.installed)
}

/// Whether Tweaks answers "installed" (`IsPluginInstalled`, `GetPluginVersion`): not while its
/// eight script commands are not built, so a mod that checks for Tweaks never calls one that is
/// missing (the lead, 2026-11-18). The commands' PR turns it on.
pub const ANSWERS_INSTALLED: bool = false;

/// xNVSE's record of Tweaks: listed only when it answers "installed" ([`ANSWERS_INSTALLED`]), no
/// callbacks yet; its opcode base is its co-save section's header.
fn plugin() -> nvse::plugins::BuiltIn {
    nvse::plugins::BuiltIn {
        file: FILE,
        name: NAME,
        version: VERSION,
        opcode_base: OPCODE_BASE,
        listed: ANSWERS_INSTALLED,
        listener: None,
        save: None,
        load: None,
        pre_load: None,
        new_game: None,
    }
}

/// The player's INI: through the mods' files seam (the port's folder beside the saves first, then
/// the game's), then the game's own file calls when no platform answers the seam. `None`: no file.
fn read_ini(e: &mut Engine) -> Option<Vec<u8>> {
    match engine_core::mod_files::read_or_error(e, INI) {
        Ok(bytes) => Some(bytes),
        Err(engine_core::mod_files::ERROR_FILE_NOT_FOUND)
        | Err(engine_core::mod_files::ERROR_PATH_NOT_FOUND)
        | Err(engine_core::mod_files::ERROR_NOT_SUPPORTED) => nvse::saveload::read_file(e, INI),
        // A copy that is there but cannot be read: no later place is looked at (the seam's rule).
        Err(_) => None,
    }
}

/// The settings with the player's INI (if any) over the defaults, and the rows that are on.
pub fn settings_and_rows(
    ini: Option<&[u8]>,
    keys: &rows::Keys,
) -> (settings::Settings, Vec<&'static rows::Row>) {
    let name = String::from_utf8_lossy(INI).into_owned();
    let files: Vec<(&str, &[u8])> = ini.map(|b| (name.as_str(), b)).into_iter().collect();
    let settings = settings::Settings::new(&files, keys.as_tweaks);
    let on = rows::ROWS
        .iter()
        .filter(|r| rows::on(r, keys, &settings))
        .collect();
    (settings, on)
}

/// Builds Tweaks into the engine when the row switched it on: its settings read, its rows' bits,
/// its place in xNVSE's list, its start-up lines in the log. Once per engine; off, or a second
/// call, does nothing.
pub fn install(e: &mut Engine) {
    let state = e.ext::<Stewie>();
    if state.installed || !state.on {
        return;
    }
    let keys = state.keys.clone();
    let ini = read_ini(e);
    let (settings, on) = settings_and_rows(ini.as_deref(), &keys);
    e.clear_stewie();
    for row in &on {
        e.set_stewie(row.bit, true);
    }
    for line in start_lines(ini.is_some(), &settings, &keys, on.len()) {
        eprintln!("{line}");
    }
    let state = e.ext::<Stewie>();
    state.installed = true;
    state.settings = settings;
    nvse::plugins::register(e, plugin());
    // The game's state, not one engine's: real threads' engines share it.
    e.share_ext::<Stewie>();
}

/// The start-up log's lines: each file read and what it changed (or that none was found), the rows
/// on (Fallout.ini keys that could not be used are named in the host's line for its keys).
pub fn start_lines(
    found: bool,
    settings: &settings::Settings,
    keys: &rows::Keys,
    rows_on: usize,
) -> Vec<String> {
    let mut lines = settings.log_lines();
    if !found {
        lines.push(format!(
            "lStewieAl's Tweaks: no {}: {}",
            String::from_utf8_lossy(INI),
            if keys.as_tweaks {
                "Tweaks' own defaults (bMissingKeysAsTweaks=1)"
            } else {
                "VNV's values"
            }
        ));
    }
    lines.push(format!(
        "lStewieAl's Tweaks: {rows_on} of its {} rows built are on",
        rows::ROWS.len()
    ));
    lines
}
