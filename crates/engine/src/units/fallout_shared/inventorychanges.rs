//! `fallout shared/inventorychanges.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! Session notes (read these first when you continue the unit).
//!
//! * The unit is large (148 functions). Translated so far: `004bc550` to
//!   `004c0c60` (the `ItemChange` methods, the weapon mod helpers, the
//!   save/load code of `ItemChange`, the `InventoryChanges` constructor and
//!   its hot key, equip and lookup methods). The next session continues at
//!   `004c0c90` (the `InventoryChanges` layout is declared below with the
//!   constants of the second session; `004c0cf0`, 2030 bytes, is the item
//!   removal that `RemoveAllObjectsWorn` calls by address).
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

// Constants of the second session (functions 004be060 to 004c0c60).

/// RTTI type descriptors the casts of this part use (names from the
/// `.?AV...@@` strings): `TESForm`, `TESFullName`, `TESBipedModelForm`,
/// `TESNPC`.
pub(crate) const TYPE_TES_FORM: u32 = 0x0118_3028;
pub(crate) const TYPE_TES_FULL_NAME: u32 = 0x0118_3158;
pub(crate) const TYPE_TES_BIPED_MODEL_FORM: u32 = 0x0118_3978;
pub(crate) const TYPE_TES_NPC: u32 = 0x0118_3a1c;

/// The source path string the game's logging passes
/// (`D:\_Fallout3\Platforms\Common\Code\Fallout Shared\InventoryChanges.cpp`).
pub(crate) const SOURCE_FILE_NAME: u32 = 0x0102_0778;
/// The player character pointer: the global the game compares actors with
/// and whose `+0xe3c` holds the hot key table.
pub(crate) const PLAYER_GLOBAL: u32 = 0x011d_ea3c;
/// The save/load game object (`TESSaveLoadGame`, by the names of the methods
/// called on it): the `this` of every save size/save/load call of the unit.
pub(crate) const SAVE_LOAD_GAME_GLOBAL: u32 = 0x011d_e45c;
/// The object `00408d60` turns into the "log the save sizes" flag byte.
pub(crate) const SAVE_SIZE_LOG_OBJECT: u32 = 0x011d_e4e8;
/// `InventoryChanges::pTempRef` (Xbox PDB): the scratch `TESObjectREFR`
/// the first `InventoryChanges` constructor creates.
pub(crate) const TEMP_REF_GLOBAL: u32 = 0x011c_6444;
/// The two string objects `00403df0` turns into a `char *` when no icon or
/// no name was found.
pub(crate) const NO_ICON_STRING: u32 = 0x011d_33f0;
pub(crate) const NO_NAME_STRING: u32 = 0x011d_35b8;
/// `0.0` as a double (the comparison constant of `fn_004be0b0`).
pub(crate) const ZERO_DOUBLE: u32 = 0x0101_2060;

/// `00404eb0(guard, 0x36, 1, file, line)` and `00404ee0(guard)`: the scope
/// guard (4 bytes on the stack) the methods of the unit open and close.
pub(crate) const SCOPE_GUARD_OPEN: u32 = 0x0040_4eb0;
pub(crate) const SCOPE_GUARD_CLOSE: u32 = 0x0040_4ee0;
/// `operator delete(ptr)` and `memset(ptr, value, size)` (`__cdecl`).
pub(crate) const OPERATOR_DELETE: u32 = 0x0040_1030;
pub(crate) const MEMSET: u32 = 0x0040_3d30;
/// `Error(format, ...)` (Xbox PDB, `__cdecl`, varargs) and the save/load
/// log `005b5e40(format, ...)` (`__cdecl`, varargs).
pub(crate) const ERROR_LOG: u32 = 0x0040_fbe0;
pub(crate) const SAVE_LOAD_LOG: u32 = 0x005b_5e40;
/// `delete item` of an `ItemChange` (`004459e0(item, flags)`, a scalar
/// deleting destructor).
pub(crate) const ITEM_CHANGE_DELETE: u32 = 0x0044_59e0;
/// `005f65d0(list, &item)` (`__thiscall`): true when the list holds the item.
pub(crate) const LIST_CONTAINS: u32 = 0x005f_65d0;
/// The word at +0xC of a form (`0084e3a0`): its form id.
pub(crate) const FORM_ID: u32 = 0x0084_e3a0;
/// `LookupFormByID(id)` (`004839c0`, `__cdecl`).
pub(crate) const LOOKUP_FORM_BY_ID: u32 = 0x0048_39c0;
/// `InventoryChanges::StartOEIFastInventoryIteration()` and
/// `GetNextOEIFastInventoryItem(iterator)` (Xbox PDB), and the iterator's
/// destructor body (`004ca1a0`).
pub(crate) const START_FAST_ITERATION: u32 = 0x004c_a200;
pub(crate) const GET_NEXT_FAST_ITEM: u32 = 0x004c_a330;
pub(crate) const FAST_ITERATOR_DESTRUCT: u32 = 0x004c_a1a0;
/// `004c0cf0` (2030 bytes, of this unit, translated by a later session): the
/// routine `RemoveAllObjectsWorn` calls to take an item out.
pub(crate) const INVENTORY_REMOVE_ITEM: u32 = 0x004c_0cf0;
/// `ExtraDataList` methods used by this part (Xbox PDB names).
pub(crate) const EXTRA_COMPARE_LIST: u32 = 0x0041_27e0;
pub(crate) const EXTRA_COPY_LIST: u32 = 0x0041_1ec0;
pub(crate) const EXTRA_SET_COUNT: u32 = 0x0041_9ad0;
pub(crate) const EXTRA_SET_WORN: u32 = 0x0041_aa20;
pub(crate) const EXTRA_SET_CAN_NOT_WEAR: u32 = 0x0041_ab70;
pub(crate) const EXTRA_GET_CAN_NOT_WEAR: u32 = 0x0041_8b10;
pub(crate) const EXTRA_REMOVE_HOT_KEY: u32 = 0x0042_dec0;
pub(crate) const EXTRA_SET_HOT_KEY: u32 = 0x0042_dde0;
pub(crate) const EXTRA_GET_REFERENCE_POINTER: u32 = 0x0041_c8d0;
pub(crate) const EXTRA_GET_CONTAINER_CHANGES: u32 = 0x0041_8520;
pub(crate) const EXTRA_SET_CONTAINER_CHANGES: u32 = 0x0041_9650;
/// Unnamed `ExtraDataList` methods: the save size (`00422c40(0, 0)`), the
/// `SaveGame(0, 0)` and `LoadGame(0, 0, 0)` of the file format, the
/// `00426020(0, 0, 0, form)` that runs after loading and `0042ceb0(arg,
/// form)` (the other, buffer based, finishing step).
pub(crate) const EXTRA_GET_SAVE_SIZE: u32 = 0x0042_2c40;
pub(crate) const EXTRA_SAVE_GAME: u32 = 0x0042_35e0;
pub(crate) const EXTRA_LOAD_GAME: u32 = 0x0042_4960;
pub(crate) const EXTRA_AFTER_LOAD_GAME: u32 = 0x0042_6020;
pub(crate) const EXTRA_AFTER_LOAD_GAME_BUFFER: u32 = 0x0042_ceb0;
/// `ExtraDataList::SaveGame_ov2(buffer)` and `LoadGame_ov2(buffer)`
/// (Xbox PDB).
pub(crate) const EXTRA_SAVE_GAME_BUFFER: u32 = 0x0042_6a30;
pub(crate) const EXTRA_LOAD_GAME_BUFFER: u32 = 0x0042_8150;
/// `BGSSaveGameBuffer::SaveFormID_ov2(form, 0)`, the raw data save
/// (`00865e50(ptr, size, 0)`), `StartVariableSizedValue()` and
/// `SaveVariableSizedValue_ov2(count, start)` (Xbox PDB).
pub(crate) const SAVE_BUFFER_SAVE_FORM_ID: u32 = 0x0086_5df0;
pub(crate) const SAVE_BUFFER_SAVE_DATA: u32 = 0x0086_5e50;
pub(crate) const SAVE_BUFFER_START_SIZED: u32 = 0x0086_5f20;
pub(crate) const SAVE_BUFFER_END_SIZED: u32 = 0x0086_5ff0;
/// `BGSLoadGameBuffer::LoadFormID()`, the raw data load (`00864980(ptr,
/// size)`) and `LoadVariableSizedValue()` (Xbox PDB).
pub(crate) const LOAD_BUFFER_LOAD_FORM_ID: u32 = 0x0086_48a0;
pub(crate) const LOAD_BUFFER_LOAD_DATA: u32 = 0x0086_4980;
pub(crate) const LOAD_BUFFER_LOAD_SIZED: u32 = 0x0086_4a60;
/// `TESSaveLoadGame` methods: `UseSaveGameBlocks` (Xbox PDB), the stream
/// position (`00825c00`, the word at +0x14), the raw save (`008579b0(ptr,
/// size)`), `SaveNumericID(ptr, size)` (Xbox PDB), the raw load
/// (`008579e0(ptr, size)`), `LoadNumericID(ptr, size)` (Xbox PDB), the two
/// current-form words (`004fd3c0` at +0x84 for loading, `004fd3e0` at +0x88
/// for saving, the map names the latter `TES::GetWorldSpace` for a folded
/// body) and the version byte (`008df040`).
pub(crate) const USE_SAVE_GAME_BLOCKS: u32 = 0x0086_2110;
pub(crate) const SAVE_POSITION: u32 = 0x0082_5c00;
pub(crate) const SAVE_BYTES: u32 = 0x0085_79b0;
pub(crate) const SAVE_NUMERIC_ID: u32 = 0x0085_7a10;
pub(crate) const LOAD_BYTES: u32 = 0x0085_79e0;
pub(crate) const LOAD_NUMERIC_ID: u32 = 0x0085_7aa0;
pub(crate) const CURRENT_LOAD_FORM: u32 = 0x004f_d3c0;
pub(crate) const CURRENT_SAVE_FORM: u32 = 0x004f_d3e0;
pub(crate) const LOAD_VERSION: u32 = 0x008d_f040;
/// `00408d60(object)`: the address of the flag byte at +4 of `object` (of a
/// global zero byte for null).
pub(crate) const FLAG_BYTE_ADDRESS: u32 = 0x0040_8d60;
/// `008256d0(node)`: true for the head node of an empty `BSSimpleList`
/// (no item, no next node).
pub(crate) const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// Virtual slot 0x130 of a form (called without arguments by the log
/// messages, its result is the `%s` of the message).
pub(crate) const FORM_NAME_SLOT: u32 = 0x130;
/// Callees of `004bffe0` and the other equip/hot key functions.
pub(crate) const GET_FORM_AS_BIPED_MODEL: u32 = 0x0048_0db0;
pub(crate) const BIPED_IS_POWER_ARMOR: u32 = 0x0048_0d10;
pub(crate) const BIPED_FLAG_FOUR: u32 = 0x0048_0d30;
pub(crate) const CONTAINER_COUNT: u32 = 0x0048_2a90;
pub(crate) const REFR_HAS_CONTAINER: u32 = 0x0055_d310;
pub(crate) const FORM_SET_TEMPORARY: u32 = 0x0048_4490;
pub(crate) const REFR_CONSTRUCT: u32 = 0x0055_a2f0;
pub(crate) const REFR_EXTRA_LIST: u32 = 0x005d_43c0;
/// The map names the body of `008d8520` `MiddleHighProcess::GetSavedAcquireObject`;
/// it returns the word at +0x68 of an `Actor` (`pCurrentProcess`).
pub(crate) const ACTOR_PROCESS: u32 = 0x008d_8520;
pub(crate) const ACTOR_RELOAD_TARGETS: u32 = 0x008b_0b00;
pub(crate) const ACTOR_INIT_LIGHTING: u32 = 0x008b_0bd0;
pub(crate) const ACTOR_GET_ANIMATION: u32 = 0x008b_70d0;
pub(crate) const ACTOR_LIGHTING_ARGUMENT: u32 = 0x0043_fcd0;
pub(crate) const HANDLE_ENTRY_POINT: u32 = 0x005e_58f0;
pub(crate) const FORM_WEIGHT_FIELD: u32 = 0x004e_4620;
pub(crate) const ANIMATION_STORE_FLOAT: u32 = 0x004c_0c90;
/// `008248e0(form, 1)` on the sub-object at +0x94 of a reference (the actor):
/// the call `RemoveAllObjectsWorn` makes before removing a worn item.
pub(crate) const REFERENCE_FORM_NOTIFY: u32 = 0x0082_48e0;
pub(crate) const PLAYER_STORE_FORM: u32 = 0x004c_0cd0;
pub(crate) const REFR_UPDATE_WEAPON: u32 = 0x0057_1760;
pub(crate) const PREDICATE_008C7AA0: u32 = 0x008c_7aa0;
pub(crate) const THREAD_VALUE_0047B200: u32 = 0x0047_b200;
pub(crate) const TASK_QUEUE_GETTER: u32 = 0x0045_37b0;
pub(crate) const TASK_QUEUE_ATTACH_WEAPON: u32 = 0x0087_b360;
pub(crate) const STORE_NOTIFY_004534F0: u32 = 0x0045_34f0;
/// `006b9130(weightForm)`: the weight of a `TESWeightForm`; for a weapon it
/// sits at +0x8c.
pub(crate) const GET_WEIGHT: u32 = 0x006b_9130;
pub(crate) const WEAPON_WEIGHT_FORM: u32 = 0x8c;
/// `TESActorBase::GetSex` (Xbox PDB), the inventory icon of a biped form for
/// a sex (`00481230`), the generic icon of a form (`0048e730`), the string
/// object to `char *` conversion (`00403df0`), and the two calls of the full
/// name lookup (`0048cee0` test, `MapMarkerData::GetLocationName` `00408da0`).
pub(crate) const ACTOR_BASE_GET_SEX: u32 = 0x005f_0cc0;
pub(crate) const BIPED_ICON_FOR_SEX: u32 = 0x0048_1230;
pub(crate) const FORM_ICON: u32 = 0x0048_e730;
pub(crate) const STRING_OBJECT_TO_CHARS: u32 = 0x0040_3df0;
pub(crate) const FULL_NAME_TEST: u32 = 0x0048_cee0;
pub(crate) const FULL_NAME_GET: u32 = 0x0040_8da0;
/// The player's hot key table (a map at `+0xe3c`): `006a7ad0(index)` returns
/// the address of the entry, `0047a110(index, &value)` sets it.
pub(crate) const HOT_KEY_TABLE: u32 = 0xe3c;
pub(crate) const HOT_KEY_TABLE_ENTRY: u32 = 0x006a_7ad0;
pub(crate) const HOT_KEY_TABLE_SET: u32 = 0x0047_a110;
/// `TESObjectARMO` fields (PC offsets, the Xbox PDB ones are 0x10 higher):
/// the `TESBipedModelForm` sub-object at +0x70, `OBJ_ARMO::sRating` at +0x178
/// (u16), `fDamageThreshold` at +0x17c and `cFlags` at +0x180.
pub(crate) const ARMOR_BIPED_MODEL: u32 = 0x70;
pub(crate) const ARMOR_RATING: u32 = 0x178;
pub(crate) const ARMOR_DAMAGE_THRESHOLD: u32 = 0x17c;
pub(crate) const ARMOR_FLAGS: u32 = 0x180;
/// `TESObjectWEAP::OBJ_WEAP::eType` (a signed byte) at +0xf4.
pub(crate) const WEAPON_TYPE: u32 = 0xf4;
/// `Actor::pCurrentProcess` (Xbox PDB `+0x78`) on PC.
pub(crate) const ACTOR_CURRENT_PROCESS: u32 = 0x68;

layout! {
    /// `InventoryChanges` (Xbox PDB), 0x14 bytes: the changes of one
    /// container's inventory.
    pub struct InventoryChanges: 0x14 {
        /// `pListofChanges` (Xbox PDB): `BSSimpleList<ItemChange *>*`.
        0x00 pListofChanges: Ptr,
        /// `pRef` (Xbox PDB): the owning `TESObjectREFR*`.
        0x04 pRef: Ptr,
        /// `fcontainerweight` (Xbox PDB).
        0x08 fcontainerweight: f32,
        /// `fpreviousContainerWeight` (Xbox PDB).
        0x0C fpreviousContainerWeight: f32,
        /// `bcountdirty` (Xbox PDB).
        0x10 bcountdirty: bool,
    }
}

/// Runs `body` between the scope guard `00404eb0(guard, 0x36, 1, file,
/// line)` and `00404ee0(guard)` (the guard is a 4-byte local of the game).
fn with_scope_guard<R>(e: &mut Engine, line: u32, body: impl FnOnce(&mut Engine) -> R) -> R {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_OPEN,
            &args![guard, 0x36u32, 1u32, SOURCE_FILE_NAME, line],
        );
        let result = body(e);
        e.call(SCOPE_GUARD_CLOSE, &args![guard]);
        result
    })
}

/// The form type byte of `form` (`00401170`).
fn form_type_of(e: &mut Engine, form: u32) -> u32 {
    e.call(FORM_TYPE, &args![form]).u32()
}

/// The save/load game object the save size/save/load methods run on.
fn save_load_game(e: &mut Engine) -> u32 {
    e.global::<u32>(SAVE_LOAD_GAME_GLOBAL)
}

/// Whether the "log the save sizes" flag byte is set (`00408d60` on the
/// flag object returns the byte's address).
fn save_size_logging(e: &mut Engine) -> bool {
    let flag = e
        .call(FLAG_BYTE_ADDRESS, &args![SAVE_SIZE_LOG_OBJECT])
        .u32();
    e.mem.u8(flag) != 0
}

/// `new ItemChange(form, number)`: 0xC bytes, constructed unless the
/// allocation failed.
fn new_item_change(e: &mut Engine, form: u32, number: u32) -> u32 {
    let item = e.call(OPERATOR_NEW, &args![0xcu32]).u32();
    if item == 0 {
        return 0;
    }
    item_change_item_change(e, Ptr::new(item), Ptr::new(form), number as i32).addr()
}

/// `delete item` of an `ItemChange` through `004459e0(item, 1)`, nothing for
/// null.
fn delete_item_change(e: &mut Engine, item: u32) {
    if item != 0 {
        e.call(ITEM_CHANGE_DELETE, &args![item, 1u32]);
    }
}

/// Makes sure `*(item + 0)` (the `pExtraObjectList` of an `ItemChange`, the
/// list of an entry) is a list: creates the empty head node when it is null.
fn ensure_extra_list(e: &mut Engine, item: u32) {
    if e.mem.u32(item) == 0 {
        let node = new_list_node(e);
        e.mem.set_u32(item, node);
    }
}

