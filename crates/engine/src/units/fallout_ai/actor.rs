//! `fallout/ai/actor.cpp` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! This is the unit's main file: the `Actor` layout and the helpers shared with
//! the part files (`actor_p2.rs` ... `actor_p7.rs`) are declared here. The
//! first block of this file (batch b0148) covers the two functions of the
//! range that sit far from the rest (`004b4880`, `00575610`) and `0087d4a0` up
//! to `0087f620`: the constructor and destructor, the ragdoll set-up and a run
//! of one-line accessors.
//!
//! Notes for the next session (this file's range is `00000000` up to
//! `00884990`, translated in blocks of 40 functions in address order):
//! - Block 1 (b0148) ends with `0087f620`; block 2 (`0087f660` up to
//!   `00881360`: editor location, disposition modifiers, the actor-value
//!   change functions) ends with `00881360`. The next function is `00881450`
//!   (`Actor::GetPackage`).
//! - Functions at `0x0088xxxx` whose `this` is the actor-value owner
//!   sub-object (`Actor + 0xa4`) take it as a plain `Ptr` and subtract `0xa4`.
//! - `004181e0` (the engine map calls it `BGSSaveFormBuffer::GetForm`, the
//!   linker folded it) returns the reference's base form; [`base_form`] wraps
//!   it. `005d43c0` returns the reference's `ExtraDataList`
//!   ([`extra_data_list`]).
//! - The Xbox PDB's `Actor` is 0x1c4 bytes with `TESForm` at 0x28; on PC the
//!   whole class sits `0x10` lower (0x1b4 bytes), so every field offset below is
//!   the PDB's minus `0x10`.
//! - This unit's code is debug-style (every argument in a stack slot); the
//!   decompiler hangs pushed words on the wrong call, so every translation was
//!   read from the disassembly. A `PUSH x` before a nested call whose own `RET`
//!   takes no argument belongs to the OUTER call.
//! - The one-line wrappers `0087f350`, `0087f370`, `0087f390`, `0087f3b0`,
//!   `0087f4a0` and `0087f4c0` all forward to a `TESActorBase` predicate of the
//!   actor's base form; their names are the addresses because the map has none.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleList;

layout! {
    /// `Actor` (Xbox PDB), `0x1b4` bytes on PC (the PDB's `0x1c4` minus the `0x10`
    /// bytes of `TESForm` the PC build lacks). Offsets are the PC ones; only
    /// the fields the translations use are listed.
    pub struct Actor: 0x1b4 {
        /// `pCurrentProcess` (Xbox PDB): `BaseProcess *`.
        0x68 pCurrentProcess: Ptr,
        /// `fUpdateTargetTimer` (Xbox PDB), randomised by the initializer.
        0x74 fUpdateTargetTimer: f32,
        /// `bIgnoreChangeAnimationCall` (Xbox PDB), cleared by the initializer.
        0x7E bIgnoreChangeAnimationCall: u8,
        /// `pRagdollController` (Xbox PDB): `bhkRagdollController *`.
        0xAC pRagdollController: Ptr,
        /// `pPenetrationDetection` (Xbox PDB): `bhkRagdollPenetrationUtil *`.
        0xB0 pPenetrationDetection: Ptr,
        /// `ePersuasionEmotion` (Xbox PDB).
        0xB4 ePersuasionEmotion: u32,
        /// `fEmotionValue` (Xbox PDB).
        0xB8 fEmotionValue: f32,
        /// `bProcessMe` (Xbox PDB).
        0xBC bProcessMe: bool,
        /// `pMyKiller` (Xbox PDB).
        0xC0 pMyKiller: Ptr,
        /// `bMurderAlarm` (Xbox PDB).
        0xC4 bMurderAlarm: bool,
        /// `fCheckMyDeadBodyTimer` (Xbox PDB).
        0xC8 fCheckMyDeadBodyTimer: f32,
        /// `fDeadBodyAlarm` (Xbox PDB).
        0xCC fDeadBodyAlarm: f32,
        /// `bBlockPostAnim` (Xbox PDB).
        0xF0 bBlockPostAnim: bool,
        /// `bReloadTargetQueued` (Xbox PDB).
        0xF1 bReloadTargetQueued: bool,
        /// `DispModifierList` (Xbox PDB `+0x10c`): `BSSimpleList<DispositionModifier *>`,
        /// the list head embedded in the actor.
        0xFC DispModifierList: Inline<BSSimpleList>,
        /// `bInCombat` (Xbox PDB).
        0x104 bInCombat: bool,
        /// `eLifeState` (Xbox PDB).
        0x108 eLifeState: u32,
        /// `eCriticalStage` (Xbox PDB).
        0x10C eCriticalStage: u32,
        /// `eQueuedattack` (Xbox PDB).
        0x110 eQueuedattack: u32,
        /// `fLastUpdate` (Xbox PDB).
        0x114 fLastUpdate: f32,
        /// `bDeadFlag` (Xbox PDB).
        0x118 bDeadFlag: bool,
        /// `iVisFlags` (Xbox PDB).
        0x11C iVisFlags: u32,
        /// `iLastSeenTime` (Xbox PDB).
        0x120 iLastSeenTime: u32,
        /// `bForceRun` (Xbox PDB).
        0x124 bForceRun: bool,
        /// `bForceSneak` (Xbox PDB).
        0x125 bForceSneak: bool,
        /// `bForceUpdateQuestTarget` (Xbox PDB).
        0x126 bForceUpdateQuestTarget: bool,
        /// `bSearchingInCombat` (Xbox PDB).
        0x127 bSearchingInCombat: bool,
        /// `pCurrentCombatTarget` (Xbox PDB).
        0x128 pCurrentCombatTarget: Ptr,
        /// `pCurrentCombatTargetArray` (Xbox PDB).
        0x12C pCurrentCombatTargetArray: Ptr,
        /// `pCurrentCombatMemberArray` (Xbox PDB).
        0x130 pCurrentCombatMemberArray: Ptr,
        /// `bAttackOnNextTheft` (Xbox PDB).
        0x134 bAttackOnNextTheft: bool,
        /// `iThiefCrimeStamp` (Xbox PDB).
        0x138 iThiefCrimeStamp: u32,
        /// `iMinorCrimes` (Xbox PDB).
        0x13C iMinorCrimes: u32,
        /// `iMajorCrimes` (Xbox PDB).
        0x140 iMajorCrimes: u32,
        /// `bIgnoreCrime` (Xbox PDB).
        0x144 bIgnoreCrime: bool,
        /// `bEVPBuffered` (Xbox PDB).
        0x145 bEVPBuffered: bool,
        /// `bResetAI` (Xbox PDB).
        0x146 bResetAI: bool,
        /// `pTemplateActorBase` (Xbox PDB).
        0x148 pTemplateActorBase: Ptr,
        /// `bInWater` (Xbox PDB).
        0x14C bInWater: bool,
        /// `bSwimming` (Xbox PDB).
        0x14D bSwimming: bool,
        /// `bUpdateLighting` (Xbox PDB).
        0x14E bUpdateLighting: bool,
        /// `iActionValue` (Xbox PDB).
        0x150 iActionValue: u32,
        /// `fTimeronAction` (Xbox PDB).
        0x154 fTimeronAction: f32,
        /// `fHeadTrackTimer` (Xbox PDB).
        0x158 fHeadTrackTimer: f32,
        /// `bWasInFrustum` (Xbox PDB).
        0x15C bWasInFrustum: bool,
        /// `bShouldRotateToTrack` (Xbox PDB).
        0x15D bShouldRotateToTrack: bool,
        /// `fEditorLocZRot` (Xbox PDB).
        0x16C fEditorLocZRot: f32,
        /// `pEditorLocForm` (Xbox PDB).
        0x170 pEditorLocForm: Ptr,
        /// `bSetOnDeath` (Xbox PDB).
        0x174 bSetOnDeath: bool,
        /// `bContainerReset` (Xbox PDB).
        0x175 bContainerReset: bool,
        /// `bFootIKInRange` (Xbox PDB).
        0x18C bFootIKInRange: bool,
        /// `bPlayerTeammate` (Xbox PDB).
        0x18D bPlayerTeammate: bool,
        /// `bLightingUpdatedNonMoving` (Xbox PDB).
        0x18E bLightingUpdatedNonMoving: bool,
        /// `pActorMover` (Xbox PDB).
        0x190 pActorMover: Ptr,
        /// `pLastHitData` (Xbox PDB).
        0x194 pLastHitData: Ptr,
        /// `pInitialPackage` (Xbox PDB).
        0x198 pInitialPackage: Ptr,
        /// `pContinuousBeamPersistant` (Xbox PDB).
        0x1A0 pContinuousBeamPersistant: Ptr,
        /// `iEmotion` (Xbox PDB).
        0x1A4 iEmotion: u32,
        /// `iEmotionValue` (Xbox PDB).
        0x1A8 iEmotionValue: u32,
        /// `cCurrentSitSleepState` (Xbox PDB).
        0x1AC cCurrentSitSleepState: u32,
        /// `bTurretBehavior` (Xbox PDB).
        0x1B0 bTurretBehavior: u8,
        /// `bForceHitReaction` (Xbox PDB).
        0x1B1 bForceHitReaction: bool,
    }

    /// `DispositionModifier` (Xbox PDB), 8 bytes: one entry of
    /// [`Actor::DispModifierList`].
    pub struct DispositionModifier: 0x08 {
        /// `modifieramount` (Xbox PDB).
        0x00 modifieramount: i32,
        /// `pactormodified` (Xbox PDB): `Actor *`.
        0x04 pactormodified: Ptr,
    }

    /// `bhkRagdollController` (Xbox PDB, `0x2d0` bytes there; the PC size is not
    /// confirmed). Only the fields that `actor.cpp` reaches are listed, at PC
    /// offsets; the PDB's layout differs from the PC one past `+0x60`, so the
    /// fields past it are named by their offset.
    pub struct RagdollController: 0x2d0 {
        /// `bInitRagdollAnim` (Xbox PDB), the flag `0087ea20` reads.
        0x42 bInitRagdollAnim: u8,
        /// `iGroup` (Xbox PDB), set by `0087ea40`.
        0x5C iGroup: u32,
        /// The word `0087ea80` sets from `bhkCharacterProxy::operator P` (PC
        /// only; the PDB has `spSkeletonDebug` at `+0x90`).
        0x90 pCharacterProxy: Ptr,
        /// Byte set by `0087ea60` (PC offset; the PDB has other fields here).
        0x224 bFlag224: u8,
        /// Byte `0087e9d0` reads (PC offset).
        0x254 bFlag254: u8,
        /// Byte `0087e9d0` writes (PC offset).
        0x255 bFlag255: u8,
    }
}

// ---------------------------------------------------------------------------
// Callees outside this file

/// `TESObjectREFR`'s base form (`BGSSaveFormBuffer::GetForm` in the engine map:
/// the linker folded it with `TESForm`'s getter).
const GET_BASE_FORM: u32 = 0x0041_81e0;
/// `007af430`: the same getter under another address (`GetForm`).
const GET_FORM: u32 = 0x007a_f430;
/// `005d43c0`: the reference's `ExtraDataList`.
const GET_EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// `MobileObject::Load3D(bool)`.
const MOBILE_OBJECT_LOAD3D: u32 = 0x0093_35b0;
/// `Actor::ClearInCombat(bool)`.
const CLEAR_IN_COMBAT: u32 = 0x008a_08e0;
/// `Actor::StopCombat(bool)`.
const STOP_COMBAT: u32 = 0x008a_06c0;
/// Sets or clears the bits `mask` of the word at `this + 0x30`: `(this, set, mask)`.
const SET_FLAG_BITS: u32 = 0x0043_b370;
/// `operator new(size)` (cdecl).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// Allocates `size` bytes from the engine's memory manager (cdecl).
const ALLOCATE_BLOCK: u32 = 0x0055_3300;
/// Releases a memory-manager block (cdecl).
const FREE_BLOCK: u32 = 0x0055_3380;
/// `ExtraDataList::ExtraDataList()` (`0x20` bytes).
const EXTRA_DATA_LIST_CONSTRUCTOR: u32 = 0x0041_0360;
/// `__RTDynamicCast(object, 0, source type, target type, 0)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// Returns a pointer to a setting's value byte (`ECX` is the setting).
const SETTING_VALUE_POINTER: u32 = 0x0040_8d60;
/// `TESObjectREFR::Set3D(node, flag)`.
const SET_3D: u32 = 0x0057_02e0;
/// `PlayerCharacter::RemoveActorFromPlayercombatList`.
const REMOVE_FROM_PLAYER_COMBAT_LIST: u32 = 0x0093_a660;
/// `PlayerCharacter::RemovePerceivedActor`.
const REMOVE_PERCEIVED_ACTOR: u32 = 0x0096_7350;

/// `ProcessLists *` (the singleton object itself).
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// The `LipSyncBackgroundManager` singleton.
const LIP_SYNC_MANAGER: u32 = 0x011f_11e0;
/// `PlayerCharacter *`.
const PLAYER_POINTER: u32 = 0x011d_ea3c;
/// A global pointer to the object that `004226e0` is called on.
const SHUTDOWN_CHECK_OBJECT_POINTER: u32 = 0x011c_3f2c;
/// `ModelLoader *`.
const MODEL_LOADER_POINTER: u32 = 0x011c_3b3c;
/// The global word `0087e9a0` returns.
const GLOBAL_VALUE_011C625C: u32 = 0x011c_625c;
/// The identity matrix the ragdoll set-up copies (9 floats).
const IDENTITY_MATRIX: u32 = 0x011a_9448;
/// `0.01745329238474369` (`double`), degrees to radians.
const DEGREES_TO_RADIANS: u32 = 0x0102_3128;
/// The format `"%s_%08X_%08X"` and the error and file-name strings of the
/// ragdoll set-up.
const RAGDOLL_NAME_FORMAT: u32 = 0x0108_4820;
const RAGDOLL_ERROR_CONTROLLER: u32 = 0x0108_47ec;
const RAGDOLL_ERROR_LOOK_IK: u32 = 0x0108_47bc;
const RAGDOLL_ERROR_FOOT_IK: u32 = 0x0108_4784;
const PRIME_TEXT: u32 = 0x0108_47b4;
const DEATH_POSE_FILE: u32 = 0x0108_4778;

// ---------------------------------------------------------------------------
// Helpers shared with the part files

/// `TESObjectREFR`'s base form (`004181e0`).
pub(crate) fn base_form(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    e.call(GET_BASE_FORM, &args![this]).ptr()
}

/// The reference's `ExtraDataList` (`005d43c0`).
pub(crate) fn extra_data_list(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    e.call(GET_EXTRA_DATA_LIST, &args![this]).ptr()
}

// ---------------------------------------------------------------------------
// Translations

// Translated from 004b4880 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTransform::InvertNonUniform` (Xbox PDB): writes into `result` the inverse
/// of the transform `this` (rotation at `+0`, translate at `+0x24`, scale at
/// `+0x30`): the transposed rotation, `1 / scale`, and the translation
/// `scale' * (R' * (-t))` computed with the helpers `004a0bd0`, `004b4500` and
/// `004a3760`.
pub fn ni_transform_invert_non_uniform(e: &mut Engine, this: Ptr, result: Ptr) {
    // Locals: the transposed matrix (0x24 bytes) and three point temporaries.
    e.with_stack(0x80, |e, frame| {
        let matrix = frame;
        let negated_slot = frame.byte_add(0x30);
        let rotated_slot = frame.byte_add(0x40);
        let scaled_slot = frame.byte_add(0x50);
        let transposed: Ptr = e.call(0x0047_68c0, &args![this, matrix]).ptr();
        for i in 0..9 {
            let word = e.mem.u32(transposed.addr() + 4 * i);
            e.mem.set_u32(result.addr() + 4 * i, word);
        }
        let scale = e.mem.f32(this.addr() + 0x30);
        let inverse = (1.0f64 / scale as f64) as f32;
        e.mem.set_f32(result.addr() + 0x30, inverse);
        let translate = this.byte_add(0x24);
        let negated: Ptr = e.call(0x004a_0bd0, &args![translate, negated_slot]).ptr();
        let rotated: Ptr = e
            .call(0x004b_4500, &args![result, rotated_slot, negated])
            .ptr();
        let scale_now = e.mem.f32(result.addr() + 0x30);
        let scaled: Ptr = e
            .call(0x004a_3760, &args![scaled_slot, scale_now, rotated])
            .ptr();
        for i in 0..3 {
            let word = e.mem.u32(scaled.addr() + 4 * i);
            e.mem.set_u32(result.addr() + 0x24 + 4 * i, word);
        }
    });
}

// Translated from 00575610 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetItemCountinContainer` (Xbox PDB): the count of `item` in the
/// actor's inventory changes (0 when it has none).
pub fn actor_get_item_countin_container(e: &mut Engine, this: Ptr<Actor>, item: Ptr) -> u32 {
    let changes: Ptr = e.call(0x004b_f220, &args![this]).ptr();
    if changes.is_null() {
        0
    } else {
        e.call(0x004c_8f30, &args![changes, item]).u32()
    }
}

// Translated from 0087d4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::Actor` (Xbox PDB): constructs the base `MobileObject`, the parts at
/// `+0x88`, `+0x94`, `+0xa4` and `+0xa8`, installs the six vtables, builds the
/// two modifier lists (`+0xd0`, `+0xe0`), the two simple lists (`+0xf4`,
/// `+0xfc`) and the point at `+0x160`, then runs the initializer `0087d890`.
/// The compiler's exception-unwinding frame is not translated.
pub fn actor_constructor(e: &mut Engine, this: Ptr<Actor>, add_to_process_lists: bool) -> Ptr {
    e.call(0x0092_eb50, &args![this]);
    e.call(0x0081_5300, &args![this.byte_add(0x88)]);
    e.call(0x0082_2a40, &args![this.byte_add(0x94)]);
    e.call(0x005f_78e0, &args![this.byte_add(0xa4)]);
    fn_0087d860(e, this.byte_add(0xa8));
    e.mem.set_u32(this.addr(), 0x0108_4254);
    e.mem.set_u32(this.addr() + 0x18, 0x0108_4248);
    e.mem.set_u32(this.addr() + 0x88, 0x0108_41f4);
    e.mem.set_u32(this.addr() + 0x94, 0x0108_41bc);
    e.mem.set_u32(this.addr() + 0xa4, 0x0108_418c);
    e.mem.set_u32(this.addr() + 0xa8, 0x0108_4144);
    e.call(0x0093_7000, &args![this.byte_add(0xd0)]);
    e.call(0x0093_7000, &args![this.byte_add(0xe0)]);
    e.call(0x0096_a2d0, &args![this.byte_add(0xf4)]);
    e.call(0x0096_a2d0, &args![this.byte_add(0xfc)]);
    e.call(0x0068_15c0, &args![this.byte_add(0x160)]);
    fn_0087d890(e, this, add_to_process_lists);
    this.cast()
}

// Translated from 0087d860 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the small object at `Actor + 0xa8`: installs its vtable
/// (`01084734`) and returns it.
pub fn fn_0087d860(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), 0x0108_4734);
    this.cast()
}

// Translated from 0087d890 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor`'s field initializer (called by the constructor): creates the
/// process object (`0xb4` bytes, `00906dc0`), registers the actor with
/// `ProcessLists` when asked (`add_to_process_lists`), and gives every field
/// its starting value, in the order of the exe.
pub fn fn_0087d890(e: &mut Engine, this: Ptr<Actor>, add_to_process_lists: bool) {
    e.call(0x0046_a010, &args![this, 1u32]);
    let block = e.call(OPERATOR_NEW, &args![0xb4u32]).u32();
    let process: Ptr = if block != 0 {
        e.call(0x0090_6dc0, &args![block]).ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, Actor::pCurrentProcess, process);
    if add_to_process_lists {
        if e.call(0x008c_7aa0, &args![]).bool() {
            e.call(0x0096_e870, &args![PROCESS_LISTS, this]);
        } else {
            e.call(
                0x0096_d450,
                &args![PROCESS_LISTS, this, 3u32, 0u32, 0u32, 0u32],
            );
        }
    }
    let default_last_update: f32 = e.global(0x0101_2054);
    e.set(this, Actor::fLastUpdate, default_last_update);
    e.set(this, Actor::iLastSeenTime, 0);
    e.set(this, Actor::eLifeState, 0);
    e.set(this, Actor::eCriticalStage, 0);
    e.set(this, Actor::bDeadFlag, false);
    e.set(this, Actor::ePersuasionEmotion, 8);
    e.set(this, Actor::bProcessMe, true);
    e.set(this, Actor::bMurderAlarm, false);
    e.set(this, Actor::fEmotionValue, 0.0);
    e.set(this, Actor::bForceSneak, false);
    e.set(this, Actor::bForceRun, false);
    e.set(this, Actor::pMyKiller, Ptr::NULL);
    // MobileObject's word at +0x70 (PDB +0x80, `pDialogueItemTarget`).
    e.mem.set_u32(this.addr() + 0x70, 0);
    e.set(this, Actor::pTemplateActorBase, Ptr::NULL);
    e.set(this, Actor::bForceUpdateQuestTarget, true);
    e.set(this, Actor::bInWater, false);
    e.set(this, Actor::fHeadTrackTimer, 0.0);
    e.set(this, Actor::bWasInFrustum, true);
    // The random delay before the first target update: `RandomFloat(low,
    // high)` of two game settings (`00403e20` returns a pointer to a
    // setting's value; the setting at `011df810` is the first argument).
    let high_pointer = e.call(0x0040_3e20, &args![0x011d_f828u32]).u32();
    let high = e.mem.f32(high_pointer);
    let low_pointer = e.call(0x0040_3e20, &args![0x011d_f810u32]).u32();
    let low = e.mem.f32(low_pointer);
    let random = e.call(0x0047_6b70, &args![low, high]).f32();
    e.set(this, Actor::fUpdateTargetTimer, random);
    e.set(this, Actor::iVisFlags, 0);
    e.set(this, Actor::pEditorLocForm, Ptr::NULL);
    e.set(this, Actor::fEditorLocZRot, 0.0);
    // `EditorLocCoord` (a `NiPoint3` at +0x160) starts as the three words at 011f426c.
    for i in 0..3 {
        let word: u32 = e.global(0x011f_426c + 4 * i);
        e.mem.set_u32(this.addr() + 0x160 + 4 * i, word);
    }
    e.set(this, Actor::fCheckMyDeadBodyTimer, 0.0);
    let default_dead_body_alarm: f32 = e.global(0x0101_7718);
    e.set(this, Actor::fDeadBodyAlarm, default_dead_body_alarm);
    e.set(this, Actor::bSetOnDeath, false);
    e.set(this, Actor::bContainerReset, true);
    e.set(this, Actor::eQueuedattack, 0xff);
    e.vcall(this.addr(), 0x4d0, &args![]);
    e.call(CLEAR_IN_COMBAT, &args![this, 0u32]);
    e.set(this, Actor::bSwimming, false);
    e.set(this, Actor::pRagdollController, Ptr::NULL);
    let block = e.call(ALLOCATE_BLOCK, &args![0x30u32]).u32();
    let detection: Ptr = if block != 0 {
        e.call(0x00ca_0dc0, &args![block]).ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, Actor::pPenetrationDetection, detection);
    e.set(this, Actor::bFootIKInRange, false);
    e.call(0x008b_bdb0, &args![this]);
    e.call(0x008b_be00, &args![this]);
    e.vcall(this.addr(), 0x3c4, &args![]);
    e.call(0x008c_4400, &args![this]);
    e.set(this, Actor::iActionValue, 0);
    e.set(this, Actor::fTimeronAction, 0.0);
    e.set(this, Actor::bReloadTargetQueued, false);
    e.call(0x0093_7090, &args![this.byte_add(0xe0), 1u32]);
    e.set(this, Actor::bEVPBuffered, false);
    e.set(this, Actor::bResetAI, false);
    e.set(this, Actor::bIgnoreChangeAnimationCall, 0);
    e.set(this, Actor::bBlockPostAnim, false);
    e.set(this, Actor::iEmotion, 0);
    e.set(this, Actor::iEmotionValue, 0);
    e.set(this, Actor::bPlayerTeammate, false);
    e.set(this, Actor::bSearchingInCombat, false);
    e.set(this, Actor::pCurrentCombatTarget, Ptr::NULL);
    e.set(this, Actor::pCurrentCombatTargetArray, Ptr::NULL);
    e.set(this, Actor::pCurrentCombatMemberArray, Ptr::NULL);
    e.set(this, Actor::cCurrentSitSleepState, 0);
    e.set(this, Actor::bLightingUpdatedNonMoving, false);
    e.set(this, Actor::pLastHitData, Ptr::NULL);
    e.set(this, Actor::bShouldRotateToTrack, true);
    e.set(this, Actor::pInitialPackage, Ptr::NULL);
    e.set(this, Actor::bIgnoreCrime, false);
    e.set(this, Actor::bTurretBehavior, 0);
    e.set(this, Actor::pContinuousBeamPersistant, Ptr::NULL);
    e.set(this, Actor::bForceHitReaction, false);
    e.set(this, Actor::iMajorCrimes, 0);
    e.set(this, Actor::iMinorCrimes, 0);
}

// Translated from 0087dcb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::~Actor` (Xbox PDB), the body: puts the vtables back, deletes the
/// ragdoll controller and the penetration utility, releases the sound handle of
/// the reference's extra data, switches the NPC radio off, and, unless the
/// shutdown check `004077c0` says the world is going away, takes the actor out
/// of the player's combat and perceived lists, the process lists, the lip-sync
/// manager, its magic, and its process; then runs the destructors of the
/// members and the `MobileObject` base. The scalar-deleting wrapper and the
/// exception-unwinding frame are not translated.
pub fn actor_destructor(e: &mut Engine, this: Ptr<Actor>) {
    e.mem.set_u32(this.addr(), 0x0108_4254);
    e.mem.set_u32(this.addr() + 0x18, 0x0108_4248);
    e.mem.set_u32(this.addr() + 0x88, 0x0108_41f4);
    e.mem.set_u32(this.addr() + 0x94, 0x0108_41bc);
    e.mem.set_u32(this.addr() + 0xa4, 0x0108_418c);
    e.mem.set_u32(this.addr() + 0xa8, 0x0108_4144);
    let ragdoll = e.get(this, Actor::pRagdollController);
    if !ragdoll.is_null() {
        e.vcall(ragdoll.addr(), 0, &args![1u32]);
    }
    let detection = e.get(this, Actor::pPenetrationDetection);
    if !detection.is_null() {
        e.vcall(detection.addr(), 0, &args![1u32]);
    }
    // The sound handle of the extra data: constructed, fetched, released if
    // valid and destroyed at the end.
    let handle: Ptr = Ptr::new(e.mem.alloc(0x10));
    e.call(0x0041_a250, &args![handle]);
    let list = extra_data_list(e, this);
    e.call(0x0041_8890, &args![list, handle]);
    if e.call(0x00ad_8ce0, &args![handle]).bool() {
        e.call(0x00ad_8d10, &args![handle]);
    }
    e.call(0x0083_5980, &args![this]);
    if !e.call(0x0040_77c0, &args![this]).bool() {
        let player = e.global::<u32>(PLAYER_POINTER);
        e.call(REMOVE_FROM_PLAYER_COMBAT_LIST, &args![player, this]);
        e.call(REMOVE_PERCEIVED_ACTOR, &args![player, this]);
        e.call(0x0096_e6f0, &args![PROCESS_LISTS, this]);
        e.call(0x0087_fd20, &args![this]);
        e.call(0x0090_6050, &args![LIP_SYNC_MANAGER, this]);
        e.call(0x0097_8660, &args![PROCESS_LISTS, this]);
        let shutdown_object = e.global::<u32>(SHUTDOWN_CHECK_OBJECT_POINTER);
        if !e.call(0x0042_26e0, &args![shutdown_object]).bool() {
            e.call(0x0082_4970, &args![this.byte_add(0x94)]);
            e.call(0x0066_e100, &args![this]);
            e.call(0x0092_ed60, &args![this, 1u32]);
            e.call(0x0088_1680, &args![this, 0u32]);
            e.call(0x0097_6680, &args![PROCESS_LISTS, this]);
            e.call(0x0096_f600, &args![PROCESS_LISTS, this, 1u32]);
            e.call(0x0097_5320, &args![PROCESS_LISTS, this]);
            let process = e.get(this, Actor::pCurrentProcess);
            if !process.is_null() {
                let kind = e.call(0x0045_cd60, &args![process]).i32();
                if kind == 1 || e.call(0x0045_cd60, &args![process]).i32() == 0 {
                    let accumulator = e.call(0x00b4_f5c0, &args![]).u32();
                    if accumulator != 0 {
                        let value = e.call(0x0084_e3a0, &args![this]).u32();
                        e.call(0x00b6_6050, &args![accumulator, value]);
                    }
                    e.call(0x008c_30a0, &args![this]);
                }
                e.call(0x008c_51f0, &args![this]);
            }
        }
        e.call(0x0096_f400, &args![PROCESS_LISTS, this]);
        if e.get(this, Actor::bInCombat) {
            e.call(STOP_COMBAT, &args![this, 0u32]);
        }
        e.call(0x008c_5090, &args![this]);
        e.call(0x0093_70b0, &args![this.byte_add(0xd0)]);
        e.call(0x0093_70b0, &args![this.byte_add(0xe0)]);
        e.call(0x008b_3180, &args![this]);
        e.call(SET_3D, &args![this, 0u32, 0u32]);
    }
    e.call(0x0048_3710, &args![handle]);
    e.mem.free(handle.addr());
    e.call(0x0046_ffb0, &args![this.byte_add(0xfc)]);
    e.call(0x0046_ffb0, &args![this.byte_add(0xf4)]);
    e.call(0x0093_7030, &args![this.byte_add(0xe0)]);
    e.call(0x0093_7030, &args![this.byte_add(0xd0)]);
    e.call(0x0082_2a80, &args![this.byte_add(0x94)]);
    e.call(0x0081_53b0, &args![this.byte_add(0x88)]);
    e.call(0x0092_ec40, &args![this]);
}

