//! `fallout shared/extradataobjects.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds the constructors, destructors and `Compare` methods of the
//! `BSExtraData` subclasses that hang on a reference's `ExtraDataList`
//! (`ExtraAnim`, `ExtraDismemberedLimbs`, `ExtraStartingPosition`,
//! `ExtraLight`, `ExtraLock`, `ExtraFollower`, `ExtraGuardedRefData`,
//! `ExtraTeleport`, ...), in address order, together with `REFR_LOCK`.
//!
//! Translated so far: the first 40 functions of the queue (`004300f0` to
//! `00431260`). The next session continues at `004312f0`
//! (`ExtraTeleport::Compare`).
//!
//! Notes for the next session:
//! - Every destructor and constructor here is compiled without inlining: the
//!   base `BSExtraData` constructor (`0040ec80`, takes the extra-data type
//!   byte) and destructor (`0040ecb0`) are separate calls, the vtable pointer
//!   is stored in between, and every `_scalar_deleting_destructor_` is the
//!   destructor followed by `operator delete` (`00401030`) when bit 0 of its
//!   flags is set.
//! - The compiler's exception-unwinding frames (`FS:[0]` chains, the
//!   `__CxxFrameHandler` state words) are not translated.
//! - Functions this unit still has to translate and that the ones below call
//!   by address: `00438570`, `004385a0` (the `DismemberedLimbs` array
//!   constructor/destructor) and `00438660`, `00438690` (the `Guards` array
//!   constructor/destructor).

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleArray;

/// `operator new(size)` (cdecl, one stack argument).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete(pointer)` (cdecl, one stack argument).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `BSExtraData::BSExtraData(type)` (the base constructor; `this`, then the
/// extra-data type byte).
const BS_EXTRA_DATA_CONSTRUCT: u32 = 0x0040_ec80;
/// `BSExtraData::~BSExtraData` (the base destructor body).
const BS_EXTRA_DATA_DESTRUCT: u32 = 0x0040_ecb0;
/// `BSExtraData::Compare` (Xbox PDB; `this`, other): true when `other` is
/// null or `004f1540` gives a different answer for the two.
const BS_EXTRA_DATA_COMPARE: u32 = 0x0040_f700;
/// `BSSimpleArray<T,1024>::size` (the folded accessor: `this` is the array,
/// the count is returned in EAX).
const SIMPLE_ARRAY_SIZE: u32 = 0x0044_ddc0;
/// `BSSimpleArray<T,1024>::operator[]` (`this` is the array, the index is
/// the argument): the address of the element slot.
const SIMPLE_ARRAY_AT: u32 = 0x006a_7ad0;
/// `BSSimpleArray<BGSBodyPart_P_1024>::AddUninitialized` (Xbox PDB name of
/// the folded body): appends the pointer stored at the address given and
/// returns the index of the new slot.
const SIMPLE_ARRAY_ADD: u32 = 0x007c_b2e0;
/// `BSSimpleArray<TESBoundObject_P_1024>::CompareBuffer<1024>` (Xbox PDB):
/// whether the `this` array holds the same elements as the one given.
const SIMPLE_ARRAY_COMPARE_BUFFER: u32 = 0x0043_87b0;
/// `BSSimpleArray<T,1024>::Find` of the folded body (`this` is the array;
/// the arguments are the address of the value, the first index to look at
/// and a comparison function; the index found, or -1).
const SIMPLE_ARRAY_FIND: u32 = 0x0071_9b20;
/// The comparison function `00719b20` is given by `ExtraGuardedRefData`.
const GUARD_COMPARE: u32 = 0x009a_3830;
/// The array's clear (`this` is the array, the argument says whether to free
/// the buffer); `CompareBuffer`'s caller uses it to drop an array that
/// duplicates an earlier one.
const SIMPLE_ARRAY_CLEAR: u32 = 0x0084_54f0;
/// `__RTDynamicCast(object, vfDelta, sourceType, targetType, isReference)`.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `memcmp(a, b, size)`.
const MEMCMP: u32 = 0x00ec_4835;
/// `_ftol2_sse` (`00ec62c0`): truncates the float in ST0 to an integer in
/// EAX. The uniform form has no ST0 argument, so the value is passed as an
/// `f64` argument (two words, exact for every `float`).
const FTOL: u32 = 0x00ec_62c0;

/// `RTTI Type Descriptor` of `BSExtraData`, the source type of the
/// `dynamic_cast`s in the `Compare` methods.
const BS_EXTRA_DATA_TYPE: u32 = 0x0118_3b2c;
/// `RTTI Type Descriptor` of `ExtraStartingPosition`.
const EXTRA_STARTING_POSITION_TYPE: u32 = 0x0118_4b10;
/// `RTTI Type Descriptor` of `ExtraLock`.
const EXTRA_LOCK_TYPE: u32 = 0x0118_4754;

/// `0.0` (`double`), what `REFR_LOCK::IsBroken` compares the entry point's
/// result with.
const ZERO: u32 = 0x0101_2060;
/// The global holding the `PlayerCharacter` pointer (`PlayerCharacter::pSingleton`).
const PLAYER: u32 = 0x011d_ea3c;

/// `Actor::GetLevel` (returns the level in AX; `this` is the actor).
const ACTOR_GET_LEVEL: u32 = 0x0087_f9f0;
/// `TESObjectREFR::GetCalcLevel(bool)` (Xbox PDB name).
const REFR_GET_CALC_LEVEL: u32 = 0x0056_7e10;
/// A float game-setting accessor: `this` is the setting (an exe global), the
/// result is the address of its value.
const SETTING_FLOAT_VALUE: u32 = 0x0040_3e20;
/// An integer game-setting accessor: `this` is the setting (an exe global),
/// the result is the address of its value.
const SETTING_INT_VALUE: u32 = 0x0043_d4d0;
/// `BGSEntryPoint::HandleEntryPoint(entryPoint, actor, result, ...)` (Xbox
/// PDB name, cdecl).
const HANDLE_ENTRY_POINT: u32 = 0x005e_58f0;
/// The entry point `REFR_LOCK::IsBroken` asks (its result tells whether the
/// player's perks make a failed attempt count less).
const ENTRY_POINT_LOCK_BROKEN: u32 = 0x20;

/// The float setting `REFR_LOCK::GetLevel` scales a leveled lock's level
/// bonus by.
const LEVELED_LOCK_SETTING: u32 = 0x011c_39b4;
/// The integer settings that hold the top of each lock difficulty bracket
/// (very easy, easy, average, hard, very hard), and the one `00430bc0` also
/// knows for the enumeration value 5 (key only). See
/// `crates/world/src/locks.rs`.
const LOCK_LEVEL_SETTINGS: [u32; 6] = [
    0x011c_3a4c,
    0x011c_3a30,
    0x011c_3a00,
    0x011c_3ab0,
    0x011c_399c,
    0x011c_3a24,
];

/// `ExtraAnim`'s vtable.
const EXTRA_ANIM_VTABLE: u32 = 0x0101_5b28;
/// `ExtraDismemberedLimbs`'s vtable.
const EXTRA_DISMEMBERED_LIMBS_VTABLE: u32 = 0x0101_5b34;
/// `ExtraStartingPosition`'s vtable.
const EXTRA_STARTING_POSITION_VTABLE: u32 = 0x0101_5b40;
/// The vtable of the `BSExtraData` subclass of type 0x49 built by `004308c0`.
const EXTRA_TYPE_49_VTABLE: u32 = 0x0101_5b4c;
/// `ExtraLight`'s vtable.
const EXTRA_LIGHT_VTABLE: u32 = 0x0101_5b58;
/// `ExtraLock`'s vtable.
const EXTRA_LOCK_VTABLE: u32 = 0x0101_589c;
/// `ExtraFollower`'s vtable.
const EXTRA_FOLLOWER_VTABLE: u32 = 0x0101_5b64;
/// `ExtraGuardedRefData`'s vtable.
const EXTRA_GUARDED_REF_DATA_VTABLE: u32 = 0x0101_5b70;
/// The vtable of the `BSExtraData` subclass of type 0x2e built by `004311f0`.
const EXTRA_TYPE_2E_VTABLE: u32 = 0x0101_5b7c;
/// `ExtraTeleport`'s vtable.
const EXTRA_TELEPORT_VTABLE: u32 = 0x0101_58a8;

/// Extra-data type bytes, as passed to the base constructor.
const TYPE_DISMEMBERED_LIMBS: u32 = 0x5f;
const TYPE_STARTING_POSITION: u32 = 0x0f;
const TYPE_49: u32 = 0x49;
const TYPE_LIGHT: u32 = 0x29;
const TYPE_LOCK: u32 = 0x2a;
const TYPE_FOLLOWER: u32 = 0x1d;
const TYPE_GUARDED_REF_DATA: u32 = 0x7c;
const TYPE_2E: u32 = 0x2e;
const TYPE_TELEPORT: u32 = 0x2b;

/// Constructor of the `DismemberedLimbs` array member of
/// `ExtraDismemberedLimbs` (`this` is the array). In this unit; not yet
/// translated.
const DISMEMBERED_LIMBS_ARRAY_CONSTRUCT: u32 = 0x0043_8570;
/// Destructor of the same array. In this unit; not yet translated.
const DISMEMBERED_LIMBS_ARRAY_DESTRUCT: u32 = 0x0043_85a0;
/// Constructor of the `Guards` array member of `ExtraGuardedRefData`. In
/// this unit; not yet translated.
const GUARDS_ARRAY_CONSTRUCT: u32 = 0x0043_8660;
/// Destructor of the same array. In this unit; not yet translated.
const GUARDS_ARRAY_DESTRUCT: u32 = 0x0043_8690;
/// Constructor of a `DismemberedLimb` entry (`this` is the 0x14-byte block).
const DISMEMBERED_LIMB_CONSTRUCT: u32 = 0x0042_c470;
/// Destructor body of the `ObjectArray` member of a `DismemberedLimb` (`this`
/// is the array).
const DISMEMBERED_LIMB_ARRAY_DESTRUCT: u32 = 0x0042_ff80;
/// `TESNPC::BuildObjectArray(reference, base, array)` (Xbox PDB name); `this`
/// is what `004181e0` returns for the reference.
const TESNPC_BUILD_OBJECT_ARRAY: u32 = 0x0060_5fc0;
/// Takes a reference and returns the object `TESNPC::BuildObjectArray` is
/// called on (a 19-byte accessor in `extradatalist.cpp`).
const REFR_NPC_ACCESSOR: u32 = 0x0041_81e0;
/// The constructor of the `FILE_POS_ROT` member of `ExtraStartingPosition`
/// (`this` is the member).
const FILE_POS_ROT_CONSTRUCT: u32 = 0x0069_2710;
/// Deletes the animation an `ExtraAnim` owns (`this` is the animation, the
/// argument says whether to free it).
const ANIMATION_DELETE: u32 = 0x0041_8d20;
/// Deletes the light an `ExtraLight` owns (`this` is the light, the argument
/// says whether to free it).
const LIGHT_DELETE: u32 = 0x0041_8f10;
/// Deletes the teleport data an `ExtraTeleport` owns (`this` is the data,
/// the argument says whether to free it).
const TELEPORT_DATA_DELETE: u32 = 0x0041_9220;
/// Constructor of the 8-byte actor list an `ExtraFollower` owns.
const ACTOR_LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// Clears the actor list (`this` is the list).
const ACTOR_LIST_CLEAR: u32 = 0x0047_0470;
/// Scalar deleting destructor of the actor list (`this` is the list, the
/// argument says whether to free it).
const ACTOR_LIST_DELETE: u32 = 0x0047_02f0;
/// Returns the value `ExtraGuardedRefData::AddGuard` stores for a reference
/// (`this` is the reference).
const REFR_GUARD_VALUE: u32 = 0x0084_e3a0;
/// `LookupFormByID(id)` (cdecl, one stack argument).
const LOOKUP_FORM_BY_ID: u32 = 0x0048_39c0;
/// Returns the process object a form's guard notification goes to (the
/// engine map's `MiddleHighProcess::GetSavedAcquireObject`; `this` is the
/// form).
const FORM_PROCESS: u32 = 0x008d_8520;

