//! `fallout shared/tesobjectrefr.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! # Layout of a reference on the PC
//!
//! A `TESObjectREFR` is 0x68 bytes on the PC (0x78 on the Xbox): the Xbox PDB
//! offsets of everything after `TESForm` are 0x10 lower here, because the PC
//! `TESForm` is 0x18 bytes (no editor ID, no version-control fields). The
//! `TESChildCell` base is only a vtable pointer at +0x18. The embedded
//! `ExtraDataList` (`m_Extra`) is at +0x44; the game reaches it through
//! `005d43c0` ([`GET_EXTRA_LIST`], `this + 0x44`, also used on other
//! references), and so do the translations.
//!
//! Fields of `TESForm` read here are read at their PC offset with a comment
//! (the form's own unit declares them): `iFormFlags` +0x08, `iFormID` +0x0C.
//!
//! # Where this file stops
//!
//! This file holds the first 40 functions the queue listed as open or traced
//! (`00426720` to `0055e730`; `0055e230`, the player's running speed, is
//! already translated in `crates/physics`). The next session continues with
//! the next open function after `0055e730` in address order.
//!
//! Several functions in this range are not `TESObjectREFR` methods but small
//! `ExtraDataList`, list-node and manager helpers the linker placed in this
//! unit; their `this` is documented per function. Callees outside this
//! translation (including the unit's own later functions, such as
//! `005653d0 GetRefPersists`) are called by address.
//!
//! The vtable slots called here (`vcall`) are PC offsets taken from the
//! code; the PC vtable has one more entry than the Xbox one between `+0x130`
//! and `+0x1c4`, so slot names from the Xbox PDB are quoted only where the
//! PC body matches.
//!
//! # Compiled-out code
//!
//! Some functions test `flags & 0` (a debug feature compiled out); those
//! branches never run and are not translated.

#[allow(unused_imports)]
use crate::prelude::*;

/// `TESObjectREFR`'s vtable (`0102f55c`) and `TESChildCell`'s (`0102f550`).
const REFR_VTABLE: u32 = 0x0102_f55c;
const CHILD_CELL_VTABLE: u32 = 0x0102_f550;

/// `this + 0x44`: the reference's embedded `ExtraDataList` (also called on
/// other references and on list nodes; the game never inlines it).
const GET_EXTRA_LIST: u32 = 0x005d_43c0;
/// `__RTDynamicCast(object, 0, from, to, 0)` (cdecl).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// RTTI type descriptors the casts use (`.?AVTESForm@@` and so on).
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const TYPE_TES_BOUND_OBJECT: u32 = 0x0118_3108;
const TYPE_TES_OBJECT_CELL: u32 = 0x0118_3fb4;
const TYPE_TES_CHILD_CELL: u32 = 0x0118_ac2c;
const TYPE_BS_EXTRA_DATA: u32 = 0x0118_3b2c;
const TYPE_EXTRA_TELEPORT: u32 = 0x0118_4430;
const TYPE_EXTRA_ENABLE_STATE_PARENT: u32 = 0x0118_4d7c;
const TYPE_EXTRA_RANDOM_TELEPORT_MARKER: u32 = 0x0118_4dcc;
const TYPE_EXTRA_LINKED_REF: u32 = 0x0118_4e1c;
const TYPE_EXTRA_ACTIVATE_REF: u32 = 0x0118_4e84;
const TYPE_EXTRA_DECAL_REFS: u32 = 0x0118_4ea4;
const TYPE_EXTRA_MERCHANT_CONTAINER: u32 = 0x0118_4ec4;
const TYPE_EXTRA_MULTI_BOUND_REF: u32 = 0x0118_4f50;

/// `BaseExtraList` methods (the receiver is the list, `this + 0x44` of a
/// reference).
const EXTRA_GET_DATA: u32 = 0x0041_0220; // GetExtraData(type) -> BSExtraData*
const EXTRA_REMOVE: u32 = 0x0041_0020; // RemoveExtra(extra, free)
const EXTRA_ADD: u32 = 0x0040_ff60; // AddExtra(extra)
const EXTRA_REMOVE_ALL: u32 = 0x0040_fae0; // RemoveAll(keep_defaults)
const EXTRA_REMOVE_ALL_DEFAULT: u32 = 0x0040_fcb0; // RemoveAllDefault(keep_defaults)
const EXTRA_REMOVE_TYPE: u32 = 0x0041_0140; // RemoveExtra_ov2(type)
const EXTRA_GET_PERSISTENT_CELL: u32 = 0x0041_d460;
const EXTRA_SET_PERSISTENT_CELL: u32 = 0x0041_d390;
/// `TESObjectCELL::RemoveReference(cell, ref)`.
const CELL_REMOVE_REFERENCE: u32 = 0x0054_ca90;
/// `BSSimpleList` helpers: the node's item slot (identity on the node), the
/// next node, whether a node is empty.
const LIST_ITEM_SLOT: u32 = 0x0068_15c0;
const LIST_NEXT: u32 = 0x0072_6070;
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// Removes the item at a pointer from a `BSSimpleList` (`list`, `&item`).
const LIST_REMOVE: u32 = 0x0090_5330;
/// `*(u32 *)this`: the first dword of a pointer wrapper or `BSStringT`.
const READ_FIRST_DWORD: u32 = 0x0055_9450;
/// `TESForm` accessors.
const FORM_TYPE: u32 = 0x0040_1170; // cFormType (byte at +4)
const FORM_ID: u32 = 0x0084_e3a0; // iFormID (+0x0C)
const FORM_IS_DELETED: u32 = 0x0044_0d80; // iFormFlags & 0x20
const FORM_IS_DISABLED: u32 = 0x0044_0da0; // iFormFlags & 0x800
/// The reference's base object (`data.pObjectReference`, +0x20; the map
/// names the body `BGSSaveFormBuffer::GetForm (Xbox PDB)`).
const GET_BASE_FORM: u32 = 0x007a_f430;
/// The reference's parent cell (`pParentCell`, +0x40).
const GET_PARENT_CELL: u32 = 0x008d_6f30;
/// `&this->data` (`OBJ_REFR`, +0x20).
const GET_DATA: u32 = 0x0089_1170;
/// Constructor/destructor of the 8-byte `BSStringT` the game keeps on its
/// stack, and `BSStringT::Format(fmt, ...)`.
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
const STRING_FORMAT: u32 = 0x0040_6f60;
/// The CRT (called by address): `sprintf`, `memcmp`, `memset` (via the
/// wrapper at `00403d30`), `_finite` and `_isnan` (which take a `double`).
const SPRINTF: u32 = 0x00ec_623a;
const MEMCMP: u32 = 0x00ec_4835;
const MEMSET: u32 = 0x0040_3d30;
const CRT_FINITE: u32 = 0x00ec_7595;
const CRT_ISNAN: u32 = 0x00ec_75b1;
/// The error logger (`Error(fmt, ...)`) and the message log (`005b5e40`).
const ERROR_LOG: u32 = 0x0040_fbe0;
const MESSAGE: u32 = 0x005b_5e40;
/// `TESObjectREFR::GetRefPersists` (translated later in this unit).
const GET_REF_PERSISTS: u32 = 0x0056_53d0;
/// Whether the current thread's loading flag (TLS +0x294 bit 2) is set; the
/// game calls it on `011ddf38`'s object (it does not use `this`).
const LOADING_FLAG: u32 = 0x0041_21b0;

/// Writers of the save record (`TESForm::AddChunk (value)` and
/// `__AddChunkData`), whether the build swaps endianness on save, and the
/// double `1.0`.
const ADD_CHUNK_VALUE: u32 = 0x0048_5910;
const ADD_CHUNK_DATA: u32 = 0x0048_5990;
const ENDIAN_SWAP_ON_SAVE: u32 = 0x0040_1500;
const CONSTANT_ONE: u32 = 0x0101_2070;

/// Globals the code reads.
const GLOBAL_PLAYER: u32 = 0x011d_ea3c; // PlayerCharacter *
const GLOBAL_SAVE_LOAD: u32 = 0x011d_e45c; // TESSaveLoadGame *
const GLOBAL_DATA_HANDLER: u32 = 0x011c_3f2c; // TESDataHandler *
const GLOBAL_LOADING_OBJECT: u32 = 0x011d_df38; // object whose 4121b0 flag says "loading"
const GLOBAL_TEMP_REFR_MANAGER: u32 = 0x011d_ea10;
const GLOBAL_LAST_REFR: u32 = 0x011d_ea24; // reference remembered by 0055ac70/0055ac80
const GLOBAL_STORED_BY_825C00: u32 = 0x011c_a28c;
const GLOBAL_IO_MANAGER: u32 = 0x0120_2d98;
const GLOBAL_DEBUG_SWITCHES: u32 = 0x011d_e4e8; // an object, passed as `this`
/// Static objects passed as `this`.
const OBJECT_PROCESS_LISTS: u32 = 0x011e_0e80; // ProcessLists
const OBJECT_FURNITURE_MANAGER: u32 = 0x011e_043c;
const OBJECT_ACTOR_LISTS: u32 = 0x011f_6394;
/// Getters (in `teswater.cpp`) that return the addresses of two global
/// lists (`011ca13c`, `011ca144`) the destructor removes the reference from.
const WATER_LIST_A_GETTER: u32 = 0x004e_3260;
const WATER_LIST_B_GETTER: u32 = 0x004e_c7b0;
/// The `float` the loaded-data water height is reset to.
const LOADED_DATA_DEFAULT_HEIGHT: u32 = 0x0101_5f5c;
/// The zero vector (three dwords) at `011f426c`.
const ZERO_VECTOR: u32 = 0x011f_426c;
/// The `float` tolerance `Copy` compares radii with (0.001).
const RADIUS_EPSILON: u32 = 0x0101_7d00;
/// The global holding the vtable address of the 0x14-byte group-data record
/// that `fn_0055e5e0` fills.
const GROUP_DATA_VTABLE_PTR: u32 = 0x0118_7020;
/// The empty string.
const EMPTY_STRING: u32 = 0x0101_1584;

/// Message and format strings in the exe.
const MISSING_BASE_FORMAT: u32 = 0x0102_f828;
const LOAD_TERMINATED_MESSAGE: u32 = 0x0102_f7ec;
const PATROL_NOT_COMPILED_FORMAT: u32 = 0x0102_f7a0;
const LEVELED_BASE_MESSAGE: u32 = 0x0102_fbb0;
const CORRUPT_LOCATION_MESSAGE: u32 = 0x0102_fb68;
const CORRUPT_ANGLE_MESSAGE: u32 = 0x0102_fb20;
const SHARED_DATA_REMOVED_MESSAGE: u32 = 0x0102_f870;
const INIT_ITEM_ERROR_CELL: u32 = 0x0102_fa78;
const INIT_ITEM_ERROR_WORLD: u32 = 0x0102_f9b0;
const INIT_ITEM_ERROR_NO_CELL: u32 = 0x0102_f910;
const DETAILED_FORMAT: u32 = 0x0102_f8b8;
const DETAILED_BASE_FORMAT: u32 = 0x0102_f8f8;
const DETAILED_WORLD_FORMAT: u32 = 0x0102_f8dc;
const SAVE_SIZE_FORMAT: u32 = 0x0101_2cb0;
const SAVE_SIZE_SHORT_FORMAT: u32 = 0x0101_2c78;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\TESObjectREFR.cpp"`
/// and the line `GetSaveSize` reports.
const SOURCE_FILE: u32 = 0x0102_fc20;
const SOURCE_LINE: u32 = 0x0ad0;

/// A four-character record or chunk tag as the game's 32-bit constant.
const fn tag(name: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*name)
}

/// The chunk kinds `Load` hands to `ExtraDataList::Load`.
const EXTRA_DATA_CHUNKS: [u32; 76] = [
    tag(b"XPPA"),
    tag(b"XRGB"),
    tag(b"XGLB"),
    tag(b"XLMB"),
    tag(b"XAMC"),
    tag(b"XLOC"),
    tag(b"XMRC"),
    tag(b"XRAD"),
    tag(b"XSED"),
    tag(b"XRGD"),
    tag(b"XLOD"),
    tag(b"XPOD"),
    tag(b"XAPD"),
    tag(b"XORD"),
    tag(b"XPRD"),
    tag(b"XSRD"),
    tag(b"XUSE"),
    tag(b"XSRF"),
    tag(b"XCHG"),
    tag(b"XTRG"),
    tag(b"XPCI"),
    tag(b"XEMI"),
    tag(b"XTRI"),
    tag(b"XRNK"),
    tag(b"AMRK"),
    tag(b"MMRK"),
    tag(b"XMRK"),
    tag(b"XEDL"),
    tag(b"XTEL"),
    tag(b"XSOL"),
    tag(b"XPSL"),
    tag(b"XPTL"),
    tag(b"RNAM"),
    tag(b"XCCM"),
    tag(b"XLCM"),
    tag(b"XCIM"),
    tag(b"XTIM"),
    tag(b"XCLM"),
    tag(b"XPRM"),
    tag(b"XRTM"),
    tag(b"XPSN"),
    tag(b"XOWN"),
    tag(b"XEZN"),
    tag(b"XMBO"),
    tag(b"XRDO"),
    tag(b"XCMO"),
    tag(b"XROO"),
    tag(b"XATO"),
    tag(b"XMBP"),
    tag(b"XOCP"),
    tag(b"XNDP"),
    tag(b"XCLP"),
    tag(b"XHLP"),
    tag(b"XESP"),
    tag(b"XNVP"),
    tag(b"XMBR"),
    tag(b"XACR"),
    tag(b"XDCR"),
    tag(b"XLKR"),
    tag(b"XCLR"),
    tag(b"XRMR"),
    tag(b"XAPR"),
    tag(b"XPWR"),
    tag(b"XCAS"),
    tag(b"XIBS"),
    tag(b"XRDS"),
    tag(b"XHRS"),
    tag(b"XACT"),
    tag(b"XCET"),
    tag(b"XHLT"),
    tag(b"XWLT"),
    tag(b"XAMT"),
    tag(b"XCMT"),
    tag(b"XCNT"),
    tag(b"XCWT"),
    tag(b"XLTW"),
];

layout! {
    /// `OBJ_REFR` (Xbox PDB), 0x1C bytes: the reference's base object,
    /// rotation and position (two `NiPoint3`).
    #[allow(non_camel_case_types)]
    pub struct OBJ_REFR: 0x1c {
        /// `pObjectReference` (Xbox PDB): `TESBoundObject*`.
        0x00 pObjectReference: Ptr,
        /// `Angle` (Xbox PDB), `NiPoint3`.
        0x04 AngleX: f32,
        0x08 AngleY: f32,
        0x0C AngleZ: f32,
        /// `Location` (Xbox PDB), `NiPoint3`.
        0x10 LocationX: f32,
        0x14 LocationY: f32,
        0x18 LocationZ: f32,
    }

    /// `LOADED_REF_DATA` (Xbox PDB), 0x1C bytes, only the fields used so far.
    #[allow(non_camel_case_types)]
    pub struct LOADED_REF_DATA: 0x1c {
        /// `pCurrentWaterObject` (Xbox PDB): `TESObjectREFR*`.
        0x00 pCurrentWaterObject: Ptr,
        /// `fRelevantWaterHeight` (Xbox PDB).
        0x08 fRelevantWaterHeight: f32,
        /// `spPhantom` (Xbox PDB): `NiPointer<bhkPhantom>`.
        0x18 spPhantom: Ptr,
    }

    /// `TESObjectREFR` (Xbox PDB), 0x68 bytes on the PC; offsets are the Xbox
    /// ones minus 0x10 (see the module notes). The `TESForm` base is the first
    /// 0x18 bytes.
    pub struct TESObjectREFR: 0x68 {
        /// Vtable pointer of the `TESChildCell` base.
        0x18 vfptrTESChildCell: u32,
        /// `pRandomSound` (Xbox PDB): `TESSound*`.
        0x1C pRandomSound: Ptr,
        /// `data` (Xbox PDB): the embedded [`OBJ_REFR`].
        0x20 data: Inline<OBJ_REFR>,
        /// `fRefScale` (Xbox PDB).
        0x3C fRefScale: f32,
        /// `pParentCell` (Xbox PDB): `TESObjectCELL*`.
        0x40 pParentCell: Ptr,
        /// `pLoadedData` (Xbox PDB): `LOADED_REF_DATA*`.
        0x64 pLoadedData: Ptr,
    }
}

/// `m_Extra` (Xbox PDB), the embedded `ExtraDataList`, at +0x44.
const EXTRA_OFFSET: u32 = 0x44;
/// `TESChildCell` base subobject, at +0x18.
const CHILD_CELL_OFFSET: u32 = 0x18;

fn extra_list(e: &mut Engine, refr: u32) -> u32 {
    e.call(GET_EXTRA_LIST, &args![refr]).u32()
}

fn dynamic_cast(e: &mut Engine, object: u32, to: u32) -> u32 {
    e.call(DYNAMIC_CAST, &args![object, 0u32, TYPE_TES_FORM, to, 0u32])
        .u32()
}

/// `IsActor` (virtual at +0x100) on `actor`, false for a null pointer: the
/// guard these helpers all start with.
fn is_actor(e: &mut Engine, actor: u32) -> bool {
    actor != 0 && e.vcall(actor, 0x100, &args![]).bool()
}

/// The cell the `TESChildCell` base reports (its virtual at slot 0).
fn save_parent_cell(e: &mut Engine, refr: Ptr<TESObjectREFR>) -> u32 {
    e.vcall(refr.addr() + CHILD_CELL_OFFSET, 0, &args![]).u32()
}

fn global_ptr(e: &Engine, addr: u32) -> u32 {
    e.global::<u32>(addr)
}

/// `BGSSaveLoadReferencesMap::Lookup(old, &new)` (`00853130`, on the map):
/// the new pointer for `old`, 0 when it has none.
fn lookup_new_reference(e: &mut Engine, map: u32, old: u32) -> u32 {
    e.with_stack(4, |e, out| {
        e.mem.set_u32(out.addr(), 0);
        let found = e.call(0x0085_3130, &args![map, old, out]).bool();
        if found {
            e.mem.u32(out.addr())
        } else {
            0
        }
    })
}

/// [`lookup_new_reference`], cast to `TESObjectREFR`; 0 when either fails.
fn resolve_reference(e: &mut Engine, map: u32, old: u32) -> u32 {
    let new = lookup_new_reference(e, map, old);
    if new == 0 {
        return 0;
    }
    dynamic_cast(e, new, TYPE_TES_OBJECT_REFR)
}

/// The body of `TESObjectREFR::Load` (`0055af90`): `id_slot` is the stack word
/// `TESFile::GetChunkData` writes the base object's form ID to.
fn load_record(e: &mut Engine, this: Ptr<TESObjectREFR>, file: u32, id_slot: u32) -> bool {
    let me = this.addr();
    let data = this.at(TESObjectREFR::data);
    e.with_stack(0x18, |e, scratch| {
        e.call(0x0069_2710, &args![scratch]);
    });
    e.call(0x0048_4ab0, &args![this, 0u32]);
    let mut base_missing = false;
    e.call(0x0048_5110, &args![this, file]);
    e.call(0x0048_4ab0, &args![this, 0u32]);

    if e.call(FORM_IS_DELETED, &args![this]).bool() {
        let cell = e
            .call(EXTRA_GET_PERSISTENT_CELL, &args![me + EXTRA_OFFSET])
            .u32();
        let loading = global_ptr(e, GLOBAL_LOADING_OBJECT);
        if loading == 0 || !e.call(LOADING_FLAG, &args![loading]).bool() {
            e.call(0x0056_b0b0, &args![this]);
        }
        e.call(EXTRA_REMOVE_ALL, &args![me + EXTRA_OFFSET, 1u32]);
        if cell != 0 {
            e.call(EXTRA_SET_PERSISTENT_CELL, &args![me + EXTRA_OFFSET, cell]);
        }
        return e.call(GET_BASE_FORM, &args![this]).u32() != 0;
    }

    loop {
        let chunk = e.call(0x0047_26b0, &args![file]).u32();
        if chunk == 0 {
            break;
        }
        if EXTRA_DATA_CHUNKS.contains(&chunk) {
            let list = extra_list(e, me);
            e.call(0x0041_44a0, &args![list, file, this]);
        } else if chunk == tag(b"EDID") {
            let size = e.call(0x0040_1660, &args![file]).u32();
            // `_alloca(chunk size)`
            e.with_stack(size.max(1), |e, text| {
                e.call(0x0047_2890, &args![file, text, 0x200u32]);
                e.vcall(me, 0x134, &args![text]);
            });
        } else if chunk == tag(b"OBND") {
            e.vcall(me, 0xe0, &args![file]);
        } else if chunk == tag(b"XSCL") {
            e.call(0x0047_27f0, &args![file, me + 0x3c]);
        } else if chunk == tag(b"NAME") {
            e.call(0x0047_27f0, &args![file, id_slot]);
            e.call(0x0048_5d50, &args![id_slot, file]);
            let form_id = e.mem.u32(id_slot);
            let form = e.call(0x0048_39c0, &args![form_id]).u32();
            let base = dynamic_cast(e, form, TYPE_TES_BOUND_OBJECT);
            let mut accept = true;
            let current = e.get(data, OBJ_REFR::pObjectReference);
            let loading = global_ptr(e, GLOBAL_LOADING_OBJECT);
            if !current.is_null() && e.call(LOADING_FLAG, &args![loading]).bool() {
                let current_id = e.call(FORM_ID, &args![current]).u32();
                let handler = global_ptr(e, GLOBAL_DATA_HANDLER);
                if e.call(0x0046_9860, &args![handler, current_id]).bool() {
                    accept = false;
                }
            }
            if accept {
                e.set(data, OBJ_REFR::pObjectReference, Ptr::new(base));
                if base == 0 {
                    let my_id = e.call(FORM_ID, &args![this]).u32();
                    e.call(MESSAGE, &args![MISSING_BASE_FORMAT, form_id, my_id]);
                    base_missing = true;
                }
            }
        } else if chunk == tag(b"ONAM") {
            e.call(0x0057_2d50, &args![this, 8u32]);
        } else if chunk == tag(b"DATA") {
            if !base_missing {
                e.with_stack(0x18, |e, buffer| {
                    e.call(0x0069_2710, &args![buffer]);
                    e.vcall(me, 0xc4, &args![0u32]);
                    e.call(0x0047_2890, &args![file, buffer, 0x18u32]);
                    if e.call(0x0040_1680, &args![file]).bool() {
                        e.call(0x0069_3dc0, &args![buffer]);
                    }
                    for i in 0..3 {
                        let position = e.mem.u32(buffer.addr() + 4 * i);
                        e.mem.set_u32(me + 0x30 + 4 * i, position);
                        let rotation = e.mem.u32(buffer.addr() + 0xc + 4 * i);
                        e.mem.set_u32(me + 0x24 + 4 * i, rotation);
                    }
                });
            }
        } else if chunk == tag(b"RCLR") {
            // skipped
        } else {
            e.call(MESSAGE, &args![LOAD_TERMINATED_MESSAGE]);
        }
        if !e.call(0x0047_26f0, &args![file]).bool() {
            break;
        }
    }

    let list = extra_list(e, me);
    let patrol = e.call(0x0041_fe90, &args![list]).u32();
    if patrol != 0 {
        let script = e.call(0x008d_0430, &args![patrol]).u32();
        if script != 0 {
            let info = e.call(0x0050_0940, &args![script]).u32();
            if e.mem.u32(info + 8) == 0
                && fn_0055b980(e, Ptr::new(script)) != 0
                && !fn_0055b9a0(e, Ptr::new(script))
            {
                e.with_stack(8, |e, text| {
                    e.call(STRING_CONSTRUCT, &args![text]);
                    let name = e.vcall(me, 0x130, &args![]).u32();
                    e.call(
                        STRING_FORMAT,
                        &args![text, PATROL_NOT_COMPILED_FORMAT, name],
                    );
                    let line = e.call(READ_FIRST_DWORD, &args![text]).u32();
                    e.call(MESSAGE, &args![line]);
                    e.call(STRING_DESTRUCT, &args![text]);
                });
            }
        }
    }
    !e.get(data, OBJ_REFR::pObjectReference).is_null()
}

/// `Get3D` (virtual at +0x1d0): the reference's loaded 3D, null when none.
fn get_3d(e: &mut Engine, refr: u32) -> u32 {
    e.vcall(refr, 0x1d0, &args![]).u32()
}

/// The reference's world position as `GetLocationOnReference`-style
/// accessor (virtual at +0x1f4, returns `&data.Location`).
fn location_of(e: &mut Engine, refr: u32) -> u32 {
    e.vcall(refr, 0x1f4, &args![]).u32()
}

/// The tail the collision reset shares between `Copy`'s places: resets the
/// 3D node's simulation (`00c6bd00(node, 1)`), then gives it a zero velocity
/// (`NiPoint3(0, 0, 0)` built at `scratch`, 12 bytes).
fn reset_collision(e: &mut Engine, refr: u32, scratch: u32) {
    let node = get_3d(e, refr);
    e.call(0x00c6_bd00, &args![node, 1u32]);
    e.call(0x0043_d410, &args![scratch, 0.0f32, 0u32, 0u32]);
    let node = get_3d(e, refr);
    e.call(0x00a5_9c60, &args![node, scratch]);
}

/// One pass over the water-light list of `source`'s extra data for `Copy`:
/// every `(light owner, ...)` entry has its light moved: with `remove` the
/// light is taken out of the shadow scene (`ShadowSceneNode::RemoveLight`
/// after a membership test), without it the light is added
/// (`0057c730`). Each entry is finally pointed at `this`
/// (`ExtraDataList::SetWaterLightRef`, `0041f840`, with `remove` as flag).
fn move_water_lights(e: &mut Engine, this: u32, source_list: u32, remove: bool) {
    let list = extra_list(e, source_list);
    let mut node = e.call(0x0041_f810, &args![list]).u32();
    loop {
        if node == 0 {
            break;
        }
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        if e.mem.u32(slot) == 0 {
            break;
        }
        let own_list = extra_list(e, this);
        let light = e.call(0x0041_8250, &args![own_list]).u32();
        if light != 0 && e.call(READ_FIRST_DWORD, &args![light]).u32() != 0 {
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let owner = e.mem.u32(slot);
            if get_3d(e, owner) != 0 {
                let scene = global_ptr(e, GLOBAL_TEMP_REFR_MANAGER);
                if e.call(0x0070_ec90, &args![scene]).u32() != 0 {
                    let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                    let owner = e.mem.u32(slot);
                    let scene_node = e.call(0x0070_ec90, &args![scene]).u32();
                    let attached = e.call(0x004e_8030, &args![scene_node, owner]).u32();
                    let parent = e.call(0x0040_30b0, &args![attached]).u32();
                    if parent != 0 && e.call(0x008d_8520, &args![parent]).u32() == 0xd {
                        let lights = e.call(0x00a5_9d30, &args![attached, 3u32]).u32();
                        let handle = e.call(READ_FIRST_DWORD, &args![light]).u32();
                        e.with_stack(4, |e, slot| {
                            e.mem.set_u32(slot.addr(), handle);
                            if remove {
                                if e.call(0x0049_c680, &args![lights + 0x128, slot, 0u32])
                                    .u32()
                                    == 0
                                {
                                    let handle = e.call(READ_FIRST_DWORD, &args![light]).u32();
                                    e.mem.set_u32(slot.addr(), handle);
                                    e.call(0x004e_d8c0, &args![lights + 0x128, slot]);
                                    e.mem.set_u8(lights + 0x83, 1);
                                }
                                let handle = e.call(READ_FIRST_DWORD, &args![light]).u32();
                                let manager = e.call(0x0045_0b80, &args![0u32]).u32();
                                e.call(0x00b5_eed0, &args![manager, handle]);
                            } else {
                                e.call(0x0057_c730, &args![lights + 0x128, slot]);
                            }
                        });
                    }
                }
            }
        }
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let item = e.mem.u32(slot);
        let item_extra = extra_list(e, item);
        e.call(0x0041_f840, &args![item_extra, this, remove as u32]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
}

/// Whether a `float` coordinate is finite and not a NaN, by the CRT's
/// `_finite` (`00ec7595`) and `_isnan` (`00ec75b1`), which take a `double`.
fn is_not_finite(e: &mut Engine, bits: u32) -> bool {
    let value = f32::from_bits(bits) as f64;
    e.call(CRT_FINITE, &args![value]).i32() == 0
}

fn is_nan_float(e: &mut Engine, bits: u32) -> bool {
    let value = f32::from_bits(bits) as f64;
    e.call(CRT_ISNAN, &args![value]).i32() != 0
}

/// `_finite` on all three, then `_isnan` on all three, as `InitItem` tests
/// a position or a rotation; true when any is infinite or a NaN.
fn vector_is_corrupt(e: &mut Engine, at: u32) -> bool {
    let words = [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)];
    words.iter().any(|w| is_not_finite(e, *w)) || words.iter().any(|w| is_nan_float(e, *w))
}

/// Overwrites three dwords at `at` with the zero vector the game keeps at
/// `011f426c`.
fn reset_vector(e: &mut Engine, at: u32) {
    for i in 0..3 {
        let word = e.global::<u32>(ZERO_VECTOR + 4 * i);
        e.mem.set_u32(at + 4 * i, word);
    }
}

/// The error text `InitItem` writes when its warning count went up, in three
/// shapes: the cell is a plain one (`00425fd0` holds), the cell has
/// coordinates in a world, or there is no cell.
fn report_init_item_errors(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    if cell != 0 {
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        if e.call(0x0042_5fd0, &args![cell]).bool() {
            let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
            let base = e.call(GET_BASE_FORM, &args![this]).u32();
            let cell_for_id = e.call(GET_PARENT_CELL, &args![this]).u32();
            let cell_id = e.call(FORM_ID, &args![cell_for_id]).u32();
            let cell_name = e.vcall(cell, 0x130, &args![]).u32();
            let ref_id = e.call(FORM_ID, &args![this]).u32();
            let ref_name = e.call(0x0040_1280, &args![this]).u32();
            let base_again = e.call(GET_BASE_FORM, &args![this]).u32();
            let base_id = e.call(FORM_ID, &args![base_again]).u32();
            let base_name = e.vcall(base, 0x130, &args![]).u32();
            e.call(
                MESSAGE,
                &args![
                    INIT_ITEM_ERROR_CELL,
                    base_name,
                    base_id,
                    ref_name,
                    ref_id,
                    cell_name,
                    cell_id
                ],
            );
            return;
        }
    }
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    if cell != 0 {
        let world = e.call(0x0057_5d70, &args![this]).u32();
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        let world_again = e.call(0x0057_5d70, &args![this]).u32();
        let world_id = e.call(FORM_ID, &args![world_again]).u32();
        let world_name = e.vcall(world, 0x130, &args![]).u32();
        let cell_for_y = e.call(GET_PARENT_CELL, &args![this]).u32();
        let y = e.call(0x0054_4c60, &args![cell_for_y]).u32();
        let cell_for_x = e.call(GET_PARENT_CELL, &args![this]).u32();
        let x = e.call(0x0054_4c30, &args![cell_for_x]).u32();
        let cell_for_id = e.call(GET_PARENT_CELL, &args![this]).u32();
        let cell_id = e.call(FORM_ID, &args![cell_for_id]).u32();
        let cell_name = e.vcall(cell, 0x130, &args![]).u32();
        let ref_id = e.call(FORM_ID, &args![this]).u32();
        let ref_name = e.call(0x0040_1280, &args![this]).u32();
        let base_again = e.call(GET_BASE_FORM, &args![this]).u32();
        let base_id = e.call(FORM_ID, &args![base_again]).u32();
        let base_name = e.vcall(base, 0x130, &args![]).u32();
        e.call(
            MESSAGE,
            &args![
                INIT_ITEM_ERROR_WORLD,
                base_name,
                base_id,
                ref_name,
                ref_id,
                cell_name,
                cell_id,
                x,
                y,
                world_name,
                world_id
            ],
        );
    } else {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        let ref_id = e.call(FORM_ID, &args![this]).u32();
        let ref_name = e.call(0x0040_1280, &args![this]).u32();
        let base_again = e.call(GET_BASE_FORM, &args![this]).u32();
        let base_id = e.call(FORM_ID, &args![base_again]).u32();
        let base_name = e.vcall(base, 0x130, &args![]).u32();
        e.call(
            MESSAGE,
            &args![
                INIT_ITEM_ERROR_NO_CELL,
                base_name,
                base_id,
                ref_name,
                ref_id
            ],
        );
    }
}

// Translated from 00426720 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reference-change handler on an `ExtraDataList` (`this`; the map names the
/// body `TESObjectREFR::SetUnderwater (Xbox PDB)`, but it does not touch
/// water, so it is kept as `fn_00426720`). For the change flag `0x20000` and
/// a `target` that is not an actor it fetches the extra of type 0x2b's
/// pointer (`00418460`) and hands it to the `TESForm` helper `00483710`. The
/// other flag test is `flags & 0` and compiled out.
pub fn fn_00426720(e: &mut Engine, this: Ptr, flags: u32, _unused: u32, target: Ptr) {
    let actor = is_actor(e, target.addr());
    if !actor && flags & 0x20000 != 0 {
        let entry = e.call(0x0041_8460, &args![this]).u32();
        e.call(0x0048_3710, &args![entry]);
    }
}

// Translated from 004267d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reference-change handler on an `ExtraDataList` (`this`): for the change
/// `flags`, clears extras of the `target` reference. Seven of its flag tests
/// are `flags & 0` and compiled out.
///
/// * `0x20000` on a non-actor with a teleport extra removes it;
/// * `0x10000000` removes the last finished sequence;
/// * when `0055f5b0` on the save/load object says so, an actor target loses
///   its package, follower and other extras (and the item-dropper link), and
///   any other target hands its dropped item back.
pub fn fn_004267d0(e: &mut Engine, this: Ptr, flags: u32, target: Ptr) {
    let actor = is_actor(e, target.addr());
    if flags & 0x20000 != 0 && !actor {
        let teleport = e.call(0x0041_8460, &args![this]).u32();
        if teleport != 0 {
            e.call(0x0041_ae90, &args![this]);
        }
    }
    if flags & 0x1000_0000 != 0 {
        e.call(0x0042_2920, &args![this]);
    }
    let save_load = global_ptr(e, GLOBAL_SAVE_LOAD);
    if e.call(0x0055_f5b0, &args![save_load]).bool() {
        if actor {
            e.call(0x0041_cc60, &args![this]);
            e.call(0x0041_d930, &args![this]);
            e.call(0x0041_b060, &args![this]);
            e.call(0x0042_2720, &args![this]);
            e.call(0x0042_2670, &args![this]);
            e.call(EXTRA_REMOVE_TYPE, &args![this, 0x3au32]);
            e.call(0x0041_9dc0, &args![this, 0u32]);
            e.call(0x0041_aff0, &args![this]);
            e.call(0x0042_e040, &args![this]);
            e.call(0x0042_16b0, &args![this]);
        } else {
            let dropper = e.call(0x0041_de00, &args![this]).u32();
            if dropper != 0 {
                let dropper_extra = extra_list(e, dropper);
                e.call(0x0041_e0d0, &args![dropper_extra, target]);
                e.call(0x0041_de40, &args![this, 0u32]);
            }
        }
    }
}

// Translated from 004269c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Handler on an `ExtraDataList` (`this`) for an actor `target`: if the list
/// has a persistent cell (`0041d460`) and the actor has a parent cell
/// (`008d6f30`), clears the list's persistent cell and removes the actor from
/// that cell.
pub fn fn_004269c0(e: &mut Engine, this: Ptr, _unused: u32, target: Ptr) {
    if !is_actor(e, target.addr()) {
        return;
    }
    let cell = e.call(EXTRA_GET_PERSISTENT_CELL, &args![this]).u32();
    if cell != 0 && e.call(GET_PARENT_CELL, &args![target]).u32() != 0 {
        e.call(EXTRA_SET_PERSISTENT_CELL, &args![this, 0u32]);
        e.call(CELL_REMOVE_REFERENCE, &args![cell, target]);
    }
}

// Translated from 004b0130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTPointerMap<TESObjectREFR *,bool>::DeleteItem` (Xbox PDB): clears the
/// item's `bool` value (+8) and gives the item back to the map's allocator
/// (the pool at `this + 0xC`).
pub fn ni_t_pointer_map_tes_object_refr_p_bool_delete_item(e: &mut Engine, this: Ptr, item: Ptr) {
    e.mem.set_u8(item.addr() + 8, 0);
    e.call(0x0045_cee0, &args![this.addr() + 0xc, item]);
}

// Translated from 0055a2f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::TESObjectREFR` (Xbox PDB): constructs the form base and
/// the child-cell base, sets both vtables, constructs the `OBJ_REFR` points
/// and the `ExtraDataList`, sets the form type 0x3a, clears the loaded data
/// pointer, resets the data (`0055ac90`) and clears the base object, parent
/// cell and random sound.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn tes_object_refr_tes_object_refr(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
) -> Ptr<TESObjectREFR> {
    e.call(0x0048_3370, &args![this]);
    e.call(0x0053_3240, &args![this.addr() + CHILD_CELL_OFFSET]);
    e.mem.set_u32(this.addr(), REFR_VTABLE);
    e.set(this, TESObjectREFR::vfptrTESChildCell, CHILD_CELL_VTABLE);
    fn_0055a400(e, this.at(TESObjectREFR::data));
    e.call(0x0041_0360, &args![this.addr() + EXTRA_OFFSET]);
    e.call(0x004f_15a0, &args![this, 0x3au32]);
    e.set(this, TESObjectREFR::pLoadedData, Ptr::NULL);
    fn_0055ac90(e, this);
    let data = this.at(TESObjectREFR::data);
    e.set(data, OBJ_REFR::pObjectReference, Ptr::NULL);
    e.set(this, TESObjectREFR::pParentCell, Ptr::NULL);
    e.set(this, TESObjectREFR::pRandomSound, Ptr::NULL);
    this
}

