//! `functions_ln/ln_fn_leveled_list.h` (JIP LN NVSE): a leveled list's entries, chance none, its
//! global and flags, a console dump and a deep search through nested lists. All 21 of its
//! registered commands (`SetChanceNoneGlobal` is defined in the file but JIP's table has no
//! record for it). The list is read as any form; `GetLvlList` answers the component at +0x30 for
//! a leveled creature (0x2C), character (0x2D) or item (0x34) and nothing for any other form.
//!
//! The component (`forms::TESLeveledList`) is checked in the leveled item loader and the game's
//! own `AddItem`; JIP builds its entries itself: an entry (0xC bytes) and its extra (0xC bytes:
//! owner, global, health) from the game's heap. JIP's list edits leave entries allocated
//! (`RemoveNth`, `RemoveAll`) or free the entry but not its extra (`RemoveItem`); the port does
//! the same, as the entries' memory is the game's.
//!
//! One change from JIP: `AddItem` puts the new entry before the first entry whose level is at
//! least the new one; when every entry is lower, JIP's `tList::Insert` at index == count finds no
//! node and inserts nothing, leaving the two blocks it allocated. The port inserts nothing too
//! (scripts see JIP's answer) but frees those two blocks.

use super::forms::{
    form_type_b7, ContainerExtra, LeveledListData, ListNode, TESLeveledList, LEVELED_LIST_COMPONENT,
};
use super::{
    heap_alloc, heap_free, list_count, list_items, list_remove_all, nth_item, ref_id,
    set_form_result, the_name, type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x221e, leveled_list_add_form),
    (0x221f, leveled_list_remove_form),
    (0x2220, get_lev_item_index_by_form),
    (0x2221, get_lev_item_index_by_level),
    (0x2222, get_chance_none),
    (0x2223, set_chance_none),
    (0x2224, get_num_lev_items),
    (0x2225, get_nth_lev_item),
    (0x2226, get_nth_lev_item_level),
    (0x2227, get_nth_lev_item_count),
    (0x2228, remove_nth_lev_item),
    (0x2229, leveled_list_clear),
    (0x222a, dump_lev_list),
    (0x222b, get_chance_none_global),
    (0x228e, get_leveled_list_flags),
    (0x228f, set_leveled_list_flags),
    (0x22e8, leveled_list_replace_form),
    (0x2311, set_nth_lev_item),
    (0x2312, set_nth_lev_item_level),
    (0x2313, set_nth_lev_item_count),
    (0x28da, leveled_list_has_form_deep),
];

type L = TESLeveledList;
type D = LeveledListData;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetLvlList (asm).
/// The form's leveled list (+0x30 of a leveled creature, character or item), `None` otherwise
/// (and for no form).
fn lvl_list(e: &Engine, form: u32) -> Option<Ptr<L>> {
    if form == 0 {
        return None;
    }
    matches!(
        type_of(e, form),
        form_type_b7::TES_LEV_CREATURE
            | form_type_b7::TES_LEV_CHARACTER
            | form_type_b7::TES_LEV_ITEM
    )
    .then(|| Ptr::new(form + LEVELED_LIST_COMPONENT))
}

/// Argument 0's leveled list when the read succeeded.
fn list_arg(e: &Engine, read: &Read) -> Option<Ptr<L>> {
    if read.ok {
        lvl_list(e, read.form(0))
    } else {
        None
    }
}

fn entries(l: Ptr<L>) -> Ptr<ListNode> {
    l.at(L::list)
}

fn data(d: u32) -> Ptr<D> {
    Ptr::new(d)
}

/// `list.GetNthItem(index)`: entry `index` (a negative index none), 0 past the end.
fn nth(e: &Engine, l: Ptr<L>, index: u32) -> u32 {
    nth_item(e, entries(l), index as i32)
}