// Translated from 0087e060 (decompiled, FalloutNV.exe 1.4.0.525)
/// Wrapper around `MobileObject::Load3D(flag)`: when it returns a node, hands
/// it to the process (virtual `+0x6c8`, with the base form's virtual `+0x180`
/// result), clears the node's `0x800000` bits unless the setting at `011c7664`
/// is on, and tells `008b7360` about it when `008b7b70` is true. Returns the
/// node.
pub fn fn_0087e060(e: &mut Engine, this: Ptr<Actor>, flag: bool) -> Ptr {
    let node: Ptr = e.call(MOBILE_OBJECT_LOAD3D, &args![this, flag]).ptr();
    if !node.is_null() {
        let process = e.get(this, Actor::pCurrentProcess);
        if !process.is_null() {
            let base = base_form(e, this);
            let model = e.vcall(base.addr(), 0x180, &args![]).u32();
            e.vcall(process.addr(), 0x6c8, &args![node, model]);
        }
        let setting = e.call(SETTING_VALUE_POINTER, &args![0x011c_7664u32]).u32();
        if e.mem.u8(setting) == 0 {
            fn_0087e100(e, node, false);
        }
        if e.call(0x008b_7b70, &args![this]).bool() {
            e.call(0x008b_7360, &args![this, 1u32, 0u32]);
        }
    }
    node
}

// Translated from 0087e100 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` true) or clears the `0x800000` bits of the node's flag word at
/// `+0x30` (`0043b370(node, !flag, 0x800000)`: the helper's second argument is
/// the "set" request, so a false `flag` asks for them to be set).
pub fn fn_0087e100(e: &mut Engine, node: Ptr, flag: bool) {
    e.call(SET_FLAG_BITS, &args![node, !flag, 0x0080_0000u32]);
}

// Translated from 0087e130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::CreateRagdollInstance` (Xbox PDB). `biped` is the word given to
/// `004ade00` and to the controller's constructor (`bipedanim.cpp`);
/// `replace_existing` deletes the current controller first; `init_iks` runs the
/// look-at and foot IK set-up over the model's nodes; `unique_name` appends the
/// form id and reference to the model name.
///
/// Builds the ragdoll controller (`0x2c0` bytes, `00c7f060`), initializes its
/// look-at and foot IK (when asked), its pose matching from the `Death.psa`
/// file next to the model, and its character proxy. An actor whose base is
/// immobile and which is not in a world space only gets its turret flag set
/// (`0087e960`) and returns. The exception-unwinding frame and the stack
/// cookie check are not translated.
pub fn actor_create_ragdoll_instance(
    e: &mut Engine,
    this: Ptr<Actor>,
    biped: Ptr,
    replace_existing: bool,
    init_iks: bool,
    unique_name: bool,
) {
    let base = base_form(e, this);
    let node: Ptr = e.vcall(base.addr(), 0x180, &args![]).ptr();
    let world_space: Ptr = if node.is_null() {
        Ptr::NULL
    } else {
        e.call(0x004f_d380, &args![node]).ptr()
    };
    let existing = e.get(this, Actor::pRagdollController);
    if !existing.is_null() && replace_existing {
        e.vcall(existing.addr(), 0, &args![1u32]);
    }
    let global_word = fn_0087e9a0(e);
    let found = e.call(0x004a_de00, &args![biped, global_word]).u32();
    if found != 0 {
        let base_again = base_form(e, this);
        if e.call(0x005f_0c80, &args![base_again]).bool() && world_space.is_null() {
            fn_0087e960(e, this, true);
            return;
        }
    }
    let form: Ptr = e.call(GET_FORM, &args![this]).ptr();
    let model_file = e.call(0x0050_fd90, &args![form, this]).u32();
    let form_again: Ptr = e.call(GET_FORM, &args![this]).ptr();
    let lod_mult = e.call(0x0045_c6b0, &args![form_again]).u32();
    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
    let loaded = e
        .call(
            0x0044_7080,
            &args![loader, model_file, lod_mult, 1u32, 0u32, 0u32, 1u32],
        )
        .u32();
    // Every local of the exe's frame gets a slot of its own here.
    let frame: Ptr = Ptr::new(e.mem.alloc(0x1000));
    let name_buffer = frame; // char[0x104]: the model file name
    let path_buffer = frame.byte_add(0x120); // char[0x104]: the directory of the model
    let matrix = frame.byte_add(0x240); // NiMatrix3 (0x24)
    let matrix_result = frame.byte_add(0x280); // NiMatrix3 out (0x24)
    let bit_set = frame.byte_add(0x2c0); // QuickBitSet (8)
    let array = frame.byte_add(0x2e0); // BSSimpleArray (0x10)
    let slot_word = frame.byte_add(0x300); // a word passed by address
    let biped_out = frame.byte_add(0x310); // out word of 00931ed0
    let pose_file = frame.byte_add(0x320); // object built by 00438170
    let pose_file_flag = frame.byte_add(0x330); // object built by 00633c90

    let form_third: Ptr = e.call(GET_FORM, &args![this]).ptr();
    e.call(0x0050_fdf0, &args![form_third, this, name_buffer]);
    if unique_name {
        let old = e.get(this, Actor::pRagdollController);
        e.call(
            0x0040_6d00,
            &args![
                name_buffer,
                0x104u32,
                RAGDOLL_NAME_FORMAT,
                name_buffer,
                this,
                old
            ],
        );
    }
    let block = e.call(ALLOCATE_BLOCK, &args![0x2c0u32]).u32();
    let controller: Ptr = if block != 0 {
        e.call(0x00c7_f060, &args![block, biped, loaded, name_buffer])
            .ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, Actor::pRagdollController, controller);
    if fn_0087ea20(e, controller) == 0 {
        let name = ragdoll_error_name(e, this);
        e.call(0x005b_5e40, &args![RAGDOLL_ERROR_CONTROLLER, name]);
        e.mem.free(frame.addr());
        return;
    }
    e.call(0x0068_15c0, &args![matrix]);
    for i in 0..9 {
        let word: u32 = e.global(IDENTITY_MATRIX + 4 * i);
        e.mem.set_u32(matrix.addr() + 4 * i, word);
    }
    e.call(0x0056_fac0, &args![this, matrix_result, matrix]);
    let controller = e.get(this, Actor::pRagdollController);
    let basis = e.call(0x004e_a950, &args![controller]).u32();
    e.call(0x004b_46a0, &args![matrix, basis]);
    if init_iks {
        e.call(0x0096_a2d0, &args![bit_set]);
        e.call(0x005e_5320, &args![node, bit_set]);
        e.call(0x005e_ede0, &args![array]);
        let mut look_ik_done = false;
        while !e.call(0x0082_56d0, &args![bit_set]).bool() {
            let current_slot = e.call(0x0068_15c0, &args![bit_set]).u32();
            let current = Ptr::new(e.mem.u32(current_slot));
            e.call(0x0063_f7b0, &args![bit_set]);
            if fn_0087e920(e, current) {
                let part = e.call(0x0063_d040, &args![current]).u32();
                e.mem.set_u32(slot_word.addr(), part);
                e.call(0x007c_b2e0, &args![array, slot_word]);
            }
            if !look_ik_done && fn_0087e940(e, current) {
                if !e.vcall(this.addr(), 0x21c, &args![]).bool() {
                    let controller = e.get(this, Actor::pRagdollController);
                    e.call(0x004e_4660, &args![controller, 1u32]);
                }
                look_ik_done = true;
                let part = e.call(0x0063_d040, &args![current]).u32();
                let controller = e.get(this, Actor::pRagdollController);
                e.call(0x00c7_de60, &args![controller, part]);
                let degrees = e.call(0x0051_b860, &args![current]).f32();
                let factor: f64 = e.global(DEGREES_TO_RADIANS);
                let radians = (degrees as f64 * factor) as f32;
                let controller = e.get(this, Actor::pRagdollController);
                e.call(0x0060_7810, &args![controller, 1u32, radians]);
                let controller = e.get(this, Actor::pRagdollController);
                e.call(0x005b_a4f0, &args![controller, 1u32]);
                let controller = e.get(this, Actor::pRagdollController);
                if !e.call(0x005b_a540, &args![controller]).bool() {
                    let name = ragdoll_error_name(e, this);
                    e.call(0x005b_5e40, &args![RAGDOLL_ERROR_LOOK_IK, name]);
                }
            }
        }
        let node_name = e.vcall(node.addr(), 0x130, &args![]).u32();
        if e.call(0x00ec_7750, &args![node_name, PRIME_TEXT]).u32() != 0 {
            let controller = e.get(this, Actor::pRagdollController);
            e.call(0x005c_6b40, &args![controller, 1u32]);
            let controller = e.get(this, Actor::pRagdollController);
            fn_0087ea60(e, controller, 0);
        }
        let controller = e.get(this, Actor::pRagdollController);
        e.call(0x00c7_c3a0, &args![controller, array]);
        let controller = e.get(this, Actor::pRagdollController);
        e.call(0x005b_a130, &args![controller, 1u32]);
        let controller = e.get(this, Actor::pRagdollController);
        e.call(0x005b_a310, &args![controller, 1u32]);
        let controller = e.get(this, Actor::pRagdollController);
        if !e.call(0x005b_a180, &args![controller]).bool()
            && e.call(0x0044_ddc0, &args![array]).u32() != 0
        {
            let name = ragdoll_error_name(e, this);
            e.call(0x005b_5e40, &args![RAGDOLL_ERROR_FOOT_IK, name]);
        }
        let controller = e.get(this, Actor::pRagdollController);
        if e.call(0x005b_a360, &args![controller]).bool() {
            e.call(0x008b_c300, &args![this]);
        }
        e.call(0x005e_ee10, &args![array]);
        e.call(0x0046_ffb0, &args![bit_set]);
    }
    e.call(0x0093_1ed0, &args![this, biped_out]);
    let group = e.call(0x004a_3a20, &args![biped_out]).u32();
    let controller = e.get(this, Actor::pRagdollController);
    fn_0087ea40(e, controller, group);
    // The model's file path, cut after its last backslash, plus "Death.psa".
    let model_path = e.vcall(node.addr() + 0x18, 0x14, &args![]).u32();
    e.call(0x0040_6d30, &args![path_buffer, 0x104u32, model_path]);
    let last_slash = e.call(0x0040_ab30, &args![path_buffer, 0x5cu32]).u32();
    if last_slash != 0 {
        e.mem.set_u8(last_slash + 1, 0);
        e.call(0x0040_6d50, &args![path_buffer, 0x104u32, DEATH_POSE_FILE]);
        e.call(0x0043_8170, &args![pose_file, path_buffer]);
        e.call(0x0063_3c90, &args![pose_file_flag, 0u32]);
        let pose = e.call(0x0043_b1b0, &args![pose_file]).u32();
        let controller = e.get(this, Actor::pRagdollController);
        e.call(
            0x00c7_b1f0,
            &args![
                controller,
                pose,
                0xffff_ffffu32,
                0xffff_ffffu32,
                0xffff_ffffu32
            ],
        );
        e.call(0x0045_cec0, &args![pose_file_flag]);
        e.call(0x0043_81b0, &args![pose_file]);
    }
    if !world_space.is_null() && this.addr() != e.global::<u32>(PLAYER_POINTER) {
        let area = fn_0087eaa0(e, world_space);
        let controller = e.get(this, Actor::pRagdollController);
        e.call(0x00c7_5ad0, &args![controller, area]);
    }
    let controller = e.get(this, Actor::pRagdollController);
    fn_0087e9d0(e, controller, true);
    let character_controller = e.call(0x0093_06d0, &args![this]).u32();
    if character_controller != 0 {
        let proxy = fn_0087e980(e, Ptr::new(character_controller));
        let controller = e.get(this, Actor::pRagdollController);
        fn_0087ea80(e, controller, Ptr::new(proxy));
    }
    e.mem.free(frame.addr());
}

/// The name an error message of the ragdoll set-up gives the actor: the
/// virtual `+0x130` when `00474cb0` says it has one, `0055d520` otherwise.
fn ragdoll_error_name(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    if e.call(0x0047_4cb0, &args![this]).u32() != 0 {
        e.vcall(this.addr(), 0x130, &args![]).u32()
    } else {
        e.call(0x0055_d520, &args![this]).u32()
    }
}

// Translated from 0087e920 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit `4` of the byte at `+0x60` of a model node (`NiAVObject`).
pub fn fn_0087e920(e: &mut Engine, node: Ptr) -> bool {
    e.mem.u8(node.addr() + 0x60) & 4 != 0
}

// Translated from 0087e940 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit `0x20` of the byte at `+0x60` of a model node (`NiAVObject`).
pub fn fn_0087e940(e: &mut Engine, node: Ptr) -> bool {
    e.mem.u8(node.addr() + 0x60) & 0x20 != 0
}

// Translated from 0087e960 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `bTurretBehavior` (Xbox PDB `+0x1c0`).
pub fn fn_0087e960(e: &mut Engine, this: Ptr<Actor>, value: bool) {
    e.set(this, Actor::bTurretBehavior, value as u8);
}

// Translated from 0087e980 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word `00559450` reads at `+0x594` of the character controller (its
/// first word: the pointer stored there).
pub fn fn_0087e980(e: &mut Engine, character_controller: Ptr) -> u32 {
    e.call(0x0055_9450, &args![character_controller.byte_add(0x594)])
        .u32()
}

// Translated from 0087e9a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The global word at `011c625c`.
pub fn fn_0087e9a0(e: &mut Engine) -> u32 {
    e.global(GLOBAL_VALUE_011C625C)
}

// Translated from 0087e9b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases a memory-manager block (`00553380`, cdecl).
pub fn fn_0087e9b0(e: &mut Engine, block: Ptr) {
    e.call(FREE_BLOCK, &args![block]);
}

// Translated from 0087e9d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// On a `bhkRagdollController`: sets the byte at `+0x255` to `[+0x254] && flag`.
pub fn fn_0087e9d0(e: &mut Engine, this: Ptr, flag: bool) {
    let this: Ptr<RagdollController> = this.cast();
    let on = e.get(this, RagdollController::bFlag254) != 0 && flag;
    e.set(this, RagdollController::bFlag255, on as u8);
}

// Translated from 0087ea20 (decompiled, FalloutNV.exe 1.4.0.525)
/// On a `bhkRagdollController`: the byte at `+0x42` (`bInitRagdollAnim`).
pub fn fn_0087ea20(e: &mut Engine, this: Ptr) -> u8 {
    e.get(
        this.cast::<RagdollController>(),
        RagdollController::bInitRagdollAnim,
    )
}

// Translated from 0087ea40 (decompiled, FalloutNV.exe 1.4.0.525)
/// On a `bhkRagdollController`: sets `iGroup` (`+0x5c`).
pub fn fn_0087ea40(e: &mut Engine, this: Ptr, group: u32) {
    e.set(
        this.cast::<RagdollController>(),
        RagdollController::iGroup,
        group,
    );
}

// Translated from 0087ea60 (decompiled, FalloutNV.exe 1.4.0.525)
/// On a `bhkRagdollController`: sets the byte at `+0x224`.
pub fn fn_0087ea60(e: &mut Engine, this: Ptr, value: u8) {
    e.set(
        this.cast::<RagdollController>(),
        RagdollController::bFlag224,
        value,
    );
}

// Translated from 0087ea80 (decompiled, FalloutNV.exe 1.4.0.525)
/// On a `bhkRagdollController`: stores at `+0x90` the pointer
/// `bhkCharacterProxy::operator P` (`004ae750`) makes of `proxy`.
pub fn fn_0087ea80(e: &mut Engine, this: Ptr, proxy: Ptr) {
    let pointer: Ptr = e.call(0x004a_e750, &args![proxy]).ptr();
    e.set(
        this.cast::<RagdollController>(),
        RagdollController::pCharacterProxy,
        pointer,
    );
}

// Translated from 0087eaa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `0x40` bytes into the object.
pub fn fn_0087eaa0(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x40)
}

// Translated from 0087eac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Puts the actor on `package`, or takes it off (`flag_a`): `flag_a` true is the
/// "leaving" path (the process's virtual `+0x210`/`+0x284`, `+0x48` of the
/// actor), false the "starting" one (the process's `+0x1e0` timer made
/// negative, `+0x230`, the sit/sleep checks). `flag_b` marks the package as
/// created (`TESPackage::SetIsCreated`). Does nothing without a process.
pub fn fn_0087eac0(e: &mut Engine, this: Ptr<Actor>, package: Ptr, flag_a: bool, flag_b: bool) {
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() {
        return;
    }
    if e.call(0x0096_11e0, &args![package]).i32() == -1 {
        let target = e.call(0x0056_9b80, &args![this]).u32();
        e.call(0x0067_77b0, &args![package, target]);
    }
    // Leaving this block (the label `ec07` of the exe) skips the rest of the
    // sit/sleep handling.
    'handled: {
        if package_kind(e, package) == 0x1c
            || package_kind(e, package) == 0x1b
            || package_kind(e, package) == 0x1a
        {
            break 'handled;
        }
        if e.vcall(this.addr(), 0x214, &args![]).u32() == 0 {
            break 'handled;
        }
        if (package_kind(e, package) == 0x15 || package_kind(e, package) == 0x1c)
            && e.vcall(this.addr(), 0x214, &args![]).u32() == 4
        {
            break 'handled;
        }
        let leave_furniture = e.vcall(this.addr(), 0x214, &args![]).u32() == 4
            || e.vcall(this.addr(), 0x214, &args![]).u32() == 9
            || {
                let acquire = e.call(0x008d_8520, &args![this]).u32();
                e.vcall(acquire, 0x4d4, &args![]).u32() != 0 && {
                    let acquire_again = e.call(0x008d_8520, &args![this]).u32();
                    let object = e.vcall(acquire_again, 0x4d4, &args![]).u32();
                    e.mem.u8(object + 0xe) > 0x14
                }
            };
        if leave_furniture {
            e.vcall(this.addr(), 0x418, &args![]);
            if flag_a {
                return;
            }
        } else {
            e.call(0x0088_d640, &args![this]);
        }
    }
    if !package.is_null() && flag_b {
        e.call(0x0067_4d70, &args![package, 1u32]);
    }
    let process_address = process.addr();
    if !flag_a {
        let timer = e.vcall(process_address, 0x1e0, &args![]).f32();
        let zero: f64 = e.global(0x0101_2060);
        if timer as f64 > zero {
            let timer = e.vcall(process_address, 0x1e0, &args![]).f32();
            let factor: f64 = e.global(0x0101_a6b0);
            let negated = (timer as f64 * factor) as f32;
            e.vcall(process_address, 0x1e4, &args![negated]);
            e.vcall(process_address, 0x214, &args![]);
            e.vcall(process_address, 0x294, &args![this]);
            let player = e.global::<u32>(PLAYER_POINTER);
            e.vcall(process_address, 0x628, &args![player]);
        }
        if package.is_null() || package_kind(e, package) != 0x12 {
            e.call(CLEAR_IN_COMBAT, &args![this, 1u32]);
        }
        e.vcall(process_address, 0x230, &args![package, this]);
        e.call(0x0045_34f0, &args![this, package]);
        if !e.call(0x0049_3bb0, &args![this]).bool() && e.call(0x008b_70d0, &args![this]).u32() != 0
        {
            let state = e.vcall(this.addr(), 0x214, &args![]).u32();
            let sitting_or_sleeping = state == 0
                || e.vcall(this.addr(), 0x214, &args![]).u32() == 9
                || e.vcall(this.addr(), 0x214, &args![]).u32() == 4;
            if sitting_or_sleeping && package_kind(e, package) != 0x1c {
                e.vcall(process_address, 0x614, &args![0x800u32]);
            }
        }
    } else {
        e.vcall(process_address, 0x210, &args![package, this]);
        e.vcall(process_address, 0x284, &args![0u32]);
        let mut notify = true;
        if package_kind(e, package) == 0x1c && fn_0087eed0(e, package) != 0 {
            notify = false;
        }
        if notify {
            e.vcall(process_address, 0x294, &args![this]);
        }
        e.vcall(this.addr(), 0x48, &args![0u32]);
    }
    e.vcall(process_address, 0x12c, &args![0u32]);
    e.vcall(process_address, 0x244, &args![]);
    if !e.call(0x0049_3bb0, &args![this]).bool() && fn_0087eeb0(e, this) == 0 {
        e.call(0x008b_3d30, &args![this]);
    }
    e.call(0x008a_8f60, &args![this]);
}

/// The package's type word (`0041ca90`).
fn package_kind(e: &mut Engine, package: Ptr) -> u32 {
    e.call(0x0041_ca90, &args![package]).u32()
}

// Translated from 0087eeb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bTurretBehavior` (Xbox PDB `+0x1c0`).
pub fn fn_0087eeb0(e: &mut Engine, this: Ptr<Actor>) -> u8 {
    e.get(this, Actor::bTurretBehavior)
}

// Translated from 0087eed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0xbe` of a `TESPackage`.
pub fn fn_0087eed0(e: &mut Engine, package: Ptr) -> u8 {
    e.mem.u8(package.addr() + 0xbe)
}

// Translated from 0087eef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Gives the actor the items its base form's container lists: builds a
/// snapshot of the base form's contents (`00487f70`, over the component the
/// base form's `+0x30` word leads to), and for every entry adds the item
/// (virtual `+0x190`) with the right count and, when the item has a script or
/// an entry health value, an extra-data list carrying them. Finally lets the
/// inventory changes of the reference's extra data update (`004d1960`).
pub fn fn_0087eef0(e: &mut Engine, this: Ptr<Actor>) {
    let base = base_form(e, this);
    let source = e.call(0x0044_1110, &args![base.byte_add(0x30)]).u32();
    if source != 0 {
        e.with_stack(0x40, |e, container| {
            e.call(0x0048_1610, &args![container]);
            let flags = e.call(0x0087_f9f0, &args![this]).u16();
            e.call(
                0x0048_7f70,
                &args![
                    Ptr::<()>::new(source).byte_add(0x30),
                    flags,
                    1u32,
                    container,
                    0u32
                ],
            );
            let mut node = e.call(0x0071_7e50, &args![container]).u32();
            while node != 0 {
                let data = e.call(0x0068_15c0, &args![node]).u32();
                let entry = e.mem.u32(data);
                if entry == 0 {
                    break;
                }
                add_default_inventory_entry(e, this, node, entry);
                node = e.call(0x0072_6070, &args![node]).u32();
            }
            e.call(0x0048_1680, &args![container]);
        });
    }
    let list = extra_data_list(e, this);
    let changes = e.call(0x0041_8520, &args![list]).u32();
    if changes != 0 {
        e.call(0x004d_1960, &args![changes]);
    }
}

/// One entry of the default inventory: `entry` is the container entry (count
/// at `+0`, item at `+4`, health description at `+8`); `node` is the list node
/// it came from.
fn add_default_inventory_entry(e: &mut Engine, this: Ptr<Actor>, node: u32, entry: u32) {
    let mut form = 0;
    let item = e.mem.u32(entry + 4);
    if item != 0 && e.vcall(item, 0xe4, &args![]).bool() {
        let data = e.call(0x0068_15c0, &args![node]).u32();
        let entry_again = e.mem.u32(data);
        form = e.mem.u32(entry_again + 4);
    }
    if form == 0 {
        return;
    }
    let script = e.call(0x0048_26d0, &args![item]).u32();
    let count = e.mem.u32(entry);
    let health_entry = e.mem.u32(entry + 8);
    if script != 0 {
        for _ in 0..count {
            let extra = new_extra_data_list(e);
            e.call(0x0041_9ad0, &args![extra, 1u32]);
            if extra != 0 && e.call(0x0041_8800, &args![extra]).u32() == 0 {
                e.call(0x0041_9ed0, &args![extra, script]);
                let attached = e.call(0x0041_8800, &args![extra]).u32();
                let value = e.call(0x005a_bf60, &args![attached]).u32();
                e.call(0x0041_9f80, &args![extra, value]);
            }
            if health_entry != 0 {
                let health = e.call(0x0048_72e0, &args![item]).u32();
                e.call(0x0040_ea20, &args![health_entry, extra, health]);
            }
            e.vcall(this.addr(), 0x190, &args![form, extra, 1u32]);
        }
    } else if health_entry != 0 {
        let extra = new_extra_data_list(e);
        e.call(0x0041_9ad0, &args![extra, count as u16]);
        let health = e.call(0x0048_72e0, &args![item]).u32();
        e.call(0x0040_ea20, &args![health_entry, extra, health]);
        e.vcall(this.addr(), 0x190, &args![form, extra, count]);
    } else {
        e.vcall(this.addr(), 0x190, &args![form, 0u32, count]);
    }
}

/// `new ExtraDataList` (`0x20` bytes), null when the allocation fails.
fn new_extra_data_list(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![0x20u32]).u32();
    if block != 0 {
        e.call(EXTRA_DATA_LIST_CONSTRUCTOR, &args![block]).u32()
    } else {
        0
    }
}

// Translated from 0087f200 (decompiled, FalloutNV.exe 1.4.0.525)
/// False when the base form's component at `+0x30` says (virtual `+0x44`) yes
/// or `008ace90` says yes; true otherwise.
pub fn fn_0087f200(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    if e.vcall(base.addr() + 0x30, 0x44, &args![]).bool() {
        return false;
    }
    !e.call(0x008a_ce90, &args![this]).bool()
}

