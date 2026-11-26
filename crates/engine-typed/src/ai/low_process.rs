//! `LowProcess` (the Xbox PDB's source unit of that name, in `fallout/ai`), the functions typed so
//! far: the process lock map (`NiTMap<LowProcess*, LP_LOCK_DATA>` and its base), the processed
//! date's parts, the flags and timers, the target and location setters and `ResetTarget`, the damage
//! modifiers, the package tests, starts, schedule and done step, the greeting, the procedure index
//! steps, the patrol (its init, wait at a reference and travel to the next), the clear of the
//! current data, the level copy, the time update, the door lock and unlock callbacks, the small
//! procedure steps, the release and clears of the acquire lists, the time delta, the flee, pursue,
//! remove-worn and sandbox procedures, the current destination's world space and cell, the loads'
//! resolves and the package's save to and load from the extra data, each as its translation in
//! `engine-fallout-ai` does it.
//!
//! The fields read here (Xbox PDB names, checked against the PC code): the embedded current package
//! (+4; its start hour the float at +0x10 of it), `iDateLastProcessed` (+0x24), `m_uFlags` (the
//! byte at +0x30), `pTarget` (+0x40), `pGenericLocation` and `pGenericSecondLocation` (+0x44,
//! +0x48), `pDamageModifiers` (+0x94), `fEssentialDownTimer` (+0xa4) and `fTrackedDamage` (+0xac).
//! A package's type is its word at +0x18 (`009611e0`), its procedure the signed byte at +0x20
//! (`0041ca90`), its flags +0x1c (`00441110`), its target +0x30 (`00671d10`; a target's reference
//! is its word at +4 when its kind byte at +0 is 0, `00680020`), its start script and topic +0x50
//! and +0x54 (`00903bd0`, `00903bb0`) and its change script and topic +0x70 and +0x74 (`00903cf0`,
//! `00903cd0`); a script's two words at +0x2c and +0x30 say whether it has anything to run. A
//! reference's flags are its word at +8 (targeted: 0x40000, `00564db0`), its base form +0x20 (a
//! form's type the byte at +4, `00401170`), its parent cell +0x40 (an interior's byte at +0x24 has
//! bit 0, `00425fd0`) and its extra data list +0x44 (`005d43c0`).
//!
//! The process classes' slots, as the exe's vtables hold them (`LowProcess` `010886e4`,
//! `MiddleLowProcess` `01089bcc`, `MiddleHighProcess` `0108904c`, `HighProcess` `01087864`): 0x12c is
//! [`set_target`], 0x8c [`set_target_for_package`], 0x90 [`set_location_for_package`] and 0x33c
//! [`enter_combat`] on the first two; 0x244 is the empty body `00483710`, 0x278 `00717e50` (the
//! embedded package: `this + 4`) and 0x27c `00859670` (the current package again) on the first two;
//! 0x22c is `007fa950` on all four; 0x280 and 0x284 are `BaseProcess`'s running procedure index
//! getter and setter on the first two. A call through one of them is the typed call, or the field
//! in place, when the slot holds that body, and a call through the slot otherwise.
//!
//! The callees whose bodies are one load, store or test are done in place (the getters above, the
//! flag setter `008d7e00`, the frame timer's step `0084d030`, the package's start hour `00621b00`,
//! the actor's test `00566950`, the target's setter `00403550`); a getter whose answer the exe never
//! reads (`0055b980` in [`set_location_for_package`]) is left out.

use super::actor_package::procedure_count;
use super::middle_low_process::{
    calendar_hour, current_package, loaded, package_kind, procedure, procedure_index_running,
    process_flag, set_procedure_index_running, set_process_flag, value_has_flag,
};
use super::{actor_package, base_process, FAMILY};
use crate::native::to_float;
use crate::{
    ai_queries, cell_refs, crt, dispatch, draw_f, extra_data, forms, pathing, slot_target,
};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// The lock map's vtables: `NiTMap<LowProcess*, LP_LOCK_DATA>` and its base `NiTMapBase`.
const LOCK_MAP_VTABLE: u32 = 0x0108_8fd4;
const LOCK_MAP_BASE_VTABLE: u32 = 0x0108_8ff4;
/// `NiTMapBase::RemoveAll` (each item through the map's own slots).
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;
/// `NiAlloc` and `NiFree` (`cdecl`, one word).
const NI_ALLOCATE: u32 = 0x00aa_1070;
const NI_FREE: u32 = 0x00aa_10f0;
/// The map allocator's item free (`thiscall` on the map's allocator at +0xc, one word).
const MAP_ALLOCATOR_DEALLOCATE: u32 = 0x006b_8310;
/// The free (`cdecl`, one word).
const FREE: u32 = 0x0040_1030;

/// `iDateLastProcessed`.
const DATE_LAST_PROCESSED: u32 = 0x24;
/// `m_uFlags`.
const PROCESS_FLAGS: u32 = 0x30;
/// `pTarget`.
const TARGET: u32 = 0x40;
/// `pGenericLocation` and `pGenericSecondLocation`.
const GENERIC_LOCATION: u32 = 0x44;
const GENERIC_SECOND_LOCATION: u32 = 0x48;
/// `pDamageModifiers` (a `ModifierList`).
const DAMAGE_MODIFIERS: u32 = 0x94;
/// `fEssentialDownTimer` and `fTrackedDamage`.
const ESSENTIAL_DOWN_TIMER: u32 = 0xa4;
const TRACKED_DAMAGE: u32 = 0xac;
/// An actor's `ActorValueOwner`.
const ACTOR_VALUE_OWNER: u32 = 0xa4;

/// The procedure lists, by package type: a list's word at the running procedure index is that
/// procedure (1 wander, 0x36 done).
const PROCEDURE_LISTS: u32 = 0x011a_3ff0;
const PROCEDURE_WANDER: u32 = 1;
const PROCEDURE_DONE: u32 = 0x36;
/// The patrol package's procedure.
const PATROL: i32 = 0xd;
/// The actor value whose damage modifier the damage changes, and the flag of the values that have
/// none.
const DAMAGED_ACTOR_VALUE: u32 = 0x10;
const NO_MODIFIERS: u32 = 0x100;
/// A door's form type.
const DOOR_FORM_TYPE: u8 = 0x1c;
/// A reference's targeted flag (in its flags word, +8).
const TARGETED: u32 = 0x0004_0000;
/// The process flag `fn_0090ac20` sets and clears.
const FLAG_40: u8 = 0x40;

/// The frame timer whose float at +0xc is the step (`0084d030`).
const FRAME_TIMER: u32 = 0x011f_6394;
/// The float -1.0 the package's slot 0x13c gets.
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
/// The point a package without a location answers.
const ZERO_POINT: u32 = 0x011f_426c;
/// The double the blocked time is compared with.
const BLOCKED_TIME_LIMIT: u32 = 0x0101_ffa0;

/// The callees called by address (not typed yet).
const GET_MODIFIER: u32 = 0x0093_7730;
const ADJUST_MODIFIER: u32 = 0x0093_7480;
const CHECK_CLAMP_DAMAGE_MODIFIER: u32 = 0x0066_eea0;
const GET_LOCATION_COORD: u32 = 0x0067_5de0;
const PATROL_DATA_INIT: u32 = 0x009f_2ba0;
const PATROL_SET_NEXT_REFERENCE: u32 = 0x009f_2ea0;
const PATROL_CURRENT_REFERENCE: u32 = 0x009f_3390;
const EXTRA_SET_PACKAGE_TARGET: u32 = 0x0041_cab0;
const EXTRA_SCRIPT_LOCALS: u32 = 0x0041_8830;
const SCRIPT_RUN: u32 = 0x005a_c1e0;
const ACTOR_AFTER_NEW_PACKAGE: u32 = 0x0088_4f80;
const REFERENCE_DISTANCE: u32 = 0x0057_23b0;
const ACTOR_STOP_MOVING: u32 = 0x008b_3ab0;
const PACKAGE_SET_IS_CREATED: u32 = 0x0067_4d70;
const TELEPORT_DATA: u32 = 0x0056_8e50;
const IS_AN_OWNER: u32 = 0x0057_85e0;
const GET_LOCK: u32 = 0x0056_9160;
const LOCK: u32 = 0x0056_9250;
const UNLOCK: u32 = 0x0056_92a0;
const SET_CONNECTED_CELLS_PUBLIC: u32 = 0x0043_a210;
const TELEPORT_WORLD_SPACE: u32 = 0x0043_a320;

// Typed from 009137b0
/// The `NiTMap<LowProcess*, LP_LOCK_DATA>` constructor: its base's ([`lock_map_base_construct`]),
/// then its own vtable. Returns `this`.
pub fn lock_map_construct(e: &mut Engine, this: Ptr, buckets: u32) -> Ptr {
    typed_call!(
        FAMILY,
        0x0091_3810,
        lock_map_base_construct(e, this, buckets)
    );
    e.mem.set_u32(this.addr(), LOCK_MAP_VTABLE);
    this
}

// Typed from 00913810
/// The `NiTMapBase` constructor: the vtable, the bucket count and an empty item count, then a
/// bucket table of `buckets * 4` bytes (`NiAlloc`) cleared (`memset`). Returns `this`.
pub fn lock_map_base_construct(e: &mut Engine, this: Ptr, buckets: u32) -> Ptr {
    let base = this.addr();
    e.mem.set_u32(base, LOCK_MAP_BASE_VTABLE);
    e.mem.set_u32(base.wrapping_add(4), buckets);
    e.mem.set_u32(base.wrapping_add(0xc), 0);
    let table = e.call(NI_ALLOCATE, &args![buckets << 2]).u32();
    e.mem.set_u32(base.wrapping_add(8), table);
    // The count is read again after the allocation, as the exe reads it.
    let size = e.mem.u32(base.wrapping_add(4)) << 2;
    typed_call!(
        crt::FAMILY,
        0x0040_3d30,
        crt::mem::memset_wrapper(e, Ptr::new(table), 0, size)
    );
    this
}

// Typed from 00913880
/// The `NiTMap<LowProcess*, LP_LOCK_DATA>` destructor: its vtable, every item removed, then its
/// base's ([`lock_map_base_destructor`]).
pub fn lock_map_destructor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), LOCK_MAP_VTABLE);
    e.call(MAP_REMOVE_ALL, &args![this]);
    typed_call!(FAMILY, 0x0091_38e0, lock_map_base_destructor(e, this));
}

// Typed from 009138e0
/// The `NiTMapBase` destructor: its vtable, every item removed, the bucket table freed (`NiFree`).
pub fn lock_map_base_destructor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), LOCK_MAP_BASE_VTABLE);
    e.call(MAP_REMOVE_ALL, &args![this]);
    let table = e.mem.u32(this.addr().wrapping_add(8));
    e.call(NI_FREE, &args![table]);
}

// Typed from 009137e0
/// `NiTMap<LowProcess*, LP_LOCK_DATA>::_scalar_deleting_destructor_`: [`lock_map_destructor`],
/// then `this` freed when bit 0 of `flags` is set. Returns `this`.
pub fn lock_map_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x0091_3880, lock_map_destructor(e, this));
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Typed from 00913980
/// `NiTMapBase<..>::_scalar_deleting_destructor_`: [`lock_map_base_destructor`], then `this`
/// freed when bit 0 of `flags` is set. Returns `this`.
pub fn lock_map_base_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x0091_38e0, lock_map_base_destructor(e, this));
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Typed from 00913910
/// The lock map's item delete: the item's value (+8, +0xc) set to a new `LP_LOCK_DATA` (both words
/// 0, [`lock_data_construct`]'s), then the item back to the map's allocator (+0xc).
pub fn lock_map_delete_item(e: &mut Engine, this: Ptr, item: u32) {
    e.mem.set_u32(item.wrapping_add(8), 0);
    e.mem.set_u32(item.wrapping_add(0xc), 0);
    e.call(
        MAP_ALLOCATOR_DEALLOCATE,
        &args![this.addr().wrapping_add(0xc), item],
    );
}

// Typed from 00913950
/// The `LP_LOCK_DATA` constructor: both words 0 (the stack word unread). Returns `this`.
pub fn lock_data_construct(e: &mut Engine, this: Ptr, _unused: u32) -> Ptr {
    e.mem.set_u32(this.addr(), 0);
    e.mem.set_u32(this.addr().wrapping_add(4), 0);
    this
}

/// `iDateLastProcessed`.
#[inline]
fn processed_date(e: &Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(DATE_LAST_PROCESSED))
}

// Typed from 009076a0
/// `LowProcess::GetProcessedDay` (Xbox PDB): bits 0-8 of `iDateLastProcessed`.
pub fn processed_day(e: &mut Engine, this: Ptr) -> u32 {
    processed_date(e, this) & 0x1ff
}

// Typed from 009076c0
/// `LowProcess::GetProcessedMonth` (Xbox PDB): bits 9-12 of `iDateLastProcessed`.
pub fn processed_month(e: &mut Engine, this: Ptr) -> u32 {
    (processed_date(e, this) >> 9) & 0xf
}

// Typed from 009076e0
/// `LowProcess::GetProcessedYear` (Xbox PDB): bits 13-26 of `iDateLastProcessed`.
pub fn processed_year(e: &mut Engine, this: Ptr) -> u32 {
    (processed_date(e, this) >> 13) & 0x3fff
}

// Typed from 0090ac20
/// Sets (non-zero `set`) or clears the process flag 0x40 (`008d7e00` in place).
pub fn set_flag_40(e: &mut Engine, this: Ptr, set: u8) {
    let at = this.addr().wrapping_add(PROCESS_FLAGS);
    let flags = e.mem.u8(at);
    e.mem.set_u8(
        at,
        if set != 0 {
            flags | FLAG_40
        } else {
            flags & !FLAG_40
        },
    );
}

// Typed from 00907620
/// `LowProcess::ModEssentialDownTimer` (engine map): `fEssentialDownTimer` loses the frame
/// timer's step (both floats: one difference rounded to a float, the exe's on either thread).
pub fn mod_essential_down_timer(e: &mut Engine, this: Ptr) {
    let step = loaded(e.mem.u32(FRAME_TIMER.wrapping_add(0xc)));
    let at = this.addr().wrapping_add(ESSENTIAL_DOWN_TIMER);
    let left = to_float(loaded(e.mem.u32(at)) - step);
    e.mem.set_u32(at, left.to_bits());
}

