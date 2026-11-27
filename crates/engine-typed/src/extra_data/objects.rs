//! The extra data objects unit's first 116 functions (`extradataobjects.rs`, `004300f0` to
//! `00432c90`): the constructors, destructors and `Compare`s of the small extra data classes that
//! hang on a list (`ExtraAnim`, `ExtraDismemberedLimbs`, `ExtraStartingPosition`, `ExtraLight`,
//! `ExtraLock` with its lock record `REFR_LOCK`, `ExtraFollower`, `ExtraGuardedRefData`,
//! `ExtraTeleport`, the map, audio and buoy markers, `ExtraAction`, `ExtraContainerChanges`, the
//! one-word classes from `ExtraOriginalReference` to `ExtraScale`, the worn and ghost flags, the hot
//! key, the info general topic, the seed, the package start location, the reference pointer, the
//! packages, the player crime list, the leveled item and the persistent cell).
//!
//! A constructor is `BSExtraData`'s (`0040ec80`) in place with the class's type, then its vtable and
//! its payload. A scalar deleting destructor is the class's destructor as a typed call, then
//! `operator delete` for bit 0 of the flags. A destructor puts the class's vtable back, drops what
//! the class owns and ends in `BSExtraData`'s destructor (`0040ecb0`, the cell references family's,
//! a typed call). A `Compare` casts the other extra data to its class (`__RTDynamicCast`, the C
//! runtime's, called by address), asks `BSExtraData::Compare` (`0040f700`, the cell references
//! family's, a typed call) and then compares its own fields: true when the two differ.
//!
//! **Native.** The exe's order of work. Done in place: `BSExtraData`'s constructor, an array's count
//! (`0044ddc0`) and element (`006a7ad0`), the 00430830 accessor of a reference's angles, the actor
//! list's constructor (`0096a2d0`, two null words), the world location's constructor (`006d5320`,
//! nothing), a form's word at +0xc (`0084e3a0`) and +0x68 (`008d8520`), and the save-load object's
//! test that always answers false (`0047c850`). Typed calls: this family's own functions (the
//! `DismemberedLimb` record's constructor `0042c470` and its array's destructor `0042ff80`, the
//! reference's NPC `004181e0`, the teleport, map marker and audio marker data's deletes), the cell
//! references family's `BSExtraData` destructor and `Compare`, the dispatch family's array search and
//! clear (`00719b20`, `008454f0`), the C runtime's `memcmp` and float-to-int, the multibound family's
//! setting value (`0043d4d0`) and the forms family's lookup by id (`004839c0`). Other units'
//! constructors, destructors, deletes and compares are called by address, and so is `ScriptLocals`'
//! delete (`0041af70`, which xNVSE wraps). A virtual call stays a call through the slot.
//!
//! **Extenders.** None of these addresses is wrapped or hooked. JIP LN NVSE writes its own routines
//! into `ExtraTimeLeft`'s vtable slots 0 and 4 for its keyed data (`0x10158FC`, `0x1015900`): a slot
//! write, so callers through the vtable reach JIP's routine and `00431ef0` stays the vanilla
//! `Compare`. JIP calls the limbs' constructor (`00430200`) by address, which reaches it here.

use super::classes::{self, extra_with_vtable};
use super::{access, arrays, cell_refs, dispatch, free_when, word_at};
use super::{EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE, OPERATOR_NEW};
use crate::{crt, forms, multibound};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

// ---- The classes' types and vtables ---------------------------------------------------------

const KIND_PERSISTENT_CELL: u8 = 0x0c;
const KIND_SCRIPT: u8 = 0x0d;
const KIND_ACTION: u8 = 0x0e;
const KIND_STARTING_POSITION: u8 = 0x0f;
const KIND_CONTAINER_CHANGES: u8 = 0x15;
const KIND_WORN: u8 = 0x16;
const KIND_WORN_LEFT: u8 = 0x17;
const KIND_PACKAGE_START_LOCATION: u8 = 0x18;
const KIND_PACKAGE: u8 = 0x19;
const KIND_TRESPASS_PACKAGE: u8 = 0x1a;
const KIND_REFERENCE_POINTER: u8 = 0x1c;
const KIND_FOLLOWER: u8 = 0x1d;
const KIND_GHOST: u8 = 0x1f;
const KIND_ORIGINAL_REFERENCE: u8 = 0x20;
const KIND_OWNERSHIP: u8 = 0x21;
const KIND_GLOBAL: u8 = 0x22;
const KIND_RANK: u8 = 0x23;
const KIND_COUNT: u8 = 0x24;
const KIND_HEALTH: u8 = 0x25;
const KIND_USES: u8 = 0x26;
const KIND_TIME_LEFT: u8 = 0x27;
const KIND_CHARGE: u8 = 0x28;
const KIND_LIGHT: u8 = 0x29;
const KIND_LOCK: u8 = 0x2a;
const KIND_TELEPORT: u8 = 0x2b;
const KIND_MAP_MARKER: u8 = 0x2c;
const KIND_2E: u8 = 0x2e;
const KIND_LEVELED_ITEM: u8 = 0x2f;
const KIND_SCALE: u8 = 0x30;
const KIND_SEED: u8 = 0x31;
const KIND_PLAYER_CRIME_LIST: u8 = 0x35;
const KIND_CANNOT_WEAR: u8 = 0x3e;
const KIND_49: u8 = 0x49;
const KIND_HOT_KEY: u8 = 0x4a;
const KIND_INFO_GENERAL_TOPIC: u8 = 0x4d;
const KIND_DISMEMBERED_LIMBS: u8 = 0x5f;
const KIND_GUARDED_REF_DATA: u8 = 0x7c;
const KIND_AUDIO_MARKER: u8 = 0x90;
const KIND_AUDIO_BUOY_MARKER: u8 = 0x91;

const VTABLE_ANIM: u32 = 0x0101_5b28;
const VTABLE_DISMEMBERED_LIMBS: u32 = 0x0101_5b34;
const VTABLE_STARTING_POSITION: u32 = 0x0101_5b40;
const VTABLE_49: u32 = 0x0101_5b4c;
const VTABLE_LIGHT: u32 = 0x0101_5b58;
const VTABLE_LOCK: u32 = 0x0101_589c;
const VTABLE_FOLLOWER: u32 = 0x0101_5b64;
const VTABLE_GUARDED_REF_DATA: u32 = 0x0101_5b70;
const VTABLE_2E: u32 = 0x0101_5b7c;
const VTABLE_TELEPORT: u32 = 0x0101_58a8;
const VTABLE_MAP_MARKER: u32 = 0x0101_5b88;
const VTABLE_AUDIO_MARKER: u32 = 0x0101_5b94;
const VTABLE_AUDIO_BUOY_MARKER: u32 = 0x0101_5ba0;
const VTABLE_ACTION: u32 = 0x0101_5bac;
const VTABLE_CONTAINER_CHANGES: u32 = 0x0101_5bb8;
const VTABLE_ORIGINAL_REFERENCE: u32 = 0x0101_5bc4;
const VTABLE_OWNERSHIP: u32 = 0x0101_58b4;
const VTABLE_GLOBAL: u32 = 0x0101_58c0;
const VTABLE_RANK: u32 = 0x0101_58cc;
const VTABLE_COUNT: u32 = 0x0101_58d8;
const VTABLE_HEALTH: u32 = 0x0101_58e4;
const VTABLE_USES: u32 = 0x0101_58f0;
const VTABLE_TIME_LEFT: u32 = 0x0101_58fc;
const VTABLE_CHARGE: u32 = 0x0101_5908;
const VTABLE_SCRIPT: u32 = 0x0101_5914;
const VTABLE_SCALE: u32 = 0x0101_5920;
const VTABLE_HOT_KEY: u32 = 0x0101_592c;
const VTABLE_REFERENCE_POINTER: u32 = 0x0101_5938;
const VTABLE_TRESPASS_PACKAGE: u32 = 0x0101_5944;
const VTABLE_LEVELED_ITEM: u32 = 0x0101_5950;
const VTABLE_GHOST: u32 = 0x0101_5bd0;
const VTABLE_WORN: u32 = 0x0101_5bdc;
const VTABLE_WORN_LEFT: u32 = 0x0101_5be8;
const VTABLE_CANNOT_WEAR: u32 = 0x0101_5bf4;
const VTABLE_INFO_GENERAL_TOPIC: u32 = 0x0101_5c00;
const VTABLE_SEED: u32 = 0x0101_5c0c;
const VTABLE_PACKAGE_START_LOCATION: u32 = 0x0101_5c18;
const VTABLE_PACKAGE: u32 = 0x0101_5c24;
const VTABLE_PLAYER_CRIME_LIST: u32 = 0x0101_5c30;
const VTABLE_PERSISTENT_CELL: u32 = 0x0101_5c3c;

