//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): JIP's own extra data on a reference or an
//! inventory stack (`RefHasExtraData`, `GetRefExtraData`, `SetRefExtraData`), kept per mod: 18
//! numbers or form ids and 2 texts under a key the reference's `ExtraJIP` holds (group GH1's
//! framework, row G44: [`crate::game_patches::gh1`]; the co-save's `JPED` record,
//! [`crate::cosave::Saved::extra_data`]). The two last are expression-format commands: their
//! arguments are read by xNVSE's expression evaluator ([`nvse::expr::Eval`]) as JIP's
//! `PluginExpressionEvaluator` reads them.
//!
//! State: JIP's map of data (`s_extraDataKeysMap`) and its changed flags are reached through one
//! accessor, [`saved`]; when the hooks follow-up (#614's second part) moves the map out of JIP's
//! shared slot into a lock-free one, that function is the one line to change (with
//! `gh1::make_key`, which reaches the map itself).
//!
//! Threads and locks: the commands run on the main thread. As JIP's `XDATA_CS`, each takes the
//! game's extra data lock (`0x11C3920`, the exe's recursive enter `0x40FBF0` and leave `0x40FBA0`)
//! around its reads and writes, before it reaches JIP's shared slot, in the order group GH1's
//! seams use (the game holds the lock when they run and they take the slot).
//!
//! Deliberate differences (where JIP would read through null): no reference answers nothing; an
//! inventory reference whose container has no entry for the item when its stack is emptied keeps
//! the stack (as JIP); an entry without a list of stacks is left (JIP removes from the null list).

use super::extra;
use super::jip_fn_aux_variable::overriding_mod;
use super::{cvttsd2si, Handlers};
use crate::containers::Uint;
use crate::cosave::{changed, ExtraJipData, Saved};
use crate::game_patches::gh1;
use crate::Jip;
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
/// `ContChangesEntry::extendData` (+0).
const ENTRY_LISTS: u32 = 0x00;
/// `ExtraDataList`'s vtable slot 0 (`00410380`, the deleting destructor: `(1)` frees it).
const LIST_DESTROY_SLOT: u32 = 0x00;
/// `MarkModified`'s flag JIP sets when it gives a reference its key.
const CHANGED_EXTRA_DATA: u32 = 0x400;
/// The values' and texts' indexes: 0 to 17 numbers or forms, 18 and 19 texts.
const VALUES: u32 = 18;
const LAST_INDEX: u32 = 19;

/// JIP's saved data (the extra data map and the changed flags): the one accessor.
fn saved(e: &mut Engine) -> &mut Saved {
    &mut e.ext::<Jip>().saved
}

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
    saved(e)
        .extra_data
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
        // `ExtractArgsVA(nullptr, &fVarIdx, &fModIdx)`: one number and an optional one.
        let Some(toks) = toks.filter(|t| (1..=2).contains(&t.len())) else {
            return;
        };
        if !toks.iter().all(|t| ev.can_convert(e, t, Ty::Number)) {
            return;
        }
        let var = as_index(ev.get_number(e, &toks[0]));
        let module = toks.get(1).map_or(0.0, |t| ev.get_number(e, t));
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
        Stored::Text(t) => data.strings[(index - VALUES) as usize] = t,
        Stored::Nothing => {}
    }
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
        let xjip = if data.xdata == 0 {
            let xdata =
                super::jip_fn_inventory::inv_ref_create_extra_data(e, id, data.item, container);
            if xdata == 0 {
                return None;
            }
            let count = nvse::invref::count(e, id);
            if count > 1 {
                gh1::add_extra_count(e, xdata, count);
            }
            gh1::add_extra_jip(e, xdata, 0)
        } else {
            match extra::get_by_type(e, data.xdata, gh1::EXTRA_JIP) {
                0 => gh1::add_extra_jip(e, data.xdata, 0),
                x => x,
            }
        };
        return Some((xjip, container));
    }
    let list = refr + extra::REFR_EXTRA_LIST;
    let xjip = match extra::get_by_type(e, list, gh1::EXTRA_JIP) {
        0 => gh1::add_extra_jip(e, list, 0),
        x => x,
    };
    Some((xjip, refr))
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
    let s = saved(e);
    let data = s
        .extra_data
        .emplace(Uint(key))
        .data
        .entry(mod_index)
        .or_default();
    store(data, var, value);
    s.changed |= changed::EXTRA_DATA;
}

/// The mod's data erased from the list's `ExtraJIP`; true when no mod has data left there (the
/// entry's reference id cleared, the `ExtraJIP` taken off the list).
fn erase_on(e: &mut Engine, list: u32, mod_index: u32) -> bool {
    let xjip = extra::get_by_type(e, list, gh1::EXTRA_JIP);
    if xjip == 0 {
        return false;
    }
    let key = e.mem.u32(xjip + XJIP_KEY);
    let s = saved(e);
    let Some(entry) = s.extra_data.get_mut(&Uint(key)) else {
        return false;
    };
    if entry.data.remove(&mod_index).is_none() {
        return false;
    }
    let emptied = entry.data.is_empty();
    if emptied {
        entry.ref_id = 0;
    }
    s.changed |= changed::EXTRA_DATA;
    if emptied {
        extra::remove_by_type(e, list, gh1::EXTRA_JIP);
    }
    emptied
}

/// The mod's data erased (see [`set_ref_extra_data`]).
fn erase(e: &mut Engine, refr: u32, mod_index: u32) {
    let Some((_, data, container)) = super::jip_fn_inventory::inv_ref_of(e, refr) else {
        erase_on(e, refr + extra::REFR_EXTRA_LIST, mod_index);
        return;
    };
    let xdata = data.xdata;
    if xdata == 0 || !erase_on(e, xdata, mod_index) {
        return;
    }
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
    if lists != 0 {
        super::list_remove(e, Ptr::new(lists), xdata);
        e.vcall(xdata, LIST_DESTROY_SLOT, &args![1u32]);
    }
}

#[cfg(test)]
#[path = "jip_fn_extra_data_tests.rs"]
mod tests;