// Translated from 0087f260 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the actor may play a random idle. False when (the base is a
/// humanoid creature, or the actor's virtual `+0x218` is true) and (the
/// setting at `011df7f8` is on or the actor's virtual `+0x1a0(0)` is true);
/// false when `005a29b0` finds a last idle; false when the base form's
/// component at `+0x30` has virtual `+0x50` and virtual `+0x48` both true;
/// true otherwise.
pub fn fn_0087f260(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    let as_creature = e
        .call(
            RT_DYNAMIC_CAST,
            &args![base, 0u32, 0x0118_46e8u32, 0x0118_3a00u32, 0u32],
        )
        .u32();
    let humanoid_creature = as_creature != 0 && e.call(0x005f_bf20, &args![as_creature]).bool();
    if humanoid_creature || e.vcall(this.addr(), 0x218, &args![]).bool() {
        let setting = e.call(SETTING_VALUE_POINTER, &args![0x011d_f7f8u32]).u32();
        if e.mem.u8(setting) != 0 || e.vcall(this.addr(), 0x1a0, &args![0u32]).bool() {
            return false;
        }
    }
    if e.call(0x005a_29b0, &args![this]).u32() != 0 {
        return false;
    }
    let base = base_form(e, this);
    if e.vcall(base.addr() + 0x30, 0x50, &args![]).bool() {
        let base = base_form(e, this);
        if e.vcall(base.addr() + 0x30, 0x48, &args![]).bool() {
            return false;
        }
    }
    true
}

// Translated from 0087f350 (decompiled, FalloutNV.exe 1.4.0.525)
/// `005f0b50` (a `TESActorBase` predicate) on the actor's base form.
pub fn fn_0087f350(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    e.call(0x005f_0b50, &args![base]).bool()
}

// Translated from 0087f370 (decompiled, FalloutNV.exe 1.4.0.525)
/// `005f0bf0` (a `TESActorBase` predicate) on the actor's base form.
pub fn fn_0087f370(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    e.call(0x005f_0bf0, &args![base]).bool()
}

// Translated from 0087f390 (decompiled, FalloutNV.exe 1.4.0.525)
/// `005f0c40` (a `TESActorBase` predicate) on the actor's base form.
pub fn fn_0087f390(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    e.call(0x005f_0c40, &args![base]).bool()
}

// Translated from 0087f3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `005f0ba0` (a `TESActorBase` predicate) on the actor's base form.
pub fn fn_0087f3b0(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    e.call(0x005f_0ba0, &args![base]).bool()
}

// Translated from 0087f3d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetEssential` (Xbox PDB): whether the actor is essential. Uses the
/// leveled-creature original base (`004216f0` on the extra data list) when it
/// has one, else the base form; false without a base. True while the player
/// exists, `00566950` is true for the actor and `004d1360` on the player is
/// false; otherwise `0087f480` on the base form's component at `+0x30` or
/// `008c1b30` on the actor.
pub fn actor_get_essential(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let mut base = base_form(e, this);
    let list = extra_data_list(e, this);
    if e.call(0x0042_16f0, &args![list]).u32() != 0 {
        let list = extra_data_list(e, this);
        base = e.call(0x0042_16f0, &args![list]).ptr();
    }
    if base.is_null() {
        return false;
    }
    let player = e.global::<u32>(PLAYER_POINTER);
    if player != 0
        && e.call(0x0056_6950, &args![this]).bool()
        && !e.call(0x004d_1360, &args![player]).bool()
    {
        return true;
    }
    fn_0087f480(e, base.byte_add(0x30)) || e.call(0x008c_1b30, &args![this]).bool()
}

// Translated from 0087f480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `2` of the flag word at `+4` of the object is set
/// (`00461580(this, 2)`).
pub fn fn_0087f480(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0046_1580, &args![this, 2u32]).bool()
}

// Translated from 0087f4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `0056af00` (bit `8` of the flag word) on the base form's component at `+0x30`.
pub fn fn_0087f4a0(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    e.call(0x0056_af00, &args![base.byte_add(0x30)]).bool()
}

// Translated from 0087f4c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESActorBase::GetSex` (`005f0cc0`) on the actor's base form: 0 or 1, or -1
/// when the base is not an NPC.
pub fn fn_0087f4c0(e: &mut Engine, this: Ptr<Actor>) -> i32 {
    let base = base_form(e, this);
    e.call(0x005f_0cc0, &args![base]).i32()
}

// Translated from 0087f4e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The base form's virtual `+0x144`, with its result.
pub fn fn_0087f4e0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let base = base_form(e, this);
    e.vcall(base.addr(), 0x144, &args![]).u32()
}

// Translated from 0087f510 (decompiled, FalloutNV.exe 1.4.0.525)
/// 1 when the actor's virtual `+0x218` is false; otherwise the byte at `+0xc`
/// of the extra data of type `0x4e` when the reference has it, else
/// `005d2780` on the base form's component at `+0x30`.
pub fn fn_0087f510(e: &mut Engine, this: Ptr<Actor>) -> u8 {
    if !e.vcall(this.addr(), 0x218, &args![]).bool() {
        return 1;
    }
    let list = extra_data_list(e, this);
    let extra = e.call(0x0041_0220, &args![list, 0x4eu32]).u32();
    if extra != 0 {
        e.mem.u8(extra + 0xc)
    } else {
        let base = base_form(e, this);
        e.call(0x005d_2780, &args![base.byte_add(0x30)]).u8()
    }
}

// Translated from 0087f570 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SwimsOnly` (Xbox PDB): true when `0087f370` is false, `0087f350` is
/// true and `0087f390` is false.
pub fn actor_swims_only(e: &mut Engine, this: Ptr<Actor>) -> bool {
    !fn_0087f370(e, this) && fn_0087f350(e, this) && !fn_0087f390(e, this)
}

// Translated from 0087f5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the actor's virtual `+0x218` is false and it only swims;
/// otherwise whether the actor-value owner at `+0xa4` (virtual `+8`) gives a
/// non-zero value for `0x35`.
pub fn fn_0087f5c0(e: &mut Engine, this: Ptr<Actor>) -> bool {
    if !e.vcall(this.addr(), 0x218, &args![]).bool() && actor_swims_only(e, this) {
        return true;
    }
    e.vcall(this.addr() + 0xa4, 8, &args![0x35u32]).u32() != 0
}

// Translated from 0087f620 (decompiled, FalloutNV.exe 1.4.0.525)
/// False when the actor's virtual `+0x218` is false and it only swims, true
/// otherwise.
pub fn fn_0087f620(e: &mut Engine, this: Ptr<Actor>) -> bool {
    e.vcall(this.addr(), 0x218, &args![]).bool() || !actor_swims_only(e, this)
}

// ---------------------------------------------------------------------------
// Block 2 (batch b0148, continued): `0087f660` up to `00881360`

/// `_ftol2_sse` (`00ec62c0`): truncates the value the game holds in `ST0`; the
/// uniform form takes it as a leading `f64` argument.
const FLOAT_TO_INT: u32 = 0x00ec_62c0;
/// The form-type byte of a form (`00401170`, the byte at `+4`).
const FORM_TYPE: u32 = 0x0040_1170;
/// Bit 0 of the byte at `+0x24` of an object (`00425fd0`).
const FLAG_BIT_0_AT_0X24: u32 = 0x0042_5fd0;
/// A list node's own address (`006815c0`): the item is the word at the result.
const LIST_NODE_ITEM_SLOT: u32 = 0x0068_15c0;
/// A list node's next pointer (`00726070`, the word at `+4`).
const LIST_NODE_NEXT: u32 = 0x0072_6070;
/// Removes the first node holding the item whose address is passed (a
/// pointer to a local holding the item): `(list, &item)`.
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
/// Appends the item whose address is passed: `(list, &item)`.
const LIST_APPEND_ITEM: u32 = 0x005a_e3d0;
/// Empties a list (`00470470`).
const LIST_CLEAR: u32 = 0x0047_0470;
/// Releases a block allocated with `operator new` (`00401030`, cdecl).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The actor's process (`008d8520`: the word at `+0x68`; the engine map names
/// it `MiddleHighProcess::GetSavedAcquireObject` because the code is folded).
const ACTOR_PROCESS: u32 = 0x008d_8520;
/// `bool` test of actor value `(index)`, true when the value is a plain
/// stat that has a base entry (`0066ee10`, cdecl).
const ACTOR_VALUE_HAS_BASE: u32 = 0x0066_ee10;
/// `(index, mask)`: whether the actor-value flag word has a bit of `mask`
/// (`00406d70`, cdecl).
const ACTOR_VALUE_HAS_FLAG: u32 = 0x0040_6d70;
/// Tells the actor value `(owner, index, base, value, other owner)` changed
/// (`0066ee50`, cdecl, five words).
const ACTOR_VALUE_CHANGED: u32 = 0x0066_ee50;
/// `(index)` whether `index` is within `0 ..= 0x2d` (`0047f060`, cdecl).
const ACTOR_VALUE_IS_STAT: u32 = 0x0047_f060;
/// Offset of the actor-value owner sub-object inside an `Actor`.
const ACTOR_VALUE_OWNER: u32 = 0xa4;
/// Offset of the editor location (three `float`s) inside an `Actor`.
const EDITOR_LOCATION: u32 = 0x160;

/// `_ftol2_sse` on a value in `ST0`.
fn float_to_int(e: &mut Engine, value: f64) -> i32 {
    e.call(FLOAT_TO_INT, &args![value]).i32()
}

/// Copies three words (a `NiPoint3`).
fn copy_point(e: &mut Engine, from: u32, to: u32) {
    for i in 0..3 {
        let word = e.mem.u32(from + 4 * i);
        e.mem.set_u32(to + 4 * i, word);
    }
}

/// The actor's process (`008d8520`).
fn actor_process(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    e.call(ACTOR_PROCESS, &args![this]).ptr()
}

/// `owner + 0xa4` of a pointer that may be null (null stays null).
fn owner_of(actor: u32) -> u32 {
    if actor != 0 {
        actor + ACTOR_VALUE_OWNER
    } else {
        0
    }
}

/// The base value of actor value `index` (the owner's virtual `+0xc`), or
/// `0.0` when `0066ee10` says the value has none.
fn base_actor_value(e: &mut Engine, this: Ptr<Actor>, index: u32) -> f32 {
    if e.call(ACTOR_VALUE_HAS_BASE, &args![index]).bool() {
        e.vcall(this.addr() + ACTOR_VALUE_OWNER, 0xc, &args![index])
            .f32()
    } else {
        0.0
    }
}

/// The report every actor-value change ends with: `0066ee50(owner of this,
/// index, base, value, owner of source)`.
fn report_actor_value_change(
    e: &mut Engine,
    this: Ptr<Actor>,
    index: u32,
    base: f32,
    value: f32,
    source: u32,
) {
    e.call(
        ACTOR_VALUE_CHANGED,
        &args![owner_of(this.addr()), index, base, value, owner_of(source)],
    );
}

/// The item of the first node of the disposition-modifier list whose actor
/// (`DispositionModifier::pactormodified`) is `target`, or 0.
fn find_disposition_modifier(e: &mut Engine, this: Ptr<Actor>, target: u32) -> u32 {
    let mut node = this.at(Actor::DispModifierList).cast::<()>();
    while !node.is_null() {
        let slot = e.call(LIST_NODE_ITEM_SLOT, &args![node]).u32();
        let item = e.mem.u32(slot);
        if item == 0 {
            break;
        }
        if e.mem.u32(item + 4) == target {
            return item;
        }
        node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
    }
    0
}

// Translated from 0087f660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::CheckBreathTimer` (Xbox PDB): false when the actor has a process
/// whose virtual `+0x300` (the breath timer) is smaller than the swim breath
/// time (`00648a10` of the integer part of `008be7a0`); true otherwise.
pub fn actor_check_breath_timer(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let mut result = true;
    let level = e.call(0x008b_e7a0, &args![this]).f64();
    let level = float_to_int(e, level);
    let limit = e.call(0x0064_8a10, &args![level]).f32();
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        let timer = e.vcall(process.addr(), 0x300, &args![]).f32();
        if limit > timer {
            result = false;
        }
    }
    result
}

// Translated from 0087f6c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The result of `00408da0` on the data at `+0x18` of `004ac110`'s record of
/// the actor's form (`MapMarkerData::GetLocationName` in the decompiler's
/// naming), 0 when the actor has no form (virtual `+0x218` false) or the
/// record's check `0048cee0` fails.
pub fn fn_0087f6c0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mut form = 0;
    if e.vcall(this.addr(), 0x218, &args![]).bool() {
        form = e.call(GET_FORM, &args![this]).u32();
    }
    if form != 0 {
        let record = e.call(0x004a_c110, &args![form]).u32();
        let data = if record != 0 { record + 0x18 } else { 0 };
        if data != 0 && e.call(0x0048_cee0, &args![data]).u32() != 0 {
            return e.call(0x0040_8da0, &args![data]).u32();
        }
    }
    0
}

// Translated from 0087f750 (decompiled, FalloutNV.exe 1.4.0.525)
/// The editor location form when it is set and its form type is `0x41`, else null.
pub fn fn_0087f750(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    let form = e.get(this, Actor::pEditorLocForm);
    if !form.is_null() && e.call(FORM_TYPE, &args![form]).u32() == 0x41 {
        form
    } else {
        Ptr::NULL
    }
}

// Translated from 0087f7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The editor location form when it is set, its form type is `0x39` and
/// `00425fd0` is true for it, else null.
pub fn fn_0087f7a0(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    let mut form = Ptr::NULL;
    let location = e.get(this, Actor::pEditorLocForm);
    if !location.is_null() && e.call(FORM_TYPE, &args![location]).u32() == 0x39 {
        form = location;
    }
    if !form.is_null() && !e.call(FLAG_BIT_0_AT_0X24, &args![form]).bool() {
        form = Ptr::NULL;
    }
    form
}

// Translated from 0087f800 (decompiled, FalloutNV.exe 1.4.0.525)
/// Records the actor's current place as its editor location: the position
/// (`00436aa0`, three floats) into `+0x160`, the z rotation (virtual `+0x2bc`
/// with argument 0) into `fEditorLocZRot`, and the form: `008d6f30` when it
/// passes `00425fd0`, else `00575d70` (`TESObjectREFR::GetWorldSpace`).
pub fn fn_0087f800(e: &mut Engine, this: Ptr<Actor>) {
    let position = e.call(0x0043_6aa0, &args![this]).u32();
    copy_point(e, position, this.addr() + EDITOR_LOCATION);
    let z_rotation = e.vcall(this.addr(), 0x2bc, &args![0u32]).f32();
    e.set(this, Actor::fEditorLocZRot, z_rotation);
    if e.call(0x008d_6f30, &args![this]).u32() != 0 {
        let candidate = e.call(0x008d_6f30, &args![this]).ptr::<()>();
        if e.call(FLAG_BIT_0_AT_0X24, &args![candidate]).bool() {
            let form = e.call(0x008d_6f30, &args![this]).ptr();
            e.set(this, Actor::pEditorLocForm, form);
            return;
        }
    }
    let form = e.call(0x0057_5d70, &args![this]).ptr();
    e.set(this, Actor::pEditorLocForm, form);
}

// Translated from 0087f890 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the editor location from `position` and `rotation`; the form is
/// `candidate` when it is set and passes `00425fd0`, else `fallback`.
pub fn fn_0087f890(
    e: &mut Engine,
    this: Ptr<Actor>,
    fallback: Ptr,
    candidate: Ptr,
    position: Ptr,
    rotation: f32,
) {
    copy_point(e, position.addr(), this.addr() + EDITOR_LOCATION);
    e.set(this, Actor::fEditorLocZRot, rotation);
    if !candidate.is_null() && e.call(FLAG_BIT_0_AT_0X24, &args![candidate]).bool() {
        e.set(this, Actor::pEditorLocForm, candidate);
    } else {
        e.set(this, Actor::pEditorLocForm, fallback);
    }
}

// Translated from 0087f900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the actor has an editor location form.
pub fn fn_0087f900(e: &mut Engine, this: Ptr<Actor>) -> bool {
    !e.get(this, Actor::pEditorLocForm).is_null()
}

// Translated from 0087f920 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the editor location back: false when the actor has none; otherwise
/// the position goes to `position`, `(0, 0, z rotation)` to `rotation` and the
/// form to the word at `form`, and the result is true. The fourth word the
/// function pops is never read.
pub fn fn_0087f920(
    e: &mut Engine,
    this: Ptr<Actor>,
    position: Ptr,
    rotation: Ptr,
    form: Ptr,
    _unused_3: u32,
) -> bool {
    let location = e.get(this, Actor::pEditorLocForm);
    if location.is_null() {
        return false;
    }
    copy_point(e, this.addr() + EDITOR_LOCATION, position.addr());
    e.mem.set_f32(rotation.addr(), 0.0);
    e.mem.set_f32(rotation.addr() + 4, 0.0);
    let z_rotation = e.get(this, Actor::fEditorLocZRot);
    e.mem.set_f32(rotation.addr() + 8, z_rotation);
    e.mem.set_u32(form.addr(), location.addr());
    true
}

// Translated from 0087f990 (decompiled, FalloutNV.exe 1.4.0.525)
/// `005bb4d0` on the base form's component at `+0x90`.
pub fn fn_0087f990(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let base = base_form(e, this);
    e.call(0x005b_b4d0, &args![base.byte_add(0x90)]).bool()
}

// Translated from 0087f9c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The actor-value owner's virtual `+0` (an integer) for actor value `0x15`,
/// as a `float`.
pub fn fn_0087f9c0(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    let value = e
        .vcall(this.addr() + ACTOR_VALUE_OWNER, 0, &args![0x15u32])
        .i32();
    value as f32
}

// Translated from 0087f9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `0047ded0` on the base form's component at `+0x30`.
pub fn fn_0087f9f0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let base = base_form(e, this);
    e.call(0x0047_ded0, &args![base.byte_add(0x30)]).u32()
}

// Translated from 0087fa10 (decompiled, FalloutNV.exe 1.4.0.525)
/// With a process: its virtual `+0x118` with `flag`, then its virtual `+0x4a0`.
pub fn fn_0087fa10(e: &mut Engine, this: Ptr<Actor>, flag: bool) {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        e.vcall(process.addr(), 0x118, &args![flag]);
        e.vcall(process.addr(), 0x4a0, &args![]);
    }
}

// Translated from 0087fa60 (decompiled, FalloutNV.exe 1.4.0.525)
/// With a process: its virtual `+0x120` with `flag`.
pub fn fn_0087fa60(e: &mut Engine, this: Ptr<Actor>, flag: bool) {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        e.vcall(process.addr(), 0x120, &args![flag]);
    }
}

// Translated from 0087faa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::EndMovement` (Xbox PDB): with a process, its virtual `+0x294` with
/// the actor.
pub fn actor_end_movement(e: &mut Engine, this: Ptr<Actor>) {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        e.vcall(process.addr(), 0x294, &args![this]);
    }
}

// Translated from 0087fad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes from the disposition-modifier list the entry for `target`
/// (`DispositionModifier::pactormodified`), if there is one.
pub fn fn_0087fad0(e: &mut Engine, this: Ptr<Actor>, target: u32) {
    let item = find_disposition_modifier(e, this, target);
    if item != 0 {
        let list = this.at(Actor::DispModifierList);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item);
            e.call(LIST_REMOVE_ITEM, &args![list, slot]);
        });
    }
}

// Translated from 0087fb40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `amount` to the disposition modifier the actor keeps toward `target`,
/// only when `target` is the player. Sets the actor's flag bit `0x80000`
/// (virtual `+0x48`). When the actor already has an entry for the player,
/// the amount is first limited so that the actor's disposition (virtual
/// `+0x344` with the player and 0) plus the amount stays within `0 ..= 100`
/// (a sum below 0 turns the amount into `-disposition`; a sum above 100
/// keeps only the part up to 100, or 0), then it is added to the entry.
/// Otherwise a new entry `{ amount, target }` is allocated and appended
/// (`00564db0(target, 1)` is called on the player first).
pub fn fn_0087fb40(e: &mut Engine, this: Ptr<Actor>, target: Ptr, amount: f32) {
    let player = e.global::<u32>(PLAYER_POINTER);
    if target.addr() != player {
        return;
    }
    let mut amount = amount;
    e.vcall(this.addr(), 0x48, &args![0x80000u32]);
    let entry = find_disposition_modifier(e, this, target.addr());
    if entry != 0 {
        let entry: Ptr<DispositionModifier> = Ptr::new(entry);
        let disposition = e.vcall(this.addr(), 0x344, &args![target, 0u32]).i32();
        let sum = disposition as f64 + amount as f64;
        let zero: f64 = e.global(0x0101_2060);
        let hundred: f64 = e.global(0x0101_7a40);
        if sum < zero {
            amount = (amount as f64 - sum) as f32;
        } else if sum > hundred {
            let total = float_to_int(e, amount as f64).wrapping_add(disposition);
            let excess = total.wrapping_sub(100);
            if amount as f64 > excess as f64 {
                amount = (amount as f64 - excess as f64) as f32;
            } else {
                amount = 0.0;
            }
        }
        let delta = float_to_int(e, amount as f64);
        let current = e.get(entry, DispositionModifier::modifieramount);
        e.set(
            entry,
            DispositionModifier::modifieramount,
            current.wrapping_add(delta),
        );
    } else {
        let item = e.call(OPERATOR_NEW, &args![8u32]).u32();
        let delta = float_to_int(e, amount as f64);
        let item: Ptr<DispositionModifier> = Ptr::new(item);
        e.set(item, DispositionModifier::modifieramount, delta);
        e.set(item, DispositionModifier::pactormodified, target);
        e.call(0x0056_4db0, &args![target, 1u32]);
        let list = this.at(Actor::DispModifierList);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item.addr());
            e.call(LIST_APPEND_ITEM, &args![list, slot]);
        });
    }
}

// Translated from 0087fcb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The disposition modifier the actor keeps toward `target` as a `float`, 0
/// when it has none.
pub fn fn_0087fcb0(e: &mut Engine, this: Ptr<Actor>, target: u32) -> f32 {
    let entry = find_disposition_modifier(e, this, target);
    if entry != 0 {
        e.mem.i32(entry) as f32
    } else {
        0.0
    }
}

// Translated from 0087fd20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ClearDispositionModifiers` (Xbox PDB): frees every entry of the
/// disposition-modifier list, then empties the list (`00470470`).
pub fn actor_clear_disposition_modifiers(e: &mut Engine, this: Ptr<Actor>) {
    let list = this.at(Actor::DispModifierList);
    let mut node = list.cast::<()>();
    while !node.is_null() {
        let slot = e.call(LIST_NODE_ITEM_SLOT, &args![node]).u32();
        let item = e.mem.u32(slot);
        if item == 0 {
            break;
        }
        e.call(OPERATOR_DELETE, &args![item]);
        node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
    }
    e.call(LIST_CLEAR, &args![list]);
}

// Translated from 0087fd90 (decompiled, FalloutNV.exe 1.4.0.525)
/// A score of the actor towards `other`, clamped to `0 ..= 100` (it looks
/// like the actor's disposition towards `other`; the virtual `+0x344` the
/// modifier code above calls with `(other, 0)` has this shape). Returns 0
/// without `other` and 100 at once when `other` is the actor's own target
/// form. `cached` (may be null) is a block of six words that stands in for the
/// values the function otherwise reads from the actor: `+0`, `+4`, `+8`
/// (the target form), `+0xc`, `+0x10` and `+0x14`.
///
/// Two paths follow: the first (when the actor is not a creature-like
/// actor, virtual `+0x218` false, and it has an owner reference or a saved
/// acquire object, or `00608d80` is true) and the second both read the
/// values of `other`'s actor-value owner and combine them with
/// `00642a60`; the result goes through `BGSEntryPoint::HandleEntryPoint`
/// (`005e58f0`, entry point `0xe`) and is clamped. The extra argument words
/// the code pushes for the owner's virtual `+8` call are never read by it
/// (the callee pops one word), so only `0x17` is passed.
pub fn fn_0087fd90(e: &mut Engine, this: Ptr<Actor>, other: Ptr<Actor>, cached: Ptr) -> i32 {
    let mut result: i32 = 0;
    if other.is_null() {
        return result;
    }
    let mut saved_acquire = 0u32;
    let cached_37c;
    let virtual_37c = e.vcall(this.addr(), 0x37c, &args![]).u32();
    let mut target_form = 0u32;
    let base_of_this;
    let flags_word;
    let skill_word;
    let owner_reference;
    let mut other_base = base_form(e, other).addr();
    if other_base != 0 {
        let component = e.call(0x005d_8a70, &args![other_base + 0x30]).u32();
        if e.call(0x0082_56d0, &args![component]).bool() {
            other_base = base_form(e, other).addr();
        }
    }
    if !cached.is_null() {
        cached_37c = e.mem.u32(cached.addr());
        owner_reference = e.mem.u32(cached.addr() + 0xc);
        flags_word = e.mem.u32(cached.addr() + 0x10);
        skill_word = e.mem.u32(cached.addr() + 0x14);
        target_form = e.mem.u32(cached.addr() + 8);
        base_of_this = e.mem.u32(cached.addr() + 4);
    } else {
        cached_37c = e.vcall(this.addr(), 0x37c, &args![]).u32();
        owner_reference = e.call(0x0056_7790, &args![this]).u32();
        base_of_this = base_form(e, this).addr();
        flags_word = e.call(0x0047_d3d0, &args![base_of_this + 0x30]).u16() as u32;
        skill_word = e
            .vcall(this.addr() + ACTOR_VALUE_OWNER, 8, &args![3u32])
            .u32();
        if e.vcall(this.addr(), 0x218, &args![]).bool() {
            target_form = e.call(GET_FORM, &args![this]).u32();
        }
    }
    let process = actor_process(e, this);
    if !process.is_null() {
        saved_acquire = e.vcall(process.addr(), 0x52c, &args![]).u32();
    }
    if e.call(GET_FORM, &args![other]).u32() == target_form {
        return 100;
    }
    let creature_like = e.vcall(this.addr(), 0x218, &args![]).bool();
    let first_path = (!creature_like && (owner_reference != 0 || saved_acquire != 0))
        || e.call(0x0060_8d80, &args![this]).bool();
    if first_path {
        if target_form == 0 {
            if owner_reference != 0 && e.call(FORM_TYPE, &args![owner_reference]).u32() == 0x2a {
                target_form = owner_reference;
            }
            if target_form == 0 && saved_acquire != 0 {
                let form = e.call(GET_FORM, &args![saved_acquire]).u32();
                if e.call(FORM_TYPE, &args![form]).u32() == 0x2a {
                    target_form = e.call(GET_FORM, &args![saved_acquire]).u32();
                }
            }
        }
        if target_form != 0 && e.call(GET_FORM, &args![other]).u32() == target_form {
            return 100;
        }
        if target_form != 0 && other_base != 0 {
            // Locals the compiler keeps (an unused copy of `other_base` among them).
            let mut cached_value = 0u32;
            if e.vcall(other.addr(), 0x218, &args![]).bool() {
                cached_value = e
                    .call(0x0048_bf50, &args![cached_37c + 0x40, virtual_37c])
                    .u32();
            }
            let (outcome, out_first) = e.with_stack(12, |e, outs| {
                e.mem.set_u32(outs.addr(), 0xffff_ffff);
                e.mem.set_u32(outs.addr() + 4, 0);
                e.mem.set_u32(outs.addr() + 8, 0);
                let outcome = e
                    .call(
                        0x008b_7fe0,
                        &args![this, other, outs, outs.byte_add(4), outs.byte_add(8)],
                    )
                    .u32();
                (outcome, e.mem.u32(outs.addr()))
            });
            let mut hostile = 0u32;
            if e.call(0x008a_16d0, &args![other]).bool() {
                if !e.call(0x0049_3bb0, &args![other]).bool() {
                    hostile = 1;
                } else {
                    let combat_target = e.vcall(this.addr(), 0x428, &args![]).u32();
                    if combat_target != 0 && e.call(0x0097_fa10, &args![combat_target, this]).bool()
                    {
                        hostile = 1;
                    }
                }
            }
            let player = e.global::<u32>(PLAYER_POINTER);
            let actor_in_high = if target_form == e.call(GET_FORM, &args![player]).u32() {
                player
            } else {
                e.call(0x0097_0a20, &args![PROCESS_LISTS, target_form, 0u32])
                    .u32()
            };
            let mut other_value = 0i32;
            if actor_in_high != 0 {
                let value = e.vcall(actor_in_high, 0x464, &args![other]).f64();
                other_value = float_to_int(e, value);
            }
            e.vcall(this.addr(), 0x21c, &args![]);
            let _ = (cached_value, outcome, out_first, hostile, other_value);
            let owner_value = e
                .vcall(other.addr() + ACTOR_VALUE_OWNER, 8, &args![0x17u32])
                .u32();
            let clamped = e
                .call(0x0066_ef20, &args![other.addr() + ACTOR_VALUE_OWNER, 8u32])
                .u32();
            result = e
                .call(
                    0x0064_2a60,
                    &args![flags_word, clamped, skill_word, owner_value],
                )
                .i32();
        }
    } else {
        if target_form == 0
            && owner_reference != 0
            && e.call(FORM_TYPE, &args![owner_reference]).u32() == 0x2a
        {
            target_form = owner_reference;
        }
        if target_form != 0 && e.call(GET_FORM, &args![other]).u32() == target_form {
            return 100;
        }
        let other_base_again = base_form(e, other).addr();
        if other_base_again != 0 && base_of_this != 0 {
            let mut cached_value = 0u32;
            if cached_37c != 0 && virtual_37c != 0 {
                cached_value = e
                    .call(0x0048_bf50, &args![cached_37c + 0x40, virtual_37c])
                    .u32();
            }
            let (outcome, out_first) = e.with_stack(12, |e, outs| {
                e.mem.set_u32(outs.addr(), 0xffff_ffff);
                e.mem.set_u32(outs.addr() + 4, 0);
                e.mem.set_u32(outs.addr() + 8, 0);
                let outcome = e
                    .call(
                        0x008b_7fe0,
                        &args![this, other, outs, outs.byte_add(4), outs.byte_add(8)],
                    )
                    .u32();
                (outcome, e.mem.u32(outs.addr()))
            });
            let weapon_drawn = e.call(0x008a_16d0, &args![other]).u8();
            let value = e.vcall(this.addr(), 0x464, &args![other]).f64();
            let other_value = float_to_int(e, value);
            e.vcall(this.addr(), 0x21c, &args![]);
            let _ = (cached_value, outcome, out_first, weapon_drawn, other_value);
            let owner_value = e
                .vcall(other.addr() + ACTOR_VALUE_OWNER, 8, &args![0x17u32])
                .u32();
            let clamped = e
                .call(0x0066_ef20, &args![other.addr() + ACTOR_VALUE_OWNER, 8u32])
                .u32();
            result = e
                .call(
                    0x0064_2a60,
                    &args![flags_word, clamped, skill_word, owner_value],
                )
                .i32();
        }
    }
    let mut score = result as f32;
    e.with_stack(4, |e, cell| {
        e.mem.set_f32(cell.addr(), score);
        e.call(0x005e_58f0, &args![0xeu32, this, other, cell]);
        score = e.mem.f32(cell.addr());
    });
    result = float_to_int(e, score as f64);
    result.clamp(0, 100)
}

