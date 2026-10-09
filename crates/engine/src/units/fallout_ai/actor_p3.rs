//! `fallout/ai/actor.cpp` (Xbox PDB source unit), part 3: its functions from `00891d70` up to
//! (not including) `008a50d0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::actor`]; anything public there may be used here.
//!
//! This part is a collection of `Actor` methods around inventory (adding
//! and removing items, picking objects up), attacking and animation groups.
//! The game code is unoptimized, so most of it is a chain of tiny accessor
//! calls (`005d43c0` returns `this + 0x44`, the `TESObjectREFR::m_Extra`
//! list; `007af430` returns the base form at `this + 0x20`; `0044ddc0`
//! returns the form at `this + 8` of an inventory entry; `00401170` returns
//! the form type byte at `this + 4`). They are called by address like every
//! other function outside this file.
//!
//! x87 note: floats are computed in `f64` and rounded to `f32` where the code
//! stores a `float`.

#[allow(unused_imports)]
use super::actor::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// The global `PlayerCharacter *` (`thePlayer`).
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
/// The `ProcessLists` singleton (its address is the `this` of `0096f450`).
const PROCESS_LISTS: u32 = 0x011e_0e80;

/// `TESObjectREFR::m_Extra` accessor: returns `this + 0x44`.
const REFR_EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// Form type byte accessor: `*(u8 *)(form + 4)`.
const FORM_TYPE: u32 = 0x0040_1170;
/// `TESObjectREFR` base form accessor: `*(this + 0x20)`.
const REFR_BASE_FORM: u32 = 0x007a_f430;
/// Inventory entry form accessor: `*(this + 8)`.
const ENTRY_FORM: u32 = 0x0044_ddc0;
/// `ExtraDataList::GetContainerChanges`.
const GET_CONTAINER_CHANGES: u32 = 0x0041_8520;
/// `InventoryChanges::GetGoldAmount`.
const INVENTORY_CHANGES_GET_GOLD_AMOUNT: u32 = 0x004c_b320;
/// `InventoryChanges::ContainerHasObjects`.
const CONTAINER_HAS_OBJECTS: u32 = 0x004c_ffe0;
/// `InventoryChanges::RemoveGold`.
const INVENTORY_CHANGES_REMOVE_GOLD: u32 = 0x004c_b4b0;
/// `InventoryChanges::GetInventoryItem`.
const GET_INVENTORY_ITEM: u32 = 0x004d_0650;
/// `bhkWorld::Activate` (cdecl).
const BHK_WORLD_ACTIVATE: u32 = 0x00c6_a270;
/// `GarbageCollector::Add` (cdecl).
const GARBAGE_COLLECTOR_ADD: u32 = 0x0086_7f90;
/// `TESObjectREFR` parent cell accessor: `*(this + 0x40)`.
const REFR_PARENT_CELL: u32 = 0x008d_6f30;
/// `TESObjectCELL::RemoveReference`.
const CELL_REMOVE_REFERENCE: u32 = 0x0054_ca90;
/// `TESObjectREFR::MarkAsPickedUp`.
const MARK_AS_PICKED_UP: u32 = 0x0057_2230;
/// `Actor::GetAnimAction`.
const ACTOR_GET_ANIM_ACTION: u32 = 0x008a_7570;
/// `Actor::IsWeaponDrawn`.
const ACTOR_IS_WEAPON_DRAWN: u32 = 0x008a_16d0;
/// `Actor::SetCurrentTarget`.
const ACTOR_SET_CURRENT_TARGET: u32 = 0x0088_1620;
/// `Actor::GetCurrentPackageTarget`.
const ACTOR_GET_CURRENT_PACKAGE_TARGET: u32 = 0x0088_1650;
/// `MiddleHighProcess::GetSavedAcquireObject` (reads the process-side object
/// the actor is acquiring an item with; the code calls virtual functions on
/// it).
const GET_SAVED_ACQUIRE_OBJECT: u32 = 0x008d_8520;
/// `Actor::QueueEquipObject` (6 words).
const ACTOR_QUEUE_EQUIP_OBJECT: u32 = 0x0088_c650;
/// `Actor::EquipObject` (6 words).
const ACTOR_EQUIP_OBJECT: u32 = 0x0088_c830;
/// `Actor::StealAlarm`.
const ACTOR_STEAL_ALARM: u32 = 0x008b_fa40;
/// `TESObjectWEAP::GetCurrentAmmo`.
const WEAPON_GET_CURRENT_AMMO: u32 = 0x0052_5980;
/// `TESObjectWEAP::GetAmmoRegenRate` (returns a `float` in ST0).
const WEAPON_GET_AMMO_REGEN_RATE: u32 = 0x0070_9430;
/// `ItemChange::HasModEffectActive_ov2` (engine map name; takes a mod effect
/// number).
const ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE: u32 = 0x004b_da70;
/// `ItemChange::GetWorn`.
const ITEM_CHANGE_GET_WORN: u32 = 0x004b_ddd0;
/// Scalar deleting destructor of the inventory entry objects the code gets
/// from `Actor` virtual slot `0x3bc` and `GetInventoryItem` (flag 1 = delete).
const ENTRY_DELETE: u32 = 0x0044_59e0;
/// Next-node accessor of a list: `*(this + 4)`.
const LIST_NEXT: u32 = 0x0072_6070;
/// Returns its `this` (the address of a list node's item slot).
const LIST_ITEM_SLOT: u32 = 0x0068_15c0;
/// Returns the first extra data list of an inventory entry (`0` if none).
const ENTRY_FIRST_EXTRA_LIST: u32 = 0x0055_9450;
/// Clears the list returned by `ProcessLists` (the temporary list's
/// `RemoveAll`).
const LIST_CLEAR: u32 = 0x0047_0470;
/// Scalar deleting destructor of that temporary list (flag 1 = delete).
const LIST_DELETE: u32 = 0x0047_02f0;
/// `0096f450`: the list of actors the process lists find for a form id and
/// actor (`this` = [`PROCESS_LISTS`]).
const PROCESS_LISTS_FIND_ACTORS: u32 = 0x0096_f450;
/// `TESObjectREFR` form id accessor: `*(this + 0x0C)`.
const REFR_FORM_ID: u32 = 0x0084_e3a0;
/// `0041c8d0` (`ExtraDataList::GetReferencePointer`): the reference stored in
/// an extra data list, or 0.
const EXTRA_GET_REFERENCE_POINTER: u32 = 0x0041_c8d0;
/// `ExtraDataList::RemoveOwnership`.
const EXTRA_REMOVE_OWNERSHIP: u32 = 0x0041_aed0;
/// Sets the ownership extra data of a list (takes the owner form).
const EXTRA_SET_OWNERSHIP: u32 = 0x0041_9700;
/// `0041c7f0`: extra-list call that takes an actor (used on the reference of
/// an item that is being picked up).
const EXTRA_SET_ACTOR: u32 = 0x0041_c7f0;
/// `Actor` virtual slot `0x17c`: the item-adding worker (10 words) the
/// item functions end in.
const ACTOR_SLOT_ADD_ITEM: u32 = 0x17c;
/// `Actor` virtual slot `0x22c` (returns a bool, takes one word).
const ACTOR_SLOT_22C: u32 = 0x22c;
/// `Actor` virtual slot `0x3bc`: returns a freshly allocated inventory entry
/// (one word argument, `6`).
const ACTOR_SLOT_3BC: u32 = 0x3bc;
/// `Actor` virtual slot `0x218`.
const ACTOR_SLOT_218: u32 = 0x218;
/// `Actor` virtual slot `0x1e4` (`Actor::GetAnimation` in the `Character`
/// vtable).
const ACTOR_SLOT_GET_ANIMATION: u32 = 0x1e4;
/// `Actor` virtual slot `0x1e8`.
const ACTOR_SLOT_1E8: u32 = 0x1e8;
/// `Actor` virtual slot `0x3ec` (4 words).
const ACTOR_SLOT_3EC: u32 = 0x3ec;
/// `Actor` virtual slot `0x1f4`: returns a pointer to a position (z at +8).
const ACTOR_SLOT_POSITION: u32 = 0x1f4;
/// Process virtual slot `0x148` (`MiddleHighProcess::GetCurrentWeapon` in
/// the high process vtable).
const PROCESS_SLOT_CURRENT_WEAPON: u32 = 0x148;
/// Process virtual slot `0x14c` (`MiddleHighProcess::GetCurrentAmmo` in the
/// high process vtable).
const PROCESS_SLOT_CURRENT_AMMO: u32 = 0x14c;
/// Process virtual slot `0x1cc` (`MiddleHighProcess::ForceWeaponDrawnSheathed`
/// in the high process vtable; `drawn`, two animation objects, the actor).
const PROCESS_SLOT_FORCE_WEAPON_DRAWN: u32 = 0x1cc;

/// `0.0` (`double`).
const ZERO_DOUBLE: u32 = 0x0101_2060;
/// `FLT_MAX` (`float`).
const FLOAT_MAX: u32 = 0x0108_4100;

layout! {
    /// `Actor` (Xbox PDB). Only the fields this part uses, at their PC
    /// offsets (the Xbox PDB's offsets minus `0x10` from `pCurrentProcess`
    /// on, since `TESForm` is `0x18` bytes on PC). The size is the Xbox
    /// PDB's; the PC allocation size has not been checked, so do not
    /// allocate an `Actor` by it.
    pub struct Actor: 0x1c4 {
        /// `pCurrentProcess` (Xbox PDB, +0x78): `BaseProcess *`.
        0x68 pCurrentProcess: Ptr,
        /// `eQueuedattack` (Xbox PDB, +0x120): `ANIM_GROUP_ENUM`.
        0x110 eQueuedattack: u32,
        /// `pInitialPackage` (Xbox PDB, +0x1a8): `TESPackage *`. Also the
        /// package `Actor::InitPackageLocations` remembers.
        0x198 pInitialPackage: u32,
    }
}

fn player(e: &Engine) -> Ptr<Actor> {
    Ptr::new(e.global::<u32>(PLAYER_CHARACTER))
}

fn extra_data_list_of(e: &mut Engine, refr: Ptr) -> Ptr {
    e.call(REFR_EXTRA_DATA_LIST, &args![refr]).ptr()
}

fn form_type(e: &mut Engine, form: Ptr) -> u32 {
    e.call(FORM_TYPE, &args![form]).u32()
}

fn base_form(e: &mut Engine, refr: Ptr) -> Ptr {
    e.call(REFR_BASE_FORM, &args![refr]).ptr()
}

fn entry_form(e: &mut Engine, entry: Ptr) -> Ptr {
    e.call(ENTRY_FORM, &args![entry]).ptr()
}

/// The first extra data list of an inventory entry, `0` when it has none
/// (`if (list) first = *list->item_slot`).
fn first_extra_data_list(e: &mut Engine, entry: Ptr) -> Ptr {
    let mut first = Ptr::NULL;
    if e.call(ENTRY_FIRST_EXTRA_LIST, &args![entry]).u32() != 0 {
        let list = e.call(ENTRY_FIRST_EXTRA_LIST, &args![entry]).ptr::<()>();
        let slot = e.call(LIST_ITEM_SLOT, &args![list]).u32();
        first = Ptr::new(e.mem.u32(slot));
    }
    first
}

/// Deletes an inventory entry object, if there is one.
fn delete_entry(e: &mut Engine, entry: Ptr) {
    if !entry.is_null() {
        e.call(ENTRY_DELETE, &args![entry, 1u32]);
    }
}

/// Walks the actors found by `0096f450` and calls `visit` on each until a
/// null item, then clears and deletes the temporary list.
fn visit_found_actors(e: &mut Engine, list: Ptr, mut visit: impl FnMut(&mut Engine, Ptr)) {
    let mut node = list;
    while !node.is_null() {
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        if e.mem.u32(slot) == 0 {
            break;
        }
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let actor = Ptr::new(e.mem.u32(slot));
        visit(e, actor);
        node = e.call(LIST_NEXT, &args![node]).ptr();
    }
    if !list.is_null() {
        e.call(LIST_CLEAR, &args![list]);
        e.call(LIST_DELETE, &args![list, 1u32]);
    }
}

// Translated from 00891d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetGoldAmount` (Xbox PDB): the gold of the actor's inventory
/// changes, 0 when it has none.
pub fn actor_get_gold_amount(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mut gold = 0;
    let extra = extra_data_list_of(e, this.cast());
    let changes = e.call(GET_CONTAINER_CHANGES, &args![extra]).ptr::<()>();
    if !changes.is_null() {
        gold = e
            .call(INVENTORY_CHANGES_GET_GOLD_AMOUNT, &args![changes])
            .u32();
    }
    gold
}

// Translated from 00891db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::HasObjects` (Xbox PDB): asks the inventory changes
/// (`InventoryChanges::ContainerHasObjects`) whether the actor holds the
/// objects the five arguments describe; false when it has no inventory
/// changes. The arguments are passed on in the order the game swaps them
/// (the fourth and fifth change places).
pub fn actor_has_objects(
    e: &mut Engine,
    this: Ptr<Actor>,
    arg_1: u32,
    arg_2: u32,
    arg_3: u32,
    arg_4: u32,
    arg_5: u32,
) -> bool {
    let extra = extra_data_list_of(e, this.cast());
    let changes = e.call(GET_CONTAINER_CHANGES, &args![extra]).ptr::<()>();
    if changes.is_null() {
        return false;
    }
    e.call(
        CONTAINER_HAS_OBJECTS,
        &args![changes, arg_1, arg_2, arg_3, arg_5, arg_4, this],
    )
    .bool()
}

// Translated from 00892460 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::RemovePickedUpObjectFromWorld` (Xbox PDB), a `cdecl` function
/// without `this`: activates the Havok world of `refr` (virtual slot
/// `0x1d0`), then either marks the reference as picked up (`delete_it`
/// zero) or detaches it (virtual slot `0x1c0`), queues it for garbage
/// collection and removes it from its parent cell.
pub fn actor_remove_picked_up_object_from_world(e: &mut Engine, refr: Ptr, delete_it: u8) {
    let world = e.vcall(refr.addr(), 0x1d0, &args![]).u32();
    e.call(BHK_WORLD_ACTIVATE, &args![world, 1u32, 1u32, 0u32]);
    if delete_it == 0 {
        e.call(MARK_AS_PICKED_UP, &args![refr]);
    } else {
        let cell = e.call(REFR_PARENT_CELL, &args![refr]).ptr::<()>();
        e.vcall(refr.addr(), 0x1c0, &args![]);
        e.call(GARBAGE_COLLECTOR_ADD, &args![refr]);
        if !cell.is_null() {
            e.call(CELL_REMOVE_REFERENCE, &args![cell, refr]);
        }
    }
}

// Translated from 008924e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::PayGoldToActor` (Xbox PDB): removes gold from the inventory
/// changes with `InventoryChanges::RemoveGold`, passing this actor and the
/// two arguments swapped.
pub fn actor_pay_gold_to_actor(e: &mut Engine, this: Ptr<Actor>, arg_1: u32, arg_2: u32) {
    let extra = extra_data_list_of(e, this.cast());
    let changes = e.call(GET_CONTAINER_CHANGES, &args![extra]).ptr::<()>();
    e.call(
        INVENTORY_CHANGES_REMOVE_GOLD,
        &args![changes, this, arg_2, arg_1],
    );
}

// Translated from 00893500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetFatigue` (Xbox PDB): the current value (`float`, in ST0) of
/// actor value `0x16` from the actor-value owner at `this + 0xa4`
/// (virtual slot `0x0c`).
pub fn actor_get_fatigue(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    e.vcall(this.addr() + 0xa4, 0x0c, &args![0x16u32]).f32()
}

/// `current / base` of an actor value of the owner at `this + 0xa4`
/// (slot `0x00` gives the base as an integer, slot `0x0c` the current value),
/// `1.0` when the base is 0.
fn actor_value_percentage(e: &mut Engine, this: Ptr<Actor>, actor_value: u32) -> f32 {
    let base = e.vcall(this.addr() + 0xa4, 0x00, &args![actor_value]).i32();
    let result = if base == 0 {
        1.0
    } else {
        let current = e.vcall(this.addr() + 0xa4, 0x0c, &args![actor_value]).f32();
        current as f64 / base as f64
    };
    result as f32
}

// Translated from 00893530 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetFatiguePercentage` (Xbox PDB): actor value `0x16` current over
/// base, `1.0` when the base is 0.
pub fn actor_get_fatigue_percentage(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    actor_value_percentage(e, this, 0x16)
}

// Translated from 00893590 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetHealthPercentage` (Xbox PDB): actor value `0x10` current over
/// base, `1.0` when the base is 0.
pub fn actor_get_health_percentage(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    actor_value_percentage(e, this, 0x10)
}

/// `TESObjectREFR::GetOwner` (Ghidra name of `00567790`): the owner form of
/// a reference, or 0.
const REFR_GET_OWNER: u32 = 0x0056_7790;
/// `TESObjectREFR::IsAnOwner` (actor, flag).
const REFR_IS_AN_OWNER: u32 = 0x0057_85e0;
/// Accessor on the object `008d8520` returns: `*(this + 0x28)`.
const ACQUIRE_OBJECT_FIELD_28: u32 = 0x0045_cd60;
/// Actor combat flag accessor: `*(u8 *)(this + 0x104)` (`bInCombat`, Xbox PDB
/// +0x114).
const ACTOR_IN_COMBAT: u32 = 0x0049_3bb0;
/// `ExtraDataList::GetLevCreaOriginalBase`.
const EXTRA_GET_LEV_CREA_ORIGINAL_BASE: u32 = 0x0042_16f0;
/// Item count of an extra data list (`i16` in AX; 1 when it has none).
const EXTRA_GET_COUNT: u32 = 0x0041_8770;
/// Weapon type accessor (`*(i8 *)(form + 0xf4)`, sign-extended).
const WEAPON_TYPE: u32 = 0x0044_6390;
/// Form flag accessor: `*(form + 8) & 1`.
const FORM_FLAG_BIT_0: u32 = 0x0046_0250;
/// `TESForm::GetFile`.
const FORM_GET_FILE: u32 = 0x0048_4e60;
/// `TESObjectREFR::GetRefPersists`.
const REFR_GET_REF_PERSISTS: u32 = 0x0056_53d0;
/// Reference extra-flag test taking one word (`2`).
const REFR_TEST_EXTRA_FLAG: u32 = 0x0057_2d30;
/// The pickup worker (reference, count, flag, zero) the pickup functions call
/// on the actor.
const ACTOR_PICK_UP_WORKER: u32 = 0x0057_4b30;
/// `008c7aa0` (no arguments): whether the pickup is reported to the task
/// queue instead of being removed from the world right away.
const PICK_UP_GOES_TO_TASK_QUEUE: u32 = 0x008c_7aa0;
/// Returns the global at `0x011df1a8`, the `this` of [`TES_QUEUE_PICK_UP`].
const GET_TES: u32 = 0x0045_37b0;
/// `0087afa0` (task queue interface): takes the object [`GET_TES`] returns as
/// `this`, a reference and a flag (stack arguments, `RET 8`).
const TES_QUEUE_PICK_UP: u32 = 0x0087_afa0;
/// `008aded0`: actor call taking a form and two flags.
const ACTOR_FORM_WORKER: u32 = 0x008a_ded0;
/// `008bc9d0`: actor call taking a form and a flag; true when the form may be
/// equipped.
const ACTOR_CAN_EQUIP: u32 = 0x008b_c9d0;

/// `Actor` slots `0x1e4` and `0x1e8` give the two objects the process needs
/// to force the weapon drawn (`drawn` 1) or sheathed (0); the process is read
/// after those calls.
fn force_weapon_drawn(e: &mut Engine, this: Ptr<Actor>, drawn: u32) {
    let biped_a = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .u32();
    let biped_b = e.vcall(this.addr(), ACTOR_SLOT_1E8, &args![]).u32();
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(
        process.addr(),
        PROCESS_SLOT_FORCE_WEAPON_DRAWN,
        &args![drawn, biped_b, biped_a, this],
    );
}

/// Whether `ammo_form` is the ammunition of the process's current weapon
/// (`TESObjectWEAP::GetCurrentAmmo` of the current weapon's form).
fn current_weapon_takes_ammo(e: &mut Engine, this: Ptr<Actor>, ammo_form: Ptr) -> bool {
    let process = e.get(this, Actor::pCurrentProcess);
    let weapon = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
        .ptr::<()>();
    let weapon_form = entry_form(e, weapon);
    let ammo = e
        .call(WEAPON_GET_CURRENT_AMMO, &args![weapon_form, this])
        .ptr::<()>();
    ammo_form == ammo
}

/// The tail both pickup functions share once the picked-up form is the
/// current weapon's ammunition: when the weapon has no ammunition regeneration
/// (`TESObjectWEAP::GetAmmoRegenRate` of the weapon with mod effect 6, at
/// most 0.0), the weapon is equipped again through `Actor` slot `0x3ec`.
fn requip_weapon_if_no_ammo_regen(e: &mut Engine, this: Ptr<Actor>) {
    let process = e.get(this, Actor::pCurrentProcess);
    let item = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
        .ptr::<()>();
    let mod_active = e
        .call(ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, &args![item, 6u32])
        .u8();
    let item = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
        .ptr::<()>();
    let weapon_form = entry_form(e, item);
    let rate = e
        .call(WEAPON_GET_AMMO_REGEN_RATE, &args![weapon_form, mod_active])
        .f32() as f64;
    let zero: f64 = e.global(ZERO_DOUBLE);
    if rate <= zero {
        let item = e
            .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .ptr::<()>();
        let mod_active = e
            .call(ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, &args![item, 2u32])
            .u8();
        let drawn = e.call(ACTOR_IS_WEAPON_DRAWN, &args![this]).u8();
        let item = e
            .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .ptr::<()>();
        let weapon_form = entry_form(e, item);
        e.vcall(
            this.addr(),
            ACTOR_SLOT_3EC,
            &args![weapon_form, drawn, mod_active, 0u32],
        );
    }
}

// Translated from 00891e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks the reference `refr` up into the actor's inventory. Everyone who has
/// it as a target is retargeted, ownership is fixed (a steal alarm if the
/// actor is not an owner), a drawn weapon of the same kind is sheathed around
/// the pickup (`Actor` virtual slot `0x218` gates the ownership part), the
/// pickup worker `00574b30` runs, ammunition is re-equipped, and the
/// reference leaves the world (`Actor::RemovePickedUpObjectFromWorld`, or the
/// task queue).
///
/// `count` and `flag` are stack arguments the code reads and, for a form of
/// type `0x29`, overwrites. The compiler's exception-unwinding frame is not
/// translated.
pub fn fn_00891e00(e: &mut Engine, this: Ptr<Actor>, refr: Ptr, count: u32, flag: u8) {
    let mut count = count;
    let mut flag = flag;
    let player = player(e);

    e.vcall(refr.addr(), 0x4c, &args![0u32]);
    let base = if refr.is_null() {
        Ptr::NULL
    } else {
        base_form(e, refr)
    };
    let base_type = if base.is_null() {
        0
    } else {
        form_type(e, base)
    };

    // Everyone who has this reference as their package target stops
    // targeting it.
    let form_id = e.call(REFR_FORM_ID, &args![refr]).u32();
    let list = e
        .call(
            PROCESS_LISTS_FIND_ACTORS,
            &args![PROCESS_LISTS, form_id, this],
        )
        .ptr::<()>();
    e.call(ACTOR_FORM_WORKER, &args![this, base, 1u32, 0u32]);
    visit_found_actors(e, list, |e, target| {
        let on_this_reference = !target.is_null()
            && e.call(ACTOR_GET_CURRENT_PACKAGE_TARGET, &args![target])
                .ptr::<()>()
                == refr;
        if on_this_reference {
            e.call(ACTOR_SET_CURRENT_TARGET, &args![target, this]);
        } else {
            e.call(ACTOR_SET_CURRENT_TARGET, &args![target, 0u32]);
        }
    });

    // Ownership.
    if e.vcall(this.addr(), ACTOR_SLOT_218, &args![]).bool() {
        let mut remove_ownership = true;
        let owner = e.call(REFR_GET_OWNER, &args![refr]).u32();
        if owner != 0 && !e.call(REFR_IS_AN_OWNER, &args![refr, this, 1u32]).bool() {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
            if e.call(ACQUIRE_OBJECT_FIELD_28, &args![acquire]).u32() == 0 {
                let owner = e.call(REFR_GET_OWNER, &args![refr]).u32();
                e.call(
                    ACTOR_STEAL_ALARM,
                    &args![this, refr, base, count, 0u32, owner],
                );
                if e.call(ACTOR_IN_COMBAT, &args![this]).bool() && this != player {
                    let extra = extra_data_list_of(e, refr);
                    e.call(EXTRA_REMOVE_OWNERSHIP, &args![extra]);
                } else {
                    let extra = extra_data_list_of(e, refr);
                    let mut owner_form = e
                        .call(EXTRA_GET_LEV_CREA_ORIGINAL_BASE, &args![extra])
                        .ptr::<()>();
                    if owner_form.is_null() {
                        owner_form = base_form(e, refr);
                    }
                    let extra = extra_data_list_of(e, refr);
                    e.call(EXTRA_SET_OWNERSHIP, &args![extra, owner_form]);
                }
                remove_ownership = false;
            }
        }
        if remove_ownership {
            let extra = extra_data_list_of(e, refr);
            e.call(EXTRA_REMOVE_OWNERSHIP, &args![extra]);
        }
    }

    // A drawn weapon is sheathed around the pickup.
    let mut redraw = false;
    let mut weapon_form = Ptr::NULL;
    let picked_base = base_form(e, refr);
    let picked_type = form_type(e, picked_base);
    if picked_type == 0x28 {
        weapon_form = base_form(e, refr);
        let weapon_type = e.call(WEAPON_TYPE, &args![weapon_form]).i32();
        if weapon_type == 10 || weapon_type == 11 {
            let process = e.get(this, Actor::pCurrentProcess);
            if e.vcall(process.addr(), PROCESS_SLOT_CURRENT_AMMO, &args![])
                .u32()
                != 0
            {
                let current = e
                    .vcall(process.addr(), PROCESS_SLOT_CURRENT_AMMO, &args![])
                    .ptr::<()>();
                let current_form = entry_form(e, current);
                if weapon_form == current_form && e.call(ACTOR_IS_WEAPON_DRAWN, &args![this]).bool()
                {
                    force_weapon_drawn(e, this, 0);
                    redraw = true;
                }
            }
        }
    } else if picked_type == 0x29 {
        flag = 0;
        let extra = extra_data_list_of(e, refr);
        let stack_count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16;
        if stack_count < 1 {
            let base = base_form(e, refr);
            count = e.call(FORM_TYPE, &args![base.byte_add(0x80)]).u32();
        }
    }

    // The pickup itself.
    let mut removed_from_world = false;
    if e.call(FORM_FLAG_BIT_0, &args![refr]).bool()
        || e.call(FORM_GET_FILE, &args![refr, 0xffff_ffffu32]).u32() != 0
        || e.call(REFR_GET_REF_PERSISTS, &args![refr]).bool()
    {
        e.call(ACTOR_PICK_UP_WORKER, &args![this, refr, count, flag, 0u32]);
    } else if e.call(REFR_TEST_EXTRA_FLAG, &args![refr, 2u32]).bool()
        || e.vcall(refr.addr(), 0x224, &args![]).bool()
    {
        e.call(ACTOR_PICK_UP_WORKER, &args![this, refr, count, 0u32, 0u32]);
    } else {
        e.call(ACTOR_PICK_UP_WORKER, &args![this, refr, count, 0u32, 0u32]);
        removed_from_world = true;
    }

    if redraw {
        e.vcall(
            this.addr(),
            ACTOR_SLOT_3EC,
            &args![weapon_form, 0u32, 0u32, 0u32],
        );
        force_weapon_drawn(e, this, 1);
    }

    // Ammunition regeneration: skipped when the current ammunition item has a
    // successor.
    let process = e.get(this, Actor::pCurrentProcess);
    let mut skip_ammo = false;
    if e.vcall(process.addr(), PROCESS_SLOT_CURRENT_AMMO, &args![])
        .u32()
        != 0
    {
        let ammo_item = e
            .vcall(process.addr(), PROCESS_SLOT_CURRENT_AMMO, &args![])
            .ptr::<()>();
        skip_ammo = e.call(LIST_NEXT, &args![ammo_item]).u32() != 0;
    }
    if !skip_ammo {
        let weapon = e
            .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .u32();
        if weapon != 0 && current_weapon_takes_ammo(e, this, base) {
            requip_weapon_if_no_ammo_regen(e, this);
        }
    }

    // Equip the ammunition entry the actor now holds.
    if !e.call(ACTOR_IN_COMBAT, &args![this]).bool() {
        let mut entry = Ptr::NULL;
        if base_type == 0x28 {
            entry = e.vcall(this.addr(), ACTOR_SLOT_3BC, &args![6u32]).ptr();
        }
        if !entry.is_null() {
            let form = entry_form(e, entry);
            if e.call(ACTOR_CAN_EQUIP, &args![this, form, 1u32]).bool() {
                let form = entry_form(e, entry);
                e.call(
                    ACTOR_QUEUE_EQUIP_OBJECT,
                    &args![this, form, 1u32, 0u32, 1u32, 0u32, 1u32],
                );
            }
            delete_entry(e, entry);
        }
    }

    e.vcall(refr.addr(), 0xc4, &args![1u32]);
    let removed = u32::from(removed_from_world);
    if e.call(PICK_UP_GOES_TO_TASK_QUEUE, &args![]).bool() {
        let tes = e.call(GET_TES, &args![]).ptr::<()>();
        e.call(TES_QUEUE_PICK_UP, &args![tes, refr, removed]);
    } else {
        actor_remove_picked_up_object_from_world(e, refr, removed as u8);
    }
}

/// `PlayerCharacter::AddNote` (item, flag).
const PLAYER_ADD_NOTE: u32 = 0x0096_6a70;
/// Accessor of the ownership extra data (type `0x21`): the owner form, or 0.
const EXTRA_GET_OWNER_FORM: u32 = 0x0041_8660;
/// Interface test taking no arguments; non-zero while a blocking menu is up.
const INTERFACE_BLOCKING_MENU: u32 = 0x0070_5020;
/// Interface menu mode accessor (no arguments); `0` or `3` allow the
/// automatic equip.
const INTERFACE_MENU_MODE: u32 = 0x0070_5040;
/// `Interface::IsInMenuMode`.
const INTERFACE_IS_IN_MENU_MODE: u32 = 0x0070_2360;
/// `MiddleHighProcess::GetForceNextUpdate` (engine map name).
const PROCESS_GET_FORCE_NEXT_UPDATE: u32 = 0x0056_6950;
/// `MobileObject::IsInDialoguewithPlayer`.
const IS_IN_DIALOGUE_WITH_PLAYER: u32 = 0x0093_3840;
/// `ExtraDataList::GetWorn` (flag).
const EXTRA_GET_WORN: u32 = 0x0041_8ab0;
/// The item-adding worker on the actor (item, extra data list, count).
const ACTOR_ADD_ITEM_WORKER: u32 = 0x0057_4fa0;
/// Accessor on an animation object: `*(this + 0xd0)` as a `float`.
const ANIMATION_FLOAT_D0: u32 = 0x0045_3700;
/// Animation lookup of the sequence for a slot (`4`), `0` when none.
const ANIMATION_GET_SEQUENCE: u32 = 0x0049_1040;
/// `BSAnimGroupSequence::GetScaledTime` (takes a `float`, returns one).
const SEQUENCE_GET_SCALED_TIME: u32 = 0x004e_ec60;
/// Accessor on a sequence: `*(this + 0x30)` as a `float`.
const SEQUENCE_FLOAT_30: u32 = 0x0050_8100;
/// Accessor on a sequence: `*(this + 0x48)` as a `float`.
const SEQUENCE_FLOAT_48: u32 = 0x0063_9aa0;
/// `Animation::ZeroGlobalTransform` (engine map name).
const ANIMATION_ZERO_GLOBAL_TRANSFORM: u32 = 0x0048_f7f0;
/// `005f2b60`: animation worker taking the actor, two `float`s and the
/// sequence (`RET 0x10`).
const ANIMATION_APPLY_TIME_RANGE: u32 = 0x005f_2b60;
/// Message text source (`this` = [`MESSAGE_SOURCE`], no arguments).
const MESSAGE_TEXT: u32 = 0x004c_69f0;
/// The `this` of [`MESSAGE_TEXT`].
const MESSAGE_SOURCE: u32 = 0x011d_3264;
/// `007052f0` (cdecl, 6 words): shows a message: text, 0, icon path, 0,
/// duration (`float`), 0.
const SHOW_MESSAGE: u32 = 0x0070_52f0;
/// `"Interface\Icons\Message Icons\glow_message_vaultboy_sad.dds"`.
const MESSAGE_ICON_PATH: u32 = 0x0102_08a0;
/// The message duration, `2.0f`.
const MESSAGE_DURATION: u32 = 0x0101_62c0;
/// `007043c0`: no arguments; true when the item may be added into a cell
/// directly.
const ADD_INTO_CELL_ENABLED: u32 = 0x0070_43c0;
/// `TESObjectCELL` call without argument that returns the cell to add into.
const CELL_GET_ADD_TARGET: u32 = 0x0055_1110;
/// The same with the actor as argument.
const CELL_FIND_ADD_TARGET: u32 = 0x0055_1180;
/// `TESObjectCELL::GetOwner`.
const CELL_GET_OWNER: u32 = 0x0054_6a40;
/// `TESValueForm::GetFormValue` (cdecl, one argument).
const VALUE_FORM_GET_FORM_VALUE: u32 = 0x0048_e8a0;
/// The setting-like object whose `float` the owner-value check compares.
const OWNER_VALUE_SETTING: u32 = 0x011d_0f78;
/// Returns a pointer to the float of the setting object (`this` =
/// [`OWNER_VALUE_SETTING`]).
const SETTING_GET_VALUE_POINTER: u32 = 0x0040_3e20;
/// `008a6840`: actor call taking a flag (here `0`).
const ACTOR_FLAG_WORKER: u32 = 0x008a_6840;
/// `TESObjectWEAP::GetFormClipRounds` (takes a flag).
const WEAPON_GET_FORM_CLIP_ROUNDS: u32 = 0x004f_e160;
/// `00944460` (cdecl, two arguments: 0 and the clip rounds): ammunition
/// count to give.
const AMMO_COUNT_FOR_CLIP: u32 = 0x0094_4460;
/// Sets the ammunition of an extra data list (ammo form, count).
const EXTRA_SET_AMMO: u32 = 0x0042_eb60;
/// `bhkNiCollisionObject::ResetSim` (cdecl, world and flag).
const RESET_SIM: u32 = 0x00c6_bd00;
/// `Actor` virtual slot `0x3c4` (no arguments).
const ACTOR_SLOT_3C4: u32 = 0x3c4;
/// `Actor` virtual slot `0x428` (returns an object that takes `1`).
const ACTOR_SLOT_428: u32 = 0x428;
/// `009819d0`: call on the object `Actor` slot `0x428` returns (combat
/// controller, by its unit).
const COMBAT_CONTROLLER_CALL: u32 = 0x0098_19d0;

