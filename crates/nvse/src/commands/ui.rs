//! xNVSE's `Commands_UI.cpp` (xNVSE): the UI's trait values read and set by component path
//! (`GetUIFloat` and the `Alt` forms, `GetUIFloatInherited`, `ModUIFloat`), a list box's items
//! sorted (`SortUIListBox`), the active tile's name, VATS ended and the level-up menu shown;
//! and what they share from xNVSE's `GameTiles.cpp` and `GameUI.cpp` (the menu by name, a tile's
//! child and component by path, a trait by name).
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`). Two things kept as xNVSE has them:
//! `Tile::GetChild`, which the commands without `Alt` use, walks a tile's children through
//! xNVSE's singly linked list type over the game's doubly linked one (the first child, then the
//! last back to the second); the `Alt` forms walk them in order. And `SortUIListBox` orders the
//! items with MSVC's `std::sort` (not stable), so items that compare equal end where it puts
//! them.
//!
//! One difference, kept on purpose. xNVSE keeps the trait values it found in one cache keyed by
//! the path alone, shared by the lookups with and without `Alt`, and empties it whenever a tile is
//! made or destroyed. Here every call looks the value up: the answers differ only when a script
//! reads the same path both ways under a parent with several children of one name, where xNVSE
//! answers the first form's child for both until a tile changes.
//!
//! A fixes row ([`super::array::UI_SORT_PATHS`]): `SortUIListBox` with a `/` in a key, destination
//! or increment path. xNVSE's lookup writes a 0 over the path's slashes in the spec itself, so
//! only the first shown item is found by it and every later item is skipped (the list does not
//! sort); that is vanilla. With the row every item is looked up with the whole path.

use super::string::{format_string_args_at, Formatted};
use super::{console_print, double_words, is_console_mode, Read};
use crate::args::Arg;
use crate::table::{guest_text, Command, Param, Returns};
use crate::vars;
use crate::{handled, CommandArgs};
use engine_core::prelude::*;
use std::cmp::Ordering;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// `kParamType_String`, `_Integer`, `_Float` (xNVSE `CommandTable.h`).
const STRING: u32 = 0x00;
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
/// `kParams_OneString`, `kParams_OneString_OneFloat`, `kParams_TwoStrings`,
/// `kParams_OneString_OneOptionalInt` and `kParams_SetUIStringEx` (the component's name, then a
/// format string and its twenty arguments).
const ONE_STRING: &[Param] = &[p("string", STRING, false)];
const ONE_STRING_ONE_FLOAT: &[Param] = &[p("string", STRING, false), p("float", FLOAT, false)];
const TWO_STRINGS: &[Param] = &[p("string", STRING, false), p("string", STRING, false)];
const ONE_STRING_ONE_OPTIONAL_INT: &[Param] = &[p("string", STRING, false), p("int", INT, true)];
const SET_UI_STRING_EX: [Param; 22] = set_ui_string_ex_params();

/// `kParams_SetUIStringEx`: the component's name, then `FORMAT_STRING_PARAMS` (the format string
/// and twenty optional floats).
const fn set_ui_string_ex_params() -> [Param; 22] {
    let mut out = [p("variable", FLOAT, true); 22];
    out[0] = p("component name", STRING, false);
    out[1] = p("format string", STRING, false);
    out
}

const fn cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        opcode,
        name,
        short: "",
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns: Returns::Default,
    }
}

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x14a4, "GetUIFloat", "returns the value of a float UI trait", ONE_STRING, get_ui_float),
    cmd(0x14a5, "SetUIFloat", "sets the value of a float UI trait", ONE_STRING_ONE_FLOAT, set_ui_float),
    cmd(0x14a6, "SetUIString", "sets the value of a string UI trait", TWO_STRINGS, set_ui_string),
    cmd(0x14d6, "SortUIListBox", "sorts the items in a UI list_box", TWO_STRINGS, sort_ui_list_box),
    cmd(0x14e2, "SetUIStringEx", "sets the value of a string UI trait to a formatted string", &SET_UI_STRING_EX, set_ui_string_ex),
    cmd(0x15b5, "EndVATScam", "\"Remove all targets and close VATS mode\"", &[], end_vats_cam),
    cmd(0x161e, "ShowLevelUpMenu", "shows the LevelUp menu", &[], show_level_up_menu),
    cmd(0x161f, "GetUIFloatAlt", "", ONE_STRING, get_ui_float_alt),
    cmd(0x1620, "SetUIFloatAlt", "", ONE_STRING_ONE_FLOAT, set_ui_float_alt),
    cmd(0x1621, "SetUIStringAlt", "", &SET_UI_STRING_EX, set_ui_string_alt),
    cmd(0x163a, "ModUIFloat", "", ONE_STRING_ONE_FLOAT, mod_ui_float),
    cmd(0x163e, "PrintActiveTile", "prints name of highlighted UI component for debug purposes", &[], print_active_tile),
    cmd(0x168b, "GetUIFloatInherited", "", ONE_STRING_ONE_OPTIONAL_INT, get_ui_float_inherited),
];

/// What the `Get` commands answer when the trait is not there (`fErrorReturnValue`).
const NOT_FOUND: f64 = -999.0;

/// `TraitNameToID` (`0x00A01860`, `cdecl`), `Tile::SetFloatValue` (`0x00A012D0`) and
/// `Tile::SetStringValue` (`0x00A01350`, both `thiscall`: the trait's id, the value, propagate).
const TRAIT_NAME_TO_ID: u32 = 0x00a0_1860;
const SET_FLOAT_VALUE: u32 = 0x00a0_12d0;
const SET_STRING_VALUE: u32 = 0x00a0_1350;
/// The menus' tiles by menu id (`g_tileMenuArray`, `*0x011F350C`, from `kMenuType_Min`).
const TILE_MENU_ARRAY: u32 = 0x011f_350c;
const MENU_MIN: u32 = 0x3e9;
/// `InterfaceManager::GetSingleton` (`*0x011D8A80`) and its `activeTile` (`+0xCC`).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const ACTIVE_TILE: u32 = 0xcc;
/// `VATSCameraData` (the object at `0x011F2250`: its targets `+0`, its mode `+8`), its `Unclick`
/// (`0x009C88A0`, `thiscall`, two outputs) and `Accept` (`0x00705780`); the mode
/// `kVATSMode_LimbSelectOrZoom` (2).
const VATS_CAMERA: u32 = 0x011f_2250;
const VATS_MODE: u32 = 8;
const VATS_UNCLICK: u32 = 0x009c_88a0;
const VATS_ACCEPT: u32 = 0x0070_5780;
const LIMB_SELECT_OR_ZOOM: u32 = 2;
/// `DoShowLevelUpMenu` (`0x00784C80`).
const SHOW_LEVEL_UP_MENU: u32 = 0x0078_4c80;

/// A `Tile`: its children (`+4`, the game's doubly linked list: head `+4`, tail `+8`; a node is
/// next `+0`, previous `+4`, tile `+8`), its trait values (`BSSimpleArray<Value *>`: data `+0x14`,
/// count `+0x18`, sorted by id), its name (`+0x20`) and its parent (`+0x28`).
const CHILDREN: u32 = 4;
const VALUES: u32 = 0x14;
const VALUE_COUNT: u32 = 0x18;
const NAME: u32 = 0x20;
const PARENT: u32 = 0x28;
/// A `Tile::Value`: its trait id `+0`, its tile `+4`, its number `+8`, its string `+0xC`.
const VALUE_TILE: u32 = 4;
const VALUE_NUM: u32 = 8;
const VALUE_STR: u32 = 0xc;
/// `kTileValue_locus`.
const LOCUS: u32 = 0x0fa8;

/// The menus by name (`s_menuNameToID`, `GameUI.cpp`), found without case.
const MENUS: &[(&str, u32)] = &[
    ("MessageMenu", 0x3e9),
    ("InventoryMenu", 0x3ea),
    ("StatsMenu", 0x3eb),
    ("HUDMainMenu", 0x3ec),
    ("LoadingMenu", 0x3ef),
    ("ContainerMenu", 0x3f0),
    ("DialogMenu", 0x3f1),
    ("SleepWaitMenu", 0x3f4),
    ("StartMenu", 0x3f5),
    ("LockpickMenu", 0x3f6),
    ("QuantityMenu", 0x3f8),
    ("MapMenu", 0x3ff),
    ("BookMenu", 0x402),
    ("LevelUpMenu", 0x403),
    ("RepairMenu", 0x40b),
    ("RaceSexMenu", 0x40c),
    ("CharGenMenu", 0x418),
    ("TextEditMenu", 0x41b),
    ("BarterMenu", 0x41d),
    ("SurgeryMenu", 0x41e),
    ("HackingMenu", 0x41f),
    ("VATSMenu", 0x420),
    ("ComputersMenu", 0x421),
    ("RepairServicesMenu", 0x422),
    ("TutorialMenu", 0x423),
    ("SpecialBookMenu", 0x424),
    ("ItemModMenu", 0x425),
    ("LoveTesterMenu", 0x432),
    ("CompanionWheelMenu", 0x433),
    ("TraitSelectMenu", 0x434),
    ("RecipeMenu", 0x435),
    ("SlotMachineMenu", 0x438),
    ("BlackjackMenu", 0x439),
    ("RouletteMenu", 0x43a),
    ("CaravanMenu", 0x43b),
    ("TraitMenu", 0x43c),
];

/// `SlashPos` (xNVSE `utility.cpp`): the first `/` or `\`.
fn slash_pos(path: &[u8]) -> Option<usize> {
    path.iter().position(|&b| b == b'/' || b == b'\\')
}

/// A guest copy of `text` for the length of `f` (a C string on the stack).
fn with_text<T>(e: &mut Engine, text: &[u8], f: impl FnOnce(&mut Engine, u32) -> T) -> T {
    let size = (text.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, at| {
        e.mem.write(at.addr(), text);
        e.mem.set_u8(at.addr() + text.len() as u32, 0);
        f(e, at.addr())
    })
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameUI.cpp: InterfaceManager::GetMenuByPath.
/// The menu named by `path` up to its first slash (its tile, 0 for none) and where that slash is.
fn menu_by_path(e: &mut Engine, path: &[u8]) -> (u32, Option<usize>) {
    let slash = slash_pos(path);
    let name = &path[..slash.unwrap_or(path.len())];
    let id = MENUS
        .iter()
        .find(|(n, _)| n.as_bytes().eq_ignore_ascii_case(name))
        .map_or(0, |&(_, id)| id);
    if id == 0 {
        return (0, slash);
    }
    let array = e.mem.u32(TILE_MENU_ARRAY);
    let menu = if array == 0 {
        0
    } else {
        e.mem.u32(array + 4 * (id - MENU_MIN))
    };
    (menu, slash)
}

/// A tile's name (empty without one).
fn tile_name(e: &Engine, tile: u32) -> Vec<u8> {
    let data = e.mem.u32(tile + NAME);
    if data == 0 {
        Vec::new()
    } else {
        guest_text(e, data)
    }
}

/// `StrCompare(name, wanted) == 0` (xNVSE `utility.cpp`): the same without case, by the game's
/// upper-case table; a tile without a name matches nothing.
fn names_match(e: &mut Engine, tile: u32, wanted: &[u8]) -> bool {
    if e.mem.u32(tile + NAME) == 0 {
        return false;
    }
    let name = tile_name(e, tile);
    let upper = vars::upper_table(e);
    vars::str_compare(&upper, &name, wanted) == Ordering::Equal
}

/// A child's name and index in `childName[:index]`, the index `atoi`'s; `None` for a name that
/// starts with the colon.
fn child_spec(child: &[u8]) -> Option<(&[u8], i32)> {
    match child.iter().position(|&b| b == b':') {
        Some(0) => None,
        Some(at) => Some((&child[..at], atoi(&child[at + 1..]))),
        None => Some((child, 0)),
    }
}

/// `atoi`: leading spaces, a sign, digits (wrapping as the C runtime's 32-bit result does).
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = (v * 10 + i64::from(s[i] - b'0')).min(i64::from(i32::MAX) + 1);
        i += 1;
    }
    if negative {
        (-v).max(i64::from(i32::MIN)) as i32
    } else {
        v.min(i64::from(i32::MAX)) as i32
    }
}

/// The most nodes a child list is walked through (a broken list ends there).
const MAX_CHILDREN: u32 = 1 << 16;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetChild.
/// The child named `child` (`*` for any; `name:n` for the `n`-th such), walked as xNVSE's `tList`
/// iterator walks the game's doubly linked list: a node read as data and next, so the first child,
/// then the last back to the second.
fn child_old(e: &mut Engine, tile: u32, child: &[u8]) -> u32 {
    let Some((name, mut index)) = child_spec(child) else {
        return 0;
    };
    let any = name.first() == Some(&b'*');
    let mut node = tile + CHILDREN;
    for _ in 0..MAX_CHILDREN {
        let (data, next) = (e.mem.u32(node), e.mem.u32(node + 4));
        if data == 0 && next == 0 {
            break;
        }
        if data != 0 {
            let candidate = e.mem.u32(data + 8);
            if candidate != 0 && (any || names_match(e, candidate, name)) {
                if index == 0 {
                    return candidate;
                }
                index = index.wrapping_sub(1);
            }
        }
        if next == 0 {
            break;
        }
        node = next;
    }
    0
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetChildAlt.
/// [`child_old`] over the children in their order.
fn child_alt(e: &mut Engine, tile: u32, child: &[u8]) -> u32 {
    let Some((name, mut index)) = child_spec(child) else {
        return 0;
    };
    let any = name.first() == Some(&b'*');
    let mut node = e.mem.u32(tile + CHILDREN);
    for _ in 0..MAX_CHILDREN {
        if node == 0 {
            break;
        }
        let candidate = e.mem.u32(node + 8);
        if candidate != 0 && (any || names_match(e, candidate, name)) {
            if index == 0 {
                return candidate;
            }
            index = index.wrapping_sub(1);
        }
        node = e.mem.u32(node);
    }
    0
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetValue (its binary
// search, written in assembly).
/// The tile's value of trait `id`, 0 for none.
fn value_of(e: &Engine, tile: u32, id: u32) -> u32 {
    let data = e.mem.u32(tile + VALUES);
    let (mut lo, mut hi) = (0u32, e.mem.u32(tile + VALUE_COUNT));
    while lo != hi {
        let mid = lo.wrapping_add(hi) >> 1;
        let value = e.mem.u32(data + 4 * mid);
        let at = e.mem.u32(value);
        if at == id {
            return value;
        }
        if at < id {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    0
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetValueName.
/// The tile's value of the trait named `name` (the game's `TraitNameToID`).
fn value_named(e: &mut Engine, tile: u32, name: &[u8]) -> u32 {
    let id = with_text(e, name, |e, at| e.call(TRAIT_NAME_TO_ID, &args![at]).u32());
    value_of(e, tile, id)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetComponent.
/// The tile at `path` below `tile` ([`child_old`] for each name) and, when its last part names no
/// child, that part as a trait; `None` when a tile on the way is not there.
fn component<'a>(e: &mut Engine, tile: u32, path: &'a [u8]) -> Option<(u32, Option<&'a [u8]>)> {
    let mut parent = tile;
    let mut rest = path;
    while let Some(slash) = slash_pos(rest) {
        parent = child_old(e, parent, &rest[..slash]);
        if parent == 0 {
            return None;
        }
        rest = &rest[slash + 1..];
    }
    if !rest.is_empty() {
        let child = child_old(e, parent, rest);
        if child != 0 {
            return Some((child, None));
        }
        return Some((parent, Some(rest)));
    }
    Some((parent, None))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetComponentValue.
/// The value at `path` below `tile`, 0 when the path ends at a tile or is not there.
fn component_value(e: &mut Engine, tile: u32, path: &[u8]) -> u32 {
    match component(e, tile, path) {
        Some((tile, Some(name))) => value_named(e, tile, name),
        _ => 0,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetComponentValueAlt.
/// [`component_value`] with the children in their order ([`child_alt`]); the last part is always
/// the trait.
fn component_value_alt(e: &mut Engine, tile: u32, path: &[u8]) -> u32 {
    let mut parent = tile;
    let mut rest = path;
    while let Some(slash) = slash_pos(rest) {
        parent = child_alt(e, parent, &rest[..slash]);
        if parent == 0 {
            return 0;
        }
        rest = &rest[slash + 1..];
    }
    if rest.is_empty() {
        0
    } else {
        value_named(e, parent, rest)
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameUI.cpp:
// InterfaceManager::GetMenuComponentValue and GetMenuComponentValueAlt.
/// The value at `path` (`Menu/tile/.../trait`), 0 when it is not there.
fn menu_component_value(e: &mut Engine, path: &[u8], alt: bool) -> u32 {
    let (menu, slash) = menu_by_path(e, path);
    match slash {
        Some(slash) if menu != 0 => {
            let rest = &path[slash + 1..];
            if alt {
                component_value_alt(e, menu, rest)
            } else {
                component_value(e, menu, rest)
            }
        }
        _ => 0,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameUI.cpp:
// InterfaceManager::GetMenuComponentTile; GameTiles.cpp: Tile::GetComponentTile.
/// The tile at `path` (`Menu/tile/...`): the menu's own tile for a path without a slash.
fn menu_component_tile(e: &mut Engine, path: &[u8]) -> u32 {
    let (menu, slash) = menu_by_path(e, path);
    match slash {
        Some(slash) if menu != 0 => match component(e, menu, &path[slash + 1..]) {
            Some((tile, None)) => tile,
            _ => 0,
        },
        _ => menu,
    }
}

/// A value's number.
fn value_num(e: &Engine, value: u32) -> f32 {
    e.mem.f32(value + VALUE_NUM)
}

/// `Tile::SetFloatValue(value->id, num, true)` on the value's tile.
fn set_float(e: &mut Engine, value: u32, num: f32) {
    let (tile, id) = (e.mem.u32(value + VALUE_TILE), e.mem.u32(value));
    e.call(SET_FLOAT_VALUE, &args![tile, id, num.to_bits(), 1u32]);
}

/// `Tile::SetStringValue(value->id, text, true)` on the value's tile.
fn set_string(e: &mut Engine, value: u32, text: &[u8]) {
    let (tile, id) = (e.mem.u32(value + VALUE_TILE), e.mem.u32(value));
    with_text(e, text, |e, at| {
        e.call(SET_STRING_VALUE, &args![tile, id, at, 1u32]);
    });
}

/// The text of a read's string argument `i`.
fn arg_text(args: &[Arg], i: usize) -> Vec<u8> {
    match args.get(i) {
        Some(Arg::Str { text, .. }) => text.clone(),
        _ => Vec::new(),
    }
}

/// What `GetSetUIValue_Execute` does.
#[derive(Clone, Copy, PartialEq)]
enum Action {
    GetFloat,
    SetFloat,
    SetString,
    SetFormattedString,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp: GetSetUIValue_Execute.
/// A trait at a component path read or set; without the trait, `GetUIFloat` answers -999, and the
/// console says it is not there.
fn get_set_ui_value(e: &mut Engine, w: &[u32], action: Action) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let (ok, component, num, text) = if action == Action::SetFormattedString {
        let f: Formatted = format_string_args_at(e, &a, 1, 22);
        (f.ok, arg_text(&f.leading, 0), 0.0, f.text)
    } else {
        let read = Read::of(e, &a);
        // The second argument is a string for SetUIString, a float for the others.
        let (num, text) = if action == Action::SetString {
            (0.0, read.text(1))
        } else {
            (read.float(1, 0.0), Vec::new())
        };
        (read.ok, read.text(0), num, text)
    };
    if !ok {
        return handled(true);
    }
    let value = menu_component_value(e, &component, false);
    if value != 0 {
        match action {
            Action::GetFloat => {
                let num = value_num(e, value);
                a.set_result(e, f64::from(num));
                if is_console_mode(e) {
                    console_print(e, "GetUIFloat >> %.4f", &double_words(f64::from(num)));
                }
            }
            Action::SetFloat => set_float(e, value, num),
            Action::SetString | Action::SetFormattedString => set_string(e, value, &text),
        }
    } else {
        if action == Action::GetFloat {
            a.set_result(e, NOT_FOUND);
        }
        if is_console_mode(e) {
            console_print(e, "Trait not found", &[]);
        }
    }
    handled(true)
}

fn get_ui_float(e: &mut Engine, w: &[u32]) -> Ret {
    get_set_ui_value(e, w, Action::GetFloat)
}

fn set_ui_float(e: &mut Engine, w: &[u32]) -> Ret {
    get_set_ui_value(e, w, Action::SetFloat)
}

fn set_ui_string(e: &mut Engine, w: &[u32]) -> Ret {
    get_set_ui_value(e, w, Action::SetString)
}

fn set_ui_string_ex(e: &mut Engine, w: &[u32]) -> Ret {
    get_set_ui_value(e, w, Action::SetFormattedString)
}

/// The `Alt` commands' and `GetUIFloatInherited`'s path buffer (`char component[0x200]`): a
/// path of this many bytes or more runs past it ([`super::arg_in_buffer`]).
const ALT_BUFFER: usize = 0x200;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp: Cmd_GetUIFloatAlt_Execute.
/// The trait's number ([`component_value_alt`]), -999 without it; the console prints it.
fn get_ui_float_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, NOT_FOUND);
    let read = Read::of(e, &a);
    if read.ok {
        let path = super::arg_in_buffer(e, &read, 0, ALT_BUFFER, "GetUIFloatAlt");
        let value = menu_component_value(e, &path, true);
        if value != 0 {
            let num = f64::from(value_num(e, value));
            a.set_result(e, num);
            if is_console_mode(e) {
                console_print(e, "GetUIFloatAlt >> %.4f", &double_words(num));
            }
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp:
// Cmd_SetUIFloatAlt_Execute and Cmd_ModUIFloat_Execute.
/// The trait set to the number (`add`: added to it), and 1; 0 without the trait.
fn set_or_mod_ui_float(e: &mut Engine, w: &[u32], add: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let command = if add { "ModUIFloat" } else { "SetUIFloatAlt" };
        let path = super::arg_in_buffer(e, &read, 0, ALT_BUFFER, command);
        let value = menu_component_value(e, &path, true);
        if value != 0 {
            let num = read.float(1, 0.0);
            let num = if add { value_num(e, value) + num } else { num };
            set_float(e, value, num);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

fn set_ui_float_alt(e: &mut Engine, w: &[u32]) -> Ret {
    set_or_mod_ui_float(e, w, false)
}

fn mod_ui_float(e: &mut Engine, w: &[u32]) -> Ret {
    set_or_mod_ui_float(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp:
// Cmd_SetUIStringAlt_Execute.
/// The trait set to the formatted string, and 1; 0 without the trait.
fn set_ui_string_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let f = format_string_args_at(e, &a, 1, 22);
    if f.ok {
        let path = super::args_in_buffer(e, &f.leading, 0, ALT_BUFFER, "SetUIStringAlt");
        let value = menu_component_value(e, &path, true);
        if value != 0 {
            set_string(e, value, &f.text);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp:
// Cmd_GetUIFloatInherited_Execute.
/// A trait as the tile shows it: for `x`, `y` and `depth`, the sum of the trait over the path's
/// tiles that have `locus` set (the menu's own left out with the second argument) and the last;
/// for `systemcolor`, the nearest tile's that has one; any other trait as `GetUIFloatAlt` reads
/// it. -999 when the path is not there.
fn get_ui_float_inherited(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, NOT_FOUND);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let component = super::arg_in_buffer(e, &read, 0, ALT_BUFFER, "GetUIFloatInherited");
    let exclude_menu_root = read.int(1, 0) != 0;
    let (menu, slash) = menu_by_path(e, &component);
    let Some(slash) = slash.filter(|_| menu != 0) else {
        return handled(true);
    };
    let mut sub = &component[slash + 1..];
    let trait_name = match sub.iter().rposition(|&b| b == b'/' || b == b'\\') {
        Some(at) => &sub[at + 1..],
        None => sub,
    };
    let is_pos = [&b"x"[..], b"y", b"depth"]
        .iter()
        .any(|n| trait_name.eq_ignore_ascii_case(n));
    let is_color = trait_name.eq_ignore_ascii_case(b"systemcolor");
    let print = |e: &mut Engine| {
        if is_console_mode(e) {
            let v = e.mem.f64(a.result);
            console_print(e, "GetUIFloatInherited >> %.4f", &double_words(v));
        }
    };
    if !is_pos && !is_color {
        let value = component_value_alt(e, menu, sub);
        if value != 0 {
            let num = f64::from(value_num(e, value));
            a.set_result(e, num);
        }
        print(e);
        return handled(true);
    }
    let mut current = menu;
    let mut cumulative: f32 = 0.0;
    while let Some(sep) = slash_pos(sub) {
        if !exclude_menu_root || current != menu {
            if is_pos {
                let locus = value_of(e, current, LOCUS);
                if locus != 0 && value_num(e, locus) != 0.0 {
                    let local = value_named(e, current, trait_name);
                    if local != 0 {
                        cumulative += value_num(e, local);
                    }
                }
            } else {
                let color = value_named(e, current, trait_name);
                if color != 0 {
                    cumulative = value_num(e, color);
                }
            }
        }
        let next = child_alt(e, current, &sub[..sep]);
        if next == 0 {
            return handled(true);
        }
        current = next;
        sub = &sub[sep + 1..];
    }
    if !sub.is_empty() {
        let last = value_named(e, current, sub);
        if last != 0 {
            if is_pos {
                cumulative += value_num(e, last);
            } else {
                cumulative = value_num(e, last);
            }
        }
        a.set_result(e, f64::from(cumulative));
    }
    print(e);
    handled(true)
}

/// A sort key's flags (`kReverseSort`, `kNormalizeItemNames`, `kAsFloat`).
const REVERSE: u32 = 1;
const NORMALIZE: u32 = 2;
const AS_FLOAT: u32 = 4;

/// A `SortUIListBox` specification: `key[,key...][$destination[$increment]]`, each key
/// `[-][#][@]path`.
struct SortSpec {
    keys: Vec<(Vec<u8>, u32)>,
    destination: Vec<u8>,
    increment: Vec<u8>,
}

/// xNVSE's `Tokenizer`: the tokens between the delimiters (runs of them skipped).
fn tokens(s: &[u8], delimiter: u8) -> impl Iterator<Item = &[u8]> {
    s.split(move |&b| b == delimiter).filter(|t| !t.is_empty())
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp: SortUIListBox_SortSpec.
fn parse_spec(spec: &[u8]) -> SortSpec {
    let mut out = SortSpec {
        keys: Vec::new(),
        destination: b"_y".to_vec(),
        increment: b"height".to_vec(),
    };
    let mut outer = tokens(spec, b'$');
    if let Some(keys) = outer.next() {
        for key in tokens(keys, b',') {
            let mut path = key;
            let mut flags = 0;
            if path.first() == Some(&b'-') {
                flags |= REVERSE;
                path = &path[1..];
            }
            if path.first() == Some(&b'#') {
                flags |= AS_FLOAT;
                path = &path[1..];
            }
            if path.first() == Some(&b'@') {
                flags |= NORMALIZE;
                path = &path[1..];
            }
            out.keys.push((path.to_vec(), flags));
        }
        if let Some(destination) = outer.next() {
            out.destination = destination.to_vec();
            if let Some(increment) = outer.next() {
                out.increment = increment.to_vec();
            }
        }
    }
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp: NormalizeItemName.
/// "9mm Pistol (5)" to "9mm Pistol", and a final "+" off ("9mm Pistol+").
fn normalize_item_name(s: &mut Vec<u8>) {
    let mut last = s.len() as isize - 1;
    if last > 0 && s[last as usize] == b')' {
        let mut digits = 0;
        loop {
            last -= 1;
            if last <= 0 {
                break;
            }
            if s[last as usize].is_ascii_digit() {
                digits += 1;
            } else {
                break;
            }
        }
        if digits > 0 && last >= 0 && s[last as usize] == b'(' {
            loop {
                last -= 1;
                if last <= 0 || s[last as usize] != b' ' {
                    break;
                }
            }
            s.truncate((last + 1).max(0) as usize);
        }
    }
    let last = s.len() as isize - 1;
    if last > 0 && s[last as usize] == b'+' {
        s.pop();
    }
}

/// A sort key's value: a number or a string.
#[derive(Clone)]
enum SortValue {
    Num(f32),
    Str(Vec<u8>),
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp:
// SortUIListBox_SortSpec::operator().
/// Whether item `l` sorts before item `r`: key by key, numbers by their difference, strings by
/// their bytes, a key reversed by its flag; a number and a string are never before each other.
fn sorts_before(keys: &[(Vec<u8>, u32)], l: &[SortValue], r: &[SortValue]) -> bool {
    for (i, (_, flags)) in keys.iter().enumerate() {
        let mut cmp: f32 = match (&l[i], &r[i]) {
            (SortValue::Num(a), SortValue::Num(b)) => a - b,
            (SortValue::Str(a), SortValue::Str(b)) => match a.cmp(b) {
                Ordering::Less => -1.0,
                Ordering::Equal => 0.0,
                Ordering::Greater => 1.0,
            },
            _ => return false,
        };
        if flags & REVERSE != 0 {
            cmp = -cmp;
        }
        if cmp < 0.0 {
            return true;
        } else if cmp > 0.0 {
            return false;
        }
    }
    false
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetComponent, as
// SortUIListBox calls it on its spec's own strings.
/// A spec path's value on `tile`. xNVSE's lookup writes a 0 over each slash it walks past, in the
/// spec's string itself, so after the first lookup the path ends at its first slash; the fixes
/// row ([`super::array::UI_SORT_PATHS`], `whole`) keeps the whole path for every item.
fn spec_lookup(e: &mut Engine, tile: u32, path: &mut Vec<u8>, whole: bool) -> u32 {
    let value = component_value(e, tile, path);
    if !whole {
        if let Some(at) = slash_pos(path) {
            path.truncate(at);
        }
    }
    value
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp: Cmd_SortUIListBox_Execute.
/// The visible items of a list box (`listindex` 0 or more, every key there) put in order: each
/// item's destination trait (`_y`) set to the running sum of the items' increment traits
/// (`height`) before it.
fn sort_ui_list_box(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut spec = parse_spec(&read.text(1));
    let whole = super::array::fix_on(e, super::array::UI_SORT_PATHS);
    let parent = menu_component_tile(e, &read.text(0));
    if parent == 0 {
        return handled(true);
    }
    let mut items: Vec<(u32, Vec<SortValue>)> = Vec::new();
    let mut node = parent + CHILDREN;
    for _ in 0..MAX_CHILDREN {
        let (data, next) = (e.mem.u32(node), e.mem.u32(node + 4));
        if data == 0 && next == 0 {
            break;
        }
        let tile = if data == 0 { 0 } else { e.mem.u32(data + 8) };
        if tile != 0 {
            let index = component_value(e, tile, b"listindex");
            // Hidden: no `listindex`, or one under 0 (a NaN is not).
            let shown = index != 0 && {
                let num = value_num(e, index);
                num >= 0.0 || num.is_nan()
            };
            if shown {
                let mut values = Vec::new();
                let mut skip = false;
                for (path, flags) in spec.keys.iter_mut() {
                    let flags = *flags;
                    let value = spec_lookup(e, tile, path, whole);
                    if value == 0 {
                        skip = true;
                        break;
                    }
                    let text_at = e.mem.u32(value + VALUE_STR);
                    if text_at != 0 {
                        let mut text = guest_text(e, text_at);
                        if flags & AS_FLOAT != 0 {
                            values.push(SortValue::Num(crate::cnum::strtod(&text).0 as f32));
                        } else {
                            if flags & NORMALIZE != 0 {
                                normalize_item_name(&mut text);
                            }
                            values.push(SortValue::Str(text));
                        }
                    } else {
                        values.push(SortValue::Num(value_num(e, value)));
                    }
                }
                if !skip {
                    items.push((tile, values));
                }
            }
        }
        if next == 0 {
            break;
        }
        node = next;
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    msvc_sort(&mut order, &|&l: &usize, &r: &usize| {
        sorts_before(&spec.keys, &items[l].1, &items[r].1)
    });
    let mut current: f32 = 0.0;
    for &i in &order {
        let tile = items[i].0;
        let increment = spec_lookup(e, tile, &mut spec.increment, whole);
        if increment == 0 {
            continue;
        }
        let step = value_num(e, increment);
        let destination = spec_lookup(e, tile, &mut spec.destination, whole);
        if destination == 0 {
            continue;
        }
        set_float(e, destination, current);
        current += step;
    }
    handled(true)
}

/// `_ISORT_MAX` (MSVC's `<algorithm>`): ranges this short are insertion sorted.
const ISORT_MAX: usize = 32;

// Translated from MSVC's STL `<algorithm>` (`std::sort`: `_Sort_unchecked` and what it calls), the
// sort xNVSE's `SortUIListBox` is compiled with; `less` is the predicate.
/// `std::sort` as MSVC's library sorts: introsort, insertion sort below 33 elements, heap sort
/// when the partitions go bad; elements that compare equal end where it puts them.
fn msvc_sort<T: Copy>(v: &mut [T], less: &dyn Fn(&T, &T) -> bool) {
    let len = v.len();
    sort_unchecked(v, 0, len, len as isize, less);
}

fn sort_unchecked<T: Copy>(
    v: &mut [T],
    mut first: usize,
    mut last: usize,
    mut ideal: isize,
    less: &dyn Fn(&T, &T) -> bool,
) {
    loop {
        if last - first <= ISORT_MAX {
            insertion_sort(v, first, last, less);
            return;
        }
        if ideal <= 0 {
            make_heap(v, first, last, less);
            sort_heap(v, first, last, less);
            return;
        }
        let (mid_first, mid_last) = partition_by_median_guess(v, first, last, less);
        ideal = (ideal >> 1) + (ideal >> 2);
        if mid_first - first < last - mid_last {
            sort_unchecked(v, first, mid_first, ideal, less);
            first = mid_last;
        } else {
            sort_unchecked(v, mid_last, last, ideal, less);
            last = mid_first;
        }
    }
}

fn insertion_sort<T: Copy>(v: &mut [T], first: usize, last: usize, less: &dyn Fn(&T, &T) -> bool) {
    if first == last {
        return;
    }
    let mut mid = first + 1;
    while mid != last {
        let val = v[mid];
        if less(&val, &v[first]) {
            v.copy_within(first..mid, first + 1);
            v[first] = val;
        } else {
            let mut hole = mid;
            while less(&val, &v[hole - 1]) {
                v[hole] = v[hole - 1];
                hole -= 1;
            }
            v[hole] = val;
        }
        mid += 1;
    }
}

fn med3<T: Copy>(
    v: &mut [T],
    first: usize,
    mid: usize,
    last: usize,
    less: &dyn Fn(&T, &T) -> bool,
) {
    if less(&v[mid], &v[first]) {
        v.swap(mid, first);
    }
    if less(&v[last], &v[mid]) {
        v.swap(last, mid);
        if less(&v[mid], &v[first]) {
            v.swap(mid, first);
        }
    }
}

fn guess_median<T: Copy>(
    v: &mut [T],
    first: usize,
    mid: usize,
    last: usize,
    less: &dyn Fn(&T, &T) -> bool,
) {
    let count = last - first;
    if count > 40 {
        let step = (count + 1) >> 3;
        let two_step = step << 1;
        med3(v, first, first + step, first + two_step, less);
        med3(v, mid - step, mid, mid + step, less);
        med3(v, last - two_step, last - step, last, less);
        med3(v, first + step, mid, last - step, less);
    } else {
        med3(v, first, mid, last, less);
    }
}

fn partition_by_median_guess<T: Copy>(
    v: &mut [T],
    first: usize,
    last: usize,
    less: &dyn Fn(&T, &T) -> bool,
) -> (usize, usize) {
    let mid = first + ((last - first) >> 1);
    guess_median(v, first, mid, last - 1, less);
    let mut pfirst = mid;
    let mut plast = pfirst + 1;
    while first < pfirst && !less(&v[pfirst - 1], &v[pfirst]) && !less(&v[pfirst], &v[pfirst - 1]) {
        pfirst -= 1;
    }
    while plast < last && !less(&v[plast], &v[pfirst]) && !less(&v[pfirst], &v[plast]) {
        plast += 1;
    }
    let mut gfirst = plast;
    let mut glast = pfirst;
    loop {
        while gfirst < last {
            if less(&v[pfirst], &v[gfirst]) {
                gfirst += 1;
                continue;
            } else if less(&v[gfirst], &v[pfirst]) {
                break;
            } else if plast != gfirst {
                v.swap(plast, gfirst);
                plast += 1;
            } else {
                plast += 1;
            }
            gfirst += 1;
        }
        while first < glast {
            let prev = glast - 1;
            if less(&v[prev], &v[pfirst]) {
                glast -= 1;
                continue;
            } else if less(&v[pfirst], &v[prev]) {
                break;
            } else {
                pfirst -= 1;
                if pfirst != prev {
                    v.swap(pfirst, prev);
                }
            }
            glast -= 1;
        }
        if glast == first && gfirst == last {
            return (pfirst, plast);
        }
        if glast == first {
            if plast != gfirst {
                v.swap(pfirst, plast);
            }
            plast += 1;
            v.swap(pfirst, gfirst);
            pfirst += 1;
            gfirst += 1;
        } else if gfirst == last {
            glast -= 1;
            pfirst -= 1;
            if glast != pfirst {
                v.swap(glast, pfirst);
            }
            plast -= 1;
            v.swap(pfirst, plast);
        } else {
            glast -= 1;
            v.swap(gfirst, glast);
            gfirst += 1;
        }
    }
}

/// `_Pop_heap_hole_by_index` and `_Push_heap_by_index` (MSVC): the value sifted into the heap of
/// `bottom` elements from `hole`.
fn pop_heap_hole<T: Copy>(
    v: &mut [T],
    base: usize,
    mut hole: usize,
    bottom: usize,
    val: T,
    less: &dyn Fn(&T, &T) -> bool,
) {
    let top = hole;
    let mut idx = hole;
    let max_sequence_non_leaf = (bottom - 1) >> 1;
    while idx < max_sequence_non_leaf {
        idx = 2 * idx + 2;
        if less(&v[base + idx], &v[base + idx - 1]) {
            idx -= 1;
        }
        v[base + hole] = v[base + idx];
        hole = idx;
    }
    if idx == max_sequence_non_leaf && bottom % 2 == 0 {
        v[base + hole] = v[base + bottom - 1];
        hole = bottom - 1;
    }
    // `_Push_heap_by_index`.
    let mut idx = if hole > 0 { (hole - 1) >> 1 } else { 0 };
    while top < hole && less(&v[base + idx], &val) {
        v[base + hole] = v[base + idx];
        hole = idx;
        idx = if hole > 0 { (hole - 1) >> 1 } else { 0 };
    }
    v[base + hole] = val;
}

fn make_heap<T: Copy>(v: &mut [T], first: usize, last: usize, less: &dyn Fn(&T, &T) -> bool) {
    let bottom = last - first;
    let mut hole = bottom >> 1;
    while hole > 0 {
        hole -= 1;
        let val = v[first + hole];
        pop_heap_hole(v, first, hole, bottom, val, less);
    }
}

fn sort_heap<T: Copy>(v: &mut [T], first: usize, mut last: usize, less: &dyn Fn(&T, &T) -> bool) {
    while last - first >= 2 {
        last -= 1;
        let val = v[last];
        v[last] = v[first];
        pop_heap_hole(v, first, 0, last - first, val, less);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp: Cmd_EndVATScam_Execute.
/// Every VATS target taken back, then VATS accepted when it is choosing limbs or zooming.
fn end_vats_cam(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    for _ in 0..MAX_CHILDREN {
        if e.mem.u32(VATS_CAMERA) == 0 {
            break;
        }
        e.with_stack(8, |e, out| {
            e.call(
                VATS_UNCLICK,
                &args![VATS_CAMERA, out.addr(), out.addr() + 4],
            );
        });
    }
    if e.mem.u32(VATS_CAMERA + VATS_MODE) == LIMB_SELECT_OR_ZOOM {
        e.call(VATS_ACCEPT, &args![]);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp:
// Cmd_ShowLevelUpMenu_Execute.
/// The level-up menu shown (`DoShowLevelUpMenu`).
fn show_level_up_menu(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    e.call(SHOW_LEVEL_UP_MENU, &args![]);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTiles.cpp: Tile::GetQualifiedName.
/// The tile's name after its parents', each followed by a backslash.
fn qualified_name(e: &Engine, tile: u32) -> Vec<u8> {
    let mut chain = Vec::new();
    let mut at = tile;
    for _ in 0..MAX_CHILDREN {
        if at == 0 {
            break;
        }
        chain.push(at);
        at = e.mem.u32(at + PARENT);
    }
    let mut out = Vec::new();
    for (i, &t) in chain.iter().rev().enumerate() {
        if i > 0 {
            out.push(b'\\');
        }
        out.extend(tile_name(e, t));
    }
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_UI.cpp:
// Cmd_PrintActiveTile_Execute.
/// The highlighted tile's full name to the console (a line saying so when it is too long, or
/// when there is none).
fn print_active_tile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let manager = e.global::<u32>(INTERFACE_MANAGER);
    let tile = if manager == 0 {
        0
    } else {
        e.mem.u32(manager + ACTIVE_TILE)
    };
    if tile != 0 {
        let name = qualified_name(e, tile);
        if name.len() < 0x200 {
            super::console::print_text(e, &name);
        } else {
            super::console::print_text(
                e,
                b"Name too long to print in console, sending to nvse.log",
            );
        }
    } else {
        super::console::print_text(e, b"Could not read active tile");
    }
    handled(true)
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