// Translated from 00880370 (decompiled, FalloutNV.exe 1.4.0.525)
/// On the actor-value owner `this` (the sub-object at `Actor + 0xa4`): the
/// virtual `+4` value of actor value `index`, passed through `00404040` and
/// truncated.
pub fn fn_00880370(e: &mut Engine, this: Ptr, index: u32) -> i32 {
    let value = e.vcall(this.addr(), 4, &args![index]).f32();
    let adjusted = e.call(0x0040_4040, &args![value]).f64();
    float_to_int(e, adjusted)
}

// Translated from 008803a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// On the actor-value owner `this` (the sub-object at `Actor + 0xa4`): the
/// value of actor value `index` once the modifiers apply. The base value is
/// the integer result of the base form component's (`+0x100`) virtual `+8`;
/// the actor's virtual `+0x48c` gives the modifier and a flag. Which of the
/// two (or their sum) is returned depends on `0047f060`, the actor-value flag
/// words (`00406d70` with `0x1000`, `0x800`, `0x80`, `0x40`), the actor's
/// virtual `+0x21c` and `+0x360`, `00566950`, and the result of `0066ed60`.
pub fn fn_008803a0(e: &mut Engine, this: Ptr, index: u32) -> f32 {
    let actor: Ptr<Actor> = Ptr::new(this.addr().wrapping_sub(ACTOR_VALUE_OWNER));
    let component = base_form(e, actor).addr() + 0x100;
    let mut base_value = e.vcall(component, 8, &args![index]).i32() as f32;
    let (modifier, flag) = e.with_stack(4, |e, flag| {
        e.mem.set_u8(flag.addr(), 0);
        let modifier = e.vcall(actor.addr(), 0x48c, &args![index, flag]).f32();
        (modifier, e.mem.u8(flag.addr()) != 0)
    });
    if flag && !e.call(ACTOR_VALUE_IS_STAT, &args![index]).bool() {
        return modifier;
    }
    if e.vcall(actor.addr(), 0x21c, &args![]).bool()
        && e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x1000u32])
            .bool()
    {
        return base_value;
    }
    let combine = e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x800u32]).bool()
        || e.vcall(actor.addr(), 0x360, &args![]).bool()
        || e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x80u32]).bool()
        || (e.call(0x0056_6950, &args![actor]).bool() && index == 0x10);
    if !combine {
        return base_value;
    }
    if flag && e.call(ACTOR_VALUE_IS_STAT, &args![index]).bool() {
        base_value = modifier;
    }
    let (accepted, result) = e.with_stack(4, |e, cell| {
        e.mem.set_f32(cell.addr(), 0.0);
        let accepted = e
            .call(0x0066_ed60, &args![actor.addr(), index, cell])
            .bool();
        (accepted, e.mem.f32(cell.addr()))
    });
    if !accepted {
        return base_value;
    }
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x40u32]).bool() || flag {
        return (result as f64 + base_value as f64) as f32;
    }
    result
}

// Translated from 00880580 (decompiled, FalloutNV.exe 1.4.0.525)
/// On the actor-value owner `this` (the sub-object at `Actor + 0xa4`): with a
/// process, the process's virtual `+0x398` called with (the actor's base
/// form, `index`, the actor); without, the owner's own virtual `+0` with
/// `index`.
pub fn fn_00880580(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let actor: Ptr<Actor> = Ptr::new(this.addr().wrapping_sub(ACTOR_VALUE_OWNER));
    if !e.get(actor, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, actor);
        let base = base_form(e, actor);
        e.vcall(process.addr(), 0x398, &args![base, index, actor])
            .u32()
    } else {
        e.vcall(this.addr(), 0, &args![index]).u32()
    }
}

// Translated from 008805f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` counterpart of `00880580`: with a process, its virtual `+0x39c`
/// result; without, the owner's virtual `+0` (an integer) as a `float`.
pub fn fn_008805f0(e: &mut Engine, this: Ptr, index: u32) -> f32 {
    let actor: Ptr<Actor> = Ptr::new(this.addr().wrapping_sub(ACTOR_VALUE_OWNER));
    if !e.get(actor, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, actor);
        let base = base_form(e, actor);
        e.vcall(process.addr(), 0x39c, &args![base, index, actor])
            .f32()
    } else {
        e.vcall(this.addr(), 0, &args![index]).i32() as f32
    }
}

// Translated from 00880660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009376e0` (the modifier list at `+0xe0`) with (`kind`, `value`), as a `float`.
pub fn fn_00880660(e: &mut Engine, this: Ptr<Actor>, kind: u8, value: u32) -> f32 {
    e.call(0x0093_76e0, &args![this.byte_add(0xe0), kind as u32, value])
        .f32()
}

// Translated from 00880690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009375e0` (the modifier list at `+0xe0`) with (`kind`, `amount`), then sets
/// the actor's flag bit `0x400000` (virtual `+0x48`).
pub fn fn_00880690(e: &mut Engine, this: Ptr<Actor>, kind: u8, amount: f32) {
    e.call(
        0x0093_75e0,
        &args![this.byte_add(0xe0), kind as u32, amount],
    );
    e.vcall(this.addr(), 0x48, &args![0x0040_0000u32]);
}

// Translated from 008806d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The actor's virtual `+0x394` with (`index`, `value` as a `float`).
pub fn fn_008806d0(e: &mut Engine, this: Ptr<Actor>, index: u32, value: i32) {
    e.vcall(this.addr(), 0x394, &args![index, value as f32]);
}

// Translated from 00880700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Changes actor value `index` by `amount` (a `float`) for the actor: the
/// base value is read first (`0` unless `0066ee10`). With the flag `0x100`
/// (`00406d70`) the amount is clamped: a negative amount becomes 0 and,
/// otherwise, an amount not below the maximum of `0066e920`'s record
/// (`+0x98`) becomes that maximum minus 1. Then the actor's virtual `+0x490`
/// is called with (`index`, amount), the process's virtual `+0x3b0` with
/// `index` when the actor's virtual `+0x360` is false and it has a process,
/// and `0066ee50` reports the change.
pub fn fn_00880700(e: &mut Engine, this: Ptr<Actor>, index: u32, amount: f32) {
    let base = base_actor_value(e, this, index);
    let mut amount = amount;
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        let zero: f64 = e.global(0x0101_2060);
        if (amount as f64) < zero {
            amount = 0.0;
        } else {
            let record = e.call(0x0066_e920, &args![index]).u32();
            if record != 0 {
                let maximum = e.mem.i32(record + 0x98) as f64;
                if amount as f64 >= maximum {
                    let one: f64 = e.global(0x0101_2070);
                    amount = (maximum - one) as f32;
                }
            }
        }
    }
    e.vcall(this.addr(), 0x490, &args![index, amount]);
    if !e.vcall(this.addr(), 0x360, &args![]).bool() && !actor_process(e, this).is_null() {
        let process = actor_process(e, this);
        e.vcall(process.addr(), 0x3b0, &args![index]);
    }
    e.call(
        ACTOR_VALUE_CHANGED,
        &args![owner_of(this.addr()), index, base, amount, 0u32],
    );
}

// Translated from 00880850 (decompiled, FalloutNV.exe 1.4.0.525)
/// `00880890` on (`index`, `value` as a `float`, `source`), passed through
/// `00404040` and truncated.
pub fn fn_00880850(e: &mut Engine, this: Ptr, index: u32, value: i32, source: u32) -> i32 {
    let adjusted = fn_00880890(e, this, index, value as f32, source);
    let rounded = e.call(0x0040_4040, &args![adjusted]).f64();
    float_to_int(e, rounded)
}

// Translated from 00880890 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns its second word, a `float`, unchanged (the first and third words
/// are never read).
pub fn fn_00880890(_e: &mut Engine, _this: Ptr, _unused_0: u32, value: f32, _unused_2: u32) -> f32 {
    value
}

// Translated from 008808a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::DifficultyLevelAdjustHealthModifier` (Xbox PDB): `modifier` times
/// `00648cb0(difficulty, 0x10, actor's virtual +0x360)`, where the difficulty
/// is the player's `+0x7b8` (`005be4d0`). The second word is never read.
pub fn actor_difficulty_level_adjust_health_modifier(
    e: &mut Engine,
    this: Ptr<Actor>,
    modifier: f32,
    _unused_1: u32,
) -> f32 {
    let flag = e.vcall(this.addr(), 0x360, &args![]).bool();
    let player = e.global::<u32>(PLAYER_POINTER);
    let difficulty = e.call(0x005b_e4d0, &args![player]).u32();
    let factor = e.call(0x0064_8cb0, &args![difficulty, 0x10u32, flag]).f32();
    (modifier as f64 * factor as f64) as f32
}

// Translated from 008808f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `0047f060(index)`; the second word is never read.
pub fn fn_008808f0(e: &mut Engine, _this: Ptr, index: u32, _unused_1: u32) -> u32 {
    e.call(ACTOR_VALUE_IS_STAT, &args![index]).u32()
}

// Translated from 00880910 (decompiled, FalloutNV.exe 1.4.0.525)
/// `004bfc80` on the container changes (`00418520`) of the reference's extra
/// data list; false when it has none.
pub fn fn_00880910(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let mut result = false;
    let list = extra_data_list(e, this);
    let changes = e.call(0x0041_8520, &args![list]).u32();
    if changes != 0 {
        result = e.call(0x004b_fc80, &args![changes]).bool();
    }
    result
}

// Translated from 00880950 (decompiled, FalloutNV.exe 1.4.0.525)
/// Changes actor value `index` by the integer `value` (first adjusted by
/// `00880850`). Does nothing for index `0x16` with a negative value unless the
/// actor's virtual `+0x38c` is true, nor for values with the flag `0x100`.
/// Calls the process's virtual `+0x3a4` with (actor, `index`, value), for
/// index `0x10` with a negative value the actor's virtual `+0x4b8` with
/// (`source`, value as a `float`), sets the actor's flag bit `0x100000`
/// (virtual `+0x48`), and reports the change with `0066ee50`.
pub fn fn_00880950(e: &mut Engine, this: Ptr<Actor>, index: u32, value: i32, source: Ptr<Actor>) {
    if index == 0x16 && value < 0 && !e.vcall(this.addr(), 0x38c, &args![]).bool() {
        return;
    }
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        return;
    }
    let value = fn_00880850(e, this.cast(), index, value, source.addr());
    let base = base_actor_value(e, this, index);
    if !e.get(this, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, this);
        e.vcall(process.addr(), 0x3a4, &args![this, index, value]);
    }
    if index == 0x10 && value < 0 {
        e.vcall(this.addr(), 0x4b8, &args![source, value as f32]);
    }
    e.vcall(this.addr(), 0x48, &args![0x0010_0000u32]);
    fn_008808f0(e, this.cast(), index, 0);
    report_actor_value_change(e, this, index, base, value as f32, source.addr());
}

// Translated from 00880ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` counterpart of `00880950` (adjusted by `00880890`; process
/// virtual `+0x3a0`; the negative test is `amount < 0.0`).
pub fn fn_00880ad0(e: &mut Engine, this: Ptr<Actor>, index: u32, amount: f32, source: Ptr<Actor>) {
    let zero: f64 = e.global(0x0101_2060);
    if index == 0x16 && (amount as f64) < zero && !e.vcall(this.addr(), 0x38c, &args![]).bool() {
        return;
    }
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        return;
    }
    let amount = fn_00880890(e, this.cast(), index, amount, source.addr());
    let base = base_actor_value(e, this, index);
    if !e.get(this, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, this);
        e.vcall(process.addr(), 0x3a0, &args![this, index, amount]);
    }
    if index == 0x10 && (amount as f64) < zero {
        e.vcall(this.addr(), 0x4b8, &args![source, amount]);
    }
    e.vcall(this.addr(), 0x48, &args![0x0010_0000u32]);
    fn_008808f0(e, this.cast(), index, 0);
    report_actor_value_change(e, this, index, base, amount, source.addr());
}

// Translated from 00880c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like `00880950`, but the change goes to the modifier list at `+0xd0`
/// (`00937480(kind = index, value as a float, 2)`), the flag bit set is
/// `0x800000` and the process call is its virtual `+0x3b0` with `index`
/// (after the flag bit, with no `0x3a4` call before).
pub fn fn_00880c70(e: &mut Engine, this: Ptr<Actor>, index: u32, value: i32, source: Ptr<Actor>) {
    if index == 0x16 && value < 0 && !e.vcall(this.addr(), 0x38c, &args![]).bool() {
        return;
    }
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        return;
    }
    let value = fn_00880850(e, this.cast(), index, value, source.addr());
    let base = base_actor_value(e, this, index);
    e.call(
        0x0093_7480,
        &args![this.byte_add(0xd0), index as u8 as u32, value as f32, 2u32],
    );
    if index == 0x10 && value < 0 {
        e.vcall(this.addr(), 0x4b8, &args![source, value as f32]);
    }
    e.vcall(this.addr(), 0x48, &args![0x0080_0000u32]);
    fn_008808f0(e, this.cast(), index, 0);
    if !e.get(this, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, this);
        e.vcall(process.addr(), 0x3b0, &args![index]);
    }
    report_actor_value_change(e, this, index, base, value as f32, source.addr());
}

// Translated from 00880e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` counterpart of `00880c70` (adjusted by `00880890`).
pub fn fn_00880e00(e: &mut Engine, this: Ptr<Actor>, index: u32, amount: f32, source: Ptr<Actor>) {
    let zero: f64 = e.global(0x0101_2060);
    if index == 0x16 && (amount as f64) < zero && !e.vcall(this.addr(), 0x38c, &args![]).bool() {
        return;
    }
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        return;
    }
    let amount = fn_00880890(e, this.cast(), index, amount, source.addr());
    let base = base_actor_value(e, this, index);
    e.call(
        0x0093_7480,
        &args![this.byte_add(0xd0), index as u8 as u32, amount, 2u32],
    );
    if index == 0x10 && (amount as f64) < zero {
        e.vcall(this.addr(), 0x4b8, &args![source, amount]);
    }
    e.vcall(this.addr(), 0x48, &args![0x0080_0000u32]);
    fn_008808f0(e, this.cast(), index, 0);
    if !e.get(this, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, this);
        e.vcall(process.addr(), 0x3b0, &args![index]);
    }
    report_actor_value_change(e, this, index, base, amount, source.addr());
}

// Translated from 00880fb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like `00880950` with the process's virtual `+0x3ac` and the flag bit `0x200000`.
pub fn fn_00880fb0(e: &mut Engine, this: Ptr<Actor>, index: u32, value: i32, source: Ptr<Actor>) {
    if index == 0x16 && value < 0 && !e.vcall(this.addr(), 0x38c, &args![]).bool() {
        return;
    }
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        return;
    }
    let value = fn_00880850(e, this.cast(), index, value, source.addr());
    let base = base_actor_value(e, this, index);
    if !e.get(this, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, this);
        e.vcall(process.addr(), 0x3ac, &args![this, index, value]);
    }
    if index == 0x10 && value < 0 {
        e.vcall(this.addr(), 0x4b8, &args![source, value as f32]);
    }
    e.vcall(this.addr(), 0x48, &args![0x0020_0000u32]);
    fn_008808f0(e, this.cast(), index, 0);
    report_actor_value_change(e, this, index, base, value as f32, source.addr());
}

// Translated from 00881130 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` counterpart of `00880fb0` (adjusted by `00880890`; process
/// virtual `+0x3a8`). For index `0x16` with a negative amount the change is
/// also bounded by the float setting at `011d2664` (read through `00403e20`):
/// nothing happens unless the setting is below the base value, and an amount
/// that would take base plus amount to the setting or below becomes setting
/// minus base.
pub fn fn_00881130(e: &mut Engine, this: Ptr<Actor>, index: u32, amount: f32, source: Ptr<Actor>) {
    let zero: f64 = e.global(0x0101_2060);
    if index == 0x16 && (amount as f64) < zero && !e.vcall(this.addr(), 0x38c, &args![]).bool() {
        return;
    }
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        return;
    }
    let mut amount = fn_00880890(e, this.cast(), index, amount, source.addr());
    let base = base_actor_value(e, this, index);
    if index == 0x16 && (amount as f64) < zero {
        let limit_slot = e.call(0x0040_3e20, &args![0x011d_2664u32]).u32();
        let limit = e.mem.f32(limit_slot);
        if limit as f64 >= base as f64 {
            return;
        }
        let sum = (base as f64 + amount as f64) as f32;
        let limit_slot = e.call(0x0040_3e20, &args![0x011d_2664u32]).u32();
        let limit = e.mem.f32(limit_slot);
        if limit as f64 > sum as f64 {
            let limit_slot = e.call(0x0040_3e20, &args![0x011d_2664u32]).u32();
            let limit = e.mem.f32(limit_slot);
            amount = (limit as f64 - base as f64) as f32;
        }
    }
    if !e.get(this, Actor::pCurrentProcess).is_null() {
        let process = actor_process(e, this);
        e.vcall(process.addr(), 0x3a8, &args![this, index, amount]);
    }
    if index == 0x10 && (amount as f64) < zero {
        e.vcall(this.addr(), 0x4b8, &args![source, amount]);
    }
    e.vcall(this.addr(), 0x48, &args![0x0020_0000u32]);
    fn_008808f0(e, this.cast(), index, 0);
    report_actor_value_change(e, this, index, base, amount, source.addr());
}

// Translated from 00881330 (decompiled, FalloutNV.exe 1.4.0.525)
/// The actor's virtual `+0x3b4` with (`index`, `value` as a `float`).
pub fn fn_00881330(e: &mut Engine, this: Ptr<Actor>, index: u32, value: i32) {
    e.vcall(this.addr(), 0x3b4, &args![index, value as f32]);
}

