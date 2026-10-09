//! `fallout shared/extradatalist.cpp` (Xbox PDB source unit), part 3: its functions from `0041db00` up to
//! (not including) `00421400` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::extradatalist`]; anything public there may be used here.
//!
//! Translated so far, in address order: the first 40 functions of the range,
//! `0041db00` to `0041eb60`: the accessors of the extra data of the enable
//! state parent (type `0x37`) and children (`0x38`), the item dropper
//! (`0x39`), the dropped item list (`0x3A`), the water type (`3`), the
//! type `0x3B` marker, the ash pile reference (`0x89`), the linked
//! reference (`0x51`) and its children (`0x52`), the open/close activate
//! reference (`0x6C`) and the activate reference (`0x53`, up to its flag
//! getter). The second session did the next 40, `0041eba0` to `00420210`:
//! the rest of the activate reference (flag setter, activate text,
//! children `0x54` with their timer), the decal references (`0x57`), the
//! reflected and reflector references (`0x65`, `0x66`), the water light and
//! lit water references (`0x84`, `0x85`), the primitive (`0x6B`), the
//! patrol reference data (`0x6F`), the occlusion plane reference data
//! (`0x76`) and the setter of the portal reference data (`0x77`). It also
//! translated three functions of the range that neither the engine map nor
//! Ghidra's function list know, found by following calls: `0041fcd0`
//! (constructor of the patrol reference data), `0041fdb0` and `00420060`
//! (the destructors of the patrol and the occlusion plane reference data).
//! The next session continues at `004202d0`.
//!
//! The type numbers are `EXTRA_DATA_TYPE` of the Xbox PDB (`0x37`
//! `EXTRA_ENABLESTATEPARENT`, `0x3B` `EXTRA_TELEPORTMARKER`, `0x51`
//! `EXTRA_LINKED_REF`, ...). The functions of the same unit outside this
//! file (`GetExtraData` `00410220`, `AddExtra` `0040ff60`, the two
//! `RemoveExtra` `00410020` and `00410140`, the node constructor `00414010`)
//! are called by address, like every other callee. The compiler's
//! exception-unwinding frames (the `FS:[0]` chains of the functions that
//! allocate) are not translated.

#[allow(unused_imports)]
use super::extradatalist::*;
use super::extradataobjects::{
    ExtraActivateRef, ExtraActivateRefChildren, ExtraAshPileRef, ExtraEnableStateChildren,
    ExtraEnableStateParent, ExtraLinkedRef, ExtraLinkedRefChildren, ExtraRandomTeleportMarker,
    NiPoint3, RefActivateData, RefReflectData,
};
#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleList, BSStringT};

// ---------------------------------------------------------------------------
// Layouts

layout! {
    /// `ExtraItemDropper` (Xbox PDB), type `0x39`, 0x10 bytes.
    pub struct ExtraItemDropper: 0x10 {
        /// `pDropper` (Xbox PDB): `TESObjectREFR*`.
        0x0C pDropper: Ptr,
    }

    /// `ExtraDroppedItemList` (Xbox PDB), type `0x3A`, 0x14 bytes.
    pub struct ExtraDroppedItemList: 0x14 {
        /// `DroppedItemList` (Xbox PDB): `BSSimpleList<TESObjectREFR *>`.
        0x0C DroppedItemList: Inline<BSSimpleList>,
    }

    /// `ExtraCellWaterType` (Xbox PDB), type 3, 0x10 bytes.
    pub struct ExtraCellWaterType: 0x10 {
        /// `pWater` (Xbox PDB): `TESWaterForm*`.
        0x0C pWater: Ptr,
    }

    /// `ExtraOpenCloseActivateRef` (Xbox PDB), type `0x6C`, 0x10 bytes.
    pub struct ExtraOpenCloseActivateRef: 0x10 {
        /// `pActivateRef` (Xbox PDB): `TESObjectREFR*`.
        0x0C pActivateRef: Ptr,
    }

    /// `ExtraPrimitive` (Xbox PDB), type `0x6B`, 0x10 bytes: owns its
    /// primitive.
    pub struct ExtraPrimitive: 0x10 {
        /// `pPrimitive` (Xbox PDB): `BGSPrimitive*`.
        0x0C pPrimitive: Ptr,
    }

    /// `ExtraPatrolRefData` (Xbox PDB), type `0x6F`, 0x10 bytes: owns its
    /// patrol data.
    pub struct ExtraPatrolRefData: 0x10 {
        /// `pPatrolData` (Xbox PDB): `PatrolRefData*`.
        0x0C pPatrolData: Ptr,
    }

    /// `ExtraOcclusionPlaneRefData` (Xbox PDB), type `0x76`, 0x10 bytes:
    /// owns its data, four words (the references of the plane).
    pub struct ExtraOcclusionPlaneRefData: 0x10 {
        /// `pData` (Xbox PDB): `OcclusionPlaneLinkedRefData*`, 0x10 bytes.
        0x0C pData: Ptr,
    }
}

// ---------------------------------------------------------------------------
// Constants

/// `ExtraEnableStateParent` (`EXTRA_ENABLESTATEPARENT`).
const EXTRA_ENABLE_STATE_PARENT: u8 = 0x37;
/// `ExtraEnableStateChildren` (`EXTRA_ENABLESTATECHILDREN`).
const EXTRA_ENABLE_STATE_CHILDREN: u8 = 0x38;
/// `ExtraItemDropper` (`EXTRA_ITEMDROPPER`).
const EXTRA_ITEM_DROPPER_TYPE: u8 = 0x39;
/// `ExtraDroppedItemList` (`EXTRA_DROPPEDITEMLIST`).
const EXTRA_DROPPED_ITEM_LIST: u8 = 0x3a;
/// `ExtraCellWaterType` (`EXTRA_WATERTYPE`).
const EXTRA_WATER_TYPE: u8 = 0x03;
/// `ExtraRandomTeleportMarker` (`EXTRA_TELEPORTMARKER`).
const EXTRA_TELEPORT_MARKER: u8 = 0x3b;
/// `ExtraAshPileRef` (`EXTRA_ASHPILE_REF`).
const EXTRA_ASH_PILE_REF: u8 = 0x89;
/// `ExtraLinkedRef` (`EXTRA_LINKED_REF`).
const EXTRA_LINKED_REF_TYPE: u8 = 0x51;
/// `ExtraLinkedRefChildren` (`EXTRA_LINKED_REF_CHILDREN`).
const EXTRA_LINKED_REF_CHILDREN_TYPE: u8 = 0x52;
/// `ExtraActivateRef` (`EXTRA_ACTIVATE_REF`).
const EXTRA_ACTIVATE_REF_TYPE: u8 = 0x53;
/// `ExtraOpenCloseActivateRef` (`EXTRA_OPENCLOSEACTIVATE_REF`).
const EXTRA_OPEN_CLOSE_ACTIVATE_REF_TYPE: u8 = 0x6c;

