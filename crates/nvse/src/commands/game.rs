//! xNVSE's `Commands_Game.cpp` (xNVSE): the game's and the INI's numeric settings, the references
//! under the crosshair and selected in the console, the scripts told of a loaded or restarted
//! game, the loaded files, form ids, messages and message boxes, the grid count and its local
//! map override, and `AddSpellNS`.
//!
//! Read from xNVSE's source and checked in the shipped DLL (xNVSE 6.4.9's `nvse_1_4.dll`,
//! executes `10025250` to `10025D63`; the records found by name: names, parameters, help texts).
//! `OutputLocalMapPicturesOverride` writes the loop count of `005d4580` (the byte at `005D45A5`)
//! around its call: here the routine with that count is run instead of the game's, through its two
//! callees.

use super::string::{
    format_string_args, game_setting, ini_setting, numeric_ini_setting, separator, setting_number,
    setting_type, MAX_MESSAGE_LENGTH,
};
use super::{console_print, double_words, is_console_mode, Read};
use crate::args::{dtol3, f64_to_i32};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs, Nvse};
use engine_core::extend::{Owner, WrapSlot};
use engine_core::prelude::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_String`, `_Integer`, `_Float`,
/// `_SpellItem`, `_AnyForm`.
const STRING: u32 = 0x00;
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const SPELL_ITEM: u32 = 0x07;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneString`, `kParams_OneString_OneFloat`, `kParams_OneOptionalForm`,
/// `kParams_TwoInts`, `kParams_OneInt`, `kParams_OneSpellItem`, `kParams_FormatString` (xNVSE).
const ONE_STRING: &[Param] = &[p("string", STRING, false)];
const ONE_STRING_ONE_FLOAT: &[Param] = &[p("string", STRING, false), p("float", FLOAT, false)];
const ONE_OPTIONAL_FORM: &[Param] = &[p("form", ANY_FORM, true)];
const TWO_INTS: &[Param] = &[p("int", INT, false), p("int", INT, false)];
const ONE_INT: &[Param] = &[p("int", INT, false)];
const ONE_SPELL_ITEM: &[Param] = &[p("spell", SPELL_ITEM, false)];
const FORMAT_STRING: [Param; 21] = super::string::fmt_params(&[]);

/// The commands' name buffers (`char settingName[512]`, `char modName[512]`).
const NAME_BUFFER: usize = 512;

/// `InterfaceManager::GetSingleton` (`*0x011D8A80`): its `debugSelection` (`+0xF0`) and
/// `crosshairRef` (`+0xFC`), as the DLL reads them (`100259C6`, `100255D5`).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const DEBUG_SELECTION: u32 = 0xf0;
const CROSSHAIR_REF: u32 = 0xfc;
/// `TESForm::flags` (`+8`; `0x4000`, temporary) and `refID` (`+0xC`).
const FORM_FLAGS: u32 = 8;
const FORM_TEMPORARY: u32 = 0x4000;
const FORM_ID: u32 = 0xc;
/// `uGridsToLoad` (`kuGridsToLoadAddr`, `0x011C63D0`).
const GRIDS_TO_LOAD: u32 = 0x011c_63d0;
/// `DataHandler`'s singleton (`0x011C3F2C`) and its list of files (`modList.modInfoList`,
/// `+0x210`); a file's name is at `+0x20` (`ModInfo`).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const MOD_INFO_LIST: u32 = 0x210;
const MOD_NAME: u32 = 0x20;
/// `QueueUIMessage` (cdecl, six words: the text, the emotion, the icon, the sound, the seconds
/// as a float, a byte) and `ShowMessageBox` (cdecl, nine words and the buttons, ended by 0).
const QUEUE_UI_MESSAGE: u32 = 0x0070_52f0;
const SHOW_MESSAGE_BOX: u32 = 0x0070_3e80;
/// `ShowMessageBox_Callback` (the game's own answer callback, `005B4A70`), the message box's
/// owner (`ShowMessageBox_pScriptRefID`, `0x011CAC64`) and answer (`ShowMessageBox_button`,
/// `0x0118C684`).
const MESSAGE_BOX_CALLBACK: u32 = 0x005b_4a70;
const MESSAGE_BOX_OWNER: u32 = 0x011c_ac64;
const MESSAGE_BOX_ANSWER: u32 = 0x0118_c684;
/// `Cmd_AddSpell_Execute` (`0x005C1D40`).
const ADD_SPELL: u32 = 0x005c_1d40;

