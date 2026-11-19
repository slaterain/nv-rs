//! `functions_jip/jip_fn_ccc.h` (JIP LN NVSE), the CC&C commands that need `CCCOnLoad`'s start:
//! `CCCOnLoad` (the mod's version check, its avatar pictures from `Data\Config\ccc_avatars.ini`
//! and the `Data\textures\jazzisparis\ccc` folder, the saved forms of the companion mods it
//! works with, and the 119 tile values of the HUD component the mod injects), `CCCSetFloat`,
//! `CCCSetString` and `CCCSetTrait` (those values written), `CCCSavedForm` (a saved form) and
//! `CCCSMS` (the mod's effect shaders on a reference stopped). The rest of the file is
//! `jip_fn_ccc.rs`.
//!
//! The ini file and the folder are read through the mods' files seam
//! ([`engine_core::mod_files`]): the port's folder first, then the game's Data, as a mod reads
//! them; nothing is written.
//!
//! nv-rs's own, where JIP reads through a null pointer or past an array (a crash in the game):
//! a tile missing on the way through the HUD component gives the component's load error (as a
//! missing value does in JIP); an index past the 119 values or the 10 saved forms does nothing.

use super::jip_fn_ui::{component, trait_id, value_of, value_set_bool, value_set_float};
use super::Handlers;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::collections::HashMap;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26fd, ccc_on_load),
    (0x26fe, ccc_set_float),
    (0x26ff, ccc_set_string),
    (0x2700, ccc_set_trait),
    (0x2708, ccc_saved_form),
    (0x2771, ccc_sms),
];

/// The values `CCCOnLoad` finds (`s_UIelements`, 0x77 of them).
pub const UI_ELEMENTS: usize = 119;

/// What `CCCOnLoad` keeps (JIP's statics in `jip_fn_ccc.h`, never saved or cleared): the mod's
/// index (`s_CCCModIdx`, 0 before its first call), the avatar pictures by name (`s_avatarPaths`:
/// JIP's `const char*` keys compare by `StrHashCI` alone) and the common ones (`s_avatarCommon`:
/// a `Map` sorted by `StrCompareCI`, searched in order), the HUD values and whether all were found
/// (`s_UIelements`, `s_UILoaded`), the saved forms' ids (`s_savedForms`), the task faction
/// (`s_taskFaction`), and the picture chosen for each actor base (`s_pathForID`).
#[derive(Default)]
pub struct Hud {
    pub mod_index: u8,
    pub avatar_paths: HashMap<u32, Vec<u8>>,
    pub avatar_common: Vec<(Vec<u8>, Vec<u8>)>,
    pub elements: Vec<u32>,
    pub loaded: bool,
    pub saved_forms: [u32; 10],
    pub task_faction: u32,
    pub path_for_id: HashMap<u32, Vec<u8>>,
}

fn hud(e: &mut Engine) -> &mut Hud {
    &mut e.ext::<Jip>().ccc.hud
}

