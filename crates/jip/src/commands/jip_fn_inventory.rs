//! `functions_jip/jip_fn_inventory.h` (JIP LN NVSE): an item's value and uniqueness, a container's
//! base items, an actor's items (also into a form list), the equipped weapon's poison, and an
//! inventory reference's weapon mod flags and "cannot unequip" mark. 9 of its 29 commands.
//!
//! Built elsewhere: `ToggleItemUnique` (world_hooks.rs) and `SetOnUseAidItemEventHandler`
//! (actor_hooks.rs), the core track's hook commands.
//!
//! Not built yet (no hook or patch needed; left for the next agent of the lane, each with what
//! it needs): `AddItemAlt`, `RemoveItemTarget`, `DropAlt`, `DropMeAlt`, `RemoveMeIRAlt` (JIP's
//! `TESObjectREFR::AddItemAlt` / `RemoveItemTarget` and the game's `RemoveItem` virtual, and the
//! inventory reference's `removed` mark), `EquipItemAlt`, `UnequipItemAlt` (JIP's
//! `Actor::EquipItemAlt`, the game's `UnequipItem` and `ShowItemMessage`),
//! `SetWeaponRefModFlags`, `SetItemRefCurrentHealth`, `SetHotkeyItemRef` (the inventory reference's
//! `CreateExtraData` and `SplitFromStack`), `GetItemRefCurrentHealth` (`ContChangesEntry::
//! GetBaseHealth`), `GetArmorEffectiveDT`/`DR` (the game's `004BE0B0`/`004BDF90` on a temporary
//! entry), `GetAllItemRefs`, `GetEquippedArmorRefs`, `GetEquippedItemRef`, `GetHotkeyItemRef`,
//! `GetSelfAsInventoryRef` (inventory references made through the built-in xNVSE,
//! `nvse::invref::create_entry`, and JIP's `GetEquippedItemRef` / `GetHotkeyItemEntry`).
//!
//! Layouts confirmed in the exe (AUDIT-lane-B.md): an extra data list's presence bits and walk
//! (commands/extra.rs); `ExtraWeaponModFlags` 0x8D (its flags byte at +0xC: the constructor
//! `0042E450`), `ExtraCannotWear` 0x3E (`00432350`, vtable `01015BF4`), `ExtraPoison` 0x3F (the
//! poison at +0xC: `0042C900`), `ExtraWorn` 0x16; a changes entry's extra lists +0, count +4,
//! item +8; an actor's process +0x68, its level +0x28 and weapon entry +0x114 (equip_hooks.rs);
//! an alchemy item's value +0xB8 (`ENIT`'s first word); a value component's value +4; a form's
//! flags +8 (0x400 quest item), JIP's flags byte +6 (world_hooks.rs); the playable bits of
//! armour (+0x78, 0x40), weapon (+0x100, 0x80) and ammunition (+0xAC, 2).

use super::arrays::{create_array, Element};
use super::forms::INVENTORY_TYPE;
use super::{extra, heap_alloc, list_items, list_prepend, list_remove_all, type_of, Handlers};
use crate::game::ListNode;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2607, get_value_alt),
    (0x26ab, set_value_alt),
    (0x270c, get_weapon_ref_mod_flags),
    (0x2766, get_all_items),
    (0x2864, get_no_unequip),
    (0x2865, set_no_unequip),
    (0x288f, get_equipped_weapon_poison),
    (0x28a7, get_base_items),
    (0x2934, is_item_unique),
];

/// `kFormType_*` (JIP LN NVSE).
const ARMOR: u8 = 0x18;
const WEAPON: u8 = 0x28;
const AMMO: u8 = 0x29;
const ALCHEMY_ITEM: u8 = 0x2f;
const CREATURE_REF: u8 = 0x3c;
const LIST_FORM: u8 = 0x55;

/// `kVtbl_TESObjectARMO` (`IS_TYPE(item, TESObjectARMO)` compares the vtable).
const VTBL_ARMOR: u32 = 0x0102_a62c;

