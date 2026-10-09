//! `fallout shared/tesobjectcell.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds `TESObjectCELL` (409 functions to translate). This session
//! translated the first 40 open or traced functions by address, `005415b0`
//! to `00544490`: the constructor and destructors, the form-record
//! `Save`/`Load`/`InitItem`/`Copy`/`Compare`/`CreateDuplicateForm` overrides, the
//! group-record helpers (`SavesBefore`, `BelongsInGroup`, `CreateGroupData`),
//! the exterior block keys and the flag setters. The next session continues
//! with the first `open` function after `00544490` (`005444c0`).
//!
//! ## Layout (PC build)
//!
//! The PC `TESForm` part is 0x18 bytes (the `TESFullName` component starts
//! there), 0x10 less than the Xbox one, so every Xbox offset up to `pNavMeshes`
//! is 0x10 lower here, and the fields from `iCriticalQueuedRefCount` on are
//! 0x20 higher (the PC build adds fields between `pNavMeshes` and the lock).
//! The fields below are named after the Xbox PDB field whose place and initial
//! value they have; offsets are the PC ones as read from the constructor.
//!
//! ## Helpers the translations call by address
//!
//! Almost every callee is a one-line accessor of the original source. The
//! ones used here are named by what their bodies do (the exe has no symbol
//! for them): `004077c0`, `00440d80`, `00460340`, `004013e0` test bits of the
//! form flags (`+0x08`); `00425fd0` is "cell is interior"
//! (`cCellFlags & 1`); `005516c0` tests form flag 0x400 (the cell's
//! persistent flag); `008256d0`, `006815c0` and `00726070` are the
//! `BSSimpleList` iterator (end test, item address, next node);
//! `009604f0` returns the list of references (`this + 0xAC`); `00559450`
//! reads a smart-pointer slot. The array calls on the local reference array
//! of `SaveReferences` are the `NiTArray` ones (`005594b0` constructor,
//! `0096ad30` SetSize, `00559490` SetGrowBy, `009938d0` Add, `00600bc0`
//! Compact, `009938b0` element count, `00877a30` element address,
//! `0096ae90` SetAt, `00559460` destructor).
//!
//! `__RTDynamicCast` (`00ec43fb`) is called as the CRT does, with the
//! RTTI type descriptors the exe passes; the descriptors are named after
//! what the code does with the result.
//!
//! x87: the one float computation (`InitItem`, rotation times -1.0) rounds
//! to `f32` once, as the code stores a `float`.
//!
//! Not translated: the C++ exception-unwinding frames (`FS:[0]` chains) of
//! the functions that have them (constructor, destructor, `SaveReferences`,
//! `Load`, `InitItem`, `CreateDuplicateForm`, `fn_00541a60`) and the stack
//! cookie check of `Load`.

#[allow(unused_imports)]
use crate::prelude::*;

/// A four-character chunk tag as the record reader compares it (the bytes
/// in file order, read as a little-endian word).
const fn tag(name: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*name)
}

const DATA: u32 = tag(b"DATA");
const EXTERIOR_COORDINATES: u32 = tag(b"XCLC");
const INTERIOR_LIGHTING: u32 = tag(b"XCLL");
const WATER_HEIGHT: u32 = tag(b"XCLW");
const LIGHTING_TEMPLATE_ID: u32 = tag(b"LTMP");
const INHERITANCE_FLAGS: u32 = tag(b"LNAM");
const WATER_NOISE_TEXTURE: u32 = tag(b"XNAM");
const FULL_NAME: u32 = tag(b"FULL");
const EDITOR_ID: u32 = tag(b"EDID");
const OBJECT_BOUNDS: u32 = tag(b"OBND");
const IMPACT_SWAP_FIRST: u32 = tag(b"IMPF");
const IMPACT_SWAP_SECOND: u32 = tag(b"IMPS");