/// The RTTI type descriptors a `Compare` casts with: `BSExtraData`'s, then each class's.
const TYPE_BS_EXTRA_DATA: u32 = 0x0118_3b2c;
const TYPE_STARTING_POSITION: u32 = 0x0118_4b10;
const TYPE_LOCK: u32 = 0x0118_4754;
const TYPE_TELEPORT: u32 = 0x0118_4430;
const TYPE_MAP_MARKER: u32 = 0x0118_45fc;
const TYPE_AUDIO_MARKER: u32 = 0x0118_45dc;
const TYPE_AUDIO_BUOY_MARKER: u32 = 0x0118_45b8;
const TYPE_ACTION: u32 = 0x0118_4bc0;
const TYPE_ORIGINAL_REFERENCE: u32 = 0x0118_4c00;
const TYPE_OWNERSHIP: u32 = 0x0118_476c;
const TYPE_GLOBAL: u32 = 0x0118_478c;
const TYPE_RANK: u32 = 0x0118_47a8;
const TYPE_COUNT: u32 = 0x0118_47c0;
const TYPE_LEVELED_ITEM: u32 = 0x0118_4680;
const TYPE_HEALTH: u32 = 0x0118_47dc;
const TYPE_HEALTH_PERC: u32 = 0x0118_4208;
const TYPE_USES: u32 = 0x0118_47f8;
const TYPE_TIME_LEFT: u32 = 0x0118_4810;
const TYPE_CHARGE: u32 = 0x0118_482c;
const TYPE_SCRIPT: u32 = 0x0118_4514;
const TYPE_WEAPON_MOD_FLAGS: u32 = 0x0118_48d8;
const TYPE_MODDING_ITEM: u32 = 0x0118_493c;
const TYPE_SCALE: u32 = 0x0118_4848;
const TYPE_HOT_KEY: u32 = 0x0118_4864;
const TYPE_SEED: u32 = 0x0118_4c98;
const TYPE_PACKAGE_START_LOCATION: u32 = 0x0118_448c;

// ---- What the functions call ----------------------------------------------------------------

/// `__RTDynamicCast(object, offset, from, to, is_reference)` (the C runtime's, `cdecl`).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// Other units' deletes of what a class owns (`thiscall(object, flags)`): the animation, the light,
/// the actor list, the audio buoy data, `ScriptLocals` (xNVSE wraps it), the menu topic.
const ANIMATION_DELETE: u32 = 0x0041_8d20;
const LIGHT_DELETE: u32 = 0x0041_8f10;
const ACTOR_LIST_DELETE: u32 = 0x0047_02f0;
const AUDIO_BUOY_DATA_DELETE: u32 = 0x007b_3fa0;
const SCRIPT_LOCALS_DELETE: u32 = 0x0041_af70;
const MENU_TOPIC_DELETE: u32 = 0x0042_5ff0;
/// The actor list's clear (`thiscall(list)`) and the inventory changes' destructor body.
const ACTOR_LIST_CLEAR: u32 = 0x0047_0470;
const INVENTORY_CHANGES_DROP: u32 = 0x004b_f150;
/// Other units' constructors and destructors of members (`thiscall(member)`): the limbs' array, the
/// guards' array, the starting position's `FILE_POS_ROT`, the menu topic.
const LIMBS_ARRAY_NEW: u32 = 0x0043_8570;
const LIMBS_ARRAY_DROP: u32 = 0x0043_85a0;
const GUARDS_ARRAY_NEW: u32 = 0x0043_8660;
const GUARDS_ARRAY_DROP: u32 = 0x0043_8690;
const FILE_POS_ROT_NEW: u32 = 0x0069_2710;
const MENU_TOPIC_NEW: u32 = 0x0083_da50;
/// The pointer arrays' add (`thiscall(array, &item)`, the new index) and buffer compare
/// (`thiscall(array, other)`).
const ARRAY_ADD: u32 = 0x007c_b2e0;
const ARRAY_SAME_BUFFER: u32 = 0x0043_87b0;
/// The word compare the guards' search matches with (`009a3830`, which the dispatch family's search
/// does in place).
const WORDS_EQUAL: u32 = 0x009a_3830;
/// The data compares of the teleport, map marker, audio marker and audio buoy marker
/// (`thiscall(data, other)`).
const DOOR_TELEPORT_COMPARE: u32 = 0x0043_a860;
const MAP_MARKER_DATA_COMPARE: u32 = 0x0043_8e40;
const AUDIO_MARKER_DATA_COMPARE: u32 = 0x0058_97c0;
const AUDIO_BUOY_DATA_COMPARE: u32 = 0x0040_1290;
/// `TESNPC::BuildObjectArray(reference, base, array)` (`thiscall(npc, ...)`).
const BUILD_OBJECT_ARRAY: u32 = 0x0060_5fc0;
/// The level getters: the player's (`thiscall(actor)`, a `u16`) and a reference's calculated one
/// (`thiscall(reference, 0)`).
const ACTOR_LEVEL: u32 = 0x0087_f9f0;
const REFERENCE_CALC_LEVEL: u32 = 0x0056_7e10;
/// A float setting's value (`thiscall(setting)`: the value's address).
const SETTING_FLOAT: u32 = 0x0040_3e20;
/// The perk entry point handler (`cdecl(entry, actor, &result)`) and the lock-broken entry.
const HANDLE_ENTRY_POINT: u32 = 0x005e_58f0;
const ENTRY_LOCK_BROKEN: u32 = 0x20;
/// The package's created flag setter (`thiscall(package, flag)`).
const PACKAGE_SET_CREATED: u32 = 0x0067_4d70;
/// The crime list's add (`thiscall(list, &crime)`).
const CRIME_LIST_ADD: u32 = 0x005a_e3d0;

/// The player (`PlayerCharacter*`), the leveled lock's float setting, the six lock level settings
/// (very easy to requires key) and the `double` 0.0 the lock-broken test compares with.
const PLAYER: u32 = 0x011d_ea3c;
const LEVELED_LOCK_SETTING: u32 = 0x011c_39b4;
const LOCK_LEVEL_SETTINGS: [u32; 6] = [
    0x011c_3a4c,
    0x011c_3a30,
    0x011c_3a00,
    0x011c_3ab0,
    0x011c_399c,
    0x011c_3a24,
];
const ZERO_DOUBLE: u32 = 0x0101_2060;

// ---- What the file shares --------------------------------------------------------------------

/// `BSExtraData`'s destructor (`0040ecb0`, the cell references family's).
#[inline]
fn extra_drop(e: &mut Engine, this: u32) {
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

/// A constructor's start with one word of payload at +0xc. Answers `this`.
#[inline]
fn with_word(e: &mut Engine, this: u32, kind: u8, vtable: u32, word: u32) -> u32 {
    extra_with_vtable(e, this, kind, vtable);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), word);
    this
}

/// A constructor's start with one byte of payload at +0xc. Answers `this`.
#[inline]
fn with_byte(e: &mut Engine, this: u32, kind: u8, vtable: u32, byte: u8) -> u32 {
    extra_with_vtable(e, this, kind, vtable);
    e.mem.set_u8(this.wrapping_add(EXTRA_PAYLOAD), byte);
    this
}

/// A destructor that owns nothing: the vtable back, then `BSExtraData`'s.
#[inline]
fn drop_plain(e: &mut Engine, this: u32, vtable: u32) {
    e.mem.set_u32(this, vtable);
    extra_drop(e, this);
}

/// A destructor that owns the object at +0xc: the vtable back, `delete` of the object (`thiscall(
/// object, 1)` by address) when there is one, then `BSExtraData`'s.
#[inline]
fn drop_owner(e: &mut Engine, this: u32, vtable: u32, delete: u32) {
    e.mem.set_u32(this, vtable);
    let owned = word_at(e, this, EXTRA_PAYLOAD);
    if owned != 0 {
        e.call(delete, &args![owned, 1u32]);
    }
    extra_drop(e, this);
}

