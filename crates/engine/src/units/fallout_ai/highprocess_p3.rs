//! `fallout/ai/highprocess.cpp` (Xbox PDB source unit), part 3: its functions from `008d9780` up to
//! (not including) `008f3fe0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::highprocess`]; anything public there may be used here.
//!
//! Notes for this part (first session, `008d9780` to `008dd880`):
//! - Virtual slots called on the process (`this`) carry the Xbox PDB names
//!   of `HighProcess`' vtable (they agree with the PC vtable); slots called
//!   on an `Actor` are given by their byte offset only, because the PC
//!   `TESObjectREFR` vtable is longer than the Xbox one and the PDB names do
//!   not line up.
//! - Offsets of `Actor` fields are the PC ones (the Xbox PDB's minus `0x10`).
//! - Fields of the process classes below `HighProcess` that the shared
//!   layout does not declare are read at their offset, with the PDB name in
//!   a comment.

// The comparisons keep the x87 behavior for NaN (a negated "greater than" is
// not "at most"), and the nested conditions follow the original branches.
#![allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::collapsible_if,
    clippy::needless_late_init
)]

#[allow(unused_imports)]
use super::highprocess::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// The global holding the `PlayerCharacter` pointer.
const PLAYER_POINTER: u32 = 0x011d_ea3c;
/// `operator delete` (`00401030`, cdecl, one argument).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `HighProcess`' vtable (`01087864`).
const HIGH_PROCESS_VTABLE_ADDRESS: u32 = 0x0108_7864;
/// `Actor`'s process (`008d8520`): the word the actor keeps for it
/// (`BaseProcess*`); the code uses it to recognise the player's process.
const ACTOR_PROCESS: u32 = 0x008d_8520;
/// `BSSimpleList` node item address (`006815c0`): returns `this`, the node,
/// whose first word is the item.
const NODE_ITEM_ADDRESS: u32 = 0x0068_15c0;
/// `BSSimpleList` next node (`00726070`).
const NODE_NEXT: u32 = 0x0072_6070;
/// `BSSimpleList::RemoveAll` (`00470470`).
const LIST_CLEAR: u32 = 0x0047_0470;
/// `BSSimpleList` scalar deleting destructor (`004702f0`, argument: delete flag).
const LIST_DELETE: u32 = 0x0047_02f0;
/// `BSSimpleList` destructor (`0046ffb0`).
const LIST_DESTRUCTOR: u32 = 0x0046_ffb0;
/// By-value removal from a `BSSimpleList` (`00905330`): the argument is the
/// address of a word holding the item.
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
/// Adds the item whose address is passed to a `BSSimpleList` (`00905820`).
const LIST_ADD_ITEM: u32 = 0x0090_5820;
/// `NiPointer` getter (`00559450`): the first word of `this`.
const NI_POINTER_GET: u32 = 0x0055_9450;
/// `NiPointer` assignment (`0066b0d0`).
const NI_POINTER_SET: u32 = 0x0066_b0d0;
/// `NiPointer` destructor (`0045cec0`).
const NI_POINTER_DESTRUCTOR: u32 = 0x0045_cec0;
/// `NiPointer<KFModel>` assignment (`0044b070`).
const KF_MODEL_POINTER_SET: u32 = 0x0044_b070;
/// `NiPointer<KFModel>` destructor (`0044b030`).
const KF_MODEL_POINTER_DESTRUCTOR: u32 = 0x0044_b030;
/// `BSSoundHandle` destructor (`00483710`).
const SOUND_HANDLE_DESTRUCTOR: u32 = 0x0048_3710;
/// `__ehvec_dtor` (`00ec5fce`): array pointer, element size, count, destructor.
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;
/// `_ftol2_sse` (`00ec62c0`): truncation of the `f64` in the first two words.
const FLOAT_TO_INT: u32 = 0x00ec_62c0;
/// `__RTDynamicCast` (`00ec43fb`).
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// The frame-time timer object whose `0084d030` returns the elapsed time.
const FRAME_TIMER: u32 = 0x011f_6394;
/// `0084d030`: elapsed time of the timer in `this` (an `f32` in `ST0`).
const FRAME_TIME: u32 = 0x0084_d030;
/// A game setting's value address (`00403e20`): `this` is the setting.
const SETTING_VALUE: u32 = 0x0040_3e20;
/// Package type word (`0041ca90`).
const PACKAGE_TYPE: u32 = 0x0041_ca90;
/// `Actor::GetCurrentPackage` (`009344a0`).
const ACTOR_CURRENT_PACKAGE: u32 = 0x0093_44a0;
/// `Actor::GetMoveMode` (`008846e0`).
const ACTOR_MOVE_MODE: u32 = 0x0088_46e0;
/// `Actor::SetMoveMode` (`008b39f0`).
const ACTOR_SET_MOVE_MODE: u32 = 0x008b_39f0;
/// `Actor::IsPathingComplete` (`008b3bb0`).
const ACTOR_IS_PATHING_COMPLETE: u32 = 0x008b_3bb0;
/// `Actor::EndInterruptPackage` (`00881680`).
const ACTOR_END_INTERRUPT_PACKAGE: u32 = 0x0088_1680;
/// `TESObjectREFR::GetDistanceFromReference` (`005723b0`): `ST0` result.
const DISTANCE_FROM_REFERENCE: u32 = 0x0057_23b0;
/// `TESTopic::GetTopic` (`0061a2d0`, cdecl).
const GET_TOPIC: u32 = 0x0061_a2d0;
/// Base-class `SetupNewPackage` (`MiddleHighProcess::SetupNewPackage`, `00915400`).
const MIDDLE_HIGH_SETUP_NEW_PACKAGE: u32 = 0x0091_5400;
/// Base-class destructor (`00914610`).
const BASE_DESTRUCTOR: u32 = 0x0091_4610;

// Virtual slots of `HighProcess` (Xbox PDB names).
const SLOT_SET_EXTRA_HEAD_TRACK: u32 = 0x628;
const SLOT_GET_PACKAGE_THAT_IS_RUNNING: u32 = 0x27c;
const SLOT_GET_INSTANCE_DATA_THAT_IS_RUNNING: u32 = 0x274;
const SLOT_GET_RUN_ONCE_PACKAGE: u32 = 0x20c;
const SLOT_CLEAR_RUN_ONCE_PACKAGE: u32 = 0x214;
const SLOT_GET_CURRENT_PACKAGE: u32 = 0x22c;
const SLOT_GET_CURRENT_PROCEDURE_INDEX: u32 = 0x23c;
const SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING: u32 = 0x288;
const SLOT_END_MOVE_MESSAGE: u32 = 0x294;
const SLOT_PROCESS_GREET: u32 = 0x2a4;
const SLOT_SET_CURRENT_MOVEMENT_COMPLETE: u32 = 0x120;
const SLOT_SET_TARGET: u32 = 0x12c;
const SLOT_GET_TARGET: u32 = 0x128;
const SLOT_CREATE_FOLLOW_FOR_ESCORT: u32 = 0xc8;
const SLOT_SET_TARGET_FOR_PACKAGE: u32 = 0x8c;
const SLOT_SETUP_SPECIAL_IDLE: u32 = 0x44;
const SLOT_GET_IDLE_TIMER: u32 = 0x334;
const SLOT_SET_IDLE_TIMER: u32 = 0x338;
const SLOT_GET_ANIM_ACTION: u32 = 0x3e4;
const SLOT_GET_ANIMATION: u32 = 0x1b8;
const SLOT_GET_MOVEMENT_STOPPED: u32 = 0x49c;
const SLOT_SET_ACTORS_ANIMATION: u32 = 0x350;
const SLOT_SET_END_IDLES_PLAYED: u32 = 0x590;
const SLOT_GET_END_IDLES_PLAYED: u32 = 0x594;
const SLOT_ADD_POST_ANIMATION_ACTION: u32 = 0x614;
const SLOT_FINISH_SETUP_SPECIAL_IDLE: u32 = 0x70c;
const SLOT_IS_DOING_SAY_TO: u32 = 0x78;
const SLOT_GET_WEAPON_DRAWN: u32 = 0x454;
const SLOT_GET_PLANTED_EXPLOSIVE: u32 = 0x388;
const SLOT_SET_FORCE_ROTATE: u32 = 0x684;
const SLOT_SET_GREETING_FLAG: u32 = 0x310;
const SLOT_ATTACK_CALLBACK: u32 = 0x448;
const SLOT_SET_LAST_GREETED: u32 = 0x488;
const SLOT_SET_DOING_SAY_TO: u32 = 0x88;
// Virtual slots of `Actor` (PC byte offsets; the meaning is described where
// the slot is used).
/// Returns the actor's 3D node (a null result stops the idle code).
const ACTOR_SLOT_NODE: u32 = 0x1d0;
/// Returns the actor's `Animation` (a null result: no animation yet).
const ACTOR_SLOT_ANIMATION: u32 = 0x1e4;
/// Returns a small state number; the idle and package code accept 0, 4 and 9.
const ACTOR_SLOT_STATE: u32 = 0x214;
const ACTOR_SLOT_IS_ACTOR: u32 = 0x100;
const ACTOR_SLOT_IS_MOBILE_OBJECT: u32 = 0xfc;
const ACTOR_SLOT_POSITION: u32 = 0x1f4;
const ACTOR_SLOT_PAUSED_A: u32 = 0x230;
const ACTOR_SLOT_PAUSED_B: u32 = 0x234;
const ACTOR_SLOT_IS_CREATURE: u32 = 0x218;
const ACTOR_SLOT_IS_EXPLOSION: u32 = 0x21c;
const ACTOR_SLOT_COMBAT_CONTROLLER_ATTACK: u32 = 0x424;

/// `Actor` state accepted by the idle and package code: the value of
/// [`ACTOR_SLOT_STATE`] is 0, 4 or 9.
fn actor_state(e: &mut Engine, actor: u32) -> u32 {
    e.vcall(actor, ACTOR_SLOT_STATE, &args![]).u32()
}

/// The three-way test the idle code repeats: the actor's state is 0, 4 or 9.
/// The code calls the slot up to three times (once per comparison).
fn actor_state_is_0_4_or_9(e: &mut Engine, actor: u32) -> bool {
    actor_state(e, actor) == 0 || actor_state(e, actor) == 4 || actor_state(e, actor) == 9
}

fn player_pointer(e: &Engine) -> u32 {
    e.global::<u32>(PLAYER_POINTER)
}

// Translated from 008d9780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::SetAvoidWaitTimer` (Xbox PDB) stores `fAvoidWaitTimer` (+0x450).
pub fn high_process_set_avoid_wait_timer(e: &mut Engine, this: Ptr<HighProcess>, value: f32) {
    e.set(this, HighProcess::fAvoidWaitTimer, value);
}

// Translated from 008d97a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetRadiationDelta` (Xbox PDB): the larger of
/// `fHighestRadiation` (+0x440) and `fRadiationDelta` (+0x234); on equal or
/// unordered values the `fRadiationDelta` one.
pub fn high_process_get_radiation_delta(e: &mut Engine, this: Ptr<HighProcess>) -> f32 {
    let highest = e.get(this, HighProcess::fHighestRadiation);
    // MiddleHighProcess::fRadiationDelta (Xbox PDB) +0x234
    let delta = e.mem.f32(this.addr() + 0x234);
    if highest > delta {
        highest
    } else {
        delta
    }
}

// Translated from 008d97f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::SetForceRotate` (Xbox PDB) stores `bForceRotate` (+0x420).
pub fn high_process_set_force_rotate(e: &mut Engine, this: Ptr<HighProcess>, value: u8) {
    e.set(this, HighProcess::bForceRotate, value);
}

// Translated from 008d9810 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetForceRotate` (Xbox PDB) returns `bForceRotate` (+0x420).
pub fn high_process_get_force_rotate(e: &mut Engine, this: Ptr<HighProcess>) -> u8 {
    e.get(this, HighProcess::bForceRotate)
}

// Translated from 008d9830 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetLastHeadTrackTarget` (Xbox PDB) returns `pLastTarget` (+0x41c).
pub fn high_process_get_last_head_track_target(e: &mut Engine, this: Ptr<HighProcess>) -> Ptr {
    e.get(this, HighProcess::pLastTarget)
}

// Translated from 008d9850 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ClearLastHeadTrackTarget` (Xbox PDB) clears `pLastTarget` (+0x41c).
pub fn high_process_clear_last_head_track_target(e: &mut Engine, this: Ptr<HighProcess>) {
    e.set(this, HighProcess::pLastTarget, Ptr::NULL);
}

// Translated from 008d9870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ClearDialogTarget` (Xbox PDB) clears `pDialogTarget` (+0x370).
pub fn high_process_clear_dialog_target(e: &mut Engine, this: Ptr<HighProcess>) {
    e.set(this, HighProcess::pDialogTarget, Ptr::NULL);
}

// Translated from 008d9890 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::_scalar_deleting_destructor_` (Xbox PDB): runs
/// `~HighProcess` and, when bit 0 of `flags` is set, frees the object
/// (`00401030`). Returns `this`.
pub fn high_process_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    flags: u32,
) -> Ptr<HighProcess> {
    high_process_destructor(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 008d98c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map. The element constructor `HighProcess`' constructor
/// gives to the `+0x35c` array of two `NiPointer<KFModel>`: it forwards to
/// `0044afe0` with a null pointer.
pub fn fn_008d98c0(e: &mut Engine, this: Ptr) {
    e.call(0x0044_afe0, &args![this, 0u32]);
}

/// Destroys one of the process' lists of owned items: deletes each item
/// (`00401030`) while the node is non-null and its item slot is non-null,
/// empties the list (`00470470`) and deletes the list itself (`004702f0`).
fn free_owned_list(e: &mut Engine, process: u32, offset: u32) {
    let mut node = e.mem.u32(process + offset);
    while node != 0 {
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        if e.mem.u32(slot) == 0 {
            break;
        }
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        let item = e.mem.u32(slot);
        e.call(OPERATOR_DELETE, &args![item]);
        node = e.call(NODE_NEXT, &args![node]).u32();
    }
    let list = e.mem.u32(process + offset);
    e.call(LIST_CLEAR, &args![list]);
    delete_list(e, process, offset);
}

/// Deletes the list whose pointer is at `process + offset` when it is not null
/// (`004702f0` with the delete flag).
fn delete_list(e: &mut Engine, process: u32, offset: u32) {
    let list = e.mem.u32(process + offset);
    if list != 0 {
        e.call(LIST_DELETE, &args![list, 1u32]);
    }
}

// Translated from 008d98e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::~HighProcess` (Xbox PDB). Sets the vtable, warns when the
/// player's own process is destroyed outside the shutdown (`005b5e40` with
/// the message at `010880d8`), releases the `+0x380` pointer (through the
/// virtual `+0xe8` of the object `009611e0` finds in it), the lip-sync
/// animation, the detection lists (the four that own their items delete them
/// first), the avoid areas, both sound handles, the greeting topic, the
/// leveled spell list, the muzzle flash and the cached actor values, then the
/// members' own destructors in reverse order and finally the base class'.
/// C++ exception states are not translated.
pub fn high_process_destructor(e: &mut Engine, this: Ptr<HighProcess>) {
    let base = this.addr();
    e.mem.set_u32(base, HIGH_PROCESS_VTABLE_ADDRESS);
    let player = player_pointer(e);
    let player_process = e.call(ACTOR_PROCESS, &args![player]).u32();
    if player_process == base {
        let shutting_down_object = e.global::<u32>(0x011c_3f2c);
        if !e.call(0x0042_26e0, &args![shutting_down_object]).bool() {
            e.call(0x005b_5e40, &args![0x0108_80d8u32]);
        }
    }
    // The `NiPointer` at +0x380.
    let held = e.call(NI_POINTER_GET, &args![base + 0x380]).u32();
    if held != 0 {
        let held = e.call(NI_POINTER_GET, &args![base + 0x380]).u32();
        let found = e.call(0x0096_11e0, &args![held]).u32();
        if found != 0 {
            let held = e.call(NI_POINTER_GET, &args![base + 0x380]).u32();
            let found = e.call(0x0096_11e0, &args![held]).u32();
            let held = e.call(NI_POINTER_GET, &args![base + 0x380]).u32();
            e.vcall(found, 0xe8, &args![held]);
        }
        e.call(NI_POINTER_SET, &args![base + 0x380, 0u32]);
    }
    // pLipSynicAnim (+0x3cc).
    let lip_sync = e.mem.u32(base + 0x3cc);
    if lip_sync != 0 {
        let lip_sync = e.mem.u32(base + 0x3cc);
        if lip_sync != 0 {
            e.call(0x004d_5850, &args![lip_sync, 1u32]);
        }
    }
    e.mem.set_u32(base + 0x3cc, 0);
    e.mem.set_u32(base + 0x2cc, 0);
    // pLastSpokeToList (+0x264) is emptied without deleting its items.
    let list = e.mem.u32(base + 0x264);
    e.call(LIST_CLEAR, &args![list]);
    delete_list(e, base, 0x264);
    // pDetectedActorList (+0x25c), pThreadDetectList (+0x268),
    // pTempActorsWhoDetectMeList (+0x26c), pActorsWhoDetectMeList (+0x260).
    free_owned_list(e, base, 0x25c);
    free_owned_list(e, base, 0x268);
    free_owned_list(e, base, 0x26c);
    free_owned_list(e, base, 0x260);
    // ClearAvoidAreas, then StopSoundHandle for both handles.
    e.call(0x0090_4160, &args![this]);
    for handle in 0..2u32 {
        e.call(0x008f_f1b0, &args![this, handle]);
    }
    // pGreetTopic (+0x368).
    let topic = e.mem.u32(base + 0x368);
    if topic != 0 {
        e.call(0x005c_90d0, &args![topic, 1u32]);
    }
    // pLeveledSpellList (+0x3b4).
    if e.mem.u32(base + 0x3b4) != 0 {
        let list = e.mem.u32(base + 0x3b4);
        e.call(LIST_CLEAR, &args![list]);
        delete_list(e, base, 0x3b4);
        e.mem.set_u32(base + 0x3b4, 0);
    }
    // pCurrentMuzzleFlash (+0x3d4).
    if e.mem.u32(base + 0x3d4) != 0 {
        let flash = e.mem.u32(base + 0x3d4);
        if flash != 0 {
            e.call(0x008d_9f70, &args![flash, 1u32]);
        }
    }
    e.mem.set_u32(base + 0x3d4, 0);
    // pActorValueCache (+0x428).
    if e.mem.u32(base + 0x428) != 0 {
        let cache = e.mem.u32(base + 0x428);
        e.call(OPERATOR_DELETE, &args![cache]);
        e.mem.set_u32(base + 0x428, 0);
    }
    e.call(NI_POINTER_SET, &args![base + 0x354, 0u32]);
    e.call(NI_POINTER_SET, &args![base + 0x358, 0u32]);
    e.call(KF_MODEL_POINTER_SET, &args![base + 0x35c, 0u32]);
    e.call(KF_MODEL_POINTER_SET, &args![base + 0x360, 0u32]);
    e.call(NI_POINTER_SET, &args![base + 0x434, 0u32]);
    // pActorsGeneratedDetectionEvent (+0x3dc).
    if e.mem.u32(base + 0x3dc) != 0 {
        let events = e.mem.u32(base + 0x3dc);
        e.call(OPERATOR_DELETE, &args![events]);
    }
    // Members, in reverse order of declaration.
    e.call(0x006e_bf90, &args![base + 0x468]);
    e.call(0x006e_bf90, &args![base + 0x464]);
    e.call(NI_POINTER_DESTRUCTOR, &args![base + 0x460]);
    e.call(NI_POINTER_DESTRUCTOR, &args![base + 0x45c]);
    e.call(NI_POINTER_DESTRUCTOR, &args![base + 0x434]);
    e.call(LIST_DESTRUCTOR, &args![base + 0x394]);
    e.call(LIST_DESTRUCTOR, &args![base + 0x38c]);
    e.call(NI_POINTER_DESTRUCTOR, &args![base + 0x380]);
    e.call(
        VECTOR_DESTRUCT,
        &args![base + 0x35c, 4u32, 2u32, KF_MODEL_POINTER_DESTRUCTOR],
    );
    e.call(
        VECTOR_DESTRUCT,
        &args![base + 0x354, 4u32, 2u32, NI_POINTER_DESTRUCTOR],
    );
    e.call(
        VECTOR_DESTRUCT,
        &args![base + 0x314, 0x0cu32, 2u32, SOUND_HANDLE_DESTRUCTOR],
    );
    e.call(LIST_DESTRUCTOR, &args![base + 0x28c]);
    e.call(LIST_DESTRUCTOR, &args![base + 0x284]);
    e.call(LIST_DESTRUCTOR, &args![base + 0x27c]);
    e.call(LIST_DESTRUCTOR, &args![base + 0x274]);
    e.call(BASE_DESTRUCTOR, &args![this]);
}

/// Copies every item of the list whose head is at `source_list` into the
/// list at `target_list`: walks the nodes while the item is non-null and
/// adds the address of each item slot with `00905820`.
fn copy_list_items(e: &mut Engine, source_list: u32, target_list: u32) {
    let mut node = source_list;
    while node != 0 {
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        if e.mem.u32(slot) == 0 {
            break;
        }
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        e.call(LIST_ADD_ITEM, &args![target_list, slot]);
        node = e.call(NODE_NEXT, &args![node]).u32();
    }
}

// Translated from 008d9fa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::HighCopy` (Xbox PDB): copies the state of `other` into
/// `this`: the package evaluation timer (through `008da180`), the use-item and
/// hold-attack timers, `bActivateAnim`, `bIsDoingSayTo` (the virtual `+0x78`
/// of `other`), the detected-actor and who-detects-me lists (added item by
/// item to this process' lists), the bone LOD, healing counts, reanimating
/// flag, lip-sync animation and flags, dialog target, queued-lip flag,
/// say-to flag, detection flag, light level and path look-at target.
pub fn high_process_high_copy(e: &mut Engine, this: Ptr<HighProcess>, other: Ptr<HighProcess>) {
    let t = this.addr();
    let o = other.addr();
    let eval_timer = fn_008da180(e, other);
    e.set(this, HighProcess::fPackageEvalTimer, eval_timer);
    let use_item = e.get(other, HighProcess::fUseItemTimer);
    e.set(this, HighProcess::fUseItemTimer, use_item);
    let hold_attack = e.get(other, HighProcess::fHoldAttackTimer);
    e.set(this, HighProcess::fHoldAttackTimer, hold_attack);
    let activate = e.get(other, HighProcess::bActivateAnim);
    e.set(this, HighProcess::bActivateAnim, activate);
    let say_to = e.vcall(o, SLOT_IS_DOING_SAY_TO, &args![]).u8();
    e.set(this, HighProcess::bIsDoingSayTo, say_to);
    let source = e.mem.u32(o + 0x25c);
    let target = e.mem.u32(t + 0x25c);
    copy_list_items(e, source, target);
    let source = e.mem.u32(o + 0x260);
    let target = e.mem.u32(t + 0x260);
    copy_list_items(e, source, target);
    let v = e.get(other, HighProcess::pBoneLOD);
    e.set(this, HighProcess::pBoneLOD, v);
    let v = e.get(other, HighProcess::iLastBoneLOD);
    e.set(this, HighProcess::iLastBoneLOD, v);
    let v = e.get(other, HighProcess::iHasHealingSpell);
    e.set(this, HighProcess::iHasHealingSpell, v);
    let v = e.get(other, HighProcess::iHasHealingPotion);
    e.set(this, HighProcess::iHasHealingPotion, v);
    let v = e.get(other, HighProcess::bCurrentlyReanimating);
    e.set(this, HighProcess::bCurrentlyReanimating, v);
    let v = e.get(other, HighProcess::pLipSynicAnim);
    e.set(this, HighProcess::pLipSynicAnim, v);
    let v = e.get(other, HighProcess::bWaitingForLipFile);
    e.set(this, HighProcess::bWaitingForLipFile, v);
    let v = e.get(other, HighProcess::pDialogTarget);
    e.set(this, HighProcess::pDialogTarget, v);
    let v = e.get(other, HighProcess::bLipQuequed);
    e.set(this, HighProcess::bLipQuequed, v);
    let v = e.get(other, HighProcess::bProcessGreetSayTo);
    e.set(this, HighProcess::bProcessGreetSayTo, v);
    let v = e.get(other, HighProcess::bEvaluateDetection);
    e.set(this, HighProcess::bEvaluateDetection, v);
    let v = e.get(other, HighProcess::fLightLevel);
    e.set(this, HighProcess::fLightLevel, v);
    let v = e.get(other, HighProcess::pPathLookAtTarget);
    e.set(this, HighProcess::pPathLookAtTarget, v);
}

// Translated from 008da180 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: returns `fPackageEvalTimer` (+0x2b4).
pub fn fn_008da180(e: &mut Engine, this: Ptr<HighProcess>) -> f32 {
    e.get(this, HighProcess::fPackageEvalTimer)
}

// Translated from 008da1a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map. Clears `pBoneLOD` (+0x2e4), then looks for the
/// bone-LOD controller of `actor`: `0043fcd0` gives the actor's model,
/// `00571530` (with the word `00571550` returns) its root node, and the
/// controller is the first node in the chain starting at `0043b230` of it,
/// following `004a8a90`, that `0045bad0` accepts together with the data at
/// `012031b8`; that node is stored in `pBoneLOD`.
pub fn fn_008da1a0(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    e.set(this, HighProcess::pBoneLOD, Ptr::NULL);
    let model = e.call(0x0043_fcd0, &args![actor]).u32();
    if model == 0 {
        return;
    }
    let global_word = e.call(0x0057_1550, &args![]).u32();
    let root = e.call(0x0057_1530, &args![actor, model, global_word]).u32();
    if root == 0 {
        return;
    }
    let mut node = e.call(0x0043_b230, &args![root]).u32();
    while node != 0 {
        if e.call(0x0045_bad0, &args![0x0120_31b8u32, node]).bool() {
            e.set(this, HighProcess::pBoneLOD, Ptr::<()>::new(node));
            return;
        }
        node = e.call(0x004a_8a90, &args![node]).u32();
    }
}

// Translated from 008da230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetAnimAction` (Xbox PDB) returns `sAnimAction` (+0x2ec), sign-extended.
pub fn high_process_get_anim_action(e: &mut Engine, this: Ptr<HighProcess>) -> i32 {
    e.get(this, HighProcess::sAnimAction) as i32
}

// Translated from 008da250 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetAnimActionAnimSeq` (Xbox PDB) returns `pAnimSeq` (+0x2f0).
pub fn high_process_get_anim_action_anim_seq(e: &mut Engine, this: Ptr<HighProcess>) -> Ptr {
    e.get(this, HighProcess::pAnimSeq)
}

// Translated from 008da270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::SetAnimAction` (Xbox PDB) stores `sAnimAction` (+0x2ec) and `pAnimSeq` (+0x2f0).
pub fn high_process_set_anim_action(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    action: i16,
    anim_seq: Ptr,
) {
    e.set(this, HighProcess::sAnimAction, action);
    e.set(this, HighProcess::pAnimSeq, anim_seq);
}

// Translated from 008da2a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::CanAttack` (Xbox PDB). The animation (virtual `+0x1b8`) and
/// the anim action (virtual `+0x3e4`) decide: actions 2 and 5 refuse when the
/// animation's `00491040(2)` sub-object has an anim group (`005f2420`) of
/// `0xe2` or `0xf0`; actions -1, 3, 4, 6, 7 and 0x11 go on; every other
/// action refuses. Then it refuses when the animation reports
/// `008846c0`, and for the player's own process while the Pipboy is active
/// (`00967ae0`), and for another process when the animation's reference
/// (`008da3d0`) is an idle form that `005ff260` rejects.
pub fn high_process_can_attack(e: &mut Engine, this: Ptr<HighProcess>) -> bool {
    let process = this.addr();
    let animation = e.vcall(process, SLOT_GET_ANIMATION, &args![]).u32();
    let action = e.vcall(process, SLOT_GET_ANIM_ACTION, &args![]).i32();
    match action {
        2 | 5 => {
            if animation != 0 {
                let part = e.call(0x0049_1040, &args![animation, 2u32]).u32();
                if part != 0 {
                    let group = e.call(0x0048_f7f0, &args![part]).u32();
                    let kind = e.call(0x005f_2420, &args![group]).u32();
                    if kind == 0xe2 || kind == 0xf0 {
                        return false;
                    }
                }
            }
        }
        -1 | 3 | 4 | 6 | 7 | 0x11 => {}
        _ => return false,
    }
    if animation != 0 && e.call(0x0088_46c0, &args![animation]).bool() {
        return false;
    }
    let player = player_pointer(e);
    let player_process = e.call(ACTOR_PROCESS, &args![player]).u32();
    if process == player_process {
        if e.call(0x0096_7ae0, &args![player]).bool() {
            return false;
        }
    } else if animation != 0 {
        let form = fn_008da3d0(e, Ptr::new(animation));
        if form != 0 && e.call(0x005f_f260, &args![form]).bool() {
            return false;
        }
    }
    true
}

// Translated from 008da3d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map. `this` is an `Animation`: when the `NiPointer`
/// at +0x124 holds an object, returns `0055b980` of that object, else 0.
pub fn fn_008da3d0(e: &mut Engine, this: Ptr) -> u32 {
    let held = e.call(NI_POINTER_GET, &args![this.addr() + 0x124]).u32();
    if held == 0 {
        return 0;
    }
    let held = e.call(NI_POINTER_GET, &args![this.addr() + 0x124]).u32();
    e.call(0x0055_b980, &args![held]).u32()
}

// Translated from 008da420 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::CanForceGreet` (Xbox PDB): whether the player may be greeted
/// by `other`. Refuses when `00525430` of the object at `011f2250` holds, when
/// `other` has no 360-degree line of sight to the player, when the player's
/// package (type 6) or state (anything but 0 or 4) says so, or when the
/// player's `00885560` value (with the float at `00436aa0`+8 and `008d6f30`)
/// is at least 0.875. Then it depends on `sAnimAction`: -1 and 7 allow the
/// greeting when the sit/sleep state (+0x13d) is 0, 4 or 9; 2, 4 and 5 set the
/// byte at `011e07a9` when `other` is the player and refuse; others refuse.
/// Two comparisons the code computes against `01016408` and `0104ee98` are
/// not used and are left out.
pub fn high_process_can_force_greet(e: &mut Engine, this: Ptr<HighProcess>, other: Ptr) -> bool {
    if e.call(0x0052_5430, &args![0x011f_2250u32]).bool() {
        return false;
    }
    let player = player_pointer(e);
    if !e.call(0x0088_c600, &args![other, player]).bool() {
        return false;
    }
    let player = player_pointer(e);
    if e.call(ACTOR_CURRENT_PACKAGE, &args![player]).u32() != 0 {
        let package = e.call(ACTOR_CURRENT_PACKAGE, &args![player]).u32();
        if e.call(PACKAGE_TYPE, &args![package]).u32() == 6 {
            return false;
        }
    }
    if actor_state(e, player) != 0 && actor_state(e, player) != 4 {
        return false;
    }
    let first = e.call(0x008d_6f30, &args![player]).u32();
    let object = e.call(0x0043_6aa0, &args![player]).u32();
    let value = e.mem.f32(object + 8);
    let level = e.call(0x0088_5560, &args![player, value, first]).f32();
    let level = f64::from(level);
    if level >= e.global::<f64>(0x0107_3828) || level.is_nan() {
        return false;
    }
    // sAnimAction (+0x2ec) as the switch value.
    match e.get(this, HighProcess::sAnimAction) {
        2 | 4 | 5 => {
            if other.addr() == player_pointer(e) {
                e.mem.set_u8(0x011e_07a9, 1);
            }
            false
        }
        -1 | 7 => {
            // MiddleHighProcess::cSitSleepState (Xbox PDB) +0x13d
            let state = e.mem.u8(this.addr() + 0x13d);
            state == 0 || state == 4 || state == 9
        }
        _ => false,
    }
}

// Translated from 008da600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ClearAcquireList` (Xbox PDB): empties the `ObjectList`
/// (`BSSimpleList<ObjectstoAcquire *>`, +0x5c) one head item at a time, each
/// item destroyed with `007b3fa0` (delete flag 1) before it is removed.
pub fn high_process_clear_acquire_list(e: &mut Engine, this: Ptr<HighProcess>) {
    let list = this.addr() + 0x5c;
    while !e.call(0x0082_56d0, &args![list]).bool() {
        let slot = e.call(NODE_ITEM_ADDRESS, &args![list]).u32();
        let item = e.mem.u32(slot);
        e.with_stack(4, |e, holder| {
            e.mem.set_u32(holder.addr(), item);
            if item != 0 {
                e.call(0x007b_3fa0, &args![item, 1u32]);
            }
            e.call(LIST_REMOVE_ITEM, &args![list, holder]);
        });
    }
}

// Translated from 008da670 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::CheckforNewPackage` (Xbox PDB): evaluates whether the
/// actor should switch package. Returns false unless the actor's state is
/// 0, 4 or 9; a run-once package (virtual `+0x20c`) blocks it unless `force`;
/// `GetEndIdlesPlayed` (`+0x594`) blocks it. Then, unless the current
/// package is a script package whose procedure maps to 0x36, it re-evaluates
/// (`LowProcess::CheckforNewPackage`, `0090a1a0`) when `force`, no current
/// package (`+0x22c`), the evaluation timer has run out (at most the double
/// at `01012060`) or the game hour (truncated) differs from
/// `iHourPackageEvaluated`, and then resets the timer to the float at
/// `01017868`. The timer then drops by the frame time. After a change the
/// run-once package is cleared (`+0x214`, unless it is type 0x1a) and the
/// shooting action is cleared (`0092bf20`). Returns whether the package changed.
pub fn high_process_checkfor_new_package(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    force: u8,
) -> bool {
    let process = this.addr();
    let a = actor.addr();
    if !actor_state_is_0_4_or_9(e, a) {
        return false;
    }
    let run_once = e.vcall(process, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32();
    if run_once != 0 && force == 0 {
        return false;
    }
    if e.vcall(process, SLOT_GET_END_IDLES_PLAYED, &args![]).bool() {
        return false;
    }
    if e.vcall(process, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32() != 0 {
        let package = e.vcall(process, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32();
        if e.call(PACKAGE_TYPE, &args![package]).u32() != 0x1a {
            e.vcall(process, SLOT_CLEAR_RUN_ONCE_PACKAGE, &args![]);
        }
    }
    e.call(ACTOR_CURRENT_PACKAGE, &args![actor]);
    let hour = e.call(0x0086_7da0, &args![0x011d_e7b8u32]).f32();
    let mut changed = false;
    let set_as_current = e.call(0x0088_1510, &args![actor]).u32();
    if set_as_current != 0 && e.call(0x0067_4dd0, &args![set_as_current]).bool() {
        let kind = e.call(0x0096_11e0, &args![set_as_current]).u32();
        let procedure = e
            .vcall(process, SLOT_GET_CURRENT_PROCEDURE_INDEX, &args![])
            .u32();
        let table = e.mem.u32(0x011a_3ff0 + kind.wrapping_mul(4));
        if e.mem.u32(table.wrapping_add(procedure.wrapping_mul(4))) != 0x36 {
            return changed;
        }
    }
    let mut keep = false;
    if force == 0 && e.vcall(process, SLOT_GET_CURRENT_PACKAGE, &args![]).u32() != 0 {
        let timer = e.get(this, HighProcess::fPackageEvalTimer);
        let timer = f64::from(timer);
        if timer > e.global::<f64>(0x0101_2060) || timer.is_nan() {
            let truncated = e.call(FLOAT_TO_INT, &args![f64::from(hour)]).u32();
            keep = e.get(this, HighProcess::iHourPackageEvaluated) as u32 == truncated;
        }
    }
    if !keep {
        changed = e
            .call(0x0090_a1a0, &args![this, actor, force as u32])
            .bool();
        let truncated = e.call(FLOAT_TO_INT, &args![f64::from(hour)]).i32();
        e.set(this, HighProcess::iHourPackageEvaluated, truncated);
        let reset = e.global::<f32>(0x0101_7868);
        e.set(this, HighProcess::fPackageEvalTimer, reset);
    }
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
    let timer = e.get(this, HighProcess::fPackageEvalTimer);
    e.set(
        this,
        HighProcess::fPackageEvalTimer,
        (f64::from(timer) - frame) as f32,
    );
    if changed {
        let run_once = e.vcall(process, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32();
        e.vcall(process, SLOT_GET_CURRENT_PACKAGE, &args![]);
        if run_once != 0 && e.call(PACKAGE_TYPE, &args![run_once]).u32() != 0x1a {
            e.vcall(process, SLOT_CLEAR_RUN_ONCE_PACKAGE, &args![]);
        }
        e.call(0x0092_bf20, &args![this, actor]);
    }
    changed
}

// Translated from 008da8b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::SetupNewPackage` (Xbox PDB): the base class' setup
/// (`00915400`), then resets the package state: `bActivateAnim`, the shot
/// counters (0, -1, 0), the hold-attack, script-package-end, use-item and
/// acquire timers, `bStop`, `bWeaponAlertDrawn`, `pCurrentIdle` (+0x194) and
/// `bFinishingCombatPackage`.
pub fn high_process_setup_new_package(e: &mut Engine, this: Ptr<HighProcess>) {
    e.call(MIDDLE_HIGH_SETUP_NEW_PACKAGE, &args![this]);
    e.set(this, HighProcess::bActivateAnim, 0);
    e.set(this, HighProcess::sShotsFired, 0);
    e.set(this, HighProcess::sShotsToFire, -1);
    e.set(this, HighProcess::sBurstsFired, 0);
    e.set(this, HighProcess::fHoldAttackTimer, 0.0);
    e.set(this, HighProcess::fScriptPackageEndTime, 0.0);
    e.set(this, HighProcess::fUseItemTimer, 0.0);
    e.set(this, HighProcess::bStop, 0);
    e.set(this, HighProcess::fEvaluateAcquireTimer, 0.0);
    e.set(this, HighProcess::bWeaponAlertDrawn, 0);
    // MiddleHighProcess::pCurrentIdle (Xbox PDB) +0x194
    e.mem.set_u32(this.addr() + 0x194, 0);
    e.set(this, HighProcess::bFinishingCombatPackage, 0);
}

// Translated from 008da950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::SetActorsAnimation` (Xbox PDB): unless the movement is
/// stopped (virtual `+0x49c`), sets the actor's move mode to the actor's
/// current mode bits `0xc00` OR the requested `mode` (-1 means 0x100; 0x201
/// when `flag` is set and the running package passes `0067a480`).
pub fn high_process_set_actors_animation(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    mode: i32,
    flag: u8,
) {
    let process = this.addr();
    if e.vcall(process, SLOT_GET_MOVEMENT_STOPPED, &args![]).bool() {
        return;
    }
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    let mut mode = mode;
    if mode == -1 {
        mode = 0x100;
    }
    if flag != 0 && package != 0 && e.call(0x0067_a480, &args![package]).bool() {
        mode = 0x201;
    }
    let current = e.call(ACTOR_MOVE_MODE, &args![actor]).u32() & 0xc00;
    let combined = (current as i32 | mode) as i16 as i32;
    e.call(ACTOR_SET_MOVE_MODE, &args![actor, combined]);
}

// Translated from 008da9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::EndMoveMessage` (Xbox PDB): marks the movement complete
/// (virtual `+0x120`, argument 1) and, when `actor` is given, stops it
/// moving (`008b3ab0`).
pub fn high_process_end_move_message(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    e.vcall(
        this.addr(),
        SLOT_SET_CURRENT_MOVEMENT_COMPLETE,
        &args![1u32],
    );
    if !actor.is_null() {
        e.call(0x008b_3ab0, &args![actor]);
    }
}

// Translated from 008daa20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::CalculateMoveMode` (Xbox PDB): returns the move mode word
/// for `actor`. 0 when `00437bf0` or `00437bd0` hold; 0x201 when the current
/// weapon is a grenade (`008d8220`), the running package passes `0067a480`
/// or `00493bb0` holds; otherwise 0x201 when `run_flag`, 0x101 when
/// `walk_flag`, else it compares the three distances: with bit 0x200 of the
/// actor's move mode set, 0x101 only if `walk_limit > distance`, else 0x201;
/// without it 0x201 only if `distance > run_limit`, else 0x101 (unordered
/// values give 0x201 and 0x101 respectively, as the x87 flags do).
#[allow(clippy::too_many_arguments)]
pub fn high_process_calculate_move_mode(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    distance: f32,
    walk_limit: f32,
    run_limit: f32,
    run_flag: u8,
    walk_flag: u8,
) -> u32 {
    e.call(ACTOR_MOVE_MODE, &args![actor]);
    let package = e
        .vcall(this.addr(), SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if e.call(0x0043_7bf0, &args![actor]).bool() || e.call(0x0043_7bd0, &args![actor]).bool() {
        return 0;
    }
    if e.call(0x008d_8220, &args![actor]).bool()
        || (package != 0 && e.call(0x0067_a480, &args![package]).bool())
        || e.call(0x0049_3bb0, &args![actor]).bool()
    {
        return 0x201;
    }
    if run_flag != 0 {
        return 0x201;
    }
    if walk_flag != 0 {
        return 0x101;
    }
    let mode = e.call(ACTOR_MOVE_MODE, &args![actor]).u32();
    if mode & 0x200 != 0 {
        if walk_limit > distance {
            0x101
        } else {
            0x201
        }
    } else if run_limit < distance {
        0x201
    } else {
        0x101
    }
}

// Translated from 008dab40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::SetupSpecialIdle` (Xbox PDB): starts the special idle `idle`
/// (or picks one) on `actor`; true when one was started. The actor must be an
/// actor with an animation, in state 0, 4 or 9, not paused, without a package
/// flagged in bit 24 (`fn_008dade0`); when no idle is given, or `check_conditions`
/// is set and the idle's conditions (`00436aa0` + `00680c30`, evaluated for
/// the actor and the process' target) fail or the actor may not use it
/// (`008b2010`), one is chosen by the idle manager (`00600950`) when the
/// animation allows it and the actor has no anim action or `play_type` is 2.
/// It needs the animation to be done playing (`004985f0`), an idle given,
/// the actor to be the player or `start_anyway`. The idle goes to
/// `pIdleToPlay` with `play_type`; the player's is played at once with
/// `finish_now` (virtual `+0x70c`), any other actor queues the
/// post-animation action 0x10 (virtual `+0x614`).
#[allow(clippy::too_many_arguments)]
pub fn high_process_setup_special_idle(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    idle: Ptr,
    play_type: i32,
    finish_now: u8,
    start_anyway: u8,
    check_conditions: u8,
) -> bool {
    let process = this.addr();
    let a = actor.addr();
    let mut idle = idle.addr();
    let mut target = 0u32;
    let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
    if e.vcall(a, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
        target = a;
    }
    if target == 0 || animation == 0 {
        return false;
    }
    if target == player_pointer(e) && e.call(0x0044_ddc0, &args![0x011f_2250u32]).i32() == 4 {
        return false;
    }
    let state = actor_state(e, target);
    if state != 0 && state != 4 && state != 9 {
        return false;
    }
    if e.call(0x0043_7bf0, &args![target]).bool() && idle == 0 {
        return false;
    }
    if e.vcall(target, ACTOR_SLOT_PAUSED_B, &args![]).bool()
        || e.vcall(target, ACTOR_SLOT_PAUSED_A, &args![]).bool()
    {
        return false;
    }
    if e.call(ACTOR_CURRENT_PACKAGE, &args![target]).u32() != 0 {
        let package = e.call(ACTOR_CURRENT_PACKAGE, &args![target]).u32();
        if fn_008dade0(e, Ptr::new(package)) {
            return false;
        }
    }
    let done_playing = e.call(0x0049_85f0, &args![animation]).bool();
    if !(done_playing || idle != 0 || player_pointer(e) == target || start_anyway != 0) {
        return false;
    }
    if check_conditions != 0 && idle != 0 && e.call(0x0043_6aa0, &args![idle]).u32() != 0 {
        let subject = e.vcall(process, SLOT_GET_TARGET, &args![]).u32();
        let conditions = e.call(0x0043_6aa0, &args![idle]).u32();
        if !e
            .call(0x0068_0c30, &args![conditions, target, subject])
            .bool()
        {
            idle = 0;
        }
    }
    if idle != 0 && !e.call(0x008b_2010, &args![target, idle]).bool() {
        idle = 0;
    }
    if idle == 0
        && !e.call(0x0049_8f80, &args![animation]).bool()
        && (e.call(0x008a_7570, &args![target]).i32() == -1 || play_type == 2)
    {
        let subject = e.vcall(process, SLOT_GET_TARGET, &args![]).u32();
        let manager = e.global::<u32>(0x011c_b6a0);
        idle = e.call(0x0060_0950, &args![manager, actor, subject]).u32();
    }
    if idle == 0 || e.call(0x0049_8d30, &args![animation, idle]).bool() {
        return false;
    }
    e.set(this, HighProcess::pIdleToPlay, Ptr::<()>::new(idle));
    e.set(this, HighProcess::eSpecialIdleType, play_type as u32);
    e.vcall(process, SLOT_SET_END_IDLES_PLAYED, &args![0u32]);
    if target == player_pointer(e) {
        if finish_now != 0 {
            let player = player_pointer(e);
            e.vcall(process, SLOT_FINISH_SETUP_SPECIAL_IDLE, &args![player]);
        }
        e.set(this, HighProcess::pIdleToPlay, Ptr::NULL);
    } else {
        e.vcall(process, SLOT_ADD_POST_ANIMATION_ACTION, &args![0x10u32]);
    }
    true
}

// Translated from 008dade0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: whether bit 24 of the flags word at +0x1c is set.
pub fn fn_008dade0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x1c) & 0x0100_0000 != 0
}

// Translated from 008dae00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::FinishSetupSpecialIdle` (Xbox PDB): with an animation and
/// a pending idle (`pIdleToPlay`, +0x350): when the animation reports
/// `00498f80` only queues the post-animation action 0x10; otherwise plays the
/// idle (`00497f20` with the idle's anim group section `005ff160`),
/// clears `bDoneOnce` (+0xe0) and the idle, sets `eSpecialIdleType` back to 2
/// and, when the weapon is drawn and an explosive is planted, calls `0089f580`.
pub fn high_process_finish_setup_special_idle(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let animation = e.vcall(actor.addr(), ACTOR_SLOT_ANIMATION, &args![]).u32();
    if animation == 0 || e.get(this, HighProcess::pIdleToPlay).is_null() {
        return;
    }
    if e.call(0x0049_8f80, &args![animation]).bool() {
        e.vcall(process, SLOT_ADD_POST_ANIMATION_ACTION, &args![0x10u32]);
        return;
    }
    let idle = e.get(this, HighProcess::pIdleToPlay);
    let play_type = e.get(this, HighProcess::eSpecialIdleType);
    let section = e.call(0x005f_f160, &args![idle, play_type]).u32();
    let idle = e.get(this, HighProcess::pIdleToPlay);
    e.call(0x0049_7f20, &args![animation, idle, actor, section]);
    // MiddleHighProcess::bDoneOnce (Xbox PDB) +0xe0
    e.mem.set_u8(process + 0xe0, 0);
    e.set(this, HighProcess::pIdleToPlay, Ptr::NULL);
    e.set(this, HighProcess::eSpecialIdleType, 2);
    if e.vcall(process, SLOT_GET_WEAPON_DRAWN, &args![]).bool()
        && e.vcall(process, SLOT_GET_PLANTED_EXPLOSIVE, &args![])
            .bool()
    {
        e.call(0x0089_f580, &args![actor]);
    }
}

// Translated from 008daef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::FreeUpSpecialIdle` (Xbox PDB): queues the post-animation
/// action 0x800 (virtual `+0x614`). The function pops one stack word that it
/// never reads.
pub fn high_process_free_up_special_idle(e: &mut Engine, this: Ptr<HighProcess>, _unused_1: u32) {
    e.vcall(
        this.addr(),
        SLOT_ADD_POST_ANIMATION_ACTION,
        &args![0x800u32],
    );
}

// Translated from 008daf20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::PostAnimFreeUpSpecialIdle` (Xbox PDB): when the actor's
/// state is 0, 4 or 9, frees the actor's special idle (`00498910` of its
/// animation with 1, 0) and tells the face animation data (`008adcb0`, virtual
/// `+0xbc`) to blend with the float at `01016264` and four 1s.
pub fn high_process_post_anim_free_up_special_idle(
    e: &mut Engine,
    _this: Ptr<HighProcess>,
    actor: Ptr,
) {
    let a = actor.addr();
    if !actor_state_is_0_4_or_9(e, a) {
        return;
    }
    let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
    if animation != 0 {
        e.call(0x0049_8910, &args![animation, 1u32, 0u32]);
    }
    let face = e.call(0x008a_dcb0, &args![actor]).u32();
    if face != 0 {
        let blend = e.global::<f32>(0x0101_6264);
        e.vcall(face, 0xbc, &args![blend, 1u32, 1u32, 1u32, 1u32]);
    }
}

// Translated from 008dafd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::RandomlyPlaySpecialIdles` (Xbox PDB): ticks the idle timer
/// and, when it runs out, starts a random special idle on `actor`. Does
/// nothing without a 3D node, with an idle pending (`pIdleToPlay`), when the
/// player is farther than `setting(011cd674) * (size / 64)` (the size from
/// the node's bound), when the running package forbids it (`005f36f0`, type
/// 0x1a), without an animation that is free (`00498f80`, `004985f0`), while an
/// anim action runs, while the actor eats (`008a7870`) or is paused (`+0x234`).
/// With the timer (`+0x334`) below 0 and the setting at `011cdd30` not 0 it
/// calls `SetupSpecialIdle(actor, 0, 2, 1, 0, 1)` and sets the timer to that
/// setting; otherwise it lowers the timer by the frame time.
pub fn high_process_randomly_play_special_idles(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
) {
    let process = this.addr();
    let a = actor.addr();
    if e.vcall(a, ACTOR_SLOT_NODE, &args![]).u32() == 0 {
        return;
    }
    let mut package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
    if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x1c {
        let extra = e.call(0x005d_43c0, &args![actor]).u32();
        package = e.call(0x0041_cb10, &args![extra]).u32();
    }
    if e.vcall(process, SLOT_GET_END_IDLES_PLAYED, &args![]).bool()
        && !e.call(0x0049_8f80, &args![animation]).bool()
        && e.call(0x0049_85f0, &args![animation]).bool()
    {
        e.vcall(process, SLOT_SET_END_IDLES_PLAYED, &args![0u32]);
    }
    let node = e.vcall(a, ACTOR_SLOT_NODE, &args![]).u32();
    let bound = e.call(0x0043_d450, &args![node]).u32();
    let size = e.call(FRAME_TIME, &args![bound]).f32();
    let scale = (f64::from(size) / e.global::<f64>(0x0102_40c0)) as f32;
    if e.get(this, HighProcess::pIdleToPlay).addr() != 0 {
        return;
    }
    let player = player_pointer(e);
    let distance = e
        .call(DISTANCE_FROM_REFERENCE, &args![actor, player, 0u32, 1u32])
        .f64();
    let setting = e.call(SETTING_VALUE, &args![0x011c_d674u32]).u32();
    let limit = f64::from(e.mem.f32(setting)) * f64::from(scale);
    if limit < distance {
        return;
    }
    if package != 0
        && (e.call(0x005f_36f0, &args![package]).u32() != 0
            || e.call(PACKAGE_TYPE, &args![package]).u32() == 0x1a)
    {
        return;
    }
    if animation == 0
        || e.call(0x0049_8f80, &args![animation]).bool()
        || !e.call(0x0049_85f0, &args![animation]).bool()
        || e.vcall(process, SLOT_GET_ANIM_ACTION, &args![]).i32() != -1
        || e.call(0x008a_7870, &args![actor]).bool()
        || e.vcall(a, ACTOR_SLOT_PAUSED_B, &args![]).bool()
    {
        return;
    }
    let timer = e.vcall(process, SLOT_GET_IDLE_TIMER, &args![]).f32();
    if timer < 0.0 {
        let setting = e.call(SETTING_VALUE, &args![0x011c_dd30u32]).u32();
        if e.mem.f32(setting) != 0.0 {
            e.vcall(
                process,
                SLOT_SETUP_SPECIAL_IDLE,
                &args![actor, 0u32, 2u32, 1u32, 0u32, 1u32],
            );
            let setting = e.call(SETTING_VALUE, &args![0x011c_dd30u32]).u32();
            let value = e.mem.f32(setting);
            e.vcall(process, SLOT_SET_IDLE_TIMER, &args![value]);
            return;
        }
    }
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
    let remaining = (f64::from(timer) - frame) as f32;
    e.vcall(process, SLOT_SET_IDLE_TIMER, &args![remaining]);
}

// Translated from 008db240 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: a package step for the run-time package of type
/// 0x17 (the package `+0x27c` gives). Takes the process' target
/// (`BaseProcess::pTarget`, +0x40; set through virtual `+0x8c` when empty)
/// when it is an actor. With the type-0x17 package and an actor target:
/// the target not attackable (virtual `+0x448`) leaves with index +1;
/// package types 0x22 and 0x23 with `008b06d0` true end the interrupt
/// package and start combat with the target (`actor` virtual `+0x424`);
/// otherwise, while the package's `00644790` value is 0 or below, it says the
/// topic 0x14 as the `008d6f30` owner (`008bff30` brackets), adds the setting at
/// `011cd2c4` to the package's counter (`008db4c0`), runs `009f94d0(1)` and
/// leaves with index -1. Without such a package but with an actor target it
/// has the player process greet it with topic 9 (type 0x22) or 0xc and leaves
/// with index +1.
pub fn fn_008db240(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let current = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    let mut watched_package = 0u32;
    let mut target_actor = 0u32;
    // BaseProcess::pTarget (Xbox PDB) +0x40
    if e.mem.u32(process + 0x40) == 0 {
        e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
    }
    if e.mem.u32(process + 0x40) != 0 {
        let target = e.mem.u32(process + 0x40);
        if e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
            target_actor = e.mem.u32(process + 0x40);
        }
    }
    if current != 0 && e.call(PACKAGE_TYPE, &args![current]).u32() == 0x17 {
        watched_package = current;
    }
    let mut leave_with_minus_one = false;
    if watched_package != 0 {
        if target_actor == 0 {
            return;
        }
        if !e.vcall(target_actor, SLOT_ATTACK_CALLBACK, &args![]).bool() {
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
            return;
        }
        let kind = e.call(PACKAGE_TYPE, &args![watched_package]).u32();
        let kind_again = if kind == 0x22 {
            kind
        } else {
            e.call(PACKAGE_TYPE, &args![watched_package]).u32()
        };
        if kind == 0x22 || kind_again == 0x23 {
            let attack_flags = e.with_stack(4, |e, out| {
                e.mem.set_u32(out.addr(), 0);
                e.call(0x008b_06d0, &args![actor, target_actor, 0u32, out, 0u32])
                    .bool()
            });
            if attack_flags {
                e.call(ACTOR_END_INTERRUPT_PACKAGE, &args![actor, 0u32]);
                e.vcall(
                    a,
                    ACTOR_SLOT_COMBAT_CONTROLLER_ATTACK,
                    &args![target_actor, 0u32, 0u32, 0u32, 0u32, 0u32, 1u32, 0u32],
                );
                return;
            }
        }
        let value = e.call(0x0064_4790, &args![watched_package]).f64();
        if value <= e.global::<f64>(0x0101_2060) {
            let player = player_pointer(e);
            let owner_source = e.call(0x008d_6f30, &args![player]).u32();
            let owner = e.call(0x0054_6a40, &args![owner_source]).u32();
            e.call(0x008b_ff30, &args![owner]);
            let topic = e.call(GET_TOPIC, &args![2u32, 0x14u32]).u32();
            e.call(0x008b_ff30, &args![0u32]);
            if topic != 0 {
                e.vcall(
                    process,
                    SLOT_PROCESS_GREET,
                    &args![actor, topic, 0u32, 0u32, 0u32, 0u32],
                );
                let setting = e.call(SETTING_VALUE, &args![0x011c_d2c4u32]).u32();
                let amount = e.mem.f32(setting);
                fn_008db4c0(e, Ptr::new(watched_package), amount);
                e.call(0x009f_94d0, &args![watched_package, 1u32]);
                leave_with_minus_one = true;
            }
        }
        if leave_with_minus_one {
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, -1i32],
            );
        }
    } else if target_actor != 0 {
        let topic_index = if e.call(PACKAGE_TYPE, &args![current]).u32() == 0x22 {
            9u32
        } else {
            0xcu32
        };
        let topic = e.call(GET_TOPIC, &args![2u32, topic_index]).u32();
        let greeter = e.call(ACTOR_PROCESS, &args![actor]).u32();
        e.vcall(
            greeter,
            SLOT_PROCESS_GREET,
            &args![target_actor, topic, 0u32, 0u32, 1u32, 0u32],
        );
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
    }
}

// Translated from 008db4c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: adds `amount` to the float at +0x80 of `this`
/// (the package of type 0x17 in `008db240`).
pub fn fn_008db4c0(e: &mut Engine, this: Ptr, amount: f32) {
    let current = e.mem.f32(this.addr() + 0x80);
    e.mem.set_f32(
        this.addr() + 0x80,
        (f64::from(current) + f64::from(amount)) as f32,
    );
}

/// `Actor` virtual `+0x410` (meaning unconfirmed): `ProcessObserveCombat`
/// calls it with the first threat, three 1s, two 0s and the float at `01012054` twice.
const ACTOR_SLOT_0X410: u32 = 0x410;
/// `Actor` virtual `+0x284` (meaning unconfirmed): says a dialogue response;
/// takes twelve words and returns the delay in `ST0`.
const ACTOR_SLOT_SAY: u32 = 0x284;
const SLOT_CLEAR_ACTION_HEAD_TRACK: u32 = 0x644;
/// `.?AVTESPackage@@` type descriptor (`011846a0`).
const TYPE_PACKAGE: u32 = 0x0118_46a0;
/// `.?AVFleePackage@@` type descriptor (`0118c6f4`).
const TYPE_FLEE_PACKAGE: u32 = 0x0118_c6f4;

// Translated from 008db4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessCreateFollow` (Xbox PDB): creates the followers an
/// escort package asks for. The package (`+0x27c`) gives its package data
/// (`00671d10`) whose target type (`00519b00`) is 0 or 3 (a reference), 1 (a
/// type) or other; the wanted count is 1 for a reference target, else
/// `0044ddc0` of the data, less what the escort data already has of the
/// target (`009f0a80`). For each wanted follower it creates one
/// (`006780e0`), makes it the process' target and, if that is an actor,
/// counts it (the player is only noted, `009f0a20` records others). A
/// reference target with no actor ends the call by moving on; otherwise, when
/// the hour (reduced by `00408840`) has passed the limit (`01070c80`) or no
/// more followers are needed, it adds 1 to the running procedure index.
pub fn high_process_process_create_follow(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if package == 0 {
        return;
    }
    let data = e.call(0x0067_1d10, &args![package]).u32();
    if data == 0 {
        return;
    }
    let target_type = e.call(0x0051_9b00, &args![data]).i32();
    let target_is_reference = target_type == 0 || target_type == 3;
    let target_is_type = target_type == 1;
    let escort_data = e
        .vcall(process, SLOT_GET_INSTANCE_DATA_THAT_IS_RUNNING, &args![])
        .u32();
    let target_object = e.call(0x0068_0050, &args![data]).u32();
    let player = player_pointer(e);
    let mut player_found = e.with_stack(4, |e, holder| {
        e.mem.set_u32(holder.addr(), player);
        e.call(0x005f_65d0, &args![escort_data + 0x10, holder])
            .bool()
    });
    let mut wanted = if target_is_reference {
        1
    } else {
        e.call(0x0044_ddc0, &args![data]).i32()
    };
    let mut loop_limit = wanted;
    if target_is_type {
        loop_limit -= e
            .call(0x009f_0a80, &args![escort_data, actor, target_object])
            .i32();
    }
    let mut created = 0;
    while created < loop_limit {
        let follower = e.call(0x0067_80e0, &args![package, actor, 1u32]).u32();
        e.vcall(process, SLOT_SET_TARGET, &args![follower]);
        // BaseProcess::pTarget (Xbox PDB) +0x40
        if e.mem.u32(process + 0x40) == 0 {
            if target_is_reference {
                if player_found {
                    let player = player_pointer(e);
                    e.vcall(process, SLOT_SET_TARGET, &args![player]);
                }
                if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
                    e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
                }
                e.vcall(
                    process,
                    SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                    &args![actor, 1u32],
                );
                return;
            }
            break;
        }
        let target = e.mem.u32(process + 0x40);
        if e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
            if e.mem.u32(process + 0x40) == player_pointer(e) {
                player_found = true;
                wanted -= 1;
            } else {
                e.vcall(
                    process,
                    SLOT_CREATE_FOLLOW_FOR_ESCORT,
                    &args![actor, package, 0u32],
                );
                let instance_data = e
                    .vcall(process, SLOT_GET_INSTANCE_DATA_THAT_IS_RUNNING, &args![])
                    .u32();
                let new_target = e.mem.u32(process + 0x40);
                e.call(0x009f_0a20, &args![instance_data, new_target]);
            }
        }
        created += 1;
    }
    if player_found {
        let player = player_pointer(e);
        e.vcall(process, SLOT_SET_TARGET, &args![player]);
    }
    if wanted > 0 {
        wanted -= e
            .call(0x009f_0a80, &args![escort_data, actor, target_object])
            .i32();
    }
    // ActorPackage subobjects: RunOncePackage (+0xe4) when there is one,
    // else CurrentPackage (+0x04).
    let package_slot = if e.vcall(process, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32() != 0 {
        process + 0xe4
    } else {
        process + 4
    };
    let hour = e.call(0x0086_7da0, &args![0x011d_e7b8u32]).f64();
    let package_hour = e.call(0x0062_1b00, &args![package_slot]).f64();
    let difference = (hour - package_hour) as f32;
    let wrapped = e.call(0x0040_8840, &args![difference]).f32();
    let limit = e.global::<f64>(0x0107_0c80);
    if f64::from(wrapped) >= limit
        || wrapped.is_nan()
        || (e.mem.u32(process + 0x40) != 0 && wanted <= 0)
    {
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
    }
}

// Translated from 008db810 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessObserveCombat` (Xbox PDB): the observe-combat package
/// (type 0x18 of the package `+0x27c`) makes `actor` watch a fight. Either
/// it is done at once (`008d0300` or no live combatant, `actor + 0xa4`
/// virtual `+8` not above 0): then the spectator threat array is built
/// (`009f8660`); with threats the actor's interrupt package ends, the actor
/// is told to search for the first threat (virtual `+0x410`) and, when its
/// current package is a flee package (`__RTDynamicCast`), the threats are
/// added as avoided references (`009f1310`), and the call ends. Without
/// threats, or otherwise, the package may end the observe (`009f7fe0`) by
/// ending the interrupt package; else the actor flees from the observed
/// fight: with a pathing result the hide request (`PathingRequestHide`)
/// is built (radius from `00507e10` + the double at `0102e430`, `0101e6ec`,
/// the pathing location), `SetPathfindingFlee` (`008bb630`) is tried, the
/// move mode becomes 0x200 and, when the distance from the last location is
/// at most 64, the actor is sent running (virtual `+0x350`, mode 0x101).
/// Two settings reads whose result the code never uses are kept (the
/// distance is compared with them but the outcome is dead).
pub fn high_process_process_observe_combat(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    let mut observe = 0u32;
    if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x18 {
        observe = package;
    }
    if observe == 0 {
        return;
    }
    let manager = e.global::<u32>(0x011f_1958);
    let build_threats = e.call(0x008d_0300, &args![manager, actor, 0u32]).bool()
        || e.vcall(a + 0xa4, 8, &args![1u32]).i32() < 1;
    if build_threats {
        let finished = e.with_stack(0x20, |e, array| {
            e.call(0x008c_1c00, &args![array]);
            e.call(0x009f_8660, &args![observe, array]);
            let count = e.call(0x0044_ddc0, &args![array]).i32();
            let mut finished = false;
            if count > 0 {
                e.call(ACTOR_END_INTERRUPT_PACKAGE, &args![actor, 0u32]);
                let first_slot = e.call(0x006a_7ad0, &args![array, 0u32]).u32();
                let first = e.mem.u32(first_slot);
                let search_value = e.global::<f32>(0x0101_2054);
                e.vcall(
                    a,
                    ACTOR_SLOT_0X410,
                    &args![
                        first,
                        1u32,
                        1u32,
                        1u32,
                        0u32,
                        0u32,
                        search_value,
                        search_value
                    ],
                );
                let current = e.call(ACTOR_CURRENT_PACKAGE, &args![actor]).u32();
                let flee_package = e
                    .call(
                        RT_DYNAMIC_CAST,
                        &args![current, 0u32, TYPE_PACKAGE, TYPE_FLEE_PACKAGE, 0u32],
                    )
                    .u32();
                if flee_package != 0 {
                    for index in 0..count {
                        let slot = e.call(0x006a_7ad0, &args![array, index]).u32();
                        let threat = e.mem.u32(slot);
                        e.call(0x009f_1310, &args![flee_package, threat]);
                    }
                }
                finished = true;
            }
            e.call(0x008c_1cb0, &args![array]);
            finished
        });
        if finished {
            return;
        }
    }
    if e.call(0x009f_7fe0, &args![observe]).bool() {
        e.call(ACTOR_END_INTERRUPT_PACKAGE, &args![actor, 0u32]);
        return;
    }
    e.with_stack(0x200, |e, frame| {
        let base = frame.addr();
        let point_a = base;
        let point_b = base + 0x10;
        let point_c = base + 0x20;
        let hide = base + 0x40;
        let location = base + 0x140;
        let mut mode = 0x201u32;
        e.call(0x009f_8300, &args![observe, point_a]);
        let distance = e.call(0x0057_2380, &args![actor, point_a]).f32();
        let _ = distance;
        let cell = e.call(0x008d_6f30, &args![actor]).u32();
        if cell != 0 && e.call(0x0042_5fd0, &args![cell]).bool() {
            e.call(SETTING_VALUE, &args![0x011c_d814u32]);
            e.call(SETTING_VALUE, &args![0x011c_ddb8u32]);
        } else {
            e.call(SETTING_VALUE, &args![0x011c_d990u32]);
            e.call(SETTING_VALUE, &args![0x011c_dfe4u32]);
        }
        e.call(0x009f_8020, &args![observe, actor]);
        if e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
            e.call(0x009f_7f20, &args![observe, actor, 1u32]);
            e.call(0x009f_7d70, &args![observe, actor, 1u32]);
            let now = e.call(0x0082_5c00, &args![FRAME_TIMER]).u32();
            let limit = e
                .call(0x004f_b070, &args![observe])
                .u32()
                .wrapping_add(0x5dc);
            if now > limit {
                e.call(0x008a_c890, &args![observe, now]);
                // The position the virtual `+0x1f4` returns is copied to a
                // local the code never reads again.
                e.vcall(a, ACTOR_SLOT_POSITION, &args![]);
                e.call(0x006e_5510, &args![hide]);
                e.call(0x006e_29f0, &args![hide, actor]);
                if e.vcall(a, ACTOR_SLOT_IS_EXPLOSION, &args![]).bool() {
                    let form = e.call(0x0041_81e0, &args![actor]).u32();
                    let flag = e.vcall(form + 0x30, 0x28, &args![]).u8();
                    e.call(0x006e_2b50, &args![hide, flag as u32]);
                }
                let position = e.call(0x009f_8300, &args![observe, point_b]).u32();
                let pathing_location = e.call(0x006d_ce10, &args![location, position, actor]).u32();
                fn_008dbda0(e, Ptr::new(hide), Ptr::new(pathing_location));
                e.call(0x004f_f7e0, &args![location]);
                let size = e.call(0x0050_7e10, &args![observe]).f64();
                let radius = (size + e.global::<f64>(0x0102_e430)) as f32;
                fn_008dbd80(e, Ptr::new(hide), radius);
                let flee_distance = e.global::<f32>(0x0101_e6ec);
                e.call(0x006d_3b00, &args![hide, flee_distance]);
                let flee_value = e.call(0x009f_8570, &args![observe]).u32();
                e.call(0x006d_61e0, &args![hide, flee_value]);
                if !e.call(0x008b_b630, &args![actor, hide]).bool() {
                    e.call(0x006e_55e0, &args![hide]);
                    return;
                }
                if e.call(0x0089_4d60, &args![actor]).bool() {
                    e.call(0x0089_4cc0, &args![actor, 0u32]);
                }
                e.call(ACTOR_SET_MOVE_MODE, &args![actor, 0x200u32]);
                // Actor +0x190 (Xbox PDB: +0x1a0) holds the object whose
                // `009de2b0` fills the point.
                let holder = e.mem.u32(a + 0x190);
                if e.call(0x009d_e2b0, &args![holder, point_c]).bool() {
                    let remaining = e.call(0x0057_2380, &args![actor, point_c]).f32();
                    if f64::from(remaining) <= e.global::<f64>(0x0102_40c0) {
                        mode = 0x101;
                    }
                    e.vcall(
                        process,
                        SLOT_SET_ACTORS_ANIMATION,
                        &args![actor, mode, 1u32],
                    );
                }
                e.call(0x006e_55e0, &args![hide]);
            }
        } else if e.vcall(a, ACTOR_SLOT_IS_CREATURE, &args![]).bool() {
            e.call(0x009f_7f20, &args![observe, actor, 1u32]);
            e.call(0x009f_7d70, &args![observe, actor, 1u32]);
        }
        e.call(0x008b_3b90, &args![actor]);
    });
}

// Translated from 008dbd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: stores the float at +0xd8 of `this` (the
/// `PathingRequestHide` that `ProcessObserveCombat` builds: its radius).
pub fn fn_008dbd80(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0xd8, value);
}

// Translated from 008dbda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: assigns the `PathingLocation` `location` to the
/// one at +0xb0 of `this` (`PathingLocation::operator=`, `006dcce0`).
pub fn fn_008dbda0(e: &mut Engine, this: Ptr, location: Ptr) {
    e.call(0x006d_cce0, &args![this.addr() + 0xb0, location]);
}

// Translated from 008dbdc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ClearGreetingInfoData` (Xbox PDB): deletes `pGreetTopic`
/// (+0x368, `005c90d0` with the delete flag) when set and clears it,
/// `pSubtitleVoice` (+0x454) and `m_pSayToDialogueTopic` (+0x3e4).
pub fn high_process_clear_greeting_info_data(e: &mut Engine, this: Ptr<HighProcess>) {
    let topic = e.get(this, HighProcess::pGreetTopic);
    if !topic.is_null() {
        e.call(0x005c_90d0, &args![topic, 1u32]);
    }
    e.set(this, HighProcess::pGreetTopic, Ptr::NULL);
    e.set(this, HighProcess::pSubtitleVoice, Ptr::NULL);
    e.set(this, HighProcess::m_pSayToDialogueTopic, Ptr::NULL);
}

// Translated from 008dd880 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: sets the greet flag (+0x6cc, see
/// `PlayerCharacter::ResetPlayerGreetFlag`) of the `PlayerCharacter` in `this`.
pub fn fn_008dd880(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 0x6cc, 1);
}

/// The values `ProcessGreet` shares between its blocks.
struct Greet {
    /// The process (`this`).
    process: u32,
    /// The actor the process belongs to (first argument).
    actor: u32,
    /// The topic (second argument; the continued greeting replaces it).
    topic: u32,
    /// Fifth argument: stored in `bStop`.
    stop: u8,
    /// Sixth argument: with a greeting topic held, keep it instead of
    /// creating a new dialogue item (only honoured when no lip data waits).
    reuse_held_topic: u8,
    /// Seventh argument: this is a say-to greeting.
    say_to: u8,
    /// Starts 1; 0 when the current package is a script
    /// package (`005f36f0`) or the run-once package is of type 0x1a.
    can_greet: u8,
    /// `00493bb0` of the actor.
    creature_like: bool,
    /// The dialog target (`004fd380`, else `pDialogTarget`).
    dialog: u32,
}

/// Greet timer at `011cd450` in the game settings.
const GREET_TIMER_SETTING: u32 = 0x011c_d450;
/// Sound completion callbacks `ProcessGreet` gives the sound handle.
const SOUND_CALLBACK_SAY_TO: u32 = 0x0093_6a20;
const SOUND_CALLBACK_PLAIN: u32 = 0x0093_5cc0;

/// Copies the greet timer setting (`011cd450`) into `fGreetingTimer`
/// (+0x330).
fn greet_reset_greeting_timer(e: &mut Engine, process: u32) {
    let setting = e.call(SETTING_VALUE, &args![GREET_TIMER_SETTING]).u32();
    let value = e.mem.f32(setting);
    e.mem.set_f32(process + 0x330, value);
}

/// The last greeted actor of the dialog target's process becomes `actor`
/// (virtual `+0x488`) when the target is not an `00440da0` one.
fn greet_notify_dialog_target(e: &mut Engine, g: &Greet) {
    if g.dialog != 0 && !e.call(0x0044_0da0, &args![g.dialog]).bool() {
        let target_process = e.call(ACTOR_PROCESS, &args![g.dialog]).u32();
        e.vcall(target_process, SLOT_SET_LAST_GREETED, &args![g.actor]);
    }
}

/// Starts the special idles for the greeting's response `response`: the
/// actor's (type word `009611e0`) and the dialog target's (`00441110`).
fn greet_play_idles(e: &mut Engine, g: &Greet, response: u32) {
    let animation = e.vcall(g.actor, ACTOR_SLOT_ANIMATION, &args![]).u32();
    if animation != 0 {
        let idle = e.call(0x0096_11e0, &args![response]).u32();
        if idle != 0 {
            e.vcall(
                g.process,
                SLOT_SETUP_SPECIAL_IDLE,
                &args![g.actor, idle, 3u32, 0u32, 1u32, 1u32],
            );
        }
    }
    if g.dialog != 0
        && g.dialog != g.actor
        && (e
            .vcall(g.dialog, ACTOR_SLOT_IS_MOBILE_OBJECT, &args![])
            .bool()
            || e.vcall(g.dialog, ACTOR_SLOT_IS_ACTOR, &args![]).bool())
    {
        let idle = e.call(0x0044_1110, &args![response]).u32();
        if idle != 0 {
            let target_process = e.call(ACTOR_PROCESS, &args![g.dialog]).u32();
            e.vcall(
                target_process,
                SLOT_SETUP_SPECIAL_IDLE,
                &args![g.dialog, idle, 3u32, 0u32, 1u32, 1u32],
            );
        }
    }
}

/// Starts the voice file of the response (`00933150` into a temporary sound
/// handle that is assigned to the process' handle at +0x314, `00418900`).
fn greet_start_voice(e: &mut Engine, g: &Greet, response: u32) {
    let form = e.call(0x007a_f430, &args![response]).u32();
    let name = e.call(0x0084_e3a0, &args![form]).u32();
    e.with_stack(0x10, |e, handle| {
        let started = e
            .call(
                0x0093_3150,
                &args![g.actor, handle, name, 0u32, 0x4000_0102u32, 1u32],
            )
            .u32();
        e.call(0x0041_8900, &args![g.process + 0x314, started]);
        e.call(SOUND_HANDLE_DESTRUCTOR, &args![handle]);
    });
}

/// Sets the completion callback of the process' sound handle (`+0x314`) to
/// `callback` with the actor's name word (`0084e3a0`).
fn greet_set_sound_callback(e: &mut Engine, g: &Greet, callback: u32) {
    let name = e.call(0x0084_e3a0, &args![g.actor]).u32();
    e.call(0x00ad_8e60, &args![g.process + 0x314, callback, name]);
}

/// The calls the first and the continued greeting make to say the response:
/// the actor's virtual `+0x284` with twelve words; returns the delay it gives
/// (an `f32`).
fn greet_speak(e: &mut Engine, g: &Greet, response: u32) -> f32 {
    let idle = e.call(0x0044_1110, &args![response]).u32();
    let kind = e.call(0x0096_11e0, &args![response]).u32();
    let node = e.call(NODE_ITEM_ADDRESS, &args![response]).u32();
    let magic = e.call(0x0040_48e0, &args![node]).u32();
    let name = e.call(0x0084_e3a0, &args![response]).u32();
    let count = e.call(0x0044_ddc0, &args![response]).u32();
    let holder = e
        .call(0x0046_0140, &args![response, g.process + 0x314])
        .u32();
    let held = e.call(NI_POINTER_GET, &args![holder]).u32();
    e.vcall(
        g.actor,
        ACTOR_SLOT_SAY,
        &args![
            held,
            count,
            name,
            magic,
            kind,
            idle,
            g.dialog,
            1u32,
            0u32,
            1u32,
            1u32,
            g.can_greet as u32
        ],
    )
    .f32()
}

/// `00543c30` of the response and `008a5cf0` of the actor with it.
fn greet_mark_response(e: &mut Engine, g: &Greet, response: u32) {
    let flag = e.call(0x0054_3c30, &args![response]).u8();
    e.call(0x008a_5cf0, &args![g.actor, flag as u32]);
}

/// The block that tells the engine nobody answers: with a say-to greeting the
/// action flag is set (`005ac750`), a current package of type 0xf that
/// `00672710` accepts is completed (virtual `+0x118`, then index +3), the
/// greeting timer is reset and the greeting flag is cleared.
fn greet_no_response(e: &mut Engine, g: &Greet) {
    let t = g.process;
    if g.say_to != 0 {
        let extra = e.call(0x005d_43c0, &args![g.actor]).u32();
        e.call(0x005a_c750, &args![g.topic, extra, 0x40000u32]);
    }
    let package = e.vcall(t, SLOT_GET_CURRENT_PACKAGE, &args![]).u32();
    if package != 0
        && e.call(PACKAGE_TYPE, &args![package]).u32() == 0xf
        && e.call(0x0067_2710, &args![package]).bool()
    {
        e.vcall(t, 0x118, &args![1u32]);
        e.vcall(
            t,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![g.actor, 3u32],
        );
    }
    greet_reset_greeting_timer(e, t);
    e.vcall(t, SLOT_SET_GREETING_FLAG, &args![0u32]);
}

/// The end of the new-topic branches: the greeting timer is reset from the
/// settings, the dialog target becomes the action head-track target and, when
/// the actor's state is 0 and `008a40e0` accepts the target, the process is
/// told to force-rotate.
fn greet_tail(e: &mut Engine, g: &Greet) {
    let t = g.process;
    if g.dialog != 0 {
        greet_reset_greeting_timer(e, t);
    }
    e.vcall(t, SLOT_SET_EXTRA_HEAD_TRACK, &args![g.dialog]);
    if actor_state(e, g.actor) == 0
        && g.dialog != 0
        && e.call(0x008a_40e0, &args![g.actor, g.dialog]).bool()
    {
        e.vcall(t, SLOT_SET_FORCE_ROTATE, &args![1u32]);
    }
}

/// The first-greeting and continued-greeting response code (`008dc0d4..` and
/// `008dd0dc..`): `response` is the current response. Starts the idles and
/// the voice when the response has a form (`007af430`) and the actor
/// has a voice file waiting (`+0x7f`), says it, and releases the say-to topic
/// data. `counter_mark` selects the continued greeting's mark word
/// (`008dd860` of the actor, plus 1) instead of the plain 1.
fn greet_respond(e: &mut Engine, g: &Greet, response: u32, counter_mark: bool) {
    let t = g.process;
    let a = g.actor;
    // Actor::bSoundFileDone (Xbox PDB +0x8f) +0x7f
    let mut waiting = true;
    if e.call(0x007a_f430, &args![response]).u32() != 0 && e.mem.u8(a + 0x7f) != 0 {
        greet_play_idles(e, g, response);
        greet_start_voice(e, g, response);
        if g.say_to != 0 {
            e.call(0x0057_ad20, &args![g.actor, g.topic]);
            let current = e.mem.u32(t + 0x3e4);
            let id = e.call(0x0084_e3a0, &args![current]).u32();
            e.call(0x0057_ace0, &args![g.actor, id]);
            let mark = if counter_mark {
                e.call(0x008d_d860, &args![g.actor]).u32().wrapping_add(1)
            } else {
                1
            };
            e.call(0x0057_ad60, &args![g.actor, mark]);
            greet_set_sound_callback(e, g, SOUND_CALLBACK_SAY_TO);
            e.mem.set_u8(t + 0x29d, 0);
        } else {
            greet_set_sound_callback(e, g, SOUND_CALLBACK_PLAIN);
        }
        e.mem.set_u8(a + 0x7f, 0);
        waiting = false;
    }
    if e.mem.u8(t + 0x3a0) != 0 && !g.creature_like {
        e.vcall(t, SLOT_END_MOVE_MESSAGE, &args![g.actor]);
    }
    if waiting && e.mem.u8(a + 0x7f) != 0 {
        greet_mark_response(e, g, response);
        let delay = greet_speak(e, g, response);
        e.mem.set_f32(t + 0x310, delay);
        e.mem.set_u8(t + 0x32c, 1);
        if e.mem.u8(t + 0x29d) != 0 && e.mem.u8(t + 0x3d0) == 0 {
            greet_set_sound_callback(e, g, SOUND_CALLBACK_SAY_TO);
            e.call(0x0057_ad20, &args![g.actor, g.topic]);
            let current = e.mem.u32(t + 0x3e4);
            let id = e.call(0x0084_e3a0, &args![current]).u32();
            e.call(0x0057_ace0, &args![g.actor, id]);
            let mark = if counter_mark {
                e.call(0x008d_d860, &args![g.actor]).u32().wrapping_add(1)
            } else {
                1
            };
            e.call(0x0057_ad60, &args![g.actor, mark]);
            e.mem.set_u8(t + 0x29d, 0);
            e.mem.set_u8(a + 0x80, 0);
        }
    }
    let held = e.mem.u32(t + 0x368);
    if held != 0 && e.mem.u8(t + 0x3d0) == 0 {
        e.call(0x0083_c850, &args![held, 0u32]);
    }
}

/// Ends the movement of the actor (`0087faa0`) when the running package's
/// current procedure maps to 1 in the procedure table (`011a3ff0`).
fn greet_end_movement_if_needed(e: &mut Engine, g: &Greet, package: u32) {
    if !g.creature_like && package != 0 {
        let kind = e.call(0x0096_11e0, &args![package]).u32();
        let procedure = e.vcall(g.process, 0x280, &args![]).u32();
        let table = e.mem.u32(0x011a_3ff0 + kind.wrapping_mul(4));
        if e.mem.u32(table.wrapping_add(procedure.wrapping_mul(4))) == 1 {
            e.call(0x0087_faa0, &args![g.actor]);
        }
    }
}

/// The keep-the-running-greeting branch of `ProcessGreet` (`008dc642..`): a
/// greeting topic is held (`pGreetTopic`) and either the caller keeps it
/// running or a voice file waits. Returns through `done` the "greeting is
/// finished" flag.
fn greet_continue_running(e: &mut Engine, g: &Greet, done: &mut bool) {
    let t = g.process;
    let a = g.actor;
    e.call(0x0093_4250, &args![g.actor]);
    if g.reuse_held_topic == 0 {
        if e.mem.u32(t + 0x368) != 0 {
            let held = e.mem.u32(t + 0x368);
            if held != 0 {
                e.call(0x005c_90d0, &args![held, 1u32]);
            }
            e.mem.set_u32(t + 0x368, 0);
            e.mem.set_u32(t + 0x454, 0);
        }
        let item = e
            .call(
                0x0061_b320,
                &args![g.topic, g.actor, g.dialog, 0u32, 0u32, 1u32],
            )
            .u32();
        e.mem.set_u32(t + 0x3e4, item);
        if e.mem.u32(t + 0x3e4) != 0 {
            let item = e.mem.u32(t + 0x3e4);
            e.call(0x0083_c7b0, &args![item]);
            e.mem.set_u32(t + 0x368, item);
            e.mem.set_u32(t + 0x454, item);
        } else {
            greet_no_response(e, g);
        }
    } else {
        let held = e.mem.u32(t + 0x368);
        e.mem.set_u32(t + 0x3e4, held);
    }
    if e.mem.u32(t + 0x368) != 0 {
        let held = e.mem.u32(t + 0x368);
        e.call(0x0083_c850, &args![held, 0u32]);
    }
    if e.mem.u32(t + 0x3e4) == 0 {
        if g.say_to != 0 {
            let extra = e.call(0x005d_43c0, &args![g.actor]).u32();
            e.call(0x005a_c750, &args![g.topic, extra, 0x40000u32]);
            e.call(0x0057_ad20, &args![g.actor, 0u32]);
            let extra = e.call(0x005d_43c0, &args![g.actor]).u32();
            e.call(0x0042_edb0, &args![extra]);
            e.vcall(t, SLOT_SET_DOING_SAY_TO, &args![0u32]);
        }
        return;
    }
    greet_notify_dialog_target(e, g);
    let item = e.mem.u32(t + 0x3e4);
    let response = e.call(0x0083_c820, &args![item]).u32();
    if response == 0 {
        return;
    }
    let mut waiting = true;
    if e.call(0x007a_f430, &args![response]).u32() != 0 && e.mem.u8(a + 0x7f) != 0 {
        let form = e.call(0x007a_f430, &args![response]).u32();
        let name = e.call(0x0084_e3a0, &args![form]).u32();
        e.with_stack(0x10, |e, handle| {
            e.call(
                0x0093_3150,
                &args![g.actor, handle, name, 0u32, 0x4000_0102u32, 1u32],
            );
            // BSAudioManager::QInstance (Xbox PDB); the result is unused.
            e.call(0x00ad_9060, &args![]);
            if g.say_to != 0 {
                e.call(0x0057_ad20, &args![g.actor, g.topic]);
                let current = e.mem.u32(t + 0x3e4);
                let id = e.call(0x0084_e3a0, &args![current]).u32();
                e.call(0x0057_ace0, &args![g.actor, id]);
                let mark = e.call(0x0045_cd60, &args![response]).u32();
                e.call(0x0057_ad60, &args![g.actor, mark]);
                greet_set_sound_callback(e, g, SOUND_CALLBACK_SAY_TO);
                e.mem.set_u8(t + 0x29d, 0);
            } else {
                greet_set_sound_callback(e, g, SOUND_CALLBACK_PLAIN);
            }
            e.mem.set_u8(a + 0x7f, 0);
            waiting = false;
            let extra = e.call(0x005d_43c0, &args![g.actor]).u32();
            e.call(0x005a_c750, &args![g.topic, extra, 0x40000u32]);
            e.call(0x0057_ad20, &args![g.actor, 0u32]);
            let extra = e.call(0x005d_43c0, &args![g.actor]).u32();
            e.call(0x0042_edb0, &args![extra]);
            e.vcall(t, SLOT_SET_DOING_SAY_TO, &args![0u32]);
            e.call(SOUND_HANDLE_DESTRUCTOR, &args![handle]);
        });
    }
    greet_mark_response(e, g, response);
    if waiting && e.mem.u8(a + 0x7f) != 0 {
        let delay = greet_speak(e, g, response);
        e.mem.set_f32(t + 0x310, delay);
        if e.mem.u8(t + 0x29d) != 0 {
            // A `NiPointer` read whose result the code does not use.
            e.call(NI_POINTER_GET, &args![t + 0x314]);
            greet_set_sound_callback(e, g, SOUND_CALLBACK_SAY_TO);
            let current = e.mem.u32(t + 0x3e4);
            let procedure = e.call(0x0044_edb0, &args![current]).u32();
            e.call(0x0057_ad20, &args![g.actor, procedure]);
            let current = e.mem.u32(t + 0x3e4);
            let id = e.call(0x0084_e3a0, &args![current]).u32();
            e.call(0x0057_ace0, &args![g.actor, id]);
            let mark = e.call(0x0045_cd60, &args![response]).u32();
            e.call(0x0057_ad60, &args![g.actor, mark]);
            e.mem.set_u8(t + 0x29d, 0);
            e.mem.set_u8(a + 0x80, 0);
        }
    }
    if g.can_greet != 0 {
        e.vcall(
            t,
            SLOT_SETUP_SPECIAL_IDLE,
            &args![g.actor, 0u32, 2u32, 1u32, 0u32, 1u32],
        );
    }
    let timer = e.mem.f32(t + 0x310);
    if f64::from(timer) > e.global::<f64>(0x0101_2060) || timer.is_nan() {
        if e.mem.u8(t + 0x3a0) != 0 && !g.creature_like {
            e.vcall(t, SLOT_END_MOVE_MESSAGE, &args![g.actor]);
        }
        e.mem.set_u8(t + 0x32c, 1);
    } else {
        *done = true;
    }
    let package = e.vcall(t, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![]).u32();
    greet_end_movement_if_needed(e, g, package);
    e.vcall(t, SLOT_SET_EXTRA_HEAD_TRACK, &args![g.dialog]);
    if actor_state(e, g.actor) == 0
        && g.dialog != 0
        && e.call(0x008a_40e0, &args![g.actor, g.dialog]).bool()
    {
        e.vcall(t, SLOT_SET_FORCE_ROTATE, &args![1u32]);
    }
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
    let delay = e.mem.f32(t + 0x310);
    e.mem.set_f32(t + 0x310, (f64::from(delay) - frame) as f32);
    if g.dialog != 0 {
        greet_reset_greeting_timer(e, t);
    }
}

/// The new-greeting branch (`008dbfeb..`): creates the dialogue item of the
/// topic (`0061b320`), keeps it as `pGreetTopic` / `pSubtitleVoice` /
/// `m_pSayToDialogueTopic`, and says its first response.
fn greet_new_greeting(e: &mut Engine, g: &Greet) {
    let t = g.process;
    let item = e
        .call(
            0x0061_b320,
            &args![g.topic, g.actor, g.dialog, 0u32, 0u32, 1u32],
        )
        .u32();
    e.mem.set_u32(t + 0x3e4, item);
    if e.mem.u32(t + 0x368) != 0 {
        let held = e.mem.u32(t + 0x368);
        if held != 0 {
            e.call(0x005c_90d0, &args![held, 1u32]);
        }
        e.mem.set_u32(t + 0x454, 0);
        e.mem.set_u32(t + 0x368, 0);
    }
    let item = e.mem.u32(t + 0x3e4);
    e.mem.set_u32(t + 0x368, item);
    let item = e.mem.u32(t + 0x3e4);
    e.mem.set_u32(t + 0x454, item);
    e.mem.set_u32(t + 0x370, g.dialog);
    if e.mem.u32(t + 0x3e4) == 0 {
        greet_no_response(e, g);
        return;
    }
    greet_notify_dialog_target(e, g);
    let item = e.mem.u32(t + 0x3e4);
    e.call(0x0083_c7b0, &args![item]);
    let item = e.mem.u32(t + 0x3e4);
    let response = e.call(0x0083_c820, &args![item]).u32();
    if response != 0 {
        greet_respond(e, g, response, false);
    }
    greet_tail(e, g);
}

/// Package types (in the order the code tests them) that stop the idle branch of
/// `ProcessGreet` from turning the actor towards the dialog target.
const GREET_PACKAGE_TYPES_THAT_KEEP_FACING: [u32; 9] = [6, 2, 1, 7, 0xd, 8, 0x10, 3, 0xc];

/// The idle branch (`008dcd28..`): no new topic and nothing in the lip file
/// queue. Counts the greeting delay down (or marks the greeting finished when
/// it ran out) and keeps the actor facing the dialog target.
fn greet_idle(e: &mut Engine, g: &Greet, done: &mut bool) {
    let t = g.process;
    let a = g.actor;
    let timer = e.mem.f32(t + 0x310);
    if f64::from(timer) > e.global::<f64>(0x0101_2060) {
        let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
        let timer = e.mem.f32(t + 0x310);
        e.mem.set_f32(t + 0x310, (f64::from(timer) - frame) as f32);
    } else {
        *done = true;
    }
    if g.dialog == 0 {
        return;
    }
    let package = e.vcall(t, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![]).u32();
    greet_end_movement_if_needed(e, g, package);
    // A process that is forced to rotate (virtual `+0x688`) always turns;
    // otherwise `stop` must be set and the actor must be free to turn: not a
    // creature, in state 0, not pathing, and not running a package of one of
    // the types in [`GREET_PACKAGE_TYPES_THAT_KEEP_FACING`].
    let rotate = e.vcall(t, 0x688, &args![]).bool()
        || !(g.stop == 0
            || !e.call(0x008a_40e0, &args![a, 0u32]).bool()
            || g.creature_like
            || actor_state(e, a) != 0
            || e.call(0x008a_3b30, &args![a]).bool()
            || e.call(0x008b_3bd0, &args![a]).bool()
            || (package != 0
                && GREET_PACKAGE_TYPES_THAT_KEEP_FACING
                    .iter()
                    .any(|&kind| e.call(PACKAGE_TYPE, &args![package]).u32() == kind)));
    if rotate && e.call(0x008a_40e0, &args![a, 0u32]).bool() {
        e.vcall(t, SLOT_SET_EXTRA_HEAD_TRACK, &args![g.dialog]);
        let position = e.vcall(g.dialog, ACTOR_SLOT_POSITION, &args![0u32]).u32();
        let x = e.mem.u32(position);
        let y = e.mem.u32(position + 4);
        let z = e.mem.u32(position + 8);
        e.call(0x008b_b520, &args![g.actor, x, y, z]);
    }
}

/// The finished-greeting branch (`008dcfd5..`): moves on to the next response
/// of the held dialogue item (`0083c7e0`) and says it, or when there is none
/// clears the greeting (`008dd623..`).
fn greet_next_response(e: &mut Engine, g: &mut Greet) {
    let t = g.process;
    let a = g.actor;
    let item = e.mem.u32(t + 0x3e4);
    if item == 0 || !e.call(0x0083_c7e0, &args![item]).bool() {
        // 008dd623: nothing left to say.
        e.mem.set_u32(t + 0x3e4, 0);
        if e.mem.u32(t + 0x368) != 0 {
            let held = e.mem.u32(t + 0x368);
            if held != 0 {
                e.call(0x005c_90d0, &args![held, 1u32]);
            }
        }
        e.mem.set_u32(t + 0x368, 0);
        e.mem.set_u32(t + 0x370, 0);
        e.vcall(t, 0x494, &args![0u32]);
        e.mem.set_u8(t + 0x3a0, 0);
        e.mem.set_u8(t + 0x32c, 0);
        let package = e.vcall(t, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![]).u32();
        if package != 0
            && e.call(PACKAGE_TYPE, &args![package]).u32() == 0xf
            && e.mem.u32(t + 0x40) == player_pointer(e)
            && e.call(0x0067_2710, &args![package]).bool()
        {
            e.vcall(
                t,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![g.actor, 3u32],
            );
        }
        let player = player_pointer(e);
        e.call(0x0095_3ce0, &args![player]);
        let talker = e.call(0x008a_2ed0, &args![g.actor]).u32();
        let mut talker_actor = 0u32;
        if talker != 0 && e.vcall(talker, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
            talker_actor = talker;
        }
        if talker_actor != 0
            && e.call(ACTOR_PROCESS, &args![talker_actor]).u32() != 0
            && talker_actor != player_pointer(e)
        {
            let target_process = e.call(ACTOR_PROCESS, &args![talker_actor]).u32();
            e.vcall(target_process, SLOT_SET_LAST_GREETED, &args![0u32]);
            let target_process = e.call(ACTOR_PROCESS, &args![talker_actor]).u32();
            e.vcall(target_process, SLOT_CLEAR_ACTION_HEAD_TRACK, &args![1u32]);
        }
        e.vcall(t, SLOT_CLEAR_ACTION_HEAD_TRACK, &args![1u32]);
        return;
    }
    let item = e.mem.u32(t + 0x3e4);
    let response = e.call(0x0083_c820, &args![item]).u32();
    if response == 0 {
        return;
    }
    let held = e.mem.u32(t + 0x3e4);
    e.mem.set_u32(t + 0x368, held);
    let held = e.mem.u32(t + 0x3e4);
    e.mem.set_u32(t + 0x454, held);
    e.mem.set_u32(t + 0x370, g.dialog);
    e.mem.set_u8(t + 0x29d, 1);
    let item = e.mem.u32(t + 0x3e4);
    g.topic = e.call(0x0044_edb0, &args![item]).u32();
    if e.mem.u32(t + 0x3e4) == 0 {
        // 008dd551: no item (not reachable in practice).
        greet_no_response(e, g);
        return;
    }
    greet_notify_dialog_target(e, g);
    greet_respond(e, g, response, true);
    // 008dd4cf: the head-track tail without the force-rotate test's
    // `dialog` guard on the timer.
    greet_tail(e, g);
    let _ = a;
}

// Translated from 008dbe30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessGreet` (Xbox PDB): the greeting step of a package
/// (called with the actor, the topic to say or null, an unused word, `stop`
/// stored in `bStop`, `reuse_held_topic` and `say_to`). Does nothing more once a
/// lip file is being waited for (`bWaitingForLipFile`). With a topic, a
/// queued lip animation or a failed lip file it starts or continues the
/// dialogue item: `greet_continue_running` when a greeting topic is held
/// (`pGreetTopic`) and `reuse_held_topic` is 0 or lip data waits (it replaces
/// the held item with a new one unless `reuse_held_topic` is set),
/// `greet_new_greeting` otherwise; with none of them it counts the delay down
/// (`greet_idle`). When the delay ran out and the voice has finished
/// (`BSSoundHandle::IsPlaying`) it moves to the next response or clears the
/// greeting. Ends with `0057bd60(actor, 0)` except when it left at the
/// lip-file test. C++ exception states are not translated.
#[allow(clippy::too_many_arguments)]
pub fn high_process_process_greet(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    topic: Ptr,
    _unused_3: u32,
    stop: u8,
    reuse_held_topic: u8,
    say_to: u8,
) {
    let t = this.addr();
    let a = actor.addr();
    let mut done = false;
    e.mem.set_u8(t + 0x3a0, stop);
    let mut g = Greet {
        process: t,
        actor: a,
        topic: topic.addr(),
        stop,
        reuse_held_topic,
        say_to,
        can_greet: 1,
        creature_like: false,
        dialog: 0,
    };
    if e.call(0x0049_3bb0, &args![actor]).bool() {
        g.creature_like = true;
    }
    if say_to != 0 {
        e.mem.set_u8(t + 0x459, 1);
    }
    let current = e.vcall(t, SLOT_GET_CURRENT_PACKAGE, &args![]).u32();
    if current != 0 && e.call(0x005f_36f0, &args![current]).u32() != 0 {
        g.can_greet = 0;
    }
    g.dialog = e.call(0x004f_d380, &args![actor]).u32();
    if e.vcall(t, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32() != 0 {
        let run_once = e.vcall(t, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32();
        if e.call(PACKAGE_TYPE, &args![run_once]).u32() == 0x1a {
            g.can_greet = 0;
        }
    }
    if say_to != 0 {
        e.mem.set_u8(t + 0x29d, 1);
    }
    if g.dialog == 0 {
        g.dialog = e.mem.u32(t + 0x370);
    }
    if g.dialog != 0 {
        let player = player_pointer(e);
        fn_008dd880(e, Ptr::new(player));
    }
    if e.mem.u8(t + 0x3d0) != 0 {
        return;
    }
    let new_topic = g.topic != 0
        || (e.mem.u8(t + 0x3d0) == 0 && e.mem.u32(t + 0x3cc) != 0)
        || e.mem.u8(t + 0x3d1) != 0;
    if new_topic {
        if e.mem.u32(t + 0x368) != 0
            && (reuse_held_topic == 0 || e.mem.u32(t + 0x3cc) != 0 || e.mem.u8(t + 0x3d1) != 0)
        {
            greet_continue_running(e, &g, &mut done);
        } else {
            greet_new_greeting(e, &g);
        }
    } else {
        greet_idle(e, &g, &mut done);
    }
    if done && e.mem.u8(t + 0x3d0) == 0 && !e.call(0x00ad_8930, &args![t + 0x314]).bool() {
        greet_next_response(e, &mut g);
    }
    e.call(0x0057_bd60, &args![actor, 0u32]);
}

// ---- Second session: `008dd8e0` onwards ----

/// `LowProcess::pTarget` (Xbox PDB) +0x40: the package's target reference.
const LOW_TARGET: u32 = 0x40;
/// `Actor::SetAlert` (`008a5e40`, one argument).
const ACTOR_SET_ALERT: u32 = 0x008a_5e40;
/// Animation: the special idle has finished playing (`004985f0`).
const ANIMATION_SPECIAL_IDLE_DONE_PLAYING: u32 = 0x0049_85f0;
/// `HighProcess::FindSpecialIdletoPlay` (`008ff0b0`): `(actor, 0, 0)`.
const FIND_SPECIAL_IDLE_TO_PLAY: u32 = 0x008f_f0b0;
/// A game constant (`01012054`): the float the flee and search code uses for
/// "no limit" (read from the exe).
const NO_LIMIT_FLOAT: u32 = 0x0101_2054;
/// `GetWorldSpace` of the reference in `this` (`00575d70`): no arguments.
const REFERENCE_GET_WORLD_SPACE: u32 = 0x0057_5d70;
/// `008d6f30`: no arguments, called on the reference (kept for the call
/// that follows).
const REFERENCE_PATHING_CELL: u32 = 0x008d_6f30;
/// `Actor::SetPathfindingGoal_ov2` (`008b3690`): `(location, cell, world
/// space, float, int)`.
const ACTOR_SET_PATHFINDING_GOAL: u32 = 0x008b_3690;
/// `SearchPackage::CalculateLocationToExamine` (`009f6900`): `(out, actor)`.
const SEARCH_CALCULATE_LOCATION: u32 = 0x009f_6900;
const SLOT_SET_LOCATION_FOR_PACKAGE: u32 = 0x90;
const SLOT_GET_GENERIC_LOCATION: u32 = 0x514;
const SLOT_SET_CURRENT_ACTION_COMPLETE: u32 = 0x118;
const SLOT_CHECK_FOR_NEW_PACKAGE: u32 = 0x24;
/// Slot of `Actor` called first by `ProcessFleeNonCombat` (returns a flag).
const ACTOR_SLOT_0X358: u32 = 0x358;

// Translated from 008dd8e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessAlertBehavior` (Xbox PDB): with a running package,
/// sets the actor alert, gives the process a target when it has none
/// (`SetTargetForPackage`), starts a special idle when the actor's animation
/// finished playing the last one, and adds procedure 1 to the running ones.
pub fn high_process_process_alert_behavior(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if package == 0 {
        return;
    }
    e.call(ACTOR_SET_ALERT, &args![actor, 1u32]);
    if e.mem.u32(process + LOW_TARGET) == 0 {
        e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
    }
    let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
    if animation != 0
        && e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
            .bool()
    {
        e.call(FIND_SPECIAL_IDLE_TO_PLAY, &args![this, actor, 0u32, 0u32]);
    }
    e.vcall(
        process,
        SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
        &args![actor, 1u32],
    );
}

// Translated from 008dd980 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessSearchforTarget` (Xbox PDB): only runs with a
/// running package; the search package (type 0x1e, `0041ca90`) is kept when
/// that is what runs. Gives the process a target if it has none, sets the
/// actor alert, adds the frame time to `fEvaluateAcquireTimer` (+0x2e0),
/// sets the actor's animation (`SetActorsAnimation(actor, 0x201, 1)`) and,
/// once the actor's pathing is complete, marks the current action complete,
/// asks the search package for the location to examine
/// (`009f6900`) and, if a path to it can be set (`008b3690`), stores it in
/// the three words at +0xfc.
pub fn high_process_process_searchfor_target(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let mut search = 0u32;
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if package != 0 && e.call(PACKAGE_TYPE, &args![package]).i32() == 0x1e {
        search = e
            .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
            .u32();
    } else if package == 0 {
        return;
    }
    if e.mem.u32(process + LOW_TARGET) == 0 {
        e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
    }
    e.call(ACTOR_SET_ALERT, &args![actor, 1u32]);
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
    let timer = (frame + f64::from(e.mem.f32(process + 0x2e0))) as f32;
    e.mem.set_f32(process + 0x2e0, timer);
    e.vcall(
        process,
        SLOT_SET_ACTORS_ANIMATION,
        &args![actor, 0x201u32, 1u32],
    );
    if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
        return;
    }
    e.vcall(process, SLOT_SET_CURRENT_ACTION_COMPLETE, &args![1u32]);
    e.with_stack(12, |e, location| {
        e.call(SEARCH_CALCULATE_LOCATION, &args![search, location, actor]);
        let target = e.mem.u32(process + LOW_TARGET);
        // `GetWorldSpace` and `008d6f30` take no arguments; the two words
        // pushed before them stay on the stack and are the goal's last two
        // arguments.
        let world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![target]).u32();
        let cell = e.call(REFERENCE_PATHING_CELL, &args![target]).u32();
        let set = e
            .call(
                ACTOR_SET_PATHFINDING_GOAL,
                &args![actor, location, cell, world_space, 0.0f32, 0u32],
            )
            .bool();
        if set {
            for word in 0..3 {
                let value = e.mem.u32(location.addr() + word * 4);
                e.mem.set_u32(process + 0xfc + word * 4, value);
            }
        }
    });
}

/// `Package` field +0x30 (`00671d10`): returns the word at +0x30 of `this`.
const PACKAGE_TARGET_WORD: u32 = 0x0067_1d10;
/// `PackageTarget::GetTargReference` (Xbox PDB, `00680020`).
const PACKAGE_TARGET_GET_REFERENCE: u32 = 0x0068_0020;
/// `0044ddc0`: returns the word at +8 of `this` (used as a radius number).
const WORD_AT_8: u32 = 0x0044_ddc0;
/// `00676280`: the package's distance number for an actor (`this`, actor).
const PACKAGE_DISTANCE_FOR_ACTOR: u32 = 0x0067_6280;
/// `FleePackage::AddAvoidedRef` (Xbox PDB, `009f1310`).
const FLEE_PACKAGE_ADD_AVOIDED_REF: u32 = 0x009f_1310;
/// `0067efd0`: bit 2 of the word at +0x1c of `this`.
const PACKAGE_FLAG_4: u32 = 0x0067_efd0;
/// `008b1ff0`: bit 1 of the word at +0x1c of `this`.
const PACKAGE_FLAG_2: u32 = 0x008b_1ff0;

// Translated from 008ddac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessFleeNonCombat` (Xbox PDB): an actor that flees
/// without combat. When the actor's slot `+0x358` says so, only procedure 1
/// is added to the running ones. Otherwise the process gets a target and a
/// location for the package, the package's target reference becomes the
/// process target when there is none, and the flee distance (`-1` without a
/// package target, else the number at +8 of it) is compared with the actor's
/// distance to the target, or the package's distance with the location's:
/// when the actor is not yet far enough it flees (actor slot `+0x410`) and a
/// running flee package (type 0x16) remembers the avoided reference;
/// otherwise it stops its movement and, once pathing is complete and the
/// package has flag 4 or 2, ends the package (current action complete,
/// procedure 1, `CheckforNewPackage`).
pub fn high_process_process_flee_non_combat(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if e.vcall(a, ACTOR_SLOT_0X358, &args![]).bool() {
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
        return;
    }
    if e.mem.u32(process + LOW_TARGET) == 0 {
        e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
    }
    e.vcall(process, SLOT_SET_LOCATION_FOR_PACKAGE, &args![actor, 0u32]);
    let package_target = e.call(PACKAGE_TARGET_WORD, &args![package]).u32();
    if e.mem.u32(process + LOW_TARGET) == 0 && package_target != 0 {
        let reference = e
            .call(PACKAGE_TARGET_GET_REFERENCE, &args![package_target])
            .u32();
        e.vcall(process, SLOT_SET_TARGET, &args![reference]);
    }
    let target = e.vcall(process, SLOT_GET_TARGET, &args![]).u32();
    let location = e.vcall(process, SLOT_GET_GENERIC_LOCATION, &args![]).u32();
    if target == 0 && location == 0 {
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
        return;
    }
    // `FILD` of the unsigned number: exact in the x87, rounded once to `float`.
    let package_distance = e
        .call(PACKAGE_DISTANCE_FOR_ACTOR, &args![package, actor])
        .u32() as f32;
    let flee_distance = if package_target != 0 {
        e.call(WORD_AT_8, &args![package_target]).i32() as f32
    } else {
        e.global::<f32>(NO_LIMIT_FLOAT)
    };
    let must_flee = if location != 0 {
        let distance = e
            .call(DISTANCE_FROM_REFERENCE, &args![actor, location, 0u32, 0u32])
            .f64();
        f64::from(package_distance) < distance
    } else {
        let distance = e
            .call(DISTANCE_FROM_REFERENCE, &args![actor, target, 0u32, 0u32])
            .f64();
        f64::from(flee_distance) > distance
    };
    if must_flee {
        e.vcall(
            a,
            ACTOR_SLOT_0X410,
            &args![
                target,
                1u32,
                1u32,
                0u32,
                0u32,
                location,
                flee_distance,
                package_distance
            ],
        );
        let current = e
            .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
            .u32();
        let mut flee_package = 0u32;
        if current != 0 && e.call(PACKAGE_TYPE, &args![current]).i32() == 0x16 {
            flee_package = current;
        }
        if flee_package != 0 && target != 0 {
            e.call(FLEE_PACKAGE_ADD_AVOIDED_REF, &args![flee_package, target]);
        }
        return;
    }
    if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
        e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
        let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
        if animation != 0
            && e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                .bool()
        {
            e.vcall(
                process,
                SLOT_SETUP_SPECIAL_IDLE,
                &args![actor, 0u32, 2u32, 1u32, 0u32, 1u32],
            );
        }
    } else if e.call(PACKAGE_FLAG_4, &args![package]).bool()
        || e.call(PACKAGE_FLAG_2, &args![package]).bool()
    {
        e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
        e.vcall(process, SLOT_SET_CURRENT_ACTION_COMPLETE, &args![1u32]);
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
        e.vcall(process, SLOT_CHECK_FOR_NEW_PACKAGE, &args![actor, 0u32]);
    }
}

// Translated from 008defe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: returns the byte at +0x6cc of `this`, the greet
/// flag that `008dd880` sets (a `PlayerCharacter` field).
pub fn fn_008defe0(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x6cc)
}

// Translated from 008df000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: calls `006d2c00` (pathfinding) on the member at
/// +0x34 of `this`.
pub fn fn_008df000(e: &mut Engine, this: Ptr) {
    e.call(0x006d_2c00, &args![this.addr() + 0x34]);
}

// Translated from 008df020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: returns the byte at +0x94 of `this`.
pub fn fn_008df020(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x94)
}

// Translated from 008df040 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: returns the byte at +0x80 of `this`.
pub fn fn_008df040(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x80)
}

// Translated from 008df1c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: stores `value` in the byte at +0xa1 of `this`
/// (a `PathingRequest`: `ProcessSurface` sets it to 1).
pub fn fn_008df1c0(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0xa1, value);
}

/// `Actor::CheckBreathTimer` (`0087f660`).
const ACTOR_CHECK_BREATH_TIMER: u32 = 0x0087_f660;
/// `TESObjectREFR::GetRelevantWaterHeight` (`0057b0a0`): `ST0` result.
const REFERENCE_WATER_HEIGHT: u32 = 0x0057_b0a0;
/// `PathingRequest::PathingRequest` (`006e2420`), size 0xb4.
const PATHING_REQUEST_CONSTRUCTOR: u32 = 0x006e_2420;
/// `Actor::BuildRequest` (`008b3880`): `(request, position, cell, world
/// space, float, int)`.
const ACTOR_BUILD_REQUEST: u32 = 0x008b_3880;
/// `PathingRequest` call `006e2960` with a float.
const PATHING_REQUEST_SET_RADIUS: u32 = 0x006e_2960;
/// `Actor::SetPathfindingGoal` (`008b3630`).
const ACTOR_SET_PATHFINDING_GOAL_REQUEST: u32 = 0x008b_3630;
/// `PathingRequest` destructor (`006e2620`).
const PATHING_REQUEST_DESTRUCTOR: u32 = 0x006e_2620;
/// `Actor::StopMoving` (`008b3ab0`).
const ACTOR_STOP_MOVING: u32 = 0x008b_3ab0;
/// The float `ProcessSurface` raises the surface height by (`010181e8`, 25.0).
const SURFACE_HEIGHT_OFFSET: u32 = 0x0101_81e8;

// Translated from 008df060 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessSurface` (Xbox PDB): an actor swimming that must come
/// up. Does nothing without an actor. While the actor's breath timer runs
/// (`CheckBreathTimer`) it clears the run-once package (virtual `+0x214` of the
/// process) and stops moving; otherwise, once its pathing is complete, it builds
/// a `PathingRequest` to its own position with the height raised to the
/// relevant water height plus 25.0 (`010181e8`), gives it the radius 25.0,
/// sets the byte at +0xa1 and sets that request as the pathfinding goal. The
/// SEH frame is not translated.
pub fn high_process_process_surface(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    if actor.is_null() {
        return;
    }
    let a = actor.addr();
    if e.call(ACTOR_CHECK_BREATH_TIMER, &args![actor]).bool() {
        e.vcall(process, SLOT_CLEAR_RUN_ONCE_PACKAGE, &args![]);
        e.call(ACTOR_STOP_MOVING, &args![actor]);
        return;
    }
    if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
        return;
    }
    let rise = e.global::<f32>(SURFACE_HEIGHT_OFFSET);
    let position_slot = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
    e.with_stack(12, |e, position| {
        for word in 0..3 {
            let value = e.mem.u32(position_slot + word * 4);
            e.mem.set_u32(position.addr() + word * 4, value);
        }
        let water = e.call(REFERENCE_WATER_HEIGHT, &args![actor]).f64();
        let height = (water + f64::from(rise)) as f32;
        e.mem.set_f32(position.addr() + 8, height);
        e.with_stack(0xb4, |e, request| {
            e.call(PATHING_REQUEST_CONSTRUCTOR, &args![request]);
            // `GetWorldSpace` and `008d6f30` take no arguments; the pushed
            // float and 0 remain as the last two arguments of `BuildRequest`.
            let world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![actor]).u32();
            let cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
            e.call(
                ACTOR_BUILD_REQUEST,
                &args![actor, request, position, cell, world_space, rise, 0u32],
            );
            e.call(PATHING_REQUEST_SET_RADIUS, &args![request, rise]);
            fn_008df1c0(e, request, 1);
            e.call(ACTOR_SET_PATHFINDING_GOAL_REQUEST, &args![actor, request]);
            e.call(PATHING_REQUEST_DESTRUCTOR, &args![request]);
        });
    });
}

/// `operator new` (`00aa13e0`, cdecl, one argument).
const OPERATOR_NEW: u32 = 0x00aa_13e0;
/// `operator delete` with a size (`00aa1460`, cdecl: pointer, size).
const OPERATOR_DELETE_SIZED: u32 = 0x00aa_1460;
/// `PathingRequest::PathingRequest` allocation size.
const PATHING_REQUEST_SIZE: u32 = 0xb0;
/// `TESPackage::GetEscortFollowDistance` (Xbox PDB, `006784c0`).
const PACKAGE_ESCORT_FOLLOW_DISTANCE: u32 = 0x0067_84c0;
/// `NiPoint3` difference (`00439ef0`): `(this - other)` written to `out`.
const POINT_DIFFERENCE: u32 = 0x0043_9ef0;
/// Squared length of the `NiPoint3` in `this` (`004a7290`, `ST0`).
const POINT_SQUARED_LENGTH: u32 = 0x004a_7290;
/// `Actor::IsPathing` (`008b3bd0`).
const ACTOR_IS_PATHING: u32 = 0x008b_3bd0;
/// Cell flag test (`00425fd0`): bit 0 of the byte at +0x24 of the cell.
const CELL_FLAG_TEST: u32 = 0x0042_5fd0;
/// `PathingLocation::GetCell` (Xbox PDB, `006dd4f0`).
const PATHING_LOCATION_GET_CELL: u32 = 0x006d_d4f0;
/// `PathingLocation::GetWorldspace` (Xbox PDB, `00441110`).
const PATHING_LOCATION_GET_WORLDSPACE: u32 = 0x0044_1110;
/// `PathingLocation::PathingLocation_ov3` (Xbox PDB, `006dcd70`): builds a
/// location from an actor.
const PATHING_LOCATION_FROM_ACTOR: u32 = 0x006d_cd70;
/// `PathingLocation` destructor (`004ff7e0`).
const PATHING_LOCATION_DESTRUCTOR: u32 = 0x004f_f7e0;
/// `ActorPathingMessage::ActorPathingMessage` (Xbox PDB, `006e9bd0`).
const ACTOR_PATHING_MESSAGE_CONSTRUCTOR: u32 = 0x006e_9bd0;
/// `ActorPathingMessage::~ActorPathingMessage` (Xbox PDB, `00800190`).
const ACTOR_PATHING_MESSAGE_DESTRUCTOR: u32 = 0x0080_0190;
/// Fills a `PathingRequest` for an actor (`006e29f0`, one argument: the actor).
const PATHING_REQUEST_SET_ACTOR: u32 = 0x006e_29f0;
/// `PathingRequest` goal assignment from a `PathingLocation` (`006d1bc0`).
const PATHING_REQUEST_SET_GOAL: u32 = 0x006d_1bc0;
/// `NiPointer<ActorPathingMessageQueue>` assignment (`009057d0`).
const QUEUE_POINTER_SET: u32 = 0x0090_57d0;
/// `NiPointer` copy constructors: `(slot, address of the source pointer)`,
/// for the queue (`006ebf30`) and for the request (`00559a40`).
const QUEUE_POINTER_COPY: u32 = 0x006e_bf30;
const REQUEST_POINTER_COPY: u32 = 0x0055_9a40;
/// `PathManager::QInstance` (Xbox PDB, `0047d0b0`).
const PATH_MANAGER_INSTANCE: u32 = 0x0047_d0b0;
/// `PathManager::BuildPath` (Xbox PDB, `006eb9d0`): `(request pointer, queue
/// pointer, flag)`, the pointers passed by value.
const PATH_MANAGER_BUILD_PATH: u32 = 0x006e_b9d0;
/// Length of the solution a path message carries (`006e7800`, `ST0`).
const PATHING_SOLUTION_LENGTH: u32 = 0x006e_7800;
/// Settings `ShouldWaitForEscortTarget` reads.
const ESCORT_SETTING_BASE_RADIUS: u32 = 0x011c_dbac;
const ESCORT_SETTING_RADIUS_SCALE: u32 = 0x011c_d82c;
const ESCORT_SETTING_PATHING_EXTRA: u32 = 0x011c_d1c8;
/// `HighProcess` members (Xbox PDB): `NiPointer<PathingRequest>
/// spMeToGoalRequest` (+0x45c) and `spEscortedToGoalRequest` (+0x460),
/// `NiPointer<ActorPathingMessageQueue> spMeToGoalMessageQueue` (+0x464) and
/// `spEscortedToGoalMessageQueue` (+0x468).
const ME_TO_GOAL_REQUEST: u32 = 0x45c;
const ESCORTED_TO_GOAL_REQUEST: u32 = 0x460;
const ME_TO_GOAL_QUEUE: u32 = 0x464;
const ESCORTED_TO_GOAL_QUEUE: u32 = 0x468;
/// A `double` at `01088140`: the factor `ShouldWaitForEscortTarget` applies
/// to the squared radius for its out flag.
const ESCORT_FLAG_FACTOR: u32 = 0x0108_8140;
/// Message-queue slots (`BSTMessageQueue`): `Push` (+0x4), `TryPop` (+0x10).
const QUEUE_SLOT_PUSH: u32 = 0x4;
const QUEUE_SLOT_TRY_POP: u32 = 0x10;

// Translated from 008df900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; the body is the constructor of
/// `ActorPathingMessageQueue` (Xbox PDB class, 0x1c bytes): the
/// `BSTCommonLLMessageQueue` base (`00905580`, with the message pool at
/// `011d77f0`), the `NiRefObject` base at +0x14 (`004968b0`) and the two
/// vtables (`01088158`, `0108814c`). Returns `this`. The SEH frame is not
/// translated.
pub fn fn_008df900(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x0090_5580, &args![this, 0x011d_77f0u32]);
    e.call(0x0049_68b0, &args![this.addr() + 0x14]);
    e.mem.set_u32(this.addr(), 0x0108_8158);
    e.mem.set_u32(this.addr() + 0x14, 0x0108_814c);
    this
}

// Translated from 008df980 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; the body is the scalar deleting destructor of
/// `ActorPathingMessageQueue`: runs `008df9b0` and, when bit 0 of `flags` is
/// set, frees the 0x1c bytes (`00aa1460`). Returns `this`.
pub fn fn_008df980(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_008df9b0(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE_SIZED, &args![this, 0x1cu32]);
    }
    this
}

// Translated from 008df9b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; the body is the destructor of
/// `ActorPathingMessageQueue`: the `NiRefObject` base at +0x14 (`00496910`),
/// then the `BSTCommonLLMessageQueue` base (`009055c0`). The SEH frame is not
/// translated.
pub fn fn_008df9b0(e: &mut Engine, this: Ptr) {
    e.call(0x0049_6910, &args![this.addr() + 0x14]);
    e.call(0x0090_55c0, &args![this]);
}

/// `ShouldWaitForEscortTarget`'s path building for one actor: allocates a
/// `PathingRequest` into `request_slot` of the process, fills it from `actor`
/// and the goal `location`, allocates an `ActorPathingMessageQueue` into
/// `queue_slot`, takes counted copies of both pointers and hands them to
/// `PathManager::BuildPath`.
fn escort_build_path(
    e: &mut Engine,
    process: u32,
    request_slot: u32,
    queue_slot: u32,
    actor: Ptr,
    location: Ptr,
) {
    let memory = e.call(OPERATOR_NEW, &args![PATHING_REQUEST_SIZE]).u32();
    let request = if memory != 0 {
        e.call(PATHING_REQUEST_CONSTRUCTOR, &args![memory]).u32()
    } else {
        0
    };
    e.call(NI_POINTER_SET, &args![process + request_slot, request]);
    let held = e.call(NI_POINTER_GET, &args![process + request_slot]).u32();
    e.call(PATHING_REQUEST_SET_ACTOR, &args![held, actor]);
    let held = e.call(NI_POINTER_GET, &args![process + request_slot]).u32();
    e.call(PATHING_REQUEST_SET_GOAL, &args![held, location]);
    let memory = e.call(OPERATOR_NEW, &args![0x1cu32]).u32();
    let queue = if memory != 0 {
        fn_008df900(e, Ptr::new(memory)).addr()
    } else {
        0
    };
    e.call(QUEUE_POINTER_SET, &args![process + queue_slot, queue]);
    let queue_copy = e.with_stack(4, |e, slot| {
        e.call(QUEUE_POINTER_COPY, &args![slot, process + queue_slot]);
        e.mem.u32(slot.addr())
    });
    let request_copy = e.with_stack(4, |e, slot| {
        e.call(REQUEST_POINTER_COPY, &args![slot, process + request_slot]);
        e.mem.u32(slot.addr())
    });
    let manager = e.call(PATH_MANAGER_INSTANCE, &args![]).u32();
    e.call(
        PATH_MANAGER_BUILD_PATH,
        &args![manager, request_copy, queue_copy, 1u32],
    );
}

/// `ShouldWaitForEscortTarget`'s check of the answers to the earlier path
/// requests: asks the "me" queue and the "escorted" queue for a message
/// (`TryPop`). When both deliver, releases the requests and queues and
/// returns whether both messages carry a solution and the first is shorter
/// than the second; when only the first delivers, pushes it back on the "me"
/// queue.
fn escort_collect_messages(e: &mut Engine, process: u32) -> bool {
    e.with_stack(8, |e, my_message| {
        e.with_stack(8, |e, escorted_message| {
            e.call(ACTOR_PATHING_MESSAGE_CONSTRUCTOR, &args![my_message]);
            e.call(ACTOR_PATHING_MESSAGE_CONSTRUCTOR, &args![escorted_message]);
            let mut shorter = false;
            let queue = e
                .call(NI_POINTER_GET, &args![process + ME_TO_GOAL_QUEUE])
                .u32();
            if e.vcall(queue, QUEUE_SLOT_TRY_POP, &args![my_message])
                .bool()
            {
                let other_queue = e
                    .call(NI_POINTER_GET, &args![process + ESCORTED_TO_GOAL_QUEUE])
                    .u32();
                if e.vcall(other_queue, QUEUE_SLOT_TRY_POP, &args![escorted_message])
                    .bool()
                {
                    e.call(NI_POINTER_SET, &args![process + ME_TO_GOAL_REQUEST, 0u32]);
                    e.call(
                        NI_POINTER_SET,
                        &args![process + ESCORTED_TO_GOAL_REQUEST, 0u32],
                    );
                    e.call(QUEUE_POINTER_SET, &args![process + ME_TO_GOAL_QUEUE, 0u32]);
                    e.call(
                        QUEUE_POINTER_SET,
                        &args![process + ESCORTED_TO_GOAL_QUEUE, 0u32],
                    );
                    // The solution pointers sit at +4 of the messages.
                    let mine = e.call(NI_POINTER_GET, &args![my_message.addr() + 4]).u32();
                    let theirs = e
                        .call(NI_POINTER_GET, &args![escorted_message.addr() + 4])
                        .u32();
                    if mine != 0 && theirs != 0 {
                        let mine = e.call(NI_POINTER_GET, &args![my_message.addr() + 4]).u32();
                        let my_length = e.call(PATHING_SOLUTION_LENGTH, &args![mine]).f32();
                        let theirs = e
                            .call(NI_POINTER_GET, &args![escorted_message.addr() + 4])
                            .u32();
                        let their_length = e.call(PATHING_SOLUTION_LENGTH, &args![theirs]).f32();
                        shorter = my_length < their_length;
                    }
                } else {
                    let queue = e
                        .call(NI_POINTER_GET, &args![process + ME_TO_GOAL_QUEUE])
                        .u32();
                    e.vcall(queue, QUEUE_SLOT_PUSH, &args![my_message]);
                }
            }
            e.call(ACTOR_PATHING_MESSAGE_DESTRUCTOR, &args![escorted_message]);
            e.call(ACTOR_PATHING_MESSAGE_DESTRUCTOR, &args![my_message]);
            shorter
        })
    })
}

// Translated from 008df1e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ShouldWaitForEscortTarget` (Xbox PDB): whether the escorting
/// actor `me` should wait for the `escorted` one on the way to `location`.
/// `wait_flag`, when not null, receives a byte (see below).
///
/// The wait radius starts at the package's escort follow distance (at least
/// 200). For the player's package it is used as it is; otherwise it is the
/// setting `011cdbac` in a cell for which `00425fd0` holds and
/// `011cd82c * distance` in the others; it grows by the setting `011cd1c8`
/// while `me` is pathing. The squared distance between the two positions is
/// compared with the squared radius: `wait_flag` receives 1 when it exceeds
/// `radius^2` times the `double` at `01088140`, and the function returns
/// false right away when it is not above the squared radius. Otherwise
/// `wait_flag` is cleared and the pathing requests are looked at: with both
/// requests present their message queues are polled
/// ([`escort_collect_messages`]) and a shorter own solution returns true;
/// without them requests and queues are built for both actors
/// ([`escort_build_path`]). In every other case the result is "`me` is not
/// pathing". The SEH frame and the unwinding of the path objects are not
/// translated.
pub fn high_process_should_wait_for_escort_target(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    me: Ptr,
    escorted: Ptr,
    location: Ptr,
    wait_flag: Ptr,
) -> bool {
    let process = this.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    let other_position = e
        .vcall(escorted.addr(), ACTOR_SLOT_POSITION, &args![])
        .u32();
    let (squared_length, radius) = e.with_stack(12, |e, difference| {
        let my_position = e.vcall(me.addr(), ACTOR_SLOT_POSITION, &args![]).u32();
        e.call(
            POINT_DIFFERENCE,
            &args![my_position, difference, other_position],
        );
        let distance = e
            .call(PACKAGE_ESCORT_FOLLOW_DISTANCE, &args![package])
            .i32();
        let distance = if distance > 200 { distance } else { 200 };
        let mut radius = if e.mem.u32(process + LOW_TARGET) == player_pointer(e) {
            distance as f32
        } else {
            let cell = e.call(REFERENCE_PATHING_CELL, &args![me]).u32();
            if e.call(CELL_FLAG_TEST, &args![cell]).bool() {
                let setting = e
                    .call(SETTING_VALUE, &args![ESCORT_SETTING_BASE_RADIUS])
                    .u32();
                e.mem.f32(setting)
            } else {
                let setting = e
                    .call(SETTING_VALUE, &args![ESCORT_SETTING_RADIUS_SCALE])
                    .u32();
                (f64::from(e.mem.f32(setting)) * f64::from(distance)) as f32
            }
        };
        if e.call(ACTOR_IS_PATHING, &args![me]).bool() {
            let setting = e
                .call(SETTING_VALUE, &args![ESCORT_SETTING_PATHING_EXTRA])
                .u32();
            radius = (f64::from(radius) + f64::from(e.mem.f32(setting))) as f32;
        }
        let length = e.call(POINT_SQUARED_LENGTH, &args![difference]).f32();
        (length, radius)
    });
    let squared_radius = (f64::from(radius) * f64::from(radius)) as f32;
    if !wait_flag.is_null() {
        let factor = e.global::<f64>(ESCORT_FLAG_FACTOR);
        let limit = (f64::from(squared_radius) * factor) as f32;
        e.mem
            .set_u8(wait_flag.addr(), u8::from(limit < squared_length));
    }
    // Not above the squared radius (or unordered): nothing to wait for.
    if squared_radius.partial_cmp(&squared_length) != Some(std::cmp::Ordering::Less) {
        return false;
    }
    if !wait_flag.is_null() {
        e.mem.set_u8(wait_flag.addr(), 0);
    }
    // The cell and world space of the goal are fetched and not used.
    e.call(PATHING_LOCATION_GET_CELL, &args![location]);
    e.call(PATHING_LOCATION_GET_WORLDSPACE, &args![location]);
    e.with_stack(0x34, |e, my_location| {
        e.call(PATHING_LOCATION_FROM_ACTOR, &args![my_location, me]);
        let result = e.with_stack(0x34, |e, escorted_location| {
            e.call(
                PATHING_LOCATION_FROM_ACTOR,
                &args![escorted_location, escorted],
            );
            let my_request = e
                .call(NI_POINTER_GET, &args![process + ME_TO_GOAL_REQUEST])
                .u32();
            let escorted_request = if my_request != 0 {
                e.call(NI_POINTER_GET, &args![process + ESCORTED_TO_GOAL_REQUEST])
                    .u32()
            } else {
                0
            };
            let result = if my_request == 0 || escorted_request == 0 {
                escort_build_path(
                    e,
                    process,
                    ME_TO_GOAL_REQUEST,
                    ME_TO_GOAL_QUEUE,
                    me,
                    location,
                );
                escort_build_path(
                    e,
                    process,
                    ESCORTED_TO_GOAL_REQUEST,
                    ESCORTED_TO_GOAL_QUEUE,
                    escorted,
                    location,
                );
                None
            } else if escort_collect_messages(e, process) {
                Some(true)
            } else {
                None
            };
            let result = match result {
                Some(done) => done,
                None => !e.call(ACTOR_IS_PATHING, &args![me]).bool(),
            };
            e.call(PATHING_LOCATION_DESTRUCTOR, &args![escorted_location]);
            result
        });
        e.call(PATHING_LOCATION_DESTRUCTOR, &args![my_location]);
        result
    })
}

/// Process slot `0x11c` (`GetCurrentActionComplete`, Xbox PDB), `0x394`,
/// `0x7d0` and `0x818` (called with the actor), `0x338` is
/// [`SLOT_SET_IDLE_TIMER`].
const SLOT_GET_CURRENT_ACTION_COMPLETE: u32 = 0x11c;
const SLOT_EAT_AFTER: u32 = 0x394;
const SLOT_ACQUIRE_FOR_TARGET: u32 = 0x7d0;
const SLOT_SEARCH_AFTER: u32 = 0x818;
/// Slot `0x3d0` of the `Actor`: `(reference, 1, 0)`.
const ACTOR_SLOT_0X3D0: u32 = 0x3d0;
/// `Actor::GetBestFoodItem` (Xbox PDB, `00891d30`), `Actor::Eat` (Xbox PDB,
/// `008c1de0`: `(this, form, extra data, 1)`) and `Actor::QueueEquipObject`
/// (Xbox PDB, `0088c650`: `(this, item, 1, 0, 1, 0, 1)`).
const ACTOR_GET_BEST_FOOD_ITEM: u32 = 0x0089_1d30;
const ACTOR_EAT: u32 = 0x008c_1de0;
const ACTOR_QUEUE_EQUIP_OBJECT: u32 = 0x0088_c650;
/// `TESPackage::GetLocationReference` (Xbox PDB, `00676140`): `(this, actor)`.
const PACKAGE_GET_LOCATION_REFERENCE: u32 = 0x0067_6140;
/// `TESIdleManager::SetUsedItem` (Xbox PDB, `00600900`, cdecl) and
/// `RandomFloat` (Xbox PDB, `00476b70`, cdecl `(low, high)`, `ST0`).
const IDLE_MANAGER_SET_USED_ITEM: u32 = 0x0060_0900;
const RANDOM_FLOAT_RANGE: u32 = 0x0047_6b70;
const EAT_IDLE_TIMER_MAXIMUM: u32 = 0x0105_0c48;
/// `Actor` calls clearing a move mode bit: `00884f80` and
/// `Actor::ClearMoveMode` (Xbox PDB, `008b3a80`, one argument).
const ACTOR_BEFORE_CLEAR_MOVE_MODE: u32 = 0x0088_4f80;
const ACTOR_CLEAR_MOVE_MODE: u32 = 0x008b_3a80;
/// `BGSDefaultObjectManager::GetDefaultObject` (Xbox PDB, `0058db10`, cdecl).
const GET_DEFAULT_OBJECT: u32 = 0x0058_db10;
/// `PackageTarget::GetTargObject` (Xbox PDB, `00680050`), the form list's
/// first node (`00500940`) and a random integer in a range (`00944460`,
/// cdecl `(low, high)`).
const PACKAGE_TARGET_GET_OBJECT: u32 = 0x0068_0050;
const FORM_LIST_FIRST_NODE: u32 = 0x0050_0940;
const RANDOM_INT_IN_RANGE: u32 = 0x0094_4460;
/// `TESPackage` search calls: `GetPackageSearchLocation` (`00672f20`),
/// `GetSearchLocationRadius` (`006758a0`), `GetSearchLocationCell`
/// (`006757e0`) and `GetSearchLocationCoord` (`00675830`: `(this, out,
/// actor)`).
const PACKAGE_SEARCH_LOCATION: u32 = 0x0067_2f20;
const PACKAGE_SEARCH_RADIUS: u32 = 0x0067_58a0;
const PACKAGE_SEARCH_CELL: u32 = 0x0067_57e0;
const PACKAGE_SEARCH_COORD: u32 = 0x0067_5830;
/// A reference's name word (`0055d520`) and the logging call `005b5e40`
/// (cdecl: a format string, then three words).
const REFERENCE_NAME_WORD: u32 = 0x0055_d520;
const LOG_CALL: u32 = 0x005b_5e40;
const SEARCH_RADIUS_MESSAGE: u32 = 0x0108_8178;
const PROCESS_SLOT_0X130: u32 = 0x130;
/// The acquire callback `0090dd80` and the globals around the search:
/// the byte `011e01dc`, the float `01016970` and the setting `011cdebc`.
const ACQUIRE_CALLBACK: u32 = 0x0090_dd80;
const SEARCH_IN_PROGRESS: u32 = 0x011e_01dc;
const INTERIOR_SEARCH_RADIUS: u32 = 0x0101_6970;
const EXTERIOR_SEARCH_RADIUS_SETTING: u32 = 0x011c_debc;
/// `MiddleHighProcess`/`LowProcess` fields set around the search:
/// `pObjecttoAcquire` (+0x84), `eFormType` (+0x8c) and `pFactiontoAquire`
/// (+0x90).
const LOW_OBJECT_TO_ACQUIRE: u32 = 0x84;
const LOW_FORM_TYPE: u32 = 0x8c;
const LOW_FACTION_TO_ACQUIRE: u32 = 0x90;
/// The item the process is using, `pItemBeingUsed` (`LowProcess`, +0x34).
const LOW_ITEM_BEING_USED: u32 = 0x34;
/// `HighProcess` field: the entries of the acquire list have a dword at +0xc
/// that `ProcessEat` sets to 1 and a kind at +0x18.
const ACQUIRE_TAKEN: u32 = 0xc;
/// The word at +0xc... `ObjectstoAcquire::eKind` is [`ACQUIRE_KIND`].
const EAT_CLEAR_MOVE_MODE_BIT: u32 = 0x400;

/// What `ProcessEat` does for a food object found in the package target: a
/// form of type 0x55 (a form list) is replaced by a random element of it; the
/// result is kept when its type is 0x1d or 0x2f.
fn eat_pick_food_object(e: &mut Engine, package: u32) -> Option<u32> {
    let package_target = e.call(PACKAGE_TARGET_WORD, &args![package]).u32();
    let mut object = 0u32;
    if package_target != 0
        && e.call(PACKAGE_TARGET_GET_TYPE, &args![package_target])
            .i32()
            == 1
    {
        object = e
            .call(PACKAGE_TARGET_GET_OBJECT, &args![package_target])
            .u32();
    }
    if object == 0 {
        object = e.call(GET_DEFAULT_OBJECT, &args![0x12u32]).u32();
    }
    if object == 0 {
        return None;
    }
    if e.call(FORM_TYPE_BYTE, &args![object]).i32() == 0x55 {
        let mut node = e.call(FORM_LIST_FIRST_NODE, &args![object]).u32();
        if node != 0 {
            let count = e.call(LIST_COUNT, &args![node]).i32();
            let mut remaining = e
                .call(RANDOM_INT_IN_RANGE, &args![0u32, (count - 1) as u32])
                .i32();
            while node != 0 && remaining > 0 {
                node = e.call(NODE_NEXT, &args![node]).u32();
                remaining -= 1;
            }
            let first = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
            object = e.mem.u32(first);
        }
    }
    let kind = e.call(FORM_TYPE_BYTE, &args![object]).i32();
    if kind == 0x1d || kind == 0x2f {
        Some(object)
    } else {
        None
    }
}

// Translated from 008e3140 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessEat` (Xbox PDB): the eating package; `location` is
/// the reference of the place to eat at (0: the package's
/// `GetLocationReference`).
///
/// An actor in state 9 calls its slot `0x418` and stops. When the current
/// action is complete, or the actor is not of the kind tested by slot `0x218`,
/// it eats what is on the ground ([`eat_from_the_world`]); otherwise it looks
/// for food in its inventory (`GetBestFoodItem`) and for furniture to eat at
/// ([`eat_with_inventory_food`]), or, without food, for something to acquire
/// ([`eat_find_food`]).
pub fn high_process_process_eat(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, location: u32) {
    let process = this.addr();
    let a = actor.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32() == 9 {
        e.vcall(a, ACTOR_SLOT_0X418, &args![]);
        return;
    }
    if e.vcall(process, SLOT_GET_CURRENT_ACTION_COMPLETE, &args![])
        .bool()
        || !e.vcall(a, ACTOR_SLOT_0X218, &args![]).bool()
    {
        eat_from_the_world(e, this, actor, package);
    } else {
        let food = e.call(ACTOR_GET_BEST_FOOD_ITEM, &args![actor]).u32();
        if food != 0 {
            let word = e.call(WORD_AT_8, &args![food]).u32();
            e.mem.set_u32(process + LOW_ITEM_BEING_USED, word);
        }
        if food != 0 || e.mem.u32(process + LOW_ITEM_BEING_USED) != 0 {
            eat_with_inventory_food(e, this, actor, package, food, location);
        } else {
            eat_find_food(e, this, actor, package);
        }
    }
}

/// `ProcessEat` for an actor whose action is complete (or that is not of the
/// kind tested by slot `0x218`): eats the food the package location refers
/// to, or picks the object to eat from the package target, then runs the
/// idle timer.
fn eat_from_the_world(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, package: u32) {
    let process = this.addr();
    let a = actor.addr();
    let reference = e
        .call(PACKAGE_GET_LOCATION_REFERENCE, &args![package, actor])
        .u32();
    let mut ate = false;
    if !e.vcall(a, ACTOR_SLOT_0X218, &args![]).bool()
        && reference != 0
        && !e.call(REFERENCE_FLAG_20, &args![reference]).bool()
    {
        let form = e.call(REFERENCE_GET_FORM, &args![reference]).u32();
        if e.call(FORM_TYPE_BYTE, &args![form]).i32() == 0x1d {
            e.vcall(a, ACTOR_SLOT_0X3D0, &args![reference, 1u32, 0u32]);
            let form = e.call(REFERENCE_GET_FORM, &args![reference]).u32();
            e.mem.set_u32(process + LOW_ITEM_BEING_USED, form);
            let extra = e.call(REFERENCE_EXTRA_DATA, &args![reference]).u32();
            let form = e.call(REFERENCE_GET_FORM, &args![reference]).u32();
            e.call(ACTOR_EAT, &args![actor, form, extra, 1u32]);
            e.vcall(process, SLOT_EAT_AFTER, &args![actor]);
            ate = true;
        }
    }
    if !ate && e.mem.u32(process + LOW_ITEM_BEING_USED) == 0 {
        if let Some(object) = eat_pick_food_object(e, package) {
            e.mem.set_u32(process + LOW_ITEM_BEING_USED, object);
            return;
        }
    }
    // The idle timer: while the actor's animation has finished its special
    // idle the timer runs down; at 0 the item is used for a new idle.
    let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
    if animation != 0 {
        let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
        if e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
            .bool()
        {
            let timer = e.vcall(process, SLOT_GET_IDLE_TIMER, &args![]).f32();
            // `FCOMP` against 0.0: above it (or unordered) counts down.
            if f64::from(timer) > e.global::<f64>(ZERO_DOUBLE) || timer.is_nan() {
                let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
                let remaining = (f64::from(timer) - frame) as f32;
                e.vcall(process, SLOT_SET_IDLE_TIMER, &args![remaining]);
            } else {
                let item = e.mem.u32(process + LOW_ITEM_BEING_USED);
                e.call(IDLE_MANAGER_SET_USED_ITEM, &args![item]);
                e.vcall(
                    process,
                    SLOT_SETUP_SPECIAL_IDLE,
                    &args![actor, 0u32, 2u32, 1u32, 0u32, 1u32],
                );
                let high = e.global::<f32>(EAT_IDLE_TIMER_MAXIMUM);
                let random = e.call(RANDOM_FLOAT_RANGE, &args![1.0f32, high]).f32();
                e.vcall(process, SLOT_SET_IDLE_TIMER, &args![random]);
                e.call(IDLE_MANAGER_SET_USED_ITEM, &args![0u32]);
            }
        }
    }
}

/// `ProcessEat` for an actor with food in its inventory (or an item in use):
/// finds furniture to sit at, eats once seated and finishes.
fn eat_with_inventory_food(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    package: u32,
    food: u32,
    location: u32,
) {
    let process = this.addr();
    let a = actor.addr();
    let furniture = process + PROCESS_CURRENT_FURNITURE;
    let list = process + PROCESS_CHAIR_BED_LIST;
    let unlimited = e.global::<f32>(NO_LIMIT_FLOAT);
    if !e
        .vcall(
            package,
            PACKAGE_SLOT_0X13C,
            &args![actor, 0u32, unlimited, 0u32],
        )
        .bool()
        && e.mem.u32(furniture) == 0
    {
        e.vcall(process, SLOT_SET_PROCEDURE_INDEX_RUNNING, &args![0u32]);
        return;
    }
    let place = if location != 0 {
        location
    } else {
        e.call(PACKAGE_GET_LOCATION_REFERENCE, &args![package, actor])
            .u32()
    };
    if e.vcall(a, ACTOR_SLOT_STATE, &args![]).u32() == 0
        && e.mem.u32(furniture) == 0
        && (e.call(LIST_IS_EMPTY, &args![list]).bool()
            || e.global::<u32>(FURNITURE_TIME_GLOBAL)
                >= e.mem.u32(process + PROCESS_FURNITURE_LIST_TIMER))
    {
        e.call(PROCESS_FIND_BED_CHAIRS, &args![this, actor, 1u32, place]);
    }
    if e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32() == 4 {
        // Seated: eats what is held once the idle is done.
        let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
        let mut extra = 0u32;
        if food != 0 {
            let word = e.call(WORD_AT_8, &args![food]).u32();
            e.mem.set_u32(process + LOW_ITEM_BEING_USED, word);
            if e.call(NI_POINTER_GET, &args![food]).u32() != 0 {
                let held = e.call(NI_POINTER_GET, &args![food]).u32();
                let node = e.call(NODE_ITEM_ADDRESS, &args![held]).u32();
                extra = e.mem.u32(node);
            }
        }
        if animation == 0
            || e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                .bool()
        {
            let item = e.mem.u32(process + LOW_ITEM_BEING_USED);
            e.call(ACTOR_EAT, &args![actor, item, extra, 1u32]);
        }
        e.vcall(process, SLOT_SET_CURRENT_ACTION_COMPLETE, &args![1u32]);
        e.call(LIST_CLEAR, &args![list]);
        e.vcall(process, SLOT_SET_IDLE_TIMER, &args![0.0f32]);
    } else {
        process_pick_furniture(e, this);
        let chosen = e.mem.u32(furniture);
        e.vcall(process, SLOT_SET_TARGET, &args![chosen]);
        let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
        let idle_done = |e: &mut Engine, animation: u32| {
            e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                .bool()
        };
        if e.mem.u32(furniture) != 0
            && (animation == 0
                || idle_done(e, animation)
                || e.call(PACKAGE_WORD_AT_0X18, &args![package]).i32() == 0x25)
        {
            e.vcall(process, SLOT_PROCESS_ACTIVATE, &args![actor, 0u32]);
        } else if animation != 0 && !idle_done(e, animation) {
            e.vcall(process, SLOT_EAT_AFTER, &args![actor]);
        }
        if e.mem.u32(furniture) == 0 && e.call(LIST_IS_EMPTY, &args![list]).bool() {
            if food != 0 {
                let word = e.call(WORD_AT_8, &args![food]).u32();
                e.mem.set_u32(process + LOW_ITEM_BEING_USED, word);
                let item = e.call(WORD_AT_8, &args![food]).u32();
                e.call(
                    ACTOR_QUEUE_EQUIP_OBJECT,
                    &args![actor, item, 1u32, 0u32, 1u32, 0u32, 1u32],
                );
            }
            e.vcall(process, SLOT_SET_CURRENT_ACTION_COMPLETE, &args![1u32]);
            e.call(LIST_CLEAR, &args![list]);
            e.vcall(process, SLOT_SET_IDLE_TIMER, &args![0.0f32]);
        }
        if e.call(PROCESS_WANTS_TO_FINISH_SLEEP, &args![this, actor])
            .bool()
        {
            e.vcall(process, SLOT_CLEAR_FURNITURE, &args![actor]);
            e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            e.vcall(process, SLOT_SET_IDLE_TIMER, &args![0.0f32]);
        }
    }
    if e.call(ACTOR_MOVE_MODE, &args![actor]).u32() & EAT_CLEAR_MOVE_MODE_BIT != 0 {
        e.call(ACTOR_BEFORE_CLEAR_MOVE_MODE, &args![actor]);
        e.call(
            ACTOR_CLEAR_MOVE_MODE,
            &args![actor, EAT_CLEAR_MOVE_MODE_BIT],
        );
    }
}

/// `ProcessEat` for an actor without food: takes the next thing to acquire
/// (an entry of the acquire list whose reference becomes the target) or looks
/// for food around the package search location.
fn eat_find_food(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, package: u32) {
    let process = this.addr();
    let target = e.mem.u32(process + LOW_TARGET);
    if target != 0 {
        e.vcall(process, SLOT_ACQUIRE_FOR_TARGET, &args![actor]);
        return;
    }
    let target = e.mem.u32(process + LOW_TARGET);
    if target != 0
        && !e.call(REFERENCE_FLAG_20, &args![target]).bool()
        && !e.call(REFERENCE_FLAG_800, &args![target]).bool()
    {
        return;
    }
    let object_list = process + LOW_OBJECT_LIST;
    if !e.call(LIST_IS_EMPTY, &args![object_list]).bool() {
        let node = e.call(NODE_ITEM_ADDRESS, &args![object_list]).u32();
        let entry = e.mem.u32(node);
        e.mem.set_u32(process + LOW_ACQUIRE_OBJECT, entry);
        e.mem.set_u32(entry + ACQUIRE_TAKEN, 1);
        e.call(
            LIST_REMOVE_ITEM,
            &args![object_list, process + LOW_ACQUIRE_OBJECT],
        );
        let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
        let reference = e.mem.u32(entry);
        if e.mem.u32(entry + ACQUIRE_KIND) == 2 {
            if e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
                e.vcall(process, SLOT_SET_TARGET, &args![reference]);
            } else {
                let mut owner_form = 0u32;
                let owner = e.call(REFERENCE_GET_OWNER, &args![reference]).u32();
                if owner != 0 && e.call(FORM_TYPE_BYTE, &args![owner]).i32() == 0x2a {
                    owner_form = owner;
                }
                if owner_form != 0 {
                    let owner_actor = e
                        .call(
                            PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH,
                            &args![PROCESS_LISTS_INSTANCE, owner_form, 0u32],
                        )
                        .u32();
                    e.vcall(process, SLOT_SET_TARGET, &args![owner_actor]);
                }
            }
        } else {
            e.vcall(process, SLOT_SET_TARGET, &args![reference]);
        }
        return;
    }
    // The object list is empty: food is looked for around the package's
    // search location.
    let search_location = e.call(PACKAGE_SEARCH_LOCATION, &args![package]).u32();
    if search_location != 0 {
        let mut radius = e.call(PACKAGE_SEARCH_RADIUS, &args![package, actor]).u32();
        let cell = e.call(PACKAGE_SEARCH_CELL, &args![package, actor]).u32();
        e.with_stack(12, |e, coordinates| {
            e.call(PACKAGE_SEARCH_COORD, &args![package, coordinates, actor]);
            if radius == 0 && cell == 0 {
                let package_slot = e.vcall(package, PROCESS_SLOT_0X130, &args![]).u32();
                let actor_slot = e.vcall(actor.addr(), PROCESS_SLOT_0X130, &args![]).u32();
                let name = e.call(REFERENCE_NAME_WORD, &args![actor]).u32();
                e.call(
                    LOG_CALL,
                    &args![SEARCH_RADIUS_MESSAGE, name, actor_slot, package_slot],
                );
                radius = 0x40;
            }
            let reach = if radius > 0 {
                f64::from(radius) as f32
            } else if e.call(CELL_FLAG_TEST, &args![cell]).bool() {
                e.global::<f32>(INTERIOR_SEARCH_RADIUS)
            } else {
                let setting = e
                    .call(SETTING_VALUE, &args![EXTERIOR_SEARCH_RADIUS_SETTING])
                    .u32();
                e.mem.f32(setting)
            };
            e.mem.set_u8(SEARCH_IN_PROGRESS, 1);
            e.mem.set_u32(process + LOW_FORM_TYPE, 0x12);
            e.mem.set_u32(process + LOW_OBJECT_TO_ACQUIRE, 0);
            let handler = e.global::<u32>(DATA_HANDLER_POINTER);
            e.call(
                ENUM_REFERENCES_CLOSE_TO_POINT,
                &args![
                    handler,
                    cell,
                    coordinates,
                    reach,
                    coordinates,
                    reach,
                    ACQUIRE_CALLBACK,
                    actor
                ],
            );
            e.mem.set_u8(SEARCH_IN_PROGRESS, 0);
            e.mem.set_u32(process + LOW_FORM_TYPE, 0);
            e.mem.set_u32(process + LOW_FACTION_TO_ACQUIRE, 0);
            e.vcall(process, SLOT_SEARCH_AFTER, &args![actor]);
        });
    }
    if e.call(LIST_IS_EMPTY, &args![object_list]).bool() {
        if let Some(object) = eat_pick_food_object(e, package) {
            e.mem.set_u32(process + LOW_ITEM_BEING_USED, object);
            return;
        }
        e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
        let value = e.global::<f32>(SLEEP_ACQUIRE_TIMER_VALUE);
        e.mem.set_f32(process + 0x2e0, value);
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
    }
}

/// The actor's process (`+0x68`) and its virtual `+0x5f0`, which answers the
/// shape the phantom of the actor is made from (an object with a box size).
const ACTOR_PROCESS_WORD: u32 = 0x68;
const PROCESS_SLOT_SHAPE_SOURCE: u32 = 0x5f0;
/// `bhkSimpleShapePhantom` construction: size 0x18 (`0056e2d0`, one argument:
/// the construction info), construction info constructor (`0056e090`) and
/// destructor (`0056ea90`).
const PHANTOM_SIZE: u32 = 0x18;
const PHANTOM_CONSTRUCTOR: u32 = 0x0056_e2d0;
const PHANTOM_INFO_CONSTRUCTOR: u32 = 0x0056_e090;
const PHANTOM_INFO_DESTRUCTOR: u32 = 0x0056_ea90;
/// `bhkBoxShape` allocation size.
const BOX_SHAPE_SIZE: u32 = 0x14;
/// Collision filter helpers: sets the low 7 bits (`004a39f0`), reads the high
/// 16 bits (`004a3a20`) and sets them (`0059ce80`); the filter of a
/// character controller (`0070c440`, `(this, out)`).
const FILTER_SET_LOW: u32 = 0x004a_39f0;
const FILTER_HIGH: u32 = 0x004a_3a20;
const FILTER_SET_HIGH: u32 = 0x0059_ce80;
const CONTROLLER_FILTER: u32 = 0x0070_c440;
/// `TESObjectREFR::GetScale` (Xbox PDB, `00567400`, `ST0`).
const REFERENCE_GET_SCALE: u32 = 0x0056_7400;
/// Box dimensions of the shape source (`00500940`: the pointer to three
/// floats).
const SHAPE_SOURCE_DIMENSIONS: u32 = 0x0050_0940;
/// `NiPoint3` to Havok vector, scaled (`004b4e50`: `(out, point)`), and the
/// world the actor's cell uses (`004543c0`).
const POINT_TO_SCALED_VECTOR: u32 = 0x004b_4e50;
const CELL_PHYSICS_WORLD: u32 = 0x0045_43c0;
/// Settings and doubles: `fDetectDoorForPathingTimer`'s source setting
/// (`011e029c`), the doubles `01018c00` (2.5) and `01011588` (0.5).
const DOOR_TIMER_SETTING: u32 = 0x011e_029c;
const PHANTOM_Y_FACTOR: u32 = 0x0101_8c00;
const PHANTOM_X_FACTOR: u32 = 0x0101_1588;
/// `HighProcess::fDetectDoorForPathingTimer` (Xbox PDB, f32 +0x438).
const DOOR_TIMER: u32 = 0x438;
const PHANTOM_SLOT_ADD_TO_WORLD: u32 = 0x9c;
const PHANTOM_SLOT_0X94: u32 = 0x94;

// Translated from 008e4e50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: gives the process the shape phantom
/// (`spShapePhantom`, +0x434) of `actor` and adds it to the physics world of
/// the actor's cell. Only for an actor whose process has a shape source (its
/// virtual `+0x5f0` is not 0). When the process holds no phantom yet, one is
/// made: a `bhkBoxShape` (`008e5100`) with the half sizes of the source scaled
/// by the actor's scale (x times 0.5, y times 2.5 unless it is the larger one
/// of y and z, z), a collision filter (low bits 0x22, high bits from the
/// character controller's filter) and a position taken from the actor; the
/// timer at +0x438 is set from the setting `011e029c`. The SEH frame and the
/// stack realignment are not translated.
pub fn fn_008e4e50(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let actor_process = e.mem.u32(a + ACTOR_PROCESS_WORD);
    if e.vcall(actor_process, PROCESS_SLOT_SHAPE_SOURCE, &args![])
        .u32()
        == 0
    {
        return;
    }
    let slot = process + SHAPE_PHANTOM;
    if e.call(NI_POINTER_GET, &args![slot]).u32() == 0 {
        let source = e
            .vcall(actor_process, PROCESS_SLOT_SHAPE_SOURCE, &args![])
            .u32();
        let dimensions = e.call(SHAPE_SOURCE_DIMENSIONS, &args![source]).u32();
        let mut x = e.mem.f32(dimensions);
        let mut y = e.mem.f32(dimensions + 4);
        let mut z = e.mem.f32(dimensions + 8);
        let scale = e.call(REFERENCE_GET_SCALE, &args![actor]).f32();
        // `FCOMPP` of z against y: "z < y" scales y alone.
        if z < y {
            y = (f64::from(y) * f64::from(scale)) as f32;
        } else {
            y = (f64::from(y) * e.global::<f64>(PHANTOM_Y_FACTOR) * f64::from(scale)) as f32;
        }
        x = (f64::from(x) * e.global::<f64>(PHANTOM_X_FACTOR) * f64::from(scale)) as f32;
        z = (f64::from(z) * f64::from(scale)) as f32;
        let box_shape = e.with_stack(12, |e, half_extents| {
            e.mem.set_f32(half_extents.addr(), x);
            e.mem.set_f32(half_extents.addr() + 4, y);
            e.mem.set_f32(half_extents.addr() + 8, z);
            let memory = e.call(OPERATOR_NEW, &args![BOX_SHAPE_SIZE]).u32();
            if memory != 0 {
                fn_008e5100(e, Ptr::new(memory), half_extents).addr()
            } else {
                0
            }
        });
        e.with_stack(4, |e, filter| {
            e.mem.set_u32(filter.addr(), 0);
            e.call(FILTER_SET_LOW, &args![filter, 0x22u32]);
            let controller = e
                .vcall(process, SLOT_GET_CHARACTER_CONTROLLER, &args![])
                .u32();
            if controller != 0 {
                e.with_stack(4, |e, controller_filter| {
                    let value = e
                        .call(CONTROLLER_FILTER, &args![controller, controller_filter])
                        .u32();
                    let high = e.call(FILTER_HIGH, &args![value]).u32();
                    e.call(FILTER_SET_HIGH, &args![filter, high]);
                });
            }
            e.with_stack(0x60, |e, info| {
                e.call(PHANTOM_INFO_CONSTRUCTOR, &args![info]);
                let filter_value = e.call(NI_POINTER_GET, &args![filter]).u32();
                e.mem.set_u32(info.addr(), filter_value);
                let shape_word = e.call(ACTOR_CHARACTER_CONTROLLER, &args![box_shape]).u32();
                e.mem.set_u32(info.addr() + 4, shape_word);
                let position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
                e.call(POINT_TO_SCALED_VECTOR, &args![info.addr() + 0x50, position]);
                let memory = e.call(OPERATOR_NEW, &args![PHANTOM_SIZE]).u32();
                let phantom = if memory != 0 {
                    e.call(PHANTOM_CONSTRUCTOR, &args![memory, info]).u32()
                } else {
                    0
                };
                e.call(NI_POINTER_SET, &args![slot, phantom]);
                let setting = e.call(SETTING_VALUE, &args![DOOR_TIMER_SETTING]).u32();
                let value = e.mem.f32(setting);
                e.mem.set_f32(process + DOOR_TIMER, value);
                e.call(PHANTOM_INFO_DESTRUCTOR, &args![info]);
            });
        });
    }
    let cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
    let world = e.call(CELL_PHYSICS_WORLD, &args![cell]).u32();
    let phantom = e.call(NI_POINTER_GET, &args![slot]).u32();
    e.vcall(phantom, PHANTOM_SLOT_ADD_TO_WORLD, &args![world]);
}

/// Allocated sizes and helpers of `008e51b0`: the transform blocks, the
/// collector (`hkpAllCdBodyPairCollector`-like, 0x11c bytes) and the calls
/// working with them.
const NODE_TRANSFORM_SIZE: u32 = 13 * 4;
const COLLECTOR_SIZE: u32 = 0x11c;
const NODE_TRANSFORM: u32 = 0x0046_1130;
const TRANSFORM_COMPOSE: u32 = 0x0062_c250;
const PHANTOM_SET_TRANSFORM: u32 = 0x0056_df20;
const COLLECTOR_CONSTRUCTOR: u32 = 0x0062_d350;
const COLLECTOR_RESET: u32 = 0x0062_d310;
const COLLECTOR_DESTRUCTOR: u32 = 0x0062_d4b0;
const PHANTOM_FILL_COLLECTOR: u32 = 0x0062_3150;
const COLLECTOR_ARRAY: u32 = 0x0041_3f40;
const COLLECTOR_ENTRY: u32 = 0x0062_e150;
const NI_OBJECT_OF_BODY: u32 = 0x00c7_fa90;
const FIND_REFERENCE_FOR_3D: u32 = 0x0056_f930;
const REFERENCE_HAS_EXTRA_FLAG: u32 = 0x0056_8e50;
const ACTOR_SET_BLOCKING_DOOR: u32 = 0x008b_3cb0;
const POINT_CONSTRUCT_DEFAULT: u32 = NODE_ITEM_ADDRESS;
/// The shape of a phantom (`004ae6a0`).
const PHANTOM_SHAPE: u32 = 0x004a_e6a0;
const MATRIX_CONSTRUCT: u32 = 0x0047_6a80;
const MATRIX_FROM_ANGLE: u32 = 0x004a_0c90;
const IDENTITY_MATRIX: u32 = 0x011a_9448;
const DOOR_TIMER_RANDOM_LOW: u32 = 0x0103_0ff0;
const DOOR_TIMER_RANDOM_HIGH: u32 = 0x0101_8204;
const DOOR_ANGLE_FLAG_2: u32 = 0x0102_b3c8;
const DOOR_ANGLE_FLAG_4: u32 = 0x0101_ff38;
const DOOR_ANGLE_FLAG_8: u32 = 0x0101_6b78;

// Translated from 008e51b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: looks for a closed door in the way of `actor`.
/// The timer at +0x438 runs down by the frame time; at 0 it is set to the
/// setting `011e029c` times a random 0.8..1.2 and, when the actor's process
/// has a shape source (+0x5f0), the actor has a 3D node and the process
/// holds a phantom that is in the world (virtual `+0x94`), the phantom is
/// moved in front of the actor (the direction comes from the actor's move
/// mode: bit 2 backwards, bit 4 to the left, bit 8 to the right, else
/// forwards; the offset is the phantom's size) and every body it overlaps
/// is looked at: the last reference found of form type 0x1c that
/// `00568e50` says is not locked is given to `Actor` (`008b3cb0`). Without a
/// phantom in the world nothing is done. The SEH frame is not translated.
pub fn fn_008e51b0(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
    let timer = (f64::from(e.mem.f32(process + DOOR_TIMER)) - frame) as f32;
    e.mem.set_f32(process + DOOR_TIMER, timer);
    // `FCOMP` against 0.0: continues for "at most 0" or unordered.
    if f64::from(timer) > e.global::<f64>(ZERO_DOUBLE) {
        return;
    }
    let setting = e.call(SETTING_VALUE, &args![DOOR_TIMER_SETTING]).u32();
    let low = e.global::<f32>(DOOR_TIMER_RANDOM_LOW);
    let high = e.global::<f32>(DOOR_TIMER_RANDOM_HIGH);
    let random = e.call(RANDOM_FLOAT_RANGE, &args![low, high]).f32();
    let next = (f64::from(random) * f64::from(e.mem.f32(setting))) as f32;
    e.mem.set_f32(process + DOOR_TIMER, next);
    let mut door = 0u32;
    let actor_process = e.mem.u32(a + ACTOR_PROCESS_WORD);
    let slot = process + SHAPE_PHANTOM;
    let look = e
        .vcall(actor_process, PROCESS_SLOT_SHAPE_SOURCE, &args![])
        .u32()
        != 0
        && e.vcall(a, ACTOR_SLOT_NODE_FOR_FOLLOW, &args![]).u32() != 0;
    if look && e.call(NI_POINTER_GET, &args![slot]).u32() != 0 {
        let phantom = e.call(NI_POINTER_GET, &args![slot]).u32();
        if e.vcall(phantom, PHANTOM_SLOT_0X94, &args![]).u32() == 0 {
            return;
        }
    }
    if look {
        door = door_search(e, actor, slot);
    }
    e.call(ACTOR_SET_BLOCKING_DOOR, &args![actor, door]);
}

/// The phantom move and overlap query of `008e51b0`; returns the last door
/// reference found, or 0.
fn door_search(e: &mut Engine, actor: Ptr, slot: u32) -> u32 {
    let a = actor.addr();
    let mut door = 0u32;
    // The phantom's size, as a point (three floats at ebp-0x50).
    let rotation = e.mem.alloc(0x30);
    let unused = e.mem.alloc(0x40);
    let size = e.mem.alloc(12);
    let transform = e.mem.alloc(NODE_TRANSFORM_SIZE);
    let composed = e.mem.alloc(0x40);
    let collector = e.mem.alloc(COLLECTOR_SIZE + 4);
    e.call(POINT_CONSTRUCT_DEFAULT, &args![size]);
    e.call(MATRIX_CONSTRUCT, &args![unused]);
    e.call(MATRIX_CONSTRUCT, &args![rotation]);
    let phantom = e.call(NI_POINTER_GET, &args![slot]).u32();
    let shape = e.call(PHANTOM_SHAPE, &args![phantom]).u32();
    let converted = e.with_stack(12, |e, point| {
        let out = fn_008e5620(e, Ptr::new(shape), point).addr();
        [e.mem.u32(out), e.mem.u32(out + 4), e.mem.u32(out + 8)]
    });
    for (index, word) in converted.iter().enumerate() {
        e.mem.set_u32(size + index as u32 * 4, *word);
    }
    let flags = e.call(ACTOR_MOVE_MODE, &args![actor]).u32();
    let y = f32::from_bits(converted[1]);
    let z = f32::from_bits(converted[2]);
    let (offset, angle) = if flags & 2 != 0 {
        ([0.0, -y, z], Some(DOOR_ANGLE_FLAG_2))
    } else if flags & 4 != 0 {
        ([-y, 0.0, z], Some(DOOR_ANGLE_FLAG_4))
    } else if flags & 8 != 0 {
        ([y, 0.0, z], Some(DOOR_ANGLE_FLAG_8))
    } else {
        ([0.0, y, z], None)
    };
    e.with_stack(12, |e, built| {
        e.call(
            POINT_CONSTRUCT,
            &args![built, offset[0], offset[1], offset[2]],
        );
        for word in 0..3 {
            let value = e.mem.u32(built.addr() + word * 4);
            e.mem.set_u32(rotation + 0x24 + word * 4, value);
        }
    });
    match angle {
        Some(angle) => {
            let value = e.global::<f32>(angle);
            e.call(MATRIX_FROM_ANGLE, &args![rotation, value]);
        }
        None => {
            for word in 0..9 {
                let value = e.mem.u32(IDENTITY_MATRIX + word * 4);
                e.mem.set_u32(rotation + word * 4, value);
            }
        }
    }
    let node = e.vcall(a, ACTOR_SLOT_NODE_FOR_FOLLOW, &args![]).u32();
    let node_transform = e.call(NODE_TRANSFORM, &args![node]).u32();
    for word in 0..13 {
        let value = e.mem.u32(node_transform + word * 4);
        e.mem.set_u32(transform + word * 4, value);
    }
    e.call(TRANSFORM_COMPOSE, &args![transform, composed, rotation]);
    let phantom = e.call(NI_POINTER_GET, &args![slot]).u32();
    e.call(PHANTOM_SET_TRANSFORM, &args![phantom, composed]);
    e.call(COLLECTOR_CONSTRUCTOR, &args![collector]);
    e.call(COLLECTOR_RESET, &args![collector]);
    let phantom = e.call(NI_POINTER_GET, &args![slot]).u32();
    e.call(PHANTOM_FILL_COLLECTOR, &args![phantom, collector]);
    let array = e.call(COLLECTOR_ARRAY, &args![collector]).u32();
    let count = e.call(NODE_NEXT, &args![array]).i32();
    let mut index = 0;
    while index < count {
        let array = e.call(COLLECTOR_ARRAY, &args![collector]).u32();
        let entry = e.call(COLLECTOR_ENTRY, &args![array, index as u32]).u32();
        let body = e.mem.u32(entry + 8);
        let object = e.call(NI_OBJECT_OF_BODY, &args![body]).u32();
        let reference = e.call(FIND_REFERENCE_FOR_3D, &args![object]).u32();
        if reference != 0 {
            let form = e.call(REFERENCE_GET_FORM, &args![reference]).u32();
            if e.call(FORM_TYPE_BYTE, &args![form]).i32() == 0x1c
                && e.call(REFERENCE_HAS_EXTRA_FLAG, &args![reference]).u32() == 0
            {
                door = reference;
            }
        }
        index += 1;
    }
    e.call(COLLECTOR_DESTRUCTOR, &args![collector]);
    for block in [rotation, unused, size, transform, composed, collector] {
        e.mem.free(block);
    }
    door
}

/// Settings and calls of the bone level of detail (`008e5730`).
const BONE_LOD_SCALE_SETTING: u32 = 0x011c_cf88;
const GLOBAL_OBJECT_POINTER_CALL: u32 = 0x0045_c670;
const OBJECT_CHILD_AT_ZERO: u32 = 0x0055_8310;
const OBJECT_POSITION_ADDRESS: u32 = 0x0043_c490;
const POINT_DIVIDE_BY_SCALE: u32 = 0x0053_d280;
const POINT_LENGTH_FOR_LOD: u32 = POINT_LENGTH;
const FLOAT_ABSOLUTE_FOR_LOD: u32 = 0x0040_8840;
const FACE_NODE_ANIMATION_DATA: u32 = 0x0066_29f0;
const FLOAT_AT_0X110: u32 = 0x0050_8070;
const LOD_TABLE_ENTRY: u32 = 0x007d_1d00;
const BONE_LOD_MAXIMUM: u32 = 0x009e_32d0;
const NODE_HAS_FLAG_100000: u32 = 0x0055_2470;
const IS_KIND_OF: u32 = 0x0045_bad0;
const KIND_OF_FACE_DATA: u32 = 0x0120_2828;
const KIND_OF_PORTAL: u32 = 0x0120_30b8;
const PORTAL_FACE_QUERY: u32 = 0x00c4_f7b0;
const POINTER_TARGET_OF: u32 = 0x0043_b230;
const NODE_FLAG_2: u32 = 0x004f_0140;
const SET_FLAG_ON_NODE: u32 = 0x0049_02f0;
const SET_BONE_LOD: u32 = 0x00c5_2960;
const DISTANCE_FACTOR: u32 = 0x0103_5710;
const PROCESS_SLOT_LOD_NODE_A: u32 = 0x790;
const PROCESS_SLOT_LOD_NODE_B: u32 = 0x798;
const PROCESS_SLOT_LOD_NODE_C: u32 = 0x7a0;
/// `HighProcess` fields: `pBoneLOD` (+0x2e4), `iLastBoneLOD` (+0x2e8) and
/// the byte at +0x364 (`bDialoguewithPlayer`).
const BONE_LOD_CONTROLLER: u32 = 0x2e4;
const LAST_BONE_LOD: u32 = 0x2e8;
const DIALOGUE_WITH_PLAYER: u32 = 0x364;

// Translated from 008e5730 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: works out the bone level of detail of `actor`
/// (`NiBSBoneLODController::SetBoneLOD`) and applies it.
///
/// The distance from the camera object (`011deb7c`) to the actor, divided by
/// the actor's scale, times the factor `01035710` and a float of the face
/// animation data, divided by the LOD table entry 3 times the integer setting
/// `011ccf88`, truncated, is the level; at or above the controller's maximum
/// (`+0x38`) it is -1, and it is -1 for a node with the flag 0x100000 unless
/// the process is in a dialogue (+0x364). For other actors than the player a
/// face node or portal object (`0045bad0`) of the node's 0x18 word that
/// is not "1" also gives -1 outside a dialogue. When the level differs from
/// `iLastBoneLOD` (+0x2e8) the controller is told and the flag 2 of the
/// process's three nodes (virtual `0x790`, `0x798`, `0x7a0`) is set on
/// two of them.
pub fn fn_008e5730(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let scale = e.call(REFERENCE_GET_SCALE, &args![actor]).f32();
    let level = e.with_stack(12, |e, scaled| {
        e.with_stack(12, |e, difference| {
            let camera = e.call(GLOBAL_OBJECT_POINTER_CALL, &args![]).u32();
            let child = e.call(OBJECT_CHILD_AT_ZERO, &args![camera]).u32();
            let camera_position = e.call(OBJECT_POSITION_ADDRESS, &args![child]).u32();
            let actor_position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
            e.call(
                POINT_DIFFERENCE,
                &args![actor_position, difference, camera_position],
            );
            // The two words pushed before the camera lookups stay on the
            // stack: `(out, scale)` for the division by the scale.
            e.call(POINT_DIVIDE_BY_SCALE, &args![difference, scaled, scale]);
            let length = e.call(POINT_LENGTH_FOR_LOD, &args![scaled]).f32();
            let magnitude = e.call(FLOAT_ABSOLUTE_FOR_LOD, &args![length]).f32();
            let distance = f64::from(magnitude) * e.global::<f64>(DISTANCE_FACTOR);
            let camera = e.call(GLOBAL_OBJECT_POINTER_CALL, &args![]).u32();
            let face = e.call(FACE_NODE_ANIMATION_DATA, &args![camera]).u32();
            let face_float = e.call(FLOAT_AT_0X110, &args![face]).f32();
            let numerator = f64::from(face_float) * distance;
            let setting = e
                .call(SETTING_VALUE_ADDRESS, &args![BONE_LOD_SCALE_SETTING])
                .u32();
            let setting_value = f64::from(e.mem.i32(setting));
            let table = e.call(LOD_TABLE_ENTRY, &args![3u32]).f32();
            let denominator = f64::from(table) * setting_value;
            e.call(FLOAT_TO_INT, &args![numerator / denominator]).i32()
        })
    });
    let mut level = level;
    let controller = e.mem.u32(process + BONE_LOD_CONTROLLER);
    let maximum = e.call(BONE_LOD_MAXIMUM, &args![controller]).i32();
    let dialogue = e.mem.u8(process + DIALOGUE_WITH_PLAYER) != 0;
    let mut settled = false;
    if level >= maximum {
        level = -1;
        settled = true;
    } else {
        let node = e.vcall(a, ACTOR_SLOT_NODE_FOR_FOLLOW, &args![]).u32();
        if node != 0 {
            let node = e.vcall(a, ACTOR_SLOT_NODE_FOR_FOLLOW, &args![]).u32();
            if e.call(NODE_HAS_FLAG_100000, &args![node]).bool() && !dialogue {
                level = -1;
                settled = true;
            }
        }
    }
    if !settled && a != e.global::<u32>(PLAYER_POINTER) {
        let mut in_view = true;
        let node = e.vcall(a, ACTOR_SLOT_NODE_FOR_FOLLOW, &args![]).u32();
        let object = e.call(PACKAGE_WORD_AT_0X18, &args![node]).u32();
        if object != 0 {
            if e.call(IS_KIND_OF, &args![KIND_OF_FACE_DATA, object]).bool() {
                let data = e.call(FACE_NODE_ANIMATION_DATA, &args![object]).u32();
                let target = e.call(POINTER_TARGET_OF, &args![data]).u32();
                in_view = e.call(WORD_AT_8, &args![target]).i32() == 1;
            } else if e.call(IS_KIND_OF, &args![KIND_OF_PORTAL, object]).bool() {
                let inner = e.call(FOLLOW_TARGET_OF_SLOT, &args![object]).u32();
                in_view = e.call(PORTAL_FACE_QUERY, &args![inner]).i32() == 1;
            }
        }
        if !in_view && !dialogue {
            level = -1;
        }
    }
    if level != e.mem.i32(process + LAST_BONE_LOD) {
        let controller = e.mem.u32(process + BONE_LOD_CONTROLLER);
        e.call(SET_BONE_LOD, &args![controller, level as u32]);
        let first = e.vcall(process, PROCESS_SLOT_LOD_NODE_A, &args![]).u32();
        let second = e.vcall(process, PROCESS_SLOT_LOD_NODE_B, &args![]).u32();
        let third = e.vcall(process, PROCESS_SLOT_LOD_NODE_C, &args![]).u32();
        if third != 0 && first != 0 && second != 0 {
            let word = e.call(PACKAGE_WORD_AT_0X18, &args![third]).u32();
            let flag = e.call(NODE_FLAG_2, &args![word]).bool();
            e.call(SET_FLAG_ON_NODE, &args![first, u32::from(flag)]);
            e.call(SET_FLAG_ON_NODE, &args![second, u32::from(flag)]);
        }
        e.mem.set_u32(process + LAST_BONE_LOD, level as u32);
    }
}

/// `TESObjectREFR::GetLinkedDoorTeleportPosition` (Xbox PDB, `00568fa0`): the
/// position to go to for the door, as a pointer.
const LINKED_DOOR_TELEPORT_POSITION: u32 = 0x0056_8fa0;
/// `Actor::PayGoldToActor` (Xbox PDB, `008924e0`): `(this, receiver, amount)`.
const ACTOR_PAY_GOLD_TO_ACTOR: u32 = 0x0089_24e0;
/// `Script::SetActionFlag` (Xbox PDB, `005ac750`, cdecl): `(actor, owner
/// word, flag)`.
const SCRIPT_SET_ACTION_FLAG: u32 = 0x005a_c750;
/// `TESObjectREFR::Activate` (Xbox PDB, `00573170`): `(this, activator, 0,
/// form, 1)`.
const REFERENCE_ACTIVATE: u32 = 0x0057_3170;
/// `ExtraDataList::GetLevCreaOriginalBase` (Xbox PDB, `004216f0`) and the
/// call that stores the base form in the list (`00419700`).
const EXTRA_LEV_CREA_ORIGINAL_BASE: u32 = 0x0042_16f0;
const EXTRA_SET_BASE_FORM: u32 = 0x0041_9700;
/// Package slot `0x144` `(actor, 0)` and actor slot `0x17c` (ten arguments).
const PACKAGE_SLOT_0X144: u32 = 0x144;
const ACTOR_SLOT_0X17C: u32 = 0x17c;
/// `ObjectstoAcquire` fields: the form (+4), the count (+0xc) and the flag
/// word (+0x14).
const ACQUIRE_COUNT: u32 = 0xc;
const ACQUIRE_ACTION_FLAG: u32 = 0x14;
const ACTION_FLAG_BUY: u32 = 0x4000;

// Translated from 008e59c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessBuyObject` (Xbox PDB): the actor buys the object of
/// the process' `pAcquireObject` (+0x64). Without a target (asked for with
/// `SetTargetForPackage`), or with one that has the flag 0x20 or 0x800, procedure
/// 1 is added and nothing else done; without an acquire object nothing is
/// done. The price is the form's value times the count.
///
/// The actor walks to the seller (the target, or the position of the linked
/// door `00568fa0` when the reference has the extra flag `00568e50`) until
/// it is within the setting `011cde98` of the door position, or the package
/// (slot `0x144`) says it is there. When it is, the buy happens (once
/// `00915ef0` is done or the actor is there): for a seller who is not an
/// actor, the owning actor is found (`GetActorRefInHigh`), the actor's base
/// form is stored in the reference's extra data, the owner's action flag is
/// set (0x4000) and the reference is activated by the actor; for an actor
/// seller the actor's slot `0x17c` is called. The buyer pays the price to the
/// seller when it is another actor. The acquire object is then deleted
/// (`007b3fa0`) and cleared.
pub fn high_process_process_buy_object(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if e.mem.u32(process + LOW_TARGET) == 0 {
        e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
    }
    let target = e.mem.u32(process + LOW_TARGET);
    if target == 0
        || e.call(REFERENCE_FLAG_20, &args![target]).bool()
        || e.call(REFERENCE_FLAG_800, &args![target]).bool()
    {
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
        return;
    }
    let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
    if entry == 0 {
        return;
    }
    let form_word = e.mem.u32(entry + 4);
    let value = e.call(FORM_VALUE, &args![form_word]).i32();
    let price = value.wrapping_mul(e.mem.i32(entry + ACQUIRE_COUNT));
    if e.mem.u32(process + LOW_TARGET) == 0 {
        let reference = e.mem.u32(entry);
        let form = e.call(REFERENCE_GET_FORM, &args![reference]).u32();
        if e.mem.u32(entry + 4) == form {
            let reference = e.mem.u32(entry);
            e.vcall(process, SLOT_SET_TARGET, &args![reference]);
            return;
        }
    }
    let target = e.mem.u32(process + LOW_TARGET);
    let door_flag = e.call(REFERENCE_HAS_EXTRA_FLAG, &args![target]).u32();
    let reached = if door_flag != 0 {
        let my_position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
        let distance = e.with_stack(12, |e, difference| {
            let door = e.call(LINKED_DOOR_TELEPORT_POSITION, &args![target]).u32();
            let out = e
                .call(POINT_DIFFERENCE, &args![door, difference, my_position])
                .u32();
            e.call(POINT_LENGTH, &args![out]).f64()
        });
        let setting = e
            .call(SETTING_VALUE_ADDRESS, &args![FOLLOW_RADIUS_SETTING])
            .u32();
        // `FCOMP`: reached unless the setting is below the distance (or the
        // values are unordered).
        let setting_value = f64::from(e.mem.i32(setting));
        !(setting_value < distance || distance.is_nan())
    } else {
        e.vcall(package, PACKAGE_SLOT_0X144, &args![actor, 0u32])
            .bool()
    };
    if !reached
        && !e
            .call(PROCESS_WANTS_TO_FINISH_SLEEP, &args![this, actor])
            .bool()
    {
        if e.vcall(a, ACTOR_SLOT_STATE, &args![]).u32() != 0 {
            e.vcall(a, ACTOR_SLOT_0X418, &args![]);
            return;
        }
        if e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
            let door_flag = e.call(REFERENCE_HAS_EXTRA_FLAG, &args![target]).u32();
            // `GetWorldSpace` and `008d6f30` take no arguments; the float
            // and 0 pushed before them are the last two arguments of the goal.
            let world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![target]).u32();
            let cell = e.call(REFERENCE_PATHING_CELL, &args![target]).u32();
            let goal = if door_flag == 0 {
                e.vcall(target, ACTOR_SLOT_POSITION, &args![]).u32()
            } else {
                e.call(LINKED_DOOR_TELEPORT_POSITION, &args![target]).u32()
            };
            let set = e
                .call(
                    ACTOR_SET_PATHFINDING_GOAL,
                    &args![actor, goal, cell, world_space, 0.0f32, 0u32],
                )
                .bool();
            if !set {
                return;
            }
        }
        if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
            e.vcall(
                process,
                SLOT_SET_ACTORS_ANIMATION,
                &args![actor, 0x101u32, 1u32],
            );
        }
        return;
    }
    if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
        e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
    }
    if !e
        .call(PROCESS_WANTS_TO_FINISH_SLEEP, &args![this, actor])
        .bool()
        || reached
    {
        let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
        let reference = e.mem.u32(entry);
        let mut seller = 0u32;
        if !e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
            let owner = e.call(REFERENCE_GET_OWNER, &args![reference]).u32();
            let mut owner_form = 0u32;
            if owner != 0 && e.call(FORM_TYPE_BYTE, &args![owner]).i32() == 0x2a {
                owner_form = e.call(REFERENCE_GET_OWNER, &args![reference]).u32();
            }
            if owner_form != 0 {
                seller = e
                    .call(
                        PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH,
                        &args![PROCESS_LISTS_INSTANCE, owner_form, 0u32],
                    )
                    .u32();
            }
            let extra = e.call(REFERENCE_EXTRA_DATA, &args![actor]).u32();
            let mut base = e.call(EXTRA_LEV_CREA_ORIGINAL_BASE, &args![extra]).u32();
            if base == 0 {
                base = e.call(REFERENCE_GET_FORM, &args![actor]).u32();
            }
            let reference = e.mem.u32(e.mem.u32(process + LOW_ACQUIRE_OBJECT));
            let reference_extra = e.call(REFERENCE_EXTRA_DATA, &args![reference]).u32();
            e.call(EXTRA_SET_BASE_FORM, &args![reference_extra, base]);
            let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
            let flag = e.mem.u32(entry + ACQUIRE_ACTION_FLAG);
            e.call(
                SCRIPT_SET_ACTION_FLAG,
                &args![seller, flag, ACTION_FLAG_BUY],
            );
            let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
            let form = e.mem.u32(entry + 4);
            let reference = e.mem.u32(entry);
            e.call(
                REFERENCE_ACTIVATE,
                &args![reference, actor, 0u32, form, 1u32],
            );
        } else {
            seller = e.mem.u32(process + LOW_TARGET);
            let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
            let flag = e.mem.u32(entry + ACQUIRE_ACTION_FLAG);
            e.call(
                SCRIPT_SET_ACTION_FLAG,
                &args![seller, flag, ACTION_FLAG_BUY],
            );
            let form = e.mem.u32(entry + 4);
            e.vcall(
                seller,
                ACTOR_SLOT_0X17C,
                &args![form, 0u32, 1u32, 0u32, 0u32, actor, 0u32, 0u32, 1u32, 0u32],
            );
        }
        if seller != 0 && seller != a {
            e.call(ACTOR_PAY_GOLD_TO_ACTOR, &args![actor, seller, price as u32]);
        }
    } else {
        e.vcall(process, SLOT_SET_TARGET, &args![0u32]);
    }
    let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
    if entry != 0 {
        e.call(ACQUIRE_OBJECT_DELETE, &args![entry, 1u32]);
    }
    e.mem.set_u32(process + LOW_ACQUIRE_OBJECT, 0);
}

/// `FaderManager::GetFaderAlpha` (Xbox PDB, `007014e0`: `(this, index)`,
/// `ST0`) on the manager at `011d8804`.
const FADER_GET_ALPHA: u32 = 0x0070_14e0;
const FADER_MANAGER: u32 = 0x011d_8804;
/// `00608d80` and `008ace90`: tests of the detecting actor that end the
/// detection (`this` is the actor).
const ACTOR_DETECTION_BLOCK_A: u32 = 0x0060_8d80;
const ACTOR_DETECTION_BLOCK_B: u32 = 0x008a_ce90;
/// The distance (8192.0, a `double` at `01084d28`) from the player beyond
/// which an actor does not detect, and the byte `011f1221` that turns
/// detection on.
const DETECTION_DISTANCE_LIMIT: u32 = 0x0108_4d28;
const DETECTION_ENABLED: u32 = 0x011f_1221;
/// The sound handle at `011e0244` that `RunDetection` destroys at its start
/// and end (`00483710` is [`SOUND_HANDLE_DESTRUCTOR`]) and the list at
/// `011e0280` it clears.
const DETECTION_SOUND_HANDLE: u32 = 0x011e_0244;
const DETECTION_SCRATCH_LIST: u32 = 0x011e_0280;
/// Settings: the detection radius (`011cd7d8`, float), the threshold above
/// which an actor counts as detected (`011cdf10`, float), the view cone angle
/// (`011cd668`), the level cap (`011cd45c`), the level kept for far actors
/// (`011cf788`, integer) and the detection timer (`011d0b08`).
const DETECTION_RADIUS_SETTING: u32 = 0x011c_d7d8;
const DETECTION_THRESHOLD_SETTING: u32 = 0x011c_df10;
const DETECTION_VIEW_CONE_SETTING: u32 = 0x011c_d668;
const DETECTION_LEVEL_CAP_SETTING: u32 = 0x011c_d45c;
const DETECTION_FAR_LEVEL_SETTING: u32 = 0x011c_f788;
const DETECTION_TIMER_SETTING: u32 = 0x011d_0b08;
/// A `double` at `01011590` (the distance below which an actor is sensed at
/// once).
const DETECTION_CLOSE_DISTANCE: u32 = 0x0101_1590;
/// `Actor::GetDetectionLevelAgainstActor` (Xbox PDB, `008a0d10`): `(this, 1,
/// target, &flag, sneaking, 0 or creature flag, combat flag, &flag)`.
const ACTOR_DETECTION_LEVEL: u32 = 0x008a_0d10;
/// `Actor::IsInCombatWithActor` (Xbox PDB, `008bc700`) and
/// `Actor::GetShouldAttackActor` (Xbox PDB, `008b06d0`: `(this, actor, 0,
/// &out, 1)`).
const ACTOR_IS_IN_COMBAT_WITH: u32 = 0x008b_c700;
const ACTOR_GET_SHOULD_ATTACK: u32 = 0x008b_06d0;
/// `HighProcess::ShouldRunCombatDetection` (Xbox PDB, `008ffd10`) and
/// `ShouldRunCombatDetectionEventCheck` (Xbox PDB, `008ffdc0`): `(this,
/// actor, other)`.
const PROCESS_SHOULD_RUN_COMBAT_DETECTION: u32 = 0x008f_fd10;
const PROCESS_SHOULD_RUN_COMBAT_DETECTION_EVENT: u32 = 0x008f_fdc0;
/// `Actor::IsPointInViewCone` (Xbox PDB, `0088c570`: `(this, point,
/// angle)`) and the visibility test `0088c240` (`(this, point, 1, 1)`).
const ACTOR_IS_POINT_IN_VIEW_CONE: u32 = 0x0088_c570;
const ACTOR_POINT_VISIBLE: u32 = 0x0088_c240;
/// `Actor::GetAlert` (Xbox PDB, `008a5e80`), `00579670` (whether the
/// actor's current weapon is a mine: the word at +0x1ac is 9), the distance
/// of an actor to a point (`00572380`, `ST0`), `ActorValueOwner::
/// GetClampedActorValue` (Xbox PDB, `0066ef20`: `(this, 0x2a)`) and the
/// stealth formula `00642bc0` (cdecl, eleven arguments).
const ACTOR_GET_ALERT: u32 = 0x008a_5e80;
const ACTOR_WEAPON_IS_MINE: u32 = 0x0057_9670;
const ACTOR_DISTANCE_TO_POINT: u32 = 0x0057_2380;
const ACTOR_VALUE_OWNER_CLAMPED: u32 = 0x0066_ef20;
const STEALTH_FORMULA: u32 = 0x0064_2bc0;
/// `PlayerCharacter::IsPlayerCharacterInCombat` (Xbox PDB, `00953c50`:
/// `(this, &out)`), `PlayerCharacter` `008c8bb0` (one argument) and the
/// high process list helpers: `BaseProcess::GetActorPackageThatIsRunning`
/// is [`PROCESS_LISTS_INSTANCE`]'s list getter (`00717e50`), the list's
/// size (`005be5c0`) and element (`00968670`).
const PLAYER_IS_IN_COMBAT: u32 = 0x0095_3c50;
const PLAYER_WAS_SEEN: u32 = 0x008c_8bb0;
const HIGH_ACTORS_LIST: u32 = 0x0071_7e50;
const HIGH_ACTORS_COUNT: u32 = 0x005b_e5c0;
const HIGH_ACTORS_AT: u32 = 0x0096_8670;
/// The time global `435dd0` (a float), and its store into a stack slot
/// (`00435de0`).
const GAME_TIME_FLOAT: u32 = 0x0043_5dd0;
/// Process slots: `InsertIntoDetectionList` (`0xf0`), `GetActorsDetectionEvent`
/// (`0xf8`), `RemoveDetectionActor` (`0x2c4`), `GetDetectionState` (`0x504`),
/// `GetCommandingActor` (`0x52c`), `GetActorLightLevel` (`0x734`).
const SLOT_INSERT_INTO_DETECTION_LIST: u32 = 0xf0;
const SLOT_GET_ACTORS_DETECTION_EVENT: u32 = 0xf8;
const SLOT_REMOVE_DETECTION_ACTOR: u32 = 0x2c4;
const SLOT_GET_DETECTION_STATE: u32 = 0x504;
const SLOT_GET_COMMANDING_ACTOR: u32 = 0x52c;
const SLOT_GET_ACTOR_LIGHT_LEVEL: u32 = 0x734;
const SLOT_GET_INSTANCE_DATA_RUNNING_FOR_DETECTION: u32 = 0x274;
/// `HighProcess` fields: `bCheckDeadTalk` (+0x2c6), `plastDetected` (+0x2a4),
/// `fDetectionTimer` (+0x2f8), `bEvaluateDetection` (+0x270) and
/// `iDetectionCounter` (+0x3d8). The two lists cleared at the start are the
/// words at +0x38c and +0x394.
const DETECTION_CHECK_DEAD_TALK: u32 = 0x2c6;
const DETECTION_LAST_DETECTED: u32 = 0x2a4;
const DETECTION_TIMER: u32 = 0x2f8;
const DETECTION_EVALUATE: u32 = 0x270;
const DETECTION_COUNTER: u32 = 0x3d8;
const DETECTION_LIST_A: u32 = 0x38c;
const DETECTION_LIST_B: u32 = 0x394;
/// `DetectionState` fields (see [`DetectionState`]): level +8, line of
/// sight byte +0x1e, raw level +0x20, last position +0xc, time stamp +0x18.
const DETECTION_STATE_LEVEL: u32 = 0x08;
const DETECTION_STATE_POSITION: u32 = 0x0c;
const DETECTION_STATE_TIME: u32 = 0x18;
const DETECTION_STATE_LINE_OF_SIGHT: u32 = 0x1e;
const DETECTION_STATE_RAW: u32 = 0x20;

/// `GetShouldAttackActor` the way `RunDetection` asks it: an actor that is
/// in combat with `other` always counts; otherwise
/// `Actor::GetShouldAttackActor(other, 0, &out, 1)` decides.
fn detection_should_attack(
    e: &mut Engine,
    me: u32,
    other: u32,
    combat_flag: bool,
    out: u32,
) -> bool {
    if combat_flag && e.call(ACTOR_IS_IN_COMBAT_WITH, &args![me, other]).bool() {
        return true;
    }
    e.call(ACTOR_GET_SHOULD_ATTACK, &args![me, other, 0u32, out, 1u32])
        .bool()
}

/// What both halves of `RunDetection` do when the detection level of `other`
/// is below the threshold but `other` has a detection event of the process:
/// works out the level the event gives (the stealth formula `00642bc0`) and,
/// if it is above `level` and the cap setting is lower than it, raises
/// `level` to the cap plus one. Returns the formula's result.
fn detection_event_level(e: &mut Engine, me: u32, other: u32, level: &mut i32) -> i32 {
    let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
    let event = e
        .vcall(
            other_process,
            SLOT_GET_ACTORS_DETECTION_EVENT,
            &args![other],
        )
        .u32();
    let my_process = e.call(ACTOR_PROCESS, &args![me]).u32();
    let light = e
        .vcall(my_process, SLOT_GET_ACTOR_LIGHT_LEVEL, &args![])
        .f64();
    let light_level = e.call(FLOAT_TO_INT, &args![light]).i32();
    let mut event_space = true;
    if event != 0 && e.mem.u32(event + 0x18) != 0 {
        let reference = e.mem.u32(event + 0x18);
        if e.call(REFERENCE_PATHING_CELL, &args![reference]).u32() != 0 {
            let reference = e.mem.u32(event + 0x18);
            let cell = e.call(REFERENCE_PATHING_CELL, &args![reference]).u32();
            event_space = !e.call(CELL_FLAG_TEST, &args![cell]).bool();
        }
    }
    let visible = e
        .call(ACTOR_POINT_VISIBLE, &args![me, event + 4, 1u32, 1u32])
        .u8();
    let mut in_cone = false;
    if visible != 0 {
        let setting = e
            .call(SETTING_VALUE, &args![DETECTION_VIEW_CONE_SETTING])
            .u32();
        let angle = e.mem.f32(setting);
        if e.call(ACTOR_IS_POINT_IN_VIEW_CONE, &args![me, event + 4, angle])
            .bool()
        {
            in_cone = true;
        }
    }
    let creature_like = e.call(ACTOR_CREATURE_LIKE, &args![me]).u8();
    let alert = e.call(ACTOR_GET_ALERT, &args![me]).u8();
    let is_mine = e.call(ACTOR_WEAPON_IS_MINE, &args![me]).u8();
    let distance = e.call(ACTOR_DISTANCE_TO_POINT, &args![me, event + 4]).f32();
    let clamped = e
        .call(ACTOR_VALUE_OWNER_CLAMPED, &args![me + 0xa4, 0x2au32])
        .u32();
    let first = e.mem.u32(event);
    let raw = e
        .call(
            STEALTH_FORMULA,
            &args![
                clamped,
                u32::from(visible),
                distance,
                0u32,
                light_level,
                u32::from(is_mine),
                u32::from(alert),
                u32::from(creature_like),
                first,
                u32::from(in_cone),
                u32::from(event_space)
            ],
        )
        .i32();
    if raw > *level {
        let setting = e
            .call(SETTING_VALUE, &args![DETECTION_LEVEL_CAP_SETTING])
            .u32();
        // `FCOMP`: only an ordered "cap below the raw level" counts.
        if e.mem.f32(setting) < raw as f32 {
            let setting = e
                .call(SETTING_VALUE, &args![DETECTION_LEVEL_CAP_SETTING])
                .u32();
            let cap = f64::from(e.mem.f32(setting));
            *level = e.call(FLOAT_TO_INT, &args![cap]).i32().wrapping_add(1);
        }
    }
    raw
}

/// The detection radius setting, doubled for an actor outside an interior
/// cell (`RunDetection` reads it twice).
fn detection_radius(e: &mut Engine, me: u32) -> f32 {
    let setting = e
        .call(SETTING_VALUE, &args![DETECTION_RADIUS_SETTING])
        .u32();
    let radius = e.mem.f32(setting);
    detection_radius_for_cell(e, me, radius)
}

fn detection_radius_for_cell(e: &mut Engine, me: u32, radius: f32) -> f32 {
    let cell = e.call(REFERENCE_PATHING_CELL, &args![me]).u32();
    if cell == 0 || !e.call(CELL_FLAG_TEST, &args![cell]).bool() {
        (f64::from(radius) + f64::from(radius)) as f32
    } else {
        radius
    }
}

/// The detection state `RunDetection` leaves: the level, the line of sight
/// byte and the raw level.
fn detection_state_set(e: &mut Engine, state: u32, level: i32, sight: u8, raw: i32) {
    e.mem.set_i32(state + DETECTION_STATE_LEVEL, level);
    e.mem.set_u8(state + DETECTION_STATE_LINE_OF_SIGHT, sight);
    e.mem.set_i32(state + DETECTION_STATE_RAW, raw);
}

/// The "level is above the threshold" test of `RunDetection`: `FCOMP` of the
/// setting against the integer level, true when the setting is below it.
fn detection_threshold_below(e: &mut Engine, level: i32) -> bool {
    let setting = e
        .call(SETTING_VALUE, &args![DETECTION_THRESHOLD_SETTING])
        .u32();
    f64::from(e.mem.f32(setting)) < f64::from(level)
}

/// `RunDetection`'s player half (`008e42e7` to `008e4760`): the detection of
/// `other` (the player) by `me`.
fn detect_player(e: &mut Engine, this: Ptr<HighProcess>, me: u32, other: u32, combat_flag: bool) {
    let process = this.addr();
    let player = other;
    let creature = e.call(ACTOR_CREATURE_LIKE, &args![other]).u8();
    // v23: line of sight byte; v21: combat byte; v28: should-attack out word.
    e.with_stack(16, |e, scratch| {
        let sight = scratch.addr();
        let combat_byte = scratch.addr() + 1;
        let attack_word = scratch.addr() + 4;
        e.mem.set_u8(combat_byte, u8::from(combat_flag));
        e.mem.set_u32(attack_word, 0);
        let should_attack = detection_should_attack(e, me, other, combat_flag, attack_word);
        if combat_flag
            && !e
                .call(PROCESS_SHOULD_RUN_COMBAT_DETECTION, &args![this, me, other])
                .bool()
        {
            e.vcall(process, SLOT_REMOVE_DETECTION_ACTOR, &args![player, 0u32]);
            let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
            e.vcall(other_process, SLOT_REMOVE_DETECTION_ACTOR, &args![me, 1u32]);
            return;
        }
        let mut level = e
            .call(
                ACTOR_DETECTION_LEVEL,
                &args![
                    me,
                    1u32,
                    other,
                    sight,
                    u32::from(creature),
                    0u32,
                    u32::from(combat_flag),
                    combat_byte
                ],
            )
            .i32();
        let mut raw = -100;
        if !detection_threshold_below(e, level) {
            let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
            let event = e
                .vcall(
                    other_process,
                    SLOT_GET_ACTORS_DETECTION_EVENT,
                    &args![other],
                )
                .u32();
            if event != 0
                && should_attack
                && (!combat_flag
                    || e.call(
                        PROCESS_SHOULD_RUN_COMBAT_DETECTION_EVENT,
                        &args![this, me, other],
                    )
                    .bool())
            {
                raw = detection_event_level(e, me, other, &mut level);
            }
        }
        let position = e.vcall(other, ACTOR_SLOT_POSITION, &args![]).u32();
        let position_words = [
            e.mem.u32(position),
            e.mem.u32(position + 4),
            e.mem.u32(position + 8),
        ];
        let state = e
            .vcall(
                process,
                SLOT_INSERT_INTO_DETECTION_LIST,
                &args![
                    other,
                    0u32,
                    u32::from(e.mem.u8(sight)),
                    level,
                    0u32,
                    u32::from(e.mem.u8(combat_byte)),
                    u32::from(combat_flag)
                ],
            )
            .u32();
        if level > 0 {
            let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
            e.vcall(
                other_process,
                SLOT_INSERT_INTO_DETECTION_LIST,
                &args![
                    me,
                    0u32,
                    u32::from(e.mem.u8(sight)),
                    level,
                    1u32,
                    u32::from(e.mem.u8(combat_byte)),
                    u32::from(creature)
                ],
            );
        } else {
            let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
            e.vcall(other_process, SLOT_REMOVE_DETECTION_ACTOR, &args![me, 1u32]);
        }
        let sight_byte = e.mem.u8(sight);
        detection_state_set(e, state, level, sight_byte, raw);
        if level > 0 {
            for (index, word) in position_words.iter().enumerate() {
                e.mem
                    .set_u32(state + DETECTION_STATE_POSITION + index as u32 * 4, *word);
            }
            let time = e.call(GAME_TIME_FLOAT, &args![]).f32();
            e.mem.set_f32(state + DETECTION_STATE_TIME, time);
            if should_attack {
                e.call(PLAYER_WAS_SEEN, &args![player, 1u32]);
            }
        }
    });
}

/// `RunDetection`'s loop body (`008e4841` to `008e4da5`) for one high actor
/// `other`: within the radius and 8192 of the player the detection level is
/// worked out and stored; otherwise the actor keeps the far level.
fn detect_loop_actor(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    me: u32,
    other: u32,
    combat_flag: bool,
    radius: f32,
    player: u32,
) {
    let process = this.addr();
    let distance = e
        .call(DISTANCE_FROM_REFERENCE, &args![me, other, 0u32, 0u32])
        .f32();
    if combat_flag
        && me != player
        && !e
            .call(PROCESS_SHOULD_RUN_COMBAT_DETECTION, &args![this, me, other])
            .bool()
    {
        return;
    }
    let mut far = radius < distance;
    if !far {
        let from_player = e
            .call(DISTANCE_FROM_REFERENCE, &args![other, player, 0u32, 0u32])
            .f64();
        // `FCOMP`: "above the limit or unordered" is far.
        far = !(from_player <= e.global::<f64>(DETECTION_DISTANCE_LIMIT));
    }
    if far {
        let state = e
            .vcall(process, SLOT_GET_DETECTION_STATE, &args![other, 0u32])
            .u32();
        if state == 0 {
            let setting = e
                .call(SETTING_VALUE_ADDRESS, &args![DETECTION_FAR_LEVEL_SETTING])
                .u32();
            let level = e.mem.u32(setting);
            e.vcall(
                process,
                SLOT_INSERT_INTO_DETECTION_LIST,
                &args![other, 0u32, 0u32, level, 0u32, 0u32, u32::from(combat_flag)],
            );
        } else {
            let setting = e
                .call(SETTING_VALUE_ADDRESS, &args![DETECTION_FAR_LEVEL_SETTING])
                .u32();
            let level = e.mem.u32(setting);
            e.mem.set_u32(state + DETECTION_STATE_LEVEL, level);
        }
        let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
        e.vcall(other_process, SLOT_REMOVE_DETECTION_ACTOR, &args![me, 1u32]);
        return;
    }
    e.with_stack(16, |e, scratch| {
        let sight = scratch.addr();
        let combat_byte = scratch.addr() + 1;
        let attack_word = scratch.addr() + 4;
        let player_flag = scratch.addr() + 2;
        e.mem.set_u8(sight, 0);
        e.mem.set_u8(player_flag, 0);
        let own_state_flag: u8;
        if other == player {
            own_state_flag = e
                .call(PLAYER_IS_IN_COMBAT, &args![player, player_flag])
                .u8();
        } else {
            own_state_flag = e
                .vcall(
                    other,
                    SLOT_GET_INSTANCE_DATA_RUNNING_FOR_DETECTION,
                    &args![1u32],
                )
                .u8();
        }
        e.mem.set_u8(combat_byte, u8::from(combat_flag));
        let mut raw = -100;
        let mut level;
        if f64::from(distance) <= e.global::<f64>(DETECTION_CLOSE_DISTANCE) {
            level = 100;
            e.mem.set_u8(sight, 1);
            e.mem.set_u8(combat_byte, 1);
        } else {
            level = e
                .call(
                    ACTOR_DETECTION_LEVEL,
                    &args![
                        me,
                        1u32,
                        other,
                        sight,
                        u32::from(own_state_flag),
                        0u32,
                        0u32,
                        combat_byte
                    ],
                )
                .i32();
        }
        e.mem.set_u32(attack_word, 0);
        if !detection_threshold_below(e, level) {
            let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
            let event = e
                .vcall(
                    other_process,
                    SLOT_GET_ACTORS_DETECTION_EVENT,
                    &args![other],
                )
                .u32();
            if event != 0 {
                let should_attack = detection_should_attack(e, me, other, combat_flag, attack_word);
                if should_attack
                    && (!combat_flag
                        || e.call(
                            PROCESS_SHOULD_RUN_COMBAT_DETECTION_EVENT,
                            &args![this, me, other],
                        )
                        .bool())
                {
                    raw = detection_event_level(e, me, other, &mut level);
                }
            }
        }
        let mut kind = 0u32;
        if detection_threshold_below(e, level) {
            kind = 3;
        }
        let state = e
            .vcall(
                process,
                SLOT_INSERT_INTO_DETECTION_LIST,
                &args![
                    other,
                    kind,
                    u32::from(e.mem.u8(sight)),
                    level,
                    0u32,
                    u32::from(e.mem.u8(combat_byte)),
                    u32::from(combat_flag)
                ],
            )
            .u32();
        if level > 0 {
            let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
            e.vcall(
                other_process,
                SLOT_INSERT_INTO_DETECTION_LIST,
                &args![
                    me,
                    kind,
                    u32::from(e.mem.u8(sight)),
                    level,
                    1u32,
                    u32::from(e.mem.u8(combat_byte)),
                    u32::from(own_state_flag)
                ],
            );
        } else {
            let other_process = e.call(ACTOR_PROCESS, &args![other]).u32();
            e.vcall(other_process, SLOT_REMOVE_DETECTION_ACTOR, &args![me, 1u32]);
        }
        if state != 0 {
            let sight_byte = e.mem.u8(sight);
            detection_state_set(e, state, level, sight_byte, raw);
        }
    });
}

// Translated from 008e40d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::RunDetection` (Xbox PDB): runs the detection of `me` (the
/// process' actor) against the player and every high actor, and stores the
/// results in the process' detection list. `delta` is added to the detection
/// timer for an actor that is not creature-like (the code's `00493bb0`).
///
/// The two lists at +0x38c/+0x394 are cleared first. It returns while the
/// screen is faded (`FaderManager::GetFaderAlpha(1)` above 0), when the actor
/// is paused (actor slot `0x230`), when `00608d80` and `008ace90` both
/// hold, when the player is 8192 or more away or the byte `011f1221` is off.
/// Otherwise the sound handle at `011e0244` is destroyed, `bCheckDeadTalk`
/// cleared and the scratch list at `011e0280` cleared.
///
/// The player half ([`detect_player`], for a player that is not `me`): the
/// detection level (`GetDetectionLevelAgainstActor`) and the combat state
/// decide whether the player is detected; when it is not, an event of the
/// process can still give it a level ([`detection_event_level`]). The result
/// goes to the process (`InsertIntoDetectionList`, slot `0xf0`) and to the
/// player's own process (this actor is added to who detects it, or removed
/// with slot `0x2c4`); the state gets the level, the line of sight flag and
/// the raw level and, for a positive level, the player's position and the
/// time. The commanding actors (slot `0x52c`) are looked up first but `me`
/// and the target are put back afterwards, so only the calls remain.
///
/// The loop ([`detect_loop_actor`]) goes over the high actors that are not
/// flagged 0x800, are actors and are not `me`; a creature-like `me` other
/// than the player skips those `ShouldRunCombatDetection` refuses. At the
/// end the detection timer, the evaluate flag and the counter (modulo 12)
/// are set.
pub fn high_process_run_detection(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, delta: f32) {
    let process = this.addr();
    let me = actor.addr();
    e.call(LIST_CLEAR, &args![process + DETECTION_LIST_A]);
    e.call(LIST_CLEAR, &args![process + DETECTION_LIST_B]);
    let fader = e.global::<u32>(FADER_MANAGER);
    let alpha = e.call(FADER_GET_ALPHA, &args![fader, 1u32]).f64();
    // `FCOMP` against 0.0: continues for "at most 0" or unordered.
    if alpha > e.global::<f64>(ZERO_DOUBLE) {
        return;
    }
    if e.vcall(me, ACTOR_SLOT_PAUSED_A, &args![]).bool() {
        return;
    }
    if e.call(ACTOR_DETECTION_BLOCK_A, &args![me]).bool()
        && e.call(ACTOR_DETECTION_BLOCK_B, &args![me]).bool()
    {
        return;
    }
    let player = player_pointer(e);
    let distance_to_player = e
        .call(DISTANCE_FROM_REFERENCE, &args![me, player, 0u32, 0u32])
        .f64();
    // `FCOMP`: continues only below the limit.
    if !(distance_to_player < e.global::<f64>(DETECTION_DISTANCE_LIMIT)) {
        return;
    }
    if e.mem.u8(DETECTION_ENABLED) == 0 {
        return;
    }
    e.call(SOUND_HANDLE_DESTRUCTOR, &args![DETECTION_SOUND_HANDLE]);
    e.mem.set_u8(process + DETECTION_CHECK_DEAD_TALK, 0);
    e.call(LIST_CLEAR, &args![DETECTION_SCRATCH_LIST]);
    let combat_flag = e.call(ACTOR_CREATURE_LIKE, &args![me]).bool();
    let setting = e
        .call(SETTING_VALUE, &args![DETECTION_RADIUS_SETTING])
        .u32();
    let first_radius = e.mem.f32(setting);
    if !e.vcall(player, ACTOR_SLOT_0X22C, &args![0u32]).bool() {
        let other_process = e.call(ACTOR_PROCESS, &args![player]).u32();
        if other_process != 0 {
            let other_process = e.call(ACTOR_PROCESS, &args![player]).u32();
            if e.vcall(other_process, SLOT_GET_COMMANDING_ACTOR, &args![])
                .u32()
                != 0
            {
                let other_process = e.call(ACTOR_PROCESS, &args![player]).u32();
                e.vcall(other_process, SLOT_GET_COMMANDING_ACTOR, &args![]);
            }
        }
        if e.vcall(process, SLOT_GET_COMMANDING_ACTOR, &args![]).u32() != 0 {
            e.vcall(process, SLOT_GET_COMMANDING_ACTOR, &args![]);
        }
    }
    // The first radius (and its doubling) is not used: the loop reads the
    // setting again.
    let _ = detection_radius_for_cell(e, me, first_radius);
    if player != me {
        detect_player(e, this, me, player, combat_flag);
    }
    e.mem.set_u32(process + DETECTION_LAST_DETECTED, 0);
    let radius = detection_radius(e, me);
    let high_list = e
        .call(HIGH_ACTORS_LIST, &args![PROCESS_LISTS_INSTANCE])
        .u32();
    let mut index = 0u32;
    while e.mem.u32(process + DETECTION_LAST_DETECTED) == 0
        && index < e.call(HIGH_ACTORS_COUNT, &args![high_list, 0u32]).u32()
    {
        let found = e.call(HIGH_ACTORS_AT, &args![high_list, index]).u32();
        index += 1;
        if found != 0
            && !e.call(REFERENCE_FLAG_800, &args![found]).bool()
            && e.vcall(found, ACTOR_SLOT_IS_ACTOR, &args![]).bool()
            && found != me
        {
            detect_loop_actor(e, this, me, found, combat_flag, radius, player);
        }
    }
    e.call(LIST_CLEAR, &args![DETECTION_SCRATCH_LIST]);
    let setting = e.call(SETTING_VALUE, &args![DETECTION_TIMER_SETTING]).u32();
    let timer = e.mem.f32(setting);
    if !combat_flag {
        e.mem.set_f32(
            process + DETECTION_TIMER,
            (f64::from(timer) + f64::from(delta)) as f32,
        );
    } else {
        e.mem.set_f32(process + DETECTION_TIMER, timer);
    }
    e.mem.set_u8(process + DETECTION_EVALUATE, 1);
    let counter = e.mem.u32(process + DETECTION_COUNTER).wrapping_add(1);
    e.mem.set_u32(process + DETECTION_COUNTER, counter % 12);
    e.call(SOUND_HANDLE_DESTRUCTOR, &args![DETECTION_SOUND_HANDLE]);
}

/// `FleePackage` (Xbox PDB class; the package pointer is a `TESPackage` of
/// type 0x16). Its members as `ProcessFlee` calls them: `009f1830` (the
/// process), `009f1990` (`ShouldShutDown`), `009f1210`
/// (`GetClosestAvoidedRef(actor)`), `009f1440`, `009f1140`
/// (`FindTeleportDoor(actor, avoided)`), `00810530` (`(flag)`), `00898250`
/// (`(float)`), `006ca4e0` (`ST0`), `00574900`, `004fd400`, `005e3fc0`,
/// `008b6240`, `00994ef0` (`(0)`), `00500a20` (`(0)`) and `00477ba0`.
const FLEE_PACKAGE_UPDATE: u32 = 0x009f_1830;
const FLEE_PACKAGE_SHOULD_SHUT_DOWN: u32 = 0x009f_1990;
const FLEE_PACKAGE_CLOSEST_AVOIDED: u32 = 0x009f_1210;
const FLEE_PACKAGE_FLEE_RADIUS: u32 = 0x009f_1440;
const FLEE_PACKAGE_FIND_TELEPORT_DOOR: u32 = 0x009f_1140;
const FLEE_PACKAGE_SET_FLAG: u32 = 0x0081_0530;
const FLEE_PACKAGE_ADD_TIME: u32 = 0x0089_8250;
const FLEE_PACKAGE_TIME_VALUE: u32 = 0x006c_a4e0;
const FLEE_PACKAGE_FLAG_574900: u32 = 0x0057_4900;
const FLEE_PACKAGE_DOOR: u32 = 0x004f_d400;
const FLEE_PACKAGE_TARGET_REFERENCE: u32 = 0x005e_3fc0;
const FLEE_PACKAGE_FLAG_8B6240: u32 = 0x008b_6240;
const FLEE_PACKAGE_CLEAR_TARGET: u32 = 0x0099_4ef0;
const FLEE_PACKAGE_RESET_LOCATION: u32 = 0x0050_0a20;
const REFERENCE_FLAG_477BA0: u32 = 0x0047_7ba0;
/// `PathingRequestFlee::PathingRequestFlee` (Xbox PDB, `006e5390`), its
/// setters (`006e2b50`: a byte, `006d3b00` and `00507610`: floats,
/// `006d61e0`: a word, `006e2bb0`: a float), `Actor::SetPathfindingFlee`
/// (Xbox PDB, `008bb630`) and the request's destructor (`006dad70`).
const FLEE_REQUEST_CONSTRUCTOR: u32 = 0x006e_5390;
const FLEE_REQUEST_SET_BYTE: u32 = 0x006e_2b50;
const FLEE_REQUEST_SET_RADIUS: u32 = 0x006d_3b00;
const FLEE_REQUEST_SET_DISTANCE: u32 = 0x0050_7610;
const FLEE_REQUEST_SET_WORD: u32 = 0x006d_61e0;
const FLEE_REQUEST_SET_SPEED: u32 = 0x006e_2bb0;
const ACTOR_SET_PATHFINDING_FLEE: u32 = 0x008b_b630;
const FLEE_REQUEST_DESTRUCTOR: u32 = 0x006d_ad70;
const FLEE_REQUEST_SIZE: u32 = 0xbc;
/// `PathingLocation::PathingLocation_ov4` (Xbox PDB, `006dce10`).
const PATHING_LOCATION_FROM_TWO: u32 = 0x006d_ce10;
/// Further calls of the flee code: `Actor::IsPathValid` (`008b3960`), the
/// mover's distance traveled (`009dcc20`) and remaining (`009dcc50`),
/// `008b3b90`, `Actor::CanMove` (`008843a0`), the door position getter
/// `00717e50`, `PackageLocation::GetLocCell` (`0067f3e0`), the player's
/// greeting tests (`00969860`, [`fn_008defe0`]), the chance roll (`00476c00`,
/// `00944480`, cdecl `(0, 100)`), `CombatFormulas::GetRandomBetween`
/// (`006465f0`, cdecl `(low, high)`, `ST0`).
const ACTOR_IS_PATH_VALID: u32 = 0x008b_3960;
const MOVER_DISTANCE_TRAVELED: u32 = 0x009d_cc20;
const ACTOR_FLEE_ARRIVAL_CHECK: u32 = 0x008b_3b90;
const ACTOR_CAN_MOVE: u32 = 0x0088_43a0;
const DOOR_POSITION_OWNER: u32 = 0x0071_7e50;
const PACKAGE_LOCATION_GET_CELL: u32 = 0x0067_f3e0;
const PLAYER_GREET_TEST: u32 = 0x0096_9860;
const ROLL_ZERO_TO: u32 = 0x0047_6c00;
const ROLL_TO_CHANCE: u32 = 0x0094_4480;
const RANDOM_BETWEEN: u32 = 0x0064_65f0;
/// Process slot `0x204` `(1)`, slot `0x514` is [`SLOT_GET_GENERIC_LOCATION`].
const SLOT_PROCESS_0X204: u32 = 0x204;
/// Settings and constants: `011ce570`/`011cf998` and `011ce6c0`/`011cf70c`
/// (floats, interior/exterior), `011cd838` (an integer), `011cd18c`,
/// `011cd008` (floats); the doubles `0101a6b0`, `01020998`, `0102e430`,
/// `01088138` and the floats `01016970` and `01022958`.
const FLEE_RADIUS_INTERIOR: u32 = 0x011c_e570;
const FLEE_RADIUS_EXTERIOR: u32 = 0x011c_f998;
const FLEE_DISTANCE_INTERIOR: u32 = 0x011c_e6c0;
const FLEE_DISTANCE_EXTERIOR: u32 = 0x011c_f70c;
const FLEE_GREET_CHANCE_SETTING: u32 = 0x011c_d838;
const FLEE_TIMER_HIGH_SETTING: u32 = 0x011c_d18c;
const FLEE_TIMER_LOW_SETTING: u32 = 0x011c_d008;
const FLEE_TIME_FACTOR: u32 = 0x0101_a6b0;
const FLEE_TIME_LIMIT: u32 = 0x0102_0998;
const FLEE_MOVER_LIMIT: u32 = 0x0102_e430;
const FLEE_ARRIVED_DISTANCE: u32 = 0x0108_8138;
const FLEE_AVOID_DISTANCE_DEFAULT: u32 = 0x0101_6970;
const FLEE_DOOR_DISTANCE_DEFAULT: u32 = 0x0102_2958;
/// `HighProcess` fields: `fAvoidWaitTimer`'s neighbours `+0x298`
/// (`fIdleChatterTimer`), `+0x2b4` (`fPackageEvalTimer`) and `+0xa4`
/// (`LowProcess::fEssentialDownTimer`).
const FLEE_IDLE_CHATTER_TIMER: u32 = 0x298;
const FLEE_PACKAGE_EVAL_TIMER: u32 = 0x2b4;
const FLEE_ESSENTIAL_TIMER: u32 = 0xa4;
/// Actor slot `0x2bc` `(0)`: a float for the request (`ST0`).
const ACTOR_SLOT_0X2BC: u32 = 0x2bc;

/// The common tail of every flee request: the setters for the actor and
/// the speed. Builds nothing; see the three sequences in `ProcessFlee`.
fn flee_request_speed(e: &mut Engine, request: Ptr, actor: u32) {
    let speed = e.vcall(actor, ACTOR_SLOT_0X2BC, &args![0u32]).f32();
    e.call(FLEE_REQUEST_SET_SPEED, &args![request, speed]);
}

/// The byte `ProcessFlee` hands the request when the actor's form allows it
/// (actor slot `0x21c`): `004181e0` of the actor, plus 0x30, virtual `+0x28`.
fn flee_request_form_byte(e: &mut Engine, request: Ptr, actor: u32) {
    if e.vcall(actor, ACTOR_SLOT_0X21C, &args![]).bool() {
        let form = e.call(REFERENCE_AI_FORM, &args![actor]).u32();
        let component = form + 0x30;
        let value = e.vcall(component, 0x28, &args![]).u8();
        e.call(FLEE_REQUEST_SET_BYTE, &args![request, u32::from(value)]);
    }
}

/// The package distance of the flee request: `GetPackageDistance(actor)` as
/// an unsigned number, converted to float.
fn flee_request_radius(e: &mut Engine, request: Ptr, package: u32, actor: u32) {
    let radius = e
        .call(PACKAGE_DISTANCE_FOR_ACTOR, &args![package, actor])
        .u32();
    e.call(FLEE_REQUEST_SET_RADIUS, &args![request, radius as f32]);
}

/// The goal location of a flee request: the package's target reference as a
/// `PathingLocation`, or the "no location" form.
fn flee_request_location(e: &mut Engine, request: Ptr, flee: u32) {
    let target = e.call(FLEE_PACKAGE_TARGET_REFERENCE, &args![flee]).u32();
    if target != 0 {
        let target = e.call(FLEE_PACKAGE_TARGET_REFERENCE, &args![flee]).u32();
        e.with_stack(0x34, |e, location| {
            e.call(PATHING_LOCATION_FROM_ACTOR, &args![location, target]);
            e.call(PATHING_REQUEST_SET_GOAL, &args![request, location]);
            e.call(PATHING_LOCATION_DESTRUCTOR, &args![location]);
        });
    } else {
        fn_008df000(e, request);
    }
}

// Translated from 008dddf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessFlee` (Xbox PDB): the flee package (type 0x16) of
/// `actor`.
///
/// The flee package (`flee`) is the running package when its type is 0x16.
/// It is updated (`009f1830`); an actor whose slot `0x358` is set only
/// advances the package's timer (`006ca4e0` times `0101a6b0`). The flee
/// target (`ref`) is the package target's reference. Procedure 1 is added and
/// nothing else done when there is no target, the package is not an
/// actor-following one and `ShouldShutDown` holds while the player is not
/// to be attacked, or when the target is an inanimate reference that
/// `00477ba0` or `008df020` flags.
///
/// Otherwise the distances come from the settings (interior or exterior
/// cell). The three stages are:
/// - when the package has a flee duration (`004fd400`) or avoided reference
///   and `67efd0`/`8b1ff0` allow it, the actor stops, tells the process the
///   movement ended and starts a special idle; the idle chatter timer
///   (`+0x298`) counts down and, at 0, may start a greeting with the topic
///   (2, 3) of the player's process; the package's time is advanced and at
///   `01020998` the package ends (procedure 1);
/// - when the actor has no path (`008b3b90`), a flee request
///   (`PathingRequestFlee`) is built from the package's distance and
///   target and given to the actor (`SetPathfindingFlee`);
/// - otherwise the nearest avoided reference or the target decides: the
///   actor flees to a location (the package location, the teleport door,
///   `FindTeleportDoor`), walks to a door and activates it, goes to a
///   reference or keeps its movement animation (`SetActorsAnimation`).
///
/// The SEH frame is not translated. The decompiled sequence for each stage
/// is kept in the order of the code.
pub fn high_process_process_flee(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let player = player_pointer(e);
    let mut flee = 0u32;
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if package != 0 && e.call(PACKAGE_TYPE, &args![package]).i32() == 0x16 {
        flee = package;
    }
    if flee != 0 {
        e.call(FLEE_PACKAGE_UPDATE, &args![flee, this]);
    }
    let mut reference = 0u32;
    let mut following = false;
    if e.vcall(a, ACTOR_SLOT_0X428, &args![]).u32() != 0
        && e.call(ACTOR_FLAG_BYTE_14D, &args![actor]).bool()
    {
        following = true;
    }
    if e.vcall(a, ACTOR_SLOT_0X358, &args![]).bool() {
        e.call(FLEE_PACKAGE_SET_FLAG, &args![flee, 0u32]);
        let time = e.call(FLEE_PACKAGE_TIME_VALUE, &args![flee]).f64();
        let scaled = (time * e.global::<f64>(FLEE_TIME_FACTOR)) as f32;
        e.call(FLEE_PACKAGE_ADD_TIME, &args![flee, scaled]);
        return;
    }
    let flag_574900 = e.call(FLEE_PACKAGE_FLAG_574900, &args![flee]).bool();
    if !flag_574900 && flee != 0 {
        let package_target = e.call(PACKAGE_TARGET_WORD, &args![flee]).u32();
        if package_target != 0
            && e.call(PACKAGE_TARGET_GET_REFERENCE, &args![package_target])
                .u32()
                != 0
        {
            let package_target = e.call(PACKAGE_TARGET_WORD, &args![flee]).u32();
            reference = e
                .call(PACKAGE_TARGET_GET_REFERENCE, &args![package_target])
                .u32();
        }
    }
    let stop = e.with_stack(4, |e, attack_word| {
        e.mem.set_u32(attack_word.addr(), 0);
        let mut stop = false;
        let alive_reference =
            reference == 0 || e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool();
        if (alive_reference && !following && flee == 0)
            || (e.call(FLEE_PACKAGE_SHOULD_SHUT_DOWN, &args![flee]).bool()
                && !e
                    .call(
                        ACTOR_GET_SHOULD_ATTACK,
                        &args![actor, player, 0u32, attack_word, 0u32],
                    )
                    .bool())
        {
            stop = true;
        }
        stop
    });
    if stop {
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
        return;
    }
    if reference != 0
        && !e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool()
        && (e.call(REFERENCE_FLAG_477BA0, &args![reference]).bool()
            || fn_008df020(e, Ptr::new(flee)) != 0)
    {
        e.vcall(
            process,
            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
            &args![actor, 1u32],
        );
        return;
    }
    flee_stages(e, this, actor, flee, reference, flag_574900, following);
}

/// The second part of `ProcessFlee` (`008de012` on).
fn flee_stages(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    flee: u32,
    reference: u32,
    flag_574900: bool,
    _following: bool,
) {
    let process = this.addr();
    let a = actor.addr();
    let player = player_pointer(e);
    e.call(FLEE_PACKAGE_DOOR, &args![flee]);
    let mut cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
    let mut world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![actor]).u32();
    let time_value = e.call(FLEE_PACKAGE_TIME_VALUE, &args![flee]).f32();
    let mut avoided = e
        .call(FLEE_PACKAGE_CLOSEST_AVOIDED, &args![flee, actor])
        .u32();
    let target = e.call(FLEE_PACKAGE_TARGET_REFERENCE, &args![flee]).u32();
    let flag_8b6240 = e.call(FLEE_PACKAGE_FLAG_8B6240, &args![flee]).bool();
    let mut escape_ready = false;
    let move_mode = 0x201u32;
    if !flag_574900 && avoided == 0 {
        avoided = reference;
    }
    let interior = cell != 0 && e.call(CELL_FLAG_TEST, &args![cell]).bool();
    let setting = e
        .call(
            SETTING_VALUE,
            &args![if interior {
                FLEE_RADIUS_INTERIOR
            } else {
                FLEE_RADIUS_EXTERIOR
            }],
        )
        .u32();
    let flee_radius = e.mem.f32(setting);
    let interior = cell != 0 && e.call(CELL_FLAG_TEST, &args![cell]).bool();
    let setting = e
        .call(
            SETTING_VALUE,
            &args![if interior {
                FLEE_DISTANCE_INTERIOR
            } else {
                FLEE_DISTANCE_EXTERIOR
            }],
        )
        .u32();
    let mut flee_distance = e.mem.f32(setting);
    let package_target = e.call(PACKAGE_TARGET_WORD, &args![flee]).u32();
    if package_target != 0 {
        let count = e.call(WORD_AT_8, &args![package_target]).i32();
        if f64::from(count) > 0.0 {
            let package_target = e.call(PACKAGE_TARGET_WORD, &args![flee]).u32();
            flee_distance = e.call(WORD_AT_8, &args![package_target]).i32() as f32;
        }
    }
    // The package is at its spot: the actor stops and idles.
    if target != 0 || avoided == 0 {
        if e.call(PACKAGE_FLAG_4, &args![flee]).bool()
            || e.call(PACKAGE_FLAG_2, &args![flee]).bool()
        {
            let mut engaged = true;
            if target != 0 {
                let distance = e
                    .call(DISTANCE_FROM_REFERENCE, &args![actor, target, 0u32, 0u32])
                    .f64();
                let package_distance = e
                    .call(PACKAGE_DISTANCE_FOR_ACTOR, &args![flee, actor])
                    .u32();
                // `FILD` of the unsigned number against the distance: the
                // package distance has to be above it.
                engaged = f64::from(package_distance) > distance;
            }
            if engaged {
                if e.call(PACKAGE_FLAG_2, &args![flee]).bool() {
                    e.mem.set_f32(process + FLEE_PACKAGE_EVAL_TIMER, 0.0);
                }
                e.call(ACTOR_STOP_MOVING, &args![actor]);
                e.call(FLEE_PACKAGE_SET_FLAG, &args![flee, 1u32]);
                e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
                let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
                if animation != 0
                    && e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                        .bool()
                {
                    e.vcall(
                        process,
                        SLOT_SETUP_SPECIAL_IDLE,
                        &args![actor, 0u32, 2u32, 1u32, 0u32, 1u32],
                    );
                }
                flee_idle_chatter(e, this, actor, player);
                let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f32();
                e.call(FLEE_PACKAGE_ADD_TIME, &args![flee, frame]);
                let time = e.call(FLEE_PACKAGE_TIME_VALUE, &args![flee]).f64();
                // `FCOMP`: the package ends once its time reaches the limit.
                if !(time < e.global::<f64>(FLEE_TIME_LIMIT)) {
                    e.vcall(
                        process,
                        SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                        &args![actor, 1u32],
                    );
                }
                return;
            }
            e.mem.set_f32(process + FLEE_IDLE_CHATTER_TIMER, 0.0);
        }
    }
    // No path: a flee request is built and given to the actor.
    if e.call(ACTOR_FLEE_ARRIVAL_CHECK, &args![actor]).bool() {
        e.vcall(process, SLOT_PROCESS_0X204, &args![1u32]);
        e.call(FLEE_PACKAGE_CLEAR_TARGET, &args![flee, 0u32]);
        e.call(FLEE_PACKAGE_RESET_LOCATION, &args![flee, 0u32]);
        e.with_stack(FLEE_REQUEST_SIZE, |e, request| {
            e.call(FLEE_REQUEST_CONSTRUCTOR, &args![request]);
            e.call(PATHING_REQUEST_SET_ACTOR, &args![request, actor]);
            flee_request_form_byte(e, request, a);
            flee_request_radius(e, request, flee, a);
            let word = e.call(FLEE_PACKAGE_FLEE_RADIUS, &args![flee]).u32();
            e.call(FLEE_REQUEST_SET_WORD, &args![request, word]);
            e.call(FLEE_REQUEST_SET_DISTANCE, &args![request, flee_distance]);
            flee_request_speed(e, request, a);
            flee_request_location(e, request, flee);
            e.call(ACTOR_SET_PATHFINDING_FLEE, &args![actor, request]);
            e.call(FLEE_REQUEST_DESTRUCTOR, &args![request]);
        });
        return;
    }
    if cell != 0 {
        let mut avoid_distance = e.global::<f32>(FLEE_AVOID_DISTANCE_DEFAULT);
        if avoided != 0
            && !e.vcall(avoided, ACTOR_SLOT_0X22C, &args![0u32]).bool()
            && !e.call(REFERENCE_FLAG_800, &args![avoided]).bool()
        {
            avoid_distance = e
                .call(DISTANCE_FROM_REFERENCE, &args![actor, avoided, 0u32, 0u32])
                .f32();
        }
        let mut flee_target = e.call(FLEE_PACKAGE_TARGET_REFERENCE, &args![flee]).u32();
        let mut flee_actor = 0u32;
        if flee_target != 0 && e.vcall(flee_target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
            flee_actor = flee_target;
        }
        let mut clear = false;
        if flee_target != 0 {
            if e.vcall(flee_target, ACTOR_SLOT_0X22C, &args![0u32]).bool()
                || e.call(REFERENCE_FLAG_800, &args![flee_target]).bool()
            {
                clear = true;
            }
        }
        if !clear && flee_actor != 0 {
            if e.call(ACTOR_CREATURE_LIKE, &args![flee_actor]).bool()
                || !e.call(ACTOR_CAN_MOVE, &args![flee_actor]).bool()
            {
                clear = true;
            }
        }
        if clear {
            flee_target = 0;
            e.call(FLEE_PACKAGE_CLEAR_TARGET, &args![flee, 0u32]);
        }
        let door = e.call(FLEE_PACKAGE_DOOR, &args![flee]).u32();
        // `FCOMPP`: the flee radius has to be above the avoid distance.
        let mut tick = f64::from(flee_radius) > f64::from(avoid_distance) && door == 0;
        if !tick {
            tick = !flag_8b6240 && time_value == 0.0 && fn_008df040(e, Ptr::new(flee)) != 0;
        }
        if tick {
            let essential = e.mem.f32(process + FLEE_ESSENTIAL_TIMER);
            // `FCOMP` against 0.0: above it (or unordered) counts down.
            if f64::from(essential) > e.global::<f64>(ZERO_DOUBLE) || essential.is_nan() {
                let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
                let value = (f64::from(essential) - frame) as f32;
                e.mem.set_f32(process + FLEE_ESSENTIAL_TIMER, value);
            } else {
                escape_ready = true;
                let value = e.global::<f32>(SLEEP_ACQUIRE_TIMER_VALUE);
                e.mem.set_f32(process + FLEE_ESSENTIAL_TIMER, value);
            }
            if fn_008df040(e, Ptr::new(flee)) != 0 {
                let mut from = e.call(FLEE_PACKAGE_TARGET_REFERENCE, &args![flee]).u32();
                if from == player {
                    from = 0;
                }
                if from == 0 {
                    cell = 0;
                    world_space = 0;
                    if escape_ready {
                        e.call(
                            FLEE_PACKAGE_FIND_TELEPORT_DOOR,
                            &args![flee, actor, avoided],
                        );
                    } else {
                        e.call(FLEE_PACKAGE_RESET_LOCATION, &args![flee, 0u32]);
                    }
                } else {
                    cell = e.call(REFERENCE_PATHING_CELL, &args![from]).u32();
                    world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![from]).u32();
                }
            }
            if e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool()
                || flee_mover_far_enough(e, actor)
            {
                let mut built = true;
                e.with_stack(FLEE_REQUEST_SIZE, |e, request| {
                    e.call(FLEE_REQUEST_CONSTRUCTOR, &args![request]);
                    e.call(PATHING_REQUEST_SET_ACTOR, &args![request, actor]);
                    flee_request_speed(e, request, a);
                    flee_request_form_byte(e, request, a);
                    flee_request_radius(e, request, flee, a);
                    e.call(FLEE_REQUEST_SET_DISTANCE, &args![request, flee_distance]);
                    let word = e.call(FLEE_PACKAGE_FLEE_RADIUS, &args![flee]).u32();
                    e.call(FLEE_REQUEST_SET_WORD, &args![request, word]);
                    flee_request_speed(e, request, a);
                    flee_request_location(e, request, flee);
                    built = e
                        .call(ACTOR_SET_PATHFINDING_FLEE, &args![actor, request])
                        .bool();
                    e.call(FLEE_REQUEST_DESTRUCTOR, &args![request]);
                });
                if !built {
                    return;
                }
            }
            e.call(FLEE_PACKAGE_SET_FLAG, &args![flee, 0u32]);
            let time = e.call(FLEE_PACKAGE_TIME_VALUE, &args![flee]).f64();
            let scaled = (time * e.global::<f64>(FLEE_TIME_FACTOR)) as f32;
            e.call(FLEE_PACKAGE_ADD_TIME, &args![flee, scaled]);
        } else if fn_008df040(e, Ptr::new(flee)) == 0 {
            let location = e.call(PACKAGE_LOCATION_WORD, &args![flee]).u32();
            let kind = e.call(PACKAGE_LOCATION_TYPE, &args![location]).i32();
            if kind == 1 {
                cell = e.call(PACKAGE_LOCATION_GET_CELL, &args![location]).u32();
            } else if kind == 0 || e.call(PACKAGE_LOCATION_TYPE, &args![location]).i32() == 6 {
                let generic = e.vcall(process, SLOT_GET_GENERIC_LOCATION, &args![]).u32();
                if generic != 0 {
                    cell = e.call(REFERENCE_PATHING_CELL, &args![generic]).u32();
                    world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![generic]).u32();
                }
            }
        } else if flee_target != 0 {
            cell = e.call(REFERENCE_PATHING_CELL, &args![flee_target]).u32();
            world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![flee_target]).u32();
        }
    }
    // The flee itself.
    let door = e.call(FLEE_PACKAGE_DOOR, &args![flee]).u32();
    if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool()
        && (door != 0 || !flee_mover_far_enough(e, actor))
    {
        let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
        if animation != 0
            && !e
                .call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                .bool()
        {
            e.vcall(process, SLOT_EAT_AFTER, &args![actor]);
        }
        e.vcall(
            process,
            SLOT_SET_ACTORS_ANIMATION,
            &args![actor, move_mode, 1u32],
        );
        return;
    }
    let mut destination = e.call(FLEE_PACKAGE_TARGET_REFERENCE, &args![flee]).u32();
    if destination == player {
        destination = 0;
        e.call(FLEE_PACKAGE_CLEAR_TARGET, &args![flee, 0u32]);
    }
    if door != 0 {
        flee_to_door(e, this, actor, flee, door, flee_distance);
    } else if destination != 0 {
        let distance = e
            .call(
                DISTANCE_FROM_REFERENCE,
                &args![actor, destination, 0u32, 0u32],
            )
            .f64();
        // `FCOMP`: at most the arrival distance (or unordered) idles.
        if distance > e.global::<f64>(FLEE_ARRIVED_DISTANCE) {
            let position = e.vcall(destination, ACTOR_SLOT_POSITION, &args![]).u32();
            let cell = e.call(REFERENCE_PATHING_CELL, &args![destination]).u32();
            let world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![destination]).u32();
            if !e
                .call(
                    ACTOR_SET_PATHFINDING_GOAL,
                    &args![actor, position, cell, world_space, 0.0f32, 0u32],
                )
                .bool()
            {
                e.call(FLEE_PACKAGE_CLEAR_TARGET, &args![flee, 0u32]);
            }
        } else {
            flee_idle(e, this, actor, flee);
        }
    } else {
        flee_idle(e, this, actor, flee);
    }
    let _ = (cell, world_space);
}

/// The actor idles while fleeing from nothing: a special idle may start,
/// the package flag is set and its time advances by the frame time.
fn flee_idle(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, flee: u32) {
    let process = this.addr();
    let animation = e.vcall(actor.addr(), ACTOR_SLOT_ANIMATION, &args![]).u32();
    if animation != 0
        && e.call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
            .bool()
    {
        e.vcall(
            process,
            SLOT_SETUP_SPECIAL_IDLE,
            &args![actor, 0u32, 2u32, 1u32, 0u32, 1u32],
        );
    }
    e.call(FLEE_PACKAGE_SET_FLAG, &args![flee, 1u32]);
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f32();
    e.call(FLEE_PACKAGE_ADD_TIME, &args![flee, frame]);
}

/// `ProcessFlee`'s mover test: the actor's mover has travelled more and has
/// less to go than the limit `0102e430`, with a valid path.
fn flee_mover_far_enough(e: &mut Engine, actor: Ptr) -> bool {
    if !e.call(ACTOR_IS_PATH_VALID, &args![actor]).bool() {
        return false;
    }
    let mover = e.mem.u32(actor.addr() + ACTOR_MOVER);
    let traveled = e.call(MOVER_DISTANCE_TRAVELED, &args![mover]).f64();
    let limit = e.global::<f64>(FLEE_MOVER_LIMIT);
    // `FCOMP`: the traveled distance has to be above the limit.
    if !(traveled > limit) {
        return false;
    }
    let mover = e.mem.u32(actor.addr() + ACTOR_MOVER);
    let remaining = e.call(MOVER_DISTANCE_REMAINING, &args![mover]).f64();
    remaining < limit
}

/// The idle chatter of a fleeing actor standing at its spot: while its timer
/// (+0x298) runs it counts down; at 0 a greeting may start (topic 2 of 3,
/// from the player's process) and the timer is set again.
fn flee_idle_chatter(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, player: u32) {
    let process = this.addr();
    let timer = e.mem.f32(process + FLEE_IDLE_CHATTER_TIMER);
    if f64::from(timer) > e.global::<f64>(ZERO_DOUBLE) || timer.is_nan() {
        flee_chatter_count_down(e, process, timer);
        return;
    }
    if !e.call(PLAYER_GREET_TEST, &args![player]).bool() && fn_008defe0(e, Ptr::new(player)) == 0 {
        let roll = e.call(ROLL_ZERO_TO, &args![0u32, 100u32]).u32();
        let chance = e.call(ROLL_TO_CHANCE, &args![roll]).i32();
        let setting = e
            .call(SETTING_VALUE_ADDRESS, &args![FLEE_GREET_CHANCE_SETTING])
            .u32();
        if chance <= e.mem.i32(setting) {
            let topic = e.call(GET_TOPIC, &args![2u32, 3u32]).u32();
            if topic != 0 {
                e.vcall(
                    process,
                    SLOT_GREET,
                    &args![actor, topic, 0u32, 0u32, 1u32, 0u32],
                );
            }
        }
        let high = e.call(SETTING_VALUE, &args![FLEE_TIMER_HIGH_SETTING]).u32();
        let high = e.mem.f32(high);
        let low = e.call(SETTING_VALUE, &args![FLEE_TIMER_LOW_SETTING]).u32();
        let low = e.mem.f32(low);
        let next = e.call(RANDOM_BETWEEN, &args![low, high]).f32();
        e.mem.set_f32(process + FLEE_IDLE_CHATTER_TIMER, next);
        return;
    }
    flee_chatter_count_down(e, process, timer);
}

fn flee_chatter_count_down(e: &mut Engine, process: u32, timer: f32) {
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
    e.mem.set_f32(
        process + FLEE_IDLE_CHATTER_TIMER,
        (f64::from(timer) - frame) as f32,
    );
}

/// The door stage of `ProcessFlee`: the actor goes to the door and activates
/// it once close, or builds a flee request to the door's far side.
fn flee_to_door(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    flee: u32,
    door: u32,
    flee_distance: f32,
) {
    let process = this.addr();
    let a = actor.addr();
    let mut door_distance = e.global::<f32>(FLEE_DOOR_DISTANCE_DEFAULT);
    let linked = e.call(REFERENCE_HAS_EXTRA_FLAG, &args![door]).u32();
    if linked != 0 {
        let position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
        door_distance = e.with_stack(12, |e, out| {
            let held = e.call(NI_POINTER_GET, &args![linked]).u32();
            let extra = e.call(REFERENCE_HAS_EXTRA_FLAG, &args![held]).u32();
            let owner = e.call(DOOR_POSITION_OWNER, &args![extra]).u32();
            let vector = e.call(POINT_DIFFERENCE, &args![owner, out, position]).u32();
            e.call(POINT_LENGTH, &args![vector]).f32()
        });
    }
    let distance = e
        .call(DISTANCE_FROM_REFERENCE, &args![actor, door, 0u32, 0u32])
        .f32();
    let nearest = e.call(FLOAT_MIN, &args![door_distance, distance]).f32();
    // `FCOMP`: only an ordered "less than" counts as arrived.
    if f64::from(nearest) < e.global::<f64>(FLEE_ARRIVED_DISTANCE) {
        e.vcall(process, SLOT_EAT_AFTER, &args![actor]);
        e.call(FLEE_PACKAGE_RESET_LOCATION, &args![flee, 0u32]);
        e.call(REFERENCE_ACTIVATE, &args![door, actor, 0u32, 0u32, 1u32]);
        let value = e.global::<f32>(SLEEP_ACQUIRE_TIMER_VALUE);
        e.mem.set_f32(process + FLEE_ESSENTIAL_TIMER, value);
        return;
    }
    if linked == 0 {
        return;
    }
    e.with_stack(FLEE_REQUEST_SIZE, |e, request| {
        e.call(FLEE_REQUEST_CONSTRUCTOR, &args![request]);
        e.call(PATHING_REQUEST_SET_ACTOR, &args![request, actor]);
        flee_request_speed(e, request, a);
        flee_request_form_byte(e, request, a);
        flee_request_radius(e, request, flee, a);
        e.call(FLEE_REQUEST_SET_DISTANCE, &args![request, flee_distance]);
        let word = e.call(FLEE_PACKAGE_FLEE_RADIUS, &args![flee]).u32();
        e.call(FLEE_REQUEST_SET_WORD, &args![request, word]);
        e.with_stack(0x34, |e, location| {
            let held = e.call(NI_POINTER_GET, &args![linked]).u32();
            let extra = e.call(REFERENCE_HAS_EXTRA_FLAG, &args![held]).u32();
            let owner = e.call(DOOR_POSITION_OWNER, &args![extra]).u32();
            e.call(PATHING_LOCATION_FROM_TWO, &args![location, owner, door]);
            e.call(PATHING_REQUEST_SET_GOAL, &args![request, location]);
            e.call(PATHING_LOCATION_DESTRUCTOR, &args![location]);
        });
        flee_request_speed(e, request, a);
        let built = e
            .call(ACTOR_SET_PATHFINDING_FLEE, &args![actor, request])
            .bool();
        e.call(FLEE_REQUEST_DESTRUCTOR, &args![request]);
        let _ = built;
    });
}

/// `LowProcess` members (Xbox PDB): `pGenericLocation` (+0x44),
/// `ObjectList` (`BSSimpleList<ObjectstoAcquire *>`, +0x5c) and
/// `pAcquireObject` (`ObjectstoAcquire*`, +0x64).
const LOW_GENERIC_LOCATION: u32 = 0x44;
const LOW_OBJECT_LIST: u32 = 0x5c;
const LOW_ACQUIRE_OBJECT: u32 = 0x64;

/// `Actor` helper `008a2d40`: the inventory entry (a heap object the caller
/// deletes) the actor's running package wants it to pick up, or null.
const ACTOR_PACKAGE_ITEM: u32 = 0x008a_2d40;
/// `TESObjectREFR::GetInventoryItem` (Xbox PDB, `00576260`): `this`, the
/// object, a flag.
const REFERENCE_GET_INVENTORY_ITEM: u32 = 0x0057_6260;
/// Scalar deleting destructor of the inventory entry (`004459e0`): `this`,
/// delete flag.
const ITEM_DELETE: u32 = 0x0044_59e0;
/// The word at +0xc of `this` (`0084e3a0`).
const WORD_AT_0XC: u32 = 0x0084_e3a0;
/// `00575450`: whether the reference (`this`) has a container and it holds
/// the given object.
const REFERENCE_HOLDS_OBJECT: u32 = 0x0057_5450;
/// `00440da0`: bit 0x800 of the flags at +8 of `this`.
const REFERENCE_FLAG_800: u32 = 0x0044_0da0;
/// `00440d80`: bit 0x20 of the flags at +8 of `this`.
const REFERENCE_FLAG_20: u32 = 0x0044_0d80;
/// `005d43c0`: the `ExtraDataList` (`this + 0x44`) of a reference.
const REFERENCE_EXTRA_DATA: u32 = 0x005d_43c0;
/// `ExtraDataList::GetReferencePointer` (Xbox PDB, `0041c8d0`).
const EXTRA_DATA_REFERENCE_POINTER: u32 = 0x0041_c8d0;
/// `TESPackage::SetNeverRun` (Xbox PDB, `00674e70`): `this`, reference, flag.
const PACKAGE_SET_NEVER_RUN: u32 = 0x0067_4e70;
/// `TESPackage::GetIsCreated` (Xbox PDB, `00674d40`).
const PACKAGE_GET_IS_CREATED: u32 = 0x0067_4d40;
/// `PackageTarget::GetTargType` (Xbox PDB, `00519b00`).
const PACKAGE_TARGET_GET_TYPE: u32 = 0x0051_9b00;
/// `PackageTarget::GetTargObjectType` (Xbox PDB, `00680080`).
const PACKAGE_TARGET_GET_OBJECT_TYPE: u32 = 0x0068_0080;
/// `00441b00`: bit 0x200000 of the flags at +0x1c of `this`.
const PACKAGE_FLAG_200000: u32 = 0x0044_1b00;
/// The package's location word (`0055b980`: the word at +0x2c).
const PACKAGE_LOCATION_WORD: u32 = 0x0055_b980;
/// `PackageLocation::GetLocReference` (Xbox PDB, `0067f390`).
const PACKAGE_LOCATION_GET_REFERENCE: u32 = 0x0067_f390;
/// `TESObjectREFR::HasContainer` (Xbox PDB, `0055d310`).
const REFERENCE_HAS_CONTAINER: u32 = 0x0055_d310;
/// `BGSSaveFormBuffer::GetForm` (`007af430`; the engine map's name, the body
/// returns the form of a reference).
const REFERENCE_GET_FORM: u32 = 0x007a_f430;
/// `TESPackage::GetInitialTargetCount` (Xbox PDB, `00672930`): the argument
/// the caller pushed before it stays on the stack for the next call.
const PACKAGE_INITIAL_TARGET_COUNT: u32 = 0x0067_2930;
/// `00892e90` (`Actor`): hands an item over to a container reference:
/// `(this, amount word, first entry, container, count, form)`.
const ACTOR_PUT_IN_CONTAINER: u32 = 0x0089_2e90;
/// `008256d0`: whether the `BSSimpleList` in `this` is empty.
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// `ObjectstoAcquire` scalar deleting destructor (`007b3fa0`).
const ACQUIRE_OBJECT_DELETE: u32 = 0x007b_3fa0;
/// Forms created for an actor that is an NPC (`006047c0`, `tesnpc.cpp`):
/// `(this form, actor, flag, flag, 0, 1)`.
const NPC_FORM_EQUIP: u32 = 0x0060_47c0;
/// `TESCreature::InitDefaultWorn` (Xbox PDB, `005f9e00`).
const CREATURE_INIT_DEFAULT_WORN: u32 = 0x005f_9e00;
/// `Actor::IsPlayingLowerBodySpecialIdle` (Xbox PDB, `008ba530`).
const ACTOR_IS_PLAYING_LOWER_BODY_SPECIAL_IDLE: u32 = 0x008b_a530;
/// `Animation::SpecialIdleFree` (Xbox PDB, `00498910`): `(this, 1, 0)`.
const ANIMATION_SPECIAL_IDLE_FREE: u32 = 0x0049_8910;
/// `TESPackage::GetLocationWorld`, `GetLocationCell`, `GetLocationCoord`
/// (Xbox PDB, `00675a50`, `00675c20`, `00675de0`).
const PACKAGE_LOCATION_WORLD: u32 = 0x0067_5a50;
const PACKAGE_LOCATION_CELL: u32 = 0x0067_5c20;
const PACKAGE_LOCATION_COORD: u32 = 0x0067_5de0;
/// `PathingLocation::PathingLocation_ov7` (Xbox PDB, `006dcee0`): `(this,
/// coordinates, cell, world space)`.
const PATHING_LOCATION_FROM_COORD: u32 = 0x006d_cee0;
/// `TESPackage::GetPackageRadiusActorToLocation` (Xbox PDB, `00678670`):
/// `(this, actor, flag)`, `ST0` result.
const PACKAGE_RADIUS_ACTOR_TO_LOCATION: u32 = 0x0067_8670;
/// `MiddleHighProcess::IsCurrentWeaponGrenade` (Xbox PDB, `008d8220`).
const PROCESS_IS_CURRENT_WEAPON_GRENADE: u32 = 0x008d_8220;
/// `0067a480`: package test used with the move flags (bit 0x?? of the
/// package, kept as a call).
const PACKAGE_TEST_67A480: u32 = 0x0067_a480;
/// `Actor::EndMovement` (Xbox PDB, `0087faa0`).
const ACTOR_END_MOVEMENT: u32 = 0x0087_faa0;
/// `TESPackage::IsInterruptPackage` (Xbox PDB, `00678610`).
const PACKAGE_IS_INTERRUPT: u32 = 0x0067_8610;
/// `009611e0`: the word at +0x18 of `this` (the package's type-like word).
const PACKAGE_WORD_AT_0X18: u32 = 0x0096_11e0;
/// `Actor::GetCurrentPackage` / `MobileObject::GetCurrentPackage`
/// (Xbox PDB, `009344a0`) is [`ACTOR_CURRENT_PACKAGE`].
const SLOT_SET_PROCEDURE_INDEX_RUNNING: u32 = 0x284;
const SLOT_CLEAR_ALL_HEAD_TRACK_TARGETS: u32 = 0x660;
const SLOT_CALCULATE_MOVE_MODE: u32 = 0x34c;
const SLOT_CLEAR_CURRENT_PACKAGE: u32 = 0x234;
/// Slot `0x13c` of the package itself.
const PACKAGE_SLOT_0X13C: u32 = 0x13c;
/// Slots of `Actor` (PC byte offsets): `0x3cc` receives what an escort
/// hands over, `0x418` is called for an actor stuck in state 4 or 9, `0x218`
/// and `0x21c` say how the actor's form is initialised (`0x218`: NPC form
/// code, `0x21c`: creature form code).
const ACTOR_SLOT_0X3CC: u32 = 0x3cc;
const ACTOR_SLOT_0X418: u32 = 0x418;
const ACTOR_SLOT_0X218: u32 = 0x218;
const ACTOR_SLOT_0X21C: u32 = 0x21c;
/// The two forms `ProcessEscort` accepts as a location's form
/// (`011ca244`, `011ca248`).
const FORM_WORD_A: u32 = 0x011c_a244;
const FORM_WORD_B: u32 = 0x011c_a248;

/// Deletes the inventory entry `ProcessEscort` fetched, when there is one.
fn escort_free_item(e: &mut Engine, item: u32) {
    if item != 0 {
        e.call(ITEM_DELETE, &args![item, 1u32]);
    }
}

/// The first entry of the list in the word at `item` (`00559450` then
/// `006815c0`), or `current` when the entry has none.
fn escort_first_entry(e: &mut Engine, item: u32, current: u32) -> u32 {
    if e.call(NI_POINTER_GET, &args![item]).u32() != 0 {
        let list = e.call(NI_POINTER_GET, &args![item]).u32();
        let node = e.call(NODE_ITEM_ADDRESS, &args![list]).u32();
        e.mem.u32(node)
    } else {
        current
    }
}

/// What `ProcessEscort` works out for an actor that is not next to a
/// container: the reference of the package's location (or the process'
/// generic location) when its form is one of the two accepted ones, the
/// position slot of it (`0x1f4`) or 0.
fn escort_location_position(e: &mut Engine, process: u32, package_location: u32) -> u32 {
    let mut position = 0;
    if package_location != 0 {
        let mut location = e
            .call(PACKAGE_LOCATION_GET_REFERENCE, &args![package_location])
            .u32();
        if location == 0 {
            location = e.mem.u32(process + LOW_GENERIC_LOCATION);
        }
        if location != 0 {
            let form_a = e.global::<u32>(FORM_WORD_A);
            let form_b = e.global::<u32>(FORM_WORD_B);
            if e.call(REFERENCE_GET_FORM, &args![location]).u32() == form_a
                || e.call(REFERENCE_GET_FORM, &args![location]).u32() == form_b
            {
                position = e.vcall(location, ACTOR_SLOT_POSITION, &args![]).u32();
            }
        }
    }
    position
}

// Translated from 008dfa10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessEscort` (Xbox PDB): the escort package. The escorted
/// reference is the process' target when it is an actor other than `actor`.
///
/// - Without an inventory entry (`008a2d40`) the process first gets a target
///   for the package and, when it is not an actor that is carrying the item
///   (`+0x100` and `00575450`), only procedure 1 is added to the running
///   ones and the movement ended.
/// - An escorted actor whose current package is neither type 1 nor a created
///   one (unless it is the player) ends the escort (`SetProcedureIndexRunning
///   (0)`), otherwise its head tracking is cleared and the actor stops looking
///   at it.
/// - A package target of type 0 or 3 with a reference that is gone (flags
///   `0x800` or `0x20`) ends the package (`SetNeverRun`).
/// - The special idle is freed, and for an escorted actor
///   [`high_process_should_wait_for_escort_target`] says whether to wait.
/// - A package that has a location (`slot 0x13c` of the package) and no
///   escorted actor hands the inventory item over: to the container
///   reference (`00892e90`) or to the actor's slot `0x3cc`, letting the
///   special idle play first (`bActivateAnim`, +0x375). For a target object
///   type 0xc or 0x13..0x17 the process then takes the next objects to
///   acquire, and the actor's worn items are set up (`006047c0`,
///   `TESCreature::InitDefaultWorn`).
/// - With an escorted actor it keeps the pace (move mode, `CalculateMoveMode`)
///   and ends the escorted actor's package when it has the right one.
/// - Finally, when the actor is not pathing, it builds a `PathingRequest` to the
///   package location with the package radius and sets it as the goal.
///
/// The SEH frame and the unwinding are not translated.
pub fn high_process_process_escort(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    e.with_stack(4, |e, wait_flag| {
        escort_run(e, this, actor, wait_flag.addr());
    });
}

fn escort_run(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, wait_flag: u32) {
    let process = this.addr();
    let a = actor.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    let mut item = e.call(ACTOR_PACKAGE_ITEM, &args![actor]).u32();
    if item == 0 {
        let mut stop = false;
        let target = e.mem.u32(process + LOW_TARGET);
        if target != 0 {
            if e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
                stop = true;
            } else {
                let word = e.call(WORD_AT_0XC, &args![target]).u32();
                if e.call(REFERENCE_HOLDS_OBJECT, &args![actor, word]).bool() {
                    stop = true;
                }
            }
        }
        if !stop {
            e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
            let target = e.mem.u32(process + LOW_TARGET);
            if target != 0 {
                if e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
                    stop = true;
                } else {
                    let word = e.call(WORD_AT_0XC, &args![target]).u32();
                    if e.call(REFERENCE_HOLDS_OBJECT, &args![actor, word]).bool() {
                        stop = true;
                    }
                }
            }
            if !stop {
                e.vcall(
                    process,
                    SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                    &args![actor, 1u32],
                );
                if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
                    e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
                }
                return;
            }
        }
    }
    let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
    if item == 0 && entry != 0 && e.mem.u32(entry) == a {
        let object = e.mem.u32(entry + 4);
        item = e
            .call(REFERENCE_GET_INVENTORY_ITEM, &args![actor, object, 0u32])
            .u32();
    }
    let mut has_escorted = false;
    let mut escorted = 0u32;
    let target = e.mem.u32(process + LOW_TARGET);
    if target != 0 && e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() && target != a {
        has_escorted = true;
        escorted = target;
        let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
        let target_package = e
            .vcall(target_process, SLOT_GET_CURRENT_PACKAGE, &args![])
            .u32();
        if target != player_pointer(e)
            && (target_package == 0
                || (e.call(PACKAGE_TYPE, &args![target_package]).i32() != 1
                    && !e
                        .call(PACKAGE_GET_IS_CREATED, &args![target_package])
                        .bool()))
        {
            e.vcall(process, SLOT_SET_PROCEDURE_INDEX_RUNNING, &args![0u32]);
            escort_free_item(e, item);
            return;
        }
        e.vcall(process, SLOT_CLEAR_ALL_HEAD_TRACK_TARGETS, &args![]);
        e.call(ACTOR_CLEAR_LOOK_AT_TARGET, &args![actor]);
    }
    let package_target = e.call(PACKAGE_TARGET_WORD, &args![package]).u32();
    if package_target != 0 {
        let kind = e
            .call(PACKAGE_TARGET_GET_TYPE, &args![package_target])
            .i32();
        let reference_kind = kind == 0 || kind == 3;
        if has_escorted && reference_kind {
            let target = e.mem.u32(process + LOW_TARGET);
            if target == 0 || e.call(REFERENCE_FLAG_800, &args![target]).bool() {
                e.vcall(
                    process,
                    SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                    &args![actor, 1u32],
                );
                return;
            }
            if e.call(REFERENCE_FLAG_20, &args![target]).bool() {
                let extra = e.call(REFERENCE_EXTRA_DATA, &args![target]).u32();
                if e.call(EXTRA_DATA_REFERENCE_POINTER, &args![extra]).u32() == 0 {
                    e.call(PACKAGE_SET_NEVER_RUN, &args![package, target, 1u32]);
                    e.vcall(
                        process,
                        SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                        &args![actor, 1u32],
                    );
                    return;
                }
            }
        }
    }
    let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
    if animation != 0
        && e.call(ACTOR_IS_PLAYING_LOWER_BODY_SPECIAL_IDLE, &args![actor])
            .bool()
    {
        e.call(ANIMATION_SPECIAL_IDLE_FREE, &args![animation, 1u32, 0u32]);
    }
    let mut should_wait = false;
    e.mem.set_u8(wait_flag, 0);
    if has_escorted {
        let world = e.call(PACKAGE_LOCATION_WORLD, &args![package, actor]).u32();
        let cell = e.call(PACKAGE_LOCATION_CELL, &args![package, actor]).u32();
        should_wait = e.with_stack(12, |e, coordinates| {
            let coordinates = e
                .call(PACKAGE_LOCATION_COORD, &args![package, coordinates, actor])
                .u32();
            e.with_stack(0x34, |e, location| {
                e.call(
                    PATHING_LOCATION_FROM_COORD,
                    &args![location, coordinates, cell, world],
                );
                let wait = high_process_should_wait_for_escort_target(
                    e,
                    this,
                    actor,
                    Ptr::new(escorted),
                    location,
                    Ptr::new(wait_flag),
                );
                e.call(PATHING_LOCATION_DESTRUCTOR, &args![location]);
                wait
            })
        });
    }
    let package_location = e.call(PACKAGE_LOCATION_WORD, &args![package]).u32();
    let moves_on = package_location != 0
        && e.vcall(
            package,
            PACKAGE_SLOT_0X13C,
            &args![actor, 0u32, e.global::<f32>(NO_LIMIT_FLOAT), 0u32],
        )
        .bool();
    if moves_on {
        if !has_escorted {
            let animation = e.vcall(a, ACTOR_SLOT_ANIMATION, &args![]).u32();
            if animation != 0
                && !e
                    .call(ANIMATION_SPECIAL_IDLE_DONE_PLAYING, &args![animation])
                    .bool()
            {
                if e.get(this, HighProcess::bActivateAnim) != 0 {
                    escort_free_item(e, item);
                    return;
                }
                e.call(ANIMATION_SPECIAL_IDLE_FREE, &args![animation, 1u32, 0u32]);
            }
            let package_location = e.call(PACKAGE_LOCATION_WORD, &args![package]).u32();
            let package_target = e.call(PACKAGE_TARGET_WORD, &args![package]).u32();
            if item != 0 {
                let mut first = escort_first_entry(e, item, 0);
                let mut target_form = 0;
                if first != 0 && e.call(EXTRA_DATA_REFERENCE_POINTER, &args![first]).u32() != 0 {
                    let reference = e.call(EXTRA_DATA_REFERENCE_POINTER, &args![first]).u32();
                    target_form = e.call(WORD_AT_0XC, &args![reference]).u32();
                }
                let location_word = e.call(PACKAGE_LOCATION_WORD, &args![package]).u32();
                let mut container = e
                    .call(PACKAGE_LOCATION_GET_REFERENCE, &args![location_word])
                    .u32();
                if container == 0 {
                    container = e.mem.u32(process + LOW_GENERIC_LOCATION);
                }
                let has_container =
                    container != 0 && e.call(REFERENCE_HAS_CONTAINER, &args![container]).u32() != 0;
                if has_container {
                    if e.get(this, HighProcess::bActivateAnim) == 0 {
                        e.set(this, HighProcess::bActivateAnim, 1);
                        let form = e.call(REFERENCE_GET_FORM, &args![container]).u32();
                        e.call(
                            FIND_SPECIAL_IDLE_TO_PLAY,
                            &args![this, actor, form, container],
                        );
                        escort_free_item(e, item);
                        return;
                    }
                    let count = e.call(PACKAGE_INITIAL_TARGET_COUNT, &args![package]).u32();
                    let amount = e.call(WORD_AT_8, &args![item]).u32();
                    e.call(
                        ACTOR_PUT_IN_CONTAINER,
                        &args![actor, amount, first, container, count, target_form],
                    );
                    e.set(this, HighProcess::bActivateAnim, 0);
                } else {
                    first = escort_first_entry(e, item, first);
                    let position = escort_location_position(e, process, package_location);
                    let count = e.call(PACKAGE_INITIAL_TARGET_COUNT, &args![package]).u32();
                    if e.get(this, HighProcess::bActivateAnim) == 0 {
                        e.set(this, HighProcess::bActivateAnim, 1);
                        let amount = e.call(WORD_AT_8, &args![item]).u32();
                        e.call(FIND_SPECIAL_IDLE_TO_PLAY, &args![this, actor, amount, 0u32]);
                        escort_free_item(e, item);
                        return;
                    }
                    let amount = e.call(WORD_AT_8, &args![item]).u32();
                    e.vcall(
                        a,
                        ACTOR_SLOT_0X3CC,
                        &args![amount, first, count, position, 0u32],
                    );
                    e.set(this, HighProcess::bActivateAnim, 0);
                }
                escort_free_item(e, item);
                let object_type = e
                    .call(PACKAGE_TARGET_GET_OBJECT_TYPE, &args![package_target])
                    .i32();
                if object_type == 0xc
                    || (e
                        .call(PACKAGE_TARGET_GET_OBJECT_TYPE, &args![package_target])
                        .i32()
                        > 0x12
                        && e.call(PACKAGE_TARGET_GET_OBJECT_TYPE, &args![package_target])
                            .i32()
                            < 0x18)
                {
                    while !e
                        .call(LIST_IS_EMPTY, &args![process + LOW_OBJECT_LIST])
                        .bool()
                    {
                        let node = e
                            .call(NODE_ITEM_ADDRESS, &args![process + LOW_OBJECT_LIST])
                            .u32();
                        let next = e.mem.u32(node);
                        e.mem.set_u32(process + LOW_ACQUIRE_OBJECT, next);
                        e.call(
                            LIST_REMOVE_ITEM,
                            &args![process + LOW_OBJECT_LIST, process + LOW_ACQUIRE_OBJECT],
                        );
                        let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
                        let wanted = e.mem.u32(entry);
                        e.vcall(process, SLOT_SET_TARGET, &args![wanted]);
                        let object = e.mem.u32(e.mem.u32(process + LOW_ACQUIRE_OBJECT) + 4);
                        item = e
                            .call(REFERENCE_GET_INVENTORY_ITEM, &args![actor, object, 0u32])
                            .u32();
                        let entry = e.mem.u32(process + LOW_ACQUIRE_OBJECT);
                        if entry != 0 {
                            e.call(ACQUIRE_OBJECT_DELETE, &args![entry, 1u32]);
                        }
                        e.mem.set_u32(process + LOW_ACQUIRE_OBJECT, 0);
                        if container != 0
                            && e.call(REFERENCE_HAS_CONTAINER, &args![container]).u32() != 0
                        {
                            let count = e.call(PACKAGE_INITIAL_TARGET_COUNT, &args![package]).u32();
                            let amount = e.call(WORD_AT_8, &args![item]).u32();
                            e.call(
                                ACTOR_PUT_IN_CONTAINER,
                                &args![actor, amount, first, container, count, target_form],
                            );
                        } else {
                            first = escort_first_entry(e, item, first);
                            let position = escort_location_position(e, process, package_location);
                            let count = e.call(PACKAGE_INITIAL_TARGET_COUNT, &args![package]).u32();
                            let amount = e.call(WORD_AT_8, &args![item]).u32();
                            e.vcall(
                                a,
                                ACTOR_SLOT_0X3CC,
                                &args![amount, first, count, position, 0u32],
                            );
                        }
                        escort_free_item(e, item);
                        // (the game also clears its local here; it is not read again)
                    }
                    let mut second_flag = true;
                    let first_flag = true;
                    let current = e.vcall(process, SLOT_GET_CURRENT_PACKAGE, &args![]).u32();
                    if current != 0 {
                        let current = e.vcall(process, SLOT_GET_CURRENT_PACKAGE, &args![]).u32();
                        if e.call(PACKAGE_FLAG_200000, &args![current]).bool() {
                            second_flag = false;
                        }
                    }
                    // The two flags handed to the form: `first_flag` is always 1, `second_flag` is 0 when the process' current package has the flag 0x200000.
                    if e.vcall(a, ACTOR_SLOT_0X218, &args![]).bool() {
                        let form = e.call(REFERENCE_GET_FORM, &args![actor]).u32();
                        if form != 0 {
                            e.call(
                                NPC_FORM_EQUIP,
                                &args![
                                    form,
                                    actor,
                                    u32::from(first_flag),
                                    u32::from(second_flag),
                                    0u32,
                                    1u32
                                ],
                            );
                        }
                    } else if e.vcall(a, ACTOR_SLOT_0X21C, &args![]).bool() {
                        let form = e.call(REFERENCE_GET_FORM, &args![actor]).u32();
                        if form != 0 {
                            e.call(
                                CREATURE_INIT_DEFAULT_WORN,
                                &args![
                                    form,
                                    actor,
                                    u32::from(first_flag),
                                    u32::from(second_flag),
                                    1u32
                                ],
                            );
                        }
                    }
                }
                e.vcall(process, SLOT_SET_CURRENT_ACTION_COMPLETE, &args![1u32]);
            }
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
        } else if !should_wait {
            let target = e.mem.u32(process + LOW_TARGET);
            if target != 0 {
                let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
                if target_process != 0 {
                    let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
                    let package_of_target = e
                        .vcall(target_process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
                        .u32();
                    if package_of_target != 0 {
                        let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
                        let running = e
                            .vcall(target_process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
                            .u32();
                        if e.call(PACKAGE_WORD_AT_0X18, &args![running]).i32() == 6 {
                            let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
                            e.vcall(
                                target_process,
                                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                                &args![target, 1u32],
                            );
                            let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
                            let running = e
                                .vcall(target_process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
                                .u32();
                            if e.call(PACKAGE_GET_IS_CREATED, &args![running]).bool() {
                                let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
                                e.vcall(target_process, SLOT_CLEAR_CURRENT_PACKAGE, &args![]);
                                if e.call(ACTOR_IS_PATHING_COMPLETE, &args![target]).bool() {
                                    e.call(ACTOR_END_MOVEMENT, &args![target]);
                                }
                                e.vcall(process, SLOT_SET_CURRENT_ACTION_COMPLETE, &args![1u32]);
                                e.vcall(process, SLOT_CHECK_FOR_NEW_PACKAGE, &args![actor, 0u32]);
                                e.vcall(
                                    process,
                                    SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                                    &args![actor, 1u32],
                                );
                                return;
                            }
                        }
                    }
                }
            }
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
        }
        if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
            e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
        }
        return;
    }
    if has_escorted {
        let mut check_pathing = should_wait;
        if !should_wait && escorted != player_pointer(e) {
            let escorted_package = e.call(ACTOR_CURRENT_PACKAGE, &args![escorted]).u32();
            if escorted_package != 0 {
                let escorted_package = e.call(ACTOR_CURRENT_PACKAGE, &args![escorted]).u32();
                if e.call(PACKAGE_IS_INTERRUPT, &args![escorted_package])
                    .bool()
                {
                    check_pathing = true;
                }
            }
        }
        if check_pathing && !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
            e.vcall(process, SLOT_SET_CURRENT_MOVEMENT_COMPLETE, &args![1u32]);
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 0xffff_ffffu32],
            );
            return;
        }
        let zero = 0.0f32;
        let radius = e
            .call(
                PACKAGE_RADIUS_ACTOR_TO_LOCATION,
                &args![package, actor, 0u32],
            )
            .f32();
        let doubled = (f64::from(radius) + f64::from(radius)) as f32;
        let mut flags = e
            .vcall(
                process,
                SLOT_CALCULATE_MOVE_MODE,
                &args![actor, zero, radius, doubled, 0u32, 0u32],
            )
            .u32();
        if e.mem.u8(wait_flag) != 0 {
            flags = (flags & 0xffff_fdff) | 0x100;
        } else if e
            .call(PROCESS_IS_CURRENT_WEAPON_GRENADE, &args![actor])
            .bool()
            || e.call(PACKAGE_TEST_67A480, &args![package]).bool()
        {
            flags = (flags & 0xffff_feff) | 0x200;
        }
        e.vcall(
            process,
            SLOT_SET_ACTORS_ANIMATION,
            &args![actor, flags, 0u32],
        );
    }
    if e.call(ACTOR_IS_PATHING, &args![actor]).bool() {
        return;
    }
    let state = e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32();
    if state == 4 || e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32() == 9 {
        e.vcall(a, ACTOR_SLOT_0X418, &args![]);
        return;
    }
    if has_escorted && should_wait {
        return;
    }
    let radius = e
        .call(
            PACKAGE_RADIUS_ACTOR_TO_LOCATION,
            &args![package, actor, 0u32],
        )
        .f32();
    e.with_stack(0xb4, |e, request| {
        e.call(PATHING_REQUEST_CONSTRUCTOR, &args![request]);
        let world = e.call(PACKAGE_LOCATION_WORLD, &args![package, actor]).u32();
        let cell = e.call(PACKAGE_LOCATION_CELL, &args![package, actor]).u32();
        e.with_stack(12, |e, coordinates| {
            let coordinates = e
                .call(PACKAGE_LOCATION_COORD, &args![package, coordinates, actor])
                .u32();
            e.call(
                ACTOR_BUILD_REQUEST,
                &args![actor, request, coordinates, cell, world, radius, 0u32],
            );
        });
        fn_008e0ab0(e, request, u8::from(escorted == player_pointer(e)));
        e.call(ACTOR_SET_PATHFINDING_GOAL_REQUEST, &args![actor, request]);
        e.call(PATHING_REQUEST_DESTRUCTOR, &args![request]);
    });
}

/// `Actor::ClearLookAtTarget` (Xbox PDB, `008b3d30`).
const ACTOR_CLEAR_LOOK_AT_TARGET: u32 = 0x008b_3d30;

// Translated from 008e0ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: stores `value` in the byte at +0x9f of `this`
/// (a `PathingRequest`: `ProcessEscort` passes whether the escorted actor is
/// the player).
pub fn fn_008e0ab0(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x9f, value);
}

// Translated from 008e0ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: forwards to `HighProcess::ProcessFollowOneHour`
/// (`008f31d0`) with `value` (a float) and the last argument 1.
pub fn fn_008e0ad0(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr, value: f32) {
    e.call(0x008f_31d0, &args![this, actor, value, 1u32]);
}

/// `008a6290`: bit 0x10000 of the flags at +0x1c of `this`.
const PACKAGE_FLAG_10000: u32 = 0x008a_6290;
/// `ActorMover` helper (`009de0a0`): writes the position the actor's mover
/// currently heads for into `out`.
const MOVER_POSITION: u32 = 0x009d_e0a0;
/// A square root (`004019d0`, cdecl, a float on the stack, `ST0` result).
const FLOAT_SQUARE_ROOT: u32 = 0x0040_19d0;
const SLOT_GET_CURRENT_DESTINATION_COORDINATE: u32 = 0x98;
const SLOT_GET_CURRENT_DESTINATION_CELL: u32 = 0x9c;
const SLOT_GET_CURRENT_DESTINATION_WORLD_SPACE: u32 = 0xa0;
/// Slot `0x22c` of an `Actor` (PC byte offset), called with 0 by
/// `ProcessAccompany`; the other slot `0x22c` it uses, on the process, is
/// `GetCurrentPackage`.
const ACTOR_SLOT_0X22C: u32 = 0x22c;
/// `Actor`'s mover pointer, at +0x190.
const ACTOR_MOVER: u32 = 0x190;
/// The doubles `ProcessAccompany` reads: `01077e08` (300.0) and `01021928`
/// (3.0).
const ACCOMPANY_EXTRA_DISTANCE: u32 = 0x0107_7e08;
const ACCOMPANY_RUN_FACTOR: u32 = 0x0102_1928;

// Translated from 008e0b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessAccompany` (Xbox PDB): the actor accompanies the
/// process' target. Does nothing without a package or when the package has
/// the flag 0x10000. The process gets a target for the package when its
/// target is not an actor; without an actor that is carrying the item
/// (`00575450`) only procedure 1 is added and the movement ended.
///
/// For an actor target with a package, the destination the target's own
/// process heads for (coordinates, cell and world space, slots `0x98`,
/// `0x9c` and `0xa0`) is taken, and a target that is dead (actor slot
/// `0x22c`) or has the reference flag `0x800` ends it like above. The
/// squared distances from the actor to the destination, from the target to
/// its destination, between the two actors and between their movers'
/// positions are compared with the squared escort follow distance `d` of the
/// target's package: a pathing-complete actor farther than that from the
/// destination or from the target ends its movement (message) and, farther
/// than `d + 300` from the target, adds the procedure -1; otherwise, when it
/// is not pathing and the target is farther than `d`, the actor runs (move
/// mode of the target, `CalculateMoveMode(actor, sqrt, d, 3d, 0, 0)`) to the
/// target's mover position.
pub fn high_process_process_accompany(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if package == 0 || e.call(PACKAGE_FLAG_10000, &args![package]).bool() {
        return;
    }
    let mut target = e.mem.u32(process + LOW_TARGET);
    let mut accompany = target != 0 && e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool();
    if !accompany {
        e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
        target = e.mem.u32(process + LOW_TARGET);
        if target != 0 {
            if e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
                accompany = true;
            } else {
                let word = e.call(WORD_AT_0XC, &args![target]).u32();
                if e.call(REFERENCE_HOLDS_OBJECT, &args![actor, word]).bool() {
                    accompany = true;
                }
            }
        }
        if !accompany {
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
            if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
                e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            }
            return;
        }
    }
    let target = e.mem.u32(process + LOW_TARGET);
    if target == 0 {
        return;
    }
    let target_process = e.call(ACTOR_PROCESS, &args![target]).u32();
    if e.call(ACTOR_PROCESS, &args![target]).u32() == 0 {
        return;
    }
    let target_package = e
        .vcall(target_process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    if target_package == 0 {
        return;
    }
    e.with_stack(12, |e, destination| {
        e.vcall(
            target_process,
            SLOT_GET_CURRENT_DESTINATION_COORDINATE,
            &args![destination, target, 1u32],
        );
        let cell = e
            .vcall(
                target_process,
                SLOT_GET_CURRENT_DESTINATION_CELL,
                &args![target],
            )
            .u32();
        let world_space = e
            .vcall(
                target_process,
                SLOT_GET_CURRENT_DESTINATION_WORLD_SPACE,
                &args![target],
            )
            .u32();
        if e.vcall(target, ACTOR_SLOT_0X22C, &args![0u32]).bool()
            || e.call(REFERENCE_FLAG_800, &args![target]).bool()
        {
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
            if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
                e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            }
            return;
        }
        let my_position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
        let target_position = e.vcall(target, ACTOR_SLOT_POSITION, &args![]).u32();
        e.with_stack(12, |e, my_mover_position| {
            let mover = e.mem.u32(a + ACTOR_MOVER);
            e.call(MOVER_POSITION, &args![mover, my_mover_position]);
            e.with_stack(12, |e, target_mover_position| {
                let mover = e.mem.u32(target + ACTOR_MOVER);
                e.call(MOVER_POSITION, &args![mover, target_mover_position]);
                // Four differences (destination - me, destination - target,
                // target - me, target mover - my mover) and, after the follow
                // distance, their squared lengths.
                let differences = e.mem.alloc(48);
                let (to_me, to_target, between, mover_gap) = (
                    differences,
                    differences + 12,
                    differences + 24,
                    differences + 36,
                );
                e.call(POINT_DIFFERENCE, &args![destination, to_me, my_position]);
                e.call(
                    POINT_DIFFERENCE,
                    &args![destination, to_target, target_position],
                );
                e.call(
                    POINT_DIFFERENCE,
                    &args![target_position, between, my_position],
                );
                e.call(
                    POINT_DIFFERENCE,
                    &args![target_mover_position, mover_gap, my_mover_position],
                );
                let follow_distance = e
                    .call(PACKAGE_ESCORT_FOLLOW_DISTANCE, &args![target_package])
                    .i32() as f32;
                let squared_follow =
                    (f64::from(follow_distance) * f64::from(follow_distance)) as f32;
                let destination_to_me = e.call(POINT_SQUARED_LENGTH, &args![to_me]).f32();
                let destination_to_target = e.call(POINT_SQUARED_LENGTH, &args![to_target]).f32();
                let target_to_me = e.call(POINT_SQUARED_LENGTH, &args![between]).f32();
                let movers = e.call(POINT_SQUARED_LENGTH, &args![mover_gap]).f32();
                e.mem.free(differences);
                let complete = e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool();
                // `FCOMPP` tests: strictly greater, false when unordered.
                let farther =
                    destination_to_target > destination_to_me || squared_follow > target_to_me;
                if complete && farther {
                    e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
                    let distance = e
                        .call(DISTANCE_FROM_REFERENCE, &args![actor, target, 0u32, 0u32])
                        .f64();
                    let limit =
                        f64::from(follow_distance) + e.global::<f64>(ACCOMPANY_EXTRA_DISTANCE);
                    if limit < distance {
                        e.vcall(
                            process,
                            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                            &args![actor, 0xffff_ffffu32],
                        );
                    }
                } else if !e.call(ACTOR_IS_PATHING, &args![actor]).bool() && squared_follow < movers
                {
                    let root = e.call(FLOAT_SQUARE_ROOT, &args![movers]).f32();
                    let run =
                        (f64::from(follow_distance) * e.global::<f64>(ACCOMPANY_RUN_FACTOR)) as f32;
                    e.vcall(
                        process,
                        SLOT_CALCULATE_MOVE_MODE,
                        &args![actor, root, follow_distance, run, 0u32, 0u32],
                    );
                    let mode = e.call(ACTOR_MOVE_MODE, &args![target]).u32();
                    e.call(ACTOR_SET_MOVE_MODE, &args![actor, mode]);
                    e.call(
                        ACTOR_SET_PATHFINDING_GOAL,
                        &args![
                            actor,
                            target_mover_position,
                            cell,
                            world_space,
                            follow_distance,
                            0u32
                        ],
                    );
                }
            });
        });
    });
}

// Translated from 008e30e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: whether bit 0x100 of the word at +0x24 of `this`
/// is set.
pub fn fn_008e30e0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u16(this.addr() + 0x24) & 0x100 != 0
}

// Translated from 008e3100 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: whether bit 0x200 of the word at +0x24 of `this`
/// is set.
pub fn fn_008e3100(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u16(this.addr() + 0x24) & 0x200 != 0
}

// Translated from 008e3120 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: whether bit 0x400 of the word at +0x24 of `this`
/// is set.
pub fn fn_008e3120(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u16(this.addr() + 0x24) & 0x400 != 0
}

/// A random float (`005c5420`, `ST0` result).
const RANDOM_FLOAT: u32 = 0x005c_5420;
/// `NiPoint3` constructor from three floats (`00416870`).
const POINT_CONSTRUCT: u32 = 0x0041_6870;
/// `NiPoint3::Unitize` (`004a0c10`).
const POINT_UNITIZE: u32 = 0x004a_0c10;
/// `NiPoint3` times a float (`0045bb20`): `(this, out, factor)`.
const POINT_SCALE: u32 = 0x0045_bb20;
/// `NiPoint3` sum (`00439e90`): `(this, out, other)`.
const POINT_SUM: u32 = 0x0043_9e90;

// Translated from 008e3dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (cdecl): writes to `out` the point `point` plus a
/// random offset and returns `out`. The offset is a random direction (two
/// random floats, a third one when `flag` is set, 0 otherwise), unitized, times `max((a + b) * random, b)`.
#[allow(clippy::too_many_arguments)]
pub fn fn_008e3dc0(
    e: &mut Engine,
    out: Ptr,
    point_x: u32,
    point_y: u32,
    point_z: u32,
    a: f32,
    b: f32,
    flag: u8,
) -> Ptr {
    let x = e.call(RANDOM_FLOAT, &args![]).f32();
    let y = e.call(RANDOM_FLOAT, &args![]).f32();
    let mut z = 0.0f32;
    if flag != 0 {
        z = e.call(RANDOM_FLOAT, &args![]).f32();
    }
    e.with_stack(12, |e, direction| {
        e.call(POINT_CONSTRUCT, &args![direction, x, y, z]);
        e.call(POINT_UNITIZE, &args![direction]);
        let random = e.call(RANDOM_FLOAT, &args![]).f32();
        let mut length = ((f64::from(a) + f64::from(b)) * f64::from(random)) as f32;
        if b > length {
            length = b;
        }
        e.with_stack(12, |e, scaled| {
            let scaled = e.call(POINT_SCALE, &args![direction, scaled, length]).u32();
            e.with_stack(12, |e, point| {
                e.mem.set_u32(point.addr(), point_x);
                e.mem.set_u32(point.addr() + 4, point_y);
                e.mem.set_u32(point.addr() + 8, point_z);
                e.with_stack(12, |e, sum| {
                    let sum = e.call(POINT_SUM, &args![point, sum, scaled]).u32();
                    for word in 0..3 {
                        let value = e.mem.u32(sum + word * 4);
                        e.mem.set_u32(out.addr() + word * 4, value);
                    }
                });
            });
        });
    });
    out
}

// Translated from 008e3e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: returns false.
pub fn fn_008e3e90(_e: &mut Engine) -> bool {
    false
}

/// The list at `011e025c` that `008e3ea0` adds references to (`BSSimpleList`
/// add, `005ae3d0`: `this`, the address of a word holding the item).
const REFERENCE_LIST_ADD: u32 = 0x005a_e3d0;
const REFERENCE_LIST: u32 = 0x011e_025c;
/// The form type byte (`00401170`: the byte at +4 of the form).
const FORM_TYPE_BYTE: u32 = 0x0040_1170;
/// `004077c0`: bit 0x4000 of the flags at +8 of `this`.
const REFERENCE_FLAG_4000: u32 = 0x0040_77c0;
/// `00568e50`: the reference's extra data list entry (see `005d43c0`).
const REFERENCE_EXTRA_DATA_ENTRY: u32 = 0x0056_8e50;

// Translated from 008e3ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (cdecl): for a reference that is not flagged
/// 0x20 or 0x4000, whose form type byte (`00401170`) is 0x1c and that has an
/// extra data entry (`00568e50`), adds the reference to the list at
/// `011e025c`. Always returns false.
pub fn fn_008e3ea0(e: &mut Engine, reference: Ptr) -> bool {
    if reference.is_null() {
        return false;
    }
    if e.call(REFERENCE_FLAG_20, &args![reference]).bool()
        || e.call(REFERENCE_FLAG_4000, &args![reference]).bool()
    {
        return false;
    }
    let form = e.call(REFERENCE_GET_FORM, &args![reference]).u32();
    if e.call(FORM_TYPE_BYTE, &args![form]).i32() == 0x1c
        && e.call(REFERENCE_EXTRA_DATA_ENTRY, &args![reference]).u32() != 0
    {
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), reference.addr());
            e.call(REFERENCE_LIST_ADD, &args![REFERENCE_LIST, slot]);
        });
    }
    false
}

// Translated from 008e40a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ReduceDetectionTimer` (Xbox PDB): lowers `fDetectionTimer`
/// (+0x2f8) by the frame time.
pub fn high_process_reduce_detection_timer(e: &mut Engine, this: Ptr<HighProcess>) {
    let frame = e.call(FRAME_TIME, &args![FRAME_TIMER]).f64();
    let timer = e.mem.f32(this.addr() + 0x2f8);
    e.mem
        .set_f32(this.addr() + 0x2f8, (f64::from(timer) - frame) as f32);
}

/// `bhkBoxShape` instance counter (`0126825c`).
const BOX_SHAPE_COUNT: u32 = 0x0126_825c;
/// `bhkConvexShape` constructor (`0056e690`).
const CONVEX_SHAPE_CONSTRUCTOR: u32 = 0x0056_e690;
/// `NiPoint3` to Havok vector conversion (`004a3e00`, cdecl: `(out, point)`).
const POINT_TO_HAVOK_VECTOR: u32 = 0x004a_3e00;
/// Sets the shape's dimensions from a Havok vector (`0056e810`).
const BOX_SHAPE_SET_DIMENSIONS: u32 = 0x0056_e810;

// Translated from 008e5100 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; the vtable it installs (`0103093c`) is that of
/// `bhkBoxShape` (RTTI), so this is `bhkBoxShape`'s constructor taking the
/// half extents as a `NiPoint3` pointer: base constructor (`0056e690`), vtable,
/// instance counter `0126825c` + 1, the point converted to a Havok vector
/// (`004a3e00`) and set as the dimensions (`0056e810`). Returns `this`. The
/// SEH frame and the stack realignment are not translated.
pub fn fn_008e5100(e: &mut Engine, this: Ptr, half_extents: Ptr) -> Ptr {
    e.call(CONVEX_SHAPE_CONSTRUCTOR, &args![this]);
    e.mem.set_u32(this.addr(), 0x0103_093c);
    let count = e.mem.u32(BOX_SHAPE_COUNT);
    e.mem.set_u32(BOX_SHAPE_COUNT, count.wrapping_add(1));
    e.with_stack(0x18, |e, vector| {
        e.call(NODE_ITEM_ADDRESS, &args![vector]);
        let vector = e
            .call(POINT_TO_HAVOK_VECTOR, &args![vector, half_extents])
            .u32();
        e.call(BOX_SHAPE_SET_DIMENSIONS, &args![this, vector]);
    });
    this
}

/// `bhkCharacterProxy::operatorP` (engine map name, `004ae750`): the actor's
/// character controller or null.
const ACTOR_CHARACTER_CONTROLLER: u32 = 0x004a_e750;
/// The vector at +0x20 of the controller (`00891170`).
const CONTROLLER_VECTOR: u32 = 0x0089_1170;
/// A default Havok vector (`00458b20`: the address `01267e30`).
const DEFAULT_VECTOR: u32 = 0x0045_8b20;
/// Havok vector to `NiPoint3` (`00458620`, cdecl: `(out, vector)`).
const HAVOK_VECTOR_TO_POINT: u32 = 0x0045_8620;

// Translated from 008e5620 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: writes to `out` the `NiPoint3` made from the
/// Havok vector `008e5670` finds for `this` and returns `out`.
pub fn fn_008e5620(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    e.with_stack(12, |e, point| {
        e.call(NODE_ITEM_ADDRESS, &args![point]);
        let vector = fn_008e5670(e, this);
        let converted = e.call(HAVOK_VECTOR_TO_POINT, &args![point, vector]).u32();
        for word in 0..3 {
            let value = e.mem.u32(converted + word * 4);
            e.mem.set_u32(out.addr() + word * 4, value);
        }
    });
    out
}

// Translated from 008e5670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: the Havok vector of `this` (an actor): the
/// vector at +0x20 of its character controller (`004ae750`, `00891170`), or
/// the default vector (`00458b20`) without one.
pub fn fn_008e5670(e: &mut Engine, this: Ptr) -> u32 {
    let controller = e.call(ACTOR_CHARACTER_CONTROLLER, &args![this]).u32();
    if controller != 0 {
        e.call(CONTROLLER_VECTOR, &args![controller]).u32()
    } else {
        e.call(DEFAULT_VECTOR, &args![]).u32()
    }
}

/// `HighProcess::spShapePhantom` (Xbox PDB): `NiPointer<bhkSimpleShapePhantom>`
/// at +0x434.
const SHAPE_PHANTOM: u32 = 0x434;
/// Slot `0xa0` of the phantom object.
const PHANTOM_SLOT_0XA0: u32 = 0xa0;

// Translated from 008e56b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: when the process holds a shape phantom
/// (`spShapePhantom`, +0x434), calls its virtual `+0xa0` and releases it
/// (assigns null).
pub fn fn_008e56b0(e: &mut Engine, this: Ptr<HighProcess>) {
    let slot = this.addr() + SHAPE_PHANTOM;
    if e.call(NI_POINTER_GET, &args![slot]).u32() != 0 {
        let phantom = e.call(NI_POINTER_GET, &args![slot]).u32();
        e.vcall(phantom, PHANTOM_SLOT_0XA0, &args![]);
        e.call(NI_POINTER_SET, &args![slot, 0u32]);
    }
}

// Translated from 008e5700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: whether the process holds a shape phantom
/// (`spShapePhantom`, +0x434).
pub fn fn_008e5700(e: &mut Engine, this: Ptr<HighProcess>) -> bool {
    e.call(NI_POINTER_GET, &args![this.addr() + SHAPE_PHANTOM])
        .u32()
        != 0
}

/// `MiddleHighProcess` members (Xbox PDB): `RefListChairBed`
/// (`BSSimpleList<TESObjectREFR *>`, +0xd0), `pCurrentFurniture`
/// (`TESObjectREFR*`, +0x140) and `nFurnitureListTimer` (`u32`, +0x258).
const PROCESS_CHAIR_BED_LIST: u32 = 0xd0;
const PROCESS_CURRENT_FURNITURE: u32 = 0x140;
const PROCESS_FURNITURE_LIST_TIMER: u32 = 0x258;
/// `Actor::GetOutofFurnitureQuick` (Xbox PDB, `0088d640`).
const ACTOR_GET_OUT_OF_FURNITURE_QUICK: u32 = 0x0088_d640;
/// `MiddleHighProcess::ClearFurnitureMarker` (Xbox PDB, `0092c680`).
const PROCESS_CLEAR_FURNITURE_MARKER: u32 = 0x0092_c680;
/// `MiddleHighProcess::FindBedChairs` (Xbox PDB, `00922670`): `(this, actor,
/// 0, 0)`.
const PROCESS_FIND_BED_CHAIRS: u32 = 0x0092_2670;
/// `VATS::GetCount` (engine map name, `005ae380`): the number of items of the
/// `BSSimpleList` in `this`.
const LIST_COUNT: u32 = 0x005a_e380;
/// `0044b130`, called on the object at `011e043c`.
const FURNITURE_GATE_CHECK: u32 = 0x0044_b130;
/// `0082f1f0`, called on the object at `011e043c`.
const FURNITURE_GATE_RELEASE: u32 = 0x0082_f1f0;
const FURNITURE_GATE_OBJECT: u32 = 0x011e_043c;
/// `TESObjectREFR::HasFreeMarker` (Xbox PDB, `00568260`).
const REFERENCE_HAS_FREE_MARKER: u32 = 0x0056_8260;
/// `TESObjectREFR::GetOwner` (Xbox PDB, `00567790`).
const REFERENCE_GET_OWNER: u32 = 0x0056_7790;
/// A random number (`00487f50`).
const RANDOM_NUMBER: u32 = 0x0048_7f50;
/// `00915ef0` on the process (`this`, actor).
const PROCESS_WANTS_TO_FINISH_SLEEP: u32 = 0x0091_5ef0;
/// `Actor::GetFaceAnimationData` (Xbox PDB, `008adcb0`).
const ACTOR_GET_FACE_ANIMATION_DATA: u32 = 0x008a_dcb0;
/// The game global read by `ProcessSleep` (`011ca28c`) against the furniture
/// list timer.
const FURNITURE_TIME_GLOBAL: u32 = 0x011c_a28c;
/// The float stored into `fEvaluateAcquireTimer` (`01017868`).
const SLEEP_ACQUIRE_TIMER_VALUE: u32 = 0x0101_7868;
const SLOT_GET_SIT_SLEEP_STATE: u32 = 0x4bc;
const SLOT_PROCESS_ACTIVATE: u32 = 0x7c8;
const SLOT_CLEAR_FURNITURE: u32 = 0x84;
/// Slots of the actor's face animation data (`0xd4`: a flag, `0xd8`:
/// `(1, 0)`).
const FACE_DATA_SLOT_0XD4: u32 = 0xd4;
const FACE_DATA_SLOT_0XD8: u32 = 0xd8;

/// The furniture choice `ProcessSleep` and `ProcessEat` share. Without a
/// furniture (`pCurrentFurniture`, +0x140), with candidates in the chair and
/// bed list (+0xd0) and with the gate at `011e043c` open, picks the first
/// candidate that has a free marker (removing the ones that have not) or,
/// when that one has no owner, a random one of the list, and releases the
/// gate.
fn process_pick_furniture(e: &mut Engine, this: Ptr<HighProcess>) {
    let process = this.addr();
    let list = process + PROCESS_CHAIR_BED_LIST;
    let furniture = process + PROCESS_CURRENT_FURNITURE;
    if e.mem.u32(furniture) == 0
        && e.call(LIST_COUNT, &args![list]).u32() != 0
        && e.call(FURNITURE_GATE_CHECK, &args![FURNITURE_GATE_OBJECT])
            .bool()
    {
        let node = e.call(NODE_ITEM_ADDRESS, &args![list]).u32();
        let first = e.mem.u32(node);
        e.mem.set_u32(furniture, first);
        e.call(PROCESS_CLEAR_FURNITURE_MARKER, &args![this]);
        while e.mem.u32(furniture) != 0
            && !e
                .call(
                    REFERENCE_HAS_FREE_MARKER,
                    &args![e.mem.u32(furniture), 0u32],
                )
                .bool()
        {
            e.call(LIST_REMOVE_ITEM, &args![list, furniture]);
            let node = e.call(NODE_ITEM_ADDRESS, &args![list]).u32();
            let next = e.mem.u32(node);
            e.mem.set_u32(furniture, next);
            e.call(PROCESS_CLEAR_FURNITURE_MARKER, &args![this]);
        }
        if e.mem.u32(furniture) != 0
            && e.call(REFERENCE_GET_OWNER, &args![e.mem.u32(furniture)])
                .u32()
                == 0
        {
            let count = e.call(LIST_COUNT, &args![list]).u32();
            let random = e.call(RANDOM_NUMBER, &args![]).u32();
            let mut steps = random % count;
            if steps as i32 >= count as i32 {
                steps = count;
            }
            let mut cursor = list;
            let mut step = 0;
            while step < steps as i32 {
                cursor = e.call(NODE_NEXT, &args![cursor]).u32();
                step += 1;
            }
            let node = e.call(NODE_ITEM_ADDRESS, &args![cursor]).u32();
            let chosen = e.mem.u32(node);
            e.mem.set_u32(furniture, chosen);
            e.call(PROCESS_CLEAR_FURNITURE_MARKER, &args![this]);
        }
        e.call(FURNITURE_GATE_RELEASE, &args![FURNITURE_GATE_OBJECT]);
    }
}

// Translated from 008e26e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessSleep` (Xbox PDB): the sleeping package.
///
/// Without a furniture (`pCurrentFurniture`, +0x140) the actor gets out of
/// furniture quickly (when its slot `0x214` is not 0) and the furniture
/// marker is cleared. A creature-state actor (slot `0x218`) without sit/sleep
/// state (slot `0x4bc` of the process) and furniture looks for beds and
/// chairs when the list is empty or the time global `011ca28c` has reached
/// `nFurnitureListTimer`. For an actor not in state 9: in state 4 it calls
/// its slot `0x418` and stops; otherwise, without furniture but with a list
/// of candidates and with the gate at `011e043c` open, it picks the first
/// furniture of the list that has a free marker (removing the ones that have
/// not), or, when the pick has no owner, a random one of the list, makes it
/// the target of the package (when there is none) and activates it (virtual
/// `0x7c8` with the actor). In state 9 the current action completes and the
/// furniture list is cleared; otherwise an actor without furniture and an
/// empty list ends its movement, sets `fEvaluateAcquireTimer` (+0x2e0) from
/// `01017868` and adds procedure 1, and an actor for whom `00915ef0` holds
/// clears its furniture (virtual `0x84`), ends the movement and adds
/// procedure 1. A sleeping actor (state 9) whose face data is not busy
/// (slot `0xd4`) gets slot `0xd8` called with (1, 0).
pub fn high_process_process_sleep(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let process = this.addr();
    let a = actor.addr();
    let list = process + PROCESS_CHAIR_BED_LIST;
    let furniture = process + PROCESS_CURRENT_FURNITURE;
    e.vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![]);
    if e.mem.u32(furniture) == 0 {
        if e.vcall(a, ACTOR_SLOT_STATE, &args![]).u32() != 0 {
            e.call(ACTOR_GET_OUT_OF_FURNITURE_QUICK, &args![actor]);
        }
        e.call(PROCESS_CLEAR_FURNITURE_MARKER, &args![this]);
    }
    if e.vcall(a, ACTOR_SLOT_0X218, &args![]).bool()
        && e.vcall(process, SLOT_GET_SIT_SLEEP_STATE, &args![]).u32() == 0
        && e.mem.u32(furniture) == 0
        && (e.call(LIST_IS_EMPTY, &args![list]).bool()
            || e.global::<u32>(FURNITURE_TIME_GLOBAL)
                >= e.mem.u32(process + PROCESS_FURNITURE_LIST_TIMER))
    {
        e.call(PROCESS_FIND_BED_CHAIRS, &args![this, actor, 0u32, 0u32]);
    }
    if e.vcall(a, ACTOR_SLOT_STATE, &args![]).u32() != 9
        && e.vcall(a, ACTOR_SLOT_0X218, &args![]).bool()
    {
        if e.vcall(a, ACTOR_SLOT_STATE, &args![]).u32() == 4 {
            e.vcall(a, ACTOR_SLOT_0X418, &args![]);
            return;
        }
        process_pick_furniture(e, this);
        if e.mem.u32(furniture) != 0 {
            if e.mem.u32(process + LOW_TARGET) == 0 {
                let chosen = e.mem.u32(furniture);
                e.vcall(process, SLOT_SET_TARGET, &args![chosen]);
            }
            e.vcall(process, SLOT_PROCESS_ACTIVATE, &args![actor, 0u32]);
        }
    }
    if e.vcall(a, ACTOR_SLOT_STATE, &args![]).u32() == 9 {
        e.vcall(process, SLOT_SET_CURRENT_ACTION_COMPLETE, &args![1u32]);
        e.call(LIST_CLEAR, &args![list]);
    } else {
        if e.mem.u32(furniture) == 0
            && e.call(LIST_IS_EMPTY, &args![list]).bool()
            && e.vcall(a, ACTOR_SLOT_0X218, &args![]).bool()
        {
            e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            let value = e.global::<f32>(SLEEP_ACQUIRE_TIMER_VALUE);
            e.mem.set_f32(process + 0x2e0, value);
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
            return;
        }
        if e.call(PROCESS_WANTS_TO_FINISH_SLEEP, &args![this, actor])
            .bool()
        {
            e.vcall(process, SLOT_CLEAR_FURNITURE, &args![actor]);
            e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
            return;
        }
    }
    if e.vcall(a, ACTOR_SLOT_STATE, &args![]).u32() == 9 {
        let face = e.call(ACTOR_GET_FACE_ANIMATION_DATA, &args![actor]).u32();
        if face != 0 && !e.vcall(face, FACE_DATA_SLOT_0XD4, &args![]).bool() {
            e.vcall(face, FACE_DATA_SLOT_0XD8, &args![1u32, 0u32]);
        }
    }
}

/// `MiddleHighProcess` members (Xbox PDB): `TempObjectList`
/// (`BSSimpleList<ObjectstoAcquire *>`, +0x74); `ObjectList` is at +0x5c.
const PROCESS_TEMP_OBJECT_LIST: u32 = 0x74;
/// `ObjectstoAcquire` fields the evaluation sets: the reference (+0x0), the
/// form (+0x4), two flag bytes (+0x8, +0x9), the distance (+0x10) and the
/// kind (+0x18).
const ACQUIRE_REFERENCE: u32 = 0x0;
const ACQUIRE_FORM: u32 = 0x4;
const ACQUIRE_FLAG_A: u32 = 0x8;
const ACQUIRE_FLAG_B: u32 = 0x9;
const ACQUIRE_DISTANCE: u32 = 0x10;
const ACQUIRE_KIND: u32 = 0x18;
/// `TESObjectREFR::IsAnOwner` (Xbox PDB, `005785e0`): `(this, actor, flag)`.
const REFERENCE_IS_AN_OWNER: u32 = 0x0057_85e0;
/// `MiddleHighProcess::GetForceNextUpdate` (Xbox PDB, `00566950`).
const PROCESS_GET_FORCE_NEXT_UPDATE: u32 = 0x0056_6950;
/// `ProcessLists::GetActorRefInHigh` (Xbox PDB, `00970a20`): the instance is
/// `011e0e80`; arguments `(form, flag)`.
const PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH: u32 = 0x0097_0a20;
const PROCESS_LISTS_INSTANCE: u32 = 0x011e_0e80;
/// `004181e0`: the form-side object of a reference (`GetForm`).
const REFERENCE_AI_FORM: u32 = 0x0041_81e0;
/// `TESAIForm::SellBuysItem` (Xbox PDB, `0047f600`).
const AI_FORM_SELL_BUYS_ITEM: u32 = 0x0047_f600;
/// `TESPackage::GetObjectTypeFromForm` (Xbox PDB, `00679ba0`, cdecl).
const PACKAGE_OBJECT_TYPE_FROM_FORM: u32 = 0x0067_9ba0;
/// `TESValueForm::GetFormValue` (Xbox PDB, `0048e8a0`, cdecl).
const FORM_VALUE: u32 = 0x0048_e8a0;
/// `Actor::GetGoldAmount` (Xbox PDB, `00891d70`).
const ACTOR_GOLD_AMOUNT: u32 = 0x0089_1d70;
/// `00493bb0` of the actor (the creature-like test `ProcessGreet` uses).
const ACTOR_CREATURE_LIKE: u32 = 0x0049_3bb0;
/// `Actor::HasObjects` (Xbox PDB, `00891db0`): `(this, form, 0, 1, 0, out)`.
const ACTOR_HAS_OBJECTS: u32 = 0x0089_1db0;
/// `TESObjectREFR::GetLock` (Xbox PDB, `00569160`).
const REFERENCE_GET_LOCK: u32 = 0x0056_9160;
/// `TESObjectREFR::SetTargeted` (Xbox PDB, `00564db0`).
const REFERENCE_SET_TARGETED: u32 = 0x0056_4db0;
/// The game form global `HasObjects` is asked about (`011ca268`).
const KEY_FORM_GLOBAL: u32 = 0x011c_a268;
/// `MiddleHighProcess` calls ending the evaluation (`0091b580`, `0091b640`).
const PROCESS_EVALUATION_END_A: u32 = 0x0091_b580;
const PROCESS_EVALUATION_END_B: u32 = 0x0091_b640;
/// `GetDistanceFromReference(actor, reference, 0, 0)` as the game stores it:
/// `_ftol2` of the distance.
fn acquire_distance_word(e: &mut Engine, actor: Ptr, reference: u32) -> u32 {
    let distance = e
        .call(
            DISTANCE_FROM_REFERENCE,
            &args![actor, reference, 0u32, 0u32],
        )
        .f64();
    e.call(FLOAT_TO_INT, &args![distance]).u32()
}

// Translated from 008e2b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::EvaluateOrderAcquireList` (Xbox PDB): goes through the
/// process' `TempObjectList` (+0x74) of `ObjectstoAcquire` entries (reference,
/// form, flag bytes +0x8/+0x9, distance +0x10, kind +0x18) and moves the ones
/// worth keeping to the `ObjectList` (+0x5c), marking their reference as
/// targeted; the others are deleted (`007b3fa0`). The running package gives
/// three flags: `buys` (`008e30e0`), `008e3120` and `008e3100`. At the end the
/// two hooks `0091b580` and `0091b640` run on the process and the temporary
/// list is cleared.
///
/// Per entry: a form of type 0x2a or 0x2b is kept untouched. A reference that
/// is owned by nobody (or by `actor`) and is not an actor only gets its
/// distance from `actor`. Otherwise the owner actor is worked out (the
/// reference itself when it is a live actor, else the actor of the owning
/// form of type 0x2a found by `ProcessLists::GetActorRefInHigh`) and the
/// entry gets its kind and flags:
/// - kind 2 and the distance: the package buys, the owner actor buys the item
///   (`SellBuysItem`), `actor` can pay for it (unless it is of object type
///   0x12) and is not creature like (otherwise it is dropped);
/// - kind 0 and kept: no live owner actor and the reference is of the kind
///   tested by actor slot `0x218`;
/// - otherwise (the reference not forced to update) kind 3 or 4 for a
///   non-actor (4 for kind 1, dropped when it is locked and `actor` holds no
///   key), 4 or 5 for an actor, with the two flag bytes written, or kind 5 /
///   dropped by the creature-like and force-update tests.
pub fn high_process_evaluate_order_acquire_list(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
) {
    let process = this.addr();
    let mut node = process + PROCESS_TEMP_OBJECT_LIST;
    let package = e
        .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
        .u32();
    let package_buys = package != 0 && fn_008e30e0(e, Ptr::new(package));
    let package_flag_400 = package != 0 && fn_008e3120(e, Ptr::new(package));
    let package_flag_200 = package != 0 && fn_008e3100(e, Ptr::new(package));
    loop {
        if node == 0 {
            break;
        }
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        if e.mem.u32(slot) == 0 {
            break;
        }
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        let entry = e.mem.u32(slot);
        let reference = e.mem.u32(entry + ACQUIRE_REFERENCE);
        let form = e.mem.u32(entry + ACQUIRE_FORM);
        let mut keep = true;
        let form_type = e.call(FORM_TYPE_BYTE, &args![form]).i32();
        if form_type != 0x2a && form_type != 0x2b {
            acquire_evaluate_entry(
                e,
                &mut keep,
                actor,
                entry,
                [package_buys, package_flag_400, package_flag_200],
            );
        }
        if keep {
            e.call(REFERENCE_SET_TARGETED, &args![reference, 1u32]);
            e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), entry);
                e.call(LIST_ADD_ITEM, &args![process + LOW_OBJECT_LIST, slot]);
            });
        } else if entry != 0 {
            e.call(ACQUIRE_OBJECT_DELETE, &args![entry, 1u32]);
        }
        node = e.call(NODE_NEXT, &args![node]).u32();
    }
    e.call(PROCESS_EVALUATION_END_A, &args![this]);
    e.call(PROCESS_EVALUATION_END_B, &args![this]);
    e.call(LIST_CLEAR, &args![process + PROCESS_TEMP_OBJECT_LIST]);
}

/// Whether `reference_actor` (a live actor, or 0) is being forced to update
/// (`MiddleHighProcess::GetForceNextUpdate`, only asked for actors).
fn acquire_forced(e: &mut Engine, reference_actor: u32) -> bool {
    reference_actor != 0
        && e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![reference_actor])
            .bool()
}

/// The part of `EvaluateOrderAcquireList` that handles one entry whose form
/// is not of type 0x2a or 0x2b; `keep` is the entry's verdict.
fn acquire_evaluate_entry(
    e: &mut Engine,
    keep: &mut bool,
    actor: Ptr,
    entry: u32,
    package_flags: [bool; 3],
) {
    let [package_buys, package_flag_400, package_flag_200] = package_flags;
    let a = actor.addr();
    let reference = e.mem.u32(entry + ACQUIRE_REFERENCE);
    let form = e.mem.u32(entry + ACQUIRE_FORM);
    let owner = e.call(REFERENCE_GET_OWNER, &args![reference]).u32();
    let unowned_or_mine = owner == 0
        || e.call(REFERENCE_IS_AN_OWNER, &args![reference, actor, 1u32])
            .bool();
    if unowned_or_mine && !e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
        let distance = acquire_distance_word(e, actor, reference);
        e.mem.set_u32(entry + ACQUIRE_DISTANCE, distance);
        return;
    }
    let mut buy_flag = false;
    let mut sell_flag = false;
    let mut owner_form = 0u32;
    let mut owner_actor = 0u32;
    let mut priced = false;
    let mut handled = false;
    let reference_actor = if e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
        reference
    } else {
        0
    };
    if acquire_forced(e, reference_actor) {
        *keep = false;
    }
    if e.vcall(reference, ACTOR_SLOT_0X218, &args![]).bool()
        && !e.vcall(reference, ACTOR_SLOT_0X22C, &args![0u32]).bool()
    {
        owner_actor = reference;
    } else {
        let owner = e.call(REFERENCE_GET_OWNER, &args![reference]).u32();
        if owner != 0 && e.call(FORM_TYPE_BYTE, &args![owner]).i32() == 0x2a {
            owner_form = owner;
        }
        if owner_form != 0 {
            owner_actor = e
                .call(
                    PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH,
                    &args![PROCESS_LISTS_INSTANCE, owner_form, 0u32],
                )
                .u32();
        }
    }
    if owner_actor != 0 && !e.vcall(owner_actor, ACTOR_SLOT_0X22C, &args![0u32]).bool() {
        if package_buys {
            let ai_form = e.call(REFERENCE_AI_FORM, &args![owner_actor]).u32();
            if e.call(AI_FORM_SELL_BUYS_ITEM, &args![ai_form + 0x90, form])
                .bool()
            {
                let mut affordable = true;
                if e.call(PACKAGE_OBJECT_TYPE_FROM_FORM, &args![form]).i32() != 0x12 {
                    let gold = e.call(ACTOR_GOLD_AMOUNT, &args![actor]).i32();
                    let entry_form = e.mem.u32(entry + ACQUIRE_FORM);
                    let value = e.call(FORM_VALUE, &args![entry_form]).i32();
                    if gold < value {
                        affordable = false;
                    }
                }
                if affordable {
                    if !e.call(ACTOR_CREATURE_LIKE, &args![actor]).bool() {
                        e.mem.set_u32(entry + ACQUIRE_KIND, 2);
                        let distance = acquire_distance_word(e, actor, reference);
                        e.mem.set_u32(entry + ACQUIRE_DISTANCE, distance);
                        priced = true;
                    } else {
                        *keep = false;
                    }
                }
            }
        }
    } else if e.vcall(reference, ACTOR_SLOT_0X218, &args![]).bool() {
        *keep = true;
        e.mem.set_u32(entry + ACQUIRE_KIND, 0);
        handled = true;
    }
    if priced || handled || acquire_forced(e, reference_actor) {
        return;
    }
    if !e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
        if package_flag_400 {
            buy_flag = true;
        }
    } else if e.vcall(reference, ACTOR_SLOT_0X22C, &args![0u32]).bool() {
        *keep = true;
    } else if e.vcall(reference, ACTOR_SLOT_0X218, &args![]).bool() {
        if package_flag_400 {
            buy_flag = true;
        }
        if package_flag_200 {
            sell_flag = true;
        }
    }
    if (!buy_flag && !sell_flag) || acquire_forced(e, reference_actor) {
        if e.vcall(a, ACTOR_SLOT_0X21C, &args![]).bool() {
            *keep = false;
            return;
        }
        let stop = !e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool()
            || e.vcall(reference, ACTOR_SLOT_0X218, &args![]).bool()
            || e.call(REFERENCE_GET_OWNER, &args![reference]).u32()
                == e.call(REFERENCE_GET_FORM, &args![actor]).u32()
            || e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![actor]).bool();
        if stop {
            *keep = false;
        } else if reference_actor != 0 && !acquire_forced(e, reference_actor) {
            e.mem.set_u32(entry + ACQUIRE_KIND, 5);
            *keep = true;
        }
        return;
    }
    if !e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
        if e.mem.u32(entry + ACQUIRE_KIND) == 1 {
            e.mem.set_u32(entry + ACQUIRE_KIND, 4);
            let has_object = e.with_stack(4, |e, out| {
                e.mem.set_u32(out.addr(), 0);
                let key = e.global::<u32>(KEY_FORM_GLOBAL);
                e.call(ACTOR_HAS_OBJECTS, &args![actor, key, 0u32, 1u32, 0u32, out])
                    .bool()
            });
            if !has_object && e.call(REFERENCE_GET_LOCK, &args![reference]).u32() != 0 {
                *keep = false;
            }
        } else {
            e.mem.set_u32(entry + ACQUIRE_KIND, 3);
        }
    } else if buy_flag {
        if !e.call(ACTOR_CREATURE_LIKE, &args![actor]).bool() {
            e.mem.set_u32(entry + ACQUIRE_KIND, 4);
        } else {
            *keep = false;
        }
    } else if sell_flag {
        if e.call(ACTOR_CREATURE_LIKE, &args![actor]).bool()
            || e.call(PROCESS_GET_FORCE_NEXT_UPDATE, &args![actor]).bool()
        {
            *keep = false;
        } else if !acquire_forced(e, reference_actor) {
            e.mem.set_u32(entry + ACQUIRE_KIND, 5);
        }
    }
    e.mem.set_u8(entry + ACQUIRE_FLAG_A, u8::from(buy_flag));
    e.mem.set_u8(entry + ACQUIRE_FLAG_B, u8::from(sell_flag));
}

/// The setting `011cdbec` (an integer) read through `0043d4d0`, which returns
/// the address of its value.
const GUARD_CALL_FOR_HELP_RADIUS_SETTING: u32 = 0x011c_dbec;
/// `0043d4d0`: the address of the value of the setting in `this`.
const SETTING_VALUE_ADDRESS: u32 = 0x0043_d4d0;
/// The `TESDataHandler` global pointer (`011c3f2c`).
const DATA_HANDLER_POINTER: u32 = 0x011c_3f2c;
/// `TESDataHandler::EnumReferencesCloseToPoint` (Xbox PDB, `0046f280`):
/// `(this, cell, point, radius, point, radius, callback, argument)`.
const ENUM_REFERENCES_CLOSE_TO_POINT: u32 = 0x0046_f280;
/// `ProcessLists` instance (`011e0e80`) and its calls: the list of actors
/// found (`009715c0`), `AddReference` (`0096d450`) and `RemoveReference`
/// (`0096d470`).
const PROCESS_LISTS_FIND_ACTORS: u32 = 0x0097_15c0;
const PROCESS_LISTS_ADD_REFERENCE: u32 = 0x0096_d450;
const PROCESS_LISTS_REMOVE_REFERENCE: u32 = 0x0096_d470;
/// `MobileObject::GetCurrentProcessType` (Xbox PDB, `00931850`).
const MOBILE_OBJECT_PROCESS_TYPE: u32 = 0x0093_1850;
/// `Calendar::GetHour` (Xbox PDB, `00867da0`) on the calendar at `011de7b8`.
const CALENDAR_GET_HOUR: u32 = 0x0086_7da0;
const CALENDAR_INSTANCE: u32 = 0x011d_e7b8;
/// Sets a float on a process (`00693d50`, one argument).
const PROCESS_SET_TIME_FLOAT: u32 = 0x0069_3d50;
/// A `double` (1.0, `01012070`) the hour is lowered by.
const ONE_HOUR_BACK: u32 = 0x0101_2070;
// `BSSimpleList` scalar deleting destructor (`004702f0`) is [`LIST_DELETE`].

// Translated from 008e3f00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GuardCallforHelp` (Xbox PDB): `this` is not used. Collects
/// the references near `actor` (a radius of the integer setting `011cdbec`;
/// `008e3ea0` adds the ones it accepts to the list at `011e025c`), asks
/// `ProcessLists` (`009715c0`, with `argument`) for the actors among them and,
/// for each actor that is in a process, sets the float of its process (`00693d50`) to
/// the current hour minus 1.0 and moves it out of and back into the process
/// lists (`RemoveReference`, then `AddReference` with the same process type).
/// The found list is cleared and deleted and the list at `011e025c` is cleared.
pub fn high_process_guard_call_for_help(
    e: &mut Engine,
    _this: Ptr<HighProcess>,
    actor: Ptr,
    argument: u32,
) {
    let a = actor.addr();
    let radius = e
        .call(
            SETTING_VALUE_ADDRESS,
            &args![GUARD_CALL_FOR_HELP_RADIUS_SETTING],
        )
        .u32();
    let first_radius = e.mem.i32(radius) as f32;
    let first_position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
    let radius = e
        .call(
            SETTING_VALUE_ADDRESS,
            &args![GUARD_CALL_FOR_HELP_RADIUS_SETTING],
        )
        .u32();
    let second_radius = e.mem.i32(radius) as f32;
    let second_position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
    let cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
    let handler = e.global::<u32>(DATA_HANDLER_POINTER);
    e.call(
        ENUM_REFERENCES_CLOSE_TO_POINT,
        &args![
            handler,
            cell,
            second_position,
            second_radius,
            first_position,
            first_radius,
            0x008e_3ea0u32,
            actor
        ],
    );
    let found = e
        .call(
            PROCESS_LISTS_FIND_ACTORS,
            &args![PROCESS_LISTS_INSTANCE, argument, REFERENCE_LIST],
        )
        .u32();
    let mut node = found;
    loop {
        if node == 0 {
            break;
        }
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        if e.mem.u32(slot) == 0 {
            break;
        }
        let mut live_actor = 0u32;
        let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
        let reference = e.mem.u32(slot);
        if e.vcall(reference, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
            let slot = e.call(NODE_ITEM_ADDRESS, &args![node]).u32();
            live_actor = e.mem.u32(slot);
        }
        node = e.call(NODE_NEXT, &args![node]).u32();
        if live_actor != 0 && e.call(MOBILE_OBJECT_PROCESS_TYPE, &args![live_actor]).u32() != 0 {
            let hour = e.call(CALENDAR_GET_HOUR, &args![CALENDAR_INSTANCE]).f64();
            let earlier = (hour - e.global::<f64>(ONE_HOUR_BACK)) as f32;
            let process = e.call(ACTOR_PROCESS, &args![live_actor]).u32();
            e.call(PROCESS_SET_TIME_FLOAT, &args![process, earlier]);
            let process_type = e.call(MOBILE_OBJECT_PROCESS_TYPE, &args![live_actor]).u32();
            e.call(
                PROCESS_LISTS_REMOVE_REFERENCE,
                &args![PROCESS_LISTS_INSTANCE, live_actor, process_type],
            );
            let process_type = e.call(MOBILE_OBJECT_PROCESS_TYPE, &args![live_actor]).u32();
            e.call(
                PROCESS_LISTS_ADD_REFERENCE,
                &args![
                    PROCESS_LISTS_INSTANCE,
                    live_actor,
                    process_type,
                    0u32,
                    0u32,
                    0u32
                ],
            );
        }
    }
    if found != 0 {
        e.call(LIST_CLEAR, &args![found]);
        e.call(LIST_DELETE, &args![found, 1u32]);
    }
    e.call(LIST_CLEAR, &args![REFERENCE_LIST]);
}

/// `MiddleHighProcess::fPursueTimer` (Xbox PDB, f32 +0xd8) and
/// `lastSeenPostion` (`NiPoint3`, +0xfc).
const PROCESS_PURSUE_TIMER: u32 = 0xd8;
const PROCESS_LAST_SEEN_POSITION: u32 = 0xfc;
/// `Actor::AddFollower` (Xbox PDB, `008bc790`), called on the player.
const ACTOR_ADD_FOLLOWER: u32 = 0x008b_c790;
/// `009549a0`: whether the player (`this`) takes `actor` as a follower.
const PLAYER_CAN_TAKE_FOLLOWER: u32 = 0x0095_49a0;
/// `004030b0` (the engine map calls it `SchedulerBase::GetIScheduler`, a
/// folded getter): the word at +0xc0 of `this`; `ProcessFollow` applies it to
/// the result of the actor's slot `0x428` to get the follow target.
const FOLLOW_TARGET_OF_SLOT: u32 = 0x0040_30b0;
/// Slots of `Actor` (PC byte offsets): `0x428` (the creature's follow
/// package or object), `0x1d0` (the 3D node), `0x2cc` (whether a position can
/// be seen, `(position, float, 0, 0)`), `0x434` (takes the actor to run from
/// or stand by), `0x390`, `0x45c` and `0x458` (the run and walk speed floats).
const ACTOR_SLOT_0X428: u32 = 0x428;
const ACTOR_SLOT_NODE_FOR_FOLLOW: u32 = 0x1d0;
const ACTOR_SLOT_CAN_SEE_POSITION: u32 = 0x2cc;
const ACTOR_SLOT_0X434: u32 = 0x434;
const ACTOR_SLOT_0X390: u32 = 0x390;
const ACTOR_SLOT_RUN_SPEED: u32 = 0x45c;
const ACTOR_SLOT_WALK_SPEED: u32 = 0x458;
/// Slots of the process: `0x504` `(target, 0)`, `0x28c` (an object whose
/// `+0x3f0` word `008e2680` sets), `0x30c` (a flag) and `0x2a4`
/// (`ProcessGreet`).
const SLOT_FOLLOW_OBJECT_FOR: u32 = 0x504;
const SLOT_GET_CHARACTER_CONTROLLER: u32 = 0x28c;
const SLOT_ASK_GREETING: u32 = 0x30c;
const SLOT_GREET: u32 = 0x2a4;
/// `005a2030` and `005a3740` read the bytes +0x14d and +0x14c of `this` (an
/// actor's swimming related flags).
const ACTOR_FLAG_BYTE_14D: u32 = 0x005a_2030;
const ACTOR_FLAG_BYTE_14C: u32 = 0x005a_3740;
/// Swimming tests of the actor: `0087f350`, `0087f390`, `Actor::IsAllowedToSwim`
/// (`00884b50`) and `Actor::SwimsOnly` (`0087f570`).
const ACTOR_SWIM_TEST_A: u32 = 0x0087_f350;
const ACTOR_SWIM_TEST_B: u32 = 0x0087_f390;
const ACTOR_IS_ALLOWED_TO_SWIM: u32 = 0x0088_4b50;
const ACTOR_SWIMS_ONLY: u32 = 0x0087_f570;
/// `Actor::IsVisible` (Xbox PDB, `008b0190`): `(this, mask)`.
const ACTOR_IS_VISIBLE: u32 = 0x008b_0190;
/// `00430830`: the address `this + 0x24`.
const REFERENCE_DATA_ADDRESS: u32 = 0x0043_0830;
/// `008ad1c0` (`Actor`): puts the actor next to a position: `(this, position,
/// float, cell, world space, 1, 1)`.
const ACTOR_MOVE_TO_POSITION: u32 = 0x008a_d1c0;
/// `008840f0` and `008840d0`: bits 0x200 and 1 of the flags at +0x1c of `this`
/// (a package).
const PACKAGE_FLAG_200: u32 = 0x0088_40f0;
const PACKAGE_FLAG_1: u32 = 0x0088_40d0;
/// `00546ca0` on a cell: `(this, actor)`.
const CELL_ALLOWS_ACTOR: u32 = 0x0054_6ca0;
/// `Actor::LineOfSight` (Xbox PDB, `0088b880`): `(this, 0, target, 1, 0, 0)`.
const ACTOR_LINE_OF_SIGHT: u32 = 0x0088_b880;
/// `004938e0`: the actor's move mode has one of the bits 0xf.
const ACTOR_MOVE_MODE_HAS_SPEED: u32 = 0x0049_38e0;
/// `Actor::IsWaitingOnPath` (Xbox PDB, `008b3bf0`), `Actor::IsRotating`
/// (`008b3c30`) and `Actor::ForceStopMoving` (`008b3ad0`).
const ACTOR_IS_WAITING_ON_PATH: u32 = 0x008b_3bf0;
const ACTOR_IS_ROTATING: u32 = 0x008b_3c30;
const ACTOR_FORCE_STOP_MOVING: u32 = 0x008b_3ad0;
/// `NiPoint3` length (`00457990`, `ST0`) and the horizontal length
/// (`00595c80`, `ST0`).
const POINT_LENGTH: u32 = 0x0045_7990;
const POINT_HORIZONTAL_LENGTH: u32 = 0x0059_5c80;
/// `ActorMover` calls: `IsPathing` (`009dcdb0`), `GetDistanceRemaining`
/// (`009dcc50`, `ST0`), `GetCurrentPercent` (`009dcbf0`, `ST0`) and
/// `MoveActorToLocation` (`009df010`).
const MOVER_IS_PATHING: u32 = 0x009d_cdb0;
const MOVER_DISTANCE_REMAINING: u32 = 0x009d_cc50;
const MOVER_CURRENT_PERCENT: u32 = 0x009d_cbf0;
const MOVER_MOVE_TO_LOCATION: u32 = 0x009d_f010;
/// Float helpers: `_finite` (`00ec7595`, a `double` on the stack), the maximum
/// (`00404010`) and the minimum (`0040ebd0`) of two floats (cdecl, `ST0`) and
/// the absolute value (`00408860`).
const FINITE: u32 = 0x00ec_7595;
const FLOAT_MAX: u32 = 0x0040_4010;
const FLOAT_MIN: u32 = 0x0040_ebd0;
const FLOAT_ABSOLUTE: u32 = 0x0040_8860;
/// `AiFormulas` calls: `GetFollowRadiusFudgeForMultipleFollowers`
/// (`00643ff0`, cdecl `(actor, follow actor)`, `ST0`) and `00643f20`,
/// `00643f90`, `00643fa0` (cdecl, one float, `ST0`).
const FOLLOW_RADIUS_FUDGE: u32 = 0x0064_3ff0;
const FOLLOW_DISTANCE_A: u32 = 0x0064_3f20;
const FOLLOW_DISTANCE_B: u32 = 0x0064_3f90;
const FOLLOW_DISTANCE_C: u32 = 0x0064_3fa0;
/// `TESPackage::GetPackageRadiusActorToRefTarget` (Xbox PDB, `006787e0`):
/// `(this, actor, flag)`, `ST0`.
const PACKAGE_RADIUS_TO_REF_TARGET: u32 = 0x0067_87e0;
/// `PackageLocation::GetLocType` (Xbox PDB, `00678ca0`): the byte at +0.
const PACKAGE_LOCATION_TYPE: u32 = 0x0067_8ca0;
/// `008e26a0`: `max(0, float at +0xe0)` of `this`.
const NON_NEGATIVE_FLOAT_AT_0XE0: u32 = 0x008e_26a0;
/// `TESObjectREFR::GetDeltaForAngle` (Xbox PDB, `00572500`): `(this, out,
/// angle)`; `NiPoint3` scale (`00439180`) and add (`0063c8a0`) in place.
const REFERENCE_DELTA_FOR_ANGLE: u32 = 0x0057_2500;
const POINT_SCALE_IN_PLACE: u32 = 0x0043_9180;
const POINT_ADD_IN_PLACE: u32 = 0x0063_c8a0;
/// `Actor::GetHeight` (Xbox PDB, `008853a0`, `ST0`).
const ACTOR_GET_HEIGHT: u32 = 0x0088_53a0;
/// Settings: `011cde98` (an integer), `011cd950` and `011cd5d4` (floats).
const FOLLOW_RADIUS_SETTING: u32 = 0x011c_de98;
const FOLLOW_MAX_DISTANCE_SETTING: u32 = 0x011c_d950;
const FOLLOW_STUCK_DISTANCE_SETTING: u32 = 0x011c_d5d4;
/// Globals: floats `0101b2d0`, `0101b268`, `01016410`; doubles `01012060`
/// (0.0), `01012070` (1.0, [`ONE_HOUR_BACK`]), `0101de30`, `01011588`,
/// `0101db88`.
const FOLLOW_ANGLE_SCALE: u32 = 0x0101_b2d0;
const FOLLOW_TARGET_EXTRA: u32 = 0x0101_b268;
const FOLLOW_CRUMB_DISTANCE: u32 = 0x0101_6410;
const ZERO_DOUBLE: u32 = 0x0101_2060;
const FOLLOW_STOP_FACTOR: u32 = 0x0101_de30;
const FOLLOW_MOVER_PERCENT_LIMIT: u32 = 0x0101_1588;
const FOLLOW_PURSUE_LIMIT: u32 = 0x0101_db88;
/// Breadcrumb calls: `ExtraDataList::GetOrCreateExtraFollowerSwimBreadcrumbs`
/// (`0042f420`), the list's first element (`00437dd0`), pop (`00437df0`) and
/// add (`00437e50`: `(this, mode, x, y, z, nav mesh)`).
const GET_BREADCRUMBS: u32 = 0x0042_f420;
const BREADCRUMB_FIRST: u32 = 0x0043_7dd0;
const BREADCRUMB_POP: u32 = 0x0043_7df0;
const BREADCRUMB_ADD: u32 = 0x0043_7e50;
/// `PathingLocation` calls: `ResolveNavMeshInfo` (`006dd6f0`, one argument),
/// `ResolveToClosestNavmeshAndTriangle` (`006ddc00`), `GetBestNavMeshInfo`
/// (`006dd540`) and the coordinates copy (`005c3420`: `(this, out)`).
const PATHING_LOCATION_RESOLVE: u32 = 0x006d_d6f0;
const PATHING_LOCATION_RESOLVE_CLOSEST: u32 = 0x006d_dc00;
const PATHING_LOCATION_BEST_NAV_MESH: u32 = 0x006d_d540;
const PATHING_LOCATION_COORDINATES: u32 = 0x005c_3420;
/// `MobileObject::GetCharController` (Xbox PDB, `009306d0`) and `005c0880`.
const MOBILE_CHARACTER_CONTROLLER: u32 = 0x0093_06d0;
const CONTROLLER_FLAG: u32 = 0x005c_0880;
/// Nav mesh helpers: `TES::GetNavMeshInfoMap` (`0045af00`, on the `TES`
/// pointer at `011dea10`), `00693d70`, `0069ad00`, the pointer's constructor
/// (`0042fb00`), destructor (`0042fa40`), test (`00458b50`), the pointed
/// object's space (`0059bb30`) and `NavMesh::GetParentSpace` (`0068ef70`).
const TES_POINTER: u32 = 0x011d_ea10;
const NAV_MESH_INFO_MAP: u32 = 0x0045_af00;
const NAV_MESH_LOOKUP: u32 = 0x0069_3d70;
const NAV_MESH_FILL_POINTER: u32 = 0x0069_ad00;
const NAV_POINTER_CONSTRUCTOR: u32 = 0x0042_fb00;
const NAV_POINTER_DESTRUCTOR: u32 = 0x0042_fa40;
const NAV_POINTER_IS_SET: u32 = 0x0045_8b50;
const NAV_POINTER_SPACE: u32 = 0x0059_bb30;
const NAV_MESH_PARENT_SPACE: u32 = 0x0068_ef70;
/// `Actor::BuildRequest_ov2` (Xbox PDB, `008b38d0`): `(this, request,
/// position, space, float, 0)`.
const ACTOR_BUILD_REQUEST_OV2: u32 = 0x008b_38d0;
/// `Actor::ModifyMoveModeForFollow` (`008858c0`), `Actor::AdjustSpeedForFollowing`
/// (`008855f0`), `Actor::SetRunSpeed` (`00884ff0`) and `Actor::SetWalkSpeed`
/// (`00884fc0`).
const ACTOR_MODIFY_MOVE_MODE_FOR_FOLLOW: u32 = 0x0088_58c0;
const ACTOR_ADJUST_SPEED_FOR_FOLLOWING: u32 = 0x0088_55f0;
const ACTOR_SET_RUN_SPEED: u32 = 0x0088_4ff0;
const ACTOR_SET_WALK_SPEED: u32 = 0x0088_4fc0;
/// `Actor::IsContinuingPackageforPC` (Xbox PDB, `008a69d0`), the actor's
/// facing target setter (`0057bd60`) and `0045cd60` (the word at +0x28).
const ACTOR_IS_CONTINUING_PACKAGE_FOR_PC: u32 = 0x008a_69d0;
const ACTOR_SET_FACING_TARGET: u32 = 0x0057_bd60;
const PROCESS_WORD_AT_0X28: u32 = 0x0045_cd60;
/// `Actor::EvaluatePackage` (Xbox PDB, `008a6ce0`): `(this, 0, 0)`.
const ACTOR_EVALUATE_PACKAGE: u32 = 0x008a_6ce0;
/// Slot `0x1c` of the actor's mover.
const MOVER_SLOT_0X1C: u32 = 0x1c;
/// `008e2680`: sets the word at +0x3f0 of `this` (the engine map names it
/// `Actor::DoesFly`).
const SET_WORD_AT_0X3F0: u32 = 0x008e_2680;
/// Slot `0x48` of the actor receives `0x80000000` before the breadcrumbs.
const ACTOR_SLOT_0X48: u32 = 0x48;

/// What the parts of `ProcessFollow` share.
struct Follow {
    process: u32,
    actor: u32,
    player: u32,
    /// Third argument: the move mode to use, -1 to work one out.
    mode: i32,
    /// Fourth argument: use the pursuit timer.
    check_stuck: bool,
    package: u32,
    target: u32,
    /// The target when it is an actor, else 0.
    follow_actor: u32,
    /// `+0x14d` flag of the follow actor (swimming).
    swim_flag: bool,
    /// The result of the actor's slot `0x428`.
    actor_slot_value: u32,
    /// The address of the 12-byte copy of the target's position.
    position: u32,
    /// Distance between the actor and the target (the mover's remaining
    /// distance lowers it).
    distance: f32,
    radius: f32,
    near_distance: f32,
    follow_distance: f32,
    stop_distance: f32,
    sight_waits: bool,
    /// The swim breadcrumbs took over the movement.
    breadcrumbs: bool,
}

// Translated from 008e0f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ProcessFollow` (Xbox PDB): the follow package. `stop` makes
/// the process add procedure 1 when there is nothing to follow; `mode` is a
/// move mode to use instead of working one out (-1: work it out) and
/// `check_stuck` turns on the pursuit timer (`fPursueTimer`, +0xd8).
///
/// The code works with a package (the actor's own, from its slot `0x428`,
/// else the running package of the process), a target reference (the
/// creature's follow target from that slot, else the process' target set by
/// `SetTargetForPackage`), the follow distances from the package radius and
/// the settings, and the actor's mover:
/// 1. early exits: package flag `0x10000`, state 4 or 9, no target or a
///    target with the flag `0x800`; a target whose reference vanished ends the
///    package (`SetNeverRun`); a dead target ends an interrupt package or
///    clears the process' target;
/// 2. the target's position is kept (`lastSeenPostion`, +0xfc); an actor
///    target that swims is checked against the follower's own ability to
///    swim, and a target nobody can see is simply stood next to (`008ad1c0`);
/// 3. for a created package the line of sight to the target is checked;
/// 4. the radii come from the package radius (or a setting), the fudge for
///    several followers and `00643f20`/`00643f90`/`00643fa0`;
/// 5. a package that wants the actor at its location (slot `0x13c`) ends the
///    follow; for a player target the swim breadcrumbs leave a trail
///    ([`follow_breadcrumbs`]);
/// 6. the chase: when the target is far enough (or cannot be seen) the actor
///    paths to it (`SetPathfindingGoal_ov2`), with a pursuit timer that gives
///    up after a while;
/// 7. the move mode (`CalculateMoveMode`, `SetActorsAnimation`) and the run
///    and walk speeds (`ModifyMoveModeForFollow`, `AdjustSpeedForFollowing`);
/// 8. when close enough the actor stops, faces the target and, if the package
///    continues for the player, starts the greeting (`ProcessGreet`).
///
/// The SEH frame is not translated.
pub fn high_process_process_follow(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    stop: u8,
    mode: i32,
    check_stuck: u8,
) {
    e.with_stack(12, |e, position| {
        follow_run(
            e,
            this,
            actor,
            stop != 0,
            mode,
            check_stuck != 0,
            position.addr(),
        );
    });
}

fn follow_run(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    actor: Ptr,
    stop: bool,
    mode: i32,
    check_stuck: bool,
    position: u32,
) {
    let process = this.addr();
    let a = actor.addr();
    let player = e.global::<u32>(PLAYER_POINTER);
    let mut package = e.vcall(a, ACTOR_SLOT_0X428, &args![]).u32();
    if package == 0 {
        package = e
            .vcall(process, SLOT_GET_PACKAGE_THAT_IS_RUNNING, &args![])
            .u32();
    }
    if package != 0 && e.call(PACKAGE_FLAG_10000, &args![package]).bool() {
        return;
    }
    if e.call(PLAYER_CAN_TAKE_FOLLOWER, &args![player, actor])
        .bool()
    {
        e.call(ACTOR_ADD_FOLLOWER, &args![player, actor]);
    }
    if e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32() == 4
        || e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32() == 9
    {
        e.vcall(a, ACTOR_SLOT_0X418, &args![]);
        return;
    }
    // The target of the follow.
    let target;
    if e.call(ACTOR_CREATURE_LIKE, &args![actor]).bool()
        && e.vcall(a, ACTOR_SLOT_0X428, &args![]).u32() != 0
    {
        let slot_value = e.vcall(a, ACTOR_SLOT_0X428, &args![]).u32();
        target = e.call(FOLLOW_TARGET_OF_SLOT, &args![slot_value]).u32();
    } else {
        if e.mem.u32(process + LOW_TARGET) == 0 {
            e.vcall(process, SLOT_SET_TARGET_FOR_PACKAGE, &args![actor]);
        }
        target = e.mem.u32(process + LOW_TARGET);
    }
    if target == 0 || e.call(REFERENCE_FLAG_800, &args![target]).bool() {
        if stop {
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
        }
        return;
    }
    // A target whose reference is gone ends the package.
    let mut gone = false;
    if e.call(REFERENCE_FLAG_20, &args![target]).bool() {
        let extra = e.call(REFERENCE_EXTRA_DATA, &args![target]).u32();
        if e.call(EXTRA_DATA_REFERENCE_POINTER, &args![extra]).u32() == 0 {
            gone = true;
        }
    }
    if !gone {
        gone = e.call(REFERENCE_FLAG_800, &args![target]).bool();
    }
    if gone {
        if e.call(REFERENCE_FLAG_20, &args![target]).bool() {
            e.call(PACKAGE_SET_NEVER_RUN, &args![package, target, 1u32]);
        }
        if stop {
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
        }
        return;
    }
    if e.vcall(target, ACTOR_SLOT_0X22C, &args![1u32]).bool() {
        if e.call(PACKAGE_GET_IS_CREATED, &args![package]).bool() {
            e.call(ACTOR_END_INTERRUPT_PACKAGE, &args![actor, 0u32]);
            e.call(ACTOR_EVALUATE_PACKAGE, &args![actor, 0u32, 0u32]);
        } else {
            let package_target = e.call(PACKAGE_TARGET_WORD, &args![package]).u32();
            let mut kind = e
                .call(PACKAGE_TARGET_GET_TYPE, &args![package_target])
                .i32();
            if kind != 0 {
                let package_target = e.call(PACKAGE_TARGET_WORD, &args![package]).u32();
                kind = e
                    .call(PACKAGE_TARGET_GET_TYPE, &args![package_target])
                    .i32();
            }
            if kind == 0 || kind == 3 {
                let reference = e.mem.u32(process + LOW_TARGET);
                e.call(PACKAGE_SET_NEVER_RUN, &args![package, reference, 1u32]);
            } else {
                e.vcall(process, SLOT_SET_TARGET, &args![0u32]);
                e.vcall(process, SLOT_SET_PROCEDURE_INDEX_RUNNING, &args![0u32]);
            }
        }
        return;
    }
    // The target's position, copied and kept in the process.
    let live_position = e.vcall(target, ACTOR_SLOT_POSITION, &args![]).u32();
    for word in 0..3 {
        let value = e.mem.u32(live_position + word * 4);
        e.mem.set_u32(position + word * 4, value);
    }
    if target != 0 {
        for word in 0..3 {
            let value = e.mem.u32(position + word * 4);
            e.mem
                .set_u32(process + PROCESS_LAST_SEEN_POSITION + word * 4, value);
        }
    }
    let mut swim_flag = false;
    let mut follow_actor = 0u32;
    if e.vcall(target, ACTOR_SLOT_IS_ACTOR, &args![]).bool() {
        follow_actor = target;
    }
    if follow_actor != 0 {
        swim_flag = e.call(ACTOR_FLAG_BYTE_14D, &args![follow_actor]).bool();
        if swim_flag {
            if !e.call(ACTOR_SWIM_TEST_A, &args![actor]).bool()
                || !e.call(ACTOR_IS_ALLOWED_TO_SWIM, &args![actor]).bool()
            {
                e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
                return;
            }
        } else if e.call(ACTOR_SWIMS_ONLY, &args![actor]).bool() {
            e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            return;
        }
        if e.call(MOBILE_OBJECT_PROCESS_TYPE, &args![follow_actor])
            .i32()
            == 3
            && !e.vcall(follow_actor, ACTOR_SLOT_0X22C, &args![0u32]).bool()
            && !e.call(REFERENCE_FLAG_800, &args![follow_actor]).bool()
            && follow_actor != a
            && follow_actor != player
            && (e
                .vcall(follow_actor, ACTOR_SLOT_NODE_FOR_FOLLOW, &args![])
                .u32()
                == 0
                || !e.call(ACTOR_IS_VISIBLE, &args![follow_actor, 7u32]).bool())
            && (e.vcall(a, ACTOR_SLOT_NODE_FOR_FOLLOW, &args![]).u32() == 0
                || !e.call(ACTOR_IS_VISIBLE, &args![actor, 7u32]).bool())
        {
            // Nobody can see either of them: the actor is put next to the
            // target. `GetWorldSpace` and `008d6f30` take no arguments; the
            // two words pushed before them stay and are the last two
            // arguments of the move.
            let world_space = e
                .call(REFERENCE_GET_WORLD_SPACE, &args![follow_actor])
                .u32();
            let cell = e.call(REFERENCE_PATHING_CELL, &args![follow_actor]).u32();
            let data = e.call(REFERENCE_DATA_ADDRESS, &args![follow_actor]).u32();
            let spread = e.mem.u32(data + 8);
            let follow_position = e.vcall(follow_actor, ACTOR_SLOT_POSITION, &args![]).u32();
            e.call(
                ACTOR_MOVE_TO_POSITION,
                &args![
                    actor,
                    follow_position,
                    spread,
                    cell,
                    world_space,
                    1u32,
                    1u32
                ],
            );
            return;
        }
    }
    // The line of sight of a created package.
    let mut sight_waits = false;
    if e.call(PACKAGE_GET_IS_CREATED, &args![package]).bool() {
        let current = e.vcall(process, SLOT_GET_CURRENT_PACKAGE, &args![]).u32();
        if current != 0
            && e.call(PACKAGE_FLAG_200, &args![current]).bool()
            && e.call(PACKAGE_FLAG_1, &args![current]).bool()
            && e.call(REFERENCE_PATHING_CELL, &args![actor]).u32() != 0
        {
            let cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
            if e.call(CELL_ALLOWS_ACTOR, &args![cell, actor]).bool() {
                e.vcall(process, SLOT_SET_EXTRA_HEAD_TRACK, &args![player]);
                if e.call(
                    ACTOR_LINE_OF_SIGHT,
                    &args![actor, 0u32, target, 1u32, 0u32, 0u32],
                )
                .bool()
                    || e.vcall(process, SLOT_FOLLOW_OBJECT_FOR, &args![target, 0u32])
                        .u32()
                        == 0
                {
                    if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
                        e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
                    }
                    return;
                }
                if !e.call(ACTOR_MOVE_MODE_HAS_SPEED, &args![actor]).bool()
                    && !e.call(ACTOR_IS_WAITING_ON_PATH, &args![actor]).bool()
                {
                    sight_waits = true;
                } else if e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32() == 9
                    || e.vcall(a, ACTOR_SLOT_STATE, &args![]).i32() == 4
                {
                    e.vcall(a, ACTOR_SLOT_0X418, &args![]);
                    return;
                }
            }
        }
    }
    // Distances and radii.
    let actor_slot_value = e.vcall(a, ACTOR_SLOT_0X428, &args![]).u32();
    let mut distance = e.with_stack(12, |e, difference| {
        let my_position = e.vcall(a, ACTOR_SLOT_POSITION, &args![]).u32();
        let out = e
            .call(POINT_DIFFERENCE, &args![my_position, difference, position])
            .u32();
        e.call(POINT_LENGTH, &args![out]).f32()
    });
    let mover = e.mem.u32(a + ACTOR_MOVER);
    if mover != 0 && e.call(MOVER_IS_PATHING, &args![mover]).bool() {
        let remaining = e.call(MOVER_DISTANCE_REMAINING, &args![mover]).f32();
        if e.call(FINITE, &args![f64::from(remaining)]).u32() != 0 {
            distance = e.call(FLOAT_MAX, &args![distance, remaining]).f32();
        }
    }
    let follow_actor_busy = follow_actor != 0
        && e.call(ACTOR_MOVE_MODE_HAS_SPEED, &args![follow_actor])
            .bool();
    let mut radius = 0.0f32;
    if target == player || e.call(PACKAGE_TARGET_WORD, &args![package]).u32() != 0 {
        radius = e
            .call(PACKAGE_RADIUS_TO_REF_TARGET, &args![package, actor, 0u32])
            .f32();
    }
    // `FCOMP` against 1.0: only an ordered "less than" counts.
    if f64::from(radius) < e.global::<f64>(ONE_HOUR_BACK) {
        let setting = e
            .call(SETTING_VALUE_ADDRESS, &args![FOLLOW_RADIUS_SETTING])
            .u32();
        radius = e.mem.i32(setting) as f32;
    }
    let fudge = e
        .call(FOLLOW_RADIUS_FUDGE, &args![actor, follow_actor])
        .f64();
    radius = (fudge + f64::from(radius)) as f32;
    let near_distance = e.call(FOLLOW_DISTANCE_A, &args![radius]).f32();
    let follow_distance = if follow_actor_busy {
        near_distance
    } else {
        e.call(FOLLOW_DISTANCE_B, &args![radius]).f32()
    };
    let stop_distance = (f64::from(follow_distance) * e.global::<f64>(FOLLOW_STOP_FACTOR)) as f32;
    let mut f = Follow {
        process,
        actor: a,
        player,
        mode,
        check_stuck,
        package,
        target,
        follow_actor,
        swim_flag,
        actor_slot_value,
        position,
        distance,
        radius,
        near_distance,
        follow_distance,
        stop_distance,
        sight_waits,
        breadcrumbs: false,
    };
    // A package that wants the actor at its location ends the follow.
    let mut wants_location = target == 0;
    if target != 0 && e.call(PACKAGE_TYPE, &args![package]).i32() == 1 {
        let location = e.call(PACKAGE_LOCATION_WORD, &args![package]).u32();
        if location != 0 {
            let location = e.call(PACKAGE_LOCATION_WORD, &args![package]).u32();
            let mut asks = e.call(PACKAGE_LOCATION_TYPE, &args![location]).i32() < 2;
            if !asks {
                let location = e.call(PACKAGE_LOCATION_WORD, &args![package]).u32();
                asks = e.call(PACKAGE_LOCATION_TYPE, &args![location]).i32() == 6;
            }
            if asks {
                let unlimited = e.global::<f32>(NO_LIMIT_FLOAT);
                if e.vcall(
                    package,
                    PACKAGE_SLOT_0X13C,
                    &args![actor, 0u32, unlimited, 0u32],
                )
                .bool()
                {
                    wants_location = true;
                }
            }
        }
    }
    if wants_location {
        if stop {
            e.vcall(
                process,
                SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                &args![actor, 1u32],
            );
            if e.call(PACKAGE_GET_IS_CREATED, &args![package]).bool() {
                if e.vcall(process, SLOT_GET_RUN_ONCE_PACKAGE, &args![]).u32() == 0 {
                    e.vcall(process, SLOT_CLEAR_CURRENT_PACKAGE, &args![]);
                } else {
                    e.vcall(process, SLOT_CLEAR_RUN_ONCE_PACKAGE, &args![]);
                }
                e.vcall(process, SLOT_CHECK_FOR_NEW_PACKAGE, &args![actor, 0u32]);
                return;
            }
            if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
                e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            }
        }
    } else {
        if !follow_chase(e, &mut f) {
            return;
        }
        if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
            follow_choose_animation(e, &f);
        }
    }
    follow_finish(e, &f);
}

/// `ProcessFollow`, `008e188e` to `008e21c3`: the chase. Returns false when
/// the function ends there.
fn follow_chase(e: &mut Engine, f: &mut Follow) -> bool {
    let actor = f.actor;
    let process = f.process;
    let last_seen = process + PROCESS_LAST_SEEN_POSITION;
    // How far the target is from where it was last seen.
    let last_seen_gap = e.with_stack(12, |e, difference| {
        let out = e
            .call(POINT_DIFFERENCE, &args![last_seen, difference, f.position])
            .u32();
        e.call(POINT_LENGTH, &args![out]).f32()
    });
    if f.actor_slot_value != 0 {
        let angle = e
            .call(NON_NEGATIVE_FLOAT_AT_0XE0, &args![f.actor_slot_value])
            .f32();
        // `FCOMP` against 0.0: continues only above it.
        if f64::from(angle) > e.global::<f64>(ZERO_DOUBLE) {
            e.with_stack(12, |e, delta| {
                e.call(REFERENCE_DELTA_FOR_ANGLE, &args![f.target, delta, angle]);
                let scale = e.global::<f32>(FOLLOW_ANGLE_SCALE);
                e.call(POINT_SCALE_IN_PLACE, &args![delta, scale]);
                e.call(POINT_ADD_IN_PLACE, &args![f.position, delta]);
            });
        }
    }
    if f.follow_actor != 0
        && (f.swim_flag || e.call(ACTOR_FLAG_BYTE_14C, &args![f.follow_actor]).bool())
    {
        let height = e.call(ACTOR_GET_HEIGHT, &args![f.follow_actor]).f64();
        let z = f64::from(e.mem.f32(f.position + 8));
        let raised = (height + z) as f32;
        let water = e.call(REFERENCE_WATER_HEIGHT, &args![f.follow_actor]).f32();
        let level = e.call(FLOAT_MIN, &args![raised, water]).f32();
        e.mem.set_f32(f.position + 8, level);
    }
    let setting = e
        .call(SETTING_VALUE, &args![FOLLOW_MAX_DISTANCE_SETTING])
        .u32();
    if f.stop_distance > e.mem.f32(setting) {
        let setting = e
            .call(SETTING_VALUE, &args![FOLLOW_MAX_DISTANCE_SETTING])
            .u32();
        f.stop_distance = e.mem.f32(setting);
    }
    let extra = if f.actor_slot_value != 0 {
        0.0f32
    } else {
        e.global::<f32>(FOLLOW_TARGET_EXTRA)
    };
    let wait_distance = e.call(FOLLOW_DISTANCE_C, &args![f.radius]).f32();
    if f.follow_actor != 0 && f.follow_actor == f.player {
        f.breadcrumbs = follow_breadcrumbs(e, f);
    }
    if f.breadcrumbs {
        return true;
    }
    // Nothing to do while the actor is standing and the target is near.
    if !e.call(ACTOR_MOVE_MODE_HAS_SPEED, &args![actor]).bool() && !(f.distance > wait_distance) {
        return true;
    }
    // Goes to the target when it is far or moved off, or when the position
    // cannot be seen; a moving actor is left alone until its mover is far on.
    if !f.sight_waits && !(f.stop_distance < last_seen_gap) {
        if f.follow_actor == 0
            || !e
                .call(ACTOR_MOVE_MODE_HAS_SPEED, &args![f.follow_actor])
                .bool()
        {
            let seen_from = (f64::from(f.follow_distance) + f64::from(extra)) as f32;
            if e.vcall(
                actor,
                ACTOR_SLOT_CAN_SEE_POSITION,
                &args![f.position, seen_from, 0u32, 0u32],
            )
            .bool()
            {
                return true;
            }
        }
        if !e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
            if e.call(ACTOR_IS_WAITING_ON_PATH, &args![actor]).bool() {
                return true;
            }
            let mover = e.mem.u32(actor + ACTOR_MOVER);
            let percent = e.call(MOVER_CURRENT_PERCENT, &args![mover]).f64();
            // `FCOMP`: at most the limit (or unordered) leaves it.
            if !(percent > e.global::<f64>(FOLLOW_MOVER_PERCENT_LIMIT)) {
                return true;
            }
        }
    }
    if f.check_stuck {
        let setting = e
            .call(SETTING_VALUE, &args![FOLLOW_STUCK_DISTANCE_SETTING])
            .u32();
        if e.mem.f32(setting) < f.distance {
            if !e
                .call(
                    ACTOR_LINE_OF_SIGHT,
                    &args![actor, 0u32, f.target, 1u32, 0u32, 0u32],
                )
                .bool()
            {
                let timer = f64::from(e.mem.f32(process + PROCESS_PURSUE_TIMER))
                    + e.global::<f64>(ONE_HOUR_BACK);
                e.mem.set_f32(process + PROCESS_PURSUE_TIMER, timer as f32);
                let timer = f64::from(e.mem.f32(process + PROCESS_PURSUE_TIMER));
                // `FCOMP`: "less than" skips the give-up branch.
                if !(timer < e.global::<f64>(FOLLOW_PURSUE_LIMIT)) {
                    if e.call(ACTOR_CREATURE_LIKE, &args![actor]).bool() {
                        let mut runner = 0u32;
                        if f.target != 0 && e.vcall(f.target, ACTOR_SLOT_IS_ACTOR, &args![]).bool()
                        {
                            runner = f.target;
                        }
                        if runner != 0 {
                            e.vcall(actor, ACTOR_SLOT_0X434, &args![runner]);
                        }
                    } else {
                        e.vcall(
                            process,
                            SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING,
                            &args![actor, 2u32],
                        );
                    }
                    e.mem.set_f32(process + PROCESS_PURSUE_TIMER, 0.0);
                    return false;
                }
            } else {
                e.mem.set_f32(process + PROCESS_PURSUE_TIMER, 0.0);
            }
        } else {
            e.mem.set_f32(process + PROCESS_PURSUE_TIMER, 0.0);
        }
    }
    // `GetWorldSpace` and `008d6f30` take no arguments; the float and the 0
    // pushed before them stay and are the last two arguments of the goal.
    let world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![f.target]).u32();
    let cell = e.call(REFERENCE_PATHING_CELL, &args![f.target]).u32();
    let set = e
        .call(
            ACTOR_SET_PATHFINDING_GOAL,
            &args![
                actor,
                f.position,
                cell,
                world_space,
                f.follow_distance,
                0u32
            ],
        )
        .bool();
    if !set {
        return false;
    }
    let live_position = e.vcall(f.target, ACTOR_SLOT_POSITION, &args![]).u32();
    for word in 0..3 {
        let value = e.mem.u32(live_position + word * 4);
        e.mem.set_u32(last_seen + word * 4, value);
    }
    true
}

/// `ProcessFollow`, `008e19ad` to `008e1f56`: while following the player the
/// actor lays and follows swim breadcrumbs. Returns whether breadcrumbs are
/// being followed (the byte at `ebp-0x1e` of the game's frame).
fn follow_breadcrumbs(e: &mut Engine, f: &Follow) -> bool {
    let actor = f.actor;
    let process = f.process;
    let reach = e.global::<f32>(FOLLOW_CRUMB_DISTANCE);
    let extra_data = e.call(REFERENCE_EXTRA_DATA, &args![actor]).u32();
    let crumbs = e.call(GET_BREADCRUMBS, &args![extra_data]).u32();
    let mut following = false;
    if crumbs == 0 {
        return false;
    }
    e.vcall(actor, ACTOR_SLOT_0X48, &args![0x8000_0000u32]);
    e.with_stack(0x34, |e, location| {
        e.call(
            PATHING_LOCATION_FROM_ACTOR,
            &args![location, f.follow_actor],
        );
        let mut resolved = e
            .call(PATHING_LOCATION_RESOLVE, &args![location, 0u32])
            .bool();
        if !resolved {
            resolved = e
                .call(PATHING_LOCATION_RESOLVE_CLOSEST, &args![location])
                .bool();
        }
        if resolved {
            let mut mode = 0u32;
            if f.swim_flag || e.call(ACTOR_FLAG_BYTE_14C, &args![f.follow_actor]).bool() {
                mode = 2;
            } else {
                let controller = e
                    .call(MOBILE_CHARACTER_CONTROLLER, &args![f.follow_actor])
                    .u32();
                if controller != 0 && e.call(CONTROLLER_FLAG, &args![controller]).u32() == 0 {
                    mode = 1;
                }
            }
            let info = e
                .call(PATHING_LOCATION_BEST_NAV_MESH, &args![location])
                .u32();
            let nav_mesh = if info != 0 {
                e.call(NI_POINTER_GET, &args![info]).u32()
            } else {
                0
            };
            let coordinates = e.with_stack(12, |e, coordinates| {
                e.call(PATHING_LOCATION_COORDINATES, &args![location, coordinates]);
                [
                    e.mem.u32(coordinates.addr()),
                    e.mem.u32(coordinates.addr() + 4),
                    e.mem.u32(coordinates.addr() + 8),
                ]
            });
            let added = e
                .call(
                    BREADCRUMB_ADD,
                    &args![
                        crumbs,
                        mode,
                        coordinates[0],
                        coordinates[1],
                        coordinates[2],
                        nav_mesh
                    ],
                )
                .bool();
            if added {
                e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
            }
            if e.call(BREADCRUMB_FIRST, &args![crumbs]).u32() != 0 {
                let swimming_now = e.call(ACTOR_SWIM_TEST_B, &args![actor]).bool();
                let mut start_over = swimming_now;
                if !start_over && mode == 2 {
                    start_over = e.call(ACTOR_FLAG_BYTE_14D, &args![actor]).bool()
                        || e.call(ACTOR_FLAG_BYTE_14C, &args![actor]).bool();
                }
                if !start_over && mode == 1 {
                    let own_controller = e.call(MOBILE_CHARACTER_CONTROLLER, &args![actor]).u32();
                    if own_controller != 0 {
                        let own_controller =
                            e.call(MOBILE_CHARACTER_CONTROLLER, &args![actor]).u32();
                        if e.call(CONTROLLER_FLAG, &args![own_controller]).u32() == 0 {
                            start_over = true;
                        }
                    }
                }
                if start_over {
                    e.with_stack(0x34, |e, own_location| {
                        e.call(PATHING_LOCATION_FROM_ACTOR, &args![own_location, actor]);
                        let resolved = e
                            .call(PATHING_LOCATION_RESOLVE, &args![own_location, 0u32])
                            .bool()
                            || e.call(PATHING_LOCATION_RESOLVE_CLOSEST, &args![own_location])
                                .bool();
                        if resolved {
                            let own_info = e
                                .call(PATHING_LOCATION_BEST_NAV_MESH, &args![own_location])
                                .u32();
                            let own_mesh = if own_info != 0 {
                                e.call(NI_POINTER_GET, &args![own_info]).u32()
                            } else {
                                0
                            };
                            if own_mesh == nav_mesh {
                                e.vcall(process, SLOT_END_MOVE_MESSAGE, &args![actor]);
                                following = false;
                                while e.call(BREADCRUMB_FIRST, &args![crumbs]).u32() != 0 {
                                    e.call(BREADCRUMB_POP, &args![crumbs]);
                                }
                            }
                        }
                        e.call(PATHING_LOCATION_DESTRUCTOR, &args![own_location]);
                    });
                }
            }
        }
        let node = e.call(BREADCRUMB_FIRST, &args![crumbs]).u32();
        if node != 0 {
            following = true;
            if e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool() {
                follow_next_breadcrumb(e, f, crumbs, node, reach, &mut following);
            }
        }
        e.call(PATHING_LOCATION_DESTRUCTOR, &args![location]);
    });
    following
}

/// The part of the breadcrumb handling that runs when the actor's pathing is
/// complete: either it has reached the breadcrumb `node` (it is then moved
/// onto it and the breadcrumb dropped) or a request to it is built and set.
fn follow_next_breadcrumb(
    e: &mut Engine,
    f: &Follow,
    crumbs: u32,
    node: u32,
    reach: f32,
    following: &mut bool,
) {
    let actor = f.actor;
    let process = f.process;
    e.with_stack(8, |e, mesh_pointer| {
        e.call(NAV_POINTER_CONSTRUCTOR, &args![mesh_pointer]);
        let tes = e.global::<u32>(TES_POINTER);
        let mesh_data = e.mem.u32(node + 0xc);
        let info_map = e.call(NAV_MESH_INFO_MAP, &args![tes, mesh_data]).u32();
        let mesh = e.call(NAV_MESH_LOOKUP, &args![info_map]).u32();
        if mesh != 0 {
            e.call(NAV_MESH_FILL_POINTER, &args![mesh, mesh_pointer]);
        }
        let mut arrived_here = true;
        if e.call(NAV_POINTER_IS_SET, &args![mesh_pointer]).u32() != 0 {
            let held = e.call(NI_POINTER_GET, &args![mesh_pointer]).u32();
            let mesh_space = e.call(NAV_POINTER_SPACE, &args![held]).u32();
            let cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
            if mesh_space != cell {
                arrived_here = false;
            }
        }
        if arrived_here {
            let my_position = e.vcall(actor, ACTOR_SLOT_POSITION, &args![]).u32();
            let horizontal = e.with_stack(12, |e, difference| {
                let out = e
                    .call(POINT_DIFFERENCE, &args![my_position, difference, node])
                    .u32();
                e.call(POINT_HORIZONTAL_LENGTH, &args![out]).f64()
            });
            let squared_reach = (f64::from(reach) * f64::from(reach)) as f32;
            if f64::from(squared_reach) > horizontal {
                let my_position = e.vcall(actor, ACTOR_SLOT_POSITION, &args![]).u32();
                let rise =
                    (f64::from(e.mem.f32(my_position + 8)) - f64::from(e.mem.f32(node + 8))) as f32;
                let rise = e.call(FLOAT_ABSOLUTE, &args![rise]).f64();
                if f64::from(reach) > rise {
                    e.call(ACTOR_FORCE_STOP_MOVING, &args![actor]);
                    let mover = e.mem.u32(actor + ACTOR_MOVER);
                    e.call(MOVER_MOVE_TO_LOCATION, &args![mover, node + 0x10]);
                    if !e.call(ACTOR_SWIM_TEST_B, &args![actor]).bool() {
                        let object = e
                            .vcall(process, SLOT_GET_CHARACTER_CONTROLLER, &args![])
                            .u32();
                        if object != 0 {
                            let flag = if e.mem.u8(node + 0x20) != 0 { 2u32 } else { 0 };
                            e.call(SET_WORD_AT_0X3F0, &args![object, flag]);
                        }
                    }
                    e.call(BREADCRUMB_POP, &args![crumbs]);
                    *following = e.call(BREADCRUMB_FIRST, &args![crumbs]).u32() != 0;
                    e.call(NAV_POINTER_DESTRUCTOR, &args![mesh_pointer]);
                    return;
                }
            }
        }
        // Not there yet: a request to the breadcrumb is built and set.
        e.with_stack(0xb4, |e, request| {
            e.call(PATHING_REQUEST_CONSTRUCTOR, &args![request]);
            if e.call(NAV_POINTER_IS_SET, &args![mesh_pointer]).u32() != 0 {
                let held = e.call(NI_POINTER_GET, &args![mesh_pointer]).u32();
                let space = e.call(NAV_MESH_PARENT_SPACE, &args![held]).u32();
                e.call(
                    ACTOR_BUILD_REQUEST_OV2,
                    &args![actor, request, node, space, reach, 0u32],
                );
            } else {
                let world_space = e.call(REFERENCE_GET_WORLD_SPACE, &args![actor]).u32();
                let cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
                e.call(
                    ACTOR_BUILD_REQUEST,
                    &args![actor, request, node, cell, world_space, reach, 0u32],
                );
            }
            e.call(ACTOR_SET_PATHFINDING_GOAL_REQUEST, &args![actor, request]);
            for word in 0..3 {
                let value = e.mem.u32(node + word * 4);
                e.mem.set_u32(f.position + word * 4, value);
            }
            e.call(PATHING_REQUEST_DESTRUCTOR, &args![request]);
        });
        e.call(NAV_POINTER_DESTRUCTOR, &args![mesh_pointer]);
    });
}

/// `ProcessFollow`, `008e21c3`: the move mode and animation of a moving
/// actor.
fn follow_choose_animation(e: &mut Engine, f: &Follow) {
    let actor = f.actor;
    let process = f.process;
    let mut running_target = true;
    if f.follow_actor != 0 {
        running_target = e.call(ACTOR_MOVE_MODE, &args![f.follow_actor]).u32() & 0x200 != 0;
    }
    if f.mode != -1 {
        e.vcall(
            process,
            SLOT_SET_ACTORS_ANIMATION,
            &args![actor, f.mode, u32::from(running_target)],
        );
        return;
    }
    let mut mode_value = 0u32;
    let mut package_is_trade_like = false;
    if f.package != 0 {
        let kind = e.call(PACKAGE_TYPE, &args![f.package]).i32();
        if kind == 0x15 || kind == 0x12 {
            package_is_trade_like = true;
        }
    }
    let mut work_out = true;
    if e.call(PACKAGE_GET_IS_CREATED, &args![f.package]).bool() {
        let current = e.vcall(process, SLOT_GET_CURRENT_PACKAGE, &args![]).u32();
        if current != 0
            && e.call(PACKAGE_FLAG_200, &args![current]).bool()
            && e.call(PACKAGE_FLAG_1, &args![current]).bool()
            && e.call(REFERENCE_PATHING_CELL, &args![actor]).u32() != 0
        {
            let cell = e.call(REFERENCE_PATHING_CELL, &args![actor]).u32();
            if e.call(CELL_ALLOWS_ACTOR, &args![cell, actor]).bool() {
                mode_value = 0x101;
                work_out = false;
            }
        }
    }
    if work_out {
        let doubled = (f64::from(f.follow_distance) + f64::from(f.follow_distance)) as f32;
        mode_value = e
            .vcall(
                process,
                SLOT_CALCULATE_MOVE_MODE,
                &args![
                    actor,
                    f.distance,
                    f.follow_distance,
                    doubled,
                    u32::from(package_is_trade_like),
                    0u32
                ],
            )
            .u32();
    }
    e.vcall(
        process,
        SLOT_SET_ACTORS_ANIMATION,
        &args![actor, mode_value, u32::from(running_target)],
    );
}

/// `ProcessFollow`, `008e2381` to the end: the move mode and the speeds of
/// the actor, and, when it is close enough and the follow continues for the
/// player, the stop and the greeting.
fn follow_finish(e: &mut Engine, f: &Follow) {
    let actor = f.actor;
    let process = f.process;
    let mover = e.mem.u32(actor + ACTOR_MOVER);
    let mover_value = e.vcall(mover, MOVER_SLOT_0X1C, &args![]).u32();
    let mode = e
        .call(
            ACTOR_MODIFY_MOVE_MODE_FOR_FOLLOW,
            &args![actor, mover_value, f.radius, f.distance],
        )
        .u32();
    e.call(ACTOR_SET_MOVE_MODE, &args![actor, mode]);
    let fast_mode = mode & 0x200 != 0 && mode & 0xf != 0;
    let slot_blocked = e.vcall(actor, ACTOR_SLOT_0X390, &args![]).u32() == 0;
    let run = fast_mode || slot_blocked;
    let walk = !fast_mode || slot_blocked;
    if run {
        let speed = e.vcall(actor, ACTOR_SLOT_RUN_SPEED, &args![]).f32();
        let speed = e.with_stack(4, |e, slot| {
            e.mem.set_f32(slot.addr(), speed);
            e.call(
                ACTOR_ADJUST_SPEED_FOR_FOLLOWING,
                &args![actor, slot, f.radius, f.distance],
            );
            e.mem.f32(slot.addr())
        });
        e.call(ACTOR_SET_RUN_SPEED, &args![actor, speed]);
    }
    if walk {
        let speed = e.vcall(actor, ACTOR_SLOT_WALK_SPEED, &args![]).f32();
        let speed = e.with_stack(4, |e, slot| {
            e.mem.set_f32(slot.addr(), speed);
            e.call(
                ACTOR_ADJUST_SPEED_FOR_FOLLOWING,
                &args![actor, slot, f.radius, f.distance],
            );
            e.mem.f32(slot.addr())
        });
        e.call(ACTOR_SET_WALK_SPEED, &args![actor, speed]);
    }
    if f.breadcrumbs || !(f.near_distance > f.distance) {
        return;
    }
    if f.target == f.player
        && !e
            .call(
                ACTOR_LINE_OF_SIGHT,
                &args![actor, 0u32, f.target, 1u32, 0u32, 0u32],
            )
            .bool()
    {
        return;
    }
    if e.call(ACTOR_IS_PATHING_COMPLETE, &args![actor]).bool()
        || e.call(ACTOR_IS_ROTATING, &args![actor]).bool()
    {
        return;
    }
    let mut greeter = 0u32;
    if e.call(ACTOR_PROCESS, &args![actor]).u32() != 0 {
        let held = e.call(ACTOR_PROCESS, &args![actor]).u32();
        if e.call(PROCESS_WORD_AT_0X28, &args![held]).u32() == 0 {
            greeter = e.call(ACTOR_PROCESS, &args![actor]).u32();
        }
    }
    let topic = e.call(GET_TOPIC, &args![6u32, 2u32]).u32();
    if e.call(ACTOR_IS_CONTINUING_PACKAGE_FOR_PC, &args![actor])
        .bool()
        && greeter != 0
    {
        e.call(ACTOR_SET_FACING_TARGET, &args![actor, f.player]);
        if topic != 0 {
            e.vcall(
                greeter,
                SLOT_GREET,
                &args![actor, topic, 0u32, 0u32, 1u32, 0u32],
            );
        } else if e.vcall(process, SLOT_ASK_GREETING, &args![]).bool() {
            e.vcall(
                greeter,
                SLOT_GREET,
                &args![actor, 0u32, 0u32, 0u32, 1u32, 0u32],
            );
        }
    }
    e.call(ACTOR_END_MOVEMENT, &args![actor]);
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x008d9780,
            high_process_set_avoid_wait_timer(Ptr<HighProcess>, f32)
        ),
        entry!(
            0x008d97a0,
            high_process_get_radiation_delta(Ptr<HighProcess>) -> f32
        ),
        entry!(
            0x008d97f0,
            high_process_set_force_rotate(Ptr<HighProcess>, u8)
        ),
        entry!(
            0x008d9810,
            high_process_get_force_rotate(Ptr<HighProcess>) -> u8
        ),
        entry!(
            0x008d9830,
            high_process_get_last_head_track_target(Ptr<HighProcess>) -> Ptr
        ),
        entry!(
            0x008d9850,
            high_process_clear_last_head_track_target(Ptr<HighProcess>)
        ),
        entry!(
            0x008d9870,
            high_process_clear_dialog_target(Ptr<HighProcess>)
        ),
        entry!(
            0x008d9890,
            high_process_scalar_deleting_destructor(Ptr<HighProcess>, u32) -> Ptr<HighProcess>
        ),
        entry!(0x008d98c0, fn_008d98c0(Ptr)),
        entry!(0x008d98e0, high_process_destructor(Ptr<HighProcess>)),
        entry!(
            0x008d9fa0,
            high_process_high_copy(Ptr<HighProcess>, Ptr<HighProcess>)
        ),
        entry!(0x008da180, fn_008da180(Ptr<HighProcess>) -> f32),
        entry!(0x008da1a0, fn_008da1a0(Ptr<HighProcess>, Ptr)),
        entry!(
            0x008da230,
            high_process_get_anim_action(Ptr<HighProcess>) -> i32
        ),
        entry!(
            0x008da250,
            high_process_get_anim_action_anim_seq(Ptr<HighProcess>) -> Ptr
        ),
        entry!(
            0x008da270,
            high_process_set_anim_action(Ptr<HighProcess>, i16, Ptr)
        ),
        entry!(
            0x008da2a0,
            high_process_can_attack(Ptr<HighProcess>) -> bool
        ),
        entry!(0x008da3d0, fn_008da3d0(Ptr) -> u32),
        entry!(
            0x008da420,
            high_process_can_force_greet(Ptr<HighProcess>, Ptr) -> bool
        ),
        entry!(
            0x008da600,
            high_process_clear_acquire_list(Ptr<HighProcess>)
        ),
        entry!(
            0x008da670,
            high_process_checkfor_new_package(Ptr<HighProcess>, Ptr, u8) -> bool
        ),
        entry!(0x008da8b0, high_process_setup_new_package(Ptr<HighProcess>)),
        entry!(
            0x008da950,
            high_process_set_actors_animation(Ptr<HighProcess>, Ptr, i32, u8)
        ),
        entry!(
            0x008da9f0,
            high_process_end_move_message(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008daa20,
            high_process_calculate_move_mode(Ptr<HighProcess>, Ptr, f32, f32, f32, u8, u8) -> u32
        ),
        entry!(
            0x008dab40,
            high_process_setup_special_idle(Ptr<HighProcess>, Ptr, Ptr, i32, u8, u8, u8) -> bool
        ),
        entry!(0x008dade0, fn_008dade0(Ptr) -> bool),
        entry!(
            0x008dae00,
            high_process_finish_setup_special_idle(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008daef0,
            high_process_free_up_special_idle(Ptr<HighProcess>, u32)
        ),
        entry!(
            0x008daf20,
            high_process_post_anim_free_up_special_idle(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008dafd0,
            high_process_randomly_play_special_idles(Ptr<HighProcess>, Ptr)
        ),
        entry!(0x008db240, fn_008db240(Ptr<HighProcess>, Ptr)),
        entry!(0x008db4c0, fn_008db4c0(Ptr, f32)),
        entry!(
            0x008db4f0,
            high_process_process_create_follow(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008db810,
            high_process_process_observe_combat(Ptr<HighProcess>, Ptr)
        ),
        entry!(0x008dbd80, fn_008dbd80(Ptr, f32)),
        entry!(0x008dbda0, fn_008dbda0(Ptr, Ptr)),
        entry!(
            0x008dbdc0,
            high_process_clear_greeting_info_data(Ptr<HighProcess>)
        ),
        entry!(
            0x008dbe30,
            high_process_process_greet(Ptr<HighProcess>, Ptr, Ptr, u32, u8, u8, u8)
        ),
        entry!(0x008dd880, fn_008dd880(Ptr)),
        entry!(
            0x008dd8e0,
            high_process_process_alert_behavior(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008dd980,
            high_process_process_searchfor_target(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008ddac0,
            high_process_process_flee_non_combat(Ptr<HighProcess>, Ptr)
        ),
        entry!(0x008defe0, fn_008defe0(Ptr) -> u8),
        entry!(0x008df000, fn_008df000(Ptr)),
        entry!(0x008df020, fn_008df020(Ptr) -> u8),
        entry!(0x008df040, fn_008df040(Ptr) -> u8),
        entry!(
            0x008df060,
            high_process_process_surface(Ptr<HighProcess>, Ptr)
        ),
        entry!(0x008df1c0, fn_008df1c0(Ptr, u8)),
        entry!(
            0x008df1e0,
            high_process_should_wait_for_escort_target(
                Ptr<HighProcess>,
                Ptr,
                Ptr,
                Ptr,
                Ptr,
            ) -> bool
        ),
        entry!(0x008df900, fn_008df900(Ptr) -> Ptr),
        entry!(0x008df980, fn_008df980(Ptr, u32) -> Ptr),
        entry!(0x008df9b0, fn_008df9b0(Ptr)),
        entry!(
            0x008dfa10,
            high_process_process_escort(Ptr<HighProcess>, Ptr)
        ),
        entry!(0x008e0ab0, fn_008e0ab0(Ptr, u8)),
        entry!(0x008e0ad0, fn_008e0ad0(Ptr<HighProcess>, Ptr, f32)),
        entry!(
            0x008e0b00,
            high_process_process_accompany(Ptr<HighProcess>, Ptr)
        ),
        entry!(0x008e30e0, fn_008e30e0(Ptr) -> bool),
        entry!(0x008e3100, fn_008e3100(Ptr) -> bool),
        entry!(0x008e3120, fn_008e3120(Ptr) -> bool),
        entry!(
            0x008e3dc0,
            fn_008e3dc0(Ptr, u32, u32, u32, f32, f32, u8) -> Ptr
        ),
        entry!(0x008e3e90, fn_008e3e90() -> bool),
        entry!(0x008e3ea0, fn_008e3ea0(Ptr) -> bool),
        entry!(
            0x008e40a0,
            high_process_reduce_detection_timer(Ptr<HighProcess>)
        ),
        entry!(0x008e5100, fn_008e5100(Ptr, Ptr) -> Ptr),
        entry!(0x008e5620, fn_008e5620(Ptr, Ptr) -> Ptr),
        entry!(0x008e5670, fn_008e5670(Ptr) -> u32),
        entry!(0x008e56b0, fn_008e56b0(Ptr<HighProcess>)),
        entry!(0x008e5700, fn_008e5700(Ptr<HighProcess>) -> bool),
        entry!(
            0x008e26e0,
            high_process_process_sleep(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008e2b00,
            high_process_evaluate_order_acquire_list(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008e3f00,
            high_process_guard_call_for_help(Ptr<HighProcess>, Ptr, u32)
        ),
        entry!(
            0x008e0f80,
            high_process_process_follow(Ptr<HighProcess>, Ptr, u8, i32, u8)
        ),
        entry!(
            0x008e3140,
            high_process_process_eat(Ptr<HighProcess>, Ptr, u32)
        ),
        entry!(0x008e4e50, fn_008e4e50(Ptr<HighProcess>, Ptr)),
        entry!(0x008e51b0, fn_008e51b0(Ptr<HighProcess>, Ptr)),
        entry!(0x008e5730, fn_008e5730(Ptr<HighProcess>, Ptr)),
        entry!(
            0x008e59c0,
            high_process_process_buy_object(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008e40d0,
            high_process_run_detection(Ptr<HighProcess>, Ptr, f32)
        ),
        entry!(0x008dddf0, high_process_process_flee(Ptr<HighProcess>, Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// Registers a double that returns `value` in `eax`.
    fn returning(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| ret(value));
    }

    /// Registers a double that returns `value` in `ST0`.
    fn returning_float(e: &mut Engine, addr: u32, value: f64) {
        e.register_double(addr, move |_, _| Ret {
            st0: value,
            ..Ret::default()
        });
    }

    /// Registers doubles that return 0 and do nothing else.
    fn stub(e: &mut Engine, addrs: &[u32]) {
        for &addr in addrs {
            e.register(addr, |_, _| Ret::default());
        }
    }

    /// `006815c0` returns `this`; `00726070` follows the node's next word;
    /// `00559450` reads the first word of `this`.
    fn list_and_pointer_doubles(e: &mut Engine) {
        e.register(NODE_ITEM_ADDRESS, |_, a| ret(a[0]));
        e.register(NODE_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(NI_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
    }

    /// A global word at `addr`.
    fn global_word(e: &mut Engine, addr: u32, value: u32) {
        e.map(addr, 8);
        e.mem.set_u32(addr, value);
    }

    fn global_f32(e: &mut Engine, addr: u32, value: f32) {
        e.map(addr, 8);
        e.mem.set_f32(addr, value);
    }

    fn global_f64(e: &mut Engine, addr: u32, value: f64) {
        e.map(addr, 8);
        e.mem.set_f64(addr, value);
    }

    /// A zeroed block big enough for every class here (`HighProcess` is
    /// 0x46c bytes).
    fn block(e: &mut Engine) -> Ptr<HighProcess> {
        Ptr::new(e.mem.alloc(0x800))
    }

    /// Gives `object` the vtable at `table` whose slots return `value` for
    /// each `(slot, value)` pair; the targets are doubles that record their
    /// calls.
    fn give_vtable(e: &mut Engine, object: u32, table: u32, slots: &[(u32, u32)]) {
        e.map(table, 0x900);
        for &(slot, value) in slots {
            let target = slot_target(table, slot);
            e.mem.set_u32(table + slot, target);
            returning(e, target, value);
        }
        e.mem.set_u32(object, table);
    }

    /// A new object with such a vtable.
    fn object_with_slots(e: &mut Engine, table: u32, slots: &[(u32, u32)]) -> u32 {
        let object = e.mem.alloc(0x800);
        give_vtable(e, object, table, slots);
        object
    }

    /// The target address of a slot made by [`give_vtable`].
    fn slot_target(table: u32, slot: u32) -> u32 {
        table + 0x10_0000 + slot
    }

    /// Makes a slot of `table` return a float in `ST0`.
    fn float_slot(e: &mut Engine, table: u32, slot: u32, value: f64) {
        let target = slot_target(table, slot);
        e.map(table, 0x900);
        e.mem.set_u32(table + slot, target);
        returning_float(e, target, value);
    }

    fn calls_to(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The addresses of the logged calls, in order, without the first (the
    /// call the test makes).
    fn call_order(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .skip(1)
            .map(|(a, _)| *a)
            .collect()
    }

    const PROCESS_TABLE: u32 = 0x7100_0000;
    const ACTOR_TABLE: u32 = 0x7200_0000;
    const OTHER_TABLE: u32 = 0x7300_0000;

    #[test]
    fn test_high_process_set_avoid_wait_timer() {
        let mut e = Engine::new();
        let p = block(&mut e);
        e.call(0x008d9780, &args![p, 2.5f32]);
        assert_eq!(e.mem.f32(p.addr() + 0x450), 2.5);
        assert_eq!(e.mem.u32(p.addr() + 0x454), 0, "only the timer is written");
    }

    #[test]
    fn test_high_process_get_radiation_delta() {
        let mut e = Engine::new();
        let p = block(&mut e);
        // fHighestRadiation (+0x440) and fRadiationDelta (+0x234)
        e.mem.set_f32(p.addr() + 0x440, 3.0);
        e.mem.set_f32(p.addr() + 0x234, 2.0);
        assert_eq!(e.call(0x008d97a0, &args![p]).f32(), 3.0, "the larger one");
        e.mem.set_f32(p.addr() + 0x440, 1.0);
        assert_eq!(e.call(0x008d97a0, &args![p]).f32(), 2.0);
        e.mem.set_f32(p.addr() + 0x440, f32::NAN);
        assert_eq!(
            e.call(0x008d97a0, &args![p]).f32(),
            2.0,
            "unordered values give the +0x234 one"
        );
    }

    #[test]
    fn test_high_process_force_rotate() {
        let mut e = Engine::new();
        let p = block(&mut e);
        e.call(0x008d97f0, &args![p, 7u32]);
        assert_eq!(e.mem.u8(p.addr() + 0x420), 7);
        assert_eq!(e.mem.u8(p.addr() + 0x421), 0);
        assert_eq!(e.call(0x008d9810, &args![p]).u8(), 7);
    }

    #[test]
    fn test_high_process_last_head_track_target_and_dialog_target() {
        let mut e = Engine::new();
        let p = block(&mut e);
        e.mem.set_u32(p.addr() + 0x41c, 0x1234_5678);
        e.mem.set_u32(p.addr() + 0x370, 0x2222);
        assert_eq!(e.call(0x008d9830, &args![p]).u32(), 0x1234_5678);
        e.call(0x008d9850, &args![p]);
        assert_eq!(e.mem.u32(p.addr() + 0x41c), 0);
        assert_eq!(
            e.mem.u32(p.addr() + 0x370),
            0x2222,
            "the dialog target stays"
        );
        e.call(0x008d9870, &args![p]);
        assert_eq!(e.mem.u32(p.addr() + 0x370), 0);
    }

    /// Doubles for everything `~HighProcess` calls.
    fn destructor_world(e: &mut Engine, process_of_player: u32) {
        stub(
            e,
            &[
                0x0042_26e0,
                0x005b_5e40,
                0x0096_11e0,
                NI_POINTER_SET,
                0x004d_5850,
                LIST_CLEAR,
                LIST_DELETE,
                OPERATOR_DELETE,
                0x0090_4160,
                0x008f_f1b0,
                0x005c_90d0,
                0x008d_9f70,
                KF_MODEL_POINTER_SET,
                0x006e_bf90,
                NI_POINTER_DESTRUCTOR,
                LIST_DESTRUCTOR,
                VECTOR_DESTRUCT,
                BASE_DESTRUCTOR,
            ],
        );
        list_and_pointer_doubles(e);
        returning(e, ACTOR_PROCESS, process_of_player);
        global_word(e, PLAYER_POINTER, 0x0aaa_0000);
        global_word(e, 0x011c_3f2c, 0x0bbb_0000);
    }

    /// Points the five detection lists at an empty node.
    fn empty_detection_lists(e: &mut Engine, process: u32) {
        let empty = e.mem.alloc(8);
        for offset in [0x264u32, 0x25c, 0x268, 0x26c, 0x260] {
            e.mem.set_u32(process + offset, empty);
        }
    }

    #[test]
    fn test_high_process_destructor_releases_members_in_order() {
        let mut e = Engine::new();
        destructor_world(&mut e, 0x0ccc_0000);
        let p = block(&mut e);
        let a = p.addr();
        // pDetectedActorList: two nodes {item, next}; the second ends the chain.
        let second = e.mem.alloc(8);
        e.mem.set_u32(second, 0x5002);
        let first = e.mem.alloc(8);
        e.mem.set_u32(first, 0x5001);
        e.mem.set_u32(first + 4, second);
        e.mem.set_u32(a + 0x25c, first);
        // pThreadDetectList: the first node has no item, so nothing is deleted.
        let empty_node = e.mem.alloc(8);
        e.mem.set_u32(a + 0x268, empty_node);
        // pLastSpokeToList: emptied but its items are not deleted.
        let spoke = e.mem.alloc(8);
        e.mem.set_u32(spoke, 0x6001);
        e.mem.set_u32(a + 0x264, spoke);
        e.mem.set_u32(a + 0x3cc, 0x9999);
        e.mem.set_u32(a + 0x2cc, 0x4444);
        e.mem.set_u32(a + 0x368, 0x7001);
        e.mem.set_u32(a + 0x3b4, 0x7002);
        e.mem.set_u32(a + 0x3d4, 0x7003);
        e.mem.set_u32(a + 0x428, 0x7004);
        e.mem.set_u32(a + 0x3dc, 0x7005);
        e.call_log = Some(vec![]);
        e.call(0x008d98e0, &args![p]);
        assert_eq!(e.mem.u32(a), HIGH_PROCESS_VTABLE_ADDRESS);
        for offset in [0x3cc, 0x2cc, 0x3b4, 0x3d4, 0x428] {
            assert_eq!(e.mem.u32(a + offset), 0, "+{offset:#x} is cleared");
        }
        assert_eq!(e.mem.u32(a + 0x368), 0x7001, "pGreetTopic is only deleted");
        assert_eq!(calls_to(&e, 0x004d_5850), vec![vec![0x9999, 1]]);
        assert_eq!(
            calls_to(&e, OPERATOR_DELETE),
            vec![vec![0x5001], vec![0x5002], vec![0x7004], vec![0x7005]],
            "owned items, then the cached values and the generated events"
        );
        assert_eq!(
            calls_to(&e, LIST_DELETE),
            vec![
                vec![spoke, 1],
                vec![first, 1],
                vec![empty_node, 1],
                vec![0x7002, 1]
            ]
        );
        assert_eq!(calls_to(&e, 0x0090_4160), vec![vec![a]]);
        assert_eq!(calls_to(&e, 0x008f_f1b0), vec![vec![a, 0], vec![a, 1]]);
        assert_eq!(calls_to(&e, 0x005c_90d0), vec![vec![0x7001, 1]]);
        assert_eq!(calls_to(&e, 0x008d_9f70), vec![vec![0x7003, 1]]);
        assert_eq!(
            calls_to(&e, NI_POINTER_SET),
            vec![vec![a + 0x354, 0], vec![a + 0x358, 0], vec![a + 0x434, 0]]
        );
        assert_eq!(
            calls_to(&e, KF_MODEL_POINTER_SET),
            vec![vec![a + 0x35c, 0], vec![a + 0x360, 0]]
        );
        assert_eq!(
            calls_to(&e, VECTOR_DESTRUCT),
            vec![
                vec![a + 0x35c, 4, 2, KF_MODEL_POINTER_DESTRUCTOR],
                vec![a + 0x354, 4, 2, NI_POINTER_DESTRUCTOR],
                vec![a + 0x314, 0xc, 2, SOUND_HANDLE_DESTRUCTOR],
            ]
        );
        assert_eq!(
            calls_to(&e, LIST_DESTRUCTOR),
            vec![
                vec![a + 0x394],
                vec![a + 0x38c],
                vec![a + 0x28c],
                vec![a + 0x284],
                vec![a + 0x27c],
                vec![a + 0x274],
            ]
        );
        assert_eq!(
            call_order(&e).last().copied(),
            Some(BASE_DESTRUCTOR),
            "the base class' destructor runs last"
        );
        assert!(
            calls_to(&e, 0x005b_5e40).is_empty(),
            "not the player's process"
        );
    }

    #[test]
    fn test_high_process_destructor_warns_for_the_players_process() {
        let mut e = Engine::new();
        let p = block(&mut e);
        destructor_world(&mut e, p.addr());
        empty_detection_lists(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008d98e0, &args![p]);
        assert_eq!(
            calls_to(&e, 0x0042_26e0),
            vec![vec![0x0bbb_0000]],
            "asks the object held by 011c3f2c"
        );
        assert_eq!(calls_to(&e, 0x005b_5e40), vec![vec![0x0108_80d8]]);
        // When that object says the game is shutting down there is no warning.
        let mut e = Engine::new();
        let p = block(&mut e);
        destructor_world(&mut e, p.addr());
        returning(&mut e, 0x0042_26e0, 1);
        empty_detection_lists(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008d98e0, &args![p]);
        assert!(calls_to(&e, 0x005b_5e40).is_empty());
    }

    #[test]
    fn test_high_process_destructor_releases_the_ni_pointer_through_its_object() {
        let mut e = Engine::new();
        destructor_world(&mut e, 0);
        let p = block(&mut e);
        let a = p.addr();
        empty_detection_lists(&mut e, a);
        // The NiPointer at +0x380 holds `held`; 009611e0 finds `found`, whose
        // virtual +0xe8 gets `held`.
        let held = 0x00dd_0000;
        e.mem.set_u32(a + 0x380, held);
        let found = object_with_slots(&mut e, OTHER_TABLE, &[(0xe8, 0)]);
        returning(&mut e, 0x0096_11e0, found);
        e.call_log = Some(vec![]);
        e.call(0x008d98e0, &args![p]);
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, 0xe8)),
            vec![vec![found, held]]
        );
        assert_eq!(calls_to(&e, NI_POINTER_SET)[0], vec![a + 0x380, 0]);
    }

    #[test]
    fn test_high_process_scalar_deleting_destructor() {
        let mut e = Engine::new();
        destructor_world(&mut e, 0);
        let p = block(&mut e);
        empty_detection_lists(&mut e, p.addr());
        e.call_log = Some(vec![]);
        let r = e.call(0x008d9890, &args![p, 0u32]);
        assert_eq!(r.ptr::<()>().addr(), p.addr(), "returns this");
        assert_eq!(call_order(&e).last().copied(), Some(BASE_DESTRUCTOR));
        assert!(
            calls_to(&e, OPERATOR_DELETE).is_empty(),
            "flag 0 keeps the object"
        );
        e.call_log = Some(vec![]);
        e.call(0x008d9890, &args![p, 1u32]);
        assert_eq!(call_order(&e).last().copied(), Some(OPERATOR_DELETE));
        assert_eq!(calls_to(&e, OPERATOR_DELETE), vec![vec![p.addr()]]);
    }

    #[test]
    fn test_fn_008d98c0_forwards_a_null_pointer() {
        let mut e = Engine::new();
        stub(&mut e, &[0x0044_afe0]);
        let p = block(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008d98c0, &args![p]);
        assert_eq!(calls_to(&e, 0x0044_afe0), vec![vec![p.addr(), 0]]);
    }

    #[test]
    fn test_high_process_high_copy() {
        let mut e = Engine::new();
        list_and_pointer_doubles(&mut e);
        stub(&mut e, &[LIST_ADD_ITEM]);
        let this = block(&mut e);
        let other = block(&mut e);
        let (t, o) = (this.addr(), other.addr());
        give_vtable(&mut e, o, OTHER_TABLE, &[(0x78, 1)]);
        e.mem.set_f32(o + 0x2b4, 4.5);
        e.mem.set_f32(o + 0x2b8, 1.25);
        e.mem.set_f32(o + 0x2bc, 2.25);
        e.mem.set_u8(o + 0x375, 1);
        // Detected-actor list: two nodes; who-detects-me list: one node.
        let second = e.mem.alloc(8);
        e.mem.set_u32(second, 0x22);
        let first = e.mem.alloc(8);
        e.mem.set_u32(first, 0x11);
        e.mem.set_u32(first + 4, second);
        e.mem.set_u32(o + 0x25c, first);
        let third = e.mem.alloc(8);
        e.mem.set_u32(third, 0x33);
        e.mem.set_u32(o + 0x260, third);
        e.mem.set_u32(t + 0x25c, 0x1000_0001);
        e.mem.set_u32(t + 0x260, 0x1000_0002);
        let copied_words = [
            (0x2e4u32, 0x1111u32),
            (0x2e8, 0x2222),
            (0x3ac, 0x3333),
            (0x3b0, 0x4444),
            (0x3cc, 0x5555),
            (0x370, 0x6666),
            (0x2ac, 0x7777),
        ];
        for (offset, value) in copied_words {
            e.mem.set_u32(o + offset, value);
        }
        let copied_bytes = [
            (0x3b9u32, 1u8),
            (0x3d0, 2),
            (0x348, 3),
            (0x29d, 4),
            (0x270, 5),
        ];
        for (offset, value) in copied_bytes {
            e.mem.set_u8(o + offset, value);
        }
        e.mem.set_f32(o + 0x3c4, 0.5);
        e.call_log = Some(vec![]);
        e.call(0x008d9fa0, &args![this, other]);
        assert_eq!(e.mem.f32(t + 0x2b4), 4.5, "through 008da180");
        assert_eq!(e.mem.f32(t + 0x2b8), 1.25);
        assert_eq!(e.mem.f32(t + 0x2bc), 2.25);
        assert_eq!(e.mem.u8(t + 0x375), 1);
        assert_eq!(
            e.mem.u8(t + 0x459),
            1,
            "bIsDoingSayTo from other's virtual +0x78"
        );
        for (offset, value) in copied_words {
            assert_eq!(e.mem.u32(t + offset), value, "+{offset:#x}");
        }
        for (offset, value) in copied_bytes {
            assert_eq!(e.mem.u8(t + offset), value, "+{offset:#x}");
        }
        assert_eq!(e.mem.f32(t + 0x3c4), 0.5);
        assert_eq!(
            calls_to(&e, LIST_ADD_ITEM),
            vec![
                vec![0x1000_0001, first],
                vec![0x1000_0001, second],
                vec![0x1000_0002, third],
            ]
        );
        assert_eq!(e.mem.u32(t + 0x264), 0, "other fields are not copied");
    }

    #[test]
    fn test_fn_008da180_returns_the_package_evaluation_timer() {
        let mut e = Engine::new();
        let p = block(&mut e);
        e.mem.set_f32(p.addr() + 0x2b4, 6.5);
        assert_eq!(e.call(0x008da180, &args![p]).f32(), 6.5);
    }

    #[test]
    fn test_fn_008da1a0_finds_the_bone_lod_controller() {
        let mut e = Engine::new();
        let p = block(&mut e);
        let actor = 0x00a0_0000u32;
        e.mem.set_u32(p.addr() + 0x2e4, 0xdead);
        returning(&mut e, 0x0043_fcd0, 0x0100);
        returning(&mut e, 0x0057_1550, 0x77);
        returning(&mut e, 0x0057_1530, 0x0200);
        returning(&mut e, 0x0043_b230, 0x0301);
        e.register(0x0045_bad0, |_, a| ret((a[1] == 0x0302) as u32));
        e.register(0x004a_8a90, |_, a| ret(a[0] + 1));
        e.call_log = Some(vec![]);
        e.call(0x008da1a0, &args![p, actor]);
        assert_eq!(
            e.mem.u32(p.addr() + 0x2e4),
            0x0302,
            "the second node is accepted"
        );
        assert_eq!(calls_to(&e, 0x0057_1530), vec![vec![actor, 0x0100, 0x77]]);
        assert_eq!(
            calls_to(&e, 0x0045_bad0),
            vec![vec![0x0120_31b8, 0x0301], vec![0x0120_31b8, 0x0302]]
        );
    }

    #[test]
    fn test_fn_008da1a0_leaves_the_pointer_clear_without_a_model() {
        let mut e = Engine::new();
        let p = block(&mut e);
        e.mem.set_u32(p.addr() + 0x2e4, 0xdead);
        returning(&mut e, 0x0043_fcd0, 0);
        e.call_log = Some(vec![]);
        e.call(0x008da1a0, &args![p, 0x00a0_0000u32]);
        assert_eq!(e.mem.u32(p.addr() + 0x2e4), 0);
        assert_eq!(call_order(&e), vec![0x0043_fcd0]);
        // A model without a node chain: nothing accepted.
        returning(&mut e, 0x0043_fcd0, 0x0100);
        returning(&mut e, 0x0057_1550, 0);
        returning(&mut e, 0x0057_1530, 0x0200);
        returning(&mut e, 0x0043_b230, 0);
        e.mem.set_u32(p.addr() + 0x2e4, 0xdead);
        e.call(0x008da1a0, &args![p, 0x00a0_0000u32]);
        assert_eq!(e.mem.u32(p.addr() + 0x2e4), 0);
    }

    #[test]
    fn test_high_process_anim_action_accessors() {
        let mut e = Engine::new();
        let p = block(&mut e);
        e.call(0x008da270, &args![p, -2i16, 0x1234u32]);
        assert_eq!(e.mem.i16(p.addr() + 0x2ec), -2);
        assert_eq!(e.mem.u32(p.addr() + 0x2f0), 0x1234);
        assert_eq!(e.call(0x008da230, &args![p]).i32(), -2, "sign-extended");
        assert_eq!(e.call(0x008da250, &args![p]).u32(), 0x1234);
    }

    /// A process whose virtual `+0x1b8` gives `animation` and `+0x3e4`
    /// `action`, with doubles for the callees of `CanAttack`.
    fn attack_world(e: &mut Engine, action: i32, animation: u32) -> Ptr<HighProcess> {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x1b8, animation), (0x3e4, action as u32)],
        );
        global_word(e, PLAYER_POINTER, 0x0aaa_0000);
        returning(e, ACTOR_PROCESS, 0);
        stub(
            e,
            &[
                0x0049_1040,
                0x0048_f7f0,
                0x005f_2420,
                0x0088_46c0,
                0x0096_7ae0,
                0x0055_b980,
                0x005f_f260,
            ],
        );
        list_and_pointer_doubles(e);
        p
    }

    #[test]
    fn test_high_process_can_attack_depends_on_the_anim_action() {
        let mut e = Engine::new();
        for action in [-1, 3, 4, 6, 7, 0x11] {
            let p = attack_world(&mut e, action, 0);
            assert!(e.call(0x008da2a0, &args![p]).bool(), "action {action}");
        }
        for action in [-2, 0, 1, 8, 9, 0x10, 0x12] {
            let p = attack_world(&mut e, action, 0);
            e.call_log = Some(vec![]);
            assert!(!e.call(0x008da2a0, &args![p]).bool(), "action {action}");
            assert!(calls_to(&e, 0x0088_46c0).is_empty());
        }
    }

    #[test]
    fn test_high_process_can_attack_refuses_for_two_anim_groups() {
        let mut e = Engine::new();
        let animation = e.mem.alloc(0x200);
        for (action, group, expected) in [
            (2, 0xe2, false),
            (5, 0xf0, false),
            (2, 0x10, true),
            (5, 0x11, true),
        ] {
            let p = attack_world(&mut e, action, animation);
            returning(&mut e, 0x0049_1040, 0x600);
            returning(&mut e, 0x0048_f7f0, 0x700);
            returning(&mut e, 0x005f_2420, group);
            e.call_log = Some(vec![]);
            assert_eq!(
                e.call(0x008da2a0, &args![p]).bool(),
                expected,
                "action {action} group {group:#x}"
            );
            assert_eq!(calls_to(&e, 0x0049_1040), vec![vec![animation, 2]]);
            assert_eq!(calls_to(&e, 0x0048_f7f0), vec![vec![0x600]]);
            assert_eq!(calls_to(&e, 0x005f_2420), vec![vec![0x700]]);
        }
        // Without a sub-object the group is not asked.
        let p = attack_world(&mut e, 2, animation);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008da2a0, &args![p]).bool());
        assert!(calls_to(&e, 0x005f_2420).is_empty());
    }

    #[test]
    fn test_high_process_can_attack_checks_the_animation_and_the_players_pipboy() {
        let mut e = Engine::new();
        let animation = e.mem.alloc(0x200);
        let p = attack_world(&mut e, 3, animation);
        returning(&mut e, 0x0088_46c0, 1);
        assert!(!e.call(0x008da2a0, &args![p]).bool(), "008846c0 refuses");
        returning(&mut e, 0x0088_46c0, 0);
        assert!(e.call(0x008da2a0, &args![p]).bool());
        // The player's own process: refused while the Pipboy is active.
        returning(&mut e, ACTOR_PROCESS, p.addr());
        returning(&mut e, 0x0096_7ae0, 1);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da2a0, &args![p]).bool());
        assert_eq!(calls_to(&e, 0x0096_7ae0), vec![vec![0x0aaa_0000]]);
        returning(&mut e, 0x0096_7ae0, 0);
        assert!(e.call(0x008da2a0, &args![p]).bool());
    }

    #[test]
    fn test_high_process_can_attack_asks_the_animations_idle_form_for_other_processes() {
        let mut e = Engine::new();
        let animation = e.mem.alloc(0x200);
        let held = 0x00ee_0000;
        e.mem.set_u32(animation + 0x124, held);
        let p = attack_world(&mut e, 3, animation);
        returning(&mut e, 0x0055_b980, 0x900);
        returning(&mut e, 0x005f_f260, 1);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da2a0, &args![p]).bool());
        assert_eq!(calls_to(&e, 0x0055_b980), vec![vec![held]]);
        assert_eq!(calls_to(&e, 0x005f_f260), vec![vec![0x900]]);
        returning(&mut e, 0x005f_f260, 0);
        assert!(e.call(0x008da2a0, &args![p]).bool());
        // No idle form: not asked.
        returning(&mut e, 0x0055_b980, 0);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008da2a0, &args![p]).bool());
        assert!(calls_to(&e, 0x005f_f260).is_empty());
    }

    #[test]
    fn test_fn_008da3d0() {
        let mut e = Engine::new();
        list_and_pointer_doubles(&mut e);
        returning(&mut e, 0x0055_b980, 0x42);
        let animation = e.mem.alloc(0x200);
        assert_eq!(
            e.call(0x008da3d0, &args![animation]).u32(),
            0,
            "empty pointer"
        );
        e.mem.set_u32(animation + 0x124, 0x1000);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008da3d0, &args![animation]).u32(), 0x42);
        assert_eq!(calls_to(&e, 0x0055_b980), vec![vec![0x1000]]);
    }

    /// The world `CanForceGreet` needs, with everything allowing the greeting.
    fn force_greet_world(e: &mut Engine, action: i16) -> (Ptr<HighProcess>, u32, u32) {
        let player = object_with_slots(e, ACTOR_TABLE, &[(0x214, 0)]);
        global_word(e, PLAYER_POINTER, player);
        global_f64(e, 0x0107_3828, 0.875);
        e.map(0x011e_07a9, 8);
        returning(e, 0x0052_5430, 0);
        returning(e, 0x0088_c600, 1);
        returning(e, 0x0093_44a0, 0);
        returning(e, PACKAGE_TYPE, 0);
        returning(e, 0x008d_6f30, 0x55);
        let object = e.mem.alloc(0x40);
        e.mem.set_f32(object + 8, 0.5);
        returning(e, 0x0043_6aa0, object);
        returning_float(e, 0x0088_5560, 0.5);
        let p = block(e);
        e.mem.set_i16(p.addr() + 0x2ec, action);
        let other = e.mem.alloc(0x40);
        (p, other, player)
    }

    #[test]
    fn test_high_process_can_force_greet_early_refusals() {
        let mut e = Engine::new();
        let (p, other, player) = force_greet_world(&mut e, -1);
        assert!(e.call(0x008da420, &args![p, other]).bool(), "all clear");
        e.call_log = Some(vec![]);
        returning(&mut e, 0x0052_5430, 1);
        assert!(!e.call(0x008da420, &args![p, other]).bool());
        assert_eq!(calls_to(&e, 0x0052_5430), vec![vec![0x011f_2250]]);
        assert!(calls_to(&e, 0x0088_c600).is_empty(), "stops at once");
        returning(&mut e, 0x0052_5430, 0);
        returning(&mut e, 0x0088_c600, 0);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da420, &args![p, other]).bool());
        assert_eq!(calls_to(&e, 0x0088_c600), vec![vec![other, player]]);
        assert!(calls_to(&e, 0x0093_44a0).is_empty());
    }

    #[test]
    fn test_high_process_can_force_greet_looks_at_the_players_package_and_state() {
        let mut e = Engine::new();
        let (p, other, player) = force_greet_world(&mut e, -1);
        // A package of type 6 on the player.
        returning(&mut e, 0x0093_44a0, 0x600);
        returning(&mut e, PACKAGE_TYPE, 6);
        assert!(!e.call(0x008da420, &args![p, other]).bool());
        returning(&mut e, PACKAGE_TYPE, 5);
        assert!(e.call(0x008da420, &args![p, other]).bool());
        // The player's state must be 0 or 4.
        returning(&mut e, 0x0093_44a0, 0);
        for (state, expected) in [(0, true), (4, true), (1, false), (9, false)] {
            e.mem.set_u32(player, 0);
            give_vtable(&mut e, player, ACTOR_TABLE, &[(0x214, state)]);
            assert_eq!(
                e.call(0x008da420, &args![p, other]).bool(),
                expected,
                "state {state}"
            );
        }
    }

    #[test]
    fn test_high_process_can_force_greet_compares_the_players_level() {
        let mut e = Engine::new();
        let (p, other, player) = force_greet_world(&mut e, -1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008da420, &args![p, other]).bool());
        assert_eq!(
            calls_to(&e, 0x0088_5560),
            vec![vec![player, 0.5f32.to_bits(), 0x55]],
            "the float at +8 of 00436aa0's object, then the 008d6f30 word"
        );
        returning_float(&mut e, 0x0088_5560, 0.875);
        assert!(
            !e.call(0x008da420, &args![p, other]).bool(),
            "0.875 is too much"
        );
        returning_float(&mut e, 0x0088_5560, 0.874);
        assert!(e.call(0x008da420, &args![p, other]).bool());
        returning_float(&mut e, 0x0088_5560, f64::NAN);
        assert!(
            !e.call(0x008da420, &args![p, other]).bool(),
            "unordered refuses"
        );
    }

    #[test]
    fn test_high_process_can_force_greet_by_anim_action() {
        let mut e = Engine::new();
        let (p, other, player) = force_greet_world(&mut e, -1);
        // -1 and 7: allowed in the sit/sleep states 0, 4 and 9.
        for (action, state, expected) in [
            (-1, 0, true),
            (-1, 4, true),
            (7, 9, true),
            (7, 5, false),
            (3, 0, false),
            (0, 0, false),
        ] {
            e.mem.set_i16(p.addr() + 0x2ec, action);
            e.mem.set_u8(p.addr() + 0x13d, state);
            assert_eq!(
                e.call(0x008da420, &args![p, other]).bool(),
                expected,
                "action {action} state {state}"
            );
        }
        // 2, 4 and 5 refuse; with the player as `other` they set the flag.
        for action in [2, 4, 5] {
            e.mem.set_i16(p.addr() + 0x2ec, action);
            e.mem.set_u8(0x011e_07a9, 0);
            assert!(!e.call(0x008da420, &args![p, other]).bool());
            assert_eq!(e.mem.u8(0x011e_07a9), 0, "not the player");
            assert!(!e.call(0x008da420, &args![p, Ptr::<()>::new(player)]).bool());
            assert_eq!(e.mem.u8(0x011e_07a9), 1, "the player was greeted");
        }
    }

    #[test]
    fn test_high_process_clear_acquire_list() {
        let mut e = Engine::new();
        let p = block(&mut e);
        let list = p.addr() + 0x5c;
        e.mem.set_u32(list, 0x1000);
        list_and_pointer_doubles(&mut e);
        // Two rounds, then the list reports empty.
        let rounds = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let seen = rounds.clone();
        e.register_double(0x0082_56d0, move |_, _| {
            seen.set(seen.get() + 1);
            ret((seen.get() > 2) as u32)
        });
        stub(&mut e, &[0x007b_3fa0]);
        let holders = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let kept = holders.clone();
        e.register_double(LIST_REMOVE_ITEM, move |e, a| {
            kept.borrow_mut().push((a[0], e.mem.u32(a[1])));
            ret(0)
        });
        e.call_log = Some(vec![]);
        e.call(0x008da600, &args![p]);
        assert_eq!(
            calls_to(&e, 0x007b_3fa0),
            vec![vec![0x1000, 1], vec![0x1000, 1]],
            "the item is destroyed with the delete flag"
        );
        assert_eq!(*holders.borrow(), vec![(list, 0x1000), (list, 0x1000)]);
        assert_eq!(rounds.get(), 3);
        // A null head item is removed without being destroyed.
        e.mem.set_u32(list, 0);
        rounds.set(1);
        e.call_log = Some(vec![]);
        e.call(0x008da600, &args![p]);
        assert!(calls_to(&e, 0x007b_3fa0).is_empty());
        assert_eq!(holders.borrow().len(), 3);
    }

    /// A process and an actor with every `CheckforNewPackage` callee doubled
    /// so that a package change happens.
    fn package_world(e: &mut Engine) -> (Ptr<HighProcess>, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x20c, 0), (0x594, 0), (0x22c, 0), (0x23c, 3), (0x214, 0)],
        );
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x214, 0)]);
        returning(e, PACKAGE_TYPE, 5);
        returning(e, 0x0093_44a0, 0);
        returning_float(e, 0x0086_7da0, 13.7);
        returning(e, 0x0088_1510, 0);
        returning(e, 0x0067_4dd0, 0);
        returning(e, 0x0096_11e0, 2);
        e.register(FLOAT_TO_INT, |_, a| {
            let value = f64::from_bits(a[0] as u64 | (a[1] as u64) << 32);
            ret(value as i32 as u32)
        });
        returning(e, 0x0090_a1a0, 1);
        returning_float(e, FRAME_TIME, 0.5);
        stub(e, &[0x0092_bf20]);
        global_f64(e, 0x0101_2060, 0.0);
        global_f32(e, 0x0101_7868, 20.0);
        (p, actor)
    }

    #[test]
    fn test_high_process_checkfor_new_package_changes_the_package() {
        let mut e = Engine::new();
        let (p, actor) = package_world(&mut e);
        e.mem.set_i32(p.addr() + 0xb4, 99);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008da670, &args![p, actor, 0u32]).bool());
        assert_eq!(
            calls_to(&e, 0x0090_a1a0),
            vec![vec![p.addr(), actor, 0]],
            "LowProcess::CheckforNewPackage(actor, force)"
        );
        assert_eq!(e.mem.i32(p.addr() + 0xb4), 13, "the hour, truncated");
        assert_eq!(
            e.mem.f32(p.addr() + 0x2b4),
            19.5,
            "20.0 less the frame time"
        );
        assert_eq!(calls_to(&e, 0x0092_bf20), vec![vec![p.addr(), actor]]);
        assert!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x214)).is_empty(),
            "no run-once package to clear"
        );
    }

    #[test]
    fn test_high_process_checkfor_new_package_refusals() {
        let mut e = Engine::new();
        let (p, actor) = package_world(&mut e);
        // The actor's state must be 0, 4 or 9.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x214, 7)]);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da670, &args![p, actor, 1u32]).bool());
        assert!(calls_to(&e, 0x0090_a1a0).is_empty());
        for state in [0, 4, 9] {
            give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x214, state)]);
            assert!(e.call(0x008da670, &args![p, actor, 0u32]).bool(), "{state}");
        }
        // A run-once package blocks it unless forced.
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x20c, 0x1234),
                (0x594, 0),
                (0x22c, 0),
                (0x23c, 3),
                (0x214, 0),
            ],
        );
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da670, &args![p, actor, 0u32]).bool());
        assert!(calls_to(&e, 0x0090_a1a0).is_empty());
        assert!(e.call(0x008da670, &args![p, actor, 1u32]).bool());
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x214)).len(),
            2,
            "the run-once package (not type 0x1a) is cleared before and after"
        );
        // GetEndIdlesPlayed blocks it.
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x20c, 0), (0x594, 1), (0x22c, 0), (0x23c, 3), (0x214, 0)],
        );
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da670, &args![p, actor, 1u32]).bool());
        assert!(calls_to(&e, 0x0090_a1a0).is_empty());
    }

    #[test]
    fn test_high_process_checkfor_new_package_keeps_a_fresh_evaluation() {
        let mut e = Engine::new();
        let (p, actor) = package_world(&mut e);
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x20c, 0),
                (0x594, 0),
                (0x22c, 0x77),
                (0x23c, 3),
                (0x214, 0),
            ],
        );
        e.mem.set_f32(p.addr() + 0x2b4, 10.0);
        e.mem.set_i32(p.addr() + 0xb4, 13);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da670, &args![p, actor, 0u32]).bool());
        assert!(calls_to(&e, 0x0090_a1a0).is_empty(), "same hour, time left");
        assert_eq!(e.mem.f32(p.addr() + 0x2b4), 9.5);
        // Another hour, or no time left, or force: evaluated again.
        e.mem.set_i32(p.addr() + 0xb4, 12);
        assert!(e.call(0x008da670, &args![p, actor, 0u32]).bool());
        e.mem.set_i32(p.addr() + 0xb4, 13);
        e.mem.set_f32(p.addr() + 0x2b4, 0.0);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008da670, &args![p, actor, 0u32]).bool());
        assert_eq!(calls_to(&e, 0x0090_a1a0).len(), 1);
        e.mem.set_f32(p.addr() + 0x2b4, 10.0);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008da670, &args![p, actor, 1u32]).bool());
        assert_eq!(calls_to(&e, 0x0090_a1a0), vec![vec![p.addr(), actor, 1]]);
        // The evaluation says nothing changed: no follow-up.
        returning(&mut e, 0x0090_a1a0, 0);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da670, &args![p, actor, 1u32]).bool());
        assert!(calls_to(&e, 0x0092_bf20).is_empty());
    }

    #[test]
    fn test_high_process_checkfor_new_package_script_package_gate() {
        let mut e = Engine::new();
        let (p, actor) = package_world(&mut e);
        // A script package whose current procedure maps to 0x10 in table 2.
        returning(&mut e, 0x0088_1510, 0x500);
        returning(&mut e, 0x0067_4dd0, 1);
        let table = e.mem.alloc(0x40);
        e.map(0x011a_3ff0, 0x40);
        e.mem.set_u32(0x011a_3ff0 + 2 * 4, table);
        e.mem.set_u32(table + 3 * 4, 0x10);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008da670, &args![p, actor, 1u32]).bool());
        assert!(calls_to(&e, 0x0090_a1a0).is_empty());
        assert_eq!(calls_to(&e, 0x0067_4dd0), vec![vec![0x500]]);
        // Procedure 0x36 lets it through.
        e.mem.set_u32(table + 3 * 4, 0x36);
        assert!(e.call(0x008da670, &args![p, actor, 1u32]).bool());
    }

    #[test]
    fn test_high_process_setup_new_package() {
        let mut e = Engine::new();
        stub(&mut e, &[0x0091_5400]);
        let p = block(&mut e);
        let a = p.addr();
        for word in 0..0x46c / 4 {
            e.mem.set_u32(a + 4 * word, 0xaaaa_aaaa);
        }
        e.call_log = Some(vec![]);
        e.call(0x008da8b0, &args![p]);
        assert_eq!(calls_to(&e, 0x0091_5400), vec![vec![a]]);
        for offset in [0x375u32, 0x3a0, 0x349, 0x3e0] {
            assert_eq!(e.mem.u8(a + offset), 0, "+{offset:#x}");
        }
        assert_eq!(e.mem.i16(a + 0x2c0), 0);
        assert_eq!(e.mem.i16(a + 0x2c2), -1);
        assert_eq!(e.mem.i16(a + 0x2c4), 0);
        for offset in [0x2bcu32, 0x378, 0x2b8, 0x2e0, 0x194] {
            assert_eq!(e.mem.u32(a + offset), 0, "+{offset:#x}");
        }
        assert_eq!(e.mem.u8(a + 0x2c6), 0xaa, "neighbours are untouched");
        assert_eq!(e.mem.u32(a + 0x2b4), 0xaaaa_aaaa);
        assert_eq!(e.mem.u8(a + 0x374), 0xaa);
    }

    #[test]
    fn test_high_process_set_actors_animation() {
        let mut e = Engine::new();
        let p = block(&mut e);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x49c, 0), (0x27c, 0)]);
        returning(&mut e, 0x0088_46e0, 0xc00 | 0x3);
        returning(&mut e, 0x0067_a480, 0);
        stub(&mut e, &[ACTOR_SET_MOVE_MODE]);
        let actor = 0x00a0_0000u32;
        e.call_log = Some(vec![]);
        e.call(0x008da950, &args![p, actor, -1i32, 0u32]);
        assert_eq!(
            calls_to(&e, ACTOR_SET_MOVE_MODE),
            vec![vec![actor, 0xc00 | 0x100]],
            "-1 means 0x100; only the 0xc00 bits of the current mode stay"
        );
        e.call_log = Some(vec![]);
        e.call(0x008da950, &args![p, actor, 0x101i32, 0u32]);
        assert_eq!(calls_to(&e, ACTOR_SET_MOVE_MODE), vec![vec![actor, 0xd01]]);
        // The flag with a package that passes 0067a480 gives 0x201.
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x49c, 0), (0x27c, 0x500)],
        );
        returning(&mut e, 0x0067_a480, 1);
        e.call_log = Some(vec![]);
        e.call(0x008da950, &args![p, actor, 0x101i32, 1u32]);
        assert_eq!(calls_to(&e, 0x0067_a480), vec![vec![0x500]]);
        assert_eq!(calls_to(&e, ACTOR_SET_MOVE_MODE), vec![vec![actor, 0xe01]]);
        // Without the flag the package is not asked.
        e.call_log = Some(vec![]);
        e.call(0x008da950, &args![p, actor, 0x101i32, 0u32]);
        assert!(calls_to(&e, 0x0067_a480).is_empty());
        // The result is a signed 16-bit word.
        returning(&mut e, 0x0088_46e0, 0);
        e.call_log = Some(vec![]);
        e.call(0x008da950, &args![p, actor, 0x8000i32, 0u32]);
        assert_eq!(
            calls_to(&e, ACTOR_SET_MOVE_MODE),
            vec![vec![actor, 0xffff_8000]]
        );
        // Movement stopped: nothing happens.
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x49c, 1), (0x27c, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008da950, &args![p, actor, 0x101i32, 0u32]);
        assert!(calls_to(&e, ACTOR_SET_MOVE_MODE).is_empty());
    }

    #[test]
    fn test_high_process_end_move_message() {
        let mut e = Engine::new();
        let p = block(&mut e);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x120, 0)]);
        stub(&mut e, &[0x008b_3ab0]);
        e.call_log = Some(vec![]);
        e.call(0x008da9f0, &args![p, 0u32]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x120)),
            vec![vec![p.addr(), 1]]
        );
        assert!(calls_to(&e, 0x008b_3ab0).is_empty());
        e.call(0x008da9f0, &args![p, 0x00a0_0000u32]);
        assert_eq!(calls_to(&e, 0x008b_3ab0), vec![vec![0x00a0_0000]]);
    }

    /// The world of `CalculateMoveMode`: nothing refuses and no flag is set.
    fn move_mode_world(e: &mut Engine, package: u32) -> Ptr<HighProcess> {
        let p = block(e);
        give_vtable(e, p.addr(), PROCESS_TABLE, &[(0x27c, package)]);
        returning(e, 0x0088_46e0, 0);
        for address in [
            0x0043_7bf0,
            0x0043_7bd0,
            0x008d_8220,
            0x0067_a480,
            0x0049_3bb0,
        ] {
            returning(e, address, 0);
        }
        p
    }

    fn move_mode(
        e: &mut Engine,
        p: Ptr<HighProcess>,
        floats: [f32; 3],
        run: u32,
        walk: u32,
    ) -> u32 {
        e.call(
            0x008daa20,
            &args![
                p,
                0x00a0_0000u32,
                floats[0],
                floats[1],
                floats[2],
                run,
                walk
            ],
        )
        .u32()
    }

    #[test]
    fn test_high_process_calculate_move_mode_refusals_and_overrides() {
        let mut e = Engine::new();
        let p = move_mode_world(&mut e, 0);
        assert_eq!(move_mode(&mut e, p, [1.0, 2.0, 3.0], 0, 0), 0x101);
        returning(&mut e, 0x0043_7bf0, 1);
        assert_eq!(move_mode(&mut e, p, [1.0, 2.0, 3.0], 1, 0), 0, "00437bf0");
        returning(&mut e, 0x0043_7bf0, 0);
        returning(&mut e, 0x0043_7bd0, 1);
        assert_eq!(move_mode(&mut e, p, [1.0, 2.0, 3.0], 1, 0), 0, "00437bd0");
        returning(&mut e, 0x0043_7bd0, 0);
        returning(&mut e, 0x008d_8220, 1);
        assert_eq!(
            move_mode(&mut e, p, [1.0, 2.0, 3.0], 0, 1),
            0x201,
            "grenade"
        );
        returning(&mut e, 0x008d_8220, 0);
        returning(&mut e, 0x0049_3bb0, 1);
        assert_eq!(
            move_mode(&mut e, p, [1.0, 2.0, 3.0], 0, 1),
            0x201,
            "00493bb0"
        );
        returning(&mut e, 0x0049_3bb0, 0);
        // The package test only runs with a package.
        returning(&mut e, 0x0067_a480, 1);
        assert_eq!(move_mode(&mut e, p, [1.0, 2.0, 3.0], 0, 1), 0x101);
        let p = move_mode_world(&mut e, 0x500);
        returning(&mut e, 0x0067_a480, 1);
        assert_eq!(move_mode(&mut e, p, [1.0, 2.0, 3.0], 0, 1), 0x201);
        returning(&mut e, 0x0067_a480, 0);
        // The two flags: run wins.
        assert_eq!(move_mode(&mut e, p, [1.0, 2.0, 3.0], 1, 1), 0x201);
        assert_eq!(move_mode(&mut e, p, [1.0, 2.0, 3.0], 0, 1), 0x101);
    }

    #[test]
    fn test_high_process_calculate_move_mode_compares_distances() {
        let mut e = Engine::new();
        let p = move_mode_world(&mut e, 0);
        // Without bit 0x200 of the actor's move mode: run when c < a.
        returning(&mut e, 0x0088_46e0, 0x100);
        assert_eq!(move_mode(&mut e, p, [5.0, 0.0, 4.0], 0, 0), 0x201);
        assert_eq!(move_mode(&mut e, p, [5.0, 0.0, 5.0], 0, 0), 0x101, "equal");
        assert_eq!(move_mode(&mut e, p, [5.0, 0.0, 6.0], 0, 0), 0x101);
        assert_eq!(move_mode(&mut e, p, [5.0, 0.0, f32::NAN], 0, 0), 0x101);
        // With bit 0x200: walk (0x101) only when b > a.
        returning(&mut e, 0x0088_46e0, 0x200);
        assert_eq!(move_mode(&mut e, p, [5.0, 6.0, 0.0], 0, 0), 0x101);
        assert_eq!(move_mode(&mut e, p, [5.0, 5.0, 0.0], 0, 0), 0x201, "equal");
        assert_eq!(move_mode(&mut e, p, [5.0, 4.0, 0.0], 0, 0), 0x201);
        assert_eq!(move_mode(&mut e, p, [5.0, f32::NAN, 0.0], 0, 0), 0x201);
    }

    /// The world of `SetupSpecialIdle`: a normal actor with an animation that
    /// is done playing; `idle_to_pick` is what `00600950` finds.
    fn idle_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x128, 0x1111), (0x590, 0), (0x70c, 0), (0x614, 0)],
        );
        let animation = e.mem.alloc(0x40);
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[
                (0x1e4, animation),
                (0x100, 1),
                (0x214, 0),
                (0x234, 0),
                (0x230, 0),
            ],
        );
        global_word(e, PLAYER_POINTER, 0x0aaa_0000);
        global_word(e, 0x011c_b6a0, 0x0ddd_0000);
        for (address, value) in [
            (0x0044_ddc0, 0),
            (0x0043_7bf0, 0),
            (0x0093_44a0, 0),
            (0x0049_85f0, 1),
            (0x0043_6aa0, 0),
            (0x0068_0c30, 1),
            (0x008b_2010, 1),
            (0x0049_8f80, 0),
            (0x008a_7570, 0),
            (0x0060_0950, 0x2222),
            (0x0049_8d30, 0),
        ] {
            returning(e, address, value);
        }
        (p, actor, animation)
    }

    fn setup_idle(
        e: &mut Engine,
        p: Ptr<HighProcess>,
        actor: u32,
        idle: u32,
        mode: i32,
        flags: [u32; 3],
    ) -> bool {
        e.call(
            0x008dab40,
            &args![p, actor, idle, mode, flags[0], flags[1], flags[2]],
        )
        .bool()
    }

    #[test]
    fn test_high_process_setup_special_idle_starts_the_given_idle() {
        let mut e = Engine::new();
        let (p, actor, animation) = idle_world(&mut e);
        e.call_log = Some(vec![]);
        assert!(setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]));
        assert_eq!(e.mem.u32(p.addr() + 0x350), 0x5000, "pIdleToPlay");
        assert_eq!(e.mem.u32(p.addr() + 0x430), 3, "eSpecialIdleType");
        assert_eq!(calls_to(&e, 0x008b_2010), vec![vec![actor, 0x5000]]);
        assert_eq!(calls_to(&e, 0x0049_8d30), vec![vec![animation, 0x5000]]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x590)),
            vec![vec![p.addr(), 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x614)),
            vec![vec![p.addr(), 0x10]],
            "not the player: the post-animation action is queued"
        );
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x70c)).is_empty());
        assert!(calls_to(&e, 0x0060_0950).is_empty(), "no idle to pick");
    }

    #[test]
    fn test_high_process_setup_special_idle_picks_an_idle() {
        let mut e = Engine::new();
        let (p, actor, _) = idle_world(&mut e);
        e.call_log = Some(vec![]);
        // No idle given; the animation allows (00498f80 false) and the mode is 2.
        assert!(setup_idle(&mut e, p, actor, 0, 2, [0, 0, 0]));
        assert_eq!(
            calls_to(&e, 0x0060_0950),
            vec![vec![0x0ddd_0000, actor, 0x1111]],
            "the idle manager, the actor and the process' target"
        );
        assert_eq!(e.mem.u32(p.addr() + 0x350), 0x2222);
        // Mode 3 and a current anim action: nothing to pick.
        returning(&mut e, 0x008a_7570, 5);
        e.mem.set_u32(p.addr() + 0x350, 0);
        assert!(!setup_idle(&mut e, p, actor, 0, 3, [0, 0, 0]));
        // 008a7570 == -1 picks whatever the mode.
        returning(&mut e, 0x008a_7570, u32::MAX);
        assert!(setup_idle(&mut e, p, actor, 0, 3, [0, 0, 0]));
        // An animation that refuses (00498f80) picks nothing.
        returning(&mut e, 0x0049_8f80, 1);
        assert!(!setup_idle(&mut e, p, actor, 0, 2, [0, 0, 0]));
        // An animation that cannot start this idle (00498d30) ends it.
        returning(&mut e, 0x0049_8f80, 0);
        returning(&mut e, 0x0049_8d30, 1);
        assert!(!setup_idle(&mut e, p, actor, 0x5000, 2, [0, 0, 0]));
    }

    #[test]
    fn test_high_process_setup_special_idle_refusals() {
        let mut e = Engine::new();
        let (p, actor, _) = idle_world(&mut e);
        // Not an actor.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x1e4, 0x40), (0x100, 0)]);
        assert!(!setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]));
        // No animation.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x1e4, 0), (0x100, 1)]);
        assert!(!setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]));
        let (p, actor, _) = idle_world(&mut e);
        // The actor's state must be 0, 4 or 9.
        for (state, expected) in [(0, true), (4, true), (9, true), (3, false)] {
            give_vtable(
                &mut e,
                actor,
                ACTOR_TABLE,
                &[
                    (0x1e4, 0x40),
                    (0x100, 1),
                    (0x214, state),
                    (0x234, 0),
                    (0x230, 0),
                ],
            );
            assert_eq!(
                setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]),
                expected,
                "state {state}"
            );
        }
        // Paused: either slot refuses.
        for slot in [0x234u32, 0x230] {
            let (p, actor, _) = idle_world(&mut e);
            give_vtable(
                &mut e,
                actor,
                ACTOR_TABLE,
                &[(0x1e4, 0x40), (0x100, 1), (0x214, 0), (slot, 1)],
            );
            assert!(
                !setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]),
                "{slot:#x}"
            );
        }
        // 00437bf0 refuses only when no idle was given.
        let (p, actor, _) = idle_world(&mut e);
        returning(&mut e, 0x0043_7bf0, 1);
        assert!(!setup_idle(&mut e, p, actor, 0, 2, [0, 0, 0]));
        assert!(setup_idle(&mut e, p, actor, 0x5000, 2, [0, 0, 0]));
    }

    #[test]
    fn test_high_process_setup_special_idle_package_flag_and_waiting() {
        let mut e = Engine::new();
        let (p, actor, _) = idle_world(&mut e);
        // A package whose flags word has bit 24 refuses.
        let package = e.mem.alloc(0x40);
        returning(&mut e, 0x0093_44a0, package);
        assert!(setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]));
        e.mem.set_u32(package + 0x1c, 0x0100_0000);
        assert!(!setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]));
        returning(&mut e, 0x0093_44a0, 0);
        // A busy animation (00498f80... done-playing 004985f0 false) refuses
        // unless an idle was given, the actor is the player or `start_anyway`.
        returning(&mut e, 0x0049_85f0, 0);
        assert!(setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]));
        assert!(!setup_idle(&mut e, p, actor, 0, 2, [0, 0, 0]));
        assert!(setup_idle(&mut e, p, actor, 0, 2, [0, 1, 0]));
    }

    #[test]
    fn test_high_process_setup_special_idle_conditions() {
        let mut e = Engine::new();
        let (p, actor, _) = idle_world(&mut e);
        // check_conditions with an idle that has conditions (00436aa0): the
        // conditions are evaluated for the actor and the process' target.
        returning(&mut e, 0x0043_6aa0, 0x600);
        returning(&mut e, 0x0068_0c30, 0);
        e.call_log = Some(vec![]);
        // They fail: the idle is dropped and another is picked (mode 2).
        assert!(setup_idle(&mut e, p, actor, 0x5000, 2, [0, 0, 1]));
        assert_eq!(
            calls_to(&e, 0x0068_0c30),
            vec![vec![0x600, actor, 0x1111]],
            "conditions, actor, target"
        );
        assert_eq!(e.mem.u32(p.addr() + 0x350), 0x2222);
        // They hold: the idle stays.
        returning(&mut e, 0x0068_0c30, 1);
        assert!(setup_idle(&mut e, p, actor, 0x5000, 2, [0, 0, 1]));
        assert_eq!(e.mem.u32(p.addr() + 0x350), 0x5000);
        // The idle may not be used by this actor (008b2010).
        returning(&mut e, 0x008b_2010, 0);
        assert!(setup_idle(&mut e, p, actor, 0x5000, 2, [0, 0, 0]));
        assert_eq!(
            e.mem.u32(p.addr() + 0x350),
            0x2222,
            "another one was picked"
        );
    }

    #[test]
    fn test_high_process_setup_special_idle_for_the_player() {
        let mut e = Engine::new();
        let (p, actor, _) = idle_world(&mut e);
        global_word(&mut e, PLAYER_POINTER, actor);
        e.map(0x011f_2250, 8);
        e.call_log = Some(vec![]);
        assert!(setup_idle(&mut e, p, actor, 0x5000, 3, [1, 0, 0]));
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x70c)),
            vec![vec![p.addr(), actor]],
            "FinishSetupSpecialIdle(player)"
        );
        assert_eq!(
            e.mem.u32(p.addr() + 0x350),
            0,
            "the pending idle is cleared"
        );
        assert_eq!(e.mem.u32(p.addr() + 0x430), 3);
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x614)).is_empty());
        // Without finish_now only the clearing happens.
        e.call_log = Some(vec![]);
        assert!(setup_idle(&mut e, p, actor, 0x5000, 3, [0, 0, 0]));
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x70c)).is_empty());
        // While the object at 011f2250 reports 4, the player does nothing.
        returning(&mut e, 0x0044_ddc0, 4);
        assert!(!setup_idle(&mut e, p, actor, 0x5000, 3, [1, 0, 0]));
    }

    #[test]
    fn test_fn_008dade0_tests_bit_24() {
        let mut e = Engine::new();
        let package = e.mem.alloc(0x40);
        assert!(!e.call(0x008dade0, &args![package]).bool());
        e.mem.set_u32(package + 0x1c, 0x0100_0000);
        assert!(e.call(0x008dade0, &args![package]).bool());
        e.mem.set_u32(package + 0x1c, 0xfeff_ffff);
        assert!(!e.call(0x008dade0, &args![package]).bool());
    }

    /// Finish-setup world: an actor with an animation, a process whose
    /// weapon is drawn and which has planted an explosive.
    fn finish_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x614, 0), (0x454, 1), (0x388, 1)],
        );
        let animation = e.mem.alloc(0x40);
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x1e4, animation)]);
        returning(e, 0x0049_8f80, 0);
        returning(e, 0x005f_f160, 0x66);
        stub(e, &[0x0049_7f20, 0x0089_f580]);
        (p, actor, animation)
    }

    #[test]
    fn test_high_process_finish_setup_special_idle_plays_the_pending_idle() {
        let mut e = Engine::new();
        let (p, actor, animation) = finish_world(&mut e);
        let a = p.addr();
        e.mem.set_u32(a + 0x350, 0x5000);
        e.mem.set_u32(a + 0x430, 3);
        e.mem.set_u8(a + 0xe0, 1);
        e.call_log = Some(vec![]);
        e.call(0x008dae00, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x005f_f160), vec![vec![0x5000, 3]]);
        assert_eq!(
            calls_to(&e, 0x0049_7f20),
            vec![vec![animation, 0x5000, actor, 0x66]]
        );
        assert_eq!(e.mem.u8(a + 0xe0), 0);
        assert_eq!(e.mem.u32(a + 0x350), 0);
        assert_eq!(e.mem.u32(a + 0x430), 2);
        assert_eq!(calls_to(&e, 0x0089_f580), vec![vec![actor]]);
        // Without a planted explosive, or without the weapon drawn.
        for slots in [[(0x454, 1), (0x388, 0)], [(0x454, 0), (0x388, 1)]] {
            give_vtable(&mut e, a, PROCESS_TABLE, &[(0x614, 0), slots[0], slots[1]]);
            e.mem.set_u32(a + 0x350, 0x5000);
            e.call_log = Some(vec![]);
            e.call(0x008dae00, &args![p, actor]);
            assert!(calls_to(&e, 0x0089_f580).is_empty());
        }
    }

    #[test]
    fn test_high_process_finish_setup_special_idle_other_cases() {
        let mut e = Engine::new();
        let (p, actor, _) = finish_world(&mut e);
        let a = p.addr();
        // Nothing pending.
        e.call_log = Some(vec![]);
        e.call(0x008dae00, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0049_8f80).len(), 0);
        // An animation that is busy only queues the post-animation action.
        e.mem.set_u32(a + 0x350, 0x5000);
        returning(&mut e, 0x0049_8f80, 1);
        e.call_log = Some(vec![]);
        e.call(0x008dae00, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x614)),
            vec![vec![a, 0x10]]
        );
        assert!(calls_to(&e, 0x0049_7f20).is_empty());
        assert_eq!(e.mem.u32(a + 0x350), 0x5000, "still pending");
        // No animation.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x1e4, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008dae00, &args![p, actor]);
        assert!(calls_to(&e, 0x0049_8f80).is_empty());
    }

    #[test]
    fn test_high_process_free_up_special_idle() {
        let mut e = Engine::new();
        let p = block(&mut e);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x614, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008daef0, &args![p, 0x1234u32]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x614)),
            vec![vec![p.addr(), 0x800]]
        );
    }

    #[test]
    fn test_high_process_post_anim_free_up_special_idle() {
        let mut e = Engine::new();
        let p = block(&mut e);
        let animation = 0x0040_0000u32;
        let actor = object_with_slots(&mut e, ACTOR_TABLE, &[(0x1e4, animation), (0x214, 0)]);
        let face = object_with_slots(&mut e, OTHER_TABLE, &[(0xbc, 0)]);
        returning(&mut e, 0x008a_dcb0, face);
        stub(&mut e, &[0x0049_8910]);
        global_f32(&mut e, 0x0101_6264, 0.75);
        e.call_log = Some(vec![]);
        e.call(0x008daf20, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0049_8910), vec![vec![animation, 1, 0]]);
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, 0xbc)),
            vec![vec![face, 0.75f32.to_bits(), 1, 1, 1, 1]]
        );
        // No animation, no face data.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x1e4, 0), (0x214, 4)]);
        returning(&mut e, 0x008a_dcb0, 0);
        e.call_log = Some(vec![]);
        e.call(0x008daf20, &args![p, actor]);
        assert!(calls_to(&e, 0x0049_8910).is_empty());
        assert_eq!(calls_to(&e, 0x008a_dcb0).len(), 1);
        // Any other state does nothing.
        give_vtable(
            &mut e,
            actor,
            ACTOR_TABLE,
            &[(0x1e4, animation), (0x214, 5)],
        );
        e.call_log = Some(vec![]);
        e.call(0x008daf20, &args![p, actor]);
        assert!(calls_to(&e, 0x008a_dcb0).is_empty());
    }

    const SETTING_OFFSET: u32 = 0x1000;

    /// The world of `RandomlyPlaySpecialIdles`: everything allows a new idle;
    /// the idle timer is `timer`.
    fn random_idles_world(e: &mut Engine, timer: f64) -> (Ptr<HighProcess>, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, 0),
                (0x594, 0),
                (0x590, 0),
                (0x3e4, u32::MAX),
                (0x44, 0),
                (0x338, 0),
            ],
        );
        float_slot(e, PROCESS_TABLE, 0x334, timer);
        let animation = 0x0040_0000u32;
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[(0x1d0, 0x1000), (0x1e4, animation), (0x234, 0)],
        );
        global_word(e, PLAYER_POINTER, 0x0aaa_0000);
        returning(e, 0x0043_d450, 0x2000);
        e.register(FRAME_TIME, |_, a| Ret {
            st0: if a[0] == 0x2000 { 64.0 } else { 0.25 },
            ..Ret::default()
        });
        global_f64(e, 0x0102_40c0, 64.0);
        returning_float(e, DISTANCE_FROM_REFERENCE, 10.0);
        e.register(SETTING_VALUE, |_, a| ret(a[0] + SETTING_OFFSET));
        global_f32(e, 0x011c_d674 + SETTING_OFFSET, 50.0);
        global_f32(e, 0x011c_dd30 + SETTING_OFFSET, 12.0);
        returning(e, 0x0049_8f80, 0);
        returning(e, 0x0049_85f0, 1);
        returning(e, 0x008a_7870, 0);
        returning(e, 0x005f_36f0, 0);
        returning(e, PACKAGE_TYPE, 5);
        (p, actor)
    }

    #[test]
    fn test_high_process_randomly_play_special_idles_counts_the_timer_down() {
        let mut e = Engine::new();
        let (p, actor) = random_idles_world(&mut e, 2.0);
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x338)),
            vec![vec![p.addr(), 1.75f32.to_bits()]],
            "the timer less the frame time"
        );
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x44)).is_empty());
        assert_eq!(
            calls_to(&e, DISTANCE_FROM_REFERENCE),
            vec![vec![actor, 0x0aaa_0000, 0, 1]]
        );
        assert_eq!(
            calls_to(&e, FRAME_TIME),
            vec![vec![0x2000], vec![FRAME_TIMER]],
            "the size of the node's bound, then the frame time"
        );
    }

    #[test]
    fn test_high_process_randomly_play_special_idles_starts_an_idle() {
        let mut e = Engine::new();
        let (p, actor) = random_idles_world(&mut e, -1.0);
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x44)),
            vec![vec![p.addr(), actor, 0, 2, 1, 0, 1]]
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x338)),
            vec![vec![p.addr(), 12.0f32.to_bits()]],
            "the timer is set to the setting at 011cdd30"
        );
        // With that setting 0 the timer just counts down.
        global_f32(&mut e, 0x011c_dd30 + SETTING_OFFSET, 0.0);
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x44)).is_empty());
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x338)),
            vec![vec![p.addr(), (-1.25f32).to_bits()]]
        );
    }

    #[test]
    fn test_high_process_randomly_play_special_idles_stops_early() {
        let mut e = Engine::new();
        // No 3D node.
        let (p, actor) = random_idles_world(&mut e, 2.0);
        give_vtable(
            &mut e,
            actor,
            ACTOR_TABLE,
            &[(0x1d0, 0), (0x1e4, 0x40), (0x234, 0)],
        );
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert_eq!(call_order(&e), vec![slot_target(ACTOR_TABLE, 0x1d0)]);
        // An idle is already pending.
        let (p, actor) = random_idles_world(&mut e, 2.0);
        e.mem.set_u32(p.addr() + 0x350, 0x5000);
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert!(calls_to(&e, DISTANCE_FROM_REFERENCE).is_empty());
        // The player is too far: 50 * (size / 64) < distance.
        let (p, actor) = random_idles_world(&mut e, 2.0);
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 50.5);
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x338)).is_empty());
        // A bigger bound makes the same distance fine: 50 * 2 = 100.
        e.register(FRAME_TIME, |_, a| Ret {
            st0: if a[0] == 0x2000 { 128.0 } else { 0.25 },
            ..Ret::default()
        });
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 100.0);
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert_eq!(calls_to(&e, slot_target(PROCESS_TABLE, 0x338)).len(), 1);
    }

    #[test]
    fn test_high_process_randomly_play_special_idles_conditions() {
        // Each condition that stops the idle, one at a time.
        type Setup = fn(&mut Engine, Ptr<HighProcess>, u32);
        let stoppers: [(&str, Setup); 7] = [
            ("package refuses", |e, _, _| returning(e, 0x005f_36f0, 1)),
            ("package type 0x1a", |e, _, _| {
                returning(e, PACKAGE_TYPE, 0x1a)
            }),
            ("animation busy", |e, _, _| returning(e, 0x0049_8f80, 1)),
            ("animation not done", |e, _, _| returning(e, 0x0049_85f0, 0)),
            ("anim action", |e, p, _| {
                give_vtable(e, p.addr(), PROCESS_TABLE, &[(0x3e4, 2), (0x27c, 0x500)])
            }),
            ("eating", |e, _, _| returning(e, 0x008a_7870, 1)),
            ("paused", |e, _, a| {
                give_vtable(
                    e,
                    a,
                    ACTOR_TABLE,
                    &[(0x1d0, 0x1000), (0x1e4, 0x0040_0000), (0x234, 1)],
                )
            }),
        ];
        for (name, setup) in stoppers {
            let mut e = Engine::new();
            let (p, actor) = random_idles_world(&mut e, 2.0);
            give_vtable(
                &mut e,
                p.addr(),
                PROCESS_TABLE,
                &[
                    (0x27c, 0x500),
                    (0x594, 0),
                    (0x590, 0),
                    (0x3e4, u32::MAX),
                    (0x338, 0),
                ],
            );
            setup(&mut e, p, actor);
            e.call_log = Some(vec![]);
            e.call(0x008dafd0, &args![p, actor]);
            assert!(
                calls_to(&e, slot_target(PROCESS_TABLE, 0x338)).is_empty(),
                "{name}"
            );
        }
        // Control: with the package present nothing stops it.
        let mut e = Engine::new();
        let (p, actor) = random_idles_world(&mut e, 2.0);
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, 0x500),
                (0x594, 0),
                (0x590, 0),
                (0x3e4, u32::MAX),
                (0x338, 0),
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert_eq!(calls_to(&e, slot_target(PROCESS_TABLE, 0x338)).len(), 1);
    }

    #[test]
    fn test_high_process_randomly_play_special_idles_ends_idles_and_reads_extra_package() {
        let mut e = Engine::new();
        let (p, actor) = random_idles_world(&mut e, 2.0);
        // GetEndIdlesPlayed true with an animation that is not busy but done:
        // SetEndIdlesPlayed(0).
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, 0),
                (0x594, 1),
                (0x590, 0),
                (0x3e4, u32::MAX),
                (0x338, 0),
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x590)),
            vec![vec![p.addr(), 0]]
        );
        // A package of type 0x1c is replaced by the extra data's package.
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, 0x500),
                (0x594, 0),
                (0x590, 0),
                (0x3e4, u32::MAX),
                (0x338, 0),
            ],
        );
        e.register(PACKAGE_TYPE, |_, a| {
            ret(if a[0] == 0x500 { 0x1c } else { 5 })
        });
        returning(&mut e, 0x005d_43c0, 0x700);
        returning(&mut e, 0x0041_cb10, 0x600);
        e.call_log = Some(vec![]);
        e.call(0x008dafd0, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0041_cb10), vec![vec![0x700]]);
        assert_eq!(
            calls_to(&e, 0x005f_36f0),
            vec![vec![0x600]],
            "the new package"
        );
    }

    /// The world of `fn_008db240`: a process whose running package (type
    /// 0x17) has a pursue-timer of 1.0 at +0x80, with an actor target.
    fn step_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32, u32) {
        let package = e.mem.alloc(0x100);
        e.mem.set_f32(package + 0x80, 1.0);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x27c, package), (0x8c, 0), (0x288, 0), (0x2a4, 0)],
        );
        let target = object_with_slots(e, OTHER_TABLE, &[(0x100, 1), (0x448, 1)]);
        e.mem.set_u32(p.addr() + 0x40, target);
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x424, 0)]);
        returning(e, PACKAGE_TYPE, 0x17);
        returning(e, 0x008b_06d0, 0);
        returning(e, 0x0088_1680, 0);
        returning_float(e, 0x0064_4790, 1.0);
        returning(e, 0x0054_6a40, 0x9100);
        returning(e, 0x008d_6f30, 0x9000);
        returning(e, 0x008b_ff30, 0);
        returning(e, GET_TOPIC, 0x8000);
        returning(e, 0x009f_94d0, 0);
        e.register(SETTING_VALUE, |_, a| ret(a[0] + SETTING_OFFSET));
        global_f32(e, 0x011c_d2c4 + SETTING_OFFSET, 2.5);
        global_f64(e, 0x0101_2060, 0.0);
        global_word(e, PLAYER_POINTER, 0x0aaa_0000);
        (p, actor, target, package)
    }

    #[test]
    fn test_fn_008db240_target_that_cannot_be_attacked() {
        let mut e = Engine::new();
        let (p, actor, target, _) = step_world(&mut e);
        give_vtable(&mut e, target, OTHER_TABLE, &[(0x448, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x288)),
            vec![vec![p.addr(), actor, 1]],
            "moves on to the next procedure"
        );
        assert!(calls_to(&e, 0x0064_4790).is_empty());
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x8c)).is_empty());
    }

    #[test]
    fn test_fn_008db240_says_the_topic_and_leaves_with_minus_one() {
        let mut e = Engine::new();
        let (p, actor, _, package) = step_world(&mut e);
        // The pursue timer is 1.0: nothing happens.
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x288)).is_empty());
        // At 0 or below it says topic 0x14 (while the 008d6f30 owner is set).
        e.mem.set_f32(package + 0x80, 0.0);
        returning_float(&mut e, 0x0064_4790, 0.0);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        let shown = [
            0x008d_6f30,
            0x0054_6a40,
            0x008b_ff30,
            GET_TOPIC,
            slot_target(PROCESS_TABLE, 0x2a4),
            0x009f_94d0,
            slot_target(PROCESS_TABLE, 0x288),
        ];
        assert_eq!(
            call_order(&e)
                .into_iter()
                .filter(|a| shown.contains(a))
                .collect::<Vec<_>>(),
            vec![
                0x008d_6f30,
                0x0054_6a40,
                0x008b_ff30,
                GET_TOPIC,
                0x008b_ff30,
                slot_target(PROCESS_TABLE, 0x2a4),
                0x009f_94d0,
                slot_target(PROCESS_TABLE, 0x288),
            ]
        );
        assert_eq!(calls_to(&e, 0x008d_6f30), vec![vec![0x0aaa_0000]]);
        assert_eq!(calls_to(&e, 0x0054_6a40), vec![vec![0x9000]]);
        assert_eq!(calls_to(&e, 0x008b_ff30), vec![vec![0x9100], vec![0]]);
        assert_eq!(calls_to(&e, GET_TOPIC), vec![vec![2, 0x14]]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x2a4)),
            vec![vec![p.addr(), actor, 0x8000, 0, 0, 0, 0]]
        );
        assert_eq!(e.mem.f32(package + 0x80), 2.5, "the setting is added");
        assert_eq!(calls_to(&e, 0x009f_94d0), vec![vec![package, 1]]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x288)),
            vec![vec![p.addr(), actor, u32::MAX]]
        );
        // No topic: nothing is said and the procedure is not changed.
        returning(&mut e, GET_TOPIC, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x2a4)).is_empty());
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x288)).is_empty());
    }

    #[test]
    fn test_fn_008db240_attack_branch_for_package_types_0x22_and_0x23() {
        let mut e = Engine::new();
        let (p, actor, target, _) = step_world(&mut e);
        // The type byte is read again after the 0x17 test; make the second
        // read give 0x22 (it cannot differ in the game).
        let reads = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let counter = reads.clone();
        e.register_double(PACKAGE_TYPE, move |_, _| {
            counter.set(counter.get() + 1);
            ret(if counter.get() == 1 { 0x17 } else { 0x22 })
        });
        returning(&mut e, 0x008b_06d0, 1);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        let calls = calls_to(&e, 0x008b_06d0);
        assert_eq!(calls.len(), 1);
        assert_eq!(
            (calls[0][0], calls[0][1], calls[0][2], calls[0][4]),
            (actor, target, 0, 0)
        );
        assert_eq!(calls_to(&e, 0x0088_1680), vec![vec![actor, 0]]);
        assert_eq!(
            calls_to(&e, slot_target(ACTOR_TABLE, 0x424)),
            vec![vec![actor, target, 0, 0, 0, 0, 0, 1, 0]]
        );
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x288)).is_empty());
        // When 008b06d0 says no, the flow goes on to the pursue timer.
        returning(&mut e, 0x008b_06d0, 0);
        reads.set(0);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert!(calls_to(&e, 0x0088_1680).is_empty());
        assert_eq!(calls_to(&e, 0x0064_4790).len(), 1);
    }

    #[test]
    fn test_fn_008db240_without_a_package_of_type_0x17() {
        let mut e = Engine::new();
        let (p, actor, target, _) = step_world(&mut e);
        // The running package is of another type; the actor's process says
        // topic 0xc (0x22 gives 9) to the target and the procedure advances.
        let greeter = object_with_slots(&mut e, 0x7400_0000, &[(0x2a4, 0)]);
        returning(&mut e, ACTOR_PROCESS, greeter);
        returning(&mut e, PACKAGE_TYPE, 0x10);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert_eq!(calls_to(&e, GET_TOPIC), vec![vec![2, 0xc]]);
        assert_eq!(
            calls_to(&e, slot_target(0x7400_0000, 0x2a4)),
            vec![vec![greeter, target, 0x8000, 0, 0, 1, 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        returning(&mut e, PACKAGE_TYPE, 0x22);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert_eq!(calls_to(&e, GET_TOPIC), vec![vec![2, 9]]);
        // A target that is not an actor: nothing.
        give_vtable(&mut e, target, OTHER_TABLE, &[(0x100, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert!(calls_to(&e, GET_TOPIC).is_empty());
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x288)).is_empty());
    }

    #[test]
    fn test_fn_008db240_asks_for_a_target_when_there_is_none() {
        let mut e = Engine::new();
        let (p, actor, _, _) = step_world(&mut e);
        e.mem.set_u32(p.addr() + 0x40, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db240, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x8c)),
            vec![vec![p.addr(), actor]]
        );
    }

    #[test]
    fn test_fn_008db4c0_adds_to_the_float_at_0x80() {
        let mut e = Engine::new();
        let package = e.mem.alloc(0x100);
        e.mem.set_f32(package + 0x80, 1.5);
        e.call(0x008db4c0, &args![package, 2.25f32]);
        assert_eq!(e.mem.f32(package + 0x80), 3.75);
        assert_eq!(e.mem.f32(package + 0x84), 0.0);
    }

    /// The world of `ProcessCreateFollow`. `SetTarget` (virtual `+0x12c`) stores
    /// its argument in `pTarget`, as the game's does; the followers the
    /// package creates are `followers`, in order.
    fn follow_world(
        e: &mut Engine,
        target_type: u32,
        followers: Vec<u32>,
    ) -> (Ptr<HighProcess>, u32, u32, u32, u32) {
        let package = e.mem.alloc(0x100);
        let escort = e.mem.alloc(0x100);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, package),
                (0x274, escort),
                (0x20c, 0),
                (0x294, 0),
                (0x288, 0),
                (0xc8, 0),
            ],
        );
        let set_target = slot_target(PROCESS_TABLE, 0x12c);
        e.mem.set_u32(PROCESS_TABLE + 0x12c, set_target);
        e.register(set_target, |e, a| {
            e.mem.set_u32(a[0] + 0x40, a[1]);
            Ret::default()
        });
        let data = e.mem.alloc(0x40);
        returning(e, 0x0067_1d10, data);
        returning(e, 0x0051_9b00, target_type);
        returning(e, 0x0068_0050, 0x4040);
        returning(e, 0x005f_65d0, 0);
        returning(e, 0x0044_ddc0, 2);
        returning(e, 0x009f_0a80, 0);
        let mut queue = followers.into_iter();
        e.register_double(0x0067_80e0, move |_, _| ret(queue.next().unwrap_or(0)));
        returning(e, ACTOR_IS_PATHING_COMPLETE, 0);
        stub(e, &[0x009f_0a20]);
        returning_float(e, 0x0086_7da0, 10.0);
        returning_float(e, 0x0062_1b00, 9.9);
        returning_float(e, 0x0040_8840, 0.1);
        global_f64(e, 0x0107_0c80, 0.15);
        global_word(e, PLAYER_POINTER, 0x0aaa_0000);
        let actor = 0x00a0_0000u32;
        (p, actor, package, escort, data)
    }

    #[test]
    fn test_high_process_process_create_follow_does_nothing_without_a_package() {
        let mut e = Engine::new();
        let (p, actor, _, _, _) = follow_world(&mut e, 2, vec![]);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x27c, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008db4f0, &args![p, actor]);
        assert_eq!(call_order(&e), vec![slot_target(PROCESS_TABLE, 0x27c)]);
        // A package without data.
        let (p, actor, package, _, _) = follow_world(&mut e, 2, vec![]);
        returning(&mut e, 0x0067_1d10, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db4f0, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0067_1d10), vec![vec![package]]);
        assert!(calls_to(&e, 0x0051_9b00).is_empty());
    }

    #[test]
    fn test_high_process_process_create_follow_reference_target() {
        let mut e = Engine::new();
        // Type 0: one follower wanted; the package gives none (pTarget stays 0).
        let (p, actor, package, escort, data) = follow_world(&mut e, 0, vec![0]);
        e.call_log = Some(vec![]);
        e.call(0x008db4f0, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0067_80e0), vec![vec![package, actor, 1]]);
        assert_eq!(calls_to(&e, 0x0051_9b00), vec![vec![data]]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x294)),
            vec![vec![p.addr(), actor]],
            "EndMoveMessage while the actor is not done pathing"
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert_eq!(calls_to(&e, 0x005f_65d0)[0][0], escort + 0x10);
        assert!(
            calls_to(&e, 0x0044_ddc0).is_empty(),
            "one follower, no count"
        );
        // Pathing complete: no EndMoveMessage. The player found in the escort
        // data becomes the target first.
        let (p, actor, _, _, _) = follow_world(&mut e, 3, vec![0]);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 1);
        returning(&mut e, 0x005f_65d0, 1);
        e.call_log = Some(vec![]);
        e.call(0x008db4f0, &args![p, actor]);
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x294)).is_empty());
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x12c)),
            vec![vec![p.addr(), 0], vec![p.addr(), 0x0aaa_0000]]
        );
    }

    #[test]
    fn test_high_process_process_create_follow_creates_followers() {
        let mut e = Engine::new();
        let first = object_with_slots(&mut e, OTHER_TABLE, &[(0x100, 1)]);
        let second = object_with_slots(&mut e, 0x7400_0000, &[(0x100, 1)]);
        let (p, actor, package, escort, _) = follow_world(&mut e, 2, vec![first, second]);
        e.call_log = Some(vec![]);
        e.call(0x008db4f0, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0044_ddc0).len(), 1, "two wanted");
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x12c)),
            vec![vec![p.addr(), first], vec![p.addr(), second]]
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0xc8)),
            vec![vec![p.addr(), actor, package, 0]; 2]
        );
        assert_eq!(
            calls_to(&e, 0x009f_0a20),
            vec![vec![escort, first], vec![escort, second]]
        );
        // The package hour comes from the current package (+4) when there is
        // no run-once package; the difference goes through 00408840.
        assert_eq!(calls_to(&e, 0x0062_1b00), vec![vec![p.addr() + 4]]);
        assert_eq!(
            calls_to(&e, 0x0040_8840),
            vec![vec![((10.0f64 - 9.9f64) as f32).to_bits()]]
        );
        // 0.1 < 0.15, a target is set and followers are still wanted (2):
        // the procedure stays.
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x288)).is_empty());
        // 0.2 is past the limit.
        returning_float(&mut e, 0x0040_8840, 0.2);
        let first = object_with_slots(&mut e, OTHER_TABLE, &[(0x100, 1)]);
        let (p, actor, _, _, _) = follow_world(&mut e, 2, vec![first]);
        returning_float(&mut e, 0x0040_8840, 0.2);
        e.call_log = Some(vec![]);
        e.call(0x008db4f0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
    }

    #[test]
    fn test_high_process_process_create_follow_counts_the_player_and_escorted() {
        let mut e = Engine::new();
        let other = object_with_slots(&mut e, OTHER_TABLE, &[(0x100, 1)]);
        let player = 0x0aaa_0000u32;
        e.map(player, 0x100);
        give_vtable(&mut e, player, 0x7500_0000, &[(0x100, 1)]);
        // Type 1: the escort data already has one of this type, so one of
        // the two wanted remains. The first follower is the player (noted,
        // not recorded), the second is created.
        let (p, actor, _, escort, _) = follow_world(&mut e, 1, vec![player, other]);
        give_vtable(&mut e, player, 0x7500_0000, &[(0x100, 1)]);
        returning(&mut e, 0x009f_0a80, 1);
        e.call_log = Some(vec![]);
        e.call(0x008db4f0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, 0x009f_0a80),
            vec![vec![escort, actor, 0x4040], vec![escort, actor, 0x4040]]
        );
        assert_eq!(
            calls_to(&e, 0x0067_80e0).len(),
            1,
            "wanted 2 less 1 escorted"
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x12c)),
            vec![vec![p.addr(), player], vec![p.addr(), player]],
            "the target is set to the follower, then to the player after the loop"
        );
        assert!(calls_to(&e, 0x009f_0a20).is_empty());
    }

    const THREATS: u32 = 0x0b00_0000;
    const SUB_TABLE: u32 = 0x7600_0000;

    /// The world of `ProcessObserveCombat`: an observe package (type 0x18),
    /// no threats, the fight still on (`actor + 0xa4` virtual `+8` gives 1)
    /// and a hide request that succeeds.
    fn observe_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        let package = e.mem.alloc(0x100);
        let p = block(e);
        give_vtable(e, p.addr(), PROCESS_TABLE, &[(0x27c, package), (0x350, 0)]);
        let position = e.mem.alloc(0x10);
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[(0x410, 0), (0x1f4, position), (0x21c, 0), (0x218, 0)],
        );
        e.map(SUB_TABLE, 0x100);
        e.mem.set_u32(actor + 0xa4, SUB_TABLE);
        let live = SUB_TABLE + 0x10_0000 + 8;
        e.mem.set_u32(SUB_TABLE + 8, live);
        returning(e, live, 1);
        e.map(THREATS, 0x100);
        e.mem.set_u32(THREATS, 0x1111);
        e.mem.set_u32(THREATS + 4, 0x2222);
        returning(e, PACKAGE_TYPE, 0x18);
        returning(e, 0x008d_0300, 0);
        returning(e, 0x0044_ddc0, 0);
        e.register(0x006a_7ad0, |_, a| ret(THREATS + 4 * a[1]));
        returning(e, ACTOR_CURRENT_PACKAGE, 0x5000);
        returning(e, RT_DYNAMIC_CAST, 0);
        returning(e, 0x009f_7fe0, 0);
        e.register(0x009f_8300, |_, a| ret(a[1]));
        returning_float(e, 0x0057_2380, 30.0);
        returning(e, 0x008d_6f30, 0);
        returning(e, 0x0042_5fd0, 0);
        e.register(SETTING_VALUE, |_, a| ret(a[0] + SETTING_OFFSET));
        returning(e, ACTOR_IS_PATHING_COMPLETE, 1);
        returning(e, 0x0082_5c00, 100_000);
        returning(e, 0x004f_b070, 0);
        returning(e, 0x006d_ce10, 0x7777);
        returning_float(e, 0x0050_7e10, 5.0);
        returning(e, 0x009f_8570, 3);
        returning(e, 0x008b_b630, 1);
        returning(e, 0x0089_4d60, 0);
        returning(e, 0x009d_e2b0, 1);
        stub(
            e,
            &[
                0x008c_1c00,
                0x008c_1cb0,
                0x009f_8660,
                0x0088_1680,
                0x009f_1310,
                0x009f_8020,
                0x009f_7f20,
                0x009f_7d70,
                0x008a_c890,
                0x006e_5510,
                0x006e_29f0,
                0x0041_81e0,
                0x006e_2b50,
                0x004f_f7e0,
                0x006d_3b00,
                0x006d_61e0,
                0x0089_4cc0,
                ACTOR_SET_MOVE_MODE,
                0x006e_55e0,
                0x008b_3b90,
                0x006d_cce0,
            ],
        );
        global_word(e, 0x011f_1958, 0x0c00_0000);
        global_f64(e, 0x0102_e430, 128.0);
        global_f32(e, 0x0101_e6ec, 64.0);
        global_f64(e, 0x0102_40c0, 64.0);
        global_f32(e, 0x0101_2054, -1.0);
        (p, actor, package)
    }

    #[test]
    fn test_high_process_process_observe_combat_needs_an_observe_package() {
        let mut e = Engine::new();
        let (p, actor, package) = observe_world(&mut e);
        returning(&mut e, PACKAGE_TYPE, 0x10);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(
            call_order(&e),
            vec![slot_target(PROCESS_TABLE, 0x27c), PACKAGE_TYPE]
        );
        assert_eq!(calls_to(&e, PACKAGE_TYPE), vec![vec![package]]);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x27c, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(call_order(&e), vec![slot_target(PROCESS_TABLE, 0x27c)]);
    }

    #[test]
    fn test_high_process_process_observe_combat_avoids_the_threats_of_a_flee_package() {
        let mut e = Engine::new();
        let (p, actor, package) = observe_world(&mut e);
        // 008d0300 true builds the threat array whatever the fight does.
        returning(&mut e, 0x008d_0300, 1);
        returning(&mut e, 0x0044_ddc0, 2);
        returning(&mut e, RT_DYNAMIC_CAST, 0x6000);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x008d_0300), vec![vec![0x0c00_0000, actor, 0]]);
        let arrays = calls_to(&e, 0x008c_1c00);
        assert_eq!(arrays.len(), 1);
        let array = arrays[0][0];
        assert_eq!(calls_to(&e, 0x009f_8660), vec![vec![package, array]]);
        assert_eq!(calls_to(&e, 0x0088_1680), vec![vec![actor, 0]]);
        assert_eq!(
            calls_to(&e, slot_target(ACTOR_TABLE, 0x410)),
            vec![vec![
                actor,
                0x1111,
                1,
                1,
                1,
                0,
                0,
                (-1.0f32).to_bits(),
                (-1.0f32).to_bits()
            ]]
        );
        assert_eq!(
            calls_to(&e, RT_DYNAMIC_CAST),
            vec![vec![0x5000, 0, TYPE_PACKAGE, TYPE_FLEE_PACKAGE, 0]]
        );
        assert_eq!(
            calls_to(&e, 0x009f_1310),
            vec![vec![0x6000, 0x1111], vec![0x6000, 0x2222]]
        );
        assert_eq!(calls_to(&e, 0x008c_1cb0), vec![vec![array]]);
        assert!(calls_to(&e, 0x009f_7fe0).is_empty(), "the call ends there");
        // A package that is not a flee package: threats are not added.
        returning(&mut e, RT_DYNAMIC_CAST, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert!(calls_to(&e, 0x009f_1310).is_empty());
        assert_eq!(calls_to(&e, 0x0088_1680).len(), 1);
    }

    #[test]
    fn test_high_process_process_observe_combat_without_threats_goes_on() {
        let mut e = Engine::new();
        let (p, actor, package) = observe_world(&mut e);
        // No live fight (virtual +8 gives 0): the array is built, empty.
        let live = SUB_TABLE + 0x10_0000 + 8;
        returning(&mut e, live, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(calls_to(&e, live), vec![vec![actor + 0xa4, 1]]);
        assert_eq!(calls_to(&e, 0x008c_1cb0).len(), 1, "the array is destroyed");
        assert_eq!(calls_to(&e, 0x009f_7fe0), vec![vec![package]]);
        assert!(calls_to(&e, 0x0088_1680).is_empty());
        // 009f7fe0 true ends the interrupt package and nothing else follows.
        returning(&mut e, 0x009f_7fe0, 1);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0088_1680), vec![vec![actor, 0]]);
        assert!(calls_to(&e, 0x009f_8300).is_empty());
    }

    #[test]
    fn test_high_process_process_observe_combat_hides_from_the_fight() {
        let mut e = Engine::new();
        let (p, actor, package) = observe_world(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        let hide = calls_to(&e, 0x006e_5510)[0][0];
        let interesting = [
            0x009f_8020,
            ACTOR_IS_PATHING_COMPLETE,
            0x009f_7f20,
            0x009f_7d70,
            0x008a_c890,
            0x006e_5510,
            0x006e_29f0,
            0x006d_ce10,
            0x006d_cce0,
            0x004f_f7e0,
            0x006d_3b00,
            0x006d_61e0,
            0x008b_b630,
            0x0089_4d60,
            ACTOR_SET_MOVE_MODE,
            0x009d_e2b0,
            slot_target(PROCESS_TABLE, 0x350),
            0x006e_55e0,
            0x008b_3b90,
        ];
        assert_eq!(
            call_order(&e)
                .into_iter()
                .filter(|a| interesting.contains(a))
                .collect::<Vec<_>>(),
            vec![
                0x009f_8020,
                ACTOR_IS_PATHING_COMPLETE,
                0x009f_7f20,
                0x009f_7d70,
                0x008a_c890,
                0x006e_5510,
                0x006e_29f0,
                0x006d_ce10,
                0x006d_cce0,
                0x004f_f7e0,
                0x006d_3b00,
                0x006d_61e0,
                0x008b_b630,
                0x0089_4d60,
                ACTOR_SET_MOVE_MODE,
                0x009d_e2b0,
                slot_target(PROCESS_TABLE, 0x350),
                0x006e_55e0,
                0x008b_3b90,
            ]
        );
        assert_eq!(calls_to(&e, 0x009f_7f20), vec![vec![package, actor, 1]]);
        assert_eq!(calls_to(&e, 0x009f_7d70), vec![vec![package, actor, 1]]);
        assert_eq!(calls_to(&e, 0x008a_c890), vec![vec![package, 100_000]]);
        assert_eq!(calls_to(&e, 0x006e_29f0), vec![vec![hide, actor]]);
        assert_eq!(
            calls_to(&e, 0x006d_ce10)[0][2],
            actor,
            "the pathing location is made for the actor"
        );
        assert_eq!(
            calls_to(&e, 0x006d_cce0),
            vec![vec![hide + 0xb0, 0x7777]],
            "fn_008dbda0 assigns the location"
        );
        assert_eq!(e.mem.f32(hide + 0xd8), 133.0, "fn_008dbd80: radius 5 + 128");
        assert_eq!(
            calls_to(&e, 0x006d_3b00),
            vec![vec![hide, 64.0f32.to_bits()]]
        );
        assert_eq!(calls_to(&e, 0x006d_61e0), vec![vec![hide, 3]]);
        assert_eq!(calls_to(&e, 0x008b_b630), vec![vec![actor, hide]]);
        assert_eq!(calls_to(&e, ACTOR_SET_MOVE_MODE), vec![vec![actor, 0x200]]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x350)),
            vec![vec![p.addr(), actor, 0x101, 1]],
            "30 is within 64: walk"
        );
        assert_eq!(calls_to(&e, 0x006e_55e0), vec![vec![hide]]);
        assert_eq!(calls_to(&e, 0x008b_3b90), vec![vec![actor]]);
        // Farther than 64: run.
        returning_float(&mut e, 0x0057_2380, 80.0);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x350)),
            vec![vec![p.addr(), actor, 0x201, 1]]
        );
        // No such object (009de2b0 false): no animation call.
        returning(&mut e, 0x009d_e2b0, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x350)).is_empty());
        assert_eq!(calls_to(&e, 0x006e_55e0).len(), 1);
    }

    #[test]
    fn test_high_process_process_observe_combat_hide_variants() {
        let mut e = Engine::new();
        let (p, actor, _) = observe_world(&mut e);
        // SetPathfindingFlee fails: the request is destroyed and the call ends
        // without 008b3b90.
        returning(&mut e, 0x008b_b630, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x006e_55e0).len(), 1);
        assert!(calls_to(&e, 0x008b_3b90).is_empty());
        assert!(calls_to(&e, ACTOR_SET_MOVE_MODE).is_empty());
        // A blocked actor is unblocked first; an explosion form passes its flag.
        returning(&mut e, 0x008b_b630, 1);
        returning(&mut e, 0x0089_4d60, 1);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0089_4cc0), vec![vec![actor, 0]]);
        let form = e.mem.alloc(0x80);
        let sub = form + 0x30;
        give_vtable(&mut e, sub, 0x7700_0000, &[(0x28, 1)]);
        returning(&mut e, 0x0041_81e0, form);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x21c, 1)]);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x0041_81e0), vec![vec![actor]]);
        assert_eq!(calls_to(&e, 0x006e_2b50)[0][1], 1);
        // The time limit has not passed: no request at all.
        returning(&mut e, 0x0082_5c00, 0x5dc);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert!(calls_to(&e, 0x006e_5510).is_empty());
        assert_eq!(calls_to(&e, 0x008b_3b90).len(), 1);
        // Not done pathing: creatures re-run the package steps.
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert!(calls_to(&e, 0x009f_7f20).is_empty());
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x218, 1)]);
        e.call_log = Some(vec![]);
        e.call(0x008db810, &args![p, actor]);
        assert_eq!(calls_to(&e, 0x009f_7f20).len(), 1);
        assert_eq!(calls_to(&e, 0x009f_7d70).len(), 1);
    }

    #[test]
    fn test_fn_008dbd80_and_fn_008dbda0() {
        let mut e = Engine::new();
        let hide = e.mem.alloc(0x100);
        e.call(0x008dbd80, &args![hide, 2.5f32]);
        assert_eq!(e.mem.f32(hide + 0xd8), 2.5);
        stub(&mut e, &[0x006d_cce0]);
        e.call_log = Some(vec![]);
        e.call(0x008dbda0, &args![hide, 0x1234u32]);
        assert_eq!(calls_to(&e, 0x006d_cce0), vec![vec![hide + 0xb0, 0x1234]]);
    }

    #[test]
    fn test_high_process_clear_greeting_info_data() {
        let mut e = Engine::new();
        stub(&mut e, &[0x005c_90d0]);
        let p = block(&mut e);
        let a = p.addr();
        e.mem.set_u32(a + 0x368, 0x1000);
        e.mem.set_u32(a + 0x454, 0x2000);
        e.mem.set_u32(a + 0x3e4, 0x3000);
        e.mem.set_u32(a + 0x370, 0x4000);
        e.call_log = Some(vec![]);
        e.call(0x008dbdc0, &args![p]);
        assert_eq!(calls_to(&e, 0x005c_90d0), vec![vec![0x1000, 1]]);
        assert_eq!(e.mem.u32(a + 0x368), 0);
        assert_eq!(e.mem.u32(a + 0x454), 0);
        assert_eq!(e.mem.u32(a + 0x3e4), 0);
        assert_eq!(e.mem.u32(a + 0x370), 0x4000, "the dialog target stays");
        // Nothing to delete the second time.
        e.call_log = Some(vec![]);
        e.call(0x008dbdc0, &args![p]);
        assert!(calls_to(&e, 0x005c_90d0).is_empty());
    }

    #[test]
    fn test_fn_008dd880_sets_the_greet_flag() {
        let mut e = Engine::new();
        let player = e.mem.alloc(0x800);
        e.call(0x008dd880, &args![player]);
        assert_eq!(e.mem.u8(player + 0x6cc), 1);
        assert_eq!(e.mem.u8(player + 0x6cd), 0);
    }

    const GREET_TARGET_TABLE: u32 = 0x7800_0000;

    /// Handles of the greet world.
    struct GreetObjects {
        procedures: u32,
        process: Ptr<HighProcess>,
        actor: u32,
        player: u32,
        position: u32,
    }

    /// The world of `ProcessGreet`: nothing is happening (no topic, no lip
    /// file, no dialog target) and every callee answers 0 unless set here.
    fn greet_world(e: &mut Engine) -> GreetObjects {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x22c, 0),
                (0x20c, 0),
                (0x27c, 0),
                (0x44, 0),
                (0x628, 0),
                (0x684, 0),
                (0x688, 0),
                (0x310, 0),
                (0x294, 0),
                (0x118, 0),
                (0x288, 0),
                (0x88, 0),
                (0x280, 0),
                (0x494, 0),
                (0x644, 0),
            ],
        );
        let player = e.mem.alloc(0x800);
        global_word(e, PLAYER_POINTER, player);
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[(0x214, 0), (0x1e4, 0), (0xfc, 0), (0x100, 0)],
        );
        float_slot(e, ACTOR_TABLE, 0x284, 1.5);
        let position = e.mem.alloc(0x10);
        e.mem.set_u32(position, 0x11);
        e.mem.set_u32(position + 4, 0x22);
        e.mem.set_u32(position + 8, 0x33);
        let holder = e.mem.alloc(0x10);
        e.mem.set_u32(holder, 0xabcd);
        list_and_pointer_doubles(e);
        for (address, value) in [
            (0x0049_3bb0, 0),
            (0x005f_36f0, 0),
            (0x004f_d380, 0),
            (PACKAGE_TYPE, 5),
            (0x0044_0da0, 0),
            (ACTOR_PROCESS, 0),
            (0x0083_c820, 0),
            (0x0083_c7e0, 0),
            (0x007a_f430, 0),
            (0x0096_11e0, 0),
            (0x0044_1110, 0),
            (0x0093_3150, 0x0e00),
            (0x0084_e3a0, 0x1234),
            (0x0054_3c30, 0),
            (0x0040_48e0, 0x4048),
            (0x0044_ddc0, 3),
            (0x0046_0140, holder),
            (0x005d_43c0, 0x700),
            (0x0067_2710, 0),
            (0x0061_b320, 0),
            (0x008a_40e0, 0),
            (0x008a_3b30, 0),
            (0x008b_3bd0, 0),
            (0x00ad_8930, 0),
            (0x0044_edb0, 0x55),
            (0x008a_2ed0, 0),
            (0x008d_d860, 7),
        ] {
            returning(e, address, value);
        }
        stub(
            e,
            &[
                0x0083_c7b0,
                0x0083_c850,
                0x0041_8900,
                SOUND_HANDLE_DESTRUCTOR,
                0x00ad_8e60,
                0x00ad_9060,
                0x0057_ad20,
                0x0057_ace0,
                0x0057_ad60,
                0x008a_5cf0,
                0x005a_c750,
                0x0093_4250,
                0x005c_90d0,
                0x0042_edb0,
                0x0087_faa0,
                0x008b_b520,
                0x0095_3ce0,
                0x0057_bd60,
            ],
        );
        e.register(SETTING_VALUE, |_, a| ret(a[0] + SETTING_OFFSET));
        global_f32(e, GREET_TIMER_SETTING + SETTING_OFFSET, 5.0);
        e.register(FRAME_TIME, |_, _| Ret {
            st0: 0.25,
            ..Ret::default()
        });
        global_f64(e, 0x0101_2060, 0.0);
        // The procedure table of package kind 0: procedure 0 maps to 0.
        let procedures = e.mem.alloc(0x40);
        e.map(0x011a_3ff0, 0x40);
        e.mem.set_u32(0x011a_3ff0, procedures);
        GreetObjects {
            procedures,
            process: p,
            actor,
            player,
            position,
        }
    }

    /// Calls `ProcessGreet(actor, topic, 0, stop, reuse, say_to)`.
    fn greet(e: &mut Engine, w: &GreetObjects, topic: u32, flags: [u32; 3]) {
        e.call(
            0x008dbe30,
            &args![w.process, w.actor, topic, 0u32, flags[0], flags[1], flags[2]],
        );
    }

    /// A dialog target: an actor with a process `greeter` that has the slots
    /// `ProcessGreet` calls.
    fn dialog_target(e: &mut Engine, w: &GreetObjects) -> (u32, u32) {
        let target = object_with_slots(
            e,
            GREET_TARGET_TABLE,
            &[(0xfc, 0), (0x100, 1), (0x1f4, w.position)],
        );
        let greeter = object_with_slots(e, 0x7900_0000, &[(0x488, 0), (0x644, 0), (0x44, 0)]);
        returning(e, 0x004f_d380, target);
        e.register_double(ACTOR_PROCESS, move |_, a| {
            ret(if a[0] == target { greeter } else { 0 })
        });
        (target, greeter)
    }

    #[test]
    fn test_high_process_process_greet_stops_while_waiting_for_the_lip_file() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        e.mem.set_u8(a + 0x3d0, 1);
        let (target, _) = dialog_target(&mut e, &w);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [1, 0, 1]);
        assert_eq!(e.mem.u8(a + 0x3a0), 1, "bStop");
        assert_eq!(e.mem.u8(a + 0x459), 1, "bIsDoingSayTo");
        assert_eq!(e.mem.u8(a + 0x29d), 1, "bProcessGreetSayTo");
        assert_eq!(e.mem.u8(w.player + 0x6cc), 1, "the player's greet flag");
        assert_eq!(calls_to(&e, 0x0049_3bb0), vec![vec![w.actor]]);
        assert!(calls_to(&e, 0x0061_b320).is_empty());
        assert!(
            calls_to(&e, 0x0057_bd60).is_empty(),
            "left before the last call"
        );
        assert_ne!(target, 0);
    }

    #[test]
    fn test_high_process_process_greet_counts_the_delay_down() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        e.mem.set_f32(a + 0x310, 2.0);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert_eq!(e.mem.f32(a + 0x310), 1.75);
        assert_eq!(e.mem.u8(a + 0x3a0), 0);
        assert_eq!(
            call_order(&e).last().copied(),
            Some(0x0057_bd60),
            "ends with 0057bd60(actor, 0)"
        );
        assert_eq!(calls_to(&e, 0x0057_bd60), vec![vec![w.actor, 0]]);
        assert_eq!(e.mem.u8(w.player + 0x6cc), 0, "no dialog target");
        // The dialog target of the process is used when the actor has none.
        e.mem.set_u32(a + 0x370, 0x0abc_0000);
        e.mem.set_f32(a + 0x310, 2.0);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert_eq!(e.mem.u8(w.player + 0x6cc), 1);
    }

    #[test]
    fn test_high_process_process_greet_turns_towards_the_dialog_target() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        let (target, _) = dialog_target(&mut e, &w);
        e.mem.set_f32(a + 0x310, 2.0);
        returning(&mut e, 0x008a_40e0, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [1, 0, 0]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x628)),
            vec![vec![a, target]]
        );
        assert_eq!(
            calls_to(&e, slot_target(GREET_TARGET_TABLE, 0x1f4)),
            vec![vec![target, 0]]
        );
        assert_eq!(
            calls_to(&e, 0x008b_b520),
            vec![vec![w.actor, 0x11, 0x22, 0x33]],
            "the target's position by value"
        );
        assert_eq!(
            calls_to(&e, 0x008a_40e0),
            vec![vec![w.actor, 0], vec![w.actor, 0]]
        );
        // Without `stop` nothing turns, unless the process is forced to rotate.
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert!(calls_to(&e, 0x008b_b520).is_empty());
        give_vtable(&mut e, a, PROCESS_TABLE, &[(0x688, 1)]);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert_eq!(calls_to(&e, 0x008b_b520).len(), 1);
        give_vtable(&mut e, a, PROCESS_TABLE, &[(0x688, 0)]);
        // Each of these stops the turning.
        for (name, stopper) in [
            ("8a40e0", (0x008a_40e0u32, 0u32)),
            ("creature", (0x0049_3bb0, 1)),
            ("8a3b30", (0x008a_3b30, 1)),
            ("pathing", (0x008b_3bd0, 1)),
        ] {
            returning(&mut e, stopper.0, stopper.1);
            e.call_log = Some(vec![]);
            greet(&mut e, &w, 0, [1, 0, 0]);
            assert!(calls_to(&e, 0x008b_b520).is_empty(), "{name}");
            returning(&mut e, 0x008a_40e0, 1);
            returning(&mut e, 0x0049_3bb0, 0);
            returning(&mut e, 0x008a_3b30, 0);
            returning(&mut e, 0x008b_3bd0, 0);
        }
        give_vtable(&mut e, w.actor, ACTOR_TABLE, &[(0x214, 3)]);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [1, 0, 0]);
        assert!(calls_to(&e, 0x008b_b520).is_empty(), "actor state 3");
        give_vtable(&mut e, w.actor, ACTOR_TABLE, &[(0x214, 0)]);
        // Package types that stop it.
        give_vtable(&mut e, a, PROCESS_TABLE, &[(0x27c, 0x500)]);
        for (kind, expected) in [
            (6, false),
            (2, false),
            (1, false),
            (7, false),
            (0xd, false),
            (8, false),
            (0x10, false),
            (3, false),
            (0xc, false),
            (4, true),
            (0xf, true),
        ] {
            returning(&mut e, PACKAGE_TYPE, kind);
            e.call_log = Some(vec![]);
            greet(&mut e, &w, 0, [1, 0, 0]);
            assert_eq!(
                !calls_to(&e, 0x008b_b520).is_empty(),
                expected,
                "package type {kind}"
            );
        }
    }

    #[test]
    fn test_high_process_process_greet_finished_greeting_is_cleared() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        let (target, greeter) = dialog_target(&mut e, &w);
        // The talker 008a2ed0 finds is an actor with a process.
        let talker = object_with_slots(&mut e, 0x7a00_0000, &[(0x100, 1)]);
        returning(&mut e, 0x008a_2ed0, talker);
        let talker_process = object_with_slots(&mut e, 0x7b00_0000, &[(0x488, 0), (0x644, 0)]);
        e.register_double(ACTOR_PROCESS, move |_, x| {
            ret(if x[0] == talker {
                talker_process
            } else if x[0] == target {
                greeter
            } else {
                0
            })
        });
        // A type-0xf package of the player's conversation.
        give_vtable(&mut e, a, PROCESS_TABLE, &[(0x27c, 0x500)]);
        returning(&mut e, PACKAGE_TYPE, 0xf);
        returning(&mut e, 0x0067_2710, 1);
        e.mem.set_u32(a + 0x40, w.player);
        e.mem.set_u32(a + 0x368, 0x6000);
        e.mem.set_u32(a + 0x454, 0x6000);
        e.mem.set_u32(a + 0x370, 0x1);
        e.mem.set_u8(a + 0x32c, 1);
        e.mem.set_f32(a + 0x310, 0.0);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [1, 0, 0]);
        assert_eq!(e.mem.u8(a + 0x3a0), 0, "bStop is cleared");
        assert_eq!(e.mem.u32(a + 0x3e4), 0);
        assert_eq!(calls_to(&e, 0x005c_90d0), vec![vec![0x6000, 1]]);
        assert_eq!(e.mem.u32(a + 0x368), 0);
        assert_eq!(
            e.mem.u32(a + 0x454),
            0x6000,
            "this path leaves pSubtitleVoice"
        );
        assert_eq!(e.mem.u32(a + 0x370), 0);
        assert_eq!(e.mem.u8(a + 0x32c), 0);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x494)),
            vec![vec![a, 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x288)),
            vec![vec![a, w.actor, 3]]
        );
        assert_eq!(calls_to(&e, 0x0095_3ce0), vec![vec![w.player]]);
        assert_eq!(
            calls_to(&e, slot_target(0x7b00_0000, 0x488)),
            vec![vec![talker_process, 0]]
        );
        assert_eq!(
            calls_to(&e, slot_target(0x7b00_0000, 0x644)),
            vec![vec![talker_process, 1]]
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x644)),
            vec![vec![a, 1]]
        );
        assert_eq!(call_order(&e).last().copied(), Some(0x0057_bd60));
        // The voice is still playing: nothing is cleared.
        e.mem.set_u32(a + 0x368, 0x6000);
        returning(&mut e, 0x00ad_8930, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [1, 0, 0]);
        assert_eq!(e.mem.u32(a + 0x368), 0x6000);
        assert!(calls_to(&e, 0x0095_3ce0).is_empty());
    }

    #[test]
    fn test_high_process_process_greet_starts_a_new_greeting() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        let (target, greeter) = dialog_target(&mut e, &w);
        let (item, response, form) = (0x0c10_0000u32, 0x0c20_0000u32, 0x0c30_0000u32);
        returning(&mut e, 0x0061_b320, item);
        returning(&mut e, 0x0083_c820, response);
        returning(&mut e, 0x007a_f430, form);
        returning(&mut e, 0x0096_11e0, 9);
        returning(&mut e, 0x0044_1110, 0x31);
        give_vtable(&mut e, w.actor, ACTOR_TABLE, &[(0x1e4, 0x40)]);
        e.mem.set_u8(w.actor + 0x7f, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [0, 0, 0]);
        assert_eq!(
            calls_to(&e, 0x0061_b320),
            vec![vec![0x7000, w.actor, target, 0, 0, 1]]
        );
        assert_eq!(e.mem.u32(a + 0x3e4), item);
        assert_eq!(e.mem.u32(a + 0x368), item);
        assert_eq!(e.mem.u32(a + 0x454), item);
        assert_eq!(e.mem.u32(a + 0x370), target);
        assert_eq!(
            calls_to(&e, slot_target(0x7900_0000, 0x488)),
            vec![vec![greeter, w.actor]],
            "the target is told who greeted it"
        );
        assert_eq!(calls_to(&e, 0x0083_c7b0), vec![vec![item]]);
        assert_eq!(calls_to(&e, 0x0083_c820), vec![vec![item]]);
        // The idles: the actor's with the response's type word, then the target's.
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x44)),
            vec![vec![a, w.actor, 9, 3, 0, 1, 1]]
        );
        assert_eq!(
            calls_to(&e, slot_target(0x7900_0000, 0x44)),
            vec![vec![greeter, target, 0x31, 3, 0, 1, 1]]
        );
        // The voice.
        assert_eq!(
            calls_to(&e, 0x0093_3150)[0][0..1],
            [w.actor],
            "the actor starts the voice file"
        );
        let started = calls_to(&e, 0x0093_3150)[0].clone();
        assert_eq!(started[2..], [0x1234, 0, 0x4000_0102, 1]);
        assert_eq!(calls_to(&e, 0x0041_8900), vec![vec![a + 0x314, 0x0e00]]);
        assert_eq!(
            calls_to(&e, SOUND_HANDLE_DESTRUCTOR),
            vec![vec![started[1]]]
        );
        assert_eq!(
            calls_to(&e, 0x00ad_8e60),
            vec![vec![a + 0x314, SOUND_CALLBACK_PLAIN, 0x1234]]
        );
        assert_eq!(e.mem.u8(w.actor + 0x7f), 0, "the voice file is taken");
        assert!(
            calls_to(&e, 0x0057_ad20).is_empty(),
            "not a say-to greeting"
        );
        assert!(calls_to(&e, slot_target(ACTOR_TABLE, 0x284)).is_empty());
        // The end: the held topic is run, the timer reset, the target looked at.
        assert_eq!(calls_to(&e, 0x0083_c850), vec![vec![item, 0]]);
        assert_eq!(e.mem.f32(a + 0x330), 5.0);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x628)),
            vec![vec![a, target]]
        );
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x684)).is_empty());
        assert_eq!(calls_to(&e, 0x0057_bd60), vec![vec![w.actor, 0]]);
    }

    #[test]
    fn test_high_process_process_greet_say_to_greeting_marks_the_actor() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        let (item, response, form) = (0x0c10_0000u32, 0x0c20_0000u32, 0x0c30_0000u32);
        returning(&mut e, 0x0061_b320, item);
        returning(&mut e, 0x0083_c820, response);
        returning(&mut e, 0x007a_f430, form);
        e.mem.set_u8(w.actor + 0x7f, 1);
        returning(&mut e, 0x008a_40e0, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [1, 0, 1]);
        assert_eq!(calls_to(&e, 0x0057_ad20), vec![vec![w.actor, 0x7000]]);
        assert_eq!(calls_to(&e, 0x0057_ace0), vec![vec![w.actor, 0x1234]]);
        assert_eq!(calls_to(&e, 0x0057_ad60), vec![vec![w.actor, 1]]);
        assert_eq!(
            calls_to(&e, 0x00ad_8e60),
            vec![vec![a + 0x314, SOUND_CALLBACK_SAY_TO, 0x1234]]
        );
        assert_eq!(e.mem.u8(a + 0x29d), 0, "bProcessGreetSayTo is cleared");
        // `stop` with an actor that is not a creature: EndMoveMessage.
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x294)),
            vec![vec![a, w.actor]]
        );
        // No dialog target: the force-rotate test needs one.
        assert!(calls_to(&e, slot_target(PROCESS_TABLE, 0x684)).is_empty());
    }

    #[test]
    fn test_high_process_process_greet_says_a_response_without_a_form() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        let (target, _) = dialog_target(&mut e, &w);
        let (item, response) = (0x0c10_0000u32, 0x0c20_0000u32);
        returning(&mut e, 0x0061_b320, item);
        returning(&mut e, 0x0083_c820, response);
        returning(&mut e, 0x0096_11e0, 9);
        returning(&mut e, 0x0044_1110, 0x31);
        returning(&mut e, 0x0054_3c30, 1);
        e.mem.set_u8(w.actor + 0x7f, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [0, 0, 0]);
        assert_eq!(calls_to(&e, 0x0054_3c30), vec![vec![response]]);
        assert_eq!(calls_to(&e, 0x008a_5cf0), vec![vec![w.actor, 1]]);
        assert_eq!(
            calls_to(&e, slot_target(ACTOR_TABLE, 0x284)),
            vec![vec![
                w.actor, 0xabcd, 3, 0x1234, 0x4048, 9, 0x31, target, 1, 0, 1, 1, 1
            ]],
            "held pointer, count, name, 004048e0, type, idle, dialog target, 1, 0, 1, 1, can-greet"
        );
        assert_eq!(calls_to(&e, 0x0046_0140), vec![vec![response, a + 0x314]]);
        assert_eq!(e.mem.f32(a + 0x310), 1.5, "the delay the say call returns");
        assert_eq!(e.mem.u8(a + 0x32c), 1);
        assert!(
            calls_to(&e, 0x0093_3150).is_empty(),
            "no voice file started"
        );
        // A script package (005f36f0) makes can-greet 0.
        give_vtable(&mut e, a, PROCESS_TABLE, &[(0x22c, 0x500)]);
        returning(&mut e, 0x005f_36f0, 1);
        e.mem.set_u8(w.actor + 0x7f, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [0, 0, 0]);
        let say = calls_to(&e, slot_target(ACTOR_TABLE, 0x284));
        assert_eq!(say[0][12], 0);
    }

    #[test]
    fn test_high_process_process_greet_without_an_item_asks_nothing() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        // 0061b320 gives no item; the package is a type-0xf one whose say-to
        // flag holds.
        give_vtable(&mut e, a, PROCESS_TABLE, &[(0x22c, 0x500)]);
        returning(&mut e, PACKAGE_TYPE, 0xf);
        returning(&mut e, 0x0067_2710, 1);
        e.mem.set_f32(a + 0x330, 9.0);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [0, 0, 1]);
        assert_eq!(
            calls_to(&e, 0x005a_c750),
            vec![vec![0x7000, 0x700, 0x40000]],
            "the action flag of the extra data"
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x118)),
            vec![vec![a, 1]]
        );
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x288)),
            vec![vec![a, w.actor, 3]]
        );
        assert_eq!(e.mem.f32(a + 0x330), 5.0, "the greeting timer setting");
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x310)),
            vec![vec![a, 0]],
            "SetGreetingFlag(false)"
        );
        assert!(calls_to(&e, 0x0083_c820).is_empty());
    }

    #[test]
    fn test_high_process_process_greet_replaces_the_held_topic() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        let (item, response) = (0x0c10_0000u32, 0x0c20_0000u32);
        returning(&mut e, 0x0061_b320, item);
        returning(&mut e, 0x0083_c820, response);
        e.mem.set_u32(a + 0x368, 0x6000);
        e.mem.set_u32(a + 0x454, 0x6000);
        e.mem.set_f32(a + 0x310, 2.0);
        e.call_log = Some(vec![]);
        // `reuse` is 0, so the held greeting is dropped and a new item made.
        greet(&mut e, &w, 0x7000, [1, 0, 0]);
        assert_eq!(calls_to(&e, 0x0093_4250), vec![vec![w.actor]]);
        assert_eq!(calls_to(&e, 0x005c_90d0), vec![vec![0x6000, 1]]);
        assert_eq!(
            calls_to(&e, 0x0061_b320),
            vec![vec![0x7000, w.actor, 0, 0, 0, 1]]
        );
        assert_eq!(e.mem.u32(a + 0x368), item);
        assert_eq!(e.mem.u32(a + 0x454), item);
        assert_eq!(calls_to(&e, 0x0083_c850), vec![vec![item, 0]]);
        // The response: marked, can-greet idle, the delay counted down.
        assert_eq!(calls_to(&e, 0x0054_3c30), vec![vec![response]]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x44)),
            vec![vec![a, w.actor, 0, 2, 1, 0, 1]]
        );
        assert_eq!(e.mem.u8(a + 0x32c), 1);
        assert_eq!(e.mem.f32(a + 0x310), 1.75, "2.0 less the frame time");
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x294)),
            vec![vec![a, w.actor]],
            "stop and not a creature"
        );
        // With `reuse` set and no lip file, the greeting is created anew
        // because the held topic is only kept when the lip data waits.
        e.mem.set_u32(a + 0x368, 0x6000);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [0, 1, 0]);
        assert!(calls_to(&e, 0x0093_4250).is_empty());
        assert_eq!(calls_to(&e, 0x005c_90d0), vec![vec![0x6000, 1]]);
        // With the lip file failed (+0x3d1) the held topic is kept.
        e.mem.set_u32(a + 0x368, 0x6000);
        e.mem.set_u8(a + 0x3d1, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [0, 1, 0]);
        assert_eq!(calls_to(&e, 0x0093_4250).len(), 1);
        assert!(calls_to(&e, 0x0061_b320).is_empty());
        assert_eq!(e.mem.u32(a + 0x3e4), 0x6000);
        assert_eq!(calls_to(&e, 0x0083_c850), vec![vec![0x6000, 0]]);
    }

    #[test]
    fn test_high_process_process_greet_held_topic_without_an_item_ends_the_say_to() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        e.mem.set_u32(a + 0x368, 0x6000);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0x7000, [0, 0, 1]);
        // No item was made: the say-to bookkeeping runs twice (the no-response
        // block, then the empty-item block).
        assert_eq!(calls_to(&e, 0x005a_c750).len(), 2);
        assert_eq!(calls_to(&e, 0x0057_ad20), vec![vec![w.actor, 0]]);
        assert_eq!(calls_to(&e, 0x0042_edb0), vec![vec![0x700]]);
        assert_eq!(
            calls_to(&e, slot_target(PROCESS_TABLE, 0x88)),
            vec![vec![a, 0]],
            "SetDoingSayTo(false)"
        );
    }

    #[test]
    fn test_high_process_process_greet_moves_to_the_next_response() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        let (item, response) = (0x0c10_0000u32, 0x0c20_0000u32);
        e.mem.set_u32(a + 0x3e4, item);
        e.mem.set_f32(a + 0x310, 0.0);
        returning(&mut e, 0x0083_c7e0, 1);
        returning(&mut e, 0x0083_c820, response);
        e.mem.set_u8(w.actor + 0x7f, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert_eq!(calls_to(&e, 0x0083_c7e0), vec![vec![item]]);
        assert_eq!(e.mem.u32(a + 0x368), item);
        assert_eq!(e.mem.u32(a + 0x454), item);
        assert_eq!(
            e.mem.u8(a + 0x29d),
            0,
            "set to 1, then cleared by the say-to block"
        );
        // The topic word is the item's procedure index (0044edb0).
        assert_eq!(calls_to(&e, 0x0044_edb0), vec![vec![item]]);
        // The response is said: the sound callback, the topic, the info, the
        // counter mark (008dd860 + 1).
        assert_eq!(calls_to(&e, 0x0057_ad20), vec![vec![w.actor, 0x55]]);
        assert_eq!(calls_to(&e, 0x0057_ace0), vec![vec![w.actor, 0x1234]]);
        assert_eq!(calls_to(&e, 0x0057_ad60), vec![vec![w.actor, 8]]);
        assert_eq!(
            calls_to(&e, 0x00ad_8e60),
            vec![vec![a + 0x314, SOUND_CALLBACK_SAY_TO, 0x1234]]
        );
        assert_eq!(e.mem.u8(w.actor + 0x80), 0);
        assert_eq!(e.mem.f32(a + 0x310), 1.5);
        assert_eq!(calls_to(&e, 0x0083_c850), vec![vec![item, 0]]);
        assert_eq!(calls_to(&e, 0x0057_bd60), vec![vec![w.actor, 0]]);
        // No further response ends the greeting (the H3 branch).
        returning(&mut e, 0x0083_c7e0, 0);
        e.mem.set_f32(a + 0x310, 0.0);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert_eq!(e.mem.u32(a + 0x3e4), 0);
        assert_eq!(calls_to(&e, 0x0095_3ce0), vec![vec![w.player]]);
        // A response that cannot be fetched ends quietly.
        e.mem.set_u32(a + 0x3e4, item);
        e.mem.set_f32(a + 0x310, 0.0);
        returning(&mut e, 0x0083_c7e0, 1);
        returning(&mut e, 0x0083_c820, 0);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert_eq!(e.mem.u32(a + 0x3e4), item);
        assert!(calls_to(&e, 0x0095_3ce0).is_empty());
        assert_eq!(calls_to(&e, 0x0057_bd60).len(), 1);
    }

    #[test]
    fn test_high_process_process_greet_ends_the_movement_of_a_standing_procedure() {
        let mut e = Engine::new();
        let w = greet_world(&mut e);
        let a = w.process.addr();
        dialog_target(&mut e, &w);
        e.mem.set_f32(a + 0x310, 2.0);
        // A package (type 5) whose procedure 0 maps to 1 in the table of its kind.
        give_vtable(&mut e, a, PROCESS_TABLE, &[(0x27c, 0x500), (0x280, 0)]);
        e.mem.set_u32(w.procedures, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert_eq!(calls_to(&e, 0x0087_faa0), vec![vec![w.actor]]);
        assert_eq!(calls_to(&e, 0x0096_11e0), vec![vec![0x500]]);
        // Not for a creature, and not for another procedure.
        returning(&mut e, 0x0049_3bb0, 1);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert!(calls_to(&e, 0x0087_faa0).is_empty());
        returning(&mut e, 0x0049_3bb0, 0);
        e.mem.set_u32(w.procedures, 2);
        e.call_log = Some(vec![]);
        greet(&mut e, &w, 0, [0, 0, 0]);
        assert!(calls_to(&e, 0x0087_faa0).is_empty());
    }

    // ---- Second session: `008dd8e0` onwards ----

    /// The target of the virtual slot `slot` of the process table.
    fn process_slot(slot: u32) -> u32 {
        slot_target(PROCESS_TABLE, slot)
    }

    /// The target of the virtual slot `slot` of the actor table.
    fn actor_slot(slot: u32) -> u32 {
        slot_target(ACTOR_TABLE, slot)
    }

    #[test]
    fn test_high_process_process_alert_behavior() {
        let mut e = Engine::new();
        let p = block(&mut e);
        let actor = object_with_slots(&mut e, ACTOR_TABLE, &[(0x1e4, 0)]);
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x27c, 0), (0x8c, 0), (0x288, 0)],
        );
        stub(&mut e, &[ACTOR_SET_ALERT, FIND_SPECIAL_IDLE_TO_PLAY]);
        returning(&mut e, ANIMATION_SPECIAL_IDLE_DONE_PLAYING, 0);
        // Without a running package nothing else happens.
        e.call_log = Some(vec![]);
        e.call(0x008dd8e0, &args![p, actor]);
        assert_eq!(call_order(&e), vec![process_slot(0x27c)]);
        // With one: alert, a target for the package (none yet), no idle
        // (no animation), procedure 1.
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x27c, 0x5000)]);
        e.call_log = Some(vec![]);
        e.call(0x008dd8e0, &args![p, actor]);
        assert_eq!(
            call_order(&e),
            vec![
                process_slot(0x27c),
                ACTOR_SET_ALERT,
                process_slot(0x8c),
                actor_slot(0x1e4),
                process_slot(0x288)
            ]
        );
        assert_eq!(calls_to(&e, ACTOR_SET_ALERT), vec![vec![actor, 1]]);
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        // With a target and an animation that finished its idle.
        e.mem.set_u32(p.addr() + 0x40, 0x6000);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x1e4, 0x7000)]);
        returning(&mut e, ANIMATION_SPECIAL_IDLE_DONE_PLAYING, 1);
        e.call_log = Some(vec![]);
        e.call(0x008dd8e0, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x8c)).is_empty());
        assert_eq!(
            calls_to(&e, ANIMATION_SPECIAL_IDLE_DONE_PLAYING),
            vec![vec![0x7000]]
        );
        assert_eq!(
            calls_to(&e, FIND_SPECIAL_IDLE_TO_PLAY),
            vec![vec![p.addr(), actor, 0, 0]]
        );
    }

    /// The world of `ProcessSearchforTarget`: a search package (type 0x1e),
    /// a target set, pathing complete and a goal that can be set.
    fn search_world(e: &mut Engine) -> (Ptr<HighProcess>, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x27c, 0x5000), (0x8c, 0), (0x350, 0), (0x118, 0)],
        );
        e.mem.set_u32(p.addr() + 0x40, 0x6000);
        e.mem.set_f32(p.addr() + 0x2e0, 1.0);
        let actor = e.mem.alloc(0x100);
        returning(e, PACKAGE_TYPE, 0x1e);
        stub(e, &[ACTOR_SET_ALERT]);
        returning_float(e, FRAME_TIME, 0.5);
        returning(e, ACTOR_IS_PATHING_COMPLETE, 1);
        e.register(SEARCH_CALCULATE_LOCATION, |e, a| {
            e.mem.set_f32(a[1], 1.0);
            e.mem.set_f32(a[1] + 4, 2.0);
            e.mem.set_f32(a[1] + 8, 3.0);
            Ret::default()
        });
        returning(e, REFERENCE_GET_WORLD_SPACE, 0x111);
        returning(e, REFERENCE_PATHING_CELL, 0x222);
        returning(e, ACTOR_SET_PATHFINDING_GOAL, 1);
        (p, actor)
    }

    #[test]
    fn test_high_process_process_searchfor_target() {
        let mut e = Engine::new();
        let (p, actor) = search_world(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008dd980, &args![p, actor]);
        // The target is set, so the process is not asked for one; the timer
        // grows by the frame time; the animation is set.
        assert!(calls_to(&e, process_slot(0x8c)).is_empty());
        assert_eq!(e.mem.f32(p.addr() + 0x2e0), 1.5);
        assert_eq!(
            calls_to(&e, process_slot(0x350)),
            vec![vec![p.addr(), actor, 0x201, 1]]
        );
        assert_eq!(calls_to(&e, process_slot(0x118)), vec![vec![p.addr(), 1]]);
        // The search package examines the location for the actor.
        let search = calls_to(&e, SEARCH_CALCULATE_LOCATION);
        assert_eq!(search.len(), 1);
        assert_eq!((search[0][0], search[0][2]), (0x5000, actor));
        // The goal gets the location, the cell, the world space, 0.0 and 0.
        let goal = calls_to(&e, ACTOR_SET_PATHFINDING_GOAL);
        assert_eq!(goal.len(), 1);
        assert_eq!(
            (goal[0][0], goal[0][1], goal[0][2], goal[0][3], goal[0][4]),
            (actor, search[0][1], 0x222, 0x111, 0)
        );
        assert_eq!(goal[0][5], 0);
        // The location is stored in the three words at +0xfc.
        assert_eq!(e.mem.f32(p.addr() + 0xfc), 1.0);
        assert_eq!(e.mem.f32(p.addr() + 0x100), 2.0);
        assert_eq!(e.mem.f32(p.addr() + 0x104), 3.0);
    }

    #[test]
    fn test_high_process_process_searchfor_target_other_cases() {
        let mut e = Engine::new();
        let (p, actor) = search_world(&mut e);
        // A goal that cannot be set leaves the words alone.
        returning(&mut e, ACTOR_SET_PATHFINDING_GOAL, 0);
        e.call(0x008dd980, &args![p, actor]);
        assert_eq!(e.mem.u32(p.addr() + 0xfc), 0);
        // Pathing not complete: no location.
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        e.call_log = Some(vec![]);
        e.call(0x008dd980, &args![p, actor]);
        assert!(calls_to(&e, SEARCH_CALCULATE_LOCATION).is_empty());
        // Another kind of package: the search package stays null, and the
        // process is asked for a target when it has none.
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 1);
        returning(&mut e, PACKAGE_TYPE, 0x10);
        e.mem.set_u32(p.addr() + 0x40, 0);
        e.call_log = Some(vec![]);
        e.call(0x008dd980, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x8c)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(calls_to(&e, SEARCH_CALCULATE_LOCATION)[0][0], 0);
        // No package at all: nothing.
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x27c, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008dd980, &args![p, actor]);
        assert_eq!(call_order(&e), vec![process_slot(0x27c)]);
    }

    const FLEE_PACKAGE: u32 = 0x5000;
    const FLEE_TARGET: u32 = 0x6000;
    const FLEE_LOCATION: u32 = 0x6100;
    const FLEE_PACKAGE_TARGET: u32 = 0x6200;

    /// The world of `ProcessFleeNonCombat`: a process with a package, a
    /// package target, no target yet and no location.
    fn flee_world(e: &mut Engine) -> (Ptr<HighProcess>, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, FLEE_PACKAGE),
                (0x8c, 0),
                (0x90, 0),
                (0x12c, 0),
                (0x128, 0),
                (0x514, 0),
                (0x288, 0),
                (0x294, 0),
                (0x118, 0),
                (0x44, 0),
                (0x24, 0),
            ],
        );
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x358, 0), (0x410, 0), (0x1e4, 0)]);
        returning(e, PACKAGE_TARGET_WORD, FLEE_PACKAGE_TARGET);
        returning(e, PACKAGE_TARGET_GET_REFERENCE, 0x7000);
        returning(e, WORD_AT_8, 50);
        returning(e, PACKAGE_DISTANCE_FOR_ACTOR, 80);
        returning(e, PACKAGE_TYPE, 0x16);
        returning(e, ACTOR_IS_PATHING_COMPLETE, 0);
        returning(e, ANIMATION_SPECIAL_IDLE_DONE_PLAYING, 0);
        returning(e, PACKAGE_FLAG_4, 0);
        returning(e, PACKAGE_FLAG_2, 0);
        returning_float(e, DISTANCE_FROM_REFERENCE, 0.0);
        stub(e, &[FLEE_PACKAGE_ADD_AVOIDED_REF]);
        global_f32(e, NO_LIMIT_FLOAT, -1.0);
        (p, actor)
    }

    #[test]
    fn test_high_process_process_flee_non_combat_stops_when_the_actor_says_so() {
        let mut e = Engine::new();
        let (p, actor) = flee_world(&mut e);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x358, 1)]);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert_eq!(
            call_order(&e),
            vec![process_slot(0x27c), actor_slot(0x358), process_slot(0x288)]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
    }

    #[test]
    fn test_high_process_process_flee_non_combat_without_target_or_location() {
        let mut e = Engine::new();
        let (p, actor) = flee_world(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        // No target yet: one is asked for the package and located; the
        // package target's reference becomes the process' target.
        assert_eq!(
            calls_to(&e, process_slot(0x8c)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x90)),
            vec![vec![p.addr(), actor, 0]]
        );
        assert_eq!(
            calls_to(&e, PACKAGE_TARGET_GET_REFERENCE),
            vec![vec![FLEE_PACKAGE_TARGET]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x12c)),
            vec![vec![p.addr(), 0x7000]]
        );
        // Neither a target nor a location: procedure 1 and out.
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert!(calls_to(&e, PACKAGE_DISTANCE_FOR_ACTOR).is_empty());
        // Without a package target nothing is set, and with a target
        // already in the process neither.
        returning(&mut e, PACKAGE_TARGET_WORD, 0);
        e.mem.set_u32(p.addr() + 0x40, 0x4444);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x8c)).is_empty());
        assert!(calls_to(&e, process_slot(0x12c)).is_empty());
    }

    #[test]
    fn test_high_process_process_flee_non_combat_flees_from_a_location() {
        let mut e = Engine::new();
        let (p, actor) = flee_world(&mut e);
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x128, FLEE_TARGET), (0x514, FLEE_LOCATION)],
        );
        // The location is 100 away, the package says 80: not far enough.
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 100.0);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, DISTANCE_FROM_REFERENCE),
            vec![vec![actor, FLEE_LOCATION, 0, 0]]
        );
        // The flee distance is the number at +8 of the package target (50).
        assert_eq!(
            calls_to(&e, actor_slot(0x410)),
            vec![vec![
                actor,
                FLEE_TARGET,
                1,
                1,
                0,
                0,
                FLEE_LOCATION,
                50.0f32.to_bits(),
                80.0f32.to_bits()
            ]]
        );
        // A running flee package (type 0x16) remembers the avoided target.
        assert_eq!(
            calls_to(&e, FLEE_PACKAGE_ADD_AVOIDED_REF),
            vec![vec![FLEE_PACKAGE, FLEE_TARGET]]
        );
        // Another kind of package does not.
        returning(&mut e, PACKAGE_TYPE, 0x10);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert!(calls_to(&e, FLEE_PACKAGE_ADD_AVOIDED_REF).is_empty());
        // Far enough (80 is not below 80): it does not flee.
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 80.0);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert!(calls_to(&e, actor_slot(0x410)).is_empty());
    }

    #[test]
    fn test_high_process_process_flee_non_combat_flees_from_a_target() {
        let mut e = Engine::new();
        let (p, actor) = flee_world(&mut e);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x128, FLEE_TARGET)]);
        // The flee distance is 50: a target 40 away is too close.
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 40.0);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, DISTANCE_FROM_REFERENCE),
            vec![vec![actor, FLEE_TARGET, 0, 0]]
        );
        assert_eq!(calls_to(&e, actor_slot(0x410)).len(), 1);
        // Without a package target the distance is the constant at 01012054
        // (-1.0): nothing is too close.
        returning(&mut e, PACKAGE_TARGET_WORD, 0);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert!(calls_to(&e, actor_slot(0x410)).is_empty());
    }

    #[test]
    fn test_high_process_process_flee_non_combat_settles_when_far_enough() {
        let mut e = Engine::new();
        let (p, actor) = flee_world(&mut e);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x128, FLEE_TARGET)]);
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 60.0);
        // Still moving: the movement is ended and, with a finished idle, a
        // new special idle starts.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x1e4, 0x7000)]);
        returning(&mut e, ANIMATION_SPECIAL_IDLE_DONE_PLAYING, 1);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x294)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x44)),
            vec![vec![p.addr(), actor, 0, 2, 1, 0, 1]]
        );
        assert!(calls_to(&e, process_slot(0x118)).is_empty());
        // Pathing complete but no package flag: nothing more.
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 1);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x294)).is_empty());
        assert!(calls_to(&e, process_slot(0x24)).is_empty());
        // With the flag 2 (or 4) the package ends.
        returning(&mut e, PACKAGE_FLAG_2, 1);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert_eq!(
            call_order(&e)[call_order(&e).len() - 4..],
            [
                process_slot(0x294),
                process_slot(0x118),
                process_slot(0x288),
                process_slot(0x24)
            ]
        );
        assert_eq!(calls_to(&e, process_slot(0x118)), vec![vec![p.addr(), 1]]);
        assert_eq!(
            calls_to(&e, process_slot(0x24)),
            vec![vec![p.addr(), actor, 0]]
        );
        returning(&mut e, PACKAGE_FLAG_2, 0);
        returning(&mut e, PACKAGE_FLAG_4, 1);
        e.call_log = Some(vec![]);
        e.call(0x008ddac0, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x24)).len(), 1);
    }

    #[test]
    fn test_small_accessors_between_008defe0_and_008df1c0() {
        let mut e = Engine::new();
        let o = block(&mut e);
        e.mem.set_u8(o.addr() + 0x6cc, 7);
        e.mem.set_u8(o.addr() + 0x94, 8);
        e.mem.set_u8(o.addr() + 0x80, 9);
        assert_eq!(e.call(0x008defe0, &args![o]).u8(), 7);
        assert_eq!(e.call(0x008df020, &args![o]).u8(), 8);
        assert_eq!(e.call(0x008df040, &args![o]).u8(), 9);
        e.call(0x008df1c0, &args![o, 1u32]);
        assert_eq!(e.mem.u8(o.addr() + 0xa1), 1);
        stub(&mut e, &[0x006d_2c00]);
        e.call_log = Some(vec![]);
        e.call(0x008df000, &args![o]);
        assert_eq!(calls_to(&e, 0x006d_2c00), vec![vec![o.addr() + 0x34]]);
    }

    /// The world of `ProcessSurface`: records the position `BuildRequest`
    /// receives at the address `0x7a00_0000`.
    fn surface_world(e: &mut Engine) -> (Ptr<HighProcess>, u32) {
        let p = block(e);
        give_vtable(e, p.addr(), PROCESS_TABLE, &[(0x214, 0)]);
        let position = e.mem.alloc(0x10);
        e.mem.set_f32(position, 1.0);
        e.mem.set_f32(position + 4, 2.0);
        e.mem.set_f32(position + 8, 3.0);
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x1f4, position)]);
        returning(e, ACTOR_CHECK_BREATH_TIMER, 0);
        returning(e, ACTOR_IS_PATHING_COMPLETE, 1);
        returning_float(e, REFERENCE_WATER_HEIGHT, 10.0);
        returning(e, REFERENCE_GET_WORLD_SPACE, 0x111);
        returning(e, REFERENCE_PATHING_CELL, 0x222);
        global_f32(e, SURFACE_HEIGHT_OFFSET, 25.0);
        e.map(0x7a00_0000, 0x40);
        e.register(ACTOR_BUILD_REQUEST, |e, a| {
            // The position copy, as BuildRequest sees it.
            for word in 0..3 {
                let value = e.mem.u32(a[2] + word * 4);
                e.mem.set_u32(0x7a00_0000 + word * 4, value);
            }
            Ret::default()
        });
        stub(
            e,
            &[
                ACTOR_STOP_MOVING,
                PATHING_REQUEST_CONSTRUCTOR,
                PATHING_REQUEST_SET_RADIUS,
                ACTOR_SET_PATHFINDING_GOAL_REQUEST,
                PATHING_REQUEST_DESTRUCTOR,
            ],
        );
        (p, actor)
    }

    #[test]
    fn test_high_process_process_surface_builds_a_request_above_the_water() {
        let mut e = Engine::new();
        let (p, actor) = surface_world(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008df060, &args![p, actor]);
        // The position is the actor's with the height raised to the water
        // height plus 25.
        assert_eq!(e.mem.f32(0x7a00_0000), 1.0);
        assert_eq!(e.mem.f32(0x7a00_0004), 2.0);
        assert_eq!(e.mem.f32(0x7a00_0008), 35.0);
        let build = calls_to(&e, ACTOR_BUILD_REQUEST);
        assert_eq!(build.len(), 1);
        let request = build[0][1];
        assert_eq!(
            (build[0][0], build[0][3], build[0][4]),
            (actor, 0x222, 0x111)
        );
        assert_eq!((build[0][5], build[0][6]), (25.0f32.to_bits(), 0));
        assert_eq!(
            calls_to(&e, PATHING_REQUEST_SET_RADIUS),
            vec![vec![request, 25.0f32.to_bits()]]
        );
        // The byte at +0xa1 was set while the request existed; it is then the
        // goal and destroyed.
        assert_eq!(
            calls_to(&e, ACTOR_SET_PATHFINDING_GOAL_REQUEST),
            vec![vec![actor, request]]
        );
        assert_eq!(
            calls_to(&e, PATHING_REQUEST_DESTRUCTOR),
            vec![vec![request]]
        );
        let order = call_order(&e);
        let position_of = |addr: u32| order.iter().position(|a| *a == addr).unwrap();
        assert!(
            position_of(PATHING_REQUEST_CONSTRUCTOR) < position_of(ACTOR_BUILD_REQUEST)
                && position_of(ACTOR_BUILD_REQUEST) < position_of(PATHING_REQUEST_SET_RADIUS)
                && position_of(PATHING_REQUEST_SET_RADIUS)
                    < position_of(ACTOR_SET_PATHFINDING_GOAL_REQUEST)
                && position_of(ACTOR_SET_PATHFINDING_GOAL_REQUEST)
                    < position_of(PATHING_REQUEST_DESTRUCTOR)
        );
    }

    #[test]
    fn test_high_process_process_surface_other_cases() {
        let mut e = Engine::new();
        let (p, actor) = surface_world(&mut e);
        // Without an actor nothing happens.
        e.call_log = Some(vec![]);
        e.call(0x008df060, &args![p, 0u32]);
        assert!(call_order(&e).is_empty());
        // Pathing not complete: no request.
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        e.call_log = Some(vec![]);
        e.call(0x008df060, &args![p, actor]);
        assert!(calls_to(&e, PATHING_REQUEST_CONSTRUCTOR).is_empty());
        // A running breath timer: the run-once package is cleared and the
        // actor stops.
        returning(&mut e, ACTOR_CHECK_BREATH_TIMER, 1);
        e.call_log = Some(vec![]);
        e.call(0x008df060, &args![p, actor]);
        assert_eq!(
            call_order(&e),
            vec![
                ACTOR_CHECK_BREATH_TIMER,
                process_slot(0x214),
                ACTOR_STOP_MOVING
            ]
        );
    }

    const ESCORT_SOLUTION_MINE: u32 = 0x8100;
    const ESCORT_SOLUTION_THEIRS: u32 = 0x8200;
    const ESCORT_QUEUE_TABLE: u32 = 0x7b00_0000;
    const ESCORT_OTHER_QUEUE_TABLE: u32 = 0x7c00_0000;

    /// The world of `ShouldWaitForEscortTarget`: the escorting actor `me`
    /// and the `escorted` one far apart (squared distance 1e6), a radius of
    /// 400 (interior cell), no requests yet.
    fn escort_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32, u32, u32) {
        let p = block(e);
        give_vtable(e, p.addr(), PROCESS_TABLE, &[(0x27c, 0x5000)]);
        let me_position = e.mem.alloc(0x10);
        let other_position = e.mem.alloc(0x10);
        let me = object_with_slots(e, ACTOR_TABLE, &[(0x1f4, me_position)]);
        let escorted = object_with_slots(e, OTHER_TABLE, &[(0x1f4, other_position)]);
        let location = e.mem.alloc(0x40);
        let flag = e.mem.alloc(0x10);
        e.mem.set_u32(p.addr() + 0x40, 0x1234);
        global_word(e, PLAYER_POINTER, 0x9999);
        returning(e, PACKAGE_ESCORT_FOLLOW_DISTANCE, 300);
        returning(e, REFERENCE_PATHING_CELL, 0x9000);
        returning(e, CELL_FLAG_TEST, 1);
        // The settings live 0x1000 above their game setting number.
        e.register(SETTING_VALUE, |_, a| ret(a[0] + 0x1000));
        global_f32(e, ESCORT_SETTING_BASE_RADIUS + 0x1000, 400.0);
        global_f32(e, ESCORT_SETTING_RADIUS_SCALE + 0x1000, 2.0);
        global_f32(e, ESCORT_SETTING_PATHING_EXTRA + 0x1000, 100.0);
        global_f64(e, ESCORT_FLAG_FACTOR, 0.5);
        returning(e, ACTOR_IS_PATHING, 0);
        stub(e, &[POINT_DIFFERENCE]);
        returning_float(e, POINT_SQUARED_LENGTH, 1.0e6);
        stub(
            e,
            &[
                PATHING_LOCATION_GET_CELL,
                PATHING_LOCATION_GET_WORLDSPACE,
                PATHING_LOCATION_FROM_ACTOR,
                PATHING_LOCATION_DESTRUCTOR,
                ACTOR_PATHING_MESSAGE_DESTRUCTOR,
                PATHING_REQUEST_CONSTRUCTOR,
                PATHING_REQUEST_SET_ACTOR,
                PATHING_REQUEST_SET_GOAL,
                PATH_MANAGER_BUILD_PATH,
                0x0090_5580,
                0x0049_68b0,
            ],
        );
        list_and_pointer_doubles(e);
        e.register(PATHING_REQUEST_CONSTRUCTOR, |_, a| ret(a[0]));
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(QUEUE_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        let copy: AbiFn = |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            ret(a[0])
        };
        e.register(QUEUE_POINTER_COPY, copy);
        e.register(REQUEST_POINTER_COPY, copy);
        returning(e, PATH_MANAGER_INSTANCE, 0x5555);
        (p, me, escorted, location, flag)
    }

    /// Forgets the requests and queues the process holds.
    fn clear_escort_requests(e: &mut Engine, process: u32) {
        for offset in [0x45c, 0x460, 0x464, 0x468] {
            e.mem.set_u32(process + offset, 0);
        }
    }

    /// Whether `ShouldWaitForEscortTarget` goes past the radius check for a
    /// squared distance of `length`.
    fn escort_continues(
        e: &mut Engine,
        w: &(Ptr<HighProcess>, u32, u32, u32, u32),
        length: f64,
    ) -> bool {
        clear_escort_requests(e, w.0.addr());
        returning_float(e, POINT_SQUARED_LENGTH, length);
        e.call_log = Some(vec![]);
        e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]);
        !calls_to(e, PATHING_LOCATION_GET_CELL).is_empty()
    }

    #[test]
    fn test_high_process_should_wait_for_escort_target_radius() {
        let mut e = Engine::new();
        let w = escort_world(&mut e);
        let p = w.0.addr();
        // An interior cell: the radius is the setting (400), squared 160000.
        assert!(!escort_continues(&mut e, &w, 150_000.0));
        assert!(escort_continues(&mut e, &w, 170_000.0));
        // Elsewhere it is the setting (2.0) times the distance (300).
        returning(&mut e, CELL_FLAG_TEST, 0);
        assert!(!escort_continues(&mut e, &w, 170_000.0));
        assert!(!escort_continues(&mut e, &w, 360_000.0), "not above");
        assert!(escort_continues(&mut e, &w, 400_000.0));
        // The cell asked is the one of `me`.
        assert_eq!(calls_to(&e, CELL_FLAG_TEST), vec![vec![0x9000]]);
        // For the player's package it is the distance itself.
        e.mem.set_u32(p + 0x40, 0x9999);
        assert!(!escort_continues(&mut e, &w, 80_000.0));
        assert!(escort_continues(&mut e, &w, 100_000.0));
        // A distance of at most 200 counts as 200.
        returning(&mut e, PACKAGE_ESCORT_FOLLOW_DISTANCE, 200);
        assert!(!escort_continues(&mut e, &w, 30_000.0));
        assert!(escort_continues(&mut e, &w, 50_000.0));
        returning(&mut e, PACKAGE_ESCORT_FOLLOW_DISTANCE, 100);
        assert!(!escort_continues(&mut e, &w, 30_000.0));
        // A pathing actor adds the setting 100 to the radius.
        e.mem.set_u32(p + 0x40, 0x1234);
        returning(&mut e, CELL_FLAG_TEST, 1);
        returning(&mut e, ACTOR_IS_PATHING, 1);
        assert!(!escort_continues(&mut e, &w, 240_000.0));
        assert!(escort_continues(&mut e, &w, 260_000.0));
        // A squared length that is not a number is never above.
        assert!(!escort_continues(&mut e, &w, f64::NAN));
    }

    #[test]
    fn test_high_process_should_wait_for_escort_target_flag() {
        let mut e = Engine::new();
        let w = escort_world(&mut e);
        // Radius 400: squared 160000, half of it 80000.
        e.mem.set_u8(w.4, 7);
        assert!(!escort_continues(&mut e, &w, 100_000.0));
        assert_eq!(e.mem.u8(w.4), 1, "above half, not above the whole");
        assert!(!escort_continues(&mut e, &w, 60_000.0));
        assert_eq!(e.mem.u8(w.4), 0);
        // Above the whole: the flag was set and is cleared again.
        e.mem.set_u8(w.4, 7);
        assert!(escort_continues(&mut e, &w, 170_000.0));
        assert_eq!(e.mem.u8(w.4), 0);
        // Without a flag address nothing is written.
        clear_escort_requests(&mut e, w.0.addr());
        e.call_log = Some(vec![]);
        let wait = e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, 0u32]).bool();
        assert!(wait);
        // The difference is "my position - the escorted one's".
        let difference = calls_to(&e, POINT_DIFFERENCE);
        assert_eq!(difference.len(), 1);
        assert_eq!(calls_to(&e, actor_slot(0x1f4)).len(), 1);
        assert_eq!(calls_to(&e, slot_target(OTHER_TABLE, 0x1f4)).len(), 1);
    }

    #[test]
    fn test_high_process_should_wait_for_escort_target_builds_the_paths() {
        let mut e = Engine::new();
        let w = escort_world(&mut e);
        let p = w.0.addr();
        returning_float(&mut e, POINT_SQUARED_LENGTH, 1.0e6);
        e.call_log = Some(vec![]);
        // Nothing present yet: requests and queues are built for both actors,
        // and the answer is "`me` is not pathing".
        assert!(e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool());
        let requests = calls_to(&e, OPERATOR_NEW);
        assert_eq!(
            requests,
            vec![vec![0xb0], vec![0x1c], vec![0xb0], vec![0x1c]]
        );
        let my_request = e.mem.u32(p + 0x45c);
        let other_request = e.mem.u32(p + 0x460);
        let my_queue = e.mem.u32(p + 0x464);
        let other_queue = e.mem.u32(p + 0x468);
        assert!(my_request != 0 && other_request != 0 && my_request != other_request);
        assert!(my_queue != 0 && other_queue != 0 && my_queue != other_queue);
        assert_eq!(e.mem.u32(my_queue), 0x0108_8158, "queue vtable");
        assert_eq!(
            e.mem.u32(my_queue + 0x14),
            0x0108_814c,
            "NiRefObject vtable"
        );
        assert_eq!(
            calls_to(&e, PATHING_REQUEST_SET_ACTOR),
            vec![vec![my_request, w.1], vec![other_request, w.2]]
        );
        assert_eq!(
            calls_to(&e, PATHING_REQUEST_SET_GOAL),
            vec![vec![my_request, w.3], vec![other_request, w.3]]
        );
        assert_eq!(
            calls_to(&e, PATH_MANAGER_BUILD_PATH),
            vec![
                vec![0x5555, my_request, my_queue, 1],
                vec![0x5555, other_request, other_queue, 1]
            ]
        );
        // Both locations were built (me, then escorted) and destroyed in the
        // opposite order.
        let built = calls_to(&e, PATHING_LOCATION_FROM_ACTOR);
        assert_eq!((built[0][1], built[1][1]), (w.1, w.2));
        let destroyed = calls_to(&e, PATHING_LOCATION_DESTRUCTOR);
        assert_eq!(destroyed, vec![vec![built[1][0]], vec![built[0][0]]]);
        // While `me` is pathing the answer is false.
        returning(&mut e, ACTOR_IS_PATHING, 1);
        clear_escort_requests(&mut e, p);
        assert!(!e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool());
        // With only one request present both are built again.
        e.mem.set_u32(p + 0x460, 0);
        e.call_log = Some(vec![]);
        e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]);
        assert_eq!(calls_to(&e, PATH_MANAGER_BUILD_PATH).len(), 2);
    }

    /// The same world with both requests and both queues present; the
    /// message constructor hands out the solutions `ESCORT_SOLUTION_MINE`
    /// and `ESCORT_SOLUTION_THEIRS`, whose lengths are `mine` and `theirs`.
    fn escort_message_world(
        e: &mut Engine,
        mine_length: f64,
        theirs_length: f64,
    ) -> (Ptr<HighProcess>, u32, u32, u32, u32) {
        let w = escort_world(e);
        let p = w.0.addr();
        e.mem.set_u32(p + 0x45c, 0x100);
        e.mem.set_u32(p + 0x460, 0x200);
        let my_queue = object_with_slots(e, ESCORT_QUEUE_TABLE, &[(0x10, 1), (4, 0)]);
        let other_queue = object_with_slots(e, ESCORT_OTHER_QUEUE_TABLE, &[(0x10, 1)]);
        e.mem.set_u32(p + 0x464, my_queue);
        e.mem.set_u32(p + 0x468, other_queue);
        let mut made = 0;
        e.register_double(ACTOR_PATHING_MESSAGE_CONSTRUCTOR, move |e, a| {
            let solution = if made == 0 {
                ESCORT_SOLUTION_MINE
            } else {
                ESCORT_SOLUTION_THEIRS
            };
            made += 1;
            e.mem.set_u32(a[0] + 4, solution);
            Ret::default()
        });
        e.register_double(PATHING_SOLUTION_LENGTH, move |_, a| Ret {
            st0: if a[0] == ESCORT_SOLUTION_MINE {
                mine_length
            } else {
                theirs_length
            },
            ..Ret::default()
        });
        w
    }

    #[test]
    fn test_high_process_should_wait_for_escort_target_shorter_own_solution() {
        let mut e = Engine::new();
        let w = escort_message_world(&mut e, 5.0, 9.0);
        let p = w.0.addr();
        // `me` is pathing, so only the early answer can be true.
        returning(&mut e, ACTOR_IS_PATHING, 1);
        e.call_log = Some(vec![]);
        let wait = e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool();
        assert!(wait, "my solution is shorter");
        assert_eq!(calls_to(&e, slot_target(ESCORT_QUEUE_TABLE, 0x10)).len(), 1);
        assert_eq!(
            calls_to(&e, slot_target(ESCORT_OTHER_QUEUE_TABLE, 0x10)).len(),
            1
        );
        // The requests and queues were released.
        for offset in [0x45c, 0x460, 0x464, 0x468] {
            assert_eq!(e.mem.u32(p + offset), 0, "+{offset:#x}");
        }
        // Only the radius check asked whether `me` is pathing.
        assert_eq!(calls_to(&e, ACTOR_IS_PATHING).len(), 1);
        // Both messages and both locations were destroyed.
        assert_eq!(calls_to(&e, ACTOR_PATHING_MESSAGE_DESTRUCTOR).len(), 2);
        assert_eq!(calls_to(&e, PATHING_LOCATION_DESTRUCTOR).len(), 2);
        assert!(calls_to(&e, PATH_MANAGER_BUILD_PATH).is_empty());
    }

    #[test]
    fn test_high_process_should_wait_for_escort_target_longer_own_solution() {
        let mut e = Engine::new();
        let w = escort_message_world(&mut e, 9.0, 5.0);
        let p = w.0.addr();
        returning(&mut e, ACTOR_IS_PATHING, 1);
        // Not shorter: falls through to "me is not pathing" (false).
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool());
        assert_eq!(e.mem.u32(p + 0x45c), 0, "still released");
        assert_eq!(calls_to(&e, ACTOR_IS_PATHING).len(), 2);
        // Equal lengths are not shorter either.
        let w = escort_message_world(&mut e, 5.0, 5.0);
        returning(&mut e, ACTOR_IS_PATHING, 0);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool());
        assert_eq!(calls_to(&e, ACTOR_IS_PATHING).len(), 2);
    }

    #[test]
    fn test_high_process_should_wait_for_escort_target_queue_answers() {
        let mut e = Engine::new();
        let w = escort_message_world(&mut e, 5.0, 9.0);
        let p = w.0.addr();
        returning(&mut e, ACTOR_IS_PATHING, 1);
        let my_queue = e.mem.u32(p + 0x464);
        // The first queue has nothing: nothing else is done.
        give_vtable(&mut e, my_queue, ESCORT_QUEUE_TABLE, &[(0x10, 0)]);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool());
        assert!(calls_to(&e, slot_target(ESCORT_OTHER_QUEUE_TABLE, 0x10)).is_empty());
        assert_eq!(e.mem.u32(p + 0x45c), 0x100, "requests kept");
        // The second queue has nothing: the first message goes back on the
        // first queue.
        give_vtable(&mut e, my_queue, ESCORT_QUEUE_TABLE, &[(0x10, 1)]);
        let other_queue = e.mem.u32(p + 0x468);
        give_vtable(&mut e, other_queue, ESCORT_OTHER_QUEUE_TABLE, &[(0x10, 0)]);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool());
        let first_pops = calls_to(&e, slot_target(ESCORT_QUEUE_TABLE, 0x10));
        let pushes = calls_to(&e, slot_target(ESCORT_QUEUE_TABLE, 0x4));
        assert_eq!(pushes, first_pops, "the popped message is pushed back");
        assert_eq!(e.mem.u32(p + 0x45c), 0x100, "requests kept");
        assert_eq!(calls_to(&e, ACTOR_PATHING_MESSAGE_DESTRUCTOR).len(), 2);
        // Solutions missing: the requests are released and the answer is
        // "me is not pathing".
        give_vtable(&mut e, other_queue, ESCORT_OTHER_QUEUE_TABLE, &[(0x10, 1)]);
        stub(&mut e, &[ACTOR_PATHING_MESSAGE_CONSTRUCTOR]);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008df1e0, &args![w.0, w.1, w.2, w.3, w.4]).bool());
        assert_eq!(e.mem.u32(p + 0x45c), 0);
        assert!(calls_to(&e, PATHING_SOLUTION_LENGTH).is_empty());
    }

    #[test]
    fn test_actor_pathing_message_queue_constructor_and_destructors() {
        let mut e = Engine::new();
        let queue = e.mem.alloc(0x20);
        stub(
            &mut e,
            &[0x0090_5580, 0x0049_68b0, 0x0049_6910, 0x0090_55c0],
        );
        stub(&mut e, &[OPERATOR_DELETE_SIZED]);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008df900, &args![queue]).u32(), queue);
        assert_eq!(
            call_order(&e),
            vec![0x0090_5580, 0x0049_68b0],
            "base queue, then the NiRefObject part"
        );
        assert_eq!(calls_to(&e, 0x0090_5580), vec![vec![queue, 0x011d_77f0]]);
        assert_eq!(calls_to(&e, 0x0049_68b0), vec![vec![queue + 0x14]]);
        assert_eq!(e.mem.u32(queue), 0x0108_8158);
        assert_eq!(e.mem.u32(queue + 0x14), 0x0108_814c);
        // The destructor undoes the NiRefObject part first.
        e.call_log = Some(vec![]);
        e.call(0x008df9b0, &args![queue]);
        assert_eq!(call_order(&e), vec![0x0049_6910, 0x0090_55c0]);
        assert_eq!(calls_to(&e, 0x0049_6910), vec![vec![queue + 0x14]]);
        // The deleting destructor frees only when bit 0 is set.
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008df980, &args![queue, 0u32]).u32(), queue);
        assert!(calls_to(&e, OPERATOR_DELETE_SIZED).is_empty());
        assert_eq!(calls_to(&e, 0x0090_55c0).len(), 1);
        assert_eq!(e.call(0x008df980, &args![queue, 3u32]).u32(), queue);
        assert_eq!(calls_to(&e, OPERATOR_DELETE_SIZED), vec![vec![queue, 0x1c]]);
    }

    #[test]
    fn test_flag_tests_of_the_word_at_0x24() {
        let mut e = Engine::new();
        let o = block(&mut e);
        e.mem.set_u16(o.addr() + 0x24, 0x100);
        assert!(e.call(0x008e30e0, &args![o]).bool());
        assert!(!e.call(0x008e3100, &args![o]).bool());
        assert!(!e.call(0x008e3120, &args![o]).bool());
        e.mem.set_u16(o.addr() + 0x24, 0x600);
        assert!(!e.call(0x008e30e0, &args![o]).bool());
        assert!(e.call(0x008e3100, &args![o]).bool());
        assert!(e.call(0x008e3120, &args![o]).bool());
        // Only the low word counts.
        e.mem.set_u32(o.addr() + 0x24, 0x0001_0000);
        assert!(!e.call(0x008e30e0, &args![o]).bool());
    }

    #[test]
    fn test_fn_008e3e90_returns_false() {
        let mut e = Engine::new();
        assert!(!e.call(0x008e3e90, &args![]).bool());
    }

    #[test]
    fn test_high_process_reduce_detection_timer() {
        let mut e = Engine::new();
        let p = block(&mut e);
        returning_float(&mut e, FRAME_TIME, 0.25);
        e.mem.set_f32(p.addr() + 0x2f8, 1.0);
        e.call_log = Some(vec![]);
        e.call(0x008e40a0, &args![p]);
        assert_eq!(e.mem.f32(p.addr() + 0x2f8), 0.75);
        assert_eq!(calls_to(&e, FRAME_TIME), vec![vec![FRAME_TIMER]]);
        e.call(0x008e40a0, &args![p]);
        e.call(0x008e40a0, &args![p]);
        e.call(0x008e40a0, &args![p]);
        assert_eq!(e.mem.f32(p.addr() + 0x2f8), 0.0);
    }

    #[test]
    fn test_shape_phantom_release_and_test() {
        let mut e = Engine::new();
        let p = block(&mut e);
        list_and_pointer_doubles(&mut e);
        e.register(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        // Without a phantom: false, and releasing does nothing.
        assert!(!e.call(0x008e5700, &args![p]).bool());
        e.call_log = Some(vec![]);
        e.call(0x008e56b0, &args![p]);
        assert!(calls_to(&e, NI_POINTER_SET).is_empty());
        // With one: true; releasing calls its slot 0xa0 and clears the word.
        let phantom = object_with_slots(&mut e, OTHER_TABLE, &[(0xa0, 0)]);
        e.mem.set_u32(p.addr() + 0x434, phantom);
        assert!(e.call(0x008e5700, &args![p]).bool());
        e.call_log = Some(vec![]);
        e.call(0x008e56b0, &args![p]);
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, 0xa0)),
            vec![vec![phantom]]
        );
        assert_eq!(
            calls_to(&e, NI_POINTER_SET),
            vec![vec![p.addr() + 0x434, 0]]
        );
        assert_eq!(e.mem.u32(p.addr() + 0x434), 0);
        assert!(!e.call(0x008e5700, &args![p]).bool());
    }

    #[test]
    fn test_fn_008e5670_and_008e5620_convert_the_havok_vector() {
        let mut e = Engine::new();
        let actor = e.mem.alloc(0x20);
        let out = e.mem.alloc(0x20);
        returning(&mut e, DEFAULT_VECTOR, 0x9000);
        returning(&mut e, CONTROLLER_VECTOR, 0x7020);
        returning(&mut e, ACTOR_CHARACTER_CONTROLLER, 0);
        e.register(NODE_ITEM_ADDRESS, |_, a| ret(a[0]));
        // Without a controller the default vector is used.
        assert_eq!(e.call(0x008e5670, &args![actor]).u32(), 0x9000);
        // With one, the vector at +0x20 of it.
        returning(&mut e, ACTOR_CHARACTER_CONTROLLER, 0x7000);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008e5670, &args![actor]).u32(), 0x7020);
        assert_eq!(calls_to(&e, CONTROLLER_VECTOR), vec![vec![0x7000]]);
        // The point is made from that vector.
        e.register(HAVOK_VECTOR_TO_POINT, |e, a| {
            e.mem.set_f32(a[0], 1.0);
            e.mem.set_f32(a[0] + 4, 2.0);
            e.mem.set_f32(a[0] + 8, a[1] as f32);
            ret(a[0])
        });
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008e5620, &args![actor, out]).u32(), out);
        assert_eq!(e.mem.f32(out), 1.0);
        assert_eq!(e.mem.f32(out + 4), 2.0);
        assert_eq!(e.mem.f32(out + 8), 0x7020 as f32);
        let converted = calls_to(&e, HAVOK_VECTOR_TO_POINT);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0][1], 0x7020);
    }

    #[test]
    fn test_fn_008e5100_builds_a_box_shape() {
        let mut e = Engine::new();
        let shape = e.mem.alloc(0x40);
        let half_extents = e.mem.alloc(0x10);
        global_word(&mut e, BOX_SHAPE_COUNT, 5);
        stub(
            &mut e,
            &[
                CONVEX_SHAPE_CONSTRUCTOR,
                NODE_ITEM_ADDRESS,
                BOX_SHAPE_SET_DIMENSIONS,
            ],
        );
        returning(&mut e, POINT_TO_HAVOK_VECTOR, 0x5555);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008e5100, &args![shape, half_extents]).u32(), shape);
        assert_eq!(e.mem.u32(shape), 0x0103_093c);
        assert_eq!(e.mem.u32(BOX_SHAPE_COUNT), 6);
        assert_eq!(
            call_order(&e),
            vec![
                CONVEX_SHAPE_CONSTRUCTOR,
                NODE_ITEM_ADDRESS,
                POINT_TO_HAVOK_VECTOR,
                BOX_SHAPE_SET_DIMENSIONS
            ]
        );
        assert_eq!(calls_to(&e, CONVEX_SHAPE_CONSTRUCTOR), vec![vec![shape]]);
        let converted = calls_to(&e, POINT_TO_HAVOK_VECTOR);
        assert_eq!(converted[0][1], half_extents);
        assert_eq!(
            calls_to(&e, BOX_SHAPE_SET_DIMENSIONS),
            vec![vec![shape, 0x5555]]
        );
    }

    /// The random-offset world of `008e3dc0`: the random floats come from
    /// `values` in order; the point helpers do real arithmetic.
    fn random_offset_world(e: &mut Engine, values: Vec<f64>) {
        let mut next = 0;
        e.register_double(RANDOM_FLOAT, move |_, _| {
            let value = values[next];
            next += 1;
            Ret {
                st0: value,
                ..Ret::default()
            }
        });
        e.register(POINT_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            e.mem.set_u32(a[0] + 4, a[2]);
            e.mem.set_u32(a[0] + 8, a[3]);
            ret(a[0])
        });
        stub(e, &[POINT_UNITIZE]);
        e.register(POINT_SCALE, |e, a| {
            let factor = f32::from_bits(a[2]);
            for i in 0..3 {
                let value = e.mem.f32(a[0] + i * 4) * factor;
                e.mem.set_f32(a[1] + i * 4, value);
            }
            ret(a[1])
        });
        e.register(POINT_SUM, |e, a| {
            for i in 0..3 {
                let value = e.mem.f32(a[0] + i * 4) + e.mem.f32(a[2] + i * 4);
                e.mem.set_f32(a[1] + i * 4, value);
            }
            ret(a[1])
        });
    }

    #[test]
    fn test_fn_008e3dc0_adds_a_random_offset() {
        let mut e = Engine::new();
        let out = e.mem.alloc(0x10);
        // Directions (0.5, 0.25, 0.75), the length (2 + 1) * 0.5 = 1.5.
        random_offset_world(&mut e, vec![0.5, 0.25, 0.75, 0.5]);
        e.call_log = Some(vec![]);
        let result = e
            .call(
                0x008e3dc0,
                &args![out, 10.0f32, 20.0f32, 30.0f32, 2.0f32, 1.0f32, 1u32],
            )
            .u32();
        assert_eq!(result, out);
        assert_eq!(e.mem.f32(out), 10.75);
        assert_eq!(e.mem.f32(out + 4), 20.375);
        assert_eq!(e.mem.f32(out + 8), 31.125);
        assert_eq!(calls_to(&e, POINT_UNITIZE).len(), 1);
        // Without the flag the third component stays 0 and only three random
        // floats are used; a length below `b` is raised to `b` (0.1 * 3 < 1).
        random_offset_world(&mut e, vec![0.5, 0.25, 0.1]);
        e.call(
            0x008e3dc0,
            &args![out, 0.0f32, 0.0f32, 0.0f32, 2.0f32, 1.0f32, 0u32],
        );
        assert_eq!(e.mem.f32(out), 0.5);
        assert_eq!(e.mem.f32(out + 4), 0.25);
        assert_eq!(e.mem.f32(out + 8), 0.0);
    }

    #[test]
    fn test_fn_008e3ea0_adds_references_to_the_list() {
        let mut e = Engine::new();
        let reference = 0x7a10_0000u32;
        e.map(0x7a10_0000, 0x100);
        returning(&mut e, REFERENCE_FLAG_20, 0);
        returning(&mut e, REFERENCE_FLAG_4000, 0);
        returning(&mut e, REFERENCE_GET_FORM, 0x9000);
        returning(&mut e, FORM_TYPE_BYTE, 0x1c);
        returning(&mut e, REFERENCE_EXTRA_DATA_ENTRY, 1);
        // The double saves what the added slot holds.
        e.register(REFERENCE_LIST_ADD, |e, a| {
            let held = e.mem.u32(a[1]);
            e.mem.set_u32(0x7a10_0010, held);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008e3ea0, &args![reference]).bool());
        assert_eq!(
            calls_to(&e, REFERENCE_LIST_ADD)
                .iter()
                .map(|c| c[0])
                .collect::<Vec<_>>(),
            vec![REFERENCE_LIST]
        );
        assert_eq!(e.mem.u32(0x7a10_0010), reference);
        // Each condition alone keeps the reference out.
        for reason in 0..5 {
            returning(&mut e, REFERENCE_FLAG_20, u32::from(reason == 0));
            returning(&mut e, REFERENCE_FLAG_4000, u32::from(reason == 1));
            returning(
                &mut e,
                FORM_TYPE_BYTE,
                if reason == 2 { 0x1d } else { 0x1c },
            );
            returning(&mut e, REFERENCE_EXTRA_DATA_ENTRY, u32::from(reason != 3));
            let target = if reason == 4 { 0 } else { reference };
            e.call_log = Some(vec![]);
            assert!(!e.call(0x008e3ea0, &args![target]).bool());
            assert!(calls_to(&e, REFERENCE_LIST_ADD).is_empty(), "case {reason}");
        }
    }

    const ACCOMPANY_PACKAGE: u32 = 0x5000;
    const ACCOMPANY_TARGET_PACKAGE: u32 = 0x5100;
    const TARGET_PROCESS_TABLE: u32 = 0x7d00_0000;
    const GOAL_SCRATCH: u32 = 0x7a20_0000;
    const ACCOMPANY_TARGET_POSITION: u32 = 0x7a30_0000;

    /// Point helpers that do the arithmetic: `00439ef0` subtracts, `004a7290`
    /// returns the squared length, `004019d0` is the square root.
    fn point_arithmetic(e: &mut Engine) {
        e.register(POINT_DIFFERENCE, |e, a| {
            for i in 0..3 {
                let value = e.mem.f32(a[0] + i * 4) - e.mem.f32(a[2] + i * 4);
                e.mem.set_f32(a[1] + i * 4, value);
            }
            ret(a[1])
        });
        e.register(POINT_SQUARED_LENGTH, |e, a| {
            let mut sum = 0.0f64;
            for i in 0..3 {
                let value = f64::from(e.mem.f32(a[0] + i * 4));
                sum += value * value;
            }
            Ret {
                st0: sum,
                ..Ret::default()
            }
        });
        e.register(FLOAT_SQUARE_ROOT, |_, a| Ret {
            st0: f64::from(f32::from_bits(a[0])).sqrt(),
            ..Ret::default()
        });
    }

    /// Writes the three floats to `address`.
    fn put_point(e: &mut Engine, address: u32, x: f32, y: f32, z: f32) {
        e.mem.set_f32(address, x);
        e.mem.set_f32(address + 4, y);
        e.mem.set_f32(address + 8, z);
    }

    /// The world of `ProcessAccompany`: the actor at the origin, its target
    /// (an actor) at (500, 0, 0), the target's destination at (100, 0, 0) and
    /// the movers' positions at (0, 0, 0) and (300, 0, 0).
    fn accompany_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, ACCOMPANY_PACKAGE),
                (0x8c, 0),
                (0x288, 0),
                (0x294, 0),
                (0x34c, 0),
                (0x12c, 0),
            ],
        );
        let my_position = e.mem.alloc(0x10);
        let target_position = ACCOMPANY_TARGET_POSITION;
        e.map(target_position, 0x10);
        put_point(e, target_position, 500.0, 0.0, 0.0);
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x1f4, my_position), (0x100, 1)]);
        let target = object_with_slots(
            e,
            OTHER_TABLE,
            &[(0x100, 1), (0x22c, 0), (0x1f4, target_position)],
        );
        e.mem.set_u32(p.addr() + 0x40, target);
        let target_process = object_with_slots(
            e,
            TARGET_PROCESS_TABLE,
            &[
                (0x27c, ACCOMPANY_TARGET_PACKAGE),
                (0x98, 0),
                (0x9c, 0x111),
                (0xa0, 0x222),
            ],
        );
        e.register(slot_target(TARGET_PROCESS_TABLE, 0x98), |e, a| {
            put_point(e, a[1], 100.0, 0.0, 0.0);
            Ret::default()
        });
        e.register_double(ACTOR_PROCESS, move |_, a| {
            ret(if a[0] == target { target_process } else { 0 })
        });
        e.mem.set_u32(actor + 0x190, 0x6000);
        e.mem.set_u32(target + 0x190, 0x6100);
        e.register(MOVER_POSITION, |e, a| {
            let x = if a[0] == 0x6100 { 300.0 } else { 0.0 };
            put_point(e, a[1], x, 0.0, 0.0);
            ret(a[1])
        });
        point_arithmetic(e);
        returning(e, PACKAGE_FLAG_10000, 0);
        returning(e, PACKAGE_ESCORT_FOLLOW_DISTANCE, 100);
        returning(e, ACTOR_IS_PATHING_COMPLETE, 1);
        returning(e, ACTOR_IS_PATHING, 0);
        returning(e, REFERENCE_FLAG_800, 0);
        returning(e, REFERENCE_HOLDS_OBJECT, 0);
        returning(e, WORD_AT_0XC, 0);
        returning_float(e, DISTANCE_FROM_REFERENCE, 0.0);
        returning(e, ACTOR_MOVE_MODE, 2);
        stub(e, &[ACTOR_SET_MOVE_MODE]);
        e.map(GOAL_SCRATCH, 0x40);
        e.register(ACTOR_SET_PATHFINDING_GOAL, |e, a| {
            // The goal location, as it is when the call is made.
            for i in 0..3 {
                let value = e.mem.u32(a[1] + i * 4);
                e.mem.set_u32(GOAL_SCRATCH + i * 4, value);
            }
            Ret::default()
        });
        global_f64(e, ACCOMPANY_EXTRA_DISTANCE, 300.0);
        global_f64(e, ACCOMPANY_RUN_FACTOR, 3.0);
        (p, actor, target)
    }

    #[test]
    fn test_high_process_process_accompany_early_exits() {
        let mut e = Engine::new();
        let (p, actor, target) = accompany_world(&mut e);
        // No package.
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x27c, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(call_order(&e), vec![process_slot(0x27c)]);
        // A package with the flag 0x10000.
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[(0x27c, ACCOMPANY_PACKAGE)],
        );
        returning(&mut e, PACKAGE_FLAG_10000, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(
            call_order(&e),
            vec![process_slot(0x27c), PACKAGE_FLAG_10000]
        );
        // A target without a process.
        returning(&mut e, PACKAGE_FLAG_10000, 0);
        e.register_double(ACTOR_PROCESS, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert!(calls_to(&e, slot_target(TARGET_PROCESS_TABLE, 0x98)).is_empty());
        // No target at all: the process asks for one, which is not an actor.
        e.mem.set_u32(p.addr() + 0x40, 0);
        give_vtable(&mut e, target, OTHER_TABLE, &[(0x100, 0)]);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x8c)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
    }

    #[test]
    fn test_high_process_process_accompany_without_an_actor_target() {
        let mut e = Engine::new();
        let (p, actor, target) = accompany_world(&mut e);
        give_vtable(&mut e, target, OTHER_TABLE, &[(0x100, 0)]);
        returning(&mut e, WORD_AT_0XC, 0x77);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        // The package target is (re)set; the actor holds no such object.
        assert_eq!(
            calls_to(&e, process_slot(0x8c)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(
            calls_to(&e, REFERENCE_HOLDS_OBJECT),
            vec![vec![actor, 0x77]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x294)),
            vec![vec![p.addr(), actor]]
        );
        // An actor that does hold it goes on to the accompanying code.
        returning(&mut e, REFERENCE_HOLDS_OBJECT, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x288)).is_empty());
        assert!(!calls_to(&e, slot_target(TARGET_PROCESS_TABLE, 0x98)).is_empty());
    }

    #[test]
    fn test_high_process_process_accompany_dead_target() {
        let mut e = Engine::new();
        let (p, actor, target) = accompany_world(&mut e);
        give_vtable(&mut e, target, OTHER_TABLE, &[(0x22c, 1)]);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, 0x22c)),
            vec![vec![target, 0]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert!(calls_to(&e, process_slot(0x294)).is_empty());
        assert!(calls_to(&e, POINT_DIFFERENCE).is_empty());
        // The flag 0x800 of the target does the same.
        give_vtable(&mut e, target, OTHER_TABLE, &[(0x22c, 0)]);
        returning(&mut e, REFERENCE_FLAG_800, 1);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x294)).len(), 1);
    }

    #[test]
    fn test_high_process_process_accompany_waits_for_a_far_target() {
        let mut e = Engine::new();
        let (p, actor, target) = accompany_world(&mut e);
        // The destination (100, 0, 0) is nearer to the actor (10000) than to
        // the target (160000): the target is far, the movement ends.
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 350.0);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x294)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(
            calls_to(&e, DISTANCE_FROM_REFERENCE),
            vec![vec![actor, target, 0, 0]]
        );
        assert!(
            calls_to(&e, process_slot(0x288)).is_empty(),
            "350 < 100 + 300"
        );
        // At 450 the procedure -1 is added.
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 450.0);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 0xffff_ffff]]
        );
        // The squared follow distance (10000) above the squared distance to
        // the target is enough too: the target at 50 from the actor.
        put_point(&mut e, ACCOMPANY_TARGET_POSITION, 50.0, 0.0, 0.0);
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 350.0);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x294)).len(), 1);
        assert_eq!(calls_to(&e, DISTANCE_FROM_REFERENCE).len(), 1);
    }

    #[test]
    fn test_high_process_process_accompany_runs_after_a_far_target() {
        let mut e = Engine::new();
        let (p, actor, target) = accompany_world(&mut e);
        // Destination at the target: not nearer to the actor, and the target
        // (500) is farther than the follow distance (100).
        e.register(slot_target(TARGET_PROCESS_TABLE, 0x98), |e, a| {
            put_point(e, a[1], 500.0, 0.0, 0.0);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        // The movers are 300 apart: the actor runs after the target.
        assert_eq!(
            calls_to(&e, FLOAT_SQUARE_ROOT),
            vec![vec![90_000.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x34c)),
            vec![vec![
                p.addr(),
                actor,
                300.0f32.to_bits(),
                100.0f32.to_bits(),
                300.0f32.to_bits(),
                0,
                0
            ]]
        );
        assert_eq!(calls_to(&e, ACTOR_MOVE_MODE), vec![vec![target]]);
        assert_eq!(calls_to(&e, ACTOR_SET_MOVE_MODE), vec![vec![actor, 2]]);
        let goal = calls_to(&e, ACTOR_SET_PATHFINDING_GOAL);
        assert_eq!(goal.len(), 1);
        assert_eq!(
            (goal[0][0], goal[0][2], goal[0][3], goal[0][4], goal[0][5]),
            (actor, 0x111, 0x222, 100.0f32.to_bits(), 0)
        );
        assert_eq!(
            e.mem.f32(GOAL_SCRATCH),
            300.0,
            "the target mover's position"
        );
        // A pathing actor does not start again.
        returning(&mut e, ACTOR_IS_PATHING, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert!(calls_to(&e, ACTOR_SET_PATHFINDING_GOAL).is_empty());
        // Movers close together: nothing to do.
        returning(&mut e, ACTOR_IS_PATHING, 0);
        e.register(MOVER_POSITION, |e, a| {
            let x = if a[0] == 0x6100 { 50.0 } else { 0.0 };
            put_point(e, a[1], x, 0.0, 0.0);
            ret(a[1])
        });
        e.call_log = Some(vec![]);
        e.call(0x008e0b00, &args![p, actor]);
        assert!(calls_to(&e, ACTOR_SET_PATHFINDING_GOAL).is_empty());
    }

    const SLEEP_FURNITURE_A: u32 = 0x7a40_0000;
    const SLEEP_FURNITURE_B: u32 = 0x7a40_1000;

    /// Puts `items` into the `BSSimpleList` whose first node is at `head`
    /// (each node: the item, then the next node).
    fn fill_list(e: &mut Engine, head: u32, items: &[u32]) {
        let mut node = head;
        for (index, item) in items.iter().enumerate() {
            e.mem.set_u32(node, *item);
            if index + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            } else {
                e.mem.set_u32(node + 4, 0);
            }
        }
        if items.is_empty() {
            e.mem.set_u32(head, 0);
            e.mem.set_u32(head + 4, 0);
        }
    }

    /// The list helpers on real memory: nodes are `(item, next)`.
    fn list_doubles(e: &mut Engine) {
        e.register(NODE_ITEM_ADDRESS, |_, a| ret(a[0]));
        e.register(NODE_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(LIST_IS_EMPTY, |e, a| {
            ret(u32::from(e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0))
        });
        e.register(LIST_COUNT, |e, a| {
            let mut node = a[0];
            let mut count = 0;
            while node != 0 && e.mem.u32(node) != 0 {
                count += 1;
                node = e.mem.u32(node + 4);
            }
            ret(count)
        });
        // Removes the head when its item is the one in the given word.
        e.register(LIST_REMOVE_ITEM, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next != 0 {
                let item = e.mem.u32(next);
                let after = e.mem.u32(next + 4);
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(a[0] + 4, after);
            } else {
                e.mem.set_u32(a[0], 0);
            }
            Ret::default()
        });
    }

    /// The world of `ProcessSleep`: an NPC actor in state 1, no furniture, an
    /// empty candidate list, the gate open.
    fn sleep_world(e: &mut Engine) -> (Ptr<HighProcess>, u32) {
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, 0x5000),
                (0x4bc, 0),
                (0x12c, 0),
                (0x7c8, 0),
                (0x118, 0),
                (0x294, 0),
                (0x288, 0),
                (0x84, 0),
            ],
        );
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x214, 1), (0x218, 1), (0x418, 0)]);
        for furniture in [SLEEP_FURNITURE_A, SLEEP_FURNITURE_B] {
            e.map(furniture, 0x40);
        }
        list_doubles(e);
        stub(
            e,
            &[
                ACTOR_GET_OUT_OF_FURNITURE_QUICK,
                PROCESS_CLEAR_FURNITURE_MARKER,
                PROCESS_FIND_BED_CHAIRS,
                FURNITURE_GATE_RELEASE,
                LIST_CLEAR,
            ],
        );
        returning(e, FURNITURE_GATE_CHECK, 1);
        // A furniture has a free marker when its word +0x10 is set, an owner
        // in the word +0x14.
        e.register(REFERENCE_HAS_FREE_MARKER, |e, a| {
            ret(e.mem.u32(a[0] + 0x10))
        });
        e.register(REFERENCE_GET_OWNER, |e, a| ret(e.mem.u32(a[0] + 0x14)));
        returning(e, RANDOM_NUMBER, 5);
        returning(e, PROCESS_WANTS_TO_FINISH_SLEEP, 0);
        returning(e, ACTOR_GET_FACE_ANIMATION_DATA, 0);
        global_word(e, FURNITURE_TIME_GLOBAL, 100);
        global_f32(e, SLEEP_ACQUIRE_TIMER_VALUE, 20.0);
        (p, actor)
    }

    #[test]
    fn test_high_process_process_sleep_in_bed() {
        let mut e = Engine::new();
        let (p, actor) = sleep_world(&mut e);
        let list = p.addr() + 0xd0;
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x214, 9)]);
        let face = object_with_slots(&mut e, OTHER_TABLE, &[(0xd4, 0), (0xd8, 0)]);
        returning(&mut e, ACTOR_GET_FACE_ANIMATION_DATA, face);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        // No furniture: out of furniture quickly, the marker cleared, beds
        // searched for (the list is empty).
        assert_eq!(
            calls_to(&e, ACTOR_GET_OUT_OF_FURNITURE_QUICK),
            vec![vec![actor]]
        );
        assert_eq!(calls_to(&e, PROCESS_CLEAR_FURNITURE_MARKER).len(), 1);
        assert_eq!(
            calls_to(&e, PROCESS_FIND_BED_CHAIRS),
            vec![vec![p.addr(), actor, 0, 0]]
        );
        // Asleep: the action completes and the list is cleared.
        assert_eq!(calls_to(&e, process_slot(0x118)), vec![vec![p.addr(), 1]]);
        assert_eq!(calls_to(&e, LIST_CLEAR), vec![vec![list]]);
        assert!(calls_to(&e, process_slot(0x294)).is_empty());
        // The face data, not busy, is asked to finish.
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, 0xd8)),
            vec![vec![face, 1, 0]]
        );
        // Busy face data is left alone.
        give_vtable(&mut e, face, OTHER_TABLE, &[(0xd4, 1)]);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert!(calls_to(&e, slot_target(OTHER_TABLE, 0xd8)).is_empty());
    }

    #[test]
    fn test_high_process_process_sleep_state_4_stops() {
        let mut e = Engine::new();
        let (p, actor) = sleep_world(&mut e);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x214, 4)]);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert_eq!(calls_to(&e, actor_slot(0x418)), vec![vec![actor]]);
        assert!(calls_to(&e, LIST_CLEAR).is_empty());
        assert!(calls_to(&e, process_slot(0x288)).is_empty());
    }

    #[test]
    fn test_high_process_process_sleep_picks_a_free_furniture() {
        let mut e = Engine::new();
        let (p, actor) = sleep_world(&mut e);
        let list = p.addr() + 0xd0;
        // The first furniture has no free marker, the second one has and no
        // owner.
        e.mem.set_u32(SLEEP_FURNITURE_B + 0x10, 1);
        fill_list(&mut e, list, &[SLEEP_FURNITURE_A, SLEEP_FURNITURE_B]);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert_eq!(e.mem.u32(p.addr() + 0x140), SLEEP_FURNITURE_B);
        assert_eq!(
            calls_to(&e, REFERENCE_HAS_FREE_MARKER),
            vec![vec![SLEEP_FURNITURE_A, 0], vec![SLEEP_FURNITURE_B, 0]]
        );
        assert_eq!(
            calls_to(&e, LIST_REMOVE_ITEM),
            vec![vec![list, p.addr() + 0x140]]
        );
        // The marker is cleared after each pick, the gate checked and released.
        assert_eq!(calls_to(&e, PROCESS_CLEAR_FURNITURE_MARKER).len(), 4);
        assert_eq!(
            calls_to(&e, FURNITURE_GATE_CHECK),
            vec![vec![FURNITURE_GATE_OBJECT]]
        );
        assert_eq!(
            calls_to(&e, FURNITURE_GATE_RELEASE),
            vec![vec![FURNITURE_GATE_OBJECT]]
        );
        // The furniture becomes the target and is activated.
        assert_eq!(
            calls_to(&e, process_slot(0x12c)),
            vec![vec![p.addr(), SLEEP_FURNITURE_B]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x7c8)),
            vec![vec![p.addr(), actor, 0]]
        );
        // With a target already set the process is not given one.
        e.mem.set_u32(p.addr() + 0x40, 0x4444);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x12c)).is_empty());
        assert_eq!(calls_to(&e, process_slot(0x7c8)).len(), 1);
    }

    #[test]
    fn test_high_process_process_sleep_random_furniture_when_unowned() {
        let mut e = Engine::new();
        let (p, actor) = sleep_world(&mut e);
        let list = p.addr() + 0xd0;
        e.mem.set_u32(SLEEP_FURNITURE_A + 0x10, 1);
        e.mem.set_u32(SLEEP_FURNITURE_B + 0x10, 1);
        fill_list(&mut e, list, &[SLEEP_FURNITURE_A, SLEEP_FURNITURE_B]);
        // Both are free and unowned: the random number (5) modulo the count
        // (2) is 1, the second furniture.
        e.call(0x008e26e0, &args![p, actor]);
        assert_eq!(e.mem.u32(p.addr() + 0x140), SLEEP_FURNITURE_B);
        // An owned furniture is kept as picked.
        e.mem.set_u32(p.addr() + 0x140, 0);
        e.mem.set_u32(SLEEP_FURNITURE_A + 0x14, 0x99);
        fill_list(&mut e, list, &[SLEEP_FURNITURE_A, SLEEP_FURNITURE_B]);
        e.call(0x008e26e0, &args![p, actor]);
        assert_eq!(e.mem.u32(p.addr() + 0x140), SLEEP_FURNITURE_A);
        // The gate closed: nothing is picked.
        e.mem.set_u32(p.addr() + 0x140, 0);
        returning(&mut e, FURNITURE_GATE_CHECK, 0);
        fill_list(&mut e, list, &[SLEEP_FURNITURE_A, SLEEP_FURNITURE_B]);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert_eq!(e.mem.u32(p.addr() + 0x140), 0);
        assert!(calls_to(&e, FURNITURE_GATE_RELEASE).is_empty());
    }

    #[test]
    fn test_high_process_process_sleep_without_candidates() {
        let mut e = Engine::new();
        let (p, actor) = sleep_world(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        // Nothing to sleep on: the movement ends, the timer is set from the
        // game constant and procedure 1 is added.
        assert_eq!(
            calls_to(&e, process_slot(0x294)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(e.mem.f32(p.addr() + 0x2e0), 20.0);
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert!(calls_to(&e, PROCESS_WANTS_TO_FINISH_SLEEP).is_empty());
        // The beds are searched for when the list is empty.
        assert_eq!(calls_to(&e, PROCESS_FIND_BED_CHAIRS).len(), 1);
    }

    #[test]
    fn test_high_process_process_sleep_finishes_when_the_process_says_so() {
        let mut e = Engine::new();
        let (p, actor) = sleep_world(&mut e);
        let list = p.addr() + 0xd0;
        e.mem.set_u32(p.addr() + 0x140, SLEEP_FURNITURE_A);
        e.mem.set_u32(p.addr() + 0x40, 0x4444);
        fill_list(&mut e, list, &[SLEEP_FURNITURE_B]);
        returning(&mut e, PROCESS_WANTS_TO_FINISH_SLEEP, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, PROCESS_WANTS_TO_FINISH_SLEEP),
            vec![vec![p.addr(), actor]]
        );
        let order = call_order(&e);
        let tail: Vec<u32> = order[order.len() - 3..].to_vec();
        assert_eq!(
            tail,
            vec![process_slot(0x84), process_slot(0x294), process_slot(0x288)]
        );
        // With a furniture in hand the time global does not matter and the
        // beds are not searched for.
        assert!(calls_to(&e, PROCESS_FIND_BED_CHAIRS).is_empty());
        // The furniture is activated.
        assert_eq!(
            calls_to(&e, process_slot(0x7c8)),
            vec![vec![p.addr(), actor, 0]]
        );
    }

    #[test]
    fn test_high_process_process_sleep_searches_for_beds_by_the_time_global() {
        let mut e = Engine::new();
        let (p, actor) = sleep_world(&mut e);
        let list = p.addr() + 0xd0;
        fill_list(&mut e, list, &[SLEEP_FURNITURE_A]);
        e.mem.set_u32(SLEEP_FURNITURE_A + 0x10, 0);
        returning(&mut e, FURNITURE_GATE_CHECK, 0);
        // The global (100) is below the timer (+0x258): no search.
        e.mem.set_u32(p.addr() + 0x258, 101);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert!(calls_to(&e, PROCESS_FIND_BED_CHAIRS).is_empty());
        // Equal or above: search.
        e.mem.set_u32(p.addr() + 0x258, 100);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert_eq!(calls_to(&e, PROCESS_FIND_BED_CHAIRS).len(), 1);
        // A process with a sit/sleep state (virtual 0x4bc) does not search.
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x4bc, 1)]);
        e.call_log = Some(vec![]);
        e.call(0x008e26e0, &args![p, actor]);
        assert!(calls_to(&e, PROCESS_FIND_BED_CHAIRS).is_empty());
    }

    const ACQUIRE_ADDED: u32 = 0x7a50_0000;
    const REFERENCE_TABLE_BASE: u32 = 0x7e00_0000;

    /// A reference with its own vtable (number `index`): `is_actor`, `npc`
    /// (slot 0x218) and `dead` (slot 0x22c).
    fn acquire_reference(e: &mut Engine, index: u32, is_actor: bool, npc: bool, dead: bool) -> u32 {
        let table = REFERENCE_TABLE_BASE + index * 0x1000;
        let reference = object_with_slots(
            e,
            table,
            &[
                (0x100, u32::from(is_actor)),
                (0x218, u32::from(npc)),
                (0x22c, u32::from(dead)),
            ],
        );
        // The owner word is read by the owner double.
        e.mem.set_u32(reference + 0x14, 0);
        reference
    }

    /// A form: its type byte is at +4.
    fn acquire_form(e: &mut Engine, form_type: u8) -> u32 {
        let form = e.mem.alloc(0x20);
        e.mem.set_u8(form + 4, form_type);
        form
    }

    /// A `ObjectstoAcquire` entry for `reference` and `form`.
    fn acquire_entry(e: &mut Engine, reference: u32, form: u32) -> u32 {
        let entry = e.mem.alloc(0x40);
        e.mem.set_u32(entry, reference);
        e.mem.set_u32(entry + 4, form);
        entry
    }

    /// An entry for `reference` with a new form of type `form_type`.
    fn acquire_typed_entry(e: &mut Engine, reference: u32, form_type: u8) -> u32 {
        let form = acquire_form(e, form_type);
        acquire_entry(e, reference, form)
    }

    /// The world of `EvaluateOrderAcquireList`: a package without flags, an
    /// actor that is neither creature-like nor forced, every test answering no.
    fn acquire_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        let p = block(e);
        let package = e.mem.alloc(0x40);
        give_vtable(e, p.addr(), PROCESS_TABLE, &[(0x27c, package)]);
        let actor = object_with_slots(e, ACTOR_TABLE, &[(0x21c, 0)]);
        list_doubles(e);
        e.register(FORM_TYPE_BYTE, |e, a| ret(u32::from(e.mem.u8(a[0] + 4))));
        e.register(REFERENCE_GET_OWNER, |e, a| ret(e.mem.u32(a[0] + 0x14)));
        returning(e, REFERENCE_IS_AN_OWNER, 0);
        returning(e, PROCESS_GET_FORCE_NEXT_UPDATE, 0);
        returning(e, ACTOR_CREATURE_LIKE, 0);
        returning(e, REFERENCE_GET_FORM, 0x9999);
        returning(e, REFERENCE_GET_LOCK, 0);
        returning(e, ACTOR_HAS_OBJECTS, 0);
        global_word(e, KEY_FORM_GLOBAL, 0x1111);
        returning_float(e, DISTANCE_FROM_REFERENCE, 123.9);
        e.register(FLOAT_TO_INT, |_, a| {
            let value = f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32);
            ret(value as i32 as u32)
        });
        stub(
            e,
            &[
                REFERENCE_SET_TARGETED,
                ACQUIRE_OBJECT_DELETE,
                PROCESS_EVALUATION_END_A,
                PROCESS_EVALUATION_END_B,
                LIST_CLEAR,
            ],
        );
        // The entries added to the object list are recorded.
        e.map(ACQUIRE_ADDED, 0x100);
        e.register(LIST_ADD_ITEM, |e, a| {
            let count = e.mem.u32(ACQUIRE_ADDED);
            let item = e.mem.u32(a[1]);
            e.mem.set_u32(ACQUIRE_ADDED + 4 + 4 * count, item);
            e.mem.set_u32(ACQUIRE_ADDED, count + 1);
            Ret::default()
        });
        (p, package, actor)
    }

    /// The entries added so far.
    fn acquire_added(e: &Engine) -> Vec<u32> {
        (0..e.mem.u32(ACQUIRE_ADDED))
            .map(|i| e.mem.u32(ACQUIRE_ADDED + 4 + 4 * i))
            .collect()
    }

    #[test]
    fn test_high_process_evaluate_order_acquire_list_keeps_special_forms_and_measures_the_rest() {
        let mut e = Engine::new();
        let (p, _package, actor) = acquire_world(&mut e);
        // Entry 1: a form of type 0x2a is kept as it is. Entry 2: type 0x2b
        // too. Entry 3: an unowned object only gets its distance.
        let special_a_reference = acquire_reference(&mut e, 1, false, false, false);
        let special_a = acquire_typed_entry(&mut e, special_a_reference, 0x2a);
        let special_b_reference = acquire_reference(&mut e, 2, false, false, false);
        let special_b = acquire_typed_entry(&mut e, special_b_reference, 0x2b);
        let plain_reference = acquire_reference(&mut e, 3, false, false, false);
        let plain = acquire_typed_entry(&mut e, plain_reference, 0x10);
        fill_list(&mut e, p.addr() + 0x74, &[special_a, special_b, plain]);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(acquire_added(&e), vec![special_a, special_b, plain]);
        assert_eq!(e.mem.u32(plain + 0x10), 123, "the distance is truncated");
        assert_eq!(
            calls_to(&e, DISTANCE_FROM_REFERENCE),
            vec![vec![actor, plain_reference, 0, 0]]
        );
        assert_eq!(calls_to(&e, REFERENCE_SET_TARGETED).len(), 3);
        assert_eq!(
            calls_to(&e, REFERENCE_SET_TARGETED)[2],
            vec![plain_reference, 1]
        );
        // The hooks run, then the temporary list is cleared.
        let order = call_order(&e);
        let tail = &order[order.len() - 3..];
        assert_eq!(
            tail,
            [
                PROCESS_EVALUATION_END_A,
                PROCESS_EVALUATION_END_B,
                LIST_CLEAR
            ]
        );
        assert_eq!(
            calls_to(&e, LIST_CLEAR).last().unwrap(),
            &vec![p.addr() + 0x74]
        );
        assert!(calls_to(&e, ACQUIRE_OBJECT_DELETE).is_empty());
    }

    #[test]
    fn test_high_process_evaluate_order_acquire_list_package_flags() {
        let mut e = Engine::new();
        let (p, package, actor) = acquire_world(&mut e);
        // A dead NPC with no owner: kind 0, kept. (First entry.)
        let dead_npc = acquire_reference(&mut e, 1, true, true, true);
        let dead = acquire_typed_entry(&mut e, dead_npc, 0x10);
        // A live NPC with the package flags 0x400 and 0x200: both flag bytes
        // are set, kind 4 (second entry).
        let npc = acquire_reference(&mut e, 2, true, true, false);
        let live = acquire_typed_entry(&mut e, npc, 0x10);
        // A plain object owned by someone else (a form of another type); the
        // package has the flag 0x400 and the entry the kind 1.
        let object = acquire_reference(&mut e, 3, false, false, false);
        let someone = acquire_form(&mut e, 0x10);
        e.mem.set_u32(object + 0x14, someone);
        let owned = acquire_typed_entry(&mut e, object, 0x10);
        e.mem.set_u32(owned + 0x18, 1);
        fill_list(&mut e, p.addr() + 0x74, &[dead, live, owned]);
        e.mem.set_u16(package + 0x24, 0x600);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(e.mem.u32(dead + 0x18), 0, "kind 0");
        assert_eq!(e.mem.u32(live + 0x18), 4);
        assert_eq!(e.mem.u8(live + 8), 1);
        assert_eq!(e.mem.u8(live + 9), 1);
        // The object had kind 1: it becomes 4 (it is not locked, so it stays).
        assert_eq!(e.mem.u32(owned + 0x18), 4);
        assert_eq!(e.mem.u8(owned + 8), 1);
        assert_eq!(e.mem.u8(owned + 9), 0);
        assert_eq!(acquire_added(&e), vec![dead, live, owned]);
    }

    #[test]
    fn test_high_process_evaluate_order_acquire_list_drops_what_is_locked() {
        let mut e = Engine::new();
        let (p, package, actor) = acquire_world(&mut e);
        let someone = acquire_form(&mut e, 0x10);
        let object = acquire_reference(&mut e, 3, false, false, false);
        e.mem.set_u32(object + 0x14, someone);
        let locked = acquire_typed_entry(&mut e, object, 0x10);
        e.mem.set_u32(locked + 0x18, 1);
        let other_object = acquire_reference(&mut e, 4, false, false, false);
        e.mem.set_u32(other_object + 0x14, someone);
        let unlocked = acquire_typed_entry(&mut e, other_object, 0x10);
        e.mem.set_u32(unlocked + 0x18, 2);
        fill_list(&mut e, p.addr() + 0x74, &[locked, unlocked]);
        e.mem.set_u16(package + 0x24, 0x400);
        returning(&mut e, REFERENCE_GET_LOCK, 0x66);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        // Kind 1 with a lock and no key: dropped (deleted with the flag 1).
        assert_eq!(calls_to(&e, ACQUIRE_OBJECT_DELETE), vec![vec![locked, 1]]);
        // Another kind: kind 3, kept.
        assert_eq!(e.mem.u32(unlocked + 0x18), 3);
        assert_eq!(acquire_added(&e), vec![unlocked]);
        // The key is looked for among the actor's items.
        let objects = calls_to(&e, ACTOR_HAS_OBJECTS);
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0][..5], [actor, 0x1111, 0, 1, 0]);
    }

    #[test]
    fn test_high_process_evaluate_order_acquire_list_prices_what_an_owner_buys() {
        let mut e = Engine::new();
        let (p, package, actor) = acquire_world(&mut e);
        e.mem.set_u16(package + 0x24, 0x100);
        // A non-actor object owned by a form of type 0x2a whose actor buys it.
        let owner_form = acquire_form(&mut e, 0x2a);
        let object = acquire_reference(&mut e, 3, false, false, false);
        e.mem.set_u32(object + 0x14, owner_form);
        let form = acquire_form(&mut e, 0x10);
        let entry = acquire_entry(&mut e, object, form);
        fill_list(&mut e, p.addr() + 0x74, &[entry]);
        let owner_actor = acquire_reference(&mut e, 5, true, true, false);
        e.register_double(PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH, move |_, _| {
            ret(owner_actor)
        });
        let ai_form = e.mem.alloc(0x100);
        returning(&mut e, REFERENCE_AI_FORM, ai_form);
        returning(&mut e, AI_FORM_SELL_BUYS_ITEM, 1);
        returning(&mut e, PACKAGE_OBJECT_TYPE_FROM_FORM, 5);
        returning(&mut e, ACTOR_GOLD_AMOUNT, 100);
        returning(&mut e, FORM_VALUE, 50);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(e.mem.u32(entry + 0x18), 2);
        assert_eq!(e.mem.u32(entry + 0x10), 123);
        assert_eq!(acquire_added(&e), vec![entry]);
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_GET_ACTOR_REF_IN_HIGH),
            vec![vec![PROCESS_LISTS_INSTANCE, owner_form, 0]]
        );
        assert_eq!(
            calls_to(&e, AI_FORM_SELL_BUYS_ITEM),
            vec![vec![ai_form + 0x90, form]]
        );
        assert_eq!(calls_to(&e, REFERENCE_AI_FORM), vec![vec![owner_actor]]);
        // Too expensive: the entry is dropped (nothing else claims it).
        returning(&mut e, FORM_VALUE, 150);
        e.mem.set_u32(entry + 0x18, 0);
        e.mem.set_u32(ACQUIRE_ADDED, 0);
        fill_list(&mut e, p.addr() + 0x74, &[entry]);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(calls_to(&e, ACQUIRE_OBJECT_DELETE), vec![vec![entry, 1]]);
        // Object type 0x12 is free: priced again.
        returning(&mut e, PACKAGE_OBJECT_TYPE_FROM_FORM, 0x12);
        e.mem.set_u32(ACQUIRE_ADDED, 0);
        fill_list(&mut e, p.addr() + 0x74, &[entry]);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(e.mem.u32(entry + 0x18), 2);
        assert!(calls_to(&e, ACTOR_GOLD_AMOUNT).is_empty());
        // A creature-like actor drops it.
        returning(&mut e, ACTOR_CREATURE_LIKE, 1);
        e.mem.set_u32(ACQUIRE_ADDED, 0);
        fill_list(&mut e, p.addr() + 0x74, &[entry]);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(calls_to(&e, ACQUIRE_OBJECT_DELETE), vec![vec![entry, 1]]);
    }

    #[test]
    fn test_high_process_evaluate_order_acquire_list_forced_and_creature_actors() {
        let mut e = Engine::new();
        let (p, _package, actor) = acquire_world(&mut e);
        // A live actor reference that is forced to update is dropped; one
        // that is not and has no flags goes to the last tests: the evaluating
        // actor's slot 0x21c says no and the reference is an NPC: dropped.
        let npc = acquire_reference(&mut e, 2, true, true, false);
        let entry = acquire_typed_entry(&mut e, npc, 0x10);
        fill_list(&mut e, p.addr() + 0x74, &[entry]);
        returning(&mut e, PROCESS_GET_FORCE_NEXT_UPDATE, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(calls_to(&e, ACQUIRE_OBJECT_DELETE), vec![vec![entry, 1]]);
        // Not forced: with no package flags the entry reaches the tail tests;
        // an NPC reference is dropped there.
        returning(&mut e, PROCESS_GET_FORCE_NEXT_UPDATE, 0);
        fill_list(&mut e, p.addr() + 0x74, &[entry]);
        e.call_log = Some(vec![]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(calls_to(&e, ACQUIRE_OBJECT_DELETE).len(), 1);
        // A creature evaluating (slot 0x21c) drops it as well.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x21c, 1)]);
        e.call_log = Some(vec![]);
        fill_list(&mut e, p.addr() + 0x74, &[entry]);
        e.call(0x008e2b00, &args![p, actor]);
        assert_eq!(calls_to(&e, ACQUIRE_OBJECT_DELETE).len(), 1);
    }

    #[test]
    fn test_high_process_guard_call_for_help() {
        let mut e = Engine::new();
        let process = block(&mut e);
        let position = e.mem.alloc(0x10);
        let actor = object_with_slots(&mut e, ACTOR_TABLE, &[(0x1f4, position)]);
        // The radius setting holds the integer 600.
        let setting = e.mem.alloc(0x10);
        e.mem.set_i32(setting, 600);
        returning(&mut e, SETTING_VALUE_ADDRESS, setting);
        returning(&mut e, REFERENCE_PATHING_CELL, 0x9000);
        global_word(&mut e, DATA_HANDLER_POINTER, 0x5000_0000);
        stub(&mut e, &[ENUM_REFERENCES_CLOSE_TO_POINT]);
        // Two references found: an actor in a process and a non-actor.
        let found = e.mem.alloc(8);
        let thing = object_with_slots(&mut e, OTHER_TABLE, &[(0x100, 0)]);
        let helper = object_with_slots(&mut e, ACTOR_TABLE + 0x1000, &[(0x100, 1)]);
        fill_list(&mut e, found, &[thing, helper]);
        returning(&mut e, PROCESS_LISTS_FIND_ACTORS, found);
        e.register(NODE_ITEM_ADDRESS, |_, a| ret(a[0]));
        e.register(NODE_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        returning(&mut e, MOBILE_OBJECT_PROCESS_TYPE, 3);
        returning_float(&mut e, CALENDAR_GET_HOUR, 14.5);
        global_f64(&mut e, ONE_HOUR_BACK, 1.0);
        returning(&mut e, ACTOR_PROCESS, 0x4000);
        stub(
            &mut e,
            &[
                PROCESS_SET_TIME_FLOAT,
                PROCESS_LISTS_REMOVE_REFERENCE,
                PROCESS_LISTS_ADD_REFERENCE,
                LIST_CLEAR,
                LIST_DELETE,
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x008e3f00, &args![process, actor, 0x1234u32]);
        // The references near the actor are enumerated with the callback.
        let enumerated = calls_to(&e, ENUM_REFERENCES_CLOSE_TO_POINT);
        assert_eq!(
            enumerated,
            vec![vec![
                0x5000_0000,
                0x9000,
                position,
                600.0f32.to_bits(),
                position,
                600.0f32.to_bits(),
                0x008e_3ea0,
                actor
            ]]
        );
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_FIND_ACTORS),
            vec![vec![PROCESS_LISTS_INSTANCE, 0x1234, REFERENCE_LIST]]
        );
        // Only the actor is moved: its process gets the hour minus 1, and it
        // leaves and re-enters the process lists with its process type.
        assert_eq!(
            calls_to(&e, PROCESS_SET_TIME_FLOAT),
            vec![vec![0x4000, 13.5f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_REMOVE_REFERENCE),
            vec![vec![PROCESS_LISTS_INSTANCE, helper, 3]]
        );
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_ADD_REFERENCE),
            vec![vec![PROCESS_LISTS_INSTANCE, helper, 3, 0, 0, 0]]
        );
        // The found list is cleared and deleted, then the shared list cleared.
        assert_eq!(
            calls_to(&e, LIST_CLEAR),
            vec![vec![found], vec![REFERENCE_LIST]]
        );
        assert_eq!(calls_to(&e, LIST_DELETE), vec![vec![found, 1]]);
        // An actor that is in no process is left alone.
        returning(&mut e, MOBILE_OBJECT_PROCESS_TYPE, 0);
        fill_list(&mut e, found, &[helper]);
        e.call_log = Some(vec![]);
        e.call(0x008e3f00, &args![process, actor, 0x1234u32]);
        assert!(calls_to(&e, PROCESS_LISTS_REMOVE_REFERENCE).is_empty());
        // No list found: no deletion.
        returning(&mut e, PROCESS_LISTS_FIND_ACTORS, 0);
        e.call_log = Some(vec![]);
        e.call(0x008e3f00, &args![process, actor, 0x1234u32]);
        assert!(calls_to(&e, LIST_DELETE).is_empty());
        assert_eq!(calls_to(&e, LIST_CLEAR), vec![vec![REFERENCE_LIST]]);
    }

    const PACKAGE_TABLE: u32 = 0x7f00_0000;
    const ESCORT_ITEM_SCRATCH: u32 = 0x7f20_0000;

    /// The world of `ProcessEscort`: the world of `ShouldWaitForEscortTarget`
    /// (the squared distance is huge) plus a package whose slot `0x13c` says
    /// no, no inventory item, no package target, no package location, a
    /// pathing-complete actor and a process target that is `0x1234` (not set
    /// up as an actor). Returns `(process, actor, escorted actor, package)`.
    fn escort_run_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32, u32) {
        let (p, actor, escorted, _location, _flag) = escort_world(e);
        let package = object_with_slots(e, PACKAGE_TABLE, &[(0x13c, 0)]);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (0x27c, package),
                (0x8c, 0),
                (0x288, 0),
                (0x294, 0),
                (0x284, 0),
                (0x660, 0),
                (0x118, 0),
                (0x24, 0),
                (0x120, 0),
                (0x34c, 0),
                (0x350, 0),
                (0x12c, 0),
                (0x22c, 0),
            ],
        );
        give_vtable(
            e,
            actor,
            ACTOR_TABLE,
            &[
                (0x1e4, 0),
                (0x214, 0),
                (0x218, 0),
                (0x21c, 0),
                (0x418, 0),
                (0x3cc, 0),
                (0x100, 1),
            ],
        );
        give_vtable(e, escorted, OTHER_TABLE, &[(0x100, 1)]);
        e.mem.set_u32(p.addr() + 0x40, 0);
        returning(e, ACTOR_PACKAGE_ITEM, 0);
        returning(e, WORD_AT_0XC, 0);
        returning(e, REFERENCE_HOLDS_OBJECT, 0);
        returning(e, ACTOR_IS_PATHING_COMPLETE, 1);
        returning(e, PACKAGE_TYPE, 0);
        returning(e, PACKAGE_GET_IS_CREATED, 0);
        returning(e, PACKAGE_TARGET_WORD, 0);
        returning(e, PACKAGE_TARGET_GET_TYPE, 1);
        returning(e, REFERENCE_FLAG_800, 0);
        returning(e, REFERENCE_FLAG_20, 0);
        returning(e, REFERENCE_EXTRA_DATA, 0x3300);
        returning(e, EXTRA_DATA_REFERENCE_POINTER, 0);
        returning(e, ACTOR_IS_PLAYING_LOWER_BODY_SPECIAL_IDLE, 0);
        returning(e, PACKAGE_LOCATION_WORD, 0);
        returning(e, PACKAGE_LOCATION_WORLD, 0x111);
        returning(e, PACKAGE_LOCATION_CELL, 0x222);
        returning(e, ACTOR_CURRENT_PACKAGE, 0);
        returning(e, PACKAGE_IS_INTERRUPT, 0);
        returning_float(e, PACKAGE_RADIUS_ACTOR_TO_LOCATION, 12.5);
        returning(e, PROCESS_IS_CURRENT_WEAPON_GRENADE, 0);
        returning(e, PACKAGE_TEST_67A480, 0);
        returning(e, ANIMATION_SPECIAL_IDLE_DONE_PLAYING, 1);
        e.register(PACKAGE_LOCATION_COORD, |e, a| {
            // Returns the output buffer, as `GetLocationCoord` does.
            e.mem.set_u32(a[1], 0x333);
            ret(a[1])
        });
        e.register_double(ACTOR_PROCESS, |_, _| ret(0));
        stub(
            e,
            &[
                ACTOR_CLEAR_LOOK_AT_TARGET,
                PACKAGE_SET_NEVER_RUN,
                ANIMATION_SPECIAL_IDLE_FREE,
                ITEM_DELETE,
                ACTOR_BUILD_REQUEST,
                ACTOR_SET_PATHFINDING_GOAL_REQUEST,
                PATHING_REQUEST_DESTRUCTOR,
                FIND_SPECIAL_IDLE_TO_PLAY,
                ACTOR_END_MOVEMENT,
                ACTOR_PUT_IN_CONTAINER,
                NPC_FORM_EQUIP,
                CREATURE_INIT_DEFAULT_WORN,
                ACQUIRE_OBJECT_DELETE,
                PATHING_LOCATION_FROM_COORD,
            ],
        );
        global_f32(e, NO_LIMIT_FLOAT, -1.0);
        (p, actor, escorted, package)
    }

    #[test]
    fn test_high_process_process_escort_without_item_or_target() {
        let mut e = Engine::new();
        let (p, actor, _escorted, _package) = escort_run_world(&mut e);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            call_order(&e),
            vec![
                process_slot(0x27c),
                ACTOR_PACKAGE_ITEM,
                process_slot(0x8c),
                process_slot(0x288),
                ACTOR_IS_PATHING_COMPLETE,
                process_slot(0x294)
            ]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x8c)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x294)),
            vec![vec![p.addr(), actor]]
        );
        // Pathing complete: no end-of-movement message.
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x294)).is_empty());
    }

    #[test]
    fn test_high_process_process_escort_builds_a_request_to_the_package_location() {
        let mut e = Engine::new();
        let (p, actor, escorted, package) = escort_run_world(&mut e);
        // A target that is not an actor but holds the item: the process goes
        // on; there is no package location, so a request is built.
        give_vtable(&mut e, escorted, OTHER_TABLE, &[(0x100, 0)]);
        e.mem.set_u32(p.addr() + 0x40, escorted);
        returning(&mut e, WORD_AT_0XC, 7);
        returning(&mut e, REFERENCE_HOLDS_OBJECT, 1);
        e.map(ESCORT_ITEM_SCRATCH, 0x100);
        e.register(ACTOR_SET_PATHFINDING_GOAL_REQUEST, |e, a| {
            // The byte `008e0ab0` set, as the goal sees it.
            let flag = e.mem.u8(a[1] + 0x9f);
            e.mem.set_u8(ESCORT_ITEM_SCRATCH, flag);
            Ret::default()
        });
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, REFERENCE_HOLDS_OBJECT), vec![vec![actor, 7]]);
        assert!(calls_to(&e, process_slot(0x288)).is_empty());
        let build = calls_to(&e, ACTOR_BUILD_REQUEST);
        assert_eq!(build.len(), 1);
        assert_eq!(
            (
                build[0][0],
                build[0][3],
                build[0][4],
                build[0][5],
                build[0][6]
            ),
            (actor, 0x222, 0x111, 12.5f32.to_bits(), 0)
        );
        assert_eq!(
            calls_to(&e, PACKAGE_RADIUS_ACTOR_TO_LOCATION),
            vec![vec![package, actor, 0]]
        );
        let goal = calls_to(&e, ACTOR_SET_PATHFINDING_GOAL_REQUEST);
        assert_eq!(goal, vec![vec![actor, build[0][1]]]);
        assert_eq!(
            calls_to(&e, PATHING_REQUEST_DESTRUCTOR),
            vec![vec![build[0][1]]]
        );
        // The byte at +0x9f says whether the escorted actor is the player.
        assert_eq!(e.mem.u8(ESCORT_ITEM_SCRATCH), 0);
        // An actor in state 4 or 9 calls its slot 0x418 instead.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x214, 4)]);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, ACTOR_BUILD_REQUEST).is_empty());
        assert_eq!(calls_to(&e, actor_slot(0x418)), vec![vec![actor]]);
        // A pathing actor is left alone.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x214, 0)]);
        returning(&mut e, ACTOR_IS_PATHING, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, ACTOR_BUILD_REQUEST).is_empty());
        assert!(calls_to(&e, actor_slot(0x418)).is_empty());
    }

    /// Makes `escorted` the target of the process, with a process that has a
    /// running package of type 1 (so it is not bailed out of).
    fn escort_with_escorted_actor(
        e: &mut Engine,
        p: Ptr<HighProcess>,
        escorted: u32,
        player_is_escorted: bool,
    ) -> u32 {
        e.mem.set_u32(p.addr() + 0x40, escorted);
        let target_process =
            object_with_slots(e, TARGET_PROCESS_TABLE, &[(0x22c, 0x9c00), (0x27c, 0x9b00)]);
        e.register_double(ACTOR_PROCESS, move |_, a| {
            ret(if a[0] == escorted { target_process } else { 0 })
        });
        returning(e, PACKAGE_TYPE, 1);
        if player_is_escorted {
            global_word(e, PLAYER_POINTER, escorted);
        }
        target_process
    }

    #[test]
    fn test_high_process_process_escort_ends_when_the_escorted_actor_has_no_package() {
        let mut e = Engine::new();
        let (p, actor, escorted, package) = escort_run_world(&mut e);
        let target_process = escort_with_escorted_actor(&mut e, p, escorted, false);
        // The escorted actor's process has no current package.
        give_vtable(&mut e, target_process, TARGET_PROCESS_TABLE, &[(0x22c, 0)]);
        returning(&mut e, ACTOR_PACKAGE_ITEM, 0x8800);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x284)), vec![vec![p.addr(), 0]]);
        assert_eq!(calls_to(&e, ITEM_DELETE), vec![vec![0x8800, 1]]);
        assert!(calls_to(&e, process_slot(0x660)).is_empty());
        assert!(calls_to(&e, PACKAGE_TARGET_WORD).is_empty());
        // A package of another type that is not created ends it too; one that
        // was created by a script does not.
        give_vtable(
            &mut e,
            target_process,
            TARGET_PROCESS_TABLE,
            &[(0x22c, 0x9c00)],
        );
        returning(&mut e, PACKAGE_TYPE, 3);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x284)).len(), 1);
        returning(&mut e, PACKAGE_GET_IS_CREATED, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x284)).is_empty());
        // The player is never bailed out of; the head tracking is cleared and
        // the actor stops looking at it.
        returning(&mut e, PACKAGE_GET_IS_CREATED, 0);
        global_word(&mut e, PLAYER_POINTER, escorted);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x660)), vec![vec![p.addr()]]);
        assert_eq!(calls_to(&e, ACTOR_CLEAR_LOOK_AT_TARGET), vec![vec![actor]]);
        let _ = package;
    }

    #[test]
    fn test_high_process_process_escort_package_target_of_a_vanished_reference() {
        let mut e = Engine::new();
        let (p, actor, escorted, package) = escort_run_world(&mut e);
        escort_with_escorted_actor(&mut e, p, escorted, false);
        // A package target of type 0 (a reference).
        returning(&mut e, PACKAGE_TARGET_WORD, 0x6600);
        returning(&mut e, PACKAGE_TARGET_GET_TYPE, 0);
        returning(&mut e, REFERENCE_FLAG_800, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, PACKAGE_TARGET_GET_TYPE), vec![vec![0x6600]]);
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert!(calls_to(&e, PACKAGE_SET_NEVER_RUN).is_empty());
        // The flag 0x20 and no reference pointer: the package never runs.
        returning(&mut e, REFERENCE_FLAG_800, 0);
        returning(&mut e, REFERENCE_FLAG_20, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, PACKAGE_SET_NEVER_RUN),
            vec![vec![package, escorted, 1]]
        );
        assert_eq!(
            calls_to(&e, EXTRA_DATA_REFERENCE_POINTER),
            vec![vec![0x3300]]
        );
        assert_eq!(calls_to(&e, process_slot(0x288)).len(), 1);
        // With a reference pointer the escort goes on.
        returning(&mut e, EXTRA_DATA_REFERENCE_POINTER, 0x4400);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, PACKAGE_SET_NEVER_RUN).is_empty());
        assert!(calls_to(&e, process_slot(0x288)).is_empty());
        // A target type other than 0 or 3 is not looked at.
        returning(&mut e, PACKAGE_TARGET_GET_TYPE, 1);
        returning(&mut e, EXTRA_DATA_REFERENCE_POINTER, 0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, REFERENCE_FLAG_800).is_empty());
    }

    #[test]
    fn test_high_process_process_escort_keeps_pace_with_the_escorted_actor() {
        let mut e = Engine::new();
        let (p, actor, escorted, package) = escort_run_world(&mut e);
        escort_with_escorted_actor(&mut e, p, escorted, false);
        // Close together: nothing to wait for. The move mode is asked for.
        returning_float(&mut e, POINT_SQUARED_LENGTH, 100.0);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 1);
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x34c, 0x300)]);
        e.map(ESCORT_ITEM_SCRATCH, 0x100);
        e.register(ACTOR_SET_PATHFINDING_GOAL_REQUEST, |e, a| {
            let flag = e.mem.u8(a[1] + 0x9f);
            e.mem.set_u8(ESCORT_ITEM_SCRATCH, flag);
            Ret::default()
        });
        global_word(&mut e, PLAYER_POINTER, escorted);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x34c)),
            vec![vec![
                p.addr(),
                actor,
                0,
                12.5f32.to_bits(),
                25.0f32.to_bits(),
                0,
                0
            ]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x350)),
            vec![vec![p.addr(), actor, 0x300, 0]]
        );
        // Not waiting, so the request to the package location is built, and
        // the byte at +0x9f is 1 since the escorted actor is the player.
        assert_eq!(calls_to(&e, ACTOR_BUILD_REQUEST).len(), 1);
        assert_eq!(e.mem.u8(ESCORT_ITEM_SCRATCH), 1);
        let radius = calls_to(&e, PACKAGE_RADIUS_ACTOR_TO_LOCATION);
        assert_eq!(radius.len(), 2, "once for the move mode, once for the goal");
        let _ = package;
        // The wait flag (0x44444 * 80000 < squared distance < the radius
        // squared) changes the flags: 0x200 off, 0x100 on.
        global_word(&mut e, PLAYER_POINTER, 0x9999);
        returning_float(&mut e, POINT_SQUARED_LENGTH, 100_000.0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x350)),
            vec![vec![p.addr(), actor, 0x100, 0]]
        );
        // A grenade (or the package test 0067a480) turns 0x100 off, 0x200 on.
        returning_float(&mut e, POINT_SQUARED_LENGTH, 100.0);
        returning(&mut e, PROCESS_IS_CURRENT_WEAPON_GRENADE, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, PROCESS_IS_CURRENT_WEAPON_GRENADE),
            vec![vec![actor]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x350)),
            vec![vec![p.addr(), actor, 0x200, 0]]
        );
        returning(&mut e, PROCESS_IS_CURRENT_WEAPON_GRENADE, 0);
        returning(&mut e, PACKAGE_TEST_67A480, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(0x350)),
            vec![vec![p.addr(), actor, 0x200, 0]]
        );
    }

    #[test]
    fn test_high_process_process_escort_waits_while_the_escorted_actor_is_far() {
        let mut e = Engine::new();
        let (p, actor, escorted, _package) = escort_run_world(&mut e);
        escort_with_escorted_actor(&mut e, p, escorted, false);
        // Far apart: the escort waits (should wait is true). While the actor
        // is still moving it stops (procedure -1).
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x120)), vec![vec![p.addr(), 1]]);
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 0xffff_ffff]]
        );
        assert!(calls_to(&e, process_slot(0x34c)).is_empty());
        // Standing still: the move mode is set, but no request is built.
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, process_slot(0x34c)).len(), 1);
        assert!(calls_to(&e, ACTOR_BUILD_REQUEST).is_empty());
        // Close together but the escorted actor runs an interrupt package:
        // the same waiting.
        returning_float(&mut e, POINT_SQUARED_LENGTH, 100.0);
        returning(&mut e, ACTOR_CURRENT_PACKAGE, 0x9a00);
        returning(&mut e, PACKAGE_IS_INTERRUPT, 1);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, PACKAGE_IS_INTERRUPT), vec![vec![0x9a00]]);
        assert_eq!(calls_to(&e, process_slot(0x120)).len(), 1);
        // Not an interrupt package: no waiting.
        returning(&mut e, PACKAGE_IS_INTERRUPT, 0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x120)).is_empty());
    }

    #[test]
    fn test_high_process_process_escort_ends_the_escorted_actors_created_package() {
        let mut e = Engine::new();
        let (p, actor, escorted, package) = escort_run_world(&mut e);
        escort_with_escorted_actor(&mut e, p, escorted, false);
        returning_float(&mut e, POINT_SQUARED_LENGTH, 100.0);
        // The package has a location and its slot 0x13c says go on.
        returning(&mut e, PACKAGE_LOCATION_WORD, 0x3000);
        give_vtable(&mut e, package, PACKAGE_TABLE, &[(0x13c, 1)]);
        // The escorted actor's process has a package whose word at +0x18 is 6
        // that was created: its process ends it.
        let target_process = object_with_slots(
            &mut e,
            TARGET_PROCESS_TABLE,
            &[(0x22c, 0x9c00), (0x27c, 0x9b00)],
        );
        give_vtable(
            &mut e,
            target_process,
            TARGET_PROCESS_TABLE,
            &[(0x288, 0), (0x234, 0)],
        );
        e.register_double(ACTOR_PROCESS, move |_, a| {
            ret(if a[0] == escorted { target_process } else { 0 })
        });
        returning(&mut e, PACKAGE_WORD_AT_0X18, 6);
        returning(&mut e, PACKAGE_GET_IS_CREATED, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(TARGET_PROCESS_TABLE, 0x288)),
            vec![vec![target_process, escorted, 1]]
        );
        assert_eq!(
            calls_to(&e, slot_target(TARGET_PROCESS_TABLE, 0x234)),
            vec![vec![target_process]]
        );
        assert_eq!(calls_to(&e, ACTOR_END_MOVEMENT), vec![vec![escorted]]);
        let order = call_order(&e);
        let tail = &order[order.len() - 3..];
        assert_eq!(
            tail,
            [process_slot(0x118), process_slot(0x24), process_slot(0x288)]
        );
        assert_eq!(calls_to(&e, process_slot(0x118)), vec![vec![p.addr(), 1]]);
        assert_eq!(
            calls_to(&e, process_slot(0x24)),
            vec![vec![p.addr(), actor, 0]]
        );
        // A package that is not of that kind: only procedure 1 is added; with
        // pathing incomplete the movement message follows.
        returning(&mut e, PACKAGE_WORD_AT_0X18, 5);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, process_slot(0x118)).is_empty());
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert_eq!(
            calls_to(&e, process_slot(0x294)),
            vec![vec![p.addr(), actor]]
        );
        // A package that was not created: no movement ended.
        returning(&mut e, PACKAGE_WORD_AT_0X18, 6);
        returning(&mut e, PACKAGE_GET_IS_CREATED, 0);
        returning(&mut e, ACTOR_IS_PATHING_COMPLETE, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, ACTOR_END_MOVEMENT).is_empty());
        assert!(calls_to(&e, process_slot(0x24)).is_empty());
    }

    /// An inventory entry for `ProcessEscort`: the word at +0 points to a node
    /// whose first word is `entry`; the amount word at +8 is 42.
    fn escort_item(e: &mut Engine, entry: u32) -> u32 {
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, entry);
        let item = e.mem.alloc(0x20);
        e.mem.set_u32(item, node);
        e.mem.set_u32(item + 8, 42);
        item
    }

    /// The world of an escort that carries an item to a location: no
    /// escorted actor, the package location `0x3000`, and slot `0x13c` of the
    /// package says go on. Returns `(process, actor, package, item, entry)`.
    fn escort_carrying_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32, u32, u32) {
        let (p, actor, _escorted, package) = escort_run_world(e);
        give_vtable(e, package, PACKAGE_TABLE, &[(0x13c, 1)]);
        returning(e, PACKAGE_LOCATION_WORD, 0x3000);
        let entry = e.mem.alloc(0x10);
        let item = escort_item(e, entry);
        returning(e, ACTOR_PACKAGE_ITEM, item);
        returning(e, EXTRA_DATA_REFERENCE_POINTER, 0x4400);
        returning(e, WORD_AT_0XC, 0x7c00);
        e.register(WORD_AT_8, |e, a| ret(e.mem.u32(a[0] + 8)));
        returning(e, PACKAGE_INITIAL_TARGET_COUNT, 4);
        returning(e, PACKAGE_TARGET_GET_OBJECT_TYPE, 3);
        returning(e, REFERENCE_GET_FORM, 0x9999);
        let container = e.mem.alloc(0x10);
        returning(e, PACKAGE_LOCATION_GET_REFERENCE, container);
        returning(e, REFERENCE_HAS_CONTAINER, 1);
        (p, actor, package, item, entry)
    }

    #[test]
    fn test_high_process_process_escort_hands_an_item_to_a_container() {
        let mut e = Engine::new();
        let (p, actor, _package, item, entry) = escort_carrying_world(&mut e);
        let container = e.call(PACKAGE_LOCATION_GET_REFERENCE, &args![0u32]).u32();
        // The first time the special idle is started and the item freed.
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(e.get(p, HighProcess::bActivateAnim), 1);
        assert_eq!(
            calls_to(&e, FIND_SPECIAL_IDLE_TO_PLAY),
            vec![vec![p.addr(), actor, 0x9999, container]]
        );
        assert_eq!(calls_to(&e, REFERENCE_GET_FORM), vec![vec![container]]);
        assert_eq!(calls_to(&e, ITEM_DELETE), vec![vec![item, 1]]);
        assert!(calls_to(&e, process_slot(0x288)).is_empty());
        // The second time the item goes into the container.
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(e.get(p, HighProcess::bActivateAnim), 0);
        assert_eq!(
            calls_to(&e, ACTOR_PUT_IN_CONTAINER),
            vec![vec![actor, 42, entry, container, 4, 0x7c00]]
        );
        assert_eq!(calls_to(&e, ITEM_DELETE), vec![vec![item, 1]]);
        assert_eq!(calls_to(&e, process_slot(0x118)), vec![vec![p.addr(), 1]]);
        assert_eq!(
            calls_to(&e, process_slot(0x288)),
            vec![vec![p.addr(), actor, 1]]
        );
        // The target form of the first entry comes from its reference pointer.
        assert_eq!(
            calls_to(&e, EXTRA_DATA_REFERENCE_POINTER),
            vec![vec![entry], vec![entry]]
        );
        assert_eq!(calls_to(&e, WORD_AT_0XC), vec![vec![0x4400]]);
    }

    #[test]
    fn test_high_process_process_escort_hands_an_item_to_the_actor() {
        let mut e = Engine::new();
        let (p, actor, _package, item, entry) = escort_carrying_world(&mut e);
        // No container: the generic location of the process is not one either.
        let location = object_with_slots(&mut e, OTHER_TABLE, &[(0x1f4, 0x5551)]);
        returning(&mut e, PACKAGE_LOCATION_GET_REFERENCE, 0);
        e.mem.set_u32(p.addr() + 0x44, location);
        returning(&mut e, REFERENCE_HAS_CONTAINER, 0);
        global_word(&mut e, FORM_WORD_A, 0x9999);
        global_word(&mut e, FORM_WORD_B, 0x8888);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(e.get(p, HighProcess::bActivateAnim), 1);
        assert_eq!(
            calls_to(&e, FIND_SPECIAL_IDLE_TO_PLAY),
            vec![vec![p.addr(), actor, 42, 0]]
        );
        assert_eq!(calls_to(&e, ITEM_DELETE), vec![vec![item, 1]]);
        // The second time slot 0x3cc of the actor gets the amount, the first
        // entry, the count and the position of the location.
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(e.get(p, HighProcess::bActivateAnim), 0);
        assert_eq!(
            calls_to(&e, actor_slot(0x3cc)),
            vec![vec![actor, 42, entry, 4, 0x5551, 0]]
        );
        assert!(calls_to(&e, ACTOR_PUT_IN_CONTAINER).is_empty());
        assert_eq!(calls_to(&e, process_slot(0x118)).len(), 1);
        // A location whose form is neither of the two has no position.
        returning(&mut e, REFERENCE_GET_FORM, 0x1111);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        e.call(0x008dfa10, &args![p, actor]);
        assert!(calls_to(&e, actor_slot(0x3cc)).iter().all(|c| c[4] == 0));
    }

    #[test]
    fn test_high_process_process_escort_takes_the_next_objects_to_acquire() {
        let mut e = Engine::new();
        let (p, actor, _package, item, _entry) = escort_carrying_world(&mut e);
        list_doubles(&mut e);
        returning(&mut e, PACKAGE_TARGET_WORD, 0x6600);
        returning(&mut e, PACKAGE_TARGET_GET_OBJECT_TYPE, 0xc);
        // Container case, second time (the flag set): the item is delivered.
        p.addr();
        e.set(p, HighProcess::bActivateAnim, 1);
        // Two objects to acquire: (wanted reference, object).
        let first = e.mem.alloc(0x10);
        e.mem.set_u32(first, 0xa001);
        e.mem.set_u32(first + 4, 0xb001);
        let second = e.mem.alloc(0x10);
        e.mem.set_u32(second, 0xa002);
        e.mem.set_u32(second + 4, 0xb002);
        fill_list(&mut e, p.addr() + 0x5c, &[first, second]);
        let next_item = escort_item(&mut e, 0xe001);
        returning(&mut e, REFERENCE_GET_INVENTORY_ITEM, next_item);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        // Both objects are taken: the target, the inventory item and the
        // deletion of the acquire entry, in order.
        assert_eq!(
            calls_to(&e, process_slot(0x12c)),
            vec![vec![p.addr(), 0xa001], vec![p.addr(), 0xa002]]
        );
        assert_eq!(
            calls_to(&e, REFERENCE_GET_INVENTORY_ITEM),
            vec![vec![actor, 0xb001, 0], vec![actor, 0xb002, 0]]
        );
        assert_eq!(
            calls_to(&e, ACQUIRE_OBJECT_DELETE),
            vec![vec![first, 1], vec![second, 1]]
        );
        assert_eq!(e.mem.u32(p.addr() + 0x64), 0);
        // The item of the first handover and of each object are deleted.
        let deleted = calls_to(&e, ITEM_DELETE);
        assert_eq!(
            deleted,
            vec![vec![item, 1], vec![next_item, 1], vec![next_item, 1]]
        );
        // An NPC form is equipped (slot 0x218), with both flags set.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x218, 1)]);
        fill_list(&mut e, p.addr() + 0x5c, &[]);
        e.set(p, HighProcess::bActivateAnim, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, NPC_FORM_EQUIP),
            vec![vec![0x9999, actor, 1, 1, 0, 1]]
        );
        // The running package of the process with the flag 0x200000 clears the
        // second flag.
        give_vtable(&mut e, p.addr(), PROCESS_TABLE, &[(0x22c, 0x9d00)]);
        returning(&mut e, PACKAGE_FLAG_200000, 1);
        e.set(p, HighProcess::bActivateAnim, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, NPC_FORM_EQUIP),
            vec![vec![0x9999, actor, 1, 0, 0, 1]]
        );
        // A creature form uses InitDefaultWorn.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x218, 0), (0x21c, 1)]);
        e.set(p, HighProcess::bActivateAnim, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, CREATURE_INIT_DEFAULT_WORN),
            vec![vec![0x9999, actor, 1, 0, 1]]
        );
        assert!(calls_to(&e, NPC_FORM_EQUIP).is_empty());
    }

    #[test]
    fn test_high_process_process_escort_waits_for_the_special_idle_to_finish() {
        let mut e = Engine::new();
        let (p, actor, _package, item, _entry) = escort_carrying_world(&mut e);
        // The actor's animation is still playing its special idle.
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(0x1e4, 0x7100)]);
        returning(&mut e, ANIMATION_SPECIAL_IDLE_DONE_PLAYING, 0);
        e.set(p, HighProcess::bActivateAnim, 1);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(calls_to(&e, ITEM_DELETE), vec![vec![item, 1]]);
        assert!(calls_to(&e, ACTOR_PUT_IN_CONTAINER).is_empty());
        // Without the flag the idle is freed and the escort goes on.
        e.set(p, HighProcess::bActivateAnim, 0);
        clear_escort_requests(&mut e, p.addr());
        e.call_log = Some(vec![]);
        e.call(0x008dfa10, &args![p, actor]);
        assert_eq!(
            calls_to(&e, ANIMATION_SPECIAL_IDLE_FREE).last().unwrap(),
            &vec![0x7100, 1, 0]
        );
        assert_eq!(calls_to(&e, FIND_SPECIAL_IDLE_TO_PLAY).len(), 1);
    }
    /// Stubs (returning 0) every callee address named above that is not
    /// translated; tests then register the doubles they care about.
    fn stub_callees(e: &mut Engine) {
        let callees: &[u32] = &[
            0x0040_1170,
            0x0040_19d0,
            0x0040_30b0,
            0x0040_4010,
            0x0040_77c0,
            0x0040_8840,
            0x0040_8860,
            0x0040_ebd0,
            0x0041_3f40,
            0x0041_6870,
            0x0041_81e0,
            0x0041_9700,
            0x0041_c8d0,
            0x0042_16f0,
            0x0042_5fd0,
            0x0042_f420,
            0x0042_fa40,
            0x0042_fb00,
            0x0043_0830,
            0x0043_5dd0,
            0x0043_7dd0,
            0x0043_7df0,
            0x0043_7e50,
            0x0043_9180,
            0x0043_9e90,
            0x0043_9ef0,
            0x0043_b230,
            0x0043_c490,
            0x0043_d4d0,
            0x0044_0d80,
            0x0044_0da0,
            0x0044_1110,
            0x0044_1b00,
            0x0044_59e0,
            0x0044_b130,
            0x0044_ddc0,
            0x0045_43c0,
            0x0045_7990,
            0x0045_8620,
            0x0045_8b20,
            0x0045_8b50,
            0x0045_af00,
            0x0045_bad0,
            0x0045_bb20,
            0x0045_c670,
            0x0045_cd60,
            0x0046_1130,
            0x0046_f280,
            0x0047_6a80,
            0x0047_6b70,
            0x0047_6c00,
            0x0047_7ba0,
            0x0047_d0b0,
            0x0047_f600,
            0x0048_7f50,
            0x0048_e8a0,
            0x0049_02f0,
            0x0049_38e0,
            0x0049_3bb0,
            0x0049_85f0,
            0x0049_8910,
            0x004a_0c10,
            0x004a_0c90,
            0x004a_39f0,
            0x004a_3a20,
            0x004a_3e00,
            0x004a_7290,
            0x004a_e6a0,
            0x004a_e750,
            0x004b_4e50,
            0x004f_0140,
            0x004f_d400,
            0x004f_f7e0,
            0x0050_0940,
            0x0050_0a20,
            0x0050_7610,
            0x0050_8070,
            0x0051_9b00,
            0x0053_d280,
            0x0054_6ca0,
            0x0055_2470,
            0x0055_8310,
            0x0055_9a40,
            0x0055_b980,
            0x0055_d310,
            0x0055_d520,
            0x0056_4db0,
            0x0056_6950,
            0x0056_7400,
            0x0056_7790,
            0x0056_8260,
            0x0056_8e50,
            0x0056_8fa0,
            0x0056_9160,
            0x0056_df20,
            0x0056_e090,
            0x0056_e2d0,
            0x0056_e690,
            0x0056_e810,
            0x0056_ea90,
            0x0056_f930,
            0x0057_2380,
            0x0057_2500,
            0x0057_3170,
            0x0057_4900,
            0x0057_5450,
            0x0057_5d70,
            0x0057_6260,
            0x0057_85e0,
            0x0057_9670,
            0x0057_b0a0,
            0x0057_bd60,
            0x0058_db10,
            0x0059_5c80,
            0x0059_bb30,
            0x0059_ce80,
            0x005a_2030,
            0x005a_3740,
            0x005a_c750,
            0x005a_e380,
            0x005a_e3d0,
            0x005b_5e40,
            0x005b_e5c0,
            0x005c_0880,
            0x005c_3420,
            0x005c_5420,
            0x005d_43c0,
            0x005e_3fc0,
            0x005f_9e00,
            0x0060_0900,
            0x0060_47c0,
            0x0060_8d80,
            0x0062_3150,
            0x0062_c250,
            0x0062_d310,
            0x0062_d350,
            0x0062_d4b0,
            0x0062_e150,
            0x0063_c8a0,
            0x0064_2bc0,
            0x0064_3f20,
            0x0064_3f90,
            0x0064_3fa0,
            0x0064_3ff0,
            0x0064_65f0,
            0x0066_29f0,
            0x0066_ef20,
            0x0067_1d10,
            0x0067_2930,
            0x0067_2f20,
            0x0067_4d40,
            0x0067_4e70,
            0x0067_57e0,
            0x0067_5830,
            0x0067_58a0,
            0x0067_5a50,
            0x0067_5c20,
            0x0067_5de0,
            0x0067_6140,
            0x0067_6280,
            0x0067_84c0,
            0x0067_8610,
            0x0067_8670,
            0x0067_87e0,
            0x0067_8ca0,
            0x0067_9ba0,
            0x0067_a480,
            0x0067_efd0,
            0x0067_f390,
            0x0067_f3e0,
            0x0068_0020,
            0x0068_0050,
            0x0068_0080,
            0x0068_ef70,
            0x0069_3d50,
            0x0069_3d70,
            0x0069_ad00,
            0x006c_a4e0,
            0x006d_1bc0,
            0x006d_3b00,
            0x006d_61e0,
            0x006d_ad70,
            0x006d_cd70,
            0x006d_ce10,
            0x006d_cee0,
            0x006d_d4f0,
            0x006d_d540,
            0x006d_d6f0,
            0x006d_dc00,
            0x006e_2420,
            0x006e_2620,
            0x006e_2960,
            0x006e_29f0,
            0x006e_2b50,
            0x006e_2bb0,
            0x006e_5390,
            0x006e_7800,
            0x006e_9bd0,
            0x006e_b9d0,
            0x006e_bf30,
            0x0070_14e0,
            0x0070_c440,
            0x0071_7e50,
            0x007a_f430,
            0x007b_3fa0,
            0x007d_1d00,
            0x0080_0190,
            0x0081_0530,
            0x0082_56d0,
            0x0082_f1f0,
            0x0084_e3a0,
            0x0086_7da0,
            0x0087_f350,
            0x0087_f390,
            0x0087_f570,
            0x0087_f660,
            0x0087_faa0,
            0x0088_40d0,
            0x0088_40f0,
            0x0088_43a0,
            0x0088_4b50,
            0x0088_4f80,
            0x0088_4fc0,
            0x0088_4ff0,
            0x0088_53a0,
            0x0088_55f0,
            0x0088_58c0,
            0x0088_b880,
            0x0088_c240,
            0x0088_c570,
            0x0088_c650,
            0x0088_d640,
            0x0089_1170,
            0x0089_1d30,
            0x0089_1d70,
            0x0089_1db0,
            0x0089_24e0,
            0x0089_2e90,
            0x0089_8250,
            0x008a_0d10,
            0x008a_2d40,
            0x008a_5e40,
            0x008a_5e80,
            0x008a_6290,
            0x008a_69d0,
            0x008a_6ce0,
            0x008a_ce90,
            0x008a_d1c0,
            0x008a_dcb0,
            0x008b_0190,
            0x008b_06d0,
            0x008b_1ff0,
            0x008b_3630,
            0x008b_3690,
            0x008b_3880,
            0x008b_38d0,
            0x008b_3960,
            0x008b_3a80,
            0x008b_3ab0,
            0x008b_3ad0,
            0x008b_3b90,
            0x008b_3bd0,
            0x008b_3bf0,
            0x008b_3c30,
            0x008b_3cb0,
            0x008b_3d30,
            0x008b_6240,
            0x008b_a530,
            0x008b_b630,
            0x008b_c700,
            0x008b_c790,
            0x008c_1de0,
            0x008c_8bb0,
            0x008d_6f30,
            0x008d_8220,
            0x008f_f0b0,
            0x008f_fd10,
            0x008f_fdc0,
            0x0090_57d0,
            0x0090_dd80,
            0x0091_5ef0,
            0x0091_b580,
            0x0091_b640,
            0x0092_2670,
            0x0092_c680,
            0x0093_06d0,
            0x0093_1850,
            0x0094_4460,
            0x0094_4480,
            0x0095_3c50,
            0x0095_49a0,
            0x0096_11e0,
            0x0096_8670,
            0x0096_9860,
            0x0096_d450,
            0x0096_d470,
            0x0097_0a20,
            0x0097_15c0,
            0x0099_4ef0,
            0x009d_cbf0,
            0x009d_cc20,
            0x009d_cc50,
            0x009d_cdb0,
            0x009d_e0a0,
            0x009d_f010,
            0x009e_32d0,
            0x009f_1140,
            0x009f_1210,
            0x009f_1310,
            0x009f_1440,
            0x009f_1830,
            0x009f_1990,
            0x009f_6900,
            0x00aa_13e0,
            0x00aa_1460,
            0x00c4_f7b0,
            0x00c5_2960,
            0x00c7_fa90,
            0x00ec_7595,
        ];
        for &addr in callees {
            {
                e.register_double(addr, |_, _| Ret::default());
            }
        }
    }

    /// The world of `ProcessBuyObject`: a process whose package says the
    /// actor is at the seller, an acquire object of 3 items worth 7 each and
    /// a seller that is an actor.
    fn buy_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32, u32, u32) {
        stub_callees(e);
        let package = object_with_slots(e, PACKAGE_TABLE, &[(0x144, 1)]);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (SLOT_GET_PACKAGE_THAT_IS_RUNNING, package),
                (SLOT_SET_TARGET_FOR_PACKAGE, 0),
                (SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING, 0),
                (SLOT_END_MOVE_MESSAGE, 0),
                (SLOT_SET_TARGET, 0),
                (SLOT_SET_ACTORS_ANIMATION, 0),
            ],
        );
        let actor = object_with_slots(e, ACTOR_TABLE, &[(ACTOR_SLOT_STATE, 0)]);
        let seller = object_with_slots(e, OTHER_TABLE, &[(ACTOR_SLOT_0X17C, 0)]);
        let reference = object_with_slots(e, ACTOR_TABLE, &[(ACTOR_SLOT_IS_ACTOR, 1)]);
        let entry = e.mem.alloc(0x40);
        e.mem.set_u32(entry, reference);
        e.mem.set_u32(entry + 4, 0x9100);
        e.mem.set_u32(entry + ACQUIRE_COUNT, 3);
        e.mem.set_u32(entry + ACQUIRE_ACTION_FLAG, 0x55);
        e.mem.set_u32(p.addr() + LOW_TARGET, seller);
        e.mem.set_u32(p.addr() + LOW_ACQUIRE_OBJECT, entry);
        returning(e, FORM_VALUE, 7);
        returning(e, ACTOR_IS_PATHING_COMPLETE, 1);
        (p, actor, seller, entry, package)
    }

    #[test]
    fn test_high_process_process_buy_object() {
        let mut e = Engine::new();
        let (p, actor, seller, entry, _package) = buy_world(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008e59c0, &args![p, actor]);
        // The seller is an actor: its action flag is set and its slot
        // `0x17c` called with the form; the actor pays 3 * 7.
        assert_eq!(
            calls_to(&e, SCRIPT_SET_ACTION_FLAG),
            vec![vec![seller, 0x55, ACTION_FLAG_BUY]]
        );
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, ACTOR_SLOT_0X17C)),
            vec![vec![seller, 0x9100, 0, 1, 0, 0, actor, 0, 0, 1, 0]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_PAY_GOLD_TO_ACTOR),
            vec![vec![actor, seller, 21]]
        );
        // The acquire object is deleted and cleared.
        assert_eq!(calls_to(&e, ACQUIRE_OBJECT_DELETE), vec![vec![entry, 1]]);
        assert_eq!(e.mem.u32(p.addr() + LOW_ACQUIRE_OBJECT), 0);
    }

    #[test]
    fn test_high_process_process_buy_object_other_cases() {
        let mut e = Engine::new();
        let (p, actor, _seller, _entry, _package) = buy_world(&mut e);
        // A flagged target only adds procedure 1.
        returning(&mut e, REFERENCE_FLAG_800, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e59c0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)),
            vec![vec![p.addr(), actor, 1]]
        );
        assert!(calls_to(&e, ACTOR_PAY_GOLD_TO_ACTOR).is_empty());
        assert_ne!(e.mem.u32(p.addr() + LOW_ACQUIRE_OBJECT), 0);
        // Without an acquire object nothing happens.
        returning(&mut e, REFERENCE_FLAG_800, 0);
        e.mem.set_u32(p.addr() + LOW_ACQUIRE_OBJECT, 0);
        e.call_log = Some(vec![]);
        e.call(0x008e59c0, &args![p, actor]);
        assert!(calls_to(&e, ACTOR_PAY_GOLD_TO_ACTOR).is_empty());
        assert!(calls_to(&e, SCRIPT_SET_ACTION_FLAG).is_empty());
    }

    /// Makes the setting getters answer a float of 4.0 (`SETTING_VALUE`)
    /// and an integer of 2 (`SETTING_VALUE_ADDRESS`).
    fn settings_doubles(e: &mut Engine) {
        let float = e.mem.alloc(16);
        e.mem.set_f32(float, 4.0);
        let integer = e.mem.alloc(16);
        e.mem.set_u32(integer, 2);
        returning(e, SETTING_VALUE, float);
        returning(e, SETTING_VALUE_ADDRESS, integer);
    }

    /// `NI_POINTER_GET` reads the word at its argument and `NI_POINTER_SET`
    /// stores its second argument there.
    fn pointer_doubles(e: &mut Engine) {
        e.register_double(NI_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        e.register_double(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
    }

    // ---- `ProcessEat` ----

    /// The world of `ProcessEat`: a process and an actor with the slots it
    /// asks, and a package.
    fn eat_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        stub_callees(e);
        settings_doubles(e);
        pointer_doubles(e);
        global_f64(e, ZERO_DOUBLE, 0.0);
        let package = object_with_slots(e, PACKAGE_TABLE, &[(PACKAGE_SLOT_0X13C, 0)]);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (SLOT_GET_PACKAGE_THAT_IS_RUNNING, package),
                (SLOT_GET_CURRENT_ACTION_COMPLETE, 1),
                (SLOT_EAT_AFTER, 0),
                (SLOT_SET_PROCEDURE_INDEX_RUNNING, 0),
                (SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING, 0),
            ],
        );
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[
                (ACTOR_SLOT_STATE, 0),
                (ACTOR_SLOT_0X418, 0),
                (ACTOR_SLOT_0X218, 0),
                (ACTOR_SLOT_0X3D0, 0),
                (ACTOR_SLOT_ANIMATION, 0),
            ],
        );
        (p, actor, package)
    }

    #[test]
    fn test_high_process_process_eat_stops_in_state_9() {
        let mut e = Engine::new();
        let (p, actor, _package) = eat_world(&mut e);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(ACTOR_SLOT_STATE, 9)]);
        e.call_log = Some(vec![]);
        e.call(0x008e3140, &args![p, actor, 0]);
        assert_eq!(
            call_order(&e),
            vec![
                process_slot(SLOT_GET_PACKAGE_THAT_IS_RUNNING),
                actor_slot(ACTOR_SLOT_STATE),
                actor_slot(ACTOR_SLOT_0X418)
            ]
        );
    }

    #[test]
    fn test_high_process_process_eat_eats_the_food_on_the_ground() {
        let mut e = Engine::new();
        let (p, actor, package) = eat_world(&mut e);
        // The package location is a food reference (form type 0x1d).
        returning(&mut e, PACKAGE_GET_LOCATION_REFERENCE, 0x5000);
        returning(&mut e, REFERENCE_GET_FORM, 0x6000);
        returning(&mut e, FORM_TYPE_BYTE, 0x1d);
        returning(&mut e, REFERENCE_EXTRA_DATA, 0x7000);
        e.call_log = Some(vec![]);
        e.call(0x008e3140, &args![p, actor, 0]);
        assert_eq!(
            calls_to(&e, PACKAGE_GET_LOCATION_REFERENCE),
            vec![vec![package, actor]]
        );
        assert_eq!(
            calls_to(&e, actor_slot(ACTOR_SLOT_0X3D0)),
            vec![vec![actor, 0x5000, 1, 0]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_EAT),
            vec![vec![actor, 0x6000, 0x7000, 1]]
        );
        assert_eq!(
            calls_to(&e, process_slot(SLOT_EAT_AFTER)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(e.mem.u32(p.addr() + LOW_ITEM_BEING_USED), 0x6000);
    }

    #[test]
    fn test_high_process_process_eat_without_food_or_furniture() {
        let mut e = Engine::new();
        let (p, actor, _package) = eat_world(&mut e);
        // The action is not complete and the actor is of the tested kind:
        // the food comes from the inventory (an item with the word 0x9000),
        // the package does not want a location and there is no furniture.
        give_vtable(
            &mut e,
            p.addr(),
            PROCESS_TABLE,
            &[(SLOT_GET_CURRENT_ACTION_COMPLETE, 0)],
        );
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(ACTOR_SLOT_0X218, 1)]);
        returning(&mut e, ACTOR_GET_BEST_FOOD_ITEM, 0x8000);
        returning(&mut e, WORD_AT_8, 0x9000);
        global_f32(&mut e, NO_LIMIT_FLOAT, -1.0);
        e.call_log = Some(vec![]);
        e.call(0x008e3140, &args![p, actor, 0]);
        assert_eq!(e.mem.u32(p.addr() + LOW_ITEM_BEING_USED), 0x9000);
        assert_eq!(calls_to(&e, ACTOR_GET_BEST_FOOD_ITEM), vec![vec![actor]]);
        assert_eq!(
            calls_to(&e, process_slot(SLOT_SET_PROCEDURE_INDEX_RUNNING)),
            vec![vec![p.addr(), 0]]
        );
        assert!(calls_to(&e, ACTOR_EAT).is_empty());
    }

    // ---- The shape phantom ----

    /// The world of `008e4e50`/`008e51b0`: an actor whose process (at
    /// +0x68) has a shape source, and a process.
    fn phantom_world(e: &mut Engine, source: u32) -> (Ptr<HighProcess>, u32) {
        stub_callees(e);
        settings_doubles(e);
        pointer_doubles(e);
        global_f64(e, ZERO_DOUBLE, 0.0);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(SLOT_GET_CHARACTER_CONTROLLER, 0)],
        );
        let actor_process = object_with_slots(
            e,
            TARGET_PROCESS_TABLE,
            &[(PROCESS_SLOT_SHAPE_SOURCE, source)],
        );
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[
                (ACTOR_SLOT_POSITION, 0),
                (ACTOR_SLOT_NODE_FOR_FOLLOW, 0x4400),
            ],
        );
        e.mem.set_u32(actor + ACTOR_PROCESS_WORD, actor_process);
        (p, actor)
    }

    #[test]
    fn test_fn_008e4e50_needs_a_shape_source() {
        let mut e = Engine::new();
        let (p, actor) = phantom_world(&mut e, 0);
        e.call_log = Some(vec![]);
        e.call(0x008e4e50, &args![p, actor]);
        assert_eq!(
            call_order(&e),
            vec![slot_target(TARGET_PROCESS_TABLE, PROCESS_SLOT_SHAPE_SOURCE)]
        );
    }

    #[test]
    fn test_fn_008e4e50_adds_the_existing_phantom_to_the_world() {
        let mut e = Engine::new();
        let (p, actor) = phantom_world(&mut e, 0x4000);
        let phantom = object_with_slots(&mut e, OTHER_TABLE, &[(PHANTOM_SLOT_ADD_TO_WORLD, 0)]);
        e.mem.set_u32(p.addr() + SHAPE_PHANTOM, phantom);
        returning(&mut e, REFERENCE_PATHING_CELL, 0x222);
        returning(&mut e, CELL_PHYSICS_WORLD, 0x333);
        e.call_log = Some(vec![]);
        e.call(0x008e4e50, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, PHANTOM_SLOT_ADD_TO_WORLD)),
            vec![vec![phantom, 0x333]]
        );
        assert_eq!(calls_to(&e, CELL_PHYSICS_WORLD), vec![vec![0x222]]);
        assert!(calls_to(&e, PHANTOM_CONSTRUCTOR).is_empty());
    }

    #[test]
    fn test_fn_008e4e50_makes_the_phantom() {
        let mut e = Engine::new();
        let (p, actor) = phantom_world(&mut e, 0x4000);
        global_word(&mut e, BOX_SHAPE_COUNT, 0);
        // The box of the source is 1 x 2 x 3 and the actor is scaled by 2.
        let dimensions = e.mem.alloc(16);
        e.mem.set_f32(dimensions, 1.0);
        e.mem.set_f32(dimensions + 4, 2.0);
        e.mem.set_f32(dimensions + 8, 3.0);
        returning(&mut e, SHAPE_SOURCE_DIMENSIONS, dimensions);
        returning_float(&mut e, REFERENCE_GET_SCALE, 2.0);
        global_f64(&mut e, PHANTOM_Y_FACTOR, 2.5);
        global_f64(&mut e, PHANTOM_X_FACTOR, 0.5);
        e.register_double(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        let seen = e.mem.alloc(16);
        e.register_double(POINT_TO_HAVOK_VECTOR, move |e, a| {
            for word in 0..3 {
                let value = e.mem.u32(a[1] + word * 4);
                e.mem.set_u32(seen + word * 4, value);
            }
            ret(a[0])
        });
        e.register_double(PHANTOM_CONSTRUCTOR, |e, a| {
            give_vtable(e, a[0], OTHER_TABLE, &[(PHANTOM_SLOT_ADD_TO_WORLD, 0)]);
            ret(a[0])
        });
        returning(&mut e, CELL_PHYSICS_WORLD, 0x333);
        e.call_log = Some(vec![]);
        e.call(0x008e4e50, &args![p, actor]);
        // x * 0.5 * 2, y * 2.5 * 2 (z is not below y) and z * 2.
        assert_eq!(
            (e.mem.f32(seen), e.mem.f32(seen + 4), e.mem.f32(seen + 8)),
            (1.0, 10.0, 6.0)
        );
        // The collision filter starts with the low bits 0x22.
        let low = calls_to(&e, FILTER_SET_LOW);
        assert_eq!((low.len(), low[0][1]), (1, 0x22));
        // The phantom is stored, the timer set from the setting and the
        // phantom added to the world.
        let phantom = e.mem.u32(p.addr() + SHAPE_PHANTOM);
        assert_ne!(phantom, 0);
        assert_eq!(e.mem.f32(p.addr() + DOOR_TIMER), 4.0);
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, PHANTOM_SLOT_ADD_TO_WORLD)),
            vec![vec![phantom, 0x333]]
        );
        assert_eq!(calls_to(&e, PHANTOM_INFO_DESTRUCTOR).len(), 1);
    }

    fn door_world(e: &mut Engine, source: u32) -> (Ptr<HighProcess>, u32) {
        let (p, actor) = phantom_world(e, source);
        returning_float(e, FRAME_TIME, 0.5);
        global_f32(e, DOOR_TIMER_RANDOM_LOW, 0.8);
        global_f32(e, DOOR_TIMER_RANDOM_HIGH, 1.2);
        returning_float(e, RANDOM_FLOAT_RANGE, 1.0);
        (p, actor)
    }

    #[test]
    fn test_fn_008e51b0_counts_down_the_door_timer() {
        let mut e = Engine::new();
        let (p, actor) = door_world(&mut e, 0x4000);
        e.mem.set_f32(p.addr() + DOOR_TIMER, 3.0);
        e.call_log = Some(vec![]);
        e.call(0x008e51b0, &args![p, actor]);
        assert_eq!(e.mem.f32(p.addr() + DOOR_TIMER), 2.5);
        assert!(calls_to(&e, ACTOR_SET_BLOCKING_DOOR).is_empty());
        assert!(calls_to(&e, RANDOM_FLOAT_RANGE).is_empty());
    }

    #[test]
    fn test_fn_008e51b0_without_a_shape_source_gives_no_door() {
        let mut e = Engine::new();
        let (p, actor) = door_world(&mut e, 0);
        e.mem.set_f32(p.addr() + DOOR_TIMER, 0.25);
        e.call_log = Some(vec![]);
        e.call(0x008e51b0, &args![p, actor]);
        // The timer ran out: it is set from the setting (4.0) times the
        // random factor (1.0); the actor is told there is no door.
        assert_eq!(e.mem.f32(p.addr() + DOOR_TIMER), 4.0);
        let random = calls_to(&e, RANDOM_FLOAT_RANGE);
        assert_eq!(random, vec![vec![0.8f32.to_bits(), 1.2f32.to_bits()]]);
        assert_eq!(calls_to(&e, ACTOR_SET_BLOCKING_DOOR), vec![vec![actor, 0]]);
    }

    #[test]
    fn test_fn_008e51b0_stops_when_the_phantom_is_not_in_the_world() {
        let mut e = Engine::new();
        let (p, actor) = door_world(&mut e, 0x4000);
        let phantom = object_with_slots(&mut e, OTHER_TABLE, &[(PHANTOM_SLOT_0X94, 0)]);
        e.mem.set_u32(p.addr() + SHAPE_PHANTOM, phantom);
        e.mem.set_f32(p.addr() + DOOR_TIMER, 0.0);
        e.call_log = Some(vec![]);
        e.call(0x008e51b0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, slot_target(OTHER_TABLE, PHANTOM_SLOT_0X94)),
            vec![vec![phantom]]
        );
        assert!(calls_to(&e, ACTOR_SET_BLOCKING_DOOR).is_empty());
    }

    // ---- The bone level of detail ----

    fn bone_lod_world(e: &mut Engine, maximum: u32) -> (Ptr<HighProcess>, u32) {
        stub_callees(e);
        settings_doubles(e);
        global_f64(e, DISTANCE_FACTOR, 1.0);
        global_word(e, PLAYER_POINTER, 0x1234);
        returning_float(e, REFERENCE_GET_SCALE, 1.0);
        returning_float(e, POINT_LENGTH_FOR_LOD, 100.0);
        returning_float(e, FLOAT_ABSOLUTE_FOR_LOD, 100.0);
        returning_float(e, FLOAT_AT_0X110, 2.0);
        returning_float(e, LOD_TABLE_ENTRY, 1.0);
        e.register_double(FLOAT_TO_INT, |_, a| {
            let value = f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32);
            ret(value as i32 as u32)
        });
        returning(e, BONE_LOD_MAXIMUM, maximum);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (PROCESS_SLOT_LOD_NODE_A, 0xa000),
                (PROCESS_SLOT_LOD_NODE_B, 0xb000),
                (PROCESS_SLOT_LOD_NODE_C, 0xc000),
            ],
        );
        e.mem.set_u32(p.addr() + BONE_LOD_CONTROLLER, 0xd000);
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[(ACTOR_SLOT_POSITION, 0), (ACTOR_SLOT_NODE_FOR_FOLLOW, 0)],
        );
        (p, actor)
    }

    #[test]
    fn test_fn_008e5730_far_actors_get_level_minus_one() {
        let mut e = Engine::new();
        // (2 * 100 * 1) / (1 * 2) = 100, at or above the maximum 50.
        let (p, actor) = bone_lod_world(&mut e, 50);
        returning(&mut e, NODE_FLAG_2, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e5730, &args![p, actor]);
        assert_eq!(calls_to(&e, SET_BONE_LOD), vec![vec![0xd000, u32::MAX]]);
        assert_eq!(e.mem.i32(p.addr() + LAST_BONE_LOD), -1);
        // The flag of the third node is copied to the first two.
        assert_eq!(
            calls_to(&e, SET_FLAG_ON_NODE),
            vec![vec![0xa000, 1], vec![0xb000, 1]]
        );
        // The same level again changes nothing.
        e.call_log = Some(vec![]);
        e.call(0x008e5730, &args![p, actor]);
        assert!(calls_to(&e, SET_BONE_LOD).is_empty());
    }

    #[test]
    fn test_fn_008e5730_near_actors_get_their_level() {
        let mut e = Engine::new();
        let (p, actor) = bone_lod_world(&mut e, 1000);
        e.call_log = Some(vec![]);
        e.call(0x008e5730, &args![p, actor]);
        assert_eq!(calls_to(&e, SET_BONE_LOD), vec![vec![0xd000, 100]]);
        assert_eq!(e.mem.i32(p.addr() + LAST_BONE_LOD), 100);
        // The point is divided by the scale.
        assert_eq!(calls_to(&e, POINT_DIVIDE_BY_SCALE).len(), 1);
        assert_eq!(calls_to(&e, POINT_DIVIDE_BY_SCALE)[0][2], 1.0f32.to_bits());
    }

    // ---- `RunDetection` ----

    fn detection_world(e: &mut Engine) -> (Ptr<HighProcess>, u32) {
        stub_callees(e);
        stub(e, &[LIST_CLEAR, SOUND_HANDLE_DESTRUCTOR]);
        settings_doubles(e);
        global_f64(e, ZERO_DOUBLE, 0.0);
        global_f64(e, DETECTION_DISTANCE_LIMIT, 8192.0);
        global_word(e, FADER_MANAGER, 0x4000);
        e.map(DETECTION_ENABLED, 8);
        e.mem.set_u8(DETECTION_ENABLED, 1);
        returning_float(e, FADER_GET_ALPHA, 0.0);
        returning_float(e, DISTANCE_FROM_REFERENCE, 10.0);
        returning(e, HIGH_ACTORS_COUNT, 0);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[(SLOT_GET_COMMANDING_ACTOR, 0)],
        );
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[
                (ACTOR_SLOT_PAUSED_A, 0),
                (ACTOR_SLOT_0X22C, 0),
                (ACTOR_SLOT_IS_ACTOR, 1),
            ],
        );
        // The actor is the player: the player half is skipped.
        global_word(e, PLAYER_POINTER, actor);
        (p, actor)
    }

    #[test]
    fn test_high_process_run_detection_does_nothing_while_faded() {
        let mut e = Engine::new();
        let (p, actor) = detection_world(&mut e);
        returning_float(&mut e, FADER_GET_ALPHA, 0.5);
        e.call_log = Some(vec![]);
        e.call(0x008e40d0, &args![p, actor, 0.5f32]);
        // Only the two lists are cleared and the fader asked.
        assert_eq!(
            call_order(&e),
            vec![LIST_CLEAR, LIST_CLEAR, FADER_GET_ALPHA]
        );
        assert_eq!(
            calls_to(&e, LIST_CLEAR),
            vec![
                vec![p.addr() + DETECTION_LIST_A],
                vec![p.addr() + DETECTION_LIST_B]
            ]
        );
        assert_eq!(calls_to(&e, FADER_GET_ALPHA), vec![vec![0x4000, 1]]);
    }

    #[test]
    fn test_high_process_run_detection_far_from_the_player_or_switched_off() {
        let mut e = Engine::new();
        let (p, actor) = detection_world(&mut e);
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 9000.0);
        e.call_log = Some(vec![]);
        e.call(0x008e40d0, &args![p, actor, 0.5f32]);
        assert!(calls_to(&e, SOUND_HANDLE_DESTRUCTOR).is_empty());
        returning_float(&mut e, DISTANCE_FROM_REFERENCE, 10.0);
        e.mem.set_u8(DETECTION_ENABLED, 0);
        e.call(0x008e40d0, &args![p, actor, 0.5f32]);
        assert!(calls_to(&e, SOUND_HANDLE_DESTRUCTOR).is_empty());
        // A paused actor stops it too.
        e.mem.set_u8(DETECTION_ENABLED, 1);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(ACTOR_SLOT_PAUSED_A, 1)]);
        e.call(0x008e40d0, &args![p, actor, 0.5f32]);
        assert!(calls_to(&e, SOUND_HANDLE_DESTRUCTOR).is_empty());
    }

    #[test]
    fn test_high_process_run_detection_sets_the_timer_and_counter() {
        let mut e = Engine::new();
        let (p, actor) = detection_world(&mut e);
        e.mem.set_u32(p.addr() + DETECTION_COUNTER, 11);
        e.call_log = Some(vec![]);
        e.call(0x008e40d0, &args![p, actor, 0.5f32]);
        // No creature: the timer is the setting (4.0) plus the delta.
        assert_eq!(e.mem.f32(p.addr() + DETECTION_TIMER), 4.5);
        assert_eq!(e.mem.u8(p.addr() + DETECTION_EVALUATE), 1);
        assert_eq!(e.mem.u32(p.addr() + DETECTION_COUNTER), 0);
        assert_eq!(e.mem.u8(p.addr() + DETECTION_CHECK_DEAD_TALK), 0);
        assert_eq!(calls_to(&e, SOUND_HANDLE_DESTRUCTOR).len(), 2);
        // A creature-like actor gets the setting alone.
        returning(&mut e, ACTOR_CREATURE_LIKE, 1);
        e.call(0x008e40d0, &args![p, actor, 0.5f32]);
        assert_eq!(e.mem.f32(p.addr() + DETECTION_TIMER), 4.0);
        assert_eq!(e.mem.u32(p.addr() + DETECTION_COUNTER), 1);
    }

    // ---- `ProcessFollow` ----

    fn follow_main_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        stub_callees(e);
        settings_doubles(e);
        global_f64(e, ZERO_DOUBLE, 0.0);
        let package = object_with_slots(e, PACKAGE_TABLE, &[(PACKAGE_SLOT_0X13C, 0)]);
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (SLOT_GET_PACKAGE_THAT_IS_RUNNING, package),
                (SLOT_SET_TARGET_FOR_PACKAGE, 0),
                (SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING, 0),
                (SLOT_SET_TARGET, 0),
                (SLOT_SET_PROCEDURE_INDEX_RUNNING, 0),
                (SLOT_END_MOVE_MESSAGE, 0),
            ],
        );
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[
                (ACTOR_SLOT_0X428, 0),
                (ACTOR_SLOT_STATE, 0),
                (ACTOR_SLOT_0X418, 0),
            ],
        );
        global_word(e, PLAYER_POINTER, 0x1234);
        (p, actor, package)
    }

    #[test]
    fn test_high_process_process_follow_early_exits() {
        let mut e = Engine::new();
        let (p, actor, package) = follow_main_world(&mut e);
        // A package with the flag 0x10000 does nothing.
        returning(&mut e, PACKAGE_FLAG_10000, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e0f80, &args![p, actor, 1, u32::MAX, 0]);
        assert_eq!(calls_to(&e, PACKAGE_FLAG_10000), vec![vec![package]]);
        assert!(calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)).is_empty());
        // An actor in state 4 calls its slot 0x418.
        returning(&mut e, PACKAGE_FLAG_10000, 0);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(ACTOR_SLOT_STATE, 4)]);
        e.call_log = Some(vec![]);
        e.call(0x008e0f80, &args![p, actor, 1, u32::MAX, 0]);
        assert_eq!(
            calls_to(&e, actor_slot(ACTOR_SLOT_0X418)),
            vec![vec![actor]]
        );
        assert!(calls_to(&e, process_slot(SLOT_SET_TARGET_FOR_PACKAGE)).is_empty());
    }

    #[test]
    fn test_high_process_process_follow_without_a_target_ends_the_package() {
        let mut e = Engine::new();
        let (p, actor, _package) = follow_main_world(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008e0f80, &args![p, actor, 1, u32::MAX, 0]);
        // The target is asked for and none comes: with `stop` set procedure
        // 1 is added.
        assert_eq!(
            calls_to(&e, process_slot(SLOT_SET_TARGET_FOR_PACKAGE)),
            vec![vec![p.addr(), actor]]
        );
        assert_eq!(
            calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)),
            vec![vec![p.addr(), actor, 1]]
        );
        // Without `stop` nothing is added.
        e.call_log = Some(vec![]);
        e.call(0x008e0f80, &args![p, actor, 0, u32::MAX, 0]);
        assert!(calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)).is_empty());
    }

    #[test]
    fn test_high_process_process_follow_dead_target() {
        let mut e = Engine::new();
        let (p, actor, package) = follow_main_world(&mut e);
        let target = object_with_slots(
            &mut e,
            OTHER_TABLE,
            &[(ACTOR_SLOT_0X22C, 1), (ACTOR_SLOT_IS_ACTOR, 1)],
        );
        e.mem.set_u32(p.addr() + LOW_TARGET, target);
        // A dead target of a created package ends the interrupt package and
        // evaluates the packages again.
        returning(&mut e, PACKAGE_GET_IS_CREATED, 1);
        e.call_log = Some(vec![]);
        e.call(0x008e0f80, &args![p, actor, 1, u32::MAX, 0]);
        assert_eq!(
            calls_to(&e, ACTOR_END_INTERRUPT_PACKAGE),
            vec![vec![actor, 0]]
        );
        assert_eq!(
            calls_to(&e, ACTOR_EVALUATE_PACKAGE),
            vec![vec![actor, 0, 0]]
        );
        assert!(calls_to(&e, PACKAGE_SET_NEVER_RUN).is_empty());
        // Otherwise a package without a target type is never run again for it.
        returning(&mut e, PACKAGE_GET_IS_CREATED, 0);
        returning(&mut e, PACKAGE_TARGET_GET_TYPE, 0);
        e.call_log = Some(vec![]);
        e.call(0x008e0f80, &args![p, actor, 1, u32::MAX, 0]);
        assert_eq!(
            calls_to(&e, PACKAGE_SET_NEVER_RUN),
            vec![vec![package, target, 1]]
        );
        // A target of another kind is dropped and the procedure restarted.
        returning(&mut e, PACKAGE_TARGET_GET_TYPE, 2);
        e.call_log = Some(vec![]);
        e.call(0x008e0f80, &args![p, actor, 1, u32::MAX, 0]);
        assert_eq!(
            calls_to(&e, process_slot(SLOT_SET_TARGET)),
            vec![vec![p.addr(), 0]]
        );
        assert_eq!(
            calls_to(&e, process_slot(SLOT_SET_PROCEDURE_INDEX_RUNNING)),
            vec![vec![p.addr(), 0]]
        );
    }

    // ---- `ProcessFlee` ----

    fn flee_main_world(e: &mut Engine) -> (Ptr<HighProcess>, u32, u32) {
        stub_callees(e);
        settings_doubles(e);
        pointer_doubles(e);
        global_f64(e, ZERO_DOUBLE, 0.0);
        global_f64(e, FLEE_TIME_FACTOR, 2.0);
        global_f64(e, FLEE_TIME_LIMIT, 10.0);
        global_word(e, PLAYER_POINTER, 0x1234);
        let package = 0x5500;
        let p = block(e);
        give_vtable(
            e,
            p.addr(),
            PROCESS_TABLE,
            &[
                (SLOT_GET_PACKAGE_THAT_IS_RUNNING, package),
                (SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING, 0),
                (SLOT_END_MOVE_MESSAGE, 0),
            ],
        );
        let actor = object_with_slots(
            e,
            ACTOR_TABLE,
            &[
                (ACTOR_SLOT_0X428, 0),
                (ACTOR_SLOT_0X358, 0),
                (ACTOR_SLOT_ANIMATION, 0),
            ],
        );
        returning(e, PACKAGE_TYPE, 0x16);
        returning_float(e, FRAME_TIME, 0.5);
        (p, actor, package)
    }

    #[test]
    fn test_high_process_process_flee_stops_without_a_flee_package() {
        let mut e = Engine::new();
        let (p, actor, _package) = flee_main_world(&mut e);
        returning(&mut e, PACKAGE_TYPE, 0x10);
        e.call_log = Some(vec![]);
        e.call(0x008dddf0, &args![p, actor]);
        assert!(calls_to(&e, FLEE_PACKAGE_UPDATE).is_empty());
        assert_eq!(
            calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)),
            vec![vec![p.addr(), actor, 1]]
        );
    }

    #[test]
    fn test_high_process_process_flee_only_advances_the_time_for_slot_0x358() {
        let mut e = Engine::new();
        let (p, actor, package) = flee_main_world(&mut e);
        give_vtable(&mut e, actor, ACTOR_TABLE, &[(ACTOR_SLOT_0X358, 1)]);
        returning_float(&mut e, FLEE_PACKAGE_TIME_VALUE, 1.5);
        e.call_log = Some(vec![]);
        e.call(0x008dddf0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, FLEE_PACKAGE_UPDATE),
            vec![vec![package, p.addr()]]
        );
        assert_eq!(calls_to(&e, FLEE_PACKAGE_SET_FLAG), vec![vec![package, 0]]);
        assert_eq!(
            calls_to(&e, FLEE_PACKAGE_ADD_TIME),
            vec![vec![package, 3.0f32.to_bits()]]
        );
        assert!(calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)).is_empty());
    }

    #[test]
    fn test_high_process_process_flee_idles_at_the_spot() {
        let mut e = Engine::new();
        let (p, actor, package) = flee_main_world(&mut e);
        // The package is the "stay" kind (flag 4); nothing to flee from.
        returning(&mut e, FLEE_PACKAGE_FLAG_574900, 1);
        returning(&mut e, PACKAGE_FLAG_4, 1);
        returning_float(&mut e, FLEE_PACKAGE_TIME_VALUE, 1.0);
        e.mem.set_f32(p.addr() + FLEE_IDLE_CHATTER_TIMER, 5.0);
        e.call_log = Some(vec![]);
        e.call(0x008dddf0, &args![p, actor]);
        assert_eq!(calls_to(&e, ACTOR_STOP_MOVING), vec![vec![actor]]);
        assert_eq!(calls_to(&e, FLEE_PACKAGE_SET_FLAG), vec![vec![package, 1]]);
        assert_eq!(
            calls_to(&e, process_slot(SLOT_END_MOVE_MESSAGE)),
            vec![vec![p.addr(), actor]]
        );
        // The chatter timer counts down by the frame time, the package
        // time advances by it and the package goes on (1.0 < 10.0).
        assert_eq!(e.mem.f32(p.addr() + FLEE_IDLE_CHATTER_TIMER), 4.5);
        assert_eq!(
            calls_to(&e, FLEE_PACKAGE_ADD_TIME),
            vec![vec![package, 0.5f32.to_bits()]]
        );
        assert!(calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)).is_empty());
        // Once the package time reaches the limit, procedure 1 is added.
        returning_float(&mut e, FLEE_PACKAGE_TIME_VALUE, 12.0);
        e.call_log = Some(vec![]);
        e.call(0x008dddf0, &args![p, actor]);
        assert_eq!(
            calls_to(&e, process_slot(SLOT_ADD_TO_PROCEDURE_INDEX_RUNNING)),
            vec![vec![p.addr(), actor, 1]]
        );
    }
}
