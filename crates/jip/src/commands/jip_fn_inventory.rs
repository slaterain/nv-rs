//! `functions_jip/jip_fn_inventory.h` (JIP LN NVSE): an item's value and uniqueness, a container's
//! base items, an actor's items (also into a form list), the equipped weapon's poison, and an
//! inventory reference's weapon mod flags and "cannot unequip" mark; inventory references for
//! an actor's stacks, worn items and hotkeys and for a scripted item itself (made through the
//! built-in xNVSE, `nvse::invref::create_entry`, as JIP's `InventoryRefCreate`), an item
//! reference's health, an armour's effective DT and DR (the game's `004BE0B0` / `004BDF90`), and
//! items moved to another reference; items added (JIP's `AddItemAlt` through a temporary
//! container), equipped (`EquipItemAlt`) and unequipped with JIP's corner message, and dropped
//! or taken by count or by inventory reference (`DropAlt`, `DropMeAlt`, `RemoveMeIRAlt`, the
//! last two marking the reference removed through `nvse::invref::set_removed`); an inventory
//! reference's mod flags, health and hotkey set (`SetWeaponRefModFlags`,
//! `SetItemRefCurrentHealth`, `SetHotkeyItemRef`, through JIP's own `InventoryRef::CreateExtraData`
//! and `SplitFromStack`, translated here). All 27 of its 29 commands not built elsewhere.
//!
//! Built elsewhere: `ToggleItemUnique` (world_hooks.rs) and `SetOnUseAidItemEventHandler`
//! (actor_hooks.rs), the core track's hook commands.
//!
//! Calls confirmed in the exe for lane B4 (AUDIT-lane-B4.md): `ExtraDataList::CopyListForReference`
//! `00412490` (`RET 8`: this, the source, the byte), the vtables of `ExtraDataList` `010143E8`
//! (0x20 bytes), `ExtraHealth` `010158E4` (`00431D10`), `ExtraHotkey` `0101592C` (`00432380`),
//! `ExtraWeaponModFlags` `010159A4` (`0042E450`), each value at +0xC; a changes entry 0xC bytes.
//!
//! Calls confirmed in the exe for lane B3 (AUDIT-lane-B3.md; the words passed are the callee's):
//! the temporary `TESContainer`'s constructor `00481610` and destructor `00481680`, its `AddItem`
//! `004818E0` (`RET 0xC`), `TESLeveledList`'s add `00487F70` (`RET 0x10`), the move into a
//! reference `004821A0` (translated: container, reference, byte), `GetCalcLevel` `00567E10`
//! (`RET 4`), `Actor::EquipItem` / `UnequipItem` `0088C650` / `0088C790` (`RET 0x18`),
//! `QueueUIMessage` `007052F0` (six words; it copies the text), the icon path at `010208E0`, a
//! `ContainerObject`'s `ContainerItemExtra` (0xC bytes, health +8: `00482090`).
//!
//! Layouts confirmed in the exe for these (AUDIT-lane-B2.md): `TESHealthForm` at armour +0x68 and
//! weapon +0x94 (`GetFormAsHealthForm` `004872E0`, the health its +4), a weapon's mod effects
//! +0x180 and values +0x18C (`TESObjectWEAP::GetModEffectValue` `004BCF60`), effect 10 the
//! health mod (`ItemChange::GetItemHealth` `004BCDB0`), `ExtraHealth` 0x25 (+0xC, `004186F0`),
//! the reference's 3D (`pLoadedData` +0x64, its +0x14: game.rs), the biped data (virtual
//! +0x1E8, slots +0x2C of 0x10: equip_hooks.rs), an armour's part mask +0x74 (`BMDT`),
//! `RemoveItem` virtual +0x17C (nvse invref.rs), `ExtraHotkey` 0x4A (ln_fn_inventory.rs),
//! `ExtraScript` 0x0D with its locals at +0x10 (nvse inventory_info.rs).
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
use super::{
    extra, heap_alloc, heap_free, list_items, list_prepend, list_remove_all, type_of, Handlers,
};
use crate::game::ListNode;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2606, add_item_alt_cmd),
    (0x2607, get_value_alt),
    (0x26ab, set_value_alt),
    (0x2688, remove_item_target_cmd),
    (0x270c, get_weapon_ref_mod_flags),
    (0x270d, set_weapon_ref_mod_flags),
    (0x270f, get_item_ref_current_health),
    (0x2710, set_item_ref_current_health),
    (0x2716, set_hotkey_item_ref),
    (0x2717, equip_item_alt_cmd),
    (0x2718, unequip_item_alt),
    (0x2719, drop_alt),
    (0x271a, drop_me_alt),
    (0x2766, get_all_items),
    (0x2767, get_all_item_refs),
    (0x27e2, remove_me_ir_alt),
    (0x2839, get_equipped_item_ref),
    (0x2864, get_no_unequip),
    (0x2865, set_no_unequip),
    (0x288f, get_equipped_weapon_poison),
    (0x28a7, get_base_items),
    (0x28d1, get_equipped_armor_refs),
    (0x28e5, get_armor_effective_dt),
    (0x28e6, get_armor_effective_dr),
    (0x28f1, get_hotkey_item_ref),
    (0x2934, is_item_unique),
    (0x2936, get_self_as_inventory_ref),
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
const EXTRA_SCRIPT: u32 = 0x0d;
const EXTRA_HEALTH: u32 = 0x25;
const EXTRA_HOTKEY: u32 = 0x4a;
/// `ExtraScript::eventList`.
const EXTRA_SCRIPT_EVENTS: u32 = 0x10;

const ENTRY_COUNT: u32 = 0x04;
/// `pLoadedData` and its 3D root.
const REFR_LOADED_DATA: u32 = 0x64;
const LOADED_3D: u32 = 0x14;
/// `GetBipedAnim` (virtual), `RemoveItem` (virtual).
const VT_GET_BIPED_ANIM: u32 = 0x1e8;
const VT_REMOVE_ITEM: u32 = 0x17c;
/// `BipedAnim::slotData[20]`, 0x10 bytes each, the item first.
const BIPED_SLOTS: u32 = 0x2c;
const BIPED_SLOT_SIZE: u32 = 0x10;
const BIPED_SLOT_COUNT: u32 = 20;
/// `kSlot_Weapon`.
const WEAPON_SLOT: u32 = 5;
const ARMOR_PART_MASK: u32 = 0x74;
/// The health of `TESHealthForm` (armour +0x68, weapon +0x94) at its +4.
const ARMOR_HEALTH: u32 = 0x6c;
const WEAPON_HEALTH: u32 = 0x98;
const WEAPON_MOD_EFFECTS: u32 = 0x180;
const WEAPON_MOD_VALUES: u32 = 0x18c;
/// `kWeaponModEffect_IncreaseHealth`.
const MOD_EFFECT_HEALTH: u32 = 10;
/// The game's effective DT and DR of a changes entry (`thiscall`, the entry and one more word, which
/// JIP passes as 0: `RET 4`, and the translations take `(Ptr<ItemChange>, u32)`; a float on the FPU
/// stack).
const ARMOR_EFFECTIVE_DT: u32 = 0x004b_e0b0;
const ARMOR_EFFECTIVE_DR: u32 = 0x004b_df90;
/// How deep `RemoveItemTarget` follows form lists inside form lists (JIP: without a bound).
const MAX_LIST_DEPTH: u32 = 16;

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

// ---- inventory references (lane B2) ----

