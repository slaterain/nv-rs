//! `fallout/ai/actor.cpp` (Xbox PDB source unit), part 2: its functions from `00884990` up to
//! (not including) `00891d70` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::actor`]; anything public there may be used here.
//!
//! This session covers `00884990` to `00885d70`: the actor's swimming state,
//! its cached values (radius, width, length, walk and run speed, the
//! `CachedValuesOwner` virtuals that `Actor` overrides through its secondary
//! vtable at `Actor + 0xa8`), its height, and the follow-package speed rules.
//!
//! The second block (b0148p2) covers `00885d90` to `0088b150`: two more speed
//! formulas, the process-level change, three large per-frame updates
//! (`00886360`, `00886cb0` and `Actor::Update`, `00888b50`), the ragdoll and
//! character-proxy accessors, `ResetLoadedAnimations`,
//! `UpdateAnimationMovement` and `UpdateActor3DPosition`. Their exception
//! frames (SEH) are not translated. Next block: continue at `0088b4e0`
//! (the next function the queue lists as open after `0088b150`).
//!
//! x87 note: the game computes in extended precision and stores `float`
//! results. The translations compute in `f64` and round to `f32` where the
//! code stores a `float`.
//!
//! Virtual slots: the PC `Actor` vtable (`01084254`) has its slots 4 bytes
//! higher than the Xbox PDB's from the bounds getters on, so a PC slot is
//! described by its offset and, where the vtable contents confirm it, by the
//! Xbox PDB name of the slot four bytes lower.

#[allow(unused_imports)]
use super::actor::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// Returns the actor's flag word (`00884990` copies bit `0x800` into
/// `bSwimming`; `004997b0`, `00884750` and the follow code test other bits).
const GET_FLAG_WORD: u32 = 0x0088_46e0;
/// Tells whether the process has a cached-values block (`pCachedValues != 0`).
const HAS_CACHED_VALUES: u32 = 0x0088_4920;
/// ORs a mask into `CachedValues::iFlags`.
const CACHED_VALUES_ADD_FLAGS: u32 = 0x0088_48c0;
/// Process-level function that forwards a `CachedValues::iFlags` mask to
/// `00884970` when the process has a cached-values block.
const PROCESS_FORWARD_FLAG_MASK: u32 = 0x0088_4940;
/// `Actor::GetCurrentPackageTarget` (Xbox PDB): the reference the actor's
/// current package targets, or null.
const GET_CURRENT_PACKAGE_TARGET: u32 = 0x0088_1650;
/// `MobileObject::GetCurrentPackage` (Xbox PDB).
const GET_CURRENT_PACKAGE: u32 = 0x0093_44a0;
/// `MobileObject::GetCharController` (Xbox PDB).
const GET_CHAR_CONTROLLER: u32 = 0x0093_06d0;
/// `TESObjectREFR::GetScale` (Xbox PDB), a `float` in ST0.
const GET_SCALE: u32 = 0x0056_7400;
/// `TESObjectREFR::GetDistanceFromReference` (Xbox PDB): reference, two
/// more words, `float` in ST0.
const GET_DISTANCE_FROM_REFERENCE: u32 = 0x0057_23b0;
/// `TESObjectREFR::GetWorldSpace` (Xbox PDB).
const GET_WORLD_SPACE: u32 = 0x0057_5d70;
/// `TESObjectREFR::GetRelevantWaterHeight` (Xbox PDB), `float` in ST0.
const GET_RELEVANT_WATER_HEIGHT: u32 = 0x0057_b0a0;
/// `TESObjectCELL::GetWaterHeight` (Xbox PDB), `float` in ST0.
const CELL_GET_WATER_HEIGHT: u32 = 0x0054_71e0;
/// The reference's parent cell (`ECX` = reference).
const GET_PARENT_CELL: u32 = 0x008d_6f30;
/// Cell test (`ECX` = cell, `bool` in `AL`) the follow code applies to the
/// two actors' cells to decide whether they are in an interior.
const CELL_TEST_INTERIOR: u32 = 0x0042_5fd0;
/// `Actor::GetEyeLevel` (Xbox PDB), `float` in ST0.
const GET_EYE_LEVEL: u32 = 0x008b_e940;
/// `Actor::GetArmorBeingWorn` (Xbox PDB): actor and a slot number.
const GET_ARMOR_BEING_WORN: u32 = 0x0089_1b90;
/// `Actor::GetCurrentSpeed` (Xbox PDB), `float` in ST0.
const GET_CURRENT_SPEED: u32 = 0x008a_0b10;
/// `cdecl`, two `float` arguments, `float` in ST0: returns one of them (the
/// follow code uses it to keep a distance from going below zero and the
/// cached extents above a minimum, so it is a maximum).
const FLOAT_HELPER_MAX: u32 = 0x0040_4010;
/// `cdecl`, two `float` arguments, `float` in ST0: the other of the pair
/// (the follow code uses it as a minimum).
const FLOAT_HELPER_MIN: u32 = 0x0040_ebd0;
/// `cdecl` (`float*` value, `float` lower, `float` upper): forces `*value`
/// into `[lower, upper]` (the upper bound is applied first).
const CLAMP_FLOAT: u32 = 0x0053_30e0;
/// Returns the address of the `float` held by the settings object in `ECX`
/// (a static zero when `ECX` is null).
const SETTING_VALUE_ADDRESS: u32 = 0x0040_3e20;
/// `cdecl`, one `float`: the larger of the value minus a setting and the
/// value times another setting.
const FOLLOW_RADIUS_NEAR: u32 = 0x0064_3f20;
/// `cdecl`, one `float`: returns its argument unchanged.
const FOLLOW_RADIUS_MID: u32 = 0x0064_3f90;
/// `AiFormulas::GetFollowRadiusWalk` (Xbox PDB), `cdecl`, one `float`.
const FOLLOW_RADIUS_WALK: u32 = 0x0064_3f70;
/// `AiFormulas::GetFollowRadiusMatchSpeed` (Xbox PDB), `cdecl`, one `float`.
const FOLLOW_RADIUS_MATCH_SPEED: u32 = 0x0064_3fd0;
/// Actor-level test (`bool` in `AL`) that the follow code requires before
/// it forces a run.
const FOLLOW_MODE_ALLOWED: u32 = 0x0088_4750;
/// `BGSEntryPoint::HandleEntryPoint` (Xbox PDB), `cdecl`: entry point
/// number, actor, `float*` value.
const HANDLE_ENTRY_POINT: u32 = 0x005e_58f0;
/// The walk speed formula: `cdecl`, seven words (`ActorValueOwner*`, two
/// item values, four flags), `float` in ST0.
const CALCULATE_WALK_SPEED_FORMULA: u32 = 0x0064_7d10;
/// The run speed formula: `cdecl`, seven words, `float` in ST0.
const CALCULATE_RUN_SPEED_FORMULA: u32 = 0x0064_7f00;
/// Flag test of the actor used by the speed formulas (`ECX` = actor,
/// `bool` in `AL`).
const SPEED_FLAG_TEST: u32 = 0x0049_97b0;
/// `ECX` = item entry, returns its word at +0x08.
const ITEM_VALUE: u32 = 0x0044_ddc0;
/// `ECX` = item, one stack word; called on the worn armor after its value
/// is read.
const ITEM_RELEASE: u32 = 0x0044_59e0;
/// `ECX` = the object `Actor` virtual `+0x1e4` returned, one stack word
/// (`1`); returns an object or null.
const ANIMATION_LOOKUP: u32 = 0x0049_1040;
/// Named `Animation::ZeroGlobalTransform` in the engine map; takes the
/// object `00491040` returned in `ECX` and returns an object.
const ANIMATION_STEP: u32 = 0x0048_f7f0;
/// Tests the object `0048f7f0` returned (`ECX`), `bool` in `AL`.
const ANIMATION_TEST: u32 = 0x005f_4d60;
/// Returns a kind number for the object `0048f7f0` returned (`ECX`).
const ANIMATION_KIND: u32 = 0x005f_2420;
/// Returns a state number for the character controller (`ECX`).
const CONTROLLER_STATE: u32 = 0x005c_0880;
/// `ImpactMixer::PlayJump` (Xbox PDB), `cdecl`: actor and a word.
const PLAY_JUMP: u32 = 0x0083_8780;
/// `MobileObject` call (actor in `ECX`) whose non-zero answer makes
/// `00884aa0` play the jump sound.
const JUMP_TEST: u32 = 0x0093_0640;
/// `Actor::StopMoving` (Xbox PDB).
const STOP_MOVING: u32 = 0x008b_3ab0;
/// `ECX` = the object process virtual `+0x22c` returned; a type number.
const OBJECT_TYPE: u32 = 0x0041_ca90;
/// Actor-level test (`bool` in `AL`) the swim check ends with.
const SWIM_TEST_FINAL: u32 = 0x0087_f350;
/// Actor-level test (`bool` in `AL`) the swim check applies before asking
/// whether the controller is fleeing.
const SWIM_TEST_COMBAT: u32 = 0x0087_f3b0;
/// `CombatController::IsFleeing` (Xbox PDB).
const COMBAT_CONTROLLER_IS_FLEEING: u32 = 0x0098_1990;
/// Test of the combat controller (`ECX`) the swim check applies.
const COMBAT_CONTROLLER_TEST: u32 = 0x0047_c850;
/// Test of the object process virtual `+0x22c` returned (`ECX`), used by
/// the swim check for creatures.
const SWIM_OBJECT_TEST: u32 = 0x0067_a560;
/// `BGSSaveFormBuffer::GetForm` (Xbox PDB): the form behind a reference
/// (`ECX`).
const SAVE_FORM_BUFFER_GET_FORM: u32 = 0x007a_f430;
/// Returns the form type byte (`TESForm::cFormType`, +0x04) of the form in
/// `ECX`.
const FORM_TYPE: u32 = 0x0040_1170;
/// Pointer global holding the player character (`005732d3` passes it as
/// the `this` of `008859e0`).
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
/// Process-level test (`ECX` = process): non-zero means the actor height is
/// not cached in the process.
const CACHED_HEIGHT_BYPASS: u32 = 0x0045_cd60;

/// `0.0` as a `double` (the unset value of the cached actor height).
const DOUBLE_ZERO: u32 = 0x0101_2060;
/// `1.0` as a `double`.
const DOUBLE_ONE: u32 = 0x0101_2070;
/// `0.5` as a `double`.
const DOUBLE_HALF: u32 = 0x0101_1588;
/// `0.57` (the `float`, widened) as a `double`: the sneak-height factor.
const SNEAK_HEIGHT_FACTOR: u32 = 0x0108_4830;
/// `1.25` as a `double`.
const FOLLOW_CATCH_UP_FACTOR: u32 = 0x0102_1798;
/// `32.0` as a `float`: the least extent before halving into a radius.
const MINIMUM_EXTENT: u32 = 0x0101_e340;

/// Settings objects whose `float` the follow code reads through
/// [`SETTING_VALUE_ADDRESS`]: the speed limit factors by the actor's flag
/// state and the interpolation range.
const SETTING_UPPER_FACTOR_ALL_FLAGS: u32 = 0x011c_d744;
const SETTING_UPPER_FACTOR_OTHERWISE: u32 = 0x011c_d9b4;
const SETTING_LOWER_FACTOR_ALL_FLAGS: u32 = 0x011c_d5f0;
const SETTING_LOWER_FACTOR_OTHERWISE: u32 = 0x011c_df9c;
const SETTING_INTERPOLATION_FACTOR: u32 = 0x011c_d354;

/// `CachedValues::iFlags` masks that `Actor`'s cached-value setters use.
const CACHED_RADIUS_FLAG: u32 = 0x1;
const CACHED_WIDTH_FLAG: u32 = 0x2;
const CACHED_LENGTH_FLAG: u32 = 0x4;
const CACHED_WALK_SPEED_FLAG: u32 = 0x1000;
const CACHED_RUN_SPEED_FLAG: u32 = 0x2000;
const CACHED_FORWARD_LENGTH_FLAG: u32 = 0x8000;

/// `Actor` virtual `+0x458`: `CalculateWalkSpeed` (Xbox PDB name of the slot
/// at `+0x454`; the vtable at `01084254` holds `00885a50` there).
const VSLOT_CALCULATE_WALK_SPEED: u32 = 0x458;
/// `Actor` virtual `+0x45c`: `CalculateRunSpeed` (Xbox PDB, `+0x458`);
/// holds `00885bf0`.
const VSLOT_CALCULATE_RUN_SPEED: u32 = 0x45c;
/// `Actor` virtual `+0x100`: `IsActor` (Xbox PDB).
const VSLOT_IS_ACTOR: u32 = 0x100;
/// `Actor` virtual `+0x1dc`: writes a 12-byte vector into its argument and
/// returns it (the upper corner of the bounds, since the height is its z
/// minus `+0x1d8`'s).
const VSLOT_BOUNDS_UPPER: u32 = 0x1dc;
/// `Actor` virtual `+0x1d8`: the lower corner of the bounds.
const VSLOT_BOUNDS_LOWER: u32 = 0x1d8;
/// `Actor` virtual `+0x1e4`: returns an animation object (the Xbox PDB's
/// `GetAnimation` is at `+0x1e0`).
const VSLOT_GET_ANIMATION: u32 = 0x1e4;
/// `Actor` virtual `+0x218` (Xbox PDB `IsCreature`, which sits below the
/// shift).
const VSLOT_IS_CREATURE: u32 = 0x218;
/// `Actor` virtual `+0x358` (Xbox PDB `+0x354`; the name is not confirmed).
const VSLOT_FLAG_0X358: u32 = 0x358;
/// `Actor` virtual `+0x428`: `GetCombatController` (Xbox PDB `+0x424`);
/// `IsAllowedToSwim` passes its result to `CombatController::IsFleeing`.
const VSLOT_GET_COMBAT_CONTROLLER: u32 = 0x428;
/// Process virtual `+0x22c`: returns the object `00884ae0` types.
const PROCESS_VSLOT_OBJECT: u32 = 0x22c;
/// Process virtual `+0x454`: a flag the speed formulas negate.
const PROCESS_VSLOT_FLAG: u32 = 0x454;
/// Process virtual `+0x148`: returns the item the speed formulas weigh.
const PROCESS_VSLOT_ITEM: u32 = 0x148;
/// `CachedValuesOwner` virtual `+0x38` (`CalculateCachedWalkSpeed`) and
/// `+0x3c` (`CalculateCachedRunSpeed`), Xbox PDB.
const OWNER_VSLOT_WALK_SPEED: u32 = 0x38;
const OWNER_VSLOT_RUN_SPEED: u32 = 0x3c;

/// How far the `CachedValuesOwner` subobject sits inside an `Actor` on PC.
const CACHED_VALUES_OWNER_OFFSET: u32 = 0xa8;
/// How far the `ActorValueOwner` subobject sits inside an `Actor` on PC.
const ACTOR_VALUE_OWNER_OFFSET: u32 = 0xa4;

/// `Actor::GetArmorBeingWorn`'s slot for the speed formulas.
const SPEED_ARMOR_SLOT: u32 = 2;

/// Bits of the flag word `GET_FLAG_WORD` returns, as the follow code tests
/// them (meanings not confirmed), and of the movement mode word.
const FLAGS_MATCHING_PACE_MASK: u32 = 0x20f;
const TARGET_FLAGS_HOLD_MASK: u32 = 0xf;
const TARGET_FLAGS_MODE_MASK: u32 = 0x500;
const TARGET_FLAGS_MODE_VALUE: u32 = 0x400;
const TARGET_FLAGS_BIT_200: u32 = 0x200;
const MODE_BIT_200: u32 = 0x200;
const MODE_BIT_100: u32 = 0x100;
const MODE_BIT_800: u32 = 0x800;
/// `TESForm::iFormFlags` bit tested by `00885a30`.
const FORM_FLAG_20000000: u32 = 0x2000_0000;

/// The compiler's byte table for the switch at `00884a2b` (`00884a88`), by
/// `kind - 0xe3`: `0` is the case that answers true (kinds `0xe3`, `0xe5`
/// and `0xf1` to `0xf4`), `1` carries on.
const ANIMATION_KIND_TABLE: [u8; 18] = [0, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0];

layout! {
    /// `BaseProcess` (Xbox PDB), size 0x30.
    pub struct BaseProcess: 0x30 {
        /// `pCachedValues` (Xbox PDB).
        0x2c pCachedValues: Ptr<CachedValues>,
    }

    /// `HighProcess` (Xbox PDB), size 0x46c: only the cached height.
    pub struct HighProcess: 0x46c {
        /// `fCachedActorHeight` (Xbox PDB).
        0x42c fCachedActorHeight: f32,
    }

    /// `CachedValues` (Xbox PDB), 0x48 bytes.
    pub struct CachedValues: 0x48 {
        /// `fCachedRadius` (Xbox PDB).
        0x00 fCachedRadius: f32,
        /// `fCachedWidth` (Xbox PDB).
        0x04 fCachedWidth: f32,
        /// `fCachedLength` (Xbox PDB).
        0x08 fCachedLength: f32,
        /// `fCachedForwardLength` (Xbox PDB).
        0x0c fCachedForwardLength: f32,
        /// `fCachedWalkSpeed` (Xbox PDB).
        0x38 fCachedWalkSpeed: f32,
        /// `fCachedRunSpeed` (Xbox PDB).
        0x3c fCachedRunSpeed: f32,
        /// `iFlags` (Xbox PDB): which cached values are valid.
        0x44 iFlags: u32,
    }

    /// `CachedValuesOwner` (Xbox PDB), the 4-byte secondary base at
    /// `Actor + 0xa8`.
    pub struct CachedValuesOwner: 0x4 {}
}

/// The `Actor` an `Actor + 0xa8` subobject pointer belongs to.
fn actor_of_owner(owner: Ptr<CachedValuesOwner>) -> Ptr<Actor> {
    Ptr::new(owner.addr().wrapping_sub(CACHED_VALUES_OWNER_OFFSET))
}

/// The `float` a settings object holds, read through
/// [`SETTING_VALUE_ADDRESS`].
fn setting_value(e: &mut Engine, setting: u32) -> f32 {
    let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
    e.mem.f32(address)
}

// ---------------------------------------------------------------------------
// Callees, globals and slots of the second block (b0148p2). Meanings are
// given only where the body of the callee or its use confirms them.

/// Reads `[ECX + 0x68]`: the actor's process (`Actor::pCurrentProcess`).
const GET_PROCESS: u32 = 0x008d_8520;
/// An empty function (`ret`); the game calls it on scope objects (a global
/// or a stack temporary in `ECX`).
const EMPTY_FUNCTION: u32 = 0x0048_3710;
const SCOPE_OBJECT_UPDATE: u32 = 0x011d_f69d;
const SCOPE_OBJECT_0X11DF71D: u32 = 0x011d_f71d;
const SCOPE_OBJECT_0X11DF69F: u32 = 0x011d_f69f;
const SCOPE_OBJECT_0X11DF69C: u32 = 0x011d_f69c;
/// `ExtraDataList` of the reference (same function as `extra_data_list`).
const GET_EXTRA_DATA_LIST_ADDRESS: u32 = 0x005d_43c0;
/// Returns the `float` frame step (`ST0`) of the settings object.
const UPDATE_STEP_SETTING: u32 = 0x0084_d030;
const UPDATE_STEP_OBJECT: u32 = 0x011f_6394;
/// The `Calendar` singleton object.
const CALENDAR: u32 = 0x011d_e7b8;
const CALENDAR_GET_DAY: u32 = 0x0086_7d60;
const CALENDAR_GET_MONTH: u32 = 0x0086_7ef0;
const CALENDAR_GET_HOUR: u32 = 0x0086_7da0;
const CALENDAR_GET_TIME_SCALE: u32 = 0x0086_7950;
const CALENDAR_GET_TIME: u32 = 0x0086_7e30;
/// The process-level number of the process in `ECX`.
const PROCESS_GET_LEVEL: u32 = 0x0045_cd60;
/// Returns the address of the `int` an integer setting object holds.
const GLOBAL_WORD_ADDRESS: u32 = 0x0043_d4d0;
const SETTING_INT_VALUE: u32 = 0x0044_ddc0;
const SETTING_INT_FN: u32 = 0x0040_3df0;
const SETTING_VALUE_POINTER: u32 = 0x0040_8d60;
const MODE_SETTING: u32 = 0x011c_3ea4;
const VATS_STATE_OBJECT: u32 = 0x011f_2250;
const VATS_TARGET_ACTOR: u32 = 0x011f_21cc;
const VATS_TARGET_UPDATE_MULT: u32 = 0x009c_8d60;
const PROCESS_LISTS: u32 = 0x011e_0e80;
const TES_POINTER: u32 = 0x011d_ea10;
const PLAYER_UPDATE_NESTING: u32 = 0x011e_07a8;
const SCALE_VECTOR: u32 = 0x011f_426c;
const HEAD_TARGET_A: u32 = 0x011e_07d4;
const HEAD_TARGET_B: u32 = 0x011e_07d0;
const FOLLOWER_BARK_COUNTDOWN: u32 = 0x011e_07e8;
const FOLLOWER_BARK_RESET: u32 = 0x0101_3dc0;
const SKIP_SET_POSITION: u32 = 0x011e_0550;
const PROCESSING_ENABLED: u32 = 0x011f_1220;
const INTERRUPT_UPDATES_ENABLED: u32 = 0x011f_122c;
const MANAGER_011DE45C: u32 = 0x011d_e45c;
const COMBAT_OBJECT_POINTER: u32 = 0x011d_e45c;
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const KF_GROUP_TABLE: u32 = 0x0118_a838;
const GLOBAL_VALUE_011C6260: u32 = 0x011c_6260;
const GLOBAL_FLAG_012677A3: u32 = 0x0126_77a3;
const GLOBAL_FLAG_011F3E60: u32 = 0x011f_3e60;
const MOVER_SETTING: u32 = 0x011d_f6b0;
const ESSENTIAL_SETTING: u32 = 0x011e_0888;
const MONTH_SETTING_A: u32 = 0x011d_058c;
const MONTH_SETTING_B: u32 = 0x011d_0c3c;
const ANIMATION_SPEED_SETTING: u32 = 0x011d_0fa8;
const EYE_DIRECTION: u32 = 0x011a_9484;
const ANIMATION_GROUP_KEY: u32 = 0x0119_9760;
const SPEED_SETTING_21C_SET_TRUE: u32 = 0x011f_2b10;
const SPEED_SETTING_21C_SET_FALSE: u32 = 0x011f_2c10;
const SPEED_SETTING_21C_CLEAR_TRUE: u32 = 0x011f_2ac4;
const SPEED_SETTING_21C_CLEAR_FALSE: u32 = 0x011f_2ad8;
const WATER_DAMAGE_SETTING_A: u32 = 0x011d_0460;
const WATER_DAMAGE_SETTING_B: u32 = 0x011d_1464;
const SPLASH_SETTING_A: u32 = 0x011d_1458;
const SPLASH_SETTING_B: u32 = 0x011d_12c0;
const SPLASH_SOUND_NAME: u32 = 0x0108_48a0;
const DROWNING_SOUND_NAME: u32 = 0x0108_48b8;
const BAD_INIT_ANIMATION_FORMAT: u32 = 0x0108_4840;
/// Constants (`float` or `double`) read through memory.
const UPDATE_TIME_LIMIT: u32 = 0x0108_48d0;
const SLEEP_UPDATE_TIME: u32 = 0x0108_4838;
const WATER_THRESHOLD_A: u32 = 0x0101_6408;
const WATER_THRESHOLD_B: u32 = 0x0104_ee98;
const WATER_THRESHOLD_C: u32 = 0x0107_3828;
const WATER_HEIGHT_UNSET: u32 = 0x0102_41b0;
const NO_WATER_HEIGHT: u32 = 0x0101_5f5c;
const WAKE_TIME_FACTOR: u32 = 0x0108_4898;
const HOURS_PER_DAY: u32 = 0x0103_5838;
const MILLISECONDS_PER_SECOND: u32 = 0x0101_7b70;
const DROWN_TIME_FACTOR: u32 = 0x0101_6ff0;
const PROCESS_TIMER_UNSET: u32 = 0x0102_31b0;
const DEAD_BODY_ALARM_RESET: u32 = 0x0101_7718;
const VOLUME_CAP: u32 = 0x0101_7718;
const CONTROLLER_VALUE_PLAYER: u32 = 0x011a_32d8;
const CONTROLLER_VALUE_OTHER: u32 = 0x0104_f4b0;
const CONTROLLER_VALUE_CREATURE: u32 = 0x0101_e2bc;
const CONTROLLER_LIMIT_B: u32 = 0x0101_a6b0;
const CONTROLLER_LIMIT_C: u32 = 0x0108_48b0;
const CONTROLLER_DIVISOR: u32 = 0x0101_e2c0;
const CONTROLLER_FACTOR_A: u32 = 0x0101_ffa0;
const CONTROLLER_OFFSET_A: u32 = 0x0102_0998;
const CONTROLLER_PUSH_DOWN: u32 = 0x0101_712c;
const CONTROLLER_PUSH_UP: u32 = 0x0102_caf8;
const MENU_ID_0X3F0: u32 = 0x3f0;
const ANIMATION_SIZE: u32 = 0x13c;
/// The compiler's byte table at `00888950` for the switch in `008885e0`
/// (kind `3` to `10`): `0` is the case that rescales the actor.
const SCALE_CASE_TABLE: [u8; 8] = [0, 1, 0, 1, 1, 0, 1, 0];
/// Speed formulas (`cdecl`).
const SPEED_FORMULA_00647F50: u32 = 0x0064_7f50;
const SPEED_FORMULA_00647F90: u32 = 0x0064_7f90;
const SPEED_FORMULA_00647FD0: u32 = 0x0064_7fd0;
const ACTOR_SPEED_VALUE_008A0C60: u32 = 0x008a_0c60;
/// Virtual slots of the actor.
const VSLOT_DESTROY: u32 = 0x10;
const VSLOT_UPDATE_HOOK_0X140: u32 = 0x140;
const VSLOT_GET_3D: u32 = 0x1d0;
const VSLOT_GET_POSITION_0X1F4: u32 = 0x1f4;
const VSLOT_FOLLOW_0X20C: u32 = 0x20c;
const VSLOT_KIND_0X214: u32 = 0x214;
const VSLOT_ANIMATION_FLAG_0X21C: u32 = 0x21c;
const VSLOT_IS_PROCESS_FLAG_0X22C: u32 = 0x22c;
const VSLOT_PACKAGE_TEST_0X230: u32 = 0x230;
const VSLOT_EVALUATE_PACKAGE_TEST_0X26C: u32 = 0x26c;
const VSLOT_0X2A0: u32 = 0x2a0;
const VSLOT_GET_HEADING_0X2BC: u32 = 0x2bc;
const VSLOT_FLAG_0X2E8: u32 = 0x2e8;
const VSLOT_DAMAGE_0X338: u32 = 0x338;
const VSLOT_ACTOR_FLAG_0X360: u32 = 0x360;
const VSLOT_ACTOR_FORM_TEST_0X38C: u32 = 0x38c;
const VSLOT_0X3AC: u32 = 0x3ac;
const VSLOT_UPDATE_0X44C: u32 = 0x44c;
const VSLOT_UPDATE_0X494: u32 = 0x494;
const VSLOT_UPDATE_0X4C8: u32 = 0x4c8;
const VSLOT_UPDATE_0X4CC: u32 = 0x4cc;
const VSLOT_0XD4: u32 = 0xd4;
/// Virtual slots of the process.
const PROCESS_VSLOT_STORED_DAY: u32 = 0x30;
const PROCESS_VSLOT_0XE0: u32 = 0xe0;
const PROCESS_VSLOT_0XE4: u32 = 0xe4;
const PROCESS_VSLOT_0XFC: u32 = 0xfc;
const PROCESS_VSLOT_0X128: u32 = 0x128;
const PROCESS_VSLOT_0X27C: u32 = 0x27c;
const PROCESS_VSLOT_0X294: u32 = 0x294;
const PROCESS_VSLOT_0X2F0: u32 = 0x2f0;
const PROCESS_VSLOT_0X2F4: u32 = 0x2f4;
const PROCESS_VSLOT_0X2F8: u32 = 0x2f8;
const PROCESS_VSLOT_0X2FC: u32 = 0x2fc;
const PROCESS_VSLOT_0X300: u32 = 0x300;
const PROCESS_VSLOT_0X35C: u32 = 0x35c;
const PROCESS_VSLOT_0X40C: u32 = 0x40c;
const PROCESS_VSLOT_0X41C: u32 = 0x41c;
const PROCESS_VSLOT_0X424: u32 = 0x424;
const PROCESS_VSLOT_0X428: u32 = 0x428;
const PROCESS_VSLOT_0X464: u32 = 0x464;
const PROCESS_VSLOT_0X48C: u32 = 0x48c;
const PROCESS_VSLOT_0X4B0: u32 = 0x4b0;
const PROCESS_VSLOT_0X4B4: u32 = 0x4b4;
const PROCESS_VSLOT_0X4BC: u32 = 0x4bc;
const PROCESS_VSLOT_0X5F8: u32 = 0x5f8;
const PROCESS_VSLOT_0X610: u32 = 0x610;
const PROCESS_VSLOT_0X618: u32 = 0x618;
const PROCESS_VSLOT_WAIT_TIME: u32 = 0x69c;
const PROCESS_VSLOT_MUZZLE_FLASH: u32 = 0x6b8;
const PROCESS_VSLOT_0X6FC: u32 = 0x6fc;
const PROCESS_VSLOT_0X704: u32 = 0x704;
const PROCESS_VSLOT_0X740: u32 = 0x740;
const PROCESS_VSLOT_0X744: u32 = 0x744;
const PROCESS_VSLOT_0X748: u32 = 0x748;
const PROCESS_VSLOT_0X74C: u32 = 0x74c;
const PROCESS_VSLOT_0X750: u32 = 0x750;
const PROCESS_VSLOT_0X760: u32 = 0x760;
const FACE_VSLOT_0XC8: u32 = 0xc8;
const FACE_VSLOT_0XD4: u32 = 0xd4;
const FACE_VSLOT_0XD8: u32 = 0xd8;
const MOVER_VSLOT_MODE: u32 = 0x1c;
const MAGIC_VSLOT_0X10: u32 = 0x10;
/// Functions.
const REFERENCE_DISABLE: u32 = 0x0057_4400;
const GET_DESIRED_PROCESS_LEVEL: u32 = 0x0093_34b0;
const PACKAGE_TYPE: u32 = 0x0096_11e0;
const PACKAGE_LOCATION_OF: u32 = 0x0055_b980;
const PACKAGE_RUNNING_OF: u32 = 0x0071_7e50;
const PACKAGE_IS_INTERRUPT: u32 = 0x0067_8610;
const PACKAGE_TEST_0X67A4F0: u32 = 0x0067_a4f0;
const GET_REF_PERSISTS: u32 = 0x0056_53d0;
const ACTOR_SKIP_UPDATE_TEST: u32 = 0x0044_0da0;
const ACTOR_ROTATE_TO_TRACK_TEST: u32 = 0x008a_40e0;
const ACTOR_RESET_FLOATS: u32 = 0x008b_bdb0;
const ACTOR_GET_LIFE_STATE: u32 = 0x004f_8960;
const ACTOR_GET_IN_COMBAT: u32 = 0x0049_3bb0;
const ACTOR_GET_CURRENT_WEAPON: u32 = 0x008a_1710;
const ACTOR_BLOCKED_TEST: u32 = 0x0043_7bd0;
const ACTOR_RANGE_TEST: u32 = 0x0089_4900;
const ACTOR_GET_BLOCKED: u32 = 0x0089_4d60;
const ACTOR_SET_BLOCK: u32 = 0x0089_4cc0;
const ACTOR_MOVER_FLAG_TEST: u32 = 0x005c_e8f0;
const ACTOR_GET_ESSENTIAL: u32 = 0x0087_f3d0;
const ACTOR_FLAG_TEST_0087F4A0: u32 = 0x0087_f4a0;
const ACTOR_TEST_0X87F5C0: u32 = 0x0087_f5c0;
const ACTOR_TEST_0X87F620: u32 = 0x0087_f620;
const ACTOR_TEST_0X87F570: u32 = 0x0087_f570;
const ACTOR_TEST_0X87F390: u32 = 0x0087_f390;
const ACTOR_TEST_0X8A6840: u32 = 0x008a_6840;
const ACTOR_TEST_0X5A2030: u32 = 0x005a_2030;
const ACTOR_IS_SURFACING: u32 = 0x008a_7a40;
const ACTOR_INITIATE_SURFACE_PACKAGE: u32 = 0x0089_84c0;
const ACTOR_SET_LIFE_STATE: u32 = 0x008a_1800;
const ACTOR_RESTORE_FULL_HEALTH: u32 = 0x008a_0960;
const ACTOR_TRIGGER_PAIN: u32 = 0x008a_7d50;
const ACTOR_TIMED_UPDATE: u32 = 0x008b_3230;
const ACTOR_END_OF_UPDATE: u32 = 0x008c_1470;
const ACTOR_UPDATE_MAGIC: u32 = 0x008c_3c40;
const ACTOR_EVALUATE_PACKAGE: u32 = 0x008a_6ce0;
const ACTOR_EVALUATE_FLAG: u32 = 0x0088_6b10;
const ACTOR_SET_MOVE_MODE: u32 = 0x008b_39f0;
const ACTOR_CLEAR_MOVE_MODE: u32 = 0x008b_3a80;
const ACTOR_GET_ENDURANCE: u32 = 0x008b_e7a0;
const ACTOR_GET_RADIATION_RESISTANCE_MULT: u32 = 0x008c_4330;
const ACTOR_MOVEMENT_STEP: u32 = 0x008b_cdf0;
const ACTOR_MOVER_TEST_0X9DCB90: u32 = 0x009d_cb90;
const ACTOR_MOVER_RESPONSE: u32 = 0x0089_85d0;
const ACTOR_PRE_ANIMATION_STEP: u32 = 0x0056_7050;
const ACTOR_AFTER_DEATH_ANIMATION: u32 = 0x0089_f580;
const ACTOR_LIGHTING_FORCE_TEST: u32 = 0x0049_38e0;
const ACTOR_QUEUE_ANIMATION_0X8B28C0: u32 = 0x008b_28c0;
const ACTOR_RAGDOLL_FACE_STEP: u32 = 0x0088_8970;
const ACTOR_DELETE_TEST: u32 = 0x008c_51f0;
const ACTOR_UNLOAD_PREPARE: u32 = 0x008c_2b60;
const ACTOR_DISPEL_ALL: u32 = 0x0082_4970;
const ACTOR_CAST_PERMANENT_MAGIC: u32 = 0x008c_26e0;
const ACTOR_MOVER_TEST: u32 = 0x009d_ccf0;
const MANAGER_REMOVE_ACTOR: u32 = 0x008d_0370;
const SAVE_LOAD_UNLOAD_FORM: u32 = 0x0084_9730;
const ACTOR_ROTATION_VECTOR: u32 = 0x0043_0830;
const POSITION_VECTOR_OF: u32 = 0x0043_6aa0;
const GET_FACE_ANIMATION_DATA: u32 = 0x008a_dcb0;
const GET_FORCE_NEXT_UPDATE: u32 = 0x0056_6950;
const TRIGGER_FOLLOWER_BARK: u32 = 0x008d_5cb0;
const PLAYER_STATE_TEST: u32 = 0x004e_af60;
const PLAYER_SLEEPING_OR_RESTING: u32 = 0x0094_df60;
const PLAYER_IN_COMBAT: u32 = 0x0095_3c50;
const PLAYER_GET_ANIMATION: u32 = 0x0095_0a60;
const PLAYER_TEST_0X524D10: u32 = 0x0052_4d10;
const PLAYER_TEST_0X9549A0: u32 = 0x0095_49a0;
const PLAYER_UPDATE_0X952290: u32 = 0x0095_2290;
const PLAYER_RADIATION_0X968730: u32 = 0x0096_8730;
const ADD_ACTOR_TO_TEMP_CHANGE_LIST: u32 = 0x0096_e870;
const TIME_NOW_LOW: u32 = 0x0052_6100;
const TES_IS_CELL_LOADED: u32 = 0x0045_11e0;
const CELL_TEST_0X450FF0: u32 = 0x0045_0ff0;
const CELL_CONTAINS_POSITION: u32 = 0x0055_0200;
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
const CELL_GET_WATER_TYPE: u32 = 0x0054_7770;
const MOVE_REF_TO_NEW_SPACE: u32 = 0x0057_3800;
const WATER_FORM_GET_DANGEROUS: u32 = 0x0058_0060;
const WATER_TYPE_COUNT: u32 = 0x0077_8950;
const TES_WATER_HEIGHT_AT: u32 = 0x0045_cbc0;
const SWIM_BREATH_TIME: u32 = 0x0064_8a10;
const DAMAGE_RATE: u32 = 0x0064_8a50;
const FTOL2: u32 = 0x00ec_62c0;
const EXTRA_GET_MERCHANT_CONTAINER: u32 = 0x0042_1400;
const MERCHANT_CONTAINER_TEST: u32 = 0x0056_ae60;
const EXTRA_LIST_DAY_CHANGED: u32 = 0x0042_e040;
const EXTRA_LIST_SET_DAY: u32 = 0x0041_d7b0;
const EXTRA_GET_DISMEMBERMENT: u32 = 0x0042_e8c0;
const EXTRA_GET_SOUND: u32 = 0x0041_8890;
const EXTRA_SET_SOUND: u32 = 0x0041_a800;
const EXTRA_GET_WEAPON_ATTACK_SOUND: u32 = 0x0041_8a00;
const EXTRA_SET_WEAPON_ATTACK_SOUND: u32 = 0x0041_a540;
const EXTRA_GET_CREATURE_AWAKE_SOUND: u32 = 0x0041_8940;
const EXTRA_SET_CREATURE_AWAKE_SOUND: u32 = 0x0041_a090;
const SOUND_HANDLE_INIT: u32 = 0x0041_a250;
const SOUND_HANDLE_ASSIGN: u32 = 0x0041_8900;
const SOUND_HANDLE_IS_PLAYING: u32 = 0x00ad_8930;
const SOUND_HANDLE_IS_VALID: u32 = 0x00ad_8ce0;
const SOUND_HANDLE_PLAY: u32 = 0x00ad_8830;
const SOUND_HANDLE_STOP: u32 = 0x00ad_88f0;
const SOUND_HANDLE_RELEASE: u32 = 0x00ad_8d10;
const SOUND_HANDLE_SET_VOLUME: u32 = 0x00ad_89e0;
const SOUND_HANDLE_SET_POSITION: u32 = 0x00ad_8b60;
const SOUND_HANDLE_SET_POSITION_PTR: u32 = 0x0068_a7d0;
const SOUND_HANDLE_SET_OBJECT_TO_FOLLOW: u32 = 0x00ad_8f20;
const SOUND_HANDLE_SET_0X4F15A0: u32 = 0x004f_15a0;
const AUDIO_INSTANCE: u32 = 0x0045_3a70;
const AUDIO_GET_SOUND_HANDLE_BY_FILENAME: u32 = 0x00ad_7480;
const AUDIO_GET_SOUND_HANDLE_BY_NAME: u32 = 0x00ad_7550;
const SOUND_FLAGS_OF: u32 = 0x005e_39b0;
const PICK_CREATURE_SOUND: u32 = 0x005f_92d0;
const PLAY_SOUND_BY_EDITOR_NAME: u32 = 0x0093_3270;
const PLAY_SPLASH_EFFECTS: u32 = 0x0062_eb50;
const CREATE_SPLASH_EFFECT: u32 = 0x0068_9210;
const RANDOM_ANGLE: u32 = 0x004a_4240;
const OBJECT_SIZE_0X87CE50: u32 = 0x0087_ce50;
const CLAMP_MIN_4DD150: u32 = 0x004d_d150;
const WEAPON_OBJECT_OF: u32 = 0x008b_ffa0;
const WEAPON_OBJECT_TEST: u32 = 0x0051_1840;
const WEAPON_SOUND_NAME: u32 = 0x0051_1840;
const WEAPON_SOUND_ITEM: u32 = 0x004e_75d0;
const WEAPON_TYPE_0X51F5F0: u32 = 0x0051_f5f0;
const WEAPON_TEST_0X524B40: u32 = 0x0052_4b40;
const GET_LAST_BOUND_WEAPON: u32 = 0x008d_85e0;
const MUZZLE_FLASH_UPDATE: u32 = 0x009b_b080;
const HIGH_PROCESS_FADE_UPDATE: u32 = 0x008f_ec10;
const IS_IN_MENU_MODE: u32 = 0x0070_2360;
const IS_MENU_ID_VISIBLE: u32 = 0x0070_2680;
const REFERENCE_FLAG_TEST_0X200000: u32 = 0x0057_6d30;
const REFERENCE_TEST_0X579670: u32 = 0x0057_9670;
const REFERENCE_FLAG_0X50D4A0: u32 = 0x0050_d4a0;
const SET_ANIMATION: u32 = 0x0057_2e50;
const BUILD_KF_FILE_LIST: u32 = 0x0056_6970;
const INIT_ANIMATION: u32 = 0x0048_ffd0;
const HAS_KF_FILES: u32 = 0x0047_fd90;
const GET_MODEL: u32 = 0x0057_15d0;
const ADD_SPECIAL_ANIMATIONS: u32 = 0x0049_0330;
const ANIMATION_CONSTRUCTOR: u32 = 0x0048_f810;
const OPERATOR_NEW_FN: u32 = 0x0040_1000;
const LOG_MESSAGE: u32 = 0x005b_5e40;
const STRING_COPY: u32 = 0x0040_6d30;
const STRING_FIND_LAST: u32 = 0x0040_ab30;
const ITEM_GROUP_INDEX: u32 = 0x0044_6390;
const ACTOR_ANIMATION_GROUP_TEST: u32 = 0x008a_6970;
const RESET_ANIMS_SHOULD_QUEUE: u32 = 0x008c_7aa0;
const TASK_QUEUE_INTERFACE: u32 = 0x0045_37b0;
const QUEUE_ACTOR_RESET_LOADED_ANIMS: u32 = 0x0087_b850;
const ANIMATION_GROUP_LOADED: u32 = 0x0049_4710;
const ANIMATION_GROUP_FIELD: u32 = 0x0043_01b0;
const ANIM_GROUP_GET_TYPE: u32 = 0x005f_2440;
const IS_POWER_ATTACK_ACTION: u32 = 0x005f_2670;
const IS_ATTACK_ACTION: u32 = 0x005f_2540;
const ANIMATION_FORCE_SECTION: u32 = 0x0049_55c0;
const ANIMATION_UPDATE: u32 = 0x0049_1180;
const ANIMATION_UPDATE_MOVEMENT: u32 = 0x0049_3900;
const ANIMATION_UPDATE_MOVEMENT_NO_WORLD_UPDATE: u32 = 0x0049_39d0;
const ANIMATION_UPDATE_SCENE_GRAPH_NO_CONTROLLER: u32 = 0x0049_3bd0;
const ANIMATION_SET_SPEED_SCALE: u32 = 0x0088_b030;
const ANIMATION_BLEND_OUT: u32 = 0x0049_94f0;
const ANIMATION_ENTRY_KIND: u32 = 0x0080_41a0;
const ANIMATION_ENTRY_TEST_0X4937E0: u32 = 0x0049_37e0;
const ANIMATION_FIELD_0X70F490: u32 = 0x0070_f490;
const ANIMATION_SPECIAL_IDLE_FREE: u32 = 0x0049_8910;
const ANIMATION_CLEAR_GROUP: u32 = 0x0049_6080;
const ANIMATION_FUNCTION_0X4964D0: u32 = 0x0049_64d0;
const ANIMATION_TIME: u32 = 0x0045_3700;
const ANIMATION_SCALE_VECTOR: u32 = 0x0049_4390;
const ANIMATION_SET_FRACTION: u32 = 0x0049_bdc0;
const LIST_FRACTION_AT: u32 = 0x004d_50e0;
const LIST_COUNT: u32 = 0x0055_9450;
const WORD_AT_START: u32 = 0x0055_9450;
const WORD_AT_0XC: u32 = 0x0084_e3a0;
const ENTRY_FLOAT_08100: u32 = 0x0050_8100;
const ENTRY_SET_VALUE_98ADB0: u32 = 0x0098_adb0;
const PICK_ANIMATIONS: u32 = 0x0089_5110;
const PICK_COUNT_OVERRIDE_BYTE: u32 = 0x0088_8960;
const SET_PICK_COUNT_OVERRIDE: u32 = 0x00c6_6230;
const CONSOLIDATE_SIM_ISLANDS: u32 = 0x0062_c430;
const DO_RAGDOLL_ANIM: u32 = 0x00c7_d630;
const RAGDOLL_TEST_0X4955A0: u32 = 0x0049_55a0;
const PROXY_OF: u32 = 0x004a_e750;
const SET_FLOAT_ON_TARGET: u32 = 0x0093_14d0;
const SET_MOTOR_ACTIVE: u32 = 0x00cb_8fe0;
const BYTE_STORE: u32 = 0x0062_2570;
const SUBOBJECT_SET_VALUE: u32 = 0x004a_3f10;
const FLAG_SET_OR_CLEAR: u32 = 0x0062_9670;
const BLEND_OBJECT_OF: u32 = 0x0043_fcd0;
const SET_BIP_TRANSFORM: u32 = 0x00c8_21b0;
const MATRIX_CONSTRUCT: u32 = 0x0068_15c0;
const MATRIX_MAKE_ROTATION: u32 = 0x004a_0c90;
const MATRIX_MAKE_X_ROTATION: u32 = 0x0052_4ac0;
const MATRIX_MAKE_Y_ROTATION: u32 = 0x0043_f850;
const MATRIX_MULTIPLY: u32 = 0x0043_f8d0;
const MULTIPLY_MATRIX_BY_RACE: u32 = 0x0056_fac0;
const SET_WORLD_POSITION: u32 = 0x0044_0460;
const SET_WORLD_ROTATION: u32 = 0x0043_fa80;
const VECTOR_TIMES_SCALAR: u32 = 0x004a_3760;
const VECTOR_TIMES_MATRIX: u32 = 0x004b_3ae0;
const VECTOR_PLUS_VECTOR: u32 = 0x0043_9e90;
const NI_POINT3_SET: u32 = 0x0041_6870;
const OBJECT_POSITION_0X8C: u32 = 0x0045_bb80;
const CONTROLLER_SET_0X520: u32 = 0x0088_b0a0;
const CONTROLLER_FLAG_0X88B0C0: u32 = 0x0088_b0c0;
const CONTROLLER_SPEED_0X88B0F0: u32 = 0x0088_b0f0;
const CONTROLLER_PITCH: u32 = 0x0056_7730;
const CONTROLLER_ROLL: u32 = 0x0056_7750;
const CONTROLLER_SET_TARGET: u32 = 0x00c6_d6b0;
const SET_VELOCITY_MODIFIER: u32 = 0x00c6_d4d0;
const CHARACTER_PROXY_POSITION: u32 = 0x0062_1a40;
const CONTROLLER_FUNCTION_0X4A3E90: u32 = 0x004a_3e90;
const INDEXED_ELEMENT_ADDRESS: u32 = 0x0056_0d30;
const FIND_NEXT_COLLISION_OBJECT: u32 = 0x00c8_02d0;
const COLLISION_RIGID_BODY: u32 = 0x006f_a820;
const RIGID_BODY_IS_ACTIVE: u32 = 0x0056_09b0;
const SHADOW_SCENE_NODE: u32 = 0x0045_0b80;
const UPDATE_OBJECT_LIGHTING: u32 = 0x00b5_d9f0;
const SHADOW_SCENE_NODE_UPDATE_0XB5F080: u32 = 0x00b5_f080;
const CELL_UPDATE_REFERENCE: u32 = 0x0054_a070;
const GET_LINKED_OBJECT: u32 = 0x0057_25f0;
const GET_HEADING: u32 = 0x008b_d7b0;
const SET_LOCATION_ON_REFERENCE_Z: u32 = 0x0057_5b70;
const GET_BIP_BLEND_VALUE: u32 = 0x00c8_2580;
const PROCESS_FLOAT_0X7DF1F0: u32 = 0x007d_f1f0;
const CLEAR_INTERPOLATORS: u32 = 0x0057_1560;
const SCOPE_OBJECT_0X11DF69E: u32 = 0x011d_f69e;
const GET_CURRENT_PROCESS_TYPE: u32 = 0x0093_1850;
const MOVER_VSLOT_0X20: u32 = 0x20;
const MOVER_VSLOT_0X24: u32 = 0x24;
const VSLOT_0X48: u32 = 0x48;
const VSLOT_0X4B4: u32 = 0x4b4;
const VSLOT_0X250: u32 = 0x250;
const ACTOR_GET_ANIMATION: u32 = 0x008b_70d0;
const SET_LOCATION_ON_REFERENCE: u32 = 0x0057_5830;
const MATRIX_TIMES_VECTOR: u32 = 0x004b_4500;
const ACTOR_FUNCTION_0X8B00C0: u32 = 0x008b_00c0;
const PROCESS_VSLOT_0X28C: u32 = 0x28c;
const GET_WORLD_BOUND: u32 = 0x0043_d450;
const VECTOR_MINUS_VECTOR: u32 = 0x0043_9ef0;
const VECTOR_LENGTH: u32 = 0x0045_7990;
const SKELETON_VSLOT_0X10: u32 = 0x10;
const LOD_DIVISOR: u32 = 0x007d_1d00;
const SKELETON_RADIUS: u32 = 0x006d_2c20;
const GET_ACCUMULATOR: u32 = 0x00b4_f5c0;
const ACCUMULATOR_TEST: u32 = 0x004b_6e70;
const ACCUMULATOR_UPDATE: u32 = 0x00b6_5f60;
const GLOBAL_BYTE_FN_525420: u32 = 0x0052_5420;
const TURRET_SET_FLAG_0X810610: u32 = 0x0081_0610;
const ACTOR_TURRET_BEHAVIOR: u32 = 0x0087_eeb0;
const GLOBAL_WORD_FN_87E9A0: u32 = 0x0087_e9a0;
const RAGDOLL_TEST_0X552490: u32 = 0x0055_2490;
const DISABLE_RAGDOLL_ANIM: u32 = 0x00c7_c150;
const ACTOR_SHAPE_0X931ED0: u32 = 0x0093_1ed0;
const SHAPE_RESULT_0X4A3A20: u32 = 0x004a_3a20;
const PENETRATION_UPDATE: u32 = 0x00ca_2ad0;
const DO_KNOCK_DOWN: u32 = 0x00c9_b670;
const FIND_COLLISION_OBJECT: u32 = 0x004a_de00;
const COLLISION_FUNCTION_0X6838B0: u32 = 0x0068_38b0;
const FIND_ENTRY: u32 = 0x0065_3270;
const ENTRY_TABLE_A: u32 = 0x0126_7e64;
const ENTRY_TABLE_B: u32 = 0x0126_83ec;
const BODY_LIST_0X7D6BB0: u32 = 0x007d_6bb0;
const BODY_SHAPE_0X517630: u32 = 0x0051_7630;
const WORLD_TEST_0XC8CCE0: u32 = 0x00c8_cce0;
const SET_MOTION: u32 = 0x00c6_a350;
const BODY_VSLOT_0X94: u32 = 0x94;
const BODY_FUNCTION_0X621480: u32 = 0x0062_1480;
const ADD_ACT_CON_TO_WORLD: u32 = 0x00c8_dac0;
const BODY_FLAGS_0X43B4F0: u32 = 0x0043_b4f0;
const BODY_KIND_0X43B4D0: u32 = 0x0043_b4d0;
const COLLISION_SET_0XC804D0: u32 = 0x00c8_04d0;
const BODY_ACTIVATE_0X561580: u32 = 0x0056_1580;
const NEXT_LIST_NODE: u32 = 0x0072_6070;
const ANGLE_0X5B9E80: u32 = 0x005b_9e80;
const ANGLE_0X5C53D0: u32 = 0x005c_53d0;
const QUATERNION_SET: u32 = 0x0055_32a0;
const NODE_VSLOT_0X44: u32 = 0x44;
const HANDLE_FLOAT_0X9A1260: u32 = 0x009a_1260;
const HANDLE_FLOAT_0X644A50: u32 = 0x0064_4a50;
const QUEUE_COLLISION_SYNC: u32 = 0x0087_ab50;
const COLLISION_SYNCHRONIZE: u32 = 0x00c6_c3d0;