/// A `Compare`'s start: `other` cast to the class of `target` (null: the two differ), then
/// `BSExtraData::Compare`. `None` when the answer is already "they differ", else the cast.
#[inline]
fn compare_start(e: &mut Engine, this: u32, other: u32, target: u32) -> Option<u32> {
    let cast = e
        .call(
            DYNAMIC_CAST,
            &args![other, 0u32, TYPE_BS_EXTRA_DATA, target, 0u32],
        )
        .u32();
    if cast == 0 || base_compare(e, this, other) {
        return None;
    }
    Some(cast)
}

/// `BSExtraData::Compare` (`0040f700`, the cell references family's).
#[inline]
fn base_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    typed_call!(
        cell_refs::FAMILY,
        0x0040_f700,
        cell_refs::extra_list::extra_compare(e, this, other)
    )
}

/// A `Compare` of the word at +0xc.
#[inline]
fn compare_word(e: &mut Engine, this: u32, other: u32, target: u32) -> bool {
    match compare_start(e, this, other, target) {
        None => true,
        Some(cast) => word_at(e, this, EXTRA_PAYLOAD) != word_at(e, cast, EXTRA_PAYLOAD),
    }
}

/// A `Compare` of the byte at +0xc.
#[inline]
fn compare_byte(e: &mut Engine, this: u32, other: u32, target: u32) -> bool {
    match compare_start(e, this, other, target) {
        None => true,
        Some(cast) => {
            e.mem.u8(this.wrapping_add(EXTRA_PAYLOAD)) != e.mem.u8(cast.wrapping_add(EXTRA_PAYLOAD))
        }
    }
}

/// A `Compare` of the `float` at +0xc (the x87's compare: a NaN differs from everything, 0.0 equals
/// -0.0).
#[inline]
fn compare_float(e: &mut Engine, this: u32, other: u32, target: u32) -> bool {
    match compare_start(e, this, other, target) {
        None => true,
        Some(cast) => e.mem.f32(this.wrapping_add(EXTRA_PAYLOAD)) != e.mem.f32(cast.wrapping_add(EXTRA_PAYLOAD)),
    }
}

/// A `Compare` of the two objects at +0xc by their own compare (`thiscall(mine, theirs)` by address).
#[inline]
fn compare_by(e: &mut Engine, this: u32, other: u32, target: u32, compare: u32) -> bool {
    match compare_start(e, this, other, target) {
        None => true,
        Some(cast) => {
            let mine = word_at(e, this, EXTRA_PAYLOAD);
            let theirs = word_at(e, cast, EXTRA_PAYLOAD);
            e.call(compare, &args![mine, theirs]).bool()
        }
    }
}

/// A `Compare` of `n` bytes at +0xc by `memcmp` (the C runtime family's).
#[inline]
fn compare_bytes(e: &mut Engine, this: u32, other: u32, target: u32, n: u32) -> bool {
    match compare_start(e, this, other, target) {
        None => true,
        Some(cast) => memcmp(e, this.wrapping_add(EXTRA_PAYLOAD), cast.wrapping_add(EXTRA_PAYLOAD), n) != 0,
    }
}

/// `memcmp` (`00ec4835`, the C runtime family's).
#[inline]
fn memcmp(e: &mut Engine, a: u32, b: u32, n: u32) -> i32 {
    typed_call!(crt::FAMILY, 0x00ec_4835, crt::mem::memcmp(e, a, b, n))
}

/// A pointer array's count (`0044ddc0`, in place: the word at +8).
#[inline]
fn count_of(e: &Engine, array: u32) -> u32 {
    e.mem.u32(array.wrapping_add(8))
}

/// The word in slot `index` of a pointer array (`006a7ad0`'s element, in place: the buffer at +4).
#[inline]
fn item_of(e: &Engine, array: u32, index: u32) -> u32 {
    let buffer = e.mem.u32(array.wrapping_add(4));
    e.mem.u32(buffer.wrapping_add(index.wrapping_mul(4)))
}

/// The address of slot `index` of a pointer array (`006a7ad0`, in place).
#[inline]
fn slot_of(e: &Engine, array: u32, index: u32) -> u32 {
    e.mem.u32(array.wrapping_add(4)).wrapping_add(index.wrapping_mul(4))
}

/// An empty actor list (`0096a2d0` in place: two null words) in an 8-byte block from `operator new`,
/// or null when the allocation failed.
#[inline]
fn new_actor_list(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
    if block != 0 {
        e.mem.set_u32(block, 0);
        e.mem.set_u32(block.wrapping_add(4), 0);
    }
    block
}

/// A setting's `int` value: the setting's value address (`0043d4d0`, the multibound family's), read.
#[inline]
fn setting_int(e: &mut Engine, setting: u32) -> i32 {
    let at = typed_call!(
        multibound::FAMILY,
        0x0043_d4d0,
        multibound::getters::setting_value(e, Ptr::new(setting))
    );
    e.mem.u32(at) as i32
}

// ---- ExtraAnim and ExtraDismemberedLimbs ------------------------------------------------------

/// The limbs' array (+0x20) and a `DismemberedLimb`'s object array (+4).
const LIMBS_ARRAY: u32 = 0x20;
const LIMB_OBJECTS: u32 = 4;

// Typed from 004300f0
/// `ExtraAnim`'s scalar deleting destructor. Answers `this`.
pub fn anim_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_0120, anim_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00430120
/// `ExtraAnim`'s destructor: the owned animation deleted (`00418d20`).
pub fn anim_drop(e: &mut Engine, this: u32) {
    drop_owner(e, this, VTABLE_ANIM, ANIMATION_DELETE);
}

// Typed from 004301b0
/// The `u16` of an animation sequence object at +0x4c for `section`'s slot (the whole body, 0x14,
/// is slot 1; the upper body, 0x15, slot 4).
pub fn sequence_word(e: &mut Engine, this: u32, section: i32) -> u16 {
    let slot = match section {
        0x14 => 1,
        0x15 => 4,
        other => other,
    };
    e.mem
        .u16(this.wrapping_add(0x4c).wrapping_add((slot as u32).wrapping_mul(2)))
}

// Typed from 00430200
/// `ExtraDismemberedLimbs::ExtraDismemberedLimbs` (type 0x5f): the array made (`00438570`), no limbs,
/// no cause of death or last hit limb (-1), no death object, not eaten. Answers `this`.
pub fn limbs_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_DISMEMBERED_LIMBS, VTABLE_DISMEMBERED_LIMBS);
    e.call(LIMBS_ARRAY_NEW, &args![this.wrapping_add(LIMBS_ARRAY)]);
    e.mem.set_u16(this.wrapping_add(0x0c), 0);
    e.mem.set_u32(this.wrapping_add(0x10), u32::MAX);
    e.mem.set_u32(this.wrapping_add(0x14), 0);
    e.mem.set_u32(this.wrapping_add(0x18), u32::MAX);
    e.mem.set_u8(this.wrapping_add(0x1c), 0);
    this
}

// Typed from 004302a0
/// `ExtraDismemberedLimbs`' scalar deleting destructor. Answers `this`.
pub fn limbs_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_02d0, limbs_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004302d0
/// `ExtraDismemberedLimbs`' destructor: every limb record deleted (`00430390` with 1; the count read
/// again each pass, as the exe does), the array dropped (`004385a0`).
pub fn limbs_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_DISMEMBERED_LIMBS);
    let array = this.wrapping_add(LIMBS_ARRAY);
    let mut index = 0;
    while index < count_of(e, array) {
        let limb = item_of(e, array, index);
        if limb != 0 {
            typed_call!(FAMILY, 0x0043_0390, limb_delete(e, limb, 1));
        }
        index += 1;
    }
    e.call(LIMBS_ARRAY_DROP, &args![array]);
    extra_drop(e, this);
}

// Typed from 00430390
/// A `DismemberedLimb`'s scalar deleting destructor. Answers `this`.
pub fn limb_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_03c0, limb_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004303c0
/// A `DismemberedLimb`'s destructor: its object array dropped (`0042ff80`).
pub fn limb_drop(e: &mut Engine, this: u32) {
    let objects = this.wrapping_add(LIMB_OBJECTS);
    typed_call!(
        FAMILY,
        0x0042_ff80,
        arrays::bound_object_array_drop(e, objects)
    );
}

