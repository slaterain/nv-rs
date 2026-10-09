//! `fallout shared/inventorychanges.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! Session notes (read these first when you continue the unit).
//!
//! * The unit is large (148 functions). Translated so far: `004bc550` to
//!   `004bdf90` (the `ItemChange` methods and the weapon mod helpers around
//!   them). The next session continues at `004be060` (`004be060` and
//!   `004be080` are small armor helpers that `004bdf90` calls by address).
//! * An `ItemChange` is a `BSSimpleList<ExtraDataList *>` of the extra data
//!   lists of one stack of an item, the number of items, and the item's form
//!   (see [`ItemChange`]). The list is walked the way the game does it: the
//!   head node `[this]` is nonnull and its item is the first extra list; an
//!   empty list is a head node whose item is null. The accessors
//!   `006815c0` (address of the item word of a node) and `00726070` (next
//!   node) are the shared ones in `extradatalist.rs` (`list_item`,
//!   `list_next`).
//! * Most methods act on the first extra list only (`first_item`); the ones
//!   that loop restart from the head after they delete a list from the
//!   `BSSimpleList`, as the game does.
//! * Form pointers are read at their PC offsets with the Xbox PDB names in
//!   comments (the classes belong to other units).
//! * x87 note: values the game keeps as `float` are rounded to `f32` where
//!   the code stores a `float`; a `float` returned in `ST0` is an `f32`
//!   result of the translation.

#[allow(unused_imports)]
use crate::prelude::*;

use super::extradatalist::{list_item, list_next};

/// `operator new(size)` (`platform`).
pub(crate) const OPERATOR_NEW: u32 = 0x0040_1000;
/// `_ftol2_sse` (`00ec62c0`): truncates the float in ST0 to an integer in
/// EAX. The value is passed as an `f64` argument (two words).
pub(crate) const FTOL: u32 = 0x00ec_62c0;
/// `__RTDynamicCast(object, vfDelta, srcType, targetType, isReference)`.
pub(crate) const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// The word at +4 of an object (`00726070`; the next node of a list node, or
/// the value of a `TESHealthForm`).
pub(crate) const WORD_AT_4: u32 = 0x0072_6070;
/// The word at +8 of an object (`0044ddc0`; for an `ItemChange` its form).
pub(crate) const WORD_AT_8: u32 = 0x0044_ddc0;
/// The word at +0 of an object (`00559450`; for an `ItemChange` or an
/// inventory entry its list).
pub(crate) const WORD_AT_0: u32 = 0x0055_9450;
/// The byte at +4 of a form (`00401170`): its form type.
pub(crate) const FORM_TYPE: u32 = 0x0040_1170;
/// `BSSimpleList<T>::BSSimpleList()` (`0096a2d0`, named
/// `Concurrency::details::QuickBitSet::QuickBitSet` by the linker's folding):
/// zeroes the item and next words. The map has no name for it.
pub(crate) const LIST_NODE_CONSTRUCT: u32 = 0x0096_a2d0;
/// `BSSimpleList<T>::Remove(&item)` (`00905330`): unlinks the node holding
/// `*item`, starting from the node it is called on.
pub(crate) const LIST_REMOVE: u32 = 0x0090_5330;
/// `BSSimpleList<T>::AddHead/Append(&item)` (`005ae3d0`): stores `*item` in
/// the head node when it is empty, else in a new node after the head.
pub(crate) const LIST_ADD: u32 = 0x005a_e3d0;
/// `BSSimpleList<T>::AddTail(&item)` (`00905820`): stores `*item` in the
/// last node when it is empty, else in a new node after it.
pub(crate) const LIST_ADD_TAIL: u32 = 0x0090_5820;
/// `BSSimpleList` clear (`00470470`) and the deleting destructor
/// (`004702f0(list, flags)`).
pub(crate) const LIST_CLEAR: u32 = 0x0047_0470;
pub(crate) const LIST_DESTROY: u32 = 0x0047_02f0;
/// `ExtraDataList::ExtraDataList` (Xbox PDB).
pub(crate) const EXTRA_DATA_LIST_CONSTRUCT: u32 = 0x0041_0360;
/// `ExtraDataList::DuplicateExtraListForContainer` (Xbox PDB).
pub(crate) const EXTRA_DATA_LIST_DUPLICATE: u32 = 0x0041_2380;
/// `BaseExtraList::RemoveAll(bool)` (Xbox PDB).
pub(crate) const EXTRA_REMOVE_ALL: u32 = 0x0040_fae0;
/// `BaseExtraList::ItemsInList` (Xbox PDB).
pub(crate) const EXTRA_ITEMS_IN_LIST: u32 = 0x0040_fe20;
/// `BaseExtraList::RemoveExtra(extra, bool)` (Xbox PDB).
pub(crate) const EXTRA_REMOVE_EXTRA: u32 = 0x0041_0020;
/// `ExtraDataList` accessors (Xbox PDB names where the map has one).
pub(crate) const EXTRA_IS_DEFAULT_FOR_CONTAINER: u32 = 0x0041_d120;
pub(crate) const EXTRA_GET_WORN: u32 = 0x0041_8ab0;
pub(crate) const EXTRA_GET_COUNT: u32 = 0x0041_8770;
pub(crate) const EXTRA_REMOVE_COUNT: u32 = 0x0041_af10;
pub(crate) const EXTRA_HAS_LEVELED_ITEM: u32 = 0x0041_8750;
pub(crate) const EXTRA_GET_LEVELED_ITEM: u32 = 0x0041_8720;
/// `ExtraDataList::GetOriginalReference` / `RemoveOriginalReferenceExtra`.
pub(crate) const EXTRA_GET_ORIGINAL_REFERENCE: u32 = 0x0041_8630;
pub(crate) const EXTRA_REMOVE_ORIGINAL_REFERENCE: u32 = 0x0041_8600;
/// The unnamed `ExtraDataList` test used by `004bccb0`.
pub(crate) const EXTRA_TEST_D030: u32 = 0x0041_d030;
/// `ExtraDataList::GetHealth` / `SetHealth` / `RemoveHealth` (Xbox PDB), and
/// the setter `00419c60` that `SetItemHealth` uses on a new list.
pub(crate) const EXTRA_GET_HEALTH: u32 = 0x0041_86f0;
pub(crate) const EXTRA_SET_HEALTH: u32 = 0x0041_9970;
pub(crate) const EXTRA_SET_HEALTH_NEW: u32 = 0x0041_9c60;
pub(crate) const EXTRA_REMOVE_HEALTH: u32 = 0x0041_aef0;
/// `00418660` (the "ownership" extra), `004187a0` (a float extra, `-1.0`
/// when absent), `004187d0` (`ExtraDataList::GetPoison`), `0041af30` (removes
/// it), `00419d10` (`ExtraDataList::SetRank`), `00419700` (copies another
/// list's extras into this one).
pub(crate) const EXTRA_GET_OWNERSHIP: u32 = 0x0041_8660;
pub(crate) const EXTRA_GET_FLOAT_EXTRA: u32 = 0x0041_87a0;
pub(crate) const EXTRA_GET_POISON: u32 = 0x0041_87d0;
pub(crate) const EXTRA_REMOVE_POISON: u32 = 0x0041_af30;
pub(crate) const EXTRA_SET_RANK: u32 = 0x0041_9d10;
pub(crate) const EXTRA_COPY_FROM: u32 = 0x0041_9700;
/// `ExtraDataList::GetHotKey`, `GetScript`, `GetScriptLocals`.
pub(crate) const EXTRA_GET_HOT_KEY: u32 = 0x0042_de90;
pub(crate) const EXTRA_GET_SCRIPT: u32 = 0x0041_8800;
pub(crate) const EXTRA_GET_SCRIPT_LOCALS: u32 = 0x0041_8830;
/// `ExtraDataList::HasWeaponMods`, `GetWeaponModFlags`,
/// `GetWeaponModSlotActive(slot)`, `SetIsModding(bool)`, `RemoveIsModding`.
pub(crate) const EXTRA_HAS_WEAPON_MODS: u32 = 0x0041_8ba0;
pub(crate) const EXTRA_GET_WEAPON_MOD_FLAGS: u32 = 0x0042_e560;
pub(crate) const EXTRA_GET_WEAPON_MOD_SLOT_ACTIVE: u32 = 0x0041_8c00;
pub(crate) const EXTRA_SET_IS_MODDING: u32 = 0x0042_e5a0;
pub(crate) const EXTRA_REMOVE_IS_MODDING: u32 = 0x0042_e6f0;
/// `TESHealthForm::GetFormAsHealthForm(form)` (`__cdecl`) and
/// `TESHealthForm::GetFormHealth(form)` (`__cdecl`), `TESValueForm::GetFormValue`.
pub(crate) const GET_FORM_AS_HEALTH_FORM: u32 = 0x0048_72e0;
pub(crate) const GET_FORM_HEALTH: u32 = 0x0048_73d0;
pub(crate) const GET_FORM_VALUE: u32 = 0x0048_e8a0;
/// `min(a, b)` on floats (`__cdecl`, result in ST0).
pub(crate) const FLOAT_MIN: u32 = 0x0040_ebd0;
/// `InventoryChanges::GetObjectInList(form, 1, 0)` (Xbox PDB).
pub(crate) const INVENTORY_CHANGES_GET_OBJECT_IN_LIST: u32 = 0x004b_fba0;
/// `CombatFormulas::CalcWeaponDamage` and `CalcArmorRating` (Xbox PDB,
/// `__cdecl`), and the price formula `00647c00(value, health)`.
pub(crate) const CALC_WEAPON_DAMAGE: u32 = 0x0064_4ce0;
pub(crate) const CALC_ARMOR_RATING: u32 = 0x0064_6360;
pub(crate) const CALC_ITEM_PRICE: u32 = 0x0064_7c00;
/// `004be060(armorForm)`: the armor's rating as a float (it calls `004be080`).
pub(crate) const ARMOR_RATING_FLOAT: u32 = 0x004b_e060;
/// `00476b20(float)` (`__cdecl`): the rounding applied to the armor result.
pub(crate) const ROUND_ARMOR_RESULT: u32 = 0x0047_6b20;
/// `TESActorBaseData::GetFatigue` (the map's name, source `vt`): a 16-bit
/// value read from the enchantable form.
pub(crate) const ENCHANTABLE_FORM_VALUE: u32 = 0x004a_8ae0;

/// RTTI type descriptors the casts use (names from the `.?AV...@@` strings).
pub(crate) const TYPE_TES_BOUND_OBJECT: u32 = 0x0118_3108;
pub(crate) const TYPE_TES_OBJECT_WEAP: u32 = 0x0118_3998;
pub(crate) const TYPE_TES_ENCHANTABLE_FORM: u32 = 0x0118_39b4;
pub(crate) const TYPE_TES_HEALTH_FORM: u32 = 0x0118_6c3c;

/// `-1.0f` (the "no value" float).
pub(crate) const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
/// `-1.0` as a double (the comparison constant).
pub(crate) const MINUS_ONE_DOUBLE: u32 = 0x0101_a6b0;
/// `100.0f`.
pub(crate) const HUNDRED_FLOAT: u32 = 0x0101_6410;
/// `100.0` as a double.
pub(crate) const HUNDRED_DOUBLE: u32 = 0x0101_7a40;
/// `0.5` as a double (the rounding threshold of `004bd510`).
pub(crate) const HALF_DOUBLE: u32 = 0x0101_1588;
/// The float multiple `GetItemValue` rounds the price to (`0.1f`).
pub(crate) const PRICE_STEP: u32 = 0x0101_e2bc;

