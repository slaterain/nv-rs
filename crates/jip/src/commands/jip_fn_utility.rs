//! `functions_jip/jip_fn_utility.h` (JIP LN NVSE): form ids to text and back, the smallest and
//! largest of up to five numbers, the load order checks, clearing a mod's JIP data, a plugin's
//! header version, an array element as a number, a random integer, and whether the game has the
//! large address space, (lane C1) the seconds since the game started, and (lane files) arrays and
//! texts read from and written to files and mod logs (`ReadArrayFromFile`, `WriteArrayToFile`,
//! `ReadStringFromFile`, `WriteStringToFile`, `ModLogPrint`) through the core's seam
//! `engine_core::mod_files`: what a mod writes is kept in the port's own folder beside the saves,
//! never in the game folder. 17 of its 19 commands.
//!
//! Left for later, each for its reason: `SetOptionalPatch` switches JIP's patches (the switch
//! rows of step 3.9); `GetOptionalPatch` answers whether each switch's hook or patch is in (`HOOK_INSTALLED`,
//! `s_patchInstallState`): the port's switch word (`Engine::jip_patches`) says what the player's
//! ini asks for, and of the switch rows only O6 and O7 are on main (survey of 2026-11-15), so it
//! would answer 1 for patches the port does not have (`bIgnoreDTDRFix` is on by default);
//! it waits for those rows, or for a per-switch "in" state that each row sets. `SetOptionalPatch`
//! must also do what JIP does on a switch: turning O1 (`bIgnoreDTDRFix`) off sets the last hit's
//! damage threshold G121 shows back to `-FLT_MAX` (`gc::damage`, `s_VATSHitDT`).