// Translated from 00881360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `amount` to the current value of actor value `index` (the owner's
/// virtual `+4`) through the actor's virtual `+0x394`, then reports the
/// change (`0066ee50`) with the base value read before; does nothing for
/// values with the flag `0x100`.
pub fn fn_00881360(e: &mut Engine, this: Ptr<Actor>, index: u32, amount: f32) {
    if e.call(ACTOR_VALUE_HAS_FLAG, &args![index, 0x100u32]).bool() {
        return;
    }
    let base = base_actor_value(e, this, index);
    let current = e
        .vcall(this.addr() + ACTOR_VALUE_OWNER, 4, &args![index])
        .f32();
    let new_value = (current as f64 + amount as f64) as f32;
    e.vcall(this.addr(), 0x394, &args![index, new_value]);
    fn_008808f0(e, this.cast(), index, 0);
    e.call(
        ACTOR_VALUE_CHANGED,
        &args![owner_of(this.addr()), index, base, amount, 0u32],
    );
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x004b4880, ni_transform_invert_non_uniform(Ptr, Ptr)),
        entry!(
            0x00575610,
            actor_get_item_countin_container(Ptr<Actor>, Ptr) -> u32
        ),
        entry!(0x0087d4a0, actor_constructor(Ptr<Actor>, bool) -> Ptr),
        entry!(0x0087d860, fn_0087d860(Ptr) -> Ptr),
        entry!(0x0087d890, fn_0087d890(Ptr<Actor>, bool)),
        entry!(0x0087dcb0, actor_destructor(Ptr<Actor>)),
        entry!(0x0087e060, fn_0087e060(Ptr<Actor>, bool) -> Ptr),
        entry!(0x0087e100, fn_0087e100(Ptr, bool)),
        entry!(
            0x0087e130,
            actor_create_ragdoll_instance(Ptr<Actor>, Ptr, bool, bool, bool)
        ),
        entry!(0x0087e920, fn_0087e920(Ptr) -> bool),
        entry!(0x0087e940, fn_0087e940(Ptr) -> bool),
        entry!(0x0087e960, fn_0087e960(Ptr<Actor>, bool)),
        entry!(0x0087e980, fn_0087e980(Ptr) -> u32),
        entry!(0x0087e9a0, fn_0087e9a0() -> u32),
        entry!(0x0087e9b0, fn_0087e9b0(Ptr)),
        entry!(0x0087e9d0, fn_0087e9d0(Ptr, bool)),
        entry!(0x0087ea20, fn_0087ea20(Ptr) -> u8),
        entry!(0x0087ea40, fn_0087ea40(Ptr, u32)),
        entry!(0x0087ea60, fn_0087ea60(Ptr, u8)),
        entry!(0x0087ea80, fn_0087ea80(Ptr, Ptr)),
        entry!(0x0087eaa0, fn_0087eaa0(Ptr) -> Ptr),
        entry!(0x0087eac0, fn_0087eac0(Ptr<Actor>, Ptr, bool, bool)),
        entry!(0x0087eeb0, fn_0087eeb0(Ptr<Actor>) -> u8),
        entry!(0x0087eed0, fn_0087eed0(Ptr) -> u8),
        entry!(0x0087eef0, fn_0087eef0(Ptr<Actor>)),
        entry!(0x0087f200, fn_0087f200(Ptr<Actor>) -> bool),
        entry!(0x0087f260, fn_0087f260(Ptr<Actor>) -> bool),
        entry!(0x0087f350, fn_0087f350(Ptr<Actor>) -> bool),
        entry!(0x0087f370, fn_0087f370(Ptr<Actor>) -> bool),
        entry!(0x0087f390, fn_0087f390(Ptr<Actor>) -> bool),
        entry!(0x0087f3b0, fn_0087f3b0(Ptr<Actor>) -> bool),
        entry!(0x0087f3d0, actor_get_essential(Ptr<Actor>) -> bool),
        entry!(0x0087f480, fn_0087f480(Ptr) -> bool),
        entry!(0x0087f4a0, fn_0087f4a0(Ptr<Actor>) -> bool),
        entry!(0x0087f4c0, fn_0087f4c0(Ptr<Actor>) -> i32),
        entry!(0x0087f4e0, fn_0087f4e0(Ptr<Actor>) -> u32),
        entry!(0x0087f510, fn_0087f510(Ptr<Actor>) -> u8),
        entry!(0x0087f570, actor_swims_only(Ptr<Actor>) -> bool),
        entry!(0x0087f5c0, fn_0087f5c0(Ptr<Actor>) -> bool),
        entry!(0x0087f620, fn_0087f620(Ptr<Actor>) -> bool),
        entry!(0x0087f660, actor_check_breath_timer(Ptr<Actor>) -> bool),
        entry!(0x0087f6c0, fn_0087f6c0(Ptr<Actor>) -> u32),
        entry!(0x0087f750, fn_0087f750(Ptr<Actor>) -> Ptr),
        entry!(0x0087f7a0, fn_0087f7a0(Ptr<Actor>) -> Ptr),
        entry!(0x0087f800, fn_0087f800(Ptr<Actor>)),
        entry!(0x0087f890, fn_0087f890(Ptr<Actor>, Ptr, Ptr, Ptr, f32)),
        entry!(0x0087f900, fn_0087f900(Ptr<Actor>) -> bool),
        entry!(
            0x0087f920,
            fn_0087f920(Ptr<Actor>, Ptr, Ptr, Ptr, u32) -> bool
        ),
        entry!(0x0087f990, fn_0087f990(Ptr<Actor>) -> bool),
        entry!(0x0087f9c0, fn_0087f9c0(Ptr<Actor>) -> f32),
        entry!(0x0087f9f0, fn_0087f9f0(Ptr<Actor>) -> u32),
        entry!(0x0087fa10, fn_0087fa10(Ptr<Actor>, bool)),
        entry!(0x0087fa60, fn_0087fa60(Ptr<Actor>, bool)),
        entry!(0x0087faa0, actor_end_movement(Ptr<Actor>)),
        entry!(0x0087fad0, fn_0087fad0(Ptr<Actor>, u32)),
        entry!(0x0087fb40, fn_0087fb40(Ptr<Actor>, Ptr, f32)),
        entry!(0x0087fcb0, fn_0087fcb0(Ptr<Actor>, u32) -> f32),
        entry!(0x0087fd20, actor_clear_disposition_modifiers(Ptr<Actor>)),
        entry!(0x0087fd90, fn_0087fd90(Ptr<Actor>, Ptr<Actor>, Ptr) -> i32),
        entry!(0x00880370, fn_00880370(Ptr, u32) -> i32),
        entry!(0x008803a0, fn_008803a0(Ptr, u32) -> f32),
        entry!(0x00880580, fn_00880580(Ptr, u32) -> u32),
        entry!(0x008805f0, fn_008805f0(Ptr, u32) -> f32),
        entry!(0x00880660, fn_00880660(Ptr<Actor>, u8, u32) -> f32),
        entry!(0x00880690, fn_00880690(Ptr<Actor>, u8, f32)),
        entry!(0x008806d0, fn_008806d0(Ptr<Actor>, u32, i32)),
        entry!(0x00880700, fn_00880700(Ptr<Actor>, u32, f32)),
        entry!(0x00880850, fn_00880850(Ptr, u32, i32, u32) -> i32),
        entry!(0x00880890, fn_00880890(Ptr, u32, f32, u32) -> f32),
        entry!(
            0x008808a0,
            actor_difficulty_level_adjust_health_modifier(Ptr<Actor>, f32, u32) -> f32
        ),
        entry!(0x008808f0, fn_008808f0(Ptr, u32, u32) -> u32),
        entry!(0x00880910, fn_00880910(Ptr<Actor>) -> bool),
        entry!(0x00880950, fn_00880950(Ptr<Actor>, u32, i32, Ptr<Actor>)),
        entry!(0x00880ad0, fn_00880ad0(Ptr<Actor>, u32, f32, Ptr<Actor>)),
        entry!(0x00880c70, fn_00880c70(Ptr<Actor>, u32, i32, Ptr<Actor>)),
        entry!(0x00880e00, fn_00880e00(Ptr<Actor>, u32, f32, Ptr<Actor>)),
        entry!(0x00880fb0, fn_00880fb0(Ptr<Actor>, u32, i32, Ptr<Actor>)),
        entry!(0x00881130, fn_00881130(Ptr<Actor>, u32, f32, Ptr<Actor>)),
        entry!(0x00881330, fn_00881330(Ptr<Actor>, u32, i32)),
        entry!(0x00881360, fn_00881360(Ptr<Actor>, u32, f32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn ret(value: u32) -> Ret {
        value.into_ret()
    }

    /// An engine whose listed callees are doubles returning zero.
    fn engine(addrs: &[u32]) -> Engine {
        let mut e = Engine::new();
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
        e
    }

    /// The address a test vtable slot jumps to.
    fn slot_target(vtable: u32, offset: u32) -> u32 {
        0x6000_0000 + (vtable & 0x00ff_ffff) + offset
    }

    /// An object of `size` bytes whose vtable (at `vtable`, a page-aligned
    /// address) has the listed slots, each a double returning zero.
    fn object(e: &mut Engine, size: u32, vtable: u32, offsets: &[u32]) -> Ptr {
        e.map(vtable, 0x1000);
        for offset in offsets {
            let target = slot_target(vtable, *offset);
            e.mem.set_u32(vtable + offset, target);
            e.register(target, |_, _| Ret::default());
        }
        let object = Ptr::new(e.mem.alloc(size));
        e.mem.set_u32(object.addr(), vtable);
        object
    }

    /// The addresses called so far (the test's own top-level call included).
    fn called(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(addr, _)| *addr)
            .collect()
    }

    /// The argument words of the first call to `addr`.
    fn args_of(e: &Engine, addr: u32) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .find(|(a, _)| *a == addr)
            .unwrap_or_else(|| panic!("{addr:08x} was not called"))
            .1
            .clone()
    }

    fn map_pages(e: &mut Engine, pages: &[u32]) {
        for page in pages {
            e.map(*page, 0x1000);
        }
    }

    // ---- 004b4880 ----

    #[test]
    fn invert_non_uniform_transposes_inverts_the_scale_and_rebuilds_the_translation() {
        let mut e = Engine::new();
        let transposed = e.mem.alloc(0x24);
        for i in 0..9 {
            e.mem.set_u32(transposed + 4 * i, 0x100 + i);
        }
        let scaled = e.mem.alloc(0x10);
        for i in 0..3 {
            e.mem.set_u32(scaled + 4 * i, 0x200 + i);
        }
        e.register_double(0x0047_68c0, move |_, _| ret(transposed));
        e.register(0x004a_0bd0, |_, _| ret(0x7001));
        e.register(0x004b_4500, |_, _| ret(0x7002));
        e.register_double(0x004a_3760, move |_, _| ret(scaled));
        let source = e.mem.alloc(0x40);
        e.mem.set_f32(source + 0x30, 4.0);
        let result = e.mem.alloc(0x40);
        e.call_log = Some(vec![]);
        e.call(
            0x004b_4880,
            &args![Ptr::<()>::new(source), Ptr::<()>::new(result)],
        );
        for i in 0..9 {
            assert_eq!(e.mem.u32(result + 4 * i), 0x100 + i);
        }
        assert_eq!(e.mem.f32(result + 0x30), 0.25);
        for i in 0..3 {
            assert_eq!(e.mem.u32(result + 0x24 + 4 * i), 0x200 + i);
        }
        // The translation goes through the helpers: negate the translate at
        // +0x24 of the source, rotate it by the result, scale by 1/scale.
        let negate = args_of(&e, 0x004a_0bd0);
        assert_eq!(negate[0], source + 0x24);
        let rotate = args_of(&e, 0x004b_4500);
        assert_eq!((rotate[0], rotate[2]), (result, 0x7001));
        let scale = args_of(&e, 0x004a_3760);
        assert_eq!((scale[1], scale[2]), (0.25f32.to_bits(), 0x7002));
    }

    // ---- 00575610 ----

    #[test]
    fn get_item_countin_container_is_zero_without_inventory_changes() {
        let mut e = engine(&[0x004b_f220]);
        e.register(0x004c_8f30, |_, _| ret(9));
        let actor: Ptr<Actor> = e.new_object();
        let item = Ptr::<()>::new(0x1234);
        assert_eq!(e.call(0x0057_5610, &args![actor, item]).u32(), 0);
    }

    #[test]
    fn get_item_countin_container_asks_the_inventory_changes() {
        let mut e = engine(&[]);
        e.register(0x004b_f220, |_, _| ret(0x5000));
        e.register(0x004c_8f30, |_, a| ret(a[0] + a[1]));
        let actor: Ptr<Actor> = e.new_object();
        let item = Ptr::<()>::new(7);
        assert_eq!(e.call(0x0057_5610, &args![actor, item]).u32(), 0x5007);
    }

    // ---- 0087d860 ----

    #[test]
    fn fn_0087d860_installs_its_vtable() {
        let mut e = Engine::new();
        let part = Ptr::<()>::new(e.mem.alloc(8));
        assert_eq!(e.call(0x0087_d860, &args![part]).ptr::<()>(), part);
        assert_eq!(e.mem.u32(part.addr()), 0x0108_4734);
    }

    // ---- 0087d890 and 0087d4a0 ----

    /// Doubles for everything the initializer calls, and the pages that hold
    /// the constants and settings it reads.
    fn init_engine(process_block: u32) -> Engine {
        let mut e = engine(&[
            0x0046_a010,
            0x008a_08e0,
            0x008b_bdb0,
            0x008b_be00,
            0x008c_4400,
            0x0093_7090,
            0x0096_e870,
            0x0096_d450,
            0x0092_eb50,
            0x0081_5300,
            0x0082_2a40,
            0x005f_78e0,
            0x0093_7000,
            0x0096_a2d0,
            0x0068_15c0,
        ]);
        map_pages(
            &mut e,
            &[0x0101_2000, 0x0101_7000, 0x011f_4000, 0x011d_f000],
        );
        e.set_global(0x0101_2054, -1.0f32);
        e.set_global(0x0101_7718, 3.0f32);
        e.set_global(0x011f_426c, 0x1111u32);
        e.set_global(0x011f_4270, 0x2222u32);
        e.set_global(0x011f_4274, 0x3333u32);
        e.set_global(0x011d_f810, 2.0f32);
        e.set_global(0x011d_f828, 5.0f32);
        e.register_double(0x0040_1000, move |_, _| ret(process_block));
        e.register(0x0090_6dc0, |_, a| ret(a[0]));
        e.register(0x0055_3300, |e, _| ret(e.mem.alloc(0x30)));
        e.register(0x00ca_0dc0, |_, a| ret(a[0]));
        // Settings: the value pointer is the setting's own address.
        e.register(0x0040_3e20, |_, a| ret(a[0]));
        // RandomFloat(low, high): encodes the argument order.
        e.register(0x0047_6b70, |_, a| {
            (f32::from_bits(a[0]) * 10.0 + f32::from_bits(a[1])).into_ret()
        });
        e
    }

    fn init_actor(e: &mut Engine) -> Ptr<Actor> {
        let actor = object(e, 0x1b4, 0x0200_0000, &[0x4d0, 0x3c4]);
        // Stale values the initializer must overwrite.
        for offset in (0x68..0x1b4).step_by(4) {
            e.mem.set_u32(actor.addr() + offset, 0xdead_beef);
        }
        actor.cast()
    }

    #[test]
    fn initializer_sets_every_field_and_registers_with_the_process_lists() {
        let mut e = init_engine(0x5000);
        let actor = init_actor(&mut e);
        e.register(0x008c_7aa0, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(0x0087_d890, &args![actor, true]);
        assert_eq!(e.get(actor, Actor::pCurrentProcess), Ptr::new(0x5000));
        assert_eq!(e.get(actor, Actor::fLastUpdate), -1.0);
        assert_eq!(e.get(actor, Actor::ePersuasionEmotion), 8);
        assert!(e.get(actor, Actor::bProcessMe));
        assert_eq!(e.get(actor, Actor::eQueuedattack), 0xff);
        assert_eq!(e.get(actor, Actor::fDeadBodyAlarm), 3.0);
        // RandomFloat(setting at 011df810, setting at 011df828).
        assert_eq!(e.get(actor, Actor::fUpdateTargetTimer), 25.0);
        assert_eq!(e.mem.u32(actor.addr() + 0x160), 0x1111);
        assert_eq!(e.mem.u32(actor.addr() + 0x168), 0x3333);
        assert!(e.get(actor, Actor::bShouldRotateToTrack));
        assert!(e.get(actor, Actor::bContainerReset));

        assert_eq!(e.get(actor, Actor::pContinuousBeamPersistant), Ptr::NULL);
        assert_eq!(e.get(actor, Actor::pLastHitData), Ptr::NULL);
        assert_eq!(e.get(actor, Actor::pInitialPackage), Ptr::NULL);
        assert_eq!(e.get(actor, Actor::iMinorCrimes), 0);
        assert_eq!(e.get(actor, Actor::pRagdollController), Ptr::NULL);
        assert!(!e.get(actor, Actor::pPenetrationDetection).is_null());
        // Not in the temp-change list: the plain AddReference(actor, 3, 0, 0, 0).
        assert_eq!(
            args_of(&e, 0x0096_d450),
            vec![PROCESS_LISTS, actor.addr(), 3, 0, 0, 0]
        );
        let calls = called(&e);
        assert!(!calls.contains(&0x0096_e870));
        // The clear-in-combat call comes after the virtual at +0x4d0.
        let pos = |addr| calls.iter().position(|a| *a == addr).unwrap();
        assert!(pos(0x008a_08e0) > pos(slot_target(0x0200_0000, 0x4d0)));
        assert!(pos(0x008b_bdb0) > pos(0x00ca_0dc0));
        assert_eq!(args_of(&e, 0x0093_7090), vec![actor.addr() + 0xe0, 1]);
    }

    #[test]
    fn initializer_uses_the_temp_change_list_and_survives_failed_allocations() {
        let mut e = init_engine(0);
        let actor = init_actor(&mut e);
        e.register(0x008c_7aa0, |_, _| ret(1));
        e.register(0x0055_3300, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(0x0087_d890, &args![actor, true]);
        assert_eq!(e.get(actor, Actor::pCurrentProcess), Ptr::NULL);
        assert_eq!(e.get(actor, Actor::pPenetrationDetection), Ptr::NULL);
        assert_eq!(args_of(&e, 0x0096_e870), vec![PROCESS_LISTS, actor.addr()]);
        assert!(!called(&e).contains(&0x0096_d450));
        assert!(!called(&e).contains(&0x0090_6dc0));
        assert!(!called(&e).contains(&0x00ca_0dc0));
    }

    #[test]
    fn initializer_skips_the_process_lists_when_not_asked() {
        let mut e = init_engine(0x5000);
        let actor = init_actor(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0087_d890, &args![actor, false]);
        let calls = called(&e);
        assert!(!calls.contains(&0x0096_e870) && !calls.contains(&0x0096_d450));
        assert_eq!(args_of(&e, 0x0046_a010), vec![actor.addr(), 1]);
    }

    #[test]
    fn constructor_builds_the_parts_installs_the_vtables_and_initializes() {
        let mut e = init_engine(0x5000);
        e.register(0x008c_7aa0, |_, _| ret(0));
        let actor = Ptr::<Actor>::new(e.mem.alloc(0x1b4));
        // The initializer calls the virtuals at +0x4d0 and +0x3c4 of the new
        // vtable (01084254), which the test supplies.
        e.map(0x0108_4000, 0x1000);
        e.register(0x7000_0001, |_, _| Ret::default());
        e.mem.set_u32(0x0108_4254 + 0x4d0, 0x7000_0001);
        e.mem.set_u32(0x0108_4254 + 0x3c4, 0x7000_0001);
        e.call_log = Some(vec![]);
        let back = e.call(0x0087_d4a0, &args![actor, true]).ptr::<Actor>();
        assert_eq!(back, actor);
        for (offset, vtable) in [
            (0x00, 0x0108_4254),
            (0x18, 0x0108_4248),
            (0x88, 0x0108_41f4),
            (0x94, 0x0108_41bc),
            (0xa4, 0x0108_418c),
            (0xa8, 0x0108_4144),
        ] {
            assert_eq!(e.mem.u32(actor.addr() + offset), vtable);
        }
        let calls = called(&e);
        let pos = |addr| calls.iter().position(|a| *a == addr).unwrap();
        // Base first, parts in offset order, the initializer last.
        assert_eq!(args_of(&e, 0x0092_eb50), vec![actor.addr()]);
        assert!(pos(0x0092_eb50) < pos(0x0081_5300));
        assert!(pos(0x0081_5300) < pos(0x0082_2a40));
        assert!(pos(0x0082_2a40) < pos(0x005f_78e0));
        assert!(pos(0x005f_78e0) < pos(0x0093_7000));
        assert!(pos(0x0096_a2d0) < pos(0x0068_15c0));
        assert!(pos(0x0068_15c0) < pos(0x0046_a010));
        assert_eq!(e.mem.u32(actor.addr() + 0xa8), 0x0108_4144);
    }

    // ---- 0087dcb0 ----

    const DESTRUCTOR_CALLEES: [u32; 38] = [
        0x0041_a250,
        0x0041_8890,
        0x00ad_8ce0,
        0x00ad_8d10,
        0x0083_5980,
        0x0040_77c0,
        REMOVE_FROM_PLAYER_COMBAT_LIST,
        REMOVE_PERCEIVED_ACTOR,
        0x0096_e6f0,
        0x0087_fd20,
        0x0090_6050,
        0x0097_8660,
        0x0042_26e0,
        0x0082_4970,
        0x0066_e100,
        0x0092_ed60,
        0x0088_1680,
        0x0097_6680,
        0x0096_f600,
        0x0097_5320,
        0x0045_cd60,
        0x00b4_f5c0,
        0x0084_e3a0,
        0x00b6_6050,
        0x008c_30a0,
        0x008c_51f0,
        0x0096_f400,
        STOP_COMBAT,
        0x008c_5090,
        0x0093_70b0,
        0x008b_3180,
        SET_3D,
        0x0048_3710,
        0x0046_ffb0,
        0x0093_7030,
        0x0082_2a80,
        0x0081_53b0,
        0x0092_ec40,
    ];

    fn destructor_setup() -> (Engine, Ptr<Actor>, Ptr, Ptr) {
        let mut e = engine(&DESTRUCTOR_CALLEES);
        e.register(GET_EXTRA_DATA_LIST, |_, a| ret(a[0] + 0x44));
        let actor: Ptr<Actor> = object(&mut e, 0x1b4, 0x0200_0000, &[]).cast();
        let ragdoll = object(&mut e, 0x10, 0x0201_0000, &[0]);
        let detection = object(&mut e, 0x10, 0x0202_0000, &[0]);
        e.set(actor, Actor::pRagdollController, ragdoll);
        e.set(actor, Actor::pPenetrationDetection, detection);
        e.map(0x011d_e000, 0x1000);
        e.map(0x011c_3000, 0x1000);
        e.set_global(PLAYER_POINTER, 0x7777u32);
        e.set_global(SHUTDOWN_CHECK_OBJECT_POINTER, 0x8888u32);
        (e, actor, ragdoll, detection)
    }

    #[test]
    fn destructor_takes_the_actor_out_of_every_list_and_runs_the_member_destructors() {
        let (mut e, actor, ragdoll, detection) = destructor_setup();
        let process = Ptr::<()>::new(e.mem.alloc(0x40));
        e.set(actor, Actor::pCurrentProcess, process);
        e.set(actor, Actor::bInCombat, true);
        e.register(0x0045_cd60, |_, _| ret(1));
        e.register(0x00b4_f5c0, |_, _| ret(0x9000));
        e.register(0x0084_e3a0, |_, _| ret(0x9100));
        e.register(0x00ad_8ce0, |_, _| ret(1));
        e.call_log = Some(vec![]);
        e.call(0x0087_dcb0, &args![actor]);
        for (offset, vtable) in [
            (0x00, 0x0108_4254),
            (0x18, 0x0108_4248),
            (0x88, 0x0108_41f4),
            (0x94, 0x0108_41bc),
            (0xa4, 0x0108_418c),
            (0xa8, 0x0108_4144),
        ] {
            assert_eq!(e.mem.u32(actor.addr() + offset), vtable);
        }
        let calls = called(&e);
        let pos = |addr| calls.iter().position(|a| *a == addr).unwrap();
        // Both objects are deleted through their first virtual with flag 1.
        assert_eq!(
            args_of(&e, slot_target(0x0201_0000, 0)),
            vec![ragdoll.addr(), 1]
        );
        assert_eq!(
            args_of(&e, slot_target(0x0202_0000, 0)),
            vec![detection.addr(), 1]
        );
        // The sound handle is released because it is valid.
        assert!(calls.contains(&0x00ad_8d10));
        assert_eq!(args_of(&e, 0x0041_8890)[0], actor.addr() + 0x44);
        assert_eq!(
            args_of(&e, REMOVE_FROM_PLAYER_COMBAT_LIST),
            vec![0x7777, actor.addr()]
        );
        assert_eq!(
            args_of(&e, REMOVE_PERCEIVED_ACTOR),
            vec![0x7777, actor.addr()]
        );
        assert_eq!(
            args_of(&e, 0x0096_f600),
            vec![PROCESS_LISTS, actor.addr(), 1]
        );
        assert_eq!(args_of(&e, 0x0042_26e0), vec![0x8888]);
        // The process kind is 1: the accumulator is told, then 008c30a0 runs.
        assert_eq!(args_of(&e, 0x00b6_6050), vec![0x9000, 0x9100]);
        assert!(pos(0x00b6_6050) < pos(0x008c_30a0));
        assert!(pos(0x008c_30a0) < pos(0x008c_51f0));
        assert_eq!(args_of(&e, STOP_COMBAT), vec![actor.addr(), 0]);
        assert_eq!(args_of(&e, SET_3D), vec![actor.addr(), 0, 0]);
        assert_eq!(args_of(&e, 0x0093_70b0), vec![actor.addr() + 0xd0]);
        // Member destructors in reverse order of construction; base last.
        assert!(pos(0x0046_ffb0) > pos(SET_3D));
        assert!(pos(0x0093_7030) < pos(0x0082_2a80));
        assert!(pos(0x0082_2a80) < pos(0x0081_53b0));
        assert_eq!(*calls.last().unwrap(), 0x0092_ec40);
    }

    #[test]
    fn destructor_skips_the_lists_when_the_shutdown_check_says_so() {
        let (mut e, actor, _, _) = destructor_setup();
        e.register(0x0040_77c0, |_, _| ret(1));
        e.call_log = Some(vec![]);
        e.call(0x0087_dcb0, &args![actor]);
        let calls = called(&e);
        for skipped in [
            REMOVE_FROM_PLAYER_COMBAT_LIST,
            0x0096_e6f0,
            0x0042_26e0,
            SET_3D,
            0x0096_f400,
        ] {
            assert!(!calls.contains(&skipped), "{skipped:08x} must be skipped");
        }
        // The members are still taken apart.
        assert!(calls.contains(&0x0046_ffb0) && calls.contains(&0x0092_ec40));
    }

    #[test]
    fn destructor_only_does_part_of_the_magic_teardown_while_shutting_down() {
        let (mut e, actor, _, _) = destructor_setup();
        e.register(0x0042_26e0, |_, _| ret(1));
        e.call_log = Some(vec![]);
        e.call(0x0087_dcb0, &args![actor]);
        let calls = called(&e);
        assert!(calls.contains(&0x0096_e6f0));
        assert!(!calls.contains(&0x0082_4970) && !calls.contains(&0x0097_6680));
        assert!(calls.contains(&0x0096_f400) && calls.contains(&SET_3D));
    }

    // ---- 0087e060 and 0087e100 ----

    #[test]
    fn load3d_wrapper_returns_null_without_a_node() {
        let mut e = engine(&[MOBILE_OBJECT_LOAD3D]);
        let actor: Ptr<Actor> = e.new_object();
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0087_e060, &args![actor, true]).u32(), 0);
        assert_eq!(called(&e), vec![0x0087_e060, MOBILE_OBJECT_LOAD3D]);
    }

    #[test]
    fn load3d_wrapper_hands_the_node_to_the_process_and_sets_the_flag_bits() {
        let mut e = engine(&[SET_FLAG_BITS, 0x008b_7360]);
        e.register(MOBILE_OBJECT_LOAD3D, |_, _| ret(0x4000));
        e.register(GET_BASE_FORM, |e, a| ret(e.mem.u32(a[0] + 0x20)));
        e.register(0x008b_7b70, |_, _| ret(1));
        // The setting at 011c7664 holds 0.
        e.map(0x011c_7000, 0x1000);
        e.register(SETTING_VALUE_POINTER, |_, _| ret(0x011c_7800));
        let actor: Ptr<Actor> = object(&mut e, 0x1b4, 0x0200_0000, &[]).cast();
        let base = object(&mut e, 0x40, 0x0203_0000, &[0x180]);
        e.register(slot_target(0x0203_0000, 0x180), |_, _| ret(0x6100));
        e.mem.set_u32(actor.addr() + 0x20, base.addr());
        let process = object(&mut e, 0x10, 0x0204_0000, &[0x6c8]);
        e.set(actor, Actor::pCurrentProcess, process);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0087_e060, &args![actor, false]).u32(), 0x4000);
        assert_eq!(
            args_of(&e, slot_target(0x0204_0000, 0x6c8)),
            vec![process.addr(), 0x4000, 0x6100]
        );
        // The setting is off: the node's bits are asked to be set.
        assert_eq!(args_of(&e, SET_FLAG_BITS), vec![0x4000, 1, 0x0080_0000]);
        assert_eq!(args_of(&e, 0x008b_7360), vec![actor.addr(), 1, 0]);
    }

    #[test]
    fn load3d_wrapper_leaves_the_flag_bits_when_the_setting_is_on() {
        let mut e = engine(&[SET_FLAG_BITS, 0x008b_7b70]);
        e.register(MOBILE_OBJECT_LOAD3D, |_, _| ret(0x4000));
        e.map(0x011c_7000, 0x1000);
        e.mem.set_u8(0x011c_7800, 1);
        e.register(SETTING_VALUE_POINTER, |_, _| ret(0x011c_7800));
        let actor: Ptr<Actor> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0087_e060, &args![actor, true]);
        let calls = called(&e);
        assert!(!calls.contains(&SET_FLAG_BITS) && !calls.contains(&0x008b_7360));
    }

    #[test]
    fn fn_0087e100_asks_for_the_bits_to_be_set_when_the_flag_is_false() {
        let mut e = engine(&[SET_FLAG_BITS]);
        e.call_log = Some(vec![]);
        e.call(0x0087_e100, &args![Ptr::<()>::new(0x4000), false]);
        assert_eq!(args_of(&e, SET_FLAG_BITS), vec![0x4000, 1, 0x0080_0000]);
        e.call_log = Some(vec![]);
        e.call(0x0087_e100, &args![Ptr::<()>::new(0x4000), true]);
        assert_eq!(args_of(&e, SET_FLAG_BITS), vec![0x4000, 0, 0x0080_0000]);
    }

    // ---- 0087e130 ----

    const RAGDOLL_CALLEES: [u32; 50] = [
        0x004f_d380,
        0x004a_de00,
        0x005f_0c80,
        0x0050_fd90,
        0x0045_c6b0,
        0x0044_7080,
        0x0050_fdf0,
        0x0040_6d00,
        0x0056_fac0,
        0x004e_a950,
        0x004b_46a0,
        0x0096_a2d0,
        0x005e_5320,
        0x005e_ede0,
        0x0063_f7b0,
        0x007c_b2e0,
        0x004e_4660,
        0x00c7_de60,
        0x0051_b860,
        0x0060_7810,
        0x005b_a4f0,
        0x005b_a540,
        0x005c_6b40,
        0x00c7_c3a0,
        0x005b_a130,
        0x005b_a310,
        0x005b_a180,
        0x0044_ddc0,
        0x005b_a360,
        0x008b_c300,
        0x005e_ee10,
        0x0046_ffb0,
        0x0093_1ed0,
        0x004a_3a20,
        0x0040_6d30,
        0x0040_6d50,
        0x0043_8170,
        0x0063_3c90,
        0x0043_b1b0,
        0x00c7_b1f0,
        0x0045_cec0,
        0x0043_81b0,
        0x00c7_5ad0,
        0x0093_06d0,
        0x0047_4cb0,
        0x0055_d520,
        0x005b_5e40,
        0x0063_d040,
        0x00ec_7750,
        0x0040_ab30,
    ];

    struct Ragdoll {
        e: Engine,
        actor: Ptr<Actor>,
        controller: Ptr,
    }

    /// An actor whose base form (`GetBaseForm` result) is `node`'s owner: the
    /// base form's virtual `+0x180` gives `node`, whose virtuals `+0x130` and
    /// (at `+0x18`) `+0x14` give names. The controller built by `00c7f060` is
    /// `controller`, with `bInitRagdollAnim` = `controller_ok`.
    fn ragdoll_setup(controller_ok: bool) -> Ragdoll {
        let mut e = engine(&RAGDOLL_CALLEES);
        map_pages(
            &mut e,
            &[
                0x011a_9000,
                0x011c_6000,
                0x0102_3000,
                0x011d_e000,
                0x011c_3000,
            ],
        );
        e.set_global(PLAYER_POINTER, 0x7777u32);
        e.set_global(MODEL_LOADER_POINTER, 0x6666u32);
        e.set_global(DEGREES_TO_RADIANS, 0.5f64);
        for i in 0..9u32 {
            e.set_global(IDENTITY_MATRIX + 4 * i, 0x300 + i);
        }
        let node = object(&mut e, 0x40, 0x0210_0000, &[0x130]);
        e.mem.set_u32(node.addr() + 0x18, 0x0211_0000);
        e.map(0x0211_0000, 0x1000);
        e.mem.set_u32(0x0211_0000 + 0x14, 0x6001_1001);
        e.register(0x6001_1001, |e, _| ret(e.mem.alloc(0x40)));
        let base = object(&mut e, 0x40, 0x0212_0000, &[0x180]);
        e.register_double(slot_target(0x0212_0000, 0x180), {
            let node = node.addr();
            move |_, _| ret(node)
        });
        e.register_double(GET_BASE_FORM, move |_, _| ret(base.addr()));
        e.register(GET_FORM, |_, a| ret(a[0] + 0x20));
        let controller = Ptr::<()>::new(e.mem.alloc(0x300));
        e.mem.set_u8(controller.addr() + 0x42, controller_ok as u8);
        e.register_double(0x00c7_f060, {
            let controller = controller.addr();
            move |_, _| ret(controller)
        });
        e.register(ALLOCATE_BLOCK, |e, a| ret(e.mem.alloc(a[0])));
        e.register(0x0068_15c0, |_, a| ret(a[0]));
        e.register(0x004a_de00, |_, _| ret(0));
        let actor: Ptr<Actor> = object(&mut e, 0x1b4, 0x0213_0000, &[0x130, 0x21c]).cast();
        e.register(slot_target(0x0213_0000, 0x130), |_, _| ret(0xa100));
        // Strings the ragdoll code copies.
        e.register(0x0040_6d30, |e, a| {
            e.mem
                .set_cstr(a[0], b"Data\\Meshes\\Characters\\Skeleton.nif");
            Ret::default()
        });
        e.register(0x0040_ab30, |e, a| {
            let text = e.mem.cstr(a[0]);
            let index = text.iter().rposition(|byte| *byte == b'\\').unwrap();
            ret(a[0] + index as u32)
        });
        Ragdoll {
            e,
            actor,
            controller,
        }
    }

    #[test]
    fn ragdoll_of_an_immobile_actor_outside_a_world_space_only_sets_the_turret_flag() {
        let mut r = ragdoll_setup(true);
        r.e.register(0x004a_de00, |_, _| ret(1));
        r.e.register(0x005f_0c80, |_, _| ret(1));
        // No world space: 004fd380 gives null.
        r.e.call_log = Some(vec![]);
        r.e.call(
            0x0087_e130,
            &args![r.actor, Ptr::<()>::new(0x5555), false, true, false],
        );
        assert_eq!(r.e.get(r.actor, Actor::bTurretBehavior), 1);
        let calls = called(&r.e);
        assert!(!calls.contains(&GET_FORM) && !calls.contains(&0x00c7_f060));
        assert_eq!(args_of(&r.e, 0x004a_de00), vec![0x5555, 0]);
    }

    #[test]
    fn ragdoll_replaces_the_existing_controller_and_reports_a_failed_initialization() {
        let mut r = ragdoll_setup(false);
        let old = object(&mut r.e, 0x20, 0x0214_0000, &[0]);
        r.e.set(r.actor, Actor::pRagdollController, old);
        r.e.register(0x0047_4cb0, |_, _| ret(1));
        r.e.call_log = Some(vec![]);
        r.e.call(
            0x0087_e130,
            &args![r.actor, Ptr::<()>::new(0x5555), true, true, true],
        );
        assert_eq!(
            args_of(&r.e, slot_target(0x0214_0000, 0)),
            vec![old.addr(), 1]
        );
        // The new controller is stored, the name has the id and old pointer.
        assert_eq!(r.e.get(r.actor, Actor::pRagdollController), r.controller);
        let format = args_of(&r.e, 0x0040_6d00);
        assert_eq!(format[1..3], [0x104, RAGDOLL_NAME_FORMAT]);
        assert_eq!(format[4..], [r.actor.addr(), old.addr()]);
        // ModelLoader::LoadFile(file, lod, 1, 0, 0, 1) on the loader pointer.
        assert_eq!(args_of(&r.e, 0x0044_7080), vec![0x6666, 0, 0, 1, 0, 0, 1]);
        // The error names the actor through its virtual +0x130.
        assert_eq!(
            args_of(&r.e, 0x005b_5e40),
            vec![RAGDOLL_ERROR_CONTROLLER, 0xa100]
        );
        assert!(!called(&r.e).contains(&0x0056_fac0));
    }

    #[test]
    fn ragdoll_without_iks_sets_the_group_pose_matching_and_the_character_proxy() {
        let mut r = ragdoll_setup(true);
        let world = Ptr::<()>::new(0x8000);
        r.e.register(0x004f_d380, |_, _| ret(0x8000));
        r.e.register(0x004a_3a20, |_, _| ret(0xabc));
        r.e.register(0x0043_b1b0, |_, _| ret(0xf00d));
        r.e.register(0x0093_06d0, |e, _| {
            let controller = e.mem.alloc(0x600);
            e.mem.set_u32(controller + 0x594, 0xcafe);
            ret(controller)
        });
        r.e.register(0x0055_9450, |e, a| ret(e.mem.u32(a[0])));
        r.e.register(0x004a_e750, |_, a| ret(a[0] + 1));
        r.e.register(0x004e_a950, |_, _| ret(0x6a6a));
        r.e.call_log = Some(vec![]);
        r.e.call(
            0x0087_e130,
            &args![r.actor, Ptr::<()>::new(0x5555), false, false, false],
        );
        let calls = called(&r.e);
        // No IK set-up.
        assert!(!calls.contains(&0x0096_a2d0) && !calls.contains(&0x00c7_c3a0));
        // The identity matrix is copied before MultipleMatrixByRace and the
        // inverse is taken against the controller's matrix.
        let multiply = args_of(&r.e, 0x0056_fac0);
        assert_eq!(multiply[0], r.actor.addr());
        for i in 0..9u32 {
            assert_eq!(r.e.mem.u32(multiply[2] + 4 * i), 0x300 + i);
        }
        assert_eq!(args_of(&r.e, 0x004b_46a0), vec![multiply[2], 0x6a6a]);
        // The group comes from 004a3a20, the pose file path ends in Death.psa.
        assert_eq!(
            r.e.mem.u32(r.controller.addr() + 0x5c),
            0xabc,
            "iGroup (+0x5c)"
        );
        let path = args_of(&r.e, 0x0043_8170)[1];
        assert_eq!(r.e.mem.cstr(path), b"Data\\Meshes\\Characters\\");
        assert_eq!(
            args_of(&r.e, 0x0040_6d50),
            vec![path, 0x104, DEATH_POSE_FILE]
        );
        assert_eq!(
            args_of(&r.e, 0x00c7_b1f0),
            vec![r.controller.addr(), 0xf00d, u32::MAX, u32::MAX, u32::MAX]
        );
        // A world space and a non-player actor: the area at +0x40 is given.
        assert_eq!(
            args_of(&r.e, 0x00c7_5ad0),
            vec![r.controller.addr(), world.addr() + 0x40]
        );
        // The character controller's word at +0x594 goes through 004ae750 into +0x90.
        assert_eq!(r.e.mem.u32(r.controller.addr() + 0x90), 0xcaff);
        // bFlag254 is zero, so +0x255 is zero.
        assert_eq!(r.e.mem.u8(r.controller.addr() + 0x255), 0);
    }

    #[test]
    fn ragdoll_walks_the_nodes_for_look_ik_and_foot_ik() {
        let mut r = ragdoll_setup(true);
        r.e.set_global(PLAYER_POINTER, r.actor.addr());
        // Two nodes: the first has bit 4 (collected), the second bit 0x20
        // (look IK); the Prime name is found; foot IK fails and complains.
        let first = r.e.mem.alloc(0x80);
        r.e.mem.set_u8(first + 0x60, 4);
        let second = r.e.mem.alloc(0x80);
        r.e.mem.set_u8(second + 0x60, 0x20);
        let queue = Rc::new(RefCell::new(vec![second, first]));
        let advanced = Rc::new(Cell::new(0u32));
        r.e.register_double(0x0082_56d0, {
            let queue = queue.clone();
            move |e, a| match queue.borrow_mut().pop() {
                Some(node) => {
                    e.mem.set_u32(a[0], node);
                    ret(0)
                }
                None => ret(1),
            }
        });
        r.e.register_double(0x0063_f7b0, {
            let advanced = advanced.clone();
            move |_, _| {
                advanced.set(advanced.get() + 1);
                Ret::default()
            }
        });
        r.e.register(0x0063_d040, |_, a| ret(a[0] + 0x1000));
        r.e.register(0x0051_b860, |_, _| 90.0f32.into_ret());
        r.e.register(0x005b_a540, |_, _| ret(0));
        r.e.register(0x00ec_7750, |_, _| ret(1));
        r.e.register(0x0044_ddc0, |_, _| ret(1));
        r.e.register(0x005b_a360, |_, _| ret(1));
        r.e.register(0x0047_4cb0, |_, _| ret(0));
        r.e.register(0x0055_d520, |_, _| ret(0xbeef));
        r.e.call_log = Some(vec![]);
        r.e.call(
            0x0087_e130,
            &args![r.actor, Ptr::<()>::new(0x5555), false, true, false],
        );
        let log = r.e.call_log.clone().unwrap();
        let calls = called(&r.e);
        assert_eq!(advanced.get(), 2);
        // The first node is added to the array through its part (+0x1000).
        let add = log.iter().find(|(a, _)| *a == 0x007c_b2e0).unwrap();
        assert_eq!(r.e.mem.u32(add.1[1]), first + 0x1000);
        // Look IK is initialized once, from the second node: the virtual
        // +0x21c is false so the controller is asked first; the angle is
        // 90 degrees times the factor.
        assert_eq!(
            log.iter().filter(|(a, _)| *a == 0x00c7_de60).count(),
            1,
            "look IK once"
        );
        assert_eq!(args_of(&r.e, 0x004e_4660), vec![r.controller.addr(), 1]);
        assert_eq!(
            args_of(&r.e, 0x0060_7810),
            vec![r.controller.addr(), 1, 45.0f32.to_bits()]
        );
        // Both IK errors name the actor through 0055d520 (00474cb0 is zero).
        let errors: Vec<_> = log.iter().filter(|(a, _)| *a == 0x005b_5e40).collect();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].1, vec![RAGDOLL_ERROR_LOOK_IK, 0xbeef]);
        assert_eq!(errors[1].1, vec![RAGDOLL_ERROR_FOOT_IK, 0xbeef]);
        // "Prime" in the name clears the byte at +0x224 and asks for +0x5c6b40.
        assert_eq!(args_of(&r.e, 0x005c_6b40), vec![r.controller.addr(), 1]);
        assert_eq!(args_of(&r.e, 0x00ec_7750)[1], PRIME_TEXT);
        // The foot IK result 005ba360 is true: the actor is told (008bc300).
        assert_eq!(args_of(&r.e, 0x008b_c300), vec![r.actor.addr()]);
        // The player skips the area call (no world space here either).
        assert!(!calls.contains(&0x00c7_5ad0));
        let position = |addr| calls.iter().position(|a| *a == addr).unwrap();
        assert!(position(0x005e_ee10) < position(0x0093_1ed0));
    }
}