/// `TESObjectWEAP` fields (PC offsets; the Xbox PDB offsets are 0x10 higher
/// up to here): `TESHealthForm` sub-object at +0x94 (its `iHealth` is the
/// word at +4 of it), `OBJ_WEAP::eModActionOne..Three` at +0x180, +0x184,
/// +0x188, `fModActionOneValue..` at +0x18c, +0x190, +0x194 and
/// `fModActionOneValueTwo..` at +0x1ac, +0x1b0, +0x1b4,
/// `pModObjectOne..Three` at +0x350, +0x354, +0x358.
pub(crate) const WEAPON_HEALTH_FORM: u32 = 0x94;
pub(crate) const WEAPON_MOD_ACTION: u32 = 0x180;
pub(crate) const WEAPON_MOD_ACTION_VALUE: u32 = 0x18c;
pub(crate) const WEAPON_MOD_ACTION_VALUE_TWO: u32 = 0x1ac;
pub(crate) const WEAPON_MOD_OBJECT: u32 = 0x350;
/// `TESObjectIMOD`: the value word read at +0x7c + 4 of a mod object.
pub(crate) const MOD_OBJECT_VALUE: u32 = 0x7c;
/// The weapon mod effect type `10` that `GetItemHealth` asks about.
pub(crate) const MOD_EFFECT_HEALTH: u32 = 10;

layout! {
    /// `ItemChange` (Xbox PDB), 0xC bytes: one stack of an item in an
    /// inventory.
    pub struct ItemChange: 0x0C {
        /// `pExtraObjectList` (Xbox PDB): `BSSimpleList<ExtraDataList *>*`.
        0x00 pExtraObjectList: Ptr,
        /// `iNumber` (Xbox PDB).
        0x04 iNumber: i32,
        /// `pContainerObj` (Xbox PDB): `TESBoundObject*`.
        0x08 pContainerObj: Ptr,
    }
}

/// The head node of the extra list list of `this`.
fn list_head(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    e.get(this, ItemChange::pExtraObjectList).addr()
}

/// The item's form (`pContainerObj`).
fn container_object(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    e.get(this, ItemChange::pContainerObj).addr()
}

/// The first extra data list of `this`, or 0: the head node is nonnull and its
/// item word is set (the check every accessor of the unit starts with).
fn first_item(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let node = list_head(e, this);
    if node == 0 {
        return 0;
    }
    list_item(e, node)
}

/// `delete object` through its scalar deleting destructor (slot 0), nothing
/// for null.
fn delete_object(e: &mut Engine, object: u32) {
    if object != 0 {
        e.vcall(object, 0, &args![1u32]);
    }
}

/// Calls a list method that takes the address of an item word
/// (`list->Method(&item)`).
fn list_call_with_item(e: &mut Engine, method: u32, list: u32, item: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        e.call(method, &args![list, slot]);
    });
}

/// `new ExtraDataList`: 0x20 bytes, constructed unless the allocation failed.
fn new_extra_list(e: &mut Engine) -> u32 {
    let extra = e.call(OPERATOR_NEW, &args![0x20u32]).u32();
    if extra == 0 {
        return 0;
    }
    e.call(EXTRA_DATA_LIST_CONSTRUCT, &args![extra]).u32()
}

/// `new BSSimpleList` node: 8 bytes, constructed unless the allocation
/// failed.
fn new_list_node(e: &mut Engine) -> u32 {
    let node = e.call(OPERATOR_NEW, &args![8u32]).u32();
    if node == 0 {
        return 0;
    }
    e.call(LIST_NODE_CONSTRUCT, &args![node]).u32()
}

/// `_ftol2_sse` on a float held in ST0.
fn float_to_int(e: &mut Engine, value: f64) -> i32 {
    e.call(FTOL, &args![value]).i32()
}

/// `FISTP` with the truncating rounding mode set: out of range (and NaN)
/// values give the "integer indefinite" `0x80000000`.
fn truncate_to_i32(value: f64) -> i32 {
    let t = value.trunc();
    if t.is_nan() || !(-2147483648.0..2147483648.0).contains(&t) {
        i32::MIN
    } else {
        t as i32
    }
}

// Translated from 004bc550 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::ItemChange` (Xbox PDB): allocates an empty list node for the
/// extra lists and stores the item's form and the count.
pub fn item_change_item_change(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    container_obj: Ptr,
    number: i32,
) -> Ptr<ItemChange> {
    e.set(this, ItemChange::pContainerObj, container_obj);
    let node = new_list_node(e);
    e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
    e.set(this, ItemChange::iNumber, number);
    this
}

// Translated from 004bc5f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases the list of extra data lists: clears it (`00470470`), deletes
/// the list (`004702f0(list, 1)`) and nulls the field. The extra lists
/// themselves are not deleted.
pub fn fn_004bc5f0(e: &mut Engine, this: Ptr<ItemChange>) {
    let list = list_head(e, this);
    if list != 0 {
        e.call(LIST_CLEAR, &args![list]);
        let list = list_head(e, this);
        if list != 0 {
            e.call(LIST_DESTROY, &args![list, 1u32]);
        }
    }
    e.set(this, ItemChange::pExtraObjectList, Ptr::NULL);
}

// Translated from 004bc650 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::ItemChange_ov2` (Xbox PDB), the copy constructor: copies the
/// form, makes an empty list and appends a duplicate
/// (`DuplicateExtraListForContainer`) of every extra list of `other`, then
/// copies the count.
pub fn item_change_item_change_ov2(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    other: Ptr<ItemChange>,
) -> Ptr<ItemChange> {
    let form = container_object(e, other);
    e.set(this, ItemChange::pContainerObj, Ptr::new(form));
    let node = new_list_node(e);
    e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
    let mut cursor = list_head(e, other);
    while cursor != 0 {
        let source = list_item(e, cursor);
        if source == 0 {
            break;
        }
        let copy = new_extra_list(e);
        e.call(EXTRA_DATA_LIST_DUPLICATE, &args![copy, source]);
        let list = list_head(e, this);
        list_call_with_item(e, LIST_ADD_TAIL, list, copy);
        cursor = list_next(e, cursor);
    }
    let number = e.get(other, ItemChange::iNumber);
    e.set(this, ItemChange::iNumber, number);
    this
}

// Translated from 004bc780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::DeleteAllExtra` (Xbox PDB): for every extra list, removes all
/// its extras (`RemoveAll(true)`), takes it out of the list (the removal is
/// started from the current node, as the game does) and deletes it.
pub fn item_change_delete_all_extra(e: &mut Engine, this: Ptr<ItemChange>) {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        e.call(EXTRA_REMOVE_ALL, &args![extra, 1u32]);
        list_call_with_item(e, LIST_REMOVE, cursor, extra);
        delete_object(e, extra);
        cursor = list_next(e, cursor);
    }
}

// Translated from 004bc810 (decompiled, FalloutNV.exe 1.4.0.525)
/// Shallow copy constructor from another `ItemChange`: when `other` is not
/// null, copies its form, makes an empty list and appends the same extra
/// list pointers (no duplicates) of `other`'s list.
pub fn fn_004bc810(e: &mut Engine, this: Ptr<ItemChange>, other: Ptr<ItemChange>) {
    if other.is_null() {
        return;
    }
    let form = container_object(e, other);
    e.set(this, ItemChange::pContainerObj, Ptr::new(form));
    let node = new_list_node(e);
    e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
    let mut cursor = e.call(WORD_AT_0, &args![other]).u32();
    while cursor != 0 {
        let item = list_item(e, cursor);
        if item == 0 {
            break;
        }
        let list = e.call(WORD_AT_0, &args![this]).u32();
        list_call_with_item(e, LIST_ADD, list, item);
        cursor = list_next(e, cursor);
    }
}

// Translated from 004bc8f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetExtraTotalCount` (Xbox PDB): the sum of the counts of the
/// extra lists that are not the container's default one and, unless
/// `include_worn`, are not worn.
pub fn item_change_get_extra_total_count(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    include_worn: bool,
) -> i32 {
    let mut cursor = list_head(e, this);
    let mut total = 0i32;
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        let is_default = e
            .call(EXTRA_IS_DEFAULT_FOR_CONTAINER, &args![extra, 0u32])
            .bool();
        if !is_default && (include_worn || !e.call(EXTRA_GET_WORN, &args![extra, 0u32]).bool()) {
            let count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16;
            total += count as i32;
        }
        cursor = list_next(e, cursor);
    }
    total
}

// Translated from 004bc980 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetExtraTotalDefaultCount` (Xbox PDB): the sum of the counts
/// of the container's default extra lists. A list left without any extras
/// (`fn_004bca60`) is removed and deleted, and the walk starts over with a
/// total of zero.
pub fn item_change_get_extra_total_default_count(e: &mut Engine, this: Ptr<ItemChange>) -> i32 {
    let mut cursor = list_head(e, this);
    let mut total = 0i32;
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_IS_DEFAULT_FOR_CONTAINER, &args![extra, 0u32])
            .bool()
        {
            let count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16;
            total += count as i32;
        }
        cursor = list_next(e, cursor);
        if fn_004bca60(e, Ptr::new(extra)) {
            let list = list_head(e, this);
            list_call_with_item(e, LIST_REMOVE, list, extra);
            delete_object(e, extra);
            total = 0;
            cursor = list_head(e, this);
        }
    }
    total
}

// Translated from 004bca60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether an extra data list has no extras (its first extra pointer, +4, is
/// null).
pub fn fn_004bca60(e: &mut Engine, extra_list: Ptr) -> bool {
    e.mem.u32(extra_list.addr() + 4) == 0 // BaseExtraList::pHead
}

// Translated from 004bca80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetAmountNonDefaultExtra` (Xbox PDB): the number of extra
/// lists that are not the container's default and have a positive count. A
/// list that holds one item type (`ItemsInList == 1`) with a count above 1
/// has its count removed, is taken out of the list and deleted, and the walk
/// starts over with a total of zero.
pub fn item_change_get_amount_non_default_extra(e: &mut Engine, this: Ptr<ItemChange>) -> i32 {
    let mut cursor = list_head(e, this);
    let mut total = 0i32;
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        let count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16;
        let is_default = e
            .call(EXTRA_IS_DEFAULT_FOR_CONTAINER, &args![extra, 0u32])
            .bool();
        if !is_default && count > 0 {
            total += 1;
        }
        cursor = list_next(e, cursor);
        if e.call(EXTRA_ITEMS_IN_LIST, &args![extra]).i32() == 1 && count > 1 {
            e.call(EXTRA_REMOVE_COUNT, &args![extra]);
            let list = list_head(e, this);
            list_call_with_item(e, LIST_REMOVE, list, extra);
            delete_object(e, extra);
            total = 0;
            cursor = list_head(e, this);
        }
    }
    total
}

// Translated from 004bcb70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether any extra list has a leveled item extra (`HasLeveledItem`).
pub fn fn_004bcb70(e: &mut Engine, this: Ptr<ItemChange>) -> bool {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_HAS_LEVELED_ITEM, &args![extra]).bool() {
            return true;
        }
        cursor = list_next(e, cursor);
    }
    false
}

// Translated from 004bcbd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::RemoveOriginalChanges` (Xbox PDB): removes the original
/// reference extra from every extra list that has one.
pub fn item_change_remove_original_changes(e: &mut Engine, this: Ptr<ItemChange>) {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_GET_ORIGINAL_REFERENCE, &args![extra]).u32() != 0 {
            e.call(EXTRA_REMOVE_ORIGINAL_REFERENCE, &args![extra]);
        }
        cursor = list_next(e, cursor);
    }
}

// Translated from 004bcc30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::NumberLeveledExtra` (Xbox PDB): the sum of the counts of the
/// extra lists that have a leveled item extra.
pub fn item_change_number_leveled_extra(e: &mut Engine, this: Ptr<ItemChange>) -> i32 {
    let mut cursor = list_head(e, this);
    let mut total = 0i32;
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_HAS_LEVELED_ITEM, &args![extra]).bool() {
            let count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16;
            total += count as i32;
        }
        cursor = list_next(e, cursor);
    }
    total
}

// Translated from 004bccb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether any extra list passes the test `0041d030` (an `ExtraDataList`
/// predicate the map does not name).
pub fn fn_004bccb0(e: &mut Engine, this: Ptr<ItemChange>) -> bool {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_TEST_D030, &args![extra]).bool() {
            return true;
        }
        cursor = list_next(e, cursor);
    }
    false
}