// Typed from 009134c0
/// `LowProcess::ModTrackedDamage` (engine map): for an actor whose slot 0x360 or byte at +0x18d
/// (`00566950`) is set, `fTrackedDamage` grows by `amount` (one sum rounded to a float).
pub fn mod_tracked_damage(e: &mut Engine, this: Ptr, actor: u32, amount: f32) {
    if actor == 0 {
        return;
    }
    if !e.vcall(actor, 0x360, &[]).bool() && e.mem.u8(actor.wrapping_add(0x18d)) == 0 {
        return;
    }
    let at = this.addr().wrapping_add(TRACKED_DAMAGE);
    let sum = to_float(loaded(e.mem.u32(at)) + loaded(amount.to_bits()));
    e.mem.set_u32(at, sum.to_bits());
}

// Typed from 00911ae0
/// A `LowProcess` slot that answers 0.0 (both stack words unread).
pub fn float_zero(_e: &mut Engine, _this: Ptr, _unused_0: u32, _unused_1: u32) -> f32 {
    0.0
}

// Typed from 00908f40
/// `LowProcess::ProcessAvoidArea` (engine map): the running procedure moves on by one (the
/// process's slot 0x288 with the actor and 1).
pub fn process_avoid_area(e: &mut Engine, this: Ptr, actor: u32) {
    add_to_procedure_index_running_slot(e, this.addr(), actor, 1);
}

// Typed from 00911ed0
/// `LowProcess::ProcessSummonCreatureDefend` (engine map): the process's slot 0x298 with the
/// actor, 0, 0x101 and 0.
pub fn process_summon_creature_defend(e: &mut Engine, this: Ptr, actor: u32) {
    e.vcall(this.addr(), 0x298, &args![actor, 0u32, 0x101u32, 0u32]);
}

// Typed from 00912960
/// `LowProcess::ProcessPatrolStartAtRefBehavior` (Xbox PDB): `SetupNewPackage` (the process's slot
/// 0x244), then the patrol data's float at +8 cleared and its state at +4 set to 2 (the first word
/// unread).
pub fn process_patrol_start_at_ref_behavior(e: &mut Engine, this: Ptr, _unused: u32, data: Ptr) {
    if slot_target(e, this.addr(), 0x244) != 0x0048_3710 {
        e.vcall(this.addr(), 0x244, &[]);
    }
    e.mem.set_u32(data.addr().wrapping_add(8), 0);
    e.mem.set_u32(data.addr().wrapping_add(4), 2);
}

// Typed from 0090dc10
/// `LowProcess::EnterCombat` (engine map; thirteen stack words, answers `true`): the actor's slot
/// 0x424 with words 1 and 4, the low bytes of words 9, 7 and 5, 0, the low byte of word 11 and word
/// 12 (the actor is word 0; words 2, 3, 6, 8 and 10 are unread).
#[allow(clippy::too_many_arguments)]
pub fn enter_combat(
    e: &mut Engine,
    _this: Ptr,
    actor: u32,
    word_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    word_4: u32,
    word_5: u8,
    _unused_6: u32,
    word_7: u8,
    _unused_8: u32,
    word_9: u8,
    _unused_10: u32,
    word_11: u8,
    word_12: u32,
) -> bool {
    e.vcall(
        actor,
        0x424,
        &args![
            word_1,
            word_4,
            u32::from(word_9),
            u32::from(word_7),
            u32::from(word_5),
            0u32,
            u32::from(word_11),
            word_12
        ],
    );
    true
}

/// The process's slot 0x278: its embedded package (`this + 4`) when the slot holds `00717e50`.
fn running_actor_package(e: &mut Engine, process: u32) -> u32 {
    if slot_target(e, process, 0x278) == 0x0071_7e50 {
        process.wrapping_add(4)
    } else {
        e.vcall(process, 0x278, &[]).u32()
    }
}

/// The process's slot 0x27c: its current package (`00859670`, the slot 0x22c again) when the slot
/// holds that body.
fn running_package(e: &mut Engine, process: u32) -> u32 {
    if slot_target(e, process, 0x27c) == 0x0085_9670 {
        typed_call!(
            ai_queries::FAMILY,
            0x0085_9670,
            ai_queries::queries::package_that_is_running(e, process)
        )
    } else {
        e.vcall(process, 0x27c, &[]).u32()
    }
}

/// The process's slot 0x12c ([`set_target`] on the low processes).
fn set_target_slot(e: &mut Engine, process: u32, target: u32) {
    if slot_target(e, process, 0x12c) == 0x0091_1e80 {
        typed_call!(
            FAMILY,
            0x0091_1e80,
            set_target(e, Ptr::new(process), Ptr::new(target))
        );
    } else {
        e.vcall(process, 0x12c, &args![target]);
    }
}

/// The process's slot 0x8c ([`set_target_for_package`] on the low processes); its answer unread.
fn set_target_for_package_slot(e: &mut Engine, process: u32, actor: u32) {
    if slot_target(e, process, 0x8c) == 0x0090_8bf0 {
        typed_call!(
            FAMILY,
            0x0090_8bf0,
            set_target_for_package(e, Ptr::new(process), Ptr::new(actor))
        );
    } else {
        e.vcall(process, 0x8c, &args![actor]);
    }
}

/// The process's slot 0x90 ([`set_location_for_package`] on the low processes) with 0; its answer
/// unread.
fn set_location_for_package_slot(e: &mut Engine, process: u32, actor: u32) {
    if slot_target(e, process, 0x90) == 0x0090_8c50 {
        typed_call!(
            FAMILY,
            0x0090_8c50,
            set_location_for_package(e, Ptr::new(process), Ptr::new(actor), 0)
        );
    } else {
        e.vcall(process, 0x90, &args![actor, 0u32]);
    }
}

// Typed from 00911e80
/// `LowProcess::SetTarget` (Xbox PDB): a target is marked targeted (`SetTargeted(1)`, its flag
/// 0x40000), becomes `pTarget` and is handed to the running package (the slot 0x278's package,
/// its word at +8).
pub fn set_target(e: &mut Engine, this: Ptr, target: Ptr) {
    let target = target.addr();
    if target != 0 {
        let at = target.wrapping_add(8);
        let flags = e.mem.u32(at);
        e.mem.set_u32(at, flags | TARGETED);
    }
    e.mem.set_u32(this.addr().wrapping_add(TARGET), target);
    let package = running_actor_package(e, this.addr());
    e.mem.set_u32(package.wrapping_add(8), target);
}

// Typed from 00908bf0
/// `LowProcess::SetTargetForPackage` (Xbox PDB): the target `GetTargetForPackage` (the process's
/// slot 0x810) gives for the actor and the current package becomes the target (slot 0x12c);
/// answers whether `pTarget` is set.
pub fn set_target_for_package(e: &mut Engine, this: Ptr, actor: Ptr) -> bool {
    let process = this.addr();
    let package = current_package(e, process);
    let target = e.vcall(process, 0x810, &args![actor, package]).u32();
    set_target_slot(e, process, target);
    e.mem.u32(process.wrapping_add(TARGET)) != 0
}

// Typed from 00908c50
/// `LowProcess`'s `SetLocationForPackage` (slot 0x90): 0 without a current package; otherwise the
/// location `GetLocationForPackage` (slot 0x814) gives for the actor, the package and `second`
/// becomes `pGenericLocation` (`second` 0) or `pGenericSecondLocation`, and is the answer.
pub fn set_location_for_package(e: &mut Engine, this: Ptr, actor: Ptr, second: u8) -> u32 {
    let process = this.addr();
    let package = current_package(e, process);
    if package == 0 {
        return 0;
    }
    let location = e
        .vcall(process, 0x814, &args![actor, package, u32::from(second)])
        .u32();
    let field = if second == 0 {
        GENERIC_LOCATION
    } else {
        GENERIC_SECOND_LOCATION
    };
    e.mem.set_u32(process.wrapping_add(field), location);
    location
}

// Typed from 0090a130
/// `LowProcess::GetCurrentPackageLocation` (engine map): the current package's location point
/// (`TESPackage::GetLocationCoord(out, actor)`), or the zero point without a package. Answers
/// `out`.
pub fn current_package_location(e: &mut Engine, this: Ptr, out: u32, actor: u32) -> u32 {
    let process = this.addr();
    if current_package(e, process) != 0 {
        let package = current_package(e, process);
        e.call(GET_LOCATION_COORD, &args![package, out, actor]);
        return out;
    }
    for offset in [0u32, 4, 8] {
        let word = e.mem.u32(ZERO_POINT.wrapping_add(offset));
        e.mem.set_u32(out.wrapping_add(offset), word);
    }
    out
}

// Typed from 009074f0
/// `LowProcess::GetDamageModifier` (Xbox PDB): the damage modifiers' `GetModifier` for the actor
/// value (its ST0).
pub fn damage_modifier(e: &mut Engine, this: Ptr, code: u8) -> f64 {
    e.call(
        GET_MODIFIER,
        &args![this.addr().wrapping_add(DAMAGE_MODIFIERS), u32::from(code)],
    )
    .st0
}

// Typed from 00907460
/// `LowProcess::DamageModActorValue` (engine map): only for the actor value 0x10, when it has
/// modifiers, the damage modifiers are adjusted (kind 0) by `ActorValue::CheckClampDamageModifier`
/// of the actor's value owner (0 without an actor), the value and `amount`, stored as a float.
pub fn damage_mod_actor_value(e: &mut Engine, this: Ptr, actor: u32, code: u32, amount: f32) {
    if code != DAMAGED_ACTOR_VALUE || value_has_flag(e, code, NO_MODIFIERS) {
        return;
    }
    let owner = if actor != 0 {
        actor.wrapping_add(ACTOR_VALUE_OWNER)
    } else {
        0
    };
    let clamped = e
        .call(
            CHECK_CLAMP_DAMAGE_MODIFIER,
            &args![owner, code, x87_quiet_bits(amount.to_bits())],
        )
        .st0;
    e.call(
        ADJUST_MODIFIER,
        &args![
            this.addr().wrapping_add(DAMAGE_MODIFIERS),
            code & 0xff,
            to_float(clamped).to_bits(),
            0u32
        ],
    );
}

/// The procedure the list of package type `kind` holds at `index`.
fn listed_procedure(e: &Engine, kind: u32, index: u32) -> u32 {
    let list = e
        .mem
        .u32(PROCEDURE_LISTS.wrapping_add(kind.wrapping_mul(4)));
    e.mem.u32(list.wrapping_add(index.wrapping_mul(4)))
}

// Typed from 0090cfa0
/// `LowProcess::IsWandering` (engine map): with a current package, whether its type's procedure at
/// the running procedure index is the wander (1).
pub fn is_wandering(e: &mut Engine, this: Ptr) -> bool {
    let process = this.addr();
    if current_package(e, process) == 0 {
        return false;
    }
    let package = current_package(e, process);
    let kind = package_kind(e, package);
    let index = procedure_index_running(e, process);
    listed_procedure(e, kind, index) == PROCEDURE_WANDER
}

// Typed from 00913460
/// `LowProcess::IsPackageDone` (engine map): `true` without a current package; otherwise whether
/// its type's procedure at the running procedure index is the done (0x36). The word unread.
pub fn is_package_done(e: &mut Engine, this: Ptr, _unused: u32) -> bool {
    let process = this.addr();
    let package = current_package(e, process);
    if package == 0 {
        return true;
    }
    let kind = package_kind(e, package);
    let index = procedure_index_running(e, process);
    listed_procedure(e, kind, index) == PROCEDURE_DONE
}

/// The procedures whose packages the combat controller runs: 0, 1, 6, 7, 0xd, 0xe and 0x11.
#[inline]
fn combat_procedure(procedure: i32) -> bool {
    matches!(procedure, 0 | 1 | 6 | 7 | 0xd | 0xe | 0x11)
}

// Typed from 009078d0
/// `LowProcess::ShouldUseCombatControllerForPackage` (Xbox PDB): for a combat controller's
/// procedure, whether the package's flags have 0x2000000 (or 0x4000000, with `flag`) and the
/// process's level is at most 1.
pub fn should_use_combat_controller_for_package(
    e: &mut Engine,
    this: Ptr,
    package: Ptr,
    flag: u8,
) -> bool {
    let package = package.addr();
    if !combat_procedure(procedure(e, package)) {
        return false;
    }
    let mask = if flag != 0 { 0x0600_0000 } else { 0x0200_0000 };
    if e.mem.u32(package.wrapping_add(0x1c)) & mask == 0 {
        return false;
    }
    e.mem.i32(this.addr().wrapping_add(0x28)) <= 1
}

// Typed from 00907960
/// `LowProcess::ShouldStopCombatControllerToFinishPackage` (engine map): `false` for a combat
/// controller's procedure; otherwise `true` unless the package has a type whose procedure at
/// `index` is the done (0x36).
pub fn should_stop_combat_controller_to_finish_package(
    e: &mut Engine,
    _this: Ptr,
    package: u32,
    index: u32,
) -> bool {
    if combat_procedure(procedure(e, package)) {
        return false;
    }
    let kind = package_kind(e, package);
    if kind == u32::MAX {
        return true;
    }
    listed_procedure(e, kind, index) != PROCEDURE_DONE
}

// Typed from 00907840
/// `LowProcess::CheckAndStartCombatControllerForPackage` (Xbox PDB): for a package the combat
/// controller should run ([`should_use_combat_controller_for_package`]), the package's target and
/// location are set (slots 0x8c and 0x90 with 0) and the process enters combat (slot 0x33c: the
/// actor, eleven zeros and the package); answers whether it did.
pub fn check_and_start_combat_controller_for_package(
    e: &mut Engine,
    this: Ptr,
    actor: Ptr,
    package: Ptr,
    flag: u8,
) -> bool {
    if package.addr() == 0
        || !typed_call!(
            FAMILY,
            0x0090_78d0,
            should_use_combat_controller_for_package(e, this, package, flag)
        )
    {
        return false;
    }
    let process = this.addr();
    let actor = actor.addr();
    set_target_for_package_slot(e, process, actor);
    set_location_for_package_slot(e, process, actor);
    if slot_target(e, process, 0x33c) == 0x0090_dc10 {
        typed_call!(
            FAMILY,
            0x0090_dc10,
            enter_combat(
                e,
                this,
                actor,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                package.addr()
            )
        );
    } else {
        let mut words = [0u32; 13];
        words[0] = actor;
        words[12] = package.addr();
        e.vcall(process, 0x33c, &words);
    }
    true
}