// Translated from 00884990 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies bit `0x800` of the actor's flag word (`008846e0`) into the
/// `bSwimming` byte.
pub fn fn_00884990(e: &mut Engine, this: Ptr<Actor>) {
    let flags = e.call(GET_FLAG_WORD, &args![this]).u32();
    e.set(this, Actor::bSwimming, flags & 0x800 != 0);
}

// Translated from 008849c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the actor's animation says so (the object behind virtual
/// `+0x1e4` leads to an object that passes `005f4d60`, or whose kind from
/// `005f2420` is `0xe3`, `0xe5` or `0xf1` to `0xf4`), or else when the
/// character controller's state (`005c0880`) is 2.
pub fn fn_008849c0(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let animation = e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32();
    if animation != 0 && e.call(ANIMATION_LOOKUP, &args![animation, 1u32]).u32() != 0 {
        let entry = e.call(ANIMATION_LOOKUP, &args![animation, 1u32]).u32();
        let step = e.call(ANIMATION_STEP, &args![entry]).u32();
        if e.call(ANIMATION_TEST, &args![step]).bool() {
            return true;
        }
        let entry = e.call(ANIMATION_LOOKUP, &args![animation, 1u32]).u32();
        let step = e.call(ANIMATION_STEP, &args![entry]).u32();
        let kind = e.call(ANIMATION_KIND, &args![step]).u32();
        let index = kind.wrapping_sub(0xe3);
        // The compiler's switch: the byte table maps the kind to case 0
        // (answer true) or case 1 (carry on), compared as unsigned.
        if index <= 0x11 && ANIMATION_KIND_TABLE[index as usize] == 0 {
            return true;
        }
    }
    let controller = e.call(GET_CHAR_CONTROLLER, &args![this]).u32();
    controller != 0 && e.call(CONTROLLER_STATE, &args![controller]).i32() == 2
}

// Translated from 00884aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00930640` on the actor and, when it answers non-zero, plays the
/// jump sound (`ImpactMixer::PlayJump(actor, 0)`); returns the answer.
pub fn fn_00884aa0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let result = e.call(JUMP_TEST, &args![this]).u32();
    if result != 0 {
        e.call(PLAY_JUMP, &args![this, 0u32]);
    }
    result
}

// Translated from 00884ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the process's virtual `+0x22c` object has type `0x12` or
/// `0x15` (read by `0041ca90`); false without a process or object.
pub fn fn_00884ae0(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let process = e.get(this, Actor::pCurrentProcess);
    let object = if process.is_null() {
        0
    } else {
        e.vcall(process.addr(), PROCESS_VSLOT_OBJECT, &args![])
            .u32()
    };
    if object == 0 {
        return false;
    }
    e.call(OBJECT_TYPE, &args![object]).i32() == 0x12
        || e.call(OBJECT_TYPE, &args![object]).i32() == 0x15
}

// Translated from 00884b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsAllowedToSwim` (Xbox PDB). True when the flag byte at `+0x14c`
/// is set. Otherwise, when `00884ae0` holds: a combat controller (virtual
/// `+0x428`) that is neither accompanied (`0087f3b0`) nor fleeing, or that
/// fails `0047c850`, forbids it; else the answer is `0087f350`'s. When
/// `00884ae0` does not hold: creatures (virtual `+0x218`) may swim if the
/// process's `+0x22c` object passes `0067a560`; everything else gets
/// `0087f350`'s answer.
pub fn actor_is_allowed_to_swim(e: &mut Engine, this: Ptr<Actor>) -> bool {
    if e.get(this, Actor::bInWater) {
        return true;
    }
    if fn_00884ae0(e, this) {
        let controller = e
            .vcall(this.addr(), VSLOT_GET_COMBAT_CONTROLLER, &args![])
            .u32();
        if controller != 0 {
            if !e.call(SWIM_TEST_COMBAT, &args![this]).bool()
                && !e
                    .call(COMBAT_CONTROLLER_IS_FLEEING, &args![controller])
                    .bool()
            {
                return false;
            }
            if !e.call(COMBAT_CONTROLLER_TEST, &args![controller]).bool() {
                return false;
            }
        }
        return e.call(SWIM_TEST_FINAL, &args![this]).bool();
    }
    if e.vcall(this.addr(), VSLOT_IS_CREATURE, &args![]).bool() {
        let process = e.get(this, Actor::pCurrentProcess);
        let object = if process.is_null() {
            0
        } else {
            e.vcall(process.addr(), PROCESS_VSLOT_OBJECT, &args![])
                .u32()
        };
        object != 0 && e.call(SWIM_OBJECT_TEST, &args![object]).bool()
    } else {
        e.call(SWIM_TEST_FINAL, &args![this]).bool()
    }
}

// Translated from 00884c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `Actor::StopMoving` (Xbox PDB).
pub fn fn_00884c60(e: &mut Engine, this: Ptr<Actor>) {
    e.call(STOP_MOVING, &args![this]);
}

// Translated from 00884c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CachedValuesOwner::CalculateCachedWalkSpeed` (Xbox PDB name of slot
/// `+0x38`, which `01084144` fills with this function) as `Actor`
/// implements it. `this` is the `CachedValuesOwner` subobject: asks the
/// actor for its walk speed (virtual `+0x458`) and, when its process has a
/// cached-values block, stores the value there.
pub fn fn_00884c80(e: &mut Engine, this: Ptr<CachedValuesOwner>) -> f32 {
    let actor = actor_of_owner(this);
    let speed = e
        .vcall(actor.addr(), VSLOT_CALCULATE_WALK_SPEED, &args![])
        .f32();
    let process = e.get(actor, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        fn_00884ce0(e, process.cast(), speed);
    }
    speed
}

// Translated from 00884ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the walk speed in the process's cached values and marks it valid
/// (`iFlags |= 0x1000`).
pub fn fn_00884ce0(e: &mut Engine, this: Ptr<BaseProcess>, speed: f32) {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if !cached.is_null() {
        e.set(cached, CachedValues::fCachedWalkSpeed, speed);
        e.call(
            CACHED_VALUES_ADD_FLAGS,
            &args![cached, CACHED_WALK_SPEED_FLAG],
        );
    }
}

// Translated from 00884d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CachedValuesOwner::CalculateCachedRunSpeed` (Xbox PDB name of slot
/// `+0x3c`) as `Actor` implements it: like `00884c80` with the run speed
/// (virtual `+0x45c`).
pub fn fn_00884d20(e: &mut Engine, this: Ptr<CachedValuesOwner>) -> f32 {
    let actor = actor_of_owner(this);
    let speed = e
        .vcall(actor.addr(), VSLOT_CALCULATE_RUN_SPEED, &args![])
        .f32();
    let process = e.get(actor, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        fn_00884d80(e, process.cast(), speed);
    }
    speed
}

// Translated from 00884d80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the run speed in the process's cached values and marks it valid
/// (`iFlags |= 0x2000`).
pub fn fn_00884d80(e: &mut Engine, this: Ptr<BaseProcess>, speed: f32) {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if !cached.is_null() {
        e.set(cached, CachedValues::fCachedRunSpeed, speed);
        e.call(
            CACHED_VALUES_ADD_FLAGS,
            &args![cached, CACHED_RUN_SPEED_FLAG],
        );
    }
}

// Translated from 00884dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetWalkSpeed` (Xbox PDB): the cached walk speed when the
/// process has a cached-values block (`00884e30`, with the actor's
/// `CachedValuesOwner` subobject), else the actor's virtual `+0x458`.
pub fn actor_get_walk_speed(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        let owner = if this.is_null() {
            Ptr::NULL
        } else {
            Ptr::new(this.addr() + CACHED_VALUES_OWNER_OFFSET)
        };
        return fn_00884e30(e, process.cast(), owner);
    }
    e.vcall(this.addr(), VSLOT_CALCULATE_WALK_SPEED, &args![])
        .f32()
}

// Translated from 00884e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The process's walk speed: 0 without a cached-values block; the cached
/// value while its `0x1000` flag is set; otherwise the owner's virtual
/// `+0x38` (`CalculateCachedWalkSpeed`).
pub fn fn_00884e30(e: &mut Engine, this: Ptr<BaseProcess>, owner: Ptr<CachedValuesOwner>) -> f32 {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if cached.is_null() {
        return 0.0;
    }
    if !fn_00884e90(e, cached, CACHED_WALK_SPEED_FLAG) {
        e.get(cached, CachedValues::fCachedWalkSpeed)
    } else {
        e.vcall(owner.addr(), OWNER_VSLOT_WALK_SPEED, &args![])
            .f32()
    }
}

// Translated from 00884e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when none of the `mask` bits are set in `iFlags`.
pub fn fn_00884e90(e: &mut Engine, this: Ptr<CachedValues>, mask: u32) -> bool {
    e.get(this, CachedValues::iFlags) & mask == 0
}

// Translated from 00884eb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetRunSpeed` (Xbox PDB): like `GetWalkSpeed` with the run speed
/// (`00884f20`; fallback virtual `+0x45c`).
pub fn actor_get_run_speed(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        let owner = if this.is_null() {
            Ptr::NULL
        } else {
            Ptr::new(this.addr() + CACHED_VALUES_OWNER_OFFSET)
        };
        return fn_00884f20(e, process.cast(), owner);
    }
    e.vcall(this.addr(), VSLOT_CALCULATE_RUN_SPEED, &args![])
        .f32()
}

// Translated from 00884f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The process's run speed: 0 without a cached-values block; the cached
/// value while its `0x2000` flag is set; otherwise the owner's virtual
/// `+0x3c` (`CalculateCachedRunSpeed`).
pub fn fn_00884f20(e: &mut Engine, this: Ptr<BaseProcess>, owner: Ptr<CachedValuesOwner>) -> f32 {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if cached.is_null() {
        return 0.0;
    }
    if !fn_00884e90(e, cached, CACHED_RUN_SPEED_FLAG) {
        e.get(cached, CachedValues::fCachedRunSpeed)
    } else {
        e.vcall(owner.addr(), OWNER_VSLOT_RUN_SPEED, &args![]).f32()
    }
}

// Translated from 00884f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the actor has a process, passes the run-speed mask `0x2000` and
/// then the walk-speed mask `0x1000` to the process-level `00884940`.
pub fn fn_00884f80(e: &mut Engine, this: Ptr<Actor>) {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        e.call(
            PROCESS_FORWARD_FLAG_MASK,
            &args![process, CACHED_RUN_SPEED_FLAG],
        );
        e.call(
            PROCESS_FORWARD_FLAG_MASK,
            &args![process, CACHED_WALK_SPEED_FLAG],
        );
    }
}

// Translated from 00884fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetWalkSpeed` (Xbox PDB): stores the speed in the process's
/// cached values (`00884ce0`) when the actor has a process.
pub fn actor_set_walk_speed(e: &mut Engine, this: Ptr<Actor>, speed: f32) {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        fn_00884ce0(e, process.cast(), speed);
    }
}

// Translated from 00884ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetRunSpeed` (Xbox PDB): as `SetWalkSpeed`, through `00884d80`.
pub fn actor_set_run_speed(e: &mut Engine, this: Ptr<Actor>, speed: f32) {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        fn_00884d80(e, process.cast(), speed);
    }
}

/// Runs the two bounds virtuals into scratch vectors (upper corner at
/// `scratch`, lower corner 12 bytes on) and returns the pointers they
/// returned, upper corner first.
fn bounds_pointers(e: &mut Engine, actor: Ptr<Actor>, scratch: Ptr) -> (u32, u32) {
    let upper = e
        .vcall(actor.addr(), VSLOT_BOUNDS_UPPER, &args![scratch.addr()])
        .u32();
    let lower = e
        .vcall(
            actor.addr(),
            VSLOT_BOUNDS_LOWER,
            &args![scratch.addr() + 12],
        )
        .u32();
    (upper, lower)
}

// Translated from 00885020 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CachedValuesOwner::CalculateCachedRadius` (Xbox PDB name of slot
/// `+0x00`) as `Actor` implements it. The larger (`00404010`) of the
/// bounds' x and y extents, scaled by the reference scale, at least `32.0`,
/// halved. Stored in the process's cached values (`00885110`) when it has
/// them.
pub fn fn_00885020(e: &mut Engine, this: Ptr<CachedValuesOwner>) -> f32 {
    let actor = actor_of_owner(this);
    let (width, depth) = e.with_stack(24, |e, scratch| {
        bounds_pointers(e, actor, scratch);
        let upper_x = e.mem.f32(scratch.addr());
        let lower_x = e.mem.f32(scratch.addr() + 12);
        let upper_y = e.mem.f32(scratch.addr() + 4);
        let lower_y = e.mem.f32(scratch.addr() + 16);
        (
            (upper_x as f64 - lower_x as f64) as f32,
            (upper_y as f64 - lower_y as f64) as f32,
        )
    });
    let extent = e.call(FLOAT_HELPER_MAX, &args![width, depth]).f32();
    let scale = e.call(GET_SCALE, &args![actor]).f32();
    let extent = (scale as f64 * extent as f64) as f32;
    let minimum: f32 = e.global(MINIMUM_EXTENT);
    let extent = e.call(FLOAT_HELPER_MAX, &args![extent, minimum]).f32();
    let half: f64 = e.global(DOUBLE_HALF);
    let radius = (extent as f64 * half) as f32;
    let process = e.get(actor, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        fn_00885110(e, process.cast(), radius);
    }
    radius
}

// Translated from 00885110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the radius in the process's cached values and marks it valid
/// (`iFlags |= 1`).
pub fn fn_00885110(e: &mut Engine, this: Ptr<BaseProcess>, radius: f32) {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if !cached.is_null() {
        e.set(cached, CachedValues::fCachedRadius, radius);
        e.call(CACHED_VALUES_ADD_FLAGS, &args![cached, CACHED_RADIUS_FLAG]);
    }
}

// Translated from 00885140 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CachedValuesOwner::CalculateCachedWidth` (Xbox PDB name of slot
/// `+0x04`) as `Actor` implements it: the bounds' x extent times the
/// reference scale, stored in the cached values (`008851e0`) when the
/// process has them.
pub fn fn_00885140(e: &mut Engine, this: Ptr<CachedValuesOwner>) -> f32 {
    let actor = actor_of_owner(this);
    let extent = e.with_stack(24, |e, scratch| {
        bounds_pointers(e, actor, scratch);
        let upper = e.mem.f32(scratch.addr());
        let lower = e.mem.f32(scratch.addr() + 12);
        (upper as f64 - lower as f64) as f32
    });
    let scale = e.call(GET_SCALE, &args![actor]).f32();
    let width = (scale as f64 * extent as f64) as f32;
    let process = e.get(actor, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        fn_008851e0(e, process.cast(), width);
    }
    width
}

// Translated from 008851e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the width in the process's cached values and marks it valid
/// (`iFlags |= 2`).
pub fn fn_008851e0(e: &mut Engine, this: Ptr<BaseProcess>, width: f32) {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if !cached.is_null() {
        e.set(cached, CachedValues::fCachedWidth, width);
        e.call(CACHED_VALUES_ADD_FLAGS, &args![cached, CACHED_WIDTH_FLAG]);
    }
}

// Translated from 00885210 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CachedValuesOwner::CalculateCachedLength` (Xbox PDB name of slot
/// `+0x08`) as `Actor` implements it: like `00885140` with the y extent,
/// stored through `008852b0`.
pub fn fn_00885210(e: &mut Engine, this: Ptr<CachedValuesOwner>) -> f32 {
    let actor = actor_of_owner(this);
    let extent = e.with_stack(24, |e, scratch| {
        bounds_pointers(e, actor, scratch);
        let upper = e.mem.f32(scratch.addr() + 4);
        let lower = e.mem.f32(scratch.addr() + 16);
        (upper as f64 - lower as f64) as f32
    });
    let scale = e.call(GET_SCALE, &args![actor]).f32();
    let length = (scale as f64 * extent as f64) as f32;
    let process = e.get(actor, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        fn_008852b0(e, process.cast(), length);
    }
    length
}

// Translated from 008852b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the length in the process's cached values and marks it valid
/// (`iFlags |= 4`).
pub fn fn_008852b0(e: &mut Engine, this: Ptr<BaseProcess>, length: f32) {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if !cached.is_null() {
        e.set(cached, CachedValues::fCachedLength, length);
        e.call(CACHED_VALUES_ADD_FLAGS, &args![cached, CACHED_LENGTH_FLAG]);
    }
}

// Translated from 008852e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CachedValuesOwner::CalculateCachedForwardLength` (Xbox PDB name of slot
/// `+0x0c`) as `Actor` implements it: the upper bounds corner's y times the
/// reference scale, stored through `00885360`. Only the upper corner is
/// fetched.
pub fn fn_008852e0(e: &mut Engine, this: Ptr<CachedValuesOwner>) -> f32 {
    let actor = actor_of_owner(this);
    let upper_y = e.with_stack(12, |e, scratch| {
        e.vcall(actor.addr(), VSLOT_BOUNDS_UPPER, &args![scratch.addr()]);
        e.mem.f32(scratch.addr() + 4)
    });
    let scale = e.call(GET_SCALE, &args![actor]).f32();
    let length = (scale as f64 * upper_y as f64) as f32;
    let process = e.get(actor, Actor::pCurrentProcess);
    if !process.is_null() && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        fn_00885360(e, process.cast(), length);
    }
    length
}

// Translated from 00885360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the forward length in the process's cached values and marks it
/// valid (`iFlags |= 0x8000`).
pub fn fn_00885360(e: &mut Engine, this: Ptr<BaseProcess>, length: f32) {
    let cached = e.get(this, BaseProcess::pCachedValues);
    if !cached.is_null() {
        e.set(cached, CachedValues::fCachedForwardLength, length);
        e.call(
            CACHED_VALUES_ADD_FLAGS,
            &args![cached, CACHED_FORWARD_LENGTH_FLAG],
        );
    }
}

/// The reference scale times the height between the bounds corners
/// (`virtual +0x1dc` z minus `virtual +0x1d8` z), read through the pointers
/// the virtuals return.
fn scaled_bounds_height(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    let (upper, lower) = e.with_stack(24, |e, scratch| {
        let (upper, lower) = bounds_pointers(e, this, scratch);
        (e.mem.f32(upper + 8), e.mem.f32(lower + 8))
    });
    let difference = upper as f64 - lower as f64;
    let scale = e.call(GET_SCALE, &args![this]).f32();
    (scale as f64 * difference) as f32
}

// Translated from 008853a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetHeight` (Xbox PDB): the reference scale times the bounds'
/// z extent. With a process that passes `0045cd60` the height is cached in
/// the process (`fCachedActorHeight`, `0.0` meaning unset), computed and
/// stored when unset; otherwise it is computed every time.
pub fn actor_get_height(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() && e.call(CACHED_HEIGHT_BYPASS, &args![process]).u32() == 0 {
        let cached = fn_00885490(e, process.cast());
        let unset: f64 = e.global(DOUBLE_ZERO);
        if cached as f64 == unset {
            let height = scaled_bounds_height(e, this);
            fn_008854b0(e, process.cast(), height);
            return height;
        }
        return cached;
    }
    scaled_bounds_height(e, this)
}

// Translated from 00885490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::fCachedActorHeight` (Xbox PDB name of the field at
/// `+0x42c`): reads it.
pub fn fn_00885490(e: &mut Engine, this: Ptr<HighProcess>) -> f32 {
    e.get(this, HighProcess::fCachedActorHeight)
}

// Translated from 008854b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes `HighProcess::fCachedActorHeight` (Xbox PDB).
pub fn fn_008854b0(e: &mut Engine, this: Ptr<HighProcess>, height: f32) {
    e.set(this, HighProcess::fCachedActorHeight, height);
}

// Translated from 008854d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetSneakHeight` (Xbox PDB): the eye level (`008be940`) when
/// `use_eye_level` is set, else `GetHeight`, times `0.57`.
pub fn actor_get_sneak_height(e: &mut Engine, this: Ptr<Actor>, use_eye_level: bool) -> f32 {
    let base = if use_eye_level {
        e.call(GET_EYE_LEVEL, &args![this]).f32()
    } else {
        actor_get_height(e, this)
    };
    let factor: f64 = e.global(SNEAK_HEIGHT_FACTOR);
    (base as f64 * factor) as f32
}

// Translated from 00885520 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `threshold <= fn_00885560(point.z, cell)`: how far the actor
/// would be submerged at the point's height, compared with a fraction.
/// `point` points at a vector whose z (+8) is used.
pub fn fn_00885520(
    e: &mut Engine,
    this: Ptr<Actor>,
    point: Ptr,
    cell: Ptr,
    threshold: f32,
) -> bool {
    let z = e.mem.f32(point.addr() + 8);
    let depth = fn_00885560(e, this, z, cell);
    threshold as f64 <= depth as f64
}

// Translated from 00885560 (decompiled, FalloutNV.exe 1.4.0.525)
/// How much of the actor's height is under water at height `z`: `0` when
/// `z` is not below the water, otherwise `(water - z) / height` capped at
/// `1.0`. The water height is the given cell's (`005471e0`) unless the cell
/// is null or the actor's own cell, when it is the actor's relevant water
/// height (`0057b0a0`).
pub fn fn_00885560(e: &mut Engine, this: Ptr<Actor>, z: f32, cell: Ptr) -> f32 {
    let mut result = 0.0f32;
    let water = if !cell.is_null() && cell.addr() != e.call(GET_PARENT_CELL, &args![this]).u32() {
        e.call(CELL_GET_WATER_HEIGHT, &args![cell]).f32()
    } else {
        e.call(GET_RELEVANT_WATER_HEIGHT, &args![this]).f32()
    };
    if (z as f64) < water as f64 {
        let height = actor_get_height(e, this);
        result = ((water as f64 - z as f64) / height as f64) as f32;
        let one: f64 = e.global(DOUBLE_ONE);
        if result as f64 > one {
            result = 1.0;
        }
    }
    result
}

// Translated from 008855f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::AdjustSpeedForFollowing` (Xbox PDB). `speed` points at the
/// `float` movement speed being adjusted, `follow_distance` is the
/// package's follow value (turned into radii by the follow functions),
/// `distance` the current distance to the followed actor.
///
/// Does nothing unless the current package targets another actor in the
/// same area (the same cell when either cell is an interior, else the same
/// world space). The target's speed is scaled by the ratio of the two
/// references' scales. Closer than the mid radius the new speed is
/// `max(0, distance - near) / (mid - near)` times the lesser of the old
/// speed and the target's; from the far radius on, when the flag word has
/// all of `0x20f`, it is the target's speed times `1.25`; otherwise it is
/// `1 + (distance - mid) / (far - mid) * setting` times the target's speed.
/// The result is clamped between the old speed times two settings.
pub fn actor_adjust_speed_for_following(
    e: &mut Engine,
    this: Ptr<Actor>,
    speed: Ptr,
    follow_distance: f32,
    distance: f32,
) {
    // The result is stored by the game but never read.
    e.call(GET_CURRENT_PACKAGE, &args![this]);
    let target = Ptr::<Actor>::new(e.call(GET_CURRENT_PACKAGE_TARGET, &args![this]).u32());
    if target.is_null()
        || !e.vcall(target.addr(), VSLOT_IS_ACTOR, &args![]).bool()
        || target == this
    {
        return;
    }
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    let target_cell = e.call(GET_PARENT_CELL, &args![target]).u32();
    // The game also fetches this actor's world space here, unused.
    e.call(GET_WORLD_SPACE, &args![this]);
    let target_world_space = e.call(GET_WORLD_SPACE, &args![target]).u32();
    let interior = (target_cell != 0 && e.call(CELL_TEST_INTERIOR, &args![target_cell]).bool())
        || (cell != 0 && e.call(CELL_TEST_INTERIOR, &args![cell]).bool());
    let same_area = if interior {
        target_cell == cell
    } else {
        target_world_space == e.call(GET_WORLD_SPACE, &args![this]).u32()
    };
    if !same_area {
        return;
    }

    let near = e.call(FOLLOW_RADIUS_NEAR, &args![follow_distance]).f32();
    let mid = e.call(FOLLOW_RADIUS_MID, &args![follow_distance]).f32();
    let far = e
        .call(FOLLOW_RADIUS_MATCH_SPEED, &args![follow_distance])
        .f32();
    let target_speed = if e.vcall(target.addr(), VSLOT_IS_ACTOR, &args![]).bool() {
        e.call(GET_CURRENT_SPEED, &args![target]).f32()
    } else {
        0.0
    };
    let target_scale = e.call(GET_SCALE, &args![target]).f32() as f64;
    let own_scale = e.call(GET_SCALE, &args![this]).f32() as f64;
    let follow_speed = ((target_scale / own_scale) * target_speed as f64) as f32;

    let flags = e.call(GET_FLAG_WORD, &args![this]).u32();
    let all_flags = flags & FLAGS_MATCHING_PACE_MASK == FLAGS_MATCHING_PACE_MASK;
    let upper_factor = setting_value(
        e,
        if all_flags {
            SETTING_UPPER_FACTOR_ALL_FLAGS
        } else {
            SETTING_UPPER_FACTOR_OTHERWISE
        },
    );
    let upper_limit = (e.mem.f32(speed.addr()) as f64 * upper_factor as f64) as f32;
    let lower_factor = setting_value(
        e,
        if all_flags {
            SETTING_LOWER_FACTOR_ALL_FLAGS
        } else {
            SETTING_LOWER_FACTOR_OTHERWISE
        },
    );
    let lower_limit = (e.mem.f32(speed.addr()) as f64 * lower_factor as f64) as f32;

    if mid as f64 > distance as f64 {
        let nearer = e
            .call(
                FLOAT_HELPER_MAX,
                &args![0.0f32, (distance as f64 - near as f64) as f32],
            )
            .f32();
        let span = (mid as f64 - near as f64) as f32;
        let current = e.mem.f32(speed.addr());
        let capped = e
            .call(FLOAT_HELPER_MIN, &args![current, follow_speed])
            .f32();
        e.mem.set_f32(
            speed.addr(),
            ((nearer as f64 / span as f64) * capped as f64) as f32,
        );
    } else if far as f64 <= distance as f64 && all_flags {
        let catch_up: f64 = e.global(FOLLOW_CATCH_UP_FACTOR);
        e.mem
            .set_f32(speed.addr(), (follow_speed as f64 * catch_up) as f32);
    } else {
        let past_mid = (distance as f64 - mid as f64) as f32;
        let range = (far as f64 - mid as f64) as f32;
        let factor = setting_value(e, SETTING_INTERPOLATION_FACTOR);
        let ratio = (past_mid as f64 / range as f64) * factor as f64;
        e.mem
            .set_f32(speed.addr(), ((ratio + 1.0) * follow_speed as f64) as f32);
    }
    e.call(CLAMP_FLOAT, &args![speed, lower_limit, upper_limit]);
}

