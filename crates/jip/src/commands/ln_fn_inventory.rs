//! `functions_ln/ln_fn_inventory.h` (JIP LN NVSE): the player's hotkeys (set, clear, save and
//! restore), and a base form's container items. All 8 of its commands.
//!
//! The player's inventory changes (`ExtraContainerChanges` 0x15, its data at +0xC, the entry list
//! `objList` at +0 of that): each entry with its extra data lists (`extendData`, a `tList*` at +0)
//! and its form (+8). A hotkey is an `ExtraHotkey` (type 0x4A, its index the byte at +0xC, made by
//! JIP's `Create` with the exe's vtable `0101592C`) on one of an armour's (0x18), weapon's (0x28)
//! or aid item's (0x2F) extra data lists; a list made for it is JIP's `ExtraDataList::Create`
//! (0x20 zeroed bytes with the vtable `010143E8`). The health (`ExtraHealth` 0x25, a float) and mod
//! flags (`ExtraWeaponModFlags` 0x8D, a byte) pick which list of a weapon or armour gets it.
//!
//! A base form's container (`TESForm::GetContainer`): a container's (type 0x1B) at +0x30, else
//! for a form whose vtable answers true at +0xF8 (an actor base) at +0x64; its items a `tList` at
//! +4 of 0xC-byte `{count, form, extra}`.

use super::arrays::{create_array, Element};
use super::extra::{self, kind};
use super::forms::ListNode;
use super::jip_fn_aux_variable::array_values;
use super::{heap_alloc, heap_free, is_actor, list_items, list_prepend, list_remove, list_unlink};
use super::{type_of, Handlers};
use crate::cosave::lookup_form;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::vars::Data;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2319, set_hotkey),
    (0x222c, clear_all_hotkeys),
    (0x2216, save_hotkeys),
    (0x2217, restore_hotkeys),
    (0x22f7, base_get_item_count),
    (0x22f8, base_add_item),
    (0x22fb, base_add_item_health),
    (0x22f9, base_remove_item),
];

const PLAYER: u32 = 0x011d_ea3c;
const BASE: u32 = 0x20;
const EXTRA_WORD: u32 = 0x0c;
const ENTRY_EXTEND: u32 = 0x0;
const ENTRY_TYPE: u32 = 0x8;
/// `kFormType_TESObjectARMO`, `kFormType_TESObjectWEAP`, `kFormType_AlchemyItem`,
/// `kFormType_TESObjectCONT`.
const ARMO: u8 = 0x18;
const WEAP: u8 = 0x28;
const ALCH: u8 = 0x2f;
const CONT: u8 = 0x1b;
const EXTRA_HOTKEY: u32 = 0x4a;
const EXTRA_HEALTH: u32 = 0x25;
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
const HOTKEY_VTBL: u32 = 0x0101_592c;
const EXTRA_DATA_LIST_VTBL: u32 = 0x0101_43e8;
const EXTRA_DATA_LIST_SIZE: u32 = 0x20;
/// An extra data list's first extra (`m_data`, +4).
const LIST_DATA: u32 = 0x04;
/// `ADDR_ReturnTrue` and the vtable slot +0xF8 `GetContainer` asks.
const RETURN_TRUE: u32 = 0x008d_0360;
/// `Actor::UnequipItem` (`0088C790`, `thiscall`: the actor, the form, 1, no list, remove the
/// enchantment, no lock, no message).
const UNEQUIP_ITEM: u32 = 0x0088_c790;

/// `HotkeyInfo` (JIP LN NVSE): a form, its health (-1 none, -2 any) and its mod flags (-1 any).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HotkeyInfo {
    pub form: u32,
    pub health: f32,
    pub mod_flags: i32,
}

impl Default for HotkeyInfo {
    fn default() -> Self {
        HotkeyInfo {
            form: 0,
            health: -1.0,
            mod_flags: 0,
        }
    }
}

/// `s_savedHotkeys[8]` (JIP LN NVSE): the hotkeys `SaveHotkeys` last saw, for the session.
#[derive(Default)]
pub struct SavedHotkeys(pub [HotkeyInfo; 8]);

fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}

/// The player's inventory entries (`GetContainerChangesList`'s nodes' items), empty for none.
fn player_entries(e: &mut Engine) -> Vec<u32> {
    let p = player(e);
    let list = match extra::refr_extra(e, p, kind::CONTAINER_CHANGES) {
        0 => 0,
        x => match e.mem.u32(x + EXTRA_WORD) {
            0 => 0,
            data => e.mem.u32(data),
        },
    };
    if list == 0 {
        Vec::new()
    } else {
        list_items(e, Ptr::<ListNode>::new(list))
    }
}

fn hotkey_type(e: &Engine, entry: u32) -> Option<u8> {
    let t = type_of(e, e.mem.u32(entry + ENTRY_TYPE));
    matches!(t, ARMO | WEAP | ALCH).then_some(t)
}

/// `ExtraHotkey::Create(index)` added to the list.
fn add_hotkey(e: &mut Engine, xdl: u32, index: u8) {
    let x = extra::create(e, HOTKEY_VTBL, EXTRA_HOTKEY);
    e.mem.set_u32(x + EXTRA_WORD, u32::from(index));
    extra::add_extra(e, xdl, x);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameBSExtraData.cpp: ExtraDataList::Create.
/// A new extra data list (0x20 zeroed bytes of the game's heap, its vtable) holding a new hotkey.
fn new_hotkey_list(e: &mut Engine, index: u8) -> u32 {
    let xdl = heap_alloc(e, EXTRA_DATA_LIST_SIZE);
    e.mem.write(xdl, &[0; EXTRA_DATA_LIST_SIZE as usize]);
    e.mem.set_u32(xdl, EXTRA_DATA_LIST_VTBL);
    add_hotkey(e, xdl, index);
    xdl
}

/// The hotkey's list emptied of its hotkey: removed from the entry's lists and freed when nothing
/// is left in it (`m_data` 0).
fn drop_hotkey(e: &mut Engine, entry: u32, xdl: u32) {
    extra::remove_by_type(e, xdl, EXTRA_HOTKEY);
    if e.mem.u32(xdl + LIST_DATA) == 0 {
        let lists = e.mem.u32(entry + ENTRY_EXTEND);
        list_remove(e, Ptr::new(lists), xdl);
        heap_free(e, xdl);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetHotkeyItemEntry,
// ClearHotkey.
/// The first list (in the player's armour, weapons and aid items) with a hotkey of the index
/// loses it.
fn clear_hotkey(e: &mut Engine, index: u8) {
    for entry in player_entries(e) {
        if entry == 0 || e.mem.u32(entry + ENTRY_EXTEND) == 0 || hotkey_type(e, entry).is_none() {
            continue;
        }
        let lists = e.mem.u32(entry + ENTRY_EXTEND);
        for xdl in list_items(e, Ptr::<ListNode>::new(lists)) {
            if xdl == 0 {
                continue;
            }
            let h = extra::get_by_type(e, xdl, EXTRA_HOTKEY);
            if h != 0 && e.mem.u8(h + EXTRA_WORD) == index {
                drop_hotkey(e, entry, xdl);
                return;
            }
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesExtraList::CleanEmpty.
/// Every empty list (`m_data` 0) freed and its node taken out.
fn clean_empty(e: &mut Engine, lists: u32) {
    let (mut node, mut prev) = (Ptr::<ListNode>::new(lists), Ptr::<ListNode>::NULL);
    while !node.is_null() {
        let xdl = e.get(node, ListNode::data);
        if xdl != 0 && e.mem.u32(xdl + LIST_DATA) == 0 {
            heap_free(e, xdl);
            if !prev.is_null() {
                list_unlink(e, node, prev);
                node = Ptr::new(e.get(prev, ListNode::next));
            } else {
                let had_next = e.get(node, ListNode::next) != 0;
                list_unlink(e, node, prev);
                if !had_next {
                    node = Ptr::NULL;
                }
            }
        } else {
            prev = node;
            node = Ptr::new(e.get(node, ListNode::next));
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h: SetHotkey.
/// No form: the index cleared. Else, in the player's armour, weapons and aid items: the form's
/// entry's matching list (health and mod flags as given; for aid items any) gets the hotkey (a
/// new list when the entry has none and no health or flags are asked), other lists of the entry
/// with the index lose it, empty lists go; another entry's first list loses a hotkey of the index.
pub fn set_hotkey_info(e: &mut Engine, index: u8, hk: HotkeyInfo) {
    if hk.form == 0 {
        clear_hotkey(e, index);
        return;
    }
    let plain = hk.health < 0.0 && hk.mod_flags <= 0;
    for entry in player_entries(e) {
        if entry == 0 {
            continue;
        }
        let Some(t) = hotkey_type(e, entry) else {
            continue;
        };
        let lists = e.mem.u32(entry + ENTRY_EXTEND);
        if e.mem.u32(entry + ENTRY_TYPE) == hk.form {
            if lists == 0 {
                if plain {
                    let xdl = new_hotkey_list(e, index);
                    let node = heap_alloc(e, ListNode::SIZE);
                    e.mem.set_u32(node, xdl);
                    e.mem.set_u32(node + 4, 0);
                    e.mem.set_u32(entry + ENTRY_EXTEND, node);
                }
                continue;
            }
            if e.mem.u32(lists) == 0 {
                if plain {
                    let xdl = new_hotkey_list(e, index);
                    list_prepend(e, Ptr::new(lists), xdl);
                }
                continue;
            }
            let mut found = false;
            for xdl in list_items(e, Ptr::<ListNode>::new(lists)) {
                if xdl == 0 {
                    continue;
                }
                let h = extra::get_by_type(e, xdl, EXTRA_HOTKEY);
                let mut matching = true;
                if t != ALCH {
                    if hk.health != -2.0 {
                        let x = extra::get_by_type(e, xdl, EXTRA_HEALTH);
                        matching = if x != 0 {
                            e.mem.u32(x + EXTRA_WORD) == hk.health.to_bits()
                        } else {
                            hk.health < 0.0
                        };
                    }
                    if matching && hk.mod_flags >= 0 {
                        let x = extra::get_by_type(e, xdl, EXTRA_WEAPON_MOD_FLAGS);
                        matching = if x != 0 {
                            i32::from(e.mem.u8(x + EXTRA_WORD)) == hk.mod_flags
                        } else {
                            hk.mod_flags == 0
                        };
                    }
                }
                if h != 0 {
                    if matching && !found {
                        found = true;
                        e.mem.set_u8(h + EXTRA_WORD, index);
                    } else if e.mem.u8(h + EXTRA_WORD) == index {
                        extra::remove_by_type(e, xdl, EXTRA_HOTKEY);
                    }
                } else if matching && !found {
                    found = true;
                    add_hotkey(e, xdl, index);
                }
            }
            clean_empty(e, lists);
        } else if lists != 0 {
            // JIP's loop here looks at the first list only (its `break` is outside the test).
            let xdl = e.mem.u32(lists);
            if xdl != 0 {
                let h = extra::get_by_type(e, xdl, EXTRA_HOTKEY);
                if h != 0 && e.mem.u8(h + EXTRA_WORD) == index {
                    extra::remove_by_type(e, xdl, EXTRA_HOTKEY);
                    if e.mem.u32(xdl + LIST_DATA) == 0 {
                        list_remove(e, Ptr::new(lists), xdl);
                        heap_free(e, xdl);
                    }
                }
            }
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_SetHotkey_Execute.
/// Hotkeys 1 to 8; the health defaults to -2 (any), the flags to -1 (any).
fn set_hotkey(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let index = read.int(0, 0);
    if read.ok && index != 0 && index <= 8 {
        let hk = HotkeyInfo {
            form: read.form(1),
            health: read.float(2, -2.0),
            mod_flags: read.int(3, u32::MAX) as i32,
        };
        set_hotkey_info(e, (index - 1) as u8, hk);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_ClearAllHotkeys_Execute.
/// In each armour, weapon and aid item entry: the first list with a hotkey loses it.
fn clear_all_hotkeys(e: &mut Engine, w: &[u32]) -> Ret {
    let _ = w;
    for entry in player_entries(e) {
        if entry == 0 || e.mem.u32(entry + ENTRY_EXTEND) == 0 || hotkey_type(e, entry).is_none() {
            continue;
        }
        let lists = e.mem.u32(entry + ENTRY_EXTEND);
        for xdl in list_items(e, Ptr::<ListNode>::new(lists)) {
            if xdl != 0 && extra::has_type(e, xdl, EXTRA_HOTKEY) {
                drop_hotkey(e, entry, xdl);
                break;
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_SaveHotkeys_Execute.
/// The eight hotkeys kept (each list with a hotkey: the item, and for armour and weapons its
/// health and mod flags), and given as an array of eight arrays `[form, health, flags]`.
/// A hotkey index of 8 or more (the game makes none) is passed over, where JIP writes past its
/// table.
fn save_hotkeys(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut saved = [HotkeyInfo::default(); 8];
    for entry in player_entries(e) {
        if entry == 0 || e.mem.u32(entry + ENTRY_EXTEND) == 0 {
            continue;
        }
        let Some(t) = hotkey_type(e, entry) else {
            continue;
        };
        let lists = e.mem.u32(entry + ENTRY_EXTEND);
        for xdl in list_items(e, Ptr::<ListNode>::new(lists)) {
            if xdl == 0 {
                continue;
            }
            let h = extra::get_by_type(e, xdl, EXTRA_HOTKEY);
            if h == 0 {
                continue;
            }
            let Some(slot) = saved.get_mut(usize::from(e.mem.u8(h + EXTRA_WORD))) else {
                continue;
            };
            slot.form = e.mem.u32(entry + ENTRY_TYPE);
            if t != ALCH {
                let x = extra::get_by_type(e, xdl, EXTRA_HEALTH);
                if x != 0 {
                    slot.health = e.mem.f32(x + EXTRA_WORD);
                }
                let x = extra::get_by_type(e, xdl, EXTRA_WEAPON_MOD_FLAGS);
                if x != 0 {
                    slot.mod_flags = i32::from(e.mem.u8(x + EXTRA_WORD));
                }
            }
        }
    }
    e.ext::<SavedHotkeys>().0 = saved;
    let mut outer = Vec::new();
    for hk in saved {
        let els = [
            Element::Form(hk.form),
            Element::Number(f64::from(hk.health)),
            Element::Number(f64::from(hk.mod_flags)),
        ];
        outer.push(Element::Array(create_array(e, a.script, &els)));
    }
    let id = create_array(e, a.script, &outer);
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

fn number(d: &Data) -> f64 {
    match d {
        Data::Number(n) => *n,
        _ => 0.0,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_RestoreHotkeys_Execute.
/// From an array of eight `[form, health, flags]` arrays (others left alone), or with no array
/// from the hotkeys last saved.
fn restore_hotkeys(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let id = read.int(0, 0);
    if id != 0 {
        let Some(vals) = array_values(e, id).filter(|v| v.len() == 8) else {
            return handled(true);
        };
        for (idx, v) in vals.iter().enumerate() {
            let Data::Array(inner) = v else {
                continue;
            };
            let Some(d) = array_values(e, *inner).filter(|d| d.len() == 3) else {
                continue;
            };
            let form = match d[0] {
                Data::Form(f) => lookup_form(e, f),
                _ => 0,
            };
            let hk = HotkeyInfo {
                form,
                health: number(&d[1]) as f32,
                mod_flags: super::cvttsd2si(number(&d[2])),
            };
            set_hotkey_info(e, idx as u8, hk);
        }
    } else {
        let saved = e.ext::<SavedHotkeys>().0;
        for (idx, hk) in saved.iter().enumerate() {
            set_hotkey_info(e, idx as u8, *hk);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetContainer (asm).
/// The form's container component, 0 for none.
pub fn container_of(e: &Engine, form: u32) -> u32 {
    if form == 0 {
        0
    } else if type_of(e, form) == CONT {
        form + 0x30
    } else if e.mem.u32(e.mem.u32(form) + 0xf8) == RETURN_TRUE {
        form + 0x64
    } else {
        0
    }
}

/// The base argument, else the calling reference's base form.
fn base_arg(e: &Engine, base: u32, this: u32) -> u32 {
    if base != 0 {
        base
    } else if this != 0 {
        e.mem.u32(this + BASE)
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_BaseGetItemCount_Execute.
/// The sum of the counts of the form's items in the base form's container (nothing without one).
fn base_get_item_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let form = read.form(0);
    let c = container_of(e, base_arg(e, read.form(1), a.this_obj));
    if c != 0 {
        let mut count = 0i32;
        for fc in list_items(e, Ptr::<ListNode>::new(c + 4)) {
            if fc != 0 && e.mem.u32(fc + 4) == form {
                count = count.wrapping_add(e.mem.u32(fc) as i32);
            }
        }
        a.set_result(e, f64::from(count));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h: DoBaseAddItem.
/// The count added to the form's first item; none there: a new item (0xC bytes) with a new
/// container extra (0xC bytes: no owner, no global, the health) put first.
fn do_base_add_item(e: &mut Engine, base: u32, this: u32, form: u32, count: u32, health: f32) {
    let c = container_of(e, base_arg(e, base, this));
    if c == 0 {
        return;
    }
    for fc in list_items(e, Ptr::<ListNode>::new(c + 4)) {
        if fc != 0 && e.mem.u32(fc + 4) == form {
            e.mem.set_u32(fc, e.mem.u32(fc).wrapping_add(count));
            return;
        }
    }
    let fc = heap_alloc(e, 0xc);
    let ce = heap_alloc(e, 0xc);
    e.mem.set_u32(fc + 8, ce);
    e.mem.set_u32(ce, 0);
    e.mem.set_u32(ce + 4, 0);
    e.mem.set_f32(ce + 8, health);
    e.mem.set_u32(fc + 4, form);
    e.mem.set_u32(fc, count);
    list_prepend(e, Ptr::new(c + 4), fc);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_BaseAddItem_Execute.
fn base_add_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let (form, count, base) = (read.form(0), read.int(1, 0), read.form(2));
        do_base_add_item(e, base, a.this_obj, form, count, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_BaseAddItemHealth_Execute.
fn base_add_item_health(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let (form, count, health, base) = (
            read.form(0),
            read.int(1, 0),
            read.float(2, 0.0),
            read.form(3),
        );
        do_base_add_item(e, base, a.this_obj, form, count, health);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_inventory.h:
// Cmd_BaseRemoveItem_Execute.
/// The count taken from the form's first item; below 1 the calling actor unequips the form and
/// the item's node is taken out (the item itself is not freed, as JIP's).
fn base_remove_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (form, count) = (read.form(0), read.int(1, 0));
    let c = container_of(e, base_arg(e, read.form(2), a.this_obj));
    if c == 0 {
        return handled(true);
    }
    let (mut node, mut prev) = (Ptr::<ListNode>::new(c + 4), Ptr::<ListNode>::NULL);
    loop {
        let fc = e.get(node, ListNode::data);
        if fc != 0 && e.mem.u32(fc + 4) == form {
            let left = (e.mem.u32(fc) as i32).wrapping_sub(count as i32);
            e.mem.set_u32(fc, left as u32);
            if left < 1 {
                if a.this_obj != 0 && is_actor(e, a.this_obj) {
                    e.call(
                        UNEQUIP_ITEM,
                        &args![a.this_obj, form, 1u32, 0u32, 1u32, 0u32, 1u32],
                    );
                }
                list_unlink(e, node, prev);
            }
            break;
        }
        prev = node;
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            break;
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_inventory_tests.rs"]
mod tests;