// Typed from 004303e0
/// `ExtraDismemberedLimbs::Dismembered`: bit `limb` (modulo 32, as `SHL` takes it) of the limbs word.
pub fn limbs_dismembered(e: &mut Engine, this: u32, limb: u8) -> bool {
    let limbs = u32::from(e.mem.u16(this.wrapping_add(0x0c)));
    limbs & (1u32 << (limb & 31)) != 0
}

// Typed from 00430410
/// `ExtraDismemberedLimbs::Dismember`: unless `limb` is already off, its bit set and a new limb record
/// appended (`0042c470` in a 0x14-byte block, `007cb2e0`). An exploded limb is marked identical. Else,
/// for an actor (slots 0x100 and 0x218) other than the player, its object array is built into the
/// record (`00605fc0`); when the newest earlier record not itself marked identical has the same
/// objects (`004387b0`), the record is marked identical and its array cleared (`008454f0`).
pub fn limbs_dismember(e: &mut Engine, this: u32, actor: u32, limb: u8, exploded: u8) {
    if typed_call!(FAMILY, 0x0043_03e0, limbs_dismembered(e, this, limb)) {
        return;
    }
    let at = this.wrapping_add(0x0c);
    let limbs = u32::from(e.mem.u16(at));
    e.mem.set_u16(at, (limbs | (1u32 << (limb & 31))) as u16);
    let block = e.call(OPERATOR_NEW, &args![0x14u32]).u32();
    let record = if block == 0 {
        0
    } else {
        typed_call!(FAMILY, 0x0042_c470, classes::flags_record_new(e, block))
    };
    let array = this.wrapping_add(LIMBS_ARRAY);
    // The exe appends the pointer in a stack local and reads the local again for the record.
    let (record, count) = e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), record);
        let count = e.call(ARRAY_ADD, &args![array, local]).u32();
        (e.mem.u32(local.addr()), count)
    });
    e.mem.set_u8(record, limb);
    e.mem.set_u8(record.wrapping_add(1), exploded);
    e.mem.set_u8(record.wrapping_add(2), 0);
    if exploded != 0 {
        e.mem.set_u8(record.wrapping_add(2), 1);
        return;
    }
    if !e.vcall(actor, 0x100, &[]).bool()
        || actor == e.mem.u32(PLAYER)
        || !e.vcall(actor, 0x218, &[]).bool()
    {
        return;
    }
    let npc = typed_call!(FAMILY, 0x0041_81e0, access::save_form(e, actor));
    let objects = record.wrapping_add(LIMB_OBJECTS);
    let base = e.vcall(actor, 0x1e8, &[]).u32();
    e.call(BUILD_OBJECT_ARRAY, &args![npc, actor, base, objects]);
    let mut index = (count as i32).wrapping_sub(1);
    let earlier = loop {
        if index < 0 {
            return;
        }
        let candidate = item_of(e, array, index as u32);
        if e.mem.u8(candidate.wrapping_add(2)) == 0 {
            break candidate;
        }
        index -= 1;
    };
    let earlier_objects = earlier.wrapping_add(LIMB_OBJECTS);
    if e.call(ARRAY_SAME_BUFFER, &args![earlier_objects, objects]).bool() {
        e.mem.set_u8(record.wrapping_add(2), 1);
        typed_call!(
            dispatch::FAMILY,
            0x0084_54f0,
            dispatch::refs::array_clear(e, Ptr::new(objects), 1)
        );
    }
}

// Typed from 004305f0
/// The removed flag of every limb record of `limb` set to `removed`.
pub fn limbs_set_removed(e: &mut Engine, this: u32, limb: u32, removed: u8) {
    let array = this.wrapping_add(LIMBS_ARRAY);
    let mut index = 0;
    while index < count_of(e, array) {
        let record = item_of(e, array, index);
        if record != 0 && u32::from(e.mem.u8(record)) == limb {
            e.mem.set_u8(record.wrapping_add(3), removed);
        }
        index += 1;
    }
}

// Typed from 00430660
/// `ExtraDismemberedLimbs::CheckDismemberedLimbsIdentical`: the same limbs word, as many records, and
/// record by record the same four flags and the same objects (`004387b0`).
pub fn limbs_identical(e: &mut Engine, this: u32, other: u32) -> bool {
    if e.mem.u16(other.wrapping_add(0x0c)) != e.mem.u16(this.wrapping_add(0x0c)) {
        return false;
    }
    let theirs = other.wrapping_add(LIMBS_ARRAY);
    let mine = this.wrapping_add(LIMBS_ARRAY);
    if count_of(e, theirs) != count_of(e, mine) {
        return false;
    }
    let mut index = 0;
    while index < count_of(e, mine) {
        let a = item_of(e, mine, index);
        let b = item_of(e, theirs, index);
        if e.mem.u32(a) != e.mem.u32(b) {
            return false;
        }
        let same = e
            .call(
                ARRAY_SAME_BUFFER,
                &args![a.wrapping_add(LIMB_OBJECTS), b.wrapping_add(LIMB_OBJECTS)],
            )
            .bool();
        if !same {
            return false;
        }
        index += 1;
    }
    true
}

// ---- ExtraStartingPosition, the type 0x49 and ExtraLight ---------------------------------------

// Typed from 00430780
/// `ExtraStartingPosition`'s constructor from a reference (type 0x0f): its `FILE_POS_ROT` made
/// (`00692710`), the position the reference's slot 0x1f4 points at and its angles (+0x24) copied.
/// Answers `this`.
pub fn starting_position_new(e: &mut Engine, this: u32, reference: u32) -> u32 {
    extra_with_vtable(e, this, KIND_STARTING_POSITION, VTABLE_STARTING_POSITION);
    let start = this.wrapping_add(0x0c);
    e.call(FILE_POS_ROT_NEW, &args![start]);
    let position = e.vcall(reference, 0x1f4, &[]).u32();
    copy_point(e, position, start);
    copy_point(e, angles_of(e, reference), start.wrapping_add(0x0c));
    this
}

/// Three words copied (a point moved as its bits).
#[inline]
fn copy_point(e: &mut Engine, from: u32, to: u32) {
    for offset in [0, 4, 8] {
        let word = e.mem.u32(from.wrapping_add(offset));
        e.mem.set_u32(to.wrapping_add(offset), word);
    }
}

// Typed from 00430830
/// A reference's angles: the address of its point at +0x24.
pub fn angles_of(_e: &mut Engine, this: u32) -> u32 {
    this.wrapping_add(0x24)
}

// Typed from 00430850
/// `ExtraStartingPosition::Compare`: the 0x18 bytes of the positions (`memcmp`).
pub fn starting_position_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_bytes(e, this, other, TYPE_STARTING_POSITION, 0x18)
}

// Typed from 004308c0
/// The type 0x49's constructor (`ExtraStartingWorldOrCell`): no form. Answers `this`.
pub fn type_49_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_49, VTABLE_49, 0)
}

// Typed from 004308f0
/// `ExtraLight`'s constructor (type 0x29) with its light. Answers `this`.
pub fn light_new(e: &mut Engine, this: u32, light: u32) -> u32 {
    with_word(e, this, KIND_LIGHT, VTABLE_LIGHT, light)
}

// Typed from 00430920
/// `ExtraLight`'s scalar deleting destructor. Answers `this`.
pub fn light_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_0950, light_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00430950
/// `ExtraLight`'s destructor: the owned light deleted (`00418f10`).
pub fn light_drop(e: &mut Engine, this: u32) {
    drop_owner(e, this, VTABLE_LIGHT, LIGHT_DELETE);
}

// ---- REFR_LOCK and ExtraLock --------------------------------------------------------------------

// Typed from 004309e0
/// `REFR_LOCK::GetLockLevel`: the level's bracket.
pub fn lock_level(e: &mut Engine, this: u32, reference: u32) -> i32 {
    let level = typed_call!(FAMILY, 0x0043_0a10, lock_base_level(e, this, reference));
    typed_call!(FAMILY, 0x0043_0b40, lock_bracket(e, level))
}