/// `QueueUIMessage`'s entry: `ToggleUIMessages` (`Hooks_Gameplay.cpp` (xNVSE)) writes a `ret`
/// there while `AddSpellNS` runs AddSpell and puts the `push ebp` back after.
pub const QUEUE_UI_MESSAGE_ENTRY: WrapSlot = WrapSlot::new(Owner::Nvse, QUEUE_UI_MESSAGE);

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns,
    }
}

const RD: Returns = Returns::Default;
const RF: Returns = Returns::Form;

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x149a, "GetNumericGameSetting", "", "returns the value of a numeric game setting", ONE_STRING, get_numeric_game_setting, RD),
    cmd(0x149b, "SetNumericGameSetting", "", "sets a numeric game setting", ONE_STRING_ONE_FLOAT, set_numeric_game_setting, RD),
    cmd(0x149c, "GetNumericIniSetting", "", "returns the value of an ini setting", ONE_STRING, get_numeric_ini_setting, RD),
    cmd(0x149d, "SetNumericIniSetting", "", "sets an ini setting", ONE_STRING_ONE_FLOAT, set_numeric_ini_setting, RD),
    cmd(0x14a7, "GetCrosshairRef", "", "returns the reference under the crosshair", &[], get_crosshair_ref, RF),
    cmd(0x14a8, "GetGameRestarted", "", "returns 1 when the game is restarted", &[], get_game_restarted, RD),
    cmd(0x14ac, "GetGameLoaded", "", "returns 1 after a game is loaded", &[], get_game_loaded, RD),
    cmd(0x14ae, "IsModLoaded", "", "returns the whether the specified mod is loaded", ONE_STRING, is_mod_loaded, RD),
    cmd(0x14af, "GetModIndex", "", "returns the index of the specified mod", ONE_STRING, get_mod_index, RD),
    cmd(0x14b0, "GetNumLoadedMods", "", "returns the number of loaded mods", &[], get_num_loaded_mods, RD),
    cmd(0x14b1, "GetSourceModIndex", "", "returns the index of the mod associated with the form", ONE_OPTIONAL_FORM, get_source_mod_index, RD),
    cmd(0x14b2, "GetDebugSelection", "", "returns the current selected object in the console", &[], get_debug_selection, RF),
    cmd(0x14d8, "GetLocalRefIndex", "", "returns the index of the ref", ONE_OPTIONAL_FORM, get_local_ref_index, RD),
    cmd(0x14d9, "BuildRef", "", "builds a reference from a mod and ref index", TWO_INTS, build_ref, RF),
    cmd(0x14db, "MessageEx", "", "prints a formatted message", &FORMAT_STRING, message_ex, RD),
    cmd(0x14dc, "MessageBoxEx", "", "displays a formatted message box", &FORMAT_STRING, message_box_ex, RD),
    cmd(0x15e6, "GetGridsToLoad", "", "returns the effective value of the uGridsToLoad ini setting", &[], get_grids_to_load, RD),
    cmd(0x15e7, "OutputLocalMapPicturesOverride", "OLMPOR", "\"identical to the OutputLocalMapPictures console command, but overrides the uGridsToLoad ini setting\"", &[], output_local_map_pictures_override, RD),
    cmd(0x15e8, "SetOutputLocalMapPicturesGrids", "SetOLMPGrids", "sets the value with which to override uGridsToLoad when generating local maps with OLMPOR", ONE_INT, set_output_local_map_pictures_grids, RD),
    cmd(0x15ef, "AddSpellNS", "", "identical to AddSpell but without the UI message", ONE_SPELL_ITEM, add_spell_ns, RD),
];