// Translated from 0055a3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x1e4 (`GetAnimation` in the Xbox PDB order): the animation
/// the reference's `ExtraDataList` holds (`00418220`).
pub fn fn_0055a3b0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let extra = extra_list(e, this.addr());
    e.call(0x0041_8220, &args![extra]).u32()
}

// Translated from 0055a3d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::__vecDelDtor` (Xbox PDB), the scalar deleting destructor
/// (virtual at +0x10): runs the destructor (`0055a430`) and, when bit 0 of
/// `flags` is set, frees the object with `operator delete` (`00401030`).
pub fn fn_0055a3d0(e: &mut Engine, this: Ptr<TESObjectREFR>, flags: u32) -> Ptr<TESObjectREFR> {
    fn_0055a430(e, this);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 0055a400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `OBJ_REFR` constructor: constructs its two `NiPoint3` (at +4 and +0x10,
/// whose constructor `006815c0` does nothing). `this` is the `OBJ_REFR`.
pub fn fn_0055a400(e: &mut Engine, this: Ptr<OBJ_REFR>) -> Ptr<OBJ_REFR> {
    e.call(LIST_ITEM_SLOT, &args![this.addr() + 4]);
    e.call(LIST_ITEM_SLOT, &args![this.addr() + 0x10]);
    this
}

// Translated from 0055a430 (decompiled, FalloutNV.exe 1.4.0.525)
/// The body of `~TESObjectREFR` (the destructor `0055a3d0` calls). Resets the
/// vtables, takes the reference out of everything that tracks it (furniture
/// users, the garbage collector, the terrain tree, water lists, process
/// lists, the player's follower and enemy slots, `ExtraDataList`
/// cross-links, the parent and persistent cells and the navmesh obstacle
/// manager), clears the data (`0055ad20`) and runs the `ExtraDataList` and
/// `TESForm` destructors.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_0055a430(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    e.mem.set_u32(me, REFR_VTABLE);
    e.set(this, TESObjectREFR::vfptrTESChildCell, CHILD_CELL_VTABLE);
    e.call(0x0057_acc0, &args![this]);

    let is_furniture = e.call(0x0056_8680, &args![this]).bool();
    if is_furniture {
        e.call(0x0045_38e0, &args![OBJECT_FURNITURE_MANAGER, 0u32]);
        let stored = e.call(0x0082_5c00, &args![OBJECT_ACTOR_LISTS]).u32();
        e.set_global(GLOBAL_STORED_BY_825C00, stored);
        let mut node = e.call(0x0096_e290, &args![OBJECT_PROCESS_LISTS]).u32();
        while node != 0 {
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let item = e.mem.u32(slot);
            if item != 0 && e.call(0x008d_8520, &args![item]).u32() != 0 {
                let target = e.call(0x008d_8520, &args![item]).u32();
                e.vcall(target, 0x4cc, &args![this]);
            }
            node = e.call(LIST_NEXT, &args![node]).u32();
        }
        e.call(0x0082_f1f0, &args![OBJECT_FURNITURE_MANAGER]);
    }
    if e.call(FORM_TYPE, &args![this]).i32() == 0x27 {
        let stored = e.call(0x0082_5c00, &args![OBJECT_ACTOR_LISTS]).u32();
        e.set_global(GLOBAL_STORED_BY_825C00, stored);
    }
    // GarbageCollector::Remove (cdecl)
    e.call(0x0086_8270, &args![this]);

    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        if e.call(0x0054_9580, &args![base]).bool() {
            let world = e.call(0x0057_5d70, &args![this]).u32();
            if world != 0 && e.call(0x0058_6170, &args![world]).u32() != 0 {
                let terrain = e.call(0x0058_6170, &args![world]).u32();
                e.call(0x006f_cf30, &args![terrain, this]);
            }
        }
    }

    let loaded = e.get(this, TESObjectREFR::pLoadedData);
    if !loaded.is_null() {
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        let tes = if cell != 0 {
            let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
            e.call(0x0045_43c0, &args![cell]).u32()
        } else {
            0
        };
        if tes != 0 {
            let phantom = e.call(READ_FIRST_DWORD, &args![loaded.addr() + 0x18]).u32();
            e.call(0x0053_80d0, &args![tes, phantom]);
        }
        let loaded = e.get(this, TESObjectREFR::pLoadedData);
        e.call(0x0066_b0d0, &args![loaded.addr() + 0x18, 0u32]);
    }

    let save_load = global_ptr(e, GLOBAL_SAVE_LOAD);
    let saved_version = e.call(0x008a_81c0, &args![save_load]).u8();
    e.call(0x008a_8150, &args![save_load, 0u32]);

    if !e.call(0x0040_77c0, &args![this]).bool() {
        e.call(0x0057_6760, &args![this]);
        e.call(0x008d_15b0, &args![this]);
        e.call(0x0086_23a0, &args![save_load, this]);
        let data_handler = global_ptr(e, GLOBAL_DATA_HANDLER);
        if !e.call(0x0042_26e0, &args![data_handler]).bool() {
            let enable_parent = e.call(0x0056_a9f0, &args![this]).u32();
            if enable_parent != 0 {
                let extra = extra_list(e, enable_parent);
                e.call(0x0041_dda0, &args![extra, this]);
            }
            let mut node = e.call(0x0056_ac90, &args![this]).u32();
            while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
                let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                let child = e.mem.u32(slot);
                e.call(0x0056_aa10, &args![child, 0u32]);
                node = e.call(0x0056_ac90, &args![this]).u32();
            }
        }
        let extra = extra_list(e, me);
        let dropper = e.call(0x0041_de00, &args![extra]).u32();
        if dropper != 0 {
            let dropper_extra = extra_list(e, dropper);
            e.call(0x0041_e0d0, &args![dropper_extra, this]);
        }
        let extra = extra_list(e, me);
        let mut node = e.call(0x0041_df90, &args![extra]).u32();
        while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let dropped = e.mem.u32(slot);
            let dropped_extra = extra_list(e, dropped);
            e.call(0x0041_de40, &args![dropped_extra, 0u32]);
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let dropped = e.mem.u32(slot);
            if e.call(0x0056_7790, &args![dropped]).u32() == me {
                let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                let dropped = e.mem.u32(slot);
                e.call(0x0056_7c50, &args![dropped]);
            }
            node = e.call(LIST_NEXT, &args![node]).u32();
        }
        let data_handler = global_ptr(e, GLOBAL_DATA_HANDLER);
        if !e.call(0x0042_26e0, &args![data_handler]).bool() {
            let extra = extra_list(e, me);
            let ash_pile = e.call(0x0041_e310, &args![extra]).u32();
            if ash_pile != 0 {
                let ash_extra = extra_list(e, ash_pile);
                e.call(0x0041_e340, &args![ash_extra, 0u32]);
            }
            let extra = extra_list(e, me);
            let mut node = e.call(0x0041_f170, &args![extra]).u32();
            while node != 0 {
                let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                if e.mem.u32(slot) == 0 {
                    break;
                }
                let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                let entry = e.mem.u32(slot);
                node = e.call(LIST_NEXT, &args![node]).u32();
                let first = e.mem.u32(entry);
                let first_extra = extra_list(e, first);
                e.call(0x0041_f4c0, &args![first_extra, this, 0u32]);
                let first_extra = extra_list(e, first);
                e.call(0x0041_f650, &args![first_extra, this, 0u32]);
            }
            // Script::RemoveDelayedScriptActionReference (cdecl)
            e.call(0x005a_a5d0, &args![this]);
            for getter in [WATER_LIST_A_GETTER, WATER_LIST_B_GETTER] {
                let list = e.call(getter, &args![]).u32();
                e.with_stack(4, |e, local| {
                    e.mem.set_u32(local.addr(), me);
                    e.call(LIST_REMOVE, &args![list, local]);
                });
            }
            if e.call(0x0056_4d80, &args![this]).bool() {
                let kind = e.call(FORM_TYPE, &args![this]).i32();
                let player = global_ptr(e, GLOBAL_PLAYER);
                if (0x3b..=0x3c).contains(&kind) {
                    e.call(0x0096_3d60, &args![player, this]);
                } else {
                    e.call(0x0097_6680, &args![OBJECT_PROCESS_LISTS, this]);
                    e.call(0x0096_3d60, &args![player, this]);
                    e.call(0x0096_f600, &args![OBJECT_PROCESS_LISTS, this, 1u32]);
                    e.call(0x0097_5320, &args![OBJECT_PROCESS_LISTS, this]);
                    e.call(0x0097_8660, &args![OBJECT_PROCESS_LISTS, this]);
                }
            }
            if !e.call(0x0057_25b0, &args![this]).bool() {
                e.call(0x0097_49b0, &args![OBJECT_PROCESS_LISTS, this]);
            }
        }
        if fn_0055abe0(e, this) {
            let player = global_ptr(e, GLOBAL_PLAYER);
            fn_0055ac40(e, Ptr::new(player), this);
        }
        let data_handler = global_ptr(e, GLOBAL_DATA_HANDLER);
        if !e.call(0x0042_26e0, &args![data_handler]).bool()
            && e.call(0x0056_8680, &args![this]).bool()
        {
            let manager = global_ptr(e, GLOBAL_TEMP_REFR_MANAGER);
            let list = fn_0055ac00(e, Ptr::new(manager));
            e.with_stack(4, |e, local| {
                e.mem.set_u32(local.addr(), me);
                e.call(LIST_REMOVE, &args![list, local]);
            });
        }
        if e.call(0x0045_2370, &args![this]).bool() {
            e.call(0x0047_7410, &args![this]);
        }
    }

    let linked = e.call(0x0056_9b80, &args![this]).u32();
    if linked != 0 {
        let linked_extra = extra_list(e, linked);
        e.call(0x0041_e600, &args![linked_extra, this]);
    }
    let extra = extra_list(e, me);
    let mut node = e.call(0x0041_e500, &args![extra]).u32();
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let child = e.mem.u32(slot);
        e.call(0x0056_9ba0, &args![child, 0u32]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    let extra = extra_list(e, me);
    let mut node = e.call(0x0041_e780, &args![extra]).u32();
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let entry = e.mem.u32(slot);
        let first = e.mem.u32(entry);
        let first_extra = extra_list(e, first);
        e.call(0x0041_ef20, &args![first_extra, this]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    let extra = extra_list(e, me);
    let mut node = e.call(0x0041_eda0, &args![extra]).u32();
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let entry = e.mem.u32(slot);
        let first = e.mem.u32(entry);
        let first_extra = extra_list(e, first);
        e.call(0x0041_e960, &args![first_extra, this]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }

    // TESObjectREFR::SetDelete(true), then ClearData (direct, not virtual)
    e.call(0x0056_4930, &args![this, 1u32]);
    tes_object_refr_clear_data(e, this);

    let parent = e.get(this, TESObjectREFR::pParentCell);
    if !parent.is_null() {
        e.call(CELL_REMOVE_REFERENCE, &args![parent, this]);
    }
    if e.call(GET_REF_PERSISTS, &args![this]).bool() {
        let data_handler = global_ptr(e, GLOBAL_DATA_HANDLER);
        if data_handler != 0 && !e.call(0x0042_26e0, &args![data_handler]).bool() {
            let persistent = e.call(0x0057_5dc0, &args![me + CHILD_CELL_OFFSET]).u32();
            if persistent != 0 {
                e.call(CELL_REMOVE_REFERENCE, &args![persistent, this]);
            }
        }
    }
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 && e.call(0x0056_4bc0, &args![this]).bool() {
        let manager = e.call(0x006c_0720, &args![]).u32();
        e.call(0x006c_0c80, &args![manager, this]);
    }
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        if e.call(FORM_TYPE, &args![base]).i32() == 0x1c {
            let manager = e.call(0x006c_0720, &args![]).u32();
            e.call(0x006c_1060, &args![manager, this]);
        }
    }
    // Interface::... (cdecl)
    e.call(0x0070_5b30, &args![this]);
    if fn_0055ac70(e) == me {
        fn_0055ac80(e);
    }
    let data_handler = global_ptr(e, GLOBAL_DATA_HANDLER);
    if !e.call(0x0042_26e0, &args![data_handler]).bool() {
        if e.call(0x0070_38d0, &args![]).u32() == me {
            // Interface::SetEnemyActor(0) (cdecl)
            e.call(0x0070_38b0, &args![0u32]);
        }
        let player = global_ptr(e, GLOBAL_PLAYER);
        if player != 0 {
            e.call(0x0096_2cd0, &args![player, this]);
        }
    }
    let player = global_ptr(e, GLOBAL_PLAYER);
    if e.call(0x0089_f4e0, &args![player]).u32() == me {
        fn_0055ac20(e, Ptr::new(player));
    }
    e.call(0x008a_8150, &args![save_load, saved_version as u32]);
    e.call(0x0041_03b0, &args![me + EXTRA_OFFSET]);
    e.call(0x0048_3630, &args![this]);
}

// Translated from 0055abe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x400000` of the form's `iFormFlags` (+0x08) is set.
pub fn fn_0055abe0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    // TESForm::iFormFlags (Xbox PDB) +0x08
    e.mem.u32(this.addr() + 0x08) & 0x40_0000 != 0
}

// Translated from 0055ac00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Address of the list at +0x94 of the object `this` (the global at
/// `011dea10`; the destructor removes furniture from it).
pub fn fn_0055ac00(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x94)
}

// Translated from 0055ac20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the field at +0x638 of `this` (the player; the destructor calls it
/// when the reference held there is the one being destroyed).
pub fn fn_0055ac20(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr() + 0x638, 0);
}

// Translated from 0055ac40 (decompiled, FalloutNV.exe 1.4.0.525)
/// If `item` is not null, removes it from the list at `this + 0x84c`
/// (`this` is the player; `00905330` takes the address of the item slot).
pub fn fn_0055ac40(e: &mut Engine, this: Ptr, item: Ptr<TESObjectREFR>) {
    if item.is_null() {
        return;
    }
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item.addr());
        e.call(LIST_REMOVE, &args![this.addr() + 0x84c, slot]);
    });
}

// Translated from 0055ac70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The reference remembered in the global at `011dea24`.
pub fn fn_0055ac70(e: &mut Engine) -> u32 {
    e.global::<u32>(GLOBAL_LAST_REFR)
}

// Translated from 0055ac80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forgets the reference remembered in `011dea24`.
pub fn fn_0055ac80(e: &mut Engine) {
    e.set_global(GLOBAL_LAST_REFR, 0u32);
}

// Translated from 0055ac90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resets the reference's `OBJ_REFR` data (all but the base object pointer
/// is zeroed), its scale to 1.0 and, if it has loaded data, the water object
/// and water height; then, unless the loading object says the game is
/// loading (`004121b0`), restores the leveled creature's original base
/// (`0057c300`).
pub fn fn_0055ac90(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let data = this.at(TESObjectREFR::data);
    let base = e.get(data, OBJ_REFR::pObjectReference);
    e.call(MEMSET, &args![data, 0u32, 0x1cu32]);
    e.set(data, OBJ_REFR::pObjectReference, base);
    e.set(this, TESObjectREFR::fRefScale, 1.0f32);
    let loaded: Ptr<LOADED_REF_DATA> = e.get(this, TESObjectREFR::pLoadedData).cast();
    if !loaded.is_null() {
        e.set(loaded, LOADED_REF_DATA::pCurrentWaterObject, Ptr::NULL);
        let height: f32 = e.global(LOADED_DATA_DEFAULT_HEIGHT);
        e.set(loaded, LOADED_REF_DATA::fRelevantWaterHeight, height);
    }
    let loading = global_ptr(e, GLOBAL_LOADING_OBJECT);
    if loading == 0 || !e.call(LOADING_FLAG, &args![loading]).bool() {
        e.call(0x0057_c300, &args![this]);
    }
}

// Translated from 0055ad20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::ClearData` (Xbox PDB), virtual at +0x18: resets the
/// reference while keeping the extras that matter. Restores the leveled
/// creature base unless loading, kills its light, remembers the persistent
/// cell and the extra of type 0x5b tied to the 3D (taking it out of the
/// list), removes the reference from all water, wipes the extra list
/// (`RemoveAllDefault` when loading, else `RemoveAll`), puts the 0x5b extra
/// and the persistent cell back, drops the 3D (virtual at +0x1cc,
/// `Set3D(0, 1)`) unless loading and finally removes the extra again.
pub fn tes_object_refr_clear_data(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let loading_object = global_ptr(e, GLOBAL_LOADING_OBJECT);
    if loading_object == 0 || !e.call(LOADING_FLAG, &args![loading_object]).bool() {
        e.call(0x0056_b0b0, &args![this]);
    }
    e.call(0x0057_2a50, &args![this, 0u32]);
    let cell = e
        .call(EXTRA_GET_PERSISTENT_CELL, &args![me + EXTRA_OFFSET])
        .u32();
    let loading = e.call(LOADING_FLAG, &args![loading_object]).bool();
    let mut extra = 0;
    if e.vcall(me, 0x1d0, &args![]).u32() != 0 {
        let list = extra_list(e, me);
        extra = e.call(EXTRA_GET_DATA, &args![list, 0x5bu32]).u32();
        if extra != 0 {
            let list = extra_list(e, me);
            e.call(EXTRA_REMOVE, &args![list, extra, 0u32]);
        }
    }
    e.call(0x0057_b520, &args![this, 0u32]);
    if loading {
        e.call(EXTRA_REMOVE_ALL_DEFAULT, &args![me + EXTRA_OFFSET, 1u32]);
    } else {
        e.call(EXTRA_REMOVE_ALL, &args![me + EXTRA_OFFSET, 1u32]);
    }
    if extra != 0 {
        let list = extra_list(e, me);
        e.call(EXTRA_ADD, &args![list, extra]);
    }
    if cell != 0 {
        e.call(EXTRA_SET_PERSISTENT_CELL, &args![me + EXTRA_OFFSET, cell]);
    }
    let loading_object = global_ptr(e, GLOBAL_LOADING_OBJECT);
    if !e.call(LOADING_FLAG, &args![loading_object]).bool() {
        e.vcall(me, 0x1cc, &args![0u32, 1u32]);
    }
    if extra != 0 {
        let list = extra_list(e, me);
        e.call(EXTRA_REMOVE, &args![list, extra, 1u32]);
    }
}

// Translated from 0055ae70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::Save` (Xbox PDB), virtual at +0x2c: writes the reference
/// into the open save record. A form flagged deleted (`iFormFlags & 0x20`)
/// is just closed. Otherwise it writes `NAME` (the base object's form ID),
/// the extra data (`00412970`), `XSCL` when the scale is not 1.0, `ONAM` when
/// the extra-data test `00572d30(8)` says so, and `DATA`: position then
/// rotation (24 bytes, byte-swapped around the write when the build swaps
/// endianness).
pub fn tes_object_refr_save(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    e.call(0x0048_55a0, &args![this]);
    if e.call(FORM_IS_DELETED, &args![this]).bool() {
        e.call(0x0048_5680, &args![this]);
        return;
    }
    let data = this.at(TESObjectREFR::data);
    let base = e.get(data, OBJ_REFR::pObjectReference);
    let base_id = e.call(FORM_ID, &args![base]).u32();
    e.call(ADD_CHUNK_VALUE, &args![tag(b"NAME"), base_id]);
    let list = extra_list(e, me);
    e.call(0x0041_2970, &args![list]);
    let scale = e.get(this, TESObjectREFR::fRefScale);
    let one: f64 = e.global(CONSTANT_ONE);
    if scale as f64 != one {
        e.call(ADD_CHUNK_VALUE, &args![tag(b"XSCL"), scale]);
    }
    if e.call(0x0057_2d30, &args![this, 8u32]).bool() {
        e.call(0x0048_56d0, &args![tag(b"ONAM")]);
    }
    e.with_stack(0x18, |e, buffer| {
        e.call(0x0069_2710, &args![buffer]);
        // Position (+0x30) first, then rotation (+0x24), as raw words.
        for i in 0..3 {
            let position = e.mem.u32(me + 0x30 + 4 * i);
            e.mem.set_u32(buffer.addr() + 4 * i, position);
            let rotation = e.mem.u32(me + 0x24 + 4 * i);
            e.mem.set_u32(buffer.addr() + 0xc + 4 * i, rotation);
        }
        if e.call(ENDIAN_SWAP_ON_SAVE, &args![]).bool() {
            e.call(0x0069_3dc0, &args![buffer]);
        }
        e.call(ADD_CHUNK_DATA, &args![tag(b"DATA"), buffer, 0x18u32]);
        if e.call(ENDIAN_SWAP_ON_SAVE, &args![]).bool() {
            e.call(0x0069_3dc0, &args![buffer]);
        }
    });
    e.call(0x0048_5680, &args![this]);
}

// Translated from 0055af90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::Load` (Xbox PDB), virtual at +0x20: reads the reference
/// record from `file`.
///
/// After `TESForm::LoadForm`, a form flagged deleted just drops its extras
/// (keeping the persistent cell) and returns whether it has a base object.
/// Otherwise it reads chunks until the record ends: `NAME` (base object,
/// looked up through the compile index; refused while loading when the
/// existing base is already known; a bad one logs `MASTERFILE: Missing/Invalid
/// base object` and ignores the `DATA` chunk), `XSCL` (scale), `ONAM`,
/// `DATA` (position then rotation, byte-swapped when the build does), `EDID`
/// (editor ID via the virtual at +0x134), `OBND` (virtual at +0xe0) and 76
/// extra-data chunk kinds ([`EXTRA_DATA_CHUNKS`]), which go to
/// `ExtraDataList::Load`. `RCLR` is skipped; an unknown chunk logs `abnormally
/// terminated`. Afterwards a patrol point script that has text but is not
/// compiled is reported. Returns whether the reference has a base object.
///
/// The compiler's exception-unwinding frame is not translated; a local the
/// code sets after `NAME` but never reads is left out. The body is
/// [`load_record`] (a helper kept at the top of the file).
pub fn tes_object_refr_load(e: &mut Engine, this: Ptr<TESObjectREFR>, file: Ptr) -> bool {
    e.with_stack(4, |e, id_slot| {
        load_record(e, this, file.addr(), id_slot.addr())
    })
}

// Translated from 0055b980 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at +0x2c of the object `this` (a patrol-point script holder;
/// `Load` tests it for the script's text).
pub fn fn_0055b980(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2c)
}

// Translated from 0055b9a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at +0x2a of the object `this` (whether its script is compiled).
pub fn fn_0055b9a0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x2a) != 0
}

// Translated from 0055b9c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x44 (`PostDuplicateProcess` in the Xbox PDB order): after a
/// reference has been duplicated through the save/load references map `map`
/// (`BGSSaveLoadReferencesMap`), rewires the duplicate's cross references.
/// For this reference's duplicate `target`: its enable parent (and the
/// parent's children), its other linked reference, the two sets of extra
/// references by slot (0 and 1; 0 to 3), the activate-ref children (with
/// their float), the decal refs, the multi-bound reference, and finally the
/// teleport data (door link), whose destination position is the linked
/// door's position plus the offset of this door's teleport point.
pub fn fn_0055b9c0(e: &mut Engine, this: Ptr<TESObjectREFR>, map: Ptr) {
    let me = this.addr();
    let map = map.addr();
    if map == 0 {
        return;
    }
    let old = lookup_new_reference(e, map, me);
    if old == 0 {
        return;
    }
    let target = dynamic_cast(e, old, TYPE_TES_OBJECT_REFR);
    if target == 0 {
        return;
    }

    let enable_parent = e.call(0x0056_a9f0, &args![this]).u32();
    if enable_parent != 0 {
        let parent = resolve_reference(e, map, enable_parent);
        if parent != 0 {
            e.call(0x0056_aa10, &args![target, parent]);
            let flag = e.call(0x0056_aa70, &args![this]).u8();
            e.call(0x0056_aa90, &args![target, flag as u32]);
            let parent_extra = extra_list(e, parent);
            e.call(0x0041_dcd0, &args![parent_extra, target]);
        }
    }

    let linked = e.call(0x0056_9b20, &args![this]).u32();
    if linked != 0 {
        let resolved = resolve_reference(e, map, linked);
        if resolved != 0 {
            e.call(0x0056_9b40, &args![target, resolved]);
        }
    }

    let parent_ref = e.call(0x0056_9b80, &args![this]).u32();
    if parent_ref != 0 {
        let resolved = resolve_reference(e, map, parent_ref);
        if resolved != 0 {
            e.call(0x0056_9ba0, &args![target, resolved]);
            let resolved_extra = extra_list(e, resolved);
            e.call(0x0041_e530, &args![resolved_extra, target]);
        }
    }

    for slot in 0..2u32 {
        let list = extra_list(e, me);
        let old_ref = e.call(0x0042_0bc0, &args![list, slot]).u32();
        if old_ref != 0 {
            let new = lookup_new_reference(e, map, old_ref);
            if new != 0 {
                let resolved = dynamic_cast(e, new, TYPE_TES_OBJECT_REFR);
                if resolved != 0 {
                    let target_extra = extra_list(e, target);
                    e.call(0x0042_0a60, &args![target_extra, slot, resolved]);
                }
            } else {
                // no mapping: the old reference stays
                let target_extra = extra_list(e, target);
                e.call(0x0042_0a60, &args![target_extra, slot, old_ref]);
            }
        }
    }
    for slot in 0..4u32 {
        let list = extra_list(e, me);
        let old_ref = e.call(0x0042_0130, &args![list, slot]).u32();
        if old_ref != 0 {
            let resolved = resolve_reference(e, map, old_ref);
            if resolved != 0 {
                let target_extra = extra_list(e, target);
                e.call(0x0042_0160, &args![target_extra, resolved, slot]);
            }
        }
    }

    let list = extra_list(e, me);
    let mut node = e.call(0x0041_e780, &args![list]).u32();
    let mut any = false;
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let entry = e.mem.u32(slot);
        node = e.call(LIST_NEXT, &args![node]).u32();
        let old_child = e.mem.u32(entry);
        let child = resolve_reference(e, map, old_child);
        if child != 0 {
            let target_extra = extra_list(e, target);
            e.call(0x0041_e7f0, &args![target_extra, child]);
            let value = e.mem.f32(entry + 4);
            let target_extra = extra_list(e, target);
            e.call(0x0041_ea30, &args![target_extra, child, value]);
            any = true;
            let child_extra = extra_list(e, child);
            e.call(0x0041_edd0, &args![child_extra, target]);
        }
    }
    if any {
        let list = extra_list(e, me);
        let flag = e.call(0x0041_eb60, &args![list]).u8();
        let target_extra = extra_list(e, target);
        e.call(0x0041_eba0, &args![target_extra, flag as u32]);
    }

    let list = extra_list(e, me);
    let mut node = e.call(0x0041_f050, &args![list]).u32();
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
        let entry = e.mem.u32(slot);
        let old_decal = e.mem.u32(entry);
        let decal = resolve_reference(e, map, old_decal);
        if decal != 0 {
            e.call(0x0056_a470, &args![target, decal, entry + 4, entry + 0x10]);
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }

    let bound = e.call(0x0056_a990, &args![this]).u32();
    if bound != 0 {
        let resolved = resolve_reference(e, map, bound);
        if resolved != 0 {
            e.call(0x0056_a9b0, &args![target, resolved]);
        }
    }

    let teleport = e.call(0x0056_8e50, &args![this]).u32();
    if teleport == 0 {
        return;
    }
    let door = e.call(READ_FIRST_DWORD, &args![teleport]).u32();
    if door == 0 {
        return;
    }
    let resolved = resolve_reference(e, map, door);
    if resolved == 0 {
        return;
    }
    let new_teleport = e.call(0x0056_8e70, &args![target]).u32();
    e.call(0x0053_7e90, &args![new_teleport, resolved]);
    let point = e.call(0x0046_0140, &args![teleport]).u32();
    e.with_stack(0x24, |e, frame| {
        let offset = frame.addr();
        let moved = frame.addr() + 0xc;
        let point_copy = frame.addr() + 0x18;
        for i in 0..3 {
            let value = e.mem.u32(point + 4 * i);
            e.mem.set_u32(point_copy + 4 * i, value);
        }
        let door = e.call(READ_FIRST_DWORD, &args![teleport]).u32();
        // NiPoint3 offset = (teleport + 4) - door->Location
        let door_location = e.vcall(door, 0x1f4, &args![]).u32();
        let teleport_base = e.call(0x0071_7e50, &args![teleport]).u32();
        e.call(0x0043_9ef0, &args![teleport_base, offset, door_location]);
        // NiPoint3 moved = resolved->Location + offset
        let location = e.vcall(resolved, 0x1f4, &args![]).u32();
        e.call(0x0043_9e90, &args![location, moved, offset]);
        e.call(0x006f_4ed0, &args![new_teleport, moved]);
        e.call(0x0043_a280, &args![new_teleport, point_copy]);
    });
}