// ---- entries ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_LeveledListAddForm_Execute and nvse/GameForms.cpp: TESLeveledList::AddItem.
/// A new entry for the form (level and count as 16 bits; health below 0 taken as 0, above 1 as
/// a percentage, `* 0.01F`) with a new extra (no owner, no global, the health), put before the
/// first entry that is empty or whose level is at least the new one (`tList::Insert`: at index
/// 0 into an empty first node or before the first item; later, into that node, its item moved to
/// a new node after it). With no such entry nothing is inserted (JIP's answer); the port frees
/// the entry and extra JIP would leave allocated.
fn leveled_list_add_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(l) = list_arg(e, &read) else {
        return handled(true);
    };
    let (form, level, count) = (read.form(1), read.int(2, 0) as u16, read.int(3, 0) as u16);
    let mut health = read.float(4, 0.0);
    if health < 0.0 {
        health = 0.0;
    } else if health > 1.0 {
        health *= 0.01;
    }
    let items = list_items(e, entries(l));
    let index = items
        .iter()
        .position(|&d| d == 0 || e.get(data(d), D::level) as u16 >= level)
        .unwrap_or(items.len());
    let entry = data(heap_alloc(e, D::SIZE));
    let extra = Ptr::<ContainerExtra>::new(heap_alloc(e, ContainerExtra::SIZE));
    e.set(extra, ContainerExtra::ownerFaction, 0);
    e.set(extra, ContainerExtra::globalVar, 0);
    e.set(extra, ContainerExtra::health, health);
    e.set(entry, D::form, form);
    e.set(entry, D::level, level as i16);
    e.set(entry, D::count, count as i16);
    e.set(entry, D::extra, extra.addr());
    if index == items.len() {
        // `GetNthNode(count)` is null: JIP inserts nothing and leaks both blocks.
        heap_free(e, entry.addr());
        heap_free(e, extra.addr());
        return handled(true);
    }
    // The node `index` takes the new entry; its own item moves to a new node after it (an
    // empty first node just takes it).
    let mut node = entries(l);
    for _ in 0..index {
        node = Ptr::new(e.get(node, ListNode::next));
    }
    let old = e.get(node, ListNode::data);
    if index == 0 && old == 0 {
        e.set(node, ListNode::data, entry.addr());
        return handled(true);
    }
    let added = Ptr::<ListNode>::new(heap_alloc(e, ListNode::SIZE));
    let next = e.get(node, ListNode::next);
    e.set(added, ListNode::data, old);
    e.set(node, ListNode::data, entry.addr());
    e.set(added, ListNode::next, next);
    e.set(node, ListNode::next, added.addr());
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESLeveledList::RemoveItem.
/// Every entry of the form: the entry freed (not its extra) and its node taken out (a later node
/// freed, the walk going on from the next one; the first node takes the next one's item and is
/// looked at again); the count of entries removed, as an `int`.
fn leveled_list_remove_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(l) = list_arg(e, &read) else {
        return handled(true);
    };
    let form = read.form(1);
    let mut removed = 0u32;
    let (mut node, mut prev) = (entries(l), Ptr::<ListNode>::NULL);
    while !node.is_null() {
        let d = e.get(node, ListNode::data);
        if d != 0 && e.get(data(d), D::form) == form {
            heap_free(e, d);
            if prev.is_null() {
                // `RemoveMe` on the first node: the next node's item and link move up.
                let next = e.get(node, ListNode::next);
                if next == 0 {
                    e.set(node, ListNode::data, 0);
                } else {
                    let nn = Ptr::<ListNode>::new(next);
                    let (item, after) = (e.get(nn, ListNode::data), e.get(nn, ListNode::next));
                    e.set(node, ListNode::data, item);
                    e.set(node, ListNode::next, after);
                    heap_free(e, next);
                }
            } else {
                // `iter = prev->RemoveNext()`: the node freed, the walk goes on after it.
                let after = e.get(node, ListNode::next);
                e.set(prev, ListNode::next, after);
                heap_free(e, node.addr());
                node = Ptr::new(after);
            }
            removed += 1;
        } else {
            prev = node;
            node = Ptr::new(e.get(node, ListNode::next));
        }
    }
    a.set_result(e, f64::from(removed as i32));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_LeveledListReplaceForm_Execute.