layout! {
    /// `NiPoint3` (Xbox PDB): three floats.
    pub struct NiPoint3: 0x0C {
        /// `x` (Xbox PDB).
        0x00 x: f32,
        /// `y` (Xbox PDB).
        0x04 y: f32,
        /// `z` (Xbox PDB).
        0x08 z: f32,
    }

    /// `FILE_POS_ROT` (Xbox PDB): a position and a rotation.
    pub struct FilePosRot: 0x18 {
        /// `pos` (Xbox PDB).
        0x00 pos: Inline<NiPoint3>,
        /// `rot` (Xbox PDB).
        0x0C rot: Inline<NiPoint3>,
    }

    /// `ExtraAnim` (Xbox PDB), 0x10 bytes. The `BSExtraData` base holds the
    /// vtable at +0, `cEtype` at +4 and `pNext` at +8.
    pub struct ExtraAnim: 0x10 {
        /// `pAnimation` (Xbox PDB): `Animation*`, owned.
        0x0C pAnimation: Ptr,
    }

    /// `ExtraDismemberedLimbs` (Xbox PDB), 0x30 bytes.
    pub struct ExtraDismemberedLimbs: 0x30 {
        /// `sLimbs` (Xbox PDB): bit `n` is set when limb `n` is dismembered.
        0x0C sLimbs: u16,
        /// `eCauseofDeath` (Xbox PDB).
        0x10 eCauseofDeath: u32,
        /// `pDeathObject` (Xbox PDB): `TESForm*`.
        0x14 pDeathObject: Ptr,
        /// `eLastHitLimb` (Xbox PDB).
        0x18 eLastHitLimb: u32,
        /// `bEaten` (Xbox PDB).
        0x1C bEaten: bool,
        /// `DismemberedLimbs` (Xbox PDB): `BSSimpleArray<DismemberedLimb *,1024>`.
        0x20 DismemberedLimbs: Inline<BSSimpleArray>,
    }

    /// `DismemberedLimb` (Xbox PDB), 0x14 bytes.
    pub struct DismemberedLimb: 0x14 {
        /// `cLimb` (Xbox PDB).
        0x00 cLimb: u8,
        /// `bLimbExploded` (Xbox PDB; a `bool`, kept as the byte the caller
        /// passed).
        0x01 bLimbExploded: u8,
        /// `bObjectArrayIdentical` (Xbox PDB; a `bool`, kept as a byte).
        0x02 bObjectArrayIdentical: u8,
        /// `bLimbRemoved` (Xbox PDB; a `bool`, kept as the byte the caller
        /// passed).
        0x03 bLimbRemoved: u8,
        /// `ObjectArray` (Xbox PDB): `BSSimpleArray<TESBoundObject *,1024>`.
        0x04 ObjectArray: Inline<BSSimpleArray>,
    }

    /// `ExtraStartingPosition` (Xbox PDB), 0x24 bytes.
    pub struct ExtraStartingPosition: 0x24 {
        /// `startPosition` (Xbox PDB).
        0x0C startPosition: Inline<FilePosRot>,
    }

    /// The `BSExtraData` subclass of type 0x49 (vtable `01015b4c`), 0x10
    /// bytes; the Xbox PDB name is not known.
    pub struct ExtraType49: 0x10 {
        /// The word `004308c0` clears.
        0x0C unnamed0C: u32,
    }

    /// `ExtraLight` (Xbox PDB), 0x10 bytes.
    pub struct ExtraLight: 0x10 {
        /// `pLight` (Xbox PDB): `REFR_LIGHT*`, owned.
        0x0C pLight: Ptr,
    }

    /// `REFR_LOCK` (Xbox PDB), 0x14 bytes.
    pub struct RefrLock: 0x14 {
        /// `cBaseLevel` (Xbox PDB).
        0x00 cBaseLevel: u8,
        /// `pKey` (Xbox PDB): `TESKey*`.
        0x04 pKey: Ptr,
        /// `cFlags` (Xbox PDB): bit 0 locked, bit 2 leveled.
        0x08 cFlags: i8,
        /// `uiNumTries` (Xbox PDB).
        0x0C uiNumTries: u32,
        /// `uiTimesUnlocked` (Xbox PDB).
        0x10 uiTimesUnlocked: u32,
    }

    /// `ExtraLock` (Xbox PDB), 0x10 bytes.
    pub struct ExtraLock: 0x10 {
        /// `pLock` (Xbox PDB): `REFR_LOCK*`, owned.
        0x0C pLock: Ptr<RefrLock>,
    }

    /// `ExtraFollower` (Xbox PDB), 0x10 bytes.
    pub struct ExtraFollower: 0x10 {
        /// `pActorlist` (Xbox PDB): `BSSimpleList<Actor *>*`, owned.
        0x0C pActorlist: Ptr,
    }

    /// `ExtraGuardedRefData` (Xbox PDB), 0x1C bytes.
    pub struct ExtraGuardedRefData: 0x1C {
        /// `Guards` (Xbox PDB): `BSSimpleArray<unsigned int,1024>`.
        0x0C Guards: Inline<BSSimpleArray>,
    }

    /// The `BSExtraData` subclass of type 0x2e (vtable `01015b7c`), 0x14
    /// bytes; the Xbox PDB name is not known.
    pub struct ExtraType2e: 0x14 {
        /// The first word `004311f0` clears.
        0x0C unnamed0C: u32,
        /// The second word `004311f0` clears.
        0x10 unnamed10: u32,
    }

    /// `ExtraTeleport` (Xbox PDB), 0x10 bytes.
    pub struct ExtraTeleport: 0x10 {
        /// `pData` (Xbox PDB): the teleport data, owned.
        0x0C pData: Ptr,
    }
}

/// The count of a `BSSimpleArray`.
fn array_size(e: &mut Engine, array: Ptr<BSSimpleArray>) -> u32 {
    e.call(SIMPLE_ARRAY_SIZE, &args![array]).u32()
}

/// The address of slot `index` of a `BSSimpleArray`.
fn array_slot(e: &mut Engine, array: Ptr<BSSimpleArray>, index: u32) -> Ptr {
    e.call(SIMPLE_ARRAY_AT, &args![array, index]).ptr()
}

/// The pointer stored in slot `index` of a `BSSimpleArray` of pointers.
fn array_pointer_at<T>(e: &mut Engine, array: Ptr<BSSimpleArray>, index: u32) -> Ptr<T> {
    let slot = array_slot(e, array, index);
    Ptr::new(e.mem.u32(slot.addr()))
}

/// `_ftol2_sse` on a `float`.
fn float_to_int(e: &mut Engine, value: f32) -> i32 {
    e.call(FTOL, &args![value as f64]).i32()
}

/// The base constructor, the vtable store that follows it.
fn construct_base(e: &mut Engine, this: Ptr, extra_type: u32, vtable: u32) {
    e.call(BS_EXTRA_DATA_CONSTRUCT, &args![this, extra_type]);
    e.mem.set_u32(this.addr(), vtable);
}

/// `operator delete(this)` when bit 0 of `flags` is set (the tail of every
/// scalar deleting destructor).
fn delete_when_asked(e: &mut Engine, this: Ptr, flags: u32) {
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
}

// Translated from 004300f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraAnim::_scalar_deleting_destructor_` (Xbox PDB): the destructor, then
/// `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_anim_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraAnim>,
    flags: u32,
) -> Ptr<ExtraAnim> {
    fn_00430120(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00430120 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraAnim`'s destructor body (the engine map has no name for it): resets
/// the vtable, deletes the owned animation if there is one, then runs the
/// base destructor. The exception-unwinding frame is not translated.
pub fn fn_00430120(e: &mut Engine, this: Ptr<ExtraAnim>) {
    e.mem.set_u32(this.addr(), EXTRA_ANIM_VTABLE);
    let animation = e.get(this, ExtraAnim::pAnimation);
    if !animation.is_null() {
        e.call(ANIMATION_DELETE, &args![animation, 1u32]);
    }
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 004301b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `u16` at `this + 0x4c + 2 * slot` of an animation-sequence object, where
/// `slot` is the section number except that section 0x14 (the whole body) uses
/// slot 1 and section 0x15 (the upper body) uses slot 4. The class is not named
/// by the Xbox PDB or the engine map.
pub fn fn_004301b0(e: &mut Engine, this: Ptr, section: i32) -> u16 {
    let slot = match section {
        0x14 => 1,
        0x15 => 4,
        other => other,
    };
    e.mem.u16(
        this.addr()
            .wrapping_add(0x4c)
            .wrapping_add((slot as u32).wrapping_mul(2)),
    )
}

// Translated from 00430200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDismemberedLimbs::ExtraDismemberedLimbs` (Xbox PDB): extra-data type
/// 0x5f, an empty limb array, no limbs dismembered, no cause of death
/// (`eCauseofDeath` and `eLastHitLimb` are -1), not eaten. Returns `this`.
pub fn extra_dismembered_limbs_extra_dismembered_limbs(
    e: &mut Engine,
    this: Ptr<ExtraDismemberedLimbs>,
) -> Ptr<ExtraDismemberedLimbs> {
    construct_base(
        e,
        this.cast(),
        TYPE_DISMEMBERED_LIMBS,
        EXTRA_DISMEMBERED_LIMBS_VTABLE,
    );
    let array = this.at(ExtraDismemberedLimbs::DismemberedLimbs);
    e.call(DISMEMBERED_LIMBS_ARRAY_CONSTRUCT, &args![array]);
    e.set(this, ExtraDismemberedLimbs::sLimbs, 0);
    e.set(this, ExtraDismemberedLimbs::eCauseofDeath, 0xffff_ffff);
    e.set(this, ExtraDismemberedLimbs::pDeathObject, Ptr::NULL);
    e.set(this, ExtraDismemberedLimbs::eLastHitLimb, 0xffff_ffff);
    e.set(this, ExtraDismemberedLimbs::bEaten, false);
    this
}

// Translated from 004302a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDismemberedLimbs::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_dismembered_limbs_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraDismemberedLimbs>,
    flags: u32,
) -> Ptr<ExtraDismemberedLimbs> {
    fn_004302d0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 004302d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDismemberedLimbs`' destructor body: deletes every non-null
/// `DismemberedLimb` in the array (the count is read again on every pass, as
/// the loop condition does), destroys the array, then runs the base
/// destructor. The exception-unwinding frame is not translated.
pub fn fn_004302d0(e: &mut Engine, this: Ptr<ExtraDismemberedLimbs>) {
    e.mem.set_u32(this.addr(), EXTRA_DISMEMBERED_LIMBS_VTABLE);
    let array = this.at(ExtraDismemberedLimbs::DismemberedLimbs);
    let mut index = 0;
    while index < array_size(e, array) {
        let limb: Ptr<DismemberedLimb> = array_pointer_at(e, array, index);
        if !limb.is_null() {
            fn_00430390(e, limb, 1);
        }
        index += 1;
    }
    e.call(DISMEMBERED_LIMBS_ARRAY_DESTRUCT, &args![array]);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 00430390 (decompiled, FalloutNV.exe 1.4.0.525)
/// `DismemberedLimb`'s scalar deleting destructor (the engine map has no
/// name): the destructor, then `operator delete` when `flags & 1`. Returns
/// `this`.
pub fn fn_00430390(e: &mut Engine, this: Ptr<DismemberedLimb>, flags: u32) -> Ptr<DismemberedLimb> {
    fn_004303c0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 004303c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `DismemberedLimb`'s destructor: destroys the `ObjectArray` member.
pub fn fn_004303c0(e: &mut Engine, this: Ptr<DismemberedLimb>) {
    let array = this.at(DismemberedLimb::ObjectArray);
    e.call(DISMEMBERED_LIMB_ARRAY_DESTRUCT, &args![array]);
}

// Translated from 004303e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDismemberedLimbs::Dismembered` (Xbox PDB): whether bit `limb` of
/// `sLimbs` is set. The shift count is taken modulo 32, as `SHL` does.
pub fn extra_dismembered_limbs_dismembered(
    e: &mut Engine,
    this: Ptr<ExtraDismemberedLimbs>,
    limb: u8,
) -> bool {
    let limbs = e.get(this, ExtraDismemberedLimbs::sLimbs) as u32;
    limbs & (1u32 << (limb & 31)) != 0
}

// Translated from 00430410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDismemberedLimbs::Dismember` (Xbox PDB): records that `limb` of
/// `actor` (a `TESObjectREFR*`) is dismembered, unless it already is.
///
/// A new `DismemberedLimb` (limb, `exploded`, not identical) is appended to
/// the array. An exploded limb is simply marked identical. Otherwise, for a
/// non-player actor whose vtable `+0x100` and `+0x218` functions are both
/// true, the actor's object array is built into the new entry, and if the
/// most recent earlier entry that is not itself marked identical has the same
/// object array, the new entry is marked identical and its array cleared
/// (`008454f0(array, 1)`).
///
/// The exception-unwinding frame is not translated.
pub fn extra_dismembered_limbs_dismember(
    e: &mut Engine,
    this: Ptr<ExtraDismemberedLimbs>,
    actor: Ptr,
    limb: u8,
    exploded: u8,
) {
    if extra_dismembered_limbs_dismembered(e, this, limb) {
        return;
    }
    let limbs = e.get(this, ExtraDismemberedLimbs::sLimbs) as u32;
    e.set(
        this,
        ExtraDismemberedLimbs::sLimbs,
        (limbs | (1u32 << (limb & 31))) as u16,
    );

    let memory = e.call(OPERATOR_NEW, &args![0x14u32]).u32();
    let entry: Ptr<DismemberedLimb> = if memory != 0 {
        e.call(DISMEMBERED_LIMB_CONSTRUCT, &args![memory]).ptr()
    } else {
        Ptr::NULL
    };
    let array = this.at(ExtraDismemberedLimbs::DismemberedLimbs);
    // The game appends the pointer held in a stack local and then reads the
    // local again to fill the entry in.
    let (entry, previous_count) = e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), entry.addr());
        let count = e.call(SIMPLE_ARRAY_ADD, &args![array, local]).u32();
        (Ptr::<DismemberedLimb>::new(e.mem.u32(local.addr())), count)
    });
    e.set(entry, DismemberedLimb::cLimb, limb);
    e.set(entry, DismemberedLimb::bLimbExploded, exploded);
    e.set(entry, DismemberedLimb::bObjectArrayIdentical, 0);

    if exploded != 0 {
        e.set(entry, DismemberedLimb::bObjectArrayIdentical, 1);
        return;
    }
    if !e.vcall(actor.addr(), 0x100, &[]).bool() {
        return;
    }
    if actor.addr() == e.global::<u32>(PLAYER) {
        return;
    }
    if !e.vcall(actor.addr(), 0x218, &[]).bool() {
        return;
    }
    let npc = e.call(REFR_NPC_ACCESSOR, &args![actor]).u32();
    let object_array = entry.at(DismemberedLimb::ObjectArray);
    let base = e.vcall(actor.addr(), 0x1e8, &[]).u32();
    e.call(
        TESNPC_BUILD_OBJECT_ARRAY,
        &args![npc, actor, base, object_array],
    );
    if previous_count == 0 {
        return;
    }
    // The newest earlier entry that is not itself marked identical.
    let mut index = previous_count as i32 - 1;
    let earlier: Ptr<DismemberedLimb> = loop {
        if index < 0 {
            return;
        }
        let candidate: Ptr<DismemberedLimb> = array_pointer_at(e, array, index as u32);
        if e.get(candidate, DismemberedLimb::bObjectArrayIdentical) == 0 {
            break candidate;
        }
        index -= 1;
    };
    let earlier_array = earlier.at(DismemberedLimb::ObjectArray);
    if e.call(
        SIMPLE_ARRAY_COMPARE_BUFFER,
        &args![earlier_array, object_array],
    )
    .bool()
    {
        e.set(entry, DismemberedLimb::bObjectArrayIdentical, 1);
        e.call(SIMPLE_ARRAY_CLEAR, &args![object_array, 1u32]);
    }
}