/// The current package's slot 0x13c with the actor, 0, -1.0 and 0.
fn package_slot_13c(e: &mut Engine, package: u32, actor: u32) -> bool {
    let minus_one = x87_quiet_bits(e.mem.u32(MINUS_ONE_FLOAT));
    e.vcall(package, 0x13c, &args![actor, 0u32, minus_one, 0u32])
        .bool()
}

// Typed from 00908e90
/// `LowProcess::ProcessUseItemAt` (Xbox PDB): with a current package whose slot 0x13c fails, the
/// running procedure moves back by one (slot 0x288 with -1); otherwise, when the current package's
/// flag 4 is set (`0067efd0`), on by one. Answers `false`.
pub fn process_use_item_at(e: &mut Engine, this: Ptr, actor: Ptr) -> bool {
    let process = this.addr();
    let actor = actor.addr();
    if current_package(e, process) != 0 {
        let package = current_package(e, process);
        if !package_slot_13c(e, package, actor) {
            add_to_procedure_index_running_slot(e, process, actor, u32::MAX);
            return false;
        }
    }
    let package = current_package(e, process);
    if e.mem.u32(package.wrapping_add(0x1c)) & 4 != 0 {
        add_to_procedure_index_running_slot(e, process, actor, 1);
    }
    false
}

/// Whether a script has something to run (its word at +0x30, or at +0x2c).
#[inline]
fn script_runs(e: &Engine, script: u32) -> bool {
    e.mem.u32(script.wrapping_add(0x30)) != 0 || e.mem.u32(script.wrapping_add(0x2c)) != 0
}

/// Runs a package's script on the actor (`Script::Run(actor, locals, 0, 1)` with the script locals
/// of the actor's extra data list).
fn run_package_script(e: &mut Engine, script: u32, actor: u32) {
    let locals = e
        .call(EXTRA_SCRIPT_LOCALS, &args![actor.wrapping_add(0x44)])
        .u32();
    e.call(SCRIPT_RUN, &args![script, actor, locals, 0u32, 1u32]);
}

/// Greets with a package's topic: `ProcessGreet(actor, topic, 0, 0, 1, 0)` (slot 0x2a4: the typed
/// [`process_greet`] when it holds `009136b0`).
fn greet_with_topic(e: &mut Engine, process: u32, actor: u32, topic: u32) {
    if slot_target(e, process, 0x2a4) == 0x0091_36b0 {
        typed_call!(
            FAMILY,
            0x0091_36b0,
            process_greet(
                e,
                Ptr::new(process),
                Ptr::new(actor),
                Ptr::new(topic),
                0,
                0,
                1,
                0
            )
        );
    } else {
        e.vcall(process, 0x2a4, &args![actor, topic, 0u32, 0u32, 1u32, 0u32]);
    }
}

// Typed from 009135a0
/// `LowProcess::ChangePackage` (Xbox PDB): with a package, its change script runs on the actor when
/// it has something to run, and its change topic is greeted with.
pub fn change_package(e: &mut Engine, this: Ptr, actor: Ptr, package: Ptr) {
    let package = package.addr();
    if package == 0 {
        return;
    }
    let actor = actor.addr();
    let script = e.mem.u32(package.wrapping_add(0x70));
    if script != 0 && script_runs(e, script) {
        run_package_script(e, script, actor);
    }
    let topic = e.mem.u32(package.wrapping_add(0x74));
    if topic != 0 {
        greet_with_topic(e, this.addr(), actor, topic);
    }
}

// Typed from 00913510
/// `LowProcess::StartNewPackage` (Xbox PDB): with a package, its start script runs on the actor and
/// its start topic is greeted with; then, with an actor, `00884f80` on it.
pub fn start_new_package(e: &mut Engine, this: Ptr, actor: Ptr, package: Ptr) {
    let package = package.addr();
    let actor = actor.addr();
    if package != 0 {
        let script = e.mem.u32(package.wrapping_add(0x50));
        if script != 0 {
            run_package_script(e, script, actor);
        }
        let topic = e.mem.u32(package.wrapping_add(0x54));
        if topic != 0 {
            greet_with_topic(e, this.addr(), actor, topic);
        }
    }
    if actor != 0 {
        e.call(ACTOR_AFTER_NEW_PACKAGE, &args![actor]);
    }
}

// Typed from 009122e0
/// `LowProcess::ProcessPatrolInit` (Xbox PDB): the patrol data is set up for the actor and the
/// current package (`PatrolActorPackageData::Init`); when that fails the running procedure index
/// becomes -1 (slot 0x284) and the answer is `false`, otherwise
/// `ProcessPatrolStartTravelToNextRef` runs and the answer is `true`.
pub fn process_patrol_init(e: &mut Engine, this: Ptr, actor: Ptr, data: Ptr) -> bool {
    let process = this.addr();
    let package = current_package(e, process);
    if !e
        .call(PATROL_DATA_INIT, &args![data, actor, package])
        .bool()
    {
        set_procedure_index_running(e, process, u32::MAX);
        return false;
    }
    typed_call!(
        FAMILY,
        0x0091_2340,
        process_patrol_start_travel_to_next_ref(e, this, actor, data)
    );
    true
}

/// Hands the patrol's current reference to the actor's extra data list as its package target.
fn set_patrol_extra_target(e: &mut Engine, data: u32, actor: u32) {
    let reference = e.call(PATROL_CURRENT_REFERENCE, &args![data]).u32();
    e.call(
        EXTRA_SET_PACKAGE_TARGET,
        &args![actor.wrapping_add(0x44), reference],
    );
}

// Typed from 009079f0
/// `LowProcess::UpdatePackageTargetForCombat` (engine map): only for a patrol package with package
/// data on the actor: unstarted data (its word at +4 0) is set up (`Init`, then +4 is 1), started
/// data moves to its next reference; on success the current reference becomes the actor's package
/// target and the answer is `true`.
pub fn update_package_target_for_combat(
    e: &mut Engine,
    _this: Ptr,
    actor: u32,
    package: u32,
) -> bool {
    if procedure(e, package) != PATROL {
        return false;
    }
    let data = typed_call!(
        dispatch::FAMILY,
        0x0042_12b0,
        dispatch::extra::actor_package_data(e, Ptr::new(actor.wrapping_add(0x44)))
    );
    if data == 0 {
        return false;
    }
    if e.mem.u32(data.wrapping_add(4)) == 0 {
        if !e
            .call(PATROL_DATA_INIT, &args![data, actor, package])
            .bool()
        {
            return false;
        }
        e.mem.set_u32(data.wrapping_add(4), 1);
    } else if !e.call(PATROL_SET_NEXT_REFERENCE, &args![data, 1u32]).bool() {
        return false;
    }
    set_patrol_extra_target(e, data, actor);
    true
}

/// Whether the door callbacks go on: an actor in the fight (its slot 0x22c with 0 false) and a
/// reference whose base form is a door.
fn door_and_actor(e: &mut Engine, door: u32, actor: u32) -> bool {
    if actor == 0 || e.vcall(actor, 0x22c, &args![0u32]).bool() {
        return false;
    }
    if door == 0 {
        return false;
    }
    let form = e.mem.u32(door.wrapping_add(0x20));
    e.mem.u8(form.wrapping_add(4)) == DOOR_FORM_TYPE
}

// Typed from 0090d570
/// The door unlock callback (`cdecl`, door and actor; answers `false`): an owner (`IsAnOwner`,
/// with whether the door has teleport data) unlocks a locked door and makes its teleport's cells
/// public.
pub fn unlock_door(e: &mut Engine, door: Ptr, actor: Ptr) -> bool {
    let (door, actor) = (door.addr(), actor.addr());
    if !door_and_actor(e, door, actor) {
        return false;
    }
    let teleport = e.call(TELEPORT_DATA, &args![door]).u32();
    if e.call(IS_AN_OWNER, &args![door, actor, u32::from(teleport != 0)])
        .bool()
        && e.call(GET_LOCK, &args![door]).u32() != 0
    {
        e.call(UNLOCK, &args![door]);
        if teleport != 0 {
            e.call(SET_CONNECTED_CELLS_PUBLIC, &args![teleport, door, 1u32]);
        }
    }
    false
}

// Typed from 0090d480
/// The door lock callback (`cdecl`, door and actor; answers `false`): the owner test's flag is set
/// for a door with teleport data that leads to a world space, or whose actor has no parent cell or
/// one that is not an interior; an owner locks a door that has a lock and makes its teleport's cells
/// private (the teleport data asked again).
pub fn lock_door(e: &mut Engine, door: Ptr, actor: Ptr) -> bool {
    let (door, actor) = (door.addr(), actor.addr());
    if !door_and_actor(e, door, actor) {
        return false;
    }
    let teleport = e.call(TELEPORT_DATA, &args![door]).u32();
    let flag = teleport != 0
        && (e.call(TELEPORT_WORLD_SPACE, &args![teleport]).u32() != 0 || {
            let cell = e.mem.u32(actor.wrapping_add(0x40));
            cell == 0 || e.mem.u8(cell.wrapping_add(0x24)) & 1 == 0
        });
    if e.call(IS_AN_OWNER, &args![door, actor, u32::from(flag)])
        .bool()
        && e.call(GET_LOCK, &args![door]).u32() != 0
    {
        e.call(LOCK, &args![door]);
        let teleport = e.call(TELEPORT_DATA, &args![door]).u32();
        if teleport != 0 {
            e.call(SET_CONNECTED_CELLS_PUBLIC, &args![teleport, door, 0u32]);
        }
    }
    false
}

// Typed from 00912150
/// `LowProcess::ProcessMovementBlocked` (engine map): the hours since the package started (the
/// calendar's hour less the embedded package's start hour, its float at +0x10: one difference
/// stored as a float), made positive (`00408840`); from the limit (the double at `0101ffa0`) on, the
/// running procedure moves on (slot 0x288 with 1) and the package is marked created; below it (or
/// unordered) the actor stops moving.
pub fn process_movement_blocked(e: &mut Engine, this: Ptr, actor: u32) {
    let process = this.addr();
    let start = loaded(e.mem.u32(process.wrapping_add(4 + 0x10)));
    let elapsed = to_float(f64::from(f32::from_bits(calendar_hour(e))) - start);
    let elapsed = typed_call!(
        pathing::FAMILY,
        0x0040_8840,
        pathing::point::abs(e, elapsed)
    );
    let limit = e.mem.f64(BLOCKED_TIME_LIMIT);
    if f64::from(elapsed) >= limit {
        add_to_procedure_index_running_slot(e, process, actor, 1);
        let package = typed_call!(
            leaf::FAMILY,
            0x0055_9450,
            leaf::word_at_0(e, process.wrapping_add(4))
        );
        e.call(PACKAGE_SET_IS_CREATED, &args![package, 1u32]);
    } else {
        e.call(ACTOR_STOP_MOVING, &args![actor]);
    }
}

// Typed from 00909d00
/// `LowProcess::ProcessFleeNonCombat` (Xbox PDB): for the running package (slot 0x27c), the target
/// is set when there is none (slot 0x8c) and the location (slot 0x90 with 0). With a target, the
/// distance to it is measured (`GetDistanceFromReference`) and nothing else happens; without one
/// the package's slot 0x13c runs. The exe compares that distance with the flee distance (the
/// package target's integer at +8 as a float) and returns early when the target is farther, but
/// both ways end there with a target (the slot 0x13c call needs none), so the flee distance is not
/// worked out.
pub fn process_flee_non_combat(e: &mut Engine, this: Ptr, actor: Ptr) {
    let process = this.addr();
    let actor = actor.addr();
    let package = running_package(e, process);
    if e.mem.u32(process.wrapping_add(TARGET)) == 0 {
        set_target_for_package_slot(e, process, actor);
    }
    set_location_for_package_slot(e, process, actor);
    let target = e.mem.u32(process.wrapping_add(TARGET));
    if target != 0 {
        e.call(REFERENCE_DISTANCE, &args![actor, target, 0u32, 0u32]);
    } else {
        package_slot_13c(e, package, actor);
    }
}

/// The embedded package's procedure index (its word at +0xc: the process's word at +0x10).
const PROCEDURE_INDEX: u32 = 4 + 0xc;
/// The embedded package's instance data (its word at +4: the process's word at +8, `008d0430`).
const INSTANCE_DATA: u32 = 4 + 4;
/// The hour last processed (a float; `007df1f0` reads it, `00693d50` stores it).
const HOUR_LAST_PROCESSED: u32 = 0x20;
/// `pItemBeingUsed`, the combat delay timer (two words; its float at +4 the one `00910b10`
/// clears), `iNumberItemsActivate`, `pAcquireObject`, `pSavedAcquireObject` and the death time.
const ITEM_BEING_USED: u32 = 0x34;
const COMBAT_DELAY_TIMER: u32 = 0x38;
const NUMBER_ITEMS_ACTIVATE: u32 = 0x58;
const OBJECT_LIST: u32 = 0x5c;
const ACQUIRE_OBJECT: u32 = 0x64;
const SAVED_ACQUIRE_OBJECT: u32 = 0x68;
const GENERIC_LOCATIONS_LIST: u32 = 0x6c;
const DEATH_TIME: u32 = 0xa8;
/// An actor's (a reference's) dialogue speaker (+0x70), extra data list (+0x44), parent cell
/// (+0x40, `008d6f30`), base form (+0x20, `007af430`) and start location (+0x24, `00430830`; its
/// float at +8 the angle).
const SPEAKER: u32 = 0x70;
const EXTRA_LIST: u32 = 0x44;
const PARENT_CELL: u32 = 0x40;
const BASE_FORM: u32 = 0x20;
const START_LOCATION: u32 = 0x24;
/// A topic's flag that it has dialogue to run (its flags word at +8, `00508d70`).
const TOPIC_HAS_DIALOGUE: u32 = 0x2000;
/// A package's done script and topic (+0x60 and +0x64: `00903e20`, `00903e00`).
const DONE_SCRIPT: u32 = 0x60;
const DONE_TOPIC: u32 = 0x64;
/// The patrol's base form type and form whose references give the travel an angle.
const ANGLED_FORM_TYPE: u8 = 0x30;
const ANGLED_FORM: u32 = 0x011c_a248;
/// The player character and `TES` (globals).
const PLAYER: u32 = 0x011d_ea3c;
const TES: u32 = 0x011d_ea10;
/// The doubles `0.0`, 3600.0 (seconds in an hour) and the patrol's near limit.
const ZERO_DOUBLE: u32 = 0x0101_2060;
const SECONDS_PER_HOUR: u32 = 0x0101_2640;
const PATROL_NEAR: u32 = 0x0101_6978;
/// `__RTDynamicCast`'s type descriptors of `MobileObject` and `Actor`.
const MOBILE_OBJECT_TYPE: u32 = 0x0118_4920;
const ACTOR_TYPE: u32 = 0x0118_46d4;
/// The pathing request a patrol's travel builds on the stack (its size as the translation keeps
/// it).
const PATHING_REQUEST_SIZE: u32 = 0xb4;

