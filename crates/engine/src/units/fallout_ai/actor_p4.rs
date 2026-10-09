//! `fallout/ai/actor.cpp` (Xbox PDB source unit), part 4: its functions from `008a50d0` up to
//! (not including) `008b00c0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::actor`]; anything public there may be used here.
//!
//! This file holds the first 40 functions of the range (`008a50d0` to
//! `008a7760`); the next session continues at `008a7870`.
//!
//! Notes that apply to the whole file:
//!
//! - `Actor` offsets are the Xbox PDB ones minus `0x10` (the PC `TESForm` is
//!   `0x18` bytes), checked against the code that uses them. `+0x68` is
//!   `MobileObject::pCurrentProcess`, the actor's process; many functions here
//!   forward to a virtual method of that process. `008d8520` (named
//!   `MiddleHighProcess::GetSavedAcquireObject` in the engine map, a folded
//!   body) returns `this + 0x68`, and the code sometimes calls it and
//!   sometimes reads the field.
//! - Virtual calls use the PC byte offsets taken from the code. Slot names are
//!   given only where the Xbox PDB and the body agree.
//! - x87 note: `float` results are rounded to `f32` where the game stores a
//!   `float`; intermediate values are computed in `f64`.
//! - C++ exception-unwinding frames (`FS:[0]` chains) and the stack
//!   protector (`__security_check_cookie`) are not translated.

#[allow(unused_imports)]
use super::actor::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// `MiddleHighProcess::GetSavedAcquireObject` (engine map name of `008d8520`):
/// the word at `this + 0x68`, the actor's process.
const GET_PROCESS: u32 = 0x008d_8520;
/// The player character (`011dea3c` holds the pointer).
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
/// The calendar object (`Calendar`, passed as `this` by address).
const CALENDAR: u32 = 0x011d_e7b8;
/// `Calendar::GetHour` (Xbox PDB): the hour in `ST0`.
const CALENDAR_GET_HOUR: u32 = 0x0086_7da0;
/// `1.0` (`double`).
const ONE: u32 = 0x0101_2070;
/// `0.0` (`double`).
const ZERO_DOUBLE: u32 = 0x0101_2060;
/// `-1.0` (`float`).
const MINUS_ONE: u32 = 0x0101_2054;
/// `5000.0` (`float`): the radius of the reference searches.
const SEARCH_RADIUS: u32 = 0x0103_0020;
/// `0.01745329238474369` (`double`): one degree in radians.
const ONE_DEGREE: u32 = 0x0102_3128;
/// `pi` and `-pi` (`float`): the range `BSWrap` brings an angle into.
const PI: u32 = 0x0102_b3c8;
const MINUS_PI: u32 = 0x0106_b648;
/// A `float` the face-animation reset passes (`0.75`).
const FACE_FADE: u32 = 0x0101_6264;
/// Three words (a zero `NiPoint3`) used when there is no weapon position.
const ZERO_POINT: u32 = 0x011f_426c;
/// The `TESDataHandler` pointer (`011c3f2c`), `this` of the reference search.
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// The `ModelLoader` pointer (`011c3b3c`).
const MODEL_LOADER: u32 = 0x011c_3b3c;
/// The global that remembers the actor `008a5580` is speaking for.
const SPEAKING_ACTOR: u32 = 0x011d_f680;
/// The pointer to the name `"Weapon"` (`01188b78`) the node lookup takes.
const WEAPON_NODE_NAME: u32 = 0x0118_8b78;
/// A table of `0x24`-byte entries (`011977e0`), indexed by `005f2420`; the
/// first word is compared with 2 by `SetAnimAction`.
const ANIM_GROUP_TABLE: u32 = 0x0119_77e0;
/// A table of pointers to tables of words (`011a3ff0`).
const PACKAGE_TYPE_TABLE: u32 = 0x011a_3ff0;
/// The cache (`0126817c`) `008a5f20` looks a collision object up in.
const COLLISION_CACHE: u32 = 0x0126_817c;
/// The type descriptors `__RTDynamicCast` is given by `008a7760`.
const TYPE_TES_PACKAGE: u32 = 0x0118_46a0;
const TYPE_BACK_UP_PACKAGE: u32 = 0x011a_32e0;
/// Names of the two landing sounds.
const LAND_HEAVY_SOUND: u32 = 0x0108_4dac;
const LAND_LIGHT_SOUND: u32 = 0x0108_4d98;

/// `GameSetting` objects `Actor::Move` reads through `00403e20`.
const SETTING_FALL_THRESHOLD: u32 = 0x011c_ff94;
const SETTING_FALL_MULT: u32 = 0x011d_1410;
const SETTING_FALL_EXPONENT: u32 = 0x011d_0d20;
const SETTING_HEAVY_THRESHOLD: u32 = 0x011d_f6ec;
const SETTING_LIMB_DAMAGE_MULT: u32 = 0x011d_108c;
const SETTING_LIMB_DAMAGE_CHANCE: u32 = 0x011d_10c8;

/// Call targets used more than once.
const SETTING_VALUE_POINTER: u32 = 0x0040_3e20;
const SOUND_HANDLE_INIT: u32 = 0x0041_a250;
const SOUND_HANDLE_ASSIGN: u32 = 0x0041_8900;
const SOUND_HANDLE_DESTRUCT: u32 = 0x0048_3710;
const FORM_TYPE: u32 = 0x0041_ca90;
const FORM_ID: u32 = 0x0084_e3a0;
const GET_PARENT_CELL: u32 = 0x008d_6f30;
const GET_ANIMATION: u32 = 0x008b_70d0;
const GET_FACE_ANIMATION_DATA: u32 = 0x008a_dcb0;
const GET_CURRENT_PACKAGE: u32 = 0x0093_44a0;
const GET_CURRENT_WEAPON: u32 = 0x008a_1710;
const ENUM_REFERENCES_CLOSE_TO_POINT: u32 = 0x0046_f280;
const IS_ALIVE_BODY: u32 = 0x0049_3bb0;
const GET_SAVE_FORM: u32 = 0x007a_f430;
const HOUR_TO_PROCESS: u32 = 0x0069_3d50;
const STOP_MOVING: u32 = 0x008b_3ab0;
const OPERATOR_NEW: u32 = 0x0040_1000;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const INTERFACE_IN_DIALOG: u32 = 0x0070_50d0;
const SCRIPT_FLAG: u32 = 0x005b_7470;
const PACKAGE_CALLBACK_A: u32 = 0x0090_d570;
const PACKAGE_CALLBACK_B: u32 = 0x0090_d480;

layout! {
    /// `Actor` (Xbox PDB): the fields this file reads, at their PC offsets
    /// (Xbox PDB offset minus `0x10`). The PC size of the class is not known
    /// here; the declared size only has to cover the fields.
    pub struct Actor: 0x1b4 {
        /// `MobileObject::pCurrentProcess` (Xbox PDB `+0x78`): `BaseProcess*`.
        0x68 pCurrentProcess: Ptr,
        /// `MobileObject::bSpeakingDone` (Xbox PDB `+0x8c`).
        0x7c bSpeakingDone: u8,
        /// `MobileObject::bTalkingToPlayer` (Xbox PDB `+0x8d`).
        0x7d bTalkingToPlayer: u8,
        /// `MobileObject::bSoundFileDone` (Xbox PDB `+0x8f`).
        0x7f bSoundFileDone: u8,
        /// `MobileObject::bVoiceFileDone` (Xbox PDB `+0x90`).
        0x80 bVoiceFileDone: u8,
        /// `MobileObject::bUseEmotion` (Xbox PDB `+0x96`).
        0x86 bUseEmotion: u8,
        /// `Actor::pRagdollController` (Xbox PDB `+0xbc`).
        0xac pRagdollController: Ptr,
        /// `Actor::ePersuasionEmotion` (Xbox PDB `+0xc4`).
        0xb4 ePersuasionEmotion: u32,
        /// `Actor::bEVPBuffered` (Xbox PDB `+0x155`).
        0x145 bEVPBuffered: u8,
        /// `Actor::bResetAI` (Xbox PDB `+0x156`).
        0x146 bResetAI: u8,
        /// `Actor::pActorMover` (Xbox PDB `+0x1a0`).
        0x190 pActorMover: Ptr,
    }
}

/// `TESObjectREFR::pLoadedData` (Xbox PDB `+0x74`) at its PC offset.
const LOADED_DATA: u32 = 0x64;

/// The actor's process (`this + 0x68`).
fn process(e: &Engine, this: Ptr<Actor>) -> Ptr {
    e.get(this, Actor::pCurrentProcess)
}

/// `008d8520` as a call.
fn acquire_object(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    e.call(GET_PROCESS, &args![this]).ptr()
}

/// `Calendar::GetHour - 1.0`, as a `float`.
fn hour_before(e: &mut Engine) -> f32 {
    let hour = e.call(CALENDAR_GET_HOUR, &args![CALENDAR]).f32();
    let one: f64 = e.global(ONE);
    (hour as f64 - one) as f32
}

/// The value `00403e20` finds in a `GameSetting` object (a `float`).
fn setting_float(e: &mut Engine, setting: u32) -> f32 {
    let value = e.call(SETTING_VALUE_POINTER, &args![setting]).u32();
    e.mem.f32(value)
}

/// The value `0043d4d0` finds in a `GameSetting` object (an integer).
fn setting_int(e: &mut Engine, setting: u32) -> u32 {
    let value = e.call(0x0043_d4d0, &args![setting]).u32();
    e.mem.u32(value)
}

/// Copies `count` words.
fn copy_words(e: &mut Engine, to: u32, from: u32, count: u32) {
    for i in 0..count {
        let word = e.mem.u32(from + 4 * i);
        e.mem.set_u32(to + 4 * i, word);
    }
}

/// The reference search around the actor: `TESDataHandler::
/// EnumReferencesCloseToPoint` (`0046f280`) with the actor's cell, the
/// search radius (twice), two results of the actor's virtual method `+0x1f4`
/// and a callback that is given the actor.
fn enum_references_around(e: &mut Engine, this: Ptr<Actor>, callback: u32) {
    let radius: f32 = e.global(SEARCH_RADIUS);
    let handler: u32 = e.global(DATA_HANDLER);
    let first = e.vcall(this.addr(), 0x1f4, &args![]).u32();
    let second = e.vcall(this.addr(), 0x1f4, &args![]).u32();
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    e.call(
        ENUM_REFERENCES_CLOSE_TO_POINT,
        &args![handler, cell, second, radius, first, radius, callback, this],
    );
}

// Translated from 008a50d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a `float` in word 3 of `this` (`00560d30` is the address of word
/// `n` of the object).
pub fn fn_008a50d0(e: &mut Engine, this: Ptr, value: f32) {
    let word: Ptr = e.call(0x0056_0d30, &args![this, 3u32]).ptr();
    e.mem.set_f32(word.addr(), value);
}

// Translated from 008a50f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `008a5110`.
pub fn fn_008a50f0(e: &mut Engine, this: Ptr, source: Ptr) {
    fn_008a5110(e, this, source)
}

// Translated from 008a5110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the first three floats (x, y, z) of the 16-byte vector at `source`
/// into the 16-byte vector at `this`; the fourth float of `this` stays. The
/// code does it with SSE (`UNPCKHPS`, `SHUFPS`) on aligned loads.
pub fn fn_008a5110(e: &mut Engine, this: Ptr, source: Ptr) {
    copy_words(e, this.addr(), source.addr(), 3);
}

// Translated from 008a5170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 0 of the word at `this + 4` is set (`00621270` ands that word
/// with its argument, here `1`).
pub fn fn_008a5170(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0062_1270, &args![this, 1u32]).u32() != 0
}

// Translated from 008a5190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global word at `011c6254`.
pub fn fn_008a5190(e: &mut Engine) -> u32 {
    e.global(0x011c_6254)
}

// Translated from 008a51a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The result of the process's virtual method `+0x77c`, 0 without a process.
pub fn fn_008a51a0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    if acquire_object(e, this).is_null() {
        return 0;
    }
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x77c, &args![]).u32()
}

// Translated from 008a51f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the process's virtual method `+0x780` with `value`, if there is a
/// process.
pub fn fn_008a51f0(e: &mut Engine, this: Ptr<Actor>, value: u32) {
    if acquire_object(e, this).is_null() {
        return;
    }
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x780, &args![value]);
}

// Translated from 008a5230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the process's virtual method `+0x784`, if there is a process.
pub fn fn_008a5230(e: &mut Engine, this: Ptr<Actor>) {
    if acquire_object(e, this).is_null() {
        return;
    }
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x784, &args![]);
}

// Translated from 008a5270 (decompiled, FalloutNV.exe 1.4.0.525)
/// The result of the process's virtual method `+0x770`, 0 without a process.
pub fn fn_008a5270(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    if acquire_object(e, this).is_null() {
        return 0;
    }
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x770, &args![]).u32()
}

// Translated from 008a52c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the process's virtual method `+0x774` with `value`, if there is a
/// process.
pub fn fn_008a52c0(e: &mut Engine, this: Ptr<Actor>, value: u32) {
    if acquire_object(e, this).is_null() {
        return;
    }
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x774, &args![value]);
}

// Translated from 008a5300 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the process's virtual method `+0x778`, if there is a process.
pub fn fn_008a5300(e: &mut Engine, this: Ptr<Actor>) {
    if acquire_object(e, this).is_null() {
        return;
    }
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x778, &args![]);
}

// Translated from 008a5340 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetWeaponPosition` (Xbox PDB): writes into `result` the point
/// `(0, reach, 0)` transformed by the world transform of the actor's weapon
/// node, where `reach` is `CombatFormulas::CalcWeaponReach` of the current
/// weapon (0 without one). When `keep_world` is zero the transform is first
/// multiplied by the inverse of the actor's 3D root transform. `result` is
/// set to the zero point `011f426c` when the actor has no 3D or no weapon
/// node. Returns `result`.
///
/// The weapon node is the process's method `+0x190` of the current biped
/// (`+0x1e8`), or, without a process, the `"Weapon"` node found by
/// `004aae30` under the 3D root (the weapon query before it already reads the
/// process's vtable, so that branch is only reached with a process in the
/// game). Transforms are 13 words (`NiTransform`).
pub fn actor_get_weapon_position(
    e: &mut Engine,
    this: Ptr<Actor>,
    result: Ptr,
    keep_world: u8,
) -> Ptr {
    let root = e.vcall(this.addr(), 0x1d0, &args![]).u32();
    if root == 0 {
        copy_words(e, result.addr(), ZERO_POINT, 3);
        return result;
    }
    let proc = process(e, this);
    let weapon = e.vcall(proc.addr(), 0x148, &args![]).u32();
    let reach = if weapon != 0 {
        let form = e.call(0x0044_ddc0, &args![weapon]).u32();
        let base = e.call(0x0064_47f0, &args![form]).f32();
        e.call(0x0064_63e0, &args![base]).f32()
    } else {
        0.0f32
    };
    let proc = process(e, this);
    let node = if !proc.is_null() {
        let biped = e.vcall(this.addr(), 0x1e8, &args![]).u32();
        e.vcall(proc.addr(), 0x190, &args![biped]).u32()
    } else {
        let name: u32 = e.global(WEAPON_NODE_NAME);
        e.call(0x004a_ae30, &args![root, name]).u32()
    };
    if node == 0 {
        copy_words(e, result.addr(), ZERO_POINT, 3);
        return result;
    }
    // One block holds the working values:
    // 0x00 transform, 0x34 inverse, 0x68 product, 0x9c point, 0xa8 output.
    e.with_stack(0xb8, |e, block| {
        let transform = block.addr();
        let inverse = block.addr() + 0x34;
        let product = block.addr() + 0x68;
        let point = block.addr() + 0x9c;
        let output = block.addr() + 0xa8;
        let node_transform = e.call(0x0046_1130, &args![node]).u32();
        copy_words(e, transform, node_transform, 13);
        if keep_world == 0 {
            e.call(0x0047_6a80, &args![inverse]);
            let root_transform = e.call(0x0046_1130, &args![root]).u32();
            e.call(0x004b_4880, &args![root_transform, inverse]);
            let combined = e
                .call(0x0062_c250, &args![inverse, product, transform])
                .u32();
            copy_words(e, transform, combined, 13);
        }
        let made = e
            .call(0x0041_6870, &args![point, 0.0f32, reach, 0.0f32])
            .u32();
        e.call(0x0052_4c40, &args![transform, output, made]);
        copy_words(e, result.addr(), output, 3);
    });
    result
}

// Translated from 008a5530 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the actor's face animation data's virtual method `+0xb4` with
/// `(fade, 0, 1, 1, 1, 0)`, where `fade` is the `float` at `01016264`, if the
/// actor has face animation data (`Actor::GetFaceAnimationData`).
pub fn fn_008a5530(e: &mut Engine, this: Ptr<Actor>) {
    let face = e.call(GET_FACE_ANIMATION_DATA, &args![this]).u32();
    if face == 0 {
        return;
    }
    let fade: f32 = e.global(FACE_FADE);
    e.vcall(face, 0xb4, &args![fade, 0u32, 1u32, 1u32, 1u32, 0u32]);
}

/// The face animation data's mood call `+0x11c` shared by the branches of
/// `008a5580`: when the process does not say it is busy (method `+0x2e8`)
/// and the interface is in a dialogue, asks the actor's `ActorValueOwner`
/// subobject (`this + 0xa4`, method `+8`, `GetActorValue` in the Xbox PDB)
/// for value 4 and hands the answer to the face data.
fn face_mood_from_dialogue(e: &mut Engine, this: Ptr<Actor>, face: u32) {
    let busy = if !acquire_object(e, this).is_null() {
        let proc = acquire_object(e, this);
        e.vcall(proc.addr(), 0x2e8, &args![]).u32() != 0
    } else {
        false
    };
    if busy || !e.call(INTERFACE_IN_DIALOG, &args![]).bool() {
        return;
    }
    let owner = this.addr() + 0xa4;
    let value = e.vcall(owner, 8, &args![4u32, 0u32]).u32();
    e.vcall(face, 0x11c, &args![value]);
}

/// The face animation data's reset call `+0xb4` with `(0.0, 1, 0, 0, 0, 0)`.
fn face_reset(e: &mut Engine, face: u32) {
    e.vcall(face, 0xb4, &args![0.0f32, 1u32, 0u32, 0u32, 0u32, 0u32]);
}