// Translated from 004305f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `bLimbRemoved` to `removed` on every entry whose `cLimb` is `limb`
/// (the Xbox PDB has no name for this method of `ExtraDismemberedLimbs`).
pub fn fn_004305f0(e: &mut Engine, this: Ptr<ExtraDismemberedLimbs>, limb: u32, removed: u8) {
    let array = this.at(ExtraDismemberedLimbs::DismemberedLimbs);
    let mut index = 0;
    while index < array_size(e, array) {
        let entry: Ptr<DismemberedLimb> = array_pointer_at(e, array, index);
        if !entry.is_null() && e.get(entry, DismemberedLimb::cLimb) as u32 == limb {
            e.set(entry, DismemberedLimb::bLimbRemoved, removed);
        }
        index += 1;
    }
}

// Translated from 00430660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDismemberedLimbs::CheckDismemberedLimbsIdentical` (Xbox PDB):
/// whether `other` has the same dismembered limbs, the same number of
/// entries, and entry by entry the same limb, exploded, identical and removed
/// flags and the same object array.
pub fn extra_dismembered_limbs_check_dismembered_limbs_identical(
    e: &mut Engine,
    this: Ptr<ExtraDismemberedLimbs>,
    other: Ptr<ExtraDismemberedLimbs>,
) -> bool {
    if e.get(other, ExtraDismemberedLimbs::sLimbs) != e.get(this, ExtraDismemberedLimbs::sLimbs) {
        return false;
    }
    let other_array = other.at(ExtraDismemberedLimbs::DismemberedLimbs);
    let this_array = this.at(ExtraDismemberedLimbs::DismemberedLimbs);
    if array_size(e, other_array) != array_size(e, this_array) {
        return false;
    }
    let mut index = 0;
    while index < array_size(e, this_array) {
        let mine: Ptr<DismemberedLimb> = array_pointer_at(e, this_array, index);
        let theirs: Ptr<DismemberedLimb> = array_pointer_at(e, other_array, index);
        if e.get(mine, DismemberedLimb::cLimb) != e.get(theirs, DismemberedLimb::cLimb)
            || e.get(mine, DismemberedLimb::bLimbExploded)
                != e.get(theirs, DismemberedLimb::bLimbExploded)
            || e.get(mine, DismemberedLimb::bObjectArrayIdentical)
                != e.get(theirs, DismemberedLimb::bObjectArrayIdentical)
            || e.get(mine, DismemberedLimb::bLimbRemoved)
                != e.get(theirs, DismemberedLimb::bLimbRemoved)
        {
            return false;
        }
        let mine_objects = mine.at(DismemberedLimb::ObjectArray);
        let their_objects = theirs.at(DismemberedLimb::ObjectArray);
        if !e
            .call(
                SIMPLE_ARRAY_COMPARE_BUFFER,
                &args![mine_objects, their_objects],
            )
            .bool()
        {
            return false;
        }
        index += 1;
    }
    true
}

// Translated from 00430780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraStartingPosition`'s constructor from a reference (the engine map has
/// no name for it): extra-data type 0x0f; the position (the three floats the
/// reference's vtable `+0x1f4` function returns the address of) and the
/// rotation (`00430830` of the reference) are copied into `startPosition`.
/// Returns `this`. The exception-unwinding frame is not translated.
pub fn fn_00430780(
    e: &mut Engine,
    this: Ptr<ExtraStartingPosition>,
    reference: Ptr,
) -> Ptr<ExtraStartingPosition> {
    construct_base(
        e,
        this.cast(),
        TYPE_STARTING_POSITION,
        EXTRA_STARTING_POSITION_VTABLE,
    );
    let start = this.at(ExtraStartingPosition::startPosition);
    e.call(FILE_POS_ROT_CONSTRUCT, &args![start]);
    let position = e.vcall(reference.addr(), 0x1f4, &[]).u32();
    let target = start.at(FilePosRot::pos).addr();
    copy_words(e, position, target);
    let rotation = fn_00430830(e, reference);
    let target = start.at(FilePosRot::rot).addr();
    copy_words(e, rotation.addr(), target);
    this
}

/// Copies three 32-bit words (a `NiPoint3`) bit for bit.
fn copy_words(e: &mut Engine, from: u32, to: u32) {
    for word in 0..3 {
        let value = e.mem.u32(from + 4 * word);
        e.mem.set_u32(to + 4 * word, value);
    }
}

// Translated from 00430830 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of the 12-byte vector at `this + 0x24` (an accessor of the
/// reference passed to `00430780`; the engine map attributes it to this unit
/// by address range only).
pub fn fn_00430830(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x24)
}

// Translated from 00430850 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraStartingPosition::Compare` (Xbox PDB): true when `other` is not an
/// `ExtraStartingPosition`, when `BSExtraData::Compare` returns true, or when
/// the 0x18 bytes of `startPosition` differ.
pub fn extra_starting_position_compare(
    e: &mut Engine,
    this: Ptr<ExtraStartingPosition>,
    other: Ptr,
) -> bool {
    let cast: Ptr<ExtraStartingPosition> = e
        .call(
            DYNAMIC_CAST,
            &args![
                other,
                0u32,
                BS_EXTRA_DATA_TYPE,
                EXTRA_STARTING_POSITION_TYPE,
                0u32
            ],
        )
        .ptr();
    if cast.is_null() {
        return true;
    }
    if e.call(BS_EXTRA_DATA_COMPARE, &args![this, other]).bool() {
        return true;
    }
    let mine = this.at(ExtraStartingPosition::startPosition);
    let theirs = cast.at(ExtraStartingPosition::startPosition);
    e.call(MEMCMP, &args![mine, theirs, 0x18u32]).u32() != 0
}

// Translated from 004308c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the `BSExtraData` subclass of type 0x49 (vtable
/// `01015b4c`; no Xbox PDB name is known): the word at +0xc is cleared.
/// Returns `this`.
pub fn fn_004308c0(e: &mut Engine, this: Ptr<ExtraType49>) -> Ptr<ExtraType49> {
    construct_base(e, this.cast(), TYPE_49, EXTRA_TYPE_49_VTABLE);
    e.set(this, ExtraType49::unnamed0C, 0);
    this
}

// Translated from 004308f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraLight`'s constructor (the engine map has no name for it): extra-data
/// type 0x29, taking ownership of `light`. Returns `this`.
pub fn fn_004308f0(e: &mut Engine, this: Ptr<ExtraLight>, light: Ptr) -> Ptr<ExtraLight> {
    construct_base(e, this.cast(), TYPE_LIGHT, EXTRA_LIGHT_VTABLE);
    e.set(this, ExtraLight::pLight, light);
    this
}