// Translated from 004bcd10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::RemoveLeveledItemExtra` (Xbox PDB): removes the leveled item
/// extra from every extra list that has one; a list left without extras is
/// taken out of the list (not deleted) and the walk resumes after the head.
/// Returns the last leveled item extra it removed.
pub fn item_change_remove_leveled_item_extra(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let mut cursor = list_head(e, this);
    let mut removed = 0u32;
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_GET_LEVELED_ITEM, &args![extra]).u32() != 0 {
            removed = e.call(EXTRA_GET_LEVELED_ITEM, &args![extra]).u32();
            e.call(EXTRA_REMOVE_EXTRA, &args![extra, removed, 0u32]);
            if fn_004bca60(e, Ptr::new(extra)) {
                let list = list_head(e, this);
                list_call_with_item(e, LIST_REMOVE, list, extra);
            }
            cursor = list_head(e, this);
        }
        cursor = list_next(e, cursor);
    }
    removed
}

// Translated from 004bcdb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetItemHealth` (Xbox PDB): the health of the item. The
/// base health is the form's (`GetFormHealth` through the health form's
/// virtual slot 0x10), or for a weapon `TESObjectWEAP::GetFormHealth` with
/// the mod effect flag. With `as_percent` false the first extra list's health
/// if it has one, else the base health; with it true the percentage of the
/// base health (the ratio limited to 100, times 100) or 100 without a
/// health extra.
/// `-1.0` for a form without health.
pub fn item_change_get_item_health(e: &mut Engine, this: Ptr<ItemChange>, as_percent: bool) -> f32 {
    let form = container_object(e, this);
    let health_form = e.call(GET_FORM_AS_HEALTH_FORM, &args![form]).u32();
    if health_form == 0 {
        return e.global::<f32>(MINUS_ONE_FLOAT);
    }
    let weapon = e
        .call(
            RT_DYNAMIC_CAST,
            &args![
                form,
                0u32,
                TYPE_TES_BOUND_OBJECT,
                TYPE_TES_OBJECT_WEAP,
                0u32
            ],
        )
        .u32();
    let mut base_health = e.vcall(health_form, 0x10, &args![]).i32();
    if weapon != 0 {
        let mod_active = item_change_has_mod_effect_active_ov2(e, this, MOD_EFFECT_HEALTH as u8);
        base_health = tes_object_weap_get_form_health(e, Ptr::new(weapon), mod_active as u8);
    }
    let extra = first_item(e, this);
    if extra != 0 {
        let health = e.call(EXTRA_GET_HEALTH, &args![extra]).f32();
        let none: f64 = e.global(MINUS_ONE_DOUBLE);
        if health as f64 != none {
            if !as_percent {
                return health;
            }
            let ratio = (health as f64 / base_health as f64) as f32;
            let cap: f32 = e.global(HUNDRED_FLOAT);
            let limited = e.call(FLOAT_MIN, &args![cap, ratio]).f32();
            let hundred: f64 = e.global(HUNDRED_DOUBLE);
            return (limited as f64 * hundred) as f32;
        }
    }
    if as_percent {
        e.global::<f32>(HUNDRED_FLOAT)
    } else {
        base_health as f32
    }
}

// Translated from 004bcf00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectWEAP::GetFormHealth` (Xbox PDB): the weapon's health, with
/// `with_mods` plus the health mod effect value (type 10), truncated.
pub fn tes_object_weap_get_form_health(e: &mut Engine, this: Ptr, with_mods: u8) -> i32 {
    // TESHealthForm sub-object at +0x94, TESHealthForm::iHealth (Xbox PDB).
    let health_form = this.addr() + WEAPON_HEALTH_FORM;
    if with_mods != 0 {
        let bonus = tes_object_weap_get_mod_effect_value(e, this, MOD_EFFECT_HEALTH, 0);
        let health = e.call(WORD_AT_4, &args![health_form]).u32();
        let total = health as f64 + bonus as f64;
        float_to_int(e, total)
    } else {
        e.call(WORD_AT_4, &args![health_form]).i32()
    }
}

// Translated from 004bcf60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectWEAP::GetModEffectValue` (Xbox PDB): the value of the weapon's
/// mod action of type `effect`: the first (`variant` 0) or second (1) value
/// of the action slot that has that type, 0.0 for another variant or when no
/// slot has the type.
pub fn tes_object_weap_get_mod_effect_value(
    e: &mut Engine,
    this: Ptr,
    effect: u32,
    variant: u8,
) -> f32 {
    let base = this.addr();
    for slot in 0..3u32 {
        // OBJ_WEAP::eModActionOne..Three (Xbox PDB).
        if e.mem.u32(base + WEAPON_MOD_ACTION + 4 * slot) == effect {
            return match variant {
                // fModActionOneValue.. / fModActionOneValueTwo.. (Xbox PDB)
                0 => e.mem.f32(base + WEAPON_MOD_ACTION_VALUE + 4 * slot),
                1 => e.mem.f32(base + WEAPON_MOD_ACTION_VALUE_TWO + 4 * slot),
                _ => 0.0,
            };
        }
    }
    0.0
}

// Translated from 004bd030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::SetItemHealth` (Xbox PDB): sets the health of the extra list
/// `extra` (of this item) to `health`. `inventory_changes` is the owning
/// `InventoryChanges`; when its owner (+4, `pRef`) is set, that object gets
/// its virtual slot 0x48 called with 0x20. For a form with health: if the
/// new health is below the form's health or the current health is above it,
/// the health is stored in `extra` (found in the list, or in a new extra
/// list added to it; a new list node is made when there is none); otherwise
/// the health extra is removed from `extra`, and when that leaves the list
/// empty and `remove_empty` is set the list is taken out of the entry
/// `GetObjectInList` finds and deleted.
pub fn item_change_set_item_health(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    health: f32,
    inventory_changes: Ptr,
    extra: Ptr,
    remove_empty: bool,
) {
    let form = container_object(e, this);
    let health_form = e
        .call(
            RT_DYNAMIC_CAST,
            &args![
                form,
                0u32,
                TYPE_TES_BOUND_OBJECT,
                TYPE_TES_HEALTH_FORM,
                0u32
            ],
        )
        .u32();
    let form_health = e.call(GET_FORM_HEALTH, &args![form]).i32();
    let current = item_change_get_item_health(e, this, false);
    let current = float_to_int(e, current as f64);
    // InventoryChanges::pRef (Xbox PDB) +4.
    let owner = e.call(WORD_AT_4, &args![inventory_changes]).u32();
    if owner != 0 {
        e.vcall(owner, 0x48, &args![0x20u32]);
    }
    if health_form == 0 {
        return;
    }
    if (health as f64) < form_health as f64 || current > form_health {
        let list = list_head(e, this);
        if list == 0 {
            let new_extra = new_extra_list(e);
            let node = new_list_node(e);
            e.call(EXTRA_SET_HEALTH_NEW, &args![new_extra, health]);
            list_call_with_item(e, LIST_ADD, node, new_extra);
            e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
            return;
        }
        let mut cursor = list;
        while cursor != 0 {
            let item = list_item(e, cursor);
            if item == 0 {
                break;
            }
            if item == extra.addr() {
                e.call(EXTRA_SET_HEALTH, &args![item, health]);
                return;
            }
            cursor = list_next(e, cursor);
        }
        let new_extra = new_extra_list(e);
        e.call(EXTRA_SET_HEALTH, &args![new_extra, health]);
        let list = list_head(e, this);
        list_call_with_item(e, LIST_ADD, list, new_extra);
    } else {
        let mut cursor = list_head(e, this);
        while cursor != 0 {
            let item = list_item(e, cursor);
            if item == 0 {
                break;
            }
            if item == extra.addr() {
                e.call(EXTRA_REMOVE_HEALTH, &args![item]);
                if fn_004bca60(e, Ptr::new(item)) && remove_empty {
                    let form = e.call(WORD_AT_8, &args![this]).u32();
                    let entry = e
                        .call(
                            INVENTORY_CHANGES_GET_OBJECT_IN_LIST,
                            &args![inventory_changes, form, 1u32, 0u32],
                        )
                        .u32();
                    let entry_list = e.call(WORD_AT_0, &args![entry]).u32();
                    list_call_with_item(e, LIST_REMOVE, entry_list, item);
                    delete_object(e, item);
                }
                return;
            }
            cursor = list_next(e, cursor);
        }
    }
}

// Translated from 004bd350 (decompiled, FalloutNV.exe 1.4.0.525)
/// A float for the item: when its form is an enchantable form (the cast to
/// `TESEnchantableForm`) the first extra list's float extra (`004187a0`) if
/// it has one (not `-1.0`), else the form's 16-bit value (`004a8ae0`); `-1.0`
/// for a form that is not enchantable.
pub fn fn_004bd350(e: &mut Engine, this: Ptr<ItemChange>) -> f32 {
    let form = container_object(e, this);
    let enchantable = e
        .call(
            RT_DYNAMIC_CAST,
            &args![
                form,
                0u32,
                TYPE_TES_BOUND_OBJECT,
                TYPE_TES_ENCHANTABLE_FORM,
                0u32
            ],
        )
        .u32();
    if enchantable == 0 {
        return e.global::<f32>(MINUS_ONE_FLOAT);
    }
    let extra = first_item(e, this);
    if extra != 0 {
        let value = e.call(EXTRA_GET_FLOAT_EXTRA, &args![extra]).f32();
        let none: f64 = e.global(MINUS_ONE_DOUBLE);
        if value as f64 != none {
            return e.call(EXTRA_GET_FLOAT_EXTRA, &args![extra]).f32();
        }
    }
    e.call(ENCHANTABLE_FORM_VALUE, &args![enchantable]).u16() as f32
}

// Translated from 004bd400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetItemValue` (Xbox PDB): the item's price: the form's value
/// (`GetFormValue`) scaled by the item's health percentage
/// (`00647c00(value, health)`), plus for a weapon the value word of each
/// mod object whose slot bit (1, 2, 4) is set in `GetModSlots`, rounded
/// with `fn_004bd510(price, 0.1)`.
pub fn item_change_get_item_value(e: &mut Engine, this: Ptr<ItemChange>) -> f32 {
    let health = item_change_get_item_health(e, this, true);
    let form = container_object(e, this);
    let value = e.call(GET_FORM_VALUE, &args![form]).i32();
    let mut price = e.call(CALC_ITEM_PRICE, &args![value as f32, health]).f32();
    let form = e.call(WORD_AT_8, &args![this]).u32();
    let weapon = e
        .call(
            RT_DYNAMIC_CAST,
            &args![
                form,
                0u32,
                TYPE_TES_BOUND_OBJECT,
                TYPE_TES_OBJECT_WEAP,
                0u32
            ],
        )
        .u32();
    if weapon != 0 {
        let slots = item_change_get_mod_slots(e, this) as u32;
        if slots != 0 {
            for bit in [1u32, 2, 4] {
                if slots & bit != 0 {
                    let mod_object = fn_004bd570(e, Ptr::new(weapon), bit);
                    // TESObjectIMOD value word.
                    let word = e
                        .call(WORD_AT_4, &args![mod_object + MOD_OBJECT_VALUE])
                        .i32();
                    price = (word as f64 + price as f64) as f32;
                }
            }
        }
    }
    let step: f32 = e.global(PRICE_STEP);
    fn_004bd510(e, price, step)
}

// Translated from 004bd510 (decompiled, FalloutNV.exe 1.4.0.525)
/// Rounds `value` to the nearest multiple of `step`: `value / step` is
/// truncated and one is added when the fraction is at least 0.5.
pub fn fn_004bd510(e: &mut Engine, value: f32, step: f32) -> f32 {
    let quotient = (value as f64 / step as f64) as f32;
    let whole = float_to_int(e, quotient as f64);
    let fraction = quotient as f64 - whole as f64;
    let half: f64 = e.global(HALF_DOUBLE);
    // `fcomp`; the carry is only left out when the fraction is below 0.5.
    let carry = if fraction < half { 0 } else { 1 };
    let rounded = float_to_int(e, quotient as f64) + carry;
    (rounded as f64 * step as f64) as f32
}