// Translated from 008a5580 (decompiled, FalloutNV.exe 1.4.0.525)
/// The actor's per-frame speech and movement update (the map has no name for
/// it). `delta` is the frame time, `speaker` the reference the actor speaks
/// for (it may be null) and `notify` says whether completion callbacks are
/// registered for the actor.
///
/// - Does nothing when the actor has no animation (virtual method `+0x1e4`).
/// - With a `speaker`, a process and `bSpeakingDone` set: records the actor in
///   `011df680`, sets `bTalkingToPlayer`, and either queues the speaker
///   form's sound (`00933150` and a completion callback, clearing
///   `bSoundFileDone` and `bSpeakingDone`) or, when the speech file is done,
///   starts the speech (virtual method `+0x284`, 13 words), registers the
///   completion callbacks and passes the speech length to the process
///   (`+0x318`).
/// - For an idle actor with a mover that is not rotating, requests a rotation
///   to the player's position (`ActorMover::RequestRotateActor`) when the
///   wrapped angle between the direction of the two positions and the
///   actor's rotation is more than one degree; then runs the move update
///   `+0x348` with the mover's `009c9900` flag raised around it.
/// - Updates the face animation data from the emotion state
///   (`ePersuasionEmotion`: 1 and 5 hand mood 6 and 5 to the face data; 8
///   either asks the actor value owner for the mood or resets the face,
///   depending on `005b7470`, the process and the interface).
/// - Keeps the special-idle package (type `0x1c`) pointing at the actor
///   while it speaks and restarts the idle (`+0x44` of the process) once the
///   idle is done playing.
pub fn fn_008a5580(e: &mut Engine, this: Ptr<Actor>, delta: f32, speaker: Ptr, notify: u8) {
    let animation = e.vcall(this.addr(), 0x1e4, &args![]).u32();
    let mut special_idle = Ptr::NULL;
    let proc = acquire_object(e, this);
    let package: Ptr = e.vcall(proc.addr(), 0x22c, &args![]).ptr();
    if !package.is_null() && e.call(FORM_TYPE, &args![package]).i32() == 0x1c {
        special_idle = package;
    }
    if animation == 0 {
        return;
    }
    // Two sound handles (12 bytes each) and a point.
    let scratch = e.mem.alloc(0x40);
    let handle = scratch;
    let queued = scratch + 0x10;
    let sound_state = scratch + 0x20;
    let mut speech_claimed = false;
    if !speaker.is_null()
        && !acquire_object(e, this).is_null()
        && e.get(this, Actor::bSpeakingDone) != 0
    {
        e.call(SOUND_HANDLE_INIT, &args![handle]);
        speech_claimed = true;
        e.set_global(SPEAKING_ACTOR, this.addr());
        e.set(this, Actor::bTalkingToPlayer, 1);
        let mut speaking = true;
        if !e.call(GET_SAVE_FORM, &args![speaker]).ptr::<()>().is_null() {
            let form = e.call(GET_SAVE_FORM, &args![speaker]).u32();
            let form_id = e.call(FORM_ID, &args![form]).u32();
            e.call(
                0x0093_3150,
                &args![this, queued, form_id, 0u32, 0x102u32, 1u32],
            );
            let manager = e.call(0x00ad_9060, &args![]).u32();
            let own_id = e.call(FORM_ID, &args![this]).u32();
            let sound = e.call(0x0055_9450, &args![queued]).u32();
            e.call(0x00ad_bfd0, &args![manager, sound, 0x0093_5cc0u32, own_id]);
            e.set(this, Actor::bSoundFileDone, 0);
            speaking = false;
            e.set(this, Actor::bSpeakingDone, 0);
            e.call(SOUND_HANDLE_DESTRUCT, &args![queued]);
        }
        let flag = e.call(0x0054_3c30, &args![speaker]).u8();
        fn_008a5cf0(e, this, flag);
        if speaking && e.get(this, Actor::bSoundFileDone) != 0 {
            let player: u32 = e.global(PLAYER_CHARACTER);
            let worldspace = e.call(0x0044_1110, &args![speaker]).u32();
            let owner = e.call(0x0096_11e0, &args![speaker]).u32();
            let owner_form = e.call(0x0068_15c0, &args![speaker]).u32();
            let owner_form = e.call(0x0040_48e0, &args![owner_form]).u32();
            let speaker_id = e.call(FORM_ID, &args![speaker]).u32();
            let base = e.call(0x0044_ddc0, &args![speaker]).u32();
            let name_field = e.call(0x0046_0140, &args![speaker]).u32();
            let name = e.call(0x0055_9450, &args![name_field]).u32();
            let length = e
                .vcall(
                    this.addr(),
                    0x284,
                    &args![
                        name, handle, base, speaker_id, owner_form, owner, worldspace, player,
                        1u32, 1u32, 0u32, 1u32, 1u32
                    ],
                )
                .f32();
            let manager = e.call(0x00ad_9060, &args![]).u32();
            let target = if notify != 0 { this } else { Ptr::NULL };
            if !target.is_null() {
                let id = e.call(FORM_ID, &args![target]).u32();
                let sound = e.call(0x0055_9450, &args![handle]).u32();
                e.call(0x00ad_c050, &args![manager, sound, 0x0093_5f00u32, id]);
                let id = e.call(FORM_ID, &args![target]).u32();
                let sound = e.call(0x0055_9450, &args![handle]).u32();
                e.call(0x00ad_bfd0, &args![manager, sound, 0x008b_c590u32, id]);
            }
            e.set(this, Actor::bVoiceFileDone, 0);
            let proc = acquire_object(e, this);
            e.vcall(proc.addr(), 0x318, &args![length]);
        }
        e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
    }

    let ragdoll = e.get(this, Actor::pRagdollController);
    if !ragdoll.is_null() {
        e.call(0x00c7_48b0, &args![ragdoll]);
    }

    let mover = e.get(this, Actor::pActorMover);
    if !mover.is_null()
        && e.vcall(this.addr(), 0x214, &args![]).u32() == 0
        && !e.call(IS_ALIVE_BODY, &args![this]).bool()
    {
        if !e.call(0x009d_db50, &args![mover]).bool() {
            // The direction from the player to the actor, as an angle.
            let own_position = e.vcall(this.addr(), 0x1f4, &args![]).u32();
            let player: u32 = e.global(PLAYER_CHARACTER);
            let player_position = e.vcall(player, 0x1f4, &args![]).u32();
            let offset = scratch + 0x30;
            e.call(0x0043_9ef0, &args![player_position, offset, own_position]);
            let direction = e.call(0x004b_13c0, &args![offset]).f32();
            let rotation = e.call(0x0043_0830, &args![this]).u32();
            let angle = (direction as f64 - e.mem.f32(rotation + 8) as f64) as f32;
            // The wrapped angle is kept in the last word of the scratch block.
            let wrapped = scratch + 0x3c;
            e.mem.set_f32(wrapped, angle);
            let low: f32 = e.global(MINUS_PI);
            let high: f32 = e.global(PI);
            e.call(0x004e_44f0, &args![wrapped, low, high]);
            let wrapped_angle = e.mem.f32(wrapped);
            let size = e.call(0x0040_8840, &args![wrapped_angle]).f64();
            let one_degree: f64 = e.global(ONE_DEGREE);
            if size > one_degree {
                let player_position = e.vcall(player, 0x1f4, &args![]).u32();
                let x = e.mem.u32(player_position);
                let y = e.mem.u32(player_position + 4);
                let z = e.mem.u32(player_position + 8);
                e.call(0x009d_ce20, &args![mover, x, y, z, 0u32]);
            }
        }
        e.call(0x009c_9900, &args![mover, 1u32]);
        e.vcall(this.addr(), 0x348, &args![delta, 0u32]);
        e.call(0x009c_9900, &args![mover, 0u32]);
    }

    e.vcall(this.addr(), 0x140, &args![]);

    if !acquire_object(e, this).is_null() {
        let proc = acquire_object(e, this);
        if e.call(0x0045_cd60, &args![proc]).u32() == 0 {
            let proc = acquire_object(e, this);
            let bone = e.call(0x0089_d620, &args![proc]).u32();
            if bone != 0 && e.call(0x005f_36f0, &args![bone]).u32() != 0 {
                e.call(0x00c5_2960, &args![bone, 0u32]);
            }
        }
    }

    if !acquire_object(e, this).is_null() {
        let proc = acquire_object(e, this);
        e.vcall(proc.addr(), 0x48c, &args![sound_state, 0u32]);
        let face = e.call(GET_FACE_ANIMATION_DATA, &args![this]).u32();
        let playing = e.call(0x00ad_8930, &args![sound_state]).bool();
        if !playing {
            if !special_idle.is_null() && e.mem.u32(special_idle.addr() + 0x94) != 0 {
                e.mem.set_u32(special_idle.addr() + 0x94, 0);
            }
            if face != 0 {
                let mood = e.get(this, Actor::ePersuasionEmotion);
                if mood == 8 {
                    if e.call(SCRIPT_FLAG, &args![]).bool() {
                        face_mood_from_dialogue(e, this, face);
                    } else {
                        face_reset(e, face);
                    }
                } else if mood == 1 {
                    e.vcall(face, 0x11c, &args![6u32, 0u32]);
                } else if mood == 5 {
                    e.vcall(face, 0x11c, &args![5u32, 0u32]);
                }
            }
        } else if face != 0 && e.get(this, Actor::ePersuasionEmotion) == 8 {
            let face = e.call(GET_FACE_ANIMATION_DATA, &args![this]).u32();
            if face != 0 {
                if e.call(SCRIPT_FLAG, &args![]).bool() {
                    face_mood_from_dialogue(e, this, face);
                } else {
                    face_reset(e, face);
                }
            }
        }
        e.call(SOUND_HANDLE_DESTRUCT, &args![sound_state]);
    }

    if speech_claimed && !special_idle.is_null() {
        e.mem.set_u32(special_idle.addr() + 0x94, this.addr());
    }
    if !special_idle.is_null() && e.call(0x0049_85f0, &args![animation]).bool() {
        let proc = acquire_object(e, this);
        if e.vcall(proc.addr(), 0x718, &args![]).u32() == 0 {
            let proc = acquire_object(e, this);
            e.vcall(
                proc.addr(),
                0x44,
                &args![this, 0u32, 2u32, 1u32, 0u32, 1u32],
            );
        }
    }
    e.mem.free(scratch);
}

// Translated from 008a5cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte in `bUseEmotion` (`+0x86`).
pub fn fn_008a5cf0(e: &mut Engine, this: Ptr<Actor>, value: u8) {
    e.set(this, Actor::bUseEmotion, value);
}

// Translated from 008a5d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// A per-frame update of the actor with frame time `delta`: relaxes the bone
/// level of detail of an idle process, runs the virtual methods `+0x268`,
/// `+0x350`, `+0x2f8` and `+0x178`, and updates the animation's queued scene
/// graph and the cell.
pub fn fn_008a5d10(e: &mut Engine, this: Ptr<Actor>, delta: f32) {
    let proc = acquire_object(e, this);
    let package = e.vcall(proc.addr(), 0x22c, &args![]).u32();
    if package != 0 {
        // The result (a package of type 0x1c) is not used afterwards.
        e.call(FORM_TYPE, &args![package]);
    }
    let animation = e.vcall(this.addr(), 0x1e4, &args![]).u32();
    if !proc.is_null() && e.call(0x0045_cd60, &args![proc]).u32() == 0 {
        let bone = e.call(0x0089_d620, &args![proc]).u32();
        if bone != 0 && e.call(0x005f_36f0, &args![bone]).u32() != 0 {
            e.call(0x00c5_2960, &args![bone, 0u32]);
        }
    }
    e.vcall(this.addr(), 0x268, &args![delta]);
    e.vcall(this.addr(), 0x350, &args![]);
    if !e.get(this, Actor::pRagdollController).is_null() {
        e.call(0x0088_8970, &args![this, 0u32]);
    }
    if animation != 0 {
        e.call(0x0049_3930, &args![animation]);
    }
    e.vcall(this.addr(), 0x2f8, &args![delta]);
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    if cell != 0 {
        e.call(0x0054_baf0, &args![cell]);
    }
    e.vcall(this.addr(), 0x178, &args![]);
}

// Translated from 008a5e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetAlert` (Xbox PDB): the process's virtual method `+0x320` with
/// `alert`, if there is a process.
pub fn actor_set_alert(e: &mut Engine, this: Ptr<Actor>, alert: u8) {
    let proc = process(e, this);
    if proc.is_null() {
        return;
    }
    e.vcall(proc.addr(), 0x320, &args![alert]);
}

// Translated from 008a5e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetAlert` (Xbox PDB): the process's virtual method `+0x31c`,
/// false without a process.
pub fn actor_get_alert(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let proc = process(e, this);
    if proc.is_null() {
        return false;
    }
    e.vcall(proc.addr(), 0x31c, &args![]).bool()
}

// Translated from 008a5eb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetHavokWeapon` (Xbox PDB): with a current weapon
/// (`Actor::GetCurrentWeapon`), calls `SetHavokWeapon_ov2` with 0 when the
/// ragdoll controller exists and `00552490` says so, else 1; returns its
/// result, 0 without a weapon.
pub fn actor_set_havok_weapon(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    if e.call(GET_CURRENT_WEAPON, &args![this]).u32() == 0 {
        return 0;
    }
    let ragdoll = e.get(this, Actor::pRagdollController);
    let flag: u8 = if !ragdoll.is_null() && e.call(0x0055_2490, &args![ragdoll]).bool() {
        0
    } else {
        1
    };
    actor_set_havok_weapon_ov2(e, this, flag)
}

// Translated from 008a5f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetHavokWeapon_ov2` (Xbox PDB): makes the physics weapon object of
/// the actor's current weapon, or updates it. `mode` is the byte the caller
/// derived (see [`actor_set_havok_weapon`]).
///
/// While the special state `008c7aa0` holds, only the task queue is told
/// (`TaskQueueInterface::QueueActorSetHavokWeapon`). Otherwise: takes the
/// actor's 3D (`00950bb0` for the player, `0043fcd0` for others), finds the
/// weapon node (`004ade00`), looks for an existing collision object
/// (`004b5260`, `00653270` in the cache `0126817c`), and either updates its
/// constraints (`00c89050`) or creates the weapon object (`00c89540`);
/// finally updates the child rigid bodies (`00c891c0`) when the actor is in
/// a state that needs it. Always returns 0.
pub fn actor_set_havok_weapon_ov2(e: &mut Engine, this: Ptr<Actor>, mode: u8) -> u32 {
    if e.call(0x008c_7aa0, &args![]).bool() {
        let queue = e.call(0x0045_37b0, &args![]).u32();
        e.call(0x0087_b6b0, &args![queue, this, mode]);
        return 0;
    }
    let player: u32 = e.global(PLAYER_CHARACTER);
    let root = if this.addr() == player {
        e.call(0x0095_0bb0, &args![player, 0u32]).u32()
    } else {
        e.call(0x0043_fcd0, &args![this]).u32()
    };
    if root == 0 || e.call(GET_CURRENT_WEAPON, &args![this]).u32() == 0 {
        return 0;
    }
    let name = e.call(0x0089_1350, &args![]).u32();
    let node = e.call(0x004a_de00, &args![root, name]).u32();
    if node == 0 {
        return 0;
    }
    let in_combat_use = (mode == 0
        && e.vcall(this.addr(), 0x22c, &args![0u32]).u8() == 0
        && e.vcall(this.addr(), 0x230, &args![]).u8() == 0)
        || {
            let ragdoll = e.get(this, Actor::pRagdollController);
            !ragdoll.is_null() && e.call(0x0089_d690, &args![ragdoll]).u8() != 0
        };
    let keep: u8 = in_combat_use as u8;
    let link = e.mem.alloc(4);
    e.call(0x0093_1ed0, &args![this, link]);
    let link_value = e.mem.u32(link);
    let collision = e.call(0x004b_5260, &args![node]).u32();
    let found = if collision != 0 {
        e.call(0x0065_3270, &args![COLLISION_CACHE, collision])
            .u32()
    } else {
        0
    };
    let mut weapon_object = found;
    if weapon_object != 0 {
        e.call(
            0x00c8_9050,
            &args![weapon_object, mode, link_value, root, keep],
        );
    } else {
        weapon_object = e
            .call(0x00c8_9540, &args![link_value, root, node, keep])
            .u32();
        if weapon_object != 0 && mode == 0 {
            e.call(
                0x00c8_9050,
                &args![weapon_object, mode, link_value, root, keep],
            );
        }
    }
    e.mem.free(link);
    let needs_children = e.call(0x0043_7bd0, &args![this]).bool()
        || e.vcall(this.addr(), 0x22c, &args![0u32]).u8() != 0
        || e.vcall(this.addr(), 0x2e8, &args![]).u8() != 0
        || e.vcall(this.addr(), 0x230, &args![]).u8() != 0;
    if weapon_object != 0 && needs_children {
        e.call(0x00c8_91c0, &args![weapon_object]);
    }
    0
}

// Translated from 008a6170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the actor's current package (`MobileObject::GetCurrentPackage`)
/// has type `0x18`.
pub fn fn_008a6170(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let package = e.call(GET_CURRENT_PACKAGE, &args![this]).u32();
    package != 0 && e.call(FORM_TYPE, &args![package]).i32() == 0x18
}

// Translated from 008a61b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsAlarmed` (Xbox PDB): whether the process's current package
/// (virtual method `+0x22c`) has type `0x15`.
pub fn actor_is_alarmed(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let proc = process(e, this);
    let package = if proc.is_null() {
        0
    } else {
        let proc = process(e, this);
        e.vcall(proc.addr(), 0x22c, &args![]).u32()
    };
    package != 0 && e.call(FORM_TYPE, &args![package]).i32() == 0x15
}

// Translated from 008a6210 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the current package qualifies: it has a valid index
/// (`009611e0`, not `-1`) and either `fn_008a6290` holds for the package or
/// the entry of table `011a3ff0` for that index and the process's method
/// `+0x280` is `0xd`.
pub fn fn_008a6210(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let package = e.call(GET_CURRENT_PACKAGE, &args![this]).u32();
    if package == 0 || e.call(0x0096_11e0, &args![package]).i32() == -1 {
        return false;
    }
    let table_index = e.call(0x0096_11e0, &args![package]).u32();
    let proc = acquire_object(e, this);
    let key = e.vcall(proc.addr(), 0x280, &args![]).u32();
    if fn_008a6290(e, Ptr::new(package)) {
        return true;
    }
    let table = e.mem.u32(PACKAGE_TYPE_TABLE + table_index.wrapping_mul(4));
    e.mem.u32(table.wrapping_add(key.wrapping_mul(4))) == 0xd
}

// Translated from 008a6290 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x10000` of the word at `this + 0x1c` is set.
pub fn fn_008a6290(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x1c) & 0x10000 != 0
}

// Translated from 008a62b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::Move` (Xbox PDB): moves the actor with `MobileObject::Move`
/// (`0092f260`, arguments `delta`, `arg_b`, `flags`) and, when that returned a
/// character controller that is on the ground, applies fall damage.
///
/// - An actor in dialogue with the player does not move unless the low six
///   bits of `flags` are set; one without loaded data does not move.
/// - The fall distance of the controller (`bhkCharacterController::
///   GetFallDistance`) beyond the setting at `011cff94` gives the damage
///   `pow(distance - threshold, exponent) * multiplier` (settings
///   `011d0d20`, `011d1410`), which is 0 for a base form `005f0c40` flags.
/// - Positive damage calls the virtual method `+0x338` when the actor is
///   not (`+0x360`) in the loaded state and the world space check allows
///   it, plays a landing sound (heavy above the setting `011df6ec`), and with
///   the chance of settings `011d10c8` damages the two leg values (`+0x3ac`
///   with `0x1d` and `0x1e`); then `Actor::TriggerPain(1, 1)`.
///
/// Returns what `MobileObject::Move` returned (0 when nothing moved).
pub fn actor_move(e: &mut Engine, this: Ptr<Actor>, delta: f32, arg_b: u32, flags: u32) -> Ptr {
    if e.call(0x0093_3840, &args![this]).bool() && flags & 0x3f == 0 {
        return Ptr::NULL;
    }
    if !process(e, this).is_null() {
        // The character controller; the result is not used afterwards.
        e.call(0x0093_06d0, &args![this]);
    }
    if e.mem.u32(this.addr() + LOADED_DATA) == 0 {
        return Ptr::NULL;
    }
    let moved: Ptr = e.call(0x0092_f260, &args![this, delta, arg_b, flags]).ptr();
    if moved.is_null() || !e.call(0x0087_cee0, &args![moved]).bool() {
        return moved;
    }
    let fall = e.call(0x00c7_0550, &args![moved]).f32();
    let limit = setting_float(e, SETTING_FALL_THRESHOLD);
    // `fall > limit`, false when either is not a number.
    if fall.partial_cmp(&limit) != Some(std::cmp::Ordering::Greater) {
        return moved;
    }
    let threshold = setting_float(e, SETTING_FALL_THRESHOLD);
    let excess = (fall as f64 - threshold as f64) as f32;
    let multiplier_at = e
        .call(SETTING_VALUE_POINTER, &args![SETTING_FALL_MULT])
        .u32();
    let exponent = setting_float(e, SETTING_FALL_EXPONENT);
    let scaled = e.call(0x004d_d130, &args![excess, exponent]).f64();
    let mut damage = (scaled * e.mem.f32(multiplier_at) as f64) as f32;
    let form = e.call(0x0041_81e0, &args![this]).u32();
    if form != 0 && e.call(0x005f_0c40, &args![form]).bool() {
        damage = 0.0;
    }
    let zero: f64 = e.global(ZERO_DOUBLE);
    if (damage as f64).partial_cmp(&zero) != Some(std::cmp::Ordering::Greater) {
        return moved;
    }

    let mut hurt = true;
    if e.vcall(this.addr(), 0x360, &args![]).u8() == 0
        && e.vcall(this.addr(), 0x218, &args![]).u8() != 0
        && e.call(0x0057_5d70, &args![this]).u32() != 0
    {
        let world = e.call(0x0057_5d70, &args![this]).u32();
        if !e.call(0x0058_6320, &args![world]).bool() {
            hurt = false;
        }
    }
    if hurt
        && (e.vcall(this.addr(), 0x360, &args![]).u8() != 0
            || !e.call(0x0056_6950, &args![this]).bool())
    {
        e.vcall(this.addr(), 0x338, &args![damage, 0.0f32, 0u32]);
    }

    if e.vcall(this.addr(), 0x360, &args![]).u8() != 0 {
        let sound = e.mem.alloc(0x30);
        let named = sound + 0x10;
        e.call(SOUND_HANDLE_INIT, &args![sound]);
        let heavy = damage > setting_float(e, SETTING_HEAVY_THRESHOLD);
        let name = if heavy {
            LAND_HEAVY_SOUND
        } else {
            LAND_LIGHT_SOUND
        };
        let audio = e.call(0x0045_3a70, &args![]).u32();
        let found = e
            .call(0x00ad_7550, &args![audio, named, name, 0x101u32])
            .u32();
        e.call(SOUND_HANDLE_ASSIGN, &args![sound, found]);
        e.call(SOUND_HANDLE_DESTRUCT, &args![named]);
        e.call(0x00ad_8830, &args![sound, 0u32]);
        fn_008a6630(e, moved);
        let limb_damage =
            (damage as f64 * setting_float(e, SETTING_LIMB_DAMAGE_MULT) as f64) as f32;
        for value in [0x1du32, 0x1e] {
            let roll = e.call(0x0048_7f50, &args![]).u32() % 100;
            if roll < setting_int(e, SETTING_LIMB_DAMAGE_CHANCE) {
                e.vcall(this.addr(), 0x3ac, &args![value, -limb_damage, 0u32]);
            }
        }
        e.call(SOUND_HANDLE_DESTRUCT, &args![sound]);
        e.mem.free(sound);
    }
    e.call(0x008a_7d50, &args![this, 1u32, 1u32]);
    moved
}

// Translated from 008a6630 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `006296b0` on the sub-object at `this + 0x410` with `0x80` (a flag
/// of the character controller).
pub fn fn_008a6630(e: &mut Engine, this: Ptr) {
    e.call(0x0062_96b0, &args![this.addr() + 0x410, 0x80u32]);
}