#[cfg(test)]
mod tests_accessors {
    use super::*;

    fn ret(value: u32) -> Ret {
        value.into_ret()
    }

    fn engine(addrs: &[u32]) -> Engine {
        let mut e = Engine::new();
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
        e
    }

    fn slot_target(vtable: u32, offset: u32) -> u32 {
        0x6000_0000 + (vtable & 0x00ff_ffff) + offset
    }

    fn object(e: &mut Engine, size: u32, vtable: u32, offsets: &[u32]) -> Ptr {
        e.map(vtable, 0x1000);
        for offset in offsets {
            let target = slot_target(vtable, *offset);
            e.mem.set_u32(vtable + offset, target);
            e.register(target, |_, _| Ret::default());
        }
        let object = Ptr::new(e.mem.alloc(size));
        e.mem.set_u32(object.addr(), vtable);
        object
    }

    fn called(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(addr, _)| *addr)
            .collect()
    }

    fn args_of(e: &Engine, addr: u32) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .find(|(a, _)| *a == addr)
            .unwrap_or_else(|| panic!("{addr:08x} was not called"))
            .1
            .clone()
    }

    /// An actor whose base form (the result of `004181e0`) is `base`.
    fn actor_with_base(e: &mut Engine, base: Ptr) -> Ptr<Actor> {
        let actor = object(
            e,
            0x1b4,
            0x0200_0000,
            &[0x218, 0x1a0, 0x214, 0x418, 0x48, 0x190, 0x4d4],
        );
        e.register_double(GET_BASE_FORM, move |_, _| ret(base.addr()));
        e.register(GET_EXTRA_DATA_LIST, |_, a| ret(a[0] + 0x44));
        actor.cast()
    }

    // ---- 0087e920 / 0087e940 ----

    #[test]
    fn node_flag_tests_read_bits_4_and_0x20_of_the_byte_at_0x60() {
        let mut e = Engine::new();
        let node = Ptr::<()>::new(e.mem.alloc(0x80));
        assert!(!e.call(0x0087_e920, &args![node]).bool());
        assert!(!e.call(0x0087_e940, &args![node]).bool());
        e.mem.set_u8(node.addr() + 0x60, 0x04);
        assert!(e.call(0x0087_e920, &args![node]).bool());
        assert!(!e.call(0x0087_e940, &args![node]).bool());
        e.mem.set_u8(node.addr() + 0x60, 0x20);
        assert!(!e.call(0x0087_e920, &args![node]).bool());
        assert!(e.call(0x0087_e940, &args![node]).bool());
    }

    // ---- 0087e960 / 0087eeb0 ----

    #[test]
    fn turret_flag_is_written_and_read_at_0x1b0() {
        let mut e = Engine::new();
        let actor: Ptr<Actor> = e.new_object();
        e.call(0x0087_e960, &args![actor, true]);
        assert_eq!(e.mem.u8(actor.addr() + 0x1b0), 1);
        assert_eq!(e.call(0x0087_eeb0, &args![actor]).u8(), 1);
        e.call(0x0087_e960, &args![actor, false]);
        assert_eq!(e.call(0x0087_eeb0, &args![actor]).u8(), 0);
    }

    // ---- 0087e980, 0087e9a0, 0087e9b0 ----

    #[test]
    fn character_controller_word_comes_from_the_helper_at_0x594() {
        let mut e = Engine::new();
        e.register(0x0055_9450, |_, a| ret(a[0] ^ 1));
        let result = e.call(0x0087_e980, &args![Ptr::<()>::new(0x1000)]).u32();
        assert_eq!(result, (0x1000 + 0x594) ^ 1);
    }

    #[test]
    fn fn_0087e9a0_reads_its_global() {
        let mut e = Engine::new();
        e.map(0x011c_6000, 0x1000);
        e.set_global(GLOBAL_VALUE_011C625C, 0x1234u32);
        assert_eq!(e.call(0x0087_e9a0, &args![]).u32(), 0x1234);
    }

    #[test]
    fn fn_0087e9b0_releases_the_block() {
        let mut e = engine(&[FREE_BLOCK]);
        e.call_log = Some(vec![]);
        e.call(0x0087_e9b0, &args![Ptr::<()>::new(0x4444)]);
        assert_eq!(args_of(&e, FREE_BLOCK), vec![0x4444]);
    }

    // ---- ragdoll controller accessors ----

    #[test]
    fn controller_flag_255_needs_flag_254_and_the_argument() {
        let mut e = Engine::new();
        let controller = Ptr::<()>::new(e.mem.alloc(0x300));
        e.call(0x0087_e9d0, &args![controller, true]);
        assert_eq!(e.mem.u8(controller.addr() + 0x255), 0);
        e.mem.set_u8(controller.addr() + 0x254, 1);
        e.call(0x0087_e9d0, &args![controller, true]);
        assert_eq!(e.mem.u8(controller.addr() + 0x255), 1);
        e.call(0x0087_e9d0, &args![controller, false]);
        assert_eq!(e.mem.u8(controller.addr() + 0x255), 0);
    }

    #[test]
    fn controller_setters_and_getters_use_their_offsets() {
        let mut e = Engine::new();
        let controller = Ptr::<()>::new(e.mem.alloc(0x300));
        e.mem.set_u8(controller.addr() + 0x42, 7);
        assert_eq!(e.call(0x0087_ea20, &args![controller]).u8(), 7);
        e.call(0x0087_ea40, &args![controller, 0x99u32]);
        assert_eq!(e.mem.u32(controller.addr() + 0x5c), 0x99);
        e.call(0x0087_ea60, &args![controller, 3u8]);
        assert_eq!(e.mem.u8(controller.addr() + 0x224), 3);
    }

    #[test]
    fn controller_character_proxy_goes_through_the_pointer_operator() {
        let mut e = Engine::new();
        e.register(0x004a_e750, |_, a| {
            ret(if a[0] == 0 { 0 } else { a[0] + 8 })
        });
        let controller = Ptr::<()>::new(e.mem.alloc(0x300));
        e.call(0x0087_ea80, &args![controller, Ptr::<()>::new(0x100)]);
        assert_eq!(e.mem.u32(controller.addr() + 0x90), 0x108);
    }

    #[test]
    fn fn_0087eaa0_is_the_address_0x40_in() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x0087_eaa0, &args![Ptr::<()>::new(0x1000)]).u32(),
            0x1040
        );
    }

    // ---- 0087eed0 ----

    #[test]
    fn package_byte_at_0xbe() {
        let mut e = Engine::new();
        let package = Ptr::<()>::new(e.mem.alloc(0xc0));
        e.mem.set_u8(package.addr() + 0xbe, 5);
        assert_eq!(e.call(0x0087_eed0, &args![package]).u8(), 5);
    }

    // ---- 0087eac0 ----

    const PACKAGE_CALLEES: [u32; 12] = [
        0x0096_11e0,
        0x0056_9b80,
        0x0067_77b0,
        0x008d_8520,
        0x0088_d640,
        0x0067_4d70,
        CLEAR_IN_COMBAT,
        0x0045_34f0,
        0x0049_3bb0,
        0x008b_70d0,
        0x008b_3d30,
        0x008a_8f60,
    ];

    struct Package {
        e: Engine,
        actor: Ptr<Actor>,
        process: Ptr,
        package: Ptr,
    }

    fn package_setup(state: u32, kind: u32) -> Package {
        let mut e = engine(&PACKAGE_CALLEES);
        e.map(0x0101_2000, 0x1000);
        e.map(0x0101_a000, 0x1000);
        e.map(0x011d_e000, 0x1000);
        e.set_global(0x0101_2060, 0.0f64);
        e.set_global(0x0101_a6b0, -1.0f64);
        e.set_global(PLAYER_POINTER, 0x7777u32);
        let actor = actor_with_base(&mut e, Ptr::NULL);
        e.register_double(slot_target(0x0200_0000, 0x214), move |_, _| ret(state));
        let process = object(
            &mut e,
            0x40,
            0x0201_0000,
            &[
                0x1e0, 0x1e4, 0x214, 0x294, 0x628, 0x230, 0x614, 0x210, 0x284, 0x12c, 0x244,
            ],
        );
        e.set(actor, Actor::pCurrentProcess, process);
        e.register(0x0096_11e0, |_, _| ret(5));
        e.register_double(0x0041_ca90, move |_, _| ret(kind));
        let package = Ptr::<()>::new(e.mem.alloc(0xc0));
        e.call_log = Some(vec![]);
        Package {
            e,
            actor,
            process,
            package,
        }
    }

    #[test]
    fn package_change_does_nothing_without_a_process() {
        let mut p = package_setup(0, 5);
        p.e.set(p.actor, Actor::pCurrentProcess, Ptr::NULL);
        p.e.call(0x0087_eac0, &args![p.actor, p.package, false, true]);
        assert_eq!(called(&p.e), vec![0x0087_eac0]);
    }

    #[test]
    fn package_start_adjusts_the_timer_and_clears_combat() {
        let mut p = package_setup(0, 5);
        p.e.register(slot_target(0x0201_0000, 0x1e0), |_, _| 2.0f32.into_ret());
        // The package type is not yet known (-1): it is calculated for the actor.
        p.e.register(0x0096_11e0, |_, _| ret(u32::MAX));
        p.e.register(0x0056_9b80, |_, _| ret(0x4242));
        p.e.register(0x008b_70d0, |_, _| ret(1));
        p.e.call(0x0087_eac0, &args![p.actor, p.package, false, true]);
        let calls = called(&p.e);
        assert_eq!(args_of(&p.e, 0x0067_77b0), vec![p.package.addr(), 0x4242]);
        // State 0 skips the sit/sleep handling; the package is marked created.
        assert!(!calls.contains(&0x0088_d640));
        assert_eq!(args_of(&p.e, 0x0067_4d70), vec![p.package.addr(), 1]);
        // The timer is positive: it is negated and the process told.
        assert_eq!(
            args_of(&p.e, slot_target(0x0201_0000, 0x1e4)),
            vec![p.process.addr(), (-2.0f32).to_bits()]
        );
        assert_eq!(
            args_of(&p.e, slot_target(0x0201_0000, 0x294)),
            vec![p.process.addr(), p.actor.addr()]
        );
        assert_eq!(
            args_of(&p.e, slot_target(0x0201_0000, 0x628)),
            vec![p.process.addr(), 0x7777]
        );
        assert_eq!(args_of(&p.e, CLEAR_IN_COMBAT), vec![p.actor.addr(), 1]);
        assert_eq!(
            args_of(&p.e, slot_target(0x0201_0000, 0x230)),
            vec![p.process.addr(), p.package.addr(), p.actor.addr()]
        );
        // The actor has an animation and state 0: the process gets 0x800.
        assert_eq!(
            args_of(&p.e, slot_target(0x0201_0000, 0x614)),
            vec![p.process.addr(), 0x800]
        );
        // The look-at target is cleared (not a turret, 00493bb0 false).
        assert!(calls.contains(&0x008b_3d30));
        assert_eq!(*calls.last().unwrap(), 0x008a_8f60);
    }

    #[test]
    fn package_start_keeps_combat_for_a_type_0x12_package_and_skips_a_zero_timer() {
        let mut p = package_setup(0, 0x12);
        p.e.register(0x0049_3bb0, |_, _| ret(1));
        p.e.call(0x0087_eac0, &args![p.actor, p.package, false, false]);
        let calls = called(&p.e);
        assert!(!calls.contains(&CLEAR_IN_COMBAT));
        assert!(!calls.contains(&slot_target(0x0201_0000, 0x1e4)));
        assert!(!calls.contains(&0x0067_4d70));
        assert!(!calls.contains(&0x008b_3d30));
        assert!(!calls.contains(&slot_target(0x0201_0000, 0x614)));
    }

    #[test]
    fn package_leave_from_a_furniture_state_returns_early() {
        // State 4: the actor leaves its furniture, and with flag_a set the
        // function ends there.
        let mut p = package_setup(4, 5);
        p.e.call(0x0087_eac0, &args![p.actor, p.package, true, true]);
        let calls = called(&p.e);
        assert!(calls.contains(&slot_target(0x0200_0000, 0x418)));
        assert!(!calls.contains(&0x008a_8f60));
        assert!(!calls.contains(&slot_target(0x0201_0000, 0x210)));
    }

    #[test]
    fn package_leave_tells_the_process_unless_the_package_is_type_0x1c_with_its_flag() {
        let mut p = package_setup(0, 0x1c);
        p.e.mem.set_u8(p.package.addr() + 0xbe, 1);
        p.e.call(0x0087_eac0, &args![p.actor, p.package, true, false]);
        let calls = called(&p.e);
        assert_eq!(
            args_of(&p.e, slot_target(0x0201_0000, 0x210)),
            vec![p.process.addr(), p.package.addr(), p.actor.addr()]
        );
        assert_eq!(
            args_of(&p.e, slot_target(0x0201_0000, 0x284)),
            vec![p.process.addr(), 0]
        );
        assert!(!calls.contains(&slot_target(0x0201_0000, 0x294)));
        assert_eq!(
            args_of(&p.e, slot_target(0x0200_0000, 0x48)),
            vec![p.actor.addr(), 0]
        );
        assert!(calls.contains(&slot_target(0x0201_0000, 0x12c)));
        assert_eq!(*calls.last().unwrap(), 0x008a_8f60);
    }

    #[test]
    fn package_furniture_check_uses_the_acquire_object_when_not_sitting() {
        // State 2 and a non-null package kind that is not special: the
        // acquire object's virtual +0x4d4 decides; its byte at +0xe above
        // 0x14 sends the actor out of the furniture with the virtual +0x418.
        let mut p = package_setup(2, 5);
        let acquire = object(&mut p.e, 0x20, 0x0220_0000, &[0x4d4]);
        let object_row = Ptr::<()>::new(p.e.mem.alloc(0x20));
        p.e.mem.set_u8(object_row.addr() + 0xe, 0x15);
        let row = object_row.addr();
        p.e.register_double(slot_target(0x0220_0000, 0x4d4), move |_, _| ret(row));
        let acquire_address = acquire.addr();
        p.e.register_double(0x008d_8520, move |_, _| ret(acquire_address));
        p.e.call(0x0087_eac0, &args![p.actor, p.package, false, false]);
        let calls = called(&p.e);
        assert!(calls.contains(&slot_target(0x0200_0000, 0x418)));
        assert!(!calls.contains(&0x0088_d640));
        // With a low byte the actor is taken out of furniture quickly instead.
        let mut p = package_setup(2, 5);
        let acquire = object(&mut p.e, 0x20, 0x0220_0000, &[0x4d4]);
        let object_row = Ptr::<()>::new(p.e.mem.alloc(0x20));
        p.e.mem.set_u8(object_row.addr() + 0xe, 0x10);
        let row = object_row.addr();
        p.e.register_double(slot_target(0x0220_0000, 0x4d4), move |_, _| ret(row));
        let acquire_address = acquire.addr();
        p.e.register_double(0x008d_8520, move |_, _| ret(acquire_address));
        p.e.call(0x0087_eac0, &args![p.actor, p.package, false, false]);
        let calls = called(&p.e);
        assert!(calls.contains(&0x0088_d640));
        assert!(!calls.contains(&slot_target(0x0200_0000, 0x418)));
    }

    // ---- 0087eef0 ----

    #[test]
    fn default_inventory_without_a_source_only_updates_the_inventory_changes() {
        let mut e = engine(&[0x0044_1110, 0x004d_1960]);
        let base = Ptr::new(e.mem.alloc(0x40));
        let actor = actor_with_base(&mut e, base);
        e.register(0x0041_8520, |_, _| ret(0x6600));
        e.call_log = Some(vec![]);
        e.call(0x0087_eef0, &args![actor]);
        assert_eq!(
            called(&e),
            vec![
                0x0087_eef0,
                GET_BASE_FORM,
                0x0044_1110,
                GET_EXTRA_DATA_LIST,
                0x0041_8520,
                0x004d_1960
            ]
        );
        assert_eq!(args_of(&e, 0x004d_1960), vec![0x6600]);
    }

    #[test]
    fn default_inventory_adds_each_entry_with_its_count_and_extra_data() {
        let mut e = engine(&[
            0x0048_1610,
            0x0048_7f70,
            0x0048_1680,
            0x0041_9ad0,
            0x0041_9ed0,
            0x0041_9f80,
            0x0040_ea20,
            0x004d_1960,
            0x0087_f9f0,
        ]);
        let base = Ptr::<()>::new(e.mem.alloc(0x80));
        let actor = actor_with_base(&mut e, base);
        e.register(0x0044_1110, |_, _| ret(0x2000));
        e.register(0x0041_8520, |_, _| ret(0));
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(EXTRA_DATA_LIST_CONSTRUCTOR, |_, a| ret(a[0]));
        e.register(0x0068_15c0, |_, a| ret(a[0]));
        // Three list nodes (the first word of a node is its entry pointer);
        // entry layout: count, item, health description.
        let item = object(&mut e, 0x20, 0x0230_0000, &[0xe4]);
        e.register(slot_target(0x0230_0000, 0xe4), |_, _| ret(1));
        let plain = e.mem.alloc(0x10);
        let health = e.mem.alloc(0x10);
        let scripted = e.mem.alloc(0x10);
        let nodes: Vec<u32> = (0..3).map(|_| e.mem.alloc(0x10)).collect();
        e.mem.set_u32(nodes[0], plain);
        e.mem.set_u32(nodes[1], health);
        e.mem.set_u32(nodes[2], scripted);
        for (entry, count, description) in
            [(plain, 3u32, 0u32), (health, 2, 0x9000), (scripted, 2, 0)]
        {
            e.mem.set_u32(entry, count);
            e.mem.set_u32(entry + 4, item.addr());
            e.mem.set_u32(entry + 8, description);
        }
        // The list: node i -> node i+1; the container snapshot starts at nodes[0].
        let table = nodes.clone();
        e.register_double(0x0071_7e50, move |_, _| ret(table[0]));
        let next = nodes.clone();
        e.register_double(0x0072_6070, move |_, a| {
            let at = next.iter().position(|n| *n == a[0]).unwrap();
            ret(next.get(at + 1).copied().unwrap_or(0))
        });
        // Only the third entry's item has a script.
        let scripted_entry = scripted;
        let _ = scripted_entry;
        let calls_to_script = std::rc::Rc::new(std::cell::Cell::new(0));
        let counter = calls_to_script.clone();
        e.register_double(0x0048_26d0, move |_, _| {
            counter.set(counter.get() + 1);
            // The first two entries have no script, the third has one.
            ret(if counter.get() == 3 { 0x5c5c } else { 0 })
        });
        e.register(0x0041_8800, |_, _| ret(0));
        e.register(0x005a_bf60, |_, _| ret(0x7a7a));
        e.register(0x0048_72e0, |_, _| ret(0x4e4e));
        e.call_log = Some(vec![]);
        e.call(0x0087_eef0, &args![actor]);
        let log = e.call_log.clone().unwrap();
        let adds: Vec<_> = log
            .iter()
            .filter(|(a, _)| *a == slot_target(0x0200_0000, 0x190))
            .map(|(_, args)| args.clone())
            .collect();
        // plain: no extra data, count 3; health: extra data with count 2;
        // scripted: two extra data lists with count 1 each.
        assert_eq!(adds.len(), 4);
        assert_eq!(adds[0][2..], [0, 3]);
        assert_ne!(adds[1][2], 0);
        assert_eq!(adds[1][3], 2);
        assert_eq!(adds[2][3], 1);
        assert_eq!(adds[3][3], 1);
        // The health entry's description is applied with the item's health form.
        let health_call = log.iter().find(|(a, _)| *a == 0x0040_ea20).unwrap();
        assert_eq!((health_call.1[0], health_call.1[2]), (0x9000, 0x4e4e));
        // Scripted entries get the script attached to each extra data list.
        let set_script: Vec<_> = log.iter().filter(|(a, _)| *a == 0x0041_9ed0).collect();
        assert_eq!(set_script.len(), 2);
        assert_eq!(set_script[0].1[1], 0x5c5c);
        assert_eq!(args_of(&e, 0x0041_9ad0)[1], 2); // the health entry's SetCount(2)
    }

    // ---- the base form predicates ----

    #[test]
    fn fn_0087f200_is_false_when_either_check_says_yes() {
        for (virtual_says, helper_says, expected) in [
            (false, false, true),
            (true, false, false),
            (false, true, false),
        ] {
            let mut e = engine(&[]);
            let base = object(&mut e, 0x80, 0x0240_0000, &[0x44]);
            // The virtual is on the component at base + 0x30.
            e.mem.set_u32(base.addr() + 0x30, 0x0241_0000);
            e.map(0x0241_0000, 0x1000);
            e.mem.set_u32(0x0241_0000 + 0x44, 0x7000_0044);
            e.register_double(0x7000_0044, move |_, _| ret(virtual_says as u32));
            let actor = actor_with_base(&mut e, base);
            e.register_double(0x008a_ce90, move |_, _| ret(helper_says as u32));
            e.call_log = Some(vec![]);
            assert_eq!(e.call(0x0087_f200, &args![actor]).bool(), expected);
            // The helper is only asked when the virtual says no.
            assert_eq!(called(&e).contains(&0x008a_ce90), !virtual_says);
        }
    }

    fn essential_engine(base_form_value: u32) -> (Engine, Ptr<Actor>) {
        let mut e = engine(&[]);
        let actor = actor_with_base(&mut e, Ptr::new(base_form_value));
        e.map(0x011d_e000, 0x1000);
        e.set_global(PLAYER_POINTER, 0);
        e.register(0x0042_16f0, |_, _| ret(0));
        e.register(0x0056_6950, |_, _| ret(0));
        e.register(0x004d_1360, |_, _| ret(0));
        e.register(0x008c_1b30, |_, _| ret(0));
        e.register(0x0046_1580, |e, a| {
            ret(((e.mem.u32(a[0] + 4) & a[1]) != 0) as u32)
        });
        (e, actor)
    }

    #[test]
    fn essential_is_false_without_a_base_form() {
        let (mut e, actor) = essential_engine(0);
        assert!(!e.call(0x0087_f3d0, &args![actor]).bool());
    }

    #[test]
    fn essential_follows_the_base_flag_or_the_actor_check() {
        let (mut e, actor) = essential_engine(0);
        let base = e.mem.alloc(0x80);
        e.register_double(GET_BASE_FORM, move |_, _| ret(base));
        assert!(!e.call(0x0087_f3d0, &args![actor]).bool());
        // Bit 2 of the word at +4 of the component at base + 0x30.
        e.mem.set_u32(base + 0x34, 2);
        assert!(e.call(0x0087_f3d0, &args![actor]).bool());
        e.mem.set_u32(base + 0x34, 0);
        e.register(0x008c_1b30, |_, _| ret(1));
        assert!(e.call(0x0087_f3d0, &args![actor]).bool());
    }

    #[test]
    fn essential_prefers_the_original_leveled_base() {
        let (mut e, actor) = essential_engine(0);
        let original = e.mem.alloc(0x80);
        e.mem.set_u32(original + 0x34, 2);
        e.register_double(0x0042_16f0, move |_, _| ret(original));
        assert!(e.call(0x0087_f3d0, &args![actor]).bool());
    }

    #[test]
    fn essential_is_true_while_the_player_forces_the_next_update() {
        let (mut e, actor) = essential_engine(0);
        let base = e.mem.alloc(0x80);
        e.register_double(GET_BASE_FORM, move |_, _| ret(base));
        e.set_global(PLAYER_POINTER, 0x7777u32);
        e.register(0x0056_6950, |_, _| ret(1));
        assert!(e.call(0x0087_f3d0, &args![actor]).bool());
        // Unless the player check says otherwise.
        e.register(0x004d_1360, |_, _| ret(1));
        assert!(!e.call(0x0087_f3d0, &args![actor]).bool());
    }

    #[test]
    fn fn_0087f260_checks_creature_state_settings_idle_and_base_virtuals() {
        // Plain actor: nothing blocks.
        let mut e = engine(&[]);
        let base = object(&mut e, 0x80, 0x0240_0000, &[]);
        let component = 0x0241_0000u32;
        e.mem.set_u32(base.addr() + 0x30, component);
        e.map(component, 0x1000);
        e.mem.set_u32(component + 0x50, 0x7000_0050);
        e.mem.set_u32(component + 0x48, 0x7000_0048);
        e.register(0x7000_0050, |_, _| ret(1));
        e.register(0x7000_0048, |_, _| ret(0));
        let actor = actor_with_base(&mut e, base);
        e.register(RT_DYNAMIC_CAST, |_, _| ret(0));
        e.register(0x005f_bf20, |_, _| ret(0));
        e.register(0x005a_29b0, |_, _| ret(0));
        e.map(0x011d_f000, 0x1000);
        e.register(SETTING_VALUE_POINTER, |_, _| ret(0x011d_f800));
        e.call_log = Some(vec![]);
        assert!(e.call(0x0087_f260, &args![actor]).bool());
        assert_eq!(
            args_of(&e, RT_DYNAMIC_CAST),
            vec![base.addr(), 0, 0x0118_46e8, 0x0118_3a00, 0]
        );
        // A last idle forbids it.
        e.register(0x005a_29b0, |_, _| ret(1));
        assert!(!e.call(0x0087_f260, &args![actor]).bool());
        e.register(0x005a_29b0, |_, _| ret(0));
        // Both base virtuals true forbid it.
        e.register(0x7000_0048, |_, _| ret(1));
        assert!(!e.call(0x0087_f260, &args![actor]).bool());
        e.register(0x7000_0048, |_, _| ret(0));
        // A humanoid creature with the setting on forbids it; with the
        // setting off, the actor's virtual +0x1a0(0) decides.
        e.register(RT_DYNAMIC_CAST, |_, _| ret(0x5000));
        e.register(0x005f_bf20, |_, _| ret(1));
        assert!(e.call(0x0087_f260, &args![actor]).bool());
        e.register(slot_target(0x0200_0000, 0x1a0), |_, _| ret(1));
        assert!(!e.call(0x0087_f260, &args![actor]).bool());
        e.register(slot_target(0x0200_0000, 0x1a0), |_, _| ret(0));
        e.mem.set_u8(0x011d_f800, 1);
        assert!(!e.call(0x0087_f260, &args![actor]).bool());
    }

    #[test]
    fn creature_actor_value_wrappers_forward_to_the_base_form_predicates() {
        for (address, helper) in [
            (0x0087_f350u32, 0x005f_0b50u32),
            (0x0087_f370, 0x005f_0bf0),
            (0x0087_f390, 0x005f_0c40),
            (0x0087_f3b0, 0x005f_0ba0),
        ] {
            let mut e = engine(&[]);
            let actor = actor_with_base(&mut e, Ptr::new(0x3300));
            e.register(helper, |_, a| ret((a[0] == 0x3300) as u32));
            assert!(e.call(address, &args![actor]).bool(), "{address:08x}");
            let other = actor_with_base(&mut e, Ptr::new(0x3301));
            assert!(!e.call(address, &args![other]).bool());
        }
    }

    #[test]
    fn fn_0087f480_tests_form_flag_bit_2() {
        let mut e = Engine::new();
        e.register(0x0046_1580, |_, a| ret((a[1] == 2) as u32));
        assert!(e.call(0x0087_f480, &args![Ptr::<()>::new(0x100)]).bool());
    }

    #[test]
    fn fn_0087f4a0_asks_the_component_at_0x30() {
        let mut e = engine(&[]);
        let actor = actor_with_base(&mut e, Ptr::new(0x2000));
        e.register(0x0056_af00, |_, a| ret((a[0] == 0x2030) as u32));
        assert!(e.call(0x0087_f4a0, &args![actor]).bool());
    }

    #[test]
    fn fn_0087f4c0_returns_the_sex_of_the_base() {
        let mut e = engine(&[]);
        let actor = actor_with_base(&mut e, Ptr::new(0x2000));
        e.register(0x005f_0cc0, |_, _| ret(u32::MAX));
        assert_eq!(e.call(0x0087_f4c0, &args![actor]).i32(), -1);
        e.register(0x005f_0cc0, |_, _| ret(1));
        assert_eq!(e.call(0x0087_f4c0, &args![actor]).i32(), 1);
    }

    #[test]
    fn fn_0087f4e0_calls_the_base_forms_virtual_0x144() {
        let mut e = engine(&[]);
        let base = object(&mut e, 0x40, 0x0240_0000, &[0x144]);
        e.register(slot_target(0x0240_0000, 0x144), |_, a| ret(a[0] + 1));
        let actor = actor_with_base(&mut e, base);
        assert_eq!(e.call(0x0087_f4e0, &args![actor]).u32(), base.addr() + 1);
    }

    #[test]
    fn fn_0087f510_uses_the_extra_data_or_the_component() {
        let mut e = engine(&[]);
        let actor = actor_with_base(&mut e, Ptr::new(0x2000));
        // Virtual +0x218 false: the answer is 1.
        assert_eq!(e.call(0x0087_f510, &args![actor]).u8(), 1);
        e.register(slot_target(0x0200_0000, 0x218), |_, _| ret(1));
        // Without the extra data, 005d2780 on the component at base + 0x30.
        e.register(0x0041_0220, |_, _| ret(0));
        e.register(0x005d_2780, |_, a| ret((a[0] == 0x2030) as u32 * 7));
        assert_eq!(e.call(0x0087_f510, &args![actor]).u8(), 7);
        // With it, the byte at +0xc.
        let extra = e.mem.alloc(0x10);
        e.mem.set_u8(extra + 0xc, 9);
        e.register_double(0x0041_0220, move |_, _| ret(extra));
        assert_eq!(e.call(0x0087_f510, &args![actor]).u8(), 9);
    }

    #[test]
    fn swims_only_and_its_users() {
        let mut e = engine(&[]);
        let actor = actor_with_base(&mut e, Ptr::new(0x2000));
        // 0087f370 false, 0087f350 true, 0087f390 false: swims only.
        e.register(0x005f_0bf0, |_, _| ret(0));
        e.register(0x005f_0b50, |_, _| ret(1));
        e.register(0x005f_0c40, |_, _| ret(0));
        assert!(e.call(0x0087_f570, &args![actor]).bool());
        // A flying creature does not (0087f390 true).
        e.register(0x005f_0c40, |_, _| ret(1));
        assert!(!e.call(0x0087_f570, &args![actor]).bool());
        e.register(0x005f_0c40, |_, _| ret(0));
        // 0087f620: false only for a (virtual +0x218 false) swimmer.
        assert!(!e.call(0x0087_f620, &args![actor]).bool());
        e.register(slot_target(0x0200_0000, 0x218), |_, _| ret(1));
        assert!(e.call(0x0087_f620, &args![actor]).bool());
    }

    #[test]
    fn fn_0087f5c0_falls_back_to_the_actor_value() {
        let mut e = engine(&[]);
        let actor = actor_with_base(&mut e, Ptr::new(0x2000));
        e.register(0x005f_0bf0, |_, _| ret(0));
        e.register(0x005f_0b50, |_, _| ret(1));
        e.register(0x005f_0c40, |_, _| ret(0));
        // Virtual +0x218 false and a swimmer: true without asking the owner.
        assert!(e.call(0x0087_f5c0, &args![actor]).bool());
        // Otherwise the actor-value owner at +0xa4 (virtual +8, argument 0x35).
        e.register(slot_target(0x0200_0000, 0x218), |_, _| ret(1));
        e.map(0x0250_0000, 0x1000);
        e.mem.set_u32(actor.addr() + 0xa4, 0x0250_0000);
        e.mem.set_u32(0x0250_0000 + 8, 0x7000_0008);
        let owner = actor.addr() + 0xa4;
        e.register_double(0x7000_0008, move |_, a| {
            ret((a[0] == owner && a[1] == 0x35) as u32)
        });
        assert!(e.call(0x0087_f5c0, &args![actor]).bool());
    }
}

