//! `functions_ln/ln_fn_game_data.h` (JIP LN NVSE): the loaded forms of a type (into a form list
//! or an array), the console's form search, file names and indexes, a form's files and its
//! references, a form by file and id, and the game's settings. All 12 of its commands.
//!
//! `DataHandler` (`*011C3F2C`, JIP's header; the recipe list at +0xA0 checked in batch 6): the
//! bound objects (`+4`: count, first, last; each object's next at +0x20), the form lists from
//! +8 (eight bytes each, by `kTypeListJmpTbl`), the worldspaces (+0x10), the regions
//! (`regionList` +0x1D8, its list at +4), the interior cells (`cellArray` +0x1DC: data +4, the
//! count `firstFreeEntry` +0xA), the loaded files (+0x218 count, +0x21C the 255 slots).
//! `TES` (`*011DEA10`): the grid of loaded exterior cells (+8: size +0xC, cells +0x10). A cell's
//! references: `objectList`, a `tList` at +0xAC. A worldspace's persistent cell: +0x34. Every form
//! by id: `AllFormsMap` (`*011C54C0`, a `NiTPointerMap`: buckets +4, table +8; an entry's next +0,
//! key +4, form +8), walked bucket by bucket.
//!
//! The settings are JIP's `s_gameSettingsMap`, which the core track builds
//! ([`crate::overrides::game_setting`]).

use super::arrays::{create_array, create_string_map, Element};
use super::extra::{self, kind};
use super::forms::ListNode;
use super::jip_fn_aux_variable::overriding_mod;
use super::jip_fn_utility::{hex_to_uint, lookup_mod_by_name, nth_mod_name};
use super::{
    assign_string, buffer_arg, heap_alloc, heap_free, list_items, list_prepend, list_remove_all,
    ref_id, the_name, type_of, Handlers,
};
use crate::cosave::lookup_form;
use crate::utility::str_compare_ci;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2211, get_loaded_type),
    (0x224e, get_loaded_type_array),
    (0x2246, search),
    (0x221a, get_mod_name),
    (0x2239, get_form_mods),
    (0x2251, get_form_refs),
    (0x221d, get_self_mod_index),
    (0x223a, is_form_overridden),
    (0x2266, get_form_from_mod),
    (0x2283, get_string_setting),
    (0x2284, set_string_setting),
    (0x2337, get_game_settings),
];

const DATA_HANDLER: u32 = 0x011c_3f2c;
const BOUND_OBJECTS: u32 = 0x04;
const TYPE_LISTS: u32 = 0x08;
const WORLD_SPACES: u32 = 0x10;
const REGION_LIST: u32 = 0x1d8;
const CELL_ARRAY: u32 = 0x1dc;
const TES: u32 = 0x011d_ea10;
const ALL_FORMS: u32 = 0x011c_54c0;
const BOUND_NEXT: u32 = 0x20;
const CELL_OBJECTS: u32 = 0xac;
const WS_PERSISTENT_CELL: u32 = 0x34;
/// A form's files (`mods`, a `tList` at +0x10) and a file's name (+0x20).
const FORM_FILES: u32 = 0x10;
const MOD_NAME: u32 = 0x20;
/// A form list's forms (+0x18).
const LIST_FORMS: u32 = 0x18;
/// A reference's base form (+0x20) and form flags (+8, `kFlags_Persistent` 0x400).
const BASE: u32 = 0x20;
const PERSISTENT: u32 = 0x400;
/// `kFormType_TESObjectCELL`, `kFormType_TESRegion`, `kFormType_Max` (0x79), and JIP's own two:
/// map markers (301) and radios (302).
const TES_OBJECT_CELL: u32 = 0x39;
const TES_REGION: u32 = 0x37;
const FORM_TYPE_MAX: u32 = 0x79;
const MAP_MARKERS: u32 = 301;
const RADIOS: u32 = 302;
/// `TypeSignature::Array()` (`0101C2AC`, eight bytes each, in reverse order of type).
const TYPE_SIGNATURES: u32 = 0x0101_c2ac;
/// `TESForm::GetEditorID` (vtable +0x130).
const GET_EDITOR_ID: u32 = 0x130;
/// JIP's buffers.
const MOD_NAME_BUFFER: usize = 0x80;
const REF_ID_BUFFER: usize = 0x10;
const SEARCH_BUFFER: usize = 0x40;
const TYPE_BUFFER: usize = 0x10;
const SETTING_BUFFER: usize = 0x80;