// Translated from 004bd570 (decompiled, FalloutNV.exe 1.4.0.525)
/// The weapon's mod object for slot bit 1, 2 or 4 (`pModObjectOne..Three`,
/// Xbox PDB), else 0.
pub fn fn_004bd570(e: &mut Engine, this: Ptr, slot_bit: u32) -> u32 {
    let index = match slot_bit {
        1 => 0,
        2 => 1,
        4 => 2,
        _ => return 0,
    };
    e.mem.u32(this.addr() + WEAPON_MOD_OBJECT + 4 * index)
}

// Translated from 004bd5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// With a source extra list `source`: copies it (`00419700`) into the first
/// extra list, or into a new extra list that is added to the list (made
/// first when there is none). Without one: removes the first extra list
/// from the list and deletes it.
pub fn fn_004bd5c0(e: &mut Engine, this: Ptr<ItemChange>, source: Ptr) {
    if !source.is_null() {
        let list = list_head(e, this);
        if list != 0 {
            let first = list_item(e, list);
            if first != 0 {
                e.call(EXTRA_COPY_FROM, &args![first, source]);
                return;
            }
        }
        let new_extra = new_extra_list(e);
        e.call(EXTRA_COPY_FROM, &args![new_extra, source]);
        if list_head(e, this) == 0 {
            let node = new_list_node(e);
            e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
        }
        let list = list_head(e, this);
        list_call_with_item(e, LIST_ADD, list, new_extra);
    } else {
        let list = list_head(e, this);
        if list != 0 {
            let first = list_item(e, list);
            if first != 0 {
                list_call_with_item(e, LIST_REMOVE, list, first);
                delete_object(e, first);
            }
        }
    }
}

// Translated from 004bd740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetItemOwnership` (Xbox PDB): the first extra list's
/// `00418660` value, 0 when there is none.
pub fn item_change_get_item_ownership(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let extra = first_item(e, this);
    if extra != 0 && e.call(EXTRA_GET_OWNERSHIP, &args![extra]).u32() != 0 {
        return e.call(EXTRA_GET_OWNERSHIP, &args![extra]).u32();
    }
    0
}

// Translated from 004bd7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetHotKey` (Xbox PDB): the hot key (a signed byte) of the
/// first extra list that has one (not negative), else -1.
pub fn item_change_get_hot_key(e: &mut Engine, this: Ptr<ItemChange>) -> i32 {
    let mut hot_key = -1i32;
    let mut cursor = e.call(WORD_AT_0, &args![this]).u32();
    while hot_key < 0 && cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_GET_HOT_KEY, &args![extra]).u8() as i8 >= 0 {
            hot_key = e.call(EXTRA_GET_HOT_KEY, &args![extra]).u8() as i8 as i32;
        }
        cursor = list_next(e, cursor);
    }
    hot_key
}

// Translated from 004bd820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetModSlots` (Xbox PDB): the weapon mod flags of the first
/// extra list when it has weapon mods, else 0.
pub fn item_change_get_mod_slots(e: &mut Engine, this: Ptr<ItemChange>) -> u8 {
    let extra = first_item(e, this);
    if extra != 0 && e.call(EXTRA_HAS_WEAPON_MODS, &args![extra]).bool() {
        return e.call(EXTRA_GET_WEAPON_MOD_FLAGS, &args![extra]).u8();
    }
    0
}

// Translated from 004bd880 (decompiled, FalloutNV.exe 1.4.0.525)
/// The weapon's mod action type (`eModActionOne..Three`, Xbox PDB) of slot
/// bit 1, 2 or 4, else 0.
pub fn fn_004bd880(e: &mut Engine, this: Ptr, slot_bit: u32) -> u32 {
    let index = match slot_bit {
        1 => 0,
        2 => 1,
        4 => 2,
        _ => return 0,
    };
    e.mem.u32(this.addr() + WEAPON_MOD_ACTION + 4 * index)
}

// Translated from 004bd8d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::HasModEffectActive` (Xbox PDB): whether the first extra
/// list has an active weapon mod slot (1, 2, then 4) whose mod action type
/// is `effect`; the value of that action (`fn_004bd9d0`, first value) is
/// stored through `value_out`.
pub fn item_change_has_mod_effect_active(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    effect: u8,
    value_out: Ptr,
) -> bool {
    let extra = first_item(e, this);
    if extra == 0 {
        return false;
    }
    let weapon = container_object(e, this);
    for slot_bit in [1u32, 2, 4] {
        if e.call(EXTRA_GET_WEAPON_MOD_SLOT_ACTIVE, &args![extra, slot_bit])
            .bool()
            && fn_004bd880(e, Ptr::new(weapon), slot_bit) == effect as u32
        {
            let value = fn_004bd9d0(e, Ptr::new(weapon), slot_bit, 0);
            e.mem.set_f32(value_out.addr(), value);
            return true;
        }
    }
    false
}

// Translated from 004bd9d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The weapon's mod action value (`fModActionOneValue..` for `variant` 0,
/// `fModActionOneValueTwo..` for 1; Xbox PDB) of slot bit 1, 2 or 4, else 0.0.
pub fn fn_004bd9d0(e: &mut Engine, this: Ptr, slot_bit: u32, variant: u8) -> f32 {
    let index = match slot_bit {
        1 => 0,
        2 => 1,
        4 => 2,
        _ => return 0.0,
    };
    match variant {
        0 => e.mem.f32(this.addr() + WEAPON_MOD_ACTION_VALUE + 4 * index),
        1 => e
            .mem
            .f32(this.addr() + WEAPON_MOD_ACTION_VALUE_TWO + 4 * index),
        _ => 0.0,
    }
}

// Translated from 004bda70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::HasModEffectActive_ov2` (Xbox PDB): as `HasModEffectActive`
/// without the value.
pub fn item_change_has_mod_effect_active_ov2(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    effect: u8,
) -> bool {
    let extra = first_item(e, this);
    if extra == 0 {
        return false;
    }
    let weapon = container_object(e, this);
    for slot_bit in [1u32, 2, 4] {
        if e.call(EXTRA_GET_WEAPON_MOD_SLOT_ACTIVE, &args![extra, slot_bit])
            .bool()
            && fn_004bd880(e, Ptr::new(weapon), slot_bit) == effect as u32
        {
            return true;
        }
    }
    false
}

// Translated from 004bdb40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::SetIsModding` (Xbox PDB): sets the modding flag on the first
/// extra list; when there is none, makes the list node (if the item has no
/// list) and a new extra list (added to it) first.
pub fn item_change_set_is_modding(e: &mut Engine, this: Ptr<ItemChange>) {
    let mut list = list_head(e, this);
    if list == 0 {
        list = new_list_node(e);
        e.set(this, ItemChange::pExtraObjectList, Ptr::new(list));
    }
    let first = list_item(e, list);
    if first == 0 {
        let extra = new_extra_list(e);
        e.call(EXTRA_SET_IS_MODDING, &args![extra, 1u32]);
        let list = list_head(e, this);
        list_call_with_item(e, LIST_ADD, list, extra);
    } else {
        e.call(EXTRA_SET_IS_MODDING, &args![first, 1u32]);
    }
}

// Translated from 004bdc70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::RemoveIsModding` (Xbox PDB): removes the modding flag from
/// the first extra list.
pub fn item_change_remove_is_modding(e: &mut Engine, this: Ptr<ItemChange>) {
    let extra = first_item(e, this);
    if extra != 0 {
        e.call(EXTRA_REMOVE_IS_MODDING, &args![extra]);
    }
}

// Translated from 004bdcc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetPoison` (Xbox PDB): the poison of the first extra list,
/// 0 when there is none.
pub fn item_change_get_poison(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let extra = first_item(e, this);
    if extra != 0 && e.call(EXTRA_GET_POISON, &args![extra]).u32() != 0 {
        return e.call(EXTRA_GET_POISON, &args![extra]).u32();
    }
    0
}

// Translated from 004bdd20 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the first extra list has no poison, calls `00419d10` (the map's
/// `ExtraDataList::SetRank`) on it with `value`. The game's function returns
/// whatever is left in `EAX`.
pub fn fn_004bdd20(e: &mut Engine, this: Ptr<ItemChange>, value: u32) {
    let extra = first_item(e, this);
    if extra != 0 && e.call(EXTRA_GET_POISON, &args![extra]).u32() == 0 {
        e.call(EXTRA_SET_RANK, &args![extra, value]);
    }
}

// Translated from 004bdd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the first extra list has a poison, removes it (`0041af30`).
pub fn fn_004bdd80(e: &mut Engine, this: Ptr<ItemChange>) {
    let extra = first_item(e, this);
    if extra != 0 && e.call(EXTRA_GET_POISON, &args![extra]).u32() != 0 {
        e.call(EXTRA_REMOVE_POISON, &args![extra]);
    }
}

// Translated from 004bddd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetWorn` (Xbox PDB): whether any extra list is worn
/// (`ExtraDataList::GetWorn(flag)`).
pub fn item_change_get_worn(e: &mut Engine, this: Ptr<ItemChange>, flag: u8) -> bool {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_GET_WORN, &args![extra, flag as u32]).bool() {
            return true;
        }
        cursor = list_next(e, cursor);
    }
    false
}

// Translated from 004bde40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetScript` (Xbox PDB): the first script found on an extra
/// list, 0 when none.
pub fn item_change_get_script(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_GET_SCRIPT, &args![extra]).u32() != 0 {
            return e.call(EXTRA_GET_SCRIPT, &args![extra]).u32();
        }
        cursor = list_next(e, cursor);
    }
    0
}

// Translated from 004bdea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetScriptLocals` (Xbox PDB): the first script locals found
/// on an extra list, 0 when none.
pub fn item_change_get_script_locals(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_GET_SCRIPT_LOCALS, &args![extra]).u32() != 0 {
            return e.call(EXTRA_GET_SCRIPT_LOCALS, &args![extra]).u32();
        }
        cursor = list_next(e, cursor);
    }
    0
}

// Translated from 004bdf00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetModifiedAttackDamage` (Xbox PDB): for a form of type
/// 0x28, `CalcWeaponDamage(attacker, form, health ratio, scale, 0, flag,
/// this, 1)` where the ratio is the item's health over the form's health;
/// 0.0 for any other form.
pub fn item_change_get_modified_attack_damage(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    attacker: Ptr,
    scale: f32,
    flag: u8,
) -> f32 {
    let form = container_object(e, this);
    let form_type = e.call(FORM_TYPE, &args![form]).u32();
    if form_type == 0x28 {
        let health = item_change_get_item_health(e, this, false);
        let base = e.call(GET_FORM_HEALTH, &args![form]).u32();
        let ratio = (health as f64 / base as f64) as f32;
        e.call(
            CALC_WEAPON_DAMAGE,
            &args![attacker, form, ratio, scale, 0u32, flag as u32, this, 1u32],
        )
        .f32()
    } else {
        e.call(FORM_TYPE, &args![form]);
        0.0
    }
}