// Translated from 00892520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `item` (with the extra data list `extra`) to the actor (this is the
/// function in `Actor` virtual slot `0x190` of the `Character` vtable). A note (form
/// type `0x31`) given to the player goes to `PlayerCharacter::AddNote` and
/// the extra data list is destroyed (virtual slot 0, flag 1). Otherwise:
/// ownership that is the player's is removed, everybody who had the
/// reference in the list as package target is retargeted, the item worker
/// `00574fa0` runs, a weapon is equipped for a non-player actor, ammunition is
/// re-equipped, and the combat controller (`Actor` slot `0x428`) is told.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_00892520(e: &mut Engine, this: Ptr<Actor>, item: Ptr, extra: Ptr, arg_3: u32) {
    let player = player(e);
    if form_type(e, item) == 0x31 && this == player {
        e.call(PLAYER_ADD_NOTE, &args![player, item, 1u32]);
        if !extra.is_null() {
            e.vcall(extra.addr(), 0, &args![1u32]);
        }
        return;
    }

    if !extra.is_null() && this == player {
        let owner = e.call(EXTRA_GET_OWNER_FORM, &args![extra]).u32();
        let player_base = base_form(e, player.cast());
        if owner == player_base.addr() {
            e.call(EXTRA_REMOVE_OWNERSHIP, &args![extra]);
        }
    }

    if !extra.is_null() {
        let refr = e
            .call(EXTRA_GET_REFERENCE_POINTER, &args![extra])
            .ptr::<()>();
        if !refr.is_null() {
            let form_id = e.call(REFR_FORM_ID, &args![refr]).u32();
            let reference_extra = extra_data_list_of(e, refr);
            e.call(EXTRA_SET_ACTOR, &args![reference_extra, this]);
            e.vcall(refr.addr(), 0x48, &args![0x400u32]);
            let list = e
                .call(
                    PROCESS_LISTS_FIND_ACTORS,
                    &args![PROCESS_LISTS, form_id, this],
                )
                .ptr::<()>();
            visit_found_actors(e, list, |e, target| {
                let has_package_target = !target.is_null()
                    && e.call(ACTOR_GET_CURRENT_PACKAGE_TARGET, &args![target])
                        .u32()
                        != 0;
                if has_package_target {
                    e.call(ACTOR_SET_CURRENT_TARGET, &args![target, this]);
                } else {
                    e.call(ACTOR_SET_CURRENT_TARGET, &args![target, 0u32]);
                }
            });
        }
    }

    e.call(ACTOR_ADD_ITEM_WORKER, &args![this, item, extra, arg_3]);

    // A non-player actor equips a weapon it was given.
    if e.vcall(this.addr(), ACTOR_SLOT_218, &args![]).bool()
        && player != this
        && !e.call(INTERFACE_BLOCKING_MENU, &args![]).bool()
        && (e.call(INTERFACE_MENU_MODE, &args![]).u32() == 0
            || e.call(INTERFACE_MENU_MODE, &args![]).u32() == 3)
    {
        let mut entry = Ptr::NULL;
        if form_type(e, item) == 0x28 {
            entry = e.vcall(this.addr(), ACTOR_SLOT_3BC, &args![6u32]).ptr();
        }
        if !entry.is_null() {
            if !e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool()
                && !e.call(ACTOR_IN_COMBAT, &args![this]).bool()
            {
                let form = entry_form(e, entry);
                let equip_args = args![this, form, 1u32, 0u32, 1u32, 0u32, 1u32];
                if e.call(INTERFACE_IS_IN_MENU_MODE, &args![]).bool()
                    && e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![this]).bool()
                {
                    e.call(ACTOR_EQUIP_OBJECT, &equip_args);
                } else {
                    e.call(ACTOR_QUEUE_EQUIP_OBJECT, &equip_args);
                }
            }
            delete_entry(e, entry);
        }
    }

    // Ammunition regeneration.
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null()
        && e.vcall(process.addr(), PROCESS_SLOT_CURRENT_AMMO, &args![])
            .u32()
            == 0
        && e.vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .u32()
            != 0
        && current_weapon_takes_ammo(e, this, item)
        && (this == player || !e.call(IS_IN_DIALOGUE_WITH_PLAYER, &args![this]).bool())
    {
        requip_weapon_if_no_ammo_regen(e, this);
    }

    let combat_controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).ptr::<()>();
    if !combat_controller.is_null() {
        e.call(COMBAT_CONTROLLER_CALL, &args![combat_controller, 1u32]);
    }
}

// Translated from 008929a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Gives the actor `item` (extra data list `extra`, `count`, two more words
/// passed on) through `Actor` virtual slot `0x17c` and returns the
/// reference it made, or null. The player cannot put on worn clothes while
/// an animation action is running (a message is shown and nothing is
/// added). A worn weapon's animation time range is adjusted. Items for a
/// cell are added into it. Ownership of the item is cleared or set to the
/// player's depending on the cell's owner and the item's value, a non-player
/// weapon gets its ammunition, a non-player actor equips what it was given,
/// and the reference's Havok simulation is reset.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_008929a0(
    e: &mut Engine,
    this: Ptr<Actor>,
    item: Ptr,
    extra: Ptr,
    count: u32,
    arg_4: u32,
    arg_5: u32,
) -> Ptr {
    let player = player(e);
    let worn = !extra.is_null() && e.call(EXTRA_GET_WORN, &args![extra, 0u32]).bool();

    if this == player && e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() != -1 && worn {
        let text = e.call(MESSAGE_TEXT, &args![MESSAGE_SOURCE]).u32();
        let duration: f32 = e.global(MESSAGE_DURATION);
        e.call(
            SHOW_MESSAGE,
            &args![text, 0u32, MESSAGE_ICON_PATH, 0u32, duration, 0u32],
        );
        return Ptr::NULL;
    }

    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    if !acquire.is_null() {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
        e.vcall(acquire.addr(), 0x17c, &args![item, extra]);
    }
    let animation = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .ptr::<()>();

    // A worn weapon: adjust the time range of its animation.
    if worn
        && form_type(e, item) == 0x28
        && !animation.is_null()
        && e.call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
            .u32()
            != 0
    {
        let sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
            .ptr::<()>();
        let animation_time = e.call(ANIMATION_FLOAT_D0, &args![animation]).f32();
        let scaled_time = e
            .call(SEQUENCE_GET_SCALED_TIME, &args![sequence, animation_time])
            .f32();
        let float_max: f32 = e.global(FLOAT_MAX);
        if scaled_time as f64 != -(float_max as f64) {
            let animation_time = e.call(ANIMATION_FLOAT_D0, &args![animation]).f32() as f64;
            let start = e.call(SEQUENCE_FLOAT_30, &args![sequence]).f32() as f64;
            let length = e.call(SEQUENCE_FLOAT_48, &args![sequence]).f32() as f64;
            let end = ((start + animation_time) + length) as f32;
            let transform = e
                .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                .ptr::<()>();
            e.call(
                ANIMATION_APPLY_TIME_RANGE,
                &args![transform, this, scaled_time, end, sequence],
            );
        }
    }

    // Straight into a cell, when the game allows it.
    let cell = e.call(REFR_PARENT_CELL, &args![this]).ptr::<()>();
    if e.call(ADD_INTO_CELL_ENABLED, &args![]).bool() && !cell.is_null() {
        let mut target = e.call(CELL_GET_ADD_TARGET, &args![cell]).ptr::<()>();
        if target.is_null() {
            target = e.call(CELL_FIND_ADD_TARGET, &args![cell, this]).ptr();
        }
        if !target.is_null() {
            e.vcall(
                this.addr(),
                ACTOR_SLOT_ADD_ITEM,
                &args![item, extra, count, 0u32, 0u32, target, 0u32, 0u32, 1u32, 0u32],
            );
            return Ptr::NULL;
        }
    }

    // Ownership.
    let mut fix_ownership = true;
    if !extra.is_null()
        && e.call(EXTRA_GET_OWNER_FORM, &args![extra]).u32() != 0
        && (e.call(EXTRA_GET_OWNER_FORM, &args![extra]).u32() == 0
            || !e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool())
    {
        fix_ownership = false;
    }
    if fix_ownership {
        if !extra.is_null() && e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
            e.call(EXTRA_REMOVE_OWNERSHIP, &args![extra]);
        }
        let cell = e.call(REFR_PARENT_CELL, &args![this]).ptr::<()>();
        if !cell.is_null() {
            let cell = e.call(REFR_PARENT_CELL, &args![this]).ptr::<()>();
            if e.call(CELL_GET_OWNER, &args![cell]).u32() != 0 {
                let value = e.call(VALUE_FORM_GET_FORM_VALUE, &args![item]).i32() as f64;
                let threshold_pointer = e
                    .call(SETTING_GET_VALUE_POINTER, &args![OWNER_VALUE_SETTING])
                    .u32();
                let threshold = e.mem.f32(threshold_pointer) as f64;
                if threshold >= value || e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
                    let player_base = base_form(e, player.cast());
                    e.call(EXTRA_SET_OWNERSHIP, &args![extra, player_base]);
                }
            }
        }
    }

    let result = e
        .vcall(
            this.addr(),
            ACTOR_SLOT_ADD_ITEM,
            &args![item, extra, count, 0u32, 1u32, 0u32, arg_4, arg_5, 1u32, 0u32],
        )
        .ptr::<()>();

    // A weapon given to an NPC gets its ammunition.
    if !result.is_null() && form_type(e, item) == 0x28 && this != player {
        e.call(ACTOR_FLAG_WORKER, &args![this, 0u32]);
        let ammo = e
            .call(WEAPON_GET_CURRENT_AMMO, &args![item, this])
            .ptr::<()>();
        if !ammo.is_null() {
            let clip_rounds = e
                .call(WEAPON_GET_FORM_CLIP_ROUNDS, &args![item, 0u32])
                .u32();
            let rounds = e.call(AMMO_COUNT_FOR_CLIP, &args![0u32, clip_rounds]).i32();
            if rounds > 0 {
                let ammo = e
                    .call(WEAPON_GET_CURRENT_AMMO, &args![item, this])
                    .ptr::<()>();
                let result_extra = extra_data_list_of(e, result);
                e.call(EXTRA_SET_AMMO, &args![result_extra, ammo, rounds]);
                e.vcall(result.addr(), 0x48, &args![0x800u32]);
            }
        }
    }

    // A non-player actor equips what it was given.
    let mut entry = Ptr::NULL;
    if this != player
        && e.vcall(this.addr(), ACTOR_SLOT_218, &args![]).bool()
        && !e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool()
    {
        let item_type = form_type(e, item);
        if item_type == 0x18 {
            e.vcall(this.addr(), ACTOR_SLOT_3C4, &args![]);
        } else if item_type == 0x28 && !e.call(ACTOR_IN_COMBAT, &args![this]).bool() {
            entry = e.vcall(this.addr(), ACTOR_SLOT_3BC, &args![6u32]).ptr();
        }
    }
    if !entry.is_null() {
        let first = first_extra_data_list(e, entry);
        let form = entry_form(e, entry);
        e.call(
            ACTOR_QUEUE_EQUIP_OBJECT,
            &args![this, form, 1u32, first, 1u32, 0u32, 1u32],
        );
        delete_entry(e, entry);
    }

    if !result.is_null() {
        let world = e.vcall(result.addr(), 0x1d0, &args![]).u32();
        e.call(RESET_SIM, &args![world, 1u32]);
        if !e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
            let result_base = base_form(e, result);
            e.call(ACTOR_FORM_WORKER, &args![this, result_base, 0u32, 0u32]);
        }
    }
    result
}

// Translated from 00892e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Takes `count` copies of `item` (filter `filter`) out of the actor's
/// inventory changes one at a time and drops them with `Actor` slot `0x17c`
/// at the reference `target` (which also gets the actor as its extra-list
/// actor), then, for a non-player actor, re-equips ammunition and plays the
/// use-item idle on `target`: the idle manager is told the used item, that it
/// activates, and whether `target` is above the actor's eye level. Always
/// returns false. `_unused_2` is a word the code never reads.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_00892e90(
    e: &mut Engine,
    this: Ptr<Actor>,
    item: Ptr,
    _unused_2: u32,
    target: Ptr,
    count: u32,
    filter: u32,
) -> bool {
    let player = player(e);
    let extra = extra_data_list_of(e, this.cast());
    let changes = e.call(GET_CONTAINER_CHANGES, &args![extra]).ptr::<()>();
    let mut remaining = count;
    if remaining == 0 {
        remaining = 1;
    }
    if !changes.is_null() {
        while remaining != 0 {
            let entry = e
                .call(GET_INVENTORY_ITEM, &args![changes, item, filter])
                .ptr::<()>();
            if !entry.is_null() {
                let first = first_extra_data_list(e, entry);
                if !first.is_null() {
                    let refr = e
                        .call(EXTRA_GET_REFERENCE_POINTER, &args![first])
                        .ptr::<()>();
                    if !refr.is_null() {
                        let refr_extra = extra_data_list_of(e, refr);
                        e.call(EXTRA_SET_ACTOR, &args![refr_extra, target]);
                        e.vcall(refr.addr(), 0x48, &args![0x400u32]);
                    }
                }
                e.vcall(
                    this.addr(),
                    ACTOR_SLOT_ADD_ITEM,
                    &args![item, first, remaining, 0u32, 0u32, target, 0u32, 0u32, 1u32, 0u32],
                );
                remaining -= 1;
            } else {
                remaining = 0;
            }
            delete_entry(e, entry);
        }
        if e.call(CHANGES_IS_EMPTY, &args![changes]).bool() {
            let extra = extra_data_list_of(e, this.cast());
            e.call(EXTRA_REMOVE_CONTAINER_CHANGES, &args![extra]);
        }
    }

    if this == player {
        e.call(INTERFACE_REFRESH_INVENTORY, &args![]);
    } else {
        let entry = e
            .vcall(this.addr(), ACTOR_SLOT_3BC, &args![6u32])
            .ptr::<()>();
        if !entry.is_null() && !e.call(ITEM_CHANGE_GET_WORN, &args![entry, 0u32]).bool() {
            let first = first_extra_data_list(e, entry);
            let form = entry_form(e, entry);
            e.call(
                ACTOR_QUEUE_EQUIP_OBJECT,
                &args![this, form, 1u32, first, 1u32, 0u32, 1u32],
            );
            delete_entry(e, entry);
        }
    }

    if this != player {
        let target_base = base_form(e, target);
        e.call(IDLE_MANAGER_SET_USED_ITEM, &args![target_base]);
        e.call(IDLE_MANAGER_SET_USED_ITEM_ACTIVATE, &args![1u32]);
        let eye_level = e.call(ACTOR_GET_EYE_LEVEL, &args![this]).f32() as f64;
        let actor_position = e.vcall(this.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
        let actor_top = (e.mem.f32(actor_position + 8) as f64 + eye_level) as f32;
        let target_position = e.vcall(target.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
        let target_z = e.mem.f32(target_position + 8);
        let target_is_above =
            e.vcall(target.addr(), ACTOR_SLOT_218, &args![]).bool() || actor_top < target_z;
        e.call(
            IDLE_MANAGER_SET_USED_ITEM_LEVEL,
            &args![u32::from(target_is_above)],
        );
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
        e.vcall(
            acquire.addr(),
            0x44,
            &args![this, 0u32, 2u32, 1u32, 0u32, 1u32],
        );
        e.call(IDLE_MANAGER_SET_USED_ITEM, &args![0u32]);
        e.call(IDLE_MANAGER_SET_USED_ITEM_ACTIVATE, &args![0u32]);
        e.call(IDLE_MANAGER_SET_USED_ITEM_LEVEL, &args![0xffff_ffffu32]);
    }
    false
}

/// `ExtraDataList` test: true when the inventory changes hold nothing.
const CHANGES_IS_EMPTY: u32 = 0x0042_cde0;
/// Removes the container changes from an extra data list.
const EXTRA_REMOVE_CONTAINER_CHANGES: u32 = 0x0041_aeb0;
/// Interface call without arguments the player branch of `fn_00892e90` ends
/// in (`00704af0`).
const INTERFACE_REFRESH_INVENTORY: u32 = 0x0070_4af0;
/// `TESIdleManager::SetUsedItem` (cdecl, one argument).
const IDLE_MANAGER_SET_USED_ITEM: u32 = 0x0060_0900;
/// `TESIdleManager::SetUsedItemActivate` (cdecl, one argument).
const IDLE_MANAGER_SET_USED_ITEM_ACTIVATE: u32 = 0x0060_0940;
/// `TESIdleManager::SetUsedItemLevel` (cdecl, one argument).
const IDLE_MANAGER_SET_USED_ITEM_LEVEL: u32 = 0x0060_0920;
/// `Actor::GetEyeLevel` (returns a `float` in ST0).
const ACTOR_GET_EYE_LEVEL: u32 = 0x008b_e940;

/// `Actor` virtual slot `0x234`.
const ACTOR_SLOT_234: u32 = 0x234;
/// `Actor` virtual slot `0x214`.
const ACTOR_SLOT_214: u32 = 0x214;
/// Reference test: `*(this + 0x1ac) == 9`.
const REFR_FIELD_1AC_IS_9: u32 = 0x0057_9670;
/// Extra-data-objects test (`00437bd0`).
const EXTRA_OBJECTS_TEST: u32 = 0x0043_7bd0;
/// Cell test: `*(u8 *)(this + 0x24) & 1`.
const CELL_FLAG_24_BIT_0: u32 = 0x0042_5fd0;
/// `fabs`-like float function (cdecl, one `float` argument, result in ST0).
const FLOAT_ABS: u32 = 0x0040_8840;
/// `90.0` (`double`).
const NINETY: u32 = 0x0106_6808;
/// `Actor::ShouldSkipFallOutBehavior`.
const ACTOR_SHOULD_SKIP_FALLOUT_BEHAVIOR: u32 = 0x008a_78f0;
/// `MobileObject::GetCurrentPackage`.
const MOBILE_OBJECT_GET_CURRENT_PACKAGE: u32 = 0x0093_44a0;
/// `TESPackage::IsInterruptPackage`.
const PACKAGE_IS_INTERRUPT_PACKAGE: u32 = 0x0067_8610;
/// Process virtual slot `0x30c` (a bool).
const PROCESS_SLOT_30C: u32 = 0x30c;
/// Process virtual slot `0x22c` without arguments (the process's current
/// package in `Actor::InitPackageLocations`).
const PROCESS_SLOT_22C: u32 = 0x22c;
/// Process virtual slot `0x1e0` (returns a `float`).
const PROCESS_SLOT_1E0: u32 = 0x1e0;

// Translated from 00893190 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ShouldTalkTo` (Xbox PDB): whether the actor may start a dialogue
/// with `other`. False when the process has no acquire object; when the
/// actor is in an interior cell (`00425fd0`) and `other` is more than `90.0`
/// higher or lower; when the process object holds something and the actor
/// should skip its fall-out behaviour; when the actor's state (`Actor` slot
/// `0x214`) is neither 4 nor 0; or when its package is an interrupt
/// package. Otherwise true if the process object's slot `0x1e0` `float` is
/// at most 0.
pub fn actor_should_talk_to(e: &mut Engine, this: Ptr<Actor>, other: Ptr) -> bool {
    let process_object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    if process_object.is_null() {
        return false;
    }

    let blocked = e.vcall(this.addr(), 0x234, &args![]).bool()
        || e.call(REFR_FIELD_1AC_IS_9, &args![this]).bool()
        || e.call(EXTRA_OBJECTS_TEST, &args![this]).bool();
    if !blocked {
        let process_object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
        if e.vcall(process_object.addr(), PROCESS_SLOT_30C, &args![])
            .bool()
        {
            return false;
        }
    }

    let cell = e.call(REFR_PARENT_CELL, &args![this]).ptr::<()>();
    if !cell.is_null() {
        let cell = e.call(REFR_PARENT_CELL, &args![this]).ptr::<()>();
        if e.call(CELL_FLAG_24_BIT_0, &args![cell]).bool() {
            let this_position = e.vcall(this.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
            let other_position = e.vcall(other.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
            let height_difference =
                (e.mem.f32(this_position + 8) as f64 - e.mem.f32(other_position + 8) as f64) as f32;
            let distance = e.call(FLOAT_ABS, &args![height_difference]).f32() as f64;
            let limit: f64 = e.global(NINETY);
            if distance > limit {
                return false;
            }
        }
    }

    let process_object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    let flag = e
        .vcall(process_object.addr(), PROCESS_SLOT_22C, &args![])
        .u32();
    if flag != 0
        && e.call(ACTOR_SHOULD_SKIP_FALLOUT_BEHAVIOR, &args![this, 1u32])
            .bool()
    {
        return false;
    }

    let state = e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).u32();
    if state != 4 && e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).u32() != 0 {
        return false;
    }

    let package = e
        .call(MOBILE_OBJECT_GET_CURRENT_PACKAGE, &args![this])
        .ptr::<()>();
    if !package.is_null() {
        let package = e
            .call(MOBILE_OBJECT_GET_CURRENT_PACKAGE, &args![this])
            .ptr::<()>();
        if e.call(PACKAGE_IS_INTERRUPT_PACKAGE, &args![package]).bool() {
            return false;
        }
    }

    let process_object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    let value = e
        .vcall(process_object.addr(), PROCESS_SLOT_1E0, &args![])
        .f32() as f64;
    let zero: f64 = e.global(ZERO_DOUBLE);
    value <= zero
}

/// `Actor` virtual slot `0x2a8` (takes a coordinate pointer).
const ACTOR_SLOT_2A8: u32 = 0x2a8;
/// Form flag test: `*(form + 8) & 0x800`.
const FORM_FLAG_800: u32 = 0x0044_0da0;
/// Process virtual slots used by `Actor::InitPackageLocations`.
const PROCESS_SLOT_28: u32 = 0x28;
const PROCESS_SLOT_14: u32 = 0x14;
const PROCESS_SLOT_24: u32 = 0x24;
/// Accessor on the global at `0x011dea0c`: `*(u8 *)(this + 4)`.
const GLOBAL_11DEA0C: u32 = 0x011d_ea0c;
const GLOBAL_11DEA0C_FLAG: u32 = 0x004f_1540;
/// `ExtraDataList`-style accessor of a package's type: returns a type number
/// (`0xd` is skipped by `Actor::InitPackageLocations`).
const PACKAGE_TYPE: u32 = 0x0041_ca90;
/// `TESPackage` virtual slot `0x13c` (4 words: the actor, 0, a `float`, 0).
const PACKAGE_SLOT_13C: u32 = 0x13c;
/// `TESPackage::GetLocationCell`.
const PACKAGE_GET_LOCATION_CELL: u32 = 0x0067_5c20;
/// `TESPackage::GetLocationWorld`.
const PACKAGE_GET_LOCATION_WORLD: u32 = 0x0067_5a50;
/// `TESPackage::GetLocationCoord` (out coordinate, actor).
const PACKAGE_GET_LOCATION_COORD: u32 = 0x0067_5de0;
/// Reference call taking a `float` (here `FLT_MAX`).
const REFR_FLOAT_CALL: u32 = 0x0057_5770;
/// Returns the package's location (`0` if none).
const PACKAGE_GET_LOCATION: u32 = 0x0055_b980;
/// `PackageLocation::GetLocType`.
const PACKAGE_LOCATION_GET_LOC_TYPE: u32 = 0x0067_8ca0;
/// `TESObjectREFR::MoveRefToNewSpace` (cdecl: reference, cell, world).
const MOVE_REF_TO_NEW_SPACE: u32 = 0x0057_3800;
/// `Actor::UnlockLockDoorsProcedure`.
const ACTOR_UNLOCK_LOCK_DOORS_PROCEDURE: u32 = 0x008a_6a40;
/// `-1.0` (`float`).
const MINUS_ONE: u32 = 0x0101_2054;

// Translated from 00893340 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::InitPackageLocations` (Xbox PDB): unless the actor is flagged
/// `0x800`, (when `use_remembered` is 0) tells the process to reset itself
/// (slots `0x28`, `0x14` for a persistent reference, `0x24`), remembers the
/// process's current package (slot `0x22c`) unless the global at
/// `0x011dea0c` says otherwise, and, for a package of type above 2 that
/// accepts the actor (package slot `0x13c`), moves the actor to the package's
/// location; finally calls `Actor::UnlockLockDoorsProcedure`.
pub fn actor_init_package_locations(e: &mut Engine, this: Ptr<Actor>, use_remembered: u8) {
    if e.call(FORM_FLAG_800, &args![this]).bool() {
        return;
    }
    if use_remembered == 0 {
        let process = e.get(this, Actor::pCurrentProcess);
        e.vcall(process.addr(), PROCESS_SLOT_28, &args![]);
        if e.call(REFR_GET_REF_PERSISTS, &args![this]).bool() {
            let process = e.get(this, Actor::pCurrentProcess);
            e.vcall(process.addr(), PROCESS_SLOT_14, &args![this, 0u32]);
        }
        let process = e.get(this, Actor::pCurrentProcess);
        e.vcall(process.addr(), PROCESS_SLOT_24, &args![this, 0u32]);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    let mut package = e.vcall(process.addr(), PROCESS_SLOT_22C, &args![]).u32();
    if !e
        .call(GLOBAL_11DEA0C_FLAG, &args![e.global::<u32>(GLOBAL_11DEA0C)])
        .bool()
        && e.get(this, Actor::pInitialPackage) == 0
    {
        e.set(this, Actor::pInitialPackage, package);
    }
    if use_remembered != 0 {
        package = e.get(this, Actor::pInitialPackage);
    }
    let package = Ptr::<()>::new(package);
    if !package.is_null() && e.call(PACKAGE_TYPE, &args![package]).i32() > 2 {
        let minus_one: f32 = e.global(MINUS_ONE);
        if !e
            .vcall(
                package.addr(),
                PACKAGE_SLOT_13C,
                &args![this, 0u32, minus_one, 0u32],
            )
            .bool()
        {
            let cell = e
                .call(PACKAGE_GET_LOCATION_CELL, &args![package, this])
                .u32();
            let world = e
                .call(PACKAGE_GET_LOCATION_WORLD, &args![package, this])
                .u32();
            let float_max: f32 = e.global(LARGE_FLOAT);
            e.call(REFR_FLOAT_CALL, &args![this, float_max]);
            let location = e.call(PACKAGE_GET_LOCATION, &args![package]).ptr::<()>();
            let location_is_current = !location.is_null() && {
                let location = e.call(PACKAGE_GET_LOCATION, &args![package]).ptr::<()>();
                e.call(PACKAGE_LOCATION_GET_LOC_TYPE, &args![location])
                    .u32()
                    == 1
            };
            if !location_is_current && e.call(PACKAGE_TYPE, &args![package]).u32() != 0xd {
                e.with_stack(0x0c, |e, coordinate| {
                    let found = e
                        .call(
                            PACKAGE_GET_LOCATION_COORD,
                            &args![package, coordinate, this],
                        )
                        .u32();
                    e.vcall(this.addr(), ACTOR_SLOT_2A8, &args![found]);
                });
                e.call(MOVE_REF_TO_NEW_SPACE, &args![this, cell, world]);
            }
        }
    }
    e.call(ACTOR_UNLOCK_LOCK_DOORS_PROCEDURE, &args![this]);
}
/// `FLT_MAX` (`float`), as the second copy at `0x01016970`.
const LARGE_FLOAT: u32 = 0x0101_6970;

/// `GameSetting`-style global object whose value byte makes non-player
/// actors refuse to queue attacks (`Actor::QueueAttack`).
const NO_AI_ATTACKS_SETTING: u32 = 0x011d_f6c4;
/// Returns the address of the setting's value (`this + 4`).
const SETTING_VALUE_POINTER: u32 = 0x0040_8d60;
/// Weapon byte accessor: `*(u8 *)(form + 0x102)`.
const WEAPON_BYTE_102: u32 = 0x0052_4b60;
/// `TESAnimGroup::GetMove`.
const ANIM_GROUP_GET_MOVE: u32 = 0x005f_23a0;
/// Actor test (`004997b0`): non-zero while the actor moves.
const ACTOR_IS_MOVING: u32 = 0x0049_97b0;
/// `Animation::BlendOut` (slot, flag).
const ANIMATION_BLEND_OUT: u32 = 0x0049_94f0;
/// Process virtual slots used by `Actor::QueueAttack`.
const PROCESS_SLOT_27C: u32 = 0x27c;
const PROCESS_SLOT_368: u32 = 0x368;
const PROCESS_SLOT_3F8: u32 = 0x3f8;
const PROCESS_SLOT_6DC: u32 = 0x6dc;
/// `CombatFormulas::GetWeaponConditionJamMult` (cdecl, one word).
const GET_WEAPON_CONDITION_JAM_MULT: u32 = 0x0064_77b0;
/// `004dff00` (cdecl, one `float`): a chance roll, true when it succeeds.
const CHANCE_ROLL: u32 = 0x004d_ff00;
/// Weapon call giving the attack animation group base (flag argument).
const WEAPON_GET_ATTACK_ANIM: u32 = 0x0051_e2a0;
/// `TESAnimGroup::GetType` (cdecl, one word).
const ANIM_GROUP_GET_TYPE: u32 = 0x005f_2440;
/// `TESAnimGroup::IsAttackAction` (cdecl, one word).
const ANIM_GROUP_IS_ATTACK_ACTION: u32 = 0x005f_2540;
/// Form test used to choose between name sources (`00474cb0`).
const FORM_HAS_EDITOR_NAME: u32 = 0x0047_4cb0;
/// `MapMarkerData::GetLocationName` (engine map name).
const MAP_MARKER_DATA_GET_LOCATION_NAME: u32 = 0x0040_8da0;
/// Reference name accessor (`0055d520`).
const REFR_GET_NAME: u32 = 0x0055_d520;
/// Debug message printer (cdecl, format and arguments).
const DEBUG_PRINT: u32 = 0x005b_5e40;
/// Table of animation group descriptions, `0x24` bytes each, the first word
/// being the group's name.
const ANIM_GROUP_TABLE: u32 = 0x0119_77d8;
/// `"ANIM_GROUP_NONE"`.
const ANIM_GROUP_NONE_NAME: u32 = 0x0108_4a74;
/// `"hand-to-hand"`.
const HAND_TO_HAND_NAME: u32 = 0x0108_4a64;
/// `"COMBAT: '%s' is queuing an attack with weapon '%s' with attack '%s' that
/// has non-attack group '%s'..."`.
const QUEUE_NON_ATTACK_FORMAT: u32 = 0x0108_4a00;

/// Name of animation group `group` in the table, or `"ANIM_GROUP_NONE"` for
/// `0xff`.
fn anim_group_name(e: &Engine, group: u32) -> u32 {
    if group == 0xff {
        ANIM_GROUP_NONE_NAME
    } else {
        e.mem
            .u32(ANIM_GROUP_TABLE.wrapping_add(group.wrapping_mul(0x24)))
    }
}

// Translated from 008935f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::QueueAttack` (Xbox PDB): asks the actor to attack with animation
/// group `anim_group`. Returns true when the attack is started or queued for
/// later. The player starts it directly (`Actor::StartAttack`). Non-player
/// actors refuse while the setting at `0x011df6c4` is set. Fails when the
/// weapon needs ammunition and there is none, while an animation action of
/// `9` or `0x11` is running, with the weapon not drawn, and when the move
/// state of the animation disagrees with the actor's (the animation is then
/// blended out). A weapon that may jam queues the weapon's jam attack
/// instead (the weapon's attack animation plus `0x17`). A group that is not
/// an attack action is only reported with a debug message.
pub fn actor_queue_attack(e: &mut Engine, this: Ptr<Actor>, anim_group: u32) -> bool {
    let player = player(e);
    if this != player {
        let setting = e
            .call(SETTING_VALUE_POINTER, &args![NO_AI_ATTACKS_SETTING])
            .u32();
        if e.mem.u8(setting) != 0 {
            return false;
        }
    }

    let process = e.get(this, Actor::pCurrentProcess);
    let weapon_item = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
        .ptr::<()>();
    let weapon_form = if weapon_item.is_null() {
        Ptr::NULL
    } else {
        let weapon_item = e
            .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .ptr::<()>();
        entry_form(e, weapon_item)
    };
    let ammo_item = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_AMMO, &args![])
        .ptr::<()>();
    if !weapon_form.is_null() {
        // (the byte is read and not used)
        e.call(WEAPON_BYTE_102, &args![weapon_form]);
    }
    let weapon_uses_ammo = !weapon_form.is_null()
        && !e
            .call(WEAPON_GET_CURRENT_AMMO, &args![weapon_form, this])
            .ptr::<()>()
            .is_null();
    if weapon_uses_ammo {
        if ammo_item.is_null() {
            return false;
        }
        if e.call(LIST_NEXT, &args![ammo_item]).u32() == 0 {
            return false;
        }
    }

    if e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() == 9
        || e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() == 0x11
    {
        return false;
    }
    if !e.call(ACTOR_IS_WEAPON_DRAWN, &args![this]).bool() {
        return false;
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if !e.vcall(process.addr(), PROCESS_SLOT_3F8, &args![]).bool() {
        return false;
    }

    let animation = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .ptr::<()>();
    if animation.is_null() {
        return false;
    }
    if e.call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
        .u32()
        != 0
    {
        let sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
            .ptr::<()>();
        let transform = e
            .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
            .ptr::<()>();
        let group_moves = e.call(ANIM_GROUP_GET_MOVE, &args![transform]).u32() == 1;
        let actor_moves = e.call(ACTOR_IS_MOVING, &args![this]).u8() != 0;
        if group_moves != actor_moves {
            e.call(ANIMATION_BLEND_OUT, &args![animation, 4u32, 0u32]);
            return false;
        }
    }

    // The object the process is acquiring with decides whether the attack may
    // go ahead (type 8 and 0x10 packages stop it).
    let mut package_allows = true;
    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    let package = e
        .vcall(acquire.addr(), PROCESS_SLOT_27C, &args![])
        .ptr::<()>();
    if !package.is_null() {
        package_allows = e.call(PACKAGE_TYPE, &args![package]).i32() != 8
            && e.call(PACKAGE_TYPE, &args![package]).i32() != 0x10;
    }

    if package_allows
        && !weapon_form.is_null()
        && !e
            .call(WEAPON_GET_CURRENT_AMMO, &args![weapon_form, this])
            .ptr::<()>()
            .is_null()
    {
        let process = e.get(this, Actor::pCurrentProcess);
        let condition = e
            .vcall(process.addr(), PROCESS_SLOT_368, &args![this])
            .u32();
        if package_allows && !e.vcall(process.addr(), PROCESS_SLOT_6DC, &args![]).bool() {
            let jam_multiplier = e
                .call(GET_WEAPON_CONDITION_JAM_MULT, &args![condition])
                .f32() as f64;
            let zero: f64 = e.global(ZERO_DOUBLE);
            if jam_multiplier > zero {
                let jam_multiplier = e
                    .call(GET_WEAPON_CONDITION_JAM_MULT, &args![condition])
                    .f32();
                if e.call(CHANCE_ROLL, &args![jam_multiplier]).bool() {
                    let jam_group = e
                        .call(WEAPON_GET_ATTACK_ANIM, &args![weapon_form, 0u32])
                        .u32()
                        .wrapping_add(0x17);
                    let group = actor_get_anim_group(e, this, jam_group, 0, 0, Ptr::NULL);
                    if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32() == jam_group {
                        e.set(this, Actor::eQueuedattack, jam_group);
                    }
                    return false;
                }
            }
        }
    }

    let group = actor_get_anim_group(e, this, anim_group, 0, 0, Ptr::NULL);
    if this == player {
        actor_start_attack(e, this, anim_group);
        return true;
    }
    if e.call(ANIM_GROUP_IS_ATTACK_ACTION, &args![u32::from(group)])
        .bool()
    {
        e.set(this, Actor::eQueuedattack, anim_group);
        return true;
    }

    // Not an attack action: report it.
    report_group_mismatch(
        e,
        this,
        weapon_form,
        group,
        anim_group,
        QUEUE_NON_ATTACK_FORMAT,
    );
    false
}

/// `Actor::GetAnimation` (called directly by the animation helpers).
const ACTOR_GET_ANIMATION: u32 = 0x008b_70d0;
/// `Actor::SetAnimAction` (action, sequence).
const ACTOR_SET_ANIM_ACTION: u32 = 0x008a_73e0;
/// `Animation::PlayGroup` (group, 1, -1, -1).
const ANIMATION_PLAY_GROUP: u32 = 0x0049_4740;
/// `Animation::ClearGroup` (slot, blend time `float`).
const ANIMATION_CLEAR_GROUP: u32 = 0x0049_6080;
/// Animation accessor: the animation group id of a slot (`u16`).
const ANIMATION_GROUP_OF_SLOT: u32 = 0x0043_01b0;
/// Animation accessor: the movement speed of a group (`i16`).
const ANIMATION_MOVEMENT_SPEED: u32 = 0x0049_4300;
/// `PlayerCharacter::GetAnimation` (flag).
const PLAYER_GET_ANIMATION: u32 = 0x0095_0a60;
/// `Actor::GetWalkSpeed` (returns a `float` in ST0).
const ACTOR_GET_WALK_SPEED: u32 = 0x0088_4dc0;
/// `Actor` virtual slot `0x4b0` (group id, flag).
const ACTOR_SLOT_4B0: u32 = 0x4b0;
/// `008a0330`: the object the actor's process returns from its virtual slot
/// `0x22c` (the combat controller's `this` in the calls below).
const ACTOR_PROCESS_OBJECT: u32 = 0x008a_0330;
/// `009818e0`: combat controller call taking the blocked flag.
const COMBAT_CONTROLLER_SET_BLOCKED: u32 = 0x0098_18e0;
/// `TESAnimGroup::IsPowerAttackAction` (cdecl, one word).
const ANIM_GROUP_IS_POWER_ATTACK_ACTION: u32 = 0x005f_2670;
/// Animation call (slot): the slot's blend state (`0x0070f490`).
const ANIMATION_SLOT_STATE: u32 = 0x0070_f490;
/// `LowProcess::GetGenericLocation` (engine map name) on an animation
/// sequence.
const SEQUENCE_GET_GENERIC_LOCATION: u32 = 0x0080_41a0;
/// `005f2420` on the global transform: the animation group number.
const ANIM_GROUP_GET_NUMBER: u32 = 0x005f_2420;
/// Weapon form test (`006450c0`) the block code runs on the current weapon.
const WEAPON_FORM_ALLOWS_BLOCK: u32 = 0x0064_50c0;
/// Call (`004954e0`) on the global transform: returns a byte.
const TRANSFORM_BYTE: u32 = 0x0049_54e0;
/// Game setting object whose `float` is the default blend time.
const BLEND_SETTING: u32 = 0x011c_56fc;
/// `30.0` (`double`).
const THIRTY: u32 = 0x0101_db88;
/// Process virtual slot `0x614` (a flag word).
const PROCESS_SLOT_614: u32 = 0x614;

/// Tells the combat controller (`Actor` slot `0x428`), when there is one,
/// whether the actor is blocking: `use_blocked_state` passes
/// `Actor::GetBlocked`, otherwise the `+0xf0` byte.
fn notify_combat_controller(e: &mut Engine, this: Ptr<Actor>, use_blocked_state: bool) {
    if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() == 0 {
        return;
    }
    let flag = if use_blocked_state {
        u32::from(actor_get_blocked(e, this))
    } else {
        u32::from(e.mem.u8(this.addr() + 0xf0))
    };
    let controller = e.call(ACTOR_PROCESS_OBJECT, &args![this]).ptr::<()>();
    e.call(COMBAT_CONTROLLER_SET_BLOCKED, &args![controller, flag]);
}

/// Plays `group` in `animation` (`Animation::PlayGroup` with blend -1/-1),
/// sets the action from the sequence of `slot` and tells the actor
/// (`Actor` slot `0x4b0`).
fn play_group_and_set_action(
    e: &mut Engine,
    this: Ptr<Actor>,
    animation: Ptr,
    group: u32,
    slot: u32,
    action: u32,
) {
    e.call(
        ANIMATION_PLAY_GROUP,
        &args![animation, group, 1u32, 0xffff_ffffu32, 0xffff_ffffu32],
    );
    let sequence = e
        .call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
        .u32();
    e.call(ACTOR_SET_ANIM_ACTION, &args![this, action, sequence]);
    e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![group, 1u32]);
}

// Translated from 00894900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the actor's animation action (`Actor::GetAnimAction`) is from 2 to
/// 6.
pub fn fn_00894900(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let action = e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32();
    (2..=6).contains(&action)
}

// Translated from 00894d60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetBlocked` (Xbox PDB): whether the animation action is 7.
pub fn actor_get_blocked(e: &mut Engine, this: Ptr<Actor>) -> bool {
    e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() == 7
}

// Translated from 00894940 (decompiled, FalloutNV.exe 1.4.0.525)
/// Updates the actor's blocking animation (slot 2 of its animation) to match
/// the byte at `+0xf0` (`bBlockPostAnim`) and the current action. When the
/// block sequence is there but not "generic", only the combat controller is
/// told. Otherwise the block flag is recomputed from the action and the group
/// playing in slot 4; if the block group `0xaa` is not playing and the actor
/// should block, it is played (or only the action is set when it is already
/// the group in slot 2); if the block group is playing and the actor no
/// longer blocks, it is cleared (the player's own animation as well).
pub fn fn_00894940(e: &mut Engine, this: Ptr<Actor>) {
    let animation = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .ptr::<()>();
    let block_sequence = e
        .call(ANIMATION_GET_SEQUENCE, &args![animation, 2u32])
        .ptr::<()>();
    if !block_sequence.is_null()
        && e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![block_sequence])
            .u32()
            != 1
    {
        notify_combat_controller(e, this, true);
        return;
    }

    let mut should_block = e.mem.u8(this.addr() + 0xf0) != 0;
    let action = e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32();
    if action == -1 {
        should_block = true;
    } else if action == 4 {
        let attack_group = e
            .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
            .u16();
        if e.call(ANIM_GROUP_IS_ATTACK_ACTION, &args![u32::from(attack_group)])
            .bool()
        {
            let attack_group = e
                .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
                .u16();
            if e.call(
                ANIM_GROUP_IS_POWER_ATTACK_ACTION,
                &args![u32::from(attack_group)],
            )
            .bool()
                || e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32() < 3
            {
                should_block = false;
            }
        } else {
            let group = e
                .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 2u32])
                .u16();
            if e.call(ANIM_GROUP_IS_POWER_ATTACK_ACTION, &args![u32::from(group)])
                .bool()
            {
                should_block = false;
            }
        }
    }

    let mut block_group_playing = false;
    if !block_sequence.is_null() {
        let transform = e
            .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![block_sequence])
            .ptr::<()>();
        block_group_playing = e.call(ANIM_GROUP_GET_NUMBER, &args![transform]).u32() == 0xaa;
    }

    if !block_group_playing {
        if e.mem.u8(this.addr() + 0xf0) != 0 && should_block {
            let process = e.get(this, Actor::pCurrentProcess);
            let weapon = e
                .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
                .ptr::<()>();
            let weapon_allows_block = weapon.is_null() || {
                let weapon = e
                    .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
                    .ptr::<()>();
                let weapon_form = entry_form(e, weapon);
                e.call(WEAPON_FORM_ALLOWS_BLOCK, &args![weapon_form]).bool()
            };
            if weapon_allows_block {
                let action = e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32();
                if action != -1 && action != 4 {
                    should_block = false;
                }
            }
        }
        if e.mem.u8(this.addr() + 0xf0) != 0 && should_block {
            let group = actor_get_anim_group(e, this, 0xaa, 0, 0, Ptr::NULL);
            let playing = e
                .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 2u32])
                .u16();
            if playing == group {
                let sequence = e
                    .call(ANIMATION_GET_SEQUENCE, &args![animation, 2u32])
                    .u32();
                e.call(ACTOR_SET_ANIM_ACTION, &args![this, 7u32, sequence]);
            } else {
                play_group_and_set_action(e, this, animation, u32::from(group), 2, 7);
            }
            notify_combat_controller(e, this, false);
        }
        return;
    }

    // The block group is playing: stop it unless the actor still blocks.
    if e.mem.u8(this.addr() + 0xf0) != 0 {
        return;
    }
    let setting_pointer = e
        .call(SETTING_GET_VALUE_POINTER, &args![BLEND_SETTING])
        .u32();
    let mut blend_time = e.mem.f32(setting_pointer);
    let mut transform_byte = 0u8;
    if !block_sequence.is_null() {
        let transform = e
            .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![block_sequence])
            .ptr::<()>();
        transform_byte = e.call(TRANSFORM_BYTE, &args![transform]).u8();
    }
    if transform_byte != 0 {
        let thirty: f64 = e.global(THIRTY);
        blend_time = (i32::from(transform_byte) as f64 / thirty) as f32;
    }
    e.call(ANIMATION_CLEAR_GROUP, &args![animation, 2u32, blend_time]);
    if this == player(e) {
        let the_player = player(e);
        let player_animation = e
            .call(PLAYER_GET_ANIMATION, &args![the_player, 1u32])
            .ptr::<()>();
        e.call(
            ANIMATION_CLEAR_GROUP,
            &args![player_animation, 2u32, blend_time],
        );
    }
    e.call(ACTOR_SET_ANIM_ACTION, &args![this, 0xffff_ffffu32, 0u32]);
    notify_combat_controller(e, this, false);
}