/// The first part of the error messages of the save size and save code: the
/// log line for the form being saved (`world`, the word the save/load object
/// keeps) or without one.
fn log_save_size(e: &mut Engine, saved: u32, current: u32, line: u32, formats: (u32, u32)) {
    if current != 0 {
        let form_id = e.mem.u32(current);
        let form = e.call(LOOKUP_FORM_BY_ID, &args![form_id]).u32();
        // The 0x130 virtual returns the `%s` of the message; the two words
        // pushed before it stay on the stack as the message's own arguments.
        let name = e.vcall(form, FORM_NAME_SLOT, &args![]).u32();
        let flags = e.mem.u32(current + 5);
        e.call(
            ERROR_LOG,
            &args![
                formats.0,
                saved,
                form_id,
                name,
                flags,
                line,
                SOURCE_FILE_NAME
            ],
        );
    } else {
        e.call(ERROR_LOG, &args![formats.1, saved, line, SOURCE_FILE_NAME]);
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

// Translated from 004be060 (decompiled, FalloutNV.exe 1.4.0.525)
/// The armor's rating as a float: forwards to `004be080` (the game's
/// wrapper).
pub fn fn_004be060(e: &mut Engine, this: Ptr) -> f32 {
    fn_004be080(e, this)
}

// Translated from 004be080 (decompiled, FalloutNV.exe 1.4.0.525)
/// The armor's rating as a float: `OBJ_ARMO::sRating` (a 16-bit value at
/// +0x178 of a `TESObjectARMO`) divided by 100.
pub fn fn_004be080(e: &mut Engine, this: Ptr) -> f32 {
    let rating = e.mem.u16(this.addr() + ARMOR_RATING);
    let hundred: f64 = e.global(HUNDRED_DOUBLE);
    (rating as f64 / hundred) as f32
}

// Translated from 004be0b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The damage threshold of the item as the game shows it: for a form of type
/// 0x18 the form's `fDamageThreshold` (`004be180`) truncated to 16 bits goes
/// through `CalcArmorRating(threshold, ratio)` with the item's health ratio
/// (the item health over the form's health, zero without form health);
/// another form gives `-1.0`. The result goes through `00476b20`. The stack
/// word is not read.
pub fn fn_004be0b0(e: &mut Engine, this: Ptr<ItemChange>, _unused_1: u32) -> f32 {
    let mut result = e.global::<f32>(MINUS_ONE_FLOAT);
    let form = container_object(e, this);
    if form_type_of(e, form) == 0x18 {
        let form_health = e.call(GET_FORM_HEALTH, &args![form]).u32() as f32;
        let zero: f64 = e.global(ZERO_DOUBLE);
        let ratio = if form_health as f64 == zero {
            0.0f32
        } else {
            let health = item_change_get_item_health(e, this, false);
            (health as f64 / form_health as f64) as f32
        };
        let threshold = fn_004be180(e, Ptr::new(form));
        let whole = truncate_to_i32(threshold as f64);
        result = e
            .call(CALC_ARMOR_RATING, &args![whole as u16 as u32, ratio])
            .f32();
    }
    e.call(ROUND_ARMOR_RESULT, &args![result]).f32()
}

// Translated from 004be180 (decompiled, FalloutNV.exe 1.4.0.525)
/// `OBJ_ARMO::fDamageThreshold` (Xbox PDB) of a `TESObjectARMO`, at +0x17c.
pub fn fn_004be180(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + ARMOR_DAMAGE_THRESHOLD)
}

// Translated from 004be1a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A small class number of the item's form: 4 for form types 0x18 and 0x1a,
/// 3 for 0x28, 5 for every other form. (The meaning of the numbers is not
/// confirmed by the exe.)
pub fn fn_004be1a0(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let form = container_object(e, this);
    match form_type_of(e, form) {
        0x18 | 0x1a => 4,
        0x28 => 3,
        _ => 5,
    }
}

// Translated from 004be200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetIconOne` (Xbox PDB): the inventory icon path of the item.
/// For a biped form (`TESBipedModelForm`) whose item is owned by a `TESNPC`
/// the icon for the owner's sex (`00481230`); otherwise, or when that is
/// empty, the form's own icon for `variant` (`0048e730`); when that is empty
/// too the string object `011d33f0` as a `char *`.
pub fn item_change_get_icon_one(e: &mut Engine, this: Ptr<ItemChange>, variant: u32) -> u32 {
    let mut icon = 0u32;
    let owner = item_change_get_item_ownership(e, this);
    let form = container_object(e, this);
    let biped = e
        .call(
            RT_DYNAMIC_CAST,
            &args![
                form,
                0u32,
                TYPE_TES_BOUND_OBJECT,
                TYPE_TES_BIPED_MODEL_FORM,
                0u32
            ],
        )
        .u32();
    if owner != 0 && biped != 0 {
        let npc = e
            .call(
                RT_DYNAMIC_CAST,
                &args![owner, 0u32, TYPE_TES_FORM, TYPE_TES_NPC, 0u32],
            )
            .u32();
        if npc != 0 {
            let sex = e.call(ACTOR_BASE_GET_SEX, &args![npc]).u32();
            icon = e.call(BIPED_ICON_FOR_SEX, &args![biped, sex]).u32();
        }
    }
    if icon == 0 || e.mem.u8(icon) == 0 {
        let form = container_object(e, this);
        icon = e.call(FORM_ICON, &args![form, variant]).u32();
    }
    if icon == 0 || e.mem.u8(icon) == 0 {
        return e.call(STRING_OBJECT_TO_CHARS, &args![NO_ICON_STRING]).u32();
    }
    icon
}

// Translated from 004be2d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::GetFullName` (Xbox PDB): the item's name. The form cast to
/// `TESFullName`; when it is one and `0048cee0` accepts it, its name
/// (`00408da0`, the map's `MapMarkerData::GetLocationName`), otherwise the
/// string object `011d35b8` as a `char *`.
pub fn item_change_get_full_name(e: &mut Engine, this: Ptr<ItemChange>) -> u32 {
    let form = container_object(e, this);
    let full_name = e
        .call(
            RT_DYNAMIC_CAST,
            &args![form, 0u32, TYPE_TES_BOUND_OBJECT, TYPE_TES_FULL_NAME, 0u32],
        )
        .u32();
    if full_name != 0 && e.call(FULL_NAME_TEST, &args![full_name]).u32() != 0 {
        return e.call(FULL_NAME_GET, &args![full_name]).u32();
    }
    e.call(STRING_OBJECT_TO_CHARS, &args![NO_NAME_STRING]).u32()
}

// Translated from 004be330 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESEnchantableForm::GetFormEnchanting` (Xbox PDB), a `__cdecl` function of
/// a form: the form cast to `TESEnchantableForm` and its enchantment (the word
/// at +4, `00726070`), 0 when the form is not enchantable.
pub fn tes_enchantable_form_get_form_enchanting(e: &mut Engine, form: u32) -> u32 {
    let enchantable = e
        .call(
            RT_DYNAMIC_CAST,
            &args![form, 0u32, TYPE_TES_FORM, TYPE_TES_ENCHANTABLE_FORM, 0u32],
        )
        .u32();
    if enchantable == 0 {
        return 0;
    }
    e.call(WORD_AT_4, &args![enchantable]).u32()
}

// Translated from 004be380 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectWEAP::GetFormWeight` (Xbox PDB): the weapon's weight (the
/// `TESWeightForm` sub-object at +0x8c, `006b9130`); with `with_mods` the
/// value of the weight mod effect (type 4) is subtracted.
pub fn tes_object_weap_get_form_weight(e: &mut Engine, this: Ptr, with_mods: u8) -> f32 {
    let weight = e
        .call(GET_WEIGHT, &args![this.addr() + WEAPON_WEIGHT_FORM])
        .f32();
    if with_mods == 0 {
        return weight;
    }
    let bonus = tes_object_weap_get_mod_effect_value(e, this, 4, 0);
    (weight as f64 - bonus as f64) as f32
}

// Translated from 004be3d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::SetWorn` (Xbox PDB): with `wear` a new extra data list
/// marked worn (`SetWorn(1, flag)`) is appended to the item's lists (their
/// list node is created when there is none). Without it the first extra list
/// that is worn (`GetWorn(flag)`) is taken out of the list, and deleted when
/// `delete_list` is set. The C++ exception frame is not translated.
pub fn item_change_set_worn(
    e: &mut Engine,
    this: Ptr<ItemChange>,
    wear: u8,
    flag: u8,
    delete_list: u8,
) {
    if wear != 0 {
        let extra = new_extra_list(e);
        e.call(EXTRA_SET_WORN, &args![extra, 1u32, flag as u32]);
        if list_head(e, this) == 0 {
            let node = new_list_node(e);
            e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
        }
        let list = list_head(e, this);
        list_call_with_item(e, LIST_ADD, list, extra);
        return;
    }
    if !item_change_get_worn(e, this, flag) {
        return;
    }
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            return;
        }
        if e.call(EXTRA_GET_WORN, &args![extra, flag as u32]).bool() {
            list_call_with_item(e, LIST_REMOVE, cursor, extra);
            if delete_list != 0 {
                delete_object(e, extra);
            }
            return;
        }
        cursor = list_next(e, cursor);
    }
}

// Translated from 004be580 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at +0x240 of the object (a getter, called by `FilterFunc`).
pub fn fn_004be580(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x240)
}

// Translated from 004be5a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange` save size (the 16-bit size of `SaveGame`): 6 more bytes when
/// save game blocks are used, 12 for the form id, the count and the list
/// count, and the save size (`00422c40`) of every extra list. With the size
/// logging flag set the size is reported through `Error` (the message names
/// the form being saved when there is one).
pub fn fn_004be5a0(e: &mut Engine, this: Ptr<ItemChange>) -> u16 {
    let save = save_load_game(e);
    let mut size: u16 = 0;
    if e.call(USE_SAVE_GAME_BLOCKS, &args![save]).bool() {
        size = size.wrapping_add(4);
        size = size.wrapping_add(2);
    }
    size = size.wrapping_add(8);
    size = size.wrapping_add(4);
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        if e.call(LIST_IS_EMPTY, &args![cursor]).bool() {
            break;
        }
        let extra = list_item(e, cursor);
        let extra_size = e.call(EXTRA_GET_SAVE_SIZE, &args![extra, 0u32, 0u32]).u16();
        size = size.wrapping_add(extra_size);
        cursor = list_next(e, cursor);
    }
    if save_size_logging(e) {
        let current = e.call(CURRENT_SAVE_FORM, &args![save]).u32();
        // "GetSaveSize(): %-5i for form %08X %s with flags %08X ending at
        // line %i in file %s", or without the form part.
        log_save_size(e, size as u32, current, 0x622, (0x0101_2cb0, 0x0101_2c78));
    }
    size
}

// Translated from 004be6f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange` save (to the save/load game object): with save game blocks
/// the block tag `BLOK` and a 16-bit length (patched at the end), then the
/// item form's numeric id, the count, the number of extra lists (patched
/// after the lists) and the lists (`ExtraDataList::SaveGame(0, 0)`). With the
/// logging flag set the bytes written are reported through `Error`; a block
/// over 0xFFFF bytes is reported through `005b5e40`.
pub fn fn_004be6f0(e: &mut Engine, this: Ptr<ItemChange>) {
    let save = save_load_game(e);
    let start = e.call(SAVE_POSITION, &args![save]).u32();
    let start = if save_size_logging(e) {
        e.call(SAVE_POSITION, &args![save]).u32()
    } else {
        start
    };
    // Locals of the game's frame passed by address: the tag, the form id,
    // the block length and the list count.
    e.with_stack(0x10, |e, locals| {
        let (tag, form_id_slot, length_slot, count_slot) = (
            locals.addr(),
            locals.addr() + 4,
            locals.addr() + 8,
            locals.addr() + 12,
        );
        let mut block = 0u32;
        if e.call(USE_SAVE_GAME_BLOCKS, &args![save]).bool() {
            e.mem.set_u32(tag, 0x424c_4f4b);
            e.call(SAVE_BYTES, &args![save, tag, 4u32]);
            block = e.call(SAVE_POSITION, &args![save]).u32();
            e.call(SAVE_BYTES, &args![save, length_slot, 2u32]);
        }
        let form = container_object(e, this);
        let form_id = e.call(FORM_ID, &args![form]).u32();
        e.mem.set_u32(form_id_slot, form_id);
        e.call(SAVE_NUMERIC_ID, &args![save, form_id_slot, 4u32]);
        let number = this.addr() + 4;
        e.call(SAVE_BYTES, &args![save, number, 4u32]);
        e.mem.set_u32(count_slot, 0);
        let count_position = e.call(SAVE_POSITION, &args![save]).u32();
        e.call(SAVE_BYTES, &args![save, count_slot, 4u32]);
        let mut cursor = list_head(e, this);
        while cursor != 0 {
            if e.call(LIST_IS_EMPTY, &args![cursor]).bool() {
                break;
            }
            let extra = list_item(e, cursor);
            e.call(EXTRA_SAVE_GAME, &args![extra, 0u32, 0u32]);
            let count = e.mem.u32(count_slot);
            e.mem.set_u32(count_slot, count.wrapping_add(1));
            cursor = list_next(e, cursor);
        }
        let count = e.mem.u32(count_slot);
        e.mem.set_u32(count_position, count);
        if save_size_logging(e) {
            let end = e.call(SAVE_POSITION, &args![save]).u32();
            let current = e.call(CURRENT_SAVE_FORM, &args![save]).u32();
            // "SaveGame(): %-5i for form %08X %s with flags %08X ending at
            // line %i in file %s", or without the form part.
            log_save_size(
                e,
                end.wrapping_sub(start),
                current,
                0x641,
                (0x0101_53a0, 0x0101_536c),
            );
        }
        if e.call(USE_SAVE_GAME_BLOCKS, &args![save]).bool() {
            let end = e.call(SAVE_POSITION, &args![save]).u32();
            if end > block.wrapping_add(0xffff) {
                e.call(
                    SAVE_LOAD_LOG,
                    &args![0x0101_5318u32, SOURCE_FILE_NAME, 0x641u32],
                );
            }
            e.mem.set_u16(block, end.wrapping_sub(block) as u16);
        }
    });
}

// Translated from 004be930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::LoadGame` (Xbox PDB): reads what `fn_004be6f0` wrote. With
/// save game blocks the `BLOK` tag is checked (a wrong tag is logged through
/// `005b5e40`) and the 16-bit length read; then the numeric id (looked up
/// and cast from `TESForm` to `TESBoundObject` into `pContainerObj`), the
/// count, and the extra lists, each in a new `ExtraDataList` loaded with
/// `LoadGame(0, 0, 0)` and appended. The block length is checked against the
/// bytes read (overrun and underrun are logged). The C++ exception frame is
/// not translated.
pub fn item_change_load_game(e: &mut Engine, this: Ptr<ItemChange>) {
    let save = save_load_game(e);
    let mut length = 0u16;
    let mut block = 0u32;
    // 0x10-byte frame of the locals passed by address: tag, form id,
    // length, number of lists.
    e.with_stack(0x10, |e, locals| {
        let (tag, form_id_slot, length_slot, count_slot) = (
            locals.addr(),
            locals.addr() + 4,
            locals.addr() + 8,
            locals.addr() + 12,
        );
        if e.call(USE_SAVE_GAME_BLOCKS, &args![save]).bool() {
            e.call(LOAD_BYTES, &args![save, tag, 4u32]);
            if e.mem.u32(tag) != 0x424c_4f4b {
                let current = e.call(CURRENT_LOAD_FORM, &args![save]).u32();
                if current != 0 {
                    let form_id = e.mem.u32(current);
                    let form = e.call(LOOKUP_FORM_BY_ID, &args![form_id]).u32();
                    let name = e.vcall(form, FORM_NAME_SLOT, &args![]).u32();
                    let version = e.mem.u8(current + 9) as u32;
                    let flags = e.mem.u32(current + 5);
                    e.call(
                        SAVE_LOAD_LOG,
                        &args![
                            0x0101_5718u32,
                            SOURCE_FILE_NAME,
                            0x647u32,
                            form_id,
                            name,
                            version,
                            flags
                        ],
                    );
                } else {
                    let version = e.call(LOAD_VERSION, &args![save]).u8() as u32;
                    e.call(
                        SAVE_LOAD_LOG,
                        &args![0x0101_56a8u32, SOURCE_FILE_NAME, 0x647u32, version],
                    );
                }
            }
            block = e.call(SAVE_POSITION, &args![save]).u32();
            e.call(LOAD_BYTES, &args![save, length_slot, 2u32]);
            length = e.mem.u16(length_slot);
        }
        e.call(LOAD_NUMERIC_ID, &args![save, form_id_slot, 4u32]);
        let number = this.addr() + 4;
        e.call(LOAD_BYTES, &args![save, number, 4u32]);
        let form_id = e.mem.u32(form_id_slot);
        let form = e.call(LOOKUP_FORM_BY_ID, &args![form_id]).u32();
        let cast = e
            .call(
                RT_DYNAMIC_CAST,
                &args![form, 0u32, TYPE_TES_FORM, TYPE_TES_BOUND_OBJECT, 0u32],
            )
            .u32();
        e.set(this, ItemChange::pContainerObj, Ptr::new(cast));
        e.set(this, ItemChange::pExtraObjectList, Ptr::NULL);
        e.call(LOAD_BYTES, &args![save, count_slot, 4u32]);
        let lists = e.mem.i32(count_slot);
        if lists != 0 {
            let node = new_list_node(e);
            e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
            let mut index = 0;
            while index < lists {
                let extra = new_extra_list(e);
                e.call(EXTRA_LOAD_GAME, &args![extra, 0u32, 0u32, 0u32]);
                let list = list_head(e, this);
                list_call_with_item(e, LIST_ADD_TAIL, list, extra);
                index += 1;
            }
        }
    });
    if e.call(USE_SAVE_GAME_BLOCKS, &args![save]).bool() {
        let position = e.call(SAVE_POSITION, &args![save]).u32();
        let current = e.call(CURRENT_LOAD_FORM, &args![save]).u32();
        let expected = (length as u32).wrapping_add(block);
        // The messages: "SAVELOAD: LoadGame Buffer overrun/underrun of %i
        // bytes in file %s on line %i.  Current version is %i", or with the
        // form being loaded.
        let (overrun, underrun) = if current != 0 {
            (0x0101_5588u32, 0x0101_5500u32)
        } else {
            (0x0101_54a0u32, 0x0101_5440u32)
        };
        let difference = if position > expected {
            Some((overrun, position - expected))
        } else if position < expected {
            Some((underrun, expected - position))
        } else {
            None
        };
        if current != 0 {
            let form_id = e.mem.u32(current);
            let form = e.call(LOOKUP_FORM_BY_ID, &args![form_id]).u32();
            if let Some((format, bytes)) = difference {
                let name = e.vcall(form, FORM_NAME_SLOT, &args![]).u32();
                let version = e.mem.u8(current + 9) as u32;
                let flags = e.mem.u32(current + 5);
                e.call(
                    SAVE_LOAD_LOG,
                    &args![
                        format,
                        bytes,
                        SOURCE_FILE_NAME,
                        0x66bu32,
                        form_id,
                        name,
                        version,
                        flags
                    ],
                );
            }
        } else if let Some((format, bytes)) = difference {
            let version = e.call(LOAD_VERSION, &args![save]).u8() as u32;
            e.call(
                SAVE_LOAD_LOG,
                &args![format, bytes, SOURCE_FILE_NAME, 0x66bu32, version],
            );
        }
    }
}

// Translated from 004bed00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `00426020(0, 0, 0, form)` on every extra data list of the item
/// (after loading): the walk stops at an empty head node.
pub fn fn_004bed00(e: &mut Engine, this: Ptr<ItemChange>) {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        if e.call(LIST_IS_EMPTY, &args![cursor]).bool() {
            break;
        }
        let extra = list_item(e, cursor);
        if extra != 0 {
            let form = container_object(e, this);
            e.call(EXTRA_AFTER_LOAD_GAME, &args![extra, 0u32, 0u32, 0u32, form]);
        }
        cursor = list_next(e, cursor);
    }
}

// Translated from 004bed60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::SaveGame` (Xbox PDB), the buffer version: the form id, the
/// count (4 bytes), then a variable sized value holding the number of extra
/// lists written (`ExtraDataList::SaveGame_ov2(buffer)` for each non-null
/// one).
pub fn item_change_save_game(e: &mut Engine, this: Ptr<ItemChange>, buffer: u32) {
    let form = container_object(e, this);
    e.call(SAVE_BUFFER_SAVE_FORM_ID, &args![buffer, form, 0u32]);
    let number = this.addr() + 4;
    e.call(SAVE_BUFFER_SAVE_DATA, &args![buffer, number, 4u32, 0u32]);
    let mut written = 0u32;
    let start = e.call(SAVE_BUFFER_START_SIZED, &args![buffer]).u32();
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra != 0 {
            e.call(EXTRA_SAVE_GAME_BUFFER, &args![extra, buffer]);
            written += 1;
        }
        cursor = list_next(e, cursor);
    }
    e.call(SAVE_BUFFER_END_SIZED, &args![buffer, written, start]);
}

// Translated from 004bee00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ItemChange::LoadGame_ov2` (Xbox PDB), the buffer version: the form
/// (loaded id, looked up, cast from `TESForm` to `TESBoundObject`), the count
/// (4 bytes), the old extra lists deleted (`DeleteAllExtra`), the number of
/// lists as a variable sized value and each list loaded in a new
/// `ExtraDataList` (`LoadGame_ov2(buffer)`) and appended. The C++ exception
/// frame is not translated.
pub fn item_change_load_game_ov2(e: &mut Engine, this: Ptr<ItemChange>, buffer: u32) {
    let form_id = e.call(LOAD_BUFFER_LOAD_FORM_ID, &args![buffer]).u32();
    let form = e.call(LOOKUP_FORM_BY_ID, &args![form_id]).u32();
    let cast = e
        .call(
            RT_DYNAMIC_CAST,
            &args![form, 0u32, TYPE_TES_FORM, TYPE_TES_BOUND_OBJECT, 0u32],
        )
        .u32();
    e.set(this, ItemChange::pContainerObj, Ptr::new(cast));
    let number = this.addr() + 4;
    e.call(LOAD_BUFFER_LOAD_DATA, &args![buffer, number, 4u32]);
    item_change_delete_all_extra(e, this);
    let lists = e.call(LOAD_BUFFER_LOAD_SIZED, &args![buffer]).u32();
    if lists != 0 {
        if list_head(e, this) == 0 {
            let node = new_list_node(e);
            e.set(this, ItemChange::pExtraObjectList, Ptr::new(node));
        }
        let mut index = 0u32;
        while index < lists {
            let extra = new_extra_list(e);
            e.call(EXTRA_LOAD_GAME_BUFFER, &args![extra, buffer]);
            let list = list_head(e, this);
            list_call_with_item(e, LIST_ADD_TAIL, list, extra);
            index += 1;
        }
    }
}

// Translated from 004bef60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `0042ceb0(argument, form)` on every non-null extra data list of the
/// item (the finishing step after loading with the buffer).
pub fn fn_004bef60(e: &mut Engine, this: Ptr<ItemChange>, argument: u32) {
    let mut cursor = list_head(e, this);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra != 0 {
            let form = container_object(e, this);
            e.call(EXTRA_AFTER_LOAD_GAME_BUFFER, &args![extra, argument, form]);
        }
        cursor = list_next(e, cursor);
    }
}

