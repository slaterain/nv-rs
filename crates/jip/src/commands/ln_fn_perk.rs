//! `functions_ln/ln_fn_perk.h` (JIP LN NVSE): a perk's entries, trait flag and level, and an
//! actor's rank in a perk. All 17 of its commands.
//!
//! `SetPerkRank` is asm in JIP: it builds the game's own `Cmd_AddPerk_Execute` frame (`005D4D90`)
//! and jumps into its body (`0x5D4E42`, or `0x5D4EBC` when the rank does not change) so that a hook
//! on that body (JohnnyGuitar NVSE's OnAddPerk) sees it. The port runs what that body does from
//! there, read in the exe (see `set_perk_rank`).
//!
//! The perk loader (`005EADA0`) adds each entry through `005EB410` to the list at +0x48 (a
//! `tList`, its first node inline). Entries are told apart by their vtables (JIP's `IS_TYPE`): a
//! quest entry (`01046B84`: the quest +8, the stage byte +0xC, its loader `005E9B00` reading `DATA`
//! there), an ability entry (`01046C44`: the spell +8, `005E9E20`), an entry point entry
//! (`01046D0C`: the entry point byte +8 (`DATA`, 3 bytes, loader `005EA790`) and the function data
//! +0xC). The function data are told apart by vtable too: one value (`010462C0`: a float +4), two
//! values (`01046300`: floats +4, +8), a leveled list (`01046320`: the list +4), an activation
//! choice (`01046340`: its label, a game string at +4); their fields as JIP's header gives them.
//!
//! Casts: `entry->entryPoint = func` and `perk->data.minLevel = level` store the script's `UInt32`
//! into a byte (its low byte); `stage = value` (a float into a byte: `cvttss2si`, its low byte).

use super::forms::{
    form_type_more, BGSEntryPointPerkEntry, BGSQuestPerkEntry, EntryPointData, ListNode,
};
use super::{
    assign_string, buffer_arg, cvttss2si, is_type, list_count, nth_item, set_form_result,
    string_set, string_text, type_of, Handlers,
};
use crate::game::BGSPerk;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x221b, has_perk_rank),
    (0x223b, set_perk_rank),
    (0x2267, get_perk_entry_count),
    (0x2268, get_nth_perk_entry_type),
    (0x2269, get_nth_perk_entry_function),
    (0x226a, set_nth_perk_entry_function),
    (0x226b, get_nth_perk_entry_form),
    (0x226c, set_nth_perk_entry_form),
    (0x226d, get_nth_perk_entry_value1),
    (0x226e, set_nth_perk_entry_value1),
    (0x226f, get_nth_perk_entry_value2),
    (0x2270, set_nth_perk_entry_value2),
    (0x2271, get_nth_perk_entry_string),
    (0x2272, set_nth_perk_entry_string),
    (0x229a, is_trait),
    (0x230c, get_perk_level),
    (0x28ae, set_perk_level),
];

/// The entry and function data vtables (`kVtbl_*` (JIP LN NVSE)).
const QUEST_ENTRY: u32 = 0x0104_6b84;
const ABILITY_ENTRY: u32 = 0x0104_6c44;
const ENTRY_POINT_ENTRY: u32 = 0x0104_6d0c;
const ONE_VALUE: u32 = 0x0104_62c0;
const TWO_VALUE: u32 = 0x0104_6300;
const LEVELED_LIST: u32 = 0x0104_6320;
const ACTIVATE_CHOICE: u32 = 0x0104_6340;
/// `Actor::GetPerkRank` (the actor's virtual at +0x4A0: the perk, whether for teammates).
const GET_PERK_RANK: u32 = 0x4a0;
/// `Actor::SetPerkRank` (the actor's virtual at +0x498: the perk, the rank, whether for
/// teammates), as the game's `AddPerk` body calls it at `005D4E5B`.
const SET_PERK_RANK: u32 = 0x498;
/// The game's console print (`00703C00`, `cdecl`) and `AddPerk`'s text (`0103C5A4`, "Added Perk
/// %s to %s with rank %i"); the perk's name through its `TESFullName` (+0x18, `00408DA0`) and the
/// actor's (`0055D520`).
const CONSOLE_PRINT: u32 = 0x0070_3c00;
const ADDED_PERK_TEXT: u32 = 0x0103_c5a4;
const FULL_NAME_TEXT: u32 = 0x0040_8da0;
const ACTOR_NAME: u32 = 0x0055_d520;
/// The running thread's console-line flag (`TLS + 0x268`), as `005D4E73` reads it.
const TLS_CONSOLE_LINE: u32 = 0x268;
/// JIP's label buffer (`char labelStr[0x80]`).
const LABEL_BUFFER: usize = 0x80;

