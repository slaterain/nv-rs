//! `fallout shared/extradataobjects.cpp` (Xbox PDB source unit), part 2: its functions from `004353f0` up to
//! (not including) `004382a0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::extradataobjects`]; anything public there may be used here.
//!
//! Translated so far: the first 40 functions of the range, `004353f0` to
//! `00435f50` (`ExtraXTarget::Compare` to the scalar deleting destructor of
//! `ExtraRefractionProperty`). The next session continues at `00435f80`.
//!
//! Notes for the next session:
//! - Every class here is a `BSExtraData` subclass whose payload starts at
//!   +0xc. The constructors call the base constructor (`0040ec80`, type byte),
//!   store the vtable, then initialise the payload; the destructors store the
//!   vtable again (not all of them) and end with the base destructor
//!   (`0040ecb0`). The exception-unwinding frames are not translated.
//! - `00435ac0` and `00435bb0` (the destructors of `ExtraSavedAnimation` and
//!   `ExtraSavedHavokData`, called by their scalar deleting destructors) are
//!   not in Ghidra's function list, so the ledger does not queue them; they
//!   are called by address. `00435f80` (`ExtraRefractionProperty`'s
//!   destructor) is the first function of the next session.

use super::extradatalist_p3::{ExtraDroppedItemList, ExtraItemDropper};
#[allow(unused_imports)]
use super::extradataobjects::*;
#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleArray;

/// `operator delete(pointer)` (cdecl, one stack argument).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `BSExtraData::BSExtraData(type)` (the base constructor; `this`, then the
/// extra-data type byte).
const BS_EXTRA_DATA_CONSTRUCT: u32 = 0x0040_ec80;
/// `BSExtraData::~BSExtraData` (the base destructor body).
const BS_EXTRA_DATA_DESTRUCT: u32 = 0x0040_ecb0;
/// `__RTDynamicCast(object, vfDelta, sourceType, targetType, isReference)`.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `RTTI Type Descriptor` of `BSExtraData`, the source type of the
/// `dynamic_cast`s in the `Compare` methods.
const BS_EXTRA_DATA_TYPE: u32 = 0x0118_3b2c;
/// `BSSimpleArray<T,1024>::size` of the folded body (`this` is the array,
/// the count is returned in EAX).
const SIMPLE_ARRAY_SIZE: u32 = 0x0044_ddc0;
/// `BSSimpleArray<T,1024>::operator[]` of the folded body: the address of
/// the element slot.
const SIMPLE_ARRAY_AT: u32 = 0x006a_7ad0;
/// The array element removal `RemoveOldHits` uses (`this` is the array; the
/// arguments are the index and the flag 1). The engine map names the folded
/// body `BSSimpleArray<BGSBodyPart_P_1024>::_MoveItems`.
const SIMPLE_ARRAY_REMOVE_AT: u32 = 0x009a_4320;
/// The array append `AddHit` uses (`this` is the array; the argument is the
/// address of the element to copy in; returns the new index).
const SIMPLE_ARRAY_APPEND: u32 = 0x0043_8610;
/// The `ExtraFriendHits::Hits` array's constructor (`this` is the array).
const HITS_ARRAY_CONSTRUCT: u32 = 0x0043_85c0;
/// The `ExtraFriendHits::Hits` array's destructor (`this` is the array).
const HITS_ARRAY_DESTRUCT: u32 = 0x0043_85f0;
/// `MultiBoundMarkerData::Compare` (Xbox PDB; `this`, other): true when
/// `other` is null or the two differ.
const MULTI_BOUND_MARKER_DATA_COMPARE: u32 = 0x0043_9060;
/// `NiPointer<T>::NiPointer(pointer)` (`this` is the smart pointer): stores
/// the pointer and takes a reference when it is not null.
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
/// `NiPointer<T>::operator=(pointer)` (`this` is the smart pointer).
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `NiPointer<T>::~NiPointer` (`this` is the smart pointer): releases the
/// reference when it holds one.
const NI_POINTER_DESTRUCT: u32 = 0x0045_cec0;
/// The constructor of the `BSSimpleList<TESObjectREFR *>` inside
/// `ExtraDroppedItemList` (`this` is the list; empties it).
const SIMPLE_LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// The list's node release (`this` is the list; walks the nodes and frees
/// them).
const SIMPLE_LIST_CLEAR: u32 = 0x0047_0470;
/// The list's destructor body (`this` is the list; it runs the node release
/// again).
const SIMPLE_LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// The word store `ExtraSavedAnimation::DeleteBuffer` builds its holder with
/// (`this` is the holder, the argument the buffer pointer; returns `this`).
/// The engine map names the folded body `BGSChangeFlags::IsForcedChange`.
const HOLDER_CONSTRUCT: u32 = 0x008c_71b0;
/// The destructor of `ExtraSavedAnimation` (not in Ghidra's function list).
const EXTRA_SAVED_ANIMATION_DESTRUCT: u32 = 0x0043_5ac0;
/// The destructor of `ExtraSavedHavokData` (not in Ghidra's function list).
const EXTRA_SAVED_HAVOK_DATA_DESTRUCT: u32 = 0x0043_5bb0;
/// `ExtraRefractionProperty`'s destructor body (next session of this file).
const EXTRA_REFRACTION_PROPERTY_DESTRUCT: u32 = 0x0043_5f80;
/// A float game-setting accessor: `this` is the setting (an exe global), the
/// result is the address of its value.
const SETTING_FLOAT_VALUE: u32 = 0x0040_3e20;
/// The float setting `AddHit` compares the age of the newest hit with.
const HIT_SETTING_ADD: u32 = 0x011c_d78c;
/// The float setting `RemoveOldHits` compares the age of each hit with.
const HIT_SETTING_REMOVE: u32 = 0x011c_d580;
/// The global float `00435dd0` returns (the time the hit ages are measured
/// against).
const HIT_CLOCK: u32 = 0x011f_1bf0;

/// Extra-data type bytes, as passed to the base constructor.
const TYPE_EMITTANCE_SOURCE: u32 = 0x67;
const TYPE_MULTI_BOUND_REF: u32 = 0x63;
const TYPE_MULTI_BOUND_DATA: u32 = 0x62;
const TYPE_MULTI_BOUND: u32 = 0x61;
const TYPE_OCCLUSION_PLANE: u32 = 0x71;
const TYPE_PORTAL: u32 = 0x78;
const TYPE_ROOM: u32 = 0x79;
const TYPE_ITEM_DROPPER: u32 = 0x39;
const TYPE_DROPPED_ITEM_LIST: u32 = 0x3a;
const TYPE_SAVED_ANIMATION: u32 = 0x42;
const TYPE_SAVED_HAVOK_DATA: u32 = 0x3d;
const TYPE_FRIEND_HITS: u32 = 0x45;
const TYPE_HEADING_TARGET: u32 = 0x46;
const TYPE_REFRACTION_PROPERTY: u32 = 0x48;

/// The vtables.
const EXTRA_EMITTANCE_SOURCE_VTABLE: u32 = 0x0101_5e34;
const EXTRA_MULTI_BOUND_REF_VTABLE: u32 = 0x0101_5e40;
const EXTRA_MULTI_BOUND_DATA_VTABLE: u32 = 0x0101_5e4c;
const EXTRA_MULTI_BOUND_VTABLE: u32 = 0x0101_5e58;
const EXTRA_OCCLUSION_PLANE_VTABLE: u32 = 0x0101_5e64;
const EXTRA_PORTAL_VTABLE: u32 = 0x0101_5e70;
const EXTRA_ROOM_VTABLE: u32 = 0x0101_5e7c;
const EXTRA_ITEM_DROPPER_VTABLE: u32 = 0x0101_5e88;
const EXTRA_DROPPED_ITEM_LIST_VTABLE: u32 = 0x0101_5e94;
const EXTRA_SAVED_ANIMATION_VTABLE: u32 = 0x0101_5ea0;
const EXTRA_SAVED_HAVOK_DATA_VTABLE: u32 = 0x0101_5eac;
const EXTRA_FRIEND_HITS_VTABLE: u32 = 0x0101_5eb8;
const EXTRA_HEADING_TARGET_VTABLE: u32 = 0x0101_5968;
const EXTRA_REFRACTION_PROPERTY_VTABLE: u32 = 0x0101_5ec4;