/// `RTTI_TESForm`, `RTTI_TESValueForm` (`DYNAMIC_CAST(form, TESForm, TESValueForm)`).
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_VALUE_FORM: u32 = 0x0118_6b6c;

const FORM_FLAGS: u32 = 0x08;
const QUEST_ITEM: u32 = 0x400;
const REFR_BASE: u32 = 0x20;
const REFR_EXTRA: u32 = 0x44;
const VALUE_FORM_VALUE: u32 = 0x04;
const ALCH_VALUE: u32 = 0xb8;
const ACTOR_PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const PROCESS_WEAPON_INFO: u32 = 0x114;
const ENTRY_ITEM: u32 = 0x08;
const LIST_FORM_LIST: u32 = 0x18;
/// `BGSListForm::numAddedObjects`.
const LIST_ADDED_COUNT: u32 = 0x20;
const EXTRA_WORD: u32 = 0x0c;

/// Extra data types and the one this file creates.
const EXTRA_WORN: u32 = 0x16;
const EXTRA_CANNOT_WEAR: u32 = 0x3e;
const EXTRA_POISON: u32 = 0x3f;
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
const VTBL_EXTRA_CANNOT_WEAR: u32 = 0x0101_5bf4;
/// `sizeof(ExtraCannotWear)` (a vtable, the type word, the next pointer).
const EXTRA_CANNOT_WEAR_SIZE: u32 = 0x0c;

fn is_inventory_type(kind: u32) -> bool {
    // JIP indexes its 121-entry table with any type: one past it is no inventory type here.
    INVENTORY_TYPE.get(kind as usize).copied().unwrap_or(false)
}