/// More callees called by address (not typed yet).
const CREATE_DIALOGUE_ITEM: u32 = 0x0061_b320;
const DIALOGUE_RUN_RESULT: u32 = 0x0083_c850;
const SET_SAY_TO_TOPIC: u32 = 0x0057_ad20;
const SET_SAY_TO_TOPIC_INFO: u32 = 0x0057_ace0;
const SAY_TO_CALLBACK: u32 = 0x0093_6a20;
const SET_ACTION_FLAG: u32 = 0x005a_c750;
const ACTOR_GET_PACKAGE: u32 = 0x0088_1450;
const ACTOR_CLEAR_LOOK_AT_TARGET: u32 = 0x008b_3d30;
const REFERENCE_WORLD_SPACE: u32 = 0x0057_5d70;
const SET_PACKAGE_START_LOCATION: u32 = 0x0056_ae10;
const COMBAT_UPDATE_PACKAGE_DATA: u32 = 0x0098_0ae0;
const DELETE_EMPTY_DESTRUCTOR: u32 = 0x007b_3fa0;
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
const LIST_CLEAR: u32 = 0x0047_0470;
const PATROL_TRAVEL_STEP: u32 = 0x0091_25b0;
const EXTRA_PATROL_WAIT: u32 = 0x0041_fe90;
const POINT_SUBTRACT: u32 = 0x0045_78c0;
const PLAYER_IS_SLEEPING_OR_RESTING: u32 = 0x0094_df60;
const PATROL_TRAVEL_TIME: u32 = 0x006e_25d0;
const PACKAGE_COUNT_FOR: u32 = 0x0067_6280;
const PATHING_REQUEST_CONSTRUCT: u32 = 0x006e_2420;
const PATHING_REQUEST_DESTRUCT: u32 = 0x006e_2620;
const ACTOR_BUILD_REQUEST: u32 = 0x008b_3880;
const ACTOR_SET_PATHFINDING_GOAL: u32 = 0x008b_3630;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const MODIFIER_LIST_COPY: u32 = 0x0093_7160;

/// A package target's reference (`PackageTarget::GetTargReference`, `00680020`): its word at +4
/// when its kind byte (+0, `00519b00`) is 0, else none.
#[inline]
fn target_reference(e: &Engine, target: u32) -> u32 {
    if e.mem.u8(target) == 0 {
        e.mem.u32(target.wrapping_add(4))
    } else {
        0
    }
}

/// The process's slot 0x23c (`GetCurrentProcedureIndex`): the embedded package's index in place
/// when the slot holds `008d03e0`.
fn current_procedure_index(e: &mut Engine, process: u32) -> u32 {
    if slot_target(e, process, 0x23c) == 0x008d_03e0 {
        e.mem.u32(process.wrapping_add(PROCEDURE_INDEX))
    } else {
        e.vcall(process, 0x23c, &[]).u32()
    }
}

/// The process's slot 0x238 (`SetCurrentProcedureIndex`): the store in place when the slot holds
/// `008d03c0`.
fn set_current_procedure_index(e: &mut Engine, process: u32, index: u32) {
    if slot_target(e, process, 0x238) == 0x008d_03c0 {
        e.mem.set_u32(process.wrapping_add(PROCEDURE_INDEX), index);
    } else {
        e.vcall(process, 0x238, &args![index]);
    }
}

/// The process's slot 0x240 (`AddToCurrentProcedureIndex`): the sum in place when the slot holds
/// `008d0400`.
fn add_to_current_procedure_index(e: &mut Engine, process: u32, delta: u32) {
    if slot_target(e, process, 0x240) == 0x008d_0400 {
        let at = process.wrapping_add(PROCEDURE_INDEX);
        let index = e.mem.u32(at).wrapping_add(delta);
        e.mem.set_u32(at, index);
    } else {
        e.vcall(process, 0x240, &args![delta]);
    }
}

/// The process's slot 0x288 ([`add_to_procedure_index_running`] on the low processes).
pub(super) fn add_to_procedure_index_running_slot(
    e: &mut Engine,
    process: u32,
    actor: u32,
    delta: u32,
) {
    if slot_target(e, process, 0x288) == 0x0091_19b0 {
        typed_call!(
            FAMILY,
            0x0091_19b0,
            add_to_procedure_index_running(e, Ptr::new(process), Ptr::new(actor), delta)
        );
    } else {
        e.vcall(process, 0x288, &args![actor, delta]);
    }
}

/// The process's slot 0x5a0 ([`package_done`] on all but `HighProcess`).
pub(super) fn package_done_slot(e: &mut Engine, process: u32, actor: u32, package: u32) {
    if slot_target(e, process, 0x5a0) == 0x0091_3630 {
        typed_call!(
            FAMILY,
            0x0091_3630,
            package_done(e, Ptr::new(process), Ptr::new(actor), Ptr::new(package))
        );
    } else {
        e.vcall(process, 0x5a0, &args![actor, package]);
    }
}

// Typed from 0090a070
/// `LowProcess::ResetTarget` (Xbox PDB): the current package's target's reference becomes the
/// target (slot 0x12c), unless there is no package, target or reference, the reference has the
/// form flag 0x20 (`00440d80`) or it is `pTarget` already. The exe asks for the current package
/// (slot 0x22c) before each step; so does this.
pub fn reset_target(e: &mut Engine, this: Ptr) {
    let process = this.addr();
    if current_package(e, process) == 0 {
        return;
    }
    let package = current_package(e, process);
    if e.mem.u32(package.wrapping_add(0x30)) == 0 {
        return;
    }
    let package = current_package(e, process);
    let target = e.mem.u32(package.wrapping_add(0x30));
    if target_reference(e, target) == 0 {
        return;
    }
    let package = current_package(e, process);
    let target = e.mem.u32(package.wrapping_add(0x30));
    let reference = target_reference(e, target);
    if e.mem.u32(reference.wrapping_add(8)) & 0x20 != 0
        || reference == e.mem.u32(process.wrapping_add(TARGET))
    {
        return;
    }
    set_target_slot(e, process, reference);
}

// Typed from 00913630
/// `LowProcess::PackageDone` (Xbox PDB): with a package, its done script runs on the actor and its
/// done topic is greeted with.
pub fn package_done(e: &mut Engine, this: Ptr, actor: Ptr, package: Ptr) {
    let package = package.addr();
    if package == 0 {
        return;
    }
    let actor = actor.addr();
    let script = e.mem.u32(package.wrapping_add(DONE_SCRIPT));
    if script != 0 {
        run_package_script(e, script, actor);
    }
    let topic = e.mem.u32(package.wrapping_add(DONE_TOPIC));
    if topic != 0 {
        greet_with_topic(e, this.addr(), actor, topic);
    }
}

// Typed from 009136b0
/// `LowProcess::ProcessGreet` (Xbox PDB; six words, the third to fifth unread): the actor's
/// speaker (+0x70) is taken and cleared (the exe reads it again when it was 0: 0 again); for a
/// topic with dialogue (its flag 0x2000) a dialogue item is made (`CreateDialogueItem(actor,
/// speaker, 0, 0, 1)`), set to its first response (`0083c7b0`: its iterator at +8 is the list at
/// +0) and, when there is a current response (the iterator's item), its result runs; with
/// `say_to`, the actor says the topic (`SetSayToTopic`, `SetSayToTopicInfo` with the item's info
/// at +0xc, `SayToCallBack` with the actor's word at +0xc and 1, the actor's slot 0x140). The
/// current package the exe asks for first (slot 0x22c) is unread, but asked as the exe asks.
#[allow(clippy::too_many_arguments)]
pub fn process_greet(
    e: &mut Engine,
    this: Ptr,
    actor: Ptr,
    topic: Ptr,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    say_to: u8,
) {
    let process = this.addr();
    current_package(e, process);
    let actor = actor.addr();
    let topic = topic.addr();
    let speaker_at = actor.wrapping_add(SPEAKER);
    let speaker = e.mem.u32(speaker_at);
    e.mem.set_u32(speaker_at, 0);
    if e.mem.u32(topic.wrapping_add(8)) & TOPIC_HAS_DIALOGUE == 0 {
        return;
    }
    let item = e
        .call(
            CREATE_DIALOGUE_ITEM,
            &args![topic, actor, speaker, 0u32, 0u32, 1u32],
        )
        .u32();
    if item == 0 {
        return;
    }
    e.mem.set_u32(item.wrapping_add(8), item);
    if e.mem.u32(item) == 0 {
        return;
    }
    e.call(DIALOGUE_RUN_RESULT, &args![item, 0u32]);
    if say_to == 0 {
        return;
    }
    e.call(SET_SAY_TO_TOPIC, &args![actor, topic]);
    let info = e.mem.u32(item.wrapping_add(0xc));
    e.call(SET_SAY_TO_TOPIC_INFO, &args![actor, info]);
    let object = e.mem.u32(actor.wrapping_add(0xc));
    e.call(SAY_TO_CALLBACK, &args![object, 1u32]);
    e.vcall(actor, 0x140, &[]);
}

// Typed from 009119b0
/// `LowProcess::AddToProcedureIndexRunning` (Xbox PDB): the current procedure index moves by
/// `delta` (slot 0x240), not below 0 (slot 0x238). With a current package it stays below the
/// count of its type's procedures (`Procedure::GetProcedureCount`); at the done procedure (0x36),
/// unless the package is done once (slot 0x5a4), the script action flag 0x400 is set for the
/// package on the actor's extra data (`Script::SetActionFlag`), `PackageDone` runs (slot 0x5a0)
/// and the package is marked done once (slot 0x5a8).
pub fn add_to_procedure_index_running(e: &mut Engine, this: Ptr, actor: Ptr, delta: u32) {
    let process = this.addr();
    let actor = actor.addr();
    let package = current_package(e, process);
    add_to_current_procedure_index(e, process, delta);
    if (current_procedure_index(e, process) as i32) < 0 {
        set_current_procedure_index(e, process, 0);
    }
    if package == 0 {
        return;
    }
    let count = procedure_count(e, package);
    if current_procedure_index(e, process) as i32 >= count {
        set_current_procedure_index(e, process, count.wrapping_sub(1) as u32);
    }
    let index = current_procedure_index(e, process);
    if listed_procedure(e, package_kind(e, package), index) != PROCEDURE_DONE
        || process_flag(e, process, 0x5a4, 0x008d_8000, 0x20)
    {
        return;
    }
    e.call(
        SET_ACTION_FLAG,
        &args![package, actor.wrapping_add(EXTRA_LIST), 0x400u32],
    );
    package_done_slot(e, process, actor, package);
    set_process_flag(e, process, 0x5a8, 0x008d_8020, 0x20, 1);
}

// Typed from 00907700
/// `LowProcess::SetCurrentPackageFromSchedule` (Xbox PDB): `false` when the actor's package
/// (`Actor::GetPackage`) is the current one already. Otherwise it becomes current (slot 0x230),
/// the new package is set up (slot 0x244), the actor stops looking at its target
/// (`ClearLookAtTarget`) and, for a current package without a location or with one of type 2,
/// the actor's package start location is set (`SetPackageStartLocation` with its world space,
/// parent cell, location (slot 0x1f4) and start angle); the actor's combat controller (slot
/// 0x428) takes the package and `CheckAndStartCombatControllerForPackage(actor, package, 0)`
/// runs (slot 0xb0). Answers `true`.
pub fn set_current_package_from_schedule(e: &mut Engine, this: Ptr, actor: Ptr) -> bool {
    let process = this.addr();
    let actor = actor.addr();
    let package = e.call(ACTOR_GET_PACKAGE, &args![actor]).u32();
    if package == current_package(e, process) {
        return false;
    }
    if slot_target(e, process, 0x230) == 0x008d_08b0 {
        typed_call!(
            FAMILY,
            0x008d_08b0,
            base_process::set_current_package(e, this, package, actor)
        );
    } else {
        e.vcall(process, 0x230, &args![package, actor]);
    }
    if slot_target(e, process, 0x244) != 0x0048_3710 {
        e.vcall(process, 0x244, &[]);
    }
    e.call(ACTOR_CLEAR_LOOK_AT_TARGET, &args![actor]);
    if current_package(e, process) != 0 {
        let current = current_package(e, process);
        let starts = e.mem.u32(current.wrapping_add(0x2c)) == 0 || {
            let current = current_package(e, process);
            let location = e.mem.u32(current.wrapping_add(0x2c));
            e.mem.i8(location) == 2
        };
        if starts {
            let angle = x87_quiet_bits(e.mem.u32(actor.wrapping_add(START_LOCATION + 8)));
            let location = e.vcall(actor, 0x1f4, &[]).u32();
            let cell = e.mem.u32(actor.wrapping_add(PARENT_CELL));
            let world_space = e.call(REFERENCE_WORLD_SPACE, &args![actor]).u32();
            e.call(
                SET_PACKAGE_START_LOCATION,
                &args![actor, world_space, cell, location, angle],
            );
        }
    }
    let controller = ai_queries::queries::actor_slot_428(e, actor);
    if controller != 0 {
        e.call(COMBAT_UPDATE_PACKAGE_DATA, &args![controller, package]);
    }
    if slot_target(e, process, 0xb0) == 0x0090_7840 {
        typed_call!(
            FAMILY,
            0x0090_7840,
            check_and_start_combat_controller_for_package(
                e,
                this,
                Ptr::new(actor),
                Ptr::new(package),
                0
            )
        );
    } else {
        e.vcall(process, 0xb0, &args![actor, package, 0u32]);
    }
    true
}