/// `RTTI Type Descriptor`s of the classes whose `Compare` casts `other`.
const EXTRA_X_TARGET_TYPE: u32 = 0x0118_4f10;
const EXTRA_EMITTANCE_SOURCE_TYPE: u32 = 0x0118_4f2c;
const EXTRA_MULTI_BOUND_REF_TYPE: u32 = 0x0118_4f50;
const EXTRA_MULTI_BOUND_DATA_TYPE: u32 = 0x0118_4f74;

layout! {
    /// `ExtraEmittanceSource` (Xbox PDB), type `0x67`, 0x10 bytes.
    pub struct ExtraEmittanceSource: 0x10 {
        /// `pSource` (Xbox PDB): `TESForm*`.
        0x0C pSource: Ptr,
    }

    /// `ExtraMultiBoundRef` (Xbox PDB), type `0x63`, 0x10 bytes.
    pub struct ExtraMultiBoundRef: 0x10 {
        /// `pBoundRef` (Xbox PDB): `TESObjectREFR*`.
        0x0C pBoundRef: Ptr,
    }

    /// `ExtraMultiBoundData` (Xbox PDB), type `0x62`, 0x10 bytes.
    pub struct ExtraMultiBoundData: 0x10 {
        /// `pBound` (Xbox PDB): `MultiBoundMarkerData*`, owned.
        0x0C pBound: Ptr,
    }

    /// `ExtraMultiBound` (Xbox PDB), type `0x61`, 0x10 bytes.
    pub struct ExtraMultiBound: 0x10 {
        /// `spBound` (Xbox PDB): `NiPointer<BSMultiBound>`.
        0x0C spBound: Ptr,
    }

    /// `ExtraOcclusionPlane` (Xbox PDB), type `0x71`, 0x10 bytes.
    pub struct ExtraOcclusionPlane: 0x10 {
        /// `spPlane` (Xbox PDB): `NiPointer<BSOcclusionPlane>`.
        0x0C spPlane: Ptr,
    }

    /// `ExtraPortal` (Xbox PDB), type `0x78`, 0x10 bytes.
    pub struct ExtraPortal: 0x10 {
        /// `spPortal` (Xbox PDB): `NiPointer<BSPortal>`.
        0x0C spPortal: Ptr,
    }

    /// `ExtraRoom` (Xbox PDB), type `0x79`, 0x10 bytes.
    pub struct ExtraRoom: 0x10 {
        /// `spRoom` (Xbox PDB): `NiPointer<BSMultiBoundRoom>`.
        0x0C spRoom: Ptr,
    }

    /// `ExtraSavedAnimation` (Xbox PDB), type `0x42`, 0x10 bytes.
    pub struct ExtraSavedAnimation: 0x10 {
        /// `pAnimationBuffer` (Xbox PDB): `char*`.
        0x0C pAnimationBuffer: Ptr,
    }

    /// `ExtraSavedHavokData` (Xbox PDB), type `0x3d`, 0x10 bytes.
    pub struct ExtraSavedHavokData: 0x10 {
        /// `pHavokBuffer` (Xbox PDB): `char*`.
        0x0C pHavokBuffer: Ptr,
    }

    /// `ExtraFriendHits` (Xbox PDB), type `0x45`, 0x1C bytes.
    pub struct ExtraFriendHits: 0x1C {
        /// `Hits` (Xbox PDB): `BSSimpleArray<CombatTimeStamp,1024>`; a
        /// `CombatTimeStamp` is one `float` (the time of the hit).
        0x0C Hits: Inline<BSSimpleArray>,
    }

    /// `ExtraHeadingTarget` (Xbox PDB), type `0x46`, 0x10 bytes.
    pub struct ExtraHeadingTarget: 0x10 {
        /// `pheadingtarget` (Xbox PDB): `TESObjectREFR*`.
        0x0C pheadingtarget: Ptr,
    }

    /// `ExtraRefractionProperty` (Xbox PDB), type `0x48`, 0x10 bytes.
    pub struct ExtraRefractionProperty: 0x10 {
        /// `fRefractionPower` (Xbox PDB).
        0x0C fRefractionPower: f32,
    }
}

/// `operator delete(this)` when bit 0 of `flags` is set (the tail of every
/// scalar deleting destructor).
fn delete_when_asked(e: &mut Engine, this: Ptr, flags: u32) {
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
}

/// The base constructor, then the vtable store that follows it.
fn construct_base(e: &mut Engine, this: Ptr, extra_type: u32, vtable: u32) {
    e.call(BS_EXTRA_DATA_CONSTRUCT, &args![this, extra_type]);
    e.mem.set_u32(this.addr(), vtable);
}

/// The address of the payload word at +0xc.
fn payload<T>(this: Ptr<T>) -> Ptr {
    Ptr::new(this.addr() + 0xc)
}

/// A constructor that stores a null pointer in the payload word.
fn construct_null_payload(e: &mut Engine, this: Ptr, extra_type: u32, vtable: u32) {
    construct_base(e, this, extra_type, vtable);
    e.mem.set_u32(this.addr() + 0xc, 0);
}

/// A constructor of the classes that hold a `NiPointer` at +0xc: the smart
/// pointer is constructed from null, then assigned null. The
/// exception-unwinding frame is not translated.
fn construct_ni_pointer_holder(e: &mut Engine, this: Ptr, extra_type: u32, vtable: u32) {
    construct_base(e, this, extra_type, vtable);
    e.call(NI_POINTER_CONSTRUCT, &args![payload(this), 0u32]);
    e.call(NI_POINTER_ASSIGN, &args![payload(this), 0u32]);
}

/// The compare of the classes whose only payload is one word at +0xc: casts
/// `other` to the class with the RTTI descriptor `target_type` (true when it
/// is not one), then reports whether the words differ.
fn compare_payload_word<T>(e: &mut Engine, this: Ptr<T>, other: Ptr, target_type: u32) -> bool {
    let cast: Ptr = e
        .call(
            DYNAMIC_CAST,
            &args![other, 0u32, BS_EXTRA_DATA_TYPE, target_type, 0u32],
        )
        .ptr();
    if cast.is_null() {
        return true;
    }
    e.mem.u32(this.addr() + 0xc) != e.mem.u32(cast.addr() + 0xc)
}

// Translated from 004353f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraXTarget::Compare` (Xbox PDB): true when `other` is not an
/// `ExtraXTarget` or when `pTarget` differs (the base compare is not asked).
pub fn extra_x_target_compare(e: &mut Engine, this: Ptr<ExtraXTarget>, other: Ptr) -> bool {
    compare_payload_word(e, this, other, EXTRA_X_TARGET_TYPE)
}

// Translated from 00435440 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraEmittanceSource`'s constructor (the engine map has no name for it):
/// extra-data type 0x67, no source. Returns `this`.
pub fn fn_00435440(e: &mut Engine, this: Ptr<ExtraEmittanceSource>) -> Ptr<ExtraEmittanceSource> {
    construct_null_payload(
        e,
        this.cast(),
        TYPE_EMITTANCE_SOURCE,
        EXTRA_EMITTANCE_SOURCE_VTABLE,
    );
    this
}