// Translated from 00894cc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetBlock` (Xbox PDB): stores `blocking` in the `+0xf0` byte and
/// applies it (`fn_00894940` for the player, process slot `0x614` with
/// `0x8000` for others). Returns false, after telling the combat controller,
/// when the actor has no animation.
pub fn actor_set_block(e: &mut Engine, this: Ptr<Actor>, blocking: u8) -> bool {
    let animation = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .u32();
    if animation == 0 {
        notify_combat_controller(e, this, true);
        return false;
    }
    e.mem.set_u8(this.addr() + 0xf0, blocking);
    if this == player(e) {
        fn_00894940(e, this);
    } else {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
        e.vcall(acquire.addr(), PROCESS_SLOT_614, &args![0x8000u32]);
    }
    true
}

// Translated from 00894d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Starts the idle group `0xab` (in the animation slot that the group table
/// names at +8 of its entry) when the actor has an animation and a process, is
/// not busy (`Actor::GetAnimAction` is not 10, slot `0x214` is 0) and its
/// group has type `0xab`. `_unused_1` is a word the code never reads.
pub fn fn_00894d90(e: &mut Engine, this: Ptr<Actor>, _unused_1: u32) {
    if e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() == 10 {
        return;
    }
    let animation = e.call(ACTOR_GET_ANIMATION, &args![this]).ptr::<()>();
    let process = e.get(this, Actor::pCurrentProcess);
    if animation.is_null()
        || process.is_null()
        || e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).u32() != 0
    {
        return;
    }
    let mut target_type = 0xabu32;
    let group = actor_get_anim_group(e, this, target_type, 0, 0, Ptr::NULL);
    if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32() != target_type
        && e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32() == 0xab
    {
        target_type = 0xab;
    }
    if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32() == target_type {
        e.call(
            ANIMATION_PLAY_GROUP,
            &args![
                animation,
                u32::from(group),
                1u32,
                0xffff_ffffu32,
                0xffff_ffffu32
            ],
        );
        let slot = e.mem.u32(ANIM_GROUP_TABLE + 8 + target_type * 0x24);
        let sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
            .u32();
        e.call(ACTOR_SET_ANIM_ACTION, &args![this, 7u32, sequence]);
        e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![u32::from(group), 1u32]);
    }
}

// Translated from 00894e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Starts the group `0xac` in animation slot 4 (action 8), or, when the
/// actor's group is not of type `0xac`, clears slot 4 (the player's animation
/// as well). Same guards as `fn_00894d90`.
pub fn fn_00894e90(e: &mut Engine, this: Ptr<Actor>) {
    if e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() == 10 {
        return;
    }
    let animation = e.call(ACTOR_GET_ANIMATION, &args![this]).ptr::<()>();
    let process = e.get(this, Actor::pCurrentProcess);
    if animation.is_null()
        || process.is_null()
        || e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).u32() != 0
    {
        return;
    }
    let group = actor_get_anim_group(e, this, 0xac, 0, 0, Ptr::NULL);
    if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32() == 0xac {
        play_group_and_set_action(e, this, animation, u32::from(group), 4, 8);
    } else {
        e.call(ANIMATION_CLEAR_GROUP, &args![animation, 4u32, 0.0f32]);
        if this == player(e) {
            let the_player = player(e);
            let player_animation = e
                .call(PLAYER_GET_ANIMATION, &args![the_player, 1u32])
                .ptr::<()>();
            e.call(
                ANIMATION_CLEAR_GROUP,
                &args![player_animation, 4u32, 0.0f32],
            );
        }
    }
}

// Translated from 00894f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Plays a movement-direction group for `directions`: bit 0 gives group
/// `0xb`, bit 1 `0xc`, bit 2 `0xd`, bit 3 `0xe` (the first bit set wins). If
/// the actor's animation picks one of those groups it is played in slot 1
/// (action `0xb`), and the animation's speed multiplier (`fn_008950f0`) is
/// set to the actor's walk speed over the animation's own movement speed for
/// the matching movement group. Returns the group asked for, or `0xff`.
pub fn fn_00894f90(e: &mut Engine, this: Ptr<Actor>, directions: u8) -> u32 {
    let animation = e.call(ACTOR_GET_ANIMATION, &args![this]).ptr::<()>();
    let process = e.get(this, Actor::pCurrentProcess);
    if animation.is_null() || process.is_null() {
        return 0xff;
    }
    let mut wanted = 0xffu32;
    if directions & 1 != 0 {
        wanted = 0xb;
    } else if directions & 2 != 0 {
        wanted = 0xc;
    } else if directions & 4 != 0 {
        wanted = 0xd;
    } else if directions & 8 != 0 {
        wanted = 0xe;
    }
    let group = actor_get_anim_group(e, this, wanted, 0, 0, Ptr::NULL);
    let group_number = u32::from(group);
    if !(0xb..=0xe).contains(&group_number) {
        return 0xff;
    }
    play_group_and_set_action(e, this, animation, group_number, 1, 0xb);
    let movement_group = u32::from((group & 0xff00) | 3);
    let mut speed_multiplier = 1.0f32;
    let walk_speed = e.call(ACTOR_GET_WALK_SPEED, &args![this]).f32();
    if e.call(ANIMATION_MOVEMENT_SPEED, &args![animation, movement_group])
        .u16() as i16
        != 0
    {
        let movement_speed = e
            .call(ANIMATION_MOVEMENT_SPEED, &args![animation, movement_group])
            .u16() as i16;
        speed_multiplier =
            ((walk_speed as f64 / f64::from(movement_speed)) * speed_multiplier as f64) as f32;
    }
    fn_008950f0(e, animation, speed_multiplier);
    wanted
}

// Translated from 008950f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the `float` at `+0x10c` of an animation object (its speed
/// multiplier).
pub fn fn_008950f0(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0x10c, value);
}

// Translated from 00897890 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 2 of the dword at `+0x12c` is set.
pub fn fn_00897890(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x12c) & 4 != 0
}

// Translated from 008978b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global word at `0x011c6230`.
pub fn fn_008978b0(e: &mut Engine) -> u32 {
    e.global(0x011c_6230)
}

// Translated from 008978c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global word at `0x011c6234`.
pub fn fn_008978c0(e: &mut Engine) -> u32 {
    e.global(0x011c_6234)
}

// Translated from 008978d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `00448a60(this, 0x100)` is non-zero.
pub fn fn_008978d0(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0044_8a60, &args![this, 0x100u32]).u32() != 0
}

// Translated from 008978f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at `this + index * 0x30 + 0x1bd`.
pub fn fn_008978f0(e: &mut Engine, this: Ptr, index: u32, value: u8) {
    let address = this
        .addr()
        .wrapping_add(index.wrapping_mul(0x30))
        .wrapping_add(0x1bd);
    e.mem.set_u8(address, value);
}

// Translated from 00897b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the word at `+0x6a0`.
pub fn fn_00897b30(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x6a0)
}

/// `TESAnimGroup::AnimGroup` (cdecl: selector a, selector b, group, flag).
const ANIM_GROUP_COMPOSE: u32 = 0x005f_2370;
/// `Animation::PickBestAnimation` (group, 0).
const ANIMATION_PICK_BEST_ANIMATION: u32 = 0x0049_5740;
/// Actor flags accessor (`008846e0`): the `u16` flags of the object at
/// `this + 0x190`, or 0 when there is none.
const ACTOR_ANIM_FLAGS: u32 = 0x0088_46e0;
/// Actor test (`008a6970`).
const ACTOR_TEST_8A6970: u32 = 0x008a_6970;
/// Actor tests that look at process slots `0x6f4` / `0x6ec`.
const ACTOR_TEST_8BA3E0: u32 = 0x008b_a3e0;
const ACTOR_TEST_8BA410: u32 = 0x008b_a410;
/// `Actor` virtual slot `0x390`.
const ACTOR_SLOT_390: u32 = 0x390;
/// Process virtual slot `0x454` (a bool).
const PROCESS_SLOT_454: u32 = 0x454;
/// Accessor at `+0x6a0` (see [`fn_00897b30`]).
const ACTOR_FIELD_6A0: u32 = 0x0089_7b30;
/// `Animation::GetTESAnimGroup` (group).
const ANIMATION_GET_TES_ANIM_GROUP: u32 = 0x0049_6500;
/// `TESAnimGroup::GetTime` (flag; returns a `float` in ST0).
const ANIM_GROUP_GET_TIME: u32 = 0x005f_3780;
/// Table of the animation selector words per weapon type (indexed by the
/// weapon type byte, 4 bytes each).
const WEAPON_TYPE_ANIM_TABLE: u32 = 0x0118_a838;

// Translated from 00897910 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetAnimGroup` (Xbox PDB): the animation group (`u16`) to use for
/// the group number `group_id`. `animation` is the animation object (null:
/// the actor's own, `Actor` slot `0x1e4`); `weapon_arg` selects the weapon
/// (`0xffffffff`: none, `0`: the process's current weapon). The group is
/// composed by `TESAnimGroup::AnimGroup` from two selectors (one from the
/// actor's flags `0x800` / `0x2000` / `0x400`, one from the weapon type
/// through [`WEAPON_TYPE_ANIM_TABLE`], or 1) and then refined by
/// `Animation::PickBestAnimation`. Returns `0xff` when there is no animation.
pub fn actor_get_anim_group(
    e: &mut Engine,
    this: Ptr<Actor>,
    group_id: u32,
    weapon_arg: u32,
    flag: u8,
    animation: Ptr,
) -> u16 {
    let mut animation = animation;
    if animation.is_null() {
        animation = e
            .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
            .ptr();
    }
    if animation.is_null() {
        return 0xff;
    }
    let flags = e.call(ACTOR_ANIM_FLAGS, &args![this]).u16();
    let mut selector_a = 0u32;
    let mut selector_b = 0u32;
    let weapon: Ptr = if weapon_arg == 0xffff_ffff {
        Ptr::NULL
    } else if weapon_arg != 0 {
        Ptr::new(weapon_arg)
    } else {
        let process = e.get(this, Actor::pCurrentProcess);
        e.vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .ptr()
    };
    if flags & 0x800 != 0 {
        selector_a = 2;
    } else if flags & 0x2000 != 0 {
        selector_a = 3;
    } else if flags & 0x400 != 0 {
        selector_a = 1;
    }

    // Whether the weapon (or the default `1`) decides the second selector.
    let mut uses_weapon_selector = e.vcall(this.addr(), ACTOR_SLOT_390, &args![]).u32() == 0
        && (e.call(ACTOR_TEST_8A6970, &args![this]).bool()
            || e.call(ACTOR_IN_COMBAT, &args![this]).bool());
    if !uses_weapon_selector {
        let process = e.get(this, Actor::pCurrentProcess);
        uses_weapon_selector = e.vcall(process.addr(), PROCESS_SLOT_454, &args![]).bool()
            || (this == player(e) && {
                let the_player = player(e);
                e.call(ACTOR_FIELD_6A0, &args![the_player]).u32() != 0
            });
        if !uses_weapon_selector {
            if !weapon.is_null() {
                uses_weapon_selector = weapon_arg != 0 || {
                    let process = e.get(this, Actor::pCurrentProcess);
                    e.vcall(process.addr(), PROCESS_SLOT_454, &args![]).bool()
                        || (e.call(ACTOR_TEST_8A6970, &args![this]).bool() && group_id != 0xaa)
                };
            }
            uses_weapon_selector |= flag != 0;
        }
    }
    if uses_weapon_selector {
        if !weapon.is_null() && flag == 0 {
            let weapon_form = entry_form(e, weapon);
            let weapon_type = e.call(WEAPON_TYPE, &args![weapon_form]).u32();
            selector_b = e
                .mem
                .u32(WEAPON_TYPE_ANIM_TABLE.wrapping_add(weapon_type.wrapping_mul(4)));
        } else {
            selector_b = 1;
        }
    }

    let in_special_state = e.call(ACTOR_TEST_8BA3E0, &args![this]).bool()
        || e.call(ACTOR_TEST_8BA410, &args![this]).bool();
    let composed = e
        .call(
            ANIM_GROUP_COMPOSE,
            &args![
                selector_a,
                selector_b,
                group_id,
                u32::from(in_special_state)
            ],
        )
        .u16();
    e.call(
        ANIMATION_PICK_BEST_ANIMATION,
        &args![animation, u32::from(composed), 0u32],
    )
    .u16()
}

// Translated from 00897b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetAnimGroupDuration` (Xbox PDB): the duration (`float`) of the
/// animation group `group` of the actor's animation, 0.0 when there is none.
pub fn actor_get_anim_group_duration(e: &mut Engine, this: Ptr<Actor>, group: u32) -> f32 {
    let mut duration = 0.0f32;
    let animation = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .ptr::<()>();
    if !animation.is_null() {
        let picked = actor_get_anim_group(e, this, group, 0, 0, animation);
        if picked != 0 {
            let tes_group = e
                .call(
                    ANIMATION_GET_TES_ANIM_GROUP,
                    &args![animation, u32::from(picked)],
                )
                .ptr::<()>();
            if !tes_group.is_null() {
                duration = e.call(ANIM_GROUP_GET_TIME, &args![tes_group, 1u32]).f32();
            }
        }
    }
    duration
}

/// `TESPackage::CreatePackage` (cdecl, package type).
const PACKAGE_CREATE: u32 = 0x0067_0b90;
/// `TESPackage::SetPackType`.
const PACKAGE_SET_PACK_TYPE: u32 = 0x0067_0fc0;
/// `00826b40` / `00826b90`: package flag setters taking a byte.
const PACKAGE_SET_FLAG_A: u32 = 0x0082_6b40;
const PACKAGE_SET_FLAG_B: u32 = 0x0082_6b90;
/// `operator new` (`00401000`).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `PackageLocation::PackageLocation`.
const PACKAGE_LOCATION_CONSTRUCT: u32 = 0x0067_f030;
/// `PackageLocation::SetLocReference`.
const PACKAGE_LOCATION_SET_REFERENCE: u32 = 0x0067_f3c0;
/// `PackageLocation::SetLocCell`.
const PACKAGE_LOCATION_SET_CELL: u32 = 0x0067_f410;
/// `0067f1f0`: sets a radius on a package location.
const PACKAGE_LOCATION_SET_RADIUS: u32 = 0x0067_f1f0;
/// `TESPackage::SetPackageLocation`.
const PACKAGE_SET_LOCATION: u32 = 0x0067_1d30;
/// `PackageLocation::~PackageLocation` (flag 1 deletes).
const PACKAGE_LOCATION_DESTROY: u32 = 0x0067_0b30;
/// `PackageTarget::PackageTarget`.
const PACKAGE_TARGET_CONSTRUCT: u32 = 0x0067_ff70;
/// `TESPackage::SetPackageTarget`.
const PACKAGE_SET_TARGET: u32 = 0x0067_2fc0;
/// Package accessor: the package's target (`+0x30`).
const PACKAGE_GET_TARGET: u32 = 0x0067_1d10;
/// `PackageTarget::SetTargType`.
const PACKAGE_TARGET_SET_TYPE: u32 = 0x0068_00b0;
/// `PackageTarget::SetTargReference`.
const PACKAGE_TARGET_SET_REFERENCE: u32 = 0x0068_0110;
/// Package setter: `*(u32 *)(this + 0x18) = value`.
const PACKAGE_SET_FIELD_18: u32 = 0x0098_4f60;
/// `PackageTarget` destructor with delete flag (`007b3fa0`).
const PACKAGE_TARGET_DESTROY: u32 = 0x007b_3fa0;
/// `0067a1b0`: copies the process's current package data into a package.
const PACKAGE_COPY_FROM: u32 = 0x0067_a1b0;
/// `ExtraDataList`-style accessor: `base form of this` (`004181e0` calls
/// `007af430` on its `this`).
const ACTOR_BASE_FORM: u32 = 0x0041_81e0;
/// Reads `*(this + 0x14)`.
const FORM_FIELD_14: u32 = 0x0082_5c00;
/// Sets `*(this + 8) = value` (a package target's count).
const TARGET_SET_COUNT: u32 = 0x0040_3550;
/// Process virtual slot `0x28` (no arguments).
const PROCESS_SLOT_SET_28: u32 = 0x28;
/// `Actor` virtual slot `0x2f4` (package, flag, flag): puts a package on the
/// actor.
const ACTOR_SLOT_ADD_PACKAGE: u32 = 0x2f4;
/// `_ftol2_sse` (`00ec62c0`), the float to integer helper (an `f64`).
const FTOL: u32 = 0x00ec_62c0;

/// Creates a package location for `actor`, hands it to the package and
/// releases it (what the three package starters do).
fn give_package_actor_location(e: &mut Engine, package: Ptr, actor: Ptr<Actor>) {
    let memory = e.call(OPERATOR_NEW, &args![0xcu32]).ptr::<()>();
    let location = if memory.is_null() {
        Ptr::NULL
    } else {
        e.call(PACKAGE_LOCATION_CONSTRUCT, &args![memory])
            .ptr::<()>()
    };
    e.call(PACKAGE_LOCATION_SET_REFERENCE, &args![location, actor]);
    e.call(PACKAGE_SET_LOCATION, &args![package, location]);
    if !location.is_null() {
        e.call(PACKAGE_LOCATION_DESTROY, &args![location, 1u32]);
    }
}

/// Allocates and constructs a `PackageTarget` (0x10 bytes).
fn new_package_target(e: &mut Engine) -> Ptr {
    let memory = e.call(OPERATOR_NEW, &args![0x10u32]).ptr::<()>();
    if memory.is_null() {
        Ptr::NULL
    } else {
        e.call(PACKAGE_TARGET_CONSTRUCT, &args![memory]).ptr()
    }
}

// Translated from 00897bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Gives the actor a package of type `0xe` (field `0x18` = `0x29`) that
/// targets the actor itself with a location at the actor, copies the
/// process's current package data into it (`0067a1b0`), sets the target's
/// count from the actor's base form (field `0x14` of the word at
/// base form `+0x90`), tells the process (slot `0x28`) and puts the package on
/// the actor (`Actor` slot `0x2f4`). `_unused_1` is a word the code never
/// reads.
pub fn fn_00897bd0(e: &mut Engine, this: Ptr<Actor>, _unused_1: u32) {
    let package = e.call(PACKAGE_CREATE, &args![0xeu32]).ptr::<()>();
    e.call(PACKAGE_SET_PACK_TYPE, &args![package, 0xeu32]);
    e.call(PACKAGE_SET_FLAG_A, &args![package, 0u32]);
    e.call(PACKAGE_SET_FLAG_B, &args![package, 0u32]);
    give_package_actor_location(e, package, this);

    let target = new_package_target(e);
    e.call(PACKAGE_SET_TARGET, &args![package, target]);
    e.call(PACKAGE_SET_FIELD_18, &args![package, 0x29u32]);
    let package_target = e.call(PACKAGE_GET_TARGET, &args![package]).ptr::<()>();
    e.call(PACKAGE_TARGET_SET_TYPE, &args![package_target, 0u32]);
    let package_target = e.call(PACKAGE_GET_TARGET, &args![package]).ptr::<()>();
    e.call(PACKAGE_TARGET_SET_REFERENCE, &args![package_target, this]);

    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    let current_package = e
        .vcall(acquire.addr(), PROCESS_SLOT_22C, &args![])
        .ptr::<()>();
    if !current_package.is_null() {
        e.call(PACKAGE_COPY_FROM, &args![package, current_package]);
    }
    let base = e.call(ACTOR_BASE_FORM, &args![this]).ptr::<()>();
    let value = e.call(FORM_FIELD_14, &args![base.byte_add(0x90)]).u32();
    let package_target = e.call(PACKAGE_GET_TARGET, &args![package]).ptr::<()>();
    e.call(TARGET_SET_COUNT, &args![package_target, value]);

    if !target.is_null() {
        e.call(PACKAGE_TARGET_DESTROY, &args![target, 1u32]);
    }
    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    e.vcall(acquire.addr(), PROCESS_SLOT_SET_28, &args![]);
    e.vcall(
        this.addr(),
        ACTOR_SLOT_ADD_PACKAGE,
        &args![package, 1u32, 1u32],
    );
}

// Translated from 008982c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::InitiateAvoidPackage` (Xbox PDB): ends the actor's movement, then
/// gives it a package of type `0x1f` (both flags 1) with a location at the
/// actor and a target of type 0 on `target`, marks `target` as targeted,
/// computes the procedure type, copies the process's current package data,
/// passes `speed` to the process (slot `0x25c`) and puts the package on the
/// actor.
pub fn actor_initiate_avoid_package(e: &mut Engine, this: Ptr<Actor>, target: Ptr, speed: f32) {
    e.call(ACTOR_END_MOVEMENT, &args![this]);
    let package = e.call(PACKAGE_CREATE, &args![0x1fu32]).ptr::<()>();
    e.call(PACKAGE_SET_FLAG_A, &args![package, 1u32]);
    e.call(PACKAGE_SET_FLAG_B, &args![package, 1u32]);
    give_package_actor_location(e, package, this);

    let package_target = new_package_target(e);
    e.call(PACKAGE_SET_TARGET, &args![package, package_target]);
    let stored_target = e.call(PACKAGE_GET_TARGET, &args![package]).ptr::<()>();
    e.call(PACKAGE_TARGET_SET_TYPE, &args![stored_target, 0u32]);
    let stored_target = e.call(PACKAGE_GET_TARGET, &args![package]).ptr::<()>();
    e.call(PACKAGE_TARGET_SET_REFERENCE, &args![stored_target, target]);
    e.call(REFR_SET_TARGETED, &args![target, 1u32]);
    if !package_target.is_null() {
        e.call(PACKAGE_TARGET_DESTROY, &args![package_target, 1u32]);
    }
    e.call(PACKAGE_CALCULATE_PROCEDURE_TYPE, &args![package, 0u32]);

    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    let current_package = e
        .vcall(acquire.addr(), PROCESS_SLOT_22C, &args![])
        .ptr::<()>();
    if !current_package.is_null() {
        e.call(PACKAGE_COPY_FROM, &args![package, current_package]);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), PROCESS_SLOT_25C, &args![speed]);
    e.vcall(
        this.addr(),
        ACTOR_SLOT_ADD_PACKAGE,
        &args![package, 1u32, 1u32],
    );
}

// Translated from 008984c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::InitiateSurfacePackage` (Xbox PDB): stops the actor
/// (`Actor::ForceStopMoving`), then gives it a package of type `0x1d` (both
/// flags 1) with a location at the actor and the procedure type computed.
pub fn actor_initiate_surface_package(e: &mut Engine, this: Ptr<Actor>) {
    e.call(ACTOR_FORCE_STOP_MOVING, &args![this]);
    let package = e.call(PACKAGE_CREATE, &args![0x1du32]).ptr::<()>();
    e.call(PACKAGE_SET_PACK_TYPE, &args![package, 0x1du32]);
    e.call(PACKAGE_SET_FLAG_A, &args![package, 1u32]);
    e.call(PACKAGE_SET_FLAG_B, &args![package, 1u32]);
    give_package_actor_location(e, package, this);
    e.call(PACKAGE_CALCULATE_PROCEDURE_TYPE, &args![package, 0u32]);
    e.vcall(
        this.addr(),
        ACTOR_SLOT_ADD_PACKAGE,
        &args![package, 1u32, 1u32],
    );
}

/// `Actor::EndMovement`.
const ACTOR_END_MOVEMENT: u32 = 0x0087_faa0;
/// `Actor::ForceStopMoving`.
const ACTOR_FORCE_STOP_MOVING: u32 = 0x008b_3ad0;
/// `TESObjectREFR::SetTargeted`.
const REFR_SET_TARGETED: u32 = 0x0056_4db0;
/// `TESPackage::CalculateProcedureType`.
const PACKAGE_CALCULATE_PROCEDURE_TYPE: u32 = 0x0067_77b0;
/// Process virtual slot `0x25c` (a `float`).
const PROCESS_SLOT_25C: u32 = 0x25c;

/// `FleePackage::FleePackage` (this, avoided reference, 0, 0).
const FLEE_PACKAGE_CONSTRUCT: u32 = 0x009f_0e00;
/// `FleePackage::AddAvoidedRef`.
const FLEE_PACKAGE_ADD_AVOIDED_REF: u32 = 0x009f_1310;
/// `FleePackage::FindTeleportDoor` (actor, avoided reference).
const FLEE_PACKAGE_FIND_TELEPORT_DOOR: u32 = 0x009f_1140;
/// Extra-data test (`00437bf0`): the object's extra type is 5.
const EXTRA_TYPE_IS_5: u32 = 0x0043_7bf0;
/// `MobileObject::GetCurrentPackage`.
const GET_CURRENT_PACKAGE: u32 = 0x0093_44a0;
/// Package setter (`005d0b80`): `*(u8 *)(this + 0x80) = value`.
const PACKAGE_SET_BYTE_80: u32 = 0x005d_0b80;
/// `00994ef0`: package call taking a reference.
const PACKAGE_SET_REFERENCE_994EF0: u32 = 0x0099_4ef0;
/// `005e3fc0`: package accessor `*(this + 0xa4)`.
const PACKAGE_FIELD_A4: u32 = 0x005e_3fc0;
/// `00810530`: package setter `*(u8 *)(this + 0x94) = value`.
const PACKAGE_SET_BYTE_94: u32 = 0x0081_0530;
/// `006ca4e0`: package accessor `*(f32 *)(this + 0x90)`.
const PACKAGE_FLOAT_90: u32 = 0x006c_a4e0;
/// `0067a720`: package call taking a flag.
const PACKAGE_CALL_67A720: u32 = 0x0067_a720;
/// Process virtual slots used by `fn_00897de0`: `0xe4` returns a `float`,
/// `0xe8` takes one, `0x710` takes the actor.
const PROCESS_SLOT_E4: u32 = 0xe4;
const PROCESS_SLOT_E8: u32 = 0xe8;
const PROCESS_SLOT_710: u32 = 0x710;
/// `20.0` (`float`).
const TWENTY: u32 = 0x0101_7868;
/// `-1.0` (`double`).
const MINUS_ONE_DOUBLE: u32 = 0x0101_a6b0;

/// What `FISTP` with truncation leaves in the low word of the 64-bit result
/// (the indefinite value, whose low word is 0, when out of range).
fn x87_truncate_low_word(value: f32) -> u32 {
    let truncated = (value as f64).trunc();
    let limit = 2f64.powi(63);
    if !(-limit..limit).contains(&truncated) {
        0
    } else {
        truncated as i64 as u32
    }
}

// Translated from 00897de0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Makes the actor flee from `avoided`: unless the actor is excluded (extra
/// type tests `00437bf0` / `00437bd0`, `Actor` slot `0x22c`, byte `+0x118`)
/// it reuses its current package when that is a flee package (type `0x16`) or
/// makes a new `FleePackage`, adds `avoided` to the avoided references and
/// to the package's target (with `arg_7` as the target's count when it is not
/// negative), and gives the package a location: `location_ref`, else `cell`,
/// else the process's own (and then `FleePackage::FindTeleportDoor` runs
/// when the package has no teleport door yet). Finally the package's flags
/// and distance are set from `arg_3`, `arg_4` and the package's own float
/// (negated), and it is put on the actor (`Actor` slot `0x2f4`).
///
/// `arg_2` is cleared when `Actor` slot `0x214` is not 0. The compiler's
/// exception-unwinding frame is not translated.
#[allow(clippy::too_many_arguments)]
pub fn fn_00897de0(
    e: &mut Engine,
    this: Ptr<Actor>,
    avoided: Ptr,
    arg_2: u8,
    arg_3: u8,
    arg_4: u8,
    cell: Ptr,
    location_ref: Ptr,
    arg_7: f32,
    arg_8: f32,
) {
    if e.call(EXTRA_TYPE_IS_5, &args![this]).bool()
        || e.call(EXTRA_OBJECTS_TEST, &args![this]).bool()
        || e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool()
        || e.mem.u8(this.addr() + 0x118) != 0
    {
        return;
    }
    let mut arg_2 = arg_2;
    if e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).u32() != 0 {
        arg_2 = 0;
    }
    let current = e.call(GET_CURRENT_PACKAGE, &args![this]).ptr::<()>();
    let mut flee = Ptr::NULL;
    if !current.is_null() && e.call(PACKAGE_TYPE, &args![current]).i32() == 0x16 {
        flee = current;
    }
    if flee.is_null() {
        let memory = e.call(OPERATOR_NEW, &args![0xacu32]).ptr::<()>();
        flee = if memory.is_null() {
            Ptr::NULL
        } else {
            e.call(FLEE_PACKAGE_CONSTRUCT, &args![memory, avoided, 0u32, 0u32])
                .ptr()
        };
    }
    e.call(FLEE_PACKAGE_ADD_AVOIDED_REF, &args![flee, avoided]);
    let mut target = e.call(PACKAGE_GET_TARGET, &args![flee]).ptr::<()>();
    if target.is_null() {
        target = new_package_target(e);
        e.call(PACKAGE_SET_TARGET, &args![flee, target]);
    }
    e.call(PACKAGE_TARGET_SET_REFERENCE, &args![target, avoided]);
    let zero: f64 = e.global(ZERO_DOUBLE);
    if arg_7 as f64 >= zero {
        let count = e.call(FTOL, &args![arg_7 as f64]).u32();
        e.call(TARGET_SET_COUNT, &args![target, count]);
    }

    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    e.vcall(acquire.addr(), PROCESS_SLOT_SET_28, &args![]);
    let memory = e.call(OPERATOR_NEW, &args![0xcu32]).ptr::<()>();
    let location = if memory.is_null() {
        Ptr::NULL
    } else {
        e.call(PACKAGE_LOCATION_CONSTRUCT, &args![memory])
            .ptr::<()>()
    };
    if arg_8 as f64 >= zero {
        let radius = x87_truncate_low_word(arg_8);
        e.call(PACKAGE_LOCATION_SET_RADIUS, &args![location, radius]);
    }
    if !location_ref.is_null() {
        e.call(
            PACKAGE_LOCATION_SET_REFERENCE,
            &args![location, location_ref],
        );
        e.call(PACKAGE_SET_BYTE_80, &args![flee, 0u32]);
        e.call(PACKAGE_SET_LOCATION, &args![flee, location]);
        e.call(PACKAGE_SET_REFERENCE_994EF0, &args![flee, location_ref]);
    } else if !cell.is_null() {
        e.call(PACKAGE_LOCATION_SET_CELL, &args![location, cell]);
        e.call(PACKAGE_SET_BYTE_80, &args![flee, 0u32]);
        e.call(PACKAGE_SET_LOCATION, &args![flee, location]);
    } else {
        // (the interior test's result is not used)
        let actor_cell = e.call(REFR_PARENT_CELL, &args![this]).ptr::<()>();
        if !actor_cell.is_null() {
            let actor_cell = e.call(REFR_PARENT_CELL, &args![this]).ptr::<()>();
            e.call(CELL_FLAG_24_BIT_0, &args![actor_cell]);
        }
        if e.call(PACKAGE_FIELD_A4, &args![flee]).u32() == 0 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
            let value = e.vcall(acquire.addr(), PROCESS_SLOT_E4, &args![]).f32() as f64;
            if value <= zero {
                let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
                let twenty: f32 = e.global(TWENTY);
                e.vcall(acquire.addr(), PROCESS_SLOT_E8, &args![twenty]);
            }
            e.call(FLEE_PACKAGE_FIND_TELEPORT_DOOR, &args![flee, this, avoided]);
        }
    }
    if !location.is_null() {
        e.call(PACKAGE_LOCATION_DESTROY, &args![location, 1u32]);
    }

    if arg_2 == 0 {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
        e.vcall(acquire.addr(), PROCESS_SLOT_710, &args![this]);
    }
    fn_008982a0(e, flee, arg_3);
    e.call(PACKAGE_SET_FIELD_18, &args![flee, 0x13u32]);
    e.call(PACKAGE_SET_BYTE_94, &args![flee, 0u32]);
    let own_value = e.call(PACKAGE_FLOAT_90, &args![flee]).f32() as f64;
    let minus_one: f64 = e.global(MINUS_ONE_DOUBLE);
    fn_00898250(e, flee, (own_value * minus_one) as f32);
    fn_00898280(e, flee, arg_4);
    if arg_4 != 0 {
        e.call(PACKAGE_CALL_67A720, &args![flee, 0u32]);
        fn_00898200(e, flee, 0);
    }
    e.vcall(
        this.addr(),
        ACTOR_SLOT_ADD_PACKAGE,
        &args![flee, u32::from(arg_2), 1u32],
    );
}

