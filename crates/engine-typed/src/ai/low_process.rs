//! `LowProcess` (the Xbox PDB's source unit of that name, in `fallout/ai`), part 1: the process
//! lock map (`NiTMap<LowProcess*, LP_LOCK_DATA>` and its base), the processed date's parts, the
//! flags and timers, the target and location setters, the damage modifiers, the package tests and
//! starts, the door lock and unlock callbacks and the small procedure steps, each as its translation
//! in `engine-fallout-ai` does it.
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

use super::middle_low_process::{
    calendar_hour, current_package, loaded, package_kind, procedure, procedure_index_running,
    set_procedure_index_running, value_has_flag,
};
use super::FAMILY;
use crate::native::to_float;
use crate::{ai_queries, crt, dispatch, pathing, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet_bits;
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
const PROCESS_PATROL_START_TRAVEL: u32 = 0x0091_2340;
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
    e.vcall(this.addr(), 0x288, &args![actor, 1u32]);
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
            e.vcall(process, 0x288, &args![actor, u32::MAX]);
            return false;
        }
    }
    let package = current_package(e, process);
    if e.mem.u32(package.wrapping_add(0x1c)) & 4 != 0 {
        e.vcall(process, 0x288, &args![actor, 1u32]);
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

/// Greets with a package's topic: `ProcessGreet(actor, topic, 0, 0, 1, 0)` (slot 0x2a4).
fn greet_with_topic(e: &mut Engine, process: u32, actor: u32, topic: u32) {
    e.vcall(process, 0x2a4, &args![actor, topic, 0u32, 0u32, 1u32, 0u32]);
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
    e.call(PROCESS_PATROL_START_TRAVEL, &args![this, actor, data]);
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
        e.vcall(process, 0x288, &args![actor, 1u32]);
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