/// A form's mod index (`modIndex`: the id's high byte, +0xF); a reference's base form (+0x20); a
/// global's value (+0x24); a `TESPackage`'s location (+0x2C, its radius +4); `TESActorBase`:
/// `baseData.flags` (+0x34, bit 0 female), `fullName` (its text +0xD4); the creature form type.
const MOD_INDEX: u32 = 0xf;
const REF_BASE: u32 = 0x20;
const GLOBAL_VALUE: u32 = 0x24;
const PACKAGE_LOCATION: u32 = 0x2c;
const LOCATION_RADIUS: u32 = 4;
const BASE_FLAGS: u32 = 0x34;
const BASE_NAME: u32 = 0xd4;
const TES_CREATURE: u8 = 0x2b;
/// `g_HUDMainMenu` (`*011D96C0`) and a menu's tile (+4); a tile's children (`DList`: tail +8; a
/// node's previous +4, its tile +8) and parent (+0x28); a value's number (+8).
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
const MENU_TILE: u32 = 0x04;
const CHILDREN_TAIL: u32 = 0x08;
const NODE_PREV: u32 = 0x04;
const NODE_DATA: u32 = 0x08;
const TILE_PARENT: u32 = 0x28;
const VALUE_NUM: u32 = 0x08;
/// `kTileValue_x`, `_visible`, `_string`, `_filename`.
const TRAIT_X: u32 = 0xfa1;
const TRAIT_VISIBLE: u32 = 0xfa3;
const TRAIT_STRING: u32 = 0xfc4;
const TRAIT_FILENAME: u32 = 0xfcc;
/// The game's `ShowMessageBox` (`00703E80`, cdecl) and its own answer callback (`005B4A70`).
const SHOW_MESSAGE_BOX: u32 = 0x0070_3e80;
const MESSAGE_BOX_CALLBACK: u32 = 0x005b_4a70;
/// `g_thePlayer`; an actor's process (+0x68): its level (+0x28), `currentPackage` (+4: the
/// package, its target +8 after it), a middle-high process's `interruptPackage` (+0xE4) and
/// knocked state (+0x13C); an actor's actor values (+0xA4: `GetActorValue` +0xC,
/// `GetBaseActorValue` +4); a package's type (+0x20); `TESPackage::GetFleeCombat` (`00981990`).
const PLAYER: u32 = 0x011d_ea3c;
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const CURRENT_PACKAGE: u32 = 0x04;
const INTERRUPT_PACKAGE: u32 = 0xe4;
const INFO_TARGET: u32 = 0x08;
const KNOCKED_STATE: u32 = 0x13c;
const AV_OWNER: u32 = 0xa4;
const GET_ACTOR_VALUE: u32 = 0x0c;
const GET_BASE_ACTOR_VALUE: u32 = 0x04;
const AV_HEALTH: u32 = 0x10;
const PACKAGE_TYPE: u32 = 0x20;
const GET_FLEE_COMBAT: u32 = 0x0098_1990;
/// `ProcessManager` (`011E0E80`): `tempEffects` (an inline `tList` at +0x60); a
/// `MagicShaderHitEffect` (vtable `0107B70C`): its target (+0x1C), its flags (byte +0x24, 1 stop)
/// and its effect shader (+0x30).
const PROCESS_MANAGER: u32 = 0x011e_0e80;
const TEMP_EFFECTS: u32 = 0x60;
const MAGIC_SHADER_HIT_EFFECT_VTBL: u32 = 0x0107_b70c;
const EFFECT_TARGET: u32 = 0x1c;
const EFFECT_FLAGS: u32 = 0x24;
const EFFECT_SHADER: u32 = 0x30;
/// `kCommandInfo_CCCSetString.numParams` (the trait and xNVSE's 21 format parameters); JIP's
/// string buffer (`GetStrArgBuffer`).
const SET_STRING_PARAMS: u32 = 22;
const STR_ARG_BUFFER: usize = 0x4_0000;

const LOAD_ERROR: &[u8] = b"ERROR: Failed to load JIP CC&C UI components.";
const OLD_VERSION: &[u8] = b"Please be advised:\n\nJIP CC&C versions older than 2.60 will NOT work with JIP LN NVSE.\n\nYou MUST update JIP CC&C to the latest version.";
const AVATAR_MALE: &[u8] = b"jazzisparis\\ccc\\avatar_male.dds";
const AVATAR_FEMALE: &[u8] = b"jazzisparis\\ccc\\avatar_female.dds";
const AVATARS_INI: &[u8] = b"Data\\Config\\ccc_avatars.ini";
const AVATARS_FOLDER: &[u8] = b"Data\\textures\\jazzisparis\\ccc\\";

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: LineIterator::LineIterator
// and operator++.
/// The file's lines as JIP's `LineIterator` gives them: up to the file's first 0 byte, carriage
/// returns and line feeds end a line (JIP's `ReplaceChr` stops at that 0); after it only 0 bytes
/// do, so a CR or LF there stays inside its piece. Empty pieces are skipped, and a piece that
/// starts with byte 3 (JIP's end mark) ends the walk.
fn lines(text: &[u8]) -> Vec<Vec<u8>> {
    let first_zero = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    let mut bytes = text.to_vec();
    for b in &mut bytes[..first_zero] {
        if *b == b'\r' || *b == b'\n' {
            *b = 0;
        }
    }
    let mut out = Vec::new();
    for line in bytes.split(|&b| b == 0) {
        if line.is_empty() {
            continue;
        }
        if line[0] == 3 {
            break;
        }
        out.push(line.to_vec());
    }
    out
}