// Translated from 00430920 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraLight::_scalar_deleting_destructor_` (Xbox PDB): the destructor,
/// then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_light_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraLight>,
    flags: u32,
) -> Ptr<ExtraLight> {
    fn_00430950(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00430950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraLight`'s destructor body (the engine map has no name for it): resets
/// the vtable, deletes the owned light if there is one, then runs the base
/// destructor. The exception-unwinding frame is not translated.
pub fn fn_00430950(e: &mut Engine, this: Ptr<ExtraLight>) {
    e.mem.set_u32(this.addr(), EXTRA_LIGHT_VTABLE);
    let light = e.get(this, ExtraLight::pLight);
    if !light.is_null() {
        e.call(LIGHT_DELETE, &args![light, 1u32]);
    }
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 004309e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `REFR_LOCK::GetLockLevel` (Xbox PDB): the difficulty bracket (0 very easy
/// to 4 very hard, 5 key only) of the lock's level for `reference`.
pub fn refr_lock_get_lock_level(e: &mut Engine, this: Ptr<RefrLock>, reference: Ptr) -> i32 {
    let level = refr_lock_get_level(e, this, reference);
    refr_lock_numeric_value_to_enum(e, level)
}

// Translated from 00430a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `REFR_LOCK::GetLevel` (Xbox PDB): the lock's level. A leveled lock
/// (`cFlags` bit 2) adds the level of `reference` (the player when it is
/// null) times the float setting at `011c39b4` truncated to an integer, and
/// is capped at 99.
pub fn refr_lock_get_level(e: &mut Engine, this: Ptr<RefrLock>, reference: Ptr) -> i32 {
    let mut level = e.get(this, RefrLock::cBaseLevel) as i32;
    if e.get(this, RefrLock::cFlags) as i32 & 4 != 0 {
        let reference_level = if reference.is_null() {
            let player = e.global::<u32>(PLAYER);
            e.call(ACTOR_GET_LEVEL, &args![player]).u16() as i32
        } else {
            e.call(REFR_GET_CALC_LEVEL, &args![reference, 0u32]).i32()
        };
        let setting = e
            .call(SETTING_FLOAT_VALUE, &args![LEVELED_LOCK_SETTING])
            .u32();
        let scale = e.mem.f32(setting);
        let scale = float_to_int(e, scale);
        level = scale.wrapping_mul(reference_level).wrapping_add(level);
        if level > 99 {
            level = 99;
        }
    }
    level
}

// Translated from 00430a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `REFR_LOCK::SetLocked` (Xbox PDB): sets or clears bit 0 of `cFlags`;
/// unlocking also resets `uiNumTries`.
pub fn refr_lock_set_locked(e: &mut Engine, this: Ptr<RefrLock>, locked: bool) {
    let flags = e.get(this, RefrLock::cFlags);
    if locked {
        e.set(this, RefrLock::cFlags, flags | 1);
    } else {
        e.set(this, RefrLock::cFlags, flags & !1);
        e.set(this, RefrLock::uiNumTries, 0);
    }
}

// Translated from 00430ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `REFR_LOCK::IsBroken` (Xbox PDB): whether `uiNumTries` has reached the
/// number of tries the lock takes: 1 when the entry point 0x20 (asked of the
/// player) leaves its value at 0.0, otherwise 2.
pub fn refr_lock_is_broken(e: &mut Engine, this: Ptr<RefrLock>) -> bool {
    let player = e.global::<u32>(PLAYER);
    let value = e.with_stack(4, |e, result| {
        e.mem.set_f32(result.addr(), 0.0);
        e.call(
            HANDLE_ENTRY_POINT,
            &args![ENTRY_POINT_LOCK_BROKEN, player, result],
        );
        e.mem.f32(result.addr())
    });
    let zero: f64 = e.global(ZERO);
    let tries_allowed: u32 = if value as f64 == zero { 1 } else { 2 };
    e.get(this, RefrLock::uiNumTries) >= tries_allowed
}

// Translated from 00430b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `REFR_LOCK::NumericValueToEnum` (Xbox PDB): the first difficulty bracket
/// whose top setting (`LOCK_LEVEL_SETTINGS`) is at least `value`; 5 when
/// `value` passes every one.
pub fn refr_lock_numeric_value_to_enum(e: &mut Engine, value: i32) -> i32 {
    for (bracket, setting) in LOCK_LEVEL_SETTINGS[..5].iter().enumerate() {
        let top = e.call(SETTING_INT_VALUE, &args![*setting]).u32();
        if value <= e.mem.i32(top) {
            return bracket as i32;
        }
    }
    5
}

// Translated from 00430bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The inverse of `REFR_LOCK::NumericValueToEnum` (no Xbox PDB name): the
/// value of the setting for difficulty `bracket` (0 to 5), 0 for anything
/// else. The switch is the jump table at `00430c50`.
pub fn fn_00430bc0(e: &mut Engine, bracket: u32) -> i32 {
    match LOCK_LEVEL_SETTINGS.get(bracket as usize) {
        Some(setting) => {
            let value = e.call(SETTING_INT_VALUE, &args![*setting]).u32();
            e.mem.i32(value)
        }
        None => 0,
    }
}

// Translated from 00430c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraLock::ExtraLock` (Xbox PDB): extra-data type 0x2a, taking ownership
/// of `lock`. Returns `this`.
pub fn extra_lock_extra_lock(
    e: &mut Engine,
    this: Ptr<ExtraLock>,
    lock: Ptr<RefrLock>,
) -> Ptr<ExtraLock> {
    construct_base(e, this.cast(), TYPE_LOCK, EXTRA_LOCK_VTABLE);
    e.set(this, ExtraLock::pLock, lock);
    this
}

// Translated from 00430ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraLock`'s destructor body (the engine map has no name for it; the
/// scalar deleting destructor is `0042c550`, in `extradatalist.cpp`): resets
/// the vtable, frees the owned lock (without a null check), then runs the base
/// destructor.
pub fn fn_00430ca0(e: &mut Engine, this: Ptr<ExtraLock>) {
    e.mem.set_u32(this.addr(), EXTRA_LOCK_VTABLE);
    let lock = e.get(this, ExtraLock::pLock);
    e.call(OPERATOR_DELETE, &args![lock]);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 00430ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraLock::Compare` (Xbox PDB): true when `other` is not an `ExtraLock`,
/// when `BSExtraData::Compare` returns true, or when the two locks differ
/// in `cFlags`, `pKey`, difficulty bracket of `cBaseLevel`, `uiNumTries` or
/// `uiTimesUnlocked`.
pub fn extra_lock_compare(e: &mut Engine, this: Ptr<ExtraLock>, other: Ptr) -> bool {
    let cast: Ptr<ExtraLock> = e
        .call(
            DYNAMIC_CAST,
            &args![other, 0u32, BS_EXTRA_DATA_TYPE, EXTRA_LOCK_TYPE, 0u32],
        )
        .ptr();
    if cast.is_null() {
        return true;
    }
    if e.call(BS_EXTRA_DATA_COMPARE, &args![this, other]).bool() {
        return true;
    }
    let mine = e.get(this, ExtraLock::pLock);
    let theirs = e.get(cast, ExtraLock::pLock);
    if e.get(mine, RefrLock::cFlags) != e.get(theirs, RefrLock::cFlags) {
        return true;
    }
    if e.get(mine, RefrLock::pKey) != e.get(theirs, RefrLock::pKey) {
        return true;
    }
    let my_bracket = refr_lock_numeric_value_to_enum(e, e.get(mine, RefrLock::cBaseLevel) as i32);
    let their_bracket =
        refr_lock_numeric_value_to_enum(e, e.get(theirs, RefrLock::cBaseLevel) as i32);
    if my_bracket != their_bracket {
        return true;
    }
    if e.get(mine, RefrLock::uiNumTries) != e.get(theirs, RefrLock::uiNumTries) {
        return true;
    }
    e.get(mine, RefrLock::uiTimesUnlocked) != e.get(theirs, RefrLock::uiTimesUnlocked)
}

// Translated from 00430dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFollower::ExtraFollower` (Xbox PDB): extra-data type 0x1d and a
/// newly allocated, constructed 8-byte actor list (null when the allocation
/// fails). Returns `this`. The exception-unwinding frame is not translated.
pub fn extra_follower_extra_follower(
    e: &mut Engine,
    this: Ptr<ExtraFollower>,
) -> Ptr<ExtraFollower> {
    construct_base(e, this.cast(), TYPE_FOLLOWER, EXTRA_FOLLOWER_VTABLE);
    let memory = e.call(OPERATOR_NEW, &args![8u32]).u32();
    let list: Ptr = if memory != 0 {
        e.call(ACTOR_LIST_CONSTRUCT, &args![memory]).ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, ExtraFollower::pActorlist, list);
    this
}

// Translated from 00430e70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFollower::_scalar_deleting_destructor_` (Xbox PDB): the destructor,
/// then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_follower_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraFollower>,
    flags: u32,
) -> Ptr<ExtraFollower> {
    fn_00430ea0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00430ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFollower`'s destructor body (the engine map has no name for it):
/// resets the vtable, clears the actor list (called even when the pointer is
/// null), deletes it if there is one, then runs the base destructor. The
/// exception-unwinding frame is not translated.
pub fn fn_00430ea0(e: &mut Engine, this: Ptr<ExtraFollower>) {
    e.mem.set_u32(this.addr(), EXTRA_FOLLOWER_VTABLE);
    let list = e.get(this, ExtraFollower::pActorlist);
    e.call(ACTOR_LIST_CLEAR, &args![list]);
    let list = e.get(this, ExtraFollower::pActorlist);
    if !list.is_null() {
        e.call(ACTOR_LIST_DELETE, &args![list, 1u32]);
    }
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 00430f30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraGuardedRefData::ExtraGuardedRefData` (Xbox PDB): extra-data type
/// 0x7c and an empty `Guards` array. Returns `this`. The exception-unwinding
/// frame is not translated.
pub fn extra_guarded_ref_data_extra_guarded_ref_data(
    e: &mut Engine,
    this: Ptr<ExtraGuardedRefData>,
) -> Ptr<ExtraGuardedRefData> {
    construct_base(
        e,
        this.cast(),
        TYPE_GUARDED_REF_DATA,
        EXTRA_GUARDED_REF_DATA_VTABLE,
    );
    let guards = this.at(ExtraGuardedRefData::Guards);
    e.call(GUARDS_ARRAY_CONSTRUCT, &args![guards]);
    this
}

// Translated from 00430fa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraGuardedRefData::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_guarded_ref_data_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraGuardedRefData>,
    flags: u32,
) -> Ptr<ExtraGuardedRefData> {
    fn_00430fd0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00430fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraGuardedRefData`'s destructor body (the engine map has no name for
/// it): resets the vtable, destroys the `Guards` array, then runs the base
/// destructor. The exception-unwinding frame is not translated.
pub fn fn_00430fd0(e: &mut Engine, this: Ptr<ExtraGuardedRefData>) {
    e.mem.set_u32(this.addr(), EXTRA_GUARDED_REF_DATA_VTABLE);
    let guards = this.at(ExtraGuardedRefData::Guards);
    e.call(GUARDS_ARRAY_DESTRUCT, &args![guards]);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 00431030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraGuardedRefData::Compare` (Xbox PDB): true when
/// `BSExtraData::Compare` returns false (the opposite test to the other
/// `Compare` methods of this unit, as the code has it), when the guard counts
/// differ, or when some guard of `this` is not in `other`'s array. `other` is
/// used as an `ExtraGuardedRefData` without a type check.
pub fn extra_guarded_ref_data_compare(
    e: &mut Engine,
    this: Ptr<ExtraGuardedRefData>,
    other: Ptr<ExtraGuardedRefData>,
) -> bool {
    if !e.call(BS_EXTRA_DATA_COMPARE, &args![this, other]).bool() {
        return true;
    }
    let their_guards = other.at(ExtraGuardedRefData::Guards);
    let my_guards = this.at(ExtraGuardedRefData::Guards);
    if array_size(e, their_guards) != array_size(e, my_guards) {
        return true;
    }
    let mut index = 0;
    while index < array_size(e, my_guards) {
        let guard = array_slot(e, my_guards, index);
        let found = e
            .call(
                SIMPLE_ARRAY_FIND,
                &args![their_guards, guard, 0u32, GUARD_COMPARE],
            )
            .u32();
        if found == 0xffff_ffff {
            return true;
        }
        index += 1;
    }
    false
}

// Translated from 004310d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraGuardedRefData::AddGuard` (Xbox PDB): appends the value
/// `0084e3a0` gives for `reference` to `Guards` unless it is already there.
pub fn extra_guarded_ref_data_add_guard(
    e: &mut Engine,
    this: Ptr<ExtraGuardedRefData>,
    reference: Ptr,
) {
    let guard = e.call(REFR_GUARD_VALUE, &args![reference]).u32();
    let guards = this.at(ExtraGuardedRefData::Guards);
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), guard);
        let found = e
            .call(
                SIMPLE_ARRAY_FIND,
                &args![guards, local, 0u32, GUARD_COMPARE],
            )
            .u32();
        if found == 0xffff_ffff {
            e.call(SIMPLE_ARRAY_ADD, &args![guards, local]);
        }
    });
}

// Translated from 00431120 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every guard in `Guards` (the engine map has no name for this method of
/// `ExtraGuardedRefData`): looks the form up, and when it is a form whose
/// vtable `+0xf0` and `+0x100` functions are both true and `008d8520` gives it
/// a process, calls that process's vtable `+0x60c` function with the form,
/// `second` and `first`.
pub fn fn_00431120(e: &mut Engine, this: Ptr<ExtraGuardedRefData>, first: u32, second: u32) {
    let guards = this.at(ExtraGuardedRefData::Guards);
    let mut index = 0;
    while index < array_size(e, guards) {
        let id = array_pointer_at::<()>(e, guards, index).addr();
        let form = e.call(LOOKUP_FORM_BY_ID, &args![id]).u32();
        if form != 0 && e.vcall(form, 0xf0, &[]).bool() && e.vcall(form, 0x100, &[]).bool() {
            let process = e.call(FORM_PROCESS, &args![form]).u32();
            if process != 0 {
                e.vcall(process, 0x60c, &args![form, second, first]);
            }
        }
        index += 1;
    }
}

// Translated from 004311f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the `BSExtraData` subclass of type 0x2e (vtable
/// `01015b7c`; no Xbox PDB name is known): the two words at +0xc and +0x10
/// are cleared. Returns `this`.
pub fn fn_004311f0(e: &mut Engine, this: Ptr<ExtraType2e>) -> Ptr<ExtraType2e> {
    construct_base(e, this.cast(), TYPE_2E, EXTRA_TYPE_2E_VTABLE);
    e.set(this, ExtraType2e::unnamed0C, 0);
    e.set(this, ExtraType2e::unnamed10, 0);
    this
}

// Translated from 00431230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraTeleport`'s constructor (the engine map has no name for it):
/// extra-data type 0x2b, taking ownership of `data`. Returns `this`.
pub fn fn_00431230(e: &mut Engine, this: Ptr<ExtraTeleport>, data: Ptr) -> Ptr<ExtraTeleport> {
    construct_base(e, this.cast(), TYPE_TELEPORT, EXTRA_TELEPORT_VTABLE);
    e.set(this, ExtraTeleport::pData, data);
    this
}