#[cfg(test)]
mod tests_block2 {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The argument words of each call a double saw.
    type Log = Rc<RefCell<Vec<Vec<u32>>>>;

    const VTABLE: u32 = 0x0200_0000;
    const OWNER_VTABLE: u32 = 0x0201_0000;
    const PROCESS_VTABLE: u32 = 0x0202_0000;

    /// An engine with the pages of the globals the tests set mapped.
    fn new_engine() -> Engine {
        let mut e = Engine::new();
        for page in [0x0101_2000, 0x0101_7000, 0x011d_e000] {
            e.map(page, 0x1000);
        }
        e
    }

    fn ret(value: u32) -> Ret {
        value.into_ret()
    }

    fn slot(vtable: u32, offset: u32) -> u32 {
        0x6000_0000 + (vtable & 0x00ff_ffff) + offset
    }

    /// A double over `addr` that returns `value` and logs the argument words.
    fn record(e: &mut Engine, addr: u32, value: Ret) -> Log {
        let log: Log = Rc::new(RefCell::new(vec![]));
        let seen = log.clone();
        e.register_double(addr, move |_, a| {
            seen.borrow_mut().push(a.to_vec());
            value
        });
        log
    }

    /// Doubles returning zero over every address.
    fn stub(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// An object of `size` bytes whose vtable (at `vtable`) has the listed
    /// slots, each a double returning zero.
    fn object(e: &mut Engine, size: u32, vtable: u32, offsets: &[u32]) -> Ptr {
        e.map(vtable, 0x1000);
        for offset in offsets {
            let target = slot(vtable, *offset);
            e.mem.set_u32(vtable + offset, target);
            e.register(target, |_, _| Ret::default());
        }
        let object = Ptr::new(e.mem.alloc(size));
        e.mem.set_u32(object.addr(), vtable);
        object
    }

    /// An actor whose vtable has the listed slots.
    fn actor(e: &mut Engine, slots: &[u32]) -> Ptr<Actor> {
        object(e, 0x1b4, VTABLE, slots).cast()
    }

    /// Gives the actor a process with the listed virtual slots.
    fn give_process(e: &mut Engine, actor: Ptr<Actor>, slots: &[u32]) -> Ptr {
        let process = object(e, 0x400, PROCESS_VTABLE, slots);
        e.set(actor, Actor::pCurrentProcess, process);
        e.register(ACTOR_PROCESS, |e, a| ret(e.mem.u32(a[0] + 0x68)));
        process
    }

    /// Gives the actor an actor-value owner at `+0xa4` with the listed slots.
    fn give_owner(e: &mut Engine, actor: Ptr<Actor>, slots: &[u32]) -> u32 {
        e.map(OWNER_VTABLE, 0x1000);
        for offset in slots {
            let target = slot(OWNER_VTABLE, *offset);
            e.mem.set_u32(OWNER_VTABLE + offset, target);
            e.register(target, |_, _| Ret::default());
        }
        e.mem.set_u32(actor.addr() + 0xa4, OWNER_VTABLE);
        actor.addr() + 0xa4
    }

    /// Makes `base` the actor's base form.
    fn give_base(e: &mut Engine, base: u32) {
        e.register_double(GET_BASE_FORM, move |_, _| ret(base));
    }

    /// `_ftol2_sse`: truncation of the `f64` in the first two words.
    fn install_float_to_int(e: &mut Engine) {
        e.register(FLOAT_TO_INT, |_, a| {
            let value = f64::from_bits(a[0] as u64 | (a[1] as u64) << 32);
            ret(value as i32 as u32)
        });
    }

    /// The disposition-modifier list of `actor` with the given entries
    /// `(amount, target)`; returns the item addresses. Installs the two node
    /// helpers.
    fn modifier_list(e: &mut Engine, actor: Ptr<Actor>, entries: &[(i32, u32)]) -> Vec<u32> {
        e.register(LIST_NODE_ITEM_SLOT, |_, a| ret(a[0]));
        e.register(LIST_NODE_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        let mut items = vec![];
        let mut node = actor.addr() + 0xfc;
        for (i, (amount, target)) in entries.iter().enumerate() {
            let item = e.mem.alloc(8);
            e.mem.set_i32(item, *amount);
            e.mem.set_u32(item + 4, *target);
            e.mem.set_u32(node, item);
            if i + 1 < entries.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
            items.push(item);
        }
        items
    }

    // ---- 0087f660 ----

    #[test]
    fn check_breath_timer_is_false_only_when_the_swim_time_exceeds_the_timer() {
        let mut e = new_engine();
        install_float_to_int(&mut e);
        let actor = actor(&mut e, &[]);
        e.register(0x008b_e7a0, |_, _| 7.9f32.into_ret());
        // 00648a10 gives twice the integer level: 14.0.
        e.register(0x0064_8a10, |_, a| ((a[0] as i32) as f32 * 2.0).into_ret());
        assert!(e.call(0x0087_f660, &args![actor]).bool());
        give_process(&mut e, actor, &[0x300]);
        e.register(slot(PROCESS_VTABLE, 0x300), |_, _| 20.0f32.into_ret());
        assert!(e.call(0x0087_f660, &args![actor]).bool());
        e.register(slot(PROCESS_VTABLE, 0x300), |_, _| 10.0f32.into_ret());
        assert!(!e.call(0x0087_f660, &args![actor]).bool());
    }

    // ---- 0087f6c0 ----

    #[test]
    fn location_name_needs_a_form_a_record_and_a_passing_check() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[0x218]);
        e.register(GET_FORM, |_, _| ret(0x5000));
        e.register(0x004a_c110, |_, _| ret(0x6000));
        e.register(0x0048_cee0, |_, _| ret(1));
        let name = record(&mut e, 0x0040_8da0, ret(0x777));
        assert_eq!(e.call(0x0087_f6c0, &args![actor]).u32(), 0);
        e.register(slot(VTABLE, 0x218), |_, _| ret(1));
        assert_eq!(e.call(0x0087_f6c0, &args![actor]).u32(), 0x777);
        assert_eq!(name.borrow()[0], vec![0x6018]);
        e.register(0x0048_cee0, |_, _| ret(0));
        assert_eq!(e.call(0x0087_f6c0, &args![actor]).u32(), 0);
        e.register(0x004a_c110, |_, _| ret(0));
        assert_eq!(e.call(0x0087_f6c0, &args![actor]).u32(), 0);
    }

    // ---- 0087f750 / 0087f7a0 ----

