//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): JIP's own extra data on a reference or an
//! inventory stack (`RefHasExtraData`, `GetRefExtraData`, `SetRefExtraData`), kept per mod: 18
//! numbers or form ids and 2 texts under a key the reference's `ExtraJIP` holds (group GH1's
//! framework, row G44: [`crate::game_patches::gh1`]; the co-save's `JPED` record,
//! [`crate::cosave::ExtraData`]). The two last are expression-format commands: their
//! arguments are read by xNVSE's expression evaluator ([`nvse::expr::Eval`]) as JIP's
//! `PluginExpressionEvaluator` reads them.
//!
//! State: JIP's map of data (`s_extraDataKeysMap`) is [`cosave::extra_data`] (a std `Mutex` in a
//! lock-free slot, not JIP's shared slot) and its changed flag is set with
//! [`cosave::mark_changed`], as group GH1's routines reach them.
//!
//! Threads and locks: the commands run on the main thread or a script's. As JIP's `XDATA_CS`, each
//! takes the game's extra data lock (`0x11C3920`, the exe's recursive enter `0x40FBF0` and leave
//! `0x40FBA0`) around its reads and writes, and inside it takes the map's lock, a leaf: one take
//! per read or write, never held across a call into the game's code (`gh1::make_key` draws its
//! key before the map is taken; an `ExtraJIP` is taken off the list after the map is let go), the
//! order group GH1's routines use (the game holds `XDATA_CS` when they run and they take the map).
//!
//! Deliberate differences, where JIP (with xNVSE's inventory references) would read through null,
//! read or write freed memory, or overrun its own buffers:
//! - no reference answers nothing (JIP reads through it);
//! - an inventory reference whose container has no entry for the item when its stack is emptied
//!   keeps the stack (as JIP); an entry without a list of stacks is left (JIP removes from the null
//!   list);
//! - an emptied stack is freed only when it was really taken out of the entry's list of stacks
//!   (JIP frees it even when the entry did not hold it, leaving it linked wherever it is), and
//!   not while an inventory reference's deferred equip or removal is still to run on it (JIP's
//!   would run on the freed list); it then stays, holding only its count;
//! - when a stack is freed every inventory reference standing for it is made to stand for no
//!   stack (`set_data` with no list, which also clears the temporary reference's copy of the
//!   list's words): JIP leaves them pointing at the freed list, so a `ForEach` loop's write-back
//!   writes into freed memory and a second command on the reference in the same frame walks it;
//! - when the command adds to or takes from a stack, the copy of the stack's words each inventory
//!   reference's temporary reference holds follows: after an addition, a copy that matched the
//!   stack before is copied again; after the `ExtraJIP` is taken off (and freed), a copy whose
//!   chain still led to it is unlinked from it as the stack was. With JIP a `ForEach` loop's
//!   write-back puts the old copy back: an addition's extras are lost (a leak, the mod's data
//!   gone) and a removal's freed `ExtraJIP` can come back as the stack's first extra. A copy the
//!   script had already changed in the same step keeps JIP's outcome for an addition (the loop
//!   writes it back over the stack);
//! - a text is stored as JIP's `XString` holds it, up to its first zero byte and with JIP's 16-bit
//!   length (so the co-save's 16-bit length always matches the bytes written), but never more
//!   than that length: JIP copies one byte more and reads its text on to the next zero byte, and
//!   for a length of 0xFFEF or more its buffer size wraps to 0 and the copy overruns it;
//! - `GetRefExtraData` with no argument or more than two does nothing (xNVSE's `ExtractArgsV`
//!   leaves JIP's index unset, or writes past JIP's two arguments), nor with an index that is no
//!   number (JIP's index would be a pointer in an unset double); a mod index that is no number
//!   is 0, the script's own, as JIP's zeroed double with a pointer in its low word reads.

use super::extra;
use super::jip_fn_aux_variable::overriding_mod;
use super::{cvttsd2si, Handlers};
use crate::containers::Uint;
use crate::cosave::{self, changed, ExtraJipData};
use crate::game_patches::gh1;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::expr::{expect_return_type, Eval, Tok, Ty};
use nvse::table::Returns;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x292e, ref_has_extra_data),
    (0x292f, get_ref_extra_data),
    (0x2930, set_ref_extra_data),
];