// Translated from 004befb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryChanges::InventoryChanges` (Xbox PDB): stores the owner
/// reference, makes an empty list of changes, creates the shared scratch
/// `TESObjectREFR` (`pTempRef`, Xbox PDB; made temporary with
/// `TESForm::SetTemporary`) the first time, resets the weights to `-1.0`
/// (`fn_004bf0e0`) and clears the count dirty flag. The C++ exception frame
/// is not translated.
pub fn inventory_changes_inventory_changes(
    e: &mut Engine,
    this: Ptr<InventoryChanges>,
    owner: Ptr,
) -> Ptr<InventoryChanges> {
    with_scope_guard(e, 0x6de, |e| {
        e.set(this, InventoryChanges::pRef, owner);
        let node = new_list_node(e);
        e.set(this, InventoryChanges::pListofChanges, Ptr::new(node));
        if e.global::<u32>(TEMP_REF_GLOBAL) == 0 {
            let memory = e.call(OPERATOR_NEW, &args![0x68u32]).u32();
            let temp_ref = if memory == 0 {
                0
            } else {
                e.call(REFR_CONSTRUCT, &args![memory]).u32()
            };
            e.set_global(TEMP_REF_GLOBAL, temp_ref);
            let temp_ref = e.global::<u32>(TEMP_REF_GLOBAL);
            e.call(FORM_SET_TEMPORARY, &args![temp_ref]);
        }
        let none = e.global::<f32>(MINUS_ONE_FLOAT);
        e.set(this, InventoryChanges::fcontainerweight, none);
        let weight = e.get(this, InventoryChanges::fcontainerweight);
        e.set(this, InventoryChanges::fpreviousContainerWeight, weight);
        fn_004bf0e0(e, this);
        e.set(this, InventoryChanges::bcountdirty, false);
    });
    this
}

// Translated from 004bf0e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forgets the cached container weight: the previous weight becomes the
/// current one, the current one `-1.0`, and when the owner reference's
/// virtual 0x100 says so the owner is told (virtual 0x30c with `-1.0`).
pub fn fn_004bf0e0(e: &mut Engine, this: Ptr<InventoryChanges>) {
    let weight = e.get(this, InventoryChanges::fcontainerweight);
    e.set(this, InventoryChanges::fpreviousContainerWeight, weight);
    let none = e.global::<f32>(MINUS_ONE_FLOAT);
    e.set(this, InventoryChanges::fcontainerweight, none);
    let owner = e.get(this, InventoryChanges::pRef).addr();
    if owner != 0 && e.vcall(owner, 0x100, &args![]).bool() {
        let owner = e.get(this, InventoryChanges::pRef).addr();
        let none = e.global::<f32>(MINUS_ONE_FLOAT);
        e.vcall(owner, 0x30c, &args![none]);
    }
}

// Translated from 004bf150 (decompiled, FalloutNV.exe 1.4.0.525)
/// The body of the `InventoryChanges` destructor: every `ItemChange` of the
/// list gets `DeleteAllExtra` and is deleted (the next node is taken first),
/// the list is cleared, the owner is told (virtual 0x4c with `0x8000020`) and
/// the list deleted.
pub fn fn_004bf150(e: &mut Engine, this: Ptr<InventoryChanges>) {
    let mut cursor = e.get(this, InventoryChanges::pListofChanges).addr();
    while cursor != 0 {
        let item = list_item(e, cursor);
        if item == 0 {
            break;
        }
        cursor = list_next(e, cursor);
        item_change_delete_all_extra(e, Ptr::new(item));
        delete_item_change(e, item);
    }
    let list = e.get(this, InventoryChanges::pListofChanges).addr();
    e.call(LIST_CLEAR, &args![list]);
    let owner = e.get(this, InventoryChanges::pRef).addr();
    if owner != 0 {
        e.vcall(owner, 0x4c, &args![0x0800_0020u32]);
    }
    let list = e.get(this, InventoryChanges::pListofChanges).addr();
    if list != 0 {
        e.call(LIST_DESTROY, &args![list, 1u32]);
    }
}

// Translated from 004bf220 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryChanges::GetInventoryChanges` (Xbox PDB), a `__cdecl` function of
/// a reference: the `InventoryChanges` kept in the reference's extra data
/// (`GetContainerChanges` on the list `005d43c0` returns), made with the
/// constructor and stored (`SetContainerChanges`) when there is none. Nothing
/// is made (0 returned) when virtual 0xfc is true and virtual 0x100 false.
/// The C++ exception frame is not translated.
pub fn inventory_changes_get_inventory_changes(e: &mut Engine, reference: u32) -> u32 {
    if e.vcall(reference, 0xfc, &args![]).bool() && !e.vcall(reference, 0x100, &args![]).bool() {
        return 0;
    }
    let extra_list = e.call(REFR_EXTRA_LIST, &args![reference]).u32();
    let mut changes = e
        .call(EXTRA_GET_CONTAINER_CHANGES, &args![extra_list])
        .u32();
    if changes == 0 {
        changes = with_scope_guard(e, 0x71d, |e| {
            let memory = e.call(OPERATOR_NEW, &args![0x14u32]).u32();
            let created = if memory == 0 {
                0
            } else {
                inventory_changes_inventory_changes(e, Ptr::new(memory), Ptr::new(reference)).addr()
            };
            let extra_list = e.call(REFR_EXTRA_LIST, &args![reference]).u32();
            e.call(EXTRA_SET_CONTAINER_CHANGES, &args![extra_list, created]);
            created
        });
    }
    changes
}

// Translated from 004bf330 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryChanges::RemoveHotKeyItem` (Xbox PDB): takes the hot key off the
/// extra lists of the entry for the item's form (`GetObjectInList`): all
/// hot keys with `hot_key == -1`, else the lists with that key. A list left
/// without extras (`fn_004bca60`), or one of a count above 1 that holds one
/// kind of extra, is removed from the entry (the walk then restarts at the
/// head). An entry left without lists is emptied, and when the item's count
/// is zero removed from the changes and deleted.
pub fn inventory_changes_remove_hot_key_item(
    e: &mut Engine,
    this: Ptr<InventoryChanges>,
    item: Ptr<ItemChange>,
    hot_key: i32,
) {
    let form = container_object(e, item);
    let entry = inventory_changes_get_object_in_list(e, this, form, 1, 0);
    if entry == 0 {
        return;
    }
    let mut cursor = e.mem.u32(entry);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra != 0 {
            let mut matches = false;
            if hot_key != -1 {
                let key = e.call(EXTRA_GET_HOT_KEY, &args![extra]).u8() as i8;
                if key as i32 == hot_key {
                    matches = true;
                }
            }
            if !matches && hot_key == -1 {
                let key = e.call(EXTRA_GET_HOT_KEY, &args![extra]).u8() as i8;
                if key >= 0 {
                    matches = true;
                }
            }
            if matches {
                e.call(EXTRA_REMOVE_HOT_KEY, &args![extra]);
                let emptied = fn_004bca60(e, Ptr::new(extra));
                let single_kind = !emptied && {
                    let count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16;
                    count > 1 && e.call(EXTRA_ITEMS_IN_LIST, &args![extra]).i32() == 1
                };
                if emptied || single_kind {
                    let list = e.mem.u32(entry);
                    list_call_with_item(e, LIST_REMOVE, list, extra);
                    cursor = e.mem.u32(entry);
                } else {
                    cursor = list_next(e, cursor);
                }
            } else {
                cursor = list_next(e, cursor);
            }
        } else {
            cursor = list_next(e, cursor);
        }
    }
    let list = e.mem.u32(entry);
    if list != 0 && e.call(LIST_IS_EMPTY, &args![list]).bool() {
        let list = e.mem.u32(entry);
        if list != 0 {
            e.call(LIST_DESTROY, &args![list, 1u32]);
        }
        e.mem.set_u32(entry, 0);
        if e.get(Ptr::<ItemChange>::new(entry), ItemChange::iNumber) == 0 {
            let changes = e.get(this, InventoryChanges::pListofChanges).addr();
            list_call_with_item(e, LIST_REMOVE, changes, entry);
            delete_item_change(e, entry);
        }
    }
}

// Translated from 004bf4b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `ItemChange` (a new copy made by the fast inventory iteration) that
/// has an extra list with the hot key `hot_key`, 0 when there is none. The
/// iteration (`StartOEIFastInventoryIteration`/`GetNextOEIFastInventoryItem`)
/// hands out new items: the ones without a match are deleted, the iterator
/// too. The C++ exception frame is not translated.
pub fn fn_004bf4b0(e: &mut Engine, this: Ptr<InventoryChanges>, hot_key: i32) -> u32 {
    with_scope_guard(e, 0x76a, |e| {
        let iterator = e.call(START_FAST_ITERATION, &args![this]).u32();
        loop {
            let item = e.call(GET_NEXT_FAST_ITEM, &args![this, iterator]).u32();
            if item != 0 {
                let mut cursor = e.call(WORD_AT_0, &args![item]).u32();
                while cursor != 0 {
                    let extra = list_item(e, cursor);
                    if extra == 0 {
                        break;
                    }
                    let key = e.call(EXTRA_GET_HOT_KEY, &args![extra]).u8() as i8;
                    if key as i32 == hot_key {
                        if iterator != 0 {
                            oei_fast_inventory_iterator_scalar_deleting_destructor(e, iterator, 1);
                        }
                        return item;
                    }
                    cursor = list_next(e, cursor);
                }
                delete_item_change(e, item);
            }
            if item == 0 {
                break;
            }
        }
        if iterator != 0 {
            oei_fast_inventory_iterator_scalar_deleting_destructor(e, iterator, 1);
        }
        0
    })
}

// Translated from 004bf630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `OEI_Fast_InventoryIterator::_scalar_deleting_destructor_` (Xbox PDB):
/// runs the destructor body (`004ca1a0`) and frees the iterator when bit 0 of
/// `flags` is set; returns `this`.
pub fn oei_fast_inventory_iterator_scalar_deleting_destructor(
    e: &mut Engine,
    this: u32,
    flags: u32,
) -> u32 {
    e.call(FAST_ITERATOR_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 004bf660 (decompiled, FalloutNV.exe 1.4.0.525)
/// A new array of `high - low + 1` `ItemChange` pointers (zeroed) indexed by
/// hot key minus `low`: for every item of the fast inventory iteration the
/// extra lists with a hot key in `low..=high` put the item in their slot
/// (deleting an item already there). Items that were not used are deleted,
/// the iterator too. 0 when `low > high`.
pub fn fn_004bf660(e: &mut Engine, this: Ptr<InventoryChanges>, low: i32, high: i32) -> u32 {
    if low > high {
        return 0;
    }
    let count = (high.wrapping_sub(low).wrapping_add(1)) as u32;
    // `count * 4`, the allocation size saturating on overflow.
    let size = count.saturating_mul(4);
    let array = e.call(OPERATOR_NEW, &args![size]).u32();
    e.call(MEMSET, &args![array, 0u32, count << 2]);
    let iterator = e.call(START_FAST_ITERATION, &args![this]).u32();
    loop {
        let item = e.call(GET_NEXT_FAST_ITEM, &args![this, iterator]).u32();
        if item != 0 {
            let mut used = false;
            let mut cursor = e.call(WORD_AT_0, &args![item]).u32();
            while cursor != 0 {
                let extra = list_item(e, cursor);
                if extra == 0 {
                    break;
                }
                let key = e.call(EXTRA_GET_HOT_KEY, &args![extra]).u8() as i8 as i32;
                if key >= low && key <= high {
                    let slot = array.wrapping_add(((key - low) as u32).wrapping_mul(4));
                    let previous = e.mem.u32(slot);
                    delete_item_change(e, previous);
                    e.mem.set_u32(slot, item);
                    used = true;
                }
                cursor = list_next(e, cursor);
            }
            if !used {
                delete_item_change(e, item);
            }
        }
        if item == 0 {
            break;
        }
    }
    if iterator != 0 {
        oei_fast_inventory_iterator_scalar_deleting_destructor(e, iterator, 1);
    }
    array
}

// Translated from 004bf800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryChanges::SetHotKeyItem` (Xbox PDB): gives the hot key
/// `hot_key` (0 to 7) to `extra` of `item`. Without such arguments nothing is
/// done. The player's hot key slot (`004bfb30`) is cleared (`004bfb70(key,
/// 0)`) when set, the item's other hot keys removed
/// (`RemoveHotKeyItem(item, -1)`). When the changes have an entry for the
/// item's form the key goes to `extra` if the entry has it, else to the
/// default list of the entry, else to a new list holding the item count not
/// yet in lists (`iNumber - extra count - 1 if worn`); with no entry a new
/// `ItemChange` with a new list holding the key is added. The C++ exception
/// frame is not translated.
pub fn inventory_changes_set_hot_key_item(
    e: &mut Engine,
    this: Ptr<InventoryChanges>,
    item: Ptr<ItemChange>,
    extra: u32,
    hot_key: i32,
) {
    with_scope_guard(e, 0x7b6, |e| {
        if item.is_null() || !(0..=7).contains(&hot_key) {
            return;
        }
        let player = e.global::<u32>(PLAYER_GLOBAL);
        if fn_004bfb30(e, player, hot_key) != 0 {
            fn_004bfb70(e, player, hot_key, 0);
        }
        inventory_changes_remove_hot_key_item(e, this, item, -1);
        let form = container_object(e, item);
        let entry = inventory_changes_get_object_in_list(e, this, form, 1, 0);
        if entry != 0 {
            let mut chosen = 0u32;
            ensure_extra_list(e, entry);
            let mut cursor = e.mem.u32(entry);
            while cursor != 0 {
                let candidate = list_item(e, cursor);
                if candidate == 0 {
                    break;
                }
                if chosen == 0 {
                    let candidate = list_item(e, cursor);
                    if e.call(EXTRA_IS_DEFAULT_FOR_CONTAINER, &args![candidate, 0u32])
                        .bool()
                    {
                        chosen = list_item(e, cursor);
                    }
                }
                if list_item(e, cursor) == extra {
                    chosen = extra;
                }
                cursor = list_next(e, cursor);
            }
            if chosen == 0 {
                let new_list = new_extra_list(e);
                e.call(EXTRA_SET_HOT_KEY, &args![new_list, hot_key as u8 as u32]);
                let number = e.get(item, ItemChange::iNumber);
                let counted = item_change_get_extra_total_count(e, Ptr::new(entry), false);
                let worn = item_change_get_worn(e, Ptr::new(entry), 0);
                let remaining = number.wrapping_sub(counted).wrapping_sub(worn as i32);
                e.call(EXTRA_SET_COUNT, &args![new_list, remaining]);
                let list = e.mem.u32(entry);
                list_call_with_item(e, LIST_ADD, list, new_list);
            } else {
                e.call(EXTRA_SET_HOT_KEY, &args![chosen, hot_key as u8 as u32]);
            }
        } else {
            let number = e.get(item, ItemChange::iNumber);
            let change = new_item_change(e, form, number as u32);
            let new_list = new_extra_list(e);
            // The constructor made a list node; the game replaces it.
            let node = new_list_node(e);
            e.mem.set_u32(change, node);
            e.call(EXTRA_SET_HOT_KEY, &args![new_list, hot_key as u8 as u32]);
            let change_list = e.mem.u32(change);
            list_call_with_item(e, LIST_ADD, change_list, new_list);
            let changes = e.get(this, InventoryChanges::pListofChanges).addr();
            list_call_with_item(e, LIST_ADD, changes, change);
        }
    });
}

// Translated from 004bfb30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The player's hot key table entry for `index` (0 to 8; 0 outside): reads
/// the entry `006a7ad0` finds in the map at +0xe3c.
pub fn fn_004bfb30(e: &mut Engine, this: u32, index: i32) -> u32 {
    if !(0..=8).contains(&index) {
        return 0;
    }
    let entry = e
        .call(HOT_KEY_TABLE_ENTRY, &args![this + HOT_KEY_TABLE, index])
        .u32();
    e.mem.u32(entry)
}

// Translated from 004bfb70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the player's hot key table entry for `index` to `value`
/// (`0047a110(index, &value)` on the map at +0xe3c).
pub fn fn_004bfb70(e: &mut Engine, this: u32, index: i32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(HOT_KEY_TABLE_SET, &args![this + HOT_KEY_TABLE, index, slot]);
    });
}

// Translated from 004bfba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryChanges::GetObjectInList` (Xbox PDB): the `ItemChange` of the
/// list of changes whose form is `form`. With a non-zero `reference_id` only
/// one that has an extra list whose reference pointer
/// (`GetReferencePointer`) has that form id; 0 when there is none. The
/// second stack word is not read.
pub fn inventory_changes_get_object_in_list(
    e: &mut Engine,
    this: Ptr<InventoryChanges>,
    form: u32,
    _unused_1: u32,
    reference_id: u32,
) -> u32 {
    let mut cursor = e.get(this, InventoryChanges::pListofChanges).addr();
    let mut searching = true;
    while cursor != 0 && searching {
        let item = list_item(e, cursor);
        if item != 0
            && e.get(Ptr::<ItemChange>::new(item), ItemChange::pContainerObj)
                .addr()
                == form
        {
            searching = false;
        } else {
            cursor = list_next(e, cursor);
        }
    }
    if cursor == 0 {
        return 0;
    }
    let item = list_item(e, cursor);
    if item == 0 || reference_id == 0 {
        return item;
    }
    let mut inner = e.mem.u32(item);
    while inner != 0 {
        let extra = list_item(e, inner);
        if extra != 0 && e.call(EXTRA_GET_REFERENCE_POINTER, &args![extra]).u32() != 0 {
            let reference = e.call(EXTRA_GET_REFERENCE_POINTER, &args![extra]).u32();
            if e.call(FORM_ID, &args![reference]).u32() == reference_id {
                return item;
            }
        }
        inner = list_next(e, inner);
    }
    0
}

// Translated from 004bfc80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether an armor form (type 0x18) of the changes has a worn extra list
/// (`GetWorn(0)`) while its `TESObjectARMO` flag bit 0 (`fn_004bfd80`) is
/// set. The walk of the changes stops at the first one that is null.
pub fn fn_004bfc80(e: &mut Engine, this: Ptr<InventoryChanges>) -> bool {
    let mut found = false;
    let mut cursor = e.get(this, InventoryChanges::pListofChanges).addr();
    while cursor != 0 {
        let entry = list_item(e, cursor);
        if entry == 0 {
            break;
        }
        let form = container_object(e, Ptr::new(entry));
        if form_type_of(e, form) == 0x18 && !found {
            let armor = container_object(e, Ptr::new(entry));
            let mut inner = e.mem.u32(entry);
            while inner != 0 {
                let extra = list_item(e, inner);
                if extra == 0 || found {
                    break;
                }
                if e.call(EXTRA_GET_WORN, &args![extra, 0u32]).bool()
                    && fn_004bfd80(e, Ptr::new(armor))
                {
                    found = true;
                }
                inner = list_next(e, inner);
            }
        }
        cursor = list_next(e, cursor);
    }
    found
}

// Translated from 004bfd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `OBJ_ARMO::cFlags` (Xbox PDB) bit 0 of a `TESObjectARMO`, at +0x180.
pub fn fn_004bfd80(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + ARMOR_FLAGS) & 1 != 0
}

// Translated from 004bfda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryChanges::WearingObject` (Xbox PDB): the worn extra list of the
/// entry for `form` (`GetWorn(0)`); with `wear_unwearable` also one that is
/// marked "can not wear", which then gets `SetWorn(1, 0)`. 0 when there is
/// none.
pub fn inventory_changes_wearing_object(
    e: &mut Engine,
    this: Ptr<InventoryChanges>,
    form: u32,
    wear_unwearable: u8,
) -> u32 {
    let entry = inventory_changes_get_object_in_list(e, this, form, 1, 0);
    if entry == 0 {
        return 0;
    }
    let mut cursor = e.mem.u32(entry);
    while cursor != 0 {
        let extra = list_item(e, cursor);
        if extra == 0 {
            break;
        }
        if e.call(EXTRA_GET_WORN, &args![extra, 0u32]).bool() {
            return extra;
        }
        if e.call(EXTRA_GET_CAN_NOT_WEAR, &args![extra]).bool() && wear_unwearable != 0 {
            e.call(EXTRA_SET_WORN, &args![extra, 1u32, 0u32]);
            return extra;
        }
        cursor = list_next(e, cursor);
    }
    0
}

// Translated from 004bfe50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryChanges::RemoveAllObjectsWorn` (Xbox PDB): for every change that
/// is worn (`ItemChange::GetWorn(0)`) takes the item out with `004c0cf0`,
/// passing the count and extra list of its first worn extra list (1 and 0
/// without one); with a `reference` (the actor) the form is first passed to
/// `008248e0` on its sub-object at +0x94. When `004c0cf0` reports a removal
/// the walk restarts at the head. The first stack word is not read.
pub fn inventory_changes_remove_all_objects_worn(
    e: &mut Engine,
    this: Ptr<InventoryChanges>,
    _unused_1: u32,
    reference: u32,
) {
    e.with_stack(4, |e, removed| {
        let mut cursor = e.get(this, InventoryChanges::pListofChanges).addr();
        while cursor != 0 {
            let entry = list_item(e, cursor);
            if entry == 0 {
                cursor = list_next(e, cursor);
                continue;
            }
            e.mem.set_u8(removed.addr(), 0);
            if !item_change_get_worn(e, Ptr::new(entry), 0) {
                cursor = list_next(e, cursor);
                continue;
            }
            let mut count = 1i32;
            let mut worn_extra = 0u32;
            let mut inner = e.mem.u32(entry);
            while inner != 0 {
                let extra = list_item(e, inner);
                if extra == 0 {
                    break;
                }
                if e.call(EXTRA_GET_WORN, &args![extra, 0u32]).bool() {
                    count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16 as i32;
                    worn_extra = list_item(e, inner);
                    break;
                }
                inner = list_next(e, inner);
            }
            let form = container_object(e, Ptr::new(entry));
            if reference != 0 {
                e.call(REFERENCE_FORM_NOTIFY, &args![reference + 0x94, form, 1u32]);
            }
            e.call(
                INVENTORY_REMOVE_ITEM,
                &args![this, removed, form, count, reference, worn_extra, 0u32, 1u32, entry, 0u32],
            );
            if e.mem.u8(removed.addr()) != 0 {
                cursor = e.get(this, InventoryChanges::pListofChanges).addr();
            } else {
                cursor = list_next(e, cursor);
            }
        }
    });
}

