//! `functions_jip/jip_fn_utility.h` (JIP LN NVSE): form ids to text and back, the smallest and
//! largest of up to five numbers, the load order checks, clearing a mod's JIP data, a plugin's
//! header version, an array element as a number, and a random integer. 10 of its 19 commands.
//!
//! Left for later, each for its reason: `ReadArrayFromFile`, `WriteArrayToFile`,
//! `ReadStringFromFile`, `WriteStringToFile` and `ModLogPrint` read or write the player's files
//! (the platform keeps every file the game writes in memory, so a file a mod writes would not be
//! there next session; built together with JIP's INI commands once there is a file seam that
//! keeps them); `GetOptionalPatch`/`SetOptionalPatch` read and switch JIP's patches (the core
//! track's hooks); `GetIsLAA` asks about the process's address space (the 64-bit port's own
//! answer is the lead's to decide); `GetSessionTime` needs `s_initialTickCount`, which JIP takes
//! in its deferred start (not built yet).

use super::jip_fn_aux_variable::overriding_mod;
use super::{ref_id, Handlers};
use crate::containers::str_hash_ci;
use crate::cosave::{changed, lookup_form, resolved_mod_index, resolved_ref_id};
use crate::Jip;
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::Read;
use nvse::vars::{Data, Key};
use nvse::{handled, CommandArgs};
use std::collections::HashMap;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x262a, ref_to_string),
    (0x262b, string_to_ref_command),
    (0x262e, get_min_of),
    (0x262f, get_max_of),
    (0x268e, get_load_order_changed),
    (0x268f, validate_mod_index),
    (0x26aa, clear_jip_saved_data),
    (0x287f, get_plugin_header_version),
    (0x28c8, get_array_value),
    (0x28de, get_random_in_range),
];

/// `g_dataHandler` (`*011C3F2C`): its loaded file count (`modList.loadedModCount`, +0x218) and
/// the loaded files (`modList.loadedMods[255]`, +0x21C).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const LOADED_COUNT: u32 = 0x218;
const LOADED_MODS: u32 = 0x21c;
/// A file's name (`ModInfo::name`, +0x20) and its header's version (`header.version`, a float
/// at +0x3DC).
const MOD_NAME: u32 = 0x20;
const MOD_HEADER_VERSION: u32 = 0x3dc;
/// `GameGlobals::EditorIDsMap()` (`*011C54C8`, a `NiTStringPointerMap<TESForm>`): its vtable's
/// `CalculateBucket` (+4) and `Equal` (+8), its buckets (+8); an entry's next (+0), key (+4),
/// form (+8).
const EDITOR_IDS_MAP: u32 = 0x011c_54c8;
/// `g_BGSSaveLoadGame` (`*011DDF38`): `saveMods[255]` (+0x44) and `loadedMods[255]` (+0x143).
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const SAVE_MODS: u32 = 0x44;
/// `ADDR_GetRandomInt` (`00AA5230`, `thiscall` on the game's generator `GAME_RNG` `011C4180`,
/// with the bound).
const GET_RANDOM_INT: u32 = 0x00aa_5230;
const GAME_RNG: u32 = 0x011c_4180;
/// JIP's buffers: `char refStr[0x80]`, `char modName[0x50]`.
const REF_STR_BUFFER: usize = 0x80;
const MOD_NAME_BUFFER: usize = 0x50;
/// `IS_REFERENCE`: the vtable answers true (`008D0360`) at +0xF0.
const RETURN_TRUE: u32 = 0x008d_0360;
const VT_IS_REFERENCE: u32 = 0xf0;

/// `s_strRefs` (JIP LN NVSE, `internal/jip_core.cpp`): `StringToRef`'s answers for the session,
/// keyed only by the text's `StrHashCI` (`HashedKey<const char*>` keeps the hash, not the text:
/// two texts of one hash share an answer). Never cleared.
#[derive(Default)]
pub struct StrRefs(pub HashMap<u32, u32>);

/// `NUM_ARGS` (JIP LN NVSE): the argument count byte at the start of the arguments.
fn num_args(e: &Engine, a: &CommandArgs) -> u8 {
    e.mem.u8(a.data.wrapping_add(e.mem.u32(a.opcode_offset)))
}