/// `operator new(size)`.
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete(block)` (cdecl).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `BaseExtraList::GetExtraData(type)` (`00410220`).
const GET_EXTRA_DATA: u32 = 0x0041_0220;
/// `BaseExtraList::AddExtra(extra)` (`0040ff60`).
const ADD_EXTRA: u32 = 0x0040_ff60;
/// `BaseExtraList::RemoveExtra(extra, destroy)` (`00410020`).
const REMOVE_EXTRA: u32 = 0x0041_0020;
/// `BaseExtraList::RemoveExtra_ov2(type)` (`00410140`).
const REMOVE_EXTRA_BY_TYPE: u32 = 0x0041_0140;
/// `BSExtraData::BSExtraData(type)` (`0040ec80`): base vtable, type, null next.
const BS_EXTRA_DATA_INIT: u32 = 0x0040_ec80;
/// `00414010`: constructor of a two-word list node (item and next null; the
/// engine map files it as `BSSimpleList<REF_ACTIVATE_DATA_P>::AddHead`).
const LIST_NODE_INIT: u32 = 0x0041_4010;
/// `BSSimpleList::AddHead(&item)` (`005ae3d0`); `this` is the head node.
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
/// Whether the list at `this` holds an item equal to the word at the address
/// given (`005f65d0`).
const LIST_CONTAINS: u32 = 0x005f_65d0;
/// Removes from the list at `this` the first node whose item equals the word
/// at the address given (`00905330`).
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
/// `BSSimpleList::IsEmpty`: no item and no next (`008256d0`).
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// `MOV EAX,ECX`: the address of a list node is the address of its item slot
/// (`006815c0`).
const LIST_ITEM_SLOT: u32 = 0x0068_15c0;
/// `MOV EAX,[ECX+4]`: the next node of a list node (`00726070`).
const LIST_NEXT: u32 = 0x0072_6070;
/// `BGSSaveFormBuffer::GetForm` (`007af430`, `MOV EAX,[ECX+0x20]`): on a
/// reference, its base form.
const REFERENCE_BASE_FORM: u32 = 0x007a_f430;
/// `TESForm::GetFormType` (`00401170`, `MOVZX EAX,[ECX+4]`).
const FORM_TYPE: u32 = 0x0040_1170;
/// Form type the dropped item search looks for.
const FORM_TYPE_DROPPED_ITEM_MATCH: u32 = 0x28;
/// Search in an `ExtraActivateRef` (`00433a00`): the `REF_ACTIVATE_DATA` of
/// its parent list whose reference is the argument, or null.
const ACTIVATE_REF_FIND: u32 = 0x0043_3a00;

/// Constructors of the extra data (`this` = the new block, no other
/// argument), in `extradataobjects.cpp` and `extradatacell.cpp`.
const ENABLE_STATE_CHILDREN_INIT: u32 = 0x0043_33a0;
const ITEM_DROPPER_INIT: u32 = 0x0043_5920;
const DROPPED_ITEM_LIST_INIT: u32 = 0x0043_5950;
const WATER_TYPE_INIT: u32 = 0x0040_f370;
const TELEPORT_MARKER_INIT: u32 = 0x0043_34b0;
const ASH_PILE_REF_INIT: u32 = 0x0043_36c0;
const LINKED_REF_INIT: u32 = 0x0043_3640;
const LINKED_REF_CHILDREN_INIT: u32 = 0x0043_3530;
const ACTIVATE_REF_INIT: u32 = 0x0043_38b0;

/// Vtable set by the constructor `0041e750`.
const VTABLE_EXTRA_OPEN_CLOSE_ACTIVATE_REF: u32 = 0x0101_51a8;

/// `ExtraActivateRefChildren` (`EXTRA_ACTIVATE_REF_CHILDREN`).
const EXTRA_ACTIVATE_REF_CHILDREN_TYPE: u8 = 0x54;
/// `ExtraDecalRefs` (`EXTRA_DECAL_REFS`).
const EXTRA_DECAL_REFS_TYPE: u8 = 0x57;
/// `ExtraReflectedRefs` (`EXTRA_REFLECTED_REFS`).
const EXTRA_REFLECTED_REFS_TYPE: u8 = 0x65;
/// `ExtraReflectorRefs` (`EXTRA_REFLECTOR_REFS`).
const EXTRA_REFLECTOR_REFS_TYPE: u8 = 0x66;
/// `ExtraPrimitive` (`EXTRA_PRIMITIVE`).
const EXTRA_PRIMITIVE_TYPE: u8 = 0x6b;
/// `ExtraPatrolRefData` (`EXTRA_PATROL_REF_DATA`).
const EXTRA_PATROL_REF_DATA_TYPE: u8 = 0x6f;
/// `ExtraOcclusionPlaneRefData` (`EXTRA_OCCLUSION_PLANE_REF_DATA`).
const EXTRA_OCCLUSION_PLANE_REF_DATA_TYPE: u8 = 0x76;
/// `ExtraPortalRefData` (`EXTRA_PORTAL_REF_DATA`).
const EXTRA_PORTAL_REF_DATA_TYPE: u8 = 0x77;
/// `ExtraWaterLightRefs` (`EXTRA_WATER_LIGHT_REFS`).
const EXTRA_WATER_LIGHT_REFS_TYPE: u8 = 0x84;
/// `ExtraLitWaterRefs` (`EXTRA_LIT_WATER_REFS`).
const EXTRA_LIT_WATER_REFS_TYPE: u8 = 0x85;

/// Constructors of the extra data and of the data they own (`this` = the new
/// block): in `extradataobjects.cpp`, `extradatalist.cpp` and (the portal
/// reference data, the next function of this unit to translate) this part.
const ACTIVATE_REF_CHILDREN_INIT: u32 = 0x0043_3790;
const DECAL_REFS_INIT: u32 = 0x0043_3ca0;
const REFLECTED_REFS_INIT: u32 = 0x0041_1b40;
const REFLECTOR_REFS_INIT: u32 = 0x0041_1be0;
const WATER_LIGHT_REFS_INIT: u32 = 0x0041_1c80;
const LIT_WATER_REFS_INIT: u32 = 0x0041_1d20;
/// Constructor of the four-word `OcclusionPlaneLinkedRefData` (`00411dc0`).
const OCCLUSION_PLANE_DATA_INIT: u32 = 0x0041_1dc0;
const PORTAL_REF_DATA_INIT: u32 = 0x0042_02d0;

/// `ExtraDecalRefs` method (`00434480`) that adds, or updates, the decal
/// of a reference: `(this, reference, intersect*, normal*)`.
const DECAL_REFS_ADD: u32 = 0x0043_4480;
/// Method of `ExtraReflectedRefs` and `ExtraReflectorRefs` (`00434800`)
/// that removes the entry of a reference: `(this, reference)`.
const REFLECT_REFS_REMOVE: u32 = 0x0043_4800;
/// Method of `ExtraReflectedRefs` and `ExtraReflectorRefs` (`004348a0`)
/// that sets the effect flags of the entry of a reference, adding the entry
/// when there is none: `(this, reference, flags)`.
const REFLECT_REFS_SET_FLAGS: u32 = 0x0043_48a0;
/// Stores its argument at `this + 0x0C` (`0041fd00`, `RET 4`; the engine
/// map files it as `NonActorMagicCaster::SetCurrentSpell`).
const STORE_WORD_AT_0C: u32 = 0x0041_fd00;
/// `PatrolRefData::Compare(other)` (Xbox PDB, `0067c720`): true when the two
/// differ.
const PATROL_DATA_COMPARE: u32 = 0x0067_c720;
/// Destructor body of `PatrolRefData` (`0067c670`).
const PATROL_DATA_DESTROY: u32 = 0x0067_c670;

/// `BSStringT<char>` copy constructor (`004047f0`): `this` = the new string,
/// the argument the string copied.
const STRING_COPY_CONSTRUCT: u32 = 0x0040_47f0;
/// `BSStringT<char>` constructor from characters (`0040c0e0`): `this` = the
/// new string, the argument the address of a C string.
const STRING_CONSTRUCT_FROM_CHARS: u32 = 0x0040_c0e0;
/// `BSStringT<char>` assignment from characters (`004037f0`): `this` = the
/// string, then the address of a C string and a second word (0 here).
const STRING_SET: u32 = 0x0040_37f0;
/// An empty C string in `.rdata`.
const EMPTY_TEXT: u32 = 0x0101_1584;
/// The float (-1.0) `GetActivateChildrenTimer` gives when there is no such
/// extra data.
const NO_TIMER: u32 = 0x0101_2054;

/// Vtables set by the constructors `0041faf0`, `0041fcd0` and `0041ff80`.
const VTABLE_EXTRA_PRIMITIVE: u32 = 0x0101_51b4;
const VTABLE_EXTRA_PATROL_REF_DATA: u32 = 0x0101_51c0;
const VTABLE_EXTRA_OCCLUSION_PLANE_REF_DATA: u32 = 0x0101_51cc;

// ---------------------------------------------------------------------------
// Helpers

/// `list->GetExtraData(extra_type)`.
fn find_extra(e: &mut Engine, list: Ptr<ExtraDataList>, extra_type: u8) -> Ptr<BSExtraData> {
    e.call(GET_EXTRA_DATA, &args![list, extra_type as u32])
        .ptr()
}

/// `list->AddExtra(extra)`.
fn add_extra(e: &mut Engine, list: Ptr<ExtraDataList>, extra: Ptr<BSExtraData>) {
    e.call(ADD_EXTRA, &args![list, extra]);
}

/// `list->RemoveExtra(extra, true)`: unlinks and deletes the extra data.
fn remove_extra(e: &mut Engine, list: Ptr<ExtraDataList>, extra: Ptr<BSExtraData>) {
    e.call(REMOVE_EXTRA, &args![list, extra, 1u32]);
}

/// `list->RemoveExtra(extra_type)`.
fn remove_extra_by_type(e: &mut Engine, list: Ptr<ExtraDataList>, extra_type: u8) {
    e.call(REMOVE_EXTRA_BY_TYPE, &args![list, extra_type as u32]);
}

/// `new T`: a block of `size` bytes built by the constructor at `construct`
/// (`this` = the block). As in the code, a failed allocation gives null and
/// the constructor is not run.
fn new_object(e: &mut Engine, size: u32, construct: u32) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        0
    } else {
        e.call(construct, &args![block]).u32()
    }
}

/// The word at +0x0C of the first extra data of `extra_type`, or 0.
fn extra_word_or_zero(e: &mut Engine, list: Ptr<ExtraDataList>, extra_type: u8) -> u32 {
    let extra = find_extra(e, list, extra_type);
    if extra.is_null() {
        0
    } else {
        e.mem.u32(extra.addr() + 0x0c)
    }
}

/// The address of the list at +0x0C of the first extra data of `extra_type`,
/// or null.
fn extra_list_or_null(
    e: &mut Engine,
    list: Ptr<ExtraDataList>,
    extra_type: u8,
) -> Ptr<BSSimpleList> {
    let extra = find_extra(e, list, extra_type);
    if extra.is_null() {
        Ptr::NULL
    } else {
        extra.byte_add(0x0c).cast()
    }
}

/// The shape of the setters of a reference (water, teleport marker, linked
/// reference): a null `value` deletes the extra data of `extra_type` (by
/// type); otherwise the word at +0x0C of the existing one is overwritten, or
/// a new `0x10`-byte one is built by `construct`, given the value, and
/// added.
fn set_word_or_remove_by_type(
    e: &mut Engine,
    list: Ptr<ExtraDataList>,
    extra_type: u8,
    construct: u32,
    value: u32,
) {
    if value == 0 {
        remove_extra_by_type(e, list, extra_type);
        return;
    }
    let extra = find_extra(e, list, extra_type);
    if extra.is_null() {
        let block = new_object(e, 0x10, construct);
        e.mem.set_u32(block + 0x0c, value);
        add_extra(e, list, Ptr::new(block));
    } else {
        e.mem.set_u32(extra.addr() + 0x0c, value);
    }
}

/// The shape of the adders to a reference list: when `item` is not null,
/// finds the extra data of `extra_type` or builds a `0x14`-byte one with
/// `construct` and adds it, then puts `item` at the head of the list at
/// +0x0C unless the list already holds it.
fn add_to_reference_list(
    e: &mut Engine,
    list: Ptr<ExtraDataList>,
    extra_type: u8,
    construct: u32,
    item: u32,
) {
    if item == 0 {
        return;
    }
    let mut extra = find_extra(e, list, extra_type);
    if extra.is_null() {
        extra = Ptr::new(new_object(e, 0x14, construct));
        add_extra(e, list, extra);
    }
    let head = extra.addr() + 0x0c;
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        if !e.call(LIST_CONTAINS, &args![head, slot]).bool() {
            e.call(LIST_ADD_HEAD, &args![head, slot]);
        }
    });
}

/// The shape of the removers from a reference list: when `item` is not
/// null and the list has an extra data of `extra_type`, removes `item` from
/// the list at +0x0C, and deletes the extra data if that left the list empty.
fn remove_from_reference_list(e: &mut Engine, list: Ptr<ExtraDataList>, extra_type: u8, item: u32) {
    if item == 0 {
        return;
    }
    let extra = find_extra(e, list, extra_type);
    if extra.is_null() {
        return;
    }
    let head = extra.addr() + 0x0c;
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        e.call(LIST_REMOVE_ITEM, &args![head, slot]);
    });
    if e.call(LIST_IS_EMPTY, &args![head]).bool() {
        remove_extra(e, list, extra);
    }
}

/// A `float` loaded and stored through the x87 stack (`FLD`/`FSTP`): the
/// same bits, except that a signalling NaN comes out quiet.
fn x87_float(value: f32) -> f32 {
    let bits = value.to_bits();
    let is_nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
    f32::from_bits(if is_nan { bits | 0x0040_0000 } else { bits })
}

/// A new `REF_ACTIVATE_DATA` (8 bytes, node constructor `00414010`): its
/// address, or 0 when the allocation fails (then the code stores through the
/// null pointer all the same; the model has no failing allocation).
fn new_activate_data(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
    if block == 0 {
        0
    } else {
        e.call(LIST_NODE_INIT, &args![block]).u32()
    }
}

/// Stores `reference` in `entry` (`pActivateRef`), then puts the entry at the
/// head of the `ParentList` of `extra` (`AddHead` is given the address of a
/// local holding the entry).
fn push_activate_parent(e: &mut Engine, extra: Ptr<BSExtraData>, entry: u32, reference: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), entry);
        e.mem.set_u32(entry, reference);
        e.call(LIST_ADD_HEAD, &args![extra.addr() + 0x0c, slot]);
    });
}

// Translated from 0041db00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Flag bit 0 of `cFlags` of the type `0x37` extra data
/// (`ExtraEnableStateParent`, `0041db30`), false when the list has none. No
/// Xbox PDB name.
pub fn fn_0041db00(e: &mut Engine, this: Ptr<ExtraDataList>) -> bool {
    let extra = find_extra(e, this, EXTRA_ENABLE_STATE_PARENT);
    if extra.is_null() {
        false
    } else {
        fn_0041db30(e, extra.cast())
    }
}

// Translated from 0041db30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 0 of `cFlags` (+0x10) of an `ExtraEnableStateParent`.
pub fn fn_0041db30(e: &mut Engine, this: Ptr<ExtraEnableStateParent>) -> bool {
    e.get(this, ExtraEnableStateParent::cFlags) & 1 != 0
}

// Translated from 0041db50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears (`value`) flag bit 0 of the type `0x37` extra data
/// (`0041db80`); does nothing when the list has none.
pub fn fn_0041db50(e: &mut Engine, this: Ptr<ExtraDataList>, value: bool) {
    let extra = find_extra(e, this, EXTRA_ENABLE_STATE_PARENT);
    if !extra.is_null() {
        fn_0041db80(e, extra.cast(), value);
    }
}

// Translated from 0041db80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`value` true) or clears bit 0 of `cFlags` (+0x10) of an
/// `ExtraEnableStateParent`.
pub fn fn_0041db80(e: &mut Engine, this: Ptr<ExtraEnableStateParent>, value: bool) {
    let flags = e.get(this, ExtraEnableStateParent::cFlags);
    let flags = if value { flags | 1 } else { flags & 0xfe };
    e.set(this, ExtraEnableStateParent::cFlags, flags);
}

// Translated from 0041dbd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::ShouldPopInWhenEnabledByParent` (Xbox PDB): flag bit 1 of
/// `cFlags` of the type `0x37` extra data (`0041dc00`), false when the list
/// has none.
pub fn extra_data_list_should_pop_in_when_enabled_by_parent(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> bool {
    let extra = find_extra(e, this, EXTRA_ENABLE_STATE_PARENT);
    if extra.is_null() {
        false
    } else {
        fn_0041dc00(e, extra.cast())
    }
}

// Translated from 0041dc00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 1 of `cFlags` (+0x10) of an `ExtraEnableStateParent`.
pub fn fn_0041dc00(e: &mut Engine, this: Ptr<ExtraEnableStateParent>) -> bool {
    e.get(this, ExtraEnableStateParent::cFlags) & 2 != 0
}

// Translated from 0041dc20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Flag bit 2 of `cFlags` of the type `0x37` extra data (`0041dc50`), false
/// when the list has none. No Xbox PDB name.
pub fn fn_0041dc20(e: &mut Engine, this: Ptr<ExtraDataList>) -> bool {
    let extra = find_extra(e, this, EXTRA_ENABLE_STATE_PARENT);
    if extra.is_null() {
        false
    } else {
        fn_0041dc50(e, extra.cast())
    }
}

// Translated from 0041dc50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 2 of `cFlags` (+0x10) of an `ExtraEnableStateParent`.
pub fn fn_0041dc50(e: &mut Engine, this: Ptr<ExtraEnableStateParent>) -> bool {
    e.get(this, ExtraEnableStateParent::cFlags) & 4 != 0
}

// Translated from 0041dc70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `flags` in the whole `cFlags` byte of the type `0x37` extra data;
/// does nothing when the list has none. No Xbox PDB name.
pub fn fn_0041dc70(e: &mut Engine, this: Ptr<ExtraDataList>, flags: u8) {
    let extra: Ptr<ExtraEnableStateParent> = find_extra(e, this, EXTRA_ENABLE_STATE_PARENT).cast();
    if !extra.is_null() {
        e.set(extra, ExtraEnableStateParent::cFlags, flags);
    }
}

// Translated from 0041dca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `ChildList` (+0x0C, the inline head node) of the type `0x38` extra
/// data (`ExtraEnableStateChildren`), or null when the list has none.
pub fn fn_0041dca0(e: &mut Engine, this: Ptr<ExtraDataList>) -> Ptr<BSSimpleList> {
    let extra: Ptr<ExtraEnableStateChildren> =
        find_extra(e, this, EXTRA_ENABLE_STATE_CHILDREN).cast();
    if extra.is_null() {
        Ptr::NULL
    } else {
        extra.at(ExtraEnableStateChildren::ChildList)
    }
}

// Translated from 0041dcd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `child` to the `ChildList` of the type `0x38` extra data
/// (`ExtraEnableStateChildren`, `0x14` bytes, `004333a0`), which it builds
/// and adds to the list when there is none, unless the child is already in
/// the list. A null `child` does nothing. No Xbox PDB name.
pub fn fn_0041dcd0(e: &mut Engine, this: Ptr<ExtraDataList>, child: u32) {
    add_to_reference_list(
        e,
        this,
        EXTRA_ENABLE_STATE_CHILDREN,
        ENABLE_STATE_CHILDREN_INIT,
        child,
    );
}

// Translated from 0041dda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `child` from the `ChildList` of the type `0x38` extra data and
/// deletes the extra data when the list is then empty. A null `child`, or no
/// such extra data, does nothing. No Xbox PDB name.
pub fn fn_0041dda0(e: &mut Engine, this: Ptr<ExtraDataList>, child: u32) {
    remove_from_reference_list(e, this, EXTRA_ENABLE_STATE_CHILDREN, child);
}

// Translated from 0041de00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetItemDropper` (Xbox PDB): `pDropper` of the type `0x39`
/// extra data (`ExtraItemDropper`), or 0.
pub fn extra_data_list_get_item_dropper(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    extra_word_or_zero(e, this, EXTRA_ITEM_DROPPER_TYPE)
}

// Translated from 0041de40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::AddDroppedItem` (Xbox PDB; the body sets `pDropper`): a
/// null `dropper` deletes the type `0x39` extra data (by type); otherwise the
/// extra data is found or built (`0x10` bytes, `00435920`) and added, and
/// `dropper` is stored in it (after the add, for a new one).
pub fn extra_data_list_add_dropped_item(e: &mut Engine, this: Ptr<ExtraDataList>, dropper: u32) {
    if dropper == 0 {
        remove_extra_by_type(e, this, EXTRA_ITEM_DROPPER_TYPE);
        return;
    }
    let mut extra = find_extra(e, this, EXTRA_ITEM_DROPPER_TYPE);
    if extra.is_null() {
        extra = Ptr::new(new_object(e, 0x10, ITEM_DROPPER_INIT));
        add_extra(e, this, extra);
    }
    e.set(extra.cast(), ExtraItemDropper::pDropper, Ptr::new(dropper));
}

// Translated from 0041df00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks the dropped item list (`DroppedItemList` of the type `0x3A` extra
/// data) and returns the first reference whose base form (`007af430`) has
/// form type `0x28` (`00401170`). The walk stops with 0 at the first node
/// whose item is null, and ends with 0 when the list has none or no match.
/// No Xbox PDB name.
pub fn fn_0041df00(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    let mut found = 0;
    let extra = find_extra(e, this, EXTRA_DROPPED_ITEM_LIST);
    if extra.is_null() {
        return 0;
    }
    let mut node = extra.addr() + 0x0c;
    while found == 0 && node != 0 {
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let item = e.mem.u32(slot);
        if item == 0 {
            return 0;
        }
        let form = e.call(REFERENCE_BASE_FORM, &args![item]).u32();
        let form_type = e.call(FORM_TYPE, &args![form]).u32();
        if form_type == FORM_TYPE_DROPPED_ITEM_MATCH {
            found = item;
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    found
}

// Translated from 0041df90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetDroppedItemList` (Xbox PDB): the `DroppedItemList`
/// (+0x0C, the inline head node) of the type `0x3A` extra data, or null.
pub fn extra_data_list_get_dropped_item_list(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<BSSimpleList> {
    extra_list_or_null(e, this, EXTRA_DROPPED_ITEM_LIST)
}

// Translated from 0041dfd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::RemoveDroppedItemList` (Xbox PDB): deletes the type `0x3A`
/// extra data (by type) when the list has one.
pub fn extra_data_list_remove_dropped_item_list(e: &mut Engine, this: Ptr<ExtraDataList>) {
    let extra = find_extra(e, this, EXTRA_DROPPED_ITEM_LIST);
    if !extra.is_null() {
        remove_extra_by_type(e, this, EXTRA_DROPPED_ITEM_LIST);
    }
}

// Translated from 0041e000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `item` to the `DroppedItemList` of the type `0x3A` extra data
/// (`0x14` bytes, `00435950`, built and added when there is none), unless the
/// list already holds it. A null `item` does nothing. No Xbox PDB name.
pub fn fn_0041e000(e: &mut Engine, this: Ptr<ExtraDataList>, item: u32) {
    add_to_reference_list(
        e,
        this,
        EXTRA_DROPPED_ITEM_LIST,
        DROPPED_ITEM_LIST_INIT,
        item,
    );
}

// Translated from 0041e0d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::RemoveDroppedItem` (Xbox PDB): removes `item` from the
/// `DroppedItemList` of the type `0x3A` extra data and deletes the extra data
/// when the list is then empty.
pub fn extra_data_list_remove_dropped_item(e: &mut Engine, this: Ptr<ExtraDataList>, item: u32) {
    remove_from_reference_list(e, this, EXTRA_DROPPED_ITEM_LIST, item);
}

// Translated from 0041e130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetWaterType` (Xbox PDB): `pWater` of the type 3 extra data
/// (`ExtraCellWaterType`), or 0.
pub fn extra_data_list_get_water_type(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    extra_word_or_zero(e, this, EXTRA_WATER_TYPE)
}

// Translated from 0041e160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `pWater` of the type 3 extra data: a null `water` deletes it (by
/// type); otherwise the existing one is overwritten, or a `0x10`-byte one
/// (`0040f370`) is built, given the water and added. No Xbox PDB name.
pub fn fn_0041e160(e: &mut Engine, this: Ptr<ExtraDataList>, water: u32) {
    set_word_or_remove_by_type(e, this, EXTRA_WATER_TYPE, WATER_TYPE_INIT, water);
}

// Translated from 0041e220 (decompiled, FalloutNV.exe 1.4.0.525)
/// `pMarker` of the type `0x3B` extra data (`ExtraRandomTeleportMarker`), or
/// 0. No Xbox PDB name.
pub fn fn_0041e220(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    let extra: Ptr<ExtraRandomTeleportMarker> = find_extra(e, this, EXTRA_TELEPORT_MARKER).cast();
    if extra.is_null() {
        0
    } else {
        e.get(extra, ExtraRandomTeleportMarker::pMarker).addr()
    }
}

// Translated from 0041e250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `pMarker` of the type `0x3B` extra data: a null `marker` deletes it
/// (by type); otherwise the existing one is overwritten, or a `0x10`-byte one
/// (`004334b0`) is built, given the marker and added. No Xbox PDB name.
pub fn fn_0041e250(e: &mut Engine, this: Ptr<ExtraDataList>, marker: u32) {
    set_word_or_remove_by_type(e, this, EXTRA_TELEPORT_MARKER, TELEPORT_MARKER_INIT, marker);
}

// Translated from 0041e310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetAshPileRef` (Xbox PDB): `pAshPileRef` of the type
/// `0x89` extra data (`ExtraAshPileRef`), or 0.
pub fn extra_data_list_get_ash_pile_ref(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    let extra: Ptr<ExtraAshPileRef> = find_extra(e, this, EXTRA_ASH_PILE_REF).cast();
    if extra.is_null() {
        0
    } else {
        e.get(extra, ExtraAshPileRef::pAshPileRef).addr()
    }
}

// Translated from 0041e340 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `pAshPileRef` of the type `0x89` extra data. The extra data is looked
/// up first: a null `reference` deletes it (the object itself, if any); a
/// non-null one is stored in the existing extra data, or in a `0x10`-byte one
/// (`004336c0`) built and added first. No Xbox PDB name.
pub fn fn_0041e340(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32) {
    let mut extra = find_extra(e, this, EXTRA_ASH_PILE_REF);
    if reference == 0 {
        if !extra.is_null() {
            remove_extra(e, this, extra);
        }
        return;
    }
    if extra.is_null() {
        extra = Ptr::new(new_object(e, 0x10, ASH_PILE_REF_INIT));
        add_extra(e, this, extra);
    }
    e.set(
        extra.cast(),
        ExtraAshPileRef::pAshPileRef,
        Ptr::new(reference),
    );
}

// Translated from 0041e410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `pLinkedRef` of the type `0x51` extra data (`ExtraLinkedRef`), or 0. No
/// Xbox PDB name.
pub fn fn_0041e410(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    let extra: Ptr<ExtraLinkedRef> = find_extra(e, this, EXTRA_LINKED_REF_TYPE).cast();
    if extra.is_null() {
        0
    } else {
        e.get(extra, ExtraLinkedRef::pLinkedRef).addr()
    }
}

// Translated from 0041e440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `pLinkedRef` of the type `0x51` extra data: a null `reference`
/// deletes it (by type); otherwise the existing one is overwritten, or a
/// `0x10`-byte one (`00433640`) is built, given the reference and added. No
/// Xbox PDB name.
pub fn fn_0041e440(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32) {
    set_word_or_remove_by_type(e, this, EXTRA_LINKED_REF_TYPE, LINKED_REF_INIT, reference);
}

// Translated from 0041e500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetLinkedRefChildren` (Xbox PDB): the `ChildList` (+0x0C,
/// the inline head node) of the type `0x52` extra data
/// (`ExtraLinkedRefChildren`), or null.
pub fn extra_data_list_get_linked_ref_children(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<BSSimpleList> {
    let extra: Ptr<ExtraLinkedRefChildren> =
        find_extra(e, this, EXTRA_LINKED_REF_CHILDREN_TYPE).cast();
    if extra.is_null() {
        Ptr::NULL
    } else {
        extra.at(ExtraLinkedRefChildren::ChildList)
    }
}

// Translated from 0041e530 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `child` to the `ChildList` of the type `0x52` extra data
/// (`0x14` bytes, `00433530`, built and added when there is none), unless the
/// list already holds it. A null `child` does nothing. No Xbox PDB name.
pub fn fn_0041e530(e: &mut Engine, this: Ptr<ExtraDataList>, child: u32) {
    add_to_reference_list(
        e,
        this,
        EXTRA_LINKED_REF_CHILDREN_TYPE,
        LINKED_REF_CHILDREN_INIT,
        child,
    );
}

// Translated from 0041e600 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `child` from the `ChildList` of the type `0x52` extra data and
/// deletes the extra data when the list is then empty. No Xbox PDB name.
pub fn fn_0041e600(e: &mut Engine, this: Ptr<ExtraDataList>, child: u32) {
    remove_from_reference_list(e, this, EXTRA_LINKED_REF_CHILDREN_TYPE, child);
}

// Translated from 0041e660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `pActivateRef` of the type `0x6C` extra data (`ExtraOpenCloseActivateRef`),
/// or 0. No Xbox PDB name.
pub fn fn_0041e660(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    let extra: Ptr<ExtraOpenCloseActivateRef> =
        find_extra(e, this, EXTRA_OPEN_CLOSE_ACTIVATE_REF_TYPE).cast();
    if extra.is_null() {
        0
    } else {
        e.get(extra, ExtraOpenCloseActivateRef::pActivateRef).addr()
    }
}

// Translated from 0041e690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::SetOpenCloseActivateRef` (Xbox PDB): a null `reference`
/// deletes the type `0x6C` extra data (by type); otherwise the existing one
/// is overwritten, or a `0x10`-byte one is built with the constructor of this
/// file (`0041e750`), given the reference and added.
pub fn extra_data_list_set_open_close_activate_ref(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    reference: u32,
) {
    if reference == 0 {
        remove_extra_by_type(e, this, EXTRA_OPEN_CLOSE_ACTIVATE_REF_TYPE);
        return;
    }
    let extra = find_extra(e, this, EXTRA_OPEN_CLOSE_ACTIVATE_REF_TYPE);
    if extra.is_null() {
        let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
        let built = if block == 0 {
            0
        } else {
            fn_0041e750(e, Ptr::new(block)).addr()
        };
        e.mem.set_u32(built + 0x0c, reference);
        add_extra(e, this, Ptr::new(built));
    } else {
        e.mem.set_u32(extra.addr() + 0x0c, reference);
    }
}

// Translated from 0041e750 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraOpenCloseActivateRef` (type `0x6C`): the base
/// constructor (`0040ec80`) with the type, the vtable `010151a8`, and a null
/// `pActivateRef`. Returns `this`.
pub fn fn_0041e750(
    e: &mut Engine,
    this: Ptr<ExtraOpenCloseActivateRef>,
) -> Ptr<ExtraOpenCloseActivateRef> {
    e.call(
        BS_EXTRA_DATA_INIT,
        &args![this, EXTRA_OPEN_CLOSE_ACTIVATE_REF_TYPE as u32],
    );
    e.mem
        .set_u32(this.addr(), VTABLE_EXTRA_OPEN_CLOSE_ACTIVATE_REF);
    e.set(this, ExtraOpenCloseActivateRef::pActivateRef, Ptr::NULL);
    this
}

// Translated from 0041e780 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `ParentList` (+0x0C, the inline head node) of the type `0x53` extra
/// data (`ExtraActivateRef`), or null. No Xbox PDB name.
pub fn fn_0041e780(e: &mut Engine, this: Ptr<ExtraDataList>) -> Ptr<BSSimpleList> {
    let extra: Ptr<ExtraActivateRef> = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE).cast();
    if extra.is_null() {
        Ptr::NULL
    } else {
        extra.at(ExtraActivateRef::ParentList)
    }
}

// Translated from 0041e7b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `REF_ACTIVATE_DATA` of the type `0x53` extra data's parent list whose
/// reference is `reference` (`00433a00`), or 0 when the list has no such
/// extra data or the parent is not in it. No Xbox PDB name.
pub fn fn_0041e7b0(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32) -> u32 {
    let extra = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE);
    if extra.is_null() {
        0
    } else {
        e.call(ACTIVATE_REF_FIND, &args![extra, reference]).u32()
    }
}

// Translated from 0041e7f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `reference` as an activate parent: a new `REF_ACTIVATE_DATA` (8
/// bytes, node constructor `00414010`, `pActivateRef` = `reference`) goes to
/// the head of the `ParentList` of the type `0x53` extra data (`0x20` bytes,
/// `004338b0`, built and added when there is none), unless the existing extra
/// data already has an entry for `reference` (`00433a00`). A null `reference`
/// does nothing. No Xbox PDB name.
pub fn fn_0041e7f0(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32) {
    if reference == 0 {
        return;
    }
    let extra = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE);
    let extra = if extra.is_null() {
        let built = Ptr::new(new_object(e, 0x20, ACTIVATE_REF_INIT));
        add_extra(e, this, built);
        built
    } else {
        if e.call(ACTIVATE_REF_FIND, &args![extra, reference]).u32() != 0 {
            return;
        }
        extra
    };
    let entry = new_activate_data(e);
    push_activate_parent(e, extra, entry, reference);
}

// Translated from 0041e960 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::RemoveActivateParent` (Xbox PDB): finds the entry for
/// `reference` in the `ParentList` of the type `0x53` extra data (`00433a00`),
/// takes it out of the list (`00905330`), deletes it, and deletes the extra
/// data (by type) when the list is then empty. A null `reference`, no such
/// extra data or no entry does nothing.
pub fn extra_data_list_remove_activate_parent(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    reference: u32,
) {
    if reference == 0 {
        return;
    }
    let extra = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE);
    if extra.is_null() {
        return;
    }
    let entry = e.call(ACTIVATE_REF_FIND, &args![extra, reference]).u32();
    if entry == 0 {
        return;
    }
    let head = extra.addr() + 0x0c;
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), entry);
        e.call(LIST_REMOVE_ITEM, &args![head, slot]);
    });
    e.call(OPERATOR_DELETE, &args![entry]);
    if e.call(LIST_IS_EMPTY, &args![head]).bool() {
        remove_extra_by_type(e, this, EXTRA_ACTIVATE_REF_TYPE);
    }
}

