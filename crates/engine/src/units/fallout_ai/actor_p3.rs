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
        /// `pMyKiller` (Xbox PDB, +0xd0).
        0xC0 pMyKiller: Ptr,
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
/// Process virtual slot `0x214`, as the actor slot above.
const PROCESS_SLOT_214: u32 = 0x214;
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

/// `MobileObject::GetCurrentProcessType` (Xbox PDB): the process level of the
/// object (a signed number; `0` is the high process).
const MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE: u32 = 0x0093_1850;
/// `TaskQueueInterface::QueueActorKill` (Xbox PDB): thiscall on the object
/// `GET_TES` returns, taking the actor, the kill data and a flag.
const QUEUE_ACTOR_KILL: u32 = 0x0087_aff0;
/// `FollowerBarks::TriggerFollowerBark` (Xbox PDB), cdecl: the actor and the
/// bark number.
const TRIGGER_FOLLOWER_BARK: u32 = 0x008d_5cb0;
/// `CombatController::IsActoraCombatTarget` (Xbox PDB): thiscall on the
/// combat controller, taking the actor.
const COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET: u32 = 0x0097_fa10;
/// `Actor::GetFactionFightReaction` (Xbox PDB): the other actor and a pointer
/// to a byte the function writes.
const ACTOR_GET_FACTION_FIGHT_REACTION: u32 = 0x008b_87a0;
/// `Actor::Kill` (Xbox PDB): the attacker and the damage (`float`).
const ACTOR_KILL: u32 = 0x0089_d900;
/// `Actor::GetArmorBeingWorn` (Xbox PDB): thiscall taking a slot number.
const ACTOR_GET_ARMOR_BEING_WORN: u32 = 0x0089_1b90;
/// Test that returns whether the debug messages are enabled (`005b6f70`,
/// takes the actor).
const DEBUG_MESSAGES_ENABLED: u32 = 0x005b_6f70;
/// The cdecl `printf`-style function that prints a debug line (`00703c00`).
const PRINT_DEBUG_LINE: u32 = 0x0070_3c00;
/// `"%.20s blocks %.0f%% of %.20s's blow!"`.
const BLOCKS_BLOW_FORMAT: u32 = 0x0108_4b54;
/// `100.0` (`double`).
const HUNDRED: u32 = 0x0101_7a40;
/// `0.25` (`double`).
const QUARTER: u32 = 0x0102_90b0;
/// `0.5` (`double`).
const HALF: u32 = 0x0101_1588;
/// `Actor` virtual slot `0x360`: a bool predicate (`0047c850` returns 0 in
/// the base class).
const ACTOR_SLOT_360: u32 = 0x360;
/// `Actor` virtual slot `0x380`: the actor's reach as a `float` (`0088b850`
/// is `CombatFormulas::CalcWeaponReach` of a setting).
const ACTOR_SLOT_REACH: u32 = 0x380;
/// `Actor` virtual slot `0x3ac`: changes an actor value (`00881130`: the
/// value number, the amount, the source actor).
const ACTOR_SLOT_MODIFY_ACTOR_VALUE: u32 = 0x3ac;
/// `Actor` virtual slot `0x3cc` (`008929a0`): drops an item: form, extra
/// data list, count, position, rotation; returns the new reference.
const ACTOR_SLOT_DROP_OBJECT: u32 = 0x3cc;
/// Process virtual slot `0x534`: the item the actor has equipped in the
/// other hand (compared with the weapon entry's form).
const PROCESS_SLOT_534: u32 = 0x534;
/// `CombatFormulas::CalcWeaponReach` (Xbox PDB), cdecl: the weapon's reach
/// as a `float`; the result is in ST0.
const CALC_WEAPON_REACH: u32 = 0x0064_63e0;
/// Reach of a weapon form (`006447f0`, thiscall on the form; ST0).
const WEAPON_REACH: u32 = 0x0064_47f0;
/// `TESObjectREFR::GetScale` (Xbox PDB): ST0.
const REFR_GET_SCALE: u32 = 0x0056_7400;
/// `009a60e0`: cdecl, takes the actor and a `float` distance.
const ACTOR_REACH_WORKER: u32 = 0x009a_60e0;
/// The setting object the reach is multiplied by for a special idle of kind 4
/// (the address `SETTING_GET_VALUE_POINTER` is called on).
const REACH_MULTIPLIER_SETTING: u32 = 0x011c_e194;
/// `ProcessLists::PrintLists` (Xbox PDB, the engine map's name): thiscall on
/// the combat controller of the attacker with the victim and the amount.
const COMBAT_CONTROLLER_NOTIFY_DAMAGE: u32 = 0x008d_0600;
/// `0097f7f0`: thiscall on the combat controller `008a0330` returns, taking
/// the attacker and the amount.
const CONTROLLER_RECORD_DAMAGE: u32 = 0x0097_f7f0;
/// `0097f6d0`: thiscall on the combat controller, taking one flag.
const CONTROLLER_SET_FLAG: u32 = 0x0097_f6d0;
/// `Actor::GetEssential` (Xbox PDB).
const ACTOR_GET_ESSENTIAL: u32 = 0x0087_f3d0;
/// `Actor::DifficultyLevelAdjustHealthModifier` (Xbox PDB): the amount and
/// the source actor; the result is in ST0.
const DIFFICULTY_ADJUST_HEALTH_MODIFIER: u32 = 0x0088_08a0;
/// `00579220`: thiscall on the reference, takes a `float` and a flag.
const REFR_FLOAT_SETTER_579220: u32 = 0x0057_9220;
/// The setting object whose value the fatigue is compared with.
const FATIGUE_SETTING: u32 = 0x011c_e118;
/// Process virtual slot `0x72c`: reacts to damage (attacker, amount).
const PROCESS_SLOT_72C: u32 = 0x72c;
/// Process virtual slot `0x580` (shader effects: 1, 0, 0).
const PROCESS_SLOT_580: u32 = 0x580;
/// `ExtraDataList::GetCanNotWear` (Xbox PDB).
const EXTRA_GET_CAN_NOT_WEAR: u32 = 0x0041_8b10;
/// Whether the form (`this` = form) has bit `0x8` of the byte at `+0x100`
/// (`00891b70`).
const FORM_FLAG_100_BIT_3: u32 = 0x0089_1b70;
/// Whether the form has bit `0x20` of the byte at `+0x100` (`0046e8c0`).
const FORM_FLAG_100_BIT_5: u32 = 0x0046_e8c0;
/// The position of a bone node: the address of its 3 floats (`0045bb80`,
/// `node + 0x8c`).
const NODE_WORLD_TRANSLATE: u32 = 0x0045_bb80;
/// `node + 0x68` (`00461130`): the rotation matrix of a node.
const NODE_ROTATION: u32 = 0x0046_1130;
/// `NiMatrix3::ToEulerAnglesXYZ` (Xbox PDB): thiscall on the matrix with
/// three pointers for the angles.
const NI_MATRIX3_TO_EULER_ANGLES_XYZ: u32 = 0x00a5_92c0;
/// `0056acb0` (cdecl): the dropper actor and the dropped reference.
const ACTOR_DROPPED_OBJECT_WORKER: u32 = 0x0056_acb0;
/// Form flag test (`00461580`): thiscall on the form with a mask; true when
/// `*(this + 4) & mask` is not zero.
const FORM_TEST_FLAGS: u32 = 0x0046_1580;

/// The actor's value owner embedded at `+0xa4` (virtual slot `4`: base value,
/// slot `0xc`: current value, both taking the actor value number).
const ACTOR_VALUE_OWNER_OFFSET: u32 = 0xa4;

/// Reads a `float` constant stored as a `double` at `address`.
fn double_constant(e: &Engine, address: u32) -> f64 {
    e.global::<f64>(address)
}

// Translated from 008985d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Ends the actor's movement, creates a package of type `0x24` and adds it
/// through the actor's add-package slot (`0x2f4`): with `1, 1` when the
/// actor's process type is at most 1, otherwise after setting the package's
/// second flag, with `0, 0`.
pub fn fn_008985d0(e: &mut Engine, this: Ptr<Actor>) {
    e.call(ACTOR_END_MOVEMENT, &args![this]);
    let package = e.call(PACKAGE_CREATE, &args![0x24u32]).ptr::<()>();
    e.call(PACKAGE_CALCULATE_PROCEDURE_TYPE, &args![package, 0u32]);
    let process_type = e
        .call(MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, &args![this])
        .i32();
    if process_type <= 1 {
        e.vcall(
            this.addr(),
            ACTOR_SLOT_ADD_PACKAGE,
            &args![package, 1u32, 1u32],
        );
    } else {
        e.call(PACKAGE_SET_FLAG_B, &args![package, 1u32]);
        e.vcall(
            this.addr(),
            ACTOR_SLOT_ADD_PACKAGE,
            &args![package, 0u32, 0u32],
        );
    }
}

// Translated from 00898650 (decompiled, FalloutNV.exe 1.4.0.525)
/// Applies a health loss to an actor that is not an essential-style
/// protected one (`this`'s slot `0x22c(0)` false): works out the health
/// ratio (current over base of actor value `0x10`), lets a follower
/// (teammate flag at `+0x18d`) bark when it is under a half (bark 4) or a
/// quarter (bark 5), turns the attacker into nobody when the player attacks
/// a creature that the player's faction reaction does not allow it to fight,
/// and, when the health is below one, kills the actor: directly
/// (`Actor::Kill`) or, when the task queue is used (`008c7aa0`), by queueing
/// the kill with the damage `float` in a 4-byte block and remembering the
/// attacker as the killer (`+0xc0`).
pub fn fn_00898650(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>, damage: f32) {
    let mut attacker = attacker;
    if e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
        return;
    }
    let owner = this.addr() + ACTOR_VALUE_OWNER_OFFSET;
    let current = e.vcall(owner, 0x0c, &args![0x10u32]).f32();
    let base = e.vcall(owner, 0x04, &args![0x10u32]).f32();
    let mut ratio = 1.0f32;
    if base != 0.0 {
        ratio = (current as f64 / base as f64) as f32;
    }
    if e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![this]).bool() {
        let quarter = double_constant(e, QUARTER);
        let half = double_constant(e, HALF);
        if (ratio as f64) < quarter {
            e.call(TRIGGER_FOLLOWER_BARK, &args![this, 5u32]);
        } else if (ratio as f64) < half {
            e.call(TRIGGER_FOLLOWER_BARK, &args![this, 4u32]);
        }
    }
    let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
    let player_address = e.global::<u32>(PLAYER_CHARACTER);
    if attacker.addr() == player_address
        && controller != 0
        && !e
            .call(
                COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET,
                &args![controller, player_address],
            )
            .bool()
    {
        let reaction = e.with_stack(4, |e, out| {
            e.mem.set_u8(out.addr(), 0);
            e.call(
                ACTOR_GET_FACTION_FIGHT_REACTION,
                &args![this, player_address, out],
            )
            .i32()
        });
        if reaction == 2 {
            attacker = Ptr::NULL;
        }
    }
    let one = double_constant(e, ONE_DOUBLE);
    if (current as f64) < one {
        if !e.call(PICK_UP_GOES_TO_TASK_QUEUE, &args![]).bool() {
            e.call(ACTOR_KILL, &args![this, attacker, damage]);
        } else {
            let block = e.call(OPERATOR_NEW, &args![4u32]).u32();
            e.mem.set_f32(block, damage);
            e.set(this, Actor::pMyKiller, attacker.cast());
            let tes = e.call(GET_TES, &args![]).u32();
            e.call(QUEUE_ACTOR_KILL, &args![tes, this, block, 0u32]);
        }
    }
}

// Translated from 00898fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x10` of the dword at `+0x12c` is set.
pub fn fn_00898fd0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x12c) & 0x10 != 0
}

// Translated from 00898ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls virtual slot `0x18` of the object embedded at `+0x30` of the
/// actor's base form (`004181e0`) and returns its flag.
pub fn fn_00898ff0(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
    e.vcall(base + 0x30, 0x18, &args![]).bool()
}

// Translated from 00899030 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to virtual slot `0x4f0` of the object
/// `MiddleHighProcess::GetSavedAcquireObject` returns for the actor, and
/// returns its result.
pub fn fn_00899030(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    e.vcall(object, 0x4f0, &args![]).u32()
}

// Translated from 00899060 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the actor has an acquire object (`008d8520`), calls its virtual
/// slot `0x4f4` with `flag`.
pub fn fn_00899060(e: &mut Engine, this: Ptr<Actor>, flag: u8) {
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(object, 0x4f4, &args![flag]);
    }
}

// Translated from 008990a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::HandleBlockedAttack` (Xbox PDB): when debug messages are on,
/// prints `"<other> blocks <percent>% of <this>'s blow!"`; `percent` is
/// `blocked * 100`. Two of the four stack words are never read.
pub fn actor_handle_blocked_attack(
    e: &mut Engine,
    this: Ptr<Actor>,
    _unused_0: u32,
    blocked: f32,
    other: Ptr<Actor>,
    _unused_3: u32,
) {
    if e.call(DEBUG_MESSAGES_ENABLED, &args![]).bool() {
        let this_name = e.call(REFR_GET_NAME, &args![this]).u32();
        let percent = blocked as f64 * double_constant(e, HUNDRED);
        let other_name = e.call(REFR_GET_NAME, &args![other]).u32();
        e.call(
            PRINT_DEBUG_LINE,
            &args![BLOCKS_BLOW_FORMAT, other_name, percent, this_name],
        );
    }
}

// Translated from 008990f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Looks for the reference the actor can hit: works out the actor's reach (the
/// reach of the weapon it holds, `CalcWeaponReach` of the weapon form's reach,
/// or with no weapon its own, slot `0x380`; times the actor's scale; times
/// the reach setting when slot `0x360` is true and the special idle state
/// object's form is of kind 4) and returns what `009a60e0(actor, reach)`
/// finds.
pub fn fn_008990f0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mut weapon = 0;
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        weapon = e.vcall(object, PROCESS_SLOT_CURRENT_WEAPON, &args![]).u32();
    }
    let mut weapon_form = 0;
    if weapon != 0 {
        weapon_form = e.call(ENTRY_FORM, &args![weapon]).u32();
    }
    let reach = if weapon_form != 0 {
        let raw = e.call(WEAPON_REACH, &args![weapon_form]).f32();
        e.call(CALC_WEAPON_REACH, &args![raw]).f32()
    } else {
        e.vcall(this.addr(), ACTOR_SLOT_REACH, &args![]).f32()
    };
    let scale = e.call(REFR_GET_SCALE, &args![this]).f32();
    let mut result = (scale as f64 * reach as f64) as f32;
    if e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
        && e.call(ENTRY_FORM, &args![SPECIAL_IDLE_STATE_OBJECT]).u32() == 4
    {
        let setting = e
            .call(SETTING_GET_VALUE_POINTER, &args![REACH_MULTIPLIER_SETTING])
            .u32();
        let multiplier = e.mem.f32(setting);
        result = (result as f64 * multiplier as f64) as f32;
    }
    e.call(ACTOR_REACH_WORKER, &args![this, result]).u32()
}

// Translated from 0089d5e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag bit `0x8000000` is set (the flag test `00461580`:
/// `*(this + 4) & mask != 0`).
pub fn fn_0089d5e0(e: &mut Engine, this: Ptr) -> bool {
    e.call(FORM_TEST_FLAGS, &args![this, 0x0800_0000u32]).bool()
}

// Translated from 0089d600 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of the embedded object at `+0x450`.
pub fn fn_0089d600(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x450)
}

// Translated from 0089d620 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at `+0x2e4`.
pub fn fn_0089d620(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2e4)
}

// Translated from 0089d640 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores three dwords at `+0xf4`, `+0xf8` and `+0xfc`.
pub fn fn_0089d640(e: &mut Engine, this: Ptr, first: u32, second: u32, third: u32) {
    e.mem.set_u32(this.addr() + 0xf4, first);
    e.mem.set_u32(this.addr() + 0xf8, second);
    e.mem.set_u32(this.addr() + 0xfc, third);
}

// Translated from 0089d670 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0x28d`.
pub fn fn_0089d670(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x28d)
}

// Translated from 0089d690 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0x28e`.
pub fn fn_0089d690(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x28e)
}

// Translated from 0089d6b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at `+0x29c`.
pub fn fn_0089d6b0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x29c)
}

// Translated from 0089d6d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at `+0x298`.
pub fn fn_0089d6d0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x298)
}

// Translated from 0089d6f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Applies damage to an actor that is not protected (the object embedded at
/// `+0x94`, slot `0x10`, is false). `health_loss` goes through
/// `DifficultyLevelAdjustHealthModifier`; when the adjusted loss is positive
/// the attacker's combat controller (slot `0x428`) and this actor's process
/// object are told, a non-player actor's acquire object reacts (slot
/// `0x72c`) unless it is essential, actor value `0x10` (health) is lowered
/// by the loss through slot `0x3ac` and `00579220` runs. A positive
/// `fatigue_loss` lowers actor value `0x16` the same way when the fatigue
/// has reached the fatigue setting. Returns slot `0x22c(0)`'s result, or the
/// protected test's result with its low byte cleared.
pub fn fn_0089d6f0(
    e: &mut Engine,
    this: Ptr<Actor>,
    health_loss: f32,
    fatigue_loss: f32,
    attacker: Ptr<Actor>,
) -> u32 {
    let protected = e.vcall(this.addr() + 0x94, 0x10, &args![]).u32();
    if protected as u8 != 0 {
        return protected & 0xffff_ff00;
    }
    let adjusted = e
        .call(
            DIFFICULTY_ADJUST_HEALTH_MODIFIER,
            &args![this, -health_loss, attacker],
        )
        .f32();
    let loss = -adjusted;
    if loss as f64 > double_constant(e, ZERO_DOUBLE) {
        if !attacker.is_null() {
            let controller = e.vcall(attacker.addr(), ACTOR_SLOT_428, &args![]).u32();
            if controller != 0 {
                let controller = e.vcall(attacker.addr(), ACTOR_SLOT_428, &args![]).u32();
                e.call(
                    COMBAT_CONTROLLER_NOTIFY_DAMAGE,
                    &args![controller, this, loss],
                );
            }
        }
        if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() != 0 {
            let controller = fn_008a0330(e, this);
            e.call(CONTROLLER_RECORD_DAMAGE, &args![controller, attacker, loss]);
        }
        if !e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
            && e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0
            && !e.call(ACTOR_GET_ESSENTIAL, &args![this]).bool()
        {
            let object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            e.vcall(object, PROCESS_SLOT_72C, &args![attacker, loss]);
        }
        e.vcall(
            this.addr(),
            ACTOR_SLOT_MODIFY_ACTOR_VALUE,
            &args![0x10u32, -loss, attacker],
        );
        e.call(REFR_FLOAT_SETTER_579220, &args![this, health_loss, 0u32]);
    }
    if fatigue_loss as f64 > double_constant(e, ZERO_DOUBLE) {
        let fatigue = actor_get_fatigue(e, this) as f64;
        let setting = e
            .call(SETTING_GET_VALUE_POINTER, &args![FATIGUE_SETTING])
            .u32();
        let threshold = e.mem.f32(setting) as f64;
        if threshold <= fatigue {
            e.vcall(
                this.addr(),
                ACTOR_SLOT_MODIFY_ACTOR_VALUE,
                &args![0x16u32, -fatigue_loss, attacker],
            );
        }
    }
    e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).u32()
}

// Translated from 0089d8b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetRandomWornArmor` (Xbox PDB): with `flag` set, the armor worn in
/// slot 0 or, when there is none, in slot 1; otherwise the armor worn in
/// slot 2.
pub fn actor_get_random_worn_armor(e: &mut Engine, this: Ptr<Actor>, flag: u8) -> u32 {
    if flag == 0 {
        e.call(ACTOR_GET_ARMOR_BEING_WORN, &args![this, 2u32]).u32()
    } else {
        let armor = e.call(ACTOR_GET_ARMOR_BEING_WORN, &args![this, 0u32]).u32();
        if armor == 0 {
            e.call(ACTOR_GET_ARMOR_BEING_WORN, &args![this, 1u32]).u32()
        } else {
            armor
        }
    }
}

// Translated from 0089f4e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at `+0x638`.
pub fn fn_0089f4e0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x638)
}

// Translated from 0089f500 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0xe2d`.
pub fn fn_0089f500(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0xe2d)
}

// Translated from 0089f520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte `value` at `+0x18a`.
pub fn fn_0089f520(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x18a, value);
}

// Translated from 0089f540 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0x18a`.
pub fn fn_0089f540(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x18a)
}

// Translated from 0089f560 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds 1 to the dword at `+0x3c`.
pub fn fn_0089f560(e: &mut Engine, this: Ptr) {
    let count = e.mem.u32(this.addr() + 0x3c);
    e.mem.set_u32(this.addr() + 0x3c, count.wrapping_add(1));
}

// Translated from 0089f580 (decompiled, FalloutNV.exe 1.4.0.525)
/// Drops the weapon the actor has drawn when it is not the item equipped in
/// the other hand (process slot `0x534`), is not flagged (form bytes at
/// `+0x100`: bits `0x8`, `0x20`) and cannot be worn (its extra data list says
/// `GetCanNotWear` false): the drop (`Actor` slot `0x3cc`) happens at the
/// weapon bone's position and angles (process slot `0x190` of the
/// `Actor` slot `0x1e8` result) when the node exists, then the dropped
/// reference is passed to `0056acb0` and, for an enchanted form, process
/// slot `0x580` runs with `1, 0, 0`. Ends by telling the actor's
/// controller of its fight (`0097f6d0(0)`) when it has one (slot `0x428`).
pub fn fn_0089f580(e: &mut Engine, this: Ptr<Actor>) {
    let object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    let weapon = e.vcall(object, PROCESS_SLOT_CURRENT_WEAPON, &args![]).u32();
    let object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    let other_hand = e.vcall(object, PROCESS_SLOT_534, &args![]).u32();
    if weapon != 0 {
        let form = e.call(ENTRY_FORM, &args![weapon]).u32();
        if e.call(FORM_FLAG_100_BIT_3, &args![form]).bool() {
            return;
        }
        let form = e.call(ENTRY_FORM, &args![weapon]).u32();
        if e.call(FORM_FLAG_100_BIT_5, &args![form]).bool() {
            return;
        }
        if !e.call(ACTOR_IS_WEAPON_DRAWN, &args![this]).bool() {
            return;
        }
        let lists = e.call(ENTRY_FIRST_EXTRA_LIST, &args![weapon]).u32();
        let lists = e.call(LIST_ITEM_SLOT, &args![lists]).u32();
        let first = e.mem.u32(lists);
        if e.call(EXTRA_GET_CAN_NOT_WEAR, &args![first]).bool() {
            return;
        }
        if e.call(ENTRY_FORM, &args![weapon]).u32() == other_hand {
            return;
        }
        let mut enchanted = false;
        let form = e.call(ENTRY_FORM, &args![weapon]).u32();
        if e.call(GET_FORM_ENCHANTING, &args![form]).u32() != 0 {
            enchanted = true;
        }
        let bone_slot = e.vcall(this.addr(), ACTOR_SLOT_1E8, &args![]).u32();
        let process = e.get(this, Actor::pCurrentProcess).addr();
        let node = e
            .vcall(process, PROCESS_SLOT_WEAPON_BONE, &args![bone_slot])
            .u32();
        if node != 0 {
            let translate = e.call(NODE_WORLD_TRANSLATE, &args![node]).u32();
            let position = e.mem.alloc(12);
            for word in 0..3 {
                let value = e.mem.u32(translate + 4 * word);
                e.mem.set_u32(position + 4 * word, value);
            }
            let angles = e.mem.alloc(12);
            let rotation = e.call(NODE_ROTATION, &args![node]).u32();
            e.call(
                NI_MATRIX3_TO_EULER_ANGLES_XYZ,
                &args![rotation, angles, angles + 4, angles + 8],
            );
            let lists = e.call(ENTRY_FIRST_EXTRA_LIST, &args![weapon]).u32();
            let first = e.mem.u32(lists);
            let form = e.call(ENTRY_FORM, &args![weapon]).u32();
            let dropped = e
                .vcall(
                    this.addr(),
                    ACTOR_SLOT_DROP_OBJECT,
                    &args![form, first, 1u32, position, angles],
                )
                .u32();
            e.call(ACTOR_DROPPED_OBJECT_WORKER, &args![this, dropped]);
            e.mem.free(position);
            e.mem.free(angles);
            if enchanted {
                let process = e.get(this, Actor::pCurrentProcess).addr();
                e.vcall(process, PROCESS_SLOT_580, &args![1u32, 0u32, 0u32]);
            }
        }
        if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() != 0 {
            let controller = fn_008a0330(e, this);
            e.call(CONTROLLER_SET_FLAG, &args![controller, 0u32]);
        }
    }
}

// Translated from 008a0250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte at `+0xc4` to 1 (`bMurderAlarm` of an `Actor`, Xbox PDB; the
/// combat starter also calls it on a combat controller).
pub fn fn_008a0250(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 0xc4, 1);
}

// Translated from 008a0330 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls virtual slot `0x22c` of the actor's process (`pCurrentProcess`) and
/// returns its result: the actor's combat controller as the process holds it
/// (`Actor::GetCombatController`, `008a02d0`, checks its type and the combat
/// flag first).
pub fn fn_008a0330(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let process = e.get(this, Actor::pCurrentProcess).addr();
    e.vcall(process, PROCESS_SLOT_22C, &args![]).u32()
}

/// The `CombatManager` singleton pointer.
const COMBAT_MANAGER: u32 = 0x011f_1958;
/// The `TES` singleton pointer (`GET_TES` returns the same object).
const TES_SINGLETON: u32 = 0x011d_ea10;
/// The `ModelLoader` singleton pointer.
const MODEL_LOADER: u32 = 0x011c_3b3c;
/// The `FaderManager` singleton pointer.
const FADER_MANAGER: u32 = 0x011d_8804;
/// The global object `Actor::Resurrect` locks around its work (`0040fbf0`
/// locks it, `0040fba0` unlocks it).
const RESURRECT_LOCK: u32 = 0x011f_11a0;
/// The global object whose do-nothing method `00483710` the combat start
/// calls around the combat controller's start (`0x11df71c`).
const COMBAT_START_GUARD: u32 = 0x011d_f71c;
/// The pointer read as the `this` of `0047c850` (`TESActorBaseData`'s
/// do-nothing predicate that returns 0).
const BASE_DATA_OBJECT: u32 = 0x011d_e45c;
/// `0047c850`: predicate on `BASE_DATA_OBJECT` (returns 0 in the exe).
const BASE_DATA_PREDICATE: u32 = 0x0047_c850;
/// `PathManager::QInstance` (Xbox PDB): the singleton, no arguments.
const PATH_MANAGER_Q_INSTANCE: u32 = 0x0047_d0b0;
/// `PathManager::ClearInventoryPointersFromCurrentRequests` (Xbox PDB):
/// thiscall on the manager, taking the inventory changes.
const PATH_MANAGER_CLEAR_INVENTORY_POINTERS: u32 = 0x006e_bd10;
/// `00574920`: thiscall on the actor, taking a flag.
const ACTOR_UPDATE_INVENTORY_WORKER: u32 = 0x0057_4920;
/// `TESNPC::InitDefaultWorn`-style worker (`006047c0`, the 5-word form of
/// the creature one): the actor, `1`, a flag, `0`, `1`.
const NPC_INIT_DEFAULT_WORN: u32 = 0x0060_47c0;
/// `TESCreature::InitDefaultWorn` (Xbox PDB): the actor, `1`, a flag, `1`.
const CREATURE_INIT_DEFAULT_WORN: u32 = 0x005f_9e00;
/// Whether a package (`this`) is one of the kinds `00441b00` accepts.
const PACKAGE_KIND_TEST: u32 = 0x0044_1b00;
/// `Actor` virtual slot `0x1cc`.
const ACTOR_SLOT_1CC: u32 = 0x1cc;
/// Process virtual slot `0x468`.
const PROCESS_SLOT_468: u32 = 0x468;
/// `Actor` virtual slot `0x418`.
const ACTOR_SLOT_418: u32 = 0x418;
/// `Actor` virtual slot `0x434`: `Actor::StopCombat` (Xbox PDB), takes the
/// target.
const ACTOR_SLOT_STOP_COMBAT: u32 = 0x434;
/// Process virtual slot `0x4d4` (a furniture marker entry, whose byte at
/// `+0xe` is compared with `0x14`).
const PROCESS_SLOT_4D4: u32 = 0x4d4;
/// Process virtual slot `0x52c`.
const PROCESS_SLOT_52C: u32 = 0x52c;
/// Process virtual slot `0x208` (takes the target and the reference
/// `0x52c` gave; the result is a `float`).
const PROCESS_SLOT_208: u32 = 0x208;
/// Process virtual slot `0x2c8` (takes a `float`).
const PROCESS_SLOT_2C8: u32 = 0x2c8;
/// `TESObjectREFR::GetDistanceFromReference` (Xbox PDB): the other
/// reference and two flags; the distance is in ST0.
const REFR_GET_DISTANCE_FROM_REFERENCE: u32 = 0x0057_23b0;
/// `Actor` `0087f9c0` (a `float` read).
const ACTOR_GETTER_87F9C0: u32 = 0x0087_f9c0;
/// `PlayerCharacter::ChangePerceivedActorHostileStatus` (Xbox PDB): thiscall
/// on the player, taking the actor and a flag.
const PLAYER_CHANGE_PERCEIVED_HOSTILE_STATUS: u32 = 0x0096_7220;
/// `00703350`: returns the reference the crosshair targets (compared with
/// the actor).
const CROSSHAIR_TARGET: u32 = 0x0070_3350;
/// `Interface::SetCrosshairTargetType` (Xbox PDB), cdecl.
const SET_CROSSHAIR_TARGET_TYPE: u32 = 0x0070_3860;
/// `Actor::EndInterruptPackage` (Xbox PDB): takes a flag.
const ACTOR_END_INTERRUPT_PACKAGE: u32 = 0x0088_1680;
/// `CombatManager::AddGroupMember` (Xbox PDB): thiscall on the manager,
/// taking the actor and the group.
const COMBAT_MANAGER_ADD_GROUP_MEMBER: u32 = 0x0099_22b0;
/// `CombatManager::AddCombatant` (Xbox PDB): the actor, the target, the
/// value and a flag.
const COMBAT_MANAGER_ADD_COMBATANT: u32 = 0x0099_2110;
/// `CombatManager::AddPretendCombatant` (Xbox PDB): the actor and the
/// pretend target.
const COMBAT_MANAGER_ADD_PRETEND_COMBATANT: u32 = 0x0099_23d0;
/// `CombatController::AddTarget` (Xbox PDB): the target, the value, a flag
/// and two `float`s.
const COMBAT_CONTROLLER_ADD_TARGET: u32 = 0x0097_f930;
/// The target of a combat controller: `*(controller + 0xc0)` (`004030b0`).
const COMBAT_CONTROLLER_TARGET: u32 = 0x0040_30b0;
/// The value `Interface` calls `Error`: a do-nothing cdecl function
/// (`0040fbe0`) that takes the actor.
const ERROR_NO_OP: u32 = 0x0040_fbe0;
/// `FaderManager::GetFaderAlpha` (Xbox PDB): thiscall on the manager,
/// taking a fader number; the result is in ST0.
const FADER_MANAGER_GET_FADER_ALPHA: u32 = 0x0070_14e0;
/// Test on an actor (`00576d30`).
const ACTOR_TEST_576D30: u32 = 0x0057_6d30;
/// Test on a combat controller (`00981450`).
const CONTROLLER_TEST_981450: u32 = 0x0098_1450;
/// Test on the player (`005a03f0`, takes a flag).
const PLAYER_TEST_5A03F0: u32 = 0x005a_03f0;
/// Process-type test on the actor (`00437bf0`: process level equal to 5).
const ACTOR_PROCESS_LEVEL_IS_5: u32 = 0x0043_7bf0;
/// Process-type test on the actor (`00437bd0`: process level equal to 3).
const ACTOR_PROCESS_LEVEL_IS_3: u32 = 0x0043_7bd0;
/// Test on an actor (`00440da0`).
const ACTOR_TEST_440DA0: u32 = 0x0044_0da0;
/// Test on an actor (`00440d80`).
const ACTOR_TEST_440D80: u32 = 0x0044_0d80;
/// `Actor` virtual slot `0x2e8`.
const ACTOR_SLOT_2E8: u32 = 0x2e8;
/// How many combat entries the player's combat list holds for the actor
/// (`008a8230`: the player and the actor).
const PLAYER_COMBAT_LIST_COUNT: u32 = 0x008a_8230;
/// `PlayerCharacter::AddActorToPlayerCombatList` (Xbox PDB): thiscall on the
/// player, taking the actor.
const PLAYER_ADD_ACTOR_TO_COMBAT_LIST: u32 = 0x0093_a690;
/// Test on the combat controller (`009818b0`).
const CONTROLLER_TEST_9818B0: u32 = 0x0098_18b0;
/// Process virtual slot `0x64c`.
const PROCESS_SLOT_64C: u32 = 0x64c;
/// Process virtual slot `0x630` (takes the target).
const PROCESS_SLOT_630: u32 = 0x630;
/// Combat controller start for the high process level (`0097da50`).
const CONTROLLER_START_HIGH: u32 = 0x0097_da50;
/// Combat controller start for the other process levels (`0097e4b0`).
const CONTROLLER_START_OTHER: u32 = 0x0097_e4b0;
/// `00972aa0`: thiscall on the process lists, taking the actor.
const PROCESS_LISTS_WORKER_972AA0: u32 = 0x0097_2aa0;
/// `ExtraDataList::RemoveSavedHavokData` (Xbox PDB).
const EXTRA_REMOVE_SAVED_HAVOK_DATA: u32 = 0x0042_2c20;
/// `ExtraDataList::RemoveDismembermentExtra` (Xbox PDB).
const EXTRA_REMOVE_DISMEMBERMENT_EXTRA: u32 = 0x0042_e8e0;
/// `ExtraDataList::RemovePlayerCrimeListExtra` (Xbox PDB): takes the actor.
const EXTRA_REMOVE_PLAYER_CRIME_LIST_EXTRA: u32 = 0x0041_cf30;
/// `MiddleHighProcess::GetLastIdlePlayed` (Xbox PDB).
const PROCESS_GET_LAST_IDLE_PLAYED: u32 = 0x005a_29b0;
/// `Actor::SetLifeState` (Xbox PDB): takes the state number.
const ACTOR_SET_LIFE_STATE: u32 = 0x008a_1800;
/// `00884f80`: thiscall on the actor.
const ACTOR_WORKER_884F80: u32 = 0x0088_4f80;
/// `NiPointer` constructor (`00633c90`): stores the pointer and adds a
/// reference when it is not null.
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
/// `bhkWorld::SetMotion` (Xbox PDB), cdecl: the world object (from `Actor`
/// slot `0x1d0`) and four flags.
const BHK_WORLD_SET_MOTION: u32 = 0x00c6_a350;
/// `Actor` virtual slot `0x1d0` (no arguments).
const ACTOR_SLOT_1D0: u32 = 0x1d0;
/// `Actor` virtual slot `0xc4` (`MobileObject::SetDelete`, Xbox PDB).
const ACTOR_SLOT_SET_DELETE: u32 = 0xc4;
/// `Actor` virtual slot `0x4c` (takes a flag mask).
const ACTOR_SLOT_4C: u32 = 0x4c;
/// `Actor` virtual slot `0x1c4` (`MobileObject::InitHavok`, Xbox PDB).
const ACTOR_SLOT_INIT_HAVOK: u32 = 0x1c4;
/// Process virtual slot `0x290`.
const PROCESS_SLOT_290: u32 = 0x290;
/// Process virtual slot `0x410`.
const PROCESS_SLOT_410: u32 = 0x410;
/// `ModifierList::DeleteAllModifiers` (Xbox PDB).
const MODIFIER_LIST_DELETE_ALL: u32 = 0x0093_70b0;
/// `ProcessLists::RemoveReference` (Xbox PDB): thiscall on the process
/// lists, taking the actor and the process level.
const PROCESS_LISTS_REMOVE_REFERENCE: u32 = 0x0096_d470;
/// `ProcessLists::AddReference` (Xbox PDB): the actor, the level and three
/// zeros.
const PROCESS_LISTS_ADD_REFERENCE: u32 = 0x0096_d450;
/// Constructor of the 0xb4-byte process object `Resurrect` installs
/// (`00906dc0`).
const NEW_PROCESS_CONSTRUCT: u32 = 0x0090_6dc0;
/// `Actor` virtual slot `0x208` (`fn_0089fb80`).
const ACTOR_SLOT_208: u32 = 0x208;
/// `Actor` virtual slots `0x240`, `0x248` and `0x24c`: the process-level
/// switches (to the high, middle high and middle low level; the names are
/// those the code's `GetDesiredProcessLevel` switch uses, not confirmed).
const ACTOR_SLOT_240: u32 = 0x240;
const ACTOR_SLOT_248: u32 = 0x248;
const ACTOR_SLOT_24C: u32 = 0x24c;
/// `TES::IsCellLoaded` (Xbox PDB): thiscall on the TES, taking the cell and
/// a flag.
const TES_IS_CELL_LOADED: u32 = 0x0045_11e0;
/// `TES::GetCellPriority` (Xbox PDB): the cell and two flags.
const TES_GET_CELL_PRIORITY: u32 = 0x0045_8be0;
/// `ModelLoader::QueueReference` (Xbox PDB): thiscall on the loader, taking
/// the reference and the priority.
const MODEL_LOADER_QUEUE_REFERENCE: u32 = 0x0044_4850;
/// Test on a cell (`00450ff0`).
const CELL_TEST_450FF0: u32 = 0x0045_0ff0;
/// `MobileObject::GetDesiredProcessLevel` (Xbox PDB).
const MOBILE_OBJECT_GET_DESIRED_PROCESS_LEVEL: u32 = 0x0093_34b0;
/// Lock and unlock of the global at `RESURRECT_LOCK`.
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
/// `008a1a40`: thiscall on the actor, taking a flag (the function after
/// `Actor::SetLifeState`; its translation is outside this session's list).
const ACTOR_FN_008A1A40: u32 = 0x008a_1a40;
/// Process virtual slot `0x54` (takes the actor and a kind number).
const PROCESS_SLOT_54: u32 = 0x54;
/// `0097efe0`: thiscall on a combat controller, taking two words.
const CONTROLLER_FORWARD_97EFE0: u32 = 0x0097_efe0;
/// `00483710`: a do-nothing method (it only stores `this`) the combat start
/// calls on `COMBAT_START_GUARD` before and after starting the controller.
const COMBAT_START_GUARD_NO_OP: u32 = 0x0048_3710;

// Translated from 0089f780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::Resurrect` (Xbox PDB), `reset_process`, `queue_load`, `heal`:
/// brings an actor back to life. Clears the delete state (slot `0xc4`),
/// the saved havok and dismemberment extras, the idle played and the
/// player crime extra, removes the actor from the process lists
/// (`00972aa0`), then:
/// - with `heal` set and a collision object (slot `0x1d0`): sets the life
///   state to 0, heals actor value `0x10` to its base through slot `0x3ac`,
///   re-initializes havok and the collision motion and tells the process
///   (slots `0x290`, `0x410`);
/// - otherwise clears the modifier lists, a new `0xb4`-byte process object
///   replaces the actor's one when it is not the player, the life state is
///   set to 0 and the actor is put back in the process level it wants
///   (through the cell load queue when `queue_load` is set).
///
/// The compiler's exception-unwinding frame is not translated.
pub fn actor_resurrect(
    e: &mut Engine,
    this: Ptr<Actor>,
    reset_process: u8,
    queue_load: u8,
    heal: u8,
) {
    e.vcall(this.addr(), ACTOR_SLOT_SET_DELETE, &args![0u32]);
    let extra = extra_data_list_of(e, this.cast());
    e.call(EXTRA_REMOVE_SAVED_HAVOK_DATA, &args![extra]);
    let extra = extra_data_list_of(e, this.cast());
    e.call(EXTRA_REMOVE_DISMEMBERMENT_EXTRA, &args![extra]);
    e.vcall(this.addr(), ACTOR_SLOT_4C, &args![0x0002_0000u32]);
    if e.call(PROCESS_GET_LAST_IDLE_PLAYED, &args![this]).u32() != 0 {
        e.call(ACTOR_FN_008A1A40, &args![this, 0u32]);
    }
    e.mem.set_u8(this.addr() + 0x174, 0);
    e.call(ACTOR_WORKER_884F80, &args![this]);
    e.call(PROCESS_LISTS_WORKER_972AA0, &args![PROCESS_LISTS, this]);
    let extra = extra_data_list_of(e, this.cast());
    e.call(EXTRA_REMOVE_PLAYER_CRIME_LIST_EXTRA, &args![extra, this]);

    if heal != 0 && e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32() != 0 {
        e.call(ACTOR_SET_LIFE_STATE, &args![this, 0u32]);
        let owner = this.addr() + ACTOR_VALUE_OWNER_OFFSET;
        let current = e.vcall(owner, 0x0c, &args![0x10u32]).f32();
        let base = e.vcall(owner, 0x00, &args![0x10u32]).i32();
        let missing = ((base as f32) as f64 - current as f64) as f32;
        e.vcall(
            this.addr(),
            ACTOR_SLOT_MODIFY_ACTOR_VALUE,
            &args![0x10u32, missing, 0u32],
        );
        let process = e.get(this, Actor::pCurrentProcess).addr();
        // A `NiPointer` temporary on the stack, built from null and passed
        // by value.
        e.with_stack(4, |e, pointer| {
            e.call(NI_POINTER_CONSTRUCT, &args![pointer, 0u32]);
            let value = e.mem.u32(pointer.addr());
            e.vcall(process, PROCESS_SLOT_290, &args![value]);
        });
        e.vcall(this.addr(), ACTOR_SLOT_INIT_HAVOK, &args![]);
        let world = e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32();
        e.call(BHK_WORLD_SET_MOTION, &args![world, 1u32, 1u32, 0u32, 1u32]);
        if process != 0 {
            e.vcall(process, PROCESS_SLOT_410, &args![3u32]);
        }
        return;
    }

    e.vcall(this.addr(), ACTOR_SLOT_4C, &args![4u32]);
    e.call(MODIFIER_LIST_DELETE_ALL, &args![this.byte_add(0xd0)]);
    e.call(MODIFIER_LIST_DELETE_ALL, &args![this.byte_add(0xe0)]);
    e.mem.set_u8(this.addr() + 0x118, 0);
    e.call(LOCK_ENTER, &args![RESURRECT_LOCK, 0u32]);
    let player_address = e.global::<u32>(PLAYER_CHARACTER);
    if this.addr() != player_address {
        let process = e.get(this, Actor::pCurrentProcess).addr();
        if process != 0 {
            let level = e.call(ACQUIRE_OBJECT_FIELD_28, &args![process]).u32();
            e.call(
                PROCESS_LISTS_REMOVE_REFERENCE,
                &args![PROCESS_LISTS, this, level],
            );
        }
        let old_process = e.get(this, Actor::pCurrentProcess).addr();
        if old_process != 0 {
            e.vcall(old_process, 0, &args![1u32]);
        }
        let block = e.call(OPERATOR_NEW, &args![0xb4u32]).u32();
        let new_process = if block != 0 {
            e.call(NEW_PROCESS_CONSTRUCT, &args![block]).u32()
        } else {
            0
        };
        e.mem.set_u32(this.addr() + 0x68, new_process);
    }
    if reset_process != 0 {
        e.vcall(this.addr(), ACTOR_SLOT_208, &args![0u32]);
    }
    e.call(ACTOR_SET_LIFE_STATE, &args![this, 0u32]);
    if this.addr() != player_address {
        let cell = e.call(REFR_PARENT_CELL, &args![this]).u32();
        if cell != 0 {
            let tes = e.global::<u32>(TES_SINGLETON);
            if e.call(TES_IS_CELL_LOADED, &args![tes, cell, 0u32]).bool() && queue_load != 0 {
                let priority = e
                    .call(TES_GET_CELL_PRIORITY, &args![tes, cell, 0u32, 0u32])
                    .u32();
                let loader = e.global::<u32>(MODEL_LOADER);
                e.call(MODEL_LOADER_QUEUE_REFERENCE, &args![loader, this, priority]);
                if e.call(CELL_TEST_450FF0, &args![cell]).bool() {
                    e.vcall(this.addr(), ACTOR_SLOT_240, &args![]);
                } else {
                    e.vcall(this.addr(), ACTOR_SLOT_24C, &args![]);
                }
                e.call(LOCK_LEAVE, &args![RESURRECT_LOCK]);
                return;
            }
        }
        match e
            .call(MOBILE_OBJECT_GET_DESIRED_PROCESS_LEVEL, &args![this])
            .u32()
        {
            0 => {
                e.vcall(this.addr(), ACTOR_SLOT_240, &args![]);
            }
            1 => {
                e.vcall(this.addr(), ACTOR_SLOT_24C, &args![]);
            }
            2 => {
                e.vcall(this.addr(), ACTOR_SLOT_248, &args![]);
            }
            3 => {
                e.call(
                    PROCESS_LISTS_ADD_REFERENCE,
                    &args![PROCESS_LISTS, this, 3u32, 0u32, 0u32, 0u32],
                );
            }
            _ => {}
        }
    }
    e.call(LOCK_LEAVE, &args![RESURRECT_LOCK]);
}

// Translated from 0089fb80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor` virtual slot `0x208`: re-initializes the actor's worn items.
/// Unless the base data predicate (`0047c850`) is set, asks the acquire
/// object (when its field `+0x28` is at most 1; otherwise the actor itself
/// through slot `0x1cc`) to refresh, clears the path manager's pointers into
/// the actor's inventory and runs `00574920`; then, for an actor with a
/// collision object (slot `0x1d0`), runs the default-worn setup of its base
/// form: the NPC one (form type `0x2a`) or the creature one (type `0x2b`),
/// with a flag that is cleared while the current package is of the kind
/// `00441b00` accepts.
pub fn fn_0089fb80(e: &mut Engine, this: Ptr<Actor>, flag: u8) {
    let base_data = e.global::<u32>(BASE_DATA_OBJECT);
    if !e.call(BASE_DATA_PREDICATE, &args![base_data]).bool() {
        let mut object = 0;
        if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            if e.call(ACQUIRE_OBJECT_FIELD_28, &args![acquire]).i32() <= 1 {
                object = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            }
        }
        if object != 0 {
            e.vcall(object, PROCESS_SLOT_468, &args![1u32]);
        } else {
            e.vcall(this.addr(), ACTOR_SLOT_1CC, &args![0u32, 1u32]);
        }
    }
    let extra = extra_data_list_of(e, this.cast());
    let changes = e.call(GET_CONTAINER_CHANGES, &args![extra]).u32();
    let manager = e.call(PATH_MANAGER_Q_INSTANCE, &args![]).u32();
    e.call(
        PATH_MANAGER_CLEAR_INVENTORY_POINTERS,
        &args![manager, changes],
    );
    e.call(ACTOR_UPDATE_INVENTORY_WORKER, &args![this, flag]);
    if e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32() != 0 {
        let form = base_form(e, this.cast());
        let mut npc_form = Ptr::NULL;
        let mut creature_form = Ptr::NULL;
        match form_type(e, form) {
            0x2a => npc_form = form,
            0x2b => creature_form = form,
            _ => {}
        }
        let mut apply = 1u32;
        let package = e.call(GET_CURRENT_PACKAGE, &args![this]).u32();
        if package != 0 && e.call(PACKAGE_KIND_TEST, &args![package]).bool() {
            apply = 0;
        }
        if !npc_form.is_null() {
            e.call(
                NPC_INIT_DEFAULT_WORN,
                &args![npc_form, this, 1u32, apply, 0u32, 1u32],
            );
        } else if !creature_form.is_null() {
            e.call(
                CREATURE_INIT_DEFAULT_WORN,
                &args![creature_form, this, 1u32, apply, 1u32],
            );
        }
    }
}

// Translated from 0089fcf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Starts combat between this actor and `target` (or a combat group, or a
/// pretend target). Does nothing for actors whose process level is 5 or 3
/// (`00437bf0`, `00437bd0`), for the target being the actor, for protected
/// (slot `0x22c(0)`) or dead actors. Unless `forced`, an actor in an
/// interrupting state (slot `0x214` not 0, 4 or 9) needs a furniture marker
/// entry whose byte `+0xe` is above `0x14` (slot `0x418` runs then); when
/// `0087f9c0` is 0 the target must be within the actor's reach plus a
/// quarter of it. Without a combat controller (slot `0x428`) the actor's
/// process is told (slot `0x2c8(0.0)`), the player's hostile status updated,
/// the crosshair cleared when it was on the actor, the equipped item queued
/// again, the interrupt package ended, and the combatant is added through
/// the `CombatManager` (a group member, a combatant with `value` or a
/// pretend combatant); the new controller then gets the acquire object's
/// slot `0x710` and is added as a package; byte `+0x104` (`bInCombat`)
/// records whether there is one. With a controller, a different target is
/// added to it, otherwise the group member is added. `value` is worked out
/// from the acquire object's slot `0x208` when it is 0.
#[allow(clippy::too_many_arguments)]
pub fn fn_0089fcf0(
    e: &mut Engine,
    this: Ptr<Actor>,
    target: Ptr<Actor>,
    group: u32,
    flag_a: u8,
    _unused_3: u32,
    flag_b: u8,
    value: i32,
    _unused_6: u32,
    pretend: u32,
) {
    if e.call(ACTOR_PROCESS_LEVEL_IS_5, &args![this]).bool()
        || e.call(ACTOR_PROCESS_LEVEL_IS_3, &args![this]).bool()
        || target == this
        || e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool()
        || e.mem.u8(this.addr() + 0x118) != 0
    {
        return;
    }
    if flag_a == 0 {
        let kind = e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).i32();
        if kind != 0 && kind != 4 && kind != 9 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            if e.vcall(acquire, PROCESS_SLOT_4D4, &args![]).u32() == 0 {
                return;
            }
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            let entry = e.vcall(acquire, PROCESS_SLOT_4D4, &args![]).u32();
            if e.mem.u8(entry + 0xe) <= 0x14 {
                return;
            }
            e.vcall(this.addr(), ACTOR_SLOT_418, &args![]);
        }
    }
    if e.call(ACTOR_GETTER_87F9C0, &args![this]).f64() == double_constant(e, ZERO_DOUBLE) {
        let distance = e
            .call(
                REFR_GET_DISTANCE_FROM_REFERENCE,
                &args![this, target, 0u32, 0u32],
            )
            .f32();
        let quarter_reach = (e.vcall(this.addr(), ACTOR_SLOT_REACH, &args![]).f64()
            * double_constant(e, QUARTER)) as f32;
        let reach = e.vcall(this.addr(), ACTOR_SLOT_REACH, &args![]).f64();
        if distance as f64 > reach + quarter_reach as f64 {
            return;
        }
    }
    let ranked = value > 0;
    let mut combat_value = 0;
    if value == 0 {
        let mut other = this.addr();
        if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            if e.vcall(acquire, PROCESS_SLOT_52C, &args![]).u32() != 0 {
                let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
                other = e.vcall(acquire, PROCESS_SLOT_52C, &args![]).u32();
            }
        }
        if !target.is_null() && e.call(GET_SAVED_ACQUIRE_OBJECT, &args![target]).u32() != 0 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![target]).u32();
            let result = e
                .vcall(acquire, PROCESS_SLOT_208, &args![target, other])
                .f64();
            combat_value = e.call(FTOL, &args![result]).i32();
        }
    } else {
        combat_value = value;
    }
    let manager = e.global::<u32>(COMBAT_MANAGER);
    if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() == 0 {
        let process = e.get(this, Actor::pCurrentProcess).addr();
        e.vcall(process, PROCESS_SLOT_2C8, &args![0.0f32]);
        let player_address = e.global::<u32>(PLAYER_CHARACTER);
        if target.addr() == player_address {
            e.call(
                PLAYER_CHANGE_PERCEIVED_HOSTILE_STATUS,
                &args![player_address, this, 1u32],
            );
        }
        // The base form's type is read but its result is not used.
        let form = base_form(e, this.cast());
        form_type(e, form);
        if e.call(CROSSHAIR_TARGET, &args![]).u32() == this.addr() {
            e.call(SET_CROSSHAIR_TARGET_TYPE, &args![0u32]);
        }
        let process = e.get(this, Actor::pCurrentProcess).addr();
        let held = e.vcall(process, PROCESS_SLOT_22C, &args![]).u32();
        if held != 0 {
            let held = e.vcall(process, PROCESS_SLOT_22C, &args![]).u32();
            if e.call(PACKAGE_KIND_TEST, &args![held]).bool() {
                let item = e.vcall(this.addr(), ACTOR_SLOT_3BC, &args![6u32]).u32();
                if item != 0 {
                    // (the compiler also looked at the entry's form type
                    // here, without using it)
                    if e.call(ENTRY_FORM, &args![item]).u32() != 0 {
                        let form = e.call(ENTRY_FORM, &args![item]).ptr::<()>();
                        if form_type(e, form) == 0x28 {
                            e.call(ENTRY_FORM, &args![item]);
                        }
                    }
                    let form = e.call(ENTRY_FORM, &args![item]).u32();
                    e.call(
                        ACTOR_QUEUE_EQUIP_OBJECT,
                        &args![this, form, 1u32, 0u32, 1u32, 0u32, 1u32],
                    );
                    delete_entry(e, Ptr::new(item));
                }
            }
        }
        e.call(ACTOR_END_INTERRUPT_PACKAGE, &args![this, 0u32]);
        let process = e.get(this, Actor::pCurrentProcess).addr();
        e.vcall(process, PROCESS_SLOT_214, &args![]);
        let kind = e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).i32();
        if kind == 4 || e.vcall(this.addr(), ACTOR_SLOT_214, &args![]).i32() == 9 {
            e.vcall(this.addr(), ACTOR_SLOT_418, &args![]);
        }
        let mut controller = 0;
        if group != 0 {
            controller = e
                .call(
                    COMBAT_MANAGER_ADD_GROUP_MEMBER,
                    &args![manager, this, group],
                )
                .u32();
        } else if !target.is_null() {
            controller = e
                .call(
                    COMBAT_MANAGER_ADD_COMBATANT,
                    &args![manager, this, target, combat_value, u32::from(ranked)],
                )
                .u32();
        } else if pretend != 0 {
            controller = e
                .call(
                    COMBAT_MANAGER_ADD_PRETEND_COMBATANT,
                    &args![manager, this, pretend],
                )
                .u32();
        }
        if flag_b != 0 && controller != 0 {
            e.call(
                COMBAT_CONTROLLER_NOTIFY_DAMAGE,
                &args![controller, target, 0u32],
            );
        }
        if flag_a != 0 && controller != 0 {
            fn_008a0250(e, Ptr::new(controller));
        }
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(acquire, PROCESS_SLOT_28, &args![]);
        if controller != 0 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            e.vcall(acquire, PROCESS_SLOT_710, &args![this]);
            e.vcall(
                this.addr(),
                ACTOR_SLOT_ADD_PACKAGE,
                &args![controller, 0u32, 1u32],
            );
            e.mem.set_u8(this.addr() + 0x104, 1);
        } else {
            e.mem.set_u8(this.addr() + 0x104, 0);
        }
    } else if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() != 0 {
        if !target.is_null() {
            let controller = fn_008a0330(e, this);
            if e.call(COMBAT_CONTROLLER_TARGET, &args![controller]).u32() != target.addr() {
                let controller = fn_008a0330(e, this);
                e.call(
                    COMBAT_CONTROLLER_ADD_TARGET,
                    &args![
                        controller,
                        target,
                        combat_value,
                        u32::from(ranked),
                        0.0f32,
                        0.0f32
                    ],
                );
                return;
            }
        }
        if group != 0 {
            e.call(
                COMBAT_MANAGER_ADD_GROUP_MEMBER,
                &args![manager, this, group],
            );
        }
    }
}

// Translated from 008a0270 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the actor is in combat (`bInCombat`, `+0x104`): 0 for the player
/// and for an actor without a process; with `ignore_searching` set, 0 also
/// when it is only searching in combat (`bSearchingInCombat`, `+0x127`).
pub fn fn_008a0270(e: &mut Engine, this: Ptr<Actor>, ignore_searching: u8) -> u8 {
    if this.addr() == e.global::<u32>(PLAYER_CHARACTER) {
        return 0;
    }
    if e.get(this, Actor::pCurrentProcess).is_null() {
        return 0;
    }
    let in_combat = e.mem.u8(this.addr() + 0x104);
    if in_combat != 0 && ignore_searching != 0 && e.mem.u8(this.addr() + 0x127) != 0 {
        return 0;
    }
    in_combat
}

// Translated from 008a02d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetCombatController` (the base-class body of `Actor` slot
/// `0x428`): the object the actor's process holds in its slot `0x22c`, when
/// the actor is in combat and that object's type byte (`+0x20`) is `0x12`;
/// otherwise 0.
pub fn fn_008a02d0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let process = e.get(this, Actor::pCurrentProcess).addr();
    if process == 0 || !e.call(ACTOR_IN_COMBAT, &args![this]).bool() {
        return 0;
    }
    let held = e.vcall(process, PROCESS_SLOT_22C, &args![]).u32();
    if held != 0 && e.call(PACKAGE_TYPE, &args![held]).i32() == 0x12 {
        held
    } else {
        0
    }
}

// Translated from 008a0360 (decompiled, FalloutNV.exe 1.4.0.525)
/// The target of the actor's combat controller (slot `0x428`), or 0.
pub fn fn_008a0360(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
    if controller != 0 {
        e.call(COMBAT_CONTROLLER_TARGET, &args![controller]).u32()
    } else {
        0
    }
}

// Translated from 008a03a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Keeps the actor's combat going once the screen has faded in: does
/// nothing while the fader (number 1) is visible, for actors failing
/// `00576d30`, actors not in combat or without a combat controller.
/// Stops the combat (slot `0x434`) when `00981450` says so or the
/// controller's target is no longer a combat target or is protected, dead
/// or otherwise unsuitable; otherwise registers the target with the
/// player's combat list, starts the controller (slot `0x64c`, or `0x630`
/// with the target) and runs the high (process level 0) or other combat
/// start of the controller.
pub fn fn_008a03a0(e: &mut Engine, this: Ptr<Actor>) {
    let fader = e.global::<u32>(FADER_MANAGER);
    let alpha = e
        .call(FADER_MANAGER_GET_FADER_ALPHA, &args![fader, 1u32])
        .f32();
    if alpha as f64 > double_constant(e, ZERO_DOUBLE) {
        return;
    }
    if e.call(ACTOR_TEST_576D30, &args![this]).bool() {
        return;
    }
    if !e.call(ACTOR_IN_COMBAT, &args![this]).bool() {
        return;
    }
    let mut controller = fn_008a0330(e, this);
    if controller == 0 {
        return;
    }
    if e.call(CONTROLLER_TEST_981450, &args![controller]).bool() {
        e.vcall(this.addr(), ACTOR_SLOT_STOP_COMBAT, &args![0u32]);
        return;
    }
    let mut target = e.call(COMBAT_CONTROLLER_TARGET, &args![controller]).u32();
    if target != 0
        && !e
            .call(
                COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET,
                &args![controller, target],
            )
            .bool()
    {
        e.vcall(this.addr(), ACTOR_SLOT_STOP_COMBAT, &args![target]);
        controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
        if controller == 0 {
            return;
        }
        target = e.call(COMBAT_CONTROLLER_TARGET, &args![controller]).u32();
    }
    if e.call(ACTOR_PROCESS_LEVEL_IS_5, &args![this]).bool() {
        return;
    }
    let player_address = e.global::<u32>(PLAYER_CHARACTER);
    if target == player_address
        && e.call(PLAYER_TEST_5A03F0, &args![player_address, 1u32])
            .bool()
    {
        return;
    }
    if target != 0 {
        let protected = e.vcall(target, ACTOR_SLOT_22C, &args![0u32]).bool();
        let unsuitable = (protected && !e.vcall(target, ACTOR_SLOT_2E8, &args![]).bool())
            || e.call(ACTOR_TEST_440DA0, &args![target]).bool()
            || e.call(ACTOR_TEST_440D80, &args![target]).bool()
            || e.call(ACTOR_PROCESS_LEVEL_IS_3, &args![target]).bool();
        if unsuitable {
            e.vcall(this.addr(), ACTOR_SLOT_STOP_COMBAT, &args![target]);
            return;
        }
    }
    if target == player_address
        && e.call(PLAYER_COMBAT_LIST_COUNT, &args![player_address, this])
            .i32()
            > 0
    {
        e.call(
            PLAYER_ADD_ACTOR_TO_COMBAT_LIST,
            &args![player_address, this],
        );
    }
    if e.call(CONTROLLER_TEST_9818B0, &args![controller]).bool() {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(acquire, PROCESS_SLOT_64C, &args![]);
    } else {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(acquire, PROCESS_SLOT_630, &args![target]);
    }
    e.call(COMBAT_START_GUARD_NO_OP, &args![COMBAT_START_GUARD]);
    if e.call(MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, &args![this])
        .i32()
        == 0
    {
        e.call(CONTROLLER_START_HIGH, &args![controller]);
    } else {
        e.call(CONTROLLER_START_OTHER, &args![controller]);
    }
    e.call(COMBAT_START_GUARD_NO_OP, &args![COMBAT_START_GUARD]);
}

// Translated from 008a05f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// With a combat controller (slot `0x428`) calls the do-nothing `Error`
/// (`0040fbe0`) with the actor; without one sets the actor's process
/// (slot `0x54`) to the kind `1` when the process' field `+0x28` is 1, `2`
/// when it is 2 and `4` otherwise.
pub fn fn_008a05f0(e: &mut Engine, this: Ptr<Actor>) {
    if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() != 0 {
        e.call(ERROR_NO_OP, &args![this]);
        return;
    }
    let process = e.get(this, Actor::pCurrentProcess).addr();
    let kind = match e.call(ACQUIRE_OBJECT_FIELD_28, &args![process]).u32() {
        1 => 1u32,
        2 => 2,
        _ => 4,
    };
    e.vcall(process, PROCESS_SLOT_54, &args![this, kind]);
}

// Translated from 008a0680 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards two words to `0097efe0` on the actor's combat controller (slot
/// `0x428`) when there is one.
pub fn fn_008a0680(e: &mut Engine, this: Ptr<Actor>, first: u32, second: u32) {
    let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
    if controller != 0 {
        e.call(CONTROLLER_FORWARD_97EFE0, &args![controller, first, second]);
    }
}

/// `Actor::GetCurrentWeapon` (Xbox PDB).
const ACTOR_GET_CURRENT_WEAPON: u32 = 0x008a_1710;
/// `Actor::GetPackageSetAsPcurrent` (Xbox PDB).
const ACTOR_GET_PACKAGE_SET_AS_PCURRENT: u32 = 0x0088_1510;
/// Test on the actor (`008ace90`).
const ACTOR_TEST_8ACE90: u32 = 0x008a_ce90;
/// The process level number of an actor or process (`004f8960`; 4 is
/// compared with).
const PROCESS_LEVEL_NUMBER: u32 = 0x004f_8960;
/// `Actor::GetDetectionLevelAgainstActor` (Xbox PDB), thiscall with the 7
/// words `0, other, &flag, 1, 1, 1, &flag`; returns the detection level.
const ACTOR_GET_DETECTION_LEVEL_AGAINST_ACTOR: u32 = 0x008a_0d10;
/// Test on a package (`0067a770`).
const PACKAGE_TEST_67A770: u32 = 0x0067_a770;
/// Test on the actor (`005a3790`).
const ACTOR_TEST_5A3790: u32 = 0x005a_3790;
/// Pointer to the integer value of a setting (`0043d4d0`, `this` = the
/// setting).
const SETTING_GET_INT_POINTER: u32 = 0x0043_d4d0;
/// `ExtraDataList` call that runs when the friend-hit limit is at least
/// 1000 (`00422670`).
const EXTRA_RESET_FRIEND_HITS: u32 = 0x0042_2670;
/// `ExtraDataList::AddFriendHit` (Xbox PDB): returns the new count.
const EXTRA_ADD_FRIEND_HIT: u32 = 0x0042_2590;
/// `CombatDialogueManager::StartDialogue_ov2` (Xbox PDB): thiscall on the
/// manager with the speaker, the target and four numbers.
const COMBAT_DIALOGUE_START_DIALOGUE: u32 = 0x0098_39b0;
/// The `CombatDialogueManager` singleton pointer.
const COMBAT_DIALOGUE_MANAGER: u32 = 0x011f_1708;
/// `Actor::IsTalking` (Xbox PDB).
const ACTOR_IS_TALKING: u32 = 0x008a_67f0;
/// `MobileObject::IsinDialogue` (Xbox PDB).
const MOBILE_OBJECT_IS_IN_DIALOGUE: u32 = 0x0093_36c0;
/// `ProcessLists::GetActorRefInHigh` (Xbox PDB): thiscall on the process
/// lists with a form and a flag.
const PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH: u32 = 0x0097_0a20;
/// The three-word variant of that lookup, used for owners of form type 8
/// (`00970b30`: the form, 0, 1).
const PROCESS_LISTS_GET_OWNER_ACTOR: u32 = 0x0097_0b30;
/// `Actor::AttackAlarm` (Xbox PDB): thiscall with the attacker, a flag and
/// `1`.
const ACTOR_ATTACK_ALARM: u32 = 0x008c_0460;
/// Test taking the other actor (`008b0670`).
const ACTOR_TEST_8B0670: u32 = 0x008b_0670;
/// Returns an actor related to the other one (`008b0ba0`).
const ACTOR_RELATED_8B0BA0: u32 = 0x008b_0ba0;
/// `Actor::IsAngryWithPlayer` (Xbox PDB).
const ACTOR_IS_ANGRY_WITH_PLAYER: u32 = 0x008b_ffc0;
/// `Actor::GetShouldAttackActor` (Xbox PDB): the other actor, a flag, a
/// pointer to the reaction and a flag.
const ACTOR_GET_SHOULD_ATTACK_ACTOR: u32 = 0x008b_06d0;
/// `Actor::IsFleeing` (Xbox PDB): takes a flag.
const ACTOR_IS_FLEEING: u32 = 0x008a_6650;
/// Returns the object at `this + 0x30` of a base form (`005d8a70` is called
/// with it); the result goes to `008256d0`.
const FORM_SUB_OBJECT_ACCESSOR: u32 = 0x005d_8a70;
/// Test on that object (`008256d0`).
const FORM_SUB_OBJECT_TEST: u32 = 0x0082_56d0;
/// `0047ead0`: thiscall on the base form's embedded object (`+0x30`), taking
/// a flag.
const BASE_DATA_SET_FLAG: u32 = 0x0047_ead0;
/// `0097f580`: thiscall on the combat controller with the target and a word.
const CONTROLLER_WORKER_97F580: u32 = 0x0097_f580;
/// Process virtual slot `0x504` (target, flag): returns the detection record
/// of the target, or 0.
const PROCESS_SLOT_504: u32 = 0x504;
/// Process virtual slot `0xf0`: creates a detection record (target, level,
/// flag, detection, 0, flag, 1).
const PROCESS_SLOT_F0: u32 = 0xf0;
/// Process virtual slot `0x33c` (13 words).
const PROCESS_SLOT_33C: u32 = 0x33c;
/// `Actor` virtual slot `0x100` (a bool on the target).
const ACTOR_SLOT_100: u32 = 0x100;
/// `Actor` virtual slot `0x48` (takes a flag mask).
const ACTOR_SLOT_48: u32 = 0x48;
/// `Actor` virtual slot `0x304` (bool).
const ACTOR_SLOT_304: u32 = 0x304;
/// `Actor` virtual slot `0x37c`.
const ACTOR_SLOT_37C: u32 = 0x37c;
/// `Actor` virtual slot `0x42c` (returns an actor: the one this actor is
/// attacking or following).
const ACTOR_SLOT_42C: u32 = 0x42c;
/// `Actor` virtual slot `0x21c` (bool).
const ACTOR_SLOT_21C: u32 = 0x21c;
/// `Actor` virtual slot `0x460` (target, float).
const ACTOR_SLOT_460: u32 = 0x460;
/// `Actor` virtual slot `0x474` (flag).
const ACTOR_SLOT_474: u32 = 0x474;
/// Settings read by `fn_008987f0`: detection threshold (`float`), the
/// `float` that is unused afterwards, the friend-hit limits (ints) and the
/// float passed to slot `0x460`.
const DETECTION_THRESHOLD_SETTING: u32 = 0x011c_df10;
const UNUSED_FLOAT_SETTING: u32 = 0x011c_d45c;
const FRIEND_HIT_LIMIT_SETTINGS: [u32; 4] = [0x011c_d204, 0x011c_d5b0, 0x011c_d4b4, 0x011c_d468];
const SLOT_460_FLOAT_SETTING: u32 = 0x011d_13e0;

// Translated from 008987f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reacts to `target` being noticed (or acting) near this actor. Returns
/// early for protected actors (slot `0x22c(0)`), actors failing `008ace90`,
/// `target == this` and an actor of process level 4 whose slot `0x52c`
/// object is the target. Otherwise, unless this actor's combat controller
/// (slot `0x428`) already has `target` as a combat target:
/// - works out the detection level against the target
///   (`GetDetectionLevelAgainstActor`) and, at the high process level, stores
///   it (with the level 3 when it exceeds the detection setting) in the
///   target's detection record in the acquire object (slot `0x504`, created
///   by slot `0xf0` if needed);
/// - when the target is the player: asks the faction reaction; for reaction
///   2 or 3 counts a friend hit against the setting limit and either starts
///   the dialogue of `CombatDialogueManager` (talking and dialogue states
///   permitting) or goes on to raise the attack alarm through the owner and
///   itself;
/// - when `target` is not the player, this actor passes `0x304`, has no
///   controller and the target answers slots `0x218` (twice) and `0x37c`:
///   raises the attack alarm (unless the player branch did) and returns;
/// - when `008b0670` says so and `target` relates to this actor or the
///   player: works out whether it should attack, steals or alarms, sets the
///   "bark" flag through slot `0x460`, and tells the acquire object (slot
///   `0x33c`, 13 words);
///
/// Then lets the actor's slot `0x474` run, updates the base data (`+0x30`
/// object) and, when nothing alarmed and the flagged alarm is still due,
/// raises it; finally passes `target` and `param` to the combat controller
/// (`0097f580`) when the actor has one.
pub fn fn_008987f0(e: &mut Engine, this: Ptr<Actor>, target: Ptr<Actor>, param: u32) {
    if e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
        return;
    }
    if e.call(ACTOR_TEST_8ACE90, &args![this]).bool() {
        return;
    }
    if target == this {
        return;
    }
    let package = e
        .call(ACTOR_GET_PACKAGE_SET_AS_PCURRENT, &args![this])
        .u32();
    if e.call(PROCESS_LEVEL_NUMBER, &args![this]).i32() == 4 {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        if e.vcall(acquire, PROCESS_SLOT_52C, &args![]).u32() == target.addr() {
            return;
        }
    }
    let mut weapon_flag = 0u8;
    let mut deferred_alarm = false;
    let mut alarmed = false;
    let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    let process_type = e
        .call(MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, &args![this])
        .i32();
    let weapon = if target.is_null() {
        0
    } else {
        e.call(ACTOR_GET_CURRENT_WEAPON, &args![target]).u32()
    };
    if weapon != 0 && fn_00898fd0(e, Ptr::new(weapon)) {
        weapon_flag = 1;
    }
    let player_address = e.global::<u32>(PLAYER_CHARACTER);
    let already_target = controller != 0
        && e.call(
            COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET,
            &args![controller, target],
        )
        .bool();
    if !already_target {
        // Two bytes the detection call writes: the first at +0, the second
        // (preset to 1) at +1.
        let flags = e.mem.alloc(4);
        e.mem.set_u8(flags + 1, 1);
        let detection = e
            .call(
                ACTOR_GET_DETECTION_LEVEL_AGAINST_ACTOR,
                &args![this, 0u32, target, flags, 1u32, 1u32, 1u32, flags + 1],
            )
            .i32();
        let mut level = 0u32;
        let setting = e
            .call(
                SETTING_GET_VALUE_POINTER,
                &args![DETECTION_THRESHOLD_SETTING],
            )
            .u32();
        if (e.mem.f32(setting) as f64) < detection as f64 {
            level = 3;
        }
        if process_type == 0 {
            let mut record = e
                .vcall(acquire, PROCESS_SLOT_504, &args![target, 0u32])
                .u32();
            if record == 0 && e.vcall(target.addr(), ACTOR_SLOT_100, &args![]).bool() {
                record = e
                    .vcall(
                        acquire,
                        PROCESS_SLOT_F0,
                        &args![
                            target,
                            level,
                            u32::from(e.mem.u8(flags)),
                            detection,
                            0u32,
                            u32::from(e.mem.u8(flags + 1)),
                            1u32
                        ],
                    )
                    .u32();
            }
            e.mem.set_u32(record + 8, detection as u32);
            e.mem.set_u32(record + 4, level);
            let detected_flag = e.mem.u8(flags);
            e.mem.set_u8(record + 0x1e, detected_flag);
            let second_flag = e.mem.u8(flags + 1);
            e.mem.set_u8(record + 0x1c, second_flag);
            e.mem.set_u8(record + 0x1d, 1);
        }
        // The value of this setting is read; nothing uses it afterwards.
        e.call(SETTING_GET_VALUE_POINTER, &args![UNUSED_FLOAT_SETTING]);
        if package != 0 {
            e.call(PACKAGE_TEST_67A770, &args![package]);
        }
        if target.addr() == player_address
            && (controller == 0
                || !e
                    .call(
                        COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET,
                        &args![controller, player_address],
                    )
                    .bool())
        {
            let reaction_flag = e.mem.alloc(4);
            e.mem.set_u8(reaction_flag, 0);
            let reaction = e
                .call(
                    ACTOR_GET_FACTION_FIGHT_REACTION,
                    &args![this, player_address, reaction_flag],
                )
                .i32();
            if reaction == 3 || reaction == 2 {
                if e.call(ACTOR_TEST_5A3790, &args![this]).bool() {
                    return;
                }
                let setting_address = match (reaction == 3, controller != 0) {
                    (true, true) => FRIEND_HIT_LIMIT_SETTINGS[0],
                    (true, false) => FRIEND_HIT_LIMIT_SETTINGS[1],
                    (false, true) => FRIEND_HIT_LIMIT_SETTINGS[2],
                    (false, false) => FRIEND_HIT_LIMIT_SETTINGS[3],
                };
                let pointer = e
                    .call(SETTING_GET_INT_POINTER, &args![setting_address])
                    .u32();
                let limit = e.mem.u32(pointer) as i32;
                if limit > 0 {
                    if limit >= 1000 {
                        let extra = extra_data_list_of(e, this.cast());
                        e.call(EXTRA_RESET_FRIEND_HITS, &args![extra]);
                    }
                    let extra = extra_data_list_of(e, this.cast());
                    let hits = e.call(EXTRA_ADD_FRIEND_HIT, &args![extra]).i32();
                    e.vcall(this.addr(), ACTOR_SLOT_48, &args![0x8000_0000u32]);
                    if hits <= limit {
                        if e.call(ACTOR_IS_TALKING, &args![this]).bool() {
                            return;
                        }
                        if e.call(MOBILE_OBJECT_IS_IN_DIALOGUE, &args![this]).bool() {
                            return;
                        }
                        if e.call(ACTOR_SHOULD_SKIP_FALLOUT_BEHAVIOR, &args![this, 5u32])
                            .bool()
                        {
                            return;
                        }
                        let manager = e.global::<u32>(COMBAT_DIALOGUE_MANAGER);
                        e.call(
                            COMBAT_DIALOGUE_START_DIALOGUE,
                            &args![manager, this, target, 2u32, 2u32, 1u32, 0u32],
                        );
                        return;
                    }
                }
            }
            if e.call(REFR_GET_OWNER, &args![this]).u32() != 0 {
                let owner = e.call(REFR_GET_OWNER, &args![this]).ptr::<()>();
                let owner_actor = if form_type(e, owner) == 8 {
                    e.call(
                        PROCESS_LISTS_GET_OWNER_ACTOR,
                        &args![PROCESS_LISTS, owner, 0u32, 1u32],
                    )
                    .u32()
                } else {
                    e.call(
                        PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH,
                        &args![PROCESS_LISTS, owner, 0u32],
                    )
                    .u32()
                };
                if owner_actor != 0 {
                    e.call(
                        ACTOR_ATTACK_ALARM,
                        &args![owner_actor, target, u32::from(weapon_flag), 1u32],
                    );
                }
            }
            e.call(
                ACTOR_ATTACK_ALARM,
                &args![this, target, u32::from(weapon_flag), 1u32],
            );
            alarmed = true;
        }
        if target.addr() != player_address
            && e.vcall(this.addr(), ACTOR_SLOT_304, &args![]).bool()
            && controller == 0
            && e.vcall(target.addr(), ACTOR_SLOT_218, &args![]).bool()
            && e.vcall(target.addr(), ACTOR_SLOT_218, &args![]).bool()
            && e.vcall(target.addr(), ACTOR_SLOT_37C, &args![]).u32() != 0
        {
            if !alarmed {
                e.call(
                    ACTOR_ATTACK_ALARM,
                    &args![this, target, u32::from(weapon_flag), 1u32],
                );
            }
            return;
        }
        if e.call(ACTOR_TEST_8B0670, &args![this, target]).bool() {
            let related = target.addr() == player_address
                || e.vcall(target.addr(), ACTOR_SLOT_42C, &args![]).u32() == this.addr()
                || (e.call(ACTOR_RELATED_8B0BA0, &args![target]).u32() == player_address
                    && e.vcall(target.addr(), ACTOR_SLOT_42C, &args![]).u32() == player_address);
            if related && this.addr() != player_address {
                let mut should_attack = 0u32;
                let state = e.mem.alloc(8);
                let reaction = e
                    .call(
                        ACTOR_GET_FACTION_FIGHT_REACTION,
                        &args![this, target, state + 4],
                    )
                    .u32();
                e.mem.set_u32(state, reaction);
                if !e.call(ACTOR_IS_ANGRY_WITH_PLAYER, &args![this]).bool()
                    && e.mem.u8(state + 4) == 0
                    && e.call(
                        ACTOR_GET_SHOULD_ATTACK_ACTOR,
                        &args![this, target, 0u32, state, 0u32],
                    )
                    .bool()
                {
                    should_attack = 1;
                }
                if controller == 0
                    && !e.vcall(target.addr(), ACTOR_SLOT_304, &args![]).bool()
                    && target.addr() == player_address
                {
                    if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() == 0
                        && !e.call(ACTOR_IS_FLEEING, &args![this, 0u32]).bool()
                        && !alarmed
                    {
                        if e.vcall(this.addr(), ACTOR_SLOT_21C, &args![]).bool() {
                            let extra = extra_data_list_of(e, this.cast());
                            if e.call(EXTRA_GET_OWNER_FORM, &args![extra]).u32() != 0 {
                                let extra = extra_data_list_of(e, this.cast());
                                let owner = e.call(EXTRA_GET_OWNER_FORM, &args![extra]).u32();
                                e.call(
                                    ACTOR_STEAL_ALARM,
                                    &args![player_address, this, 0u32, 1u32, 0u32, owner],
                                );
                            }
                        } else {
                            e.call(
                                ACTOR_ATTACK_ALARM,
                                &args![this, target, u32::from(weapon_flag), 1u32],
                            );
                        }
                    } else {
                        let setting = e
                            .call(SETTING_GET_VALUE_POINTER, &args![SLOT_460_FLOAT_SETTING])
                            .u32();
                        let value = e.mem.f32(setting);
                        e.vcall(this.addr(), ACTOR_SLOT_460, &args![target, value]);
                        deferred_alarm = true;
                    }
                }
                e.vcall(
                    acquire,
                    PROCESS_SLOT_33C,
                    &args![
                        this,
                        target,
                        1u32,
                        1u32,
                        0u32,
                        0u32,
                        0u32,
                        should_attack,
                        0u32,
                        0u32,
                        should_attack,
                        0u32,
                        0u32
                    ],
                );
            }
        }
        if this.addr() != player_address || e.call(ACTOR_IN_COMBAT, &args![target]).bool() {
            e.vcall(this.addr(), ACTOR_SLOT_474, &args![1u32]);
        }
        let mut base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
        if base != 0 {
            let part = e.call(FORM_SUB_OBJECT_ACCESSOR, &args![base + 0x30]).u32();
            if e.call(FORM_SUB_OBJECT_TEST, &args![part]).bool() {
                base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
            }
        }
        if target.addr() == player_address {
            e.call(BASE_DATA_SET_FLAG, &args![base + 0x30, 1u32]);
        }
    }
    if !alarmed && deferred_alarm && !e.call(ACTOR_IN_COMBAT, &args![this]).bool() {
        if e.vcall(this.addr(), ACTOR_SLOT_21C, &args![]).bool() {
            let extra = extra_data_list_of(e, this.cast());
            if e.call(EXTRA_GET_OWNER_FORM, &args![extra]).u32() != 0 {
                let extra = extra_data_list_of(e, this.cast());
                let owner = e.call(EXTRA_GET_OWNER_FORM, &args![extra]).u32();
                e.call(
                    ACTOR_STEAL_ALARM,
                    &args![player_address, this, 0u32, 1u32, 0u32, owner],
                );
            }
        } else {
            e.call(
                ACTOR_ATTACK_ALARM,
                &args![this, target, u32::from(weapon_flag), 1u32],
            );
        }
    }
    let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
    if controller != 0 {
        e.call(CONTROLLER_WORKER_97F580, &args![controller, target, param]);
    }
}

/// The `VATS` singleton (the address the code passes as `this` of the
/// `VATS` calls and of the accessor `0044ddc0`; the dword at `+8` is 4 while
/// a VATS action plays).
const VATS_OBJECT: u32 = 0x011f_2250;
/// The pointer to the actor VATS is acting for.
const VATS_ACTOR: u32 = 0x011f_21f0;
/// `VATS::GetCurrentAction` (Xbox PDB).
const VATS_GET_CURRENT_ACTION: u32 = 0x009c_71c0;
/// `BGSDefaultObjectManager::GetDefaultObject` (Xbox PDB), cdecl.
const GET_DEFAULT_OBJECT: u32 = 0x0058_db10;
/// The RTTI type descriptor of `MagicItem` (the class the default object
/// number 5 is cast to).
const RTTI_TYPE_MAGIC_ITEM: u32 = 0x0118_3140;
/// `MiscStatManager::Increment` (Xbox PDB), cdecl.
const MISC_STAT_INCREMENT: u32 = 0x004d_5c60;
/// Offset of the magic caster embedded in an `Actor`.
const ACTOR_MAGIC_CASTER_OFFSET: u32 = 0x88;
/// Magic caster virtual slot `0x40`.
const MAGIC_CASTER_SLOT_40: u32 = 0x40;
/// Magic caster virtual slot `0x48`.
const MAGIC_CASTER_SLOT_48: u32 = 0x48;
/// Reads the integer setting at `DEBUG_DRAW_SETTING` (`004503f0`).
const DEBUG_DRAW_LEVEL: u32 = 0x0045_03f0;
const DEBUG_DRAW_SETTING: u32 = 0x011d_f838;
/// Four-`float` constructor (`00414430`, thiscall, 4 floats).
const COLOR_CONSTRUCT: u32 = 0x0041_4430;
/// Rotation matrix times vector (`00524c40`: thiscall on the matrix, taking
/// an output vector and the input vector; returns the output).
const MATRIX_TRANSFORM: u32 = 0x0052_4c40;
/// Vector subtraction (`00439ef0`: thiscall on the minuend, taking an output
/// vector and the subtrahend; returns the output).
const VECTOR_SUBTRACT: u32 = 0x0043_9ef0;
/// Vector normalization in place (`004a0c10`).
const VECTOR_NORMALIZE: u32 = 0x004a_0c10;
/// Vector times scalar (`0045bb20`: thiscall on the vector, taking an
/// output vector and the scalar; returns the output).
const VECTOR_SCALE: u32 = 0x0045_bb20;
/// `BSTestObjects::CreateDirArrow` (Xbox PDB), cdecl: vector, colour.
const CREATE_DIR_ARROW: u32 = 0x00c5_9a50;
/// Sets the position of a debug object (`00440460`, takes the position).
const DEBUG_OBJECT_SET_POSITION: u32 = 0x0044_0460;
/// Reads a `float` setting (`00450410`, `this` = `DEBUG_DRAW_DURATION`).
const DEBUG_DRAW_DURATION_GETTER: u32 = 0x0045_0410;
const DEBUG_DRAW_DURATION: u32 = 0x011d_f754;
/// `TES::AddTempDebugObject` (Xbox PDB): the object and the seconds.
const TES_ADD_TEMP_DEBUG_OBJECT: u32 = 0x0045_8e20;
/// `800.0f` and `0.1f` (the scales applied to the arrow vector).
const ARROW_LENGTH_SCALE: u32 = 0x0102_2454;
const ARROW_WIDTH_SCALE: u32 = 0x0101_e2bc;
/// `0087b040`: thiscall on the object `GET_TES` returns, taking the actor
/// and the two flags.
const TES_QUEUE_ATTACK: u32 = 0x0087_b040;
/// Process virtual slot `0x460`: returns the address of a vector (the weapon
/// bone offset the debug arrow starts from).
const PROCESS_SLOT_460: u32 = 0x460;
/// Process virtual slot `0x448`.
const PROCESS_SLOT_448: u32 = 0x448;
/// `008bc240`: thiscall on the actor, taking one word.
const ACTOR_WORKER_8BC240: u32 = 0x008b_c240;
/// `005fbeb0`: thiscall on the base form, returns a word.
const FORM_GETTER_5FBEB0: u32 = 0x005f_beb0;
/// The integer setting passed to `008bc240` when slot `0x21c` is false.
const WORD_SETTING_3B4: u32 = 0x011c_d3b4;
/// Process virtual slot `0x274`.
const PROCESS_SLOT_274: u32 = 0x274;
/// `*(this + 0x18)` (`009611e0`).
const OBJECT_TYPE_FIELD: u32 = 0x0096_11e0;
/// `*(this + 4)` (`00726070`).
const NODE_NEXT: u32 = 0x0072_6070;
/// Actor test `008ae660` (takes a flag).
const ACTOR_TEST_8AE660: u32 = 0x008a_e660;
/// `0097f7d0`: thiscall on the combat controller, taking the target.
const CONTROLLER_WORKER_97F7D0: u32 = 0x0097_f7d0;
/// `Actor::CombatHit` (Xbox PDB), `00899cb0`.
const ACTOR_COMBAT_HIT: u32 = 0x0089_9cb0;
/// `BSSoundHandle::SetPosition` (Xbox PDB): the three coordinates.
const SOUND_HANDLE_SET_POSITION_XYZ: u32 = 0x00ad_8b60;
/// `BSSoundHandle::SetObjectToFollow` (Xbox PDB).
const SOUND_HANDLE_SET_OBJECT_TO_FOLLOW: u32 = 0x00ad_8f20;
/// Test on the weapon's actor record (`00406090`).
const RECORD_TEST_406090: u32 = 0x0040_6090;
/// `TESActorBaseData::GetFatigue` (Xbox PDB): a 16-bit value.
const ACTOR_BASE_DATA_GET_FATIGUE: u32 = 0x004a_8ae0;

// Translated from 00899200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Carries out the attack animation's hit on the actor's side.
///
/// With the task queue in use (`008c7aa0`) only forwards `(this, flag_a,
/// flag_b)` to `0087b040`. Otherwise: during a VATS action of this actor
/// uses up a shot of the action or casts the default magic object number 5
/// and counts the stat `0x15`; draws a debug arrow from the weapon bone to
/// the weapon position when the debug setting is above 1; tells the
/// acquire object (slot `0x448`); and runs `008bc240` with the form
/// word (or the setting) for the actor kind. With `flag_b` set it then
/// finds the target to hit (the acquire object's slot `0x274` reference when
/// slot `0x27c`'s object has the type number `0x2c`, otherwise
/// `fn_008990f0`), gives up for an actor whose slot `0x360` says to when
/// `008ae660` agrees, and either runs `Actor::CombatHit` on the target (with
/// the kind 1 for the animation groups `0x38`, `0x64`, `0x65` and 2 for
/// `0x3e`) or, with no target, plays the weapon's sound at the actor and
/// casts the weapon's actor-base spell when the animation matches. Ends by
/// telling the combat controller (slot `0x428`) with `0097f7d0`.
/// The compiler's exception-unwinding frame is not translated; `CombatHit`
/// is called by its address.
pub fn fn_00899200(e: &mut Engine, this: Ptr<Actor>, flag_a: u8, flag_b: u8) {
    if e.call(PICK_UP_GOES_TO_TASK_QUEUE, &args![]).bool() {
        let tes = e.call(GET_TES, &args![]).u32();
        e.call(TES_QUEUE_ATTACK, &args![tes, this, flag_a, flag_b]);
        return;
    }
    let caster = this.addr() + ACTOR_MAGIC_CASTER_OFFSET;
    if e.call(ENTRY_FORM, &args![VATS_OBJECT]).u32() == 4
        && this.addr() == e.global::<u32>(VATS_ACTOR)
    {
        let process = e.get(this, Actor::pCurrentProcess).addr();
        let weapon_entry = e
            .vcall(process, PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .u32();
        let weapon_form = if weapon_entry != 0 {
            let weapon_entry = e
                .vcall(process, PROCESS_SLOT_CURRENT_WEAPON, &args![])
                .u32();
            e.call(ENTRY_FORM, &args![weapon_entry]).u32()
        } else {
            0
        };
        let action = e.call(VATS_GET_CURRENT_ACTION, &args![VATS_OBJECT]).u32();
        if action != 0 {
            if weapon_form != 0 && e.call(WEAPON_BYTE_FLAG, &args![weapon_form]).bool() {
                let shots = e.mem.u8(action + 8);
                e.mem.set_u8(action + 8, shots.wrapping_sub(1));
            } else {
                let reference = e.mem.u32(action + 0xc);
                if e.mem.u8(action + 7) != 0
                    && !e.vcall(reference, 0x234, &args![]).bool()
                    && e.vcall(reference, 0x100, &args![]).bool()
                {
                    let object = e.call(GET_DEFAULT_OBJECT, &args![5u32]).u32();
                    if object != 0 {
                        let item = e
                            .call(
                                RTTI_DYNAMIC_CAST,
                                &args![
                                    object,
                                    0u32,
                                    RTTI_TYPE_TES_FORM,
                                    RTTI_TYPE_MAGIC_ITEM,
                                    0u32
                                ],
                            )
                            .u32();
                        e.vcall(caster, MAGIC_CASTER_SLOT_40, &args![item]);
                        let reference = e.mem.u32(action + 0xc);
                        let source = if reference != 0 { reference + 0x94 } else { 0 };
                        e.vcall(caster, MAGIC_CASTER_SLOT_48, &args![source]);
                        if e.call(MAGIC_CASTER_RELEASE_CAST, &args![caster, 0u32])
                            .bool()
                        {
                            e.call(MISC_STAT_INCREMENT, &args![0x15u32]);
                        }
                    }
                }
            }
        }
    }
    if e.call(DEBUG_DRAW_LEVEL, &args![DEBUG_DRAW_SETTING]).i32() > 1 {
        draw_hit_arrow(e, this);
    }
    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    e.vcall(acquire, PROCESS_SLOT_448, &args![1u32]);
    if e.vcall(this.addr(), ACTOR_SLOT_21C, &args![]).bool() {
        let form = base_form(e, this.cast());
        let word = e.call(FORM_GETTER_5FBEB0, &args![form]).u32();
        e.call(ACTOR_WORKER_8BC240, &args![this, word]);
    } else {
        let setting = e
            .call(SETTING_GET_INT_POINTER, &args![WORD_SETTING_3B4])
            .u32();
        let word = e.mem.u32(setting);
        e.call(ACTOR_WORKER_8BC240, &args![this, word]);
    }
    if flag_b == 0 {
        return;
    }

    let mut found = false;
    let mut hit_target = 0u32;
    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    if acquire != 0 {
        let holder = e.vcall(acquire, PROCESS_SLOT_27C, &args![]).u32();
        if holder != 0 && e.call(OBJECT_TYPE_FIELD, &args![holder]).u32() == 0x2c {
            let entry = e.vcall(acquire, PROCESS_SLOT_274, &args![]).u32();
            if entry != 0 {
                e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]);
                let next = e.call(NODE_NEXT, &args![entry]).u32();
                if next != 0 && e.vcall(next, ACTOR_SLOT_100, &args![]).bool() {
                    hit_target = next;
                    found = true;
                }
            }
        }
    }
    if !found {
        hit_target = fn_008990f0(e, this);
    }
    if hit_target == 0
        && e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
        && e.call(ACTOR_TEST_8AE660, &args![this, flag_a]).bool()
    {
        return;
    }
    if hit_target != 0 && e.call(ACTOR_TEST_8ACE90, &args![hit_target]).bool() {
        hit_target = 0;
    }
    let weapon_entry = if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(acquire, PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .u32()
    } else {
        0
    };
    let weapon_form = if weapon_entry != 0 {
        e.call(ENTRY_FORM, &args![weapon_entry]).u32()
    } else {
        0
    };
    if hit_target != 0 {
        let mut kind = 0u32;
        if weapon_form == 0 || e.call(WEAPON_TYPE, &args![weapon_form]).u32() == 0 {
            let process = e.get(this, Actor::pCurrentProcess).addr();
            let sequence = e.vcall(process, PROCESS_SLOT_ANIM_SEQUENCE, &args![]).u32();
            let transform = e
                .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                .u32();
            match e.call(ANIM_GROUP_GET_NUMBER, &args![transform]).u32() {
                0x38 | 0x64 | 0x65 => kind = 1,
                0x3e => kind = 2,
                _ => {}
            }
        }
        e.call(
            ACTOR_COMBAT_HIT,
            &args![this, hit_target, flag_a, 0u32, kind],
        );
    } else {
        let sound_set = if weapon_form != 0 {
            e.call(WEAPON_SOUND_SET, &args![weapon_form]).u32()
        } else {
            0
        };
        if sound_set != 0 {
            play_weapon_sound(e, this, sound_set);
        }
        cast_weapon_spell(e, this, caster);
    }
    let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
    if controller != 0 {
        e.call(CONTROLLER_WORKER_97F7D0, &args![controller, hit_target]);
    }
}

/// Copies the three words at `from` to `to`.
fn copy_vector(e: &mut Engine, from: u32, to: u32) {
    for word in 0..3 {
        let value = e.mem.u32(from + 4 * word);
        e.mem.set_u32(to + 4 * word, value);
    }
}

/// The debug drawing of `fn_00899200`: an arrow from the position of the
/// weapon bone towards the weapon position, in blue for an actor whose slot
/// `0x360` is true and red otherwise, shown for a number of seconds.
fn draw_hit_arrow(e: &mut Engine, this: Ptr<Actor>) {
    let locals = e.mem.alloc(0x100);
    let color = locals;
    let color_temp = locals + 0x10;
    let first = locals + 0x20;
    let second = locals + 0x30;
    let bone_temp = locals + 0x40;
    let weapon_temp = locals + 0x50;
    let transformed_temp = locals + 0x60;
    let difference_temp = locals + 0x70;
    let direction = locals + 0x80;
    let scaled = locals + 0x90;
    let scaled_again = locals + 0xa0;
    let zero = 0.0f32;
    let one = 1.0f32;
    e.call(COLOR_CONSTRUCT, &args![color, zero, zero, zero, zero]);
    let made = if e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool() {
        e.call(COLOR_CONSTRUCT, &args![color_temp, zero, zero, one, one])
    } else {
        e.call(COLOR_CONSTRUCT, &args![color_temp, one, zero, zero, one])
    }
    .u32();
    for word in 0..4 {
        let value = e.mem.u32(made + 4 * word);
        e.mem.set_u32(color + 4 * word, value);
    }
    // `first` and `second` are `NiPoint3` temporaries whose constructor
    // (`006815c0`) does nothing.
    let node = e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32();
    if node != 0 {
        let process = e.get(this, Actor::pCurrentProcess).addr();
        let bone = e.vcall(process, PROCESS_SLOT_460, &args![]).u32();
        let rotation = e.call(NODE_ROTATION, &args![node]).u32();
        let result = e
            .call(MATRIX_TRANSFORM, &args![rotation, bone_temp, bone])
            .u32();
        copy_vector(e, result, first);
        let position = e
            .call(ACTOR_GET_WEAPON_POSITION, &args![this, weapon_temp, 0u32])
            .u32();
        let rotation = e.call(NODE_ROTATION, &args![node]).u32();
        let result = e
            .call(
                MATRIX_TRANSFORM,
                &args![rotation, transformed_temp, position],
            )
            .u32();
        copy_vector(e, result, second);
    }
    let result = e
        .call(VECTOR_SUBTRACT, &args![second, difference_temp, first])
        .u32();
    copy_vector(e, result, direction);
    e.call(VECTOR_NORMALIZE, &args![direction]);
    let length_scale = e.global::<f32>(ARROW_LENGTH_SCALE);
    e.call(VECTOR_SCALE, &args![direction, scaled, length_scale]);
    let width_scale = e.global::<f32>(ARROW_WIDTH_SCALE);
    let arrow_vector = e
        .call(VECTOR_SCALE, &args![scaled, scaled_again, width_scale])
        .u32();
    let arrow = e.call(CREATE_DIR_ARROW, &args![arrow_vector, color]).u32();
    e.call(DEBUG_OBJECT_SET_POSITION, &args![arrow, second]);
    let seconds = e
        .call(DEBUG_DRAW_DURATION_GETTER, &args![DEBUG_DRAW_DURATION])
        .f32();
    let tes = e.global::<u32>(TES_SINGLETON);
    e.call(TES_ADD_TEMP_DEBUG_OBJECT, &args![tes, arrow, seconds]);
    e.mem.free(locals);
}

/// Plays the sound of the weapon's sound set (`sound_set`) at the actor's
/// position, following its node.
fn play_weapon_sound(e: &mut Engine, this: Ptr<Actor>, sound_set: u32) {
    let handle = e.mem.alloc(12);
    let file_name = e.call(SOUND_SET_FILE_NAME, &args![sound_set]).u32();
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    e.call(
        AUDIO_GET_SOUND_HANDLE_BY_FILENAME,
        &args![audio, handle, file_name, 0x102u32, sound_set],
    );
    let position = e.vcall(this.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
    let x = e.mem.u32(position);
    let y = e.mem.u32(position + 4);
    let z = e.mem.u32(position + 8);
    e.call(SOUND_HANDLE_SET_POSITION_XYZ, &args![handle, x, y, z]);
    let node = e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32();
    e.call(SOUND_HANDLE_SET_OBJECT_TO_FOLLOW, &args![handle, node]);
    e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
    e.call(SOUND_HANDLE_DESTROY, &args![handle]);
    e.mem.free(handle);
}

/// Casts the spell of the actor base form's record (`base + 0x70`, its first
/// entry plus `0x18`) when its test passes and the animation the actor plays
/// has the group the record names (or the record names `0xff`).
fn cast_weapon_spell(e: &mut Engine, this: Ptr<Actor>, caster: u32) {
    let base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
    let node = e.call(NODE_NEXT, &args![base + 0x70]).u32();
    let record = if node != 0 { node + 0x18 } else { 0 };
    if record == 0 || !e.call(RECORD_TEST_406090, &args![record + 0xc]).bool() {
        return;
    }
    let mut matched = false;
    let base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
    let group = u32::from(
        e.call(ACTOR_BASE_DATA_GET_FATIGUE, &args![base + 0x70])
            .u16(),
    );
    if group == 0xff {
        matched = true;
    } else if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        if e.vcall(acquire, PROCESS_SLOT_ANIM_SEQUENCE, &args![]).u32() != 0 {
            let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            let sequence = e.vcall(acquire, PROCESS_SLOT_ANIM_SEQUENCE, &args![]).u32();
            if e.call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                .u32()
                != 0
            {
                let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
                let sequence = e.vcall(acquire, PROCESS_SLOT_ANIM_SEQUENCE, &args![]).u32();
                let transform = e
                    .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
                    .u32();
                if e.call(ANIM_GROUP_GET_NUMBER, &args![transform]).u32() == group {
                    matched = true;
                }
            }
        }
    }
    if matched {
        e.vcall(caster, MAGIC_CASTER_SLOT_40, &args![record]);
        let source = if this.is_null() {
            0
        } else {
            this.addr() + 0x94
        };
        e.vcall(caster, MAGIC_CASTER_SLOT_48, &args![source]);
        e.call(MAGIC_CASTER_RELEASE_CAST, &args![caster, 0u32]);
    }
}
/// `Actor::HitMe` (Xbox PDB), `0089a760`: thiscall on the target, taking the
/// `HitData` and a kind byte.
const ACTOR_HIT_ME: u32 = 0x0089_a760;
/// Process-level test on an actor (`00437b90`).
const ACTOR_PROCESS_LEVEL_TEST_437B90: u32 = 0x0043_7b90;
/// `HitData::HitData` (Xbox PDB): thiscall on the memory, returns it.
const HIT_DATA_CONSTRUCT: u32 = 0x009b_4d90;
/// `NiPointer<HitData>` construction from a raw pointer (`008c1c30`):
/// thiscall on the 4-byte holder; stores the pointer and takes a reference.
const NI_POINTER_HIT_DATA_CONSTRUCT: u32 = 0x008c_1c30;
/// `NiPointer<HitData>::~NiPointer<HitData>` (Xbox PDB).
const NI_POINTER_HIT_DATA_DESTRUCT: u32 = 0x008c_1c60;
/// `NiPointer::operator->`: returns the pointer the holder (`this`) keeps
/// (`*this`).
const NI_POINTER_GET: u32 = 0x0055_9450;
/// `HitData::Copy` (Xbox PDB): thiscall on the destination, taking the
/// source.
const HIT_DATA_COPY: u32 = 0x009b_4ec0;
/// `HitData::InitializeHitData_ov2` (Xbox PDB), cdecl: the hit data, the
/// attacker, the target, the weapon entry, the attack flag byte and the
/// source reference.
const HIT_DATA_INITIALIZE: u32 = 0x009b_5120;
/// Form flag test (`00444d00`): thiscall on the reference, taking a mask.
const REFR_FLAG_TEST: u32 = 0x0044_4d00;
/// `00407e00`: thiscall on the hit data's critical effect holder, taking
/// two words (`0x3fc`, 0).
const HIT_DATA_CLEAR_EFFECT: u32 = 0x0040_7e00;
/// `00476c90`: thiscall on the source reference, taking a flag, 0 and the
/// attacker; returns the weapon object the projectile count is read from.
const SOURCE_WEAPON_LOOKUP: u32 = 0x0047_6c90;
/// `TESObjectWEAP::GetNumProjectiles` (Xbox PDB): thiscall, a byte result.
const WEAPON_GET_NUM_PROJECTILES: u32 = 0x0052_5b20;
/// Test on the base object of the source reference (`004fd3c0`).
const BASE_TEST_4FD3C0: u32 = 0x004f_d3c0;
/// `00885d70`: thiscall on the source reference; the result is in ST0.
const REFR_GETTER_885D70: u32 = 0x0088_5d70;
/// `HitData` flag test (`0058cba0`): thiscall on the hit data, taking a
/// flag number.
const HIT_DATA_FLAG_TEST: u32 = 0x0058_cba0;
/// `ImpactMixer::PlayWeaponBlock` (Xbox PDB), cdecl: the weapon type, the
/// weapon form, two positions of 3 words and a flag.
const IMPACT_MIXER_PLAY_WEAPON_BLOCK: u32 = 0x0083_97f0;
/// The weapon-hit sound call (`00837520`), cdecl, 9 words.
const IMPACT_MIXER_PLAY_WEAPON_HIT: u32 = 0x0083_7520;
/// The pointer read as the second word of the block sound call when the
/// target holds no weapon.
const IMPACT_BLOCK_DEFAULT_SOUND: u32 = 0x011c_a278;
/// `00476c90`'s companion: `ItemChange` number read (`004bd350`, ST0).
const ITEM_CHANGE_GETTER_4BD350: u32 = 0x004b_d350;
/// `Actor` virtual slot `0x3d8` (weapon entry, target, source reference, out
/// byte): returns a byte.
const ACTOR_SLOT_3D8: u32 = 0x3d8;
/// `Actor` virtual slot `0x3c8` (weapon entry, `float`, flag): returns a
/// byte.
const ACTOR_SLOT_3C8: u32 = 0x3c8;
/// `Actor` virtual slots `0x32c` (getter), `0x328` (setter) and `0x330`.
const ACTOR_SLOT_32C: u32 = 0x32c;
const ACTOR_SLOT_328: u32 = 0x328;
const ACTOR_SLOT_330: u32 = 0x330;
/// `Actor` virtual slot `0x4c0` (hit damage, blocked fraction, target,
/// source reference).
const ACTOR_SLOT_4C0: u32 = 0x4c0;
/// `00444d00` mask used for the critical-hit check.
const REFR_FLAG_NO_CRIT: u32 = 0x0002_0000;
/// `TESPackage::GetUseWeaponPackageData` (Xbox PDB).
const PACKAGE_GET_USE_WEAPON_PACKAGE_DATA: u32 = 0x0067_58f0;
/// Test on that data (`00524cd0`).
const USE_WEAPON_DATA_TEST: u32 = 0x0052_4cd0;
/// `00407e00`'s mask word.
const HIT_DATA_EFFECT_MASK: u32 = 0x3fc;
/// Player worker (`009627a0`, thiscall on the player).
const PLAYER_WORKER_9627A0: u32 = 0x0096_27a0;

// `HitData` (Xbox PDB, 0x64 bytes) field offsets used here.
const HIT_SOURCE_REF: u32 = 0x08;
const HIT_HEALTH_DAMAGE: u32 = 0x14;
const HIT_TOTAL_DAMAGE: u32 = 0x18;
const HIT_FATIGUE_DAMAGE: u32 = 0x1c;
const HIT_TARGETED_LIMB_DAMAGE: u32 = 0x20;
const HIT_PERCENT_BLOCKED: u32 = 0x24;
const HIT_ARMOR_DAMAGE: u32 = 0x28;
const HIT_DAMAGE_TO_WEAPON: u32 = 0x2c;
const HIT_LOCATION: u32 = 0x38;
const HIT_DIRECTION: u32 = 0x44;
const HIT_CRITICAL_EFFECT: u32 = 0x50;
const HIT_ATTACK_SKILL: u32 = 0x0c;

/// The `HitData` pointer a holder keeps (through `operator->`, `00559450`).
fn hit_data_of(e: &mut Engine, holder: u32) -> u32 {
    e.call(NI_POINTER_GET, &args![holder]).u32()
}

/// Divides the `float` at `address` by `divisor`.
fn divide_float(e: &mut Engine, address: u32, divisor: u32) {
    let value = e.mem.f32(address) as f64 / divisor as f64;
    e.mem.set_f32(address, value as f32);
}

/// Multiplies the `float` at `address` by `factor`.
fn multiply_float(e: &mut Engine, address: u32, factor: f32) {
    let value = e.mem.f32(address) as f64 * factor as f64;
    e.mem.set_f32(address, value as f32);
}

/// The three words of the position `slot 0x1f4` of `actor` returns.
fn position_words(e: &mut Engine, actor: u32) -> [u32; 3] {
    let position = e.vcall(actor, ACTOR_SLOT_POSITION, &args![]).u32();
    [
        e.mem.u32(position),
        e.mem.u32(position + 4),
        e.mem.u32(position + 8),
    ]
}

// Translated from 00899cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::CombatHit` (Xbox PDB): the attack `this` makes lands on `target`.
///
/// Gives up for a missing target, `008ace90` and `00437b90`. Looks up the
/// weapon entry/form/type, asks slot `0x3d8` whether the hit connects, builds
/// the `HitData` (a new `0x64`-byte object kept in an `NiPointer`): during a
/// VATS action the action's data is copied in and the freshly initialized
/// one is merged over it, otherwise it is just initialized
/// (`HitData::InitializeHitData_ov2`). Zeroes the damage of an attack whose
/// source reference has the form flag `0x20000`, splits it among the
/// projectiles of the weapon and applies the armor-piercing correction
/// (`004fd3c0`). Updates the actor's hit counter (slots `0x32c`, `0x328`) and
/// slot `0x330`; plays the block or hit sounds (flag 1 of the hit data
/// chooses between them), casts a weapon effect when its threshold is
/// passed, gives up the weapon for a destroyed one (slot `0x3c8`), clears
/// the damage for a use-weapon package and finally lets `target` take the hit
/// (`Actor::HitMe`) unless this actor's combat controller does not have the
/// target as a combat target.
/// The compiler's exception-unwinding frame is not translated; `HitMe` is
/// called by its address.
pub fn actor_combat_hit(
    e: &mut Engine,
    this: Ptr<Actor>,
    target: Ptr<Actor>,
    flag: u8,
    source_ref: Ptr,
    kind: u8,
) {
    if target.is_null()
        || e.call(ACTOR_TEST_8ACE90, &args![target]).bool()
        || e.call(ACTOR_PROCESS_LEVEL_TEST_437B90, &args![target])
            .bool()
    {
        return;
    }
    let player_address = e.global::<u32>(PLAYER_CHARACTER);
    let mut not_a_target = false;
    if this.addr() != player_address && e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() != 0 {
        let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
        if !e
            .call(
                COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET,
                &args![controller, target],
            )
            .bool()
        {
            not_a_target = true;
        }
    }
    let weapon_entry = if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(acquire, PROCESS_SLOT_CURRENT_WEAPON, &args![])
            .u32()
    } else {
        0
    };
    let weapon_form = if weapon_entry != 0 {
        e.call(ENTRY_FORM, &args![weapon_entry]).u32()
    } else {
        0
    };
    let weapon_type = if weapon_form != 0 {
        e.call(WEAPON_TYPE, &args![weapon_form]).u32()
    } else {
        0
    };
    let out_flag = e.mem.alloc(4);
    let connects = e
        .vcall(
            this.addr(),
            ACTOR_SLOT_3D8,
            &args![weapon_entry, target, source_ref, out_flag],
        )
        .u8();
    if e.mem.u8(out_flag) != 0 && e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool() {
        e.call(PLAYER_WORKER_9627A0, &args![player_address]);
    }
    e.mem.free(out_flag);

    let block = e.call(OPERATOR_NEW, &args![0x64u32]).u32();
    let new_hit_data = if block != 0 {
        e.call(HIT_DATA_CONSTRUCT, &args![block]).u32()
    } else {
        0
    };
    let holder = e.mem.alloc(4);
    e.call(NI_POINTER_HIT_DATA_CONSTRUCT, &args![holder, new_hit_data]);

    let action = if e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
        && e.call(ENTRY_FORM, &args![VATS_OBJECT]).u32() == 4
    {
        e.call(VATS_GET_CURRENT_ACTION, &args![VATS_OBJECT]).u32()
    } else {
        0
    };
    if action != 0 {
        let saved = e.mem.u32(action + 0x14);
        let hit_data = hit_data_of(e, holder);
        e.call(HIT_DATA_COPY, &args![hit_data, saved]);
        let fresh = e.mem.alloc(0x64);
        e.call(HIT_DATA_CONSTRUCT, &args![fresh]);
        e.call(
            HIT_DATA_INITIALIZE,
            &args![fresh, this, target, weapon_entry, flag, source_ref],
        );
        let hit_data = hit_data_of(e, holder);
        for word in 0..3 {
            let value = e.mem.u32(fresh + HIT_DIRECTION + 4 * word);
            e.mem.set_u32(hit_data + HIT_DIRECTION + 4 * word, value);
        }
        let hit_data = hit_data_of(e, holder);
        for word in 0..3 {
            let value = e.mem.u32(fresh + HIT_LOCATION + 4 * word);
            e.mem.set_u32(hit_data + HIT_LOCATION + 4 * word, value);
        }
        if e.mem.u8(action + 4) == 0 {
            for offset in [
                HIT_HEALTH_DAMAGE,
                HIT_TOTAL_DAMAGE,
                HIT_FATIGUE_DAMAGE,
                HIT_ARMOR_DAMAGE,
                HIT_TARGETED_LIMB_DAMAGE,
            ] {
                let hit_data = hit_data_of(e, holder);
                let value = e.mem.f32(fresh + offset);
                e.mem.set_f32(hit_data + offset, value);
            }
        }
        if !source_ref.is_null()
            && e.call(REFR_FLAG_TEST, &args![source_ref, REFR_FLAG_NO_CRIT])
                .bool()
        {
            let hit_data = hit_data_of(e, holder);
            e.mem.set_u32(hit_data + HIT_CRITICAL_EFFECT, 0);
            for offset in [
                HIT_HEALTH_DAMAGE,
                HIT_TOTAL_DAMAGE,
                HIT_FATIGUE_DAMAGE,
                HIT_DAMAGE_TO_WEAPON,
                HIT_ARMOR_DAMAGE,
                HIT_TARGETED_LIMB_DAMAGE,
            ] {
                let hit_data = hit_data_of(e, holder);
                e.mem.set_f32(hit_data + offset, 0.0);
            }
            let hit_data = hit_data_of(e, holder);
            e.call(
                HIT_DATA_CLEAR_EFFECT,
                &args![hit_data, HIT_DATA_EFFECT_MASK, 0u32],
            );
        }
        let hit_data = hit_data_of(e, holder);
        if e.mem.u32(hit_data + HIT_SOURCE_REF) == 0 && !source_ref.is_null() {
            let hit_data = hit_data_of(e, holder);
            e.mem.set_u32(hit_data + HIT_SOURCE_REF, source_ref.addr());
        }
        let hit_data = hit_data_of(e, holder);
        if !source_ref.is_null() && e.mem.f32(hit_data + HIT_HEALTH_DAMAGE) as f64 != 0.0 {
            let mut has_effect = 0u32;
            if weapon_entry != 0
                && e.call(
                    ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE,
                    &args![weapon_entry, 0xcu32],
                )
                .bool()
            {
                has_effect = 1;
            }
            let weapon_object = e
                .call(
                    SOURCE_WEAPON_LOOKUP,
                    &args![source_ref, has_effect, 0u32, this],
                )
                .u32();
            let projectiles = e
                .call(WEAPON_GET_NUM_PROJECTILES, &args![weapon_object])
                .u8() as u32;
            if projectiles as i32 > 1 {
                for offset in [
                    HIT_HEALTH_DAMAGE,
                    HIT_TOTAL_DAMAGE,
                    HIT_TARGETED_LIMB_DAMAGE,
                ] {
                    let hit_data = hit_data_of(e, holder);
                    divide_float(e, hit_data + offset, projectiles);
                }
            }
            let base = e.call(ACTOR_BASE_FORM, &args![source_ref]).u32();
            if e.call(BASE_TEST_4FD3C0, &args![base]).u32() != 0 {
                let hit_data = hit_data_of(e, holder);
                let reduction = e.call(REFR_GETTER_885D70, &args![source_ref]).f64();
                let numerator = (e.mem.f32(hit_data + HIT_HEALTH_DAMAGE) as f64 - reduction) as f32;
                let hit_data = hit_data_of(e, holder);
                let ratio = numerator as f64 / e.mem.f32(hit_data + HIT_HEALTH_DAMAGE) as f64;
                let factor = (1.0 - ratio) as f32;
                for offset in [
                    HIT_HEALTH_DAMAGE,
                    HIT_TOTAL_DAMAGE,
                    HIT_TARGETED_LIMB_DAMAGE,
                ] {
                    let hit_data = hit_data_of(e, holder);
                    multiply_float(e, hit_data + offset, factor);
                }
            }
        }
        hit_data_of(e, holder);
    } else {
        let hit_data = hit_data_of(e, holder);
        e.call(
            HIT_DATA_INITIALIZE,
            &args![hit_data, this, target, weapon_entry, flag, source_ref],
        );
    }

    let counter = e.vcall(this.addr(), ACTOR_SLOT_32C, &args![]).u32();
    e.vcall(this.addr(), ACTOR_SLOT_328, &args![counter.wrapping_add(1)]);
    let hit_data = hit_data_of(e, holder);
    let attack_skill = e.mem.u32(hit_data + HIT_ATTACK_SKILL);
    e.vcall(this.addr(), ACTOR_SLOT_330, &args![attack_skill]);

    let hit_data = hit_data_of(e, holder);
    if e.call(HIT_DATA_FLAG_TEST, &args![hit_data, 1u32]).bool() {
        let hit_data = hit_data_of(e, holder);
        let health_damage = e.mem.f32(hit_data + HIT_HEALTH_DAMAGE);
        let hit_data = hit_data_of(e, holder);
        let percent_blocked = e.mem.f32(hit_data + HIT_PERCENT_BLOCKED);
        e.vcall(
            this.addr(),
            ACTOR_SLOT_4C0,
            &args![health_damage, percent_blocked, target, source_ref],
        );
        let hit_data = hit_data_of(e, holder);
        if e.call(HIT_DATA_FLAG_TEST, &args![hit_data, 2u32]).bool() {
            let target_process = e.get(target, Actor::pCurrentProcess).addr();
            let target_weapon = e
                .vcall(target_process, PROCESS_SLOT_CURRENT_WEAPON, &args![])
                .u32();
            if target_weapon != 0 {
                let target_position = position_words(e, target.addr());
                let this_position = position_words(e, this.addr());
                let target_process = e.get(target, Actor::pCurrentProcess).addr();
                let target_weapon = e
                    .vcall(target_process, PROCESS_SLOT_CURRENT_WEAPON, &args![])
                    .u32();
                let target_form = e.call(ENTRY_FORM, &args![target_weapon]).u32();
                e.call(
                    IMPACT_MIXER_PLAY_WEAPON_BLOCK,
                    &args![
                        weapon_type,
                        target_form,
                        this_position[0],
                        this_position[1],
                        this_position[2],
                        target_position[0],
                        target_position[1],
                        target_position[2],
                        u32::from(connects)
                    ],
                );
            } else {
                let target_position = position_words(e, target.addr());
                let this_position = position_words(e, this.addr());
                let default_sound = e.global::<u32>(IMPACT_BLOCK_DEFAULT_SOUND);
                e.call(
                    IMPACT_MIXER_PLAY_WEAPON_BLOCK,
                    &args![
                        weapon_type,
                        default_sound,
                        this_position[0],
                        this_position[1],
                        this_position[2],
                        target_position[0],
                        target_position[1],
                        target_position[2],
                        u32::from(connects)
                    ],
                );
            }
        } else {
            let target_position = position_words(e, target.addr());
            let this_position = position_words(e, this.addr());
            e.call(
                IMPACT_MIXER_PLAY_WEAPON_BLOCK,
                &args![
                    weapon_type,
                    0u32,
                    this_position[0],
                    this_position[1],
                    this_position[2],
                    target_position[0],
                    target_position[1],
                    target_position[2],
                    u32::from(connects == 0)
                ],
            );
        }
    } else {
        let hit_type = if weapon_form != 0 {
            e.call(WEAPON_TYPE, &args![weapon_form]).u32()
        } else {
            0xffff_ffff
        };
        let hit_data = hit_data_of(e, holder);
        let health_damage = e.mem.f32(hit_data + HIT_HEALTH_DAMAGE);
        e.call(
            IMPACT_MIXER_PLAY_WEAPON_HIT,
            &args![
                this,
                health_damage,
                0.0f32,
                target,
                hit_type,
                0xffff_ffffu32,
                0xffff_ffffu32,
                0u32,
                u32::from(connects)
            ],
        );
    }

    if weapon_form != 0 && weapon_entry != 0 {
        let mut spell = 0u32;
        if spell == 0 {
            let node = e.call(NODE_NEXT, &args![weapon_form + 0x74]).u32();
            spell = if node != 0 { node + 0x18 } else { 0 };
        }
        if spell == 0 {
            let poison = e.call(ITEM_CHANGE_GET_POISON, &args![weapon_entry]).u32();
            spell = if poison != 0 { poison + 0x30 } else { 0 };
        }
        if spell != 0 && connects != 0 && source_ref.is_null() {
            let threshold = e
                .call(ITEM_CHANGE_GETTER_4BD350, &args![weapon_entry])
                .f64();
            let value = e.vcall(spell + 0xc, 8, &args![this]).f64();
            if value > threshold {
                let process = e.get(this, Actor::pCurrentProcess).addr();
                e.vcall(process, PROCESS_SLOT_580, &args![1u32, 0u32, 0u32]);
            }
        }
    }
    if weapon_entry != 0
        && weapon_form != 0
        && source_ref.is_null()
        && e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
        && !e.call(FORM_FLAG_100_BIT_5, &args![weapon_form]).bool()
    {
        let hit_data = hit_data_of(e, holder);
        let damage_to_weapon = e.mem.f32(hit_data + HIT_DAMAGE_TO_WEAPON);
        // The result clears a local that nothing reads afterwards.
        e.vcall(
            this.addr(),
            ACTOR_SLOT_3C8,
            &args![weapon_entry, damage_to_weapon, 0u32],
        );
    }
    let package = e.call(GET_CURRENT_PACKAGE, &args![this]).u32();
    if package != 0 {
        let data = e
            .call(PACKAGE_GET_USE_WEAPON_PACKAGE_DATA, &args![package])
            .u32();
        if data != 0 && e.call(USE_WEAPON_DATA_TEST, &args![data]).bool() {
            let hit_data = hit_data_of(e, holder);
            e.mem.set_f32(hit_data + HIT_HEALTH_DAMAGE, 0.0);
        }
    }
    if !not_a_target {
        let hit_data = hit_data_of(e, holder);
        e.call(ACTOR_HIT_ME, &args![target, hit_data, kind]);
    }
    e.call(NI_POINTER_HIT_DATA_DESTRUCT, &args![holder]);
    e.mem.free(holder);
}
/// `Actor::GetCurrentEditorPackage` (Xbox PDB).
const ACTOR_GET_CURRENT_EDITOR_PACKAGE: u32 = 0x0088_14b0;
/// `ExtraDataList::GetWeaponAttackSound` (Xbox PDB): thiscall on the extra
/// data list, taking the sound handle to fill.
const EXTRA_GET_WEAPON_ATTACK_SOUND: u32 = 0x0041_8a00;
/// `ExtraDataList::SetWeaponAttackSound` (Xbox PDB).
const EXTRA_SET_WEAPON_ATTACK_SOUND: u32 = 0x0041_a540;
/// `ExtraDataList::GetWeaponIdleSound` (Xbox PDB).
const EXTRA_GET_WEAPON_IDLE_SOUND: u32 = 0x0041_89c0;
/// `ExtraDataList::SetWeaponIdleSound` (Xbox PDB).
const EXTRA_SET_WEAPON_IDLE_SOUND: u32 = 0x0041_a3e0;
/// `ExtraDataList::GetCreatureAwakeSound` (Xbox PDB).
const EXTRA_GET_CREATURE_AWAKE_SOUND: u32 = 0x0041_8940;
/// `ExtraDataList::SetCreatureAwakeSound` (Xbox PDB).
const EXTRA_SET_CREATURE_AWAKE_SOUND: u32 = 0x0041_a090;
/// `BSSoundHandle::IsPlaying` (Xbox PDB).
const SOUND_HANDLE_IS_PLAYING: u32 = 0x00ad_8930;
/// `BSSoundHandle::FadeOutAndRelease` (Xbox PDB): the fade time in
/// milliseconds.
const SOUND_HANDLE_FADE_OUT_AND_RELEASE: u32 = 0x00ad_8da0;
/// `BSSoundHandle::Release` (Xbox PDB).
const SOUND_HANDLE_RELEASE: u32 = 0x00ad_8d10;
/// `BSSoundHandle::Stop` (Xbox PDB).
const SOUND_HANDLE_STOP: u32 = 0x00ad_88f0;
/// `BSAudio::GetSoundHandleByNumericID` (Xbox PDB): thiscall on the audio
/// object, taking the output handle, the id and the flags.
const AUDIO_GET_SOUND_HANDLE_BY_NUMERIC_ID: u32 = 0x00ad_73b0;
/// `BSAudio::GetSoundHandleByName` (Xbox PDB): the output handle, the name
/// and the flags; returns the handle.
const AUDIO_GET_SOUND_HANDLE_BY_NAME: u32 = 0x00ad_7550;
/// `"MUSDeath"`.
const DEATH_MUSIC_NAME: u32 = 0x0108_4d00;
/// `MobileObject::StopCurrentDialogue` (Xbox PDB).
const MOBILE_OBJECT_STOP_CURRENT_DIALOGUE: u32 = 0x0093_4250;
/// `bhkRigidBody::SetFixed` (Xbox PDB), cdecl: the body and two flags.
const BHK_RIGID_BODY_SET_FIXED: u32 = 0x00c8_f210;
/// `bhkRagdollController::DisableRagdollAnim` (Xbox PDB): thiscall on the
/// controller, taking a flag.
const BHK_RAGDOLL_DISABLE_RAGDOLL_ANIM: u32 = 0x00c7_c150;
/// Pointer to the byte value of a setting (`00408d60`, `this` = the
/// setting).
const SETTING_GET_BOOL_POINTER: u32 = 0x0040_8d60;
/// The `bool` setting that, with `GetEssential`, keeps an actor from dying.
const ESSENTIAL_SETTING: u32 = 0x011e_0888;
/// The `float` setting `Kill` multiplies the base health by for the actors
/// of process level 3 and 5.
const LOW_LEVEL_HEALTH_SETTING: u32 = 0x011d_0144;
/// The `float` setting passed to process slot `0xe8` for an essential
/// actor.
const ESSENTIAL_DOWN_TIMER_SETTING: u32 = 0x011d_0700;
/// `Actor::ClearInCombat` (Xbox PDB): takes a flag.
const ACTOR_CLEAR_IN_COMBAT: u32 = 0x008a_08e0;
/// `Actor::SetHavokWeapon` (Xbox PDB).
const ACTOR_SET_HAVOK_WEAPON: u32 = 0x008a_5eb0;
/// `Actor::RestoreFullHealthAndConditions` (Xbox PDB).
const ACTOR_RESTORE_FULL_HEALTH_AND_CONDITIONS: u32 = 0x008a_0960;
/// `MagicCaster::InterruptCast` (Xbox PDB), `this` = the caster at `+0x88`.
const MAGIC_CASTER_INTERRUPT_CAST: u32 = 0x0081_5b00;
/// `TESObjectREFR::SetMarkerUsed` (Xbox PDB): thiscall on the marker with
/// the index and a flag.
const REFR_SET_MARKER_USED: u32 = 0x0056_8020;
/// The CRT's `sprintf` (`00ec623a`), cdecl.
const SPRINTF: u32 = 0x00ec_623a;
/// `"%s %s"`.
const TWO_STRINGS_FORMAT: u32 = 0x0101_2058;
/// `"Interface\Icons\Message Icons\glow_message_vaultboy_surprised.dds"`.
const MESSAGE_ICON_SURPRISED: u32 = 0x0104_9638;
/// The setting object whose string follows the actor's name in the message
/// shown for an essential actor that goes down.
const DOWN_MESSAGE_SETTING: u32 = 0x011d_1e24;
/// String of a setting (`00403df0`, `this` = the setting).
const SETTING_GET_STRING: u32 = 0x0040_3df0;
/// `ProcessLists` worker taking the actor (`0096e2f0`).
const PROCESS_LISTS_WORKER_96E2F0: u32 = 0x0096_e2f0;
/// `TESObjectREFR::IsPartofEvilFaction` (Xbox PDB).
const REFR_IS_PART_OF_EVIL_FACTION: u32 = 0x0056_78a0;
/// `PlayerCharacter::SetIsAMurderer` (Xbox PDB).
const PLAYER_SET_IS_A_MURDERER: u32 = 0x0095_3f80;
/// `TESActorBaseData::GetAlignmentForKarma` (Xbox PDB), cdecl, a `float`.
const GET_ALIGNMENT_FOR_KARMA: u32 = 0x0047_e040;
/// `Actor::HasFactionThatCaresAboutCrime` (Xbox PDB).
const ACTOR_HAS_FACTION_THAT_CARES_ABOUT_CRIME: u32 = 0x008b_8490;
/// `PlayerCharacter::RewardKarma` (Xbox PDB): thiscall on the player.
const PLAYER_REWARD_KARMA: u32 = 0x0094_fd30;
/// `float` settings of the karma reward: the default, the one for
/// creatures and the ones per alignment (indexed by the alignment number;
/// entry 1 is the default).
const KARMA_REWARD_SETTING: u32 = 0x011c_df6c;
const KARMA_REWARD_CREATURE_SETTING: u32 = 0x011c_d164;
const KARMA_REWARD_ALIGNMENT_SETTINGS: [u32; 5] = [
    0x011c_d7a4,
    KARMA_REWARD_SETTING,
    0x011c_ded4,
    0x011c_dca4,
    0x011c_d508,
];
/// Test on the player (`004997b0`).
const PLAYER_TEST_4997B0: u32 = 0x0049_97b0;
/// `PlayerCharacter::IsPlayerDetectedByNonTeammates` (Xbox PDB): thiscall on
/// the player, taking an actor.
const PLAYER_IS_DETECTED_BY_NON_TEAMMATES: u32 = 0x0096_7160;
/// Test on a weapon form (`006450c0`).
const WEAPON_FORM_TEST_6450C0: u32 = 0x0064_50c0;
/// `00525620`: thiscall on a weapon form, taking two flags.
const WEAPON_FORM_NUMBER: u32 = 0x0052_5620;
/// The integer setting that number is compared with.
const WEAPON_NUMBER_SETTING: u32 = 0x011c_d3b4;
/// Test of the tracked damage against the health (`00670700`, cdecl, two
/// `float`s).
const TRACKED_DAMAGE_TEST: u32 = 0x0067_0700;
/// `0087f9f0`: thiscall on the actor, a 16-bit result.
const ACTOR_GETTER_87F9F0: u32 = 0x0087_f9f0;
/// `ExperiencePoints::GetExperiencePoints` (Xbox PDB), cdecl: a flag and a
/// 16-bit number; returns an integer.
const GET_EXPERIENCE_POINTS: u32 = 0x0067_05b0;
/// `005be4d0`: thiscall on the player, returns an integer.
const PLAYER_INT_5BE4D0: u32 = 0x005b_e4d0;
/// `00648c80`: cdecl, a `float` and an integer; returns a `float`.
const SCALE_EXPERIENCE: u32 = 0x0064_8c80;
/// `00406ce0`: cdecl, a `float`; returns a `float`.
const FLOAT_ROUND: u32 = 0x0040_6ce0;
/// Player virtual slot `0x488` (takes the amount).
const PLAYER_SLOT_488: u32 = 0x488;
/// `Actor` virtual slot `0x1a0` (takes a flag).
const ACTOR_SLOT_1A0: u32 = 0x1a0;
/// `Actor` virtual slot `0x2a0`.
const ACTOR_SLOT_2A0: u32 = 0x2a0;
/// `Actor::GetHeading` (Xbox PDB), `Actor` virtual slot `0x2bc` (takes a
/// flag; the result is a `float`).
const ACTOR_SLOT_GET_HEADING: u32 = 0x2bc;
/// `Actor` virtual slot `0x38c` (a bool).
const ACTOR_SLOT_38C: u32 = 0x38c;
/// `Actor` virtual slot `0x448` (a bool).
const ACTOR_SLOT_448: u32 = 0x448;
/// Process virtual slots used by `Kill`: `GetIsAggressor` (`0x1c4`),
/// `ClearMuzzleFlash` (`0x6c4`), `RemoveAllQueuedItems` (`0x180`),
/// `SetGreetingFlag` (`0x310`), `GetCurrentFurniture` (`0x4c8`),
/// `GetCurrentFurnitureIndex` (`0x4d0`), `SetBreathTimer` (`0x2fc`),
/// `GetTrackedDamage` (`0x730`), `ClearAllHeadTrackTargets` (`0x660`),
/// `SetDeathTime` (`0x6a0`), `SetGreetingTimer` (`0x4b4`) and
/// `GetKnockState` (`0x40c`) (Xbox PDB names of the `HighProcess` vtable).
const PROCESS_SLOT_1C4: u32 = 0x1c4;
const PROCESS_SLOT_6C4: u32 = 0x6c4;
const PROCESS_SLOT_180: u32 = 0x180;
const PROCESS_SLOT_310: u32 = 0x310;
const PROCESS_SLOT_4C8: u32 = 0x4c8;
const PROCESS_SLOT_4D0: u32 = 0x4d0;
const PROCESS_SLOT_2FC: u32 = 0x2fc;
const PROCESS_SLOT_730: u32 = 0x730;
const PROCESS_SLOT_660: u32 = 0x660;
const PROCESS_SLOT_6A0: u32 = 0x6a0;
const PROCESS_SLOT_4B4: u32 = 0x4b4;
const PROCESS_SLOT_40C: u32 = 0x40c;
/// Static byte read (`00525420`: a global flag).
const GLOBAL_FLAG_525420: u32 = 0x0052_5420;
/// `TESCreature::PickCreatureSound` (Xbox PDB): thiscall on the creature,
/// taking the sound type.
const CREATURE_PICK_CREATURE_SOUND: u32 = 0x005f_92d0;
/// The numeric id of a sound form (`0084e3a0`; also used for the ids in the
/// death report).
const FORM_NUMERIC_ID: u32 = 0x0084_e3a0;
/// `FalloutRadio::PipboyRadioEnable` (Xbox PDB), cdecl.
const PIPBOY_RADIO_ENABLE: u32 = 0x0083_24e0;
/// `00830680`: cdecl, `1000, 0`.
const SOUND_FADE_WORKER: u32 = 0x0083_0680;
/// `VATS::QuitVATSPlayback` (Xbox PDB): thiscall on `VATS`, two flags.
const VATS_QUIT_PLAYBACK: u32 = 0x009c_8950;
/// `Interface::IsTopMenuID` (Xbox PDB), cdecl.
const INTERFACE_IS_TOP_MENU_ID: u32 = 0x0070_2450;
/// `SurgeryMenu::Close` (Xbox PDB).
const SURGERY_MENU_CLOSE: u32 = 0x007e_1980;
/// `Actor::GetFaceAnimationData` (Xbox PDB).
const ACTOR_GET_FACE_ANIMATION_DATA: u32 = 0x008a_dcb0;
/// `009ca1f0`: thiscall on `VATS`.
const VATS_WORKER_9CA1F0: u32 = 0x009c_a1f0;
/// `0080ced0`: cdecl, the actor and a number; returns a byte.
const ACTOR_GETTER_80CED0: u32 = 0x0080_ced0;
/// `00867e30`: thiscall on `TIME_STAMP_OBJECT`, an unsigned result.
const TIME_STAMP_867E30: u32 = 0x0086_7e30;
const TIME_STAMP_OBJECT: u32 = 0x011d_e7b8;
/// `00459060`: thiscall on `TES`, taking a form and `1`.
const TES_WORKER_459060: u32 = 0x0045_9060;
/// `ExtraDataList::RemoveTrespassPackage` (Xbox PDB).
const EXTRA_REMOVE_TRESPASS_PACKAGE: u32 = 0x0041_cda0;
/// `0043fcd0`: the collision object of the actor (the base function of
/// `Actor` slot `0x1d0`).
const ACTOR_COLLISION_OBJECT: u32 = 0x0043_fcd0;
/// Four-`float` constructor (`005532a0`).
const VECTOR4_CONSTRUCT: u32 = 0x0055_32a0;
/// `bhkCharacterProxy::GetLinearVelocity` (Xbox PDB): thiscall, takes the
/// output vector.
const BHK_CHARACTER_PROXY_GET_LINEAR_VELOCITY: u32 = 0x0066_ca00;
/// Vector assignment (`004a3e00`, cdecl: destination, source).
const VECTOR_ASSIGN: u32 = 0x004a_3e00;
/// `MobileObject::SetChaseBip` (Xbox PDB): takes a flag.
const MOBILE_OBJECT_SET_CHASE_BIP: u32 = 0x0093_1fb0;
/// The `float` setting passed to process slot `0x4b4`.
const GREETING_TIMER_SETTING: u32 = 0x011c_cf7c;
/// Test on the actor (`0087eeb0`).
const ACTOR_TEST_87EEB0: u32 = 0x0087_eeb0;
/// `Actor` helper (`0087eef0`, thiscall on the actor).
const ACTOR_WORKER_87EEF0: u32 = 0x0087_eef0;
/// The three `float`s at `0x11f426c`: the default impulse direction.
const DEFAULT_DIRECTION: u32 = 0x011f_426c;
/// `00630b40`: thiscall on a vector, taking an output; returns an object.
const VECTOR_TRANSFORM_630B40: u32 = 0x0063_0b40;
/// `004586d0`: thiscall on that object; the result is in ST0.
const VECTOR_LENGTH_4586D0: u32 = 0x0045_86d0;
/// `00458620`: cdecl, output vector and input vector.
const VECTOR_COPY_458620: u32 = 0x0045_8620;
/// `004a0c90`: thiscall on a vector, taking a `float`.
const VECTOR_ROTATE_4A0C90: u32 = 0x004a_0c90;
/// `004b4500`: thiscall on a vector, taking an output and a vector;
/// returns the output.
const VECTOR_MULTIPLY_4B4500: u32 = 0x004b_4500;
/// A constant vector at `0x11a9478`.
const UNIT_VECTOR: u32 = 0x011a_9478;
/// `bhkCharacterController::GetPosition` (Xbox PDB, `00812b00`): thiscall,
/// takes the output vector.
const BHK_CHARACTER_CONTROLLER_GET_POSITION: u32 = 0x0081_2b00;
/// Three-`float` constructors (`0043d410` and `00416870`).
const VECTOR3_CONSTRUCT_43D410: u32 = 0x0043_d410;
const VECTOR3_CONSTRUCT_416870: u32 = 0x0041_6870;
/// `00a59c60`: thiscall on the collision object, taking a vector.
const COLLISION_OBJECT_SET_ANGULAR_VELOCITY: u32 = 0x00a5_9c60;
/// `00439180`: thiscall on a vector, taking a scalar.
const VECTOR_SCALE_IN_PLACE: u32 = 0x0043_9180;
/// The scalar setting multiplying the impulse (`0x11cf724`).
const DEATH_IMPULSE_SETTING: u32 = 0x011c_f724;
/// `TESHavokUtilities::AddVelocity` (Xbox PDB), cdecl: body, vector, flag.
const TES_HAVOK_ADD_VELOCITY: u32 = 0x0062_b8d0;
/// `0062b930`: cdecl, body, vector, flag.
const TES_HAVOK_SET_VELOCITY: u32 = 0x0062_b930;
/// The `float` used as the upward component (`0x101712c`).
const UPWARD_COMPONENT: u32 = 0x0101_712c;
/// `CombatManager::GetCombatantCount` (Xbox PDB): thiscall on the manager,
/// taking an actor and a pointer.
const COMBAT_MANAGER_GET_COMBATANT_COUNT: u32 = 0x0099_31c0;
/// Integer settings: the combatant count limit and the drop chance.
const COMBATANT_COUNT_SETTING: u32 = 0x011d_f780;
const DROP_WEAPON_CHANCE_SETTING: u32 = 0x011d_055c;
/// Random number (`00487f50`).
const RANDOM_NUMBER: u32 = 0x0048_7f50;
/// `Actor::DoDeathStuff` (Xbox PDB).
const ACTOR_DO_DEATH_STUFF: u32 = 0x008b_01c0;
/// `TESObjectREFR::RunScript` (Xbox PDB).
const REFR_RUN_SCRIPT: u32 = 0x0056_5870;
/// `bhkRagdollPenetrationUtil::Clear` (Xbox PDB).
const BHK_RAGDOLL_PENETRATION_UTIL_CLEAR: u32 = 0x00ca_20e0;
/// `00931ed0`: thiscall on the actor, taking an output; returns an object.
const ACTOR_OUTPUT_931ED0: u32 = 0x0093_1ed0;
/// `004a3a20` (thiscall).
const OBJECT_ACCESSOR_4A3A20: u32 = 0x004a_3a20;
/// `00ca2ad0`: thiscall on the penetration utility, taking the collision
/// object and a value.
const PENETRATION_UTIL_WORKER: u32 = 0x00ca_2ad0;
/// The `bool` setting that enables the death report.
const DEATH_REPORT_SETTING: u32 = 0x011d_f690;
/// `"'%s' (%08X) was killed by '%s' (%08X)."`.
const KILLED_BY_FORMAT: u32 = 0x0108_4cc8;
/// `"'%s' (%08X) has died with no attacker."`.
const DIED_WITH_NO_ATTACKER_FORMAT: u32 = 0x0108_4ca0;
/// `"%.20s is dead!"`.
const IS_DEAD_FORMAT: u32 = 0x0108_4cf0;
/// `MapMarkerData::GetLocationName` (Xbox PDB), used on the base form's
/// name member (`+0xd0`).
const LOCATION_NAME: u32 = 0x0040_8da0;
/// `Error` (Xbox PDB), cdecl, does nothing.
const ERROR_REPORT: u32 = 0x0040_fbe0;
/// Test on a setting object (`00454af0`).
const SETTING_TEST_454AF0: u32 = 0x0045_4af0;
const EXPORT_SETTING: u32 = 0x011e_0a30;
/// `PlayerCharacter::ExportProgressData` (Xbox PDB).
const PLAYER_EXPORT_PROGRESS_DATA: u32 = 0x0094_ed40;
/// `Actor::AddFactionMinorCrime` (Xbox PDB).
const ACTOR_ADD_FACTION_MINOR_CRIME: u32 = 0x008b_7c00;
/// `PlayerCharacter::SetSlowMoCamera` (Xbox PDB).
const PLAYER_SET_SLOW_MO_CAMERA: u32 = 0x0093_e530;
const SLOW_MO_SETTING: u32 = 0x011d_00fc;
/// `ProcessLists::FinishMagicShaderHitEffect` (Xbox PDB).
const PROCESS_LISTS_FINISH_MAGIC_SHADER_HIT_EFFECT: u32 = 0x0097_4a50;
/// `ContinuousBeamProjectile::SetKill` (Xbox PDB).
const CONTINUOUS_BEAM_PROJECTILE_SET_KILL: u32 = 0x009a_b9a0;
/// The pointer that is the `this` of `fn_0089f4e0`.
const MAGIC_SHADER_HOLDER: u32 = 0x011c_3f2c;
/// Test on a form (`0056af40`).
const ACTOR_TEST_56AF40: u32 = 0x0056_af40;
/// `ExtraDataList` accessor (`00421720`).
const EXTRA_GET_BASE_FORM_421720: u32 = 0x0042_1720;
/// `TESCreature` accessor (`0059f3a0`): a signed byte.
const CREATURE_GETTER_59F3A0: u32 = 0x0059_f3a0;
/// RTTI type descriptor of `TESActorBase` and of `TESCreature`.
const RTTI_TYPE_TES_ACTOR_BASE: u32 = 0x0118_46e8;
const RTTI_TYPE_TES_CREATURE: u32 = 0x0118_3a00;
/// Statistics worker (`005f5950`, cdecl, 7 words).
const STATISTICS_WORKER_5F5950: u32 = 0x005f_5950;
/// Weapon-form accessor returning a 16-bit value (`008d85e0`).
const WEAPON_GETTER_8D85E0: u32 = 0x008d_85e0;
/// The `float` setting (distance) that decides whether a dying actor speaks.
const DEATH_VOICE_DISTANCE_SETTING: u32 = 0x011d_0f9c;
/// `Actor::GetAnimGroup` (Xbox PDB), called by its address.
const ACTOR_GET_ANIM_GROUP_ADDRESS: u32 = 0x0089_7910;

/// Fetches one of the actor's remembered sounds from its extra data list
/// into `handle`, stops it (fading it out first while it plays, when
/// `fade_out`; otherwise `Stop` then `Release`) and stores the handle back.
fn forget_actor_sound(
    e: &mut Engine,
    this: Ptr<Actor>,
    handle: u32,
    getter: u32,
    setter: u32,
    fade_out: bool,
) {
    e.call(SOUND_HANDLE_CONSTRUCT, &args![handle]);
    let extra = extra_data_list_of(e, this.cast());
    e.call(getter, &args![extra, handle]);
    if fade_out {
        if e.call(SOUND_HANDLE_IS_PLAYING, &args![handle]).bool() {
            e.call(SOUND_HANDLE_FADE_OUT_AND_RELEASE, &args![handle, 500u32]);
        } else {
            e.call(SOUND_HANDLE_RELEASE, &args![handle]);
        }
    } else {
        e.call(SOUND_HANDLE_STOP, &args![handle]);
        e.call(SOUND_HANDLE_RELEASE, &args![handle]);
    }
    let extra = extra_data_list_of(e, this.cast());
    e.call(setter, &args![extra, handle]);
}

/// Destroys the three sound handles `actor_kill` keeps (in the order the
/// compiler's cleanup does) and releases their memory.
fn destroy_kill_sound_handles(e: &mut Engine, handles: u32) {
    e.call(SOUND_HANDLE_DESTROY, &args![handles + 24]);
    e.call(SOUND_HANDLE_DESTROY, &args![handles + 12]);
    e.call(SOUND_HANDLE_DESTROY, &args![handles]);
    e.mem.free(handles);
}

/// Calls `TESObjectREFR::SetMarkerUsed` on the furniture the process
/// reports (`GetCurrentFurniture`) with its index, when it reports one.
fn release_furniture_marker(e: &mut Engine, this: Ptr<Actor>) {
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        if e.vcall(process, PROCESS_SLOT_4C8, &args![]).u32() != 0 {
            let index_source = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            let furniture_source = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            let index = e.vcall(index_source, PROCESS_SLOT_4D0, &args![]).u32();
            let furniture = e.vcall(furniture_source, PROCESS_SLOT_4C8, &args![]).u32();
            e.call(REFR_SET_MARKER_USED, &args![furniture, index, 0u32]);
        }
    }
}

/// `*(setting)` as a `float`, for a `float` setting object.
fn float_setting(e: &mut Engine, setting: u32) -> f32 {
    let pointer = e.call(SETTING_GET_VALUE_POINTER, &args![setting]).u32();
    e.mem.f32(pointer)
}

/// `*(setting)` as an integer, for an integer setting object.
fn int_setting(e: &mut Engine, setting: u32) -> u32 {
    let pointer = e.call(SETTING_GET_INT_POINTER, &args![setting]).u32();
    e.mem.u32(pointer)
}

/// The kill statistics `Actor::Kill` records when the attacker belongs to
/// the player: the entry point `0x15`, and the statistics worker three
/// times with the weapon, the creature type and the weapon's numbers; then
/// the `VATS` counter.
fn record_kill_statistics(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>, player: u32) {
    e.call(HANDLE_ENTRY_POINT, &args![0x15u32, player, this, this]);
    let acquire = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![attacker]).u32();
    let weapon_entry = e
        .vcall(acquire, PROCESS_SLOT_CURRENT_WEAPON, &args![])
        .u32();
    let weapon_form = if weapon_entry != 0 {
        e.call(ENTRY_FORM, &args![weapon_entry]).u32()
    } else {
        0
    };
    let mut base = base_form(e, this.cast()).addr();
    if e.call(ACTOR_TEST_56AF40, &args![this]).bool() {
        let extra = extra_data_list_of(e, this.cast());
        base = e.call(EXTRA_GET_BASE_FORM_421720, &args![extra]).u32();
    }
    let creature_type = if e.vcall(this.addr(), ACTOR_SLOT_218, &args![]).bool() {
        0u8
    } else {
        let actor_base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
        let creature = e
            .call(
                RTTI_DYNAMIC_CAST,
                &args![
                    actor_base,
                    0u32,
                    RTTI_TYPE_TES_ACTOR_BASE,
                    RTTI_TYPE_TES_CREATURE,
                    0u32
                ],
            )
            .u32();
        let kind = e.call(CREATURE_GETTER_59F3A0, &args![creature]).u8() as i8;
        (kind as i32 + 1) as u8
    };
    if !e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![this]).bool() {
        for (first, second) in [(1u32, 1u32), (0, 1)] {
            let number = weapon_number(e, weapon_form);
            let slot = weapon_slot(e, weapon_form);
            e.call(
                STATISTICS_WORKER_5F5950,
                &args![first, second, base, weapon_form, 0u32, slot, number],
            );
        }
        let number = weapon_number(e, weapon_form);
        e.call(
            STATISTICS_WORKER_5F5950,
            &args![
                2u32,
                1u32,
                0u32,
                weapon_form,
                0u32,
                u32::from(creature_type as i8 as i16 as u16),
                number
            ],
        );
    }
    if e.call(ENTRY_FORM, &args![VATS_OBJECT]).u32() == 4 {
        fn_0089f560(e, Ptr::new(VATS_OBJECT));
    }
}

/// `(weapon type + 1) & 0xffff` of a weapon form, 1 when there is none.
fn weapon_number(e: &mut Engine, weapon_form: u32) -> u32 {
    if weapon_form != 0 {
        u32::from((e.call(WEAPON_TYPE, &args![weapon_form]).u16()).wrapping_add(1))
    } else {
        1
    }
}

/// The 16-bit slot number of a weapon form (`008d85e0`), `0x2d` when none.
fn weapon_slot(e: &mut Engine, weapon_form: u32) -> u32 {
    if weapon_form != 0 {
        u32::from(e.call(WEAPON_GETTER_8D85E0, &args![weapon_form]).u16())
    } else {
        0x2d
    }
}

/// The part of `Actor::Kill` for an essential actor (the essential setting
/// is on and `GetEssential` is true): it does not die. At process levels 3
/// and 5 its health is set to the setting's share of the base health;
/// otherwise its crime list, timers and package are reset, it leaves
/// combat, releases its furniture, goes to life state 6 with full health
/// and, at process level 6, shows a message. Returns true when the whole
/// kill ends here (the first case, which also destroys the sound handles).
fn kill_essential_actor(e: &mut Engine, this: Ptr<Actor>, handles: u32) -> bool {
    if e.call(ACTOR_PROCESS_LEVEL_IS_3, &args![this]).bool()
        || e.call(ACTOR_PROCESS_LEVEL_IS_5, &args![this]).bool()
    {
        let owner = this.addr() + ACTOR_VALUE_OWNER_OFFSET;
        let base_health = e.vcall(owner, 0x00, &args![0x10u32]).i32() as f32;
        let share = float_setting(e, LOW_LEVEL_HEALTH_SETTING);
        let mut amount = (share as f64 * base_health as f64) as f32;
        let current = e.vcall(owner, 0x0c, &args![0x10u32]).f32();
        amount = (amount as f64 - current as f64) as f32;
        e.vcall(
            this.addr(),
            ACTOR_SLOT_MODIFY_ACTOR_VALUE,
            &args![0x10u32, amount, 0u32],
        );
        destroy_kill_sound_handles(e, handles);
        return true;
    }
    let package = e.call(ACTOR_GET_CURRENT_EDITOR_PACKAGE, &args![this]).u32();
    let extra = extra_data_list_of(e, this.cast());
    e.call(EXTRA_REMOVE_PLAYER_CRIME_LIST_EXTRA, &args![extra, this]);
    let timer = float_setting(e, ESSENTIAL_DOWN_TIMER_SETTING);
    let process = e.get(this, Actor::pCurrentProcess).addr();
    e.vcall(process, PROCESS_SLOT_E8, &args![timer]);
    let twenty: f32 = e.global(TWENTY);
    let process = e.get(this, Actor::pCurrentProcess).addr();
    e.vcall(process, PROCESS_SLOT_2FC, &args![twenty]);
    if package != 0 && e.call(PACKAGE_TYPE, &args![package]).i32() != 0xf {
        e.call(ACTOR_END_INTERRUPT_PACKAGE, &args![this, 0u32]);
    }
    e.call(ACTOR_CLEAR_IN_COMBAT, &args![this, 1u32]);
    release_furniture_marker(e, this);
    e.call(MAGIC_CASTER_INTERRUPT_CAST, &args![this.byte_add(0x88)]);
    e.call(ACTOR_SET_LIFE_STATE, &args![this, 6u32]);
    e.call(ACTOR_SET_HAVOK_WEAPON, &args![this]);
    e.call(ACTOR_RESTORE_FULL_HEALTH_AND_CONDITIONS, &args![this]);
    if e.call(PROCESS_LEVEL_NUMBER, &args![this]).i32() == 6 {
        let text = e
            .call(SETTING_GET_STRING, &args![DOWN_MESSAGE_SETTING])
            .u32();
        let name = e.call(REFR_GET_NAME, &args![this]).u32();
        let buffer = e.mem.alloc(204);
        e.call(SPRINTF, &args![buffer, TWO_STRINGS_FORMAT, name, text]);
        let duration: f32 = e.global(MESSAGE_DURATION);
        e.call(
            SHOW_MESSAGE,
            &args![buffer, 0u32, MESSAGE_ICON_SURPRISED, 0u32, duration, 0u32],
        );
        e.mem.free(buffer);
    }
    false
}

/// Reports the death through `Error` (which does nothing) with the two
/// actors' names and ids.
fn report_death(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>) {
    if !attacker.is_null() {
        let attacker_id = e.call(FORM_NUMERIC_ID, &args![attacker]).u32();
        let base = e.call(ACTOR_BASE_FORM, &args![attacker]).u32();
        let attacker_name = e.call(LOCATION_NAME, &args![base + 0xd0]).u32();
        let id = e.call(FORM_NUMERIC_ID, &args![this]).u32();
        let base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
        let name = e.call(LOCATION_NAME, &args![base + 0xd0]).u32();
        e.call(
            ERROR_REPORT,
            &args![KILLED_BY_FORMAT, name, id, attacker_name, attacker_id],
        );
    } else {
        let id = e.call(FORM_NUMERIC_ID, &args![this]).u32();
        let base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
        let name = e.call(LOCATION_NAME, &args![base + 0xd0]).u32();
        e.call(ERROR_REPORT, &args![DIED_WITH_NO_ATTACKER_FORMAT, name, id]);
    }
}

/// The part of `Actor::Kill` that updates the combat bookkeeping for the
/// player or a teammate of the player: crime for a combatant that sees the
/// player, and the slow-motion camera when nothing else is fighting.
fn update_kill_counts(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>, player: u32) {
    let counter = e.mem.alloc(8);
    if attacker.addr() == player
        || (!attacker.is_null()
            && e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![attacker])
                .bool())
    {
        let manager = e.global::<u32>(COMBAT_MANAGER);
        let count = e
            .call(
                COMBAT_MANAGER_GET_COMBATANT_COUNT,
                &args![manager, player, counter],
            )
            .u32();
        if count > 0
            && e.call(PLAYER_IS_DETECTED_BY_NON_TEAMMATES, &args![player, this])
                .bool()
        {
            e.call(ACTOR_ADD_FACTION_MINOR_CRIME, &args![this, 1u32, 1u32]);
        }
        if e.call(ENTRY_FORM, &args![VATS_OBJECT]).u32() == 0 && count == 0 {
            let factor = float_setting(e, SLOW_MO_SETTING);
            e.call(
                PLAYER_SET_SLOW_MO_CAMERA,
                &args![player, this, factor, 1u32, 0xffff_ffffu32],
            );
        }
    }
    e.mem.free(counter);
}

/// The killing part of `Actor::Kill` (the actor is not essential): clears
/// the actor out of the process lists, settles who gets the blame
/// (`attacker` may be replaced by its commanding actor), does the crime,
/// karma and experience bookkeeping for the player, releases furniture,
/// plays the death dialogue or creature sound, sets the life state to 1,
/// handles the player's own death, gives the body a ragdoll impulse and
/// finishes with `Actor::DoDeathStuff` and the debug line.
fn kill_actor(
    e: &mut Engine,
    this: Ptr<Actor>,
    attacker: &mut Ptr<Actor>,
    was_aggressor: bool,
    player: u32,
) {
    e.call(PROCESS_LISTS_WORKER_96E2F0, &args![PROCESS_LISTS, this]);
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(process, PROCESS_SLOT_310, &args![0u32]);
    }
    if !attacker.is_null() && e.call(GET_SAVED_ACQUIRE_OBJECT, &args![*attacker]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![*attacker]).u32();
        if e.vcall(process, PROCESS_SLOT_52C, &args![]).u32() != 0 {
            let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![*attacker]).u32();
            *attacker = Ptr::new(e.vcall(process, PROCESS_SLOT_52C, &args![]).u32());
        }
    }
    if !attacker.is_null() && e.vcall(attacker.addr(), ACTOR_SLOT_218, &args![]).bool() {
        let mut blamed = true;
        if e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32() != 0 {
            let controller = e.vcall(this.addr(), ACTOR_SLOT_428, &args![]).u32();
            blamed = e
                .call(
                    COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET,
                    &args![controller, *attacker],
                )
                .bool();
        }
        if blamed {
            e.mem.set_u8(this.addr() + 0xc4, u8::from(!was_aggressor));
            if attacker.addr() == player {
                let flag = e.mem.alloc(4);
                e.mem.set_u8(flag, 0);
                let reaction = e
                    .call(ACTOR_GET_FACTION_FIGHT_REACTION, &args![this, player, flag])
                    .i32();
                e.mem.free(flag);
                if (reaction == 3 || reaction == 2)
                    && e.call(ACTOR_TEST_5A3790, &args![this]).bool()
                {
                    e.mem.set_u8(this.addr() + 0xc4, 0);
                }
            }
        }
    }
    if attacker.addr() == player && !e.vcall(this.addr(), ACTOR_SLOT_2E8, &args![]).bool() {
        if e.vcall(this.addr(), ACTOR_SLOT_218, &args![]).bool() {
            e.call(MISC_STAT_INCREMENT, &args![2u32]);
            if !e.call(REFR_IS_PART_OF_EVIL_FACTION, &args![this]).bool()
                && e.mem.u8(this.addr() + 0xc4) != 0
            {
                e.call(PLAYER_SET_IS_A_MURDERER, &args![player]);
            }
        } else {
            e.call(MISC_STAT_INCREMENT, &args![3u32]);
        }
        e.call(MISC_STAT_INCREMENT, &args![0x23u32]);
        e.call(ACTOR_BASE_FORM, &args![this]);
        let base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
        let karma = e.vcall(base + 0x100, 0x0c, &args![0x17u32]).f32();
        let alignment = e.call(GET_ALIGNMENT_FOR_KARMA, &args![karma]).u32();
        if !was_aggressor
            && e.call(ACTOR_HAS_FACTION_THAT_CARES_ABOUT_CRIME, &args![this])
                .bool()
        {
            let mut reward = float_setting(e, KARMA_REWARD_SETTING);
            if e.vcall(this.addr(), ACTOR_SLOT_21C, &args![]).bool() {
                reward = float_setting(e, KARMA_REWARD_CREATURE_SETTING);
            }
            if alignment != 1 && alignment <= 4 {
                reward = float_setting(e, KARMA_REWARD_ALIGNMENT_SETTINGS[alignment as usize]);
            }
            if reward as f64 != 0.0 {
                let amount = e.call(FTOL, &args![reward as f64]).u32();
                e.call(PLAYER_REWARD_KARMA, &args![player, amount]);
            }
        }
    }
    if attacker.addr() == player
        && e.call(PLAYER_TEST_4997B0, &args![player]).bool()
        && !e
            .call(PLAYER_IS_DETECTED_BY_NON_TEAMMATES, &args![player, 0u32])
            .bool()
    {
        let mut weapon_form = 0u32;
        if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32() != 0 {
            let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32();
            if e.vcall(process, PROCESS_SLOT_CURRENT_WEAPON, &args![])
                .u32()
                != 0
            {
                let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32();
                let entry = e
                    .vcall(process, PROCESS_SLOT_CURRENT_WEAPON, &args![])
                    .u32();
                weapon_form = e.call(ENTRY_FORM, &args![entry]).u32();
            }
        }
        let mut clear = true;
        if weapon_form != 0 && !e.call(WEAPON_FORM_TEST_6450C0, &args![weapon_form]).bool() {
            let number = e
                .call(WEAPON_FORM_NUMBER, &args![weapon_form, 0u32, 0u32])
                .u32();
            if number != int_setting(e, WEAPON_NUMBER_SETTING) {
                let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32();
                let entry = e
                    .vcall(process, PROCESS_SLOT_CURRENT_WEAPON, &args![])
                    .u32();
                if !e
                    .call(ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, &args![entry, 0xbu32])
                    .bool()
                {
                    clear = false;
                }
            }
        }
        if clear {
            e.mem.set_u8(this.addr() + 0xc4, 0);
        }
    }
    award_experience(e, this, *attacker, player);
    release_furniture_marker(e, this);
    if this.addr() != player {
        e.call(ACTOR_WORKER_87EEF0, &args![this]);
        e.set(this, Actor::pMyKiller, attacker.cast());
    }
    let group = e
        .call(
            ACTOR_GET_ANIM_GROUP_ADDRESS,
            &args![this, 0xe0u32, 0u32, 0u32, 0u32],
        )
        .u16();
    if e.call(ANIM_GROUP_GET_TYPE, &args![u32::from(group)]).i32() == 0xe0 {
        let process = e.get(this, Actor::pCurrentProcess).addr();
        e.vcall(process, PROCESS_SLOT_614, &args![0x20u32]);
    }
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(process, PROCESS_SLOT_660, &args![]);
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        if e.call(ACQUIRE_OBJECT_FIELD_28, &args![process]).u32() == 0 {
            play_death_voice(e, this, *attacker, player);
        }
    }
    e.call(ACTOR_SET_LIFE_STATE, &args![this, 1u32]);
    e.call(ACTOR_SET_HAVOK_WEAPON, &args![this]);
    if this.addr() == player {
        player_dies(e, this);
    }
    finish_dying(e, this, *attacker, player);
    if e.call(DEBUG_MESSAGES_ENABLED, &args![]).bool() {
        let name = e.call(REFR_GET_NAME, &args![this]).u32();
        e.call(PRINT_DEBUG_LINE, &args![IS_DEAD_FORMAT, name]);
    }
}

/// The experience the player gets for the kill: only when the actor was
/// not a teammate's kill (or the attacker is a teammate of the player), is
/// neither a protected nor an unfriendly actor, and the damage tracked on
/// it passes the test against its health.
fn award_experience(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>, player: u32) {
    let mut by_player_side = true;
    if e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![this]).bool() {
        by_player_side =
            !attacker.is_null() && e.vcall(attacker.addr(), ACTOR_SLOT_360, &args![]).bool();
    }
    if e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
        || e.vcall(this.addr(), ACTOR_SLOT_2E8, &args![]).bool()
        || !by_player_side
    {
        return;
    }
    let tracked = if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(process, PROCESS_SLOT_730, &args![]).f32()
    } else {
        0.0
    };
    if (tracked as f64) <= 0.0 || tracked.is_nan() {
        return;
    }
    let owner = this.addr() + ACTOR_VALUE_OWNER_OFFSET;
    let base_health = e.vcall(owner, 0x00, &args![0x10u32]).i32() as f32;
    if !e
        .call(TRACKED_DAMAGE_TEST, &args![tracked, base_health])
        .bool()
    {
        return;
    }
    let word = e.call(ACTOR_GETTER_87F9F0, &args![this]).u16();
    let flag = u32::from(!e.vcall(this.addr(), ACTOR_SLOT_21C, &args![]).bool());
    let points = e
        .call(GET_EXPERIENCE_POINTS, &args![flag, u32::from(word)])
        .i32() as f32;
    let multiplier = e.call(PLAYER_INT_5BE4D0, &args![player]).u32();
    let scaled = e.call(SCALE_EXPERIENCE, &args![points, multiplier]).f32();
    let rounded = e.call(FLOAT_ROUND, &args![scaled]).f64();
    let amount = e.call(FTOL, &args![rounded]).u32();
    e.vcall(player, PLAYER_SLOT_488, &args![amount]);
}

/// Death dialogue or sound: unless the actor is within the hearing
/// distance of the player (and is not a teammate-killed or protected
/// case), a creature plays a creature sound (`PickCreatureSound(8)`) and
/// anything else starts the combat dialogue 6.
fn play_death_voice(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>, player: u32) {
    let mut near = !e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
        && e.call(GLOBAL_FLAG_525420, &args![]).bool();
    if near {
        let distance = e
            .call(
                REFR_GET_DISTANCE_FROM_REFERENCE,
                &args![this, player, 0u32, 0u32],
            )
            .f32();
        let limit = float_setting(e, DEATH_VOICE_DISTANCE_SETTING);
        if limit as f64 >= distance as f64 {
            near = false;
        }
    }
    if near {
        return;
    }
    let manager = e.global::<u32>(COMBAT_DIALOGUE_MANAGER);
    if !e.vcall(this.addr(), ACTOR_SLOT_21C, &args![]).bool() {
        e.call(
            COMBAT_DIALOGUE_START_DIALOGUE,
            &args![manager, this, attacker, 2u32, 6u32, 1u32, 0u32],
        );
        return;
    }
    let mut creature = 0u32;
    if e.vcall(this.addr(), ACTOR_SLOT_21C, &args![]).bool() {
        creature = base_form(e, this.cast()).addr();
    }
    let sound = if creature != 0 {
        e.call(CREATURE_PICK_CREATURE_SOUND, &args![creature, 8u32])
            .u32()
    } else {
        0
    };
    if sound == 0 {
        e.call(
            COMBAT_DIALOGUE_START_DIALOGUE,
            &args![manager, this, attacker, 2u32, 6u32, 1u32, 0u32],
        );
        return;
    }
    let handle = e.mem.alloc(12);
    let id = e.call(FORM_NUMERIC_ID, &args![sound]).u32();
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    e.call(
        AUDIO_GET_SOUND_HANDLE_BY_NUMERIC_ID,
        &args![audio, handle, id, 0x102u32],
    );
    let position = e.vcall(this.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
    let x = e.mem.u32(position);
    let y = e.mem.u32(position + 4);
    let z = e.mem.u32(position + 8);
    e.call(SOUND_HANDLE_SET_POSITION_XYZ, &args![handle, x, y, z]);
    let node = e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32();
    e.call(SOUND_HANDLE_SET_OBJECT_TO_FOLLOW, &args![handle, node]);
    e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
    e.call(SOUND_HANDLE_DESTROY, &args![handle]);
    e.mem.free(handle);
}

/// What happens when the player is the one dying: the Pipboy radio is
/// switched off, the death music plays, a VATS playback ends and an open
/// surgery menu closes.
fn player_dies(e: &mut Engine, this: Ptr<Actor>) {
    e.call(PIPBOY_RADIO_ENABLE, &args![0u32]);
    e.call(SOUND_FADE_WORKER, &args![0x3e8u32, 0u32]);
    let handle = e.mem.alloc(12);
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    let music = e
        .call(
            AUDIO_GET_SOUND_HANDLE_BY_NAME,
            &args![audio, handle, DEATH_MUSIC_NAME, 0x901u32],
        )
        .u32();
    e.call(SOUND_HANDLE_PLAY, &args![music, 0u32]);
    e.call(SOUND_HANDLE_DESTROY, &args![handle]);
    e.mem.free(handle);
    e.call(VATS_QUIT_PLAYBACK, &args![VATS_OBJECT, 0u32, 0u32]);
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(process, PROCESS_SLOT_3F4, &args![0u32]);
    }
    if e.call(INTERFACE_IS_TOP_MENU_ID, &args![0x41eu32]).bool() {
        e.call(SURGERY_MENU_CLOSE, &args![]);
    }
}

/// The last part of the killing: face animation, VATS, process state,
/// `TES` bookkeeping, the ragdoll impulse (with the many vector helpers)
/// and the weapon drop, or `Actor::DoDeathStuff` and the dead flag when
/// there is no process.
fn finish_dying(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>, player: u32) {
    let face = e.call(ACTOR_GET_FACE_ANIMATION_DATA, &args![this]).u32();
    if face != 0 {
        e.vcall(face, 0xd8, &args![1u32, 0u32]);
    }
    if this.addr() == player {
        let vats_acting = e.call(ENTRY_FORM, &args![VATS_OBJECT]).u32() == 4
            && e.call(VATS_GET_CURRENT_ACTION, &args![VATS_OBJECT]).u32() != 0
            && {
                let action = e.call(VATS_GET_CURRENT_ACTION, &args![VATS_OBJECT]).u32();
                e.mem.u32(action) == 0xf
            };
        if !vats_acting {
            e.call(VATS_WORKER_9CA1F0, &args![VATS_OBJECT]);
        }
    }
    let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    if process != 0 && e.call(ACQUIRE_OBJECT_FIELD_28, &args![process]).u32() == 0 {
        let flag = fn_0089f540(e, Ptr::new(process));
        if flag != 0 {
            let result = e
                .call(ACTOR_GETTER_80CED0, &args![this, u32::from(flag)])
                .u8();
            fn_0089f520(e, Ptr::new(process), result);
        }
    }
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(process, PROCESS_SLOT_28, &args![]);
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        let stamp_object = TIME_STAMP_OBJECT;
        let stamp = e.call(TIME_STAMP_867E30, &args![stamp_object]).u32();
        e.vcall(process, PROCESS_SLOT_6A0, &args![stamp as f32]);
    }
    let extra = extra_data_list_of(e, this.cast());
    let mut base = e
        .call(EXTRA_GET_LEV_CREA_ORIGINAL_BASE, &args![extra])
        .u32();
    if base == 0 {
        base = e.call(ACTOR_BASE_FORM, &args![this]).u32();
    }
    let tes = e.global::<u32>(TES_SINGLETON);
    e.call(TES_WORKER_459060, &args![tes, base, 1u32]);
    if e.vcall(this.addr(), ACTOR_SLOT_448, &args![]).bool() {
        let extra = extra_data_list_of(e, this.cast());
        e.call(EXTRA_REMOVE_TRESPASS_PACKAGE, &args![extra]);
    }
    let node = e.call(ACTOR_COLLISION_OBJECT, &args![this]).u32();
    let velocity = e.mem.alloc(16);
    let zero = 0.0f32;
    e.call(VECTOR4_CONSTRUCT, &args![velocity, zero, zero, zero, zero]);
    if e.call(MOBILE_OBJECT_GET_CHAR_CONTROLLER, &args![this])
        .u32()
        != 0
    {
        let linear = e.mem.alloc(16);
        let controller = e
            .call(MOBILE_OBJECT_GET_CHAR_CONTROLLER, &args![this])
            .u32();
        e.call(
            BHK_CHARACTER_PROXY_GET_LINEAR_VELOCITY,
            &args![controller, linear],
        );
        e.call(VECTOR_ASSIGN, &args![velocity, linear]);
        e.mem.free(linear);
    }
    if this.addr() == player {
        e.call(MOBILE_OBJECT_SET_CHASE_BIP, &args![this, 1u32]);
    } else {
        e.vcall(this.addr(), ACTOR_SLOT_2A0, &args![]);
    }
    let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
    let alive_body =
        process != 0 && e.call(ACQUIRE_OBJECT_FIELD_28, &args![process]).u32() == 0 && node != 0;
    if alive_body {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        let timer = float_setting(e, GREETING_TIMER_SETTING);
        e.vcall(process, PROCESS_SLOT_4B4, &args![timer]);
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        let mut impulse_branch = e.vcall(process, PROCESS_SLOT_40C, &args![]).u32() == 0;
        if !impulse_branch {
            let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            impulse_branch = e.vcall(process, PROCESS_SLOT_40C, &args![]).u32() == 6;
        }
        if impulse_branch {
            apply_death_impulse(e, this, attacker, player, node, velocity);
        } else if e.vcall(this.addr(), ACTOR_SLOT_234, &args![]).bool() {
            apply_knocked_impulse(e, this, node);
        }
        let manager = e.global::<u32>(COMBAT_MANAGER);
        let count = e
            .call(
                COMBAT_MANAGER_GET_COMBATANT_COUNT,
                &args![manager, 0u32, 0u32],
            )
            .u32();
        let too_many = count > int_setting(e, COMBATANT_COUNT_SETTING);
        let skip = too_many && e.vcall(this.addr(), ACTOR_SLOT_38C, &args![]).bool();
        if !skip && e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
            let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
            if e.vcall(process, PROCESS_SLOT_52C, &args![]).u32() == 0 {
                let roll = e.call(RANDOM_NUMBER, &args![]).u32() % 100;
                if roll < int_setting(e, DROP_WEAPON_CHANCE_SETTING)
                    || !e.vcall(this.addr(), ACTOR_SLOT_38C, &args![]).bool()
                {
                    fn_0089f580(e, this);
                }
            }
        }
    } else {
        e.call(ACTOR_DO_DEATH_STUFF, &args![this]);
        e.call(REFR_RUN_SCRIPT, &args![this]);
        e.mem.set_u8(this.addr() + 0x118, 1);
    }
    e.mem.free(velocity);
}

/// The impulse a dying actor's body gets when it is not knocked down in a
/// way that already moves it: the direction is the character proxy's
/// velocity direction, the actor's heading when that is zero, or away from
/// the player when the player is the killer.
fn apply_death_impulse(
    e: &mut Engine,
    this: Ptr<Actor>,
    attacker: Ptr<Actor>,
    player: u32,
    node: u32,
    velocity: u32,
) {
    if e.call(ACTOR_TEST_87EEB0, &args![this]).bool() {
        return;
    }
    let locals = e.mem.alloc(0x100);
    let direction = locals;
    let transformed = locals + 0x10;
    let rotation = locals + 0x20;
    let rotated = locals + 0x30;
    let player_position = locals + 0x40;
    let own_position = locals + 0x50;
    let difference = locals + 0x60;
    let angular = locals + 0x70;
    copy_vector(e, DEFAULT_DIRECTION, direction);
    let object = e
        .call(VECTOR_TRANSFORM_630B40, &args![velocity, transformed])
        .u32();
    let length = e.call(VECTOR_LENGTH_4586D0, &args![object]).f64();
    if length == 0.0 {
        let heading = e
            .vcall(this.addr(), ACTOR_SLOT_GET_HEADING, &args![0u32])
            .f32();
        e.call(VECTOR_ROTATE_4A0C90, &args![rotation, heading]);
        let result = e
            .call(
                VECTOR_MULTIPLY_4B4500,
                &args![rotation, rotated, UNIT_VECTOR],
            )
            .u32();
        copy_vector(e, result, direction);
    } else {
        e.call(VECTOR_COPY_458620, &args![direction, velocity]);
        e.call(VECTOR_NORMALIZE, &args![direction]);
    }
    if attacker.addr() == player
        && this.addr() != player
        && e.call(MOBILE_OBJECT_GET_CHAR_CONTROLLER, &args![this])
            .u32()
            != 0
    {
        copy_vector(e, DEFAULT_DIRECTION, player_position);
        copy_vector(e, DEFAULT_DIRECTION, own_position);
        let controller = e
            .call(MOBILE_OBJECT_GET_CHAR_CONTROLLER, &args![player])
            .u32();
        e.call(
            BHK_CHARACTER_CONTROLLER_GET_POSITION,
            &args![controller, player_position],
        );
        let controller = e
            .call(MOBILE_OBJECT_GET_CHAR_CONTROLLER, &args![this])
            .u32();
        e.call(
            BHK_CHARACTER_CONTROLLER_GET_POSITION,
            &args![controller, own_position],
        );
        let result = e
            .call(
                VECTOR_SUBTRACT,
                &args![own_position, difference, player_position],
            )
            .u32();
        copy_vector(e, result, direction);
        e.call(VECTOR_NORMALIZE, &args![direction]);
    }
    e.call(BHK_WORLD_SET_MOTION, &args![node, 1u32, 1u32, 1u32, 1u32]);
    let zero = 0.0f32;
    e.call(VECTOR3_CONSTRUCT_43D410, &args![angular, zero, 0u32, 0u32]);
    e.call(COLLISION_OBJECT_SET_ANGULAR_VELOCITY, &args![node, angular]);
    let factor = float_setting(e, DEATH_IMPULSE_SETTING);
    e.call(VECTOR_SCALE_IN_PLACE, &args![direction, factor]);
    e.call(TES_HAVOK_ADD_VELOCITY, &args![node, direction, 0u32]);
    e.vcall(this.addr(), ACTOR_SLOT_48, &args![4u32]);
    e.mem.free(locals);
}

/// The push a dying actor gets in the knocked-down state: the body is
/// un-fixed and given an upward velocity along its heading.
fn apply_knocked_impulse(e: &mut Engine, this: Ptr<Actor>, node: u32) {
    e.call(BHK_RIGID_BODY_SET_FIXED, &args![node, 0u32, 1u32]);
    let locals = e.mem.alloc(0x100);
    let rotation = locals;
    let up = locals + 0x20;
    let rotated = locals + 0x30;
    let velocity = locals + 0x40;
    let heading = e
        .vcall(this.addr(), ACTOR_SLOT_GET_HEADING, &args![0u32])
        .f32();
    e.call(VECTOR_ROTATE_4A0C90, &args![rotation, heading]);
    let upward: f32 = e.global(UPWARD_COMPONENT);
    e.call(VECTOR3_CONSTRUCT_416870, &args![up, 0.0f32, upward, 0.0f32]);
    let result = e
        .call(VECTOR_MULTIPLY_4B4500, &args![rotation, rotated, up])
        .u32();
    copy_vector(e, result, up);
    e.call(VECTOR_ASSIGN, &args![velocity, up]);
    e.call(TES_HAVOK_SET_VELOCITY, &args![node, velocity, 0u32]);
    e.mem.free(locals);
}

// Translated from 0089d900 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::Kill` (Xbox PDB): the actor `attacker` (may be null) kills this
/// one with `damage`.
///
/// Does nothing for protected actors (slot `0x22c(0)`). Forgets the
/// actor's remembered weapon, idle and awake sounds. With the task queue in
/// use it only queues the kill (`QueueActorKill`, `damage` in a 4-byte
/// block, `attacker` stored at `+0xc0`). Otherwise records the kill
/// statistics for a player-owned attacker, clears `bMurderAlarm`, stops
/// dialogue and combat, tells the process and clears collision and ragdoll
/// state, and then takes one of two branches chosen by the essential
/// setting (`0x11e0888`) and `GetEssential`: [`kill_essential_actor`] keeps
/// the actor alive, [`kill_actor`] kills it. Both end in the common tail:
/// clearing the penetration utility, the death report, exporting progress
/// for the player, the combat bookkeeping, the magic shader effect and the
/// continuous beam.
/// The compiler's exception-unwinding frame is not translated;
/// `Actor::GetAnimGroup` is called by its address.
pub fn actor_kill(e: &mut Engine, this: Ptr<Actor>, attacker: Ptr<Actor>, damage: f32) {
    let mut attacker = attacker;
    if e.vcall(this.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
        return;
    }
    let player_address = e.global::<u32>(PLAYER_CHARACTER);
    if e.vcall(this.addr(), ACTOR_SLOT_1A0, &args![0u32]).bool() && this.addr() != player_address {
        return;
    }
    // Three sound handles of 12 bytes (`BSSoundHandle`).
    let handles = e.mem.alloc(36);
    forget_actor_sound(
        e,
        this,
        handles,
        EXTRA_GET_WEAPON_ATTACK_SOUND,
        EXTRA_SET_WEAPON_ATTACK_SOUND,
        true,
    );
    forget_actor_sound(
        e,
        this,
        handles + 12,
        EXTRA_GET_WEAPON_IDLE_SOUND,
        EXTRA_SET_WEAPON_IDLE_SOUND,
        true,
    );
    forget_actor_sound(
        e,
        this,
        handles + 24,
        EXTRA_GET_CREATURE_AWAKE_SOUND,
        EXTRA_SET_CREATURE_AWAKE_SOUND,
        false,
    );
    if e.call(PICK_UP_GOES_TO_TASK_QUEUE, &args![]).bool() {
        let block = e.call(OPERATOR_NEW, &args![4u32]).u32();
        e.mem.set_f32(block, damage);
        e.set(this, Actor::pMyKiller, attacker.cast());
        let tes = e.call(GET_TES, &args![]).u32();
        e.call(QUEUE_ACTOR_KILL, &args![tes, this, block, 0u32]);
        destroy_kill_sound_handles(e, handles);
        return;
    }

    if !attacker.is_null()
        && e.vcall(attacker.addr(), ACTOR_SLOT_360, &args![]).bool()
        && !e.vcall(this.addr(), ACTOR_SLOT_360, &args![]).bool()
    {
        record_kill_statistics(e, this, attacker, player_address);
    }

    e.mem.set_u8(this.addr() + 0xc4, 0);
    let mut was_aggressor = false;
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        if e.vcall(process, PROCESS_SLOT_1C4, &args![]).bool() {
            was_aggressor = true;
        }
    }
    e.call(MOBILE_OBJECT_STOP_CURRENT_DIALOGUE, &args![this]);
    e.vcall(this.addr(), ACTOR_SLOT_STOP_COMBAT, &args![0u32]);
    if e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32() != 0 {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(process, PROCESS_SLOT_6C4, &args![]);
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![this]).u32();
        e.vcall(process, PROCESS_SLOT_180, &args![]);
    }
    if e.vcall(this.addr(), ACTOR_SLOT_234, &args![]).bool() {
        let collision = e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32();
        e.call(BHK_RIGID_BODY_SET_FIXED, &args![collision, 0u32, 1u32]);
    }
    let ragdoll = e.mem.u32(this.addr() + 0xac);
    if ragdoll != 0 {
        e.call(BHK_RAGDOLL_DISABLE_RAGDOLL_ANIM, &args![ragdoll, 1u32]);
    }

    let setting = e
        .call(SETTING_GET_BOOL_POINTER, &args![ESSENTIAL_SETTING])
        .u32();
    if e.mem.u8(setting) != 0 && e.call(ACTOR_GET_ESSENTIAL, &args![this]).bool() {
        if kill_essential_actor(e, this, handles) {
            return;
        }
    } else {
        kill_actor(e, this, &mut attacker, was_aggressor, player_address);
    }

    // The common tail.
    let penetration = e.mem.u32(this.addr() + 0xb0);
    if penetration != 0 {
        e.call(BHK_RAGDOLL_PENETRATION_UTIL_CLEAR, &args![penetration]);
        let output = e.mem.alloc(4);
        let object = e.call(ACTOR_OUTPUT_931ED0, &args![this, output]).u32();
        let value = e.call(OBJECT_ACCESSOR_4A3A20, &args![object]).u32();
        e.mem.free(output);
        let collision = e.vcall(this.addr(), ACTOR_SLOT_1D0, &args![]).u32();
        e.call(
            PENETRATION_UTIL_WORKER,
            &args![penetration, collision, value],
        );
    }
    let report = e
        .call(SETTING_GET_BOOL_POINTER, &args![DEATH_REPORT_SETTING])
        .u32();
    if e.mem.u8(report) != 0 {
        report_death(e, this, attacker);
    }
    let export = e.call(SETTING_TEST_454AF0, &args![EXPORT_SETTING]).bool();
    if export && this.addr() == player_address {
        e.set(this, Actor::pMyKiller, attacker.cast());
        e.call(PLAYER_EXPORT_PROGRESS_DATA, &args![player_address, 1u32]);
        e.set(this, Actor::pMyKiller, Ptr::NULL);
    }
    update_kill_counts(e, this, attacker, player_address);
    if fn_0089f500(e, Ptr::new(player_address)) != 0 {
        let holder = e.global::<u32>(MAGIC_SHADER_HOLDER);
        let shader = fn_0089f4e0(e, Ptr::new(holder));
        if shader != 0 {
            e.call(
                PROCESS_LISTS_FINISH_MAGIC_SHADER_HIT_EFFECT,
                &args![PROCESS_LISTS, this, shader],
            );
        }
    }
    let beam = e.mem.u32(this.addr() + 0x1a0);
    if beam != 0 {
        e.call(CONTINUOUS_BEAM_PROJECTILE_SET_KILL, &args![beam, this]);
    }
    destroy_kill_sound_handles(e, handles);
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
        entry!(0x008985d0, fn_008985d0(Ptr<Actor>)),
        entry!(0x00898650, fn_00898650(Ptr<Actor>, Ptr<Actor>, f32)),
        entry!(0x00898fd0, fn_00898fd0(Ptr) -> bool),
        entry!(0x00898ff0, fn_00898ff0(Ptr<Actor>) -> bool),
        entry!(0x00899030, fn_00899030(Ptr<Actor>) -> u32),
        entry!(0x00899060, fn_00899060(Ptr<Actor>, u8)),
        entry!(
            0x008990a0,
            actor_handle_blocked_attack(Ptr<Actor>, u32, f32, Ptr<Actor>, u32)
        ),
        entry!(0x008990f0, fn_008990f0(Ptr<Actor>) -> u32),
        entry!(0x0089d5e0, fn_0089d5e0(Ptr) -> bool),
        entry!(0x0089d600, fn_0089d600(Ptr) -> Ptr),
        entry!(0x0089d620, fn_0089d620(Ptr) -> u32),
        entry!(0x0089d640, fn_0089d640(Ptr, u32, u32, u32)),
        entry!(0x0089d670, fn_0089d670(Ptr) -> u8),
        entry!(0x0089d690, fn_0089d690(Ptr) -> u8),
        entry!(0x0089d6b0, fn_0089d6b0(Ptr) -> u32),
        entry!(0x0089d6d0, fn_0089d6d0(Ptr) -> u32),
        entry!(
            0x0089d6f0,
            fn_0089d6f0(Ptr<Actor>, f32, f32, Ptr<Actor>) -> u32
        ),
        entry!(
            0x0089d8b0,
            actor_get_random_worn_armor(Ptr<Actor>, u8) -> u32
        ),
        entry!(0x0089f4e0, fn_0089f4e0(Ptr) -> u32),
        entry!(0x0089f500, fn_0089f500(Ptr) -> u8),
        entry!(0x0089f520, fn_0089f520(Ptr, u8)),
        entry!(0x0089f540, fn_0089f540(Ptr) -> u8),
        entry!(0x0089f560, fn_0089f560(Ptr)),
        entry!(0x0089f580, fn_0089f580(Ptr<Actor>)),
        entry!(0x008a0250, fn_008a0250(Ptr)),
        entry!(0x008a0330, fn_008a0330(Ptr<Actor>) -> u32),
        entry!(0x0089f780, actor_resurrect(Ptr<Actor>, u8, u8, u8)),
        entry!(0x0089fb80, fn_0089fb80(Ptr<Actor>, u8)),
        entry!(
            0x0089fcf0,
            fn_0089fcf0(Ptr<Actor>, Ptr<Actor>, u32, u8, u32, u8, i32, u32, u32)
        ),
        entry!(0x008a0270, fn_008a0270(Ptr<Actor>, u8) -> u8),
        entry!(0x008a02d0, fn_008a02d0(Ptr<Actor>) -> u32),
        entry!(0x008a0360, fn_008a0360(Ptr<Actor>) -> u32),
        entry!(0x008a03a0, fn_008a03a0(Ptr<Actor>)),
        entry!(0x008a05f0, fn_008a05f0(Ptr<Actor>)),
        entry!(0x008a0680, fn_008a0680(Ptr<Actor>, u32, u32)),
        entry!(0x008987f0, fn_008987f0(Ptr<Actor>, Ptr<Actor>, u32)),
        entry!(0x00899200, fn_00899200(Ptr<Actor>, u8, u8)),
        entry!(
            0x00899cb0,
            actor_combat_hit(Ptr<Actor>, Ptr<Actor>, u8, Ptr, u8)
        ),
        entry!(0x0089d900, actor_kill(Ptr<Actor>, Ptr<Actor>, f32)),
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
        0x00931850, 0x0087aff0, 0x008d5cb0, 0x0097fa10, 0x008b87a0, 0x00891b90, 0x005b6f70,
        0x00703c00, 0x006463e0, 0x006447f0, 0x00567400, 0x009a60e0, 0x008d0600, 0x0097f7f0,
        0x0097f6d0, 0x0087f3d0, 0x008808a0, 0x00579220, 0x00418b10, 0x00891b70, 0x0046e8c0,
        0x0045bb80, 0x00461130, 0x00a592c0, 0x0056acb0, 0x00461580, 0x0089d900, 0x00422c20,
        0x0042e8e0, 0x005a29b0, 0x008a1a40, 0x00884f80, 0x00972aa0, 0x0041cf30, 0x00633c90,
        0x00c6a350, 0x009370b0, 0x0096d470, 0x0096d450, 0x00906dc0, 0x004511e0, 0x00458be0,
        0x00444850, 0x00450ff0, 0x009334b0, 0x0040fbf0, 0x0040fba0, 0x0047c850, 0x0047d0b0,
        0x006ebd10, 0x00574920, 0x006047c0, 0x005f9e00, 0x00441b00, 0x005723b0, 0x0087f9c0,
        0x00967220, 0x00703350, 0x00703860, 0x00881680, 0x009922b0, 0x00992110, 0x009923d0,
        0x0097f930, 0x004030b0, 0x0040fbe0, 0x007014e0, 0x00576d30, 0x00981450, 0x005a03f0,
        0x00440d80, 0x008a8230, 0x0093a690, 0x009818b0, 0x0097da50, 0x0097e4b0, 0x0097efe0,
        0x008a1800, 0x00881510, 0x008ace90, 0x004f8960, 0x008a0d10, 0x0067a770, 0x005a3790,
        0x0043d4d0, 0x00422670, 0x00422590, 0x009839b0, 0x008a67f0, 0x009336c0, 0x00970a20,
        0x00970b30, 0x008c0460, 0x008b0670, 0x008b0ba0, 0x008bffc0, 0x008b06d0, 0x008a6650,
        0x005d8a70, 0x008256d0, 0x0047ead0, 0x0097f580, 0x008a1710, 0x009c71c0, 0x0058db10,
        0x004d5c60, 0x004503f0, 0x00414430, 0x00524c40, 0x00439ef0, 0x004a0c10, 0x0045bb20,
        0x00c59a50, 0x00440460, 0x00450410, 0x00458e20, 0x0087b040, 0x008bc240, 0x005fbeb0,
        0x009611e0, 0x008ae660, 0x0097f7d0, 0x00ad8b60, 0x00ad8f20, 0x00406090, 0x004a8ae0,
        0x00899cb0, 0x0089a760, 0x00437b90, 0x009b4d90, 0x008c1c30, 0x008c1c60, 0x009b4ec0,
        0x009b5120, 0x00444d00, 0x00407e00, 0x00476c90, 0x00525b20, 0x004fd3c0, 0x00885d70,
        0x0058cba0, 0x008397f0, 0x00837520, 0x004bd350, 0x009627a0, 0x006758f0, 0x00524cd0,
        0x008814b0, 0x00418a00, 0x0041a540, 0x004189c0, 0x0041a3e0, 0x00418940, 0x0041a090,
        0x00ad8930, 0x00ad8da0, 0x00ad8d10, 0x00ad88f0, 0x00ad73b0, 0x00ad7550, 0x00934250,
        0x00c8f210, 0x00c7c150, 0x008a08e0, 0x008a5eb0, 0x008a0960, 0x00815b00, 0x00568020,
        0x00ec623a, 0x00403df0, 0x0096e2f0, 0x005678a0, 0x00953f80, 0x0047e040, 0x008b8490,
        0x0094fd30, 0x00967160, 0x00525620, 0x00670700, 0x0087f9f0, 0x006705b0, 0x005be4d0,
        0x00648c80, 0x00406ce0, 0x00525420, 0x005f92d0, 0x008324e0, 0x00830680, 0x009c8950,
        0x00702450, 0x007e1980, 0x008adcb0, 0x009ca1f0, 0x0080ced0, 0x00867e30, 0x00459060,
        0x0041cda0, 0x0043fcd0, 0x005532a0, 0x0066ca00, 0x004a3e00, 0x00931fb0, 0x0087eeb0,
        0x0087eef0, 0x00630b40, 0x004586d0, 0x00458620, 0x004a0c90, 0x004b4500, 0x00812b00,
        0x0043d410, 0x00416870, 0x00a59c60, 0x00439180, 0x0062b8d0, 0x0062b930, 0x009931c0,
        0x00487f50, 0x008b01c0, 0x00565870, 0x00ca20e0, 0x00931ed0, 0x004a3a20, 0x00ca2ad0,
        0x00454af0, 0x0094ed40, 0x008b7c00, 0x0093e530, 0x00974a50, 0x009ab9a0, 0x0056af40,
        0x00421720, 0x0059f3a0, 0x005f5950, 0x008d85e0, 0x00881130,
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

    /// Gives the object whose vtable pointer is at `address` (an embedded
    /// part of a larger object) a table with one double per slot.
    fn embedded_table(e: &mut Engine, address: u32, slots: &[(u32, Ret)]) {
        let table = e.mem.alloc(0x800);
        for (offset, result) in slots {
            let target = e.mem.alloc(8);
            let result = *result;
            e.register_double(target, move |_, _| result);
            e.mem.set_u32(table + offset, target);
        }
        e.mem.set_u32(address, table);
    }

    /// Maps a `double` constant of the exe and sets it.
    fn double_global(e: &mut Engine, address: u32, value: f64) {
        e.map(address, 8);
        e.set_global(address, value);
    }

    /// An actor whose value owner (`+0xa4`) reports `current` for slot `0xc`
    /// and `base` for slot `4`.
    fn actor_with_health(
        e: &mut Engine,
        extra_slots: &[(u32, Ret)],
        current: f32,
        base: f32,
    ) -> Ptr<Actor> {
        let mut slots = extra_slots.to_vec();
        if !slots.iter().any(|(offset, _)| *offset == ACTOR_SLOT_22C) {
            slots.push((ACTOR_SLOT_22C, ret(0)));
        }
        let actor = actor_with(e, &slots, None);
        embedded_table(
            e,
            actor.addr() + 0xa4,
            &[(0x0c, float_ret(current)), (0x04, float_ret(base))],
        );
        actor
    }

    #[test]
    fn fn_008985d0_adds_a_package_by_process_type() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[(ACTOR_SLOT_ADD_PACKAGE, ret(0))], None);
        stub(&mut e, PACKAGE_CREATE, 0x5000);
        stub(&mut e, MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, 1);
        fn_008985d0(&mut e, actor);
        assert_eq!(calls_to(&e, PACKAGE_CREATE), vec![vec![0x24]]);
        assert_eq!(calls_to(&e, ACTOR_END_MOVEMENT), vec![vec![actor.addr()]]);
        assert_eq!(
            calls_to(&e, PACKAGE_CALCULATE_PROCEDURE_TYPE),
            vec![vec![0x5000, 0]]
        );
        let add = slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_PACKAGE);
        assert_eq!(calls_to(&e, add), vec![vec![actor.addr(), 0x5000, 1, 1]]);
        assert_eq!(call_count(&e, PACKAGE_SET_FLAG_B), 0);

        e.call_log = Some(vec![]);
        stub(&mut e, MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, 2);
        fn_008985d0(&mut e, actor);
        assert_eq!(calls_to(&e, PACKAGE_SET_FLAG_B), vec![vec![0x5000, 1]]);
        assert_eq!(calls_to(&e, add), vec![vec![actor.addr(), 0x5000, 0, 0]]);
    }

    /// The constants `fn_00898650` reads.
    fn kill_constants(e: &mut Engine) {
        double_global(e, QUARTER, 0.25);
        double_global(e, HALF, 0.5);
        double_global(e, ONE_DOUBLE, 1.0);
    }

    #[test]
    fn fn_00898650_kills_at_zero_health_and_barks_at_low_health() {
        let mut e = engine();
        kill_constants(&mut e);
        let attacker = actor_with(&mut e, &[], None);
        // Health 0.5 of 10: the actor is killed directly; the follower barks
        // the quarter-health bark.
        let actor = actor_with_health(&mut e, &[(ACTOR_SLOT_428, ret(0))], 0.5, 10.0);
        stub(&mut e, PROCESS_GET_FORCE_NEXT_UPDATE, 1);
        fn_00898650(&mut e, actor, attacker, 3.0);
        assert_eq!(
            calls_to(&e, TRIGGER_FOLLOWER_BARK),
            vec![vec![actor.addr(), 5]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_KILL),
            vec![vec![actor.addr(), attacker.addr(), 3.0f32.to_bits()]]
        );

        // A third of the health gives bark 4 and, being above 1, no kill.
        e.call_log = Some(vec![]);
        let actor = actor_with_health(&mut e, &[(ACTOR_SLOT_428, ret(0))], 3.0, 10.0);
        fn_00898650(&mut e, actor, attacker, 3.0);
        assert_eq!(
            calls_to(&e, TRIGGER_FOLLOWER_BARK),
            vec![vec![actor.addr(), 4]]
        );
        assert_eq!(call_count(&e, ACTOR_KILL), 0);

        // A base of 0 gives the ratio 1.0: no bark.
        e.call_log = Some(vec![]);
        let actor = actor_with_health(&mut e, &[(ACTOR_SLOT_428, ret(0))], 0.0, 0.0);
        fn_00898650(&mut e, actor, attacker, 3.0);
        assert_eq!(call_count(&e, TRIGGER_FOLLOWER_BARK), 0);
        assert_eq!(call_count(&e, ACTOR_KILL), 1);
    }

    #[test]
    fn fn_00898650_queues_the_kill_when_the_task_queue_is_used() {
        let mut e = engine();
        kill_constants(&mut e);
        let attacker = actor_with(&mut e, &[], None);
        let actor = actor_with_health(&mut e, &[(ACTOR_SLOT_428, ret(0))], 0.0, 10.0);
        stub(&mut e, PICK_UP_GOES_TO_TASK_QUEUE, 1);
        stub(&mut e, GET_TES, 0x7777);
        stub_with(&mut e, OPERATOR_NEW, |e, _| ret(e.mem.alloc(4)));
        fn_00898650(&mut e, actor, attacker, 2.5);
        let queued = calls_to(&e, QUEUE_ACTOR_KILL);
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0][..2], [0x7777, actor.addr()]);
        assert_eq!(queued[0][3], 0);
        assert_eq!(e.mem.f32(queued[0][2]), 2.5);
        assert_eq!(e.get(actor, Actor::pMyKiller).addr(), attacker.addr());
        assert_eq!(call_count(&e, ACTOR_KILL), 0);
    }

    #[test]
    fn fn_00898650_drops_the_player_as_attacker_when_the_reaction_is_two() {
        let mut e = engine();
        kill_constants(&mut e);
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        let actor = actor_with_health(&mut e, &[(ACTOR_SLOT_428, ret(0x4400))], 0.0, 10.0);
        stub(&mut e, ACTOR_GET_FACTION_FIGHT_REACTION, 2);
        fn_00898650(&mut e, actor, player, 1.0);
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET),
            vec![vec![0x4400, player.addr()]]
        );
        let reaction = calls_to(&e, ACTOR_GET_FACTION_FIGHT_REACTION);
        assert_eq!(reaction[0][..2], [actor.addr(), player.addr()]);
        assert_eq!(calls_to(&e, ACTOR_KILL)[0][1], 0);

        // Another reaction keeps the attacker.
        e.call_log = Some(vec![]);
        stub(&mut e, ACTOR_GET_FACTION_FIGHT_REACTION, 1);
        fn_00898650(&mut e, actor, player, 1.0);
        assert_eq!(calls_to(&e, ACTOR_KILL)[0][1], player.addr());

        // A combat target is never asked.
        e.call_log = Some(vec![]);
        stub(&mut e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET, 1);
        fn_00898650(&mut e, actor, player, 1.0);
        assert_eq!(call_count(&e, ACTOR_GET_FACTION_FIGHT_REACTION), 0);
    }

    #[test]
    fn fn_00898650_does_nothing_for_a_protected_actor() {
        let mut e = engine();
        kill_constants(&mut e);
        let attacker = actor_with(&mut e, &[], None);
        let actor = actor_with_health(&mut e, &[(ACTOR_SLOT_22C, ret(1))], 0.0, 10.0);
        fn_00898650(&mut e, actor, attacker, 1.0);
        assert_eq!(call_count(&e, ACTOR_KILL), 0);
        assert_eq!(call_count(&e, TRIGGER_FOLLOWER_BARK), 0);
    }

    #[test]
    fn small_flag_and_field_accessors() {
        let mut e = engine();
        let object = Ptr::<()>::new(e.mem.alloc(0x1000));
        e.mem.set_u32(object.addr() + 0x12c, 0x10);
        assert!(fn_00898fd0(&mut e, object));
        e.mem.set_u32(object.addr() + 0x12c, 0xffef);
        assert!(!fn_00898fd0(&mut e, object));
        assert_eq!(fn_0089d600(&mut e, object), object.byte_add(0x450));
        e.mem.set_u32(object.addr() + 0x2e4, 0x2e4);
        assert_eq!(fn_0089d620(&mut e, object), 0x2e4);
        fn_0089d640(&mut e, object, 1, 2, 3);
        assert_eq!(e.mem.u32(object.addr() + 0xf4), 1);
        assert_eq!(e.mem.u32(object.addr() + 0xf8), 2);
        assert_eq!(e.mem.u32(object.addr() + 0xfc), 3);
        e.mem.set_u8(object.addr() + 0x28d, 7);
        e.mem.set_u8(object.addr() + 0x28e, 8);
        e.mem.set_u32(object.addr() + 0x29c, 0x29c);
        e.mem.set_u32(object.addr() + 0x298, 0x298);
        assert_eq!(fn_0089d670(&mut e, object), 7);
        assert_eq!(fn_0089d690(&mut e, object), 8);
        assert_eq!(fn_0089d6b0(&mut e, object), 0x29c);
        assert_eq!(fn_0089d6d0(&mut e, object), 0x298);
        e.mem.set_u32(object.addr() + 0x638, 0x638);
        e.mem.set_u8(object.addr() + 0xe2d, 9);
        assert_eq!(fn_0089f4e0(&mut e, object), 0x638);
        assert_eq!(fn_0089f500(&mut e, object), 9);
        fn_0089f520(&mut e, object, 5);
        assert_eq!(e.mem.u8(object.addr() + 0x18a), 5);
        assert_eq!(fn_0089f540(&mut e, object), 5);
        e.mem.set_u32(object.addr() + 0x3c, 0xffff_ffff);
        fn_0089f560(&mut e, object);
        assert_eq!(e.mem.u32(object.addr() + 0x3c), 0);
        assert!(!fn_0089d5e0(&mut e, object));
        stub(&mut e, FORM_TEST_FLAGS, 1);
        assert!(fn_0089d5e0(&mut e, object));
        assert_eq!(
            calls_to(&e, FORM_TEST_FLAGS),
            vec![
                vec![object.addr(), 0x0800_0000],
                vec![object.addr(), 0x0800_0000]
            ]
        );
        let actor = actor_with(&mut e, &[], None);
        fn_008a0250(&mut e, actor.cast());
        assert_eq!(e.mem.u8(actor.addr() + 0xc4), 1);
    }

    #[test]
    fn fn_00898ff0_asks_the_object_inside_the_base_form() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        let base = e.mem.alloc(0x40);
        embedded_table(&mut e, base + 0x30, &[(0x18, ret(1))]);
        stub(&mut e, ACTOR_BASE_FORM, base);
        assert!(fn_00898ff0(&mut e, actor));
        embedded_table(&mut e, base + 0x30, &[(0x18, ret(0))]);
        assert!(!fn_00898ff0(&mut e, actor));
    }

    #[test]
    fn acquire_object_forwarders() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        let acquire = object(&mut e, 0x10, &[(0x4f0, ret(0x1234)), (0x4f4, ret(0))]);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        assert_eq!(fn_00899030(&mut e, actor), 0x1234);
        fn_00899060(&mut e, actor, 7);
        let slot = slot_target(&e, acquire, 0x4f4);
        assert_eq!(calls_to(&e, slot), vec![vec![acquire.addr(), 7]]);

        e.call_log = Some(vec![]);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        fn_00899060(&mut e, actor, 7);
        assert_eq!(call_count(&e, slot), 0);
    }

    #[test]
    fn blocked_attack_prints_the_message_only_when_debugging() {
        let mut e = engine();
        double_global(&mut e, HUNDRED, 100.0);
        let this = actor_with(&mut e, &[], None);
        let other = actor_with(&mut e, &[], None);
        stub_with(&mut e, REFR_GET_NAME, |_, a| ret(a[0] + 1));
        actor_handle_blocked_attack(&mut e, this, 0, 0.5, other, 0);
        assert_eq!(call_count(&e, PRINT_DEBUG_LINE), 0);

        stub(&mut e, DEBUG_MESSAGES_ENABLED, 1);
        actor_handle_blocked_attack(&mut e, this, 0, 0.5, other, 0);
        let percent = (0.5f32 as f64 * 100.0).to_bits();
        assert_eq!(
            calls_to(&e, PRINT_DEBUG_LINE),
            vec![vec![
                BLOCKS_BLOW_FORMAT,
                other.addr() + 1,
                percent as u32,
                (percent >> 32) as u32,
                this.addr() + 1
            ]]
        );
    }

    #[test]
    fn fn_008990f0_scales_the_weapon_reach() {
        let mut e = engine();
        e.map(SPECIAL_IDLE_STATE_OBJECT, 0x10);
        let actor = actor_with(
            &mut e,
            &[(ACTOR_SLOT_REACH, float_ret(2.0)), (ACTOR_SLOT_360, ret(0))],
            None,
        );
        stub_float(&mut e, REFR_GET_SCALE, 1.5);
        stub(&mut e, ACTOR_REACH_WORKER, 0x99);
        // No weapon: the actor's own reach times the scale.
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        assert_eq!(fn_008990f0(&mut e, actor), 0x99);
        let passed = calls_to(&e, ACTOR_REACH_WORKER);
        assert_eq!(passed, vec![vec![actor.addr(), 3.0f32.to_bits()]]);

        // A weapon: its reach goes through CalcWeaponReach.
        e.call_log = Some(vec![]);
        let acquire = object(&mut e, 0x10, &[(PROCESS_SLOT_CURRENT_WEAPON, ret(0x6000))]);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(&mut e, ENTRY_FORM, 0x6100);
        stub_float(&mut e, WEAPON_REACH, 4.0);
        stub_float(&mut e, CALC_WEAPON_REACH, 8.0);
        fn_008990f0(&mut e, actor);
        assert_eq!(
            calls_to(&e, CALC_WEAPON_REACH),
            vec![vec![4.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_REACH_WORKER),
            vec![vec![actor.addr(), 12.0f32.to_bits()]]
        );
    }

    #[test]
    fn fn_008990f0_multiplies_by_the_setting_for_special_idle_kind_four() {
        let mut e = engine();
        e.map(SPECIAL_IDLE_STATE_OBJECT, 0x10);
        let actor = actor_with(
            &mut e,
            &[(ACTOR_SLOT_REACH, float_ret(2.0)), (ACTOR_SLOT_360, ret(1))],
            None,
        );
        stub_float(&mut e, REFR_GET_SCALE, 1.0);
        let setting = e.mem.alloc(4);
        e.mem.set_f32(setting, 0.5);
        stub(&mut e, SETTING_GET_VALUE_POINTER, setting);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        stub_with(&mut e, ENTRY_FORM, |_, a| {
            ret(if a[0] == SPECIAL_IDLE_STATE_OBJECT {
                4
            } else {
                0
            })
        });
        fn_008990f0(&mut e, actor);
        assert_eq!(
            calls_to(&e, SETTING_GET_VALUE_POINTER),
            vec![vec![REACH_MULTIPLIER_SETTING]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_REACH_WORKER),
            vec![vec![actor.addr(), 1.0f32.to_bits()]]
        );
    }

    #[test]
    fn fn_0089d6f0_applies_health_and_fatigue_loss() {
        let mut e = engine();
        double_global(&mut e, ZERO_DOUBLE, 0.0);
        let attacker = actor_with(&mut e, &[(ACTOR_SLOT_428, ret(0x4400))], None);
        let actor = actor_with(
            &mut e,
            &[
                (ACTOR_SLOT_428, ret(0x4500)),
                (ACTOR_SLOT_360, ret(0)),
                (ACTOR_SLOT_MODIFY_ACTOR_VALUE, ret(0)),
                (ACTOR_SLOT_22C, ret(0x42)),
            ],
            None,
        );
        embedded_table(&mut e, actor.addr() + 0x94, &[(0x10, ret(0))]);
        let acquire = object(&mut e, 0x10, &[(PROCESS_SLOT_72C, ret(0))]);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        // The difficulty adjustment returns -4 for an amount of -3.
        stub_float(&mut e, DIFFICULTY_ADJUST_HEALTH_MODIFIER, -4.0);
        // The fatigue is 10 and the setting 5: the fatigue loss applies.
        embedded_table(&mut e, actor.addr() + 0xa4, &[(0x0c, float_ret(10.0))]);
        let setting = e.mem.alloc(4);
        e.mem.set_f32(setting, 5.0);
        stub(&mut e, SETTING_GET_VALUE_POINTER, setting);
        stub(&mut e, CONTROLLER_SET_FLAG, 0);
        stub(&mut e, ACTOR_GET_ESSENTIAL, 0);
        stub(&mut e, PROCESS_GET_FORCE_NEXT_UPDATE, 0);
        let process = object(&mut e, 0x10, &[(PROCESS_SLOT_22C, ret(0x4600))]);
        e.set(actor, Actor::pCurrentProcess, process);

        let result = fn_0089d6f0(&mut e, actor, 3.0, 2.0, attacker);
        assert_eq!(result, 0x42);
        assert_eq!(
            calls_to(&e, DIFFICULTY_ADJUST_HEALTH_MODIFIER),
            vec![vec![actor.addr(), (-3.0f32).to_bits(), attacker.addr()]]
        );
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_NOTIFY_DAMAGE),
            vec![vec![0x4400, actor.addr(), 4.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, CONTROLLER_RECORD_DAMAGE),
            vec![vec![0x4600, attacker.addr(), 4.0f32.to_bits()]]
        );
        let reaction = slot_target(&e, acquire, PROCESS_SLOT_72C);
        assert_eq!(
            calls_to(&e, reaction),
            vec![vec![acquire.addr(), attacker.addr(), 4.0f32.to_bits()]]
        );
        let modify = slot_target(&e, actor.cast(), ACTOR_SLOT_MODIFY_ACTOR_VALUE);
        assert_eq!(
            calls_to(&e, modify),
            vec![
                vec![actor.addr(), 0x10, (-4.0f32).to_bits(), attacker.addr()],
                vec![actor.addr(), 0x16, (-2.0f32).to_bits(), attacker.addr()],
            ]
        );
        assert_eq!(
            calls_to(&e, REFR_FLOAT_SETTER_579220),
            vec![vec![actor.addr(), 3.0f32.to_bits(), 0]]
        );
    }

    #[test]
    fn fn_0089d6f0_stops_for_a_protected_actor_and_skips_small_losses() {
        let mut e = engine();
        double_global(&mut e, ZERO_DOUBLE, 0.0);
        let actor = actor_with(&mut e, &[(ACTOR_SLOT_22C, ret(0))], None);
        embedded_table(&mut e, actor.addr() + 0x94, &[(0x10, ret(0x1201))]);
        assert_eq!(fn_0089d6f0(&mut e, actor, 3.0, 2.0, Ptr::NULL), 0x1200);
        assert_eq!(call_count(&e, DIFFICULTY_ADJUST_HEALTH_MODIFIER), 0);

        // A zero adjusted loss and a zero fatigue loss change nothing.
        embedded_table(&mut e, actor.addr() + 0x94, &[(0x10, ret(0))]);
        stub_float(&mut e, DIFFICULTY_ADJUST_HEALTH_MODIFIER, 0.0);
        fn_0089d6f0(&mut e, actor, 3.0, 0.0, Ptr::NULL);
        assert_eq!(call_count(&e, REFR_FLOAT_SETTER_579220), 0);
        assert_eq!(call_count(&e, SETTING_GET_VALUE_POINTER), 0);
    }

    #[test]
    fn worn_armor_falls_back_to_the_second_slot() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        stub_with(&mut e, ACTOR_GET_ARMOR_BEING_WORN, |_, a| {
            ret(if a[1] == 1 { 0xaa } else { 0 })
        });
        assert_eq!(actor_get_random_worn_armor(&mut e, actor, 1), 0xaa);
        assert_eq!(
            calls_to(&e, ACTOR_GET_ARMOR_BEING_WORN),
            vec![vec![actor.addr(), 0], vec![actor.addr(), 1]]
        );
        e.call_log = Some(vec![]);
        assert_eq!(actor_get_random_worn_armor(&mut e, actor, 0), 0);
        assert_eq!(
            calls_to(&e, ACTOR_GET_ARMOR_BEING_WORN),
            vec![vec![actor.addr(), 2]]
        );
    }

    /// An actor with a drawn weapon entry; `fn_0089f580` finds the weapon
    /// bone's node and drops the weapon.
    fn drop_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, Ptr) {
        let process = object(
            e,
            0x800,
            &[
                (PROCESS_SLOT_CURRENT_WEAPON, ret(0)),
                (PROCESS_SLOT_534, ret(0x7000)),
                (PROCESS_SLOT_WEAPON_BONE, ret(0)),
                (PROCESS_SLOT_580, ret(0)),
                (PROCESS_SLOT_22C, ret(0x4600)),
            ],
        );
        let actor = actor_with(
            e,
            &[
                (ACTOR_SLOT_1E8, ret(3)),
                (ACTOR_SLOT_DROP_OBJECT, ret(0x8000)),
                (ACTOR_SLOT_428, ret(1)),
            ],
            None,
        );
        e.set(actor, Actor::pCurrentProcess, process);
        let weapon = e.mem.alloc(8);
        let list = e.mem.alloc(8);
        let first = e.mem.alloc(8);
        e.mem.set_u32(weapon, list);
        e.mem.set_u32(list, first);
        stub(e, GET_SAVED_ACQUIRE_OBJECT, process.addr());
        stub(e, ACTOR_IS_WEAPON_DRAWN, 1);
        stub(e, ENTRY_FORM, 0x7100);
        stub(e, ENTRY_FIRST_EXTRA_LIST, list);
        stub_with(e, LIST_ITEM_SLOT, |_, a| ret(a[0]));
        // The process reports `weapon` as the current weapon.
        let slot = slot_target(e, process, PROCESS_SLOT_CURRENT_WEAPON);
        e.register_double(slot, move |_, _| ret(weapon));
        (actor, process, Ptr::new(weapon))
    }

    #[test]
    fn fn_0089f580_drops_the_drawn_weapon_at_the_bone() {
        let mut e = engine();
        let (actor, process, weapon) = drop_setup(&mut e);
        let node = e.mem.alloc(0x100);
        let bone = slot_target(&e, process, PROCESS_SLOT_WEAPON_BONE);
        e.register_double(bone, move |_, _| ret(node));
        let translate = e.mem.alloc(12);
        e.mem.set_f32(translate, 1.0);
        e.mem.set_f32(translate + 4, 2.0);
        e.mem.set_f32(translate + 8, 3.0);
        stub(&mut e, NODE_WORLD_TRANSLATE, translate);
        stub(&mut e, NODE_ROTATION, node + 0x68);
        stub(&mut e, GET_FORM_ENCHANTING, 0x9999);
        let seen_z = e.mem.alloc(4);
        let drop_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_DROP_OBJECT);
        e.register_double(drop_slot, move |e, a| {
            let z = e.mem.f32(a[4] + 8);
            e.mem.set_f32(seen_z, z);
            ret(0x8000)
        });
        fn_0089f580(&mut e, actor);

        let slot = slot_target(&e, process, PROCESS_SLOT_WEAPON_BONE);
        assert_eq!(calls_to(&e, slot), vec![vec![process.addr(), 3]]);
        let angles = calls_to(&e, NI_MATRIX3_TO_EULER_ANGLES_XYZ);
        assert_eq!(angles.len(), 1);
        assert_eq!(angles[0][0], node + 0x68);
        assert_eq!(angles[0][2], angles[0][1] + 4);
        let drop = calls_to(&e, drop_slot);
        assert_eq!(drop.len(), 1);
        // this, form, extra data list, count, position, angles
        assert_eq!(
            drop[0][..4],
            [actor.addr(), 0x7100, e.mem.u32(e.mem.u32(weapon.addr())), 1]
        );
        assert_eq!(e.mem.f32(seen_z), 3.0);
        assert_eq!(drop[0][5], angles[0][1]);
        assert_eq!(
            calls_to(&e, ACTOR_DROPPED_OBJECT_WORKER),
            vec![vec![actor.addr(), 0x8000]]
        );
        // The form is enchanted: the shader effects slot runs.
        let effects = slot_target(&e, process, PROCESS_SLOT_580);
        assert_eq!(calls_to(&e, effects), vec![vec![process.addr(), 1, 0, 0]]);
        assert_eq!(calls_to(&e, CONTROLLER_SET_FLAG), vec![vec![0x4600, 0]]);
    }

    #[test]
    fn fn_0089f580_stops_at_each_blocking_condition() {
        // The form is the one equipped in the other hand.
        let mut e = engine();
        let (actor, _, _) = drop_setup(&mut e);
        stub(&mut e, ENTRY_FORM, 0x7000);
        fn_0089f580(&mut e, actor);
        assert_eq!(call_count(&e, GET_FORM_ENCHANTING), 0);
        assert_eq!(call_count(&e, CONTROLLER_SET_FLAG), 0);

        // The weapon cannot be worn.
        let mut e = engine();
        let (actor, _, _) = drop_setup(&mut e);
        stub(&mut e, EXTRA_GET_CAN_NOT_WEAR, 1);
        fn_0089f580(&mut e, actor);
        assert_eq!(call_count(&e, GET_FORM_ENCHANTING), 0);

        // Not drawn.
        let mut e = engine();
        let (actor, _, _) = drop_setup(&mut e);
        stub(&mut e, ACTOR_IS_WEAPON_DRAWN, 0);
        fn_0089f580(&mut e, actor);
        assert_eq!(call_count(&e, EXTRA_GET_CAN_NOT_WEAR), 0);

        // The form flags.
        let mut e = engine();
        let (actor, _, _) = drop_setup(&mut e);
        stub(&mut e, FORM_FLAG_100_BIT_3, 1);
        fn_0089f580(&mut e, actor);
        assert_eq!(call_count(&e, FORM_FLAG_100_BIT_5), 0);
        let mut e = engine();
        let (actor, _, _) = drop_setup(&mut e);
        stub(&mut e, FORM_FLAG_100_BIT_5, 1);
        fn_0089f580(&mut e, actor);
        assert_eq!(call_count(&e, ACTOR_IS_WEAPON_DRAWN), 0);

        // Without a bone node nothing is dropped but the process is told.
        let mut e = engine();
        let (actor, _, _) = drop_setup(&mut e);
        fn_0089f580(&mut e, actor);
        assert_eq!(call_count(&e, ACTOR_DROPPED_OBJECT_WORKER), 0);
        assert_eq!(call_count(&e, CONTROLLER_SET_FLAG), 1);
    }

    #[test]
    fn fn_0089fb80_refreshes_the_worn_items_by_form_type() {
        let mut e = engine();
        e.map(BASE_DATA_OBJECT, 4);
        e.set_global(BASE_DATA_OBJECT, 0x5a5a_0000u32);
        let actor = actor_with(
            &mut e,
            &[(ACTOR_SLOT_1CC, ret(0)), (ACTOR_SLOT_1D0, ret(0x9000))],
            None,
        );
        stub(&mut e, GET_CONTAINER_CHANGES, 0x6100);
        stub(&mut e, PATH_MANAGER_Q_INSTANCE, 0x6200);
        stub(&mut e, REFR_BASE_FORM, 0x6300);
        stub(&mut e, FORM_TYPE, 0x2a);
        fn_0089fb80(&mut e, actor, 7);
        // No acquire object: the actor's own slot 0x1cc is called.
        let refresh = slot_target(&e, actor.cast(), ACTOR_SLOT_1CC);
        assert_eq!(calls_to(&e, refresh), vec![vec![actor.addr(), 0, 1]]);
        assert_eq!(calls_to(&e, BASE_DATA_PREDICATE), vec![vec![0x5a5a_0000]]);
        assert_eq!(
            calls_to(&e, PATH_MANAGER_CLEAR_INVENTORY_POINTERS),
            vec![vec![0x6200, 0x6100]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_UPDATE_INVENTORY_WORKER),
            vec![vec![actor.addr(), 7]]
        );
        assert_eq!(
            calls_to(&e, NPC_INIT_DEFAULT_WORN),
            vec![vec![0x6300, actor.addr(), 1, 1, 0, 1]]
        );
        assert_eq!(call_count(&e, CREATURE_INIT_DEFAULT_WORN), 0);

        // A creature form, and a current package of the accepted kind.
        e.call_log = Some(vec![]);
        stub(&mut e, FORM_TYPE, 0x2b);
        stub(&mut e, GET_CURRENT_PACKAGE, 0x6400);
        stub(&mut e, PACKAGE_KIND_TEST, 1);
        fn_0089fb80(&mut e, actor, 0);
        assert_eq!(
            calls_to(&e, CREATURE_INIT_DEFAULT_WORN),
            vec![vec![0x6300, actor.addr(), 1, 0, 1]]
        );
        assert_eq!(call_count(&e, NPC_INIT_DEFAULT_WORN), 0);
    }

    #[test]
    fn fn_0089fb80_prefers_the_acquire_object_and_honours_the_predicate() {
        let mut e = engine();
        e.map(BASE_DATA_OBJECT, 4);
        let actor = actor_with(&mut e, &[(ACTOR_SLOT_1D0, ret(0))], None);
        let acquire = object(&mut e, 0x10, &[(PROCESS_SLOT_468, ret(0))]);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(&mut e, ACQUIRE_OBJECT_FIELD_28, 1);
        fn_0089fb80(&mut e, actor, 0);
        let slot = slot_target(&e, acquire, PROCESS_SLOT_468);
        assert_eq!(calls_to(&e, slot), vec![vec![acquire.addr(), 1]]);

        // A field above 1 uses the actor's slot (not present in this vtable:
        // the predicate is set instead, so nothing is refreshed at all).
        e.call_log = Some(vec![]);
        stub(&mut e, ACQUIRE_OBJECT_FIELD_28, 2);
        stub(&mut e, BASE_DATA_PREDICATE, 1);
        fn_0089fb80(&mut e, actor, 0);
        assert_eq!(call_count(&e, slot), 0);
        assert_eq!(call_count(&e, GET_SAVED_ACQUIRE_OBJECT), 0);
        assert_eq!(call_count(&e, NPC_INIT_DEFAULT_WORN), 0);
    }

    /// An actor ready for `fn_0089fcf0`: no combat controller yet, a process
    /// object and the constants the function reads.
    fn combat_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, Ptr<Actor>, Ptr) {
        double_global(e, ZERO_DOUBLE, 0.0);
        double_global(e, QUARTER, 0.25);
        e.map(COMBAT_MANAGER, 4);
        e.set_global(COMBAT_MANAGER, 0x7100u32);
        let process = object(
            e,
            0x800,
            &[
                (PROCESS_SLOT_2C8, ret(0)),
                (PROCESS_SLOT_22C, ret(0)),
                (ACTOR_SLOT_214, ret(0)),
            ],
        );
        let actor = actor_with(
            e,
            &[
                (ACTOR_SLOT_22C, ret(0)),
                (ACTOR_SLOT_214, ret(0)),
                (ACTOR_SLOT_REACH, float_ret(10.0)),
                (ACTOR_SLOT_428, ret(0)),
                (ACTOR_SLOT_3BC, ret(0)),
                (ACTOR_SLOT_ADD_PACKAGE, ret(0)),
                (ACTOR_SLOT_418, ret(0)),
            ],
            None,
        );
        e.set(actor, Actor::pCurrentProcess, process);
        let target = actor_with(e, &[], None);
        let acquire = object(
            e,
            0x10,
            &[
                (PROCESS_SLOT_208, float_ret(55.7)),
                (PROCESS_SLOT_28, ret(0)),
                (PROCESS_SLOT_710, ret(0)),
                (PROCESS_SLOT_52C, ret(0)),
                (PROCESS_SLOT_4D4, ret(0)),
            ],
        );
        stub(e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        (actor, process, target, acquire)
    }

    #[test]
    fn fn_0089fcf0_adds_a_combatant_and_starts_the_package() {
        let mut e = engine();
        let (actor, process, target, acquire) = combat_setup(&mut e);
        let controller = e.mem.alloc(0x100);
        stub_with(&mut e, FTOL, |_, a| {
            ret(f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32) as i32 as u32)
        });
        stub(&mut e, COMBAT_MANAGER_ADD_COMBATANT, controller);
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 1, 0, 0, 0);
        assert_eq!(
            calls_to(&e, COMBAT_MANAGER_ADD_COMBATANT),
            vec![vec![0x7100, actor.addr(), target.addr(), 55, 0]]
        );
        let slot_208 = slot_target(&e, acquire, PROCESS_SLOT_208);
        assert_eq!(
            calls_to(&e, slot_208),
            vec![vec![acquire.addr(), target.addr(), actor.addr()]]
        );
        // flag_b set: the controller is told about the target.
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_NOTIFY_DAMAGE),
            vec![vec![controller, target.addr(), 0]]
        );
        let add_package = slot_target(&e, actor.cast(), ACTOR_SLOT_ADD_PACKAGE);
        assert_eq!(
            calls_to(&e, add_package),
            vec![vec![actor.addr(), controller, 0, 1]]
        );
        let slot_710 = slot_target(&e, acquire, PROCESS_SLOT_710);
        assert_eq!(
            calls_to(&e, slot_710),
            vec![vec![acquire.addr(), actor.addr()]]
        );
        assert_eq!(e.mem.u8(actor.addr() + 0x104), 1);
        let hostile = slot_target(&e, process, PROCESS_SLOT_2C8);
        assert_eq!(calls_to(&e, hostile), vec![vec![process.addr(), 0]]);
        // The controller was not marked (flag_a is 0).
        assert_eq!(e.mem.u8(controller + 0xc4), 0);
    }

    #[test]
    fn fn_0089fcf0_adds_a_group_member_or_a_pretend_combatant() {
        let mut e = engine();
        let (actor, _, target, _) = combat_setup(&mut e);
        stub(&mut e, COMBAT_MANAGER_ADD_GROUP_MEMBER, 0);
        fn_0089fcf0(&mut e, actor, target, 0x55, 0, 0, 0, 3, 0, 0x66);
        assert_eq!(
            calls_to(&e, COMBAT_MANAGER_ADD_GROUP_MEMBER),
            vec![vec![0x7100, actor.addr(), 0x55]]
        );
        assert_eq!(call_count(&e, COMBAT_MANAGER_ADD_COMBATANT), 0);
        // No controller came back: the combat flag is cleared.
        assert_eq!(e.mem.u8(actor.addr() + 0x104), 0);

        e.call_log = Some(vec![]);
        stub(&mut e, COMBAT_MANAGER_ADD_PRETEND_COMBATANT, 0);
        fn_0089fcf0(&mut e, actor, Ptr::NULL, 0, 0, 0, 0, 3, 0, 0x66);
        assert_eq!(
            calls_to(&e, COMBAT_MANAGER_ADD_PRETEND_COMBATANT),
            vec![vec![0x7100, actor.addr(), 0x66]]
        );
    }

    #[test]
    fn fn_0089fcf0_marks_the_controller_and_clears_the_crosshair_for_the_player() {
        let mut e = engine();
        let (actor, _, _, _) = combat_setup(&mut e);
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        let controller = e.mem.alloc(0x100);
        stub(&mut e, COMBAT_MANAGER_ADD_COMBATANT, controller);
        stub(&mut e, CROSSHAIR_TARGET, actor.addr());
        fn_0089fcf0(&mut e, actor, player, 0, 1, 0, 0, 5, 0, 0);
        assert_eq!(e.mem.u8(controller + 0xc4), 1);
        assert_eq!(
            calls_to(&e, PLAYER_CHANGE_PERCEIVED_HOSTILE_STATUS),
            vec![vec![player.addr(), actor.addr(), 1]]
        );
        assert_eq!(calls_to(&e, SET_CROSSHAIR_TARGET_TYPE), vec![vec![0]]);
        // A positive value is passed on with the "ranked" flag.
        assert_eq!(
            calls_to(&e, COMBAT_MANAGER_ADD_COMBATANT),
            vec![vec![0x7100, actor.addr(), player.addr(), 5, 1]]
        );
    }

    #[test]
    fn fn_0089fcf0_queues_the_held_item_again() {
        let mut e = engine();
        let (actor, process, target, _) = combat_setup(&mut e);
        let held = e.mem.alloc(0x40);
        let process_slot = slot_target(&e, process, PROCESS_SLOT_22C);
        e.register_double(process_slot, move |_, _| ret(held));
        stub(&mut e, PACKAGE_KIND_TEST, 1);
        let item_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_3BC);
        e.register_double(item_slot, |_, _| ret(0x7700));
        stub(&mut e, ENTRY_FORM, 0x7800);
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 0, 1, 0, 0);
        assert_eq!(calls_to(&e, item_slot), vec![vec![actor.addr(), 6]]);
        assert_eq!(
            calls_to(&e, ACTOR_QUEUE_EQUIP_OBJECT),
            vec![vec![actor.addr(), 0x7800, 1, 0, 1, 0, 1]]
        );
        assert_eq!(calls_to(&e, ENTRY_DELETE), vec![vec![0x7700, 1]]);
        assert_eq!(
            calls_to(&e, ACTOR_END_INTERRUPT_PACKAGE),
            vec![vec![actor.addr(), 0]]
        );
    }

    #[test]
    fn fn_0089fcf0_adds_the_target_to_an_existing_controller() {
        let mut e = engine();
        let (actor, process, target, _) = combat_setup(&mut e);
        let controller_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(0x9200));
        let process_slot = slot_target(&e, process, PROCESS_SLOT_22C);
        e.register_double(process_slot, |_, _| ret(0x9300));
        stub(&mut e, COMBAT_CONTROLLER_TARGET, 0x1111);
        fn_0089fcf0(&mut e, actor, target, 0x44, 0, 0, 0, 4, 0, 0);
        let float_zero = 0.0f32.to_bits();
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_ADD_TARGET),
            vec![vec![0x9300, target.addr(), 4, 1, float_zero, float_zero]]
        );
        assert_eq!(call_count(&e, COMBAT_MANAGER_ADD_GROUP_MEMBER), 0);

        // The same target again: only the group member is added.
        e.call_log = Some(vec![]);
        stub(&mut e, COMBAT_CONTROLLER_TARGET, target.addr());
        fn_0089fcf0(&mut e, actor, target, 0x44, 0, 0, 0, 4, 0, 0);
        assert_eq!(call_count(&e, COMBAT_CONTROLLER_ADD_TARGET), 0);
        assert_eq!(
            calls_to(&e, COMBAT_MANAGER_ADD_GROUP_MEMBER),
            vec![vec![0x7100, actor.addr(), 0x44]]
        );
    }

    #[test]
    fn fn_0089fcf0_refuses_in_the_cases_the_code_lists() {
        // Process level 5.
        let mut e = engine();
        let (actor, _, target, _) = combat_setup(&mut e);
        stub(&mut e, ACTOR_PROCESS_LEVEL_IS_5, 1);
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 0, 1, 0, 0);
        assert_eq!(call_count(&e, COMBAT_MANAGER_ADD_COMBATANT), 0);
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 0);

        // The target is the actor itself.
        let mut e = engine();
        let (actor, _, _, _) = combat_setup(&mut e);
        fn_0089fcf0(&mut e, actor, actor, 0, 0, 0, 0, 1, 0, 0);
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 0);

        // Dead.
        let mut e = engine();
        let (actor, _, target, _) = combat_setup(&mut e);
        e.mem.set_u8(actor.addr() + 0x118, 1);
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 0, 1, 0, 0);
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 0);

        // Too far away: distance 20 against reach 10 plus a quarter.
        let mut e = engine();
        let (actor, _, target, _) = combat_setup(&mut e);
        stub_float(&mut e, REFR_GET_DISTANCE_FROM_REFERENCE, 20.0);
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 0, 1, 0, 0);
        assert_eq!(
            calls_to(&e, REFR_GET_DISTANCE_FROM_REFERENCE),
            vec![vec![actor.addr(), target.addr(), 0, 0]]
        );
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 0);
        // Distance 12.5 is within 10 + 2.5.
        stub_float(&mut e, REFR_GET_DISTANCE_FROM_REFERENCE, 12.5);
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 0, 1, 0, 0);
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 1);

        // A busy state (3) without a furniture entry above 0x14 stops it.
        let mut e = engine();
        let (actor, _, target, acquire) = combat_setup(&mut e);
        let kind = slot_target(&e, actor.cast(), ACTOR_SLOT_214);
        e.register_double(kind, |_, _| ret(3));
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 0, 1, 0, 0);
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 0);
        // A furniture entry whose byte is 0x15 lets it go on, after slot 0x418.
        let entry = e.mem.alloc(0x20);
        e.mem.set_u8(entry + 0xe, 0x15);
        let furniture = slot_target(&e, acquire, PROCESS_SLOT_4D4);
        e.register_double(furniture, move |_, _| ret(entry));
        fn_0089fcf0(&mut e, actor, target, 0, 0, 0, 0, 1, 0, 0);
        let slot_418 = slot_target(&e, actor.cast(), ACTOR_SLOT_418);
        assert_eq!(call_count(&e, slot_418), 1);
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 1);
    }

    #[test]
    fn fn_008a0270_reports_the_combat_flag() {
        let mut e = engine();
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        let actor = actor_with(&mut e, &[], None);
        // No process: 0.
        e.mem.set_u8(actor.addr() + 0x104, 1);
        assert_eq!(fn_008a0270(&mut e, actor, 0), 0);
        let process = e.mem.alloc(8);
        e.set(actor, Actor::pCurrentProcess, Ptr::new(process));
        assert_eq!(fn_008a0270(&mut e, actor, 0), 1);
        // Searching in combat is ignored on request.
        e.mem.set_u8(actor.addr() + 0x127, 1);
        assert_eq!(fn_008a0270(&mut e, actor, 0), 1);
        assert_eq!(fn_008a0270(&mut e, actor, 1), 0);
        // The player is never "in combat" here.
        e.set(player, Actor::pCurrentProcess, Ptr::new(process));
        e.mem.set_u8(player.addr() + 0x104, 1);
        assert_eq!(fn_008a0270(&mut e, player, 0), 0);
    }

    #[test]
    fn fn_008a02d0_returns_the_controller_of_type_0x12() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[], None);
        assert_eq!(fn_008a02d0(&mut e, actor), 0);
        let process = object(&mut e, 0x800, &[(PROCESS_SLOT_22C, ret(0x9300))]);
        e.set(actor, Actor::pCurrentProcess, process);
        // Not in combat.
        assert_eq!(fn_008a02d0(&mut e, actor), 0);
        stub(&mut e, ACTOR_IN_COMBAT, 1);
        stub(&mut e, PACKAGE_TYPE, 0x11);
        assert_eq!(fn_008a02d0(&mut e, actor), 0);
        stub(&mut e, PACKAGE_TYPE, 0x12);
        assert_eq!(fn_008a02d0(&mut e, actor), 0x9300);
        let none = slot_target(&e, process, PROCESS_SLOT_22C);
        e.register_double(none, |_, _| ret(0));
        assert_eq!(fn_008a02d0(&mut e, actor), 0);
    }

    #[test]
    fn small_combat_controller_forwarders() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[(ACTOR_SLOT_428, ret(0))], None);
        assert_eq!(fn_008a0360(&mut e, actor), 0);
        fn_008a0680(&mut e, actor, 1, 2);
        assert_eq!(call_count(&e, CONTROLLER_FORWARD_97EFE0), 0);
        let controller_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(0x9400));
        stub(&mut e, COMBAT_CONTROLLER_TARGET, 0x1234);
        assert_eq!(fn_008a0360(&mut e, actor), 0x1234);
        fn_008a0680(&mut e, actor, 1, 2);
        assert_eq!(
            calls_to(&e, CONTROLLER_FORWARD_97EFE0),
            vec![vec![0x9400, 1, 2]]
        );
    }

    #[test]
    fn fn_008a05f0_picks_a_process_kind_without_a_controller() {
        let mut e = engine();
        let process = object(&mut e, 0x800, &[(PROCESS_SLOT_54, ret(0))]);
        let actor = actor_with(&mut e, &[(ACTOR_SLOT_428, ret(0))], None);
        e.set(actor, Actor::pCurrentProcess, process);
        let slot = slot_target(&e, process, PROCESS_SLOT_54);
        for (field, kind) in [(1, 1), (2, 2), (3, 4), (0, 4)] {
            e.call_log = Some(vec![]);
            stub(&mut e, ACQUIRE_OBJECT_FIELD_28, field);
            fn_008a05f0(&mut e, actor);
            assert_eq!(
                calls_to(&e, slot),
                vec![vec![process.addr(), actor.addr(), kind]]
            );
        }
        // With a controller only the do-nothing function is called.
        e.call_log = Some(vec![]);
        let controller_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(1));
        fn_008a05f0(&mut e, actor);
        assert_eq!(call_count(&e, slot), 0);
        assert_eq!(calls_to(&e, ERROR_NO_OP), vec![vec![actor.addr()]]);
    }

    /// An actor with a combat controller for `fn_008a03a0`.
    fn keep_combat_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr, u32) {
        double_global(e, ZERO_DOUBLE, 0.0);
        e.map(FADER_MANAGER, 4);
        e.set_global(FADER_MANAGER, 0x7200u32);
        let target = actor_with(
            e,
            &[(ACTOR_SLOT_22C, ret(0)), (ACTOR_SLOT_2E8, ret(0))],
            None,
        );
        let process = object(e, 0x800, &[(PROCESS_SLOT_22C, ret(0x9500))]);
        let acquire = object(
            e,
            0x10,
            &[(PROCESS_SLOT_64C, ret(0)), (PROCESS_SLOT_630, ret(0))],
        );
        let actor = actor_with(
            e,
            &[
                (ACTOR_SLOT_428, ret(0x9500)),
                (ACTOR_SLOT_STOP_COMBAT, ret(0)),
            ],
            None,
        );
        e.set(actor, Actor::pCurrentProcess, process);
        stub(e, ACTOR_IN_COMBAT, 1);
        stub(e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(e, COMBAT_CONTROLLER_TARGET, target.addr());
        stub(e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET, 1);
        (actor, acquire, target.addr())
    }

    #[test]
    fn fn_008a03a0_restarts_the_combat_controller() {
        let mut e = engine();
        let (actor, acquire, target) = keep_combat_setup(&mut e);
        stub(&mut e, MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, 0);
        fn_008a03a0(&mut e, actor);
        assert_eq!(
            calls_to(&e, FADER_MANAGER_GET_FADER_ALPHA),
            vec![vec![0x7200, 1]]
        );
        let slot_630 = slot_target(&e, acquire.cast(), PROCESS_SLOT_630);
        assert_eq!(calls_to(&e, slot_630), vec![vec![acquire.addr(), target]]);
        assert_eq!(calls_to(&e, CONTROLLER_START_HIGH), vec![vec![0x9500]]);
        assert_eq!(call_count(&e, CONTROLLER_START_OTHER), 0);
        assert_eq!(call_count(&e, COMBAT_START_GUARD_NO_OP), 2);

        // Another process level, and the controller test 9818b0.
        e.call_log = Some(vec![]);
        stub(&mut e, MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, 2);
        stub(&mut e, CONTROLLER_TEST_9818B0, 1);
        fn_008a03a0(&mut e, actor);
        let slot_64c = slot_target(&e, acquire.cast(), PROCESS_SLOT_64C);
        assert_eq!(calls_to(&e, slot_64c), vec![vec![acquire.addr()]]);
        assert_eq!(calls_to(&e, CONTROLLER_START_OTHER), vec![vec![0x9500]]);
    }

    #[test]
    fn fn_008a03a0_adds_the_actor_to_the_players_combat_list() {
        let mut e = engine();
        let (actor, _, _) = keep_combat_setup(&mut e);
        let player = actor_with(
            &mut e,
            &[(ACTOR_SLOT_22C, ret(0)), (ACTOR_SLOT_2E8, ret(0))],
            None,
        );
        set_player(&mut e, player);
        stub(&mut e, COMBAT_CONTROLLER_TARGET, player.addr());
        stub(&mut e, PLAYER_COMBAT_LIST_COUNT, 2);
        fn_008a03a0(&mut e, actor);
        assert_eq!(
            calls_to(&e, PLAYER_ADD_ACTOR_TO_COMBAT_LIST),
            vec![vec![player.addr(), actor.addr()]]
        );
        // The player's refusal stops everything.
        e.call_log = Some(vec![]);
        stub(&mut e, PLAYER_TEST_5A03F0, 1);
        fn_008a03a0(&mut e, actor);
        assert_eq!(
            call_count(&e, CONTROLLER_START_HIGH) + call_count(&e, CONTROLLER_START_OTHER),
            0
        );
    }

    #[test]
    fn fn_008a03a0_stops_the_combat_in_the_cases_the_code_lists() {
        // Visible fader: nothing.
        let mut e = engine();
        let (actor, _, _) = keep_combat_setup(&mut e);
        stub_float(&mut e, FADER_MANAGER_GET_FADER_ALPHA, 0.5);
        fn_008a03a0(&mut e, actor);
        assert_eq!(call_count(&e, ACTOR_TEST_576D30), 0);

        // The controller says to stop: slot 0x434 with 0.
        let mut e = engine();
        let (actor, _, _) = keep_combat_setup(&mut e);
        stub(&mut e, CONTROLLER_TEST_981450, 1);
        fn_008a03a0(&mut e, actor);
        let stop = slot_target(&e, actor.cast(), ACTOR_SLOT_STOP_COMBAT);
        assert_eq!(calls_to(&e, stop), vec![vec![actor.addr(), 0]]);
        assert_eq!(call_count(&e, CONTROLLER_START_HIGH), 0);

        // The target is no longer a combat target: stop with it, and give up
        // when the actor has no controller afterwards.
        let mut e = engine();
        let (actor, _, target) = keep_combat_setup(&mut e);
        stub(&mut e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET, 0);
        let controller_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(0));
        fn_008a03a0(&mut e, actor);
        let stop = slot_target(&e, actor.cast(), ACTOR_SLOT_STOP_COMBAT);
        assert_eq!(calls_to(&e, stop), vec![vec![actor.addr(), target]]);
        assert_eq!(call_count(&e, CONTROLLER_START_HIGH), 0);

        // A protected target that is not "2e8": stop with it.
        let mut e = engine();
        let (actor, _, target) = keep_combat_setup(&mut e);
        let protected = slot_target(&e, Ptr::new(target), ACTOR_SLOT_22C);
        e.register_double(protected, |_, _| ret(1));
        fn_008a03a0(&mut e, actor);
        let stop = slot_target(&e, actor.cast(), ACTOR_SLOT_STOP_COMBAT);
        assert_eq!(calls_to(&e, stop), vec![vec![actor.addr(), target]]);
        // ... but a "2e8" target is not stopped by that alone.
        e.call_log = Some(vec![]);
        let flagged = slot_target(&e, Ptr::new(target), ACTOR_SLOT_2E8);
        e.register_double(flagged, |_, _| ret(1));
        fn_008a03a0(&mut e, actor);
        assert_eq!(call_count(&e, stop), 0);
        // The three other tests each stop it too.
        for test in [
            ACTOR_TEST_440DA0,
            ACTOR_TEST_440D80,
            ACTOR_PROCESS_LEVEL_IS_3,
        ] {
            e.call_log = Some(vec![]);
            stub(&mut e, test, 1);
            fn_008a03a0(&mut e, actor);
            assert_eq!(calls_to(&e, stop), vec![vec![actor.addr(), target]]);
            stub(&mut e, test, 0);
        }
    }

    #[test]
    fn actor_resurrect_brings_the_actor_back_with_new_process_data() {
        let mut e = engine();
        e.map(TES_SINGLETON, 4);
        e.set_global(TES_SINGLETON, 0x7300u32);
        e.map(MODEL_LOADER, 4);
        e.set_global(MODEL_LOADER, 0x7400u32);
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        let old_process = object(&mut e, 0x800, &[(0, ret(0))]);
        let actor = actor_with(
            &mut e,
            &[
                (ACTOR_SLOT_SET_DELETE, ret(0)),
                (ACTOR_SLOT_4C, ret(0)),
                (ACTOR_SLOT_1D0, ret(0)),
                (ACTOR_SLOT_208, ret(0)),
                (ACTOR_SLOT_240, ret(0)),
                (ACTOR_SLOT_248, ret(0)),
                (ACTOR_SLOT_24C, ret(0)),
            ],
            None,
        );
        e.set(actor, Actor::pCurrentProcess, old_process);
        e.mem.set_u8(actor.addr() + 0x174, 1);
        e.mem.set_u8(actor.addr() + 0x118, 1);
        stub(&mut e, ACQUIRE_OBJECT_FIELD_28, 2);
        stub_with(&mut e, OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        stub_with(&mut e, NEW_PROCESS_CONSTRUCT, |_, a| ret(a[0]));
        stub(&mut e, MOBILE_OBJECT_GET_DESIRED_PROCESS_LEVEL, 2);
        actor_resurrect(&mut e, actor, 1, 0, 0);
        let slot = |e: &Engine, offset: u32| slot_target(e, actor.cast(), offset);
        assert_eq!(
            calls_to(&e, slot(&e, ACTOR_SLOT_SET_DELETE)),
            vec![vec![actor.addr(), 0]]
        );
        assert_eq!(
            calls_to(&e, slot(&e, ACTOR_SLOT_4C)),
            vec![vec![actor.addr(), 0x20000], vec![actor.addr(), 4]]
        );
        assert_eq!(e.mem.u8(actor.addr() + 0x174), 0);
        assert_eq!(e.mem.u8(actor.addr() + 0x118), 0);
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_REMOVE_REFERENCE),
            vec![vec![PROCESS_LISTS, actor.addr(), 2]]
        );
        // The old process object is deleted, a new one built and installed.
        assert_eq!(
            calls_to(&e, slot_target(&e, old_process, 0)),
            vec![vec![old_process.addr(), 1]]
        );
        let new_process = e.get(actor, Actor::pCurrentProcess).addr();
        assert_ne!(new_process, old_process.addr());
        assert_eq!(calls_to(&e, NEW_PROCESS_CONSTRUCT), vec![vec![new_process]]);
        assert_eq!(calls_to(&e, OPERATOR_NEW), vec![vec![0xb4]]);
        assert_eq!(
            calls_to(&e, MODIFIER_LIST_DELETE_ALL),
            vec![vec![actor.addr() + 0xd0], vec![actor.addr() + 0xe0]]
        );
        assert_eq!(
            calls_to(&e, slot(&e, ACTOR_SLOT_208)),
            vec![vec![actor.addr(), 0]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_LIFE_STATE),
            vec![vec![actor.addr(), 0]]
        );
        // Desired level 2 uses slot 0x248; the lock is taken and released.
        assert_eq!(call_count(&e, slot(&e, ACTOR_SLOT_248)), 1);
        assert_eq!(calls_to(&e, LOCK_ENTER), vec![vec![RESURRECT_LOCK, 0]]);
        assert_eq!(calls_to(&e, LOCK_LEAVE), vec![vec![RESURRECT_LOCK]]);
        assert_eq!(
            calls_to(&e, EXTRA_REMOVE_PLAYER_CRIME_LIST_EXTRA),
            vec![vec![0, actor.addr()]]
        );
    }

    #[test]
    fn actor_resurrect_picks_the_process_level_switch() {
        for (level, slot) in [
            (0, ACTOR_SLOT_240),
            (1, ACTOR_SLOT_24C),
            (2, ACTOR_SLOT_248),
        ] {
            let mut e = engine();
            e.map(TES_SINGLETON, 4);
            let player = actor_with(
                &mut e,
                &[(ACTOR_SLOT_SET_DELETE, ret(0)), (ACTOR_SLOT_4C, ret(0))],
                None,
            );
            set_player(&mut e, player);
            let actor = actor_with(
                &mut e,
                &[
                    (ACTOR_SLOT_4C, ret(0)),
                    (ACTOR_SLOT_1D0, ret(0)),
                    (ACTOR_SLOT_240, ret(0)),
                    (ACTOR_SLOT_248, ret(0)),
                    (ACTOR_SLOT_24C, ret(0)),
                    (ACTOR_SLOT_SET_DELETE, ret(0)),
                ],
                None,
            );
            stub_with(&mut e, OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
            stub(&mut e, MOBILE_OBJECT_GET_DESIRED_PROCESS_LEVEL, level);
            actor_resurrect(&mut e, actor, 0, 0, 0);
            let target = slot_target(&e, actor.cast(), slot);
            assert_eq!(call_count(&e, target), 1, "level {level}");
        }
        // Level 3 goes to the process lists; the player skips the switch.
        let mut e = engine();
        e.map(TES_SINGLETON, 4);
        let player = actor_with(
            &mut e,
            &[(ACTOR_SLOT_SET_DELETE, ret(0)), (ACTOR_SLOT_4C, ret(0))],
            None,
        );
        set_player(&mut e, player);
        let actor = actor_with(
            &mut e,
            &[
                (ACTOR_SLOT_4C, ret(0)),
                (ACTOR_SLOT_1D0, ret(0)),
                (ACTOR_SLOT_SET_DELETE, ret(0)),
            ],
            None,
        );
        stub_with(&mut e, OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        stub(&mut e, MOBILE_OBJECT_GET_DESIRED_PROCESS_LEVEL, 3);
        actor_resurrect(&mut e, actor, 0, 0, 0);
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_ADD_REFERENCE),
            vec![vec![PROCESS_LISTS, actor.addr(), 3, 0, 0, 0]]
        );
        e.call_log = Some(vec![]);
        actor_resurrect(&mut e, player, 0, 0, 0);
        assert_eq!(call_count(&e, MOBILE_OBJECT_GET_DESIRED_PROCESS_LEVEL), 0);
        assert_eq!(call_count(&e, PROCESS_LISTS_REMOVE_REFERENCE), 0);
    }

    #[test]
    fn actor_resurrect_queues_the_load_of_a_loaded_cell() {
        let mut e = engine();
        e.map(TES_SINGLETON, 4);
        e.set_global(TES_SINGLETON, 0x7300u32);
        e.map(MODEL_LOADER, 4);
        e.set_global(MODEL_LOADER, 0x7400u32);
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        let actor = actor_with(
            &mut e,
            &[
                (ACTOR_SLOT_4C, ret(0)),
                (ACTOR_SLOT_1D0, ret(0)),
                (ACTOR_SLOT_240, ret(0)),
                (ACTOR_SLOT_24C, ret(0)),
                (ACTOR_SLOT_SET_DELETE, ret(0)),
            ],
            None,
        );
        stub_with(&mut e, OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        stub(&mut e, REFR_PARENT_CELL, 0x7500);
        stub(&mut e, TES_IS_CELL_LOADED, 1);
        stub(&mut e, TES_GET_CELL_PRIORITY, 0x33);
        actor_resurrect(&mut e, actor, 0, 1, 0);
        assert_eq!(
            calls_to(&e, TES_IS_CELL_LOADED),
            vec![vec![0x7300, 0x7500, 0]]
        );
        assert_eq!(
            calls_to(&e, TES_GET_CELL_PRIORITY),
            vec![vec![0x7300, 0x7500, 0, 0]]
        );
        assert_eq!(
            calls_to(&e, MODEL_LOADER_QUEUE_REFERENCE),
            vec![vec![0x7400, actor.addr(), 0x33]]
        );
        // The cell test picks slot 0x24c (false) here.
        let low = slot_target(&e, actor.cast(), ACTOR_SLOT_24C);
        assert_eq!(call_count(&e, low), 1);
        assert_eq!(call_count(&e, MOBILE_OBJECT_GET_DESIRED_PROCESS_LEVEL), 0);
        assert_eq!(call_count(&e, LOCK_LEAVE), 1);
    }

    #[test]
    fn actor_resurrect_with_heal_restores_health_and_havok() {
        let mut e = engine();
        let actor = actor_with(
            &mut e,
            &[
                (ACTOR_SLOT_SET_DELETE, ret(0)),
                (ACTOR_SLOT_4C, ret(0)),
                (ACTOR_SLOT_1D0, ret(0x9900)),
                (ACTOR_SLOT_MODIFY_ACTOR_VALUE, ret(0)),
                (ACTOR_SLOT_INIT_HAVOK, ret(0)),
            ],
            None,
        );
        embedded_table(
            &mut e,
            actor.addr() + 0xa4,
            &[(0x0c, float_ret(30.5)), (0x00, ret(100))],
        );
        let process = object(
            &mut e,
            0x800,
            &[(PROCESS_SLOT_290, ret(0)), (PROCESS_SLOT_410, ret(0))],
        );
        e.set(actor, Actor::pCurrentProcess, process);
        stub_with(&mut e, NI_POINTER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        actor_resurrect(&mut e, actor, 0, 0, 1);
        assert_eq!(
            calls_to(&e, ACTOR_SET_LIFE_STATE),
            vec![vec![actor.addr(), 0]]
        );
        let modify = slot_target(&e, actor.cast(), ACTOR_SLOT_MODIFY_ACTOR_VALUE);
        assert_eq!(
            calls_to(&e, modify),
            vec![vec![actor.addr(), 0x10, 69.5f32.to_bits(), 0]]
        );
        let motion = calls_to(&e, BHK_WORLD_SET_MOTION);
        assert_eq!(motion, vec![vec![0x9900, 1, 1, 0, 1]]);
        let slot_290 = slot_target(&e, process, PROCESS_SLOT_290);
        assert_eq!(calls_to(&e, slot_290), vec![vec![process.addr(), 0]]);
        let slot_410 = slot_target(&e, process, PROCESS_SLOT_410);
        assert_eq!(calls_to(&e, slot_410), vec![vec![process.addr(), 3]]);
        // The branch for the dead: no modifier lists were cleared.
        assert_eq!(call_count(&e, MODIFIER_LIST_DELETE_ALL), 0);
        assert_eq!(call_count(&e, LOCK_ENTER), 0);
    }

    /// Makes `SETTING_GET_VALUE_POINTER` return a pointer to the `float`
    /// given for the setting address it is called on.
    fn float_settings(e: &mut Engine, values: &[(u32, f32)]) {
        let mut table = std::collections::HashMap::new();
        for (address, value) in values {
            let cell = e.mem.alloc(4);
            e.mem.set_f32(cell, *value);
            table.insert(*address, cell);
        }
        e.register_double(SETTING_GET_VALUE_POINTER, move |_, a| ret(table[&a[0]]));
    }

    /// Makes `SETTING_GET_INT_POINTER` return a pointer to the integer given
    /// for the setting address it is called on.
    fn int_settings(e: &mut Engine, values: &[(u32, u32)]) {
        let mut table = std::collections::HashMap::new();
        for (address, value) in values {
            let cell = e.mem.alloc(4);
            e.mem.set_u32(cell, *value);
            table.insert(*address, cell);
        }
        e.register_double(SETTING_GET_INT_POINTER, move |_, a| ret(table[&a[0]]));
    }

    /// An actor, a target and an acquire object for `fn_008987f0`.
    fn reaction_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr<Actor>, Ptr) {
        let actor = actor_with(
            e,
            &[
                (ACTOR_SLOT_22C, ret(0)),
                (ACTOR_SLOT_428, ret(0)),
                (ACTOR_SLOT_304, ret(0)),
                (ACTOR_SLOT_21C, ret(0)),
                (ACTOR_SLOT_460, ret(0)),
                (ACTOR_SLOT_474, ret(0)),
                (ACTOR_SLOT_48, ret(0)),
            ],
            None,
        );
        let target = actor_with(
            e,
            &[
                (ACTOR_SLOT_100, ret(1)),
                (ACTOR_SLOT_218, ret(0)),
                (ACTOR_SLOT_37C, ret(0)),
                (ACTOR_SLOT_42C, ret(0)),
                (ACTOR_SLOT_304, ret(0)),
            ],
            None,
        );
        let acquire = object(
            e,
            0x10,
            &[
                (PROCESS_SLOT_52C, ret(0)),
                (PROCESS_SLOT_504, ret(0)),
                (PROCESS_SLOT_F0, ret(0)),
                (PROCESS_SLOT_33C, ret(0)),
            ],
        );
        stub(e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(e, MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, 1);
        float_settings(
            e,
            &[
                (DETECTION_THRESHOLD_SETTING, 10.0),
                (UNUSED_FLOAT_SETTING, 1.0),
                (SLOT_460_FLOAT_SETTING, 5.0),
            ],
        );
        e.map(COMBAT_DIALOGUE_MANAGER, 4);
        e.set_global(COMBAT_DIALOGUE_MANAGER, 0x7600u32);
        (actor, target, acquire)
    }

    #[test]
    fn fn_008987f0_returns_early_in_the_cases_the_code_lists() {
        let mut e = engine();
        let (actor, target, _) = reaction_setup(&mut e);
        // Protected.
        let protected = slot_target(&e, actor.cast(), ACTOR_SLOT_22C);
        e.register_double(protected, |_, _| ret(1));
        fn_008987f0(&mut e, actor, target, 0);
        assert_eq!(call_count(&e, ACTOR_TEST_8ACE90), 0);

        // The test 008ace90.
        let mut e = engine();
        let (actor, target, _) = reaction_setup(&mut e);
        stub(&mut e, ACTOR_TEST_8ACE90, 1);
        fn_008987f0(&mut e, actor, target, 0);
        assert_eq!(call_count(&e, ACTOR_GET_PACKAGE_SET_AS_PCURRENT), 0);

        // The target is the actor itself.
        let mut e = engine();
        let (actor, _, _) = reaction_setup(&mut e);
        fn_008987f0(&mut e, actor, actor, 0);
        assert_eq!(call_count(&e, ACTOR_GET_PACKAGE_SET_AS_PCURRENT), 0);

        // Process level 4 and the target is the acquire object's slot 0x52c.
        let mut e = engine();
        let (actor, target, acquire) = reaction_setup(&mut e);
        stub(&mut e, PROCESS_LEVEL_NUMBER, 4);
        let slot = slot_target(&e, acquire, PROCESS_SLOT_52C);
        let target_address = target.addr();
        e.register_double(slot, move |_, _| ret(target_address));
        fn_008987f0(&mut e, actor, target, 0);
        assert_eq!(call_count(&e, ACTOR_GET_CURRENT_WEAPON), 0);
    }

    #[test]
    fn fn_008987f0_records_the_detection_at_the_high_process_level() {
        let mut e = engine();
        let (actor, target, acquire) = reaction_setup(&mut e);
        stub(&mut e, MOBILE_OBJECT_GET_CURRENT_PROCESS_TYPE, 0);
        // The detection level is 50 (above the setting 10): level 3; the
        // call writes 7 into its first flag byte and leaves the second at 1.
        stub_with(&mut e, ACTOR_GET_DETECTION_LEVEL_AGAINST_ACTOR, |e, a| {
            e.mem.set_u8(a[3], 7);
            ret(50)
        });
        let record = e.mem.alloc(0x40);
        let creator = slot_target(&e, acquire, PROCESS_SLOT_F0);
        e.register_double(creator, move |_, _| ret(record));
        fn_008987f0(&mut e, actor, target, 0);
        assert_eq!(e.mem.u32(record + 8), 50);
        assert_eq!(e.mem.u32(record + 4), 3);
        assert_eq!(e.mem.u8(record + 0x1e), 7);
        assert_eq!(e.mem.u8(record + 0x1c), 1);
        assert_eq!(e.mem.u8(record + 0x1d), 1);
        let created = calls_to(&e, creator);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0][1..], [target.addr(), 3, 7, 50, 0, 1, 1]);
        let detection = calls_to(&e, ACTOR_GET_DETECTION_LEVEL_AGAINST_ACTOR);
        assert_eq!(detection[0][..3], [actor.addr(), 0, target.addr()]);

        // An existing record is reused, with the level 0 for a low detection.
        e.call_log = Some(vec![]);
        stub(&mut e, ACTOR_GET_DETECTION_LEVEL_AGAINST_ACTOR, 4);
        let finder = slot_target(&e, acquire, PROCESS_SLOT_504);
        let existing = e.mem.alloc(0x40);
        e.register_double(finder, move |_, _| ret(existing));
        fn_008987f0(&mut e, actor, target, 0);
        assert_eq!(call_count(&e, creator), 0);
        assert_eq!(e.mem.u32(existing + 8), 4);
        assert_eq!(e.mem.u32(existing + 4), 0);
    }

    #[test]
    fn fn_008987f0_starts_the_dialogue_for_the_player_on_a_friend_hit() {
        let mut e = engine();
        let (actor, _, _) = reaction_setup(&mut e);
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        stub(&mut e, ACTOR_GET_FACTION_FIGHT_REACTION, 2);
        stub(&mut e, EXTRA_ADD_FRIEND_HIT, 1);
        int_settings(&mut e, &[(FRIEND_HIT_LIMIT_SETTINGS[3], 3)]);
        fn_008987f0(&mut e, actor, player, 0);
        assert_eq!(
            calls_to(&e, COMBAT_DIALOGUE_START_DIALOGUE),
            vec![vec![0x7600, actor.addr(), player.addr(), 2, 2, 1, 0]]
        );
        let flags = slot_target(&e, actor.cast(), ACTOR_SLOT_48);
        assert_eq!(calls_to(&e, flags), vec![vec![actor.addr(), 0x8000_0000]]);
        assert_eq!(call_count(&e, ACTOR_ATTACK_ALARM), 0);
        // The limit of 1000 or more resets the hits first.
        e.call_log = Some(vec![]);
        int_settings(&mut e, &[(FRIEND_HIT_LIMIT_SETTINGS[3], 1000)]);
        fn_008987f0(&mut e, actor, player, 0);
        assert_eq!(call_count(&e, EXTRA_RESET_FRIEND_HITS), 1);

        // Talking, dialogue and the skip test each stop the dialogue (and the
        // whole reaction).
        for blocker in [
            ACTOR_IS_TALKING,
            MOBILE_OBJECT_IS_IN_DIALOGUE,
            ACTOR_SHOULD_SKIP_FALLOUT_BEHAVIOR,
        ] {
            e.call_log = Some(vec![]);
            stub(&mut e, blocker, 1);
            fn_008987f0(&mut e, actor, player, 0);
            assert_eq!(call_count(&e, COMBAT_DIALOGUE_START_DIALOGUE), 0);
            assert_eq!(call_count(&e, ACTOR_ATTACK_ALARM), 0);
            stub(&mut e, blocker, 0);
        }
        // The test 005a3790 ends it before any count.
        e.call_log = Some(vec![]);
        stub(&mut e, ACTOR_TEST_5A3790, 1);
        fn_008987f0(&mut e, actor, player, 0);
        assert_eq!(call_count(&e, EXTRA_ADD_FRIEND_HIT), 0);
    }

    #[test]
    fn fn_008987f0_raises_the_alarm_through_the_owner_for_the_player() {
        let mut e = engine();
        let (actor, _, _) = reaction_setup(&mut e);
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        // A reaction other than 2 and 3: no friend-hit counting.
        stub(&mut e, ACTOR_GET_FACTION_FIGHT_REACTION, 0);
        stub(&mut e, REFR_GET_OWNER, 0x7700);
        stub(&mut e, FORM_TYPE, 8);
        stub(&mut e, PROCESS_LISTS_GET_OWNER_ACTOR, 0x7800);
        stub(&mut e, ACTOR_IN_COMBAT, 0);
        fn_008987f0(&mut e, actor, player, 0);
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_GET_OWNER_ACTOR),
            vec![vec![PROCESS_LISTS, 0x7700, 0, 1]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_ATTACK_ALARM),
            vec![
                vec![0x7800, player.addr(), 0, 1],
                vec![actor.addr(), player.addr(), 0, 1]
            ]
        );
        assert_eq!(call_count(&e, EXTRA_ADD_FRIEND_HIT), 0);

        // Another owner type uses the other lookup.
        e.call_log = Some(vec![]);
        stub(&mut e, FORM_TYPE, 9);
        stub(&mut e, PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH, 0);
        fn_008987f0(&mut e, actor, player, 0);
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH),
            vec![vec![PROCESS_LISTS, 0x7700, 0]]
        );
        assert_eq!(call_count(&e, ACTOR_ATTACK_ALARM), 1);
    }

    #[test]
    fn fn_008987f0_alarms_for_a_target_that_is_not_the_player() {
        let mut e = engine();
        let (actor, target, _) = reaction_setup(&mut e);
        let actor_304 = slot_target(&e, actor.cast(), ACTOR_SLOT_304);
        e.register_double(actor_304, |_, _| ret(1));
        for slot in [ACTOR_SLOT_218, ACTOR_SLOT_37C] {
            let target_slot = slot_target(&e, target.cast(), slot);
            e.register_double(target_slot, |_, _| ret(1));
        }
        fn_008987f0(&mut e, actor, target, 0);
        assert_eq!(
            calls_to(&e, ACTOR_ATTACK_ALARM),
            vec![vec![actor.addr(), target.addr(), 0, 1]]
        );
        // It returns before the controller work.
        assert_eq!(call_count(&e, CONTROLLER_WORKER_97F580), 0);
    }

    #[test]
    fn fn_008987f0_tells_the_acquire_object_when_the_target_relates_to_the_player() {
        let mut e = engine();
        let (actor, _, acquire) = reaction_setup(&mut e);
        let player = actor_with(
            &mut e,
            &[(ACTOR_SLOT_304, ret(0)), (ACTOR_SLOT_42C, ret(0))],
            None,
        );
        set_player(&mut e, player);
        stub(&mut e, ACTOR_TEST_8B0670, 1);
        // The reaction 0 with the byte 0 and "should attack" true.
        stub(&mut e, ACTOR_GET_FACTION_FIGHT_REACTION, 0);
        stub(&mut e, ACTOR_GET_SHOULD_ATTACK_ACTOR, 1);
        stub(&mut e, REFR_GET_OWNER, 0);
        fn_008987f0(&mut e, actor, player, 0x55);
        let slot = slot_target(&e, acquire, PROCESS_SLOT_33C);
        assert_eq!(
            calls_to(&e, slot),
            vec![vec![
                acquire.addr(),
                actor.addr(),
                player.addr(),
                1,
                1,
                0,
                0,
                0,
                1,
                0,
                0,
                1,
                0,
                0
            ]]
        );
        // No combat controller, not fleeing, not alarmed before (the player
        // branch above already alarmed through the first block).
        assert_eq!(call_count(&e, ACTOR_ATTACK_ALARM), 1);
        let reaction_args = calls_to(&e, ACTOR_GET_SHOULD_ATTACK_ACTOR);
        assert_eq!(reaction_args[0][..3], [actor.addr(), player.addr(), 0]);
        // No controller, not fleeing but already alarmed: slot 0x460 gets the
        // target and the setting 5.0.
        let slot_460 = slot_target(&e, actor.cast(), ACTOR_SLOT_460);
        assert_eq!(
            calls_to(&e, slot_460),
            vec![vec![actor.addr(), player.addr(), 5.0f32.to_bits()]]
        );
        // The actor's slot 0x474 runs with 1 (the actor is not the player).
        let slot_474 = slot_target(&e, actor.cast(), ACTOR_SLOT_474);
        assert_eq!(calls_to(&e, slot_474), vec![vec![actor.addr(), 1]]);
    }

    #[test]
    fn fn_008987f0_tells_the_acquire_object_without_alarm_for_other_targets() {
        let mut e = engine();
        let (actor, target, acquire) = reaction_setup(&mut e);
        // A non-player target that relates to the actor through slot 0x42c.
        let target_42c = slot_target(&e, target.cast(), ACTOR_SLOT_42C);
        let actor_address = actor.addr();
        e.register_double(target_42c, move |_, _| ret(actor_address));
        let player = actor_with(&mut e, &[], None);
        set_player(&mut e, player);
        stub(&mut e, ACTOR_TEST_8B0670, 1);
        stub(&mut e, ACTOR_IS_FLEEING, 0);
        // The target is not the player, so only the plain tell happens.
        fn_008987f0(&mut e, actor, target, 0);
        let slot = slot_target(&e, acquire, PROCESS_SLOT_33C);
        assert_eq!(call_count(&e, slot), 1);
        assert_eq!(call_count(&e, ACTOR_ATTACK_ALARM), 0);
        let slot_460 = slot_target(&e, actor.cast(), ACTOR_SLOT_460);
        assert_eq!(call_count(&e, slot_460), 0);
    }

    #[test]
    fn fn_008987f0_with_a_controller_that_has_the_target_only_passes_it_on() {
        let mut e = engine();
        let (actor, target, _) = reaction_setup(&mut e);
        let controller_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(0x9700));
        stub(&mut e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET, 1);
        fn_008987f0(&mut e, actor, target, 0x66);
        assert_eq!(
            calls_to(&e, CONTROLLER_WORKER_97F580),
            vec![vec![0x9700, target.addr(), 0x66]]
        );
        assert_eq!(call_count(&e, ACTOR_GET_DETECTION_LEVEL_AGAINST_ACTOR), 0);
        assert_eq!(call_count(&e, ACTOR_ATTACK_ALARM), 0);
    }

    /// An actor for `fn_00899200` with the slots the function uses and an
    /// acquire object without a weapon.
    fn hit_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr) {
        let process = object(
            e,
            0x800,
            &[
                (PROCESS_SLOT_CURRENT_WEAPON, ret(0)),
                (PROCESS_SLOT_ANIM_SEQUENCE, ret(0x7a00)),
                (PROCESS_SLOT_460, ret(0x7b00)),
            ],
        );
        let acquire = object(
            e,
            0x10,
            &[
                (PROCESS_SLOT_448, ret(0)),
                (PROCESS_SLOT_27C, ret(0)),
                (PROCESS_SLOT_274, ret(0)),
                (PROCESS_SLOT_CURRENT_WEAPON, ret(0)),
                (PROCESS_SLOT_ANIM_SEQUENCE, ret(0x7a00)),
            ],
        );
        let actor = actor_with(
            e,
            &[
                (ACTOR_SLOT_21C, ret(0)),
                (ACTOR_SLOT_360, ret(0)),
                (ACTOR_SLOT_428, ret(0)),
                (ACTOR_SLOT_1D0, ret(0x7c00)),
                (ACTOR_SLOT_REACH, float_ret(2.0)),
                (ACTOR_SLOT_POSITION, ret(0)),
            ],
            None,
        );
        e.set(actor, Actor::pCurrentProcess, process);
        embedded_table(
            e,
            actor.addr() + ACTOR_MAGIC_CASTER_OFFSET,
            &[
                (MAGIC_CASTER_SLOT_40, ret(0)),
                (MAGIC_CASTER_SLOT_48, ret(0)),
            ],
        );
        stub(e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(e, DEBUG_DRAW_LEVEL, 0);
        int_settings(e, &[(WORD_SETTING_3B4, 0x1234)]);
        float_settings(e, &[]);
        stub_float(e, REFR_GET_SCALE, 1.0);
        e.map(SPECIAL_IDLE_STATE_OBJECT, 0x10);
        e.map(VATS_ACTOR, 4);
        (actor, acquire)
    }

    #[test]
    fn fn_00899200_only_queues_when_the_task_queue_is_used() {
        let mut e = engine();
        let (actor, _) = hit_setup(&mut e);
        stub(&mut e, PICK_UP_GOES_TO_TASK_QUEUE, 1);
        stub(&mut e, GET_TES, 0x7d00);
        fn_00899200(&mut e, actor, 3, 4);
        assert_eq!(
            calls_to(&e, TES_QUEUE_ATTACK),
            vec![vec![0x7d00, actor.addr(), 3, 4]]
        );
        assert_eq!(call_count(&e, GET_SAVED_ACQUIRE_OBJECT), 0);
    }

    #[test]
    fn fn_00899200_updates_the_acquire_object_and_the_word() {
        let mut e = engine();
        let (actor, acquire) = hit_setup(&mut e);
        fn_00899200(&mut e, actor, 1, 0);
        let slot = slot_target(&e, acquire, PROCESS_SLOT_448);
        assert_eq!(calls_to(&e, slot), vec![vec![acquire.addr(), 1]]);
        // Slot 0x21c false: the setting is the word.
        assert_eq!(
            calls_to(&e, ACTOR_WORKER_8BC240),
            vec![vec![actor.addr(), 0x1234]]
        );
        // Slot 0x21c true: the base form's word.
        e.call_log = Some(vec![]);
        let slot_21c = slot_target(&e, actor.cast(), ACTOR_SLOT_21C);
        e.register_double(slot_21c, |_, _| ret(1));
        stub(&mut e, REFR_BASE_FORM, 0x7e00);
        stub(&mut e, FORM_GETTER_5FBEB0, 0x77);
        fn_00899200(&mut e, actor, 1, 0);
        assert_eq!(
            calls_to(&e, ACTOR_WORKER_8BC240),
            vec![vec![actor.addr(), 0x77]]
        );
        // With flag_b clear nothing else happens.
        assert_eq!(call_count(&e, ACTOR_COMBAT_HIT), 0);
        assert_eq!(call_count(&e, CONTROLLER_WORKER_97F7D0), 0);
    }

    #[test]
    fn fn_00899200_vats_uses_up_a_shot_or_casts_the_default_spell() {
        let mut e = engine();
        let (actor, _) = hit_setup(&mut e);
        e.set_global(VATS_ACTOR, actor.addr());
        stub_with(&mut e, ENTRY_FORM, |_, a| {
            ret(if a[0] == VATS_OBJECT { 4 } else { 0x7f00 })
        });
        let weapon_slot = slot_target(
            &e,
            e.get(actor, Actor::pCurrentProcess),
            PROCESS_SLOT_CURRENT_WEAPON,
        );
        e.register_double(weapon_slot, |_, _| ret(0x7f10));
        let action = e.mem.alloc(0x20);
        stub(&mut e, VATS_GET_CURRENT_ACTION, action);
        e.mem.set_u8(action + 8, 5);
        stub(&mut e, WEAPON_BYTE_FLAG, 1);
        fn_00899200(&mut e, actor, 0, 0);
        assert_eq!(e.mem.u8(action + 8), 4);

        // Without the weapon flag: the spell is cast through the caster.
        let reference = object(&mut e, 0x10, &[(0x234, ret(0)), (0x100, ret(1))]);
        e.mem.set_u8(action + 7, 1);
        e.mem.set_u32(action + 0xc, reference.addr());
        stub(&mut e, WEAPON_BYTE_FLAG, 0);
        stub(&mut e, GET_DEFAULT_OBJECT, 0x8100);
        stub(&mut e, RTTI_DYNAMIC_CAST, 0x8200);
        stub(&mut e, MAGIC_CASTER_RELEASE_CAST, 1);
        e.call_log = Some(vec![]);
        fn_00899200(&mut e, actor, 0, 0);
        assert_eq!(calls_to(&e, GET_DEFAULT_OBJECT), vec![vec![5]]);
        assert_eq!(
            calls_to(&e, RTTI_DYNAMIC_CAST),
            vec![vec![0x8100, 0, RTTI_TYPE_TES_FORM, RTTI_TYPE_MAGIC_ITEM, 0]]
        );
        let caster = actor.addr() + ACTOR_MAGIC_CASTER_OFFSET;
        let slot_40 = slot_target(&e, Ptr::new(caster), MAGIC_CASTER_SLOT_40);
        let slot_48 = slot_target(&e, Ptr::new(caster), MAGIC_CASTER_SLOT_48);
        assert_eq!(calls_to(&e, slot_40), vec![vec![caster, 0x8200]]);
        assert_eq!(
            calls_to(&e, slot_48),
            vec![vec![caster, reference.addr() + 0x94]]
        );
        assert_eq!(
            calls_to(&e, MAGIC_CASTER_RELEASE_CAST),
            vec![vec![caster, 0]]
        );
        assert_eq!(calls_to(&e, MISC_STAT_INCREMENT), vec![vec![0x15]]);
        // A target that is already blocked (slot 0x234) casts nothing.
        let blocked = slot_target(&e, reference, 0x234);
        e.register_double(blocked, |_, _| ret(1));
        e.call_log = Some(vec![]);
        fn_00899200(&mut e, actor, 0, 0);
        assert_eq!(call_count(&e, GET_DEFAULT_OBJECT), 0);
    }

    #[test]
    fn fn_00899200_draws_the_debug_arrow() {
        let mut e = engine();
        let (actor, _) = hit_setup(&mut e);
        stub(&mut e, DEBUG_DRAW_LEVEL, 2);
        e.map(TES_SINGLETON, 4);
        e.set_global(TES_SINGLETON, 0x8300u32);
        e.map(ARROW_LENGTH_SCALE, 4);
        e.set_global(ARROW_LENGTH_SCALE, 800.0f32);
        e.map(ARROW_WIDTH_SCALE, 4);
        e.set_global(ARROW_WIDTH_SCALE, 0.1f32);
        // The colour constructor returns the address of what it built.
        stub_with(&mut e, COLOR_CONSTRUCT, |e, a| {
            for i in 0..4 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            ret(a[0])
        });
        stub(&mut e, NODE_ROTATION, 0x8400);
        let vector = e.mem.alloc(12);
        e.mem.set_f32(vector, 1.0);
        stub(&mut e, MATRIX_TRANSFORM, vector);
        stub(&mut e, VECTOR_SUBTRACT, vector);
        stub(&mut e, VECTOR_SCALE, vector);
        stub(&mut e, ACTOR_GET_WEAPON_POSITION, vector);
        stub(&mut e, CREATE_DIR_ARROW, 0x8500);
        stub_float(&mut e, DEBUG_DRAW_DURATION_GETTER, 4.5);
        fn_00899200(&mut e, actor, 0, 0);
        // Red: the actor's slot 0x360 is false.
        let arrow = calls_to(&e, CREATE_DIR_ARROW);
        assert_eq!(arrow.len(), 1);
        assert_eq!(arrow[0][0], vector);
        let color = arrow[0][1];
        assert_eq!(
            [
                e.mem.f32(color),
                e.mem.f32(color + 4),
                e.mem.f32(color + 8),
                e.mem.f32(color + 12)
            ],
            [1.0, 0.0, 0.0, 1.0]
        );
        let scales = calls_to(&e, VECTOR_SCALE);
        assert_eq!(scales[0][2], 800.0f32.to_bits());
        assert_eq!(scales[1][2], 0.1f32.to_bits());
        assert_eq!(
            calls_to(&e, TES_ADD_TEMP_DEBUG_OBJECT),
            vec![vec![0x8300, 0x8500, 4.5f32.to_bits()]]
        );
        assert_eq!(calls_to(&e, MATRIX_TRANSFORM).len(), 2);
        assert_eq!(calls_to(&e, VECTOR_NORMALIZE).len(), 1);
        assert_eq!(calls_to(&e, DEBUG_OBJECT_SET_POSITION)[0][0], 0x8500);

        // Blue when slot 0x360 is true.
        e.call_log = Some(vec![]);
        let slot_360 = slot_target(&e, actor.cast(), ACTOR_SLOT_360);
        e.register_double(slot_360, |_, _| ret(1));
        fn_00899200(&mut e, actor, 0, 0);
        let color = calls_to(&e, CREATE_DIR_ARROW)[0][1];
        assert_eq!([e.mem.f32(color + 4), e.mem.f32(color + 8)], [0.0, 1.0]);
    }

    #[test]
    fn fn_00899200_hits_the_target_the_acquire_object_names() {
        let mut e = engine();
        let (actor, acquire) = hit_setup(&mut e);
        let holder = slot_target(&e, acquire, PROCESS_SLOT_27C);
        e.register_double(holder, |_, _| ret(0x8600));
        let entry = slot_target(&e, acquire, PROCESS_SLOT_274);
        e.register_double(entry, |_, _| ret(0x8700));
        stub(&mut e, OBJECT_TYPE_FIELD, 0x2c);
        let target = actor_with(&mut e, &[(ACTOR_SLOT_100, ret(1))], None);
        let target_address = target.addr();
        stub_with(&mut e, NODE_NEXT, move |_, a| {
            ret(if a[0] == 0x8700 { target_address } else { 0 })
        });
        // The animation group 0x64: hand to hand kind 1.
        stub(&mut e, ANIMATION_ZERO_GLOBAL_TRANSFORM, 0x8800);
        stub(&mut e, ANIM_GROUP_GET_NUMBER, 0x64);
        let controller_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(0x8900));
        fn_00899200(&mut e, actor, 9, 1);
        assert_eq!(
            calls_to(&e, ACTOR_COMBAT_HIT),
            vec![vec![actor.addr(), target.addr(), 9, 0, 1]]
        );
        assert_eq!(
            calls_to(&e, CONTROLLER_WORKER_97F7D0),
            vec![vec![0x8900, target.addr()]]
        );

        // The group 0x3e gives kind 2; a weapon of another type gives 0.
        e.call_log = Some(vec![]);
        stub(&mut e, ANIM_GROUP_GET_NUMBER, 0x3e);
        fn_00899200(&mut e, actor, 9, 1);
        assert_eq!(calls_to(&e, ACTOR_COMBAT_HIT)[0][4], 2);
        e.call_log = Some(vec![]);
        let weapon_slot = slot_target(&e, acquire, PROCESS_SLOT_CURRENT_WEAPON);
        e.register_double(weapon_slot, |_, _| ret(0x8a00));
        stub(&mut e, ENTRY_FORM, 0x8b00);
        stub(&mut e, WEAPON_TYPE, 3);
        fn_00899200(&mut e, actor, 9, 1);
        assert_eq!(calls_to(&e, ACTOR_COMBAT_HIT)[0][4], 0);

        // The test 008ace90 on the target drops it (no hit).
        e.call_log = Some(vec![]);
        stub(&mut e, ACTOR_TEST_8ACE90, 1);
        fn_00899200(&mut e, actor, 9, 1);
        assert_eq!(call_count(&e, ACTOR_COMBAT_HIT), 0);
    }

    #[test]
    fn fn_00899200_plays_the_weapon_sound_and_casts_when_nothing_is_hit() {
        let mut e = engine();
        let (actor, acquire) = hit_setup(&mut e);
        let weapon_slot = slot_target(&e, acquire, PROCESS_SLOT_CURRENT_WEAPON);
        e.register_double(weapon_slot, |_, _| ret(0x8a00));
        stub(&mut e, ENTRY_FORM, 0x8b00);
        stub(&mut e, WEAPON_SOUND_SET, 0x8c00);
        stub(&mut e, SOUND_SET_FILE_NAME, 0x8d00);
        stub(&mut e, AUDIO_INSTANCE, 0x8e00);
        let position = e.mem.alloc(12);
        e.mem.set_f32(position, 1.5);
        e.mem.set_f32(position + 4, 2.5);
        e.mem.set_f32(position + 8, 3.5);
        let position_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_POSITION);
        e.register_double(position_slot, move |_, _| ret(position));
        // The reach worker finds no target.
        stub(&mut e, ACTOR_REACH_WORKER, 0);
        // Spell part: the base form record names group 0xff.
        stub(&mut e, ACTOR_BASE_FORM, 0x9000);
        stub(&mut e, NODE_NEXT, 0x9100);
        stub(&mut e, RECORD_TEST_406090, 1);
        stub(&mut e, ACTOR_BASE_DATA_GET_FATIGUE, 0xff);
        fn_00899200(&mut e, actor, 0, 1);
        let play = calls_to(&e, AUDIO_GET_SOUND_HANDLE_BY_FILENAME);
        assert_eq!(play.len(), 1);
        assert_eq!(play[0][0], 0x8e00);
        assert_eq!(play[0][2..], [0x8d00, 0x102, 0x8c00]);
        let handle = play[0][1];
        assert_eq!(
            calls_to(&e, SOUND_HANDLE_SET_POSITION_XYZ),
            vec![vec![
                handle,
                1.5f32.to_bits(),
                2.5f32.to_bits(),
                3.5f32.to_bits()
            ]]
        );
        assert_eq!(
            calls_to(&e, SOUND_HANDLE_SET_OBJECT_TO_FOLLOW),
            vec![vec![handle, 0x7c00]]
        );
        assert_eq!(calls_to(&e, SOUND_HANDLE_PLAY), vec![vec![handle, 0]]);
        assert_eq!(calls_to(&e, SOUND_HANDLE_DESTROY), vec![vec![handle]]);
        // The spell is cast: record at node + 0x18.
        let caster = actor.addr() + ACTOR_MAGIC_CASTER_OFFSET;
        let slot_40 = slot_target(&e, Ptr::new(caster), MAGIC_CASTER_SLOT_40);
        let slot_48 = slot_target(&e, Ptr::new(caster), MAGIC_CASTER_SLOT_48);
        assert_eq!(calls_to(&e, slot_40), vec![vec![caster, 0x9118]]);
        assert_eq!(
            calls_to(&e, slot_48),
            vec![vec![caster, actor.addr() + 0x94]]
        );
        assert_eq!(calls_to(&e, RECORD_TEST_406090), vec![vec![0x9118 + 0xc]]);
        assert_eq!(
            calls_to(&e, MAGIC_CASTER_RELEASE_CAST),
            vec![vec![caster, 0]]
        );
        assert_eq!(call_count(&e, ACTOR_COMBAT_HIT), 0);

        // The record names a group the animation does not match: no cast.
        e.call_log = Some(vec![]);
        stub(&mut e, ACTOR_BASE_DATA_GET_FATIGUE, 0x20);
        stub(&mut e, ANIMATION_ZERO_GLOBAL_TRANSFORM, 0x9200);
        stub(&mut e, ANIM_GROUP_GET_NUMBER, 0x21);
        fn_00899200(&mut e, actor, 0, 1);
        assert_eq!(call_count(&e, slot_40), 0);
        // ... and one it does.
        stub(&mut e, ANIM_GROUP_GET_NUMBER, 0x20);
        fn_00899200(&mut e, actor, 0, 1);
        assert_eq!(call_count(&e, slot_40), 1);
    }

    #[test]
    fn fn_00899200_gives_up_for_an_actor_that_the_test_8ae660_stops() {
        let mut e = engine();
        let (actor, _) = hit_setup(&mut e);
        let slot_360 = slot_target(&e, actor.cast(), ACTOR_SLOT_360);
        e.register_double(slot_360, |_, _| ret(1));
        stub(&mut e, ACTOR_REACH_WORKER, 0);
        stub(&mut e, ACTOR_TEST_8AE660, 1);
        let controller_slot = slot_target(&e, actor.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(0x8900));
        fn_00899200(&mut e, actor, 6, 1);
        assert_eq!(calls_to(&e, ACTOR_TEST_8AE660), vec![vec![actor.addr(), 6]]);
        assert_eq!(call_count(&e, CONTROLLER_WORKER_97F7D0), 0);
        assert_eq!(call_count(&e, SOUND_HANDLE_PLAY), 0);
    }

    /// What the `HitData` doubles do: the allocation is a plain block, the
    /// pointer holder stores and returns the pointer, and the initializer
    /// writes 12.5 as the health damage.
    fn hit_data_doubles(e: &mut Engine) {
        stub_with(e, OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        stub_with(e, HIT_DATA_CONSTRUCT, |_, a| ret(a[0]));
        stub_with(e, NI_POINTER_HIT_DATA_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        stub_with(e, NI_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        stub_with(e, HIT_DATA_INITIALIZE, |e, a| {
            e.mem.set_f32(a[0] + HIT_HEALTH_DAMAGE, 12.5);
            e.mem.set_f32(a[0] + HIT_TOTAL_DAMAGE, 20.0);
            e.mem.set_f32(a[0] + HIT_TARGETED_LIMB_DAMAGE, 8.0);
            e.mem.set_u32(a[0] + HIT_ATTACK_SKILL, 0x1b);
            ret(0)
        });
    }

    /// An attacker with a drawn weapon entry, a target and the `HitData`
    /// doubles. Returns (attacker, target, weapon entry).
    fn combat_hit_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr<Actor>, u32) {
        hit_data_doubles(e);
        let player = actor_with(e, &[], None);
        set_player(e, player);
        let process = object(
            e,
            0x800,
            &[
                (PROCESS_SLOT_CURRENT_WEAPON, ret(0)),
                (PROCESS_SLOT_580, ret(0)),
            ],
        );
        let weapon_entry = 0x8a00u32;
        let acquire = object(e, 0x10, &[(PROCESS_SLOT_CURRENT_WEAPON, ret(weapon_entry))]);
        stub(e, GET_SAVED_ACQUIRE_OBJECT, acquire.addr());
        stub(e, ENTRY_FORM, 0x8b00);
        stub(e, WEAPON_TYPE, 6);
        let position = e.mem.alloc(12);
        e.mem.set_f32(position, 1.0);
        e.mem.set_f32(position + 4, 2.0);
        e.mem.set_f32(position + 8, 3.0);
        let slots = [
            (ACTOR_SLOT_428, ret(0)),
            (ACTOR_SLOT_360, ret(0)),
            (ACTOR_SLOT_3D8, ret(1)),
            (ACTOR_SLOT_32C, ret(5)),
            (ACTOR_SLOT_328, ret(0)),
            (ACTOR_SLOT_330, ret(0)),
            (ACTOR_SLOT_4C0, ret(0)),
            (ACTOR_SLOT_3C8, ret(0)),
            (ACTOR_SLOT_POSITION, ret(position)),
        ];
        let attacker = actor_with(e, &slots, None);
        e.set(attacker, Actor::pCurrentProcess, process);
        let target_process = object(e, 0x800, &[(PROCESS_SLOT_CURRENT_WEAPON, ret(0))]);
        let target = actor_with(e, &[(ACTOR_SLOT_POSITION, ret(position))], None);
        e.set(target, Actor::pCurrentProcess, target_process);
        e.map(IMPACT_BLOCK_DEFAULT_SOUND, 4);
        e.set_global(IMPACT_BLOCK_DEFAULT_SOUND, 0x4141u32);
        e.map(VATS_OBJECT, 0x10);
        (attacker, target, weapon_entry)
    }

    #[test]
    fn actor_combat_hit_gives_up_without_a_usable_target() {
        let mut e = engine();
        let (attacker, target, _) = combat_hit_setup(&mut e);
        actor_combat_hit(&mut e, attacker, Ptr::NULL, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, HIT_DATA_INITIALIZE), 0);
        stub(&mut e, ACTOR_TEST_8ACE90, 1);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, HIT_DATA_INITIALIZE), 0);
        stub(&mut e, ACTOR_TEST_8ACE90, 0);
        stub(&mut e, ACTOR_PROCESS_LEVEL_TEST_437B90, 1);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, HIT_DATA_INITIALIZE), 0);
        assert_eq!(call_count(&e, NI_POINTER_HIT_DATA_DESTRUCT), 0);
    }

    #[test]
    fn actor_combat_hit_plays_the_hit_sound_and_hits_the_target() {
        let mut e = engine();
        let (attacker, target, weapon_entry) = combat_hit_setup(&mut e);
        actor_combat_hit(&mut e, attacker, target, 3, Ptr::NULL, 2);
        let init = calls_to(&e, HIT_DATA_INITIALIZE);
        assert_eq!(init.len(), 1);
        assert_eq!(
            init[0][1..],
            [attacker.addr(), target.addr(), weapon_entry, 3, 0]
        );
        let hit_data = init[0][0];
        // The slot 0x3d8 got the weapon entry, the target, the source and an
        // out byte.
        let connects = calls_to(&e, slot_target(&e, attacker.cast(), ACTOR_SLOT_3D8));
        assert_eq!(
            connects[0][..4],
            [attacker.addr(), weapon_entry, target.addr(), 0]
        );
        // The counter is read, incremented and the attack skill passed on.
        let setter = slot_target(&e, attacker.cast(), ACTOR_SLOT_328);
        assert_eq!(calls_to(&e, setter), vec![vec![attacker.addr(), 6]]);
        let skill = slot_target(&e, attacker.cast(), ACTOR_SLOT_330);
        assert_eq!(calls_to(&e, skill), vec![vec![attacker.addr(), 0x1b]]);
        // Flag 1 of the hit data is not set: the plain hit sound.
        assert_eq!(
            calls_to(&e, IMPACT_MIXER_PLAY_WEAPON_HIT),
            vec![vec![
                attacker.addr(),
                12.5f32.to_bits(),
                0.0f32.to_bits(),
                target.addr(),
                6,
                0xffff_ffff,
                0xffff_ffff,
                0,
                1
            ]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_HIT_ME),
            vec![vec![target.addr(), hit_data, 2]]
        );
        assert_eq!(calls_to(&e, NI_POINTER_HIT_DATA_DESTRUCT).len(), 1);
    }

    #[test]
    fn actor_combat_hit_without_a_weapon_uses_the_unarmed_sound_type() {
        let mut e = engine();
        let (attacker, target, _) = combat_hit_setup(&mut e);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        let sound = calls_to(&e, IMPACT_MIXER_PLAY_WEAPON_HIT);
        assert_eq!(sound[0][4], 0xffff_ffff);
        assert_eq!(call_count(&e, ACTOR_HIT_ME), 1);
    }

    #[test]
    fn actor_combat_hit_plays_the_block_sounds() {
        let mut e = engine();
        let (attacker, target, _) = combat_hit_setup(&mut e);
        // Flag 1 set, flag 2 clear: the block sound with the "connects"
        // flag inverted, a zero form and the two positions.
        stub_with(&mut e, HIT_DATA_FLAG_TEST, |_, a| ret(u32::from(a[1] == 1)));
        stub_with(&mut e, HIT_DATA_INITIALIZE, |e, a| {
            e.mem.set_f32(a[0] + HIT_HEALTH_DAMAGE, 12.5);
            e.mem.set_f32(a[0] + HIT_PERCENT_BLOCKED, 0.25);
            ret(0)
        });
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        let one = 1.0f32.to_bits();
        let two = 2.0f32.to_bits();
        let three = 3.0f32.to_bits();
        assert_eq!(
            calls_to(&e, IMPACT_MIXER_PLAY_WEAPON_BLOCK),
            vec![vec![6, 0, one, two, three, one, two, three, 0]]
        );
        // Slot 0x4c0 got the damage, the blocked fraction, target, source.
        let report = slot_target(&e, attacker.cast(), ACTOR_SLOT_4C0);
        assert_eq!(
            calls_to(&e, report),
            vec![vec![
                attacker.addr(),
                12.5f32.to_bits(),
                0.25f32.to_bits(),
                target.addr(),
                0
            ]]
        );
        assert_eq!(call_count(&e, IMPACT_MIXER_PLAY_WEAPON_HIT), 0);

        // Both flags: the target's weapon form is used when it holds one,
        // the default sound word otherwise.
        e.call_log = Some(vec![]);
        stub(&mut e, HIT_DATA_FLAG_TEST, 1);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(
            calls_to(&e, IMPACT_MIXER_PLAY_WEAPON_BLOCK),
            vec![vec![6, 0x4141, one, two, three, one, two, three, 1]]
        );
        e.call_log = Some(vec![]);
        let target_weapon = slot_target(
            &e,
            Ptr::new(e.get(target, Actor::pCurrentProcess).addr()),
            PROCESS_SLOT_CURRENT_WEAPON,
        );
        e.register_double(target_weapon, |_, _| ret(0x8c00));
        stub(&mut e, ENTRY_FORM, 0x8d00);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(
            calls_to(&e, IMPACT_MIXER_PLAY_WEAPON_BLOCK),
            vec![vec![6, 0x8d00, one, two, three, one, two, three, 1]]
        );
    }

    #[test]
    fn actor_combat_hit_does_not_hit_when_the_controller_has_other_targets() {
        let mut e = engine();
        let (attacker, target, _) = combat_hit_setup(&mut e);
        let controller_slot = slot_target(&e, attacker.cast(), ACTOR_SLOT_428);
        e.register_double(controller_slot, |_, _| ret(0x9800));
        stub(&mut e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET, 0);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(
            calls_to(&e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET),
            vec![vec![0x9800, target.addr()]]
        );
        assert_eq!(call_count(&e, ACTOR_HIT_ME), 0);
        assert_eq!(call_count(&e, NI_POINTER_HIT_DATA_DESTRUCT), 1);
        // When it does have the target, the hit goes ahead.
        e.call_log = Some(vec![]);
        stub(&mut e, COMBAT_CONTROLLER_IS_ACTOR_A_COMBAT_TARGET, 1);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, ACTOR_HIT_ME), 1);
    }

    #[test]
    fn actor_combat_hit_vats_merges_the_action_hit_data() {
        let mut e = engine();
        let (attacker, target, weapon_entry) = combat_hit_setup(&mut e);
        let slot_360 = slot_target(&e, attacker.cast(), ACTOR_SLOT_360);
        e.register_double(slot_360, |_, _| ret(1));
        stub_with(&mut e, ENTRY_FORM, |_, a| {
            ret(if a[0] == VATS_OBJECT { 4 } else { 0x8b00 })
        });
        let action = e.mem.alloc(0x20);
        let saved = e.mem.alloc(0x64);
        e.mem.set_u32(action + 0x14, saved);
        e.mem.set_u8(action + 4, 0);
        stub(&mut e, VATS_GET_CURRENT_ACTION, action);
        // The freshly initialized data carries a location, a direction and
        // damages, which are merged into the copy.
        stub_with(&mut e, HIT_DATA_INITIALIZE, |e, a| {
            for i in 0..3 {
                e.mem.set_f32(a[0] + HIT_LOCATION + 4 * i, 1.0 + i as f32);
                e.mem.set_f32(a[0] + HIT_DIRECTION + 4 * i, 4.0 + i as f32);
            }
            e.mem.set_f32(a[0] + HIT_HEALTH_DAMAGE, 10.0);
            e.mem.set_f32(a[0] + HIT_TOTAL_DAMAGE, 11.0);
            e.mem.set_f32(a[0] + HIT_FATIGUE_DAMAGE, 12.0);
            e.mem.set_f32(a[0] + HIT_ARMOR_DAMAGE, 13.0);
            e.mem.set_f32(a[0] + HIT_TARGETED_LIMB_DAMAGE, 14.0);
            ret(0)
        });
        actor_combat_hit(&mut e, attacker, target, 3, Ptr::NULL, 1);
        let copy = calls_to(&e, HIT_DATA_COPY);
        assert_eq!(copy.len(), 1);
        assert_eq!(copy[0][1], saved);
        let hit_data = copy[0][0];
        let init = calls_to(&e, HIT_DATA_INITIALIZE);
        assert_eq!(init.len(), 1);
        assert_ne!(init[0][0], hit_data);
        assert_eq!(
            init[0][1..],
            [attacker.addr(), target.addr(), weapon_entry, 3, 0]
        );
        assert_eq!(e.mem.f32(hit_data + HIT_LOCATION + 8), 3.0);
        assert_eq!(e.mem.f32(hit_data + HIT_DIRECTION + 4), 5.0);
        assert_eq!(e.mem.f32(hit_data + HIT_HEALTH_DAMAGE), 10.0);
        assert_eq!(e.mem.f32(hit_data + HIT_FATIGUE_DAMAGE), 12.0);
        assert_eq!(e.mem.f32(hit_data + HIT_ARMOR_DAMAGE), 13.0);
        assert_eq!(e.mem.f32(hit_data + HIT_TARGETED_LIMB_DAMAGE), 14.0);
        assert_eq!(
            calls_to(&e, ACTOR_HIT_ME)[0][..2],
            [target.addr(), hit_data]
        );

        // With the action's byte at +4 set, the damages are not merged.
        e.call_log = Some(vec![]);
        e.mem.set_u8(action + 4, 1);
        e.mem.set_f32(saved + HIT_HEALTH_DAMAGE, 99.0);
        actor_combat_hit(&mut e, attacker, target, 3, Ptr::NULL, 1);
        assert_eq!(call_count(&e, ACTOR_HIT_ME), 1);
    }

    #[test]
    fn actor_combat_hit_vats_scales_the_damage_of_a_source_reference() {
        let mut e = engine();
        let (attacker, target, _) = combat_hit_setup(&mut e);
        let slot_360 = slot_target(&e, attacker.cast(), ACTOR_SLOT_360);
        e.register_double(slot_360, |_, _| ret(1));
        stub_with(&mut e, ENTRY_FORM, |_, a| {
            ret(if a[0] == VATS_OBJECT { 4 } else { 0x8b00 })
        });
        let action = e.mem.alloc(0x20);
        let saved = e.mem.alloc(0x64);
        e.mem.set_u32(action + 0x14, saved);
        e.mem.set_u8(action + 4, 1);
        stub(&mut e, VATS_GET_CURRENT_ACTION, action);
        let source = e.mem.alloc(0x10);
        // The source reference is not flagged; the weapon fires 4
        // projectiles; the base object test passes and the reduction is 1.
        stub(&mut e, REFR_FLAG_TEST, 0);
        stub(&mut e, ITEM_CHANGE_HAS_MOD_EFFECT_ACTIVE, 1);
        stub(&mut e, SOURCE_WEAPON_LOOKUP, 0x9a00);
        stub(&mut e, WEAPON_GET_NUM_PROJECTILES, 4);
        stub(&mut e, ACTOR_BASE_FORM, 0x9b00);
        stub(&mut e, BASE_TEST_4FD3C0, 1);
        stub_float(&mut e, REFR_GETTER_885D70, 1.0);
        // The merge is skipped (action byte 1), so the damages the copy holds
        // are those of the saved data.
        e.mem.set_f32(saved + HIT_HEALTH_DAMAGE, 16.0);
        stub_with(&mut e, HIT_DATA_COPY, |e, a| {
            for offset in [
                HIT_HEALTH_DAMAGE,
                HIT_TOTAL_DAMAGE,
                HIT_TARGETED_LIMB_DAMAGE,
            ] {
                let value = e.mem.f32(a[1] + offset);
                e.mem.set_f32(a[0] + offset, value);
            }
            ret(0)
        });
        e.mem.set_f32(saved + HIT_TOTAL_DAMAGE, 20.0);
        e.mem.set_f32(saved + HIT_TARGETED_LIMB_DAMAGE, 8.0);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::new(source), 0);
        // The source reference is recorded and the count passed on.
        let lookup = calls_to(&e, SOURCE_WEAPON_LOOKUP);
        assert_eq!(lookup, vec![vec![source, 1, 0, attacker.addr()]]);
        let hit_me = calls_to(&e, ACTOR_HIT_ME);
        let hit_data = hit_me[0][1];
        assert_eq!(e.mem.u32(hit_data + HIT_SOURCE_REF), source);
        // 16 / 4 = 4 health, then times (1 - (4 - 1) / 4) = 0.25 -> 1.
        assert_eq!(e.mem.f32(hit_data + HIT_HEALTH_DAMAGE), 1.0);
        // 20 / 4 = 5 -> 1.25; 8 / 4 = 2 -> 0.5.
        assert_eq!(e.mem.f32(hit_data + HIT_TOTAL_DAMAGE), 1.25);
        assert_eq!(e.mem.f32(hit_data + HIT_TARGETED_LIMB_DAMAGE), 0.5);
    }

    #[test]
    fn actor_combat_hit_vats_clears_a_flagged_source_reference() {
        let mut e = engine();
        let (attacker, target, _) = combat_hit_setup(&mut e);
        let slot_360 = slot_target(&e, attacker.cast(), ACTOR_SLOT_360);
        e.register_double(slot_360, |_, _| ret(1));
        stub_with(&mut e, ENTRY_FORM, |_, a| {
            ret(if a[0] == VATS_OBJECT { 4 } else { 0x8b00 })
        });
        let action = e.mem.alloc(0x20);
        let saved = e.mem.alloc(0x64);
        e.mem.set_u32(action + 0x14, saved);
        e.mem.set_u8(action + 4, 1);
        stub(&mut e, VATS_GET_CURRENT_ACTION, action);
        let source = e.mem.alloc(0x10);
        stub(&mut e, REFR_FLAG_TEST, 1);
        stub_with(&mut e, HIT_DATA_COPY, |e, a| {
            e.mem.set_f32(a[0] + HIT_HEALTH_DAMAGE, 9.0);
            e.mem.set_u32(a[0] + HIT_CRITICAL_EFFECT, 0x77);
            ret(0)
        });
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::new(source), 0);
        let flag_test = calls_to(&e, REFR_FLAG_TEST);
        assert_eq!(flag_test, vec![vec![source, 0x20000]]);
        let hit_data = calls_to(&e, ACTOR_HIT_ME)[0][1];
        assert_eq!(e.mem.u32(hit_data + HIT_CRITICAL_EFFECT), 0);
        assert_eq!(e.mem.f32(hit_data + HIT_HEALTH_DAMAGE), 0.0);
        assert_eq!(
            calls_to(&e, HIT_DATA_CLEAR_EFFECT),
            vec![vec![hit_data, 0x3fc, 0]]
        );
        // The damage is zero, so the projectile split is skipped.
        assert_eq!(call_count(&e, SOURCE_WEAPON_LOOKUP), 0);
    }

    #[test]
    fn actor_combat_hit_casts_the_weapon_effect_past_its_threshold() {
        let mut e = engine();
        let (attacker, target, weapon_entry) = combat_hit_setup(&mut e);
        // The form's list holds a record at node + 0x18 whose embedded
        // object (record + 0xc) has a slot 8 reporting 9.0 against the
        // entry's threshold of 5.0.
        let node = e.mem.alloc(0x60);
        stub_with(&mut e, NODE_NEXT, move |_, a| {
            ret(if a[0] == 0x8b74 { node } else { 0 })
        });
        embedded_table(&mut e, node + 0x18 + 0xc, &[(8, float_ret(9.0))]);
        stub_float(&mut e, ITEM_CHANGE_GETTER_4BD350, 5.0);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        let process = e.get(attacker, Actor::pCurrentProcess);
        let slot_580 = slot_target(&e, process, PROCESS_SLOT_580);
        assert_eq!(calls_to(&e, slot_580), vec![vec![process.addr(), 1, 0, 0]]);
        let value_slot = e.mem.u32(e.mem.u32(node + 0x18 + 0xc) + 8);
        assert_eq!(
            calls_to(&e, value_slot),
            vec![vec![node + 0x18 + 0xc, attacker.addr()]]
        );
        assert_eq!(
            calls_to(&e, ITEM_CHANGE_GETTER_4BD350),
            vec![vec![weapon_entry]]
        );

        // Below the threshold nothing happens.
        e.call_log = Some(vec![]);
        stub_float(&mut e, ITEM_CHANGE_GETTER_4BD350, 9.0);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, slot_580), 0);

        // Without a form record the entry's poison is used (record + 0x30).
        e.call_log = Some(vec![]);
        stub(&mut e, NODE_NEXT, 0);
        let poison = e.mem.alloc(0x60);
        embedded_table(&mut e, poison + 0x30 + 0xc, &[(8, float_ret(9.0))]);
        stub(&mut e, ITEM_CHANGE_GET_POISON, poison);
        stub_float(&mut e, ITEM_CHANGE_GETTER_4BD350, 5.0);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, slot_580), 1);
        // A hit that does not connect (slot 0x3d8 false) casts nothing.
        e.call_log = Some(vec![]);
        let connects = slot_target(&e, attacker.cast(), ACTOR_SLOT_3D8);
        e.register_double(connects, |_, _| ret(0));
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, slot_580), 0);
    }

    #[test]
    fn actor_combat_hit_wears_the_weapon_and_clears_use_weapon_damage() {
        let mut e = engine();
        let (attacker, target, weapon_entry) = combat_hit_setup(&mut e);
        let slot_360 = slot_target(&e, attacker.cast(), ACTOR_SLOT_360);
        e.register_double(slot_360, |_, _| ret(1));
        stub_with(&mut e, ENTRY_FORM, |_, a| {
            ret(if a[0] == VATS_OBJECT { 0 } else { 0x8b00 })
        });
        stub_with(&mut e, HIT_DATA_INITIALIZE, |e, a| {
            e.mem.set_f32(a[0] + HIT_HEALTH_DAMAGE, 12.5);
            e.mem.set_f32(a[0] + HIT_DAMAGE_TO_WEAPON, 1.5);
            ret(0)
        });
        stub(&mut e, GET_CURRENT_PACKAGE, 0x9c00);
        stub(&mut e, PACKAGE_GET_USE_WEAPON_PACKAGE_DATA, 0x9d00);
        stub(&mut e, USE_WEAPON_DATA_TEST, 1);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        let wear = slot_target(&e, attacker.cast(), ACTOR_SLOT_3C8);
        assert_eq!(
            calls_to(&e, wear),
            vec![vec![attacker.addr(), weapon_entry, 1.5f32.to_bits(), 0]]
        );
        // The use-weapon package data zeroes the damage before the hit.
        let hit_data = calls_to(&e, ACTOR_HIT_ME)[0][1];
        assert_eq!(e.mem.f32(hit_data + HIT_HEALTH_DAMAGE), 0.0);
        assert_eq!(
            calls_to(&e, PACKAGE_GET_USE_WEAPON_PACKAGE_DATA),
            vec![vec![0x9c00]]
        );

        // A weapon form with the flag bit set is not worn.
        e.call_log = Some(vec![]);
        stub(&mut e, FORM_FLAG_100_BIT_5, 1);
        actor_combat_hit(&mut e, attacker, target, 0, Ptr::NULL, 0);
        assert_eq!(call_count(&e, wear), 0);
    }

    /// Allocates a zero cell per setting address and makes the three
    /// setting-pointer functions return the cell of the address they are
    /// called on. Returns the address-to-cell table.
    fn setting_cells(e: &mut Engine, addresses: &[u32]) -> std::collections::HashMap<u32, u32> {
        let mut table = std::collections::HashMap::new();
        for address in addresses {
            table.insert(*address, e.mem.alloc(8));
        }
        for function in [
            SETTING_GET_VALUE_POINTER,
            SETTING_GET_INT_POINTER,
            SETTING_GET_BOOL_POINTER,
        ] {
            let copy = table.clone();
            e.register_double(function, move |_, a| ret(copy[&a[0]]));
        }
        table
    }

    /// An actor of `0x1000` bytes with the given virtual slots (the
    /// player's `+0xe2d` byte is read by `Kill`).
    fn big_actor(e: &mut Engine, slots: &[(u32, Ret)]) -> Ptr<Actor> {
        object(e, 0x1000, slots).cast::<Actor>()
    }

    /// The slots a killed NPC answers.
    fn npc_slots() -> Vec<(u32, Ret)> {
        vec![
            (ACTOR_SLOT_22C, ret(0)),
            (ACTOR_SLOT_1A0, ret(0)),
            (ACTOR_SLOT_360, ret(0)),
            (ACTOR_SLOT_2E8, ret(0)),
            (ACTOR_SLOT_218, ret(1)),
            (ACTOR_SLOT_21C, ret(0)),
            (ACTOR_SLOT_STOP_COMBAT, ret(0)),
            (ACTOR_SLOT_234, ret(0)),
            (ACTOR_SLOT_428, ret(0)),
            (ACTOR_SLOT_1D0, ret(0)),
            (ACTOR_SLOT_48, ret(0)),
            (ACTOR_SLOT_2A0, ret(0)),
            (ACTOR_SLOT_448, ret(0)),
            (ACTOR_SLOT_38C, ret(0)),
            (ACTOR_SLOT_MODIFY_ACTOR_VALUE, ret(0)),
            (ACTOR_SLOT_GET_HEADING, float_ret(0.0)),
            (PLAYER_SLOT_488, ret(0)),
        ]
    }

    /// The slots a process object answers during `Kill`.
    fn process_slots() -> Vec<(u32, Ret)> {
        vec![
            (PROCESS_SLOT_1C4, ret(0)),
            (PROCESS_SLOT_6C4, ret(0)),
            (PROCESS_SLOT_180, ret(0)),
            (PROCESS_SLOT_310, ret(0)),
            (PROCESS_SLOT_4C8, ret(0)),
            (PROCESS_SLOT_4D0, ret(0)),
            (PROCESS_SLOT_660, ret(0)),
            (PROCESS_SLOT_28, ret(0)),
            (PROCESS_SLOT_6A0, ret(0)),
            (PROCESS_SLOT_4B4, ret(0)),
            (PROCESS_SLOT_40C, ret(0)),
            (PROCESS_SLOT_CURRENT_WEAPON, ret(0)),
            (PROCESS_SLOT_52C, ret(0)),
            (PROCESS_SLOT_730, float_ret(0.0)),
            (PROCESS_SLOT_3F4, ret(0)),
            (PROCESS_SLOT_614, ret(0)),
            (PROCESS_SLOT_E8, ret(0)),
            (PROCESS_SLOT_2FC, ret(0)),
            (PROCESS_SLOT_534, ret(0)),
        ]
    }

    /// A victim with a process, a player and the settings, ready for
    /// `actor_kill`. Returns (victim, process, player, setting cells).
    fn kill_setup(
        e: &mut Engine,
    ) -> (
        Ptr<Actor>,
        Ptr,
        Ptr<Actor>,
        std::collections::HashMap<u32, u32>,
    ) {
        let cells = setting_cells(
            e,
            &[
                ESSENTIAL_SETTING,
                DEATH_REPORT_SETTING,
                LOW_LEVEL_HEALTH_SETTING,
                ESSENTIAL_DOWN_TIMER_SETTING,
                KARMA_REWARD_SETTING,
                KARMA_REWARD_CREATURE_SETTING,
                KARMA_REWARD_ALIGNMENT_SETTINGS[0],
                KARMA_REWARD_ALIGNMENT_SETTINGS[2],
                KARMA_REWARD_ALIGNMENT_SETTINGS[3],
                KARMA_REWARD_ALIGNMENT_SETTINGS[4],
                WEAPON_NUMBER_SETTING,
                DEATH_VOICE_DISTANCE_SETTING,
                GREETING_TIMER_SETTING,
                DEATH_IMPULSE_SETTING,
                COMBATANT_COUNT_SETTING,
                DROP_WEAPON_CHANCE_SETTING,
                SLOW_MO_SETTING,
            ],
        );
        let process = object(e, 0x800, &process_slots());
        let player = big_actor(e, &npc_slots());
        e.mem.set_u32(player.addr() + 0xa4, 0);
        set_player(e, player);
        let victim = big_actor(e, &npc_slots());
        e.set(victim, Actor::pCurrentProcess, process);
        embedded_table(
            e,
            victim.addr() + ACTOR_VALUE_OWNER_OFFSET,
            &[(0x00, ret(100)), (0x0c, float_ret(30.0))],
        );
        e.map(TES_SINGLETON, 4);
        e.set_global(TES_SINGLETON, 0x8800u32);
        e.map(COMBAT_MANAGER, 4);
        e.set_global(COMBAT_MANAGER, 0x8900u32);
        e.map(VATS_OBJECT, 0x10);
        e.map(MAGIC_SHADER_HOLDER, 4);
        let shader_owner = e.mem.alloc(0x700);
        e.set_global(MAGIC_SHADER_HOLDER, shader_owner);
        e.map(COMBAT_DIALOGUE_MANAGER, 4);
        let karma_base = e.mem.alloc(0x200);
        embedded_table(e, karma_base + 0x100, &[(0x0c, float_ret(0.0))]);
        stub(e, ACTOR_BASE_FORM, karma_base);
        e.map(0x011f_158c, 4);
        stub(e, GET_SAVED_ACQUIRE_OBJECT, process.addr());
        stub_with(e, OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        stub(e, ACTOR_GET_ANIM_GROUP_ADDRESS, 0xff);
        (victim, process, player, cells)
    }

    #[test]
    fn actor_kill_does_nothing_for_protected_actors() {
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        let slot = slot_target(&e, victim.cast(), ACTOR_SLOT_22C);
        e.register_double(slot, |_, _| ret(1));
        actor_kill(&mut e, victim, Ptr::NULL, 3.0);
        assert_eq!(call_count(&e, SOUND_HANDLE_CONSTRUCT), 0);

        // Slot 0x1a0 stops everything but the player.
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        let slot = slot_target(&e, victim.cast(), ACTOR_SLOT_1A0);
        e.register_double(slot, |_, _| ret(1));
        actor_kill(&mut e, victim, Ptr::NULL, 3.0);
        assert_eq!(call_count(&e, SOUND_HANDLE_CONSTRUCT), 0);
    }

    #[test]
    fn actor_kill_forgets_the_sounds_and_queues_the_kill() {
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        let attacker = big_actor(&mut e, &npc_slots());
        stub(&mut e, PICK_UP_GOES_TO_TASK_QUEUE, 1);
        stub(&mut e, GET_TES, 0x8a00);
        stub(&mut e, SOUND_HANDLE_IS_PLAYING, 1);
        actor_kill(&mut e, victim, attacker, 7.5);
        let constructed = calls_to(&e, SOUND_HANDLE_CONSTRUCT);
        assert_eq!(constructed.len(), 3);
        let handles = [constructed[0][0], constructed[1][0], constructed[2][0]];
        assert_eq!(handles[1], handles[0] + 12);
        assert_eq!(handles[2], handles[0] + 24);
        // The extra data list is `this + 0x44` (accessor `005d43c0` is a
        // double returning 0 here).
        assert_eq!(
            calls_to(&e, EXTRA_GET_WEAPON_ATTACK_SOUND),
            vec![vec![0, handles[0]]]
        );
        assert_eq!(
            calls_to(&e, EXTRA_SET_WEAPON_IDLE_SOUND),
            vec![vec![0, handles[1]]]
        );
        // The attack and idle sounds fade out while they play; the awake
        // sound is stopped and released.
        assert_eq!(
            calls_to(&e, SOUND_HANDLE_FADE_OUT_AND_RELEASE),
            vec![vec![handles[0], 500], vec![handles[1], 500]]
        );
        assert_eq!(calls_to(&e, SOUND_HANDLE_STOP), vec![vec![handles[2]]]);
        assert_eq!(calls_to(&e, SOUND_HANDLE_RELEASE), vec![vec![handles[2]]]);
        // The kill is queued with the damage in a block and the killer
        // remembered.
        let queued = calls_to(&e, QUEUE_ACTOR_KILL);
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0][..2], [0x8a00, victim.addr()]);
        assert_eq!(queued[0][3], 0);
        assert_eq!(e.mem.f32(queued[0][2]), 7.5);
        assert_eq!(e.get(victim, Actor::pMyKiller).addr(), attacker.addr());
        // The handles are destroyed in reverse order and nothing else runs.
        assert_eq!(
            calls_to(&e, SOUND_HANDLE_DESTROY),
            vec![vec![handles[2]], vec![handles[1]], vec![handles[0]]]
        );
        assert_eq!(call_count(&e, ACTOR_SET_LIFE_STATE), 0);

        // A sound that is not playing is only released.
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        stub(&mut e, PICK_UP_GOES_TO_TASK_QUEUE, 1);
        stub(&mut e, GET_TES, 0x8a00);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(call_count(&e, SOUND_HANDLE_FADE_OUT_AND_RELEASE), 0);
        assert_eq!(call_count(&e, SOUND_HANDLE_RELEASE), 3);
    }

    #[test]
    fn actor_kill_without_a_process_finishes_with_the_death_stuff() {
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(
            calls_to(&e, ACTOR_SET_LIFE_STATE),
            vec![vec![victim.addr(), 1]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_HAVOK_WEAPON),
            vec![vec![victim.addr()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_DO_DEATH_STUFF),
            vec![vec![victim.addr()]]
        );
        assert_eq!(calls_to(&e, REFR_RUN_SCRIPT), vec![vec![victim.addr()]]);
        assert_eq!(e.mem.u8(victim.addr() + 0x118), 1);
        assert_eq!(e.mem.u8(victim.addr() + 0xc4), 0);
        assert_eq!(
            calls_to(&e, MOBILE_OBJECT_STOP_CURRENT_DIALOGUE),
            vec![vec![victim.addr()]]
        );
        let stop = slot_target(&e, victim.cast(), ACTOR_SLOT_STOP_COMBAT);
        assert_eq!(calls_to(&e, stop), vec![vec![victim.addr(), 0]]);
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_WORKER_96E2F0),
            vec![vec![PROCESS_LISTS, victim.addr()]]
        );
        assert_eq!(calls_to(&e, SOUND_HANDLE_DESTROY).len(), 3);
        // TES learns about the original base form.
        assert_eq!(calls_to(&e, TES_WORKER_459060).len(), 1);
    }

    #[test]
    fn actor_kill_clears_collision_and_ragdoll_state() {
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        let fixed = slot_target(&e, victim.cast(), ACTOR_SLOT_234);
        e.register_double(fixed, |_, _| ret(1));
        let collision = slot_target(&e, victim.cast(), ACTOR_SLOT_1D0);
        e.register_double(collision, |_, _| ret(0x9000));
        e.mem.set_u32(victim.addr() + 0xac, 0x9100);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(
            calls_to(&e, BHK_RIGID_BODY_SET_FIXED),
            vec![vec![0x9000, 0, 1]]
        );
        assert_eq!(
            calls_to(&e, BHK_RAGDOLL_DISABLE_RAGDOLL_ANIM),
            vec![vec![0x9100, 1]]
        );
    }

    #[test]
    fn actor_kill_by_the_player_records_the_statistics_and_karma() {
        let mut e = engine();
        let (victim, process, player, cells) = kill_setup(&mut e);
        // The player's side: its slot 0x360 is true, it can be killed...
        let player_360 = slot_target(&e, player.cast(), ACTOR_SLOT_360);
        e.register_double(player_360, |_, _| ret(1));
        e.mem
            .set_f32(cells[&KARMA_REWARD_ALIGNMENT_SETTINGS[0]], 5.0);
        stub(&mut e, GET_ALIGNMENT_FOR_KARMA, 0);
        stub(&mut e, ACTOR_HAS_FACTION_THAT_CARES_ABOUT_CRIME, 1);
        stub(&mut e, PLAYER_TEST_4997B0, 1);
        stub(&mut e, REFR_BASE_FORM, 0x9300);
        let _ = process;
        let karma_owner = e.mem.alloc(0x200);
        embedded_table(&mut e, karma_owner + 0x100, &[(0x0c, float_ret(10.0))]);
        stub(&mut e, ACTOR_BASE_FORM, karma_owner);
        stub_with(&mut e, FTOL, |_, a| {
            ret(f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32) as i32 as u32)
        });
        stub(&mut e, CREATURE_GETTER_59F3A0, 0);
        stub(&mut e, RTTI_DYNAMIC_CAST, 0x9400);
        actor_kill(&mut e, victim, player, 1.0);

        assert_eq!(
            calls_to(&e, HANDLE_ENTRY_POINT),
            vec![vec![0x15, player.addr(), victim.addr(), victim.addr()]]
        );
        // The three statistics calls: weapon type 0 + 1, slot 0x2d without
        // a weapon.
        let stats = calls_to(&e, STATISTICS_WORKER_5F5950);
        assert_eq!(stats.len(), 3);
        assert_eq!(stats[0], vec![1, 1, 0x9300, 0, 0, 0x2d, 1]);
        assert_eq!(stats[1], vec![0, 1, 0x9300, 0, 0, 0x2d, 1]);
        assert_eq!(stats[2], vec![2, 1, 0, 0, 0, 0, 1]);
        // The kill counts (NPC kill, overall) and the murderer mark.
        assert_eq!(calls_to(&e, MISC_STAT_INCREMENT), vec![vec![2], vec![0x23]]);
        assert_eq!(
            calls_to(&e, PLAYER_SET_IS_A_MURDERER),
            vec![vec![player.addr()]]
        );
        // Karma: alignment 0 uses its own setting; the amount is truncated.
        assert_eq!(
            calls_to(&e, GET_ALIGNMENT_FOR_KARMA),
            vec![vec![10.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, PLAYER_REWARD_KARMA),
            vec![vec![player.addr(), 5]]
        );
        // The player is not detected and holds no weapon: the murder mark
        // is cleared again.
        assert_eq!(e.mem.u8(victim.addr() + 0xc4), 0);
        assert_eq!(e.get(victim, Actor::pMyKiller).addr(), player.addr());
    }

    #[test]
    fn actor_kill_by_the_player_gives_experience() {
        let mut e = engine();
        let (victim, process, player, _) = kill_setup(&mut e);
        // The tracked damage is 12.0 against a base health of 100.
        let tracked = slot_target(&e, process, PROCESS_SLOT_730);
        e.register_double(tracked, |_, _| float_ret(12.0));
        stub(&mut e, TRACKED_DAMAGE_TEST, 1);
        stub(&mut e, ACTOR_GETTER_87F9F0, 0x77);
        stub(&mut e, GET_EXPERIENCE_POINTS, 50);
        stub(&mut e, PLAYER_INT_5BE4D0, 2);
        stub_float(&mut e, SCALE_EXPERIENCE, 100.0);
        stub_float(&mut e, FLOAT_ROUND, 100.0);
        stub_with(&mut e, FTOL, |_, a| {
            ret(f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32) as i32 as u32)
        });
        let player_488 = slot_target(&e, player.cast(), PLAYER_SLOT_488);
        e.register_double(player_488, |_, _| ret(0));
        actor_kill(&mut e, victim, player, 1.0);
        assert_eq!(
            calls_to(&e, TRACKED_DAMAGE_TEST),
            vec![vec![12.0f32.to_bits(), 100.0f32.to_bits()]]
        );
        // Experience points for a non-creature (flag 1) and the word.
        assert_eq!(calls_to(&e, GET_EXPERIENCE_POINTS), vec![vec![1, 0x77]]);
        assert_eq!(
            calls_to(&e, SCALE_EXPERIENCE),
            vec![vec![50.0f32.to_bits(), 2]]
        );
        assert_eq!(calls_to(&e, player_488), vec![vec![player.addr(), 100]]);
        // A teammate-owned victim (process level flag) only counts when the
        // attacker's slot 0x360 is true.
        e.call_log = Some(vec![]);
        stub(&mut e, PROCESS_GET_FORCE_NEXT_UPDATE, 1);
        actor_kill(&mut e, victim, player, 1.0);
        assert_eq!(call_count(&e, GET_EXPERIENCE_POINTS), 0);
    }

    #[test]
    fn actor_kill_of_an_essential_actor_at_a_low_process_level() {
        let mut e = engine();
        let (victim, _, _, cells) = kill_setup(&mut e);
        e.mem.set_u8(cells[&ESSENTIAL_SETTING], 1);
        e.mem.set_f32(cells[&LOW_LEVEL_HEALTH_SETTING], 0.5);
        stub(&mut e, ACTOR_GET_ESSENTIAL, 1);
        stub(&mut e, ACTOR_PROCESS_LEVEL_IS_3, 1);
        e.mem.set_u32(victim.addr() + 0xb0, 0x9500);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        // 0.5 * 100 - 30 = 20.
        let modify = slot_target(&e, victim.cast(), ACTOR_SLOT_MODIFY_ACTOR_VALUE);
        assert_eq!(
            calls_to(&e, modify),
            vec![vec![victim.addr(), 0x10, 20.0f32.to_bits(), 0]]
        );
        assert_eq!(call_count(&e, SOUND_HANDLE_DESTROY), 3);
        // The common tail does not run.
        assert_eq!(call_count(&e, BHK_RAGDOLL_PENETRATION_UTIL_CLEAR), 0);
        assert_eq!(call_count(&e, ACTOR_SET_LIFE_STATE), 0);
    }

    #[test]
    fn actor_kill_of_an_essential_actor_puts_it_into_life_state_six() {
        let mut e = engine();
        let (victim, process, _, cells) = kill_setup(&mut e);
        e.mem.set_u8(cells[&ESSENTIAL_SETTING], 1);
        e.mem.set_f32(cells[&ESSENTIAL_DOWN_TIMER_SETTING], 3.0);
        stub(&mut e, ACTOR_GET_ESSENTIAL, 1);
        stub(&mut e, ACTOR_GET_CURRENT_EDITOR_PACKAGE, 0x9600);
        stub(&mut e, PACKAGE_TYPE, 5);
        stub(&mut e, PROCESS_LEVEL_NUMBER, 6);
        stub(&mut e, SETTING_GET_STRING, 0x9700);
        stub(&mut e, REFR_GET_NAME, 0x9800);
        e.map(TWENTY, 4);
        e.set_global(TWENTY, 20.0f32);
        e.map(MESSAGE_DURATION, 4);
        e.set_global(MESSAGE_DURATION, 2.0f32);
        e.mem.set_u32(victim.addr() + 0xb0, 0);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        let down_timer = slot_target(&e, process, PROCESS_SLOT_E8);
        assert_eq!(
            calls_to(&e, down_timer),
            vec![vec![process.addr(), 3.0f32.to_bits()]]
        );
        let breath = slot_target(&e, process, PROCESS_SLOT_2FC);
        assert_eq!(
            calls_to(&e, breath),
            vec![vec![process.addr(), 20.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_END_INTERRUPT_PACKAGE),
            vec![vec![victim.addr(), 0]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_CLEAR_IN_COMBAT),
            vec![vec![victim.addr(), 1]]
        );
        assert_eq!(
            calls_to(&e, MAGIC_CASTER_INTERRUPT_CAST),
            vec![vec![victim.addr() + 0x88]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_SET_LIFE_STATE),
            vec![vec![victim.addr(), 6]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_RESTORE_FULL_HEALTH_AND_CONDITIONS),
            vec![vec![victim.addr()]]
        );
        // Level 6: the message "<name> <string>".
        let sprintf = calls_to(&e, SPRINTF);
        assert_eq!(sprintf.len(), 1);
        assert_eq!(sprintf[0][1..], [TWO_STRINGS_FORMAT, 0x9800, 0x9700]);
        assert_eq!(
            calls_to(&e, SHOW_MESSAGE),
            vec![vec![
                sprintf[0][0],
                0,
                MESSAGE_ICON_SURPRISED,
                0,
                2.0f32.to_bits(),
                0
            ]]
        );
        // The package of type 0xf would not be interrupted.
        e.call_log = Some(vec![]);
        stub(&mut e, PACKAGE_TYPE, 0xf);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(call_count(&e, ACTOR_END_INTERRUPT_PACKAGE), 0);
        // The dead flag is not set for an essential actor.
        assert_eq!(e.mem.u8(victim.addr() + 0x118), 0);
        assert_eq!(call_count(&e, ACTOR_DO_DEATH_STUFF), 0);
    }

    #[test]
    fn actor_kill_gives_the_body_an_impulse() {
        let mut e = engine();
        let (victim, process, _, cells) = kill_setup(&mut e);
        let node = e.mem.alloc(0x40);
        let collision = slot_target(&e, victim.cast(), ACTOR_SLOT_1D0);
        e.register_double(collision, move |_, _| ret(node));
        stub(&mut e, ACTOR_COLLISION_OBJECT, node);
        e.mem.set_f32(cells[&GREETING_TIMER_SETTING], 4.0);
        e.mem.set_f32(cells[&DEATH_IMPULSE_SETTING], 2.0);
        // The proxy velocity has a length: its direction is normalized.
        stub_float(&mut e, VECTOR_LENGTH_4586D0, 1.0);
        e.map(DEFAULT_DIRECTION, 12);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        let greeting = slot_target(&e, process, PROCESS_SLOT_4B4);
        assert_eq!(
            calls_to(&e, greeting),
            vec![vec![process.addr(), 4.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, BHK_WORLD_SET_MOTION),
            vec![vec![node, 1, 1, 1, 1]]
        );
        let scale = calls_to(&e, VECTOR_SCALE_IN_PLACE);
        assert_eq!(scale.len(), 1);
        assert_eq!(scale[0][1], 2.0f32.to_bits());
        let velocity = calls_to(&e, TES_HAVOK_ADD_VELOCITY);
        assert_eq!(velocity, vec![vec![node, scale[0][0], 0]]);
        let flags = slot_target(&e, victim.cast(), ACTOR_SLOT_48);
        assert_eq!(calls_to(&e, flags), vec![vec![victim.addr(), 4]]);
        assert_eq!(call_count(&e, VECTOR_COPY_458620), 1);
        // The weapon drop runs (it asks the process for the other-hand
        // item); no combatant limit is exceeded and no commanding actor.
        let other_hand = slot_target(&e, process, PROCESS_SLOT_534);
        assert_eq!(call_count(&e, other_hand), 1);
        assert_eq!(call_count(&e, ACTOR_DO_DEATH_STUFF), 0);
    }

    #[test]
    fn actor_kill_uses_the_heading_when_the_velocity_is_zero() {
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        let node = e.mem.alloc(0x40);
        stub(&mut e, ACTOR_COLLISION_OBJECT, node);
        stub_float(&mut e, VECTOR_LENGTH_4586D0, 0.0);
        let heading = slot_target(&e, victim.cast(), ACTOR_SLOT_GET_HEADING);
        e.register_double(heading, |_, _| float_ret(1.5));
        let result = e.mem.alloc(12);
        e.mem.set_f32(result, 0.5);
        stub(&mut e, VECTOR_MULTIPLY_4B4500, result);
        e.map(DEFAULT_DIRECTION, 12);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        let rotate = calls_to(&e, VECTOR_ROTATE_4A0C90);
        assert_eq!(rotate.len(), 1);
        assert_eq!(rotate[0][1], 1.5f32.to_bits());
        assert_eq!(call_count(&e, VECTOR_COPY_458620), 0);
        let multiply = calls_to(&e, VECTOR_MULTIPLY_4B4500);
        assert_eq!(multiply[0][2], UNIT_VECTOR);
        // The impulse direction is the result (normalized).
        let impulse = calls_to(&e, TES_HAVOK_ADD_VELOCITY);
        assert_eq!(e.mem.f32(impulse[0][1]), 0.5);
    }

    #[test]
    fn actor_kill_pushes_a_knocked_down_body() {
        let mut e = engine();
        let (victim, process, _, _) = kill_setup(&mut e);
        let node = e.mem.alloc(0x40);
        stub(&mut e, ACTOR_COLLISION_OBJECT, node);
        let knock = slot_target(&e, process, PROCESS_SLOT_40C);
        e.register_double(knock, |_, _| ret(3));
        let fixed = slot_target(&e, victim.cast(), ACTOR_SLOT_234);
        e.register_double(fixed, |_, _| ret(1));
        let collision = slot_target(&e, victim.cast(), ACTOR_SLOT_1D0);
        e.register_double(collision, move |_, _| ret(node));
        e.map(UPWARD_COMPONENT, 4);
        e.set_global(UPWARD_COMPONENT, 5.0f32);
        let result = e.mem.alloc(12);
        stub(&mut e, VECTOR_MULTIPLY_4B4500, result);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        // Un-fixed twice: at the start (slot 0x234) and for the push.
        assert_eq!(
            calls_to(&e, BHK_RIGID_BODY_SET_FIXED),
            vec![vec![node, 0, 1], vec![node, 0, 1]]
        );
        let up = calls_to(&e, VECTOR3_CONSTRUCT_416870);
        assert_eq!(
            up[0][1..],
            [0.0f32.to_bits(), 5.0f32.to_bits(), 0.0f32.to_bits()]
        );
        let set = calls_to(&e, TES_HAVOK_SET_VELOCITY);
        assert_eq!(set.len(), 1);
        assert_eq!(set[0][0], node);
        assert_eq!(set[0][2], 0);
        assert_eq!(call_count(&e, BHK_WORLD_SET_MOTION), 0);
    }

    #[test]
    fn actor_kill_plays_the_death_dialogue() {
        let mut e = engine();
        let (victim, process, _, cells) = kill_setup(&mut e);
        e.mem.set_f32(cells[&DEATH_VOICE_DISTANCE_SETTING], 50.0);
        e.map(COMBAT_DIALOGUE_MANAGER, 4);
        e.set_global(COMBAT_DIALOGUE_MANAGER, 0x8b00u32);
        let attacker = big_actor(&mut e, &npc_slots());
        let _ = process;
        actor_kill(&mut e, victim, attacker, 1.0);
        assert_eq!(
            calls_to(&e, COMBAT_DIALOGUE_START_DIALOGUE),
            vec![vec![0x8b00, victim.addr(), attacker.addr(), 2, 6, 1, 0]]
        );

        // The global flag with a distance beyond the setting suppresses it.
        let mut e = engine();
        let (victim, _, _, cells) = kill_setup(&mut e);
        e.mem.set_f32(cells[&DEATH_VOICE_DISTANCE_SETTING], 50.0);
        e.map(COMBAT_DIALOGUE_MANAGER, 4);
        stub(&mut e, GLOBAL_FLAG_525420, 1);
        stub_float(&mut e, REFR_GET_DISTANCE_FROM_REFERENCE, 80.0);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(call_count(&e, COMBAT_DIALOGUE_START_DIALOGUE), 0);
        // ... and a player within the distance lets the dialogue happen.
        e.call_log = Some(vec![]);
        e.set_global(COMBAT_DIALOGUE_MANAGER, 0x8b00u32);
        stub_float(&mut e, REFR_GET_DISTANCE_FROM_REFERENCE, 10.0);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(call_count(&e, COMBAT_DIALOGUE_START_DIALOGUE), 1);
    }

    #[test]
    fn actor_kill_of_a_creature_plays_its_death_sound() {
        let mut e = engine();
        let (victim, _, _, _) = kill_setup(&mut e);
        // A creature: slot 0x218 false and 0x21c true.
        for (slot, value) in [(ACTOR_SLOT_218, 0), (ACTOR_SLOT_21C, 1)] {
            let target = slot_target(&e, victim.cast(), slot);
            e.register_double(target, move |_, _| ret(value));
        }
        let position = e.mem.alloc(12);
        e.mem.set_f32(position, 1.0);
        let position_slot = slot_target(&e, victim.cast(), ACTOR_SLOT_POSITION);
        e.register_double(position_slot, move |_, _| ret(position));
        let node_slot = slot_target(&e, victim.cast(), ACTOR_SLOT_1D0);
        e.register_double(node_slot, |_, _| ret(0x9900));
        stub(&mut e, REFR_BASE_FORM, 0x9a00);
        stub(&mut e, CREATURE_PICK_CREATURE_SOUND, 0x9b00);
        stub(&mut e, FORM_NUMERIC_ID, 0x4242);
        stub(&mut e, AUDIO_INSTANCE, 0x9c00);
        stub(&mut e, ACTOR_BASE_FORM, 0);
        stub(&mut e, RTTI_DYNAMIC_CAST, 0);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(
            calls_to(&e, CREATURE_PICK_CREATURE_SOUND),
            vec![vec![0x9a00, 8]]
        );
        let play = calls_to(&e, AUDIO_GET_SOUND_HANDLE_BY_NUMERIC_ID);
        assert_eq!(play.len(), 1);
        assert_eq!(play[0][0], 0x9c00);
        assert_eq!(play[0][2..], [0x4242, 0x102]);
        let handle = play[0][1];
        assert_eq!(
            calls_to(&e, SOUND_HANDLE_SET_POSITION_XYZ)[0][..2],
            [handle, 1.0f32.to_bits()]
        );
        assert_eq!(
            calls_to(&e, SOUND_HANDLE_SET_OBJECT_TO_FOLLOW),
            vec![vec![handle, 0x9900]]
        );
        assert_eq!(calls_to(&e, SOUND_HANDLE_PLAY), vec![vec![handle, 0]]);
        assert_eq!(call_count(&e, COMBAT_DIALOGUE_START_DIALOGUE), 0);
    }

    #[test]
    fn actor_kill_of_the_player_ends_the_world_around_it() {
        let mut e = engine();
        let (_, _, player, _) = kill_setup(&mut e);
        stub(&mut e, INTERFACE_IS_TOP_MENU_ID, 1);
        stub(&mut e, AUDIO_INSTANCE, 0x9c00);
        stub(&mut e, AUDIO_GET_SOUND_HANDLE_BY_NAME, 0x9d00);
        stub(&mut e, SETTING_TEST_454AF0, 1);
        let attacker = big_actor(&mut e, &npc_slots());
        e.mem.set_u32(player.addr() + 0x68, 0);
        let process = object(&mut e, 0x800, &process_slots());
        e.set(player, Actor::pCurrentProcess, process);
        embedded_table(
            &mut e,
            player.addr() + ACTOR_VALUE_OWNER_OFFSET,
            &[(0x00, ret(100)), (0x0c, float_ret(30.0))],
        );
        actor_kill(&mut e, player, attacker, 1.0);
        assert_eq!(calls_to(&e, PIPBOY_RADIO_ENABLE), vec![vec![0]]);
        assert_eq!(calls_to(&e, SOUND_FADE_WORKER), vec![vec![0x3e8, 0]]);
        let music = calls_to(&e, AUDIO_GET_SOUND_HANDLE_BY_NAME);
        assert_eq!(music[0][0], 0x9c00);
        assert_eq!(music[0][2..], [DEATH_MUSIC_NAME, 0x901]);
        assert_eq!(calls_to(&e, SOUND_HANDLE_PLAY), vec![vec![0x9d00, 0]]);
        assert_eq!(
            calls_to(&e, VATS_QUIT_PLAYBACK),
            vec![vec![VATS_OBJECT, 0, 0]]
        );
        assert_eq!(call_count(&e, SURGERY_MENU_CLOSE), 1);
        // The player is not VATS-playing: the VATS worker runs, and the
        // progress export remembers the killer only for the call.
        assert_eq!(call_count(&e, VATS_WORKER_9CA1F0), 1);
        assert_eq!(
            calls_to(&e, PLAYER_EXPORT_PROGRESS_DATA),
            vec![vec![player.addr(), 1]]
        );
        assert_eq!(e.get(player, Actor::pMyKiller).addr(), 0);
        // The chase bip (not slot 0x2a0) is used for the player.
        assert_eq!(
            calls_to(&e, MOBILE_OBJECT_SET_CHASE_BIP),
            vec![vec![player.addr(), 1]]
        );
    }

    #[test]
    fn actor_kill_tail_reports_and_updates_the_combat_counts() {
        let mut e = engine();
        let (victim, _, player, cells) = kill_setup(&mut e);
        e.mem.set_u8(cells[&DEATH_REPORT_SETTING], 1);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        stub(&mut e, FORM_NUMERIC_ID, 0x55);
        stub(&mut e, LOCATION_NAME, 0x9e00);
        stub(&mut e, ACTOR_BASE_FORM, 0x9f00);
        // The shader effect and the beam.
        e.mem.set_u8(player.addr() + 0xe2d, 1);
        let holder = e.global::<u32>(MAGIC_SHADER_HOLDER);
        e.mem.set_u32(holder + 0x638, 0xa100);
        e.mem.set_u32(victim.addr() + 0x1a0, 0xa200);
        e.mem.set_u32(victim.addr() + 0xb0, 0xa300);
        stub(&mut e, ACTOR_OUTPUT_931ED0, 0xa400);
        stub(&mut e, OBJECT_ACCESSOR_4A3A20, 0xa500);
        let collision = slot_target(&e, victim.cast(), ACTOR_SLOT_1D0);
        e.register_double(collision, |_, _| ret(0xa600));
        // The player kills with a teammate-free combat count of zero.
        stub(&mut e, GET_ALIGNMENT_FOR_KARMA, 1);
        e.map(SLOW_MO_SETTING, 4);
        e.mem.set_f32(cells[&SLOW_MO_SETTING], 0.25);
        let attacker = big_actor(&mut e, &npc_slots());
        let attacker_address = attacker.addr();
        actor_kill(&mut e, victim, attacker, 1.0);
        assert_eq!(
            calls_to(&e, ERROR_REPORT),
            vec![vec![KILLED_BY_FORMAT, 0x9e00, 0x55, 0x9e00, 0x55]]
        );
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_FINISH_MAGIC_SHADER_HIT_EFFECT),
            vec![vec![PROCESS_LISTS, victim.addr(), 0xa100]]
        );
        assert_eq!(
            calls_to(&e, CONTINUOUS_BEAM_PROJECTILE_SET_KILL),
            vec![vec![0xa200, victim.addr()]]
        );
        assert_eq!(call_count(&e, BHK_RAGDOLL_PENETRATION_UTIL_CLEAR), 1);
        let utility = calls_to(&e, PENETRATION_UTIL_WORKER);
        assert_eq!(utility, vec![vec![0xa300, 0xa600, 0xa500]]);
        let _ = attacker_address;

        // Without an attacker the other message is used.
        e.call_log = Some(vec![]);
        actor_kill(&mut e, victim, Ptr::NULL, 1.0);
        assert_eq!(
            calls_to(&e, ERROR_REPORT),
            vec![vec![DIED_WITH_NO_ATTACKER_FORMAT, 0x9e00, 0x55]]
        );
    }

    #[test]
    fn actor_kill_by_the_player_counts_combatants_and_the_slow_motion_camera() {
        let mut e = engine();
        let (victim, _, player, cells) = kill_setup(&mut e);
        stub(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0);
        e.mem.set_f32(cells[&SLOW_MO_SETTING], 0.25);
        stub(&mut e, PLAYER_INT_5BE4D0, 0);
        stub(&mut e, ACTOR_IS_FLEEING, 0);
        // No combatants: the slow-motion camera is requested.
        actor_kill(&mut e, victim, player, 1.0);
        let counted = calls_to(&e, COMBAT_MANAGER_GET_COMBATANT_COUNT);
        assert!(counted
            .iter()
            .any(|call| call[..2] == [0x8900, player.addr()]));
        assert_eq!(
            calls_to(&e, PLAYER_SET_SLOW_MO_CAMERA),
            vec![vec![
                player.addr(),
                victim.addr(),
                0.25f32.to_bits(),
                1,
                0xffff_ffff
            ]]
        );
        assert_eq!(call_count(&e, ACTOR_ADD_FACTION_MINOR_CRIME), 0);

        // With combatants that see the player, a minor crime is added and
        // there is no camera.
        e.call_log = Some(vec![]);
        stub(&mut e, COMBAT_MANAGER_GET_COMBATANT_COUNT, 2);
        stub(&mut e, PLAYER_IS_DETECTED_BY_NON_TEAMMATES, 1);
        actor_kill(&mut e, victim, player, 1.0);
        assert_eq!(
            calls_to(&e, ACTOR_ADD_FACTION_MINOR_CRIME),
            vec![vec![victim.addr(), 1, 1]]
        );
        assert_eq!(call_count(&e, PLAYER_SET_SLOW_MO_CAMERA), 0);
    }
}