// Translated from 004bffb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The container of the owner reference (`TESObjectREFR::HasContainer`, the
/// map's name for `0055d310`, which returns it), 0 without an owner.
pub fn fn_004bffb0(e: &mut Engine, this: Ptr<InventoryChanges>) -> u32 {
    let owner = e.get(this, InventoryChanges::pRef).addr();
    if owner == 0 {
        return 0;
    }
    e.call(REFR_HAS_CONTAINER, &args![owner]).u32()
}

/// Splits one unit off the extra list `source` of an equipped stack:
/// `new ExtraDataList` copied from `source` (`CopyList`) with count 1, the
/// count of `source` reduced by one and its hot key removed. Returns the new
/// list.
fn split_extra_list(e: &mut Engine, source: u32) -> u32 {
    let created = new_extra_list(e);
    e.call(EXTRA_COPY_LIST, &args![created, source]);
    e.call(EXTRA_SET_COUNT, &args![created, 1u32]);
    let count = e.call(EXTRA_GET_COUNT, &args![source]).u16() as i16 as i32;
    e.call(EXTRA_SET_COUNT, &args![source, count - 1]);
    let key = e.call(EXTRA_GET_HOT_KEY, &args![source]).u8() as i8;
    if key >= 0 {
        e.call(EXTRA_REMOVE_HOT_KEY, &args![source]);
    }
    created
}

/// Whether `form` is a weapon (type 0x28) of the kinds `fn_004c0bf0` accepts.
fn is_special_weapon(e: &mut Engine, form: u32) -> bool {
    form_type_of(e, form) == 0x28 && fn_004c0bf0(e, Ptr::new(form))
}

// Translated from 004bffe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Puts `count` of `form` on `actor` as worn (the equip step of the
/// inventory): `extra` is the extra list to wear (0 for any), `worn_flag` is
/// the `SetWorn` argument and `cannot_wear` the `SetCanNotWear` one.
///
/// The cache of the weight is reset, the owner told (virtual 0x48 with 0x20)
/// and the entry of the changes for `form` found (or made, with an empty
/// list, when there is none or the item is not in the changes yet). The
/// number of items not in extra lists is `count in the container + entry
/// number - the extra lists' total`. A list of the entry that is the given
/// one, or the first with a free count when none was given, is the one worn:
/// when its count is above 1 and the form is neither type 0x29 nor a special
/// weapon one unit is split off (`CopyList`) and worn, otherwise the whole
/// list. A list that can not be worn is not used; a new list is made when no
/// list could be used. With an entry already worn nothing is done.
///
/// For an `actor` with a current process the equip is announced to the
/// actor by form type: 0x18 (armor) tells the process (virtual 0x6e4 of its
/// acquire object) for power armor kinds and sets the biped slot
/// (virtual 0x468); 0x1a only the latter; 0x28 (weapon) and 0x29 (ammo)
/// keep an `ItemChange` in the process (virtual 0x148/0x14c, filled with the
/// worn list, or a new one accepted through virtual 0x160/0x168), the weapon
/// also running the entry point 0x2b and updating the weapon model. At the end
/// the actor's lighting is refreshed. The C++ exception frame is not
/// translated.
#[allow(clippy::too_many_arguments)]
pub fn fn_004bffe0(
    e: &mut Engine,
    this: Ptr<InventoryChanges>,
    form: u32,
    count: i32,
    actor: u32,
    mut extra: u32,
    worn_flag: u8,
    cannot_wear: u8,
) {
    with_scope_guard(e, 0x960, |e| {
        fn_004bf0e0(e, this);
        let owner = e.get(this, InventoryChanges::pRef).addr();
        if owner != 0 {
            e.vcall(owner, 0x48, &args![0x20u32]);
            let kind = form_type_of(e, form);
            if kind == 0x1a || kind == 0x18 {
                e.call(GET_FORM_AS_BIPED_MODEL, &args![form]);
            }
        }
        let mut entry = inventory_changes_get_object_in_list(e, this, form, 1, 0);
        let container = fn_004bffb0(e, this);
        let in_container = e.call(CONTAINER_COUNT, &args![container, form]).i32();
        let mut total = in_container;
        if entry != 0 {
            total += e.get(Ptr::<ItemChange>::new(entry), ItemChange::iNumber);
        }
        let mut in_lists = 0i32;
        if entry != 0 && e.mem.u32(entry) != 0 {
            let counted = item_change_get_extra_total_count(e, Ptr::new(entry), false);
            in_lists = counted + item_change_get_extra_total_default_count(e, Ptr::new(entry));
        }
        let mut free = total - in_lists;
        if entry != 0 && e.mem.u32(entry) != 0 {
            let head = e.mem.u32(entry);
            if list_item(e, head) != 0 {
                let first = list_item(e, head);
                if e.call(EXTRA_HAS_LEVELED_ITEM, &args![first]).bool() {
                    free = in_container - in_lists;
                }
            }
            let mut matched = false;
            if extra != 0 {
                let head = e.mem.u32(entry);
                let held = with_item_slot(e, extra, |e, slot| {
                    e.call(LIST_CONTAINS, &args![head, slot]).bool()
                });
                if !held {
                    let mut cursor = e.mem.u32(entry);
                    while cursor != 0 {
                        let candidate = list_item(e, cursor);
                        if candidate == 0 {
                            break;
                        }
                        if !e.call(EXTRA_COMPARE_LIST, &args![candidate, extra]).bool() {
                            extra = candidate;
                            matched = true;
                            break;
                        }
                        cursor = list_next(e, cursor);
                    }
                    if !matched {
                        extra = 0;
                    }
                }
            }
        }
        let mut needs_new = true;
        let mut used = 0u32;
        if (extra == 0 && free > 0) || entry == 0 {
            if entry == 0 {
                entry = new_item_change(e, form, 0);
                let list = e.get(this, InventoryChanges::pListofChanges).addr();
                list_call_with_item(e, LIST_ADD_TAIL, list, entry);
                extra = 0;
            }
        } else {
            if item_change_get_worn(e, Ptr::new(entry), 0) {
                return;
            }
            let mut cursor = e.mem.u32(entry);
            while cursor != 0 && needs_new {
                used = list_item(e, cursor);
                if extra != 0 && extra == used {
                    let special = is_special_weapon(e, form);
                    let count_here = e.call(EXTRA_GET_COUNT, &args![used]).u16() as i16;
                    if count_here > 1 && form_type_of(e, form) != 0x29 && !special {
                        extra = split_extra_list(e, used);
                    } else {
                        e.call(EXTRA_SET_WORN, &args![used, 1u32, worn_flag as u32]);
                        let number = e.get(Ptr::<ItemChange>::new(entry), ItemChange::iNumber);
                        e.call(EXTRA_SET_COUNT, &args![used, number]);
                    }
                    needs_new = false;
                } else if used != 0 && extra == 0 {
                    if e.call(EXTRA_GET_COUNT, &args![used]).u16() as i16 == 1 {
                        extra = used;
                    } else {
                        let special = is_special_weapon(e, form);
                        let count_here = e.call(EXTRA_GET_COUNT, &args![used]).u16() as i16;
                        if count_here > 1 && form_type_of(e, form) != 0x29 && !special {
                            extra = split_extra_list(e, used);
                        } else {
                            extra = used;
                        }
                    }
                    needs_new = false;
                } else {
                    cursor = list_next(e, cursor);
                }
            }
        }
        if extra != 0 && !e.call(EXTRA_GET_CAN_NOT_WEAR, &args![extra]).bool() {
            e.call(EXTRA_SET_WORN, &args![extra, 1u32, worn_flag as u32]);
            e.call(EXTRA_SET_COUNT, &args![extra, count as u16 as u32]);
            needs_new = false;
            ensure_extra_list(e, entry);
            let head = e.mem.u32(entry);
            let held = with_item_slot(e, extra, |e, slot| {
                e.call(LIST_CONTAINS, &args![head, slot]).bool()
            });
            if !held {
                list_call_with_item(e, LIST_ADD, head, extra);
            }
            used = extra;
        }
        if needs_new {
            used = new_extra_list(e);
            e.call(EXTRA_SET_WORN, &args![used, 1u32, worn_flag as u32]);
            if count > 1 {
                e.call(EXTRA_SET_COUNT, &args![used, count as u16 as u32]);
            }
            ensure_extra_list(e, entry);
            let head = e.mem.u32(entry);
            let held = with_item_slot(e, used, |e, slot| {
                e.call(LIST_CONTAINS, &args![head, slot]).bool()
            });
            if !held {
                list_call_with_item(e, LIST_ADD, head, used);
            }
        }
        if used != 0 {
            e.call(EXTRA_SET_CAN_NOT_WEAR, &args![used, cannot_wear as u32]);
        }
        if e.mem.u32(actor + ACTOR_CURRENT_PROCESS) != 0 {
            announce_equip(e, form, count, actor, used);
        }
    });
}

/// Calls `body` with the address of a 4-byte slot holding `item` (the
/// `&item` the list methods take).
fn with_item_slot<R>(e: &mut Engine, item: u32, body: impl FnOnce(&mut Engine, u32) -> R) -> R {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        body(e, slot.addr())
    })
}

/// The last part of `004bffe0`: tells the actor's process about the worn
/// extra list `worn_list` of `form`, by form type.
fn announce_equip(e: &mut Engine, form: u32, count: i32, actor: u32, worn_list: u32) {
    e.call(GET_FORM_AS_BIPED_MODEL, &args![form]);
    let kind = form_type_of(e, form);
    match kind {
        0x18 => {
            let biped = form + ARMOR_BIPED_MODEL;
            let power_armor = e.call(BIPED_IS_POWER_ARMOR, &args![biped]).bool()
                || fn_004c0bd0(e, Ptr::new(biped))
                || e.call(BIPED_FLAG_FOUR, &args![biped]).bool();
            if power_armor && e.call(ACTOR_PROCESS, &args![actor]).u32() != 0 {
                let acquire = e.call(ACTOR_PROCESS, &args![actor]).u32();
                e.vcall(acquire, 0x6e4, &args![actor]);
            }
            let process = e.mem.u32(actor + ACTOR_CURRENT_PROCESS);
            e.vcall(process, 0x468, &args![1u32]);
        }
        0x1a => {
            let process = e.mem.u32(actor + ACTOR_CURRENT_PROCESS);
            e.vcall(process, 0x468, &args![1u32]);
        }
        0x28 => {
            let acquire = e.call(ACTOR_PROCESS, &args![actor]).u32();
            let mut kept = e.vcall(acquire, 0x148, &args![]).u32();
            if kept != 0 {
                let list = e.mem.u32(kept);
                e.call(LIST_CLEAR, &args![list]);
                let list = e.mem.u32(kept);
                list_call_with_item(e, LIST_ADD, list, worn_list);
                e.mem.set_u32(kept + 8, form);
                e.call(ACTOR_RELOAD_TARGETS, &args![actor, 0u32]);
            } else {
                kept = new_item_change(e, form, count as u32);
                let list = e.mem.u32(kept);
                list_call_with_item(e, LIST_ADD, list, worn_list);
                let acquire = e.call(ACTOR_PROCESS, &args![actor]).u32();
                let hand = e.vcall(actor, 0x1d0, &args![0u32]).u32();
                if !e.vcall(acquire, 0x160, &args![kept, hand]).bool() {
                    delete_item_change(e, kept);
                    kept = 0;
                } else {
                    let animation = e.call(ACTOR_GET_ANIMATION, &args![actor]).u32();
                    let item_form = e.mem.u32(kept + 8);
                    let value = e.call(FORM_WEIGHT_FIELD, &args![item_form]).f32();
                    e.with_stack(4, |e, slot| {
                        e.mem.set_f32(slot.addr(), value);
                        let item_form = e.mem.u32(kept + 8);
                        e.call(HANDLE_ENTRY_POINT, &args![0x2bu32, actor, item_form, slot]);
                        let value = e.mem.f32(slot.addr());
                        if animation != 0 {
                            e.call(ANIMATION_STORE_FLOAT, &args![animation, value]);
                        }
                    });
                }
            }
            let player = e.global::<u32>(PLAYER_GLOBAL);
            let process = e.mem.u32(actor + ACTOR_CURRENT_PROCESS);
            let announced = actor != player && e.vcall(process, 0x474, &args![1u32]).bool();
            if !announced && kept != 0 {
                let item_form = e.mem.u32(kept + 8);
                if actor == player {
                    e.call(PLAYER_STORE_FORM, &args![player, item_form]);
                } else if e.call(PREDICATE_008C7AA0, &args![]).bool()
                    && e.call(THREAD_VALUE_0047B200, &args![]).u32() != actor
                {
                    let queue = e.call(TASK_QUEUE_GETTER, &args![]).u32();
                    e.call(TASK_QUEUE_ATTACH_WEAPON, &args![queue, actor, item_form]);
                } else {
                    e.call(REFR_UPDATE_WEAPON, &args![actor, item_form]);
                }
            }
        }
        0x29 => {
            let acquire = e.call(ACTOR_PROCESS, &args![actor]).u32();
            let mut kept = e.vcall(acquire, 0x14c, &args![]).u32();
            if kept != 0 {
                let list = e.mem.u32(kept);
                e.call(LIST_CLEAR, &args![list]);
                let list = e.mem.u32(kept);
                list_call_with_item(e, LIST_ADD, list, worn_list);
                e.mem.set_u32(kept + 8, form);
            } else {
                kept = new_item_change(e, form, 0);
                let list = e.mem.u32(kept);
                list_call_with_item(e, LIST_ADD, list, worn_list);
                let acquire = e.call(ACTOR_PROCESS, &args![actor]).u32();
                if !e.vcall(acquire, 0x168, &args![kept]).bool() {
                    delete_item_change(e, kept);
                    kept = 0;
                }
            }
            let process = e.mem.u32(actor + ACTOR_CURRENT_PROCESS);
            if !e.vcall(process, 0x474, &args![1u32]).bool() && kept != 0 {
                let item_form = e.mem.u32(kept + 8);
                e.call(STORE_NOTIFY_004534F0, &args![actor, item_form]);
            }
        }
        _ => {}
    }
    let lighting = e.call(ACTOR_LIGHTING_ARGUMENT, &args![actor]).u32();
    e.call(ACTOR_INIT_LIGHTING, &args![actor, lighting]);
}

// Translated from 004c0bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 7 of the flag byte (`BIPED_MODEL::sFlags`, +8 of a
/// `TESBipedModelForm`).
pub fn fn_004c0bd0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 8) & 0x80 != 0
}

// Translated from 004c0bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A weapon whose type is in the `004c0c30` range but not in the `004c0c60`
/// one.
pub fn fn_004c0bf0(e: &mut Engine, this: Ptr) -> bool {
    fn_004c0c30(e, this) && !fn_004c0c60(e, this)
}

// Translated from 004c0c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The weapon type (`OBJ_WEAP::eType`, a signed byte at +0xf4 of a
/// `TESObjectWEAP`) is 3 to 13.
pub fn fn_004c0c30(e: &mut Engine, this: Ptr) -> bool {
    (3..=13).contains(&e.mem.i8(this.addr() + WEAPON_TYPE))
}