/// Deletes an object of the acquire lists (`007b3fa0` with 1), when there is one.
fn delete_object(e: &mut Engine, object: u32) {
    if object != 0 {
        e.call(DELETE_EMPTY_DESTRUCTOR, &args![object, 1u32]);
    }
}

// Typed from 00910b30
/// `LowProcess::ClearCurrentDataforProcess` (Xbox PDB; the word unread): the action complete
/// flag (slot 0x118), the flag 0x40 ([`set_flag_40`]) and the target activated flag (slot 0x4ec)
/// are cleared, the target (slot 0x12c with 0) and `pGenericLocation` too. The acquire object is
/// deleted and cleared; the saved one is taken off `ObjectList` (`00905330` with the field's
/// address), deleted and cleared. Every item left on `ObjectList` is deleted and taken off
/// (through a stack word holding it), and `GenericLocationsList` is emptied (`00470470`).
pub fn clear_current_data_for_process(e: &mut Engine, this: Ptr, _unused: u32) {
    let process = this.addr();
    set_process_flag(e, process, 0x118, 0x008d_7de0, 2, 0);
    typed_call!(FAMILY, 0x0090_ac20, set_flag_40(e, this, 0));
    set_process_flag(e, process, 0x4ec, 0x008d_7fe0, 1, 0);
    set_target_slot(e, process, 0);
    e.mem.set_u32(process.wrapping_add(GENERIC_LOCATION), 0);
    let acquire_at = process.wrapping_add(ACQUIRE_OBJECT);
    let acquire = e.mem.u32(acquire_at);
    if acquire != 0 {
        delete_object(e, acquire);
        e.mem.set_u32(acquire_at, 0);
    }
    let list = process.wrapping_add(OBJECT_LIST);
    let saved_at = process.wrapping_add(SAVED_ACQUIRE_OBJECT);
    if e.mem.u32(saved_at) != 0 {
        e.call(LIST_REMOVE_ITEM, &args![list, saved_at]);
        let saved = e.mem.u32(saved_at);
        delete_object(e, saved);
        e.mem.set_u32(saved_at, 0);
    }
    // `008256d0`: the list is empty when its words at +4 and +0 are both 0.
    while e.mem.u32(list.wrapping_add(4)) != 0 || e.mem.u32(list) != 0 {
        let item = e.mem.u32(list);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item);
            delete_object(e, item);
            e.call(LIST_REMOVE_ITEM, &args![list, slot]);
        });
    }
    e.call(
        LIST_CLEAR,
        &args![process.wrapping_add(GENERIC_LOCATIONS_LIST)],
    );
}

// Typed from 009121e0
/// `LowProcess::ProcessPatrol` (Xbox PDB): without running instance data (the embedded package's
/// word at +4) the running procedure index becomes -1 (slot 0x284); an actor whose slot 0x214
/// answers non-zero runs its slot 0x418 instead. Otherwise the time since the last update
/// (`GetTimeDelta`, stored as a float) is spent while it is above 0.0, by the patrol data's state
/// (+4): 0 `ProcessPatrolInit` (on failure the time becomes 0.0), 1 the travel step `009125b0`, 2
/// [`process_patrol_at_ref`] (each answers the time left), any other 0.0.
pub fn process_patrol(e: &mut Engine, this: Ptr, actor: Ptr) {
    let process = this.addr();
    let data = e.mem.u32(process.wrapping_add(INSTANCE_DATA));
    if data == 0 {
        set_procedure_index_running(e, process, u32::MAX);
        return;
    }
    let actor_word = actor.addr();
    if e.vcall(actor_word, 0x214, &[]).u32() != 0 {
        e.vcall(actor_word, 0x418, &[]);
        return;
    }
    let mut time = typed_call!(
        FAMILY,
        0x0090_7260,
        get_time_delta(e, Ptr::new(process), actor_word)
    );
    let zero = e.mem.f64(ZERO_DOUBLE);
    let data_ptr = Ptr::new(data);
    while f64::from(time) > zero {
        time = match e.mem.u32(data.wrapping_add(4)) {
            0 => {
                if typed_call!(
                    FAMILY,
                    0x0091_22e0,
                    process_patrol_init(e, this, actor, data_ptr)
                ) {
                    time
                } else {
                    0.0
                }
            }
            1 => to_float(
                e.call(
                    PATROL_TRAVEL_STEP,
                    &args![process, actor_word, data, time.to_bits()],
                )
                .f64(),
            ),
            2 => typed_call!(
                FAMILY,
                0x0091_2990,
                process_patrol_at_ref(e, this, actor, data_ptr, time)
            ),
            _ => 0.0,
        };
    }
}

// Typed from 00912990
/// `LowProcess::ProcessPatrolAtRef` (Xbox PDB): waiting at the patrol's current reference; without
/// one the running procedure index becomes -1 (slot 0x284) and the answer is 0.0. The wait is the
/// float of the reference's patrol extra (`0041fe90`, its float through `006a7f50`; 0.0 without
/// it) less the time waited already (the data's float at +8), at least 0.0 (`00404010`); the time
/// left is `time` less that (each difference one float operation, the exe's on either thread).
/// With time left the patrol moves to its next reference (`SetNextReferenceForPatrol(1)`; on
/// failure the running procedure moves on, slot 0x288 with 1, and the answer is 0.0): the time
/// left becomes 0.0 when that is the same reference or one nearer than the limit (the squared
/// length of the difference of their locations, slot 0x1f4, on a stack copy of the next one's),
/// and the travel to it starts ([`process_patrol_start_travel_to_next_ref`]). Without time left
/// the whole `time` is added to the time waited. Answers the time left.
pub fn process_patrol_at_ref(e: &mut Engine, this: Ptr, actor: Ptr, data: Ptr, time: f32) -> f32 {
    let process = this.addr();
    let data_word = data.addr();
    let reference = e.call(PATROL_CURRENT_REFERENCE, &args![data_word]).u32();
    if reference == 0 {
        set_procedure_index_running(e, process, u32::MAX);
        return 0.0;
    }
    let extra = reference.wrapping_add(EXTRA_LIST);
    let mut wait = 0.0;
    if e.call(EXTRA_PATROL_WAIT, &args![extra]).u32() != 0 {
        let object = e.call(EXTRA_PATROL_WAIT, &args![extra]).u32();
        wait = to_float(typed_call!(
            pathing::FAMILY,
            0x006a_7f50,
            pathing::search::first_float(e, Ptr::new(object))
        ));
    }
    let waited_at = data_word.wrapping_add(8);
    let remaining = to_float(f64::from(wait) - loaded(e.mem.u32(waited_at)));
    let wait = typed_call!(
        draw_f::FAMILY,
        0x0040_4010,
        draw_f::helpers::float_max(e, 0.0, remaining)
    );
    let mut left = to_float(loaded(time.to_bits()) - f64::from(wait));
    if f64::from(left) > e.mem.f64(ZERO_DOUBLE) {
        if !e
            .call(PATROL_SET_NEXT_REFERENCE, &args![data_word, 1u32])
            .bool()
        {
            add_to_procedure_index_running_slot(e, process, actor.addr(), 1);
            return 0.0;
        }
        if e.call(PATROL_CURRENT_REFERENCE, &args![data_word]).u32() == reference {
            left = 0.0;
        } else {
            let next = e.call(PATROL_CURRENT_REFERENCE, &args![data_word]).u32();
            let position = e.vcall(next, 0x1f4, &[]).u32();
            let near = e.with_stack(12, |e, point| {
                let point = point.addr();
                for offset in [0, 4, 8] {
                    let word = e.mem.u32(position.wrapping_add(offset));
                    e.mem.set_u32(point.wrapping_add(offset), word);
                }
                let other = e.vcall(reference, 0x1f4, &[]).u32();
                e.call(POINT_SUBTRACT, &args![point, other]);
                let squared = typed_call!(
                    pathing::FAMILY,
                    0x004a_7290,
                    pathing::point::length_squared(e, Ptr::new(point))
                );
                f64::from(squared) < e.mem.f64(PATROL_NEAR)
            });
            if near {
                left = 0.0;
            }
        }
        typed_call!(
            FAMILY,
            0x0091_2340,
            process_patrol_start_travel_to_next_ref(e, this, actor, data)
        );
    } else {
        let waited = to_float(loaded(e.mem.u32(waited_at)) + loaded(time.to_bits()));
        e.mem.set_u32(waited_at, waited.to_bits());
    }
    left
}

/// Whether neither cell (when set) is loaded (`TES::IsCellLoaded` with 0 for each).
fn cells_unloaded(e: &mut Engine, first: u32, second: u32) -> bool {
    for cell in [first, second] {
        if cell != 0 {
            let tes = e.mem.u32(TES);
            if typed_call!(
                cell_refs::FAMILY,
                0x0045_11e0,
                cell_refs::attach::is_cell_loaded(e, tes, cell, 0)
            ) {
                return false;
            }
        }
    }
    true
}

// Typed from 00912340
/// `LowProcess::ProcessPatrolStartTravelToNextRef` (Xbox PDB): without a current patrol reference
/// the running procedure index becomes -1 (slot 0x284). Otherwise the reference becomes the target
/// (slot 0x12c) and, unless the player sleeps or rests or (with the actor's slot 0x1d0 answering
/// 0) neither the actor's nor the reference's parent cell is loaded, the actor gets a pathing
/// request to it: a stack `PathingRequest` whose time is the larger of `006e25d0`'s and, with a
/// running package (slot 0x27c), the package's count for the actor (`00676280`, as a float);
/// `BuildRequest(request, location (slot 0x1f4), cell, world space, time, 0)`, its byte at +0x9c
/// cleared (`008e7c50`, in place), the reference's start angle and its flag (`008e7c20`, in place)
/// when its base form is of type 0x30 or the form at `011ca248`, `SetPathfindingGoal`, the request's
/// destructor. Then the patrol data's state (+4) is 1 (also when the travel is skipped). The exe's
/// exception frame only destroys the request on unwinding.
pub fn process_patrol_start_travel_to_next_ref(e: &mut Engine, this: Ptr, actor: Ptr, data: Ptr) {
    let process = this.addr();
    let actor = actor.addr();
    let data = data.addr();
    let reference = e.call(PATROL_CURRENT_REFERENCE, &args![data]).u32();
    if reference == 0 {
        set_procedure_index_running(e, process, u32::MAX);
        return;
    }
    set_target_slot(e, process, reference);
    let player = e.mem.u32(PLAYER);
    let skip = e.call(PLAYER_IS_SLEEPING_OR_RESTING, &args![player]).bool()
        || (e.vcall(actor, 0x1d0, &[]).u32() == 0 && {
            let actor_cell = e.mem.u32(actor.wrapping_add(PARENT_CELL));
            let reference_cell = e.mem.u32(reference.wrapping_add(PARENT_CELL));
            cells_unloaded(e, actor_cell, reference_cell)
        });
    if !skip {
        let mut time = to_float(e.call(PATROL_TRAVEL_TIME, &[]).f64());
        let package = running_package(e, process);
        if package != 0 {
            let count = e.call(PACKAGE_COUNT_FOR, &args![package, actor]).u32();
            let count = to_float(f64::from(count));
            time = typed_call!(
                draw_f::FAMILY,
                0x0040_4010,
                draw_f::helpers::float_max(e, time, count)
            );
        }
        e.with_stack(PATHING_REQUEST_SIZE, |e, request| {
            let request = request.addr();
            e.call(PATHING_REQUEST_CONSTRUCT, &args![request]);
            let world_space = e.call(REFERENCE_WORLD_SPACE, &args![reference]).u32();
            let cell = e.mem.u32(reference.wrapping_add(PARENT_CELL));
            let location = e.vcall(reference, 0x1f4, &[]).u32();
            e.call(
                ACTOR_BUILD_REQUEST,
                &args![
                    actor,
                    request,
                    location,
                    cell,
                    world_space,
                    time.to_bits(),
                    0u32
                ],
            );
            // `008e7c50` with 0: the request's byte at +0x9c.
            e.mem.set_u8(request.wrapping_add(0x9c), 0);
            let base_form = e.mem.u32(reference.wrapping_add(BASE_FORM));
            if e.mem.u8(base_form.wrapping_add(4)) == ANGLED_FORM_TYPE
                || base_form == e.mem.u32(ANGLED_FORM)
            {
                let at = reference.wrapping_add(START_LOCATION + 8);
                let angle = x87_quiet_bits(e.mem.u32(at));
                // `008e7c20`: the request's angle (+0x8c, through the x87 stack again) and
                // its flag (+0x99).
                e.mem.set_u32(request.wrapping_add(0x8c), angle);
                e.mem.set_u8(request.wrapping_add(0x99), 1);
            }
            e.call(ACTOR_SET_PATHFINDING_GOAL, &args![actor, request]);
            e.call(PATHING_REQUEST_DESTRUCT, &args![request]);
        });
    }
    e.mem.set_u32(data.wrapping_add(4), 1);
}

// Typed from 0090c360
/// `LowProcess::Update` of a time (Xbox PDB `Update_ov2`): for an actor that is one
/// (`__RTDynamicCast` from `MobileObject` to `Actor`), the missed packages are updated (slot 0x14
/// with the actor and 1) when `time` is above an hour's seconds (not when equal, below or NaN),
/// then the actor's slot 0x25c runs with 0.0.
pub fn update_buffer(e: &mut Engine, this: Ptr, actor: Ptr, time: f32) {
    let actor = actor.addr();
    if actor == 0 {
        return;
    }
    let cast = e
        .call(
            DYNAMIC_CAST,
            &args![actor, 0u32, MOBILE_OBJECT_TYPE, ACTOR_TYPE, 0u32],
        )
        .u32();
    if cast == 0 {
        return;
    }
    if f64::from(time) > e.mem.f64(SECONDS_PER_HOUR) {
        e.vcall(this.addr(), 0x14, &args![actor, 1u32]);
    }
    e.vcall(cast, 0x25c, &args![0u32]);
}