// Translated from 008a6650 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsFleeing` (Xbox PDB): whether the actor's current package is a
/// flee package (type `0x16`, then for the player the process's method
/// `+0x234` runs and the answer is false), or, when `skip_checks` is 0, a
/// package of type `0xa`, or of type `0x13` while the process's method
/// `+0x280` is 1, or the actor's combat controller (virtual method `+0x428`)
/// is fleeing (`CombatController::IsFleeing`).
pub fn actor_is_fleeing(e: &mut Engine, this: Ptr<Actor>, skip_checks: u8) -> bool {
    let package = e.call(GET_CURRENT_PACKAGE, &args![this]).u32();
    if package == 0 {
        return false;
    }
    if e.call(FORM_TYPE, &args![package]).i32() == 0x16 {
        let player: u32 = e.global(PLAYER_CHARACTER);
        if player == this.addr() {
            let proc = e.call(GET_PROCESS, &args![player]).u32();
            e.vcall(proc, 0x234, &args![]);
            return false;
        }
        return true;
    }
    if skip_checks != 0 {
        return false;
    }
    if e.call(FORM_TYPE, &args![package]).i32() == 0x16 {
        return true;
    }
    if e.call(FORM_TYPE, &args![package]).i32() == 0xa {
        return true;
    }
    if e.call(FORM_TYPE, &args![package]).i32() == 0x13 {
        let proc = process(e, this);
        if e.vcall(proc.addr(), 0x280, &args![]).i32() == 1 {
            return true;
        }
    }
    let controller = e.vcall(this.addr(), 0x428, &args![]).u32();
    if controller == 0 {
        return false;
    }
    e.call(0x0098_1990, &args![controller]).bool()
}

// Translated from 008a6730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::RemoveFleeTarget` (Xbox PDB): when the current package is a flee
/// package (type `0x16`) that `fn_008a67d0` does not yet flag, removes
/// `target` from it (`009f1350`); if that makes `fn_008a67d0` true the actor
/// stops: its virtual method `+0x434` when `00493bb0` holds, else
/// `Actor::EndInterruptPackage(0)`.
pub fn actor_remove_flee_target(e: &mut Engine, this: Ptr<Actor>, target: u32) {
    let mut flee = Ptr::NULL;
    let package = e.call(GET_CURRENT_PACKAGE, &args![this]).u32();
    if package != 0 && e.call(FORM_TYPE, &args![package]).i32() == 0x16 {
        flee = e.call(GET_CURRENT_PACKAGE, &args![this]).ptr();
    }
    if flee.is_null() || fn_008a67d0(e, flee) {
        return;
    }
    e.call(0x009f_1350, &args![flee, target]);
    if !fn_008a67d0(e, flee) {
        return;
    }
    if e.call(IS_ALIVE_BODY, &args![this]).bool() {
        e.vcall(this.addr(), 0x434, &args![0u32]);
    } else {
        e.call(0x0088_1680, &args![this, 0u32]);
    }
}

// Translated from 008a67d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `008256d0` on the sub-object at `this + 0x98` (here a flee package's
/// list of targets).
pub fn fn_008a67d0(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0082_56d0, &args![this.addr() + 0x98]).bool()
}

// Translated from 008a67f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsTalking` (Xbox PDB): `bTalkingToPlayer`, else the process's
/// virtual method `+0x4b8` with the actor (false without a process).
pub fn actor_is_talking(e: &mut Engine, this: Ptr<Actor>) -> bool {
    if e.get(this, Actor::bTalkingToPlayer) != 0 {
        return true;
    }
    let proc = process(e, this);
    if proc.is_null() {
        return false;
    }
    let proc = process(e, this);
    e.vcall(proc.addr(), 0x4b8, &args![this]).bool()
}

// Translated from 008a6840 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets a boolean state of the actor's process, probably the weapon-out state
/// (`fn_008a6970` reads it, process method `+0x450` writes it). Nothing
/// happens when
/// `drawn` is set and `008846e0` has flag `0x800`, for the player while the
/// iron sights are up, without a process, or when `drawn` equals
/// `fn_008a6970`. Otherwise: virtual method `+0x340` with `0xd`, the
/// process's method `+0x450` with `drawn`, and, for another actor than the
/// player whose process method `+0x148` returned 0, loads the animation
/// group `0x18` if it is missing (`ModelLoader::BuildKFFileList` and
/// `Animation::AddAnimationsFromList`).
pub fn fn_008a6840(e: &mut Engine, this: Ptr<Actor>, drawn: u8) {
    if drawn != 0 && e.call(0x0088_46e0, &args![this]).u32() & 0x800 != 0 {
        return;
    }
    let player: u32 = e.global(PLAYER_CHARACTER);
    if this.addr() == player && e.call(0x008b_bc10, &args![this]).bool() {
        return;
    }
    let proc = acquire_object(e, this);
    if proc.is_null() {
        return;
    }
    let current = fn_008a6970(e, this) as u8;
    if drawn == current {
        return;
    }
    e.vcall(this.addr(), 0x340, &args![0xdu32]);
    e.vcall(proc.addr(), 0x450, &args![drawn]);
    if this.addr() == player {
        return;
    }
    if e.vcall(proc.addr(), 0x148, &args![]).u32() != 0 || drawn == 0 {
        return;
    }
    let animation = e.call(GET_ANIMATION, &args![this]).u32();
    if animation == 0 {
        return;
    }
    let group = e.call(0x005f_2370, &args![0u32, 1u32, 0x18u32, 0u32]).u16();
    if e.call(0x0049_4710, &args![animation, group as u32]).bool() {
        return;
    }
    let model = e.call(0x0057_15d0, &args![this, 1u32, 0u32, 1u32]).u32();
    let loader: u32 = e.global(MODEL_LOADER);
    let list = e.call(0x0044_7330, &args![loader, model]).u32();
    e.call(0x0049_0400, &args![animation, list]);
}

// Translated from 008a6970 (decompiled, FalloutNV.exe 1.4.0.525)
/// With `008846e0` flag `0x800`: for the player, quits the VATS playback when
/// the VATS mode is 4, and returns false; otherwise the process's virtual
/// method `+0x44c` (this assumes a process).
pub fn fn_008a6970(e: &mut Engine, this: Ptr<Actor>) -> bool {
    if e.call(0x0088_46e0, &args![this]).u32() & 0x800 != 0 {
        let player: u32 = e.global(PLAYER_CHARACTER);
        if this.addr() == player && e.call(0x0044_ddc0, &args![0x011f_2250u32]).u32() == 4 {
            e.call(0x009c_8950, &args![0x011f_2250u32, 0u32, 0u32]);
        }
        return false;
    }
    let proc = process(e, this);
    e.vcall(proc.addr(), 0x44c, &args![]).bool()
}

// Translated from 008a69d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsContinuingPackageforPC` (Xbox PDB): the process's virtual
/// method `+0x4e0`, false without a process.
pub fn actor_is_continuing_package_for_pc(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let proc = process(e, this);
    if proc.is_null() {
        return false;
    }
    let proc = process(e, this);
    e.vcall(proc.addr(), 0x4e0, &args![]).bool()
}

// Translated from 008a6a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Turns the actor to `heading` (`MobileObject::SetHeading`) when `004f8960`
/// of the actor is 0 or 4.
pub fn fn_008a6a00(e: &mut Engine, this: Ptr<Actor>, heading: f32) {
    if e.call(0x004f_8960, &args![this]).u32() != 0 && e.call(0x004f_8960, &args![this]).u32() != 4
    {
        return;
    }
    e.call(0x0093_1b60, &args![this, heading]);
}

// Translated from 008a6a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::UnlockLockDoorsProcedure` (Xbox PDB): for an actor with a process
/// whose current package (process method `+0x22c`) is not an interrupt
/// package and which is in a cell, runs the package's door reference search
/// (callback `0090d480` when `005394a0` holds, else `0090d570` when
/// `00670ed0` does), then asks the package (virtual method `+0x13c` with the
/// actor, 0, `-1.0` and 0) and, when that succeeds and the process method
/// `+0x1bc` is false, searches again (`0090d480` when `00670f40` holds, and
/// the process method `+0x1c0` with 1; else `0090d570` when `00670f60`).
pub fn actor_unlock_lock_doors_procedure(e: &mut Engine, this: Ptr<Actor>) {
    if process(e, this).is_null() || e.vcall(this.addr(), 0x22c, &args![0u32]).u8() != 0 {
        return;
    }
    let global_object: u32 = e.global(0x011d_df38);
    if e.call(0x0042_ce10, &args![global_object]).bool() {
        return;
    }
    let proc = process(e, this);
    let package = e.vcall(proc.addr(), 0x22c, &args![]).u32();
    if package == 0
        || e.call(GET_PARENT_CELL, &args![this]).u32() == 0
        || e.call(0x0067_8610, &args![package]).bool()
    {
        return;
    }
    if e.call(0x0053_94a0, &args![package]).bool() {
        enum_references_around(e, this, PACKAGE_CALLBACK_B);
    } else if e.call(0x0067_0ed0, &args![package]).bool() {
        enum_references_around(e, this, PACKAGE_CALLBACK_A);
    }
    let minus_one: f32 = e.global(MINUS_ONE);
    let accepted = e
        .vcall(package, 0x13c, &args![this, 0u32, minus_one, 0u32])
        .bool();
    if !accepted {
        return;
    }
    let proc = acquire_object(e, this);
    if e.vcall(proc.addr(), 0x1bc, &args![]).bool() {
        return;
    }
    if e.call(0x0067_0f40, &args![package]).bool() {
        enum_references_around(e, this, PACKAGE_CALLBACK_B);
        let proc = acquire_object(e, this);
        e.vcall(proc.addr(), 0x1c0, &args![1u32]);
    } else if e.call(0x0067_0f60, &args![package]).bool() {
        enum_references_around(e, this, PACKAGE_CALLBACK_A);
    }
}

// Translated from 008a6ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::EvaluatePackage` (Xbox PDB): re-evaluates which package the actor
/// runs. With `now` zero the evaluation is only buffered (`bEVPBuffered` set,
/// `bResetAI` = `reset_ai`) and the process is told the hour before now.
/// With `now` set and `reset_ai` zero the actor stops moving and the process
/// re-starts its package (`+0x524`, `+0x118`, `+0x24`) when the actor has no
/// package change pending (method `+0x214` is 0, 4 or 9). With both set the
/// whole change is made: reference searches for the current package, removal
/// as a follower, `EndInterruptPackage`, leaving furniture, the process's
/// package switch (`+0x524`, `+0x4ec`, `+0x24`), the editor package's script
/// action flag, and the cleanup that depends on method `+0x214`
/// (`+0x418`, or `SetChaseBip`, `SetAnimAction` and clearing the animation
/// groups). Returns nothing; the two flags are cleared at the end.
pub fn actor_evaluate_package(e: &mut Engine, this: Ptr<Actor>, now: u8, reset_ai: u8) {
    if e.vcall(this.addr(), 0x22c, &args![1u32]).u8() != 0 {
        return;
    }
    if now == 0 {
        e.set(this, Actor::bEVPBuffered, 1);
        e.set(this, Actor::bResetAI, reset_ai);
        let proc = process(e, this);
        if !proc.is_null() {
            let hour = hour_before(e);
            let proc = process(e, this);
            e.call(HOUR_TO_PROCESS, &args![proc, hour]);
        }
        return;
    }
    if e.call(IS_ALIVE_BODY, &args![this]).bool() {
        e.vcall(this.addr(), 0x434, &args![0u32]);
    }
    if reset_ai == 0 {
        if e.vcall(this.addr(), 0x214, &args![]).u32() != 0
            && e.vcall(this.addr(), 0x214, &args![]).u32() != 4
            && e.vcall(this.addr(), 0x214, &args![]).u32() != 9
        {
            return;
        }
        e.call(STOP_MOVING, &args![this]);
        let proc = process(e, this);
        if !proc.is_null() {
            let proc = process(e, this);
            e.vcall(proc.addr(), 0x524, &args![this]);
            let proc = process(e, this);
            e.vcall(proc.addr(), 0x118, &args![1u32]);
            let proc = process(e, this);
            e.vcall(proc.addr(), 0x24, &args![this, 1u32]);
            let proc = process(e, this);
            if e.call(0x0045_cd60, &args![proc]).u32() != 0 {
                let hour = hour_before(e);
                let proc = process(e, this);
                e.call(HOUR_TO_PROCESS, &args![proc, hour]);
            }
        }
        e.set(this, Actor::bEVPBuffered, 0);
        e.set(this, Actor::bResetAI, 0);
        return;
    }

    if !acquire_object(e, this).is_null() {
        let proc = acquire_object(e, this);
        // The result is not used afterwards.
        e.vcall(proc.addr(), 0x20c, &args![]);
        let proc = acquire_object(e, this);
        if e.vcall(proc.addr(), 0x22c, &args![]).u32() != 0 {
            let proc = acquire_object(e, this);
            let package = e.vcall(proc.addr(), 0x22c, &args![]).u32();
            if e.call(0x0067_0f10, &args![package]).bool() {
                enum_references_around(e, this, PACKAGE_CALLBACK_A);
            }
            if e.call(0x0067_0ef0, &args![package]).bool() {
                enum_references_around(e, this, PACKAGE_CALLBACK_B);
            }
        }
        let proc = acquire_object(e, this);
        if e.vcall(proc.addr(), 0x360, &args![]).u8() != 0 {
            let mut follower = 0u32;
            let proc = acquire_object(e, this);
            if e.vcall(proc.addr(), 0x128, &args![]).u32() != 0 {
                let proc = acquire_object(e, this);
                let candidate = e.vcall(proc.addr(), 0x128, &args![]).u32();
                if e.vcall(candidate, 0x100, &args![]).u8() != 0 {
                    let proc = acquire_object(e, this);
                    follower = e.vcall(proc.addr(), 0x128, &args![]).u32();
                }
            }
            if follower != 0 {
                let list = e.call(0x005d_43c0, &args![follower]).u32();
                e.call(0x0042_2690, &args![list, this]);
            }
        }
        let editor_package = e.call(0x0088_14b0, &args![this]).ptr::<()>();
        e.call(0x0088_1680, &args![this, 0u32]);
        let proc = acquire_object(e, this);
        let old_package = e.vcall(proc.addr(), 0x22c, &args![]).ptr::<()>();
        let proc = acquire_object(e, this);
        if e.vcall(proc.addr(), 0x22c, &args![]).u32() != 0 {
            let proc = acquire_object(e, this);
            e.vcall(proc.addr(), 0x234, &args![]);
        }
        e.call(0x0088_d640, &args![this]);
        let proc = acquire_object(e, this);
        e.vcall(proc.addr(), 0x524, &args![this]);
        let proc = acquire_object(e, this);
        e.vcall(proc.addr(), 0x4ec, &args![0u32]);
        let proc = acquire_object(e, this);
        e.vcall(proc.addr(), 0x24, &args![this, 1u32]);
        let proc = acquire_object(e, this);
        let new_package = e.vcall(proc.addr(), 0x22c, &args![]).ptr::<()>();
        if !editor_package.is_null() && new_package != editor_package {
            let list = e.call(0x005d_43c0, &args![this]).u32();
            e.call(0x005a_c750, &args![editor_package, list, 0x800u32]);
            let proc = acquire_object(e, this);
            e.vcall(proc.addr(), 0x59c, &args![this, editor_package]);
            let proc = acquire_object(e, this);
            e.vcall(proc.addr(), 0x5a8, &args![0u32]);
        }
        if !old_package.is_null()
            && (e.call(FORM_TYPE, &args![old_package]).i32() == 4
                || e.call(0x0044_1b00, &args![old_package]).bool())
        {
            // The form's type (0x2a or 0x2b) is looked at but nothing is done
            // with it.
            let form = e.call(GET_SAVE_FORM, &args![this]).u32();
            e.call(0x0040_1170, &args![form]);
        }
        e.call(0x0087_faa0, &args![this]);
        let kind = e.vcall(this.addr(), 0x214, &args![]).u32();
        if kind != 0 {
            let mut finish = true;
            if kind == 4 {
                let proc = acquire_object(e, this);
                if e.vcall(proc.addr(), 0x22c, &args![]).u32() == 0 {
                    finish = false;
                }
            } else if kind != 9 {
                finish = false;
                e.call(0x0093_1fb0, &args![this, 0u32]);
                let proc = process(e, this);
                e.vcall(proc.addr(), 0x4c0, &args![this, 0u32, 0u32, 0x7fu32]);
                actor_set_anim_action(e, this, -1, 0);
                let animation = e.call(GET_ANIMATION, &args![this]).u32();
                if animation != 0 {
                    e.call(0x0049_6080, &args![animation, 0x14u32, 0.0f32]);
                    e.call(0x0049_8910, &args![animation, 1u32, 0u32]);
                }
            }
            if finish {
                e.vcall(this.addr(), 0x418, &args![]);
            }
        }
    }

    let proc = process(e, this);
    if !proc.is_null() {
        let proc = process(e, this);
        if e.call(0x0045_cd60, &args![proc]).u32() != 0 {
            let hour = hour_before(e);
            let proc = process(e, this);
            e.call(HOUR_TO_PROCESS, &args![proc, hour]);
        }
    }
    let proc = process(e, this);
    let package = if proc.is_null() {
        0
    } else {
        let proc = process(e, this);
        e.vcall(proc.addr(), 0x22c, &args![]).u32()
    };
    if package != 0 && e.call(0x0067_1d10, &args![package]).u32() != 0 {
        let count = e.call(0x0067_2930, &args![package]).u32();
        let proc = process(e, this);
        e.vcall(proc.addr(), 0x15c, &args![count]);
    }
    e.set(this, Actor::bEVPBuffered, 0);
    e.set(this, Actor::bResetAI, 0);
}

// Translated from 008a73e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetAnimAction` (Xbox PDB): sets the actor's animation action
/// (`action`; `sequence` is an animation sequence or 0). Before the process
/// is told (virtual method `+0x3ec`), clears the animation's upper-body
/// group 2 (`Animation::ClearGroup`, for the player also its first-person
/// animation) when it plays an animation group of type `0xaa`, unless the
/// current action is 4, the action is 7, the sequence's group entry in table
/// `011977e0` is 2, or the action is a weapon one (2, 4 to 6) that the weapon
/// form (`006450c0`) does not allow.
pub fn actor_set_anim_action(e: &mut Engine, this: Ptr<Actor>, action: i32, sequence: u32) {
    let animation = e.call(GET_ANIMATION, &args![this]).u32();
    let current = actor_get_anim_action(e, this);
    'clear: {
        if current == 4
            || animation == 0
            || e.call(0x0049_1040, &args![animation, 2u32]).u32() == 0
            || action == 7
        {
            break 'clear;
        }
        if sequence != 0 {
            let global = e.call(0x0048_f7f0, &args![sequence]).u32();
            let index = e.call(0x005f_2420, &args![global]).u32();
            if e.mem.u32(ANIM_GROUP_TABLE + index.wrapping_mul(0x24)) == 2 {
                break 'clear;
            }
        }
        let proc = process(e, this);
        let mut allowed = true;
        let weapon = e.vcall(proc.addr(), 0x148, &args![]).u32();
        let weapon_form = if weapon != 0 {
            let proc = process(e, this);
            let weapon = e.vcall(proc.addr(), 0x148, &args![]).u32();
            e.call(0x0044_ddc0, &args![weapon]).u32()
        } else {
            0
        };
        if (action == 2 || (action > 3 && action <= 6))
            && weapon_form != 0
            && !e.call(0x0064_50c0, &args![weapon_form]).bool()
        {
            allowed = false;
        }
        if allowed {
            let group = e.call(0x0043_01b0, &args![animation, 2u32]).u16();
            let group_type = e.call(0x005f_2440, &args![group as u32]).i32();
            if group_type == 0xaa {
                e.call(0x0049_6080, &args![animation, 2u32, 0.0f32]);
                let player: u32 = e.global(PLAYER_CHARACTER);
                if this.addr() == player {
                    let first_person = e.call(0x0095_0a60, &args![player, 1u32]).u32();
                    e.call(0x0049_6080, &args![first_person, 2u32, 0.0f32]);
                }
            }
        }
    }
    let proc = process(e, this);
    if !proc.is_null() {
        let proc = process(e, this);
        e.vcall(proc.addr(), 0x3ec, &args![action, sequence]);
    }
}

// Translated from 008a7570 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetAnimAction` (Xbox PDB): the process's virtual method `+0x3e4`,
/// `-1` without a process.
pub fn actor_get_anim_action(e: &mut Engine, this: Ptr<Actor>) -> i32 {
    let proc = process(e, this);
    if proc.is_null() {
        return -1;
    }
    let proc = process(e, this);
    e.vcall(proc.addr(), 0x3e4, &args![]).i32()
}

/// The package changes shared by `008a75a0` and `008a7760`: nothing happens
/// when `00437bf0` or `00437bd0` hold or the current package already has
/// type `0x1a`; returns the process on which the new package is installed.
fn package_change_allowed(e: &mut Engine, this: Ptr<Actor>) -> bool {
    let proc = acquire_object(e, this);
    let package = e.vcall(proc.addr(), 0x27c, &args![]).u32();
    if e.call(0x0043_7bf0, &args![this]).bool() || e.call(0x0043_7bd0, &args![this]).bool() {
        return false;
    }
    !(package != 0 && e.call(FORM_TYPE, &args![package]).i32() == 0x1a)
}