/// The chunk tags `Load` hands to `ExtraDataList::Load`.
const EXTRA_DATA_TAGS: [u32; 76] = [
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

// Vtables the constructor and destructor store.
const CELL_VTABLE: u32 = 0x0102_e9b4;
const CELL_FULL_NAME_VTABLE: u32 = 0x0102_e9a0;

// Form and cell accessors (see the header).
/// `cFormType` (`+0x04`) of a form.
const FORM_TYPE: u32 = 0x0040_1170;
/// `iFormFlags & 0x4000`.
const FORM_FLAG_4000: u32 = 0x0040_77c0;
/// `iFormFlags & 0x20`.
const FORM_FLAG_20: u32 = 0x0044_0d80;
/// `iFormFlags & 0x2`.
const FORM_FLAG_2: u32 = 0x0046_0340;
/// `iFormFlags & 0x8`.
const FORM_FLAG_8: u32 = 0x0040_13e0;
/// `iFormFlags` (`+0x08`).
const FORM_FLAGS: u32 = 0x0044_ddc0;
/// `iFormID` (`+0x0C`).
const FORM_ID: u32 = 0x0084_e3a0;
/// `TESObjectCELL` "is interior": `cCellFlags & 1`.
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
/// The cell's persistent flag: `iFormFlags & 0x400`.
const CELL_PERSISTENT_FLAG: u32 = 0x0055_16c0;
/// `TESObjectCELL::GetWorldSpace` (Xbox PDB): `pWorldSpace` for an exterior
/// cell, null for an interior one.
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
/// The exterior data (`pCellData`) of an exterior cell, null for an interior.
const CELL_EXTERIOR_DATA: u32 = 0x0054_45d0;
/// The interior data (`pCellData`) of an interior cell, null for an exterior.
const CELL_INTERIOR_DATA: u32 = 0x0054_4600;
/// `TESObjectCELL::CreateCellData` (Xbox PDB).
const CELL_CREATE_CELL_DATA: u32 = 0x0054_4630;
/// `TESObjectCELL::GetDataX` / `GetDataY` (Xbox PDB).
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
/// Reads / writes the lighting template pointer (`+0xD8`).
const CELL_GET_LIGHTING_TEMPLATE: u32 = 0x0055_8b40;
const CELL_SET_LIGHTING_TEMPLATE: u32 = 0x0055_8b60;
/// Reads the lighting-template inheritance flags (`+0xDC`).
const CELL_GET_INHERITANCE_FLAGS: u32 = 0x0049_7280;
/// Writes the cell flags byte (`+0x24`).
const CELL_SET_CELL_FLAGS: u32 = 0x0046_1310;
/// The cell's `ExtraDataList` (`this + 0x28`).
const CELL_EXTRA_DATA_LIST: u32 = 0x0046_10d0;
/// `TESObjectCELL::SetLand` (Xbox PDB).
const CELL_SET_LAND: u32 = 0x0054_70a0;
/// `TESObjectCELL::AddReference` (Xbox PDB): `(reference, 0)`.
const CELL_ADD_REFERENCE: u32 = 0x0054_8230;
/// Sets `fWaterHeight` (argument: the new value).
const CELL_SET_WATER_HEIGHT: u32 = 0x0054_7440;
/// Replaces the cell's `NavMeshArray` (`+0x64`), deleting the old one.
const CELL_SET_NAV_MESHES: u32 = 0x0055_7760;
/// Unidentified helpers the destructor and the loader call.
const CELL_CLEAR_STATE: u32 = 0x0054_5c10;
const CELL_RELEASE_STATE: u32 = 0x0054_cd20;
const CELL_CLEAR_REFERENCES: u32 = 0x0054_5030;
const CELL_ERASE: u32 = 0x0055_10b0;
const CELL_SET_LOADED_MASTER_DATA: u32 = 0x0054_def0;
/// The cell-ref lock (`this + 0x80`): enter (with the name string) and leave.
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
/// `BSSimpleList` constructor, destructor body and clear (on a stack list)
/// and the list-iterator accessors.
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
const LIST_CLEAR: u32 = 0x0047_0470;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_PUSH_FRONT: u32 = 0x005a_e3d0;
const CELL_REFERENCE_LIST: u32 = 0x0096_04f0;
const LIST_IS_END: u32 = 0x0082_56d0;
const LIST_ITEM_ADDRESS: u32 = 0x0068_15c0;
const LIST_NEXT: u32 = 0x0072_6070;
/// Smart-pointer slot reader / constructor / assignment / destructor.
const SLOT_GET: u32 = 0x0055_9450;
const SLOT_CONSTRUCT: u32 = 0x0063_3c90;
const SLOT_ASSIGN: u32 = 0x0066_b0d0;
const SLOT_RELEASE: u32 = 0x0045_cec0;
/// Heap allocation / free through the memory manager.
const ALLOCATE: u32 = 0x0040_1000;
const DEALLOCATE: u32 = 0x0040_1030;
/// `memcpy(dest, source, count)` wrapper, CRT `memcmp` and `strlen`.
const MEMORY_COPY: u32 = 0x0040_1460;
const CRT_MEMCMP: u32 = 0x00ec_4835;
const CRT_STRLEN: u32 = 0x00ec_6130;
/// `__RTDynamicCast(object, 0, from, to, 0)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `printf`-style diagnostic output (a stub that returns 0 in this build).
const DEBUG_PRINT: u32 = 0x005b_5e40;
/// `MessageHandler::IncDisableWarningCount(bool)` (Xbox PDB).
const INC_DISABLE_WARNING_COUNT: u32 = 0x0043_b2b0;

// TESForm record helpers.
const FORM_CONSTRUCT: u32 = 0x0048_3370;
const FORM_DESTRUCT: u32 = 0x0048_3630;
/// Writes `cFormType` (`+0x04`); the constructor passes 0x39.
const FORM_SET_FORM_TYPE: u32 = 0x004f_15a0;
/// Sets (non-zero argument) or clears `iFormFlags` bit 8.
const FORM_SET_FLAG_8: u32 = 0x0048_4ab0;
const FORM_START_FORM: u32 = 0x0048_55a0;
const FORM_CLOSE_FORM: u32 = 0x0048_5680;
const FORM_ADD_CHUNK_BYTE: u32 = 0x0048_58f0;
const FORM_ADD_CHUNK_WORD: u32 = 0x0048_5910;
const FORM_ADD_CHUNK_DATA: u32 = 0x0048_5990;
const FORM_ADD_CHUNK_ARRAY: u32 = 0x0048_56f0;
const FORM_LOAD_FORM: u32 = 0x0048_5110;
const FORM_GET_FILE: u32 = 0x0048_4e60;
const FORM_COPY_ALL_COMPONENTS: u32 = 0x0048_51b0;
const FORM_COMPARE_ALL_COMPONENTS: u32 = 0x0048_5270;
const FORM_COMPARE_FALLBACK: u32 = 0x0048_4020;
const FORM_BELONGS_IN_GROUP_FALLBACK: u32 = 0x0048_4150;
const FORM_BELONGS_IN_GROUP_INTERIOR: u32 = 0x0048_54e0;
const FORM_DUPLICATE: u32 = 0x0048_67a0;
const FORM_SAVE_TO_FILE: u32 = 0x0048_3d20;
const FORM_LOOK_UP: u32 = 0x0048_39c0;
const FORM_ADD_COMPILE_INDEX: u32 = 0x0048_5d50;
const FORM_ID_IN_WORLD_SPACE: u32 = 0x0048_5be0;
/// `TESFullName::Save` / loader, and the full-name accessors.
const FULL_NAME_SAVE: u32 = 0x0048_7010;
const FULL_NAME_LOAD: u32 = 0x0048_7050;
const FULL_NAME_LENGTH: u32 = 0x0048_cee0;
const TEXT_GET_STRING: u32 = 0x0040_8da0;
const FULL_NAME_SET: u32 = 0x0048_9100;
/// `TESTexture` constructor and destructor (the water noise texture).
const TEXTURE_CONSTRUCT: u32 = 0x0048_e270;
const TEXTURE_DESTRUCT: u32 = 0x0048_e2e0;
/// `ExtraDataList` constructor, destructor, `InitItem`, `Save`, `Load`,
/// `CopyList`, `CompareList`, `RemoveNonPersistentCellData`,
/// `GetRegionList`, `SetNorthRotation`, impact-swap data accessors.
const EXTRA_LIST_CONSTRUCT: u32 = 0x0041_0360;
const EXTRA_LIST_DESTRUCT: u32 = 0x0041_03b0;
const EXTRA_LIST_INIT_ITEM: u32 = 0x0041_6be0;
const EXTRA_LIST_SAVE: u32 = 0x0041_2970;
const EXTRA_LIST_LOAD: u32 = 0x0041_44a0;
const EXTRA_LIST_COPY_LIST: u32 = 0x0041_1ec0;
const EXTRA_LIST_COMPARE_LIST: u32 = 0x0041_27e0;
const EXTRA_LIST_REMOVE_NON_PERSISTENT: u32 = 0x0041_20b0;
const EXTRA_LIST_GET_REGION_LIST: u32 = 0x0041_bce0;
const EXTRA_LIST_SET_NORTH_ROTATION: u32 = 0x0042_1a70;
const EXTRA_LIST_GET_IMPACT_SWAP: u32 = 0x0041_c460;
const EXTRA_LIST_SET_IMPACT_SWAP: u32 = 0x0041_c390;
/// Impact swap data: constructor (size 0x15c), `Save`, loader.
const IMPACT_SWAP_CONSTRUCT: u32 = 0x0058_efd0;
const IMPACT_SWAP_SAVE: u32 = 0x0058_f080;
const IMPACT_SWAP_LOAD: u32 = 0x0058_f180;
const REGION_LIST_RELEASE: u32 = 0x004f_6640;
/// Endian swap of the cell data in memory: flag, exterior swap and interior
/// swap.
const ENDIAN_SWAP_ENABLED: u32 = 0x0040_1500;
const SWAP_EXTERIOR_DATA: u32 = 0x0046_2230;
const SWAP_INTERIOR_DATA: u32 = 0x0052_6790;

// TESFile accessors used by Save and Load.
const FILE_IS_MASTER: u32 = 0x0047_1c20;
const FILE_GET_FORM_TYPE: u32 = 0x0047_2660;
const FILE_GET_CHUNK: u32 = 0x0047_26b0;
const FILE_NEXT_CHUNK: u32 = 0x0047_26f0;
const FILE_GET_CHUNK_DATA: u32 = 0x0047_27f0;
const FILE_GET_CHUNK_DATA_SIZED: u32 = 0x0047_2890;
const FILE_ADD_FORM: u32 = 0x0047_2fe0;
const FILE_START_GROUP: u32 = 0x0047_3310;
const FILE_CHUNK_SIZE: u32 = 0x0040_1660;
const FILE_SWAP_ENDIAN: u32 = 0x0040_1680;
const FILE_MASTER_DATA: u32 = 0x0046_7bb0;

// NavMeshArray (`+0x64`) and its smart-pointer elements.
const NAV_MESH_ARRAY_CONSTRUCT: u32 = 0x0046_94e0;
const NAV_MESH_ARRAY_COUNT: u32 = 0x0062_0b80;
const NAV_MESH_ARRAY_GET: u32 = 0x0046_4f60;
const NAV_MESH_ARRAY_ADD: u32 = 0x0046_9500;
/// `NiPointer<NavMesh>` helpers: constructor from a raw pointer, default
/// constructor, assignment, copy constructor, destructor, raw pointer read.
const NAV_POINTER_FROM_RAW: u32 = 0x0046_4fc0;
const NAV_POINTER_CONSTRUCT: u32 = 0x0042_fb00;
const NAV_POINTER_ASSIGN: u32 = 0x0042_f4c0;
const NAV_POINTER_COPY: u32 = 0x0042_fa20;
const NAV_POINTER_RELEASE: u32 = 0x0042_fa40;
const NAV_POINTER_GET: u32 = 0x0045_8b50;
/// Returns the `NavMeshArray` pointer `this + 0x64` (`pNavMeshes`).
const CELL_NAV_MESHES: u32 = 0x0070_ec90;

// Save / load of the whole game (InitItem's tail).
const GAME_LOADER_FLAG_SETTER: u32 = 0x0046_23f0;
const GAME_LOAD_FORM: u32 = 0x0084_95d0;
const GAME_LOAD_NOTIFY: u32 = 0x0085_8730;
const GAME_FINISH_A: u32 = 0x0085_9690;
const GAME_FINISH_B: u32 = 0x0085_8af0;
const GAME_FINISH_C: u32 = 0x0085_f850;
const GAME_FLAG_READ: u32 = 0x0047_c850;
const GAME_FLAG_TEST: u32 = 0x0087_27b0;
const GAME_FLAG_WRITE: u32 = 0x0045_34f0;
const REFERENCE_PARENT_CELL: u32 = 0x008d_6f30;
const SAVE_FORM_BUFFER_GET_FORM: u32 = 0x007a_f430;
const REFERENCE_ROTATION: u32 = 0x0043_0830;
/// The thread's error count (`TLS + 0x2B8`): getter and setter.
const ERROR_COUNT_GET: u32 = 0x0046_e8a0;
const ERROR_COUNT_SET: u32 = 0x004f_ffe0;
/// `TESObjectREFR::GetRefPersists` / `SetRefPersists(bool)` (Xbox PDB).
const REFERENCE_GET_PERSISTS: u32 = 0x0056_53d0;
const REFERENCE_SET_PERSISTS: u32 = 0x0056_5480;
/// `TESObjectLAND::SetCell` (Xbox PDB).
const LAND_SET_CELL: u32 = 0x0053_40e0;
/// `TESWorldSpace::ReleaseCell` (Xbox PDB).
const WORLD_SPACE_RELEASE_CELL: u32 = 0x0058_7760;
/// Packs two 16-bit block coordinates into one key.
const PACK_BLOCK_COORDINATES: u32 = 0x0058_7410;

/// `TESFullName` constructor and destructor (the component at `this + 0x18`).
const CELL_FULL_NAME_CONSTRUCT: u32 = 0x0040_2d00;
const CELL_FULL_NAME_DESTRUCT: u32 = 0x0059_1300;
/// Byte at `+0x61D` of the loader singleton.
const LOADER_STATE_FLAG_61D: u32 = 0x0042_26e0;
/// `NavMeshArray` destructor body: clears the elements, then the base.
const NAV_MESH_ARRAY_CLEAR: u32 = 0x0069_bca0;
const NAV_MESH_ARRAY_BASE_DESTRUCT: u32 = 0x0042_f830;
/// `NiTArray` operations on the local array of `SaveReferences`.
const NI_ARRAY_CONSTRUCT: u32 = 0x0055_94b0;
const NI_ARRAY_SET_SIZE: u32 = 0x0096_ad30;
const NI_ARRAY_SET_GROW_BY: u32 = 0x0055_9490;
const NI_ARRAY_ADD: u32 = 0x0099_38d0;
const NI_ARRAY_COMPACT: u32 = 0x0060_0bc0;
const NI_ARRAY_COUNT: u32 = 0x0099_38b0;
const NI_ARRAY_ELEMENT_ADDRESS: u32 = 0x0087_7a30;
const NI_ARRAY_SET_AT: u32 = 0x0096_ae90;
const NI_ARRAY_DESTRUCT: u32 = 0x0055_9460;
/// Tests cell flag 0x20 (`cCellFlags`).
const CELL_FLAG_20: u32 = 0x0050_2180;

// Statics.
/// `TESObjectCELL` statics `InitStatics` and `fn_00541c00` use.
const STATIC_SETTINGS_SOURCE: u32 = 0x011f_426c;
const STATIC_FLAG: u32 = 0x011c_a08c;
const STATIC_POINTER_SLOT: u32 = 0x011c_a0d8;
const STATIC_OBJECT: u32 = 0x011c_a088;
const STATIC_SETTING_BLOCK: u32 = 0x011c_a094;
const STATIC_RESET_WORD: u32 = 0x011c_c54c;
const STATIC_SETTING_BUILDER: u32 = 0x0055_4010;
const STATIC_SETTING_GET: u32 = 0x0040_3e20;
const STATIC_SETTING_SET: u32 = 0x004e_d780;
const STATIC_RELEASE_HELPER: u32 = 0x0062_42f0;
/// `2048.0` as a `double` and as a `float`.
const SETTING_LIMIT_DOUBLE: u32 = 0x0101_6968;
const SETTING_LIMIT_FLOAT: u32 = 0x0101_8bfc;
/// `FLT_MAX` (the default water height).
const DEFAULT_WATER_HEIGHT: u32 = 0x0101_6970;
/// `-1.0` as a `double` (the north rotation factor).
const MINUS_ONE: u32 = 0x0101_a6b0;
/// `"TESObjectCELL::CellRefLockEnter()"`.
const LOCK_NAME: u32 = 0x0102_eaec;

// Globals read by the record code.
/// The `GRUP` tag word of group headers, and the form ids of the persistent
/// world / interior groups.
const GROUP_TAG: u32 = 0x0118_7020;
const EXTERIOR_PARENT_LABEL: u32 = 0x0118_7314;
const INTERIOR_PARENT_LABEL: u32 = 0x0118_72b4;
const PLAYER_HELPER_FORM: u32 = 0x011c_a254;
/// Singleton pointers: the object `004226e0` / `00542d20` read, the game
/// loader (`0046 23f0`, `008495d0`) and the save-game object.
const LOADER_STATE_POINTER: u32 = 0x011c_3f2c;
const GAME_LOADER_POINTER: u32 = 0x011d_df38;
const SAVE_GAME_POINTER: u32 = 0x011d_e45c;

// Messages.
const CELL_NAME_TOO_LONG: u32 = 0x0102_ece8;
const INIT_ERRORS_EXTERIOR_WORLD: u32 = 0x0102_ec58;
const INIT_ERRORS_EXTERIOR_UNKNOWN_WORLD: u32 = 0x0102_ebd0;
const INIT_ERRORS_INTERIOR: u32 = 0x0102_eb60;
const LIGHTING_TEMPLATE_MISSING: u32 = 0x0102_eb10;

// RTTI type descriptors passed to `__RTDynamicCast`.
/// The source type of every cast (`TESForm`).
const RTTI_TES_FORM: u32 = 0x0118_3028;
/// Cast target: `TESObjectCELL` (cells cast to themselves in `Copy`).
const RTTI_CELL: u32 = 0x0118_3fb4;
/// Cast target: `TESObjectLAND` (the duplicated land).
const RTTI_LAND: u32 = 0x0118_ac10;
/// Cast target: the nav mesh class whose `+0x24` holds its cell.
const RTTI_NAV_MESH: u32 = 0x0118_b6b4;
/// Cast target: `TESObjectREFR`.
const RTTI_REFERENCE: u32 = 0x0118_41cc;
/// Cast target for a lighting template.
const RTTI_LIGHTING_TEMPLATE: u32 = 0x0118_a8d0;
/// Cast target of the "world" group labels (`TESWorldSpace`).
const RTTI_WORLD_SPACE: u32 = 0x0118_3fd0;
/// Cast target in `fn_00543230`: an object whose virtual slot 0 gives the
/// cell it belongs to.
const RTTI_CELL_OWNED: u32 = 0x0118_ac2c;

layout! {
    /// `TESObjectCELL` (Xbox PDB), 0xC0 bytes on the Xbox, 0xE0 on the PC.
    /// The vtable is at +0x00; `TESFullName` (a second vtable at +0x18,
    /// `cFullName` at +0x1C) and the `ExtraDataList` at +0x28 are other
    /// classes.
    pub struct TESObjectCELL: 0xe0 {
        /// `iFormFlags` (`TESForm`, Xbox PDB).
        0x08 iFormFlags: u32,
        /// `iFormID` (`TESForm`, Xbox PDB).
        0x0C iFormID: u32,
        /// `cCellFlags` (Xbox PDB): bit 0 interior, 0x20 and 0x40 set by
        /// the two setters, 0x80 and 0x01 tested by `Load`.
        0x24 cCellFlags: u8,
        /// `cCellGameFlags` (Xbox PDB).
        0x25 cCellGameFlags: u8,
        /// `cCellState` (Xbox PDB).
        0x26 cCellState: u8,
        /// `pCellData` (Xbox PDB): the exterior or the interior data.
        0x48 pCellData: Ptr,
        /// `pCellLand` (Xbox PDB): `TESObjectLAND*`.
        0x4C pCellLand: Ptr,
        /// `fWaterHeight` (Xbox PDB).
        0x50 fWaterHeight: f32,
        /// `bAutoWaterLoaded` (Xbox PDB).
        0x54 bAutoWaterLoaded: bool,
        /// `pNavMeshes` (Xbox PDB): `NavMeshArray*`.
        0x64 pNavMeshes: Ptr,
        /// `iCriticalQueuedRefCount` (Xbox PDB).
        0xA0 iCriticalQueuedRefCount: i32,
        /// `iQueuedRefCount` (Xbox PDB).
        0xA4 iQueuedRefCount: i32,
        /// `sNumRefsWithVisibleDistant` (Xbox PDB).
        0xA8 sNumRefsWithVisibleDistant: i16,
        /// `sNumLoadedRefsWithVisibleDistant` (Xbox PDB).
        0xAA sNumLoadedRefsWithVisibleDistant: i16,
        /// `spLightMarkerNode` (Xbox PDB): `NiPointer<NiNode>` slot.
        0xB4 spLightMarkerNode: Ptr,
        /// `spSoundMarkerNode` (Xbox PDB).
        0xB8 spSoundMarkerNode: Ptr,
        /// `spMultiBoundNode` (Xbox PDB).
        0xBC spMultiBoundNode: Ptr,
        /// `pWorldSpace` (Xbox PDB, with `iTempDataOffset`).
        0xC0 pWorldSpace: Ptr,
        /// `pLoadedData` (Xbox PDB).
        0xC4 pLoadedData: Ptr,
        /// `fLodFadeInPercent` (Xbox PDB).
        0xC8 fLodFadeInPercent: f32,
        /// `bLODFadingIn` (Xbox PDB).
        0xCC bLODFadingIn: bool,
        /// `bFadedIn` (Xbox PDB).
        0xCD bFadedIn: bool,
        /// `bFadingToHighDetail` (Xbox PDB).
        0xCE bFadingToHighDetail: bool,
        /// `bFadingToLowDetail` (Xbox PDB).
        0xCF bFadingToLowDetail: bool,
        /// `bDisplayHighDetail` (Xbox PDB).
        0xD0 bDisplayHighDetail: bool,
        /// `bCellDetached` (Xbox PDB).
        0xD1 bCellDetached: bool,
        /// `bUpdateTerrain` (Xbox PDB).
        0xD2 bUpdateTerrain: bool,
        /// `spPortalGraph` (Xbox PDB): `NiPointer<BSPortalGraph>` slot.
        0xD4 spPortalGraph: Ptr,
        /// `pLightingTemplate` (Xbox PDB).
        0xD8 pLightingTemplate: Ptr,
        /// `iLightingTemplateInheritanceFlags` (Xbox PDB).
        0xDC iLightingTemplateInheritanceFlags: u32,
    }
}

/// Offsets of the sub-objects inside the cell that are other classes.
const FULL_NAME_OFFSET: u32 = 0x18;
const EXTRA_DATA_OFFSET: u32 = 0x28;
const WATER_NOISE_TEXTURE_OFFSET: u32 = 0x58;
const CELL_LOCK_OFFSET: u32 = 0x80;
const REFERENCE_LIST_OFFSET: u32 = 0xac;

/// `__RTDynamicCast` of `object` from `TESForm` to `target`.
fn dynamic_cast(e: &mut Engine, object: Ptr, target: u32) -> Ptr {
    e.call(
        RT_DYNAMIC_CAST,
        &args![object, 0u32, RTTI_TES_FORM, target, 0u32],
    )
    .ptr()
}

fn is_interior(e: &mut Engine, cell: Ptr) -> bool {
    e.call(CELL_IS_INTERIOR, &args![cell]).bool()
}

fn persistent_flag(e: &mut Engine, cell: Ptr) -> bool {
    e.call(CELL_PERSISTENT_FLAG, &args![cell]).bool()
}

fn flag_4000(e: &mut Engine, form: Ptr) -> bool {
    e.call(FORM_FLAG_4000, &args![form]).bool()
}

fn flag_20(e: &mut Engine, form: Ptr) -> bool {
    e.call(FORM_FLAG_20, &args![form]).bool()
}

fn flag_2(e: &mut Engine, form: Ptr) -> bool {
    e.call(FORM_FLAG_2, &args![form]).bool()
}

fn form_id(e: &mut Engine, form: Ptr) -> u32 {
    e.call(FORM_ID, &args![form]).u32()
}

fn world_space(e: &mut Engine, cell: Ptr) -> Ptr {
    e.call(CELL_GET_WORLD_SPACE, &args![cell]).ptr()
}

fn slot_get(e: &mut Engine, slot: Ptr) -> Ptr {
    e.call(SLOT_GET, &args![slot]).ptr()
}

fn slot_assign(e: &mut Engine, slot: Ptr, value: Ptr) {
    e.call(SLOT_ASSIGN, &args![slot, value]);
}

/// The cell's list of references (`this + 0xAC`), through its accessor.
fn reference_list(e: &mut Engine, cell: Ptr) -> Ptr {
    e.call(CELL_REFERENCE_LIST, &args![cell]).ptr()
}

/// One step of the `BSSimpleList` walk: false at the end of the list.
fn list_is_end(e: &mut Engine, node: Ptr) -> bool {
    node.is_null() || e.call(LIST_IS_END, &args![node]).bool()
}

/// The item of a list node (the word at the address the accessor returns).
fn list_item(e: &mut Engine, node: Ptr) -> Ptr {
    let address = e.call(LIST_ITEM_ADDRESS, &args![node]).u32();
    Ptr::new(e.mem.u32(address))
}

fn list_next(e: &mut Engine, node: Ptr) -> Ptr {
    e.call(LIST_NEXT, &args![node]).ptr()
}

/// A group header as the group helpers read and write it: `[0]` the `GRUP`
/// tag, `[1]` zero, `[2]` the label, `[3]` the group type, `[4]` zero.
fn write_group_header(e: &mut Engine, header: Ptr, group_type: u32, label: u32) {
    let tag = e.global::<u32>(GROUP_TAG);
    let at = header.addr();
    e.mem.set_u32(at, tag);
    e.mem.set_u32(at + 0xc, group_type);
    e.mem.set_u32(at + 8, label);
    e.mem.set_u32(at + 4, 0);
    e.mem.set_u32(at + 0x10, 0);
}

// Translated from 005415b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::TESObjectCELL` (Xbox PDB): builds the base form, the
/// full-name component, the extra data list, the water noise texture, the
/// lock and reference list, the four smart-pointer slots, and sets the
/// default field values (form type 0x39, water height `FLT_MAX`, lighting
/// template inheritance flags 0x9F, form flag 8). Returns `this`.
pub fn tes_object_cell_tes_object_cell(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
) -> Ptr<TESObjectCELL> {
    e.call(FORM_CONSTRUCT, &args![this]);
    e.call(
        CELL_FULL_NAME_CONSTRUCT,
        &args![this.byte_add(FULL_NAME_OFFSET)],
    );
    e.mem.set_u32(this.addr(), CELL_VTABLE);
    e.mem
        .set_u32(this.addr() + FULL_NAME_OFFSET, CELL_FULL_NAME_VTABLE);
    e.call(
        EXTRA_LIST_CONSTRUCT,
        &args![this.byte_add(EXTRA_DATA_OFFSET)],
    );
    e.call(
        TEXTURE_CONSTRUCT,
        &args![this.byte_add(WATER_NOISE_TEXTURE_OFFSET)],
    );
    e.call(LIST_CONSTRUCT, &args![this.byte_add(CELL_LOCK_OFFSET)]);
    e.call(LIST_CONSTRUCT, &args![this.byte_add(REFERENCE_LIST_OFFSET)]);
    for slot in [
        TESObjectCELL::spLightMarkerNode,
        TESObjectCELL::spSoundMarkerNode,
        TESObjectCELL::spMultiBoundNode,
        TESObjectCELL::spPortalGraph,
    ] {
        e.call(SLOT_CONSTRUCT, &args![this.byte_add(slot.off), 0u32]);
    }
    e.set(this, TESObjectCELL::cCellFlags, 0);
    e.set(this, TESObjectCELL::cCellState, 0);
    e.set(this, TESObjectCELL::iLightingTemplateInheritanceFlags, 0x9f);
    e.set(this, TESObjectCELL::pLightingTemplate, Ptr::NULL);
    e.set(this, TESObjectCELL::pCellData, Ptr::NULL);
    e.set(this, TESObjectCELL::pCellLand, Ptr::NULL);
    let water_height: f32 = e.global(DEFAULT_WATER_HEIGHT);
    e.set(this, TESObjectCELL::fWaterHeight, water_height);
    e.set(this, TESObjectCELL::pNavMeshes, Ptr::NULL);
    e.call(FORM_SET_FORM_TYPE, &args![this, 0x39u32]);
    e.set(this, TESObjectCELL::pWorldSpace, Ptr::NULL);
    e.set(this, TESObjectCELL::cCellGameFlags, 0);
    e.set(this, TESObjectCELL::pLoadedData, Ptr::NULL);
    e.set(this, TESObjectCELL::iQueuedRefCount, 0);
    e.set(this, TESObjectCELL::iCriticalQueuedRefCount, 0);
    e.set(this, TESObjectCELL::sNumRefsWithVisibleDistant, 0);
    e.set(this, TESObjectCELL::sNumLoadedRefsWithVisibleDistant, 0);
    e.set(this, TESObjectCELL::fLodFadeInPercent, 0.0);
    e.set(this, TESObjectCELL::bLODFadingIn, false);
    e.set(this, TESObjectCELL::bFadedIn, false);
    e.set(this, TESObjectCELL::bAutoWaterLoaded, false);
    e.set(this, TESObjectCELL::bFadingToLowDetail, false);
    e.set(this, TESObjectCELL::bFadingToHighDetail, false);
    e.set(this, TESObjectCELL::bDisplayHighDetail, false);
    e.set(this, TESObjectCELL::bCellDetached, false);
    e.set(this, TESObjectCELL::bUpdateTerrain, false);
    e.call(FORM_SET_FLAG_8, &args![this, 1u32]);
    slot_assign(
        e,
        this.byte_add(TESObjectCELL::spPortalGraph.off),
        Ptr::NULL,
    );
    // The water noise texture's virtual slot 0 (`InitializeDataComponent`).
    e.vcall(this.addr() + WATER_NOISE_TEXTURE_OFFSET, 0, &[]);
    this
}

// Translated from 005417e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's scalar deleting destructor (virtual slot 0x10): runs
/// `fn_00541810`, then frees the cell when bit 0 of `flags` is set.
/// Returns `this`.
pub fn fn_005417e0(e: &mut Engine, this: Ptr<TESObjectCELL>, flags: u32) -> Ptr<TESObjectCELL> {
    fn_00541810(e, this);
    if flags & 1 != 0 {
        e.call(DEALLOCATE, &args![this]);
    }
    this
}

// Translated from 00541810 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::~TESObjectCELL` body. Resets the vtables, then, for a
/// form without flag 0x4000, releases the cell's state, deletes its land and
/// its nav mesh array and releases the region list of its extra data; unless
/// the loader singleton says otherwise it releases the cell from its world
/// space; clears the references, unregisters the cell and destroys the
/// members in reverse order of construction.
pub fn fn_00541810(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.mem.set_u32(this.addr(), CELL_VTABLE);
    e.mem
        .set_u32(this.addr() + FULL_NAME_OFFSET, CELL_FULL_NAME_VTABLE);
    e.call(CELL_CLEAR_STATE, &args![this]);
    if !flag_4000(e, this.cast()) {
        e.call(CELL_RELEASE_STATE, &args![this]);
        let land = e.get(this, TESObjectCELL::pCellLand);
        if !land.is_null() {
            // Virtual slot 0x10 with the delete flag: the scalar deleting
            // destructor.
            e.vcall(land.addr(), 0x10, &args![1u32]);
            e.set(this, TESObjectCELL::pCellLand, Ptr::NULL);
        }
        let nav_meshes = e.get(this, TESObjectCELL::pNavMeshes);
        if !nav_meshes.is_null() {
            fn_00541a30(e, nav_meshes, 1);
            e.set(this, TESObjectCELL::pNavMeshes, Ptr::NULL);
        }
        let region_list: Ptr = e
            .call(
                EXTRA_LIST_GET_REGION_LIST,
                &args![this.byte_add(EXTRA_DATA_OFFSET)],
            )
            .ptr();
        if !region_list.is_null() {
            e.call(REGION_LIST_RELEASE, &args![region_list]);
            e.vcall(region_list.addr(), 0, &args![1u32]);
        }
    }
    let loader = e.global::<u32>(LOADER_STATE_POINTER);
    if !e.call(LOADER_STATE_FLAG_61D, &args![loader]).bool() {
        let world = world_space(e, this.cast());
        if !world.is_null() {
            e.call(WORLD_SPACE_RELEASE_CELL, &args![world, this]);
        }
    }
    e.call(CELL_CLEAR_REFERENCES, &args![this]);
    e.call(CELL_ERASE, &args![this]);
    fn_00541b00(e, this);
    // Members are destroyed in the reverse order of the constructor.
    for slot in [
        TESObjectCELL::spPortalGraph,
        TESObjectCELL::spMultiBoundNode,
        TESObjectCELL::spSoundMarkerNode,
        TESObjectCELL::spLightMarkerNode,
    ] {
        e.call(SLOT_RELEASE, &args![this.byte_add(slot.off)]);
    }
    e.call(LIST_DESTRUCT, &args![this.byte_add(REFERENCE_LIST_OFFSET)]);
    e.call(
        TEXTURE_DESTRUCT,
        &args![this.byte_add(WATER_NOISE_TEXTURE_OFFSET)],
    );
    e.call(
        EXTRA_LIST_DESTRUCT,
        &args![this.byte_add(EXTRA_DATA_OFFSET)],
    );
    e.call(
        CELL_FULL_NAME_DESTRUCT,
        &args![this.byte_add(FULL_NAME_OFFSET)],
    );
    e.call(FORM_DESTRUCT, &args![this]);
}

// Translated from 00541a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of the cell's `NavMeshArray`: runs
/// `fn_00541a60`, then frees the array when bit 0 of `flags` is set.
/// Returns `this`.
pub fn fn_00541a30(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00541a60(e, this);
    if flags & 1 != 0 {
        e.call(DEALLOCATE, &args![this]);
    }
    this
}

// Translated from 00541a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor body of the `NavMeshArray`: `0069bca0`, then `0042f830`
/// (which restores the base vtable and runs the base destructor).
pub fn fn_00541a60(e: &mut Engine, this: Ptr) {
    e.call(NAV_MESH_ARRAY_CLEAR, &args![this]);
    e.call(NAV_MESH_ARRAY_BASE_DESTRUCT, &args![this]);
}

// Translated from 00541ac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CellRefLockEnter`: enters the cell's reference lock (`this + 0x80`),
/// passing the name `"TESObjectCELL::CellRefLockEnter()"`.
pub fn fn_00541ac0(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.call(
        LOCK_ENTER,
        &args![this.byte_add(CELL_LOCK_OFFSET), LOCK_NAME],
    );
}

// Translated from 00541ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Leaves the cell's reference lock (`this + 0x80`).
pub fn fn_00541ae0(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.call(LOCK_LEAVE, &args![this.byte_add(CELL_LOCK_OFFSET)]);
}

// Translated from 00541b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the cell's data: removes the non-persistent cell data from the
/// extra data list, frees `pCellData` (the code frees the same field in both
/// the interior and the exterior case), clears the lighting template and the
/// inheritance flags.
pub fn fn_00541b00(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.call(
        EXTRA_LIST_REMOVE_NON_PERSISTENT,
        &args![this.byte_add(EXTRA_DATA_OFFSET)],
    );
    // The original tests `IsInterior` and frees `pCellData` in both
    // branches (two differently typed `delete`s).
    let _ = is_interior(e, this.cast());
    let data = e.get(this, TESObjectCELL::pCellData);
    e.call(DEALLOCATE, &args![data]);
    e.set(this, TESObjectCELL::pCellData, Ptr::NULL);
    e.call(CELL_SET_LIGHTING_TEMPLATE, &args![this, 0u32]);
    e.set(this, TESObjectCELL::iLightingTemplateInheritanceFlags, 0);
}

// Translated from 00541b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::InitStatics` (Xbox PDB): resets the static flag, builds
/// the static pointer from the three settings words at `011f426c`, and raises
/// the setting at `011ca094` to 2048.0 when it is above the limit.
pub fn tes_object_cell_init_statics(e: &mut Engine) {
    let words = [
        e.global::<u32>(STATIC_SETTINGS_SOURCE),
        e.global::<u32>(STATIC_SETTINGS_SOURCE + 4),
        e.global::<u32>(STATIC_SETTINGS_SOURCE + 8),
    ];
    e.set_global(STATIC_FLAG, 0u32);
    let built = e.with_stack(12, |e, block| {
        for (i, word) in words.iter().enumerate() {
            e.mem.set_u32(block.addr() + 4 * i as u32, *word);
        }
        e.call(STATIC_SETTING_BUILDER, &args![block]).ptr::<()>()
    });
    e.call(SLOT_ASSIGN, &args![STATIC_POINTER_SLOT, built]);
    let setting = e
        .call(STATIC_SETTING_GET, &args![STATIC_SETTING_BLOCK])
        .u32();
    let value = e.mem.f32(setting);
    let limit: f64 = e.global(SETTING_LIMIT_DOUBLE);
    // Not taken when the value is below, equal to, or unordered with the limit.
    if (value as f64) > limit {
        let new_value: f32 = e.global(SETTING_LIMIT_FLOAT);
        e.call(STATIC_SETTING_SET, &args![STATIC_SETTING_BLOCK, new_value]);
    }
}

// Translated from 00541c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases the statics: when the static object at `011ca088` exists, hands
/// it to its release helper (with the value `fn_00541c80` makes of the
/// static pointer) and deletes it; clears the pointer slot at `011ca0d8` and
/// resets the word at `011cc54c` (`fn_00541cd0`).
pub fn fn_00541c00(e: &mut Engine) {
    let object = e.global::<u32>(STATIC_OBJECT);
    if object != 0 {
        let pointer = slot_get(e, Ptr::new(STATIC_POINTER_SLOT));
        let value = fn_00541c80(e, pointer);
        e.call(
            STATIC_RELEASE_HELPER,
            &args![e.global::<u32>(STATIC_OBJECT), value],
        );
        let object = e.global::<u32>(STATIC_OBJECT);
        if object != 0 {
            // Virtual slot 0 with the delete flag: the scalar deleting
            // destructor.
            e.vcall(object, 0, &args![1u32]);
        }
        e.set_global(STATIC_OBJECT, 0u32);
    }
    slot_assign(e, Ptr::new(STATIC_POINTER_SLOT), Ptr::NULL);
    fn_00541cd0(e);
}

// Translated from 00541c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `fn_00541cb0` of `object`, or 0 for a null object.
pub fn fn_00541c80(e: &mut Engine, object: Ptr) -> u32 {
    if object.is_null() {
        0
    } else {
        fn_00541cb0(e, object)
    }
}

// Translated from 00541cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the object's virtual slot 0x94 and returns its result.
pub fn fn_00541cb0(e: &mut Engine, object: Ptr) -> u32 {
    e.vcall(object.addr(), 0x94, &[]).u32()
}

// Translated from 00541cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `011cc54c`.
pub fn fn_00541cd0(e: &mut Engine) {
    e.set_global(STATIC_RESET_WORD, 0u32);
}

// Translated from 00541ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's form-record `Save` (virtual slot 0x2c). Starts the record,
/// writes the full name, a `DATA` byte (the cell flags), the exterior
/// (`XCLC`, 12 bytes) or interior (`XCLL`, 44 bytes) data (byte-swapped in
/// memory around the write when the swap flag is on; interiors also save
/// their impact swap data), then `LTMP` (the lighting template's form id, or
/// 0), `LNAM`, `XCLW`, the `XNAM` water noise texture name and the extra data
/// list, and closes the record.
pub fn fn_00541ce0(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.call(FORM_START_FORM, &args![this]);
    e.call(FULL_NAME_SAVE, &args![this.byte_add(FULL_NAME_OFFSET)]);
    let flags = e.get(this, TESObjectCELL::cCellFlags);
    e.call(FORM_ADD_CHUNK_BYTE, &args![DATA, flags as u32]);
    if is_interior(e, this.cast()) {
        let data = e.call(CELL_INTERIOR_DATA, &args![this]).ptr::<()>();
        if e.call(ENDIAN_SWAP_ENABLED, &[]).bool() {
            e.call(SWAP_INTERIOR_DATA, &args![data]);
        }
        e.call(
            FORM_ADD_CHUNK_DATA,
            &args![INTERIOR_LIGHTING, data, 0x2cu32],
        );
        if e.call(ENDIAN_SWAP_ENABLED, &[]).bool() {
            e.call(SWAP_INTERIOR_DATA, &args![data]);
        }
        let list = e.call(CELL_EXTRA_DATA_LIST, &args![this]).ptr::<()>();
        let swap = e.call(EXTRA_LIST_GET_IMPACT_SWAP, &args![list]).ptr::<()>();
        if !swap.is_null() {
            e.call(IMPACT_SWAP_SAVE, &args![swap]);
        }
    } else {
        let data = e.call(CELL_EXTERIOR_DATA, &args![this]).ptr::<()>();
        if e.call(ENDIAN_SWAP_ENABLED, &[]).bool() {
            e.call(SWAP_EXTERIOR_DATA, &args![data]);
        }
        e.call(
            FORM_ADD_CHUNK_DATA,
            &args![EXTERIOR_COORDINATES, data, 0xcu32],
        );
        if e.call(ENDIAN_SWAP_ENABLED, &[]).bool() {
            e.call(SWAP_EXTERIOR_DATA, &args![data]);
        }
    }
    let template_id = if e.call(CELL_GET_LIGHTING_TEMPLATE, &args![this]).u32() != 0 {
        let template = e.call(CELL_GET_LIGHTING_TEMPLATE, &args![this]).ptr::<()>();
        form_id(e, template)
    } else {
        0
    };
    e.call(
        FORM_ADD_CHUNK_WORD,
        &args![LIGHTING_TEMPLATE_ID, template_id],
    );
    let inheritance = e.get(this, TESObjectCELL::iLightingTemplateInheritanceFlags);
    e.call(FORM_ADD_CHUNK_WORD, &args![INHERITANCE_FLAGS, inheritance]);
    let height = e.get(this, TESObjectCELL::fWaterHeight);
    e.call(FORM_ADD_CHUNK_WORD, &args![WATER_HEIGHT, height]);
    let texture = this.byte_add(WATER_NOISE_TEXTURE_OFFSET);
    let length = e
        .call(FULL_NAME_LENGTH, &args![texture])
        .u32()
        .wrapping_add(1);
    let name = e.call(TEXT_GET_STRING, &args![texture]).u32();
    e.call(
        FORM_ADD_CHUNK_ARRAY,
        &args![WATER_NOISE_TEXTURE, name, length],
    );
    e.call(EXTRA_LIST_SAVE, &args![this.byte_add(EXTRA_DATA_OFFSET)]);
    e.call(FORM_CLOSE_FORM, &args![this]);
}

// Translated from 00541e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::Save` (Xbox PDB): saves the cell, its group header and its
/// references into `file`; false for a deleted cell or one with nothing to
/// save in this file.
///
/// A cell is saved when `file` is a master, when the cell has form flag 2, or
/// when one of its references (walked under the reference lock) comes from
/// `file`, has no file, or has flag 2. Saving runs the form `Save` (virtual
/// slot 0x2c), adds the form to the file, starts the cell's group (type 6,
/// label = the cell's form id) and, for a master file or a cell that comes
/// from `file`, saves the references.
pub fn tes_object_cell_save(e: &mut Engine, this: Ptr<TESObjectCELL>, file: Ptr) -> bool {
    if e.call(FORM_FLAG_20, &args![this]).bool() {
        return false;
    }
    let mut wanted = false;
    fn_00541ac0(e, this);
    if e.call(FILE_IS_MASTER, &args![file]).bool() || flag_2(e, this.cast()) {
        wanted = true;
    } else {
        let mut node = reference_list(e, this.cast());
        while !list_is_end(e, node) {
            let reference = list_item(e, node);
            node = list_next(e, node);
            let reference_file = e
                .call(FORM_GET_FILE, &args![reference, 0xffff_ffffu32])
                .ptr::<()>();
            if reference_file == file || reference_file.is_null() || flag_2(e, reference) {
                wanted = true;
                break;
            }
        }
    }
    fn_00541ae0(e, this);
    if !wanted {
        return false;
    }
    e.vcall(this.addr(), 0x2c, &[]);
    e.call(FILE_ADD_FORM, &args![file, this]);
    start_cell_group(e, this, file);
    if !e.call(FILE_IS_MASTER, &args![file]).bool()
        || e.call(FORM_GET_FILE, &args![this, 0u32]).ptr::<()>() == file
    {
        tes_object_cell_save_references(e, this, file);
    }
    true
}

/// Starts the cell's group (type 6, label = the cell's form id) in `file`.
fn start_cell_group(e: &mut Engine, this: Ptr<TESObjectCELL>, file: Ptr) {
    e.with_stack(0x14, |e, header| {
        write_group_header(e, header, 6, 0);
        let id = e.call(FORM_ID, &args![this]).u32();
        e.mem.set_u32(header.addr() + 8, id);
        e.call(FILE_START_GROUP, &args![file, header]);
    });
}

// Translated from 00541fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::SaveEdit` (Xbox PDB): saves the cell form (virtual slot
/// 0x2c), adds it to `file`, starts its group and saves all references.
/// Always true.
pub fn tes_object_cell_save_edit(e: &mut Engine, this: Ptr<TESObjectCELL>, file: Ptr) -> bool {
    e.vcall(this.addr(), 0x2c, &[]);
    e.call(FILE_ADD_FORM, &args![file, this]);
    start_cell_group(e, this, file);
    tes_object_cell_save_references(e, this, file);
    true
}

// Translated from 00542040 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::SaveReferences` (Xbox PDB): collects the references to
/// save (skipping those with form flag 0x4000, deleted ones that do not come
/// from a master file, ones from other files without flag 2, and persistent
/// references of an exterior cell that is not itself persistent) into a local
/// `NiTArray`, compacts it, orders it with a selection sort driven by each
/// form's virtual `SavesBefore` (slot 0x3c), and saves each form to `file`.
pub fn tes_object_cell_save_references(e: &mut Engine, this: Ptr<TESObjectCELL>, file: Ptr) {
    // The local `NiTArray<TESObjectREFR *>` is 0x10 bytes.
    e.with_stack(0x10, |e, array| {
        e.call(NI_ARRAY_CONSTRUCT, &args![array, 0u32, 1u32]);
        e.call(NI_ARRAY_SET_SIZE, &args![array, 0x32u32]);
        e.call(NI_ARRAY_SET_GROW_BY, &args![array, 0x32u32]);
        fn_00541ac0(e, this);
        let mut node = reference_list(e, this.cast());
        while !list_is_end(e, node) {
            let reference = list_item(e, node);
            node = list_next(e, node);
            let mut keep = true;
            if flag_4000(e, reference) {
                keep = false;
            }
            if flag_20(e, reference) {
                let reference_file = e.call(FORM_GET_FILE, &args![reference, 0u32]).ptr::<()>();
                if !reference_file.is_null()
                    && !e.call(FILE_IS_MASTER, &args![reference_file]).bool()
                {
                    keep = false;
                }
            }
            let source = e
                .call(FORM_GET_FILE, &args![reference, 0xffff_ffffu32])
                .ptr::<()>();
            if file != source && !flag_2(e, reference) {
                keep = false;
            }
            if !is_interior(e, this.cast())
                && e.call(REFERENCE_GET_PERSISTS, &args![reference]).bool()
                && !persistent_flag(e, this.cast())
            {
                keep = false;
            }
            if keep {
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), reference.addr());
                    e.call(NI_ARRAY_ADD, &args![array, slot]);
                });
            }
        }
        fn_00541ae0(e, this);
        e.call(NI_ARRAY_COMPACT, &args![array]);
        let count = e.call(NI_ARRAY_COUNT, &args![array]).u32() as i32;
        // Selection sort: after pass `i`, element `i` is the one no later
        // element is ordered before.
        let mut i = 0;
        while i < count {
            let mut first = array_element(e, array, i as u32);
            let mut j = i + 1;
            while j < count {
                let second = array_element(e, array, j as u32);
                let before = e.vcall(second.addr(), 0x3c, &args![first]).bool();
                if before {
                    set_array_element(e, array, i as u32, second);
                    set_array_element(e, array, j as u32, first);
                    first = second;
                }
                j += 1;
            }
            i += 1;
        }
        for i in 0..count {
            let form = array_element(e, array, i as u32);
            if !form.is_null() {
                e.call(FORM_SAVE_TO_FILE, &args![form, file]);
            }
        }
        e.call(NI_ARRAY_DESTRUCT, &args![array]);
    });
}

/// Element `index` of the local reference array, through the element-address
/// accessor.
fn array_element(e: &mut Engine, array: Ptr, index: u32) -> Ptr {
    let address = e.call(NI_ARRAY_ELEMENT_ADDRESS, &args![array, index]).u32();
    Ptr::new(e.mem.u32(address))
}

/// `SetAt(index, &value)` on the local reference array.
fn set_array_element(e: &mut Engine, array: Ptr, index: u32, value: Ptr) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value.addr());
        e.call(NI_ARRAY_SET_AT, &args![array, index, slot]);
    });
}

// Translated from 005422b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `std::basic_streambuf<>::~basic_streambuf` as the exe instantiates it for
/// the `NiTArray` destructor: calls `00559460` on `this`.
pub fn fn_005422b0(e: &mut Engine, this: Ptr) {
    e.call(NI_ARRAY_DESTRUCT, &args![this]);
}

