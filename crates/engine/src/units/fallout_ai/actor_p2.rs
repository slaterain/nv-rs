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
    /// `Actor` (Xbox PDB): only the fields this part uses, at their PC
    /// offsets (the Xbox ones less 0x10, since the PC `TESForm` is 0x10
    /// shorter). The size is the Xbox PDB's 0x1c4 less 0x10 and has not been
    /// checked against the PC allocation.
    pub struct Actor: 0x1b4 {
        /// `pCurrentProcess` (Xbox PDB, +0x78): `BaseProcess*`.
        0x68 pCurrentProcess: Ptr,
        /// `bInWater` (Xbox PDB, +0x15c).
        0x14c bInWater: bool,
        /// `bSwimming` (Xbox PDB, +0x15d).
        0x14d bSwimming: bool,
    }

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
}