// Translated from 008a75a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Makes the actor run a freshly created package of type `0x1a` located at
/// the actor itself: unless the change is not allowed, or the process method
/// `+0x4d0` returns `0x7f` (then the process method `+0x4c0` is called with
/// the actor, 0, 0 and `0x7f` instead), it creates the package
/// (`TESPackage::CreatePackage`, `SetPackType`), gives it a
/// `PackageLocation` referring to the actor, sets its data and installs it
/// with the virtual method `+0x2f4`.
pub fn fn_008a75a0(e: &mut Engine, this: Ptr<Actor>) {
    if !package_change_allowed(e, this) {
        return;
    }
    let proc = process(e, this);
    if e.vcall(proc.addr(), 0x4d0, &args![]).i32() == 0x7f {
        let proc = process(e, this);
        e.vcall(proc.addr(), 0x4c0, &args![this, 0u32, 0u32, 0x7fu32]);
        return;
    }
    let package = e.call(0x0067_0b90, &args![0x1au32]).u32();
    e.call(0x0067_0fc0, &args![package, 0x1au32]);
    e.call(0x0082_6b40, &args![package, 0u32]);
    e.call(0x0082_6b90, &args![package, 1u32]);
    let memory = e.call(OPERATOR_NEW, &args![0xcu32]).u32();
    let location = if memory != 0 {
        e.call(0x0067_f030, &args![memory]).u32()
    } else {
        0
    };
    e.call(0x0067_f3c0, &args![location, this]);
    e.call(0x0067_1d30, &args![package, location]);
    if location != 0 {
        e.call(0x0067_0b30, &args![location, 1u32]);
    }
    e.call(0x0098_4f60, &args![package, 0x15u32]);
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x28, &args![]);
    e.vcall(this.addr(), 0x2f4, &args![package, 1u32, 1u32]);
}