// Translated from 005422d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's form-record `Load` (virtual slot 0x20): false unless the
/// record in `file` is a cell (form type 0x39). Loads the base form, then
/// reads chunks until the file has no more:
///
/// - `DATA`: the cell flags (one byte, from a 4-byte chunk if that is its
///   size); bit 2 is set when bit 1 is clear; an interior with bit 0x80 gets
///   the default water height; bit 0x40 is cleared when the loader singleton
///   says so; the cell data is created, and an interior with master data
///   gets it stored.
/// - `XCLC` / `XCLL`: the exterior (12 bytes) / interior (44 bytes) data into
///   the cell's data block, byte-swapped around the read when the file needs
///   it.
/// - `XCLW`, `LNAM`, `LTMP`: the water height, the inheritance flags and the
///   lighting template id.
/// - `EDID` (virtual slot 0x134), `OBND` (slot 0xE0), `FULL` (full name),
///   `XNAM` (water noise texture name), `IMPF`/`IMPS` (the impact swap data,
///   created on first use) and the 76 extra-data tags (`ExtraDataList::Load`).
///
/// Finally clears form flag 8 and returns true.
pub fn fn_005422d0(e: &mut Engine, this: Ptr<TESObjectCELL>, file: Ptr) -> bool {
    if e.call(FILE_GET_FORM_TYPE, &args![file]).u8() != 0x39 {
        return false;
    }
    let mut master_data = 0u32;
    if e.call(FILE_IS_MASTER, &args![file]).bool() {
        master_data = e.call(FILE_MASTER_DATA, &args![file]).u32();
    }
    e.call(FORM_LOAD_FORM, &args![this, file]);
    loop {
        let chunk = e.call(FILE_GET_CHUNK, &args![file]).u32();
        match chunk {
            EDITOR_ID => {
                let size = e.call(FILE_CHUNK_SIZE, &args![file]).u32();
                let buffer = e.mem.alloc(size.max(4));
                e.call(FILE_GET_CHUNK_DATA_SIZED, &args![file, buffer, 0x200u32]);
                e.vcall(this.addr(), 0x134, &args![buffer]);
                e.mem.free(buffer);
            }
            OBJECT_BOUNDS => {
                e.vcall(this.addr(), 0xe0, &args![file]);
            }
            FULL_NAME => {
                let full_name = if this.is_null() {
                    0
                } else {
                    this.addr() + FULL_NAME_OFFSET
                };
                e.call(FULL_NAME_LOAD, &args![full_name, file]);
            }
            DATA => load_cell_flags(e, this, file, master_data),
            IMPACT_SWAP_FIRST | IMPACT_SWAP_SECOND => {
                let list = e.call(CELL_EXTRA_DATA_LIST, &args![this]).ptr::<()>();
                let mut swap = e.call(EXTRA_LIST_GET_IMPACT_SWAP, &args![list]).ptr::<()>();
                if swap.is_null() {
                    let memory = e.call(ALLOCATE, &args![0x15cu32]).ptr::<()>();
                    swap = if memory.is_null() {
                        Ptr::NULL
                    } else {
                        e.call(IMPACT_SWAP_CONSTRUCT, &args![memory]).ptr()
                    };
                    let list = e.call(CELL_EXTRA_DATA_LIST, &args![this]).ptr::<()>();
                    e.call(EXTRA_LIST_SET_IMPACT_SWAP, &args![list, swap]);
                }
                e.call(IMPACT_SWAP_LOAD, &args![swap, file]);
            }
            LIGHTING_TEMPLATE_ID => {
                let mut id = 0u32;
                e.with_stack(4, |e, slot| {
                    e.call(FILE_GET_CHUNK_DATA, &args![file, slot]);
                    id = e.mem.u32(slot.addr());
                });
                e.set(this, TESObjectCELL::pLightingTemplate, Ptr::new(id));
            }
            WATER_NOISE_TEXTURE => {
                let size = e.call(FILE_CHUNK_SIZE, &args![file]).u32();
                if size != 0 {
                    let buffer = e.mem.alloc(size);
                    e.call(FILE_GET_CHUNK_DATA_SIZED, &args![file, buffer, size]);
                    e.call(
                        FULL_NAME_SET,
                        &args![this.byte_add(WATER_NOISE_TEXTURE_OFFSET), buffer],
                    );
                    e.mem.free(buffer);
                }
            }
            INHERITANCE_FLAGS => {
                e.call(
                    FILE_GET_CHUNK_DATA,
                    &args![
                        file,
                        this.byte_add(TESObjectCELL::iLightingTemplateInheritanceFlags.off)
                    ],
                );
            }
            WATER_HEIGHT => {
                e.call(
                    FILE_GET_CHUNK_DATA,
                    &args![file, this.byte_add(TESObjectCELL::fWaterHeight.off)],
                );
            }
            EXTERIOR_COORDINATES => {
                let data = e.call(CELL_EXTERIOR_DATA, &args![this]).ptr::<()>();
                load_cell_data(e, file, data, 0xc, SWAP_EXTERIOR_DATA);
            }
            INTERIOR_LIGHTING => {
                let data = e.call(CELL_INTERIOR_DATA, &args![this]).ptr::<()>();
                load_cell_data(e, file, data, 0x2c, SWAP_INTERIOR_DATA);
            }
            other if EXTRA_DATA_TAGS.contains(&other) => {
                e.call(
                    EXTRA_LIST_LOAD,
                    &args![this.byte_add(EXTRA_DATA_OFFSET), file, this],
                );
            }
            _ => {}
        }
        if !e.call(FILE_NEXT_CHUNK, &args![file]).bool() {
            break;
        }
    }
    e.call(FORM_SET_FLAG_8, &args![this, 0u32]);
    true
}

/// `Load`'s `DATA` chunk (see `fn_005422d0`).
fn load_cell_flags(e: &mut Engine, this: Ptr<TESObjectCELL>, file: Ptr, master_data: u32) {
    if e.call(FILE_CHUNK_SIZE, &args![file]).u32() == 4 {
        let mut word = 0u32;
        e.with_stack(4, |e, slot| {
            e.call(FILE_GET_CHUNK_DATA, &args![file, slot]);
            word = e.mem.u32(slot.addr());
        });
        e.set(this, TESObjectCELL::cCellFlags, word as u8);
    } else {
        e.call(
            FILE_GET_CHUNK_DATA_SIZED,
            &args![file, this.byte_add(TESObjectCELL::cCellFlags.off), 1u32],
        );
    }
    let flags = e.get(this, TESObjectCELL::cCellFlags);
    if flags & 1 == 0 {
        e.set(this, TESObjectCELL::cCellFlags, flags | 2);
    }
    let flags = e.get(this, TESObjectCELL::cCellFlags);
    if flags & 1 != 0 && flags & 0x80 != 0 {
        let height: f32 = e.global(DEFAULT_WATER_HEIGHT);
        e.call(CELL_SET_WATER_HEIGHT, &args![this, height]);
    }
    let loader = e.global::<u32>(LOADER_STATE_POINTER);
    if !fn_00542d20(e, Ptr::new(loader)) {
        let flags = e.get(this, TESObjectCELL::cCellFlags);
        e.set(this, TESObjectCELL::cCellFlags, flags & 0xbf);
    }
    e.call(CELL_CREATE_CELL_DATA, &args![this]);
    if is_interior(e, this.cast()) && master_data != 0 {
        e.call(CELL_SET_LOADED_MASTER_DATA, &args![this, master_data]);
    }
}

/// `Load`'s `XCLC` / `XCLL` chunks: reads `size` bytes into the cell data
/// block, swapping it before the read when the chunk is not exactly `size`
/// bytes and the file needs swapping, and after it when it needs swapping.
fn load_cell_data(e: &mut Engine, file: Ptr, data: Ptr, size: u32, swap: u32) {
    if data.is_null() {
        return;
    }
    if e.call(FILE_SWAP_ENDIAN, &args![file]).bool()
        && e.call(FILE_CHUNK_SIZE, &args![file]).u32() != size
    {
        e.call(swap, &args![data]);
    }
    e.call(FILE_GET_CHUNK_DATA_SIZED, &args![file, data, size]);
    if e.call(FILE_SWAP_ENDIAN, &args![file]).bool() {
        e.call(swap, &args![data]);
    }
}

// Translated from 00542d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `this + 0x619` of the loader singleton (`011c3f2c`).
pub fn fn_00542d20(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x619) != 0
}

// Translated from 00542d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::InitItem` (Xbox PDB) (virtual slot 0x88).
///
/// Unless form flag 8 is set: counts the errors raised meanwhile (the
/// thread's error count is saved, set to 0, and read back afterwards), mutes
/// warnings, initializes the extra data list, clears the full name of an
/// exterior cell, sets flag 8, warns about a full name over 33 characters,
/// reports a non-zero error count with a message that names the cell (and its
/// world space and coordinates for an exterior one), and resolves the
/// lighting template by form id (warning when it is missing).
///
/// Then, with the loader's flag switched off, it initializes every reference
/// (those without flag 8 go through the form `InitItem`, `00858730` and the
/// game loader; the player helper form of an interior sets the north
/// rotation to minus its rotation; a reference that moved to another cell
/// restarts the walk) and every nav mesh, restores the flag, and finishes the
/// save-game object's pending load.
pub fn tes_object_cell_init_item(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    let cell: Ptr = this.cast();
    if !e.call(FORM_FLAG_8, &args![cell]).bool() {
        let saved_errors = e.call(ERROR_COUNT_GET, &[]).u32();
        e.call(INC_DISABLE_WARNING_COUNT, &args![1u32]);
        e.call(ERROR_COUNT_SET, &args![0u32]);
        e.call(
            EXTRA_LIST_INIT_ITEM,
            &args![this.byte_add(EXTRA_DATA_OFFSET), this],
        );
        if !is_interior(e, cell) {
            e.call(FULL_NAME_SET, &args![this.byte_add(FULL_NAME_OFFSET), 0u32]);
        }
        e.call(FORM_SET_FLAG_8, &args![this, 1u32]);
        let full_name = this.byte_add(FULL_NAME_OFFSET);
        if e.call(TEXT_GET_STRING, &args![full_name]).u32() != 0 {
            let text = e.call(TEXT_GET_STRING, &args![full_name]).u32();
            if e.call(CRT_STRLEN, &args![text]).u32() > 0x21 {
                let id = form_id(e, cell);
                let editor_id = e.vcall(this.addr(), 0x130, &[]).u32();
                let text = e.call(TEXT_GET_STRING, &args![full_name]).u32();
                e.call(DEBUG_PRINT, &args![CELL_NAME_TOO_LONG, text, editor_id, id]);
            }
        }
        e.call(INC_DISABLE_WARNING_COUNT, &args![0u32]);
        let errors = e.call(ERROR_COUNT_GET, &[]).u32();
        e.call(ERROR_COUNT_SET, &args![saved_errors]);
        if errors != 0 {
            report_init_errors(e, this);
        }
        resolve_lighting_template(e, this);
    }
    initialize_references(e, this);
}

/// The error message `InitItem` prints for a cell with errors.
fn report_init_errors(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    let cell: Ptr = this.cast();
    let world = world_space(e, cell);
    if !is_interior(e, cell) {
        if !world.is_null() {
            let world_id = form_id(e, world);
            let world_name = e.vcall(world.addr(), 0x130, &[]).u32();
            let y = e.call(CELL_GET_DATA_Y, &args![cell]).u32();
            let x = e.call(CELL_GET_DATA_X, &args![cell]).u32();
            let id = form_id(e, cell);
            let name = e.vcall(this.addr(), 0x130, &[]).u32();
            e.call(
                DEBUG_PRINT,
                &args![
                    INIT_ERRORS_EXTERIOR_WORLD,
                    name,
                    id,
                    x,
                    y,
                    world_name,
                    world_id
                ],
            );
        } else {
            let y = e.call(CELL_GET_DATA_Y, &args![cell]).u32();
            let x = e.call(CELL_GET_DATA_X, &args![cell]).u32();
            let id = form_id(e, cell);
            let name = e.vcall(this.addr(), 0x130, &[]).u32();
            e.call(
                DEBUG_PRINT,
                &args![INIT_ERRORS_EXTERIOR_UNKNOWN_WORLD, name, id, x, y],
            );
        }
    } else {
        let id = form_id(e, cell);
        let name = e.vcall(this.addr(), 0x130, &[]).u32();
        e.call(DEBUG_PRINT, &args![INIT_ERRORS_INTERIOR, name, id]);
    }
}

/// Looks the lighting template up by the id stored in `pLightingTemplate`
/// (made file-relative with `TESForm::AddCompileIndex`) and stores the
/// resolved object; warns when the id does not resolve.
fn resolve_lighting_template(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    let cell: Ptr = this.cast();
    if e.call(CELL_GET_LIGHTING_TEMPLATE, &args![cell]).u32() == 0 {
        return;
    }
    let stored = e.call(CELL_GET_LIGHTING_TEMPLATE, &args![cell]).u32();
    let file = e.call(FORM_GET_FILE, &args![cell, 0xffff_ffffu32]).u32();
    e.with_stack(4, |e, id_slot| {
        e.mem.set_u32(id_slot.addr(), stored);
        e.call(FORM_ADD_COMPILE_INDEX, &args![id_slot, file]);
        let id = e.mem.u32(id_slot.addr());
        let form = e.call(FORM_LOOK_UP, &args![id]).ptr::<()>();
        let template = dynamic_cast(e, form, RTTI_LIGHTING_TEMPLATE);
        e.call(CELL_SET_LIGHTING_TEMPLATE, &args![cell, template]);
        if template.is_null() && id != 0 {
            let cell_id = form_id(e, cell);
            let name = e.vcall(cell.addr(), 0x130, &[]).u32();
            e.call(
                DEBUG_PRINT,
                &args![LIGHTING_TEMPLATE_MISSING, id, name, cell_id],
            );
        }
    });
}

/// The reference and nav mesh walk at the end of `InitItem`.
fn initialize_references(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    let cell: Ptr = this.cast();
    let loader = Ptr::<()>::new(e.global::<u32>(GAME_LOADER_POINTER));
    let save_game = Ptr::<()>::new(e.global::<u32>(SAVE_GAME_POINTER));
    let previous_flag = e.call(GAME_LOADER_FLAG_SETTER, &args![loader, 0u32]).u8();
    let mut node = reference_list(e, cell);
    let mut previous = Ptr::NULL;
    while !node.is_null() && !list_is_end(e, node) {
        let reference = list_item(e, node);
        if !e.call(FORM_FLAG_8, &args![reference]).bool() {
            e.vcall(reference.addr(), 0x88, &[]);
            e.call(GAME_LOAD_NOTIFY, &args![save_game, reference]);
            e.call(GAME_LOAD_FORM, &args![loader, reference]);
        }
        if is_interior(e, cell) {
            let form = e.call(SAVE_FORM_BUFFER_GET_FORM, &args![reference]).u32();
            if form == e.global::<u32>(PLAYER_HELPER_FORM) {
                let rotation = e.call(REFERENCE_ROTATION, &args![reference]).u32();
                let factor: f64 = e.global(MINUS_ONE);
                let north = (e.mem.f32(rotation + 8) as f64 * factor) as f32;
                let list = e.call(CELL_EXTRA_DATA_LIST, &args![cell]).ptr::<()>();
                e.call(EXTRA_LIST_SET_NORTH_ROTATION, &args![list, north]);
            }
        }
        if !persistent_flag(e, cell)
            && e.call(REFERENCE_PARENT_CELL, &args![reference]).ptr::<()>() != cell
        {
            // The reference left this cell: continue after the previous node
            // (or from the head).
            node = if previous.is_null() {
                reference_list(e, cell)
            } else {
                list_next(e, previous)
            };
        } else {
            previous = node;
            node = list_next(e, node);
        }
    }
    if !e.call(CELL_NAV_MESHES, &args![cell]).ptr::<()>().is_null() {
        let mut index = 0u32;
        loop {
            let nav_meshes = e.call(CELL_NAV_MESHES, &args![cell]).ptr::<()>();
            let count = e.call(NAV_MESH_ARRAY_COUNT, &args![nav_meshes]).u32();
            if index >= count {
                break;
            }
            let nav_meshes = e.call(CELL_NAV_MESHES, &args![cell]).ptr::<()>();
            e.with_stack(4, |e, pointer| {
                e.call(NAV_MESH_ARRAY_GET, &args![nav_meshes, pointer, index]);
                let nav_mesh = slot_get(e, pointer);
                e.vcall(nav_mesh.addr(), 0x88, &[]);
                let raw = e.call(NAV_POINTER_GET, &args![pointer]).u32();
                e.call(GAME_LOAD_FORM, &args![loader, raw]);
                e.call(NAV_POINTER_RELEASE, &args![pointer]);
            });
            index += 1;
        }
    }
    e.call(
        GAME_LOADER_FLAG_SETTER,
        &args![loader, previous_flag as u32],
    );
    if e.call(GAME_FLAG_READ, &args![save_game]).bool() {
        e.call(GAME_FINISH_A, &args![save_game, cell]);
        if !e.call(GAME_FLAG_READ, &args![save_game]).bool()
            && e.call(GAME_FLAG_TEST, &args![save_game]).bool()
        {
            let saved = e.call(GAME_FLAG_READ, &args![save_game]).u8();
            e.call(GAME_FLAG_WRITE, &args![save_game, 1u32]);
            e.call(GAME_FINISH_B, &args![save_game, 0u32, 0u32, 0u32]);
            e.call(GAME_FINISH_C, &args![save_game, 0u32]);
            e.call(GAME_FLAG_WRITE, &args![save_game, saved as u32]);
        }
    }
}

// Translated from 00543230 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's `SavesBefore` (virtual slot 0x3c): whether this cell is saved
/// before `other`, by form type.
///
/// - Cell (0x39): an interior cell before an exterior one; otherwise cells
///   of the same kind are ordered by world space (the world space's own
///   `SavesBefore`), persistence, block key, sub-block key and form id.
/// - Types 0x3a to 0x40, 0x42, 0x43, 0x69: the cast object's cell (virtual
///   slot 0) is this cell, or this cell's `SavesBefore` of it.
/// - Type 0x41: always true for an interior cell, else the world space's
///   `SavesBefore(other)`.
/// - Anything else: `00484020`.
pub fn fn_00543230(e: &mut Engine, this: Ptr<TESObjectCELL>, other: Ptr) -> bool {
    let cell: Ptr = this.cast();
    let form_type = e.call(FORM_TYPE, &args![other]).u32();
    match form_type {
        0x39 => {
            let other_cell = dynamic_cast(e, other, RTTI_CELL);
            if other_cell.is_null() {
                return false;
            }
            let this_interior = is_interior(e, cell);
            let other_interior = is_interior(e, other_cell);
            if this_interior && !other_interior {
                return true;
            }
            if this_interior != other_interior {
                return false;
            }
            let mut compare_keys = true;
            let mut result = false;
            if !this_interior {
                let world = world_space(e, cell);
                let other_world = world_space(e, other_cell);
                if world != other_world {
                    result = e.vcall(world.addr(), 0x3c, &args![other_world]).bool();
                    compare_keys = false;
                } else if persistent_flag(e, cell) != persistent_flag(e, other_cell) {
                    result = persistent_flag(e, cell);
                    compare_keys = false;
                }
            }
            if compare_keys {
                let (a, b) = (fn_005441b0(e, this), fn_005441b0(e, other_cell.cast()));
                if a < b {
                    return true;
                }
                if a == b {
                    let (c, d) = (fn_00544210(e, this), fn_00544210(e, other_cell.cast()));
                    if c < d {
                        return true;
                    }
                    if c == d && form_id(e, cell) < form_id(e, other_cell) {
                        return true;
                    }
                }
            }
            result
        }
        0x3a..=0x40 | 0x42 | 0x43 | 0x69 => {
            let owned = dynamic_cast(e, other, RTTI_CELL_OWNED);
            let owner = e.vcall(owned.addr(), 0, &[]).ptr::<()>();
            if owner == cell {
                true
            } else {
                let owner = e.vcall(owned.addr(), 0, &[]).u32();
                e.vcall(this.addr(), 0x3c, &args![owner]).bool()
            }
        }
        0x41 => {
            if !is_interior(e, cell) {
                let world = world_space(e, cell);
                e.vcall(world.addr(), 0x3c, &args![other]).bool()
            } else {
                true
            }
        }
        _ => e.call(FORM_COMPARE_FALLBACK, &args![this, other]).bool(),
    }
}

// Translated from 005434d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's `SavesBefore` for a group header (virtual slot 0x38): whether
/// the cell is saved before `header`'s group, by group type
/// (`header[3]`, label `header[2]`).
///
/// - 0: `00484150`. 1: an interior cell is before it; an exterior cell is
///   when it is before the world space the label names. 2: an interior
///   block key lower than the label. 3: sub-block key lower than the label.
///   4: not before for an interior cell; true for a persistent exterior
///   cell; else block key lower than the label. 5: the same with the
///   sub-block key. 6, 8, 9: before the cell the label names. 7: always.
/// - Not a group header (`header` null, or its tag is not `GRUP`): false.
pub fn fn_005434d0(e: &mut Engine, this: Ptr<TESObjectCELL>, header: Ptr) -> bool {
    let cell: Ptr = this.cast();
    if header.is_null() || e.mem.u32(header.addr()) != e.global::<u32>(GROUP_TAG) {
        return false;
    }
    let label = e.mem.u32(header.addr() + 8);
    match e.mem.u32(header.addr() + 0xc) {
        0 => e
            .call(FORM_BELONGS_IN_GROUP_FALLBACK, &args![this, header])
            .bool(),
        1 => {
            if is_interior(e, cell) {
                true
            } else {
                let form = e.call(FORM_LOOK_UP, &args![label]).ptr::<()>();
                let world = dynamic_cast(e, form, RTTI_WORLD_SPACE);
                !world.is_null() && e.vcall(this.addr(), 0x3c, &args![world]).bool()
            }
        }
        2 => is_interior(e, cell) && fn_005441b0(e, this) < label,
        3 => is_interior(e, cell) && fn_00544210(e, this) < label,
        4 => {
            if is_interior(e, cell) || persistent_flag(e, cell) {
                true
            } else {
                fn_005441b0(e, this) < label
            }
        }
        5 => {
            if is_interior(e, cell) || persistent_flag(e, cell) {
                true
            } else {
                fn_00544210(e, this) < label
            }
        }
        6 | 8 | 9 => {
            let form = e.call(FORM_LOOK_UP, &args![label]).ptr::<()>();
            let other_cell = dynamic_cast(e, form, RTTI_CELL);
            !other_cell.is_null() && e.vcall(this.addr(), 0x3c, &args![other_cell]).bool()
        }
        7 => true,
        _ => false,
    }
}

// Translated from 005436e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::CreateDuplicateForm` (Xbox PDB) (virtual slot 0x40).
/// `flag` is not used; `arg` is passed down to every duplication.
///
/// Null for an interior cell. Otherwise duplicates the form (`004867a0`),
/// casts it to a cell, then duplicates the land (setting it on the new cell
/// and the new cell on it, marking it altered) and the nav mesh array (each
/// non-deleted nav mesh duplicated, pointed at the new cell and added to a
/// new array), and finally duplicates the references that are not flagged
/// 0x4000 and are either in an interior or persistent cell or not persistent
/// themselves, adding each to the new cell (carrying over the cell's
/// persistent flag) and marking it altered. Returns the new cell.
pub fn tes_object_cell_create_duplicate_form(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    flag: u32,
    arg: u32,
) -> Ptr<TESObjectCELL> {
    let _ = flag;
    let cell: Ptr = this.cast();
    if is_interior(e, cell) {
        return Ptr::NULL;
    }
    let copy = e.call(FORM_DUPLICATE, &args![this, 0u32, arg]).ptr::<()>();
    let new_cell = dynamic_cast(e, copy, RTTI_CELL);
    let land = e.get(this, TESObjectCELL::pCellLand);
    if !land.is_null() {
        let duplicate = e.vcall(land.addr(), 0x40, &args![0u32, arg]).ptr::<()>();
        let new_land = dynamic_cast(e, duplicate, RTTI_LAND);
        if !new_land.is_null() {
            e.call(CELL_SET_LAND, &args![new_cell, new_land]);
            e.call(LAND_SET_CELL, &args![new_land, new_cell]);
            e.vcall(new_land.addr(), 0xc8, &args![1u32]);
        }
    }
    let nav_meshes = e.get(this, TESObjectCELL::pNavMeshes);
    if !nav_meshes.is_null() {
        duplicate_nav_meshes(e, this, new_cell, arg);
    }
    fn_00541ac0(e, this);
    e.with_stack(8, |e, list| {
        e.call(LIST_CONSTRUCT, &args![list]);
        let mut node = reference_list(e, cell);
        while !list_is_end(e, node) {
            let reference = list_item(e, node);
            if !flag_4000(e, reference)
                && (is_interior(e, cell)
                    || persistent_flag(e, cell)
                    || !e.call(REFERENCE_GET_PERSISTS, &args![reference]).bool())
            {
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), reference.addr());
                    e.call(LIST_PUSH_FRONT, &args![list, slot]);
                });
            }
            node = list_next(e, node);
        }
        fn_00541ae0(e, this);
        let mut node = list;
        while !list_is_end(e, node) {
            let reference = list_item(e, node);
            let duplicate = e
                .vcall(reference.addr(), 0x40, &args![0u32, arg])
                .ptr::<()>();
            let new_reference = dynamic_cast(e, duplicate, RTTI_REFERENCE);
            if !new_reference.is_null() {
                let persists = persistent_flag(e, new_cell);
                e.call(
                    REFERENCE_SET_PERSISTS,
                    &args![new_reference, persists as u32],
                );
                e.call(CELL_ADD_REFERENCE, &args![new_cell, new_reference, 0u32]);
                e.vcall(new_reference.addr(), 0xc8, &args![1u32]);
            }
            node = list_next(e, node);
        }
        e.call(LIST_CLEAR, &args![list]);
        e.call(LIST_DESTRUCT, &args![list]);
    });
    new_cell.cast()
}

/// The nav mesh part of `CreateDuplicateForm`.
fn duplicate_nav_meshes(e: &mut Engine, this: Ptr<TESObjectCELL>, new_cell: Ptr, arg: u32) {
    let memory = e.call(ALLOCATE, &args![0x10u32]).ptr::<()>();
    let array = if memory.is_null() {
        Ptr::NULL
    } else {
        e.call(NAV_MESH_ARRAY_CONSTRUCT, &args![memory]).ptr::<()>()
    };
    e.call(CELL_SET_NAV_MESHES, &args![new_cell, array]);
    // `NiPointer<NavMesh>` temporaries on the stack.
    let collected = e.mem.alloc(4);
    let element = e.mem.alloc(4);
    let wrapped = e.mem.alloc(4);
    let by_value = e.mem.alloc(4);
    e.call(NAV_POINTER_CONSTRUCT, &args![collected]);
    let nav_meshes = e.get(this, TESObjectCELL::pNavMeshes);
    let mut index = 0u32;
    while index < e.call(NAV_MESH_ARRAY_COUNT, &args![nav_meshes]).u32() {
        let slot = e
            .call(NAV_MESH_ARRAY_GET, &args![nav_meshes, element, index])
            .ptr::<()>();
        let nav_mesh = slot_get(e, slot);
        e.call(NAV_POINTER_RELEASE, &args![element]);
        if !nav_mesh.is_null() && !flag_20(e, nav_mesh) {
            let duplicate = e
                .vcall(nav_mesh.addr(), 0x40, &args![0u32, arg])
                .ptr::<()>();
            let new_nav_mesh = dynamic_cast(e, duplicate, RTTI_NAV_MESH);
            if !new_nav_mesh.is_null() {
                // The duplicate's cell field (`+0x24`).
                e.mem.set_u32(new_nav_mesh.addr() + 0x24, new_cell.addr());
                let pointer = e
                    .call(NAV_POINTER_FROM_RAW, &args![wrapped, new_nav_mesh])
                    .ptr::<()>();
                e.call(NAV_POINTER_ASSIGN, &args![collected, pointer]);
                e.call(NAV_POINTER_RELEASE, &args![wrapped]);
                e.call(NAV_POINTER_COPY, &args![by_value, collected]);
                let copy = e.mem.u32(by_value);
                let new_nav_meshes = e.mem.u32(new_cell.addr() + TESObjectCELL::pNavMeshes.off);
                // The callee takes the smart pointer by value and releases it.
                e.call(NAV_MESH_ARRAY_ADD, &args![new_nav_meshes, copy]);
            }
        }
        index += 1;
    }
    e.call(NAV_POINTER_RELEASE, &args![collected]);
    for block in [collected, element, wrapped, by_value] {
        e.mem.free(block);
    }
}