// Translated from 008858c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ModifyMoveModeForFollow` (Xbox PDB): adjusts the movement mode
/// word `mode` for a follow package. Without an actor target the mode is
/// unchanged. Closer than the walk radius with none of the target's low
/// four flag bits: bit `0x100` replaces `0x200`. Otherwise, if
/// `00884750` allows it and the target's flags `& 0x500 == 0x400` or has
/// bit `0x200`, the target is farther than the match-speed radius, or is
/// the player while `mode` has bit `0x800`: bit `0x200` replaces `0x100`.
/// The third stack word is not read.
pub fn actor_modify_move_mode_for_follow(
    e: &mut Engine,
    this: Ptr<Actor>,
    mode: u32,
    follow_distance: f32,
    _unused_3: f32,
) -> u32 {
    let target = e.call(GET_CURRENT_PACKAGE_TARGET, &args![this]).u32();
    if target == 0 || !e.vcall(target, VSLOT_IS_ACTOR, &args![]).bool() {
        return mode;
    }
    let distance = e
        .call(
            GET_DISTANCE_FROM_REFERENCE,
            &args![this, target, 0u32, 0u32],
        )
        .f32();
    let walk_radius = e.call(FOLLOW_RADIUS_WALK, &args![follow_distance]).f32();
    let match_radius = e
        .call(FOLLOW_RADIUS_MATCH_SPEED, &args![follow_distance])
        .f32();
    let target_flags = e.call(GET_FLAG_WORD, &args![target]).u32();
    if walk_radius as f64 > distance as f64 && target_flags & TARGET_FLAGS_HOLD_MASK == 0 {
        return (mode & !MODE_BIT_200) | MODE_BIT_100;
    }
    if e.call(FOLLOW_MODE_ALLOWED, &args![this]).bool()
        && (target_flags & TARGET_FLAGS_MODE_MASK == TARGET_FLAGS_MODE_VALUE
            || (match_radius as f64) < distance as f64
            || (target == e.global::<u32>(PLAYER_CHARACTER) && mode & MODE_BIT_800 != 0)
            || target_flags & TARGET_FLAGS_BIT_200 != 0)
    {
        return (mode & !MODE_BIT_100) | MODE_BIT_200;
    }
    mode
}

// Translated from 008859e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True unless the form behind `reference` (`BGSSaveFormBuffer::GetForm`)
/// has form type `0x15`, `0x27` or `0x30` and is missing flag
/// `0x20000000` (`00885a30`). `this` is not read (callers pass the player
/// singleton).
pub fn fn_008859e0(e: &mut Engine, _unused_this: Ptr, reference: Ptr) -> bool {
    let form = e.call(SAVE_FORM_BUFFER_GET_FORM, &args![reference]).u32();
    let form_type = e.call(FORM_TYPE, &args![form]).u32();
    if form_type == 0x15 || form_type == 0x27 || form_type == 0x30 {
        let form = e.call(SAVE_FORM_BUFFER_GET_FORM, &args![reference]).u32();
        return fn_00885a30(e, Ptr::new(form));
    }
    true
}

// Translated from 00885a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `TESForm::iFormFlags` (Xbox PDB, +0x08) has bit `0x20000000`.
pub fn fn_00885a30(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 8) & FORM_FLAG_20000000 != 0
}

// Translated from 00885a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::CalculateWalkSpeed` (Xbox PDB name of the slot at `+0x454`; the
/// Actor vtable holds this function at `+0x458`). Asks `00647d10` for the
/// speed from the actor's value owner (`Actor + 0xa4`), the values of the
/// process's item (virtual `+0x148`) and of the worn armor in slot 2, and
/// four flags; multiplied by the combat controller's float at `+0xd8`
/// (`fn_00885bd0`) when the actor has a controller.
pub fn actor_calculate_walk_speed(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    let speed_flag = e.call(SPEED_FLAG_TEST, &args![this]).bool();
    let mut process_flag_clear = true;
    let mut item = 0u32;
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        process_flag_clear = !e.vcall(process.addr(), PROCESS_VSLOT_FLAG, &args![]).bool();
        item = e.vcall(process.addr(), PROCESS_VSLOT_ITEM, &args![]).u32();
    }
    let armor = e
        .call(GET_ARMOR_BEING_WORN, &args![this, SPEED_ARMOR_SLOT])
        .u32();
    let item_value = if item != 0 {
        e.call(ITEM_VALUE, &args![item]).u32()
    } else {
        0
    };
    let armor_value = if armor != 0 {
        e.call(ITEM_VALUE, &args![armor]).u32()
    } else {
        0
    };
    if armor != 0 {
        e.call(ITEM_RELEASE, &args![armor, 1u32]);
    }
    let value_owner = if this.is_null() {
        0
    } else {
        this.addr() + ACTOR_VALUE_OWNER_OFFSET
    };
    let flag_0x358 = e.vcall(this.addr(), VSLOT_FLAG_0X358, &args![]).bool();
    let creature = e.vcall(this.addr(), VSLOT_IS_CREATURE, &args![]).bool();
    let mut speed = e
        .call(
            CALCULATE_WALK_SPEED_FORMULA,
            &args![
                value_owner,
                item_value,
                armor_value,
                speed_flag,
                process_flag_clear,
                creature,
                flag_0x358
            ],
        )
        .f32();
    let controller = e
        .vcall(this.addr(), VSLOT_GET_COMBAT_CONTROLLER, &args![])
        .u32();
    if controller != 0 {
        let multiplier = fn_00885bd0(e, Ptr::new(controller));
        speed = (multiplier as f64 * speed as f64) as f32;
    }
    speed
}

// Translated from 00885bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the float at `+0xd8` of the combat controller (the Xbox PDB has
/// `CombatController::fWalkSpeedMult` at `+0xe8`; the PC layout is 0x10
/// shorter).
pub fn fn_00885bd0(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xd8)
}

// Translated from 00885bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::CalculateRunSpeed` (Xbox PDB name of the slot at `+0x458`; the
/// Actor vtable holds this function at `+0x45c`). Like
/// `CalculateWalkSpeed` with `00647f00` (the flag arguments in another
/// order, and the `004997b0` test only as an argument), multiplied by the
/// combat controller's float at `+0xdc` (`fn_00885d70`), then passed
/// through `BGSEntryPoint::HandleEntryPoint(0x2a, actor, &speed)`.
pub fn actor_calculate_run_speed(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    let mut process_flag_clear = true;
    let mut item = 0u32;
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        process_flag_clear = !e.vcall(process.addr(), PROCESS_VSLOT_FLAG, &args![]).bool();
        item = e.vcall(process.addr(), PROCESS_VSLOT_ITEM, &args![]).u32();
    }
    let armor = e
        .call(GET_ARMOR_BEING_WORN, &args![this, SPEED_ARMOR_SLOT])
        .u32();
    let item_value = if item != 0 {
        e.call(ITEM_VALUE, &args![item]).u32()
    } else {
        0
    };
    let armor_value = if armor != 0 {
        e.call(ITEM_VALUE, &args![armor]).u32()
    } else {
        0
    };
    if armor != 0 {
        e.call(ITEM_RELEASE, &args![armor, 1u32]);
    }
    let value_owner = if this.is_null() {
        0
    } else {
        this.addr() + ACTOR_VALUE_OWNER_OFFSET
    };
    let flag_0x358 = e.vcall(this.addr(), VSLOT_FLAG_0X358, &args![]).bool();
    let speed_flag = e.call(SPEED_FLAG_TEST, &args![this]).bool();
    let creature = e.vcall(this.addr(), VSLOT_IS_CREATURE, &args![]).bool();
    let mut speed = e
        .call(
            CALCULATE_RUN_SPEED_FORMULA,
            &args![
                value_owner,
                item_value,
                armor_value,
                process_flag_clear,
                creature,
                speed_flag,
                flag_0x358
            ],
        )
        .f32();
    let controller = e
        .vcall(this.addr(), VSLOT_GET_COMBAT_CONTROLLER, &args![])
        .u32();
    if controller != 0 {
        let multiplier = fn_00885d70(e, Ptr::new(controller));
        speed = (multiplier as f64 * speed as f64) as f32;
    }
    e.with_stack(4, |e, slot| {
        e.mem.set_f32(slot.addr(), speed);
        e.call(HANDLE_ENTRY_POINT, &args![0x2au32, this, slot]);
        e.mem.f32(slot.addr())
    })
}

// Translated from 00885d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the float at `+0xdc` of the combat controller (the Xbox PDB has
/// `CombatController::fRunSpeedMult` at `+0xec`; the PC layout is 0x10
/// shorter).
pub fn fn_00885d70(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xdc)
}

/// The common body of `00885d90` and `00885ed0`: the process virtuals
/// `+0x454` (negated) and `+0x148` give the flag and the item, the worn
/// armor (slot 2) and the item give two values, and `formula` (`cdecl`, six
/// words: the `ActorValueOwner`, the two values, the flag, `IsCreature`,
/// the `+0x358` flag) gives the speed, a `float` in ST0.
fn weighted_speed(e: &mut Engine, this: Ptr<Actor>, formula: u32) -> f32 {
    let mut process_flag_clear = true;
    let mut item = 0u32;
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        process_flag_clear = !e.vcall(process.addr(), PROCESS_VSLOT_FLAG, &args![]).bool();
        item = e.vcall(process.addr(), PROCESS_VSLOT_ITEM, &args![]).u32();
    }
    let armor = e
        .call(GET_ARMOR_BEING_WORN, &args![this, SPEED_ARMOR_SLOT])
        .u32();
    let item_value = if item != 0 {
        e.call(ITEM_VALUE, &args![item]).u32()
    } else {
        0
    };
    let armor_value = if armor != 0 {
        e.call(ITEM_VALUE, &args![armor]).u32()
    } else {
        0
    };
    if armor != 0 {
        e.call(ITEM_RELEASE, &args![armor, 1u32]);
    }
    let value_owner = if this.is_null() {
        0
    } else {
        this.addr() + ACTOR_VALUE_OWNER_OFFSET
    };
    let flag_0x358 = e.vcall(this.addr(), VSLOT_FLAG_0X358, &args![]).bool();
    let creature = e.vcall(this.addr(), VSLOT_IS_CREATURE, &args![]).bool();
    e.call(
        formula,
        &args![
            value_owner,
            item_value,
            armor_value,
            process_flag_clear,
            creature,
            flag_0x358
        ],
    )
    .f32()
}

// Translated from 00885d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// A speed from the actor values through `00647f50` (the flag arguments as
/// in `CalculateWalkSpeed`, without its `004997b0` test and its
/// post-processing). The engine map has no name for it.
pub fn fn_00885d90(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    weighted_speed(e, this, SPEED_FORMULA_00647F50)
}

// Translated from 00885ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The twin of `00885d90` with `00647f90` as the formula.
pub fn fn_00885ed0(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    weighted_speed(e, this, SPEED_FORMULA_00647F90)
}

// Translated from 00886010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Zero when the `+0x358` virtual says so; otherwise `00647fd0` applied to
/// the actor values and the actor's `008a0c60` value (a `float` in ST0).
pub fn fn_00886010(e: &mut Engine, this: Ptr<Actor>) -> f32 {
    if e.vcall(this.addr(), VSLOT_FLAG_0X358, &args![]).bool() {
        return 0.0;
    }
    let value = e.call(ACTOR_SPEED_VALUE_008A0C60, &args![this]).f32();
    let value_owner = if this.is_null() {
        0
    } else {
        this.addr() + ACTOR_VALUE_OWNER_OFFSET
    };
    e.call(SPEED_FORMULA_00647FD0, &args![value_owner, value])
        .f32()
}

// Translated from 00886080 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetCurrentPackageLocation` (Xbox PDB): the package location of
/// the object the process virtual `+0x27c` returns, or 0 without a process
/// or object (`0055b980` reads it).
pub fn actor_get_current_package_location(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() {
        return 0;
    }
    let object = e.vcall(process.addr(), PROCESS_VSLOT_0X27C, &args![]).u32();
    if object == 0 {
        return 0;
    }
    e.call(PACKAGE_LOCATION_OF, &args![object]).u32()
}

// Translated from 008860d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Moves the actor to the process level `MobileObject::GetDesiredProcessLevel`
/// (`009334b0`) asks for. True when the actor stays (the level already is the
/// desired one, or it is handled and kept); false when the process is
/// missing or the reference was deleted or unloaded. The meanings of the
/// process virtual `+0x610` values 5 and 6 and of the actor virtuals
/// `+0x240` to `+0x24c` are not confirmed.
pub fn fn_008860d0(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() {
        return false;
    }
    let current_level = e.call(CACHED_HEIGHT_BYPASS, &args![process]).u32();
    let desired_level = e.call(GET_DESIRED_PROCESS_LEVEL, &args![this]).u32();
    if current_level == desired_level {
        return true;
    }
    if current_level == 0 && e.vcall(process.addr(), PROCESS_VSLOT_0X610, &args![]).u32() != 0 {
        if e.vcall(process.addr(), PROCESS_VSLOT_0X610, &args![]).u32() == 6 {
            if !this.is_null() {
                e.vcall(this.addr(), VSLOT_DESTROY, &args![1u32]);
            }
            return false;
        }
        if e.vcall(process.addr(), PROCESS_VSLOT_0X610, &args![]).u32() == 5 {
            e.call(REFERENCE_DISABLE, &args![this]);
            return false;
        }
    }
    if desired_level != 0 {
        let package = e.vcall(process.addr(), PROCESS_VSLOT_0X27C, &args![]).u32();
        if package != 0
            && e.call(PACKAGE_TYPE, &args![package]).u32() == 8
            && e.vcall(process.addr(), PROCESS_VSLOT_0X128, &args![]).u32()
                == e.mem.u32(PLAYER_CHARACTER)
        {
            let mover = e.get(this, Actor::pActorMover);
            if !e.call(ACTOR_MOVER_TEST, &args![mover]).bool() {
                e.call(STOP_MOVING, &args![this]);
            }
        }
    }
    let mut unload = false;
    match desired_level {
        0 => {
            e.vcall(this.addr(), 0x240, &args![]);
        }
        1 => {
            e.vcall(this.addr(), 0x24c, &args![]);
        }
        2 => {
            e.vcall(this.addr(), 0x248, &args![]);
            unload = !e.call(GET_REF_PERSISTS, &args![this]).bool();
        }
        3 => {
            e.vcall(this.addr(), 0x244, &args![]);
            unload = !e.call(GET_REF_PERSISTS, &args![this]).bool();
        }
        _ => {}
    }
    if !unload {
        return true;
    }
    if !e.call(ACTOR_DELETE_TEST, &args![this]).bool()
        && e.call(GET_PARENT_CELL, &args![this]).u32() != 0
    {
        e.call(ACTOR_UNLOAD_PREPARE, &args![this]);
        e.call(ACTOR_DISPEL_ALL, &args![this.addr() + 0x94]);
        e.call(ACTOR_CAST_PERMANENT_MAGIC, &args![this, 0u32]);
        let manager = e.mem.u32(MANAGER_011DE45C);
        e.call(MANAGER_REMOVE_ACTOR, &args![manager, this]);
        let save_load = e.mem.u32(SAVE_LOAD_GAME);
        e.call(SAVE_LOAD_UNLOAD_FORM, &args![save_load, this, 0u32]);
    }
    if !this.is_null() {
        e.vcall(this.addr(), VSLOT_DESTROY, &args![1u32]);
    }
    false
}

// Translated from 00886360 (decompiled, FalloutNV.exe 1.4.0.525)
/// A per-frame step of an actor's process: timers, the merchant container
/// reset on a new day, the package evaluation, the dead-body alarm and the
/// check that moves a reference out of an unloaded cell. `delta` is the
/// frame time. It returns at once when `00440da0` says so or the actor is
/// not marked `bProcessMe`. Not named in the engine map; the meanings of most
/// callees are not confirmed.
pub fn fn_00886360(e: &mut Engine, this: Ptr<Actor>, delta: f32) {
    if e.call(ACTOR_SKIP_UPDATE_TEST, &args![this]).bool() {
        return;
    }
    if !e.get(this, Actor::bProcessMe) {
        return;
    }
    let player = e.mem.u32(PLAYER_CHARACTER);
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF69F]);
    e.vcall(this.addr(), VSLOT_UPDATE_HOOK_0X140, &args![]);
    let rotate = e.call(ACTOR_ROTATE_TO_TRACK_TEST, &args![this, 0u32]).u8();
    // Actor::bShouldRotateToTrack (Xbox PDB) +0x15d, stored as the raw byte.
    e.mem.set_u8(this.addr() + 0x15d, rotate);
    let step = e
        .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
        .f32();
    let step = vats_scaled(e, this, step);
    let action_timer = e.get(this, Actor::fTimeronAction);
    if action_timer > 0.0 {
        e.set(
            this,
            Actor::fTimeronAction,
            (action_timer as f64 - step as f64) as f32,
        );
    } else {
        e.set(this, Actor::iActionValue, 0);
    }
    let process = e.call(GET_PROCESS, &args![this]).u32();
    if process != 0
        && !e
            .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
            .bool()
    {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        let stored_day = e.vcall(process, PROCESS_VSLOT_STORED_DAY, &args![]).u32();
        let day = e.call(CALENDAR_GET_DAY, &args![CALENDAR]).u8() as i8 as i32 as u32;
        if stored_day == day {
            // Actor::bContainerReset (Xbox PDB) +0x175.
            e.mem.set_u8(this.addr() + 0x175, 0);
        } else {
            if e.mem.u8(this.addr() + 0x175) == 0 {
                let month = e.call(CALENDAR_GET_MONTH, &args![CALENDAR]).u32();
                let first = e.call(GLOBAL_WORD_ADDRESS, &args![MONTH_SETTING_A]).u32();
                let matches = month == e.mem.u32(first) || {
                    let second = e.call(GLOBAL_WORD_ADDRESS, &args![MONTH_SETTING_B]).u32();
                    month == e.mem.u32(second)
                };
                if matches {
                    let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
                    let merchant = e.call(EXTRA_GET_MERCHANT_CONTAINER, &args![lists]).u32();
                    if merchant != 0 {
                        let flag = e.call(MERCHANT_CONTAINER_TEST, &args![merchant]).bool();
                        e.vcall(merchant, 0x208, &args![!flag]);
                    }
                }
            }
            e.mem.set_u8(this.addr() + 0x175, 1);
        }
    }
    let process = e.call(GET_PROCESS, &args![this]).u32();
    let level = e.call(PROCESS_GET_LEVEL, &args![process]).u32();
    if level != 0 || e.call(PLAYER_SLEEPING_OR_RESTING, &args![player]).bool() {
        let mut time = delta;
        if e.call(PLAYER_SLEEPING_OR_RESTING, &args![player]).bool() {
            time = e.global::<f32>(SLEEP_UPDATE_TIME);
        } else if e.call(ACTOR_GET_IN_COMBAT, &args![this]).bool() {
            time = 0.0;
        }
        e.call(ACTOR_TIMED_UPDATE, &args![this, time]);
        e.call(ACTOR_UPDATE_MAGIC, &args![this, 0u32, 0u32]);
    }
    let not_processed = e
        .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
        .bool();
    let dead = e.get(this, Actor::bDeadFlag);
    if not_processed
        || dead
        || e.mem.u8(PROCESSING_ENABLED) == 0
        || e.call(ACTOR_GET_LIFE_STATE, &args![this]).u32() == 6
    {
        if e.vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
            .bool()
        {
            dead_body_step(e, this, player, step);
        }
    } else {
        package_step(e, this, player, delta);
    }
    let mode = e.call(GLOBAL_WORD_ADDRESS, &args![MODE_SETTING]).u32();
    if e.mem.u32(mode) == 1 {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        if e.call(PROCESS_GET_LEVEL, &args![process]).u32() == 0 {
            let object = e.mem.u32(COMBAT_OBJECT_POINTER);
            if !e.call(COMBAT_CONTROLLER_TEST, &args![object]).bool() {
                e.call(ACTOR_UPDATE_MAGIC, &args![this, 0u32, 0u32]);
            }
        }
    }
    if this.addr() != player && e.call(SAVE_FORM_BUFFER_GET_FORM, &args![this]).u32() != 0 {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        if e.call(PROCESS_GET_LEVEL, &args![process]).u32() != 0 {
            move_out_of_cell(e, this);
        }
    }
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF69F]);
}

/// The part of `00886360` for an actor that is dead or not processed (its
/// first branch): while the virtual `+0x22c` holds, either the dead-body
/// timers run down and a killed-by-the-player actor tells the player's
/// process, or an essential-flag actor is handled; then the actor may be
/// queued on the temporary change list.
fn dead_body_step(e: &mut Engine, this: Ptr<Actor>, player: u32, step: f32) {
    let state = e.call(ACTOR_GET_LIFE_STATE, &args![this]).u32();
    if state != 1 {
        let process = e.get(this, Actor::pCurrentProcess);
        e.vcall(process.addr(), 0x28, &args![]);
        let body_timer = e.get(this, Actor::fCheckMyDeadBodyTimer);
        e.set(
            this,
            Actor::fCheckMyDeadBodyTimer,
            (body_timer as f64 - step as f64) as f32,
        );
        let alarm = e.get(this, Actor::fDeadBodyAlarm);
        let alarm = (alarm as f64 - step as f64) as f32;
        e.set(this, Actor::fDeadBodyAlarm, alarm);
        if alarm <= 0.0 {
            if e.get(this, Actor::pMyKiller).addr() == player {
                let player_process = e.call(GET_PROCESS, &args![player]).u32();
                let seen = e.vcall(player_process, 0xf8, &args![player]).u32();
                if seen == 0 {
                    let player_process = e.call(GET_PROCESS, &args![player]).u32();
                    let position = e
                        .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
                        .u32();
                    let x = e.mem.u32(position);
                    let y = e.mem.u32(position + 4);
                    let z = e.mem.u32(position + 8);
                    e.vcall(
                        player_process,
                        PROCESS_VSLOT_0XFC,
                        &args![player, x, y, z, 1u32, 4u32, 0u32],
                    );
                }
            }
            let reset = e.global::<f32>(DEAD_BODY_ALARM_RESET);
            e.set(this, Actor::fDeadBodyAlarm, reset);
        }
    } else if e.call(ACTOR_GET_LIFE_STATE, &args![this]).u32() == 1 {
        let setting = e
            .call(SETTING_VALUE_POINTER, &args![ESSENTIAL_SETTING])
            .u32();
        if e.mem.u8(setting) != 0 && e.call(ACTOR_GET_ESSENTIAL, &args![this]).bool() {
            e.vcall(this.addr(), 0x324, &args![0u32, 0u32, 1u32]);
        } else {
            let process = e.get(this, Actor::pCurrentProcess);
            e.vcall(process.addr(), 0x10, &args![this]);
        }
    }
    if !e.call(ACTOR_GET_ESSENTIAL, &args![this]).bool()
        && e.call(ACTOR_FLAG_TEST_0087F4A0, &args![this]).bool()
    {
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        if !e
            .call(TES_IS_CELL_LOADED, &args![TES_POINTER, cell, 0u32])
            .bool()
        {
            let process = e.call(GET_PROCESS, &args![this]).u32();
            if process != 0 {
                let wait = e.vcall(process, PROCESS_VSLOT_WAIT_TIME, &args![]).f32();
                if wait != 0.0 {
                    let wait = e.vcall(process, PROCESS_VSLOT_WAIT_TIME, &args![]).f32();
                    let now_low = e.call(TIME_NOW_LOW, &args![]).u32();
                    let now = e.call(CALENDAR_GET_TIME, &args![CALENDAR]).u32();
                    if (now_low as f64 + wait as f64) < now as f64 {
                        e.call(ADD_ACTOR_TO_TEMP_CHANGE_LIST, &args![PROCESS_LISTS, this]);
                    }
                }
            }
        }
    }
}

/// The part of `00886360` for an actor that is alive and processed (its
/// second branch): the movement-mover check, the day change, the package
/// evaluation and the process's own update.
fn package_step(e: &mut Engine, this: Ptr<Actor>, player: u32, delta: f32) {
    let setting = e.call(SETTING_VALUE_POINTER, &args![MOVER_SETTING]).u32();
    if e.mem.u8(setting) != 0 {
        let process = process_of(e, this);
        if !e.vcall(process, PROCESS_VSLOT_0X35C, &args![]).bool() {
            let mover = e.get(this, Actor::pActorMover);
            if !mover.is_null() && e.call(ACTOR_MOVER_TEST_0X9DCB90, &args![mover]).bool() {
                e.call(ACTOR_MOVER_RESPONSE, &args![this]);
            }
        }
    }
    let process = process_of(e, this);
    let stored_day = e.vcall(process, PROCESS_VSLOT_STORED_DAY, &args![]).u8();
    let day = e.call(CALENDAR_GET_DAY, &args![CALENDAR]).u8();
    if day as i8 != stored_day as i8 {
        let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
        e.call(EXTRA_LIST_DAY_CHANGED, &args![lists]);
    }
    if e.vcall(this.addr(), VSLOT_EVALUATE_PACKAGE_TEST_0X26C, &args![])
        .bool()
    {
        let flag = e.call(ACTOR_EVALUATE_FLAG, &args![this]).u8();
        e.call(ACTOR_EVALUATE_PACKAGE, &args![this, 1u32, flag as u32]);
    }
    if !e
        .vcall(this.addr(), VSLOT_PACKAGE_TEST_0X230, &args![])
        .bool()
    {
        let mut update = false;
        if e.mem.u8(INTERRUPT_UPDATES_ENABLED) == 0 {
            let process = process_of(e, this);
            let package = e.vcall(process, PROCESS_VSLOT_OBJECT, &args![]).u32();
            if package != 0 {
                let process = process_of(e, this);
                let package = e.vcall(process, PROCESS_VSLOT_OBJECT, &args![]).u32();
                update = e.call(PACKAGE_IS_INTERRUPT, &args![package]).bool();
            }
        } else {
            update = true;
        }
        if update {
            let process = process_of(e, this);
            let package = e.vcall(process, PROCESS_VSLOT_OBJECT, &args![]).u32();
            let mut skip = false;
            if package != 0 {
                let process = process_of(e, this);
                let package = e.vcall(process, PROCESS_VSLOT_OBJECT, &args![]).u32();
                skip = e.call(OBJECT_TYPE, &args![package]).u32() == 0x1b;
            }
            if !skip {
                let process = process_of(e, this);
                if delta == 0.0 || this.addr() == player {
                    e.vcall(process, 0x10, &args![this]);
                } else {
                    e.vcall(process, 0xc, &args![this, delta]);
                }
            }
        }
    }
    let day = e.call(CALENDAR_GET_DAY, &args![CALENDAR]).u8() as u32;
    let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
    e.call(EXTRA_LIST_SET_DAY, &args![lists, day]);
}

// Translated from 00886b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns `Actor::bResetAI` (Xbox PDB, +0x146 on PC).
pub fn fn_00886b10(e: &mut Engine, this: Ptr<Actor>) -> bool {
    e.get(this, Actor::bResetAI)
}

// Translated from 00886b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the actor has a process and the process virtual `+0x40c` says six
/// (or, failing that, the virtual `+0x22c` or the process virtual `+0x6fc`
/// say so): blends the actor's bip transform (`bhkBlendCollisionObject::
/// SetBipTransform`, `00c821b0`), clears a flag on the character controller
/// and rotates the 3-vector at `vector` by the actor's heading (a
/// z-rotation matrix built from the `float` at `+0x30` of the actor's
/// `+0x24` vector). Returns true when it did so. The meanings of the
/// virtuals `+0x230`, `+0x40c` and `+0x6fc` are not confirmed.
pub fn fn_00886b30(e: &mut Engine, this: Ptr<Actor>, vector: Ptr) -> bool {
    let process = e.call(GET_PROCESS, &args![this]).u32();
    if process == 0 {
        return false;
    }
    let mut proceed = false;
    if e.vcall(this.addr(), VSLOT_PACKAGE_TEST_0X230, &args![])
        .bool()
    {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        if e.vcall(process, PROCESS_VSLOT_0X40C, &args![]).u32() != 6 {
            proceed = true;
        }
    }
    if !proceed {
        if e.vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
            .bool()
        {
            proceed = true;
        } else {
            let process = e.call(GET_PROCESS, &args![this]).u32();
            if e.vcall(process, PROCESS_VSLOT_0X6FC, &args![]).bool() {
                proceed = true;
            }
        }
    }
    if !proceed {
        return false;
    }
    let not_flag = !e
        .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
        .bool();
    let blend = e.call(BLEND_OBJECT_OF, &args![this]).u32();
    e.call(SET_BIP_TRANSFORM, &args![blend, vector, not_flag]);
    let controller = e.call(GET_CHAR_CONTROLLER, &args![this]).u32();
    if controller != 0 {
        fn_00886c80(e, Ptr::new(controller), true);
    }
    let rotation = e.call(ACTOR_ROTATION_VECTOR, &args![this]).u32();
    let heading = e.mem.f32(rotation + 8);
    e.with_stack(0x30, |e, base| {
        let matrix = base.addr();
        let result = base.addr() + 0x24;
        e.call(MATRIX_CONSTRUCT, &args![matrix]);
        e.call(MATRIX_MAKE_ROTATION, &args![matrix, heading]);
        let rotated = e
            .call(VECTOR_TIMES_MATRIX, &args![result, vector, matrix])
            .u32();
        for offset in [0, 4, 8] {
            let word = e.mem.u32(rotated + offset);
            e.mem.set_u32(vector.addr() + offset, word);
        }
    });
    true
}

// Translated from 00886c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears the bit `0x1000` of the flag word at `+0x410` of the
/// character controller (`00629670`: `flag` set ORs the mask into the word
/// at +4 of that sub-object, otherwise `006296b0` clears it).
pub fn fn_00886c80(e: &mut Engine, this: Ptr, flag: bool) {
    e.call(
        FLAG_SET_OR_CLEAR,
        &args![this.addr() + 0x410, 0x1000u32, flag],
    );
}

// Translated from 008879d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The same as `00886c80` with the bit `0x40000`.
pub fn fn_008879d0(e: &mut Engine, this: Ptr, flag: bool) {
    e.call(
        FLAG_SET_OR_CLEAR,
        &args![this.addr() + 0x410, 0x40000u32, flag],
    );
}

// Translated from 00887a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global word at `011c6260`.
pub fn fn_00887a00(e: &mut Engine) -> u32 {
    e.mem.u32(GLOBAL_VALUE_011C6260)
}

// Translated from 00887a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The object behind `this`'s `004ae750` pointer, passed through `0084e3a0`
/// (which reads its word at +0xc); without it, the word the pointer at
/// `this + 0xc` points to.
pub fn fn_00887a10(e: &mut Engine, this: Ptr) -> u32 {
    let proxy = e.call(PROXY_OF, &args![this]).u32();
    if proxy != 0 {
        return e.call(WORD_AT_0XC, &args![proxy]).u32();
    }
    let link = e.mem.u32(this.addr() + 0xc);
    if link == 0 {
        return 0;
    }
    // The pointer is adjusted by -4 and the word at +4 of that is read.
    e.mem.u32(link)
}

// Translated from 00887a80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Hands the object `00887a10` finds a 4-byte value built from `flag`
/// (`00622570` stores the byte) and the `004ae750` pointer of `this`
/// (`hkpLimitedHingeConstraintData::setMotorActive` in the engine map).
pub fn fn_00887a80(e: &mut Engine, this: Ptr, flag: bool) {
    let target = fn_00887a10(e, this);
    if target == 0 {
        return;
    }
    let value = e.with_stack(4, |e, slot| {
        e.call(BYTE_STORE, &args![slot, flag]);
        e.mem.u32(slot.addr())
    });
    let proxy = e.call(PROXY_OF, &args![this]).u32();
    e.call(SET_MOTOR_ACTIVE, &args![target, proxy, value]);
}

// Translated from 00887ac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes `value` to `009314d0` of the object `00887a10` finds, if any.
pub fn fn_00887ac0(e: &mut Engine, this: Ptr, value: f32) {
    let target = fn_00887a10(e, this);
    if target != 0 {
        e.call(SET_FLOAT_ON_TARGET, &args![target, value]);
    }
}

// Translated from 00887af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards `value` to `00887b20` of the `004ae750` pointer of `this`.
pub fn fn_00887af0(e: &mut Engine, this: Ptr, value: u32) {
    let proxy = e.call(PROXY_OF, &args![this]).u32();
    fn_00887b20(e, Ptr::new(proxy), value);
}

// Translated from 00887b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes `value` to `004a3f10` of the sub-object at +0x40.
pub fn fn_00887b20(e: &mut Engine, this: Ptr, value: u32) {
    e.call(SUBOBJECT_SET_VALUE, &args![this.addr() + 0x40, value]);
}

// Translated from 00887b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards the byte `value` to `00887b70` of the `004ae750` pointer of
/// `this`.
pub fn fn_00887b40(e: &mut Engine, this: Ptr, value: u8) {
    let proxy = e.call(PROXY_OF, &args![this]).u32();
    fn_00887b70(e, Ptr::new(proxy), value);
}

// Translated from 00887b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte `value` at +0x5c.
pub fn fn_00887b70(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x5c, value);
}

/// The frame time scaled by the VATS multiplier when the VATS state is 4
/// and `this` is the VATS target (the common prologue of several updates):
/// `value * 009c8d60()`, rounded to `float`.
fn vats_scaled(e: &mut Engine, this: Ptr<Actor>, value: f32) -> f32 {
    if e.call(SETTING_INT_VALUE, &args![VATS_STATE_OBJECT]).u32() == 4
        && this.addr() == e.mem.u32(VATS_TARGET_ACTOR)
    {
        let multiplier = e
            .call(VATS_TARGET_UPDATE_MULT, &args![VATS_STATE_OBJECT])
            .f32();
        return (multiplier as f64 * value as f64) as f32;
    }
    value
}

/// Moves the reference to the world space of its cell when its position is
/// outside the cell (the tail `00887b90` and `00886360` share): the cell,
/// the position and the three tests, then `MoveRefToNewSpace(this, 0,
/// worldspace)`.
fn move_out_of_cell(e: &mut Engine, this: Ptr<Actor>) {
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    let position = e
        .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
        .u32();
    let copy = [
        e.mem.u32(position),
        e.mem.u32(position + 4),
        e.mem.u32(position + 8),
    ];
    if cell == 0 || e.call(CELL_TEST_INTERIOR, &args![cell]).bool() {
        return;
    }
    let inside = e.with_stack(12, |e, slot| {
        for (i, word) in copy.iter().enumerate() {
            e.mem.set_u32(slot.addr() + 4 * i as u32, *word);
        }
        e.call(CELL_CONTAINS_POSITION, &args![cell, slot]).bool()
    });
    if !inside {
        let world_space = e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32();
        e.call(MOVE_REF_TO_NEW_SPACE, &args![this, 0u32, world_space]);
    }
}