// Translated from 008a7760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Makes the actor run a freshly created package of type `0x27` (cast to the
/// back-up package type and initialized by `BackUpPackage::
/// InitializeBackUpPackage` with the actor, its cell and the player), under
/// the same conditions as `008a75a0`.
pub fn fn_008a7760(e: &mut Engine, this: Ptr<Actor>) {
    if !package_change_allowed(e, this) {
        return;
    }
    let package = e.call(0x0067_0b90, &args![0x27u32]).u32();
    e.call(0x0067_0fc0, &args![package, 0x27u32]);
    e.call(0x0082_6b40, &args![package, 0u32]);
    e.call(0x0082_6b90, &args![package, 1u32]);
    let back_up = e
        .call(
            DYNAMIC_CAST,
            &args![package, 0u32, TYPE_TES_PACKAGE, TYPE_BACK_UP_PACKAGE, 0u32],
        )
        .u32();
    if back_up != 0 {
        let player: u32 = e.global(PLAYER_CHARACTER);
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        e.call(0x009e_d0d0, &args![back_up, this, cell, player]);
    }
    e.call(0x0098_4f60, &args![package, 0x30u32]);
    let proc = acquire_object(e, this);
    e.vcall(proc.addr(), 0x28, &args![]);
    e.vcall(this.addr(), 0x2f4, &args![package, 1u32, 1u32]);
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x008a50d0, fn_008a50d0(Ptr, f32)),
        entry!(0x008a50f0, fn_008a50f0(Ptr, Ptr)),
        entry!(0x008a5110, fn_008a5110(Ptr, Ptr)),
        entry!(0x008a5170, fn_008a5170(Ptr) -> bool),
        entry!(0x008a5190, fn_008a5190() -> u32),
        entry!(0x008a51a0, fn_008a51a0(Ptr<Actor>) -> u32),
        entry!(0x008a51f0, fn_008a51f0(Ptr<Actor>, u32)),
        entry!(0x008a5230, fn_008a5230(Ptr<Actor>)),
        entry!(0x008a5270, fn_008a5270(Ptr<Actor>) -> u32),
        entry!(0x008a52c0, fn_008a52c0(Ptr<Actor>, u32)),
        entry!(0x008a5300, fn_008a5300(Ptr<Actor>)),
        entry!(
            0x008a5340,
            actor_get_weapon_position(Ptr<Actor>, Ptr, u8) -> Ptr
        ),
        entry!(0x008a5530, fn_008a5530(Ptr<Actor>)),
        entry!(0x008a5580, fn_008a5580(Ptr<Actor>, f32, Ptr, u8)),
        entry!(0x008a5cf0, fn_008a5cf0(Ptr<Actor>, u8)),
        entry!(0x008a5d10, fn_008a5d10(Ptr<Actor>, f32)),
        entry!(0x008a5e40, actor_set_alert(Ptr<Actor>, u8)),
        entry!(0x008a5e80, actor_get_alert(Ptr<Actor>) -> bool),
        entry!(0x008a5eb0, actor_set_havok_weapon(Ptr<Actor>) -> u32),
        entry!(
            0x008a5f20,
            actor_set_havok_weapon_ov2(Ptr<Actor>, u8) -> u32
        ),
        entry!(0x008a6170, fn_008a6170(Ptr<Actor>) -> bool),
        entry!(0x008a61b0, actor_is_alarmed(Ptr<Actor>) -> bool),
        entry!(0x008a6210, fn_008a6210(Ptr<Actor>) -> bool),
        entry!(0x008a6290, fn_008a6290(Ptr) -> bool),
        entry!(0x008a62b0, actor_move(Ptr<Actor>, f32, u32, u32) -> Ptr),
        entry!(0x008a6630, fn_008a6630(Ptr)),
        entry!(0x008a6650, actor_is_fleeing(Ptr<Actor>, u8) -> bool),
        entry!(0x008a6730, actor_remove_flee_target(Ptr<Actor>, u32)),
        entry!(0x008a67d0, fn_008a67d0(Ptr) -> bool),
        entry!(0x008a67f0, actor_is_talking(Ptr<Actor>) -> bool),
        entry!(0x008a6840, fn_008a6840(Ptr<Actor>, u8)),
        entry!(0x008a6970, fn_008a6970(Ptr<Actor>) -> bool),
        entry!(
            0x008a69d0,
            actor_is_continuing_package_for_pc(Ptr<Actor>) -> bool
        ),
        entry!(0x008a6a00, fn_008a6a00(Ptr<Actor>, f32)),
        entry!(0x008a6a40, actor_unlock_lock_doors_procedure(Ptr<Actor>)),
        entry!(0x008a6ce0, actor_evaluate_package(Ptr<Actor>, u8, u8)),
        entry!(0x008a73e0, actor_set_anim_action(Ptr<Actor>, i32, u32)),
        entry!(0x008a7570, actor_get_anim_action(Ptr<Actor>) -> i32),
        entry!(0x008a75a0, fn_008a75a0(Ptr<Actor>)),
        entry!(0x008a7760, fn_008a7760(Ptr<Actor>)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vtables of the fake actor, process and other objects. Slot `s` of
    /// the table at `at` points at the fake function [`target`]`(at, s)`.
    const ACTOR_VT: u32 = 0x0900_0000;
    const PROCESS_VT: u32 = 0x0902_0000;
    const OBJECT_VT: u32 = 0x0904_0000;

    fn target(at: u32, slot: u32) -> u32 {
        at + 0x1_0000 + slot
    }

    /// A vtable at `at` with the given slots, each pointing at its fake
    /// function, which does nothing and returns 0 until a test replaces it.
    fn make_vtable(e: &mut Engine, at: u32, slots: &[u32]) {
        let len = slots.iter().max().unwrap() / 4 + 1;
        let mut words = vec![0u32; len as usize];
        for slot in slots {
            words[(*slot / 4) as usize] = target(at, *slot);
            e.register(target(at, *slot), |_, _| Ret::default());
        }
        e.put_vtable(at, &words);
    }

    /// Every actor vtable slot any test of this file calls.
    const ACTOR_SLOTS: [u32; 27] = [
        0x13c, 0x140, 0x178, 0x1d0, 0x1e4, 0x1e8, 0x1f4, 0x214, 0x218, 0x22c, 0x230, 0x268, 0x27c,
        0x284, 0x2e8, 0x2f4, 0x2f8, 0x338, 0x340, 0x348, 0x350, 0x360, 0x3ac, 0x418, 0x428, 0x434,
        0x4c0,
    ];
    /// Every process vtable slot any test of this file calls.
    const PROCESS_SLOTS: [u32; 51] = [
        0x24, 0x28, 0x44, 0x100, 0x118, 0x128, 0x148, 0x15c, 0x190, 0x1bc, 0x1c0, 0x20c, 0x22c,
        0x234, 0x27c, 0x280, 0x2e8, 0x318, 0x31c, 0x320, 0x360, 0x3e4, 0x3ec, 0x44c, 0x450, 0x48c,
        0x4b8, 0x4c0, 0x4d0, 0x4e0, 0x4ec, 0x524, 0x59c, 0x5a8, 0x718, 0x770, 0x774, 0x778, 0x77c,
        0x780, 0x784, 0x428, 0x434, 0x418, 0x2f4, 0x3ac, 0x338, 0x1f4, 0x13c, 0x11c, 0xb4,
    ];

    fn engine() -> Engine {
        let mut e = Engine::new();
        // The pages of the exe's data these functions read.
        for page in [
            0x0101_2000,
            0x0101_6000,
            0x0102_3000,
            0x0102_b000,
            0x0103_0000,
            0x0106_b000,
            0x0118_8000,
            0x0119_7000,
            0x011a_3000,
            0x011c_3000,
            0x011c_6000,
            0x011d_d000,
            0x011d_e000,
            0x011d_f000,
            0x011f_4000,
        ] {
            e.map(page, 0x1000);
        }
        make_vtable(&mut e, ACTOR_VT, &ACTOR_SLOTS);
        make_vtable(&mut e, PROCESS_VT, &PROCESS_SLOTS);
        make_vtable(
            &mut e,
            OBJECT_VT,
            &[0x8, 0x11c, 0xb4, 0x100, 0x13c, 0x128, 0x1f4],
        );
        // `008d8520`: the process field.
        e.register(GET_PROCESS, |e, a| e.mem.u32(a[0] + 0x68).into_ret());
        e
    }

    fn new_actor(e: &mut Engine) -> Ptr<Actor> {
        let actor: Ptr<Actor> = e.new_object();
        e.mem.set_u32(actor.addr(), ACTOR_VT);
        actor
    }

    /// An actor with a process.
    fn new_actor_with_process(e: &mut Engine) -> (Ptr<Actor>, Ptr) {
        let actor = new_actor(e);
        let proc = Ptr::new(e.mem.alloc(0x800));
        e.mem.set_u32(proc.addr(), PROCESS_VT);
        e.set(actor, Actor::pCurrentProcess, proc);
        (actor, proc)
    }

    fn new_object(e: &mut Engine) -> Ptr {
        let object: Ptr = Ptr::new(e.mem.alloc(0x100));
        e.mem.set_u32(object.addr(), OBJECT_VT);
        object
    }

    /// Makes the fake function behind a vtable slot return `value`.
    fn answer(e: &mut Engine, at: u32, slot: u32, value: u32) {
        e.register_double(target(at, slot), move |_, _| Ret {
            eax: value,
            ..Ret::default()
        });
    }

    /// Does nothing, returns 0.
    fn stub(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// Returns the calls recorded since `e.call_log = Some(vec![])`.
    fn calls(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.take().unwrap_or_default()
    }

    /// The argument lists of the calls to `addr`.
    fn called(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    #[test]
    fn word_store_puts_the_float_in_word_three() {
        let mut e = engine();
        e.register(0x0056_0d30, |_, a| (a[0] + 4 * a[1]).into_ret());
        let object: Ptr = Ptr::new(e.mem.alloc(0x20));
        e.call(0x008a_50d0, &args![object, 2.5f32]);
        assert_eq!(e.mem.f32(object.addr() + 12), 2.5);
        assert_eq!(e.mem.f32(object.addr() + 8), 0.0);
    }

    #[test]
    fn vector_copy_keeps_the_fourth_float() {
        let mut e = engine();
        let source: Ptr = Ptr::new(e.mem.alloc(0x10));
        let dest: Ptr = Ptr::new(e.mem.alloc(0x10));
        for (i, v) in [1.0f32, 2.0, 3.0, 4.0].iter().enumerate() {
            e.mem.set_f32(source.addr() + 4 * i as u32, *v);
            e.mem.set_f32(dest.addr() + 4 * i as u32, 9.0);
        }
        e.call(0x008a_5110, &args![dest, source]);
        let words: Vec<f32> = (0..4).map(|i| e.mem.f32(dest.addr() + 4 * i)).collect();
        assert_eq!(words, vec![1.0, 2.0, 3.0, 9.0]);
        // The thunk forwards.
        e.mem.set_f32(dest.addr(), 0.0);
        e.call(0x008a_50f0, &args![dest, source]);
        assert_eq!(e.mem.f32(dest.addr()), 1.0);
    }

    #[test]
    fn flag_test_ands_word_one_with_one() {
        let mut e = engine();
        e.register(0x0062_1270, |e, a| (e.mem.u32(a[0] + 4) & a[1]).into_ret());
        let object: Ptr = Ptr::new(e.mem.alloc(0x10));
        assert!(!e.call(0x008a_5170, &args![object]).bool());
        e.mem.set_u32(object.addr() + 4, 3);
        assert!(e.call(0x008a_5170, &args![object]).bool());
        e.mem.set_u32(object.addr() + 4, 2);
        assert!(!e.call(0x008a_5170, &args![object]).bool());
    }

    #[test]
    fn global_word_is_returned() {
        let mut e = engine();
        e.set_global(0x011c_6254, 0x1234u32);
        assert_eq!(e.call(0x008a_5190, &args![]).u32(), 0x1234);
    }

    /// The six process forwarders: (address, slot, takes an argument,
    /// returns a value).
    const FORWARDERS: [(u32, u32, bool, bool); 6] = [
        (0x008a_51a0, 0x77c, false, true),
        (0x008a_51f0, 0x780, true, false),
        (0x008a_5230, 0x784, false, false),
        (0x008a_5270, 0x770, false, true),
        (0x008a_52c0, 0x774, true, false),
        (0x008a_5300, 0x778, false, false),
    ];

    #[test]
    fn process_forwarders_call_their_slot() {
        for (addr, slot, takes_argument, returns) in FORWARDERS {
            let mut e = engine();
            let (actor, proc) = new_actor_with_process(&mut e);
            answer(&mut e, PROCESS_VT, slot, 77);
            e.call_log = Some(vec![]);
            let words = if takes_argument {
                args![actor, 5u32]
            } else {
                args![actor]
            };
            let result = e.call(addr, &words).u32();
            let log = calls(&mut e);
            let slot_calls = called(&log, target(PROCESS_VT, slot));
            assert_eq!(slot_calls.len(), 1, "{addr:08x}");
            assert_eq!(slot_calls[0][0], proc.addr());
            if takes_argument {
                assert_eq!(slot_calls[0][1], 5);
            }
            if returns {
                assert_eq!(result, 77, "{addr:08x}");
            }
        }
    }

    #[test]
    fn process_forwarders_do_nothing_without_a_process() {
        for (addr, slot, takes_argument, _) in FORWARDERS {
            let mut e = engine();
            let actor = new_actor(&mut e);
            e.call_log = Some(vec![]);
            let words = if takes_argument {
                args![actor, 5u32]
            } else {
                args![actor]
            };
            let result = e.call(addr, &words).u32();
            let log = calls(&mut e);
            assert!(called(&log, target(PROCESS_VT, slot)).is_empty());
            assert_eq!(result, 0);
        }
    }

    /// A block of `n` floats.
    fn floats(e: &mut Engine, values: &[f32]) -> Ptr {
        let block: Ptr = Ptr::new(e.mem.alloc(4 * values.len().max(13) as u32));
        for (i, v) in values.iter().enumerate() {
            e.mem.set_f32(block.addr() + 4 * i as u32, *v);
        }
        block
    }

    fn point(e: &Engine, at: Ptr) -> [f32; 3] {
        [
            e.mem.f32(at.addr()),
            e.mem.f32(at.addr() + 4),
            e.mem.f32(at.addr() + 8),
        ]
    }

    /// Doubles for the transform helpers of `Actor::GetWeaponPosition`:
    /// a transform's first three words stand for its translation.
    fn weapon_position_engine() -> (Engine, Ptr<Actor>, Ptr, Ptr) {
        let mut e = engine();
        let (actor, _) = new_actor_with_process(&mut e);
        let root = floats(&mut e, &[10.0, 20.0, 30.0]);
        let node = floats(&mut e, &[1.0, 2.0, 3.0]);
        answer(&mut e, ACTOR_VT, 0x1d0, root.addr());
        answer(&mut e, ACTOR_VT, 0x1e8, 0x55);
        answer(&mut e, PROCESS_VT, 0x148, 0x1234);
        answer(&mut e, PROCESS_VT, 0x190, node.addr());
        e.register(0x0044_ddc0, |_, a| a[0].into_ret());
        e.register(0x0064_47f0, |_, _| 10.0f32.into_ret());
        e.register(0x0064_63e0, |_, a| (f32::from_bits(a[0]) * 2.0).into_ret());
        // The node's transform is the node itself.
        e.register(0x0046_1130, |_, a| a[0].into_ret());
        e.register(0x0041_6870, |e, a| {
            for i in 1..4 {
                e.mem.set_u32(a[0] + 4 * (i - 1), a[i as usize]);
            }
            a[0].into_ret()
        });
        e.register(0x0052_4c40, |e, a| {
            for i in 0..3 {
                let sum = e.mem.f32(a[0] + 4 * i) + e.mem.f32(a[2] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, sum);
            }
            Ret::default()
        });
        // The inverse is marked, and the product adds 100 to word 0.
        stub(&mut e, &[0x0047_6a80]);
        e.register(0x004b_4880, |e, a| {
            e.mem.set_f32(a[1], -1.0);
            Ret::default()
        });
        e.register(0x0062_c250, |e, a| {
            for i in 0..13 {
                let word = e.mem.u32(a[2] + 4 * i);
                e.mem.set_u32(a[1] + 4 * i, word);
            }
            let x = e.mem.f32(a[1]) + 100.0 * e.mem.f32(a[0]);
            e.mem.set_f32(a[1], x);
            a[1].into_ret()
        });
        e.set_global(ZERO_POINT, 7.0f32);
        e.set_global(ZERO_POINT + 4, 8.0f32);
        e.set_global(ZERO_POINT + 8, 9.0f32);
        let result: Ptr = Ptr::new(e.mem.alloc(0x10));
        (e, actor, result, root)
    }

    #[test]
    fn weapon_position_is_the_reach_point_through_the_node_transform() {
        let (mut e, actor, result, _) = weapon_position_engine();
        let back = e.call(0x008a_5340, &args![actor, result, 1u8]).ptr::<()>();
        assert_eq!(back, result);
        // reach 20; translation (1, 2, 3): (0 + 1, 20 + 2, 0 + 3).
        assert_eq!(point(&e, result), [1.0, 22.0, 3.0]);
    }

    #[test]
    fn weapon_position_in_root_space_multiplies_by_the_inverse() {
        let (mut e, actor, result, _) = weapon_position_engine();
        e.call_log = Some(vec![]);
        e.call(0x008a_5340, &args![actor, result, 0u8]);
        let log = calls(&mut e);
        let order: Vec<u32> = log
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| [0x0047_6a80, 0x004b_4880, 0x0062_c250, 0x0052_4c40].contains(a))
            .collect();
        assert_eq!(
            order,
            vec![0x0047_6a80, 0x004b_4880, 0x0062_c250, 0x0052_4c40]
        );
        // The inverse is built from the root's transform (a double marks
        // its word 0 as -1): the product's word 0 is 1 + 100 * -1 = -99.
        assert_eq!(point(&e, result), [-99.0 + 0.0, 22.0, 3.0]);
    }

    #[test]
    fn weapon_position_without_weapon_has_no_reach() {
        let (mut e, actor, result, _) = weapon_position_engine();
        answer(&mut e, PROCESS_VT, 0x148, 0);
        e.call(0x008a_5340, &args![actor, result, 1u8]);
        assert_eq!(point(&e, result), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn weapon_position_falls_back_to_the_zero_point() {
        // No 3D.
        let (mut e, actor, result, _) = weapon_position_engine();
        answer(&mut e, ACTOR_VT, 0x1d0, 0);
        e.call(0x008a_5340, &args![actor, result, 1u8]);
        assert_eq!(point(&e, result), [7.0, 8.0, 9.0]);
        // No weapon node.
        let (mut e, actor, result, _) = weapon_position_engine();
        answer(&mut e, PROCESS_VT, 0x190, 0);
        e.call(0x008a_5340, &args![actor, result, 1u8]);
        assert_eq!(point(&e, result), [7.0, 8.0, 9.0]);
    }

    #[test]
    fn face_reset_passes_the_fade() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        let face = new_object(&mut e);
        e.set_global(FACE_FADE, 0.75f32);
        e.register(GET_FACE_ANIMATION_DATA, |e, _| {
            e.mem.u32(0x011d_f000).into_ret()
        });
        e.set_global(0x011d_f000, face.addr());
        e.call_log = Some(vec![]);
        e.call(0x008a_5530, &args![actor]);
        let log = calls(&mut e);
        let call = called(&log, target(OBJECT_VT, 0xb4));
        assert_eq!(
            call,
            vec![vec![face.addr(), 0.75f32.to_bits(), 0, 1, 1, 1, 0]]
        );
        // Without face data nothing is called.
        e.set_global(0x011d_f000, 0u32);
        e.call_log = Some(vec![]);
        e.call(0x008a_5530, &args![actor]);
        assert!(called(&calls(&mut e), target(OBJECT_VT, 0xb4)).is_empty());
    }

    #[test]
    fn use_emotion_byte_is_stored() {
        let mut e = engine();
        let actor = new_actor(&mut e);
        e.call(0x008a_5cf0, &args![actor, 1u8]);
        assert_eq!(e.get(actor, Actor::bUseEmotion), 1);
        e.call(0x008a_5cf0, &args![actor, 0u8]);
        assert_eq!(e.get(actor, Actor::bUseEmotion), 0);
    }

    #[test]
    fn alert_goes_through_the_process() {
        let mut e = engine();
        let (actor, proc) = new_actor_with_process(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008a_5e40, &args![actor, 1u8]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x320)),
            vec![vec![proc.addr(), 1]]
        );
        answer(&mut e, PROCESS_VT, 0x31c, 1);
        assert!(e.call(0x008a_5e80, &args![actor]).bool());
        answer(&mut e, PROCESS_VT, 0x31c, 0);
        assert!(!e.call(0x008a_5e80, &args![actor]).bool());
        // No process: nothing is called and the alert is off.
        let bare = new_actor(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008a_5e40, &args![bare, 1u8]);
        assert!(!e.call(0x008a_5e80, &args![bare]).bool());
        let log = calls(&mut e);
        assert!(called(&log, target(PROCESS_VT, 0x320)).is_empty());
    }

    #[test]
    fn current_package_type_tests() {
        // 008a6170: type 0x18; 008a61b0: process package of type 0x15.
        let mut e = engine();
        let (actor, _) = new_actor_with_process(&mut e);
        let package: Ptr = Ptr::new(e.mem.alloc(0x40));
        e.register(FORM_TYPE, |e, a| (e.mem.i8(a[0] + 0x20) as i32).into_ret());
        e.register(GET_CURRENT_PACKAGE, |e, a| {
            let _ = a;
            e.mem.u32(0x011d_f000).into_ret()
        });
        e.set_global(0x011d_f000, package.addr());
        e.mem.set_u8(package.addr() + 0x20, 0x18);
        assert!(e.call(0x008a_6170, &args![actor]).bool());
        e.mem.set_u8(package.addr() + 0x20, 0x17);
        assert!(!e.call(0x008a_6170, &args![actor]).bool());
        e.set_global(0x011d_f000, 0u32);
        assert!(!e.call(0x008a_6170, &args![actor]).bool());

        answer(&mut e, PROCESS_VT, 0x22c, package.addr());
        e.mem.set_u8(package.addr() + 0x20, 0x15);
        assert!(e.call(0x008a_61b0, &args![actor]).bool());
        e.mem.set_u8(package.addr() + 0x20, 0x16);
        assert!(!e.call(0x008a_61b0, &args![actor]).bool());
        answer(&mut e, PROCESS_VT, 0x22c, 0);
        assert!(!e.call(0x008a_61b0, &args![actor]).bool());
        let bare = new_actor(&mut e);
        assert!(!e.call(0x008a_61b0, &args![bare]).bool());
    }

    #[test]
    fn package_flag_bit_is_0x10000_of_word_0x1c() {
        let mut e = engine();
        let object: Ptr = Ptr::new(e.mem.alloc(0x40));
        assert!(!e.call(0x008a_6290, &args![object]).bool());
        e.mem.set_u32(object.addr() + 0x1c, 0x1_0000);
        assert!(e.call(0x008a_6290, &args![object]).bool());
        e.mem.set_u32(object.addr() + 0x1c, 0xffff);
        assert!(!e.call(0x008a_6290, &args![object]).bool());
    }

    #[test]
    fn package_table_check() {
        let mut e = engine();
        let (actor, _) = new_actor_with_process(&mut e);
        let package: Ptr = Ptr::new(e.mem.alloc(0x40));
        e.register(GET_CURRENT_PACKAGE, |e, _| {
            e.mem.u32(0x011d_f000).into_ret()
        });
        e.register(0x0096_11e0, |e, a| e.mem.u32(a[0] + 0x18).into_ret());
        e.set_global(0x011d_f000, package.addr());
        // Table 1 holds the words (3, 0xd, 5).
        let table = floats(&mut e, &[]);
        e.mem.set_u32(table.addr(), 3);
        e.mem.set_u32(table.addr() + 4, 0xd);
        e.mem.set_u32(table.addr() + 8, 5);
        e.set_global(PACKAGE_TYPE_TABLE + 4, table.addr());
        e.mem.set_u32(package.addr() + 0x18, 1);
        // Process method +0x280 picks the entry.
        answer(&mut e, PROCESS_VT, 0x280, 1);
        assert!(e.call(0x008a_6210, &args![actor]).bool());
        answer(&mut e, PROCESS_VT, 0x280, 2);
        assert!(!e.call(0x008a_6210, &args![actor]).bool());
        // The flag in the package overrides the table.
        e.mem.set_u32(package.addr() + 0x1c, 0x1_0000);
        assert!(e.call(0x008a_6210, &args![actor]).bool());
        // Index -1 and no package are false.
        e.mem.set_u32(package.addr() + 0x18, u32::MAX);
        assert!(!e.call(0x008a_6210, &args![actor]).bool());
        e.set_global(0x011d_f000, 0u32);
        assert!(!e.call(0x008a_6210, &args![actor]).bool());
    }

    /// The addresses of `log`'s calls that are in `of`, in order.
    fn order(log: &[(u32, Vec<u32>)], of: &[u32]) -> Vec<u32> {
        log.iter()
            .map(|(a, _)| *a)
            .filter(|a| of.contains(a))
            .collect()
    }

    #[test]
    fn frame_update_runs_the_bone_relaxation_and_the_virtual_updates() {
        let mut e = engine();
        let (actor, proc) = new_actor_with_process(&mut e);
        let package: Ptr = Ptr::new(e.mem.alloc(0x40));
        let ragdoll: Ptr = Ptr::new(e.mem.alloc(0x40));
        e.set(actor, Actor::pRagdollController, ragdoll);
        answer(&mut e, PROCESS_VT, 0x22c, package.addr());
        answer(&mut e, ACTOR_VT, 0x1e4, 0x6000);
        e.register(FORM_TYPE, |_, _| 0x1c.into_ret());
        e.register(0x0045_cd60, |_, _| Ret::default());
        e.register(0x0089_d620, |_, _| 0x7000.into_ret());
        e.register(0x005f_36f0, |_, _| 1.into_ret());
        e.register(GET_PARENT_CELL, |_, _| 0x8000.into_ret());
        stub(
            &mut e,
            &[0x00c5_2960, 0x0088_8970, 0x0049_3930, 0x0054_baf0],
        );
        e.call_log = Some(vec![]);
        e.call(0x008a_5d10, &args![actor, 0.5f32]);
        let log = calls(&mut e);
        assert_eq!(called(&log, 0x00c5_2960), vec![vec![0x7000, 0]]);
        assert_eq!(called(&log, 0x0088_8970), vec![vec![actor.addr(), 0]]);
        assert_eq!(called(&log, 0x0049_3930), vec![vec![0x6000]]);
        assert_eq!(called(&log, 0x0054_baf0), vec![vec![0x8000]]);
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x268)),
            vec![vec![actor.addr(), 0.5f32.to_bits()]]
        );
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x2f8)),
            vec![vec![actor.addr(), 0.5f32.to_bits()]]
        );
        let slots = [0x268, 0x350, 0x2f8, 0x178].map(|s| target(ACTOR_VT, s));
        assert_eq!(order(&log, &slots), slots.to_vec());
        let _ = proc;
    }

    #[test]
    fn frame_update_skips_what_is_missing() {
        let mut e = engine();
        let (actor, _) = new_actor_with_process(&mut e);
        // The process is busy, so the bones stay; no animation, ragdoll or cell.
        e.register(0x0045_cd60, |_, _| 1.into_ret());
        stub(
            &mut e,
            &[
                0x00c5_2960,
                0x0088_8970,
                0x0049_3930,
                0x0054_baf0,
                GET_PARENT_CELL,
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x008a_5d10, &args![actor, 0.5f32]);
        let log = calls(&mut e);
        for addr in [0x00c5_2960, 0x0088_8970, 0x0049_3930, 0x0054_baf0] {
            assert!(called(&log, addr).is_empty(), "{addr:08x}");
        }
        assert_eq!(called(&log, target(ACTOR_VT, 0x178)).len(), 1);
    }

    /// Doubles for `SetHavokWeapon_ov2`.
    fn havok_engine() -> (Engine, Ptr<Actor>, Ptr) {
        let mut e = engine();
        let (actor, _) = new_actor_with_process(&mut e);
        let root: Ptr = Ptr::new(e.mem.alloc(0x40));
        e.register(0x008c_7aa0, |_, _| Ret::default());
        e.register(0x0043_fcd0, |_, _| 0x1111.into_ret());
        e.register(GET_CURRENT_WEAPON, |_, _| 0x2222.into_ret());
        e.register(0x0089_1350, |_, _| 0x3333.into_ret());
        e.register(0x004a_de00, |_, a| (a[0] + a[1]).into_ret());
        e.register(0x0093_1ed0, |e, a| {
            e.mem.set_u32(a[1], 0x4444);
            Ret::default()
        });
        e.register(0x004b_5260, |_, _| 0x5555.into_ret());
        e.register(0x0065_3270, |_, _| Ret::default());
        e.register(0x00c8_9540, |_, _| 0x6666.into_ret());
        stub(
            &mut e,
            &[
                0x00c8_9050,
                0x00c8_91c0,
                0x0043_7bd0,
                0x0089_d690,
                0x0087_b6b0,
            ],
        );
        e.register(0x0045_37b0, |_, _| 0x7777.into_ret());
        (e, actor, root)
    }

    #[test]
    fn havok_weapon_is_only_queued_in_the_special_state() {
        let (mut e, actor, _) = havok_engine();
        e.register(0x008c_7aa0, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008a_5f20, &args![actor, 1u8]).u32(), 0);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, 0x0087_b6b0),
            vec![vec![0x7777, actor.addr(), 1]]
        );
        assert!(called(&log, 0x00c8_9540).is_empty());
    }

    #[test]
    fn havok_weapon_creates_the_object_and_updates_it_for_mode_zero() {
        let (mut e, actor, _) = havok_engine();
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008a_5f20, &args![actor, 0u8]).u32(), 0);
        let log = calls(&mut e);
        // The node is `4ade00(root, name)`.
        let node = 0x1111 + 0x3333;
        // Mode 0 with both actor checks false: the flag is 1.
        assert_eq!(
            called(&log, 0x00c8_9540),
            vec![vec![0x4444, 0x1111, node, 1]]
        );
        assert_eq!(
            called(&log, 0x00c8_9050),
            vec![vec![0x6666, 0, 0x4444, 0x1111, 1]]
        );
    }

    #[test]
    fn havok_weapon_creates_only_for_a_nonzero_mode() {
        let (mut e, actor, _) = havok_engine();
        e.call_log = Some(vec![]);
        e.call(0x008a_5f20, &args![actor, 1u8]);
        let log = calls(&mut e);
        let node = 0x1111 + 0x3333;
        // Mode 1, no ragdoll: the flag is 0.
        assert_eq!(
            called(&log, 0x00c8_9540),
            vec![vec![0x4444, 0x1111, node, 0]]
        );
        assert!(called(&log, 0x00c8_9050).is_empty());
    }

    #[test]
    fn havok_weapon_updates_an_existing_object_and_its_children() {
        let (mut e, actor, _) = havok_engine();
        e.register(0x0065_3270, |_, a| (a[1] + 1).into_ret());
        e.register(0x0043_7bd0, |_, _| 1.into_ret());
        // A ragdoll controller whose flag byte is set makes the flag 1.
        let ragdoll: Ptr = Ptr::new(e.mem.alloc(0x40));
        e.set(actor, Actor::pRagdollController, ragdoll);
        e.register(0x0089_d690, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_5f20, &args![actor, 1u8]);
        let log = calls(&mut e);
        assert!(called(&log, 0x00c8_9540).is_empty());
        assert_eq!(
            called(&log, 0x00c8_9050),
            vec![vec![0x5556, 1, 0x4444, 0x1111, 1]]
        );
        assert_eq!(called(&log, 0x00c8_91c0), vec![vec![0x5556]]);
    }

    #[test]
    fn havok_weapon_stops_without_3d_weapon_or_node() {
        for missing in 0..3 {
            let (mut e, actor, _) = havok_engine();
            match missing {
                0 => e.register(0x0043_fcd0, |_, _| Ret::default()),
                1 => e.register(GET_CURRENT_WEAPON, |_, _| Ret::default()),
                _ => e.register(0x004a_de00, |_, _| Ret::default()),
            }
            e.call_log = Some(vec![]);
            e.call(0x008a_5f20, &args![actor, 1u8]);
            let log = calls(&mut e);
            assert!(called(&log, 0x00c8_9540).is_empty(), "{missing}");
        }
    }

    #[test]
    fn havok_weapon_of_the_player_uses_the_player_3d() {
        let (mut e, actor, _) = havok_engine();
        e.set_global(PLAYER_CHARACTER, actor.addr());
        e.register(0x0095_0bb0, |_, a| (a[0] ^ 0xff).into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_5f20, &args![actor, 1u8]);
        let log = calls(&mut e);
        assert_eq!(called(&log, 0x0095_0bb0), vec![vec![actor.addr(), 0]]);
        assert!(called(&log, 0x0043_fcd0).is_empty());
    }

    #[test]
    fn set_havok_weapon_chooses_the_mode_from_the_ragdoll() {
        let (mut e, actor, _) = havok_engine();
        e.register(0x008c_7aa0, |_, _| 1.into_ret());
        // No ragdoll: 1.
        e.call_log = Some(vec![]);
        e.call(0x008a_5eb0, &args![actor]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, 0x0087_b6b0),
            vec![vec![0x7777, actor.addr(), 1]]
        );
        // A ragdoll that answers true: 0.
        let ragdoll: Ptr = Ptr::new(e.mem.alloc(0x40));
        e.set(actor, Actor::pRagdollController, ragdoll);
        e.register(0x0055_2490, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_5eb0, &args![actor]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, 0x0087_b6b0),
            vec![vec![0x7777, actor.addr(), 0]]
        );
        // A ragdoll that answers false: 1.
        e.register(0x0055_2490, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_5eb0, &args![actor]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, 0x0087_b6b0),
            vec![vec![0x7777, actor.addr(), 1]]
        );
        // No weapon: nothing is queued.
        e.register(GET_CURRENT_WEAPON, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x008a_5eb0, &args![actor]).u32(), 0);
        assert!(called(&calls(&mut e), 0x0087_b6b0).is_empty());
    }

    const KNOB: u32 = 0x011d_f000;
    const NO_SPEAKER: Ptr = Ptr::NULL;

    /// Doubles for `Actor::Move`: a fall of 300 against the threshold 100
    /// with exponent 3, multiplier 2 gives the damage `(200 * 3) * 2`.
    fn move_engine() -> (Engine, Ptr<Actor>, Ptr) {
        let mut e = engine();
        for page in [0x011c_f000, 0x011d_0000, 0x011d_1000] {
            e.map(page, 0x1000);
        }
        let (actor, _) = new_actor_with_process(&mut e);
        e.mem.set_u32(actor.addr() + LOADED_DATA, 1);
        let moved: Ptr = Ptr::new(e.mem.alloc(0x500));
        e.set_global(KNOB, moved.addr());
        e.register(0x0093_3840, |_, _| Ret::default());
        e.register(0x0092_f260, |e, _| e.mem.u32(KNOB).into_ret());
        e.register(0x0087_cee0, |_, _| 1.into_ret());
        e.register(0x00c7_0550, |_, _| 300.0f32.into_ret());
        e.register(SETTING_VALUE_POINTER, |_, a| (a[0] + 4).into_ret());
        e.register(0x0043_d4d0, |_, a| (a[0] + 4).into_ret());
        e.register(0x004d_d130, |_, a| {
            (f32::from_bits(a[0]) as f64 * f32::from_bits(a[1]) as f64).into_ret()
        });
        e.set_global(SETTING_FALL_THRESHOLD + 4, 100.0f32);
        e.set_global(SETTING_FALL_MULT + 4, 2.0f32);
        e.set_global(SETTING_FALL_EXPONENT + 4, 3.0f32);
        e.set_global(SETTING_HEAVY_THRESHOLD + 4, 500.0f32);
        e.set_global(SETTING_LIMB_DAMAGE_MULT + 4, 0.5f32);
        e.set_global(SETTING_LIMB_DAMAGE_CHANCE + 4, 50u32);
        e.register(0x0041_81e0, |_, _| Ret::default());
        e.register(0x0056_6950, |_, _| Ret::default());
        e.register(0x0045_3a70, |_, _| 0x9999.into_ret());
        e.register(0x00ad_7550, |_, _| 0xaaaa.into_ret());
        e.register(0x0048_7f50, |_, _| 205.into_ret());
        stub(
            &mut e,
            &[
                0x0093_06d0,
                0x005f_0c40,
                0x0057_5d70,
                0x0058_6320,
                SOUND_HANDLE_INIT,
                SOUND_HANDLE_ASSIGN,
                SOUND_HANDLE_DESTRUCT,
                0x00ad_8830,
                0x0062_96b0,
                0x008a_7d50,
            ],
        );
        (e, actor, moved)
    }

    #[test]
    fn move_applies_fall_damage_and_hurts() {
        let (mut e, actor, moved) = move_engine();
        e.call_log = Some(vec![]);
        let result = e
            .call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32])
            .ptr::<()>();
        assert_eq!(result, moved);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, 0x0092_f260),
            vec![vec![actor.addr(), 0.5f32.to_bits(), 7, 1]]
        );
        // The fall passes the exponent: 200 ^ 3 stand-in, times 2.
        assert_eq!(
            called(&log, 0x004d_d130),
            vec![vec![200.0f32.to_bits(), 3.0f32.to_bits()]]
        );
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x338)),
            vec![vec![actor.addr(), 1200.0f32.to_bits(), 0, 0]]
        );
        assert_eq!(called(&log, 0x008a_7d50), vec![vec![actor.addr(), 1, 1]]);
        // The actor is not in its loaded state: no sound.
        assert!(called(&log, SOUND_HANDLE_INIT).is_empty());
    }

    #[test]
    fn move_skips_the_hurt_call_when_the_world_space_check_says_so() {
        let (mut e, actor, _) = move_engine();
        answer(&mut e, ACTOR_VT, 0x218, 1);
        e.register(0x0057_5d70, |_, _| 0x1234.into_ret());
        e.register(0x0058_6320, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32]);
        let log = calls(&mut e);
        assert!(called(&log, target(ACTOR_VT, 0x338)).is_empty());
        assert_eq!(called(&log, 0x008a_7d50).len(), 1);
        // A world space that answers true lets it through again.
        e.register(0x0058_6320, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32]);
        assert_eq!(called(&calls(&mut e), target(ACTOR_VT, 0x338)).len(), 1);
    }

    #[test]
    fn move_in_the_loaded_state_plays_the_landing_sound_and_hurts_the_legs() {
        let (mut e, actor, moved) = move_engine();
        answer(&mut e, ACTOR_VT, 0x360, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32]);
        let log = calls(&mut e);
        // 1200 is above the heavy threshold 500.
        assert_eq!(called(&log, 0x00ad_7550)[0][2..], [LAND_HEAVY_SOUND, 0x101]);
        assert_eq!(called(&log, SOUND_HANDLE_ASSIGN).len(), 1);
        assert_eq!(called(&log, 0x00ad_8830)[0][1], 0);
        assert_eq!(
            called(&log, 0x0062_96b0),
            vec![vec![moved.addr() + 0x410, 0x80]]
        );
        // Random 205 % 100 = 5 is below the chance 50: both legs, -600.
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x3ac)),
            vec![
                vec![actor.addr(), 0x1d, (-600.0f32).to_bits(), 0],
                vec![actor.addr(), 0x1e, (-600.0f32).to_bits(), 0]
            ]
        );
        // The hurt call also runs in the loaded state.
        assert_eq!(called(&log, target(ACTOR_VT, 0x338)).len(), 1);
    }

    #[test]
    fn move_light_landing_and_no_leg_damage_when_the_roll_fails() {
        let (mut e, actor, _) = move_engine();
        answer(&mut e, ACTOR_VT, 0x360, 1);
        e.set_global(SETTING_HEAVY_THRESHOLD + 4, 5000.0f32);
        e.set_global(SETTING_LIMB_DAMAGE_CHANCE + 4, 5u32);
        e.call_log = Some(vec![]);
        e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32]);
        let log = calls(&mut e);
        assert_eq!(called(&log, 0x00ad_7550)[0][2..], [LAND_LIGHT_SOUND, 0x101]);
        assert!(called(&log, target(ACTOR_VT, 0x3ac)).is_empty());
    }

    #[test]
    fn move_without_damage() {
        // Below the threshold.
        let (mut e, actor, moved) = move_engine();
        e.register(0x00c7_0550, |_, _| 50.0f32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32])
                .ptr::<()>(),
            moved
        );
        assert!(called(&calls(&mut e), 0x008a_7d50).is_empty());
        // A base form that the check flags takes the damage away.
        let (mut e, actor, moved) = move_engine();
        e.register(0x0041_81e0, |_, _| 0x1.into_ret());
        e.register(0x005f_0c40, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32])
                .ptr::<()>(),
            moved
        );
        let log = calls(&mut e);
        assert!(called(&log, 0x008a_7d50).is_empty());
        assert!(called(&log, target(ACTOR_VT, 0x338)).is_empty());
        // The mover is not on the ground.
        let (mut e, actor, moved) = move_engine();
        e.register(0x0087_cee0, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32])
                .ptr::<()>(),
            moved
        );
        assert!(called(&calls(&mut e), 0x00c7_0550).is_empty());
    }

    #[test]
    fn move_gives_up_early() {
        // In dialogue with the player, without the low flag bits.
        let (mut e, actor, _) = move_engine();
        e.register(0x0093_3840, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        assert!(e
            .call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 0x40u32])
            .ptr::<()>()
            .is_null());
        assert!(called(&calls(&mut e), 0x0092_f260).is_empty());
        // With a low flag bit it moves.
        e.call_log = Some(vec![]);
        e.call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 0x01u32]);
        assert_eq!(called(&calls(&mut e), 0x0092_f260).len(), 1);
        // No loaded data.
        let (mut e, actor, _) = move_engine();
        e.mem.set_u32(actor.addr() + LOADED_DATA, 0);
        assert!(e
            .call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32])
            .ptr::<()>()
            .is_null());
        // Nothing moved.
        let (mut e, actor, _) = move_engine();
        e.set_global(KNOB, 0u32);
        e.call_log = Some(vec![]);
        assert!(e
            .call(0x008a_62b0, &args![actor, 0.5f32, 7u32, 1u32])
            .ptr::<()>()
            .is_null());
        assert!(called(&calls(&mut e), 0x0087_cee0).is_empty());
    }

    #[test]
    fn character_controller_flag_call() {
        let mut e = engine();
        e.register(0x0062_96b0, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_6630, &args![0x1000u32]);
        assert_eq!(
            called(&calls(&mut e), 0x0062_96b0),
            vec![vec![0x1410, 0x80]]
        );
    }

    /// An actor whose current package is a fresh block of the given type.
    fn actor_with_package(e: &mut Engine, kind: u8) -> (Ptr<Actor>, Ptr, Ptr) {
        let (actor, proc) = new_actor_with_process(e);
        let package: Ptr = Ptr::new(e.mem.alloc(0x100));
        e.mem.set_u8(package.addr() + 0x20, kind);
        e.register(FORM_TYPE, |e, a| (e.mem.i8(a[0] + 0x20) as i32).into_ret());
        e.register(GET_CURRENT_PACKAGE, |e, _| e.mem.u32(KNOB).into_ret());
        e.set_global(KNOB, package.addr());
        (actor, proc, package)
    }

    #[test]
    fn fleeing_by_package_type() {
        let mut e = engine();
        let (actor, _, package) = actor_with_package(&mut e, 0x16);
        e.set_global(PLAYER_CHARACTER, 0u32);
        // A flee package: fleeing, whatever the flag.
        assert!(e.call(0x008a_6650, &args![actor, 0u8]).bool());
        assert!(e.call(0x008a_6650, &args![actor, 1u8]).bool());
        // Package type 0xa counts only without the flag.
        e.mem.set_u8(package.addr() + 0x20, 0xa);
        assert!(e.call(0x008a_6650, &args![actor, 0u8]).bool());
        assert!(!e.call(0x008a_6650, &args![actor, 1u8]).bool());
        // Type 0x13 counts while the process method +0x280 is 1.
        e.mem.set_u8(package.addr() + 0x20, 0x13);
        answer(&mut e, PROCESS_VT, 0x280, 1);
        assert!(e.call(0x008a_6650, &args![actor, 0u8]).bool());
        answer(&mut e, PROCESS_VT, 0x280, 2);
        assert!(!e.call(0x008a_6650, &args![actor, 0u8]).bool());
        // No package.
        e.set_global(KNOB, 0u32);
        assert!(!e.call(0x008a_6650, &args![actor, 0u8]).bool());
    }

    #[test]
    fn fleeing_follows_the_combat_controller() {
        let mut e = engine();
        let (actor, _, _) = actor_with_package(&mut e, 0x05);
        e.register(0x0098_1990, |_, a| (a[0] == 0x4242).into_ret());
        // No controller.
        assert!(!e.call(0x008a_6650, &args![actor, 0u8]).bool());
        // A controller that is fleeing.
        answer(&mut e, ACTOR_VT, 0x428, 0x4242);
        assert!(e.call(0x008a_6650, &args![actor, 0u8]).bool());
        // One that is not.
        answer(&mut e, ACTOR_VT, 0x428, 0x4243);
        assert!(!e.call(0x008a_6650, &args![actor, 0u8]).bool());
    }

    #[test]
    fn the_player_with_a_flee_package_is_not_fleeing_and_the_process_is_told() {
        let mut e = engine();
        let (actor, proc, _) = actor_with_package(&mut e, 0x16);
        e.set_global(PLAYER_CHARACTER, actor.addr());
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008a_6650, &args![actor, 0u8]).bool());
        let log = calls(&mut e);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x234)),
            vec![vec![proc.addr()]]
        );
    }

    /// Doubles for `RemoveFleeTarget`: a flee package whose "has targets"
    /// byte (+0x98 + 0) is set by the removal.
    fn remove_target_engine(kind: u8) -> (Engine, Ptr<Actor>, Ptr) {
        let mut e = engine();
        let (actor, _, package) = actor_with_package(&mut e, kind);
        e.register(0x0082_56d0, |e, a| e.mem.u8(a[0]).into_ret());
        e.register(0x009f_1350, |e, a| {
            e.mem.set_u8(a[0] + 0x98, 1);
            Ret::default()
        });
        e.register(IS_ALIVE_BODY, |_, _| Ret::default());
        stub(&mut e, &[0x0088_1680]);
        (e, actor, package)
    }

    #[test]
    fn removing_a_flee_target_stops_the_actor() {
        let (mut e, actor, package) = remove_target_engine(0x16);
        e.call_log = Some(vec![]);
        e.call(0x008a_6730, &args![actor, 0x77u32]);
        let log = calls(&mut e);
        assert_eq!(called(&log, 0x009f_1350), vec![vec![package.addr(), 0x77]]);
        // Not a living body: the interrupt package ends.
        assert_eq!(called(&log, 0x0088_1680), vec![vec![actor.addr(), 0]]);
        assert!(called(&log, target(ACTOR_VT, 0x434)).is_empty());

        // A body that `00493bb0` accepts is stopped through +0x434 instead.
        let (mut e, actor, _) = remove_target_engine(0x16);
        e.register(IS_ALIVE_BODY, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_6730, &args![actor, 0x77u32]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x434)),
            vec![vec![actor.addr(), 0]]
        );
        assert!(called(&log, 0x0088_1680).is_empty());
    }

    #[test]
    fn removing_a_flee_target_does_nothing_when_not_applicable() {
        // Not a flee package.
        let (mut e, actor, _) = remove_target_engine(0x15);
        e.call_log = Some(vec![]);
        e.call(0x008a_6730, &args![actor, 0x77u32]);
        assert!(called(&calls(&mut e), 0x009f_1350).is_empty());
        // Already flagged.
        let (mut e, actor, package) = remove_target_engine(0x16);
        e.mem.set_u8(package.addr() + 0x98, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6730, &args![actor, 0x77u32]);
        assert!(called(&calls(&mut e), 0x009f_1350).is_empty());
        // The removal does not flag it: the actor is left alone.
        let (mut e, actor, _) = remove_target_engine(0x16);
        e.register(0x009f_1350, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_6730, &args![actor, 0x77u32]);
        let log = calls(&mut e);
        assert!(called(&log, 0x0088_1680).is_empty());
    }

    #[test]
    fn list_check_looks_at_the_sub_object() {
        let mut e = engine();
        e.register(0x0082_56d0, |_, a| a[0].into_ret());
        e.call_log = Some(vec![]);
        assert!(e.call(0x008a_67d0, &args![0x1000u32]).bool());
        assert_eq!(called(&calls(&mut e), 0x0082_56d0), vec![vec![0x1098]]);
    }

    #[test]
    fn talking_is_the_flag_or_the_process_answer() {
        let mut e = engine();
        let (actor, proc) = new_actor_with_process(&mut e);
        answer(&mut e, PROCESS_VT, 0x4b8, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008a_67f0, &args![actor]).bool());
        let log = calls(&mut e);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x4b8)),
            vec![vec![proc.addr(), actor.addr()]]
        );
        answer(&mut e, PROCESS_VT, 0x4b8, 0);
        assert!(!e.call(0x008a_67f0, &args![actor]).bool());
        e.set(actor, Actor::bTalkingToPlayer, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x008a_67f0, &args![actor]).bool());
        assert!(called(&calls(&mut e), target(PROCESS_VT, 0x4b8)).is_empty());
        let bare = new_actor(&mut e);
        assert!(!e.call(0x008a_67f0, &args![bare]).bool());
    }

    #[test]
    fn continuing_package_for_the_player_is_the_process_answer() {
        let mut e = engine();
        let (actor, _) = new_actor_with_process(&mut e);
        answer(&mut e, PROCESS_VT, 0x4e0, 1);
        assert!(e.call(0x008a_69d0, &args![actor]).bool());
        answer(&mut e, PROCESS_VT, 0x4e0, 0);
        assert!(!e.call(0x008a_69d0, &args![actor]).bool());
        let bare = new_actor(&mut e);
        assert!(!e.call(0x008a_69d0, &args![bare]).bool());
    }

    #[test]
    fn heading_is_set_only_in_modes_zero_and_four() {
        for (mode, runs) in [(0u32, true), (4, true), (1, false), (5, false)] {
            let mut e = engine();
            let actor = new_actor(&mut e);
            e.register(0x004f_8960, |e, _| e.mem.u32(KNOB).into_ret());
            e.set_global(KNOB, mode);
            stub(&mut e, &[0x0093_1b60]);
            e.call_log = Some(vec![]);
            e.call(0x008a_6a00, &args![actor, 1.5f32]);
            let log = calls(&mut e);
            let seen = called(&log, 0x0093_1b60);
            if runs {
                assert_eq!(seen, vec![vec![actor.addr(), 1.5f32.to_bits()]], "{mode}");
            } else {
                assert!(seen.is_empty(), "{mode}");
            }
        }
    }

    /// The flags word `008846e0` answers for an actor in these tests.
    const ACTOR_FLAGS_WORD: u32 = 0x1b0;

    /// Doubles for the weapon-drawn change (`008a6840`, `008a6970`).
    fn draw_engine() -> (Engine, Ptr<Actor>, Ptr) {
        let mut e = engine();
        e.map(0x011f_2000, 0x1000);
        let (actor, proc) = new_actor_with_process(&mut e);
        e.register(0x0088_46e0, |e, a| {
            e.mem.u32(a[0] + ACTOR_FLAGS_WORD).into_ret()
        });
        e.register(0x008b_bc10, |e, _| e.mem.u32(KNOB).into_ret());
        e.register(GET_ANIMATION, |_, _| 0x500.into_ret());
        e.register(0x005f_2370, |_, _| 0x66.into_ret());
        e.register(0x0049_4710, |e, _| e.mem.u32(KNOB + 4).into_ret());
        e.register(0x0057_15d0, |_, _| 0x77.into_ret());
        e.register(0x0044_7330, |_, _| 0x99.into_ret());
        e.set_global(MODEL_LOADER, 0x88u32);
        e.register(0x0044_ddc0, |e, a| e.mem.u32(a[0] + 8).into_ret());
        stub(&mut e, &[0x0049_0400, 0x009c_8950]);
        (e, actor, proc)
    }

    #[test]
    fn drawing_a_weapon_loads_the_animation_group_once() {
        let (mut e, actor, proc) = draw_engine();
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 1u8]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x340)),
            vec![vec![actor.addr(), 0xd]]
        );
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x450)),
            vec![vec![proc.addr(), 1]]
        );
        assert_eq!(called(&log, 0x005f_2370), vec![vec![0, 1, 0x18, 0]]);
        assert_eq!(called(&log, 0x0049_4710), vec![vec![0x500, 0x66]]);
        assert_eq!(called(&log, 0x0057_15d0), vec![vec![actor.addr(), 1, 0, 1]]);
        assert_eq!(called(&log, 0x0044_7330), vec![vec![0x88, 0x77]]);
        assert_eq!(called(&log, 0x0049_0400), vec![vec![0x500, 0x99]]);
        // Already loaded.
        e.set_global(KNOB + 4, 1u32);
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 1u8]);
        assert!(called(&calls(&mut e), 0x0049_0400).is_empty());
    }

    #[test]
    fn drawing_does_not_load_for_holstering_the_player_or_an_armed_process() {
        // Holstering (current state 1).
        let (mut e, actor, _) = draw_engine();
        answer(&mut e, PROCESS_VT, 0x44c, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 0u8]);
        let log = calls(&mut e);
        assert_eq!(called(&log, target(PROCESS_VT, 0x450)).len(), 1);
        assert!(called(&log, 0x0049_0400).is_empty());
        // The player.
        let (mut e, actor, _) = draw_engine();
        e.set_global(PLAYER_CHARACTER, actor.addr());
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 1u8]);
        let log = calls(&mut e);
        assert_eq!(called(&log, target(PROCESS_VT, 0x450)).len(), 1);
        assert!(called(&log, 0x0049_0400).is_empty());
        // A process that already has something (method +0x148).
        let (mut e, actor, _) = draw_engine();
        answer(&mut e, PROCESS_VT, 0x148, 0x42);
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 1u8]);
        assert!(called(&calls(&mut e), 0x0049_0400).is_empty());
    }

    #[test]
    fn drawing_is_refused_in_the_blocked_states() {
        // Same state: nothing.
        let (mut e, actor, _) = draw_engine();
        answer(&mut e, PROCESS_VT, 0x44c, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 1u8]);
        assert!(called(&calls(&mut e), target(ACTOR_VT, 0x340)).is_empty());
        // Flag 0x800 blocks drawing but not holstering.
        let (mut e, actor, _) = draw_engine();
        e.mem.set_u32(actor.addr() + ACTOR_FLAGS_WORD, 0x800);
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 1u8]);
        assert!(called(&calls(&mut e), target(ACTOR_VT, 0x340)).is_empty());
        // The player with the iron sights up.
        let (mut e, actor, _) = draw_engine();
        e.set_global(PLAYER_CHARACTER, actor.addr());
        e.set_global(KNOB, 1u32);
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![actor, 1u8]);
        assert!(called(&calls(&mut e), target(ACTOR_VT, 0x340)).is_empty());
        // No process.
        let (mut e, _, _) = draw_engine();
        let bare = new_actor(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008a_6840, &args![bare, 1u8]);
        assert!(called(&calls(&mut e), target(ACTOR_VT, 0x340)).is_empty());
    }

    #[test]
    fn drawn_state_is_the_process_answer_unless_the_flag_blocks_it() {
        let (mut e, actor, _) = draw_engine();
        answer(&mut e, PROCESS_VT, 0x44c, 1);
        assert!(e.call(0x008a_6970, &args![actor]).bool());
        answer(&mut e, PROCESS_VT, 0x44c, 0);
        assert!(!e.call(0x008a_6970, &args![actor]).bool());
        // Flag 0x800: false; the player in VATS mode 4 quits the playback.
        e.mem.set_u32(actor.addr() + ACTOR_FLAGS_WORD, 0x800);
        answer(&mut e, PROCESS_VT, 0x44c, 1);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008a_6970, &args![actor]).bool());
        assert!(called(&calls(&mut e), 0x009c_8950).is_empty());
        e.set_global(PLAYER_CHARACTER, actor.addr());
        e.mem.set_u32(0x011f_2250 + 8, 4);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x008a_6970, &args![actor]).bool());
        assert_eq!(
            called(&calls(&mut e), 0x009c_8950),
            vec![vec![0x011f_2250, 0, 0]]
        );
        e.mem.set_u32(0x011f_2250 + 8, 3);
        e.call_log = Some(vec![]);
        e.call(0x008a_6970, &args![actor]);
        assert!(called(&calls(&mut e), 0x009c_8950).is_empty());
    }

    /// Doubles for `UnlockLockDoorsProcedure`: the actor's method +0x1f4 returns
    /// 1, then 2, ...
    fn doors_engine() -> (Engine, Ptr<Actor>, Ptr, Ptr) {
        let mut e = engine();
        let (actor, proc) = new_actor_with_process(&mut e);
        let package = new_object(&mut e);
        answer(&mut e, PROCESS_VT, 0x22c, package.addr());
        let mut n = 0;
        e.register_double(target(ACTOR_VT, 0x1f4), move |_, _| {
            n += 1;
            Ret {
                eax: n,
                ..Ret::default()
            }
        });
        e.set_global(0x011d_df38, 0x5000u32);
        e.set_global(DATA_HANDLER, 0x7000u32);
        e.set_global(SEARCH_RADIUS, 5000.0f32);
        e.set_global(MINUS_ONE, -1.0f32);
        e.register(0x0042_ce10, |_, _| Ret::default());
        e.register(GET_PARENT_CELL, |_, _| 0x8000.into_ret());
        stub(
            &mut e,
            &[
                0x0067_8610,
                0x0053_94a0,
                0x0067_0ed0,
                0x0067_0f40,
                0x0067_0f60,
                ENUM_REFERENCES_CLOSE_TO_POINT,
            ],
        );
        answer(&mut e, OBJECT_VT, 0x13c, 0);
        (e, actor, proc, package)
    }

    #[test]
    fn doors_search_with_the_door_callback_then_ask_the_package() {
        let (mut e, actor, _, package) = doors_engine();
        e.register(0x0053_94a0, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        let log = calls(&mut e);
        let radius = 5000.0f32.to_bits();
        assert_eq!(
            called(&log, ENUM_REFERENCES_CLOSE_TO_POINT),
            vec![vec![
                0x7000,
                0x8000,
                2,
                radius,
                1,
                radius,
                PACKAGE_CALLBACK_B,
                actor.addr()
            ]]
        );
        // The package is asked with the actor, 0, -1.0 and 0; it refuses.
        assert_eq!(
            called(&log, target(OBJECT_VT, 0x13c)),
            vec![vec![
                package.addr(),
                actor.addr(),
                0,
                (-1.0f32).to_bits(),
                0
            ]]
        );
    }

    #[test]
    fn doors_second_search_depends_on_the_package_answer() {
        let (mut e, actor, proc, _) = doors_engine();
        e.register(0x0067_0ed0, |_, _| 1.into_ret());
        e.register(0x0067_0f40, |_, _| 1.into_ret());
        answer(&mut e, OBJECT_VT, 0x13c, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        let log = calls(&mut e);
        let searches = called(&log, ENUM_REFERENCES_CLOSE_TO_POINT);
        assert_eq!(searches.len(), 2);
        assert_eq!(searches[0][6], PACKAGE_CALLBACK_A);
        assert_eq!(searches[1][6], PACKAGE_CALLBACK_B);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x1c0)),
            vec![vec![proc.addr(), 1]]
        );

        // The second kind of search runs without the process call.
        let (mut e, actor, _, _) = doors_engine();
        e.register(0x0067_0f60, |_, _| 1.into_ret());
        answer(&mut e, OBJECT_VT, 0x13c, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        let log = calls(&mut e);
        let searches = called(&log, ENUM_REFERENCES_CLOSE_TO_POINT);
        assert_eq!(searches.len(), 1);
        assert_eq!(searches[0][6], PACKAGE_CALLBACK_A);
        assert!(called(&log, target(PROCESS_VT, 0x1c0)).is_empty());

        // The process says it is busy (method +0x1bc): no second search.
        let (mut e, actor, _, _) = doors_engine();
        e.register(0x0067_0f40, |_, _| 1.into_ret());
        answer(&mut e, OBJECT_VT, 0x13c, 1);
        answer(&mut e, PROCESS_VT, 0x1bc, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        assert!(called(&calls(&mut e), ENUM_REFERENCES_CLOSE_TO_POINT).is_empty());
    }

    #[test]
    fn doors_gates() {
        // No process.
        let (mut e, actor, _, _) = doors_engine();
        e.set(actor, Actor::pCurrentProcess, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        assert!(called(&calls(&mut e), target(OBJECT_VT, 0x13c)).is_empty());
        // The actor's method +0x22c (with 0) holds.
        let (mut e, actor, _, _) = doors_engine();
        answer(&mut e, ACTOR_VT, 0x22c, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        assert!(called(&calls(&mut e), target(OBJECT_VT, 0x13c)).is_empty());
        // The global object's check holds.
        let (mut e, actor, _, _) = doors_engine();
        e.register(0x0042_ce10, |_, a| (a[0] == 0x5000).into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        assert!(called(&calls(&mut e), target(OBJECT_VT, 0x13c)).is_empty());
        // No package, no cell, an interrupt package.
        let (mut e, actor, _, _) = doors_engine();
        answer(&mut e, PROCESS_VT, 0x22c, 0);
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        assert!(called(&calls(&mut e), target(OBJECT_VT, 0x13c)).is_empty());
        let (mut e, actor, _, _) = doors_engine();
        e.register(GET_PARENT_CELL, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        assert!(called(&calls(&mut e), target(OBJECT_VT, 0x13c)).is_empty());
        let (mut e, actor, _, _) = doors_engine();
        e.register(0x0067_8610, |_, _| 1.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_6a40, &args![actor]);
        assert!(called(&calls(&mut e), target(OBJECT_VT, 0x13c)).is_empty());
    }

    /// Makes a fake function answer the given values in turn, then the last
    /// one again.
    fn answer_in_turn(e: &mut Engine, at: u32, slot: u32, values: &[u32]) {
        let values = values.to_vec();
        let mut next = 0;
        e.register_double(target(at, slot), move |_, _| {
            let value = values[next.min(values.len() - 1)];
            next += 1;
            Ret {
                eax: value,
                ..Ret::default()
            }
        });
    }

    /// Doubles for `Actor::EvaluatePackage`.
    fn evaluate_engine() -> (Engine, Ptr<Actor>, Ptr, Ptr) {
        let mut e = engine();
        let (actor, proc) = new_actor_with_process(&mut e);
        let package = new_object(&mut e);
        e.register(CALENDAR_GET_HOUR, |_, _| 10.5f32.into_ret());
        e.set_global(ONE, 1.0f64);
        e.set_global(DATA_HANDLER, 0x7000u32);
        e.set_global(SEARCH_RADIUS, 5000.0f32);
        e.register(IS_ALIVE_BODY, |_, _| Ret::default());
        e.register(0x0045_cd60, |_, _| 1.into_ret());
        e.register(0x005d_43c0, |_, a| (a[0] + 0x44).into_ret());
        e.register(GET_PARENT_CELL, |_, _| 0x8000.into_ret());
        e.register(FORM_TYPE, |e, a| (e.mem.i8(a[0] + 0x20) as i32).into_ret());
        e.register(GET_ANIMATION, |_, _| 0x500.into_ret());
        stub(
            &mut e,
            &[
                HOUR_TO_PROCESS,
                STOP_MOVING,
                ENUM_REFERENCES_CLOSE_TO_POINT,
                0x0067_0f10,
                0x0067_0ef0,
                0x0042_2690,
                0x0088_14b0,
                0x0088_1680,
                0x0088_d640,
                0x005a_c750,
                0x0044_1b00,
                GET_SAVE_FORM,
                0x0040_1170,
                0x0087_faa0,
                0x0067_1d10,
                0x0067_2930,
                0x0093_1fb0,
                0x0049_6080,
                0x0049_8910,
                0x0049_1040,
            ],
        );
        answer(&mut e, PROCESS_VT, 0x22c, package.addr());
        (e, actor, proc, package)
    }

    #[test]
    fn evaluating_is_buffered_when_not_immediate() {
        let (mut e, actor, proc, _) = evaluate_engine();
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 0u8, 1u8]);
        let log = calls(&mut e);
        assert_eq!(e.get(actor, Actor::bEVPBuffered), 1);
        assert_eq!(e.get(actor, Actor::bResetAI), 1);
        // The process is told the hour before now: 10.5 - 1.
        assert_eq!(
            called(&log, HOUR_TO_PROCESS),
            vec![vec![proc.addr(), 9.5f32.to_bits()]]
        );
        // Without a process only the flags are set.
        let bare = new_actor(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![bare, 0u8, 0u8]);
        assert!(called(&calls(&mut e), HOUR_TO_PROCESS).is_empty());
        assert_eq!(e.get(bare, Actor::bEVPBuffered), 1);
        assert_eq!(e.get(bare, Actor::bResetAI), 0);
    }

    #[test]
    fn evaluating_does_nothing_when_the_actor_vetoes_it() {
        let (mut e, actor, _, _) = evaluate_engine();
        answer(&mut e, ACTOR_VT, 0x22c, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 1u8, 1u8]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x22c)),
            vec![vec![actor.addr(), 1]]
        );
        assert_eq!(log.len(), 2);
        assert_eq!(e.get(actor, Actor::bEVPBuffered), 0);
    }

    #[test]
    fn evaluating_without_reset_restarts_the_package_if_nothing_is_pending() {
        let (mut e, actor, proc, _) = evaluate_engine();
        e.set(actor, Actor::bEVPBuffered, 1);
        e.set(actor, Actor::bResetAI, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 1u8, 0u8]);
        let log = calls(&mut e);
        assert_eq!(called(&log, STOP_MOVING), vec![vec![actor.addr()]]);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x524)),
            vec![vec![proc.addr(), actor.addr()]]
        );
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x118)),
            vec![vec![proc.addr(), 1]]
        );
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x24)),
            vec![vec![proc.addr(), actor.addr(), 1]]
        );
        assert_eq!(
            called(&log, HOUR_TO_PROCESS),
            vec![vec![proc.addr(), 9.5f32.to_bits()]]
        );
        assert_eq!(e.get(actor, Actor::bEVPBuffered), 0);
        assert_eq!(e.get(actor, Actor::bResetAI), 0);

        // A pending change (method +0x214 = 7) leaves everything alone.
        let (mut e, actor, _, _) = evaluate_engine();
        e.set(actor, Actor::bEVPBuffered, 1);
        answer(&mut e, ACTOR_VT, 0x214, 7);
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 1u8, 0u8]);
        assert!(called(&calls(&mut e), STOP_MOVING).is_empty());
        assert_eq!(e.get(actor, Actor::bEVPBuffered), 1);
        // 4 and 9 do not count as pending.
        for kind in [0, 4, 9] {
            let (mut e, actor, _, _) = evaluate_engine();
            answer(&mut e, ACTOR_VT, 0x214, kind);
            e.call_log = Some(vec![]);
            e.call(0x008a_6ce0, &args![actor, 1u8, 0u8]);
            assert_eq!(called(&calls(&mut e), STOP_MOVING).len(), 1, "{kind}");
        }
    }

    #[test]
    fn evaluating_with_reset_switches_the_package() {
        let (mut e, actor, proc, package) = evaluate_engine();
        // The old package has type 4; the editor package differs from the new.
        let old: Ptr = Ptr::new(e.mem.alloc(0x40));
        e.mem.set_u8(old.addr() + 0x20, 4);
        let new_package: Ptr = Ptr::new(e.mem.alloc(0x40));
        let tail: Ptr = Ptr::new(e.mem.alloc(0x40));
        let editor: Ptr = Ptr::new(e.mem.alloc(0x40));
        let follower = new_object(&mut e);
        answer_in_turn(
            &mut e,
            PROCESS_VT,
            0x22c,
            &[
                package.addr(),
                package.addr(),
                old.addr(),
                old.addr(),
                new_package.addr(),
                tail.addr(),
            ],
        );
        answer(&mut e, PROCESS_VT, 0x360, 1);
        answer(&mut e, PROCESS_VT, 0x128, follower.addr());
        answer(&mut e, OBJECT_VT, 0x100, 1);
        e.register(0x0067_0f10, |_, _| 1.into_ret());
        e.register(0x0067_0ef0, |_, _| 1.into_ret());
        e.register(0x0088_14b0, move |_, _| 0x4a00.into_ret());
        e.register(0x0067_1d10, |_, _| 1.into_ret());
        e.register(0x0067_2930, |_, _| 3.into_ret());
        e.register(GET_SAVE_FORM, |_, _| 0x4b00.into_ret());
        let _ = editor;
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 1u8, 1u8]);
        let log = calls(&mut e);
        // Both reference searches ran, with the two callbacks.
        let searches = called(&log, ENUM_REFERENCES_CLOSE_TO_POINT);
        assert_eq!(searches.len(), 2);
        assert_eq!(searches[0][6], PACKAGE_CALLBACK_A);
        assert_eq!(searches[1][6], PACKAGE_CALLBACK_B);
        // The follower is removed from the actor's extra data list.
        assert_eq!(
            called(&log, 0x0042_2690),
            vec![vec![follower.addr() + 0x44, actor.addr()]]
        );
        // The editor package is flagged and told to the process.
        assert_eq!(
            called(&log, 0x005a_c750),
            vec![vec![0x4a00, actor.addr() + 0x44, 0x800]]
        );
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x59c)),
            vec![vec![proc.addr(), actor.addr(), 0x4a00]]
        );
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x5a8)),
            vec![vec![proc.addr(), 0]]
        );
        // The old package (type 4) makes the save form's type be looked at.
        assert_eq!(called(&log, 0x0040_1170), vec![vec![0x4b00]]);
        // The tail passes the package's initial target count.
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x15c)),
            vec![vec![proc.addr(), 3]]
        );
        assert_eq!(called(&log, HOUR_TO_PROCESS).len(), 1);
        // Order of the main steps.
        let steps = [
            target(PROCESS_VT, 0x20c),
            0x0088_1680,
            target(PROCESS_VT, 0x234),
            0x0088_d640,
            target(PROCESS_VT, 0x524),
            target(PROCESS_VT, 0x4ec),
            target(PROCESS_VT, 0x24),
            0x005a_c750,
            0x0087_faa0,
            target(PROCESS_VT, 0x15c),
        ];
        assert_eq!(order(&log, &steps), steps.to_vec());
        assert_eq!(called(&log, 0x0088_1680), vec![vec![actor.addr(), 0]]);
        assert_eq!(e.get(actor, Actor::bEVPBuffered), 0);
    }

    #[test]
    fn evaluating_with_reset_skips_the_editor_package_when_it_is_the_new_one() {
        let (mut e, actor, _, package) = evaluate_engine();
        e.register(0x0088_14b0, |e, _| e.mem.u32(KNOB).into_ret());
        e.set_global(KNOB, package.addr());
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 1u8, 1u8]);
        let log = calls(&mut e);
        assert!(called(&log, 0x005a_c750).is_empty());
        assert!(called(&log, target(PROCESS_VT, 0x59c)).is_empty());
    }

    #[test]
    fn evaluating_with_reset_finishes_by_the_package_change_kind() {
        // Kind 9 and kind 4 with a package: method +0x418.
        for kind in [9u32, 4] {
            let (mut e, actor, _, _) = evaluate_engine();
            answer(&mut e, ACTOR_VT, 0x214, kind);
            e.call_log = Some(vec![]);
            e.call(0x008a_6ce0, &args![actor, 1u8, 1u8]);
            assert_eq!(
                called(&calls(&mut e), target(ACTOR_VT, 0x418)).len(),
                1,
                "{kind}"
            );
        }
        // Kind 4 without a package: nothing more.
        let (mut e, actor, _, package) = evaluate_engine();
        answer(&mut e, ACTOR_VT, 0x214, 4);
        // Calls 1 to 5 see a package; the check of the kind is the sixth.
        answer_in_turn(
            &mut e,
            PROCESS_VT,
            0x22c,
            &[
                package.addr(),
                package.addr(),
                package.addr(),
                package.addr(),
                package.addr(),
                0,
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 1u8, 1u8]);
        assert!(called(&calls(&mut e), target(ACTOR_VT, 0x418)).is_empty());
        // Any other kind: the chase bip is cleared and the animation reset.
        let (mut e, actor, proc, _) = evaluate_engine();
        answer(&mut e, ACTOR_VT, 0x214, 5);
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![actor, 1u8, 1u8]);
        let log = calls(&mut e);
        assert!(called(&log, target(ACTOR_VT, 0x418)).is_empty());
        assert_eq!(called(&log, 0x0093_1fb0), vec![vec![actor.addr(), 0]]);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x4c0)),
            vec![vec![proc.addr(), actor.addr(), 0, 0, 0x7f]]
        );
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x3ec)),
            vec![vec![proc.addr(), u32::MAX, 0]]
        );
        assert_eq!(called(&log, 0x0049_6080), vec![vec![0x500, 0x14, 0]]);
        assert_eq!(called(&log, 0x0049_8910), vec![vec![0x500, 1, 0]]);
    }

    #[test]
    fn evaluating_with_reset_but_no_process_only_clears_the_flags() {
        let (mut e, _, _, _) = evaluate_engine();
        let bare = new_actor(&mut e);
        e.set(bare, Actor::bEVPBuffered, 1);
        e.set(bare, Actor::bResetAI, 1);
        e.call_log = Some(vec![]);
        e.call(0x008a_6ce0, &args![bare, 1u8, 1u8]);
        let log = calls(&mut e);
        assert!(called(&log, HOUR_TO_PROCESS).is_empty());
        assert_eq!(e.get(bare, Actor::bEVPBuffered), 0);
        assert_eq!(e.get(bare, Actor::bResetAI), 0);
    }

    /// Doubles for `SetAnimAction`.
    fn anim_action_engine() -> (Engine, Ptr<Actor>, Ptr) {
        let mut e = engine();
        let (actor, proc) = new_actor_with_process(&mut e);
        e.register(GET_ANIMATION, |_, _| 0x500.into_ret());
        e.register(0x0049_1040, |_, _| 1.into_ret());
        e.register(0x0048_f7f0, |_, a| a[0].into_ret());
        e.register(0x005f_2420, |_, a| a[0].into_ret());
        e.register(0x0043_01b0, |_, _| 0x31.into_ret());
        e.register(0x005f_2440, |_, a| {
            (if a[0] == 0x31 { 0xaa } else { 0 }).into_ret()
        });
        e.register(0x0044_ddc0, |_, a| (a[0] + 1).into_ret());
        e.register(0x0064_50c0, |_, a| (a[0] == 0x1001).into_ret());
        e.register(0x0095_0a60, |_, _| 0x600.into_ret());
        stub(&mut e, &[0x0049_6080]);
        (e, actor, proc)
    }

    #[test]
    fn anim_action_clears_the_group_for_the_group_type_and_tells_the_process() {
        let (mut e, actor, proc) = anim_action_engine();
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 3i32, 0u32]);
        let log = calls(&mut e);
        assert_eq!(called(&log, 0x0049_6080), vec![vec![0x500, 2, 0]]);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x3ec)),
            vec![vec![proc.addr(), 3, 0]]
        );
        // Another group type: not cleared.
        e.register(0x005f_2440, |_, _| 5.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 3i32, 0u32]);
        assert!(called(&calls(&mut e), 0x0049_6080).is_empty());
    }

    #[test]
    fn anim_action_blocked_cases_still_tell_the_process() {
        // The current action is 4.
        let (mut e, actor, proc) = anim_action_engine();
        answer(&mut e, PROCESS_VT, 0x3e4, 4);
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 3i32, 0u32]);
        let log = calls(&mut e);
        assert!(called(&log, 0x0049_6080).is_empty());
        assert_eq!(called(&log, target(PROCESS_VT, 0x3ec)).len(), 1);
        // The new action is 7.
        let (mut e, actor, _) = anim_action_engine();
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 7i32, 0u32]);
        assert!(called(&calls(&mut e), 0x0049_6080).is_empty());
        // The animation lacks group 2.
        let (mut e, actor, _) = anim_action_engine();
        e.register(0x0049_1040, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 3i32, 0u32]);
        assert!(called(&calls(&mut e), 0x0049_6080).is_empty());
        // The sequence's table entry is 2.
        let (mut e, actor, _) = anim_action_engine();
        e.set_global(ANIM_GROUP_TABLE + 3 * 0x24, 2u32);
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 3i32, 3u32]);
        assert!(called(&calls(&mut e), 0x0049_6080).is_empty());
        // Another value in the entry does not block.
        e.set_global(ANIM_GROUP_TABLE + 3 * 0x24, 1u32);
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 3i32, 3u32]);
        assert_eq!(called(&calls(&mut e), 0x0049_6080).len(), 1);
        // No animation, no process.
        let (mut e, _, _) = anim_action_engine();
        e.register(GET_ANIMATION, |_, _| Ret::default());
        let bare = new_actor(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![bare, 3i32, 0u32]);
        let log = calls(&mut e);
        assert!(called(&log, target(PROCESS_VT, 0x3ec)).is_empty());
        let _ = proc;
    }

    #[test]
    fn anim_action_weapon_actions_need_the_weapon_form_to_allow_them() {
        // Process method +0x148 gives the weapon, whose form is `weapon + 1`;
        // the form 0x1001 is allowed.
        for (weapon, action, cleared) in [
            (0x1000u32, 5i32, true),
            (0x2000, 5, false),
            (0x2000, 2, false),
            (0x2000, 6, false),
            (0x2000, 3, true),
            (0x2000, 4, false),
            (0x2000, 1, true),
            (0, 5, true),
        ] {
            let (mut e, actor, _) = anim_action_engine();
            answer(&mut e, PROCESS_VT, 0x148, weapon);
            e.call_log = Some(vec![]);
            e.call(0x008a_73e0, &args![actor, action, 0u32]);
            let seen = called(&calls(&mut e), 0x0049_6080).len();
            assert_eq!(seen == 1, cleared, "{weapon:x} {action}");
        }
    }

    #[test]
    fn anim_action_for_the_player_also_clears_the_first_person_animation() {
        let (mut e, actor, _) = anim_action_engine();
        e.set_global(PLAYER_CHARACTER, actor.addr());
        e.call_log = Some(vec![]);
        e.call(0x008a_73e0, &args![actor, 3i32, 0u32]);
        let log = calls(&mut e);
        assert_eq!(called(&log, 0x0095_0a60), vec![vec![actor.addr(), 1]]);
        assert_eq!(
            called(&log, 0x0049_6080),
            vec![vec![0x500, 2, 0], vec![0x600, 2, 0]]
        );
    }

    #[test]
    fn anim_action_is_the_process_answer_or_minus_one() {
        let mut e = engine();
        let (actor, _) = new_actor_with_process(&mut e);
        answer(&mut e, PROCESS_VT, 0x3e4, 6);
        assert_eq!(e.call(0x008a_7570, &args![actor]).i32(), 6);
        let bare = new_actor(&mut e);
        assert_eq!(e.call(0x008a_7570, &args![bare]).i32(), -1);
    }

    /// Doubles for the two package starters.
    fn starter_engine() -> (Engine, Ptr<Actor>, Ptr) {
        let mut e = engine();
        let (actor, proc) = new_actor_with_process(&mut e);
        e.set_global(PLAYER_CHARACTER, 0x1357u32);
        e.register(FORM_TYPE, |e, a| (e.mem.i8(a[0] + 0x20) as i32).into_ret());
        e.register(0x0067_0b90, |_, _| 0x4000.into_ret());
        e.register(OPERATOR_NEW, |_, _| 0x4100.into_ret());
        e.register(0x0067_f030, |_, a| a[0].into_ret());
        e.register(GET_PARENT_CELL, |_, _| 0x8000.into_ret());
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        stub(
            &mut e,
            &[
                0x0043_7bf0,
                0x0043_7bd0,
                0x0067_0fc0,
                0x0082_6b40,
                0x0082_6b90,
                0x0067_f3c0,
                0x0067_1d30,
                0x0067_0b30,
                0x0098_4f60,
                0x009e_d0d0,
            ],
        );
        (e, actor, proc)
    }

    #[test]
    fn starting_a_package_at_the_actor_builds_and_installs_it() {
        let (mut e, actor, proc) = starter_engine();
        e.call_log = Some(vec![]);
        e.call(0x008a_75a0, &args![actor]);
        let log = calls(&mut e);
        let steps = [
            0x0067_0b90,
            0x0067_0fc0,
            0x0082_6b40,
            0x0082_6b90,
            OPERATOR_NEW,
            0x0067_f030,
            0x0067_f3c0,
            0x0067_1d30,
            0x0067_0b30,
            0x0098_4f60,
            target(PROCESS_VT, 0x28),
            target(ACTOR_VT, 0x2f4),
        ];
        assert_eq!(order(&log, &steps), steps.to_vec());
        assert_eq!(called(&log, 0x0067_0b90), vec![vec![0x1a]]);
        assert_eq!(called(&log, 0x0067_0fc0), vec![vec![0x4000, 0x1a]]);
        assert_eq!(called(&log, 0x0067_f3c0), vec![vec![0x4100, actor.addr()]]);
        assert_eq!(called(&log, 0x0067_1d30), vec![vec![0x4000, 0x4100]]);
        assert_eq!(called(&log, 0x0067_0b30), vec![vec![0x4100, 1]]);
        assert_eq!(called(&log, 0x0098_4f60), vec![vec![0x4000, 0x15]]);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x28)),
            vec![vec![proc.addr()]]
        );
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x2f4)),
            vec![vec![actor.addr(), 0x4000, 1, 1]]
        );
    }

    #[test]
    fn starting_a_package_at_the_actor_without_memory_skips_the_location_cleanup() {
        let (mut e, actor, _) = starter_engine();
        e.register(OPERATOR_NEW, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_75a0, &args![actor]);
        let log = calls(&mut e);
        assert!(called(&log, 0x0067_f030).is_empty());
        assert!(called(&log, 0x0067_0b30).is_empty());
        assert_eq!(called(&log, 0x0067_1d30), vec![vec![0x4000, 0]]);
    }

    #[test]
    fn starting_a_package_at_the_actor_defers_to_process_method_4c0_when_it_says_7f() {
        let (mut e, actor, proc) = starter_engine();
        answer(&mut e, PROCESS_VT, 0x4d0, 0x7f);
        e.call_log = Some(vec![]);
        e.call(0x008a_75a0, &args![actor]);
        let log = calls(&mut e);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x4c0)),
            vec![vec![proc.addr(), actor.addr(), 0, 0, 0x7f]]
        );
        assert!(called(&log, 0x0067_0b90).is_empty());
    }

    #[test]
    fn package_starters_stop_at_their_gates() {
        let package: u32 = 0;
        let _ = package;
        for addr in [0x008a_75a0u32, 0x008a_7760] {
            // Either of the two checks holds.
            for gate in [0x0043_7bf0u32, 0x0043_7bd0] {
                let (mut e, actor, _) = starter_engine();
                e.register(gate, |_, _| 1.into_ret());
                e.call_log = Some(vec![]);
                e.call(addr, &args![actor]);
                assert!(called(&calls(&mut e), 0x0067_0b90).is_empty());
            }
            // The current package already has type 0x1a.
            let (mut e, actor, _) = starter_engine();
            let current: Ptr = Ptr::new(e.mem.alloc(0x40));
            e.mem.set_u8(current.addr() + 0x20, 0x1a);
            answer(&mut e, PROCESS_VT, 0x27c, current.addr());
            e.call_log = Some(vec![]);
            e.call(addr, &args![actor]);
            assert!(called(&calls(&mut e), 0x0067_0b90).is_empty());
            // Another type does not stop it.
            e.mem.set_u8(current.addr() + 0x20, 0x19);
            e.call_log = Some(vec![]);
            e.call(addr, &args![actor]);
            assert_eq!(called(&calls(&mut e), 0x0067_0b90).len(), 1);
        }
    }

    #[test]
    fn starting_a_back_up_package_initializes_it() {
        let (mut e, actor, proc) = starter_engine();
        e.set_global(PLAYER_CHARACTER, 0x1357u32);
        e.call_log = Some(vec![]);
        e.call(0x008a_7760, &args![actor]);
        let log = calls(&mut e);
        assert_eq!(called(&log, 0x0067_0b90), vec![vec![0x27]]);
        assert_eq!(called(&log, 0x0067_0fc0), vec![vec![0x4000, 0x27]]);
        assert_eq!(
            called(&log, DYNAMIC_CAST),
            vec![vec![0x4000, 0, TYPE_TES_PACKAGE, TYPE_BACK_UP_PACKAGE, 0]]
        );
        assert_eq!(
            called(&log, 0x009e_d0d0),
            vec![vec![0x4000, actor.addr(), 0x8000, 0x1357]]
        );
        assert_eq!(called(&log, 0x0098_4f60), vec![vec![0x4000, 0x30]]);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x28)),
            vec![vec![proc.addr()]]
        );
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x2f4)),
            vec![vec![actor.addr(), 0x4000, 1, 1]]
        );
        // A package that is not a back-up package is not initialized.
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008a_7760, &args![actor]);
        let log = calls(&mut e);
        assert!(called(&log, 0x009e_d0d0).is_empty());
        assert_eq!(called(&log, target(ACTOR_VT, 0x2f4)).len(), 1);
    }

    /// Everything `008a5580` needs: an actor with an animation, a process
    /// whose current package is a special-idle package (type 0x1c), a
    /// speaker and the player.
    struct UpdateWorld {
        e: Engine,
        actor: Ptr<Actor>,
        proc: Ptr,
        idle: Ptr,
        speaker: Ptr,
        player: Ptr,
        face: Ptr,
    }

    fn update_world() -> UpdateWorld {
        let mut e = engine();
        e.map(0x011c_f000, 0x1000);
        let (actor, proc) = new_actor_with_process(&mut e);
        e.mem.set_u32(actor.addr() + 0xa4, OBJECT_VT);
        e.mem.set_u32(actor.addr() + 0xc, 0xaa11);
        let idle: Ptr = Ptr::new(e.mem.alloc(0x100));
        e.mem.set_u8(idle.addr() + 0x20, 0x1c);
        let speaker: Ptr = Ptr::new(e.mem.alloc(0x100));
        e.mem.set_u32(speaker.addr(), 0x0d00d);
        e.mem.set_u32(speaker.addr() + 8, 0x0b05e);
        e.mem.set_u32(speaker.addr() + 0xc, 0x0c0de);
        e.mem.set_u32(speaker.addr() + 0x10, 0x0ace);
        e.mem.set_u32(speaker.addr() + 0x18, 0x0e00);
        e.mem.set_u32(speaker.addr() + 0x1c, 0x0f00);
        e.mem.set_u8(speaker.addr() + 0x24, 1);
        let player = new_object(&mut e);
        e.set_global(PLAYER_CHARACTER, player.addr());
        let face = new_object(&mut e);
        answer(&mut e, ACTOR_VT, 0x1e4, 0x6000);
        answer(&mut e, PROCESS_VT, 0x22c, idle.addr());
        e.register_double(target(ACTOR_VT, 0x284), |_, _| Ret {
            st0: 1.5,
            ..Ret::default()
        });
        e.register(FORM_TYPE, |e, a| (e.mem.i8(a[0] + 0x20) as i32).into_ret());
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        e.register(GET_SAVE_FORM, |e, a| e.mem.u32(a[0] + 0x4c).into_ret());
        e.register(0x0054_3c30, |e, a| e.mem.u8(a[0] + 0x24).into_ret());
        e.register(0x0044_1110, |e, a| e.mem.u32(a[0] + 0x1c).into_ret());
        e.register(0x0096_11e0, |e, a| e.mem.u32(a[0] + 0x18).into_ret());
        e.register(0x0068_15c0, |_, a| a[0].into_ret());
        e.register(0x0040_48e0, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(0x0044_ddc0, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(0x0046_0140, |_, a| (a[0] + 0x10).into_ret());
        e.register(0x0055_9450, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(0x00ad_9060, |_, _| 0x9000.into_ret());
        e.register(SOUND_HANDLE_INIT, |e, a| {
            e.mem.set_u32(a[0], u32::MAX);
            Ret::default()
        });
        e.register(GET_FACE_ANIMATION_DATA, |e, _| e.mem.u32(KNOB).into_ret());
        e.set_global(KNOB, face.addr());
        e.register(0x00ad_8930, |e, _| e.mem.u32(KNOB + 4).into_ret());
        e.register(SCRIPT_FLAG, |e, _| e.mem.u32(KNOB + 8).into_ret());
        e.register(INTERFACE_IN_DIALOG, |e, _| e.mem.u32(KNOB + 12).into_ret());
        e.register(0x0049_3bb0, |_, _| Ret::default());
        e.register(0x0045_cd60, |_, _| 1.into_ret());
        e.register(0x0049_85f0, |_, _| Ret::default());
        stub(
            &mut e,
            &[
                0x0093_3150,
                0x00ad_bfd0,
                0x00ad_c050,
                SOUND_HANDLE_DESTRUCT,
                0x00c7_48b0,
                0x009d_db50,
                0x009d_ce20,
                0x009c_9900,
                0x0089_d620,
                0x005f_36f0,
                0x00c5_2960,
            ],
        );
        UpdateWorld {
            e,
            actor,
            proc,
            idle,
            speaker,
            player,
            face,
        }
    }

    #[test]
    fn update_without_animation_does_nothing() {
        let mut w = update_world();
        answer(&mut w.e, ACTOR_VT, 0x1e4, 0);
        w.e.set(w.actor, Actor::bSpeakingDone, 1);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, w.speaker, 1u8]);
        let log = calls(&mut w.e);
        assert!(called(&log, target(ACTOR_VT, 0x140)).is_empty());
        assert!(called(&log, SOUND_HANDLE_INIT).is_empty());
        assert_eq!(w.e.get(w.actor, Actor::bTalkingToPlayer), 0);
    }

    #[test]
    fn update_starts_the_speech_of_a_speaker_without_a_form() {
        let mut w = update_world();
        w.e.set(w.actor, Actor::bSpeakingDone, 1);
        w.e.set(w.actor, Actor::bSoundFileDone, 1);
        w.e.set(w.actor, Actor::bVoiceFileDone, 1);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, w.speaker, 1u8]);
        let log = calls(&mut w.e);
        assert_eq!(w.e.global::<u32>(SPEAKING_ACTOR), w.actor.addr());
        assert_eq!(w.e.get(w.actor, Actor::bTalkingToPlayer), 1);
        assert_eq!(w.e.get(w.actor, Actor::bUseEmotion), 1);
        let handle = called(&log, SOUND_HANDLE_INIT)[0][0];
        let speak = called(&log, target(ACTOR_VT, 0x284));
        assert_eq!(
            speak,
            vec![vec![
                w.actor.addr(),
                0x0ace,
                handle,
                0x0b05e,
                0x0c0de,
                0x0d00d,
                0x0e00,
                0x0f00,
                w.player.addr(),
                1,
                1,
                0,
                1,
                1
            ]]
        );
        // The speech length goes to the process; the voice flag is cleared.
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x318)),
            vec![vec![w.proc.addr(), 1.5f32.to_bits()]]
        );
        assert_eq!(w.e.get(w.actor, Actor::bVoiceFileDone), 0);
        // The completion callbacks are registered for the actor (its id
        // 0xaa11, the handle word -1).
        assert_eq!(
            called(&log, 0x00ad_c050),
            vec![vec![0x9000, u32::MAX, 0x0093_5f00, 0xaa11]]
        );
        assert_eq!(
            called(&log, 0x00ad_bfd0),
            vec![vec![0x9000, u32::MAX, 0x008b_c590, 0xaa11]]
        );
        assert!(called(&log, 0x0093_3150).is_empty());
        // The special idle package now points at the actor.
        assert_eq!(w.e.mem.u32(w.idle.addr() + 0x94), w.actor.addr());
    }

    #[test]
    fn update_registers_no_callbacks_without_the_notify_flag() {
        let mut w = update_world();
        w.e.set(w.actor, Actor::bSpeakingDone, 1);
        w.e.set(w.actor, Actor::bSoundFileDone, 1);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, w.speaker, 0u8]);
        let log = calls(&mut w.e);
        assert_eq!(called(&log, target(ACTOR_VT, 0x284)).len(), 1);
        assert!(called(&log, 0x00ad_c050).is_empty());
        assert!(called(&log, 0x00ad_bfd0).is_empty());
    }

    #[test]
    fn update_queues_the_sound_of_a_speaker_with_a_form() {
        let mut w = update_world();
        let form: Ptr = Ptr::new(w.e.mem.alloc(0x100));
        w.e.mem.set_u32(form.addr() + 0xc, 0xf0f0);
        w.e.mem.set_u32(w.speaker.addr() + 0x4c, form.addr());
        w.e.set(w.actor, Actor::bSpeakingDone, 1);
        w.e.set(w.actor, Actor::bSoundFileDone, 1);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, w.speaker, 1u8]);
        let log = calls(&mut w.e);
        let queued = called(&log, 0x0093_3150);
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0][0], w.actor.addr());
        assert_eq!(queued[0][2..], [0xf0f0, 0, 0x102, 1]);
        assert_eq!(
            called(&log, 0x00ad_bfd0),
            vec![vec![0x9000, 0, 0x0093_5cc0, 0xaa11]]
        );
        // The flags are cleared and no speech starts.
        assert_eq!(w.e.get(w.actor, Actor::bSoundFileDone), 0);
        assert_eq!(w.e.get(w.actor, Actor::bSpeakingDone), 0);
        assert!(called(&log, target(ACTOR_VT, 0x284)).is_empty());
        assert_eq!(w.e.get(w.actor, Actor::bUseEmotion), 1);
    }

    #[test]
    fn update_does_not_speak_unless_the_actor_is_ready() {
        for (speaking, has_speaker) in [(0u8, true), (1, false)] {
            let mut w = update_world();
            w.e.set(w.actor, Actor::bSpeakingDone, speaking);
            let speaker = if has_speaker { w.speaker } else { Ptr::NULL };
            w.e.call_log = Some(vec![]);
            w.e.call(0x008a_5580, &args![w.actor, 0.5f32, speaker, 1u8]);
            let log = calls(&mut w.e);
            assert!(called(&log, SOUND_HANDLE_INIT).is_empty());
            assert_eq!(w.e.get(w.actor, Actor::bTalkingToPlayer), 0);
            assert_eq!(w.e.global::<u32>(SPEAKING_ACTOR), 0);
        }
    }

    /// The actor's mover, the actor's and the player's positions, and the
    /// numbers the angle computation sees: the direction `direction`, the
    /// actor's rotation `rotation`.
    fn mover_world(direction: f32, rotation: f32) -> (UpdateWorld, Ptr, Ptr) {
        let mut w = update_world();
        let mover: Ptr = Ptr::new(w.e.mem.alloc(0x40));
        w.e.set(w.actor, Actor::pActorMover, mover);
        let player_position = floats(&mut w.e, &[10.0, 20.0, 30.0]);
        let own_position = floats(&mut w.e, &[1.0, 2.0, 3.0]);
        answer(&mut w.e, OBJECT_VT, 0x1f4, player_position.addr());
        answer(&mut w.e, ACTOR_VT, 0x1f4, own_position.addr());
        let rotation_block = floats(&mut w.e, &[0.0, 0.0, rotation]);
        w.e.set_global(KNOB + 16, direction);
        w.e.set_global(PI, std::f32::consts::PI);
        w.e.set_global(MINUS_PI, -std::f32::consts::PI);
        w.e.set_global(ONE_DEGREE, std::f64::consts::PI / 180.0);
        w.e.register(0x0043_9ef0, |_, _| Ret::default());
        w.e.register(0x004b_13c0, |e, _| e.mem.f32(KNOB + 16).into_ret());
        w.e.register(0x0043_0830, |e, _| e.mem.u32(KNOB + 20).into_ret());
        w.e.set_global(KNOB + 20, rotation_block.addr());
        stub(&mut w.e, &[0x004e_44f0]);
        w.e.register(0x0040_8840, |_, a| {
            (f32::from_bits(a[0]).abs() as f64).into_ret()
        });
        (w, mover, player_position)
    }

    #[test]
    fn update_turns_the_actor_towards_the_player_for_a_big_angle() {
        let (mut w, mover, player_position) = mover_world(1.0, 0.25);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        // The player's position words go to the rotation request.
        let pos = player_position.addr();
        let words = [w.e.mem.u32(pos), w.e.mem.u32(pos + 4), w.e.mem.u32(pos + 8)];
        assert_eq!(
            called(&log, 0x009d_ce20),
            vec![vec![mover.addr(), words[0], words[1], words[2], 0]]
        );
        // The move update is bracketed by the mover's flag.
        let steps = [0x009c_9900, target(ACTOR_VT, 0x348), 0x009c_9900];
        assert_eq!(order(&log, &steps), steps.to_vec());
        assert_eq!(
            called(&log, 0x009c_9900),
            vec![vec![mover.addr(), 1], vec![mover.addr(), 0]]
        );
        assert_eq!(
            called(&log, target(ACTOR_VT, 0x348)),
            vec![vec![w.actor.addr(), 0.5f32.to_bits(), 0]]
        );
        // The vector from the player is built from the two positions.
        assert_eq!(called(&log, 0x0043_9ef0).len(), 1);
        assert_eq!(called(&log, 0x0043_9ef0)[0][0], pos);
    }

    #[test]
    fn update_does_not_turn_for_a_small_angle_or_a_rotating_mover() {
        // 1 degree = 0.01745: 0.01 is below it.
        let (mut w, _, _) = mover_world(0.26, 0.25);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert!(called(&log, 0x009d_ce20).is_empty());
        assert_eq!(called(&log, target(ACTOR_VT, 0x348)).len(), 1);
        // A mover that is rotating already.
        let (mut w, _, _) = mover_world(1.0, 0.25);
        w.e.register(0x009d_db50, |_, _| 1.into_ret());
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert!(called(&log, 0x009d_ce20).is_empty());
        assert_eq!(called(&log, target(ACTOR_VT, 0x348)).len(), 1);
        // An actor with a package change pending, or a living body check.
        let (mut w, _, _) = mover_world(1.0, 0.25);
        answer(&mut w.e, ACTOR_VT, 0x214, 3);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        assert!(called(&calls(&mut w.e), target(ACTOR_VT, 0x348)).is_empty());
        let (mut w, _, _) = mover_world(1.0, 0.25);
        w.e.register(0x0049_3bb0, |_, _| 1.into_ret());
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        assert!(called(&calls(&mut w.e), target(ACTOR_VT, 0x348)).is_empty());
    }

    #[test]
    fn update_relaxes_the_bones_of_an_idle_process() {
        let mut w = update_world();
        w.e.register(0x0045_cd60, |_, _| Ret::default());
        w.e.register(0x0089_d620, |_, _| 0x7000.into_ret());
        w.e.register(0x005f_36f0, |_, _| 1.into_ret());
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert_eq!(called(&log, 0x00c5_2960), vec![vec![0x7000, 0]]);
        assert_eq!(called(&log, target(ACTOR_VT, 0x140)).len(), 1);
    }

    #[test]
    fn update_sets_the_face_from_the_emotion() {
        // Emotion 1 and 5 pick moods 6 and 5; others do nothing.
        for (emotion, mood) in [(1u32, Some(6u32)), (5, Some(5)), (3, None)] {
            let mut w = update_world();
            w.e.set(w.actor, Actor::ePersuasionEmotion, emotion);
            w.e.mem.set_u32(w.idle.addr() + 0x94, 0x1234);
            w.e.call_log = Some(vec![]);
            w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
            let log = calls(&mut w.e);
            let mood_calls = called(&log, target(OBJECT_VT, 0x11c));
            match mood {
                Some(m) => assert_eq!(mood_calls, vec![vec![w.face.addr(), m, 0]], "{emotion}"),
                None => assert!(mood_calls.is_empty(), "{emotion}"),
            }
            // Not playing: the idle package forgets its pointer.
            assert_eq!(w.e.mem.u32(w.idle.addr() + 0x94), 0);
        }
    }

    #[test]
    fn update_dialogue_emotion_asks_for_the_actor_value_or_resets() {
        // Emotion 8 with the script flag clear: the face is reset.
        let mut w = update_world();
        w.e.set(w.actor, Actor::ePersuasionEmotion, 8);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert_eq!(
            called(&log, target(OBJECT_VT, 0xb4)),
            vec![vec![w.face.addr(), 0, 1, 0, 0, 0, 0]]
        );
        // With the flag set, the interface in a dialogue and a free process:
        // the actor value (slot +8 of the sub-object at +0xa4, with 4 and 0).
        let mut w = update_world();
        w.e.set(w.actor, Actor::ePersuasionEmotion, 8);
        w.e.set_global(KNOB + 8, 1u32);
        w.e.set_global(KNOB + 12, 1u32);
        answer(&mut w.e, OBJECT_VT, 0x8, 0x55);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert_eq!(
            called(&log, target(OBJECT_VT, 0x8)),
            vec![vec![w.actor.addr() + 0xa4, 4, 0]]
        );
        assert_eq!(
            called(&log, target(OBJECT_VT, 0x11c)),
            vec![vec![w.face.addr(), 0x55]]
        );
        // A busy process, or no dialogue, does nothing.
        for busy in [true, false] {
            let mut w = update_world();
            w.e.set(w.actor, Actor::ePersuasionEmotion, 8);
            w.e.set_global(KNOB + 8, 1u32);
            w.e.set_global(KNOB + 12, if busy { 1u32 } else { 0 });
            if busy {
                answer(&mut w.e, PROCESS_VT, 0x2e8, 1);
            }
            w.e.call_log = Some(vec![]);
            w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
            let log = calls(&mut w.e);
            assert!(called(&log, target(OBJECT_VT, 0x11c)).is_empty(), "{busy}");
            assert!(called(&log, target(OBJECT_VT, 0xb4)).is_empty(), "{busy}");
        }
    }

    #[test]
    fn update_while_a_sound_plays_only_the_dialogue_emotion_acts() {
        // Playing, emotion 8: the reset runs on the face looked up again.
        let mut w = update_world();
        w.e.set(w.actor, Actor::ePersuasionEmotion, 8);
        w.e.set_global(KNOB + 4, 1u32);
        w.e.mem.set_u32(w.idle.addr() + 0x94, 0x1234);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert_eq!(called(&log, target(OBJECT_VT, 0xb4)).len(), 1);
        assert_eq!(called(&log, GET_FACE_ANIMATION_DATA).len(), 2);
        // The idle package is left alone while the sound plays.
        assert_eq!(w.e.mem.u32(w.idle.addr() + 0x94), 0x1234);
        // Any other emotion does nothing.
        let mut w = update_world();
        w.e.set(w.actor, Actor::ePersuasionEmotion, 1);
        w.e.set_global(KNOB + 4, 1u32);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert!(called(&log, target(OBJECT_VT, 0x11c)).is_empty());
        assert!(called(&log, target(OBJECT_VT, 0xb4)).is_empty());
    }

    #[test]
    fn update_restarts_a_finished_special_idle() {
        let mut w = update_world();
        w.e.register(0x0049_85f0, |_, a| (a[0] == 0x6000).into_ret());
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        let log = calls(&mut w.e);
        assert_eq!(
            called(&log, target(PROCESS_VT, 0x44)),
            vec![vec![w.proc.addr(), w.actor.addr(), 0, 2, 1, 0, 1]]
        );
        // The process says it still has something to do (+0x718).
        answer(&mut w.e, PROCESS_VT, 0x718, 1);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        assert!(called(&calls(&mut w.e), target(PROCESS_VT, 0x44)).is_empty());
        // Without a special-idle package nothing is restarted.
        let mut w = update_world();
        w.e.register(0x0049_85f0, |_, _| 1.into_ret());
        w.e.mem.set_u8(w.idle.addr() + 0x20, 0x1b);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        assert!(called(&calls(&mut w.e), target(PROCESS_VT, 0x44)).is_empty());
    }

    #[test]
    fn update_tells_the_ragdoll_controller() {
        let mut w = update_world();
        let ragdoll: Ptr = Ptr::new(w.e.mem.alloc(0x40));
        w.e.set(w.actor, Actor::pRagdollController, ragdoll);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008a_5580, &args![w.actor, 0.5f32, NO_SPEAKER, 0u8]);
        assert_eq!(
            called(&calls(&mut w.e), 0x00c7_48b0),
            vec![vec![ragdoll.addr()]]
        );
    }
}