// Translated from 0055c050 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::CreateDuplicateForm` (Xbox PDB), virtual at +0x40: sets
/// aside eight of the reference's extras (teleport, enable-state parent,
/// random teleport marker, two kinds of type 0x51, decal refs, merchant
/// container and multi-bound ref), clears its parent cell, duplicates the
/// form through `TESForm::CreateDuplicateForm` (`004867a0`, with the second
/// argument `map`) and puts the extras and the parent cell back. The copy
/// gets its 3D dropped (`Set3D(0, 1)`) and is made non-persistent.
pub fn tes_object_refr_create_duplicate_form(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    _unused: u32,
    map: Ptr,
) -> Ptr<TESObjectREFR> {
    let me = this.addr();
    const SET_ASIDE: [(u32, u32); 8] = [
        (0x2b, TYPE_EXTRA_TELEPORT),
        (0x37, TYPE_EXTRA_ENABLE_STATE_PARENT),
        (0x3b, TYPE_EXTRA_RANDOM_TELEPORT_MARKER),
        (0x51, TYPE_EXTRA_LINKED_REF),
        (0x51, TYPE_EXTRA_ACTIVATE_REF),
        (0x57, TYPE_EXTRA_DECAL_REFS),
        (0x3c, TYPE_EXTRA_MERCHANT_CONTAINER),
        (0x63, TYPE_EXTRA_MULTI_BOUND_REF),
    ];
    let mut held = [0u32; 8];
    for (i, (kind, descriptor)) in SET_ASIDE.iter().enumerate() {
        let list = extra_list(e, me);
        let extra = e.call(EXTRA_GET_DATA, &args![list, *kind]).u32();
        let typed = e
            .call(
                DYNAMIC_CAST,
                &args![extra, 0u32, TYPE_BS_EXTRA_DATA, *descriptor, 0u32],
            )
            .u32();
        held[i] = typed;
        if typed != 0 {
            let list = extra_list(e, me);
            e.call(EXTRA_REMOVE, &args![list, typed, 0u32]);
        }
    }
    let parent_cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    e.vcall(me, 0x228, &args![0u32]);
    let duplicate = e.call(0x0048_67a0, &args![this, 0u32, map]).u32();
    let duplicate = dynamic_cast(e, duplicate, TYPE_TES_OBJECT_REFR);
    for extra in held {
        if extra != 0 {
            let list = extra_list(e, me);
            e.call(EXTRA_ADD, &args![list, extra]);
        }
    }
    e.vcall(me, 0x228, &args![parent_cell]);
    e.vcall(duplicate, 0x1cc, &args![0u32, 1u32]);
    e.call(0x0056_5480, &args![duplicate, 0u32]);
    Ptr::new(duplicate)
}

// Translated from 0055c3e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::Copy` (Xbox PDB), virtual at +0x108: makes this
/// reference a copy of the form `source` (cast to `TESObjectREFR`; anything
/// else does nothing).
///
/// It copies the form components, the 0x1c bytes of `OBJ_REFR` data, the
/// persistence flag and the extra-data list (`ExtraDataList::CopyList`),
/// sets the scale, moves the cell registration (`AddReference` to the
/// source's cell, `RemoveReference` from the old one), rebuilds the links
/// the extras hold (enable parent, linked refs, activate-ref children, the
/// water lights, the teleport door pair), and rebuilds the 3D when the
/// copied data differs (`dirty`: another base object, a different scale
/// for a non-actor, a different stack count for ammo, a different radius
/// or another `00569580`).
///
/// `Set3D` (virtual at +0x1cc) is called at `0055cde0` with only one
/// argument pushed: the exe lets the second argument be whatever lay in the
/// stack (the caller's saved `ESI`); the translation passes 0.
pub fn tes_object_refr_copy(e: &mut Engine, this: Ptr<TESObjectREFR>, source: Ptr) {
    let me = this.addr();
    let data = this.at(TESObjectREFR::data);
    let other = dynamic_cast(e, source.addr(), TYPE_TES_OBJECT_REFR);
    if other == 0 {
        return;
    }
    let mut dirty = false;
    if !e.call(0x0040_77c0, &args![this]).bool() {
        let parent = e.call(0x0056_a9f0, &args![this]).u32();
        if parent != 0 {
            let parent = e.call(0x0056_a9f0, &args![this]).u32();
            let parent_extra = extra_list(e, parent);
            e.call(0x0041_dda0, &args![parent_extra, this]);
        }
    }
    let persists = e.call(GET_REF_PERSISTS, &args![other]).u8();
    e.call(0x0056_5480, &args![this, persists as u32]);
    e.call(0x0048_51b0, &args![this, source]);
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let mine = e.call(GET_BASE_FORM, &args![this]).u32();
        let theirs = e.call(GET_BASE_FORM, &args![other]).u32();
        if mine != theirs {
            dirty = true;
        }
    }
    let other_data = e.call(GET_DATA, &args![other]).u32();
    e.call(0x0040_1460, &args![data, other_data, 0x1cu32]);

    'compare: {
        let mine = e.call(0x0059_8040, &args![this]).f32() as f64;
        let theirs = e.call(0x0059_8040, &args![other]).f32() as f64;
        if mine != theirs && !is_actor(e, me) {
            dirty = true;
            break 'compare;
        }
        if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
            let base = e.call(GET_BASE_FORM, &args![this]).u32();
            if e.call(FORM_TYPE, &args![base]).i32() == 0x29 {
                let list = extra_list(e, me);
                let mine = e.call(0x0041_8770, &args![list]).u16() as i16;
                let other_list = extra_list(e, other);
                let theirs = e.call(0x0041_8770, &args![other_list]).u16() as i16;
                if mine != theirs {
                    dirty = true;
                    break 'compare;
                }
            }
        }
        let epsilon: f32 = e.global(RADIUS_EPSILON);
        let their_radius = e.call(0x0056_8cb0, &args![other]).f32();
        let my_radius = e.call(0x0056_8cb0, &args![this]).f32();
        if !e
            .call(0x0040_eb10, &args![my_radius, their_radius, epsilon])
            .bool()
        {
            dirty = true;
            break 'compare;
        }
        let mine = e.call(0x0056_9580, &args![this]).u32();
        let theirs = e.call(0x0056_9580, &args![other]).u32();
        if mine != theirs {
            dirty = true;
        }
    }

    if get_3d(e, me) != 0 {
        let location = location_of(e, me);
        let node = get_3d(e, me);
        e.call(0x0044_0460, &args![node, location]);
        // a 3x3 matrix (0x30 bytes) and a point (12 bytes) on the stack
        e.with_stack(0x30 + 12, |e, frame| {
            let rotation = e.call(0x0056_fa00, &args![this, frame]).u32();
            let node = get_3d(e, me);
            e.call(0x0043_fa80, &args![node, rotation]);
            reset_collision(e, me, frame.addr() + 0x30);
        });
    }

    let their_cell = e.call(GET_PARENT_CELL, &args![other]).u32();
    if their_cell != 0 {
        let cell = e.call(GET_PARENT_CELL, &args![other]).u32();
        e.call(0x0054_8230, &args![cell, this, 0u32]);
        if get_3d(e, me) == 0 {
            let loader = e.global::<u32>(GLOBAL_IO_MANAGER);
            e.call(0x0045_6520, &args![loader]);
        }
    } else if e.call(GET_PARENT_CELL, &args![this]).u32() != 0 {
        let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        e.call(CELL_REMOVE_REFERENCE, &args![cell, this]);
    }

    let teleport = e.call(0x0056_8e50, &args![this]).u32();
    if teleport != 0 {
        let theirs = e.call(0x0056_8e50, &args![other]).u32();
        let same = theirs != 0
            && e.call(READ_FIRST_DWORD, &args![theirs]).u32()
                == e.call(READ_FIRST_DWORD, &args![teleport]).u32();
        if !same {
            let door = e.call(READ_FIRST_DWORD, &args![teleport]).u32();
            e.call(0x0056_8f30, &args![door]);
            e.call(0x0053_7e90, &args![teleport, 0u32]);
        }
    }

    if e.call(0x0056_9b80, &args![this]).u32() != 0 {
        let parent = e.call(0x0056_9b80, &args![this]).u32();
        let parent_extra = extra_list(e, parent);
        e.call(0x0041_e600, &args![parent_extra, this]);
        e.call(0x0056_9be0, &args![this, 0u32]);
    }

    let list = extra_list(e, me);
    if e.call(0x0041_e780, &args![list]).u32() != 0 {
        let list = extra_list(e, me);
        let mut node = e.call(0x0041_e780, &args![list]).u32();
        while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let entry = e.mem.u32(slot);
            node = e.call(LIST_NEXT, &args![node]).u32();
            let first = e.mem.u32(entry);
            let first_extra = extra_list(e, first);
            e.call(0x0041_ef20, &args![first_extra, this]);
        }
        e.call(0x0056_9df0, &args![this, 0u32]);
    }

    if e.call(0x0040_77c0, &args![other]).bool() {
        let list = extra_list(e, me);
        let mut node = e.call(0x0041_f170, &args![list]).u32();
        while node != 0 {
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            if e.mem.u32(slot) == 0 {
                break;
            }
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let entry = e.mem.u32(slot);
            let first = e.mem.u32(entry);
            let first_extra = extra_list(e, first);
            e.call(0x0041_f4c0, &args![first_extra, this, 0u32]);
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let entry = e.mem.u32(slot);
            let first = e.mem.u32(entry);
            let first_extra = extra_list(e, first);
            e.call(0x0041_f650, &args![first_extra, this, 0u32]);
            node = e.call(LIST_NEXT, &args![node]).u32();
        }
        move_water_lights(e, me, me, false);
    }

    let other_extra = extra_list(e, other);
    let my_extra = extra_list(e, me);
    e.call(0x0041_1ec0, &args![my_extra, other_extra]);

    if e.call(0x0040_77c0, &args![other]).bool() {
        let other_extra = extra_list(e, other);
        let mut node = e.call(0x0041_f170, &args![other_extra]).u32();
        while node != 0 {
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            if e.mem.u32(slot) == 0 {
                break;
            }
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let entry = e.mem.u32(slot);
            if e.mem.u32(entry + 4) & 1 != 0 {
                let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                let entry = e.mem.u32(slot);
                let first = e.mem.u32(entry);
                let first_extra = extra_list(e, first);
                e.call(0x0041_f4c0, &args![first_extra, this, 1u32]);
            }
            let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
            let entry = e.mem.u32(slot);
            if e.mem.u32(entry + 4) & 2 != 0 {
                let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                let entry = e.mem.u32(slot);
                let first = e.mem.u32(entry);
                let first_extra = extra_list(e, first);
                e.call(0x0041_f650, &args![first_extra, this, 1u32]);
            }
            node = e.call(LIST_NEXT, &args![node]).u32();
        }
        move_water_lights(e, me, other, true);
    }

    if e.call(0x0056_9b80, &args![this]).u32() != 0 {
        let parent = e.call(0x0056_9b80, &args![this]).u32();
        let parent_extra = extra_list(e, parent);
        e.call(0x0041_e530, &args![parent_extra, this]);
        e.call(0x0056_9cb0, &args![this, 1u32, 0u32]);
    }

    if !e.call(0x0040_77c0, &args![this]).bool() {
        let list = extra_list(e, me);
        if e.call(0x0041_e780, &args![list]).u32() != 0 {
            let list = extra_list(e, me);
            let mut node = e.call(0x0041_e780, &args![list]).u32();
            while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
                let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
                let entry = e.mem.u32(slot);
                node = e.call(LIST_NEXT, &args![node]).u32();
                let first = e.mem.u32(entry);
                let first_extra = extra_list(e, first);
                e.call(0x0041_edd0, &args![first_extra, this]);
            }
            e.call(0x0056_a1e0, &args![this, 1u32]);
        }
    }

    let scale = e.call(0x0059_8040, &args![other]).f32();
    e.call(0x0056_7490, &args![this, scale]);
    if dirty {
        e.vcall(me, 0x1cc, &args![0u32, 1u32]);
    }

    if get_3d(e, other) == 0 {
        e.vcall(me, 0x1cc, &args![0u32, 1u32]);
    } else {
        if e.call(0x0040_77c0, &args![this]).bool() {
            let base = e.get(data, OBJ_REFR::pObjectReference);
            if !base.is_null() && e.call(FORM_TYPE, &args![base]).i32() == 0x25 {
                let node = e.vcall(base.addr(), 0x14c, &args![this, 0u32, 1u32]).u32();
                // open: the game pushes only `node` for this call; the second
                // argument is stack garbage, passed here as 0.
                e.vcall(me, 0x1cc, &args![node, 0u32]);
            }
        }
        let mut placed = false;
        if get_3d(e, me) == 0 && e.call(GET_PARENT_CELL, &args![this]).u32() != 0 {
            let scene = global_ptr(e, GLOBAL_TEMP_REFR_MANAGER);
            let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
            if e.call(0x0045_11e0, &args![scene, cell, 0u32]).bool() {
                let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
                e.call(0x0045_1ef0, &args![scene, this, cell, 0u32, 0u32]);
                placed = true;
            }
        }
        if !placed {
            let base = e.get(data, OBJ_REFR::pObjectReference);
            if base.is_null() || e.call(FORM_TYPE, &args![base]).i32() != 0x25 {
                e.with_stack(0x30 + 12, |e, frame| {
                    let node = e.vcall(me, 0x1c8, &args![0u32]).u32();
                    let location = location_of(e, me);
                    e.call(0x0044_0460, &args![node, location]);
                    let rotation = e.call(0x0056_fa00, &args![this, frame]).u32();
                    e.call(0x0043_fa80, &args![node, rotation]);
                    let scale = e.call(0x0056_7400, &args![this]).f32();
                    e.call(0x0044_0490, &args![node, scale]);
                    let node_of_other = get_3d(e, other);
                    if e.call(0x0096_11e0, &args![node_of_other]).u32() != 0 {
                        let node_of_other = get_3d(e, other);
                        let collision = e.call(0x0096_11e0, &args![node_of_other]).u32();
                        e.vcall(collision, 0xdc, &args![node, 1u32]);
                    }
                });
            }
        }
        if get_3d(e, me) != 0 && e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
            let base = e.call(GET_BASE_FORM, &args![this]).u32();
            if e.call(FORM_TYPE, &args![base]).i32() == 0x1e {
                let node = get_3d(e, me);
                let base = e.call(GET_BASE_FORM, &args![this]).u32();
                e.call(0x0050_d810, &args![base, this, node, 0u32]);
            }
        }
        let open = e.call(0x0057_2d30, &args![this, 8u32]).bool();
        e.call(0x0047_aec0, &args![this, open as u32, 1u32]);
        e.with_stack(0x18, |e, scratch| {
            reset_collision(e, me, scratch.addr());
        });
        let node = get_3d(e, me);
        e.call(0x00a5_a040, &args![node]);
    }

    if e.call(0x0040_77c0, &args![other]).bool() {
        e.call(0x0057_ada0, &args![this]);
    }
    let mut teleport = e.call(0x0056_8e50, &args![this]).u32();
    if teleport != 0 && !e.call(0x0040_77c0, &args![this]).bool() {
        let door = e.call(READ_FIRST_DWORD, &args![teleport]).u32();
        if door == 0 {
            e.call(0x0056_8f30, &args![this]);
            teleport = 0;
        } else {
            if !e.call(GET_REF_PERSISTS, &args![door]).bool() {
                e.call(0x0056_5480, &args![door, 1u32]);
            }
            let mut door_teleport = e.call(0x0056_8e50, &args![door]).u32();
            if door_teleport == 0 {
                door_teleport = e.call(0x0056_8e70, &args![door]).u32();
            }
            e.call(0x0053_7e90, &args![door_teleport, this]);
            let mut removed = false;
            if e.call(0x0056_9140, &args![this]).u32() != 0
                && e.call(0x0056_9140, &args![door]).u32() != 0
            {
                let door_extra = extra_list(e, door);
                e.call(0x0041_ae70, &args![door_extra]);
                removed = true;
            }
            if e.call(0x0056_7770, &args![this]).u32() != 0
                && e.call(0x0056_7770, &args![door]).u32() != 0
            {
                e.call(0x0056_7c50, &args![door]);
                removed = true;
            }
            if removed {
                e.call(MESSAGE, &args![SHARED_DATA_REMOVED_MESSAGE]);
            }
        }
    }
    let _ = teleport;
    if !e.call(0x0040_77c0, &args![this]).bool() {
        let parent = e.call(0x0056_a9f0, &args![this]).u32();
        if parent != 0 {
            let parent = e.call(0x0056_a9f0, &args![this]).u32();
            let parent_extra = extra_list(e, parent);
            e.call(0x0041_dcd0, &args![parent_extra, this]);
        }
    }
}

// Translated from 0055d230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x10c (`Compare` in the Xbox PDB order): whether `other`
/// differs from this reference. True when `other` is not a reference, when
/// the form components differ (`TESForm::CompareAllComponents`), when the
/// 0x1c bytes of `OBJ_REFR` data differ (`memcmp`), when the parent cells
/// differ, when the extra lists differ (`ExtraDataList::CompareList`) or when
/// the float `00598040` returns (the scale) differs (a NaN counts as
/// different).
pub fn fn_0055d230(e: &mut Engine, this: Ptr<TESObjectREFR>, other: Ptr) -> bool {
    let me = this.addr();
    let other = dynamic_cast(e, other.addr(), TYPE_TES_OBJECT_REFR);
    if other == 0 {
        return true;
    }
    if e.call(0x0048_5270, &args![this, other]).bool() {
        return true;
    }
    let other_data = e.call(GET_DATA, &args![other]).u32();
    if e.call(MEMCMP, &args![me + 0x20, other_data, 0x1cu32]).i32() != 0 {
        return true;
    }
    let mine = e.call(GET_PARENT_CELL, &args![this]).u32();
    let theirs = e.call(GET_PARENT_CELL, &args![other]).u32();
    if mine != theirs {
        return true;
    }
    let other_extra = extra_list(e, other);
    if e.call(0x0041_27e0, &args![me + EXTRA_OFFSET, other_extra])
        .bool()
    {
        return true;
    }
    let mine = e.call(0x0059_8040, &args![this]).f32() as f64;
    let theirs = e.call(0x0059_8040, &args![other]).f32() as f64;
    mine != theirs
}

// Translated from 0055d310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::HasContainer` (Xbox PDB): the container component of the
/// base object, or null. A base object of form type 0x1b (container) gives
/// its component at +0x30; types 0x2a and 0x2b (actor bases) give theirs at
/// +0x64.
pub fn tes_object_refr_has_container(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let mut component = 0;
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        let kind = e.call(FORM_TYPE, &args![base]).i32();
        if kind == 0x1b {
            let base = e.call(GET_BASE_FORM, &args![this]).u32();
            component = if base != 0 { base + 0x30 } else { 0 };
        } else if kind > 0x29 && kind <= 0x2b {
            let base = e.call(GET_BASE_FORM, &args![this]).u32();
            component = if base != 0 { base + 0x64 } else { 0 };
        }
    }
    component
}

// Translated from 0055d3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `form` is this reference's base object or one of the two bases its
/// extra data remembers (`004216f0` and `00421720`, the leveled creature's
/// original base and its template).
pub fn fn_0055d3b0(e: &mut Engine, this: Ptr<TESObjectREFR>, form: Ptr) -> bool {
    let list = extra_list(e, this.addr());
    let original = e.call(0x0042_16f0, &args![list]).u32();
    let list = extra_list(e, this.addr());
    let template = e.call(0x0042_1720, &args![list]).u32();
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    base == form.addr() || original == form.addr() || template == form.addr()
}

// Translated from 0055d420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0xc8 (`SetAltered` in the Xbox PDB order): unless the form is
/// flagged (`004077c0`), with `flag` set it first marks the cell the
/// `TESChildCell` base reports as altered (`SetAltered(true)` on it), then
/// passes `flag` to `TESForm`'s own handler (`00484730`).
pub fn fn_0055d420(e: &mut Engine, this: Ptr<TESObjectREFR>, flag: u8) {
    if !e.call(0x0040_77c0, &args![this]).bool() && flag != 0 {
        let cell = save_parent_cell(e, this);
        if cell != 0 {
            e.vcall(cell, 0xc8, &args![1u32]);
        }
    }
    e.call(0x0048_4730, &args![this, flag as u32]);
}

// Translated from 0055d480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x130 (`GetObjectTypeName` in the Xbox PDB order): the name of
/// the reference. A reference for which `00474cb0` is true gives
/// `BaseProcess::GetCurrentHeadTrackTypeString`'s body (`00401280`). One
/// without a base object, or while the data handler says so (`004226e0`),
/// gives the empty string. Otherwise it is the base object's own name
/// (virtual +0x130), or, when that is empty, the leveled creature's original
/// base's.
pub fn fn_0055d480(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    if e.call(0x0047_4cb0, &args![this]).u32() != 0 {
        return e.call(0x0040_1280, &args![this]).u32();
    }
    let data = this.at(TESObjectREFR::data);
    let base = e.get(data, OBJ_REFR::pObjectReference);
    let handler = global_ptr(e, GLOBAL_DATA_HANDLER);
    if base.is_null() || e.call(0x0042_26e0, &args![handler]).bool() {
        return EMPTY_STRING;
    }
    let mut name = e.vcall(base.addr(), 0x130, &args![]).u32();
    if name == 0 || e.mem.i8(name) == 0 {
        let list = extra_list(e, this.addr());
        let original = e.call(0x0042_16f0, &args![list]).u32();
        if original != 0 {
            name = e.vcall(original, 0x130, &args![]).u32();
        }
    }
    name
}

// Translated from 0055d520 (decompiled, FalloutNV.exe 1.4.0.525)
/// The reference's full name: `TESFullName::GetFullName` of the base object.
pub fn fn_0055d520(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let data = e.call(GET_DATA, &args![this]).u32();
    let base = e.mem.u32(data);
    e.call(0x0048_2720, &args![base]).u32()
}

// Translated from 0055d540 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetFormDetailedString` (Xbox PDB), virtual at +0x90:
/// writes `"<kind> Form '<name>' (<id>)<to base...> in Cell <cell string>"`
/// into the `BSStringT` `out`. It builds a first fragment describing the base
/// object (`" to %s form '%s' (%08X)"`), a second describing the world space
/// (`" in WorldSpace '%s' (%08X)"`; formatted but not used in the final
/// text), asks the parent cell (virtual +0x90) for its own string, and
/// formats the lot with `BSStringT::Format`.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn tes_object_refr_get_form_detailed_string(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    out: Ptr,
) {
    // Two 0x110-byte `char` buffers on the stack.
    e.with_stack(0x220, |e, buffers| {
        let first = buffers.addr();
        let second = buffers.addr() + 0x110;
        e.mem.set_u8(first, 0);
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        if base != 0 {
            let base_id = e.call(FORM_ID, &args![base]).u32();
            let name = e.vcall(base, 0x130, &args![]).u32();
            let kind = e.call(0x0044_0e30, &args![base]).u32();
            e.call(
                SPRINTF,
                &args![first, DETAILED_BASE_FORMAT, kind, name, base_id],
            );
        }
        e.mem.set_u8(second, 0);
        let world = e.call(0x0057_5d70, &args![this]).u32();
        if world != 0 {
            let world_id = e.call(FORM_ID, &args![world]).u32();
            let name = e.vcall(world, 0x130, &args![]).u32();
            e.call(
                SPRINTF,
                &args![second, DETAILED_WORLD_FORMAT, name, world_id],
            );
        }
        e.with_stack(8, |e, cell_text| {
            e.call(STRING_CONSTRUCT, &args![cell_text]);
            let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
            if cell != 0 {
                e.vcall(cell, 0x90, &args![cell_text]);
            }
            let cell_string = e.call(READ_FIRST_DWORD, &args![cell_text]).u32();
            let id = e.call(FORM_ID, &args![this]).u32();
            let head_track = e.call(0x0040_1280, &args![this]).u32();
            let kind = e.call(0x0044_0e30, &args![this]).u32();
            e.call(
                STRING_FORMAT,
                &args![
                    out,
                    DETAILED_FORMAT,
                    kind,
                    head_track,
                    id,
                    first,
                    cell_string
                ],
            );
            e.call(STRING_DESTRUCT, &args![cell_text]);
        });
    });
}

// Translated from 0055d6d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Not a method (cdecl, two arguments): if `object` is not null and its
/// virtual at +0x10 gives a non-null `target`, makes the target's flag 9
/// agree with `refr`'s `00565090` test. When the test holds, flag 9 is set on
/// the target and `0054b800` is run on it; when it does not and flag 9 was
/// set, the flag becomes the LOD multiplier of the reference's base
/// (`TES::GetLODMult`, `0045c6b0`) and `00476ab0` runs on the target.
pub fn fn_0055d6d0(e: &mut Engine, refr: Ptr<TESObjectREFR>, object: Ptr) {
    if object.is_null() {
        return;
    }
    let target = e.vcall(object.addr(), 0x10, &args![]).u32();
    if target == 0 {
        return;
    }
    let wanted = e.call(0x0056_5090, &args![refr]).u8();
    let has_flag = (e.call(0x0070_58c0, &args![target]).i32() == 9) as u8;
    if wanted == has_flag {
        return;
    }
    if wanted != 0 {
        e.call(0x0070_5b10, &args![target, 9u32]);
        e.call(0x0054_b800, &args![target]);
    } else {
        let base = e.call(GET_BASE_FORM, &args![refr]).u32();
        let lod = e.call(0x0045_c6b0, &args![base]).u32();
        e.call(0x0070_5b10, &args![target, lod]);
        e.call(0x0047_6ab0, &args![target]);
    }
}