// Translated from 0041e9e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `fActivateDelay` (+4) of the `REF_ACTIVATE_DATA` for `reference` in the
/// type `0x53` extra data (`00433a00`), or 0.0 when there is none. Returned
/// in `ST0` (`FLD`, so a signalling NaN comes out quiet). No Xbox PDB name.
pub fn fn_0041e9e0(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32) -> f32 {
    let extra = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE);
    if !extra.is_null() {
        let entry: Ptr<RefActivateData> = e.call(ACTIVATE_REF_FIND, &args![extra, reference]).ptr();
        if !entry.is_null() {
            return x87_float(e.get(entry, RefActivateData::fActivateDelay));
        }
    }
    0.0
}

// Translated from 0041ea30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the activate delay of the entry for `reference` in the type `0x53`
/// extra data: the extra data is built (`0x20` bytes, `004338b0`) and added
/// when there is none; when it has no entry for `reference` (`00433a00`), a
/// new `REF_ACTIVATE_DATA` (8 bytes, `00414010`) goes to the head of the
/// `ParentList` and gets `pActivateRef` = `reference`; then `delay` is
/// stored in the entry (`FLD`/`FSTP`, so a signalling NaN is stored quiet).
/// A null `reference` does nothing. No Xbox PDB name.
pub fn fn_0041ea30(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32, delay: f32) {
    if reference == 0 {
        return;
    }
    let mut extra = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE);
    let mut entry = 0;
    if extra.is_null() {
        extra = Ptr::new(new_object(e, 0x20, ACTIVATE_REF_INIT));
        add_extra(e, this, extra);
    } else {
        entry = e.call(ACTIVATE_REF_FIND, &args![extra, reference]).u32();
    }
    if entry == 0 {
        entry = new_activate_data(e);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), entry);
            e.call(LIST_ADD_HEAD, &args![extra.addr() + 0x0c, slot]);
        });
        e.mem.set_u32(entry, reference);
    }
    e.set(
        Ptr::<RefActivateData>::new(entry),
        RefActivateData::fActivateDelay,
        x87_float(delay),
    );
}

// Translated from 0041eb60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 0 of `cActivateFlags` (+0x14) of the type `0x53` extra data, false
/// when the list has none. No Xbox PDB name.
pub fn fn_0041eb60(e: &mut Engine, this: Ptr<ExtraDataList>) -> bool {
    let extra: Ptr<ExtraActivateRef> = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE).cast();
    !extra.is_null() && e.get(extra, ExtraActivateRef::cActivateFlags) & 1 != 0
}

/// `new T` whose constructor is a Rust function of this file: a block of
/// `size` bytes built by `build` (given the block). As in the code, a failed
/// allocation gives null and the constructor is not run.
fn new_built(e: &mut Engine, size: u32, build: impl FnOnce(&mut Engine, u32) -> u32) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        0
    } else {
        build(e, block)
    }
}

/// The shape of the setters that replace an extra data holding one owned
/// word (patrol reference data, occlusion plane reference data, portal
/// reference data): the extra data of `extra_type` is deleted when there is
/// one; then, when `value` is not null, a `0x10`-byte one is built by
/// `build`, given the value (`0041fd00`) and added.
fn replace_extra_holding_word(
    e: &mut Engine,
    list: Ptr<ExtraDataList>,
    extra_type: u8,
    value: u32,
    build: impl FnOnce(&mut Engine, u32) -> u32,
) {
    let old = find_extra(e, list, extra_type);
    if !old.is_null() {
        remove_extra(e, list, old);
    }
    if value != 0 {
        let built = new_built(e, 0x10, build);
        e.call(STORE_WORD_AT_0C, &args![built, value]);
        add_extra(e, list, Ptr::new(built));
    }
}

/// The shape of the setters of one reflected or reflector reference
/// (`0041f1a0`, `0041f330`, `0041f4c0`, `0041f650`): walks the entries
/// (`REF_REFLECTED_DATA` / `REF_REFLECTOR_DATA`, the list at +0x0C of the
/// extra data of `extra_type`) up to the first one whose reference is
/// `reference`, stopping at an entry that is null. Found: with `set`, `bit`
/// is set in its `iEffectFlags`; otherwise `bit` is cleared, and if no flag
/// is left the entry is removed (`00434800`) and the extra data deleted when
/// its list is then empty. Not found, with `set`: the extra data is built
/// (`0x14` bytes, `construct`) and added if there is none, and the entry is
/// added with `bit` as flags (`004348a0`). Not found, without `set`: nothing.
fn set_reflect_ref_flag(
    e: &mut Engine,
    list: Ptr<ExtraDataList>,
    extra_type: u8,
    construct: u32,
    reference: u32,
    set: bool,
    bit: u32,
) {
    let mut extra = find_extra(e, list, extra_type);
    if !extra.is_null() {
        let mut node = extra.addr() + 0x0c;
        while node != 0 {
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let entry: Ptr<RefReflectData> = Ptr::new(e.mem.u32(slot));
            if entry.is_null() {
                break;
            }
            if e.get(entry, RefReflectData::pRef).addr() == reference {
                let flags = e.get(entry, RefReflectData::iEffectFlags);
                if set {
                    e.set(entry, RefReflectData::iEffectFlags, flags | bit);
                } else {
                    let flags = flags & !bit;
                    e.set(entry, RefReflectData::iEffectFlags, flags);
                    if flags == 0 {
                        e.call(REFLECT_REFS_REMOVE, &args![extra, reference]);
                        if e.call(LIST_IS_EMPTY, &args![extra.addr() + 0x0c]).bool() {
                            remove_extra(e, list, extra);
                        }
                    }
                }
                return;
            }
            node = e.call(LIST_NEXT, &args![node]).u32();
        }
    }
    if set {
        if extra.is_null() {
            extra = Ptr::new(new_object(e, 0x14, construct));
            add_extra(e, list, extra);
        }
        e.call(REFLECT_REFS_SET_FLAGS, &args![extra, reference, bit]);
    }
}

/// The shape of the setters of the water light and lit water references:
/// `reference` is the only item of a one-word local whose address is given
/// to the list calls. With the extra data of `extra_type` present, `add`
/// puts the reference at the head of its list unless the list holds it, and
/// otherwise the reference is removed from the list (`00905330`), then the
/// extra data deleted if `delete_when_empty` and the list is empty. Without
/// it, `add` builds one (`0x14` bytes, `construct`), adds it and puts the
/// reference in its list; otherwise nothing happens.
fn change_reference_in_list(
    e: &mut Engine,
    list: Ptr<ExtraDataList>,
    extra_type: u8,
    construct: u32,
    reference: u32,
    add: bool,
    delete_when_empty: bool,
) {
    let mut extra = find_extra(e, list, extra_type);
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), reference);
        if !extra.is_null() {
            let head = extra.addr() + 0x0c;
            if add {
                if !e.call(LIST_CONTAINS, &args![head, slot]).bool() {
                    e.call(LIST_ADD_HEAD, &args![head, slot]);
                }
            } else {
                e.call(LIST_REMOVE_ITEM, &args![head, slot]);
                if delete_when_empty && e.call(LIST_IS_EMPTY, &args![head]).bool() {
                    remove_extra(e, list, extra);
                }
            }
        } else if add {
            extra = Ptr::new(new_object(e, 0x14, construct));
            add_extra(e, list, extra);
            e.call(LIST_ADD_HEAD, &args![extra.addr() + 0x0c, slot]);
        }
    });
}

// Translated from 0041eba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` true) or clears bit 0 of `cActivateFlags` (+0x14) of the
/// type `0x53` extra data (`ExtraActivateRef`). With `flag` true and no such
/// extra data, one is built (`0x20` bytes, `004338b0`) and added first; with
/// `flag` false and none, nothing happens. No Xbox PDB name. The
/// exception-unwinding frame is not translated.
pub fn fn_0041eba0(e: &mut Engine, this: Ptr<ExtraDataList>, flag: bool) {
    let mut extra: Ptr<ExtraActivateRef> = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE).cast();
    if extra.is_null() && flag {
        extra = Ptr::new(new_object(e, 0x20, ACTIVATE_REF_INIT));
        add_extra(e, this, extra.cast());
    }
    if !extra.is_null() {
        let flags = e.get(extra, ExtraActivateRef::cActivateFlags);
        let flags = if flag { flags | 1 } else { flags & 0xfe };
        e.set(extra, ExtraActivateRef::cActivateFlags, flags);
    }
}

// Translated from 0041ec80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds in `out` (the caller's `BSStringT<char>`, an out parameter) a copy
/// of `ActivateTextOverride` (+0x18) of the type `0x53` extra data
/// (`004047f0`), or an empty string (`0040c0e0` on an empty C string) when
/// the list has none. Returns `out`. No Xbox PDB name.
pub fn fn_0041ec80(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    out: Ptr<BSStringT>,
) -> Ptr<BSStringT> {
    let extra: Ptr<ExtraActivateRef> = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE).cast();
    if extra.is_null() {
        e.call(STRING_CONSTRUCT_FROM_CHARS, &args![out, EMPTY_TEXT]);
    } else {
        let text = extra.at(ExtraActivateRef::ActivateTextOverride);
        e.call(STRING_COPY_CONSTRUCT, &args![out, text]);
    }
    out
}

// Translated from 0041ece0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `ActivateTextOverride` of the type `0x53` extra data from the C
/// string at `text` (`004037f0`, second argument 0). With no such extra data
/// and a non-null `text`, one is built (`0x20` bytes, `004338b0`) and added
/// first; with no extra data and a null `text`, nothing happens. A null
/// `text` on an existing extra data is passed on all the same. No Xbox PDB
/// name. The exception-unwinding frame is not translated.
pub fn fn_0041ece0(e: &mut Engine, this: Ptr<ExtraDataList>, text: u32) {
    let mut extra: Ptr<ExtraActivateRef> = find_extra(e, this, EXTRA_ACTIVATE_REF_TYPE).cast();
    if extra.is_null() && text != 0 {
        extra = Ptr::new(new_object(e, 0x20, ACTIVATE_REF_INIT));
        add_extra(e, this, extra.cast());
    }
    if !extra.is_null() {
        let string = extra.at(ExtraActivateRef::ActivateTextOverride);
        e.call(STRING_SET, &args![string, text, 0u32]);
    }
}