// Translated from 004c0c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The weapon type (`OBJ_WEAP::eType`, a signed byte at +0xf4 of a
/// `TESObjectWEAP`) is 3 to 9.
pub fn fn_004c0c60(e: &mut Engine, this: Ptr) -> bool {
    (3..=9).contains(&e.mem.i8(this.addr() + WEAPON_TYPE))
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
        entry!(0x004be060, fn_004be060(Ptr) -> f32),
        entry!(0x004be080, fn_004be080(Ptr) -> f32),
        entry!(0x004be0b0, fn_004be0b0(Ptr<ItemChange>, u32) -> f32),
        entry!(0x004be180, fn_004be180(Ptr) -> f32),
        entry!(0x004be1a0, fn_004be1a0(Ptr<ItemChange>) -> u32),
        entry!(
            0x004be200,
            item_change_get_icon_one(Ptr<ItemChange>, u32) -> u32
        ),
        entry!(
            0x004be2d0,
            item_change_get_full_name(Ptr<ItemChange>) -> u32
        ),
        entry!(
            0x004be330,
            tes_enchantable_form_get_form_enchanting(u32) -> u32
        ),
        entry!(0x004be380, tes_object_weap_get_form_weight(Ptr, u8) -> f32),
        entry!(
            0x004be3d0,
            item_change_set_worn(Ptr<ItemChange>, u8, u8, u8)
        ),
        entry!(0x004be580, fn_004be580(Ptr) -> u8),
        entry!(0x004be5a0, fn_004be5a0(Ptr<ItemChange>) -> u16),
        entry!(0x004be6f0, fn_004be6f0(Ptr<ItemChange>)),
        entry!(0x004be930, item_change_load_game(Ptr<ItemChange>)),
        entry!(0x004bed00, fn_004bed00(Ptr<ItemChange>)),
        entry!(0x004bed60, item_change_save_game(Ptr<ItemChange>, u32)),
        entry!(0x004bee00, item_change_load_game_ov2(Ptr<ItemChange>, u32)),
        entry!(0x004bef60, fn_004bef60(Ptr<ItemChange>, u32)),
        entry!(
            0x004befb0,
            inventory_changes_inventory_changes(
                Ptr<InventoryChanges>,
                Ptr,
            ) -> Ptr<InventoryChanges>
        ),
        entry!(0x004bf0e0, fn_004bf0e0(Ptr<InventoryChanges>)),
        entry!(0x004bf150, fn_004bf150(Ptr<InventoryChanges>)),
        entry!(
            0x004bf220,
            inventory_changes_get_inventory_changes(u32) -> u32
        ),
        entry!(
            0x004bf330,
            inventory_changes_remove_hot_key_item(Ptr<InventoryChanges>, Ptr<ItemChange>, i32)
        ),
        entry!(0x004bf4b0, fn_004bf4b0(Ptr<InventoryChanges>, i32) -> u32),
        entry!(
            0x004bf630,
            oei_fast_inventory_iterator_scalar_deleting_destructor(u32, u32) -> u32
        ),
        entry!(
            0x004bf660,
            fn_004bf660(Ptr<InventoryChanges>, i32, i32) -> u32
        ),
        entry!(
            0x004bf800,
            inventory_changes_set_hot_key_item(Ptr<InventoryChanges>, Ptr<ItemChange>, u32, i32)
        ),
        entry!(0x004bfb30, fn_004bfb30(u32, i32) -> u32),
        entry!(0x004bfb70, fn_004bfb70(u32, i32, u32)),
        entry!(
            0x004bfba0,
            inventory_changes_get_object_in_list(Ptr<InventoryChanges>, u32, u32, u32) -> u32
        ),
        entry!(0x004bfc80, fn_004bfc80(Ptr<InventoryChanges>) -> bool),
        entry!(0x004bfd80, fn_004bfd80(Ptr) -> bool),
        entry!(
            0x004bfda0,
            inventory_changes_wearing_object(Ptr<InventoryChanges>, u32, u8) -> u32
        ),
        entry!(
            0x004bfe50,
            inventory_changes_remove_all_objects_worn(Ptr<InventoryChanges>, u32, u32)
        ),
        entry!(0x004bffb0, fn_004bffb0(Ptr<InventoryChanges>) -> u32),
        entry!(
            0x004bffe0,
            fn_004bffe0(Ptr<InventoryChanges>, u32, i32, u32, u32, u8, u8)
        ),
        entry!(0x004c0bd0, fn_004c0bd0(Ptr) -> bool),
        entry!(0x004c0bf0, fn_004c0bf0(Ptr) -> bool),
        entry!(0x004c0c30, fn_004c0c30(Ptr) -> bool),
        entry!(0x004c0c60, fn_004c0c60(Ptr) -> bool),
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

    // ---- Tests of the second session (004be060 to 004c0c60) ----

    // More words of the fake extra data list (the base fake has 0x80 bytes).
    const X_CAN_NOT_WEAR: u32 = 0x68;
    const X_REFERENCE: u32 = 0x6c;
    const X_SAVE_SIZE: u32 = 0x70;
    const X_WORN_FLAG: u32 = 0x74;

    /// Virtual table whose slot at byte offset `s` points at `VCALL_BASE + s`
    /// (so `e.vcall(object, s, ..)` calls the double registered there).
    const BIG_VTABLE: u32 = 0x0210_0000;
    const VCALL_BASE: u32 = 0x0210_1000;
    /// The fake save/load game object (+0x14 position, +0x20 "use blocks",
    /// +0x30 a count, +0x80 version, +0x84 form being loaded, +0x88 form being
    /// saved), the stream buffer, the log flag byte, the form lookup cell and
    /// the player object.
    const SAVE_OBJECT: u32 = 0x0210_2000;
    const STREAM: u32 = 0x0211_0000;
    const LOG_FLAG: u32 = 0x0210_2100;
    const LOOKUP_CELL: u32 = 0x0210_2200;
    const PLAYER: u32 = 0x0210_8000;
    const STRINGS: u32 = 0x0210_3000;
    /// Cast results by target: biped form, NPC, full name, bound object.
    const CAST_BIPED: u32 = CAST_TABLE + 0x20;
    const CAST_NPC: u32 = CAST_TABLE + 0x24;
    const CAST_FULL_NAME: u32 = CAST_TABLE + 0x28;
    const CAST_BOUND: u32 = CAST_TABLE + 0x2c;

    fn vcall_address(slot: u32) -> u32 {
        VCALL_BASE + slot
    }

    /// A fake object whose vtable is `BIG_VTABLE` (0x400 bytes).
    fn object(e: &mut Engine) -> u32 {
        let o = e.mem.alloc(0x400);
        e.mem.set_u32(o, BIG_VTABLE);
        o
    }

    fn put_str(e: &mut Engine, address: u32, text: &[u8]) {
        e.mem.set_cstr(address, text);
    }

    /// The engine with the doubles of the second session.
    fn more() -> Engine {
        let mut e = logged();
        e.map(0x011c_0000, 0x2_0000);
        e.map(0x0210_0000, 0x4_0000);
        let slots: Vec<u32> = (0..0x200).map(|i| VCALL_BASE + 4 * i).collect();
        e.put_vtable(BIG_VTABLE, &slots);
        e.set_global(SAVE_LOAD_GAME_GLOBAL, SAVE_OBJECT);
        e.set_global(PLAYER_GLOBAL, PLAYER);
        for address in [
            SCOPE_GUARD_OPEN,
            SCOPE_GUARD_CLOSE,
            ERROR_LOG,
            SAVE_LOAD_LOG,
            ITEM_CHANGE_DELETE,
            OPERATOR_DELETE,
            FAST_ITERATOR_DESTRUCT,
            EXTRA_AFTER_LOAD_GAME,
            EXTRA_AFTER_LOAD_GAME_BUFFER,
            EXTRA_SAVE_GAME_BUFFER,
            EXTRA_LOAD_GAME_BUFFER,
            SAVE_BUFFER_SAVE_FORM_ID,
            SAVE_BUFFER_SAVE_DATA,
            SAVE_BUFFER_END_SIZED,
            FORM_SET_TEMPORARY,
            REFERENCE_FORM_NOTIFY,
            ACTOR_RELOAD_TARGETS,
            ACTOR_INIT_LIGHTING,
            ANIMATION_STORE_FLOAT,
            PLAYER_STORE_FORM,
            REFR_UPDATE_WEAPON,
            TASK_QUEUE_ATTACH_WEAPON,
            STORE_NOTIFY_004534F0,
            HANDLE_ENTRY_POINT,
        ] {
            stub(&mut e, address);
        }
        // The fake extra data lists are 0x80 bytes: allocate at least that.
        e.register(OPERATOR_NEW, |e, a| returns(e.mem.alloc(a[0].max(0x80))));
        e.register(LIST_CONTAINS, |e, a| {
            let mut node = a[0];
            let wanted = e.mem.u32(a[1]);
            while node != 0 && e.mem.u32(node) != wanted {
                node = e.mem.u32(node + 4);
            }
            returns((node != 0) as u32)
        });
        e.register(LIST_IS_EMPTY, |e, a| {
            returns((e.mem.u32(a[0] + 4) == 0 && e.mem.u32(a[0]) == 0) as u32)
        });
        e.register(EXTRA_COMPARE_LIST, |_, a| returns((a[0] != a[1]) as u32));
        e.register(EXTRA_COPY_LIST, |e, a| {
            e.mem.set_u32(a[0] + 0x14, a[1]);
            let count = e.mem.u32(a[1] + X_COUNT);
            e.mem.set_u32(a[0] + X_COUNT, count);
            Ret::default()
        });
        e.register(EXTRA_SET_COUNT, |e, a| {
            e.mem.set_u32(a[0] + X_COUNT, a[1] & 0xffff);
            Ret::default()
        });
        e.register(EXTRA_SET_WORN, |e, a| {
            e.mem.set_u32(a[0] + X_WORN, a[1]);
            e.mem.set_u32(a[0] + X_WORN_FLAG, a[2]);
            Ret::default()
        });
        e.register(EXTRA_SET_CAN_NOT_WEAR, |e, a| {
            e.mem.set_u32(a[0] + X_CAN_NOT_WEAR, a[1]);
            Ret::default()
        });
        read_word!(e, EXTRA_GET_CAN_NOT_WEAR, X_CAN_NOT_WEAR);
        e.register(EXTRA_REMOVE_HOT_KEY, |e, a| {
            e.mem.set_u32(a[0] + X_HOT_KEY, 0xff);
            Ret::default()
        });
        e.register(EXTRA_SET_HOT_KEY, |e, a| {
            e.mem.set_u32(a[0] + X_HOT_KEY, a[1]);
            Ret::default()
        });
        read_word!(e, EXTRA_GET_REFERENCE_POINTER, X_REFERENCE);
        read_word!(e, EXTRA_GET_SAVE_SIZE, X_SAVE_SIZE);
        e.register(FORM_ID, |e, a| returns(e.mem.u32(a[0] + 0xc)));
        e.register(LOOKUP_FORM_BY_ID, |e, _| returns(e.mem.u32(LOOKUP_CELL)));
        e.register(REFR_EXTRA_LIST, |_, a| returns(a[0] + 0x40));
        e.register(EXTRA_GET_CONTAINER_CHANGES, |e, a| {
            returns(e.mem.u32(a[0] + 0x1c))
        });
        e.register(EXTRA_SET_CONTAINER_CHANGES, |e, a| {
            e.mem.set_u32(a[0] + 0x1c, a[1]);
            Ret::default()
        });
        // The save/load game object.
        e.register(USE_SAVE_GAME_BLOCKS, |e, a| returns(e.mem.u32(a[0] + 0x20)));
        e.register(SAVE_POSITION, |e, a| returns(e.mem.u32(a[0] + 0x14)));
        e.register(CURRENT_SAVE_FORM, |e, a| returns(e.mem.u32(a[0] + 0x88)));
        e.register(CURRENT_LOAD_FORM, |e, a| returns(e.mem.u32(a[0] + 0x84)));
        e.register(LOAD_VERSION, |e, a| returns(e.mem.u8(a[0] + 0x80) as u32));
        e.register(FLAG_BYTE_ADDRESS, |_, _| returns(LOG_FLAG));
        for address in [SAVE_BYTES, SAVE_NUMERIC_ID] {
            e.register(address, |e, a| {
                let position = e.mem.u32(a[0] + 0x14);
                let bytes = e.mem.bytes(a[1], a[2]);
                e.mem.write(position, &bytes);
                e.mem.set_u32(a[0] + 0x14, position + a[2]);
                Ret::default()
            });
        }
        for address in [LOAD_BYTES, LOAD_NUMERIC_ID] {
            e.register(address, |e, a| {
                let position = e.mem.u32(a[0] + 0x14);
                let bytes = e.mem.bytes(position, a[2]);
                e.mem.write(a[1], &bytes);
                e.mem.set_u32(a[0] + 0x14, position + a[2]);
                Ret::default()
            });
        }
        e.register(EXTRA_SAVE_GAME, |e, a| {
            let position = e.mem.u32(SAVE_OBJECT + 0x14);
            e.mem.write(position, &[0xee, 0xee, 0xee]);
            e.mem.set_u32(SAVE_OBJECT + 0x14, position + 3);
            assert_eq!((a[1], a[2]), (0, 0));
            Ret::default()
        });
        e.register(EXTRA_LOAD_GAME, |e, a| {
            let position = e.mem.u32(SAVE_OBJECT + 0x14);
            e.mem.set_u32(a[0] + 0x14, position);
            e.mem.set_u32(SAVE_OBJECT + 0x14, position + 3);
            Ret::default()
        });
        // Buffers.
        e.register(SAVE_BUFFER_START_SIZED, |_, _| returns(0x77));
        e.register(LOAD_BUFFER_LOAD_FORM_ID, |_, _| returns(0x1234));
        e.register(LOAD_BUFFER_LOAD_SIZED, |e, _| {
            returns(e.mem.u32(SAVE_OBJECT + 0x30))
        });
        e.register(LOAD_BUFFER_LOAD_DATA, |e, a| {
            e.mem.set_u32(a[1], 7);
            Ret::default()
        });
        // Hot key table of the player.
        e.register(HOT_KEY_TABLE_ENTRY, |_, a| returns(a[0] + 0x100 + 4 * a[1]));
        e.register(HOT_KEY_TABLE_SET, |e, a| {
            let value = e.mem.u32(a[2]);
            e.mem.set_u32(a[0] + 0x100 + 4 * a[1], value);
            Ret::default()
        });
        e.register(MEMSET, |e, a| {
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            Ret::default()
        });
        // Casts by (source, target) type.
        e.register(RT_DYNAMIC_CAST, |e, a| {
            returns(match (a[2], a[3]) {
                (_, TYPE_TES_OBJECT_WEAP) => e.mem.u32(CAST_TABLE),
                (_, TYPE_TES_ENCHANTABLE_FORM) => e.mem.u32(CAST_TABLE + 4),
                (_, TYPE_TES_HEALTH_FORM) => e.mem.u32(CAST_TABLE + 8),
                (_, TYPE_TES_BIPED_MODEL_FORM) => e.mem.u32(CAST_BIPED),
                (_, TYPE_TES_NPC) => e.mem.u32(CAST_NPC),
                (_, TYPE_TES_FULL_NAME) => e.mem.u32(CAST_FULL_NAME),
                (TYPE_TES_FORM, TYPE_TES_BOUND_OBJECT) => e.mem.u32(CAST_BOUND),
                other => panic!("unexpected cast {other:08x?}"),
            })
        });
        e
    }

    #[test]
    fn armor_rating_float_is_the_rating_over_one_hundred() {
        let mut e = more();
        // The base fake engine stands in for `004be060`: use the translation.
        let (_, real) = funcs().into_iter().find(|(a, _)| *a == 0x004be060).unwrap();
        e.register(0x004be060, real);
        let armor = form(&mut e, 0x18, 0, 0);
        e.mem.set_u16(armor + ARMOR_RATING, 250);
        assert_eq!(e.call(0x004be080, &args![armor]).f32(), 2.5);
        // 004be060 forwards to it.
        assert_eq!(e.call(0x004be060, &args![armor]).f32(), 2.5);
    }

    #[test]
    fn damage_threshold_goes_through_the_armor_rating_formula() {
        let mut e = more();
        let armor = form(&mut e, 0x18, 0, 200);
        e.mem.set_f32(armor + ARMOR_DAMAGE_THRESHOLD, 12.75);
        let x = extra(&mut e);
        e.mem.set_f32(x + X_HEALTH, 100.0);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, armor);
        // threshold 12 (truncated) * ratio 0.5 = 6, doubled by the rounding stub.
        assert_eq!(e.call(0x004be0b0, &args![this, 0u32]).f32(), 12.0);
        assert_eq!(
            calls(&e, CALC_ARMOR_RATING),
            vec![vec![12, 0.5f32.to_bits()]]
        );
        // No form health: the ratio is zero and the item health not read.
        set(&mut e, armor, F_HEALTH, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004be0b0, &args![this, 0u32]).f32(), 0.0);
        assert!(calls(&e, EXTRA_GET_HEALTH).is_empty());
        // Another form type: -1.0 through the rounding.
        let weapon = form(&mut e, 0x28, 0, 200);
        let other = change(&mut e, head, weapon);
        assert_eq!(e.call(0x004be0b0, &args![other, 0u32]).f32(), -2.0);
    }

    #[test]
    fn damage_threshold_reads_the_float_at_17c() {
        let mut e = more();
        let armor = form(&mut e, 0x18, 0, 0);
        e.mem.set_f32(armor + 0x17c, 3.25);
        assert_eq!(e.call(0x004be180, &args![armor]).f32(), 3.25);
    }

    #[test]
    fn item_class_number_depends_on_the_form_type() {
        let mut e = more();
        let head = list(&mut e, &[]);
        for (form_type, expected) in [(0x18, 4), (0x1a, 4), (0x28, 3), (0x29, 5), (0x20, 5)] {
            let f = form(&mut e, form_type, 0, 0);
            let this = change(&mut e, head, f);
            assert_eq!(e.call(0x004be1a0, &args![this]).u32(), expected);
        }
    }

    #[test]
    fn icon_one_prefers_the_owner_sex_then_the_form_then_the_default() {
        let mut e = more();
        const BIPED_ICON: u32 = STRINGS + 0x100;
        const FORM_ICON_STRING: u32 = STRINGS + 0x200;
        const DEFAULT_ICON: u32 = STRINGS + 0x300;
        put_str(&mut e, BIPED_ICON, b"male.dds");
        put_str(&mut e, FORM_ICON_STRING, b"form.dds");
        put_str(&mut e, DEFAULT_ICON, b"none.dds");
        e.register(ACTOR_BASE_GET_SEX, |e, a| returns(e.mem.u32(a[0] + 0x40)));
        e.register(BIPED_ICON_FOR_SEX, |_, a| {
            returns(STRINGS + 0x100 + a[1] * 0x10)
        });
        e.register(FORM_ICON, |_, _| returns(FORM_ICON_STRING));
        e.register(STRING_OBJECT_TO_CHARS, |_, a| {
            assert_eq!(a[0], NO_ICON_STRING);
            returns(DEFAULT_ICON)
        });
        let armor = form(&mut e, 0x18, 0, 0);
        let npc = e.mem.alloc(0x80);
        set(&mut e, CAST_TABLE, 0x20, armor);
        set(&mut e, CAST_TABLE, 0x24, npc);
        let x = extra(&mut e);
        set(&mut e, x, X_OWNERSHIP, npc);
        let head = list(&mut e, &[x]);
        let this = change(&mut e, head, armor);
        assert_eq!(e.call(0x004be200, &args![this, 3u32]).u32(), BIPED_ICON);
        // The sex picks the string.
        put_str(&mut e, STRINGS + 0x110, b"female.dds");
        set(&mut e, npc, 0x40, 1);
        assert_eq!(
            e.call(0x004be200, &args![this, 3u32]).u32(),
            STRINGS + 0x110
        );
        // An empty string for that sex falls back to the form's icon with the
        // variant.
        put_str(&mut e, STRINGS + 0x110, b"");
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x004be200, &args![this, 3u32]).u32(),
            FORM_ICON_STRING
        );
        assert_eq!(calls(&e, FORM_ICON), vec![vec![armor, 3]]);
        // No owner: straight to the form's icon.
        set(&mut e, x, X_OWNERSHIP, 0);
        assert_eq!(
            e.call(0x004be200, &args![this, 4u32]).u32(),
            FORM_ICON_STRING
        );
        // An empty form icon gives the default string object.
        put_str(&mut e, FORM_ICON_STRING, b"");
        assert_eq!(e.call(0x004be200, &args![this, 4u32]).u32(), DEFAULT_ICON);
    }

    #[test]
    fn full_name_needs_a_full_name_form_the_lookup_accepts() {
        let mut e = more();
        const NAME: u32 = STRINGS + 0x400;
        const NO_NAME: u32 = STRINGS + 0x500;
        e.register(FULL_NAME_TEST, |e, a| returns(e.mem.u32(a[0] + 4)));
        e.register(FULL_NAME_GET, |_, _| returns(NAME));
        e.register(STRING_OBJECT_TO_CHARS, |_, a| {
            assert_eq!(a[0], NO_NAME_STRING);
            returns(NO_NAME)
        });
        let f = form(&mut e, 0x18, 0, 0);
        let head = list(&mut e, &[]);
        let this = change(&mut e, head, f);
        // Not a TESFullName.
        assert_eq!(e.call(0x004be2d0, &args![this]).u32(), NO_NAME);
        let named = e.mem.alloc(0x20);
        set(&mut e, CAST_TABLE, 0x28, named);
        // The lookup rejects it.
        assert_eq!(e.call(0x004be2d0, &args![this]).u32(), NO_NAME);
        set(&mut e, named, 4, 1);
        assert_eq!(e.call(0x004be2d0, &args![this]).u32(), NAME);
    }

    #[test]
    fn form_enchanting_reads_the_word_of_the_enchantable_form() {
        let mut e = more();
        let f = form(&mut e, 0x18, 0, 0);
        assert_eq!(e.call(0x004be330, &args![f]).u32(), 0);
        let enchantable = e.mem.alloc(0x20);
        set(&mut e, enchantable, 4, 0x1234_5678);
        set(&mut e, CAST_TABLE, 4, enchantable);
        assert_eq!(e.call(0x004be330, &args![f]).u32(), 0x1234_5678);
    }

    #[test]
    fn form_weight_subtracts_the_weight_mod_effect() {
        let mut e = more();
        e.register(GET_WEIGHT, |e, a| returns_float(e.mem.f32(a[0])));
        let weapon = form(&mut e, 0x28, 0, 0);
        e.mem.set_f32(weapon + WEAPON_WEIGHT_FORM, 3.0);
        set(&mut e, weapon, WEAPON_MOD_ACTION, 4);
        e.mem.set_f32(weapon + WEAPON_MOD_ACTION_VALUE, 0.5);
        assert_eq!(e.call(0x004be380, &args![weapon, 0u32]).f32(), 3.0);
        assert_eq!(e.call(0x004be380, &args![weapon, 1u32]).f32(), 2.5);
        assert_eq!(calls(&e, GET_WEIGHT)[0], vec![weapon + 0x8c]);
    }

    #[test]
    fn set_worn_adds_a_worn_list_or_removes_and_deletes_the_worn_one() {
        let mut e = more();
        // Wear: a new list marked worn is appended; a missing node is made.
        let this = change(&mut e, 0, 0);
        e.call(0x004be3d0, &args![this, 1u32, 5u32, 0u32]);
        let head = e.get(this, ItemChange::pExtraObjectList).addr();
        let added = items(&e, head);
        assert_eq!(added.len(), 1);
        assert_eq!(e.mem.u32(added[0] + X_WORN), 1);
        assert_eq!(e.mem.u32(added[0] + X_WORN_FLAG), 5);
        // Unwear: the worn list is taken out of the list; deleted on request.
        let (plain, worn) = (extra(&mut e), extra(&mut e));
        set(&mut e, worn, X_WORN, 1);
        let head = list(&mut e, &[plain, worn]);
        let this = change(&mut e, head, 0);
        e.call_log = Some(vec![]);
        e.call(0x004be3d0, &args![this, 0u32, 0u32, 0u32]);
        assert_eq!(items(&e, head), vec![plain]);
        assert!(calls(&e, DESTRUCTOR).is_empty());
        let head = list(&mut e, &[plain, worn]);
        let this = change(&mut e, head, 0);
        e.call(0x004be3d0, &args![this, 0u32, 0u32, 1u32]);
        assert_eq!(items(&e, head), vec![plain]);
        assert_eq!(calls(&e, DESTRUCTOR), vec![vec![worn, 1]]);
        // Nothing worn: nothing removed.
        let head = list(&mut e, &[plain]);
        let this = change(&mut e, head, 0);
        e.call(0x004be3d0, &args![this, 0u32, 0u32, 1u32]);
        assert_eq!(items(&e, head), vec![plain]);
    }

    #[test]
    fn byte_at_240() {
        let mut e = more();
        let o = object(&mut e);
        e.mem.set_u8(o + 0x240, 0x5a);
        assert_eq!(e.call(0x004be580, &args![o]).u8(), 0x5a);
    }

    const NAME_STRING: u32 = STRINGS + 0x600;

    /// Makes the virtual 0x130 (the name of a form) of the fake objects return
    /// `NAME_STRING`.
    fn name_slot(e: &mut Engine) {
        e.register(vcall_address(0x130), |_, _| returns(NAME_STRING));
    }

    /// A record of "the form being saved/loaded": form id at +0, flags (a
    /// word at the unaligned +5) and the version byte at +9.
    fn current_record(e: &mut Engine, at: u32, form_id: u32, flags: u32, version: u8) {
        e.mem.set_u32(at, form_id);
        e.mem.set_u32(at + 5, flags);
        e.mem.set_u8(at + 9, version);
    }

    /// The stream the save/load doubles read and write.
    fn start_stream(e: &mut Engine) {
        e.mem.set_u32(SAVE_OBJECT + 0x14, STREAM);
    }

    fn stream_position(e: &Engine) -> u32 {
        e.mem.u32(SAVE_OBJECT + 0x14)
    }

    #[test]
    fn save_size_adds_the_header_and_every_list() {
        let mut e = more();
        let (a, b) = (extra(&mut e), extra(&mut e));
        set(&mut e, a, X_SAVE_SIZE, 10);
        set(&mut e, b, X_SAVE_SIZE, 20);
        let head = list(&mut e, &[a, b]);
        let this = change(&mut e, head, 0);
        // Save game blocks add the 4 byte tag and the 2 byte length.
        set(&mut e, SAVE_OBJECT, 0x20, 1);
        assert_eq!(e.call(0x004be5a0, &args![this]).u16(), 6 + 12 + 30);
        set(&mut e, SAVE_OBJECT, 0x20, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004be5a0, &args![this]).u16(), 12 + 30);
        assert!(calls(&e, ERROR_LOG).is_empty());
        assert_eq!(
            calls(&e, EXTRA_GET_SAVE_SIZE),
            vec![vec![a, 0, 0], vec![b, 0, 0]]
        );
        // An empty list adds nothing.
        let empty = list(&mut e, &[]);
        let none = change(&mut e, empty, 0);
        assert_eq!(e.call(0x004be5a0, &args![none]).u16(), 12);
    }

    #[test]
    fn save_size_logging_reports_the_size_with_or_without_the_form() {
        let mut e = more();
        name_slot(&mut e);
        let head = list(&mut e, &[]);
        let this = change(&mut e, head, 0);
        e.mem.set_u8(LOG_FLAG, 1);
        e.call(0x004be5a0, &args![this]);
        assert_eq!(
            calls(&e, ERROR_LOG),
            vec![vec![0x0101_2c78, 12, 0x622, SOURCE_FILE_NAME]]
        );
        // With the form being saved: its id, name and flags.
        let saved_form = object(&mut e);
        e.mem.set_u32(LOOKUP_CELL, saved_form);
        current_record(&mut e, 0x0210_2300, 0x14, 0x20, 3);
        set(&mut e, SAVE_OBJECT, 0x88, 0x0210_2300);
        e.call_log = Some(vec![]);
        e.call(0x004be5a0, &args![this]);
        assert_eq!(
            calls(&e, ERROR_LOG),
            vec![vec![
                0x0101_2cb0,
                12,
                0x14,
                NAME_STRING,
                0x20,
                0x622,
                SOURCE_FILE_NAME
            ]]
        );
        assert_eq!(calls(&e, LOOKUP_FORM_BY_ID), vec![vec![0x14]]);
    }

    #[test]
    fn save_writes_the_block_the_ids_and_patches_the_counts() {
        let mut e = more();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let head = list(&mut e, &[a, b]);
        let item_form = form(&mut e, 0x28, 0, 0);
        set(&mut e, item_form, 0xc, 0xabc);
        let this = change(&mut e, head, item_form);
        e.set(this, ItemChange::iNumber, 3);
        start_stream(&mut e);
        set(&mut e, SAVE_OBJECT, 0x20, 1);
        e.call(0x004be6f0, &args![this]);
        // tag, 16 bit length, form id, number, number of lists, the lists.
        assert_eq!(e.mem.u32(STREAM), 0x424c_4f4b);
        assert_eq!(stream_position(&e), STREAM + 24);
        assert_eq!(e.mem.u16(STREAM + 4), 20);
        assert_eq!(e.mem.u32(STREAM + 6), 0xabc);
        assert_eq!(e.mem.u32(STREAM + 10), 3);
        assert_eq!(e.mem.u32(STREAM + 14), 2);
        assert_eq!(e.mem.bytes(STREAM + 18, 6), vec![0xee; 6]);
        assert!(calls(&e, SAVE_LOAD_LOG).is_empty());
        // Without blocks there is no tag and no length.
        set(&mut e, SAVE_OBJECT, 0x20, 0);
        e.mem.set_u32(SAVE_OBJECT + 0x14, STREAM + 0x100);
        e.call(0x004be6f0, &args![this]);
        assert_eq!(e.mem.u32(STREAM + 0x100), 0xabc);
        assert_eq!(e.mem.u32(STREAM + 0x108), 2);
        assert_eq!(stream_position(&e), STREAM + 0x100 + 18);
    }

    #[test]
    fn save_logging_reports_the_bytes_written() {
        let mut e = more();
        name_slot(&mut e);
        let head = list(&mut e, &[]);
        let item_form = form(&mut e, 0x28, 0, 0);
        let this = change(&mut e, head, item_form);
        start_stream(&mut e);
        e.mem.set_u8(LOG_FLAG, 1);
        e.call(0x004be6f0, &args![this]);
        assert_eq!(
            calls(&e, ERROR_LOG),
            vec![vec![0x0101_536c, 12, 0x641, SOURCE_FILE_NAME]]
        );
        let saved_form = object(&mut e);
        e.mem.set_u32(LOOKUP_CELL, saved_form);
        current_record(&mut e, 0x0210_2300, 0x14, 0x20, 3);
        set(&mut e, SAVE_OBJECT, 0x88, 0x0210_2300);
        e.call_log = Some(vec![]);
        e.call(0x004be6f0, &args![this]);
        assert_eq!(
            calls(&e, ERROR_LOG),
            vec![vec![
                0x0101_53a0,
                12,
                0x14,
                NAME_STRING,
                0x20,
                0x641,
                SOURCE_FILE_NAME
            ]]
        );
    }

    /// The stream of a saved item: tag, length, form id, number, number of
    /// lists, then the 3 byte lists.
    fn put_item_stream(e: &mut Engine, tag: u32, length: u16, lists: u32) {
        e.mem.set_u32(STREAM, tag);
        e.mem.set_u16(STREAM + 4, length);
        e.mem.set_u32(STREAM + 6, 0xabc);
        e.mem.set_u32(STREAM + 10, 9);
        e.mem.set_u32(STREAM + 14, lists);
        start_stream(e);
    }

    #[test]
    fn load_reads_the_form_the_number_and_the_lists() {
        let mut e = more();
        let loaded = form(&mut e, 0x28, 0, 0);
        e.mem.set_u32(LOOKUP_CELL, loaded);
        set(&mut e, CAST_TABLE, 0x2c, 0x5555);
        set(&mut e, SAVE_OBJECT, 0x20, 1);
        put_item_stream(&mut e, 0x424c_4f4b, 20, 2);
        let this = e.new_object::<ItemChange>();
        e.call(0x004be930, &args![this]);
        assert_eq!(e.get(this, ItemChange::pContainerObj), Ptr::new(0x5555));
        assert_eq!(e.get(this, ItemChange::iNumber), 9);
        let lists = items(&e, e.get(this, ItemChange::pExtraObjectList).addr());
        assert_eq!(lists.len(), 2);
        // Each list was loaded from its own 3 bytes of the stream.
        assert_eq!(e.mem.u32(lists[0] + 0x14), STREAM + 18);
        assert_eq!(e.mem.u32(lists[1] + 0x14), STREAM + 21);
        assert_eq!(calls(&e, EXTRA_LOAD_GAME).len(), 2);
        assert_eq!(calls(&e, LOOKUP_FORM_BY_ID), vec![vec![0xabc]]);
        // The length matches the bytes read: nothing is logged.
        assert!(calls(&e, SAVE_LOAD_LOG).is_empty());
        // No lists, no blocks: the list pointer stays null.
        set(&mut e, SAVE_OBJECT, 0x20, 0);
        e.mem.set_u32(STREAM, 0xabc);
        e.mem.set_u32(STREAM + 4, 9);
        e.mem.set_u32(STREAM + 8, 0);
        start_stream(&mut e);
        let empty = e.new_object::<ItemChange>();
        e.set(empty, ItemChange::pExtraObjectList, Ptr::new(0x1234));
        e.call(0x004be930, &args![empty]);
        assert_eq!(e.get(empty, ItemChange::pExtraObjectList), Ptr::NULL);
        assert_eq!(e.get(empty, ItemChange::iNumber), 9);
    }

    #[test]
    fn load_logs_a_wrong_tag_and_a_wrong_length() {
        let mut e = more();
        name_slot(&mut e);
        let loaded = object(&mut e);
        e.mem.set_u32(LOOKUP_CELL, loaded);
        set(&mut e, SAVE_OBJECT, 0x20, 1);
        e.mem.set_u8(SAVE_OBJECT + 0x80, 0x2a);
        // Wrong tag, no form being loaded: the version is logged.
        put_item_stream(&mut e, 0x1234_5678, 20, 2);
        let this = e.new_object::<ItemChange>();
        e.call(0x004be930, &args![this]);
        assert_eq!(
            calls(&e, SAVE_LOAD_LOG),
            vec![vec![0x0101_56a8, SOURCE_FILE_NAME, 0x647, 0x2a]]
        );
        // Length too short: overrun of 2 bytes; too long: underrun.
        e.call_log = Some(vec![]);
        put_item_stream(&mut e, 0x424c_4f4b, 18, 2);
        e.call(0x004be930, &args![this]);
        put_item_stream(&mut e, 0x424c_4f4b, 22, 2);
        e.call(0x004be930, &args![this]);
        assert_eq!(
            calls(&e, SAVE_LOAD_LOG),
            vec![
                vec![0x0101_54a0, 2, SOURCE_FILE_NAME, 0x66b, 0x2a],
                vec![0x0101_5440, 2, SOURCE_FILE_NAME, 0x66b, 0x2a],
            ]
        );
        // With a form being loaded the messages carry its id, name, version
        // and flags.
        current_record(&mut e, 0x0210_2300, 0x14, 0x20, 7);
        set(&mut e, SAVE_OBJECT, 0x84, 0x0210_2300);
        e.call_log = Some(vec![]);
        put_item_stream(&mut e, 0x1234_5678, 18, 2);
        e.call(0x004be930, &args![this]);
        let logged_calls = calls(&e, SAVE_LOAD_LOG);
        assert_eq!(
            logged_calls,
            vec![
                vec![
                    0x0101_5718,
                    SOURCE_FILE_NAME,
                    0x647,
                    0x14,
                    NAME_STRING,
                    7,
                    0x20
                ],
                vec![
                    0x0101_5588,
                    2,
                    SOURCE_FILE_NAME,
                    0x66b,
                    0x14,
                    NAME_STRING,
                    7,
                    0x20
                ],
            ]
        );
        e.call_log = Some(vec![]);
        put_item_stream(&mut e, 0x424c_4f4b, 22, 2);
        e.call(0x004be930, &args![this]);
        assert_eq!(
            calls(&e, SAVE_LOAD_LOG),
            vec![vec![
                0x0101_5500,
                2,
                SOURCE_FILE_NAME,
                0x66b,
                0x14,
                NAME_STRING,
                7,
                0x20
            ]]
        );
    }

    #[test]
    fn after_load_runs_on_every_list_until_an_empty_head() {
        let mut e = more();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let head = list(&mut e, &[a, 0, b]);
        let this = change(&mut e, head, 0x4242);
        e.call(0x004bed00, &args![this]);
        assert_eq!(
            calls(&e, EXTRA_AFTER_LOAD_GAME),
            vec![vec![a, 0, 0, 0, 0x4242], vec![b, 0, 0, 0, 0x4242]]
        );
        // An empty head node stops the walk at once.
        let empty = list(&mut e, &[]);
        let none = change(&mut e, empty, 0x4242);
        e.call_log = Some(vec![]);
        e.call(0x004bed00, &args![none]);
        assert!(calls(&e, EXTRA_AFTER_LOAD_GAME).is_empty());
    }

    #[test]
    fn save_game_buffer_writes_the_form_the_number_and_the_lists() {
        let mut e = more();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let head = list(&mut e, &[a, 0, b]);
        let this = change(&mut e, head, 0x4242);
        e.call(0x004bed60, &args![this, 0xb0ffu32]);
        assert_eq!(
            calls(&e, SAVE_BUFFER_SAVE_FORM_ID),
            vec![vec![0xb0ff, 0x4242, 0]]
        );
        assert_eq!(
            calls(&e, SAVE_BUFFER_SAVE_DATA),
            vec![vec![0xb0ff, this.addr() + 4, 4, 0]]
        );
        assert_eq!(
            calls(&e, EXTRA_SAVE_GAME_BUFFER),
            vec![vec![a, 0xb0ff], vec![b, 0xb0ff]]
        );
        // Two lists were written; the start value comes from the buffer.
        assert_eq!(
            calls(&e, SAVE_BUFFER_END_SIZED),
            vec![vec![0xb0ff, 2, 0x77]]
        );
    }

    #[test]
    fn load_game_buffer_replaces_the_lists() {
        let mut e = more();
        let old = extra(&mut e);
        let head = list(&mut e, &[old]);
        let this = change(&mut e, head, 0);
        set(&mut e, CAST_TABLE, 0x2c, 0x5555);
        set(&mut e, SAVE_OBJECT, 0x30, 2);
        e.call(0x004bee00, &args![this, 0xb0ffu32]);
        assert_eq!(e.get(this, ItemChange::pContainerObj), Ptr::new(0x5555));
        assert_eq!(e.get(this, ItemChange::iNumber), 7);
        // The old list was deleted, two new ones loaded.
        assert_eq!(calls(&e, DESTRUCTOR), vec![vec![old, 1]]);
        let loaded = items(&e, head);
        assert_eq!(loaded.len(), 2);
        assert_eq!(
            calls(&e, EXTRA_LOAD_GAME_BUFFER),
            vec![vec![loaded[0], 0xb0ff], vec![loaded[1], 0xb0ff]]
        );
        // No lists: no node is made.
        set(&mut e, SAVE_OBJECT, 0x30, 0);
        let bare = change(&mut e, 0, 0);
        e.call(0x004bee00, &args![bare, 0xb0ffu32]);
        assert_eq!(e.get(bare, ItemChange::pExtraObjectList), Ptr::NULL);
        // A missing node is made when there are lists.
        set(&mut e, SAVE_OBJECT, 0x30, 1);
        e.call(0x004bee00, &args![bare, 0xb0ffu32]);
        let node = e.get(bare, ItemChange::pExtraObjectList).addr();
        assert_eq!(items(&e, node).len(), 1);
    }

    #[test]
    fn finish_load_runs_on_every_non_null_list() {
        let mut e = more();
        let (a, b) = (extra(&mut e), extra(&mut e));
        let head = list(&mut e, &[a, 0, b]);
        let this = change(&mut e, head, 0x4242);
        e.call(0x004bef60, &args![this, 0x99u32]);
        assert_eq!(
            calls(&e, EXTRA_AFTER_LOAD_GAME_BUFFER),
            vec![vec![a, 0x99, 0x4242], vec![b, 0x99, 0x4242]]
        );
    }

    /// An `InventoryChanges` whose list holds `changes` (`ItemChange`
    /// addresses) and whose owner is `owner`.
    fn inventory(e: &mut Engine, changes: &[u32], owner: u32) -> Ptr<InventoryChanges> {
        let this = e.new_object::<InventoryChanges>();
        let head = list(e, changes);
        e.set(this, InventoryChanges::pListofChanges, Ptr::new(head));
        e.set(this, InventoryChanges::pRef, Ptr::new(owner));
        this
    }

    fn inventory_list(e: &Engine, this: Ptr<InventoryChanges>) -> Vec<u32> {
        items(e, e.get(this, InventoryChanges::pListofChanges).addr())
    }

    /// An `ItemChange` for `form` holding `extras`, with `number` items.
    fn entry(e: &mut Engine, form: u32, extras: &[u32], number: i32) -> u32 {
        let head = list(e, extras);
        let c = change(e, head, form);
        e.set(c, ItemChange::iNumber, number);
        c.addr()
    }

    /// An extra data list with hot key `key` (-1 for none).
    fn hot_extra(e: &mut Engine, key: i8) -> u32 {
        let x = extra(e);
        set(e, x, X_HOT_KEY, key as u8 as u32);
        // A non-empty extra list (+4 is the first extra), count 1.
        set(e, x, 4, 1);
        set(e, x, X_COUNT, 1);
        x
    }

    #[test]
    fn constructor_makes_the_list_and_the_scratch_reference_once() {
        let mut e = more();
        e.register(REFR_CONSTRUCT, |_, a| returns(a[0]));
        e.register(vcall_address(0x100), |_, _| returns(0));
        let owner = object(&mut e);
        let this = e.new_object::<InventoryChanges>();
        let r = e.call(0x004befb0, &args![this, owner]);
        assert_eq!(r.u32(), this.addr());
        assert_eq!(e.get(this, InventoryChanges::pRef), Ptr::new(owner));
        let node = e.get(this, InventoryChanges::pListofChanges).addr();
        assert_eq!((e.mem.u32(node), e.mem.u32(node + 4)), (0, 0));
        assert_eq!(e.get(this, InventoryChanges::fcontainerweight), -1.0);
        assert_eq!(
            e.get(this, InventoryChanges::fpreviousContainerWeight),
            -1.0
        );
        assert!(!e.get(this, InventoryChanges::bcountdirty));
        // The scratch reference: 0x68 bytes, made temporary, kept in the global.
        let temp = e.global::<u32>(TEMP_REF_GLOBAL);
        assert_ne!(temp, 0);
        assert_eq!(calls(&e, FORM_SET_TEMPORARY), vec![vec![temp]]);
        assert!(calls(&e, OPERATOR_NEW).contains(&vec![0x68]));
        let guard = &calls(&e, SCOPE_GUARD_OPEN)[0];
        assert_eq!(&guard[1..], &[0x36, 1, SOURCE_FILE_NAME, 0x6de]);
        // The second one reuses it.
        e.call_log = Some(vec![]);
        let other = e.new_object::<InventoryChanges>();
        e.call(0x004befb0, &args![other, Ptr::<()>::NULL]);
        assert!(calls(&e, FORM_SET_TEMPORARY).is_empty());
        assert_eq!(e.global::<u32>(TEMP_REF_GLOBAL), temp);
    }

    #[test]
    fn forgetting_the_weight_tells_the_owner_when_it_asks_for_it() {
        let mut e = more();
        // Virtual 0x100 answers with the byte after the log flag.
        e.register(vcall_address(0x100), |e, _| {
            returns(e.mem.u8(LOG_FLAG + 1) as u32)
        });
        e.register(vcall_address(0x30c), |_, _| Ret::default());
        let owner = object(&mut e);
        let this = inventory(&mut e, &[], owner);
        e.set(this, InventoryChanges::fcontainerweight, 5.0);
        e.call(0x004bf0e0, &args![this]);
        assert_eq!(e.get(this, InventoryChanges::fpreviousContainerWeight), 5.0);
        assert_eq!(e.get(this, InventoryChanges::fcontainerweight), -1.0);
        assert!(calls(&e, vcall_address(0x30c)).is_empty());
        e.mem.set_u8(LOG_FLAG + 1, 1);
        e.call(0x004bf0e0, &args![this]);
        assert_eq!(
            calls(&e, vcall_address(0x30c)),
            vec![vec![owner, (-1.0f32).to_bits()]]
        );
        // Without an owner only the weights change.
        let alone = inventory(&mut e, &[], 0);
        e.call_log = Some(vec![]);
        e.call(0x004bf0e0, &args![alone]);
        assert_eq!(e.call_log.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn destructor_body_deletes_every_change_and_the_list() {
        let mut e = more();
        e.register(vcall_address(0x4c), |_, _| Ret::default());
        let owner = object(&mut e);
        let x = extra(&mut e);
        let (c1, c2) = (entry(&mut e, 0x10, &[x], 1), entry(&mut e, 0x20, &[], 1));
        let this = inventory(&mut e, &[c1, c2], owner);
        let head = e.get(this, InventoryChanges::pListofChanges).addr();
        e.call(0x004bf150, &args![this]);
        // Each change got DeleteAllExtra (the first one's list is removed
        // and deleted) and was deleted.
        assert_eq!(calls(&e, EXTRA_REMOVE_ALL), vec![vec![x, 1]]);
        assert_eq!(
            calls(&e, ITEM_CHANGE_DELETE),
            vec![vec![c1, 1], vec![c2, 1]]
        );
        assert_eq!(calls(&e, LIST_CLEAR), vec![vec![head]]);
        assert_eq!(
            calls(&e, vcall_address(0x4c)),
            vec![vec![owner, 0x0800_0020]]
        );
        assert_eq!(calls(&e, LIST_DESTROY), vec![vec![head, 1]]);
    }

    #[test]
    fn get_inventory_changes_makes_and_stores_the_changes_once() {
        let mut e = more();
        e.register(REFR_CONSTRUCT, |_, a| returns(a[0]));
        // Virtual 0xfc and 0x100 answer with the two bytes after the log flag.
        e.register(vcall_address(0xfc), |e, _| {
            returns(e.mem.u8(LOG_FLAG + 2) as u32)
        });
        e.register(vcall_address(0x100), |e, _| {
            returns(e.mem.u8(LOG_FLAG + 3) as u32)
        });
        let reference = object(&mut e);
        // 0xfc true and 0x100 false: nothing is made.
        e.mem.set_u8(LOG_FLAG + 2, 1);
        assert_eq!(e.call(0x004bf220, &args![reference]).u32(), 0);
        assert!(calls(&e, OPERATOR_NEW).is_empty());
        // 0xfc false: made once, then found again.
        e.mem.set_u8(LOG_FLAG + 2, 0);
        let made = e.call(0x004bf220, &args![reference]).u32();
        assert_ne!(made, 0);
        let made_ref = e.get(Ptr::<InventoryChanges>::new(made), InventoryChanges::pRef);
        assert_eq!(made_ref, Ptr::new(reference));
        assert_eq!(
            calls(&e, EXTRA_SET_CONTAINER_CHANGES),
            vec![vec![reference + 0x40, made]]
        );
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004bf220, &args![reference]).u32(), made);
        assert!(calls(&e, EXTRA_SET_CONTAINER_CHANGES).is_empty());
        // 0xfc true and 0x100 true also works.
        e.mem.set_u8(LOG_FLAG + 2, 1);
        e.mem.set_u8(LOG_FLAG + 3, 1);
        assert_eq!(e.call(0x004bf220, &args![reference]).u32(), made);
    }

    #[test]
    fn remove_hot_key_item_unhooks_the_keys_and_drops_emptied_lists() {
        let mut e = more();
        let form_a = form(&mut e, 0x28, 0, 0);
        // Key 3 only: its list has no extras left, so it leaves the entry.
        let (x1, x2) = (hot_extra(&mut e, 3), hot_extra(&mut e, 5));
        set(&mut e, x1, 4, 0);
        let entry_a = entry(&mut e, form_a, &[x1, x2], 2);
        let this = inventory(&mut e, &[entry_a], 0);
        let item = change(&mut e, 0, form_a);
        e.call(0x004bf330, &args![this, item, 3i32]);
        assert_eq!(calls(&e, EXTRA_REMOVE_HOT_KEY), vec![vec![x1]]);
        assert_eq!(items(&e, e.mem.u32(entry_a)), vec![x2]);
        // -1 removes every key; lists with extras stay.
        let (y1, y2, y3) = (
            hot_extra(&mut e, 3),
            hot_extra(&mut e, 5),
            hot_extra(&mut e, -1),
        );
        let entry_b = entry(&mut e, form_a, &[y1, y2, y3], 2);
        let this = inventory(&mut e, &[entry_b], 0);
        e.call_log = Some(vec![]);
        e.call(0x004bf330, &args![this, item, -1i32]);
        assert_eq!(calls(&e, EXTRA_REMOVE_HOT_KEY), vec![vec![y1], vec![y2]]);
        assert_eq!(items(&e, e.mem.u32(entry_b)), vec![y1, y2, y3]);
        // A list of a count above one holding one kind of extra leaves too.
        let z = hot_extra(&mut e, 2);
        set(&mut e, z, X_COUNT, 3);
        set(&mut e, z, X_ITEMS_IN_LIST, 1);
        let entry_c = entry(&mut e, form_a, &[z], 2);
        let this = inventory(&mut e, &[entry_c], 0);
        e.call(0x004bf330, &args![this, item, 2i32]);
        // The entry has no list left but still counts items: it stays.
        assert_eq!(e.mem.u32(entry_c), 0);
        assert_eq!(inventory_list(&e, this), vec![entry_c]);
    }

    #[test]
    fn remove_hot_key_item_deletes_an_entry_left_empty_with_no_items() {
        let mut e = more();
        let form_a = form(&mut e, 0x28, 0, 0);
        let x1 = hot_extra(&mut e, 3);
        set(&mut e, x1, 4, 0);
        let entry_a = entry(&mut e, form_a, &[x1], 0);
        let other = entry(&mut e, 0x4444, &[], 1);
        let this = inventory(&mut e, &[other, entry_a], 0);
        let item = change(&mut e, 0, form_a);
        let list_head = e.mem.u32(entry_a);
        e.call(0x004bf330, &args![this, item, 3i32]);
        assert_eq!(calls(&e, LIST_DESTROY), vec![vec![list_head, 1]]);
        assert_eq!(inventory_list(&e, this), vec![other]);
        assert_eq!(calls(&e, ITEM_CHANGE_DELETE), vec![vec![entry_a, 1]]);
        // No entry for the form: nothing happens.
        e.call_log = Some(vec![]);
        let stranger = form(&mut e, 0x28, 0, 0);
        let none = change(&mut e, 0, stranger);
        e.call(0x004bf330, &args![this, none, -1i32]);
        assert!(calls(&e, EXTRA_REMOVE_HOT_KEY).is_empty());
        assert!(calls(&e, ITEM_CHANGE_DELETE).is_empty());
    }

    /// Doubles for the fast inventory iteration: it hands out `items` and
    /// then 0; the iterator is 0x1111.
    fn fast_iteration(e: &mut Engine, items: Vec<u32>) {
        e.register(START_FAST_ITERATION, |_, _| returns(0x1111));
        let mut remaining = items.into_iter();
        e.register_double(GET_NEXT_FAST_ITEM, move |_, a| {
            assert_eq!(a[1], 0x1111);
            returns(remaining.next().unwrap_or(0))
        });
    }

    #[test]
    fn hot_key_lookup_returns_the_first_item_with_the_key() {
        let mut e = more();
        let (xa, xb) = (hot_extra(&mut e, 1), hot_extra(&mut e, 2));
        let (a, b) = (entry(&mut e, 0x10, &[xa], 1), entry(&mut e, 0x20, &[xb], 1));
        fast_iteration(&mut e, vec![a, b]);
        let this = inventory(&mut e, &[], 0);
        assert_eq!(e.call(0x004bf4b0, &args![this, 2i32]).u32(), b);
        // The item without the key and the iterator are freed.
        assert_eq!(calls(&e, ITEM_CHANGE_DELETE), vec![vec![a, 1]]);
        assert_eq!(calls(&e, FAST_ITERATOR_DESTRUCT), vec![vec![0x1111]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x1111]]);
        let guard = &calls(&e, SCOPE_GUARD_OPEN)[0];
        assert_eq!(&guard[1..], &[0x36, 1, SOURCE_FILE_NAME, 0x76a]);
        assert_eq!(calls(&e, SCOPE_GUARD_CLOSE).len(), 1);
        // No item has key 7: everything is freed and 0 returned.
        fast_iteration(&mut e, vec![a, b]);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004bf4b0, &args![this, 7i32]).u32(), 0);
        assert_eq!(calls(&e, ITEM_CHANGE_DELETE), vec![vec![a, 1], vec![b, 1]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x1111]]);
    }

    #[test]
    fn iterator_destructor_frees_only_when_asked() {
        let mut e = more();
        assert_eq!(e.call(0x004bf630, &args![0x1111u32, 0u32]).u32(), 0x1111);
        assert!(calls(&e, OPERATOR_DELETE).is_empty());
        assert_eq!(calls(&e, FAST_ITERATOR_DESTRUCT), vec![vec![0x1111]]);
        assert_eq!(e.call(0x004bf630, &args![0x1111u32, 1u32]).u32(), 0x1111);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x1111]]);
    }

    #[test]
    fn hot_key_array_holds_the_last_item_of_each_key() {
        let mut e = more();
        let (xa, xb, xc, xd) = (
            hot_extra(&mut e, 1),
            hot_extra(&mut e, 3),
            hot_extra(&mut e, 1),
            hot_extra(&mut e, 7),
        );
        let items_made: Vec<u32> = [xa, xb, xc, xd]
            .iter()
            .map(|x| entry(&mut e, 0x10, &[*x], 1))
            .collect();
        let (a, b, c, d) = (items_made[0], items_made[1], items_made[2], items_made[3]);
        fast_iteration(&mut e, items_made);
        let this = inventory(&mut e, &[], 0);
        let array = e.call(0x004bf660, &args![this, 1i32, 3i32]).u32();
        assert_eq!(calls(&e, OPERATOR_NEW), vec![vec![12]]);
        assert_eq!(
            (e.mem.u32(array), e.mem.u32(array + 4), e.mem.u32(array + 8)),
            (c, 0, b)
        );
        // `a` was replaced by `c`, `d` has no slot: both freed.
        assert_eq!(calls(&e, ITEM_CHANGE_DELETE), vec![vec![a, 1], vec![d, 1]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x1111]]);
        // An empty range gives no array.
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004bf660, &args![this, 3i32, 1i32]).u32(), 0);
        assert_eq!(e.call_log.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn set_hot_key_item_ignores_bad_arguments() {
        let mut e = more();
        let this = inventory(&mut e, &[], 0);
        let item = change(&mut e, 0, 0x10);
        e.call(0x004bf800, &args![this, item, 0u32, 8i32]);
        e.call(0x004bf800, &args![this, item, 0u32, -1i32]);
        e.call(
            0x004bf800,
            &args![this, Ptr::<ItemChange>::NULL, 0u32, 3i32],
        );
        // Only the scope guard calls were made.
        for (address, _) in e.call_log.as_ref().unwrap() {
            assert!(
                [0x004bf800, SCOPE_GUARD_OPEN, SCOPE_GUARD_CLOSE].contains(address),
                "{address:08x}"
            );
        }
        assert_eq!(calls(&e, SCOPE_GUARD_OPEN).len(), 3);
    }

    #[test]
    fn set_hot_key_item_uses_the_default_list_or_the_given_one() {
        let mut e = more();
        e.mem.set_u32(PLAYER, BIG_VTABLE);
        let form_a = form(&mut e, 0x28, 0, 0);
        let default = hot_extra(&mut e, -1);
        set(&mut e, default, X_DEFAULT, 1);
        let given = hot_extra(&mut e, -1);
        let entry_a = entry(&mut e, form_a, &[default, given], 2);
        let this = inventory(&mut e, &[entry_a], 0);
        let item = change(&mut e, 0, form_a);
        // The player had a hot key table entry for key 4: it is cleared.
        let table_entry = PLAYER + HOT_KEY_TABLE + 0x100 + 16;
        e.mem.set_u32(table_entry, 9);
        e.call(0x004bf800, &args![this, item, 0u32, 4i32]);
        assert_eq!(e.mem.u32(table_entry), 0);
        assert_eq!(calls(&e, EXTRA_SET_HOT_KEY), vec![vec![default, 4]]);
        // The given list wins when it is in the entry, even after the default.
        e.call_log = Some(vec![]);
        e.call(0x004bf800, &args![this, item, given, 5i32]);
        assert_eq!(calls(&e, EXTRA_SET_HOT_KEY), vec![vec![given, 5]]);
    }

    #[test]
    fn set_hot_key_item_makes_a_list_for_the_items_outside_lists() {
        let mut e = more();
        e.mem.set_u32(PLAYER, BIG_VTABLE);
        let form_a = form(&mut e, 0x28, 0, 0);
        // One list of two (not default, not worn) and 5 items: 3 are left.
        let counted = hot_extra(&mut e, -1);
        set(&mut e, counted, X_COUNT, 2);
        let entry_a = entry(&mut e, form_a, &[counted], 5);
        let this = inventory(&mut e, &[entry_a], 0);
        let item = change(&mut e, 0, form_a);
        e.set(item, ItemChange::iNumber, 5);
        e.call(0x004bf800, &args![this, item, 0u32, 6i32]);
        let lists = items(&e, e.mem.u32(entry_a));
        assert_eq!(lists.len(), 2);
        assert_eq!(e.mem.u32(lists[1] + X_HOT_KEY), 6);
        assert_eq!(e.mem.u32(lists[1] + X_COUNT), 3);
        // A worn one among the lists takes one more.
        let worn = hot_extra(&mut e, -1);
        set(&mut e, worn, X_WORN, 1);
        set(&mut e, worn, X_COUNT, 1);
        let entry_b = entry(&mut e, form_a, &[worn], 5);
        let this = inventory(&mut e, &[entry_b], 0);
        e.call(0x004bf800, &args![this, item, 0u32, 6i32]);
        let lists = items(&e, e.mem.u32(entry_b));
        assert_eq!(e.mem.u32(lists[1] + X_COUNT), 4);
    }

    #[test]
    fn set_hot_key_item_adds_a_change_when_the_form_has_none() {
        let mut e = more();
        e.mem.set_u32(PLAYER, BIG_VTABLE);
        let form_a = form(&mut e, 0x28, 0, 0);
        let this = inventory(&mut e, &[], 0);
        let item = change(&mut e, 0, form_a);
        e.set(item, ItemChange::iNumber, 5);
        e.call(0x004bf800, &args![this, item, 0u32, 2i32]);
        let changes = inventory_list(&e, this);
        assert_eq!(changes.len(), 1);
        let made = Ptr::<ItemChange>::new(changes[0]);
        assert_eq!(e.get(made, ItemChange::pContainerObj), Ptr::new(form_a));
        assert_eq!(e.get(made, ItemChange::iNumber), 5);
        let lists = items(&e, e.get(made, ItemChange::pExtraObjectList).addr());
        assert_eq!(lists.len(), 1);
        assert_eq!(e.mem.u32(lists[0] + X_HOT_KEY), 2);
    }

    #[test]
    fn hot_key_table_of_the_player() {
        let mut e = more();
        let table_entry = PLAYER + HOT_KEY_TABLE + 0x100 + 4 * 8;
        e.mem.set_u32(table_entry, 0x77);
        assert_eq!(e.call(0x004bfb30, &args![PLAYER, 8i32]).u32(), 0x77);
        assert_eq!(e.call(0x004bfb30, &args![PLAYER, 9i32]).u32(), 0);
        assert_eq!(e.call(0x004bfb30, &args![PLAYER, -1i32]).u32(), 0);
        e.call(0x004bfb70, &args![PLAYER, 8i32, 0x99u32]);
        assert_eq!(e.mem.u32(table_entry), 0x99);
        assert_eq!(
            calls(&e, HOT_KEY_TABLE_ENTRY),
            vec![vec![PLAYER + HOT_KEY_TABLE, 8]]
        );
    }

    #[test]
    fn object_in_list_matches_the_form_and_optionally_a_reference() {
        let mut e = more();
        let reference = e.mem.alloc(0x20);
        set(&mut e, reference, 0xc, 0xfeed);
        let with_ref = extra(&mut e);
        set(&mut e, with_ref, X_REFERENCE, reference);
        let (a, b) = (
            entry(&mut e, 0x10, &[], 1),
            entry(&mut e, 0x20, &[with_ref], 1),
        );
        let this = inventory(&mut e, &[a, b], 0);
        assert_eq!(
            e.call(0x004bfba0, &args![this, 0x20u32, 1u32, 0u32]).u32(),
            b
        );
        assert_eq!(
            e.call(0x004bfba0, &args![this, 0x10u32, 1u32, 0u32]).u32(),
            a
        );
        assert_eq!(
            e.call(0x004bfba0, &args![this, 0x30u32, 1u32, 0u32]).u32(),
            0
        );
        // With a reference id: the entry needs a list with that reference.
        assert_eq!(
            e.call(0x004bfba0, &args![this, 0x20u32, 1u32, 0xfeedu32])
                .u32(),
            b
        );
        assert_eq!(
            e.call(0x004bfba0, &args![this, 0x20u32, 1u32, 0xbeefu32])
                .u32(),
            0
        );
        assert_eq!(
            e.call(0x004bfba0, &args![this, 0x10u32, 1u32, 0xfeedu32])
                .u32(),
            0
        );
    }

    #[test]
    fn worn_armor_with_the_flag_is_found() {
        let mut e = more();
        let armor = form(&mut e, 0x18, 0, 0);
        let weapon = form(&mut e, 0x28, 0, 0);
        let worn = extra(&mut e);
        set(&mut e, worn, X_WORN, 1);
        let (armor_entry, weapon_entry) = (
            entry(&mut e, armor, &[worn], 1),
            entry(&mut e, weapon, &[worn], 1),
        );
        // The flag byte is cleared: not found.
        let this = inventory(&mut e, &[armor_entry], 0);
        assert!(!e.call(0x004bfc80, &args![this]).bool());
        e.mem.set_u8(armor + ARMOR_FLAGS, 1);
        assert!(e.call(0x004bfc80, &args![this]).bool());
        // A weapon is not looked at; the worn armor after it is.
        let both = inventory(&mut e, &[weapon_entry, armor_entry], 0);
        assert!(e.call(0x004bfc80, &args![both]).bool());
        let only_weapon = inventory(&mut e, &[weapon_entry], 0);
        assert!(!e.call(0x004bfc80, &args![only_weapon]).bool());
        // An armor with a list that is not worn does not count.
        let plain = extra(&mut e);
        let unworn = entry(&mut e, armor, &[plain], 1);
        let third = inventory(&mut e, &[unworn], 0);
        assert!(!e.call(0x004bfc80, &args![third]).bool());
        // The flag helper.
        assert!(e.call(0x004bfd80, &args![armor]).bool());
        e.mem.set_u8(armor + ARMOR_FLAGS, 0xfe);
        assert!(!e.call(0x004bfd80, &args![armor]).bool());
    }

    #[test]
    fn wearing_object_finds_the_worn_list_or_wears_an_unwearable_one() {
        let mut e = more();
        let form_a = form(&mut e, 0x18, 0, 0);
        let (plain, worn, cannot) = (extra(&mut e), extra(&mut e), extra(&mut e));
        set(&mut e, worn, X_WORN, 1);
        set(&mut e, cannot, X_CAN_NOT_WEAR, 1);
        let with_worn = entry(&mut e, form_a, &[plain, worn], 2);
        let this = inventory(&mut e, &[with_worn], 0);
        assert_eq!(e.call(0x004bfda0, &args![this, form_a, 0u32]).u32(), worn);
        // A can-not-wear list is only taken with the flag, and then worn.
        let with_cannot = entry(&mut e, form_a, &[plain, cannot], 2);
        let other = inventory(&mut e, &[with_cannot], 0);
        assert_eq!(e.call(0x004bfda0, &args![other, form_a, 0u32]).u32(), 0);
        assert!(calls(&e, EXTRA_SET_WORN).is_empty());
        assert_eq!(
            e.call(0x004bfda0, &args![other, form_a, 1u32]).u32(),
            cannot
        );
        assert_eq!(calls(&e, EXTRA_SET_WORN), vec![vec![cannot, 1, 0]]);
        // No entry for the form.
        assert_eq!(e.call(0x004bfda0, &args![other, 0x99u32, 1u32]).u32(), 0);
    }

    #[test]
    fn remove_all_objects_worn_removes_each_worn_change_until_it_stays() {
        let mut e = more();
        let form_a = form(&mut e, 0x18, 0, 0);
        let (plain, worn) = (extra(&mut e), extra(&mut e));
        set(&mut e, worn, X_WORN, 1);
        set(&mut e, worn, X_COUNT, 2);
        let (unworn_entry, worn_entry) = (
            entry(&mut e, 0x4444, &[plain], 1),
            entry(&mut e, form_a, &[plain, worn], 3),
        );
        let this = inventory(&mut e, &[unworn_entry, worn_entry], 0);
        let actor = object(&mut e);
        // The removal reports a removal the first time only.
        let mut removals = 0;
        e.register_double(INVENTORY_REMOVE_ITEM, move |e, a| {
            removals += 1;
            e.mem.set_u8(a[1], (removals == 1) as u8);
            Ret::default()
        });
        e.call(0x004bfe50, &args![this, 0u32, actor]);
        let removes = calls(&e, INVENTORY_REMOVE_ITEM);
        assert_eq!(removes.len(), 2);
        let args_of = |call: &Vec<u32>| {
            let mut words = call.clone();
            words[1] = 0;
            words
        };
        assert_eq!(
            args_of(&removes[0]),
            vec![this.addr(), 0, form_a, 2, actor, worn, 0, 1, worn_entry, 0]
        );
        assert_eq!(args_of(&removes[1]), args_of(&removes[0]));
        assert_eq!(
            calls(&e, REFERENCE_FORM_NOTIFY)[0],
            vec![actor + 0x94, form_a, 1]
        );
        // Nothing worn and no reference: nothing is removed or passed on.
        e.call_log = Some(vec![]);
        let plain_inventory = inventory(&mut e, &[unworn_entry], 0);
        e.call(0x004bfe50, &args![plain_inventory, 0u32, 0u32]);
        assert!(calls(&e, INVENTORY_REMOVE_ITEM).is_empty());
        assert!(calls(&e, REFERENCE_FORM_NOTIFY).is_empty());
    }

    #[test]
    fn container_of_the_owner() {
        let mut e = more();
        e.register(REFR_HAS_CONTAINER, |_, a| returns(a[0] + 0x10));
        let none = inventory(&mut e, &[], 0);
        assert_eq!(e.call(0x004bffb0, &args![none]).u32(), 0);
        let owned = inventory(&mut e, &[], 0x5000);
        assert_eq!(e.call(0x004bffb0, &args![owned]).u32(), 0x5010);
    }

    #[test]
    fn small_form_predicates() {
        let mut e = more();
        let o = object(&mut e);
        e.mem.set_u8(o + 8, 0x80);
        assert!(e.call(0x004c0bd0, &args![o]).bool());
        e.mem.set_u8(o + 8, 0x7f);
        assert!(!e.call(0x004c0bd0, &args![o]).bool());
        // Weapon types: 3..=13 and 3..=9, and their difference.
        for (kind, wide, narrow, difference) in [
            (2i8, false, false, false),
            (3, true, true, false),
            (9, true, true, false),
            (10, true, false, true),
            (13, true, false, true),
            (14, false, false, false),
            (-3, false, false, false),
        ] {
            e.mem.set_i8(o + WEAPON_TYPE, kind);
            assert_eq!(e.call(0x004c0c30, &args![o]).bool(), wide, "{kind}");
            assert_eq!(e.call(0x004c0c60, &args![o]).bool(), narrow, "{kind}");
            assert_eq!(e.call(0x004c0bf0, &args![o]).bool(), difference, "{kind}");
        }
    }

    /// An engine with the doubles `004bffe0` needs on top of `more()`.
    fn equip_engine() -> Engine {
        let mut e = more();
        e.register(vcall_address(0x100), |_, _| returns(0));
        e.register(vcall_address(0x468), |_, _| Ret::default());
        e.register(vcall_address(0x6e4), |_, _| Ret::default());
        e.register(vcall_address(0x48), |_, _| Ret::default());
        e.register(REFR_HAS_CONTAINER, |_, a| returns(a[0] + 0x10));
        e.register(CONTAINER_COUNT, |e, _| returns(e.mem.u32(LOOKUP_CELL + 8)));
        e.register(REFR_CONSTRUCT, |_, a| returns(a[0]));
        e.register(GET_FORM_AS_BIPED_MODEL, |_, _| returns(0));
        e.register(BIPED_IS_POWER_ARMOR, |e, a| {
            returns((e.mem.u8(a[0] + 8) & 0x20 != 0) as u32)
        });
        e.register(BIPED_FLAG_FOUR, |e, a| {
            returns((e.mem.u8(a[0] + 8) & 4 != 0) as u32)
        });
        e.register(ACTOR_PROCESS, |e, a| returns(e.mem.u32(a[0] + 0x68)));
        e.register(ACTOR_GET_ANIMATION, |e, a| returns(e.mem.u32(a[0] + 0x6c)));
        e.register(ACTOR_LIGHTING_ARGUMENT, |_, _| returns(0x9999));
        e.register(FORM_WEIGHT_FIELD, |_, _| returns_float(2.5));
        e.register(PREDICATE_008C7AA0, |e, _| {
            returns(e.mem.u8(LOG_FLAG + 4) as u32)
        });
        e.register(THREAD_VALUE_0047B200, |e, _| {
            returns(e.mem.u32(LOG_FLAG + 8))
        });
        e.register(TASK_QUEUE_GETTER, |_, _| returns(0x7777));
        e.register(vcall_address(0x1d0), |_, _| returns(0x55));
        e.mem.set_u32(PLAYER, BIG_VTABLE);
        e
    }

    /// An inventory (no owner), an armor of type `form_type`, an actor with a
    /// process (both big-vtable objects) and an animation object.
    fn equip_setup(e: &mut Engine, form_type: u8) -> (Ptr<InventoryChanges>, u32, u32, u32) {
        let this = inventory(e, &[], 0);
        let item_form = form(e, form_type, 0, 0);
        let actor = object(e);
        let process = object(e);
        let animation = e.mem.alloc(0x200);
        e.mem.set_u32(actor + 0x68, process);
        e.mem.set_u32(actor + 0x6c, animation);
        (this, item_form, actor, process)
    }

    #[test]
    fn equip_makes_an_entry_and_a_worn_list_for_a_new_form() {
        let mut e = equip_engine();
        let (this, armor, actor, _) = equip_setup(&mut e, 0x18);
        e.call(
            0x004bffe0,
            &args![this, armor, 3i32, actor, 0u32, 1u32, 0u32],
        );
        let changes = inventory_list(&e, this);
        assert_eq!(changes.len(), 1);
        let made = Ptr::<ItemChange>::new(changes[0]);
        assert_eq!(e.get(made, ItemChange::pContainerObj), Ptr::new(armor));
        assert_eq!(e.get(made, ItemChange::iNumber), 0);
        let lists = items(&e, e.get(made, ItemChange::pExtraObjectList).addr());
        assert_eq!(lists.len(), 1);
        // Worn with the given flag, the count 3, the can-not-wear flag given.
        assert_eq!(e.mem.u32(lists[0] + X_WORN), 1);
        assert_eq!(e.mem.u32(lists[0] + X_WORN_FLAG), 1);
        assert_eq!(e.mem.u32(lists[0] + X_COUNT), 3);
        assert_eq!(calls(&e, EXTRA_SET_CAN_NOT_WEAR), vec![vec![lists[0], 0]]);
        let guard = &calls(&e, SCOPE_GUARD_OPEN)[0];
        assert_eq!(&guard[1..], &[0x36, 1, SOURCE_FILE_NAME, 0x960]);
        assert_eq!(calls(&e, SCOPE_GUARD_CLOSE).len(), 1);
        // A count of 1 leaves the count of the new list alone.
        let (other, armor_two, actor_two, _) = equip_setup(&mut e, 0x18);
        e.call(
            0x004bffe0,
            &args![other, armor_two, 1i32, actor_two, 0u32, 0u32, 1u32],
        );
        let made = Ptr::<ItemChange>::new(inventory_list(&e, other)[0]);
        let lists = items(&e, e.get(made, ItemChange::pExtraObjectList).addr());
        assert_eq!(e.mem.u32(lists[0] + X_COUNT), 0);
        assert_eq!(e.mem.u32(lists[0] + X_CAN_NOT_WEAR), 1);
    }

    #[test]
    fn equip_splits_one_item_off_a_stack() {
        let mut e = equip_engine();
        let (this, armor, actor, _) = equip_setup(&mut e, 0x18);
        // A default list of two items with a hot key, in an entry of 2.
        let stack = hot_extra(&mut e, 3);
        set(&mut e, stack, X_DEFAULT, 1);
        set(&mut e, stack, X_COUNT, 2);
        let entry_a = entry(&mut e, armor, &[stack], 2);
        let head = e.get(this, InventoryChanges::pListofChanges).addr();
        list_call_with_item(&mut e, LIST_ADD, head, entry_a);
        e.call(
            0x004bffe0,
            &args![this, armor, 1i32, actor, 0u32, 1u32, 0u32],
        );
        let lists = items(&e, e.mem.u32(entry_a));
        assert_eq!(lists.len(), 2);
        let (rest, worn) = (lists[0], lists[1]);
        assert_eq!(rest, stack);
        // One left in the stack, without its hot key; the copy is worn.
        assert_eq!(e.mem.u32(rest + X_COUNT), 1);
        assert_eq!(e.mem.u32(rest + X_HOT_KEY), 0xff);
        assert_eq!(e.mem.u32(worn + 0x14), stack);
        assert_eq!(e.mem.u32(worn + X_WORN), 1);
        assert_eq!(e.mem.u32(worn + X_COUNT), 1);
        // Ammunition is not split.
        e.register(vcall_address(0x14c), |_, _| returns(0));
        e.register(vcall_address(0x168), |_, _| returns(1));
        e.register(vcall_address(0x474), |_, _| returns(0));
        let (this, ammo, actor, _) = equip_setup(&mut e, 0x29);
        let stack = hot_extra(&mut e, -1);
        set(&mut e, stack, X_COUNT, 5);
        set(&mut e, stack, X_DEFAULT, 1);
        let entry_b = entry(&mut e, ammo, &[stack], 5);
        let head = e.get(this, InventoryChanges::pListofChanges).addr();
        list_call_with_item(&mut e, LIST_ADD, head, entry_b);
        e.call(
            0x004bffe0,
            &args![this, ammo, 5i32, actor, 0u32, 1u32, 0u32],
        );
        assert_eq!(items(&e, e.mem.u32(entry_b)), vec![stack]);
        assert_eq!(e.mem.u32(stack + X_WORN), 1);
        assert_eq!(e.mem.u32(stack + X_COUNT), 5);
    }

    #[test]
    fn equip_wears_the_given_list_and_does_nothing_for_a_worn_entry() {
        let mut e = equip_engine();
        let (this, armor, actor, _) = equip_setup(&mut e, 0x18);
        let given = hot_extra(&mut e, -1);
        let entry_a = entry(&mut e, armor, &[given], 1);
        let head = e.get(this, InventoryChanges::pListofChanges).addr();
        list_call_with_item(&mut e, LIST_ADD, head, entry_a);
        e.call(
            0x004bffe0,
            &args![this, armor, 1i32, actor, given, 7u32, 0u32],
        );
        // Worn twice over (the stack is whole, then the requested count).
        assert_eq!(e.mem.u32(given + X_WORN), 1);
        assert_eq!(e.mem.u32(given + X_WORN_FLAG), 7);
        assert_eq!(items(&e, e.mem.u32(entry_a)), vec![given]);
        assert_eq!(calls(&e, EXTRA_SET_WORN).len(), 2);
        // With a worn list in the entry nothing more is done.
        e.call_log = Some(vec![]);
        e.call(
            0x004bffe0,
            &args![this, armor, 1i32, actor, given, 7u32, 0u32],
        );
        assert!(calls(&e, EXTRA_SET_WORN).is_empty());
        assert!(calls(&e, ACTOR_INIT_LIGHTING).is_empty());
        assert_eq!(calls(&e, SCOPE_GUARD_CLOSE).len(), 1);
        // The walk wears the list before the can-not-wear check: nothing is
        // added and the flag is set on that list.
        let (this, armor, actor, _) = equip_setup(&mut e, 0x18);
        let fixed = hot_extra(&mut e, -1);
        set(&mut e, fixed, X_CAN_NOT_WEAR, 1);
        let entry_b = entry(&mut e, armor, &[fixed], 1);
        let head = e.get(this, InventoryChanges::pListofChanges).addr();
        list_call_with_item(&mut e, LIST_ADD, head, entry_b);
        e.call(
            0x004bffe0,
            &args![this, armor, 1i32, actor, fixed, 0u32, 0u32],
        );
        assert_eq!(e.mem.u32(fixed + X_WORN), 1);
        assert_eq!(items(&e, e.mem.u32(entry_b)), vec![fixed]);
        assert_eq!(calls(&e, EXTRA_SET_CAN_NOT_WEAR), vec![vec![fixed, 0]]);
    }

    #[test]
    fn equip_tells_the_owner_and_counts_the_container() {
        let mut e = equip_engine();
        let (_, armor, actor, _) = equip_setup(&mut e, 0x18);
        let owner = object(&mut e);
        let this = inventory(&mut e, &[], owner);
        e.mem.set_u32(LOOKUP_CELL + 8, 4);
        e.call(
            0x004bffe0,
            &args![this, armor, 1i32, actor, 0u32, 0u32, 0u32],
        );
        assert_eq!(calls(&e, vcall_address(0x48)), vec![vec![owner, 0x20]]);
        assert_eq!(calls(&e, CONTAINER_COUNT), vec![vec![owner + 0x10, armor]]);
        assert_eq!(calls(&e, GET_FORM_AS_BIPED_MODEL)[0], vec![armor]);
        // The weights were forgotten first.
        assert_eq!(e.get(this, InventoryChanges::fcontainerweight), -1.0);
    }

    #[test]
    fn equip_announces_armor_to_the_process() {
        let mut e = equip_engine();
        e.register(vcall_address(0x6e4), |_, _| Ret::default());
        e.register(vcall_address(0x468), |_, _| Ret::default());
        let (this, armor, actor, process) = equip_setup(&mut e, 0x18);
        // Power armor: the acquire object is told, then the biped slot set.
        e.mem.set_u8(armor + ARMOR_BIPED_MODEL + 8, 0x20);
        e.call(
            0x004bffe0,
            &args![this, armor, 1i32, actor, 0u32, 0u32, 0u32],
        );
        assert_eq!(calls(&e, vcall_address(0x6e4)), vec![vec![process, actor]]);
        assert_eq!(calls(&e, vcall_address(0x468)), vec![vec![process, 1]]);
        assert_eq!(calls(&e, ACTOR_INIT_LIGHTING), vec![vec![actor, 0x9999]]);
        // Plain armor: only the biped slot.
        e.mem.set_u8(armor + ARMOR_BIPED_MODEL + 8, 0);
        e.call_log = Some(vec![]);
        let (other, armor_two, actor_two, process_two) = equip_setup(&mut e, 0x18);
        e.call(
            0x004bffe0,
            &args![other, armor_two, 1i32, actor_two, 0u32, 0u32, 0u32],
        );
        assert!(calls(&e, vcall_address(0x6e4)).is_empty());
        assert_eq!(calls(&e, vcall_address(0x468)), vec![vec![process_two, 1]]);
        // The flag bits 0x80 and 4 also count as power armor kinds.
        for bits in [0x80u8, 0x04] {
            e.call_log = Some(vec![]);
            let (third, armor_three, actor_three, _) = equip_setup(&mut e, 0x18);
            e.mem.set_u8(armor_three + ARMOR_BIPED_MODEL + 8, bits);
            e.call(
                0x004bffe0,
                &args![third, armor_three, 1i32, actor_three, 0u32, 0u32, 0u32],
            );
            assert_eq!(calls(&e, vcall_address(0x6e4)).len(), 1, "{bits:02x}");
        }
        // Type 0x1a: only the biped slot, no acquire object.
        e.call_log = Some(vec![]);
        let (fourth, helmet, actor_four, process_four) = equip_setup(&mut e, 0x1a);
        e.call(
            0x004bffe0,
            &args![fourth, helmet, 1i32, actor_four, 0u32, 0u32, 0u32],
        );
        assert_eq!(calls(&e, vcall_address(0x468)), vec![vec![process_four, 1]]);
        // An actor without a process is left alone.
        e.call_log = Some(vec![]);
        let (fifth, armor_five, actor_five, _) = equip_setup(&mut e, 0x18);
        e.mem.set_u32(actor_five + 0x68, 0);
        e.call(
            0x004bffe0,
            &args![fifth, armor_five, 1i32, actor_five, 0u32, 0u32, 0u32],
        );
        assert!(calls(&e, ACTOR_INIT_LIGHTING).is_empty());
    }

    #[test]
    fn equip_a_weapon_keeps_a_new_item_in_the_process() {
        let mut e = equip_engine();
        e.register(vcall_address(0x148), |_, _| returns(0));
        e.register(vcall_address(0x160), |_, _| returns(1));
        e.register(vcall_address(0x474), |_, _| returns(0));
        let (this, weapon, actor, process) = equip_setup(&mut e, 0x28);
        e.call(
            0x004bffe0,
            &args![this, weapon, 2i32, actor, 0u32, 1u32, 0u32],
        );
        let accepted = calls(&e, vcall_address(0x160));
        assert_eq!(accepted.len(), 1);
        let kept = Ptr::<ItemChange>::new(accepted[0][1]);
        assert_eq!(&accepted[0][..1], &[process]);
        assert_eq!(accepted[0][2], 0x55);
        assert_eq!(e.get(kept, ItemChange::pContainerObj), Ptr::new(weapon));
        assert_eq!(e.get(kept, ItemChange::iNumber), 2);
        let worn_lists = items(&e, e.get(kept, ItemChange::pExtraObjectList).addr());
        assert_eq!(worn_lists.len(), 1);
        assert_eq!(e.mem.u32(worn_lists[0] + X_WORN), 1);
        // The entry point 0x2b is run with the weight and the animation told.
        let entry_point = &calls(&e, HANDLE_ENTRY_POINT)[0];
        assert_eq!(&entry_point[..4], &[0x2b, actor, weapon, entry_point[3]]);
        assert_eq!(
            calls(&e, ANIMATION_STORE_FLOAT),
            vec![vec![e.mem.u32(actor + 0x6c), 2.5f32.to_bits()]]
        );
        // The model is updated (not the player, not the task queue).
        assert_eq!(calls(&e, REFR_UPDATE_WEAPON), vec![vec![actor, weapon]]);
        assert!(calls(&e, TASK_QUEUE_ATTACH_WEAPON).is_empty());
        // The special thread asks for the task queue unless it is the actor.
        e.mem.set_u8(LOG_FLAG + 4, 1);
        e.mem.set_u32(LOG_FLAG + 8, 0x1);
        let (other, weapon_two, actor_two, _) = equip_setup(&mut e, 0x28);
        e.call_log = Some(vec![]);
        e.call(
            0x004bffe0,
            &args![other, weapon_two, 1i32, actor_two, 0u32, 1u32, 0u32],
        );
        assert_eq!(
            calls(&e, TASK_QUEUE_ATTACH_WEAPON),
            vec![vec![0x7777, actor_two, weapon_two]]
        );
        assert!(calls(&e, REFR_UPDATE_WEAPON).is_empty());
        e.mem.set_u32(LOG_FLAG + 8, actor_two);
        e.call_log = Some(vec![]);
        let (third, weapon_three, _, _) = equip_setup(&mut e, 0x28);
        e.call(
            0x004bffe0,
            &args![third, weapon_three, 1i32, actor_two, 0u32, 1u32, 0u32],
        );
        assert_eq!(
            calls(&e, REFR_UPDATE_WEAPON),
            vec![vec![actor_two, weapon_three]]
        );
    }

    #[test]
    fn equip_a_weapon_reuses_the_kept_item_or_drops_a_refused_one() {
        let mut e = equip_engine();
        e.register(vcall_address(0x474), |_, _| returns(0));
        // The process already keeps an item: its lists are replaced.
        let kept = e.mem.alloc(0x20);
        let kept_list = list(&mut e, &[0x1234]);
        e.mem.set_u32(kept, kept_list);
        e.mem.set_u32(kept + 8, 0x4321);
        e.mem.set_u32(LOOKUP_CELL + 0x20, kept);
        e.register(vcall_address(0x148), |e, _| {
            returns(e.mem.u32(LOOKUP_CELL + 0x20))
        });
        let (this, weapon, actor, _) = equip_setup(&mut e, 0x28);
        e.call(
            0x004bffe0,
            &args![this, weapon, 1i32, actor, 0u32, 1u32, 0u32],
        );
        assert_eq!(calls(&e, LIST_CLEAR), vec![vec![kept_list]]);
        assert_eq!(e.mem.u32(kept + 8), weapon);
        assert_eq!(calls(&e, ACTOR_RELOAD_TARGETS), vec![vec![actor, 0]]);
        assert_eq!(calls(&e, REFR_UPDATE_WEAPON), vec![vec![actor, weapon]]);
        assert!(calls(&e, HANDLE_ENTRY_POINT).is_empty());
        // A refused new item is deleted and nothing is attached.
        e.register(vcall_address(0x148), |_, _| returns(0));
        e.register(vcall_address(0x160), |_, _| returns(0));
        let (other, weapon_two, actor_two, _) = equip_setup(&mut e, 0x28);
        e.call_log = Some(vec![]);
        e.call(
            0x004bffe0,
            &args![other, weapon_two, 1i32, actor_two, 0u32, 1u32, 0u32],
        );
        let refused = calls(&e, vcall_address(0x160))[0][1];
        assert_eq!(calls(&e, ITEM_CHANGE_DELETE), vec![vec![refused, 1]]);
        assert!(calls(&e, REFR_UPDATE_WEAPON).is_empty());
        assert!(calls(&e, HANDLE_ENTRY_POINT).is_empty());
    }

    #[test]
    fn equip_a_weapon_on_the_player_or_an_actor_that_asks_not_to_attach() {
        let mut e = equip_engine();
        e.register(vcall_address(0x148), |_, _| returns(0));
        e.register(vcall_address(0x160), |_, _| returns(1));
        // The player: its weapon form is stored, the virtual 0x474 not asked.
        let process = object(&mut e);
        e.mem.set_u32(PLAYER + 0x68, process);
        let animation = e.mem.alloc(0x200);
        e.mem.set_u32(PLAYER + 0x6c, animation);
        let this = inventory(&mut e, &[], 0);
        let weapon = form(&mut e, 0x28, 0, 0);
        e.call(
            0x004bffe0,
            &args![this, weapon, 1i32, PLAYER, 0u32, 1u32, 0u32],
        );
        assert_eq!(calls(&e, PLAYER_STORE_FORM), vec![vec![PLAYER, weapon]]);
        assert!(calls(&e, REFR_UPDATE_WEAPON).is_empty());
        // Another actor whose virtual 0x474 says yes gets nothing.
        e.register(vcall_address(0x474), |_, _| returns(1));
        let (other, weapon_two, actor_two, _) = equip_setup(&mut e, 0x28);
        e.call_log = Some(vec![]);
        e.call(
            0x004bffe0,
            &args![other, weapon_two, 1i32, actor_two, 0u32, 1u32, 0u32],
        );
        assert!(calls(&e, REFR_UPDATE_WEAPON).is_empty());
        assert!(calls(&e, TASK_QUEUE_ATTACH_WEAPON).is_empty());
        assert!(calls(&e, PLAYER_STORE_FORM).is_empty());
        assert_eq!(calls(&e, ACTOR_INIT_LIGHTING).len(), 1);
    }

    #[test]
    fn equip_ammunition_keeps_an_item_in_the_process() {
        let mut e = equip_engine();
        e.register(vcall_address(0x14c), |_, _| returns(0));
        e.register(vcall_address(0x168), |_, _| returns(1));
        e.register(vcall_address(0x474), |_, _| returns(0));
        let (this, ammo, actor, process) = equip_setup(&mut e, 0x29);
        e.call(
            0x004bffe0,
            &args![this, ammo, 9i32, actor, 0u32, 1u32, 0u32],
        );
        let accepted = calls(&e, vcall_address(0x168));
        assert_eq!(accepted.len(), 1);
        let kept = Ptr::<ItemChange>::new(accepted[0][1]);
        assert_eq!(accepted[0][0], process);
        // The count is 0: ammunition items are not counted here.
        assert_eq!(e.get(kept, ItemChange::iNumber), 0);
        assert_eq!(e.get(kept, ItemChange::pContainerObj), Ptr::new(ammo));
        assert_eq!(calls(&e, STORE_NOTIFY_004534F0), vec![vec![actor, ammo]]);
        // Refused: deleted, no notification.
        e.register(vcall_address(0x168), |_, _| returns(0));
        let (other, ammo_two, actor_two, _) = equip_setup(&mut e, 0x29);
        e.call_log = Some(vec![]);
        e.call(
            0x004bffe0,
            &args![other, ammo_two, 1i32, actor_two, 0u32, 1u32, 0u32],
        );
        assert_eq!(calls(&e, ITEM_CHANGE_DELETE).len(), 1);
        assert!(calls(&e, STORE_NOTIFY_004534F0).is_empty());
        // The process already keeps one: its lists are replaced.
        let kept = e.mem.alloc(0x20);
        let kept_list = list(&mut e, &[0x1234]);
        e.mem.set_u32(kept, kept_list);
        e.mem.set_u32(LOOKUP_CELL + 0x20, kept);
        e.register(vcall_address(0x14c), |e, _| {
            returns(e.mem.u32(LOOKUP_CELL + 0x20))
        });
        let (third, ammo_three, actor_three, _) = equip_setup(&mut e, 0x29);
        e.call_log = Some(vec![]);
        e.call(
            0x004bffe0,
            &args![third, ammo_three, 1i32, actor_three, 0u32, 1u32, 0u32],
        );
        assert_eq!(calls(&e, LIST_CLEAR), vec![vec![kept_list]]);
        assert_eq!(e.mem.u32(kept + 8), ammo_three);
        assert_eq!(
            calls(&e, STORE_NOTIFY_004534F0),
            vec![vec![actor_three, ammo_three]]
        );
    }
}