// Typed from 00430a10
/// `REFR_LOCK::GetLevel`: the base level, and for a leveled lock (flag 4) the leveled lock setting
/// (truncated to an `int`) times the reference's calculated level (the player's level for none) added,
/// at most 99.
pub fn lock_base_level(e: &mut Engine, this: u32, reference: u32) -> i32 {
    let mut level = i32::from(e.mem.u8(this));
    if e.mem.u8(this.wrapping_add(8)) & 4 != 0 {
        let by = if reference == 0 {
            let player = e.mem.u32(PLAYER);
            i32::from(e.call(ACTOR_LEVEL, &args![player]).u16())
        } else {
            e.call(REFERENCE_CALC_LEVEL, &args![reference, 0u32]).i32()
        };
        let setting = e.call(SETTING_FLOAT, &args![LEVELED_LOCK_SETTING]).u32();
        let scale = f64::from(e.mem.f32(setting));
        let whole = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, scale));
        let scale = crt::low_word(whole) as i32;
        level = scale.wrapping_mul(by).wrapping_add(level).min(99);
    }
    level
}

// Typed from 00430a90
/// `REFR_LOCK::SetLocked`: flag 1 set, or cleared with the tries counted from 0.
pub fn lock_set_locked(e: &mut Engine, this: u32, locked: bool) {
    let at = this.wrapping_add(8);
    let flags = e.mem.u8(at);
    if locked {
        e.mem.set_u8(at, flags | 1);
    } else {
        e.mem.set_u8(at, flags & !1);
        e.mem.set_u32(this.wrapping_add(0x0c), 0);
    }
}

// Typed from 00430ae0
/// `REFR_LOCK::IsBroken`: the tries at least 1, or 2 when the player's lock-broken perk entry (on 0.0)
/// gives other than 0.0.
pub fn lock_is_broken(e: &mut Engine, this: u32) -> bool {
    let player = e.mem.u32(PLAYER);
    let value = e.with_stack(4, |e, result| {
        e.mem.set_f32(result.addr(), 0.0);
        e.call(HANDLE_ENTRY_POINT, &args![ENTRY_LOCK_BROKEN, player, result]);
        e.mem.f32(result.addr())
    });
    let allowed = if f64::from(value) == e.mem.f64(ZERO_DOUBLE) {
        1
    } else {
        2
    };
    e.mem.u32(this.wrapping_add(0x0c)) >= allowed
}

// Typed from 00430b40
/// `REFR_LOCK::NumericValueToEnum`: the first of the five lock level settings `value` is at most, else
/// 5 (requires key).
pub fn lock_bracket(e: &mut Engine, value: i32) -> i32 {
    for (bracket, &setting) in LOCK_LEVEL_SETTINGS[..5].iter().enumerate() {
        if value <= setting_int(e, setting) {
            return bracket as i32;
        }
    }
    5
}

// Typed from 00430bc0
/// The lock level setting of `bracket` (0 to 5), else 0.
pub fn lock_bracket_value(e: &mut Engine, bracket: u32) -> i32 {
    match LOCK_LEVEL_SETTINGS.get(bracket as usize) {
        Some(&setting) => setting_int(e, setting),
        None => 0,
    }
}

// Typed from 00430c70
/// `ExtraLock::ExtraLock` (type 0x2a) with its lock record. Answers `this`.
pub fn lock_new(e: &mut Engine, this: u32, lock: u32) -> u32 {
    with_word(e, this, KIND_LOCK, VTABLE_LOCK, lock)
}

// Typed from 00430ca0
/// `ExtraLock`'s destructor: its lock record freed (`operator delete`, null or not).
pub fn lock_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_LOCK);
    let lock = word_at(e, this, EXTRA_PAYLOAD);
    e.call(OPERATOR_DELETE, &args![lock]);
    extra_drop(e, this);
}

// Typed from 00430ce0
/// `ExtraLock::Compare`: the lock records' flags, keys, level brackets, tries and times unlocked.
pub fn lock_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let Some(cast) = compare_start(e, this, other, TYPE_LOCK) else {
        return true;
    };
    let mine = word_at(e, this, EXTRA_PAYLOAD);
    let theirs = word_at(e, cast, EXTRA_PAYLOAD);
    if e.mem.u8(mine.wrapping_add(8)) != e.mem.u8(theirs.wrapping_add(8))
        || e.mem.u32(mine.wrapping_add(4)) != e.mem.u32(theirs.wrapping_add(4))
    {
        return true;
    }
    let my_level = i32::from(e.mem.u8(mine));
    let my_bracket = typed_call!(FAMILY, 0x0043_0b40, lock_bracket(e, my_level));
    let their_level = i32::from(e.mem.u8(theirs));
    let their_bracket = typed_call!(FAMILY, 0x0043_0b40, lock_bracket(e, their_level));
    my_bracket != their_bracket
        || e.mem.u32(mine.wrapping_add(0x0c)) != e.mem.u32(theirs.wrapping_add(0x0c))
        || e.mem.u32(mine.wrapping_add(0x10)) != e.mem.u32(theirs.wrapping_add(0x10))
}

// ---- ExtraFollower, ExtraGuardedRefData, the type 0x2e, ExtraTeleport ---------------------------

// Typed from 00430dd0
/// `ExtraFollower::ExtraFollower` (type 0x1d): an empty actor list (null when the allocation
/// failed). Answers `this`.
pub fn follower_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_FOLLOWER, VTABLE_FOLLOWER);
    let list = new_actor_list(e);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), list);
    this
}

// Typed from 00430e70
/// `ExtraFollower`'s scalar deleting destructor. Answers `this`.
pub fn follower_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_0ea0, follower_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00430ea0
/// `ExtraFollower`'s destructor: its actor list cleared (`00470470`) and deleted (`004702f0`, the list
/// read again after the clear).
pub fn follower_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_FOLLOWER);
    let list = word_at(e, this, EXTRA_PAYLOAD);
    e.call(ACTOR_LIST_CLEAR, &args![list]);
    let list = word_at(e, this, EXTRA_PAYLOAD);
    if list != 0 {
        e.call(ACTOR_LIST_DELETE, &args![list, 1u32]);
    }
    extra_drop(e, this);
}

// Typed from 00430f30
/// `ExtraGuardedRefData::ExtraGuardedRefData` (type 0x7c): its guards' array made (`00438660`).
/// Answers `this`.
pub fn guarded_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_GUARDED_REF_DATA, VTABLE_GUARDED_REF_DATA);
    e.call(GUARDS_ARRAY_NEW, &args![this.wrapping_add(EXTRA_PAYLOAD)]);
    this
}

// Typed from 00430fa0
/// `ExtraGuardedRefData`'s scalar deleting destructor. Answers `this`.
pub fn guarded_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_0fd0, guarded_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00430fd0
/// `ExtraGuardedRefData`'s destructor: its guards' array dropped (`00438690`).
pub fn guarded_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_GUARDED_REF_DATA);
    e.call(GUARDS_ARRAY_DROP, &args![this.wrapping_add(EXTRA_PAYLOAD)]);
    extra_drop(e, this);
}

// Typed from 00431030
/// `ExtraGuardedRefData::Compare`, as the exe has it: true at once when `BSExtraData::Compare` says the
/// two do NOT differ; else whether the counts differ or a guard of this one is missing from the other
/// (the dispatch family's search with the word compare).
pub fn guarded_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    if !base_compare(e, this, other) {
        return true;
    }
    let theirs = other.wrapping_add(EXTRA_PAYLOAD);
    let mine = this.wrapping_add(EXTRA_PAYLOAD);
    if count_of(e, theirs) != count_of(e, mine) {
        return true;
    }
    let mut index = 0;
    while index < count_of(e, mine) {
        let guard = slot_of(e, mine, index);
        if find_guard(e, theirs, guard) == -1 {
            return true;
        }
        index += 1;
    }
    false
}

/// The index of the guard `*at` in the guards' array (`00719b20` with the word compare), else -1.
#[inline]
fn find_guard(e: &mut Engine, guards: u32, at: u32) -> i32 {
    typed_call!(
        dispatch::FAMILY,
        0x0071_9b20,
        dispatch::refs::array_find(e, Ptr::new(guards), at, 0, WORDS_EQUAL)
    )
}