fn perk_entries(perk: Ptr<BGSPerk>) -> Ptr<ListNode> {
    perk.at(BGSPerk::entries)
}

/// Entry `index` of the perk (the script's `UInt32` taken as `GetNthItem`'s signed index).
fn nth_entry(e: &Engine, perk: u32, index: u32) -> u32 {
    nth_item(e, perk_entries(Ptr::new(perk)), index as i32)
}

/// The function data of an entry point entry.
fn entry_data(e: &Engine, entry: u32) -> u32 {
    e.get(
        Ptr::<BGSEntryPointPerkEntry>::new(entry),
        BGSEntryPointPerkEntry::data,
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_HasPerkRank_Execute.
/// The calling actor's rank in the perk (the actor's own `GetPerkRank`), echoed to the console.
fn has_perk_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if super::is_actor(e, a.this_obj) {
        let read = Read::of(e, &a);
        let perk = read.form(0);
        if read.ok && type_of(e, perk) == form_type_more::BGS_PERK {
            let alt = u32::from(read.int(1, 0) != 0);
            let rank = e.vcall(a.this_obj, GET_PERK_RANK, &[perk, alt]).u32() as u8;
            a.set_result(e, f64::from(rank));
        }
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h: Cmd_SetPerkRank_Execute
// (asm), with the game's Cmd_AddPerk_Execute body (005D4E42-005D4ECB) it jumps into.
/// For a calling actor (vtable +0x100 true) and a perk (type 0x56): the rank (0: nothing) capped
/// at the perk's number of ranks (unsigned); the actor's current rank asked (`GetPerkRank`, the
/// alt argument passed as the whole word JIP read). The same rank (its byte): the result is the
/// rank less one (JIP's frame holds `rank - 1` where the game keeps the current rank). Otherwise,
/// as the game's body: `SetPerkRank` with the rank and the alt argument's low byte, the console
/// line "Added Perk ... with rank ..." on a console thread, and the rank as the result.
fn set_perk_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let actor = a.this_obj;
    if !super::is_actor(e, actor) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let perk = read.form(0);
    if !read.ok || type_of(e, perk) != form_type_more::BGS_PERK {
        return handled(true);
    }
    let mut rank = read.int(1, 0);
    if rank == 0 {
        return handled(true);
    }
    rank = rank.min(u32::from(
        e.get(Ptr::<BGSPerk>::new(perk), BGSPerk::cNumRanks),
    ));
    // `[ebp-0xD] = rank - 1` (a byte), `[ebp-5] = useAlt`'s low byte.
    let less_one = (rank as u8).wrapping_sub(1);
    let alt = read.int(2, 0);
    let current = e.vcall(actor, GET_PERK_RANK, &[perk, alt]).u32() as u8;
    if current == rank as u8 {
        a.set_result(e, f64::from(less_one));
        return handled(true);
    }
    let new_rank = u32::from(less_one) + 1;
    e.vcall(actor, SET_PERK_RANK, &[perk, new_rank, alt & 0xff]);
    let tls = e.tls();
    if tls != 0 && e.mem.u8(tls + TLS_CONSOLE_LINE) != 0 {
        let actor_name = e.call(ACTOR_NAME, &args![actor]).u32();
        let perk_name = e.call(FULL_NAME_TEXT, &args![perk + 0x18]).u32();
        e.call(
            CONSOLE_PRINT,
            &args![ADDED_PERK_TEXT, perk_name, actor_name, new_rank],
        );
    }
    a.set_result(e, f64::from(new_rank));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetPerkEntryCount_Execute.
fn get_perk_entry_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let count = list_count(e, perk_entries(Ptr::new(read.form(0))));
        a.set_result(e, f64::from(count as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetNthPerkEntryType_Execute.
/// An entry point entry: 1 one value, 2 two values, 3 a leveled list, 4 an activation choice
/// (other function data: nothing); a quest entry 6; any other entry 5.
fn get_nth_perk_entry_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let entry = nth_entry(e, read.form(0), read.int(1, 0));
    if entry == 0 {
        return handled(true);
    }
    let kind = if is_type(e, entry, ENTRY_POINT_ENTRY) {
        let data = entry_data(e, entry);
        match e.mem.u32(data) {
            ONE_VALUE => Some(1.0),
            TWO_VALUE => Some(2.0),
            LEVELED_LIST => Some(3.0),
            ACTIVATE_CHOICE => Some(4.0),
            _ => None,
        }
    } else if is_type(e, entry, QUEST_ENTRY) {
        Some(6.0)
    } else {
        Some(5.0)
    };
    if let Some(k) = kind {
        a.set_result(e, k);
    }
    handled(true)
}

/// Entry `index` when it is an entry point entry.
fn entry_point_arg(e: &Engine, read: &Read) -> Option<Ptr<BGSEntryPointPerkEntry>> {
    if !read.ok {
        return None;
    }
    let entry = nth_entry(e, read.form(0), read.int(1, 0));
    (entry != 0 && is_type(e, entry, ENTRY_POINT_ENTRY)).then(|| Ptr::new(entry))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetNthPerkEntryFunction_Execute.
/// The entry point; -1 otherwise (written first).
fn get_nth_perk_entry_function(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if let Some(entry) = entry_point_arg(e, &read) {
        let v = e.get(entry, BGSEntryPointPerkEntry::entryPoint);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_SetNthPerkEntryFunction_Execute.
fn set_nth_perk_entry_function(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(entry) = entry_point_arg(e, &read) {
        e.set(
            entry,
            BGSEntryPointPerkEntry::entryPoint,
            read.int(2, 0) as u8,
        );
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetNthPerkEntryForm_Execute.
/// An entry point entry's leveled list (leveled list data only), a quest entry's quest, an
/// ability entry's spell.
fn get_nth_perk_entry_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let entry = nth_entry(e, read.form(0), read.int(1, 0));
    if entry == 0 {
        return handled(true);
    }
    let form = match e.mem.u32(entry) {
        ENTRY_POINT_ENTRY => {
            let data = entry_data(e, entry);
            if data != 0 && is_type(e, data, LEVELED_LIST) {
                e.get(Ptr::<EntryPointData>::new(data), EntryPointData::word)
            } else {
                0
            }
        }
        QUEST_ENTRY | ABILITY_ENTRY => e.get(
            Ptr::<BGSQuestPerkEntry>::new(entry),
            BGSQuestPerkEntry::form,
        ),
        _ => 0,
    };
    if form != 0 {
        set_form_result(e, &a, form);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_SetNthPerkEntryForm_Execute.
/// A quest entry's quest (to a quest) or an ability entry's spell (to a spell).
fn set_nth_perk_entry_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (perk, value) = (read.form(0), read.form(2));
    if !read.ok || type_of(e, perk) != form_type_more::BGS_PERK {
        return handled(true);
    }
    let entry = nth_entry(e, perk, read.int(1, 0));
    if entry == 0 {
        return handled(true);
    }
    let e_ptr = Ptr::<BGSQuestPerkEntry>::new(entry);
    if is_type(e, entry, QUEST_ENTRY) {
        if type_of(e, value) == form_type_more::TES_QUEST {
            e.set(e_ptr, BGSQuestPerkEntry::form, value);
        }
    } else if is_type(e, entry, ABILITY_ENTRY) && type_of(e, value) == form_type_more::SPELL_ITEM {
        e.set(e_ptr, BGSQuestPerkEntry::form, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetNthPerkEntryValue1_Execute.
/// An entry point entry's first value (one or two value data), a quest entry's stage; -1
/// otherwise (written first).
fn get_nth_perk_entry_value1(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let entry = nth_entry(e, read.form(0), read.int(1, 0));
    if entry == 0 {
        return handled(true);
    }
    if is_type(e, entry, ENTRY_POINT_ENTRY) {
        let data = entry_data(e, entry);
        if is_type(e, data, ONE_VALUE) || is_type(e, data, TWO_VALUE) {
            let v = e.get(Ptr::<EntryPointData>::new(data), EntryPointData::value);
            a.set_result(e, f64::from(v));
        }
    } else if is_type(e, entry, QUEST_ENTRY) {
        let v = e.get(
            Ptr::<BGSQuestPerkEntry>::new(entry),
            BGSQuestPerkEntry::stage,
        );
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_SetNthPerkEntryValue1_Execute.
fn set_nth_perk_entry_value1(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let perk = read.form(0);
    if !read.ok || type_of(e, perk) != form_type_more::BGS_PERK {
        return handled(true);
    }
    let entry = nth_entry(e, perk, read.int(1, 0));
    if entry == 0 {
        return handled(true);
    }
    let value = read.float(2, 0.0);
    if is_type(e, entry, ENTRY_POINT_ENTRY) {
        let data = entry_data(e, entry);
        if is_type(e, data, ONE_VALUE) || is_type(e, data, TWO_VALUE) {
            e.set(
                Ptr::<EntryPointData>::new(data),
                EntryPointData::value,
                value,
            );
        }
    } else if is_type(e, entry, QUEST_ENTRY) {
        e.set(
            Ptr::<BGSQuestPerkEntry>::new(entry),
            BGSQuestPerkEntry::stage,
            cvttss2si(value) as u8,
        );
    }
    handled(true)
}

/// Entry `index`'s function data when the entry is an entry point entry and the data has the
/// vtable `vtbl`.
fn data_of(e: &Engine, read: &Read, vtbl: u32) -> Option<Ptr<EntryPointData>> {
    let entry = entry_point_arg(e, read)?;
    let data = entry_data(e, entry.addr());
    is_type(e, data, vtbl).then(|| Ptr::new(data))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetNthPerkEntryValue2_Execute.
fn get_nth_perk_entry_value2(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if let Some(d) = data_of(e, &read, TWO_VALUE) {
        let v = e.get(d, EntryPointData::value2);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_SetNthPerkEntryValue2_Execute.
fn set_nth_perk_entry_value2(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && type_of(e, read.form(0)) == form_type_more::BGS_PERK {
        if let Some(d) = data_of(e, &read, TWO_VALUE) {
            e.set(d, EntryPointData::value2, read.float(2, 0.0));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetNthPerkEntryString_Execute.
fn get_nth_perk_entry_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = data_of(e, &read, ACTIVATE_CHOICE)
        .and_then(|d| string_text(e, d.at(EntryPointData::label).addr()));
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_SetNthPerkEntryString_Execute.
fn set_nth_perk_entry_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let label = buffer_arg(e, &read, 2, LABEL_BUFFER, "SetNthPerkEntryString");
    if type_of(e, read.form(0)) == form_type_more::BGS_PERK {
        if let Some(d) = data_of(e, &read, ACTIVATE_CHOICE) {
            string_set(e, d.at(EntryPointData::label).addr(), &label);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h: Cmd_IsTrait_Execute.
fn is_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && e.get(Ptr::<BGSPerk>::new(read.form(0)), BGSPerk::isTrait) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_GetPerkLevel_Execute.
fn get_perk_level(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let v = e.get(Ptr::<BGSPerk>::new(read.form(0)), BGSPerk::minLevel);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_perk.h:
// Cmd_SetPerkLevel_Execute.
fn set_perk_level(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        e.set(
            Ptr::<BGSPerk>::new(read.form(0)),
            BGSPerk::minLevel,
            read.int(1, 0) as u8,
        );
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_perk_tests.rs"]
mod tests;