/// Every entry of the old form gets the new one.
fn leveled_list_replace_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let (old, new) = (read.form(1), read.form(2));
        for d in list_items(e, entries(l)) {
            if d != 0 && e.get(data(d), D::form) == old {
                e.set(data(d), D::form, new);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_GetNumLevItems_Execute.
fn get_num_lev_items(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let n = list_count(e, entries(l));
        a.set_result(e, f64::from(n as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_GetNthLevItem_Execute.
/// Entry `index`'s form, when it has one.
fn get_nth_lev_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let d = nth(e, l, read.int(1, 0));
        if d != 0 {
            let form = e.get(data(d), D::form);
            if form != 0 {
                set_form_result(e, &a, form);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_SetNthLevItem_Execute.
fn set_nth_lev_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let d = nth(e, l, read.int(1, 0));
        if d != 0 {
            e.set(data(d), D::form, read.form(2));
        }
    }
    handled(true)
}

/// Entry `index`'s 16-bit field as a signed number.
fn get_nth_field(e: &mut Engine, w: &[u32], field: Field<D, i16>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let d = nth(e, l, read.int(1, 0));
        if d != 0 {
            let v = e.get(data(d), field);
            a.set_result(e, f64::from(v));
        }
    }
    handled(true)
}

/// Entry `index`'s 16-bit field set to the integer's low 16 bits.
fn set_nth_field(e: &mut Engine, w: &[u32], field: Field<D, i16>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let d = nth(e, l, read.int(1, 0));
        if d != 0 {
            e.set(data(d), field, read.int(2, 0) as i16);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_GetNthLevItemLevel_Execute, Cmd_SetNthLevItemLevel_Execute, Cmd_GetNthLevItemCount_Execute
// and Cmd_SetNthLevItemCount_Execute.
fn get_nth_lev_item_level(e: &mut Engine, w: &[u32]) -> Ret {
    get_nth_field(e, w, D::level)
}
fn set_nth_lev_item_level(e: &mut Engine, w: &[u32]) -> Ret {
    set_nth_field(e, w, D::level)
}
fn get_nth_lev_item_count(e: &mut Engine, w: &[u32]) -> Ret {
    get_nth_field(e, w, D::count)
}
fn set_nth_lev_item_count(e: &mut Engine, w: &[u32]) -> Ret {
    set_nth_field(e, w, D::count)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.h: tList::RemoveNth.
/// `RemoveNth(index)` with the index as a signed number: 0 or below takes the first node out
/// (`RemoveMe`); a later one is unlinked and freed when the list reaches it. The entry itself is
/// left allocated, as JIP leaves it.
fn remove_nth_lev_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(l) = list_arg(e, &read) else {
        return handled(true);
    };
    let head = entries(l);
    let mut idx = read.int(1, 0) as i32;
    if idx <= 0 {
        super::list_unlink(e, head, Ptr::NULL);
        return handled(true);
    }
    let mut node = head;
    loop {
        let next = e.get(node, ListNode::next);
        if next == 0 {
            break;
        }
        idx -= 1;
        if idx == 0 {
            break;
        }
        node = Ptr::new(next);
    }
    if idx == 0 {
        super::list_unlink(e, Ptr::new(e.get(node, ListNode::next)), node);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_LeveledListClear_Execute.
/// `RemoveAll`: the later nodes freed, the first emptied; the entries left allocated.
fn leveled_list_clear(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        list_remove_all(e, entries(l));
    }
    handled(true)
}

/// The index of the first entry that `matches`, -1 for none or no list (written always).
fn index_of(e: &mut Engine, w: &[u32], matches: impl Fn(&Engine, u32, &Read) -> bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut index = -1;
    if let Some(l) = list_arg(e, &read) {
        if let Some(i) = list_items(e, entries(l))
            .into_iter()
            .position(|d| d != 0 && matches(e, d, &read))
        {
            index = i as i32;
        }
    }
    a.set_result(e, f64::from(index));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESLeveledList::GetItemIndexByLevel.
/// The level compared as the 16-bit level made an `int` against the `UInt32` argument.
fn get_lev_item_index_by_level(e: &mut Engine, w: &[u32]) -> Ret {
    index_of(e, w, |e, d, read| {
        i32::from(e.get(data(d), D::level)) as u32 == read.int(1, 0)
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESLeveledList::GetItemIndexByForm.
fn get_lev_item_index_by_form(e: &mut Engine, w: &[u32]) -> Ret {
    index_of(e, w, |e, d, read| e.get(data(d), D::form) == read.form(1))
}

// ---- the list's own fields ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_GetChanceNone_Execute.
/// The chance none byte; -1 for no list (written always).
fn get_chance_none(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let v = match list_arg(e, &read) {
        Some(l) => i32::from(e.get(l, L::chanceNone)),
        None => -1,
    };
    a.set_result(e, f64::from(v));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_SetChanceNone_Execute.
/// 0-100 into the byte; anything above, nothing.
fn set_chance_none(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let chance = read.int(1, 0);
    if let Some(l) = list_arg(e, &read).filter(|_| chance <= 100) {
        e.set(l, L::chanceNone, chance as u8);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_GetChanceNoneGlobal_Execute.
fn get_chance_none_global(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let global = e.get(l, L::global);
        if global != 0 {
            set_form_result(e, &a, global);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_GetLeveledListFlags_Execute.
fn get_leveled_list_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        let flags = e.get(l, L::flags);
        a.set_result(e, f64::from(flags));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_SetLeveledListFlags_Execute.
/// The integer's low byte.
fn set_leveled_list_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        e.set(l, L::flags, read.int(1, 0) as u8);
    }
    handled(true)
}

// ---- nested lists ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: LeveledListHasFormDeep.
/// Some entry holds the form, or a nested list not seen before on this search (`TempFormList`,
/// cleared for each call) holds it. JIP reads a null entry form's type through null; the port
/// takes it as no nested list.
fn has_form_deep(e: &Engine, l: Ptr<L>, form: u32, seen: &mut Vec<u32>) -> bool {
    for d in list_items(e, entries(l)) {
        if d == 0 {
            continue;
        }
        let f = e.get(data(d), D::form);
        if f == form {
            return true;
        }
        if let Some(inner) = lvl_list(e, f) {
            if let Err(at) = seen.binary_search(&f) {
                seen.insert(at, f);
                if has_form_deep(e, inner, form, seen) {
                    return true;
                }
            }
        }
    }
    false
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_LeveledListHasFormDeep_Execute.
fn leveled_list_has_form_deep(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = list_arg(e, &read) {
        if has_form_deep(e, l, read.form(1), &mut Vec::new()) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// ---- the dump ----

/// `kDumpLvlListIndentStr` (JIP LN NVSE): 60 spaces, printed from `s_dumpLvlListIndent` on.
const INDENT: usize = 60;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESLeveledList::Dump.
/// One console line per entry (empty entries skipped): the indent, "(LeveledList)" for a nested
/// list or the form's name, its id, level, count and its extra's health (0 without one); a
/// nested list's entries follow, three spaces further in. JIP's indent runs out after 20 levels
/// (its pointer then reads before the spaces); the port prints no indent there, and a list that
/// holds itself recurses until the stack ends, as in JIP.
fn dump(e: &mut Engine, l: Ptr<L>, indent: usize) {
    for d in list_items(e, entries(l)) {
        if d == 0 {
            continue;
        }
        let form = e.get(data(d), D::form);
        if form == 0 {
            // JIP reads a null form's type through null; the port skips the entry.
            continue;
        }
        let inner = lvl_list(e, form);
        let name = match inner {
            Some(_) => b"(LeveledList)".to_vec(),
            None => the_name(e, form),
        };
        let extra = e.get(data(d), D::extra);
        let health = if extra == 0 {
            0.0
        } else {
            f64::from(e.get(Ptr::<ContainerExtra>::new(extra), ContainerExtra::health))
        };
        let pad = " ".repeat(indent.min(INDENT));
        let text = nvse::table::guest_str(e, &[pad.as_bytes(), &name].concat());
        let [lo, hi] = nvse::commands::double_words(health);
        let (level, count) = (
            i32::from(e.get(data(d), D::level)) as u32,
            i32::from(e.get(data(d), D::count)) as u32,
        );
        let id = ref_id(e, form);
        nvse::commands::console_print(
            e,
            "%s [%08X] Level: %d Count: %d Health: %.2f",
            &[text, id, level, count, lo, hi],
        );
        if let Some(inner) = inner {
            dump(e, inner, indent + 3);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_leveled_list.h:
// Cmd_DumpLevList_Execute.
/// With the console open (`IsConsoleOpen`), "Dumping LeveledList [id]" and the list's lines.
fn dump_lev_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || !nvse::commands::is_console_mode(e) {
        return handled(true);
    }
    let form = read.form(0);
    if let Some(l) = lvl_list(e, form) {
        let id = ref_id(e, form);
        nvse::commands::console_print(e, "Dumping LeveledList [%08X]", &[id]);
        dump(e, l, 0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_leveled_list_tests.rs"]
mod tests;