// Translated from 00887b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// A per-frame step for a non-player actor's process (muzzle flash update,
/// the cell check and the fade update), `delta` the frame time. It does
/// nothing when `00576d30` (a flag of the reference) says so. The second
/// stack word is not read.
pub fn fn_00887b90(e: &mut Engine, this: Ptr<Actor>, delta: f32, _unused_1: u32) {
    if e.call(REFERENCE_FLAG_TEST_0X200000, &args![this]).bool() {
        return;
    }
    let delta = vats_scaled(e, this, delta);
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null()
        && !e
            .call(IS_MENU_ID_VISIBLE, &args![MENU_ID_0X3F0, 0u32])
            .bool()
    {
        e.vcall(process.addr(), PROCESS_VSLOT_0X464, &args![this]);
        if e.vcall(process.addr(), PROCESS_VSLOT_MUZZLE_FLASH, &args![])
            .u32()
            != 0
        {
            let flash = e
                .vcall(process.addr(), PROCESS_VSLOT_MUZZLE_FLASH, &args![])
                .u32();
            e.call(MUZZLE_FLASH_UPDATE, &args![flash, delta, this]);
        }
    }
    if e.call(SAVE_FORM_BUFFER_GET_FORM, &args![this]).u32() != 0 {
        move_out_of_cell(e, this);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() && e.call(PROCESS_GET_LEVEL, &args![process]).u32() == 0 {
        e.call(HIGH_PROCESS_FADE_UPDATE, &args![process, this]);
    }
}

// Translated from 00887d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ResetLoadedAnimations` (Xbox PDB): for a non-player actor with
/// a 3D object, either queues the reset on the task queue
/// (`TaskQueueInterface::QueueActorResetLoadedAnims`) when `008c7aa0`
/// says so, or replaces the actor's animation with a fresh one: clears the
/// process's three entries (virtual `+0x424` for 0 to 2), picks the
/// animation set from the process's item, builds the KF file list, inits
/// the new animation, logs when that fails and, for an actor not in life
/// state 2 whose base form has KF files, adds the special animations of
/// the model's folder. The exception frame is not translated.
pub fn actor_reset_loaded_animations(e: &mut Engine, this: Ptr<Actor>) {
    if this.addr() == e.mem.u32(PLAYER_CHARACTER) {
        return;
    }
    if e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32() == 0 {
        return;
    }
    if e.call(RESET_ANIMS_SHOULD_QUEUE, &args![]).bool() {
        let queue = e.call(TASK_QUEUE_INTERFACE, &args![]).u32();
        e.call(QUEUE_ACTOR_RESET_LOADED_ANIMS, &args![queue, this]);
        return;
    }
    e.call(SET_ANIMATION, &args![this, 0u32]);
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        e.vcall(process.addr(), PROCESS_VSLOT_0X424, &args![0u32, 0u32]);
        e.vcall(process.addr(), PROCESS_VSLOT_0X424, &args![1u32, 0u32]);
        e.vcall(process.addr(), PROCESS_VSLOT_0X424, &args![2u32, 0u32]);
    }
    let mut kf_group = 0u32;
    if e.call(GET_PROCESS, &args![this]).u32() != 0 {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        let level = e.call(PROCESS_GET_LEVEL, &args![process]).i32();
        if (0..=1).contains(&level) {
            let process = e.call(GET_PROCESS, &args![this]).u32();
            if e.vcall(process, PROCESS_VSLOT_ITEM, &args![]).u32() != 0 {
                let item = e.vcall(process, PROCESS_VSLOT_ITEM, &args![]).u32();
                let value = e.call(ITEM_VALUE, &args![item]).u32();
                let index = e.call(ITEM_GROUP_INDEX, &args![value]).i32();
                kf_group = e
                    .mem
                    .u32(KF_GROUP_TABLE.wrapping_add((index as u32).wrapping_mul(4)));
            } else if e.call(ACTOR_ANIMATION_GROUP_TEST, &args![this]).bool() {
                kf_group = 1;
            }
        }
    }
    let raw = e.call(OPERATOR_NEW_FN, &args![ANIMATION_SIZE]).u32();
    let animation = if raw != 0 {
        e.call(ANIMATION_CONSTRUCTOR, &args![raw]).u32()
    } else {
        0
    };
    let new_animation = e.call(SET_ANIMATION, &args![this, animation]).u32();
    let kf_list = e.call(BUILD_KF_FILE_LIST, &args![this, kf_group]).u32();
    let base = base_form(e, this).addr();
    let animation_slot = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
    let not_flag = !e
        .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
        .bool();
    let item = e.vcall(animation_slot, 0xc, &args![]).u32();
    let ok = e
        .call(
            INIT_ANIMATION,
            &args![new_animation, kf_list, item, this, not_flag],
        )
        .bool();
    if !ok {
        let form_id = e.call(WORD_AT_0XC, &args![base]).u32();
        let name = e.vcall(base, 0x130, &args![]).u32();
        e.call(
            LOG_MESSAGE,
            &args![BAD_INIT_ANIMATION_FORMAT, name, form_id],
        );
    }
    let life_state = e.get(this, Actor::eLifeState);
    if life_state != 2 && e.call(HAS_KF_FILES, &args![base + 0xc4]).bool() {
        let model = e.call(GET_MODEL, &args![this]).u32();
        e.with_stack(0x104, |e, buffer| {
            e.call(STRING_COPY, &args![buffer, 0x104u32, model]);
            let slash = e.call(STRING_FIND_LAST, &args![buffer, 0x5cu32]).u32();
            if slash != 0 {
                e.mem.set_u8(slash, 0);
                let running = e.call(PACKAGE_RUNNING_OF, &args![base + 0xc4]).u32();
                e.call(
                    ADD_SPECIAL_ANIMATIONS,
                    &args![new_animation, running, buffer],
                );
            }
        });
    }
}

// Translated from 00886cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A per-frame physics and 3D step of an actor: refreshes the swim flag
/// (`00884990`), moves a non-player actor along its mover's path
/// (`SetLocationOnReference`), updates the level-of-detail flag of the
/// shader accumulator, plays out the death ragdoll or knockdown once
/// `bDeadFlag` is set, and, for an actor with the turret flag (`+0x1b0`),
/// re-syncs the collision objects of its 3D object with the havok world
/// (two passes with the globals of `00887a00` and `0087e9a0`). `delta` is
/// the frame time (the VATS-scaled step when it is 0 and the process type
/// is 0); the second stack word is not read. Not named in the engine map;
/// meanings of most callees and slots are not confirmed.
pub fn fn_00886cb0(e: &mut Engine, this: Ptr<Actor>, delta: f32, _unused_2: u32) {
    let player = e.mem.u32(PLAYER_CHARACTER);
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF69E]);
    let mut delta = delta;
    if delta == 0.0 && e.call(GET_CURRENT_PROCESS_TYPE, &args![this]).u32() == 0 {
        if e.call(SETTING_INT_VALUE, &args![VATS_STATE_OBJECT]).u32() == 4
            && this.addr() == e.mem.u32(VATS_TARGET_ACTOR)
        {
            let step = e
                .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
                .f64();
            let multiplier = e
                .call(VATS_TARGET_UPDATE_MULT, &args![VATS_STATE_OBJECT])
                .f64();
            delta = (multiplier * step) as f32;
        } else {
            delta = e
                .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
                .f32();
        }
    }
    fn_00884990(e, this);
    if this.addr() != player {
        follow_mover_path(e, this, player, delta);
    }
    if e.call(ACTOR_GET_IN_COMBAT, &args![this]).bool() {
        e.call(ACTOR_FUNCTION_0X8B00C0, &args![this]);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    let turret_object = if process.is_null() {
        0
    } else {
        e.vcall(process.addr(), PROCESS_VSLOT_0X28C, &args![]).u32()
    };
    if this.addr() != player {
        let skeleton = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
        if this.addr() != player && skeleton != 0 {
            level_of_detail(e, this, player, skeleton, turret_object);
        }
    }
    if turret_object != 0 {
        let flag = e.call(GLOBAL_BYTE_FN_525420, &args![]).u8();
        e.call(TURRET_SET_FLAG_0X810610, &args![turret_object, flag as u32]);
    }
    if e.get(this, Actor::bDeadFlag) && e.call(GET_PROCESS, &args![this]).u32() != 0 {
        let collision = e.call(BLEND_OBJECT_OF, &args![this]).u32();
        if collision != 0 {
            dead_ragdoll(e, this, collision);
        }
    }
    if e.call(ACTOR_TURRET_BEHAVIOR, &args![this]).bool()
        && !e
            .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
            .bool()
        && !e.call(ACTOR_BLOCKED_TEST, &args![this]).bool()
    {
        let skeleton = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
        let mut changed = false;
        let key = e.call(GLOBAL_WORD_FN_87E9A0, &args![]).u32();
        changed |= resync_collision(e, this, skeleton, key, false);
        let key = fn_00887a00(e);
        changed |= resync_collision(e, this, skeleton, key, true);
        if changed {
            e.vcall(this.addr(), VSLOT_0X48, &args![4u32]);
        }
    }
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF69E]);
}

/// The path part of [`fn_00886cb0`] for a non-player actor: the mover's
/// next position (virtuals `+0x20`, `+0x24` of `pActorMover`, after
/// `00886b30`) and either `Actor` virtual `+0x250` or a plain
/// `SetLocationOnReference` when the cell is not ready.
fn follow_mover_path(e: &mut Engine, this: Ptr<Actor>, player: u32, delta: f32) {
    e.with_stack(0x80, |e, base| {
        let vector = base.addr();
        let matrix = base.addr() + 0x10;
        let scratch_a = base.addr() + 0x40;
        let scratch_b = base.addr() + 0x50;
        let scratch_c = base.addr() + 0x60;
        let mut extra = 0u32;
        for i in 0..3 {
            let word = e.mem.u32(SCALE_VECTOR + 4 * i);
            e.mem.set_u32(vector + 4 * i, word);
        }
        if !e.vcall(this.addr(), VSLOT_0X4B4, &args![]).bool()
            && !fn_00886b30(e, this, Ptr::new(vector))
        {
            let mover = e.get(this, Actor::pActorMover);
            extra = e.vcall(mover.addr(), MOVER_VSLOT_0X20, &args![]).u32();
            let mover = e.get(this, Actor::pActorMover);
            if !e
                .vcall(mover.addr(), MOVER_VSLOT_0X24, &args![vector])
                .bool()
            {
                let animation = e.call(ACTOR_GET_ANIMATION, &args![this]).u32();
                if animation != 0 {
                    e.call(
                        ANIMATION_SCALE_VECTOR,
                        &args![animation, vector, this, 0u32, 1u32],
                    );
                }
            }
        }
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        if cell == 0
            || !e.call(CELL_TEST_0X450FF0, &args![cell]).bool()
            || e.call(PLAYER_SLEEPING_OR_RESTING, &args![player]).bool()
        {
            e.call(MATRIX_CONSTRUCT, &args![matrix]);
            let rotation = e.call(ACTOR_ROTATION_VECTOR, &args![this]).u32();
            let heading = e.mem.f32(rotation + 8);
            e.call(MATRIX_MAKE_ROTATION, &args![matrix, heading]);
            let rotated = e
                .call(MATRIX_TIMES_VECTOR, &args![matrix, scratch_a, vector])
                .u32();
            let position = e
                .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
                .u32();
            let moved = e
                .call(VECTOR_PLUS_VECTOR, &args![position, scratch_b, rotated])
                .u32();
            e.call(SET_LOCATION_ON_REFERENCE, &args![this, moved]);
        } else {
            e.vcall(this.addr(), VSLOT_0X250, &args![delta, vector, extra]);
        }
        let _ = scratch_c;
    });
}

/// The shader accumulator part of [`fn_00886cb0`]: works out whether the
/// actor is far enough to drop out of detail (the distance from the head
/// target to the world bound, less the bound's radius scaled by 1.25) and
/// tells the accumulator and the turret object.
fn level_of_detail(e: &mut Engine, this: Ptr<Actor>, player: u32, skeleton: u32, turret: u32) {
    e.with_stack(0x80, |e, base| {
        let target = base.addr();
        let scaled = base.addr() + 0x10;
        let sum = base.addr() + 0x20;
        let bound = base.addr() + 0x30;
        let difference = base.addr() + 0x40;
        e.call(MATRIX_CONSTRUCT, &args![target]);
        let head_a = e.mem.u32(HEAD_TARGET_A);
        let source = if e.call(PLAYER_STATE_TEST, &args![player]).bool() && head_a != 0 {
            e.call(OBJECT_POSITION_0X8C, &args![head_a]).u32()
        } else if e.mem.u32(HEAD_TARGET_B) != 0 {
            let head_b = e.mem.u32(HEAD_TARGET_B);
            e.call(OBJECT_POSITION_0X8C, &args![head_b]).u32()
        } else {
            let eye = e.call(GET_EYE_LEVEL, &args![player]).f32();
            let scaled_vector = e
                .call(VECTOR_TIMES_SCALAR, &args![scaled, eye, EYE_DIRECTION])
                .u32();
            let position = e.call(POSITION_VECTOR_OF, &args![player]).u32();
            e.call(VECTOR_PLUS_VECTOR, &args![position, sum, scaled_vector])
                .u32()
        };
        for i in 0..3 {
            let word = e.mem.u32(source + 4 * i);
            e.mem.set_u32(target + 4 * i, word);
        }
        let world_bound = e.call(GET_WORLD_BOUND, &args![skeleton]).u32();
        for i in 0..4 {
            let word = e.mem.u32(world_bound + 4 * i);
            e.mem.set_u32(bound + 4 * i, word);
        }
        let same = e.call(MATRIX_CONSTRUCT, &args![bound]).u32();
        let delta = e
            .call(VECTOR_MINUS_VECTOR, &args![target, difference, same])
            .u32();
        let distance = e.call(VECTOR_LENGTH, &args![delta]).f64();
        let radius = e.call(UPDATE_STEP_SETTING, &args![bound]).f64();
        let lod = (distance - radius * e.global::<f64>(FOLLOW_CATCH_UP_FACTOR)) as f32;
        let mut far = false;
        let owner = e.vcall(skeleton, SKELETON_VSLOT_0X10, &args![]).u32();
        if owner != 0 {
            let divisor = e.call(LOD_DIVISOR, &args![3u32]).f64();
            let quotient = (lod as f64 / divisor) as f32;
            let reference = e.call(SKELETON_RADIUS, &args![owner]).f64();
            far = reference < quotient as f64;
        }
        let accumulator = e.call(GET_ACCUMULATOR, &args![]).u32();
        if accumulator != 0 && e.call(ACCUMULATOR_TEST, &args![accumulator]).u32() != 0 {
            let value = if far { false } else { lod as f64 > 0.0 };
            let id = e.call(WORD_AT_0XC, &args![this]).u32();
            e.call(ACCUMULATOR_UPDATE, &args![accumulator, id, bound, value]);
        }
        if turret != 0 {
            fn_008879d0(e, Ptr::new(turret), far);
        }
    });
}

/// The death part of [`fn_00886cb0`]: with `collision` the actor's blend
/// collision object, stops the ragdoll animation, hands the penetration
/// utility the actor's shape, sets life state 1 and either knocks the
/// body down (an actor whose form test `+0x38c` fails) or plays the death
/// animation section, then clears `bDeadFlag`.
fn dead_ragdoll(e: &mut Engine, this: Ptr<Actor>, collision: u32) {
    let mut alive_form = true;
    let mut actor_form = 0u32;
    let form = e.call(SAVE_FORM_BUFFER_GET_FORM, &args![this]).u32();
    if e.call(FORM_TYPE, &args![form]).u32() == 0x2b {
        actor_form = this.addr();
    }
    if actor_form != 0
        && !e
            .vcall(actor_form, VSLOT_ACTOR_FORM_TEST_0X38C, &args![])
            .bool()
    {
        alive_form = false;
    }
    let ragdoll = e.get(this, Actor::pRagdollController);
    if !ragdoll.is_null() && e.call(RAGDOLL_TEST_0X552490, &args![ragdoll]).bool() {
        let ragdoll = e.get(this, Actor::pRagdollController);
        e.call(DISABLE_RAGDOLL_ANIM, &args![ragdoll, 1u32]);
    }
    let penetration = e.get(this, Actor::pPenetrationDetection);
    if !penetration.is_null() {
        let shape = e.with_stack(4, |e, slot| {
            let made = e.call(ACTOR_SHAPE_0X931ED0, &args![this, slot]).u32();
            e.call(SHAPE_RESULT_0X4A3A20, &args![made]).u32()
        });
        let penetration = e.get(this, Actor::pPenetrationDetection);
        e.call(PENETRATION_UPDATE, &args![penetration, collision, shape]);
    }
    e.call(ACTOR_SET_LIFE_STATE, &args![this, 1u32]);
    e.vcall(this.addr(), VSLOT_0X2A0, &args![]);
    if alive_form {
        e.with_stack(0x60, |e, base| {
            let matrix = base.addr();
            let direction = base.addr() + 0x24;
            let scratch = base.addr() + 0x30;
            e.call(MATRIX_CONSTRUCT, &args![matrix]);
            let heading = e
                .vcall(this.addr(), VSLOT_GET_HEADING_0X2BC, &args![0u32])
                .f32();
            e.call(MATRIX_MAKE_ROTATION, &args![matrix, heading]);
            e.call(NI_POINT3_SET, &args![direction, 0.0f32, 1.0f32, 0.0f32]);
            let rotated = e
                .call(MATRIX_TIMES_VECTOR, &args![matrix, scratch, direction])
                .u32();
            for i in 0..3 {
                let word = e.mem.u32(rotated + 4 * i);
                e.mem.set_u32(direction + 4 * i, word);
            }
            e.call(
                DO_KNOCK_DOWN,
                &args![collision, direction, 1u32, 0.0f32, 0u32],
            );
        });
        e.vcall(this.addr(), VSLOT_0X48, &args![4u32]);
        let process = e.get(this, Actor::pCurrentProcess);
        e.vcall(process.addr(), 0x28, &args![]);
    } else {
        let animation = e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32();
        if animation != 0
            && e.call(ANIMATION_GROUP_LOADED, &args![animation, 0xe0u32])
                .bool()
        {
            e.call(
                ANIMATION_FORCE_SECTION,
                &args![
                    animation,
                    0x14u32,
                    0xe0u32,
                    0xffff_ffffu32,
                    0.0f32,
                    0xffff_ffffu32
                ],
            );
            let entry = e.call(ANIMATION_LOOKUP, &args![animation, 1u32]).u32();
            if entry != 0 {
                let value = e.call(ENTRY_FLOAT_08100, &args![entry]).f32();
                e.call(ENTRY_SET_VALUE_98ADB0, &args![entry, value]);
                let value = e.call(ENTRY_FLOAT_08100, &args![entry]).f32();
                e.call(ANIMATION_UPDATE, &args![animation, this, 0.0f32, value]);
                e.call(ANIMATION_UPDATE_MOVEMENT, &args![animation, this]);
            }
            e.call(ACTOR_AFTER_DEATH_ANIMATION, &args![this]);
        }
    }
    e.set(this, Actor::bDeadFlag, false);
}

/// One pass of the collision re-sync of [`fn_00886cb0`]: finds the
/// collision object of `skeleton` for `key` (`004ade00`), adds its rigid
/// body to the world when it is not there, rotates a matching entry of
/// the body's list (`second` selects the variant that clamps and hands a
/// float to the entry) and synchronises. Returns true when the rigid body
/// was added.
fn resync_collision(
    e: &mut Engine,
    this: Ptr<Actor>,
    skeleton: u32,
    key: u32,
    second: bool,
) -> bool {
    let collision = e.call(FIND_COLLISION_OBJECT, &args![skeleton, key]).u32();
    if collision == 0 {
        return false;
    }
    e.call(COLLISION_FUNCTION_0X6838B0, &args![collision]);
    let name = e.call(COLLISION_FUNCTION_0X6838B0, &args![collision]).u32();
    let entry = e.call(FIND_ENTRY, &args![ENTRY_TABLE_A, name]).u32();
    if entry == 0 {
        return false;
    }
    let mut added = false;
    let body = e.call(COLLISION_RIGID_BODY, &args![entry]).u32();
    let mut list = e.call(BODY_LIST_0X7D6BB0, &args![body]).u32();
    let shape = e.call(BODY_SHAPE_0X517630, &args![body]).u32();
    let in_world = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), shape);
        e.call(WORLD_TEST_0XC8CCE0, &args![slot]).bool()
    });
    if !in_world {
        e.call(SET_MOTION, &args![collision, 1u32, 1u32, 1u32, 1u32]);
        let word = e.vcall(body, BODY_VSLOT_0X94, &args![]).u32();
        let made = e.call(BODY_FUNCTION_0X621480, &args![word]).u32();
        e.call(ADD_ACT_CON_TO_WORLD, &args![body, made]);
        added = true;
    }
    let kind = e.with_stack(4, |e, slot| {
        e.call(BODY_FLAGS_0X43B4F0, &args![body, slot]);
        e.call(BODY_KIND_0X43B4D0, &args![slot]).u32()
    });
    if kind == 0x1d {
        e.call(COLLISION_SET_0XC804D0, &args![skeleton, 8u32]);
    }
    e.call(BODY_ACTIVATE_0X561580, &args![body, 1u32]);
    while list != 0 {
        let same = e.call(MATRIX_CONSTRUCT, &args![list]).u32();
        if e.call(WORD_AT_START, &args![same]).u32() == 0 {
            break;
        }
        let same = e.call(MATRIX_CONSTRUCT, &args![list]).u32();
        let node = e.call(WORD_AT_START, &args![same]).u32();
        if !second {
            let found = e.call(FIND_ENTRY, &args![ENTRY_TABLE_B, node]).u32();
            if found != 0 {
                let heading = e
                    .vcall(this.addr(), VSLOT_GET_HEADING_0X2BC, &args![0u32])
                    .f32();
                let first = e
                    .call(ANGLE_0X5B9E80, &args![heading, 0.0f32, 0.0f32])
                    .f32();
                let second_value = e.call(ANGLE_0X5C53D0, &args![heading, first]).f32();
                e.with_stack(0x10, |e, out| {
                    e.call(
                        QUATERNION_SET,
                        &args![out, second_value, first, 0.0f32, 0.0f32],
                    );
                    fn_00887b40(e, Ptr::new(found), 1);
                    fn_00887af0(e, Ptr::new(found), out.addr());
                });
                break;
            }
        } else {
            let handle = e.vcall(node, NODE_VSLOT_0X44, &args![]).u32();
            if handle != 0 {
                let target = fn_00887a10(e, Ptr::new(handle));
                let value = e.mem.f32(this.addr() + 0x24);
                let upper = e.call(HANDLE_FLOAT_0X9A1260, &args![target]).f32();
                let lower = e.call(HANDLE_FLOAT_0X644A50, &args![target]).f32();
                let clamped = e.with_stack(4, |e, slot| {
                    e.mem.set_f32(slot.addr(), value);
                    e.call(CLAMP_FLOAT, &args![slot, lower, upper]);
                    e.mem.f32(slot.addr())
                });
                fn_00887a80(e, Ptr::new(target), true);
                fn_00887ac0(e, Ptr::new(target), clamped);
                break;
            }
        }
        list = e.call(NEXT_LIST_NODE, &args![list]).u32();
    }
    if e.call(RESET_ANIMS_SHOULD_QUEUE, &args![]).bool() {
        let queue = e.call(TASK_QUEUE_INTERFACE, &args![]).u32();
        e.call(QUEUE_COLLISION_SYNC, &args![queue, this, collision]);
    } else {
        e.call(COLLISION_SYNCHRONIZE, &args![collision, 2u32]);
    }
    added
}

// Translated from 00888070 (decompiled, FalloutNV.exe 1.4.0.525)
/// The per-frame animation step of an actor with a 3D object: advances the
/// process's high-level animation state, the idle-wait counter of the
/// process, the dying animation (`bDeadFlag` is cleared once it is
/// played), the process virtual `+0x740` update and
/// `UpdateAnimationMovement`, then `Actor::PickAnimations(1.0, speed)`
/// with one of four settings. `delta` is the frame time passed on; the
/// meanings of the process virtuals `+0x740` to `+0x750` are not
/// confirmed.
pub fn fn_00888070(e: &mut Engine, this: Ptr<Actor>, delta: f32) {
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF69C]);
    let skeleton = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
    if skeleton != 0 {
        let skeleton = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
        if e.call(PACKAGE_TYPE, &args![skeleton]).u32() != 0 {
            animation_step(e, this, delta);
        }
    }
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF69C]);
}

/// The body of `00888070` once the actor has a 3D object whose `009611e0`
/// answer is non-zero.
fn animation_step(e: &mut Engine, this: Ptr<Actor>, delta: f32) {
    e.call(ACTOR_PRE_ANIMATION_STEP, &args![this]);
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), PROCESS_VSLOT_0X41C, &args![this]);
    let animation = e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32();
    if e.vcall(this.addr(), VSLOT_ANIMATION_FLAG_0X21C, &args![])
        .bool()
        && animation != 0
        && e.call(ANIMATION_GROUP_LOADED, &args![animation, 0xe1u32])
            .bool()
        && e.call(GET_PROCESS, &args![this]).u32() != 0
    {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        if e.vcall(process, PROCESS_VSLOT_0X748, &args![]).u32() != 0 {
            let mut count = 0u32;
            let process = e.call(GET_PROCESS, &args![this]).u32();
            let have = e.vcall(process, PROCESS_VSLOT_0X750, &args![]).u32();
            let wanted = animation_time_in_milliseconds(e, animation);
            if wanted as u32 > have {
                let process = e.call(GET_PROCESS, &args![this]).u32();
                let have = e.vcall(process, PROCESS_VSLOT_0X750, &args![]).u32();
                let wanted = animation_time_in_milliseconds(e, animation);
                count = (wanted as u32).wrapping_sub(have) / 0x21;
            }
            let process = e.call(GET_PROCESS, &args![this]).u32();
            let list = e.vcall(process, PROCESS_VSLOT_0X748, &args![]).u32();
            let fraction = e.call(LIST_FRACTION_AT, &args![list, count]).f32();
            e.call(ANIMATION_SET_FRACTION, &args![animation, fraction, count]);
            let process = e.call(GET_PROCESS, &args![this]).u32();
            if e.vcall(process, PROCESS_VSLOT_0X748, &args![]).u32() != 0 {
                let process = e.call(GET_PROCESS, &args![this]).u32();
                let list = e.vcall(process, PROCESS_VSLOT_0X748, &args![]).u32();
                if count > e.call(LIST_COUNT, &args![list]).u32() {
                    let process = e.call(GET_PROCESS, &args![this]).u32();
                    e.vcall(process, PROCESS_VSLOT_0X744, &args![0u32]);
                    let process = e.call(GET_PROCESS, &args![this]).u32();
                    e.vcall(process, PROCESS_VSLOT_0X74C, &args![0u32]);
                }
            }
        }
    }
    if e.get(this, Actor::bDeadFlag) && e.call(GET_PROCESS, &args![this]).u32() != 0 {
        let collision = e.call(BLEND_OBJECT_OF, &args![this]).u32();
        if collision != 0 {
            let mut alive_form = true;
            let mut actor_form = 0u32;
            let form = e.call(SAVE_FORM_BUFFER_GET_FORM, &args![this]).u32();
            if e.call(FORM_TYPE, &args![form]).u32() == 0x2b {
                actor_form = this.addr();
            }
            if actor_form != 0
                && !e
                    .vcall(actor_form, VSLOT_ACTOR_FORM_TEST_0X38C, &args![])
                    .bool()
            {
                alive_form = false;
            }
            if !alive_form
                && animation != 0
                && e.call(ANIMATION_GROUP_LOADED, &args![animation, 0xe0u32])
                    .bool()
            {
                e.call(ACTOR_SET_LIFE_STATE, &args![this, 1u32]);
                e.vcall(this.addr(), VSLOT_0X2A0, &args![]);
                e.call(
                    ANIMATION_FORCE_SECTION,
                    &args![
                        animation,
                        0x14u32,
                        0xe0u32,
                        0xffff_ffffu32,
                        0.0f32,
                        0xffff_ffffu32
                    ],
                );
                let entry = e.call(ANIMATION_LOOKUP, &args![animation, 1u32]).u32();
                if entry != 0 {
                    let value = e.call(ENTRY_FLOAT_08100, &args![entry]).f32();
                    e.call(ENTRY_SET_VALUE_98ADB0, &args![entry, value]);
                    let value = e.call(ENTRY_FLOAT_08100, &args![entry]).f32();
                    e.call(ANIMATION_UPDATE, &args![animation, this, 0.0f32, value]);
                    e.call(ANIMATION_UPDATE_MOVEMENT, &args![animation, this]);
                }
                e.call(ACTOR_AFTER_DEATH_ANIMATION, &args![this]);
                e.set(this, Actor::bDeadFlag, false);
            }
        }
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        let step = e
            .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
            .f32();
        if e.call(SETTING_INT_VALUE, &args![VATS_STATE_OBJECT]).u32() == 4
            && this.addr() == e.mem.u32(VATS_TARGET_ACTOR)
        {
            let step = e
                .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
                .f64();
            let multiplier = e
                .call(VATS_TARGET_UPDATE_MULT, &args![VATS_STATE_OBJECT])
                .f64();
            let scaled = (multiplier * step) as f32;
            e.vcall(process.addr(), PROCESS_VSLOT_0X740, &args![this, scaled]);
        } else {
            e.vcall(process.addr(), PROCESS_VSLOT_0X740, &args![this, step]);
        }
    }
    if !e
        .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
        .bool()
    {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        if e.vcall(process, PROCESS_VSLOT_FLAG, &args![]).bool()
            && animation != 0
            && e.call(ANIMATION_LOOKUP, &args![animation, 4u32]).u32() == 0
        {
            e.call(
                ACTOR_QUEUE_ANIMATION_0X8B28C0,
                &args![this, 0x11u32, animation],
            );
        }
    }
    actor_update_animation_movement(e, this, Ptr::new(animation), delta);
    let flag_0x21c = e
        .vcall(this.addr(), VSLOT_ANIMATION_FLAG_0X21C, &args![])
        .bool();
    let flag_0x493bb0 = e.call(ACTOR_GET_IN_COMBAT, &args![this]).bool();
    let setting = match (flag_0x21c, flag_0x493bb0) {
        (false, true) => SPEED_SETTING_21C_CLEAR_TRUE,
        (false, false) => SPEED_SETTING_21C_CLEAR_FALSE,
        (true, true) => SPEED_SETTING_21C_SET_TRUE,
        (true, false) => SPEED_SETTING_21C_SET_FALSE,
    };
    let speed = setting_value(e, setting);
    e.call(PICK_ANIMATIONS, &args![this, 1.0f32, speed]);
}

/// `Animation::fTime`-style value `453700` (a `float` at +0xd0 of the
/// animation) times 1000, truncated toward zero to 64 bits (the code
/// switches the x87 rounding mode to truncate), as a `u64`.
fn animation_time_in_milliseconds(e: &mut Engine, animation: u32) -> u64 {
    let seconds = e.call(ANIMATION_TIME, &args![animation]).f64();
    let scaled = seconds * e.global::<f64>(MILLISECONDS_PER_SECOND);
    (scaled as i64) as u64
}

// Translated from 008885a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `Animation::UpdateMovementNoWorldUpdate(this)` on the actor's
/// animation (virtual `+0x1e4`), if there is one.
pub fn fn_008885a0(e: &mut Engine, this: Ptr<Actor>) {
    let animation = e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32();
    if animation != 0 {
        e.call(
            ANIMATION_UPDATE_MOVEMENT_NO_WORLD_UPDATE,
            &args![animation, this],
        );
    }
}

// Translated from 008885e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::UpdateAnimationMovement` (Xbox PDB): moves the actor by its
/// animation's movement, `animation` being the actor's animation (`this`
/// does nothing without one). The player is counted in a global nesting
/// byte (`011e07a8`) while it runs. The second stack word (the frame time
/// `00888070` passes) is not read.
pub fn actor_update_animation_movement(
    e: &mut Engine,
    this: Ptr<Actor>,
    animation: Ptr,
    _unused_2: f32,
) {
    let player = e.mem.u32(PLAYER_CHARACTER);
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF71D]);
    if this.addr() == player {
        let count = e.mem.u8(PLAYER_UPDATE_NESTING);
        e.mem.set_u8(PLAYER_UPDATE_NESTING, count.wrapping_add(1));
    }
    if !animation.is_null() {
        let is_free_player =
            this.addr() == player && !e.call(PLAYER_STATE_TEST, &args![player]).bool();
        if this.addr() == player || mode_setting_is_one(e) {
            e.call(
                ANIMATION_UPDATE_MOVEMENT_NO_WORLD_UPDATE,
                &args![animation, this],
            );
        }
        let blocked = e.call(ACTOR_BLOCKED_TEST, &args![this]).bool()
            || e.vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
                .bool()
            || e.vcall(this.addr(), VSLOT_FLAG_0X2E8, &args![]).bool()
            || e.vcall(this.addr(), VSLOT_PACKAGE_TEST_0X230, &args![])
                .bool();
        let updates =
            !blocked && !e.call(REFERENCE_TEST_0X579670, &args![this]).bool() && !is_free_player;
        let mut step = e
            .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
            .f32();
        step = vats_scaled(e, this, step);
        if updates {
            e.vcall(this.addr(), VSLOT_UPDATE_0X4C8, &args![step]);
            e.vcall(this.addr(), VSLOT_UPDATE_0X4CC, &args![step]);
            e.vcall(this.addr(), VSLOT_UPDATE_0X494, &args![]);
        }
        if this.addr() == player || mode_setting_is_one(e) {
            let saved = e.call(PICK_COUNT_OVERRIDE_BYTE, &args![]).u8();
            if this.addr() == player {
                e.call(SET_PICK_COUNT_OVERRIDE, &args![1u32]);
            }
            if !is_free_player
                && e.call(ACTOR_MOVEMENT_STEP, &args![this, updates, step])
                    .bool()
            {
                e.call(ACTOR_RAGDOLL_FACE_STEP, &args![this, 1u32]);
            }
            if this.addr() == player {
                e.call(SET_PICK_COUNT_OVERRIDE, &args![saved as u32]);
                e.call(CONSOLIDATE_SIM_ISLANDS, &args![this]);
            }
        } else {
            e.call(ACTOR_MOVEMENT_STEP, &args![this, updates, step]);
        }
        e.call(
            ANIMATION_UPDATE_SCENE_GRAPH_NO_CONTROLLER,
            &args![animation],
        );
        let kind = e.vcall(this.addr(), VSLOT_KIND_0X214, &args![]).u32();
        let index = kind.wrapping_sub(3);
        // The compiler's byte table (`00888950`) maps kinds 3, 5, 8 and 10
        // to the case that rescales the actor; the others leave.
        if index <= 7 && SCALE_CASE_TABLE[index as usize] == 0 {
            let player_animation = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
            if animation.addr() != player_animation {
                let mut vector = [
                    e.mem.u32(SCALE_VECTOR),
                    e.mem.u32(SCALE_VECTOR + 4),
                    e.mem.u32(SCALE_VECTOR + 8),
                ];
                let scale = e.call(GET_SCALE, &args![this]).f32();
                if scale != 1.0 {
                    let flag = e.call(REFERENCE_FLAG_0X50D4A0, &args![this, 0u32]).u8();
                    e.with_stack(12, |e, slot| {
                        for (i, word) in vector.iter().enumerate() {
                            e.mem.set_u32(slot.addr() + 4 * i as u32, *word);
                        }
                        e.call(
                            ANIMATION_SCALE_VECTOR,
                            &args![animation, slot, this, flag as u32],
                        );
                        for (i, word) in vector.iter_mut().enumerate() {
                            *word = e.mem.u32(slot.addr() + 4 * i as u32);
                        }
                    });
                    let z = f32::from_bits(vector[2]);
                    if z != 0.0 {
                        let offset = (z as f64 - z as f64 * scale as f64) as f32;
                        let position = e
                            .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
                            .u32();
                        let height = e.mem.f32(position + 8);
                        let new_height = (height as f64 - offset as f64) as f32;
                        e.call(SET_LOCATION_ON_REFERENCE_Z, &args![this, new_height]);
                    }
                }
            }
        }
    }
    if this.addr() == player {
        let count = e.mem.u8(PLAYER_UPDATE_NESTING);
        e.mem.set_u8(PLAYER_UPDATE_NESTING, count.wrapping_sub(1));
    }
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_0X11DF71D]);
}

// Translated from 00888960 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global byte at `012677a3`.
pub fn fn_00888960(e: &mut Engine) -> u8 {
    e.mem.u8(GLOBAL_FLAG_012677A3)
}

// Translated from 00888970 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs the ragdoll controller's ragdoll animation (`00c7d630(1)`), then
/// copies the controller's two floats at `+0x19c`/`+0x1a0` (or zeros, when
/// the byte at `+0xb3` is clear) into the face animation data's `+0x150`
/// and `+0x154`, and resets the controller's `+0x23c` when `004955a0` says
/// so. The stack word is not read.
pub fn fn_00888970(e: &mut Engine, this: Ptr<Actor>, _unused_1: u32) {
    let controller = e.get(this, Actor::pRagdollController);
    e.call(DO_RAGDOLL_ANIM, &args![controller, 1u32]);
    let face = e.call(GET_FACE_ANIMATION_DATA, &args![this]).u32();
    if face != 0 {
        if fn_00888a50(e, controller) {
            let (first, second) = e.with_stack(8, |e, slot| {
                let first = slot.addr();
                let second = slot.addr() + 4;
                fn_00888a70(e, controller, Ptr::new(first), Ptr::new(second));
                (e.mem.f32(first), e.mem.f32(second))
            });
            fn_00888a20(e, Ptr::new(face), first, second);
        } else {
            fn_00888a20(e, Ptr::new(face), 0.0, 0.0);
        }
    }
    if !e.call(RAGDOLL_TEST_0X4955A0, &args![controller]).bool() {
        fn_00888aa0(e, controller);
    }
}

// Translated from 00888a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores two floats at `+0x150` and `+0x154`.
pub fn fn_00888a20(e: &mut Engine, this: Ptr, first: f32, second: f32) {
    e.mem.set_f32(this.addr() + 0x150, first);
    e.mem.set_f32(this.addr() + 0x154, second);
}

// Translated from 00888a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the byte at `+0xb3` of the ragdoll controller.
pub fn fn_00888a50(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0xb3) != 0
}

// Translated from 00888a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the floats at `+0x19c` and `+0x1a0` of the controller to the two
/// pointers.
pub fn fn_00888a70(e: &mut Engine, this: Ptr, first: Ptr, second: Ptr) {
    let a = e.mem.f32(this.addr() + 0x19c);
    e.mem.set_f32(first.addr(), a);
    let b = e.mem.f32(this.addr() + 0x1a0);
    e.mem.set_f32(second.addr(), b);
}

// Translated from 00888aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the `float` at `+0x23c` of the ragdoll controller.
pub fn fn_00888aa0(e: &mut Engine, this: Ptr) {
    e.mem.set_f32(this.addr() + 0x23c, 0.0);
}

// Translated from 00888ac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Process virtual `+0x2f0` with the actor and `value`.
pub fn fn_00888ac0(e: &mut Engine, this: Ptr<Actor>, value: f32) {
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), PROCESS_VSLOT_0X2F0, &args![this, value]);
}

// Translated from 00888af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Process virtual `+0x2f4`.
pub fn fn_00888af0(e: &mut Engine, this: Ptr<Actor>) {
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), PROCESS_VSLOT_0X2F4, &args![]);
}

// Translated from 00888b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Process virtual `+0x2f8`.
pub fn fn_00888b20(e: &mut Engine, this: Ptr<Actor>) {
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), PROCESS_VSLOT_0X2F8, &args![]);
}

/// The locals of `Actor::Update` (`00888b50`) that its sections share.
struct UpdateFrame {
    this: Ptr<Actor>,
    delta: f32,
    player: u32,
    /// The actor's 3D object (virtual `+0x1d0`).
    skeleton: u32,
    /// `Actor::GetFaceAnimationData` (`008adcb0`).
    face: u32,
    /// The actor's animation (virtual `+0x1e4`).
    animation: u32,
    /// `Actor::bInCombat` as `00493bb0` reads it (cleared for the player).
    in_combat: bool,
    /// `Actor::GetCurrentWeapon` (`008a1710`).
    weapon: u32,
    /// The process read at the start (`Actor::pCurrentProcess`).
    process: u32,
    /// The `BSSoundHandle` on the stack.
    sound: u32,
    /// The character controller (`MobileObject::GetCharController`).
    controller: u32,
    /// True for the player when `004eaf60` says so.
    player_flag: bool,
    /// The actor's parent cell.
    cell: u32,
}