// Typed from 004310d0
/// `ExtraGuardedRefData::AddGuard`: the reference's word at +0xc appended (`007cb2e0`) unless the
/// array holds it.
pub fn guarded_add_guard(e: &mut Engine, this: u32, reference: u32) {
    let guard = e.mem.u32(reference.wrapping_add(0x0c));
    let guards = this.wrapping_add(EXTRA_PAYLOAD);
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), guard);
        if find_guard(e, guards, local.addr()) == -1 {
            e.call(ARRAY_ADD, &args![guards, local]);
        }
    });
}

// Typed from 00431120
/// Each guard told (its process's slot 0x60c with the guard, `second`, `first`): the guard's form by
/// id (the forms family's), when it is an actor (slots 0xf0 and 0x100) with a process (+0x68).
pub fn guarded_notify(e: &mut Engine, this: u32, first: u32, second: u32) {
    let guards = this.wrapping_add(EXTRA_PAYLOAD);
    let mut index = 0;
    while index < count_of(e, guards) {
        let id = item_of(e, guards, index);
        let form = typed_call!(forms::FAMILY, 0x0048_39c0, forms::tesform::form_by_id(e, id));
        if form != 0 && e.vcall(form, 0xf0, &[]).bool() && e.vcall(form, 0x100, &[]).bool() {
            let process = e.mem.u32(form.wrapping_add(0x68));
            if process != 0 {
                e.vcall(process, 0x60c, &args![form, second, first]);
            }
        }
        index += 1;
    }
}

// Typed from 004311f0
/// The type 0x2e's constructor (`ExtraLeveledCreature`): two null words. Answers `this`.
pub fn type_2e_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_2E, VTABLE_2E, 0);
    e.mem.set_u32(this.wrapping_add(0x10), 0);
    this
}

// Typed from 00431230
/// `ExtraTeleport`'s constructor (type 0x2b) with its data. Answers `this`.
pub fn teleport_new(e: &mut Engine, this: u32, data: u32) -> u32 {
    with_word(e, this, KIND_TELEPORT, VTABLE_TELEPORT, data)
}

// Typed from 00431260
/// `ExtraTeleport`'s destructor: its data deleted (`00419220`, this family's).
pub fn teleport_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_TELEPORT);
    let data = word_at(e, this, EXTRA_PAYLOAD);
    if data != 0 {
        typed_call!(FAMILY, 0x0041_9220, access::teleport_data_delete(e, data, 1));
    }
    extra_drop(e, this);
}

// Typed from 004312f0
/// `ExtraTeleport::Compare`: the door teleport data's compare (`0043a860`).
pub fn teleport_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_by(e, this, other, TYPE_TELEPORT, DOOR_TELEPORT_COMPARE)
}

// ---- The markers ------------------------------------------------------------------------------

// Typed from 00431360
/// `ExtraMapMarker`'s constructor (type 0x2c) with its data. Answers `this`.
pub fn map_marker_new(e: &mut Engine, this: u32, data: u32) -> u32 {
    with_word(e, this, KIND_MAP_MARKER, VTABLE_MAP_MARKER, data)
}

// Typed from 00431390
/// `ExtraMapMarker`'s scalar deleting destructor. Answers `this`.
pub fn map_marker_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_13c0, map_marker_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004313c0
/// `ExtraMapMarker`'s destructor: its data deleted (`00419350`, this family's).
pub fn map_marker_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_MAP_MARKER);
    let data = word_at(e, this, EXTRA_PAYLOAD);
    if data != 0 {
        typed_call!(FAMILY, 0x0041_9350, access::map_marker_data_delete(e, data, 1));
    }
    extra_drop(e, this);
}

// Typed from 00431450
/// `ExtraMapMarker::Compare`: the map marker data's compare (`00438e40`).
pub fn map_marker_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_by(e, this, other, TYPE_MAP_MARKER, MAP_MARKER_DATA_COMPARE)
}

// Typed from 004314c0
/// `ExtraAudioMarker`'s constructor (type 0x90) with its data. Answers `this`.
pub fn audio_marker_new(e: &mut Engine, this: u32, data: u32) -> u32 {
    with_word(e, this, KIND_AUDIO_MARKER, VTABLE_AUDIO_MARKER, data)
}

// Typed from 004314f0
/// `ExtraAudioMarker`'s scalar deleting destructor. Answers `this`.
pub fn audio_marker_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_1520, audio_marker_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00431520
/// `ExtraAudioMarker`'s destructor: its data deleted (`00419480`, this family's).
pub fn audio_marker_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_AUDIO_MARKER);
    let data = word_at(e, this, EXTRA_PAYLOAD);
    if data != 0 {
        typed_call!(FAMILY, 0x0041_9480, access::audio_marker_data_delete(e, data, 1));
    }
    extra_drop(e, this);
}

// Typed from 004315b0
/// `ExtraAudioMarker::Compare`: the audio marker data's compare (`005897c0`).
pub fn audio_marker_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_by(e, this, other, TYPE_AUDIO_MARKER, AUDIO_MARKER_DATA_COMPARE)
}

// Typed from 00431620
/// `ExtraAudioBuoyMarker`'s constructor (type 0x91) with its data. Answers `this`.
pub fn audio_buoy_new(e: &mut Engine, this: u32, data: u32) -> u32 {
    with_word(e, this, KIND_AUDIO_BUOY_MARKER, VTABLE_AUDIO_BUOY_MARKER, data)
}

// Typed from 00431650
/// `ExtraAudioBuoyMarker`'s scalar deleting destructor. Answers `this`.
pub fn audio_buoy_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_1680, audio_buoy_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00431680
/// `ExtraAudioBuoyMarker`'s destructor: its data deleted (`007b3fa0`).
pub fn audio_buoy_drop(e: &mut Engine, this: u32) {
    drop_owner(e, this, VTABLE_AUDIO_BUOY_MARKER, AUDIO_BUOY_DATA_DELETE);
}

// Typed from 00431710
/// `ExtraAudioBuoyMarker::Compare`: the buoy data's compare (`00401290`).
pub fn audio_buoy_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_by(e, this, other, TYPE_AUDIO_BUOY_MARKER, AUDIO_BUOY_DATA_COMPARE)
}

// ---- ExtraAction, ExtraContainerChanges and the one-word classes ------------------------------

// Typed from 00431780
/// `ExtraAction`'s constructor (type 0x0e): action 1, no reference. Answers `this`.
pub fn action_new(e: &mut Engine, this: u32) -> u32 {
    with_byte(e, this, KIND_ACTION, VTABLE_ACTION, 1);
    e.mem.set_u32(this.wrapping_add(0x10), 0);
    this
}

// Typed from 004317c0
/// `ExtraAction::Compare`: the action byte.
pub fn action_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_byte(e, this, other, TYPE_ACTION)
}

// Typed from 00431830
/// `ExtraContainerChanges`' constructor (type 0x15) with its changes. Answers `this`.
pub fn container_changes_new(e: &mut Engine, this: u32, changes: u32) -> u32 {
    with_word(e, this, KIND_CONTAINER_CHANGES, VTABLE_CONTAINER_CHANGES, changes)
}

// Typed from 00431860
/// `ExtraContainerChanges`' scalar deleting destructor. Answers `this`.
pub fn container_changes_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_1890, container_changes_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00431890
/// `ExtraContainerChanges`' destructor: its changes deleted (`00431920` with 1).
pub fn container_changes_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_CONTAINER_CHANGES);
    let changes = word_at(e, this, EXTRA_PAYLOAD);
    if changes != 0 {
        typed_call!(FAMILY, 0x0043_1920, inventory_changes_delete(e, changes, 1));
    }
    extra_drop(e, this);
}

// Typed from 00431920
/// The inventory changes' scalar deleting destructor: their destructor (`004bf150`), then
/// `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn inventory_changes_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(INVENTORY_CHANGES_DROP, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00431950
/// `ExtraOriginalReference`'s constructor (type 0x20) with its reference. Answers `this`.
pub fn original_reference_new(e: &mut Engine, this: u32, reference: u32) -> u32 {
    with_word(e, this, KIND_ORIGINAL_REFERENCE, VTABLE_ORIGINAL_REFERENCE, reference)
}

// Typed from 00431980
/// `ExtraOriginalReference`'s scalar deleting destructor. Answers `this`.
pub fn original_reference_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_19b0, original_reference_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004319b0
/// `ExtraOriginalReference`'s destructor.
pub fn original_reference_drop(e: &mut Engine, this: u32) {
    drop_plain(e, this, VTABLE_ORIGINAL_REFERENCE);
}