fn base_of(e: &Engine, reference: u32) -> u32 {
    if reference == 0 {
        0
    } else {
        e.mem.u32(reference + REFR_BASE)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetItemValue.
/// An alchemy item's own value, else its value component's (0 without one).
fn item_value(e: &mut Engine, form: u32) -> u32 {
    if type_of(e, form) == ALCHEMY_ITEM {
        return e.mem.u32(form + ALCH_VALUE);
    }
    let value = nvse::commands::dynamic_cast(e, form, RTTI_TES_FORM, RTTI_TES_VALUE_FORM);
    if value == 0 {
        0
    } else {
        e.mem.u32(value + VALUE_FORM_VALUE)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetValueAlt_Execute.
/// The form's value (as `int`), else the calling reference's base form's when that is an
/// inventory item.
fn get_value_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 {
        let base = base_of(e, a.this_obj);
        if base != 0 && is_inventory_type(u32::from(type_of(e, base))) {
            form = base;
        }
    }
    if form != 0 {
        let value = item_value(e, form) as i32;
        a.set_result(e, f64::from(value));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_SetValueAlt_Execute.
/// The value component's value set; else an alchemy item's own; another form, nothing.
fn set_value_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 {
        return handled(true);
    }
    let value = read.int(1, 0);
    let component = nvse::commands::dynamic_cast(e, form, RTTI_TES_FORM, RTTI_TES_VALUE_FORM);
    if component != 0 {
        e.mem.set_u32(component + VALUE_FORM_VALUE, value);
    } else if type_of(e, form) == ALCHEMY_ITEM {
        e.mem.set_u32(form + ALCH_VALUE, value);
    }
    handled(true)
}

/// `InventoryRefGetForID(thisObj->refID)`: the inventory reference's extra data list, `None`
/// when the reference is no inventory reference (`Some(0)` for one without a list).
fn inv_ref_xdata(e: &mut Engine, reference: u32) -> Option<u32> {
    if reference == 0 {
        return None;
    }
    let id = super::ref_id(e, reference);
    if !nvse::invref::for_ref_id(e, id) {
        return None;
    }
    nvse::invref::get(e, id).map(|r| r.data.xdata)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetWeaponRefModFlags_Execute.
/// The mod flags of the inventory reference's stack, else of the reference's own extra data.
fn get_weapon_ref_mod_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let xdata = inv_ref_xdata(e, a.this_obj).unwrap_or(a.this_obj + REFR_EXTRA);
    if xdata != 0 {
        let x = extra::get_by_type(e, xdata, EXTRA_WEAPON_MOD_FLAGS);
        if x != 0 {
            let flags = e.mem.u8(x + EXTRA_WORD);
            a.set_result(e, f64::from(flags));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetNoUnequip_Execute.
/// 1 when the inventory reference's stack is marked "cannot wear" (`ExtraCannotWear`).
fn get_no_unequip(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let xdata = inv_ref_xdata(e, a.this_obj).unwrap_or(0);
    if xdata != 0 && extra::has_type(e, xdata, EXTRA_CANNOT_WEAR) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_SetNoUnequip_Execute.
/// 0 takes the mark off the inventory reference's stack; another value puts it on a worn stack
/// that lacks it (a new `ExtraCannotWear`).
fn set_no_unequip(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let xdata = inv_ref_xdata(e, a.this_obj).unwrap_or(0);
    if xdata == 0 {
        return handled(true);
    }
    if read.int(0, 0) == 0 {
        extra::remove_by_type(e, xdata, EXTRA_CANNOT_WEAR);
    } else if extra::has_type(e, xdata, EXTRA_WORN) && !extra::has_type(e, xdata, EXTRA_CANNOT_WEAR)
    {
        let x = heap_alloc(e, EXTRA_CANNOT_WEAR_SIZE);
        e.mem.set_u32(x, VTBL_EXTRA_CANNOT_WEAR);
        e.mem.set_u32(x + 4, EXTRA_CANNOT_WEAR);
        e.mem.set_u32(x + 8, 0);
        extra::add_extra(e, xdata, x);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetWeaponInfo.
/// The changes entry of the weapon the actor holds: its process at the middle-high level or
/// above (level at most 1), its weapon entry; 0 otherwise.
fn weapon_info(e: &Engine, actor: u32) -> u32 {
    let process = e.mem.u32(actor + ACTOR_PROCESS);
    if process == 0 || e.mem.u8(process + PROCESS_LEVEL) > 1 {
        return 0;
    }
    e.mem.u32(process + PROCESS_WEAPON_INFO)
}

/// `ContChangesEntry::extendData->GetFirstItem()`: the entry's first extra list, 0 without.
fn first_extra_list(e: &Engine, entry: u32) -> u32 {
    let lists = e.mem.u32(entry);
    if lists == 0 {
        0
    } else {
        e.mem.u32(lists)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetEquippedWeaponPoison_Execute.
/// The poison on the actor's held weapon (its first stack's `ExtraPoison`).
fn get_equipped_weapon_poison(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let entry = weapon_info(e, a.this_obj);
    if entry == 0 {
        return handled(true);
    }
    let xdata = first_extra_list(e, entry);
    if xdata != 0 {
        let x = extra::get_by_type(e, xdata, EXTRA_POISON);
        let poison = if x == 0 { 0 } else { e.mem.u32(x + EXTRA_WORD) };
        if poison != 0 {
            super::set_form_result(e, &a, poison);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetBaseItems_Execute.
/// An array of the forms of the base container's entries (the calling reference's base form when
/// none is given), an entry with no form as 0; no array for no container or no entries.
fn get_base_items(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let base = match read.form(0) {
        0 => base_of(e, a.this_obj),
        f => f,
    };
    if base == 0 {
        return handled(true);
    }
    let container = super::ln_fn_inventory::container_of(e, base);
    if container == 0 {
        return handled(true);
    }
    let items: Vec<Element> = list_items(e, Ptr::<ListNode>::new(container + 4))
        .into_iter()
        .filter(|&count| count != 0)
        .map(|count| Element::Form(e.mem.u32(count + 4)))
        .collect();
    if !items.is_empty() {
        let id = create_array(e, a.script, &items);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::IsItemPlayable.
/// Armour, weapons and ammunition by their not-playable bits; any other item is playable.
fn is_item_playable(e: &Engine, item: u32) -> bool {
    match type_of(e, item) {
        ARMOR => e.mem.u8(item + 0x78) & 0x40 == 0,
        WEAPON => e.mem.u8(item + 0x100) & 0x80 == 0,
        AMMO => e.mem.u8(item + 0xac) & 2 == 0,
        _ => true,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::IsItemEquipped.
/// A weapon: the one the actor holds; an armour (by vtable), on an actor that is no creature: a
/// worn stack of its changes entry; anything else, no.
fn is_item_equipped(e: &mut Engine, actor: u32, item: u32) -> bool {
    if type_of(e, item) == WEAPON {
        let entry = weapon_info(e, actor);
        return entry != 0 && e.mem.u32(entry + ENTRY_ITEM) == item;
    }
    if !super::is_type(e, item, VTBL_ARMOR) || type_of(e, actor) == CREATURE_REF {
        return false;
    }
    let changes = crate::overrides::changes_list(e, actor);
    let entry = if changes == 0 {
        0
    } else {
        nvse::invref::find_for_item(e, changes, item)
    };
    if entry == 0 {
        return false;
    }
    let lists = e.mem.u32(entry);
    lists != 0
        && list_items(e, Ptr::new(lists))
            .into_iter()
            .any(|xdata| xdata != 0 && extra::has_type(e, xdata, EXTRA_WORN))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetAllItems_Execute.
/// The reference's items (of one form type, 0 for all), in the order of their addresses (JIP's
/// sorted map), less the unplayable, quest items or (an actor's) equipped ones when asked: an
/// array of them, or with a form list the list emptied and filled (each put first) and the count
/// as the result. Nothing for a type that is no inventory item or an empty inventory.
fn get_all_items(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let kind = read.int(0, 0);
    let (no_unplayable, no_quest) = (read.int(1, 0) != 0, read.int(2, 0) != 0);
    let mut no_equipped = read.int(3, 0) != 0;
    let list = read.form(4);
    if kind != 0 && !is_inventory_type(kind) {
        return handled(true);
    }
    let mut items = crate::equip_hooks::Items::new();
    if !crate::equip_hooks::inventory_items(e, a.this_obj, kind as u8, &mut items) {
        return handled(true);
    }
    if !super::is_actor(e, a.this_obj) {
        no_equipped = false;
    }
    // The list argument is a form list by its parameter type; another form is left alone.
    let list = if list != 0 && type_of(e, list) == LIST_FORM {
        list
    } else {
        0
    };
    if list != 0 {
        // BGSListForm::RemoveAll: the nodes, and the count of script-added forms (the exe's
        // `00590010` counts them at +0x20; the save `00590080` and the revert `0058FFD0` walk
        // that many nodes).
        list_remove_all(e, Ptr::new(list + LIST_FORM_LIST));
        e.mem.set_u32(list + LIST_ADDED_COUNT, 0);
    }
    let mut kept = Vec::new();
    let mut count = 0u32;
    for &item in items.keys() {
        if (no_unplayable && !is_item_playable(e, item))
            || (no_quest && e.mem.u32(item + FORM_FLAGS) & QUEST_ITEM != 0)
            || (no_equipped && is_item_equipped(e, a.this_obj, item))
        {
            continue;
        }
        if list != 0 {
            list_prepend(e, Ptr::new(list + LIST_FORM_LIST), item);
        } else {
            kept.push(Element::Form(item));
        }
        count += 1;
    }
    if count != 0 {
        if list != 0 {
            a.set_result(e, f64::from(count));
        } else {
            let id = create_array(e, a.script, &kept);
            a.set_result(e, f64::from(id));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_IsItemUnique_Execute.
/// 1 when the item is marked unique (`ToggleItemUnique`'s flag in JIP's flags byte).
fn is_item_unique(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let item = read.form(0);
    if read.ok
        && item != 0
        && crate::world_hooks::flags6(e, item) & crate::world_hooks::UNIQUE_ITEM != 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_inventory_tests.rs"]
mod tests;