// Translated from 00431260 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraTeleport`'s destructor body (the engine map has no name for it; the
/// scalar deleting destructor is `0042c5b0`, in `extradatalist.cpp`): resets
/// the vtable, deletes the owned teleport data if there is any, then runs the
/// base destructor. The exception-unwinding frame is not translated.
pub fn fn_00431260(e: &mut Engine, this: Ptr<ExtraTeleport>) {
    e.mem.set_u32(this.addr(), EXTRA_TELEPORT_VTABLE);
    let data = e.get(this, ExtraTeleport::pData);
    if !data.is_null() {
        e.call(TELEPORT_DATA_DELETE, &args![data, 1u32]);
    }
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004300f0,
            extra_anim_scalar_deleting_destructor(Ptr<ExtraAnim>, u32) -> Ptr<ExtraAnim>
        ),
        entry!(0x00430120, fn_00430120(Ptr<ExtraAnim>)),
        entry!(0x004301b0, fn_004301b0(Ptr, i32) -> u16),
        entry!(
            0x00430200,
            extra_dismembered_limbs_extra_dismembered_limbs(
                Ptr<ExtraDismemberedLimbs>,
            )
                -> Ptr<ExtraDismemberedLimbs>
        ),
        entry!(
            0x004302a0,
            extra_dismembered_limbs_scalar_deleting_destructor(
                Ptr<ExtraDismemberedLimbs>,
                u32,
            )
                -> Ptr<ExtraDismemberedLimbs>
        ),
        entry!(0x004302d0, fn_004302d0(Ptr<ExtraDismemberedLimbs>)),
        entry!(
            0x00430390,
            fn_00430390(Ptr<DismemberedLimb>, u32) -> Ptr<DismemberedLimb>
        ),
        entry!(0x004303c0, fn_004303c0(Ptr<DismemberedLimb>)),
        entry!(
            0x004303e0,
            extra_dismembered_limbs_dismembered(Ptr<ExtraDismemberedLimbs>, u8) -> bool
        ),
        entry!(
            0x00430410,
            extra_dismembered_limbs_dismember(Ptr<ExtraDismemberedLimbs>, Ptr, u8, u8)
        ),
        entry!(0x004305f0, fn_004305f0(Ptr<ExtraDismemberedLimbs>, u32, u8)),
        entry!(
            0x00430660,
            extra_dismembered_limbs_check_dismembered_limbs_identical(
                Ptr<ExtraDismemberedLimbs>,
                Ptr<ExtraDismemberedLimbs>,
            ) -> bool
        ),
        entry!(
            0x00430780,
            fn_00430780(Ptr<ExtraStartingPosition>, Ptr) -> Ptr<ExtraStartingPosition>
        ),
        entry!(0x00430830, fn_00430830(Ptr) -> Ptr),
        entry!(
            0x00430850,
            extra_starting_position_compare(Ptr<ExtraStartingPosition>, Ptr) -> bool
        ),
        entry!(
            0x004308c0,
            fn_004308c0(Ptr<ExtraType49>) -> Ptr<ExtraType49>
        ),
        entry!(
            0x004308f0,
            fn_004308f0(Ptr<ExtraLight>, Ptr) -> Ptr<ExtraLight>
        ),
        entry!(
            0x00430920,
            extra_light_scalar_deleting_destructor(Ptr<ExtraLight>, u32) -> Ptr<ExtraLight>
        ),
        entry!(0x00430950, fn_00430950(Ptr<ExtraLight>)),
        entry!(
            0x004309e0,
            refr_lock_get_lock_level(Ptr<RefrLock>, Ptr) -> i32
        ),
        entry!(0x00430a10, refr_lock_get_level(Ptr<RefrLock>, Ptr) -> i32),
        entry!(0x00430a90, refr_lock_set_locked(Ptr<RefrLock>, bool)),
        entry!(0x00430ae0, refr_lock_is_broken(Ptr<RefrLock>) -> bool),
        entry!(0x00430b40, refr_lock_numeric_value_to_enum(i32) -> i32),
        entry!(0x00430bc0, fn_00430bc0(u32) -> i32),
        entry!(
            0x00430c70,
            extra_lock_extra_lock(Ptr<ExtraLock>, Ptr<RefrLock>) -> Ptr<ExtraLock>
        ),
        entry!(0x00430ca0, fn_00430ca0(Ptr<ExtraLock>)),
        entry!(0x00430ce0, extra_lock_compare(Ptr<ExtraLock>, Ptr) -> bool),
        entry!(
            0x00430dd0,
            extra_follower_extra_follower(Ptr<ExtraFollower>) -> Ptr<ExtraFollower>
        ),
        entry!(
            0x00430e70,
            extra_follower_scalar_deleting_destructor(
                Ptr<ExtraFollower>,
                u32,
            ) -> Ptr<ExtraFollower>
        ),
        entry!(0x00430ea0, fn_00430ea0(Ptr<ExtraFollower>)),
        entry!(
            0x00430f30,
            extra_guarded_ref_data_extra_guarded_ref_data(
                Ptr<ExtraGuardedRefData>,
            ) -> Ptr<ExtraGuardedRefData>
        ),
        entry!(
            0x00430fa0,
            extra_guarded_ref_data_scalar_deleting_destructor(
                Ptr<ExtraGuardedRefData>,
                u32,
            )
                -> Ptr<ExtraGuardedRefData>
        ),
        entry!(0x00430fd0, fn_00430fd0(Ptr<ExtraGuardedRefData>)),
        entry!(
            0x00431030,
            extra_guarded_ref_data_compare(
                Ptr<ExtraGuardedRefData>,
                Ptr<ExtraGuardedRefData>,
            ) -> bool
        ),
        entry!(
            0x004310d0,
            extra_guarded_ref_data_add_guard(Ptr<ExtraGuardedRefData>, Ptr)
        ),
        entry!(0x00431120, fn_00431120(Ptr<ExtraGuardedRefData>, u32, u32)),
        entry!(
            0x004311f0,
            fn_004311f0(Ptr<ExtraType2e>) -> Ptr<ExtraType2e>
        ),
        entry!(
            0x00431230,
            fn_00431230(Ptr<ExtraTeleport>, Ptr) -> Ptr<ExtraTeleport>
        ),
        entry!(0x00431260, fn_00431260(Ptr<ExtraTeleport>)),
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

    /// Test doubles for the callees every function here shares (allocator,
    /// the base `BSExtraData` constructor and destructor, `BSSimpleArray`'s
    /// accessors and append, `_ftol2_sse`) and the pages holding the exe
    /// globals the code reads.
    fn extra_engine() -> Engine {
        let mut e = Engine::new();
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(OPERATOR_DELETE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        // The base constructor stores the type byte at +4 (`cEtype`).
        e.register(BS_EXTRA_DATA_CONSTRUCT, |e, a| {
            e.mem.set_u8(a[0] + 4, a[1] as u8);
            ret(a[0])
        });
        e.register(BS_EXTRA_DATA_DESTRUCT, |_, _| Ret::default());
        // `BSSimpleArray`: buffer at +4, count at +8.
        e.register(SIMPLE_ARRAY_SIZE, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(SIMPLE_ARRAY_AT, |e, a| ret(e.mem.u32(a[0] + 4) + 4 * a[1]));
        e.register(SIMPLE_ARRAY_ADD, |e, a| {
            let old_count = e.mem.u32(a[0] + 8);
            let old_buffer = e.mem.u32(a[0] + 4);
            let buffer = e.mem.alloc(4 * (old_count + 1));
            for i in 0..old_count {
                let item = e.mem.u32(old_buffer + 4 * i);
                e.mem.set_u32(buffer + 4 * i, item);
            }
            let item = e.mem.u32(a[1]);
            e.mem.set_u32(buffer + 4 * old_count, item);
            e.mem.set_u32(a[0] + 4, buffer);
            e.mem.set_u32(a[0] + 8, old_count + 1);
            ret(old_count)
        });
        e.register(FTOL, |_, a| ret(f64::take(a, &mut 0) as i32 as u32));
        for page in [0x0101_2000, 0x011d_e000] {
            e.map(page, 0x1000);
        }
        e.set_global(ZERO, 0.0f64);
        e
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The argument words of every logged call to `addr`.
    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(called, _)| *called == addr)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// Fills the `BSSimpleArray` at `array` with `items`.
    fn make_array(e: &mut Engine, array: u32, items: &[u32]) {
        let buffer = e.mem.alloc(4 * items.len().max(1) as u32);
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(buffer + 4 * i as u32, *item);
        }
        e.mem.set_u32(array + 4, buffer);
        e.mem.set_u32(array + 8, items.len() as u32);
        e.mem.set_u32(array + 0xc, items.len().max(1) as u32);
    }

    fn array_items(e: &Engine, array: u32) -> Vec<u32> {
        let buffer = e.mem.u32(array + 4);
        (0..e.mem.u32(array + 8))
            .map(|i| e.mem.u32(buffer + 4 * i))
            .collect()
    }

    /// An object whose vtable (on the heap) has the given `(byte offset,
    /// function)` slots; the functions are registered by the test.
    fn object_with_vtable(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x700);
        for (offset, target) in slots {
            e.mem.set_u32(vtable + offset, *target);
        }
        let object = e.mem.alloc(0x40);
        e.mem.set_u32(object, vtable);
        object
    }

    fn extra_type(e: &Engine, extra: u32) -> u8 {
        e.mem.u8(extra + 4)
    }

    fn vtable_of(e: &Engine, extra: u32) -> u32 {
        e.mem.u32(extra)
    }

    #[test]
    fn anim_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = extra_engine();
        e.register(ANIMATION_DELETE, |_, _| Ret::default());
        let this: Ptr<ExtraAnim> = e.new_object();
        assert_eq!(
            e.call(0x0043_00f0, &args![this, 0u32]).ptr::<ExtraAnim>(),
            this
        );
        assert!(e.mem.block_size(this.addr()).is_some());
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_ANIM_VTABLE);
        assert_eq!(
            e.call(0x0043_00f0, &args![this, 3u32]).ptr::<ExtraAnim>(),
            this
        );
        assert_eq!(e.mem.block_size(this.addr()), None);
    }

    #[test]
    fn anim_destructor_deletes_the_animation_only_when_there_is_one() {
        let mut e = extra_engine();
        e.register(ANIMATION_DELETE, |_, _| Ret::default());
        let this: Ptr<ExtraAnim> = e.new_object();
        let animation = e.mem.alloc(8);
        e.set(this, ExtraAnim::pAnimation, Ptr::new(animation));
        start_log(&mut e);
        e.call(0x0043_0120, &args![this]);
        assert_eq!(calls(&e, ANIMATION_DELETE), vec![vec![animation, 1]]);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this.addr()]]);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_ANIM_VTABLE);

        let bare: Ptr<ExtraAnim> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_0120, &args![bare]);
        assert!(calls(&e, ANIMATION_DELETE).is_empty());
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT).len(), 1);
    }

    #[test]
    fn slot_table_entry_maps_the_whole_and_upper_body_sections() {
        let mut e = extra_engine();
        let this = e.mem.alloc(0x80);
        for slot in 0..10u32 {
            e.mem.set_u16(this + 0x4c + 2 * slot, 100 + slot as u16);
        }
        let read = |e: &mut Engine, section: i32| e.call(0x0043_01b0, &args![this, section]).u16();
        assert_eq!(read(&mut e, 0x14), 101);
        assert_eq!(read(&mut e, 0x15), 104);
        assert_eq!(read(&mut e, 0), 100);
        assert_eq!(read(&mut e, 7), 107);
    }

    #[test]
    fn dismembered_limbs_constructor_sets_the_empty_state() {
        let mut e = extra_engine();
        e.register(DISMEMBERED_LIMBS_ARRAY_CONSTRUCT, |_, _| Ret::default());
        let this: Ptr<ExtraDismemberedLimbs> = e.new_object();
        for offset in (0x0c..0x20).step_by(4) {
            e.mem.set_u32(this.addr() + offset, 0xaaaa_aaaa);
        }
        start_log(&mut e);
        let back = e
            .call(0x0043_0200, &args![this])
            .ptr::<ExtraDismemberedLimbs>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x5f);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_DISMEMBERED_LIMBS_VTABLE);
        assert_eq!(
            calls(&e, DISMEMBERED_LIMBS_ARRAY_CONSTRUCT),
            vec![vec![this.addr() + 0x20]]
        );
        assert_eq!(e.get(this, ExtraDismemberedLimbs::sLimbs), 0);
        assert_eq!(
            e.get(this, ExtraDismemberedLimbs::eCauseofDeath),
            0xffff_ffff
        );
        assert!(e.get(this, ExtraDismemberedLimbs::pDeathObject).is_null());
        assert_eq!(
            e.get(this, ExtraDismemberedLimbs::eLastHitLimb),
            0xffff_ffff
        );
        assert!(!e.get(this, ExtraDismemberedLimbs::bEaten));
    }

    #[test]
    fn dismembered_limbs_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = extra_engine();
        e.register(DISMEMBERED_LIMBS_ARRAY_DESTRUCT, |_, _| Ret::default());
        let this: Ptr<ExtraDismemberedLimbs> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_02a0, &args![this, 0u32]);
        assert!(e.mem.block_size(this.addr()).is_some());
        let back = e
            .call(0x0043_02a0, &args![this, 1u32])
            .ptr::<ExtraDismemberedLimbs>();
        assert_eq!(back, this);
        assert_eq!(e.mem.block_size(this.addr()), None);
        assert_eq!(calls(&e, DISMEMBERED_LIMBS_ARRAY_DESTRUCT).len(), 2);
    }

    #[test]
    fn dismembered_limbs_destructor_deletes_each_entry_and_the_array() {
        let mut e = extra_engine();
        e.register(DISMEMBERED_LIMBS_ARRAY_DESTRUCT, |_, _| Ret::default());
        e.register(DISMEMBERED_LIMB_ARRAY_DESTRUCT, |_, _| Ret::default());
        let this: Ptr<ExtraDismemberedLimbs> = e.new_object();
        let first = e.mem.alloc(0x14);
        let last = e.mem.alloc(0x14);
        make_array(&mut e, this.addr() + 0x20, &[first, 0, last]);
        start_log(&mut e);
        e.call(0x0043_02d0, &args![this]);
        assert_eq!(e.mem.block_size(first), None);
        assert_eq!(e.mem.block_size(last), None);
        assert_eq!(
            calls(&e, DISMEMBERED_LIMB_ARRAY_DESTRUCT),
            vec![vec![first + 4], vec![last + 4]]
        );
        assert_eq!(
            calls(&e, DISMEMBERED_LIMBS_ARRAY_DESTRUCT),
            vec![vec![this.addr() + 0x20]]
        );
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this.addr()]]);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_DISMEMBERED_LIMBS_VTABLE);
    }

    #[test]
    fn dismembered_limb_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = extra_engine();
        e.register(DISMEMBERED_LIMB_ARRAY_DESTRUCT, |_, _| Ret::default());
        let this: Ptr<DismemberedLimb> = e.new_object();
        start_log(&mut e);
        assert_eq!(
            e.call(0x0043_0390, &args![this, 0u32])
                .ptr::<DismemberedLimb>(),
            this
        );
        assert!(e.mem.block_size(this.addr()).is_some());
        e.call(0x0043_0390, &args![this, 1u32]);
        assert_eq!(e.mem.block_size(this.addr()), None);
        assert_eq!(calls(&e, DISMEMBERED_LIMB_ARRAY_DESTRUCT).len(), 2);
    }

    #[test]
    fn dismembered_limb_destructor_destroys_the_object_array() {
        let mut e = extra_engine();
        e.register(DISMEMBERED_LIMB_ARRAY_DESTRUCT, |_, _| Ret::default());
        let this: Ptr<DismemberedLimb> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_03c0, &args![this]);
        assert_eq!(
            calls(&e, DISMEMBERED_LIMB_ARRAY_DESTRUCT),
            vec![vec![this.addr() + 4]]
        );
    }

    #[test]
    fn dismembered_tests_the_limb_bit() {
        let mut e = extra_engine();
        let this: Ptr<ExtraDismemberedLimbs> = e.new_object();
        e.set(this, ExtraDismemberedLimbs::sLimbs, 0b101);
        let test = |e: &mut Engine, limb: u8| e.call(0x0043_03e0, &args![this, limb]).bool();
        assert!(test(&mut e, 0));
        assert!(!test(&mut e, 1));
        assert!(test(&mut e, 2));
        // The shift count wraps at 32, as `SHL` does.
        assert!(test(&mut e, 32));
        assert!(!test(&mut e, 33));
    }

    /// Everything `Dismember` calls, with the actor's vtable answering
    /// `+0x100` with `is_actor` and `+0x218` with `flag`; `+0x1e8` gives
    /// `0x7000_0001`. `identical` is what the array comparison answers.
    fn dismember_setup(is_actor: bool, flag: bool, identical: bool) -> (Engine, u32, u32) {
        let mut e = extra_engine();
        e.register(DISMEMBERED_LIMB_CONSTRUCT, |_, a| ret(a[0]));
        e.register(REFR_NPC_ACCESSOR, |_, _| ret(0x4e50));
        e.register(TESNPC_BUILD_OBJECT_ARRAY, |_, _| Ret::default());
        e.register(SIMPLE_ARRAY_CLEAR, |_, _| Ret::default());
        e.register_double(SIMPLE_ARRAY_COMPARE_BUFFER, move |_, _| {
            ret(identical as u32)
        });
        e.register_double(0x00a0_0100, move |_, _| ret(is_actor as u32));
        e.register_double(0x00a0_0218, move |_, _| ret(flag as u32));
        e.register(0x00a0_01e8, |_, _| ret(0x7000_0001));
        let actor = object_with_vtable(
            &mut e,
            &[
                (0x100, 0x00a0_0100),
                (0x218, 0x00a0_0218),
                (0x1e8, 0x00a0_01e8),
            ],
        );
        let this = e.mem.alloc(0x30);
        (e, this, actor)
    }

    /// A `DismemberedLimb` entry with the given limb and identical flag.
    fn limb_entry(e: &mut Engine, limb: u8, identical: u8) -> u32 {
        let entry = e.mem.alloc(0x14);
        e.mem.set_u8(entry, limb);
        e.mem.set_u8(entry + 2, identical);
        entry
    }

    fn dismember(e: &mut Engine, this: u32, actor: u32, limb: u8, exploded: u8) {
        e.call(
            0x0043_0410,
            &args![Ptr::<()>::new(this), Ptr::<()>::new(actor), limb, exploded],
        );
    }

    fn entry_flags(e: &Engine, entry: u32) -> [u8; 4] {
        [
            e.mem.u8(entry),
            e.mem.u8(entry + 1),
            e.mem.u8(entry + 2),
            e.mem.u8(entry + 3),
        ]
    }

    #[test]
    fn dismember_ignores_a_limb_that_is_already_dismembered() {
        let (mut e, this, actor) = dismember_setup(true, true, false);
        e.mem.set_u16(this + 0xc, 0b100);
        start_log(&mut e);
        dismember(&mut e, this, actor, 2, 0);
        assert!(calls(&e, OPERATOR_NEW).is_empty());
        assert_eq!(e.mem.u32(this + 0x28), 0);
        assert_eq!(e.mem.u16(this + 0xc), 0b100);
    }

    #[test]
    fn dismember_an_exploded_limb_is_marked_identical_at_once() {
        let (mut e, this, actor) = dismember_setup(true, true, false);
        start_log(&mut e);
        dismember(&mut e, this, actor, 5, 1);
        assert_eq!(e.mem.u16(this + 0xc), 1 << 5);
        let entries = array_items(&e, this + 0x20);
        assert_eq!(entries.len(), 1);
        let entry = entries[0];
        assert_eq!(entry_flags(&e, entry), [5, 1, 1, 0]);
        assert!(calls(&e, TESNPC_BUILD_OBJECT_ARRAY).is_empty());
        assert_eq!(calls(&e, DISMEMBERED_LIMB_CONSTRUCT), vec![vec![entry]]);
        assert_eq!(calls(&e, OPERATOR_NEW), vec![vec![0x14]]);
    }

    #[test]
    fn dismember_builds_nothing_for_a_non_actor_the_player_or_a_flagged_actor() {
        // Vtable +0x100 false.
        let (mut e, this, actor) = dismember_setup(false, true, true);
        start_log(&mut e);
        dismember(&mut e, this, actor, 1, 0);
        let entry = array_items(&e, this + 0x20)[0];
        assert_eq!(e.mem.u8(entry + 2), 0);
        assert!(calls(&e, TESNPC_BUILD_OBJECT_ARRAY).is_empty());

        // The player.
        let (mut e, this, actor) = dismember_setup(true, true, true);
        e.set_global(PLAYER, actor);
        start_log(&mut e);
        dismember(&mut e, this, actor, 1, 0);
        assert!(calls(&e, TESNPC_BUILD_OBJECT_ARRAY).is_empty());

        // Vtable +0x218 false.
        let (mut e, this, actor) = dismember_setup(true, false, true);
        start_log(&mut e);
        dismember(&mut e, this, actor, 1, 0);
        assert!(calls(&e, TESNPC_BUILD_OBJECT_ARRAY).is_empty());
        assert_eq!(array_items(&e, this + 0x20).len(), 1);
    }

    #[test]
    fn dismember_marks_a_duplicate_object_array_identical_and_clears_it() {
        let (mut e, this, actor) = dismember_setup(true, true, true);
        let older = limb_entry(&mut e, 1, 0);
        let newest_identical = limb_entry(&mut e, 2, 1);
        make_array(&mut e, this + 0x20, &[older, newest_identical]);
        start_log(&mut e);
        dismember(&mut e, this, actor, 5, 0);
        let entries = array_items(&e, this + 0x20);
        assert_eq!(entries.len(), 3);
        let entry = entries[2];
        assert_eq!(entry_flags(&e, entry), [5, 0, 1, 0]);
        assert_eq!(e.mem.u16(this + 0xc), 1 << 5);
        assert_eq!(
            calls(&e, TESNPC_BUILD_OBJECT_ARRAY),
            vec![vec![0x4e50, actor, 0x7000_0001, entry + 4]]
        );
        // The entry marked identical is skipped; the older one is compared.
        assert_eq!(
            calls(&e, SIMPLE_ARRAY_COMPARE_BUFFER),
            vec![vec![older + 4, entry + 4]]
        );
        assert_eq!(calls(&e, SIMPLE_ARRAY_CLEAR), vec![vec![entry + 4, 1]]);
    }

    #[test]
    fn dismember_keeps_a_different_object_array_and_stops_without_earlier_entries() {
        let (mut e, this, actor) = dismember_setup(true, true, false);
        let older = limb_entry(&mut e, 1, 0);
        make_array(&mut e, this + 0x20, &[older]);
        start_log(&mut e);
        dismember(&mut e, this, actor, 5, 0);
        let entry = array_items(&e, this + 0x20)[1];
        assert_eq!(e.mem.u8(entry + 2), 0);
        assert_eq!(calls(&e, SIMPLE_ARRAY_COMPARE_BUFFER).len(), 1);
        assert!(calls(&e, SIMPLE_ARRAY_CLEAR).is_empty());

        // Only an earlier entry that is itself identical: nothing to compare.
        let (mut e, this, actor) = dismember_setup(true, true, true);
        let skipped = limb_entry(&mut e, 1, 1);
        make_array(&mut e, this + 0x20, &[skipped]);
        start_log(&mut e);
        dismember(&mut e, this, actor, 5, 0);
        assert!(calls(&e, SIMPLE_ARRAY_COMPARE_BUFFER).is_empty());

        // No earlier entry at all.
        let (mut e, this, actor) = dismember_setup(true, true, true);
        start_log(&mut e);
        dismember(&mut e, this, actor, 5, 0);
        assert!(calls(&e, SIMPLE_ARRAY_COMPARE_BUFFER).is_empty());
        assert_eq!(calls(&e, TESNPC_BUILD_OBJECT_ARRAY).len(), 1);
    }

    #[test]
    fn set_removed_updates_every_entry_of_the_limb() {
        let mut e = extra_engine();
        let this: Ptr<ExtraDismemberedLimbs> = e.new_object();
        let first = limb_entry(&mut e, 3, 0);
        let other = limb_entry(&mut e, 4, 0);
        let last = limb_entry(&mut e, 3, 0);
        make_array(&mut e, this.addr() + 0x20, &[first, 0, other, last]);
        e.call(0x0043_05f0, &args![this, 3u32, 1u8]);
        assert_eq!(e.mem.u8(first + 3), 1);
        assert_eq!(e.mem.u8(other + 3), 0);
        assert_eq!(e.mem.u8(last + 3), 1);
        e.call(0x0043_05f0, &args![this, 3u32, 0u8]);
        assert_eq!(e.mem.u8(first + 3), 0);
        assert_eq!(e.mem.u8(last + 3), 0);
    }

    /// An `ExtraDismemberedLimbs` with the given limb mask and entries
    /// `(limb, exploded, identical, removed)`.
    fn limbs_object(e: &mut Engine, mask: u16, entries: &[(u8, u8, u8, u8)]) -> u32 {
        let this = e.mem.alloc(0x30);
        e.mem.set_u16(this + 0xc, mask);
        let mut pointers = vec![];
        for (limb, exploded, identical, removed) in entries {
            let entry = limb_entry(e, *limb, *identical);
            e.mem.set_u8(entry + 1, *exploded);
            e.mem.set_u8(entry + 3, *removed);
            pointers.push(entry);
        }
        make_array(e, this + 0x20, &pointers);
        this
    }

    fn limbs_identical(e: &mut Engine, this: u32, other: u32) -> bool {
        e.call(
            0x0043_0660,
            &args![Ptr::<()>::new(this), Ptr::<()>::new(other)],
        )
        .bool()
    }

    #[test]
    fn check_dismembered_limbs_identical_compares_every_field() {
        let mut e = extra_engine();
        e.register(SIMPLE_ARRAY_COMPARE_BUFFER, |_, _| ret(1));
        let same = [(1, 0, 0, 0), (2, 1, 1, 0)];
        let this = limbs_object(&mut e, 0b110, &same);
        let twin = limbs_object(&mut e, 0b110, &same);
        start_log(&mut e);
        assert!(limbs_identical(&mut e, this, twin));
        let this_first = e.mem.u32(e.mem.u32(this + 0x24));
        let twin_first = e.mem.u32(e.mem.u32(twin + 0x24));
        assert_eq!(
            calls(&e, SIMPLE_ARRAY_COMPARE_BUFFER)[0],
            vec![this_first + 4, twin_first + 4]
        );

        let other_mask = limbs_object(&mut e, 0b111, &same);
        assert!(!limbs_identical(&mut e, this, other_mask));
        let shorter = limbs_object(&mut e, 0b110, &same[..1]);
        assert!(!limbs_identical(&mut e, this, shorter));
        for field in 0..4 {
            let mut changed = same;
            match field {
                0 => changed[1].0 = 9,
                1 => changed[1].1 = 0,
                2 => changed[1].2 = 0,
                _ => changed[1].3 = 1,
            }
            let other = limbs_object(&mut e, 0b110, &changed);
            assert!(!limbs_identical(&mut e, this, other), "field {field}");
        }
    }

    #[test]
    fn check_dismembered_limbs_identical_fails_on_different_object_arrays() {
        let mut e = extra_engine();
        e.register(SIMPLE_ARRAY_COMPARE_BUFFER, |_, _| ret(0));
        let this = limbs_object(&mut e, 1, &[(0, 0, 0, 0)]);
        let other = limbs_object(&mut e, 1, &[(0, 0, 0, 0)]);
        assert!(!limbs_identical(&mut e, this, other));
        // Two empty lists are identical without comparing anything.
        let empty = limbs_object(&mut e, 0, &[]);
        let also_empty = limbs_object(&mut e, 0, &[]);
        assert!(limbs_identical(&mut e, empty, also_empty));
    }

    #[test]
    fn starting_position_constructor_copies_position_and_rotation() {
        let mut e = extra_engine();
        e.register(FILE_POS_ROT_CONSTRUCT, |_, _| Ret::default());
        e.register(0x00a0_01f4, |_, a| ret(a[0] + 0x30));
        let reference = object_with_vtable(&mut e, &[(0x1f4, 0x00a0_01f4)]);
        for (i, value) in [1.5f32, -2.0, 3.25].iter().enumerate() {
            e.mem.set_f32(reference + 0x30 + 4 * i as u32, *value);
        }
        for (i, value) in [0.5f32, 0.25, 8.0].iter().enumerate() {
            e.mem.set_f32(reference + 0x24 + 4 * i as u32, *value);
        }
        let this: Ptr<ExtraStartingPosition> = e.new_object();
        start_log(&mut e);
        let back = e
            .call(0x0043_0780, &args![this, Ptr::<()>::new(reference)])
            .ptr::<ExtraStartingPosition>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x0f);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_STARTING_POSITION_VTABLE);
        assert_eq!(
            calls(&e, FILE_POS_ROT_CONSTRUCT),
            vec![vec![this.addr() + 0xc]]
        );
        let start = this.at(ExtraStartingPosition::startPosition);
        let position = start.at(FilePosRot::pos);
        let rotation = start.at(FilePosRot::rot);
        assert_eq!(e.get(position, NiPoint3::x), 1.5);
        assert_eq!(e.get(position, NiPoint3::y), -2.0);
        assert_eq!(e.get(position, NiPoint3::z), 3.25);
        assert_eq!(e.get(rotation, NiPoint3::x), 0.5);
        assert_eq!(e.get(rotation, NiPoint3::y), 0.25);
        assert_eq!(e.get(rotation, NiPoint3::z), 8.0);
        assert_eq!(rotation.addr(), this.addr() + 0x18);
    }

    #[test]
    fn vector_accessor_is_at_offset_0x24() {
        let mut e = extra_engine();
        assert_eq!(
            e.call(0x0043_0830, &args![Ptr::<()>::new(0x1000)]).u32(),
            0x1024
        );
    }

    /// The doubles `Compare` methods use: a cast that gives back its
    /// argument (or null), the base `Compare`, and a real `memcmp`.
    fn compare_engine(cast_ok: bool, base_result: bool) -> Engine {
        let mut e = extra_engine();
        e.register_double(DYNAMIC_CAST, move |_, a| {
            ret(if cast_ok { a[0] } else { 0 })
        });
        e.register_double(BS_EXTRA_DATA_COMPARE, move |_, _| ret(base_result as u32));
        e.register(MEMCMP, |e, a| {
            let first = e.mem.bytes(a[0], a[2]);
            let second = e.mem.bytes(a[1], a[2]);
            ret(match first.cmp(&second) {
                std::cmp::Ordering::Less => u32::MAX,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            })
        });
        e
    }

    fn starting_position_compare(
        e: &mut Engine,
        this: Ptr<ExtraStartingPosition>,
        other: Ptr<ExtraStartingPosition>,
    ) -> bool {
        e.call(0x0043_0850, &args![this, other]).bool()
    }

    #[test]
    fn starting_position_compare_checks_type_base_and_the_position_bytes() {
        // Not an ExtraStartingPosition.
        let mut e = compare_engine(false, false);
        let this: Ptr<ExtraStartingPosition> = e.new_object();
        let other: Ptr<ExtraStartingPosition> = e.new_object();
        start_log(&mut e);
        assert!(starting_position_compare(&mut e, this, other));
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![
                other.addr(),
                0,
                BS_EXTRA_DATA_TYPE,
                EXTRA_STARTING_POSITION_TYPE,
                0
            ]]
        );
        assert!(calls(&e, BS_EXTRA_DATA_COMPARE).is_empty());

        // The base says they differ.
        let mut e = compare_engine(true, true);
        let this: Ptr<ExtraStartingPosition> = e.new_object();
        let other: Ptr<ExtraStartingPosition> = e.new_object();
        assert!(starting_position_compare(&mut e, this, other));

        // Same bytes: equal. One byte of the rotation differs: not.
        let mut e = compare_engine(true, false);
        let this: Ptr<ExtraStartingPosition> = e.new_object();
        let other: Ptr<ExtraStartingPosition> = e.new_object();
        start_log(&mut e);
        assert!(!starting_position_compare(&mut e, this, other));
        assert_eq!(
            calls(&e, MEMCMP),
            vec![vec![this.addr() + 0xc, other.addr() + 0xc, 0x18]]
        );
        e.mem.set_u8(other.addr() + 0x23, 1);
        assert!(starting_position_compare(&mut e, this, other));
    }

    #[test]
    fn type_49_constructor_clears_its_word() {
        let mut e = extra_engine();
        let this: Ptr<ExtraType49> = e.new_object();
        e.set(this, ExtraType49::unnamed0C, 0xdead);
        let back = e.call(0x0043_08c0, &args![this]).ptr::<ExtraType49>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x49);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_TYPE_49_VTABLE);
        assert_eq!(e.get(this, ExtraType49::unnamed0C), 0);
    }

    #[test]
    fn light_constructor_takes_the_light() {
        let mut e = extra_engine();
        let this: Ptr<ExtraLight> = e.new_object();
        let back = e
            .call(0x0043_08f0, &args![this, Ptr::<()>::new(0x4444)])
            .ptr::<ExtraLight>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x29);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_LIGHT_VTABLE);
        assert_eq!(e.get(this, ExtraLight::pLight).addr(), 0x4444);
    }

    #[test]
    fn light_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = extra_engine();
        e.register(LIGHT_DELETE, |_, _| Ret::default());
        let this: Ptr<ExtraLight> = e.new_object();
        e.call(0x0043_0920, &args![this, 0u32]);
        assert!(e.mem.block_size(this.addr()).is_some());
        e.call(0x0043_0920, &args![this, 1u32]);
        assert_eq!(e.mem.block_size(this.addr()), None);
    }

    #[test]
    fn light_destructor_deletes_the_light_only_when_there_is_one() {
        let mut e = extra_engine();
        e.register(LIGHT_DELETE, |_, _| Ret::default());
        let this: Ptr<ExtraLight> = e.new_object();
        e.set(this, ExtraLight::pLight, Ptr::new(0x4444));
        start_log(&mut e);
        e.call(0x0043_0950, &args![this]);
        assert_eq!(calls(&e, LIGHT_DELETE), vec![vec![0x4444, 1]]);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this.addr()]]);
        let bare: Ptr<ExtraLight> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_0950, &args![bare]);
        assert!(calls(&e, LIGHT_DELETE).is_empty());
    }

    /// The setting accessor: the five bracket settings hold 0, 25, 50, 75,
    /// 100 and the sixth 7.
    fn register_lock_settings(e: &mut Engine) {
        e.register(SETTING_INT_VALUE, |e, a| {
            let values = [0, 25, 50, 75, 100, 7];
            let index = LOCK_LEVEL_SETTINGS
                .iter()
                .position(|setting| *setting == a[0])
                .unwrap();
            let cell = e.mem.alloc(4);
            e.mem.set_i32(cell, values[index]);
            ret(cell)
        });
    }

    fn lock_engine() -> Engine {
        let mut e = extra_engine();
        register_lock_settings(&mut e);
        e
    }

    fn lock(e: &mut Engine, level: u8, flags: i8) -> Ptr<RefrLock> {
        let lock: Ptr<RefrLock> = e.new_object();
        e.set(lock, RefrLock::cBaseLevel, level);
        e.set(lock, RefrLock::cFlags, flags);
        lock
    }

    #[test]
    fn lock_level_is_the_bracket_of_the_level() {
        let mut e = lock_engine();
        let plain = lock(&mut e, 30, 0);
        assert_eq!(
            e.call(0x0043_09e0, &args![plain, Ptr::<()>::new(0)]).i32(),
            2
        );
        let key_only = lock(&mut e, 120, 0);
        assert_eq!(
            e.call(0x0043_09e0, &args![key_only, Ptr::<()>::new(0)])
                .i32(),
            5
        );
    }

    #[test]
    fn lock_level_adds_the_reference_level_for_a_leveled_lock() {
        let mut e = lock_engine();
        e.register(SETTING_FLOAT_VALUE, |e, _| {
            let cell = e.mem.alloc(4);
            e.mem.set_f32(cell, 2.5);
            ret(cell)
        });
        e.register(REFR_GET_CALC_LEVEL, |_, a| {
            assert_eq!(a[1], 0);
            ret(10)
        });
        e.register(ACTOR_GET_LEVEL, |_, _| ret(0xabcd_0005));
        e.set_global(PLAYER, 0x5000);
        let reference = Ptr::<()>::new(0x6000);

        // Unleveled: the base level, no lookups.
        let plain = lock(&mut e, 30, 0);
        start_log(&mut e);
        assert_eq!(e.call(0x0043_0a10, &args![plain, reference]).i32(), 30);
        assert!(calls(&e, REFR_GET_CALC_LEVEL).is_empty());

        // Leveled with a reference: 30 + trunc(2.5) * 10.
        let leveled = lock(&mut e, 30, 4);
        start_log(&mut e);
        assert_eq!(e.call(0x0043_0a10, &args![leveled, reference]).i32(), 50);
        assert_eq!(calls(&e, REFR_GET_CALC_LEVEL), vec![vec![0x6000, 0]]);
        assert_eq!(
            calls(&e, SETTING_FLOAT_VALUE),
            vec![vec![LEVELED_LOCK_SETTING]]
        );

        // Leveled without one: the player's level, 16 bits of it.
        start_log(&mut e);
        assert_eq!(
            e.call(0x0043_0a10, &args![leveled, Ptr::<()>::new(0)])
                .i32(),
            30 + 2 * 5
        );
        assert_eq!(calls(&e, ACTOR_GET_LEVEL), vec![vec![0x5000]]);

        // Capped at 99.
        let high = lock(&mut e, 90, 5);
        assert_eq!(e.call(0x0043_0a10, &args![high, reference]).i32(), 99);
        // The flag byte is signed: a set top bit does not matter.
        let negative = lock(&mut e, 10, -128);
        assert_eq!(e.call(0x0043_0a10, &args![negative, reference]).i32(), 10);
    }

    #[test]
    fn set_locked_sets_or_clears_bit_0_and_resets_tries() {
        let mut e = lock_engine();
        let this = lock(&mut e, 0, 4);
        e.set(this, RefrLock::uiNumTries, 3);
        e.call(0x0043_0a90, &args![this, true]);
        assert_eq!(e.get(this, RefrLock::cFlags), 5);
        assert_eq!(e.get(this, RefrLock::uiNumTries), 3);
        e.call(0x0043_0a90, &args![this, false]);
        assert_eq!(e.get(this, RefrLock::cFlags), 4);
        assert_eq!(e.get(this, RefrLock::uiNumTries), 0);
    }

    #[test]
    fn is_broken_needs_one_try_or_two_by_the_entry_point() {
        for (entry_point_value, tries, broken) in [
            (0.0f32, 0, false),
            (0.0, 1, true),
            (1.0, 1, false),
            (1.0, 2, true),
            (-0.5, 5, true),
        ] {
            let mut e = lock_engine();
            e.set_global(PLAYER, 0x5000);
            e.register_double(HANDLE_ENTRY_POINT, move |e, a| {
                e.mem.set_f32(a[2], entry_point_value);
                Ret::default()
            });
            let this = lock(&mut e, 0, 1);
            e.set(this, RefrLock::uiNumTries, tries);
            start_log(&mut e);
            assert_eq!(
                e.call(0x0043_0ae0, &args![this]).bool(),
                broken,
                "{entry_point_value} {tries}"
            );
            let call = &calls(&e, HANDLE_ENTRY_POINT)[0];
            assert_eq!(&call[..2], &[0x20, 0x5000]);
        }
    }

    #[test]
    fn numeric_value_to_enum_finds_the_first_bracket() {
        let mut e = lock_engine();
        for (value, expected) in [
            (-5, 0),
            (0, 0),
            (1, 1),
            (25, 1),
            (26, 2),
            (75, 3),
            (100, 4),
            (101, 5),
        ] {
            assert_eq!(
                e.call(0x0043_0b40, &args![value]).i32(),
                expected,
                "value {value}"
            );
        }
        // Stops at the first bracket that fits.
        start_log(&mut e);
        e.call(0x0043_0b40, &args![30i32]);
        assert_eq!(calls(&e, SETTING_INT_VALUE).len(), 3);
    }

    #[test]
    fn enum_to_numeric_value_reads_the_bracket_setting() {
        let mut e = lock_engine();
        for (bracket, expected) in [
            (0u32, 0),
            (1, 25),
            (2, 50),
            (3, 75),
            (4, 100),
            (5, 7),
            (6, 0),
            (99, 0),
        ] {
            assert_eq!(
                e.call(0x0043_0bc0, &args![bracket]).i32(),
                expected,
                "bracket {bracket}"
            );
        }
        start_log(&mut e);
        e.call(0x0043_0bc0, &args![6u32]);
        assert!(calls(&e, SETTING_INT_VALUE).is_empty());
    }

    #[test]
    fn lock_extra_constructor_takes_the_lock() {
        let mut e = extra_engine();
        let this: Ptr<ExtraLock> = e.new_object();
        let held: Ptr<RefrLock> = e.new_object();
        let back = e.call(0x0043_0c70, &args![this, held]).ptr::<ExtraLock>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x2a);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_LOCK_VTABLE);
        assert_eq!(e.get(this, ExtraLock::pLock), held);
    }

    #[test]
    fn lock_extra_destructor_frees_the_lock() {
        let mut e = extra_engine();
        let this: Ptr<ExtraLock> = e.new_object();
        let held: Ptr<RefrLock> = e.new_object();
        e.set(this, ExtraLock::pLock, held);
        start_log(&mut e);
        e.call(0x0043_0ca0, &args![this]);
        assert_eq!(e.mem.block_size(held.addr()), None);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![held.addr()]]);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this.addr()]]);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_LOCK_VTABLE);
    }

    /// An `ExtraLock` holding a lock with the given level, key, flags, tries
    /// and times unlocked.
    fn extra_lock(
        e: &mut Engine,
        (level, key, flags, tries, unlocked): (u8, u32, i8, u32, u32),
    ) -> Ptr<ExtraLock> {
        let held = lock(e, level, flags);
        e.set(held, RefrLock::pKey, Ptr::new(key));
        e.set(held, RefrLock::uiNumTries, tries);
        e.set(held, RefrLock::uiTimesUnlocked, unlocked);
        let this: Ptr<ExtraLock> = e.new_object();
        e.set(this, ExtraLock::pLock, held);
        this
    }

    fn lock_compare_engine(cast_ok: bool, base_result: bool) -> Engine {
        let mut e = compare_engine(cast_ok, base_result);
        register_lock_settings(&mut e);
        e
    }

    #[test]
    fn lock_compare_checks_type_base_and_every_lock_field() {
        let base = (30, 7, 1, 2, 3);
        let mut e = lock_compare_engine(false, false);
        let this = extra_lock(&mut e, base);
        let other = extra_lock(&mut e, base);
        start_log(&mut e);
        assert!(e.call(0x0043_0ce0, &args![this, other]).bool());
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![
                other.addr(),
                0,
                BS_EXTRA_DATA_TYPE,
                EXTRA_LOCK_TYPE,
                0
            ]]
        );

        let mut e = lock_compare_engine(true, true);
        let this = extra_lock(&mut e, base);
        let other = extra_lock(&mut e, base);
        assert!(e.call(0x0043_0ce0, &args![this, other]).bool());

        let mut e = lock_compare_engine(true, false);
        let this = extra_lock(&mut e, base);
        // Equal, including a base level that differs inside the same bracket.
        for same in [base, (40, 7, 1, 2, 3)] {
            let other = extra_lock(&mut e, same);
            assert!(!e.call(0x0043_0ce0, &args![this, other]).bool());
        }
        // One field differs each time.
        for (i, different) in [
            (30, 7, 5, 2, 3),
            (30, 8, 1, 2, 3),
            (60, 7, 1, 2, 3),
            (30, 7, 1, 9, 3),
            (30, 7, 1, 2, 9),
        ]
        .into_iter()
        .enumerate()
        {
            let other = extra_lock(&mut e, different);
            assert!(e.call(0x0043_0ce0, &args![this, other]).bool(), "field {i}");
        }
    }

    #[test]
    fn follower_constructor_allocates_the_actor_list() {
        let mut e = extra_engine();
        e.register(ACTOR_LIST_CONSTRUCT, |_, a| ret(a[0]));
        let this: Ptr<ExtraFollower> = e.new_object();
        start_log(&mut e);
        let back = e.call(0x0043_0dd0, &args![this]).ptr::<ExtraFollower>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x1d);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_FOLLOWER_VTABLE);
        assert_eq!(calls(&e, OPERATOR_NEW), vec![vec![8]]);
        let list = e.get(this, ExtraFollower::pActorlist);
        assert_eq!(e.mem.block_size(list.addr()), Some(8));
        assert_eq!(calls(&e, ACTOR_LIST_CONSTRUCT), vec![vec![list.addr()]]);

        // A failed allocation leaves the list null and constructs nothing.
        e.register(OPERATOR_NEW, |_, _| ret(0));
        let failed: Ptr<ExtraFollower> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_0dd0, &args![failed]);
        assert!(e.get(failed, ExtraFollower::pActorlist).is_null());
        assert!(calls(&e, ACTOR_LIST_CONSTRUCT).is_empty());
    }

    #[test]
    fn follower_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = extra_engine();
        e.register(ACTOR_LIST_CLEAR, |_, _| Ret::default());
        let this: Ptr<ExtraFollower> = e.new_object();
        e.call(0x0043_0e70, &args![this, 0u32]);
        assert!(e.mem.block_size(this.addr()).is_some());
        let back = e
            .call(0x0043_0e70, &args![this, 1u32])
            .ptr::<ExtraFollower>();
        assert_eq!(back, this);
        assert_eq!(e.mem.block_size(this.addr()), None);
    }

    #[test]
    fn follower_destructor_clears_then_deletes_the_list() {
        let mut e = extra_engine();
        e.register(ACTOR_LIST_CLEAR, |_, _| Ret::default());
        e.register(ACTOR_LIST_DELETE, |_, _| Ret::default());
        let this: Ptr<ExtraFollower> = e.new_object();
        e.set(this, ExtraFollower::pActorlist, Ptr::new(0x4000));
        start_log(&mut e);
        e.call(0x0043_0ea0, &args![this]);
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .skip(1)
            .map(|(addr, _)| *addr)
            .collect();
        assert_eq!(
            order,
            vec![ACTOR_LIST_CLEAR, ACTOR_LIST_DELETE, BS_EXTRA_DATA_DESTRUCT]
        );
        assert_eq!(calls(&e, ACTOR_LIST_DELETE), vec![vec![0x4000, 1]]);

        // Without a list the clear is still called (on null) and nothing is
        // deleted.
        let bare: Ptr<ExtraFollower> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_0ea0, &args![bare]);
        assert_eq!(calls(&e, ACTOR_LIST_CLEAR), vec![vec![0]]);
        assert!(calls(&e, ACTOR_LIST_DELETE).is_empty());
        assert_eq!(vtable_of(&e, bare.addr()), EXTRA_FOLLOWER_VTABLE);
    }

    #[test]
    fn guarded_ref_data_constructor_builds_the_guard_array() {
        let mut e = extra_engine();
        e.register(GUARDS_ARRAY_CONSTRUCT, |_, _| Ret::default());
        let this: Ptr<ExtraGuardedRefData> = e.new_object();
        start_log(&mut e);
        let back = e
            .call(0x0043_0f30, &args![this])
            .ptr::<ExtraGuardedRefData>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x7c);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_GUARDED_REF_DATA_VTABLE);
        assert_eq!(
            calls(&e, GUARDS_ARRAY_CONSTRUCT),
            vec![vec![this.addr() + 0xc]]
        );
    }

    #[test]
    fn guarded_ref_data_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = extra_engine();
        e.register(GUARDS_ARRAY_DESTRUCT, |_, _| Ret::default());
        let this: Ptr<ExtraGuardedRefData> = e.new_object();
        e.call(0x0043_0fa0, &args![this, 0u32]);
        assert!(e.mem.block_size(this.addr()).is_some());
        let back = e
            .call(0x0043_0fa0, &args![this, 1u32])
            .ptr::<ExtraGuardedRefData>();
        assert_eq!(back, this);
        assert_eq!(e.mem.block_size(this.addr()), None);
    }

    #[test]
    fn guarded_ref_data_destructor_destroys_the_guard_array() {
        let mut e = extra_engine();
        e.register(GUARDS_ARRAY_DESTRUCT, |_, _| Ret::default());
        let this: Ptr<ExtraGuardedRefData> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_0fd0, &args![this]);
        assert_eq!(
            calls(&e, GUARDS_ARRAY_DESTRUCT),
            vec![vec![this.addr() + 0xc]]
        );
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this.addr()]]);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_GUARDED_REF_DATA_VTABLE);
    }

    /// `BSSimpleArray::Find` of the guards: the index of the word at `a[1]`
    /// in the array `a[0]`, or -1.
    fn register_find(e: &mut Engine) {
        e.register(SIMPLE_ARRAY_FIND, |e, a| {
            assert_eq!(&a[2..], &[0, GUARD_COMPARE]);
            let wanted = e.mem.u32(a[1]);
            let found = array_items(e, a[0]).iter().position(|item| *item == wanted);
            ret(found.map_or(u32::MAX, |i| i as u32))
        });
    }

    fn guards_object(e: &mut Engine, guards: &[u32]) -> Ptr<ExtraGuardedRefData> {
        let this: Ptr<ExtraGuardedRefData> = e.new_object();
        make_array(e, this.addr() + 0xc, guards);
        this
    }

    #[test]
    fn guarded_ref_data_compare_needs_the_same_guards() {
        // The base test is the other way round from the other `Compare`
        // methods: a base result of false means "differs".
        let mut e = compare_engine(true, true);
        register_find(&mut e);
        let this = guards_object(&mut e, &[1, 2, 3]);
        let compare = |e: &mut Engine, other: Ptr<ExtraGuardedRefData>| {
            e.call(0x0043_1030, &args![this, other]).bool()
        };
        let reordered = guards_object(&mut e, &[3, 1, 2]);
        assert!(!compare(&mut e, reordered));
        let shorter = guards_object(&mut e, &[1, 2]);
        assert!(compare(&mut e, shorter));
        let different = guards_object(&mut e, &[1, 2, 4]);
        assert!(compare(&mut e, different));
        let empty = guards_object(&mut e, &[]);
        let also_empty = guards_object(&mut e, &[]);
        assert!(!e.call(0x0043_1030, &args![empty, also_empty]).bool());

        let mut e = compare_engine(true, false);
        register_find(&mut e);
        let this = guards_object(&mut e, &[1]);
        let other = guards_object(&mut e, &[1]);
        start_log(&mut e);
        assert!(e.call(0x0043_1030, &args![this, other]).bool());
        assert_eq!(
            calls(&e, BS_EXTRA_DATA_COMPARE),
            vec![vec![this.addr(), other.addr()]]
        );
        assert!(calls(&e, SIMPLE_ARRAY_SIZE).is_empty());
    }

    #[test]
    fn add_guard_appends_a_new_value_once() {
        let mut e = extra_engine();
        register_find(&mut e);
        e.register(REFR_GUARD_VALUE, |_, a| ret(a[0] + 0x100));
        let this = guards_object(&mut e, &[0x1111]);
        let reference = Ptr::<()>::new(0x2000);
        start_log(&mut e);
        e.call(0x0043_10d0, &args![this, reference]);
        assert_eq!(array_items(&e, this.addr() + 0xc), vec![0x1111, 0x2100]);
        assert_eq!(calls(&e, REFR_GUARD_VALUE), vec![vec![0x2000]]);
        assert_eq!(calls(&e, SIMPLE_ARRAY_ADD).len(), 1);
        // Already present: found, nothing appended.
        start_log(&mut e);
        e.call(0x0043_10d0, &args![this, reference]);
        assert_eq!(array_items(&e, this.addr() + 0xc), vec![0x1111, 0x2100]);
        assert!(calls(&e, SIMPLE_ARRAY_ADD).is_empty());
    }

    #[test]
    fn guard_notification_reaches_the_process_of_each_acting_form() {
        let mut e = extra_engine();
        // Ids 10, 30, 40 and 50 name forms (kept in a table the double
        // reads); 20 names none.
        e.map(0x011d_e100, 8);
        e.register(LOOKUP_FORM_BY_ID, |e, a| {
            let forms = e.mem.u32(0x011d_e100);
            let slot = match a[0] {
                10 => 0,
                30 => 2,
                40 => 3,
                50 => 4,
                _ => return ret(0),
            };
            ret(e.mem.u32(forms + 4 * slot))
        });
        // The two vtable tests read a byte each from the form; the process
        // lookup reads a pointer.
        e.register(0x00a0_00f0, |e, a| ret(e.mem.u8(a[0] + 0x30) as u32));
        e.register(0x00a0_0100, |e, a| ret(e.mem.u8(a[0] + 0x31) as u32));
        e.register(FORM_PROCESS, |e, a| ret(e.mem.u32(a[0] + 0x34)));
        e.register(0x00a0_060c, |_, _| Ret::default());
        let process = object_with_vtable(&mut e, &[(0x60c, 0x00a0_060c)]);
        let mut forms = vec![];
        for (first, second, has_process) in [
            (1u8, 1u8, true),
            (0, 0, false),
            (0, 1, true),
            (1, 0, true),
            (1, 1, false),
        ] {
            let form = object_with_vtable(&mut e, &[(0xf0, 0x00a0_00f0), (0x100, 0x00a0_0100)]);
            e.mem.set_u8(form + 0x30, first);
            e.mem.set_u8(form + 0x31, second);
            if has_process {
                e.mem.set_u32(form + 0x34, process);
            }
            forms.push(form);
        }
        let table = e.mem.alloc(0x20);
        for (i, form) in forms.iter().enumerate() {
            e.mem.set_u32(table + 4 * i as u32, *form);
        }
        e.mem.set_u32(0x011d_e100, table);
        let this = guards_object(&mut e, &[10, 20, 30, 40, 50]);
        start_log(&mut e);
        e.call(0x0043_1120, &args![this, 0xaau32, 0xbbu32]);
        // Only form 10 passes everything; the arguments are the form, then
        // the second and the first word.
        assert_eq!(
            calls(&e, 0x00a0_060c),
            vec![vec![process, forms[0], 0xbb, 0xaa]]
        );
        assert_eq!(calls(&e, LOOKUP_FORM_BY_ID).len(), 5);
    }

    #[test]
    fn type_2e_constructor_clears_its_words() {
        let mut e = extra_engine();
        let this: Ptr<ExtraType2e> = e.new_object();
        e.set(this, ExtraType2e::unnamed0C, 5);
        e.set(this, ExtraType2e::unnamed10, 6);
        let back = e.call(0x0043_11f0, &args![this]).ptr::<ExtraType2e>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x2e);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_TYPE_2E_VTABLE);
        assert_eq!(e.get(this, ExtraType2e::unnamed0C), 0);
        assert_eq!(e.get(this, ExtraType2e::unnamed10), 0);
    }

    #[test]
    fn teleport_constructor_takes_the_data() {
        let mut e = extra_engine();
        let this: Ptr<ExtraTeleport> = e.new_object();
        let back = e
            .call(0x0043_1230, &args![this, Ptr::<()>::new(0x7777)])
            .ptr::<ExtraTeleport>();
        assert_eq!(back, this);
        assert_eq!(extra_type(&e, this.addr()), 0x2b);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_TELEPORT_VTABLE);
        assert_eq!(e.get(this, ExtraTeleport::pData).addr(), 0x7777);
    }

    #[test]
    fn teleport_destructor_deletes_the_data_only_when_there_is_some() {
        let mut e = extra_engine();
        e.register(TELEPORT_DATA_DELETE, |_, _| Ret::default());
        let this: Ptr<ExtraTeleport> = e.new_object();
        e.set(this, ExtraTeleport::pData, Ptr::new(0x7777));
        start_log(&mut e);
        e.call(0x0043_1260, &args![this]);
        assert_eq!(calls(&e, TELEPORT_DATA_DELETE), vec![vec![0x7777, 1]]);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this.addr()]]);
        assert_eq!(vtable_of(&e, this.addr()), EXTRA_TELEPORT_VTABLE);
        let bare: Ptr<ExtraTeleport> = e.new_object();
        start_log(&mut e);
        e.call(0x0043_1260, &args![bare]);
        assert!(calls(&e, TELEPORT_DATA_DELETE).is_empty());
    }
}