// Typed from 004319d0
/// `ExtraOriginalReference::Compare`: the reference.
pub fn original_reference_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_word(e, this, other, TYPE_ORIGINAL_REFERENCE)
}

// Typed from 00431a40
/// `ExtraOwnership`'s constructor (type 0x21) with its owner. Answers `this`.
pub fn ownership_new(e: &mut Engine, this: u32, owner: u32) -> u32 {
    with_word(e, this, KIND_OWNERSHIP, VTABLE_OWNERSHIP, owner)
}

// Typed from 00431a70
/// `ExtraOwnership::Compare`: the owner.
pub fn ownership_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_word(e, this, other, TYPE_OWNERSHIP)
}

// Typed from 00431ae0
/// `ExtraGlobal`'s constructor (type 0x22) with its global. Answers `this`.
pub fn global_new(e: &mut Engine, this: u32, global: u32) -> u32 {
    with_word(e, this, KIND_GLOBAL, VTABLE_GLOBAL, global)
}

// Typed from 00431b10
/// `ExtraGlobal::Compare`: the global.
pub fn global_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_word(e, this, other, TYPE_GLOBAL)
}

// Typed from 00431b80
/// `ExtraRank`'s constructor (type 0x23) with its rank. Answers `this`.
pub fn rank_new(e: &mut Engine, this: u32, rank: u32) -> u32 {
    with_word(e, this, KIND_RANK, VTABLE_RANK, rank)
}

// Typed from 00431bb0
/// `ExtraRank::Compare`: the rank.
pub fn rank_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_word(e, this, other, TYPE_RANK)
}

// Typed from 00431c20
/// `ExtraCount`'s constructor (type 0x24) with its count (a 16-bit store). Answers `this`.
pub fn count_new(e: &mut Engine, this: u32, count: i16) -> u32 {
    extra_with_vtable(e, this, KIND_COUNT, VTABLE_COUNT);
    e.mem.set_u16(this.wrapping_add(EXTRA_PAYLOAD), count as u16);
    this
}

// Typed from 00431c50
/// `ExtraCount::Compare`: the 16-bit count.
pub fn count_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    match compare_start(e, this, other, TYPE_COUNT) {
        None => true,
        Some(cast) => {
            e.mem.u16(this.wrapping_add(EXTRA_PAYLOAD)) != e.mem.u16(cast.wrapping_add(EXTRA_PAYLOAD))
        }
    }
}

// Typed from 00431cc0
/// `ExtraLeveledItem::Compare`: only the cast and `BSExtraData::Compare`.
pub fn leveled_item_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_start(e, this, other, TYPE_LEVELED_ITEM).is_none()
}

// Typed from 00431d10
/// `ExtraHealth`'s constructor (type 0x25) with its health, through the x87 (a signalling NaN comes
/// out quiet). Answers `this`.
pub fn health_new(e: &mut Engine, this: u32, health: u32) -> u32 {
    let quiet = engine_core::x87::x87_quiet_bits(health);
    with_word(e, this, KIND_HEALTH, VTABLE_HEALTH, quiet)
}

// Typed from 00431d40
/// `ExtraHealth::Compare`: the health.
pub fn health_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_float(e, this, other, TYPE_HEALTH)
}

// Typed from 00431db0
/// `ExtraHealthPerc::Compare`: the health fraction.
pub fn health_perc_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_float(e, this, other, TYPE_HEALTH_PERC)
}

// Typed from 00431e20
/// `ExtraUses`' constructor (type 0x26) with its uses byte. Answers `this`.
pub fn uses_new(e: &mut Engine, this: u32, uses: u8) -> u32 {
    with_byte(e, this, KIND_USES, VTABLE_USES, uses)
}

// Typed from 00431e50
/// `ExtraUses::Compare`: the uses byte.
pub fn uses_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_byte(e, this, other, TYPE_USES)
}

// Typed from 00431ec0
/// `ExtraTimeLeft`'s constructor (type 0x27) with its time (moved as its bits). Answers `this`.
pub fn time_left_new(e: &mut Engine, this: u32, time: u32) -> u32 {
    with_word(e, this, KIND_TIME_LEFT, VTABLE_TIME_LEFT, time)
}

// Typed from 00431ef0
/// `ExtraTimeLeft::Compare`: the time.
pub fn time_left_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_float(e, this, other, TYPE_TIME_LEFT)
}

// Typed from 00431f60
/// `ExtraCharge`'s constructor (type 0x28) with its charge (moved as its bits). Answers `this`.
pub fn charge_new(e: &mut Engine, this: u32, charge: u32) -> u32 {
    with_word(e, this, KIND_CHARGE, VTABLE_CHARGE, charge)
}

// Typed from 00431f90
/// `ExtraCharge::Compare`: the charge.
pub fn charge_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_float(e, this, other, TYPE_CHARGE)
}

// Typed from 00432000
/// `ExtraScript::ExtraScript` (type 0x0d) with its script and no variables. Answers `this`.
pub fn script_new(e: &mut Engine, this: u32, script: u32) -> u32 {
    with_word(e, this, KIND_SCRIPT, VTABLE_SCRIPT, script);
    e.mem.set_u32(this.wrapping_add(0x10), 0);
    this
}

// Typed from 00432040
/// `ExtraScript`'s destructor: its variables deleted (`0041af70` by address: xNVSE wraps it) and
/// cleared.
pub fn script_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_SCRIPT);
    let at = this.wrapping_add(0x10);
    let variables = e.mem.u32(at);
    if variables != 0 {
        e.call(SCRIPT_LOCALS_DELETE, &args![variables, 1u32]);
    }
    e.mem.set_u32(at, 0);
    extra_drop(e, this);
}

// Typed from 004320d0
/// `ExtraScript::Compare`: the script.
pub fn script_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_word(e, this, other, TYPE_SCRIPT)
}

// Typed from 00432140
/// `ExtraWeaponModFlags::Compare`: the flags byte.
pub fn weapon_mod_flags_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_byte(e, this, other, TYPE_WEAPON_MOD_FLAGS)
}

// Typed from 004321b0
/// `ExtraModdingItem::Compare`: the modding byte.
pub fn modding_item_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_byte(e, this, other, TYPE_MODDING_ITEM)
}

// Typed from 00432220
/// `ExtraScale`'s constructor (type 0x30) with its scale (moved as its bits). Answers `this`.
pub fn scale_new(e: &mut Engine, this: u32, scale: u32) -> u32 {
    with_word(e, this, KIND_SCALE, VTABLE_SCALE, scale)
}

// Typed from 00432250
/// `ExtraScale::Compare`: the scale.
pub fn scale_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_float(e, this, other, TYPE_SCALE)
}

// Typed from 004322c0
/// `ExtraGhost`'s constructor (type 0x1f). Answers `this`.
pub fn ghost_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_GHOST, VTABLE_GHOST);
    this
}

// Typed from 004322f0
/// `ExtraWorn`'s constructor (type 0x16). Answers `this`.
pub fn worn_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_WORN, VTABLE_WORN);
    this
}

// Typed from 00432320
/// `ExtraWornLeft`'s constructor (type 0x17). Answers `this`.
pub fn worn_left_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_WORN_LEFT, VTABLE_WORN_LEFT);
    this
}

// Typed from 00432350
/// `ExtraCannotWear`'s constructor (type 0x3e). Answers `this`.
pub fn cannot_wear_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_CANNOT_WEAR, VTABLE_CANNOT_WEAR);
    this
}

// Typed from 00432380
/// `ExtraHotKey`'s constructor (type 0x4a) with its hot key byte. Answers `this`.
pub fn hot_key_new(e: &mut Engine, this: u32, hot_key: u8) -> u32 {
    with_byte(e, this, KIND_HOT_KEY, VTABLE_HOT_KEY, hot_key)
}

// ---- The info general topic, the seed, the package start location ------------------------------

// Typed from 004323b0
/// `ExtraInfoGeneralTopic`'s destructor: the topic's general flag (+0x24) cleared, even for none (the
/// exe's store), the topic deleted (`00425ff0`).
pub fn topic_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_INFO_GENERAL_TOPIC);
    let topic = word_at(e, this, EXTRA_PAYLOAD);
    e.mem.set_u8(topic.wrapping_add(0x24), 0);
    if topic != 0 {
        e.call(MENU_TOPIC_DELETE, &args![topic, 1u32]);
    }
    extra_drop(e, this);
}