// Translated from 00888b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::Update` (Xbox PDB): the actor's per-frame update with the frame
/// time `delta`: caches the sit/sleep state, scales the animation speed,
/// positions the actor's head target, updates the face data, the process
/// state (life state 6 healing), the weapon attack sound, the movement
/// mode, the 3D position, swimming and wading (the water height, breath,
/// drowning damage, splash effects and the controller's mode), the
/// creature sounds and the animation block flags. It does nothing
/// (apart from the cleanup) when the actor has no 3D object or cell, or
/// the cell fails `00450ff0`, and only wakes the process (virtual `+0x5f8`)
/// when `delta >= 900`. The exception frame is not translated. The
/// meaning of most callees and virtual slots is not confirmed; they are
/// described by what they take and return.
pub fn actor_update(e: &mut Engine, this: Ptr<Actor>, delta: f32) {
    e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_UPDATE]);
    e.call(ACTOR_RESET_FLOATS, &args![this]);
    let process = e.call(GET_PROCESS, &args![this]).u32();
    if process != 0 {
        let state = e.vcall(process, PROCESS_VSLOT_0X4BC, &args![]).u32();
        e.set(this, Actor::cCurrentSitSleepState, state);
    }
    let skeleton = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
    let face = e.call(GET_FACE_ANIMATION_DATA, &args![this]).u32();
    let animation = e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32();
    let in_combat = e.call(ACTOR_GET_IN_COMBAT, &args![this]).bool();
    let weapon = e.call(ACTOR_GET_CURRENT_WEAPON, &args![this]).u32();
    e.with_stack(0x10, |e, sound| {
        e.call(SOUND_HANDLE_INIT, &args![sound]);
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        let mut frame = UpdateFrame {
            this,
            delta,
            player: e.mem.u32(PLAYER_CHARACTER),
            skeleton,
            face,
            animation,
            in_combat,
            weapon,
            process,
            sound: sound.addr(),
            controller: 0,
            player_flag: false,
            cell,
        };
        if skeleton != 0 && cell != 0 && e.call(CELL_TEST_0X450FF0, &args![cell]).bool() {
            let limit = e.global::<f64>(UPDATE_TIME_LIMIT);
            if (delta as f64) >= limit {
                if process != 0 {
                    e.vcall(process, PROCESS_VSLOT_0X5F8, &args![1u32]);
                }
            } else {
                update_main(e, &mut frame);
            }
        }
        e.call(EMPTY_FUNCTION, &args![SCOPE_OBJECT_UPDATE]);
        e.call(EMPTY_FUNCTION, &args![sound.addr()]);
    });
}

/// The body of `Actor::Update` once the actor has a 3D object and a cell.
fn update_main(e: &mut Engine, f: &mut UpdateFrame) {
    let this = f.this;
    e.vcall(this.addr(), VSLOT_UPDATE_0X44C, &args![]);
    update_animation_speed(e, f);
    f.player_flag = this.addr() == f.player && !e.call(PLAYER_STATE_TEST, &args![f.player]).bool();
    f.controller = e.call(GET_CHAR_CONTROLLER, &args![this]).u32();
    if f.controller != 0 {
        update_controller_target(e, f);
    }
    update_face_and_life(e, f);
    update_weapon_sound(e, f);
    update_movement_mode(e, f);
    update_position(e, f);
    if !f.player_flag {
        update_water_and_sounds(e, f);
    }
    // 0088afcc
    e.call(ACTOR_TIMED_UPDATE, &args![this, f.delta]);
    e.call(ACTOR_END_OF_UPDATE, &args![this]);
}

/// `0088b030` on the actor's animation: `1 / [011d0fa8]` for a non-player
/// actor whose actor value `0x33` is positive, else `1.0`.
fn update_animation_speed(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    if this.addr() == f.player {
        return;
    }
    let owner = this.addr() + ACTOR_VALUE_OWNER_OFFSET;
    let value = e.vcall(owner, 8, &args![0x33u32]).i32();
    if value > 0 && e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32() != 0 {
        let setting = setting_value(e, ANIMATION_SPEED_SETTING);
        let scale = (1.0f64 / setting as f64) as f32;
        let animation = e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32();
        e.call(ANIMATION_SET_SPEED_SCALE, &args![animation, scale]);
        return;
    }
    if e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32() != 0 {
        let animation = e.vcall(this.addr(), VSLOT_GET_ANIMATION, &args![]).u32();
        e.call(ANIMATION_SET_SPEED_SCALE, &args![animation, 1.0f32]);
    }
}

/// The character controller part of `Actor::Update`: gives the controller
/// the point the actor's head looks at (for a non-player actor) and marks
/// whether the current attack is a power attack with a type below `0xa8`.
fn update_controller_target(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    let player = f.player;
    let controller = f.controller;
    if this.addr() != player {
        e.with_stack(0x24, |e, base| {
            let target = base.addr();
            let scaled = base.addr() + 0xc;
            let sum = base.addr() + 0x18;
            e.call(MATRIX_CONSTRUCT, &args![target]);
            let source = if e.call(PLAYER_STATE_TEST, &args![player]).bool()
                && e.mem.u32(HEAD_TARGET_A) != 0
            {
                let object = e.mem.u32(HEAD_TARGET_A);
                Some(e.call(OBJECT_POSITION_0X8C, &args![object]).u32())
            } else if e.mem.u32(HEAD_TARGET_B) != 0 {
                let object = e.mem.u32(HEAD_TARGET_B);
                Some(e.call(OBJECT_POSITION_0X8C, &args![object]).u32())
            } else {
                None
            };
            let source = match source {
                Some(address) => address,
                None => {
                    let eye = e.call(GET_EYE_LEVEL, &args![player]).f32();
                    let scaled_vector = e
                        .call(VECTOR_TIMES_SCALAR, &args![scaled, eye, EYE_DIRECTION])
                        .u32();
                    let position = e.call(POSITION_VECTOR_OF, &args![player]).u32();
                    e.call(VECTOR_PLUS_VECTOR, &args![position, sum, scaled_vector])
                        .u32()
                }
            };
            for offset in [0, 4, 8] {
                let word = e.mem.u32(source + offset);
                e.mem.set_u32(target + offset, word);
            }
            e.call(
                CONTROLLER_SET_TARGET,
                &args![controller, target, f.skeleton],
            );
        });
    }
    let mut power_attack = false;
    if f.animation != 0 {
        let group = e
            .call(ANIMATION_GROUP_FIELD, &args![f.animation, 4u32])
            .u16();
        if e.call(IS_POWER_ATTACK_ACTION, &args![group as u32]).bool()
            && e.call(ANIM_GROUP_GET_TYPE, &args![group as u32]).i32() < 0xa8
        {
            power_attack = true;
        } else {
            let group = e
                .call(ANIMATION_GROUP_FIELD, &args![f.animation, 2u32])
                .u16();
            if e.call(IS_POWER_ATTACK_ACTION, &args![group as u32]).bool()
                && e.call(ANIM_GROUP_GET_TYPE, &args![group as u32]).i32() < 0xa8
            {
                power_attack = true;
            }
        }
    }
    let controller_now = e.call(GET_CHAR_CONTROLLER, &args![this]).u32();
    let target = if controller_now != 0 {
        controller_now + 0x410
    } else {
        0
    };
    e.with_stack(4, |e, flag| {
        e.mem.set_u8(flag.addr(), power_attack as u8);
        fn_0088b070(e, Ptr::new(target), flag);
    });
}

/// The process and face part of `Actor::Update`: the face animation data's
/// two virtuals (`+0xd4`, `+0xd8`), the process virtual `+0x704`, and, in
/// life state 6, healing the actor and restoring its health once the
/// process says it is rested.
fn update_face_and_life(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    let face = f.face;
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        e.vcall(process.addr(), PROCESS_VSLOT_0X704, &args![this]);
        if e.get(this, Actor::cCurrentSitSleepState) == 9
            || e.call(ACTOR_BLOCKED_TEST, &args![this]).bool()
        {
            if face != 0 && !e.vcall(face, FACE_VSLOT_0XD4, &args![]).bool() {
                e.vcall(face, FACE_VSLOT_0XD8, &args![1u32, 0u32]);
            }
        } else if face != 0
            && !e
                .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
                .bool()
            && e.vcall(face, FACE_VSLOT_0XD4, &args![]).bool()
        {
            e.vcall(face, FACE_VSLOT_0XD8, &args![0u32, 0u32]);
        }
        if e.call(ACTOR_GET_LIFE_STATE, &args![this]).u32() == 6 {
            let level = e.vcall(process.addr(), PROCESS_VSLOT_0X40C, &args![]).u32();
            let level_ok =
                level == 3 || e.vcall(process.addr(), PROCESS_VSLOT_0X40C, &args![]).u32() == 1;
            if level_ok {
                if e.call(GET_FORCE_NEXT_UPDATE, &args![this]).bool() {
                    let player = e.mem.u32(PLAYER_CHARACTER);
                    if player == 0 || !e.call(PLAYER_IN_COMBAT, &args![player, 0u32]).bool() {
                        e.vcall(process.addr(), PROCESS_VSLOT_0XE0, &args![]);
                    }
                    let health = e.vcall(process.addr(), PROCESS_VSLOT_0XE4, &args![]).f64();
                    if health <= 0.0 {
                        e.call(ACTOR_SET_LIFE_STATE, &args![this, 0u32]);
                        e.call(ACTOR_RESTORE_FULL_HEALTH, &args![this]);
                        e.call(TRIGGER_FOLLOWER_BARK, &args![this, 0xbu32]);
                    }
                } else {
                    e.vcall(process.addr(), PROCESS_VSLOT_0XE0, &args![]);
                    let health = e.vcall(process.addr(), PROCESS_VSLOT_0XE4, &args![]).f64();
                    if health <= 0.0 {
                        e.call(ACTOR_SET_LIFE_STATE, &args![this, 0u32]);
                        e.call(ACTOR_RESTORE_FULL_HEALTH, &args![this]);
                    }
                }
            }
        }
    }
}

/// The weapon attack sound part of `Actor::Update`: an actor that is not
/// in the state `vcall(+0x22c)` tests and has an animation and a weapon
/// keeps the attack sound of its extra data in step with the attack
/// animation: stops it when the attack ends, or starts it for a looping
/// attack animation.
fn update_weapon_sound(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    let animation = f.animation;
    let weapon = f.weapon;
    let sound = f.sound;
    if e.vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
        .bool()
        || animation == 0
        || weapon == 0
    {
        return;
    }
    let weapon_object = e.call(WEAPON_OBJECT_OF, &args![weapon]).u32();
    if weapon_object == 0 {
        return;
    }
    let weapon_object = e.call(WEAPON_OBJECT_OF, &args![weapon]).u32();
    if e.call(WEAPON_OBJECT_TEST, &args![weapon_object]).u32() == 0 {
        return;
    }
    if e.call(GET_LAST_BOUND_WEAPON, &args![weapon]).u32() == 0x2d
        && e.call(WEAPON_TYPE_0X51F5F0, &args![weapon]).u32() != 0x50
    {
        return;
    }
    let in_range = e.call(ACTOR_RANGE_TEST, &args![this]).bool();
    let attack_state = if !in_range
        || e.vcall(this.addr(), VSLOT_PACKAGE_TEST_0X230, &args![])
            .bool()
    {
        true
    } else if !e.call(ACTOR_RANGE_TEST, &args![this]).bool() {
        false
    } else {
        e.call(ANIMATION_FIELD_0X70F490, &args![animation, 4u32])
            .i32()
            > 1
    };
    if attack_state {
        let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
        e.call(EXTRA_GET_WEAPON_ATTACK_SOUND, &args![lists, sound]);
        if e.call(SOUND_HANDLE_IS_PLAYING, &args![sound]).bool() {
            e.call(SOUND_HANDLE_STOP, &args![sound]);
            if e.call(WEAPON_TEST_0X524B40, &args![weapon]).bool() {
                let entry = e.call(ANIMATION_LOOKUP, &args![animation, 4u32]).u32();
                if entry != 0 {
                    let entry = e.call(ANIMATION_LOOKUP, &args![animation, 4u32]).u32();
                    let step = e.call(ANIMATION_STEP, &args![entry]).u32();
                    if e.call(ANIMATION_ENTRY_TEST_0X4937E0, &args![step]).bool() {
                        let entry = e.call(ANIMATION_LOOKUP, &args![animation, 4u32]).u32();
                        if e.call(ANIMATION_ENTRY_KIND, &args![entry]).u32() == 1 {
                            if this.addr() == f.player {
                                let first =
                                    e.call(PLAYER_GET_ANIMATION, &args![f.player, 1u32]).u32();
                                e.call(ANIMATION_BLEND_OUT, &args![first, 4u32, 0u32]);
                                let second =
                                    e.call(PLAYER_GET_ANIMATION, &args![f.player, 0u32]).u32();
                                e.call(ANIMATION_BLEND_OUT, &args![second, 4u32, 0u32]);
                            } else {
                                e.call(ANIMATION_BLEND_OUT, &args![animation, 4u32, 0u32]);
                            }
                        }
                    }
                }
            }
        }
        let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
        e.call(EXTRA_SET_WEAPON_ATTACK_SOUND, &args![lists, sound]);
        return;
    }
    if !e.call(ACTOR_RANGE_TEST, &args![this]).bool() {
        return;
    }
    if e.call(ANIMATION_LOOKUP, &args![animation, 4u32]).u32() == 0 {
        return;
    }
    let group = e.call(ANIMATION_GROUP_FIELD, &args![animation, 4u32]).u16();
    if !e.call(IS_ATTACK_ACTION, &args![group as u32]).bool() {
        return;
    }
    let entry = e.call(ANIMATION_LOOKUP, &args![animation, 4u32]).u32();
    if e.call(ANIMATION_ENTRY_KIND, &args![entry]).u32() != 1 {
        return;
    }
    if e.call(ANIMATION_FIELD_0X70F490, &args![animation, 4u32])
        .i32()
        >= 1
    {
        return;
    }
    let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
    e.call(EXTRA_GET_WEAPON_ATTACK_SOUND, &args![lists, sound]);
    if !e.call(SOUND_HANDLE_IS_VALID, &args![sound]).bool() {
        start_weapon_attack_sound(e, f);
    } else if !e.call(SOUND_HANDLE_IS_PLAYING, &args![sound]).bool() {
        e.call(SOUND_HANDLE_PLAY, &args![sound, 0u32]);
    }
    let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
    e.call(EXTRA_SET_WEAPON_ATTACK_SOUND, &args![lists, sound]);
}

/// Creates the sound handle for the weapon attack sound of `Actor::Update`
/// (the weapon's sound file with flags depending on whether the actor is
/// the player), follows the actor's 3D object (or the virtual `+0x20c`
/// object) and sets its position.
fn start_weapon_attack_sound(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    let weapon = f.weapon;
    let mut follow = e.vcall(this.addr(), VSLOT_FOLLOW_0X20C, &args![]).u32();
    if follow == 0 {
        follow = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
    }
    e.with_stack(0x20, |e, scratch| {
        let item_scratch = scratch.addr();
        let handle_scratch = scratch.addr() + 0x10;
        let sound_object = e.call(WEAPON_OBJECT_OF, &args![weapon]).u32();
        let item = e
            .call(WEAPON_SOUND_ITEM, &args![sound_object, item_scratch])
            .u32();
        let word = e.mem.u32(item + 4);
        let sound_flags = e.call(SOUND_FLAGS_OF, &args![word]).u32();
        let mut flags: u32 = if this.addr() != f.player {
            0xf800_0001u32.wrapping_add(0x0800_0001)
        } else {
            0x0800_0001
        };
        if this.addr() == f.player && !e.call(PLAYER_TEST_0X524D10, &args![f.player]).bool() {
            flags |= 0x4_0000;
        }
        flags |= 0x1_0000;
        let first = e.call(WEAPON_OBJECT_OF, &args![weapon]).u32();
        let third = e.call(WEAPON_OBJECT_OF, &args![weapon]).u32();
        let name = e.call(WEAPON_SOUND_NAME, &args![third]).u32();
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        let made = e
            .call(
                AUDIO_GET_SOUND_HANDLE_BY_FILENAME,
                &args![
                    audio,
                    handle_scratch,
                    name,
                    sound_flags | flags | 0x4000_0000,
                    first
                ],
            )
            .u32();
        e.call(SOUND_HANDLE_ASSIGN, &args![f.sound, made]);
        e.call(EMPTY_FUNCTION, &args![handle_scratch]);
    });
    e.call(SOUND_HANDLE_SET_0X4F15A0, &args![f.sound, 1u32]);
    e.call(SOUND_HANDLE_SET_OBJECT_TO_FOLLOW, &args![f.sound, follow]);
    let position = e
        .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
        .u32();
    e.call(SOUND_HANDLE_SET_POSITION_PTR, &args![f.sound, position]);
}

/// The movement mode part of `Actor::Update` for a non-player actor (the
/// player clears `in_combat` instead): the blocked flag, and the
/// `ActorMover` flags (`0x400`) kept in step with the follow and weapon
/// conditions.
fn update_movement_mode(e: &mut Engine, f: &mut UpdateFrame) {
    let this = f.this;
    let player = f.player;
    if this.addr() == player {
        f.in_combat = false;
        return;
    }
    let mut package = 0u32;
    if !e.get(this, Actor::pCurrentProcess).is_null() {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        package = e.vcall(process, PROCESS_VSLOT_0X27C, &args![]).u32();
    }
    if f.in_combat {
        return;
    }
    let typed = package != 0
        && (e.call(OBJECT_TYPE, &args![package]).u32() == 8
            || e.call(OBJECT_TYPE, &args![package]).u32() == 0x10);
    if !typed && e.call(ACTOR_GET_BLOCKED, &args![this]).bool() {
        e.call(ACTOR_SET_BLOCK, &args![this, 0u32]);
    }
    let follow_state = e.call(ACTOR_MOVER_FLAG_TEST, &args![this]).bool()
        || (package != 0 && e.call(PACKAGE_TEST_0X67A4F0, &args![package]).bool())
        || (e.call(GET_FORCE_NEXT_UPDATE, &args![this]).bool()
            && e.call(SPEED_FLAG_TEST, &args![player]).bool()
            && e.call(PLAYER_TEST_0X9549A0, &args![player, this]).bool());
    if follow_state {
        if !e.call(SPEED_FLAG_TEST, &args![this]).bool() {
            fn_00884f80(e, this);
        }
        let mover = e.get(this, Actor::pActorMover);
        if !mover.is_null() {
            let mode = e.vcall(mover.addr(), MOVER_VSLOT_MODE, &args![]).u32();
            if mode & 0x400 == 0 {
                let counter = e.global::<f32>(FOLLOWER_BARK_COUNTDOWN);
                if counter <= 0.0 {
                    e.call(TRIGGER_FOLLOWER_BARK, &args![this, 0xcu32]);
                    let reset = e.global::<f32>(FOLLOWER_BARK_RESET);
                    e.set_global(FOLLOWER_BARK_COUNTDOWN, reset);
                }
            }
        }
        let mover = e.get(this, Actor::pActorMover);
        let mode = e.vcall(mover.addr(), MOVER_VSLOT_MODE, &args![]).u32();
        e.call(ACTOR_SET_MOVE_MODE, &args![this, mode | 0x400]);
        return;
    }
    let mover = e.get(this, Actor::pActorMover);
    let clear = e.call(SPEED_FLAG_TEST, &args![this]).bool()
        || (e.call(GET_FORCE_NEXT_UPDATE, &args![this]).bool()
            && !mover.is_null()
            && e.vcall(mover.addr(), MOVER_VSLOT_MODE, &args![]).u32() & 0x400 != 0);
    if clear {
        fn_00884f80(e, this);
        let word = e.call(GET_FLAG_WORD, &args![this]).u32();
        e.call(ACTOR_SET_MOVE_MODE, &args![this, word & !0x400u32]);
    }
}

/// The position part of `Actor::Update`: the player's 3D object is placed
/// at the actor's position (`00440460`), any other actor goes through
/// [`actor_update_actor_3d_position`] and the process virtual `+0x428`;
/// then the lighting of the extra scene node is refreshed and the
/// `ProcessLists` flag byte is cleared.
fn update_position(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    if f.player_flag {
        let position = e.call(POSITION_VECTOR_OF, &args![this]).u32();
        e.call(SET_WORLD_POSITION, &args![f.skeleton, position]);
        e.call(PLAYER_UPDATE_0X952290, &args![f.player]);
    } else {
        actor_update_actor_3d_position(e, this);
        let process = e.get(this, Actor::pCurrentProcess);
        e.vcall(process.addr(), PROCESS_VSLOT_0X428, &args![this]);
    }
    let link = e.call(GET_LINKED_OBJECT, &args![this]).u32();
    if link != 0 && e.call(WORD_AT_START, &args![link]).u32() != 0 {
        let word = e.call(WORD_AT_START, &args![link]).u32();
        let node = e.call(SHADOW_SCENE_NODE, &args![0u32]).u32();
        e.call(SHADOW_SCENE_NODE_UPDATE_0XB5F080, &args![node, word]);
    }
    fn_0088b020(e, 0);
}

/// The rest of `Actor::Update` for an actor other than the player: the
/// swim and water section (when it has a character controller and no
/// menu is open) and the animation block flags.
fn update_water_and_sounds(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    if f.controller != 0
        && !e.call(IS_IN_MENU_MODE, &args![]).bool()
        && !e
            .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
            .bool()
    {
        swim_and_water(e, f);
    }
    animation_block_flags(e, f);
}

/// The process of `this` as read from `+0x68`.
fn process_of(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    e.get(this, Actor::pCurrentProcess).addr()
}

/// The swim and water section of `Actor::Update`.
fn swim_and_water(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    let position = e.call(POSITION_VECTOR_OF, &args![this]).u32();
    let z = e.mem.f32(position + 8);
    let depth = fn_00885560(e, this, z, Ptr::new(f.cell));
    let in_water = depth as f64 >= e.global::<f64>(WATER_THRESHOLD_A);
    let wading = depth as f64 >= e.global::<f64>(WATER_THRESHOLD_B);
    let deep = depth as f64 >= e.global::<f64>(WATER_THRESHOLD_C);
    let endurance = e.call(ACTOR_GET_ENDURANCE, &args![this]).f64();
    let ticks = e.call(FTOL2, &args![endurance]).u32();
    let breath_time = e.call(SWIM_BREATH_TIME, &args![ticks]).f32();
    let was_in_water = e.get(this, Actor::bInWater);
    let just_entered = !was_in_water && in_water;
    e.set(this, Actor::bInWater, in_water);
    let mut breathing = true;
    if depth as f64 <= 0.0 {
        breathing = e.call(ACTOR_TEST_0X87F620, &args![this]).bool();
    } else if deep {
        let magic = this.addr() + 0x94;
        if !e.vcall(magic, MAGIC_VSLOT_0X10, &args![]).bool()
            && !e.call(ACTOR_TEST_0X87F5C0, &args![this]).bool()
        {
            breathing = false;
        }
    }
    if breathing {
        let process = process_of(e, this);
        e.vcall(process, PROCESS_VSLOT_0X2FC, &args![breath_time]);
    } else {
        drowning_step(e, f, breath_time);
    }
    let creature_sound = creature_sound_step(e, f);
    let _ = just_entered;
    controller_mode_step(e, f, creature_sound, in_water, wading, just_entered);
    water_effects(e, f, in_water, wading, creature_sound);
}

/// The breath and drowning part of the swim section: the process's
/// breath timer (virtual `+0x300`) runs down by the frame step; when it
/// is spent the actor takes drowning damage and the pain sound
/// plays; otherwise the timer is capped by the breath time, and an actor
/// with the surfacing conditions starts the surface package.
fn drowning_step(e: &mut Engine, f: &UpdateFrame, breath_time: f32) {
    let this = f.this;
    let player = f.player;
    let process = process_of(e, this);
    let timer = e.vcall(process, PROCESS_VSLOT_0X300, &args![]).f32();
    let step = e
        .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
        .f64();
    let mut remaining = (timer as f64 - step) as f32;
    if remaining < 0.0 {
        if e.vcall(this.addr(), VSLOT_ACTOR_FLAG_0X360, &args![])
            .bool()
        {
            let owner = this.addr() + ACTOR_VALUE_OWNER_OFFSET;
            let value = e.vcall(owner, 0, &args![0x10u32]).u32();
            let damage_rate = e.call(DAMAGE_RATE, &args![value]).f64();
            let step = e
                .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
                .f64();
            let damage = (step * damage_rate) as f32;
            e.vcall(
                this.addr(),
                VSLOT_DAMAGE_0X338,
                &args![damage, 0.0f32, 0u32],
            );
            e.with_stack(0x10, |e, handle| {
                e.call(SOUND_HANDLE_INIT, &args![handle]);
                let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
                e.call(EXTRA_GET_SOUND, &args![lists, handle]);
                if !e.call(SOUND_HANDLE_IS_VALID, &args![handle]).bool() {
                    e.with_stack(0x10, |e, scratch| {
                        let made = e
                            .call(
                                PLAY_SOUND_BY_EDITOR_NAME,
                                &args![this, scratch, DROWNING_SOUND_NAME, 0u32, 0x102u32, 0u32],
                            )
                            .u32();
                        e.call(SOUND_HANDLE_ASSIGN, &args![handle, made]);
                        e.call(EMPTY_FUNCTION, &args![scratch]);
                    });
                    let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
                    e.call(EXTRA_SET_SOUND, &args![lists, handle]);
                }
                e.call(ACTOR_TRIGGER_PAIN, &args![this, 0u32, 1u32]);
                e.call(EMPTY_FUNCTION, &args![handle]);
            });
        }
        remaining = 0.0;
    } else if this.addr() != player {
        if breath_time < remaining {
            remaining = breath_time;
        }
        let speed = fn_00885d90(e, this);
        let water = e.call(GET_RELEVANT_WATER_HEIGHT, &args![this]).f64();
        let position = e
            .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
            .u32();
        let height = e.mem.f32(position + 8);
        let depth = (water - height as f64) as f32;
        let ratio = (depth as f64 / speed as f64) as f32;
        let scaled = (ratio as f64 * e.global::<f64>(DROWN_TIME_FACTOR)) as f32;
        let one = e.global::<f64>(DOUBLE_ONE);
        if (remaining as f64) < scaled as f64 + one
            && !e.call(ACTOR_TEST_0X87F570, &args![this]).bool()
            && !e.call(ACTOR_IS_SURFACING, &args![this]).bool()
        {
            e.call(ACTOR_INITIATE_SURFACE_PACKAGE, &args![this]);
        }
    }
    let process = process_of(e, this);
    e.vcall(process, PROCESS_VSLOT_0X2FC, &args![remaining]);
}

/// The creature sound part of the swim section: while the actor passes
/// `vcall(+0x21c)`, its creature "awake" sound (`PickCreatureSound` with 11)
/// is created, positioned and played, or stopped. Returns the byte
/// `0087f390` gave (the creature flag the rest of the section uses).
fn creature_sound_step(e: &mut Engine, f: &UpdateFrame) -> bool {
    let this = f.this;
    let mut creature = false;
    if !e
        .vcall(this.addr(), VSLOT_ANIMATION_FLAG_0X21C, &args![])
        .bool()
    {
        return creature;
    }
    creature = e.call(ACTOR_TEST_0X87F390, &args![this]).bool();
    let form = e.call(SAVE_FORM_BUFFER_GET_FORM, &args![this]).u32();
    let sounds = if form != 0 {
        e.call(PICK_CREATURE_SOUND, &args![form, 0xbu32]).u32()
    } else {
        0
    };
    if sounds == 0 || e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32() == 0 {
        return creature;
    }
    let awake = !e.call(ACTOR_BLOCKED_TEST, &args![this]).bool()
        && !e
            .vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
            .bool();
    e.with_stack(0x10, |e, handle| {
        e.call(SOUND_HANDLE_INIT, &args![handle]);
        let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
        e.call(EXTRA_GET_CREATURE_AWAKE_SOUND, &args![lists, handle]);
        if awake && !e.call(SOUND_HANDLE_IS_PLAYING, &args![handle]).bool() {
            if !e.call(SOUND_HANDLE_IS_VALID, &args![handle]).bool() {
                let item = e.with_stack(0x10, |e, scratch| {
                    e.call(WEAPON_SOUND_ITEM, &args![sounds, scratch]).u32()
                });
                let word = e.mem.u32(item + 4);
                let flags = e.call(SOUND_FLAGS_OF, &args![word]).u32() | 2;
                let name = e.call(WEAPON_SOUND_NAME, &args![sounds]).u32();
                let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                e.with_stack(0x10, |e, scratch| {
                    let made = e
                        .call(
                            AUDIO_GET_SOUND_HANDLE_BY_FILENAME,
                            &args![audio, scratch, name, flags, sounds],
                        )
                        .u32();
                    e.call(SOUND_HANDLE_ASSIGN, &args![handle, made]);
                    e.call(EMPTY_FUNCTION, &args![scratch]);
                });
                let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
                e.call(EXTRA_SET_CREATURE_AWAKE_SOUND, &args![lists, handle]);
                let follow = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
                e.call(SOUND_HANDLE_SET_OBJECT_TO_FOLLOW, &args![handle, follow]);
            }
            if !e.call(SOUND_HANDLE_IS_PLAYING, &args![handle]).bool() {
                let position = e
                    .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
                    .u32();
                e.call(SOUND_HANDLE_SET_POSITION_PTR, &args![handle, position]);
                e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
            }
        } else if e.call(SOUND_HANDLE_IS_PLAYING, &args![handle]).bool()
            && (!awake
                || e.vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
                    .bool())
        {
            e.call(SOUND_HANDLE_STOP, &args![handle]);
            e.call(SOUND_HANDLE_RELEASE, &args![handle]);
        }
        e.call(EMPTY_FUNCTION, &args![handle]);
    });
    creature
}

/// Sets the character controller's move mode word and the process's
/// cached-values flags the way `Actor::Update` does after a move mode
/// change: when the process has a cached-values block, `0x1000` and
/// `0x2000` are forwarded to it.
fn forward_cached_flags(e: &mut Engine, this: Ptr<Actor>) {
    let process = process_of(e, this);
    if process != 0 && e.call(HAS_CACHED_VALUES, &args![process]).bool() {
        let process = process_of(e, this);
        e.call(PROCESS_FORWARD_FLAG_MASK, &args![process, 0x1000u32]);
        let process = process_of(e, this);
        e.call(PROCESS_FORWARD_FLAG_MASK, &args![process, 0x2000u32]);
    }
}

/// `ActorMover` virtual `+0x1c` (the mode word) of the actor's mover.
fn mover_mode(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = e.get(this, Actor::pActorMover);
    e.vcall(mover.addr(), MOVER_VSLOT_MODE, &args![]).u32()
}

/// Stores the water height the call `TES::GetWaterHeight`-like `0045cbc0`
/// gives for the actor's position into the float at +8 of the object at
/// `this + 0x64`.
fn store_water_height(e: &mut Engine, this: Ptr<Actor>) {
    let position = e
        .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
        .u32();
    let tes = e.mem.u32(TES_POINTER);
    let height = e
        .call(TES_WATER_HEIGHT_AT, &args![tes, position, 0u32])
        .f32();
    let target = e.mem.u32(this.addr() + 0x64);
    e.mem.set_f32(target + 8, height);
}

/// The controller mode part of the swim section of `Actor::Update`
/// (`00889ffa` to `0088a7f0`): the controller's `0088b110` value, then for
/// creatures the water-height and wading mode handling and the
/// velocity modifier, and for the others the wading, leaving-water and
/// splash handling.
fn controller_mode_step(
    e: &mut Engine,
    f: &UpdateFrame,
    creature: bool,
    in_water: bool,
    wading: bool,
    just_entered: bool,
) {
    let this = f.this;
    let player = f.player;
    let controller = f.controller;
    let value_address = if creature {
        CONTROLLER_VALUE_CREATURE
    } else if this.addr() == player && !e.call(PLAYER_TEST_0X524D10, &args![player]).bool() {
        CONTROLLER_VALUE_PLAYER
    } else {
        CONTROLLER_VALUE_OTHER
    };
    let value = e.global::<f32>(value_address);
    fn_0088b110(e, Ptr::new(controller), value);
    if creature {
        creature_mode(e, f, in_water, wading);
    } else {
        other_mode(e, f, wading, just_entered);
    }
}

/// The creature branch of [`controller_mode_step`].
fn creature_mode(e: &mut Engine, f: &UpdateFrame, in_water: bool, wading: bool) {
    let this = f.this;
    let controller = f.controller;
    if e.call(CONTROLLER_STATE, &args![controller]).u32() == 5 {
        let target = e.mem.u32(this.addr() + 0x64);
        if e.mem.f32(target + 8) as f64 == e.global::<f64>(WATER_HEIGHT_UNSET) {
            store_water_height(e, this);
        }
    }
    if wading {
        let mode = mover_mode(e, this);
        e.call(ACTOR_SET_MOVE_MODE, &args![this, mode | 0x800]);
    } else {
        e.call(ACTOR_CLEAR_MOVE_MODE, &args![this, 0x800u32]);
    }
    forward_cached_flags(e, this);
    if e.call(CONTROLLER_STATE, &args![controller]).u32() == 5 && !in_water {
        e.call(CONTROLLER_SET_0X520, &args![controller, 0u32]);
        let target = e.mem.u32(this.addr() + 0x64);
        let height = e.global::<f32>(NO_WATER_HEIGHT);
        e.mem.set_f32(target + 8, height);
    } else if e.call(CONTROLLER_STATE, &args![controller]).u32() != 5 && in_water {
        let value = indexed_float(e, controller + 0x4f0, 2);
        if (value as f64) < e.global::<f64>(DOUBLE_ONE) {
            let water = e.call(GET_RELEVANT_WATER_HEIGHT, &args![this]).f64();
            if water == e.global::<f64>(WATER_HEIGHT_UNSET) {
                store_water_height(e, this);
            }
            e.call(CONTROLLER_SET_0X520, &args![controller, 5u32]);
        }
    }
    if e.call(CONTROLLER_STATE, &args![controller]).u32() == 5 {
        let limit = e.global::<f64>(WATER_THRESHOLD_A);
        let current = e.mem.f32(controller + 0x524);
        if (current as f64) < limit && in_water && !wading {
            let base = fn_0088b130(e, Ptr::new(controller));
            let low = indexed_float(e, base, 2);
            let proxy = e.call(CHARACTER_PROXY_POSITION, &args![controller]).u32();
            let high = indexed_float(e, proxy, 2);
            let factor = e.global::<f64>(CONTROLLER_FACTOR_A);
            let blend = ((high as f64 - low as f64)
                + (low as f64 * factor * e.mem.f32(controller + 0x55c) as f64))
                as f32;
            let reference = (e.mem.f32(controller + 0x53c) as f64
                - e.global::<f64>(CONTROLLER_OFFSET_A)) as f32;
            let drop = e
                .call(CONTROLLER_FUNCTION_0X4A3E90, &args![reference])
                .f64();
            let excess = (blend as f64 - drop) as f32;
            if (excess as f64) < factor {
                velocity_modifier(e, controller, CONTROLLER_PUSH_DOWN);
            } else if excess as f64 > factor {
                velocity_modifier(e, controller, CONTROLLER_PUSH_UP);
            }
        }
    }
}

/// `bhkCharacterController::SetVelocityModifier(controller, (0, 0, z), 1.0)`
/// with `z` the float at `z_address`.
fn velocity_modifier(e: &mut Engine, controller: u32, z_address: u32) {
    let z = e.global::<f32>(z_address);
    e.with_stack(0xc, |e, vector| {
        let made = e
            .call(NI_POINT3_SET, &args![vector, 0.0f32, 0.0f32, z])
            .u32();
        e.call(SET_VELOCITY_MODIFIER, &args![controller, made, 1.0f32]);
    });
}

/// The branch of [`controller_mode_step`] for the other actors.
fn other_mode(e: &mut Engine, f: &UpdateFrame, wading: bool, just_entered: bool) {
    let this = f.this;
    let controller = f.controller;
    if wading {
        let mode = mover_mode(e, this);
        e.call(ACTOR_SET_MOVE_MODE, &args![this, mode | 0x800]);
        e.call(CONTROLLER_SET_0X520, &args![controller, 5u32]);
        e.call(ACTOR_TEST_0X8A6840, &args![this, 0u32]);
        forward_cached_flags(e, this);
        return;
    }
    if e.call(ACTOR_TEST_0X5A2030, &args![this]).bool()
        || e.call(GET_FLAG_WORD, &args![this]).u32() & 0x800 != 0
        || e.call(CONTROLLER_STATE, &args![controller]).u32() == 5
    {
        if e.call(CONTROLLER_FLAG_0X88B0C0, &args![controller]).bool() {
            e.call(ACTOR_CLEAR_MOVE_MODE, &args![this, 0x800u32]);
            e.call(CONTROLLER_SET_0X520, &args![controller, 0u32]);
            forward_cached_flags(e, this);
        }
        return;
    }
    if just_entered {
        splash_sound_and_effects(e, f);
    }
}

/// The splash part of [`other_mode`]: when the actor has just entered the
/// water and the controller's velocity is high enough, plays the splash
/// sound at the actor's position and, over non-dangerous water, the
/// splash particle effects.
fn splash_sound_and_effects(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    let controller = f.controller;
    let speed = e.call(CONTROLLER_SPEED_0X88B0F0, &args![controller]).f64();
    let fast_enough = speed > e.global::<f64>(DOUBLE_HALF);
    if !fast_enough {
        return;
    }
    let velocity = indexed_float(e, controller + 0x4f0, 2);
    let slow_enough = (velocity as f64) < e.global::<f64>(CONTROLLER_LIMIT_B);
    if !slow_enough {
        return;
    }
    let velocity = indexed_float(e, controller + 0x4f0, 2);
    let mut volume = 1.0f32;
    if (velocity as f64) > e.global::<f64>(CONTROLLER_LIMIT_C) {
        volume = (velocity as f64 * e.global::<f64>(CONTROLLER_LIMIT_B)
            / e.global::<f64>(CONTROLLER_DIVISOR)) as f32;
    }
    let cap = e.global::<f32>(VOLUME_CAP);
    volume = e.call(CLAMP_MIN_4DD150, &args![volume, cap]).f32();
    e.with_stack(0x10, |e, handle| {
        e.call(SOUND_HANDLE_INIT, &args![handle]);
        if e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32() == 0 {
            e.call(EMPTY_FUNCTION, &args![handle]);
            return;
        }
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        let made = e.with_stack(0x10, |e, scratch| {
            let made = e
                .call(
                    AUDIO_GET_SOUND_HANDLE_BY_NAME,
                    &args![audio, scratch, SPLASH_SOUND_NAME, 0x102u32],
                )
                .u32();
            e.call(SOUND_HANDLE_ASSIGN, &args![handle, made]);
            e.call(EMPTY_FUNCTION, &args![scratch]);
            made
        });
        let _ = made;
        if e.call(SOUND_HANDLE_IS_PLAYING, &args![handle]).bool() {
            e.call(EMPTY_FUNCTION, &args![handle]);
            return;
        }
        let position = e
            .vcall(this.addr(), VSLOT_GET_POSITION_0X1F4, &args![])
            .u32();
        let (x, y, z) = (
            e.mem.u32(position),
            e.mem.u32(position + 4),
            e.mem.u32(position + 8),
        );
        e.call(SOUND_HANDLE_SET_POSITION, &args![handle, x, y, z]);
        e.call(SOUND_HANDLE_SET_VOLUME, &args![handle, volume]);
        e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
        if f.cell != 0 {
            let water_type = e.call(CELL_GET_WATER_TYPE, &args![f.cell]).u32();
            if water_type != 0 {
                let water_type = e.call(CELL_GET_WATER_TYPE, &args![f.cell]).u32();
                if !e.call(WATER_FORM_GET_DANGEROUS, &args![water_type]).bool() {
                    splash_particles(e, f, [x, y, z]);
                }
            }
        }
        e.call(EMPTY_FUNCTION, &args![handle]);
    });
}