/// The game's extra data lock (`EXTRA_DATA_CS`) and the exe's recursive enter (lock, label) and
/// leave (lock) of it.
const EXTRA_DATA_CS: u32 = 0x011c_3920;
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
/// `ExtraJIP::key` (+0xC); an extra's type byte (+4) and next (+8); a list's first extra (+4).
const XJIP_KEY: u32 = 0x0c;
const EXTRA_TYPE: u32 = 0x04;
const EXTRA_NEXT: u32 = 0x08;
const LIST_FIRST: u32 = 0x04;
/// `BaseExtraList`'s presence bits (+8) and the bytes of its words xNVSE's inventory references
/// copy (`m_data` and the presence bits, 0x1C from +4).
const LIST_PRESENCE: u32 = 0x08;
const LIST_WORDS: usize = 0x1c;
/// `ContChangesEntry::extendData` (+0).
const ENTRY_LISTS: u32 = 0x00;
/// `ExtraDataList`'s vtable slot 0 (`00410380`, the deleting destructor: `(1)` frees it).
const LIST_DESTROY_SLOT: u32 = 0x00;
/// `MarkModified`'s flag JIP sets when it gives a reference its key.
const CHANGED_EXTRA_DATA: u32 = 0x400;
/// The values' and texts' indexes: 0 to 17 numbers or forms, 18 and 19 texts.
const VALUES: u32 = 18;
const LAST_INDEX: u32 = 19;

/// `XDATA_CS` around `f`.
fn with_extra_data_lock<R>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> R) -> R {
    e.call(LOCK_ENTER, &args![EXTRA_DATA_CS, 0u32]);
    let r = f(e);
    e.call(LOCK_LEAVE, &args![EXTRA_DATA_CS]);
    r
}