// Translated from 0041eda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetActivateRefChildren` (Xbox PDB): the `ChildList`
/// (+0x0C, the inline head node) of the type `0x54` extra data
/// (`ExtraActivateRefChildren`), or null.
pub fn extra_data_list_get_activate_ref_children(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<BSSimpleList> {
    let extra: Ptr<ExtraActivateRefChildren> =
        find_extra(e, this, EXTRA_ACTIVATE_REF_CHILDREN_TYPE).cast();
    if extra.is_null() {
        Ptr::NULL
    } else {
        extra.at(ExtraActivateRefChildren::ChildList)
    }
}

// Translated from 0041edd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::AddActivateRefChild` (Xbox PDB): adds `child` to the
/// `ChildList` of the type `0x54` extra data (`0x18` bytes, `00433790`,
/// built and added when there is none) as a new `REF_ACTIVATE_DATA` (8
/// bytes, node constructor `00414010`, `pActivateRef` = `child`) at the head,
/// unless an entry of the list already names `child`. The walk stops at the
/// empty node. A null `child` does nothing. The exception-unwinding frame is
/// not translated.
pub fn extra_data_list_add_activate_ref_child(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    child: u32,
) {
    if child == 0 {
        return;
    }
    let mut extra = find_extra(e, this, EXTRA_ACTIVATE_REF_CHILDREN_TYPE);
    if extra.is_null() {
        extra = Ptr::new(new_object(e, 0x18, ACTIVATE_REF_CHILDREN_INIT));
        add_extra(e, this, extra);
    }
    let mut found = false;
    let mut node = extra.addr() + 0x0c;
    while node != 0 {
        if e.call(LIST_IS_EMPTY, &args![node]).bool() || found {
            break;
        }
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let entry: Ptr<RefActivateData> = Ptr::new(e.mem.u32(slot));
        if e.get(entry, RefActivateData::pActivateRef).addr() == child {
            found = true;
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    if !found {
        let entry = new_activate_data(e);
        push_activate_parent(e, extra, entry, child);
    }
}

// Translated from 0041ef20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes the entry for `child` from the `ChildList` of the type `0x54`
/// extra data and deletes it, then deletes the extra data (the object, with
/// `RemoveExtra(extra, true)`) when the list is empty, whether or not an
/// entry was removed. The walk stops at the empty node. As in the exe, the
/// "previous node" the code keeps for the unlink is never set, so the entry
/// is always taken out with the remove-head call (`0063f7b0`) on the node it
/// was found at. A null `child` or no such extra data does nothing. No Xbox
/// PDB name.
pub fn fn_0041ef20(e: &mut Engine, this: Ptr<ExtraDataList>, child: u32) {
    if child == 0 {
        return;
    }
    let extra = find_extra(e, this, EXTRA_ACTIVATE_REF_CHILDREN_TYPE);
    if extra.is_null() {
        return;
    }
    let head = extra.addr() + 0x0c;
    let mut node = head;
    while node != 0 {
        if e.call(LIST_IS_EMPTY, &args![node]).bool() {
            break;
        }
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let entry: Ptr<RefActivateData> = Ptr::new(e.mem.u32(slot));
        if e.get(entry, RefActivateData::pActivateRef).addr() == child {
            e.call(LIST_REMOVE_HEAD, &args![node]);
            e.call(OPERATOR_DELETE, &args![entry]);
            break;
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    if e.call(LIST_IS_EMPTY, &args![head]).bool() {
        remove_extra(e, this, extra);
    }
}

// Translated from 0041eff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::SetActivateChildrenTimer` (Xbox PDB):
/// `fActivateChildrenTimer` (+0x14) of the type `0x54` extra data; does
/// nothing when the list has none. Stored through the x87 stack (`FLD`/`FSTP`,
/// so a signalling NaN is stored quiet).
pub fn extra_data_list_set_activate_children_timer(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    timer: f32,
) {
    let extra: Ptr<ExtraActivateRefChildren> =
        find_extra(e, this, EXTRA_ACTIVATE_REF_CHILDREN_TYPE).cast();
    if !extra.is_null() {
        e.set(
            extra,
            ExtraActivateRefChildren::fActivateChildrenTimer,
            x87_float(timer),
        );
    }
}

// Translated from 0041f020 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetActivateChildrenTimer` (Xbox PDB):
/// `fActivateChildrenTimer` of the type `0x54` extra data, or -1.0 (the
/// float at `01012054`) when the list has none. Returned in `ST0` (`FLD`, so
/// a signalling NaN comes out quiet).
pub fn extra_data_list_get_activate_children_timer(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> f32 {
    let extra: Ptr<ExtraActivateRefChildren> =
        find_extra(e, this, EXTRA_ACTIVATE_REF_CHILDREN_TYPE).cast();
    let timer = if extra.is_null() {
        e.global::<f32>(NO_TIMER)
    } else {
        e.get(extra, ExtraActivateRefChildren::fActivateChildrenTimer)
    };
    x87_float(timer)
}

// Translated from 0041f050 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetDecalRefs` (Xbox PDB): the `DecalRefList` (+0x0C, the
/// inline head node) of the type `0x57` extra data (`ExtraDecalRefs`), or
/// null.
pub fn extra_data_list_get_decal_refs(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<BSSimpleList> {
    extra_list_or_null(e, this, EXTRA_DECAL_REFS_TYPE)
}

// Translated from 0041f080 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a decal for `reference` to the type `0x57` extra data
/// (`ExtraDecalRefs`, `0x14` bytes, `00433ca0`, built and added when there
/// is none): its method `00434480` is given the reference and the addresses
/// of the intersection point and the normal (`NiPoint3`), which it copies
/// into the entry. A null `reference` does nothing. No Xbox PDB name. The
/// exception-unwinding frame is not translated.
pub fn fn_0041f080(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    reference: u32,
    intersect: Ptr<NiPoint3>,
    normal: Ptr<NiPoint3>,
) {
    if reference == 0 {
        return;
    }
    let mut extra = find_extra(e, this, EXTRA_DECAL_REFS_TYPE);
    if extra.is_null() {
        extra = Ptr::new(new_object(e, 0x14, DECAL_REFS_INIT));
        add_extra(e, this, extra);
    }
    e.call(DECAL_REFS_ADD, &args![extra, reference, intersect, normal]);
}

// Translated from 0041f140 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetReflectedRefs` (Xbox PDB): the `RefList` (+0x0C, the
/// inline head node) of the type `0x65` extra data (`ExtraReflectedRefs`),
/// or null.
pub fn extra_data_list_get_reflected_refs(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<BSSimpleList> {
    extra_list_or_null(e, this, EXTRA_REFLECTED_REFS_TYPE)
}

// Translated from 0041f170 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `RefList` (+0x0C, the inline head node) of the type `0x66` extra data
/// (`ExtraReflectorRefs`), or null. No Xbox PDB name.
pub fn fn_0041f170(e: &mut Engine, this: Ptr<ExtraDataList>) -> Ptr<BSSimpleList> {
    extra_list_or_null(e, this, EXTRA_REFLECTOR_REFS_TYPE)
}

// Translated from 0041f1a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` true) or clears flag bit 0 of the entry for `reference` in the
/// type `0x66` extra data (`ExtraReflectorRefs`, constructor `00411be0`);
/// see [`set_reflect_ref_flag`]. No Xbox PDB name. The exception-unwinding
/// frame is not translated.
pub fn fn_0041f1a0(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32, set: bool) {
    set_reflect_ref_flag(
        e,
        this,
        EXTRA_REFLECTOR_REFS_TYPE,
        REFLECTOR_REFS_INIT,
        reference,
        set,
        1,
    );
}

// Translated from 0041f330 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` true) or clears flag bit 1 (value 2) of the entry for
/// `reference` in the type `0x66` extra data (`ExtraReflectorRefs`,
/// constructor `00411be0`); see [`set_reflect_ref_flag`]. No Xbox PDB name.
/// The exception-unwinding frame is not translated.
pub fn fn_0041f330(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32, set: bool) {
    set_reflect_ref_flag(
        e,
        this,
        EXTRA_REFLECTOR_REFS_TYPE,
        REFLECTOR_REFS_INIT,
        reference,
        set,
        2,
    );
}

// Translated from 0041f4c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` true) or clears flag bit 0 of the entry for `reference` in the
/// type `0x65` extra data (`ExtraReflectedRefs`, constructor `00411b40`);
/// see [`set_reflect_ref_flag`]. No Xbox PDB name. The exception-unwinding
/// frame is not translated.
pub fn fn_0041f4c0(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32, set: bool) {
    set_reflect_ref_flag(
        e,
        this,
        EXTRA_REFLECTED_REFS_TYPE,
        REFLECTED_REFS_INIT,
        reference,
        set,
        1,
    );
}

// Translated from 0041f650 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` true) or clears flag bit 1 (value 2) of the entry for
/// `reference` in the type `0x65` extra data (`ExtraReflectedRefs`,
/// constructor `00411b40`); see [`set_reflect_ref_flag`]. No Xbox PDB name.
/// The exception-unwinding frame is not translated.
pub fn fn_0041f650(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32, set: bool) {
    set_reflect_ref_flag(
        e,
        this,
        EXTRA_REFLECTED_REFS_TYPE,
        REFLECTED_REFS_INIT,
        reference,
        set,
        2,
    );
}

// Translated from 0041f7e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetWaterLightRefs` (Xbox PDB): the `RefList` (+0x0C, the
/// inline head node) of the type `0x84` extra data (`ExtraWaterLightRefs`),
/// or null.
pub fn extra_data_list_get_water_light_refs(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<BSSimpleList> {
    extra_list_or_null(e, this, EXTRA_WATER_LIGHT_REFS_TYPE)
}

// Translated from 0041f810 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetLitWaterRefs` (Xbox PDB): the `RefList` (+0x0C, the
/// inline head node) of the type `0x85` extra data (`ExtraLitWaterRefs`), or
/// null.
pub fn extra_data_list_get_lit_water_refs(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<BSSimpleList> {
    extra_list_or_null(e, this, EXTRA_LIT_WATER_REFS_TYPE)
}

// Translated from 0041f840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::SetWaterLightRef` (Xbox PDB): adds (`add` true) or removes
/// `reference` in the `RefList` of the type `0x84` extra data
/// (`ExtraWaterLightRefs`, `0x14` bytes, `00411c80`, built when an add finds
/// none); see [`change_reference_in_list`]. A removal never deletes the
/// extra data. The exception-unwinding frame is not translated.
pub fn extra_data_list_set_water_light_ref(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    reference: u32,
    add: bool,
) {
    change_reference_in_list(
        e,
        this,
        EXTRA_WATER_LIGHT_REFS_TYPE,
        WATER_LIGHT_REFS_INIT,
        reference,
        add,
        false,
    );
}

// Translated from 0041f940 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds (`add` true) or removes `reference` in the `RefList` of the type
/// `0x85` extra data (`ExtraLitWaterRefs`, `0x14` bytes, `00411d20`, built
/// when an add finds none); see [`change_reference_in_list`]. A removal
/// deletes the extra data when its list is then empty. No Xbox PDB name. The
/// exception-unwinding frame is not translated.
pub fn fn_0041f940(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32, add: bool) {
    change_reference_in_list(
        e,
        this,
        EXTRA_LIT_WATER_REFS_TYPE,
        LIT_WATER_REFS_INIT,
        reference,
        add,
        true,
    );
}

// Translated from 0041fa60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::AddPrimitive` (Xbox PDB): adds a new `ExtraPrimitive`
/// (`0x10` bytes, constructor `0041faf0`) owning `primitive`; a null
/// `primitive` does nothing. The list is not checked for an existing one.
/// The exception-unwinding frame is not translated.
pub fn extra_data_list_add_primitive(e: &mut Engine, this: Ptr<ExtraDataList>, primitive: u32) {
    if primitive == 0 {
        return;
    }
    let built = new_built(e, 0x10, |e, block| {
        fn_0041faf0(e, Ptr::new(block), primitive).addr()
    });
    add_extra(e, this, Ptr::new(built));
}

// Translated from 0041faf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraPrimitive` (type `0x6B`): the base constructor
/// (`0040ec80`) with the type, the vtable `010151b4`, and `pPrimitive` =
/// `primitive`. Returns `this`.
pub fn fn_0041faf0(
    e: &mut Engine,
    this: Ptr<ExtraPrimitive>,
    primitive: u32,
) -> Ptr<ExtraPrimitive> {
    e.call(
        BS_EXTRA_DATA_INIT,
        &args![this, EXTRA_PRIMITIVE_TYPE as u32],
    );
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_PRIMITIVE);
    e.set(this, ExtraPrimitive::pPrimitive, Ptr::new(primitive));
    this
}

// Translated from 0041fb20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraPrimitive::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`0041fb50`) and frees the object when bit 0 of `flags` is
/// set. Returns `this`.
pub fn extra_primitive_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraPrimitive>,
    flags: u32,
) -> Ptr<ExtraPrimitive> {
    fn_0041fb50(e, this);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 0041fb50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of `ExtraPrimitive`: sets its vtable, deletes the primitive it
/// owns through its own scalar deleting destructor (slot 0, argument 1) when
/// there is one, then runs the `BSExtraData` destructor (`0040ecb0`). The
/// exception-unwinding frame is not translated.
pub fn fn_0041fb50(e: &mut Engine, this: Ptr<ExtraPrimitive>) {
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_PRIMITIVE);
    let primitive = e.get(this, ExtraPrimitive::pPrimitive);
    if !primitive.is_null() {
        e.vcall(primitive.addr(), 0, &args![1u32]);
    }
    e.call(BS_EXTRA_DATA_DESTROY, &args![this]);
}

// Translated from 0041fbe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetPrimitive` (Xbox PDB): `pPrimitive` of the type `0x6B`
/// extra data (`ExtraPrimitive`), or 0.
pub fn extra_data_list_get_primitive(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    extra_word_or_zero(e, this, EXTRA_PRIMITIVE_TYPE)
}

// Translated from 0041fc10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the patrol reference data: the type `0x6F` extra data
/// (`ExtraPatrolRefData`) is deleted when there is one, then, for a non-null
/// `patrol_data`, a new one (`0x10` bytes, constructor `0041fcd0`) is given
/// it (`0041fd00`) and added. No Xbox PDB name. The exception-unwinding frame
/// is not translated.
pub fn fn_0041fc10(e: &mut Engine, this: Ptr<ExtraDataList>, patrol_data: u32) {
    replace_extra_holding_word(
        e,
        this,
        EXTRA_PATROL_REF_DATA_TYPE,
        patrol_data,
        |e, block| fn_0041fcd0(e, Ptr::new(block)).addr(),
    );
}

// Translated from 0041fcd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraPatrolRefData` (type `0x6F`): the base constructor
/// (`0040ec80`) with the type and the vtable `010151c0`; `pPatrolData` is
/// left as it is (the setter `0041fd00` fills it). Neither the engine map nor
/// Ghidra's function list has this function (the map files it under no unit);
/// it is the callee of `0041fc10`. Returns `this`.
pub fn fn_0041fcd0(e: &mut Engine, this: Ptr<ExtraPatrolRefData>) -> Ptr<ExtraPatrolRefData> {
    e.call(
        BS_EXTRA_DATA_INIT,
        &args![this, EXTRA_PATROL_REF_DATA_TYPE as u32],
    );
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_PATROL_REF_DATA);
    this
}

// Translated from 0041fd20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraPatrolRefData::Compare` (Xbox PDB): true when the two differ. Two
/// extra data without patrol data are equal; one without differs; otherwise
/// `PatrolRefData::Compare` (`0067c720`) decides.
pub fn extra_patrol_ref_data_compare(
    e: &mut Engine,
    this: Ptr<ExtraPatrolRefData>,
    other: Ptr<ExtraPatrolRefData>,
) -> bool {
    let mine = e.get(this, ExtraPatrolRefData::pPatrolData);
    let theirs = e.get(other, ExtraPatrolRefData::pPatrolData);
    if mine.is_null() && theirs.is_null() {
        return false;
    }
    if mine.is_null() || theirs.is_null() {
        return true;
    }
    e.call(PATROL_DATA_COMPARE, &args![mine, theirs]).bool()
}

// Translated from 0041fd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraPatrolRefData::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`0041fdb0`) and frees the object when bit 0 of `flags` is
/// set. Returns `this`.
pub fn extra_patrol_ref_data_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraPatrolRefData>,
    flags: u32,
) -> Ptr<ExtraPatrolRefData> {
    fn_0041fdb0(e, this);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 0041fdb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of `ExtraPatrolRefData` (called by `0041fd80`; not in the
/// engine map nor in Ghidra's function list): sets its vtable, deletes the
/// patrol data (`0041fe10`), then runs the `BSExtraData` destructor
/// (`0040ecb0`). The exception-unwinding frame is not translated.
pub fn fn_0041fdb0(e: &mut Engine, this: Ptr<ExtraPatrolRefData>) {
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_PATROL_REF_DATA);
    fn_0041fe10(e, this);
    e.call(BS_EXTRA_DATA_DESTROY, &args![this]);
}

// Translated from 0041fe10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the patrol data of an `ExtraPatrolRefData` through its scalar
/// deleting destructor (`0041fe60`, argument 1) when there is one, and sets
/// `pPatrolData` to null. No Xbox PDB name.
pub fn fn_0041fe10(e: &mut Engine, this: Ptr<ExtraPatrolRefData>) {
    let patrol_data = e.get(this, ExtraPatrolRefData::pPatrolData);
    if !patrol_data.is_null() {
        fn_0041fe60(e, patrol_data, 1);
    }
    e.set(this, ExtraPatrolRefData::pPatrolData, Ptr::NULL);
}

// Translated from 0041fe60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of `PatrolRefData` (the compiler emitted a
/// copy in this unit): runs the destructor body (`0067c670`) and frees the
/// object when bit 0 of `flags` is set. Returns `this`. No Xbox PDB name.
pub fn fn_0041fe60(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(PATROL_DATA_DESTROY, &args![this]);
    finish_scalar_deleting_destructor(e, this, flags)
}

// Translated from 0041fe90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `pPatrolData` of the type `0x6F` extra data (`ExtraPatrolRefData`), or 0.
/// No Xbox PDB name.
pub fn fn_0041fe90(e: &mut Engine, this: Ptr<ExtraDataList>) -> u32 {
    extra_word_or_zero(e, this, EXTRA_PATROL_REF_DATA_TYPE)
}

// Translated from 0041fec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the occlusion plane reference data: the type `0x76` extra data
/// (`ExtraOcclusionPlaneRefData`) is deleted when there is one, then, for a
/// non-null `data`, a new one (`0x10` bytes, constructor `0041ff80`) is given
/// it (`0041fd00`) and added. No Xbox PDB name. The exception-unwinding frame
/// is not translated.
pub fn fn_0041fec0(e: &mut Engine, this: Ptr<ExtraDataList>, data: u32) {
    replace_extra_holding_word(
        e,
        this,
        EXTRA_OCCLUSION_PLANE_REF_DATA_TYPE,
        data,
        |e, block| fn_0041ff80(e, Ptr::new(block)).addr(),
    );
}

// Translated from 0041ff80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraOcclusionPlaneRefData` (type `0x76`): the base
/// constructor (`0040ec80`) with the type and the vtable `010151cc`; `pData`
/// is left as it is (the setter `0041fd00` fills it). Returns `this`.
pub fn fn_0041ff80(
    e: &mut Engine,
    this: Ptr<ExtraOcclusionPlaneRefData>,
) -> Ptr<ExtraOcclusionPlaneRefData> {
    e.call(
        BS_EXTRA_DATA_INIT,
        &args![this, EXTRA_OCCLUSION_PLANE_REF_DATA_TYPE as u32],
    );
    e.mem
        .set_u32(this.addr(), VTABLE_EXTRA_OCCLUSION_PLANE_REF_DATA);
    this
}

// Translated from 0041ffb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraOcclusionPlaneRefData::Compare` (Xbox PDB): true when the two
/// differ. Two extra data without data are equal; one without differs;
/// otherwise they differ when any of the four words of the data differs.
pub fn extra_occlusion_plane_ref_data_compare(
    e: &mut Engine,
    this: Ptr<ExtraOcclusionPlaneRefData>,
    other: Ptr<ExtraOcclusionPlaneRefData>,
) -> bool {
    let mine = e.get(this, ExtraOcclusionPlaneRefData::pData);
    let theirs = e.get(other, ExtraOcclusionPlaneRefData::pData);
    if mine.is_null() && theirs.is_null() {
        return false;
    }
    if mine.is_null() || theirs.is_null() {
        return true;
    }
    (0..4).any(|index| e.mem.u32(mine.addr() + 4 * index) != e.mem.u32(theirs.addr() + 4 * index))
}

// Translated from 00420030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraOcclusionPlaneRefData::_scalar_deleting_destructor_` (Xbox PDB):
/// runs the destructor (`00420060`) and frees the object when bit 0 of
/// `flags` is set. Returns `this`.
pub fn extra_occlusion_plane_ref_data_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraOcclusionPlaneRefData>,
    flags: u32,
) -> Ptr<ExtraOcclusionPlaneRefData> {
    fn_00420060(e, this);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 00420060 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of `ExtraOcclusionPlaneRefData` (called by `00420030`; not in
/// the engine map nor in Ghidra's function list): sets its vtable, frees the
/// data (`004200c0`), then runs the `BSExtraData` destructor (`0040ecb0`).
/// The exception-unwinding frame is not translated.
pub fn fn_00420060(e: &mut Engine, this: Ptr<ExtraOcclusionPlaneRefData>) {
    e.mem
        .set_u32(this.addr(), VTABLE_EXTRA_OCCLUSION_PLANE_REF_DATA);
    fn_004200c0(e, this);
    e.call(BS_EXTRA_DATA_DESTROY, &args![this]);
}

// Translated from 004200c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Frees the data of an `ExtraOcclusionPlaneRefData` (plain `operator
/// delete`) when there is one, and sets `pData` to null. No Xbox PDB name.
pub fn fn_004200c0(e: &mut Engine, this: Ptr<ExtraOcclusionPlaneRefData>) {
    let data = e.get(this, ExtraOcclusionPlaneRefData::pData);
    if !data.is_null() {
        e.call(OPERATOR_DELETE, &args![data]);
    }
    e.set(this, ExtraOcclusionPlaneRefData::pData, Ptr::NULL);
}

// Translated from 00420100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::GetOcclusionPlaneLinkedRefData` (Xbox PDB): `pData` of the
/// type `0x76` extra data (`ExtraOcclusionPlaneRefData`), or 0.
pub fn extra_data_list_get_occlusion_plane_linked_ref_data(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> u32 {
    extra_word_or_zero(e, this, EXTRA_OCCLUSION_PLANE_REF_DATA_TYPE)
}

// Translated from 00420130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Word `index` of the occlusion plane reference data of the list
/// (`00420100`), or 0 when the list has none. The index is not checked
/// against the four words. No Xbox PDB name.
pub fn fn_00420130(e: &mut Engine, this: Ptr<ExtraDataList>, index: i32) -> u32 {
    let data = extra_data_list_get_occlusion_plane_linked_ref_data(e, this);
    if data == 0 {
        0
    } else {
        e.mem.u32(data.wrapping_add((index as u32).wrapping_mul(4)))
    }
}

// Translated from 00420160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `reference` as word `index` of the occlusion plane reference data.
/// When the list has none and `reference` is not null, the data is built
/// first (`0x10` bytes, constructor `00411dc0`) and set (`0041fec0`); the
/// word is then stored if the list has data, whatever the value (a null
/// `reference` clears the word). The index is not checked against the four
/// words. No Xbox PDB name. The exception-unwinding frame is not translated.
pub fn fn_00420160(e: &mut Engine, this: Ptr<ExtraDataList>, reference: u32, index: i32) {
    if extra_data_list_get_occlusion_plane_linked_ref_data(e, this) == 0 && reference != 0 {
        let data = new_object(e, 0x10, OCCLUSION_PLANE_DATA_INIT);
        fn_0041fec0(e, this, data);
    }
    let data = extra_data_list_get_occlusion_plane_linked_ref_data(e, this);
    if data != 0 {
        e.mem
            .set_u32(data.wrapping_add((index as u32).wrapping_mul(4)), reference);
    }
}

// Translated from 00420210 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the portal reference data: the type `0x77` extra data
/// (`ExtraPortalRefData`) is deleted when there is one, then, for a non-null
/// `data`, a new one (`0x10` bytes, constructor `004202d0`) is given it
/// (`0041fd00`) and added. No Xbox PDB name. The exception-unwinding frame is
/// not translated.
pub fn fn_00420210(e: &mut Engine, this: Ptr<ExtraDataList>, data: u32) {
    replace_extra_holding_word(e, this, EXTRA_PORTAL_REF_DATA_TYPE, data, |e, block| {
        e.call(PORTAL_REF_DATA_INIT, &args![block]).u32()
    });
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0041db00, fn_0041db00(Ptr<ExtraDataList>) -> bool),
        entry!(0x0041db30, fn_0041db30(Ptr<ExtraEnableStateParent>) -> bool),
        entry!(0x0041db50, fn_0041db50(Ptr<ExtraDataList>, bool)),
        entry!(0x0041db80, fn_0041db80(Ptr<ExtraEnableStateParent>, bool)),
        entry!(
            0x0041dbd0,
            extra_data_list_should_pop_in_when_enabled_by_parent(Ptr<ExtraDataList>) -> bool
        ),
        entry!(0x0041dc00, fn_0041dc00(Ptr<ExtraEnableStateParent>) -> bool),
        entry!(0x0041dc20, fn_0041dc20(Ptr<ExtraDataList>) -> bool),
        entry!(0x0041dc50, fn_0041dc50(Ptr<ExtraEnableStateParent>) -> bool),
        entry!(0x0041dc70, fn_0041dc70(Ptr<ExtraDataList>, u8)),
        entry!(
            0x0041dca0,
            fn_0041dca0(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(0x0041dcd0, fn_0041dcd0(Ptr<ExtraDataList>, u32)),
        entry!(0x0041dda0, fn_0041dda0(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041de00,
            extra_data_list_get_item_dropper(Ptr<ExtraDataList>) -> u32
        ),
        entry!(
            0x0041de40,
            extra_data_list_add_dropped_item(Ptr<ExtraDataList>, u32)
        ),
        entry!(0x0041df00, fn_0041df00(Ptr<ExtraDataList>) -> u32),
        entry!(
            0x0041df90,
            extra_data_list_get_dropped_item_list(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0041dfd0,
            extra_data_list_remove_dropped_item_list(Ptr<ExtraDataList>)
        ),
        entry!(0x0041e000, fn_0041e000(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041e0d0,
            extra_data_list_remove_dropped_item(Ptr<ExtraDataList>, u32)
        ),
        entry!(
            0x0041e130,
            extra_data_list_get_water_type(Ptr<ExtraDataList>) -> u32
        ),
        entry!(0x0041e160, fn_0041e160(Ptr<ExtraDataList>, u32)),
        entry!(0x0041e220, fn_0041e220(Ptr<ExtraDataList>) -> u32),
        entry!(0x0041e250, fn_0041e250(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041e310,
            extra_data_list_get_ash_pile_ref(Ptr<ExtraDataList>) -> u32
        ),
        entry!(0x0041e340, fn_0041e340(Ptr<ExtraDataList>, u32)),
        entry!(0x0041e410, fn_0041e410(Ptr<ExtraDataList>) -> u32),
        entry!(0x0041e440, fn_0041e440(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041e500,
            extra_data_list_get_linked_ref_children(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(0x0041e530, fn_0041e530(Ptr<ExtraDataList>, u32)),
        entry!(0x0041e600, fn_0041e600(Ptr<ExtraDataList>, u32)),
        entry!(0x0041e660, fn_0041e660(Ptr<ExtraDataList>) -> u32),
        entry!(
            0x0041e690,
            extra_data_list_set_open_close_activate_ref(Ptr<ExtraDataList>, u32)
        ),
        entry!(
            0x0041e750,
            fn_0041e750(Ptr<ExtraOpenCloseActivateRef>) -> Ptr<ExtraOpenCloseActivateRef>
        ),
        entry!(
            0x0041e780,
            fn_0041e780(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(0x0041e7b0, fn_0041e7b0(Ptr<ExtraDataList>, u32) -> u32),
        entry!(0x0041e7f0, fn_0041e7f0(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041e960,
            extra_data_list_remove_activate_parent(Ptr<ExtraDataList>, u32)
        ),
        entry!(0x0041e9e0, fn_0041e9e0(Ptr<ExtraDataList>, u32) -> f32),
        entry!(0x0041ea30, fn_0041ea30(Ptr<ExtraDataList>, u32, f32)),
        entry!(0x0041eb60, fn_0041eb60(Ptr<ExtraDataList>) -> bool),
        entry!(0x0041eba0, fn_0041eba0(Ptr<ExtraDataList>, bool)),
        entry!(
            0x0041ec80,
            fn_0041ec80(Ptr<ExtraDataList>, Ptr<BSStringT>) -> Ptr<BSStringT>
        ),
        entry!(0x0041ece0, fn_0041ece0(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041eda0,
            extra_data_list_get_activate_ref_children(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0041edd0,
            extra_data_list_add_activate_ref_child(Ptr<ExtraDataList>, u32)
        ),
        entry!(0x0041ef20, fn_0041ef20(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041eff0,
            extra_data_list_set_activate_children_timer(Ptr<ExtraDataList>, f32)
        ),
        entry!(
            0x0041f020,
            extra_data_list_get_activate_children_timer(Ptr<ExtraDataList>) -> f32
        ),
        entry!(
            0x0041f050,
            extra_data_list_get_decal_refs(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0041f080,
            fn_0041f080(Ptr<ExtraDataList>, u32, Ptr<NiPoint3>, Ptr<NiPoint3>)
        ),
        entry!(
            0x0041f140,
            extra_data_list_get_reflected_refs(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0041f170,
            fn_0041f170(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(0x0041f1a0, fn_0041f1a0(Ptr<ExtraDataList>, u32, bool)),
        entry!(0x0041f330, fn_0041f330(Ptr<ExtraDataList>, u32, bool)),
        entry!(0x0041f4c0, fn_0041f4c0(Ptr<ExtraDataList>, u32, bool)),
        entry!(0x0041f650, fn_0041f650(Ptr<ExtraDataList>, u32, bool)),
        entry!(
            0x0041f7e0,
            extra_data_list_get_water_light_refs(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0041f810,
            extra_data_list_get_lit_water_refs(Ptr<ExtraDataList>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0041f840,
            extra_data_list_set_water_light_ref(Ptr<ExtraDataList>, u32, bool)
        ),
        entry!(0x0041f940, fn_0041f940(Ptr<ExtraDataList>, u32, bool)),
        entry!(
            0x0041fa60,
            extra_data_list_add_primitive(Ptr<ExtraDataList>, u32)
        ),
        entry!(
            0x0041faf0,
            fn_0041faf0(Ptr<ExtraPrimitive>, u32) -> Ptr<ExtraPrimitive>
        ),
        entry!(
            0x0041fb20,
            extra_primitive_scalar_deleting_destructor(
                Ptr<ExtraPrimitive>,
                u32,
            ) -> Ptr<ExtraPrimitive>
        ),
        entry!(0x0041fb50, fn_0041fb50(Ptr<ExtraPrimitive>)),
        entry!(
            0x0041fbe0,
            extra_data_list_get_primitive(Ptr<ExtraDataList>) -> u32
        ),
        entry!(0x0041fc10, fn_0041fc10(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041fcd0,
            fn_0041fcd0(Ptr<ExtraPatrolRefData>) -> Ptr<ExtraPatrolRefData>
        ),
        entry!(
            0x0041fd20,
            extra_patrol_ref_data_compare(Ptr<ExtraPatrolRefData>, Ptr<ExtraPatrolRefData>) -> bool
        ),
        entry!(
            0x0041fd80,
            extra_patrol_ref_data_scalar_deleting_destructor(
                Ptr<ExtraPatrolRefData>,
                u32,
            )
                -> Ptr<ExtraPatrolRefData>
        ),
        entry!(0x0041fdb0, fn_0041fdb0(Ptr<ExtraPatrolRefData>)),
        entry!(0x0041fe10, fn_0041fe10(Ptr<ExtraPatrolRefData>)),
        entry!(0x0041fe60, fn_0041fe60(Ptr, u32) -> Ptr),
        entry!(0x0041fe90, fn_0041fe90(Ptr<ExtraDataList>) -> u32),
        entry!(0x0041fec0, fn_0041fec0(Ptr<ExtraDataList>, u32)),
        entry!(
            0x0041ff80,
            fn_0041ff80(Ptr<ExtraOcclusionPlaneRefData>) -> Ptr<ExtraOcclusionPlaneRefData>
        ),
        entry!(
            0x0041ffb0,
            extra_occlusion_plane_ref_data_compare(
                Ptr<ExtraOcclusionPlaneRefData>,
                Ptr<ExtraOcclusionPlaneRefData>,
            ) -> bool
        ),
        entry!(
            0x00420030,
            extra_occlusion_plane_ref_data_scalar_deleting_destructor(
                Ptr<ExtraOcclusionPlaneRefData>,
                u32,
            ) -> Ptr<
                ExtraOcclusionPlaneRefData,
            >
        ),
        entry!(0x00420060, fn_00420060(Ptr<ExtraOcclusionPlaneRefData>)),
        entry!(0x004200c0, fn_004200c0(Ptr<ExtraOcclusionPlaneRefData>)),
        entry!(
            0x00420100,
            extra_data_list_get_occlusion_plane_linked_ref_data(Ptr<ExtraDataList>) -> u32
        ),
        entry!(0x00420130, fn_00420130(Ptr<ExtraDataList>, i32) -> u32),
        entry!(0x00420160, fn_00420160(Ptr<ExtraDataList>, u32, i32)),
        entry!(0x00420210, fn_00420210(Ptr<ExtraDataList>, u32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    type Log = Vec<(u32, Vec<u32>)>;

    /// A fake list: the extra data of type `t` sits in the word at
    /// `list + 0x100 + 4 * t` (the doubles of `GetExtraData`, `AddExtra` and
    /// the two `RemoveExtra` read and write it).
    fn table_slot(list: u32, extra_type: u32) -> u32 {
        list + 0x100 + 4 * extra_type
    }

    fn returns(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// Makes the constructor at `address` a double that sets the type byte
    /// of the new block and returns it.
    fn constructor_double(e: &mut Engine, address: u32, extra_type: u8) {
        e.register_double(address, move |e, a| {
            e.mem.set_u8(a[0] + 4, extra_type);
            returns(a[0])
        });
    }

    fn engine() -> Engine {
        let mut e = Engine::new();
        e.register(GET_EXTRA_DATA, |e, a| {
            returns(e.mem.u32(table_slot(a[0], a[1])))
        });
        e.register(ADD_EXTRA, |e, a| {
            let extra_type = e.mem.u8(a[1] + 4) as u32;
            e.mem.set_u32(table_slot(a[0], extra_type), a[1]);
            returns(a[1])
        });
        e.register(REMOVE_EXTRA, |e, a| {
            let extra_type = e.mem.u8(a[1] + 4) as u32;
            e.mem.set_u32(table_slot(a[0], extra_type), 0);
            Ret::default()
        });
        e.register(REMOVE_EXTRA_BY_TYPE, |e, a| {
            e.mem.set_u32(table_slot(a[0], a[1]), 0);
            Ret::default()
        });
        e.register(OPERATOR_NEW, |e, a| returns(e.mem.alloc(a[0])));
        e.register(OPERATOR_DELETE, |_, _| Ret::default());
        e.register(BS_EXTRA_DATA_INIT, |e, a| {
            e.mem.set_u8(a[0] + 4, a[1] as u8);
            e.mem.set_u32(a[0] + 8, 0);
            returns(a[0])
        });
        // The list doubles work on the inline head node of an extra data;
        // an add puts the item in the head node, which is all the tests need.
        e.register(LIST_CONTAINS, |e, a| {
            let wanted = e.mem.u32(a[1]);
            let mut node = a[0];
            let mut found = false;
            while node != 0 {
                found |= e.mem.u32(node) == wanted;
                node = e.mem.u32(node + 4);
            }
            returns(found as u32)
        });
        e.register(LIST_ADD_HEAD, |e, a| {
            let item = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], item);
            Ret::default()
        });
        e.register(LIST_REMOVE_ITEM, |e, a| {
            let wanted = e.mem.u32(a[1]);
            if e.mem.u32(a[0]) == wanted {
                e.mem.set_u32(a[0], 0);
            }
            Ret::default()
        });
        e.register(LIST_IS_EMPTY, |e, a| {
            returns((e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32)
        });
        e.register(LIST_ITEM_SLOT, |_, a| returns(a[0]));
        e.register(LIST_NEXT, |e, a| returns(e.mem.u32(a[0] + 4)));
        constructor_double(&mut e, ENABLE_STATE_CHILDREN_INIT, 0x38);
        constructor_double(&mut e, ITEM_DROPPER_INIT, 0x39);
        constructor_double(&mut e, DROPPED_ITEM_LIST_INIT, 0x3a);
        constructor_double(&mut e, WATER_TYPE_INIT, 0x03);
        constructor_double(&mut e, TELEPORT_MARKER_INIT, 0x3b);
        constructor_double(&mut e, ASH_PILE_REF_INIT, 0x89);
        constructor_double(&mut e, LINKED_REF_INIT, 0x51);
        constructor_double(&mut e, LINKED_REF_CHILDREN_INIT, 0x52);
        constructor_double(&mut e, ACTIVATE_REF_INIT, 0x53);
        e
    }

    fn new_list(e: &mut Engine) -> Ptr<ExtraDataList> {
        Ptr::new(e.mem.alloc(0x400))
    }

    /// An extra data of the given type, put in the list's table.
    fn put_extra(e: &mut Engine, list: Ptr<ExtraDataList>, extra_type: u8) -> u32 {
        let extra = e.mem.alloc(0x40);
        e.mem.set_u8(extra + 4, extra_type);
        e.mem
            .set_u32(table_slot(list.addr(), extra_type as u32), extra);
        extra
    }

    fn extra_of(e: &Engine, list: Ptr<ExtraDataList>, extra_type: u8) -> u32 {
        e.mem.u32(table_slot(list.addr(), extra_type as u32))
    }

    fn calls_to(log: &Log, address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(callee, _)| *callee == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    // 0041db00, 0041db30, 0041db50, 0041db80, 0041dbd0, 0041dc00, 0041dc20,
    // 0041dc50, 0041dc70: the flags of the enable state parent.

    #[test]
    fn flag_bit_zero_getter_reads_the_flags_or_gives_false() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert!(!e.call(0x0041db00, &args![list]).bool());
        let extra = put_extra(&mut e, list, 0x37);
        assert!(!e.call(0x0041db00, &args![list]).bool());
        e.mem.set_u8(extra + 0x10, 0b110);
        assert!(!e.call(0x0041db00, &args![list]).bool());
        e.mem.set_u8(extra + 0x10, 0b001);
        assert!(e.call(0x0041db00, &args![list]).bool());
    }

    #[test]
    fn flag_bit_zero_of_the_extra_data_is_its_low_bit() {
        let mut e = engine();
        let extra: Ptr<ExtraEnableStateParent> = e.new_object();
        e.set(extra, ExtraEnableStateParent::cFlags, 0xfe);
        assert!(!e.call(0x0041db30, &args![extra]).bool());
        e.set(extra, ExtraEnableStateParent::cFlags, 0x01);
        assert!(e.call(0x0041db30, &args![extra]).bool());
    }

    #[test]
    fn flag_bit_zero_setter_goes_through_the_extra_data_setter() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call(0x0041db50, &args![list, true]);
        let extra = put_extra(&mut e, list, 0x37);
        e.mem.set_u8(extra + 0x10, 0b1000_0110);
        e.call_log = Some(vec![]);
        e.call(0x0041db50, &args![list, true]);
        assert_eq!(e.mem.u8(extra + 0x10), 0b1000_0111);
        e.call(0x0041db50, &args![list, false]);
        assert_eq!(e.mem.u8(extra + 0x10), 0b1000_0110);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, GET_EXTRA_DATA).len(), 2);
    }

    #[test]
    fn flag_bit_zero_setter_on_the_extra_data_keeps_the_other_bits() {
        let mut e = engine();
        let extra: Ptr<ExtraEnableStateParent> = e.new_object();
        e.set(extra, ExtraEnableStateParent::cFlags, 0b0101_0100);
        e.call(0x0041db80, &args![extra, true]);
        assert_eq!(e.get(extra, ExtraEnableStateParent::cFlags), 0b0101_0101);
        e.call(0x0041db80, &args![extra, false]);
        assert_eq!(e.get(extra, ExtraEnableStateParent::cFlags), 0b0101_0100);
        e.call(0x0041db80, &args![extra, false]);
        assert_eq!(e.get(extra, ExtraEnableStateParent::cFlags), 0b0101_0100);
    }

    #[test]
    fn pop_in_when_enabled_by_parent_is_bit_one() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert!(!e.call(0x0041dbd0, &args![list]).bool());
        let extra = put_extra(&mut e, list, 0x37);
        e.mem.set_u8(extra + 0x10, 0b101);
        assert!(!e.call(0x0041dbd0, &args![list]).bool());
        e.mem.set_u8(extra + 0x10, 0b010);
        assert!(e.call(0x0041dbd0, &args![list]).bool());
    }

    #[test]
    fn flag_bit_one_of_the_extra_data() {
        let mut e = engine();
        let extra: Ptr<ExtraEnableStateParent> = e.new_object();
        e.set(extra, ExtraEnableStateParent::cFlags, 0b101);
        assert!(!e.call(0x0041dc00, &args![extra]).bool());
        e.set(extra, ExtraEnableStateParent::cFlags, 0b010);
        assert!(e.call(0x0041dc00, &args![extra]).bool());
    }

    #[test]
    fn flag_bit_two_getter_reads_the_flags_or_gives_false() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert!(!e.call(0x0041dc20, &args![list]).bool());
        let extra = put_extra(&mut e, list, 0x37);
        e.mem.set_u8(extra + 0x10, 0b011);
        assert!(!e.call(0x0041dc20, &args![list]).bool());
        e.mem.set_u8(extra + 0x10, 0b100);
        assert!(e.call(0x0041dc20, &args![list]).bool());
    }

    #[test]
    fn flag_bit_two_of_the_extra_data() {
        let mut e = engine();
        let extra: Ptr<ExtraEnableStateParent> = e.new_object();
        e.set(extra, ExtraEnableStateParent::cFlags, 0b011);
        assert!(!e.call(0x0041dc50, &args![extra]).bool());
        e.set(extra, ExtraEnableStateParent::cFlags, 0b100);
        assert!(e.call(0x0041dc50, &args![extra]).bool());
    }

    #[test]
    fn flags_setter_replaces_the_whole_byte_or_does_nothing() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call(0x0041dc70, &args![list, 0x55u32]);
        let extra = put_extra(&mut e, list, 0x37);
        e.mem.set_u8(extra + 0x10, 0xff);
        e.call(0x0041dc70, &args![list, 0x12u32]);
        assert_eq!(e.mem.u8(extra + 0x10), 0x12);
        // Only the byte is stored.
        e.call(0x0041dc70, &args![list, 0x1234u32]);
        assert_eq!(e.mem.u8(extra + 0x10), 0x34);
    }

    // 0041dca0, 0041dcd0, 0041dda0: the children of the enable state parent.

    #[test]
    fn enable_state_children_getter_gives_the_inline_list_or_null() {
        let mut e = engine();
        let list = new_list(&mut e);
        let none = e.call(0x0041dca0, &args![list]).ptr::<BSSimpleList>();
        assert!(none.is_null());
        let extra = put_extra(&mut e, list, 0x38);
        let head = e.call(0x0041dca0, &args![list]).ptr::<BSSimpleList>();
        assert_eq!(head.addr(), extra + 0x0c);
    }

    #[test]
    fn add_enable_state_child_builds_the_extra_data_and_adds_the_child() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041dcd0, &args![list, 0xaaa0u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x38);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x14]]);
        assert_eq!(calls_to(&log, ENABLE_STATE_CHILDREN_INIT).len(), 1);
        assert_eq!(calls_to(&log, ADD_EXTRA), vec![vec![list.addr(), extra]]);
        assert_eq!(calls_to(&log, LIST_ADD_HEAD).len(), 1);
        assert_eq!(calls_to(&log, LIST_ADD_HEAD)[0][0], extra + 0x0c);
        assert_eq!(e.mem.u32(extra + 0x0c), 0xaaa0);
    }

    #[test]
    fn add_enable_state_child_skips_a_child_already_in_the_list() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x38);
        e.mem.set_u32(extra + 0x0c, 0xaaa0);
        e.call_log = Some(vec![]);
        e.call(0x0041dcd0, &args![list, 0xaaa0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(calls_to(&log, LIST_CONTAINS).len(), 1);
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
    }

    #[test]
    fn add_enable_state_child_ignores_a_null_child() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041dcd0, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(extra_of(&e, list, 0x38), 0);
    }

    #[test]
    fn remove_enable_state_child_deletes_the_extra_data_when_empty() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x38);
        e.mem.set_u32(extra + 0x0c, 0xaaa0);
        e.call_log = Some(vec![]);
        e.call(0x0041dda0, &args![list, 0xaaa0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, LIST_REMOVE_ITEM)[0][0], extra + 0x0c);
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), extra, 1]]
        );
        assert_eq!(extra_of(&e, list, 0x38), 0);
    }

    #[test]
    fn remove_enable_state_child_keeps_the_extra_data_when_not_empty() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x38);
        e.mem.set_u32(extra + 0x0c, 0xaaa0);
        e.mem.set_u32(extra + 0x10, 0x1000);
        e.call_log = Some(vec![]);
        e.call(0x0041dda0, &args![list, 0xbbb0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, REMOVE_EXTRA).is_empty());
        assert_eq!(extra_of(&e, list, 0x38), extra);
    }

    #[test]
    fn remove_enable_state_child_needs_a_child_and_an_extra_data() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041dda0, &args![list, 0xaaa0u32]);
        e.call(0x0041dda0, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, LIST_REMOVE_ITEM).is_empty());
        assert!(calls_to(&log, LIST_IS_EMPTY).is_empty());
    }

    // 0041de00, 0041de40: the item dropper.

    #[test]
    fn item_dropper_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041de00, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x39);
        e.mem.set_u32(extra + 0x0c, 0x7777);
        assert_eq!(e.call(0x0041de00, &args![list]).u32(), 0x7777);
    }

    #[test]
    fn add_dropped_item_with_null_removes_by_type() {
        let mut e = engine();
        let list = new_list(&mut e);
        put_extra(&mut e, list, 0x39);
        e.call_log = Some(vec![]);
        e.call(0x0041de40, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA_BY_TYPE),
            vec![vec![list.addr(), 0x39]]
        );
        assert_eq!(extra_of(&e, list, 0x39), 0);
    }

    #[test]
    fn add_dropped_item_builds_the_extra_data_then_stores_the_dropper() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041de40, &args![list, 0x7777u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x39);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(calls_to(&log, ITEM_DROPPER_INIT), vec![vec![extra]]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x7777);
    }

    #[test]
    fn add_dropped_item_overwrites_an_existing_dropper() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x39);
        e.mem.set_u32(extra + 0x0c, 0x1111);
        e.call_log = Some(vec![]);
        e.call(0x0041de40, &args![list, 0x7777u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(e.mem.u32(extra + 0x0c), 0x7777);
    }

    // 0041df00: the search of the dropped item list.

    /// A reference whose base form has the given form type.
    fn reference_with_form_type(e: &mut Engine, form_type: u8) -> u32 {
        let form = e.mem.alloc(0x20);
        e.mem.set_u8(form + 4, form_type);
        let reference = e.mem.alloc(0x40);
        e.mem.set_u32(reference + 0x20, form);
        reference
    }

    fn engine_with_form_doubles() -> Engine {
        let mut e = engine();
        e.register(REFERENCE_BASE_FORM, |e, a| returns(e.mem.u32(a[0] + 0x20)));
        e.register(FORM_TYPE, |e, a| returns(e.mem.u8(a[0] + 4) as u32));
        e
    }

    #[test]
    fn dropped_item_search_without_the_extra_data_gives_zero() {
        let mut e = engine_with_form_doubles();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041df00, &args![list]).u32(), 0);
    }

    #[test]
    fn dropped_item_search_returns_the_first_item_of_form_type_0x28() {
        let mut e = engine_with_form_doubles();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x3a);
        let other = reference_with_form_type(&mut e, 0x10);
        let wanted = reference_with_form_type(&mut e, 0x28);
        let later = reference_with_form_type(&mut e, 0x28);
        let second = e.mem.alloc(8);
        let third = e.mem.alloc(8);
        e.mem.set_u32(extra + 0x0c, other);
        e.mem.set_u32(extra + 0x10, second);
        e.mem.set_u32(second, wanted);
        e.mem.set_u32(second + 4, third);
        e.mem.set_u32(third, later);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0041df00, &args![list]).u32(), wanted);
        let log = e.call_log.take().unwrap();
        // The walk stops at the first match: the third node is not read.
        assert_eq!(calls_to(&log, REFERENCE_BASE_FORM).len(), 2);
    }

    #[test]
    fn dropped_item_search_without_a_match_gives_zero() {
        let mut e = engine_with_form_doubles();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x3a);
        let only = reference_with_form_type(&mut e, 0x11);
        e.mem.set_u32(extra + 0x0c, only);
        assert_eq!(e.call(0x0041df00, &args![list]).u32(), 0);
    }

    #[test]
    fn dropped_item_search_stops_at_a_null_item() {
        let mut e = engine_with_form_doubles();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x3a);
        let wanted = reference_with_form_type(&mut e, 0x28);
        let second = e.mem.alloc(8);
        e.mem.set_u32(extra + 0x10, second);
        e.mem.set_u32(second, wanted);
        // The head item is null: the later node is never reached.
        assert_eq!(e.call(0x0041df00, &args![list]).u32(), 0);
    }

    // 0041df90, 0041dfd0, 0041e000, 0041e0d0: the dropped item list.

    #[test]
    fn dropped_item_list_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert!(e
            .call(0x0041df90, &args![list])
            .ptr::<BSSimpleList>()
            .is_null());
        let extra = put_extra(&mut e, list, 0x3a);
        assert_eq!(e.call(0x0041df90, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn remove_dropped_item_list_only_when_there_is_one() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041dfd0, &args![list]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, REMOVE_EXTRA_BY_TYPE).is_empty());
        put_extra(&mut e, list, 0x3a);
        e.call_log = Some(vec![]);
        e.call(0x0041dfd0, &args![list]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA_BY_TYPE),
            vec![vec![list.addr(), 0x3a]]
        );
        assert_eq!(extra_of(&e, list, 0x3a), 0);
    }

    #[test]
    fn add_to_dropped_item_list_builds_the_extra_data_and_adds_the_item() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041e000, &args![list, 0x4242u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x3a);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x14]]);
        assert_eq!(calls_to(&log, DROPPED_ITEM_LIST_INIT), vec![vec![extra]]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x4242);
    }

    #[test]
    fn add_to_dropped_item_list_skips_an_item_already_there() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x3a);
        e.mem.set_u32(extra + 0x0c, 0x4242);
        e.call_log = Some(vec![]);
        e.call(0x0041e000, &args![list, 0x4242u32]);
        e.call(0x0041e000, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
    }

    #[test]
    fn remove_dropped_item_deletes_the_extra_data_when_empty() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x3a);
        e.mem.set_u32(extra + 0x0c, 0x4242);
        e.call_log = Some(vec![]);
        e.call(0x0041e0d0, &args![list, 0x4242u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), extra, 1]]
        );
    }

    #[test]
    fn remove_dropped_item_keeps_a_non_empty_list() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x3a);
        e.mem.set_u32(extra + 0x0c, 0x4242);
        e.call_log = Some(vec![]);
        e.call(0x0041e0d0, &args![list, 0x9999u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, REMOVE_EXTRA).is_empty());
        assert_eq!(extra_of(&e, list, 0x3a), extra);
    }

    // 0041e130, 0041e160: the water type.

    #[test]
    fn water_type_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041e130, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x03);
        e.mem.set_u32(extra + 0x0c, 0x6060);
        assert_eq!(e.call(0x0041e130, &args![list]).u32(), 0x6060);
    }

    #[test]
    fn water_type_setter_stores_the_value_before_adding_a_new_extra_data() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.register(ADD_EXTRA, |e, a| {
            // The value is already in the block when it is added.
            assert_eq!(e.mem.u32(a[1] + 0x0c), 0x6060);
            e.mem.set_u32(table_slot(a[0], 3), a[1]);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0041e160, &args![list, 0x6060u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(calls_to(&log, WATER_TYPE_INIT).len(), 1);
        assert_eq!(e.mem.u32(extra_of(&e, list, 3) + 0x0c), 0x6060);
    }

    #[test]
    fn water_type_setter_overwrites_or_removes() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x03);
        e.call_log = Some(vec![]);
        e.call(0x0041e160, &args![list, 0x7070u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(e.mem.u32(extra + 0x0c), 0x7070);
        e.call_log = Some(vec![]);
        e.call(0x0041e160, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA_BY_TYPE),
            vec![vec![list.addr(), 3]]
        );
        assert_eq!(extra_of(&e, list, 3), 0);
    }

    // 0041e220, 0041e250: the type 0x3B marker.

    #[test]
    fn marker_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041e220, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x3b);
        e.mem.set_u32(extra + 0x0c, 0x3131);
        assert_eq!(e.call(0x0041e220, &args![list]).u32(), 0x3131);
    }

    #[test]
    fn marker_setter_builds_overwrites_and_removes() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041e250, &args![list, 0x3131u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x3b);
        assert_eq!(calls_to(&log, TELEPORT_MARKER_INIT), vec![vec![extra]]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x3131);
        e.call(0x0041e250, &args![list, 0x3232u32]);
        assert_eq!(extra_of(&e, list, 0x3b), extra);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x3232);
        e.call_log = Some(vec![]);
        e.call(0x0041e250, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA_BY_TYPE),
            vec![vec![list.addr(), 0x3b]]
        );
    }

    // 0041e310, 0041e340: the ash pile reference.

    #[test]
    fn ash_pile_ref_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041e310, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x89);
        e.mem.set_u32(extra + 0x0c, 0x8989);
        assert_eq!(e.call(0x0041e310, &args![list]).u32(), 0x8989);
    }

    #[test]
    fn ash_pile_ref_setter_adds_before_it_stores() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.register(ADD_EXTRA, |e, a| {
            // The value is stored after the add in this function.
            assert_eq!(e.mem.u32(a[1] + 0x0c), 0);
            e.mem.set_u32(table_slot(a[0], 0x89), a[1]);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0041e340, &args![list, 0x8989u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x89);
        assert_eq!(calls_to(&log, ASH_PILE_REF_INIT), vec![vec![extra]]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x8989);
    }

    #[test]
    fn ash_pile_ref_setter_overwrites_or_deletes_the_object() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x89);
        e.call(0x0041e340, &args![list, 0x9090u32]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x9090);
        e.call_log = Some(vec![]);
        e.call(0x0041e340, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), extra, 1]]
        );
        // With no extra data a null reference does nothing at all.
        e.call_log = Some(vec![]);
        e.call(0x0041e340, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, REMOVE_EXTRA).is_empty());
        assert!(calls_to(&log, REMOVE_EXTRA_BY_TYPE).is_empty());
    }

    // 0041e410, 0041e440: the linked reference.

    #[test]
    fn linked_ref_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041e410, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x51);
        e.mem.set_u32(extra + 0x0c, 0x5151);
        assert_eq!(e.call(0x0041e410, &args![list]).u32(), 0x5151);
    }

    #[test]
    fn linked_ref_setter_builds_overwrites_and_removes() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041e440, &args![list, 0x5151u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x51);
        assert_eq!(calls_to(&log, LINKED_REF_INIT), vec![vec![extra]]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x5151);
        e.call(0x0041e440, &args![list, 0x5252u32]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x5252);
        e.call(0x0041e440, &args![list, 0u32]);
        assert_eq!(extra_of(&e, list, 0x51), 0);
    }

    // 0041e500, 0041e530, 0041e600: the linked reference children.

    #[test]
    fn linked_ref_children_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert!(e
            .call(0x0041e500, &args![list])
            .ptr::<BSSimpleList>()
            .is_null());
        let extra = put_extra(&mut e, list, 0x52);
        assert_eq!(e.call(0x0041e500, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn add_linked_ref_child_builds_the_extra_data_and_adds_the_child() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041e530, &args![list, 0x5353u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x52);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x14]]);
        assert_eq!(calls_to(&log, LINKED_REF_CHILDREN_INIT), vec![vec![extra]]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x5353);
        // The same child again, and a null one, change nothing.
        e.call_log = Some(vec![]);
        e.call(0x0041e530, &args![list, 0x5353u32]);
        e.call(0x0041e530, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
    }

    #[test]
    fn remove_linked_ref_child_deletes_the_extra_data_when_empty() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x52);
        e.mem.set_u32(extra + 0x0c, 0x5353);
        e.call_log = Some(vec![]);
        e.call(0x0041e600, &args![list, 0x5353u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), extra, 1]]
        );
        assert_eq!(extra_of(&e, list, 0x52), 0);
    }

    // 0041e660, 0041e690, 0041e750: the open/close activate reference.

    #[test]
    fn open_close_activate_ref_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041e660, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x6c);
        e.mem.set_u32(extra + 0x0c, 0x6c6c);
        assert_eq!(e.call(0x0041e660, &args![list]).u32(), 0x6c6c);
    }

    #[test]
    fn open_close_activate_ref_setter_builds_with_the_constructor_of_this_file() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.register(ADD_EXTRA, |e, a| {
            assert_eq!(e.mem.u32(a[1] + 0x0c), 0x6c6c);
            e.mem.set_u32(table_slot(a[0], 0x6c), a[1]);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0041e690, &args![list, 0x6c6cu32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x6c);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(calls_to(&log, BS_EXTRA_DATA_INIT), vec![vec![extra, 0x6c]]);
        assert_eq!(e.mem.u32(extra), 0x0101_51a8);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x6c6c);
    }

    #[test]
    fn open_close_activate_ref_setter_overwrites_or_removes() {
        let mut e = engine();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x6c);
        e.call(0x0041e690, &args![list, 0x6d6du32]);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x6d6d);
        e.call_log = Some(vec![]);
        e.call(0x0041e690, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA_BY_TYPE),
            vec![vec![list.addr(), 0x6c]]
        );
        assert_eq!(extra_of(&e, list, 0x6c), 0);
    }

    #[test]
    fn open_close_activate_ref_constructor() {
        let mut e = engine();
        let extra: Ptr<ExtraOpenCloseActivateRef> = e.new_object();
        e.mem.set_u32(extra.addr() + 0x0c, 0xdead);
        let result = e
            .call(0x0041e750, &args![extra])
            .ptr::<ExtraOpenCloseActivateRef>();
        assert_eq!(result, extra);
        assert_eq!(e.mem.u8(extra.addr() + 4), 0x6c);
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_51a8);
        assert_eq!(
            e.get(extra, ExtraOpenCloseActivateRef::pActivateRef),
            Ptr::NULL
        );
    }

    // 0041e780, 0041e7b0, 0041e7f0, 0041e960, 0041e9e0, 0041ea30, 0041eb60:
    // the activate reference.

    /// Doubles `00433a00` as a search of the first entry of the extra data's
    /// parent list (an entry is `[reference, delay]`, found when its first
    /// word is the reference; the test lists are one entry long).
    fn activate_find_double(e: &mut Engine) {
        e.register(ACTIVATE_REF_FIND, |e, a| {
            let entry = e.mem.u32(a[0] + 0x0c);
            let found = entry != 0 && e.mem.u32(entry) == a[1];
            returns(if found { entry } else { 0 })
        });
    }

    #[test]
    fn parent_list_getter() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert!(e
            .call(0x0041e780, &args![list])
            .ptr::<BSSimpleList>()
            .is_null());
        let extra = put_extra(&mut e, list, 0x53);
        assert_eq!(e.call(0x0041e780, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn parent_entry_finder_asks_the_extra_data() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041e7b0, &args![list, 0x5000u32]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x53);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5000);
        e.mem.set_u32(extra + 0x0c, entry);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0041e7b0, &args![list, 0x5000u32]).u32(), entry);
        assert_eq!(e.call(0x0041e7b0, &args![list, 0x5001u32]).u32(), 0);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, ACTIVATE_REF_FIND).len(), 2);
        assert_eq!(calls_to(&log, ACTIVATE_REF_FIND)[0], vec![extra, 0x5000]);
    }

    #[test]
    fn add_activate_parent_builds_the_extra_data_and_the_entry() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041e7f0, &args![list, 0x5000u32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x53);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x20], vec![0x08]]);
        assert_eq!(calls_to(&log, ACTIVATE_REF_INIT), vec![vec![extra]]);
        // A new extra data is not searched.
        assert!(calls_to(&log, ACTIVATE_REF_FIND).is_empty());
        let entry = e.mem.u32(extra + 0x0c);
        assert_ne!(entry, 0);
        assert_eq!(e.mem.u32(entry), 0x5000);
        assert_eq!(e.mem.u32(entry + 4), 0);
        assert_eq!(calls_to(&log, LIST_NODE_INIT).len(), 1);
    }

    #[test]
    fn add_activate_parent_adds_an_entry_only_for_a_new_parent() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x53);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5000);
        e.mem.set_u32(extra + 0x0c, entry);
        e.call_log = Some(vec![]);
        e.call(0x0041e7f0, &args![list, 0x5000u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
        e.call_log = Some(vec![]);
        e.call(0x0041e7f0, &args![list, 0x6000u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x08]]);
        assert_eq!(calls_to(&log, LIST_ADD_HEAD).len(), 1);
        assert!(calls_to(&log, ADD_EXTRA).is_empty());
        let added = e.mem.u32(extra + 0x0c);
        assert_ne!(added, entry);
        assert_eq!(e.mem.u32(added), 0x6000);
    }

    #[test]
    fn add_activate_parent_ignores_a_null_reference() {
        let mut e = engine();
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041e7f0, &args![list, 0u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    #[test]
    fn remove_activate_parent_deletes_the_entry_and_the_empty_extra_data() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x53);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5000);
        e.mem.set_u32(extra + 0x0c, entry);
        e.call_log = Some(vec![]);
        e.call(0x0041e960, &args![list, 0x5000u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, LIST_REMOVE_ITEM)[0][0], extra + 0x0c);
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![entry]]);
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA_BY_TYPE),
            vec![vec![list.addr(), 0x53]]
        );
        assert_eq!(extra_of(&e, list, 0x53), 0);
    }

    #[test]
    fn remove_activate_parent_keeps_a_non_empty_list_and_ignores_unknown_parents() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x53);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5000);
        e.mem.set_u32(extra + 0x0c, entry);
        e.mem.set_u32(extra + 0x10, 0x1234);
        e.call_log = Some(vec![]);
        e.call(0x0041e960, &args![list, 0x5000u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![entry]]);
        assert!(calls_to(&log, REMOVE_EXTRA_BY_TYPE).is_empty());
        e.call_log = Some(vec![]);
        e.call(0x0041e960, &args![list, 0x7000u32]);
        e.call(0x0041e960, &args![list, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());
        assert!(calls_to(&log, LIST_REMOVE_ITEM).is_empty());
    }

    #[test]
    fn activate_delay_getter() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041e9e0, &args![list, 0x5000u32]).f32(), 0.0);
        let extra = put_extra(&mut e, list, 0x53);
        assert_eq!(e.call(0x0041e9e0, &args![list, 0x5000u32]).f32(), 0.0);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5000);
        e.mem.set_u32(entry + 4, 2.5f32.to_bits());
        e.mem.set_u32(extra + 0x0c, entry);
        assert_eq!(e.call(0x0041e9e0, &args![list, 0x5000u32]).f32(), 2.5);
        assert_eq!(e.call(0x0041e9e0, &args![list, 0x5001u32]).f32(), 0.0);
        // A signalling NaN comes out of the x87 load quiet.
        e.mem.set_u32(entry + 4, 0x7f80_0001);
        let result = e.call(0x0041e9e0, &args![list, 0x5000u32]).f32();
        assert_eq!(result.to_bits(), 0x7fc0_0001);
    }

    #[test]
    fn set_activate_delay_builds_everything_for_a_new_parent() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041ea30, &args![list, 0x5000u32, 1.5f32]);
        let log = e.call_log.take().unwrap();
        let extra = extra_of(&e, list, 0x53);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x20], vec![0x08]]);
        assert_eq!(calls_to(&log, ACTIVATE_REF_INIT), vec![vec![extra]]);
        let entry = e.mem.u32(extra + 0x0c);
        assert_eq!(e.mem.u32(entry), 0x5000);
        assert_eq!(f32::from_bits(e.mem.u32(entry + 4)), 1.5);
    }

    #[test]
    fn set_activate_delay_updates_an_existing_entry() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x53);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5000);
        e.mem.set_u32(extra + 0x0c, entry);
        e.call_log = Some(vec![]);
        e.call(0x0041ea30, &args![list, 0x5000u32, 4.0f32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
        assert_eq!(f32::from_bits(e.mem.u32(entry + 4)), 4.0);
    }

    #[test]
    fn set_activate_delay_adds_an_entry_for_a_new_parent_of_an_existing_extra_data() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x53);
        e.call_log = Some(vec![]);
        e.call(0x0041ea30, &args![list, 0x6000u32, 0.25f32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x08]]);
        assert!(calls_to(&log, ADD_EXTRA).is_empty());
        let entry = e.mem.u32(extra + 0x0c);
        assert_eq!(e.mem.u32(entry), 0x6000);
        assert_eq!(f32::from_bits(e.mem.u32(entry + 4)), 0.25);
    }

    #[test]
    fn set_activate_delay_ignores_a_null_reference_and_quiets_a_nan() {
        let mut e = engine();
        activate_find_double(&mut e);
        let list = new_list(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0041ea30, &args![list, 0u32, 1.0f32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        let extra = put_extra(&mut e, list, 0x53);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5000);
        e.mem.set_u32(extra + 0x0c, entry);
        e.call(
            0x0041ea30,
            &args![list, 0x5000u32, f32::from_bits(0x7f80_0001)],
        );
        assert_eq!(e.mem.u32(entry + 4), 0x7fc0_0001);
    }

    #[test]
    fn activate_flag_getter_is_bit_zero_of_the_activate_flags() {
        let mut e = engine();
        let list = new_list(&mut e);
        assert!(!e.call(0x0041eb60, &args![list]).bool());
        let extra = put_extra(&mut e, list, 0x53);
        e.mem.set_u8(extra + 0x14, 0b10);
        assert!(!e.call(0x0041eb60, &args![list]).bool());
        e.mem.set_u8(extra + 0x14, 0b11);
        assert!(e.call(0x0041eb60, &args![list]).bool());
    }

    // ------------------------------------------------------------------
    // Second session: 0041eba0 to 00420210.

    /// The engine of the first session, with the constructors and helpers
    /// the second session's functions call.
    fn engine_b() -> Engine {
        let mut e = engine();
        constructor_double(&mut e, ACTIVATE_REF_CHILDREN_INIT, 0x54);
        constructor_double(&mut e, DECAL_REFS_INIT, 0x57);
        constructor_double(&mut e, REFLECTED_REFS_INIT, 0x65);
        constructor_double(&mut e, REFLECTOR_REFS_INIT, 0x66);
        constructor_double(&mut e, WATER_LIGHT_REFS_INIT, 0x84);
        constructor_double(&mut e, LIT_WATER_REFS_INIT, 0x85);
        constructor_double(&mut e, PORTAL_REF_DATA_INIT, 0x77);
        e.register(LIST_NODE_INIT, |_, a| returns(a[0]));
        e.register(OCCLUSION_PLANE_DATA_INIT, |_, a| returns(a[0]));
        e.register(STORE_WORD_AT_0C, |e, a| {
            e.mem.set_u32(a[0] + 0x0c, a[1]);
            Ret::default()
        });
        e.register(BS_EXTRA_DATA_DESTROY, |_, _| Ret::default());
        e.register(LIST_REMOVE_HEAD, |e, a| {
            e.mem.set_u32(a[0], 0);
            Ret::default()
        });
        e
    }

    /// An 8-byte entry (first word `reference`) made the head item of the
    /// list at +0x0C of `extra`.
    fn put_head_entry(e: &mut Engine, extra: u32, reference: u32) -> u32 {
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, reference);
        e.mem.set_u32(extra + 0x0c, entry);
        entry
    }

    /// Runs `f` and gives the calls it made.
    fn logged(e: &mut Engine, f: impl FnOnce(&mut Engine)) -> Log {
        e.call_log = Some(vec![]);
        f(e);
        e.call_log.take().unwrap()
    }

    // 0041eba0: the activate flag setter.

    #[test]
    fn activate_flag_setter_builds_sets_and_clears_bit_zero() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        // Clearing with no extra data builds nothing.
        let log = logged(&mut e, |e| {
            e.call(0x0041eba0, &args![list, 0u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(extra_of(&e, list, 0x53), 0);
        // Setting builds a 0x20-byte one, adds it and sets bit 0 (only the
        // low byte of the argument counts).
        let log = logged(&mut e, |e| {
            e.call(0x0041eba0, &args![list, 0x0100_0001u32]);
        });
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x20]]);
        let extra = extra_of(&e, list, 0x53);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, ACTIVATE_REF_INIT).len(), 1);
        assert_eq!(e.mem.u8(extra + 0x14), 0b1);
        // The other bits stay, and the extra data is not built again.
        e.mem.set_u8(extra + 0x14, 0b1010_0100);
        let log = logged(&mut e, |e| {
            e.call(0x0041eba0, &args![list, 1u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(e.mem.u8(extra + 0x14), 0b1010_0101);
        e.call(0x0041eba0, &args![list, 0u32]);
        assert_eq!(e.mem.u8(extra + 0x14), 0b1010_0100);
    }

    // 0041ec80: the activate text getter.

    #[test]
    fn activate_text_getter_copies_the_override_or_makes_an_empty_string() {
        let mut e = engine_b();
        e.register(STRING_COPY_CONSTRUCT, |_, a| returns(a[0]));
        e.register(STRING_CONSTRUCT_FROM_CHARS, |_, a| returns(a[0]));
        let list = new_list(&mut e);
        let out = e.mem.alloc(8);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041ec80, &args![list, out]).u32(), out);
        });
        assert_eq!(
            calls_to(&log, STRING_CONSTRUCT_FROM_CHARS),
            vec![vec![out, EMPTY_TEXT]]
        );
        assert!(calls_to(&log, STRING_COPY_CONSTRUCT).is_empty());
        let extra = put_extra(&mut e, list, 0x53);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041ec80, &args![list, out]).u32(), out);
        });
        assert_eq!(
            calls_to(&log, STRING_COPY_CONSTRUCT),
            vec![vec![out, extra + 0x18]]
        );
        assert!(calls_to(&log, STRING_CONSTRUCT_FROM_CHARS).is_empty());
    }

    // 0041ece0: the activate text setter.

    #[test]
    fn activate_text_setter_builds_the_extra_data_only_for_a_text() {
        let mut e = engine_b();
        e.register(STRING_SET, |_, _| Ret::default());
        let list = new_list(&mut e);
        let log = logged(&mut e, |e| {
            e.call(0x0041ece0, &args![list, 0u32]);
        });
        assert_eq!(log.len(), 1 + 1, "the call itself and the lookup");
        assert_eq!(extra_of(&e, list, 0x53), 0);
        let log = logged(&mut e, |e| {
            e.call(0x0041ece0, &args![list, 0x7000u32]);
        });
        let extra = extra_of(&e, list, 0x53);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x20]]);
        assert_eq!(
            calls_to(&log, STRING_SET),
            vec![vec![extra + 0x18, 0x7000, 0]]
        );
        // An existing one is given the text, also a null one.
        let log = logged(&mut e, |e| {
            e.call(0x0041ece0, &args![list, 0u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(calls_to(&log, STRING_SET), vec![vec![extra + 0x18, 0, 0]]);
    }

    // 0041eda0, 0041edd0, 0041ef20, 0041eff0, 0041f020: the activate
    // reference children.

    #[test]
    fn activate_children_getter_gives_the_inline_list_or_null() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert!(e
            .call(0x0041eda0, &args![list])
            .ptr::<BSSimpleList>()
            .is_null());
        let extra = put_extra(&mut e, list, 0x54);
        assert_eq!(e.call(0x0041eda0, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn add_activate_child_builds_the_extra_data_and_an_entry_once() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        // A null child does nothing.
        let log = logged(&mut e, |e| {
            e.call(0x0041edd0, &args![list, 0u32]);
        });
        assert_eq!(log.len(), 1);
        // The first child builds the 0x18-byte extra data and an 8-byte
        // entry, which holds the child and goes to the head of the list.
        let log = logged(&mut e, |e| {
            e.call(0x0041edd0, &args![list, 0x5001u32]);
        });
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x18], vec![8]]);
        let extra = extra_of(&e, list, 0x54);
        assert_eq!(calls_to(&log, ACTIVATE_REF_CHILDREN_INIT).len(), 1);
        let entry = e.mem.u32(extra + 0x0c);
        assert_eq!(e.mem.u32(entry), 0x5001);
        assert_eq!(calls_to(&log, LIST_NODE_INIT), vec![vec![entry]]);
        // The same child again: found, nothing is built.
        let log = logged(&mut e, |e| {
            e.call(0x0041edd0, &args![list, 0x5001u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
        // Another child: a new entry (the head-node double keeps one item).
        let log = logged(&mut e, |e| {
            e.call(0x0041edd0, &args![list, 0x5002u32]);
        });
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![8]]);
        assert_eq!(calls_to(&log, LIST_ADD_HEAD).len(), 1);
        let entry = e.mem.u32(extra + 0x0c);
        assert_eq!(e.mem.u32(entry), 0x5002);
    }

    #[test]
    fn add_activate_child_finds_the_child_further_down_the_list() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x54);
        put_head_entry(&mut e, extra, 0x5001);
        let second = e.mem.alloc(8);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5002);
        e.mem.set_u32(second, entry);
        e.mem.set_u32(extra + 0x0c + 4, second);
        let log = logged(&mut e, |e| {
            e.call(0x0041edd0, &args![list, 0x5002u32]);
        });
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        let log = logged(&mut e, |e| {
            e.call(0x0041edd0, &args![list, 0x5003u32]);
        });
        assert_eq!(calls_to(&log, LIST_ADD_HEAD).len(), 1);
    }

    #[test]
    fn remove_activate_child_deletes_the_entry_and_the_empty_extra_data() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        // No extra data, or a null child: only the lookup.
        let log = logged(&mut e, |e| {
            e.call(0x0041ef20, &args![list, 0x5001u32]);
            e.call(0x0041ef20, &args![list, 0u32]);
        });
        assert_eq!(log.len(), 3);
        let extra = put_extra(&mut e, list, 0x54);
        let entry = put_head_entry(&mut e, extra, 0x5001);
        // A child that is not in the list: nothing removed, the extra data
        // stays (the list is not empty).
        let log = logged(&mut e, |e| {
            e.call(0x0041ef20, &args![list, 0x5009u32]);
        });
        assert!(calls_to(&log, LIST_REMOVE_HEAD).is_empty());
        assert!(calls_to(&log, REMOVE_EXTRA).is_empty());
        // The child: taken out with the remove-head call on its node,
        // deleted, and the empty list takes the extra data with it.
        let log = logged(&mut e, |e| {
            e.call(0x0041ef20, &args![list, 0x5001u32]);
        });
        assert_eq!(calls_to(&log, LIST_REMOVE_HEAD), vec![vec![extra + 0x0c]]);
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![entry]]);
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), extra, 1]]
        );
        assert_eq!(extra_of(&e, list, 0x54), 0);
    }

    #[test]
    fn remove_activate_child_uses_remove_head_even_for_a_later_node() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x54);
        let first = put_head_entry(&mut e, extra, 0x5001);
        let second = e.mem.alloc(8);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 0x5002);
        e.mem.set_u32(second, entry);
        e.mem.set_u32(extra + 0x0c + 4, second);
        let log = logged(&mut e, |e| {
            e.call(0x0041ef20, &args![list, 0x5002u32]);
        });
        // The node the child was found at, not its predecessor.
        assert_eq!(calls_to(&log, LIST_REMOVE_HEAD), vec![vec![second]]);
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![entry]]);
        assert!(calls_to(&log, LIST_REMOVE_AFTER).is_empty());
        // The list still holds the first entry: the extra data stays.
        assert!(calls_to(&log, REMOVE_EXTRA).is_empty());
        assert_eq!(e.mem.u32(extra + 0x0c), first);
    }

    #[test]
    fn activate_children_timer_setter_and_getter() {
        let mut e = engine_b();
        e.map(NO_TIMER, 4);
        e.set_global(NO_TIMER, -1.0f32);
        let list = new_list(&mut e);
        // No extra data: the setter does nothing, the getter gives -1.0.
        e.call(0x0041eff0, &args![list, 2.5f32]);
        assert_eq!(extra_of(&e, list, 0x54), 0);
        assert_eq!(e.call(0x0041f020, &args![list]).f32(), -1.0);
        let extra = put_extra(&mut e, list, 0x54);
        e.call(0x0041eff0, &args![list, 2.5f32]);
        assert_eq!(f32::from_bits(e.mem.u32(extra + 0x14)), 2.5);
        assert_eq!(e.call(0x0041f020, &args![list]).f32(), 2.5);
        // The x87 load and store quiet a signalling NaN.
        e.call(0x0041eff0, &args![list, f32::from_bits(0x7f80_0001)]);
        assert_eq!(e.mem.u32(extra + 0x14), 0x7fc0_0001);
        e.mem.set_u32(extra + 0x14, 0x7f80_0002);
        assert_eq!(
            e.call(0x0041f020, &args![list]).f32().to_bits(),
            0x7fc0_0002
        );
    }

    // 0041f050, 0041f080: the decal references.

    #[test]
    fn decal_refs_getter_gives_the_inline_list_or_null() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041f050, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x57);
        assert_eq!(e.call(0x0041f050, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn add_decal_ref_builds_the_extra_data_and_passes_the_points_on() {
        let mut e = engine_b();
        e.register(DECAL_REFS_ADD, |_, _| Ret::default());
        let list = new_list(&mut e);
        let intersect = e.mem.alloc(12);
        let normal = e.mem.alloc(12);
        // A null reference does nothing.
        let log = logged(&mut e, |e| {
            e.call(0x0041f080, &args![list, 0u32, intersect, normal]);
        });
        assert_eq!(log.len(), 1);
        let log = logged(&mut e, |e| {
            e.call(0x0041f080, &args![list, 0x6001u32, intersect, normal]);
        });
        let extra = extra_of(&e, list, 0x57);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x14]]);
        assert_eq!(
            calls_to(&log, DECAL_REFS_ADD),
            vec![vec![extra, 0x6001, intersect, normal]]
        );
        // With the extra data there, it is not built again.
        let log = logged(&mut e, |e| {
            e.call(0x0041f080, &args![list, 0x6002u32, intersect, normal]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(
            calls_to(&log, DECAL_REFS_ADD),
            vec![vec![extra, 0x6002, intersect, normal]]
        );
    }

    // 0041f140, 0041f170, 0041f7e0, 0041f810: the reference list getters.

    #[test]
    fn reflected_refs_getter_gives_the_inline_list_or_null() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041f140, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x65);
        assert_eq!(e.call(0x0041f140, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn reflector_refs_getter_gives_the_inline_list_or_null() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041f170, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x66);
        assert_eq!(e.call(0x0041f170, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn water_light_refs_getter_gives_the_inline_list_or_null() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041f7e0, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x84);
        assert_eq!(e.call(0x0041f7e0, &args![list]).u32(), extra + 0x0c);
    }

    #[test]
    fn lit_water_refs_getter_gives_the_inline_list_or_null() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041f810, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x85);
        assert_eq!(e.call(0x0041f810, &args![list]).u32(), extra + 0x0c);
    }

    // 0041f1a0, 0041f330, 0041f4c0, 0041f650: the reflector and reflected
    // flag setters, one body for the four.

    /// The branches of the shared body for the function at `address`, which
    /// works on extra data `extra_type` and the flag `bit`.
    fn check_reflect_flag_setter(address: u32, extra_type: u8, bit: u32) {
        let mut e = engine_b();
        e.register(REFLECT_REFS_REMOVE, |e, a| {
            // Takes the only entry out of the list.
            e.mem.set_u32(a[0] + 0x0c, 0);
            Ret::default()
        });
        e.register(REFLECT_REFS_SET_FLAGS, |_, _| Ret::default());
        let list = new_list(&mut e);
        let reference = 0x7001u32;
        let set = |e: &mut Engine, value: u32| {
            logged(e, |e| {
                e.call(address, &args![list, reference, value]);
            })
        };
        // Clearing with no extra data: nothing.
        let log = set(&mut e, 0);
        assert_eq!(log.len(), 2);
        // Setting with no extra data: built (0x14 bytes), added, and the
        // entry added with the bit as flags.
        let log = set(&mut e, 1);
        let extra = extra_of(&e, list, extra_type);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x14]]);
        assert_eq!(
            calls_to(&log, REFLECT_REFS_SET_FLAGS),
            vec![vec![extra, reference, bit]]
        );
        // Setting with an extra data that has no entry for the reference:
        // not built again, the entry added.
        put_head_entry(&mut e, extra, 0x7777);
        let log = set(&mut e, 1);
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(calls_to(&log, REFLECT_REFS_SET_FLAGS).len(), 1);
        // Clearing with no entry for the reference: nothing.
        let log = set(&mut e, 0);
        assert!(calls_to(&log, REFLECT_REFS_REMOVE).is_empty());
        assert!(calls_to(&log, REMOVE_EXTRA).is_empty());
        // A null first entry ends the walk even if more follow.
        let behind = e.mem.alloc(8);
        let wanted = e.mem.alloc(8);
        e.mem.set_u32(wanted, reference);
        e.mem.set_u32(behind, wanted);
        e.mem.set_u32(extra + 0x0c, 0);
        e.mem.set_u32(extra + 0x0c + 4, behind);
        let log = set(&mut e, 1);
        assert_eq!(calls_to(&log, REFLECT_REFS_SET_FLAGS).len(), 1);
        assert_eq!(e.mem.u32(wanted + 4), 0);
        // The entry is there: setting ors the bit in, no entry is added.
        let entry = put_head_entry(&mut e, extra, reference);
        e.mem.set_u32(extra + 0x0c + 4, 0);
        e.mem.set_u32(entry + 4, 0x10);
        let log = set(&mut e, 1);
        assert!(calls_to(&log, REFLECT_REFS_SET_FLAGS).is_empty());
        assert_eq!(e.mem.u32(entry + 4), 0x10 | bit);
        // Clearing removes the bit and keeps the entry while other flags
        // are left.
        let log = set(&mut e, 0);
        assert!(calls_to(&log, REFLECT_REFS_REMOVE).is_empty());
        assert_eq!(e.mem.u32(entry + 4), 0x10);
        // The last flag gone: the entry removed, then the empty extra data.
        e.mem.set_u32(entry + 4, bit);
        let log = set(&mut e, 0);
        assert_eq!(
            calls_to(&log, REFLECT_REFS_REMOVE),
            vec![vec![extra, reference]]
        );
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), extra, 1]]
        );
        assert_eq!(extra_of(&e, list, extra_type), 0);
    }

    #[test]
    fn reflector_flag_zero_setter() {
        check_reflect_flag_setter(0x0041f1a0, 0x66, 1);
    }

    #[test]
    fn reflector_flag_one_setter() {
        check_reflect_flag_setter(0x0041f330, 0x66, 2);
    }

    #[test]
    fn reflected_flag_zero_setter() {
        check_reflect_flag_setter(0x0041f4c0, 0x65, 1);
    }

    #[test]
    fn reflected_flag_one_setter() {
        check_reflect_flag_setter(0x0041f650, 0x65, 2);
    }

    #[test]
    fn reflect_setters_keep_the_other_bit_of_a_shared_entry() {
        // Both flag setters of a class work on the same entry.
        let mut e = engine_b();
        e.register(REFLECT_REFS_REMOVE, |_, _| Ret::default());
        let list = new_list(&mut e);
        let extra = put_extra(&mut e, list, 0x66);
        let entry = put_head_entry(&mut e, extra, 0x7001);
        e.call(0x0041f1a0, &args![list, 0x7001u32, 1u32]);
        e.call(0x0041f330, &args![list, 0x7001u32, 1u32]);
        assert_eq!(e.mem.u32(entry + 4), 3);
        e.call(0x0041f1a0, &args![list, 0x7001u32, 0u32]);
        assert_eq!(e.mem.u32(entry + 4), 2);
    }

    // 0041f840, 0041f940: the water light and lit water setters.

    /// The branches of the shared body for the function at `address`, which
    /// works on extra data `extra_type` (constructor double of that type in
    /// `engine_b`), deleting the empty extra data after a removal when
    /// `deletes_when_empty`.
    fn check_reference_list_setter(address: u32, extra_type: u8, deletes_when_empty: bool) {
        let mut e = engine_b();
        let list = new_list(&mut e);
        let reference = 0x8001u32;
        let call = |e: &mut Engine, add: u32| {
            logged(e, |e| {
                e.call(address, &args![list, reference, add]);
            })
        };
        // Removing with no extra data: nothing.
        let log = call(&mut e, 0);
        assert_eq!(log.len(), 2);
        // Adding with none: built (0x14 bytes), added, the reference put in
        // its list through a local (the list calls get its address).
        let log = call(&mut e, 1);
        let extra = extra_of(&e, list, extra_type);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x14]]);
        let added = calls_to(&log, LIST_ADD_HEAD);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0][0], extra + 0x0c);
        assert_eq!(e.mem.u32(extra + 0x0c), reference);
        // Adding what the list holds: only the lookup.
        let log = call(&mut e, 1);
        assert_eq!(calls_to(&log, LIST_CONTAINS).len(), 1);
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        // Removing it: the list call gets the head and the address of a
        // local that holds the reference.
        let log = call(&mut e, 0);
        let removed = calls_to(&log, LIST_REMOVE_ITEM);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0][0], extra + 0x0c);
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA).len(),
            deletes_when_empty as usize
        );
        assert_eq!(extra_of(&e, list, extra_type) == 0, deletes_when_empty);
        // Adding to an extra data that is there, not holding it.
        let extra = put_extra(&mut e, list, extra_type);
        e.mem.set_u32(extra + 0x0c, 0x8002);
        let log = call(&mut e, 1);
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(calls_to(&log, LIST_ADD_HEAD).len(), 1);
        // Removing from a list that is not left empty keeps the extra data.
        e.mem.set_u32(extra + 0x0c, 0x8002);
        e.mem.set_u32(extra + 0x0c + 4, 0x1234);
        let log = call(&mut e, 0);
        assert!(calls_to(&log, REMOVE_EXTRA).is_empty());
    }

    #[test]
    fn water_light_ref_setter_never_deletes_the_extra_data() {
        check_reference_list_setter(0x0041f840, 0x84, false);
    }

    #[test]
    fn lit_water_ref_setter_deletes_the_empty_extra_data() {
        check_reference_list_setter(0x0041f940, 0x85, true);
    }

    // 0041fa60, 0041faf0, 0041fb20, 0041fb50, 0041fbe0: the primitive.

    const PRIMITIVE_DESTRUCTOR: u32 = 0x00a0_0100;
    const PRIMITIVE_VTABLE: u32 = 0x00a0_0000;

    /// A fake primitive whose vtable has a scalar deleting destructor
    /// double at slot 0.
    fn fake_primitive(e: &mut Engine) -> u32 {
        e.put_vtable(PRIMITIVE_VTABLE, &[PRIMITIVE_DESTRUCTOR]);
        e.register(PRIMITIVE_DESTRUCTOR, |_, a| returns(a[0]));
        let primitive = e.mem.alloc(0x20);
        e.mem.set_u32(primitive, PRIMITIVE_VTABLE);
        primitive
    }

    #[test]
    fn add_primitive_builds_an_extra_primitive_owning_it() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        let log = logged(&mut e, |e| {
            e.call(0x0041fa60, &args![list, 0u32]);
        });
        assert_eq!(log.len(), 1);
        let log = logged(&mut e, |e| {
            e.call(0x0041fa60, &args![list, 0x9001u32]);
        });
        let extra = extra_of(&e, list, 0x6b);
        assert_ne!(extra, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(e.mem.u32(extra), 0x0101_51b4);
        assert_eq!(e.mem.u32(extra + 0x0c), 0x9001);
        assert_eq!(calls_to(&log, ADD_EXTRA), vec![vec![list.addr(), extra]]);
        // An existing one is not looked for: another is built.
        e.call(0x0041fa60, &args![list, 0x9002u32]);
        assert_eq!(e.mem.u32(extra_of(&e, list, 0x6b) + 0x0c), 0x9002);
    }

    #[test]
    fn extra_primitive_constructor() {
        let mut e = engine_b();
        let extra: Ptr<ExtraPrimitive> = e.new_object();
        let log = logged(&mut e, |e| {
            assert_eq!(
                e.call(0x0041faf0, &args![extra, 0x9001u32]).u32(),
                extra.addr()
            );
        });
        assert_eq!(
            calls_to(&log, BS_EXTRA_DATA_INIT),
            vec![vec![extra.addr(), 0x6b]]
        );
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_51b4);
        assert_eq!(e.get(extra, ExtraPrimitive::pPrimitive).addr(), 0x9001);
    }

    #[test]
    fn extra_primitive_destructor_deletes_the_primitive_it_owns() {
        let mut e = engine_b();
        let primitive = fake_primitive(&mut e);
        let extra: Ptr<ExtraPrimitive> = e.new_object();
        e.set(extra, ExtraPrimitive::pPrimitive, Ptr::new(primitive));
        let log = logged(&mut e, |e| {
            e.call(0x0041fb50, &args![extra]);
        });
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_51b4);
        assert_eq!(
            calls_to(&log, PRIMITIVE_DESTRUCTOR),
            vec![vec![primitive, 1]]
        );
        assert_eq!(
            calls_to(&log, BS_EXTRA_DATA_DESTROY),
            vec![vec![extra.addr()]]
        );
        // Without a primitive only the base destructor runs.
        e.set(extra, ExtraPrimitive::pPrimitive, Ptr::NULL);
        let log = logged(&mut e, |e| {
            e.call(0x0041fb50, &args![extra]);
        });
        assert!(calls_to(&log, PRIMITIVE_DESTRUCTOR).is_empty());
        assert_eq!(calls_to(&log, BS_EXTRA_DATA_DESTROY).len(), 1);
    }

    #[test]
    fn extra_primitive_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = engine_b();
        let extra: Ptr<ExtraPrimitive> = e.new_object();
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041fb20, &args![extra, 0u32]).u32(), extra.addr());
        });
        assert_eq!(calls_to(&log, BS_EXTRA_DATA_DESTROY).len(), 1);
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041fb20, &args![extra, 3u32]).u32(), extra.addr());
        });
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![extra.addr()]]);
    }

    #[test]
    fn primitive_getter() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041fbe0, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x6b);
        e.mem.set_u32(extra + 0x0c, 0x9001);
        assert_eq!(e.call(0x0041fbe0, &args![list]).u32(), 0x9001);
    }

    // 0041fc10, 0041fcd0, 0041fd20, 0041fd80, 0041fdb0, 0041fe10, 0041fe60,
    // 0041fe90: the patrol reference data.

    #[test]
    fn patrol_ref_data_setter_replaces_the_extra_data() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        // Null with none: nothing built.
        let log = logged(&mut e, |e| {
            e.call(0x0041fc10, &args![list, 0u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        // Data: built (0x10 bytes), given the data, added.
        let log = logged(&mut e, |e| {
            e.call(0x0041fc10, &args![list, 0xa001u32]);
        });
        let first = extra_of(&e, list, 0x6f);
        assert_ne!(first, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(e.mem.u32(first), 0x0101_51c0);
        assert_eq!(e.mem.u32(first + 0x0c), 0xa001);
        assert_eq!(calls_to(&log, STORE_WORD_AT_0C), vec![vec![first, 0xa001]]);
        // New data: the old extra data is deleted (the object), a new one
        // built.
        let log = logged(&mut e, |e| {
            e.call(0x0041fc10, &args![list, 0xa002u32]);
        });
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), first, 1]]
        );
        let second = extra_of(&e, list, 0x6f);
        assert_eq!(e.mem.u32(second + 0x0c), 0xa002);
        // Null: only the deletion.
        let log = logged(&mut e, |e| {
            e.call(0x0041fc10, &args![list, 0u32]);
        });
        assert_eq!(calls_to(&log, REMOVE_EXTRA).len(), 1);
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(extra_of(&e, list, 0x6f), 0);
    }

    #[test]
    fn patrol_ref_data_constructor_leaves_the_data_word() {
        let mut e = engine_b();
        let extra: Ptr<ExtraPatrolRefData> = e.new_object();
        e.set(extra, ExtraPatrolRefData::pPatrolData, Ptr::new(0xa5a5));
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041fcd0, &args![extra]).u32(), extra.addr());
        });
        assert_eq!(
            calls_to(&log, BS_EXTRA_DATA_INIT),
            vec![vec![extra.addr(), 0x6f]]
        );
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_51c0);
        assert_eq!(e.get(extra, ExtraPatrolRefData::pPatrolData).addr(), 0xa5a5);
    }

    #[test]
    fn patrol_ref_data_compare_decides_by_presence_then_by_the_data() {
        let mut e = engine_b();
        e.register(PATROL_DATA_COMPARE, |e, a| {
            returns((e.mem.u32(a[0]) != e.mem.u32(a[1])) as u32)
        });
        let this: Ptr<ExtraPatrolRefData> = e.new_object();
        let other: Ptr<ExtraPatrolRefData> = e.new_object();
        // Neither has data: equal.
        assert!(!e.call(0x0041fd20, &args![this, other]).bool());
        // Only one has: different.
        let data = e.mem.alloc(4);
        e.set(this, ExtraPatrolRefData::pPatrolData, Ptr::new(data));
        assert!(e.call(0x0041fd20, &args![this, other]).bool());
        e.set(this, ExtraPatrolRefData::pPatrolData, Ptr::NULL);
        e.set(other, ExtraPatrolRefData::pPatrolData, Ptr::new(data));
        assert!(e.call(0x0041fd20, &args![this, other]).bool());
        // Both: the patrol data compare gets this's data as `this` and
        // other's as its argument, and decides.
        let same = e.mem.alloc(4);
        e.set(this, ExtraPatrolRefData::pPatrolData, Ptr::new(same));
        e.mem.set_u32(same, 7);
        e.mem.set_u32(data, 7);
        let log = logged(&mut e, |e| {
            assert!(!e.call(0x0041fd20, &args![this, other]).bool());
        });
        assert_eq!(calls_to(&log, PATROL_DATA_COMPARE), vec![vec![same, data]]);
        e.mem.set_u32(data, 8);
        assert!(e.call(0x0041fd20, &args![this, other]).bool());
    }

    #[test]
    fn patrol_ref_data_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = engine_b();
        e.register(PATROL_DATA_DESTROY, |_, _| Ret::default());
        let extra: Ptr<ExtraPatrolRefData> = e.new_object();
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041fd80, &args![extra, 0u32]).u32(), extra.addr());
        });
        assert_eq!(calls_to(&log, BS_EXTRA_DATA_DESTROY).len(), 1);
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041fd80, &args![extra, 1u32]).u32(), extra.addr());
        });
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![extra.addr()]]);
    }

    #[test]
    fn patrol_ref_data_destructor_deletes_the_data_then_the_base() {
        let mut e = engine_b();
        e.register(PATROL_DATA_DESTROY, |_, _| Ret::default());
        let extra: Ptr<ExtraPatrolRefData> = e.new_object();
        let data = e.mem.alloc(0x20);
        e.set(extra, ExtraPatrolRefData::pPatrolData, Ptr::new(data));
        let log = logged(&mut e, |e| {
            e.call(0x0041fdb0, &args![extra]);
        });
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_51c0);
        assert_eq!(calls_to(&log, PATROL_DATA_DESTROY), vec![vec![data]]);
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![data]]);
        assert_eq!(e.get(extra, ExtraPatrolRefData::pPatrolData).addr(), 0);
        let order: Vec<u32> = log.iter().map(|(callee, _)| *callee).collect();
        assert_eq!(
            order,
            vec![
                0x0041fdb0,
                PATROL_DATA_DESTROY,
                OPERATOR_DELETE,
                BS_EXTRA_DATA_DESTROY
            ]
        );
    }

    #[test]
    fn patrol_data_cleanup_does_nothing_without_data() {
        let mut e = engine_b();
        e.register(PATROL_DATA_DESTROY, |_, _| Ret::default());
        let extra: Ptr<ExtraPatrolRefData> = e.new_object();
        let log = logged(&mut e, |e| {
            e.call(0x0041fe10, &args![extra]);
        });
        assert_eq!(log.len(), 1);
        let data = e.mem.alloc(0x20);
        e.set(extra, ExtraPatrolRefData::pPatrolData, Ptr::new(data));
        let log = logged(&mut e, |e| {
            e.call(0x0041fe10, &args![extra]);
        });
        assert_eq!(calls_to(&log, PATROL_DATA_DESTROY).len(), 1);
        assert_eq!(e.get(extra, ExtraPatrolRefData::pPatrolData).addr(), 0);
    }

    #[test]
    fn patrol_data_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = engine_b();
        e.register(PATROL_DATA_DESTROY, |_, _| Ret::default());
        let data = e.mem.alloc(0x20);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041fe60, &args![data, 0u32]).u32(), data);
        });
        assert_eq!(calls_to(&log, PATROL_DATA_DESTROY), vec![vec![data]]);
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041fe60, &args![data, 1u32]).u32(), data);
        });
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![data]]);
    }

    #[test]
    fn patrol_ref_data_getter() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x0041fe90, &args![list]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x6f);
        e.mem.set_u32(extra + 0x0c, 0xa001);
        assert_eq!(e.call(0x0041fe90, &args![list]).u32(), 0xa001);
    }

    // 0041fec0, 0041ff80, 0041ffb0, 00420030, 00420060, 004200c0, 00420100,
    // 00420130, 00420160: the occlusion plane reference data.

    #[test]
    fn occlusion_plane_ref_data_setter_replaces_the_extra_data() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        let log = logged(&mut e, |e| {
            e.call(0x0041fec0, &args![list, 0u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        let log = logged(&mut e, |e| {
            e.call(0x0041fec0, &args![list, 0xb001u32]);
        });
        let first = extra_of(&e, list, 0x76);
        assert_ne!(first, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(e.mem.u32(first), 0x0101_51cc);
        assert_eq!(e.mem.u32(first + 0x0c), 0xb001);
        let log = logged(&mut e, |e| {
            e.call(0x0041fec0, &args![list, 0xb002u32]);
        });
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), first, 1]]
        );
        let second = extra_of(&e, list, 0x76);
        assert_eq!(e.mem.u32(second + 0x0c), 0xb002);
        e.call(0x0041fec0, &args![list, 0u32]);
        assert_eq!(extra_of(&e, list, 0x76), 0);
    }

    #[test]
    fn occlusion_plane_ref_data_constructor() {
        let mut e = engine_b();
        let extra: Ptr<ExtraOcclusionPlaneRefData> = e.new_object();
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0041ff80, &args![extra]).u32(), extra.addr());
        });
        assert_eq!(
            calls_to(&log, BS_EXTRA_DATA_INIT),
            vec![vec![extra.addr(), 0x76]]
        );
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_51cc);
    }

    #[test]
    fn occlusion_plane_ref_data_compare_looks_at_four_words() {
        let mut e = engine_b();
        let this: Ptr<ExtraOcclusionPlaneRefData> = e.new_object();
        let other: Ptr<ExtraOcclusionPlaneRefData> = e.new_object();
        assert!(!e.call(0x0041ffb0, &args![this, other]).bool());
        let mine = e.mem.alloc(0x14);
        let theirs = e.mem.alloc(0x14);
        e.set(this, ExtraOcclusionPlaneRefData::pData, Ptr::new(mine));
        assert!(e.call(0x0041ffb0, &args![this, other]).bool());
        e.set(this, ExtraOcclusionPlaneRefData::pData, Ptr::NULL);
        e.set(other, ExtraOcclusionPlaneRefData::pData, Ptr::new(theirs));
        assert!(e.call(0x0041ffb0, &args![this, other]).bool());
        e.set(this, ExtraOcclusionPlaneRefData::pData, Ptr::new(mine));
        assert!(!e.call(0x0041ffb0, &args![this, other]).bool());
        // Each of the four words counts, the fifth does not.
        for word in 0..4 {
            e.mem.set_u32(theirs + 4 * word, 1);
            assert!(e.call(0x0041ffb0, &args![this, other]).bool());
            e.mem.set_u32(theirs + 4 * word, 0);
        }
        e.mem.set_u32(theirs + 0x10, 1);
        assert!(!e.call(0x0041ffb0, &args![this, other]).bool());
    }

    #[test]
    fn occlusion_plane_ref_data_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = engine_b();
        let extra: Ptr<ExtraOcclusionPlaneRefData> = e.new_object();
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x00420030, &args![extra, 0u32]).u32(), extra.addr());
        });
        assert_eq!(calls_to(&log, BS_EXTRA_DATA_DESTROY).len(), 1);
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x00420030, &args![extra, 1u32]).u32(), extra.addr());
        });
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![extra.addr()]]);
    }

    #[test]
    fn occlusion_plane_ref_data_destructor_frees_the_data_then_the_base() {
        let mut e = engine_b();
        let extra: Ptr<ExtraOcclusionPlaneRefData> = e.new_object();
        let data = e.mem.alloc(0x10);
        e.set(extra, ExtraOcclusionPlaneRefData::pData, Ptr::new(data));
        let log = logged(&mut e, |e| {
            e.call(0x00420060, &args![extra]);
        });
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_51cc);
        let order: Vec<u32> = log.iter().map(|(callee, _)| *callee).collect();
        assert_eq!(
            order,
            vec![0x00420060, OPERATOR_DELETE, BS_EXTRA_DATA_DESTROY]
        );
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![data]]);
        assert_eq!(e.get(extra, ExtraOcclusionPlaneRefData::pData).addr(), 0);
    }

    #[test]
    fn occlusion_plane_data_cleanup_frees_only_what_is_there() {
        let mut e = engine_b();
        let extra: Ptr<ExtraOcclusionPlaneRefData> = e.new_object();
        let log = logged(&mut e, |e| {
            e.call(0x004200c0, &args![extra]);
        });
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());
        let data = e.mem.alloc(0x10);
        e.set(extra, ExtraOcclusionPlaneRefData::pData, Ptr::new(data));
        let log = logged(&mut e, |e| {
            e.call(0x004200c0, &args![extra]);
        });
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![data]]);
        assert_eq!(e.get(extra, ExtraOcclusionPlaneRefData::pData).addr(), 0);
    }

    #[test]
    fn occlusion_plane_data_getters() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        assert_eq!(e.call(0x00420100, &args![list]).u32(), 0);
        assert_eq!(e.call(0x00420130, &args![list, 2u32]).u32(), 0);
        let extra = put_extra(&mut e, list, 0x76);
        // An extra data without data gives 0 too.
        assert_eq!(e.call(0x00420130, &args![list, 2u32]).u32(), 0);
        let data = e.mem.alloc(0x10);
        e.mem.set_u32(extra + 0x0c, data);
        e.mem.set_u32(data + 8, 0xc002);
        assert_eq!(e.call(0x00420100, &args![list]).u32(), data);
        assert_eq!(e.call(0x00420130, &args![list, 2u32]).u32(), 0xc002);
        assert_eq!(e.call(0x00420130, &args![list, 0u32]).u32(), 0);
    }

    #[test]
    fn occlusion_plane_word_setter_builds_the_data_for_a_reference() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        // A null reference with no data: nothing built, nothing stored.
        let log = logged(&mut e, |e| {
            e.call(0x00420160, &args![list, 0u32, 1u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(extra_of(&e, list, 0x76), 0);
        // A reference: the four-word data is built (0x10 bytes,
        // constructor 00411dc0), set through the setter of this file, and
        // the word stored.
        let log = logged(&mut e, |e| {
            e.call(0x00420160, &args![list, 0xc001u32, 3u32]);
        });
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10], vec![0x10]]);
        assert_eq!(calls_to(&log, OCCLUSION_PLANE_DATA_INIT).len(), 1);
        let extra = extra_of(&e, list, 0x76);
        let data = e.mem.u32(extra + 0x0c);
        assert_ne!(data, 0);
        assert_eq!(e.mem.u32(data + 0x0c), 0xc001);
        // With data there: only the word is stored, a null reference
        // clears it.
        let log = logged(&mut e, |e| {
            e.call(0x00420160, &args![list, 0xc002u32, 0u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(e.mem.u32(data), 0xc002);
        assert_eq!(e.mem.u32(data + 0x0c), 0xc001);
        e.call(0x00420160, &args![list, 0u32, 3u32]);
        assert_eq!(e.mem.u32(data + 0x0c), 0);
    }

    // 00420210: the portal reference data setter.

    #[test]
    fn portal_ref_data_setter_replaces_the_extra_data() {
        let mut e = engine_b();
        let list = new_list(&mut e);
        let log = logged(&mut e, |e| {
            e.call(0x00420210, &args![list, 0u32]);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        let log = logged(&mut e, |e| {
            e.call(0x00420210, &args![list, 0xd001u32]);
        });
        let first = extra_of(&e, list, 0x77);
        assert_ne!(first, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(calls_to(&log, PORTAL_REF_DATA_INIT).len(), 1);
        assert_eq!(e.mem.u32(first + 0x0c), 0xd001);
        let log = logged(&mut e, |e| {
            e.call(0x00420210, &args![list, 0xd002u32]);
        });
        assert_eq!(
            calls_to(&log, REMOVE_EXTRA),
            vec![vec![list.addr(), first, 1]]
        );
        assert_eq!(e.mem.u32(extra_of(&e, list, 0x77) + 0x0c), 0xd002);
        e.call(0x00420210, &args![list, 0u32]);
        assert_eq!(extra_of(&e, list, 0x77), 0);
    }
}