// Translated from 00543ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::Copy` (Xbox PDB) (virtual slot 0x108): copies `source`
/// (cast to a cell; nothing happens for another type) into this cell: the
/// form components, the extra data list, the cell flags, the water height,
/// the cell data block (12 or 44 bytes, by interior-ness), the lighting
/// template and inheritance flags, and the water noise texture component.
pub fn tes_object_cell_copy(e: &mut Engine, this: Ptr<TESObjectCELL>, source: Ptr) {
    let cell: Ptr = this.cast();
    let source_cell = dynamic_cast(e, source, RTTI_CELL);
    if source_cell.is_null() {
        return;
    }
    e.call(FORM_COPY_ALL_COMPONENTS, &args![this, source_cell]);
    let source_list = e
        .call(CELL_EXTRA_DATA_LIST, &args![source_cell])
        .ptr::<()>();
    let list = e.call(CELL_EXTRA_DATA_LIST, &args![this]).ptr::<()>();
    e.call(EXTRA_LIST_COPY_LIST, &args![list, source_list]);
    // The original frees `pCellData` in both branches of an `IsInterior` test.
    let _ = is_interior(e, cell);
    let data = e.get(this, TESObjectCELL::pCellData);
    e.call(DEALLOCATE, &args![data]);
    e.set(this, TESObjectCELL::pCellData, Ptr::NULL);
    let flags = fn_00543c30(e, source_cell.cast());
    e.call(CELL_SET_CELL_FLAGS, &args![this, flags as u32]);
    let height = e
        .mem
        .f32(source_cell.addr() + TESObjectCELL::fWaterHeight.off);
    e.set(this, TESObjectCELL::fWaterHeight, height);
    e.call(CELL_CREATE_CELL_DATA, &args![this]);
    if is_interior(e, cell) {
        let to = e.call(CELL_INTERIOR_DATA, &args![this]).u32();
        let from = e.call(CELL_INTERIOR_DATA, &args![source_cell]).u32();
        if to != 0 && from != 0 {
            e.call(MEMORY_COPY, &args![to, from, 0x2cu32]);
        }
    } else {
        let to = e.call(CELL_EXTERIOR_DATA, &args![this]).u32();
        let from = e.call(CELL_EXTERIOR_DATA, &args![source_cell]).u32();
        if to != 0 && from != 0 {
            e.call(MEMORY_COPY, &args![to, from, 0xcu32]);
        }
    }
    let template = e
        .call(CELL_GET_LIGHTING_TEMPLATE, &args![source_cell])
        .u32();
    e.call(CELL_SET_LIGHTING_TEMPLATE, &args![this, template]);
    let inheritance = e
        .call(CELL_GET_INHERITANCE_FLAGS, &args![source_cell])
        .u32();
    e.set(
        this,
        TESObjectCELL::iLightingTemplateInheritanceFlags,
        inheritance,
    );
    // The water noise texture's virtual `CopyComponent` (slot 8).
    e.vcall(
        this.addr() + WATER_NOISE_TEXTURE_OFFSET,
        8,
        &args![source_cell.addr() + WATER_NOISE_TEXTURE_OFFSET],
    );
}