// Translated from 00898200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit `0x2000000` of the package flags at
/// `+0x1c`.
pub fn fn_00898200(e: &mut Engine, this: Ptr, flag: u8) {
    let flags = e.mem.u32(this.addr() + 0x1c);
    let flags = if flag != 0 {
        flags | 0x0200_0000
    } else {
        flags & 0xfdff_ffff
    };
    e.mem.set_u32(this.addr() + 0x1c, flags);
}

// Translated from 00898250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `value` to the `float` at `+0x90`.
pub fn fn_00898250(e: &mut Engine, this: Ptr, value: f32) {
    let sum = e.mem.f32(this.addr() + 0x90) as f64 + value as f64;
    e.mem.set_f32(this.addr() + 0x90, sum as f32);
}

// Translated from 00898280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at `+0x81`.
pub fn fn_00898280(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x81, value);
}

// Translated from 008982a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at `+0xa8`.
pub fn fn_008982a0(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0xa8, value);
}

/// `BGSAmmoForm::IsRockItLauncher` (this = the weapon's ammo component at
/// `weapon + 0xa4`).
const AMMO_FORM_IS_ROCK_IT_LAUNCHER: u32 = 0x0047_4a80;
/// `PlayerCharacter::GetNumRockItAmmo`.
const PLAYER_GET_NUM_ROCK_IT_AMMO: u32 = 0x0096_9040;
/// `Interface::QueueMenuCreate` (cdecl, 6 words).
const INTERFACE_QUEUE_MENU_CREATE: u32 = 0x0070_9470;
/// Ammo component accessor (`00474a40`, `this` = weapon + 0xa4): the list of
/// ammunition forms, or 0.
const AMMO_COMPONENT_LIST: u32 = 0x0047_4a40;
/// `00500940`: list accessor on the ammunition list.
const AMMO_LIST_FIRST_NODE: u32 = 0x0050_0940;
/// `__RTDynamicCast` (cdecl: pointer, vfdelta, source type, target type,
/// is-reference).
const RTTI_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// Type descriptor of the cast source (`TESForm`).
const RTTI_TYPE_TES_FORM: u32 = 0x0118_3028;
/// Type descriptor the ammunition cast targets.
const RTTI_TYPE_AMMO_TARGET: u32 = 0x0118_40c4;
/// Type descriptor the idle-form cast targets.
const RTTI_TYPE_IDLE_TARGET: u32 = 0x0118_6a18;
/// `TESObjectREFR::GetInventoryItem` (form, 0).
const REFR_GET_INVENTORY_ITEM: u32 = 0x0057_6260;
/// Object the menu code asks about the weapon's sound (`this` = the global
/// pointed to by `0x011dea0c`).
const OBJECT_POINTER_011DEA0C: u32 = 0x011d_ea0c;
/// `00877720` on that object (no arguments).
const OBJECT_011DEA0C_GET: u32 = 0x0087_7720;
/// `00a24660` (object, 4, 1): non-zero when the player may play the
/// dry-fire feedback.
const DRY_FIRE_ALLOWED: u32 = 0x00a2_4660;
/// Weapon call (`005224c0`): the weapon's sound set object, or 0.
const WEAPON_SOUND_SET: u32 = 0x0052_24c0;
/// `00511840`: file name of a sound set object.
const SOUND_SET_FILE_NAME: u32 = 0x0051_1840;
/// `BSAudio::QInstance` (no arguments).
const AUDIO_INSTANCE: u32 = 0x0045_3a70;
/// `BSAudio::GetSoundHandleByFilename` (handle out, name, flags, sound set).
const AUDIO_GET_SOUND_HANDLE_BY_FILENAME: u32 = 0x00ad_7480;
/// `BSSoundHandle` constructor (`0041a250`).
const SOUND_HANDLE_CONSTRUCT: u32 = 0x0041_a250;
/// `BSSoundHandle` destructor (`00483710`).
const SOUND_HANDLE_DESTROY: u32 = 0x0048_3710;
/// `BSSoundHandle::IsValid`.
const SOUND_HANDLE_IS_VALID: u32 = 0x00ad_8ce0;
/// `BSSoundHandle::Play` (flag).
const SOUND_HANDLE_PLAY: u32 = 0x00ad_8830;
/// Assigns a sound handle (`00418900`: this = destination, source).
const SOUND_HANDLE_ASSIGN: u32 = 0x0041_8900;
/// Sets a sound handle's position (`0068a7d0`).
const SOUND_HANDLE_SET_POSITION: u32 = 0x0068_a7d0;
/// `ExtraDataList::GetSound` (out handle).
const EXTRA_GET_SOUND: u32 = 0x0041_8890;
/// `ExtraDataList::SetSound` (handle).
const EXTRA_SET_SOUND: u32 = 0x0041_a800;
/// Player weapon test (`00524d10`, `this` = the player).
const PLAYER_WEAPON_TEST: u32 = 0x0052_4d10;
/// Process virtual slot `0x3f4` (a flag).
const PROCESS_SLOT_3F4: u32 = 0x3f4;
/// `BGSEntryPoint::HandleEntryPoint` (cdecl: entry point, actor, weapon,
/// pointer to the value).
const HANDLE_ENTRY_POINT: u32 = 0x005e_58f0;
/// Weapon `float` accessor (`004e4620`).
const WEAPON_FLOAT: u32 = 0x004e_4620;
/// `TESObjectWEAP::GetAttackSpeed` (mod effect flag; `float` in ST0).
const WEAPON_GET_ATTACK_SPEED: u32 = 0x0064_6020;
/// Animation call taking a `float` speed (`004c0c90`).
const ANIMATION_SET_SPEED: u32 = 0x004c_0c90;
/// Global transform tests (`004937c0`, `004937e0`).
const TRANSFORM_TEST_4937C0: u32 = 0x0049_37c0;
const TRANSFORM_TEST_4937E0: u32 = 0x0049_37e0;
/// Sequence call (`00598040`), returning a `float`.
const SEQUENCE_VALUE_598040: u32 = 0x0059_8040;
/// Animation test (`00498290`) used by the special idle.
const ANIMATION_TEST_498290: u32 = 0x0049_8290;
/// `Animation::SpecialIdleLoaded`.
const ANIMATION_SPECIAL_IDLE_LOADED: u32 = 0x0049_81f0;
/// Call on the global object `0x011f2250` (`0059bb30`).
const OBJECT_011F2250_CALL: u32 = 0x0059_bb30;
/// Global object whose word at `+8` says whether a special idle is playing
/// (it is `4` then).
const SPECIAL_IDLE_STATE_OBJECT: u32 = 0x011f_2250;
/// `008b28c0`: actor call taking an animation group and the animation
/// (plays an attack through the animation).
const ACTOR_PLAY_GROUP_WORKER: u32 = 0x008b_28c0;
/// Third word of an animation group table entry (+0xc), indexed by the
/// global transform's group number.
const ANIM_GROUP_TABLE_KIND: u32 = 0x0119_77e4;
/// `RandomFloat` (cdecl: low, high `float`s; result in ST0).
const RANDOM_FLOAT: u32 = 0x0047_6b70;
/// `100.0f`-style upper bound the random roll uses (`float`).
const RANDOM_ROLL_MAX: u32 = 0x0101_6410;
/// Game settings (objects whose `float` `00403e20` returns a pointer to).
const SETTING_THRESHOLD_A: u32 = 0x011d_1380;
const SETTING_FACTOR_A: u32 = 0x011d_10f8;
const SETTING_THRESHOLD_B: u32 = 0x011d_1500;
const SETTING_FACTOR_B: u32 = 0x011d_0a0c;
/// Weapon accessors (`0051f5f0`, `00524b40`).
const WEAPON_SKILL_NUMBER: u32 = 0x0051_f5f0;
const WEAPON_BYTE_FLAG: u32 = 0x0052_4b40;
/// Animation accessor (`0047d3b0`) on the global transform: returns a `u16`.
const TRANSFORM_U16: u32 = 0x0047_d3b0;
/// `TESForm::GetFormByEditorID` (cdecl, name).
const GET_FORM_BY_EDITOR_ID: u32 = 0x0048_3a00;
/// The editor ID the 0xa9 group looks up.
const EDITOR_ID_STRING: u32 = 0x0108_4af0;
/// Process virtual slots used by `Actor::StartAttack`.
const PROCESS_SLOT_1B0: u32 = 0x1b0;
const PROCESS_SLOT_71C: u32 = 0x71c;
/// Format of the message `Actor::StartAttack` prints for a group that is not
/// an attack.
const START_NON_ATTACK_FORMAT: u32 = 0x0108_4a88;
/// `100.0` is not assumed: the roll maximum is read from memory. The actor
/// value owner's virtual slot `8` (int result).
const OWNER_SLOT_8: u32 = 0x08;

/// Prints the debug message for an animation group that is not an attack
/// (`Actor::QueueAttack` / `Actor::StartAttack`): actor name, weapon name,
/// the requested group's name and the picked group's type name.
fn report_group_mismatch(
    e: &mut Engine,
    this: Ptr<Actor>,
    weapon_form: Ptr,
    group: u16,
    requested: u32,
    format: u32,
) {
    let group_type = e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32();
    let group_type_name = anim_group_name(e, group_type);
    let requested_name = anim_group_name(e, requested);
    let weapon_name = if weapon_form.is_null() {
        HAND_TO_HAND_NAME
    } else if e.call(FORM_HAS_EDITOR_NAME, &args![weapon_form]).u32() != 0 {
        e.vcall(weapon_form.addr(), 0x130, &args![]).u32()
    } else {
        e.call(
            MAP_MARKER_DATA_GET_LOCATION_NAME,
            &args![weapon_form.byte_add(0x30)],
        )
        .u32()
    };
    let actor_name = if e.call(FORM_HAS_EDITOR_NAME, &args![this]).u32() != 0 {
        e.vcall(this.addr(), 0x130, &args![]).u32()
    } else {
        e.call(REFR_GET_NAME, &args![this]).u32()
    };
    e.call(
        DEBUG_PRINT,
        &args![
            format,
            actor_name,
            weapon_name,
            requested_name,
            group_type_name
        ],
    );
}

/// Plays the weapon's dry-fire sound on the player (`Actor::StartAttack`):
/// reuses the sound stored in the actor's extra data when it is valid,
/// otherwise looks one up in the weapon's sound set, positions and plays it,
/// and stores it back.
fn play_dry_fire_sound(e: &mut Engine, this: Ptr<Actor>, weapon_form: Ptr, player: Ptr<Actor>) {
    let sound = Ptr::<()>::new(e.mem.alloc(12));
    e.call(SOUND_HANDLE_CONSTRUCT, &args![sound]);
    let extra = extra_data_list_of(e, this.cast());
    e.call(EXTRA_GET_SOUND, &args![extra, sound]);
    if !e.call(SOUND_HANDLE_IS_VALID, &args![sound]).bool() {
        let flags_and_position = e.call(PLAYER_WEAPON_TEST, &args![player]).bool();
        let (sound_flags, with_position) = if flags_and_position {
            (0x102u32, true)
        } else {
            (0x20_0101u32, false)
        };
        let temp = Ptr::<()>::new(e.mem.alloc(12));
        let sound_set = e.call(WEAPON_SOUND_SET, &args![weapon_form]).u32();
        let sound_set_again = e.call(WEAPON_SOUND_SET, &args![weapon_form]).ptr::<()>();
        let file_name = e.call(SOUND_SET_FILE_NAME, &args![sound_set_again]).u32();
        let audio = e.call(AUDIO_INSTANCE, &args![]).ptr::<()>();
        let found = e
            .call(
                AUDIO_GET_SOUND_HANDLE_BY_FILENAME,
                &args![audio, temp, file_name, sound_flags, sound_set],
            )
            .ptr::<()>();
        e.call(SOUND_HANDLE_ASSIGN, &args![sound, found]);
        e.call(SOUND_HANDLE_DESTROY, &args![temp]);
        e.mem.free(temp.addr());
        if with_position {
            let position = e.vcall(this.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
            e.call(SOUND_HANDLE_SET_POSITION, &args![sound, position]);
        }
        e.call(SOUND_HANDLE_PLAY, &args![sound, 0u32]);
        let extra = extra_data_list_of(e, this.cast());
        e.call(EXTRA_SET_SOUND, &args![extra, sound]);
    }
    e.call(SOUND_HANDLE_DESTROY, &args![sound]);
    e.mem.free(sound.addr());
}

// Translated from 00893a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::StartAttack` (Xbox PDB): starts an attack with the animation group
/// `anim_group`. Returns true when the attack animation was started.
///
/// With a weapon that needs ammunition and none available (a Rock-It
/// launcher counts the player's Rock-It ammunition and opens a menu; other
/// weapons need the ammunition item to hold at least the weapon's per-shot
/// count) the player gets the dry-fire sound, process slot `0x3f4` is
/// called, and the attack fails. Otherwise a number of states make it fail
/// (animation actions 9 and 0x11, the weapon not drawn, process slot `0x3f8`
/// false, no animation, a package of type 8 or 0x10). A weapon that may jam,
/// or a power attack group `200..=216`, takes the jam / power group instead
/// and plays it through `008b28c0`. A melee-capable actor outside special
/// idles may have the group replaced by `0x38` or `0x3e` by a random roll
/// against two settings. The attack animation is then played in slot 4 with
/// the weapon's attack speed (entry point `0x2b`), after the checks the
/// animation's global transform imposes. A group that is not an attack is
/// only reported with a debug message.
///
/// The compiler's exception-unwinding frame (the `BSSoundHandle` temporaries)
/// is not translated.
pub fn actor_start_attack(e: &mut Engine, this: Ptr<Actor>, anim_group: u32) -> bool {
    let mut anim_group = anim_group;
    let player = player(e);
    let process = e.get(this, Actor::pCurrentProcess);
    let weapon_item = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
        .ptr::<()>();
    let weapon_form = if weapon_item.is_null() {
        Ptr::NULL
    } else {
        let weapon_item = e
            .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .ptr::<()>();
        entry_form(e, weapon_item)
    };
    let mut ammo_item = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_AMMO, &args![])
        .ptr::<()>();
    let mut shot_count = 0u32;
    if !weapon_form.is_null() {
        shot_count = u32::from(e.call(WEAPON_BYTE_102, &args![weapon_form]).u8());
    }

    if !weapon_form.is_null() {
        let mut out_of_ammo = false;
        let ammo_component = weapon_form.byte_add(0xa4);
        if e.call(AMMO_FORM_IS_ROCK_IT_LAUNCHER, &args![ammo_component])
            .bool()
        {
            out_of_ammo = true;
            if this == player && e.call(PLAYER_GET_NUM_ROCK_IT_AMMO, &args![player]).u32() != 0 {
                out_of_ammo = false;
            } else {
                e.call(
                    INTERFACE_QUEUE_MENU_CREATE,
                    &args![1u32, 0u32, 0u32, 0u32, 4u32, 0u32],
                );
            }
        } else if !e
            .call(WEAPON_GET_CURRENT_AMMO, &args![weapon_form, this])
            .ptr::<()>()
            .is_null()
        {
            if ammo_item.is_null() || e.call(LIST_NEXT, &args![ammo_item]).i32() < shot_count as i32
            {
                out_of_ammo = true;
            }
        } else if e.call(AMMO_COMPONENT_LIST, &args![ammo_component]).u32() != 0 {
            let list = e
                .call(AMMO_COMPONENT_LIST, &args![ammo_component])
                .ptr::<()>();
            let node = e.call(AMMO_LIST_FIRST_NODE, &args![list]).ptr::<()>();
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let form = e.mem.u32(slot);
            let ammo_form = e
                .call(
                    RTTI_DYNAMIC_CAST,
                    &args![form, 0u32, RTTI_TYPE_TES_FORM, RTTI_TYPE_AMMO_TARGET, 0u32],
                )
                .ptr::<()>();
            if !ammo_form.is_null() {
                ammo_item = e
                    .call(REFR_GET_INVENTORY_ITEM, &args![this, ammo_form, 0u32])
                    .ptr();
                if ammo_item.is_null()
                    || e.call(LIST_NEXT, &args![ammo_item]).i32() < shot_count as i32
                {
                    out_of_ammo = true;
                }
            }
        }
        if out_of_ammo && !ammo_item.is_null() && e.call(LIST_NEXT, &args![ammo_item]).u32() != 0 {
            out_of_ammo = false;
        }

        if out_of_ammo {
            let object = e.global::<u32>(OBJECT_POINTER_011DEA0C);
            let sound_object = e.call(OBJECT_011DEA0C_GET, &args![object]).u32();
            if e.call(WEAPON_SOUND_SET, &args![weapon_form]).u32() != 0 {
                let is_player = this == player;
                if is_player
                    && e.call(DRY_FIRE_ALLOWED, &args![sound_object, 4u32, 1u32])
                        .u32()
                        == 0
                {
                    return false;
                }
                if is_player {
                    play_dry_fire_sound(e, this, weapon_form, player);
                }
            }
            let process = e.get(this, Actor::pCurrentProcess);
            e.vcall(process.addr(), PROCESS_SLOT_3F4, &args![0u32]);
            return false;
        }
    }

    if e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() == 9
        || e.call(ACTOR_GET_ANIM_ACTION, &args![this]).i32() == 0x11
    {
        return false;
    }
    if !e.call(ACTOR_IS_WEAPON_DRAWN, &args![this]).bool() {
        return false;
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if !e.vcall(process.addr(), PROCESS_SLOT_3F8, &args![]).bool() {
        return false;
    }
    let animation = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .ptr::<()>();
    if animation.is_null() {
        return false;
    }

    // The package of the process decides whether the weapon may be used
    // (types 8 and 0x10 stop it).
    let mut package_allows = true;
    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
    let package = e
        .vcall(acquire.addr(), PROCESS_SLOT_27C, &args![])
        .ptr::<()>();
    if !package.is_null() {
        package_allows = e.call(PACKAGE_TYPE, &args![package]).i32() != 8
            && e.call(PACKAGE_TYPE, &args![package]).i32() != 0x10;
    }
    if package_allows
        && !weapon_form.is_null()
        && !e
            .call(WEAPON_GET_CURRENT_AMMO, &args![weapon_form, this])
            .ptr::<()>()
            .is_null()
    {
        let process = e.get(this, Actor::pCurrentProcess);
        let condition = e
            .vcall(process.addr(), PROCESS_SLOT_368, &args![this])
            .u32();
        let is_power_group = (200..=216).contains(&(anim_group as i32));
        let mut take_special_group = is_power_group;
        if !is_power_group {
            let process = e.get(this, Actor::pCurrentProcess);
            if !e.vcall(process.addr(), PROCESS_SLOT_6DC, &args![]).bool() {
                let jam_multiplier = e
                    .call(GET_WEAPON_CONDITION_JAM_MULT, &args![condition])
                    .f32() as f64;
                let zero: f64 = e.global(ZERO_DOUBLE);
                if jam_multiplier > zero {
                    let jam_multiplier = e
                        .call(GET_WEAPON_CONDITION_JAM_MULT, &args![condition])
                        .f32();
                    if e.call(CHANCE_ROLL, &args![jam_multiplier]).bool() {
                        take_special_group = true;
                    }
                }
            }
        }
        if take_special_group {
            let special_group = if is_power_group {
                anim_group
            } else {
                e.call(WEAPON_GET_ATTACK_ANIM, &args![weapon_form, 0u32])
                    .u32()
                    .wrapping_add(0x17)
            };
            let group = actor_get_anim_group(e, this, special_group, 0, 0, Ptr::NULL);
            if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32() == special_group {
                e.call(
                    ACTOR_PLAY_GROUP_WORKER,
                    &args![this, special_group, animation],
                );
                let sequence = e
                    .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
                    .u32();
                e.call(ACTOR_SET_ANIM_ACTION, &args![this, 9u32, sequence]);
                e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![u32::from(group), 1u32]);
            }
            return false;
        }
    }

    // Melee roll: a strong enough actor value may turn the attack into group
    // 0x38 or 0x3e.
    let special_idle = e.call(ENTRY_FORM, &args![SPECIAL_IDLE_STATE_OBJECT]).u32() == 4;
    let moving = e.call(ACTOR_IS_MOVING, &args![this]).bool();
    if (!moving || special_idle)
        && (e.vcall(this.addr(), ACTOR_SLOT_218, &args![]).bool() || this == player)
        && (weapon_form.is_null() || e.call(WEAPON_TYPE, &args![weapon_form]).i32() == 0)
    {
        let mut is_ranged_skill = false;
        if !weapon_form.is_null() {
            let skill = e.call(WEAPON_SKILL_NUMBER, &args![weapon_form]).u32();
            if (0x4a..=0x5b).contains(&skill) {
                is_ranged_skill = true;
            }
        }
        let owner_value = e
            .vcall(this.addr() + 0xa4, OWNER_SLOT_8, &args![0x2du32])
            .i32();
        let owner_value = owner_value as f32;
        let mut rolled = false;
        let mut first_chance = 0.0f32;
        let mut second_chance = 0.0f32;
        if !is_ranged_skill {
            let pointer = e
                .call(SETTING_GET_VALUE_POINTER, &args![SETTING_THRESHOLD_A])
                .u32();
            let threshold = e.mem.f32(pointer);
            if (threshold as f64) < owner_value as f64 {
                rolled = true;
                let pointer = e
                    .call(SETTING_GET_VALUE_POINTER, &args![SETTING_FACTOR_A])
                    .u32();
                let factor = e.mem.f32(pointer);
                first_chance = (owner_value as f64 * factor as f64) as f32;
            }
        }
        if !is_ranged_skill {
            let pointer = e
                .call(SETTING_GET_VALUE_POINTER, &args![SETTING_THRESHOLD_B])
                .u32();
            let threshold = e.mem.f32(pointer);
            if (threshold as f64) < owner_value as f64 {
                rolled = true;
                let pointer = e
                    .call(SETTING_GET_VALUE_POINTER, &args![SETTING_FACTOR_B])
                    .u32();
                let factor = e.mem.f32(pointer);
                second_chance = (owner_value as f64 * factor as f64) as f32;
            }
        }
        if rolled && !special_idle {
            let roll_max: f32 = e.global(RANDOM_ROLL_MAX);
            let roll = e.call(RANDOM_FLOAT, &args![0.0f32, roll_max]).f32();
            if first_chance as f64 >= roll as f64 {
                anim_group = 0x38;
            } else if (roll as f64) < first_chance as f64 + second_chance as f64 {
                anim_group = 0x3e;
            }
        }
    }

    let mut special_idle_ready = false;
    if special_idle
        && e.call(OBJECT_011F2250_CALL, &args![SPECIAL_IDLE_STATE_OBJECT])
            .u32()
            != 0
        && e.call(ANIMATION_SPECIAL_IDLE_LOADED, &args![animation])
            .bool()
    {
        special_idle_ready = true;
    }
    let group = actor_get_anim_group(e, this, anim_group, 0, 0, Ptr::NULL);
    if !e
        .call(ANIM_GROUP_IS_ATTACK_ACTION, &args![u32::from(group)])
        .bool()
        && !special_idle_ready
    {
        report_group_mismatch(
            e,
            this,
            weapon_form,
            group,
            anim_group,
            START_NON_ATTACK_FORMAT,
        );
        return false;
    }

    // The attack animation.
    let mut started = true;
    if !weapon_form.is_null() {
        let mut attack_speed = e.call(WEAPON_FLOAT, &args![weapon_form]).f32();
        let speed_pointer = e.mem.alloc(4);
        e.mem.set_f32(speed_pointer, attack_speed);
        e.call(
            HANDLE_ENTRY_POINT,
            &args![0x2bu32, this, weapon_form, speed_pointer],
        );
        attack_speed = e.mem.f32(speed_pointer);
        e.mem.free(speed_pointer);
        let process = e.get(this, Actor::pCurrentProcess);
        let current = e
            .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .ptr::<()>();
        let has_mod_effect = e
            .call(ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, &args![current, 8u32])
            .u8();
        let base_speed = e
            .call(
                WEAPON_GET_ATTACK_SPEED,
                &args![weapon_form, u32::from(has_mod_effect)],
            )
            .f32() as f64;
        let speed = (base_speed * attack_speed as f64) as f32;
        e.call(ANIMATION_SET_SPEED, &args![animation, speed]);
    } else {
        e.call(ANIMATION_SET_SPEED, &args![animation, 1.0f32]);
    }

    let sequence = e
        .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
        .ptr::<()>();
    if sequence.is_null() {
        return false;
    }
    let transform = e
        .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
        .ptr::<()>();
    if e.call(TRANSFORM_TEST_4937C0, &args![transform]).bool() {
        if e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![sequence])
            .u32()
            != 1
        {
            return false;
        }
        for slot in [5u32, 6u32] {
            if e.call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
                .u32()
                != 0
            {
                let other = e
                    .call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
                    .ptr::<()>();
                if e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![other]).u32() != 1 {
                    return false;
                }
            }
        }
        if this != player && e.vcall(this.addr(), ACTOR_SLOT_218, &args![]).bool() {
            let transform = e
                .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                .ptr::<()>();
            let moves = e.call(ANIM_GROUP_GET_MOVE, &args![transform]).u32();
            if moves == 1 {
                if !e.call(ACTOR_IS_MOVING, &args![this]).bool() {
                    return false;
                }
            } else if e.call(ACTOR_IS_MOVING, &args![this]).bool() {
                return false;
            }
        }
    } else {
        let transform = e
            .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
            .ptr::<()>();
        if e.call(TRANSFORM_TEST_4937E0, &args![transform]).bool() {
            let transform = e
                .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                .ptr::<()>();
            let number = e.call(ANIM_GROUP_GET_NUMBER, &args![transform]).u32();
            let kind = e
                .mem
                .u32(ANIM_GROUP_TABLE_KIND.wrapping_add(number.wrapping_mul(0x24)));
            match kind {
                1 | 9 => {}
                5 => {
                    let power_with_zero_time = e
                        .call(ANIM_GROUP_IS_POWER_ATTACK_ACTION, &args![u32::from(group)])
                        .bool()
                        && {
                            let time = e.call(SEQUENCE_VALUE_598040, &args![sequence]).f32() as f64;
                            time == 0.0
                        };
                    if !power_with_zero_time
                        && e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32() != 3
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }

    // Play it, unless that very group is already playing in slot 4.
    let playing = e
        .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
        .u16();
    let already_playing =
        playing == group && [0x4a, 0x4d, 0x50, 0x53, 0x56, 0x59].contains(&(anim_group as i32));
    if !already_playing {
        if special_idle_ready && anim_group < 0x61 && anim_group > 0x64 {
            started = e.call(ANIMATION_TEST_498290, &args![animation]).u8() != 0;
        } else if e
            .call(ANIM_GROUP_IS_POWER_ATTACK_ACTION, &args![u32::from(group)])
            .bool()
        {
            let played = e
                .call(
                    ANIMATION_PLAY_GROUP,
                    &args![
                        animation,
                        u32::from(group),
                        1u32,
                        0xffff_ffffu32,
                        0xffff_ffffu32
                    ],
                )
                .u32();
            started = played != 0;
        } else {
            e.call(ACTOR_PLAY_GROUP_WORKER, &args![this, anim_group, animation]);
        }
        let attack_sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
            .ptr::<()>();
        if !attack_sequence.is_null() {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
            let action = if e.vcall(acquire.addr(), PROCESS_SLOT_1B0, &args![]).bool() {
                5u32
            } else {
                2u32
            };
            e.call(ACTOR_SET_ANIM_ACTION, &args![this, action, attack_sequence]);
            if anim_group != 0xa9 {
                let transform = e
                    .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![attack_sequence])
                    .ptr::<()>();
                let value = e.call(TRANSFORM_U16, &args![transform]).u16();
                e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![u32::from(value), 1u32]);
            } else {
                let form = e
                    .call(GET_FORM_BY_EDITOR_ID, &args![EDITOR_ID_STRING])
                    .u32();
                let idle = e
                    .call(
                        RTTI_DYNAMIC_CAST,
                        &args![form, 0u32, RTTI_TYPE_TES_FORM, RTTI_TYPE_IDLE_TARGET, 0u32],
                    )
                    .u32();
                let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
                e.vcall(acquire.addr(), PROCESS_SLOT_71C, &args![idle]);
                let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
                e.vcall(acquire.addr(), PROCESS_SLOT_614, &args![0x80u32]);
            }
        }
    }

    if !weapon_form.is_null() && e.call(WEAPON_BYTE_FLAG, &args![weapon_form]).bool() {
        let slot_four_group = e
            .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
            .u16();
        let group_type = e
            .call(ANIM_GROUP_GET_TYPE, &args![u32::from(slot_four_group)])
            .u16();
        if (0x18..=0xa8).contains(&group_type) {
            let sequence = e
                .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
                .ptr::<()>();
            if e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![sequence])
                .u32()
                == 1
            {
                let process = e.get(this, Actor::pCurrentProcess);
                e.vcall(process.addr(), PROCESS_SLOT_614, &args![1u32]);
                let process = e.get(this, Actor::pCurrentProcess);
                e.vcall(process.addr(), PROCESS_SLOT_3F4, &args![0u32]);
            }
        }
    }
    started
}

/// `Actor` first-person check (`004eaf60`, `this` = the player).
const PLAYER_FIRST_PERSON_CHECK: u32 = 0x004e_af60;
/// `"AI: Don't call Actor::PickAnimations on the 1st person pc."`.
const FIRST_PERSON_MESSAGE: u32 = 0x0108_4b18;
/// `MobileObject::GetCharController`.
const MOBILE_OBJECT_GET_CHAR_CONTROLLER: u32 = 0x0093_06d0;
/// `PlayerCharacter::GetBiped` (flag).
const PLAYER_GET_BIPED: u32 = 0x0095_0b00;
/// `TESAnimGroup::IsJumpingLoopAnim` (this = the animation; the sequence is
/// the argument; result is a `float` in ST0).
const ANIM_GROUP_IS_JUMPING_LOOP_ANIM: u32 = 0x0049_3800;
/// Animation accessor (`005585e0`) used before `ClearGunWobble`.
const ANIMATION_GUN_WOBBLE_SOURCE: u32 = 0x0055_85e0;
/// `ClearGunWobble` (cdecl: table value, the accessor's result).
const CLEAR_GUN_WOBBLE: u32 = 0x008d_6a80;
/// `Actor::SetIronSights` (three words).
const ACTOR_SET_IRON_SIGHTS: u32 = 0x008b_b650;
/// Process virtual slot `0x190` (`MiddleHighProcess::GetWeaponBone` in the
/// high process vtable; takes the biped).
const PROCESS_SLOT_WEAPON_BONE: u32 = 0x190;
/// `0045bc00` (this = the bone, one word argument).
const BONE_NODE_LOOKUP: u32 = 0x0045_bc00;
/// Returns the name `NiObjectNET::GetExtraData` is asked for (`00448a80`).
const EXTRA_DATA_NAME: u32 = 0x0044_8a80;
/// `NiObjectNET::GetExtraData` (name).
const NI_OBJECT_NET_GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
/// `00c74890` (this = `*(actor + 0xac)`; index, value).
const SET_SHADER_VALUE: u32 = 0x00c7_4890;
/// Process virtual slots used by `Actor::PickAnimations`.
const PROCESS_SLOT_QUEUED_ACTION: u32 = 0x3e4;
const PROCESS_SLOT_ANIM_SEQUENCE: u32 = 0x3e8;
const PROCESS_SLOT_SIT_SLEEP_STATE: u32 = 0x4bc;
const PROCESS_SLOT_FURNITURE_MARKER: u32 = 0x4d4;
const PROCESS_SLOT_AUTOMATIC_SHOT_DELAY: u32 = 0x444;
const PROCESS_SLOT_WEAPON_DRAWN: u32 = 0x454;
const PROCESS_SLOT_SET_WEAPON_DRAWN: u32 = 0x458;
const PROCESS_SLOT_SHADER_EFFECTS: u32 = 0x580;
const PROCESS_SLOT_WEAPON_ENCHANTMENT_VISUALS: u32 = 0x574;
const PROCESS_SLOT_SAVE_WEAPON_LAST_POSITION: u32 = 0x45c;
/// Actor virtual slot used by `Actor::PickAnimations` (see also `ACTOR_SLOT_234`).
const ACTOR_SLOT_230: u32 = 0x230;
/// Actor virtual slot `0x390` (non-zero while running).
const ACTOR_SLOT_RUNNING: u32 = 0x390;
/// `PlayerCharacter` virtual slot `0x358` (no arguments).
const PLAYER_SLOT_358: u32 = 0x358;
/// `ProcessLists::FindAndCleanupWeaponShaderHitEffect` (actor, effect).
const FIND_AND_CLEANUP_WEAPON_SHADER_HIT_EFFECT: u32 = 0x0097_4db0;
/// `MagicShaderHitEffect::ResetAlphaTimer`.
const RESET_ALPHA_TIMER: u32 = 0x0082_16c0;
/// `TESEnchantableForm::GetFormEnchanting` (cdecl, the weapon form).
const GET_FORM_ENCHANTING: u32 = 0x004b_e330;
/// `ItemChange::GetPoison`.
const ITEM_CHANGE_GET_POISON: u32 = 0x004b_dcc0;
/// `MobileObject::PlaySoundByEditorName` (out handle, name, 0, flags, 1).
const PLAY_SOUND_BY_EDITOR_NAME: u32 = 0x0093_3270;
/// `"WPNBlade1HandEquipEnchanted"`.
const ENCHANTED_EQUIP_SOUND_NAME: u32 = 0x0108_4afc;
/// `Actor::GetRunSpeed` (returns a `float` in ST0).
const ACTOR_GET_RUN_SPEED: u32 = 0x0088_4eb0;
/// Controller call (`005c0880`): the jump / fall state, 0 to 2.
const CONTROLLER_STATE: u32 = 0x005c_0880;
/// Controller call (`0088b0f0`, a `float` in ST0).
const CONTROLLER_FALL_TIME: u32 = 0x0088_b0f0;
/// Sequence test (`005f4d60`) on the global transform.
const TRANSFORM_IS_LOOPING: u32 = 0x005f_4d60;
/// The game setting object compared against the fall time.
const FALL_TIME_SETTING: u32 = 0x011d_f72c;
/// `Animation` call without arguments (`004974a0`).
const ANIMATION_LAND_RESET: u32 = 0x0049_74a0;
/// Offset of the object `0059bb30` is called on in the char controller.
const CONTROLLER_LAND_SOUND_OFFSET: u32 = 0x410;
/// `ImpactMixer::PlayJumpLand` (cdecl: actor, sound).
const IMPACT_MIXER_PLAY_JUMP_LAND: u32 = 0x0083_91b0;
/// `Actor::CanMove`.
const ACTOR_CAN_MOVE: u32 = 0x0088_43a0;
/// `Animation::SpecialIdleDonePlaying`.
const ANIMATION_SPECIAL_IDLE_DONE_PLAYING: u32 = 0x0049_85f0;
/// `Animation::SpecialIdlePlaying`.
const ANIMATION_SPECIAL_IDLE_PLAYING: u32 = 0x0049_85b0;
/// `Animation::SpecialIdleFree` (two flags).
const ANIMATION_SPECIAL_IDLE_FREE: u32 = 0x0049_8910;
/// `Animation::GroupLoaded`.
const ANIMATION_GROUP_LOADED: u32 = 0x0049_4710;
/// `TESAnimGroup::GetMove_ov2` (cdecl, one word).
const ANIM_GROUP_GET_MOVE_OV2: u32 = 0x005f_23c0;
/// Group test (`005f4cc0`, cdecl, one word).
const ANIM_GROUP_TEST_4CC0: u32 = 0x005f_4cc0;
/// Group test (`005f2700`, cdecl, one word).
const ANIM_GROUP_TEST_2700: u32 = 0x005f_2700;
/// Animation accessor (`00498cf0`).
const ANIMATION_CURRENT_BLEND_SEQUENCE: u32 = 0x0049_8cf0;
/// `Actor::GetWeaponPosition` (out `NiPoint3`, flag).
const ACTOR_GET_WEAPON_POSITION: u32 = 0x008a_5340;
/// `1.0` (`double`).
const ONE_DOUBLE: u32 = 0x0101_2070;
/// Third entry word of the group table: how the group's sequence is used
/// (`+8`), indexed by group number times `0x24`.
const ANIM_GROUP_TABLE_MODE: u32 = 0x0119_77e0;
/// `CombatFormulas` call (`006477e0`, cdecl, the weapon condition word;
/// result in ST0).
const WEAPON_CONDITION_MULT_2: u32 = 0x0064_77e0;