/// `s_avatarCommon()[key] = path`: the key kept in `StrCompareCI` order, a key already there
/// (without case) given the new path.
fn insert_common(common: &mut Vec<(Vec<u8>, Vec<u8>)>, key: &[u8], path: Vec<u8>) {
    match common.binary_search_by(|(k, _)| crate::utility::str_compare_ci(k, key).0.cmp(&0)) {
        Ok(at) => common[at].1 = path,
        Err(at) => common.insert(at, (key.to_vec(), path)),
    }
}

/// The avatar pictures: each line `name=picture` of the ini file (`jazzisparis\ccc\avatar_` and
/// the picture and `.dds`; the first 129 lines by name, the rest common), then each
/// `avatar_*.dds` file of the folder by the name between `avatar_` and `.dds`.
fn read_avatars(e: &mut Engine) {
    let text = engine_core::mod_files::read(e, AVATARS_INI).unwrap_or_default();
    for (index, line) in lines(&text).into_iter().enumerate() {
        let (key, value) = super::jip_fn_ui::next_token(&line, b'=');
        let mut path = b"jazzisparis\\ccc\\avatar_".to_vec();
        path.extend_from_slice(value);
        path.extend_from_slice(b".dds");
        let h = hud(e);
        if index < 129 {
            h.avatar_paths
                .insert(crate::containers::str_hash_ci(key), path);
        } else {
            insert_common(&mut h.avatar_common, key, path);
        }
    }
    for entry in engine_core::mod_files::list(e, AVATARS_FOLDER) {
        let name = &entry.name;
        let lower = name.to_ascii_lowercase();
        if entry.is_folder || !lower.starts_with(b"avatar_") || !lower.ends_with(b".dds") {
            continue;
        }
        let mut path = b"jazzisparis\\ccc\\".to_vec();
        path.extend_from_slice(name);
        let key = &name[7..name.len() - 4];
        hud(e)
            .avatar_paths
            .insert(crate::containers::str_hash_ci(key), path);
    }
}

/// The saved forms of the companion mods CC&C works with, when loaded, and Companion Core's
/// follow package's location radius set to 0.
fn read_saved_forms(e: &mut Engine) {
    let index = super::jip_fn_utility::mod_index(e, b"Companion Core.esm");
    if index < 0xff {
        let base = index << 24;
        hud(e).saved_forms[..4].copy_from_slice(&[
            base | 0x812,
            base | 0x810,
            base | 0x950d,
            base | 0x5b3b4,
        ]);
        let package = crate::cosave::lookup_form(e, base | 0x80e);
        if package != 0 {
            let location = e.mem.u32(package + PACKAGE_LOCATION);
            if location != 0 {
                e.mem.set_u32(location + LOCATION_RADIUS, 0);
            }
        }
    }
    let index = super::jip_fn_utility::mod_index(e, b"NosCo Companion System.esm");
    if index < 0xff {
        let base = index << 24;
        hud(e).saved_forms[4..9].copy_from_slice(&[
            base | 0xae0,
            base | 0xae2,
            base | 0x86a7,
            base | 0xae5,
            base | 0xadd,
        ]);
    }
    let index = super::jip_fn_utility::mod_index(e, b"EZ_CompanionNVSE.esm");
    if index < 0xff {
        hud(e).saved_forms[9] = (index << 24) | 0xdc3;
    }
}