// Typed from 00907510
/// The `LowProcess` level of `BaseProcess::Copy` (`008d0660` calls it; no map name): `source`'s
/// low-process state moves into `this`: the hour last processed (a float load and store, which
/// quiets a signalling NaN), the embedded package (`ActorPackage::MoveTo`), the flags byte, the
/// acquire object (moved: the source's cleared), the combat delay timer's two words, the item
/// being used, the saved acquire object (moved when set), the date last processed, the target
/// (slot 0x12c), the items activated, the damage modifiers (`00937160`) and the three floats at
/// +0xa4, +0xa8 and +0xac (each quieted as the hour).
pub fn copy_level(e: &mut Engine, this: Ptr, source: Ptr) {
    let to = this.addr();
    let from = source.addr();
    let hour = x87_quiet_bits(e.mem.u32(from.wrapping_add(HOUR_LAST_PROCESSED)));
    e.mem.set_u32(to.wrapping_add(HOUR_LAST_PROCESSED), hour);
    typed_call!(
        FAMILY,
        0x008c_6130,
        actor_package::move_to(
            e,
            Ptr::new(from.wrapping_add(4)),
            Ptr::new(to.wrapping_add(4))
        )
    );
    let flags = e.mem.u8(from.wrapping_add(PROCESS_FLAGS));
    e.mem.set_u8(to.wrapping_add(PROCESS_FLAGS), flags);
    let acquire = e.mem.u32(from.wrapping_add(ACQUIRE_OBJECT));
    e.mem.set_u32(to.wrapping_add(ACQUIRE_OBJECT), acquire);
    e.mem.set_u32(from.wrapping_add(ACQUIRE_OBJECT), 0);
    for offset in [COMBAT_DELAY_TIMER, COMBAT_DELAY_TIMER + 4, ITEM_BEING_USED] {
        let word = e.mem.u32(from.wrapping_add(offset));
        e.mem.set_u32(to.wrapping_add(offset), word);
    }
    let saved = e.mem.u32(from.wrapping_add(SAVED_ACQUIRE_OBJECT));
    if saved != 0 {
        e.mem.set_u32(to.wrapping_add(SAVED_ACQUIRE_OBJECT), saved);
        e.mem.set_u32(from.wrapping_add(SAVED_ACQUIRE_OBJECT), 0);
    }
    let date = e.mem.u32(from.wrapping_add(DATE_LAST_PROCESSED));
    e.mem.set_u32(to.wrapping_add(DATE_LAST_PROCESSED), date);
    let target = e.mem.u32(from.wrapping_add(TARGET));
    set_target_slot(e, to, target);
    let activated = e.mem.u32(from.wrapping_add(NUMBER_ITEMS_ACTIVATE));
    e.mem
        .set_u32(to.wrapping_add(NUMBER_ITEMS_ACTIVATE), activated);
    e.call(
        MODIFIER_LIST_COPY,
        &args![
            to.wrapping_add(DAMAGE_MODIFIERS),
            from.wrapping_add(DAMAGE_MODIFIERS)
        ],
    );
    for offset in [ESSENTIAL_DOWN_TIMER, DEATH_TIME, TRACKED_DAMAGE] {
        let bits = x87_quiet_bits(e.mem.u32(from.wrapping_add(offset)));
        e.mem.set_u32(to.wrapping_add(offset), bits);
    }
}

// Typed from 0090e600
/// The constructor of a 0x1c-byte acquire record (`ObjectstoAcquire`; no map name): the words at
/// +0, +4, +0xc and +0x14 and the bytes at +8 and +9 cleared, +0x10 set to 0x7fffffff and +0x18 to
/// -1. Answers `this`.
pub fn objects_to_acquire_construct(e: &mut Engine, this: Ptr) -> Ptr {
    let at = this.addr();
    for offset in [0, 4, 0xc, 0x14] {
        e.mem.set_u32(at.wrapping_add(offset), 0);
    }
    e.mem.set_u8(at.wrapping_add(8), 0);
    e.mem.set_u8(at.wrapping_add(9), 0);
    e.mem.set_u32(at.wrapping_add(0x10), 0x7fff_ffff);
    e.mem.set_u32(at.wrapping_add(0x18), u32::MAX);
    this
}

// ---- Part 3: the acquire lists, the time delta, the flee, pursue, remove-worn and sandbox
// procedures, the destinations, the loads' resolves and the package's extra data ----

/// `TempObjectList` and `TempLocList`, and the `ObjectstoAcquire` record (`pObjectToAcquire`).
const TEMP_OBJECT_LIST: u32 = 0x74;
const TEMP_LOCATIONS_LIST: u32 = 0x7c;
const OBJECT_TO_ACQUIRE: u32 = 0x84;
/// The embedded package's running instance data (`008d0430`: the process's word at +8).
const RUNNING_INSTANCE_DATA: u32 = 8;
/// The package's flags (+0x1c): created (`00674d40`), and the one `00441b00` tests.
const PACKAGE_FLAGS: u32 = 0x1c;
const PACKAGE_CREATED: u32 = 0x800;
const PACKAGE_FLAG_200000: u32 = 0x0020_0000;
/// The package's type of an unequip-all package (`0041ca90`'s procedure 4) and of a pursue
/// package's own procedure (0x13).
const PROCEDURE_REMOVE_WORN_ALL: i32 = 4;
const PROCEDURE_PURSUE: i32 = 0x13;
/// `TimeScale`'s form id, the hours of a day (24.0, a double), the pursue distance (a double) and
/// the sandbox's least time (a float).
const TIME_SCALE_ID: u32 = 0x3a;
const HOURS_PER_DAY: u32 = 0x0103_56d8;
const PURSUE_DISTANCE: u32 = 0x0101_79e0;
const SANDBOX_LEAST_TIME: u32 = 0x0101_7b78;
/// The RTTI type descriptors of the casts.
const TYPE_FORM: u32 = 0x0118_3028;
const TYPE_BOUND_OBJECT: u32 = 0x0118_3108;
const TYPE_REFERENCE: u32 = 0x0118_41cc;
const TYPE_NPC: u32 = 0x0118_3a1c;
const TYPE_CREATURE: u32 = 0x0118_3a00;

const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
const ACTOR_END_INTERRUPT_PACKAGE: u32 = 0x0088_1680;
const ACTOR_IS_ALARMED: u32 = 0x008a_61b0;
const ACTOR_REMOVE_FLEE_TARGET: u32 = 0x008a_6730;
const NPC_INIT_DEFAULT_WORN: u32 = 0x0060_6540;
const CREATURE_INIT_DEFAULT_WORN: u32 = 0x005f_9e00;
const ACTOR_IS_PATHING_COMPLETE: u32 = 0x008b_3bb0;
const SANDBOX_START: u32 = 0x009f_3af0;
const SANDBOX_MOVE_TO_NEXT_LOCATION: u32 = 0x009f_3c10;
const REFERENCE_MOVE_TO_NEW_SPACE: u32 = 0x0057_3800;
const ACTOR_SET_PATHFINDING_GOAL_TO_REFERENCE: u32 = 0x008b_36f0;
const PACKAGE_LOCATION_WORLD_SPACE: u32 = 0x0067_5a50;
const PACKAGE_LOCATION_CELL: u32 = 0x0067_5c20;
const EXTRA_SET_ACTOR_PACKAGE_DATA: u32 = 0x0042_11d0;
const EXTRA_REMOVE_ACTOR_PACKAGE_DATA: u32 = 0x0042_12e0;
const SET_INSTANCE_DATA: u32 = 0x006e_cd40;

/// The acquire record at `field` (`pAcquireObject` or `pSavedAcquireObject`) deleted and cleared
/// when there is one and `matches` it.
fn clear_acquire_if(e: &mut Engine, field: u32, matches: impl FnOnce(&Engine, u32) -> bool) {
    let object = e.mem.u32(field);
    if object == 0 || !matches(e, object) {
        return;
    }
    delete_object(e, object);
    e.mem.set_u32(field, 0);
}

/// Every record on `ObjectList` that `matches` taken off (`00905330` with a stack word holding it)
/// and deleted (the word as the remove leaves it); the walk starts over after each.
fn clear_objects_if(e: &mut Engine, process: u32, matches: impl Fn(&Engine, u32) -> bool) {
    let list = process.wrapping_add(OBJECT_LIST);
    let mut node = list;
    while node != 0 {
        let item = e.mem.u32(node);
        if item == 0 {
            break;
        }
        if matches(e, item) {
            let removed = e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), item);
                e.call(LIST_REMOVE_ITEM, &args![list, slot]);
                e.mem.u32(slot.addr())
            });
            delete_object(e, removed);
            node = list;
        } else {
            node = e.mem.u32(node.wrapping_add(4));
        }
    }
}

/// Every location on `GenericLocationsList` that `matches` taken off: through the node before it
/// (`00905330` on that node with a stack word holding it, then on from that node's next), or the
/// head (`0063f7b0`, the walk staying on it).
fn clear_locations_if(e: &mut Engine, process: u32, matches: impl Fn(&Engine, u32) -> bool) {
    let mut node = process.wrapping_add(GENERIC_LOCATIONS_LIST);
    let mut previous = 0u32;
    while node != 0 {
        let location = e.mem.u32(node);
        if location == 0 {
            break;
        }
        if !matches(e, location) {
            previous = node;
            node = e.mem.u32(node.wrapping_add(4));
        } else if previous != 0 {
            e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), location);
                e.call(LIST_REMOVE_ITEM, &args![previous, slot]);
            });
            node = e.mem.u32(previous.wrapping_add(4));
        } else {
            e.call(LIST_REMOVE_HEAD, &args![node]);
        }
    }
}

/// Every record on the list at `list` taken off (`00905330` with a stack word holding the head's)
/// and deleted (the word as the remove leaves it), until the list is empty (`008256d0`: both its
/// words 0).
fn delete_list_records(e: &mut Engine, list: u32) {
    while e.mem.u32(list.wrapping_add(4)) != 0 || e.mem.u32(list) != 0 {
        let item = e.mem.u32(list);
        let removed = e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item);
            e.call(LIST_REMOVE_ITEM, &args![list, slot]);
            e.mem.u32(slot.addr())
        });
        delete_object(e, removed);
    }
}

// Typed from 009070a0
/// `LowProcess`'s release of its acquire data (no map name; the destructor and `Revert_ov2` call
/// it): the acquire record deleted and cleared (the saved one cleared first when it is the same),
/// then the saved one; `ObjectList` and `TempObjectList` emptied with their records deleted,
/// `GenericLocationsList` and `TempLocList` cleared (`00470470`), `pGenericLocation` cleared, and
/// the record at +0x84 deleted (its slot 0x10 with 1) and cleared.
pub fn release_acquire_data(e: &mut Engine, this: Ptr) {
    let process = this.addr();
    let acquire_at = process.wrapping_add(ACQUIRE_OBJECT);
    let saved_at = process.wrapping_add(SAVED_ACQUIRE_OBJECT);
    let acquire = e.mem.u32(acquire_at);
    if acquire != 0 {
        if e.mem.u32(saved_at) == acquire {
            e.mem.set_u32(saved_at, 0);
        }
        delete_object(e, acquire);
        e.mem.set_u32(acquire_at, 0);
    }
    let saved = e.mem.u32(saved_at);
    if saved != 0 {
        delete_object(e, saved);
        e.mem.set_u32(saved_at, 0);
    }
    delete_list_records(e, process.wrapping_add(OBJECT_LIST));
    e.call(
        LIST_CLEAR,
        &args![process.wrapping_add(GENERIC_LOCATIONS_LIST)],
    );
    e.mem.set_u32(process.wrapping_add(GENERIC_LOCATION), 0);
    delete_list_records(e, process.wrapping_add(TEMP_OBJECT_LIST));
    e.call(
        LIST_CLEAR,
        &args![process.wrapping_add(TEMP_LOCATIONS_LIST)],
    );
    let record_at = process.wrapping_add(OBJECT_TO_ACQUIRE);
    let record = e.mem.u32(record_at);
    if record != 0 {
        e.vcall(record, 0x10, &args![1u32]);
    }
    e.mem.set_u32(record_at, 0);
}

// Typed from 0090cc10
/// `LowProcess::ClearObjectFromAcquireList` (Xbox PDB): every hold of `reference` let go: the
/// acquire and saved acquire records whose reference (their word at +0) it is deleted and cleared,
/// its records taken off `ObjectList` and deleted, `pGenericLocation` cleared when it is that, and
/// it taken off `GenericLocationsList`.
pub fn clear_object_from_acquire_list(e: &mut Engine, this: Ptr, reference: u32) {
    let process = this.addr();
    let holds = |e: &Engine, record: u32| e.mem.u32(record) == reference;
    clear_acquire_if(e, process.wrapping_add(ACQUIRE_OBJECT), holds);
    clear_acquire_if(e, process.wrapping_add(SAVED_ACQUIRE_OBJECT), holds);
    clear_objects_if(e, process, holds);
    let location_at = process.wrapping_add(GENERIC_LOCATION);
    if e.mem.u32(location_at) == reference {
        e.mem.set_u32(location_at, 0);
    }
    clear_locations_if(e, process, |_, location| location == reference);
}

// Typed from 0090cdc0
/// `LowProcess::ClearObjectFromAcquireList` of a form (Xbox PDB `ClearObjectFromAcquireList_ov2`):
/// as [`clear_object_from_acquire_list`], for every reference whose form id (its word at +0xc,
/// `0084e3a0`) is `form`'s word.
pub fn clear_form_from_acquire_list(e: &mut Engine, this: Ptr, form: u32) {
    let process = this.addr();
    let is_form = move |e: &Engine, reference: u32| e.mem.u32(reference.wrapping_add(0xc)) == form;
    let holds = move |e: &Engine, record: u32| is_form(e, e.mem.u32(record));
    clear_acquire_if(e, process.wrapping_add(ACQUIRE_OBJECT), holds);
    clear_acquire_if(e, process.wrapping_add(SAVED_ACQUIRE_OBJECT), holds);
    clear_objects_if(e, process, holds);
    let location_at = process.wrapping_add(GENERIC_LOCATION);
    let location = e.mem.u32(location_at);
    if location != 0 && is_form(e, location) {
        e.mem.set_u32(location_at, 0);
    }
    clear_locations_if(e, process, is_form);
}