/// The "slot mode" word of an animation group number (which animation slot
/// the group plays in).
fn group_mode(e: &Engine, number: u32) -> u32 {
    e.mem
        .u32(ANIM_GROUP_TABLE_MODE.wrapping_add(number.wrapping_mul(0x24)))
}

/// The "kind" word of an animation group number.
fn group_kind(e: &Engine, number: u32) -> u32 {
    e.mem
        .u32(ANIM_GROUP_TABLE_KIND.wrapping_add(number.wrapping_mul(0x24)))
}

/// The animation group number of an animation sequence
/// (`005f2420(Animation::ZeroGlobalTransform(sequence))`).
fn sequence_group_number(e: &mut Engine, sequence: Ptr) -> u32 {
    let transform = e
        .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
        .ptr::<()>();
    e.call(ANIM_GROUP_GET_NUMBER, &args![transform]).u32()
}

/// The sequence the process currently plays (`HighProcess::GetAnimActionAnimSeq`
/// through virtual slot `0x3e8`).
fn process_sequence(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), PROCESS_SLOT_ANIM_SEQUENCE, &args![])
        .ptr()
}

fn set_anim_action(e: &mut Engine, this: Ptr<Actor>, action: u32, sequence: Ptr) {
    e.call(ACTOR_SET_ANIM_ACTION, &args![this, action, sequence]);
}

/// `true` when either `Actor::Test8BA3E0` or `Actor::Test8BA410` is set.
fn in_special_state(e: &mut Engine, this: Ptr<Actor>) -> bool {
    e.call(ACTOR_TEST_8BA3E0, &args![this]).bool() || e.call(ACTOR_TEST_8BA410, &args![this]).bool()
}

/// `PickBestAnimation(AnimGroup(selector_a, selector_b, group_type, special), 0)`.
fn compose_group(
    e: &mut Engine,
    animation: Ptr,
    selector_a: u32,
    selector_b: u32,
    group_type: u32,
    special: bool,
) -> u16 {
    let composed = e
        .call(
            ANIM_GROUP_COMPOSE,
            &args![selector_a, selector_b, group_type, u32::from(special)],
        )
        .u16();
    e.call(
        ANIMATION_PICK_BEST_ANIMATION,
        &args![animation, u32::from(composed), 0u32],
    )
    .u16()
}

/// Group `(group & 0xff00) | low` (the movement group of another speed).
fn movement_group(group: u16, low: u16) -> u16 {
    (group & 0xff00) | low
}

/// For a running actor, a movement group is mapped to the run (`|7`) or walk
/// (`|3`) variant (tables at `0x897848` / `0x89787c`: types 4, 5, 6 and 11 to
/// 14 walk, 8 to 10 run, 7 unchanged).
fn running_variant(e: &mut Engine, group: u16) -> u16 {
    let group_type = e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32();
    match group_type {
        4..=6 | 0xb..=0xe => movement_group(group, 3),
        8..=0xa => movement_group(group, 7),
        _ => group,
    }
}

/// Plays the biped/animation pairs of the actor (and for the player its
/// first-person pair) through process slot `0x1cc`.
fn force_weapon_drawn_pairs(
    e: &mut Engine,
    this: Ptr<Actor>,
    animation: Ptr,
    biped: u32,
    drawn: u32,
) {
    let player = player(e);
    let mut current_animation = animation;
    let mut current_biped = biped;
    let mut count = if this == player { 2 } else { 1 };
    while count != 0 {
        if this == player && count == 1 {
            current_animation = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).ptr();
            current_biped = e.call(PLAYER_GET_BIPED, &args![player, 1u32]).u32();
        }
        let process = e.get(this, Actor::pCurrentProcess);
        e.vcall(
            process.addr(),
            PROCESS_SLOT_FORCE_WEAPON_DRAWN,
            &args![drawn, current_biped, current_animation, this],
        );
        count -= 1;
    }
}

fn process_of(e: &Engine, this: Ptr<Actor>) -> Ptr {
    e.get(this, Actor::pCurrentProcess)
}

/// The check `Actor::PickAnimations` makes first on the actor's animation and
/// then, for the player, on the first-person animation: if the animation's
/// attack sequence (slot 4) disagrees with the actor about moving and its
/// group differs from the group `Actor::GetAnimGroup` would pick now, the
/// slot is blended out; or when that sequence plays a group of type `0x4a` or
/// `0x4d` with no automatic shot delay it is blended out.
/// `group_animation` is the animation `GetAnimGroup` gets (null for the
/// actor's own).
fn blend_out_mismatched_attack(
    e: &mut Engine,
    this: Ptr<Actor>,
    animation: Ptr,
    group_animation: Ptr,
) {
    let sequence = e
        .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
        .ptr::<()>();
    let mut check_looping_group = true;
    if !sequence.is_null() {
        let transform = e
            .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
            .ptr::<()>();
        let group_moves = e.call(ANIM_GROUP_GET_MOVE, &args![transform]).u32() == 1;
        let actor_moves = e.call(ACTOR_IS_MOVING, &args![this]).u8();
        if u8::from(group_moves) != actor_moves
            && e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![sequence])
                .u32()
                == 1
        {
            let transform = e
                .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                .ptr::<()>();
            if e.call(TRANSFORM_TEST_4937C0, &args![transform]).bool() {
                let playing = e
                    .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
                    .u16();
                let number = sequence_group_number(e, sequence);
                let picked = actor_get_anim_group(e, this, number, 0, 0, group_animation);
                if playing != picked {
                    e.call(ANIMATION_BLEND_OUT, &args![animation, 4u32, 0u32]);
                }
                check_looping_group = false;
            }
        }
    }
    if check_looping_group {
        let first = e
            .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
            .u16();
        let mut matches = e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(first)]).u32() == 0x4a;
        if !matches {
            let second = e
                .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
                .u16();
            matches = e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(second)]).u32() == 0x4d;
        }
        if matches {
            let process = process_of(e, this);
            let shot_delay = e
                .vcall(process.addr(), PROCESS_SLOT_AUTOMATIC_SHOT_DELAY, &args![])
                .f32() as f64;
            if shot_delay == 0.0
                && e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![sequence])
                    .u32()
                    == 1
            {
                e.call(ANIMATION_BLEND_OUT, &args![animation, 4u32, 0u32]);
            }
        }
    }
}

// Translated from 00895110 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::PickAnimations` (Xbox PDB): the actor's animation state machine.
/// It blends out stale attack animations, reads the process's queued action
/// (`HighProcess::GetAnimAction`, virtual slot `0x3e4`) and its sequence
/// (slot `0x3e8`), and when the sequence finished (`GetGenericLocation`
/// non-zero) advances the action (weapon drawing and sheathing, reloading,
/// equipping, aiming, jumping and landing, post-attack states: the arms of
/// the `switch` on the queued action). Otherwise, with nothing queued, it
/// picks a locomotion or idle group from the actor's movement flags, sit/sleep
/// state and weapon, composes it with `TESAnimGroup::AnimGroup` and
/// `Animation::PickBestAnimation`, scales the animation speed (`arg_2` for
/// locomotion; `arg_3` for the two turn-in-place groups `0xf` and `0x10`),
/// and plays it in the slot the group table names.
///
/// `arg_2` and `arg_3` are the two `float` stack arguments (`RET 8`). The
/// compiler's exception-unwinding frame is not translated.
pub fn actor_pick_animations(e: &mut Engine, this: Ptr<Actor>, arg_2: f32, arg_3: f32) {
    let player = player(e);
    if e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
        return;
    }
    if e.vcall(this.addr(), ACTOR_SLOT_230, &args![]).bool() {
        return;
    }
    if this == player && !e.call(PLAYER_FIRST_PERSON_CHECK, &args![player]).bool() {
        e.call(DEBUG_PRINT, &args![FIRST_PERSON_MESSAGE]);
        return;
    }
    let animation = e
        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
        .ptr::<()>();
    let controller = e
        .call(MOBILE_OBJECT_GET_CHAR_CONTROLLER, &args![this])
        .ptr::<()>();
    if animation.is_null() || process_of(e, this).is_null() || controller.is_null() {
        return;
    }
    let process = process_of(e, this);
    if e.call(ACQUIRE_OBJECT_FIELD_28, &args![process]).u32() == 0 {
        e.call(SOUND_HANDLE_DESTROY, &args![process]);
    }

    blend_out_mismatched_attack(e, this, animation, Ptr::NULL);
    if this == player {
        let player_animation = e
            .call(PLAYER_GET_ANIMATION, &args![player, 1u32])
            .ptr::<()>();
        blend_out_mismatched_attack(e, this, player_animation, player_animation);
    }

    let mut speed_scale = 1.0f32;
    let mut move_speed = 0.0f32;
    let is_special = e.call(ACTOR_TEST_8A6970, &args![this]).u8() != 0;
    let mut group_type = 0u32;
    let mut selector_a = 0u32;
    let mut selector_b = 0u32;
    let mut new_action: i32 = -1;
    let slot1_sequence = e
        .call(ANIMATION_GET_SEQUENCE, &args![animation, 1u32])
        .ptr::<()>();
    let biped = e.vcall(this.addr(), ACTOR_SLOT_1E8, &args![]).u32();
    let process = process_of(e, this);
    let weapon_item = e
        .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
        .ptr::<()>();
    let weapon_form = if weapon_item.is_null() {
        Ptr::NULL
    } else {
        entry_form(e, weapon_item)
    };
    let flags = e.call(ACTOR_ANIM_FLAGS, &args![this]).u16();
    let process = process_of(e, this);
    let mut queued_action = e
        .vcall(process.addr(), PROCESS_SLOT_QUEUED_ACTION, &args![])
        .i32();
    let process = process_of(e, this);
    let sit_state = e
        .vcall(process.addr(), PROCESS_SLOT_SIT_SLEEP_STATE, &args![])
        .i32();
    let process = process_of(e, this);
    let furniture = e
        .vcall(process.addr(), PROCESS_SLOT_FURNITURE_MARKER, &args![])
        .ptr::<()>();

    if queued_action != -1 {
        let mut switch_taken = false;
        let process = process_of(e, this);
        if e.vcall(process.addr(), PROCESS_SLOT_ANIM_SEQUENCE, &args![])
            .u32()
            != 0
        {
            let sequence = process_sequence(e, this);
            if e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![sequence])
                .u32()
                != 0
            {
                switch_taken = true;
                match queued_action as u32 {
                    0x11 => {
                        let sequence = process_sequence(e, this);
                        let number = sequence_group_number(e, sequence);
                        let process = process_of(e, this);
                        if is_special
                            && !e
                                .vcall(process.addr(), PROCESS_SLOT_WEAPON_DRAWN, &args![])
                                .bool()
                            && !weapon_form.is_null()
                            && !weapon_item.is_null()
                        {
                            let mod_active = e
                                .call(ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, &args![weapon_item, 2u32])
                                .u8();
                            e.vcall(
                                this.addr(),
                                ACTOR_SLOT_3EC,
                                &args![weapon_form, 0u32, u32::from(mod_active), 1u32],
                            );
                            new_action = -1;
                            set_anim_action(e, this, 0xffff_ffff, Ptr::NULL);
                        } else if number as i32 <= 0xb0 {
                            let loop_time = e
                                .call(ANIM_GROUP_IS_JUMPING_LOOP_ANIM, &args![animation, sequence])
                                .f32() as f64;
                            let transform = e
                                .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                                .ptr::<()>();
                            let time =
                                e.call(ANIM_GROUP_GET_TIME, &args![transform, 1u32]).f32() as f64;
                            if time <= loop_time {
                                let mod_active = e
                                    .call(
                                        ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE,
                                        &args![weapon_item, 2u32],
                                    )
                                    .u8();
                                let land_group = e
                                    .call(
                                        WEAPON_GET_ATTACK_ANIM,
                                        &args![weapon_form, u32::from(mod_active)],
                                    )
                                    .u32();
                                let group =
                                    actor_get_anim_group(e, this, land_group, 0, 0, Ptr::NULL);
                                if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32()
                                    == land_group
                                {
                                    let own_animation = e
                                        .vcall(this.addr(), ACTOR_SLOT_GET_ANIMATION, &args![])
                                        .ptr::<()>();
                                    e.call(
                                        ANIMATION_PLAY_GROUP,
                                        &args![
                                            own_animation,
                                            u32::from(group),
                                            1u32,
                                            0xffff_ffffu32,
                                            0xffff_ffffu32
                                        ],
                                    );
                                    if this == player {
                                        e.vcall(
                                            player.addr(),
                                            ACTOR_SLOT_4B0,
                                            &args![u32::from(group), 1u32],
                                        );
                                    }
                                }
                                new_action = 0x11;
                                let attack_sequence = e
                                    .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
                                    .ptr::<()>();
                                set_anim_action(e, this, 0x11, attack_sequence);
                            }
                        } else if e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32() == 1 {
                            let process = process_of(e, this);
                            e.vcall(process.addr(), PROCESS_SLOT_614, &args![0x4_0000u32]);
                        }
                    }
                    5 => {
                        let mut wanted_state = 2;
                        let sequence = process_sequence(e, this);
                        let number = sequence_group_number(e, sequence);
                        let kind = group_kind(e, number);
                        let applies = match kind {
                            7 => true,
                            8 => {
                                wanted_state = 1;
                                true
                            }
                            _ => false,
                        };
                        if applies
                            && e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32()
                                == wanted_state
                        {
                            if !weapon_form.is_null()
                                && !e.call(WEAPON_FORM_ALLOWS_BLOCK, &args![weapon_form]).bool()
                            {
                                let process = process_of(e, this);
                                e.vcall(process.addr(), PROCESS_SLOT_614, &args![1u32]);
                            }
                            if fn_00894900(e, this) {
                                let sequence = process_sequence(e, this);
                                set_anim_action(e, this, 6, sequence);
                            }
                        }
                    }
                    6 => {
                        let mut wanted_state = 3;
                        let sequence = process_sequence(e, this);
                        let number = sequence_group_number(e, sequence);
                        let kind = group_kind(e, number);
                        let applies = match kind {
                            7 => true,
                            8 => {
                                wanted_state = 2;
                                true
                            }
                            _ => false,
                        };
                        if applies
                            && e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32()
                                == wanted_state
                        {
                            if !weapon_form.is_null()
                                && !e.call(WEAPON_FORM_ALLOWS_BLOCK, &args![weapon_form]).bool()
                            {
                                force_weapon_drawn_pairs(e, this, animation, biped, 1);
                            }
                            if fn_00894900(e, this) {
                                let sequence = process_sequence(e, this);
                                set_anim_action(e, this, 4, sequence);
                            }
                        }
                    }
                    2 => {
                        let sequence = process_sequence(e, this);
                        let number = sequence_group_number(e, sequence);
                        let mode = group_mode(e, number);
                        if mode == 2 {
                            if e.call(ANIMATION_SLOT_STATE, &args![animation, 2u32]).i32() == 1 {
                                let group = e
                                    .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 2u32])
                                    .u16();
                                let group_type_of =
                                    e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32();
                                let kind = group_kind(e, group_type_of) as i32;
                                if (5..=6).contains(&kind) {
                                    release_cast_if_casting(e, this);
                                    let sequence = process_sequence(e, this);
                                    set_anim_action(e, this, 4, sequence);
                                }
                            }
                        } else if mode == 4 {
                            queued_action_2_mode_4(e, this, animation, weapon_form);
                        }
                    }
                    3 => {
                        let sequence = process_sequence(e, this);
                        let number = sequence_group_number(e, sequence);
                        if group_mode(e, number) == 4 && !weapon_form.is_null() {
                            let sequence = process_sequence(e, this);
                            let number = sequence_group_number(e, sequence);
                            if group_kind(e, number) != 1
                                && e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32() == 2
                            {
                                let process = process_of(e, this);
                                e.vcall(process.addr(), PROCESS_SLOT_614, &args![0x2_0000u32]);
                                let sequence = process_sequence(e, this);
                                set_anim_action(e, this, 4, sequence);
                            }
                        }
                    }
                    0 | 1 => {
                        queued_action_draw_or_sheathe(
                            e,
                            this,
                            animation,
                            biped,
                            weapon_form,
                            weapon_item,
                            queued_action,
                        );
                    }
                    0xc => {
                        if sit_state == 0 {
                            let state = e.call(CONTROLLER_STATE, &args![controller]).u32();
                            if state == 1 {
                                set_anim_action(e, this, 0xffff_ffff, Ptr::NULL);
                            } else if state == 2 && !slot1_sequence.is_null() {
                                let transform = e
                                    .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![slot1_sequence])
                                    .ptr::<()>();
                                if e.call(TRANSFORM_IS_LOOPING, &args![transform]).bool() {
                                    set_anim_action(e, this, 0xffff_ffff, Ptr::NULL);
                                }
                            }
                        }
                    }
                    0xb => {
                        move_speed = e.call(ACTOR_GET_WALK_SPEED, &args![this]).f32();
                    }
                    _ => {}
                }
            }
        }
        if !switch_taken {
            let slot2 = e
                .call(ANIMATION_GET_SEQUENCE, &args![animation, 2u32])
                .u32();
            if slot2 != 0 {
                let group = e
                    .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 2u32])
                    .u16();
                if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32() == 0xaa {
                    new_action = 7;
                    let sequence = e
                        .call(ANIMATION_GET_SEQUENCE, &args![animation, 2u32])
                        .ptr::<()>();
                    set_anim_action(e, this, new_action as u32, sequence);
                }
            }
            if !weapon_form.is_null() && !fn_00897890(e, weapon_form) && queued_action == 9 {
                let sequence = process_sequence(e, this);
                let number = sequence_group_number(e, sequence);
                if e.call(ANIM_GROUP_TEST_2700, &args![number]).bool() {
                    let process = process_of(e, this);
                    let condition = e
                        .vcall(process.addr(), PROCESS_SLOT_368, &args![this])
                        .u32();
                    let chance = e.call(WEAPON_CONDITION_MULT_2, &args![condition]).f32() as f64;
                    let zero: f64 = e.global(ZERO_DOUBLE);
                    if chance > zero {
                        let chance = e.call(WEAPON_CONDITION_MULT_2, &args![condition]).f32();
                        if e.call(CHANCE_ROLL, &args![chance]).bool() {
                            let attack_group = e
                                .call(WEAPON_GET_ATTACK_ANIM, &args![weapon_form, 0u32])
                                .u32()
                                .wrapping_add(0x17);
                            let group =
                                actor_get_anim_group(e, this, attack_group, 0, 0, Ptr::NULL);
                            if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).u32()
                                == attack_group
                            {
                                new_action = 9;
                                e.call(
                                    ACTOR_PLAY_GROUP_WORKER,
                                    &args![this, attack_group, animation],
                                );
                                let sequence = e
                                    .call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
                                    .ptr::<()>();
                                set_anim_action(e, this, 9, sequence);
                                e.vcall(
                                    this.addr(),
                                    ACTOR_SLOT_4B0,
                                    &args![u32::from(group), 1u32],
                                );
                            }
                        }
                    }
                }
            }
            if new_action == -1 {
                set_anim_action(e, this, 0xffff_ffff, Ptr::NULL);
            }
        }
    }

    // Nothing was started by the queued action.
    if new_action == -1 {
        if sit_state == 0 {
            if matches!(queued_action, -1 | 2 | 4 | 0xc) {
                let state = e.call(CONTROLLER_STATE, &args![controller]).u32();
                if state == 0 {
                    if !slot1_sequence.is_null() {
                        let transform = e
                            .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![slot1_sequence])
                            .ptr::<()>();
                        let looping = e.call(TRANSFORM_IS_LOOPING, &args![transform]).bool();
                        let landing = looping || {
                            let transform = e
                                .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![slot1_sequence])
                                .ptr::<()>();
                            e.call(ANIM_GROUP_GET_NUMBER, &args![transform]).u32() == 0xe3
                        };
                        if landing {
                            queued_action = -1;
                            new_action = 0xc;
                            group_type = if flags & 1 != 0 {
                                0xf1
                            } else if flags & 2 != 0 {
                                0xf2
                            } else if flags & 4 != 0 {
                                0xf3
                            } else if flags & 8 != 0 {
                                0xf4
                            } else {
                                0xe5
                            };
                            e.call(ANIMATION_LAND_RESET, &args![animation]);
                            let landing_sound = e
                                .call(
                                    LANDING_SOUND_LOOKUP,
                                    &args![controller.byte_add(CONTROLLER_LAND_SOUND_OFFSET)],
                                )
                                .u32();
                            e.call(IMPACT_MIXER_PLAY_JUMP_LAND, &args![this, landing_sound]);
                        }
                    }
                } else if state == 1 {
                    group_type = 0xe4;
                } else if state == 2 {
                    let fall_time = e.call(CONTROLLER_FALL_TIME, &args![controller]).f32() as f64;
                    let pointer = e
                        .call(SETTING_GET_VALUE_POINTER, &args![FALL_TIME_SETTING])
                        .u32();
                    let threshold = e.mem.f32(pointer) as f64;
                    if threshold < fall_time {
                        group_type = 0xe4;
                    } else if !slot1_sequence.is_null() {
                        let transform = e
                            .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![slot1_sequence])
                            .ptr::<()>();
                        if e.call(TRANSFORM_IS_LOOPING, &args![transform]).bool() {
                            group_type = 0xe4;
                        }
                    }
                }
            }
            if is_special {
                let process = process_of(e, this);
                if !e
                    .vcall(process.addr(), PROCESS_SLOT_WEAPON_DRAWN, &args![])
                    .bool()
                    && queued_action == -1
                    && e.call(ACTOR_CAN_MOVE, &args![this]).bool()
                {
                    if !e
                        .call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                        .bool()
                    {
                        e.call(ANIMATION_SPECIAL_IDLE_FREE, &args![animation, 1u32, 1u32]);
                    }
                    group_type = 0x18;
                    new_action = 0;
                }
            }
        }
        if !is_special {
            let process = process_of(e, this);
            if e.vcall(process.addr(), PROCESS_SLOT_WEAPON_DRAWN, &args![])
                .bool()
                && queued_action == -1
                && !e.vcall(this.addr(), ACTOR_SLOT_234, &args![]).bool()
                && !e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool()
                && !e.vcall(this.addr(), ACTOR_SLOT_230, &args![]).bool()
                && !e.call(EXTRA_TYPE_IS_5, &args![this]).bool()
                && !e.call(EXTRA_OBJECTS_TEST, &args![this]).bool()
            {
                group_type = 0x19;
                new_action = 1;
            }
        }
    }

    // The second selector (from the weapon).
    let running = e.vcall(this.addr(), ACTOR_SLOT_RUNNING, &args![]).u32() != 0;
    let process = process_of(e, this);
    let weapon_selector_applies = (!running
        && (is_special || e.call(ACTOR_IN_COMBAT, &args![this]).bool()))
        || e.vcall(process.addr(), PROCESS_SLOT_WEAPON_DRAWN, &args![])
            .bool()
        || new_action == 0
        || new_action == 1;
    if weapon_selector_applies {
        if weapon_item.is_null() {
            selector_b = 1;
        } else {
            let form = entry_form(e, weapon_item);
            let weapon_type = e.call(WEAPON_TYPE, &args![form]).u32();
            selector_b = e
                .mem
                .u32(WEAPON_TYPE_ANIM_TABLE.wrapping_add(weapon_type.wrapping_mul(4)));
        }
    }
    if flags & 0x800 != 0 {
        selector_a = 2;
    } else if flags & 0x2000 != 0 {
        selector_a = 3;
    } else if flags & 0x400 != 0 {
        selector_a = 1;
    }

    if group_type == 0 {
        match sit_state {
            3 | 8 => {
                if e.call(ANIMATION_SPECIAL_IDLE_PLAYING, &args![animation])
                    .bool()
                    || e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                        .bool()
                {
                    let small = furniture.is_null() || e.mem.u8(furniture.addr() + 0xe) <= 0x14;
                    if small {
                        group_type = 1;
                    }
                }
            }
            4 | 5 | 9 | 10 => {
                let mut skip = false;
                if this == player {
                    let player_animation = e
                        .call(PLAYER_GET_ANIMATION, &args![player, 1u32])
                        .ptr::<()>();
                    let list = player_animation.byte_add(0x128);
                    if e.call(ENTRY_FIRST_EXTRA_LIST, &args![list]).u32() != 0 {
                        skip = true;
                    }
                }
                if !skip {
                    let small = furniture.is_null() || e.mem.u8(furniture.addr() + 0xe) <= 0x14;
                    if small {
                        group_type = 1;
                    }
                }
            }
            _ => {}
        }
        speed_scale = arg_2;
        if flags & 0xf != 0 {
            if flags & 0x200 != 0 {
                if flags & 1 != 0 {
                    group_type = 7;
                } else if flags & 2 != 0 {
                    group_type = 8;
                } else if flags & 4 != 0 {
                    group_type = 9;
                } else if flags & 8 != 0 {
                    group_type = 0xa;
                }
                move_speed = if e.vcall(this.addr(), ACTOR_SLOT_RUNNING, &args![]).u32() == 0 {
                    e.call(ACTOR_GET_WALK_SPEED, &args![this]).f32()
                } else {
                    e.call(ACTOR_GET_RUN_SPEED, &args![this]).f32()
                };
            } else if flags & 0xff00 != 0 {
                if flags & 1 != 0 {
                    group_type = 3;
                } else if flags & 2 != 0 {
                    group_type = 4;
                } else if flags & 4 != 0 {
                    group_type = 5;
                } else if flags & 8 != 0 {
                    group_type = 6;
                }
                move_speed = e.call(ACTOR_GET_WALK_SPEED, &args![this]).f32();
            }
        } else if flags & 0x10 != 0 {
            group_type = 0xf;
        } else if flags & 0x20 != 0 {
            group_type = 0x10;
        }
    } else if group_type == 0xe4 && flags & 0xf != 0 {
        if flags & 1 != 0 {
            group_type = 0xec;
        } else if flags & 2 != 0 {
            group_type = 0xed;
        } else if flags & 4 != 0 {
            group_type = 0xee;
        } else if flags & 8 != 0 {
            group_type = 0xef;
        }
    }

    if new_action != -1 && queued_action != -1 {
        return;
    }
    let one: f64 = e.global(ONE_DOUBLE);
    if (move_speed as f64) < one
        && (3..=0x10).contains(&(group_type as i32))
        && group_type != 0xf
        && group_type != 0x10
    {
        if this == player {
            e.vcall(player.addr(), PLAYER_SLOT_358, &args![]);
        }
        group_type = 0;
    }

    let special = in_special_state(e, this);
    let mut picked = compose_group(e, animation, selector_a, selector_b, group_type, special);
    let picked_type = e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(picked)]).u32();
    if new_action != -1 && group_type != picked_type {
        if new_action == 1 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
            e.vcall(
                acquire.addr(),
                PROCESS_SLOT_SET_WEAPON_DRAWN,
                &args![this, 0u32],
            );
        }
        new_action = -1;
    }
    group_type = picked_type;

    let process = process_of(e, this);
    if queued_action != -1
        && e.vcall(process.addr(), PROCESS_SLOT_ANIM_SEQUENCE, &args![])
            .u32()
            != 0
    {
        let slot = group_mode(e, group_type);
        let slot_sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
            .ptr::<()>();
        let current = process_sequence(e, this);
        if slot_sequence == current {
            if queued_action != 0xe {
                return;
            }
            picked = e
                .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 1u32])
                .u16();
            if e.vcall(this.addr(), ACTOR_SLOT_RUNNING, &args![]).u32() == 0 {
                picked = movement_group(picked, 3);
            } else {
                picked = running_variant(e, picked);
            }
            let picked_kind = e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(picked)]).u32();
            match picked_kind {
                3..=6 | 0xb..=0xe => {
                    move_speed = e.call(ACTOR_GET_WALK_SPEED, &args![this]).f32();
                }
                7..=0xa => {
                    move_speed = e.call(ACTOR_GET_RUN_SPEED, &args![this]).f32();
                }
                0xf | 0x10 => {
                    fn_008950f0(e, animation, arg_3);
                    return;
                }
                _ => {}
            }
            let group_speed = e
                .call(
                    ANIMATION_MOVEMENT_SPEED,
                    &args![animation, u32::from(picked)],
                )
                .u16() as i16;
            let group_speed = f32::from(group_speed);
            if group_speed != 0.0 {
                speed_scale =
                    ((move_speed as f64 / group_speed as f64) * speed_scale as f64) as f32;
            }
            fn_008950f0(e, animation, speed_scale);
            return;
        }
    }

    if picked != 0xff {
        if group_type == 0xf || group_type == 0x10 {
            fn_008950f0(e, animation, arg_3);
        } else if (3..=0x10).contains(&(group_type as i32)) {
            let speed_group = if e.vcall(this.addr(), ACTOR_SLOT_RUNNING, &args![]).u32() == 0 {
                movement_group(picked, 3)
            } else {
                running_variant(e, picked)
            };
            let group_speed = e
                .call(
                    ANIMATION_MOVEMENT_SPEED,
                    &args![animation, u32::from(speed_group)],
                )
                .u16() as i16;
            let group_speed = f32::from(group_speed);
            if group_speed != 0.0 {
                speed_scale =
                    ((move_speed as f64 / group_speed as f64) * speed_scale as f64) as f32;
            }
            fn_008950f0(e, animation, speed_scale);
        } else if (0x18..=0xa8).contains(&(group_type as i32)) {
            if weapon_form.is_null() {
                e.call(ANIMATION_SET_SPEED, &args![animation, 1.0f32]);
            } else {
                let base_speed = e.call(WEAPON_FLOAT, &args![weapon_form]).f32();
                let speed_pointer = e.mem.alloc(4);
                e.mem.set_f32(speed_pointer, base_speed);
                e.call(
                    HANDLE_ENTRY_POINT,
                    &args![0x2bu32, this, weapon_form, speed_pointer],
                );
                let entry_speed = e.mem.f32(speed_pointer);
                e.mem.free(speed_pointer);
                if e.call(ANIM_GROUP_IS_ATTACK_ACTION, &args![group_type & 0xffff])
                    .bool()
                    && !e.call(WEAPON_FORM_ALLOWS_BLOCK, &args![weapon_form]).bool()
                {
                    let process = process_of(e, this);
                    let current = e
                        .vcall(process.addr(), PROCESS_SLOT_CURRENT_WEAPON, &args![])
                        .ptr::<()>();
                    let mod_active = e
                        .call(ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, &args![current, 8u32])
                        .u8();
                    let attack_speed = e
                        .call(
                            WEAPON_GET_ATTACK_SPEED,
                            &args![weapon_form, u32::from(mod_active)],
                        )
                        .f32() as f64;
                    let speed = (attack_speed * entry_speed as f64) as f32;
                    e.call(ANIMATION_SET_SPEED, &args![animation, speed]);
                } else {
                    e.call(ANIMATION_SET_SPEED, &args![animation, entry_speed]);
                }
            }
        }
    }

    play_picked_group(
        e, this, animation, picked, group_type, new_action, flags, selector_a, selector_b,
    );

    // The movement slot is blended out when it no longer matches.
    let slot1 = e
        .call(ANIMATION_GET_SEQUENCE, &args![animation, 1u32])
        .u32();
    if slot1 != 0 && group_mode(e, group_type) != 1 {
        let mut blend = true;
        if queued_action != -1 {
            let current = process_sequence(e, this);
            let sequence = e
                .call(ANIMATION_GET_SEQUENCE, &args![animation, 1u32])
                .ptr::<()>();
            if current == sequence {
                blend = false;
            }
        }
        let blend_sequence = e
            .call(ANIMATION_CURRENT_BLEND_SEQUENCE, &args![animation])
            .ptr::<()>();
        let sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, 1u32])
            .ptr::<()>();
        if blend_sequence == sequence {
            blend = false;
        }
        let sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, 1u32])
            .ptr::<()>();
        if e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![sequence])
            .u32()
            != 1
        {
            blend = false;
        }
        if blend {
            e.call(ANIMATION_BLEND_OUT, &args![animation, 1u32, 0u32]);
            if this == player {
                let player_animation = e
                    .call(PLAYER_GET_ANIMATION, &args![player, 1u32])
                    .ptr::<()>();
                e.call(ANIMATION_BLEND_OUT, &args![player_animation, 1u32, 0u32]);
            }
        }
    }

    // The weapon's position is saved while an attack animation plays.
    if e.call(ANIMATION_GET_SEQUENCE, &args![animation, 4u32])
        .u32()
        != 0
    {
        let attack_group = e
            .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
            .u16();
        if e.call(ANIM_GROUP_IS_ATTACK_ACTION, &args![u32::from(attack_group)])
            .bool()
        {
            let mut position = [0u32; 3];
            e.with_stack(12, |e, out| {
                let result = e
                    .call(ACTOR_GET_WEAPON_POSITION, &args![this, out, 0u32])
                    .u32();
                for (i, word) in position.iter_mut().enumerate() {
                    *word = e.mem.u32(result + 4 * i as u32);
                }
            });
            let process = process_of(e, this);
            e.vcall(
                process.addr(),
                PROCESS_SLOT_SAVE_WEAPON_LAST_POSITION,
                &args![position[0], position[1], position[2]],
            );
        }
    }
}

/// `MagicCaster::ReleaseCast` (this = the caster at `actor + 0x88`, flag).
const MAGIC_CASTER_RELEASE_CAST: u32 = 0x0081_5870;
/// Animation call (`00494300`) is [`ANIMATION_MOVEMENT_SPEED`]; the weapon
/// type table is [`WEAPON_TYPE_ANIM_TABLE`].
/// Weapon test (`004c0c30`).
const WEAPON_TEST_4C0C30: u32 = 0x004c_0c30;
/// `ClearGunWobble` argument table is the same as [`WEAPON_TYPE_ANIM_TABLE`].
/// Lookup of the sound the landing plays (`0059bb30` on the controller's
/// object at `+0x410`).
const LANDING_SOUND_LOOKUP: u32 = 0x0059_bb30;

/// Releases the spell the actor is casting (`MagicCaster::ReleaseCast` on the
/// caster at `+0x88`) when its virtual slot `0x34` says it is casting.
fn release_cast_if_casting(e: &mut Engine, this: Ptr<Actor>) {
    let caster = this.byte_add(0x88);
    if e.vcall(caster.addr(), 0x34, &args![]).u32() != 0 {
        e.call(MAGIC_CASTER_RELEASE_CAST, &args![caster, 0u32]);
    }
}