fn tail(e: &Engine, tile: u32) -> Option<u32> {
    match e.mem.u32(tile + CHILDREN_TAIL) {
        0 => None,
        node => Some(node),
    }
}

fn prev(e: &Engine, node: u32) -> Option<u32> {
    match e.mem.u32(node + NODE_PREV) {
        0 => None,
        node => Some(node),
    }
}

fn data(e: &Engine, node: u32) -> Option<u32> {
    match e.mem.u32(node + NODE_DATA) {
        0 => None,
        tile => Some(tile),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetNthChild.
/// The `n`-th child node counted from the last child backwards.
fn nth_child(e: &Engine, tile: u32, n: u32) -> Option<u32> {
    let mut node = tail(e, tile)?;
    for _ in 0..n {
        node = prev(e, node)?;
    }
    Some(node)
}

/// `Tile::GetValueName`: the tile's value of the trait named `name`.
fn value_name(e: &mut Engine, tile: u32, name: &[u8]) -> u32 {
    let id = trait_id(e, name);
    value_of(e, tile, id)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetComponentValue.
/// The value at `path` below `tile`: the path must end at a trait of the tile it reaches.
fn component_value(e: &mut Engine, tile: u32, path: &[u8]) -> u32 {
    match component(e, tile, path) {
        Some((at, Some(name))) => value_name(e, at, name),
        _ => 0,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h: Cmd_CCCOnLoad_Execute
// (its walk of the HUD component).
/// The 119 values of the `JIPCCCHUDDisplay` component, as JIP walks the mod's XML (children
/// counted from the last). `None` when a tile on the way is missing.
fn find_elements(e: &mut Engine, display: u32) -> Option<Vec<u32>> {
    let mut el = vec![0u32; UI_ELEMENTS];
    let ids: Vec<u32> = [
        &b"_Size"[..],
        b"_Anchor",
        b"_Selected",
        b"_HealthFill",
        b"_Distance",
        b"_Action",
        b"_CmdType",
        b"_Value",
    ]
    .iter()
    .map(|n| trait_id(e, n))
    .collect();
    let (t06, t07, t08, t09, t10, t11, t17, t20) = (
        ids[0], ids[1], ids[2], ids[3], ids[4], ids[5], ids[6], ids[7],
    );
    let mut node0 = tail(e, display)?;
    el[0] = value_name(e, display, b"_AltStyle");
    el[1] = value_name(e, display, b"_SelectMode");
    el[2] = value_name(e, display, b"_Adjust");
    let tile = data(e, node0)?;
    el[3] = value_name(e, tile, b"_TotalAvatars");
    el[4] = value_name(e, tile, b"_NumHired");
    el[5] = value_name(e, tile, b"_Layout");
    el[6] = value_of(e, tile, t06);
    el[7] = value_of(e, tile, t07);
    el[8] = value_of(e, tile, t08);
    el[9] = value_of(e, tile, TRAIT_VISIBLE);
    let mut node1 = tail(e, tile);
    for i in 0..10 {
        let at = node1?;
        let tile = data(e, at)?;
        node1 = prev(e, at);
        el[10 + i] = value_of(e, tile, t09);
        el[20 + i] = value_of(e, tile, t10);
        el[30 + i] = value_of(e, tile, t11);
        let picture = data(e, nth_child(e, tile, 2)?)?;
        el[40 + i] = value_of(e, picture, TRAIT_FILENAME);
        let name = data(e, nth_child(e, tile, 4)?)?;
        el[50 + i] = value_of(e, name, TRAIT_STRING);
    }
    node0 = prev(e, node0)?;
    let tile = data(e, node0)?;
    el[60] = value_of(e, tile, t06);
    el[61] = value_of(e, tile, t08);
    el[62] = value_of(e, tile, t07);
    el[63] = value_name(e, tile, b"_IsWaiting");
    el[64] = value_name(e, tile, b"_IsFollowingLong");
    el[65] = value_name(e, tile, b"_IsUsingMelee");
    el[66] = value_name(e, tile, b"_IsAggressive");
    el[67] = value_name(e, tile, b"_LabelsON");
    el[68] = value_of(e, tile, TRAIT_X);
    let mut node1 = nth_child(e, tile, 3)?;
    for slot in el.iter_mut().take(74).skip(69) {
        *slot = value_of(e, data(e, node1)?, t17);
        node1 = prev(e, node1)?;
    }
    el[74] = value_of(e, data(e, node1)?, TRAIT_STRING);
    node0 = prev(e, node0)?;
    let tile = data(e, node0)?;
    el[75] = value_name(e, tile, b"_Menu");
    el[76] = value_of(e, tile, t08);
    el[77] = value_name(e, tile, b"_HasHome");
    let node1 = nth_child(e, tile, 3)?;
    let mut node2 = tail(e, data(e, node1)?)?;
    for slot in el.iter_mut().take(81).skip(78) {
        *slot = value_of(e, data(e, node2)?, t20);
        node2 = prev(e, node2)?;
    }
    node2 = prev(e, prev(e, prev(e, node2)?)?)?;
    el[81] = value_of(e, data(e, node2)?, TRAIT_STRING);
    let node1 = prev(e, prev(e, node1)?)?;
    let mut node2 = tail(e, data(e, node1)?)?;
    el[82] = value_of(e, data(e, node2)?, TRAIT_STRING);
    node2 = prev(e, node2)?;
    el[83] = value_of(e, data(e, node2)?, TRAIT_STRING);
    node2 = prev(e, node2)?;
    for slot in el.iter_mut().take(110).skip(84) {
        *slot = value_of(e, data(e, node2)?, t20);
        node2 = prev(e, node2)?;
    }
    el[110] = value_of(e, data(e, tail(e, data(e, node2)?)?)?, TRAIT_STRING);
    node0 = prev(e, node0)?;
    let tile = data(e, node0)?;
    el[111] = value_name(e, tile, b"_Targeting");
    el[112] = value_of(e, tile, TRAIT_STRING);
    node0 = prev(e, node0)?;
    el[113] = value_of(e, data(e, tail(e, data(e, node0)?)?)?, TRAIT_STRING);
    let parent = e.mem.u32(tile + TILE_PARENT);
    let hud_tile = if parent == 0 {
        return None;
    } else {
        e.mem.u32(parent + TILE_PARENT)
    };
    if hud_tile == 0 {
        return None;
    }
    el[114] = component_value(e, hud_tile, b"SneakMeter/y");
    let duif3 = value_name(e, hud_tile, b"_DUIF3");
    if duif3 == 0 {
        return None;
    }
    if e.mem.f32(duif3 + VALUE_NUM) != 0.0 {
        el[115] = component_value(e, hud_tile, b"QuestWrap/QuestReminder/x");
        el[116] = component_value(e, hud_tile, b"XPWrap/x");
        el[118] = component_value(e, hud_tile, b"EH_Wrap/EnemyHealth/y");
    } else {
        el[115] = component_value(e, hud_tile, b"QuestReminder/x");
        el[116] = component_value(e, hud_tile, b"XPMeter/x");
        el[117] = component_value(e, hud_tile, b"Info/y");
        el[118] = component_value(e, hud_tile, b"EnemyHealth/y");
    }
    if el[117] == 0 {
        el[117] = component_value(e, hud_tile, b"InfoWrap/Info/y");
    }
    Some(el)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h: Cmd_CCCOnLoad_Execute.
/// `CCCOnLoad` (once a session: nothing after its first call): the calling script's file is
/// CC&C's. Its global `0x5000` must be 260 or more (CC&C 2.60): otherwise its global `0x8F6D` is
/// set to 1, a message box says to update, and the command fails. Then the avatar pictures, the
/// task faction (`0x2D501`), the saved forms, and the HUD component's values; a component that is
/// not there, or a value missing, prints the load error in the console and leaves the values
/// unused. No result.
fn ccc_on_load(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if hud(e).mod_index != 0 {
        return handled(true);
    }
    let index = e.mem.u8(a.script + MOD_INDEX);
    hud(e).mod_index = index;
    let base = u32::from(index) << 24;
    let version = crate::cosave::lookup_form(e, base | 0x5000);
    if version == 0 || e.mem.f32(version + GLOBAL_VALUE) < 260.0 {
        let flag = crate::cosave::lookup_form(e, base | 0x8f6d);
        if flag != 0 {
            e.mem.set_f32(flag + GLOBAL_VALUE, 1.0);
        }
        let message = nvse::table::guest_str(e, OLD_VERSION);
        let ok = nvse::table::guest_str(e, b"OK");
        // `ShowMessageBox` is variadic (its translation reads the words it is given), as
        // `jip_fn_ui`'s call.
        let words = [message, 0, 0, MESSAGE_BOX_CALLBACK, 0, 0x17, 0, 0, ok, 0];
        e.call(SHOW_MESSAGE_BOX, &words);
        return handled(false);
    }
    read_avatars(e);
    let faction = crate::cosave::lookup_form(e, base + 0x2d501);
    hud(e).task_faction = faction;
    read_saved_forms(e);
    let menu = e.mem.u32(HUD_MAIN_MENU);
    let root = if menu == 0 {
        0
    } else {
        e.mem.u32(menu + MENU_TILE)
    };
    // JIP reads the HUD's tile with no check (a crash without one); nv-rs's own: the load error.
    if root == 0 {
        crate::console::console_print_line(e, LOAD_ERROR);
        return handled(true);
    }
    let display = match component(e, root, b"JIPCCCHUDDisplay") {
        Some((tile, None)) => tile,
        _ => {
            crate::console::console_print_line(e, LOAD_ERROR);
            return handled(true);
        }
    };
    match find_elements(e, display) {
        Some(el) if el.iter().all(|&v| v != 0) => {
            let h = hud(e);
            h.elements = el;
            h.loaded = true;
        }
        _ => crate::console::console_print_line(e, LOAD_ERROR),
    }
    handled(true)
}

/// HUD value `index`, when `CCCOnLoad` found them all and the index is one of them.
fn element(e: &mut Engine, index: i64) -> Option<u32> {
    let h = hud(e);
    if !h.loaded || !(0..UI_ELEMENTS as i64).contains(&index) {
        return None;
    }
    Some(h.elements[index as usize])
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSetFloat_Execute.
/// `CCCSetFloat index value`: HUD value `index`'s number (`Tile::Value::SetFloat`).
fn ccc_set_float(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !hud(e).loaded {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(value) = element(e, i64::from(read.int(0, 0))) {
            value_set_float(e, value, read.float(1, 0.0));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSetString_Execute.
/// `CCCSetString index "text"`: HUD value `index`'s text (`Tile::Value::SetString`).
fn ccc_set_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !hud(e).loaded {
        return handled(true);
    }
    let f = nvse::commands::string::format_string_args_at(e, &a, 1, SET_STRING_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let index = f
        .leading
        .first()
        .and_then(nvse::args::Arg::int)
        .unwrap_or(0) as u32;
    let text = super::formatted_in_buffer(e, &f.text, STR_ARG_BUFFER, "CCCSetString");
    if let Some(value) = element(e, i64::from(index)) {
        crate::game_patches::gb::value_set_string(e, value, &text);
    }
    handled(true)
}

/// The actor's running package information (`Actor::GetPackageInfo`): a middle or high process's
/// interrupt package when it has one, else the current package; its address, `None` for none.
fn package_info(e: &Engine, actor: u32) -> Option<u32> {
    let process = e.mem.u32(actor + PROCESS);
    if process == 0 {
        return None;
    }
    if e.mem.u8(process + PROCESS_LEVEL) <= 1 && e.mem.u32(process + INTERRUPT_PACKAGE) != 0 {
        return Some(process + INTERRUPT_PACKAGE);
    }
    (e.mem.u32(process + CURRENT_PACKAGE) != 0).then_some(process + CURRENT_PACKAGE)
}

/// `Actor::GetKnockedState`: a middle or high process's knocked state, -1 otherwise.
fn knocked_state(e: &Engine, actor: u32) -> i8 {
    let process = e.mem.u32(actor + PROCESS);
    if process != 0 && e.mem.u8(process + PROCESS_LEVEL) <= 1 {
        e.mem.u8(process + KNOCKED_STATE) as i8
    } else {
        -1
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetDistance.
/// The 3D distance between the two, the largest float when they are not in the same cell or
/// worldspace.
fn distance(e: &mut Engine, from: u32, to: u32) -> f32 {
    if !super::ln_fn_math::in_same_cell_or_world(e, from, to) {
        return f32::MAX;
    }
    super::ln_fn_math::point3_distance(
        super::ln_fn_math::position(e, from),
        super::ln_fn_math::position(e, to),
    )
}

/// The avatar picture of the actor base `base` named `name` (`s_pathForID`, chosen once a base):
/// its picture by name; for a creature without one, the first common picture whose name is in
/// its name (case counts); a female, the female picture; else the male one.
fn avatar_of(e: &mut Engine, base: u32, name: &[u8]) -> Vec<u8> {
    let id = e.mem.u32(base + 0xc);
    if let Some(path) = hud(e).path_for_id.get(&id) {
        return path.clone();
    }
    let found = hud(e)
        .avatar_paths
        .get(&crate::containers::str_hash_ci(name))
        .cloned();
    let path = match found {
        Some(path) => path,
        None => {
            if super::type_of(e, base) == TES_CREATURE {
                hud(e)
                    .avatar_common
                    .iter()
                    .find(|(key, _)| contains(name, key))
                    .map_or(AVATAR_MALE.to_vec(), |(_, p)| p.clone())
            } else if e.mem.u32(base + BASE_FLAGS) & 1 != 0 {
                AVATAR_FEMALE.to_vec()
            } else {
                AVATAR_MALE.to_vec()
            }
        }
    };
    hud(e).path_for_id.insert(id, path.clone());
    path
}

/// `strstr(text, part) != nullptr` (an empty part is found).
fn contains(text: &[u8], part: &[u8]) -> bool {
    part.is_empty() || text.windows(part.len()).any(|w| w == part)
}

/// The action shown for a companion: below rank 7 in the task faction, its running package's
/// kind (a package following or escorting the player 1, waiting 2, fleeing 5 or 6, a sandbox 6,
/// a use-weapon package 13); `None` when it has none or another kind (nothing is set).
fn action_of(e: &mut Engine, actor: u32) -> Option<f32> {
    let faction = hud(e).task_faction;
    let rank = super::jip_fn_ccc::extra_faction_rank(e, actor, faction);
    if rank >= 7 {
        return Some(f32::from(rank));
    }
    let info = package_info(e, actor)?;
    let package = e.mem.u32(info);
    let value = match e.mem.u8(package + PACKAGE_TYPE) as i8 {
        1 | 2 | 6 | 7 => {
            if e.mem.u32(info + INFO_TARGET) != e.mem.u32(PLAYER) {
                return None;
            }
            1
        }
        5 | 9 | 13 | 14 => 2,
        18 => 5 + (e.call(GET_FLEE_COMBAT, &args![package]).u32() & 0xff != 0) as i32,
        10 | 22 => 6,
        12 => 13,
        _ => return None,
    };
    Some(value as f32)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSetTrait_Execute.
/// `[actor.]CCCSetTrait trait slot [value]` (when the HUD values were found): trait 40 the
/// avatar of slot `slot` (the calling actor's picture and name; with no actor, the male
/// picture); trait 0 the calling actor's health bar (64 times its health over its base health,
/// -1 while knocked out), whether it is more than 8192 units away from the player (not while
/// knocked out), and its action; any other trait, HUD value `trait + slot` set to `value`.
fn ccc_set_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !hud(e).loaded {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (trait_, slot, value) = (
        read.int(0, 0) as i32,
        read.int(1, 0) as i32,
        read.int(2, 0) as i32,
    );
    let at = |n: i32| i64::from(n) + i64::from(slot);
    if trait_ == 40 {
        let mut path = AVATAR_MALE.to_vec();
        if a.this_obj != 0 && super::is_actor(e, a.this_obj) {
            let base = e.mem.u32(a.this_obj + REF_BASE);
            let text = e.mem.u32(base + BASE_NAME);
            let name = if text == 0 {
                Vec::new()
            } else {
                nvse::table::guest_text(e, text)
            };
            if name.is_empty() {
                return handled(true);
            }
            path = avatar_of(e, base, &name);
            if let Some(v) = element(e, at(50)) {
                crate::game_patches::gb::value_set_string(e, v, &name);
            }
        }
        if let Some(v) = element(e, at(40)) {
            crate::game_patches::gb::value_set_string(e, v, &path);
        }
    } else if trait_ == 0 {
        let actor = a.this_obj;
        if actor == 0 || !super::is_actor(e, actor) {
            return handled(true);
        }
        if knocked_state(e, actor) != 3 {
            let health = e
                .vcall(actor + AV_OWNER, GET_ACTOR_VALUE, &[AV_HEALTH])
                .f64() as f32;
            let full = e
                .vcall(actor + AV_OWNER, GET_BASE_ACTOR_VALUE, &[AV_HEALTH])
                .f64() as f32;
            if let Some(v) = element(e, at(10)) {
                value_set_float(e, v, 64.0 * health / full);
            }
            let player = e.mem.u32(PLAYER);
            let far = distance(e, player, actor) > 8192.0;
            if let Some(v) = element(e, at(20)) {
                value_set_bool(e, v, far);
            }
        } else if let Some(v) = element(e, at(10)) {
            value_set_float(e, v, -1.0);
        }
        let Some(action) = action_of(e, actor) else {
            return handled(true);
        };
        if let Some(v) = element(e, at(30)) {
            value_set_float(e, v, action);
        }
    } else if let Some(v) = element(e, at(trait_)) {
        value_set_float(e, v, value as f32);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSavedForm_Execute.
/// `CCCSavedForm index`: saved form `index` (0 to 9; 0 before `CCCOnLoad` or for a mod not
/// loaded), as a form.
fn ccc_saved_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let index = read.int(0, 0) as usize;
        if let Some(&id) = hud(e).saved_forms.get(index) {
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h: Cmd_CCCSMS_Execute.
/// `CCCSMS [ref]`: every effect shader of CC&C's file playing on the reference (the process
/// manager's temporary effects that are a `MagicShaderHitEffect`) told to stop (flags 1).
fn ccc_sms(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let target = read.form(0);
    if !read.ok || target == 0 {
        return handled(true);
    }
    let index = hud(e).mod_index;
    let mut node = PROCESS_MANAGER + TEMP_EFFECTS;
    let mut walked = 0usize;
    while node != 0 && super::list_step(e, &mut walked) {
        let effect = e.mem.u32(node);
        if effect != 0
            && e.mem.u32(effect) == MAGIC_SHADER_HIT_EFFECT_VTBL
            && e.mem.u32(effect + EFFECT_TARGET) == target
        {
            let shader = e.mem.u32(effect + EFFECT_SHADER);
            if e.mem.u8(shader + MOD_INDEX) == index {
                e.mem.set_u8(effect + EFFECT_FLAGS, 1);
            }
        }
        node = e.mem.u32(node + 4);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ccc_hud_tests.rs"]
mod tests;