    #[test]
    fn editor_location_form_getters_check_the_form_type() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        e.register(FORM_TYPE, |_, _| ret(0x41));
        e.register(FLAG_BIT_0_AT_0X24, |_, _| ret(1));
        assert_eq!(e.call(0x0087_f750, &args![actor]).u32(), 0);
        e.set(actor, Actor::pEditorLocForm, Ptr::<()>::new(0x4444));
        assert_eq!(e.call(0x0087_f750, &args![actor]).u32(), 0x4444);
        assert_eq!(e.call(0x0087_f7a0, &args![actor]).u32(), 0);
        e.register(FORM_TYPE, |_, _| ret(0x39));
        assert_eq!(e.call(0x0087_f750, &args![actor]).u32(), 0);
        assert_eq!(e.call(0x0087_f7a0, &args![actor]).u32(), 0x4444);
        e.register(FLAG_BIT_0_AT_0X24, |_, _| ret(0));
        assert_eq!(e.call(0x0087_f7a0, &args![actor]).u32(), 0);
    }

    // ---- 0087f800 ----

    #[test]
    fn editor_location_is_taken_from_the_actor() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[0x2bc]);
        let position = e.mem.alloc(12);
        for i in 0..3 {
            e.mem.set_f32(position + 4 * i, 1.0 + i as f32);
        }
        e.register_double(0x0043_6aa0, move |_, _| ret(position));
        let rotation = record(&mut e, slot(VTABLE, 0x2bc), 2.5f32.into_ret());
        e.register(0x008d_6f30, |_, _| ret(0x9000));
        e.register(FLAG_BIT_0_AT_0X24, |_, _| ret(1));
        e.register(0x0057_5d70, |_, _| ret(0x9100));
        e.call(0x0087_f800, &args![actor]);
        assert_eq!(e.mem.f32(actor.addr() + 0x160 + 8), 3.0);
        assert_eq!(e.get(actor, Actor::fEditorLocZRot), 2.5);
        assert_eq!(e.get(actor, Actor::pEditorLocForm).addr(), 0x9000);
        assert_eq!(rotation.borrow()[0], vec![actor.addr(), 0]);
        // The candidate fails the check: the world space form is used.
        e.register(FLAG_BIT_0_AT_0X24, |_, _| ret(0));
        e.call(0x0087_f800, &args![actor]);
        assert_eq!(e.get(actor, Actor::pEditorLocForm).addr(), 0x9100);
        // No candidate at all.
        e.register(0x008d_6f30, |_, _| ret(0));
        e.set(actor, Actor::pEditorLocForm, Ptr::<()>::new(1));
        e.call(0x0087_f800, &args![actor]);
        assert_eq!(e.get(actor, Actor::pEditorLocForm).addr(), 0x9100);
    }

    // ---- 0087f890 ----

    #[test]
    fn editor_location_is_set_from_the_arguments() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        let position = e.mem.alloc(12);
        for i in 0..3 {
            e.mem.set_f32(position + 4 * i, 4.0 + i as f32);
        }
        e.register(FLAG_BIT_0_AT_0X24, |_, _| ret(1));
        let words = args![actor, 0x1000u32, 0x2000u32, position, 0.5f32];
        e.call(0x0087_f890, &words);
        assert_eq!(e.mem.f32(actor.addr() + 0x160 + 8), 6.0);
        assert_eq!(e.get(actor, Actor::fEditorLocZRot), 0.5);
        assert_eq!(e.get(actor, Actor::pEditorLocForm).addr(), 0x2000);
        e.register(FLAG_BIT_0_AT_0X24, |_, _| ret(0));
        e.call(0x0087_f890, &words);
        assert_eq!(e.get(actor, Actor::pEditorLocForm).addr(), 0x1000);
        e.set(actor, Actor::pEditorLocForm, Ptr::<()>::new(1));
        let words = args![actor, 0x1000u32, 0u32, position, 0.5f32];
        e.call(0x0087_f890, &words);
        assert_eq!(e.get(actor, Actor::pEditorLocForm).addr(), 0x1000);
    }

    // ---- 0087f900 / 0087f920 ----

    #[test]
    fn editor_location_can_be_tested_and_read_back() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        let out = e.mem.alloc(0x20);
        let words = args![actor, out, out + 0x10, out + 0x1c, 0u32];
        assert!(!e.call(0x0087_f900, &args![actor]).bool());
        assert!(!e.call(0x0087_f920, &words).bool());
        e.set(actor, Actor::pEditorLocForm, Ptr::<()>::new(0x4444));
        e.set(actor, Actor::fEditorLocZRot, 3.0f32);
        for i in 0..3 {
            e.mem.set_f32(actor.addr() + 0x160 + 4 * i, 1.0 + i as f32);
        }
        assert!(e.call(0x0087_f900, &args![actor]).bool());
        assert!(e.call(0x0087_f920, &words).bool());
        assert_eq!(e.mem.f32(out + 8), 3.0);
        let rotation = (
            e.mem.f32(out + 0x10),
            e.mem.f32(out + 0x14),
            e.mem.f32(out + 0x18),
        );
        assert_eq!(rotation, (0.0, 0.0, 3.0));
        assert_eq!(e.mem.u32(out + 0x1c), 0x4444);
    }

    // ---- 0087f990 / 0087f9f0 ----

    #[test]
    fn base_form_component_predicates() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        give_base(&mut e, 0x3000);
        let first = record(&mut e, 0x005b_b4d0, ret(1));
        let second = record(&mut e, 0x0047_ded0, ret(0x1234));
        assert!(e.call(0x0087_f990, &args![actor]).bool());
        assert_eq!(first.borrow()[0], vec![0x3090]);
        assert_eq!(e.call(0x0087_f9f0, &args![actor]).u32(), 0x1234);
        assert_eq!(second.borrow()[0], vec![0x3030]);
    }

    // ---- 0087f9c0 ----

    #[test]
    fn owner_value_for_actor_value_0x15_is_returned_as_a_float() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        give_owner(&mut e, actor, &[0]);
        let seen = record(&mut e, slot(OWNER_VTABLE, 0), ret((-3i32) as u32));
        assert_eq!(e.call(0x0087_f9c0, &args![actor]).f32(), -3.0);
        assert_eq!(seen.borrow()[0], vec![actor.addr() + 0xa4, 0x15]);
    }

    // ---- 0087fa10 / 0087fa60 / 0087faa0 ----

    #[test]
    fn process_forwarders_need_a_process() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        e.call(0x0087_fa10, &args![actor, true]);
        e.call(0x0087_fa60, &args![actor, true]);
        e.call(0x0087_faa0, &args![actor]);
        let process = give_process(&mut e, actor, &[0x118, 0x120, 0x294, 0x4a0]);
        let first = record(&mut e, slot(PROCESS_VTABLE, 0x118), Ret::default());
        let second = record(&mut e, slot(PROCESS_VTABLE, 0x4a0), Ret::default());
        let third = record(&mut e, slot(PROCESS_VTABLE, 0x120), Ret::default());
        let fourth = record(&mut e, slot(PROCESS_VTABLE, 0x294), Ret::default());
        e.call(0x0087_fa10, &args![actor, true]);
        e.call(0x0087_fa60, &args![actor, false]);
        e.call(0x0087_faa0, &args![actor]);
        assert_eq!(first.borrow()[0], vec![process.addr(), 1]);
        assert_eq!(second.borrow()[0], vec![process.addr()]);
        assert_eq!(third.borrow()[0], vec![process.addr(), 0]);
        assert_eq!(fourth.borrow()[0], vec![process.addr(), actor.addr()]);
    }

    // ---- 0087fad0 ----

    #[test]
    fn removing_a_disposition_modifier_hands_the_item_to_the_list_remover() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        let items = modifier_list(&mut e, actor, &[(5, 0x100), (6, 0x200)]);
        let seen: Rc<RefCell<Vec<(u32, u32)>>> = Rc::new(RefCell::new(vec![]));
        let log = seen.clone();
        e.register_double(LIST_REMOVE_ITEM, move |e, a| {
            log.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        e.call(0x0087_fad0, &args![actor, 0x200u32]);
        assert_eq!(*seen.borrow(), vec![(actor.addr() + 0xfc, items[1])]);
        e.call(0x0087_fad0, &args![actor, 0x300u32]);
        assert_eq!(seen.borrow().len(), 1);
    }

    // ---- 0087fb40 ----

    #[test]
    fn disposition_modifier_is_added_for_the_player_and_limited_to_0_100() {
        let mut e = new_engine();
        install_float_to_int(&mut e);
        e.set_global(0x0101_7a40, 100.0f64);
        e.set_global(PLAYER_POINTER, 0x7000u32);
        let actor = actor(&mut e, &[0x48, 0x344]);
        let flags = record(&mut e, slot(VTABLE, 0x48), Ret::default());
        // The target is not the player: nothing happens.
        e.call(0x0087_fb40, &args![actor, 0x7100u32, 5.0f32]);
        assert!(flags.borrow().is_empty());
        // No entry yet: a new one is allocated and appended.
        modifier_list(&mut e, actor, &[]);
        let block = e.mem.alloc(8);
        e.register_double(OPERATOR_NEW, move |_, _| ret(block));
        let hold = record(&mut e, 0x0056_4db0, Ret::default());
        let appended: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(vec![]));
        let log = appended.clone();
        e.register_double(LIST_APPEND_ITEM, move |e, a| {
            log.borrow_mut().push(e.mem.u32(a[1]));
            Ret::default()
        });
        e.call(0x0087_fb40, &args![actor, 0x7000u32, 12.75f32]);
        assert_eq!(flags.borrow()[0], vec![actor.addr(), 0x80000]);
        assert_eq!(hold.borrow()[0], vec![0x7000, 1]);
        assert_eq!(*appended.borrow(), vec![block]);
        assert_eq!((e.mem.i32(block), e.mem.u32(block + 4)), (12, 0x7000));
        // An entry exists (amount 10): the amount is limited by the disposition.
        let items = modifier_list(&mut e, actor, &[(10, 0x7000)]);
        // (disposition, amount, expected entry): 60 - 80 < 0 turns the amount
        // into -60; within range adds it; above 100 keeps only the part up to
        // 100; already at 100 adds 0.
        let cases = [
            (60i32, -80.0f32, -50),
            (60, 30.0, 40),
            (90, 30.0, 20),
            (100, 30.0, 10),
        ];
        for (disposition, amount, expected) in cases {
            e.mem.set_i32(items[0], 10);
            e.register_double(slot(VTABLE, 0x344), move |_, _| ret(disposition as u32));
            e.call(0x0087_fb40, &args![actor, 0x7000u32, amount]);
            assert_eq!(e.mem.i32(items[0]), expected, "{disposition} {amount}");
        }
    }

    // ---- 0087fcb0 ----

    #[test]
    fn disposition_modifier_toward_an_actor_is_its_amount_or_zero() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        modifier_list(&mut e, actor, &[(5, 0x100), (-7, 0x200)]);
        assert_eq!(e.call(0x0087_fcb0, &args![actor, 0x200u32]).f32(), -7.0);
        assert_eq!(e.call(0x0087_fcb0, &args![actor, 0x100u32]).f32(), 5.0);
        assert_eq!(e.call(0x0087_fcb0, &args![actor, 0x300u32]).f32(), 0.0);
    }

    // ---- 0087fd20 ----

    #[test]
    fn clearing_the_disposition_modifiers_frees_each_entry_then_clears_the_list() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        let items = modifier_list(&mut e, actor, &[(1, 0x100), (2, 0x200)]);
        let freed = record(&mut e, OPERATOR_DELETE, Ret::default());
        let cleared = record(&mut e, LIST_CLEAR, Ret::default());
        e.call(0x0087_fd20, &args![actor]);
        assert_eq!(*freed.borrow(), vec![vec![items[0]], vec![items[1]]]);
        assert_eq!(*cleared.borrow(), vec![vec![actor.addr() + 0xfc]]);
    }

    // ---- 0087fd90 ----

    /// The doubles the score function needs, all returning zero except the
    /// ones that matter to the cases; `GetForm` of an object is its address + 1.
    fn score_fixture(e: &mut Engine) -> (Ptr<Actor>, Ptr<Actor>) {
        install_float_to_int(e);
        stub(
            e,
            &[
                0x005d_8a70,
                0x0082_56d0,
                0x0060_8d80,
                0x0048_bf50,
                0x008b_7fe0,
                0x008a_16d0,
                0x0049_3bb0,
                0x0097_fa10,
                0x0097_0a20,
                0x0066_ef20,
                0x0056_7790,
                0x0047_d3d0,
                ACTOR_PROCESS,
                0x005e_58f0,
                0x0064_2a60,
                FORM_TYPE,
            ],
        );
        e.set_global(PLAYER_POINTER, 0x7000u32);
        let this = actor(e, &[0x37c, 0x218, 0x21c, 0x428, 0x464]);
        let other: Ptr<Actor> = object(e, 0x1b4, 0x0203_0000, &[0x218]).cast();
        e.map(OWNER_VTABLE, 0x1000);
        e.mem.set_u32(other.addr() + 0xa4, OWNER_VTABLE);
        e.mem.set_u32(OWNER_VTABLE + 8, slot(OWNER_VTABLE, 8));
        e.register(slot(OWNER_VTABLE, 8), |_, _| ret(0x55));
        give_base(e, 0x3000);
        e.register(GET_FORM, |_, a| ret(a[0] + 1));
        (this, other)
    }

    #[test]
    fn score_is_zero_without_a_target_and_100_for_the_actors_own_form() {
        let mut e = new_engine();
        let (this, other) = score_fixture(&mut e);
        assert_eq!(e.call(0x0087_fd90, &args![this, 0u32, 0u32]).i32(), 0);
        // The cached block names the target form: GetForm(other) == other + 1.
        let cached = e.mem.alloc(0x18);
        e.mem.set_u32(cached + 8, other.addr() + 1);
        assert_eq!(e.call(0x0087_fd90, &args![this, other, cached]).i32(), 100);
    }

    #[test]
    fn score_combines_the_owner_values_and_is_clamped() {
        let mut e = new_engine();
        let (this, other) = score_fixture(&mut e);
        // First path: not creature-like (virtual +0x218 false) with an owner reference.
        let cached = e.mem.alloc(0x18);
        e.mem.set_u32(cached + 4, 0x3300); // base of this
        e.mem.set_u32(cached + 8, 0x4400); // target form
        e.mem.set_u32(cached + 0xc, 0x5500); // owner reference
        e.mem.set_u32(cached + 0x10, 0x66);
        e.mem.set_u32(cached + 0x14, 0x77);
        let combine = record(&mut e, 0x0064_2a60, ret(150));
        let seen = Rc::new(RefCell::new(0.0f32));
        let entry_point_input = seen.clone();
        e.register_double(0x005e_58f0, move |e, a| {
            *entry_point_input.borrow_mut() = e.mem.f32(a[3]);
            Ret::default()
        });
        assert_eq!(e.call(0x0087_fd90, &args![this, other, cached]).i32(), 100);
        // 00642a60(flags word, clamped value of 8, skill word, owner value).
        assert_eq!(combine.borrow()[0], vec![0x66, 0, 0x77, 0x55]);
        assert_eq!(*seen.borrow(), 150.0);
        // A negative score is raised to 0.
        e.register(0x0064_2a60, |_, _| ret((-20i32) as u32));
        assert_eq!(e.call(0x0087_fd90, &args![this, other, cached]).i32(), 0);
        e.register(0x0064_2a60, |_, _| ret(42));
        assert_eq!(e.call(0x0087_fd90, &args![this, other, cached]).i32(), 42);
        // The entry point may change the score.
        e.register(0x005e_58f0, |e, a| {
            e.mem.set_f32(a[3], 7.9);
            Ret::default()
        });
        assert_eq!(e.call(0x0087_fd90, &args![this, other, cached]).i32(), 7);
        // Second path: creature-like (virtual +0x218 true), no owner reference.
        e.register(0x005e_58f0, |_, _| Ret::default());
        e.register(slot(VTABLE, 0x218), |_, _| ret(1));
        e.mem.set_u32(cached + 0xc, 0);
        e.register(0x0064_2a60, |_, _| ret(61));
        assert_eq!(e.call(0x0087_fd90, &args![this, other, cached]).i32(), 61);
    }

    // ---- 00880370 ----

    #[test]
    fn owner_value_is_adjusted_and_truncated() {
        let mut e = new_engine();
        install_float_to_int(&mut e);
        let owner = Ptr::<()>::new(e.mem.alloc(8));
        e.map(OWNER_VTABLE, 0x1000);
        e.mem.set_u32(owner.addr(), OWNER_VTABLE);
        e.mem.set_u32(OWNER_VTABLE + 4, slot(OWNER_VTABLE, 4));
        e.register(slot(OWNER_VTABLE, 4), |_, a| {
            ((a[1] as f32) * 0.5 + 0.75).into_ret()
        });
        e.register(0x0040_4040, |_, a| (f32::from_bits(a[0]) * 2.0).into_ret());
        // virtual +4(6) = 3.75; doubled 7.5; truncated 7.
        assert_eq!(e.call(0x0088_0370, &args![owner, 6u32]).i32(), 7);
    }

    // ---- 008803a0 ----

    #[test]
    fn modified_owner_value_picks_base_modifier_or_sum() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[0x48c, 0x21c, 0x360]);
        let owner = give_owner(&mut e, actor, &[]);
        // The base form's component at +0x100 has virtual +8 giving 10.
        let base = e.mem.alloc(0x200);
        e.map(0x0204_0000, 0x1000);
        e.mem.set_u32(0x0204_0000 + 8, slot(0x0204_0000, 8));
        e.mem.set_u32(base + 0x100, 0x0204_0000);
        give_base(&mut e, base);
        e.register(slot(0x0204_0000, 8), |_, _| ret(10));
        e.register(slot(VTABLE, 0x48c), |e, a| {
            e.mem.set_u8(a[2], 1);
            4.5f32.into_ret()
        });
        stub(
            &mut e,
            &[
                ACTOR_VALUE_IS_STAT,
                ACTOR_VALUE_HAS_FLAG,
                0x0056_6950,
                0x0066_ed60,
            ],
        );
        // The modifier flag is set and the index is not a stat: the modifier.
        assert_eq!(e.call(0x0088_03a0, &args![owner, 3u32]).f32(), 4.5);
        // A stat index, nothing special: the base value.
        e.register(ACTOR_VALUE_IS_STAT, |_, _| ret(1));
        assert_eq!(e.call(0x0088_03a0, &args![owner, 3u32]).f32(), 10.0);
        // Flag 0x800 set: 0066ed60 gives 2.0; with the modifier flag the base
        // becomes the modifier 4.5 and the result is the sum.
        e.register(ACTOR_VALUE_HAS_FLAG, |_, a| ret((a[1] == 0x800) as u32));
        e.register(0x0066_ed60, |e, a| {
            e.mem.set_f32(a[2], 2.0);
            ret(1)
        });
        assert_eq!(e.call(0x0088_03a0, &args![owner, 3u32]).f32(), 6.5);
        // 0066ed60 refuses: the base value (here the modifier replaced it).
        e.register(0x0066_ed60, |_, _| ret(0));
        assert_eq!(e.call(0x0088_03a0, &args![owner, 3u32]).f32(), 4.5);
    }

    // ---- 00880580 / 008805f0 ----

    #[test]
    fn owner_thunks_use_the_process_when_there_is_one() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        let owner = give_owner(&mut e, actor, &[0]);
        e.register(slot(OWNER_VTABLE, 0), |_, _| ret(9));
        give_base(&mut e, 0x3000);
        assert_eq!(e.call(0x0088_0580, &args![owner, 4u32]).u32(), 9);
        assert_eq!(e.call(0x0088_05f0, &args![owner, 4u32]).f32(), 9.0);
        let process = give_process(&mut e, actor, &[0x398, 0x39c]);
        let first = record(&mut e, slot(PROCESS_VTABLE, 0x398), ret(77));
        let second = record(&mut e, slot(PROCESS_VTABLE, 0x39c), 1.5f32.into_ret());
        assert_eq!(e.call(0x0088_0580, &args![owner, 4u32]).u32(), 77);
        assert_eq!(e.call(0x0088_05f0, &args![owner, 4u32]).f32(), 1.5);
        let expected = vec![process.addr(), 0x3000, 4, actor.addr()];
        assert_eq!(first.borrow()[0], expected);
        assert_eq!(second.borrow()[0], expected);
    }

    // ---- 00880660 / 00880690 / 008806d0 ----

    #[test]
    fn modifier_list_accessors_at_0xe0() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[0x48, 0x394]);
        let getter = record(&mut e, 0x0093_76e0, 2.5f32.into_ret());
        let setter = record(&mut e, 0x0093_75e0, Ret::default());
        let flags = record(&mut e, slot(VTABLE, 0x48), Ret::default());
        let change = record(&mut e, slot(VTABLE, 0x394), Ret::default());
        assert_eq!(e.call(0x0088_0660, &args![actor, 3u8, 9u32]).f32(), 2.5);
        assert_eq!(getter.borrow()[0], vec![actor.addr() + 0xe0, 3, 9]);
        e.call(0x0088_0690, &args![actor, 4u8, 1.5f32]);
        let expected = vec![actor.addr() + 0xe0, 4, 1.5f32.to_bits()];
        assert_eq!(setter.borrow()[0], expected);
        assert_eq!(flags.borrow()[0], vec![actor.addr(), 0x400000]);
        e.call(0x0088_06d0, &args![actor, 7u32, -2i32]);
        let expected = vec![actor.addr(), 7, (-2.0f32).to_bits()];
        assert_eq!(change.borrow()[0], expected);
    }

    // ---- 00880700 ----

    #[test]
    fn float_actor_value_change_clamps_with_flag_0x100() {
        let mut e = new_engine();
        e.set_global(0x0101_2070, 1.0f64);
        let actor = actor(&mut e, &[0x490, 0x360]);
        give_owner(&mut e, actor, &[0xc]);
        give_process(&mut e, actor, &[0x3b0]);
        e.register(ACTOR_VALUE_HAS_BASE, |_, _| ret(1));
        e.register(slot(OWNER_VTABLE, 0xc), |_, _| 8.0f32.into_ret());
        e.register(ACTOR_VALUE_HAS_FLAG, |_, a| ret((a[1] == 0x100) as u32));
        let record_block = e.mem.alloc(0x100);
        e.mem.set_i32(record_block + 0x98, 50);
        e.register_double(0x0066_e920, move |_, _| ret(record_block));
        let change = record(&mut e, slot(VTABLE, 0x490), Ret::default());
        let process_call = record(&mut e, slot(PROCESS_VTABLE, 0x3b0), Ret::default());
        let report = record(&mut e, ACTOR_VALUE_CHANGED, Ret::default());
        // Negative: 0.
        e.call(0x0088_0700, &args![actor, 5u32, -3.0f32]);
        assert_eq!(change.borrow()[0], vec![actor.addr(), 5, 0]);
        // Above the maximum: maximum - 1.
        e.call(0x0088_0700, &args![actor, 5u32, 60.0f32]);
        assert_eq!(change.borrow()[1][2], 49.0f32.to_bits());
        // Below the maximum: unchanged.
        e.call(0x0088_0700, &args![actor, 5u32, 20.0f32]);
        assert_eq!(change.borrow()[2][2], 20.0f32.to_bits());
        assert_eq!(process_call.borrow().len(), 3);
        let expected = vec![
            actor.addr() + 0xa4,
            5,
            8.0f32.to_bits(),
            20.0f32.to_bits(),
            0,
        ];
        assert_eq!(report.borrow()[2], expected);
    }

    // ---- 00880850 / 00880890 ----

    #[test]
    fn adjustment_helpers_pass_the_value_through() {
        let mut e = new_engine();
        install_float_to_int(&mut e);
        e.register(0x0040_4040, |_, a| (f32::from_bits(a[0]) + 0.5).into_ret());
        let words = args![0u32, 1u32, 2.5f32, 3u32];
        assert_eq!(e.call(0x0088_0890, &words).f32(), 2.5);
        assert_eq!(e.call(0x0088_0850, &args![0u32, 1u32, 4i32, 3u32]).i32(), 4);
    }

    // ---- 008808a0 ----

    #[test]
    fn health_modifier_is_scaled_by_the_difficulty_factor() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[0x360]);
        e.set_global(PLAYER_POINTER, 0x7000u32);
        e.register(slot(VTABLE, 0x360), |_, _| ret(1));
        let level = record(&mut e, 0x005b_e4d0, ret(2));
        let factor = record(&mut e, 0x0064_8cb0, 1.5f32.into_ret());
        assert_eq!(e.call(0x0088_08a0, &args![actor, 4.0f32, 0u32]).f32(), 6.0);
        assert_eq!(level.borrow()[0], vec![0x7000]);
        assert_eq!(factor.borrow()[0], vec![2, 0x10, 1]);
    }

    // ---- 008808f0 / 00880910 ----

    #[test]
    fn small_forwarders() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[]);
        let stat = record(&mut e, ACTOR_VALUE_IS_STAT, ret(1));
        assert_eq!(e.call(0x0088_08f0, &args![actor, 12u32, 0u32]).u32(), 1);
        assert_eq!(stat.borrow()[0], vec![12]);
        e.register(GET_EXTRA_DATA_LIST, |_, a| ret(a[0] + 0x44));
        e.register(0x0041_8520, |_, _| ret(0));
        e.register(0x004b_fc80, |_, _| ret(1));
        assert!(!e.call(0x0088_0910, &args![actor]).bool());
        e.register(0x0041_8520, |_, _| ret(0x8000));
        assert!(e.call(0x0088_0910, &args![actor]).bool());
    }

    // ---- 00880950 and its three siblings ----

    /// An actor with everything the integer and float change functions use:
    /// (actor, source, report log, flag-bit log, hit log, modifier-list log).
    /// Index `0x99` has the flag `0x100`.
    fn change_fixture(e: &mut Engine) -> (Ptr<Actor>, Ptr<Actor>, Log, Log, Log, Log) {
        install_float_to_int(e);
        e.set_global(0x0101_2070, 1.0f64);
        let slots = [0x38c, 0x4b8, 0x48, 0x3a4, 0x3a0, 0x3ac, 0x3a8, 0x3b0];
        let this = actor(e, &slots);
        let source: Ptr<Actor> = object(e, 0x1b4, 0x0205_0000, &[]).cast();
        give_owner(e, this, &[0xc]);
        give_process(e, this, &[0x3a4, 0x3a0, 0x3ac, 0x3a8, 0x3b0]);
        e.register(ACTOR_VALUE_HAS_BASE, |_, _| ret(1));
        e.register(slot(OWNER_VTABLE, 0xc), |_, _| 3.0f32.into_ret());
        e.register(ACTOR_VALUE_HAS_FLAG, |_, a| {
            ret((a[1] == 0x100 && a[0] == 0x99) as u32)
        });
        e.register(0x0040_4040, |_, a| f32::from_bits(a[0]).into_ret());
        e.register(ACTOR_VALUE_IS_STAT, |_, _| ret(1));
        let report = record(e, ACTOR_VALUE_CHANGED, Ret::default());
        let flags = record(e, slot(VTABLE, 0x48), Ret::default());
        let hit = record(e, slot(VTABLE, 0x4b8), Ret::default());
        let list = record(e, 0x0093_7480, Ret::default());
        (this, source, report, flags, hit, list)
    }

    #[test]
    fn integer_change_calls_the_process_then_reports() {
        let mut e = new_engine();
        let (this, source, report, flags, hit, _) = change_fixture(&mut e);
        let process_call = record(&mut e, slot(PROCESS_VTABLE, 0x3a4), Ret::default());
        e.call(0x0088_0950, &args![this, 0x10u32, -4i32, source]);
        assert_eq!(
            process_call.borrow()[0][1..],
            [this.addr(), 0x10, (-4i32) as u32]
        );
        let expected = vec![this.addr(), source.addr(), (-4.0f32).to_bits()];
        assert_eq!(hit.borrow()[0], expected);
        assert_eq!(flags.borrow()[0], vec![this.addr(), 0x100000]);
        let expected = vec![
            this.addr() + 0xa4,
            0x10,
            3.0f32.to_bits(),
            (-4.0f32).to_bits(),
            source.addr() + 0xa4,
        ];
        assert_eq!(report.borrow()[0], expected);
        // The flag 0x100 index is ignored; index 0x16 with a negative value too.
        e.call(0x0088_0950, &args![this, 0x99u32, 4i32, source]);
        e.call(0x0088_0950, &args![this, 0x16u32, -4i32, source]);
        assert_eq!(report.borrow().len(), 1);
        e.register(slot(VTABLE, 0x38c), |_, _| ret(1));
        e.call(0x0088_0950, &args![this, 0x16u32, -4i32, source]);
        assert_eq!(report.borrow().len(), 2);
    }

    #[test]
    fn float_change_calls_the_process_then_reports() {
        let mut e = new_engine();
        let (this, source, report, flags, hit, _) = change_fixture(&mut e);
        let process_call = record(&mut e, slot(PROCESS_VTABLE, 0x3a0), Ret::default());
        e.call(0x0088_0ad0, &args![this, 0x10u32, -4.5f32, source]);
        let expected = [this.addr(), 0x10, (-4.5f32).to_bits()];
        assert_eq!(process_call.borrow()[0][1..], expected);
        let expected = vec![this.addr(), source.addr(), (-4.5f32).to_bits()];
        assert_eq!(hit.borrow()[0], expected);
        assert_eq!(flags.borrow()[0], vec![this.addr(), 0x100000]);
        assert_eq!(report.borrow()[0][3], (-4.5f32).to_bits());
        e.call(0x0088_0ad0, &args![this, 0x16u32, -1.0f32, source]);
        e.call(0x0088_0ad0, &args![this, 0x99u32, 1.0f32, source]);
        assert_eq!(report.borrow().len(), 1);
        // A non-negative amount does not call the hit function.
        e.call(0x0088_0ad0, &args![this, 0x10u32, 2.0f32, source]);
        assert_eq!(hit.borrow().len(), 1);
    }

    #[test]
    fn integer_change_to_the_modifier_list_at_0xd0() {
        let mut e = new_engine();
        let (this, source, report, flags, _, list) = change_fixture(&mut e);
        let process_call = record(&mut e, slot(PROCESS_VTABLE, 0x3b0), Ret::default());
        e.call(0x0088_0c70, &args![this, 0x10u32, 7i32, source]);
        let expected = vec![this.addr() + 0xd0, 0x10, 7.0f32.to_bits(), 2];
        assert_eq!(list.borrow()[0], expected);
        assert_eq!(flags.borrow()[0], vec![this.addr(), 0x800000]);
        assert_eq!(process_call.borrow()[0][1], 0x10);
        assert_eq!(report.borrow()[0][3], 7.0f32.to_bits());
        e.call(0x0088_0c70, &args![this, 0x99u32, 7i32, source]);
        assert_eq!(report.borrow().len(), 1);
    }

    #[test]
    fn float_change_to_the_modifier_list_at_0xd0() {
        let mut e = new_engine();
        let (this, source, report, flags, hit, list) = change_fixture(&mut e);
        let process_call = record(&mut e, slot(PROCESS_VTABLE, 0x3b0), Ret::default());
        e.call(0x0088_0e00, &args![this, 0x10u32, -2.5f32, source]);
        let expected = vec![this.addr() + 0xd0, 0x10, (-2.5f32).to_bits(), 2];
        assert_eq!(list.borrow()[0], expected);
        assert_eq!(flags.borrow()[0], vec![this.addr(), 0x800000]);
        assert_eq!(hit.borrow().len(), 1);
        assert_eq!(process_call.borrow()[0][1], 0x10);
        assert_eq!(report.borrow()[0][3], (-2.5f32).to_bits());
        e.call(0x0088_0e00, &args![this, 0x99u32, 1.0f32, source]);
        assert_eq!(report.borrow().len(), 1);
    }

    #[test]
    fn integer_change_variant_0x3ac_sets_flag_0x200000() {
        let mut e = new_engine();
        let (this, source, report, flags, _, _) = change_fixture(&mut e);
        let process_call = record(&mut e, slot(PROCESS_VTABLE, 0x3ac), Ret::default());
        e.call(0x0088_0fb0, &args![this, 0x20u32, 5i32, source]);
        assert_eq!(process_call.borrow()[0][1..], [this.addr(), 0x20, 5]);
        assert_eq!(flags.borrow()[0], vec![this.addr(), 0x200000]);
        assert_eq!(report.borrow()[0][3], 5.0f32.to_bits());
        e.call(0x0088_0fb0, &args![this, 0x99u32, 5i32, source]);
        assert_eq!(report.borrow().len(), 1);
    }

    #[test]
    fn float_change_variant_0x3a8_bounds_index_0x16_by_the_setting() {
        let mut e = new_engine();
        let (this, source, report, flags, _, _) = change_fixture(&mut e);
        let process_call = record(&mut e, slot(PROCESS_VTABLE, 0x3a8), Ret::default());
        // The setting (float at 0x011d2664 through 00403e20) is 1.0; the base is 3.0.
        let setting = e.mem.alloc(4);
        e.mem.set_f32(setting, 1.0);
        e.register_double(0x0040_3e20, move |_, _| ret(setting));
        e.call(0x0088_1130, &args![this, 0x20u32, 2.0f32, source]);
        let expected = [this.addr(), 0x20, 2.0f32.to_bits()];
        assert_eq!(process_call.borrow()[0][1..], expected);
        assert_eq!(flags.borrow()[0], vec![this.addr(), 0x200000]);
        // Index 0x16, amount -1: 3 - 1 = 2 stays above the setting: unchanged.
        e.register(slot(VTABLE, 0x38c), |_, _| ret(1));
        e.call(0x0088_1130, &args![this, 0x16u32, -1.0f32, source]);
        assert_eq!(process_call.borrow()[1][3], (-1.0f32).to_bits());
        // Amount -5: 3 - 5 is below the setting, so the amount becomes 1 - 3 = -2.
        e.call(0x0088_1130, &args![this, 0x16u32, -5.0f32, source]);
        assert_eq!(process_call.borrow()[2][3], (-2.0f32).to_bits());
        // The setting above the base: nothing happens.
        e.mem.set_f32(setting, 4.0);
        let before = report.borrow().len();
        e.call(0x0088_1130, &args![this, 0x16u32, -1.0f32, source]);
        assert_eq!(report.borrow().len(), before);
        // The flag 0x100 index is ignored.
        e.call(0x0088_1130, &args![this, 0x99u32, 1.0f32, source]);
        assert_eq!(report.borrow().len(), before);
    }

    // ---- 00881330 / 00881360 ----

    #[test]
    fn forwarding_value_changes() {
        let mut e = new_engine();
        let actor = actor(&mut e, &[0x3b4, 0x394]);
        give_owner(&mut e, actor, &[0xc, 4]);
        let direct = record(&mut e, slot(VTABLE, 0x3b4), Ret::default());
        e.call(0x0088_1330, &args![actor, 3u32, 9i32]);
        assert_eq!(direct.borrow()[0], vec![actor.addr(), 3, 9.0f32.to_bits()]);
        e.register(ACTOR_VALUE_HAS_BASE, |_, _| ret(1));
        e.register(ACTOR_VALUE_HAS_FLAG, |_, a| ret((a[0] == 0x99) as u32));
        e.register(ACTOR_VALUE_IS_STAT, |_, _| ret(1));
        e.register(slot(OWNER_VTABLE, 0xc), |_, _| 3.0f32.into_ret());
        e.register(slot(OWNER_VTABLE, 4), |_, _| 10.0f32.into_ret());
        let set = record(&mut e, slot(VTABLE, 0x394), Ret::default());
        let report = record(&mut e, ACTOR_VALUE_CHANGED, Ret::default());
        e.call(0x0088_1360, &args![actor, 7u32, 2.5f32]);
        assert_eq!(set.borrow()[0], vec![actor.addr(), 7, 12.5f32.to_bits()]);
        let expected = vec![
            actor.addr() + 0xa4,
            7,
            3.0f32.to_bits(),
            2.5f32.to_bits(),
            0,
        ];
        assert_eq!(report.borrow()[0], expected);
        e.call(0x0088_1360, &args![actor, 0x99u32, 2.5f32]);
        assert_eq!(set.borrow().len(), 1);
    }
}