// Typed from 00432440
/// `ExtraInfoGeneralTopic`'s scalar deleting destructor. Answers `this`.
pub fn topic_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_23b0, topic_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00432470
/// `ExtraInfoGeneralTopic`'s constructor (type 0x4d) with its topic. Answers `this`.
pub fn topic_new(e: &mut Engine, this: u32, topic: u32) -> u32 {
    with_word(e, this, KIND_INFO_GENERAL_TOPIC, VTABLE_INFO_GENERAL_TOPIC, topic)
}

// Typed from 004324a0
/// `ExtraInfoGeneralTopic`'s constructor with a new menu topic (`0083da50` in a 0x2c-byte block, null
/// when the allocation failed). Answers `this`.
pub fn topic_new_empty(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_INFO_GENERAL_TOPIC, VTABLE_INFO_GENERAL_TOPIC);
    let block = e.call(OPERATOR_NEW, &args![0x2cu32]).u32();
    let topic = if block == 0 {
        0
    } else {
        e.call(MENU_TOPIC_NEW, &args![block]).u32()
    };
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), topic);
    this
}

// Typed from 00432540
/// `ExtraHotKey::Compare`: the hot key byte.
pub fn hot_key_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_byte(e, this, other, TYPE_HOT_KEY)
}

// Typed from 004325b0
/// `ExtraSeed`'s constructor (type 0x31) with its seed byte. Answers `this`.
pub fn seed_new(e: &mut Engine, this: u32, seed: u8) -> u32 {
    with_byte(e, this, KIND_SEED, VTABLE_SEED, seed)
}

// Typed from 004325e0
/// `ExtraSeed::Compare`: the seed byte.
pub fn seed_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_byte(e, this, other, TYPE_SEED)
}

// Typed from 00432650
/// `ExtraPackageStartLocation`'s constructor (type 0x18): its world location (`006d5320` in place:
/// nothing to store). Answers `this`.
pub fn start_location_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_PACKAGE_START_LOCATION, VTABLE_PACKAGE_START_LOCATION);
    this
}

// Typed from 004326c0
/// `ExtraPackageStartLocation`'s constructor with its place: the form (or `fallback` for none), the
/// point at `position` and the z angle, moved as their bits. Answers `this`.
pub fn start_location_at(
    e: &mut Engine,
    this: u32,
    form: u32,
    fallback: u32,
    position: u32,
    z_angle: u32,
) -> u32 {
    typed_call!(FAMILY, 0x0043_2650, start_location_new(e, this));
    let location = this.wrapping_add(0x0c);
    e.mem.set_u32(location, if form == 0 { fallback } else { form });
    copy_point(e, position, location.wrapping_add(4));
    e.mem.set_u32(location.wrapping_add(0x10), z_angle);
    this
}

// Typed from 00432770
/// `ExtraPackageStartLocation::Compare`: the 0x14 bytes of the world locations (`memcmp`).
pub fn start_location_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_bytes(e, this, other, TYPE_PACKAGE_START_LOCATION, 0x14)
}

// ---- The reference pointer, the packages, the crime list, the leveled item, the cell -----------

// Typed from 004327e0
/// `ExtraReferencePointer`'s constructor (type 0x1c) with its reference. Answers `this`.
pub fn reference_pointer_new(e: &mut Engine, this: u32, reference: u32) -> u32 {
    with_word(e, this, KIND_REFERENCE_POINTER, VTABLE_REFERENCE_POINTER, reference)
}

// Typed from 00432810
/// `ExtraPackage`'s constructor (type 0x19): no package, index -1, no target, the three flags clear.
/// Answers `this`.
pub fn package_new(e: &mut Engine, this: u32) -> u32 {
    package_new_with(e, this, 0, u32::MAX, 0, 0, 0, 0)
}

// Typed from 00432870
/// `ExtraPackage`'s constructor with its package, index, target and three flags. Answers `this`.
#[allow(clippy::too_many_arguments)]
pub fn package_new_with(
    e: &mut Engine,
    this: u32,
    package: u32,
    index: u32,
    target: u32,
    action_complete: u8,
    activated: u8,
    done_once: u8,
) -> u32 {
    with_word(e, this, KIND_PACKAGE, VTABLE_PACKAGE, package);
    e.mem.set_u32(this.wrapping_add(0x10), index);
    e.mem.set_u32(this.wrapping_add(0x14), target);
    e.mem.set_u8(this.wrapping_add(0x18), action_complete);
    e.mem.set_u8(this.wrapping_add(0x19), activated);
    e.mem.set_u8(this.wrapping_add(0x1a), done_once);
    this
}

// Typed from 004328d0
/// `ExtraTresPassPackage`'s constructor (type 0x1a) with its package. Answers `this`.
pub fn trespass_new(e: &mut Engine, this: u32, package: u32) -> u32 {
    with_word(e, this, KIND_TRESPASS_PACKAGE, VTABLE_TRESPASS_PACKAGE, package)
}

// Typed from 00432900
/// `ExtraTresPassPackage`'s destructor: its package marked created (`00674d70`) and deleted through
/// its slot 0x10 (the save-load object's test before it, `0047c850`, always answers false: in place).
pub fn trespass_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_TRESPASS_PACKAGE);
    let package = word_at(e, this, EXTRA_PAYLOAD);
    if package != 0 {
        e.call(PACKAGE_SET_CREATED, &args![package, 1u32]);
        let package = word_at(e, this, EXTRA_PAYLOAD);
        if package != 0 {
            e.vcall(package, 0x10, &args![1u32]);
        }
    }
    extra_drop(e, this);
}

/// The player crime list's start (the two constructors share it): type 0x35, an empty actor list.
#[inline]
fn crime_list_init(e: &mut Engine, this: u32) {
    extra_with_vtable(e, this, KIND_PLAYER_CRIME_LIST, VTABLE_PLAYER_CRIME_LIST);
    let list = new_actor_list(e);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), list);
}

// Typed from 004329d0
/// `ExtraPlayerCrimeList`'s constructor (type 0x35): an empty list. Answers `this`.
pub fn crime_list_new(e: &mut Engine, this: u32) -> u32 {
    crime_list_init(e, this);
    this
}

// Typed from 00432a70
/// `ExtraPlayerCrimeList`'s scalar deleting destructor. Answers `this`.
pub fn crime_list_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_2b50, crime_list_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00432aa0
/// `ExtraPlayerCrimeList`'s constructor with one crime added (`005ae3d0`). Answers `this`.
pub fn crime_list_new_with(e: &mut Engine, this: u32, crime: u32) -> u32 {
    crime_list_init(e, this);
    let list = word_at(e, this, EXTRA_PAYLOAD);
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), crime);
        e.call(CRIME_LIST_ADD, &args![list, local]);
    });
    this
}

// Typed from 00432b50
/// `ExtraPlayerCrimeList`'s destructor: its list deleted (`004702f0`).
pub fn crime_list_drop(e: &mut Engine, this: u32) {
    drop_owner(e, this, VTABLE_PLAYER_CRIME_LIST, ACTOR_LIST_DELETE);
}

// Typed from 00432be0
/// `ExtraLeveledItem`'s constructor (type 0x2f) with its index, the default flag set. Answers `this`.
pub fn leveled_item_new(e: &mut Engine, this: u32, index: u32) -> u32 {
    with_word(e, this, KIND_LEVELED_ITEM, VTABLE_LEVELED_ITEM, index);
    e.mem.set_u8(this.wrapping_add(0x10), 1);
    this
}

// Typed from 00432c20
/// `ExtraPersistentCell`'s constructor (type 0x0c) with its cell. Answers `this`.
pub fn persistent_cell_new(e: &mut Engine, this: u32, cell: u32) -> u32 {
    with_word(e, this, KIND_PERSISTENT_CELL, VTABLE_PERSISTENT_CELL, cell)
}

// Typed from 00432c60
/// `ExtraPersistentCell`'s scalar deleting destructor. Answers `this`.
pub fn persistent_cell_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_2c90, persistent_cell_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00432c90
/// `ExtraPersistentCell`'s destructor.
pub fn persistent_cell_drop(e: &mut Engine, this: u32) {
    drop_plain(e, this, VTABLE_PERSISTENT_CELL);
}