/// `kTypeListJmpTbl` (JIP LN NVSE): the form list of each type (below 0x80), 0x80 for the bound
/// objects' list, 0xFF for neither (walk every form).
#[rustfmt::skip]
const TYPE_LIST_JMP_TBL: [u8; 121] = [
    255, 255, 255, 255, 128, 50, 28, 15, 16, 8, 9, 10, 11, 25, 26, 255, 255, 24, 13, 6, 7, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128,
    128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 48, 128, 39, 128, 5, 2, 58, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 1, 255, 255, 255, 32, 33, 34, 255, 0, 35, 36, 128, 51, 37, 38, 255, 40, 45, 3, 4, 49, 46, 47, 128, 255, 41, 14,
    255, 29, 30, 31, 128, 12, 52, 27, 255, 53, 54, 128, 17, 255, 19, 20, 128, 22, 55, 56, 57, 18, 21, 128, 128, 23, 42, 43, 44,
];

/// `kValidSearchTypes` (JIP LN NVSE): the types `Search` looks at.
#[rustfmt::skip]
const VALID_SEARCH_TYPES: [u8; 121] = [
    0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 0,
    0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0,
];

fn handler(e: &Engine) -> u32 {
    e.mem.u32(DATA_HANDLER)
}

/// A form's file index (`modIndex`, the id's top byte at +0xF).
fn form_mod(e: &Engine, form: u32) -> u8 {
    e.mem.u8(form + 0xf)
}

/// The forms of a `tList` from its first node, empty nodes left out.
fn list_forms(e: &Engine, head: u32) -> Vec<u32> {
    list_items(e, Ptr::<ListNode>::new(head))
        .into_iter()
        .filter(|&f| f != 0)
        .collect()
}

/// The interior cells (`cellArray`, `firstFreeEntry` entries, null ones included).
fn cell_array(e: &Engine) -> Vec<u32> {
    let arr = handler(e) + CELL_ARRAY;
    let (data, count) = (e.mem.u32(arr + 4), e.mem.u16(arr + 0xa));
    (0..u32::from(count))
        .map(|i| e.mem.u32(data + 4 * i))
        .collect()
}