/// Sets the animation action and, for the arm of the `switch` in
/// `Actor::PickAnimations` that handles queued action 2 with slot mode 4
/// (the attack slot): depending on the kind of the group playing in slot 4
/// it releases a cast and queues a post-attack state.
fn queued_action_2_mode_4(e: &mut Engine, this: Ptr<Actor>, animation: Ptr, weapon_form: Ptr) {
    let player = player(e);
    let sequence = process_sequence(e, this);
    let number = sequence_group_number(e, sequence);
    if group_kind(e, number) == 1 {
        return;
    }
    if e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32() != 1 {
        return;
    }
    let slot_group = e
        .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 4u32])
        .u16();
    let group_type = e
        .call(ANIM_GROUP_GET_TYPE, &args![u32::from(slot_group)])
        .u32();
    match group_kind(e, group_type) {
        9 => {
            let process = process_of(e, this);
            let delay = e
                .vcall(process.addr(), PROCESS_SLOT_AUTOMATIC_SHOT_DELAY, &args![])
                .f32() as f64;
            if delay == 0.0 {
                let sequence = process_sequence(e, this);
                set_anim_action(e, this, 4, sequence);
            }
        }
        6 => {
            if this == player || e.call(ACTOR_IN_COMBAT, &args![this]).bool() {
                e.call(ACTOR_FN_00899200, &args![this, 1u32, 1u32]);
            } else {
                let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
                let package = e
                    .vcall(acquire.addr(), PROCESS_SLOT_27C, &args![])
                    .ptr::<()>();
                let mut threw_special = false;
                if !package.is_null()
                    && (e.call(PACKAGE_TYPE, &args![package]).i32() == 8
                        || e.call(PACKAGE_TYPE, &args![package]).i32() == 0x10)
                {
                    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
                    if !e.vcall(acquire.addr(), PROCESS_SLOT_1B0, &args![]).bool() {
                        e.call(ACTOR_FN_00899200, &args![this, 0u32, 1u32]);
                        threw_special = true;
                    }
                }
                if !threw_special {
                    e.call(ACTOR_FN_00899200, &args![this, 1u32, 0u32]);
                }
            }
            if fn_00894900(e, this) {
                let sequence = process_sequence(e, this);
                set_anim_action(e, this, 4, sequence);
            }
        }
        5 => {
            if e.vcall(this.addr() + 0x88, 0x34, &args![]).u32() != 0 {
                release_cast_if_casting(e, this);
            } else if weapon_form.is_null() || !e.call(WEAPON_BYTE_FLAG, &args![weapon_form]).bool()
            {
                if !weapon_form.is_null() && e.call(WEAPON_TEST_4C0C30, &args![weapon_form]).bool()
                {
                    let process = process_of(e, this);
                    e.vcall(process.addr(), PROCESS_SLOT_614, &args![1u32]);
                } else if this == player || e.call(ACTOR_IN_COMBAT, &args![this]).bool() {
                    e.call(ACTOR_FN_00899200, &args![this, 0u32, 1u32]);
                } else {
                    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).ptr::<()>();
                    let package = e
                        .vcall(acquire.addr(), PROCESS_SLOT_27C, &args![])
                        .ptr::<()>();
                    let special_package = !package.is_null()
                        && (e.call(PACKAGE_TYPE, &args![package]).i32() == 8
                            || e.call(PACKAGE_TYPE, &args![package]).i32() == 0x10);
                    let flag = u32::from(special_package);
                    e.call(ACTOR_FN_00899200, &args![this, 0u32, flag]);
                }
            }
            if fn_00894900(e, this) {
                if !weapon_form.is_null()
                    && !e.call(WEAPON_BYTE_FLAG, &args![weapon_form]).bool()
                    && !e.call(WEAPON_FORM_ALLOWS_BLOCK, &args![weapon_form]).bool()
                {
                    let sequence = process_sequence(e, this);
                    set_anim_action(e, this, 3, sequence);
                } else {
                    let sequence = process_sequence(e, this);
                    set_anim_action(e, this, 4, sequence);
                }
            }
        }
        _ => {}
    }
}

/// `Actor::fn_00899200` (a later function of this part): called with the
/// actor and two flags.
const ACTOR_FN_00899200: u32 = 0x0089_9200;

/// The arm of the `switch` in `Actor::PickAnimations` for queued actions 0
/// (sheathe) and 1 (draw): if the weapon is not already in the wanted state
/// and the attack slot is busy, the process is told (`SetWeaponDrawn`), the
/// animation pairs are forced to match (clearing the gun wobble first), iron
/// sights are reset, the player's weapon shader values are set, and the
/// enchantment effect and sound of the weapon are refreshed.
fn queued_action_draw_or_sheathe(
    e: &mut Engine,
    this: Ptr<Actor>,
    animation: Ptr,
    biped: u32,
    weapon_form: Ptr,
    weapon_item: Ptr,
    queued_action: i32,
) {
    let player = player(e);
    let process = process_of(e, this);
    let drawn = e
        .vcall(process.addr(), PROCESS_SLOT_WEAPON_DRAWN, &args![])
        .u8();
    let want_drawn = u32::from(queued_action == 0);
    if u32::from(drawn) == want_drawn {
        return;
    }
    if e.call(ANIMATION_SLOT_STATE, &args![animation, 4u32]).i32() < 1 {
        return;
    }
    let mut current_animation = animation;
    let mut current_biped = biped;
    let mut count = if this == player { 2 } else { 1 };
    let process = process_of(e, this);
    e.vcall(
        process.addr(),
        PROCESS_SLOT_SET_WEAPON_DRAWN,
        &args![this, want_drawn],
    );
    while count != 0 {
        if this == player && count == 1 {
            current_animation = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).ptr();
            current_biped = e.call(PLAYER_GET_BIPED, &args![player, 1u32]).u32();
        }
        let process = process_of(e, this);
        if !e
            .vcall(process.addr(), PROCESS_SLOT_WEAPON_DRAWN, &args![])
            .bool()
            && !weapon_form.is_null()
            && !current_animation.is_null()
            && e.call(ANIMATION_GUN_WOBBLE_SOURCE, &args![current_animation])
                .u32()
                != 0
        {
            let source = e
                .call(ANIMATION_GUN_WOBBLE_SOURCE, &args![current_animation])
                .u32();
            let weapon_type = e.call(WEAPON_TYPE, &args![weapon_form]).u32();
            let table_value = e
                .mem
                .u32(WEAPON_TYPE_ANIM_TABLE.wrapping_add(weapon_type.wrapping_mul(4)));
            e.call(CLEAR_GUN_WOBBLE, &args![table_value, source]);
        }
        let process = process_of(e, this);
        e.vcall(
            process.addr(),
            PROCESS_SLOT_FORCE_WEAPON_DRAWN,
            &args![want_drawn, current_biped, current_animation, this],
        );
        count -= 1;
    }
    e.call(ACTOR_SET_IRON_SIGHTS, &args![this, 0u32, 0u32, 0u32]);

    if this == player {
        let holder = e.mem.u32(this.addr() + 0xac);
        if queued_action == 0 {
            let mut first_value = 0u32;
            let mut second_value = 0u32;
            let process = process_of(e, this);
            let bone = e
                .vcall(process.addr(), PROCESS_SLOT_WEAPON_BONE, &args![biped])
                .u32();
            let node = e.call(BONE_NODE_LOOKUP, &args![bone, 0u32]).ptr::<()>();
            if !node.is_null() {
                let name = e.call(EXTRA_DATA_NAME, &args![]).u32();
                let extra = e
                    .call(NI_OBJECT_NET_GET_EXTRA_DATA, &args![node, name])
                    .ptr::<()>();
                if !extra.is_null() && fn_008978d0(e, extra) {
                    let first_key = fn_008978b0(e);
                    first_value = e.vcall(node.addr(), 0x9c, &args![first_key]).u32();
                    let second_key = fn_008978c0(e);
                    second_value = e.vcall(node.addr(), 0x9c, &args![second_key]).u32();
                }
            }
            let holder = e.mem.u32(this.addr() + 0xac);
            e.call(SET_SHADER_VALUE, &args![holder, 0u32, first_value]);
            let holder = e.mem.u32(this.addr() + 0xac);
            e.call(SET_SHADER_VALUE, &args![holder, 1u32, second_value]);
        } else {
            e.call(SET_SHADER_VALUE, &args![holder, 0u32, 0u32]);
            let holder = e.mem.u32(this.addr() + 0xac);
            e.call(SET_SHADER_VALUE, &args![holder, 1u32, 0u32]);
            let holder = e.mem.u32(this.addr() + 0xac);
            fn_008978f0(e, Ptr::new(holder), 1, 1);
            let holder = e.mem.u32(this.addr() + 0xac);
            fn_008978f0(e, Ptr::new(holder), 1, 1);
        }
    }

    let process = process_of(e, this);
    e.vcall(
        process.addr(),
        PROCESS_SLOT_SHADER_EFFECTS,
        &args![1u32, 0u32, 0u32],
    );
    let process = process_of(e, this);
    if e.vcall(process.addr(), PROCESS_SLOT_WEAPON_DRAWN, &args![])
        .bool()
    {
        let process = process_of(e, this);
        if e.vcall(
            process.addr(),
            PROCESS_SLOT_WEAPON_ENCHANTMENT_VISUALS,
            &args![],
        )
        .u32()
            != 0
        {
            let visuals = e
                .vcall(
                    process.addr(),
                    PROCESS_SLOT_WEAPON_ENCHANTMENT_VISUALS,
                    &args![],
                )
                .u32();
            let effect = e
                .call(
                    FIND_AND_CLEANUP_WEAPON_SHADER_HIT_EFFECT,
                    &args![PROCESS_LISTS, this, visuals],
                )
                .ptr::<()>();
            if !effect.is_null() {
                e.call(RESET_ALPHA_TIMER, &args![effect]);
            }
            if !weapon_form.is_null() {
                let enchanting = e.call(GET_FORM_ENCHANTING, &args![weapon_form]).u32();
                let mut enchantment = if enchanting != 0 {
                    enchanting + 0x18
                } else {
                    0
                };
                if enchantment == 0 {
                    let poison = e.call(ITEM_CHANGE_GET_POISON, &args![weapon_item]).u32();
                    enchantment = if poison != 0 { poison + 0x30 } else { 0 };
                }
                if enchantment != 0 {
                    let handle = e.mem.alloc(12);
                    e.call(
                        PLAY_SOUND_BY_EDITOR_NAME,
                        &args![
                            this,
                            handle,
                            ENCHANTED_EQUIP_SOUND_NAME,
                            0u32,
                            0x4000_0102u32,
                            1u32
                        ],
                    );
                    e.call(SOUND_HANDLE_DESTROY, &args![handle]);
                    e.mem.free(handle);
                }
            }
        }
    }
}