// Translated from 0055d760 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::InitItem` (Xbox PDB), virtual at +0x88: finishes loading
/// a reference. Unless `004013e0` says to skip: radio-station activators
/// get their station set up, warnings are counted (`MessageHandler::
/// IncDisableWarningCount`) while it runs, a leveled actor base is reported,
/// a deleted base marks the reference deleted, a corrupt position or rotation
/// (infinite or NaN) is reported and reset to the zero vector, and the
/// `ExtraDataList` is initialized (`00416be0`) with health scaled by the
/// base's health, ammo stacks of one counted from the base, a multi-bound
/// marker kept in step with a non-unit scale (and `TESWorldSpace::
/// AddMultiBoundRef`), actors given their life state, process-list entry,
/// guard flag and the player the camera-speed hook, non-actors told their
/// starting position, the enable-state parent propagated, the rotation
/// clamped, and the `MASTERFILE: Errors were encountered during InitItem`
/// message written (three shapes: with a cell and a world, a cell only, no
/// cell) when warnings were raised meanwhile. Finally it initializes the
/// inventory changes of a container, the script (`InitScript`) and hides the
/// terrain tree of a disabled landscape object.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn tes_object_refr_init_item(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let data = this.at(TESObjectREFR::data);
    if e.call(0x0040_13e0, &args![this]).bool() {
        return;
    }
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    if e.call(0x004f_ede0, &args![base, 0u32]).bool() {
        e.call(0x004f_f150, &args![this]);
    }
    let warnings_before = e.call(0x0046_e8a0, &args![]).u32();
    e.call(0x0043_b2b0, &args![1u32]);
    e.call(0x004f_ffe0, &args![0u32]);

    let base = e.get(data, OBJ_REFR::pObjectReference);
    if !base.is_null() {
        let kind = e.call(FORM_TYPE, &args![base]).i32();
        if (0x2c..=0x2d).contains(&kind) {
            e.call(MESSAGE, &args![LEVELED_BASE_MESSAGE]);
        }
    }
    let base = e.get(data, OBJ_REFR::pObjectReference);
    if !base.is_null() && e.call(FORM_IS_DELETED, &args![base]).bool() {
        e.vcall(me, 0xc4, &args![1u32]);
    }

    if vector_is_corrupt(e, me + 0x30) {
        e.call(MESSAGE, &args![CORRUPT_LOCATION_MESSAGE]);
        reset_vector(e, me + 0x30);
    }
    if vector_is_corrupt(e, me + 0x24) {
        e.call(MESSAGE, &args![CORRUPT_ANGLE_MESSAGE]);
        reset_vector(e, me + 0x24);
    }

    let list = extra_list(e, me);
    e.call(0x0041_6be0, &args![list, this]);
    let list = extra_list(e, me);
    let health_percent = e.call(0x0041_b550, &args![list]).f32();
    let one: f64 = e.global(CONSTANT_ONE);
    if health_percent as f64 != one {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        let health = e.call(0x0048_73d0, &args![base]).u32();
        let health = health as f32;
        let new_health = (health_percent as f64 * health as f64) as f32;
        let list = extra_list(e, me);
        e.call(0x0041_9970, &args![list, new_health]);
    }
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    if e.call(FORM_TYPE, &args![base]).i32() == 0x29 {
        let list = extra_list(e, me);
        if e.call(0x0041_8770, &args![list]).u16() as i16 == 1 {
            let base = e.call(GET_BASE_FORM, &args![this]).u32();
            let count = e.call(FORM_TYPE, &args![base + 0x80]).u32();
            let list = extra_list(e, me);
            e.call(0x0041_9ad0, &args![list, count]);
        }
    }

    if e.call(0x0043_9f90, &args![this]).bool() {
        let scale = e.call(0x0056_7400, &args![this]).f32();
        if scale as f64 != one {
            let mut marker = e.call(0x0056_9840, &args![this]).u32();
            if marker == 0 {
                let block = e.call(0x0040_1000, &args![0xcu32]).u32();
                marker = if block != 0 {
                    e.call(0x0043_8f50, &args![block]).u32()
                } else {
                    0
                };
                let list = extra_list(e, me);
                e.call(0x0042_1f30, &args![list, marker]);
            }
            let position = e.call(LIST_ITEM_SLOT, &args![marker]).u32();
            e.with_stack(12, |e, local| {
                for i in 0..3 {
                    let word = e.mem.u32(position + 4 * i);
                    e.mem.set_u32(local.addr() + 4 * i, word);
                }
                let scale = e.call(0x0056_7400, &args![this]).f32();
                e.call(0x0043_9180, &args![local, scale]);
                e.call(0x0098_ddd0, &args![marker, local]);
            });
            e.call(0x0056_7490, &args![this, 1.0f32]);
        }
        let world = e.call(0x0057_5d70, &args![this]).u32();
        if world != 0 {
            e.call(0x0058_78d0, &args![world, this]);
        }
    }

    let actor = if e.vcall(me, 0x100, &args![]).bool() {
        me
    } else {
        0
    };
    if actor == 0 {
        if e.call(0x0057_2c80, &args![this]).bool() {
            let zero = [
                e.global::<u32>(ZERO_VECTOR),
                e.global::<u32>(ZERO_VECTOR + 4),
                e.global::<u32>(ZERO_VECTOR + 8),
            ];
            e.vcall(me, 0x174, &args![zero[0], zero[1], zero[2]]);
        }
    } else {
        if !e.vcall(actor, 0x290, &args![]).bool() {
            e.vcall(actor, 0x46c, &args![]);
        }
        if !e.vcall(actor, 0x360, &args![]).bool() && e.call(0x0057_22c0, &args![this, 0u32]).bool()
        {
            e.call(0x008a_1800, &args![actor, 2u32]);
            e.call(0x0096_d470, &args![OBJECT_PROCESS_LISTS, actor, 3u32]);
        }
        if e.vcall(actor, 0x218, &args![]).bool() {
            let class = e.call(0x0088_4350, &args![actor]).u32();
            let mut guard = 0u8;
            if class != 0 {
                guard = e.call(0x005f_6e60, &args![class]).u8();
            }
            e.vcall(actor, 0x308, &args![guard as u32]);
        }
        let actor_data = e.call(0x0041_81e0, &args![actor]).u32();
        let base_data = actor_data + 0x30;
        if e.call(0x0047_cdb0, &args![base_data]).bool()
            && !e.call(0x0056_afc0, &args![this]).bool()
        {
            e.call(0x0047_ce10, &args![base_data, this]);
        }
        let flag = fn_0055e200(e, Ptr::new(base_data));
        e.vcall(actor, 0x1f8, &args![flag as u32]);
        let player = global_ptr(e, GLOBAL_PLAYER);
        if actor == player {
            let movement = if player != 0 { player + 0xa4 } else { 0 };
            let speed = e
                .call(
                    0x0064_7f00,
                    &args![movement, 0u32, 0u32, 0u32, 1u32, 0u32, 0u32],
                )
                .f32();
            e.call(0x0055_e230, &args![speed]);
        }
    }

    let saved = {
        let loading = global_ptr(e, GLOBAL_LOADING_OBJECT);
        e.call(0x0046_23f0, &args![loading, 1u32]).u8()
    };
    let save_load = global_ptr(e, GLOBAL_SAVE_LOAD);
    let first = e.call(0x0047_c850, &args![save_load]).bool();
    let run = !first || e.call(0x0047_c850, &args![save_load]).bool();
    if run {
        let enable_parent = e.call(0x0056_a9f0, &args![this]).u32();
        if enable_parent != 0 {
            let previous = e.call(0x0047_c850, &args![save_load]).u8();
            if !e.call(0x0047_c850, &args![save_load]).bool() {
                e.call(0x0045_34f0, &args![save_load, 0u32]);
            }
            if e.call(0x0056_aa70, &args![this]).bool() {
                let disabled = e.call(FORM_IS_DISABLED, &args![enable_parent]).bool();
                e.call(0x0048_4af0, &args![this, (!disabled) as u32]);
            } else {
                let disabled = e.call(FORM_IS_DISABLED, &args![enable_parent]).u8();
                e.call(0x0048_4af0, &args![this, disabled as u32]);
            }
            if e.call(FORM_IS_DISABLED, &args![this]).bool()
                && e.call(0x0056_4e60, &args![this]).bool()
                && e.call(GET_PARENT_CELL, &args![this]).u32() != 0
            {
                let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
                fn_0055e1d0(e, Ptr::new(cell));
            }
            e.call(0x0045_34f0, &args![save_load, previous as u32]);
        }
    }
    e.call(0x0056_ac00, &args![this]);
    e.call(
        0x0046_23f0,
        &args![global_ptr(e, GLOBAL_LOADING_OBJECT), saved as u32],
    );
    let base = e.get(data, OBJ_REFR::pObjectReference);
    let destructible = e.call(0x0047_53d0, &args![base]).u8();
    e.call(0x0048_46a0, &args![this, destructible as u32]);
    let angle = e.mem.f32(me + 0x2c);
    let clamped = e.call(0x004b_1480, &args![angle]).f32();
    e.mem.set_f32(me + 0x2c, clamped);
    e.call(0x0048_4ab0, &args![this, 1u32]);
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        if e.call(FORM_TYPE, &args![base]).i32() != 0x28 {
            let list = extra_list(e, me);
            if e.call(EXTRA_GET_DATA, &args![list, 0x6eu32]).u32() != 0 {
                let list = extra_list(e, me);
                e.call(EXTRA_REMOVE_TYPE, &args![list, 0x6eu32]);
            }
        }
    }
    e.call(0x0043_b2b0, &args![0u32]);
    let warnings = e.call(0x0046_e8a0, &args![]).u32();
    e.call(0x004f_ffe0, &args![warnings_before]);
    if warnings != 0 {
        report_init_item_errors(e, this);
    }

    let container = tes_object_refr_has_container(e, this);
    if container != 0 {
        let changes = e.call(0x004b_f220, &args![this]).u32();
        e.call(0x004d_1440, &args![changes]);
        e.call(0x004d_1610, &args![changes]);
        e.call(0x004d_1960, &args![changes]);
        if e.call(0x0042_cde0, &args![changes]).bool() {
            let list = extra_list(e, me);
            e.call(0x0041_aeb0, &args![list]);
        }
    }
    e.call(0x0056_5730, &args![this]);
    e.call(0x0046_a010, &args![this, 0u32]);
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    if e.call(0x0054_9580, &args![base]).bool() && e.call(FORM_IS_DISABLED, &args![this]).bool() {
        let world = e.call(0x0057_5d70, &args![this]).u32();
        if world != 0 {
            let terrain = e.call(0x0058_6170, &args![world]).u32();
            e.call(0x006f_cfa0, &args![terrain, this, 1u32]);
        }
    }
}

// Translated from 0055e1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Decrements the 16-bit counter at +0xa8 of `this` (`InitItem` calls it on
/// the reference's parent cell).
pub fn fn_0055e1d0(e: &mut Engine, this: Ptr) {
    let count = e.mem.u16(this.addr() + 0xa8);
    e.mem.set_u16(this.addr() + 0xa8, count.wrapping_sub(1));
}

// Translated from 0055e200 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `00461580(0x200)` on `this` is false.
pub fn fn_0055e200(e: &mut Engine, this: Ptr) -> bool {
    !e.call(0x0046_1580, &args![this, 0x200u32]).bool()
}

// Translated from 0055e250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x3c (`SavesBefore` in the Xbox PDB order): whether this
/// reference's record is written before `form`'s in a save.
///
/// A form whose type `005548a0` rejects is left to the parent cell's own
/// virtual +0x3c. Otherwise `form` is cast to the child-cell interface and
/// asked for its cell: if that differs from this reference's cell, the cell
/// decides (virtual +0x3c on this reference's cell, given the other cell).
/// With the same cell, form types 0x3a to 0x40 and 0x69 are before only when
/// this reference persists and `form` (as a reference) does not; types 0x42
/// and 0x43 whenever this one persists; other types never.
pub fn fn_0055e250(e: &mut Engine, this: Ptr<TESObjectREFR>, form: Ptr) -> bool {
    let kind = e.call(FORM_TYPE, &args![form]).u32();
    if !e.call(0x0055_48a0, &args![kind]).bool() {
        let cell = save_parent_cell(e, this);
        return e.vcall(cell, 0x3c, &args![form]).bool();
    }
    let child = dynamic_cast(e, form.addr(), TYPE_TES_CHILD_CELL);
    let their_cell = e.vcall(child, 0, &args![]).u32();
    let my_cell = save_parent_cell(e, this);
    if their_cell != my_cell {
        let cell = save_parent_cell(e, this);
        return e.vcall(cell, 0x3c, &args![their_cell]).bool();
    }
    let kind = e.call(FORM_TYPE, &args![form]).u32();
    match kind {
        0x3a..=0x40 | 0x69 => {
            let as_reference = dynamic_cast(e, form.addr(), TYPE_TES_OBJECT_REFR);
            if !e.call(GET_REF_PERSISTS, &args![this]).bool() {
                false
            } else {
                as_reference != 0 && !e.call(GET_REF_PERSISTS, &args![as_reference]).bool()
            }
        }
        0x42 | 0x43 => e.call(GET_REF_PERSISTS, &args![this]).bool(),
        _ => false,
    }
}

// Translated from 0055e3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x38 (`SavesBefore` in the Xbox PDB order): the same question
/// for a group-data record `group` (the record `fn_0055e5e0` fills; null or a
/// record of another class gives false). For kinds 8 and 9 it is true only
/// when the record's form is this reference's own cell (looked up and cast
/// to `TESObjectCELL`), this reference persists and the kind is 9. Any other
/// kind is left to the parent cell's virtual +0x38.
pub fn fn_0055e3f0(e: &mut Engine, this: Ptr<TESObjectREFR>, group: Ptr) -> bool {
    if group.is_null() || e.mem.u32(group.addr()) != e.global::<u32>(GROUP_DATA_VTABLE_PTR) {
        return false;
    }
    let kind = e.mem.u32(group.addr() + 0xc);
    if (8..=9).contains(&kind) {
        let form_id = e.mem.u32(group.addr() + 8);
        let form = e.call(0x0048_39c0, &args![form_id]).u32();
        let cell = dynamic_cast(e, form, TYPE_TES_OBJECT_CELL);
        if cell == 0 {
            return false;
        }
        if save_parent_cell(e, this) != cell {
            return false;
        }
        if e.call(GET_REF_PERSISTS, &args![this]).bool() {
            return e.mem.u32(group.addr() + 0xc) == 9;
        }
        false
    } else {
        let cell = save_parent_cell(e, this);
        e.vcall(cell, 0x38, &args![group]).bool()
    }
}

// Translated from 0055e4d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x110 (`BelongsInGroup` in the Xbox PDB order): whether this
/// reference belongs in the group-data record `group` (the class
/// `fn_0055e5e0` produces; anything else gives false). `first_flag` and
/// `second_flag` are the caller's two bytes.
///
/// * kinds 6, 8 and 9: kind 6 needs `first_flag`; then the record's form must
///   be the parent cell (`00485be0`). Then it is true when `second_flag` is
///   clear or the kind is 6, and otherwise kind 8 when this reference persists
///   or kind 9 when it does not;
/// * every other kind, with `first_flag` set, is handed to the parent cell's
///   virtual +0x110.
pub fn fn_0055e4d0(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    group: Ptr,
    first_flag: u8,
    second_flag: u8,
) -> bool {
    if group.is_null() || e.mem.u32(group.addr()) != e.global::<u32>(GROUP_DATA_VTABLE_PTR) {
        return false;
    }
    let cell = save_parent_cell(e, this);
    let kind = e.mem.u32(group.addr() + 0xc);
    if kind == 6 || (kind > 7 && kind <= 9) {
        if kind == 6 && first_flag == 0 {
            return false;
        }
        let form_id = e.mem.u32(group.addr() + 8);
        if !e.call(0x0048_5be0, &args![cell, form_id]).bool() {
            return false;
        }
        if second_flag == 0 || e.mem.u32(group.addr() + 0xc) == 6 {
            return true;
        }
        if e.call(GET_REF_PERSISTS, &args![this]).bool() {
            e.mem.u32(group.addr() + 0xc) == 8
        } else {
            e.mem.u32(group.addr() + 0xc) == 9
        }
    } else if first_flag != 0 {
        e.vcall(
            cell,
            0x110,
            &args![group, first_flag as u32, second_flag as u32],
        )
        .bool()
    } else {
        false
    }
}

// Translated from 0055e5e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual at +0x114 (`CreateGroupData` in the Xbox PDB order): fills the
/// group-data record `out` (0x14 bytes: vtable, +4, form ID +8, kind +0xc,
/// +0x10) for the group record `parent`. `out[0]` is cleared first. A parent
/// of kind 1 (when `005516c0` holds), 1, 3 or 5 whose form is the parent
/// cell's `00544210` gives kind 6; a parent of kind 6 whose form is this
/// reference's parent cell gives kind 8 when this reference persists, else 9.
pub fn fn_0055e5e0(e: &mut Engine, this: Ptr<TESObjectREFR>, out: Ptr, parent: Ptr) {
    if out.is_null() {
        return;
    }
    e.mem.set_u32(out.addr(), 0);
    if parent.is_null() {
        return;
    }
    let cell = save_parent_cell(e, this);
    let cell_id = e.call(FORM_ID, &args![cell]).u32();
    let fill = |e: &mut Engine, kind: u32| {
        let vtable = e.global::<u32>(GROUP_DATA_VTABLE_PTR);
        e.mem.set_u32(out.addr(), vtable);
        e.mem.set_u32(out.addr() + 0xc, kind);
        e.mem.set_u32(out.addr() + 8, cell_id);
        e.mem.set_u32(out.addr() + 4, 0);
        e.mem.set_u32(out.addr() + 0x10, 0);
    };
    let parent_kind = e.mem.u32(parent.addr() + 0xc).wrapping_sub(1);
    match parent_kind {
        0 | 2 | 4 => {
            let mut matches = false;
            if parent_kind == 0 && e.call(0x0055_16c0, &args![cell]).bool() {
                matches = true;
            }
            let wanted = e.call(0x0054_4210, &args![cell]).u32();
            if e.mem.u32(parent.addr() + 8) == wanted {
                matches = true;
            }
            if matches {
                fill(e, 6);
            }
        }
        5 if e.mem.u32(parent.addr() + 8) == cell_id => {
            let kind = if e.call(GET_REF_PERSISTS, &args![this]).bool() {
                8
            } else {
                9
            };
            fill(e, kind);
        }
        _ => {}
    }
}