/// `TESWaterListener::PlaySplashEffects` for the splash at `position`
/// (the water height replaces its z), with a random rotation.
fn splash_particles(e: &mut Engine, f: &UpdateFrame, position: [u32; 3]) {
    let this = f.this;
    let water_height = e.call(GET_RELEVANT_WATER_HEIGHT, &args![this]).f32();
    let at_water = [position[0], position[1], water_height.to_bits()];
    let matrix_words = e.with_stack(0x24, |e, matrix| {
        e.call(MATRIX_CONSTRUCT, &args![matrix]);
        let angle = e.call(RANDOM_ANGLE, &args![]).f32();
        e.call(MATRIX_MAKE_ROTATION, &args![matrix, angle]);
        let mut words = [0u32; 9];
        for (i, word) in words.iter_mut().enumerate() {
            *word = e.mem.u32(matrix.addr() + 4 * i as u32);
        }
        words
    });
    let rate_address = e
        .call(SETTING_VALUE_ADDRESS, &args![SPLASH_SETTING_A])
        .u32();
    let rate = e.mem.f32(rate_address);
    let count = e.call(SETTING_INT_FN, &args![SPLASH_SETTING_B]).u32();
    let mut arguments = args![f.cell, 1.0f32, count];
    for word in matrix_words {
        arguments.push(word);
    }
    for word in at_water {
        arguments.push(word);
    }
    arguments.push(rate.to_bits());
    arguments.push(1);
    arguments.push(0);
    e.call(CREATE_SPLASH_EFFECT, &arguments);
    let object = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
    let size = e.call(OBJECT_SIZE_0X87CE50, &args![object]).f32();
    e.call(
        PLAY_SPLASH_EFFECTS,
        &args![size, position[0], position[1], position[2]],
    );
}

/// The damage and breath part of the swim section (`0088a7f0` to
/// `0088aaad`): the player's process virtual `+0x760`, the drowning
/// damage of dangerous water, and the ProcessLists flag byte.
fn water_effects(e: &mut Engine, f: &UpdateFrame, in_water: bool, wading: bool, creature: bool) {
    let this = f.this;
    let player = f.player;
    if this.addr() == player && f.process != 0 {
        e.vcall(f.process, PROCESS_VSLOT_0X760, &args![0.0f32]);
    }
    if !in_water || creature {
        fn_0088b050(e, Ptr::new(PROCESS_LISTS), 0);
        return;
    }
    if f.cell == 0 {
        return;
    }
    let water_type = e.call(CELL_GET_WATER_TYPE, &args![f.cell]).u32();
    if water_type != 0 {
        let water_type = e.call(CELL_GET_WATER_TYPE, &args![f.cell]).u32();
        if e.call(WATER_FORM_GET_DANGEROUS, &args![water_type]).bool() {
            let sub = water_type + 0x24;
            let rate = e.vcall(sub, 0x10, &args![]).u16();
            let step = e
                .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
                .f64();
            let damage = (step * rate as f64) as f32;
            if damage as f64 > 0.0 {
                e.vcall(
                    this.addr(),
                    VSLOT_DAMAGE_0X338,
                    &args![damage, 0.0f32, 0u32],
                );
                e.call(ACTOR_TRIGGER_PAIN, &args![this, 1u32, 1u32]);
            }
        }
    }
    if water_type == 0 {
        return;
    }
    let layers = e.call(WATER_TYPE_COUNT, &args![water_type]).i32();
    if layers <= 0 {
        return;
    }
    if !e
        .vcall(this.addr(), VSLOT_ACTOR_FLAG_0X360, &args![])
        .bool()
        && !e
            .vcall(this.addr(), VSLOT_ANIMATION_FLAG_0X21C, &args![])
            .bool()
    {
        return;
    }
    let mut amount = layers as f32;
    let factor = if wading || e.call(GET_FLAG_WORD, &args![this]).u32() & 0x800 != 0 {
        setting_value(e, WATER_DAMAGE_SETTING_A)
    } else {
        setting_value(e, WATER_DAMAGE_SETTING_B)
    };
    amount = (amount as f64 * factor as f64) as f32;
    let resistance = e
        .call(ACTOR_GET_RADIATION_RESISTANCE_MULT, &args![this])
        .f32();
    amount = (amount as f64 * resistance as f64) as f32;
    if this.addr() == player && f.process != 0 {
        e.vcall(f.process, PROCESS_VSLOT_0X760, &args![amount]);
        e.call(PLAYER_RADIATION_0X968730, &args![player]);
    }
    let step = e
        .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
        .f64();
    let scaled = (step * amount as f64) as f32;
    if this.addr() == player {
        fn_0088b050(e, Ptr::new(PROCESS_LISTS), (scaled as f64 > 0.0) as u8);
    }
    e.vcall(this.addr(), VSLOT_0X3AC, &args![0x36u32, scaled, 0u32]);
}

/// The last part of `Actor::Update` for an actor that is not the player
/// (`0088aaad` to `0088afcc`): when it is dead-animated (virtual `+0x2e8`)
/// the animation block is managed (a blend out, the face animation data,
/// clearing the interpolators), otherwise the process timer `+0x4b0` is
/// run down.
fn animation_block_flags(e: &mut Engine, f: &UpdateFrame) {
    let this = f.this;
    let player = f.player;
    let animation = f.animation;
    let face = f.face;
    if !(e.vcall(this.addr(), VSLOT_FLAG_0X2E8, &args![]).bool() && animation != 0) {
        if e.vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
            .bool()
        {
            let process = process_of(e, this);
            let value = e.vcall(process, PROCESS_VSLOT_0X4B0, &args![]).f64();
            if value == e.global::<f64>(PROCESS_TIMER_UNSET) {
                let process = process_of(e, this);
                e.vcall(process, PROCESS_VSLOT_0X4B4, &args![0.0f32]);
            }
            let process = process_of(e, this);
            let value = e.vcall(process, PROCESS_VSLOT_0X4B0, &args![]).f64();
            let step = e
                .call(UPDATE_STEP_SETTING, &args![UPDATE_STEP_OBJECT])
                .f64();
            let remaining = (value - step) as f32;
            let process = process_of(e, this);
            e.vcall(process, PROCESS_VSLOT_0X4B4, &args![remaining]);
        }
        return;
    }
    let mut waking = true;
    let group = e.call(ANIMATION_GROUP_FIELD, &args![animation, 1u32]).u16();
    let is_dying = e.call(ANIM_GROUP_GET_TYPE, &args![group as u32]).i32() == 0xe0;
    if waking && !is_dying {
        let process = e.call(GET_PROCESS, &args![this]).u32();
        let current = e.call(PROCESS_FLOAT_0X7DF1F0, &args![process]).f32();
        let hour = e.call(CALENDAR_GET_HOUR, &args![CALENDAR]).f32();
        let difference = if current as f64 > hour as f64 {
            (current as f64 + e.global::<f64>(HOURS_PER_DAY) - hour as f64) as f32
        } else {
            (hour as f64 - current as f64) as f32
        };
        let scale = e.call(CALENDAR_GET_TIME_SCALE, &args![CALENDAR]).f64();
        waking = (difference as f64) < scale * e.global::<f64>(WAKE_TIME_FACTOR);
    }
    let owner = e.call(WORD_AT_0XC, &args![animation]).u32();
    let object = if owner != 0 {
        e.vcall(owner, 0xc, &args![]).u32()
    } else {
        0
    };
    if object != 0 && !is_dying {
        let blend = e.call(GET_BIP_BLEND_VALUE, &args![object]).f64();
        if blend <= 0.0 {
            if this.addr() == player {
                let first = e.call(PLAYER_GET_ANIMATION, &args![player, 0u32]).u32();
                free_animation(e, first);
                let second = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
                free_animation(e, second);
            } else {
                free_animation(e, animation);
            }
            let process = process_of(e, this);
            e.vcall(process, PROCESS_VSLOT_0X294, &args![this]);
        }
    }
    if face != 0 && !e.vcall(face, FACE_VSLOT_0XD4, &args![]).bool() {
        let process = process_of(e, this);
        let start = if process != 0 {
            let made = e.with_stack(0x10, |e, scratch| {
                let made = e
                    .vcall(process, PROCESS_VSLOT_0X48C, &args![scratch, 0u32])
                    .u32();
                let valid = e.call(SOUND_HANDLE_IS_VALID, &args![made]).bool();
                e.call(EMPTY_FUNCTION, &args![scratch]);
                valid
            });
            !made
        } else {
            true
        };
        if start {
            e.vcall(face, FACE_VSLOT_0XD8, &args![1u32, 0u32]);
        }
    }
    if is_dying {
        return;
    }
    if waking && face != 0 && e.vcall(face, FACE_VSLOT_0XC8, &args![]).bool() {
        return;
    }
    let mut interpolate = true;
    if e.call(ANIMATION_GROUP_LOADED, &args![animation, 0xe0u32])
        .bool()
    {
        let key = e.mem.u32(ANIMATION_GROUP_KEY);
        if e.call(ANIMATION_LOOKUP, &args![animation, key]).u32() != 0 {
            interpolate = false;
        } else {
            let process = e.call(GET_PROCESS, &args![this]).u32();
            if e.vcall(process, PROCESS_VSLOT_0X618, &args![]).u32() & 0x20 != 0 {
                interpolate = false;
            }
        }
    }
    e.vcall(this.addr(), VSLOT_0XD4, &args![1u32]);
    if interpolate {
        e.call(CLEAR_INTERPOLATORS, &args![this]);
    }
}

/// `Animation::SpecialIdleFree(1, 0)`, `Animation::ClearGroup(0x14, 0.0)`
/// and `004964d0` on one animation (the sequence `Actor::Update` repeats).
fn free_animation(e: &mut Engine, animation: u32) {
    e.call(ANIMATION_SPECIAL_IDLE_FREE, &args![animation, 1u32, 0u32]);
    e.call(ANIMATION_CLEAR_GROUP, &args![animation, 0x14u32, 0.0f32]);
    e.call(ANIMATION_FUNCTION_0X4964D0, &args![animation]);
}

/// The `float` at element `index` of the vector-like object `base`
/// (`00560d30` gives the element's address).
fn indexed_float(e: &mut Engine, base: u32, index: u32) -> f32 {
    let address = e.call(INDEXED_ELEMENT_ADDRESS, &args![base, index]).u32();
    e.mem.f32(address)
}
// Translated from 0088b020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte into the global at `011f3e60` (`cdecl`).
pub fn fn_0088b020(e: &mut Engine, value: u8) {
    e.mem.set_u8(GLOBAL_FLAG_011F3E60, value);
}

// Translated from 0088b050 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte at +0x15d of `this` (`Actor::Update` calls it on the
/// `ProcessLists` object `011e0e80`).
pub fn fn_0088b050(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x15d, value);
}

// Translated from 0088b070 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the byte `*flag` to `+0x69` of the character controller and
/// clears the word at `+0x6c` when it is zero.
pub fn fn_0088b070(e: &mut Engine, this: Ptr, flag: Ptr) {
    let value = e.mem.u8(flag.addr());
    e.mem.set_u8(this.addr() + 0x69, value);
    if value == 0 {
        e.mem.set_u32(this.addr() + 0x6c, 0);
    }
}

// Translated from 0088b110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the `float` at `+0x560`.
pub fn fn_0088b110(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0x560, value);
}

// Translated from 0088b130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the address `this + 0x570`.
pub fn fn_0088b130(_e: &mut Engine, this: Ptr) -> u32 {
    this.addr() + 0x570
}

// Translated from 0088b150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::UpdateActor3DPosition` (Xbox PDB): places the actor's 3D object
/// at its position (unless `00440460` is skipped by the global at
/// `011e0550`) and builds its rotation from the heading (`GetHeading` for
/// the player, virtual `+0x2bc` otherwise) and the character controller's
/// two extra rotations, then, for a non-player actor outside the
/// player's sleep or rest, updates the lighting of the 3D object through
/// `ShadowSceneNode::UpdateObjectLighting` and tracks the result in
/// `bLightingUpdatedNonMoving`.
pub fn actor_update_actor_3d_position(e: &mut Engine, this: Ptr<Actor>) {
    let object = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
    if object == 0 {
        return;
    }
    let controller = e.call(GET_CHAR_CONTROLLER, &args![this]).u32();
    let player = e.mem.u32(PLAYER_CHARACTER);
    let position = e.call(POSITION_VECTOR_OF, &args![this]).u32();
    let vector = [
        e.mem.u32(position),
        e.mem.u32(position + 4),
        e.mem.u32(position + 8),
    ];
    let heading = if this.addr() == player {
        e.call(GET_HEADING, &args![this, 1u32]).f32()
    } else {
        e.vcall(this.addr(), VSLOT_GET_HEADING_0X2BC, &args![1u32])
            .f32()
    };
    // Eight 9-word matrices live on the engine's stack: the rotation, the
    // second rotation, three product results and the race-adjusted copy.
    e.with_stack(0xf0, |e, base| {
        let rotation = base.addr();
        let second = base.addr() + 0x24;
        let product = base.addr() + 0x48;
        let adjusted = base.addr() + 0x6c;
        let position_copy = base.addr() + 0x90;
        for (i, word) in vector.iter().enumerate() {
            e.mem.set_u32(position_copy + 4 * i as u32, *word);
        }
        if e.mem.u8(SKIP_SET_POSITION) == 0 {
            e.call(SET_WORLD_POSITION, &args![object, position_copy]);
        }
        e.call(MATRIX_CONSTRUCT, &args![rotation]);
        e.call(MATRIX_MAKE_ROTATION, &args![rotation, heading]);
        if controller != 0 {
            e.call(MATRIX_CONSTRUCT, &args![second]);
            let pitch = e.call(CONTROLLER_PITCH, &args![controller]).f32();
            if pitch != 0.0 {
                let pitch = e.call(CONTROLLER_PITCH, &args![controller]).f32();
                e.call(MATRIX_MAKE_X_ROTATION, &args![second, pitch]);
                let result = e
                    .call(MATRIX_MULTIPLY, &args![rotation, product, second])
                    .u32();
                copy_words(e, result, rotation, 9);
            }
            let roll = e.call(CONTROLLER_ROLL, &args![controller]).f32();
            if roll != 0.0 {
                let roll = e.call(CONTROLLER_ROLL, &args![controller]).f32();
                e.call(MATRIX_MAKE_Y_ROTATION, &args![second, roll]);
                let result = e
                    .call(MATRIX_MULTIPLY, &args![rotation, product, second])
                    .u32();
                copy_words(e, result, rotation, 9);
            }
        }
        let result = e
            .call(MULTIPLY_MATRIX_BY_RACE, &args![this, adjusted, rotation])
            .u32();
        e.call(SET_WORLD_ROTATION, &args![object, result]);
    });
    if this.addr() == player || e.call(PLAYER_SLEEPING_OR_RESTING, &args![player]).bool() {
        return;
    }
    if e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32() == 0 {
        return;
    }
    let lists = e.call(GET_EXTRA_DATA_LIST_ADDRESS, &args![this]).u32();
    let dismembered = e.call(EXTRA_GET_DISMEMBERMENT, &args![lists]).u32() != 0;
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    e.call(CELL_UPDATE_REFERENCE, &args![cell, this, 1u32, dismembered]);
    if e.call(ACTOR_LIGHTING_FORCE_TEST, &args![this]).bool() {
        update_object_lighting(e, this, true);
        e.set(this, Actor::bLightingUpdatedNonMoving, false);
        return;
    }
    let mut updated_moving = false;
    if e.vcall(this.addr(), VSLOT_IS_PROCESS_FLAG_0X22C, &args![0u32])
        .bool()
        || e.vcall(this.addr(), VSLOT_FLAG_0X2E8, &args![]).bool()
    {
        let object = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
        let collision = e.call(FIND_NEXT_COLLISION_OBJECT, &args![object]).u32();
        let rigid_body = if collision != 0 {
            e.call(COLLISION_RIGID_BODY, &args![collision]).u32()
        } else {
            0
        };
        if rigid_body != 0 && e.call(RIGID_BODY_IS_ACTIVE, &args![rigid_body]).bool() {
            update_object_lighting(e, this, true);
            e.set(this, Actor::bLightingUpdatedNonMoving, false);
            updated_moving = true;
        }
    }
    if !e.get(this, Actor::bLightingUpdatedNonMoving) && !updated_moving {
        update_object_lighting(e, this, false);
        e.set(this, Actor::bLightingUpdatedNonMoving, true);
    }
}

/// `ShadowSceneNode::UpdateObjectLighting(object, flag)` (Xbox PDB) on the
/// scene node `00450b80(0)` returns, for the actor's 3D object.
fn update_object_lighting(e: &mut Engine, this: Ptr<Actor>, flag: bool) {
    let object = e.vcall(this.addr(), VSLOT_GET_3D, &args![]).u32();
    let node = e.call(SHADOW_SCENE_NODE, &args![0u32]).u32();
    e.call(UPDATE_OBJECT_LIGHTING, &args![node, object, flag]);
}

/// Copies `count` words (the game's `rep movsd`).
fn copy_words(e: &mut Engine, from: u32, to: u32, count: u32) {
    for i in 0..count {
        let word = e.mem.u32(from + 4 * i);
        e.mem.set_u32(to + 4 * i, word);
    }
}