use super::arrays::{append_element, create_array, Element};
use super::jip_fn_aux_variable::{array_values, overriding_mod};
use super::jip_fn_ui::next_token;
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
    (0x263a, read_array_from_file),
    (0x263b, read_string_from_file),
    (0x263c, write_array_to_file),
    (0x263d, write_string_to_file),
    (0x268e, get_load_order_changed),
    (0x268f, validate_mod_index),
    (0x26aa, clear_jip_saved_data),
    (0x287f, get_plugin_header_version),
    (0x28c8, get_array_value),
    (0x28de, get_random_in_range),
    (0x2892, get_is_laa),
    (0x272e, get_session_time),
    (0x27c0, mod_log_print),
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
pub(crate) fn num_args(e: &Engine, a: &CommandArgs) -> u8 {
    e.mem.u8(a.data.wrapping_add(e.mem.u32(a.opcode_offset)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h: NUM_ARGS_EX.
/// [`num_args`], or 0 when the low byte of the line's argument data length (the word before the
/// count) is 0: a statement compiled with no argument data has length 0 and no count (xNVSE's
/// own analyser skips such lines, `ScriptAnalyzer.cpp` `if (context.length)`), and the byte
/// there is the next line's. JIP uses it in `GetReticlePos`, `GetReticleRange`,
/// `MoveToEditorPosition`, `GetItemRefCurrentHealth` and `ReloadEquippedModels`.
pub(crate) fn num_args_ex(e: &Engine, a: &CommandArgs) -> u8 {
    let offset = e.mem.u32(a.opcode_offset);
    if e.mem.u8(a.data.wrapping_add(offset).wrapping_sub(2)) == 0 {
        0
    } else {
        num_args(e, a)
    }
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
    let mut flags = 0;
    if scr != 0 {
        saved.script_vars.retain(|_, vars| {
            vars.retain(|_, v| u32::from(v.mod_index) != modi);
            !vars.is_empty()
        });
    }
    if links != 0 && !saved.linked_refs.is_empty() {
        saved
            .linked_refs
            .retain(|_, l| u32::from(l.mod_index) != modi);
        flags |= changed::LINKED_REFS;
    }
    if maps != 0 && saved.ref_maps.erase(&if maps == 2 { 0xff } else { modi }) {
        flags |= changed::REF_MAPS;
    }
    if flags != 0 {
        crate::cosave::mark_changed(e, flags);
    }
    // The saved aux variables, under their own lock (`AuxMaps`), not JIP's slot.
    if aux != 0
        && super::jip_fn_aux_variable::aux_maps(e, false).erase(&if aux == 2 { 0xff } else { modi })
    {
        super::jip_fn_aux_variable::mark_changed(e, changed::AUX_VARS);
    }
    if aux != 0 {
        super::jip_fn_aux_variable::publish_saved(e);
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

/// What JIP's `GetIsLAA` answers for a large-address-aware game whose process reaches memory above
/// 2 GB: 2. JIP's answers: 0 when the exe's header has no large-address flag (`IMAGE_FILE_LARGE_
/// ADDRESS_AWARE`, 0x20 at +0x16 of the file header), 1 when it has the flag but a test allocation
/// of 256 MB, growing by 256 MB to 2 GB, never ends above 0x80000000 (a 32-bit Windows without
/// the 3 GB switch), 2 when one does. The port's memory goes past what the patched game gets, so it
/// answers the most JIP tells apart (the lead's decision, 2026-11-15); mods use it to warn about an
/// unpatched game or to size their caches.
pub const IS_LAA: f64 = 2.0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h: Cmd_GetIsLAA_Execute;
// internal/jip_core.cpp: GetIsLAA.
/// [`IS_LAA`], echoed in the console.
fn get_is_laa(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, IS_LAA);
    crate::console::print_result(e, &a);
    handled(true)
}

// ---- the session time (lane C1) ----

/// The game's millisecond clock (`00457FE0`, `cdecl`, no words: it calls the import at
/// `00FDF060` and returns its answer).
const TICK_COUNT: u32 = 0x0045_7fe0;

/// JIP's `s_initialTickCount`: the clock at `DeferredInit` (0 before it, as JIP's static). Kept on
/// the main thread's engine: the start-up message and the command both run there.
#[derive(Default)]
pub(crate) struct SessionStart {
    ticks: u32,
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit
// (`s_initialTickCount = CdeclCall<UInt32>(0x457FE0)`).
/// The clock taken once the game has started (xNVSE's deferred start message).
pub(crate) fn deferred_init(e: &mut Engine) {
    // Always there in the game; a test engine without the exe's translations has no clock.
    if !e.is_registered(TICK_COUNT) {
        return;
    }
    let ticks = e.call(TICK_COUNT, &args![]).u32();
    e.ext::<SessionStart>().ticks = ticks;
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetSessionTime_Execute.
/// The seconds since the start: the clock less the start (wrapping), read as a signed 32-bit
/// count of milliseconds, times 0.001.
fn get_session_time(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let now = e.call(TICK_COUNT, &args![]).u32();
    let start = e.ext::<SessionStart>().ticks;
    a.set_result(e, f64::from(now.wrapping_sub(start) as i32) * 0.001);
    handled(true)
}
// ---- Mods' files: arrays and texts read from and written to files, mod logs (lane files) ----
//
// Every name goes through the core's seam (`engine_core::mod_files`): a file a mod writes is kept
// in the port's own folder beside the saves, never in the game's folder; reads look there first,
// then in the game's. A name the seam refuses (outside the game's folder) answers as a file
// Windows could not open: nothing read, result 0.

/// JIP's buffers: `char filePath[0x100]`; `GetStrArgBuffer()` (`STR_BUFFER_SIZE`, 0x40000), which
/// takes a file's lines whole, `ReadStringFromFile`'s path, `WriteStringToFile`'s path (its
/// first 0x100 bytes) and text (after them), and `ModLogPrint`'s text; `kMaxMessageLength`
/// (0x4000), less one: what `ReadStringFromFile` reads of a file.
const FILE_PATH: usize = 0x100;
const STR_ARG_BUFFER: usize = 0x4_0000;
const READ_STRING_MAX: usize = 0x4000 - 1;
/// `numParams` of `WriteStringToFile` (path, append, then xNVSE's 21 format parameters) and of
/// `ModLogPrint` (the indent, then the 21).
const WRITE_STRING_PARAMS: u32 = 23;
const MOD_LOG_PARAMS: u32 = 22;
/// `ModInfo::hasModLog` (+0x29B, JIP's own byte in the game's file record): the file's log was
/// started in this session, so later prints append.
const HAS_MOD_LOG: u32 = 0x29b;
/// `kIndentLevelStr`: 40 tabs.
const INDENT_TABS: usize = 40;

/// `ReplaceChr(path, '/', '\\')`.
fn backslashes(path: &mut [u8]) {
    for b in path {
        if *b == b'/' {
            *b = b'\\';
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: LineIterator.
/// The lines `LineIterator` gives of a file: its bytes with CR and LF made 0 (those before its
/// first 0 byte), the texts between
/// 0s (empty ones skipped) up to one that starts with byte 3 (JIP's end mark after the bytes).
/// `None` when the file is not there. JIP reads the whole file into its string buffer: a file
/// that does not fit ([`super::formatted_in_buffer`]'s rule) stops the port without the list
/// guard row.
fn file_lines(e: &Engine, path: &[u8], command: &str) -> Option<Vec<Vec<u8>>> {
    let bytes = engine_core::mod_files::read(e, path)?;
    if bytes.len() + 2 > STR_ARG_BUFFER && !super::fix_on(e, super::LIST_GUARD) {
        panic!(
            "{command}: a file of {} bytes runs past JIP's {STR_ARG_BUFFER}-byte buffer: the \
             port stops here (NV_NVSE_LIST_GUARD reads it whole)",
            bytes.len()
        );
    }
    // CR and LF become 0 only before the file's first 0 byte (`ReplaceChr` stops at it); the
    // whole buffer is then cut at 0 bytes.
    let mut bytes = bytes;
    let text_end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    for b in &mut bytes[..text_end] {
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
    Some(out)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: FileStream::OpenWrite,
// FileStream::MakeAllDirs.
/// The file written: a LF and the text added at its end when `append` and it is there (the
/// seam's append: only those bytes are written, as `fopen(.., "ab")`), else made new (its folders
/// first when it is not there: JIP asks `FileExists`, the attributes, not a read). False when the
/// seam refuses the name or the write, or when appending to a file that is there but cannot be
/// opened (JIP's `fopen(.., "ab")` fails there: nothing is written, the file is kept).
fn open_write(e: &mut Engine, path: &[u8], append: bool, text: &[u8]) -> bool {
    // One kernel lock hold over the look and the write, as the seam's profile writes hold it,
    // so another thread's write of the file cannot fall between them.
    let _kernel = engine_core::real_threads::kernel_guard(e);
    if engine_core::mod_files::exists(e, path) {
        if append {
            let mut bytes = Vec::with_capacity(text.len() + 1);
            bytes.push(b'\n');
            bytes.extend_from_slice(text);
            return engine_core::mod_files::append(e, path, &bytes);
        }
    } else {
        for (i, &b) in path.iter().enumerate() {
            if b == b'\\' {
                engine_core::mod_files::create_directory(e, &path[..i]);
            }
        }
    }
    engine_core::mod_files::write(e, path, text)
}

/// One value of a file line as `ReadArrayFromFile` reads it: `@` a form by `StringToRef` (looked
/// up, 0 when gone), `$` a text, else JIP's `StrToDbl`.
fn file_element(e: &mut Engine, token: &[u8]) -> Element {
    match token.first() {
        Some(b'@') => {
            let id = string_to_ref(e, &token[1..]);
            Element::Form(lookup_form(e, id))
        }
        Some(b'$') => Element::Str(token[1..].to_vec()),
        _ => Element::Number(crate::utility::str_to_dbl(token)),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_ReadArrayFromFile_Execute; internal/utility.cpp: GetNextToken (char).
/// A file of tab-separated values as an array: the first line sets the columns (its values up
/// to an empty one; none: nothing); each later line gives that many values (0 past its end). One
/// line without `transpose`: that line's array; else an array of the columns' arrays, or with
/// `transpose` of the lines' arrays.
fn read_array_from_file(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut path = super::buffer_arg(e, &read, 0, FILE_PATH, "ReadArrayFromFile");
    let transpose = read.int(1, 0) != 0;
    backslashes(&mut path);
    let Some(lines) = file_lines(e, &path, "ReadArrayFromFile") else {
        return handled(true);
    };
    let Some((first, rest)) = lines.split_first() else {
        return handled(true);
    };
    let mut elements = Vec::new();
    let mut data: &[u8] = first;
    loop {
        let (token, next) = next_token(data, b'\t');
        if token.is_empty() {
            break;
        }
        elements.push(file_element(e, token));
        data = next;
    }
    let columns = elements.len();
    if columns == 0 {
        return handled(true);
    }
    for line in rest {
        let mut data: &[u8] = line;
        for _ in 0..columns {
            if data.is_empty() {
                elements.push(Element::Number(0.0));
            } else {
                let (token, next) = next_token(data, b'\t');
                elements.push(file_element(e, token));
                data = next;
            }
        }
    }
    let count = lines.len();
    if count == 1 && !transpose {
        let id = create_array(e, a.script, &elements);
        a.set_result(e, f64::from(id as i32));
        return handled(true);
    }
    let main = create_array(e, a.script, &[]);
    if !transpose {
        for c in 0..columns {
            let column: Vec<Element> = (0..count)
                .map(|l| elements[l * columns + c].clone())
                .collect();
            let id = create_array(e, a.script, &column);
            append_element(e, main, &Element::Array(id));
        }
    } else {
        for line in elements.chunks(columns) {
            let id = create_array(e, a.script, line);
            append_element(e, main, &Element::Array(id));
        }
    }
    a.set_result(e, f64::from(main as i32));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h: WriteElemToFile.
/// Value `idx` of a column as `WriteArrayToFile` writes it: a number as `FltToStr`, a form as `@`
/// and its editor ID (with `edid`, when it has one) or `RefToString`, a text as `$` and the
/// text; `0` for none (past the column's end, no form, an array).
fn file_value(e: &mut Engine, column: &[Data], idx: usize, edid: bool) -> Vec<u8> {
    match column.get(idx) {
        Some(Data::Number(v)) => crate::utility::flt_to_str(*v),
        Some(Data::Form(id)) => {
            let form = lookup_form(e, *id);
            if form == 0 {
                return b"0".to_vec();
            }
            let mut out = vec![b'@'];
            if edid {
                let name = e.vcall(form, super::forms::GET_EDITOR_ID, &[]).u32();
                if name != 0 && e.mem.u8(name) != 0 {
                    out.extend(nvse::table::guest_text(e, name));
                    return out;
                }
            }
            out.extend(ref_string(e, form));
            out
        }
        Some(Data::Str(text)) => [&b"$"[..], text].concat(),
        _ => b"0".to_vec(),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_WriteArrayToFile_Execute.
/// An array written as tab-separated lines (each element an array is a column, any other value a
/// column of one; as many lines as the longest column), or with flag 1 a column a line; flag 2
/// writes forms by editor ID. Appended after a LF when asked and the file is there; 1 when
/// written.
fn write_array_to_file(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut path = super::buffer_arg(e, &read, 0, FILE_PATH, "WriteArrayToFile");
    let append = read.int(1, 0) != 0;
    let flags = read.int(3, 0);
    let Some(top) = array_values(e, read.int(2, 0)) else {
        return handled(true);
    };
    if top.is_empty() {
        return handled(true);
    }
    let mut lines = 1;
    let columns: Vec<Vec<Data>> = top
        .iter()
        .map(|v| match v {
            Data::Array(id) => {
                let column = array_values(e, *id).unwrap_or_default();
                lines = lines.max(column.len());
                column
            }
            other => vec![other.clone()],
        })
        .collect();
    backslashes(&mut path);
    let edid = flags & 2 != 0;
    let mut text = Vec::new();
    if flags & 1 == 0 {
        for idx in 0..lines {
            for (c, column) in columns.iter().enumerate() {
                text.extend(file_value(e, column, idx, edid));
                if columns.len() - c > 1 {
                    text.push(b'\t');
                }
            }
            if lines - idx > 1 {
                text.push(b'\n');
            }
        }
    } else {
        for (c, column) in columns.iter().enumerate() {
            for idx in 0..lines {
                text.extend(file_value(e, column, idx, edid));
                if lines - idx > 1 {
                    text.push(b'\t');
                }
            }
            if columns.len() - c > 1 {
                text.push(b'\n');
            }
        }
    }
    if open_write(e, &path, append, &text) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_ReadStringFromFile_Execute; internal/utility.cpp: FileToBuffer.
/// A file's text (its first 0x3FFF bytes, up to a 0 byte): from line `start` (from 1; 0 or 1:
/// the first), `count` lines (the line end of the last one left out; -1 or none: to the end); a
/// `count` of 0, or an empty or missing file, `""`.
fn read_string_from_file(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut text = Vec::new();
    if read.ok {
        let mut start = read.int(1, 0) as i32;
        let mut count = read.int(2, u32::MAX) as i32;
        let mut path = super::buffer_arg(e, &read, 0, STR_ARG_BUFFER, "ReadStringFromFile");
        backslashes(&mut path);
        if count != 0 {
            // Only the first 0x3FFF bytes are read (`FileToBuffer`'s `fread`), however large
            // the file.
            if let Some(mut bytes) = engine_core::mod_files::read_head(e, &path, READ_STRING_MAX)
                .filter(|b| !b.is_empty())
            {
                // The text ends at its first 0 byte: JIP looks for line ends in the string only
                // (`FindChr` stops at the 0).
                if let Some(nul) = bytes.iter().position(|&b| b == 0) {
                    bytes.truncate(nul);
                }
                let (mut from, mut to) = (0usize, bytes.len());
                start -= 1;
                if start > 0 || count > 0 {
                    let mut at = 0usize;
                    while let Some(n) = bytes[at..].iter().position(|&b| b == b'\n') {
                        let nl = at + n;
                        if start > 0 {
                            start -= 1;
                            from = nl + 1;
                        } else if count <= 0 {
                            break;
                        } else {
                            count -= 1;
                            if count == 0 {
                                to = if nl > 0 && bytes[nl - 1] == b'\r' {
                                    nl - 1
                                } else {
                                    nl
                                };
                                break;
                            }
                        }
                        at = nl + 1;
                    }
                }
                text = bytes[from.min(to)..to].to_vec();
            }
        }
    }
    super::assign_string(e, &a, Some(text));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_WriteStringToFile_Execute.
/// The formatted text written to the file (after its bytes and a LF when asked and the file is
/// there); 1 when written.
fn write_string_to_file(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 2, WRITE_STRING_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let mut path = nvse::commands::args_in_buffer(e, &f.leading, 0, FILE_PATH, "WriteStringToFile");
    let append = matches!(f.leading.get(1), Some(Arg::Int(v)) if *v != 0);
    let text =
        super::formatted_in_buffer(e, &f.text, STR_ARG_BUFFER - FILE_PATH, "WriteStringToFile");
    backslashes(&mut path);
    if open_write(e, &path, append, &text) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_ModLogPrint_Execute.
/// The formatted text written to `Mod Logs\<the script's file>.log`, after `indent` tabs (40 for
/// 40 or more): the first print of a session starts the file anew, later ones append after a LF.
/// Nothing for a script of no file.
fn mod_log_print(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let index = overriding_mod(e, a.script);
    if index == 0xff {
        return handled(true);
    }
    let f = nvse::commands::string::format_string_args_at(e, &a, 1, MOD_LOG_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let indent = match f.leading.first() {
        Some(Arg::Int(v)) => *v as u32,
        _ => 0,
    };
    let text = super::formatted_in_buffer(e, &f.text, STR_ARG_BUFFER, "ModLogPrint");
    let info = e.mem.u32(e.mem.u32(DATA_HANDLER) + LOADED_MODS + 4 * index);
    // JIP reads the file's record unchecked; a slot with none here writes nothing.
    if info == 0 {
        return handled(true);
    }
    let mut path = b"Mod Logs\\".to_vec();
    path.extend(nvse::table::guest_text(e, info + MOD_NAME));
    path.extend_from_slice(b".log");
    let mut line = Vec::new();
    if indent != 0 {
        line.resize(
            if indent >= INDENT_TABS as u32 {
                INDENT_TABS
            } else {
                indent as usize
            },
            b'\t',
        );
    }
    line.extend_from_slice(&text);
    let append = e.mem.u8(info + HAS_MOD_LOG) != 0;
    if open_write(e, &path, append, &line) {
        e.mem.set_u8(info + HAS_MOD_LOG, 1);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_utility_tests.rs"]
mod tests;