/// The extra data list JIP's data sits on: an inventory reference's stack (0 when it has none),
/// else the reference's own list (+0x44).
fn data_list(e: &mut Engine, refr: u32) -> u32 {
    match super::jip_fn_inventory::inv_ref_of(e, refr) {
        Some((_, data, _)) => data.xdata,
        None => refr + extra::REFR_EXTRA_LIST,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// GetExtraJIPData.
/// The mod's data of the reference (or its inventory stack): its list's `ExtraJIP`, the key's
/// entry, the mod's data; `None` at any step that finds nothing.
fn jip_data(e: &mut Engine, refr: u32, mod_index: u32) -> Option<ExtraJipData> {
    let list = data_list(e, refr);
    if list == 0 {
        return None;
    }
    let xjip = extra::get_by_type(e, list, gh1::EXTRA_JIP);
    if xjip == 0 {
        return None;
    }
    let key = e.mem.u32(xjip + XJIP_KEY);
    cosave::extra_data(e)
        .get(&Uint(key))
        .and_then(|entry| entry.data.get(&mod_index))
        .cloned()
}

/// A mod index argument: 0 the script's own (`GetOverridingModIdx`).
fn mod_or_script(e: &Engine, script: u32, mod_index: u32) -> u32 {
    if mod_index == 0 {
        overriding_mod(e, script)
    } else {
        mod_index
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_RefHasExtraData_Execute.
/// 1 when the reference (or its inventory stack) has data of the mod (0: the script's), below
/// 0xFF.
fn ref_has_extra_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let mod_index = mod_or_script(e, a.script, read.int(0, 0));
    if mod_index < 0xff {
        let found = with_extra_data_lock(e, |e| jip_data(e, a.this_obj, mod_index).is_some());
        if found {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

/// `PluginExpressionEvaluator` over the command's words: the evaluator made, `f` given its
/// arguments (`ExtractArgs`; `None` when they do not read), the evaluator closed and its expected
/// return type handed on (as xNVSE's evaluator in a plugin command).
fn with_eval(
    e: &mut Engine,
    a: &CommandArgs,
    f: impl FnOnce(&mut Engine, &mut Eval, Option<Vec<Tok>>),
) {
    let mut ev = Eval::new(e, a);
    nvse::expr::with_evaluator(e, |e| {
        let toks = ev.extract_args(e);
        f(e, &mut ev, toks);
    });
    nvse::expr::report(e, &ev);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionEvaluator::ExtractArgsV.
/// Whether `ExtractArgsV` writes the argument as a double (`GetNumber`): a number, a boolean, a
/// numeric variable or a global; a kind it does not name (an array element, a command's result)
/// when it converts to a number. A text, form, reference, lambda, array or slice is written as a
/// pointer.
fn writes_number(e: &mut Engine, ev: &Eval, t: &Tok) -> bool {
    match t.ty() {
        Ty::Number | Ty::Boolean | Ty::NumericVar | Ty::Global => true,
        Ty::String
        | Ty::StringVar
        | Ty::Form
        | Ty::Ref
        | Ty::Lambda
        | Ty::RefVar
        | Ty::Array
        | Ty::ArrayVar
        | Ty::Slice => false,
        _ => ev.can_convert(e, t, Ty::Number),
    }
}

/// `(int)value` as MSVC's `cvttsd2si` gives it, kept as JIP's `UInt32`.
fn as_index(value: f64) -> u32 {
    cvttsd2si(value) as u32
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetRefExtraData_Execute.
/// The value at the index (0-19) of the mod's data (0: the script's): 18 and 19 a text (the
/// result a string), a form id where its bit says so (a form), else the number.
fn get_ref_extra_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    with_eval(e, &a, |e, ev, toks| {
        // `ExtractArgsVA(nullptr, &fVarIdx, &fModIdx)`: the index and an optional mod index
        // (`fModIdx = 0`), each a double where xNVSE writes one (see the module's differences).
        let Some(toks) = toks.filter(|t| (1..=2).contains(&t.len())) else {
            return;
        };
        if !writes_number(e, ev, &toks[0]) {
            return;
        }
        let var = as_index(ev.get_number(e, &toks[0]));
        let module = match toks.get(1) {
            Some(t) if writes_number(e, ev, t) => ev.get_number(e, t),
            _ => 0.0,
        };
        if var > LAST_INDEX || a.this_obj == 0 {
            return;
        }
        let mod_index = mod_or_script(e, a.script, as_index(module));
        if mod_index >= 0xff {
            return;
        }
        let Some(data) = with_extra_data_lock(e, |e| jip_data(e, a.this_obj, mod_index)) else {
            return;
        };
        if var >= VALUES {
            let text = data.strings[(var - VALUES) as usize].clone();
            super::assign_string(e, &a, Some(text));
            expect_return_type(e, Returns::String);
        } else if data.forms & (1 << var) != 0 {
            a.set_form_result(e, data.values[var as usize]);
            expect_return_type(e, Returns::Form);
        } else {
            a.set_result(e, f64::from(f32::from_bits(data.values[var as usize])));
        }
    });
    handled(true)
}

/// What `SetRefExtraData` stores.
enum Stored {
    Number(f64),
    Form(u32),
    Text(Vec<u8>),
    Nothing,
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.h: ExtraJIPData::Set.
/// The value stored at `index`: a number as a float (used while not 0, its form bit cleared), a
/// form id (used and a form while not 0; else unused, the form bits kept only where used), or a
/// text.
fn store(data: &mut ExtraJipData, index: u32, value: Stored) {
    let mask = 1u32 << (index & 31);
    match value {
        Stored::Number(v) => {
            data.values[index as usize] = (v as f32).to_bits();
            if v != 0.0 {
                data.used |= mask;
            } else {
                data.used &= !mask;
            }
            data.forms &= !mask;
        }
        Stored::Form(id) => {
            data.values[index as usize] = id;
            if id != 0 {
                data.used |= mask;
                data.forms |= mask;
            } else {
                data.used &= !mask;
                data.forms &= data.used;
            }
        }
        Stored::Text(t) => data.strings[(index - VALUES) as usize] = jip_text(t),
        Stored::Nothing => {}
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: XString::operator=(const
// char*), StrLen.
/// The text as JIP's `XString` keeps it: up to its first zero byte (`StrLen`), its length cut to
/// 16 bits (`UInt16 length`); the bytes kept are that many (see the module's differences).
pub(crate) fn jip_text(mut t: Vec<u8>) -> Vec<u8> {
    let len = t.iter().position(|&c| c == 0).unwrap_or(t.len());
    t.truncate(len & 0xffff);
    t
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetRefExtraData_Execute.
/// The script's mod's data (never for a mod index 0xFF): with an index (0-19) and a value, the
/// value stored at the index (a form id where the value is a form, else a number; a text at 18
/// and 19), the reference (or its inventory stack) given an `ExtraJIP` and a key first when it
/// has none; with no argument, the mod's data erased, the `ExtraJIP` taken off when no mod has
/// data left, and an inventory stack left with nothing but its count taken out of the container
/// and freed.
fn set_ref_extra_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mod_index = overriding_mod(e, a.script);
    if mod_index == 0xff {
        return handled(true);
    }
    with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks else {
            return;
        };
        let (var, value) = match toks.first() {
            Some(index) => {
                let var = as_index(ev.get_number(e, index));
                let Some(value) = toks.get(1).filter(|_| var <= LAST_INDEX) else {
                    return;
                };
                let stored = if var < VALUES {
                    if ev.can_convert(e, value, Ty::Form) {
                        Stored::Form(ev.get_form_id(e, value))
                    } else {
                        Stored::Number(ev.get_number(e, value))
                    }
                } else if ev.can_convert(e, value, Ty::String) {
                    Stored::Text(ev.get_string(e, value))
                } else {
                    Stored::Nothing
                };
                (Some(var), stored)
            }
            None => (None, Stored::Nothing),
        };
        if a.this_obj == 0 {
            return;
        }
        with_extra_data_lock(e, |e| match var {
            Some(var) => set_value(e, a.this_obj, mod_index, var, value),
            None => erase(e, a.this_obj, mod_index),
        });
    });
    handled(true)
}

/// The `ExtraJIP` to store into: an inventory reference's stack's (a new stack, with a count
/// extra when the stack holds more than one, when it has none; `None` when none can be made),
/// else the reference's own; one added when there is none. With the reference to mark.
fn xjip_for_set(e: &mut Engine, refr: u32) -> Option<(u32, u32)> {
    if let Some((id, data, container)) = super::jip_fn_inventory::inv_ref_of(e, refr) {
        let before = list_words(e, data.xdata);
        let (xdata, xjip) = if data.xdata == 0 {
            let xdata =
                super::jip_fn_inventory::inv_ref_create_extra_data(e, id, data.item, container);
            if xdata == 0 {
                return None;
            }
            let count = nvse::invref::count(e, id);
            if count > 1 {
                gh1::add_extra_count(e, xdata, count);
            }
            (xdata, gh1::add_extra_jip(e, xdata, 0))
        } else {
            match extra::get_by_type(e, data.xdata, gh1::EXTRA_JIP) {
                0 => (data.xdata, gh1::add_extra_jip(e, data.xdata, 0)),
                x => return Some((x, container)),
            }
        };
        copy_again(e, xdata, &before);
        return Some((xjip, container));
    }
    let list = refr + extra::REFR_EXTRA_LIST;
    let xjip = match extra::get_by_type(e, list, gh1::EXTRA_JIP) {
        0 => gh1::add_extra_jip(e, list, 0),
        x => x,
    };
    Some((xjip, refr))
}

/// A list's words an inventory reference's temporary reference holds a copy of (`m_data` and
/// the presence bits); zeros for no list.
fn list_words(e: &Engine, list: u32) -> [u8; LIST_WORDS] {
    let mut words = [0u8; LIST_WORDS];
    if list != 0 {
        e.mem.read(list + LIST_FIRST, &mut words);
    }
    words
}

/// After the command added to the stack `xdata`: each inventory reference standing for it whose
/// temporary reference's copy of the words was `before` (the stack's before the addition) is
/// given the stack's words again (`SetData`), so a loop's write-back keeps what was added (see
/// the module's differences).
fn copy_again(e: &mut Engine, xdata: u32, before: &[u8; LIST_WORDS]) {
    for id in nvse::invref::refs_on_stack(e, xdata).0 {
        let Some((temp, data)) = nvse::invref::get(e, id).map(|r| (r.temp_ref, r.data)) else {
            continue;
        };
        if temp != 0 && list_words(e, temp + extra::REFR_EXTRA_LIST) == *before {
            nvse::invref::set_data(e, id, data);
        }
    }
}

/// After the stack `xdata`'s `ExtraJIP` `xjip` (followed by `next`) was taken off and freed: in
/// each inventory reference's copy of the stack's words, a chain still leading to it is unlinked
/// from it as `RemoveExtra` unlinked the stack's (the first extra, or the one before it, given
/// `next`; the walk bounded as xNVSE's lists), and its presence bit cleared.
fn unlink_in_copies(e: &mut Engine, xdata: u32, xjip: u32, next: u32) {
    for id in nvse::invref::refs_on_stack(e, xdata).0 {
        let temp = nvse::invref::get(e, id).map_or(0, |r| r.temp_ref);
        if temp == 0 {
            continue;
        }
        let list = temp + extra::REFR_EXTRA_LIST;
        let mut at = list + LIST_FIRST;
        let mut walked = 0usize;
        loop {
            let x = e.mem.u32(at);
            if x == 0 || !nvse::invref::list_step(e, &mut walked) {
                break;
            }
            if x == xjip {
                e.mem.set_u32(at, next);
                break;
            }
            at = x + EXTRA_NEXT;
        }
        let bits = list + LIST_PRESENCE + (gh1::EXTRA_JIP >> 3);
        let cleared = e.mem.u8(bits) & !(1u8 << (gh1::EXTRA_JIP & 7));
        e.mem.set_u8(bits, cleared);
    }
}

/// The value stored (see [`set_ref_extra_data`]).
fn set_value(e: &mut Engine, refr: u32, mod_index: u32, var: u32, value: Stored) {
    let Some((xjip, owner)) = xjip_for_set(e, refr) else {
        return;
    };
    let mut key = e.mem.u32(xjip + XJIP_KEY);
    if key == 0 {
        key = gh1::make_key(e);
        e.mem.set_u32(xjip + XJIP_KEY, key);
        extra::mark_modified(e, owner, CHANGED_EXTRA_DATA);
    }
    // One take of the map's lock (the key drawn above, outside it).
    {
        let mut map = cosave::extra_data(e);
        let data = map.emplace(Uint(key)).data.entry(mod_index).or_default();
        store(data, var, value);
    }
    cosave::mark_changed(e, changed::EXTRA_DATA);
}

/// The mod's data erased from the list's `ExtraJIP`; when no mod has data left there (the
/// entry's reference id cleared, the `ExtraJIP` taken off the list and freed), the `ExtraJIP`
/// taken off and the extra it was followed by.
fn erase_on(e: &mut Engine, list: u32, mod_index: u32) -> Option<(u32, u32)> {
    let xjip = extra::get_by_type(e, list, gh1::EXTRA_JIP);
    if xjip == 0 {
        return None;
    }
    let key = e.mem.u32(xjip + XJIP_KEY);
    // One take of the map's lock, let go before the list is changed (game code).
    let emptied = {
        let mut map = cosave::extra_data(e);
        let entry = map.get_mut(&Uint(key))?;
        entry.data.remove(&mod_index)?;
        let emptied = entry.data.is_empty();
        if emptied {
            entry.ref_id = 0;
        }
        emptied
    };
    cosave::mark_changed(e, changed::EXTRA_DATA);
    if !emptied {
        return None;
    }
    let next = e.mem.u32(xjip + EXTRA_NEXT);
    extra::remove_by_type(e, list, gh1::EXTRA_JIP);
    Some((xjip, next))
}

/// The mod's data erased (see [`set_ref_extra_data`]).
fn erase(e: &mut Engine, refr: u32, mod_index: u32) {
    let Some((_, data, container)) = super::jip_fn_inventory::inv_ref_of(e, refr) else {
        erase_on(e, refr + extra::REFR_EXTRA_LIST, mod_index);
        return;
    };
    let xdata = data.xdata;
    if xdata == 0 {
        return;
    }
    let Some((xjip, next)) = erase_on(e, xdata, mod_index) else {
        return;
    };
    unlink_in_copies(e, xdata, xjip, next);
    let first = e.mem.u32(xdata + LIST_FIRST);
    let only_count = first == 0
        || (u32::from(e.mem.u8(first + EXTRA_TYPE)) == gh1::EXTRA_COUNT
            && e.mem.u32(first + EXTRA_NEXT) == 0);
    if !only_count || container == 0 {
        return;
    }
    let entries = crate::overrides::changes_list(e, container);
    let entry = if entries == 0 {
        0
    } else {
        nvse::invref::find_for_item(e, entries, data.item)
    };
    if entry == 0 {
        return;
    }
    let lists = e.mem.u32(entry + ENTRY_LISTS);
    if lists == 0 {
        return;
    }
    // Not while a deferred action is to run on the stack, and only once it is out of the list.
    let (refs, deferred) = nvse::invref::refs_on_stack(e, xdata);
    if deferred || !super::list_remove(e, Ptr::new(lists), xdata) {
        return;
    }
    // Every inventory reference standing for the stack stands for none from now on.
    for id in refs {
        if let Some(data) = nvse::invref::get(e, id).map(|r| r.data) {
            nvse::invref::set_data(e, id, nvse::invref::Data { xdata: 0, ..data });
        }
    }
    e.vcall(xdata, LIST_DESTROY_SLOT, &args![1u32]);
}

#[cfg(test)]
#[path = "jip_fn_extra_data_tests.rs"]
mod tests;
