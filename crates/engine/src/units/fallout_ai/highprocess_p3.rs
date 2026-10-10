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
}