// Translated from 00435470 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraEmittanceSource`'s scalar deleting destructor (the engine map has no
/// name for it): the destructor, then `operator delete` when `flags & 1`.
/// Returns `this`.
pub fn fn_00435470(
    e: &mut Engine,
    this: Ptr<ExtraEmittanceSource>,
    flags: u32,
) -> Ptr<ExtraEmittanceSource> {
    fn_004354a0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 004354a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraEmittanceSource`'s destructor body (the engine map has no name for
/// it): resets the vtable, then runs the base destructor. The source is not
/// owned.
pub fn fn_004354a0(e: &mut Engine, this: Ptr<ExtraEmittanceSource>) {
    e.mem.set_u32(this.addr(), EXTRA_EMITTANCE_SOURCE_VTABLE);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 004354c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraEmittanceSource::Compare` (Xbox PDB): true when `other` is not an
/// `ExtraEmittanceSource` or when `pSource` differs.
pub fn extra_emittance_source_compare(
    e: &mut Engine,
    this: Ptr<ExtraEmittanceSource>,
    other: Ptr,
) -> bool {
    compare_payload_word(e, this, other, EXTRA_EMITTANCE_SOURCE_TYPE)
}

// Translated from 00435510 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBoundRef`'s constructor (the engine map has no name for it):
/// extra-data type 0x63, no reference. Returns `this`.
pub fn fn_00435510(e: &mut Engine, this: Ptr<ExtraMultiBoundRef>) -> Ptr<ExtraMultiBoundRef> {
    construct_null_payload(
        e,
        this.cast(),
        TYPE_MULTI_BOUND_REF,
        EXTRA_MULTI_BOUND_REF_VTABLE,
    );
    this
}

// Translated from 00435540 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBoundRef::Compare` (Xbox PDB): true when `other` is not an
/// `ExtraMultiBoundRef` or when `pBoundRef` differs.
pub fn extra_multi_bound_ref_compare(
    e: &mut Engine,
    this: Ptr<ExtraMultiBoundRef>,
    other: Ptr,
) -> bool {
    compare_payload_word(e, this, other, EXTRA_MULTI_BOUND_REF_TYPE)
}

// Translated from 00435590 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBoundData`'s constructor (the engine map has no name for it):
/// extra-data type 0x62, no bound data. Returns `this`.
pub fn fn_00435590(e: &mut Engine, this: Ptr<ExtraMultiBoundData>) -> Ptr<ExtraMultiBoundData> {
    construct_null_payload(
        e,
        this.cast(),
        TYPE_MULTI_BOUND_DATA,
        EXTRA_MULTI_BOUND_DATA_VTABLE,
    );
    this
}

// Translated from 004355c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBoundData::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_multi_bound_data_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraMultiBoundData>,
    flags: u32,
) -> Ptr<ExtraMultiBoundData> {
    fn_004355f0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 004355f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBoundData`'s destructor body (the engine map has no name for
/// it): resets the vtable, `operator delete`s the owned bound data (without a
/// null test), then runs the base destructor.
pub fn fn_004355f0(e: &mut Engine, this: Ptr<ExtraMultiBoundData>) {
    e.mem.set_u32(this.addr(), EXTRA_MULTI_BOUND_DATA_VTABLE);
    let bound = e.get(this, ExtraMultiBoundData::pBound);
    e.call(OPERATOR_DELETE, &args![bound]);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 00435630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBoundData::Compare` (Xbox PDB): true when `other` is not an
/// `ExtraMultiBoundData`, or when `MultiBoundMarkerData::Compare` (called on
/// this one's `pBound` with the other's) says they differ.
pub fn extra_multi_bound_data_compare(
    e: &mut Engine,
    this: Ptr<ExtraMultiBoundData>,
    other: Ptr,
) -> bool {
    let cast: Ptr<ExtraMultiBoundData> = e
        .call(
            DYNAMIC_CAST,
            &args![
                other,
                0u32,
                BS_EXTRA_DATA_TYPE,
                EXTRA_MULTI_BOUND_DATA_TYPE,
                0u32
            ],
        )
        .ptr();
    if cast.is_null() {
        return true;
    }
    let other_bound = e.get(cast, ExtraMultiBoundData::pBound);
    let bound = e.get(this, ExtraMultiBoundData::pBound);
    e.call(MULTI_BOUND_MARKER_DATA_COMPARE, &args![bound, other_bound])
        .bool()
}

// Translated from 00435690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBound`'s constructor (the engine map has no name for it):
/// extra-data type 0x61, an empty `NiPointer` (constructed from null, then
/// assigned null). Returns `this`.
pub fn fn_00435690(e: &mut Engine, this: Ptr<ExtraMultiBound>) -> Ptr<ExtraMultiBound> {
    construct_ni_pointer_holder(e, this.cast(), TYPE_MULTI_BOUND, EXTRA_MULTI_BOUND_VTABLE);
    this
}

// Translated from 00435710 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBound::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_multi_bound_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraMultiBound>,
    flags: u32,
) -> Ptr<ExtraMultiBound> {
    fn_00435740(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00435740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraMultiBound`'s destructor body (the engine map has no name for it):
/// destroys the `NiPointer` at +0xc, then runs the base destructor (the
/// vtable is not stored again). The exception-unwinding frame is not
/// translated.
pub fn fn_00435740(e: &mut Engine, this: Ptr<ExtraMultiBound>) {
    e.call(NI_POINTER_DESTRUCT, &args![payload(this)]);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 004357a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraOcclusionPlane`'s constructor (the engine map has no name for it):
/// extra-data type 0x71, an empty `NiPointer`. Returns `this`.
pub fn fn_004357a0(e: &mut Engine, this: Ptr<ExtraOcclusionPlane>) -> Ptr<ExtraOcclusionPlane> {
    construct_ni_pointer_holder(
        e,
        this.cast(),
        TYPE_OCCLUSION_PLANE,
        EXTRA_OCCLUSION_PLANE_VTABLE,
    );
    this
}

// Translated from 00435820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraPortal`'s constructor (the engine map has no name for it):
/// extra-data type 0x78, an empty `NiPointer`. Returns `this`.
pub fn fn_00435820(e: &mut Engine, this: Ptr<ExtraPortal>) -> Ptr<ExtraPortal> {
    construct_ni_pointer_holder(e, this.cast(), TYPE_PORTAL, EXTRA_PORTAL_VTABLE);
    this
}

// Translated from 004358a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraRoom`'s constructor (the engine map has no name for it):
/// extra-data type 0x79, an empty `NiPointer`. Returns `this`.
pub fn fn_004358a0(e: &mut Engine, this: Ptr<ExtraRoom>) -> Ptr<ExtraRoom> {
    construct_ni_pointer_holder(e, this.cast(), TYPE_ROOM, EXTRA_ROOM_VTABLE);
    this
}

// Translated from 00435920 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraItemDropper`'s constructor (the engine map has no name for it):
/// extra-data type 0x39, no dropper. Returns `this`.
pub fn fn_00435920(e: &mut Engine, this: Ptr<ExtraItemDropper>) -> Ptr<ExtraItemDropper> {
    construct_null_payload(e, this.cast(), TYPE_ITEM_DROPPER, EXTRA_ITEM_DROPPER_VTABLE);
    this
}

// Translated from 00435950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDroppedItemList`'s constructor (the engine map has no name for it):
/// extra-data type 0x3a, then the constructor of the embedded list at +0xc.
/// Returns `this`. The exception-unwinding frame is not translated.
pub fn fn_00435950(e: &mut Engine, this: Ptr<ExtraDroppedItemList>) -> Ptr<ExtraDroppedItemList> {
    construct_base(
        e,
        this.cast(),
        TYPE_DROPPED_ITEM_LIST,
        EXTRA_DROPPED_ITEM_LIST_VTABLE,
    );
    e.call(SIMPLE_LIST_CONSTRUCT, &args![payload(this)]);
    this
}

// Translated from 004359c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDroppedItemList::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_dropped_item_list_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraDroppedItemList>,
    flags: u32,
) -> Ptr<ExtraDroppedItemList> {
    fn_004359f0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 004359f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDroppedItemList`'s destructor body (the engine map has no name for
/// it): resets the vtable, releases the embedded list's nodes, runs the
/// list's destructor body, then the base destructor. The exception-unwinding
/// frame is not translated.
pub fn fn_004359f0(e: &mut Engine, this: Ptr<ExtraDroppedItemList>) {
    e.mem.set_u32(this.addr(), EXTRA_DROPPED_ITEM_LIST_VTABLE);
    e.call(SIMPLE_LIST_CLEAR, &args![payload(this)]);
    e.call(SIMPLE_LIST_DESTRUCT, &args![payload(this)]);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 00435a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraSavedAnimation`'s constructor (the engine map has no name for it):
/// extra-data type 0x42, no buffer. Returns `this`.
pub fn fn_00435a60(e: &mut Engine, this: Ptr<ExtraSavedAnimation>) -> Ptr<ExtraSavedAnimation> {
    construct_null_payload(
        e,
        this.cast(),
        TYPE_SAVED_ANIMATION,
        EXTRA_SAVED_ANIMATION_VTABLE,
    );
    this
}

// Translated from 00435a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraSavedAnimation::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor (`00435ac0`, which Ghidra has no function for, called by
/// address), then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_saved_animation_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraSavedAnimation>,
    flags: u32,
) -> Ptr<ExtraSavedAnimation> {
    e.call(EXTRA_SAVED_ANIMATION_DESTRUCT, &args![this]);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00435b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases the buffer a pointer holder owns (the engine map has no name for
/// it): `this` is the address of the holder's pointer; the pointed-to block
/// is `operator delete`d (without a null test) and the pointer set to null.
pub fn fn_00435b20(e: &mut Engine, this: Ptr) {
    let buffer = e.mem.u32(this.addr());
    e.call(OPERATOR_DELETE, &args![buffer]);
    e.mem.set_u32(this.addr(), 0);
}

// Translated from 00435b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraSavedHavokData`'s constructor (the engine map has no name for it):
/// extra-data type 0x3d, no buffer. Returns `this`.
pub fn fn_00435b50(e: &mut Engine, this: Ptr<ExtraSavedHavokData>) -> Ptr<ExtraSavedHavokData> {
    construct_null_payload(
        e,
        this.cast(),
        TYPE_SAVED_HAVOK_DATA,
        EXTRA_SAVED_HAVOK_DATA_VTABLE,
    );
    this
}

// Translated from 00435b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraSavedHavokData::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor (`00435bb0`, which Ghidra has no function for, called by
/// address), then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_saved_havok_data_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraSavedHavokData>,
    flags: u32,
) -> Ptr<ExtraSavedHavokData> {
    e.call(EXTRA_SAVED_HAVOK_DATA_DESTRUCT, &args![this]);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00435c10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraSavedAnimation::DeleteBuffer` (Xbox PDB): builds a one-word holder
/// on the stack around `pAnimationBuffer` and releases it (`operator delete`
/// of the buffer, holder cleared). `pAnimationBuffer` itself keeps its value.
pub fn extra_saved_animation_delete_buffer(e: &mut Engine, this: Ptr<ExtraSavedAnimation>) {
    let buffer = e.get(this, ExtraSavedAnimation::pAnimationBuffer);
    e.with_stack(4, |e, holder| {
        let built = e.call(HOLDER_CONSTRUCT, &args![holder, buffer]).ptr();
        fn_00435b20(e, built);
    });
}

// Translated from 00435c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFriendHits::ExtraFriendHits` (Xbox PDB): extra-data type 0x45, then
/// the constructor of the `Hits` array. Returns `this`. The
/// exception-unwinding frame is not translated.
pub fn extra_friend_hits_extra_friend_hits(
    e: &mut Engine,
    this: Ptr<ExtraFriendHits>,
) -> Ptr<ExtraFriendHits> {
    construct_base(e, this.cast(), TYPE_FRIEND_HITS, EXTRA_FRIEND_HITS_VTABLE);
    e.call(HITS_ARRAY_CONSTRUCT, &args![payload(this)]);
    this
}

// Translated from 00435cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFriendHits::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then `operator delete` when `flags & 1`. Returns `this`.
pub fn extra_friend_hits_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraFriendHits>,
    flags: u32,
) -> Ptr<ExtraFriendHits> {
    fn_00435ce0(e, this);
    delete_when_asked(e, this.cast(), flags);
    this
}

// Translated from 00435ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFriendHits`' destructor body (the engine map has no name for it):
/// destroys the `Hits` array, then runs the base destructor (the vtable is
/// not stored again). The exception-unwinding frame is not translated.
pub fn fn_00435ce0(e: &mut Engine, this: Ptr<ExtraFriendHits>) {
    e.call(HITS_ARRAY_DESTRUCT, &args![payload(this)]);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

/// The count of a `BSSimpleArray`.
fn array_size(e: &mut Engine, array: Ptr<BSSimpleArray>) -> u32 {
    e.call(SIMPLE_ARRAY_SIZE, &args![array]).u32()
}

/// The address of slot `index` of a `BSSimpleArray`.
fn array_slot(e: &mut Engine, array: Ptr<BSSimpleArray>, index: u32) -> Ptr {
    e.call(SIMPLE_ARRAY_AT, &args![array, index]).ptr()
}

/// The age (`00435e00`) of the time stamp stored in the array slot at `slot`,
/// which the game first copies into a local.
fn stamp_age(e: &mut Engine, slot: Ptr) -> f32 {
    let stamp = e.mem.u32(slot.addr());
    e.with_stack(4, |e, holder| {
        e.mem.set_u32(holder.addr(), stamp);
        fn_00435e00(e, holder)
    })
}

/// The float value of a game setting, read through its accessor.
fn setting_value(e: &mut Engine, setting: u32) -> f32 {
    let slot: Ptr = e.call(SETTING_FLOAT_VALUE, &args![setting]).ptr();
    f32::from_bits(e.mem.u32(slot.addr()))
}

/// The tail of `AddHit`: a new time stamp (`00435de0` stores the clock
/// `00435dd0` returns) appended to `hits`.
fn record_hit(e: &mut Engine, hits: Ptr<BSSimpleArray>) {
    let now = fn_00435dd0(e);
    e.with_stack(4, |e, stamp| {
        fn_00435de0(e, stamp, now);
        e.call(SIMPLE_ARRAY_APPEND, &args![hits, stamp]);
    });
}

// Translated from 00435d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFriendHits::AddHit` (Xbox PDB): drops the hits that are too old
/// (`RemoveOldHits`), then records a hit at the current time (`00435dd0`)
/// unless the age of the newest remaining hit is below the setting at
/// `011cd78c` (a hit is also recorded when there is none, or when the
/// comparison is unordered).
pub fn extra_friend_hits_add_hit(e: &mut Engine, this: Ptr<ExtraFriendHits>) {
    extra_friend_hits_remove_old_hits(e, this);
    let hits = this.at(ExtraFriendHits::Hits);
    let count = array_size(e, hits) as i32;
    if count > 0 {
        let slot = array_slot(e, hits, (count - 1) as u32);
        let age = stamp_age(e, slot);
        let setting = setting_value(e, HIT_SETTING_ADD);
        // FCOMP then `TEST AH,0x41`: setting <= age, or unordered.
        if setting as f64 > age as f64 {
            return;
        }
    }
    record_hit(e, hits);
}

// Translated from 00435dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The current time the hits are aged against (the engine map has no name for
/// it): returns the `float` at `011f1bf0` in ST0.
pub fn fn_00435dd0(e: &mut Engine) -> f32 {
    e.global::<f32>(HIT_CLOCK)
}

// Translated from 00435de0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatTimeStamp`'s constructor (the engine map has no name for it):
/// stores `value` in the one-float time stamp at `this`. Returns `this`.
pub fn fn_00435de0(e: &mut Engine, this: Ptr, value: f32) -> Ptr {
    e.mem.set_u32(this.addr(), value.to_bits());
    this
}

// Translated from 00435e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The age of the time stamp at `this` (the engine map has no name for it):
/// the current time (`00435dd0`) minus the stamp, rounded to `float`, in ST0.
pub fn fn_00435e00(e: &mut Engine, this: Ptr) -> f32 {
    let now = fn_00435dd0(e);
    let stamp = f32::from_bits(e.mem.u32(this.addr()));
    (now as f64 - stamp as f64) as f32
}

// Translated from 00435e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFriendHits::GetHitCount` (Xbox PDB): drops the old hits, then
/// returns how many remain.
pub fn extra_friend_hits_get_hit_count(e: &mut Engine, this: Ptr<ExtraFriendHits>) -> u32 {
    extra_friend_hits_remove_old_hits(e, this);
    let hits = this.at(ExtraFriendHits::Hits);
    array_size(e, hits)
}

// Translated from 00435e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraFriendHits::RemoveOldHits` (Xbox PDB): removes every hit whose age
/// (`00435e00`) is greater than the setting at `011cd580` (the comparison is
/// ordered: an unordered result keeps the hit). The count is read again on
/// every pass; after a removal the same index is looked at again.
pub fn extra_friend_hits_remove_old_hits(e: &mut Engine, this: Ptr<ExtraFriendHits>) {
    let hits = this.at(ExtraFriendHits::Hits);
    let mut index: u32 = 0;
    while index < array_size(e, hits) {
        let slot = array_slot(e, hits, index);
        let age = stamp_age(e, slot);
        let setting = setting_value(e, HIT_SETTING_REMOVE);
        if (setting as f64) < (age as f64) {
            e.call(SIMPLE_ARRAY_REMOVE_AT, &args![hits, index, 1u32]);
            index = index.wrapping_sub(1);
        }
        index = index.wrapping_add(1);
    }
}

// Translated from 00435ec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraHeadingTarget`'s constructor (the engine map has no name for it):
/// extra-data type 0x46, storing `target`. Returns `this`.
pub fn fn_00435ec0(
    e: &mut Engine,
    this: Ptr<ExtraHeadingTarget>,
    target: Ptr,
) -> Ptr<ExtraHeadingTarget> {
    construct_base(
        e,
        this.cast(),
        TYPE_HEADING_TARGET,
        EXTRA_HEADING_TARGET_VTABLE,
    );
    e.set(this, ExtraHeadingTarget::pheadingtarget, target);
    this
}

// Translated from 00435ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraHeadingTarget`'s destructor body (the engine map has no name for
/// it): resets the vtable, clears `pheadingtarget`, then runs the base
/// destructor.
pub fn fn_00435ef0(e: &mut Engine, this: Ptr<ExtraHeadingTarget>) {
    e.mem.set_u32(this.addr(), EXTRA_HEADING_TARGET_VTABLE);
    e.set(this, ExtraHeadingTarget::pheadingtarget, Ptr::NULL);
    e.call(BS_EXTRA_DATA_DESTRUCT, &args![this]);
}

// Translated from 00435f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraRefractionProperty`'s constructor (the engine map has no name for
/// it): extra-data type 0x48, storing `power`. Returns `this`.
pub fn fn_00435f20(
    e: &mut Engine,
    this: Ptr<ExtraRefractionProperty>,
    power: f32,
) -> Ptr<ExtraRefractionProperty> {
    construct_base(
        e,
        this.cast(),
        TYPE_REFRACTION_PROPERTY,
        EXTRA_REFRACTION_PROPERTY_VTABLE,
    );
    e.set(this, ExtraRefractionProperty::fRefractionPower, power);
    this
}

// Translated from 00435f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraRefractionProperty`'s scalar deleting destructor (the engine map has
/// no name for it): the destructor (`00435f80`, next session of this file,
/// called by address), then `operator delete` when `flags & 1`. Returns
/// `this`.
pub fn fn_00435f50(
    e: &mut Engine,
    this: Ptr<ExtraRefractionProperty>,
    flags: u32,
) -> Ptr<ExtraRefractionProperty> {
    e.call(EXTRA_REFRACTION_PROPERTY_DESTRUCT, &args![this]);
    delete_when_asked(e, this.cast(), flags);
    this
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004353f0,
            extra_x_target_compare(Ptr<ExtraXTarget>, Ptr) -> bool
        ),
        entry!(
            0x00435440,
            fn_00435440(Ptr<ExtraEmittanceSource>) -> Ptr<ExtraEmittanceSource>
        ),
        entry!(
            0x00435470,
            fn_00435470(Ptr<ExtraEmittanceSource>, u32) -> Ptr<ExtraEmittanceSource>
        ),
        entry!(0x004354a0, fn_004354a0(Ptr<ExtraEmittanceSource>)),
        entry!(
            0x004354c0,
            extra_emittance_source_compare(Ptr<ExtraEmittanceSource>, Ptr) -> bool
        ),
        entry!(
            0x00435510,
            fn_00435510(Ptr<ExtraMultiBoundRef>) -> Ptr<ExtraMultiBoundRef>
        ),
        entry!(
            0x00435540,
            extra_multi_bound_ref_compare(Ptr<ExtraMultiBoundRef>, Ptr) -> bool
        ),
        entry!(
            0x00435590,
            fn_00435590(Ptr<ExtraMultiBoundData>) -> Ptr<ExtraMultiBoundData>
        ),
        entry!(
            0x004355c0,
            extra_multi_bound_data_scalar_deleting_destructor(
                Ptr<ExtraMultiBoundData>,
                u32,
            )
                -> Ptr<ExtraMultiBoundData>
        ),
        entry!(0x004355f0, fn_004355f0(Ptr<ExtraMultiBoundData>)),
        entry!(
            0x00435630,
            extra_multi_bound_data_compare(Ptr<ExtraMultiBoundData>, Ptr) -> bool
        ),
        entry!(
            0x00435690,
            fn_00435690(Ptr<ExtraMultiBound>) -> Ptr<ExtraMultiBound>
        ),
        entry!(
            0x00435710,
            extra_multi_bound_scalar_deleting_destructor(
                Ptr<ExtraMultiBound>,
                u32,
            ) -> Ptr<ExtraMultiBound>
        ),
        entry!(0x00435740, fn_00435740(Ptr<ExtraMultiBound>)),
        entry!(
            0x004357a0,
            fn_004357a0(Ptr<ExtraOcclusionPlane>) -> Ptr<ExtraOcclusionPlane>
        ),
        entry!(
            0x00435820,
            fn_00435820(Ptr<ExtraPortal>) -> Ptr<ExtraPortal>
        ),
        entry!(0x004358a0, fn_004358a0(Ptr<ExtraRoom>) -> Ptr<ExtraRoom>),
        entry!(
            0x00435920,
            fn_00435920(Ptr<ExtraItemDropper>) -> Ptr<ExtraItemDropper>
        ),
        entry!(
            0x00435950,
            fn_00435950(Ptr<ExtraDroppedItemList>) -> Ptr<ExtraDroppedItemList>
        ),
        entry!(
            0x004359c0,
            extra_dropped_item_list_scalar_deleting_destructor(
                Ptr<ExtraDroppedItemList>,
                u32,
            )
                -> Ptr<ExtraDroppedItemList>
        ),
        entry!(0x004359f0, fn_004359f0(Ptr<ExtraDroppedItemList>)),
        entry!(
            0x00435a60,
            fn_00435a60(Ptr<ExtraSavedAnimation>) -> Ptr<ExtraSavedAnimation>
        ),
        entry!(
            0x00435a90,
            extra_saved_animation_scalar_deleting_destructor(
                Ptr<ExtraSavedAnimation>,
                u32,
            )
                -> Ptr<ExtraSavedAnimation>
        ),
        entry!(0x00435b20, fn_00435b20(Ptr)),
        entry!(
            0x00435b50,
            fn_00435b50(Ptr<ExtraSavedHavokData>) -> Ptr<ExtraSavedHavokData>
        ),
        entry!(
            0x00435b80,
            extra_saved_havok_data_scalar_deleting_destructor(
                Ptr<ExtraSavedHavokData>,
                u32,
            )
                -> Ptr<ExtraSavedHavokData>
        ),
        entry!(
            0x00435c10,
            extra_saved_animation_delete_buffer(Ptr<ExtraSavedAnimation>)
        ),
        entry!(
            0x00435c40,
            extra_friend_hits_extra_friend_hits(Ptr<ExtraFriendHits>) -> Ptr<ExtraFriendHits>
        ),
        entry!(
            0x00435cb0,
            extra_friend_hits_scalar_deleting_destructor(
                Ptr<ExtraFriendHits>,
                u32,
            ) -> Ptr<ExtraFriendHits>
        ),
        entry!(0x00435ce0, fn_00435ce0(Ptr<ExtraFriendHits>)),
        entry!(0x00435d40, extra_friend_hits_add_hit(Ptr<ExtraFriendHits>)),
        entry!(0x00435dd0, fn_00435dd0() -> f32),
        entry!(0x00435de0, fn_00435de0(Ptr, f32) -> Ptr),
        entry!(0x00435e00, fn_00435e00(Ptr) -> f32),
        entry!(
            0x00435e20,
            extra_friend_hits_get_hit_count(Ptr<ExtraFriendHits>) -> u32
        ),
        entry!(
            0x00435e40,
            extra_friend_hits_remove_old_hits(Ptr<ExtraFriendHits>)
        ),
        entry!(
            0x00435ec0,
            fn_00435ec0(Ptr<ExtraHeadingTarget>, Ptr) -> Ptr<ExtraHeadingTarget>
        ),
        entry!(0x00435ef0, fn_00435ef0(Ptr<ExtraHeadingTarget>)),
        entry!(
            0x00435f20,
            fn_00435f20(Ptr<ExtraRefractionProperty>, f32) -> Ptr<ExtraRefractionProperty>
        ),
        entry!(
            0x00435f50,
            fn_00435f50(Ptr<ExtraRefractionProperty>, u32) -> Ptr<ExtraRefractionProperty>
        ),
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

    /// Test doubles for the callees every function here shares: the
    /// allocator's `operator delete`, the base constructor (stores the type
    /// byte at +4) and destructor, the dynamic cast (the object itself, so a
    /// null `other` stays null), and no-op stand-ins for the member
    /// constructors and destructors; the pages holding the exe globals the
    /// hit functions read are mapped.
    fn extra_engine() -> Engine {
        let mut e = Engine::new();
        e.register(OPERATOR_DELETE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(BS_EXTRA_DATA_CONSTRUCT, |e, a| {
            e.mem.set_u8(a[0] + 4, a[1] as u8);
            ret(a[0])
        });
        e.register(BS_EXTRA_DATA_DESTRUCT, |_, _| Ret::default());
        e.register(DYNAMIC_CAST, |_, a| ret(a[0]));
        for noop in [
            NI_POINTER_CONSTRUCT,
            NI_POINTER_ASSIGN,
            NI_POINTER_DESTRUCT,
            SIMPLE_LIST_CONSTRUCT,
            SIMPLE_LIST_CLEAR,
            SIMPLE_LIST_DESTRUCT,
            HITS_ARRAY_CONSTRUCT,
            HITS_ARRAY_DESTRUCT,
            EXTRA_SAVED_ANIMATION_DESTRUCT,
            EXTRA_SAVED_HAVOK_DATA_DESTRUCT,
            EXTRA_REFRACTION_PROPERTY_DESTRUCT,
        ] {
            e.register(noop, |_, _| Ret::default());
        }
        for page in [0x011c_d000, 0x011f_1000] {
            e.map(page, 0x1000);
        }
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

    /// A block of `size` bytes whose vtable word and payload word hold
    /// garbage, so a constructor's stores show.
    fn dirty_object(e: &mut Engine, size: u32) -> u32 {
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, 0xdead_0001);
        e.mem.set_u32(object + 0xc, 0xdead_0002);
        object
    }

    fn vtable_of(e: &Engine, object: u32) -> u32 {
        e.mem.u32(object)
    }

    fn extra_type(e: &Engine, object: u32) -> u8 {
        e.mem.u8(object + 4)
    }

    /// A constructor that builds an empty payload (null word at +0xc).
    fn check_null_payload_constructor(addr: u32, extra: u8, vtable: u32) {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        start_log(&mut e);
        assert_eq!(e.call(addr, &args![this]).u32(), this);
        assert_eq!(
            calls(&e, BS_EXTRA_DATA_CONSTRUCT),
            vec![vec![this, extra as u32]]
        );
        assert_eq!(extra_type(&e, this), extra);
        assert_eq!(vtable_of(&e, this), vtable);
        assert_eq!(e.mem.u32(this + 0xc), 0);
    }

    /// A constructor that builds an empty `NiPointer` at +0xc.
    fn check_ni_pointer_constructor(addr: u32, extra: u8, vtable: u32) {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        start_log(&mut e);
        assert_eq!(e.call(addr, &args![this]).u32(), this);
        assert_eq!(extra_type(&e, this), extra);
        assert_eq!(vtable_of(&e, this), vtable);
        assert_eq!(calls(&e, NI_POINTER_CONSTRUCT), vec![vec![this + 0xc, 0]]);
        assert_eq!(calls(&e, NI_POINTER_ASSIGN), vec![vec![this + 0xc, 0]]);
    }

    /// A scalar deleting destructor: runs the destructor (the log shows the
    /// call at `inner_call`) and frees the object only when bit 0 of the
    /// flags is set.
    fn check_scalar_deleting(addr: u32, inner_call: u32) {
        let mut e = extra_engine();
        let this = e.mem.alloc(0x20);
        start_log(&mut e);
        assert_eq!(e.call(addr, &args![this, 0u32]).u32(), this);
        assert_eq!(calls(&e, inner_call), vec![vec![this]]);
        assert!(e.mem.block_size(this).is_some());
        assert!(calls(&e, OPERATOR_DELETE).is_empty());
        assert_eq!(e.call(addr, &args![this, 3u32]).u32(), this);
        assert_eq!(e.mem.block_size(this), None);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![this]]);
    }

    /// A `Compare` of a one-word class: the answers for a null other, an
    /// equal payload word and a different one.
    fn check_word_compare(addr: u32, target_type: u32) {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        let same = dirty_object(&mut e, 0x10);
        let other = dirty_object(&mut e, 0x10);
        e.mem.set_u32(this + 0xc, 0x1234);
        e.mem.set_u32(same + 0xc, 0x1234);
        e.mem.set_u32(other + 0xc, 0x5678);
        start_log(&mut e);
        assert!(e.call(addr, &args![this, 0u32]).bool());
        assert!(!e.call(addr, &args![this, same]).bool());
        assert!(e.call(addr, &args![this, other]).bool());
        let casts = calls(&e, DYNAMIC_CAST);
        assert_eq!(casts.len(), 3);
        assert_eq!(casts[1], vec![same, 0, BS_EXTRA_DATA_TYPE, target_type, 0]);
    }

    #[test]
    fn x_target_compare_asks_only_the_target() {
        check_word_compare(0x0043_53f0, EXTRA_X_TARGET_TYPE);
    }

    #[test]
    fn emittance_source_constructor_has_no_source() {
        check_null_payload_constructor(0x0043_5440, 0x67, EXTRA_EMITTANCE_SOURCE_VTABLE);
    }

    #[test]
    fn emittance_source_scalar_deleting_destructor_frees_only_when_asked() {
        check_scalar_deleting(0x0043_5470, BS_EXTRA_DATA_DESTRUCT);
    }

    #[test]
    fn emittance_source_destructor_resets_the_vtable() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        start_log(&mut e);
        e.call(0x0043_54a0, &args![this]);
        assert_eq!(vtable_of(&e, this), EXTRA_EMITTANCE_SOURCE_VTABLE);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this]]);
        // The source is not owned.
        assert_eq!(e.mem.u32(this + 0xc), 0xdead_0002);
    }

    #[test]
    fn emittance_source_compare_asks_only_the_source() {
        check_word_compare(0x0043_54c0, EXTRA_EMITTANCE_SOURCE_TYPE);
    }

    #[test]
    fn multi_bound_ref_constructor_has_no_reference() {
        check_null_payload_constructor(0x0043_5510, 0x63, EXTRA_MULTI_BOUND_REF_VTABLE);
    }

    #[test]
    fn multi_bound_ref_compare_asks_only_the_reference() {
        check_word_compare(0x0043_5540, EXTRA_MULTI_BOUND_REF_TYPE);
    }

    #[test]
    fn multi_bound_data_constructor_has_no_data() {
        check_null_payload_constructor(0x0043_5590, 0x62, EXTRA_MULTI_BOUND_DATA_VTABLE);
    }

    #[test]
    fn multi_bound_data_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = extra_engine();
        let this = e.mem.alloc(0x10);
        let bound = e.mem.alloc(8);
        e.mem.set_u32(this + 0xc, bound);
        assert_eq!(e.call(0x0043_55c0, &args![this, 0u32]).u32(), this);
        assert!(e.mem.block_size(this).is_some());
        assert_eq!(e.mem.block_size(bound), None);
        let again = e.mem.alloc(0x10);
        let other_bound = e.mem.alloc(8);
        e.mem.set_u32(again + 0xc, other_bound);
        assert_eq!(e.call(0x0043_55c0, &args![again, 1u32]).u32(), again);
        assert_eq!(e.mem.block_size(again), None);
        assert_eq!(e.mem.block_size(other_bound), None);
    }

    #[test]
    fn multi_bound_data_destructor_deletes_the_bound_data() {
        let mut e = extra_engine();
        let this = e.mem.alloc(0x10);
        let bound = e.mem.alloc(8);
        e.mem.set_u32(this + 0xc, bound);
        start_log(&mut e);
        e.call(0x0043_55f0, &args![this]);
        assert_eq!(vtable_of(&e, this), EXTRA_MULTI_BOUND_DATA_VTABLE);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![bound]]);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this]]);
    }

    #[test]
    fn multi_bound_data_compare_defers_to_the_marker_data() {
        let mut e = extra_engine();
        // The marker data compare double answers "differ" when the words do.
        e.register(MULTI_BOUND_MARKER_DATA_COMPARE, |_, a| {
            ret((a[0] != a[1]) as u32)
        });
        let this = dirty_object(&mut e, 0x10);
        let other = dirty_object(&mut e, 0x10);
        e.mem.set_u32(this + 0xc, 0x100);
        e.mem.set_u32(other + 0xc, 0x100);
        start_log(&mut e);
        assert!(e.call(0x0043_5630, &args![this, 0u32]).bool());
        assert!(calls(&e, MULTI_BOUND_MARKER_DATA_COMPARE).is_empty());
        assert!(!e.call(0x0043_5630, &args![this, other]).bool());
        e.mem.set_u32(other + 0xc, 0x200);
        assert!(e.call(0x0043_5630, &args![this, other]).bool());
        assert_eq!(
            calls(&e, MULTI_BOUND_MARKER_DATA_COMPARE),
            vec![vec![0x100, 0x100], vec![0x100, 0x200]]
        );
        let casts = calls(&e, DYNAMIC_CAST);
        assert_eq!(
            casts[1],
            vec![other, 0, BS_EXTRA_DATA_TYPE, EXTRA_MULTI_BOUND_DATA_TYPE, 0]
        );
    }

    #[test]
    fn multi_bound_constructor_builds_an_empty_ni_pointer() {
        check_ni_pointer_constructor(0x0043_5690, 0x61, EXTRA_MULTI_BOUND_VTABLE);
    }

    #[test]
    fn multi_bound_scalar_deleting_destructor_frees_only_when_asked() {
        check_scalar_deleting(0x0043_5710, BS_EXTRA_DATA_DESTRUCT);
    }

    #[test]
    fn multi_bound_destructor_destroys_the_ni_pointer_and_keeps_the_vtable() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        start_log(&mut e);
        e.call(0x0043_5740, &args![this]);
        assert_eq!(calls(&e, NI_POINTER_DESTRUCT), vec![vec![this + 0xc]]);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this]]);
        assert_eq!(vtable_of(&e, this), 0xdead_0001);
    }

    #[test]
    fn occlusion_plane_constructor_builds_an_empty_ni_pointer() {
        check_ni_pointer_constructor(0x0043_57a0, 0x71, EXTRA_OCCLUSION_PLANE_VTABLE);
    }

    #[test]
    fn portal_constructor_builds_an_empty_ni_pointer() {
        check_ni_pointer_constructor(0x0043_5820, 0x78, EXTRA_PORTAL_VTABLE);
    }

    #[test]
    fn room_constructor_builds_an_empty_ni_pointer() {
        check_ni_pointer_constructor(0x0043_58a0, 0x79, EXTRA_ROOM_VTABLE);
    }

    #[test]
    fn item_dropper_constructor_has_no_dropper() {
        check_null_payload_constructor(0x0043_5920, 0x39, EXTRA_ITEM_DROPPER_VTABLE);
    }

    #[test]
    fn dropped_item_list_constructor_builds_the_list() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x14);
        start_log(&mut e);
        assert_eq!(e.call(0x0043_5950, &args![this]).u32(), this);
        assert_eq!(extra_type(&e, this), 0x3a);
        assert_eq!(vtable_of(&e, this), EXTRA_DROPPED_ITEM_LIST_VTABLE);
        assert_eq!(calls(&e, SIMPLE_LIST_CONSTRUCT), vec![vec![this + 0xc]]);
    }

    #[test]
    fn dropped_item_list_scalar_deleting_destructor_frees_only_when_asked() {
        check_scalar_deleting(0x0043_59c0, BS_EXTRA_DATA_DESTRUCT);
    }

    #[test]
    fn dropped_item_list_destructor_clears_then_destroys_the_list() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x14);
        start_log(&mut e);
        e.call(0x0043_59f0, &args![this]);
        assert_eq!(vtable_of(&e, this), EXTRA_DROPPED_ITEM_LIST_VTABLE);
        let order: Vec<u32> = e.call_log.as_ref().unwrap().iter().map(|c| c.0).collect();
        assert_eq!(
            order,
            vec![
                0x0043_59f0,
                SIMPLE_LIST_CLEAR,
                SIMPLE_LIST_DESTRUCT,
                BS_EXTRA_DATA_DESTRUCT
            ]
        );
        assert_eq!(calls(&e, SIMPLE_LIST_CLEAR), vec![vec![this + 0xc]]);
        assert_eq!(calls(&e, SIMPLE_LIST_DESTRUCT), vec![vec![this + 0xc]]);
    }

    #[test]
    fn saved_animation_constructor_has_no_buffer() {
        check_null_payload_constructor(0x0043_5a60, 0x42, EXTRA_SAVED_ANIMATION_VTABLE);
    }

    #[test]
    fn saved_animation_scalar_deleting_destructor_frees_only_when_asked() {
        check_scalar_deleting(0x0043_5a90, EXTRA_SAVED_ANIMATION_DESTRUCT);
    }

    #[test]
    fn pointer_holder_release_deletes_the_buffer_and_clears_the_holder() {
        let mut e = extra_engine();
        let holder = e.mem.alloc(4);
        let buffer = e.mem.alloc(16);
        e.mem.set_u32(holder, buffer);
        start_log(&mut e);
        e.call(0x0043_5b20, &args![holder]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![buffer]]);
        assert_eq!(e.mem.block_size(buffer), None);
        assert_eq!(e.mem.u32(holder), 0);
    }

    #[test]
    fn saved_havok_data_constructor_has_no_buffer() {
        check_null_payload_constructor(0x0043_5b50, 0x3d, EXTRA_SAVED_HAVOK_DATA_VTABLE);
    }

    #[test]
    fn saved_havok_data_scalar_deleting_destructor_frees_only_when_asked() {
        check_scalar_deleting(0x0043_5b80, EXTRA_SAVED_HAVOK_DATA_DESTRUCT);
    }

    #[test]
    fn saved_animation_delete_buffer_frees_the_buffer_but_keeps_the_pointer() {
        let mut e = extra_engine();
        e.register(HOLDER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        let this = e.mem.alloc(0x10);
        let buffer = e.mem.alloc(16);
        e.mem.set_u32(this + 0xc, buffer);
        start_log(&mut e);
        e.call(0x0043_5c10, &args![this]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![buffer]]);
        assert_eq!(e.mem.block_size(buffer), None);
        assert_eq!(e.mem.u32(this + 0xc), buffer);
    }

    #[test]
    fn friend_hits_constructor_builds_the_array() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x1c);
        start_log(&mut e);
        assert_eq!(e.call(0x0043_5c40, &args![this]).u32(), this);
        assert_eq!(extra_type(&e, this), 0x45);
        assert_eq!(vtable_of(&e, this), EXTRA_FRIEND_HITS_VTABLE);
        assert_eq!(calls(&e, HITS_ARRAY_CONSTRUCT), vec![vec![this + 0xc]]);
    }

    #[test]
    fn friend_hits_scalar_deleting_destructor_frees_only_when_asked() {
        check_scalar_deleting(0x0043_5cb0, BS_EXTRA_DATA_DESTRUCT);
    }

    #[test]
    fn friend_hits_destructor_destroys_the_array_and_keeps_the_vtable() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x1c);
        start_log(&mut e);
        e.call(0x0043_5ce0, &args![this]);
        assert_eq!(calls(&e, HITS_ARRAY_DESTRUCT), vec![vec![this + 0xc]]);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this]]);
        assert_eq!(vtable_of(&e, this), 0xdead_0001);
    }

    const ADD_SETTING: u32 = HIT_SETTING_ADD;
    const REMOVE_SETTING: u32 = HIT_SETTING_REMOVE;

    /// The engine for the hit functions: `BSSimpleArray` accessors on the
    /// layout (buffer at +4, count at +8), an element removal that shifts the
    /// tail down, an append, and a float setting accessor that returns the
    /// setting address itself (the test stores the value there).
    fn hits_engine(clock: f32, add_setting: f32, remove_setting: f32) -> Engine {
        let mut e = extra_engine();
        e.register(SIMPLE_ARRAY_SIZE, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(SIMPLE_ARRAY_AT, |e, a| ret(e.mem.u32(a[0] + 4) + 4 * a[1]));
        e.register(SIMPLE_ARRAY_REMOVE_AT, |e, a| {
            let buffer = e.mem.u32(a[0] + 4);
            let count = e.mem.u32(a[0] + 8);
            for i in a[1]..count - 1 {
                let next = e.mem.u32(buffer + 4 * (i + 1));
                e.mem.set_u32(buffer + 4 * i, next);
            }
            e.mem.set_u32(a[0] + 8, count - 1);
            Ret::default()
        });
        e.register(SIMPLE_ARRAY_APPEND, |e, a| {
            let buffer = e.mem.u32(a[0] + 4);
            let count = e.mem.u32(a[0] + 8);
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(buffer + 4 * count, value);
            e.mem.set_u32(a[0] + 8, count + 1);
            ret(count)
        });
        e.register(SETTING_FLOAT_VALUE, |_, a| ret(a[0]));
        e.set_global(HIT_CLOCK, clock);
        e.set_global(ADD_SETTING, add_setting);
        e.set_global(REMOVE_SETTING, remove_setting);
        e
    }

    /// An `ExtraFriendHits` whose array holds `stamps` (room for 8).
    fn friend_hits_with(e: &mut Engine, stamps: &[f32]) -> u32 {
        let this = e.mem.alloc(0x1c);
        let buffer = e.mem.alloc(32);
        for (i, stamp) in stamps.iter().enumerate() {
            e.mem.set_u32(buffer + 4 * i as u32, stamp.to_bits());
        }
        e.mem.set_u32(this + 0x10, buffer);
        e.mem.set_u32(this + 0x14, stamps.len() as u32);
        this
    }

    fn stamps_of(e: &Engine, this: u32) -> Vec<f32> {
        let buffer = e.mem.u32(this + 0x10);
        (0..e.mem.u32(this + 0x14))
            .map(|i| f32::from_bits(e.mem.u32(buffer + 4 * i)))
            .collect()
    }

    #[test]
    fn friend_hits_add_hit_records_the_first_hit() {
        let mut e = hits_engine(100.0, 5.0, 1000.0);
        let this = friend_hits_with(&mut e, &[]);
        e.call(0x0043_5d40, &args![this]);
        assert_eq!(stamps_of(&e, this), vec![100.0]);
    }

    #[test]
    fn friend_hits_add_hit_ignores_a_hit_right_after_the_last() {
        let mut e = hits_engine(100.0, 5.0, 1000.0);
        let this = friend_hits_with(&mut e, &[98.0]);
        e.call(0x0043_5d40, &args![this]);
        assert_eq!(stamps_of(&e, this), vec![98.0]);
    }

    #[test]
    fn friend_hits_add_hit_records_when_the_last_is_old_enough() {
        let mut e = hits_engine(100.0, 5.0, 1000.0);
        let this = friend_hits_with(&mut e, &[90.0, 95.0]);
        // Age exactly equal to the setting counts as old enough.
        e.call(0x0043_5d40, &args![this]);
        assert_eq!(stamps_of(&e, this), vec![90.0, 95.0, 100.0]);
        let again = friend_hits_with(&mut e, &[90.0, 80.0]);
        e.call(0x0043_5d40, &args![again]);
        assert_eq!(stamps_of(&e, again), vec![90.0, 80.0, 100.0]);
    }

    #[test]
    fn friend_hits_add_hit_drops_old_hits_first() {
        let mut e = hits_engine(100.0, 5.0, 20.0);
        let this = friend_hits_with(&mut e, &[10.0, 99.0]);
        e.call(0x0043_5d40, &args![this]);
        assert_eq!(stamps_of(&e, this), vec![99.0]);
        let only_old = friend_hits_with(&mut e, &[10.0]);
        e.call(0x0043_5d40, &args![only_old]);
        assert_eq!(stamps_of(&e, only_old), vec![100.0]);
    }

    #[test]
    fn friend_hits_add_hit_records_when_the_comparison_is_unordered() {
        let mut e = hits_engine(100.0, f32::NAN, 1000.0);
        let this = friend_hits_with(&mut e, &[99.0]);
        e.call(0x0043_5d40, &args![this]);
        assert_eq!(stamps_of(&e, this), vec![99.0, 100.0]);
    }

    #[test]
    fn hit_clock_is_the_global_float() {
        let mut e = hits_engine(12.5, 0.0, 0.0);
        assert_eq!(e.call(0x0043_5dd0, &args![]).f32(), 12.5);
    }

    #[test]
    fn time_stamp_constructor_stores_the_float() {
        let mut e = hits_engine(0.0, 0.0, 0.0);
        let stamp = e.mem.alloc(4);
        assert_eq!(e.call(0x0043_5de0, &args![stamp, 3.25f32]).u32(), stamp);
        assert_eq!(f32::from_bits(e.mem.u32(stamp)), 3.25);
    }

    #[test]
    fn time_stamp_age_is_the_clock_minus_the_stamp_in_float() {
        let mut e = hits_engine(100.0, 0.0, 0.0);
        let stamp = e.mem.alloc(4);
        e.mem.set_u32(stamp, 98.5f32.to_bits());
        assert_eq!(e.call(0x0043_5e00, &args![stamp]).f32(), 1.5);
        // The difference is rounded to float: 2^24 + 1 is not representable.
        e.set_global(HIT_CLOCK, 16_777_216.0f32);
        e.mem.set_u32(stamp, (-1.0f32).to_bits());
        assert_eq!(e.call(0x0043_5e00, &args![stamp]).f32(), 16_777_216.0);
    }

    #[test]
    fn friend_hits_get_hit_count_counts_after_dropping_old_hits() {
        let mut e = hits_engine(100.0, 5.0, 20.0);
        let this = friend_hits_with(&mut e, &[10.0, 85.0, 99.0]);
        assert_eq!(e.call(0x0043_5e20, &args![this]).u32(), 2);
        assert_eq!(stamps_of(&e, this), vec![85.0, 99.0]);
        let empty = friend_hits_with(&mut e, &[]);
        assert_eq!(e.call(0x0043_5e20, &args![empty]).u32(), 0);
    }

    #[test]
    fn friend_hits_remove_old_hits_removes_only_the_older_than_the_setting() {
        let mut e = hits_engine(100.0, 5.0, 20.0);
        // Ages 90, 5, 20, 1, 60: the first, the last and none in between
        // (age 20 equals the setting and stays); consecutive old hits are
        // all removed because the index is looked at again after a removal.
        let this = friend_hits_with(&mut e, &[10.0, 95.0, 80.0, 99.0, 40.0]);
        e.call(0x0043_5e40, &args![this]);
        assert_eq!(stamps_of(&e, this), vec![95.0, 80.0, 99.0]);
        let run = friend_hits_with(&mut e, &[1.0, 2.0, 3.0, 99.0]);
        e.call(0x0043_5e40, &args![run]);
        assert_eq!(stamps_of(&e, run), vec![99.0]);
    }

    #[test]
    fn friend_hits_remove_old_hits_keeps_everything_when_unordered() {
        let mut e = hits_engine(100.0, 5.0, f32::NAN);
        let this = friend_hits_with(&mut e, &[1.0, 2.0]);
        e.call(0x0043_5e40, &args![this]);
        assert_eq!(stamps_of(&e, this), vec![1.0, 2.0]);
    }

    #[test]
    fn heading_target_constructor_stores_the_target() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        start_log(&mut e);
        assert_eq!(e.call(0x0043_5ec0, &args![this, 0x4321u32]).u32(), this);
        assert_eq!(extra_type(&e, this), 0x46);
        assert_eq!(vtable_of(&e, this), EXTRA_HEADING_TARGET_VTABLE);
        assert_eq!(e.mem.u32(this + 0xc), 0x4321);
    }

    #[test]
    fn heading_target_destructor_clears_the_target() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        start_log(&mut e);
        e.call(0x0043_5ef0, &args![this]);
        assert_eq!(vtable_of(&e, this), EXTRA_HEADING_TARGET_VTABLE);
        assert_eq!(e.mem.u32(this + 0xc), 0);
        assert_eq!(calls(&e, BS_EXTRA_DATA_DESTRUCT), vec![vec![this]]);
    }

    #[test]
    fn refraction_property_constructor_stores_the_power() {
        let mut e = extra_engine();
        let this = dirty_object(&mut e, 0x10);
        start_log(&mut e);
        assert_eq!(e.call(0x0043_5f20, &args![this, 0.75f32]).u32(), this);
        assert_eq!(extra_type(&e, this), 0x48);
        assert_eq!(vtable_of(&e, this), EXTRA_REFRACTION_PROPERTY_VTABLE);
        assert_eq!(f32::from_bits(e.mem.u32(this + 0xc)), 0.75);
    }

    #[test]
    fn refraction_property_scalar_deleting_destructor_frees_only_when_asked() {
        check_scalar_deleting(0x0043_5f50, EXTRA_REFRACTION_PROPERTY_DESTRUCT);
    }
}