/// `ContChangesEntry::GetEquippedExtra`: the entry's first extra list marked worn, 0 without.
fn equipped_extra(e: &mut Engine, entry: u32) -> u32 {
    let lists = e.mem.u32(entry);
    if lists == 0 {
        return 0;
    }
    list_items(e, Ptr::new(lists))
        .into_iter()
        .find(|&xdata| xdata != 0 && extra::has_type(e, xdata, EXTRA_WORN))
        .unwrap_or(0)
}

/// `TESObjectREFR::GetContainerChangesEntry(item)`: the reference's changes entry for the item.
fn changes_entry(e: &mut Engine, reference: u32, item: u32) -> u32 {
    let changes = crate::overrides::changes_list(e, reference);
    if changes == 0 {
        0
    } else {
        nvse::invref::find_for_item(e, changes, item)
    }
}

/// `InventoryRefCreate` (JIP takes xNVSE's `CreateInventoryRefEntry`): a temporary reference for
/// `count` of `item` in `container` with the stack `xdata`, on a changes entry of its own.
fn inv_ref_create(e: &mut Engine, container: u32, item: u32, count: i32, xdata: u32) -> u32 {
    nvse::invref::create_entry(e, container, item, count, xdata)
}

/// The inventory reference's entry and stack (`invRef->entry`, `invRef->xData`), `None` for a
/// reference that is no inventory reference.
fn inv_ref_data(e: &mut Engine, reference: u32) -> Option<nvse::invref::Data> {
    if reference == 0 {
        return None;
    }
    let id = super::ref_id(e, reference);
    if !nvse::invref::for_ref_id(e, id) {
        return None;
    }
    nvse::invref::get(e, id).map(|r| r.data)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetAllItemRefs_Execute.
/// As `GetAllItems`, with an inventory reference per stack: each extra list with a count (at most
/// what is left of the item's count; a worn one passed over when asked, its count still taken),
/// then one for what is left, without extra data.
fn get_all_item_refs(e: &mut Engine, w: &[u32]) -> Ret {
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
    let list = if list != 0 && type_of(e, list) == LIST_FORM {
        list
    } else {
        0
    };
    if list != 0 {
        list_remove_all(e, Ptr::new(list + LIST_FORM_LIST));
        e.mem.set_u32(list + LIST_ADDED_COUNT, 0);
    }
    let mut refs = Vec::new();
    for (&item, &(total, entry)) in items.iter() {
        if (no_unplayable && !is_item_playable(e, item))
            || (no_quest && e.mem.u32(item + FORM_FLAGS) & QUEST_ITEM != 0)
        {
            continue;
        }
        let mut base_count = total;
        let lists = if entry == 0 { 0 } else { e.mem.u32(entry) };
        if lists != 0 {
            for (i, xdata) in list_items(e, Ptr::new(lists)).into_iter().enumerate() {
                // `do { ... } while (baseCount && (xdlIter = xdlIter->next))`.
                if i > 0 && base_count == 0 {
                    break;
                }
                if xdata == 0 {
                    continue;
                }
                let x_count = nvse::invref::count_for_extra_list(e, xdata);
                if x_count < 1 {
                    continue;
                }
                let x_count = x_count.min(base_count);
                base_count -= x_count;
                if no_equipped && extra::has_type(e, xdata, EXTRA_WORN) {
                    continue;
                }
                refs.push(inv_ref_create(e, a.this_obj, item, x_count, xdata));
            }
        }
        if base_count > 0 {
            refs.push(inv_ref_create(e, a.this_obj, item, base_count, 0));
        }
    }
    if list != 0 {
        for &refr in &refs {
            list_prepend(e, Ptr::new(list + LIST_FORM_LIST), refr);
        }
    }
    if !refs.is_empty() {
        if list != 0 {
            a.set_result(e, refs.len() as f64);
        } else {
            let kept: Vec<Element> = refs.iter().map(|&r| Element::Form(r)).collect();
            let id = create_array(e, a.script, &kept);
            a.set_result(e, f64::from(id));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetEquippedArmorRefs_Execute.
/// An array of inventory references for the armour in the actor's 20 biped slots that has a
/// worn stack (an armour in two slots twice); always an array when the actor has biped data and
/// changes.
fn get_equipped_armor_refs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let equipment = e.vcall(a.this_obj, VT_GET_BIPED_ANIM, &[]).u32();
    if equipment == 0 {
        return handled(true);
    }
    let entries = crate::overrides::changes_list(e, a.this_obj);
    if entries == 0 {
        return handled(true);
    }
    let mut refs = Vec::new();
    for slot in 0..BIPED_SLOT_COUNT {
        let item = e.mem.u32(equipment + BIPED_SLOTS + BIPED_SLOT_SIZE * slot);
        if item == 0 || !super::is_type(e, item, VTBL_ARMOR) {
            continue;
        }
        let entry = nvse::invref::find_for_item(e, entries, item);
        let xdata = if entry == 0 {
            0
        } else {
            equipped_extra(e, entry)
        };
        if xdata == 0 {
            continue;
        }
        let count = e.mem.u32(entry + ENTRY_COUNT) as i32;
        let refr = inv_ref_create(e, a.this_obj, item, count, xdata);
        if refr != 0 {
            refs.push(Element::Form(refr));
        }
    }
    let id = create_array(e, a.script, &refs);
    a.set_result(e, f64::from(id));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetEquippedItemRef.
/// An inventory reference for what the actor wears in the slot (5: the weapon). With 3D loaded:
/// the held weapon's entry and its first stack, or the first armour in the biped slots covering
/// the slot (none when its entry has no worn stack); without 3D: the first changes entry of a
/// weapon, or of an armour covering the slot, with a worn stack.
fn equipped_item_ref(e: &mut Engine, actor: u32, slot: u32) -> u32 {
    let entries = crate::overrides::changes_list(e, actor);
    if entries == 0 {
        return 0;
    }
    let part_mask = 1u32 << slot;
    let loaded = e.mem.u32(actor + REFR_LOADED_DATA);
    let root = if loaded == 0 {
        0
    } else {
        e.mem.u32(loaded + LOADED_3D)
    };
    if root != 0 {
        if slot == WEAPON_SLOT {
            let entry = weapon_info(e, actor);
            if entry != 0 && e.mem.u32(entry) != 0 {
                let (item, count) = (
                    e.mem.u32(entry + ENTRY_ITEM),
                    e.mem.u32(entry + ENTRY_COUNT),
                );
                let first = first_extra_list(e, entry);
                return inv_ref_create(e, actor, item, count as i32, first);
            }
        } else {
            let equipment = e.vcall(actor, VT_GET_BIPED_ANIM, &[]).u32();
            if equipment == 0 {
                return 0;
            }
            for s in 0..BIPED_SLOT_COUNT {
                let item = e.mem.u32(equipment + BIPED_SLOTS + BIPED_SLOT_SIZE * s);
                if item == 0
                    || !super::is_type(e, item, VTBL_ARMOR)
                    || e.mem.u32(item + ARMOR_PART_MASK) & part_mask == 0
                {
                    continue;
                }
                let entry = nvse::invref::find_for_item(e, entries, item);
                let xdata = if entry == 0 {
                    0
                } else {
                    equipped_extra(e, entry)
                };
                if xdata == 0 {
                    break;
                }
                let count = e.mem.u32(entry + ENTRY_COUNT) as i32;
                return inv_ref_create(e, actor, item, count, xdata);
            }
        }
        return 0;
    }
    for entry in list_items(e, Ptr::new(entries)) {
        if entry == 0 || e.mem.u32(entry) == 0 {
            continue;
        }
        let item = e.mem.u32(entry + ENTRY_ITEM);
        // JIP reads the entry's form type through a null form; such an entry is passed over.
        if item == 0 {
            continue;
        }
        if slot == WEAPON_SLOT {
            if type_of(e, item) != WEAPON {
                continue;
            }
        } else if !super::is_type(e, item, VTBL_ARMOR)
            || e.mem.u32(item + ARMOR_PART_MASK) & part_mask == 0
        {
            continue;
        }
        let xdata = equipped_extra(e, entry);
        if xdata != 0 {
            let count = e.mem.u32(entry + ENTRY_COUNT) as i32;
            return inv_ref_create(e, actor, item, count, xdata);
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetEquippedItemRef_Execute.
/// The inventory reference of what the actor wears in a slot (0 to 19).
fn get_equipped_item_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let slot = read.int(0, 0);
    if read.ok && slot <= 19 && super::is_actor(e, a.this_obj) {
        let refr = equipped_item_ref(e, a.this_obj, slot);
        if refr != 0 {
            super::set_form_result(e, &a, refr);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetHotkeyItemEntry.
/// The first of the player's armour, weapon and aid entries with a stack holding the hotkey
/// `index`: the entry and that stack.
fn hotkey_item_entry(e: &mut Engine, index: u8) -> Option<(u32, u32)> {
    for entry in super::ln_fn_inventory::player_entries(e) {
        if entry == 0
            || e.mem.u32(entry) == 0
            || super::ln_fn_inventory::hotkey_type(e, entry).is_none()
        {
            continue;
        }
        let lists = e.mem.u32(entry);
        for xdata in list_items(e, Ptr::new(lists)) {
            if xdata == 0 {
                continue;
            }
            let hotkey = extra::get_by_type(e, xdata, EXTRA_HOTKEY);
            if hotkey != 0 && e.mem.u8(hotkey + EXTRA_WORD) == index {
                return Some((entry, xdata));
            }
        }
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetHotkeyItemRef_Execute.
/// An inventory reference for the player's stack on hotkey 1 to 8.
fn get_hotkey_item_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let key = read.int(0, 0);
    if !read.ok || key == 0 || key > 8 {
        return handled(true);
    }
    if let Some((entry, xdata)) = hotkey_item_entry(e, (key - 1) as u8) {
        let item = e.mem.u32(entry + ENTRY_ITEM);
        let count = nvse::invref::count_for_extra_list(e, xdata);
        let player = super::ln_fn_inventory::player(e);
        let refr = inv_ref_create(e, player, item, count, xdata);
        super::set_form_result(e, &a, refr);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp:
// TESObjectREFR::CreateInventoryRefForScriptedObj; functions_jip/jip_fn_inventory.h:
// Cmd_GetSelfAsInventoryRef_Execute.
/// Run from an item's script inside a container: an inventory reference for the stack whose
/// script's locals are the running ones (its count, at least 1).
fn get_self_as_inventory_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let item = base_of(e, a.this_obj);
    if a.containing_obj == 0 || item == 0 {
        return handled(true);
    }
    let entry = changes_entry(e, a.containing_obj, item);
    let lists = if entry == 0 { 0 } else { e.mem.u32(entry) };
    if lists == 0 {
        return handled(true);
    }
    for xdata in list_items(e, Ptr::new(lists)) {
        if xdata == 0 {
            continue;
        }
        let script = extra::get_by_type(e, xdata, EXTRA_SCRIPT);
        if script == 0 || e.mem.u32(script + EXTRA_SCRIPT_EVENTS) != a.event_list {
            continue;
        }
        let count = nvse::invref::count_for_extra_list(e, xdata).max(1);
        let refr = inv_ref_create(e, a.containing_obj, item, count, xdata);
        super::set_form_result(e, &a, refr);
        break;
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntry::GetWeaponModEffectValue.
/// The value of the weapon's first attached mod (by the stack's mod flags) whose effect is
/// `effect`, 0 without.
fn weapon_mod_effect_value(e: &mut Engine, weapon: u32, xdata: u32, effect: u32) -> f32 {
    if xdata == 0 {
        return 0.0;
    }
    let x = extra::get_by_type(e, xdata, EXTRA_WEAPON_MOD_FLAGS);
    if x == 0 {
        return 0.0;
    }
    let flags = e.mem.u8(x + EXTRA_WORD);
    for i in 0..3u32 {
        if flags & (1 << i) != 0 && e.mem.u32(weapon + WEAPON_MOD_EFFECTS + 4 * i) == effect {
            return e.mem.f32(weapon + WEAPON_MOD_VALUES + 4 * i);
        }
    }
    0.0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntry::GetBaseHealth.
/// An item's full health: a weapon's health plus its attached health mod's value (by the first
/// stack), an armour's health, else 0.
fn base_health(e: &mut Engine, item: u32, first: u32) -> f32 {
    match type_of(e, item) {
        WEAPON => {
            let health = e.mem.u32(item + WEAPON_HEALTH) as i32 as f32;
            let added = weapon_mod_effect_value(e, item, first, MOD_EFFECT_HEALTH);
            added + health
        }
        ARMOR => e.mem.u32(item + ARMOR_HEALTH) as i32 as f32,
        _ => 0.0,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetItemRefCurrentHealth_Execute.
/// The stack's health (0), as a part of the full health (1), or the full health (2); without
/// an `ExtraHealth` the full health or 1. An inventory reference's stack, else the reference's
/// own extra data and base form.
fn get_item_ref_current_health(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let perc_or_base = read.int(0, 0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let (xdata, base) = match inv_ref_data(e, a.this_obj) {
        Some(d) if d.entry != 0 => {
            let item = e.mem.u32(d.entry + ENTRY_ITEM);
            let first = first_extra_list(e, d.entry);
            (d.xdata, base_health(e, item, first))
        }
        // JIP reads through the reference's null entry: the stack's own item and list instead.
        Some(d) => (d.xdata, base_health(e, d.item, d.xdata)),
        None => {
            let own = a.this_obj + REFR_EXTRA;
            let item = base_of(e, a.this_obj);
            (own, base_health(e, item, own))
        }
    };
    let value = if perc_or_base != 2 {
        let x = if xdata == 0 {
            0
        } else {
            extra::get_by_type(e, xdata, EXTRA_HEALTH)
        };
        if x != 0 {
            let health = e.mem.f32(x + EXTRA_WORD);
            if perc_or_base == 0 {
                health
            } else {
                health / base
            }
        } else if perc_or_base == 0 {
            base
        } else {
            1.0
        }
    } else {
        base
    };
    a.set_result(e, f64::from(value));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// GetArmorEffectiveDX.
/// The game's effective DT or DR of an armour reference: on the inventory reference's entry,
/// else on a changes entry made for the reference's own extra data (count 1).
fn armor_effective(e: &mut Engine, reference: u32, function: u32) -> f64 {
    let base = base_of(e, reference);
    if base == 0 || !super::is_type(e, base, VTBL_ARMOR) {
        return 0.0;
    }
    match inv_ref_data(e, reference) {
        Some(d) if d.entry != 0 => e.call(function, &args![d.entry, 0u32]).st0,
        // JIP calls through the reference's null entry.
        Some(_) => 0.0,
        None => {
            let entry = nvse::invref::create_temp_entry(e, base, 1, reference + REFR_EXTRA);
            let value = e.call(function, &args![entry, 0u32]).st0;
            nvse::invref::free_temp_entry(e, entry);
            value
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetArmorEffectiveDT_Execute.
fn get_armor_effective_dt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj != 0 {
        let value = armor_effective(e, a.this_obj, ARMOR_EFFECTIVE_DT);
        a.set_result(e, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_GetArmorEffectiveDR_Execute.
fn get_armor_effective_dr(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj != 0 {
        let value = armor_effective(e, a.this_obj, ARMOR_EFFECTIVE_DR);
        a.set_result(e, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::RemoveItemTarget.
/// A form list's forms one by one (a list inside it in turn), else an inventory item: up to
/// `quantity` (all for 0 or less) of the reference's count moved to `target` (0: removed), the
/// entry's stacks first, each whole or what is left (a stack without a count as 1: the loop ends
/// as `total` falls by at least 1 each time), then the rest without extra data.
///
/// `walking` holds the form lists being walked: JIP recurses without a bound, so a list holding
/// itself (or lists holding each other) overflows its stack; here a list already being walked is
/// passed over, and lists go at most [`MAX_LIST_DEPTH`] deep. Lists without such a loop are as in
/// JIP.
fn remove_item_target(
    e: &mut Engine,
    refr: u32,
    form: u32,
    target: u32,
    quantity: i32,
    keep_owner: bool,
    walking: &mut Vec<u32>,
) {
    if type_of(e, form) == LIST_FORM {
        if walking.contains(&form) || walking.len() >= MAX_LIST_DEPTH as usize {
            return;
        }
        walking.push(form);
        for item in list_items(e, Ptr::new(form + LIST_FORM_LIST)) {
            if item != 0 {
                remove_item_target(e, refr, item, target, quantity, keep_owner, walking);
            }
        }
        walking.pop();
        return;
    }
    if !is_inventory_type(u32::from(type_of(e, form))) {
        return;
    }
    let mut total = crate::overrides::item_count(e, refr, form);
    if total < 1 {
        return;
    }
    if quantity > 0 && quantity < total {
        total = quantity;
    }
    let entry = changes_entry(e, refr, form);
    if entry != 0 {
        while total > 0 {
            let xdata = first_extra_list(e, entry);
            if xdata == 0 {
                break;
            }
            let count = nvse::invref::count_for_extra_list(e, xdata);
            let sub = if count < 1 { 1 } else { count.min(total) };
            remove_item(e, refr, form, xdata, sub, keep_owner, target);
            total -= sub;
        }
    }
    if total > 0 {
        remove_item(e, refr, form, 0, total, keep_owner, target);
    }
}

/// `TESObjectREFR::RemoveItem` (virtual `0x17C`): `quantity` of `item` from the stack `xdata`,
/// not dropped, to `target`; the last flag JIP passes as 1.
fn remove_item(
    e: &mut Engine,
    refr: u32,
    item: u32,
    xdata: u32,
    quantity: i32,
    keep_owner: bool,
    target: u32,
) {
    e.vcall(
        refr,
        VT_REMOVE_ITEM,
        &[
            item,
            xdata,
            quantity as u32,
            u32::from(keep_owner),
            0,
            target,
            0,
            0,
            1,
            0,
        ],
    );
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_RemoveItemTarget_Execute.
/// An item or a form list's items moved from the calling reference to the target (a container
/// base form needed).
fn remove_item_target_cmd(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (form, target) = (read.form(0), read.form(1));
    if !read.ok || form == 0 || a.this_obj == 0 {
        return handled(true);
    }
    let (quantity, clr_owner) = (read.int(2, 0) as i32, read.int(3, 0));
    let base = base_of(e, a.this_obj);
    if base != 0 && super::ln_fn_inventory::container_of(e, base) != 0 {
        remove_item_target(
            e,
            a.this_obj,
            form,
            target,
            quantity,
            clr_owner == 0,
            &mut Vec::new(),
        );
    }
    handled(true)
}

// ---- adding, equipping and dropping (lane B3) ----

/// `TESContainer`'s constructor and destructor, `AddItem(form, count, extra)`, `TESLeveledList`'s
/// add to a container `(level, count, container, flag)` and the move of a container's items into a
/// reference `(reference, noMessage)` (all `thiscall`); `TESObjectREFR::GetCalcLevel(flag)`.
const CONTAINER_INIT: u32 = 0x0048_1610;
const CONTAINER_DESTROY: u32 = 0x0048_1680;
const CONTAINER_ADD: u32 = 0x0048_18e0;
const LEVELED_ADD_TO_CONTAINER: u32 = 0x0048_7f70;
const CONTAINER_MOVE_TO_REF: u32 = 0x0048_21a0;
const REFR_GET_CALC_LEVEL: u32 = 0x0056_7e10;
/// The `TESContainer` JIP keeps on its stack: a vtable and the item list's head node.
const TEMP_CONTAINER_SIZE: u32 = 0x0c;
/// A `TESContainer`'s item list (`+4`); an item's `ContainerObject`: count `+0`, form `+4`, its
/// `ContainerItemExtra` `+8` (owner `+0`, health `+8`, 0xC bytes: `00482090`).
const CONTAINER_LIST: u32 = 0x04;
const OBJECT_FORM: u32 = 0x04;
const OBJECT_EXTRA: u32 = 0x08;
const ITEM_EXTRA_SIZE: u32 = 0x0c;
const ITEM_EXTRA_HEALTH: u32 = 0x08;
/// `TESLevItem`'s `TESLeveledList` (JIP's `add ecx, 0x30`).
const LEVELED_LIST: u32 = 0x30;
/// `kFormType_TESObjectBOOK`, `kFormType_TESLevItem`.
const BOOK: u8 = 0x19;
const LEVELED_ITEM: u8 = 0x34;
/// `TESObjectWEAP::eWeaponType` (jip_fn_weapon.rs): 10 and above are thrown weapons and mines.
const WEAPON_TYPE: u32 = 0xf4;
const WEAPON_TYPE_THROWN: u8 = 0x0a;
/// `kXData_ExtraCount`.
const EXTRA_COUNT: u32 = 0x24;
/// `Actor::EquipItem`, `Actor::UnequipItem` (`thiscall`, six words after the actor).
const EQUIP_ITEM: u32 = 0x0088_c650;
const UNEQUIP_ITEM: u32 = 0x0088_c790;
/// The game's own message texts JIP shows after an item's name (pointers the game fills at
/// start-up) when equipping and when unequipping.
const MSG_EQUIPPED: u32 = 0x011d_2a10;
const MSG_UNEQUIPPED: u32 = 0x011d_4250;
/// `QueueUIMessage` (cdecl: the text, the emotion, the icon, the sound, the seconds as a float,
/// a flag) and the icon path JIP passes (`"Interface\Icons\Message Icons\glow_..."`).
const QUEUE_UI_MESSAGE: u32 = 0x0070_52f0;
const MESSAGE_ICON: u32 = 0x0102_08e0;
const TWO_SECONDS: u32 = 0x4000_0000;
/// JIP's message buffer (`sub esp, 0x70`).
const MESSAGE_BUFFER: usize = 0x70;
/// `ContainerItemExtra`'s greatest health JIP sets (`kMaxHealth`, 0x3F7FBE77).
const MAX_ITEM_HEALTH_BITS: u32 = 0x3f7f_be77;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: ShowItemMessage.
/// The corner message "<name> <text>." for an item with a name (none without), shown 2 seconds.
///
/// JIP builds it in a 0x70-byte stack buffer without a bound; here it is cut to fit.
fn show_item_message(e: &mut Engine, item: u32, text: u32) {
    let name = super::the_name(e, item);
    if name.is_empty() {
        return;
    }
    let mut message = name;
    message.push(b' ');
    if text != 0 {
        message.extend_from_slice(&nvse::table::guest_text(e, text));
    }
    message.push(b'.');
    message.truncate(MESSAGE_BUFFER - 1);
    let block = heap_alloc(e, MESSAGE_BUFFER as u32);
    e.mem.write(block, &message);
    e.mem.set_u8(block + message.len() as u32, 0);
    e.call(
        QUEUE_UI_MESSAGE,
        &args![block, 0u32, MESSAGE_ICON, 0u32, TWO_SECONDS, 0u32],
    );
    heap_free(e, block);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::EquipItemAlt (asm).
/// The entry's item equipped from its first stack: armour, books and aid one, ammunition the
/// entry's count, a weapon one (a thrown weapon or mine the count); other items not. Then the
/// item's "equipped" message unless `no_message` (its low byte) is set.
fn equip_item_alt(e: &mut Engine, actor: u32, entry: u32, no_unequip: u32, no_message: u32) {
    let item = e.mem.u32(entry + ENTRY_ITEM);
    // JIP reads the type through a null item; such an entry is passed over.
    if item == 0 {
        return;
    }
    let all = e.mem.u32(entry + ENTRY_COUNT);
    let count = match type_of(e, item) {
        ARMOR | BOOK | ALCHEMY_ITEM => 1,
        AMMO => all,
        WEAPON if e.mem.u8(item + WEAPON_TYPE) < WEAPON_TYPE_THROWN => 1,
        WEAPON => all,
        _ => return,
    };
    let xdata = first_extra_list(e, entry);
    e.call(
        EQUIP_ITEM,
        &args![actor, item, count, xdata, 1u32, no_unequip, 1u32],
    );
    if no_message as u8 == 0 {
        let text = e.mem.u32(MSG_EQUIPPED);
        show_item_message(e, item, text);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h:
// SetContainerItemsHealthHook (called directly by AddItemAlt; its patch of 00482090 is not made).
/// Each item of the container: armour and weapons other than thrown ones and mines get the health
/// part `health` (kept in 0 to just under 1; a lower one already there stays); other items' extra
/// data has its health reset to 1 and, without an owner, is freed.
fn set_container_items_health(e: &mut Engine, container: u32, health: f32) {
    // minss with kMaxHealth (a NaN gives it), then maxss with 0.
    let max = f32::from_bits(MAX_ITEM_HEALTH_BITS);
    let health = if health < max { health } else { max };
    let health = if health > 0.0 { health } else { 0.0 };
    for object in list_items(e, Ptr::new(container + CONTAINER_LIST)) {
        if object == 0 {
            continue;
        }
        let extra = e.mem.u32(object + OBJECT_EXTRA);
        let item = e.mem.u32(object + OBJECT_FORM);
        // JIP reads the type through a null form; none is added by the callers here.
        if item == 0 {
            continue;
        }
        let has_health = match type_of(e, item) {
            ARMOR => true,
            WEAPON => e.mem.u8(item + WEAPON_TYPE) < WEAPON_TYPE_THROWN,
            _ => false,
        };
        if !has_health {
            if extra != 0 {
                e.mem.set_f32(extra + ITEM_EXTRA_HEALTH, 1.0);
                if e.mem.u32(extra) == 0 {
                    e.mem.set_u32(object + OBJECT_EXTRA, 0);
                    heap_free(e, extra);
                }
            }
        } else if extra == 0 {
            let block = heap_alloc(e, ITEM_EXTRA_SIZE);
            e.mem.set_u32(block, 0);
            e.mem.set_u32(block + 4, 0);
            e.mem.set_f32(block + ITEM_EXTRA_HEALTH, health);
            e.mem.set_u32(object + OBJECT_EXTRA, block);
        } else {
            let old = e.mem.f32(extra + ITEM_EXTRA_HEALTH);
            e.mem.set_f32(
                extra + ITEM_EXTRA_HEALTH,
                if health < old { health } else { old },
            );
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::AddItemAlt
// (asm).
/// `count` of an item, of each inventory item of a form list, or of what a leveled item gives at
/// the reference's level, put in a temporary container, given the health part, moved into the
/// reference (its message unless `no_message`), and with `do_equip` (its low byte) each equipped
/// by an actor that has changes. Other forms add nothing.
///
/// JIP keeps the temporary container on its stack; here it is a block of the game's heap.
fn add_item_alt(
    e: &mut Engine,
    refr: u32,
    form: u32,
    count: u32,
    health: f32,
    do_equip: u32,
    no_message: u32,
) {
    let container = heap_alloc(e, TEMP_CONTAINER_SIZE);
    e.call(CONTAINER_INIT, &args![container]);
    let kind = type_of(e, form);
    let added = if kind == LEVELED_ITEM {
        let level = e.call(REFR_GET_CALC_LEVEL, &args![refr, 0u32]).u32();
        e.call(
            LEVELED_ADD_TO_CONTAINER,
            &args![form + LEVELED_LIST, level, count, container, 0u32],
        );
        true
    } else if kind == LIST_FORM {
        for item in list_items(e, Ptr::new(form + LIST_FORM_LIST)) {
            if item != 0 && is_inventory_type(u32::from(type_of(e, item))) {
                e.call(CONTAINER_ADD, &args![container, item, count, 0u32]);
            }
        }
        true
    } else if is_inventory_type(u32::from(kind)) {
        e.call(CONTAINER_ADD, &args![container, form, count, 0u32]);
        true
    } else {
        false
    };
    if added {
        set_container_items_health(e, container, health);
        e.call(CONTAINER_MOVE_TO_REF, &args![container, refr, no_message]);
        if do_equip as u8 != 0 && super::is_actor(e, refr) {
            let entries = crate::overrides::changes_list(e, refr);
            if entries != 0 {
                for object in list_items(e, Ptr::new(container + CONTAINER_LIST)) {
                    if object == 0 {
                        continue;
                    }
                    let item = e.mem.u32(object + OBJECT_FORM);
                    let entry = nvse::invref::find_for_item(e, entries, item);
                    if entry != 0 {
                        equip_item_alt(e, refr, entry, 0, no_message);
                    }
                }
            }
        }
    }
    e.call(CONTAINER_DESTROY, &args![container]);
    heap_free(e, container);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_AddItemAlt_Execute.
/// Adds `count` (default 1, none for 0) of an item, a form list's items or a leveled item to a
/// reference with a container, at `condition` percent (default 100), optionally equipped, its
/// message off by default.
fn add_item_alt_cmd(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 || a.this_obj == 0 {
        return handled(true);
    }
    let count = read.int(1, 1);
    let condition = read.float(2, 100.0);
    let (do_equip, no_message) = (read.int(3, 0), read.int(4, 1));
    let base = base_of(e, a.this_obj);
    if count != 0 && base != 0 && super::ln_fn_inventory::container_of(e, base) != 0 {
        add_item_alt(
            e,
            a.this_obj,
            form,
            count,
            condition * 0.01,
            do_equip,
            no_message,
        );
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_EquipItemAlt_Execute.
/// An actor equips an item it holds (`EquipItemAlt`), optionally locked on, its message off by
/// default.
fn equip_item_alt_cmd(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let item = read.form(0);
    if !read.ok || item == 0 || !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let (no_unequip, no_message) = (read.int(1, 0), read.int(2, 1));
    let entry = changes_entry(e, a.this_obj, item);
    if entry != 0 {
        equip_item_alt(e, a.this_obj, entry, no_unequip, no_message);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_UnequipItemAlt_Execute.
/// An actor takes off a weapon or an armour it wears (its worn stack), optionally kept off, with
/// the "unequipped" message unless it is off (by default).
fn unequip_item_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let item = read.form(0);
    if !read.ok || item == 0 || !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    if type_of(e, item) != WEAPON && !super::is_type(e, item, VTBL_ARMOR) {
        return handled(true);
    }
    let (no_equip, no_message) = (read.int(1, 0), read.int(2, 1));
    let entry = changes_entry(e, a.this_obj, item);
    let xdata = if entry == 0 {
        0
    } else {
        equipped_extra(e, entry)
    };
    if xdata == 0 {
        return handled(true);
    }
    // `UnequipItem`'s last three are `bool`s: the words become 0 or 1.
    e.call(
        UNEQUIP_ITEM,
        &args![
            a.this_obj,
            item,
            1u32,
            xdata,
            1u32,
            u32::from(no_equip != 0),
            u32::from(no_message != 0)
        ],
    );
    if no_message == 0 {
        let text = e.mem.u32(MSG_UNEQUIPPED);
        show_item_message(e, item, text);
    }
    handled(true)
}

/// `TESObjectREFR::RemoveItem` (virtual `0x17C`) dropping `quantity` of `item` from the stack
/// `xdata` into the world (no target); the last flag JIP passes as 1.
fn drop_item(e: &mut Engine, refr: u32, item: u32, xdata: u32, quantity: i32, keep_owner: bool) {
    e.vcall(
        refr,
        VT_REMOVE_ITEM,
        &[
            item,
            xdata,
            quantity as u32,
            u32::from(keep_owner),
            1,
            0,
            0,
            0,
            1,
            0,
        ],
    );
}

/// Whether an item is dropped as a stack: a weapon when thrown or a mine, else anything but
/// armour (JIP's `stacked`).
fn drops_stacked(e: &mut Engine, item: u32) -> bool {
    if type_of(e, item) == WEAPON {
        e.mem.u8(item + WEAPON_TYPE) > 9
    } else {
        !super::is_type(e, item, VTBL_ARMOR)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_DropAlt_Execute.
/// Drops an item, or each item of a form list, from a reference with a container: up to
/// `drop_count` (all for 0 or less) of its count, the entry's stacks first (a scripted item's
/// stack one at a time, its count dropped; armour and weapons other than thrown ones one by one),
/// then the rest without extra data in drops of at most 0x7FFF; the owner kept unless cleared.
fn drop_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 || a.this_obj == 0 {
        return handled(true);
    }
    let (drop_count, clr_owner) = (read.int(1, 0) as i32, read.int(2, 0));
    let container = super::ln_fn_inventory::container_of(e, base_of(e, a.this_obj));
    if container == 0 {
        return handled(true);
    }
    let entries = crate::overrides::changes_list(e, a.this_obj);
    if entries == 0 {
        return handled(true);
    }
    let items = if type_of(e, form) == LIST_FORM {
        list_items(e, Ptr::new(form + LIST_FORM_LIST))
    } else {
        vec![form]
    };
    let keep_owner = clr_owner == 0;
    let refr = a.this_obj;
    for item in items {
        if item == 0 || !is_inventory_type(u32::from(type_of(e, item))) {
            continue;
        }
        let mut total = crate::overrides::form_count(e, container + CONTAINER_LIST, entries, item);
        if total < 1 {
            continue;
        }
        if drop_count > 0 && drop_count < total {
            total = drop_count;
        }
        let entry = nvse::invref::find_for_item(e, entries, item);
        if entry != 0 && e.mem.u32(entry) != 0 {
            let has_script = crate::world_hooks::has_script(e, item);
            let stacked = drops_stacked(e, item);
            while total > 0 {
                let xdata = first_extra_list(e, entry);
                if xdata == 0 {
                    break;
                }
                let mut sub = nvse::invref::count_for_extra_list(e, xdata);
                if sub < 1 {
                    sub = 1;
                } else if sub > 1 {
                    if has_script && extra::has_type(e, xdata, EXTRA_SCRIPT) {
                        extra::remove_by_type(e, xdata, EXTRA_COUNT);
                        sub = 1;
                    } else if sub > total {
                        sub = total;
                    }
                }
                total -= sub;
                if stacked {
                    drop_item(e, refr, item, xdata, sub, keep_owner);
                } else {
                    for _ in 0..sub {
                        drop_item(e, refr, item, xdata, 1, keep_owner);
                    }
                }
            }
        }
        while total > 0 {
            let sub = total.min(0x7fff);
            drop_item(e, refr, item, 0, sub, keep_owner);
            total -= sub;
        }
    }
    handled(true)
}

/// `InventoryRefGetForID(thisObj->refID)` with what `DropMeAlt` and `RemoveMeIRAlt` read: the
/// temporary reference's id, its data, its container and its count (`entry->countDelta`).
fn inv_ref_stack(e: &mut Engine, reference: u32) -> Option<(u32, nvse::invref::Data, u32, i32)> {
    if reference == 0 {
        return None;
    }
    let id = super::ref_id(e, reference);
    if !nvse::invref::for_ref_id(e, id) {
        return None;
    }
    let (data, container) = nvse::invref::get(e, id).map(|r| (r.data, r.container))?;
    let count = nvse::invref::count(e, id);
    Some((id, data, container, count))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_DropMeAlt_Execute.
/// The inventory reference's stack dropped by its container: up to `drop_count` (all for 0 or
/// less, or more than it has), as a stack or (armour and weapons other than thrown ones) one by
/// one; then the reference is marked removed.
///
/// An inventory reference without a changes entry has no count here (JIP reads through the null
/// entry); one without a container drops nothing (JIP calls through null).
fn drop_me_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (mut drop_count, clr_owner) = (read.int(0, 0) as i32, read.int(1, 0));
    let Some((id, data, container, count)) = inv_ref_stack(e, a.this_obj) else {
        return handled(true);
    };
    if count < 1 || container == 0 {
        return handled(true);
    }
    if drop_count < 1 || drop_count > count {
        drop_count = count;
    }
    let keep_owner = clr_owner == 0;
    if drops_stacked(e, data.item) {
        drop_item(e, container, data.item, data.xdata, drop_count, keep_owner);
    } else {
        for _ in 0..drop_count {
            drop_item(e, container, data.item, data.xdata, 1, keep_owner);
        }
    }
    nvse::invref::set_removed(e, id);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_RemoveMeIRAlt_Execute.
/// Up to `quantity` (all for 0 or less, or more than it has) of the inventory reference's stack
/// taken from its container, into `target` when given; then the reference is marked removed. The
/// same guards as `DropMeAlt`.
fn remove_me_ir_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (mut quantity, clr_owner) = (read.int(0, 0) as i32, read.int(1, 0));
    let target = read.form(2);
    let Some((id, data, container, count)) = inv_ref_stack(e, a.this_obj) else {
        return handled(true);
    };
    if count < 1 || container == 0 {
        return handled(true);
    }
    if quantity < 1 || quantity > count {
        quantity = count;
    }
    remove_item(
        e,
        container,
        data.item,
        data.xdata,
        quantity,
        clr_owner == 0,
        target,
    );
    nvse::invref::set_removed(e, id);
    handled(true)
}

// ---- lane B4: setting an inventory reference's mod flags, health and hotkey ----

/// The extra data the setters create (JIP's `CreateExtraType` with `kVtbl_*`, each vtable the one
/// the exe's constructor stores): `ExtraHealth` 0x25 (`00431D10`, its health at +0xC),
/// `ExtraHotkey` 0x4A (`00432380`, its index byte at +0xC), `ExtraWeaponModFlags` 0x8D
/// (`0042E450`, its flags byte at +0xC).
const VTBL_EXTRA_HEALTH: u32 = 0x0101_58e4;
const VTBL_EXTRA_HOTKEY: u32 = 0x0101_592c;
const VTBL_EXTRA_WEAPON_MOD_FLAGS: u32 = 0x0101_59a4;
/// `ExtraDataList` (0x20 bytes, the vtable its constructor `00410360` stores) and its first extra
/// (`m_data`, +4).
const VTBL_EXTRA_DATA_LIST: u32 = 0x0101_43e8;
const EXTRA_DATA_LIST_SIZE: u32 = 0x20;
const LIST_DATA: u32 = 0x04;
/// `ExtraDataList::CopyListForReference` `00412490` (`thiscall`: the new list, the source, the
/// "copy and remove" byte; `RET 8`), which JIP's `ExtraDataList::CreateCopy` calls.
const EXTRA_COPY_FOR_REFERENCE: u32 = 0x0041_2490;
/// A `ContChangesEntry` (extra lists +0, count +4, item +8) and a list node: their sizes.
const ENTRY_SIZE: u32 = 0x0c;
/// `TESObjectWEAP::itemMod[3]` (+0x350; AUDIT-batch12.md).
const WEAPON_ITEM_MODS: u32 = 0x350;
/// `MarkModified`'s flag for a changed inventory (JIP: `MarkModified(0x20)`).
const CHANGED_INVENTORY: u32 = 0x20;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameBSExtraData.cpp: ExtraDataList::Create.
/// A new extra data list: 0x20 zeroed bytes of the game's heap with its vtable.
fn new_extra_list(e: &mut Engine) -> u32 {
    let xdl = heap_alloc(e, EXTRA_DATA_LIST_SIZE);
    e.mem.write(xdl, &[0; EXTRA_DATA_LIST_SIZE as usize]);
    e.mem.set_u32(xdl, VTBL_EXTRA_DATA_LIST);
    xdl
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp: ExtraHealth::Create,
// ExtraHotkey::Create, ExtraWeaponModFlags::Create.
/// `CreateExtraType(Type)` with `dataPtr[3]` (the whole word at +0xC) set to `word`.
fn new_extra(e: &mut Engine, vtable: u32, kind: u32, word: u32) -> u32 {
    let x = extra::create(e, vtable, kind);
    e.mem.set_u32(x + EXTRA_WORD, word);
    x
}

/// A new list node of the game's heap holding `item` (`tList::Init(item)` on a list JIP has just
/// allocated: the item, no next node).
fn new_list(e: &mut Engine, item: u32) -> u32 {
    let node = heap_alloc(e, ListNode::SIZE);
    e.mem.set_u32(node, item);
    e.mem.set_u32(node + 4, 0);
    node
}

/// The inventory reference behind `reference` (`InventoryRefGetForID(thisObj->refID)`): its id,
/// its data and its container.
fn inv_ref_of(e: &mut Engine, reference: u32) -> Option<(u32, nvse::invref::Data, u32)> {
    if reference == 0 {
        return None;
    }
    let id = super::ref_id(e, reference);
    if !nvse::invref::for_ref_id(e, id) {
        return None;
    }
    nvse::invref::get(e, id).map(|r| (id, r.data, r.container))
}

/// `invRef->xData = xdata`.
fn set_inv_ref_xdata(e: &mut Engine, id: u32, xdata: u32) {
    if let Some(r) = nvse::invref::get(e, id) {
        r.data.xdata = xdata;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp:
// InventoryRef::CreateExtraData (with nvse/GameExtraData.cpp: ContChangesEntry::Create).
/// A new extra data list for an inventory reference that has none: first in the container's
/// changes entry for the item (a new entry of count 0 first in the changes when there is none;
/// a new list of lists when the entry has none), the reference's stack from now on; the
/// container marked as changed (0x20). 0 when the container has no changes.
///
/// A reference without a container makes none (JIP calls through the null container).
fn inv_ref_create_extra_data(e: &mut Engine, id: u32, item: u32, container: u32) -> u32 {
    if container == 0 {
        return 0;
    }
    let entries = crate::overrides::changes_list(e, container);
    if entries == 0 {
        return 0;
    }
    let mut entry = nvse::invref::find_for_item(e, entries, item);
    if entry == 0 {
        entry = heap_alloc(e, ENTRY_SIZE);
        e.mem.set_u32(entry, 0);
        e.mem.set_u32(entry + ENTRY_COUNT, 0);
        e.mem.set_u32(entry + ENTRY_ITEM, item);
        list_prepend(e, Ptr::new(entries), entry);
    }
    let xdata = new_extra_list(e);
    set_inv_ref_xdata(e, id, xdata);
    let lists = e.mem.u32(entry);
    if lists != 0 {
        list_prepend(e, Ptr::new(lists), xdata);
    } else {
        let lists = new_list(e, xdata);
        e.mem.set_u32(entry, lists);
    }
    extra::mark_modified(e, container, CHANGED_INVENTORY);
    xdata
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp:
// InventoryRef::SplitFromStack (the default maxStack 1, which every caller here passes; with
// nvse/GameExtraData.cpp: ExtraDataList::CreateCopy).
/// One item of the inventory reference's stack split off: a stack with an `ExtraCount` above 1
/// is copied (the game's copy for a reference) first into the container's entry for the item,
/// the copy losing its worn mark, hotkey and count; the old stack keeps the rest (its count gone
/// when one is left, and the list freed when nothing else is in it). The copy is the reference's
/// stack from now on. A stack without a count, of one, or whose item the container has no entry
/// for stays as it is.
///
/// An entry without a list of lists keeps the stack too (JIP prepends to the null list).
fn inv_ref_split_from_stack(
    e: &mut Engine,
    id: u32,
    data: nvse::invref::Data,
    container: u32,
) -> u32 {
    let xdata = data.xdata;
    let count = extra::get_by_type(e, xdata, EXTRA_COUNT);
    if count == 0 {
        return xdata;
    }
    let delta = i32::from(e.mem.u16(count + EXTRA_WORD) as i16) - 1;
    if delta <= 0 {
        return xdata;
    }
    let entry = changes_entry(e, container, data.item);
    if entry == 0 {
        return xdata;
    }
    let lists = e.mem.u32(entry);
    if lists == 0 {
        return xdata;
    }
    let out = new_extra_list(e);
    e.call(EXTRA_COPY_FOR_REFERENCE, &args![out, xdata, 0u32]);
    list_prepend(e, Ptr::new(lists), out);
    extra::remove_by_type(e, out, EXTRA_WORN);
    extra::remove_by_type(e, out, EXTRA_HOTKEY);
    if delta < 2 {
        extra::remove_by_type(e, xdata, EXTRA_COUNT);
        if e.mem.u32(xdata + LIST_DATA) == 0 {
            super::list_remove(e, Ptr::new(lists), xdata);
            heap_free(e, xdata);
        }
    } else {
        e.mem.set_u16(count + EXTRA_WORD, delta as u16);
    }
    set_inv_ref_xdata(e, id, out);
    extra::remove_by_type(e, out, EXTRA_COUNT);
    out
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_SetWeaponRefModFlags_Execute.
/// Sets the mod flags (0 to 7; a flag for a mod slot the weapon has no mod in is dropped) of a
/// weapon inventory reference's stack: an `ExtraWeaponModFlags` changed, or removed for 0 (the
/// stack's list taken out of the entry and freed when nothing is left in it); a new one on the
/// stack (one item split off a stack of more) or on a new list for a reference without a stack.
/// 1 when set (also when 0 asked of a stack without flags), nothing when they are the same.
///
/// A stack freed here is no longer the reference's (its list 0): JIP leaves the reference
/// pointing at the freed list.
fn set_weapon_ref_mod_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut flags = read.int(0, 0);
    if !read.ok || flags > 7 {
        return handled(true);
    }
    let Some((id, data, container)) = inv_ref_of(e, a.this_obj) else {
        return handled(true);
    };
    if type_of(e, data.item) != WEAPON {
        return handled(true);
    }
    for slot in 0..3u32 {
        if e.mem.u32(data.item + WEAPON_ITEM_MODS + 4 * slot) == 0 {
            flags &= !(1 << slot);
        }
    }
    let xdata = data.xdata;
    if xdata != 0 {
        let x = extra::get_by_type(e, xdata, EXTRA_WEAPON_MOD_FLAGS);
        if x != 0 {
            if u32::from(e.mem.u8(x + EXTRA_WORD)) == flags {
                return handled(true);
            }
            if flags != 0 {
                e.mem.set_u8(x + EXTRA_WORD, flags as u8);
            } else {
                extra::remove_by_type(e, xdata, EXTRA_WEAPON_MOD_FLAGS);
                if e.mem.u32(xdata + LIST_DATA) == 0 {
                    let entry = changes_entry(e, container, data.item);
                    if entry == 0 || e.mem.u32(entry) == 0 {
                        return handled(true);
                    }
                    super::list_remove(e, Ptr::new(e.mem.u32(entry)), xdata);
                    heap_free(e, xdata);
                    set_inv_ref_xdata(e, id, 0);
                }
            }
        } else if flags != 0 {
            let stack = if nvse::invref::count(e, id) > 1 {
                inv_ref_split_from_stack(e, id, data, container)
            } else {
                xdata
            };
            let x = new_extra(
                e,
                VTBL_EXTRA_WEAPON_MOD_FLAGS,
                EXTRA_WEAPON_MOD_FLAGS,
                flags,
            );
            extra::add_extra(e, stack, x);
        }
    } else if flags != 0 {
        let stack = inv_ref_create_extra_data(e, id, data.item, container);
        if stack == 0 {
            return handled(true);
        }
        let x = new_extra(
            e,
            VTBL_EXTRA_WEAPON_MOD_FLAGS,
            EXTRA_WEAPON_MOD_FLAGS,
            flags,
        );
        extra::add_extra(e, stack, x);
    }
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_SetItemRefCurrentHealth_Execute.
/// Sets the health of an inventory reference's stack (`setPercent`: as a part of the full
/// health), at most the full health: its `ExtraHealth` changed or a new one added (on a new list
/// for a reference without a stack). Nothing for an item without full health. 1 when set.
///
/// A reference without a changes entry: the full health of its own item and stack (JIP reads
/// through the null entry), as `GetItemRefCurrentHealth` does here.
fn set_item_ref_current_health(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut health = read.float(0, 0.0);
    let set_percent = read.int(1, 0);
    if !read.ok {
        return handled(true);
    }
    let Some((id, data, container)) = inv_ref_of(e, a.this_obj) else {
        return handled(true);
    };
    let base = if data.entry != 0 {
        let item = e.mem.u32(data.entry + ENTRY_ITEM);
        let first = first_extra_list(e, data.entry);
        base_health(e, item, first)
    } else {
        base_health(e, data.item, data.xdata)
    };
    if base == 0.0 {
        return handled(true);
    }
    if set_percent != 0 {
        health *= base;
    }
    if health > base {
        health = base;
    }
    let xdata = data.xdata;
    if xdata != 0 {
        let x = extra::get_by_type(e, xdata, EXTRA_HEALTH);
        if x != 0 {
            e.mem.set_f32(x + EXTRA_WORD, health);
        } else {
            let x = new_extra(e, VTBL_EXTRA_HEALTH, EXTRA_HEALTH, health.to_bits());
            extra::add_extra(e, xdata, x);
        }
    } else {
        let stack = inv_ref_create_extra_data(e, id, data.item, container);
        if stack == 0 {
            return handled(true);
        }
        let x = new_extra(e, VTBL_EXTRA_HEALTH, EXTRA_HEALTH, health.to_bits());
        extra::add_extra(e, stack, x);
    }
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetHotkeyItemEntry,
// ClearHotkey.
/// The first list (in the player's armour, weapons and aid items) with a hotkey of `index` loses
/// it (the list taken out and freed when nothing is left in it); whether there was one.
fn clear_hotkey(e: &mut Engine, index: u8) -> bool {
    use super::ln_fn_inventory::{drop_hotkey, hotkey_type, player_entries};
    for entry in player_entries(e) {
        if entry == 0 || e.mem.u32(entry) == 0 || hotkey_type(e, entry).is_none() {
            continue;
        }
        let lists = e.mem.u32(entry);
        for xdl in list_items(e, Ptr::<ListNode>::new(lists)) {
            if xdl == 0 {
                continue;
            }
            let h = extra::get_by_type(e, xdl, EXTRA_HOTKEY);
            if h != 0 && e.mem.u8(h + EXTRA_WORD) == index {
                drop_hotkey(e, entry, xdl);
                return true;
            }
        }
    }
    false
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_SetHotkeyItemRef_Execute.
/// Puts the player's armour, weapon or aid inventory reference on hotkey 1 to 8, taking the key
/// from the item that has it. As in JIP, only when some item has that key now (`ClearHotkey`
/// answers whether one lost it): a stack without a hotkey gets a new one; a stack with another
/// key changes it; a reference without a stack gets a new list with the hotkey. 1 when set.
fn set_hotkey_item_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let key = read.int(0, 0);
    if !read.ok || key == 0 || key > 8 {
        return handled(true);
    }
    let Some((id, data, container)) = inv_ref_of(e, a.this_obj) else {
        return handled(true);
    };
    if container != super::ln_fn_inventory::player(e) {
        return handled(true);
    }
    if !matches!(type_of(e, data.item), ARMOR | WEAPON | ALCHEMY_ITEM) {
        return handled(true);
    }
    let index = (key - 1) as u8;
    let xdata = data.xdata;
    if xdata != 0 {
        let h = extra::get_by_type(e, xdata, EXTRA_HOTKEY);
        if h == 0 {
            if !clear_hotkey(e, index) {
                return handled(true);
            }
            let x = new_extra(e, VTBL_EXTRA_HOTKEY, EXTRA_HOTKEY, u32::from(index));
            extra::add_extra(e, xdata, x);
        } else if e.mem.u8(h + EXTRA_WORD) == index || !clear_hotkey(e, index) {
            return handled(true);
        } else {
            e.mem.set_u8(h + EXTRA_WORD, index);
        }
    } else {
        if !clear_hotkey(e, index) {
            return handled(true);
        }
        let stack = inv_ref_create_extra_data(e, id, data.item, container);
        if stack == 0 {
            return handled(true);
        }
        let x = new_extra(e, VTBL_EXTRA_HOTKEY, EXTRA_HOTKEY, u32::from(index));
        extra::add_extra(e, stack, x);
    }
    a.set_result(e, 1.0);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_inventory_tests.rs"]
mod tests;