// Translated from 00543c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell flags byte (`cCellFlags`).
pub fn fn_00543c30(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u8 {
    e.get(this, TESObjectCELL::cCellFlags)
}

// Translated from 00543c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's `Compare` (virtual slot 0x10c): true when `other` differs.
/// True for a non-cell, for different form components, cell flags (compared
/// as signed bytes), inheritance flags, lighting templates or water heights
/// (a NaN counts as different), different cell data blocks (when both exist),
/// different extra data lists, or different water noise texture components.
pub fn fn_00543c50(e: &mut Engine, this: Ptr<TESObjectCELL>, other: Ptr) -> bool {
    let cell: Ptr = this.cast();
    let other_cell = dynamic_cast(e, other, RTTI_CELL);
    if other_cell.is_null() {
        return true;
    }
    if e.call(FORM_COMPARE_ALL_COMPONENTS, &args![this, other_cell])
        .bool()
    {
        return true;
    }
    if fn_00543c30(e, this) as i8 != fn_00543c30(e, other_cell.cast()) as i8 {
        return true;
    }
    let other_inheritance = e.call(CELL_GET_INHERITANCE_FLAGS, &args![other_cell]).u32();
    if e.get(this, TESObjectCELL::iLightingTemplateInheritanceFlags) != other_inheritance {
        return true;
    }
    if e.call(CELL_GET_LIGHTING_TEMPLATE, &args![this]).u32()
        != e.call(CELL_GET_LIGHTING_TEMPLATE, &args![other_cell]).u32()
    {
        return true;
    }
    let this_height = e.get(this, TESObjectCELL::fWaterHeight);
    let other_height = e
        .mem
        .f32(other_cell.addr() + TESObjectCELL::fWaterHeight.off);
    if this_height != other_height {
        return true;
    }
    let (accessor, size) = if is_interior(e, cell) {
        (CELL_INTERIOR_DATA, 0x2cu32)
    } else {
        (CELL_EXTERIOR_DATA, 0xcu32)
    };
    let mine = e.call(accessor, &args![this]).u32();
    let theirs = e.call(accessor, &args![other_cell]).u32();
    if mine != 0 && theirs != 0 {
        let order = e.call(CRT_MEMCMP, &args![mine, theirs, size]).u32();
        if order != 0 {
            return true;
        }
    }
    let other_list = e.call(CELL_EXTRA_DATA_LIST, &args![other_cell]).ptr::<()>();
    let list = e.call(CELL_EXTRA_DATA_LIST, &args![this]).ptr::<()>();
    if e.call(EXTRA_LIST_COMPARE_LIST, &args![list, other_list])
        .bool()
    {
        return true;
    }
    e.vcall(
        this.addr() + WATER_NOISE_TEXTURE_OFFSET,
        0xc,
        &args![other_cell.addr() + WATER_NOISE_TEXTURE_OFFSET],
    )
    .bool()
}

// Translated from 00543df0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's `BelongsInGroup` (virtual slot 0x110): whether the cell is
/// stored in the group `header` describes. `with_children` and `strict` are
/// the two byte arguments the code receives.
///
/// By group type (`header[3]`, label `header[2]`): 0 (top-level): for
/// `with_children`, `004854e0` (interior) or the world space's `BelongsInGroup`
/// (exterior); 1 (world children): an exterior cell whose world space
/// contains the label, when `with_children` or the cell is persistent;
/// 2 and 4: not persistent, `with_children`, and the label equals the block
/// key; 3 and 5: not persistent and the label equals the sub-block key.
/// False when `header` is null or not a group header.
pub fn fn_00543df0(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    header: Ptr,
    with_children: u8,
    strict: u8,
) -> bool {
    let cell: Ptr = this.cast();
    if header.is_null() || e.mem.u32(header.addr()) != e.global::<u32>(GROUP_TAG) {
        return false;
    }
    let world = world_space(e, cell);
    let label = e.mem.u32(header.addr() + 8);
    match e.mem.u32(header.addr() + 0xc) {
        0 => {
            if with_children != 0 {
                if is_interior(e, cell) {
                    return e
                        .call(
                            FORM_BELONGS_IN_GROUP_INTERIOR,
                            &args![this, header, with_children as u32, strict as u32],
                        )
                        .bool();
                }
                return e
                    .vcall(
                        world.addr(),
                        0x110,
                        &args![header, with_children as u32, strict as u32],
                    )
                    .bool();
            }
            false
        }
        1 => {
            !is_interior(e, cell)
                && e.call(FORM_ID_IN_WORLD_SPACE, &args![world, label]).bool()
                && (with_children != 0 || persistent_flag(e, cell))
        }
        2 | 4 => !persistent_flag(e, cell) && with_children != 0 && label == fn_005441b0(e, this),
        3 | 5 => !persistent_flag(e, cell) && label == fn_00544210(e, this),
        _ => false,
    }
}

// Translated from 00543f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's `CreateGroupData` (virtual slot 0x114): writes into `out` the
/// header of the group a cell goes into under the group `parent` describes
/// (or the top-level one when `parent` is null for an interior cell); `out`
/// is left with only its first word cleared when no group applies.
///
/// Exterior: parent type 0 with the exterior label gives type 1 (label: the
/// world space's form id); parent type 1 naming this cell's world space gives
/// type 4 (block key) and parent type 4 with this cell's block key gives
/// type 5 (sub-block key), both for non-persistent cells only. Interior:
/// no parent gives type 0 (label `011872b4`); parent type 0 with that label
/// gives type 2 (block key); parent type 2 with this cell's block key gives
/// type 3 (sub-block key).
pub fn fn_00543f50(e: &mut Engine, this: Ptr<TESObjectCELL>, out: Ptr, parent: Ptr) {
    let cell: Ptr = this.cast();
    if out.is_null() {
        return;
    }
    e.mem.set_u32(out.addr(), 0);
    if !is_interior(e, cell) {
        if parent.is_null() {
            return;
        }
        let label = e.mem.u32(parent.addr() + 8);
        match e.mem.u32(parent.addr() + 0xc) {
            0 => {
                if label == e.global::<u32>(EXTERIOR_PARENT_LABEL) {
                    let world = world_space(e, cell);
                    let id = form_id(e, world);
                    write_group_header(e, out, 1, id);
                }
            }
            1 => {
                let world = world_space(e, cell);
                if label == form_id(e, world) && !persistent_flag(e, cell) {
                    let key = fn_005441b0(e, this);
                    write_group_header(e, out, 4, key);
                }
            }
            4 if label == fn_005441b0(e, this) && !persistent_flag(e, cell) => {
                let key = fn_00544210(e, this);
                write_group_header(e, out, 5, key);
            }
            _ => {}
        }
    } else if parent.is_null() {
        let label = e.global::<u32>(INTERIOR_PARENT_LABEL);
        write_group_header(e, out, 0, label);
    } else {
        let label = e.mem.u32(parent.addr() + 8);
        match e.mem.u32(parent.addr() + 0xc) {
            0 if label == e.global::<u32>(INTERIOR_PARENT_LABEL) => {
                let key = fn_005441b0(e, this);
                write_group_header(e, out, 2, key);
            }
            2 if label == fn_005441b0(e, this) => {
                let key = fn_00544210(e, this);
                write_group_header(e, out, 3, key);
            }
            _ => {}
        }
    }
}

// Translated from 005441b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's block key: for an exterior cell
/// `CalcExtGroupBlockKey(x, y)`; for an interior one the last decimal digit
/// of the form id's low 24 bits.
pub fn fn_005441b0(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u32 {
    let cell: Ptr = this.cast();
    if !is_interior(e, cell) {
        let y = e.call(CELL_GET_DATA_Y, &args![cell]).i32();
        let x = e.call(CELL_GET_DATA_X, &args![cell]).i32();
        tes_object_cell_calc_ext_group_block_key(e, x, y)
    } else {
        (form_id(e, cell) & 0x00ff_ffff) % 10
    }
}

// Translated from 00544210 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell's sub-block key: for an exterior cell
/// `CalcExtGroupSubBlockKey(x, y)`; for an interior one the second-to-last
/// decimal digit of the form id's low 24 bits.
pub fn fn_00544210(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u32 {
    let cell: Ptr = this.cast();
    if !is_interior(e, cell) {
        let y = e.call(CELL_GET_DATA_Y, &args![cell]).i32();
        let x = e.call(CELL_GET_DATA_X, &args![cell]).i32();
        tes_object_cell_calc_ext_group_sub_block_key(e, x, y)
    } else {
        ((form_id(e, cell) & 0x00ff_ffff) % 100) / 10
    }
}

// Translated from 00544280 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::CalcExtGroupBlockKey` (Xbox PDB): packs the block
/// coordinates `x >> 5` and `y >> 5` (arithmetic shifts, 16 bits each).
pub fn tes_object_cell_calc_ext_group_block_key(e: &mut Engine, x: i32, y: i32) -> u32 {
    let (block_x, block_y) = ((x >> 5) as u16, (y >> 5) as u16);
    e.call(
        PACK_BLOCK_COORDINATES,
        &args![block_x as u32, block_y as u32],
    )
    .u32()
}

// Translated from 005442c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::CalcExtGroupSubBlockKey` (Xbox PDB): packs `x >> 3` and
/// `y >> 3` (arithmetic shifts, 16 bits each).
pub fn tes_object_cell_calc_ext_group_sub_block_key(e: &mut Engine, x: i32, y: i32) -> u32 {
    let (block_x, block_y) = ((x >> 3) as u16, (y >> 3) as u16);
    e.call(
        PACK_BLOCK_COORDINATES,
        &args![block_x as u32, block_y as u32],
    )
    .u32()
}

// Translated from 00544300 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::SetInterior` (Xbox PDB): sets or clears bit 0 of the cell
/// flags.
pub fn tes_object_cell_set_interior(e: &mut Engine, this: Ptr<TESObjectCELL>, interior: bool) {
    let flags = e.get(this, TESObjectCELL::cCellFlags);
    let flags = if interior { flags | 1 } else { flags & !1 };
    e.set(this, TESObjectCELL::cCellFlags, flags);
}

// Translated from 00544340 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 0x20 of the cell flags, then calls the form's virtual
/// slot 0x48 with 2 (`AddChange`).
pub fn fn_00544340(e: &mut Engine, this: Ptr<TESObjectCELL>, set: bool) {
    let flags = e.get(this, TESObjectCELL::cCellFlags);
    let flags = if set { flags | 0x20 } else { flags & !0x20 };
    e.set(this, TESObjectCELL::cCellFlags, flags);
    e.vcall(this.addr(), 0x48, &args![2u32]);
}

// Translated from 00544390 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::SetTempPublic` (Xbox PDB): sets or clears bit 0x40 of the
/// cell flags, then calls the form's virtual slot 0x48 with 2.
pub fn tes_object_cell_set_temp_public(e: &mut Engine, this: Ptr<TESObjectCELL>, set: bool) {
    let flags = e.get(this, TESObjectCELL::cCellFlags);
    let flags = if set { flags | 0x40 } else { flags & !0x40 };
    e.set(this, TESObjectCELL::cCellFlags, flags);
    e.vcall(this.addr(), 0x48, &args![2u32]);
}

// Translated from 005443e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether cell flag 0x20 (`00502180`) or cell flag 0x40 (`fn_00544420`) is
/// set.
pub fn fn_005443e0(e: &mut Engine, this: Ptr<TESObjectCELL>) -> bool {
    e.call(CELL_FLAG_20, &args![this]).bool() || fn_00544420(e, this)
}

// Translated from 00544420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Cell flag 0x40 (`SetTempPublic`'s bit).
pub fn fn_00544420(e: &mut Engine, this: Ptr<TESObjectCELL>) -> bool {
    e.get(this, TESObjectCELL::cCellFlags) & 0x40 != 0
}

// Translated from 00544440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 1 of `cCellGameFlags`, then calls the form's virtual
/// slot 0x48 with 2.
pub fn fn_00544440(e: &mut Engine, this: Ptr<TESObjectCELL>, set: bool) {
    let flags = e.get(this, TESObjectCELL::cCellGameFlags);
    let flags = if set { flags | 1 } else { flags & !1 };
    e.set(this, TESObjectCELL::cCellGameFlags, flags);
    e.vcall(this.addr(), 0x48, &args![2u32]);
}

// Translated from 00544490 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether form flag 0x20000 is set (`iFormFlags`, through `0044ddc0`).
pub fn fn_00544490(e: &mut Engine, this: Ptr) -> bool {
    e.call(FORM_FLAGS, &args![this]).u32() & 0x20000 != 0
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x005415b0,
            tes_object_cell_tes_object_cell(Ptr<TESObjectCELL>) -> Ptr<TESObjectCELL>
        ),
        entry!(
            0x005417e0,
            fn_005417e0(Ptr<TESObjectCELL>, u32) -> Ptr<TESObjectCELL>
        ),
        entry!(0x00541810, fn_00541810(Ptr<TESObjectCELL>)),
        entry!(0x00541a30, fn_00541a30(Ptr, u32) -> Ptr),
        entry!(0x00541a60, fn_00541a60(Ptr)),
        entry!(0x00541ac0, fn_00541ac0(Ptr<TESObjectCELL>)),
        entry!(0x00541ae0, fn_00541ae0(Ptr<TESObjectCELL>)),
        entry!(0x00541b00, fn_00541b00(Ptr<TESObjectCELL>)),
        entry!(0x00541b80, tes_object_cell_init_statics()),
        entry!(0x00541c00, fn_00541c00()),
        entry!(0x00541c80, fn_00541c80(Ptr) -> u32),
        entry!(0x00541cb0, fn_00541cb0(Ptr) -> u32),
        entry!(0x00541cd0, fn_00541cd0()),
        entry!(0x00541ce0, fn_00541ce0(Ptr<TESObjectCELL>)),
        entry!(
            0x00541e80,
            tes_object_cell_save(Ptr<TESObjectCELL>, Ptr) -> bool
        ),
        entry!(
            0x00541fd0,
            tes_object_cell_save_edit(Ptr<TESObjectCELL>, Ptr) -> bool
        ),
        entry!(
            0x00542040,
            tes_object_cell_save_references(Ptr<TESObjectCELL>, Ptr)
        ),
        entry!(0x005422b0, fn_005422b0(Ptr)),
        entry!(0x005422d0, fn_005422d0(Ptr<TESObjectCELL>, Ptr) -> bool),
        entry!(0x00542d20, fn_00542d20(Ptr) -> bool),
        entry!(0x00542d40, tes_object_cell_init_item(Ptr<TESObjectCELL>)),
        entry!(0x00543230, fn_00543230(Ptr<TESObjectCELL>, Ptr) -> bool),
        entry!(0x005434d0, fn_005434d0(Ptr<TESObjectCELL>, Ptr) -> bool),
        entry!(
            0x005436e0,
            tes_object_cell_create_duplicate_form(
                Ptr<TESObjectCELL>,
                u32,
                u32,
            ) -> Ptr<TESObjectCELL>
        ),
        entry!(0x00543ab0, tes_object_cell_copy(Ptr<TESObjectCELL>, Ptr)),
        entry!(0x00543c30, fn_00543c30(Ptr<TESObjectCELL>) -> u8),
        entry!(0x00543c50, fn_00543c50(Ptr<TESObjectCELL>, Ptr) -> bool),
        entry!(
            0x00543df0,
            fn_00543df0(Ptr<TESObjectCELL>, Ptr, u8, u8) -> bool
        ),
        entry!(0x00543f50, fn_00543f50(Ptr<TESObjectCELL>, Ptr, Ptr)),
        entry!(0x005441b0, fn_005441b0(Ptr<TESObjectCELL>) -> u32),
        entry!(0x00544210, fn_00544210(Ptr<TESObjectCELL>) -> u32),
        entry!(
            0x00544280,
            tes_object_cell_calc_ext_group_block_key(i32, i32) -> u32
        ),
        entry!(
            0x005442c0,
            tes_object_cell_calc_ext_group_sub_block_key(i32, i32) -> u32
        ),
        entry!(
            0x00544300,
            tes_object_cell_set_interior(Ptr<TESObjectCELL>, bool)
        ),
        entry!(0x00544340, fn_00544340(Ptr<TESObjectCELL>, bool)),
        entry!(
            0x00544390,
            tes_object_cell_set_temp_public(Ptr<TESObjectCELL>, bool)
        ),
        entry!(0x005443e0, fn_005443e0(Ptr<TESObjectCELL>) -> bool),
        entry!(0x00544420, fn_00544420(Ptr<TESObjectCELL>) -> bool),
        entry!(0x00544440, fn_00544440(Ptr<TESObjectCELL>, bool)),
        entry!(0x00544490, fn_00544490(Ptr) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    /// The vtable of every test object. Slot `n` leads to the double at
    /// `fake(n)`; the doubles read their answers from words at the end of
    /// the object (`KEY` and friends), so one object type serves as a cell,
    /// a reference, a world space, a land or a nav mesh.
    const TEST_VTABLE: u32 = 0x0130_0000;
    const KEY: u32 = 0xe8;
    const EDITOR_ID_WORD: u32 = 0xec;
    const DUPLICATE: u32 = 0xf0;
    const SLOT_ZERO_RESULT: u32 = 0xf4;
    const BELONGS: u32 = 0xf8;
    const SLOT_94_RESULT: u32 = 0xfc;

    fn fake(slot: u32) -> u32 {
        0x7000_0000 + slot
    }

    fn calls_to(log: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The addresses called, in order, without the call under test.
    fn sequence(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        log.iter().skip(1).map(|(a, _)| *a).collect()
    }

    /// Registers do-nothing doubles returning 0.
    fn quiet(e: &mut Engine, addresses: &[u32]) {
        for address in addresses {
            e.register(*address, |_, _| Ret::default());
        }
    }

    /// A double that returns `value`.
    fn returns(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| value.into_ret());
    }

    /// An engine with the pages the cell code reads globals from, the test
    /// vtable and doubles for the accessors that only read a field, behaving
    /// as the game's do.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_6000,
            0x0101_8000,
            0x0101_a000,
            0x0118_7000,
            0x011c_3000,
            0x011c_a000,
            0x011c_c000,
            0x011d_d000,
            0x011d_e000,
            0x011f_4000,
        ] {
            e.map(page, 0x1000);
        }
        let slots: Vec<u32> = (0..=0x134 / 4).map(|i| fake(4 * i)).collect();
        e.put_vtable(TEST_VTABLE, &slots);
        // Virtual methods: those that answer read the object's own words.
        quiet(
            &mut e,
            &[
                fake(0x10),
                fake(0x20),
                fake(0x2c),
                fake(0x48),
                fake(0x88),
                fake(0xc8),
                fake(0xe0),
                fake(0x134),
            ],
        );
        e.register(fake(0x3c), |e, a| {
            u32::from(e.mem.u32(a[0] + KEY) < e.mem.u32(a[1] + KEY)).into_ret()
        });
        e.register(fake(0x40), |e, a| e.mem.u32(a[0] + DUPLICATE).into_ret());
        e.register(fake(0), |e, a| {
            e.mem.u32(a[0] + SLOT_ZERO_RESULT).into_ret()
        });
        e.register(fake(0x94), |e, a| {
            e.mem.u32(a[0] + SLOT_94_RESULT).into_ret()
        });
        e.register(fake(0x110), |e, a| e.mem.u32(a[0] + BELONGS).into_ret());
        e.register(fake(0x130), |e, a| {
            e.mem.u32(a[0] + EDITOR_ID_WORD).into_ret()
        });
        // Field readers.
        e.register(FORM_TYPE, |e, a| (e.mem.u8(a[0] + 4) as u32).into_ret());
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        e.register(FORM_FLAGS, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(FORM_FLAG_4000, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x4000 != 0).into_ret()
        });
        e.register(FORM_FLAG_20, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x20 != 0).into_ret()
        });
        e.register(FORM_FLAG_2, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x2 != 0).into_ret()
        });
        e.register(FORM_FLAG_8, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x8 != 0).into_ret()
        });
        e.register(CELL_IS_INTERIOR, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x24) & 1 != 0).into_ret()
        });
        e.register(CELL_PERSISTENT_FLAG, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x400 != 0).into_ret()
        });
        e.register(CELL_GET_WORLD_SPACE, |e, a| {
            if e.mem.u8(a[0] + 0x24) & 1 != 0 {
                0u32.into_ret()
            } else {
                e.mem.u32(a[0] + 0xc0).into_ret()
            }
        });
        e.register(CELL_EXTERIOR_DATA, |e, a| {
            if e.mem.u8(a[0] + 0x24) & 1 != 0 {
                0u32.into_ret()
            } else {
                e.mem.u32(a[0] + 0x48).into_ret()
            }
        });
        e.register(CELL_INTERIOR_DATA, |e, a| {
            if e.mem.u8(a[0] + 0x24) & 1 != 0 {
                e.mem.u32(a[0] + 0x48).into_ret()
            } else {
                0u32.into_ret()
            }
        });
        e.register(CELL_GET_DATA_X, |e, a| {
            let data = e.mem.u32(a[0] + 0x48);
            if data == 0 {
                0u32.into_ret()
            } else {
                e.mem.u32(data).into_ret()
            }
        });
        e.register(CELL_GET_DATA_Y, |e, a| {
            let data = e.mem.u32(a[0] + 0x48);
            if data == 0 {
                0u32.into_ret()
            } else {
                e.mem.u32(data + 4).into_ret()
            }
        });
        e.register(CELL_GET_LIGHTING_TEMPLATE, |e, a| {
            e.mem.u32(a[0] + 0xd8).into_ret()
        });
        e.register(CELL_SET_LIGHTING_TEMPLATE, |e, a| {
            e.mem.set_u32(a[0] + 0xd8, a[1]);
            Ret::default()
        });
        e.register(CELL_GET_INHERITANCE_FLAGS, |e, a| {
            e.mem.u32(a[0] + 0xdc).into_ret()
        });
        e.register(CELL_EXTRA_DATA_LIST, |_, a| (a[0] + 0x28).into_ret());
        e.register(CELL_SET_CELL_FLAGS, |e, a| {
            e.mem.set_u8(a[0] + 0x24, a[1] as u8);
            Ret::default()
        });
        e.register(SLOT_GET, |e, a| e.mem.u32(a[0]).into_ret());
        for store in [SLOT_ASSIGN, SLOT_CONSTRUCT] {
            e.register(store, |e, a| {
                e.mem.set_u32(a[0], a[1]);
                a[0].into_ret()
            });
        }
        e.register(SLOT_RELEASE, |_, _| Ret::default());
        e.register(ALLOCATE, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(DEALLOCATE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(MEMORY_COPY, |e, a| {
            let bytes = e.mem.bytes(a[1], a[2]);
            e.mem.write(a[0], &bytes);
            a[0].into_ret()
        });
        e.register(CRT_MEMCMP, |e, a| {
            let (x, y) = (e.mem.bytes(a[0], a[2]), e.mem.bytes(a[1], a[2]));
            (x.cmp(&y) as i32).into_ret()
        });
        e.register(CRT_STRLEN, |e, a| {
            (e.mem.cstr(a[0]).len() as u32).into_ret()
        });
        e.register(RT_DYNAMIC_CAST, |_, a| a[0].into_ret());
        // BSSimpleList: a node is { item, next }; the list head is the first
        // node; a list is at its end when node.item and node.next are both 0.
        e.register(CELL_REFERENCE_LIST, |_, a| (a[0] + 0xac).into_ret());
        e.register(LIST_IS_END, |e, a| {
            u32::from(e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0).into_ret()
        });
        e.register(LIST_ITEM_ADDRESS, |_, a| a[0].into_ret());
        e.register(LIST_NEXT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(CELL_PERSISTENT_FLAG, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x400 != 0).into_ret()
        });
        e.register(REFERENCE_GET_PERSISTS, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x800 != 0).into_ret()
        });
        e.register(FORM_GET_FILE, |e, a| {
            // File of the form: +0x80 for the last file (-1), +0x84 for the
            // first (0).
            if a[1] == 0xffff_ffff {
                e.mem.u32(a[0] + 0x80).into_ret()
            } else {
                e.mem.u32(a[0] + 0x84).into_ret()
            }
        });
        e.register(FILE_IS_MASTER, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x10) != 0).into_ret()
        });
        e.register(PACK_BLOCK_COORDINATES, |_, a| {
            ((a[0] as i16 as i32 as u32) << 16 | (a[1] & 0xffff)).into_ret()
        });
        e.set_global(GROUP_TAG, 0x5055_5247u32);
        e
    }

    /// A zeroed 0x100-byte object with the test vtable.
    fn object(e: &mut Engine) -> Ptr {
        let block = e.mem.alloc(0x100);
        e.mem.set_u32(block, TEST_VTABLE);
        Ptr::new(block)
    }

    /// A cell (an object that is also laid out as a cell).
    fn cell(e: &mut Engine, interior: bool) -> Ptr<TESObjectCELL> {
        let cell = object(e);
        e.mem.set_u8(cell.addr() + 0x24, interior as u8);
        cell.cast()
    }

    /// Fills the cell's reference list (`+0xAC`) with `items`: the first node
    /// is the list head, the others are heap nodes.
    fn fill_list(e: &mut Engine, head: u32, items: &[Ptr]) {
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(node, item.addr());
            if i + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
    }

    fn set_references(e: &mut Engine, cell: Ptr<TESObjectCELL>, items: &[Ptr]) {
        fill_list(e, cell.addr() + 0xac, items);
    }

    /// A string in memory.
    fn text(e: &mut Engine, string: &str) -> Ptr {
        let block = e.mem.alloc(string.len() as u32 + 1);
        e.mem.set_cstr(block, string.as_bytes());
        Ptr::new(block)
    }

    // ---- the small functions ---------------------------------------

    #[test]
    fn cell_flag_setters_set_and_clear_their_bits() {
        let mut e = engine();
        let cell = cell(&mut e, false);
        e.call(0x0054_4300, &args![cell, true]);
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0x01);
        e.call(0x0054_4300, &args![cell, false]);
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0x00);

        // The 0x20 and 0x40 setters also call virtual slot 0x48 with 2.
        e.call_log = Some(vec![]);
        e.call(0x0054_4340, &args![cell, true]);
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0x20);
        e.call(0x0054_4390, &args![cell, true]);
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0x60);
        e.call(0x0054_4340, &args![cell, false]);
        e.call(0x0054_4390, &args![cell, false]);
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0x00);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, fake(0x48)), vec![vec![cell.addr(), 2]; 4]);

        // The game-flags setter changes bit 0 of the other byte.
        e.call_log = Some(vec![]);
        e.set(cell, TESObjectCELL::cCellGameFlags, 0xf0);
        e.call(0x0054_4440, &args![cell, true]);
        assert_eq!(e.get(cell, TESObjectCELL::cCellGameFlags), 0xf1);
        e.call(0x0054_4440, &args![cell, false]);
        assert_eq!(e.get(cell, TESObjectCELL::cCellGameFlags), 0xf0);
        assert_eq!(calls_to(&e.call_log.take().unwrap(), fake(0x48)).len(), 2);
    }

    #[test]
    fn cell_flag_readers() {
        let mut e = engine();
        let cell = cell(&mut e, false);
        assert!(!e.call(0x0054_4420, &args![cell]).bool());
        e.set(cell, TESObjectCELL::cCellFlags, 0x40);
        assert!(e.call(0x0054_4420, &args![cell]).bool());
        assert_eq!(e.call(0x0054_3c30, &args![cell]).u8(), 0x40);

        // "Public": cell flag 0x20 (through its accessor) or 0x40.
        e.register(CELL_FLAG_20, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x24) & 0x20 != 0).into_ret()
        });
        assert!(e.call(0x0054_43e0, &args![cell]).bool());
        e.set(cell, TESObjectCELL::cCellFlags, 0x20);
        assert!(e.call(0x0054_43e0, &args![cell]).bool());
        e.set(cell, TESObjectCELL::cCellFlags, 0x01);
        assert!(!e.call(0x0054_43e0, &args![cell]).bool());

        // Form flag 0x20000 through the flags accessor.
        let form = object(&mut e);
        assert!(!e.call(0x0054_4490, &args![form]).bool());
        e.mem.set_u32(form.addr() + 8, 0x0002_0000);
        assert!(e.call(0x0054_4490, &args![form]).bool());
    }

    #[test]
    fn block_keys_shift_arithmetically_and_pack_16_bits() {
        let mut e = engine();
        // (x >> 5, y >> 5) as 16-bit values, x in the high half.
        assert_eq!(
            e.call(0x0054_4280, &args![64i32, 33i32]).u32(),
            (2u32 << 16) | 1
        );
        // Negative coordinates keep their sign through the shift: -1 >> 5 = -1.
        assert_eq!(
            e.call(0x0054_4280, &args![-1i32, -33i32]).u32(),
            0xffff_fffeu32
        );
        // The sub-block key shifts by 3.
        assert_eq!(
            e.call(0x0054_42c0, &args![17i32, 8i32]).u32(),
            (2u32 << 16) | 1
        );
    }

    #[test]
    fn block_and_sub_block_keys_of_a_cell() {
        let mut e = engine();
        // Exterior: from the cell's coordinates (x, y at pCellData).
        let exterior = cell(&mut e, false);
        let data = e.mem.alloc(12);
        e.mem.set_i32(data, 70);
        e.mem.set_i32(data + 4, -40);
        e.mem.set_u32(exterior.addr() + 0x48, data);
        e.call_log = Some(vec![]);
        let block = e.call(0x0054_41b0, &args![exterior]).u32();
        let log = e.call_log.take().unwrap();
        assert_eq!(
            block,
            ((70 >> 5) as u32) << 16 | ((-40i32 >> 5) as u16 as u32)
        );
        // Y is fetched before X, as the code evaluates its arguments.
        assert_eq!(
            sequence(&log)
                .into_iter()
                .filter(|a| *a == CELL_GET_DATA_X || *a == CELL_GET_DATA_Y)
                .collect::<Vec<_>>(),
            vec![CELL_GET_DATA_Y, CELL_GET_DATA_X]
        );
        let sub = e.call(0x0054_4210, &args![exterior]).u32();
        assert_eq!(
            sub,
            ((70 >> 3) as u32) << 16 | ((-40i32 >> 3) as u16 as u32)
        );

        // Interior: digits of the form id's low 24 bits.
        let interior = cell(&mut e, true);
        e.mem.set_u32(interior.addr() + 0xc, 0xff00_1234);
        assert_eq!(e.call(0x0054_41b0, &args![interior]).u32(), 0x1234 % 10);
        assert_eq!(
            e.call(0x0054_4210, &args![interior]).u32(),
            (0x1234 % 100) / 10
        );
    }

    #[test]
    fn noise_counter_reset_words() {
        let mut e = engine();
        e.set_global(STATIC_RESET_WORD, 9u32);
        e.call(0x0054_1cd0, &[]);
        assert_eq!(e.global::<u32>(STATIC_RESET_WORD), 0);
    }

    #[test]
    fn static_object_helpers() {
        let mut e = engine();
        let object = object(&mut e);
        e.mem.set_u32(object.addr() + SLOT_94_RESULT, 0x5151);
        assert_eq!(e.call(0x0054_1cb0, &args![object]).u32(), 0x5151);
        assert_eq!(e.call(0x0054_1c80, &args![object]).u32(), 0x5151);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0054_1c80, &args![Ptr::<()>::NULL]).u32(), 0);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    #[test]
    fn form_type_reader_and_flag_8() {
        let mut e = engine();
        let cell = cell(&mut e, false);
        e.mem.set_u8(cell.addr() + 4, 0x39);
        // 0x542d20 reads the byte at +0x619 of the loader singleton.
        let loader = e.mem.alloc(0x700);
        e.mem.set_u8(loader + 0x619, 1);
        assert!(e.call(0x0054_2d20, &args![Ptr::<()>::new(loader)]).bool());
        e.mem.set_u8(loader + 0x619, 0);
        assert!(!e.call(0x0054_2d20, &args![Ptr::<()>::new(loader)]).bool());
    }

    #[test]
    fn lock_helpers_use_the_lock_at_plus_0x80() {
        let mut e = engine();
        quiet(&mut e, &[LOCK_ENTER, LOCK_LEAVE]);
        let cell = cell(&mut e, false);
        e.call_log = Some(vec![]);
        e.call(0x0054_1ac0, &args![cell]);
        e.call(0x0054_1ae0, &args![cell]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, LOCK_ENTER),
            vec![vec![cell.addr() + 0x80, LOCK_NAME]]
        );
        assert_eq!(calls_to(&log, LOCK_LEAVE), vec![vec![cell.addr() + 0x80]]);
    }

    #[test]
    fn array_destructor_forwarder() {
        let mut e = engine();
        quiet(&mut e, &[NI_ARRAY_DESTRUCT]);
        e.call_log = Some(vec![]);
        e.call(0x0054_22b0, &args![Ptr::<()>::new(0x4000)]);
        assert_eq!(
            calls_to(&e.call_log.take().unwrap(), NI_ARRAY_DESTRUCT),
            vec![vec![0x4000]]
        );
    }

    // ---- constructor, destructors, statics --------------------------

    #[test]
    fn constructor_sets_the_default_state() {
        let mut e = engine();
        e.set_global(DEFAULT_WATER_HEIGHT, f32::MAX);
        // The sub-object constructors do nothing; the water noise texture's
        // constructor installs a vtable whose slot 0 the constructor calls.
        quiet(
            &mut e,
            &[
                FORM_CONSTRUCT,
                CELL_FULL_NAME_CONSTRUCT,
                EXTRA_LIST_CONSTRUCT,
                LIST_CONSTRUCT,
                FORM_SET_FLAG_8,
                fake(0x1000),
            ],
        );
        e.register(FORM_SET_FORM_TYPE, |e, a| {
            e.mem.set_u8(a[0] + 4, a[1] as u8);
            Ret::default()
        });
        e.put_vtable(0x0131_0000, &[fake(0x1000)]);
        e.register(TEXTURE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0x0131_0000);
            Ret::default()
        });
        // A cell whose bytes are all 0xFF, so every store is visible.
        let cell: Ptr<TESObjectCELL> = e.new_object();
        e.mem.write(cell.addr(), &[0xff; 0xe0]);
        e.call_log = Some(vec![]);
        let back = e.call(0x0054_15b0, &args![cell]).ptr::<TESObjectCELL>();
        assert_eq!(back, cell);
        let log = e.call_log.take().unwrap();

        assert_eq!(e.mem.u32(cell.addr()), CELL_VTABLE);
        assert_eq!(e.mem.u32(cell.addr() + 0x18), CELL_FULL_NAME_VTABLE);
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0);
        assert_eq!(e.get(cell, TESObjectCELL::cCellGameFlags), 0);
        assert_eq!(e.get(cell, TESObjectCELL::cCellState), 0);
        assert_eq!(
            e.get(cell, TESObjectCELL::iLightingTemplateInheritanceFlags),
            0x9f
        );
        assert!(e.get(cell, TESObjectCELL::pLightingTemplate).is_null());
        assert!(e.get(cell, TESObjectCELL::pCellData).is_null());
        assert!(e.get(cell, TESObjectCELL::pCellLand).is_null());
        assert_eq!(e.get(cell, TESObjectCELL::fWaterHeight), f32::MAX);
        assert!(e.get(cell, TESObjectCELL::pNavMeshes).is_null());
        assert!(e.get(cell, TESObjectCELL::pWorldSpace).is_null());
        assert!(e.get(cell, TESObjectCELL::pLoadedData).is_null());
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 0);
        assert_eq!(e.get(cell, TESObjectCELL::iCriticalQueuedRefCount), 0);
        assert_eq!(e.get(cell, TESObjectCELL::sNumRefsWithVisibleDistant), 0);
        assert_eq!(
            e.get(cell, TESObjectCELL::sNumLoadedRefsWithVisibleDistant),
            0
        );
        assert_eq!(e.get(cell, TESObjectCELL::fLodFadeInPercent), 0.0);
        for flag in [
            TESObjectCELL::bLODFadingIn,
            TESObjectCELL::bFadedIn,
            TESObjectCELL::bAutoWaterLoaded,
            TESObjectCELL::bFadingToLowDetail,
            TESObjectCELL::bFadingToHighDetail,
            TESObjectCELL::bDisplayHighDetail,
            TESObjectCELL::bCellDetached,
            TESObjectCELL::bUpdateTerrain,
        ] {
            assert!(!e.get(cell, flag));
        }
        // The four smart pointers are constructed empty, then the portal
        // graph is assigned null again.
        for off in [0xb4u32, 0xb8, 0xbc, 0xd4] {
            assert!(calls_to(&log, SLOT_CONSTRUCT).contains(&vec![cell.addr() + off, 0]));
            assert_eq!(e.mem.u32(cell.addr() + off), 0);
        }
        assert_eq!(
            calls_to(&log, SLOT_ASSIGN),
            vec![vec![cell.addr() + 0xd4, 0]]
        );
        // Form type 0x39, form flag 8, the lock and the reference list.
        assert_eq!(
            calls_to(&log, FORM_SET_FORM_TYPE),
            vec![vec![cell.addr(), 0x39]]
        );
        assert_eq!(e.mem.u8(cell.addr() + 4), 0x39);
        assert_eq!(calls_to(&log, FORM_SET_FLAG_8), vec![vec![cell.addr(), 1]]);
        assert_eq!(
            calls_to(&log, LIST_CONSTRUCT),
            vec![vec![cell.addr() + 0x80], vec![cell.addr() + 0xac]]
        );
        // The base form comes first; the texture's virtual slot 0 last.
        assert_eq!(sequence(&log)[0], FORM_CONSTRUCT);
        assert_eq!(*sequence(&log).last().unwrap(), fake(0x1000));
    }

    /// Doubles for everything the destructor calls, with a land, a nav mesh
    /// array and a region list present.
    fn destructor_engine() -> (Engine, Ptr<TESObjectCELL>) {
        let mut e = engine();
        quiet(
            &mut e,
            &[
                CELL_CLEAR_STATE,
                CELL_RELEASE_STATE,
                REGION_LIST_RELEASE,
                WORLD_SPACE_RELEASE_CELL,
                CELL_CLEAR_REFERENCES,
                CELL_ERASE,
                EXTRA_LIST_REMOVE_NON_PERSISTENT,
                LIST_DESTRUCT,
                TEXTURE_DESTRUCT,
                EXTRA_LIST_DESTRUCT,
                CELL_FULL_NAME_DESTRUCT,
                FORM_DESTRUCT,
                NAV_MESH_ARRAY_CLEAR,
                NAV_MESH_ARRAY_BASE_DESTRUCT,
            ],
        );
        let cell = cell(&mut e, false);
        let land = object(&mut e);
        let nav_meshes = e.mem.alloc(0x10);
        let region = object(&mut e);
        let world = object(&mut e);
        e.set(cell, TESObjectCELL::pCellLand, land);
        e.set(cell, TESObjectCELL::pNavMeshes, Ptr::new(nav_meshes));
        e.set(cell, TESObjectCELL::pWorldSpace, world);
        let data = e.mem.alloc(0xc);
        e.set(cell, TESObjectCELL::pCellData, Ptr::new(data));
        // The region list is returned by the extra data list accessor.
        e.mem.set_u32(SCRATCH, region.addr());
        e.register(EXTRA_LIST_GET_REGION_LIST, |e, _| {
            e.mem.u32(SCRATCH).into_ret()
        });
        // The loader singleton.
        let loader = e.mem.alloc(0x700);
        e.set_global(LOADER_STATE_POINTER, loader);
        e.register(LOADER_STATE_FLAG_61D, |e, a| {
            (e.mem.u8(a[0] + 0x61d) as u32).into_ret()
        });
        (e, cell)
    }

    /// A mapped word the doubles use for state.
    const SCRATCH: u32 = 0x011c_c600;

    #[test]
    fn destructor_releases_everything_of_a_live_exterior_cell() {
        let (mut e, cell) = destructor_engine();
        let land = e.get(cell, TESObjectCELL::pCellLand);
        let nav_meshes = e.get(cell, TESObjectCELL::pNavMeshes);
        let region = Ptr::<()>::new(e.mem.u32(SCRATCH));
        let world = e.get(cell, TESObjectCELL::pWorldSpace);
        let data = e.get(cell, TESObjectCELL::pCellData);
        e.call_log = Some(vec![]);
        e.call(0x0054_1810, &args![cell]);
        let log = e.call_log.take().unwrap();

        assert_eq!(e.mem.u32(cell.addr()), CELL_VTABLE);
        assert_eq!(
            sequence(&log),
            vec![
                CELL_CLEAR_STATE,
                FORM_FLAG_4000,
                CELL_RELEASE_STATE,
                fake(0x10), // the land's deleting destructor
                // the nav mesh array's deleting destructor, then the free
                NAV_MESH_ARRAY_CLEAR,
                NAV_MESH_ARRAY_BASE_DESTRUCT,
                DEALLOCATE,
                EXTRA_LIST_GET_REGION_LIST,
                REGION_LIST_RELEASE,
                fake(0), // the region list's deleting destructor
                LOADER_STATE_FLAG_61D,
                CELL_GET_WORLD_SPACE,
                WORLD_SPACE_RELEASE_CELL,
                CELL_CLEAR_REFERENCES,
                CELL_ERASE,
                EXTRA_LIST_REMOVE_NON_PERSISTENT,
                CELL_IS_INTERIOR,
                DEALLOCATE, // pCellData
                CELL_SET_LIGHTING_TEMPLATE,
                SLOT_RELEASE,
                SLOT_RELEASE,
                SLOT_RELEASE,
                SLOT_RELEASE,
                LIST_DESTRUCT,
                TEXTURE_DESTRUCT,
                EXTRA_LIST_DESTRUCT,
                CELL_FULL_NAME_DESTRUCT,
                FORM_DESTRUCT,
            ]
        );
        assert_eq!(calls_to(&log, fake(0x10)), vec![vec![land.addr(), 1]]);
        assert_eq!(calls_to(&log, fake(0)), vec![vec![region.addr(), 1]]);
        assert_eq!(
            calls_to(&log, WORLD_SPACE_RELEASE_CELL),
            vec![vec![world.addr(), cell.addr()]]
        );
        // The nav mesh array is freed with the flag, and so is the data.
        assert_eq!(e.mem.block_size(nav_meshes.addr()), None);
        assert_eq!(e.mem.block_size(data.addr()), None);
        assert!(e.get(cell, TESObjectCELL::pCellLand).is_null());
        assert!(e.get(cell, TESObjectCELL::pNavMeshes).is_null());
        assert!(e.get(cell, TESObjectCELL::pCellData).is_null());
        // The slots are released in the reverse order of construction.
        let released: Vec<u32> = calls_to(&log, SLOT_RELEASE).iter().map(|c| c[0]).collect();
        assert_eq!(
            released,
            vec![
                cell.addr() + 0xd4,
                cell.addr() + 0xbc,
                cell.addr() + 0xb8,
                cell.addr() + 0xb4
            ]
        );
    }

    #[test]
    fn destructor_skips_the_release_work_for_a_flagged_form() {
        let (mut e, cell) = destructor_engine();
        e.mem.set_u32(cell.addr() + 8, 0x4000);
        // The loader says the cell must stay registered with its world.
        let loader = e.global::<u32>(LOADER_STATE_POINTER);
        e.mem.set_u8(loader + 0x61d, 1);
        e.call_log = Some(vec![]);
        e.call(0x0054_1810, &args![cell]);
        let log = e.call_log.take().unwrap();
        let seq = sequence(&log);
        assert!(!seq.contains(&CELL_RELEASE_STATE));
        assert!(!seq.contains(&fake(0x10)));
        assert!(!seq.contains(&EXTRA_LIST_GET_REGION_LIST));
        assert!(!seq.contains(&WORLD_SPACE_RELEASE_CELL));
        assert!(!seq.contains(&CELL_GET_WORLD_SPACE));
        // The land and nav meshes are left in place.
        assert!(!e.get(cell, TESObjectCELL::pCellLand).is_null());
        assert!(!e.get(cell, TESObjectCELL::pNavMeshes).is_null());
        assert!(seq.contains(&FORM_DESTRUCT));
    }

    #[test]
    fn destructor_without_a_world_space_does_not_release_the_cell() {
        let (mut e, cell) = destructor_engine();
        e.set(cell, TESObjectCELL::pWorldSpace, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x0054_1810, &args![cell]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, WORLD_SPACE_RELEASE_CELL).is_empty());
        assert_eq!(calls_to(&log, CELL_GET_WORLD_SPACE).len(), 1);
    }

    #[test]
    fn scalar_deleting_destructors_free_on_request() {
        let (mut e, cell) = destructor_engine();
        e.call_log = Some(vec![]);
        let back = e.call(0x0054_17e0, &args![cell, 0u32]).ptr::<()>();
        assert_eq!(back.addr(), cell.addr());
        let log = e.call_log.take().unwrap();
        assert!(e.mem.block_size(cell.addr()).is_some());
        assert!(!calls_to(&log, DEALLOCATE)
            .iter()
            .any(|c| c[0] == cell.addr()));
        // Bit 0 of the flags decides; other bits do not.
        let (mut e, cell) = destructor_engine();
        e.call_log = Some(vec![]);
        e.call(0x0054_17e0, &args![cell, 3u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(*log.last().unwrap(), (DEALLOCATE, vec![cell.addr()]));
        assert_eq!(e.mem.block_size(cell.addr()), None);

        // The nav mesh array's form.
        let mut e = engine();
        quiet(
            &mut e,
            &[NAV_MESH_ARRAY_CLEAR, NAV_MESH_ARRAY_BASE_DESTRUCT],
        );
        let array = e.mem.alloc(0x10);
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0054_1a30, &args![Ptr::<()>::new(array), 0u32])
                .u32(),
            array
        );
        assert_eq!(
            sequence(&e.call_log.take().unwrap()),
            vec![NAV_MESH_ARRAY_CLEAR, NAV_MESH_ARRAY_BASE_DESTRUCT]
        );
        assert!(e.mem.block_size(array).is_some());
        e.call(0x0054_1a30, &args![Ptr::<()>::new(array), 1u32]);
        assert_eq!(e.mem.block_size(array), None);
    }

    #[test]
    fn clearing_the_cell_data() {
        let mut e = engine();
        quiet(&mut e, &[EXTRA_LIST_REMOVE_NON_PERSISTENT]);
        for interior in [true, false] {
            let cell = cell(&mut e, interior);
            let data = e.mem.alloc(0x2c);
            e.set(cell, TESObjectCELL::pCellData, Ptr::new(data));
            e.set(cell, TESObjectCELL::pLightingTemplate, Ptr::new(0x1234));
            e.set(cell, TESObjectCELL::iLightingTemplateInheritanceFlags, 0x9f);
            e.call_log = Some(vec![]);
            e.call(0x0054_1b00, &args![cell]);
            let log = e.call_log.take().unwrap();
            // The same field is freed whatever the cell's kind.
            assert_eq!(calls_to(&log, DEALLOCATE), vec![vec![data]]);
            assert_eq!(
                calls_to(&log, EXTRA_LIST_REMOVE_NON_PERSISTENT),
                vec![vec![cell.addr() + 0x28]]
            );
            assert!(e.get(cell, TESObjectCELL::pCellData).is_null());
            assert!(e.get(cell, TESObjectCELL::pLightingTemplate).is_null());
            assert_eq!(
                e.get(cell, TESObjectCELL::iLightingTemplateInheritanceFlags),
                0
            );
        }
    }

    #[test]
    fn init_statics_raises_a_setting_above_2048() {
        for (value, raised) in [
            (3000.0f32, true),
            (2048.0, false),
            (1000.0, false),
            (f32::NAN, false),
        ] {
            let mut e = engine();
            // The three settings words the builder gets.
            for (i, word) in [0x1111u32, 0x2222, 0x3333].iter().enumerate() {
                e.set_global(STATIC_SETTINGS_SOURCE + 4 * i as u32, *word);
            }
            e.set_global(STATIC_FLAG, 7u32);
            e.set_global(SETTING_LIMIT_DOUBLE, 2048.0f64);
            e.set_global(SETTING_LIMIT_FLOAT, 2048.0f32);
            // The builder returns a pointer and keeps what it was given.
            let given = Rc::new(RefCell::new(vec![]));
            let seen = given.clone();
            e.register_double(STATIC_SETTING_BUILDER, move |e, a| {
                *seen.borrow_mut() = e.mem.bytes(a[0], 12);
                0x4444u32.into_ret()
            });
            let setting = e.mem.alloc(4);
            e.mem.set_f32(setting, value);
            returns(&mut e, STATIC_SETTING_GET, setting);
            quiet(&mut e, &[STATIC_SETTING_SET]);
            e.call_log = Some(vec![]);
            e.call(0x0054_1b80, &[]);
            let log = e.call_log.take().unwrap();
            let words: Vec<u32> = given
                .borrow()
                .chunks(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            assert_eq!(words, vec![0x1111, 0x2222, 0x3333]);
            assert_eq!(e.global::<u32>(STATIC_FLAG), 0);
            assert_eq!(e.mem.u32(STATIC_POINTER_SLOT), 0x4444);
            assert_eq!(
                calls_to(&log, STATIC_SETTING_SET),
                if raised {
                    vec![vec![STATIC_SETTING_BLOCK, 2048.0f32.to_bits()]]
                } else {
                    vec![]
                }
            );
        }
    }

    #[test]
    fn releasing_the_statics() {
        let mut e = engine();
        let held = object(&mut e).addr();
        e.set_global(STATIC_OBJECT, held);
        e.set_global(STATIC_RESET_WORD, 5u32);
        quiet(&mut e, &[STATIC_RELEASE_HELPER]);
        let target = object_with_answer(&mut e, 0x77);
        e.mem.set_u32(STATIC_POINTER_SLOT, target.addr());
        e.call_log = Some(vec![]);
        e.call(0x0054_1c00, &[]);
        let log = e.call_log.take().unwrap();
        // The helper gets the static object and the value the pointed
        // object's slot 0x94 produced; the static object is deleted.
        assert_eq!(
            calls_to(&log, STATIC_RELEASE_HELPER),
            vec![vec![held, 0x77]]
        );
        assert_eq!(calls_to(&log, fake(0)), vec![vec![held, 1]]);
        assert_eq!(e.global::<u32>(STATIC_OBJECT), 0);
        assert_eq!(e.mem.u32(STATIC_POINTER_SLOT), 0);
        assert_eq!(e.global::<u32>(STATIC_RESET_WORD), 0);

        // Without a static object only the slot and the word are cleared.
        e.set_global(STATIC_RESET_WORD, 5u32);
        e.mem.set_u32(STATIC_POINTER_SLOT, target.addr());
        e.call_log = Some(vec![]);
        e.call(0x0054_1c00, &[]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, STATIC_RELEASE_HELPER).is_empty());
        assert_eq!(e.mem.u32(STATIC_POINTER_SLOT), 0);
        assert_eq!(e.global::<u32>(STATIC_RESET_WORD), 0);
    }

    /// An object whose virtual slot 0x94 answers `answer`.
    fn object_with_answer(e: &mut Engine, answer: u32) -> Ptr {
        let target = object(e);
        e.mem.set_u32(target.addr() + SLOT_94_RESULT, answer);
        target
    }

    // ---- saving ----------------------------------------------------

    /// Doubles for the record-writing helpers; the chunk calls are only
    /// recorded.
    fn save_engine() -> Engine {
        let mut e = engine();
        quiet(
            &mut e,
            &[
                FORM_START_FORM,
                FULL_NAME_SAVE,
                FORM_ADD_CHUNK_BYTE,
                FORM_ADD_CHUNK_DATA,
                FORM_ADD_CHUNK_WORD,
                FORM_ADD_CHUNK_ARRAY,
                EXTRA_LIST_SAVE,
                FORM_CLOSE_FORM,
                SWAP_INTERIOR_DATA,
                SWAP_EXTERIOR_DATA,
                IMPACT_SWAP_SAVE,
            ],
        );
        returns(&mut e, ENDIAN_SWAP_ENABLED, 0);
        returns(&mut e, EXTRA_LIST_GET_IMPACT_SWAP, 0);
        returns(&mut e, FULL_NAME_LENGTH, 4);
        returns(&mut e, TEXT_GET_STRING, 0x0900_0000);
        e
    }

    #[test]
    fn saving_the_interior_record() {
        let mut e = save_engine();
        returns(&mut e, ENDIAN_SWAP_ENABLED, 1);
        returns(&mut e, EXTRA_LIST_GET_IMPACT_SWAP, 0x6000);
        let cell = cell(&mut e, true);
        e.set(cell, TESObjectCELL::cCellFlags, 0x81);
        let data = e.mem.alloc(0x2c);
        e.set(cell, TESObjectCELL::pCellData, Ptr::new(data));
        let template = object(&mut e);
        e.mem.set_u32(template.addr() + 0xc, 0xab);
        e.set(cell, TESObjectCELL::pLightingTemplate, template);
        e.set(cell, TESObjectCELL::iLightingTemplateInheritanceFlags, 0x9f);
        e.set(cell, TESObjectCELL::fWaterHeight, 12.5);
        e.call_log = Some(vec![]);
        e.call(0x0054_1ce0, &args![cell]);
        let log = e.call_log.take().unwrap();
        let swap = ENDIAN_SWAP_ENABLED;
        assert_eq!(
            sequence(&log),
            vec![
                FORM_START_FORM,
                FULL_NAME_SAVE,
                FORM_ADD_CHUNK_BYTE,
                CELL_IS_INTERIOR,
                CELL_INTERIOR_DATA,
                swap,
                SWAP_INTERIOR_DATA,
                FORM_ADD_CHUNK_DATA,
                swap,
                SWAP_INTERIOR_DATA,
                CELL_EXTRA_DATA_LIST,
                EXTRA_LIST_GET_IMPACT_SWAP,
                IMPACT_SWAP_SAVE,
                CELL_GET_LIGHTING_TEMPLATE,
                CELL_GET_LIGHTING_TEMPLATE,
                FORM_ID,
                FORM_ADD_CHUNK_WORD,
                FORM_ADD_CHUNK_WORD,
                FORM_ADD_CHUNK_WORD,
                FULL_NAME_LENGTH,
                TEXT_GET_STRING,
                FORM_ADD_CHUNK_ARRAY,
                EXTRA_LIST_SAVE,
                FORM_CLOSE_FORM,
            ]
        );
        assert_eq!(
            calls_to(&log, FULL_NAME_SAVE),
            vec![vec![cell.addr() + 0x18]]
        );
        assert_eq!(calls_to(&log, FORM_ADD_CHUNK_BYTE), vec![vec![DATA, 0x81]]);
        assert_eq!(
            calls_to(&log, FORM_ADD_CHUNK_DATA),
            vec![vec![INTERIOR_LIGHTING, data, 0x2c]]
        );
        assert_eq!(
            calls_to(&log, SWAP_INTERIOR_DATA),
            vec![vec![data], vec![data]]
        );
        assert_eq!(calls_to(&log, IMPACT_SWAP_SAVE), vec![vec![0x6000]]);
        assert_eq!(
            calls_to(&log, FORM_ADD_CHUNK_WORD),
            vec![
                vec![LIGHTING_TEMPLATE_ID, 0xab],
                vec![INHERITANCE_FLAGS, 0x9f],
                vec![WATER_HEIGHT, 12.5f32.to_bits()],
            ]
        );
        // The texture name goes out with its length plus the terminator.
        assert_eq!(
            calls_to(&log, FORM_ADD_CHUNK_ARRAY),
            vec![vec![WATER_NOISE_TEXTURE, 0x0900_0000, 5]]
        );
        assert_eq!(
            calls_to(&log, EXTRA_LIST_SAVE),
            vec![vec![cell.addr() + 0x28]]
        );
    }

    #[test]
    fn saving_the_exterior_record() {
        let mut e = save_engine();
        let cell = cell(&mut e, false);
        let data = e.mem.alloc(0xc);
        e.set(cell, TESObjectCELL::pCellData, Ptr::new(data));
        e.call_log = Some(vec![]);
        e.call(0x0054_1ce0, &args![cell]);
        let log = e.call_log.take().unwrap();
        // No byte swapping, no impact swap; a missing lighting template is
        // saved as id 0.
        assert_eq!(
            calls_to(&log, FORM_ADD_CHUNK_DATA),
            vec![vec![EXTERIOR_COORDINATES, data, 0xc]]
        );
        assert!(calls_to(&log, SWAP_EXTERIOR_DATA).is_empty());
        assert!(calls_to(&log, EXTRA_LIST_GET_IMPACT_SWAP).is_empty());
        assert_eq!(
            calls_to(&log, FORM_ADD_CHUNK_WORD)[0],
            vec![LIGHTING_TEMPLATE_ID, 0]
        );
        assert_eq!(calls_to(&log, FORM_ADD_CHUNK_BYTE), vec![vec![DATA, 0]]);

        // With the swap flag on, the exterior block is swapped around the write.
        returns(&mut e, ENDIAN_SWAP_ENABLED, 1);
        e.call_log = Some(vec![]);
        e.call(0x0054_1ce0, &args![cell]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SWAP_EXTERIOR_DATA),
            vec![vec![data], vec![data]]
        );
    }

    /// Doubles for a small `NiTArray` of pointers (+4 base, +0xA size, +0xC
    /// element count) and the file calls `SaveReferences` makes.
    fn array_doubles(e: &mut Engine) {
        quiet(
            e,
            &[LOCK_ENTER, LOCK_LEAVE, FORM_SAVE_TO_FILE, NI_ARRAY_DESTRUCT],
        );
        e.register(NI_ARRAY_CONSTRUCT, |e, a| {
            e.mem.write(a[0], &[0; 0x10]);
            a[0].into_ret()
        });
        e.register(NI_ARRAY_SET_SIZE, |e, a| {
            let base = e.mem.alloc(4 * a[1]);
            e.mem.set_u32(a[0] + 4, base);
            e.mem.set_u16(a[0] + 8, a[1] as u16);
            Ret::default()
        });
        e.register(NI_ARRAY_SET_GROW_BY, |e, a| {
            e.mem.set_u16(a[0] + 0xe, a[1] as u16);
            Ret::default()
        });
        e.register(NI_ARRAY_ADD, |e, a| {
            let size = e.mem.u16(a[0] + 0xa);
            let base = e.mem.u32(a[0] + 4);
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(base + 4 * size as u32, value);
            e.mem.set_u16(a[0] + 0xa, size + 1);
            e.mem.set_u16(a[0] + 0xc, e.mem.u16(a[0] + 0xc) + 1);
            (size as u32).into_ret()
        });
        quiet(e, &[NI_ARRAY_COMPACT]);
        e.register(NI_ARRAY_COUNT, |e, a| {
            (e.mem.u16(a[0] + 0xc) as u32).into_ret()
        });
        e.register(NI_ARRAY_ELEMENT_ADDRESS, |e, a| {
            (e.mem.u32(a[0] + 4) + 4 * a[1]).into_ret()
        });
        e.register(NI_ARRAY_SET_AT, |e, a| {
            let value = e.mem.u32(a[2]);
            e.mem.set_u32(e.mem.u32(a[0] + 4) + 4 * a[1], value);
            Ret::default()
        });
    }

    /// A file object (the master flag at +0x10).
    fn new_file(e: &mut Engine, master: bool) -> Ptr {
        let file = e.mem.alloc(0x300);
        e.mem.set_u8(file + 0x10, master as u8);
        Ptr::new(file)
    }

    /// A reference with form flags, the file it was last saved in (`+0x80`),
    /// its first file (`+0x84`) and a sort key.
    fn reference(e: &mut Engine, flags: u32, file: Ptr, first_file: Ptr, key: u32) -> Ptr {
        let reference = object(e);
        e.mem.set_u32(reference.addr() + 8, flags);
        e.mem.set_u32(reference.addr() + 0x80, file.addr());
        e.mem.set_u32(reference.addr() + 0x84, first_file.addr());
        e.mem.set_u32(reference.addr() + KEY, key);
        reference
    }

    #[test]
    fn saving_references_filters_sorts_and_saves() {
        let mut e = engine();
        array_doubles(&mut e);
        let target = new_file(&mut e, false);
        let other = new_file(&mut e, false);
        let master = new_file(&mut e, true);
        let none = Ptr::NULL;
        // Kept: normal references, a deleted one from a master file, and one
        // from another file that has form flag 2.
        let r_a = reference(&mut e, 0, target, none, 3);
        let r_b = reference(&mut e, 0, target, none, 1);
        let r_deleted_master = reference(&mut e, 0x20, target, master, 2);
        let r_other_flag2 = reference(&mut e, 2, other, none, 0);
        // Skipped: flag 0x4000, deleted from a non-master file, from another
        // file without flag 2, persistent reference in a non-persistent
        // exterior cell.
        let r_4000 = reference(&mut e, 0x4000, target, none, 9);
        let r_deleted_other = reference(&mut e, 0x20, target, other, 9);
        let r_other = reference(&mut e, 0, other, none, 9);
        let r_persistent = reference(&mut e, 0x800, target, none, 9);
        let cell = cell(&mut e, false);
        set_references(
            &mut e,
            cell,
            &[
                r_a,
                r_4000,
                r_b,
                r_deleted_other,
                r_other,
                r_persistent,
                r_deleted_master,
                r_other_flag2,
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x0054_2040, &args![cell, target]);
        let log = e.call_log.take().unwrap();
        let saved: Vec<Vec<u32>> = calls_to(&log, FORM_SAVE_TO_FILE);
        // Ascending by key: 0, 1, 2, 3.
        assert_eq!(
            saved,
            vec![
                vec![r_other_flag2.addr(), target.addr()],
                vec![r_b.addr(), target.addr()],
                vec![r_deleted_master.addr(), target.addr()],
                vec![r_a.addr(), target.addr()],
            ]
        );
        // The array is built with 50 slots and grows by 50, the lock is held
        // only while collecting.
        assert_eq!(calls_to(&log, NI_ARRAY_CONSTRUCT)[0][1..], [0, 1]);
        assert_eq!(calls_to(&log, NI_ARRAY_SET_SIZE)[0][1], 0x32);
        assert_eq!(calls_to(&log, NI_ARRAY_SET_GROW_BY)[0][1], 0x32);
        let seq = sequence(&log);
        let enter = seq.iter().position(|a| *a == LOCK_ENTER).unwrap();
        let leave = seq.iter().position(|a| *a == LOCK_LEAVE).unwrap();
        let compact = seq.iter().position(|a| *a == NI_ARRAY_COMPACT).unwrap();
        let first_save = seq.iter().position(|a| *a == FORM_SAVE_TO_FILE).unwrap();
        assert!(enter < leave && leave < compact && compact < first_save);
        assert_eq!(calls_to(&log, NI_ARRAY_DESTRUCT).len(), 1);

        // An interior cell keeps persistent references too.
        let interior = cell_interior_with(&mut e, &[r_persistent]);
        e.call_log = Some(vec![]);
        e.call(0x0054_2040, &args![interior, target]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, FORM_SAVE_TO_FILE),
            vec![vec![r_persistent.addr(), target.addr()]]
        );
    }

    fn cell_interior_with(e: &mut Engine, items: &[Ptr]) -> Ptr<TESObjectCELL> {
        let interior = cell(e, true);
        set_references(e, interior, items);
        interior
    }

    /// Records the group header (five words) passed to `StartGroup`.
    fn record_groups(e: &mut Engine) -> Rc<RefCell<Vec<Vec<u32>>>> {
        let headers = Rc::new(RefCell::new(vec![]));
        let seen = headers.clone();
        e.register_double(FILE_START_GROUP, move |e, a| {
            let words = (0..5).map(|i| e.mem.u32(a[1] + 4 * i)).collect();
            seen.borrow_mut().push(words);
            Ret::default()
        });
        headers
    }

    #[test]
    fn saving_a_cell_in_a_master_file() {
        let mut e = engine();
        array_doubles(&mut e);
        quiet(&mut e, &[FILE_ADD_FORM]);
        let headers = record_groups(&mut e);
        let master = new_file(&mut e, true);
        let cell = cell(&mut e, false);
        e.mem.set_u32(cell.addr() + 0xc, 0x0100_00ab);
        // The cell's first file is the master: its references are saved too.
        e.mem.set_u32(cell.addr() + 0x84, master.addr());
        let reference = reference(&mut e, 0, master, Ptr::NULL, 1);
        set_references(&mut e, cell, &[reference]);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_1e80, &args![cell, master]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(
            sequence(&log)[..4],
            [FORM_FLAG_20, LOCK_ENTER, FILE_IS_MASTER, LOCK_LEAVE]
        );
        // Form save (virtual slot 0x2c), then the file takes the form and the
        // group starts: ('GRUP', 0, the cell's id, type 6, 0).
        assert_eq!(calls_to(&log, fake(0x2c)), vec![vec![cell.addr()]]);
        assert_eq!(
            calls_to(&log, FILE_ADD_FORM),
            vec![vec![master.addr(), cell.addr()]]
        );
        assert_eq!(headers.borrow()[0], vec![0x5055_5247, 0, 0x0100_00ab, 6, 0]);
        assert_eq!(
            calls_to(&log, FORM_SAVE_TO_FILE),
            vec![vec![reference.addr(), master.addr()]]
        );

        // The cell came from another file: no references are saved.
        let elsewhere = new_file(&mut e, false);
        e.mem.set_u32(cell.addr() + 0x84, elsewhere.addr());
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_1e80, &args![cell, master]).bool());
        assert!(calls_to(&e.call_log.take().unwrap(), FORM_SAVE_TO_FILE).is_empty());
    }

    #[test]
    fn saving_a_cell_depends_on_its_references() {
        let mut e = engine();
        array_doubles(&mut e);
        quiet(&mut e, &[FILE_ADD_FORM, FILE_START_GROUP]);
        let target = new_file(&mut e, false);
        let other = new_file(&mut e, false);
        // Only references from another file, none with flag 2: not saved.
        let cell = cell(&mut e, false);
        let foreign = reference(&mut e, 0, other, Ptr::NULL, 1);
        set_references(&mut e, cell, &[foreign]);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0054_1e80, &args![cell, target]).bool());
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, fake(0x2c)).is_empty());
        // The lock is left again.
        assert_eq!(
            calls_to(&log, LOCK_ENTER).len(),
            calls_to(&log, LOCK_LEAVE).len()
        );

        // A reference without a file makes the cell wanted...
        let nowhere = reference(&mut e, 0, Ptr::NULL, Ptr::NULL, 1);
        let cell2 = cell_with(&mut e, false, &[foreign, nowhere]);
        assert!(e.call(0x0054_1e80, &args![cell2, target]).bool());
        // ...as does one from this very file, or the cell's own flag 2.
        let ours = reference(&mut e, 0, target, Ptr::NULL, 1);
        let cell3 = cell_with(&mut e, false, &[ours]);
        assert!(e.call(0x0054_1e80, &args![cell3, target]).bool());
        let cell4 = cell_with(&mut e, false, &[foreign]);
        e.mem.set_u32(cell4.addr() + 8, 2);
        assert!(e.call(0x0054_1e80, &args![cell4, target]).bool());

        // A deleted cell is never saved.
        let deleted = cell_with(&mut e, false, &[ours]);
        e.mem.set_u32(deleted.addr() + 8, 0x20);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0054_1e80, &args![deleted, target]).bool());
        assert_eq!(sequence(&e.call_log.take().unwrap()), vec![FORM_FLAG_20]);
    }

    fn cell_with(e: &mut Engine, interior: bool, items: &[Ptr]) -> Ptr<TESObjectCELL> {
        let cell = cell(e, interior);
        set_references(e, cell, items);
        cell
    }

    #[test]
    fn saving_an_edit_copy_always_saves_references() {
        let mut e = engine();
        array_doubles(&mut e);
        quiet(&mut e, &[FILE_ADD_FORM]);
        let headers = record_groups(&mut e);
        let target = new_file(&mut e, false);
        let cell = cell_with(&mut e, false, &[]);
        e.mem.set_u32(cell.addr() + 0xc, 0x77);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_1fd0, &args![cell, target]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, fake(0x2c)), vec![vec![cell.addr()]]);
        assert_eq!(
            calls_to(&log, FILE_ADD_FORM),
            vec![vec![target.addr(), cell.addr()]]
        );
        assert_eq!(headers.borrow()[0], vec![0x5055_5247, 0, 0x77, 6, 0]);
        assert_eq!(calls_to(&log, NI_ARRAY_CONSTRUCT).len(), 1);
    }

    // ---- loading ---------------------------------------------------

    /// The chunk the file under test is on, and the ones still to come.
    struct Script {
        queue: VecDeque<(u32, Vec<u8>)>,
        current: Vec<u8>,
    }

    /// Doubles for the TESFile accessors and the helpers `Load` calls. The
    /// file yields `chunks` in order (each as tag, bytes); `swap` is the
    /// file's endian-swap flag. Returns the script so tests can look at what
    /// is left.
    fn load_engine(
        form_type: u8,
        chunks: &[(u32, Vec<u8>)],
        swap: bool,
    ) -> (Engine, Rc<RefCell<Script>>) {
        let mut e = engine();
        let script = Rc::new(RefCell::new(Script {
            queue: chunks.iter().cloned().collect(),
            current: vec![],
        }));
        e.register_double(FILE_GET_FORM_TYPE, move |_, _| {
            (form_type as u32).into_ret()
        });
        returns(&mut e, FILE_MASTER_DATA, 0x5150);
        returns(&mut e, FILE_SWAP_ENDIAN, swap as u32);
        let s = script.clone();
        e.register_double(FILE_GET_CHUNK, move |_, _| {
            let mut s = s.borrow_mut();
            match s.queue.pop_front() {
                Some((tag, bytes)) => {
                    s.current = bytes;
                    tag.into_ret()
                }
                None => 0u32.into_ret(),
            }
        });
        let s = script.clone();
        e.register_double(FILE_NEXT_CHUNK, move |_, _| {
            u32::from(!s.borrow().queue.is_empty()).into_ret()
        });
        let s = script.clone();
        e.register_double(FILE_CHUNK_SIZE, move |_, _| {
            (s.borrow().current.len() as u32).into_ret()
        });
        let s = script.clone();
        e.register_double(FILE_GET_CHUNK_DATA, move |e, a| {
            e.mem.write(a[1], &s.borrow().current);
            Ret::default()
        });
        let s = script.clone();
        e.register_double(FILE_GET_CHUNK_DATA_SIZED, move |e, a| {
            let s = s.borrow();
            let count = (a[2] as usize).min(s.current.len());
            e.mem.write(a[1], &s.current[..count]);
            Ret::default()
        });
        quiet(
            &mut e,
            &[
                FORM_LOAD_FORM,
                FORM_SET_FLAG_8,
                CELL_CREATE_CELL_DATA,
                CELL_SET_WATER_HEIGHT,
                CELL_SET_LOADED_MASTER_DATA,
                FULL_NAME_LOAD,
                FULL_NAME_SET,
                EXTRA_LIST_LOAD,
                EXTRA_LIST_SET_IMPACT_SWAP,
                IMPACT_SWAP_LOAD,
                SWAP_INTERIOR_DATA,
                SWAP_EXTERIOR_DATA,
            ],
        );
        returns(&mut e, EXTRA_LIST_GET_IMPACT_SWAP, 0);
        e.register(IMPACT_SWAP_CONSTRUCT, |_, a| a[0].into_ret());
        e.set_global(DEFAULT_WATER_HEIGHT, f32::MAX);
        let loader = e.mem.alloc(0x700);
        e.set_global(LOADER_STATE_POINTER, loader);
        (e, script)
    }

    fn loading_cell(e: &mut Engine, interior: bool) -> Ptr<TESObjectCELL> {
        let cell = cell(e, interior);
        e.mem.set_u8(cell.addr() + 4, 0x39);
        cell
    }

    #[test]
    fn load_rejects_other_records_and_loads_the_base_form_first() {
        let (mut e, _) = load_engine(0x3a, &[(DATA, vec![1, 0, 0, 0])], false);
        let cell = loading_cell(&mut e, false);
        let file = new_file(&mut e, false);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0054_22d0, &args![cell, file]).bool());
        assert_eq!(
            sequence(&e.call_log.take().unwrap()),
            vec![FILE_GET_FORM_TYPE]
        );

        // A cell record: the form is loaded, the chunks read until none is
        // left, and form flag 8 is cleared.
        let (mut e, _) = load_engine(0x39, &[(tag(b"ZZZZ"), vec![])], false);
        let cell = loading_cell(&mut e, false);
        let file = new_file(&mut e, false);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_22d0, &args![cell, file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(
            sequence(&log),
            vec![
                FILE_GET_FORM_TYPE,
                FILE_IS_MASTER,
                FORM_LOAD_FORM,
                FILE_GET_CHUNK,
                FILE_NEXT_CHUNK,
                FORM_SET_FLAG_8,
            ]
        );
        assert_eq!(
            calls_to(&log, FORM_LOAD_FORM),
            vec![vec![cell.addr(), file.addr()]]
        );
        assert_eq!(calls_to(&log, FORM_SET_FLAG_8), vec![vec![cell.addr(), 0]]);
    }

    #[test]
    fn load_reads_the_cell_flags_of_an_interior() {
        // A 4-byte DATA chunk: interior (bit 0) with the water flag (0x80).
        let (mut e, _) = load_engine(0x39, &[(DATA, vec![0x81, 0, 0, 0])], false);
        let file = new_file(&mut e, true);
        let cell = loading_cell(&mut e, true);
        // The loader singleton's byte +0x619 is set: bit 0x40 stays.
        let loader = e.global::<u32>(LOADER_STATE_POINTER);
        e.mem.set_u8(loader + 0x619, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_22d0, &args![cell, file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0x81);
        // Interior with 0x80: the default water height is set.
        assert_eq!(
            calls_to(&log, CELL_SET_WATER_HEIGHT),
            vec![vec![cell.addr(), f32::MAX.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, CELL_CREATE_CELL_DATA),
            vec![vec![cell.addr()]]
        );
        // The file is a master: its master data goes to the interior cell.
        assert_eq!(
            calls_to(&log, CELL_SET_LOADED_MASTER_DATA),
            vec![vec![cell.addr(), 0x5150]]
        );
        // The master data was fetched before the form was loaded.
        let seq = sequence(&log);
        let master_data = seq.iter().position(|a| *a == FILE_MASTER_DATA).unwrap();
        let load_form = seq.iter().position(|a| *a == FORM_LOAD_FORM).unwrap();
        assert!(master_data < load_form);
    }

    #[test]
    fn load_reads_the_cell_flags_of_an_exterior() {
        // A 1-byte DATA chunk: bit 0 clear sets bit 1; the loader byte is
        // clear, so bit 0x40 goes away.
        let (mut e, _) = load_engine(0x39, &[(DATA, vec![0x40])], false);
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, false);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_22d0, &args![cell, file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(cell, TESObjectCELL::cCellFlags), 0x02);
        assert!(calls_to(&log, CELL_SET_WATER_HEIGHT).is_empty());
        assert!(calls_to(&log, CELL_SET_LOADED_MASTER_DATA).is_empty());
        // The 1-byte form reads straight into the flags byte.
        assert_eq!(
            calls_to(&log, FILE_GET_CHUNK_DATA_SIZED),
            vec![vec![file.addr(), cell.addr() + 0x24, 1]]
        );
    }

    #[test]
    fn load_reads_and_swaps_the_cell_data_blocks() {
        let exterior_chunk = vec![(EXTERIOR_COORDINATES, (0..12).collect::<Vec<u8>>())];
        // The file needs swapping and the chunk has the expected size: the
        // block is swapped once, after the read.
        let (mut e, _) = load_engine(0x39, &exterior_chunk, true);
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, false);
        let data = e.mem.alloc(12);
        e.set(cell, TESObjectCELL::pCellData, Ptr::new(data));
        e.call_log = Some(vec![]);
        e.call(0x0054_22d0, &args![cell, file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.bytes(data, 12), (0..12).collect::<Vec<u8>>());
        assert_eq!(calls_to(&log, SWAP_EXTERIOR_DATA), vec![vec![data]]);

        // A chunk of another size is swapped before the read as well.
        let short = vec![(EXTERIOR_COORDINATES, vec![9; 8])];
        let (mut e, _) = load_engine(0x39, &short, true);
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, false);
        let data = e.mem.alloc(12);
        e.set(cell, TESObjectCELL::pCellData, Ptr::new(data));
        e.call_log = Some(vec![]);
        e.call(0x0054_22d0, &args![cell, file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SWAP_EXTERIOR_DATA),
            vec![vec![data], vec![data]]
        );
        assert_eq!(
            calls_to(&log, FILE_GET_CHUNK_DATA_SIZED),
            vec![vec![file.addr(), data, 12]]
        );

        // Without the swap flag nothing is swapped; the interior block is
        // 44 bytes; no block, nothing read.
        let interior_chunk = vec![(INTERIOR_LIGHTING, vec![7; 44])];
        let (mut e, _) = load_engine(0x39, &interior_chunk, false);
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, true);
        let data = e.mem.alloc(44);
        e.set(cell, TESObjectCELL::pCellData, Ptr::new(data));
        e.call_log = Some(vec![]);
        e.call(0x0054_22d0, &args![cell, file]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, SWAP_INTERIOR_DATA).is_empty());
        assert_eq!(e.mem.bytes(data, 44), vec![7; 44]);
        let (mut e, _) = load_engine(0x39, &interior_chunk, false);
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, true);
        e.call_log = Some(vec![]);
        e.call(0x0054_22d0, &args![cell, file]);
        assert!(calls_to(&e.call_log.take().unwrap(), FILE_GET_CHUNK_DATA_SIZED).is_empty());
    }

    #[test]
    fn load_reads_the_simple_chunks() {
        let chunks = vec![
            (WATER_HEIGHT, 7.5f32.to_le_bytes().to_vec()),
            (INHERITANCE_FLAGS, 0x1234u32.to_le_bytes().to_vec()),
            (LIGHTING_TEMPLATE_ID, 0x0100_0042u32.to_le_bytes().to_vec()),
        ];
        let (mut e, _) = load_engine(0x39, &chunks, false);
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, false);
        e.call(0x0054_22d0, &args![cell, file]);
        assert_eq!(e.get(cell, TESObjectCELL::fWaterHeight), 7.5);
        assert_eq!(
            e.get(cell, TESObjectCELL::iLightingTemplateInheritanceFlags),
            0x1234
        );
        assert_eq!(
            e.get(cell, TESObjectCELL::pLightingTemplate).addr(),
            0x0100_0042
        );
    }

    #[test]
    fn load_hands_names_bounds_and_textures_to_their_owners() {
        let chunks = vec![
            (EDITOR_ID, b"CellEdid\0".to_vec()),
            (OBJECT_BOUNDS, vec![0; 4]),
            (FULL_NAME, vec![]),
            (WATER_NOISE_TEXTURE, b"water.dds\0".to_vec()),
            (WATER_NOISE_TEXTURE, vec![]),
        ];
        let (mut e, _) = load_engine(0x39, &chunks, false);
        // The editor id is passed to virtual slot 0x134 as a C string.
        let seen = Rc::new(RefCell::new(vec![]));
        let strings = seen.clone();
        e.register_double(fake(0x134), move |e, a| {
            strings.borrow_mut().push(e.mem.cstr(a[1]));
            Ret::default()
        });
        let names = Rc::new(RefCell::new(vec![]));
        let texture_names = names.clone();
        e.register_double(FULL_NAME_SET, move |e, a| {
            texture_names.borrow_mut().push((a[0], e.mem.cstr(a[1])));
            Ret::default()
        });
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, false);
        e.call_log = Some(vec![]);
        e.call(0x0054_22d0, &args![cell, file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(*seen.borrow(), vec![b"CellEdid".to_vec()]);
        assert_eq!(
            calls_to(&log, fake(0xe0)),
            vec![vec![cell.addr(), file.addr()]]
        );
        assert_eq!(
            calls_to(&log, FULL_NAME_LOAD),
            vec![vec![cell.addr() + 0x18, file.addr()]]
        );
        // The empty texture chunk is ignored.
        assert_eq!(
            *names.borrow(),
            vec![(cell.addr() + 0x58, b"water.dds".to_vec())]
        );
    }

    #[test]
    fn load_creates_the_impact_swap_data_once_and_forwards_extra_data() {
        let chunks = vec![
            (IMPACT_SWAP_FIRST, vec![1]),
            (tag(b"XOWN"), vec![2]),
            (tag(b"ZZZZ"), vec![3]),
            (IMPACT_SWAP_SECOND, vec![4]),
        ];
        let (mut e, _) = load_engine(0x39, &chunks, false);
        // The first lookup finds nothing, later ones find the stored object.
        let stored = Rc::new(RefCell::new(0u32));
        let slot = stored.clone();
        e.register_double(EXTRA_LIST_GET_IMPACT_SWAP, move |_, _| {
            (*slot.borrow()).into_ret()
        });
        let slot = stored.clone();
        e.register_double(EXTRA_LIST_SET_IMPACT_SWAP, move |_, a| {
            *slot.borrow_mut() = a[1];
            Ret::default()
        });
        let file = new_file(&mut e, false);
        let cell = loading_cell(&mut e, false);
        e.call_log = Some(vec![]);
        e.call(0x0054_22d0, &args![cell, file]);
        let log = e.call_log.take().unwrap();
        // One allocation of 0x15c bytes, one construction, two loads.
        let allocated = calls_to(&log, ALLOCATE);
        assert_eq!(allocated.len(), 1);
        assert_eq!(allocated[0], vec![0x15c]);
        assert_eq!(calls_to(&log, IMPACT_SWAP_CONSTRUCT).len(), 1);
        let object = *stored.borrow();
        assert_ne!(object, 0);
        assert_eq!(
            calls_to(&log, IMPACT_SWAP_LOAD),
            vec![vec![object, file.addr()]; 2]
        );
        // The extra-data chunk goes to the cell's extra data list.
        assert_eq!(
            calls_to(&log, EXTRA_LIST_LOAD),
            vec![vec![cell.addr() + 0x28, file.addr(), cell.addr()]]
        );
    }

    #[test]
    fn load_knows_76_extra_data_tags() {
        assert_eq!(EXTRA_DATA_TAGS.len(), 76);
        let mut sorted = EXTRA_DATA_TAGS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 76, "no tag is listed twice");
        // None of the tags the other arms handle is among them.
        for own in [
            DATA,
            EXTERIOR_COORDINATES,
            INTERIOR_LIGHTING,
            WATER_HEIGHT,
            LIGHTING_TEMPLATE_ID,
            INHERITANCE_FLAGS,
            WATER_NOISE_TEXTURE,
            FULL_NAME,
            EDITOR_ID,
            OBJECT_BOUNDS,
            IMPACT_SWAP_FIRST,
            IMPACT_SWAP_SECOND,
        ] {
            assert!(!EXTRA_DATA_TAGS.contains(&own));
        }
        assert_eq!(tag(b"DATA"), 0x4154_4144);
    }

    // ---- InitItem --------------------------------------------------

    /// Words the `InitItem` doubles keep: the thread's error count, the
    /// number of errors an extra data `InitItem` raises, the lookup result and
    /// the cast result for the lighting template.
    const ERRORS: u32 = 0x011c_c700;
    const ERRORS_RAISED: u32 = 0x011c_c704;
    const LOOKED_UP: u32 = 0x011c_c708;
    const TEMPLATE_CAST: u32 = 0x011c_c70c;

    fn init_engine() -> Engine {
        let mut e = engine();
        quiet(
            &mut e,
            &[
                INC_DISABLE_WARNING_COUNT,
                DEBUG_PRINT,
                GAME_LOAD_FORM,
                GAME_LOAD_NOTIFY,
                EXTRA_LIST_SET_NORTH_ROTATION,
                GAME_FINISH_A,
                GAME_FINISH_B,
                GAME_FINISH_C,
                GAME_FLAG_WRITE,
                NAV_POINTER_RELEASE,
                FORM_ADD_COMPILE_INDEX,
            ],
        );
        e.register(FULL_NAME_SET, |e, a| {
            e.mem.set_u32(a[0] + 4, a[1]);
            Ret::default()
        });
        e.register(FORM_SET_FLAG_8, |e, a| {
            let flags = e.mem.u32(a[0] + 8);
            e.mem
                .set_u32(a[0] + 8, if a[1] != 0 { flags | 8 } else { flags & !8 });
            Ret::default()
        });
        e.register(TEXT_GET_STRING, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(ERROR_COUNT_GET, |e, _| e.mem.u32(ERRORS).into_ret());
        e.register(ERROR_COUNT_SET, |e, a| {
            e.mem.set_u32(ERRORS, a[0]);
            Ret::default()
        });
        // The extra data's `InitItem` is where errors are raised.
        e.register(EXTRA_LIST_INIT_ITEM, |e, _| {
            let raised = e.mem.u32(ERRORS_RAISED);
            e.mem.set_u32(ERRORS, raised);
            Ret::default()
        });
        e.register(FORM_LOOK_UP, |e, _| e.mem.u32(LOOKED_UP).into_ret());
        e.register(RT_DYNAMIC_CAST, |e, a| {
            if a[3] == RTTI_LIGHTING_TEMPLATE {
                e.mem.u32(TEMPLATE_CAST).into_ret()
            } else {
                a[0].into_ret()
            }
        });
        returns(&mut e, GAME_LOADER_FLAG_SETTER, 1);
        returns(&mut e, GAME_FLAG_READ, 0);
        returns(&mut e, GAME_FLAG_TEST, 0);
        // The loader and save-game singletons.
        let loader = e.mem.alloc(0x40);
        let save_game = e.mem.alloc(0x40);
        e.set_global(GAME_LOADER_POINTER, loader);
        e.set_global(SAVE_GAME_POINTER, save_game);
        e.set_global(PLAYER_HELPER_FORM, 0x1111u32);
        e.set_global(MINUS_ONE, -1.0f64);
        // Nav meshes: the array's count is its word at +8; element `i` is
        // the object at `array + 0x10 + 4 * i`.
        e.register(CELL_NAV_MESHES, |e, a| e.mem.u32(a[0] + 0x64).into_ret());
        e.register(NAV_MESH_ARRAY_COUNT, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(NAV_MESH_ARRAY_GET, |e, a| {
            let mesh = e.mem.u32(a[0] + 0x10 + 4 * a[2]);
            e.mem.set_u32(a[1], mesh);
            a[1].into_ret()
        });
        e.register(NAV_POINTER_GET, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(REFERENCE_PARENT_CELL, |e, a| {
            e.mem.u32(a[0] + 0x78).into_ret()
        });
        e.register(SAVE_FORM_BUFFER_GET_FORM, |e, a| {
            e.mem.u32(a[0] + 0x70).into_ret()
        });
        e.register(REFERENCE_ROTATION, |_, a| (a[0] + 0x20).into_ret());
        e
    }

    /// A cell for `InitItem`: form id, editor id string, full name text.
    fn init_cell(e: &mut Engine, interior: bool, full_name: &str) -> Ptr<TESObjectCELL> {
        let cell = cell(e, interior);
        e.mem.set_u32(cell.addr() + 0xc, 0x1234);
        let editor_id = text(e, "CellEditorId");
        e.mem
            .set_u32(cell.addr() + EDITOR_ID_WORD, editor_id.addr());
        let name = text(e, full_name);
        e.mem.set_u32(cell.addr() + 0x1c, name.addr());
        cell
    }

    fn debug_prints(log: &[(u32, Vec<u32>)]) -> Vec<Vec<u32>> {
        calls_to(log, DEBUG_PRINT)
    }

    #[test]
    fn init_item_of_an_initialized_cell_only_walks_the_children() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, false, "Name");
        e.mem.set_u32(cell.addr() + 8, 0x8);
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            sequence(&log),
            vec![
                FORM_FLAG_8,
                GAME_LOADER_FLAG_SETTER,
                CELL_REFERENCE_LIST,
                LIST_IS_END,
                CELL_NAV_MESHES,
                GAME_LOADER_FLAG_SETTER,
                GAME_FLAG_READ,
            ]
        );
        let loader = e.global::<u32>(GAME_LOADER_POINTER);
        assert_eq!(
            calls_to(&log, GAME_LOADER_FLAG_SETTER),
            vec![vec![loader, 0], vec![loader, 1]]
        );
    }

    #[test]
    fn init_item_of_an_interior_warns_about_a_long_name_and_errors() {
        let mut e = init_engine();
        let long_name = "N".repeat(40);
        let cell = init_cell(&mut e, true, &long_name);
        e.mem.set_u32(ERRORS, 5);
        e.mem.set_u32(ERRORS_RAISED, 2);
        // A template id that resolves to an object.
        e.mem.set_u32(cell.addr() + 0xd8, 0x0100_0007);
        let template = object(&mut e);
        e.mem.set_u32(LOOKED_UP, template.addr());
        e.mem.set_u32(TEMPLATE_CAST, template.addr());
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        let name = e.mem.u32(cell.addr() + 0x1c);
        let editor_id = e.mem.u32(cell.addr() + EDITOR_ID_WORD);
        assert_eq!(
            debug_prints(&log),
            vec![
                vec![CELL_NAME_TOO_LONG, name, editor_id, 0x1234],
                vec![INIT_ERRORS_INTERIOR, editor_id, 0x1234],
            ]
        );
        // The error count was saved, cleared, read back and restored; the
        // warnings were muted in between.
        assert_eq!(calls_to(&log, ERROR_COUNT_SET), vec![vec![0], vec![5]]);
        assert_eq!(
            calls_to(&log, INC_DISABLE_WARNING_COUNT),
            vec![vec![1], vec![0]]
        );
        // An interior keeps its name; flag 8 is set; the template resolved.
        assert!(calls_to(&log, FULL_NAME_SET).is_empty());
        assert_ne!(e.mem.u32(cell.addr() + 8) & 8, 0);
        assert_eq!(e.get(cell, TESObjectCELL::pLightingTemplate), template);
        assert_eq!(
            calls_to(&log, EXTRA_LIST_INIT_ITEM),
            vec![vec![cell.addr() + 0x28, cell.addr()]]
        );
    }

    #[test]
    fn init_item_of_an_exterior_names_its_world_and_missing_template() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, false, "Goodsprings");
        let data = e.mem.alloc(12);
        e.mem.set_i32(data, 3);
        e.mem.set_i32(data + 4, -4);
        e.mem.set_u32(cell.addr() + 0x48, data);
        let world = object(&mut e);
        e.mem.set_u32(world.addr() + 0xc, 0x55);
        let world_editor_id = text(&mut e, "WastelandNV");
        e.mem
            .set_u32(world.addr() + EDITOR_ID_WORD, world_editor_id.addr());
        e.mem.set_u32(cell.addr() + 0xc0, world.addr());
        e.mem.set_u32(ERRORS_RAISED, 1);
        e.mem.set_u32(cell.addr() + 0xd8, 0x0100_0007);
        // The id does not resolve to a template.
        e.mem.set_u32(LOOKED_UP, 0x4000);
        e.mem.set_u32(TEMPLATE_CAST, 0);
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        let editor_id = e.mem.u32(cell.addr() + EDITOR_ID_WORD);
        assert_eq!(
            debug_prints(&log),
            vec![
                vec![
                    INIT_ERRORS_EXTERIOR_WORLD,
                    editor_id,
                    0x1234,
                    3,
                    (-4i32) as u32,
                    world_editor_id.addr(),
                    0x55,
                ],
                vec![LIGHTING_TEMPLATE_MISSING, 0x0100_0007, editor_id, 0x1234],
            ]
        );
        // An exterior cell loses its full name.
        assert_eq!(
            calls_to(&log, FULL_NAME_SET),
            vec![vec![cell.addr() + 0x18, 0]]
        );
        assert!(e.get(cell, TESObjectCELL::pLightingTemplate).is_null());
        // The template id was made file-relative first.
        assert_eq!(calls_to(&log, FORM_ADD_COMPILE_INDEX).len(), 1);
    }

    #[test]
    fn init_item_of_an_exterior_without_a_world_space() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, false, "Lost");
        let data = e.mem.alloc(12);
        e.mem.set_i32(data, 7);
        e.mem.set_i32(data + 4, 9);
        e.mem.set_u32(cell.addr() + 0x48, data);
        e.mem.set_u32(ERRORS_RAISED, 1);
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        let editor_id = e.mem.u32(cell.addr() + EDITOR_ID_WORD);
        assert_eq!(
            debug_prints(&log),
            vec![vec![
                INIT_ERRORS_EXTERIOR_UNKNOWN_WORLD,
                editor_id,
                0x1234,
                7,
                9
            ]]
        );
    }

    #[test]
    fn init_item_without_errors_prints_nothing() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, true, "Short");
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        assert!(debug_prints(&log).is_empty());
        // A name of exactly 33 characters is still fine.
        let mut e = init_engine();
        let cell = init_cell(&mut e, true, &"N".repeat(33));
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        assert!(debug_prints(&e.call_log.take().unwrap()).is_empty());
    }

    /// A reference for the `InitItem` walk: form flags, parent cell (`+0x78`),
    /// the form `GetForm` reports (`+0x70`) and a rotation float at `+0x28`.
    fn walked_reference(e: &mut Engine, flags: u32, parent: Ptr, form: u32, rotation: f32) -> Ptr {
        let reference = object(e);
        e.mem.set_u32(reference.addr() + 8, flags);
        e.mem.set_u32(reference.addr() + 0x78, parent.addr());
        e.mem.set_u32(reference.addr() + 0x70, form);
        e.mem.set_f32(reference.addr() + 0x28, rotation);
        reference
    }

    #[test]
    fn init_item_initializes_every_reference_and_follows_the_ones_that_move() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, false, "Name");
        e.mem.set_u32(cell.addr() + 8, 0x8);
        let elsewhere = cell_with(&mut e, false, &[]);
        let r1 = walked_reference(&mut e, 0, cell.cast(), 0, 0.0);
        // Already initialized: only the move check applies.
        let r2 = walked_reference(&mut e, 8, elsewhere.cast(), 0, 0.0);
        let r3 = walked_reference(&mut e, 0, cell.cast(), 0, 0.0);
        set_references(&mut e, cell, &[r1, r2, r3]);
        // r2 leaves the cell: its node is unlinked, as the game does.
        let node1 = cell.addr() + 0xac;
        let node2 = e.mem.u32(node1 + 4);
        let node3 = e.mem.u32(node2 + 4);
        e.mem.set_u32(SCRATCH, node1);
        e.mem.set_u32(SCRATCH + 4, node3);
        e.register_double(REFERENCE_PARENT_CELL, move |e, a| {
            let parent = e.mem.u32(a[0] + 0x78);
            if parent != e.mem.u32(SCRATCH + 8) {
                // Seen for the first time: the reference is gone.
                let (first, third) = (e.mem.u32(SCRATCH), e.mem.u32(SCRATCH + 4));
                e.mem.set_u32(first + 4, third);
            }
            parent.into_ret()
        });
        e.mem.set_u32(SCRATCH + 8, cell.addr());
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        let loader = e.global::<u32>(GAME_LOADER_POINTER);
        let save_game = e.global::<u32>(SAVE_GAME_POINTER);
        // r1 and r3 go through the form InitItem (slot 0x88), the save-game
        // notification and the loader; r2 does not.
        assert_eq!(
            calls_to(&log, fake(0x88)),
            vec![vec![r1.addr()], vec![r3.addr()]]
        );
        assert_eq!(
            calls_to(&log, GAME_LOAD_NOTIFY),
            vec![vec![save_game, r1.addr()], vec![save_game, r3.addr()]]
        );
        assert_eq!(
            calls_to(&log, GAME_LOAD_FORM),
            vec![vec![loader, r1.addr()], vec![loader, r3.addr()]]
        );
        // After r2 moved, the walk went on with r3 (the node after r1).
        assert_eq!(calls_to(&log, REFERENCE_PARENT_CELL).len(), 3);
    }

    #[test]
    fn init_item_derives_the_north_rotation_from_the_player_helper() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, true, "Name");
        e.mem.set_u32(cell.addr() + 8, 0x8);
        let helper = walked_reference(&mut e, 8, cell.cast(), 0x1111, 1.5);
        let other = walked_reference(&mut e, 8, cell.cast(), 0x2222, 9.0);
        set_references(&mut e, cell, &[other, helper]);
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, EXTRA_LIST_SET_NORTH_ROTATION),
            vec![vec![cell.addr() + 0x28, (-1.5f32).to_bits()]]
        );
        // An exterior cell never does that.
        let mut e = init_engine();
        let cell = init_cell(&mut e, false, "Name");
        e.mem.set_u32(cell.addr() + 8, 0x8);
        let helper = walked_reference(&mut e, 8, cell.cast(), 0x1111, 1.5);
        set_references(&mut e, cell, &[helper]);
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        assert!(calls_to(&e.call_log.take().unwrap(), EXTRA_LIST_SET_NORTH_ROTATION).is_empty());
    }

    #[test]
    fn init_item_initializes_the_nav_meshes() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, false, "Name");
        e.mem.set_u32(cell.addr() + 8, 0x8);
        let array = e.mem.alloc(0x20);
        e.mem.set_u32(array + 8, 2);
        let mesh_a = object(&mut e);
        let mesh_b = object(&mut e);
        e.mem.set_u32(array + 0x10, mesh_a.addr());
        e.mem.set_u32(array + 0x14, mesh_b.addr());
        e.set(cell, TESObjectCELL::pNavMeshes, Ptr::new(array));
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        let loader = e.global::<u32>(GAME_LOADER_POINTER);
        assert_eq!(
            calls_to(&log, fake(0x88)),
            vec![vec![mesh_a.addr()], vec![mesh_b.addr()]]
        );
        assert_eq!(
            calls_to(&log, GAME_LOAD_FORM),
            vec![vec![loader, mesh_a.addr()], vec![loader, mesh_b.addr()]]
        );
        assert_eq!(calls_to(&log, NAV_POINTER_RELEASE).len(), 2);
    }

    #[test]
    fn init_item_finishes_a_pending_save_game_load() {
        let mut e = init_engine();
        let cell = init_cell(&mut e, false, "Name");
        e.mem.set_u32(cell.addr() + 8, 0x8);
        // The flag reads 1 (pending), then 0 (cleared by the first step),
        // then 5 (the value to restore).
        let reads = Rc::new(RefCell::new(VecDeque::from([1u32, 0, 5])));
        let queue = reads.clone();
        e.register_double(GAME_FLAG_READ, move |_, _| {
            queue.borrow_mut().pop_front().unwrap_or(0).into_ret()
        });
        returns(&mut e, GAME_FLAG_TEST, 1);
        e.call_log = Some(vec![]);
        e.call(0x0054_2d40, &args![cell]);
        let log = e.call_log.take().unwrap();
        let save_game = e.global::<u32>(SAVE_GAME_POINTER);
        let tail: Vec<(u32, Vec<u32>)> = log
            .into_iter()
            .skip_while(|(a, _)| *a != GAME_FINISH_A)
            .collect();
        assert_eq!(
            tail,
            vec![
                (GAME_FINISH_A, vec![save_game, cell.addr()]),
                (GAME_FLAG_READ, vec![save_game]),
                (GAME_FLAG_TEST, vec![save_game]),
                (GAME_FLAG_READ, vec![save_game]),
                (GAME_FLAG_WRITE, vec![save_game, 1]),
                (GAME_FINISH_B, vec![save_game, 0, 0, 0]),
                (GAME_FINISH_C, vec![save_game, 0]),
                (GAME_FLAG_WRITE, vec![save_game, 5]),
            ]
        );
    }

    // ---- SavesBefore and group helpers -----------------------------

    /// A cell for the ordering code: exterior coordinates, form id, key.
    fn ordered_cell(
        e: &mut Engine,
        interior: bool,
        coordinates: (i32, i32),
        id: u32,
        world: Ptr,
    ) -> Ptr<TESObjectCELL> {
        let cell = cell(e, interior);
        e.mem.set_u8(cell.addr() + 4, 0x39);
        e.mem.set_u32(cell.addr() + 0xc, id);
        let data = e.mem.alloc(12);
        e.mem.set_i32(data, coordinates.0);
        e.mem.set_i32(data + 4, coordinates.1);
        e.mem.set_u32(cell.addr() + 0x48, data);
        e.mem.set_u32(cell.addr() + 0xc0, world.addr());
        cell
    }

    fn world_with_key(e: &mut Engine, key: u32) -> Ptr {
        let world = object(e);
        e.mem.set_u32(world.addr() + KEY, key);
        world
    }

    #[test]
    fn cells_save_in_a_fixed_order() {
        let mut e = engine();
        let world = world_with_key(&mut e, 1);
        let interior = ordered_cell(&mut e, true, (0, 0), 0x100, Ptr::NULL);
        let exterior = ordered_cell(&mut e, false, (0, 0), 0x100, world);
        // Interior cells come before exterior ones, never the reverse.
        assert!(e.call(0x0054_3230, &args![interior, exterior]).bool());
        assert!(!e.call(0x0054_3230, &args![exterior, interior]).bool());

        // Exterior cells of different world spaces: the world space decides.
        let later_world = world_with_key(&mut e, 2);
        let in_later = ordered_cell(&mut e, false, (0, 0), 0x100, later_world);
        assert!(e.call(0x0054_3230, &args![exterior, in_later]).bool());
        assert!(!e.call(0x0054_3230, &args![in_later, exterior]).bool());

        // Same world: a persistent cell comes first.
        let persistent = ordered_cell(&mut e, false, (99, 99), 0x200, world);
        e.mem.set_u32(persistent.addr() + 8, 0x400);
        assert!(e.call(0x0054_3230, &args![persistent, exterior]).bool());
        assert!(!e.call(0x0054_3230, &args![exterior, persistent]).bool());

        // Then by block key, sub-block key and form id.
        let near = ordered_cell(&mut e, false, (0, 0), 0x100, world);
        let other_block = ordered_cell(&mut e, false, (64, 0), 0x050, world);
        assert!(e.call(0x0054_3230, &args![near, other_block]).bool());
        assert!(!e.call(0x0054_3230, &args![other_block, near]).bool());
        let other_sub_block = ordered_cell(&mut e, false, (8, 0), 0x050, world);
        assert!(e.call(0x0054_3230, &args![near, other_sub_block]).bool());
        assert!(!e.call(0x0054_3230, &args![other_sub_block, near]).bool());
        let same_place = ordered_cell(&mut e, false, (1, 1), 0x300, world);
        assert!(e.call(0x0054_3230, &args![near, same_place]).bool());
        assert!(!e.call(0x0054_3230, &args![same_place, near]).bool());
        // The very same position and id: not before.
        let twin = ordered_cell(&mut e, false, (1, 1), 0x100, world);
        assert!(!e.call(0x0054_3230, &args![near, twin]).bool());

        // Two interiors: by the digits of the form id.
        let first = ordered_cell(&mut e, true, (0, 0), 0x1_0001, Ptr::NULL);
        let second = ordered_cell(&mut e, true, (0, 0), 0x1_0002, Ptr::NULL);
        assert!(e.call(0x0054_3230, &args![first, second]).bool());
        assert!(!e.call(0x0054_3230, &args![second, first]).bool());
    }

    #[test]
    fn a_form_that_does_not_cast_to_a_cell_is_not_after_it() {
        let mut e = engine();
        let cell = ordered_cell(&mut e, false, (0, 0), 1, Ptr::NULL);
        let other = ordered_cell(&mut e, false, (0, 0), 2, Ptr::NULL);
        e.register(RT_DYNAMIC_CAST, |_, _| 0u32.into_ret());
        assert!(!e.call(0x0054_3230, &args![cell, other]).bool());
    }

    #[test]
    fn cells_order_against_forms_of_other_types() {
        let mut e = engine();
        let world = world_with_key(&mut e, 1);
        let cell = ordered_cell(&mut e, false, (0, 0), 1, world);
        // A reference-like form (type 0x3a): its owner cell (virtual slot 0)
        // is this cell, or compares by this cell's own SavesBefore.
        let owned = object(&mut e);
        e.mem.set_u8(owned.addr() + 4, 0x3a);
        e.mem.set_u32(owned.addr() + SLOT_ZERO_RESULT, cell.addr());
        assert!(e.call(0x0054_3230, &args![cell, owned]).bool());
        let owner = ordered_cell(&mut e, false, (0, 0), 1, world);
        e.mem.set_u32(owner.addr() + KEY, 5);
        e.mem.set_u32(cell.addr() + KEY, 2);
        e.mem.set_u32(owned.addr() + SLOT_ZERO_RESULT, owner.addr());
        assert!(e.call(0x0054_3230, &args![cell, owned]).bool());
        e.mem.set_u32(cell.addr() + KEY, 9);
        assert!(!e.call(0x0054_3230, &args![cell, owned]).bool());
        for form_type in [0x3b, 0x3f, 0x40, 0x42, 0x43, 0x69] {
            e.mem.set_u8(owned.addr() + 4, form_type);
            e.mem.set_u32(owned.addr() + SLOT_ZERO_RESULT, cell.addr());
            assert!(
                e.call(0x0054_3230, &args![cell, owned]).bool(),
                "{form_type:#x}"
            );
        }

        // Type 0x41: always true for an interior cell, otherwise the world
        // space decides.
        let marker = object(&mut e);
        e.mem.set_u8(marker.addr() + 4, 0x41);
        e.mem.set_u32(marker.addr() + KEY, 3);
        let interior = ordered_cell(&mut e, true, (0, 0), 1, Ptr::NULL);
        assert!(e.call(0x0054_3230, &args![interior, marker]).bool());
        assert!(e.call(0x0054_3230, &args![cell, marker]).bool());
        let late_world = world_with_key(&mut e, 4);
        let late_cell = ordered_cell(&mut e, false, (0, 0), 1, late_world);
        assert!(!e.call(0x0054_3230, &args![late_cell, marker]).bool());

        // Any other type goes to the fallback.
        let other = object(&mut e);
        e.mem.set_u8(other.addr() + 4, 0x10);
        returns(&mut e, FORM_COMPARE_FALLBACK, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_3230, &args![cell, other]).bool());
        assert_eq!(
            calls_to(&e.call_log.take().unwrap(), FORM_COMPARE_FALLBACK),
            vec![vec![cell.addr(), other.addr()]]
        );
    }

    /// A group header for the cell's group helpers.
    fn header(e: &mut Engine, group_type: u32, label: u32) -> Ptr {
        let header = e.mem.alloc(0x14);
        e.mem.set_u32(header, 0x5055_5247);
        e.mem.set_u32(header + 8, label);
        e.mem.set_u32(header + 0xc, group_type);
        Ptr::new(header)
    }

    /// `SavesBefore(header)` of `cell` for a group of the given type and label.
    fn before(e: &mut Engine, cell: Ptr<TESObjectCELL>, group_type: u32, label: u32) -> bool {
        let header = header(e, group_type, label);
        e.call(0x0054_34d0, &args![cell, header]).bool()
    }

    /// Makes `form_lookup(label)` answer `form`.
    fn place(e: &mut Engine, label: u32, form: Ptr) {
        e.mem.set_u32(LOOKUP_TABLE + 4 * (label & 0xf), form.addr());
    }

    const LOOKUP_TABLE: u32 = 0x011c_c800;

    #[test]
    fn cells_order_against_group_headers() {
        let mut e = engine();
        let world = world_with_key(&mut e, 1);
        e.register(FORM_LOOK_UP, |e, a| {
            e.mem.u32(LOOKUP_TABLE + 4 * (a[0] & 0xf)).into_ret()
        });
        // Form id 0x105: block key 1, sub-block key 6 as an interior.
        let exterior = ordered_cell(&mut e, false, (0, 0), 0x105, world);
        let interior = ordered_cell(&mut e, true, (0, 0), 0x105, Ptr::NULL);
        // Not a group header.
        assert!(!e
            .call(0x0054_34d0, &args![exterior, Ptr::<()>::NULL])
            .bool());
        let bad = header(&mut e, 7, 0);
        e.mem.set_u32(bad.addr(), 0x1234);
        assert!(!e.call(0x0054_34d0, &args![exterior, bad]).bool());

        // Type 0 goes to the generic code, type 7 is always true, an
        // unknown type is false.
        returns(&mut e, FORM_BELONGS_IN_GROUP_FALLBACK, 1);
        assert!(before(&mut e, exterior, 0, 0));
        assert!(before(&mut e, exterior, 7, 0));
        assert!(!before(&mut e, exterior, 10, 0));

        // Type 1 (world children): an interior is before; an exterior cell
        // by the world space the label names.
        let later_world = world_with_key(&mut e, 5);
        place(&mut e, 3, later_world);
        assert!(before(&mut e, interior, 1, 3));
        assert!(before(&mut e, exterior, 1, 3));
        let earlier_world = world_with_key(&mut e, 0);
        place(&mut e, 3, earlier_world);
        assert!(!before(&mut e, exterior, 1, 3));

        // Types 2 and 3: interior only, key lower than the label.
        assert!(!before(&mut e, exterior, 2, 5));
        assert!(before(&mut e, interior, 2, 2));
        assert!(!before(&mut e, interior, 2, 1));
        assert!(before(&mut e, interior, 3, 7));
        assert!(!before(&mut e, interior, 3, 6));
        // Types 4 and 5: an interior and a persistent cell are before;
        // otherwise the exterior key decides (block key 2<<16, sub-block
        // key 8<<16 for x = 64).
        assert!(before(&mut e, interior, 4, 0));
        assert!(before(&mut e, interior, 5, 0));
        let far = ordered_cell(&mut e, false, (64, 0), 0x100, world);
        assert!(!before(&mut e, far, 4, 0x1_0000));
        assert!(before(&mut e, far, 4, 0x3_0000));
        assert!(!before(&mut e, far, 5, 0x8_0000));
        assert!(before(&mut e, far, 5, 0x9_0000));
        e.mem.set_u32(far.addr() + 8, 0x400);
        assert!(before(&mut e, far, 4, 0));
        assert!(before(&mut e, far, 5, 0));

        // Types 6, 8 and 9: before the cell the label names.
        let big = ordered_cell(&mut e, false, (0, 0), 0x105, world);
        e.mem.set_u32(big.addr() + KEY, 9);
        e.mem.set_u32(exterior.addr() + KEY, 1);
        place(&mut e, 4, big.cast());
        for kind in [6, 8, 9] {
            assert!(before(&mut e, exterior, kind, 4));
            assert!(!before(&mut e, big, kind, 4));
        }
        // A label that names no cell: not before.
        e.register(RT_DYNAMIC_CAST, |_, _| 0u32.into_ret());
        assert!(!before(&mut e, exterior, 6, 4));
        assert!(!before(&mut e, exterior, 1, 3));
    }

    // ---- duplicating, copying, comparing ----------------------------

    #[test]
    fn duplicating_an_interior_gives_nothing() {
        let mut e = engine();
        let cell = cell(&mut e, true);
        e.call_log = Some(vec![]);
        let copy = e.call(0x0054_36e0, &args![cell, 0u32, 7u32]).ptr::<()>();
        assert!(copy.is_null());
        assert_eq!(
            sequence(&e.call_log.take().unwrap()),
            vec![CELL_IS_INTERIOR]
        );
    }

    /// Doubles for `CreateDuplicateForm`: the duplicate of every object is
    /// the object's `DUPLICATE` word, and the nav mesh and list helpers keep
    /// their state in memory.
    fn duplicate_engine() -> Engine {
        let mut e = engine();
        quiet(
            &mut e,
            &[
                LOCK_ENTER,
                LOCK_LEAVE,
                CELL_SET_LAND,
                LAND_SET_CELL,
                CELL_ADD_REFERENCE,
                REFERENCE_SET_PERSISTS,
                NAV_POINTER_CONSTRUCT,
                NAV_POINTER_RELEASE,
                NAV_MESH_ARRAY_ADD,
                LIST_CLEAR,
                LIST_DESTRUCT,
            ],
        );
        e.register(FORM_DUPLICATE, |e, a| {
            e.mem.u32(a[0] + DUPLICATE).into_ret()
        });
        e.register(NAV_MESH_ARRAY_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(CELL_SET_NAV_MESHES, |e, a| {
            e.mem.set_u32(a[0] + 0x64, a[1]);
            Ret::default()
        });
        e.register(NAV_MESH_ARRAY_COUNT, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(NAV_MESH_ARRAY_GET, |e, a| {
            let mesh = e.mem.u32(a[0] + 0x10 + 4 * a[2]);
            e.mem.set_u32(a[1], mesh);
            a[1].into_ret()
        });
        e.register(NAV_POINTER_FROM_RAW, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(NAV_POINTER_ASSIGN, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            a[0].into_ret()
        });
        e.register(NAV_POINTER_COPY, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            a[0].into_ret()
        });
        e.register(LIST_CONSTRUCT, |e, a| {
            e.mem.write(a[0], &[0; 8]);
            a[0].into_ret()
        });
        // The list push appends: it fills the head first, then new nodes.
        e.register(LIST_PUSH_FRONT, |e, a| {
            let value = e.mem.u32(a[1]);
            let mut node = a[0];
            if e.mem.u32(node) != 0 {
                while e.mem.u32(node + 4) != 0 {
                    node = e.mem.u32(node + 4);
                }
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
            e.mem.set_u32(node, value);
            Ret::default()
        });
        e
    }

    /// A duplicate for `original` (an object whose `DUPLICATE` word points at
    /// the new object).
    fn with_duplicate(e: &mut Engine, original: Ptr) -> Ptr {
        let copy = object(e);
        e.mem.set_u32(original.addr() + DUPLICATE, copy.addr());
        copy
    }

    #[test]
    fn duplicating_an_exterior_cell_duplicates_land_nav_meshes_and_references() {
        let mut e = duplicate_engine();
        let cell = cell(&mut e, false);
        let new_cell = with_duplicate(&mut e, cell.cast());
        // The land: its duplicate gets the new cell and the cell gets it.
        let land = object(&mut e);
        e.set(cell, TESObjectCELL::pCellLand, land);
        let new_land = with_duplicate(&mut e, land);
        // Two nav meshes, one of them deleted.
        let array = e.mem.alloc(0x20);
        e.mem.set_u32(array + 8, 2);
        let mesh = object(&mut e);
        let deleted_mesh = object(&mut e);
        e.mem.set_u32(deleted_mesh.addr() + 8, 0x20);
        e.mem.set_u32(array + 0x10, mesh.addr());
        e.mem.set_u32(array + 0x14, deleted_mesh.addr());
        e.set(cell, TESObjectCELL::pNavMeshes, Ptr::new(array));
        let new_mesh = with_duplicate(&mut e, mesh);
        let new_array = e.mem.alloc(0x10);
        // The new array comes from the allocator (0x10 bytes).
        e.register_double(ALLOCATE, move |_, _| new_array.into_ret());
        // References: kept ones are duplicated; 0x4000 and persistent ones
        // are not (the cell is not persistent).
        let kept = reference(&mut e, 0, Ptr::NULL, Ptr::NULL, 0);
        let new_kept = with_duplicate(&mut e, kept);
        let flagged = reference(&mut e, 0x4000, Ptr::NULL, Ptr::NULL, 0);
        let persistent = reference(&mut e, 0x800, Ptr::NULL, Ptr::NULL, 0);
        let kept2 = reference(&mut e, 0, Ptr::NULL, Ptr::NULL, 0);
        let new_kept2 = with_duplicate(&mut e, kept2);
        set_references(&mut e, cell, &[kept, flagged, persistent, kept2]);
        // The duplicated cell is persistent: that flag is carried over.
        e.mem.set_u32(new_cell.addr() + 8, 0x400);
        e.call_log = Some(vec![]);
        let result = e.call(0x0054_36e0, &args![cell, 0u32, 7u32]).ptr::<()>();
        let log = e.call_log.take().unwrap();
        assert_eq!(result, new_cell);

        // The base duplication, then the land.
        assert_eq!(
            calls_to(&log, FORM_DUPLICATE),
            vec![vec![cell.addr(), 0, 7]]
        );
        assert_eq!(calls_to(&log, fake(0x40))[0], vec![land.addr(), 0, 7]);
        assert_eq!(
            calls_to(&log, CELL_SET_LAND),
            vec![vec![new_cell.addr(), new_land.addr()]]
        );
        assert_eq!(
            calls_to(&log, LAND_SET_CELL),
            vec![vec![new_land.addr(), new_cell.addr()]]
        );
        assert_eq!(calls_to(&log, fake(0xc8))[0], vec![new_land.addr(), 1]);
        // The nav mesh array is a new 0x10-byte object set on the new cell;
        // only the live mesh is duplicated, pointed at the new cell, and added.
        assert_eq!(calls_to(&log, ALLOCATE), vec![vec![0x10]]);
        assert_eq!(e.mem.u32(new_cell.addr() + 0x64), new_array);
        assert_eq!(e.mem.u32(new_mesh.addr() + 0x24), new_cell.addr());
        assert_eq!(
            calls_to(&log, NAV_MESH_ARRAY_ADD),
            vec![vec![new_array, new_mesh.addr()]]
        );
        // References: the lock is held while collecting, the kept ones are
        // duplicated, made persistent like the new cell, added and marked.
        assert_eq!(calls_to(&log, LOCK_ENTER).len(), 1);
        assert_eq!(
            calls_to(&log, fake(0x40))[2..],
            [vec![kept.addr(), 0, 7], vec![kept2.addr(), 0, 7]]
        );
        assert_eq!(
            calls_to(&log, REFERENCE_SET_PERSISTS),
            vec![vec![new_kept.addr(), 1], vec![new_kept2.addr(), 1]]
        );
        assert_eq!(
            calls_to(&log, CELL_ADD_REFERENCE),
            vec![
                vec![new_cell.addr(), new_kept.addr(), 0],
                vec![new_cell.addr(), new_kept2.addr(), 0]
            ]
        );
        assert_eq!(
            calls_to(&log, fake(0xc8))[1..],
            [vec![new_kept.addr(), 1], vec![new_kept2.addr(), 1]]
        );
        assert_eq!(calls_to(&log, LIST_CLEAR).len(), 1);
        assert_eq!(calls_to(&log, LIST_DESTRUCT).len(), 1);
    }

    #[test]
    fn duplicating_a_persistent_exterior_cell_keeps_persistent_references() {
        let mut e = duplicate_engine();
        let cell = cell(&mut e, false);
        e.mem.set_u32(cell.addr() + 8, 0x400);
        let new_cell = with_duplicate(&mut e, cell.cast());
        let persistent = reference(&mut e, 0x800, Ptr::NULL, Ptr::NULL, 0);
        let new_reference = with_duplicate(&mut e, persistent);
        set_references(&mut e, cell, &[persistent]);
        e.call_log = Some(vec![]);
        e.call(0x0054_36e0, &args![cell, 0u32, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, CELL_ADD_REFERENCE),
            vec![vec![new_cell.addr(), new_reference.addr(), 0]]
        );
        // No land and no nav meshes: neither is duplicated.
        assert!(calls_to(&log, CELL_SET_LAND).is_empty());
        assert!(calls_to(&log, ALLOCATE).is_empty());
    }

    /// Cells set up for `Copy` and `Compare`.
    fn copy_pair(interior: bool) -> (Engine, Ptr<TESObjectCELL>, Ptr<TESObjectCELL>) {
        let mut e = engine();
        quiet(
            &mut e,
            &[
                FORM_COPY_ALL_COMPONENTS,
                EXTRA_LIST_COPY_LIST,
                CELL_CREATE_CELL_DATA,
            ],
        );
        let size = if interior { 0x2c } else { 0xc };
        let source = cell(&mut e, interior);
        let this = cell(&mut e, interior);
        for (c, fill) in [(source, 0xa5u8), (this, 0x11)] {
            let data = e.mem.alloc(size);
            e.mem.write(data, &vec![fill; size as usize]);
            e.mem.set_u32(c.addr() + 0x48, data);
        }
        e.mem.set_u8(source.addr() + 0x24, 0x80 | interior as u8);
        e.mem.set_f32(source.addr() + 0x50, 42.5);
        e.mem.set_u32(source.addr() + 0xd8, 0x7000);
        e.mem.set_u32(source.addr() + 0xdc, 0x33);
        // The water noise texture component's vtable (slot 2 copies, slot 3
        // compares), one per cell.
        for (c, table) in [(source, 0x0132_0000u32), (this, 0x0132_0100)] {
            e.put_vtable(table, &[0, 0, fake(0x1002), fake(0x1003)]);
            e.mem.set_u32(c.addr() + 0x58, table);
        }
        quiet(&mut e, &[fake(0x1002)]);
        e.register(fake(0x1003), |e, _| e.mem.u32(SCRATCH + 0x40).into_ret());
        (e, this, source)
    }

    #[test]
    fn copying_a_cell_copies_flags_data_and_lighting() {
        for interior in [false, true] {
            let (mut e, this, source) = copy_pair(interior);
            let old_data = e.mem.u32(this.addr() + 0x48);
            e.call_log = Some(vec![]);
            e.call(0x0054_3ab0, &args![this, source]);
            let log = e.call_log.take().unwrap();
            assert_eq!(
                calls_to(&log, FORM_COPY_ALL_COMPONENTS),
                vec![vec![this.addr(), source.addr()]]
            );
            assert_eq!(
                calls_to(&log, EXTRA_LIST_COPY_LIST),
                vec![vec![this.addr() + 0x28, source.addr() + 0x28]]
            );
            // The old data block was freed; the data is created again (the
            // double does not allocate) and the contents copied across.
            assert_eq!(e.mem.block_size(old_data), None);
            assert_eq!(
                e.get(this, TESObjectCELL::cCellFlags),
                0x80 | interior as u8
            );
            assert_eq!(e.get(this, TESObjectCELL::fWaterHeight), 42.5);
            assert_eq!(e.get(this, TESObjectCELL::pLightingTemplate).addr(), 0x7000);
            assert_eq!(
                e.get(this, TESObjectCELL::iLightingTemplateInheritanceFlags),
                0x33
            );
            // The texture component is copied by its virtual slot 2.
            assert_eq!(
                calls_to(&log, fake(0x1002)),
                vec![vec![this.addr() + 0x58, source.addr() + 0x58]]
            );
        }
    }

    #[test]
    fn copying_the_data_block_needs_both_blocks() {
        let (mut e, this, source) = copy_pair(true);
        // `CreateCellData` allocates the new block in the game; here the
        // double does it.
        e.register(CELL_CREATE_CELL_DATA, |e, a| {
            let data = e.mem.alloc(0x2c);
            e.mem.set_u32(a[0] + 0x48, data);
            Ret::default()
        });
        e.call(0x0054_3ab0, &args![this, source]);
        let new = e.mem.u32(this.addr() + 0x48);
        let from = e.mem.u32(source.addr() + 0x48);
        assert_eq!(e.mem.bytes(new, 0x2c), e.mem.bytes(from, 0x2c));
        // No data block on the source: nothing is copied.
        let (mut e, this, source) = copy_pair(false);
        e.mem.set_u32(source.addr() + 0x48, 0);
        e.register(CELL_CREATE_CELL_DATA, |e, a| {
            let data = e.mem.alloc(0xc);
            e.mem.set_u32(a[0] + 0x48, data);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0054_3ab0, &args![this, source]);
        assert!(calls_to(&e.call_log.take().unwrap(), MEMORY_COPY).is_empty());
    }

    #[test]
    fn copying_from_something_that_is_not_a_cell_does_nothing() {
        let (mut e, this, source) = copy_pair(false);
        e.register(RT_DYNAMIC_CAST, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0054_3ab0, &args![this, source]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }

    #[test]
    fn comparing_cells_finds_every_kind_of_difference() {
        for interior in [false, true] {
            let (mut e, this, source) = copy_pair(interior);
            quiet(
                &mut e,
                &[FORM_COMPARE_ALL_COMPONENTS, EXTRA_LIST_COMPARE_LIST],
            );
            let size = if interior { 0x2c } else { 0xc };
            // Make the pair equal first.
            let from = e.mem.u32(source.addr() + 0x48);
            let to = e.mem.u32(this.addr() + 0x48);
            let bytes = e.mem.bytes(from, size);
            e.mem.write(to, &bytes);
            for (offset, width) in [(0x24u32, 1u32), (0x50, 4), (0xd8, 4), (0xdc, 4)] {
                let value = e.mem.bytes(source.addr() + offset, width);
                e.mem.write(this.addr() + offset, &value);
            }
            let different = |e: &mut Engine| e.call(0x0054_3c50, &args![this, source]).bool();
            assert!(!different(&mut e), "equal cells");

            // Each difference alone makes the cells different.
            let flags = e.mem.u8(this.addr() + 0x24);
            e.mem.set_u8(this.addr() + 0x24, flags ^ 0x40);
            assert!(different(&mut e), "cell flags");
            e.mem.set_u8(this.addr() + 0x24, flags);
            e.mem.set_u32(this.addr() + 0xdc, 0x44);
            assert!(different(&mut e), "inheritance flags");
            e.mem.set_u32(this.addr() + 0xdc, 0x33);
            e.mem.set_u32(this.addr() + 0xd8, 0x7001);
            assert!(different(&mut e), "lighting template");
            e.mem.set_u32(this.addr() + 0xd8, 0x7000);
            e.mem.set_f32(this.addr() + 0x50, 1.0);
            assert!(different(&mut e), "water height");
            e.mem.set_f32(this.addr() + 0x50, f32::NAN);
            e.mem.set_f32(source.addr() + 0x50, f32::NAN);
            assert!(different(&mut e), "a NaN height is different from itself");
            e.mem.set_f32(this.addr() + 0x50, 42.5);
            e.mem.set_f32(source.addr() + 0x50, 42.5);
            assert!(!different(&mut e));
            e.mem.set_u8(to, e.mem.u8(to) ^ 1);
            assert!(different(&mut e), "data block");
            e.mem.set_u8(to, e.mem.u8(to) ^ 1);
            // A missing block on either side is not a difference.
            e.mem.set_u32(this.addr() + 0x48, 0);
            assert!(!different(&mut e));
            e.mem.set_u32(this.addr() + 0x48, to);
            returns(&mut e, FORM_COMPARE_ALL_COMPONENTS, 1);
            assert!(different(&mut e), "form components");
            returns(&mut e, FORM_COMPARE_ALL_COMPONENTS, 0);
            returns(&mut e, EXTRA_LIST_COMPARE_LIST, 1);
            assert!(different(&mut e), "extra data");
            returns(&mut e, EXTRA_LIST_COMPARE_LIST, 0);
            // The texture component has the last word.
            e.mem.set_u32(SCRATCH + 0x40, 1);
            assert!(different(&mut e), "texture component");
            e.mem.set_u32(SCRATCH + 0x40, 0);
            // Not a cell at all.
            e.register(RT_DYNAMIC_CAST, |_, _| 0u32.into_ret());
            assert!(different(&mut e), "not a cell");
        }
    }

    // ---- BelongsInGroup and CreateGroupData -------------------------

    /// `BelongsInGroup(header, a, b)` of `cell` for a group of the given type
    /// and label.
    fn belongs_to(
        e: &mut Engine,
        cell: Ptr<TESObjectCELL>,
        group_type: u32,
        label: u32,
        with_children: u8,
        strict: u8,
    ) -> bool {
        let header = header(e, group_type, label);
        e.call(0x0054_3df0, &args![cell, header, with_children, strict])
            .bool()
    }

    #[test]
    fn cells_belong_in_the_groups_that_name_them() {
        let mut e = engine();
        let world = world_with_key(&mut e, 1);
        e.mem.set_u32(world.addr() + BELONGS, 1);
        let exterior = ordered_cell(&mut e, false, (64, 0), 0x105, world);
        let interior = ordered_cell(&mut e, true, (0, 0), 0x105, Ptr::NULL);
        // Not a header.
        assert!(!e
            .call(0x0054_3df0, &args![exterior, Ptr::<()>::NULL, 1u8, 1u8])
            .bool());
        let bad = header(&mut e, 0, 0);
        e.mem.set_u32(bad.addr(), 9);
        assert!(!e.call(0x0054_3df0, &args![exterior, bad, 1u8, 1u8]).bool());

        // Type 0: only with the first flag. An interior asks the generic
        // code, an exterior its world space (virtual slot 0x110).
        let top = header(&mut e, 0, 0);
        returns(&mut e, FORM_BELONGS_IN_GROUP_INTERIOR, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_3df0, &args![interior, top, 1u8, 7u8]).bool());
        assert_eq!(
            calls_to(&e.call_log.take().unwrap(), FORM_BELONGS_IN_GROUP_INTERIOR),
            vec![vec![interior.addr(), top.addr(), 1, 7]]
        );
        e.call_log = Some(vec![]);
        assert!(e.call(0x0054_3df0, &args![exterior, top, 1u8, 7u8]).bool());
        assert_eq!(
            calls_to(&e.call_log.take().unwrap(), fake(0x110)),
            vec![vec![world.addr(), top.addr(), 1, 7]]
        );
        assert!(!e.call(0x0054_3df0, &args![exterior, top, 0u8, 7u8]).bool());
        assert!(!e.call(0x0054_3df0, &args![interior, top, 0u8, 7u8]).bool());

        // Type 1: an exterior cell whose world space contains the label.
        returns(&mut e, FORM_ID_IN_WORLD_SPACE, 1);
        assert!(belongs_to(&mut e, exterior, 1, 0x77, 1, 0));
        assert!(
            !belongs_to(&mut e, exterior, 1, 0x77, 0, 0),
            "needs the flag or persistence"
        );
        e.mem.set_u32(exterior.addr() + 8, 0x400);
        assert!(belongs_to(&mut e, exterior, 1, 0x77, 0, 0));
        e.mem.set_u32(exterior.addr() + 8, 0);
        assert!(!belongs_to(&mut e, interior, 1, 0x77, 1, 0));
        returns(&mut e, FORM_ID_IN_WORLD_SPACE, 0);
        assert!(!belongs_to(&mut e, exterior, 1, 0x77, 1, 0));

        // Types 2 and 4: the block key, only when the flag is set and the
        // cell is not persistent. Types 3 and 5: the sub-block key.
        let block_key = 2u32 << 16;
        let sub_key = 8u32 << 16;
        assert!(belongs_to(&mut e, exterior, 2, block_key, 1, 0));
        assert!(belongs_to(&mut e, exterior, 4, block_key, 1, 0));
        assert!(!belongs_to(&mut e, exterior, 4, block_key, 0, 0));
        assert!(!belongs_to(&mut e, exterior, 4, block_key + 1, 1, 0));
        assert!(belongs_to(&mut e, exterior, 3, sub_key, 0, 0));
        assert!(belongs_to(&mut e, exterior, 5, sub_key, 0, 0));
        assert!(!belongs_to(&mut e, exterior, 5, sub_key + 1, 0, 0));
        e.mem.set_u32(exterior.addr() + 8, 0x400);
        assert!(!belongs_to(&mut e, exterior, 2, block_key, 1, 0));
        assert!(!belongs_to(&mut e, exterior, 3, sub_key, 1, 0));
        // An unknown type.
        assert!(!belongs_to(&mut e, exterior, 6, 0, 1, 1));
    }

    #[test]
    fn cells_build_the_header_of_their_group() {
        let mut e = engine();
        e.set_global(EXTERIOR_PARENT_LABEL, 0xe0e0u32);
        e.set_global(INTERIOR_PARENT_LABEL, 0xa1a1u32);
        let world = world_with_key(&mut e, 1);
        e.mem.set_u32(world.addr() + 0xc, 0x3c);
        let exterior = ordered_cell(&mut e, false, (64, 0), 0x105, world);
        let interior = ordered_cell(&mut e, true, (0, 0), 0x105, Ptr::NULL);
        let words = |e: &Engine, out: Ptr| -> Vec<u32> {
            (0..5).map(|i| e.mem.u32(out.addr() + 4 * i)).collect()
        };
        let tag = 0x5055_5247u32;
        let build = |e: &mut Engine, cell: Ptr<TESObjectCELL>, parent: Ptr| -> Vec<u32> {
            let out = e.mem.alloc(0x14);
            e.mem.write(out, &[0xee; 0x14]);
            e.call(0x0054_3f50, &args![cell, Ptr::<()>::new(out), parent]);
            words(e, Ptr::new(out))
        };
        let block_key = 2u32 << 16;
        let sub_key = 8u32 << 16;

        // A null output is ignored (nothing to check but that it does not crash).
        e.call(
            0x0054_3f50,
            &args![exterior, Ptr::<()>::NULL, Ptr::<()>::NULL],
        );

        // Exterior: without a parent only the first word is cleared.
        let cleared = build(&mut e, exterior, Ptr::NULL);
        assert_eq!(cleared[0], 0);
        assert_eq!(cleared[1], 0xeeee_eeee);
        // World, block, sub-block groups.
        let top = header(&mut e, 0, 0xe0e0);
        assert_eq!(build(&mut e, exterior, top), vec![tag, 0, 0x3c, 1, 0]);
        let wrong = header(&mut e, 0, 0x1);
        assert_eq!(build(&mut e, exterior, wrong)[0], 0);
        let children = header(&mut e, 1, 0x3c);
        assert_eq!(
            build(&mut e, exterior, children),
            vec![tag, 0, block_key, 4, 0]
        );
        let other_world = header(&mut e, 1, 0x3d);
        assert_eq!(build(&mut e, exterior, other_world)[0], 0);
        let blocks = header(&mut e, 4, block_key);
        assert_eq!(build(&mut e, exterior, blocks), vec![tag, 0, sub_key, 5, 0]);
        let wrong_block = header(&mut e, 4, 1);
        assert_eq!(build(&mut e, exterior, wrong_block)[0], 0);
        // A persistent cell has no block group.
        e.mem.set_u32(exterior.addr() + 8, 0x400);
        assert_eq!(build(&mut e, exterior, children)[0], 0);
        assert_eq!(build(&mut e, exterior, blocks)[0], 0);
        // Other group types of the parent give nothing.
        let sub_blocks = header(&mut e, 5, sub_key);
        assert_eq!(build(&mut e, exterior, sub_blocks)[0], 0);

        // Interior: no parent gives the top-level interior group.
        assert_eq!(
            build(&mut e, interior, Ptr::NULL),
            vec![tag, 0, 0xa1a1, 0, 0]
        );
        let interior_top = header(&mut e, 0, 0xa1a1);
        // Id 0x105: block key 1, sub-block key 6.
        assert_eq!(build(&mut e, interior, interior_top), vec![tag, 0, 1, 2, 0]);
        let wrong_top = header(&mut e, 0, 0xa1a2);
        assert_eq!(build(&mut e, interior, wrong_top)[0], 0);
        let interior_blocks = header(&mut e, 2, 1);
        assert_eq!(
            build(&mut e, interior, interior_blocks),
            vec![tag, 0, 6, 3, 0]
        );
        let wrong_blocks = header(&mut e, 2, 2);
        assert_eq!(build(&mut e, interior, wrong_blocks)[0], 0);
        let sub = header(&mut e, 3, 6);
        assert_eq!(build(&mut e, interior, sub)[0], 0);
    }
}