/// The last part of `Actor::PickAnimations`: plays the chosen group `picked`
/// in the slot the group table names unless that very group already plays
/// there (a finished sequence). For the movement slot it first plays the
/// group for the actor's current movement direction when that changed; after
/// the play it sets the action, and replays the landing / movement variants
/// for groups `0xe3` and `0xf1..=0xf4`.
#[allow(clippy::too_many_arguments)]
fn play_picked_group(
    e: &mut Engine,
    this: Ptr<Actor>,
    animation: Ptr,
    picked: u16,
    group_type: u32,
    new_action: i32,
    flags: u16,
    selector_a: u32,
    selector_b: u32,
) {
    let slot = group_mode(e, group_type);
    let current = e
        .call(ANIMATION_GROUP_OF_SLOT, &args![animation, slot])
        .u16();
    if u32::from(current) == u32::from(picked)
        && e.call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
            .u32()
            != 0
    {
        let sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
            .ptr::<()>();
        if e.call(SEQUENCE_GET_GENERIC_LOCATION, &args![sequence])
            .u32()
            != 0
        {
            return;
        }
    }
    if !e
        .call(ANIMATION_GROUP_LOADED, &args![animation, u32::from(picked)])
        .bool()
    {
        return;
    }
    let mut picked = picked;
    if group_mode(e, group_type) == 1
        && e.call(ANIMATION_GET_SEQUENCE, &args![animation, 1u32])
            .u32()
            != 0
    {
        let slot_one_group = e
            .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 1u32])
            .u16();
        let movement = e
            .call(ANIM_GROUP_GET_MOVE_OV2, &args![u32::from(slot_one_group)])
            .u32();
        if selector_a != movement {
            let special = in_special_state(e, this);
            let other = compose_group(e, animation, selector_a, selector_b, 0, special);
            let slot_zero_group = e
                .call(ANIMATION_GROUP_OF_SLOT, &args![animation, 0u32])
                .u16();
            if other != slot_zero_group {
                e.call(
                    ANIMATION_PLAY_GROUP,
                    &args![
                        animation,
                        u32::from(other),
                        1u32,
                        0xffff_ffffu32,
                        0xffff_ffffu32
                    ],
                );
                e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![u32::from(other), 1u32]);
            }
        }
    }
    e.call(
        ANIMATION_PLAY_GROUP,
        &args![
            animation,
            u32::from(picked),
            1u32,
            0xffff_ffffu32,
            0xffff_ffffu32
        ],
    );
    if new_action != -1
        && !e
            .call(ANIM_GROUP_TEST_4CC0, &args![u32::from(picked)])
            .bool()
    {
        let slot = group_mode(e, group_type);
        let sequence = e
            .call(ANIMATION_GET_SEQUENCE, &args![animation, slot])
            .ptr::<()>();
        set_anim_action(e, this, new_action as u32, sequence);
    }
    e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![u32::from(picked), 1u32]);

    if group_type == 0xe3 {
        let mut variant = 0xe4;
        if flags & 0xf != 0 {
            if flags & 1 != 0 {
                variant = 0xec;
            } else if flags & 2 != 0 {
                variant = 0xed;
            } else if flags & 4 != 0 {
                variant = 0xee;
            } else if flags & 8 != 0 {
                variant = 0xef;
            }
        }
        let special = in_special_state(e, this);
        picked = compose_group(e, animation, selector_a, selector_b, variant, special);
        e.call(
            ANIMATION_PLAY_GROUP,
            &args![
                animation,
                u32::from(picked),
                0u32,
                0xffff_ffffu32,
                0xffff_ffffu32
            ],
        );
        e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![u32::from(picked), 0u32]);
    } else if (0xf1..=0xf4).contains(&(group_type as i32)) && flags & 0xf != 0 {
        let mut variant = 0xffu32;
        if flags & 0x200 != 0 {
            if flags & 1 != 0 {
                variant = 7;
            } else if flags & 2 != 0 {
                variant = 8;
            } else if flags & 4 != 0 {
                variant = 9;
            } else if flags & 8 != 0 {
                variant = 0xa;
            }
        } else if flags & 1 != 0 {
            variant = 3;
        } else if flags & 2 != 0 {
            variant = 4;
        } else if flags & 4 != 0 {
            variant = 5;
        } else if flags & 8 != 0 {
            variant = 6;
        }
        if variant != 0xff {
            let special = in_special_state(e, this);
            picked = compose_group(e, animation, selector_a, selector_b, variant, special);
            e.call(
                ANIMATION_PLAY_GROUP,
                &args![
                    animation,
                    u32::from(picked),
                    0u32,
                    0xffff_ffffu32,
                    0xffff_ffffu32
                ],
            );
            e.vcall(this.addr(), ACTOR_SLOT_4B0, &args![u32::from(picked), 0u32]);
        }
    }
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00891d70, actor_get_gold_amount(Ptr<Actor>) -> u32),
        entry!(
            0x00891db0,
            actor_has_objects(Ptr<Actor>, u32, u32, u32, u32, u32) -> bool
        ),
        entry!(0x00891e00, fn_00891e00(Ptr<Actor>, Ptr, u32, u8)),
        entry!(
            0x00892460,
            actor_remove_picked_up_object_from_world(Ptr, u8)
        ),
        entry!(0x008924e0, actor_pay_gold_to_actor(Ptr<Actor>, u32, u32)),
        entry!(0x00892520, fn_00892520(Ptr<Actor>, Ptr, Ptr, u32)),
        entry!(
            0x008929a0,
            fn_008929a0(Ptr<Actor>, Ptr, Ptr, u32, u32, u32) -> Ptr
        ),
        entry!(
            0x00892e90,
            fn_00892e90(Ptr<Actor>, Ptr, u32, Ptr, u32, u32) -> bool
        ),
        entry!(0x00893190, actor_should_talk_to(Ptr<Actor>, Ptr) -> bool),
        entry!(0x00893340, actor_init_package_locations(Ptr<Actor>, u8)),
        entry!(0x00893500, actor_get_fatigue(Ptr<Actor>) -> f32),
        entry!(0x00893530, actor_get_fatigue_percentage(Ptr<Actor>) -> f32),
        entry!(0x00893590, actor_get_health_percentage(Ptr<Actor>) -> f32),
        entry!(0x008935f0, actor_queue_attack(Ptr<Actor>, u32) -> bool),
        entry!(0x00893a40, actor_start_attack(Ptr<Actor>, u32) -> bool),
        entry!(0x00894900, fn_00894900(Ptr<Actor>) -> bool),
        entry!(0x00894940, fn_00894940(Ptr<Actor>)),
        entry!(0x00894cc0, actor_set_block(Ptr<Actor>, u8) -> bool),
        entry!(0x00894d60, actor_get_blocked(Ptr<Actor>) -> bool),
        entry!(0x00894d90, fn_00894d90(Ptr<Actor>, u32)),
        entry!(0x00894e90, fn_00894e90(Ptr<Actor>)),
        entry!(0x00894f90, fn_00894f90(Ptr<Actor>, u8) -> u32),
        entry!(0x008950f0, fn_008950f0(Ptr, f32)),
        entry!(0x00895110, actor_pick_animations(Ptr<Actor>, f32, f32)),
        entry!(0x00897890, fn_00897890(Ptr) -> bool),
        entry!(0x008978b0, fn_008978b0() -> u32),
        entry!(0x008978c0, fn_008978c0() -> u32),
        entry!(0x008978d0, fn_008978d0(Ptr) -> bool),
        entry!(0x008978f0, fn_008978f0(Ptr, u32, u8)),
        entry!(
            0x00897910,
            actor_get_anim_group(Ptr<Actor>, u32, u32, u8, Ptr) -> u16
        ),
        entry!(0x00897b30, fn_00897b30(Ptr) -> u32),
        entry!(
            0x00897b50,
            actor_get_anim_group_duration(Ptr<Actor>, u32) -> f32
        ),
        entry!(0x00897bd0, fn_00897bd0(Ptr<Actor>, u32)),
        entry!(
            0x00897de0,
            fn_00897de0(Ptr<Actor>, Ptr, u8, u8, u8, Ptr, Ptr, f32, f32)
        ),
        entry!(0x00898200, fn_00898200(Ptr, u8)),
        entry!(0x00898250, fn_00898250(Ptr, f32)),
        entry!(0x00898280, fn_00898280(Ptr, u8)),
        entry!(0x008982a0, fn_008982a0(Ptr, u8)),
        entry!(
            0x008982c0,
            actor_initiate_avoid_package(Ptr<Actor>, Ptr, f32)
        ),
        entry!(0x008984c0, actor_initiate_surface_package(Ptr<Actor>)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every function outside this file that the translations call. The test
    /// engine registers a double returning 0 at each, so that a test names
    /// only the ones whose results matter (and the real translations of other
    /// units never run).
    const EXTERNAL_CALLEES: &[u32] = &[
        0x00401170, 0x00403550, 0x00403e20, 0x00408840, 0x00408d60, 0x00408da0, 0x004181e0,
        0x00418520, 0x00418660, 0x00418770, 0x00418890, 0x00418900, 0x00418ab0, 0x00419700,
        0x0041a250, 0x0041a800, 0x0041aeb0, 0x0041aed0, 0x0041c7f0, 0x0041c8d0, 0x0041ca90,
        0x004216f0, 0x00425fd0, 0x0042cde0, 0x0042eb60, 0x004301b0, 0x00437bd0, 0x00437bf0,
        0x00440da0, 0x004459e0, 0x00446390, 0x00448a60, 0x00448a80, 0x0044ddc0, 0x00453700,
        0x004537b0, 0x00453a70, 0x0045bc00, 0x0045cd60, 0x00460250, 0x004702f0, 0x00470470,
        0x00474a40, 0x00474a80, 0x00474cb0, 0x00476b70, 0x0047d3b0, 0x00483710, 0x00483a00,
        0x00484e60, 0x0048e8a0, 0x0048f7f0, 0x00491040, 0x004937c0, 0x004937e0, 0x00493800,
        0x00493bb0, 0x00494300, 0x00494710, 0x00494740, 0x004954e0, 0x00495740, 0x00496080,
        0x00496500, 0x004974a0, 0x004981f0, 0x00498290, 0x004985b0, 0x004985f0, 0x00498910,
        0x00498cf0, 0x004994f0, 0x004997b0, 0x004bda70, 0x004bdcc0, 0x004bddd0, 0x004be330,
        0x004c0c30, 0x004c0c90, 0x004c69f0, 0x004cb320, 0x004cb4b0, 0x004cffe0, 0x004d0650,
        0x004dff00, 0x004e4620, 0x004eaf60, 0x004eec60, 0x004f1540, 0x004fe160, 0x00500940,
        0x00508100, 0x00511840, 0x0051e2a0, 0x0051f5f0, 0x005224c0, 0x00524b40, 0x00524b60,
        0x00524d10, 0x00525980, 0x00546a40, 0x0054ca90, 0x00551110, 0x00551180, 0x005585e0,
        0x00559450, 0x0055b980, 0x0055d520, 0x00564db0, 0x005653d0, 0x00566950, 0x00567790,
        0x00572230, 0x00572d30, 0x00573800, 0x00574b30, 0x00574fa0, 0x00575770, 0x00576260,
        0x005785e0, 0x00579670, 0x00598040, 0x0059bb30, 0x005b5e40, 0x005c0880, 0x005d0b80,
        0x005d43c0, 0x005e3fc0, 0x005e58f0, 0x005f2370, 0x005f23a0, 0x005f23c0, 0x005f2420,
        0x005f2440, 0x005f2540, 0x005f2670, 0x005f2700, 0x005f2b60, 0x005f3780, 0x005f4cc0,
        0x005f4d60, 0x00600900, 0x00600920, 0x00600940, 0x00639aa0, 0x006450c0, 0x00646020,
        0x006477b0, 0x006477e0, 0x00670b30, 0x00670b90, 0x00670fc0, 0x00671d10, 0x00671d30,
        0x00672fc0, 0x00675a50, 0x00675c20, 0x00675de0, 0x006777b0, 0x00678610, 0x00678ca0,
        0x0067a1b0, 0x0067a720, 0x0067f030, 0x0067f1f0, 0x0067f3c0, 0x0067f410, 0x0067ff70,
        0x006800b0, 0x00680110, 0x006815c0, 0x0068a7d0, 0x006ca4e0, 0x00702360, 0x007043c0,
        0x00704af0, 0x00705020, 0x00705040, 0x007052f0, 0x00709430, 0x00709470, 0x0070f490,
        0x00726070, 0x007af430, 0x007b3fa0, 0x008041a0, 0x00810530, 0x00815870, 0x008216c0,
        0x00825c00, 0x00826b40, 0x00826b90, 0x008391b0, 0x0084e3a0, 0x00867f90, 0x00877720,
        0x0087afa0, 0x0087faa0, 0x00881620, 0x00881650, 0x008843a0, 0x008846e0, 0x00884dc0,
        0x00884eb0, 0x0088b0f0, 0x0088c650, 0x0088c830, 0x00899200, 0x008a0330, 0x008a16d0,
        0x008a5340, 0x008a6840, 0x008a6970, 0x008a6a40, 0x008a73e0, 0x008a7570, 0x008a78f0,
        0x008aded0, 0x008b28c0, 0x008b3ad0, 0x008b70d0, 0x008ba3e0, 0x008ba410, 0x008bb650,
        0x008bc9d0, 0x008be940, 0x008bfa40, 0x008c7aa0, 0x008d6a80, 0x008d6f30, 0x008d8520,
        0x009306d0, 0x00933270, 0x00933840, 0x009344a0, 0x00944460, 0x00950a60, 0x00950b00,
        0x00966a70, 0x00969040, 0x0096f450, 0x00974db0, 0x009818e0, 0x009819d0, 0x00984f60,
        0x00994ef0, 0x009f0e00, 0x009f1140, 0x009f1310, 0x00a24660, 0x00a5bdd0, 0x00ad7480,
        0x00ad8830, 0x00ad8ce0, 0x00c6a270, 0x00c6bd00, 0x00c74890, 0x00ec43fb, 0x00ec62c0,
    ];

    fn ret(eax: u32) -> Ret {
        Ret {
            eax,
            ..Ret::default()
        }
    }

    fn float_ret(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    /// The test engine: doubles returning 0 at every external callee, and the
    /// page holding `thePlayer`.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for address in EXTERNAL_CALLEES {
            e.register_double(*address, |_, _| Ret::default());
        }
        e.map(PLAYER_CHARACTER, 4);
        e.call_log = Some(vec![]);
        e
    }

    /// Makes `address` return `value` in EAX.
    fn stub(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| ret(value));
    }

    /// Makes `address` return a `float` in ST0.
    fn stub_float(e: &mut Engine, address: u32, value: f32) {
        e.register_double(address, move |_, _| float_ret(value));
    }

    /// Makes `address` run `body` (a double that sees its arguments).
    fn stub_with(
        e: &mut Engine,
        address: u32,
        body: impl FnMut(&mut Engine, &[u32]) -> Ret + 'static,
    ) {
        e.register_double(address, body);
    }

    /// Allocates an object of `size` bytes whose virtual table has one double
    /// per `(slot offset, result)` pair; returns the object.
    fn object(e: &mut Engine, size: u32, slots: &[(u32, Ret)]) -> Ptr {
        let table = e.mem.alloc(0x800);
        for (offset, result) in slots {
            let target = e.mem.alloc(8);
            let result = *result;
            e.register_double(target, move |_, _| result);
            e.mem.set_u32(table + offset, target);
        }
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, table);
        Ptr::new(object)
    }

    /// The function a virtual slot of `object` points at.
    fn slot_target(e: &Engine, object: Ptr, offset: u32) -> u32 {
        let table = e.mem.u32(object.addr());
        e.mem.u32(table + offset)
    }

    /// An actor with the given virtual slots, and (if any) a process object
    /// with its own slots.
    fn actor_with(
        e: &mut Engine,
        slots: &[(u32, Ret)],
        process_slots: Option<&[(u32, Ret)]>,
    ) -> Ptr<Actor> {
        let actor = object(e, 0x200, slots).cast::<Actor>();
        if let Some(process_slots) = process_slots {
            let process = object(e, 0x800, process_slots);
            e.set(actor, Actor::pCurrentProcess, process);
        }
        actor
    }

    /// Sets `thePlayer`.
    fn set_player(e: &mut Engine, player: Ptr<Actor>) {
        e.set_global(PLAYER_CHARACTER, player.addr());
    }

    /// The argument lists of every logged call to `address`.
    fn calls_to(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn call_count(e: &Engine, address: u32) -> usize {
        calls_to(e, address).len()
    }

    /// Writes the animation group table entry of group `number`: the slot
    /// mode word (`+8`) and the kind word (`+0xc`).
    fn set_group(e: &mut Engine, number: u32, mode: u32, kind: u32) {
        let base = ANIM_GROUP_TABLE + number * 0x24;
        e.map(base, 0x24);
        e.mem.set_u32(base + 8, mode);
        e.mem.set_u32(base + 0xc, kind);
    }

    fn clear_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// Installs a virtual table with the given slots at `address` (the
    /// address of a sub-object's vtable pointer).
    fn vtable_at(e: &mut Engine, address: u32, slots: &[(u32, Ret)]) {
        let table = e.mem.alloc(0x100);
        for (offset, result) in slots {
            let target = e.mem.alloc(8);
            let result = *result;
            e.register_double(target, move |_, _| result);
            e.mem.set_u32(table + offset, target);
        }
        e.mem.set_u32(address, table);
    }

    #[test]
    fn gold_amount_comes_from_the_inventory_changes() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        stub(&mut e, REFR_EXTRA_DATA_LIST, 0x1000);
        assert_eq!(e.call(0x0089_1d70, &args![actor]).u32(), 0);
        stub(&mut e, GET_CONTAINER_CHANGES, 0x2000);
        stub(&mut e, INVENTORY_CHANGES_GET_GOLD_AMOUNT, 150);
        assert_eq!(actor_get_gold_amount(&mut e, actor), 150);
        assert_eq!(calls_to(&e, GET_CONTAINER_CHANGES), vec![vec![0x1000]; 2]);
        assert_eq!(
            calls_to(&e, INVENTORY_CHANGES_GET_GOLD_AMOUNT),
            vec![vec![0x2000]]
        );
    }

    #[test]
    fn has_objects_passes_its_words_on_with_two_swapped() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        stub(&mut e, GET_CONTAINER_CHANGES, 0);
        assert!(!e
            .call(0x0089_1db0, &args![actor, 1u32, 2u32, 3u32, 4u32, 5u32])
            .bool());
        assert_eq!(call_count(&e, CONTAINER_HAS_OBJECTS), 0);

        stub(&mut e, GET_CONTAINER_CHANGES, 0x2000);
        stub(&mut e, CONTAINER_HAS_OBJECTS, 1);
        assert!(actor_has_objects(&mut e, actor, 1, 2, 3, 4, 5));
        assert_eq!(
            calls_to(&e, CONTAINER_HAS_OBJECTS),
            vec![vec![0x2000, 1, 2, 3, 5, 4, actor.addr()]]
        );
    }

    #[test]
    fn picked_up_objects_are_marked_or_removed() {
        let mut e = engine();
        let refr = object(&mut e, 0x100, &[(0x1d0, ret(0x77)), (0x1c0, ret(0))]);
        stub(&mut e, REFR_PARENT_CELL, 0x9000);
        // Not deleted: only marked as picked up.
        actor_remove_picked_up_object_from_world(&mut e, refr, 0);
        assert_eq!(calls_to(&e, BHK_WORLD_ACTIVATE), vec![vec![0x77, 1, 1, 0]]);
        assert_eq!(calls_to(&e, MARK_AS_PICKED_UP), vec![vec![refr.addr()]]);
        assert_eq!(call_count(&e, GARBAGE_COLLECTOR_ADD), 0);
        // Deleted: detached, garbage collected and removed from its cell.
        assert_eq!(e.call(0x0089_2460, &args![refr, 1u32]).u32(), 0);
        assert_eq!(call_count(&e, MARK_AS_PICKED_UP), 1);
        assert_eq!(calls_to(&e, GARBAGE_COLLECTOR_ADD), vec![vec![refr.addr()]]);
        assert_eq!(
            calls_to(&e, CELL_REMOVE_REFERENCE),
            vec![vec![0x9000, refr.addr()]]
        );
        assert_eq!(call_count(&e, slot_target(&e, refr, 0x1c0)), 1);
    }

    #[test]
    fn paying_gold_swaps_the_two_words() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        stub(&mut e, GET_CONTAINER_CHANGES, 0x2000);
        e.call(0x0089_24e0, &args![actor, 7u32, 8u32]);
        assert_eq!(
            calls_to(&e, INVENTORY_CHANGES_REMOVE_GOLD),
            vec![vec![0x2000, actor.addr(), 8, 7]]
        );
    }

    #[test]
    fn fatigue_and_health_read_the_actor_value_owner() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        let owner = actor.addr() + 0xa4;
        vtable_at(&mut e, owner, &[(0x00, ret(100)), (0x0c, float_ret(25.0))]);
        assert_eq!(e.call(0x0089_3500, &args![actor]).f32(), 25.0);
        assert_eq!(e.call(0x0089_3530, &args![actor]).f32(), 0.25);
        assert_eq!(actor_get_health_percentage(&mut e, actor), 0.25);
        let first = calls_to(&e, slot_target(&e, Ptr::new(owner - 0xa4 + 0xa4), 0));
        let _ = first;
        // The actor value numbers are passed in.
        let table = e.mem.u32(owner);
        let base_slot = e.mem.u32(table);
        let current_slot = e.mem.u32(table + 0x0c);
        assert_eq!(calls_to(&e, current_slot)[0], vec![owner, 0x16]);
        assert_eq!(calls_to(&e, base_slot).last().unwrap(), &vec![owner, 0x10]);
        // A base of 0 gives 1.0.
        vtable_at(&mut e, owner, &[(0x00, ret(0)), (0x0c, float_ret(25.0))]);
        assert_eq!(actor_get_fatigue_percentage(&mut e, actor), 1.0);
    }

    const BASE_FORM: u32 = 0x0000_b000;

    /// An actor with a process (no weapon, no ammunition) and a reference of
    /// a plain form type 5, as `fn_00891e00` and `fn_00892520` expect.
    fn pickup_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr) {
        let actor = actor_with(
            e,
            &[
                (0x218, ret(0)),
                (0x22c, ret(0)),
                (0x428, ret(0)),
                (0x1e4, ret(0)),
                (0x3bc, ret(0)),
                (0x17c, ret(0)),
                (0x3c4, ret(0)),
            ],
            Some(&[(0x14c, ret(0)), (0x148, ret(0))]),
        );
        let refr = object(
            e,
            0x100,
            &[
                (0x4c, ret(0)),
                (0x224, ret(0)),
                (0xc4, ret(0)),
                (0x48, ret(0)),
                (0x1d0, ret(0x77)),
                (0x1c0, ret(0)),
            ],
        );
        stub(e, REFR_BASE_FORM, BASE_FORM);
        stub(e, FORM_TYPE, 5);
        stub(e, REFR_PARENT_CELL, 0x9000);
        stub(e, REFR_EXTRA_DATA_LIST, 0x5000);
        (actor, refr)
    }

    #[test]
    fn picking_up_a_plain_object_deletes_it_from_the_world() {
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        e.call(0x0089_1e00, &args![actor, refr, 3u32, 0u32]);
        assert_eq!(
            calls_to(&e, ACTOR_PICK_UP_WORKER),
            vec![vec![actor.addr(), refr.addr(), 3, 0, 0]]
        );
        // The reference is released (virtual slot 0x4c with 0) and removed.
        assert_eq!(
            calls_to(&e, slot_target(&e, refr, 0x4c)),
            vec![vec![refr.addr(), 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, refr, 0xc4)),
            vec![vec![refr.addr(), 1]]
        );
        assert_eq!(calls_to(&e, GARBAGE_COLLECTOR_ADD), vec![vec![refr.addr()]]);
        assert_eq!(call_count(&e, MARK_AS_PICKED_UP), 0);
        // Actors that had it as a target were looked up by form id.
        assert_eq!(call_count(&e, PROCESS_LISTS_FIND_ACTORS), 1);
    }

    #[test]
    fn a_persistent_reference_is_marked_and_queued() {
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        stub(&mut e, REFR_GET_REF_PERSISTS, 1);
        stub(&mut e, PICK_UP_GOES_TO_TASK_QUEUE, 1);
        stub(&mut e, GET_TES, 0x4321);
        fn_00891e00(&mut e, actor, refr, 3, 1);
        assert_eq!(
            calls_to(&e, ACTOR_PICK_UP_WORKER),
            vec![vec![actor.addr(), refr.addr(), 3, 1, 0]]
        );
        assert_eq!(
            calls_to(&e, TES_QUEUE_PICK_UP),
            vec![vec![0x4321, refr.addr(), 0]]
        );
        assert_eq!(call_count(&e, GARBAGE_COLLECTOR_ADD), 0);
        assert_eq!(call_count(&e, MARK_AS_PICKED_UP), 0);
    }

    #[test]
    fn picking_up_stolen_goods_raises_an_alarm_and_takes_ownership() {
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        // Slot 0x218 true: the ownership checks run.
        let actor = {
            let table = e.mem.u32(actor.addr());
            let target = e.mem.alloc(8);
            e.register_double(target, |_, _| ret(1));
            e.mem.set_u32(table + 0x218, target);
            actor
        };
        stub(&mut e, REFR_GET_OWNER, 0x7770);
        stub(&mut e, REFR_IS_AN_OWNER, 0);
        stub(&mut e, ACQUIRE_OBJECT_FIELD_28, 0);
        fn_00891e00(&mut e, actor, refr, 2, 0);
        assert_eq!(
            calls_to(&e, ACTOR_STEAL_ALARM),
            vec![vec![actor.addr(), refr.addr(), BASE_FORM, 2, 0, 0x7770]]
        );
        assert_eq!(
            calls_to(&e, EXTRA_SET_OWNERSHIP),
            vec![vec![0x5000, BASE_FORM]]
        );
        assert_eq!(call_count(&e, EXTRA_REMOVE_OWNERSHIP), 0);

        // An owner of the object just removes the ownership.
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        let table = e.mem.u32(actor.addr());
        let target = e.mem.alloc(8);
        e.register_double(target, |_, _| ret(1));
        e.mem.set_u32(table + 0x218, target);
        stub(&mut e, REFR_GET_OWNER, 0x7770);
        stub(&mut e, REFR_IS_AN_OWNER, 1);
        fn_00891e00(&mut e, actor, refr, 2, 0);
        assert_eq!(call_count(&e, ACTOR_STEAL_ALARM), 0);
        assert_eq!(calls_to(&e, EXTRA_REMOVE_OWNERSHIP), vec![vec![0x5000]]);
    }

    #[test]
    fn picking_up_ammunition_without_a_stack_uses_the_forms_count() {
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        stub_with(&mut e, FORM_TYPE, |_, a| {
            ret(if a[0] == BASE_FORM + 0x80 { 12 } else { 0x29 })
        });
        stub(&mut e, EXTRA_GET_COUNT, 0);
        fn_00891e00(&mut e, actor, refr, 3, 1);
        // The flag is cleared and the count comes from the form.
        assert_eq!(
            calls_to(&e, ACTOR_PICK_UP_WORKER),
            vec![vec![actor.addr(), refr.addr(), 12, 0, 0]]
        );
        // A stack of 5 keeps the caller's count.
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        stub(&mut e, FORM_TYPE, 0x29);
        stub(&mut e, EXTRA_GET_COUNT, 5);
        fn_00891e00(&mut e, actor, refr, 3, 1);
        assert_eq!(
            calls_to(&e, ACTOR_PICK_UP_WORKER),
            vec![vec![actor.addr(), refr.addr(), 3, 0, 0]]
        );
    }

    #[test]
    fn everyone_targeting_the_object_is_retargeted() {
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        // One list node holding another actor that targets the reference.
        let other = 0x7000_0001;
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, other);
        let node = e.mem.alloc(8);
        stub(&mut e, PROCESS_LISTS_FIND_ACTORS, node);
        stub(&mut e, LIST_ITEM_SLOT, slot);
        stub(&mut e, LIST_NEXT, 0);
        stub(&mut e, ACTOR_GET_CURRENT_PACKAGE_TARGET, refr.addr());
        fn_00891e00(&mut e, actor, refr, 1, 0);
        assert_eq!(
            calls_to(&e, ACTOR_SET_CURRENT_TARGET),
            vec![vec![other, actor.addr()]]
        );
        // The temporary list is cleared and deleted.
        assert_eq!(calls_to(&e, LIST_CLEAR), vec![vec![node]]);
        assert_eq!(calls_to(&e, LIST_DELETE), vec![vec![node, 1]]);
    }

    /// Replaces slot `offset` of `object`'s table by a double returning `value`.
    fn set_slot(e: &mut Engine, object: Ptr, offset: u32, value: Ret) {
        let table = e.mem.u32(object.addr());
        let target = e.mem.alloc(8);
        e.register_double(target, move |_, _| value);
        e.mem.set_u32(table + offset, target);
    }

    #[test]
    fn a_note_given_to_the_player_is_added_to_the_pip_boy() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        set_player(&mut e, actor);
        let extra = object(&mut e, 0x40, &[(0, ret(0))]);
        stub(&mut e, FORM_TYPE, 0x31);
        e.call(0x0089_2520, &args![actor, 0x6000u32, extra, 0u32]);
        assert_eq!(
            calls_to(&e, PLAYER_ADD_NOTE),
            vec![vec![actor.addr(), 0x6000, 1]]
        );
        // The extra data list is destroyed through its first virtual slot.
        assert_eq!(call_count(&e, slot_target(&e, extra, 0)), 1);
        assert_eq!(call_count(&e, ACTOR_ADD_ITEM_WORKER), 0);
    }

    #[test]
    fn items_are_added_through_the_worker_and_the_combat_controller_is_told() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        set_slot(&mut e, actor.cast(), 0x428, ret(0x3300));
        fn_00892520(&mut e, actor, Ptr::new(0x6000), Ptr::NULL, 7);
        assert_eq!(
            calls_to(&e, ACTOR_ADD_ITEM_WORKER),
            vec![vec![actor.addr(), 0x6000, 0, 7]]
        );
        assert_eq!(calls_to(&e, COMBAT_CONTROLLER_CALL), vec![vec![0x3300, 1]]);
    }

    #[test]
    fn a_weapon_given_to_an_npc_is_equipped() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        let entry = Ptr::<()>::new(0x6100);
        set_slot(&mut e, actor.cast(), 0x218, ret(1));
        set_slot(&mut e, actor.cast(), 0x3bc, ret(entry.addr()));
        stub(&mut e, FORM_TYPE, 0x28);
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        fn_00892520(&mut e, actor, Ptr::new(0x6000), Ptr::NULL, 0);
        let equip_args = vec![actor.addr(), 0xf0f0, 1, 0, 1, 0, 1];
        assert_eq!(
            calls_to(&e, ACTOR_QUEUE_EQUIP_OBJECT),
            vec![equip_args.clone()]
        );
        assert_eq!(calls_to(&e, ENTRY_DELETE), vec![vec![entry.addr(), 1]]);
        assert_eq!(call_count(&e, ACTOR_EQUIP_OBJECT), 0);

        // In menu mode with a forced update the equip is immediate.
        stub(&mut e, INTERFACE_IS_IN_MENU_MODE, 1);
        stub(&mut e, PROCESS_GET_FORCE_NEXT_UPDATE, 1);
        fn_00892520(&mut e, actor, Ptr::new(0x6000), Ptr::NULL, 0);
        assert_eq!(calls_to(&e, ACTOR_EQUIP_OBJECT), vec![equip_args]);
        assert_eq!(call_count(&e, ACTOR_QUEUE_EQUIP_OBJECT), 1);
    }

    #[test]
    fn a_reference_in_the_extra_list_gets_the_actor_and_its_targeters_move_on() {
        let mut e = engine();
        let (actor, refr) = pickup_setup(&mut e);
        let extra = Ptr::<()>::new(0x6200);
        stub(&mut e, EXTRA_GET_REFERENCE_POINTER, refr.addr());
        stub(&mut e, REFR_FORM_ID, 0x44);
        fn_00892520(&mut e, actor, Ptr::new(0x6000), extra, 0);
        assert_eq!(
            calls_to(&e, EXTRA_SET_ACTOR),
            vec![vec![0x5000, actor.addr()]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, refr, 0x48)),
            vec![vec![refr.addr(), 0x400]]
        );
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_FIND_ACTORS),
            vec![vec![PROCESS_LISTS, 0x44, actor.addr()]]
        );
    }

    #[test]
    fn the_players_ownership_is_removed_when_the_player_picks_up_the_item() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        set_player(&mut e, actor);
        stub(&mut e, EXTRA_GET_OWNER_FORM, BASE_FORM);
        fn_00892520(&mut e, actor, Ptr::new(0x6000), Ptr::new(0x6200), 0);
        assert_eq!(calls_to(&e, EXTRA_REMOVE_OWNERSHIP), vec![vec![0x6200]]);
    }

    #[test]
    fn ammunition_regeneration_requips_a_weapon_without_regeneration() {
        let mut e = engine();
        e.map(ZERO_DOUBLE, 8);
        let weapon_item = Ptr::<()>::new(0x6300);
        let actor = actor_with(
            &mut e,
            &[(0x218, ret(0)), (0x428, ret(0)), (0x3ec, ret(0))],
            Some(&[(0x14c, ret(0)), (0x148, ret(weapon_item.addr()))]),
        );
        set_player(&mut e, actor);
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        stub(&mut e, FORM_TYPE, 5);
        // The weapon's ammunition is the picked-up item.
        stub(&mut e, WEAPON_GET_CURRENT_AMMO, 0x6000);
        stub(&mut e, ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, 1);
        stub(&mut e, ACTOR_IS_WEAPON_DRAWN, 1);
        stub_float(&mut e, WEAPON_GET_AMMO_REGEN_RATE, 0.0);
        fn_00892520(&mut e, actor, Ptr::new(0x6000), Ptr::NULL, 0);
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), 0x3ec)),
            vec![vec![actor.addr(), 0xf0f0, 1, 1, 0]]
        );
        // With regeneration nothing is re-equipped.
        stub_float(&mut e, WEAPON_GET_AMMO_REGEN_RATE, 2.0);
        fn_00892520(&mut e, actor, Ptr::new(0x6000), Ptr::NULL, 0);
        assert_eq!(call_count(&e, slot_target(&e, actor.cast(), 0x3ec)), 1);
    }

    #[test]
    fn the_player_cannot_wear_clothes_during_an_animation_action() {
        let mut e = engine();
        e.map(MESSAGE_DURATION, 4);
        e.set_global(MESSAGE_DURATION, 2.0f32);
        let (actor, _) = pickup_setup(&mut e);
        set_player(&mut e, actor);
        stub(&mut e, EXTRA_GET_WORN, 1);
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 3);
        stub(&mut e, MESSAGE_TEXT, 0x555);
        let result = e.call(
            0x0089_29a0,
            &args![actor, 0x6000u32, 0x6200u32, 1u32, 0u32, 0u32],
        );
        assert_eq!(result.u32(), 0);
        assert_eq!(
            calls_to(&e, SHOW_MESSAGE),
            vec![vec![0x555, 0, MESSAGE_ICON_PATH, 0, 2.0f32.to_bits(), 0]]
        );
        assert_eq!(
            call_count(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_ITEM)),
            0
        );
    }

    #[test]
    fn items_are_given_through_slot_17c_and_the_reference_is_reset() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        let result = object(&mut e, 0x100, &[(0x1d0, ret(0x77)), (0x48, ret(0))]);
        set_slot(
            &mut e,
            actor.cast(),
            ACTOR_SLOT_ADD_ITEM,
            ret(result.addr()),
        );
        let given = e
            .call(
                0x0089_29a0,
                &args![actor, 0x6000u32, 0u32, 4u32, 8u32, 9u32],
            )
            .ptr::<()>();
        assert_eq!(given, result);
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_ITEM)),
            vec![vec![actor.addr(), 0x6000, 0, 4, 0, 1, 0, 8, 9, 1, 0]]
        );
        assert_eq!(calls_to(&e, RESET_SIM), vec![vec![0x77, 1]]);
        // The reference's base form is handed to the actor-form worker.
        assert_eq!(
            calls_to(&e, ACTOR_FORM_WORKER),
            vec![vec![actor.addr(), BASE_FORM, 0, 0]]
        );
    }

    #[test]
    fn items_for_a_cell_are_added_into_it_directly() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        set_slot(&mut e, actor.cast(), ACTOR_SLOT_ADD_ITEM, ret(0x1234));
        stub(&mut e, ADD_INTO_CELL_ENABLED, 1);
        stub(&mut e, CELL_GET_ADD_TARGET, 0);
        stub(&mut e, CELL_FIND_ADD_TARGET, 0x888);
        let given = fn_008929a0(&mut e, actor, Ptr::new(0x6000), Ptr::NULL, 4, 8, 9);
        assert_eq!(given, Ptr::NULL);
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_ITEM)),
            vec![vec![actor.addr(), 0x6000, 0, 4, 0, 0, 0x888, 0, 0, 1, 0]]
        );
        assert_eq!(
            calls_to(&e, CELL_FIND_ADD_TARGET),
            vec![vec![0x9000, actor.addr()]]
        );
    }

    #[test]
    fn a_weapon_given_to_an_npc_gets_its_ammunition() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        let result = object(&mut e, 0x100, &[(0x1d0, ret(0x77)), (0x48, ret(0))]);
        set_slot(
            &mut e,
            actor.cast(),
            ACTOR_SLOT_ADD_ITEM,
            ret(result.addr()),
        );
        stub(&mut e, FORM_TYPE, 0x28);
        stub(&mut e, WEAPON_GET_CURRENT_AMMO, 0xa1);
        stub(&mut e, WEAPON_GET_FORM_CLIP_ROUNDS, 3);
        stub(&mut e, AMMO_COUNT_FOR_CLIP, 5);
        fn_008929a0(&mut e, actor, Ptr::new(0x6000), Ptr::NULL, 1, 0, 0);
        assert_eq!(calls_to(&e, AMMO_COUNT_FOR_CLIP), vec![vec![0, 3]]);
        assert_eq!(calls_to(&e, EXTRA_SET_AMMO), vec![vec![0x5000, 0xa1, 5]]);
        assert_eq!(
            calls_to(&e, slot_target(&e, result, 0x48)),
            vec![vec![result.addr(), 0x800]]
        );
    }

    #[test]
    fn a_worn_weapons_animation_gets_its_time_range_adjusted() {
        let mut e = engine();
        e.map(FLOAT_MAX, 4);
        e.set_global(FLOAT_MAX, f32::MAX);
        let (actor, _) = pickup_setup(&mut e);
        let animation = Ptr::<()>::new(0x6400);
        set_slot(&mut e, actor.cast(), 0x1e4, ret(animation.addr()));
        set_slot(&mut e, actor.cast(), ACTOR_SLOT_ADD_ITEM, ret(0));
        stub(&mut e, EXTRA_GET_WORN, 1);
        stub(&mut e, FORM_TYPE, 0x28);
        stub(&mut e, ANIMATION_GET_SEQUENCE, 0x6500);
        stub(&mut e, ANIMATION_ZERO_GLOBAL_TRANSFORM, 0x6600);
        stub_float(&mut e, ANIMATION_FLOAT_D0, 2.0);
        stub_float(&mut e, SEQUENCE_GET_SCALED_TIME, 1.5);
        stub_float(&mut e, SEQUENCE_FLOAT_30, 0.25);
        stub_float(&mut e, SEQUENCE_FLOAT_48, 0.125);
        fn_008929a0(&mut e, actor, Ptr::new(0x6000), Ptr::new(0x6200), 1, 0, 0);
        assert_eq!(
            calls_to(&e, ANIMATION_APPLY_TIME_RANGE),
            vec![vec![
                0x6600,
                actor.addr(),
                1.5f32.to_bits(),
                2.375f32.to_bits(),
                0x6500
            ]]
        );
        // The sentinel -FLT_MAX leaves it alone.
        stub_float(&mut e, SEQUENCE_GET_SCALED_TIME, -f32::MAX);
        fn_008929a0(&mut e, actor, Ptr::new(0x6000), Ptr::new(0x6200), 1, 0, 0);
        assert_eq!(call_count(&e, ANIMATION_APPLY_TIME_RANGE), 1);
    }

    #[test]
    fn owned_goods_in_an_owned_cell_get_the_players_ownership_when_cheap_enough() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        set_slot(&mut e, actor.cast(), ACTOR_SLOT_ADD_ITEM, ret(0));
        let extra = Ptr::<()>::new(0x6200);
        // The cell has an owner; the item is worth 10, the threshold is 25.
        stub(&mut e, CELL_GET_OWNER, 0x7777);
        stub(&mut e, VALUE_FORM_GET_FORM_VALUE, 10);
        let setting = e.mem.alloc(4);
        e.mem.set_f32(setting, 25.0);
        stub(&mut e, SETTING_GET_VALUE_POINTER, setting);
        fn_008929a0(&mut e, actor, Ptr::new(0x6000), extra, 1, 0, 0);
        assert_eq!(
            calls_to(&e, EXTRA_SET_OWNERSHIP),
            vec![vec![0x6200, BASE_FORM]]
        );
        // Worth more than the threshold: no ownership is set.
        stub(&mut e, VALUE_FORM_GET_FORM_VALUE, 100);
        fn_008929a0(&mut e, actor, Ptr::new(0x6000), extra, 1, 0, 0);
        assert_eq!(call_count(&e, EXTRA_SET_OWNERSHIP), 1);
    }

    #[test]
    fn dropping_items_at_a_target_repeats_for_the_count() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        let target = object(&mut e, 0x100, &[(0x1f4, ret(0)), (0x218, ret(0))]);
        // The actor's and the target's positions (z at +8).
        let actor_position = e.mem.alloc(16);
        e.mem.set_f32(actor_position + 8, 10.0);
        let target_position = e.mem.alloc(16);
        e.mem.set_f32(target_position + 8, 20.0);
        set_slot(&mut e, actor.cast(), 0x1f4, ret(actor_position));
        set_slot(&mut e, target, 0x1f4, ret(target_position));
        set_slot(&mut e, actor.cast(), ACTOR_SLOT_ADD_ITEM, ret(0));
        set_slot(&mut e, actor.cast(), 0x3bc, ret(0));
        let acquire = object(&mut e, 0x40, &[(0x44, ret(0))]);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(&mut e, GET_INVENTORY_ITEM, 0x7100);
        stub(&mut e, CHANGES_IS_EMPTY, 1);
        stub(&mut e, GET_CONTAINER_CHANGES, 0x2000);
        stub_float(&mut e, ACTOR_GET_EYE_LEVEL, 5.0);
        let done = e.call(
            0x0089_2e90,
            &args![actor, 0x6000u32, 0u32, target, 2u32, 77u32],
        );
        assert!(!done.bool());
        let adds = calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_ITEM));
        assert_eq!(adds.len(), 2);
        assert_eq!(
            adds[0],
            vec![actor.addr(), 0x6000, 0, 2, 0, 0, target.addr(), 0, 0, 1, 0]
        );
        assert_eq!(adds[1][3], 1);
        assert_eq!(
            calls_to(&e, GET_INVENTORY_ITEM),
            vec![vec![0x2000, 0x6000, 77]; 2]
        );
        assert_eq!(call_count(&e, EXTRA_REMOVE_CONTAINER_CHANGES), 1);
        // The idle: the item level is 1 because the target is above the eyes.
        assert_eq!(
            calls_to(&e, IDLE_MANAGER_SET_USED_ITEM_LEVEL),
            vec![vec![1], vec![0xffff_ffff]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, acquire, 0x44)),
            vec![vec![acquire.addr(), actor.addr(), 0, 2, 1, 0, 1]]
        );
        assert_eq!(
            calls_to(&e, IDLE_MANAGER_SET_USED_ITEM_ACTIVATE),
            vec![vec![1], vec![0]]
        );
    }

    #[test]
    fn the_player_dropping_items_refreshes_the_inventory_instead() {
        let mut e = engine();
        let (actor, _) = pickup_setup(&mut e);
        set_player(&mut e, actor);
        stub(&mut e, GET_CONTAINER_CHANGES, 0);
        let target = Ptr::<()>::new(0x6700);
        assert!(!fn_00892e90(
            &mut e,
            actor,
            Ptr::new(0x6000),
            0,
            target,
            1,
            0
        ));
        assert_eq!(call_count(&e, INTERFACE_REFRESH_INVENTORY), 1);
        assert_eq!(call_count(&e, IDLE_MANAGER_SET_USED_ITEM), 0);
    }

    /// An actor with a process and the stubs `Actor::StartAttack` and
    /// `Actor::QueueAttack` need to get through a plain attack: the weapon is
    /// drawn, the process can attack, there is an animation with an attack
    /// sequence and every group is an attack group.
    fn attack_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, Ptr) {
        let animation = Ptr::<()>::new(e.mem.alloc(0x200));
        let acquire = object(
            e,
            0x40,
            &[
                (0x27c, ret(0)),
                (0x1b0, ret(0)),
                (0x44, ret(0)),
                (0x614, ret(0)),
                (0x22c, ret(0)),
                (0x28, ret(0)),
                (0xe4, float_ret(0.0)),
                (0xe8, ret(0)),
                (0x710, ret(0)),
                (0x30c, ret(0)),
                (0x1e0, float_ret(0.0)),
            ],
        );
        let position = e.mem.alloc(16);
        let actor = actor_with(
            e,
            &[
                (0x218, ret(0)),
                (0x22c, ret(0)),
                (0x1e4, ret(animation.addr())),
                (0x4b0, ret(0)),
                (0x1f4, ret(position)),
                (0x390, ret(0)),
                (0x2f4, ret(0)),
                (0x214, ret(0)),
                (0x234, ret(0)),
                (0x230, ret(0)),
                (0x428, ret(0)),
            ],
            Some(&[
                (0x148, ret(0)),
                (0x14c, ret(0)),
                (0x3f8, ret(1)),
                (0x3f4, ret(0)),
                (0x6dc, ret(0)),
                (0x368, ret(0)),
                (0x614, ret(0)),
                (0x454, ret(0)),
            ]),
        );
        stub(e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(e, ACTOR_IS_WEAPON_DRAWN, 1);
        stub(e, ANIM_GROUP_IS_ATTACK_ACTION, 1);
        stub(e, ANIM_GROUP_COMPOSE, 0x55);
        stub(e, ANIMATION_PICK_BEST_ANIMATION, 0x55);
        stub(e, ANIM_GROUP_GET_TYPE, 0x55);
        stub(e, ANIMATION_GET_SEQUENCE, 0x6500);
        stub(e, ANIMATION_ZERO_GLOBAL_TRANSFORM, 0x6600);
        stub(e, TRANSFORM_U16, 0x1234);
        e.map(ANIM_GROUP_TABLE, 0x1000);
        e.map(ZERO_DOUBLE, 8);
        (actor, animation, acquire)
    }

    #[test]
    fn non_player_actors_do_not_queue_attacks_while_the_setting_is_set() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        let setting = e.mem.alloc(4);
        e.mem.set_u8(setting + 4, 0);
        stub(&mut e, SETTING_VALUE_POINTER, setting);
        e.mem.set_u8(setting, 1);
        assert!(!actor_queue_attack(&mut e, actor, 0x30));
        assert_eq!(call_count(&e, ACTOR_IS_WEAPON_DRAWN), 0);
    }

    #[test]
    fn a_queued_attack_group_is_remembered_for_npcs() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        let setting = e.mem.alloc(4);
        stub(&mut e, SETTING_VALUE_POINTER, setting);
        assert!(e.call(0x0089_35f0, &args![actor, 0x30u32]).bool());
        assert_eq!(e.get(actor, Actor::eQueuedattack), 0x30);
    }

    #[test]
    fn a_queued_group_that_is_not_an_attack_is_reported() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        let setting = e.mem.alloc(4);
        stub(&mut e, SETTING_VALUE_POINTER, setting);
        stub(&mut e, ANIM_GROUP_IS_ATTACK_ACTION, 0);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0x31);
        e.mem.set_u32(ANIM_GROUP_TABLE + 0x30 * 0x24, 0x7001);
        e.mem.set_u32(ANIM_GROUP_TABLE + 0x31 * 0x24, 0x7002);
        stub(&mut e, REFR_GET_NAME, 0x7003);
        assert!(!actor_queue_attack(&mut e, actor, 0x30));
        assert_eq!(
            calls_to(&e, DEBUG_PRINT),
            vec![vec![
                QUEUE_NON_ATTACK_FORMAT,
                0x7003,
                HAND_TO_HAND_NAME,
                0x7001,
                0x7002
            ]]
        );
        assert_eq!(e.get(actor, Actor::eQueuedattack), 0);
    }

    #[test]
    fn the_player_starts_an_attack_directly() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        set_player(&mut e, actor);
        vtable_at(&mut e, actor.addr() + 0xa4, &[(0x08, ret(0))]);
        let setting_value = e.mem.alloc(4);
        stub(&mut e, SETTING_GET_VALUE_POINTER, setting_value);
        assert!(actor_queue_attack(&mut e, actor, 0x30));
        // `Actor::StartAttack` ran: it played the group through 008b28c0.
        assert_eq!(call_count(&e, ACTOR_PLAY_GROUP_WORKER), 1);
    }

    #[test]
    fn a_jamming_weapon_queues_its_jam_attack() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        let weapon_item = 0x6800;
        let process = e.get(actor, Actor::pCurrentProcess);
        set_slot(&mut e, process, 0x148, ret(weapon_item));
        set_slot(&mut e, process, 0x14c, ret(0x6900));
        let setting = e.mem.alloc(4);
        stub(&mut e, SETTING_VALUE_POINTER, setting);
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        stub(&mut e, WEAPON_GET_CURRENT_AMMO, 0xa1);
        stub(&mut e, LIST_NEXT, 9);
        stub_float(&mut e, GET_WEAPON_CONDITION_JAM_MULT, 1.0);
        stub(&mut e, CHANCE_ROLL, 1);
        stub(&mut e, WEAPON_GET_ATTACK_ANIM, 0x10);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x27);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0x27);
        assert!(!actor_queue_attack(&mut e, actor, 0x30));
        assert_eq!(e.get(actor, Actor::eQueuedattack), 0x27);
        assert_eq!(calls_to(&e, CHANCE_ROLL), vec![vec![1.0f32.to_bits()]]);
    }

    #[test]
    fn a_weapon_with_ammunition_but_no_ammunition_item_cannot_be_queued() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        let process = e.get(actor, Actor::pCurrentProcess);
        set_slot(&mut e, process, 0x148, ret(0x6800));
        let setting = e.mem.alloc(4);
        stub(&mut e, SETTING_VALUE_POINTER, setting);
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        stub(&mut e, WEAPON_GET_CURRENT_AMMO, 0xa1);
        assert!(!actor_queue_attack(&mut e, actor, 0x30));
        assert_eq!(call_count(&e, ACTOR_IS_WEAPON_DRAWN), 0);
    }

    #[test]
    fn starting_a_plain_attack_plays_the_group_in_slot_4() {
        let mut e = engine();
        let (actor, animation, _) = attack_setup(&mut e);
        assert!(e.call(0x0089_3a40, &args![actor, 0x30u32]).bool());
        assert_eq!(
            calls_to(&e, ANIMATION_SET_SPEED),
            vec![vec![animation.addr(), 1.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_PLAY_GROUP_WORKER),
            vec![vec![actor.addr(), 0x30, animation.addr()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION),
            vec![vec![actor.addr(), 2, 0x6500]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), 0x4b0)),
            vec![vec![actor.addr(), 0x1234, 1]]
        );
    }

    #[test]
    fn starting_an_attack_with_no_ammunition_fails() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        let process = e.get(actor, Actor::pCurrentProcess);
        set_slot(&mut e, process, 0x148, ret(0x6800));
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        stub(&mut e, WEAPON_GET_CURRENT_AMMO, 0xa1);
        assert!(!actor_start_attack(&mut e, actor, 0x30));
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x3f4)),
            vec![vec![process.addr(), 0]]
        );
        assert_eq!(call_count(&e, ANIMATION_SET_SPEED), 0);
    }

    #[test]
    fn the_player_hears_the_dry_fire_sound() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        set_player(&mut e, actor);
        let process = e.get(actor, Actor::pCurrentProcess);
        set_slot(&mut e, process, 0x148, ret(0x6800));
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        stub(&mut e, WEAPON_GET_CURRENT_AMMO, 0xa1);
        stub(&mut e, WEAPON_SOUND_SET, 0x700);
        stub(&mut e, DRY_FIRE_ALLOWED, 1);
        stub(&mut e, AUDIO_INSTANCE, 0x4400);
        stub(&mut e, AUDIO_GET_SOUND_HANDLE_BY_FILENAME, 0x4500);
        stub(&mut e, SOUND_SET_FILE_NAME, 0x4600);
        assert!(!actor_start_attack(&mut e, actor, 0x30));
        let lookups = calls_to(&e, AUDIO_GET_SOUND_HANDLE_BY_FILENAME);
        assert_eq!(lookups.len(), 1);
        // audio, out handle, file name, flags (0x200101 without the weapon
        // test), sound set
        assert_eq!(lookups[0][0], 0x4400);
        assert_eq!(lookups[0][2..], [0x4600, 0x20_0101, 0x700]);
        assert_eq!(call_count(&e, SOUND_HANDLE_PLAY), 1);
        assert_eq!(call_count(&e, EXTRA_SET_SOUND), 1);
        // Both temporary sound handles are destroyed again.
        assert_eq!(call_count(&e, SOUND_HANDLE_DESTROY), 2);
        // The dry-fire is not allowed: the attack just fails.
        stub(&mut e, DRY_FIRE_ALLOWED, 0);
        assert!(!actor_start_attack(&mut e, actor, 0x30));
        assert_eq!(call_count(&e, SOUND_HANDLE_PLAY), 1);
    }

    #[test]
    fn a_power_attack_group_is_played_through_the_worker() {
        let mut e = engine();
        let (actor, animation, _) = attack_setup(&mut e);
        let process = e.get(actor, Actor::pCurrentProcess);
        set_slot(&mut e, process, 0x148, ret(0x6800));
        set_slot(&mut e, process, 0x14c, ret(0x6900));
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        stub(&mut e, WEAPON_GET_CURRENT_AMMO, 0xa1);
        stub(&mut e, LIST_NEXT, 9);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 210);
        assert!(!actor_start_attack(&mut e, actor, 210));
        assert_eq!(
            calls_to(&e, ACTOR_PLAY_GROUP_WORKER),
            vec![vec![actor.addr(), 210, animation.addr()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION),
            vec![vec![actor.addr(), 9, 0x6500]]
        );
    }

    #[test]
    fn a_group_that_is_not_an_attack_is_only_reported() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        stub(&mut e, ANIM_GROUP_IS_ATTACK_ACTION, 0);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0x31);
        e.mem.set_u32(ANIM_GROUP_TABLE + 0x30 * 0x24, 0x7001);
        e.mem.set_u32(ANIM_GROUP_TABLE + 0x31 * 0x24, 0x7002);
        stub(&mut e, REFR_GET_NAME, 0x7003);
        assert!(!actor_start_attack(&mut e, actor, 0x30));
        assert_eq!(
            calls_to(&e, DEBUG_PRINT),
            vec![vec![
                START_NON_ATTACK_FORMAT,
                0x7003,
                HAND_TO_HAND_NAME,
                0x7001,
                0x7002
            ]]
        );
    }

    #[test]
    fn a_strong_actor_may_swap_the_attack_for_group_0x38() {
        let mut e = engine();
        let (actor, _, _) = attack_setup(&mut e);
        e.map(RANDOM_ROLL_MAX, 4);
        e.set_global(RANDOM_ROLL_MAX, 100.0f32);
        set_slot(&mut e, actor.cast(), 0x218, ret(1));
        vtable_at(&mut e, actor.addr() + 0xa4, &[(0x08, ret(50))]);
        // Settings: threshold A 10 (< 50) with factor 0.5, B never.
        let values = [
            (SETTING_THRESHOLD_A, 10.0f32),
            (SETTING_FACTOR_A, 0.5),
            (SETTING_THRESHOLD_B, 1000.0),
            (SETTING_FACTOR_B, 0.1),
        ];
        stub_with(&mut e, SETTING_GET_VALUE_POINTER, move |e, a| {
            let value = values.iter().find(|(object, _)| *object == a[0]).unwrap().1;
            let slot = e.mem.alloc(4);
            e.mem.set_f32(slot, value);
            ret(slot)
        });
        // The roll (10) is below the first chance (25): group 0x38.
        stub_float(&mut e, RANDOM_FLOAT, 10.0);
        e.call(0x0089_3a40, &args![actor, 0x30u32]);
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE)[0][2], 0x38);
        // A roll above both chances keeps the group.
        let mut e2 = engine();
        let (actor2, _, _) = attack_setup(&mut e2);
        e2.map(RANDOM_ROLL_MAX, 4);
        e2.set_global(RANDOM_ROLL_MAX, 100.0f32);
        set_slot(&mut e2, actor2.cast(), 0x218, ret(1));
        vtable_at(&mut e2, actor2.addr() + 0xa4, &[(0x08, ret(50))]);
        stub_with(&mut e2, SETTING_GET_VALUE_POINTER, move |e, a| {
            let value = values.iter().find(|(object, _)| *object == a[0]).unwrap().1;
            let slot = e.mem.alloc(4);
            e.mem.set_f32(slot, value);
            ret(slot)
        });
        stub_float(&mut e2, RANDOM_FLOAT, 99.0);
        // 25 >= 99 is false and 99 < 25 + 0 is false: unchanged.
        e2.call(0x0089_3a40, &args![actor2, 0x30u32]);
        assert_eq!(calls_to(&e2, ANIM_GROUP_COMPOSE)[0][2], 0x30);
    }

    #[test]
    fn animation_actions_two_to_six_are_the_blocking_range() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        for (action, expected) in [(-1i32, false), (1, false), (2, true), (6, true), (7, false)] {
            stub(&mut e, ACTOR_GET_ANIM_ACTION, action as u32);
            assert_eq!(e.call(0x0089_4900, &args![actor]).bool(), expected);
        }
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 7);
        assert!(e.call(0x0089_4d60, &args![actor]).bool());
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 6);
        assert!(!actor_get_blocked(&mut e, actor));
    }

    /// An attack setup plus the stubs the blocking and idle animation
    /// functions use.
    fn block_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, Ptr) {
        let (actor, animation, acquire) = attack_setup(e);
        set_slot(e, actor.cast(), 0x214, ret(0));
        set_slot(e, actor.cast(), 0x428, ret(0x3300));
        stub(e, ACTOR_PROCESS_OBJECT, 0x3400);
        stub(e, ACTOR_GET_ANIMATION, animation.addr());
        stub(e, SEQUENCE_GET_GENERIC_LOCATION, 1);
        (actor, animation, acquire)
    }

    #[test]
    fn a_blocking_actor_starts_the_block_group() {
        let mut e = engine();
        let (actor, animation, _) = block_setup(&mut e);
        e.mem.set_u8(actor.addr() + 0xf0, 1);
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 0xffff_ffff);
        stub(&mut e, ANIM_GROUP_GET_NUMBER, 0x10);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x77);
        fn_00894940(&mut e, actor);
        assert_eq!(
            calls_to(&e, ANIMATION_PLAY_GROUP),
            vec![vec![animation.addr(), 0x77, 1, 0xffff_ffff, 0xffff_ffff]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION),
            vec![vec![actor.addr(), 7, 0x6500]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), 0x4b0)),
            vec![vec![actor.addr(), 0x77, 1]]
        );
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_SET_BLOCKED),
            vec![vec![0x3400, 1]]
        );
    }

    #[test]
    fn a_block_group_that_is_not_generic_only_notifies_the_combat_controller() {
        let mut e = engine();
        let (actor, _, _) = block_setup(&mut e);
        stub(&mut e, SEQUENCE_GET_GENERIC_LOCATION, 0);
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 7);
        fn_00894940(&mut e, actor);
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_SET_BLOCKED),
            vec![vec![0x3400, 1]]
        );
        assert_eq!(call_count(&e, ANIMATION_PLAY_GROUP), 0);
    }

    #[test]
    fn a_stopped_block_clears_the_group_with_a_blend_time() {
        let mut e = engine();
        let (actor, animation, _) = block_setup(&mut e);
        let setting = e.mem.alloc(4);
        e.mem.set_f32(setting, 0.25);
        e.map(THIRTY, 8);
        e.set_global(THIRTY, 30.0f64);
        stub(&mut e, SETTING_GET_VALUE_POINTER, setting);
        stub(&mut e, ANIM_GROUP_GET_NUMBER, 0xaa);
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 7);
        fn_00894940(&mut e, actor);
        assert_eq!(
            calls_to(&e, ANIMATION_CLEAR_GROUP),
            vec![vec![animation.addr(), 2, 0.25f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION),
            vec![vec![actor.addr(), 0xffff_ffff, 0]]
        );
        // A byte from the transform gives the blend time in 30ths.
        clear_log(&mut e);
        stub(&mut e, TRANSFORM_BYTE, 15);
        fn_00894940(&mut e, actor);
        assert_eq!(
            calls_to(&e, ANIMATION_CLEAR_GROUP),
            vec![vec![animation.addr(), 2, 0.5f32.to_bits()]]
        );
        // Still blocking: nothing is cleared.
        clear_log(&mut e);
        e.mem.set_u8(actor.addr() + 0xf0, 1);
        fn_00894940(&mut e, actor);
        assert_eq!(call_count(&e, ANIMATION_CLEAR_GROUP), 0);
    }

    #[test]
    fn setting_the_block_applies_it() {
        let mut e = engine();
        let (actor, _, acquire) = block_setup(&mut e);
        assert!(e.call(0x0089_4cc0, &args![actor, 1u32]).bool());
        assert_eq!(e.mem.u8(actor.addr() + 0xf0), 1);
        assert_eq!(
            calls_to(&e, slot_target(&e, acquire, 0x614)),
            vec![vec![acquire.addr(), 0x8000]]
        );
        // Without an animation the combat controller is told and it fails.
        set_slot(&mut e, actor.cast(), 0x1e4, ret(0));
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 7);
        assert!(!actor_set_block(&mut e, actor, 1));
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_SET_BLOCKED),
            vec![vec![0x3400, 1]]
        );
    }

    #[test]
    fn idle_groups_are_started_by_type() {
        let mut e = engine();
        let (actor, animation, _) = block_setup(&mut e);
        set_group(&mut e, 0xab, 4, 0);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0xab);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x66);
        e.call(0x0089_4d90, &args![actor, 0u32]);
        assert_eq!(
            calls_to(&e, ANIMATION_PLAY_GROUP),
            vec![vec![animation.addr(), 0x66, 1, 0xffff_ffff, 0xffff_ffff]]
        );
        // The table gives the slot (4) whose sequence becomes the action's.
        assert_eq!(
            calls_to(&e, ANIMATION_GET_SEQUENCE),
            vec![vec![animation.addr(), 4]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION),
            vec![vec![actor.addr(), 7, 0x6500]]
        );
        // Busy actors do nothing.
        stub(&mut e, ACTOR_GET_ANIM_ACTION, 10);
        fn_00894d90(&mut e, actor, 0);
        assert_eq!(call_count(&e, ANIMATION_PLAY_GROUP), 1);
    }

    #[test]
    fn the_second_idle_clears_slot_4_unless_the_group_matches() {
        let mut e = engine();
        let (actor, animation, _) = block_setup(&mut e);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0xac);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x66);
        e.call(0x0089_4e90, &args![actor]);
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION),
            vec![vec![actor.addr(), 8, 0x6500]]
        );
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0x10);
        set_player(&mut e, actor);
        stub(&mut e, PLAYER_GET_ANIMATION, 0x6a00);
        fn_00894e90(&mut e, actor);
        assert_eq!(
            calls_to(&e, ANIMATION_CLEAR_GROUP),
            vec![
                vec![animation.addr(), 4, 0.0f32.to_bits()],
                vec![0x6a00, 4, 0.0f32.to_bits()]
            ]
        );
    }

    #[test]
    fn walking_directions_pick_a_movement_group() {
        let mut e = engine();
        let (actor, animation, _) = block_setup(&mut e);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x0b02);
        stub_float(&mut e, ACTOR_GET_WALK_SPEED, 150.0);
        stub(&mut e, ANIMATION_MOVEMENT_SPEED, 100);
        // Direction bit 2 asks for group 0xc, which the actor maps to 0xb02.
        let asked = e.call(0x0089_4f90, &args![actor, 2u32]).u32();
        assert_eq!(asked, 0xff);
        // 0xb02 is outside 0xb..=0xe (it is 0xb02): the group is refused.
        assert_eq!(call_count(&e, ANIMATION_PLAY_GROUP), 0);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x000c);
        stub_with(&mut e, ANIMATION_MOVEMENT_SPEED, |_, a| {
            ret(if a[1] == 3 { 100 } else { 7 })
        });
        assert_eq!(fn_00894f90(&mut e, actor, 2), 0xc);
        assert_eq!(
            calls_to(&e, ANIMATION_PLAY_GROUP),
            vec![vec![animation.addr(), 0xc, 1, 0xffff_ffff, 0xffff_ffff]]
        );
        // The speed multiplier is walk speed over the group's speed.
        assert_eq!(e.mem.f32(animation.addr() + 0x10c), 1.5);
        // No direction bits: asks for 0xff and gets nothing.
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0xff);
        assert_eq!(fn_00894f90(&mut e, actor, 0), 0xff);
    }

    #[test]
    fn small_accessors() {
        let mut e = engine();
        let object = Ptr::<()>::new(e.mem.alloc(0x800));
        fn_008950f0(&mut e, object, 2.5);
        assert_eq!(e.mem.f32(object.addr() + 0x10c), 2.5);
        assert_eq!(e.call(0x0089_50f0, &args![object, 1.0f32]).u32(), 0);

        e.mem.set_u32(object.addr() + 0x12c, 4);
        assert!(e.call(0x0089_7890, &args![object]).bool());
        e.mem.set_u32(object.addr() + 0x12c, 3);
        assert!(!fn_00897890(&mut e, object));

        e.map(0x011c_6000, 0x1000);
        e.set_global(0x011c_6230, 11u32);
        e.set_global(0x011c_6234, 22u32);
        assert_eq!(e.call(0x0089_78b0, &args![]).u32(), 11);
        assert_eq!(e.call(0x0089_78c0, &args![]).u32(), 22);

        stub(&mut e, 0x0044_8a60, 1);
        assert!(e.call(0x0089_78d0, &args![object]).bool());
        assert_eq!(calls_to(&e, 0x0044_8a60), vec![vec![object.addr(), 0x100]]);

        e.call(0x0089_78f0, &args![object, 2u32, 9u32]);
        assert_eq!(e.mem.u8(object.addr() + 2 * 0x30 + 0x1bd), 9);

        e.mem.set_u32(object.addr() + 0x6a0, 0xabcd);
        assert_eq!(e.call(0x0089_7b30, &args![object]).u32(), 0xabcd);
    }

    #[test]
    fn package_setters() {
        let mut e = engine();
        let package = Ptr::<()>::new(e.mem.alloc(0x100));
        fn_00898200(&mut e, package, 1);
        assert_eq!(e.mem.u32(package.addr() + 0x1c), 0x0200_0000);
        e.mem.set_u32(package.addr() + 0x1c, 0xffff_ffff);
        e.call(0x0089_8200, &args![package, 0u32]);
        assert_eq!(e.mem.u32(package.addr() + 0x1c), 0xfdff_ffff);

        e.mem.set_f32(package.addr() + 0x90, 1.5);
        e.call(0x0089_8250, &args![package, 2.25f32]);
        assert_eq!(e.mem.f32(package.addr() + 0x90), 3.75);

        e.call(0x0089_8280, &args![package, 5u32]);
        assert_eq!(e.mem.u8(package.addr() + 0x81), 5);
        e.call(0x0089_82a0, &args![package, 6u32]);
        assert_eq!(e.mem.u8(package.addr() + 0xa8), 6);
    }

    #[test]
    fn the_anim_group_is_composed_from_the_actor_flags_and_the_weapon() {
        let mut e = engine();
        let (actor, animation, _) = attack_setup(&mut e);
        // No animation at all.
        set_slot(&mut e, actor.cast(), 0x1e4, ret(0));
        assert_eq!(
            actor_get_anim_group(&mut e, actor, 5, 0, 0, Ptr::NULL),
            0xff
        );
        set_slot(&mut e, actor.cast(), 0x1e4, ret(animation.addr()));

        // Flags 0x800 give selector 2; in combat the weapon decides the second
        // selector (no weapon: 1).
        stub(&mut e, ACTOR_ANIM_FLAGS, 0x800);
        stub(&mut e, ACTOR_IN_COMBAT, 1);
        stub(&mut e, ANIM_GROUP_COMPOSE, 0x1234);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x4321);
        assert_eq!(
            e.call(0x0089_7910, &args![actor, 5u32, 0u32, 0u32, 0u32])
                .u16(),
            0x4321
        );
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE), vec![vec![2, 1, 5, 0]]);
        assert_eq!(
            calls_to(&e, ANIMATION_PICK_BEST_ANIMATION),
            vec![vec![animation.addr(), 0x1234, 0]]
        );

        // A given weapon of type 3 takes its selector from the table; the
        // special state sets the last flag.
        clear_log(&mut e);
        e.map(WEAPON_TYPE_ANIM_TABLE, 0x40);
        e.mem.set_u32(WEAPON_TYPE_ANIM_TABLE + 12, 0x42);
        stub(&mut e, ENTRY_FORM, 0xf0f0);
        stub(&mut e, WEAPON_TYPE, 3);
        stub(&mut e, ACTOR_TEST_8BA3E0, 1);
        stub(&mut e, ACTOR_ANIM_FLAGS, 0x2000);
        actor_get_anim_group(&mut e, actor, 5, 0x6800, 0, Ptr::NULL);
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE), vec![vec![3, 0x42, 5, 1]]);

        // `0xffffffff` means no weapon; flags 0x400 give selector 1.
        clear_log(&mut e);
        stub(&mut e, ACTOR_ANIM_FLAGS, 0x400);
        actor_get_anim_group(&mut e, actor, 5, 0xffff_ffff, 0, Ptr::NULL);
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE), vec![vec![1, 1, 5, 1]]);

        // Not in combat and nothing special: the second selector stays 0.
        clear_log(&mut e);
        stub(&mut e, ACTOR_IN_COMBAT, 0);
        actor_get_anim_group(&mut e, actor, 5, 0, 0, Ptr::NULL);
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE), vec![vec![1, 0, 5, 1]]);
    }

    #[test]
    fn the_duration_of_a_group_comes_from_its_animation_group() {
        let mut e = engine();
        let (actor, animation, _) = attack_setup(&mut e);
        stub(&mut e, ANIMATION_GET_TES_ANIM_GROUP, 0x8000);
        stub_float(&mut e, ANIM_GROUP_GET_TIME, 3.5);
        assert_eq!(e.call(0x0089_7b50, &args![actor, 9u32]).f32(), 3.5);
        assert_eq!(
            calls_to(&e, ANIMATION_GET_TES_ANIM_GROUP),
            vec![vec![animation.addr(), 0x55]]
        );
        assert_eq!(calls_to(&e, ANIM_GROUP_GET_TIME), vec![vec![0x8000, 1]]);
        // No animation: 0.0.
        set_slot(&mut e, actor.cast(), 0x1e4, ret(0));
        assert_eq!(actor_get_anim_group_duration(&mut e, actor, 9), 0.0);
    }

    #[test]
    fn a_package_of_type_0xe_targets_the_actor_itself() {
        let mut e = engine();
        let (actor, _, acquire) = attack_setup(&mut e);
        stub(&mut e, PACKAGE_CREATE, 0x7100);
        stub(&mut e, PACKAGE_GET_TARGET, 0x7200);
        stub(&mut e, ACTOR_BASE_FORM, 0xb000);
        stub(&mut e, PACKAGE_LOCATION_CONSTRUCT, 0x7500);
        stub(&mut e, PACKAGE_TARGET_CONSTRUCT, 0x7400);
        stub(&mut e, FORM_FIELD_14, 77);
        set_slot(&mut e, acquire, 0x22c, ret(0x7300));
        e.call(0x0089_7bd0, &args![actor, 0u32]);
        assert_eq!(calls_to(&e, PACKAGE_CREATE), vec![vec![0xe]]);
        assert_eq!(calls_to(&e, PACKAGE_SET_PACK_TYPE), vec![vec![0x7100, 0xe]]);
        assert_eq!(calls_to(&e, PACKAGE_SET_FIELD_18), vec![vec![0x7100, 0x29]]);
        assert_eq!(
            calls_to(&e, PACKAGE_TARGET_SET_REFERENCE),
            vec![vec![0x7200, actor.addr()]]
        );
        assert_eq!(calls_to(&e, PACKAGE_COPY_FROM), vec![vec![0x7100, 0x7300]]);
        assert_eq!(calls_to(&e, FORM_FIELD_14), vec![vec![0xb090]]);
        assert_eq!(calls_to(&e, TARGET_SET_COUNT), vec![vec![0x7200, 77]]);
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_PACKAGE)),
            vec![vec![actor.addr(), 0x7100, 1, 1]]
        );
        // The location the package got is released again.
        assert_eq!(call_count(&e, PACKAGE_LOCATION_DESTROY), 1);
        assert_eq!(call_count(&e, PACKAGE_TARGET_DESTROY), 1);
    }

    #[test]
    fn avoiding_and_surfacing_packages_are_put_on_the_actor() {
        let mut e = engine();
        let (actor, _, acquire) = attack_setup(&mut e);
        stub(&mut e, PACKAGE_CREATE, 0x7100);
        stub(&mut e, PACKAGE_GET_TARGET, 0x7200);
        set_slot(&mut e, acquire, 0x22c, ret(0x7300));
        let process = e.get(actor, Actor::pCurrentProcess);
        set_slot(&mut e, process, 0x25c, ret(0));
        let target = Ptr::<()>::new(0x7400);
        e.call(0x0089_82c0, &args![actor, target, 2.5f32]);
        assert_eq!(calls_to(&e, ACTOR_END_MOVEMENT), vec![vec![actor.addr()]]);
        assert_eq!(calls_to(&e, PACKAGE_CREATE), vec![vec![0x1f]]);
        assert_eq!(calls_to(&e, REFR_SET_TARGETED), vec![vec![0x7400, 1]]);
        assert_eq!(
            calls_to(&e, PACKAGE_TARGET_SET_REFERENCE),
            vec![vec![0x7200, 0x7400]]
        );
        assert_eq!(
            calls_to(&e, PACKAGE_CALCULATE_PROCEDURE_TYPE),
            vec![vec![0x7100, 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x25c)),
            vec![vec![process.addr(), 2.5f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_PACKAGE)),
            vec![vec![actor.addr(), 0x7100, 1, 1]]
        );

        clear_log(&mut e);
        actor_initiate_surface_package(&mut e, actor);
        assert_eq!(
            calls_to(&e, ACTOR_FORCE_STOP_MOVING),
            vec![vec![actor.addr()]]
        );
        assert_eq!(calls_to(&e, PACKAGE_CREATE), vec![vec![0x1d]]);
        assert_eq!(
            calls_to(&e, PACKAGE_SET_PACK_TYPE),
            vec![vec![0x7100, 0x1d]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_PACKAGE)),
            vec![vec![actor.addr(), 0x7100, 1, 1]]
        );
    }

    /// A flee package block big enough for the setters.
    fn flee_package(e: &mut Engine) -> u32 {
        let flee = e.mem.alloc(0x200);
        stub(e, FLEE_PACKAGE_CONSTRUCT, flee);
        flee
    }

    #[test]
    fn fleeing_makes_a_flee_package_with_a_location_reference() {
        let mut e = engine();
        let (actor, _, acquire) = attack_setup(&mut e);
        let flee = flee_package(&mut e);
        stub(&mut e, PACKAGE_TARGET_CONSTRUCT, 0x7400);
        stub(&mut e, PACKAGE_LOCATION_CONSTRUCT, 0x7500);
        stub(&mut e, FTOL, 5);
        stub_float(&mut e, PACKAGE_FLOAT_90, 4.0);
        e.map(ZERO_DOUBLE, 8);
        e.map(MINUS_ONE_DOUBLE, 8);
        e.set_global(MINUS_ONE_DOUBLE, -1.0f64);
        let avoided = Ptr::<()>::new(0x7600);
        let location_ref = Ptr::<()>::new(0x7700);
        e.call(
            0x0089_7de0,
            &args![
                actor,
                avoided,
                0u32,
                3u32,
                1u32,
                0u32,
                location_ref,
                7.5f32,
                12.75f32
            ],
        );
        assert_eq!(
            calls_to(&e, FLEE_PACKAGE_CONSTRUCT)[0][1..],
            [avoided.addr(), 0, 0]
        );
        assert_eq!(
            calls_to(&e, FLEE_PACKAGE_ADD_AVOIDED_REF),
            vec![vec![flee, avoided.addr()]]
        );
        assert_eq!(calls_to(&e, PACKAGE_SET_TARGET), vec![vec![flee, 0x7400]]);
        assert_eq!(
            calls_to(&e, PACKAGE_TARGET_SET_REFERENCE),
            vec![vec![0x7400, avoided.addr()]]
        );
        assert_eq!(calls_to(&e, TARGET_SET_COUNT), vec![vec![0x7400, 5]]);
        // The radius is truncated: 12.75 becomes 12.
        assert_eq!(
            calls_to(&e, PACKAGE_LOCATION_SET_RADIUS),
            vec![vec![0x7500, 12]]
        );
        assert_eq!(
            calls_to(&e, PACKAGE_LOCATION_SET_REFERENCE),
            vec![vec![0x7500, location_ref.addr()]]
        );
        assert_eq!(calls_to(&e, PACKAGE_SET_LOCATION), vec![vec![flee, 0x7500]]);
        assert_eq!(call_count(&e, FLEE_PACKAGE_FIND_TELEPORT_DOOR), 0);
        // The package's own setters ran: +0xa8 = arg_3, +0x81 = arg_4, +0x90
        // minus its own value, and bit 0x2000000 cleared.
        assert_eq!(e.mem.u8(flee + 0xa8), 3);
        assert_eq!(e.mem.u8(flee + 0x81), 1);
        assert_eq!(e.mem.f32(flee + 0x90), -4.0);
        assert_eq!(calls_to(&e, PACKAGE_CALL_67A720), vec![vec![flee, 0]]);
        // arg_2 is 0: the process is told, then the package is added.
        assert_eq!(
            calls_to(&e, slot_target(&e, acquire, PROCESS_SLOT_710)),
            vec![vec![acquire.addr(), actor.addr()]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_PACKAGE)),
            vec![vec![actor.addr(), flee, 0, 1]]
        );
    }

    #[test]
    fn fleeing_reuses_the_current_flee_package_and_may_look_for_a_teleport_door() {
        let mut e = engine();
        let (actor, _, acquire) = attack_setup(&mut e);
        let flee = e.mem.alloc(0x200);
        stub(&mut e, GET_CURRENT_PACKAGE, flee);
        stub(&mut e, PACKAGE_TYPE, 0x16);
        stub(&mut e, PACKAGE_GET_TARGET, 0x7400);
        stub(&mut e, PACKAGE_LOCATION_CONSTRUCT, 0x7500);
        stub(&mut e, PACKAGE_FIELD_A4, 0);
        stub_float(&mut e, PACKAGE_FLOAT_90, 1.0);
        e.map(ZERO_DOUBLE, 8);
        e.map(MINUS_ONE_DOUBLE, 8);
        e.map(TWENTY, 4);
        e.set_global(TWENTY, 20.0f32);
        set_slot(&mut e, acquire, PROCESS_SLOT_E4, float_ret(-1.0));
        let avoided = Ptr::<()>::new(0x7600);
        // Negative counts and radii are not applied.
        e.call(
            0x0089_7de0,
            &args![actor, avoided, 1u32, 0u32, 0u32, 0u32, 0u32, -1.0f32, -1.0f32],
        );
        assert_eq!(call_count(&e, FLEE_PACKAGE_CONSTRUCT), 0);
        assert_eq!(call_count(&e, TARGET_SET_COUNT), 0);
        assert_eq!(call_count(&e, PACKAGE_LOCATION_SET_RADIUS), 0);
        // No reference and no cell: the teleport door search runs, after the
        // process's essential timer (-1.0 <= 0) is set to 20.
        assert_eq!(
            calls_to(&e, slot_target(&e, acquire, PROCESS_SLOT_E8)),
            vec![vec![acquire.addr(), 20.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, FLEE_PACKAGE_FIND_TELEPORT_DOOR),
            vec![vec![flee, actor.addr(), avoided.addr()]]
        );
        // arg_2 is 1: the process is not told.
        assert_eq!(
            call_count(&e, slot_target(&e, acquire, PROCESS_SLOT_710)),
            0
        );

        // A fleeing-excluded actor does nothing.
        clear_log(&mut e);
        stub(&mut e, EXTRA_TYPE_IS_5, 1);
        fn_00897de0(
            &mut e,
            actor,
            avoided,
            1,
            0,
            0,
            Ptr::NULL,
            Ptr::NULL,
            0.0,
            0.0,
        );
        assert_eq!(call_count(&e, FLEE_PACKAGE_ADD_AVOIDED_REF), 0);
    }

    /// An actor and a conversation partner for `Actor::ShouldTalkTo`, with a
    /// process object whose slots give "may talk".
    fn talk_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, Ptr) {
        e.map(ZERO_DOUBLE, 8);
        e.map(NINETY, 8);
        e.set_global(NINETY, 90.0f64);
        let process_object = object(
            e,
            0x40,
            &[(0x30c, ret(0)), (0x22c, ret(0)), (0x1e0, float_ret(0.0))],
        );
        stub(e, GET_SAVED_ACQUIRE_OBJECT, process_object.addr());
        let position = e.mem.alloc(16);
        let actor = actor_with(
            e,
            &[(0x234, ret(0)), (0x214, ret(0)), (0x1f4, ret(position))],
            None,
        );
        let other_position = e.mem.alloc(16);
        let other = object(e, 0x40, &[(0x1f4, ret(other_position))]);
        (actor, other, process_object)
    }

    #[test]
    fn talking_is_refused_in_the_cases_the_code_lists() {
        let mut e = engine();
        let (actor, other, process_object) = talk_setup(&mut e);
        assert!(e.call(0x0089_3190, &args![actor, other]).bool());

        // The process's own refusal (slot 0x30c) when nothing else forbids.
        set_slot(&mut e, process_object, 0x30c, ret(1));
        assert!(!actor_should_talk_to(&mut e, actor, other));
        // ...which a forbidden state (here the object test) overrides.
        stub(&mut e, EXTRA_OBJECTS_TEST, 1);
        assert!(actor_should_talk_to(&mut e, actor, other));
        stub(&mut e, EXTRA_OBJECTS_TEST, 0);
        set_slot(&mut e, process_object, 0x30c, ret(0));

        // A positive timer value (slot 0x1e0) refuses.
        set_slot(&mut e, process_object, 0x1e0, float_ret(2.0));
        assert!(!actor_should_talk_to(&mut e, actor, other));
        set_slot(&mut e, process_object, 0x1e0, float_ret(0.0));

        // The actor state must be 0 or 4.
        set_slot(&mut e, actor.cast(), 0x214, ret(3));
        assert!(!actor_should_talk_to(&mut e, actor, other));
        set_slot(&mut e, actor.cast(), 0x214, ret(4));
        assert!(actor_should_talk_to(&mut e, actor, other));
        set_slot(&mut e, actor.cast(), 0x214, ret(0));

        // An interrupt package refuses.
        stub(&mut e, MOBILE_OBJECT_GET_CURRENT_PACKAGE, 0x8000);
        stub(&mut e, PACKAGE_IS_INTERRUPT_PACKAGE, 1);
        assert!(!actor_should_talk_to(&mut e, actor, other));
        stub(&mut e, PACKAGE_IS_INTERRUPT_PACKAGE, 0);
        assert!(actor_should_talk_to(&mut e, actor, other));

        // The process object's slot 0x22c and skipping the fall-out behaviour.
        set_slot(&mut e, process_object, 0x22c, ret(1));
        stub(&mut e, ACTOR_SHOULD_SKIP_FALLOUT_BEHAVIOR, 1);
        assert!(!actor_should_talk_to(&mut e, actor, other));

        // No process object at all.
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        assert!(!actor_should_talk_to(&mut e, actor, other));
    }

    #[test]
    fn in_an_interior_a_large_height_difference_refuses() {
        let mut e = engine();
        let (actor, other, _) = talk_setup(&mut e);
        stub(&mut e, REFR_PARENT_CELL, 0x9000);
        stub(&mut e, CELL_FLAG_24_BIT_0, 1);
        stub_float(&mut e, FLOAT_ABS, 40.0);
        assert!(actor_should_talk_to(&mut e, actor, other));
        stub_float(&mut e, FLOAT_ABS, 95.0);
        assert!(!actor_should_talk_to(&mut e, actor, other));
        // Outdoors the height difference does not matter.
        stub(&mut e, CELL_FLAG_24_BIT_0, 0);
        assert!(actor_should_talk_to(&mut e, actor, other));
    }

    /// An actor with a process whose package (slot `0x22c`) is `package`, a
    /// package object with slot `0x13c`, and the globals
    /// `Actor::InitPackageLocations` reads.
    fn package_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, Ptr) {
        e.map(MINUS_ONE, 4);
        e.set_global(MINUS_ONE, -1.0f32);
        e.map(LARGE_FLOAT, 4);
        e.set_global(LARGE_FLOAT, f32::MAX);
        let package = object(e, 0x40, &[(0x13c, ret(0))]);
        let actor = actor_with(
            e,
            &[(0x2a8, ret(0))],
            Some(&[
                (0x28, ret(0)),
                (0x14, ret(0)),
                (0x24, ret(0)),
                (0x22c, ret(package.addr())),
            ]),
        );
        let process = e.get(actor, Actor::pCurrentProcess);
        stub(e, PACKAGE_TYPE, 5);
        stub(e, PACKAGE_GET_LOCATION_CELL, 0xc0de);
        stub(e, PACKAGE_GET_LOCATION_WORLD, 0xdead);
        stub(e, PACKAGE_GET_LOCATION_COORD, 0x7a00);
        (actor, package, process)
    }

    #[test]
    fn package_locations_are_initialized_and_the_actor_moved() {
        let mut e = engine();
        let (actor, package, process) = package_setup(&mut e);
        stub(&mut e, REFR_GET_REF_PERSISTS, 1);
        e.call(0x0089_3340, &args![actor, 0u32]);
        // The process is reset (slot 0x28), and its package slots are called.
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x28)),
            vec![vec![process.addr()]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x14)),
            vec![vec![process.addr(), actor.addr(), 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x24)),
            vec![vec![process.addr(), actor.addr(), 0]]
        );
        // The package is remembered (the global object's flag is 0).
        assert_eq!(e.get(actor, Actor::pInitialPackage), package.addr());
        // The package accepts the actor? No (slot 0x13c false): it moves.
        assert_eq!(
            calls_to(&e, slot_target(&e, package, 0x13c)),
            vec![vec![
                package.addr(),
                actor.addr(),
                0,
                (-1.0f32).to_bits(),
                0
            ]]
        );
        assert_eq!(
            calls_to(&e, MOVE_REF_TO_NEW_SPACE),
            vec![vec![actor.addr(), 0xc0de, 0xdead]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), ACTOR_SLOT_2A8)),
            vec![vec![actor.addr(), 0x7a00]]
        );
        assert_eq!(
            calls_to(&e, REFR_FLOAT_CALL),
            vec![vec![actor.addr(), f32::MAX.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_UNLOCK_LOCK_DOORS_PROCEDURE),
            vec![vec![actor.addr()]]
        );
    }

    #[test]
    fn package_locations_are_skipped_for_flagged_actors_and_simple_packages() {
        let mut e = engine();
        let (actor, package, _) = package_setup(&mut e);
        stub(&mut e, FORM_FLAG_800, 1);
        actor_init_package_locations(&mut e, actor, 0);
        assert_eq!(call_count(&e, ACTOR_UNLOCK_LOCK_DOORS_PROCEDURE), 0);
        stub(&mut e, FORM_FLAG_800, 0);
        // A package of type 2 is left alone.
        stub(&mut e, PACKAGE_TYPE, 2);
        actor_init_package_locations(&mut e, actor, 0);
        assert_eq!(call_count(&e, MOVE_REF_TO_NEW_SPACE), 0);
        assert_eq!(call_count(&e, ACTOR_UNLOCK_LOCK_DOORS_PROCEDURE), 1);
        // With the "use remembered" flag the remembered package is used and the
        // process is not reset.
        stub(&mut e, PACKAGE_TYPE, 5);
        e.set(actor, Actor::pInitialPackage, package.addr());
        clear_log(&mut e);
        actor_init_package_locations(&mut e, actor, 1);
        assert_eq!(
            call_count(
                &e,
                slot_target(&e, e.get(actor, Actor::pCurrentProcess), 0x28)
            ),
            0
        );
        assert_eq!(call_count(&e, MOVE_REF_TO_NEW_SPACE), 1);
        // A package of type 0xd does not move the actor.
        stub(&mut e, PACKAGE_TYPE, 0xd);
        clear_log(&mut e);
        actor_init_package_locations(&mut e, actor, 1);
        assert_eq!(call_count(&e, MOVE_REF_TO_NEW_SPACE), 0);
    }

    /// An actor, a real animation block, and the process slots
    /// `Actor::PickAnimations` reads (nothing queued, nothing sitting, no
    /// weapon), with the stubs that let it run to the end.
    fn pick_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, Ptr) {
        e.map(ANIM_GROUP_TABLE, 0x4000);
        e.map(ZERO_DOUBLE, 8);
        e.map(ONE_DOUBLE, 8);
        e.set_global(ONE_DOUBLE, 1.0f64);
        e.map(WEAPON_TYPE_ANIM_TABLE, 0x100);
        let animation = Ptr::<()>::new(e.mem.alloc(0x200));
        let acquire = object(
            e,
            0x40,
            &[(0x458, ret(0)), (0x27c, ret(0)), (0x1b0, ret(0))],
        );
        stub(e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        let actor = actor_with(
            e,
            &[
                (0x22c, ret(0)),
                (0x230, ret(0)),
                (0x234, ret(0)),
                (0x218, ret(0)),
                (0x390, ret(0)),
                (0x1e4, ret(animation.addr())),
                (0x1e8, ret(0x6e00)),
                (0x4b0, ret(0)),
                (0x3ec, ret(0)),
                (0x428, ret(0)),
            ],
            Some(&[
                (0x148, ret(0)),
                (0x3e4, ret(0xffff_ffff)),
                (0x3e8, ret(0)),
                (0x4bc, ret(0)),
                (0x4d4, ret(0)),
                (0x444, float_ret(1.0)),
                (0x454, ret(0)),
                (0x458, ret(0)),
                (0x580, ret(0)),
                (0x574, ret(0)),
                (0x1cc, ret(0)),
                (0x190, ret(0)),
                (0x45c, ret(0)),
                (0x614, ret(0)),
                (0x368, ret(0)),
            ]),
        );
        let process = e.get(actor, Actor::pCurrentProcess);
        let controller = e.mem.alloc(0x500);
        stub(e, MOBILE_OBJECT_GET_CHAR_CONTROLLER, controller);
        stub(e, ACQUIRE_OBJECT_FIELD_28, 1);
        stub(e, ANIMATION_GET_SEQUENCE, 0x6500);
        stub(e, ANIMATION_ZERO_GLOBAL_TRANSFORM, 0x6600);
        stub(e, ANIMATION_GROUP_LOADED, 1);
        stub(e, ANIM_GROUP_COMPOSE, 0x55);
        stub(e, ANIMATION_PICK_BEST_ANIMATION, 0x55);
        stub(e, ANIM_GROUP_GET_TYPE, 0x55);
        (actor, animation, process)
    }

    #[test]
    fn picking_animations_does_nothing_for_dead_or_frozen_actors() {
        let mut e = engine();
        let (actor, _, _) = pick_setup(&mut e);
        set_slot(&mut e, actor.cast(), 0x22c, ret(1));
        e.call(0x0089_5110, &args![actor, 1.0f32, 1.0f32]);
        assert_eq!(call_count(&e, ANIMATION_GET_SEQUENCE), 0);
        set_slot(&mut e, actor.cast(), 0x22c, ret(0));
        set_slot(&mut e, actor.cast(), 0x230, ret(1));
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        assert_eq!(call_count(&e, ANIMATION_GET_SEQUENCE), 0);
        // The first-person player is refused with a message.
        set_slot(&mut e, actor.cast(), 0x230, ret(0));
        set_player(&mut e, actor);
        stub(&mut e, PLAYER_FIRST_PERSON_CHECK, 0);
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        assert_eq!(calls_to(&e, DEBUG_PRINT), vec![vec![FIRST_PERSON_MESSAGE]]);
        assert_eq!(call_count(&e, ANIMATION_GET_SEQUENCE), 0);
    }

    #[test]
    fn walking_forward_picks_the_walk_group_and_scales_its_speed() {
        let mut e = engine();
        let (actor, animation, _) = pick_setup(&mut e);
        stub(&mut e, ACTOR_ANIM_FLAGS, 0x0101);
        stub_float(&mut e, ACTOR_GET_WALK_SPEED, 150.0);
        stub(&mut e, ANIMATION_MOVEMENT_SPEED, 100);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 3);
        stub(&mut e, ANIMATION_PICK_BEST_ANIMATION, 0x0503);
        set_group(&mut e, 3, 1, 0);
        e.call(0x0089_5110, &args![actor, 2.0f32, 9.0f32]);
        // Flags bit 0 and a direction bit: group 3, selectors 0.
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE)[0], vec![0, 0, 3, 0]);
        // (150 / 100) * 2.0
        assert_eq!(e.mem.f32(animation.addr() + 0x10c), 3.0);
        assert_eq!(
            calls_to(&e, ANIMATION_PLAY_GROUP),
            vec![vec![animation.addr(), 0x0503, 1, 0xffff_ffff, 0xffff_ffff]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, actor.cast(), 0x4b0)),
            vec![vec![actor.addr(), 0x0503, 1]]
        );
    }

    #[test]
    fn turning_in_place_uses_the_second_speed_argument() {
        let mut e = engine();
        let (actor, animation, _) = pick_setup(&mut e);
        stub(&mut e, ACTOR_ANIM_FLAGS, 0x0010);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0xf);
        actor_pick_animations(&mut e, actor, 2.0, 9.0);
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE)[0], vec![0, 0, 0xf, 0]);
        assert_eq!(e.mem.f32(animation.addr() + 0x10c), 9.0);
        // Flag 0x20 asks for group 0x10.
        clear_log(&mut e);
        stub(&mut e, ACTOR_ANIM_FLAGS, 0x0020);
        actor_pick_animations(&mut e, actor, 2.0, 9.0);
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE)[0], vec![0, 0, 0x10, 0]);
    }

    #[test]
    fn a_queued_draw_tells_the_process_and_forces_the_animation() {
        let mut e = engine();
        let (actor, animation, process) = pick_setup(&mut e);
        set_slot(&mut e, process, 0x3e4, ret(0));
        set_slot(&mut e, process, 0x3e8, ret(0x6500));
        stub(&mut e, SEQUENCE_GET_GENERIC_LOCATION, 1);
        stub(&mut e, ANIMATION_SLOT_STATE, 1);
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x458)),
            vec![vec![process.addr(), actor.addr(), 1]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x1cc)),
            vec![vec![
                process.addr(),
                1,
                0x6e00,
                animation.addr(),
                actor.addr()
            ]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_IRON_SIGHTS),
            vec![vec![actor.addr(), 0, 0, 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x580)),
            vec![vec![process.addr(), 1, 0, 0]]
        );
        // Already drawn: a queued draw changes nothing.
        clear_log(&mut e);
        set_slot(&mut e, process, 0x454, ret(1));
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        assert_eq!(call_count(&e, slot_target(&e, process, 0x458)), 0);
    }

    #[test]
    fn the_player_draw_also_forces_the_first_person_pair() {
        let mut e = engine();
        let (actor, animation, process) = pick_setup(&mut e);
        set_player(&mut e, actor);
        stub(&mut e, PLAYER_FIRST_PERSON_CHECK, 1);
        stub(&mut e, PLAYER_GET_ANIMATION, 0x6a00);
        stub(&mut e, PLAYER_GET_BIPED, 0x6b00);
        set_slot(&mut e, process, 0x3e4, ret(0));
        set_slot(&mut e, process, 0x3e8, ret(0x6500));
        set_slot(&mut e, process, 0x190, ret(0));
        stub(&mut e, SEQUENCE_GET_GENERIC_LOCATION, 1);
        stub(&mut e, ANIMATION_SLOT_STATE, 1);
        e.map(actor.addr() + 0xac, 4);
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        let forced = calls_to(&e, slot_target(&e, process, 0x1cc));
        assert_eq!(
            forced,
            vec![
                vec![process.addr(), 1, 0x6e00, animation.addr(), actor.addr()],
                vec![process.addr(), 1, 0x6b00, 0x6a00, actor.addr()]
            ]
        );
        // Queued action 0 (draw) for the player sets shader values 0 and 1.
        assert_eq!(call_count(&e, SET_SHADER_VALUE), 2);
    }

    #[test]
    fn a_queued_landing_state_clears_the_fall_action() {
        let mut e = engine();
        let (actor, _, process) = pick_setup(&mut e);
        set_slot(&mut e, process, 0x3e4, ret(0xc));
        set_slot(&mut e, process, 0x3e8, ret(0x6500));
        stub(&mut e, SEQUENCE_GET_GENERIC_LOCATION, 1);
        stub(&mut e, CONTROLLER_STATE, 1);
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION)[0],
            vec![actor.addr(), 0xffff_ffff, 0]
        );
    }

    #[test]
    fn landing_from_a_jump_picks_a_landing_group_and_plays_the_sound() {
        let mut e = engine();
        let (actor, animation, _) = pick_setup(&mut e);
        stub(&mut e, ACTOR_ANIM_FLAGS, 1);
        stub(&mut e, CONTROLLER_STATE, 0);
        stub(&mut e, TRANSFORM_IS_LOOPING, 1);
        stub(&mut e, LANDING_SOUND_LOOKUP, 0x7b00);
        stub(&mut e, ANIM_GROUP_GET_TYPE, 0xf1);
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        assert_eq!(
            calls_to(&e, ANIMATION_LAND_RESET),
            vec![vec![animation.addr()]]
        );
        assert_eq!(
            calls_to(&e, IMPACT_MIXER_PLAY_JUMP_LAND),
            vec![vec![actor.addr(), 0x7b00]]
        );
        // The landing group follows the direction flag: 0xf1 for bit 0.
        assert_eq!(calls_to(&e, ANIM_GROUP_COMPOSE)[0][2], 0xf1);
        // The action is set to 0xc for the group's sequence.
        assert_eq!(
            calls_to(&e, ACTOR_SET_ANIM_ACTION)
                .iter()
                .filter(|a| a[1] == 0xc)
                .count(),
            1
        );
    }

    #[test]
    fn the_weapon_position_is_saved_while_an_attack_plays() {
        let mut e = engine();
        let (actor, animation, process) = pick_setup(&mut e);
        let position = e.mem.alloc(12);
        e.mem.set_f32(position, 1.0);
        e.mem.set_f32(position + 4, 2.0);
        e.mem.set_f32(position + 8, 3.0);
        stub(&mut e, ACTOR_GET_WEAPON_POSITION, position);
        stub(&mut e, ANIM_GROUP_IS_ATTACK_ACTION, 1);
        actor_pick_animations(&mut e, actor, 1.0, 1.0);
        assert_eq!(calls_to(&e, ACTOR_GET_WEAPON_POSITION)[0][0], actor.addr());
        assert_eq!(
            calls_to(&e, slot_target(&e, process, 0x45c)),
            vec![vec![
                process.addr(),
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits()
            ]]
        );
        let _ = animation;
    }
}