/// Every form of `AllFormsMap`, bucket by bucket (null values included).
fn all_forms(e: &Engine) -> Vec<u32> {
    let map = e.mem.u32(ALL_FORMS);
    let (buckets, table) = (e.mem.u32(map + 4), e.mem.u32(map + 8));
    let mut out = Vec::new();
    for b in 0..buckets {
        let mut item = e.mem.u32(table + 4 * b);
        while item != 0 {
            out.push(e.mem.u32(item + 8));
            item = e.mem.u32(item);
        }
    }
    out
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h: GetSourceList.
/// The data handler's list of a type: the regions' for a region, else by the jump table; `None`
/// for none.
fn source_list(e: &Engine, kind: u32) -> Option<u32> {
    if kind == TES_REGION {
        return Some(e.mem.u32(handler(e) + REGION_LIST) + 4);
    }
    let idx = TYPE_LIST_JMP_TBL[kind as usize];
    (idx < 0x80).then(|| handler(e) + TYPE_LISTS + 8 * u32::from(idx))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h: GetLoadedType.
/// The loaded forms of a type of file `modi` (0xFF: any), in JIP's order: cells from the cell
/// array; 301 the references with an `ExtraMapMarker` in the worldspaces' persistent cells; 302
/// the references with an `ExtraRadioData` in the interior cells; else the type's list, the bound
/// objects (types the table gives 0x80), or every form. Types of 0x79 and above (but 301, 302):
/// none.
fn loaded_type(e: &mut Engine, kind: u32, modi: u8) -> Vec<u32> {
    let of_mod = |e: &Engine, f: u32| modi == 0xff || form_mod(e, f) == modi;
    let mut out = Vec::new();
    if kind == TES_OBJECT_CELL {
        for c in cell_array(e) {
            if c != 0 && of_mod(e, c) {
                out.push(c);
            }
        }
    } else if kind == MAP_MARKERS || kind == RADIOS {
        let cells: Vec<u32> = if kind == MAP_MARKERS {
            list_forms(e, handler(e) + WORLD_SPACES)
                .into_iter()
                .map(|ws| e.mem.u32(ws + WS_PERSISTENT_CELL))
                .collect()
        } else {
            cell_array(e).into_iter().filter(|&c| c != 0).collect()
        };
        let x = if kind == MAP_MARKERS {
            kind::MAP_MARKER
        } else {
            kind::RADIO_DATA
        };
        for c in cells {
            for r in list_forms(e, c + CELL_OBJECTS) {
                if extra::has_type(e, r + extra::REFR_EXTRA_LIST, x) && of_mod(e, r) {
                    out.push(r);
                }
            }
        }
    } else if kind < FORM_TYPE_MAX {
        if let Some(list) = source_list(e, kind) {
            for f in list_forms(e, list) {
                if of_mod(e, f) {
                    out.push(f);
                }
            }
        } else if TYPE_LIST_JMP_TBL[kind as usize] == 0x80 {
            let mut o = e.mem.u32(e.mem.u32(handler(e) + BOUND_OBJECTS) + 4);
            while o != 0 {
                if u32::from(type_of(e, o)) == kind && of_mod(e, o) {
                    out.push(o);
                }
                o = e.mem.u32(o + BOUND_NEXT);
            }
        } else {
            for f in all_forms(e) {
                if f != 0 && u32::from(type_of(e, f)) == kind && of_mod(e, f) {
                    out.push(f);
                }
            }
        }
    }
    out
}

/// `(index >= 0 && index < 255) ? index : 0xFF`.
fn index_arg(index: i32) -> u8 {
    if (0..255).contains(&index) {
        index as u8
    } else {
        0xff
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetLoadedType_Execute.
/// The forms put into the form list, each first (`Prepend`), the list emptied before unless told
/// not to.
fn get_loaded_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let list = Ptr::<ListNode>::new(read.form(0) + LIST_FORMS);
    let (kind, index, no_clear) = (read.int(1, 0), read.int(2, u32::MAX) as i32, read.int(3, 0));
    if no_clear == 0 {
        list_remove_all(e, list);
    }
    for f in loaded_type(e, kind, index_arg(index)) {
        list_prepend(e, list, f);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetLoadedTypeArray_Execute.
fn get_loaded_type_array(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let (kind, index) = (read.int(0, 0), read.int(1, u32::MAX) as i32);
        let forms: Vec<Element> = loaded_type(e, kind, index_arg(index))
            .into_iter()
            .map(Element::Form)
            .collect();
        let id = create_array(e, a.script, &forms);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: SubStrCI (asm).
/// Whether `sub` is in `text` without case (`kLwrCaseConverter`: only `A`-`Z` folded); false
/// when either is empty.
pub fn sub_str_ci(text: &[u8], sub: &[u8]) -> bool {
    if text.is_empty() || sub.is_empty() || sub.len() > text.len() {
        return false;
    }
    text.windows(sub.len())
        .any(|w| w.iter().zip(sub).all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase()))
}

/// A form's editor ID (the virtual function): its text and its guest pointer.
fn editor_id(e: &mut Engine, form: u32) -> (Vec<u8>, u32) {
    let at = e.vcall(form, GET_EDITOR_ID, &[]).u32();
    let text = if at == 0 {
        Vec::new()
    } else {
        nvse::table::guest_text(e, at)
    };
    (text, at)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_Search_Execute.
/// In the console only: every form of a searchable type (or the one type given by number or by
/// its four-letter signature) whose name or editor ID holds the text, printed as `[id]  sig
/// editorID  name`. An unknown or unsearchable type prints nothing.
fn search(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !nvse::commands::is_console_mode(e) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let find = buffer_arg(e, &read, 0, SEARCH_BUFFER, "Search");
    let type_text = buffer_arg(e, &read, 1, TYPE_BUFFER, "Search");
    let mut filter = 0usize;
    if let Some(&first) = type_text.first() {
        if first.is_ascii_digit() {
            let n = crate::utility::str_to_int(&type_text) as u32;
            if n > 120 || VALID_SEARCH_TYPES[n as usize] == 0 {
                return handled(true);
            }
            filter = n as usize;
        } else {
            let mut four = [0u8; 4];
            for (i, b) in four.iter_mut().enumerate() {
                *b = type_text.get(i).copied().unwrap_or(0);
            }
            let u = u32::from_le_bytes(four) & 0xdfdf_dfdf;
            for idx in 3..=120u32 {
                if u != e.mem.u32(TYPE_SIGNATURES + 8 * idx) {
                    continue;
                }
                filter = (120 - idx) as usize;
                break;
            }
            if VALID_SEARCH_TYPES[filter] == 0 {
                return handled(true);
            }
        }
    }
    let valid = |t: usize| {
        if filter == 0 {
            VALID_SEARCH_TYPES.get(t).copied().unwrap_or(0) != 0
        } else {
            t == filter
        }
    };
    for form in all_forms(e) {
        if form == 0 || !valid(usize::from(type_of(e, form))) {
            continue;
        }
        let name = the_name(e, form);
        let (edid, edid_at) = editor_id(e, form);
        if sub_str_ci(&name, &find) || sub_str_ci(&edid, &find) {
            let t = u32::from(type_of(e, form));
            let sig = TYPE_SIGNATURES + 8 * (120u32.wrapping_sub(t));
            let name_at = nvse::table::guest_str(e, &name);
            let id = ref_id(e, form);
            nvse::commands::console_print(e, "[%08X]  %s  %s  %s", &[id, sig, edid_at, name_at]);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetModName_Execute.
/// The loaded file's name, without its extension (cut at the first dot) unless told to keep it;
/// `""` for an index of 0xFF or more.
fn get_mod_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (index, keep) = (read.int(0, 0), read.int(1, 0));
    let mut name = Vec::new();
    if read.ok && index < 0xff {
        name = nth_mod_name(e, index);
        if !name.is_empty() && keep == 0 {
            if let Some(dot) = name.iter().position(|&b| b == b'.') {
                name.truncate(dot);
            }
        }
    }
    assign_string(e, &a, Some(name));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetFormMods_Execute.
/// The names of the files that hold the form (else the calling reference's base form), in order.
fn get_form_mods(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 && a.this_obj != 0 {
        form = e.mem.u32(a.this_obj + BASE);
    }
    if form != 0 {
        let names: Vec<Element> = list_forms(e, form + FORM_FILES)
            .into_iter()
            .map(|m| Element::Str(nvse::table::guest_text(e, m + MOD_NAME)))
            .collect();
        let id = create_array(e, a.script, &names);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetFormRefs_Execute.
/// Every reference to the form: in the interior cells, then (when asked) the grid's loaded
/// cells' references that are not persistent, then the worldspaces' persistent cells.
fn get_form_refs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (form, grid) = (read.form(0), read.int(1, 0));
    let mut out = Vec::new();
    for c in cell_array(e).into_iter().filter(|&c| c != 0) {
        for r in list_forms(e, c + CELL_OBJECTS) {
            if e.mem.u32(r + BASE) == form {
                out.push(Element::Form(r));
            }
        }
    }
    if grid != 0 {
        let cells = e.mem.u32(e.mem.u32(TES) + 8);
        let size = e.mem.u32(cells + 0xc);
        let data = e.mem.u32(cells + 0x10);
        for i in 0..size.wrapping_mul(size) {
            let c = e.mem.u32(data + 4 * i);
            if c == 0 {
                continue;
            }
            for r in list_forms(e, c + CELL_OBJECTS) {
                if e.mem.u32(r + BASE) == form && e.mem.u32(r + 8) & PERSISTENT == 0 {
                    out.push(Element::Form(r));
                }
            }
        }
    }
    for ws in list_forms(e, handler(e) + WORLD_SPACES) {
        let c = e.mem.u32(ws + WS_PERSISTENT_CELL);
        if c == 0 {
            continue;
        }
        for r in list_forms(e, c + CELL_OBJECTS) {
            if e.mem.u32(r + BASE) == form {
                out.push(Element::Form(r));
            }
        }
    }
    let id = create_array(e, a.script, &out);
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetSelfModIndex_Execute.
/// The calling script's file index (its id's top byte), 0xFF without a script.
fn get_self_mod_index(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let r = if a.script != 0 {
        form_mod(e, a.script)
    } else {
        0xff
    };
    a.set_result(e, f64::from(r));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_IsFormOverridden_Execute.
/// 1 when the last file holding the form loads after the calling script's file.
fn is_form_overridden(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let last = overriding_mod(e, read.form(0));
        if last != 0xff && last > u32::from(form_mod(e, a.script)) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetFormFromMod_Execute.
/// The id (the hex's low 24 bits under the file's index; `NONE` is file 0xFF) when a form has
/// it; nothing for a file not loaded or no form.
fn get_form_from_mod(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = buffer_arg(e, &read, 0, MOD_NAME_BUFFER, "GetFormFromMod");
    let id_text = buffer_arg(e, &read, 1, REF_ID_BUFFER, "GetFormFromMod");
    let index = if str_compare_ci(&name, b"NONE").0 != 0 {
        match lookup_mod_by_name(e, &name) {
            0 => return handled(true),
            info => match e.mem.u8(info + 0x40c) {
                0xff => return handled(true),
                i => i,
            },
        }
    } else {
        0xff
    };
    let id = (hex_to_uint(&id_text) & 0x00ff_ffff) | (u32::from(index) << 24);
    if lookup_form(e, id) != 0 {
        a.set_form_result(e, id);
    }
    handled(true)
}

/// A setting's name's first letter, lower case (`*name | 0x20`), and its value word (+4).
fn setting_prefix(e: &Engine, setting: u32) -> u8 {
    e.mem.u8(e.mem.u32(setting + 8)) | 0x20
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetStringSetting_Execute.
/// A string setting's text (names starting `s` or `S`), `""` for none.
fn get_string_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut text = None;
    if read.ok {
        let name = buffer_arg(e, &read, 0, SETTING_BUFFER, "GetStringSetting");
        if name.first().map(|b| b | 0x20) == Some(b's') {
            if let Some(s) = crate::overrides::game_setting(e, &name) {
                let at = e.mem.u32(s + 4);
                text = (at != 0).then(|| nvse::table::guest_text(e, at));
            }
        }
    }
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_SetStringSetting_Execute; nvse/GameSettings.cpp: Setting::Set(const char*, bool).
/// A string setting's text replaced: the old text freed, a copy of the new one (the game's heap,
/// its length and 0) put in.
fn set_string_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = buffer_arg(e, &read, 0, SETTING_BUFFER, "SetStringSetting");
    let value = read.text(1);
    let value = &value[..value.iter().position(|&b| b == 0).unwrap_or(value.len())];
    if name.first().map(|b| b | 0x20) == Some(b's') {
        if let Some(s) = crate::overrides::game_setting(e, &name) {
            heap_free(e, e.mem.u32(s + 4));
            let copy = heap_alloc(e, value.len() as u32 + 1);
            e.mem.write(copy, value);
            e.mem.set_u8(copy + value.len() as u32, 0);
            e.mem.set_u32(s + 4, copy);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_game_data.h:
// Cmd_GetGameSettings_Execute.
/// A map by name of the settings: with bit 2 the string ones (their text), with bit 1 the others
/// (a float for names starting `f`, else the value word as a signed integer). Default 3.
fn get_game_settings(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let which = read.int(0, 3);
    let mut keys = Vec::new();
    let mut values = Vec::new();
    for s in crate::overrides::game_settings(e) {
        let name = nvse::table::guest_text(e, e.mem.u32(s + 8));
        let value = match setting_prefix(e, s) {
            b's' if which & 2 != 0 => {
                let at = e.mem.u32(s + 4);
                Element::Str(if at == 0 {
                    Vec::new()
                } else {
                    nvse::table::guest_text(e, at)
                })
            }
            b's' => continue,
            _ if which & 1 == 0 => continue,
            b'f' => Element::Number(f64::from(e.mem.f32(s + 4))),
            _ => Element::Number(f64::from(e.mem.u32(s + 4) as i32)),
        };
        keys.push(name);
        values.push(value);
    }
    let id = create_string_map(e, a.script, &keys, &values);
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_game_data_tests.rs"]
mod tests;