/// True when the integer setting at `011c3ea4` (read through `0043d4d0`)
/// is 1.
fn mode_setting_is_one(e: &mut Engine) -> bool {
    let mode = e.call(GLOBAL_WORD_ADDRESS, &args![MODE_SETTING]).u32();
    e.mem.u32(mode) == 1
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00884990, fn_00884990(Ptr<Actor>)),
        entry!(0x008849c0, fn_008849c0(Ptr<Actor>) -> bool),
        entry!(0x00884aa0, fn_00884aa0(Ptr<Actor>) -> u32),
        entry!(0x00884ae0, fn_00884ae0(Ptr<Actor>) -> bool),
        entry!(0x00884b50, actor_is_allowed_to_swim(Ptr<Actor>) -> bool),
        entry!(0x00884c60, fn_00884c60(Ptr<Actor>)),
        entry!(0x00884c80, fn_00884c80(Ptr<CachedValuesOwner>) -> f32),
        entry!(0x00884ce0, fn_00884ce0(Ptr<BaseProcess>, f32)),
        entry!(0x00884d20, fn_00884d20(Ptr<CachedValuesOwner>) -> f32),
        entry!(0x00884d80, fn_00884d80(Ptr<BaseProcess>, f32)),
        entry!(0x00884dc0, actor_get_walk_speed(Ptr<Actor>) -> f32),
        entry!(
            0x00884e30,
            fn_00884e30(Ptr<BaseProcess>, Ptr<CachedValuesOwner>) -> f32
        ),
        entry!(0x00884e90, fn_00884e90(Ptr<CachedValues>, u32) -> bool),
        entry!(0x00884eb0, actor_get_run_speed(Ptr<Actor>) -> f32),
        entry!(
            0x00884f20,
            fn_00884f20(Ptr<BaseProcess>, Ptr<CachedValuesOwner>) -> f32
        ),
        entry!(0x00884f80, fn_00884f80(Ptr<Actor>)),
        entry!(0x00884fc0, actor_set_walk_speed(Ptr<Actor>, f32)),
        entry!(0x00884ff0, actor_set_run_speed(Ptr<Actor>, f32)),
        entry!(0x00885020, fn_00885020(Ptr<CachedValuesOwner>) -> f32),
        entry!(0x00885110, fn_00885110(Ptr<BaseProcess>, f32)),
        entry!(0x00885140, fn_00885140(Ptr<CachedValuesOwner>) -> f32),
        entry!(0x008851e0, fn_008851e0(Ptr<BaseProcess>, f32)),
        entry!(0x00885210, fn_00885210(Ptr<CachedValuesOwner>) -> f32),
        entry!(0x008852b0, fn_008852b0(Ptr<BaseProcess>, f32)),
        entry!(0x008852e0, fn_008852e0(Ptr<CachedValuesOwner>) -> f32),
        entry!(0x00885360, fn_00885360(Ptr<BaseProcess>, f32)),
        entry!(0x008853a0, actor_get_height(Ptr<Actor>) -> f32),
        entry!(0x00885490, fn_00885490(Ptr<HighProcess>) -> f32),
        entry!(0x008854b0, fn_008854b0(Ptr<HighProcess>, f32)),
        entry!(0x008854d0, actor_get_sneak_height(Ptr<Actor>, bool) -> f32),
        entry!(0x00885520, fn_00885520(Ptr<Actor>, Ptr, Ptr, f32) -> bool),
        entry!(0x00885560, fn_00885560(Ptr<Actor>, f32, Ptr) -> f32),
        entry!(
            0x008855f0,
            actor_adjust_speed_for_following(Ptr<Actor>, Ptr, f32, f32)
        ),
        entry!(
            0x008858c0,
            actor_modify_move_mode_for_follow(Ptr<Actor>, u32, f32, f32) -> u32
        ),
        entry!(0x008859e0, fn_008859e0(Ptr, Ptr) -> bool),
        entry!(0x00885a30, fn_00885a30(Ptr) -> bool),
        entry!(0x00885a50, actor_calculate_walk_speed(Ptr<Actor>) -> f32),
        entry!(0x00885bd0, fn_00885bd0(Ptr) -> f32),
        entry!(0x00885bf0, actor_calculate_run_speed(Ptr<Actor>) -> f32),
        entry!(0x00885d70, fn_00885d70(Ptr) -> f32),
        entry!(0x00885d90, fn_00885d90(Ptr<Actor>) -> f32),
        entry!(0x00885ed0, fn_00885ed0(Ptr<Actor>) -> f32),
        entry!(0x00886010, fn_00886010(Ptr<Actor>) -> f32),
        entry!(
            0x00886080,
            actor_get_current_package_location(Ptr<Actor>) -> u32
        ),
        entry!(0x008860d0, fn_008860d0(Ptr<Actor>) -> bool),
        entry!(0x00886360, fn_00886360(Ptr<Actor>, f32)),
        entry!(0x00886b10, fn_00886b10(Ptr<Actor>) -> bool),
        entry!(0x00886b30, fn_00886b30(Ptr<Actor>, Ptr) -> bool),
        entry!(0x00886c80, fn_00886c80(Ptr, bool)),
        entry!(0x00886cb0, fn_00886cb0(Ptr<Actor>, f32, u32)),
        entry!(0x008879d0, fn_008879d0(Ptr, bool)),
        entry!(0x00887a00, fn_00887a00() -> u32),
        entry!(0x00887a10, fn_00887a10(Ptr) -> u32),
        entry!(0x00887a80, fn_00887a80(Ptr, bool)),
        entry!(0x00887ac0, fn_00887ac0(Ptr, f32)),
        entry!(0x00887af0, fn_00887af0(Ptr, u32)),
        entry!(0x00887b20, fn_00887b20(Ptr, u32)),
        entry!(0x00887b40, fn_00887b40(Ptr, u8)),
        entry!(0x00887b70, fn_00887b70(Ptr, u8)),
        entry!(0x00887b90, fn_00887b90(Ptr<Actor>, f32, u32)),
        entry!(0x00887d00, actor_reset_loaded_animations(Ptr<Actor>)),
        entry!(0x00888070, fn_00888070(Ptr<Actor>, f32)),
        entry!(0x008885a0, fn_008885a0(Ptr<Actor>)),
        entry!(
            0x008885e0,
            actor_update_animation_movement(Ptr<Actor>, Ptr, f32)
        ),
        entry!(0x00888960, fn_00888960() -> u8),
        entry!(0x00888970, fn_00888970(Ptr<Actor>, u32)),
        entry!(0x00888a20, fn_00888a20(Ptr, f32, f32)),
        entry!(0x00888a50, fn_00888a50(Ptr) -> bool),
        entry!(0x00888a70, fn_00888a70(Ptr, Ptr, Ptr)),
        entry!(0x00888aa0, fn_00888aa0(Ptr)),
        entry!(0x00888ac0, fn_00888ac0(Ptr<Actor>, f32)),
        entry!(0x00888af0, fn_00888af0(Ptr<Actor>)),
        entry!(0x00888b20, fn_00888b20(Ptr<Actor>)),
        entry!(0x00888b50, actor_update(Ptr<Actor>, f32)),
        entry!(0x0088b020, fn_0088b020(u8)),
        entry!(0x0088b050, fn_0088b050(Ptr, u8)),
        entry!(0x0088b070, fn_0088b070(Ptr, Ptr)),
        entry!(0x0088b110, fn_0088b110(Ptr, f32)),
        entry!(0x0088b130, fn_0088b130(Ptr) -> u32),
        entry!(0x0088b150, actor_update_actor_3d_position(Ptr<Actor>)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTOR_TABLE: u32 = 0x0200_0000;
    const OWNER_TABLE: u32 = 0x0200_2000;
    const PROCESS_TABLE: u32 = 0x0200_4000;

    fn eax(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn st0(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    /// An engine with room for the three vtables the tests build and the
    /// exe data the code reads.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for table in [ACTOR_TABLE, OWNER_TABLE, PROCESS_TABLE] {
            e.map(table, 0x600);
        }
        for page in [
            0x0101_1000,
            0x0101_2000,
            0x0101_e000,
            0x0102_1000,
            0x0108_4000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(DOUBLE_ZERO, 0.0f64);
        e.set_global(DOUBLE_ONE, 1.0f64);
        e.set_global(DOUBLE_HALF, 0.5f64);
        e.set_global(MINIMUM_EXTENT, 32.0f32);
        e.set_global(FOLLOW_CATCH_UP_FACTOR, 1.25f64);
        e.set_global(SNEAK_HEIGHT_FACTOR, 0.5f64);
        e
    }

    /// A function standing in for `addr` that answers `ret` to any call.
    fn double(e: &mut Engine, addr: u32, ret: Ret) {
        e.register_double(addr, move |_, _| ret);
    }

    fn slot_target(table: u32, offset: u32) -> u32 {
        0x0300_0000 + (table - 0x0200_0000) + offset
    }

    /// Puts a function answering `ret` into a vtable slot.
    fn slot(e: &mut Engine, table: u32, offset: u32, ret: Ret) {
        let target = slot_target(table, offset);
        e.mem.set_u32(table + offset, target);
        double(e, target, ret);
    }

    fn new_actor(e: &mut Engine) -> Ptr<Actor> {
        let actor = e.new_object::<Actor>();
        e.mem.set_u32(actor.addr(), ACTOR_TABLE);
        e.mem
            .set_u32(actor.addr() + CACHED_VALUES_OWNER_OFFSET, OWNER_TABLE);
        actor
    }

    fn owner_of(actor: Ptr<Actor>) -> Ptr<CachedValuesOwner> {
        Ptr::new(actor.addr() + CACHED_VALUES_OWNER_OFFSET)
    }

    /// Gives the actor a process (a `HighProcess`-sized block with a vtable).
    fn with_process(e: &mut Engine, actor: Ptr<Actor>) -> Ptr<BaseProcess> {
        let process = Ptr::<BaseProcess>::new(e.mem.alloc(HighProcess::SIZE));
        e.mem.set_u32(process.addr(), PROCESS_TABLE);
        e.set(actor, Actor::pCurrentProcess, process.cast());
        process
    }

    fn with_cached_values(e: &mut Engine, process: Ptr<BaseProcess>) -> Ptr<CachedValues> {
        let cached = e.new_object::<CachedValues>();
        e.set(process, BaseProcess::pCachedValues, cached);
        cached
    }

    /// Doubles for the process-level cached-values helpers.
    fn cached_value_helpers(e: &mut Engine) {
        e.register(HAS_CACHED_VALUES, |e, a| {
            eax((e.mem.u32(a[0] + 0x2c) != 0) as u32)
        });
        e.register(CACHED_VALUES_ADD_FLAGS, |e, a| {
            let flags = e.mem.u32(a[0] + 0x44);
            e.mem.set_u32(a[0] + 0x44, flags | a[1]);
            Ret::default()
        });
    }

    fn logged(e: &Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.clone().unwrap()
    }

    fn logged_addresses(e: &Engine) -> Vec<u32> {
        logged(e).iter().map(|entry| entry.0).collect()
    }

    /// The bounds virtuals write (6, 10, 8) and (2, 4, 3) and return their
    /// argument; the scale is `scale`.
    fn bounds_and_scale(e: &mut Engine, scale: f32) {
        for (offset, values) in [
            (VSLOT_BOUNDS_UPPER, [6.0f32, 10.0, 8.0]),
            (VSLOT_BOUNDS_LOWER, [2.0f32, 4.0, 3.0]),
        ] {
            let target = slot_target(ACTOR_TABLE, offset);
            e.mem.set_u32(ACTOR_TABLE + offset, target);
            e.register_double(target, move |e, a| {
                for (i, value) in values.iter().enumerate() {
                    e.mem.set_f32(a[1] + 4 * i as u32, *value);
                }
                eax(a[1])
            });
        }
        double(e, GET_SCALE, st0(scale));
        e.register(FLOAT_HELPER_MAX, |_, a| {
            st0(f32::from_bits(a[0]).max(f32::from_bits(a[1])))
        });
    }

    #[test]
    fn swimming_byte_follows_flag_0x800() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        double(&mut e, GET_FLAG_WORD, eax(0x800));
        fn_00884990(&mut e, actor);
        assert!(e.get(actor, Actor::bSwimming));
        double(&mut e, GET_FLAG_WORD, eax(0x7ff));
        fn_00884990(&mut e, actor);
        assert!(!e.get(actor, Actor::bSwimming));
    }

    fn animation_actor(e: &mut Engine, found: u32, passes: u32, kind: u32) -> Ptr<Actor> {
        let actor = new_actor(e);
        slot(e, ACTOR_TABLE, VSLOT_GET_ANIMATION, eax(0x5000));
        double(e, ANIMATION_LOOKUP, eax(found));
        double(e, ANIMATION_STEP, eax(0x7000));
        double(e, ANIMATION_TEST, eax(passes));
        double(e, ANIMATION_KIND, eax(kind));
        double(e, GET_CHAR_CONTROLLER, eax(0));
        actor
    }

    #[test]
    fn animation_kinds_in_the_switch_answer_true() {
        for kind in [0xe3, 0xe5, 0xf1, 0xf2, 0xf3, 0xf4] {
            let mut e = engine();
            let actor = animation_actor(&mut e, 0x6000, 0, kind);
            assert!(fn_008849c0(&mut e, actor), "kind {kind:#x}");
        }
        for kind in [0xe4, 0xe6, 0xf0, 0xf5, 0x100, 0x0] {
            let mut e = engine();
            let actor = animation_actor(&mut e, 0x6000, 0, kind);
            assert!(!fn_008849c0(&mut e, actor), "kind {kind:#x}");
        }
    }

    #[test]
    fn animation_test_or_controller_state_answers_true() {
        let mut e = engine();
        let actor = animation_actor(&mut e, 0x6000, 1, 0);
        assert!(fn_008849c0(&mut e, actor));

        // No animation entry: only the controller counts.
        let mut e = engine();
        let actor = animation_actor(&mut e, 0, 1, 0xe3);
        double(&mut e, GET_CHAR_CONTROLLER, eax(0x8000));
        double(&mut e, CONTROLLER_STATE, eax(2));
        assert!(fn_008849c0(&mut e, actor));
        double(&mut e, CONTROLLER_STATE, eax(1));
        assert!(!fn_008849c0(&mut e, actor));
    }

    #[test]
    fn jump_sound_plays_only_when_the_test_answers() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        double(&mut e, JUMP_TEST, eax(5));
        double(&mut e, PLAY_JUMP, Ret::default());
        e.call_log = Some(vec![]);
        assert_eq!(fn_00884aa0(&mut e, actor), 5);
        assert_eq!(logged(&e)[1], (PLAY_JUMP, vec![actor.addr(), 0]));

        double(&mut e, JUMP_TEST, eax(0));
        e.call_log = Some(vec![]);
        assert_eq!(fn_00884aa0(&mut e, actor), 0);
        assert_eq!(logged_addresses(&e), vec![JUMP_TEST]);
    }

    #[test]
    fn object_type_0x12_or_0x15_counts() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        e.register(OBJECT_TYPE, |_, a| eax(a[0]));
        assert!(!fn_00884ae0(&mut e, actor), "no process");
        with_process(&mut e, actor);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_OBJECT, eax(0));
        assert!(!fn_00884ae0(&mut e, actor), "no object");
        for (object, expected) in [(0x12, true), (0x15, true), (0x13, false)] {
            slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_OBJECT, eax(object));
            assert_eq!(fn_00884ae0(&mut e, actor), expected, "type {object:#x}");
        }
    }

    #[test]
    fn swimming_is_allowed_when_the_flag_byte_is_set() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        e.set(actor, Actor::bInWater, true);
        e.call_log = Some(vec![]);
        assert!(actor_is_allowed_to_swim(&mut e, actor));
        assert!(logged(&e).is_empty());
    }

    #[test]
    fn swimming_with_a_combat_controller_needs_each_test() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        with_process(&mut e, actor);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_OBJECT, eax(0x12));
        e.register(OBJECT_TYPE, |_, a| eax(a[0]));
        slot(
            &mut e,
            ACTOR_TABLE,
            VSLOT_GET_COMBAT_CONTROLLER,
            eax(0x9000),
        );
        double(&mut e, SWIM_TEST_FINAL, eax(1));

        // Neither accompanied nor fleeing: forbidden.
        double(&mut e, SWIM_TEST_COMBAT, eax(0));
        double(&mut e, COMBAT_CONTROLLER_IS_FLEEING, eax(0));
        double(&mut e, COMBAT_CONTROLLER_TEST, eax(1));
        assert!(!actor_is_allowed_to_swim(&mut e, actor));
        // Fleeing passes the first test; the second one decides.
        double(&mut e, COMBAT_CONTROLLER_IS_FLEEING, eax(1));
        assert!(actor_is_allowed_to_swim(&mut e, actor));
        double(&mut e, COMBAT_CONTROLLER_TEST, eax(0));
        assert!(!actor_is_allowed_to_swim(&mut e, actor));
        // Without a controller only the final test counts.
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_COMBAT_CONTROLLER, eax(0));
        assert!(actor_is_allowed_to_swim(&mut e, actor));
        double(&mut e, SWIM_TEST_FINAL, eax(0));
        assert!(!actor_is_allowed_to_swim(&mut e, actor));
    }

    #[test]
    fn swimming_without_the_object_type_depends_on_creatures() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        with_process(&mut e, actor);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_OBJECT, eax(0x13));
        e.register(OBJECT_TYPE, |_, a| eax(a[0]));
        double(&mut e, SWIM_TEST_FINAL, eax(1));
        double(&mut e, SWIM_OBJECT_TEST, eax(1));
        // Not a creature: the final test decides.
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_CREATURE, eax(0));
        assert!(actor_is_allowed_to_swim(&mut e, actor));
        // A creature: the object test decides.
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_CREATURE, eax(1));
        assert!(actor_is_allowed_to_swim(&mut e, actor));
        double(&mut e, SWIM_OBJECT_TEST, eax(0));
        assert!(!actor_is_allowed_to_swim(&mut e, actor));
    }

    #[test]
    fn stop_moving_is_forwarded() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        double(&mut e, STOP_MOVING, Ret::default());
        e.call_log = Some(vec![]);
        fn_00884c60(&mut e, actor);
        assert_eq!(logged(&e), vec![(STOP_MOVING, vec![actor.addr()])]);
    }

    #[test]
    fn cached_walk_speed_is_stored_when_the_process_has_values() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        let process = with_process(&mut e, actor);
        let cached = with_cached_values(&mut e, process);
        slot(&mut e, ACTOR_TABLE, VSLOT_CALCULATE_WALK_SPEED, st0(3.5));
        assert_eq!(fn_00884c80(&mut e, owner_of(actor)), 3.5);
        assert_eq!(e.get(cached, CachedValues::fCachedWalkSpeed), 3.5);
        assert_eq!(e.get(cached, CachedValues::iFlags), CACHED_WALK_SPEED_FLAG);

        // Without a cached-values block the speed is only returned.
        let plain = new_actor(&mut e);
        with_process(&mut e, plain);
        assert_eq!(fn_00884c80(&mut e, owner_of(plain)), 3.5);
    }

    #[test]
    fn walk_speed_setter_marks_the_flag() {
        let mut e = engine();
        double(&mut e, CACHED_VALUES_ADD_FLAGS, Ret::default());
        let process = e.new_object::<BaseProcess>();
        fn_00884ce0(&mut e, process, 2.0);
        let cached = e.new_object::<CachedValues>();
        e.set(process, BaseProcess::pCachedValues, cached);
        e.call_log = Some(vec![]);
        fn_00884ce0(&mut e, process, 2.0);
        assert_eq!(e.get(cached, CachedValues::fCachedWalkSpeed), 2.0);
        assert_eq!(
            logged(&e),
            vec![(CACHED_VALUES_ADD_FLAGS, vec![cached.addr(), 0x1000])]
        );
    }

    #[test]
    fn cached_run_speed_is_stored_when_the_process_has_values() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        let process = with_process(&mut e, actor);
        let cached = with_cached_values(&mut e, process);
        slot(&mut e, ACTOR_TABLE, VSLOT_CALCULATE_RUN_SPEED, st0(7.25));
        assert_eq!(fn_00884d20(&mut e, owner_of(actor)), 7.25);
        assert_eq!(e.get(cached, CachedValues::fCachedRunSpeed), 7.25);
        assert_eq!(e.get(cached, CachedValues::iFlags), CACHED_RUN_SPEED_FLAG);
    }

    #[test]
    fn run_speed_setter_marks_the_flag() {
        let mut e = engine();
        double(&mut e, CACHED_VALUES_ADD_FLAGS, Ret::default());
        let process = e.new_object::<BaseProcess>();
        let cached = e.new_object::<CachedValues>();
        e.set(process, BaseProcess::pCachedValues, cached);
        e.call_log = Some(vec![]);
        fn_00884d80(&mut e, process, 9.0);
        assert_eq!(e.get(cached, CachedValues::fCachedRunSpeed), 9.0);
        assert_eq!(
            logged(&e),
            vec![(CACHED_VALUES_ADD_FLAGS, vec![cached.addr(), 0x2000])]
        );
    }

    #[test]
    fn get_walk_speed_uses_the_cache_or_the_virtual() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        slot(&mut e, ACTOR_TABLE, VSLOT_CALCULATE_WALK_SPEED, st0(1.5));
        // No process: the virtual.
        assert_eq!(actor_get_walk_speed(&mut e, actor), 1.5);
        // A process without cached values: the virtual.
        let process = with_process(&mut e, actor);
        assert_eq!(actor_get_walk_speed(&mut e, actor), 1.5);
        // With cached values and the flag set: the cached value.
        let cached = with_cached_values(&mut e, process);
        e.set(cached, CachedValues::fCachedWalkSpeed, 6.0);
        e.set(cached, CachedValues::iFlags, CACHED_WALK_SPEED_FLAG);
        assert_eq!(actor_get_walk_speed(&mut e, actor), 6.0);
        // Flag clear: the owner's virtual +0x38.
        e.set(cached, CachedValues::iFlags, 0);
        slot(&mut e, OWNER_TABLE, OWNER_VSLOT_WALK_SPEED, st0(2.5));
        assert_eq!(actor_get_walk_speed(&mut e, actor), 2.5);
    }

    #[test]
    fn process_walk_speed_is_zero_without_cached_values() {
        let mut e = engine();
        let process = e.new_object::<BaseProcess>();
        assert_eq!(fn_00884e30(&mut e, process, Ptr::NULL), 0.0);
    }

    #[test]
    fn flag_test_is_true_when_no_mask_bit_is_set() {
        let mut e = engine();
        let cached = e.new_object::<CachedValues>();
        e.set(cached, CachedValues::iFlags, 0x1000);
        assert!(!fn_00884e90(&mut e, cached, 0x1000));
        assert!(fn_00884e90(&mut e, cached, 0x2000));
        assert!(!fn_00884e90(&mut e, cached, 0x3000));
    }

    #[test]
    fn get_run_speed_uses_the_cache_or_the_virtual() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        slot(&mut e, ACTOR_TABLE, VSLOT_CALCULATE_RUN_SPEED, st0(4.5));
        assert_eq!(actor_get_run_speed(&mut e, actor), 4.5);
        let process = with_process(&mut e, actor);
        assert_eq!(actor_get_run_speed(&mut e, actor), 4.5);
        let cached = with_cached_values(&mut e, process);
        e.set(cached, CachedValues::fCachedRunSpeed, 8.0);
        e.set(cached, CachedValues::iFlags, CACHED_RUN_SPEED_FLAG);
        assert_eq!(actor_get_run_speed(&mut e, actor), 8.0);
        e.set(cached, CachedValues::iFlags, 0);
        slot(&mut e, OWNER_TABLE, OWNER_VSLOT_RUN_SPEED, st0(5.5));
        assert_eq!(actor_get_run_speed(&mut e, actor), 5.5);
    }

    #[test]
    fn process_run_speed_is_zero_without_cached_values() {
        let mut e = engine();
        let process = e.new_object::<BaseProcess>();
        assert_eq!(fn_00884f20(&mut e, process, Ptr::NULL), 0.0);
    }

    #[test]
    fn speed_masks_are_forwarded_run_first() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        double(&mut e, PROCESS_FORWARD_FLAG_MASK, Ret::default());
        e.call_log = Some(vec![]);
        fn_00884f80(&mut e, actor);
        assert!(logged(&e).is_empty(), "no process, no calls");
        let process = with_process(&mut e, actor);
        fn_00884f80(&mut e, actor);
        assert_eq!(
            logged(&e),
            vec![
                (PROCESS_FORWARD_FLAG_MASK, vec![process.addr(), 0x2000]),
                (PROCESS_FORWARD_FLAG_MASK, vec![process.addr(), 0x1000]),
            ]
        );
    }

    #[test]
    fn speed_setters_need_a_process() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        actor_set_walk_speed(&mut e, actor, 1.0);
        actor_set_run_speed(&mut e, actor, 1.0);
        let process = with_process(&mut e, actor);
        let cached = with_cached_values(&mut e, process);
        actor_set_walk_speed(&mut e, actor, 2.0);
        actor_set_run_speed(&mut e, actor, 3.0);
        assert_eq!(e.get(cached, CachedValues::fCachedWalkSpeed), 2.0);
        assert_eq!(e.get(cached, CachedValues::fCachedRunSpeed), 3.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 0x3000);
    }

    #[test]
    fn cached_radius_is_half_the_larger_extent_but_at_least_16() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        let process = with_process(&mut e, actor);
        let cached = with_cached_values(&mut e, process);
        // Extents 4 and 6 times scale 1: below the minimum of 32.
        bounds_and_scale(&mut e, 1.0);
        assert_eq!(fn_00885020(&mut e, owner_of(actor)), 16.0);
        assert_eq!(e.get(cached, CachedValues::fCachedRadius), 16.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 1);
        // Times scale 8: 48, half of it.
        bounds_and_scale(&mut e, 8.0);
        assert_eq!(fn_00885020(&mut e, owner_of(actor)), 24.0);
    }

    #[test]
    fn radius_setter_marks_the_flag() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let process = e.new_object::<BaseProcess>();
        fn_00885110(&mut e, process, 1.0);
        let cached = e.new_object::<CachedValues>();
        e.set(process, BaseProcess::pCachedValues, cached);
        fn_00885110(&mut e, process, 12.0);
        assert_eq!(e.get(cached, CachedValues::fCachedRadius), 12.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 1);
    }

    #[test]
    fn cached_width_is_the_scaled_x_extent() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        let process = with_process(&mut e, actor);
        let cached = with_cached_values(&mut e, process);
        bounds_and_scale(&mut e, 2.0);
        assert_eq!(fn_00885140(&mut e, owner_of(actor)), 8.0);
        assert_eq!(e.get(cached, CachedValues::fCachedWidth), 8.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 2);
    }

    #[test]
    fn width_setter_marks_the_flag() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let process = e.new_object::<BaseProcess>();
        fn_008851e0(&mut e, process, 1.0);
        let cached = e.new_object::<CachedValues>();
        e.set(process, BaseProcess::pCachedValues, cached);
        fn_008851e0(&mut e, process, 3.0);
        assert_eq!(e.get(cached, CachedValues::fCachedWidth), 3.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 2);
    }

    #[test]
    fn cached_length_is_the_scaled_y_extent() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        let process = with_process(&mut e, actor);
        let cached = with_cached_values(&mut e, process);
        bounds_and_scale(&mut e, 2.0);
        assert_eq!(fn_00885210(&mut e, owner_of(actor)), 12.0);
        assert_eq!(e.get(cached, CachedValues::fCachedLength), 12.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 4);
    }

    #[test]
    fn length_setter_marks_the_flag() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let process = e.new_object::<BaseProcess>();
        fn_008852b0(&mut e, process, 1.0);
        let cached = e.new_object::<CachedValues>();
        e.set(process, BaseProcess::pCachedValues, cached);
        fn_008852b0(&mut e, process, 5.0);
        assert_eq!(e.get(cached, CachedValues::fCachedLength), 5.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 4);
    }

    #[test]
    fn cached_forward_length_uses_only_the_upper_corner() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let actor = new_actor(&mut e);
        let process = with_process(&mut e, actor);
        let cached = with_cached_values(&mut e, process);
        bounds_and_scale(&mut e, 2.0);
        e.call_log = Some(vec![]);
        assert_eq!(fn_008852e0(&mut e, owner_of(actor)), 20.0);
        assert!(!logged_addresses(&e).contains(&slot_target(ACTOR_TABLE, VSLOT_BOUNDS_LOWER)));
        assert_eq!(e.get(cached, CachedValues::fCachedForwardLength), 20.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 0x8000);
    }

    #[test]
    fn forward_length_setter_marks_the_flag() {
        let mut e = engine();
        cached_value_helpers(&mut e);
        let process = e.new_object::<BaseProcess>();
        fn_00885360(&mut e, process, 1.0);
        let cached = e.new_object::<CachedValues>();
        e.set(process, BaseProcess::pCachedValues, cached);
        fn_00885360(&mut e, process, 7.0);
        assert_eq!(e.get(cached, CachedValues::fCachedForwardLength), 7.0);
        assert_eq!(e.get(cached, CachedValues::iFlags), 0x8000);
    }

    #[test]
    fn height_is_computed_without_a_process_and_cached_with_one() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        bounds_and_scale(&mut e, 2.0);
        // No process: (8 - 3) * 2 every time.
        assert_eq!(actor_get_height(&mut e, actor), 10.0);

        // A process that bypasses the cache: also computed.
        let process = with_process(&mut e, actor);
        double(&mut e, CACHED_HEIGHT_BYPASS, eax(1));
        assert_eq!(actor_get_height(&mut e, actor), 10.0);
        assert_eq!(e.mem.f32(process.addr() + 0x42c), 0.0);

        // Caching process, unset: computed and stored.
        double(&mut e, CACHED_HEIGHT_BYPASS, eax(0));
        assert_eq!(actor_get_height(&mut e, actor), 10.0);
        assert_eq!(e.mem.f32(process.addr() + 0x42c), 10.0);

        // Set: returned as stored, bounds not asked.
        e.mem.set_f32(process.addr() + 0x42c, 7.0);
        e.call_log = Some(vec![]);
        assert_eq!(actor_get_height(&mut e, actor), 7.0);
        assert_eq!(logged_addresses(&e), vec![CACHED_HEIGHT_BYPASS]);
    }

    #[test]
    fn cached_height_accessors_use_offset_0x42c() {
        let mut e = engine();
        let process = Ptr::<HighProcess>::new(e.mem.alloc(HighProcess::SIZE));
        fn_008854b0(&mut e, process, 1.75);
        assert_eq!(e.mem.f32(process.addr() + 0x42c), 1.75);
        assert_eq!(fn_00885490(&mut e, process), 1.75);
    }

    #[test]
    fn sneak_height_is_a_fraction_of_height_or_eye_level() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        bounds_and_scale(&mut e, 2.0);
        double(&mut e, GET_EYE_LEVEL, st0(4.0));
        assert_eq!(actor_get_sneak_height(&mut e, actor, false), 5.0);
        assert_eq!(actor_get_sneak_height(&mut e, actor, true), 2.0);
    }

    #[test]
    fn submersion_is_depth_over_height_capped_at_one() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        bounds_and_scale(&mut e, 2.0);
        double(&mut e, GET_RELEVANT_WATER_HEIGHT, st0(10.0));
        double(&mut e, GET_PARENT_CELL, eax(0x9000));
        assert_eq!(
            fn_00885560(&mut e, actor, 4.0, Ptr::NULL),
            (6.0f64 / 10.0) as f32
        );
        assert_eq!(fn_00885560(&mut e, actor, -5.0, Ptr::NULL), 1.0);
        assert_eq!(fn_00885560(&mut e, actor, 10.0, Ptr::NULL), 0.0);
        assert_eq!(fn_00885560(&mut e, actor, 12.0, Ptr::NULL), 0.0);
    }

    #[test]
    fn submersion_uses_the_cell_water_height_for_another_cell() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        bounds_and_scale(&mut e, 2.0);
        double(&mut e, GET_RELEVANT_WATER_HEIGHT, st0(10.0));
        double(&mut e, CELL_GET_WATER_HEIGHT, st0(5.0));
        double(&mut e, GET_PARENT_CELL, eax(0x9000));
        // Another cell: its water, 5 - 0 over 10.
        assert_eq!(fn_00885560(&mut e, actor, 0.0, Ptr::new(0x9100)), 0.5);
        // The actor's own cell: the relevant water height, 10 - 0 over 10.
        assert_eq!(fn_00885560(&mut e, actor, 0.0, Ptr::new(0x9000)), 1.0);
    }

    #[test]
    fn submersion_test_compares_the_threshold() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        bounds_and_scale(&mut e, 2.0);
        double(&mut e, GET_RELEVANT_WATER_HEIGHT, st0(10.0));
        let point = Ptr::new(e.mem.alloc(12));
        e.mem.set_f32(point.addr() + 8, 5.0);
        // Depth is 0.5.
        assert!(fn_00885520(&mut e, actor, point, Ptr::NULL, 0.25));
        assert!(fn_00885520(&mut e, actor, point, Ptr::NULL, 0.5));
        assert!(!fn_00885520(&mut e, actor, point, Ptr::NULL, 0.75));
    }

    const WORLD_SPACE: u32 = 0xa000;

    /// Two actors in the same world space whose current package targets
    /// the second, plus the doubles the follow code calls.
    fn follow_setup(e: &mut Engine) -> (Ptr<Actor>, Ptr<Actor>) {
        let actor = new_actor(e);
        let target = new_actor(e);
        slot(e, ACTOR_TABLE, VSLOT_IS_ACTOR, eax(1));
        double(e, GET_CURRENT_PACKAGE, eax(0xb000));
        double(e, GET_CURRENT_PACKAGE_TARGET, eax(target.addr()));
        double(e, GET_PARENT_CELL, eax(0x9000));
        double(e, GET_WORLD_SPACE, eax(WORLD_SPACE));
        double(e, CELL_TEST_INTERIOR, eax(0));
        double(e, FOLLOW_RADIUS_NEAR, st0(2.0));
        double(e, FOLLOW_RADIUS_MID, st0(6.0));
        double(e, FOLLOW_RADIUS_WALK, st0(3.0));
        double(e, FOLLOW_RADIUS_MATCH_SPEED, st0(10.0));
        double(e, GET_CURRENT_SPEED, st0(4.0));
        double(e, GET_SCALE, st0(1.0));
        double(e, GET_FLAG_WORD, eax(0x20f));
        e.register(FLOAT_HELPER_MAX, |_, a| {
            st0(f32::from_bits(a[0]).max(f32::from_bits(a[1])))
        });
        e.register(FLOAT_HELPER_MIN, |_, a| {
            st0(f32::from_bits(a[0]).min(f32::from_bits(a[1])))
        });
        e.register(SETTING_VALUE_ADDRESS, |e, a| {
            let value = match a[0] {
                SETTING_UPPER_FACTOR_ALL_FLAGS => 2.0f32,
                SETTING_UPPER_FACTOR_OTHERWISE => 3.0,
                SETTING_LOWER_FACTOR_ALL_FLAGS => 0.5,
                SETTING_LOWER_FACTOR_OTHERWISE => 0.25,
                SETTING_INTERPOLATION_FACTOR => 0.5,
                other => panic!("unexpected setting {other:08x}"),
            };
            let address = e.mem.alloc(4);
            e.mem.set_f32(address, value);
            eax(address)
        });
        e.register(CLAMP_FLOAT, |e, a| {
            let (lower, upper) = (f32::from_bits(a[1]), f32::from_bits(a[2]));
            let value = e.mem.f32(a[0]);
            if value > upper {
                e.mem.set_f32(a[0], upper);
            } else if value < lower {
                e.mem.set_f32(a[0], lower);
            }
            Ret::default()
        });
        (actor, target)
    }

    fn adjusted_speed(e: &mut Engine, actor: Ptr<Actor>, speed: f32, distance: f32) -> f32 {
        let cell = Ptr::new(e.mem.alloc(4));
        e.mem.set_f32(cell.addr(), speed);
        actor_adjust_speed_for_following(e, actor, cell, 1.0, distance);
        e.mem.f32(cell.addr())
    }

    #[test]
    fn following_inside_the_mid_radius_ramps_from_the_near_radius() {
        let mut e = engine();
        let (actor, _) = follow_setup(&mut e);
        // nearer = 5 - 2, span = 6 - 2, lesser speed = min(2, 4): 3/4 * 2,
        // inside the clamp [2 * 0.5, 2 * 2].
        assert_eq!(adjusted_speed(&mut e, actor, 2.0, 5.0), 1.5);
        // Clamped to the lower limit 2 * 0.5.
        assert_eq!(adjusted_speed(&mut e, actor, 2.0, 3.0), 1.0);
    }

    #[test]
    fn following_past_the_far_radius_catches_up_when_all_flags_are_set() {
        let mut e = engine();
        let (actor, _) = follow_setup(&mut e);
        // 4 * 1.25 inside [2, 8].
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 5.0);
        // The clamp's upper limit 2 * 2 wins over 5.
        assert_eq!(adjusted_speed(&mut e, actor, 2.0, 12.0), 4.0);
    }

    #[test]
    fn following_between_the_radii_interpolates() {
        let mut e = engine();
        let (actor, _) = follow_setup(&mut e);
        // (1 + (8 - 6) / (10 - 6) * 0.5) * 4.
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 8.0), 5.0);
        // Without all flags the far radius interpolates too:
        // (1 + (12 - 6) / 4 * 0.5) * 4 = 7, inside [1, 12].
        double(&mut e, GET_FLAG_WORD, eax(0x20e));
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 7.0);
    }

    #[test]
    fn following_scales_the_target_speed_by_the_scale_ratio() {
        let mut e = engine();
        let (actor, target) = follow_setup(&mut e);
        e.register_double(GET_SCALE, move |_, a| {
            st0(if a[0] == target.addr() { 3.0 } else { 1.5 })
        });
        // Target speed 4 * (3 / 1.5) = 8: (1 + 0.25) * 8 = 10, inside the
        // clamp [4, 16] for the old speed 8.
        assert_eq!(adjusted_speed(&mut e, actor, 8.0, 8.0), 10.0);
    }

    #[test]
    fn following_needs_an_actor_target_in_the_same_area() {
        let mut e = engine();
        let (actor, target) = follow_setup(&mut e);
        // No target.
        double(&mut e, GET_CURRENT_PACKAGE_TARGET, eax(0));
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 4.0);
        // The actor itself.
        double(&mut e, GET_CURRENT_PACKAGE_TARGET, eax(actor.addr()));
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 4.0);
        // Not an actor.
        double(&mut e, GET_CURRENT_PACKAGE_TARGET, eax(target.addr()));
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_ACTOR, eax(0));
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 4.0);
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_ACTOR, eax(1));
        // Another world space.
        e.register_double(GET_WORLD_SPACE, move |_, a| {
            eax(if a[0] == target.addr() {
                0xc000
            } else {
                WORLD_SPACE
            })
        });
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 4.0);
        // Interior cells decide by cell instead: equal cells match whatever
        // the world spaces say, different cells differ.
        double(&mut e, CELL_TEST_INTERIOR, eax(1));
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 5.0);
        e.register_double(GET_PARENT_CELL, move |_, a| {
            eax(if a[0] == target.addr() {
                0x9100
            } else {
                0x9000
            })
        });
        assert_eq!(adjusted_speed(&mut e, actor, 4.0, 12.0), 4.0);
    }

    #[test]
    fn follow_mode_is_decided_by_distance_and_flags() {
        let mut e = engine();
        let (actor, target) = follow_setup(&mut e);
        double(&mut e, FOLLOW_MODE_ALLOWED, eax(1));
        e.set_global(PLAYER_CHARACTER, 0u32);
        double(&mut e, GET_DISTANCE_FROM_REFERENCE, st0(5.0));
        double(&mut e, GET_FLAG_WORD, eax(0));
        // Walk radius 3 < distance 5 < match radius 10: unchanged.
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x300, 1.0, 0.0),
            0x300
        );
        // Closer than the walk radius with no low flag bits: 0x100 replaces
        // 0x200.
        double(&mut e, GET_DISTANCE_FROM_REFERENCE, st0(2.0));
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x2ff, 1.0, 0.0),
            0x1ff
        );
        // ... but not with a low flag bit set.
        double(&mut e, GET_FLAG_WORD, eax(1));
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x2ff, 1.0, 0.0),
            0x2ff
        );
        // Farther than the match radius: 0x200 replaces 0x100.
        double(&mut e, GET_DISTANCE_FROM_REFERENCE, st0(11.0));
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x1ff, 1.0, 0.0),
            0x2ff
        );
        // ... unless the follow code is not allowed to.
        double(&mut e, FOLLOW_MODE_ALLOWED, eax(0));
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x1ff, 1.0, 0.0),
            0x1ff
        );
        // The target's own flags.
        double(&mut e, FOLLOW_MODE_ALLOWED, eax(1));
        double(&mut e, GET_DISTANCE_FROM_REFERENCE, st0(5.0));
        for flags in [0x400u32, 0x200] {
            double(&mut e, GET_FLAG_WORD, eax(flags));
            assert_eq!(
                actor_modify_move_mode_for_follow(&mut e, actor, 0x100, 1.0, 0.0),
                0x200,
                "flags {flags:#x}"
            );
        }
        double(&mut e, GET_FLAG_WORD, eax(0x500));
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x100, 1.0, 0.0),
            0x100
        );
        // The player as the target with mode bit 0x800.
        double(&mut e, GET_FLAG_WORD, eax(0));
        e.set_global(PLAYER_CHARACTER, target.addr());
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x900, 1.0, 0.0),
            0xa00
        );
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x100, 1.0, 0.0),
            0x100
        );
    }

    #[test]
    fn follow_mode_is_unchanged_without_an_actor_target() {
        let mut e = engine();
        let (actor, _) = follow_setup(&mut e);
        double(&mut e, GET_CURRENT_PACKAGE_TARGET, eax(0));
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x2ff, 1.0, 0.0),
            0x2ff
        );
        let other = new_actor(&mut e);
        double(&mut e, GET_CURRENT_PACKAGE_TARGET, eax(other.addr()));
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_ACTOR, eax(0));
        assert_eq!(
            actor_modify_move_mode_for_follow(&mut e, actor, 0x2ff, 1.0, 0.0),
            0x2ff
        );
    }

    #[test]
    fn form_flag_is_checked_only_for_three_form_types() {
        let mut e = engine();
        e.register(SAVE_FORM_BUFFER_GET_FORM, |_, a| eax(a[0]));
        e.register(FORM_TYPE, |e, a| eax(e.mem.u8(a[0] + 4) as u32));
        let form = Ptr::new(e.mem.alloc(16));
        for (form_type, flags, expected) in [
            (0x15u8, 0x2000_0000u32, true),
            (0x15, 0, false),
            (0x27, 0, false),
            (0x30, 0x2000_0000, true),
            (0x30, 0, false),
            (0x14, 0, true),
            (0x00, 0, true),
        ] {
            e.mem.set_u8(form.addr() + 4, form_type);
            e.mem.set_u32(form.addr() + 8, flags);
            assert_eq!(
                fn_008859e0(&mut e, Ptr::NULL, form),
                expected,
                "type {form_type:#x} flags {flags:#x}"
            );
        }
    }

    #[test]
    fn form_flag_bit_20000000() {
        let mut e = engine();
        let form = Ptr::new(e.mem.alloc(16));
        assert!(!fn_00885a30(&mut e, form));
        e.mem.set_u32(form.addr() + 8, 0x2000_0000);
        assert!(fn_00885a30(&mut e, form));
        e.mem.set_u32(form.addr() + 8, 0xdfff_ffff);
        assert!(!fn_00885a30(&mut e, form));
    }

    /// Doubles for what both speed calculations call.
    fn speed_formula_setup(e: &mut Engine) -> Ptr<Actor> {
        let actor = new_actor(e);
        with_process(e, actor);
        slot(e, PROCESS_TABLE, PROCESS_VSLOT_FLAG, eax(0));
        slot(e, PROCESS_TABLE, PROCESS_VSLOT_ITEM, eax(0x111));
        slot(e, ACTOR_TABLE, VSLOT_FLAG_0X358, eax(1));
        slot(e, ACTOR_TABLE, VSLOT_IS_CREATURE, eax(0));
        double(e, SPEED_FLAG_TEST, eax(1));
        double(e, GET_ARMOR_BEING_WORN, eax(0x222));
        e.register(ITEM_VALUE, |_, a| eax(a[0] + 1));
        double(e, ITEM_RELEASE, Ret::default());
        double(e, CALCULATE_WALK_SPEED_FORMULA, st0(4.0));
        double(e, CALCULATE_RUN_SPEED_FORMULA, st0(4.0));
        slot(e, ACTOR_TABLE, VSLOT_GET_COMBAT_CONTROLLER, eax(0));
        actor
    }

    #[test]
    fn walk_speed_formula_gets_the_actor_values_in_order() {
        let mut e = engine();
        let actor = speed_formula_setup(&mut e);
        e.call_log = Some(vec![]);
        assert_eq!(actor_calculate_walk_speed(&mut e, actor), 4.0);
        let log = logged(&e);
        let formula = log
            .iter()
            .find(|entry| entry.0 == CALCULATE_WALK_SPEED_FORMULA)
            .unwrap();
        // value owner, item value, armor value, 004997b0, process flag
        // negated, creature, virtual +0x358.
        assert_eq!(
            formula.1,
            vec![actor.addr() + 0xa4, 0x112, 0x223, 1, 1, 0, 1]
        );
        assert!(log.contains(&(ITEM_RELEASE, vec![0x222, 1])));
        assert!(log.contains(&(GET_ARMOR_BEING_WORN, vec![actor.addr(), 2])));
    }

    #[test]
    fn walk_speed_is_scaled_by_the_combat_controller() {
        let mut e = engine();
        let actor = speed_formula_setup(&mut e);
        let controller = Ptr::new(e.mem.alloc(0x100));
        e.mem.set_f32(controller.addr() + 0xd8, 1.5);
        slot(
            &mut e,
            ACTOR_TABLE,
            VSLOT_GET_COMBAT_CONTROLLER,
            eax(controller.addr()),
        );
        assert_eq!(actor_calculate_walk_speed(&mut e, actor), 6.0);
        assert_eq!(fn_00885bd0(&mut e, controller), 1.5);
    }

    #[test]
    fn walk_speed_without_items_passes_zero_values() {
        let mut e = engine();
        let actor = speed_formula_setup(&mut e);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_ITEM, eax(0));
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_FLAG, eax(1));
        double(&mut e, GET_ARMOR_BEING_WORN, eax(0));
        e.call_log = Some(vec![]);
        actor_calculate_walk_speed(&mut e, actor);
        let log = logged(&e);
        assert!(!log.iter().any(|entry| entry.0 == ITEM_VALUE));
        assert!(!log.iter().any(|entry| entry.0 == ITEM_RELEASE));
        let formula = log
            .iter()
            .find(|entry| entry.0 == CALCULATE_WALK_SPEED_FORMULA)
            .unwrap();
        assert_eq!(formula.1, vec![actor.addr() + 0xa4, 0, 0, 1, 0, 0, 1]);
    }

    #[test]
    fn run_speed_goes_through_the_entry_point() {
        let mut e = engine();
        let actor = speed_formula_setup(&mut e);
        e.register(HANDLE_ENTRY_POINT, |e, a| {
            let value = e.mem.f32(a[2]);
            e.mem.set_f32(a[2], value + 1.0);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        assert_eq!(actor_calculate_run_speed(&mut e, actor), 5.0);
        let log = logged(&e);
        let formula = log
            .iter()
            .find(|entry| entry.0 == CALCULATE_RUN_SPEED_FORMULA)
            .unwrap();
        // value owner, item value, armor value, process flag negated,
        // creature, 004997b0, virtual +0x358.
        assert_eq!(
            formula.1,
            vec![actor.addr() + 0xa4, 0x112, 0x223, 1, 0, 1, 1]
        );
        let entry = log
            .iter()
            .find(|entry| entry.0 == HANDLE_ENTRY_POINT)
            .unwrap();
        assert_eq!(entry.1[0..2], [0x2a, actor.addr()]);
    }

    #[test]
    fn run_speed_is_scaled_by_the_combat_controller_before_the_entry_point() {
        let mut e = engine();
        let actor = speed_formula_setup(&mut e);
        let controller = Ptr::new(e.mem.alloc(0x100));
        e.mem.set_f32(controller.addr() + 0xdc, 0.5);
        slot(
            &mut e,
            ACTOR_TABLE,
            VSLOT_GET_COMBAT_CONTROLLER,
            eax(controller.addr()),
        );
        e.register(HANDLE_ENTRY_POINT, |e, a| {
            let value = e.mem.f32(a[2]);
            e.mem.set_f32(a[2], value + 1.0);
            Ret::default()
        });
        assert_eq!(actor_calculate_run_speed(&mut e, actor), 3.0);
        assert_eq!(fn_00885d70(&mut e, controller), 0.5);
    }

    // -----------------------------------------------------------------
    // Tests of the second block (b0148p2)

    const OTHER_TABLE: u32 = 0x0200_6000;
    const DEFAULT_TARGET: u32 = 0x0300_f000;
    const SCRATCH_FLOAT: u32 = 0x0101_2100;

    /// Callees of the original block that the new functions also call.
    const EARLIER_CALLEES: [u32; 25] = [
        GET_FLAG_WORD,
        SPEED_FLAG_TEST,
        GET_ARMOR_BEING_WORN,
        ITEM_VALUE,
        ITEM_RELEASE,
        HAS_CACHED_VALUES,
        PROCESS_FORWARD_FLAG_MASK,
        STOP_MOVING,
        OBJECT_TYPE,
        GET_EYE_LEVEL,
        CLAMP_FLOAT,
        GET_PARENT_CELL,
        CELL_TEST_INTERIOR,
        SAVE_FORM_BUFFER_GET_FORM,
        FORM_TYPE,
        GET_RELEVANT_WATER_HEIGHT,
        GET_SCALE,
        ANIMATION_LOOKUP,
        ANIMATION_STEP,
        GET_CHAR_CONTROLLER,
        CACHED_HEIGHT_BYPASS,
        FOLLOW_MODE_ALLOWED,
        HANDLE_ENTRY_POINT,
        CONTROLLER_STATE,
        CELL_GET_WATER_HEIGHT,
    ];

    const CODE_CALLEES: [u32; 285] = [
        0x00401000, 0x00403df0, 0x00406d30, 0x00408d60, 0x0040ab30, 0x00416870, 0x00418890,
        0x00418900, 0x00418940, 0x00418a00, 0x0041a090, 0x0041a250, 0x0041a540, 0x0041a800,
        0x0041d7b0, 0x00421400, 0x0042e040, 0x0042e8c0, 0x004301b0, 0x00430830, 0x00436aa0,
        0x00437bd0, 0x00439e90, 0x00439ef0, 0x0043b4d0, 0x0043b4f0, 0x0043d450, 0x0043d4d0,
        0x0043f850, 0x0043f8d0, 0x0043fa80, 0x0043fcd0, 0x00440460, 0x00440da0, 0x00446390,
        0x0044ddc0, 0x00450b80, 0x00450ff0, 0x004511e0, 0x00453700, 0x004537b0, 0x00453a70,
        0x00457990, 0x0045bb80, 0x0045cbc0, 0x0045cd60, 0x0047fd90, 0x00483710, 0x0048f810,
        0x0048ffd0, 0x00490330, 0x00491180, 0x004937e0, 0x004938e0, 0x00493900, 0x004939d0,
        0x00493bb0, 0x00493bd0, 0x00494390, 0x00494710, 0x004955a0, 0x004955c0, 0x00496080,
        0x004964d0, 0x00498910, 0x004994f0, 0x0049bdc0, 0x004a0c90, 0x004a3760, 0x004a3a20,
        0x004a3e90, 0x004a3f10, 0x004a4240, 0x004ade00, 0x004ae750, 0x004b3ae0, 0x004b4500,
        0x004b6e70, 0x004d50e0, 0x004dd150, 0x004e75d0, 0x004eaf60, 0x004f15a0, 0x004f8960,
        0x00508100, 0x0050d4a0, 0x00511840, 0x00517630, 0x0051f5f0, 0x00524ac0, 0x00524b40,
        0x00524d10, 0x00525420, 0x00526100, 0x00547770, 0x0054a070, 0x0054ddd0, 0x00550200,
        0x00552490, 0x005532a0, 0x00559450, 0x0055b980, 0x005609b0, 0x00560d30, 0x00561580,
        0x005653d0, 0x00566950, 0x00566970, 0x00567050, 0x00567730, 0x00567750, 0x0056ae60,
        0x0056fac0, 0x00571560, 0x005715d0, 0x005725f0, 0x00572e50, 0x00573800, 0x00574400,
        0x00575830, 0x00575b70, 0x00576d30, 0x00579670, 0x00580060, 0x005a2030, 0x005b5e40,
        0x005b9e80, 0x005c53d0, 0x005ce8f0, 0x005d43c0, 0x005e39b0, 0x005f2440, 0x005f2540,
        0x005f2670, 0x005f92d0, 0x00621480, 0x00621a40, 0x00622570, 0x00629670, 0x0062c430,
        0x0062eb50, 0x00644a50, 0x00647f50, 0x00647f90, 0x00647fd0, 0x00648a10, 0x00648a50,
        0x00653270, 0x00678610, 0x0067a4f0, 0x006815c0, 0x006838b0, 0x00689210, 0x0068a7d0,
        0x006d2c20, 0x006fa820, 0x00702360, 0x00702680, 0x0070f490, 0x00717e50, 0x00726070,
        0x00778950, 0x007d1d00, 0x007d6bb0, 0x007df1f0, 0x008041a0, 0x00810610, 0x00824970,
        0x00849730, 0x0084d030, 0x0084e3a0, 0x00867950, 0x00867d60, 0x00867da0, 0x00867e30,
        0x00867ef0, 0x0087ab50, 0x0087b850, 0x0087ce50, 0x0087e9a0, 0x0087eeb0, 0x0087f390,
        0x0087f3d0, 0x0087f4a0, 0x0087f570, 0x0087f5c0, 0x0087f620, 0x00886b10, 0x00888960,
        0x00888970, 0x0088b030, 0x0088b0a0, 0x0088b0c0, 0x0088b0f0, 0x00894900, 0x00894cc0,
        0x00894d60, 0x00895110, 0x008984c0, 0x008985d0, 0x0089f580, 0x008a0960, 0x008a0c60,
        0x008a1710, 0x008a1800, 0x008a40e0, 0x008a6840, 0x008a6970, 0x008a6ce0, 0x008a7a40,
        0x008a7d50, 0x008adcb0, 0x008b00c0, 0x008b28c0, 0x008b3230, 0x008b39f0, 0x008b3a80,
        0x008b70d0, 0x008bbdb0, 0x008bcdf0, 0x008bd7b0, 0x008be7a0, 0x008bffa0, 0x008c1470,
        0x008c26e0, 0x008c2b60, 0x008c3c40, 0x008c4330, 0x008c51f0, 0x008c7aa0, 0x008d0370,
        0x008d5cb0, 0x008d8520, 0x008d85e0, 0x008fec10, 0x009314d0, 0x00931850, 0x00931ed0,
        0x00933270, 0x009334b0, 0x0094df60, 0x00950a60, 0x00952290, 0x00953c50, 0x009549a0,
        0x009611e0, 0x00968730, 0x0096e870, 0x0098adb0, 0x009a1260, 0x009bb080, 0x009c8d60,
        0x009dcb90, 0x009dccf0, 0x00ad7480, 0x00ad7550, 0x00ad8830, 0x00ad88f0, 0x00ad8930,
        0x00ad89e0, 0x00ad8b60, 0x00ad8ce0, 0x00ad8d10, 0x00ad8f20, 0x00b4f5c0, 0x00b5d9f0,
        0x00b5f080, 0x00b65f60, 0x00c66230, 0x00c6a350, 0x00c6c3d0, 0x00c6d4d0, 0x00c6d6b0,
        0x00c7c150, 0x00c7d630, 0x00c802d0, 0x00c804d0, 0x00c821b0, 0x00c82580, 0x00c8cce0,
        0x00c8dac0, 0x00c9b670, 0x00ca2ad0, 0x00cb8fe0, 0x00ec62c0,
    ];
    const GLOBAL_ADDRESSES: [u32; 70] = [
        0x01013dc0, 0x01015f5c, 0x01016408, 0x01016ff0, 0x0101712c, 0x01017718, 0x01017b70,
        0x0101a6b0, 0x0101e2bc, 0x0101e2c0, 0x0101ffa0, 0x01020998, 0x010231b0, 0x010241b0,
        0x0102caf8, 0x01035838, 0x0104ee98, 0x0104f4b0, 0x01073828, 0x01084838, 0x01084840,
        0x01084898, 0x010848a0, 0x010848b0, 0x010848b8, 0x010848d0, 0x0118a838, 0x01199760,
        0x011a32d8, 0x011a9484, 0x011c3ea4, 0x011c6260, 0x011d0460, 0x011d058c, 0x011d0c3c,
        0x011d0fa8, 0x011d12c0, 0x011d1458, 0x011d1464, 0x011ddf38, 0x011de45c, 0x011de7b8,
        0x011dea10, 0x011df69c, 0x011df69d, 0x011df69e, 0x011df69f, 0x011df6b0, 0x011df71d,
        0x011e0550, 0x011e07a8, 0x011e07d0, 0x011e07d4, 0x011e07e8, 0x011e0888, 0x011e0e80,
        0x011f1220, 0x011f122c, 0x011f21cc, 0x011f2250, 0x011f2ac4, 0x011f2ad8, 0x011f2b10,
        0x011f2c10, 0x011f3e60, 0x011f426c, 0x011f6394, 0x012677a3, 0x01267e64, 0x012683ec,
    ];

    /// An engine where every callee of the second block answers zero, the
    /// vtables have default slots and the globals' pages exist.
    fn engine2() -> Engine {
        let mut e = engine();
        for table in [ACTOR_TABLE, OWNER_TABLE, PROCESS_TABLE, OTHER_TABLE] {
            e.map(table, 0x800);
            for offset in (0..0x800).step_by(4) {
                e.mem.set_u32(table + offset, DEFAULT_TARGET);
            }
        }
        e.register(DEFAULT_TARGET, |_, _| Ret::default());
        for address in GLOBAL_ADDRESSES {
            e.map(address, 0x10);
        }
        for address in CODE_CALLEES.iter().chain(EARLIER_CALLEES.iter()) {
            e.register(*address, |_, _| Ret::default());
        }
        e.register(GET_PROCESS, |e, a| eax(e.mem.u32(a[0] + 0x68)));
        e.register(SETTING_VALUE_ADDRESS, |_, _| eax(SCRATCH_FLOAT));
        e.register(EMPTY_FUNCTION, |_, _| Ret::default());
        e
    }

    fn actor2(e: &mut Engine) -> Ptr<Actor> {
        let actor = new_actor(e);
        e.mem.set_u32(actor.addr() + 0x94, OTHER_TABLE);
        e.mem
            .set_u32(actor.addr() + ACTOR_VALUE_OWNER_OFFSET, OTHER_TABLE);
        actor
    }

    fn set_player(e: &mut Engine, player: Ptr<Actor>) {
        e.mem.set_u32(PLAYER_CHARACTER, player.addr());
    }

    fn args_of(e: &Engine, address: u32) -> Vec<u32> {
        logged(e)
            .into_iter()
            .find(|entry| entry.0 == address)
            .unwrap_or_else(|| panic!("{address:08x} was not called"))
            .1
    }

    fn was_called(e: &Engine, address: u32) -> bool {
        logged(e).iter().any(|entry| entry.0 == address)
    }

    #[test]
    fn weighted_speed_passes_the_six_formula_words() {
        let mut e = engine2();
        let actor = speed_formula_setup(&mut e);
        double(&mut e, SPEED_FORMULA_00647F50, st0(4.0));
        e.call_log = Some(vec![]);
        assert_eq!(fn_00885d90(&mut e, actor), 4.0);
        assert_eq!(
            args_of(&e, SPEED_FORMULA_00647F50),
            vec![actor.addr() + 0xa4, 0x112, 0x223, 1, 0, 1]
        );
    }

    #[test]
    fn second_weighted_speed_uses_the_other_formula() {
        let mut e = engine2();
        let actor = speed_formula_setup(&mut e);
        double(&mut e, SPEED_FORMULA_00647F90, st0(5.0));
        e.call_log = Some(vec![]);
        assert_eq!(fn_00885ed0(&mut e, actor), 5.0);
        assert!(was_called(&e, SPEED_FORMULA_00647F90));
    }

    #[test]
    fn speed_00886010_is_zero_when_the_virtual_says_so() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        slot(&mut e, ACTOR_TABLE, VSLOT_FLAG_0X358, eax(1));
        assert_eq!(fn_00886010(&mut e, actor), 0.0);
    }

    #[test]
    fn speed_00886010_applies_the_formula_to_the_actor_value() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        double(&mut e, ACTOR_SPEED_VALUE_008A0C60, st0(2.5));
        double(&mut e, SPEED_FORMULA_00647FD0, st0(7.0));
        e.call_log = Some(vec![]);
        assert_eq!(fn_00886010(&mut e, actor), 7.0);
        let formula = args_of(&e, SPEED_FORMULA_00647FD0);
        assert_eq!(formula, vec![actor.addr() + 0xa4, 2.5f32.to_bits()]);
    }

    #[test]
    fn package_location_needs_a_process_and_an_object() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        assert_eq!(actor_get_current_package_location(&mut e, actor), 0);
        with_process(&mut e, actor);
        assert_eq!(actor_get_current_package_location(&mut e, actor), 0);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X27C, eax(0x1234));
        double(&mut e, PACKAGE_LOCATION_OF, eax(0x99));
        assert_eq!(actor_get_current_package_location(&mut e, actor), 0x99);
    }

    #[test]
    fn process_level_without_a_process_is_false() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        assert!(!fn_008860d0(&mut e, actor));
    }

    #[test]
    fn process_level_already_desired_is_true() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        with_process(&mut e, actor);
        double(&mut e, PROCESS_GET_LEVEL, eax(2));
        double(&mut e, GET_DESIRED_PROCESS_LEVEL, eax(2));
        assert!(fn_008860d0(&mut e, actor));
    }

    #[test]
    fn process_level_unloads_a_non_persistent_reference() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        with_process(&mut e, actor);
        double(&mut e, PROCESS_GET_LEVEL, eax(3));
        double(&mut e, GET_DESIRED_PROCESS_LEVEL, eax(2));
        double(&mut e, GET_REF_PERSISTS, eax(0));
        double(&mut e, GET_PARENT_CELL, eax(0x5000));
        e.call_log = Some(vec![]);
        assert!(!fn_008860d0(&mut e, actor));
        let log = logged(&e);
        assert!(was_called(&e, ACTOR_CAST_PERMANENT_MAGIC));
        assert!(was_called(&e, SAVE_LOAD_UNLOAD_FORM));
        assert_eq!(args_of(&e, ACTOR_DISPEL_ALL), vec![actor.addr() + 0x94]);
        assert!(log.iter().any(|entry| entry.0 == DEFAULT_TARGET));
    }

    #[test]
    fn process_level_keeps_a_persistent_reference() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        with_process(&mut e, actor);
        double(&mut e, PROCESS_GET_LEVEL, eax(3));
        double(&mut e, GET_DESIRED_PROCESS_LEVEL, eax(3));
        double(&mut e, GET_REF_PERSISTS, eax(1));
        assert!(fn_008860d0(&mut e, actor));
    }

    #[test]
    fn update_returns_at_once_when_the_actor_is_not_processed() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        e.call_log = Some(vec![]);
        fn_00886360(&mut e, actor, 0.1);
        assert_eq!(logged(&e).len(), 1);
        double(&mut e, ACTOR_SKIP_UPDATE_TEST, eax(1));
        e.set(actor, Actor::bProcessMe, true);
        fn_00886360(&mut e, actor, 0.1);
        assert!(!was_called(&e, EMPTY_FUNCTION));
    }

    #[test]
    fn update_evaluates_the_package_of_a_living_actor() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        e.set(actor, Actor::bProcessMe, true);
        e.mem.set_u8(PROCESSING_ENABLED, 1);
        double(&mut e, UPDATE_STEP_SETTING, st0(0.5));
        double(&mut e, SETTING_INT_VALUE, eax(1));
        double(&mut e, GLOBAL_WORD_ADDRESS, eax(SCRATCH_FLOAT));
        double(&mut e, SETTING_VALUE_POINTER, eax(SCRATCH_FLOAT));
        slot(
            &mut e,
            ACTOR_TABLE,
            VSLOT_EVALUATE_PACKAGE_TEST_0X26C,
            eax(1),
        );
        double(&mut e, ACTOR_EVALUATE_FLAG, eax(1));
        e.call_log = Some(vec![]);
        fn_00886360(&mut e, actor, 0.25);
        assert_eq!(
            args_of(&e, ACTOR_EVALUATE_PACKAGE),
            vec![actor.addr(), 1, 1]
        );
        assert!(was_called(&e, EXTRA_LIST_SET_DAY));
    }

    #[test]
    fn update_runs_the_dead_body_timers_down() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        e.set(actor, Actor::bProcessMe, true);
        e.set(actor, Actor::bDeadFlag, true);
        e.set(actor, Actor::fCheckMyDeadBodyTimer, 5.0);
        e.set(actor, Actor::fDeadBodyAlarm, 1.0);
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_PROCESS_FLAG_0X22C, eax(1));
        double(&mut e, UPDATE_STEP_SETTING, st0(0.5));
        double(&mut e, SETTING_INT_VALUE, eax(1));
        double(&mut e, GLOBAL_WORD_ADDRESS, eax(SCRATCH_FLOAT));
        double(&mut e, ACTOR_GET_ESSENTIAL, eax(1));
        fn_00886360(&mut e, actor, 0.25);
        assert_eq!(e.get(actor, Actor::fCheckMyDeadBodyTimer), 4.5);
        assert_eq!(e.get(actor, Actor::fDeadBodyAlarm), 0.5);
    }

    #[test]
    fn update_tells_the_player_when_the_dead_alarm_goes_off() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let player = actor2(&mut e);
        set_player(&mut e, player);
        with_process(&mut e, actor);
        with_process(&mut e, player);
        e.set(actor, Actor::bProcessMe, true);
        e.set(actor, Actor::bDeadFlag, true);
        e.set(actor, Actor::fDeadBodyAlarm, 0.25);
        e.set(actor, Actor::pMyKiller, player.cast());
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_PROCESS_FLAG_0X22C, eax(1));
        double(&mut e, UPDATE_STEP_SETTING, st0(0.5));
        double(&mut e, SETTING_INT_VALUE, eax(1));
        double(&mut e, GLOBAL_WORD_ADDRESS, eax(SCRATCH_FLOAT));
        double(&mut e, ACTOR_GET_ESSENTIAL, eax(1));
        e.set_global(DEAD_BODY_ALARM_RESET, 3.0f32);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0XFC, Ret::default());
        let position = e.mem.alloc(16);
        e.mem.set_f32(position, 1.0);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_POSITION_0X1F4, eax(position));
        e.call_log = Some(vec![]);
        fn_00886360(&mut e, actor, 0.25);
        let target = slot_target(PROCESS_TABLE, PROCESS_VSLOT_0XFC);
        let process = e.get(player, Actor::pCurrentProcess).addr();
        assert_eq!(
            args_of(&e, target),
            vec![process, player.addr(), 1.0f32.to_bits(), 0, 0, 1, 4, 0]
        );
        assert_eq!(e.get(actor, Actor::fDeadBodyAlarm), 3.0);
    }

    #[test]
    fn reset_ai_flag_is_returned() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        assert!(!fn_00886b10(&mut e, actor));
        e.set(actor, Actor::bResetAI, true);
        assert!(fn_00886b10(&mut e, actor));
    }

    #[test]
    fn bip_transform_needs_a_process() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let vector = Ptr::new(e.mem.alloc(12));
        assert!(!fn_00886b30(&mut e, actor, vector));
    }

    #[test]
    fn bip_transform_rotates_the_vector_by_the_heading() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        with_process(&mut e, actor);
        slot(&mut e, ACTOR_TABLE, VSLOT_PACKAGE_TEST_0X230, eax(0));
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_PROCESS_FLAG_0X22C, eax(1));
        let vector = Ptr::new(e.mem.alloc(12));
        let result = e.mem.alloc(12);
        e.mem.set_f32(result, 9.0);
        e.mem.set_f32(result + 4, 8.0);
        e.mem.set_f32(result + 8, 7.0);
        let rotation = e.mem.alloc(16);
        e.mem.set_f32(rotation + 8, 0.5);
        double(&mut e, ACTOR_ROTATION_VECTOR, eax(rotation));
        double(&mut e, VECTOR_TIMES_MATRIX, eax(result));
        double(&mut e, BLEND_OBJECT_OF, eax(0x777));
        let controller = e.mem.alloc(0x500);
        double(&mut e, GET_CHAR_CONTROLLER, eax(controller));
        e.call_log = Some(vec![]);
        assert!(fn_00886b30(&mut e, actor, vector));
        assert_eq!(e.mem.f32(vector.addr() + 4), 8.0);
        assert_eq!(
            args_of(&e, SET_BIP_TRANSFORM),
            vec![0x777, vector.addr(), 0]
        );
        assert_eq!(
            args_of(&e, FLAG_SET_OR_CLEAR),
            vec![controller + 0x410, 0x1000, 1]
        );
        assert_eq!(args_of(&e, MATRIX_MAKE_ROTATION)[1], 0.5f32.to_bits());
    }

    #[test]
    fn controller_flag_setters_pass_their_masks() {
        let mut e = engine2();
        e.call_log = Some(vec![]);
        fn_00886c80(&mut e, Ptr::new(0x5000), true);
        assert_eq!(args_of(&e, FLAG_SET_OR_CLEAR), vec![0x5410, 0x1000, 1]);
        e.call_log = Some(vec![]);
        fn_008879d0(&mut e, Ptr::new(0x5000), false);
        assert_eq!(args_of(&e, FLAG_SET_OR_CLEAR), vec![0x5410, 0x40000, 0]);
    }

    #[test]
    fn global_word_is_returned() {
        let mut e = engine2();
        e.mem.set_u32(GLOBAL_VALUE_011C6260, 0x1357);
        assert_eq!(fn_00887a00(&mut e), 0x1357);
    }

    #[test]
    fn proxy_value_prefers_the_proxy_object() {
        let mut e = engine2();
        let object = Ptr::<()>::new(e.mem.alloc(0x20));
        double(&mut e, WORD_AT_0XC, eax(0x42));
        double(&mut e, PROXY_OF, eax(0x9000));
        assert_eq!(fn_00887a10(&mut e, object.cast()), 0x42);
    }

    #[test]
    fn proxy_value_falls_back_to_the_linked_word() {
        let mut e = engine2();
        let object = e.mem.alloc(0x20);
        assert_eq!(fn_00887a10(&mut e, Ptr::new(object)), 0);
        let target = e.mem.alloc(8);
        e.mem.set_u32(target, 0x5566);
        e.mem.set_u32(object + 0xc, target);
        assert_eq!(fn_00887a10(&mut e, Ptr::new(object)), 0x5566);
    }

    #[test]
    fn motor_flag_builds_the_value_on_the_stack() {
        let mut e = engine2();
        let object = e.mem.alloc(0x20);
        let target = e.mem.alloc(8);
        e.mem.set_u32(object + 0xc, target);
        e.register(BYTE_STORE, |e, a| {
            e.mem.set_u8(a[0], a[1] as u8);
            Ret::default()
        });
        double(&mut e, PROXY_OF, eax(0x9000));
        e.mem.set_u32(target, 0x5566);
        double(&mut e, WORD_AT_0XC, eax(0x4321));
        e.call_log = Some(vec![]);
        fn_00887a80(&mut e, Ptr::new(object), true);
        let call = args_of(&e, SET_MOTOR_ACTIVE);
        assert_eq!(call[0], 0x4321);
        assert_eq!(call[1], 0x9000);
        assert_eq!(call[2] & 0xff, 1);
    }

    #[test]
    fn float_setter_forwards_to_the_target() {
        let mut e = engine2();
        let object = e.mem.alloc(0x20);
        let target = e.mem.alloc(8);
        e.mem.set_u32(object + 0xc, target);
        e.mem.set_u32(target, 0x1111);
        e.call_log = Some(vec![]);
        fn_00887ac0(&mut e, Ptr::new(object), 2.0);
        assert_eq!(
            args_of(&e, SET_FLOAT_ON_TARGET),
            vec![0x1111, 2.0f32.to_bits()]
        );
    }

    #[test]
    fn proxy_forwarders_reach_their_targets() {
        let mut e = engine2();
        double(&mut e, PROXY_OF, eax(0x6000));
        e.call_log = Some(vec![]);
        fn_00887af0(&mut e, Ptr::new(0x5000), 77);
        assert_eq!(args_of(&e, SUBOBJECT_SET_VALUE), vec![0x6040, 77]);
        let proxy = e.mem.alloc(0x80);
        e.register(PROXY_OF, |_, _| eax(0x7000));
        fn_00887b20(&mut e, Ptr::new(proxy), 5);
        assert_eq!(args_of(&e, SUBOBJECT_SET_VALUE), vec![0x6040, 77]);
        fn_00887b70(&mut e, Ptr::new(proxy), 3);
        assert_eq!(e.mem.u8(proxy + 0x5c), 3);
        e.mem.map(0x7000, 0x100);
        fn_00887b40(&mut e, Ptr::new(0x5000), 9);
        assert_eq!(e.mem.u8(0x705c), 9);
    }

    #[test]
    fn muzzle_step_skips_flagged_references() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        double(&mut e, REFERENCE_FLAG_TEST_0X200000, eax(1));
        e.call_log = Some(vec![]);
        fn_00887b90(&mut e, actor, 0.1, 0);
        assert_eq!(logged(&e).len(), 1);
    }

    #[test]
    fn muzzle_step_updates_the_flash_and_fades() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        slot(
            &mut e,
            PROCESS_TABLE,
            PROCESS_VSLOT_MUZZLE_FLASH,
            eax(0xf1a5),
        );
        double(&mut e, SETTING_INT_VALUE, eax(1));
        double(&mut e, PROCESS_GET_LEVEL, eax(0));
        e.call_log = Some(vec![]);
        fn_00887b90(&mut e, actor, 0.5, 0);
        assert_eq!(
            args_of(&e, MUZZLE_FLASH_UPDATE),
            vec![0xf1a5, 0.5f32.to_bits(), actor.addr()]
        );
        let process = e.get(actor, Actor::pCurrentProcess).addr();
        assert_eq!(
            args_of(&e, HIGH_PROCESS_FADE_UPDATE),
            vec![process, actor.addr()]
        );
    }

    #[test]
    fn reset_loaded_animations_ignores_the_player() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        set_player(&mut e, actor);
        e.call_log = Some(vec![]);
        actor_reset_loaded_animations(&mut e, actor);
        assert_eq!(logged(&e).len(), 0);
    }

    #[test]
    fn reset_loaded_animations_can_be_queued() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        double(&mut e, RESET_ANIMS_SHOULD_QUEUE, eax(1));
        double(&mut e, TASK_QUEUE_INTERFACE, eax(0x8888));
        e.call_log = Some(vec![]);
        actor_reset_loaded_animations(&mut e, actor);
        assert_eq!(
            args_of(&e, QUEUE_ACTOR_RESET_LOADED_ANIMS),
            vec![0x8888, actor.addr()]
        );
        assert!(!was_called(&e, SET_ANIMATION));
    }

    #[test]
    fn reset_loaded_animations_builds_a_new_animation() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        double(&mut e, OPERATOR_NEW_FN, eax(0x2000));
        double(&mut e, ANIMATION_CONSTRUCTOR, eax(0x2000));
        double(&mut e, SET_ANIMATION, eax(0x2000));
        double(&mut e, BUILD_KF_FILE_LIST, eax(0x3000));
        double(&mut e, INIT_ANIMATION, eax(1));
        double(&mut e, PROCESS_GET_LEVEL, eax(2));
        e.mem.map(0x5000, 0x1000);
        e.mem.set_u32(0x5100 + 0xc, 0xbeef);
        let skeleton = e.mem.alloc(0x40);
        e.mem.set_u32(skeleton, OTHER_TABLE);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(skeleton));
        e.register(0x0041_81e0, |_, _| eax(0x5100));
        slot(&mut e, ACTOR_TABLE, VSLOT_IS_PROCESS_FLAG_0X22C, eax(0));
        e.set(actor, Actor::eLifeState, 2);
        e.call_log = Some(vec![]);
        actor_reset_loaded_animations(&mut e, actor);
        let log = logged(&e);
        assert_eq!(
            log.iter().filter(|entry| entry.0 == SET_ANIMATION).count(),
            2
        );
        assert_eq!(args_of(&e, OPERATOR_NEW_FN), vec![ANIMATION_SIZE]);
        let init = args_of(&e, INIT_ANIMATION);
        assert_eq!(init[0], 0x2000);
        assert_eq!(init[1], 0x3000);
        assert_eq!(init[3], actor.addr());
        assert_eq!(init[4], 1);
    }

    #[test]
    fn animation_step_ignores_an_actor_without_a_3d_object() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        e.call_log = Some(vec![]);
        fn_00888070(&mut e, actor, 0.1);
        assert!(!was_called(&e, PICK_ANIMATIONS));
    }

    #[test]
    fn animation_step_picks_the_animations() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        double(&mut e, PACKAGE_TYPE, eax(1));
        double(&mut e, SETTING_INT_VALUE, eax(1));
        double(&mut e, UPDATE_STEP_SETTING, st0(0.25));
        e.set_global(0x011f_2ac4, 0.0f32);
        e.set_global(0x011f_2ad8, 6.0f32);
        double(&mut e, SETTING_VALUE_ADDRESS, eax(0x011f_2ad8));
        slot(&mut e, ACTOR_TABLE, VSLOT_ANIMATION_FLAG_0X21C, eax(0));
        e.call_log = Some(vec![]);
        fn_00888070(&mut e, actor, 0.1);
        assert_eq!(
            args_of(&e, PICK_ANIMATIONS),
            vec![actor.addr(), 1.0f32.to_bits(), 6.0f32.to_bits()]
        );
        let process = e.get(actor, Actor::pCurrentProcess).addr();
        assert_eq!(args_of(&e, ACTOR_PRE_ANIMATION_STEP), vec![actor.addr()]);
        let _ = process;
    }

    #[test]
    fn animation_step_finishes_the_death_animation() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_ANIMATION, eax(0x4000));
        double(&mut e, PACKAGE_TYPE, eax(1));
        double(&mut e, SETTING_INT_VALUE, eax(1));
        double(&mut e, BLEND_OBJECT_OF, eax(0x777));
        double(&mut e, ANIMATION_GROUP_LOADED, eax(1));
        double(&mut e, SAVE_FORM_BUFFER_GET_FORM, eax(0x600));
        double(&mut e, FORM_TYPE, eax(0x2b));
        double(&mut e, GLOBAL_WORD_ADDRESS, eax(SCRATCH_FLOAT));
        slot(&mut e, ACTOR_TABLE, VSLOT_ACTOR_FORM_TEST_0X38C, eax(0));
        e.set(actor, Actor::bDeadFlag, true);
        e.call_log = Some(vec![]);
        fn_00888070(&mut e, actor, 0.1);
        assert_eq!(args_of(&e, ACTOR_SET_LIFE_STATE), vec![actor.addr(), 1]);
        assert_eq!(
            args_of(&e, ANIMATION_FORCE_SECTION),
            vec![0x4000, 0x14, 0xe0, 0xffff_ffff, 0, 0xffff_ffff]
        );
        assert!(!e.get(actor, Actor::bDeadFlag));
    }

    #[test]
    fn movement_update_needs_an_animation() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        e.call_log = Some(vec![]);
        fn_008885a0(&mut e, actor);
        assert!(!was_called(&e, ANIMATION_UPDATE_MOVEMENT_NO_WORLD_UPDATE));
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_ANIMATION, eax(0x4000));
        e.call_log = Some(vec![]);
        fn_008885a0(&mut e, actor);
        assert_eq!(
            args_of(&e, ANIMATION_UPDATE_MOVEMENT_NO_WORLD_UPDATE),
            vec![0x4000, actor.addr()]
        );
    }

    #[test]
    fn animation_movement_counts_the_player_while_it_runs() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        set_player(&mut e, actor);
        e.mem.set_u8(PLAYER_UPDATE_NESTING, 4);
        e.call_log = Some(vec![]);
        actor_update_animation_movement(&mut e, actor, Ptr::new(0), 0.0);
        assert_eq!(e.mem.u8(PLAYER_UPDATE_NESTING), 4);
        assert_eq!(logged(&e).len(), 2);
    }

    #[test]
    fn animation_movement_updates_the_scene_graph_and_scales() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        double(&mut e, SETTING_INT_VALUE, eax(1));
        double(&mut e, UPDATE_STEP_SETTING, st0(0.5));
        double(&mut e, GLOBAL_WORD_ADDRESS, eax(SCRATCH_FLOAT));
        slot(&mut e, ACTOR_TABLE, VSLOT_KIND_0X214, eax(3));
        double(&mut e, PLAYER_GET_ANIMATION, eax(0x7777));
        double(&mut e, GET_SCALE, st0(2.0));
        e.set_global(DOUBLE_ONE, 1.0f64);
        e.register(ANIMATION_SCALE_VECTOR, |e, a| {
            e.mem.set_f32(a[1] + 8, 4.0);
            Ret::default()
        });
        let position = e.mem.alloc(16);
        e.mem.set_f32(position + 8, 10.0);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_POSITION_0X1F4, eax(position));
        e.call_log = Some(vec![]);
        actor_update_animation_movement(&mut e, actor, Ptr::new(0x4000), 0.0);
        assert!(was_called(&e, ANIMATION_UPDATE_SCENE_GRAPH_NO_CONTROLLER));
        // The offset 4 - 4 * 2 = -4 lowers the position by that much.
        assert_eq!(
            args_of(&e, SET_LOCATION_ON_REFERENCE_Z),
            vec![actor.addr(), 14.0f32.to_bits()]
        );
    }

    #[test]
    fn small_accessors_of_the_ragdoll_controller() {
        let mut e = engine2();
        let controller = Ptr::<()>::new(e.mem.alloc(0x300));
        e.mem.set_u8(GLOBAL_FLAG_012677A3, 7);
        assert_eq!(fn_00888960(&mut e), 7);
        e.mem.set_u8(controller.addr() + 0xb3, 1);
        assert!(fn_00888a50(&mut e, controller));
        e.mem.set_f32(controller.addr() + 0x19c, 1.5);
        e.mem.set_f32(controller.addr() + 0x1a0, 2.5);
        let out = e.mem.alloc(8);
        fn_00888a70(&mut e, controller, Ptr::new(out), Ptr::new(out + 4));
        assert_eq!(e.mem.f32(out), 1.5);
        assert_eq!(e.mem.f32(out + 4), 2.5);
        let face = Ptr::<()>::new(e.mem.alloc(0x200));
        fn_00888a20(&mut e, face, 3.0, 4.0);
        assert_eq!(e.mem.f32(face.addr() + 0x150), 3.0);
        assert_eq!(e.mem.f32(face.addr() + 0x154), 4.0);
        e.mem.set_f32(controller.addr() + 0x23c, 9.0);
        fn_00888aa0(&mut e, controller);
        assert_eq!(e.mem.f32(controller.addr() + 0x23c), 0.0);
    }

    #[test]
    fn ragdoll_face_step_copies_the_controller_floats() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let controller = e.mem.alloc(0x300);
        e.set(actor, Actor::pRagdollController, Ptr::new(controller));
        e.mem.set_u8(controller + 0xb3, 1);
        e.mem.set_f32(controller + 0x19c, 1.5);
        e.mem.set_f32(controller + 0x1a0, 2.5);
        let face = e.mem.alloc(0x200);
        double(&mut e, GET_FACE_ANIMATION_DATA, eax(face));
        double(&mut e, RAGDOLL_TEST_0X4955A0, eax(0));
        e.mem.set_f32(controller + 0x23c, 9.0);
        e.call_log = Some(vec![]);
        fn_00888970(&mut e, actor, 1);
        assert_eq!(args_of(&e, DO_RAGDOLL_ANIM), vec![controller, 1]);
        assert_eq!(e.mem.f32(face + 0x150), 1.5);
        assert_eq!(e.mem.f32(face + 0x154), 2.5);
        assert_eq!(e.mem.f32(controller + 0x23c), 0.0);
        e.mem.set_u8(controller + 0xb3, 0);
        fn_00888970(&mut e, actor, 1);
        assert_eq!(e.mem.f32(face + 0x150), 0.0);
    }

    #[test]
    fn process_forwarders_use_their_slots() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        with_process(&mut e, actor);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X2F0, Ret::default());
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X2F4, Ret::default());
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X2F8, Ret::default());
        e.call_log = Some(vec![]);
        fn_00888ac0(&mut e, actor, 1.5);
        fn_00888af0(&mut e, actor);
        fn_00888b20(&mut e, actor);
        let log = logged(&e);
        let process = e.get(actor, Actor::pCurrentProcess).addr();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].1, vec![process, actor.addr(), 1.5f32.to_bits()]);
        assert_eq!(log[1].1, vec![process]);
        assert_eq!(log[2].1, vec![process]);
    }

    #[test]
    fn update_stops_at_the_cell_checks() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X4BC, eax(5));
        e.call_log = Some(vec![]);
        actor_update(&mut e, actor, 0.1);
        assert_eq!(e.get(actor, Actor::cCurrentSitSleepState), 5);
        assert!(!was_called(&e, ACTOR_TIMED_UPDATE));
    }

    #[test]
    fn update_wakes_the_process_for_a_long_frame() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        with_process(&mut e, actor);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        double(&mut e, GET_PARENT_CELL, eax(0x5000));
        double(&mut e, CELL_TEST_0X450FF0, eax(1));
        e.set_global(UPDATE_TIME_LIMIT, 900.0f64);
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X5F8, Ret::default());
        e.call_log = Some(vec![]);
        actor_update(&mut e, actor, 1000.0);
        let process = e.get(actor, Actor::pCurrentProcess).addr();
        let wake = slot_target(PROCESS_TABLE, PROCESS_VSLOT_0X5F8);
        assert_eq!(args_of(&e, wake), vec![process, 1]);
        assert!(!was_called(&e, ACTOR_TIMED_UPDATE));
    }

    fn update_setup(e: &mut Engine) -> Ptr<Actor> {
        let actor = actor2(e);
        let other = actor2(e);
        set_player(e, other);
        with_process(e, actor);
        slot(e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        double(e, GET_PARENT_CELL, eax(0x5000));
        double(e, CELL_TEST_0X450FF0, eax(1));
        e.set_global(UPDATE_TIME_LIMIT, 900.0f64);
        double(e, SETTING_INT_VALUE, eax(1));
        double(e, UPDATE_STEP_SETTING, st0(0.5));
        double(e, GLOBAL_WORD_ADDRESS, eax(SCRATCH_FLOAT));
        double(e, ACTOR_GET_LIFE_STATE, eax(1));
        e.set_global(DOUBLE_ONE, 1.0f64);
        let position = e.mem.alloc(16);
        double(e, POSITION_VECTOR_OF, eax(position));
        double(e, VECTOR_PLUS_VECTOR, eax(position));
        slot(e, ACTOR_TABLE, VSLOT_GET_POSITION_0X1F4, eax(position));
        actor
    }

    #[test]
    fn update_runs_the_whole_frame_for_a_plain_actor() {
        let mut e = engine2();
        let actor = update_setup(&mut e);
        let player = e.mem.u32(PLAYER_CHARACTER);
        with_process(&mut e, Ptr::new(player));
        e.mem.set_f32(SCRATCH_FLOAT, 4.0);
        e.call_log = Some(vec![]);
        actor_update(&mut e, actor, 0.1);
        assert_eq!(
            args_of(&e, ACTOR_TIMED_UPDATE),
            vec![actor.addr(), 0.1f32.to_bits()]
        );
        assert!(was_called(&e, ACTOR_END_OF_UPDATE));
        let log = logged(&e);
        assert_eq!(
            log.iter().filter(|entry| entry.0 == EMPTY_FUNCTION).count(),
            3
        );
    }

    #[test]
    fn update_scales_the_animation_speed_by_the_setting() {
        let mut e = engine2();
        let actor = update_setup(&mut e);
        let player = e.mem.u32(PLAYER_CHARACTER);
        with_process(&mut e, Ptr::new(player));
        e.mem.set_f32(SCRATCH_FLOAT, 4.0);
        slot(&mut e, OTHER_TABLE, 8, eax(3));
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_ANIMATION, eax(0x4000));
        e.call_log = Some(vec![]);
        actor_update(&mut e, actor, 0.1);
        assert_eq!(
            args_of(&e, ANIMATION_SET_SPEED_SCALE),
            vec![0x4000, 0.25f32.to_bits()]
        );
    }

    #[test]
    fn update_drowns_the_actor_when_the_breath_is_spent() {
        let mut e = engine2();
        let actor = update_setup(&mut e);
        let player = e.mem.u32(PLAYER_CHARACTER);
        with_process(&mut e, Ptr::new(player));
        let controller = e.mem.alloc(0x600);
        double(&mut e, GET_CHAR_CONTROLLER, eax(controller));
        double(&mut e, IS_IN_MENU_MODE, eax(0));
        double(&mut e, ACTOR_GET_LIFE_STATE, eax(1));
        // The actor is in deep water.
        e.set_global(WATER_THRESHOLD_A, 1.0f64);
        e.set_global(WATER_THRESHOLD_B, 2.0f64);
        e.set_global(WATER_THRESHOLD_C, 3.0f64);
        slot(&mut e, ACTOR_TABLE, VSLOT_ACTOR_FLAG_0X360, eax(0));
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X300, st0(0.1));
        slot(&mut e, PROCESS_TABLE, PROCESS_VSLOT_0X2FC, Ret::default());
        double(&mut e, ACTOR_TEST_0X87F5C0, eax(0));
        let position = e.mem.alloc(16);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_POSITION_0X1F4, eax(position));
        double(&mut e, POSITION_VECTOR_OF, eax(position));
        double(&mut e, VECTOR_PLUS_VECTOR, eax(position));
        e.call_log = Some(vec![]);
        actor_update(&mut e, actor, 0.1);
        let target = slot_target(PROCESS_TABLE, PROCESS_VSLOT_0X2FC);
        let process = e.get(actor, Actor::pCurrentProcess).addr();
        assert_eq!(args_of(&e, target), vec![process, 0]);
        assert!(was_called(&e, ACTOR_TIMED_UPDATE));
    }

    #[test]
    fn character_update_clears_the_dead_flag_and_kicks_the_ragdoll() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let player = actor2(&mut e);
        set_player(&mut e, player);
        with_process(&mut e, actor);
        e.set(actor, Actor::bDeadFlag, true);
        double(&mut e, BLEND_OBJECT_OF, eax(0x777));
        double(&mut e, SAVE_FORM_BUFFER_GET_FORM, eax(0x600));
        double(&mut e, FORM_TYPE, eax(0x2b));
        double(&mut e, GET_CURRENT_PROCESS_TYPE, eax(1));
        slot(&mut e, ACTOR_TABLE, VSLOT_ACTOR_FORM_TEST_0X38C, eax(1));
        let rotated = e.mem.alloc(12);
        double(&mut e, MATRIX_TIMES_VECTOR, eax(rotated));
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_POSITION_0X1F4, eax(rotated));
        let cell = 0x5000;
        double(&mut e, GET_PARENT_CELL, eax(cell));
        double(&mut e, GET_FLAG_WORD, eax(0x800));
        set_player(&mut e, actor);
        e.call_log = Some(vec![]);
        fn_00886cb0(&mut e, actor, 0.1, 0);
        assert!(e.get(actor, Actor::bSwimming));
        assert_eq!(args_of(&e, ACTOR_SET_LIFE_STATE), vec![actor.addr(), 1]);
        let knock = args_of(&e, DO_KNOCK_DOWN);
        assert_eq!(knock[0], 0x777);
        assert_eq!(knock[2], 1);
        assert!(!e.get(actor, Actor::bDeadFlag));
    }

    #[test]
    fn character_update_scales_the_step_for_the_vats_target() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        let mover = e.mem.alloc(0x40);
        e.mem.set_u32(mover, OTHER_TABLE);
        e.set(actor, Actor::pActorMover, Ptr::new(mover));
        double(&mut e, GET_CURRENT_PROCESS_TYPE, eax(0));
        double(&mut e, SETTING_INT_VALUE, eax(4));
        e.mem.set_u32(VATS_TARGET_ACTOR, actor.addr());
        double(&mut e, UPDATE_STEP_SETTING, st0(0.5));
        double(&mut e, VATS_TARGET_UPDATE_MULT, st0(0.25));
        double(&mut e, GET_FLAG_WORD, eax(0));
        double(&mut e, GET_PARENT_CELL, eax(0x5000));
        double(&mut e, CELL_TEST_0X450FF0, eax(1));
        slot(&mut e, ACTOR_TABLE, VSLOT_0X250, Ret::default());
        e.call_log = Some(vec![]);
        fn_00886cb0(&mut e, actor, 0.0, 0);
        let call = args_of(&e, slot_target(ACTOR_TABLE, VSLOT_0X250));
        assert_eq!(call[0], actor.addr());
        assert_eq!(call[1], 0.125f32.to_bits());
        assert_eq!(call[3], 0);
    }

    #[test]
    fn character_update_resyncs_the_collision_of_a_turret() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        set_player(&mut e, actor);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        e.mem.set_u8(actor.addr() + 0x1b0, 1);
        double(&mut e, ACTOR_TURRET_BEHAVIOR, eax(1));
        double(&mut e, GET_FLAG_WORD, eax(0));
        double(&mut e, FIND_COLLISION_OBJECT, eax(0x9100));
        double(&mut e, FIND_ENTRY, eax(0x9200));
        double(&mut e, COLLISION_RIGID_BODY, eax(0x9300));
        double(&mut e, BODY_LIST_0X7D6BB0, eax(0));
        double(&mut e, WORLD_TEST_0XC8CCE0, eax(0));
        double(&mut e, BODY_FUNCTION_0X621480, eax(0x9400));
        e.mem.map(0x9000, 0x1000);
        e.mem.set_u32(0x9300, OTHER_TABLE);
        e.call_log = Some(vec![]);
        fn_00886cb0(&mut e, actor, 0.1, 0);
        assert_eq!(args_of(&e, SET_MOTION), vec![0x9100, 1, 1, 1, 1]);
        assert_eq!(args_of(&e, COLLISION_SYNCHRONIZE), vec![0x9100, 2]);
    }

    #[test]
    fn small_setters_store_their_values() {
        let mut e = engine2();
        fn_0088b020(&mut e, 5);
        assert_eq!(e.mem.u8(GLOBAL_FLAG_011F3E60), 5);
        let object = e.mem.alloc(0x600);
        fn_0088b050(&mut e, Ptr::new(object), 3);
        assert_eq!(e.mem.u8(object + 0x15d), 3);
        let flag = e.mem.alloc(4);
        e.mem.set_u8(flag, 1);
        e.mem.set_u32(object + 0x6c, 9);
        fn_0088b070(&mut e, Ptr::new(object), Ptr::new(flag));
        assert_eq!(e.mem.u8(object + 0x69), 1);
        assert_eq!(e.mem.u32(object + 0x6c), 9);
        e.mem.set_u8(flag, 0);
        fn_0088b070(&mut e, Ptr::new(object), Ptr::new(flag));
        assert_eq!(e.mem.u8(object + 0x69), 0);
        assert_eq!(e.mem.u32(object + 0x6c), 0);
        fn_0088b110(&mut e, Ptr::new(object), 2.5);
        assert_eq!(e.mem.f32(object + 0x560), 2.5);
        assert_eq!(fn_0088b130(&mut e, Ptr::new(object)), object + 0x570);
    }

    #[test]
    fn position_update_does_nothing_without_a_3d_object() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        e.call_log = Some(vec![]);
        actor_update_actor_3d_position(&mut e, actor);
        assert_eq!(logged(&e).len(), 1);
    }

    #[test]
    fn position_update_places_the_object_and_refreshes_the_lighting() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_HEADING_0X2BC, st0(0.75));
        let position = e.mem.alloc(16);
        double(&mut e, POSITION_VECTOR_OF, eax(position));
        e.mem.set_f32(position + 4, 3.0);
        let result = e.mem.alloc(0x40);
        double(&mut e, MULTIPLY_MATRIX_BY_RACE, eax(result));
        double(&mut e, ACTOR_LIGHTING_FORCE_TEST, eax(0));
        double(&mut e, SHADOW_SCENE_NODE, eax(0xaaaa));
        e.mem.set_u8(SKIP_SET_POSITION, 0);
        e.call_log = Some(vec![]);
        actor_update_actor_3d_position(&mut e, actor);
        let place = args_of(&e, SET_WORLD_POSITION);
        assert_eq!(place[0], 0x1000);
        assert_eq!(args_of(&e, MATRIX_MAKE_ROTATION)[1], 0.75f32.to_bits());
        assert_eq!(args_of(&e, SET_WORLD_ROTATION), vec![0x1000, result]);
        assert_eq!(args_of(&e, UPDATE_OBJECT_LIGHTING), vec![0xaaaa, 0x1000, 0]);
        assert!(e.get(actor, Actor::bLightingUpdatedNonMoving));
    }

    #[test]
    fn position_update_forces_the_lighting_when_asked() {
        let mut e = engine2();
        let actor = actor2(&mut e);
        let other = actor2(&mut e);
        set_player(&mut e, other);
        slot(&mut e, ACTOR_TABLE, VSLOT_GET_3D, eax(0x1000));
        let position = e.mem.alloc(16);
        double(&mut e, POSITION_VECTOR_OF, eax(position));
        let result = e.mem.alloc(0x40);
        double(&mut e, MULTIPLY_MATRIX_BY_RACE, eax(result));
        double(&mut e, ACTOR_LIGHTING_FORCE_TEST, eax(1));
        double(&mut e, SHADOW_SCENE_NODE, eax(0xaaaa));
        e.set(actor, Actor::bLightingUpdatedNonMoving, true);
        e.call_log = Some(vec![]);
        actor_update_actor_3d_position(&mut e, actor);
        assert_eq!(args_of(&e, UPDATE_OBJECT_LIGHTING), vec![0xaaaa, 0x1000, 1]);
        assert!(!e.get(actor, Actor::bLightingUpdatedNonMoving));
    }
}