fn is_reference(e: &Engine, form: u32) -> bool {
    e.mem.u32(e.mem.u32(form) + VT_IS_REFERENCE) == RETURN_TRUE
}

// ---- the data handler's files ----

/// `DataHandler::GetNthModName` (JIP LN NVSE): the name of loaded file `index` (no check of the
/// count, as JIP's), `""` for an index of 0xFF or more or an empty slot.
pub fn nth_mod_name(e: &Engine, index: u32) -> Vec<u8> {
    if index >= 0xff {
        return Vec::new();
    }
    let handler = e.mem.u32(DATA_HANDLER);
    let info = e.mem.u32(handler + LOADED_MODS + 4 * index);
    if info == 0 {
        return Vec::new();
    }
    nvse::table::guest_text(e, info + MOD_NAME)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameData.cpp: DataHandler::LookupModByName
// (asm).
/// The first loaded file whose name's `StrHashCI` is the text's (JIP compares the hash its
/// `InitModInfoNameHook` keeps at `ModInfo +0x120`, the same `StrHashCI` of the name the game
/// copies in: computed here from the name); 0 for none. The walk takes the first file before it
/// checks the count (`dec`, `jnz`), as JIP's: the game always has at least one.
pub fn lookup_mod_by_name(e: &Engine, name: &[u8]) -> u32 {
    let hash = str_hash_ci(name);
    let handler = e.mem.u32(DATA_HANDLER);
    let count = e.mem.u32(handler + LOADED_COUNT).max(1);
    for i in 0..count.min(0xff) {
        let info = e.mem.u32(handler + LOADED_MODS + 4 * i);
        if info != 0 && str_hash_ci(&nvse::table::guest_text(e, info + MOD_NAME)) == hash {
            return info;
        }
    }
    0
}

/// `DataHandler::GetModIndex`: the file's index (`ModInfo::modIndex`, +0x40C), 0xFF for none.
fn mod_index(e: &Engine, name: &[u8]) -> u32 {
    match lookup_mod_by_name(e, name) {
        0 => 0xff,
        info => u32::from(e.mem.u8(info + 0x40c)),
    }
}

// ---- JIP's text helpers ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: HexToUInt (asm).
/// The hexadecimal number at the end of the text, read from the last byte back: each digit
/// (`0-9`, `a-f` either case) shifted 4 more (the shift count wraps at 32, as `shl` masks it,
/// so a ninth digit adds into the low bits), stopping at the first byte that is not one (the
/// value so far kept). The count of bytes is the length's low byte (`mov ch, al`: a length of 256
/// reads 256 bytes, 257 one).
pub fn hex_to_uint(text: &[u8]) -> u32 {
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    if len == 0 {
        return 0;
    }
    let mut count = len as u8;
    let mut at = len;
    let mut value = 0u32;
    let mut shift = 0u8;
    loop {
        at -= 1;
        let byte = text[at];
        let mut d = byte.wrapping_sub(b'0');
        if (d as i8) < 0 {
            break;
        }
        if d as i8 > 9 {
            d |= 0x20;
            if (d as i8) < 0x31 || d as i8 > 0x36 {
                break;
            }
            d -= 0x27;
        }
        value = value.wrapping_add(u32::from(d) << (shift & 31));
        shift = shift.wrapping_add(4);
        count = count.wrapping_sub(1);
        if count == 0 || at == 0 {
            break;
        }
    }
    value
}

/// `GameGlobals::EditorIDsMap()->Lookup(text)`: the game's map's own bucket and compare
/// functions (its vtable), the text given in a buffer on the stack; the form, 0 for none.
fn editor_id_lookup(e: &mut Engine, text: &[u8]) -> u32 {
    let map = e.mem.u32(EDITOR_IDS_MAP);
    if map == 0 {
        return 0;
    }
    let size = (text.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, buffer| {
        let key = buffer.addr();
        e.mem.write(key, text);
        e.mem.set_u8(key + text.len() as u32, 0);
        let vtbl = e.mem.u32(map);
        let bucket = e.call(e.mem.u32(vtbl + 4), &args![map, key]).u32();
        let equal = e.mem.u32(vtbl + 8);
        let mut entry = e.mem.u32(e.mem.u32(map + 8) + 4 * bucket);
        while entry != 0 {
            let other = e.mem.u32(entry + 4);
            if e.call(equal, &args![map, key, other]).u32() & 0xff != 0 {
                return e.mem.u32(entry + 8);
            }
            entry = e.mem.u32(entry);
        }
        0
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: StringToRef.
/// A form id from a text, kept for the session by the text's hash ([`StrRefs`]): `mod:hex` (the
/// file by name; an unknown file 0; nothing before the colon is file 0xFF) with the hex not
/// mapped; else a form's editor ID; else the text as a hex id mapped to today's load order.
pub fn string_to_ref(e: &mut Engine, text: &[u8]) -> u32 {
    let hash = str_hash_ci(text);
    if let Some(&id) = e.ext::<StrRefs>().0.get(&hash) {
        return id;
    }
    let id = if let Some(colon) = text.iter().position(|&b| b == b':') {
        let index = if colon != 0 {
            mod_index(e, &text[..colon])
        } else {
            0xff
        };
        if index == 0xff && colon != 0 {
            0
        } else {
            (index << 24) | hex_to_uint(&text[colon + 1..])
        }
    } else {
        match editor_id_lookup(e, text) {
            0 => resolved_ref_id(e, hex_to_uint(text)),
            form => ref_id(e, form),
        }
    };
    e.ext::<StrRefs>().0.insert(hash, id);
    id
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::RefToString.
/// `file name:hex`: the name of the file of the id's top byte ([`nth_mod_name`], `""` for
/// 0xFF), a colon, the low 24 bits in upper-case hex without leading zeros (JIP keeps each
/// answer by id for the session: the same text every time).
pub fn ref_string(e: &Engine, form: u32) -> Vec<u8> {
    let id = ref_id(e, form);
    let mut text = nth_mod_name(e, id >> 24);
    text.push(b':');
    text.extend(crate::console::uint_to_hex(id & 0x00ff_ffff));
    text
}

// ---- the commands ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_RefToString_Execute.
/// With no argument the calling reference's text; with one, the form's (a reference's base
/// form's); `""` for none.
fn ref_to_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let form = if num_args(e, &a) == 0 {
        a.this_obj
    } else {
        let read = Read::of(e, &a);
        let form = if read.ok { read.form(0) } else { 0 };
        if form != 0 && is_reference(e, form) {
            e.mem.u32(form + 0x20)
        } else {
            form
        }
    };
    let text = (form != 0).then(|| ref_string(e, form));
    super::assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_StringToRef_Execute.
/// The id [`string_to_ref`] finds, in the result's low word (0 too).
fn string_to_ref_command(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let text = super::buffer_arg(e, &read, 0, REF_STR_BUFFER, "StringToRef");
        let id = string_to_ref(e, &text);
        a.set_form_result(e, id);
    }
    handled(true)
}

/// The five doubles as read (unread ones 0: never used).
fn doubles(read: &Read) -> [f64; 5] {
    let mut v = [0.0; 5];
    for (i, slot) in v.iter_mut().enumerate() {
        if let Some(Arg::Double(d)) = read.args.get(i) {
            *slot = *d;
        }
    }
    v
}

/// `minsd`/`maxsd` over the first `NUM_ARGS` values, from the first (`do ... while (++i <
/// numArgs)`): each step keeps the running value only when the compare holds, so a NaN either
/// side gives the next value.
fn fold_values(e: &mut Engine, w: &[u32], keep: fn(f64, f64) -> bool) -> Ret {
    let a = CommandArgs::new(w);
    let count = num_args(e, &a);
    let read = Read::of(e, &a);
    if read.ok {
        let v = doubles(&read);
        let mut acc = v[0];
        let mut i = 1u8;
        loop {
            let next = v[usize::from(i).min(4)];
            acc = if keep(acc, next) { acc } else { next };
            i = i.wrapping_add(1);
            if i >= count {
                break;
            }
        }
        a.set_result(e, acc);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetMinOf_Execute.
fn get_min_of(e: &mut Engine, w: &[u32]) -> Ret {
    fold_values(e, w, |acc, v| acc < v)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetMaxOf_Execute.
fn get_max_of(e: &mut Engine, w: &[u32]) -> Ret {
    fold_values(e, w, |acc, v| acc > v)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetLoadOrderChanged_Execute.
/// 1 when a file of the save (from index 254 down to 1: index 0 is not looked at) is loaded
/// today at another index than it had (`saveMods[i]` against `loadedMods[i]`, not 0xFF).
fn get_load_order_changed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let sl = e.mem.u32(SAVE_LOAD_GAME);
    for idx in (1..=254u32).rev() {
        let now = e.mem.u8(sl + SAVE_MODS + idx);
        if now == 0xff || now == e.mem.u8(sl + SAVE_MODS + idx + 0xff) {
            continue;
        }
        a.set_result(e, 1.0);
        break;
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_ValidateModIndex_Execute.
/// The save's file index as loaded today (0xFF stays); -1 for a file no longer loaded, an index
/// above 0xFF (a negative one too) or an argument not read.
fn validate_mod_index(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let index = read.int(0, 0);
    let r = if read.ok && index <= 0xff {
        resolved_mod_index(e, index as u8).map_or(-1.0, f64::from)
    } else {
        -1.0
    };
    a.set_result(e, r);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_ClearJIPSavedData_Execute.
/// For the calling script's mod: its script variables (owners left empty go; no changed flag, as
/// JIP's); its linked references when there are any (then the linked refs flag, whether or not
/// one was the mod's); its saved aux variables and ref maps (`2`: those of mod 0xFF, the shared
/// ones), each setting its flag only when the mod had any.
fn clear_jip_saved_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (scr, links, aux, maps) = (
        read.int(0, 0),
        read.int(1, 0),
        read.int(2, 0),
        read.int(3, 0),
    );
    let modi = overriding_mod(e, a.script);
    let saved = &mut e.ext::<Jip>().saved;
    if scr != 0 {
        saved.script_vars.retain(|_, vars| {
            vars.retain(|_, v| u32::from(v.mod_index) != modi);
            vars.len() != 0
        });
    }
    if links != 0 && saved.linked_refs.len() != 0 {
        saved
            .linked_refs
            .retain(|_, l| u32::from(l.mod_index) != modi);
        saved.changed |= changed::LINKED_REFS;
    }
    if aux != 0 && saved.aux_vars.erase(&if aux == 2 { 0xff } else { modi }) {
        saved.changed |= changed::AUX_VARS;
    }
    if maps != 0 && saved.ref_maps.erase(&if maps == 2 { 0xff } else { modi }) {
        saved.changed |= changed::REF_MAPS;
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetPluginHeaderVersion_Execute.
/// The loaded file's header version (a float), nothing for a name not loaded.
fn get_plugin_header_version(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let name = super::buffer_arg(e, &read, 0, MOD_NAME_BUFFER, "GetPluginHeaderVersion");
        let info = lookup_mod_by_name(e, &name);
        if info != 0 {
            let v = e.mem.f32(info + MOD_HEADER_VERSION);
            a.set_result(e, f64::from(v));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetArrayValue_Execute.
/// Element `index` (a number key) of the array: a number itself, a form (still loaded) its id in
/// the low word, an array its id; a text or nothing leaves the result 0.
fn get_array_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (id, index) = (read.int(0, 0), read.int(1, 0) as i32);
    if !read.ok || id == 0 {
        return handled(true);
    }
    let Some(el) = nvse::vars::get_element(e, id, &Key::Number(f64::from(index))) else {
        return handled(true);
    };
    match el.data {
        Data::Number(n) => a.set_result(e, n),
        Data::Form(f) => {
            let form = lookup_form(e, f);
            if form != 0 {
                let fid = ref_id(e, form);
                a.set_form_result(e, fid);
            }
        }
        Data::Array(arr) => a.set_result(e, f64::from(arr as i32)),
        Data::Str(_) | Data::Invalid => {}
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetRandomInRange_Execute; internal/utility.h: GetRandomIntInRange.
/// The game's generator with the bound `max - min` (32-bit wrap), plus `min`.
fn get_random_in_range(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let (lo, hi) = (read.int(0, 0) as i32, read.int(1, 0) as i32);
        let r = e
            .call(GET_RANDOM_INT, &args![GAME_RNG, hi.wrapping_sub(lo) as u32])
            .u32() as i32;
        a.set_result(e, f64::from(r.wrapping_add(lo)));
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_utility_tests.rs"]
mod tests;