// Typed from 00907260
/// `LowProcess::GetTimeDelta` (Xbox PDB; the word unread): the game seconds since the hour last
/// processed (`fHourLastProcessed`, +0x20): the calendar's hour (`00867da0`, stored as a float),
/// a day (24.0) on when it is below that hour, the hour last processed taken off (stored as a
/// float), times an hour's seconds over the time scale (`TimeScale`, form 0x3a, through
/// `004839c0`; a global's value is its float at +0x24, `00526ac0`), stored as a float. The x87
/// steps (`FADD qword`, `FSUBR dword`, `FDIVR qword`, `FMUL dword`) round at the running
/// thread's precision ([`engine_core::x87`]). The hour last processed the exe reads twice is read
/// once (nothing writes it between).
pub fn get_time_delta(e: &mut Engine, this: Ptr, _unused: u32) -> f32 {
    let scale_form = typed_call!(
        forms::FAMILY,
        0x0048_39c0,
        forms::tesform::form_by_id(e, TIME_SCALE_ID)
    );
    let mut hour = calendar_hour(e);
    let last = loaded(e.mem.u32(this.addr().wrapping_add(HOUR_LAST_PROCESSED)));
    if x87_load32(hour) < last {
        hour = x87_store32(x87_add(x87_load32(hour), e.mem.f64(HOURS_PER_DAY)));
    }
    let delta = x87_store32(x87_sub(x87_m32(hour), last));
    let scale = loaded(e.mem.u32(scale_form.wrapping_add(0x24)));
    let per_hour = x87_div(e.mem.f64(SECONDS_PER_HOUR), scale);
    f32::from_bits(x87_store32(x87_mul(per_hour, x87_m32(delta))))
}

// Typed from 00909c90
/// `LowProcess::ProcessFlee` (Xbox PDB): when the current package's slot 0x13c (with the actor,
/// 0, -1.0 and 0) holds, the actor's interrupt package ends (`Actor::EndInterruptPackage(0)`).
/// Otherwise the exe asks [`get_time_delta`] and drops its answer; that reads only (the form
/// lookup `004839c0` is a map read), so it is not asked.
pub fn process_flee(e: &mut Engine, this: Ptr, actor: u32) {
    let package = current_package(e, this.addr());
    if package_slot_13c(e, package, actor) {
        e.call(ACTOR_END_INTERRUPT_PACKAGE, &args![actor, 0u32]);
    }
}

/// `MobileObject::GetProcessLevel` (`00931850`): the level of the reference's process (+0x68; its
/// word at +0x28), -1 without one.
fn process_level(e: &Engine, reference: u32) -> i32 {
    let process = e.mem.u32(reference.wrapping_add(0x68));
    if process == 0 {
        -1
    } else {
        e.mem.i32(process.wrapping_add(0x28))
    }
}

// Typed from 0090dc60
/// `LowProcess::ProcessPursue` (Xbox PDB; the third and fourth words unread): without a target
/// the package's (slot 0x8c); still none, the procedure moves on by one (slot 0x288). A target
/// whose process is low (level below 1) while the actor is not alarmed (`008a61b0`) moves it on by
/// two and is taken off the flee targets (`008a6730`). Farther than the pursue distance
/// (`GetDistanceFromReference`), the follow runs (slot 0x298 with the actor, 0, `word` and 1);
/// otherwise the procedure moves on by one unless the actor's slot 0x304 holds and the current
/// package's procedure is not the pursue (0x13).
pub fn process_pursue(
    e: &mut Engine,
    this: Ptr,
    actor: u32,
    _unused_2: u32,
    _unused_3: u32,
    word: u32,
) {
    let process = this.addr();
    let target_at = process.wrapping_add(TARGET);
    if e.mem.u32(target_at) == 0 {
        set_target_for_package_slot(e, process, actor);
    }
    let target = e.mem.u32(target_at);
    if target == 0 {
        add_to_procedure_index_running_slot(e, process, actor, 1);
        return;
    }
    if process_level(e, target) < 1 && !e.call(ACTOR_IS_ALARMED, &args![actor]).bool() {
        add_to_procedure_index_running_slot(e, process, actor, 2);
        e.call(ACTOR_REMOVE_FLEE_TARGET, &args![target, actor]);
    }
    let target = e.mem.u32(target_at);
    let distance = e
        .call(REFERENCE_DISTANCE, &args![actor, target, 0u32, 0u32])
        .f64();
    if distance > e.mem.f64(PURSUE_DISTANCE) {
        e.vcall(process, 0x298, &args![actor, 0u32, word, 1u32]);
        return;
    }
    if e.vcall(actor, 0x304, &[]).bool() {
        let package = current_package(e, process);
        if procedure(e, package) != PROCEDURE_PURSUE {
            return;
        }
    }
    add_to_procedure_index_running_slot(e, process, actor, 1);
}

// Typed from 00909320
/// `LowProcess::ProcessRemoveWorn` (Xbox PDB): the actor's base form (+0x20) as an NPC or a
/// creature (`__RTDynamicCast`, asked for each as the exe asks); an NPC's default outfit is worn
/// again (`TESNPC::InitDefaultWorn` with the actor's slot 0x1e8 and 1), a creature's
/// (`TESCreature::InitDefaultWorn` with two flags and 1): both clear for an unequip-all package
/// (procedure 4), the second clear for a package with the flag 0x200000 (`00441b00`). With
/// `advance`, the procedure moves on by one (slot 0x288).
pub fn process_remove_worn(e: &mut Engine, this: Ptr, actor: u32, advance: u8) {
    let process = this.addr();
    let base = e.mem.u32(actor.wrapping_add(BASE_FORM));
    let npc = base_process::runtime_cast(e, base, TYPE_BOUND_OBJECT, TYPE_NPC);
    let base = e.mem.u32(actor.wrapping_add(BASE_FORM));
    let creature = base_process::runtime_cast(e, base, TYPE_BOUND_OBJECT, TYPE_CREATURE);
    let package = current_package(e, process);
    let (first, second) = if package == 0 {
        (true, true)
    } else if procedure(e, package) == PROCEDURE_REMOVE_WORN_ALL {
        (false, false)
    } else {
        (
            true,
            e.mem.u32(package.wrapping_add(PACKAGE_FLAGS)) & PACKAGE_FLAG_200000 == 0,
        )
    };
    if npc != 0 {
        let worn = e.vcall(actor, 0x1e8, &[]).u32();
        e.call(NPC_INIT_DEFAULT_WORN, &args![npc, actor, worn, 1u32]);
    } else if creature != 0 {
        e.call(
            CREATURE_INIT_DEFAULT_WORN,
            &args![creature, actor, u32::from(first), u32::from(second), 1u32],
        );
    }
    if advance != 0 {
        add_to_procedure_index_running_slot(e, process, actor, 1);
    }
}

/// A process slot holding one of `LowProcess`'s flag tests (`body`, of `flag`), as the byte the exe
/// passes on: 0 or 1 in place (`008d7e60`), the slot's own byte otherwise.
fn process_flag_byte(e: &mut Engine, process: u32, slot: u32, body: u32, flag: u8) -> u8 {
    if slot_target(e, process, slot) == body {
        u8::from(e.mem.u8(process.wrapping_add(PROCESS_FLAGS)) & flag != 0)
    } else {
        e.vcall(process, slot, &[]).u8()
    }
}

// Typed from 00911f00
/// `LowProcess::ProcessSandbox` (Xbox PDB): with no running instance data the procedure moves on
/// by one (slot 0x288). The data's state (+4): 0 starts it (state 2, `009f3af0` with the running
/// package's location); 1 and 2 become 3; 3 goes on; any other ends. Going on, once the actor's
/// path is done (`008b3bb0`) and the data moves to its next location (`009f3c10`) that is another
/// one (its location the word at +0x10, `0044edb0`): when neither location's parent cell (+0x40)
/// is loaded, the actor is put at the next one (its slot 0x2a8 with the next's slot 0x1f4) and,
/// for another cell, moved to it (`MoveToNewSpace` with the next's world space); otherwise the
/// actor paths to it (`SetPathfindingGoalToReference`) for the package's count (`00676280`, as a
/// float) or the least time when that is larger (`00404010`).
pub fn process_sandbox(e: &mut Engine, this: Ptr, actor: Ptr) {
    let process = this.addr();
    let data = e.mem.u32(process.wrapping_add(RUNNING_INSTANCE_DATA));
    if data == 0 {
        add_to_procedure_index_running_slot(e, process, actor.addr(), 1);
        return;
    }
    let state_at = data.wrapping_add(4);
    match e.mem.u32(state_at) {
        0 => {
            let package = running_package(e, process);
            e.mem.set_u32(state_at, 2);
            let location = e.mem.u32(package.wrapping_add(0x2c));
            e.call(SANDBOX_START, &args![data, actor, location]);
            return;
        }
        1 | 2 => e.mem.set_u32(state_at, 3),
        3 => {}
        _ => return,
    }
    if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
        return;
    }
    let old = e.mem.u32(data.wrapping_add(0x10));
    if !e.call(SANDBOX_MOVE_TO_NEXT_LOCATION, &args![data]).bool() {
        return;
    }
    let next = e.mem.u32(data.wrapping_add(0x10));
    if next == 0 || next == old {
        return;
    }
    let old_cell = e.mem.u32(old.wrapping_add(PARENT_CELL));
    let next_cell = e.mem.u32(next.wrapping_add(PARENT_CELL));
    if cells_unloaded(e, old_cell, next_cell) {
        let position = e.vcall(next, 0x1f4, &[]).u32();
        e.vcall(actor.addr(), 0x2a8, &args![position]);
        if old_cell == 0 || old_cell != next_cell {
            let world_space = e.call(REFERENCE_WORLD_SPACE, &args![next]).u32();
            e.call(
                REFERENCE_MOVE_TO_NEW_SPACE,
                &args![actor, next_cell, world_space],
            );
        }
        return;
    }
    let package = running_package(e, process);
    let count = e.call(PACKAGE_COUNT_FOR, &args![package, actor]).u32();
    let time = x87_store32(f64::from(count));
    let least = x87_quiet_bits(e.mem.u32(SANDBOX_LEAST_TIME));
    let time = typed_call!(
        draw_f::FAMILY,
        0x0040_4010,
        draw_f::helpers::float_max(
            e,
            f32::from_bits(x87_quiet_bits(time)),
            f32::from_bits(least)
        )
    );
    e.call(
        ACTOR_SET_PATHFINDING_GOAL_TO_REFERENCE,
        &args![actor, next, x87_quiet_bits(time.to_bits()), 0u32],
    );
}

/// Where a procedure's destination is ([`current_destination_world_space`],
/// [`current_destination_cell`]).
enum Destination {
    /// The package's location (when it has one), else the actor's.
    PackageLocation,
    /// The target's, else the actor's.
    TargetOrActor,
    /// The actor's.
    Actor,
    /// The running instance data's reference's (when there is one).
    InstanceData,
    /// The package location when the package's word at +0x24 has no bit 4 (`00901c50`) and it has
    /// one, else the target's (slot 0x128), else the actor's.
    LocationOrTarget,
    /// The actor's (as it was first found).
    Keep,
}

fn destination(procedure: u32) -> Destination {
    match procedure {
        0 | 4 | 5 | 7 | 50 => Destination::PackageLocation,
        1 | 2 | 3 | 6 | 8 | 13 | 14 | 15 | 30 => Destination::TargetOrActor,
        27 | 54 => Destination::Actor,
        42 => Destination::InstanceData,
        47 => Destination::LocationOrTarget,
        _ => Destination::Keep,
    }
}

/// The process's slot 0x128 (`GetTarget`): `pTarget` in place when the slot holds `008d6f30`.
fn target_slot(e: &mut Engine, process: u32) -> u32 {
    if slot_target(e, process, 0x128) == 0x008d_6f30 {
        e.mem.u32(process.wrapping_add(TARGET))
    } else {
        e.vcall(process, 0x128, &[]).u32()
    }
}

/// The running procedure of `package`'s type: its list's procedure at the running index (slot
/// 0x280).
fn running_procedure(e: &mut Engine, process: u32, package: u32) -> u32 {
    let kind = package_kind(e, package);
    let index = procedure_index_running(e, process);
    listed_procedure(e, kind, index)
}

/// The running instance data's reference (`00639b40`: the data's word at +0x3c), none without
/// data.
fn instance_data_reference(e: &Engine, process: u32) -> u32 {
    let data = e.mem.u32(process.wrapping_add(RUNNING_INSTANCE_DATA));
    if data == 0 {
        0
    } else {
        e.mem.u32(data.wrapping_add(0x3c))
    }
}

/// Whether a package has a location that `LocationOrTarget` goes to: no bit 4 in its word at +0x24
/// (`00901c50`).
#[inline]
fn location_first(e: &Engine, package: u32) -> bool {
    e.mem.u16(package.wrapping_add(0x24)) & 4 == 0
}

/// A reference's world space (`TESObjectREFR::GetWorldSpace`).
fn world_space_of(e: &mut Engine, reference: u32) -> u32 {
    e.call(REFERENCE_WORLD_SPACE, &args![reference]).u32()
}

/// A reference's parent cell (+0x40, `008d6f30`).
#[inline]
fn cell_of(e: &Engine, reference: u32) -> u32 {
    e.mem.u32(reference.wrapping_add(PARENT_CELL))
}

// Typed from 0090f3f0
/// `LowProcess::GetCurrentDestinationWorldSpace` (Xbox PDB): the actor's world space (asked again
/// for a null actor where the exe asks), or, with a current package (slot 0x22c, asked once), the
/// world space of where its running procedure goes ([`Destination`]; the package location's
/// through `00675a50`; the target through slot 0x128, asked twice as the exe asks).
pub fn current_destination_world_space(e: &mut Engine, this: Ptr, actor: Ptr) -> u32 {
    let process = this.addr();
    let actor = actor.addr();
    let mut result = if actor != 0 {
        world_space_of(e, actor)
    } else {
        0
    };
    let package = current_package(e, process);
    if package == 0 {
        return result;
    }
    let location = |e: &mut Engine| {
        e.call(PACKAGE_LOCATION_WORLD_SPACE, &args![package, actor])
            .u32()
    };
    match destination(running_procedure(e, process, package)) {
        Destination::PackageLocation => {
            result = if e.mem.u32(package.wrapping_add(0x2c)) != 0 {
                location(e)
            } else {
                world_space_of(e, actor)
            };
        }
        Destination::TargetOrActor => {
            let target = e.mem.u32(process.wrapping_add(TARGET));
            result = world_space_of(e, if target != 0 { target } else { actor });
        }
        Destination::Actor => result = world_space_of(e, actor),
        Destination::InstanceData => {
            let reference = instance_data_reference(e, process);
            if reference != 0 {
                result = world_space_of(e, reference);
            }
        }
        Destination::LocationOrTarget => {
            result = if location_first(e, package) && e.mem.u32(package.wrapping_add(0x2c)) != 0 {
                location(e)
            } else if target_slot(e, process) != 0 {
                let target = target_slot(e, process);
                world_space_of(e, target)
            } else {
                world_space_of(e, actor)
            };
        }
        Destination::Keep => {}
    }
    result
}