/// What these commands keep, game-wide as xNVSE's statics are.
pub struct State {
    /// `s_gameLoadedInformedScripts`: the scripts (by form id) `GetGameLoaded` answered 1 since
    /// the last load or new game.
    pub(crate) loaded_informed: HashSet<u32>,
    /// `s_gameRestartedInformedScripts`: the scripts `GetGameRestarted` answered 1, never
    /// forgotten while the game runs.
    pub(crate) restarted_informed: HashSet<u32>,
    /// `sGridsToLoad_Override` (-1: none), the square `SetOutputLocalMapPicturesGrids` stored for
    /// `OutputLocalMapPicturesOverride`.
    pub grids_override: i8,
    /// The whole square that byte was cut from (0 with none): what the list and scan guard row
    /// draws.
    pub grids_square: u32,
}

impl Default for State {
    fn default() -> State {
        State {
            loaded_informed: HashSet::new(),
            restarted_informed: HashSet::new(),
            grids_override: -1,
            grids_square: 0,
        }
    }
}

/// Whether `QueueUIMessage` returns at once (xNVSE's `ret` at its entry): one flag for every
/// thread, as the code byte is.
#[derive(Default)]
pub struct UiMessages {
    off: AtomicBool,
}

/// The wrap over `QueueUIMessage` and its flag.
pub fn install(e: &mut Engine) {
    e.ext_free::<UiMessages>();
    e.wrap(QUEUE_UI_MESSAGE_ENTRY, queue_ui_message);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_SaveLoad.cpp: DoLoadGameHook and
// NewGameHook (their `s_gameLoadedInformedScripts.Clear()`).
/// A game was loaded or started: every script is told of it once more.
pub fn game_loaded(e: &mut Engine) {
    e.ext::<Nvse>().game.loaded_informed.clear();
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp: ToggleUIMessages.
/// `QueueUIMessage` while the messages are off returns at once, answering 0 as the game's own
/// function always does (xNVSE's `ret` leaves the caller's `eax`; every translated caller drops
/// the answer).
fn queue_ui_message(e: &mut Engine, a: &[u32]) -> Ret {
    if e.ext_free::<UiMessages>().off.load(Ordering::Relaxed) {
        return Ret::default();
    }
    e.call_next(QUEUE_UI_MESSAGE, a)
}

/// `ToggleUIMessages(on)`.
fn ui_messages(e: &mut Engine, on: bool) {
    e.ext_free::<UiMessages>().off.store(!on, Ordering::Relaxed);
}

/// A string argument xNVSE's reader copies into the command's 512-byte buffer on its stack: its
/// text up to the first 0. The reader copies a `$` variable's text whole (`memcpy` of its
/// length and terminator), so a text that does not fit runs into the command's frame in xNVSE;
/// with the list guard row (`NV_NVSE_LIST_GUARD`) it is used whole, and off (vanilla) the port
/// stops the game there (see [`super::arg_in_buffer`]).
fn name_arg(e: &Engine, read: &Read, command: &str) -> Vec<u8> {
    super::arg_in_buffer(e, read, 0, NAME_BUFFER, command)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetNumericGameSetting_Execute; GameSettings.cpp: GetNumericGameSetting.
/// The setting's number (`Setting::Get`); -1 for a failed read, an empty name, no such setting
/// or one that holds no number. The console prints the number with `%g`, or "SETTING NOT FOUND"
/// for the rest (a failed read prints nothing).
fn get_numeric_game_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = name_arg(e, &read, "GetNumericGameSetting");
    let value = if name.is_empty() {
        None
    } else {
        match game_setting(e, &name) {
            0 => None,
            setting => setting_number(e, setting),
        }
    };
    report_number(e, &a, value, "GetNumericGameSetting");
    handled(true)
}

/// The result of the numeric getters and their console lines.
fn report_number(e: &mut Engine, a: &CommandArgs, value: Option<f64>, command: &str) {
    match value {
        Some(v) => {
            a.set_result(e, v);
            if is_console_mode(e) {
                console_print(e, &format!("{command} >> %g"), &double_words(v));
            }
        }
        None => {
            if is_console_mode(e) {
                console_print(e, &format!("{command} >> SETTING NOT FOUND"), &[]);
            }
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_SetNumericGameSetting_Execute.
/// The setting set to the float read (`Setting::Set`), the result 1 when it holds a number; the
/// console prints "NOT FOUND" for no such setting (an empty name is looked up too).
fn set_numeric_game_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = name_arg(e, &read, "SetNumericGameSetting");
    let value = f64::from(read.float(1, 0.0));
    match game_setting(e, &name) {
        0 => {
            if is_console_mode(e) {
                console_print(e, "SetNumericGameSetting >> NOT FOUND", &[]);
            }
        }
        setting => {
            if set_setting_number(e, setting, value) {
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetNumericIniSetting_Execute; GameSettings.cpp: GetNumericIniSetting.
/// An INI setting's number (the preferences' first, then the INI's, by name without case), as
/// [`get_numeric_game_setting`] answers and prints.
fn get_numeric_ini_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = name_arg(e, &read, "GetNumericIniSetting");
    let value = numeric_ini_setting(e, &name);
    report_number(e, &a, value, "GetNumericIniSetting");
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_SetNumericIniSetting_Execute.
/// An INI setting set in memory (nothing is written to a file), as
/// [`set_numeric_game_setting`] sets a game setting.
fn set_numeric_ini_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = name_arg(e, &read, "SetNumericIniSetting");
    let value = f64::from(read.float(1, 0.0));
    match ini_setting(e, &name) {
        0 => {
            if is_console_mode(e) {
                console_print(e, "SetNumericIniSetting >> NOT FOUND", &[]);
            }
        }
        setting => {
            if set_setting_number(e, setting, value) {
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameSettings.cpp: Setting::Set(double).
/// A setting set to a number (`1007BF60`): a bool to 1 for any value but 0 (NaN included), an
/// integer to the value truncated (`cvttsd2si`: `0x80000000` out of range), an unsigned to the
/// low word of `__dtol3` (-1 is `0xFFFFFFFF`, 2^32 and more wrap, 2^63 and more give 0), a float
/// to the value rounded; nothing set (false) for the other types, a string's included.
fn set_setting_number(e: &mut Engine, setting: u32, v: f64) -> bool {
    let word = match setting_type(e, setting) {
        0 => u32::from(v != 0.0),
        3 => f64_to_i32(v) as u32,
        4 => dtol3(v) as u32,
        5 => (v as f32).to_bits(),
        _ => return false,
    };
    e.mem.set_u32(setting + 4, word);
    true
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetCrosshairRef_Execute.
/// The id of the reference under the crosshair, 0 for none (only the result's low word is
/// written, `REFR_RES`); the console prints it with `%08x`.
fn get_crosshair_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let manager = e.global::<u32>(INTERFACE_MANAGER);
    if manager != 0 {
        let reference = e.mem.u32(manager + CROSSHAIR_REF);
        if reference != 0 {
            let id = e.mem.u32(reference + FORM_ID);
            a.set_form_result(e, id);
        }
    }
    if is_console_mode(e) {
        let id = e.mem.u32(a.result);
        console_print(e, "GetCrosshairRef >> %08x", &[id]);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetGameLoaded_Execute.
/// 1 the first time a script asks after a game was loaded or started, else 0; the console prints
/// the answer. Without a script, 0 and no line.
fn get_game_loaded(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.script != 0 {
        let id = e.mem.u32(a.script + FORM_ID);
        if e.ext::<Nvse>().game.loaded_informed.insert(id) {
            a.set_result(e, 1.0);
        }
        if is_console_mode(e) {
            let v = e.mem.f64(a.result);
            console_print(e, "GetGameLoaded >> %.0f", &double_words(v));
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetGameRestarted_Execute.
/// 1 the first time a script asks while the game runs (a load does not reset it), else 0.
fn get_game_restarted(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let first = a.script != 0 && {
        let id = e.mem.u32(a.script + FORM_ID);
        e.ext::<Nvse>().game.restarted_informed.insert(id)
    };
    a.set_result(e, if first { 1.0 } else { 0.0 });
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameData.cpp:
// DataHandler::LookupModByName (`LoadedModFinder`: `StrCompare` of the file's name with the
// given one, the game's `toupper` of each byte read unsigned, `10079560`).
/// The first file of the list called `name` (without case), passing over empty nodes; 0 for none.
fn lookup_mod_by_name(e: &mut Engine, name: &[u8]) -> u32 {
    let upper = crate::vars::upper_table(e);
    let mut node = e.global::<u32>(DATA_HANDLER) + MOD_INFO_LIST;
    while node != 0 {
        let info = e.mem.u32(node);
        if info != 0 {
            let file = crate::table::guest_text(e, info + MOD_NAME);
            if crate::vars::str_compare(&upper, &file, name).is_eq() {
                return info;
            }
        }
        node = e.mem.u32(node + 4);
    }
    0
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp: Cmd_IsModLoaded_Execute.
/// 1 when a file of that name (without case) is in the list (`ModInfo::IsLoaded` is always
/// true); the console prints "Mod Loaded" or "Mod not loaded" (a failed read prints nothing).
fn is_mod_loaded(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = name_arg(e, &read, "IsModLoaded");
    if lookup_mod_by_name(e, &name) != 0 {
        a.set_result(e, 1.0);
    }
    if is_console_mode(e) {
        let line = if e.mem.f64(a.result) != 0.0 {
            "Mod Loaded"
        } else {
            "Mod not loaded"
        };
        console_print(e, line, &[]);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp: Cmd_GetModIndex_Execute.
/// The file's position in the list (`DataHandler::GetModIndex`: every node counts, empty ones
/// included), as a byte: 255 for none; the console prints it with `%02X`. The result is not
/// written before the read.
fn get_mod_index(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = name_arg(e, &read, "GetModIndex");
    let index = crate::saveload::mod_index(e, &name);
    a.set_result(e, f64::from(index));
    if is_console_mode(e) {
        console_print(e, "Mod Index: %02X", &[u32::from(index)]);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetNumLoadedMods_Execute; GameData.cpp: DataHandler::GetActiveModCount.
/// The files in the list (its nodes that hold one), as a byte (`UInt8`: 256 files answer 0); the
/// console prints it with `%.0f`.
fn get_num_loaded_mods(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut count = 0u32;
    let mut node = e.global::<u32>(DATA_HANDLER) + MOD_INFO_LIST;
    while node != 0 {
        if e.mem.u32(node) != 0 {
            count += 1;
        }
        node = e.mem.u32(node + 4);
    }
    let v = f64::from(count as u8);
    a.set_result(e, v);
    if is_console_mode(e) {
        console_print(e, "Mods Loaded: %.0f", &double_words(v));
    }
    handled(true)
}

/// The form a form-id command reads: the one given, else the calling reference; 0 for neither.
/// `None` when the read failed.
fn form_or_this(e: &mut Engine, a: &CommandArgs) -> Option<u32> {
    let read = Read::of(e, a);
    if !read.ok {
        return None;
    }
    Some(match read.form(0) {
        0 => a.this_obj,
        form => form,
    })
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetSourceModIndex_Execute.
/// The form's mod index (the top byte of its id), -1 without a form or a calling reference.
fn get_source_mod_index(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    if let Some(form) = form_or_this(e, &a).filter(|&f| f != 0) {
        let index = e.mem.u8(form + FORM_ID + 3);
        a.set_result(e, f64::from(index));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetLocalRefIndex_Execute.
/// The form's id without its mod index (the low 24 bits), -1 without a form or a calling
/// reference.
fn get_local_ref_index(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    if let Some(form) = form_or_this(e, &a).filter(|&f| f != 0) {
        let id = e.mem.u32(form + FORM_ID) & 0x00ff_ffff;
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp: Cmd_BuildRef_Execute.
/// The form id of a mod index (its low byte) and a local index (its low 24 bits), in the
/// result's low word over a zeroed double. Nothing is looked up: the id need not be a form's.
/// Both integers are required, so the compiler never leaves one unread (xNVSE's are not
/// initialised; the DLL keeps the second in the result pointer's stack slot, `10025968`); here
/// an unread one is 0.
fn build_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let (mod_index, ref_index) = (read.int(0, 0), read.int(1, 0));
        a.set_form_result(e, (mod_index << 24) | (ref_index & 0x00ff_ffff));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetDebugSelection_Execute.
/// The id of the console's selected reference (in the low word of a zeroed double), 0 for none.
fn get_debug_selection(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let manager = e.global::<u32>(INTERFACE_MANAGER);
    if manager != 0 {
        let selection = e.mem.u32(manager + DEBUG_SELECTION);
        if selection != 0 {
            let id = e.mem.u32(selection + FORM_ID);
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

/// Runs `f` with `text` as a NUL-terminated string in a buffer of `kMaxMessageLength` bytes on
/// the guest stack (xNVSE's `char buffer[kMaxMessageLength]`), as the game is handed it.
fn with_message_buffer<T>(e: &mut Engine, text: &[u8], f: impl FnOnce(&mut Engine, u32) -> T) -> T {
    let size = (MAX_MESSAGE_LENGTH.max(text.len() + 1) as u32).next_multiple_of(4);
    e.with_stack(size, |e, at| {
        e.mem.write(at.addr(), text);
        e.mem.set_u8(at.addr() + text.len() as u32, 0);
        f(e, at.addr())
    })
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp: Cmd_MessageEx_Execute.
/// The formatted text queued as a message (`QueueUIMessage(text, 0, NULL, NULL, 2.0, 0)`), the
/// result 1 (set before the call); a format that fails queues nothing and answers 0.
fn message_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let f = format_string_args(e, &a, FORMAT_STRING.len() as u32);
    if f.ok {
        a.set_result(e, 1.0);
        with_message_buffer(e, &f.text, |e, text| {
            e.call(
                QUEUE_UI_MESSAGE,
                &args![text, 0u32, 0u32, 0u32, 2.0f32, 0u32],
            );
        });
    }
    handled(true)
}

/// `MessageBoxEx`'s text cut at its separators (`GetSeparatorChar`: `|`, or `@` for a console
/// line of no mod), up to its first 0: each of the first ten becomes a 0 and starts a button
/// there; the offsets of the buttons' texts in it. Past the tenth the rest stays the last
/// button's, separators and all.
fn split_buttons(text: &mut [u8], sep: u8) -> Vec<usize> {
    let mut buttons = Vec::new();
    let mut i = 0;
    while i < text.len() && text[i] != 0 && buttons.len() < 10 {
        if text[i] == sep {
            text[i] = 0;
            buttons.push(i + 1);
        }
        i += 1;
    }
    buttons
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_MessageBoxEx_Execute.
/// The formatted text shown as a message box through the game's own (`ShowMessageBox` with its
/// answer callback, as `MessageBox` does), its buttons cut from the text at the separators (up
/// to ten; "Ok" when there is none). The box belongs to the calling reference when there is one
/// that is not temporary, else to the script, and its answer is reset (`0xFF`) first, so
/// `GetButtonPressed` reads this box's. The result stays 0; a format that fails shows nothing.
fn message_box_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let f = format_string_args(e, &a, FORMAT_STRING.len() as u32);
    if !f.ok {
        return handled(true);
    }
    let mut text = f.text;
    let sep = separator(e, a.script);
    let buttons = split_buttons(&mut text, sep);
    let owner = if a.this_obj != 0 && e.mem.u32(a.this_obj + FORM_FLAGS) & FORM_TEMPORARY == 0 {
        a.this_obj
    } else {
        a.script
    };
    let owner_id = e.mem.u32(owner + FORM_ID);
    let ok = crate::table::guest_str(e, b"Ok");
    with_message_buffer(e, &text, |e, at| {
        let mut words = vec![at, 0, 0, MESSAGE_BOX_CALLBACK, 0, 0x17, 0, 0];
        let mut b = [0u32; 10];
        for (slot, offset) in b.iter_mut().zip(&buttons) {
            *slot = at + *offset as u32;
        }
        if buttons.is_empty() {
            b[0] = ok;
        }
        words.extend_from_slice(&b);
        words.push(0);
        e.set_global(MESSAGE_BOX_OWNER, owner_id);
        e.set_global(MESSAGE_BOX_ANSWER, 0xffu8);
        e.call(SHOW_MESSAGE_BOX, &words);
    });
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetGridsToLoad_Execute.
/// `uGridsToLoad` as the game holds it, unsigned.
fn get_grids_to_load(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let grids = e.global::<u32>(GRIDS_TO_LOAD);
    a.set_result(e, f64::from(grids));
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_SetOutputLocalMapPicturesGrids_Execute.
/// For a count above 0, odd and no larger than `uGridsToLoad` (compared unsigned): its square
/// kept as a signed byte (13 keeps -87, the square's low byte, `10025BE1`), the result 1.
fn set_output_local_map_pictures_grids(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let grids = read.int(0, 0) as i32;
    if read.ok && grids > 0 && grids as u32 <= e.global::<u32>(GRIDS_TO_LOAD) && grids & 1 != 0 {
        a.set_result(e, 1.0);
        let low = grids as i8;
        e.ext::<Nvse>().game.grids_override = low.wrapping_mul(low);
        e.ext::<Nvse>().game.grids_square = (grids as u32).wrapping_mul(grids as u32);
    }
    handled(true)
}

/// `Cmd_OutputLocalMapPictures_Execute` (`005d4580`), the flag it sets while it runs, and the two
/// routines it calls: the map menu's reset (`005d45c0`) and one local map picture (`005d45e0`).
const OUTPUT_LOCAL_MAP_PICTURES: u32 = 0x005d_4580;
const LOCAL_MAP_BUSY: u32 = 0x0120_04fc;
const LOCAL_MAP_RESET: u32 = 0x005d_45c0;
const LOCAL_MAP_PICTURE: u32 = 0x005d_45e0;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_OutputLocalMapPicturesOverride_Execute, with FalloutNV.exe's 005d4580 as its byte at
// 005D45A5 makes it.
/// The console's `OutputLocalMapPictures` (the busy flag set, the map menu reset, a picture per
/// grid cell, the flag cleared), its count the square `SetOutputLocalMapPicturesGrids` kept in
/// place of 25. xNVSE writes the square's low byte as the `cmp`'s 8-bit immediate, which the
/// game reads sign-extended against an unsigned loop counter: a byte of 0x80 or more (13 and 15
/// grids, 169 and 225) runs about four billion pictures, and larger grids wrap (17 grids, 289,
/// draws 33). With the list and scan guard row ([`super::array::LIST_GUARD`]) the whole square.
/// Without a kept square, the game's own command with the same arguments.
fn output_local_map_pictures_override(e: &mut Engine, w: &[u32]) -> Ret {
    let (square, whole) = {
        let g = &e.ext::<Nvse>().game;
        (g.grids_override, g.grids_square)
    };
    if square == -1 {
        e.call(OUTPUT_LOCAL_MAP_PICTURES, w);
        return handled(true);
    }
    let count = if super::array::fix_on(e, super::array::LIST_GUARD) {
        whole
    } else {
        i32::from(square) as u32
    };
    e.mem.set_u8(LOCAL_MAP_BUSY, 1);
    e.call(LOCAL_MAP_RESET, &[]);
    for _ in 0..count {
        e.call(LOCAL_MAP_PICTURE, &[]);
    }
    e.mem.set_u8(LOCAL_MAP_BUSY, 0);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp: Cmd_AddSpellNS_Execute;
// Hooks_Gameplay.cpp: RunCommand_NS.
/// The game's `AddSpell` run with the same arguments while `QueueUIMessage` returns at once
/// (every thread's call of it in that time, as xNVSE's code byte is the process's), its answer
/// passed on. The console lines are AddSpell's own and stay.
fn add_spell_ns(e: &mut Engine, w: &[u32]) -> Ret {
    ui_messages(e, false);
    let answer = e.call(ADD_SPELL, w).eax & 0xff;
    ui_messages(e, true);
    Ret {
        eax: answer,
        ..Ret::default()
    }
}

#[cfg(test)]
#[path = "game_tests.rs"]
mod tests;