// Translated from 004bdf90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The armor rating of the item: for a form of type 0x18, the form's rating
/// (`004be060`) truncated, with the item's health ratio, through
/// `CalcArmorRating(rating, ratio)`; the result (or `-1.0` for another form)
/// goes through `00476b20`. The parameter is not read. (The game also
/// pushes the ratio for `004be060`, which does not take it; that word is the
/// second argument of `CalcArmorRating`.)
pub fn fn_004bdf90(e: &mut Engine, this: Ptr<ItemChange>, _unused_1: u32) -> f32 {
    let mut result = e.global::<f32>(MINUS_ONE_FLOAT);
    let form = container_object(e, this);
    if e.call(FORM_TYPE, &args![form]).u32() == 0x18 {
        let form_health = e.call(GET_FORM_HEALTH, &args![form]).u32() as f32;
        let ratio = if form_health == 0.0 {
            0.0f32
        } else {
            let health = item_change_get_item_health(e, this, false);
            (health as f64 / form_health as f64) as f32
        };
        let rating = e.call(ARMOR_RATING_FLOAT, &args![form]).f32();
        let whole = truncate_to_i32(rating as f64);
        result = e
            .call(CALC_ARMOR_RATING, &args![whole as u16 as u32, ratio])
            .f32();
    }
    e.call(ROUND_ARMOR_RESULT, &args![result]).f32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004bc550,
            item_change_item_change(Ptr<ItemChange>, Ptr, i32) -> Ptr<ItemChange>
        ),
        entry!(0x004bc5f0, fn_004bc5f0(Ptr<ItemChange>)),
        entry!(
            0x004bc650,
            item_change_item_change_ov2(Ptr<ItemChange>, Ptr<ItemChange>) -> Ptr<ItemChange>
        ),
        entry!(0x004bc780, item_change_delete_all_extra(Ptr<ItemChange>)),
        entry!(0x004bc810, fn_004bc810(Ptr<ItemChange>, Ptr<ItemChange>)),
        entry!(
            0x004bc8f0,
            item_change_get_extra_total_count(Ptr<ItemChange>, bool) -> i32
        ),
        entry!(
            0x004bc980,
            item_change_get_extra_total_default_count(Ptr<ItemChange>) -> i32
        ),
        entry!(0x004bca60, fn_004bca60(Ptr) -> bool),
        entry!(
            0x004bca80,
            item_change_get_amount_non_default_extra(Ptr<ItemChange>) -> i32
        ),
        entry!(0x004bcb70, fn_004bcb70(Ptr<ItemChange>) -> bool),
        entry!(
            0x004bcbd0,
            item_change_remove_original_changes(Ptr<ItemChange>)
        ),
        entry!(
            0x004bcc30,
            item_change_number_leveled_extra(Ptr<ItemChange>) -> i32
        ),
        entry!(0x004bccb0, fn_004bccb0(Ptr<ItemChange>) -> bool),
        entry!(
            0x004bcd10,
            item_change_remove_leveled_item_extra(Ptr<ItemChange>) -> u32
        ),
        entry!(
            0x004bcdb0,
            item_change_get_item_health(Ptr<ItemChange>, bool) -> f32
        ),
        entry!(0x004bcf00, tes_object_weap_get_form_health(Ptr, u8) -> i32),
        entry!(0x004bcf60, tes_object_weap_get_mod_effect_value(Ptr, u32, u8) -> f32),
        entry!(
            0x004bd030,
            item_change_set_item_health(Ptr<ItemChange>, f32, Ptr, Ptr, bool)
        ),
        entry!(0x004bd350, fn_004bd350(Ptr<ItemChange>) -> f32),
        entry!(
            0x004bd400,
            item_change_get_item_value(Ptr<ItemChange>) -> f32
        ),
        entry!(0x004bd510, fn_004bd510(f32, f32) -> f32),
        entry!(0x004bd570, fn_004bd570(Ptr, u32) -> u32),
        entry!(0x004bd5c0, fn_004bd5c0(Ptr<ItemChange>, Ptr)),
        entry!(
            0x004bd740,
            item_change_get_item_ownership(Ptr<ItemChange>) -> u32
        ),
        entry!(0x004bd7a0, item_change_get_hot_key(Ptr<ItemChange>) -> i32),
        entry!(0x004bd820, item_change_get_mod_slots(Ptr<ItemChange>) -> u8),
        entry!(0x004bd880, fn_004bd880(Ptr, u32) -> u32),
        entry!(
            0x004bd8d0,
            item_change_has_mod_effect_active(Ptr<ItemChange>, u8, Ptr) -> bool
        ),
        entry!(0x004bd9d0, fn_004bd9d0(Ptr, u32, u8) -> f32),
        entry!(
            0x004bda70,
            item_change_has_mod_effect_active_ov2(Ptr<ItemChange>, u8) -> bool
        ),
        entry!(0x004bdb40, item_change_set_is_modding(Ptr<ItemChange>)),
        entry!(0x004bdc70, item_change_remove_is_modding(Ptr<ItemChange>)),
        entry!(0x004bdcc0, item_change_get_poison(Ptr<ItemChange>) -> u32),
        entry!(0x004bdd20, fn_004bdd20(Ptr<ItemChange>, u32)),
        entry!(0x004bdd80, fn_004bdd80(Ptr<ItemChange>)),
        entry!(
            0x004bddd0,
            item_change_get_worn(Ptr<ItemChange>, u8) -> bool
        ),
        entry!(0x004bde40, item_change_get_script(Ptr<ItemChange>) -> u32),
        entry!(
            0x004bdea0,
            item_change_get_script_locals(Ptr<ItemChange>) -> u32
        ),
        entry!(
            0x004bdf00,
            item_change_get_modified_attack_damage(Ptr<ItemChange>, Ptr, f32, u8) -> f32
        ),
        entry!(0x004bdf90, fn_004bdf90(Ptr<ItemChange>, u32) -> f32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vtable of every fake object: slot 0 the scalar deleting destructor,
    /// slot 4 (+0x10) the health form's base health, slot 18 (+0x48) the
    /// owner's change notification.
    const VTABLE: u32 = 0x0200_0000;
    const DESTRUCTOR: u32 = 0x0200_1000;
    const BASE_HEALTH: u32 = 0x0200_1010;
    const OWNER_NOTIFY: u32 = 0x0200_1048;
    /// Cast results: weapon, enchantable form, health form (words).
    const CAST_TABLE: u32 = 0x0200_3000;

    // Fake extra data list (0x80 bytes): the doubles read these words.
    const X_ITEMS_IN_LIST: u32 = 0x20;
    const X_COUNT: u32 = 0x24;
    const X_DEFAULT: u32 = 0x28;
    const X_WORN: u32 = 0x2c;
    const X_HAS_LEVELED: u32 = 0x30;
    const X_LEVELED_ITEM: u32 = 0x34;
    const X_HOT_KEY: u32 = 0x38;
    const X_ORIGINAL: u32 = 0x3c;
    const X_HEALTH: u32 = 0x40;
    const X_POISON: u32 = 0x44;
    const X_SCRIPT: u32 = 0x48;
    const X_LOCALS: u32 = 0x4c;
    const X_HAS_MODS: u32 = 0x50;
    const X_MOD_FLAGS: u32 = 0x54;
    const X_SLOT_ACTIVE: u32 = 0x58;
    const X_OWNERSHIP: u32 = 0x5c;
    const X_FLOAT: u32 = 0x60;
    const X_D030: u32 = 0x64;

    // Fake form (0x400 bytes, also the weapon).
    const F_TYPE: u32 = 0x04;
    const F_VALUE: u32 = 0x10;
    const F_HEALTH_FORM: u32 = 0x14;
    const F_HEALTH: u32 = 0x18;
    const F_ENCHANT_VALUE: u32 = 0x1c;
    const F_RATING: u32 = 0x20;

    fn returns(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn returns_float(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    fn word_arg(a: &[u32], i: usize) -> f64 {
        f64::from_bits(a[i] as u64 | (a[i + 1] as u64) << 32)
    }

    macro_rules! read_word {
        ($e:expr, $addr:expr, $off:expr) => {
            $e.register($addr, |e, a| returns(e.mem.u32(a[0] + $off)))
        };
    }

    fn stub(e: &mut Engine, address: u32) {
        e.register(address, |_, _| Ret::default());
    }

    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x0101_0000, 0x10000);
        e.set_global(MINUS_ONE_FLOAT, -1.0f32);
        e.set_global(MINUS_ONE_DOUBLE, -1.0f64);
        e.set_global(HUNDRED_FLOAT, 100.0f32);
        e.set_global(HUNDRED_DOUBLE, 100.0f64);
        e.set_global(HALF_DOUBLE, 0.5f64);
        e.set_global(PRICE_STEP, 0.1f32);
        e.map(0x0200_0000, 0x4000);
        let mut slots = vec![0u32; 20];
        slots[0] = DESTRUCTOR;
        slots[4] = BASE_HEALTH;
        slots[18] = OWNER_NOTIFY;
        e.put_vtable(VTABLE, &slots);
        stub(&mut e, DESTRUCTOR);
        stub(&mut e, OWNER_NOTIFY);
        e.register(BASE_HEALTH, |e, a| returns(e.mem.u32(a[0] + F_HEALTH)));
        // List nodes and small accessors.
        e.register(0x0068_15c0, |_, a| returns(a[0]));
        e.register(WORD_AT_4, |e, a| returns(e.mem.u32(a[0] + 4)));
        e.register(WORD_AT_8, |e, a| returns(e.mem.u32(a[0] + 8)));
        e.register(WORD_AT_0, |e, a| returns(e.mem.u32(a[0])));
        e.register(FORM_TYPE, |e, a| returns(e.mem.u8(a[0] + F_TYPE) as u32));
        e.register(OPERATOR_NEW, |e, a| returns(e.mem.alloc(a[0])));
        e.register(LIST_NODE_CONSTRUCT, |_, a| returns(a[0]));
        e.register(EXTRA_DATA_LIST_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], VTABLE);
            returns(a[0])
        });
        e.register(EXTRA_DATA_LIST_DUPLICATE, |e, a| {
            e.mem.set_u32(a[0], VTABLE);
            e.mem.set_u32(a[0] + 0x14, a[1]);
            Ret::default()
        });
        e.register(LIST_ADD, |e, a| {
            let item = e.mem.u32(a[1]);
            if e.mem.u32(a[0]) == 0 {
                e.mem.set_u32(a[0], item);
            } else {
                let node = e.mem.alloc(8);
                e.mem.set_u32(node, item);
                let next = e.mem.u32(a[0] + 4);
                e.mem.set_u32(node + 4, next);
                e.mem.set_u32(a[0] + 4, node);
            }
            Ret::default()
        });
        e.register(LIST_ADD_TAIL, |e, a| {
            let item = e.mem.u32(a[1]);
            let mut last = a[0];
            while e.mem.u32(last + 4) != 0 {
                last = e.mem.u32(last + 4);
            }
            if e.mem.u32(last) == 0 {
                e.mem.set_u32(last, item);
            } else {
                let node = e.mem.alloc(8);
                e.mem.set_u32(node, item);
                e.mem.set_u32(last + 4, node);
            }
            Ret::default()
        });
        e.register(LIST_REMOVE, |e, a| {
            let item = e.mem.u32(a[1]);
            let mut previous = a[0];
            let mut node = a[0];
            while node != 0 && e.mem.u32(node) != item {
                previous = node;
                node = e.mem.u32(node + 4);
            }
            if node == a[0] {
                let next = e.mem.u32(node + 4);
                if next != 0 {
                    let next_item = e.mem.u32(next);
                    let next_next = e.mem.u32(next + 4);
                    e.mem.set_u32(node, next_item);
                    e.mem.set_u32(node + 4, next_next);
                } else {
                    e.mem.set_u32(node, 0);
                }
            } else if node != 0 {
                let next = e.mem.u32(node + 4);
                e.mem.set_u32(previous + 4, next);
            }
            Ret::default()
        });
        e.register(FTOL, |_, a| Ret {
            eax: (word_arg(a, 0).trunc() as i32) as u32,
            ..Ret::default()
        });
        e.register(RT_DYNAMIC_CAST, |e, a| {
            returns(match a[3] {
                TYPE_TES_OBJECT_WEAP => e.mem.u32(CAST_TABLE),
                TYPE_TES_ENCHANTABLE_FORM => e.mem.u32(CAST_TABLE + 4),
                TYPE_TES_HEALTH_FORM => e.mem.u32(CAST_TABLE + 8),
                other => panic!("unexpected cast target {other:08x}"),
            })
        });
        // Extra data list accessors.
        read_word!(e, EXTRA_IS_DEFAULT_FOR_CONTAINER, X_DEFAULT);
        read_word!(e, EXTRA_GET_WORN, X_WORN);
        read_word!(e, EXTRA_GET_COUNT, X_COUNT);
        read_word!(e, EXTRA_HAS_LEVELED_ITEM, X_HAS_LEVELED);
        read_word!(e, EXTRA_GET_LEVELED_ITEM, X_LEVELED_ITEM);
        read_word!(e, EXTRA_GET_ORIGINAL_REFERENCE, X_ORIGINAL);
        read_word!(e, EXTRA_ITEMS_IN_LIST, X_ITEMS_IN_LIST);
        read_word!(e, EXTRA_GET_HOT_KEY, X_HOT_KEY);
        read_word!(e, EXTRA_GET_POISON, X_POISON);
        read_word!(e, EXTRA_GET_SCRIPT, X_SCRIPT);
        read_word!(e, EXTRA_GET_SCRIPT_LOCALS, X_LOCALS);
        read_word!(e, EXTRA_HAS_WEAPON_MODS, X_HAS_MODS);
        read_word!(e, EXTRA_GET_WEAPON_MOD_FLAGS, X_MOD_FLAGS);
        read_word!(e, EXTRA_GET_OWNERSHIP, X_OWNERSHIP);
        read_word!(e, EXTRA_TEST_D030, X_D030);
        e.register(EXTRA_GET_WEAPON_MOD_SLOT_ACTIVE, |e, a| {
            returns((e.mem.u32(a[0] + X_SLOT_ACTIVE) & a[1] != 0) as u32)
        });
        e.register(EXTRA_GET_HEALTH, |e, a| {
            returns_float(e.mem.f32(a[0] + X_HEALTH))
        });
        e.register(EXTRA_GET_FLOAT_EXTRA, |e, a| {
            returns_float(e.mem.f32(a[0] + X_FLOAT))
        });
        e.register(EXTRA_SET_HEALTH, |e, a| {
            e.mem.set_u32(a[0] + 0x10, a[1]);
            Ret::default()
        });
        e.register(EXTRA_SET_HEALTH_NEW, |e, a| {
            e.mem.set_u32(a[0] + 0x10, a[1]);
            Ret::default()
        });
        e.register(EXTRA_SET_IS_MODDING, |e, a| {
            e.mem.set_u32(a[0] + 0x0c, a[1]);
            Ret::default()
        });
        for address in [
            EXTRA_REMOVE_ALL,
            EXTRA_REMOVE_COUNT,
            EXTRA_REMOVE_ORIGINAL_REFERENCE,
            EXTRA_REMOVE_EXTRA,
            EXTRA_REMOVE_HEALTH,
            EXTRA_REMOVE_IS_MODDING,
            EXTRA_REMOVE_POISON,
            EXTRA_SET_RANK,
            EXTRA_COPY_FROM,
            LIST_CLEAR,
            LIST_DESTROY,
        ] {
            stub(&mut e, address);
        }
        // Forms and formulas.
        read_word!(e, GET_FORM_AS_HEALTH_FORM, F_HEALTH_FORM);
        read_word!(e, GET_FORM_HEALTH, F_HEALTH);
        read_word!(e, GET_FORM_VALUE, F_VALUE);
        e.register(ENCHANTABLE_FORM_VALUE, |e, a| {
            returns(e.mem.u16(a[0] + F_ENCHANT_VALUE) as u32)
        });
        e.register(FLOAT_MIN, |_, a| {
            returns_float(f32::from_bits(a[0]).min(f32::from_bits(a[1])))
        });
        e.register(CALC_ITEM_PRICE, |_, a| {
            returns_float(f32::from_bits(a[0]) * f32::from_bits(a[1]) / 100.0)
        });
        e.register(CALC_WEAPON_DAMAGE, |_, a| {
            returns_float(f32::from_bits(a[2]) * f32::from_bits(a[3]))
        });
        e.register(CALC_ARMOR_RATING, |_, a| {
            returns_float(a[0] as f32 * f32::from_bits(a[1]))
        });
        e.register(ARMOR_RATING_FLOAT, |e, a| {
            returns_float(e.mem.f32(a[0] + F_RATING))
        });
        e.register(ROUND_ARMOR_RESULT, |_, a| {
            returns_float(f32::from_bits(a[0]) * 2.0)
        });
        e
    }

    /// An engine that logs every call.
    fn logged() -> Engine {
        let mut e = engine();
        e.call_log = Some(vec![]);
        e
    }

    fn calls(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn extra(e: &mut Engine) -> u32 {
        let x = e.mem.alloc(0x80);
        e.mem.set_u32(x, VTABLE);
        e.mem.set_f32(x + X_HEALTH, -1.0);
        e.mem.set_f32(x + X_FLOAT, -1.0);
        x
    }

    fn set(e: &mut Engine, base: u32, offset: u32, value: u32) {
        e.mem.set_u32(base + offset, value);
    }

    /// A `BSSimpleList` of `items`; an empty one is a head node with a null
    /// item.
    fn list(e: &mut Engine, items: &[u32]) -> u32 {
        let head = e.mem.alloc(8);
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(node, *item);
            if i + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
        head
    }

    fn items(e: &Engine, head: u32) -> Vec<u32> {
        let mut out = vec![];
        let mut node = head;
        while node != 0 && e.mem.u32(node) != 0 {
            out.push(e.mem.u32(node));
            node = e.mem.u32(node + 4);
        }
        out
    }

    fn form(e: &mut Engine, form_type: u8, value: u32, health: u32) -> u32 {
        let f = e.mem.alloc(0x400);
        e.mem.set_u32(f, VTABLE);
        e.mem.set_u8(f + F_TYPE, form_type);
        set(e, f, F_VALUE, value);
        set(e, f, F_HEALTH_FORM, f);
        set(e, f, F_HEALTH, health);
        f
    }

    fn change(e: &mut Engine, list_head: u32, form: u32) -> Ptr<ItemChange> {
        let c = e.new_object::<ItemChange>();
        e.set(c, ItemChange::pExtraObjectList, Ptr::new(list_head));
        e.set(c, ItemChange::pContainerObj, Ptr::new(form));
        e.set(c, ItemChange::iNumber, 1);
        c
    }

    fn casts(e: &mut Engine, weapon: u32, enchantable: u32, health_form: u32) {
        set(e, CAST_TABLE, 0, weapon);
        set(e, CAST_TABLE, 4, enchantable);
        set(e, CAST_TABLE, 8, health_form);
    }

    #[test]
    fn constructor_stores_form_number_and_an_empty_list() {
        let mut e = logged();
        let this = e.new_object::<ItemChange>();
        let r = e.call(0x004bc550, &args![this, 0x1234u32, 7i32]);
        assert_eq!(r.u32(), this.addr());
        assert_eq!(e.get(this, ItemChange::pContainerObj), Ptr::new(0x1234));
        assert_eq!(e.get(this, ItemChange::iNumber), 7);
        let node = e.get(this, ItemChange::pExtraObjectList).addr();
        assert_ne!(node, 0);
        assert_eq!((e.mem.u32(node), e.mem.u32(node + 4)), (0, 0));
        assert_eq!(calls(&e, OPERATOR_NEW), vec![vec![8]]);
    }

    #[test]
    fn release_list_clears_and_deletes_the_list() {
        let mut e = logged();
        let head = list(&mut e, &[]);
        let this = change(&mut e, head, 0);
        e.call(0x004bc5f0, &args![this]);
        assert_eq!(calls(&e, LIST_CLEAR), vec![vec![head]]);
        assert_eq!(calls(&e, LIST_DESTROY), vec![vec![head, 1]]);
        assert_eq!(e.get(this, ItemChange::pExtraObjectList), Ptr::NULL);
        // Without a list nothing is called.
        let none = change(&mut e, 0, 0);
        e.call_log = Some(vec![]);
        e.call(0x004bc5f0, &args![none]);
        assert_eq!(e.call_log.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn copy_constructor_duplicates_every_extra_list() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let source_list = list(&mut e, &[a, b]);
        let other = change(&mut e, source_list, 0x4321);
        e.set(other, ItemChange::iNumber, 9);
        let this = e.new_object::<ItemChange>();
        let r = e.call(0x004bc650, &args![this, other]);
        assert_eq!(r.u32(), this.addr());
        assert_eq!(e.get(this, ItemChange::pContainerObj), Ptr::new(0x4321));
        assert_eq!(e.get(this, ItemChange::iNumber), 9);
        let copies = items(&e, e.get(this, ItemChange::pExtraObjectList).addr());
        assert_eq!(copies.len(), 2);
        assert_eq!(e.mem.u32(copies[0] + 0x14), a);
        assert_eq!(e.mem.u32(copies[1] + 0x14), b);
        assert_eq!(calls(&e, EXTRA_DATA_LIST_DUPLICATE).len(), 2);
    }

    #[test]
    fn delete_all_extra_removes_and_deletes_starting_from_the_node() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        e.call(0x004bc780, &args![this]);
        // The removal copies the next node over the current one, so the walk
        // ends after the first list (the game's behaviour).
        assert_eq!(calls(&e, EXTRA_REMOVE_ALL), vec![vec![a, 1]]);
        assert_eq!(calls(&e, DESTRUCTOR), vec![vec![a, 1]]);
        assert_eq!(items(&e, head), vec![b]);
    }

    #[test]
    fn shallow_copy_shares_the_extra_lists() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let source_list = list(&mut e, &[a, b]);
        let other = change(&mut e, source_list, 0x4321);
        let this = e.new_object::<ItemChange>();
        e.call(0x004bc810, &args![this, other]);
        assert_eq!(e.get(this, ItemChange::pContainerObj), Ptr::new(0x4321));
        let copied = items(&e, e.get(this, ItemChange::pExtraObjectList).addr());
        assert_eq!(copied, vec![a, b]);
        // A null source does nothing.
        let untouched = e.new_object::<ItemChange>();
        e.call(0x004bc810, &args![untouched, Ptr::<ItemChange>::NULL]);
        assert_eq!(e.get(untouched, ItemChange::pExtraObjectList), Ptr::NULL);
    }

    #[test]
    fn extra_total_count_skips_default_and_optionally_worn() {
        let mut e = logged();
        let (default, worn, plain) = (extra(&mut e), extra(&mut e), extra(&mut e));
        set(&mut e, default, X_DEFAULT, 1);
        set(&mut e, default, X_COUNT, 100);
        set(&mut e, worn, X_WORN, 1);
        set(&mut e, worn, X_COUNT, 3);
        set(&mut e, plain, X_COUNT, 5);
        let head = list(&mut e, &[default, worn, plain]);
        let this = change(&mut e, head, 0);
        assert_eq!(e.call(0x004bc8f0, &args![this, false]).i32(), 5);
        assert_eq!(e.call(0x004bc8f0, &args![this, true]).i32(), 8);
    }

    #[test]
    fn extra_total_default_count_restarts_after_deleting_an_empty_list() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        for x in [a, b] {
            set(&mut e, x, X_DEFAULT, 1);
        }
        set(&mut e, a, X_COUNT, 4);
        set(&mut e, a, 4, 0x77); // has extras
        set(&mut e, b, X_COUNT, 1); // no extras: pHead is 0
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        assert_eq!(e.call(0x004bc980, &args![this]).i32(), 4);
        assert_eq!(calls(&e, DESTRUCTOR), vec![vec![b, 1]]);
        assert_eq!(items(&e, head), vec![a]);
    }

    #[test]
    fn has_no_extras_tests_the_first_extra_pointer() {
        let mut e = engine();
        let x = extra(&mut e);
        assert!(e.call(0x004bca60, &args![x]).bool());
        set(&mut e, x, 4, 1);
        assert!(!e.call(0x004bca60, &args![x]).bool());
    }

    #[test]
    fn amount_non_default_extra_counts_and_collapses_stacks() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        set(&mut e, a, X_COUNT, 2);
        set(&mut e, a, X_ITEMS_IN_LIST, 2);
        set(&mut e, b, X_COUNT, 1);
        set(&mut e, b, X_ITEMS_IN_LIST, 2);
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        assert_eq!(e.call(0x004bca80, &args![this]).i32(), 2);
        // A single item type with a count above one: count removed, list
        // deleted, total restarts at zero.
        let c = extra(&mut e);
        set(&mut e, c, X_COUNT, 3);
        set(&mut e, c, X_ITEMS_IN_LIST, 1);
        let head = list(&mut e, &[c]);
        let other = change(&mut e, head, 0);
        assert_eq!(e.call(0x004bca80, &args![other]).i32(), 0);
        assert_eq!(calls(&e, EXTRA_REMOVE_COUNT), vec![vec![c]]);
        assert_eq!(calls(&e, DESTRUCTOR), vec![vec![c, 1]]);
        assert!(items(&e, head).is_empty());
    }

    #[test]
    fn leveled_item_predicates_and_counts() {
        let mut e = engine();
        let (a, b) = (extra(&mut e), extra(&mut e));
        set(&mut e, b, X_HAS_LEVELED, 1);
        set(&mut e, b, X_COUNT, 6);
        set(&mut e, a, X_COUNT, 2);
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        assert!(e.call(0x004bcb70, &args![this]).bool());
        assert_eq!(e.call(0x004bcc30, &args![this]).i32(), 6);
        let empty = list(&mut e, &[]);
        let none = change(&mut e, empty, 0);
        assert!(!e.call(0x004bcb70, &args![none]).bool());
        assert_eq!(e.call(0x004bcc30, &args![none]).i32(), 0);
    }

    #[test]
    fn remove_original_changes_only_touches_lists_with_an_original() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        set(&mut e, b, X_ORIGINAL, 0x55);
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        e.call(0x004bcbd0, &args![this]);
        assert_eq!(calls(&e, EXTRA_REMOVE_ORIGINAL_REFERENCE), vec![vec![b]]);
    }

    #[test]
    fn any_extra_list_test_d030() {
        let mut e = engine();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        assert!(!e.call(0x004bccb0, &args![this]).bool());
        set(&mut e, b, X_D030, 1);
        assert!(e.call(0x004bccb0, &args![this]).bool());
    }

    #[test]
    fn remove_leveled_item_extra_removes_and_unlinks_empty_lists() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        set(&mut e, a, X_LEVELED_ITEM, 0x9000);
        set(&mut e, b, 4, 1);
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        assert_eq!(e.call(0x004bcd10, &args![this]).u32(), 0x9000);
        assert_eq!(calls(&e, EXTRA_REMOVE_EXTRA), vec![vec![a, 0x9000, 0]]);
        // `a` had no extras left, so it is out of the list but not deleted.
        assert_eq!(items(&e, head), vec![b]);
        assert!(calls(&e, DESTRUCTOR).is_empty());
    }

    #[test]
    fn item_health_variants() {
        let mut e = engine();
        // A form without a health form gives -1.0.
        let plain = form(&mut e, 0x28, 0, 100);
        set(&mut e, plain, F_HEALTH_FORM, 0);
        casts(&mut e, 0, 0, 0);
        let empty = list(&mut e, &[]);
        let this = change(&mut e, empty, plain);
        assert_eq!(e.call(0x004bcdb0, &args![this, false]).f32(), -1.0);
        // Base health 100, no health extra.
        let armor = form(&mut e, 0x18, 0, 100);
        let this = change(&mut e, empty, armor);
        assert_eq!(e.call(0x004bcdb0, &args![this, false]).f32(), 100.0);
        assert_eq!(e.call(0x004bcdb0, &args![this, true]).f32(), 100.0);
        // A health extra of 50.
        let x = extra(&mut e);
        e.mem.set_f32(x + X_HEALTH, 50.0);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, armor);
        assert_eq!(e.call(0x004bcdb0, &args![this, false]).f32(), 50.0);
        assert_eq!(e.call(0x004bcdb0, &args![this, true]).f32(), 50.0);
        // The ratio is capped at 100, not at 1: 250 / 100 stays 2.5.
        e.mem.set_f32(x + X_HEALTH, 250.0);
        assert_eq!(e.call(0x004bcdb0, &args![this, true]).f32(), 250.0);
        // A weapon takes its base health (plus the active health mod) from
        // the weapon, and an extra list without health (-1.0) uses it.
        let weapon = form(&mut e, 0x28, 0, 100);
        set(&mut e, weapon, 0x98, 80);
        set(&mut e, weapon, WEAPON_MOD_ACTION, 10);
        e.mem.set_f32(weapon + WEAPON_MOD_ACTION_VALUE, 15.5);
        casts(&mut e, weapon, 0, 0);
        let y = extra(&mut e);
        set(&mut e, y, X_SLOT_ACTIVE, 1);
        let head = list(&mut e, &[y]);
        let this = change(&mut e, head, weapon);
        assert_eq!(e.call(0x004bcdb0, &args![this, false]).f32(), 95.0);
        set(&mut e, y, X_SLOT_ACTIVE, 0);
        assert_eq!(e.call(0x004bcdb0, &args![this, false]).f32(), 80.0);
    }

    #[test]
    fn weapon_health_adds_the_mod_effect_value() {
        let mut e = engine();
        let weapon = form(&mut e, 0x28, 0, 0);
        set(&mut e, weapon, 0x98, 80);
        set(&mut e, weapon, WEAPON_MOD_ACTION + 4, 10);
        e.mem.set_f32(weapon + WEAPON_MOD_ACTION_VALUE + 4, 15.75);
        assert_eq!(e.call(0x004bcf00, &args![weapon, 0u32]).i32(), 80);
        assert_eq!(e.call(0x004bcf00, &args![weapon, 1u32]).i32(), 95);
        // Without a mod of that type the bonus is zero.
        set(&mut e, weapon, WEAPON_MOD_ACTION + 4, 3);
        assert_eq!(e.call(0x004bcf00, &args![weapon, 1u32]).i32(), 80);
    }

    #[test]
    fn mod_effect_value_picks_the_slot_and_variant() {
        let mut e = engine();
        let weapon = form(&mut e, 0x28, 0, 0);
        set(&mut e, weapon, WEAPON_MOD_ACTION + 8, 6);
        e.mem.set_f32(weapon + WEAPON_MOD_ACTION_VALUE + 8, 2.5);
        e.mem.set_f32(weapon + WEAPON_MOD_ACTION_VALUE_TWO + 8, 4.5);
        assert_eq!(e.call(0x004bcf60, &args![weapon, 6u32, 0u32]).f32(), 2.5);
        assert_eq!(e.call(0x004bcf60, &args![weapon, 6u32, 1u32]).f32(), 4.5);
        assert_eq!(e.call(0x004bcf60, &args![weapon, 6u32, 2u32]).f32(), 0.0);
        assert_eq!(e.call(0x004bcf60, &args![weapon, 7u32, 0u32]).f32(), 0.0);
    }

    #[test]
    fn set_item_health_stores_the_health_extra() {
        let mut e = logged();
        let armor = form(&mut e, 0x18, 0, 100);
        casts(&mut e, 0, 0, armor);
        let owner = e.mem.alloc(0x10);
        set(&mut e, owner, 0, VTABLE);
        let inventory = e.mem.alloc(0x10);
        set(&mut e, inventory, 4, owner);
        let x = extra(&mut e);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, armor);
        // Below the form's health: stored in the matching extra list, and the
        // owner is notified.
        let health = 50.0f32;
        e.call(0x004bd030, &args![this, health, inventory, x, true]);
        assert_eq!(calls(&e, OWNER_NOTIFY), vec![vec![owner, 0x20]]);
        assert_eq!(calls(&e, EXTRA_SET_HEALTH), vec![vec![x, health.to_bits()]]);
        // Another extra list that is not in the list: a new one is added.
        let other = extra(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x004bd030, &args![this, health, inventory, other, true]);
        let stored = items(&e, head);
        assert_eq!(stored.len(), 2);
        assert_eq!(calls(&e, EXTRA_SET_HEALTH).len(), 1);
        assert_eq!(e.mem.u32(stored[1] + 0x10), health.to_bits());
        // No list at all: list node and extra list are made.
        let bare = change(&mut e, 0, armor);
        e.call(0x004bd030, &args![bare, health, inventory, x, true]);
        let node = e.get(bare, ItemChange::pExtraObjectList).addr();
        let made = items(&e, node);
        assert_eq!(made.len(), 1);
        assert_eq!(e.mem.u32(made[0] + 0x10), health.to_bits());
        // No health form: nothing but the owner notification.
        casts(&mut e, 0, 0, 0);
        e.call_log = Some(vec![]);
        e.call(0x004bd030, &args![this, health, inventory, x, true]);
        assert_eq!(calls(&e, EXTRA_SET_HEALTH).len(), 0);
        assert_eq!(calls(&e, OWNER_NOTIFY).len(), 1);
    }

    #[test]
    fn set_item_health_at_or_above_the_base_removes_the_extra() {
        let mut e = logged();
        let armor = form(&mut e, 0x18, 0, 100);
        casts(&mut e, 0, 0, armor);
        let inventory = e.mem.alloc(0x10);
        let x = extra(&mut e);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, armor);
        // The extra still has other extras: only RemoveHealth.
        set(&mut e, x, 4, 1);
        e.call(0x004bd030, &args![this, 100.0f32, inventory, x, true]);
        assert_eq!(calls(&e, EXTRA_REMOVE_HEALTH), vec![vec![x]]);
        assert!(calls(&e, DESTRUCTOR).is_empty());
        // Now it is empty and `remove_empty` is set: the entry that holds the
        // item type is found and the list is removed from it and deleted.
        set(&mut e, x, 4, 0);
        let entry_list = list(&mut e, &[x]);
        let entry = e.mem.alloc(8);
        set(&mut e, entry, 0, entry_list);
        e.register(INVENTORY_CHANGES_GET_OBJECT_IN_LIST, |e, _| {
            returns(e.mem.u32(CAST_TABLE + 0x10))
        });
        set(&mut e, CAST_TABLE, 0x10, entry);
        e.call_log = Some(vec![]);
        e.call(0x004bd030, &args![this, 150.0f32, inventory, x, true]);
        assert_eq!(
            calls(&e, INVENTORY_CHANGES_GET_OBJECT_IN_LIST),
            vec![vec![inventory, armor, 1, 0]]
        );
        assert!(items(&e, entry_list).is_empty());
        assert_eq!(calls(&e, DESTRUCTOR), vec![vec![x, 1]]);
    }

    #[test]
    fn fn_004bd350_prefers_the_float_extra() {
        let mut e = engine();
        let ench = form(&mut e, 0x2a, 0, 0);
        e.mem.set_u16(ench + F_ENCHANT_VALUE, 40);
        casts(&mut e, 0, ench, 0);
        let x = extra(&mut e);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, ench);
        // No float extra (-1.0): the form's 16-bit value.
        assert_eq!(e.call(0x004bd350, &args![this]).f32(), 40.0);
        e.mem.set_f32(x + X_FLOAT, 7.5);
        assert_eq!(e.call(0x004bd350, &args![this]).f32(), 7.5);
        // No extra list at all.
        let empty = list(&mut e, &[]);
        let bare = change(&mut e, empty, ench);
        assert_eq!(e.call(0x004bd350, &args![bare]).f32(), 40.0);
        // Not an enchantable form.
        casts(&mut e, 0, 0, 0);
        assert_eq!(e.call(0x004bd350, &args![this]).f32(), -1.0);
    }

    #[test]
    fn item_value_scales_by_health_and_adds_weapon_mods() {
        let mut e = engine();
        let armor = form(&mut e, 0x18, 200, 100);
        casts(&mut e, 0, 0, 0);
        let x = extra(&mut e);
        e.mem.set_f32(x + X_HEALTH, 50.0);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, armor);
        // 200 at 50 percent health, rounded to 0.1.
        assert_eq!(e.call(0x004bd400, &args![this]).f32(), 100.0);
        // A weapon with mods in slots 1 and 4 adds each mod object's value.
        let weapon = form(&mut e, 0x28, 200, 100);
        set(&mut e, weapon, 0x98, 100);
        casts(&mut e, weapon, 0, 0);
        let mods = [e.mem.alloc(0x100), e.mem.alloc(0x100)];
        set(&mut e, mods[0], MOD_OBJECT_VALUE + 4, 10);
        set(&mut e, mods[1], MOD_OBJECT_VALUE + 4, 20);
        set(&mut e, weapon, WEAPON_MOD_OBJECT, mods[0]);
        set(&mut e, weapon, WEAPON_MOD_OBJECT + 8, mods[1]);
        set(&mut e, x, X_HAS_MODS, 1);
        set(&mut e, x, X_MOD_FLAGS, 5);
        let this = change(&mut e, head, weapon);
        assert_eq!(e.call(0x004bd400, &args![this]).f32(), 130.0);
    }

    #[test]
    fn rounding_to_a_multiple() {
        let mut e = engine();
        assert_eq!(e.call(0x004bd510, &args![1.04f32, 0.1f32]).f32(), 1.0);
        let up = e.call(0x004bd510, &args![1.06f32, 0.1f32]).f32();
        assert!((up - 1.1).abs() < 1e-6);
        assert_eq!(e.call(0x004bd510, &args![0.0f32, 0.1f32]).f32(), 0.0);
    }

    #[test]
    fn mod_object_and_effect_selectors() {
        let mut e = engine();
        let weapon = form(&mut e, 0x28, 0, 0);
        for (i, bit) in [1u32, 2, 4].into_iter().enumerate() {
            let i = i as u32;
            set(&mut e, weapon, WEAPON_MOD_OBJECT + 4 * i, 0x100 + bit);
            set(&mut e, weapon, WEAPON_MOD_ACTION + 4 * i, 0x200 + bit);
            e.mem
                .set_f32(weapon + WEAPON_MOD_ACTION_VALUE + 4 * i, bit as f32);
            e.mem.set_f32(
                weapon + WEAPON_MOD_ACTION_VALUE_TWO + 4 * i,
                10.0 * bit as f32,
            );
            assert_eq!(e.call(0x004bd570, &args![weapon, bit]).u32(), 0x100 + bit);
            assert_eq!(e.call(0x004bd880, &args![weapon, bit]).u32(), 0x200 + bit);
            assert_eq!(
                e.call(0x004bd9d0, &args![weapon, bit, 0u32]).f32(),
                bit as f32
            );
            assert_eq!(
                e.call(0x004bd9d0, &args![weapon, bit, 1u32]).f32(),
                10.0 * bit as f32
            );
            assert_eq!(e.call(0x004bd9d0, &args![weapon, bit, 2u32]).f32(), 0.0);
        }
        assert_eq!(e.call(0x004bd570, &args![weapon, 3u32]).u32(), 0);
        assert_eq!(e.call(0x004bd880, &args![weapon, 8u32]).u32(), 0);
        assert_eq!(e.call(0x004bd9d0, &args![weapon, 0u32, 0u32]).f32(), 0.0);
    }

    #[test]
    fn set_or_clear_the_first_extra_list() {
        let mut e = logged();
        let source = extra(&mut e);
        let x = extra(&mut e);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, 0);
        // Copy into the first list.
        e.call(0x004bd5c0, &args![this, source]);
        assert_eq!(calls(&e, EXTRA_COPY_FROM), vec![vec![x, source]]);
        // No list at all: node and extra list are made.
        let bare = change(&mut e, 0, 0);
        e.call(0x004bd5c0, &args![bare, source]);
        let node = e.get(bare, ItemChange::pExtraObjectList).addr();
        let made = items(&e, node);
        assert_eq!(made.len(), 1);
        assert_eq!(calls(&e, EXTRA_COPY_FROM)[1], vec![made[0], source]);
        // A null source removes and deletes the first list.
        e.call(0x004bd5c0, &args![this, Ptr::<()>::NULL]);
        assert!(items(&e, head).is_empty());
        assert_eq!(calls(&e, DESTRUCTOR), vec![vec![x, 1]]);
    }

    #[test]
    fn first_extra_list_accessors() {
        let mut e = engine();
        let x = extra(&mut e);
        set(&mut e, x, X_OWNERSHIP, 0x66);
        set(&mut e, x, X_POISON, 0x77);
        set(&mut e, x, X_HAS_MODS, 1);
        set(&mut e, x, X_MOD_FLAGS, 6);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, 0);
        assert_eq!(e.call(0x004bd740, &args![this]).u32(), 0x66);
        assert_eq!(e.call(0x004bdcc0, &args![this]).u32(), 0x77);
        assert_eq!(e.call(0x004bd820, &args![this]).u8(), 6);
        set(&mut e, x, X_HAS_MODS, 0);
        assert_eq!(e.call(0x004bd820, &args![this]).u8(), 0);
        let empty = list(&mut e, &[]);
        let none = change(&mut e, empty, 0);
        assert_eq!(e.call(0x004bd740, &args![none]).u32(), 0);
        assert_eq!(e.call(0x004bdcc0, &args![none]).u32(), 0);
        assert_eq!(e.call(0x004bd820, &args![none]).u8(), 0);
    }

    #[test]
    fn hot_key_is_the_first_one_set() {
        let mut e = engine();
        let (a, b, c) = (extra(&mut e), extra(&mut e), extra(&mut e));
        set(&mut e, a, X_HOT_KEY, 0xff); // -1
        set(&mut e, b, X_HOT_KEY, 3);
        set(&mut e, c, X_HOT_KEY, 5);
        let head = list(&mut e, &[a, b, c]);
        let this = change(&mut e, head, 0);
        assert_eq!(e.call(0x004bd7a0, &args![this]).i32(), 3);
        let only = list(&mut e, &[a]);
        let none = change(&mut e, only, 0);
        assert_eq!(e.call(0x004bd7a0, &args![none]).i32(), -1);
    }

    #[test]
    fn mod_effect_active_checks_the_slots_in_order() {
        let mut e = engine();
        let weapon = form(&mut e, 0x28, 0, 0);
        set(&mut e, weapon, WEAPON_MOD_ACTION, 3);
        set(&mut e, weapon, WEAPON_MOD_ACTION + 4, 9);
        set(&mut e, weapon, WEAPON_MOD_ACTION + 8, 9);
        e.mem.set_f32(weapon + WEAPON_MOD_ACTION_VALUE + 4, 1.5);
        e.mem.set_f32(weapon + WEAPON_MOD_ACTION_VALUE + 8, 2.5);
        let x = extra(&mut e);
        set(&mut e, x, X_SLOT_ACTIVE, 6);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, weapon);
        let out = e.mem.alloc(4);
        assert!(e.call(0x004bd8d0, &args![this, 9u32, out]).bool());
        assert_eq!(e.mem.f32(out), 1.5);
        assert!(e.call(0x004bda70, &args![this, 9u32]).bool());
        // Slot 1 has the type but is not active; type 3 is nowhere active.
        assert!(!e.call(0x004bd8d0, &args![this, 3u32, out]).bool());
        assert!(!e.call(0x004bda70, &args![this, 3u32]).bool());
        // No extra list.
        let empty = list(&mut e, &[]);
        let none = change(&mut e, empty, weapon);
        assert!(!e.call(0x004bda70, &args![none, 9u32]).bool());
        assert!(!e.call(0x004bd8d0, &args![none, 9u32, out]).bool());
    }

    #[test]
    fn modding_flag_is_set_on_the_first_list_or_a_new_one() {
        let mut e = logged();
        let x = extra(&mut e);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, 0);
        e.call(0x004bdb40, &args![this]);
        assert_eq!(e.mem.u32(x + 0x0c), 1);
        e.call(0x004bdc70, &args![this]);
        assert_eq!(calls(&e, EXTRA_REMOVE_IS_MODDING), vec![vec![x]]);
        // An empty list node gets a new extra list.
        let empty = list(&mut e, &[]);
        let bare = change(&mut e, empty, 0);
        e.call(0x004bdb40, &args![bare]);
        let made = items(&e, empty);
        assert_eq!(made.len(), 1);
        assert_eq!(e.mem.u32(made[0] + 0x0c), 1);
        // No list at all: the node is made too.
        let nothing = change(&mut e, 0, 0);
        e.call(0x004bdb40, &args![nothing]);
        let node = e.get(nothing, ItemChange::pExtraObjectList).addr();
        assert_eq!(items(&e, node).len(), 1);
    }

    #[test]
    fn poison_helpers() {
        let mut e = logged();
        let x = extra(&mut e);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, 0);
        // No poison: the rank setter runs, the remover does not.
        e.call(0x004bdd20, &args![this, 4u32]);
        e.call(0x004bdd80, &args![this]);
        assert_eq!(calls(&e, EXTRA_SET_RANK), vec![vec![x, 4]]);
        assert!(calls(&e, EXTRA_REMOVE_POISON).is_empty());
        set(&mut e, x, X_POISON, 0x99);
        e.call(0x004bdd20, &args![this, 5u32]);
        e.call(0x004bdd80, &args![this]);
        assert_eq!(calls(&e, EXTRA_SET_RANK).len(), 1);
        assert_eq!(calls(&e, EXTRA_REMOVE_POISON), vec![vec![x]]);
    }

    #[test]
    fn worn_script_and_locals_search_all_lists() {
        let mut e = logged();
        let (a, b) = (extra(&mut e), extra(&mut e));
        set(&mut e, b, X_WORN, 1);
        set(&mut e, b, X_SCRIPT, 0x31);
        set(&mut e, b, X_LOCALS, 0x32);
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        assert!(e.call(0x004bddd0, &args![this, 1u32]).bool());
        assert_eq!(calls(&e, EXTRA_GET_WORN), vec![vec![a, 1], vec![b, 1]]);
        assert_eq!(e.call(0x004bde40, &args![this]).u32(), 0x31);
        assert_eq!(e.call(0x004bdea0, &args![this]).u32(), 0x32);
        let only = list(&mut e, &[a]);
        let plain = change(&mut e, only, 0);
        assert!(!e.call(0x004bddd0, &args![plain, 0u32]).bool());
        assert_eq!(e.call(0x004bde40, &args![plain]).u32(), 0);
        assert_eq!(e.call(0x004bdea0, &args![plain]).u32(), 0);
    }

    #[test]
    fn attack_damage_is_for_weapon_forms_only() {
        let mut e = logged();
        let weapon = form(&mut e, 0x28, 0, 200);
        casts(&mut e, 0, 0, 0);
        let x = extra(&mut e);
        e.mem.set_f32(x + X_HEALTH, 100.0);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, weapon);
        let attacker = 0x0123_4560u32;
        // Health ratio 100 / 200 = 0.5; the double returns ratio * scale.
        let r = e
            .call(0x004bdf00, &args![this, attacker, 4.0f32, 1u32])
            .f32();
        assert_eq!(r, 2.0);
        assert_eq!(
            calls(&e, CALC_WEAPON_DAMAGE),
            vec![vec![
                attacker,
                weapon,
                0.5f32.to_bits(),
                4.0f32.to_bits(),
                0,
                1,
                this.addr(),
                1
            ]]
        );
        let other = form(&mut e, 0x18, 0, 200);
        let this = change(&mut e, head, other);
        assert_eq!(
            e.call(0x004bdf00, &args![this, attacker, 4.0f32, 1u32])
                .f32(),
            0.0
        );
    }

    #[test]
    fn armor_rating_uses_the_rating_and_health_ratio() {
        let mut e = logged();
        let armor = form(&mut e, 0x18, 0, 200);
        e.mem.set_f32(armor + F_RATING, 12.75);
        let x = extra(&mut e);
        e.mem.set_f32(x + X_HEALTH, 100.0);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, armor);
        // rating 12 (truncated) * ratio 0.5 = 6, doubled by the rounding stub.
        assert_eq!(e.call(0x004bdf90, &args![this, 0u32]).f32(), 12.0);
        assert_eq!(
            calls(&e, CALC_ARMOR_RATING),
            vec![vec![12, 0.5f32.to_bits()]]
        );
        // Zero form health: the ratio is zero and the item health is not read.
        set(&mut e, armor, F_HEALTH, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004bdf90, &args![this, 0u32]).f32(), 0.0);
        assert!(calls(&e, EXTRA_GET_HEALTH).is_empty());
        // Another form type: -1.0 goes through the rounding.
        let weapon = form(&mut e, 0x28, 0, 200);
        let this = change(&mut e, head, weapon);
        assert_eq!(e.call(0x004bdf90, &args![this, 0u32]).f32(), -2.0);
    }

    #[test]
    fn truncation_matches_fistp() {
        assert_eq!(truncate_to_i32(12.75), 12);
        assert_eq!(truncate_to_i32(-12.75), -12);
        assert_eq!(truncate_to_i32(3.0e10), i32::MIN);
        assert_eq!(truncate_to_i32(f64::NAN), i32::MIN);
    }
}