// Translated from 0055e730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetSaveSize` (Xbox PDB), virtual at +0x50: the number of
/// bytes `Save` writes for the change flags `flags` (16 bits, wrapping).
/// Starts from `TESForm`'s size (`00484bf0`) and adds: 6 when the game uses
/// save-game blocks, the container changes' size (flag 0x20), the actor
/// extras' size (for an actor), `2 +` `0055f880` (flag 0x10000000 on a
/// non-actor), `2 +` `00560350` (flag 4) and 4 (flag 0x10 with a save
/// version of 0x43 or more). With the debug switch (`011de4e8`'s byte) set it
/// logs the size it computed (`GetSaveSize(): ...`).
pub fn fn_0055e730(e: &mut Engine, this: Ptr<TESObjectREFR>, flags: u32) -> u16 {
    let me = this.addr();
    let base = e.call(0x0048_4bf0, &args![this, flags]).u16();
    let mut total: u16 = base;
    let save_load = global_ptr(e, GLOBAL_SAVE_LOAD);
    if e.call(0x0086_2110, &args![save_load]).bool() {
        total = total.wrapping_add(4);
        total = total.wrapping_add(2);
    }
    if flags & 0x20 != 0 {
        let list = extra_list(e, me);
        let changes = e.call(0x0041_8520, &args![list]).u32();
        let size = e.call(0x004d_3960, &args![changes]).u16();
        total = total.wrapping_add(size);
    }
    // (the code also tests `flags & 0`, which is never set)
    if is_actor(e, me) {
        let list = extra_list(e, me);
        let size = e.call(0x0042_2c40, &args![list, flags, this]).u16();
        total = total.wrapping_add(size);
    }
    if flags & 0x1000_0000 != 0 && !is_actor(e, me) {
        total = total.wrapping_add(2);
        let size = e.call(0x0055_f880, &args![this]).u16();
        total = total.wrapping_add(size);
    }
    if flags & 4 != 0 {
        total = total.wrapping_add(2);
        let size = e.call(0x0056_0350, &args![this, 0u32]).u16();
        total = total.wrapping_add(size);
    }
    let version = e.call(0x008d_f040, &args![save_load]).u8();
    if version >= 0x43 && flags & 0x10 != 0 {
        total = total.wrapping_add(4);
    }
    let switch = e.call(0x0040_8d60, &args![GLOBAL_DEBUG_SWITCHES]).u32();
    if e.mem.u8(switch) != 0 {
        let world = e.call(0x004f_d3e0, &args![save_load]).u32();
        let written = (total as u32).wrapping_sub(base as u32);
        if world != 0 {
            let form_id = e.mem.u32(world);
            let form = e.call(0x0048_39c0, &args![form_id]).u32();
            let name = e.vcall(form, 0x130, &args![]).u32();
            let extra = e.mem.u32(world + 5);
            e.call(
                ERROR_LOG,
                &args![
                    SAVE_SIZE_FORMAT,
                    written,
                    form_id,
                    name,
                    extra,
                    SOURCE_LINE,
                    SOURCE_FILE
                ],
            );
        } else {
            e.call(
                ERROR_LOG,
                &args![SAVE_SIZE_SHORT_FORMAT, written, SOURCE_LINE, SOURCE_FILE],
            );
        }
    }
    total
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00426720, fn_00426720(Ptr, u32, u32, Ptr)),
        entry!(0x004267d0, fn_004267d0(Ptr, u32, Ptr)),
        entry!(0x004269c0, fn_004269c0(Ptr, u32, Ptr)),
        entry!(
            0x004b0130,
            ni_t_pointer_map_tes_object_refr_p_bool_delete_item(Ptr, Ptr)
        ),
        entry!(
            0x0055a2f0,
            tes_object_refr_tes_object_refr(Ptr<TESObjectREFR>) -> Ptr<TESObjectREFR>
        ),
        entry!(0x0055a3b0, fn_0055a3b0(Ptr<TESObjectREFR>) -> u32),
        entry!(
            0x0055a3d0,
            fn_0055a3d0(Ptr<TESObjectREFR>, u32) -> Ptr<TESObjectREFR>
        ),
        entry!(0x0055a400, fn_0055a400(Ptr<OBJ_REFR>) -> Ptr<OBJ_REFR>),
        entry!(0x0055a430, fn_0055a430(Ptr<TESObjectREFR>)),
        entry!(0x0055abe0, fn_0055abe0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x0055ac00, fn_0055ac00(Ptr) -> Ptr),
        entry!(0x0055ac20, fn_0055ac20(Ptr)),
        entry!(0x0055ac40, fn_0055ac40(Ptr, Ptr<TESObjectREFR>)),
        entry!(0x0055ac70, fn_0055ac70() -> u32),
        entry!(0x0055ac80, fn_0055ac80()),
        entry!(0x0055ac90, fn_0055ac90(Ptr<TESObjectREFR>)),
        entry!(0x0055ad20, tes_object_refr_clear_data(Ptr<TESObjectREFR>)),
        entry!(0x0055ae70, tes_object_refr_save(Ptr<TESObjectREFR>)),
        entry!(
            0x0055af90,
            tes_object_refr_load(Ptr<TESObjectREFR>, Ptr) -> bool
        ),
        entry!(0x0055b980, fn_0055b980(Ptr) -> u32),
        entry!(0x0055b9a0, fn_0055b9a0(Ptr) -> bool),
        entry!(0x0055b9c0, fn_0055b9c0(Ptr<TESObjectREFR>, Ptr)),
        entry!(
            0x0055c050,
            tes_object_refr_create_duplicate_form(
                Ptr<TESObjectREFR>,
                u32,
                Ptr,
            ) -> Ptr<TESObjectREFR>
        ),
        entry!(0x0055c3e0, tes_object_refr_copy(Ptr<TESObjectREFR>, Ptr)),
        entry!(0x0055d230, fn_0055d230(Ptr<TESObjectREFR>, Ptr) -> bool),
        entry!(
            0x0055d310,
            tes_object_refr_has_container(Ptr<TESObjectREFR>) -> u32
        ),
        entry!(0x0055d3b0, fn_0055d3b0(Ptr<TESObjectREFR>, Ptr) -> bool),
        entry!(0x0055d420, fn_0055d420(Ptr<TESObjectREFR>, u8)),
        entry!(0x0055d480, fn_0055d480(Ptr<TESObjectREFR>) -> u32),
        entry!(0x0055d520, fn_0055d520(Ptr<TESObjectREFR>) -> u32),
        entry!(
            0x0055d540,
            tes_object_refr_get_form_detailed_string(Ptr<TESObjectREFR>, Ptr)
        ),
        entry!(0x0055d6d0, fn_0055d6d0(Ptr<TESObjectREFR>, Ptr)),
        entry!(0x0055d760, tes_object_refr_init_item(Ptr<TESObjectREFR>)),
        entry!(0x0055e1d0, fn_0055e1d0(Ptr)),
        entry!(0x0055e200, fn_0055e200(Ptr) -> bool),
        entry!(0x0055e250, fn_0055e250(Ptr<TESObjectREFR>, Ptr) -> bool),
        entry!(0x0055e3f0, fn_0055e3f0(Ptr<TESObjectREFR>, Ptr) -> bool),
        entry!(
            0x0055e4d0,
            fn_0055e4d0(Ptr<TESObjectREFR>, Ptr, u8, u8) -> bool
        ),
        entry!(0x0055e5e0, fn_0055e5e0(Ptr<TESObjectREFR>, Ptr, Ptr)),
        entry!(0x0055e730, fn_0055e730(Ptr<TESObjectREFR>, u32) -> u16),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Registers `addrs` as doubles that do nothing and return 0.
    fn stub(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// A double that returns `value` in `eax`.
    fn returns(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| Ret {
            eax: value,
            ..Ret::default()
        });
    }

    /// An engine with the small accessors the unit leans on registered with
    /// the bodies the exe gives them, and the pages of the globals and
    /// vtables the tests set mapped.
    fn engine() -> Engine {
        let mut e = Engine::new();
        // 005d43c0: this + 0x44
        e.register(GET_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        // 006815c0: returns its `this`
        e.register(LIST_ITEM_SLOT, |_, a| a[0].into_ret());
        // 00726070: the node's next pointer
        e.register(LIST_NEXT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        // 00559450: the first dword
        e.register(READ_FIRST_DWORD, |e, a| e.mem.u32(a[0]).into_ret());
        // 008d6f30, 007af430, 00891170: pParentCell, pObjectReference, &data
        e.register(GET_PARENT_CELL, |e, a| e.mem.u32(a[0] + 0x40).into_ret());
        e.register(GET_BASE_FORM, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(GET_DATA, |_, a| (a[0] + 0x20).into_ret());
        // 00401170 and 0084e3a0: the form type byte and the form ID
        e.register(FORM_TYPE, |e, a| u32::from(e.mem.u8(a[0] + 4)).into_ret());
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        // form flag tests: 0x20 deleted, 0x800 disabled, 0x4000, 0x8
        e.register(FORM_IS_DELETED, |e, a| {
            (e.mem.u32(a[0] + 8) & 0x20 != 0).into_ret()
        });
        e.register(FORM_IS_DISABLED, |e, a| {
            (e.mem.u32(a[0] + 8) & 0x800 != 0).into_ret()
        });
        e.register(0x0040_77c0, |e, a| {
            (e.mem.u32(a[0] + 8) & 0x4000 != 0).into_ret()
        });
        e.register(0x0040_13e0, |e, a| {
            (e.mem.u32(a[0] + 8) & 8 != 0).into_ret()
        });
        // 0041210b's flag test (the loading flag) is off unless a test turns it on
        stub(&mut e, &[LOADING_FLAG]);
        // the pages of the globals and vtables
        for page in [
            0x0101_2000,
            0x0101_5000,
            0x0101_7000,
            0x0102_f000,
            0x011c_a000,
            0x011c_3000,
            0x011d_d000,
            0x011d_e000,
            0x011d_f000,
            0x011f_4000,
            0x0118_7000,
            0x0120_2000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(CONSTANT_ONE, 1.0f64);
        e
    }

    /// The `TESObjectREFR` vtable with the given (byte offset, target) slots,
    /// and a child-cell vtable whose slot 0 is `00575bb0`-like `target`.
    fn refr_vtable(e: &mut Engine, slots: &[(u32, u32)], cell_getter: u32) {
        for (offset, target) in slots {
            e.mem.set_u32(REFR_VTABLE + offset, *target);
        }
        e.mem.set_u32(CHILD_CELL_VTABLE, cell_getter);
    }

    fn new_refr(e: &mut Engine) -> Ptr<TESObjectREFR> {
        let refr: Ptr<TESObjectREFR> = e.new_object();
        e.mem.set_u32(refr.addr(), REFR_VTABLE);
        e.set(refr, TESObjectREFR::vfptrTESChildCell, CHILD_CELL_VTABLE);
        refr
    }

    /// An object with a vtable of its own: `slots` as (byte offset, target).
    fn object_with_vtable(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> u32 {
        let object = e.mem.alloc(size);
        let vtable = e.mem.alloc(0x500);
        for (offset, target) in slots {
            e.mem.set_u32(vtable + offset, *target);
        }
        e.mem.set_u32(object, vtable);
        object
    }

    /// A form object with a type byte, ID and flags at the PC offsets.
    fn form(e: &mut Engine, kind: u8, id: u32, flags: u32) -> u32 {
        let form = e.mem.alloc(0x100);
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 8, flags);
        e.mem.set_u32(form + 0xc, id);
        form
    }

    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn addresses(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        log.iter().map(|(a, _)| *a).collect()
    }

    // 0x100 slot: IsActor is true for a non-null `actor` object made by this.
    fn actor_object(e: &mut Engine) -> u32 {
        e.register(0x00ff_0001, |_, _| true.into_ret());
        object_with_vtable(e, 0x20, &[(0x100, 0x00ff_0001)])
    }

    fn plain_object(e: &mut Engine) -> u32 {
        e.register(0x00ff_0002, |_, _| false.into_ret());
        object_with_vtable(e, 0x20, &[(0x100, 0x00ff_0002)])
    }

    // ---- 00426720 -------------------------------------------------------

    #[test]
    fn handler_00426720_acts_on_flag_20000_for_non_actors_only() {
        let mut e = engine();
        stub(&mut e, &[0x0048_3710]);
        returns(&mut e, 0x0041_8460, 0x1234);
        let this = e.mem.alloc(0x40);
        let plain = plain_object(&mut e);
        let actor = actor_object(&mut e);

        e.call_log = Some(vec![]);
        e.call(0x0042_6720, &args![this, 0x20000u32, 0u32, plain]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0041_8460), vec![vec![this]]);
        assert_eq!(calls_to(&log, 0x0048_3710), vec![vec![0x1234]]);

        // a null target counts as a non-actor
        e.call_log = Some(vec![]);
        e.call(0x0042_6720, &args![this, 0x20000u32, 0u32, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0048_3710).len(), 1);

        // an actor target, or another flag, does nothing
        e.call_log = Some(vec![]);
        e.call(0x0042_6720, &args![this, 0x20000u32, 0u32, actor]);
        e.call(0x0042_6720, &args![this, 0x1u32, 0u32, plain]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0048_3710).is_empty());
    }

    // ---- 004267d0 -------------------------------------------------------

    fn handler_004267d0_engine() -> (Engine, u32, u32, u32) {
        let mut e = engine();
        stub(
            &mut e,
            &[
                0x0041_ae90,
                0x0042_2920,
                0x0041_cc60,
                0x0041_d930,
                0x0041_b060,
                0x0042_2720,
                0x0042_2670,
                EXTRA_REMOVE_TYPE,
                0x0041_9dc0,
                0x0041_aff0,
                0x0042_e040,
                0x0042_16b0,
                0x0041_e0d0,
                0x0041_de40,
                0x0055_f5b0,
            ],
        );
        returns(&mut e, 0x0041_8460, 0);
        returns(&mut e, 0x0041_de00, 0);
        let save_load = e.mem.alloc(8);
        e.set_global(GLOBAL_SAVE_LOAD, save_load);
        let this = e.mem.alloc(0x40);
        let plain = plain_object(&mut e);
        let actor = actor_object(&mut e);
        (e, this, plain, actor)
    }

    #[test]
    fn handler_004267d0_removes_teleport_and_finished_sequence() {
        let (mut e, this, plain, _) = handler_004267d0_engine();
        returns(&mut e, 0x0041_8460, 7);
        e.call_log = Some(vec![]);
        e.call(0x0042_67d0, &args![this, 0x1002_0000u32, plain]);
        let log = e.call_log.take().unwrap();
        // teleport extra present: removed; then the finished sequence
        assert_eq!(calls_to(&log, 0x0041_ae90), vec![vec![this]]);
        assert_eq!(calls_to(&log, 0x0042_2920), vec![vec![this]]);

        // no teleport pointer: not removed
        returns(&mut e, 0x0041_8460, 0);
        e.call_log = Some(vec![]);
        e.call(0x0042_67d0, &args![this, 0x20000u32, plain]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0041_ae90).is_empty());
        assert!(calls_to(&log, 0x0042_2920).is_empty());
    }

    #[test]
    fn handler_004267d0_clears_an_actors_extras_when_the_save_object_asks() {
        let (mut e, this, _, actor) = handler_004267d0_engine();
        returns(&mut e, 0x0055_f5b0, 1);
        e.call_log = Some(vec![]);
        e.call(0x0042_67d0, &args![this, 0u32, actor]);
        let log = e.call_log.take().unwrap();
        let order: Vec<u32> = addresses(&log)
            .into_iter()
            .skip(1)
            .filter(|a| *a != 0x00ff_0001)
            .collect();
        assert_eq!(
            order,
            vec![
                0x0055_f5b0,
                0x0041_cc60,
                0x0041_d930,
                0x0041_b060,
                0x0042_2720,
                0x0042_2670,
                EXTRA_REMOVE_TYPE,
                0x0041_9dc0,
                0x0041_aff0,
                0x0042_e040,
                0x0042_16b0
            ]
        );
        assert_eq!(calls_to(&log, EXTRA_REMOVE_TYPE), vec![vec![this, 0x3a]]);
        assert_eq!(calls_to(&log, 0x0041_9dc0), vec![vec![this, 0]]);
    }

    #[test]
    fn handler_004267d0_returns_a_non_actors_dropped_item() {
        let (mut e, this, plain, _) = handler_004267d0_engine();
        returns(&mut e, 0x0055_f5b0, 1);
        let dropper = e.mem.alloc(0x80);
        returns(&mut e, 0x0041_de00, dropper);
        e.call_log = Some(vec![]);
        e.call(0x0042_67d0, &args![this, 0u32, plain]);
        let log = e.call_log.take().unwrap();
        // RemoveDroppedItem on the dropper's extra list with the target,
        // then AddDroppedItem(0) on this list
        assert_eq!(
            calls_to(&log, 0x0041_e0d0),
            vec![vec![dropper + 0x44, plain]]
        );
        assert_eq!(calls_to(&log, 0x0041_de40), vec![vec![this, 0]]);

        // the save object says no: nothing at all
        returns(&mut e, 0x0055_f5b0, 0);
        e.call_log = Some(vec![]);
        e.call(0x0042_67d0, &args![this, 0u32, plain]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0041_e0d0).is_empty());
    }

    // ---- 004269c0 -------------------------------------------------------

    #[test]
    fn handler_004269c0_moves_an_actor_out_of_its_cell() {
        let mut e = engine();
        stub(&mut e, &[EXTRA_SET_PERSISTENT_CELL, CELL_REMOVE_REFERENCE]);
        let this = e.mem.alloc(0x40);
        let persistent = e.mem.alloc(0x10);
        returns(&mut e, EXTRA_GET_PERSISTENT_CELL, persistent);
        let actor = actor_object(&mut e);
        e.mem.set_u32(actor + 0x40, 0x5555);
        e.call_log = Some(vec![]);
        e.call(0x0042_69c0, &args![this, 0u32, actor]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, EXTRA_SET_PERSISTENT_CELL),
            vec![vec![this, 0]]
        );
        assert_eq!(
            calls_to(&log, CELL_REMOVE_REFERENCE),
            vec![vec![persistent, actor]]
        );
    }

    #[test]
    fn handler_004269c0_ignores_non_actors_and_actors_without_a_cell() {
        let mut e = engine();
        stub(&mut e, &[EXTRA_SET_PERSISTENT_CELL, CELL_REMOVE_REFERENCE]);
        let this = e.mem.alloc(0x40);
        returns(&mut e, EXTRA_GET_PERSISTENT_CELL, 0x1111);
        let plain = plain_object(&mut e);
        let actor = actor_object(&mut e); // parent cell field is 0
        e.call_log = Some(vec![]);
        e.call(0x0042_69c0, &args![this, 0u32, plain]);
        e.call(0x0042_69c0, &args![this, 0u32, actor]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, CELL_REMOVE_REFERENCE).is_empty());
        // no persistent cell either
        returns(&mut e, EXTRA_GET_PERSISTENT_CELL, 0);
        e.mem.set_u32(actor + 0x40, 0x5555);
        e.call_log = Some(vec![]);
        e.call(0x0042_69c0, &args![this, 0u32, actor]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, CELL_REMOVE_REFERENCE).is_empty());
    }

    // ---- 004b0130 -------------------------------------------------------

    #[test]
    fn pointer_map_delete_item_clears_the_value_and_frees_the_node() {
        let mut e = engine();
        stub(&mut e, &[0x0045_cee0]);
        let map = e.mem.alloc(0x20);
        let item = e.mem.alloc(0x10);
        e.mem.set_u8(item + 8, 1);
        e.call_log = Some(vec![]);
        e.call(0x004b_0130, &args![map, item]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u8(item + 8), 0);
        assert_eq!(calls_to(&log, 0x0045_cee0), vec![vec![map + 0xc, item]]);
    }

    // ---- 0055a2f0 / 0055a400 / 0055ac90 ----------------------------------

    fn reset_engine() -> Engine {
        let mut e = engine();
        // 00403d30: memset(dst, value, count)
        e.register(MEMSET, |e, a| {
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            Ret::default()
        });
        stub(&mut e, &[0x0057_c300]);
        e.set_global(LOADED_DATA_DEFAULT_HEIGHT, -2000.0f32);
        e
    }

    #[test]
    fn data_reset_zeroes_all_but_the_base_object_and_restores_the_scale() {
        let mut e = reset_engine();
        let refr = new_refr(&mut e);
        let data = refr.at(TESObjectREFR::data);
        e.set(data, OBJ_REFR::pObjectReference, Ptr::new(0x7777));
        e.set(data, OBJ_REFR::AngleY, 3.0f32);
        e.set(data, OBJ_REFR::LocationZ, 9.0f32);
        e.set(refr, TESObjectREFR::fRefScale, 2.5f32);
        e.call_log = Some(vec![]);
        e.call(0x0055_ac90, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(data, OBJ_REFR::pObjectReference), Ptr::new(0x7777));
        assert_eq!(e.get(data, OBJ_REFR::AngleY), 0.0);
        assert_eq!(e.get(data, OBJ_REFR::LocationZ), 0.0);
        assert_eq!(e.get(refr, TESObjectREFR::fRefScale), 1.0);
        assert_eq!(calls_to(&log, MEMSET), vec![vec![data.addr(), 0, 0x1c]]);
        // no loading object: the leveled creature base is restored
        assert_eq!(calls_to(&log, 0x0057_c300), vec![vec![refr.addr()]]);
    }

    #[test]
    fn data_reset_resets_the_water_state_of_loaded_data() {
        let mut e = reset_engine();
        let refr = new_refr(&mut e);
        let loaded: Ptr<LOADED_REF_DATA> = e.new_object();
        e.set(
            loaded,
            LOADED_REF_DATA::pCurrentWaterObject,
            Ptr::new(0x4444),
        );
        e.set(loaded, LOADED_REF_DATA::fRelevantWaterHeight, 12.0f32);
        e.set(refr, TESObjectREFR::pLoadedData, loaded.cast());
        e.call(0x0055_ac90, &args![refr]);
        assert!(e
            .get(loaded, LOADED_REF_DATA::pCurrentWaterObject)
            .is_null());
        assert_eq!(
            e.get(loaded, LOADED_REF_DATA::fRelevantWaterHeight),
            -2000.0
        );
    }

    #[test]
    fn data_reset_keeps_the_leveled_base_while_loading() {
        let mut e = reset_engine();
        let refr = new_refr(&mut e);
        let loading_object = e.mem.alloc(8);
        e.set_global(GLOBAL_LOADING_OBJECT, loading_object);
        returns(&mut e, LOADING_FLAG, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_ac90, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0057_c300).is_empty());
        // a loading object that is not loading restores it
        returns(&mut e, LOADING_FLAG, 0);
        e.call_log = Some(vec![]);
        e.call(0x0055_ac90, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0057_c300).len(), 1);
    }

    #[test]
    fn point_pair_constructor_builds_both_points() {
        let mut e = engine();
        let data = e.mem.alloc(0x1c);
        e.call_log = Some(vec![]);
        let back = e.call(0x0055_a400, &args![data]).u32();
        let log = e.call_log.take().unwrap();
        assert_eq!(back, data);
        assert_eq!(
            calls_to(&log, LIST_ITEM_SLOT),
            vec![vec![data + 4], vec![data + 0x10]]
        );
    }

    #[test]
    fn constructor_builds_a_blank_reference() {
        let mut e = reset_engine();
        stub(&mut e, &[0x0048_3370, 0x0053_3240, 0x0041_0360]);
        // 004f15a0: TESForm::cFormType = arg
        e.register(0x004f_15a0, |e, a| {
            e.mem.set_u8(a[0] + 4, a[1] as u8);
            Ret::default()
        });
        let refr: Ptr<TESObjectREFR> = e.new_object();
        e.mem.set_u32(refr.addr() + 0x20, 0x99);
        e.mem.set_u32(refr.addr() + 0x40, 0x99);
        e.mem.set_u32(refr.addr() + 0x1c, 0x99);
        e.call_log = Some(vec![]);
        let back = e.call(0x0055_a2f0, &args![refr]).ptr::<TESObjectREFR>();
        let log = e.call_log.take().unwrap();
        assert_eq!(back, refr);
        assert_eq!(e.mem.u32(refr.addr()), REFR_VTABLE);
        assert_eq!(
            e.get(refr, TESObjectREFR::vfptrTESChildCell),
            CHILD_CELL_VTABLE
        );
        assert_eq!(e.mem.u8(refr.addr() + 4), 0x3a);
        assert_eq!(e.get(refr, TESObjectREFR::fRefScale), 1.0);
        assert!(e.get(refr, TESObjectREFR::pParentCell).is_null());
        assert!(e.get(refr, TESObjectREFR::pRandomSound).is_null());
        assert!(e.get(refr, TESObjectREFR::pLoadedData).is_null());
        assert!(e
            .get(refr.at(TESObjectREFR::data), OBJ_REFR::pObjectReference)
            .is_null());
        // form base first, then the child cell at +0x18, the extra list at +0x44
        assert_eq!(addresses(&log)[1..3], [0x0048_3370, 0x0053_3240]);
        assert_eq!(calls_to(&log, 0x0053_3240), vec![vec![refr.addr() + 0x18]]);
        assert_eq!(calls_to(&log, 0x0041_0360), vec![vec![refr.addr() + 0x44]]);
    }

    // ---- 0055a3b0 / 0055a3d0 --------------------------------------------

    #[test]
    fn animation_comes_from_the_extra_list() {
        let mut e = engine();
        returns(&mut e, 0x0041_8220, 0xabc);
        let refr = new_refr(&mut e);
        e.call_log = Some(vec![]);
        let animation = e.call(0x0055_a3b0, &args![refr]).u32();
        let log = e.call_log.take().unwrap();
        assert_eq!(animation, 0xabc);
        assert_eq!(calls_to(&log, 0x0041_8220), vec![vec![refr.addr() + 0x44]]);
    }

    // ---- 0055abe0 .. 0055ac80 ----------------------------------------------

    #[test]
    fn small_helpers() {
        let mut e = engine();
        let refr = new_refr(&mut e);
        // iFormFlags bit 0x400000
        e.mem.set_u32(refr.addr() + 8, 0x40_0000);
        assert!(e.call(0x0055_abe0, &args![refr]).bool());
        e.mem.set_u32(refr.addr() + 8, 0xffbf_ffff);
        assert!(!e.call(0x0055_abe0, &args![refr]).bool());
        // the list at +0x94
        assert_eq!(e.call(0x0055_ac00, &args![0x1000u32]).u32(), 0x1094);
        // the player's +0x638 is cleared
        let player = e.mem.alloc(0x700);
        e.mem.set_u32(player + 0x638, 5);
        e.call(0x0055_ac20, &args![player]);
        assert_eq!(e.mem.u32(player + 0x638), 0);
        // the remembered reference
        e.set_global(GLOBAL_LAST_REFR, 0x4242u32);
        assert_eq!(e.call(0x0055_ac70, &args![]).u32(), 0x4242);
        e.call(0x0055_ac80, &args![]);
        assert_eq!(e.call(0x0055_ac70, &args![]).u32(), 0);
    }

    #[test]
    fn follower_removal_passes_the_address_of_the_item_slot() {
        let mut e = engine();
        let seen = std::rc::Rc::new(std::cell::Cell::new((0u32, 0u32, 0u32)));
        let seen2 = seen.clone();
        e.register_double(LIST_REMOVE, move |e, a| {
            seen2.set((a[0], a[1], e.mem.u32(a[1])));
            Ret::default()
        });
        let player = e.mem.alloc(0x900);
        e.call(0x0055_ac40, &args![player, 0x1234u32]);
        assert_eq!(seen.get(), (player + 0x84c, seen.get().1, 0x1234));
        // a null reference does nothing
        seen.set((0, 0, 0));
        e.call(0x0055_ac40, &args![player, 0u32]);
        assert_eq!(seen.get(), (0, 0, 0));
    }

    // ---- a reference with a working set of virtuals ------------------------

    /// Byte offset in a test reference that the `Get3D` double reads (the
    /// loaded 3D pointer a test wants the reference to report).
    const FAKE_3D: u32 = 0x60;

    /// A reference whose vtable has `Get3D` (reads `FAKE_3D`), `Set3D`,
    /// `IsActor` (false), `SetDelete`, `SetParentCell`, the name getter and
    /// the location getter (`this + 0x30`, like `00436aa0`), and whose
    /// child-cell base reports its `pParentCell`.
    fn standard_refr(e: &mut Engine) -> Ptr<TESObjectREFR> {
        e.register(0x00ff_0010, |e, a| e.mem.u32(a[0] + FAKE_3D).into_ret());
        stub(
            e,
            &[
                0x00ff_0011,
                0x00ff_0013,
                0x00ff_0014,
                0x00ff_0015,
                0x00ff_0019,
            ],
        );
        e.register(0x00ff_0012, |_, _| false.into_ret());
        e.register(0x00ff_0016, |_, a| (a[0] + 0x30).into_ret());
        e.register(0x00ff_0017, |e, a| e.mem.u32(a[0] + 0x28).into_ret());
        returns(e, 0x00ff_0018, 0x0102_f000);
        e.register(0x00ff_0030, |_, _| 0x4e4eu32.into_ret());
        refr_vtable(
            e,
            &[
                (0x1d0, 0x00ff_0010),
                (0x1cc, 0x00ff_0011),
                (0x100, 0x00ff_0012),
                (0xc4, 0x00ff_0013),
                (0x228, 0x00ff_0014),
                (0x134, 0x00ff_0015),
                (0x1f4, 0x00ff_0016),
                (0x130, 0x00ff_0018),
                (0xe0, 0x00ff_0019),
                (0x1c8, 0x00ff_0030),
            ],
            0x00ff_0017,
        );
        new_refr(e)
    }

    // ---- 0055a430: the destructor ---------------------------------------

    const DESTRUCTOR_CALLEES: &[u32] = &[
        0x0057_acc0,
        0x0056_8680,
        0x0045_38e0,
        0x0082_5c00,
        0x0096_e290,
        0x008d_8520,
        0x0082_f1f0,
        0x0086_8270,
        0x0054_9580,
        0x0057_5d70,
        0x0058_6170,
        0x006f_cf30,
        0x0045_43c0,
        0x0053_80d0,
        0x0066_b0d0,
        0x008a_81c0,
        0x008a_8150,
        0x0057_6760,
        0x008d_15b0,
        0x0086_23a0,
        0x0042_26e0,
        0x0056_a9f0,
        0x0041_dda0,
        0x0056_ac90,
        0x0056_aa10,
        0x0041_de00,
        0x0041_e0d0,
        0x0041_df90,
        0x0041_de40,
        0x0056_7790,
        0x0056_7c50,
        0x0041_e310,
        0x0041_e340,
        0x0041_f170,
        0x0041_f4c0,
        0x0041_f650,
        0x005a_a5d0,
        WATER_LIST_A_GETTER,
        WATER_LIST_B_GETTER,
        LIST_REMOVE,
        0x0056_4d80,
        0x0096_3d60,
        0x0097_6680,
        0x0096_f600,
        0x0097_5320,
        0x0097_8660,
        0x0057_25b0,
        0x0097_49b0,
        0x0045_2370,
        0x0047_7410,
        0x0056_9b80,
        0x0041_e600,
        0x0041_e500,
        0x0056_9ba0,
        0x0041_e780,
        0x0041_ef20,
        0x0041_eda0,
        0x0041_e960,
        0x0056_4930,
        GET_REF_PERSISTS,
        0x0057_5dc0,
        CELL_REMOVE_REFERENCE,
        0x0056_4bc0,
        0x006c_0720,
        0x006c_0c80,
        0x006c_1060,
        0x0070_5b30,
        0x0070_38d0,
        0x0070_38b0,
        0x0096_2cd0,
        0x0089_f4e0,
        0x0041_03b0,
        0x0048_3630,
        LIST_IS_EMPTY,
        // ClearData and what it calls
        0x0056_b0b0,
        0x0057_2a50,
        0x0057_b520,
        EXTRA_REMOVE_ALL,
        EXTRA_REMOVE_ALL_DEFAULT,
        EXTRA_ADD,
        EXTRA_SET_PERSISTENT_CELL,
    ];

    fn destructor_engine() -> Engine {
        let mut e = reset_engine();
        stub(&mut e, DESTRUCTOR_CALLEES);
        returns(&mut e, EXTRA_GET_PERSISTENT_CELL, 0);
        returns(&mut e, EXTRA_GET_DATA, 0);
        stub(&mut e, &[EXTRA_REMOVE]);
        e
    }

    #[test]
    fn destructor_resets_vtables_and_ends_with_the_base_destructors() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        e.mem.set_u32(refr.addr(), 0x1111);
        e.mem.set_u32(refr.addr() + 0x18, 0x2222);
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(refr.addr()), REFR_VTABLE);
        assert_eq!(e.mem.u32(refr.addr() + 0x18), CHILD_CELL_VTABLE);
        let order = addresses(&log);
        // the extra list destructor, then TESForm's, are last
        assert_eq!(order[order.len() - 2..], [0x0041_03b0, 0x0048_3630]);
        assert_eq!(calls_to(&log, 0x0041_03b0), vec![vec![refr.addr() + 0x44]]);
        // SetDelete(true), the reference is removed from the garbage collector
        assert_eq!(calls_to(&log, 0x0056_4930), vec![vec![refr.addr(), 1]]);
        assert_eq!(calls_to(&log, 0x0086_8270), vec![vec![refr.addr()]]);
        // ClearData ran: RemoveAll(this + 0x44, 1) and Set3D(0, 1) through the vtable
        assert_eq!(
            calls_to(&log, EXTRA_REMOVE_ALL),
            vec![vec![refr.addr() + 0x44, 1]]
        );
        assert_eq!(calls_to(&log, 0x00ff_0011), vec![vec![refr.addr(), 0, 1]]);
        // the save/load object's flag was cleared and restored
        assert_eq!(calls_to(&log, 0x008a_8150).len(), 2);
        assert_eq!(calls_to(&log, 0x008a_8150)[0][1], 0);
    }

    #[test]
    fn destructor_asks_each_furniture_user_with_a_process_to_forget_it() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        returns(&mut e, 0x0056_8680, 1);
        // two list nodes: item then next
        let second = e.mem.alloc(8);
        let first = e.mem.alloc(8);
        let actor_a = e.mem.alloc(8);
        let actor_b = e.mem.alloc(8);
        e.mem.set_u32(first, actor_a);
        e.mem.set_u32(first + 4, second);
        e.mem.set_u32(second, actor_b);
        returns(&mut e, 0x0096_e290, first);
        // actor A has a process whose slot 0x4cc is a recorder; B has none
        e.register(0x00ff_0020, |_, _| Ret::default());
        let process = object_with_vtable(&mut e, 0x20, &[(0x4cc, 0x00ff_0020)]);
        e.register_double(0x008d_8520, move |_, a| Ret {
            eax: if a[0] == actor_a { process } else { 0 },
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x00ff_0020),
            vec![vec![process, refr.addr()]]
        );
        assert_eq!(
            calls_to(&log, 0x0045_38e0),
            vec![vec![OBJECT_FURNITURE_MANAGER, 0]]
        );
        assert_eq!(
            calls_to(&log, 0x0082_f1f0),
            vec![vec![OBJECT_FURNITURE_MANAGER]]
        );
    }

    #[test]
    fn destructor_takes_the_reference_out_of_the_water_lists() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        returns(&mut e, WATER_LIST_A_GETTER, 0x0aaa);
        returns(&mut e, WATER_LIST_B_GETTER, 0x0bbb);
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen2 = seen.clone();
        e.register_double(LIST_REMOVE, move |e, a| {
            seen2.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        e.call(0x0055_a430, &args![refr]);
        let me = refr.addr();
        assert_eq!(*seen.borrow(), vec![(0x0aaa, me), (0x0bbb, me)]);
    }

    #[test]
    fn destructor_clears_the_players_slots_that_point_at_it() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let me = refr.addr();
        let player = e.mem.alloc(0x900);
        e.set_global(GLOBAL_PLAYER, player);
        e.mem.set_u32(player + 0x638, me);
        returns(&mut e, 0x0089_f4e0, me);
        returns(&mut e, 0x0070_38d0, me);
        e.set_global(GLOBAL_LAST_REFR, me);
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(player + 0x638), 0);
        assert_eq!(e.global::<u32>(GLOBAL_LAST_REFR), 0);
        // the enemy actor is cleared and the player told
        assert_eq!(calls_to(&log, 0x0070_38b0), vec![vec![0]]);
        assert_eq!(calls_to(&log, 0x0096_2cd0), vec![vec![player, me]]);

        // a different reference in the slots leaves them alone
        let other = standard_refr(&mut e);
        e.mem.set_u32(player + 0x638, me);
        e.set_global(GLOBAL_LAST_REFR, me);
        e.call(0x0055_a430, &args![other]);
        assert_eq!(e.mem.u32(player + 0x638), me);
        assert_eq!(e.global::<u32>(GLOBAL_LAST_REFR), me);
    }

    #[test]
    fn destructor_skips_the_unlinking_for_a_flagged_form() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        // iFormFlags & 0x4000
        e.mem.set_u32(refr.addr() + 8, 0x4000);
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0057_6760).is_empty());
        assert!(calls_to(&log, 0x0096_3d60).is_empty());
        // but the cells and the base destructors still run
        assert_eq!(calls_to(&log, 0x0048_3630).len(), 1);
    }

    #[test]
    fn destructor_removes_the_reference_from_its_parent_and_persistent_cells() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let cell = e.mem.alloc(0x10);
        let persistent_cell = e.mem.alloc(0x10);
        e.set(refr, TESObjectREFR::pParentCell, Ptr::new(cell));
        returns(&mut e, GET_REF_PERSISTS, 1);
        let handler = e.mem.alloc(8);
        e.set_global(GLOBAL_DATA_HANDLER, handler);
        returns(&mut e, 0x0057_5dc0, persistent_cell);
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, CELL_REMOVE_REFERENCE),
            vec![vec![cell, refr.addr()], vec![persistent_cell, refr.addr()]]
        );
        assert_eq!(calls_to(&log, 0x0057_5dc0), vec![vec![refr.addr() + 0x18]]);
    }

    #[test]
    fn destructor_hands_loaded_data_back_to_the_physics_world() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let loaded: Ptr<LOADED_REF_DATA> = e.new_object();
        e.set(loaded, LOADED_REF_DATA::spPhantom, Ptr::new(0x6666));
        e.set(refr, TESObjectREFR::pLoadedData, loaded.cast());
        let cell = e.mem.alloc(0x50);
        e.set(refr, TESObjectREFR::pParentCell, Ptr::new(cell));
        returns(&mut e, 0x0045_43c0, 0x0cc0);
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        // the phantom handle goes to the owner the cell reports, then is released
        assert_eq!(calls_to(&log, 0x0053_80d0), vec![vec![0x0cc0, 0x6666]]);
        assert_eq!(
            calls_to(&log, 0x0066_b0d0),
            vec![vec![loaded.addr() + 0x18, 0]]
        );
    }

    #[test]
    fn destructor_without_a_cell_does_not_look_for_a_world_owner() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let loaded: Ptr<LOADED_REF_DATA> = e.new_object();
        e.set(refr, TESObjectREFR::pLoadedData, loaded.cast());
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0053_80d0).is_empty());
        assert_eq!(calls_to(&log, 0x0066_b0d0).len(), 1);
    }

    #[test]
    fn destructor_unlinks_extra_data_cross_references() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let me = refr.addr();
        // the enable-state parent keeps this reference as a child
        let parent = e.mem.alloc(0x80);
        returns(&mut e, 0x0056_a9f0, parent);
        // a dropped item whose owner is this reference
        let dropped = e.mem.alloc(0x80);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, dropped);
        returns(&mut e, 0x0041_df90, node);
        returns(&mut e, 0x0056_7790, me);
        // an item dropper
        let dropper = e.mem.alloc(0x80);
        returns(&mut e, 0x0041_de00, dropper);
        let handler = e.mem.alloc(8);
        e.set_global(GLOBAL_DATA_HANDLER, handler);
        e.call_log = Some(vec![]);
        e.call(0x0055_a430, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0041_dda0), vec![vec![parent + 0x44, me]]);
        assert_eq!(calls_to(&log, 0x0041_e0d0), vec![vec![dropper + 0x44, me]]);
        assert_eq!(calls_to(&log, 0x0041_de40), vec![vec![dropped + 0x44, 0]]);
        assert_eq!(calls_to(&log, 0x0056_7c50), vec![vec![dropped]]);
    }

    #[test]
    fn scalar_deleting_destructor_frees_the_object_only_when_asked() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        e.call_log = Some(vec![]);
        let back = e
            .call(0x0055_a3d0, &args![refr, 0u32])
            .ptr::<TESObjectREFR>();
        let log = e.call_log.take().unwrap();
        assert_eq!(back, refr);
        assert!(calls_to(&log, 0x0040_1030).is_empty());
        assert_eq!(calls_to(&log, 0x0048_3630).len(), 1);

        let heap = e.mem.alloc(0x68);
        e.mem.set_u32(heap, REFR_VTABLE);
        e.call_log = Some(vec![]);
        e.call(0x0055_a3d0, &args![heap, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0040_1030), vec![vec![heap]]);
        // freed by the platform allocator
        assert_eq!(e.mem.block_size(heap), None);
    }

    // ---- 0055ad20: ClearData ---------------------------------------------

    #[test]
    fn clear_data_wipes_extras_and_drops_the_3d() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let me = refr.addr();
        e.call_log = Some(vec![]);
        e.call(0x0055_ad20, &args![refr]);
        let log = e.call_log.take().unwrap();
        // not loading (no loading object): the leveled base is restored,
        // the light killed, the water list left
        assert_eq!(calls_to(&log, 0x0056_b0b0), vec![vec![me]]);
        assert_eq!(calls_to(&log, 0x0057_2a50), vec![vec![me, 0]]);
        assert_eq!(calls_to(&log, 0x0057_b520), vec![vec![me, 0]]);
        assert_eq!(calls_to(&log, EXTRA_REMOVE_ALL), vec![vec![me + 0x44, 1]]);
        assert!(calls_to(&log, EXTRA_REMOVE_ALL_DEFAULT).is_empty());
        assert_eq!(calls_to(&log, 0x00ff_0011), vec![vec![me, 0, 1]]);
        // no 3D: the 0x5b extra is not looked up
        assert!(calls_to(&log, EXTRA_GET_DATA).is_empty());
    }

    #[test]
    fn clear_data_keeps_the_3d_extra_and_the_persistent_cell_across_the_wipe() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let me = refr.addr();
        e.mem.set_u32(me + FAKE_3D, 0x3333);
        returns(&mut e, EXTRA_GET_PERSISTENT_CELL, 0x0c11);
        returns(&mut e, EXTRA_GET_DATA, 0x0e22);
        e.call_log = Some(vec![]);
        e.call(0x0055_ad20, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, EXTRA_GET_DATA), vec![vec![me + 0x44, 0x5b]]);
        assert_eq!(
            calls_to(&log, EXTRA_REMOVE),
            vec![vec![me + 0x44, 0x0e22, 0], vec![me + 0x44, 0x0e22, 1]]
        );
        assert_eq!(calls_to(&log, EXTRA_ADD), vec![vec![me + 0x44, 0x0e22]]);
        assert_eq!(
            calls_to(&log, EXTRA_SET_PERSISTENT_CELL),
            vec![vec![me + 0x44, 0x0c11]]
        );
        // the wipe is between removing and adding back
        let order = addresses(&log);
        let at = |a: u32| order.iter().position(|x| *x == a).unwrap();
        assert!(at(EXTRA_REMOVE) < at(EXTRA_REMOVE_ALL));
        assert!(at(EXTRA_REMOVE_ALL) < at(EXTRA_ADD));
    }

    #[test]
    fn clear_data_while_loading_keeps_defaults_and_the_3d() {
        let mut e = destructor_engine();
        let refr = standard_refr(&mut e);
        let me = refr.addr();
        let loading_object = e.mem.alloc(8);
        e.set_global(GLOBAL_LOADING_OBJECT, loading_object);
        returns(&mut e, LOADING_FLAG, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_ad20, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0056_b0b0).is_empty());
        assert_eq!(
            calls_to(&log, EXTRA_REMOVE_ALL_DEFAULT),
            vec![vec![me + 0x44, 1]]
        );
        assert!(calls_to(&log, EXTRA_REMOVE_ALL).is_empty());
        assert!(calls_to(&log, 0x00ff_0011).is_empty());
    }

    // ---- 0055ae70: Save ----------------------------------------------------

    #[test]
    fn save_of_a_deleted_form_only_opens_and_closes_it() {
        let mut e = engine();
        stub(&mut e, &[0x0048_55a0, 0x0048_5680]);
        let refr = standard_refr(&mut e);
        e.mem.set_u32(refr.addr() + 8, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0055_ae70, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            addresses(&log)[1..],
            [0x0048_55a0, FORM_IS_DELETED, 0x0048_5680]
        );
    }

    fn save_engine() -> (Engine, Ptr<TESObjectREFR>) {
        let mut e = engine();
        stub(
            &mut e,
            &[
                0x0048_55a0,
                0x0048_5680,
                ADD_CHUNK_VALUE,
                0x0041_2970,
                0x0048_56d0,
                0x0069_2710,
                0x0069_3dc0,
                ADD_CHUNK_DATA,
                ENDIAN_SWAP_ON_SAVE,
                0x0057_2d30,
            ],
        );
        let refr = standard_refr(&mut e);
        let base = form(&mut e, 0x28, 0x0001_2345, 0);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(base),
        );
        e.set(refr, TESObjectREFR::fRefScale, 1.0f32);
        (e, refr)
    }

    #[test]
    fn save_writes_the_base_object_and_the_extras() {
        let (mut e, refr) = save_engine();
        e.call_log = Some(vec![]);
        e.call(0x0055_ae70, &args![refr]);
        let log = e.call_log.take().unwrap();
        // NAME with the base object's form ID; no XSCL for a unit scale, no ONAM
        assert_eq!(
            calls_to(&log, ADD_CHUNK_VALUE),
            vec![vec![tag(b"NAME"), 0x0001_2345]]
        );
        assert_eq!(calls_to(&log, 0x0041_2970), vec![vec![refr.addr() + 0x44]]);
        assert!(calls_to(&log, 0x0048_56d0).is_empty());
        let order = addresses(&log);
        assert_eq!(order[1], 0x0048_55a0);
        assert_eq!(*order.last().unwrap(), 0x0048_5680);
    }

    #[test]
    fn save_writes_scale_and_ownership_chunks_when_set() {
        let (mut e, refr) = save_engine();
        e.set(refr, TESObjectREFR::fRefScale, 1.5f32);
        returns(&mut e, 0x0057_2d30, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_ae70, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, ADD_CHUNK_VALUE),
            vec![
                vec![tag(b"NAME"), 0x0001_2345],
                vec![tag(b"XSCL"), 1.5f32.to_bits()]
            ]
        );
        assert_eq!(calls_to(&log, 0x0057_2d30), vec![vec![refr.addr(), 8]]);
        assert_eq!(calls_to(&log, 0x0048_56d0), vec![vec![tag(b"ONAM")]]);
    }

    #[test]
    fn save_writes_position_then_rotation_in_the_data_chunk() {
        let (mut e, refr) = save_engine();
        let me = refr.addr();
        for i in 0..3u32 {
            e.mem.set_f32(me + 0x30 + 4 * i, 100.0 + i as f32);
            e.mem.set_f32(me + 0x24 + 4 * i, 1.0 + i as f32);
        }
        let written = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let written2 = written.clone();
        e.register_double(ADD_CHUNK_DATA, move |e, a| {
            let bytes = e.mem.bytes(a[1], a[2]);
            written2.borrow_mut().push((a[0], bytes));
            Ret::default()
        });
        e.call(0x0055_ae70, &args![refr]);
        let data = written.borrow();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].0, tag(b"DATA"));
        let floats: Vec<f32> = data[0]
            .1
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(floats, vec![100.0, 101.0, 102.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn save_swaps_the_data_chunk_around_the_write_when_the_build_does() {
        let (mut e, refr) = save_engine();
        returns(&mut e, ENDIAN_SWAP_ON_SAVE, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_ae70, &args![refr]);
        let log = e.call_log.take().unwrap();
        let order = addresses(&log);
        let write = order.iter().position(|a| *a == ADD_CHUNK_DATA).unwrap();
        let swaps: Vec<usize> = order
            .iter()
            .enumerate()
            .filter(|(_, a)| **a == 0x0069_3dc0)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(swaps.len(), 2);
        assert!(swaps[0] < write && write < swaps[1]);
    }

    // ---- 0055af90: Load ----------------------------------------------------

    type Shared<T> = std::rc::Rc<std::cell::RefCell<T>>;

    /// An engine whose `TESFile` doubles serve `chunks`: each entry is a
    /// chunk tag and the payload bytes `GetChunkData` copies out for it.
    /// `forms` maps a form ID to the object `LookupFormByID` returns.
    fn load_engine(
        chunks: Vec<(u32, Vec<u8>)>,
        forms: Vec<(u32, u32)>,
    ) -> (Engine, Ptr<TESObjectREFR>, u32) {
        let mut e = engine();
        stub(
            &mut e,
            &[
                0x0069_2710,
                0x0048_4ab0,
                0x0048_5110,
                0x0048_5d50,
                0x0069_3dc0,
                0x0041_44a0,
                MESSAGE,
                0x0057_2d50,
                0x0040_1680,
                0x0041_fe90,
                EXTRA_REMOVE_ALL,
                0x0056_b0b0,
                EXTRA_SET_PERSISTENT_CELL,
            ],
        );
        returns(&mut e, EXTRA_GET_PERSISTENT_CELL, 0);
        // identity cast, except a null stays null
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        let queue: Shared<std::collections::VecDeque<(u32, Vec<u8>)>> =
            std::rc::Rc::new(std::cell::RefCell::new(chunks.into_iter().collect()));
        let current: Shared<Vec<u8>> = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let q = queue.clone();
        let c = current.clone();
        e.register_double(0x0047_26b0, move |_, _| match q.borrow_mut().pop_front() {
            Some((tag, payload)) => {
                *c.borrow_mut() = payload;
                Ret {
                    eax: tag,
                    ..Ret::default()
                }
            }
            None => Ret::default(),
        });
        let q = queue.clone();
        e.register_double(0x0047_26f0, move |_, _| (!q.borrow().is_empty()).into_ret());
        let c = current.clone();
        e.register_double(0x0047_27f0, move |e, a| {
            e.mem.write(a[1], &c.borrow());
            Ret::default()
        });
        let c = current.clone();
        e.register_double(0x0047_2890, move |e, a| {
            let payload = c.borrow();
            let n = payload.len().min(a[2] as usize);
            e.mem.write(a[1], &payload[..n]);
            Ret::default()
        });
        e.register_double(0x0048_39c0, move |_, a| {
            forms
                .iter()
                .find(|(id, _)| *id == a[0])
                .map_or(0, |(_, form)| *form)
                .into_ret()
        });
        let chunk_size = e.mem.alloc(8);
        let _ = chunk_size;
        returns(&mut e, 0x0040_1660, 0x40);
        let refr = standard_refr(&mut e);
        e.mem.set_u32(refr.addr() + 0xc, 0x00aa_0001);
        let file = e.mem.alloc(0x20);
        (e, refr, file)
    }

    fn le(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn load_reads_base_object_scale_and_placement() {
        let base = 0x4000_0000u32;
        let mut payload = Vec::new();
        for f in [10.0f32, 20.0, 30.0, 0.5, 1.5, 2.5] {
            payload.extend_from_slice(&f.to_le_bytes());
        }
        let (mut e, refr, file) = load_engine(
            vec![
                (tag(b"NAME"), le(&[0x0001_0001])),
                (tag(b"XSCL"), 2.0f32.to_le_bytes().to_vec()),
                (tag(b"DATA"), payload),
            ],
            vec![(0x0001_0001, base)],
        );
        e.call_log = Some(vec![]);
        let loaded = e.call(0x0055_af90, &args![refr, file]).bool();
        let log = e.call_log.take().unwrap();
        assert!(loaded);
        let data = refr.at(TESObjectREFR::data);
        assert_eq!(e.get(data, OBJ_REFR::pObjectReference), Ptr::new(base));
        assert_eq!(e.get(refr, TESObjectREFR::fRefScale), 2.0);
        // the DATA chunk is position first, then rotation
        assert_eq!(e.get(data, OBJ_REFR::LocationX), 10.0);
        assert_eq!(e.get(data, OBJ_REFR::LocationZ), 30.0);
        assert_eq!(e.get(data, OBJ_REFR::AngleX), 0.5);
        assert_eq!(e.get(data, OBJ_REFR::AngleZ), 2.5);
        // LoadForm(this, file), then the chunk reader's compile index fix-up
        assert_eq!(calls_to(&log, 0x0048_5110), vec![vec![refr.addr(), file]]);
        assert_eq!(calls_to(&log, 0x0048_5d50).len(), 1);
        // SetDelete(false) before the placement is read
        assert_eq!(calls_to(&log, 0x00ff_0013), vec![vec![refr.addr(), 0]]);
        assert_eq!(
            calls_to(&log, 0x0047_2890),
            vec![vec![file, calls_to(&log, 0x0047_2890)[0][1], 0x18]]
        );
    }

    #[test]
    fn load_with_a_missing_base_object_logs_it_and_skips_the_placement() {
        let mut payload = Vec::new();
        for f in [10.0f32, 20.0, 30.0, 0.5, 1.5, 2.5] {
            payload.extend_from_slice(&f.to_le_bytes());
        }
        let (mut e, refr, file) = load_engine(
            vec![(tag(b"NAME"), le(&[0x00de_ad00])), (tag(b"DATA"), payload)],
            vec![],
        );
        e.call_log = Some(vec![]);
        let loaded = e.call(0x0055_af90, &args![refr, file]).bool();
        let log = e.call_log.take().unwrap();
        assert!(!loaded);
        assert_eq!(
            calls_to(&log, MESSAGE),
            vec![vec![MISSING_BASE_FORMAT, 0x00de_ad00, 0x00aa_0001]]
        );
        let data = refr.at(TESObjectREFR::data);
        assert_eq!(e.get(data, OBJ_REFR::LocationX), 0.0);
        assert!(calls_to(&log, 0x0047_2890).is_empty());
    }

    #[test]
    fn load_keeps_a_known_base_while_loading_and_replaces_an_unknown_one() {
        for known in [true, false] {
            let (mut e, refr, file) = load_engine(
                vec![(tag(b"NAME"), le(&[0x0001_0002]))],
                vec![(0x0001_0002, 0x4000_0100)],
            );
            let data = refr.at(TESObjectREFR::data);
            let old_base = form(&mut e, 0x28, 0x0001_0001, 0);
            e.set(data, OBJ_REFR::pObjectReference, Ptr::new(old_base));
            let loading_object = e.mem.alloc(8);
            e.set_global(GLOBAL_LOADING_OBJECT, loading_object);
            let handler = e.mem.alloc(8);
            e.set_global(GLOBAL_DATA_HANDLER, handler);
            returns(&mut e, LOADING_FLAG, 1);
            // 00469860(handler, id): is the old base already known?
            returns(&mut e, 0x0046_9860, known as u32);
            e.call_log = Some(vec![]);
            e.call(0x0055_af90, &args![refr, file]);
            let log = e.call_log.take().unwrap();
            let expected = if known { old_base } else { 0x4000_0100 };
            assert_eq!(e.get(data, OBJ_REFR::pObjectReference), Ptr::new(expected));
            assert_eq!(
                calls_to(&log, 0x0046_9860),
                vec![vec![handler, 0x0001_0001]]
            );
        }
    }

    #[test]
    fn load_hands_every_extra_data_chunk_kind_to_the_extra_list() {
        for kind in EXTRA_DATA_CHUNKS {
            let (mut e, refr, file) = load_engine(vec![(kind, vec![])], vec![]);
            e.call_log = Some(vec![]);
            e.call(0x0055_af90, &args![refr, file]);
            let log = e.call_log.take().unwrap();
            assert_eq!(
                calls_to(&log, 0x0041_44a0),
                vec![vec![refr.addr() + 0x44, file, refr.addr()]],
                "chunk {kind:08x}"
            );
            assert!(calls_to(&log, MESSAGE).is_empty());
        }
    }

    #[test]
    fn load_reports_an_unknown_chunk_and_skips_rclr() {
        let (mut e, refr, file) =
            load_engine(vec![(tag(b"ZZZZ"), vec![]), (tag(b"RCLR"), vec![])], vec![]);
        e.call_log = Some(vec![]);
        e.call(0x0055_af90, &args![refr, file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, MESSAGE), vec![vec![LOAD_TERMINATED_MESSAGE]]);
        assert!(calls_to(&log, 0x0041_44a0).is_empty());
    }

    #[test]
    fn load_passes_the_editor_id_and_bounds_to_the_virtuals() {
        let (mut e, refr, file) = load_engine(
            vec![
                (tag(b"EDID"), b"MyRefr\0".to_vec()),
                (tag(b"OBND"), vec![]),
                (tag(b"ONAM"), vec![]),
            ],
            vec![],
        );
        e.call_log = Some(vec![]);
        e.call(0x0055_af90, &args![refr, file]);
        let log = e.call_log.take().unwrap();
        // the editor ID is read into a stack buffer of the chunk's size (0x40 here)
        let read = calls_to(&log, 0x0047_2890);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0][0], file);
        assert_eq!(read[0][2], 0x200);
        // virtual +0x134 got that buffer; +0xe0 got the file; ONAM sets flag 8
        assert_eq!(
            calls_to(&log, 0x00ff_0015),
            vec![vec![refr.addr(), read[0][1]]]
        );
        assert_eq!(calls_to(&log, 0x00ff_0019), vec![vec![refr.addr(), file]]);
        assert_eq!(calls_to(&log, 0x0057_2d50), vec![vec![refr.addr(), 8]]);
    }

    #[test]
    fn load_of_a_deleted_form_drops_its_extras_but_keeps_the_persistent_cell() {
        let (mut e, refr, file) = load_engine(vec![(tag(b"NAME"), le(&[1]))], vec![(1, 0x4000)]);
        e.mem.set_u32(refr.addr() + 8, 0x20);
        returns(&mut e, EXTRA_GET_PERSISTENT_CELL, 0x0c0c);
        let data = refr.at(TESObjectREFR::data);
        e.set(data, OBJ_REFR::pObjectReference, Ptr::new(0x4000_0010));
        e.call_log = Some(vec![]);
        let loaded = e.call(0x0055_af90, &args![refr, file]).bool();
        let log = e.call_log.take().unwrap();
        assert!(loaded);
        assert_eq!(calls_to(&log, 0x0056_b0b0), vec![vec![refr.addr()]]);
        assert_eq!(
            calls_to(&log, EXTRA_REMOVE_ALL),
            vec![vec![refr.addr() + 0x44, 1]]
        );
        assert_eq!(
            calls_to(&log, EXTRA_SET_PERSISTENT_CELL),
            vec![vec![refr.addr() + 0x44, 0x0c0c]]
        );
        // the chunks are not read at all
        assert!(calls_to(&log, 0x0047_26b0).is_empty());
    }

    #[test]
    fn load_reports_a_patrol_script_that_has_text_but_is_not_compiled() {
        let (mut e, refr, file) = load_engine(vec![], vec![]);
        returns(&mut e, 0x0041_fe90, 0x0111);
        let script = e.mem.alloc(0x40);
        e.mem.set_u32(script + 0x2c, 0x0222);
        e.mem.set_u8(script + 0x2a, 0);
        let info = e.mem.alloc(0x10);
        returns(&mut e, 0x008d_0430, script);
        returns(&mut e, 0x0050_0940, info);
        stub(&mut e, &[STRING_CONSTRUCT, STRING_FORMAT, STRING_DESTRUCT]);
        e.register(READ_FIRST_DWORD, |_, _| 0x0777u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0055_af90, &args![refr, file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, MESSAGE), vec![vec![0x0777]]);
        let format = calls_to(&log, STRING_FORMAT);
        assert_eq!(format.len(), 1);
        assert_eq!(format[0][1], PATROL_NOT_COMPILED_FORMAT);
        // a compiled script, or one without text, stays quiet
        e.mem.set_u8(script + 0x2a, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_af90, &args![refr, file]);
        assert!(calls_to(&e.call_log.take().unwrap(), MESSAGE).is_empty());
        e.mem.set_u8(script + 0x2a, 0);
        e.mem.set_u32(script + 0x2c, 0);
        e.call_log = Some(vec![]);
        e.call(0x0055_af90, &args![refr, file]);
        assert!(calls_to(&e.call_log.take().unwrap(), MESSAGE).is_empty());
    }

    #[test]
    fn script_holder_accessors_read_their_fields() {
        let mut e = engine();
        let object = e.mem.alloc(0x40);
        e.mem.set_u32(object + 0x2c, 0x1234);
        e.mem.set_u8(object + 0x2a, 1);
        assert_eq!(e.call(0x0055_b980, &args![object]).u32(), 0x1234);
        assert!(e.call(0x0055_b9a0, &args![object]).bool());
        e.mem.set_u8(object + 0x2a, 0);
        assert!(!e.call(0x0055_b9a0, &args![object]).bool());
    }

    // ---- 0055b9c0: the references map pass ---------------------------------

    /// An engine whose `BGSSaveLoadReferencesMap::Lookup` (`00853130`)
    /// maps the old references in `pairs` to new ones.
    fn remap_engine(pairs: Vec<(u32, u32)>) -> (Engine, Ptr<TESObjectREFR>, u32, u32) {
        let mut e = engine();
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        e.register_double(0x0085_3130, move |e, a| {
            // (map, old, &new)
            match pairs.iter().find(|(old, _)| *old == a[1]) {
                Some((_, new)) => {
                    e.mem.set_u32(a[2], *new);
                    true.into_ret()
                }
                None => false.into_ret(),
            }
        });
        stub(
            &mut e,
            &[
                0x0056_a9f0,
                0x0056_aa10,
                0x0056_aa70,
                0x0056_aa90,
                0x0041_dcd0,
                0x0056_9b20,
                0x0056_9b40,
                0x0056_9b80,
                0x0056_9ba0,
                0x0041_e530,
                0x0042_0bc0,
                0x0042_0a60,
                0x0042_0130,
                0x0042_0160,
                0x0041_e780,
                0x0041_e7f0,
                0x0041_ea30,
                0x0041_eba0,
                0x0041_eb60,
                0x0041_edd0,
                0x0041_f050,
                0x0056_a470,
                0x0056_a990,
                0x0056_a9b0,
                0x0056_8e50,
                LIST_IS_EMPTY,
            ],
        );
        let refr = standard_refr(&mut e);
        let map = e.mem.alloc(0x20);
        let target = e.mem.alloc(0x80);
        (e, refr, map, target)
    }

    #[test]
    fn references_pass_needs_a_map_and_a_mapping_for_the_reference() {
        let (mut e, refr, map, _) = remap_engine(vec![]);
        e.call_log = Some(vec![]);
        e.call(0x0055_b9c0, &args![refr, 0u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        // a map that does not know the reference: one lookup, nothing else
        e.call_log = Some(vec![]);
        e.call(0x0055_b9c0, &args![refr, map]);
        let log = e.call_log.take().unwrap();
        assert_eq!(addresses(&log), vec![0x0055_b9c0, 0x0085_3130]);
        assert_eq!(calls_to(&log, 0x0085_3130)[0][..2], [map, refr.addr()]);
    }

    #[test]
    fn references_pass_moves_the_enable_parent_to_the_duplicates_parent() {
        let (mut e, refr, map, target) = remap_engine(vec![]);
        let me = refr.addr();
        let old_parent = e.mem.alloc(0x80);
        let new_parent = e.mem.alloc(0x80);
        // rebuild the engine's lookup with the real addresses
        e.register_double(0x0085_3130, move |e, a| {
            let new = if a[1] == me {
                target
            } else if a[1] == old_parent {
                new_parent
            } else {
                return false.into_ret();
            };
            e.mem.set_u32(a[2], new);
            true.into_ret()
        });
        returns(&mut e, 0x0056_a9f0, old_parent);
        returns(&mut e, 0x0056_aa70, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_b9c0, &args![refr, map]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0056_aa10), vec![vec![target, new_parent]]);
        assert_eq!(calls_to(&log, 0x0056_aa90), vec![vec![target, 1]]);
        assert_eq!(
            calls_to(&log, 0x0041_dcd0),
            vec![vec![new_parent + 0x44, target]]
        );
    }

    #[test]
    fn references_pass_keeps_unmapped_slot_references_and_replaces_mapped_ones() {
        let (mut e, refr, map, target) = remap_engine(vec![]);
        let me = refr.addr();
        let (unmapped, mapped_old, mapped_new) = (0x0a00u32, 0x0b00u32, e.mem.alloc(0x80));
        e.register_double(0x0085_3130, move |e, a| {
            let new = if a[1] == me {
                target
            } else if a[1] == mapped_old {
                mapped_new
            } else {
                return false.into_ret();
            };
            e.mem.set_u32(a[2], new);
            true.into_ret()
        });
        // slot 0 holds an unmapped reference, slot 1 a mapped one
        e.register(0x0042_0bc0, |_, a| {
            let slot = a[1];
            [0x0a00u32, 0x0b00][slot as usize].into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0055_b9c0, &args![refr, map]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x0042_0a60),
            vec![
                vec![target + 0x44, 0, unmapped],
                vec![target + 0x44, 1, mapped_new]
            ]
        );
    }

    #[test]
    fn references_pass_remaps_the_four_slot_references_when_they_are_mapped() {
        let (mut e, refr, map, target) = remap_engine(vec![]);
        let me = refr.addr();
        let new_ref = e.mem.alloc(0x80);
        e.register_double(0x0085_3130, move |e, a| {
            let new = match a[1] {
                x if x == me => target,
                0x0c00 => new_ref,
                _ => return false.into_ret(),
            };
            e.mem.set_u32(a[2], new);
            true.into_ret()
        });
        e.register(0x0042_0130, |_, a| {
            // only slot 2 has a reference
            if a[1] == 2 { 0x0c00u32 } else { 0 }.into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0055_b9c0, &args![refr, map]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x0042_0160),
            vec![vec![target + 0x44, new_ref, 2]]
        );
    }

    #[test]
    fn references_pass_rewires_activate_children_and_decals() {
        let (mut e, refr, map, target) = remap_engine(vec![]);
        let me = refr.addr();
        let child_new = e.mem.alloc(0x80);
        let decal_new = e.mem.alloc(0x80);
        e.register_double(0x0085_3130, move |e, a| {
            let new = match a[1] {
                x if x == me => target,
                0x0d00 => child_new,
                0x0e00 => decal_new,
                _ => return false.into_ret(),
            };
            e.mem.set_u32(a[2], new);
            true.into_ret()
        });
        // activate children: one node whose item points at {old child, 0.75}
        let child_entry = e.mem.alloc(8);
        e.mem.set_u32(child_entry, 0x0d00);
        e.mem.set_f32(child_entry + 4, 0.75);
        let child_node = e.mem.alloc(8);
        e.mem.set_u32(child_node, child_entry);
        returns(&mut e, 0x0041_e780, child_node);
        returns(&mut e, 0x0041_eb60, 3);
        // decals: {old ref, position at +4, rotation at +0x10}
        let decal_entry = e.mem.alloc(0x20);
        e.mem.set_u32(decal_entry, 0x0e00);
        let decal_node = e.mem.alloc(8);
        e.mem.set_u32(decal_node, decal_entry);
        returns(&mut e, 0x0041_f050, decal_node);
        e.call_log = Some(vec![]);
        e.call(0x0055_b9c0, &args![refr, map]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x0041_e7f0),
            vec![vec![target + 0x44, child_new]]
        );
        assert_eq!(
            calls_to(&log, 0x0041_ea30),
            vec![vec![target + 0x44, child_new, 0.75f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, 0x0041_edd0),
            vec![vec![child_new + 0x44, target]]
        );
        assert_eq!(calls_to(&log, 0x0041_eba0), vec![vec![target + 0x44, 3]]);
        assert_eq!(
            calls_to(&log, 0x0056_a470),
            vec![vec![target, decal_new, decal_entry + 4, decal_entry + 0x10]]
        );
    }

    #[test]
    fn references_pass_places_the_duplicated_door_link() {
        let (mut e, refr, map, target) = remap_engine(vec![]);
        let me = refr.addr();
        // the linked door (old) with a location, its replacement with another
        let old_door = standard_refr(&mut e);
        let new_door = standard_refr(&mut e);
        for (i, v) in [100.0f32, 200.0, 300.0].iter().enumerate() {
            e.mem.set_f32(old_door.addr() + 0x30 + 4 * i as u32, *v);
        }
        for (i, v) in [1000.0f32, 2000.0, 3000.0].iter().enumerate() {
            e.mem.set_f32(new_door.addr() + 0x30 + 4 * i as u32, *v);
        }
        let (old_door_addr, new_door_addr) = (old_door.addr(), new_door.addr());
        e.register_double(0x0085_3130, move |e, a| {
            let new = if a[1] == me {
                target
            } else if a[1] == old_door_addr {
                new_door_addr
            } else {
                return false.into_ret();
            };
            e.mem.set_u32(a[2], new);
            true.into_ret()
        });
        // this reference's teleport data: {door, point (+4), ..., +0x10 point}
        let teleport = e.mem.alloc(0x20);
        e.mem.set_u32(teleport, old_door_addr);
        for (i, v) in [110.0f32, 220.0, 330.0].iter().enumerate() {
            e.mem.set_f32(teleport + 4 + 4 * i as u32, *v);
        }
        for (i, v) in [7.0f32, 8.0, 9.0].iter().enumerate() {
            e.mem.set_f32(teleport + 0x10 + 4 * i as u32, *v);
        }
        returns(&mut e, 0x0056_8e50, teleport);
        let new_teleport = e.mem.alloc(0x20);
        returns(&mut e, 0x0056_8e70, new_teleport);
        stub(&mut e, &[0x0053_7e90, 0x006f_4ed0, 0x0043_a280]);
        e.register(0x0046_0140, |_, a| (a[0] + 0x10).into_ret());
        e.register(0x0071_7e50, |_, a| (a[0] + 4).into_ret());
        // NiPoint3 minus and plus (result is the first stack argument)
        let point =
            |e: &Engine, p: u32| -> [f32; 3] { [e.mem.f32(p), e.mem.f32(p + 4), e.mem.f32(p + 8)] };
        e.register_double(0x0043_9ef0, move |e, a| {
            let (this, result, other) = (point(e, a[0]), a[1], point(e, a[2]));
            for i in 0..3 {
                e.mem.set_f32(result + 4 * i as u32, this[i] - other[i]);
            }
            result.into_ret()
        });
        e.register_double(0x0043_9e90, move |e, a| {
            let (this, result, other) = (point(e, a[0]), a[1], point(e, a[2]));
            for i in 0..3 {
                e.mem.set_f32(result + 4 * i as u32, this[i] + other[i]);
            }
            result.into_ret()
        });
        let moved = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let copied = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let (moved2, copied2) = (moved.clone(), copied.clone());
        e.register_double(0x006f_4ed0, move |e, a| {
            moved2.borrow_mut().push((a[0], point(e, a[1])));
            Ret::default()
        });
        e.register_double(0x0043_a280, move |e, a| {
            copied2.borrow_mut().push((a[0], point(e, a[1])));
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0055_b9c0, &args![refr, map]);
        let log = e.call_log.take().unwrap();
        // the new teleport is created on the duplicate and linked to the new door
        assert_eq!(calls_to(&log, 0x0056_8e70), vec![vec![target]]);
        assert_eq!(
            calls_to(&log, 0x0053_7e90),
            vec![vec![new_teleport, new_door_addr]]
        );
        // new door location + (teleport point - old door location)
        assert_eq!(
            *moved.borrow(),
            vec![(new_teleport, [1010.0, 2020.0, 3030.0])]
        );
        assert_eq!(*copied.borrow(), vec![(new_teleport, [7.0, 8.0, 9.0])]);
    }

    // ---- 0055c050: CreateDuplicateForm --------------------------------------

    #[test]
    fn create_duplicate_sets_extras_aside_and_puts_them_back() {
        let mut e = destructor_engine();
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        let refr = standard_refr(&mut e);
        let me = refr.addr();
        let parent = e.mem.alloc(0x10);
        e.set(refr, TESObjectREFR::pParentCell, Ptr::new(parent));
        // extras of types 0x2b, 0x51 and 0x63 exist
        e.register(EXTRA_GET_DATA, |_, a| {
            match a[1] {
                0x2b => 0x2b00u32,
                0x51 => 0x5100,
                0x63 => 0x6300,
                _ => 0,
            }
            .into_ret()
        });
        stub(&mut e, &[EXTRA_REMOVE, EXTRA_ADD, 0x0056_5480]);
        let duplicate = standard_refr(&mut e);
        returns(&mut e, 0x0048_67a0, duplicate.addr());
        let map = e.mem.alloc(8);
        e.call_log = Some(vec![]);
        let back = e
            .call(0x0055_c050, &args![refr, 0u32, map])
            .ptr::<TESObjectREFR>();
        let log = e.call_log.take().unwrap();
        assert_eq!(back, duplicate);
        let list = me + 0x44;
        // the 0x51 extra is fetched (and removed) twice, for two classes
        assert_eq!(
            calls_to(&log, EXTRA_REMOVE),
            vec![
                vec![list, 0x2b00, 0],
                vec![list, 0x5100, 0],
                vec![list, 0x5100, 0],
                vec![list, 0x6300, 0]
            ]
        );
        assert_eq!(
            calls_to(&log, EXTRA_ADD),
            vec![
                vec![list, 0x2b00],
                vec![list, 0x5100],
                vec![list, 0x5100],
                vec![list, 0x6300]
            ]
        );
        // parent cell cleared for the duplication, then restored
        assert_eq!(
            calls_to(&log, 0x00ff_0014),
            vec![vec![me, 0], vec![me, parent]]
        );
        assert_eq!(calls_to(&log, 0x0048_67a0), vec![vec![me, 0, map]]);
        // the copy loses its 3D and its persistence
        assert_eq!(
            calls_to(&log, 0x00ff_0011),
            vec![vec![duplicate.addr(), 0, 1]]
        );
        assert_eq!(calls_to(&log, 0x0056_5480), vec![vec![duplicate.addr(), 0]]);
    }

    #[test]
    fn create_duplicate_without_extras_leaves_the_lists_alone() {
        let mut e = destructor_engine();
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        let refr = standard_refr(&mut e);
        stub(&mut e, &[EXTRA_REMOVE, EXTRA_ADD, 0x0056_5480]);
        let duplicate = standard_refr(&mut e);
        returns(&mut e, 0x0048_67a0, duplicate.addr());
        e.call_log = Some(vec![]);
        e.call(0x0055_c050, &args![refr, 0u32, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, EXTRA_REMOVE).is_empty());
        assert!(calls_to(&log, EXTRA_ADD).is_empty());
        // eight lookups, one per kind
        assert_eq!(calls_to(&log, EXTRA_GET_DATA).len(), 8);
    }

    // ---- 0055c3e0: Copy ----------------------------------------------------

    const COPY_CALLEES: &[u32] = &[
        0x0040_30b0,
        0x0041_1ec0,
        0x0041_8250,
        0x0041_ae70,
        0x0041_dcd0,
        0x0041_dda0,
        0x0041_e530,
        0x0041_e600,
        0x0041_e780,
        0x0041_edd0,
        0x0041_ef20,
        0x0041_f170,
        0x0041_f4c0,
        0x0041_f650,
        0x0041_f810,
        0x0041_f840,
        0x0043_d410,
        0x0043_fa80,
        0x0044_0460,
        0x0044_0490,
        0x0045_0b80,
        0x0045_11e0,
        0x0045_1ef0,
        0x0045_6520,
        0x0047_aec0,
        0x0048_51b0,
        0x0049_c680,
        0x004e_8030,
        0x004e_d8c0,
        0x0050_d810,
        0x0053_7e90,
        0x0054_8230,
        0x0056_5480,
        0x0056_7490,
        0x0056_7770,
        0x0056_7c50,
        0x0056_8e50,
        0x0056_8e70,
        0x0056_8f30,
        0x0056_9140,
        0x0056_9580,
        0x0056_9b80,
        0x0056_9be0,
        0x0056_9cb0,
        0x0056_9df0,
        0x0056_a1e0,
        0x0056_a9f0,
        0x0056_fa00,
        0x0057_2d30,
        0x0057_ada0,
        0x0057_c730,
        0x0070_ec90,
        0x008d_8520,
        0x0096_11e0,
        0x00a5_9c60,
        0x00a5_9d30,
        0x00a5_a040,
        0x00b5_eed0,
        0x00c6_bd00,
        LIST_IS_EMPTY,
        CELL_REMOVE_REFERENCE,
        GET_REF_PERSISTS,
        MESSAGE,
    ];

    /// `this` and the source reference of a `Copy`, with every callee stubbed
    /// and the few accessors the logic depends on given real bodies: the
    /// scale getter (`float` at +0x3c), `memcpy`, the stack-count getter and
    /// a radius getter (equal for both).
    fn copy_engine() -> (Engine, Ptr<TESObjectREFR>, Ptr<TESObjectREFR>) {
        let mut e = engine();
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        stub(&mut e, COPY_CALLEES);
        // 00598040 / 00567400: the scale; 00568cb0: a radius
        e.register(0x0059_8040, |e, a| e.mem.f32(a[0] + 0x3c).into_ret());
        e.register(0x0056_7400, |e, a| e.mem.f32(a[0] + 0x3c).into_ret());
        e.register(0x0056_8cb0, |_, _| 5.0f32.into_ret());
        returns(&mut e, 0x0040_eb10, 1);
        e.register(0x0040_1460, |e, a| {
            let bytes = e.mem.bytes(a[1], a[2]);
            e.mem.write(a[0], &bytes);
            Ret::default()
        });
        returns(&mut e, 0x0041_8770, 0);
        let this = standard_refr(&mut e);
        let other = standard_refr(&mut e);
        e.set(this, TESObjectREFR::fRefScale, 1.0f32);
        e.set(other, TESObjectREFR::fRefScale, 1.0f32);
        (e, this, other)
    }

    fn set3d_calls(log: &[(u32, Vec<u32>)], refr: Ptr<TESObjectREFR>) -> Vec<Vec<u32>> {
        calls_to(log, 0x00ff_0011)
            .into_iter()
            .filter(|c| c[0] == refr.addr())
            .collect()
    }

    #[test]
    fn copy_ignores_a_source_that_is_not_a_reference() {
        let (mut e, this, _) = copy_engine();
        e.register(DYNAMIC_CAST, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, 0x1234u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }

    #[test]
    fn copy_takes_over_the_data_persistence_cell_and_scale() {
        let (mut e, this, other) = copy_engine();
        let (me, them) = (this.addr(), other.addr());
        let cell = e.mem.alloc(0x40);
        e.set(other, TESObjectREFR::pParentCell, Ptr::new(cell));
        e.set(other, TESObjectREFR::fRefScale, 1.0f32);
        let their_data = other.at(TESObjectREFR::data);
        e.set(their_data, OBJ_REFR::LocationX, 42.0f32);
        e.set(their_data, OBJ_REFR::AngleY, 0.25f32);
        returns(&mut e, GET_REF_PERSISTS, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        // persistence, then the form components
        assert_eq!(calls_to(&log, 0x0056_5480), vec![vec![me, 1]]);
        assert_eq!(calls_to(&log, 0x0048_51b0), vec![vec![me, them]]);
        // 0x1c bytes of OBJ_REFR data
        assert_eq!(
            calls_to(&log, 0x0040_1460),
            vec![vec![me + 0x20, them + 0x20, 0x1c]]
        );
        assert_eq!(
            e.get(this.at(TESObjectREFR::data), OBJ_REFR::LocationX),
            42.0
        );
        assert_eq!(e.get(this.at(TESObjectREFR::data), OBJ_REFR::AngleY), 0.25);
        // the source's cell takes the reference; with no 3D the I/O manager is kicked
        assert_eq!(calls_to(&log, 0x0054_8230), vec![vec![cell, me, 0]]);
        e.set_global(GLOBAL_IO_MANAGER, 0x0ab0u32);
        assert_eq!(calls_to(&log, 0x0045_6520).len(), 1);
        // the scale is set from the source's
        assert_eq!(
            calls_to(&log, 0x0056_7490),
            vec![vec![me, 1.0f32.to_bits()]]
        );
        // nothing changed that needs a new 3D, and the source has none: Set3D(0, 1) once
        assert_eq!(set3d_calls(&log, this), vec![vec![me, 0, 1]]);
    }

    #[test]
    fn copy_removes_the_reference_from_its_old_cell_when_the_source_has_none() {
        let (mut e, this, other) = copy_engine();
        let old = e.mem.alloc(0x40);
        e.set(this, TESObjectREFR::pParentCell, Ptr::new(old));
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, CELL_REMOVE_REFERENCE),
            vec![vec![old, this.addr()]]
        );
        assert!(calls_to(&log, 0x0054_8230).is_empty());
    }

    #[test]
    fn copy_rebuilds_the_3d_when_the_copy_differs() {
        // each change marks the reference dirty: a second Set3D(0, 1)
        type Change = Box<dyn Fn(&mut Engine, Ptr<TESObjectREFR>, Ptr<TESObjectREFR>)>;
        let cases: Vec<(&str, Change)> = vec![
            (
                "other base object",
                Box::new(|e, this, _| {
                    let base = form(e, 0x28, 1, 0);
                    e.set(
                        this.at(TESObjectREFR::data),
                        OBJ_REFR::pObjectReference,
                        Ptr::new(base),
                    );
                }),
            ),
            (
                "other scale on a non-actor",
                Box::new(|e, _, other| {
                    e.set(other, TESObjectREFR::fRefScale, 2.0f32);
                }),
            ),
            (
                "other ammo count",
                Box::new(|e, this, _| {
                    let base = form(e, 0x29, 1, 0);
                    e.set(
                        this.at(TESObjectREFR::data),
                        OBJ_REFR::pObjectReference,
                        Ptr::new(base),
                    );
                    // the stack counts differ
                    e.register(0x0041_8770, |_, a| (a[0] & 0xff).into_ret());
                }),
            ),
            (
                "other radius",
                Box::new(|e, _, _| {
                    returns(e, 0x0040_eb10, 0);
                }),
            ),
            (
                "other 00569580 value",
                Box::new(|e, _, _| {
                    e.register(0x0056_9580, |_, a| a[0].into_ret());
                }),
            ),
        ];
        for (name, change) in cases {
            let (mut e, this, other) = copy_engine();
            change(&mut e, this, other);
            e.call_log = Some(vec![]);
            e.call(0x0055_c3e0, &args![this, other]);
            let log = e.call_log.take().unwrap();
            assert_eq!(set3d_calls(&log, this).len(), 2, "{name}");
        }
    }

    #[test]
    fn copy_ignores_a_different_scale_for_an_actor() {
        let (mut e, this, other) = copy_engine();
        e.set(other, TESObjectREFR::fRefScale, 2.0f32);
        e.register(0x00ff_0012, |_, _| true.into_ret()); // IsActor, for both
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        assert_eq!(set3d_calls(&log, this).len(), 1);
        assert_eq!(
            calls_to(&log, 0x0056_7490),
            vec![vec![this.addr(), 2.0f32.to_bits()]]
        );
    }

    #[test]
    fn copy_keeps_the_existing_3d_in_step_with_the_new_position() {
        let (mut e, this, other) = copy_engine();
        let (me, them) = (this.addr(), other.addr());
        e.mem.set_u32(me + FAKE_3D, 0x3d00); // this has a 3D
        e.mem.set_u32(them + FAKE_3D, 0x3d01); // so does the source
        returns(&mut e, 0x0056_fa00, 0x0fa0);
        let cell = e.mem.alloc(0x40);
        e.set(other, TESObjectREFR::pParentCell, Ptr::new(cell));
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        // the node is moved to the reference's location and given its matrix
        assert_eq!(calls_to(&log, 0x0044_0460)[0], vec![0x3d00, me + 0x30]);
        assert_eq!(calls_to(&log, 0x0043_fa80)[0], vec![0x3d00, 0x0fa0]);
        // the simulation reset with a zero velocity and no I/O manager kick
        assert_eq!(calls_to(&log, 0x00c6_bd00)[0], vec![0x3d00, 1]);
        assert_eq!(
            calls_to(&log, 0x0043_d410)[0][1..],
            [0.0f32.to_bits(), 0, 0]
        );
        assert!(calls_to(&log, 0x0045_6520).is_empty());
        // with a 3D, no cell kick: the cell's AddReference still ran
        assert_eq!(calls_to(&log, 0x0054_8230), vec![vec![cell, me, 0]]);
        // finally the properties of the 3D are refreshed
        assert_eq!(calls_to(&log, 0x00a5_a040), vec![vec![0x3d00]]);
        assert_eq!(set3d_calls(&log, this).len(), 0);
    }

    #[test]
    fn copy_builds_a_new_3d_for_a_copy_whose_source_has_one() {
        let (mut e, this, other) = copy_engine();
        let (me, them) = (this.addr(), other.addr());
        e.mem.set_u32(them + FAKE_3D, 0x3d01);
        returns(&mut e, 0x0056_fa00, 0x0fa0);
        // the source's 3D has a collision object with a virtual at +0xdc
        e.register(0x00ff_0031, |_, _| Ret::default());
        let collision = object_with_vtable(&mut e, 0x20, &[(0xdc, 0x00ff_0031)]);
        returns(&mut e, 0x0096_11e0, collision);
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0044_0460), vec![vec![0x4e4e, me + 0x30]]);
        assert_eq!(calls_to(&log, 0x0043_fa80), vec![vec![0x4e4e, 0x0fa0]]);
        assert_eq!(
            calls_to(&log, 0x0044_0490),
            vec![vec![0x4e4e, 1.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, 0x00ff_0031),
            vec![vec![collision, 0x4e4e, 1]]
        );
    }

    #[test]
    fn copy_regenerates_the_dynamic_light_of_a_light_base() {
        let (mut e, this, other) = copy_engine();
        let (me, them) = (this.addr(), other.addr());
        e.mem.set_u32(them + FAKE_3D, 0x3d01);
        e.mem.set_u32(me + FAKE_3D, 0x3d00);
        let light = form(&mut e, 0x1e, 0x1111, 0);
        e.set(
            this.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(light),
        );
        e.set(
            other.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(light),
        );
        returns(&mut e, 0x0057_2d30, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x0050_d810),
            vec![vec![light, me, 0x3d00, 0]]
        );
        // the open flag was set from the extra data
        assert_eq!(calls_to(&log, 0x0047_aec0), vec![vec![me, 1, 1]]);
    }

    #[test]
    fn copy_recreates_the_teleport_data_when_the_door_differs() {
        let (mut e, this, other) = copy_engine();
        let (me, them) = (this.addr(), other.addr());
        let mine = e.mem.alloc(0x20);
        let theirs = e.mem.alloc(0x20);
        e.mem.set_u32(mine, 0xd001); // this teleport's door
        e.mem.set_u32(theirs, 0xd002); // the source's
        e.register_double(0x0056_8e50, move |_, a| {
            if a[0] == me { mine } else { theirs }.into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        // the old link is dropped (the old door told) and the teleport reset
        assert_eq!(calls_to(&log, 0x0056_8f30)[0], vec![0xd001]);
        assert_eq!(calls_to(&log, 0x0053_7e90)[0], vec![mine, 0]);
        let _ = them;
    }

    #[test]
    fn copy_keeps_the_teleport_data_when_the_door_is_the_same() {
        let (mut e, this, other) = copy_engine();
        let me = this.addr();
        let mine = e.mem.alloc(0x20);
        let theirs = e.mem.alloc(0x20);
        e.mem.set_u32(mine, 0xd001);
        e.mem.set_u32(theirs, 0xd001);
        e.register_double(0x0056_8e50, move |_, a| {
            if a[0] == me { mine } else { theirs }.into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x0053_7e90)
            .iter()
            .all(|c| c[1] != 0 || c[0] != mine));
    }

    #[test]
    fn copy_ends_by_linking_the_enable_parent() {
        let (mut e, this, other) = copy_engine();
        let me = this.addr();
        let parent = e.mem.alloc(0x80);
        returns(&mut e, 0x0056_a9f0, parent);
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        // first the old parent forgets the reference, at the end the new link is made
        assert_eq!(calls_to(&log, 0x0041_dda0), vec![vec![parent + 0x44, me]]);
        assert_eq!(calls_to(&log, 0x0041_dcd0), vec![vec![parent + 0x44, me]]);
        let order = addresses(&log);
        let forget = order.iter().position(|a| *a == 0x0041_dda0).unwrap();
        let link = order.iter().position(|a| *a == 0x0041_dcd0).unwrap();
        assert!(forget < link);
    }

    #[test]
    fn copy_clears_conflicting_shared_door_data_and_logs_it() {
        let (mut e, this, other) = copy_engine();
        let me = this.addr();
        let door = standard_refr(&mut e);
        let teleport = e.mem.alloc(0x20);
        e.mem.set_u32(teleport, door.addr());
        returns(&mut e, 0x0056_8e50, teleport);
        // the source's data is different, the door has its own teleport
        let door_teleport = e.mem.alloc(0x20);
        e.register_double(0x0056_8e50, move |_, a| {
            if a[0] == door.addr() {
                door_teleport
            } else {
                teleport
            }
            .into_ret()
        });
        // both have a lock-like record (0569140) and a 00567770 record
        returns(&mut e, 0x0056_9140, 1);
        returns(&mut e, 0x0056_7770, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        // the teleport on the door is linked back to this reference
        assert_eq!(
            calls_to(&log, 0x0053_7e90).last().unwrap(),
            &vec![door_teleport, me]
        );
        assert_eq!(calls_to(&log, 0x0041_ae70), vec![vec![door.addr() + 0x44]]);
        assert_eq!(calls_to(&log, 0x0056_7c50), vec![vec![door.addr()]]);
        assert_eq!(
            calls_to(&log, MESSAGE),
            vec![vec![SHARED_DATA_REMOVED_MESSAGE]]
        );
        // a door that is not persistent becomes persistent
        assert_eq!(
            calls_to(&log, 0x0056_5480).last().unwrap(),
            &vec![door.addr(), 1]
        );
    }

    #[test]
    fn copy_moves_water_lights_to_the_copy_and_off_the_source() {
        let (mut e, this, other) = copy_engine();
        let (me, them) = (this.addr(), other.addr());
        // a form flag on the source makes it walk the extra lists
        e.mem.set_u32(them + 8, 0x4000);
        let scene = e.mem.alloc(8);
        e.set_global(GLOBAL_TEMP_REFR_MANAGER, scene);
        returns(&mut e, 0x0070_ec90, 0x5c5c);
        let owner = standard_refr(&mut e);
        e.mem.set_u32(owner.addr() + FAKE_3D, 0x3d77);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, owner.addr());
        returns(&mut e, 0x0041_f810, node);
        let holder = e.mem.alloc(8);
        e.mem.set_u32(holder, 0x11ee);
        returns(&mut e, 0x0041_8250, holder);
        returns(&mut e, 0x004e_8030, 0x7a7a);
        returns(&mut e, 0x0040_30b0, 0x8b8b);
        returns(&mut e, 0x008d_8520, 0xd);
        let lights = e.mem.alloc(0x200);
        returns(&mut e, 0x00a5_9d30, lights);
        returns(&mut e, 0x0045_0b80, 0x9c9c);
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        for addr in [0x0057_c730u32, 0x004e_d8c0, 0x0049_c680] {
            let s = seen.clone();
            e.register_double(addr, move |e, a| {
                s.borrow_mut().push((addr, a[0], e.mem.u32(a[1])));
                Ret::default()
            });
        }
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        // the first pass (list of this) adds the light; the second (list of the
        // source) removes it from the scene after checking and marking
        assert_eq!(
            *seen.borrow(),
            vec![
                (0x0057_c730, lights + 0x128, 0x11ee),
                (0x0049_c680, lights + 0x128, 0x11ee),
                (0x004e_d8c0, lights + 0x128, 0x11ee),
            ]
        );
        assert_eq!(e.mem.u8(lights + 0x83), 1);
        assert_eq!(calls_to(&log, 0x0045_0b80), vec![vec![0]]);
        assert_eq!(calls_to(&log, 0x00b5_eed0), vec![vec![0x9c9c, 0x11ee]]);
        assert_eq!(
            calls_to(&log, 0x0041_f840),
            vec![
                vec![owner.addr() + 0x44, me, 0],
                vec![owner.addr() + 0x44, me, 1]
            ]
        );
        // the scene node came from the scene manager's object and the owner
        assert_eq!(calls_to(&log, 0x004e_8030)[0], vec![0x5c5c, owner.addr()]);
    }

    #[test]
    fn copy_relinks_the_extra_data_references_of_both_lists() {
        let (mut e, this, other) = copy_engine();
        let (me, them) = (this.addr(), other.addr());
        e.mem.set_u32(them + 8, 0x4000);
        // the link lists: this one entry, the source one entry flagged 1 | 2
        let x1 = e.mem.alloc(0x80);
        let x2 = e.mem.alloc(0x80);
        let entry1 = e.mem.alloc(8);
        e.mem.set_u32(entry1, x1);
        let node1 = e.mem.alloc(8);
        e.mem.set_u32(node1, entry1);
        let entry2 = e.mem.alloc(8);
        e.mem.set_u32(entry2, x2);
        e.mem.set_u32(entry2 + 4, 3);
        let node2 = e.mem.alloc(8);
        e.mem.set_u32(node2, entry2);
        e.register_double(0x0041_f170, move |_, a| {
            (if a[0] == me + 0x44 { node1 } else { node2 }).into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        // this list: both directions cleared for the entry's reference; the source's list: set
        assert_eq!(
            calls_to(&log, 0x0041_f4c0),
            vec![vec![x1 + 0x44, me, 0], vec![x2 + 0x44, me, 1]]
        );
        assert_eq!(
            calls_to(&log, 0x0041_f650),
            vec![vec![x1 + 0x44, me, 0], vec![x2 + 0x44, me, 1]]
        );
        // an entry without the flags is not set
        e.mem.set_u32(entry2 + 4, 0);
        e.call_log = Some(vec![]);
        e.call(0x0055_c3e0, &args![this, other]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0041_f4c0), vec![vec![x1 + 0x44, me, 0]]);
    }

    // ---- 0055d230: Compare ---------------------------------------------------

    fn compare_engine() -> (Engine, Ptr<TESObjectREFR>, Ptr<TESObjectREFR>) {
        let mut e = engine();
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        stub(&mut e, &[0x0048_5270, 0x0041_27e0]);
        e.register(MEMCMP, |e, a| {
            let (x, y) = (e.mem.bytes(a[0], a[2]), e.mem.bytes(a[1], a[2]));
            (x.cmp(&y) as i32).into_ret()
        });
        e.register(0x0059_8040, |e, a| e.mem.f32(a[0] + 0x3c).into_ret());
        let this = standard_refr(&mut e);
        let other = standard_refr(&mut e);
        e.set(this, TESObjectREFR::fRefScale, 1.0f32);
        e.set(other, TESObjectREFR::fRefScale, 1.0f32);
        (e, this, other)
    }

    #[test]
    fn compare_finds_references_that_match() {
        let (mut e, this, other) = compare_engine();
        assert!(!e.call(0x0055_d230, &args![this, other]).bool());
    }

    #[test]
    fn compare_finds_every_kind_of_difference() {
        // not a reference
        let (mut e, this, _) = compare_engine();
        e.register(DYNAMIC_CAST, |_, _| 0u32.into_ret());
        assert!(e.call(0x0055_d230, &args![this, 0x1234u32]).bool());

        // the form components differ
        let (mut e, this, other) = compare_engine();
        returns(&mut e, 0x0048_5270, 1);
        assert!(e.call(0x0055_d230, &args![this, other]).bool());

        // the OBJ_REFR data differs (one byte of the rotation)
        let (mut e, this, other) = compare_engine();
        e.mem.set_u8(other.addr() + 0x2a, 9);
        assert!(e.call(0x0055_d230, &args![this, other]).bool());

        // the parent cells differ
        let (mut e, this, other) = compare_engine();
        e.set(other, TESObjectREFR::pParentCell, Ptr::new(0x4000));
        assert!(e.call(0x0055_d230, &args![this, other]).bool());

        // the extra lists differ
        let (mut e, this, other) = compare_engine();
        returns(&mut e, 0x0041_27e0, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0055_d230, &args![this, other]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x0041_27e0),
            vec![vec![this.addr() + 0x44, other.addr() + 0x44]]
        );

        // the scales differ, or are NaN
        let (mut e, this, other) = compare_engine();
        e.set(other, TESObjectREFR::fRefScale, 1.5f32);
        assert!(e.call(0x0055_d230, &args![this, other]).bool());
        let (mut e, this, other) = compare_engine();
        e.set(this, TESObjectREFR::fRefScale, f32::NAN);
        e.set(other, TESObjectREFR::fRefScale, f32::NAN);
        assert!(e.call(0x0055_d230, &args![this, other]).bool());
    }

    // ---- 0055d310: HasContainer ------------------------------------------------

    #[test]
    fn has_container_depends_on_the_kind_of_base_object() {
        let mut e = engine();
        let refr = new_refr(&mut e);
        let data = refr.at(TESObjectREFR::data);
        // no base object
        assert_eq!(e.call(0x0055_d310, &args![refr]).u32(), 0);
        for (kind, expected_offset) in [
            (0x1bu8, Some(0x30u32)),
            (0x29, None),
            (0x2a, Some(0x64)),
            (0x2b, Some(0x64)),
            (0x2c, None),
            (0x1a, None),
        ] {
            let base = form(&mut e, kind, 1, 0);
            e.set(data, OBJ_REFR::pObjectReference, Ptr::new(base));
            let found = e.call(0x0055_d310, &args![refr]).u32();
            assert_eq!(
                found,
                expected_offset.map_or(0, |o| base + o),
                "kind {kind:#x}"
            );
        }
    }

    // ---- 0055d3b0 ------------------------------------------------------------------

    #[test]
    fn base_check_accepts_the_base_or_the_two_remembered_bases() {
        let mut e = engine();
        let refr = new_refr(&mut e);
        let data = refr.at(TESObjectREFR::data);
        e.set(data, OBJ_REFR::pObjectReference, Ptr::new(0x1001));
        returns(&mut e, 0x0042_16f0, 0x1002);
        returns(&mut e, 0x0042_1720, 0x1003);
        for (form, expected) in [
            (0x1001u32, true),
            (0x1002, true),
            (0x1003, true),
            (0x1004, false),
        ] {
            assert_eq!(e.call(0x0055_d3b0, &args![refr, form]).bool(), expected);
        }
    }

    // ---- 0055d420 --------------------------------------------------------------------

    #[test]
    fn altered_flag_reaches_the_parent_cell_only_for_a_set_flag() {
        let mut e = engine();
        stub(&mut e, &[0x0048_4730]);
        e.register(0x00ff_0040, |_, _| Ret::default());
        let cell = object_with_vtable(&mut e, 0x20, &[(0xc8, 0x00ff_0040)]);
        let refr = standard_refr(&mut e);
        e.set(refr, TESObjectREFR::pParentCell, Ptr::new(cell));
        e.call_log = Some(vec![]);
        e.call(0x0055_d420, &args![refr, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x00ff_0040), vec![vec![cell, 1]]);
        assert_eq!(calls_to(&log, 0x0048_4730), vec![vec![refr.addr(), 1]]);

        // flag clear: only TESForm's handler
        e.call_log = Some(vec![]);
        e.call(0x0055_d420, &args![refr, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, 0x00ff_0040).is_empty());
        assert_eq!(calls_to(&log, 0x0048_4730), vec![vec![refr.addr(), 0]]);

        // a flagged form (iFormFlags & 0x4000) skips the cell
        e.mem.set_u32(refr.addr() + 8, 0x4000);
        e.call_log = Some(vec![]);
        e.call(0x0055_d420, &args![refr, 1u32]);
        assert!(calls_to(&e.call_log.take().unwrap(), 0x00ff_0040).is_empty());
    }

    // ---- 0055d480 --------------------------------------------------------------------

    fn name_engine() -> (Engine, Ptr<TESObjectREFR>) {
        let mut e = engine();
        stub(&mut e, &[0x0047_4cb0, 0x0042_26e0]);
        returns(&mut e, 0x0040_1280, 0x0aa1);
        returns(&mut e, 0x0042_16f0, 0);
        let refr = standard_refr(&mut e);
        (e, refr)
    }

    #[test]
    fn reference_name_prefers_the_special_case_then_the_base_objects_name() {
        let (mut e, refr) = name_engine();
        // 00474cb0 true: the special string
        returns(&mut e, 0x0047_4cb0, 1);
        assert_eq!(e.call(0x0055_d480, &args![refr]).u32(), 0x0aa1);
        returns(&mut e, 0x0047_4cb0, 0);
        // no base object: the empty string
        assert_eq!(e.call(0x0055_d480, &args![refr]).u32(), EMPTY_STRING);
        // a base object with a name
        let name = e.mem.alloc(8);
        e.mem.set_cstr(name, b"Sunny");
        e.register_double(0x00ff_0041, move |_, _| name.into_ret());
        let base = object_with_vtable(&mut e, 0x40, &[(0x130, 0x00ff_0041)]);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(base),
        );
        assert_eq!(e.call(0x0055_d480, &args![refr]).u32(), name);
        // the data handler says no names: empty again
        returns(&mut e, 0x0042_26e0, 1);
        let handler = e.mem.alloc(8);
        e.set_global(GLOBAL_DATA_HANDLER, handler);
        assert_eq!(e.call(0x0055_d480, &args![refr]).u32(), EMPTY_STRING);
    }

    #[test]
    fn reference_name_falls_back_to_the_leveled_creatures_original_base() {
        let (mut e, refr) = name_engine();
        let empty = e.mem.alloc(8);
        let name = e.mem.alloc(8);
        e.mem.set_cstr(name, b"Gecko");
        e.register_double(0x00ff_0041, move |_, _| empty.into_ret());
        e.register_double(0x00ff_0042, move |_, _| name.into_ret());
        let base = object_with_vtable(&mut e, 0x40, &[(0x130, 0x00ff_0041)]);
        let original = object_with_vtable(&mut e, 0x40, &[(0x130, 0x00ff_0042)]);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(base),
        );
        returns(&mut e, 0x0042_16f0, original);
        // the base's own name is empty: the original base's name
        assert_eq!(e.call(0x0055_d480, &args![refr]).u32(), name);
        // no original base either: the (empty) name stays
        returns(&mut e, 0x0042_16f0, 0);
        assert_eq!(e.call(0x0055_d480, &args![refr]).u32(), empty);
    }

    // ---- 0055d520 --------------------------------------------------------------------

    #[test]
    fn full_name_is_the_base_objects() {
        let mut e = engine();
        returns(&mut e, 0x0048_2720, 0x0f11);
        let refr = new_refr(&mut e);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(0x5555),
        );
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0055_d520, &args![refr]).u32(), 0x0f11);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0048_2720), vec![vec![0x5555]]);
    }

    // ---- 0055d540: GetFormDetailedString -------------------------------------------

    #[test]
    fn detailed_string_names_the_base_world_and_cell() {
        let mut e = engine();
        stub(
            &mut e,
            &[
                SPRINTF,
                STRING_CONSTRUCT,
                STRING_FORMAT,
                STRING_DESTRUCT,
                0x0057_5d70,
            ],
        );
        returns(&mut e, 0x0044_0e30, 0x0b01);
        returns(&mut e, 0x0040_1280, 0x0b02);
        e.register(READ_FIRST_DWORD, |_, _| 0x0b03u32.into_ret());
        let refr = standard_refr(&mut e);
        // base object and parent cell with a name slot (0x130) and a string slot (0x90)
        e.register(0x00ff_0050, |_, _| 0x0c01u32.into_ret());
        e.register(0x00ff_0051, |_, _| Ret::default());
        let base = object_with_vtable(&mut e, 0x40, &[(0x130, 0x00ff_0050)]);
        e.mem.set_u32(base + 0xc, 0x0001_00aa);
        let cell = object_with_vtable(&mut e, 0x40, &[(0x90, 0x00ff_0051)]);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(base),
        );
        e.set(refr, TESObjectREFR::pParentCell, Ptr::new(cell));
        e.mem.set_u32(refr.addr() + 0xc, 0x0002_00bb);
        let out = e.mem.alloc(8);
        e.call_log = Some(vec![]);
        e.call(0x0055_d540, &args![refr, out]);
        let log = e.call_log.take().unwrap();
        // the base fragment, formatted with the kind, the name and the ID
        let fragments = calls_to(&log, SPRINTF);
        assert_eq!(fragments.len(), 1);
        assert_eq!(
            fragments[0][1..],
            [DETAILED_BASE_FORMAT, 0x0b01, 0x0c01, 0x0001_00aa]
        );
        let first_buffer = fragments[0][0];
        // the cell is asked for its string into the temporary BSStringT
        let cell_call = calls_to(&log, 0x00ff_0051);
        assert_eq!(cell_call.len(), 1);
        assert_eq!(cell_call[0][0], cell);
        // the final text
        assert_eq!(
            calls_to(&log, STRING_FORMAT)
                .into_iter()
                .filter(|c| c[0] == out)
                .collect::<Vec<_>>(),
            vec![vec![
                out,
                DETAILED_FORMAT,
                0x0b01,
                0x0b02,
                0x0002_00bb,
                first_buffer,
                0x0b03
            ]]
        );
        assert_eq!(calls_to(&log, STRING_CONSTRUCT).len(), 1);
        assert_eq!(calls_to(&log, STRING_DESTRUCT).len(), 1);
    }

    #[test]
    fn detailed_string_adds_the_world_space_fragment_when_there_is_one() {
        let mut e = engine();
        stub(
            &mut e,
            &[SPRINTF, STRING_CONSTRUCT, STRING_FORMAT, STRING_DESTRUCT],
        );
        returns(&mut e, 0x0044_0e30, 1);
        returns(&mut e, 0x0040_1280, 2);
        e.register(READ_FIRST_DWORD, |_, _| 3u32.into_ret());
        let refr = standard_refr(&mut e);
        e.register(0x00ff_0050, |_, _| 0x0c01u32.into_ret());
        let world = object_with_vtable(&mut e, 0x40, &[(0x130, 0x00ff_0050)]);
        e.mem.set_u32(world + 0xc, 0x0003_00cc);
        returns(&mut e, 0x0057_5d70, world);
        let out = e.mem.alloc(8);
        e.call_log = Some(vec![]);
        e.call(0x0055_d540, &args![refr, out]);
        let log = e.call_log.take().unwrap();
        let fragments = calls_to(&log, SPRINTF);
        assert_eq!(fragments.len(), 1);
        assert_eq!(
            fragments[0][1..],
            [DETAILED_WORLD_FORMAT, 0x0c01, 0x0003_00cc]
        );
    }

    // ---- 0055d6d0 --------------------------------------------------------------------

    #[test]
    fn lod_flag_follows_the_references_test() {
        let mut e = engine();
        stub(&mut e, &[0x0070_5b10, 0x0054_b800, 0x0047_6ab0]);
        returns(&mut e, 0x0045_c6b0, 0x0100);
        let refr = new_refr(&mut e);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(0x5555),
        );
        let target = 0x0222u32;
        e.register_double(0x00ff_0060, move |_, _| target.into_ret());
        let object = object_with_vtable(&mut e, 0x20, &[(0x10, 0x00ff_0060)]);

        // the test holds, the target's flag is not 9: set it and run 0054b800
        returns(&mut e, 0x0056_5090, 1);
        returns(&mut e, 0x0070_58c0, 3);
        e.call_log = Some(vec![]);
        e.call(0x0055_d6d0, &args![refr, object]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0070_5b10), vec![vec![target, 9]]);
        assert_eq!(calls_to(&log, 0x0054_b800), vec![vec![target]]);

        // the test fails and the flag is 9: it gets the base's LOD multiplier
        returns(&mut e, 0x0056_5090, 0);
        returns(&mut e, 0x0070_58c0, 9);
        e.call_log = Some(vec![]);
        e.call(0x0055_d6d0, &args![refr, object]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0045_c6b0), vec![vec![0x5555]]);
        assert_eq!(calls_to(&log, 0x0070_5b10), vec![vec![target, 0x0100]]);
        assert_eq!(calls_to(&log, 0x0047_6ab0), vec![vec![target]]);

        // they agree: nothing is changed
        returns(&mut e, 0x0056_5090, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_d6d0, &args![refr, object]);
        assert!(calls_to(&e.call_log.take().unwrap(), 0x0070_5b10).is_empty());

        // no object, or an object without a target: nothing
        e.call_log = Some(vec![]);
        e.call(0x0055_d6d0, &args![refr, 0u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    // ---- 0055d760: InitItem ----------------------------------------------------

    const INIT_CALLEES: &[u32] = &[
        0x0040_1280,
        0x0041_6be0,
        0x0041_81e0,
        0x0041_9970,
        0x0041_9ad0,
        0x0041_aeb0,
        0x0042_1f30,
        0x0042_5fd0,
        0x0042_cde0,
        0x0043_8f50,
        0x0043_9180,
        0x0043_9f90,
        0x0043_b2b0,
        0x0045_34f0,
        0x0046_23f0,
        0x0046_a010,
        0x0047_53d0,
        0x0047_c850,
        0x0047_cdb0,
        0x0047_ce10,
        0x0048_46a0,
        0x0048_4ab0,
        0x0048_4af0,
        0x004b_1480,
        0x004b_f220,
        0x004d_1440,
        0x004d_1610,
        0x004d_1960,
        0x004f_ede0,
        0x004f_f150,
        0x0054_4c30,
        0x0054_4c60,
        0x0054_9580,
        0x0055_e230,
        0x0056_4e60,
        0x0056_5730,
        0x0056_7490,
        0x0056_9840,
        0x0056_a9f0,
        0x0056_aa70,
        0x0056_ac00,
        0x0056_afc0,
        0x0057_22c0,
        0x0057_2c80,
        0x0057_5d70,
        0x0058_6170,
        0x0058_78d0,
        0x005f_6e60,
        0x006f_cfa0,
        0x0088_4350,
        0x008a_1800,
        0x0096_d470,
        0x0098_ddd0,
        0x0040_1000,
        MESSAGE,
        EXTRA_REMOVE_TYPE,
        0x0041_8770,
        0x0044_0e30,
    ];

    /// A reference ready for `InitItem`: every callee stubbed, a base object
    /// of type 0x28 and the CRT's float tests given real bodies. The
    /// scale getter reads the float at +0x3c.
    fn init_engine() -> (Engine, Ptr<TESObjectREFR>) {
        let mut e = engine();
        stub(&mut e, INIT_CALLEES);
        e.register(CRT_FINITE, |_, a| {
            u32::from(f64::take(a, &mut 0).is_finite()).into_ret()
        });
        e.register(CRT_ISNAN, |_, a| {
            u32::from(f64::take(a, &mut 0).is_nan()).into_ret()
        });
        e.register(0x0056_7400, |e, a| e.mem.f32(a[0] + 0x3c).into_ret());
        returns(&mut e, EXTRA_GET_DATA, 0);
        // ClampAngle: the identity unless a test changes it
        e.register(0x004b_1480, |_, a| f32::from_bits(a[0]).into_ret());
        // health percent 1.0 unless a test changes it
        e.register(0x0041_b550, |_, _| 1.0f32.into_ret());
        // TLS warning counter: the getter returns the stored value
        let counter = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let c = counter.clone();
        e.register_double(0x0046_e8a0, move |_, _| c.get().into_ret());
        let c = counter;
        e.register_double(0x004f_ffe0, move |_, a| {
            c.set(a[0]);
            Ret::default()
        });
        let refr = standard_refr(&mut e);
        let base = form(&mut e, 0x28, 0x0001_0000, 0);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(base),
        );
        e.set(refr, TESObjectREFR::fRefScale, 1.0f32);
        // the save/load object for the enable-parent part
        let save_load = e.mem.alloc(8);
        e.set_global(GLOBAL_SAVE_LOAD, save_load);
        (e, refr)
    }

    #[test]
    fn init_item_does_nothing_when_the_form_says_to_skip_it() {
        let (mut e, refr) = init_engine();
        e.mem.set_u32(refr.addr() + 8, 8);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }

    #[test]
    fn init_item_replaces_a_corrupt_position_and_rotation_with_the_zero_vector() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        for (i, v) in [1.0f32, f32::INFINITY, 3.0].iter().enumerate() {
            e.mem.set_f32(me + 0x30 + 4 * i as u32, *v);
        }
        for (i, v) in [4.0f32, 5.0, f32::NAN].iter().enumerate() {
            e.mem.set_f32(me + 0x24 + 4 * i as u32, *v);
        }
        for (i, v) in [10.0f32, 20.0, 30.0].iter().enumerate() {
            e.set_global(ZERO_VECTOR + 4 * i as u32, *v);
        }
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, MESSAGE),
            vec![vec![CORRUPT_LOCATION_MESSAGE], vec![CORRUPT_ANGLE_MESSAGE]]
        );
        for i in 0..3 {
            assert_eq!(e.mem.f32(me + 0x30 + 4 * i), 10.0 * (i + 1) as f32);
            assert_eq!(e.mem.f32(me + 0x24 + 4 * i), 10.0 * (i + 1) as f32);
        }
    }

    #[test]
    fn init_item_leaves_a_sound_position_alone() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        for (i, v) in [1.0f32, 2.0, 3.0].iter().enumerate() {
            e.mem.set_f32(me + 0x30 + 4 * i as u32, *v);
            e.mem.set_f32(me + 0x24 + 4 * i as u32, *v);
        }
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, MESSAGE).is_empty());
        assert_eq!(e.mem.f32(me + 0x34), 2.0);
        // the extra list is initialized with the reference
        assert_eq!(calls_to(&log, 0x0041_6be0), vec![vec![me + 0x44, me]]);
    }

    #[test]
    fn init_item_scales_the_health_by_the_bases_health() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        let base = e.mem.u32(me + 0x20);
        e.register(0x0041_b550, |_, _| 0.5f32.into_ret());
        returns(&mut e, 0x0048_73d0, 100);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0048_73d0), vec![vec![base]]);
        assert_eq!(
            calls_to(&log, 0x0041_9970),
            vec![vec![me + 0x44, 50.0f32.to_bits()]]
        );

        // full health: not touched
        e.register(0x0041_b550, |_, _| 1.0f32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        assert!(calls_to(&e.call_log.take().unwrap(), 0x0041_9970).is_empty());
    }

    #[test]
    fn init_item_counts_a_single_ammo_stack_from_the_base() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        let ammo = form(&mut e, 0x29, 0x0001_0001, 0);
        e.mem.set_u8(ammo + 0x84, 7); // the byte `00401170` reads at +0x80
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(ammo),
        );
        returns(&mut e, 0x0041_8770, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0041_9ad0), vec![vec![me + 0x44, 7]]);
        // a stack of two keeps its count
        returns(&mut e, 0x0041_8770, 2);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        assert!(calls_to(&e.call_log.take().unwrap(), 0x0041_9ad0).is_empty());
    }

    #[test]
    fn init_item_marks_a_deleted_base_object_deleted() {
        let (mut e, refr) = init_engine();
        let base = e.mem.u32(refr.addr() + 0x20);
        e.mem.set_u32(base + 8, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x00ff_0013), vec![vec![refr.addr(), 1]]);
        // a leveled base is reported
        e.mem.set_u8(base + 4, 0x2c);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, MESSAGE), vec![vec![LEVELED_BASE_MESSAGE]]);
    }

    #[test]
    fn init_item_keeps_a_multi_bound_marker_for_a_scaled_reference() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        e.set(refr, TESObjectREFR::fRefScale, 2.0f32);
        returns(&mut e, 0x0043_9f90, 1);
        let block = e.mem.alloc(12);
        returns(&mut e, 0x0040_1000, block);
        e.register(0x0043_8f50, |_, a| a[0].into_ret());
        // the marker's position is the three floats at the start of the block
        for (i, v) in [5.0f32, 6.0, 7.0].iter().enumerate() {
            e.mem.set_f32(block + 4 * i as u32, *v);
        }
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen2 = seen.clone();
        e.register_double(0x0098_ddd0, move |e, a| {
            seen2
                .borrow_mut()
                .push((a[0], e.mem.u32(a[1]), e.mem.u32(a[1] + 8)));
            Ret::default()
        });
        let world = e.mem.alloc(0x10);
        returns(&mut e, 0x0057_5d70, world);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        // a new marker is attached to the extra list; the position scaled in place
        assert_eq!(calls_to(&log, 0x0042_1f30), vec![vec![me + 0x44, block]]);
        assert_eq!(calls_to(&log, 0x0043_9180)[0][1], 2.0f32.to_bits());
        assert_eq!(
            *seen.borrow(),
            vec![(block, 5.0f32.to_bits(), 7.0f32.to_bits())]
        );
        // the scale goes back to 1.0, the world space learns of the bound ref
        assert_eq!(
            calls_to(&log, 0x0056_7490),
            vec![vec![me, 1.0f32.to_bits()]]
        );
        assert_eq!(calls_to(&log, 0x0058_78d0), vec![vec![world, me]]);
    }

    #[test]
    fn init_item_puts_a_non_actor_at_its_starting_position() {
        let (mut e, refr) = init_engine();
        returns(&mut e, 0x0057_2c80, 1);
        for (i, v) in [1.0f32, 2.0, 3.0].iter().enumerate() {
            e.set_global(ZERO_VECTOR + 4 * i as u32, *v);
        }
        e.register(0x00ff_0070, |_, _| Ret::default());
        e.mem.set_u32(REFR_VTABLE + 0x174, 0x00ff_0070);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x00ff_0070),
            vec![vec![
                refr.addr(),
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits()
            ]]
        );
    }

    fn actor_refr(e: &mut Engine) -> Ptr<TESObjectREFR> {
        let refr = standard_refr(e);
        let base = form(e, 0x28, 0x0001_0000, 0);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(base),
        );
        e.set(refr, TESObjectREFR::fRefScale, 1.0f32);
        for (slot, target) in [
            (0x100u32, 0x00ff_0080u32),
            (0x290, 0x00ff_0081),
            (0x46c, 0x00ff_0082),
            (0x360, 0x00ff_0083),
            (0x218, 0x00ff_0084),
            (0x308, 0x00ff_0085),
            (0x1f8, 0x00ff_0086),
        ] {
            e.mem.set_u32(REFR_VTABLE + slot, target);
        }
        e.register(0x00ff_0080, |_, _| true.into_ret());
        stub(
            e,
            &[
                0x00ff_0081,
                0x00ff_0082,
                0x00ff_0083,
                0x00ff_0084,
                0x00ff_0085,
                0x00ff_0086,
            ],
        );
        refr
    }

    #[test]
    fn init_item_finishes_an_actor_loaded_dead_without_the_basics() {
        let (mut e, _) = init_engine();
        let actor = actor_refr(&mut e);
        let me = actor.addr();
        // an actor that is not alive-and-initialized: virtual +0x290 false
        // gets +0x46c; not dead per +0x360 but dead per 005722c0
        returns(&mut e, 0x0057_22c0, 1);
        let data = e.mem.alloc(0x80);
        returns(&mut e, 0x0041_81e0, data);
        returns(&mut e, 0x0047_cdb0, 1);
        returns(&mut e, 0x0056_afc0, 0);
        returns(&mut e, 0x0046_1580, 0);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![actor]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x00ff_0082), vec![vec![me]]);
        assert_eq!(calls_to(&log, 0x008a_1800), vec![vec![me, 2]]);
        assert_eq!(
            calls_to(&log, 0x0096_d470),
            vec![vec![OBJECT_PROCESS_LISTS, me, 3]]
        );
        // the base data at +0x30 of the actor's data block
        assert_eq!(calls_to(&log, 0x0047_ce10), vec![vec![data + 0x30, me]]);
        // 0055e200 was the translation: it asks 00461580 with 0x200
        assert_eq!(calls_to(&log, 0x0046_1580), vec![vec![data + 0x30, 0x200]]);
        assert_eq!(calls_to(&log, 0x00ff_0086), vec![vec![me, 1]]);
        // not a creature: the guard flag is not set
        assert!(calls_to(&log, 0x00ff_0085).is_empty());
    }

    #[test]
    fn init_item_flags_a_creature_guard_class() {
        let (mut e, _) = init_engine();
        let actor = actor_refr(&mut e);
        let me = actor.addr();
        e.register(0x00ff_0084, |_, _| true.into_ret());
        returns(&mut e, 0x0088_4350, 0x0c1a);
        returns(&mut e, 0x005f_6e60, 1);
        let data = e.mem.alloc(0x80);
        returns(&mut e, 0x0041_81e0, data);
        returns(&mut e, 0x0046_1580, 0);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![actor]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x005f_6e60), vec![vec![0x0c1a]]);
        assert_eq!(calls_to(&log, 0x00ff_0085), vec![vec![me, 1]]);
    }

    #[test]
    fn init_item_updates_the_players_speed_hook() {
        let (mut e, _) = init_engine();
        let player = actor_refr(&mut e);
        e.set_global(GLOBAL_PLAYER, player.addr());
        returns(&mut e, 0x0041_81e0, 0x4000);
        returns(&mut e, 0x0046_1580, 0);
        e.register(0x0064_7f00, |_, _| 77.5f32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![player]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, 0x0064_7f00),
            vec![vec![player.addr() + 0xa4, 0, 0, 0, 1, 0, 0]]
        );
        assert_eq!(calls_to(&log, 0x0055_e230), vec![vec![77.5f32.to_bits()]]);
    }

    #[test]
    fn init_item_follows_the_enable_state_parent() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        let parent = e.mem.alloc(0x80);
        e.mem.set_u32(parent + 8, 0x800); // the parent is disabled
        returns(&mut e, 0x0056_a9f0, parent);
        // the opposite flag: this reference is disabled when the parent is not
        returns(&mut e, 0x0056_aa70, 1);
        returns(&mut e, 0x0047_c850, 0);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0048_4af0), vec![vec![me, 0]]);
        // the save/load object's flag was set to 0 and then restored
        let flags = calls_to(&log, 0x0045_34f0);
        assert_eq!(flags.len(), 2);
        assert_eq!(flags[0][1], 0);

        // same sense: this reference takes the parent's state
        returns(&mut e, 0x0056_aa70, 0);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0048_4af0), vec![vec![me, 1]]);
    }

    #[test]
    fn init_item_reports_errors_raised_while_it_ran_in_three_shapes() {
        // no cell
        let (mut e, refr) = init_engine();
        let counter_bump = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let c = counter_bump.clone();
        // the second read of the warning counter sees a new warning
        e.register_double(0x0046_e8a0, move |_, _| {
            c.set(c.get() + 1);
            (if c.get() == 1 { 0u32 } else { 2 }).into_ret()
        });
        let base = e.mem.u32(refr.addr() + 0x20);
        e.register(0x00ff_0090, |_, _| 0x0d01u32.into_ret());
        let named_base = object_with_vtable(&mut e, 0x100, &[(0x130, 0x00ff_0090)]);
        e.mem.set_u8(named_base + 4, 0x28);
        e.mem.set_u32(named_base + 0xc, 0x0001_0000);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(named_base),
        );
        let _ = base;
        returns(&mut e, 0x0040_1280, 0x0d02);
        e.mem.set_u32(refr.addr() + 0xc, 0x00ab_cdef);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, MESSAGE),
            vec![vec![
                INIT_ITEM_ERROR_NO_CELL,
                0x0d01,
                0x0001_0000,
                0x0d02,
                0x00ab_cdef
            ]]
        );

        // a plain cell
        let cell = object_with_vtable(&mut e, 0x100, &[(0x130, 0x00ff_0090)]);
        e.mem.set_u32(cell + 0xc, 0x0007_0001);
        e.set(refr, TESObjectREFR::pParentCell, Ptr::new(cell));
        returns(&mut e, 0x0042_5fd0, 1);
        counter_bump.set(0);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, MESSAGE),
            vec![vec![
                INIT_ITEM_ERROR_CELL,
                0x0d01,
                0x0001_0000,
                0x0d02,
                0x00ab_cdef,
                0x0d01,
                0x0007_0001
            ]]
        );

        // a cell with coordinates in a world
        returns(&mut e, 0x0042_5fd0, 0);
        let world = object_with_vtable(&mut e, 0x100, &[(0x130, 0x00ff_0090)]);
        e.mem.set_u32(world + 0xc, 0x0009_0001);
        returns(&mut e, 0x0057_5d70, world);
        returns(&mut e, 0x0054_4c30, 12);
        returns(&mut e, 0x0054_4c60, 34);
        counter_bump.set(0);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, MESSAGE),
            vec![vec![
                INIT_ITEM_ERROR_WORLD,
                0x0d01,
                0x0001_0000,
                0x0d02,
                0x00ab_cdef,
                0x0d01,
                0x0007_0001,
                12,
                34,
                0x0d01,
                0x0009_0001
            ]]
        );
    }

    #[test]
    fn init_item_initializes_a_containers_inventory_and_the_script() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        let container = form(&mut e, 0x1b, 5, 0);
        e.set(
            refr.at(TESObjectREFR::data),
            OBJ_REFR::pObjectReference,
            Ptr::new(container),
        );
        returns(&mut e, 0x004b_f220, 0x1c1c);
        returns(&mut e, 0x0042_cde0, 1);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        for step in [0x004d_1440, 0x004d_1610, 0x004d_1960, 0x0042_cde0] {
            assert_eq!(calls_to(&log, step), vec![vec![0x1c1c]], "{step:08x}");
        }
        assert_eq!(calls_to(&log, 0x0041_aeb0), vec![vec![me + 0x44]]);
        assert_eq!(calls_to(&log, 0x0056_5730), vec![vec![me]]);
        assert_eq!(calls_to(&log, 0x0046_a010), vec![vec![me, 0]]);
    }

    #[test]
    fn init_item_hides_the_terrain_tree_of_a_disabled_exterior_object() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        e.mem.set_u32(me + 8, 0x800); // disabled
        returns(&mut e, 0x0054_9580, 1);
        let world = e.mem.alloc(0x10);
        returns(&mut e, 0x0057_5d70, world);
        returns(&mut e, 0x0058_6170, 0x7e7e);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x006f_cfa0), vec![vec![0x7e7e, me, 1]]);
    }

    #[test]
    fn init_item_clamps_the_rotation_and_drops_the_extra_of_type_0x6e() {
        let (mut e, refr) = init_engine();
        let me = refr.addr();
        let base = e.mem.u32(me + 0x20);
        e.mem.set_u8(base + 4, 0x2a);
        e.mem.set_f32(me + 0x2c, 7.0);
        e.register(0x004b_1480, |_, a| (f32::from_bits(a[0]) - 6.0).into_ret());
        returns(&mut e, EXTRA_GET_DATA, 0x0e6e);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.f32(me + 0x2c), 1.0);
        assert_eq!(calls_to(&log, EXTRA_GET_DATA), vec![vec![me + 0x44, 0x6e]]);
        assert_eq!(
            calls_to(&log, EXTRA_REMOVE_TYPE),
            vec![vec![me + 0x44, 0x6e]]
        );
        // a base of type 0x28 keeps it
        let base = e.mem.u32(me + 0x20);
        e.mem.set_u8(base + 4, 0x28);
        e.call_log = Some(vec![]);
        e.call(0x0055_d760, &args![refr]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, EXTRA_REMOVE_TYPE).len(), 0);
    }

    // ---- 0055e1d0 .. 0055e730 -----------------------------------------------------

    #[test]
    fn counters_and_flags_in_the_tail_of_the_range() {
        let mut e = engine();
        let cell = e.mem.alloc(0x100);
        e.mem.set_u16(cell + 0xa8, 5);
        e.call(0x0055_e1d0, &args![cell]);
        assert_eq!(e.mem.u16(cell + 0xa8), 4);
        e.mem.set_u16(cell + 0xa8, 0);
        e.call(0x0055_e1d0, &args![cell]);
        assert_eq!(e.mem.u16(cell + 0xa8), 0xffff);

        returns(&mut e, 0x0046_1580, 1);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0055_e200, &args![cell]).bool());
        assert_eq!(
            calls_to(&e.call_log.take().unwrap(), 0x0046_1580),
            vec![vec![cell, 0x200]]
        );
        returns(&mut e, 0x0046_1580, 0);
        assert!(e.call(0x0055_e200, &args![cell]).bool());
    }

    // ---- 0055e250 / 0055e3f0 / 0055e4d0 / 0055e5e0: group ordering --------------

    /// A cell object whose virtuals at +0x38, +0x3c and +0x110 are recorders
    /// returning 1, with `id` as its form ID.
    fn recording_cell(e: &mut Engine, id: u32) -> u32 {
        for addr in [0x00ff_00a0, 0x00ff_00a1, 0x00ff_00a2] {
            e.register(addr, |_, _| true.into_ret());
        }
        let cell = object_with_vtable(
            e,
            0x100,
            &[
                (0x38, 0x00ff_00a0),
                (0x3c, 0x00ff_00a1),
                (0x110, 0x00ff_00a2),
            ],
        );
        e.mem.set_u32(cell + 0xc, id);
        cell
    }

    fn group_engine() -> (Engine, Ptr<TESObjectREFR>, u32) {
        let mut e = engine();
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        stub(&mut e, &[GET_REF_PERSISTS]);
        e.set_global(GROUP_DATA_VTABLE_PTR, 0x0101_c654u32);
        let refr = standard_refr(&mut e);
        let cell = recording_cell(&mut e, 0x0007_0001);
        e.set(refr, TESObjectREFR::pParentCell, Ptr::new(cell));
        (e, refr, cell)
    }

    /// A group-data record (the class `fn_0055e5e0` produces).
    fn group_record(e: &mut Engine, kind: u32, form_id: u32) -> u32 {
        let record = e.mem.alloc(0x14);
        e.mem
            .set_u32(record, e.global::<u32>(GROUP_DATA_VTABLE_PTR));
        e.mem.set_u32(record + 8, form_id);
        e.mem.set_u32(record + 0xc, kind);
        record
    }

    /// A form object with a type, whose slot-0 virtual reports `cell`.
    fn form_in_cell(e: &mut Engine, kind: u8, cell: u32) -> u32 {
        e.register_double(0x00ff_00b0, move |_, _| cell.into_ret());
        let form = object_with_vtable(e, 0x100, &[(0, 0x00ff_00b0)]);
        e.mem.set_u8(form + 4, kind);
        form
    }

    #[test]
    fn saves_before_defers_to_the_cell_for_forms_that_are_not_references() {
        let (mut e, refr, cell) = group_engine();
        returns(&mut e, 0x0055_48a0, 0);
        let other = form_in_cell(&mut e, 0x10, 0);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0055_e250, &args![refr, other]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x00ff_00a1), vec![vec![cell, other]]);
    }

    #[test]
    fn saves_before_compares_cells_then_persistence() {
        let (mut e, refr, cell) = group_engine();
        returns(&mut e, 0x0055_48a0, 1);
        // another cell: the cell decides, given that cell
        let elsewhere = e.mem.alloc(0x10);
        let other = form_in_cell(&mut e, 0x3a, elsewhere);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0055_e250, &args![refr, other]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x00ff_00a1), vec![vec![cell, elsewhere]]);

        // the same cell, a reference form: before only when this persists and it does not
        let kinds_and_persistence = [
            (0x3au8, true, false, true),
            (0x3a, true, true, false),
            (0x3a, false, false, false),
            (0x69, true, false, true),
            (0x42, true, false, true),
            (0x43, false, false, false),
            (0x41, true, false, false),
        ];
        for (kind, mine, theirs, expected) in kinds_and_persistence {
            let other = form_in_cell(&mut e, kind, cell);
            let me = refr.addr();
            let other_addr = other;
            e.register_double(GET_REF_PERSISTS, move |_, a| {
                (if a[0] == me { mine } else { theirs }).into_ret()
            });
            let _ = other_addr;
            assert_eq!(
                e.call(0x0055_e250, &args![refr, other]).bool(),
                expected,
                "kind {kind:#x} mine {mine} theirs {theirs}"
            );
        }
    }

    #[test]
    fn saves_before_for_a_group_record_only_orders_this_references_cell_group() {
        let (mut e, refr, cell) = group_engine();
        let me = refr.addr();
        let persists = std::rc::Rc::new(std::cell::Cell::new(true));
        let p = persists.clone();
        e.register_double(GET_REF_PERSISTS, move |_, a| {
            (a[0] == me && p.get()).into_ret()
        });
        e.register_double(0x0048_39c0, move |_, a| {
            (if a[0] == 0x0007_0001 { cell } else { 0 }).into_ret()
        });
        // null or a record of another class
        assert!(!e.call(0x0055_e3f0, &args![refr, 0u32]).bool());
        let alien = e.mem.alloc(0x14);
        assert!(!e.call(0x0055_e3f0, &args![refr, alien]).bool());
        // kind 9 for its own cell while persisting
        let record = group_record(&mut e, 9, 0x0007_0001);
        assert!(e.call(0x0055_e3f0, &args![refr, record]).bool());
        // kind 8, a different cell, or not persisting: no
        let kind8 = group_record(&mut e, 8, 0x0007_0001);
        assert!(!e.call(0x0055_e3f0, &args![refr, kind8]).bool());
        let other_cell = group_record(&mut e, 9, 0x0009_0009);
        assert!(!e.call(0x0055_e3f0, &args![refr, other_cell]).bool());
        persists.set(false);
        assert!(!e.call(0x0055_e3f0, &args![refr, record]).bool());
        // another kind goes to the cell's virtual at +0x38
        let other_kind = group_record(&mut e, 3, 5);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0055_e3f0, &args![refr, other_kind]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x00ff_00a0), vec![vec![cell, other_kind]]);
    }

    #[test]
    fn belongs_in_group_applies_the_kind_rules() {
        let (mut e, refr, cell) = group_engine();
        let me = refr.addr();
        let persists = std::rc::Rc::new(std::cell::Cell::new(true));
        let p = persists.clone();
        e.register_double(GET_REF_PERSISTS, move |_, a| {
            (a[0] == me && p.get()).into_ret()
        });
        // 00485be0(cell, id): the group's form is in this cell
        e.register(0x0048_5be0, |_, a| (a[1] == 0x0007_0001).into_ret());

        assert!(!e.call(0x0055_e4d0, &args![refr, 0u32, 1u32, 1u32]).bool());
        let alien = e.mem.alloc(0x14);
        assert!(!e.call(0x0055_e4d0, &args![refr, alien, 1u32, 1u32]).bool());

        // kind 6 needs the first flag; then the cell test; with the second flag clear or kind 6: true
        let kind6 = group_record(&mut e, 6, 0x0007_0001);
        assert!(!e.call(0x0055_e4d0, &args![refr, kind6, 0u32, 0u32]).bool());
        assert!(e.call(0x0055_e4d0, &args![refr, kind6, 1u32, 0u32]).bool());
        assert!(e.call(0x0055_e4d0, &args![refr, kind6, 1u32, 1u32]).bool());
        let elsewhere6 = group_record(&mut e, 6, 0x0009_0009);
        assert!(!e
            .call(0x0055_e4d0, &args![refr, elsewhere6, 1u32, 1u32])
            .bool());

        // kinds 8 and 9 with the second flag set depend on persistence
        let kind8 = group_record(&mut e, 8, 0x0007_0001);
        let kind9 = group_record(&mut e, 9, 0x0007_0001);
        assert!(e.call(0x0055_e4d0, &args![refr, kind8, 0u32, 0u32]).bool());
        assert!(e.call(0x0055_e4d0, &args![refr, kind8, 0u32, 1u32]).bool());
        assert!(!e.call(0x0055_e4d0, &args![refr, kind9, 0u32, 1u32]).bool());
        persists.set(false);
        assert!(!e.call(0x0055_e4d0, &args![refr, kind8, 0u32, 1u32]).bool());
        assert!(e.call(0x0055_e4d0, &args![refr, kind9, 0u32, 1u32]).bool());

        // any other kind goes to the cell's virtual +0x110 when the first flag is set
        let other = group_record(&mut e, 3, 5);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0055_e4d0, &args![refr, other, 1u32, 0u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x00ff_00a2), vec![vec![cell, other, 1, 0]]);
        assert!(!e.call(0x0055_e4d0, &args![refr, other, 0u32, 1u32]).bool());
    }

    #[test]
    fn create_group_data_builds_the_record_for_the_parent_group() {
        let (mut e, refr, cell) = group_engine();
        let me = refr.addr();
        let persists = std::rc::Rc::new(std::cell::Cell::new(true));
        let p = persists.clone();
        e.register_double(GET_REF_PERSISTS, move |_, a| {
            (a[0] == me && p.get()).into_ret()
        });
        returns(&mut e, 0x0055_16c0, 0);
        returns(&mut e, 0x0054_4210, 0x00ab_0001);
        let cell_id = e.mem.u32(cell + 0xc);

        // null out record: nothing
        e.call(0x0055_e5e0, &args![refr, 0u32, 0u32]);

        // a parent of kind 3 whose form is the cell's 00544210: becomes kind 6
        let out = e.mem.alloc(0x14);
        for i in 0..5 {
            e.mem.set_u32(out + 4 * i, 0xffff_ffff);
        }
        let parent = group_record(&mut e, 3, 0x00ab_0001);
        e.call(0x0055_e5e0, &args![refr, out, parent]);
        assert_eq!(e.mem.u32(out), 0x0101_c654);
        assert_eq!(e.mem.u32(out + 4), 0);
        assert_eq!(e.mem.u32(out + 8), cell_id);
        assert_eq!(e.mem.u32(out + 0xc), 6);
        assert_eq!(e.mem.u32(out + 0x10), 0);

        // a parent of kind 1 matches through 005516c0 too
        let out = e.mem.alloc(0x14);
        let parent = group_record(&mut e, 1, 0x0099_9999);
        e.call(0x0055_e5e0, &args![refr, out, parent]);
        assert_eq!(e.mem.u32(out), 0);
        returns(&mut e, 0x0055_16c0, 1);
        e.call(0x0055_e5e0, &args![refr, out, parent]);
        assert_eq!(e.mem.u32(out + 0xc), 6);

        // a parent of kind 6 for this cell: kind 8 when persisting, 9 when not
        let parent = group_record(&mut e, 6, cell_id);
        let out = e.mem.alloc(0x14);
        e.call(0x0055_e5e0, &args![refr, out, parent]);
        assert_eq!(e.mem.u32(out + 0xc), 8);
        persists.set(false);
        let out = e.mem.alloc(0x14);
        e.call(0x0055_e5e0, &args![refr, out, parent]);
        assert_eq!(e.mem.u32(out + 0xc), 9);

        // other parents (kind 2, or kind 6 for another cell) leave out[0] cleared
        for (kind, id) in [(2u32, 0x00ab_0001u32), (6, 0x0099_9999)] {
            let out = e.mem.alloc(0x14);
            e.mem.set_u32(out, 0x1234);
            let parent = group_record(&mut e, kind, id);
            e.call(0x0055_e5e0, &args![refr, out, parent]);
            assert_eq!(e.mem.u32(out), 0);
        }
    }

    // ---- 0055e730: GetSaveSize -------------------------------------------------

    fn save_size_engine() -> (Engine, Ptr<TESObjectREFR>) {
        let mut e = engine();
        returns(&mut e, 0x0048_4bf0, 10);
        returns(&mut e, 0x0086_2110, 0);
        returns(&mut e, 0x0041_8520, 0x1c1c);
        returns(&mut e, 0x004d_3960, 20);
        returns(&mut e, 0x0042_2c40, 30);
        returns(&mut e, 0x0055_f880, 40);
        returns(&mut e, 0x0056_0350, 50);
        returns(&mut e, 0x008d_f040, 0x42);
        let switches = e.mem.alloc(8);
        returns(&mut e, 0x0040_8d60, switches);
        stub(&mut e, &[ERROR_LOG, 0x004f_d3e0]);
        let save_load = e.mem.alloc(8);
        e.set_global(GLOBAL_SAVE_LOAD, save_load);
        let refr = standard_refr(&mut e);
        (e, refr)
    }

    #[test]
    fn save_size_adds_up_the_parts_for_the_change_flags() {
        let (mut e, refr) = save_size_engine();
        let size = |e: &mut Engine, flags: u32| e.call(0x0055_e730, &args![refr, flags]).u16();
        assert_eq!(size(&mut e, 0), 10);
        // flag 0x20: the container changes
        assert_eq!(size(&mut e, 0x20), 30);
        // flag 4: 2 + 0560350
        assert_eq!(size(&mut e, 4), 10 + 2 + 50);
        // flag 0x10000000 on a non-actor: 2 + 0055f880
        assert_eq!(size(&mut e, 0x1000_0000), 10 + 2 + 40);
        // flag 0x10 only counts from version 0x43
        assert_eq!(size(&mut e, 0x10), 10);
        returns(&mut e, 0x008d_f040, 0x43);
        assert_eq!(size(&mut e, 0x10), 14);
        // save blocks: +6
        returns(&mut e, 0x0086_2110, 1);
        assert_eq!(size(&mut e, 0), 16);
    }

    #[test]
    fn save_size_of_an_actor_adds_its_extras_and_skips_the_non_actor_part() {
        let (mut e, refr) = save_size_engine();
        e.register(0x00ff_0012, |_, _| true.into_ret());
        e.call_log = Some(vec![]);
        let total = e.call(0x0055_e730, &args![refr, 0x1000_0000u32]).u16();
        let log = e.call_log.take().unwrap();
        assert_eq!(total, 10 + 30);
        assert_eq!(
            calls_to(&log, 0x0042_2c40),
            vec![vec![refr.addr() + 0x44, 0x1000_0000, refr.addr()]]
        );
        assert!(calls_to(&log, 0x0055_f880).is_empty());
    }

    #[test]
    fn save_size_wraps_at_16_bits() {
        let (mut e, refr) = save_size_engine();
        returns(&mut e, 0x0048_4bf0, 0xfffe);
        assert_eq!(
            e.call(0x0055_e730, &args![refr, 4u32]).u16(),
            0xfffeu16.wrapping_add(2 + 50)
        );
    }

    #[test]
    fn save_size_logs_what_it_computed_when_the_debug_switch_is_set() {
        let (mut e, refr) = save_size_engine();
        let switches = e.mem.alloc(8);
        e.mem.set_u8(switches, 1);
        returns(&mut e, 0x0040_8d60, switches);
        // without a world space
        e.call_log = Some(vec![]);
        e.call(0x0055_e730, &args![refr, 4u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, ERROR_LOG),
            vec![vec![SAVE_SIZE_SHORT_FORMAT, 52, SOURCE_LINE, SOURCE_FILE]]
        );
        // with one: its form ID, name, and the dword at +5
        let world = e.mem.alloc(0x20);
        e.mem.set_u32(world, 0x0006_0001);
        e.mem.write(world + 5, &0xcafe_f00du32.to_le_bytes());
        returns(&mut e, 0x004f_d3e0, world);
        e.register(0x00ff_00c0, |_, _| 0x0e01u32.into_ret());
        let form = object_with_vtable(&mut e, 0x40, &[(0x130, 0x00ff_00c0)]);
        returns(&mut e, 0x0048_39c0, form);
        e.call_log = Some(vec![]);
        e.call(0x0055_e730, &args![refr, 4u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, ERROR_LOG),
            vec![vec![
                SAVE_SIZE_FORMAT,
                52,
                0x0006_0001,
                0x0e01,
                0xcafe_f00d,
                SOURCE_LINE,
                SOURCE_FILE
            ]]
        );
    }
}