// Typed from 0090f0e0
/// `LowProcess::GetCurrentDestinationCell` (Xbox PDB): as [`current_destination_world_space`], for
/// the parent cell (a reference's word at +0x40; the package location's through `00675c20`); this
/// one asks the current package (slot 0x22c) before each use, as the exe does.
pub fn current_destination_cell(e: &mut Engine, this: Ptr, actor: Ptr) -> u32 {
    let process = this.addr();
    let actor = actor.addr();
    let mut result = if actor != 0 { cell_of(e, actor) } else { 0 };
    if current_package(e, process) == 0 {
        return result;
    }
    let package = current_package(e, process);
    let location = |e: &mut Engine| {
        let package = current_package(e, process);
        e.call(PACKAGE_LOCATION_CELL, &args![package, actor]).u32()
    };
    match destination(running_procedure(e, process, package)) {
        Destination::PackageLocation => {
            let package = current_package(e, process);
            result = if e.mem.u32(package.wrapping_add(0x2c)) != 0 {
                location(e)
            } else {
                cell_of(e, actor)
            };
        }
        Destination::TargetOrActor => {
            let target = e.mem.u32(process.wrapping_add(TARGET));
            result = cell_of(e, if target != 0 { target } else { actor });
        }
        Destination::Actor => result = cell_of(e, actor),
        Destination::InstanceData => {
            let reference = instance_data_reference(e, process);
            if reference != 0 {
                result = cell_of(e, reference);
            }
        }
        Destination::LocationOrTarget => {
            let package = current_package(e, process);
            let located = location_first(e, package) && {
                let package = current_package(e, process);
                e.mem.u32(package.wrapping_add(0x2c)) != 0
            };
            result = if located {
                location(e)
            } else if target_slot(e, process) != 0 {
                let target = target_slot(e, process);
                cell_of(e, target)
            } else {
                cell_of(e, actor)
            };
        }
        Destination::Keep => {}
    }
    result
}

/// The form of a saved id as a `TESObjectREFR` (`004839c0`, `__RTDynamicCast`).
fn reference_of_id(e: &mut Engine, id: u32) -> u32 {
    let form = typed_call!(
        forms::FAMILY,
        0x0048_39c0,
        forms::tesform::form_by_id(e, id)
    );
    base_process::runtime_cast(e, form, TYPE_FORM, TYPE_REFERENCE)
}

// Typed from 00910250
/// `LowProcess::InitLoadGame` (Xbox PDB): `BaseProcess`'s, then the loaded target id becomes its
/// reference (slot 0x12c), marked targeted (`SetTargeted(1)`, in place: its flag 0x40000) when
/// set, and the loaded `pGenericLocation` id becomes its reference.
pub fn init_load_game(e: &mut Engine, this: Ptr, first: u32, second: u32, third: u32) {
    let process = this.addr();
    typed_call!(
        FAMILY,
        0x008d_0e60,
        base_process::init_load_game(e, this, first, second, third)
    );
    let target_at = process.wrapping_add(TARGET);
    let id = e.mem.u32(target_at);
    if id != 0 {
        let target = reference_of_id(e, id);
        set_target_slot(e, process, target);
        let target = e.mem.u32(target_at);
        if target != 0 {
            // `TESObjectREFR::SetTargeted(1)` (`00564db0`): its flag 0x40000 set.
            let at = target.wrapping_add(8);
            let flags = e.mem.u32(at);
            e.mem.set_u32(at, flags | TARGETED);
        }
    }
    let location_at = process.wrapping_add(GENERIC_LOCATION);
    let id = e.mem.u32(location_at);
    if id != 0 {
        let location = reference_of_id(e, id);
        e.mem.set_u32(location_at, location);
    }
}

/// The saved id at `at` replaced by its reference (none for 0).
fn resolve_reference_at(e: &mut Engine, at: u32) {
    let id = e.mem.u32(at);
    let reference = if id != 0 { reference_of_id(e, id) } else { 0 };
    e.mem.set_u32(at, reference);
}

// Typed from 00910820
/// `LowProcess::InitLoadGame` of a buffer (Xbox PDB `InitLoadGame_ov2`): `BaseProcess`'s; the
/// loaded target id's reference set (`pTarget` cleared first, then slot 0x12c); the ids at +0x44,
/// +0x48 and +0x54 replaced by their references; and each id on `GenericLocationsList` replaced by
/// its reference (`00726c60`, in place), or taken off when it has none (after the first
/// node through the node before it, `00905330`; the head through `0063f7b0`).
pub fn init_load_game_buffer(e: &mut Engine, this: Ptr, buffer: u32) {
    let process = this.addr();
    typed_call!(
        FAMILY,
        0x008d_1000,
        base_process::init_load_game_buffer(e, this, buffer)
    );
    let target_at = process.wrapping_add(TARGET);
    let id = e.mem.u32(target_at);
    let target = if id != 0 { reference_of_id(e, id) } else { 0 };
    e.mem.set_u32(target_at, 0);
    set_target_slot(e, process, target);
    for offset in [GENERIC_LOCATION, GENERIC_SECOND_LOCATION, 0x54] {
        resolve_reference_at(e, process.wrapping_add(offset));
    }
    let mut node = process.wrapping_add(GENERIC_LOCATIONS_LIST);
    let mut previous = 0u32;
    // `008256d0`: the walk ends at a node whose words at +4 and +0 are both 0.
    while node != 0 && (e.mem.u32(node.wrapping_add(4)) != 0 || e.mem.u32(node) != 0) {
        let id = e.mem.u32(node);
        let reference = if id != 0 { reference_of_id(e, id) } else { 0 };
        if reference != 0 {
            // `00726c60` through the exe's stack word holding the reference: the store in place.
            e.mem.set_u32(node, reference);
            previous = node;
            node = e.mem.u32(node.wrapping_add(4));
        } else if previous != 0 {
            e.call(LIST_REMOVE_ITEM, &args![previous, node]);
            node = e.mem.u32(previous.wrapping_add(4));
        } else {
            e.call(LIST_REMOVE_HEAD, &args![node]);
        }
    }
}

// Typed from 009130f0
/// `LowProcess::SavePackageToExtraData` (Xbox PDB): with a current package that is not a created
/// one (its flag 0x800), the actor's extra data (+0x44) keeps it (`ExtraPackage`'s set, `0041c930`)
/// with the current procedure index (slot 0x23c), the target and the action complete, target
/// activated and done once flags (slots 0x11c, 0x4e8, 0x5a4), and the actor's slot 0x48 runs with
/// 0x80000000. With running instance data, the extra data keeps that too (`004211d0`; the exe's
/// read of the actor's process's running package's word at +8 is unread) and the embedded
/// package's instance data is cleared (`ActorPackage::NullInstanceData`).
pub fn save_package_to_extra_data(e: &mut Engine, this: Ptr, actor: Ptr) {
    let process = this.addr();
    if current_package(e, process) == 0 {
        return;
    }
    let package = current_package(e, process);
    if e.mem.u32(package.wrapping_add(PACKAGE_FLAGS)) & PACKAGE_CREATED != 0 {
        return;
    }
    let extra = actor.addr().wrapping_add(EXTRA_LIST);
    let done_once = process_flag_byte(e, process, 0x5a4, 0x008d_8000, 0x20);
    let activated = process_flag_byte(e, process, 0x4e8, 0x008d_7fc0, 1);
    let complete = process_flag_byte(e, process, 0x11c, 0x008d_7e40, 2);
    let target = e.mem.u32(process.wrapping_add(TARGET));
    let index = current_procedure_index(e, process);
    let package = current_package(e, process);
    typed_call!(
        extra_data::FAMILY,
        0x0041_c930,
        extra_data::packages::set_package(
            e,
            extra,
            package,
            i32::from_ne_bytes(index.to_ne_bytes()),
            target,
            complete,
            activated,
            done_once
        )
    );
    e.vcall(actor.addr(), 0x48, &args![0x8000_0000u32]);
    let data = e.mem.u32(process.wrapping_add(RUNNING_INSTANCE_DATA));
    if data == 0 {
        return;
    }
    // The actor's process's running package (slot 0x278), asked again when there is one (the
    // process read again after the slot, as the exe does); its word at +8 the exe then reads is
    // unread.
    let process_at = actor.addr().wrapping_add(0x68);
    if e.mem.u32(process_at) != 0 {
        let other = e.mem.u32(process_at);
        if running_actor_package(e, other) != 0 {
            let other = e.mem.u32(process_at);
            running_actor_package(e, other);
        }
    }
    e.call(EXTRA_SET_ACTOR_PACKAGE_DATA, &args![extra, data]);
    typed_call!(
        FAMILY,
        0x008c_6060,
        actor_package::null_instance_data(e, Ptr::new(process.wrapping_add(4)))
    );
}

// Typed from 00913250
/// `LowProcess::LoadPackageFromExtraData` (Xbox PDB): with a package kept in the actor's extra data
/// (`ExtraPackage`): the current data cleared when the actor's slot 0x100 holds (slot 0x524), the
/// embedded package started from it with its target (`ActorPackage::InitFromPackage`), the actor
/// told (`004534f0`, the empty body: left out), the kept procedure index set (slot 0x238; 0 when
/// it is past the package's procedure count), the target (slot 0x12c) and the three flags (slots
/// 0x118, 0x5a8, 0x4ec) set, the kept package taken off, and kept instance data handed to the
/// embedded package (`006ecd40`) and taken off. Without one: the current data cleared as above,
/// the current package (slot 0x234), the target and the flags cleared.
pub fn load_package_from_extra_data(e: &mut Engine, this: Ptr, actor: Ptr) {
    let process = this.addr();
    let extra = actor.addr().wrapping_add(EXTRA_LIST);
    let kept = typed_call!(
        extra_data::FAMILY,
        0x0041_cb10,
        extra_data::packages::package(e, extra)
    );
    if e.vcall(actor.addr(), 0x100, &[]).bool() {
        clear_current_data_slot(e, process, actor.addr());
    }
    if kept == 0 {
        clear_current_package_slot(e, process);
        set_target_slot(e, process, 0);
        set_process_flag(e, process, 0x118, 0x008d_7de0, 2, 0);
        typed_call!(FAMILY, 0x0090_ac20, set_flag_40(e, this, 0));
        set_process_flag(e, process, 0x5a8, 0x008d_8020, 0x20, 0);
        set_process_flag(e, process, 0x4ec, 0x008d_7fe0, 1, 0);
        return;
    }
    let target = typed_call!(
        dispatch::FAMILY,
        0x0041_cb70,
        dispatch::extra::package_target(e, Ptr::new(extra))
    );
    let package = typed_call!(
        extra_data::FAMILY,
        0x0041_cb10,
        extra_data::packages::package(e, extra)
    );
    typed_call!(
        FAMILY,
        0x008c_6080,
        actor_package::init_from_package(
            e,
            Ptr::new(process.wrapping_add(4)),
            Ptr::new(package),
            target
        )
    );
    current_package(e, process);
    let mut index = typed_call!(
        extra_data::FAMILY,
        0x0041_cb40,
        extra_data::packages::package_index(e, extra)
    );
    let package = e.mem.u32(process.wrapping_add(4));
    if package_kind(e, package) != u32::MAX && index >= procedure_count(e, package) {
        index = 0;
    }
    set_current_procedure_index(e, process, u32::from_ne_bytes(index.to_ne_bytes()));
    let target = typed_call!(
        dispatch::FAMILY,
        0x0041_cb70,
        dispatch::extra::package_target(e, Ptr::new(extra))
    );
    set_target_slot(e, process, target);
    let complete = typed_call!(
        extra_data::FAMILY,
        0x0041_cba0,
        extra_data::packages::package_action_complete(e, extra)
    );
    set_process_flag(e, process, 0x118, 0x008d_7de0, 2, complete);
    let done_once = typed_call!(
        extra_data::FAMILY,
        0x0041_cc30,
        extra_data::packages::package_done_once(e, extra)
    );
    set_process_flag(e, process, 0x5a8, 0x008d_8020, 0x20, done_once);
    let activated = typed_call!(
        extra_data::FAMILY,
        0x0041_cc00,
        extra_data::packages::package_activated(e, extra)
    );
    set_process_flag(e, process, 0x4ec, 0x008d_7fe0, 1, activated);
    typed_call!(
        extra_data::FAMILY,
        0x0041_cc60,
        extra_data::packages::remove_package(e, extra)
    );
    let data = typed_call!(
        dispatch::FAMILY,
        0x0042_12b0,
        dispatch::extra::actor_package_data(e, Ptr::new(extra))
    );
    if data != 0 {
        e.call(SET_INSTANCE_DATA, &args![process.wrapping_add(4), data]);
        e.call(EXTRA_REMOVE_ACTOR_PACKAGE_DATA, &args![extra]);
    }
}

/// The process's slot 0x524 ([`clear_current_data_for_process`] on the low processes).
fn clear_current_data_slot(e: &mut Engine, process: u32, actor: u32) {
    if slot_target(e, process, 0x524) == 0x0091_0b30 {
        typed_call!(
            FAMILY,
            0x0091_0b30,
            clear_current_data_for_process(e, Ptr::new(process), actor)
        );
    } else {
        e.vcall(process, 0x524, &args![actor]);
    }
}

/// The process's slot 0x234 (`BaseProcess::ClearCurrentPackage` on every process class).
fn clear_current_package_slot(e: &mut Engine, process: u32) {
    if slot_target(e, process, 0x234) == 0x008d_03a0 {
        typed_call!(
            FAMILY,
            0x008d_03a0,
            base_process::clear_current_package(e, Ptr::new(process))
        );
    } else {
        e.vcall(process, 0x234, &[]);
    }
}
