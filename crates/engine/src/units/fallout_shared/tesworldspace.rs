//! `fallout shared/tesworldspace.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is `TESWorldSpace` (a `TESForm` that owns a map of exterior
//! cells, the persistent cell, the terrain manager and the world-map data)
//! and the small `NiTMap`/`NiTPointerMap` instances it needs. It has 100
//! queue entries, `004fd380` and `00583360` to `00589420`.
//!
//! First session (40 functions, `004fd380` and `00583360` to `00586340`,
//! address order): the form record functions (`LoadPartial`, `Load`, `Save`,
//! `InitItem`, `Copy`, `Compare`, `CreateDuplicateForm`, the two
//! `SavesBefore`), the per-file cell offset tables (`FindCellInFile`,
//! `GetExtCellDataFromFileByEditorID`, `LoadCell`, `UnLoadCell`,
//! `FindLandDataInFile`) and the inherited accessors (climate, image space,
//! water, terrain manager, the `cFlags` bits). The next session continues at
//! `00586390` (`fn_00586390`: the parent world space if the child uses the
//! parent's value `n`).
//!
//! Notes for the next session:
//! - Layout. `TESWorldSpace` is 0xEC bytes on PC (`TESForm` is 0x18, so
//!   every Xbox PDB offset after it is 0x10 lower), and the PC class has an
//!   editor ID string (`BSStringT`) at `+0xC0` that the Xbox class does not
//!   have, so the fields from `fDefaultLandHeight` on are only 8 lower. The
//!   engine map names `005f36f0` `ActorMover::GetPreferredMoveMode`; its body
//!   is `return *(this + 0x34)` and the world space code uses it for
//!   `pPersistentCell`. `0084e3a0` is `return *(this + 0xC)`: the form ID of a
//!   form, and the item count of an `NiTPointerMap`.
//! - Functions of this file that come after the first 40 (so they are not
//!   translated yet) are called by address: `fn_00586390` is `00586390`
//!   (see above), `00587670` `AddCell`, `005875a0` `GetCellFromCellCoord`,
//!   `00588150` `AssignPersistentRefsToCell`, `00588120` (called last by
//!   `CreateDuplicateForm`), `005863d0` (sets `pParentWorld`), `00588a90`
//!   (the `OFFSET_DATA` of a file, below), `00588b00` `CreateOffsetData`,
//!   `00587760` `ReleaseCell`, `00589000` (the `NiTPointerMap<int,
//!   TESObjectCELL *>` constructor, takes the bucket count).
//! - `OFFSET_DATA` (Xbox PDB `TESWorldSpace::OFFSET_DATA`, 0x18 bytes) is the
//!   per-plugin table of file offsets of the exterior cell records:
//!   `pCellFileOffsets` (+0), `OffsetMinCoords` (+4, two floats) and
//!   `OffsetMaxCoords` (+0xC) bound the grid (the code converts the floats
//!   with `_ftol2` and shifts right by 12 to get cell coordinates) and
//!   `iFileOffset` (+0x14) is the offset to add to the table entries.
//! - The plugin reader's record header (`TESFile + 0x240`, `00462270`
//!   returns its address) starts with a tag word, the size at `+4`, flags at
//!   `+8` and the form ID at `+0xC`; for a `GRUP` header (tag at
//!   `0x01187020`) `+8` is the label and `+0xC` the group type. The tags the
//!   unit compares against are initialized in the exe's data.
//! - x87: the game computes in extended precision and stores `float`
//!   results; here the one place a float is computed (the coordinates
//!   `min`/`max` calls) is done by the callee `0040ebd0`/`00404010`.
//!
//! Not translated: the compiler's exception-unwinding frames (`FS:[0]`
//! chains and state variables) of `Load`, `InitItem`, `Copy`,
//! `LoadCell`, `CreateDuplicateForm`, the stack-cookie check of `Load`, and
//! the `_alloca_probe_16` of `Load` (a stack block of the engine here).

#[allow(unused_imports)]
use crate::prelude::*;

/// A four-character chunk tag as the record reader compares it (the bytes
/// in file order, read as a little-endian word).
const fn tag(name: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*name)
}

// ---- Layouts ---------------------------------------------------------------

layout! {
    /// `TESWorldSpace` (Xbox PDB), 0xEC bytes on PC (the Xbox class is 0xF4;
    /// see the module notes for the offsets). Only the fields translated
    /// code uses are listed.
    pub struct TESWorldSpace: 0xEC {
        /// `pCellMap` (Xbox PDB): `NiTPointerMap<int, TESObjectCELL *> *`.
        0x30 pCellMap: Ptr,
        /// `pPersistentCell` (Xbox PDB): `TESObjectCELL *`.
        0x34 pPersistentCell: Ptr,
        /// `pTerrainManager` (Xbox PDB): `BGSTerrainManager *`.
        0x3C pTerrainManager: Ptr,
        /// `pClimate` (Xbox PDB): `TESClimate *`. Holds the form ID until
        /// `InitItem` resolves it.
        0x40 pClimate: Ptr,
        /// `pImageSpace` (Xbox PDB): `TESImageSpace *` (form ID until
        /// `InitItem`).
        0x44 pImageSpace: Ptr,
        /// `pImpactSwap` (Xbox PDB): `ImpactSwap *`.
        0x48 pImpactSwap: Ptr,
        /// `cFlags` (Xbox PDB): bit 0 (no cells of its own), bit 1, bit 3
        /// (`GetHasBorderRegion`), bits 4, 5, 6 (inverted by `00586320`) and 7
        /// are read by the accessors translated here.
        0x4C cFlags: u8,
        /// `sParentUseFlags` (Xbox PDB): bit `n` says the world space uses
        /// its parent's value `n`.
        0x4E sParentUseFlags: u16,
        /// `pParentWorld` (Xbox PDB): `TESWorldSpace *` (form ID until
        /// `InitItem`).
        0x70 pParentWorld: Ptr,
        /// `pWorldWater` (Xbox PDB): `TESWaterForm *`.
        0x74 pWorldWater: Ptr,
        /// `pLODWater` (Xbox PDB): `TESWaterForm *`.
        0x78 pLODWater: Ptr,
        /// `fLODWaterHeight` (Xbox PDB).
        0x7C fLODWaterHeight: f32,
        /// `WorldMapData` (Xbox PDB): a [`WorldMapData`].
        0x80 WorldMapData: Inline<WorldMapData>,
        /// `WorldMapOffsetData` (Xbox PDB): a [`WorldMapOffsetData`].
        0x90 WorldMapOffsetData: Inline<WorldMapOffsetData>,
        /// `pMusicType` (Xbox PDB): `BGSMusicType *` (form ID until
        /// `InitItem`).
        0x9C pMusicType: Ptr,
        /// `MinimumCoords` (Xbox PDB, `NiPoint2`): x.
        0xA0 MinimumCoords_x: f32,
        /// `MinimumCoords`: y.
        0xA4 MinimumCoords_y: f32,
        /// `MaximumCoords` (Xbox PDB, `NiPoint2`): x.
        0xA8 MaximumCoords_x: f32,
        /// `MaximumCoords`: y.
        0xAC MaximumCoords_y: f32,
        /// PC only: the editor ID, a `BSStringT` (the form's name getter,
        /// vtable slot `+0x130`, returns its text).
        0xC0 cEditorID: Inline<crate::types::BSStringT>,
        /// `fDefaultLandHeight` (Xbox PDB).
        0xC8 fDefaultLandHeight: f32,
        /// `fDefaultWaterHeight` (Xbox PDB).
        0xCC fDefaultWaterHeight: f32,
        /// `pEncounterZone` (Xbox PDB): `BGSEncounterZone *` (form ID until
        /// `InitItem`).
        0xD0 pEncounterZone: Ptr,
        /// `CanopyShadowTexture` (Xbox PDB): a `TESTexture`, 0xC bytes.
        0xD4 CanopyShadowTexture: u32,
        /// `WaterNoiseTexture` (Xbox PDB): a `TESTexture`, 0xC bytes.
        0xE0 WaterNoiseTexture: u32,
    }

    /// `WORLD_MAP_DATA` (Xbox PDB), 0x10 bytes.
    pub struct WorldMapData: 0x10 {
        /// `iUsableWidth` (Xbox PDB).
        0x00 iUsableWidth: u32,
        /// `iUsableHeight` (Xbox PDB).
        0x04 iUsableHeight: u32,
        /// `sNWCellX` (Xbox PDB).
        0x08 sNWCellX: i16,
        /// `sNWCellY` (Xbox PDB).
        0x0A sNWCellY: i16,
        /// `sSECellX` (Xbox PDB).
        0x0C sSECellX: i16,
        /// `sSECellY` (Xbox PDB).
        0x0E sSECellY: i16,
    }

    /// `WORLD_MAP_OFFSET_DATA` (Xbox PDB), 0xC bytes.
    pub struct WorldMapOffsetData: 0x0C {
        /// `fMapScale` (Xbox PDB).
        0x00 fMapScale: f32,
        /// `fMapOffsetX` (Xbox PDB).
        0x04 fMapOffsetX: f32,
        /// `fMapOffsetY` (Xbox PDB).
        0x08 fMapOffsetY: f32,
    }

    /// `TESWorldSpace::OFFSET_DATA` (Xbox PDB), 0x18 bytes: the table of
    /// exterior cell record offsets one plugin file has for a world space.
    pub struct OffsetData: 0x18 {
        /// `pCellFileOffsets` (Xbox PDB): `u32 *`, one offset per cell of
        /// the grid (0 = none), row by row.
        0x00 pCellFileOffsets: Ptr,
        /// `OffsetMinCoords.x` (Xbox PDB, `NiPoint2` at +4).
        0x04 OffsetMinCoords_x: f32,
        /// `OffsetMinCoords.y`.
        0x08 OffsetMinCoords_y: f32,
        /// `OffsetMaxCoords.x` (Xbox PDB, `NiPoint2` at +0xC).
        0x0C OffsetMaxCoords_x: f32,
        /// `OffsetMaxCoords.y`.
        0x10 OffsetMaxCoords_y: f32,
        /// `iFileOffset` (Xbox PDB): added to the table entries.
        0x14 iFileOffset: u32,
    }
}

// ---- Callees ---------------------------------------------------------------

/// `MemoryManager::Allocate` (`00401000`) and `Deallocate` (`00401030`),
/// `__cdecl`.
const MEMORY_ALLOC: u32 = 0x0040_1000;
const MEMORY_FREE: u32 = 0x0040_1030;
/// `memset(ptr, value, size)` (`00403d30`) and `memcpy(dest, source, size)`
/// (`00401460`), `__cdecl`.
const MEMSET: u32 = 0x0040_3d30;
const MEMCPY: u32 = 0x0040_1460;
/// `memcmp(a, b, size)` (`00ec4835`, LIBCMT).
const MEMCMP: u32 = 0x00ec_4835;
/// `_ftol2_sse` (`00ec62c0`): truncates the float in ST0 to an integer in
/// EAX; the uniform form passes the value as an `f64` argument.
const FTOL: u32 = 0x00ec_62c0;
/// `__RTDynamicCast(object, 0, TESForm type, target type, 0)` (`00ec43fb`).
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `float min(a, b)` (`0040ebd0`) and `float max(a, b)` (`00404010`),
/// results in ST0.
const FLOAT_MIN: u32 = 0x0040_ebd0;
const FLOAT_MAX: u32 = 0x0040_4010;
/// Endian swaps of a 4-byte (`00401080`) and a 2-byte (`00407a90`) value in
/// place, `__cdecl(pointer, byte)`; the byte is always 0 here.
const SWAP_DWORD: u32 = 0x0040_1080;
const SWAP_WORD: u32 = 0x0040_7a90;
/// `__thiscall(this)` endian swaps of the two floats of the cell offset
/// block (`00462230`, used on the `DNAM` heights and on the cell grid
/// block) and of the 12-byte map offset block (`005a6b10`).
const SWAP_PAIR: u32 = 0x0046_2230;
const SWAP_MAP_OFFSET_BLOCK: u32 = 0x005a_6b10;
/// Whether the game is saving big-endian data (`00401500`, a global byte).
const SAVING_SWAPPED: u32 = 0x0040_1500;
/// `BSSimpleList` head item / `this` returned unchanged by the folded
/// constructors of the small local structures (`006815c0`, `00540720`).
const LOCAL_STRUCT_CONSTRUCT: u32 = 0x0068_15c0;
const CELL_COORDS_CONSTRUCT: u32 = 0x0054_0720;

// `TESFile` (tesfile.cpp), `__thiscall` on the file.
/// `GetTESForm`: the record type of the current form.
const FILE_RECORD_TYPE: u32 = 0x0047_2660;
/// `GetMaster`: whether the file is a master file.
const FILE_IS_MASTER: u32 = 0x0047_1c20;
/// `GetTESChunk`: the tag of the current chunk, 0 at the end.
const FILE_NEXT_CHUNK: u32 = 0x0047_26b0;
/// Moves to the next chunk; `false` when there is none (`004726f0`).
const FILE_ADVANCE_CHUNK: u32 = 0x0047_26f0;
/// `GetChunkData(buffer, size)` (`00472890`).
const FILE_READ_CHUNK: u32 = 0x0047_2890;
/// `GetChunkData(u32 *)` (`004727f0`) and `GetChunkData(u16 *)`
/// (`00472840`): read one value of the chunk.
const FILE_READ_CHUNK_U32: u32 = 0x0047_27f0;
const FILE_READ_CHUNK_U16: u32 = 0x0047_2840;
/// The size of the current chunk (`00401660`, a word at `+0x25C`).
const FILE_CHUNK_SIZE: u32 = 0x0040_1660;
/// Whether the file's data is byte-swapped (`00401680`, the byte at
/// `+0x299`).
const FILE_NEEDS_SWAP: u32 = 0x0040_1680;
/// `SetOffset(offset)`: positions the reader on a record.
const FILE_SET_OFFSET: u32 = 0x0047_23a0;
/// `FindForm(form)` (`004734d0`).
const FILE_FIND_FORM: u32 = 0x0047_34d0;
/// `NextForm(1)` (`00472150`).
const FILE_NEXT_FORM: u32 = 0x0047_2150;
/// The address of the current record header (`00462270`, `file + 0x240`).
const FILE_CURRENT_RECORD: u32 = 0x0046_2270;
/// The word at `file + 0x264` (`00467bb0`): the offset of the current form
/// data, stored in `OFFSET_DATA::iFileOffset` and returned as the land
/// data's position.
const FILE_CURRENT_OFFSET: u32 = 0x0046_7bb0;
/// The form ID of the current record (`008d8ac0`, the word at
/// `file + 0x24C`).
const FILE_CURRENT_FORM_ID: u32 = 0x008d_8ac0;
/// Leaves the current `GRUP` (`00473660`, resets the group state and
/// calls `004721d0`) and `00472000` (rewinds the reader to the start of the
/// current record).
const FILE_LEAVE_GROUP: u32 = 0x0047_3660;
const FILE_REWIND_RECORD: u32 = 0x0047_2000;
/// `TESFile::GetThreadSafeFile` (`004739b0`).
const FILE_THREAD_SAFE: u32 = 0x0047_39b0;

// `TESForm`.
/// `GetFile(index)` (`00484e60`; -1 for the last file).
const FORM_GET_FILE: u32 = 0x0048_4e60;
/// `AddCompileIndex(&form id, file)` (`00485d50`).
const FORM_ADD_COMPILE_INDEX: u32 = 0x0048_5d50;
/// The form with a given ID (`004839c0`).
const FORM_BY_ID: u32 = 0x0048_39c0;
/// The word at `+0xC` (`0084e3a0`): a form's ID, a map's item count.
const WORD_AT_0C: u32 = 0x0084_e3a0;
/// The record type byte of a form (`00401170`, `+4`).
const FORM_TYPE: u32 = 0x0040_1170;
/// The flags word of a form (`0044ddc0`, `+8`).
const FORM_FLAGS: u32 = 0x0044_ddc0;
/// The form flag 8 test (`004013e0`).
const FORM_FLAG_8: u32 = 0x0040_13e0;
/// `TESForm::LoadForm(file)` (`00485110`).
const FORM_LOAD: u32 = 0x0048_5110;
/// `00484ab0(flag)`: a form state setter called with 0 by `Load` and 1 by
/// `InitItem`.
const FORM_SET_STATE: u32 = 0x0048_4ab0;
/// `00484e40`: a form query that picks the cell map size in `Load`.
const FORM_QUERY_LARGE_MAP: u32 = 0x0048_4e40;
/// `TESForm::CompareAllComponents(other)` (`00485270`) and
/// `TESForm::CopyAllComponents(source)` (`004851b0`).
const FORM_COMPARE_COMPONENTS: u32 = 0x0048_5270;
const FORM_COPY_COMPONENTS: u32 = 0x0048_51b0;
/// `TESForm::CreateDuplicateForm(flag, other)` (`004867a0`).
const FORM_CREATE_DUPLICATE: u32 = 0x0048_67a0;
/// `TESForm::SavesBefore(group header)` base version (`00484150`) and the
/// default `SavesBefore(form)` (`00484020`).
const FORM_SAVES_BEFORE_HEADER: u32 = 0x0048_4150;
const FORM_SAVES_BEFORE_FORM: u32 = 0x0048_4020;
/// `StartForm` (`004855a0`), `CloseForm` (`00485680`).
const FORM_START: u32 = 0x0048_55a0;
const FORM_CLOSE: u32 = 0x0048_5680;
/// `AddChunk` helpers of the save record writer, `__cdecl`: a 4-byte value
/// (`00485910`), a 2-byte value (`00485950`), a byte (`004858f0`), a block
/// (`00485990`, `004856f0`) and an array of items (`00485710`).
const ADD_CHUNK_U32: u32 = 0x0048_5910;
const ADD_CHUNK_U16: u32 = 0x0048_5950;
const ADD_CHUNK_U8: u32 = 0x0048_58f0;
const ADD_CHUNK_DATA: u32 = 0x0048_5990;
const ADD_CHUNK_BLOCK: u32 = 0x0048_56f0;
const ADD_CHUNK_ARRAY: u32 = 0x0048_5710;

/// `TESFullName::Save` (`00487010`) and `TESFullName::Load` (`00487050`).
const FULL_NAME_SAVE: u32 = 0x0048_7010;
const FULL_NAME_LOAD: u32 = 0x0048_7050;
/// `TESTexture::Save(tag)` (`0048e370`), `LoadTextureChunk(texture, file)`
/// (`0048e3d0`) and `SetSoundFile(path)` (`00489100`, the folded texture
/// name setter), `GetNameLength` (`0048cee0`) and the text getter
/// (`00408da0`, which returns a default empty string when there is none).
const TEXTURE_SAVE: u32 = 0x0048_e370;
const TEXTURE_LOAD_CHUNK: u32 = 0x0048_e3d0;
const TEXTURE_SET_NAME: u32 = 0x0048_9100;
const TEXTURE_NAME_LENGTH: u32 = 0x0048_cee0;
const TEXTURE_NAME_TEXT: u32 = 0x0040_8da0;

/// Setters of fields this class owns, translated in other units (folded
/// identical code): climate (`0087ce80`, `+0x40`), image space (`008d8040`,
/// `+0x44`), water (`004febb0`, `+0x74`), lod water (`00442a80`, `+0x78`)
/// and music type (`00810570`, `+0x9C`); and their getter for `+0x44`
/// (`008041a0`).
const SET_CLIMATE: u32 = 0x0087_ce80;
const SET_IMAGE_SPACE: u32 = 0x008d_8040;
const SET_WATER: u32 = 0x004f_ebb0;
const SET_LOD_WATER: u32 = 0x0044_2a80;
const SET_MUSIC_TYPE: u32 = 0x0081_0570;
const GET_IMAGE_SPACE_RAW: u32 = 0x0080_41a0;
/// Setter of `pParentWorld` (`005863d0`, a later function of this file).
const SET_PARENT_WORLD: u32 = 0x0058_63d0;
/// `fn_00586390(this, n)`: the parent world space if this world space uses
/// the parent's value `n`, else 0 (a later function of this file).
const PARENT_FOR: u32 = 0x0058_6390;
/// `fLODWaterHeight` getter (`0045cd80`, a float in ST0).
const GET_LOD_WATER_HEIGHT: u32 = 0x0045_cd80;
/// The `OFFSET_DATA` of a file (`00588a90`, a later function of this
/// file), `CreateOffsetData(file)` (`00588b00`).
const OFFSET_DATA_OF_FILE: u32 = 0x0058_8a90;
const CREATE_OFFSET_DATA: u32 = 0x0058_8b00;
/// `AddCell(cell)` (`00587670`), `GetCellFromCellCoord(x, y)` (`005875a0`),
/// `ReleaseCell(cell)` (`00587760`), `AssignPersistentRefsToCell(cell)`
/// (`00588150`) and `00588120` (called last by `CreateDuplicateForm`).
const ADD_CELL: u32 = 0x0058_7670;
const GET_CELL_FROM_CELL_COORD: u32 = 0x0058_75a0;
const RELEASE_CELL: u32 = 0x0058_7760;
const ASSIGN_PERSISTENT_REFS: u32 = 0x0058_8150;
const FINISH_DUPLICATE: u32 = 0x0058_8120;
/// `ImpactSwap` (impactswap.cpp, 0x15C bytes): constructor (`0058efd0`),
/// destructor with delete flag (`0040f2a0`), `InitItem(world space)`
/// (`0058f210`), `Load` (`0058f180`), `Copy` (`0058f4c0`), `Compare`
/// (`0058f5c0`), `Save` (`0058f080`).
const IMPACT_SWAP_SIZE: u32 = 0x15C;
const IMPACT_SWAP_CONSTRUCT: u32 = 0x0058_efd0;
const IMPACT_SWAP_DESTRUCT: u32 = 0x0040_f2a0;
const IMPACT_SWAP_INIT_ITEM: u32 = 0x0058_f210;
const IMPACT_SWAP_LOAD: u32 = 0x0058_f180;
const IMPACT_SWAP_COPY: u32 = 0x0058_f4c0;
const IMPACT_SWAP_COMPARE: u32 = 0x0058_f5c0;
const IMPACT_SWAP_SAVE: u32 = 0x0058_f080;
/// `BGSTerrainManager` constructor (`006fc490`, takes the world space), 0x3C
/// bytes.
const TERRAIN_MANAGER_SIZE: u32 = 0x3C;
const TERRAIN_MANAGER_CONSTRUCT: u32 = 0x006f_c490;
/// `NiTPointerMap<int, TESObjectCELL *>` (0x10 bytes) constructor with its
/// bucket count (`00589000`), and the map operations `GetFirstPos`
/// (`004b9ba0`), `GetNext(&pos, &key, &value)` (`006b7f20`), `SetAt(key,
/// value)` (`00844700`) and `RemoveAll` (`00438af0`).
const CELL_MAP_SIZE: u32 = 0x10;
const CELL_MAP_CONSTRUCT: u32 = 0x0058_9000;
const MAP_FIRST_POS: u32 = 0x004b_9ba0;
const MAP_GET_NEXT: u32 = 0x006b_7f20;
const MAP_SET_AT: u32 = 0x0084_4700;
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;
/// Hash sizes `Load` gives the cell map: 0x1B59 when `00484e40` is true,
/// else 0x2BD.
const LARGE_CELL_MAP_BUCKETS: u32 = 0x1b59;
const SMALL_CELL_MAP_BUCKETS: u32 = 0x02bd;
/// The persistent cell getter (`005f36f0`, the folded `return *(this +
/// 0x34)`).
const GET_PERSISTENT_CELL: u32 = 0x005f_36f0;
/// `TESObjectCELL` (tesobjectcell.cpp), 0xE0 bytes: constructor
/// (`005415b0`), `GetWorldSpace` (`0054ddd0`), `SetInterior(flag)`
/// (`00544300`), `CreateCellData` (`00544630`), `SetDataCoord(x, y)`
/// (`00544c90`), `LoadTempDataFromFile(file)` (`00550500`), the cell
/// cleanup (`005508b0`), the temp-data query (`00551420`),
/// `SetHasTempData(flag)` (`00551440`), `FindLandDataInFile(file)`
/// (`00550f60`, `__cdecl`), the file count of a form (`005504e0`) and the
/// block keys `CalcExtGroupBlockKey(x, y)` (`00544280`) and
/// `CalcExtGroupSubBlockKey(x, y)` (`005442c0`), `__cdecl`.
const CELL_SIZE: u32 = 0xE0;
const CELL_CONSTRUCT: u32 = 0x0054_15b0;
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
const CELL_SET_INTERIOR: u32 = 0x0054_4300;
const CELL_CREATE_DATA: u32 = 0x0054_4630;
const CELL_SET_DATA_COORD: u32 = 0x0054_4c90;
const CELL_LOAD_TEMP_DATA: u32 = 0x0055_0500;
const CELL_CLEANUP: u32 = 0x0055_08b0;
const CELL_HAS_TEMP_DATA: u32 = 0x0055_1420;
const CELL_SET_HAS_TEMP_DATA: u32 = 0x0055_1440;
const CELL_FIND_LAND_DATA_IN_FILE: u32 = 0x0055_0f60;
const FORM_FILE_COUNT: u32 = 0x0055_04e0;
const CALC_BLOCK_KEY: u32 = 0x0054_4280;
const CALC_SUB_BLOCK_KEY: u32 = 0x0054_42c0;
/// `TESDataHandler::LoadForm(form, file)` (`004601d0`, `__cdecl`).
const DATA_HANDLER_LOAD_FORM: u32 = 0x0046_01d0;
/// The `TESSaveLoadGame` singleton (`0x011de45c`), the `this` of
/// `GetCreatedExteriorCellFormID(world form ID, x, y)` (`00861640`, the form
/// ID the save game made for an exterior cell, 0 for none) and of three
/// functions whose bodies do nothing: `008d0370(cell)` returns 0,
/// `0047c850()` returns 0 and `004534f0(state)` ignores its argument.
/// `LoadCell` still calls them (the last two around the cell's `InitItem`).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const SAVE_LOAD_CREATED_CELL_ID: u32 = 0x0086_1640;
const SAVE_LOAD_NOTE_CELL: u32 = 0x008d_0370;
const SAVE_LOAD_GET_STATE: u32 = 0x0047_c850;
const SAVE_LOAD_SET_STATE: u32 = 0x0045_34f0;
/// The data loader state (`0x011c3f2c`), the `this` of `00469860(form ID)`
/// (true for a form ID of 0xFF000000 or above, a runtime-created form), and
/// the game loader (`0x011ddf38`) with its `UnloadForm(cell, 0)`
/// (`00849730`).
const LOADER_STATE: u32 = 0x011c_3f2c;
const LOADER_STATE_QUERY: u32 = 0x0046_9860;
const GAME_LOADER: u32 = 0x011d_df38;
const GAME_LOADER_UNLOAD_FORM: u32 = 0x0084_9730;
/// The world space being loaded, a global the cell code reads
/// (`0x011c3f34`).
const CURRENT_WORLD_SPACE: u32 = 0x011c_3f34;
/// The default water form, a global pointer (`0x011ca53c`).
const DEFAULT_WATER: u32 = 0x011c_a53c;
/// The case-insensitive string compare used on editor IDs (`00404dc0`).
const EDITOR_ID_COMPARE: u32 = 0x0040_4dc0;
/// The memory-context guard of the game's stack frame: constructor
/// `__thiscall(guard, context, 1, source file name, source line)`
/// (`00404eb0`) and destructor (`00404ee0`), and the source file name.
const MEMORY_CONTEXT_ENTER: u32 = 0x0040_4eb0;
const MEMORY_CONTEXT_LEAVE: u32 = 0x0040_4ee0;
const WORLD_SPACE_SOURCE: u32 = 0x0103_1f18;
/// The game log function (`005b5e40`, `__cdecl`, variadic).
const LOG: u32 = 0x005b_5e40;
/// The form's editor ID: `00474cb0` returns its length (0 inside a
/// recursive call), and slot `+0x130` returns the text.
const EDITOR_ID_LENGTH: u32 = 0x0047_4cb0;
const SLOT_EDITOR_ID: u32 = 0x130;
const SLOT_SET_EDITOR_ID: u32 = 0x134;

// Globals: record tags initialized in the exe's data.
/// The `GRUP` tag word of group headers.
const GROUP_TAG: u32 = 0x0118_7020;
/// The tag word of cell records.
const CELL_TAG: u32 = 0x0118_72b4;
/// The tag word of a record the cell scan skips whole (`01187338`).
const SKIPPED_RECORD_TAG_01187338: u32 = 0x0118_7338;
/// The record type byte of a world space record (`0118730c`).
const WORLD_SPACE_RECORD_TYPE: u32 = 0x0118_730c;

// RTTI type descriptors (`.?AV<name>@@`), read in the exe.
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_OBJECT_CELL: u32 = 0x0118_3fb4;
const TYPE_TES_WORLD_SPACE: u32 = 0x0118_3fd0;
const TYPE_TES_CHILD_CELL: u32 = 0x0118_ac2c;
const TYPE_TES_CLIMATE: u32 = 0x0118_4170;
const TYPE_TES_IMAGE_SPACE: u32 = 0x0118_4154;
const TYPE_BGS_ENCOUNTER_ZONE: u32 = 0x0118_40dc;
const TYPE_TES_WATER_FORM: u32 = 0x0118_4118;
const TYPE_BGS_MUSIC_TYPE: u32 = 0x0118_40fc;

// Form type and form flag values.
/// Record type of a world space (`'A'`).
const RECORD_TYPE_WORLD_SPACE: u32 = 0x41;
/// Record type of a cell (`0x39`).
const RECORD_TYPE_CELL: u32 = 0x39;

// Chunk tags of the world space record.
const CHUNK_CLIMATE: u32 = tag(b"CNAM");
const CHUNK_DATA: u32 = tag(b"DATA");
const CHUNK_LOD_WATER: u32 = tag(b"NAM3");
const CHUNK_MIN_COORDS: u32 = tag(b"NAM0");
const CHUNK_WATER: u32 = tag(b"NAM2");
const CHUNK_LOD_WATER_HEIGHT: u32 = tag(b"NAM4");
const CHUNK_MAX_COORDS: u32 = tag(b"NAM9");
const CHUNK_IMPACT_FIRST: u32 = tag(b"IMPF");
const CHUNK_IMPACT_SECOND: u32 = tag(b"IMPS");
const CHUNK_EDITOR_ID: u32 = tag(b"EDID");
const CHUNK_OBJECT_BOUNDS: u32 = tag(b"OBND");
const CHUNK_FULL_NAME: u32 = tag(b"FULL");
const CHUNK_ICON: u32 = tag(b"ICON");
const CHUNK_ENCOUNTER_ZONE: u32 = tag(b"XEZN");
const CHUNK_OFFSETS: u32 = tag(b"OFST");
const CHUNK_DEFAULT_HEIGHTS: u32 = tag(b"DNAM");
const CHUNK_IMAGE_SPACE: u32 = tag(b"INAM");
const CHUNK_MAP_DATA: u32 = tag(b"MNAM");
const CHUNK_CANOPY_TEXTURE: u32 = tag(b"NNAM");
const CHUNK_MAP_OFFSET_DATA: u32 = tag(b"ONAM");
const CHUNK_PARENT_USE_FLAGS: u32 = tag(b"PNAM");
const CHUNK_SKIPPED: u32 = tag(b"SNAM");
const CHUNK_PARENT_WORLD: u32 = tag(b"WNAM");
const CHUNK_WATER_NOISE_TEXTURE: u32 = tag(b"XNAM");
const CHUNK_MUSIC_TYPE: u32 = tag(b"ZNAM");
/// The cell record's grid position chunk (`XCLC`).
const CHUNK_CELL_GRID: u32 = tag(b"XCLC");

// Log message formats (in the exe's data).
const MSG_CLIMATE_NAMED: u32 = 0x0103_1e78;
const MSG_CLIMATE_ID: u32 = 0x0103_1e30;
const MSG_IMAGE_SPACE_NAMED: u32 = 0x0103_1de8;
const MSG_IMAGE_SPACE_ID: u32 = 0x0103_1d98;
const MSG_ENCOUNTER_ZONE_NAMED: u32 = 0x0103_1d54;
const MSG_ENCOUNTER_ZONE_ID: u32 = 0x0103_1d10;
const MSG_WATER_NAMED: u32 = 0x0103_1cc8;
const MSG_WATER_ID: u32 = 0x0103_1c78;
const MSG_LOD_WATER_NAMED: u32 = 0x0103_1c28;
const MSG_LOD_WATER_ID: u32 = 0x0103_1bd8;
const MSG_MUSIC_NAMED: u32 = 0x0103_1b90;
const MSG_MUSIC_ID: u32 = 0x0103_1b40;
const MSG_PARENT_NAMED: u32 = 0x0103_1af0;
const MSG_PARENT_ID: u32 = 0x0103_1aa0;
const MSG_CELL_LOAD_FAILED: u32 = 0x0103_1ec0;

// ---- Helpers ---------------------------------------------------------------

/// `TESForm::GetFile(index)` of a form.
fn form_get_file(e: &mut Engine, form: u32, index: u32) -> Ptr {
    e.call(FORM_GET_FILE, &args![form, index]).ptr()
}

/// `TESFile::GetThreadSafeFile` of the file `GetFile` returned.
fn thread_safe_file(e: &mut Engine, file: Ptr) -> Ptr {
    e.call(FILE_THREAD_SAFE, &args![file]).ptr()
}

/// `_ftol2_sse` on a `float`.
fn float_to_int(e: &mut Engine, value: f32) -> i32 {
    e.call(FTOL, &args![value as f64]).i32()
}

/// `__RTDynamicCast(form, 0, TESForm, target, 0)`.
fn dynamic_cast(e: &mut Engine, form: u32, target: u32) -> u32 {
    e.call(
        RT_DYNAMIC_CAST,
        &args![form, 0u32, TYPE_TES_FORM, target, 0u32],
    )
    .u32()
}

/// The `OFFSET_DATA` of a file for this world space (`00588a90`).
fn offset_data_of_file(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr) -> Ptr<OffsetData> {
    e.call(OFFSET_DATA_OF_FILE, &args![this, file]).ptr()
}

/// The four cell coordinate bounds of an `OFFSET_DATA` (minimum x, minimum
/// y, maximum x, maximum y), each `_ftol2` of the float shifted right by 12,
/// converted in this order.
fn offset_data_bounds(e: &mut Engine, data: Ptr<OffsetData>) -> (i32, i32, i32, i32) {
    let min_x = e.get(data, OffsetData::OffsetMinCoords_x);
    let min_x = float_to_int(e, min_x) >> 12;
    let min_y = e.get(data, OffsetData::OffsetMinCoords_y);
    let min_y = float_to_int(e, min_y) >> 12;
    let max_x = e.get(data, OffsetData::OffsetMaxCoords_x);
    let max_x = float_to_int(e, max_x) >> 12;
    let max_y = e.get(data, OffsetData::OffsetMaxCoords_y);
    let max_y = float_to_int(e, max_y) >> 12;
    (min_x, min_y, max_x, max_y)
}

/// Swaps one 4-byte value of the loaded data in place.
fn swap_dword(e: &mut Engine, at: u32) {
    e.call(SWAP_DWORD, &args![at, 0u32]);
}

/// Swaps one 2-byte value of the loaded data in place.
fn swap_word(e: &mut Engine, at: u32) {
    e.call(SWAP_WORD, &args![at, 0u32]);
}

/// The log call every unresolved form reference makes.
fn log_unresolved(
    e: &mut Engine,
    this: Ptr<TESWorldSpace>,
    id: u32,
    message_named: u32,
    message_id: u32,
) {
    if e.call(EDITOR_ID_LENGTH, &args![this]).u32() != 0 {
        let name = e.vcall(this.addr(), SLOT_EDITOR_ID, &args![]).u32();
        e.call(LOG, &args![message_named, id, name]);
    } else {
        let own_id = e.call(WORD_AT_0C, &args![this]).u32();
        e.call(LOG, &args![message_id, id, own_id]);
    }
}

/// Resolves the form ID held in a field of the world space (`InitItem`):
/// makes it a load order ID (`AddCompileIndex` with the form's last file),
/// finds the form, casts it to `target`, and logs when the cast is null.
fn resolve_form_reference(
    e: &mut Engine,
    this: Ptr<TESWorldSpace>,
    stored_id: u32,
    target: u32,
    message_named: u32,
    message_id: u32,
) -> u32 {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), stored_id);
        let file = form_get_file(e, this.addr(), 0xffff_ffff);
        e.call(FORM_ADD_COMPILE_INDEX, &args![local, file]);
        let id = e.mem.u32(local.addr());
        let form = e.call(FORM_BY_ID, &args![id]).u32();
        let cast = dynamic_cast(e, form, target);
        if cast == 0 {
            log_unresolved(e, this, id, message_named, message_id);
        }
        cast
    })
}

// ---- Functions ---------------------------------------------------------------

// Translated from 004fd380 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::GetParentWorldSimple` (Xbox PDB): the parent world space
/// pointer, whatever the parent-use flags say.
pub fn tes_world_space_get_parent_world_simple(e: &mut Engine, this: Ptr<TESWorldSpace>) -> Ptr {
    e.get(this, TESWorldSpace::pParentWorld)
}

// Translated from 00583360 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::LoadPartial` (Xbox PDB, vtable slot `+0x24`): reads the
/// `NAM0` / `NAM9` chunks, which hold the south-west and north-east corners
/// of the world. The corner of this world space is widened to take in the
/// file's (minimum / maximum per axis), and for a master file the file's own
/// corners are kept in its `OFFSET_DATA`. Always true.
pub fn tes_world_space_load_partial(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr) -> bool {
    loop {
        let chunk = e.call(FILE_NEXT_CHUNK, &args![file]).u32();
        if chunk == 0 {
            return true;
        }
        if chunk == CHUNK_MIN_COORDS || chunk == CHUNK_MAX_COORDS {
            load_corner_chunk(e, this, file, chunk);
        }
        if !e.call(FILE_ADVANCE_CHUNK, &args![file]).bool() {
            return true;
        }
    }
}

// Translated from 00583560 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::Load` (Xbox PDB, vtable slot `+0x20`): reads a world
/// space record (type `'A'`) from the plugin `file`, chunk by chunk.
///
/// - For a master file the `OFFSET_DATA` of the file gets the record's
///   position (`iFileOffset`), and `NAM0`, `NAM9` (see `LoadPartial`) and
///   `OFST` (the table of cell record offsets, byte-swapped when the file
///   is) fill the rest.
/// - Form references (`CNAM`, `INAM`, `NAM2`, `NAM3`, `WNAM`, `ZNAM`, `XEZN`)
///   are stored through the setters as form IDs; `InitItem` resolves them
///   later.
/// - When the record leaves the cell map empty and `cFlags` bit 0 is clear,
///   the map is replaced by a new one (7001 buckets when `00484e40` says so,
///   else 701).
///
/// Returns false for a record of another type, true otherwise.
pub fn fn_00583560(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr) -> bool {
    let record_type = e.call(FILE_RECORD_TYPE, &args![file]).u8() as u32;
    if record_type != RECORD_TYPE_WORLD_SPACE {
        return false;
    }
    if e.call(FILE_IS_MASTER, &args![file]).bool() {
        let data = e
            .call(CREATE_OFFSET_DATA, &args![this, file])
            .ptr::<OffsetData>();
        let offset = e.call(FILE_CURRENT_OFFSET, &args![file]).u32();
        e.set(data, OffsetData::iFileOffset, offset);
    }
    e.call(FORM_LOAD, &args![this, file]);
    e.call(FORM_SET_STATE, &args![this, 0u32]);
    loop {
        let chunk = e.call(FILE_NEXT_CHUNK, &args![file]).u32();
        if chunk == 0 {
            break;
        }
        load_chunk(e, this, file, chunk);
        if !e.call(FILE_ADVANCE_CHUNK, &args![file]).bool() {
            break;
        }
    }
    // An empty cell map (and a world space that keeps cells) is replaced.
    let map = e.get(this, TESWorldSpace::pCellMap);
    let count = e.call(WORD_AT_0C, &args![map]).u32();
    if count == 0 && !fn_005861b0(e, this) {
        let map = e.get(this, TESWorldSpace::pCellMap);
        if !map.is_null() {
            e.vcall(map.addr(), 0, &args![1u32]);
        }
        let buckets = if e.call(FORM_QUERY_LARGE_MAP, &args![this]).bool() {
            LARGE_CELL_MAP_BUCKETS
        } else {
            SMALL_CELL_MAP_BUCKETS
        };
        let block = e.call(MEMORY_ALLOC, &args![CELL_MAP_SIZE]).u32();
        let new_map = if block == 0 {
            0
        } else {
            e.call(CELL_MAP_CONSTRUCT, &args![block, buckets]).u32()
        };
        e.set(this, TESWorldSpace::pCellMap, Ptr::new(new_map));
    }
    true
}

/// One chunk of the world space record (`Load`'s switch).
fn load_chunk(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr, chunk: u32) {
    match chunk {
        CHUNK_CLIMATE => {
            let value = read_chunk_word(e, file);
            e.call(SET_CLIMATE, &args![this, value]);
        }
        CHUNK_DATA => {
            if e.call(FILE_CHUNK_SIZE, &args![file]).u32() == 4 {
                let value = read_chunk_word(e, file);
                e.set(this, TESWorldSpace::cFlags, value as u8);
            } else {
                let at = this.addr() + 0x4C;
                e.call(FILE_READ_CHUNK, &args![file, at, 1u32]);
            }
        }
        CHUNK_LOD_WATER => {
            let value = read_chunk_word(e, file);
            e.call(SET_LOD_WATER, &args![this, value]);
        }
        CHUNK_MIN_COORDS | CHUNK_MAX_COORDS => load_corner_chunk(e, this, file, chunk),
        CHUNK_WATER => {
            let value = read_chunk_word(e, file);
            e.call(SET_WATER, &args![this, value]);
        }
        CHUNK_LOD_WATER_HEIGHT => {
            let bits = read_chunk_word(e, file);
            fn_00584150(e, this, f32::from_bits(bits));
        }
        CHUNK_IMPACT_FIRST | CHUNK_IMPACT_SECOND => {
            if e.get(this, TESWorldSpace::pImpactSwap).is_null() {
                let block = e.call(MEMORY_ALLOC, &args![IMPACT_SWAP_SIZE]).u32();
                let made = if block == 0 {
                    0
                } else {
                    e.call(IMPACT_SWAP_CONSTRUCT, &args![block]).u32()
                };
                e.set(this, TESWorldSpace::pImpactSwap, Ptr::new(made));
            }
            let swap = e.get(this, TESWorldSpace::pImpactSwap);
            e.call(IMPACT_SWAP_LOAD, &args![swap, file]);
        }
        CHUNK_EDITOR_ID => {
            let size = e.call(FILE_CHUNK_SIZE, &args![file]).u32();
            // The game allocates the chunk size on its stack (rounded up to
            // 16 bytes) and reads with a limit of 0x200.
            e.with_stack((size + 15) & !15, |e, buffer| {
                e.call(FILE_READ_CHUNK, &args![file, buffer, 0x200u32]);
                e.vcall(this.addr(), SLOT_SET_EDITOR_ID, &args![buffer]);
            });
        }
        CHUNK_OBJECT_BOUNDS => {
            e.vcall(this.addr(), 0xE0, &args![file]);
        }
        CHUNK_ICON => {
            let texture = this.addr() + 0x24;
            e.call(TEXTURE_LOAD_CHUNK, &args![texture, file]);
        }
        CHUNK_FULL_NAME => {
            let name = this.addr() + 0x18;
            e.call(FULL_NAME_LOAD, &args![name, file]);
        }
        CHUNK_ENCOUNTER_ZONE => {
            let value = read_chunk_word(e, file);
            e.set(this, TESWorldSpace::pEncounterZone, Ptr::new(value));
        }
        CHUNK_OFFSETS => load_offsets_chunk(e, this, file),
        CHUNK_DEFAULT_HEIGHTS => {
            e.with_stack(8, |e, block| {
                e.call(FILE_READ_CHUNK, &args![file, block, 8u32]);
                if e.call(FILE_NEEDS_SWAP, &args![file]).bool() {
                    e.call(SWAP_PAIR, &args![block]);
                }
                let land = e.mem.f32(block.addr());
                e.set(this, TESWorldSpace::fDefaultLandHeight, land);
                let water = e.mem.f32(block.addr() + 4);
                e.set(this, TESWorldSpace::fDefaultWaterHeight, water);
            });
        }
        CHUNK_IMAGE_SPACE => {
            let value = read_chunk_word(e, file);
            e.call(SET_IMAGE_SPACE, &args![this, value]);
        }
        CHUNK_MAP_DATA => {
            let at = this.addr() + 0x80;
            e.call(FILE_READ_CHUNK, &args![file, at, 0x10u32]);
            if e.call(FILE_NEEDS_SWAP, &args![file]).bool() {
                fn_005840b0(e, Ptr::new(at));
            }
        }
        CHUNK_CANOPY_TEXTURE => load_texture_name(e, this.addr() + 0xD4, file),
        CHUNK_MAP_OFFSET_DATA => {
            let at = this.addr() + 0x90;
            e.call(FILE_READ_CHUNK, &args![file, at, 0xCu32]);
            if e.call(FILE_NEEDS_SWAP, &args![file]).bool() {
                e.call(SWAP_MAP_OFFSET_BLOCK, &args![at]);
            }
        }
        CHUNK_PARENT_USE_FLAGS => {
            let at = this.addr() + 0x4E;
            e.call(FILE_READ_CHUNK_U16, &args![file, at]);
        }
        CHUNK_SKIPPED => {
            read_chunk_word(e, file);
        }
        CHUNK_PARENT_WORLD => {
            let value = read_chunk_word(e, file);
            e.call(SET_PARENT_WORLD, &args![this, value]);
            e.set(this, TESWorldSpace::sParentUseFlags, 0xffff);
        }
        CHUNK_WATER_NOISE_TEXTURE => load_texture_name(e, this.addr() + 0xE0, file),
        CHUNK_MUSIC_TYPE => {
            let value = read_chunk_word(e, file);
            e.call(SET_MUSIC_TYPE, &args![this, value]);
        }
        _ => {}
    }
}

/// `GetChunkData(&word)` into a zeroed local.
fn read_chunk_word(e: &mut Engine, file: Ptr) -> u32 {
    e.with_stack(4, |e, local| {
        e.call(FILE_READ_CHUNK_U32, &args![file, local]);
        e.mem.u32(local.addr())
    })
}

/// `NAM0` / `NAM9` inside `Load` (the same work as `LoadPartial`).
fn load_corner_chunk(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr, chunk: u32) {
    let is_min = chunk == CHUNK_MIN_COORDS;
    e.with_stack(8, |e, corner| {
        e.call(LOCAL_STRUCT_CONSTRUCT, &args![corner]);
        e.call(FILE_READ_CHUNK, &args![file, corner, 8u32]);
        if e.call(FILE_NEEDS_SWAP, &args![file]).bool() {
            swap_dword(e, corner.addr());
            swap_dword(e, corner.addr() + 4);
        }
        let (x_field, y_field, combine) = if is_min {
            (
                TESWorldSpace::MinimumCoords_x,
                TESWorldSpace::MinimumCoords_y,
                FLOAT_MIN,
            )
        } else {
            (
                TESWorldSpace::MaximumCoords_x,
                TESWorldSpace::MaximumCoords_y,
                FLOAT_MAX,
            )
        };
        let own = e.get(this, x_field);
        let read = e.mem.f32(corner.addr());
        let widened = e.call(combine, &args![own, read]).f32();
        e.set(this, x_field, widened);
        let own = e.get(this, y_field);
        let read = e.mem.f32(corner.addr() + 4);
        let widened = e.call(combine, &args![own, read]).f32();
        e.set(this, y_field, widened);
        if e.call(FILE_IS_MASTER, &args![file]).bool() {
            let data = e.call(CREATE_OFFSET_DATA, &args![this, file]).u32();
            let (x_at, y_at) = if is_min { (4, 8) } else { (0xC, 0x10) };
            let x_bits = e.mem.u32(corner.addr());
            let y_bits = e.mem.u32(corner.addr() + 4);
            e.mem.set_u32(data + x_at, x_bits);
            e.mem.set_u32(data + y_at, y_bits);
        }
    });
}

/// `OFST` inside `Load`: the table of cell record offsets of this file.
fn load_offsets_chunk(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr) {
    let size = e.call(FILE_CHUNK_SIZE, &args![file]).u32();
    if size == 0 {
        return;
    }
    let count = size >> 2;
    let data = e
        .call(CREATE_OFFSET_DATA, &args![this, file])
        .ptr::<OffsetData>();
    let old = e.get(data, OffsetData::pCellFileOffsets);
    if !old.is_null() {
        e.call(MEMORY_FREE, &args![old]);
    }
    // `new u32[count]`: the byte count saturates when it overflows 32 bits.
    let bytes = (count as u64 * 4).min(u32::MAX as u64) as u32;
    let table = e.call(MEMORY_ALLOC, &args![bytes]).ptr();
    e.set(data, OffsetData::pCellFileOffsets, table);
    e.call(FILE_READ_CHUNK, &args![file, table, 0u32]);
    if e.call(FILE_NEEDS_SWAP, &args![file]).bool() {
        for i in 0..count {
            swap_dword(e, table.addr() + i * 4);
        }
    }
}

/// `NNAM` / `XNAM` inside `Load`: a texture path read into a stack buffer
/// and set on the `TESTexture` at `texture`.
fn load_texture_name(e: &mut Engine, texture: u32, file: Ptr) {
    let size = e.call(FILE_CHUNK_SIZE, &args![file]).u32();
    if size != 0 {
        e.with_stack((size + 15) & !15, |e, buffer| {
            e.call(FILE_READ_CHUNK, &args![file, buffer, size]);
            e.call(TEXTURE_SET_NAME, &args![texture, buffer]);
        });
    }
}

// Translated from 005840b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Byte-swaps a `WORLD_MAP_DATA` (Xbox PDB) in place: its two 4-byte sizes
/// and four 2-byte cell coordinates.
pub fn fn_005840b0(e: &mut Engine, this: Ptr) {
    swap_dword(e, this.addr());
    swap_dword(e, this.addr() + 4);
    swap_word(e, this.addr() + 8);
    swap_word(e, this.addr() + 0xA);
    swap_word(e, this.addr() + 0xC);
    swap_word(e, this.addr() + 0xE);
}

// Translated from 00584150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `fLODWaterHeight` (Xbox PDB).
pub fn fn_00584150(e: &mut Engine, this: Ptr<TESWorldSpace>, height: f32) {
    e.set(this, TESWorldSpace::fLODWaterHeight, height);
}

// Translated from 00584170 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::Save` (Xbox PDB, vtable slot `+0x2c`): writes the world
/// space record's chunks. The parent-world values (climate, image space,
/// lod water and its height, the default heights, the world map, the music
/// type) are only written when the world space does not take them from its
/// parent (`00586340`).
pub fn fn_00584170(e: &mut Engine, this: Ptr<TESWorldSpace>) {
    e.call(FORM_START, &args![this]);
    e.call(FULL_NAME_SAVE, &args![this.addr() + 0x18]);
    if !e.get(this, TESWorldSpace::pEncounterZone).is_null() {
        let zone = e.get(this, TESWorldSpace::pEncounterZone);
        let id = e.call(WORD_AT_0C, &args![zone]).u32();
        e.call(ADD_CHUNK_U32, &args![CHUNK_ENCOUNTER_ZONE, id]);
    }
    if !e.get(this, TESWorldSpace::pParentWorld).is_null() {
        let parent = e.get(this, TESWorldSpace::pParentWorld);
        let id = e.call(WORD_AT_0C, &args![parent]).u32();
        e.call(ADD_CHUNK_U32, &args![CHUNK_PARENT_WORLD, id]);
        let flags = e.get(this, TESWorldSpace::sParentUseFlags) as u32;
        e.call(ADD_CHUNK_U16, &args![CHUNK_PARENT_USE_FLAGS, flags]);
    }
    if !fn_00586340(e, this, 4) {
        let climate = fn_00585fe0(e, this);
        if !climate.is_null() {
            let climate = fn_00585fe0(e, this);
            let id = e.call(WORD_AT_0C, &args![climate]).u32();
            e.call(ADD_CHUNK_U32, &args![CHUNK_CLIMATE, id]);
        }
    }
    if !fn_00586340(e, this, 3) {
        let water = fn_00586070(e, this);
        if !water.is_null() {
            let water = fn_00586070(e, this);
            let id = e.call(WORD_AT_0C, &args![water]).u32();
            e.call(ADD_CHUNK_U32, &args![CHUNK_WATER, id]);
        }
    }
    if !fn_00586340(e, this, 1) {
        let lod_water = fn_005860c0(e, this);
        if !lod_water.is_null() {
            let lod_water = fn_005860c0(e, this);
            let id = e.call(WORD_AT_0C, &args![lod_water]).u32();
            e.call(ADD_CHUNK_U32, &args![CHUNK_LOD_WATER, id]);
            let height = e.call(GET_LOD_WATER_HEIGHT, &args![this]).f32();
            e.call(ADD_CHUNK_U32, &args![CHUNK_LOD_WATER_HEIGHT, height]);
        }
    }
    if !fn_00586340(e, this, 0) {
        e.with_stack(8, |e, block| {
            let land = e.get(this, TESWorldSpace::fDefaultLandHeight);
            e.mem.set_f32(block.addr(), land);
            let water = e.get(this, TESWorldSpace::fDefaultWaterHeight);
            e.mem.set_f32(block.addr() + 4, water);
            if e.call(SAVING_SWAPPED, &args![]).bool() {
                e.call(SWAP_PAIR, &args![block]);
            }
            e.call(ADD_CHUNK_DATA, &args![CHUNK_DEFAULT_HEIGHTS, block, 8u32]);
            if e.call(SAVING_SWAPPED, &args![]).bool() {
                e.call(SWAP_PAIR, &args![block]);
            }
        });
    }
    if !fn_00586340(e, this, 2) {
        e.call(TEXTURE_SAVE, &args![this.addr() + 0x24, CHUNK_ICON]);
        if e.call(SAVING_SWAPPED, &args![]).bool() {
            fn_005840b0(e, Ptr::new(this.addr() + 0x80));
        }
        e.call(
            ADD_CHUNK_DATA,
            &args![CHUNK_MAP_DATA, this.addr() + 0x80, 0x10u32],
        );
        if e.call(SAVING_SWAPPED, &args![]).bool() {
            fn_005840b0(e, Ptr::new(this.addr() + 0x80));
        }
    }
    // The map offset block is written whatever the parent flags say.
    if e.call(SAVING_SWAPPED, &args![]).bool() {
        e.call(SWAP_MAP_OFFSET_BLOCK, &args![this.addr() + 0x90]);
    }
    e.call(
        ADD_CHUNK_DATA,
        &args![CHUNK_MAP_OFFSET_DATA, this.addr() + 0x90, 0xCu32],
    );
    if e.call(SAVING_SWAPPED, &args![]).bool() {
        e.call(SWAP_MAP_OFFSET_BLOCK, &args![this.addr() + 0x90]);
    }
    if !fn_00586340(e, this, 5) {
        let image_space = e.call(GET_IMAGE_SPACE_RAW, &args![this]).u32();
        if image_space != 0 {
            let image_space = e.call(GET_IMAGE_SPACE_RAW, &args![this]).u32();
            let id = e.call(WORD_AT_0C, &args![image_space]).u32();
            e.call(ADD_CHUNK_U32, &args![CHUNK_IMAGE_SPACE, id]);
        }
    }
    let flags = e.get(this, TESWorldSpace::cFlags) as u32;
    e.call(ADD_CHUNK_U8, &args![CHUNK_DATA, flags]);
    e.call(
        ADD_CHUNK_ARRAY,
        &args![CHUNK_MIN_COORDS, this.addr() + 0xA0, 2u32],
    );
    e.call(
        ADD_CHUNK_ARRAY,
        &args![CHUNK_MAX_COORDS, this.addr() + 0xA8, 2u32],
    );
    if !fn_00586150(e, this).is_null() {
        let music = fn_00586150(e, this);
        let id = e.call(WORD_AT_0C, &args![music]).u32();
        e.call(ADD_CHUNK_U32, &args![CHUNK_MUSIC_TYPE, id]);
    }
    save_texture_name(e, this.addr() + 0xD4, CHUNK_CANOPY_TEXTURE);
    save_texture_name(e, this.addr() + 0xE0, CHUNK_WATER_NOISE_TEXTURE);
    let swap = e.get(this, TESWorldSpace::pImpactSwap);
    if !swap.is_null() {
        e.call(IMPACT_SWAP_SAVE, &args![swap]);
    }
    e.call(FORM_CLOSE, &args![this]);
}

/// A texture path chunk: the text and its length plus the terminating zero.
/// (The decompiler hangs the length on the wrong call: `004856f0` takes the
/// tag, the text and the size.)
fn save_texture_name(e: &mut Engine, texture: u32, chunk: u32) {
    let length = e.call(TEXTURE_NAME_LENGTH, &args![texture]).u32();
    let text = e.call(TEXTURE_NAME_TEXT, &args![texture]).u32();
    e.call(ADD_CHUNK_BLOCK, &args![chunk, text, length + 1]);
}

// Translated from 00584530 (decompiled, FalloutNV.exe 1.4.0.525)
/// The index into a file's cell offset table of the exterior cell
/// (`x`, `y`), or -1 when the file has no table for this world space or the
/// cell is outside its grid.
pub fn fn_00584530(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr, x: i32, y: i32) -> i32 {
    let data = offset_data_of_file(e, this, file);
    if data.is_null() {
        return -1;
    }
    let (min_x, min_y, max_x, max_y) = offset_data_bounds(e, data);
    if x > max_x || x < min_x || y > max_y || y < min_y {
        return -1;
    }
    max_x
        .wrapping_sub(min_x)
        .wrapping_add(1)
        .wrapping_mul(y.wrapping_sub(min_y))
        .wrapping_add(x)
        .wrapping_sub(min_x)
}

// Translated from 005845e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::InitItem` (Xbox PDB, vtable slot `+0x88`): once per form
/// (form flag 8 clear), turns the form IDs the record loaded into the
/// climate, image space, encounter zone, water, lod water, music type and
/// parent world space they name (each through its setter, logging when the
/// form is missing), runs the impact swap's `InitItem` and sets the form
/// state. Then every cell of the cell map and the persistent cell get their
/// own `InitItem`, and the terrain manager is created.
pub fn tes_world_space_init_item(e: &mut Engine, this: Ptr<TESWorldSpace>) {
    if !e.call(FORM_FLAG_8, &args![this]).bool() {
        let climate = e.get(this, TESWorldSpace::pClimate).addr();
        if climate != 0 {
            let form = resolve_form_reference(
                e,
                this,
                climate,
                TYPE_TES_CLIMATE,
                MSG_CLIMATE_NAMED,
                MSG_CLIMATE_ID,
            );
            e.call(SET_CLIMATE, &args![this, form]);
        }
        let image_space = e.get(this, TESWorldSpace::pImageSpace).addr();
        if image_space != 0 {
            let form = resolve_form_reference(
                e,
                this,
                image_space,
                TYPE_TES_IMAGE_SPACE,
                MSG_IMAGE_SPACE_NAMED,
                MSG_IMAGE_SPACE_ID,
            );
            e.call(SET_IMAGE_SPACE, &args![this, form]);
        }
        let zone = e.get(this, TESWorldSpace::pEncounterZone).addr();
        if zone != 0 {
            let form = resolve_form_reference(
                e,
                this,
                zone,
                TYPE_BGS_ENCOUNTER_ZONE,
                MSG_ENCOUNTER_ZONE_NAMED,
                MSG_ENCOUNTER_ZONE_ID,
            );
            fn_00584bc0(e, this, Ptr::new(form));
        }
        let water = e.get(this, TESWorldSpace::pWorldWater).addr();
        if water != 0 {
            let form = resolve_form_reference(
                e,
                this,
                water,
                TYPE_TES_WATER_FORM,
                MSG_WATER_NAMED,
                MSG_WATER_ID,
            );
            e.call(SET_WATER, &args![this, form]);
        }
        let lod_water = e.get(this, TESWorldSpace::pLODWater).addr();
        if lod_water != 0 {
            let form = resolve_form_reference(
                e,
                this,
                lod_water,
                TYPE_TES_WATER_FORM,
                MSG_LOD_WATER_NAMED,
                MSG_LOD_WATER_ID,
            );
            e.call(SET_LOD_WATER, &args![this, form]);
        }
        let music = e.get(this, TESWorldSpace::pMusicType).addr();
        if music != 0 {
            let form = resolve_form_reference(
                e,
                this,
                music,
                TYPE_BGS_MUSIC_TYPE,
                MSG_MUSIC_NAMED,
                MSG_MUSIC_ID,
            );
            e.call(SET_MUSIC_TYPE, &args![this, form]);
        }
        let parent = e.get(this, TESWorldSpace::pParentWorld).addr();
        if parent != 0 {
            let form = resolve_form_reference(
                e,
                this,
                parent,
                TYPE_TES_WORLD_SPACE,
                MSG_PARENT_NAMED,
                MSG_PARENT_ID,
            );
            e.call(SET_PARENT_WORLD, &args![this, form]);
        }
        let swap = e.get(this, TESWorldSpace::pImpactSwap);
        if !swap.is_null() {
            e.call(IMPACT_SWAP_INIT_ITEM, &args![swap, this]);
        }
        e.call(FORM_SET_STATE, &args![this, 1u32]);
    }
    // Every cell of the map is initialized too (virtual slot +0x88).
    let map = e.get(this, TESWorldSpace::pCellMap);
    let mut position = e.call(MAP_FIRST_POS, &args![map]).u32();
    while position != 0 {
        let (next, _key, cell) = map_get_next(e, map.addr(), position);
        position = next;
        if cell != 0 {
            e.vcall(cell, 0x88, &args![]);
        }
    }
    let persistent = e.call(GET_PERSISTENT_CELL, &args![this]).u32();
    if persistent != 0 {
        e.vcall(persistent, 0x88, &args![]);
    }
    let block = e.call(MEMORY_ALLOC, &args![TERRAIN_MANAGER_SIZE]).u32();
    let manager = if block == 0 {
        0
    } else {
        e.call(TERRAIN_MANAGER_CONSTRUCT, &args![block, this]).u32()
    };
    e.set(this, TESWorldSpace::pTerrainManager, Ptr::new(manager));
}

/// `NiTMapBase::GetNext(&position, &key, &value)` (`006b7f20`): the next
/// position, the key and the value, all three kept in game memory by the
/// caller.
fn map_get_next(e: &mut Engine, map: u32, position: u32) -> (u32, u32, u32) {
    e.with_stack(12, |e, locals| {
        let at = locals.addr();
        e.mem.set_u32(at, position);
        e.mem.set_u32(at + 4, 0);
        e.mem.set_u32(at + 8, 0);
        e.call(MAP_GET_NEXT, &args![map, at, at + 4, at + 8]);
        (e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8))
    })
}

// Translated from 00584bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `pEncounterZone` (Xbox PDB).
pub fn fn_00584bc0(e: &mut Engine, this: Ptr<TESWorldSpace>, zone: Ptr) {
    e.set(this, TESWorldSpace::pEncounterZone, zone);
}

// Translated from 00584be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::SavesBefore` (Xbox PDB, vtable slot `+0x3c`) for a form:
/// whether this world space is saved before `form`. A cell is ordered by
/// its world space, a placed reference (the reference types
/// `0x3a` to `0x40`, `0x42`, `0x43`, `0x69`) by what its `TESChildCell`
/// part's first virtual function returns, another world space (`0x41`) by
/// form ID; everything else by the base class's order (`00484020`).
pub fn fn_00584be0(e: &mut Engine, this: Ptr<TESWorldSpace>, form: Ptr) -> bool {
    let form_type = e.call(FORM_TYPE, &args![form]).u32();
    match form_type {
        RECORD_TYPE_CELL => {
            let cell = dynamic_cast(e, form.addr(), TYPE_TES_OBJECT_CELL);
            let world = if cell == 0 {
                0
            } else {
                e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32()
            };
            if world == this.addr() {
                true
            } else if world != 0 {
                e.vcall(this.addr(), 0x3C, &args![world]).bool()
            } else {
                false
            }
        }
        0x3a..=0x40 | 0x42 | 0x43 | 0x69 => {
            let child = dynamic_cast(e, form.addr(), TYPE_TES_CHILD_CELL);
            let owner = e.vcall(child, 0, &args![]).u32();
            e.vcall(this.addr(), 0x3C, &args![owner]).bool()
        }
        0x41 => {
            let own_id = e.call(WORD_AT_0C, &args![this]).u32();
            let other_id = e.call(WORD_AT_0C, &args![form]).u32();
            own_id < other_id
        }
        _ => e.call(FORM_SAVES_BEFORE_FORM, &args![this, form]).bool(),
    }
}

// Translated from 00584d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::SavesBefore` (Xbox PDB, vtable slot `+0x38`) for a
/// `GRUP` header: group type 0 is the base class's decision (`00484150`),
/// types 1 and 6 (the groups labelled with a world space or a cell) order by
/// the labelled form (through slot `+0x3c`), 7 sorts before, 2 and 3 after,
/// every other type and any record that is not a group header: no.
pub fn fn_00584d40(e: &mut Engine, this: Ptr<TESWorldSpace>, header: Ptr) -> bool {
    if header.is_null() || e.mem.u32(header.addr()) != e.global::<u32>(GROUP_TAG) {
        return false;
    }
    let group_type = e.mem.u32(header.addr() + 0xC);
    let label = e.mem.u32(header.addr() + 8);
    match group_type {
        0 => e
            .call(FORM_SAVES_BEFORE_HEADER, &args![this, header])
            .bool(),
        1 | 6 => {
            let target = if group_type == 1 {
                TYPE_TES_WORLD_SPACE
            } else {
                TYPE_TES_OBJECT_CELL
            };
            let form = e.call(FORM_BY_ID, &args![label]).u32();
            let cast = dynamic_cast(e, form, target);
            if cast == 0 {
                false
            } else {
                e.vcall(this.addr(), 0x3C, &args![cast]).bool()
            }
        }
        7 => true,
        _ => false,
    }
}

// Translated from 00584e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::CreateDuplicateForm` (Xbox PDB, vtable slot `+0x40`):
/// duplicates the world space through the base class, clears the copy's cell
/// map and fills it with duplicates of this world space's cells (each marked
/// altered, slot `+0xc8`), takes a duplicate of the persistent cell, and
/// finishes the copy (`00588120`). Returns the copy.
pub fn fn_00584e60(e: &mut Engine, this: Ptr<TESWorldSpace>, flag: u8, other: Ptr) -> Ptr {
    let duplicate = e
        .call(FORM_CREATE_DUPLICATE, &args![this, flag, other])
        .u32();
    let copy = Ptr::<TESWorldSpace>::new(dynamic_cast(e, duplicate, TYPE_TES_WORLD_SPACE));
    let copy_map = e.mem.u32(copy.addr() + 0x30);
    e.call(MAP_REMOVE_ALL, &args![copy_map]);
    let map = e.get(this, TESWorldSpace::pCellMap);
    let mut position = e.call(MAP_FIRST_POS, &args![map]).u32();
    while position != 0 {
        let (next, _key, cell) = map_get_next(e, map.addr(), position);
        position = next;
        if cell != 0 {
            let cell_copy = e.vcall(cell, 0x40, &args![0u32, other]).u32();
            let cell_copy = dynamic_cast(e, cell_copy, TYPE_TES_OBJECT_CELL);
            if cell_copy != 0 {
                e.vcall(cell_copy, 0xC8, &args![1u32]);
                e.call(ADD_CELL, &args![copy, cell_copy]);
            }
        }
    }
    let persistent = e.call(GET_PERSISTENT_CELL, &args![this]).u32();
    if persistent != 0 {
        let cell_copy = e.vcall(persistent, 0x40, &args![0u32, other]).u32();
        let cell_copy = dynamic_cast(e, cell_copy, TYPE_TES_OBJECT_CELL);
        if cell_copy != 0 {
            e.set(copy, TESWorldSpace::pPersistentCell, Ptr::new(cell_copy));
            e.vcall(cell_copy, 0xC8, &args![1u32]);
        }
    }
    e.call(FINISH_DUPLICATE, &args![copy]);
    copy.cast()
}

// Translated from 00584fa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::Copy` (Xbox PDB, vtable slot `+0x108`): copies the
/// world space `source` (a form cast to a world space; nothing happens when
/// it is not one) over this one: the components, flags, parent, encounter
/// zone, climate, image space, impact swap (created, copied or deleted to
/// match), water, lod water and its height, default heights, map data, map
/// offset data, music type, and the cell map (cleared, then every entry of
/// the source's is set), then the two textures' own copies.
pub fn fn_00584fa0(e: &mut Engine, this: Ptr<TESWorldSpace>, source: Ptr) {
    let source = Ptr::<TESWorldSpace>::new(dynamic_cast(e, source.addr(), TYPE_TES_WORLD_SPACE));
    if source.is_null() {
        return;
    }
    e.call(FORM_COPY_COMPONENTS, &args![this, source]);
    let flags = e.get(source, TESWorldSpace::cFlags);
    e.set(this, TESWorldSpace::cFlags, flags);
    let parent_flags = e.get(source, TESWorldSpace::sParentUseFlags);
    e.set(this, TESWorldSpace::sParentUseFlags, parent_flags);
    let parent = e.get(source, TESWorldSpace::pParentWorld);
    e.set(this, TESWorldSpace::pParentWorld, parent);
    let zone = e.get(source, TESWorldSpace::pEncounterZone);
    e.set(this, TESWorldSpace::pEncounterZone, zone);
    let climate = fn_00585fe0(e, source);
    e.set(this, TESWorldSpace::pClimate, climate);
    let image_space = e.call(GET_IMAGE_SPACE_RAW, &args![source]).ptr();
    e.set(this, TESWorldSpace::pImageSpace, image_space);
    let source_swap = e.get(source, TESWorldSpace::pImpactSwap);
    if !source_swap.is_null() {
        if e.get(this, TESWorldSpace::pImpactSwap).is_null() {
            let block = e.call(MEMORY_ALLOC, &args![IMPACT_SWAP_SIZE]).u32();
            let made = if block == 0 {
                0
            } else {
                e.call(IMPACT_SWAP_CONSTRUCT, &args![block]).u32()
            };
            e.set(this, TESWorldSpace::pImpactSwap, Ptr::new(made));
        }
        let own = e.get(this, TESWorldSpace::pImpactSwap);
        e.call(IMPACT_SWAP_COPY, &args![own, source_swap]);
    } else if !e.get(this, TESWorldSpace::pImpactSwap).is_null() {
        let own = e.get(this, TESWorldSpace::pImpactSwap);
        e.call(IMPACT_SWAP_DESTRUCT, &args![own, 1u32]);
        e.set(this, TESWorldSpace::pImpactSwap, Ptr::NULL);
    }
    let water = fn_00586070(e, source);
    e.call(SET_WATER, &args![this, water]);
    let lod_water = fn_005860c0(e, source);
    e.call(SET_LOD_WATER, &args![this, lod_water]);
    let height = e.call(GET_LOD_WATER_HEIGHT, &args![source]).f32();
    fn_00584150(e, this, height);
    let land = e.get(source, TESWorldSpace::fDefaultLandHeight);
    e.set(this, TESWorldSpace::fDefaultLandHeight, land);
    let water_height = e.get(source, TESWorldSpace::fDefaultWaterHeight);
    e.set(this, TESWorldSpace::fDefaultWaterHeight, water_height);
    e.call(
        MEMCPY,
        &args![this.addr() + 0x80, source.addr() + 0x80, 0x10u32],
    );
    e.call(
        MEMCPY,
        &args![this.addr() + 0x90, source.addr() + 0x90, 0xCu32],
    );
    let music = fn_00586150(e, source);
    e.call(SET_MUSIC_TYPE, &args![this, music]);
    let own_map = e.get(this, TESWorldSpace::pCellMap);
    e.call(MAP_REMOVE_ALL, &args![own_map]);
    let source_map = e.get(source, TESWorldSpace::pCellMap);
    let mut position = e.call(MAP_FIRST_POS, &args![source_map]).u32();
    while position != 0 {
        let (next, key, value) = map_get_next(e, source_map.addr(), position);
        position = next;
        let own_map = e.get(this, TESWorldSpace::pCellMap);
        e.call(MAP_SET_AT, &args![own_map, key, value]);
    }
    // The two textures copy themselves (`CopyComponent`, slot +8).
    e.vcall(this.addr() + 0xD4, 8, &args![source.addr() + 0xD4]);
    e.vcall(this.addr() + 0xE0, 8, &args![source.addr() + 0xE0]);
}

// Translated from 00585250 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::Compare` (Xbox PDB, vtable slot `+0x10c`): true when
/// `other` differs from this world space (or is not a world space): the
/// components, flags, parent flags, parent, encounter zone, climate, image
/// space, impact swap, water, lod water and its height, the default
/// heights, the map data and map offset data bytes, the music type, and the
/// two textures' own compares.
pub fn fn_00585250(e: &mut Engine, this: Ptr<TESWorldSpace>, other: Ptr) -> bool {
    let other = Ptr::<TESWorldSpace>::new(dynamic_cast(e, other.addr(), TYPE_TES_WORLD_SPACE));
    if other.is_null() {
        return true;
    }
    if e.call(FORM_COMPARE_COMPONENTS, &args![this, other]).bool() {
        return true;
    }
    if e.get(this, TESWorldSpace::cFlags) != e.get(other, TESWorldSpace::cFlags) {
        return true;
    }
    if e.get(this, TESWorldSpace::sParentUseFlags) != e.get(other, TESWorldSpace::sParentUseFlags) {
        return true;
    }
    if e.get(this, TESWorldSpace::pParentWorld) != e.get(other, TESWorldSpace::pParentWorld) {
        return true;
    }
    if e.get(this, TESWorldSpace::pEncounterZone) != e.get(other, TESWorldSpace::pEncounterZone) {
        return true;
    }
    if fn_00585fe0(e, this) != fn_00585fe0(e, other) {
        return true;
    }
    let own_image = e.call(GET_IMAGE_SPACE_RAW, &args![this]).u32();
    let other_image = e.call(GET_IMAGE_SPACE_RAW, &args![other]).u32();
    if own_image != other_image {
        return true;
    }
    let own_swap = e.get(this, TESWorldSpace::pImpactSwap);
    let other_swap = e.get(other, TESWorldSpace::pImpactSwap);
    if !own_swap.is_null() && !other_swap.is_null() {
        if e.call(IMPACT_SWAP_COMPARE, &args![own_swap, other_swap])
            .bool()
        {
            return true;
        }
    } else if !own_swap.is_null() || !other_swap.is_null() {
        return true;
    }
    if fn_00586070(e, this) != fn_00586070(e, other) {
        return true;
    }
    if fn_005860c0(e, this) != fn_005860c0(e, other) {
        return true;
    }
    let own_height = e.call(GET_LOD_WATER_HEIGHT, &args![this]).f64();
    let other_height = e.call(GET_LOD_WATER_HEIGHT, &args![other]).f64();
    if own_height != other_height {
        return true;
    }
    if e.get(this, TESWorldSpace::fDefaultLandHeight)
        != e.get(other, TESWorldSpace::fDefaultLandHeight)
        || e.get(this, TESWorldSpace::fDefaultWaterHeight)
            != e.get(other, TESWorldSpace::fDefaultWaterHeight)
    {
        return true;
    }
    if e.call(
        MEMCMP,
        &args![this.addr() + 0x80, other.addr() + 0x80, 0x10u32],
    )
    .i32()
        != 0
    {
        return true;
    }
    if e.call(
        MEMCMP,
        &args![this.addr() + 0x90, other.addr() + 0x90, 0xCu32],
    )
    .i32()
        != 0
    {
        return true;
    }
    if fn_00586150(e, this) != fn_00586150(e, other) {
        return true;
    }
    if e.vcall(this.addr() + 0xD4, 0xC, &args![other.addr() + 0xD4])
        .bool()
    {
        return true;
    }
    e.vcall(this.addr() + 0xE0, 0xC, &args![other.addr() + 0xE0])
        .bool()
}

// Translated from 005854f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::FindCellInFile` (Xbox PDB): positions `file` on the
/// exterior cell record (`x`, `y`) of this world space. A master file with
/// an `OFFSET_DATA` table finds it by index (`fn_00584530`); any other file
/// is scanned record by record from this world space's record: the
/// `GRUP`s the scan passes are entered or skipped by group type and label
/// (cell block and sub-block keys), cell records are matched on their
/// `XCLC` grid position (an interior cell, flag 0x400, never matches). The
/// reader is left on the cell record when found.
pub fn tes_world_space_find_cell_in_file(
    e: &mut Engine,
    this: Ptr<TESWorldSpace>,
    file: Ptr,
    x: i32,
    y: i32,
) -> bool {
    if !file.is_null() && e.call(FILE_IS_MASTER, &args![file]).bool() {
        let data = offset_data_of_file(e, this, file);
        if !data.is_null() && !e.get(data, OffsetData::pCellFileOffsets).is_null() {
            let index = fn_00584530(e, this, file, x, y);
            if index == -1 {
                return false;
            }
            let table = e.get(data, OffsetData::pCellFileOffsets).addr();
            let offset = e
                .mem
                .u32(table.wrapping_add((index as u32).wrapping_mul(4)));
            if offset == 0 {
                return false;
            }
            let base = e.get(data, OffsetData::iFileOffset);
            e.call(FILE_SET_OFFSET, &args![file, offset.wrapping_add(base)]);
            return true;
        }
    }
    let mut result = false;
    let block_key = e.call(CALC_BLOCK_KEY, &args![x, y]).u32();
    let sub_block_key = e.call(CALC_SUB_BLOCK_KEY, &args![x, y]).u32();
    if !file.is_null() && e.call(FILE_FIND_FORM, &args![file, this]).bool() {
        e.call(FILE_NEXT_FORM, &args![file, 1u32]);
        let mut done = false;
        let record = e.call(FILE_CURRENT_RECORD, &args![file]).u32();
        while record != 0 && !done {
            let record_tag = e.mem.u32(record);
            if record_tag == e.global::<u32>(GROUP_TAG) {
                let mut skip = true;
                done = true;
                let group_type = e.mem.u32(record + 0xC);
                let label = e.mem.u32(record + 8);
                match group_type {
                    1 => {
                        skip = false;
                        done = false;
                    }
                    4 => {
                        if block_key == label || sub_block_key == label {
                            skip = false;
                        }
                        done = false;
                    }
                    5 => {
                        if sub_block_key == label {
                            skip = false;
                        }
                        done = false;
                    }
                    6 | 8 | 9 => done = false,
                    _ => {}
                }
                if !done {
                    if !skip {
                        e.call(FILE_NEXT_FORM, &args![file, 1u32]);
                    } else {
                        e.call(FILE_LEAVE_GROUP, &args![file]);
                    }
                }
            } else if record_tag == e.global::<u32>(CELL_TAG) {
                let matched = scan_cell_record(e, file, record, x, y);
                if matched {
                    result = true;
                    done = true;
                    e.call(FILE_REWIND_RECORD, &args![file]);
                } else {
                    e.call(FILE_NEXT_FORM, &args![file, 1u32]);
                }
            } else if record_tag == e.global::<u32>(SKIPPED_RECORD_TAG_01187338) {
                e.call(FILE_NEXT_FORM, &args![file, 1u32]);
            } else {
                done = true;
            }
        }
    }
    result
}

/// Whether the cell record under the reader is the exterior cell (`x`,
/// `y`): reads the `XCLC` chunk (three words, the grid x and y and flags)
/// into a zeroed local; an interior cell (record flag 0x400) has the grid x
/// 0x7fffffff.
fn scan_cell_record(e: &mut Engine, file: Ptr, record: u32, x: i32, y: i32) -> bool {
    e.with_stack(12, |e, grid| {
        e.call(CELL_COORDS_CONSTRUCT, &args![grid]);
        e.call(MEMSET, &args![grid, 0u32, 0xCu32]);
        if e.mem.u32(record + 8) & 0x400 != 0 {
            e.mem.set_i32(grid.addr(), 0x7fff_ffff);
        } else {
            let mut found = false;
            let mut chunk = e.call(FILE_NEXT_CHUNK, &args![file]).u32();
            while chunk != 0 && !found {
                if chunk == CHUNK_CELL_GRID {
                    e.call(FILE_READ_CHUNK, &args![file, grid, 0xCu32]);
                    if e.call(FILE_NEEDS_SWAP, &args![file]).bool() {
                        e.call(SWAP_PAIR, &args![grid]);
                    }
                    found = true;
                }
                chunk = if e.call(FILE_ADVANCE_CHUNK, &args![file]).bool() {
                    e.call(FILE_NEXT_CHUNK, &args![file]).u32()
                } else {
                    0
                };
            }
        }
        e.mem.i32(grid.addr()) == x && e.mem.i32(grid.addr() + 4) == y
    })
}

// Translated from 005857b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `delta` to every non-zero entry of the cell offset table of
/// `file` (when the file has one and `delta` is not 0): the entries'
/// base moved after the file's records were shifted.
pub fn fn_005857b0(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr, delta: u32) {
    let data = offset_data_of_file(e, this, file);
    if data.is_null() || e.get(data, OffsetData::pCellFileOffsets).is_null() || delta == 0 {
        return;
    }
    let (min_x, min_y, max_x, max_y) = offset_data_bounds(e, data);
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            let index = fn_00584530(e, this, file, x, y);
            if index >= 0 {
                let table = e.get(data, OffsetData::pCellFileOffsets).addr();
                let at = table.wrapping_add((index as u32).wrapping_mul(4));
                let entry = e.mem.u32(at);
                if entry != 0 {
                    e.mem.set_u32(at, entry.wrapping_add(delta));
                }
            }
        }
    }
}

// Translated from 005858b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::FindInFileFast` (Xbox PDB, vtable slot `+0x7c`): whether
/// `file` is a master file whose `OFFSET_DATA` has a record position for this
/// world space that holds a world space record with this form ID.
pub fn fn_005858b0(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr) -> bool {
    let data = offset_data_of_file(e, this, file);
    if !file.is_null() && e.call(FILE_IS_MASTER, &args![file]).bool() && !data.is_null() {
        let position = e.get(data, OffsetData::iFileOffset);
        if position != 0 && e.call(FILE_SET_OFFSET, &args![file, position]).bool() {
            let record_type = e.call(FILE_RECORD_TYPE, &args![file]).u32();
            if record_type == e.global::<u8>(WORLD_SPACE_RECORD_TYPE) as u32 {
                let record_id = e.call(FILE_CURRENT_FORM_ID, &args![file]).u32();
                let own_id = e.call(WORD_AT_0C, &args![this]).u32();
                return record_id == own_id;
            }
        }
    }
    false
}

// Translated from 00585940 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::GetExtCellDataFromFileByEditorID` (Xbox PDB): looks for
/// the exterior cell whose editor ID is `editor_id` in the cell offset
/// tables of this form's files, storing its grid position through `out_x`
/// and `out_y`.
pub fn tes_world_space_get_ext_cell_data_from_file_by_editor_id(
    e: &mut Engine,
    this: Ptr<TESWorldSpace>,
    editor_id: Ptr,
    out_x: Ptr,
    out_y: Ptr,
) -> bool {
    let count = e.call(FORM_FILE_COUNT, &args![this]).u32();
    for index in 0..count {
        let file = form_get_file(e, this.addr(), index);
        let file = thread_safe_file(e, file);
        let data = offset_data_of_file(e, this, file);
        if data.is_null() {
            continue;
        }
        let (min_x, min_y, max_x, max_y) = offset_data_bounds(e, data);
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let cell_index = fn_00584530(e, this, file, x, y);
                if cell_index == -1 {
                    continue;
                }
                let table = e.get(data, OffsetData::pCellFileOffsets).addr();
                let offset = e
                    .mem
                    .u32(table.wrapping_add((cell_index as u32).wrapping_mul(4)));
                if offset == 0 {
                    continue;
                }
                let base = e.get(data, OffsetData::iFileOffset);
                e.call(FILE_SET_OFFSET, &args![file, offset.wrapping_add(base)]);
                if e.call(FILE_RECORD_TYPE, &args![file]).u32() != RECORD_TYPE_CELL {
                    continue;
                }
                if e.call(FILE_NEXT_CHUNK, &args![file]).u32() != CHUNK_EDITOR_ID {
                    continue;
                }
                let size = e.call(FILE_CHUNK_SIZE, &args![file]).u32() + 1;
                let buffer = e.call(MEMORY_ALLOC, &args![size]).u32();
                e.call(MEMSET, &args![buffer, 0u32, size]);
                e.call(FILE_READ_CHUNK, &args![file, buffer, 0u32]);
                let differs = e.call(EDITOR_ID_COMPARE, &args![buffer, editor_id]).i32();
                if differs == 0 {
                    e.mem.set_i32(out_x.addr(), x);
                    e.mem.set_i32(out_y.addr(), y);
                    e.call(MEMORY_FREE, &args![buffer]);
                    return true;
                }
                e.call(MEMORY_FREE, &args![buffer]);
            }
        }
    }
    false
}

// Translated from 00585b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::LoadCell` (Xbox PDB): the exterior cell (`x`, `y`),
/// loaded or completed from every file of this world space that has it.
/// The existing cell (from the cell map) is used when it has no temporary
/// data yet; a missing one is created and registered first. Each file adds
/// its record (`TESDataHandler::LoadForm`) and its temporary data
/// (`LoadTempDataFromFile`); the finished cell gets `InitItem` and its
/// persistent references assigned. Logs when a file's temporary data does not
/// load. Returns the cell (null when no file has it).
pub fn tes_world_space_load_cell(e: &mut Engine, this: Ptr<TESWorldSpace>, x: i32, y: i32) -> Ptr {
    e.with_stack(8, |e, guard| {
        e.call(
            MEMORY_CONTEXT_ENTER,
            &args![guard, 0x1au32, 1u32, WORLD_SPACE_SOURCE, 0x796u32],
        );
        let mut loaded = true;
        let mut cell = e.call(GET_CELL_FROM_CELL_COORD, &args![this, x, y]).u32();
        e.set_global(CURRENT_WORLD_SPACE, this.addr());
        let already_loaded = cell != 0 && e.call(CELL_HAS_TEMP_DATA, &args![cell]).bool();
        if !already_loaded {
            let count = e.call(FORM_FILE_COUNT, &args![this]).u32();
            for index in 0..count {
                let file = form_get_file(e, this.addr(), index);
                let file = thread_safe_file(e, file);
                let found =
                    !file.is_null() && tes_world_space_find_cell_in_file(e, this, file, x, y);
                if !found {
                    continue;
                }
                if cell == 0 {
                    let block = e.call(MEMORY_ALLOC, &args![CELL_SIZE]).u32();
                    cell = if block == 0 {
                        0
                    } else {
                        e.call(CELL_CONSTRUCT, &args![block]).u32()
                    };
                    e.call(CELL_SET_INTERIOR, &args![cell, 0u32]);
                    e.call(CELL_CREATE_DATA, &args![cell]);
                    e.call(CELL_SET_DATA_COORD, &args![cell, x, y]);
                    e.call(ADD_CELL, &args![this, cell]);
                    let world_id = e.call(WORD_AT_0C, &args![this]).u32();
                    let save_load = e.global::<u32>(SAVE_LOAD_GAME);
                    let created_id = e
                        .call(SAVE_LOAD_CREATED_CELL_ID, &args![save_load, world_id, x, y])
                        .u32();
                    if created_id != 0 {
                        e.vcall(cell, 0x128, &args![created_id, 1u32]);
                    }
                    loaded = e.call(DATA_HANDLER_LOAD_FORM, &args![cell, file]).bool();
                } else {
                    let record = e.call(FILE_CURRENT_RECORD, &args![file]).u32();
                    if e.mem.u32(record + 8) & 0x4000 == 0 {
                        e.call(DATA_HANDLER_LOAD_FORM, &args![cell, file]);
                    }
                }
                if !e.call(CELL_LOAD_TEMP_DATA, &args![cell, file]).bool() {
                    loaded = false;
                }
            }
            if cell != 0 {
                e.call(CELL_SET_HAS_TEMP_DATA, &args![cell, 1u32]);
                let save_load = e.global::<u32>(SAVE_LOAD_GAME);
                let state = e.call(SAVE_LOAD_GET_STATE, &args![save_load]).u8();
                e.call(SAVE_LOAD_SET_STATE, &args![save_load, (state == 0) as u32]);
                e.vcall(cell, 0x88, &args![]);
                e.call(SAVE_LOAD_SET_STATE, &args![save_load, state as u32]);
                e.call(ASSIGN_PERSISTENT_REFS, &args![this, cell]);
            }
        }
        if !loaded {
            let world_id = e.call(WORD_AT_0C, &args![this]).u32();
            let name = e.vcall(this.addr(), SLOT_EDITOR_ID, &args![]).u32();
            e.call(LOG, &args![MSG_CELL_LOAD_FAILED, x, y, name, world_id]);
        }
        e.call(MEMORY_CONTEXT_LEAVE, &args![guard]);
        Ptr::new(cell)
    })
}

// Translated from 00585e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::UnLoadCell` (Xbox PDB): unloads `cell` if it belongs to
/// this world space. A cell from a master file, or from no file, is cleaned
/// up, noted to the save/load game, unloaded from the game loader unless its
/// form ID is 0xFF000000 or above (`00469860`), released from the cell map and
/// deleted; a cell from another file is only cleaned up.
pub fn tes_world_space_un_load_cell(e: &mut Engine, this: Ptr<TESWorldSpace>, cell: Ptr) {
    if cell.is_null() || e.call(CELL_GET_WORLD_SPACE, &args![cell]).ptr::<()>() != this.cast() {
        return;
    }
    let file = form_get_file(e, cell.addr(), 0xffff_ffff);
    if file.is_null() || e.call(FILE_IS_MASTER, &args![file]).bool() {
        e.call(CELL_CLEANUP, &args![cell]);
        let save_load = e.global::<u32>(SAVE_LOAD_GAME);
        e.call(SAVE_LOAD_NOTE_CELL, &args![save_load, cell]);
        let cell_id = e.call(WORD_AT_0C, &args![cell]).u32();
        let loader_state = e.global::<u32>(LOADER_STATE);
        if !e
            .call(LOADER_STATE_QUERY, &args![loader_state, cell_id])
            .bool()
        {
            let loader = e.global::<u32>(GAME_LOADER);
            e.call(GAME_LOADER_UNLOAD_FORM, &args![loader, cell, 0u32]);
        }
        e.call(RELEASE_CELL, &args![this, cell]);
        e.vcall(cell.addr(), 0x10, &args![1u32]);
    } else {
        e.call(CELL_CLEANUP, &args![cell]);
    }
}

// Translated from 00585ee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::FindLandDataInFile` (Xbox PDB): finds the file that has
/// land data for the exterior cell (`x`, `y`), searching this world space's
/// files from the last to the first, or the parent world space (the one
/// `fn_00586390(.., 0)` names) when it uses the parent's cells. On success
/// `*out_file` is the file (reader positioned on the land data) and
/// `*out_offset` its current offset; both are 0 otherwise.
pub fn tes_world_space_find_land_data_in_file(
    e: &mut Engine,
    this: Ptr<TESWorldSpace>,
    x: i32,
    y: i32,
    out_file: Ptr,
    out_offset: Ptr,
) -> bool {
    e.mem.set_u32(out_file.addr(), 0);
    e.mem.set_u32(out_offset.addr(), 0);
    let parent = e
        .call(PARENT_FOR, &args![this, 0u32])
        .ptr::<TESWorldSpace>();
    if !parent.is_null() {
        return tes_world_space_find_land_data_in_file(e, parent, x, y, out_file, out_offset);
    }
    let count = e.call(FORM_FILE_COUNT, &args![this]).u32();
    for index in (0..count).rev() {
        let file = form_get_file(e, this.addr(), index);
        let file = thread_safe_file(e, file);
        if !file.is_null()
            && tes_world_space_find_cell_in_file(e, this, file, x, y)
            && e.call(CELL_FIND_LAND_DATA_IN_FILE, &args![file]).bool()
        {
            e.mem.set_u32(out_file.addr(), file.addr());
            let offset = e.call(FILE_CURRENT_OFFSET, &args![file]).u32();
            e.mem.set_u32(out_offset.addr(), offset);
            return true;
        }
    }
    false
}

// Translated from 00585fe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The climate: the parent world space's when this one uses the parent's
/// value 4, else `pClimate`.
pub fn fn_00585fe0(e: &mut Engine, this: Ptr<TESWorldSpace>) -> Ptr {
    let parent = e
        .call(PARENT_FOR, &args![this, 4u32])
        .ptr::<TESWorldSpace>();
    if !parent.is_null() {
        let parent = e
            .call(PARENT_FOR, &args![this, 4u32])
            .ptr::<TESWorldSpace>();
        fn_00585fe0(e, parent)
    } else {
        e.get(this, TESWorldSpace::pClimate)
    }
}

// Translated from 00586020 (decompiled, FalloutNV.exe 1.4.0.525)
/// The image space: `pImageSpace`, or when that is null the parent world
/// space's (value 5) if this one uses it.
pub fn fn_00586020(e: &mut Engine, this: Ptr<TESWorldSpace>) -> Ptr {
    let mut image_space = e.get(this, TESWorldSpace::pImageSpace);
    if image_space.is_null() {
        let parent = e
            .call(PARENT_FOR, &args![this, 5u32])
            .ptr::<TESWorldSpace>();
        if !parent.is_null() {
            let parent = e
                .call(PARENT_FOR, &args![this, 5u32])
                .ptr::<TESWorldSpace>();
            image_space = fn_00586020(e, parent);
        }
    }
    image_space
}

// Translated from 00586070 (decompiled, FalloutNV.exe 1.4.0.525)
/// The water form: the parent world space's when this one uses the
/// parent's value 3, else `pWorldWater`, else the game's default water.
pub fn fn_00586070(e: &mut Engine, this: Ptr<TESWorldSpace>) -> Ptr {
    let parent = e
        .call(PARENT_FOR, &args![this, 3u32])
        .ptr::<TESWorldSpace>();
    if !parent.is_null() {
        let parent = e
            .call(PARENT_FOR, &args![this, 3u32])
            .ptr::<TESWorldSpace>();
        fn_00586070(e, parent)
    } else if !e.get(this, TESWorldSpace::pWorldWater).is_null() {
        e.get(this, TESWorldSpace::pWorldWater)
    } else {
        e.global::<Ptr>(DEFAULT_WATER)
    }
}

// Translated from 005860c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The lod water form: the parent world space's when this one uses the
/// parent's value 1, else `pLODWater`, else the game's default water.
pub fn fn_005860c0(e: &mut Engine, this: Ptr<TESWorldSpace>) -> Ptr {
    let parent = e
        .call(PARENT_FOR, &args![this, 1u32])
        .ptr::<TESWorldSpace>();
    if !parent.is_null() {
        let parent = e
            .call(PARENT_FOR, &args![this, 1u32])
            .ptr::<TESWorldSpace>();
        fn_005860c0(e, parent)
    } else if !e.get(this, TESWorldSpace::pLODWater).is_null() {
        e.get(this, TESWorldSpace::pLODWater)
    } else {
        e.global::<Ptr>(DEFAULT_WATER)
    }
}

// Translated from 00586110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `fDefaultLandHeight` (Xbox PDB).
pub fn fn_00586110(e: &mut Engine, this: Ptr<TESWorldSpace>, height: f32) {
    e.set(this, TESWorldSpace::fDefaultLandHeight, height);
}

// Translated from 00586130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `fDefaultWaterHeight` (Xbox PDB).
pub fn fn_00586130(e: &mut Engine, this: Ptr<TESWorldSpace>, height: f32) {
    e.set(this, TESWorldSpace::fDefaultWaterHeight, height);
}

// Translated from 00586150 (decompiled, FalloutNV.exe 1.4.0.525)
/// The music type (`pMusicType`, Xbox PDB).
pub fn fn_00586150(e: &mut Engine, this: Ptr<TESWorldSpace>) -> Ptr {
    e.get(this, TESWorldSpace::pMusicType)
}

// Translated from 00586170 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::GetTerrainManager` (Xbox PDB): the parent world
/// space's terrain manager when this one uses the parent's value 1, else
/// `pTerrainManager`.
pub fn tes_world_space_get_terrain_manager(e: &mut Engine, this: Ptr<TESWorldSpace>) -> Ptr {
    let parent = e
        .call(PARENT_FOR, &args![this, 1u32])
        .ptr::<TESWorldSpace>();
    if !parent.is_null() {
        let parent = e
            .call(PARENT_FOR, &args![this, 1u32])
            .ptr::<TESWorldSpace>();
        tes_world_space_get_terrain_manager(e, parent)
    } else {
        e.get(this, TESWorldSpace::pTerrainManager)
    }
}

// Translated from 005861b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cFlags` bit 0.
pub fn fn_005861b0(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.get(this, TESWorldSpace::cFlags) & 0x01 != 0
}

// Translated from 005861d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears `cFlags` bit 0.
pub fn fn_005861d0(e: &mut Engine, this: Ptr<TESWorldSpace>, set: bool) {
    let flags = e.get(this, TESWorldSpace::cFlags);
    let flags = if set { flags | 0x01 } else { flags & !0x01 };
    e.set(this, TESWorldSpace::cFlags, flags);
}

// Translated from 00586210 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cFlags` bit 1.
pub fn fn_00586210(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.get(this, TESWorldSpace::cFlags) & 0x02 != 0
}

// Translated from 00586230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Form flag `0x80000` (the form flags word is read through `0044ddc0`).
pub fn fn_00586230(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.call(FORM_FLAGS, &args![this]).u32() & 0x8_0000 != 0
}

// Translated from 00586260 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWorldSpace::GetHasBorderRegion` (Xbox PDB): `cFlags` bit 3.
pub fn tes_world_space_get_has_border_region(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.get(this, TESWorldSpace::cFlags) & 0x08 != 0
}

// Translated from 00586280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears `cFlags` bit 3 (`GetHasBorderRegion`).
pub fn fn_00586280(e: &mut Engine, this: Ptr<TESWorldSpace>, set: bool) {
    let flags = e.get(this, TESWorldSpace::cFlags);
    let flags = if set { flags | 0x08 } else { flags & !0x08 };
    e.set(this, TESWorldSpace::cFlags, flags);
}

// Translated from 005862c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cFlags` bit 4.
pub fn fn_005862c0(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.get(this, TESWorldSpace::cFlags) & 0x10 != 0
}

// Translated from 005862e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cFlags` bit 5.
pub fn fn_005862e0(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.get(this, TESWorldSpace::cFlags) & 0x20 != 0
}

// Translated from 00586300 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cFlags` bit 7.
pub fn fn_00586300(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.get(this, TESWorldSpace::cFlags) & 0x80 != 0
}

// Translated from 00586320 (decompiled, FalloutNV.exe 1.4.0.525)
/// The inverse of `cFlags` bit 6.
pub fn fn_00586320(e: &mut Engine, this: Ptr<TESWorldSpace>) -> bool {
    e.get(this, TESWorldSpace::cFlags) & 0x40 == 0
}

// Translated from 00586340 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether this world space takes value number `which` (0 to 5) from its parent: bit
/// `which` of `sParentUseFlags`. Any other number: true.
pub fn fn_00586340(e: &mut Engine, this: Ptr<TESWorldSpace>, which: i32) -> bool {
    if !(0..6).contains(&which) {
        return true;
    }
    let bit = (1u32 << which) as u8 as u16;
    e.get(this, TESWorldSpace::sParentUseFlags) & bit != 0
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004fd380,
            tes_world_space_get_parent_world_simple(Ptr<TESWorldSpace>) -> Ptr
        ),
        entry!(
            0x00583360,
            tes_world_space_load_partial(Ptr<TESWorldSpace>, Ptr) -> bool
        ),
        entry!(0x00583560, fn_00583560(Ptr<TESWorldSpace>, Ptr) -> bool),
        entry!(0x005840b0, fn_005840b0(Ptr)),
        entry!(0x00584150, fn_00584150(Ptr<TESWorldSpace>, f32)),
        entry!(0x00584170, fn_00584170(Ptr<TESWorldSpace>)),
        entry!(
            0x00584530,
            fn_00584530(Ptr<TESWorldSpace>, Ptr, i32, i32) -> i32
        ),
        entry!(0x005845e0, tes_world_space_init_item(Ptr<TESWorldSpace>)),
        entry!(0x00584bc0, fn_00584bc0(Ptr<TESWorldSpace>, Ptr)),
        entry!(0x00584be0, fn_00584be0(Ptr<TESWorldSpace>, Ptr) -> bool),
        entry!(0x00584d40, fn_00584d40(Ptr<TESWorldSpace>, Ptr) -> bool),
        entry!(0x00584e60, fn_00584e60(Ptr<TESWorldSpace>, u8, Ptr) -> Ptr),
        entry!(0x00584fa0, fn_00584fa0(Ptr<TESWorldSpace>, Ptr)),
        entry!(0x00585250, fn_00585250(Ptr<TESWorldSpace>, Ptr) -> bool),
        entry!(
            0x005854f0,
            tes_world_space_find_cell_in_file(Ptr<TESWorldSpace>, Ptr, i32, i32) -> bool
        ),
        entry!(0x005857b0, fn_005857b0(Ptr<TESWorldSpace>, Ptr, u32)),
        entry!(0x005858b0, fn_005858b0(Ptr<TESWorldSpace>, Ptr) -> bool),
        entry!(
            0x00585940,
            tes_world_space_get_ext_cell_data_from_file_by_editor_id(
                Ptr<TESWorldSpace>,
                Ptr,
                Ptr,
                Ptr,
            ) -> bool
        ),
        entry!(
            0x00585b30,
            tes_world_space_load_cell(Ptr<TESWorldSpace>, i32, i32) -> Ptr
        ),
        entry!(
            0x00585e00,
            tes_world_space_un_load_cell(Ptr<TESWorldSpace>, Ptr)
        ),
        entry!(
            0x00585ee0,
            tes_world_space_find_land_data_in_file(Ptr<TESWorldSpace>, i32, i32, Ptr, Ptr) -> bool
        ),
        entry!(0x00585fe0, fn_00585fe0(Ptr<TESWorldSpace>) -> Ptr),
        entry!(0x00586020, fn_00586020(Ptr<TESWorldSpace>) -> Ptr),
        entry!(0x00586070, fn_00586070(Ptr<TESWorldSpace>) -> Ptr),
        entry!(0x005860c0, fn_005860c0(Ptr<TESWorldSpace>) -> Ptr),
        entry!(0x00586110, fn_00586110(Ptr<TESWorldSpace>, f32)),
        entry!(0x00586130, fn_00586130(Ptr<TESWorldSpace>, f32)),
        entry!(0x00586150, fn_00586150(Ptr<TESWorldSpace>) -> Ptr),
        entry!(
            0x00586170,
            tes_world_space_get_terrain_manager(Ptr<TESWorldSpace>) -> Ptr
        ),
        entry!(0x005861b0, fn_005861b0(Ptr<TESWorldSpace>) -> bool),
        entry!(0x005861d0, fn_005861d0(Ptr<TESWorldSpace>, bool)),
        entry!(0x00586210, fn_00586210(Ptr<TESWorldSpace>) -> bool),
        entry!(0x00586230, fn_00586230(Ptr<TESWorldSpace>) -> bool),
        entry!(
            0x00586260,
            tes_world_space_get_has_border_region(Ptr<TESWorldSpace>) -> bool
        ),
        entry!(0x00586280, fn_00586280(Ptr<TESWorldSpace>, bool)),
        entry!(0x005862c0, fn_005862c0(Ptr<TESWorldSpace>) -> bool),
        entry!(0x005862e0, fn_005862e0(Ptr<TESWorldSpace>) -> bool),
        entry!(0x00586300, fn_00586300(Ptr<TESWorldSpace>) -> bool),
        entry!(0x00586320, fn_00586320(Ptr<TESWorldSpace>) -> bool),
        entry!(0x00586340, fn_00586340(Ptr<TESWorldSpace>, i32) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn ret(v: u32) -> Ret {
        Ret {
            eax: v,
            ..Ret::default()
        }
    }

    fn ret_float(v: f32) -> Ret {
        Ret {
            st0: v as f64,
            ..Ret::default()
        }
    }

    /// An engine with the callees every function reaches: `__RTDynamicCast`
    /// (the cast always succeeds), the `+0xC` word getter and the global
    /// record tags, which the exe would have initialized.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.register(RT_DYNAMIC_CAST, |_, a| ret(a[0]));
        e.register(WORD_AT_0C, |e, a| ret(e.mem.u32(a[0] + 0xC)));
        for (addr, value) in [
            (GROUP_TAG, 0x5055_5247u32),
            (CELL_TAG, 0x4c4c_4543),
            (SKIPPED_RECORD_TAG_01187338, 0x4552_4652),
            (WORLD_SPACE_RECORD_TYPE, 0x41),
        ] {
            set_word(&mut e, addr, value);
        }
        e
    }

    fn set_word(e: &mut Engine, addr: u32, value: u32) {
        if !e.mem.is_mapped(addr) {
            e.map(addr & !0xfff, 0x1000);
        }
        e.set_global(addr, value);
    }

    /// A double that always returns `value` in EAX.
    fn stub(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| ret(value));
    }

    /// A double that does nothing.
    fn noop(e: &mut Engine, addr: u32) {
        e.register(addr, |_, _| Ret::default());
    }

    fn world(e: &mut Engine) -> Ptr<TESWorldSpace> {
        e.new_object()
    }

    /// Gives `object` a vtable whose slots (byte offsets) go to `targets`.
    fn give_vtable(e: &mut Engine, object: u32, targets: &[(u32, u32)]) {
        let size = targets.iter().map(|(s, _)| s + 4).max().unwrap_or(4);
        let table = e.mem.alloc(size);
        for (slot, target) in targets {
            e.mem.set_u32(table + slot, *target);
        }
        e.mem.set_u32(object, table);
    }

    fn calls_to(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn logged(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The plugin reader: a script of chunks (tag, data), served the way the
    /// chunk functions of `TESFile` serve them.
    #[derive(Default)]
    struct Reader {
        chunks: Vec<(u32, Vec<u8>)>,
        position: usize,
        swapped: bool,
    }

    fn install_reader(e: &mut Engine, chunks: Vec<(u32, Vec<u8>)>) -> Rc<RefCell<Reader>> {
        let reader = Rc::new(RefCell::new(Reader {
            chunks,
            ..Reader::default()
        }));
        let r = reader.clone();
        e.register_double(FILE_NEXT_CHUNK, move |_, _| {
            let r = r.borrow();
            ret(r.chunks.get(r.position).map_or(0, |c| c.0))
        });
        let r = reader.clone();
        e.register_double(FILE_ADVANCE_CHUNK, move |_, _| {
            let mut r = r.borrow_mut();
            r.position += 1;
            ret((r.position < r.chunks.len()) as u32)
        });
        let r = reader.clone();
        e.register_double(FILE_CHUNK_SIZE, move |_, _| {
            let r = r.borrow();
            ret(r.chunks[r.position].1.len() as u32)
        });
        let r = reader.clone();
        e.register_double(FILE_READ_CHUNK, move |e, a| {
            let r = r.borrow();
            let data = &r.chunks[r.position].1;
            // A size of 0 means the whole chunk.
            let size = if a[2] == 0 {
                data.len()
            } else {
                (a[2] as usize).min(data.len())
            };
            e.mem.write(a[1], &data[..size]);
            ret(size as u32)
        });
        let r = reader.clone();
        e.register_double(FILE_READ_CHUNK_U32, move |e, a| {
            let r = r.borrow();
            let data = &r.chunks[r.position].1;
            e.mem.write(a[1], &data[..4]);
            Ret::default()
        });
        let r = reader.clone();
        e.register_double(FILE_READ_CHUNK_U16, move |e, a| {
            let r = r.borrow();
            let data = &r.chunks[r.position].1;
            e.mem.write(a[1], &data[..2]);
            Ret::default()
        });
        let r = reader.clone();
        e.register_double(FILE_NEEDS_SWAP, move |_, _| ret(r.borrow().swapped as u32));
        reader
    }

    fn floats(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    fn words(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// `min` and `max` as `0040ebd0` and `00404010` compute them.
    fn install_min_max(e: &mut Engine) {
        e.register(FLOAT_MIN, |_, a| {
            let (x, y) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            ret_float(if y <= x { y } else { x })
        });
        e.register(FLOAT_MAX, |_, a| {
            let (x, y) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            ret_float(if x <= y { y } else { x })
        });
    }

    // ---- accessors ------------------------------------------------------

    #[test]
    fn parent_world_simple_returns_the_field() {
        let mut e = engine();
        let w = world(&mut e);
        e.set(w, TESWorldSpace::pParentWorld, Ptr::new(0x1234));
        assert_eq!(e.call(0x004f_d380, &args![w]).u32(), 0x1234);
    }

    #[test]
    fn plain_setters_and_getters() {
        let mut e = engine();
        let w = world(&mut e);
        e.call(0x0058_4150, &args![w, 12.5f32]);
        assert_eq!(e.get(w, TESWorldSpace::fLODWaterHeight), 12.5);
        e.call(0x0058_4bc0, &args![w, 0x7777u32]);
        assert_eq!(e.get(w, TESWorldSpace::pEncounterZone).addr(), 0x7777);
        e.call(0x0058_6110, &args![w, -3.0f32]);
        assert_eq!(e.get(w, TESWorldSpace::fDefaultLandHeight), -3.0);
        e.call(0x0058_6130, &args![w, 4.5f32]);
        assert_eq!(e.get(w, TESWorldSpace::fDefaultWaterHeight), 4.5);
        e.set(w, TESWorldSpace::pMusicType, Ptr::new(0x4242));
        assert_eq!(e.call(0x0058_6150, &args![w]).u32(), 0x4242);
    }

    #[test]
    fn flag_accessors_read_and_write_their_bit() {
        let mut e = engine();
        let w = world(&mut e);
        // (getter, bit)
        let getters = [
            (0x0058_61b0u32, 0x01u8),
            (0x0058_6210, 0x02),
            (0x0058_6260, 0x08),
            (0x0058_62c0, 0x10),
            (0x0058_62e0, 0x20),
            (0x0058_6300, 0x80),
        ];
        for (getter, bit) in getters {
            e.set(w, TESWorldSpace::cFlags, 0);
            assert!(!e.call(getter, &args![w]).bool(), "{getter:08x} clear");
            e.set(w, TESWorldSpace::cFlags, bit);
            assert!(e.call(getter, &args![w]).bool(), "{getter:08x} set");
            // Every other bit alone does not set it.
            e.set(w, TESWorldSpace::cFlags, !bit);
            assert!(!e.call(getter, &args![w]).bool(), "{getter:08x} others");
        }
        // The bit 6 accessor is the inverse.
        e.set(w, TESWorldSpace::cFlags, 0);
        assert!(e.call(0x0058_6320, &args![w]).bool());
        e.set(w, TESWorldSpace::cFlags, 0x40);
        assert!(!e.call(0x0058_6320, &args![w]).bool());
        // The setters keep the other bits.
        e.set(w, TESWorldSpace::cFlags, 0xA5);
        e.call(0x0058_61d0, &args![w, false]);
        assert_eq!(e.get(w, TESWorldSpace::cFlags), 0xA4);
        e.call(0x0058_61d0, &args![w, true]);
        assert_eq!(e.get(w, TESWorldSpace::cFlags), 0xA5);
        e.call(0x0058_6280, &args![w, true]);
        assert_eq!(e.get(w, TESWorldSpace::cFlags), 0xAD);
        e.call(0x0058_6280, &args![w, false]);
        assert_eq!(e.get(w, TESWorldSpace::cFlags), 0xA5);
    }

    #[test]
    fn form_flag_0x80000_is_read_through_the_flags_getter() {
        let mut e = engine();
        let w = world(&mut e);
        stub(&mut e, FORM_FLAGS, 0x0008_0000);
        assert!(e.call(0x0058_6230, &args![w]).bool());
        stub(&mut e, FORM_FLAGS, 0xfff7_ffff);
        assert!(!e.call(0x0058_6230, &args![w]).bool());
    }

    #[test]
    fn uses_parent_value_checks_one_bit_of_the_parent_flags() {
        let mut e = engine();
        let w = world(&mut e);
        e.set(w, TESWorldSpace::sParentUseFlags, 0b0001_0100);
        for (which, expected) in [
            (0, false),
            (1, false),
            (2, true),
            (3, false),
            (4, true),
            (5, false),
        ] {
            assert_eq!(
                e.call(0x0058_6340, &args![w, which as u32]).bool(),
                expected,
                "{which}"
            );
        }
        // Outside 0..6 the answer is true whatever the flags are.
        e.set(w, TESWorldSpace::sParentUseFlags, 0);
        for which in [-1i32, 6, 100] {
            assert!(e.call(0x0058_6340, &args![w, which]).bool(), "{which}");
        }
    }

    /// `00586390` stand-in: a table of (world, value number) -> parent.
    fn install_parents(e: &mut Engine, table: Vec<(u32, u32, u32)>) {
        e.register_double(PARENT_FOR, move |_, a| {
            ret(table
                .iter()
                .find(|(w, n, _)| *w == a[0] && *n == a[1])
                .map_or(0, |(_, _, p)| *p))
        });
    }

    #[test]
    fn inherited_getters_follow_the_parent_that_owns_the_value() {
        let mut e = engine();
        let child = world(&mut e);
        let parent = world(&mut e);
        let default_water = 0x00dd_0001;
        set_word(&mut e, DEFAULT_WATER, default_water);
        install_parents(
            &mut e,
            vec![
                (child.addr(), 4, parent.addr()),
                (child.addr(), 5, parent.addr()),
                (child.addr(), 3, parent.addr()),
                (child.addr(), 1, parent.addr()),
            ],
        );
        e.set(child, TESWorldSpace::pClimate, Ptr::new(0xc1));
        e.set(parent, TESWorldSpace::pClimate, Ptr::new(0xc2));
        assert_eq!(e.call(0x0058_5fe0, &args![child]).u32(), 0xc2);
        assert_eq!(e.call(0x0058_5fe0, &args![parent]).u32(), 0xc2);

        // Image space: the own one first, the parent's only when it is null.
        e.set(child, TESWorldSpace::pImageSpace, Ptr::new(0x15));
        e.set(parent, TESWorldSpace::pImageSpace, Ptr::new(0x25));
        assert_eq!(e.call(0x0058_6020, &args![child]).u32(), 0x15);
        e.set(child, TESWorldSpace::pImageSpace, Ptr::NULL);
        assert_eq!(e.call(0x0058_6020, &args![child]).u32(), 0x25);
        e.set(parent, TESWorldSpace::pImageSpace, Ptr::NULL);
        assert_eq!(e.call(0x0058_6020, &args![child]).u32(), 0);

        // Water and lod water: parent, else own, else the default water.
        e.set(parent, TESWorldSpace::pWorldWater, Ptr::new(0x31));
        assert_eq!(e.call(0x0058_6070, &args![child]).u32(), 0x31);
        e.set(parent, TESWorldSpace::pWorldWater, Ptr::NULL);
        assert_eq!(e.call(0x0058_6070, &args![child]).u32(), default_water);
        e.set(parent, TESWorldSpace::pWorldWater, Ptr::new(0x32));
        assert_eq!(e.call(0x0058_6070, &args![parent]).u32(), 0x32);
        e.set(parent, TESWorldSpace::pLODWater, Ptr::new(0x41));
        assert_eq!(e.call(0x0058_60c0, &args![child]).u32(), 0x41);
        e.set(parent, TESWorldSpace::pLODWater, Ptr::NULL);
        assert_eq!(e.call(0x0058_60c0, &args![child]).u32(), default_water);
        e.set(parent, TESWorldSpace::pLODWater, Ptr::new(0x42));
        assert_eq!(e.call(0x0058_60c0, &args![parent]).u32(), 0x42);

        // Terrain manager.
        e.set(child, TESWorldSpace::pTerrainManager, Ptr::new(0x51));
        e.set(parent, TESWorldSpace::pTerrainManager, Ptr::new(0x52));
        assert_eq!(e.call(0x0058_6170, &args![child]).u32(), 0x52);
        assert_eq!(e.call(0x0058_6170, &args![parent]).u32(), 0x52);
    }

    #[test]
    fn a_world_without_a_parent_reads_its_own_values() {
        let mut e = engine();
        let w = world(&mut e);
        install_parents(&mut e, vec![]);
        e.set(w, TESWorldSpace::pClimate, Ptr::new(0xc1));
        e.set(w, TESWorldSpace::pTerrainManager, Ptr::new(0x51));
        assert_eq!(e.call(0x0058_5fe0, &args![w]).u32(), 0xc1);
        assert_eq!(e.call(0x0058_6170, &args![w]).u32(), 0x51);
    }

    // ---- record loading and saving -----------------------------------------

    /// A texture and the path it was given.
    type NamedTexture = (u32, Vec<u8>);

    /// Swaps a value in place, as the endian swap helpers do.
    fn install_swaps(e: &mut Engine) {
        e.register(SWAP_DWORD, |e, a| {
            let v = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], v.swap_bytes());
            Ret::default()
        });
        e.register(SWAP_WORD, |e, a| {
            let v = e.mem.u16(a[0]);
            e.mem.set_u16(a[0], v.swap_bytes());
            Ret::default()
        });
    }

    fn corner_world(e: &mut Engine) -> Ptr<TESWorldSpace> {
        let w = world(e);
        e.set(w, TESWorldSpace::MinimumCoords_x, f32::MAX);
        e.set(w, TESWorldSpace::MinimumCoords_y, f32::MAX);
        e.set(w, TESWorldSpace::MaximumCoords_x, -f32::MAX);
        e.set(w, TESWorldSpace::MaximumCoords_y, -f32::MAX);
        w
    }

    #[test]
    fn load_partial_widens_the_corners_and_records_them_for_a_master() {
        let mut e = engine();
        install_min_max(&mut e);
        noop(&mut e, LOCAL_STRUCT_CONSTRUCT);
        stub(&mut e, FILE_IS_MASTER, 1);
        let data: Ptr<OffsetData> = e.new_object();
        stub(&mut e, CREATE_OFFSET_DATA, data.addr());
        let w = corner_world(&mut e);
        install_reader(
            &mut e,
            vec![
                (CHUNK_MIN_COORDS, floats(&[-10.0, -20.0])),
                (CHUNK_DATA, words(&[1])),
                (CHUNK_MAX_COORDS, floats(&[30.0, 40.0])),
            ],
        );
        logged(&mut e);
        assert!(e.call(0x0058_3360, &args![w, 0xf11eu32]).bool());
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_x), -10.0);
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_y), -20.0);
        assert_eq!(e.get(w, TESWorldSpace::MaximumCoords_x), 30.0);
        assert_eq!(e.get(w, TESWorldSpace::MaximumCoords_y), 40.0);
        assert_eq!(e.get(data, OffsetData::OffsetMinCoords_x), -10.0);
        assert_eq!(e.get(data, OffsetData::OffsetMinCoords_y), -20.0);
        assert_eq!(e.get(data, OffsetData::OffsetMaxCoords_x), 30.0);
        assert_eq!(e.get(data, OffsetData::OffsetMaxCoords_y), 40.0);
        // The file's own corners are recorded for each of the two chunks.
        assert_eq!(calls_to(&e, CREATE_OFFSET_DATA).len(), 2);
        assert_eq!(calls_to(&e, CREATE_OFFSET_DATA)[0], vec![w.addr(), 0xf11e]);
    }

    #[test]
    fn load_partial_keeps_a_wider_world_and_ignores_the_offset_data_of_a_plugin() {
        let mut e = engine();
        install_min_max(&mut e);
        noop(&mut e, LOCAL_STRUCT_CONSTRUCT);
        stub(&mut e, FILE_IS_MASTER, 0);
        let w = corner_world(&mut e);
        e.set(w, TESWorldSpace::MinimumCoords_x, -50.0);
        e.set(w, TESWorldSpace::MinimumCoords_y, 5.0);
        e.set(w, TESWorldSpace::MaximumCoords_x, 100.0);
        e.set(w, TESWorldSpace::MaximumCoords_y, 6.0);
        install_reader(
            &mut e,
            vec![
                (CHUNK_MIN_COORDS, floats(&[-10.0, -20.0])),
                (CHUNK_MAX_COORDS, floats(&[30.0, 40.0])),
            ],
        );
        logged(&mut e);
        assert!(e.call(0x0058_3360, &args![w, 0xf11eu32]).bool());
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_x), -50.0);
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_y), -20.0);
        assert_eq!(e.get(w, TESWorldSpace::MaximumCoords_x), 100.0);
        assert_eq!(e.get(w, TESWorldSpace::MaximumCoords_y), 40.0);
        assert!(calls_to(&e, CREATE_OFFSET_DATA).is_empty());
    }

    #[test]
    fn load_partial_swaps_the_corners_of_a_big_endian_file() {
        let mut e = engine();
        install_min_max(&mut e);
        install_swaps(&mut e);
        noop(&mut e, LOCAL_STRUCT_CONSTRUCT);
        stub(&mut e, FILE_IS_MASTER, 0);
        let w = corner_world(&mut e);
        let reader = install_reader(
            &mut e,
            vec![(
                CHUNK_MIN_COORDS,
                [(-10.0f32).to_be_bytes(), 7.0f32.to_be_bytes()].concat(),
            )],
        );
        reader.borrow_mut().swapped = true;
        assert!(e.call(0x0058_3360, &args![w, 0xf11eu32]).bool());
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_x), -10.0);
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_y), 7.0);
    }

    #[test]
    fn load_partial_of_an_empty_record_does_nothing() {
        let mut e = engine();
        let w = corner_world(&mut e);
        install_reader(&mut e, vec![]);
        assert!(e.call(0x0058_3360, &args![w, 0xf11eu32]).bool());
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_x), f32::MAX);
    }

    /// An engine for `Load`: a reader over `chunks`, benign doubles for
    /// everything the chunks call, and a world space whose cell map is not
    /// empty.
    fn load_engine(
        chunks: Vec<(u32, Vec<u8>)>,
    ) -> (Engine, Ptr<TESWorldSpace>, Rc<RefCell<Reader>>) {
        let mut e = engine();
        install_min_max(&mut e);
        install_swaps(&mut e);
        let reader = install_reader(&mut e, chunks);
        stub(&mut e, FILE_RECORD_TYPE, RECORD_TYPE_WORLD_SPACE);
        stub(&mut e, FILE_IS_MASTER, 0);
        for addr in [
            FORM_LOAD,
            FORM_SET_STATE,
            SET_CLIMATE,
            SET_IMAGE_SPACE,
            SET_WATER,
            SET_LOD_WATER,
            SET_MUSIC_TYPE,
            SET_PARENT_WORLD,
            LOCAL_STRUCT_CONSTRUCT,
            TEXTURE_LOAD_CHUNK,
            FULL_NAME_LOAD,
            TEXTURE_SET_NAME,
            SWAP_PAIR,
            SWAP_MAP_OFFSET_BLOCK,
            IMPACT_SWAP_LOAD,
        ] {
            noop(&mut e, addr);
        }
        let w = corner_world(&mut e);
        let map: Ptr<crate::types::NiTPointerMap> = e.new_object();
        e.set(map, crate::types::NiTPointerMap::m_uiCount, 3);
        e.set(w, TESWorldSpace::pCellMap, map.cast());
        (e, w, reader)
    }

    #[test]
    fn load_skips_records_of_other_types() {
        let (mut e, w, _) = load_engine(vec![(CHUNK_CLIMATE, words(&[1]))]);
        stub(&mut e, FILE_RECORD_TYPE, 0x42);
        logged(&mut e);
        assert!(!e.call(0x0058_3560, &args![w, 0xf11eu32]).bool());
        assert!(calls_to(&e, FORM_LOAD).is_empty());
        assert!(calls_to(&e, SET_CLIMATE).is_empty());
    }

    #[test]
    fn load_reads_the_form_references_and_scalars() {
        let (mut e, w, _) = load_engine(vec![
            (CHUNK_DATA, words(&[0x1234_5678])),
            (CHUNK_CLIMATE, words(&[0xa1])),
            (CHUNK_WATER, words(&[0xa2])),
            (CHUNK_LOD_WATER, words(&[0xa3])),
            (CHUNK_LOD_WATER_HEIGHT, floats(&[99.5])),
            (CHUNK_IMAGE_SPACE, words(&[0xa4])),
            (CHUNK_PARENT_USE_FLAGS, vec![5, 0]),
            (CHUNK_PARENT_WORLD, words(&[0xa5])),
            (CHUNK_MUSIC_TYPE, words(&[0xa6])),
            (CHUNK_ENCOUNTER_ZONE, words(&[0xa7])),
            (CHUNK_SKIPPED, words(&[0xa8])),
            (CHUNK_DEFAULT_HEIGHTS, floats(&[1.5, 2.5])),
            (tag(b"JUNK"), words(&[0])),
        ]);
        logged(&mut e);
        let file = 0xf11eu32;
        assert!(e.call(0x0058_3560, &args![w, file]).bool());
        assert_eq!(calls_to(&e, FORM_LOAD), vec![vec![w.addr(), file]]);
        assert_eq!(calls_to(&e, FORM_SET_STATE), vec![vec![w.addr(), 0]]);
        assert_eq!(e.get(w, TESWorldSpace::cFlags), 0x78);
        assert_eq!(calls_to(&e, SET_CLIMATE), vec![vec![w.addr(), 0xa1]]);
        assert_eq!(calls_to(&e, SET_WATER), vec![vec![w.addr(), 0xa2]]);
        assert_eq!(calls_to(&e, SET_LOD_WATER), vec![vec![w.addr(), 0xa3]]);
        assert_eq!(e.get(w, TESWorldSpace::fLODWaterHeight), 99.5);
        assert_eq!(calls_to(&e, SET_IMAGE_SPACE), vec![vec![w.addr(), 0xa4]]);
        assert_eq!(calls_to(&e, SET_PARENT_WORLD), vec![vec![w.addr(), 0xa5]]);
        // The parent chunk resets the parent-use flags the earlier chunk set.
        assert_eq!(e.get(w, TESWorldSpace::sParentUseFlags), 0xffff);
        assert_eq!(calls_to(&e, SET_MUSIC_TYPE), vec![vec![w.addr(), 0xa6]]);
        assert_eq!(e.get(w, TESWorldSpace::pEncounterZone).addr(), 0xa7);
        assert_eq!(e.get(w, TESWorldSpace::fDefaultLandHeight), 1.5);
        assert_eq!(e.get(w, TESWorldSpace::fDefaultWaterHeight), 2.5);
    }

    #[test]
    fn load_data_chunk_of_one_byte_is_read_into_the_flags() {
        let (mut e, w, _) = load_engine(vec![(CHUNK_DATA, vec![0x21])]);
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(e.get(w, TESWorldSpace::cFlags), 0x21);
        assert_eq!(
            calls_to(&e, FILE_READ_CHUNK),
            vec![vec![0xf11e, w.addr() + 0x4C, 1]]
        );
    }

    #[test]
    fn load_parent_use_flags_are_read_as_a_short() {
        let (mut e, w, _) = load_engine(vec![(CHUNK_PARENT_USE_FLAGS, vec![0x2b, 0x01])]);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(e.get(w, TESWorldSpace::sParentUseFlags), 0x012b);
    }

    #[test]
    fn load_of_a_master_keeps_the_offset_table_and_the_record_position() {
        let (mut e, w, reader) = load_engine(vec![(CHUNK_OFFSETS, words(&[0x100, 0, 0x300]))]);
        stub(&mut e, FILE_IS_MASTER, 1);
        let data: Ptr<OffsetData> = e.new_object();
        let old_table = e.mem.alloc(8);
        e.set(data, OffsetData::pCellFileOffsets, Ptr::new(old_table));
        stub(&mut e, CREATE_OFFSET_DATA, data.addr());
        stub(&mut e, FILE_CURRENT_OFFSET, 0x4_0000);
        logged(&mut e);
        reader.borrow_mut().swapped = false;
        assert!(e.call(0x0058_3560, &args![w, 0xf11eu32]).bool());
        assert_eq!(e.get(data, OffsetData::iFileOffset), 0x4_0000);
        let table = e.get(data, OffsetData::pCellFileOffsets);
        assert_ne!(table.addr(), old_table);
        assert_eq!(e.mem.block_size(old_table), None, "old table freed");
        assert!(e.mem.block_size(table.addr()).unwrap() >= 12);
        assert_eq!(e.mem.u32(table.addr()), 0x100);
        assert_eq!(e.mem.u32(table.addr() + 4), 0);
        assert_eq!(e.mem.u32(table.addr() + 8), 0x300);
        assert!(calls_to(&e, SWAP_DWORD).is_empty());
    }

    #[test]
    fn load_swaps_each_entry_of_a_big_endian_offset_table() {
        let (mut e, w, reader) = load_engine(vec![(
            CHUNK_OFFSETS,
            [0x100u32.to_be_bytes(), 0x2000u32.to_be_bytes()].concat(),
        )]);
        stub(&mut e, FILE_IS_MASTER, 1);
        let data: Ptr<OffsetData> = e.new_object();
        stub(&mut e, CREATE_OFFSET_DATA, data.addr());
        stub(&mut e, FILE_CURRENT_OFFSET, 0);
        reader.borrow_mut().swapped = true;
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        let table = e.get(data, OffsetData::pCellFileOffsets).addr();
        assert_eq!(e.mem.u32(table), 0x100);
        assert_eq!(e.mem.u32(table + 4), 0x2000);
    }

    #[test]
    fn load_with_an_empty_offset_chunk_leaves_the_table_alone() {
        let (mut e, w, _) = load_engine(vec![(CHUNK_OFFSETS, vec![])]);
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert!(calls_to(&e, CREATE_OFFSET_DATA).is_empty());
    }

    #[test]
    fn load_text_chunks_go_to_their_setters() {
        let (mut e, w, _) = load_engine(vec![
            (CHUNK_EDITOR_ID, b"Wasteland\0".to_vec()),
            (CHUNK_CANOPY_TEXTURE, b"canopy.dds\0".to_vec()),
            (CHUNK_WATER_NOISE_TEXTURE, b"noise.dds\0".to_vec()),
            (CHUNK_ICON, b"icon\0".to_vec()),
            (CHUNK_FULL_NAME, b"Mojave\0".to_vec()),
            (CHUNK_OBJECT_BOUNDS, vec![0; 12]),
        ]);
        // The editor ID setter (slot 0x134) and the bounds loader (slot 0xe0).
        let seen: Rc<RefCell<Vec<Vec<u8>>>> = Rc::default();
        let s = seen.clone();
        e.register_double(0x0058_3330, move |e, a| {
            s.borrow_mut().push(e.mem.cstr(a[1]));
            ret(1)
        });
        noop(&mut e, 0x0058_0e00);
        give_vtable(
            &mut e,
            w.addr(),
            &[(0x134, 0x0058_3330), (0xE0, 0x0058_0e00)],
        );
        let names: Rc<RefCell<Vec<NamedTexture>>> = Rc::default();
        let n = names.clone();
        e.register_double(TEXTURE_SET_NAME, move |e, a| {
            n.borrow_mut().push((a[0], e.mem.cstr(a[1])));
            Ret::default()
        });
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(*seen.borrow(), vec![b"Wasteland".to_vec()]);
        assert_eq!(
            *names.borrow(),
            vec![
                (w.addr() + 0xD4, b"canopy.dds".to_vec()),
                (w.addr() + 0xE0, b"noise.dds".to_vec()),
            ]
        );
        assert_eq!(
            calls_to(&e, TEXTURE_LOAD_CHUNK),
            vec![vec![w.addr() + 0x24, 0xf11e]]
        );
        assert_eq!(
            calls_to(&e, FULL_NAME_LOAD),
            vec![vec![w.addr() + 0x18, 0xf11e]]
        );
        assert_eq!(calls_to(&e, 0x0058_0e00), vec![vec![w.addr(), 0xf11e]]);
    }

    #[test]
    fn load_empty_texture_name_chunks_set_nothing() {
        let (mut e, w, _) = load_engine(vec![
            (CHUNK_CANOPY_TEXTURE, vec![]),
            (CHUNK_WATER_NOISE_TEXTURE, vec![]),
        ]);
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert!(calls_to(&e, TEXTURE_SET_NAME).is_empty());
    }

    #[test]
    fn load_map_blocks_are_copied_and_swapped_for_big_endian_files() {
        let map_data: Vec<u8> = (1..=16).collect();
        let offset_data: Vec<u8> = (101..=112).collect();
        let (mut e, w, reader) = load_engine(vec![
            (CHUNK_MAP_DATA, map_data.clone()),
            (CHUNK_MAP_OFFSET_DATA, offset_data.clone()),
        ]);
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(e.mem.bytes(w.addr() + 0x80, 16), map_data);
        assert_eq!(e.mem.bytes(w.addr() + 0x90, 12), offset_data);
        assert!(calls_to(&e, SWAP_DWORD).is_empty());
        assert!(calls_to(&e, SWAP_MAP_OFFSET_BLOCK).is_empty());

        reader.borrow_mut().swapped = true;
        reader.borrow_mut().position = 0;
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        // The map data swap: two words and four shorts.
        assert_eq!(calls_to(&e, SWAP_DWORD).len(), 2);
        assert_eq!(calls_to(&e, SWAP_WORD).len(), 4);
        assert_eq!(
            calls_to(&e, SWAP_MAP_OFFSET_BLOCK),
            vec![vec![w.addr() + 0x90]]
        );
    }

    #[test]
    fn load_default_heights_are_swapped_when_the_file_is() {
        let (mut e, w, reader) = load_engine(vec![(CHUNK_DEFAULT_HEIGHTS, floats(&[1.0, 2.0]))]);
        reader.borrow_mut().swapped = true;
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(calls_to(&e, SWAP_PAIR).len(), 1);
    }

    #[test]
    fn load_corner_chunks_are_those_of_load_partial() {
        let (mut e, w, _) = load_engine(vec![
            (CHUNK_MIN_COORDS, floats(&[-1.0, -2.0])),
            (CHUNK_MAX_COORDS, floats(&[3.0, 4.0])),
        ]);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(e.get(w, TESWorldSpace::MinimumCoords_x), -1.0);
        assert_eq!(e.get(w, TESWorldSpace::MaximumCoords_y), 4.0);
    }

    #[test]
    fn load_creates_the_impact_swap_once() {
        let (mut e, w, _) = load_engine(vec![
            (CHUNK_IMPACT_FIRST, vec![]),
            (CHUNK_IMPACT_SECOND, vec![]),
        ]);
        e.register(IMPACT_SWAP_CONSTRUCT, |_, a| ret(a[0]));
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        let swap = e.get(w, TESWorldSpace::pImpactSwap);
        assert!(!swap.is_null());
        assert!(e.mem.block_size(swap.addr()).unwrap() >= IMPACT_SWAP_SIZE);
        assert_eq!(calls_to(&e, IMPACT_SWAP_CONSTRUCT).len(), 1);
        assert_eq!(
            calls_to(&e, IMPACT_SWAP_LOAD),
            vec![vec![swap.addr(), 0xf11e]; 2]
        );
    }

    #[test]
    fn load_replaces_an_empty_cell_map() {
        for (large, buckets) in [(true, 0x1b59u32), (false, 0x2bd)] {
            let (mut e, w, _) = load_engine(vec![]);
            let old: Ptr<crate::types::NiTPointerMap> = e.new_object();
            e.set(w, TESWorldSpace::pCellMap, old.cast());
            give_vtable(&mut e, old.addr(), &[(0, 0x0058_9060)]);
            noop(&mut e, 0x0058_9060);
            stub(&mut e, FORM_QUERY_LARGE_MAP, large as u32);
            e.register(CELL_MAP_CONSTRUCT, |_, a| ret(a[0]));
            logged(&mut e);
            assert!(e.call(0x0058_3560, &args![w, 0xf11eu32]).bool());
            // The old map is deleted through its destructor (delete flag 1).
            assert_eq!(calls_to(&e, 0x0058_9060), vec![vec![old.addr(), 1]]);
            let new_map = e.get(w, TESWorldSpace::pCellMap);
            assert_ne!(new_map, old.cast());
            assert!(e.mem.block_size(new_map.addr()).unwrap() >= CELL_MAP_SIZE);
            assert_eq!(
                calls_to(&e, CELL_MAP_CONSTRUCT),
                vec![vec![new_map.addr(), buckets]]
            );
        }
    }

    #[test]
    fn load_keeps_a_cell_map_that_has_cells_or_a_world_that_has_no_cells() {
        // Cells in the map.
        let (mut e, w, _) = load_engine(vec![]);
        let map = e.get(w, TESWorldSpace::pCellMap);
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(e.get(w, TESWorldSpace::pCellMap), map);
        assert!(calls_to(&e, CELL_MAP_CONSTRUCT).is_empty());
        // An empty map, but cFlags bit 0 is set.
        let (mut e, w, _) = load_engine(vec![]);
        let empty: Ptr<crate::types::NiTPointerMap> = e.new_object();
        e.set(w, TESWorldSpace::pCellMap, empty.cast());
        e.set(w, TESWorldSpace::cFlags, 1);
        logged(&mut e);
        e.call(0x0058_3560, &args![w, 0xf11eu32]);
        assert_eq!(e.get(w, TESWorldSpace::pCellMap), empty.cast());
        assert!(calls_to(&e, CELL_MAP_CONSTRUCT).is_empty());
    }

    #[test]
    fn map_data_swap_covers_two_words_and_four_shorts() {
        let mut e = engine();
        logged(&mut e);
        noop(&mut e, SWAP_DWORD);
        noop(&mut e, SWAP_WORD);
        e.call(0x0058_40b0, &args![0x9000u32]);
        assert_eq!(
            calls_to(&e, SWAP_DWORD),
            vec![vec![0x9000, 0], vec![0x9004, 0]]
        );
        assert_eq!(
            calls_to(&e, SWAP_WORD),
            vec![
                vec![0x9008, 0],
                vec![0x900a, 0],
                vec![0x900c, 0],
                vec![0x900e, 0]
            ]
        );
    }

    // ---- Save ---------------------------------------------------------------

    /// A form with `id` at `+0xC`.
    fn form_with_id(e: &mut Engine, id: u32) -> Ptr {
        let form = e.mem.alloc(0x20);
        e.mem.set_u32(form + 0xC, id);
        Ptr::new(form)
    }

    /// The chunk writes `Save` made, as (function, tag, first value).
    fn chunk_writes(e: &Engine) -> Vec<(u32, u32, u32)> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| {
                [
                    ADD_CHUNK_U32,
                    ADD_CHUNK_U16,
                    ADD_CHUNK_U8,
                    ADD_CHUNK_DATA,
                    ADD_CHUNK_BLOCK,
                    ADD_CHUNK_ARRAY,
                ]
                .contains(a)
            })
            .map(|(a, args)| (*a, args[0], args[1]))
            .collect()
    }

    fn save_engine() -> Engine {
        let mut e = engine();
        install_parents(&mut e, vec![]);
        for addr in [
            FORM_START,
            FORM_CLOSE,
            FULL_NAME_SAVE,
            ADD_CHUNK_U32,
            ADD_CHUNK_U16,
            ADD_CHUNK_U8,
            ADD_CHUNK_DATA,
            ADD_CHUNK_BLOCK,
            ADD_CHUNK_ARRAY,
            TEXTURE_SAVE,
            SWAP_PAIR,
            SWAP_MAP_OFFSET_BLOCK,
            IMPACT_SWAP_SAVE,
        ] {
            noop(&mut e, addr);
        }
        stub(&mut e, SAVING_SWAPPED, 0);
        stub(&mut e, TEXTURE_NAME_LENGTH, 4);
        stub(&mut e, TEXTURE_NAME_TEXT, 0xbeef);
        e.register(GET_LOD_WATER_HEIGHT, |_, _| ret_float(7.25));
        install_swaps(&mut e);
        set_word(&mut e, DEFAULT_WATER, 0);
        e
    }

    #[test]
    fn save_writes_every_chunk_of_a_world_space_with_its_own_values() {
        let mut e = save_engine();
        let w = world(&mut e);
        let zone = form_with_id(&mut e, 0x11);
        let parent = form_with_id(&mut e, 0x12);
        let climate = form_with_id(&mut e, 0x13);
        let water = form_with_id(&mut e, 0x14);
        let lod_water = form_with_id(&mut e, 0x15);
        let image_space = form_with_id(&mut e, 0x16);
        let music = form_with_id(&mut e, 0x17);
        e.set(w, TESWorldSpace::pEncounterZone, zone);
        e.set(w, TESWorldSpace::pParentWorld, parent);
        e.set(w, TESWorldSpace::sParentUseFlags, 0);
        e.set(w, TESWorldSpace::pClimate, climate);
        e.set(w, TESWorldSpace::pWorldWater, water);
        e.set(w, TESWorldSpace::pLODWater, lod_water);
        e.set(w, TESWorldSpace::pMusicType, music);
        e.set(w, TESWorldSpace::cFlags, 0x21);
        e.set(w, TESWorldSpace::fDefaultLandHeight, -2000.0);
        e.set(w, TESWorldSpace::fDefaultWaterHeight, 3.5);
        stub(&mut e, GET_IMAGE_SPACE_RAW, image_space.addr());
        let swap = 0x5151;
        e.set(w, TESWorldSpace::pImpactSwap, Ptr::new(swap));
        logged(&mut e);
        e.call(0x0058_4170, &args![w]);

        let writes = chunk_writes(&e);
        let expected = vec![
            (ADD_CHUNK_U32, CHUNK_ENCOUNTER_ZONE, 0x11),
            (ADD_CHUNK_U32, CHUNK_PARENT_WORLD, 0x12),
            (ADD_CHUNK_U16, CHUNK_PARENT_USE_FLAGS, 0),
            (ADD_CHUNK_U32, CHUNK_CLIMATE, 0x13),
            (ADD_CHUNK_U32, CHUNK_WATER, 0x14),
            (ADD_CHUNK_U32, CHUNK_LOD_WATER, 0x15),
            (ADD_CHUNK_U32, CHUNK_LOD_WATER_HEIGHT, 7.25f32.to_bits()),
            (ADD_CHUNK_DATA, CHUNK_DEFAULT_HEIGHTS, 0),
            (ADD_CHUNK_DATA, CHUNK_MAP_DATA, w.addr() + 0x80),
            (ADD_CHUNK_DATA, CHUNK_MAP_OFFSET_DATA, w.addr() + 0x90),
            (ADD_CHUNK_U32, CHUNK_IMAGE_SPACE, 0x16),
            (ADD_CHUNK_U8, CHUNK_DATA, 0x21),
            (ADD_CHUNK_ARRAY, CHUNK_MIN_COORDS, w.addr() + 0xA0),
            (ADD_CHUNK_ARRAY, CHUNK_MAX_COORDS, w.addr() + 0xA8),
            (ADD_CHUNK_U32, CHUNK_MUSIC_TYPE, 0x17),
            (ADD_CHUNK_BLOCK, CHUNK_CANOPY_TEXTURE, 0xbeef),
            (ADD_CHUNK_BLOCK, CHUNK_WATER_NOISE_TEXTURE, 0xbeef),
        ];
        // The height block is a stack local: compare everything else.
        assert_eq!(writes.len(), expected.len());
        for (got, want) in writes.iter().zip(&expected) {
            assert_eq!((got.0, got.1), (want.0, want.1));
            if want.1 != CHUNK_DEFAULT_HEIGHTS {
                assert_eq!(got.2, want.2, "tag {:08x}", want.1);
            }
        }
        assert_eq!(calls_to(&e, FORM_START), vec![vec![w.addr()]]);
        assert_eq!(calls_to(&e, FULL_NAME_SAVE), vec![vec![w.addr() + 0x18]]);
        assert_eq!(
            calls_to(&e, TEXTURE_SAVE),
            vec![vec![w.addr() + 0x24, CHUNK_ICON]]
        );
        assert_eq!(calls_to(&e, IMPACT_SWAP_SAVE), vec![vec![swap]]);
        assert_eq!(calls_to(&e, FORM_CLOSE), vec![vec![w.addr()]]);
        // The height block holds the two heights and is 8 bytes.
        let heights = calls_to(&e, ADD_CHUNK_DATA)
            .into_iter()
            .find(|a| a[0] == CHUNK_DEFAULT_HEIGHTS)
            .unwrap();
        assert_eq!(heights[2], 8);
        // The text chunks are the text and its length plus the terminator.
        assert_eq!(
            calls_to(&e, ADD_CHUNK_BLOCK)[0],
            vec![CHUNK_CANOPY_TEXTURE, 0xbeef, 5]
        );
    }

    #[test]
    fn save_leaves_out_the_values_a_child_takes_from_its_parent() {
        let mut e = save_engine();
        let w = world(&mut e);
        let climate = form_with_id(&mut e, 0x13);
        e.set(w, TESWorldSpace::pClimate, climate);
        e.set(w, TESWorldSpace::sParentUseFlags, 0x3f);
        stub(&mut e, GET_IMAGE_SPACE_RAW, 0);
        logged(&mut e);
        e.call(0x0058_4170, &args![w]);
        let tags: Vec<u32> = chunk_writes(&e).iter().map(|c| c.1).collect();
        // Without own values: the map offset block, the flags, the corners
        // and the texture paths are still written.
        assert_eq!(
            tags,
            vec![
                CHUNK_MAP_OFFSET_DATA,
                CHUNK_DATA,
                CHUNK_MIN_COORDS,
                CHUNK_MAX_COORDS,
                CHUNK_CANOPY_TEXTURE,
                CHUNK_WATER_NOISE_TEXTURE,
            ]
        );
        assert!(calls_to(&e, TEXTURE_SAVE).is_empty());
    }

    #[test]
    fn save_skips_unset_references_even_when_the_world_has_its_own_values() {
        let mut e = save_engine();
        let w = world(&mut e);
        e.set(w, TESWorldSpace::sParentUseFlags, 0);
        // No climate, no water at all: the water getters return the
        // default water form, which is null here.
        set_word(&mut e, DEFAULT_WATER, 0);
        stub(&mut e, GET_IMAGE_SPACE_RAW, 0);
        logged(&mut e);
        e.call(0x0058_4170, &args![w]);
        let tags: Vec<u32> = chunk_writes(&e).iter().map(|c| c.1).collect();
        assert_eq!(
            tags,
            vec![
                CHUNK_DEFAULT_HEIGHTS,
                CHUNK_MAP_DATA,
                CHUNK_MAP_OFFSET_DATA,
                CHUNK_DATA,
                CHUNK_MIN_COORDS,
                CHUNK_MAX_COORDS,
                CHUNK_CANOPY_TEXTURE,
                CHUNK_WATER_NOISE_TEXTURE,
            ]
        );
        assert!(calls_to(&e, IMPACT_SWAP_SAVE).is_empty());
    }

    #[test]
    fn save_swaps_the_blocks_around_the_write_when_saving_big_endian() {
        let mut e = save_engine();
        let w = world(&mut e);
        e.set(w, TESWorldSpace::sParentUseFlags, 0);
        set_word(&mut e, DEFAULT_WATER, 0);
        stub(&mut e, GET_IMAGE_SPACE_RAW, 0);
        stub(&mut e, SAVING_SWAPPED, 1);
        logged(&mut e);
        e.call(0x0058_4170, &args![w]);
        // Heights: swapped before and after the write.
        assert_eq!(calls_to(&e, SWAP_PAIR).len(), 2);
        // Map data: six values (two words, four shorts) before and after.
        assert_eq!(calls_to(&e, SWAP_DWORD).len(), 4);
        assert_eq!(calls_to(&e, SWAP_WORD).len(), 8);
        // Map offset data: before and after.
        assert_eq!(
            calls_to(&e, SWAP_MAP_OFFSET_BLOCK),
            vec![vec![w.addr() + 0x90]; 2]
        );
    }

    // ---- the per-file cell offset tables -------------------------------------

    const FILE: u32 = 0xf11e;
    const BASE_OFFSET: u32 = 0x1_0000;

    /// A world space and an `OFFSET_DATA` for `FILE`: a grid of cells x -2..=1,
    /// y -1..=2 (the float bounds are cell numbers times 4096), with the cells
    /// (0, 0) and (1, 2) at offsets 0x500 and 0x900.
    fn grid_engine() -> (Engine, Ptr<TESWorldSpace>, Ptr<OffsetData>) {
        let mut e = engine();
        let w = world(&mut e);
        let data: Ptr<OffsetData> = e.new_object();
        e.set(data, OffsetData::OffsetMinCoords_x, -8192.0);
        e.set(data, OffsetData::OffsetMinCoords_y, -4096.0);
        e.set(data, OffsetData::OffsetMaxCoords_x, 4096.0 + 100.0);
        e.set(data, OffsetData::OffsetMaxCoords_y, 8192.0);
        e.set(data, OffsetData::iFileOffset, BASE_OFFSET);
        let table = e.mem.alloc(16 * 4);
        e.mem.set_u32(table + 6 * 4, 0x500);
        e.mem.set_u32(table + 15 * 4, 0x900);
        e.set(data, OffsetData::pCellFileOffsets, Ptr::new(table));
        let at = data.addr();
        e.register_double(OFFSET_DATA_OF_FILE, move |_, a| {
            ret(if a[1] == FILE { at } else { 0 })
        });
        (e, w, data)
    }

    #[test]
    fn cell_index_is_row_major_inside_the_grid() {
        let (mut e, w, _) = grid_engine();
        let index =
            |e: &mut Engine, x: i32, y: i32| e.call(0x0058_4530, &args![w, FILE, x, y]).i32();
        assert_eq!(index(&mut e, -2, -1), 0);
        assert_eq!(index(&mut e, 1, -1), 3);
        assert_eq!(index(&mut e, -2, 0), 4);
        assert_eq!(index(&mut e, 0, 0), 6);
        assert_eq!(index(&mut e, 1, 2), 15);
        // Outside the grid on each side.
        assert_eq!(index(&mut e, 2, 0), -1);
        assert_eq!(index(&mut e, -3, 0), -1);
        assert_eq!(index(&mut e, 0, 3), -1);
        assert_eq!(index(&mut e, 0, -2), -1);
        // A file without an OFFSET_DATA for the world space.
        assert_eq!(e.call(0x0058_4530, &args![w, 0x9999u32, 0, 0]).i32(), -1);
    }

    #[test]
    fn adjusting_the_offsets_moves_the_cells_that_have_one() {
        let (mut e, w, data) = grid_engine();
        e.call(0x0058_57b0, &args![w, FILE, 0x40u32]);
        let table = e.get(data, OffsetData::pCellFileOffsets).addr();
        assert_eq!(e.mem.u32(table + 6 * 4), 0x540);
        assert_eq!(e.mem.u32(table + 15 * 4), 0x940);
        // Empty entries stay empty.
        assert_eq!(e.mem.u32(table), 0);
        assert_eq!(e.mem.u32(table + 4 * 4), 0);
        // A delta of 0 and a file without data change nothing.
        e.call(0x0058_57b0, &args![w, FILE, 0u32]);
        e.call(0x0058_57b0, &args![w, 0x9999u32, 0x40u32]);
        assert_eq!(e.mem.u32(table + 6 * 4), 0x540);
        // So does a data block without a table.
        e.set(data, OffsetData::pCellFileOffsets, Ptr::NULL);
        e.call(0x0058_57b0, &args![w, FILE, 0x40u32]);
        assert_eq!(e.mem.u32(table + 6 * 4), 0x540);
    }

    /// The reader positioned by `SetOffset`, with a record at each offset.
    #[derive(Default)]
    struct Positioned {
        offset: u32,
        /// (offset, record type, form ID, editor ID)
        records: Vec<(u32, u32, u32, &'static str)>,
        set_offsets: Vec<u32>,
    }

    fn install_positioned(
        e: &mut Engine,
        records: Vec<(u32, u32, u32, &'static str)>,
    ) -> Rc<RefCell<Positioned>> {
        let state = Rc::new(RefCell::new(Positioned {
            records,
            ..Positioned::default()
        }));
        let s = state.clone();
        e.register_double(FILE_SET_OFFSET, move |_, a| {
            let mut s = s.borrow_mut();
            s.offset = a[1];
            s.set_offsets.push(a[1]);
            ret(1)
        });
        let s = state.clone();
        e.register_double(FILE_RECORD_TYPE, move |_, _| {
            let s = s.borrow();
            ret(s
                .records
                .iter()
                .find(|r| r.0 == s.offset)
                .map_or(0, |r| r.1))
        });
        let s = state.clone();
        e.register_double(FILE_CURRENT_FORM_ID, move |_, _| {
            let s = s.borrow();
            ret(s
                .records
                .iter()
                .find(|r| r.0 == s.offset)
                .map_or(0, |r| r.2))
        });
        let s = state.clone();
        e.register_double(FILE_NEXT_CHUNK, move |_, _| {
            let s = s.borrow();
            ret(
                if s.records.iter().any(|r| r.0 == s.offset && !r.3.is_empty()) {
                    CHUNK_EDITOR_ID
                } else {
                    CHUNK_DATA
                },
            )
        });
        let s = state.clone();
        e.register_double(FILE_CHUNK_SIZE, move |_, _| {
            let s = s.borrow();
            ret(s
                .records
                .iter()
                .find(|r| r.0 == s.offset)
                .map_or(0, |r| r.3.len() as u32 + 1))
        });
        let s = state.clone();
        e.register_double(FILE_READ_CHUNK, move |e, a| {
            let s = s.borrow();
            if let Some(r) = s.records.iter().find(|r| r.0 == s.offset) {
                e.mem.write(a[1], r.3.as_bytes());
            }
            Ret::default()
        });
        state
    }

    #[test]
    fn find_in_file_fast_checks_the_world_record_at_the_stored_position() {
        let (mut e, w, data) = grid_engine();
        e.mem.set_u32(w.addr() + 0xC, 0x3c);
        stub(&mut e, FILE_IS_MASTER, 1);
        stub(&mut e, FILE_CURRENT_FORM_ID, 0x3c);
        let state = install_positioned(&mut e, vec![(BASE_OFFSET, 0x41, 0x3c, "")]);
        // The data block's position is the file offset field.
        assert!(e.call(0x0058_58b0, &args![w, FILE]).bool());
        assert_eq!(state.borrow().set_offsets, vec![BASE_OFFSET]);
        // A different form ID in the record.
        let (mut e2, w2, _) = grid_engine();
        e2.mem.set_u32(w2.addr() + 0xC, 0x3d);
        stub(&mut e2, FILE_IS_MASTER, 1);
        install_positioned(&mut e2, vec![(BASE_OFFSET, 0x41, 0x3c, "")]);
        assert!(!e2.call(0x0058_58b0, &args![w2, FILE]).bool());
        // A record of another type.
        let (mut e3, w3, _) = grid_engine();
        e3.mem.set_u32(w3.addr() + 0xC, 0x3c);
        stub(&mut e3, FILE_IS_MASTER, 1);
        install_positioned(&mut e3, vec![(BASE_OFFSET, 0x39, 0x3c, "")]);
        assert!(!e3.call(0x0058_58b0, &args![w3, FILE]).bool());
        // Not a master file, no file, no stored position.
        stub(&mut e, FILE_IS_MASTER, 0);
        assert!(!e.call(0x0058_58b0, &args![w, FILE]).bool());
        stub(&mut e, FILE_IS_MASTER, 1);
        assert!(!e.call(0x0058_58b0, &args![w, 0u32]).bool());
        e.set(data, OffsetData::iFileOffset, 0);
        assert!(!e.call(0x0058_58b0, &args![w, FILE]).bool());
    }

    fn lower(text: &[u8]) -> Vec<u8> {
        text.to_ascii_lowercase()
    }

    #[test]
    fn cell_by_editor_id_searches_each_file_of_the_world_space() {
        let (mut e, w, _) = grid_engine();
        // Two files: the first has no table, the second is `FILE`.
        const NO_TABLE: u32 = 0x5555;
        stub(&mut e, FORM_FILE_COUNT, 2);
        e.register(FORM_GET_FILE, |_, a| {
            ret(if a[1] == 0 { NO_TABLE } else { FILE })
        });
        e.register(FILE_THREAD_SAFE, |_, a| ret(a[0]));
        let state = install_positioned(
            &mut e,
            vec![
                (BASE_OFFSET + 0x500, RECORD_TYPE_CELL, 1, "OtherCell"),
                (BASE_OFFSET + 0x900, RECORD_TYPE_CELL, 2, "GoodspringsCell"),
            ],
        );
        e.register(EDITOR_ID_COMPARE, |e, a| {
            let (x, y) = (e.mem.cstr(a[0]), e.mem.cstr(a[1]));
            ret((lower(&x) != lower(&y)) as u32)
        });
        let out_x = e.mem.alloc(4);
        let out_y = e.mem.alloc(4);
        let name = e.mem.alloc(32);
        e.mem.set_cstr(name, b"GOODSPRINGSCELL");
        let found = e.call(0x0058_5940, &args![w, name, out_x, out_y]).bool();
        assert!(found);
        assert_eq!(e.mem.i32(out_x), 1);
        assert_eq!(e.mem.i32(out_y), 2);
        // Cells are tried in x-major order: (0, 0) first, then (1, 2).
        assert_eq!(
            state.borrow().set_offsets,
            vec![BASE_OFFSET + 0x500, BASE_OFFSET + 0x900]
        );

        // Not found: the outputs stay as they were.
        e.mem.set_i32(out_x, -77);
        e.mem.set_i32(out_y, -78);
        e.mem.set_cstr(name, b"NoSuchCell");
        let found = e.call(0x0058_5940, &args![w, name, out_x, out_y]).bool();
        assert!(!found);
        assert_eq!(e.mem.i32(out_x), -77);
        assert_eq!(e.mem.i32(out_y), -78);
    }

    #[test]
    fn cell_by_editor_id_ignores_records_that_are_not_cells_or_have_no_editor_id() {
        let (mut e, w, _) = grid_engine();
        stub(&mut e, FORM_FILE_COUNT, 1);
        e.register(FORM_GET_FILE, |_, _| ret(FILE));
        e.register(FILE_THREAD_SAFE, |_, a| ret(a[0]));
        install_positioned(
            &mut e,
            vec![
                // Not a cell record.
                (BASE_OFFSET + 0x500, 0x41, 1, "Match"),
                // A cell whose first chunk is not the editor ID.
                (BASE_OFFSET + 0x900, RECORD_TYPE_CELL, 2, ""),
            ],
        );
        e.register(EDITOR_ID_COMPARE, |_, _| ret(0));
        let out = e.mem.alloc(8);
        let name = e.mem.alloc(8);
        e.mem.set_cstr(name, b"Match");
        assert!(!e.call(0x0058_5940, &args![w, name, out, out + 4]).bool());
    }

    // ---- scanning a plugin for an exterior cell --------------------------------

    #[derive(Clone, Copy, Debug)]
    enum Rec {
        Group { kind: u32, label: u32 },
        Cell { x: i32, y: i32, interior: bool },
        Skipped,
        Other,
    }

    #[derive(Default)]
    struct Stream {
        records: Vec<Option<Rec>>,
        position: usize,
        header: u32,
        chunk: usize,
        operations: Vec<&'static str>,
        formed: bool,
    }

    const BLOCK_KEY: u32 = 0xb10c;
    const SUB_BLOCK_KEY: u32 = 0x5b;

    fn write_header(e: &mut Engine, s: &Stream) {
        let h = s.header;
        let (tag_word, at8, at_c) = match s.records.get(s.position).copied().flatten() {
            Some(Rec::Group { kind, label }) => (e.global::<u32>(GROUP_TAG), label, kind),
            Some(Rec::Cell { interior, .. }) => (
                e.global::<u32>(CELL_TAG),
                if interior { 0x400 } else { 0 },
                9,
            ),
            Some(Rec::Skipped) => (e.global::<u32>(SKIPPED_RECORD_TAG_01187338), 0, 0),
            Some(Rec::Other) | None => (0x1234_5678, 0, 0),
        };
        e.mem.set_u32(h, tag_word);
        e.mem.set_u32(h + 8, at8);
        e.mem.set_u32(h + 0xC, at_c);
    }

    /// Installs a reader over `records` (index 0 stands for the world space's
    /// own record, which `FindForm` positions on).
    fn install_stream(e: &mut Engine, records: Vec<Rec>) -> Rc<RefCell<Stream>> {
        let header = e.mem.alloc(0x20);
        let stream = Rc::new(RefCell::new(Stream {
            records: std::iter::once(Some(Rec::Other))
                .chain(records.into_iter().map(Some))
                .collect(),
            header,
            ..Stream::default()
        }));
        noop(e, CELL_COORDS_CONSTRUCT);
        stub(e, FILE_NEEDS_SWAP, 0);
        stub(e, CALC_BLOCK_KEY, BLOCK_KEY);
        stub(e, CALC_SUB_BLOCK_KEY, SUB_BLOCK_KEY);
        stub(e, FILE_CURRENT_RECORD, header);
        let s = stream.clone();
        e.register_double(FILE_FIND_FORM, move |e, _| {
            let mut s = s.borrow_mut();
            s.position = 0;
            s.formed = true;
            write_header(e, &s);
            ret(1)
        });
        for (addr, name) in [(FILE_NEXT_FORM, "next"), (FILE_LEAVE_GROUP, "leave")] {
            let s = stream.clone();
            e.register_double(addr, move |e, _| {
                let mut s = s.borrow_mut();
                s.position += 1;
                s.chunk = 0;
                s.operations.push(name);
                write_header(e, &s);
                ret(1)
            });
        }
        let s = stream.clone();
        e.register_double(FILE_REWIND_RECORD, move |_, _| {
            s.borrow_mut().operations.push("rewind");
            Ret::default()
        });
        let s = stream.clone();
        e.register_double(FILE_NEXT_CHUNK, move |_, _| {
            let s = s.borrow();
            ret(match s.records[s.position] {
                Some(Rec::Cell { .. }) if s.chunk == 0 => CHUNK_CELL_GRID,
                Some(Rec::Cell { .. }) if s.chunk == 1 => CHUNK_DATA,
                _ => 0,
            })
        });
        let s = stream.clone();
        e.register_double(FILE_ADVANCE_CHUNK, move |_, _| {
            let mut s = s.borrow_mut();
            s.chunk += 1;
            ret((s.chunk < 2) as u32)
        });
        let s = stream.clone();
        e.register_double(FILE_READ_CHUNK, move |e, a| {
            let s = s.borrow();
            if let Some(Rec::Cell { x, y, .. }) = s.records[s.position] {
                e.mem.set_i32(a[1], x);
                e.mem.set_i32(a[1] + 4, y);
                e.mem.set_u32(a[1] + 8, 0);
            }
            Ret::default()
        });
        stream
    }

    /// `FindCellInFile` for a file that has no offset table (the slow path).
    fn scan(records: Vec<Rec>, x: i32, y: i32) -> (bool, Vec<&'static str>) {
        let mut e = engine();
        let w = world(&mut e);
        // A plugin: not a master file.
        stub(&mut e, FILE_IS_MASTER, 0);
        let stream = install_stream(&mut e, records);
        let found = e.call(0x005854f0, &args![w, FILE, x, y]).bool();
        let operations = stream.borrow().operations.clone();
        (found, operations)
    }

    #[test]
    fn scan_finds_the_cell_through_the_groups_that_hold_it() {
        let (found, operations) = scan(
            vec![
                Rec::Group { kind: 1, label: 1 },
                Rec::Group {
                    kind: 4,
                    label: BLOCK_KEY,
                },
                Rec::Group {
                    kind: 5,
                    label: SUB_BLOCK_KEY,
                },
                Rec::Cell {
                    x: 3,
                    y: 4,
                    interior: false,
                },
            ],
            3,
            4,
        );
        assert!(found);
        // The first NextForm steps past the world record; then each group is
        // entered (NextForm) and the cell record ends the scan with a rewind.
        assert_eq!(operations, vec!["next", "next", "next", "next", "rewind"]);
    }

    #[test]
    fn scan_leaves_the_groups_whose_label_is_not_the_cells_block() {
        let (found, operations) = scan(
            vec![
                Rec::Group { kind: 1, label: 1 },
                // An exterior block of another cell.
                Rec::Group {
                    kind: 4,
                    label: 0xdead,
                },
                // A sub-block of another cell.
                Rec::Group {
                    kind: 5,
                    label: 0xbeef,
                },
                Rec::Group {
                    kind: 4,
                    label: SUB_BLOCK_KEY,
                },
                Rec::Cell {
                    x: 3,
                    y: 4,
                    interior: false,
                },
            ],
            3,
            4,
        );
        assert!(found);
        // Block key or sub-block key both match a type 4 group's label.
        assert_eq!(
            operations,
            vec!["next", "next", "leave", "leave", "next", "rewind"]
        );
    }

    #[test]
    fn scan_passes_other_cells_and_the_skipped_records() {
        let (found, operations) = scan(
            vec![
                Rec::Cell {
                    x: 0,
                    y: 0,
                    interior: false,
                },
                Rec::Skipped,
                Rec::Cell {
                    x: 3,
                    y: 4,
                    interior: true,
                },
                Rec::Cell {
                    x: 3,
                    y: 4,
                    interior: false,
                },
            ],
            3,
            4,
        );
        assert!(found);
        // An interior cell never matches, whatever its grid chunk says.
        assert_eq!(operations, vec!["next", "next", "next", "next", "rewind"]);
    }

    #[test]
    fn scan_stops_at_groups_that_end_the_world_and_at_unknown_records() {
        // Group types 2, 3 and 7 and any record that is not a cell or group.
        for stop in [
            Rec::Group { kind: 2, label: 0 },
            Rec::Group { kind: 3, label: 0 },
            Rec::Group { kind: 7, label: 0 },
            Rec::Other,
        ] {
            let (found, operations) = scan(
                vec![
                    stop,
                    Rec::Cell {
                        x: 3,
                        y: 4,
                        interior: false,
                    },
                ],
                3,
                4,
            );
            assert!(!found, "{stop:?}");
            assert_eq!(operations, vec!["next"], "{stop:?}");
        }
        // A group type 0 (and anything above 9) also stops.
        let (found, _) = scan(
            vec![
                Rec::Group { kind: 0, label: 0 },
                Rec::Cell {
                    x: 3,
                    y: 4,
                    interior: false,
                },
            ],
            3,
            4,
        );
        assert!(!found);
    }

    #[test]
    fn scan_leaves_cell_children_groups_by_the_group_exit() {
        // Cell children groups (6, 8, 9) are not entered: the scan leaves them.
        let (found, operations) = scan(
            vec![
                Rec::Group { kind: 6, label: 0 },
                Rec::Group { kind: 8, label: 0 },
                Rec::Group { kind: 9, label: 0 },
                Rec::Cell {
                    x: 1,
                    y: 1,
                    interior: false,
                },
            ],
            1,
            1,
        );
        assert!(found);
        assert_eq!(
            operations,
            vec!["next", "leave", "leave", "leave", "rewind"]
        );
    }

    #[test]
    fn scan_does_nothing_when_the_file_has_no_world_record() {
        let mut e = engine();
        let w = world(&mut e);
        stub(&mut e, FILE_IS_MASTER, 0);
        let stream = install_stream(
            &mut e,
            vec![Rec::Cell {
                x: 1,
                y: 1,
                interior: false,
            }],
        );
        stub(&mut e, FILE_FIND_FORM, 0);
        assert!(!e.call(0x005854f0, &args![w, FILE, 1i32, 1i32]).bool());
        assert!(stream.borrow().operations.is_empty());
        // A null file gives the same.
        assert!(!e.call(0x005854f0, &args![w, 0u32, 1i32, 1i32]).bool());
    }

    #[test]
    fn a_master_file_with_a_table_is_found_by_index() {
        let (mut e, w, _) = grid_engine();
        stub(&mut e, FILE_IS_MASTER, 1);
        let state = install_positioned(&mut e, vec![]);
        logged(&mut e);
        assert!(e.call(0x005854f0, &args![w, FILE, 0i32, 0i32]).bool());
        assert_eq!(state.borrow().set_offsets, vec![BASE_OFFSET + 0x500]);
        // A cell with no entry, and one outside the grid.
        assert!(!e.call(0x005854f0, &args![w, FILE, 1i32, 1i32]).bool());
        assert!(!e.call(0x005854f0, &args![w, FILE, 9i32, 9i32]).bool());
        assert_eq!(state.borrow().set_offsets.len(), 1);
        // No table scan was started.
        assert!(calls_to(&e, FILE_FIND_FORM).is_empty());
    }

    #[test]
    fn a_master_file_whose_table_is_missing_is_scanned() {
        let (mut e, w, data) = grid_engine();
        e.set(data, OffsetData::pCellFileOffsets, Ptr::NULL);
        stub(&mut e, FILE_IS_MASTER, 1);
        let stream = install_stream(
            &mut e,
            vec![Rec::Cell {
                x: 5,
                y: 6,
                interior: false,
            }],
        );
        assert!(e.call(0x005854f0, &args![w, FILE, 5i32, 6i32]).bool());
        assert_eq!(stream.borrow().operations, vec!["next", "rewind"]);
    }

    // ---- loading and unloading exterior cells ------------------------------------

    const SAVE_LOAD: u32 = 0x5a5a;
    const CELL_INIT_ITEM: u32 = 0x0054_f000;
    const CELL_SET_FORM_ID: u32 = 0x0054_f100;
    const WORLD_NAME: u32 = 0x0058_5000;

    /// An engine for `LoadCell`: the world space `0x3c` owns one file, `FILE`,
    /// whose offset table has the cell (0, 0) (a 1 x 1 grid); every cell
    /// function is a double that returns success.
    fn load_cell_engine() -> (Engine, Ptr<TESWorldSpace>) {
        let (mut e, w, data) = grid_engine();
        e.set(data, OffsetData::OffsetMinCoords_x, 0.0);
        e.set(data, OffsetData::OffsetMinCoords_y, 0.0);
        e.set(data, OffsetData::OffsetMaxCoords_x, 0.0);
        e.set(data, OffsetData::OffsetMaxCoords_y, 0.0);
        let table = e.get(data, OffsetData::pCellFileOffsets).addr();
        e.mem.set_u32(table, 0x500);
        e.mem.set_u32(w.addr() + 0xC, 0x3c);
        give_vtable(&mut e, w.addr(), &[(SLOT_EDITOR_ID, WORLD_NAME)]);
        stub(&mut e, WORLD_NAME, 0x7777);
        set_word(&mut e, SAVE_LOAD_GAME, SAVE_LOAD);
        set_word(&mut e, CURRENT_WORLD_SPACE, 0);
        stub(&mut e, FORM_FILE_COUNT, 1);
        e.register(FORM_GET_FILE, |_, _| ret(FILE));
        e.register(FILE_THREAD_SAFE, |_, a| ret(a[0]));
        stub(&mut e, FILE_IS_MASTER, 1);
        stub(&mut e, FILE_SET_OFFSET, 1);
        // The record header the reader exposes: flags at +8.
        let header = e.mem.alloc(0x20);
        stub(&mut e, FILE_CURRENT_RECORD, header);
        for addr in [
            MEMORY_CONTEXT_ENTER,
            MEMORY_CONTEXT_LEAVE,
            CELL_SET_INTERIOR,
            CELL_CREATE_DATA,
            CELL_SET_DATA_COORD,
            ADD_CELL,
            CELL_SET_HAS_TEMP_DATA,
            SAVE_LOAD_SET_STATE,
            ASSIGN_PERSISTENT_REFS,
            LOG,
            CELL_INIT_ITEM,
            CELL_SET_FORM_ID,
        ] {
            noop(&mut e, addr);
        }
        stub(&mut e, GET_CELL_FROM_CELL_COORD, 0);
        stub(&mut e, CELL_HAS_TEMP_DATA, 0);
        stub(&mut e, SAVE_LOAD_CREATED_CELL_ID, 0);
        stub(&mut e, DATA_HANDLER_LOAD_FORM, 1);
        stub(&mut e, CELL_LOAD_TEMP_DATA, 1);
        stub(&mut e, SAVE_LOAD_GET_STATE, 0);
        e.register(CELL_CONSTRUCT, |e, a| {
            give_vtable(
                e,
                a[0],
                &[(0x88, CELL_INIT_ITEM), (0x128, CELL_SET_FORM_ID)],
            );
            ret(a[0])
        });
        (e, w)
    }

    fn addresses(e: &Engine) -> Vec<u32> {
        e.call_log.as_ref().unwrap().iter().map(|c| c.0).collect()
    }

    #[test]
    fn load_cell_creates_and_loads_a_missing_cell() {
        let (mut e, w) = load_cell_engine();
        logged(&mut e);
        let cell = e.call(0x0058_5b30, &args![w, 0i32, 0i32]).ptr::<()>();
        assert!(!cell.is_null());
        assert_eq!(
            e.mem.block_size(cell.addr()).map(|s| s >= CELL_SIZE),
            Some(true)
        );
        assert_eq!(e.global::<u32>(CURRENT_WORLD_SPACE), w.addr());
        let c = cell.addr();
        let calls = e.call_log.clone().unwrap();
        let find = |addr: u32| calls.iter().find(|x| x.0 == addr).cloned();
        let enter = find(MEMORY_CONTEXT_ENTER).unwrap().1;
        assert_eq!(&enter[1..], &[0x1a, 1, WORLD_SPACE_SOURCE, 0x796]);
        assert_eq!(find(CELL_SET_INTERIOR).unwrap().1, vec![c, 0]);
        assert_eq!(find(CELL_SET_DATA_COORD).unwrap().1, vec![c, 0, 0]);
        assert_eq!(find(ADD_CELL).unwrap().1, vec![w.addr(), c]);
        assert_eq!(
            find(SAVE_LOAD_CREATED_CELL_ID).unwrap().1,
            vec![SAVE_LOAD, 0x3c, 0, 0]
        );
        assert_eq!(find(DATA_HANDLER_LOAD_FORM).unwrap().1, vec![c, FILE]);
        assert_eq!(find(CELL_LOAD_TEMP_DATA).unwrap().1, vec![c, FILE]);
        assert_eq!(find(ASSIGN_PERSISTENT_REFS).unwrap().1, vec![w.addr(), c]);
        // Order of the work, and the state switch around the cell's InitItem.
        let order: Vec<u32> = addresses(&e)
            .into_iter()
            .filter(|a| {
                [
                    CELL_CONSTRUCT,
                    ADD_CELL,
                    DATA_HANDLER_LOAD_FORM,
                    CELL_LOAD_TEMP_DATA,
                    CELL_SET_HAS_TEMP_DATA,
                    SAVE_LOAD_SET_STATE,
                    CELL_INIT_ITEM,
                    ASSIGN_PERSISTENT_REFS,
                    MEMORY_CONTEXT_LEAVE,
                ]
                .contains(a)
            })
            .collect();
        assert_eq!(
            order,
            vec![
                CELL_CONSTRUCT,
                ADD_CELL,
                DATA_HANDLER_LOAD_FORM,
                CELL_LOAD_TEMP_DATA,
                CELL_SET_HAS_TEMP_DATA,
                SAVE_LOAD_SET_STATE,
                CELL_INIT_ITEM,
                SAVE_LOAD_SET_STATE,
                ASSIGN_PERSISTENT_REFS,
                MEMORY_CONTEXT_LEAVE,
            ]
        );
        // The switch is turned on for the InitItem (the state was off), then
        // put back.
        let states = calls_to(&e, SAVE_LOAD_SET_STATE);
        assert_eq!(states, vec![vec![SAVE_LOAD, 1], vec![SAVE_LOAD, 0]]);
        assert!(calls_to(&e, LOG).is_empty());
    }

    #[test]
    fn load_cell_gives_a_created_cell_the_form_id_the_save_game_made_for_it() {
        let (mut e, w) = load_cell_engine();
        stub(&mut e, SAVE_LOAD_CREATED_CELL_ID, 0x00ff_1234);
        logged(&mut e);
        let cell = e.call(0x0058_5b30, &args![w, 0i32, 0i32]).u32();
        assert_eq!(
            calls_to(&e, CELL_SET_FORM_ID),
            vec![vec![cell, 0x00ff_1234, 1]]
        );
    }

    #[test]
    fn load_cell_restores_the_state_switch_it_found_on() {
        let (mut e, w) = load_cell_engine();
        stub(&mut e, SAVE_LOAD_GET_STATE, 1);
        logged(&mut e);
        e.call(0x0058_5b30, &args![w, 0i32, 0i32]);
        assert_eq!(
            calls_to(&e, SAVE_LOAD_SET_STATE),
            vec![vec![SAVE_LOAD, 0], vec![SAVE_LOAD, 1]]
        );
    }

    /// An existing cell with the vtable `LoadCell` needs.
    fn existing_cell(e: &mut Engine) -> u32 {
        let cell = e.mem.alloc(CELL_SIZE);
        give_vtable(
            e,
            cell,
            &[(0x88, CELL_INIT_ITEM), (0x128, CELL_SET_FORM_ID)],
        );
        stub(e, GET_CELL_FROM_CELL_COORD, cell);
        cell
    }

    #[test]
    fn load_cell_adds_the_files_record_to_an_existing_cell_unless_flagged() {
        let (mut e, w) = load_cell_engine();
        let cell = existing_cell(&mut e);
        logged(&mut e);
        // Record flag 0x4000 clear: the form is loaded again.
        assert_eq!(e.call(0x0058_5b30, &args![w, 0i32, 0i32]).u32(), cell);
        assert_eq!(calls_to(&e, DATA_HANDLER_LOAD_FORM), vec![vec![cell, FILE]]);
        assert!(calls_to(&e, ADD_CELL).is_empty());
        assert!(calls_to(&e, CELL_CONSTRUCT).is_empty());
        assert_eq!(calls_to(&e, CELL_LOAD_TEMP_DATA), vec![vec![cell, FILE]]);
        // Flag 0x4000 set in the record header: not loaded again.
        let record = e.call(FILE_CURRENT_RECORD, &args![FILE]).u32();
        e.mem.set_u32(record + 8, 0x4000);
        e.call_log = Some(vec![]);
        e.call(0x0058_5b30, &args![w, 0i32, 0i32]);
        assert!(calls_to(&e, DATA_HANDLER_LOAD_FORM).is_empty());
        assert_eq!(calls_to(&e, CELL_LOAD_TEMP_DATA).len(), 1);
    }

    #[test]
    fn load_cell_with_temporary_data_already_loaded_does_nothing_more() {
        let (mut e, w) = load_cell_engine();
        let cell = existing_cell(&mut e);
        stub(&mut e, CELL_HAS_TEMP_DATA, 1);
        logged(&mut e);
        assert_eq!(e.call(0x0058_5b30, &args![w, 0i32, 0i32]).u32(), cell);
        assert_eq!(
            addresses(&e),
            vec![
                0x0058_5b30,
                MEMORY_CONTEXT_ENTER,
                GET_CELL_FROM_CELL_COORD,
                CELL_HAS_TEMP_DATA,
                MEMORY_CONTEXT_LEAVE
            ]
        );
    }

    #[test]
    fn load_cell_logs_a_failure_to_load_the_temporary_data() {
        let (mut e, w) = load_cell_engine();
        let cell = existing_cell(&mut e);
        stub(&mut e, CELL_LOAD_TEMP_DATA, 0);
        logged(&mut e);
        // The cell is still returned and initialized.
        assert_eq!(e.call(0x0058_5b30, &args![w, 0i32, 0i32]).u32(), cell);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![MSG_CELL_LOAD_FAILED, 0, 0, 0x7777, 0x3c]]
        );
        assert_eq!(calls_to(&e, ASSIGN_PERSISTENT_REFS).len(), 1);
    }

    #[test]
    fn load_cell_logs_when_the_new_cells_form_does_not_load() {
        let (mut e, w) = load_cell_engine();
        stub(&mut e, DATA_HANDLER_LOAD_FORM, 0);
        logged(&mut e);
        e.call(0x0058_5b30, &args![w, 0i32, 0i32]);
        assert_eq!(calls_to(&e, LOG).len(), 1);
    }

    #[test]
    fn load_cell_returns_null_when_no_file_has_the_cell() {
        let (mut e, w) = load_cell_engine();
        // Two files, neither with a thread-safe version.
        stub(&mut e, FORM_FILE_COUNT, 2);
        stub(&mut e, FILE_THREAD_SAFE, 0);
        logged(&mut e);
        assert_eq!(e.call(0x0058_5b30, &args![w, 1i32, 1i32]).u32(), 0);
        assert!(calls_to(&e, CELL_CONSTRUCT).is_empty());
        assert!(calls_to(&e, LOG).is_empty());
        // The file is there but the cell (1, 1) is outside its 1 x 1 grid.
        e.register(FILE_THREAD_SAFE, |_, a| ret(a[0]));
        stub(&mut e, FORM_FILE_COUNT, 1);
        assert_eq!(e.call(0x0058_5b30, &args![w, 1i32, 1i32]).u32(), 0);
        assert!(calls_to(&e, CELL_CONSTRUCT).is_empty());
    }

    fn unload_engine() -> (Engine, Ptr<TESWorldSpace>, u32) {
        let mut e = engine();
        let w = world(&mut e);
        let cell = e.mem.alloc(CELL_SIZE);
        e.mem.set_u32(cell + 0xC, 0xc311);
        const CELL_DESTRUCT: u32 = 0x0054_f200;
        give_vtable(&mut e, cell, &[(0x10, CELL_DESTRUCT)]);
        noop(&mut e, CELL_DESTRUCT);
        let owner = w.addr();
        e.register_double(CELL_GET_WORLD_SPACE, move |_, _| ret(owner));
        e.register(FORM_GET_FILE, |_, _| ret(0));
        for addr in [
            CELL_CLEANUP,
            SAVE_LOAD_NOTE_CELL,
            GAME_LOADER_UNLOAD_FORM,
            RELEASE_CELL,
        ] {
            noop(&mut e, addr);
        }
        stub(&mut e, LOADER_STATE_QUERY, 0);
        set_word(&mut e, SAVE_LOAD_GAME, SAVE_LOAD);
        set_word(&mut e, LOADER_STATE, 0x1111);
        set_word(&mut e, GAME_LOADER, 0x2222);
        (e, w, cell)
    }

    #[test]
    fn unload_cell_of_a_cell_with_no_file_runs_the_whole_teardown() {
        let (mut e, w, cell) = unload_engine();
        logged(&mut e);
        e.call(0x0058_5e00, &args![w, cell]);
        let order: Vec<u32> = addresses(&e).into_iter().skip(1).collect();
        assert_eq!(
            order,
            vec![
                CELL_GET_WORLD_SPACE,
                FORM_GET_FILE,
                CELL_CLEANUP,
                SAVE_LOAD_NOTE_CELL,
                WORD_AT_0C,
                LOADER_STATE_QUERY,
                GAME_LOADER_UNLOAD_FORM,
                RELEASE_CELL,
                0x0054_f200,
            ]
        );
        assert_eq!(
            calls_to(&e, SAVE_LOAD_NOTE_CELL),
            vec![vec![SAVE_LOAD, cell]]
        );
        assert_eq!(calls_to(&e, LOADER_STATE_QUERY), vec![vec![0x1111, 0xc311]]);
        assert_eq!(
            calls_to(&e, GAME_LOADER_UNLOAD_FORM),
            vec![vec![0x2222, cell, 0]]
        );
        assert_eq!(calls_to(&e, RELEASE_CELL), vec![vec![w.addr(), cell]]);
        // The cell is deleted through its destructor with the delete flag.
        assert_eq!(calls_to(&e, 0x0054_f200), vec![vec![cell, 1]]);
    }

    #[test]
    fn unload_cell_keeps_the_form_loaded_when_the_loader_still_has_it() {
        let (mut e, w, cell) = unload_engine();
        stub(&mut e, LOADER_STATE_QUERY, 1);
        logged(&mut e);
        e.call(0x0058_5e00, &args![w, cell]);
        assert!(calls_to(&e, GAME_LOADER_UNLOAD_FORM).is_empty());
        assert_eq!(calls_to(&e, RELEASE_CELL).len(), 1);
    }

    #[test]
    fn unload_cell_from_a_master_file_is_torn_down_but_from_a_plugin_only_cleaned() {
        let (mut e, w, cell) = unload_engine();
        e.register(FORM_GET_FILE, |_, _| ret(FILE));
        stub(&mut e, FILE_IS_MASTER, 1);
        logged(&mut e);
        e.call(0x0058_5e00, &args![w, cell]);
        assert_eq!(calls_to(&e, RELEASE_CELL).len(), 1);
        stub(&mut e, FILE_IS_MASTER, 0);
        e.call_log = Some(vec![]);
        e.call(0x0058_5e00, &args![w, cell]);
        assert_eq!(calls_to(&e, CELL_CLEANUP), vec![vec![cell]]);
        assert!(calls_to(&e, RELEASE_CELL).is_empty());
        assert!(calls_to(&e, 0x0054_f200).is_empty());
    }

    #[test]
    fn unload_cell_ignores_null_cells_and_cells_of_other_world_spaces() {
        let (mut e, w, cell) = unload_engine();
        let other = world(&mut e);
        logged(&mut e);
        e.call(0x0058_5e00, &args![w, 0u32]);
        e.call(0x0058_5e00, &args![other, cell]);
        assert!(calls_to(&e, CELL_CLEANUP).is_empty());
        assert!(calls_to(&e, RELEASE_CELL).is_empty());
    }

    // ---- land data ---------------------------------------------------------------

    #[test]
    fn find_land_data_searches_the_files_from_the_last_and_reports_the_first_hit() {
        let (mut e, w, data) = grid_engine();
        e.set(data, OffsetData::OffsetMinCoords_x, 0.0);
        e.set(data, OffsetData::OffsetMinCoords_y, 0.0);
        e.set(data, OffsetData::OffsetMaxCoords_x, 0.0);
        e.set(data, OffsetData::OffsetMaxCoords_y, 0.0);
        let table = e.get(data, OffsetData::pCellFileOffsets).addr();
        e.mem.set_u32(table, 0x500);
        let at = data.addr();
        // Three files that all share the data block.
        e.register_double(OFFSET_DATA_OF_FILE, move |_, _| ret(at));
        install_parents(&mut e, vec![]);
        stub(&mut e, FORM_FILE_COUNT, 3);
        e.register(FORM_GET_FILE, |_, a| ret(0xf000 + a[1]));
        e.register(FILE_THREAD_SAFE, |_, a| ret(a[0]));
        stub(&mut e, FILE_IS_MASTER, 1);
        stub(&mut e, FILE_SET_OFFSET, 1);
        // Land data in files 0 and 1 only.
        e.register(CELL_FIND_LAND_DATA_IN_FILE, |_, a| {
            ret((a[0] != 0xf002) as u32)
        });
        stub(&mut e, FILE_CURRENT_OFFSET, 0x777);
        let out_file = e.mem.alloc(4);
        let out_offset = e.mem.alloc(4);
        e.mem.set_u32(out_file, 0xdead);
        logged(&mut e);
        let found = e
            .call(0x0058_5ee0, &args![w, 0i32, 0i32, out_file, out_offset])
            .bool();
        assert!(found);
        // File 2 is tried first, has no land data; file 1 is the hit.
        assert_eq!(e.mem.u32(out_file), 0xf001);
        assert_eq!(e.mem.u32(out_offset), 0x777);
        assert_eq!(
            calls_to(&e, CELL_FIND_LAND_DATA_IN_FILE),
            vec![vec![0xf002], vec![0xf001]]
        );
    }

    #[test]
    fn find_land_data_that_nothing_has_leaves_the_outputs_zeroed() {
        let (mut e, w, _) = grid_engine();
        install_parents(&mut e, vec![]);
        stub(&mut e, FORM_FILE_COUNT, 1);
        e.register(FORM_GET_FILE, |_, _| ret(FILE));
        e.register(FILE_THREAD_SAFE, |_, a| ret(a[0]));
        stub(&mut e, FILE_IS_MASTER, 1);
        let out = e.mem.alloc(8);
        e.mem.set_u32(out, 0xdead);
        e.mem.set_u32(out + 4, 0xbeef);
        // The cell (7, 7) is outside the grid.
        assert!(!e
            .call(0x0058_5ee0, &args![w, 7i32, 7i32, out, out + 4])
            .bool());
        assert_eq!(e.mem.u32(out), 0);
        assert_eq!(e.mem.u32(out + 4), 0);
    }

    #[test]
    fn find_land_data_of_a_world_that_uses_its_parents_cells_asks_the_parent() {
        let mut e = engine();
        let child = world(&mut e);
        let parent = world(&mut e);
        install_parents(&mut e, vec![(child.addr(), 0, parent.addr())]);
        // The parent has no files at all.
        e.register_double(FORM_FILE_COUNT, |_, a| ret((a[0] == 0) as u32));
        let out = e.mem.alloc(8);
        e.mem.set_u32(out, 0xdead);
        logged(&mut e);
        assert!(!e
            .call(0x0058_5ee0, &args![child, 1i32, 2i32, out, out + 4])
            .bool());
        assert_eq!(e.mem.u32(out), 0);
        // Only the parent's files were asked for.
        assert_eq!(calls_to(&e, FORM_FILE_COUNT), vec![vec![parent.addr()]]);
    }

    // ---- InitItem ------------------------------------------------------------------

    /// The doubles for walking a cell map: `GetFirstPos` gives 1 when there are
    /// cells and `GetNext` serves them in order (positions are 1-based
    /// indexes, keys are the indexes).
    fn install_cell_map(e: &mut Engine, cells: Vec<u32>) {
        let count = cells.len() as u32;
        e.register_double(MAP_FIRST_POS, move |_, _| ret((count > 0) as u32));
        e.register_double(MAP_GET_NEXT, move |e, a| {
            let position = e.mem.u32(a[1]);
            let index = position - 1;
            let next = if position < count { position + 1 } else { 0 };
            e.mem.set_u32(a[1], next);
            e.mem.set_u32(a[2], index);
            e.mem.set_u32(a[3], cells[index as usize]);
            Ret::default()
        });
    }

    /// An engine for `InitItem`: ids become load order ids (+0x01000000),
    /// forms are found from a table, and nothing is logged until asked.
    fn init_engine(known: Vec<(u32, u32)>) -> (Engine, Ptr<TESWorldSpace>) {
        let mut e = engine();
        let w = world(&mut e);
        stub(&mut e, FORM_FLAG_8, 0);
        stub(&mut e, FORM_GET_FILE, 0xf11e);
        e.register(FORM_ADD_COMPILE_INDEX, |e, a| {
            let id = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], id | 0x0100_0000);
            Ret::default()
        });
        e.register_double(FORM_BY_ID, move |_, a| {
            ret(known
                .iter()
                .find(|(id, _)| *id == a[0])
                .map_or(0, |(_, form)| *form))
        });
        for addr in [
            SET_CLIMATE,
            SET_IMAGE_SPACE,
            SET_WATER,
            SET_LOD_WATER,
            SET_MUSIC_TYPE,
            SET_PARENT_WORLD,
            IMPACT_SWAP_INIT_ITEM,
            FORM_SET_STATE,
            LOG,
        ] {
            noop(&mut e, addr);
        }
        stub(&mut e, EDITOR_ID_LENGTH, 0);
        e.register(GET_PERSISTENT_CELL, |e, a| ret(e.mem.u32(a[0] + 0x34)));
        e.register(TERRAIN_MANAGER_CONSTRUCT, |_, a| ret(a[0]));
        install_cell_map(&mut e, vec![]);
        let map: Ptr<crate::types::NiTPointerMap> = e.new_object();
        e.set(w, TESWorldSpace::pCellMap, map.cast());
        e.mem.set_u32(w.addr() + 0xC, 0x3c);
        give_vtable(&mut e, w.addr(), &[(SLOT_EDITOR_ID, 0x0058_5000)]);
        stub(&mut e, 0x0058_5000, 0x7777);
        (e, w)
    }

    #[test]
    fn init_item_resolves_each_form_reference_and_hands_it_to_its_setter() {
        let forms: Vec<(u32, u32)> = (1..=7)
            .map(|i| (0x0100_0000 | i, 0xf0f0_0000 + i))
            .collect();
        let (mut e, w) = init_engine(forms);
        // Climate, image space, water, lod water, music, parent, zone ids.
        e.set(w, TESWorldSpace::pClimate, Ptr::new(1));
        e.set(w, TESWorldSpace::pImageSpace, Ptr::new(2));
        e.set(w, TESWorldSpace::pEncounterZone, Ptr::new(3));
        e.set(w, TESWorldSpace::pWorldWater, Ptr::new(4));
        e.set(w, TESWorldSpace::pLODWater, Ptr::new(5));
        e.set(w, TESWorldSpace::pMusicType, Ptr::new(6));
        e.set(w, TESWorldSpace::pParentWorld, Ptr::new(7));
        e.set(w, TESWorldSpace::pImpactSwap, Ptr::new(0x5151));
        logged(&mut e);
        e.call(0x0058_45e0, &args![w]);
        let this = w.addr();
        assert_eq!(calls_to(&e, SET_CLIMATE), vec![vec![this, 0xf0f0_0001]]);
        assert_eq!(calls_to(&e, SET_IMAGE_SPACE), vec![vec![this, 0xf0f0_0002]]);
        assert_eq!(e.get(w, TESWorldSpace::pEncounterZone).addr(), 0xf0f0_0003);
        assert_eq!(calls_to(&e, SET_WATER), vec![vec![this, 0xf0f0_0004]]);
        assert_eq!(calls_to(&e, SET_LOD_WATER), vec![vec![this, 0xf0f0_0005]]);
        assert_eq!(calls_to(&e, SET_MUSIC_TYPE), vec![vec![this, 0xf0f0_0006]]);
        assert_eq!(
            calls_to(&e, SET_PARENT_WORLD),
            vec![vec![this, 0xf0f0_0007]]
        );
        assert_eq!(
            calls_to(&e, IMPACT_SWAP_INIT_ITEM),
            vec![vec![0x5151, this]]
        );
        assert_eq!(calls_to(&e, FORM_SET_STATE), vec![vec![this, 1]]);
        assert!(calls_to(&e, LOG).is_empty());
        // The terrain manager is created at the end.
        let manager = e.get(w, TESWorldSpace::pTerrainManager);
        assert!(!manager.is_null());
        assert_eq!(
            calls_to(&e, TERRAIN_MANAGER_CONSTRUCT),
            vec![vec![manager.addr(), this]]
        );
        // The form types: climate, image space, encounter zone, water, lod
        // water, music, world space (the cast targets, in this order).
        let targets: Vec<u32> = calls_to(&e, RT_DYNAMIC_CAST).iter().map(|c| c[3]).collect();
        assert_eq!(
            targets,
            vec![
                TYPE_TES_CLIMATE,
                TYPE_TES_IMAGE_SPACE,
                TYPE_BGS_ENCOUNTER_ZONE,
                TYPE_TES_WATER_FORM,
                TYPE_TES_WATER_FORM,
                TYPE_BGS_MUSIC_TYPE,
                TYPE_TES_WORLD_SPACE,
            ]
        );
        // `AddCompileIndex` got the form's last file.
        assert_eq!(calls_to(&e, FORM_ADD_COMPILE_INDEX).len(), 7);
        assert_eq!(calls_to(&e, FORM_ADD_COMPILE_INDEX)[0][1], 0xf11e);
    }

    #[test]
    fn init_item_logs_each_reference_it_cannot_find() {
        // (field offset, message with the editor ID, message with the form ID)
        let fields: [(u32, u32, u32); 7] = [
            (0x40, MSG_CLIMATE_NAMED, MSG_CLIMATE_ID),
            (0x44, MSG_IMAGE_SPACE_NAMED, MSG_IMAGE_SPACE_ID),
            (0xD0, MSG_ENCOUNTER_ZONE_NAMED, MSG_ENCOUNTER_ZONE_ID),
            (0x74, MSG_WATER_NAMED, MSG_WATER_ID),
            (0x78, MSG_LOD_WATER_NAMED, MSG_LOD_WATER_ID),
            (0x9C, MSG_MUSIC_NAMED, MSG_MUSIC_ID),
            (0x70, MSG_PARENT_NAMED, MSG_PARENT_ID),
        ];
        for (offset, named, plain) in fields {
            for has_editor_id in [true, false] {
                let (mut e, w) = init_engine(vec![]);
                e.mem.set_u32(w.addr() + offset, 0x42);
                stub(&mut e, EDITOR_ID_LENGTH, has_editor_id as u32);
                logged(&mut e);
                e.call(0x0058_45e0, &args![w]);
                let expected = if has_editor_id {
                    // The name comes from the form's slot +0x130.
                    vec![named, 0x0100_0042, 0x7777]
                } else {
                    vec![plain, 0x0100_0042, 0x3c]
                };
                assert_eq!(
                    calls_to(&e, LOG),
                    vec![expected],
                    "field {offset:#x} named {has_editor_id}"
                );
            }
        }
    }

    #[test]
    fn init_item_with_form_flag_8_set_only_initializes_the_cells() {
        let (mut e, w) = init_engine(vec![]);
        stub(&mut e, FORM_FLAG_8, 1);
        e.set(w, TESWorldSpace::pClimate, Ptr::new(1));
        e.set(w, TESWorldSpace::pImpactSwap, Ptr::new(0x5151));
        // Two cells in the map and a persistent cell.
        const INIT: u32 = 0x0054_f000;
        let cells: Vec<u32> = (0..3)
            .map(|_| {
                let cell = e.mem.alloc(0x20);
                give_vtable(&mut e, cell, &[(0x88, INIT)]);
                cell
            })
            .collect();
        noop(&mut e, INIT);
        install_cell_map(&mut e, vec![cells[0], 0, cells[1]]);
        e.set(w, TESWorldSpace::pPersistentCell, Ptr::new(cells[2]));
        logged(&mut e);
        e.call(0x0058_45e0, &args![w]);
        assert!(calls_to(&e, SET_CLIMATE).is_empty());
        assert!(calls_to(&e, FORM_SET_STATE).is_empty());
        assert!(calls_to(&e, IMPACT_SWAP_INIT_ITEM).is_empty());
        // The null entry of the map is skipped; the persistent cell is last.
        assert_eq!(
            calls_to(&e, INIT),
            vec![vec![cells[0]], vec![cells[1]], vec![cells[2]]]
        );
        assert!(!e.get(w, TESWorldSpace::pTerrainManager).is_null());
    }

    // ---- ordering and duplicates ---------------------------------------------------

    const WORLD_SAVES_BEFORE: u32 = 0x0058_5100;
    const CHILD_OWNER: u32 = 0x0058_5200;

    #[test]
    fn saves_before_a_form_orders_cells_references_and_worlds() {
        let mut e = engine();
        let w = world(&mut e);
        give_vtable(&mut e, w.addr(), &[(0x3C, WORLD_SAVES_BEFORE)]);
        // The world space's own answer for another world space.
        e.register_double(WORLD_SAVES_BEFORE, |_, a| ret((a[1] == 0x7001) as u32));
        e.mem.set_u32(w.addr() + 0xC, 0x20);
        let form = e.mem.alloc(0x20);
        // A cell: ordered by the world space it is in.
        stub(&mut e, FORM_TYPE, 0x39);
        stub(&mut e, CELL_GET_WORLD_SPACE, w.addr());
        logged(&mut e);
        assert!(e.call(0x0058_4be0, &args![w, form]).bool());
        // Its own world space: true without asking the world space.
        assert!(calls_to(&e, WORLD_SAVES_BEFORE).is_empty());
        assert_eq!(calls_to(&e, RT_DYNAMIC_CAST)[0][3], TYPE_TES_OBJECT_CELL);
        stub(&mut e, CELL_GET_WORLD_SPACE, 0x7001);
        assert!(e.call(0x0058_4be0, &args![w, form]).bool());
        stub(&mut e, CELL_GET_WORLD_SPACE, 0x7002);
        assert!(!e.call(0x0058_4be0, &args![w, form]).bool());
        // A cell with no world space (an interior cell).
        stub(&mut e, CELL_GET_WORLD_SPACE, 0);
        assert!(!e.call(0x0058_4be0, &args![w, form]).bool());
        // A cell that does not cast.
        e.register(RT_DYNAMIC_CAST, |_, _| ret(0));
        assert!(!e.call(0x0058_4be0, &args![w, form]).bool());
        e.register(RT_DYNAMIC_CAST, |_, a| ret(a[0]));

        // A placed reference: ordered by what its child-cell part gives.
        let child = e.mem.alloc(0x20);
        give_vtable(&mut e, child, &[(0, CHILD_OWNER)]);
        e.register_double(CHILD_OWNER, |_, _| ret(0x7001));
        e.register_double(RT_DYNAMIC_CAST, move |_, _| ret(child));
        for form_type in [0x3a, 0x40, 0x42, 0x43, 0x69] {
            stub(&mut e, FORM_TYPE, form_type);
            assert!(
                e.call(0x0058_4be0, &args![w, form]).bool(),
                "{form_type:#x}"
            );
        }
        // Types next to the range take the default.
        stub(&mut e, FORM_SAVES_BEFORE_FORM, 0);
        for form_type in [0x38, 0x44, 0x68, 0x6a] {
            stub(&mut e, FORM_TYPE, form_type);
            assert!(
                !e.call(0x0058_4be0, &args![w, form]).bool(),
                "{form_type:#x}"
            );
        }
        e.register(RT_DYNAMIC_CAST, |_, a| ret(a[0]));

        // Another world space: by form ID.
        stub(&mut e, FORM_TYPE, 0x41);
        e.mem.set_u32(form + 0xC, 0x21);
        assert!(e.call(0x0058_4be0, &args![w, form]).bool());
        e.mem.set_u32(form + 0xC, 0x20);
        assert!(!e.call(0x0058_4be0, &args![w, form]).bool());
        e.mem.set_u32(form + 0xC, 0x1f);
        assert!(!e.call(0x0058_4be0, &args![w, form]).bool());

        // Anything else: the base class's order.
        stub(&mut e, FORM_TYPE, 0x2a);
        stub(&mut e, FORM_SAVES_BEFORE_FORM, 1);
        assert!(e.call(0x0058_4be0, &args![w, form]).bool());
    }

    fn group_header(e: &mut Engine, group_type: u32, label: u32) -> u32 {
        let header = e.mem.alloc(0x20);
        e.mem.set_u32(header, e.global::<u32>(GROUP_TAG));
        e.mem.set_u32(header + 8, label);
        e.mem.set_u32(header + 0xC, group_type);
        header
    }

    #[test]
    fn saves_before_a_group_header_depends_on_the_group_type() {
        let mut e = engine();
        let w = world(&mut e);
        give_vtable(&mut e, w.addr(), &[(0x3C, WORLD_SAVES_BEFORE)]);
        e.register_double(WORLD_SAVES_BEFORE, |_, a| ret((a[1] == 0x7001) as u32));
        // The forms by label: a world space and a cell.
        e.register_double(FORM_BY_ID, |_, a| ret(a[0] + 0x6000));
        stub(&mut e, FORM_SAVES_BEFORE_HEADER, 1);
        let ask = |e: &mut Engine, kind: u32, label: u32| {
            let header = group_header(e, kind, label);
            e.call(0x0058_4d40, &args![w, header]).bool()
        };
        // Type 0: the base class decides.
        logged(&mut e);
        assert!(ask(&mut e, 0, 0));
        assert_eq!(calls_to(&e, FORM_SAVES_BEFORE_HEADER).len(), 1);
        // Types 1 and 6: the labelled form is cast to a world space / a cell
        // and ordered through slot +0x3c.
        assert!(ask(&mut e, 1, 0x1001));
        assert!(!ask(&mut e, 1, 0x1002));
        assert!(ask(&mut e, 6, 0x1001));
        let casts: Vec<u32> = calls_to(&e, RT_DYNAMIC_CAST).iter().map(|c| c[3]).collect();
        assert_eq!(
            casts,
            vec![
                TYPE_TES_WORLD_SPACE,
                TYPE_TES_WORLD_SPACE,
                TYPE_TES_OBJECT_CELL
            ]
        );
        // The form is not there.
        e.register_double(FORM_BY_ID, |_, _| ret(0));
        assert!(!ask(&mut e, 1, 0x1001));
        // Types 7: yes; 2, 3, 4, 5, 8, 9 and beyond: no.
        assert!(ask(&mut e, 7, 0));
        for kind in [2, 3, 4, 5, 8, 9, 10] {
            assert!(!ask(&mut e, kind, 0), "{kind}");
        }
        // Not a group header; no header.
        let record = e.mem.alloc(0x20);
        e.mem.set_u32(record, 0x1234);
        assert!(!e.call(0x0058_4d40, &args![w, record]).bool());
        assert!(!e.call(0x0058_4d40, &args![w, 0u32]).bool());
    }

    const CELL_DUPLICATE: u32 = 0x0054_f300;
    const CELL_SET_ALTERED: u32 = 0x0054_f400;

    #[test]
    fn create_duplicate_form_copies_the_cells_into_the_new_world_space() {
        let mut e = engine();
        let w = world(&mut e);
        let copy = world(&mut e);
        e.register_double(FORM_CREATE_DUPLICATE, move |_, _| ret(copy.addr()));
        let copy_map = e.mem.alloc(0x10);
        e.mem.set_u32(copy.addr() + 0x30, copy_map);
        noop(&mut e, MAP_REMOVE_ALL);
        noop(&mut e, ADD_CELL);
        noop(&mut e, FINISH_DUPLICATE);
        e.register(GET_PERSISTENT_CELL, |e, a| ret(e.mem.u32(a[0] + 0x34)));
        // Each source cell duplicates into a new cell object.
        e.register_double(CELL_DUPLICATE, |e, _| {
            let duplicate = e.mem.alloc(0x20);
            give_vtable(e, duplicate, &[(0xC8, CELL_SET_ALTERED)]);
            ret(duplicate)
        });
        noop(&mut e, CELL_SET_ALTERED);
        let sources: Vec<u32> = (0..3)
            .map(|_| {
                let cell = e.mem.alloc(0x20);
                give_vtable(&mut e, cell, &[(0x40, CELL_DUPLICATE)]);
                cell
            })
            .collect();
        install_cell_map(&mut e, vec![sources[0], 0, sources[1]]);
        e.set(w, TESWorldSpace::pPersistentCell, Ptr::new(sources[2]));
        logged(&mut e);
        let other = 0xcafeu32;
        let result = e.call(0x0058_4e60, &args![w, 1u32, other]).ptr::<()>();
        assert_eq!(result, copy.cast());
        assert_eq!(
            calls_to(&e, FORM_CREATE_DUPLICATE),
            vec![vec![w.addr(), 1, other]]
        );
        // The copy's cell map is emptied first.
        assert_eq!(calls_to(&e, MAP_REMOVE_ALL), vec![vec![copy_map]]);
        // Each non-null cell is duplicated with (0, other), marked altered
        // (slot +0xc8, argument 1) and added to the copy.
        let duplicates = calls_to(&e, CELL_DUPLICATE);
        assert_eq!(
            duplicates,
            vec![
                vec![sources[0], 0, other],
                vec![sources[1], 0, other],
                vec![sources[2], 0, other]
            ]
        );
        let added = calls_to(&e, ADD_CELL);
        assert_eq!(added.len(), 2);
        assert!(added.iter().all(|a| a[0] == copy.addr()));
        let altered = calls_to(&e, CELL_SET_ALTERED);
        assert_eq!(altered.len(), 3);
        assert!(altered.iter().all(|a| a[1] == 1));
        // The persistent cell's duplicate becomes the copy's.
        let persistent = e.get(copy, TESWorldSpace::pPersistentCell);
        assert_eq!(persistent.addr(), altered[2][0]);
        assert_eq!(calls_to(&e, FINISH_DUPLICATE), vec![vec![copy.addr()]]);
    }

    #[test]
    fn create_duplicate_form_skips_duplicates_that_are_not_cells_and_a_missing_persistent_cell() {
        let mut e = engine();
        let w = world(&mut e);
        let copy = world(&mut e);
        e.register_double(FORM_CREATE_DUPLICATE, move |_, _| ret(copy.addr()));
        let copy_map = e.mem.alloc(0x10);
        e.mem.set_u32(copy.addr() + 0x30, copy_map);
        noop(&mut e, MAP_REMOVE_ALL);
        noop(&mut e, ADD_CELL);
        noop(&mut e, FINISH_DUPLICATE);
        e.register(GET_PERSISTENT_CELL, |e, a| ret(e.mem.u32(a[0] + 0x34)));
        // The duplicate of the cell is not a cell.
        e.register_double(CELL_DUPLICATE, |_, _| ret(0x9999));
        e.register(RT_DYNAMIC_CAST, |_, a| {
            ret(if a[3] == TYPE_TES_OBJECT_CELL {
                0
            } else {
                a[0]
            })
        });
        let cell = e.mem.alloc(0x20);
        give_vtable(&mut e, cell, &[(0x40, CELL_DUPLICATE)]);
        install_cell_map(&mut e, vec![cell]);
        logged(&mut e);
        e.call(0x0058_4e60, &args![w, 0u32, 0u32]);
        assert!(calls_to(&e, ADD_CELL).is_empty());
        assert!(e.get(copy, TESWorldSpace::pPersistentCell).is_null());
        assert_eq!(calls_to(&e, FINISH_DUPLICATE).len(), 1);
    }

    // ---- Copy and Compare ----------------------------------------------------------

    const TEXTURE_COPY: u32 = 0x0054_f500;
    const TEXTURE_COMPARE: u32 = 0x0054_f600;

    /// Doubles shared by `Copy` and `Compare`: the getters read the fields.
    fn copy_engine() -> Engine {
        let mut e = engine();
        install_parents(&mut e, vec![]);
        set_word(&mut e, DEFAULT_WATER, 0x00dd_0001);
        e.register(GET_IMAGE_SPACE_RAW, |e, a| ret(e.mem.u32(a[0] + 0x44)));
        e.register(GET_LOD_WATER_HEIGHT, |e, a| {
            ret_float(e.mem.f32(a[0] + 0x7C))
        });
        e
    }

    /// A world space with distinctive values; its textures have vtables.
    fn filled_world(e: &mut Engine, salt: u32) -> Ptr<TESWorldSpace> {
        let w = world(e);
        e.set(w, TESWorldSpace::cFlags, 0x11);
        e.set(w, TESWorldSpace::sParentUseFlags, 0x22);
        e.set(w, TESWorldSpace::pParentWorld, Ptr::new(0x70 + salt));
        e.set(w, TESWorldSpace::pEncounterZone, Ptr::new(0xd0 + salt));
        e.set(w, TESWorldSpace::pClimate, Ptr::new(0x40 + salt));
        e.set(w, TESWorldSpace::pImageSpace, Ptr::new(0x44 + salt));
        e.set(w, TESWorldSpace::pWorldWater, Ptr::new(0x74 + salt));
        e.set(w, TESWorldSpace::pLODWater, Ptr::new(0x78 + salt));
        e.set(w, TESWorldSpace::fLODWaterHeight, 5.0 + salt as f32);
        e.set(w, TESWorldSpace::fDefaultLandHeight, -1.0 + salt as f32);
        e.set(w, TESWorldSpace::fDefaultWaterHeight, 2.0 + salt as f32);
        e.set(w, TESWorldSpace::pMusicType, Ptr::new(0x9c + salt));
        e.mem.write(w.addr() + 0x80, &[salt as u8 + 1; 0x10]);
        e.mem.write(w.addr() + 0x90, &[salt as u8 + 2; 0xC]);
        give_vtable(
            e,
            w.addr() + 0xD4,
            &[(8, TEXTURE_COPY), (0xC, TEXTURE_COMPARE)],
        );
        give_vtable(
            e,
            w.addr() + 0xE0,
            &[(8, TEXTURE_COPY), (0xC, TEXTURE_COMPARE)],
        );
        w
    }

    #[test]
    fn copy_takes_every_value_of_the_source_world_space() {
        let mut e = copy_engine();
        let this = filled_world(&mut e, 0);
        let source = filled_world(&mut e, 0x10);
        for addr in [
            FORM_COPY_COMPONENTS,
            SET_WATER,
            SET_LOD_WATER,
            SET_MUSIC_TYPE,
            TEXTURE_COPY,
            MAP_REMOVE_ALL,
            MAP_SET_AT,
        ] {
            noop(&mut e, addr);
        }
        // The source map has two entries (key = index).
        install_cell_map(&mut e, vec![0xc1, 0xc2]);
        let source_map = e.mem.alloc(0x10);
        let own_map = e.mem.alloc(0x10);
        e.set(source, TESWorldSpace::pCellMap, Ptr::new(source_map));
        e.set(this, TESWorldSpace::pCellMap, Ptr::new(own_map));
        logged(&mut e);
        e.call(0x0058_4fa0, &args![this, source]);
        assert_eq!(
            calls_to(&e, FORM_COPY_COMPONENTS),
            vec![vec![this.addr(), source.addr()]]
        );
        assert_eq!(e.get(this, TESWorldSpace::cFlags), 0x11);
        assert_eq!(e.get(this, TESWorldSpace::sParentUseFlags), 0x22);
        assert_eq!(e.get(this, TESWorldSpace::pParentWorld).addr(), 0x80);
        assert_eq!(e.get(this, TESWorldSpace::pEncounterZone).addr(), 0xe0);
        assert_eq!(e.get(this, TESWorldSpace::pClimate).addr(), 0x50);
        assert_eq!(e.get(this, TESWorldSpace::pImageSpace).addr(), 0x54);
        assert_eq!(calls_to(&e, SET_WATER), vec![vec![this.addr(), 0x84]]);
        assert_eq!(calls_to(&e, SET_LOD_WATER), vec![vec![this.addr(), 0x88]]);
        assert_eq!(calls_to(&e, SET_MUSIC_TYPE), vec![vec![this.addr(), 0xac]]);
        assert_eq!(e.get(this, TESWorldSpace::fLODWaterHeight), 21.0);
        assert_eq!(e.get(this, TESWorldSpace::fDefaultLandHeight), 15.0);
        assert_eq!(e.get(this, TESWorldSpace::fDefaultWaterHeight), 18.0);
        assert_eq!(e.mem.bytes(this.addr() + 0x80, 0x10), vec![0x11; 0x10]);
        assert_eq!(e.mem.bytes(this.addr() + 0x90, 0xC), vec![0x12; 0xC]);
        // The cell map is emptied and refilled with the source's entries.
        assert_eq!(calls_to(&e, MAP_REMOVE_ALL), vec![vec![own_map]]);
        assert_eq!(
            calls_to(&e, MAP_SET_AT),
            vec![vec![own_map, 0, 0xc1], vec![own_map, 1, 0xc2]]
        );
        // Both textures copy the source's texture.
        assert_eq!(
            calls_to(&e, TEXTURE_COPY),
            vec![
                vec![this.addr() + 0xD4, source.addr() + 0xD4],
                vec![this.addr() + 0xE0, source.addr() + 0xE0]
            ]
        );
    }

    #[test]
    fn copy_makes_copies_or_drops_the_impact_swap_to_match_the_source() {
        const SWAP: u32 = 0x5151;
        const OTHER_SWAP: u32 = 0x5252;
        for (source_has, own_has) in [(true, false), (true, true), (false, true), (false, false)] {
            let mut e = copy_engine();
            let this = filled_world(&mut e, 0);
            let source = filled_world(&mut e, 0x10);
            for addr in [
                FORM_COPY_COMPONENTS,
                SET_WATER,
                SET_LOD_WATER,
                SET_MUSIC_TYPE,
                TEXTURE_COPY,
                MAP_REMOVE_ALL,
                IMPACT_SWAP_COPY,
                IMPACT_SWAP_DESTRUCT,
            ] {
                noop(&mut e, addr);
            }
            e.register(IMPACT_SWAP_CONSTRUCT, |_, a| ret(a[0]));
            install_cell_map(&mut e, vec![]);
            if source_has {
                e.set(source, TESWorldSpace::pImpactSwap, Ptr::new(SWAP));
            }
            if own_has {
                e.set(this, TESWorldSpace::pImpactSwap, Ptr::new(OTHER_SWAP));
            }
            logged(&mut e);
            e.call(0x0058_4fa0, &args![this, source]);
            let own = e.get(this, TESWorldSpace::pImpactSwap).addr();
            match (source_has, own_has) {
                (true, false) => {
                    // A new swap (0x15C bytes) is constructed, then copied to.
                    assert_ne!(own, 0);
                    assert_eq!(calls_to(&e, IMPACT_SWAP_CONSTRUCT), vec![vec![own]]);
                    assert_eq!(calls_to(&e, IMPACT_SWAP_COPY), vec![vec![own, SWAP]]);
                }
                (true, true) => {
                    assert_eq!(own, OTHER_SWAP);
                    assert!(calls_to(&e, IMPACT_SWAP_CONSTRUCT).is_empty());
                    assert_eq!(calls_to(&e, IMPACT_SWAP_COPY), vec![vec![OTHER_SWAP, SWAP]]);
                }
                (false, true) => {
                    // Deleted through the destructor with the delete flag.
                    assert_eq!(own, 0);
                    assert_eq!(
                        calls_to(&e, IMPACT_SWAP_DESTRUCT),
                        vec![vec![OTHER_SWAP, 1]]
                    );
                    assert!(calls_to(&e, IMPACT_SWAP_COPY).is_empty());
                }
                (false, false) => {
                    assert_eq!(own, 0);
                    assert!(calls_to(&e, IMPACT_SWAP_COPY).is_empty());
                    assert!(calls_to(&e, IMPACT_SWAP_DESTRUCT).is_empty());
                }
            }
        }
    }

    #[test]
    fn copy_of_something_that_is_not_a_world_space_does_nothing() {
        let mut e = copy_engine();
        let this = filled_world(&mut e, 0);
        e.register(RT_DYNAMIC_CAST, |_, _| ret(0));
        logged(&mut e);
        e.call(0x0058_4fa0, &args![this, 0x1234u32]);
        assert_eq!(e.get(this, TESWorldSpace::pClimate).addr(), 0x40);
        assert_eq!(addresses(&e), vec![0x0058_4fa0, RT_DYNAMIC_CAST]);
    }

    fn compare_engine() -> Engine {
        let mut e = copy_engine();
        stub(&mut e, FORM_COMPARE_COMPONENTS, 0);
        stub(&mut e, IMPACT_SWAP_COMPARE, 0);
        stub(&mut e, TEXTURE_COMPARE, 0);
        e
    }

    #[test]
    fn compare_of_identical_world_spaces_says_they_are_the_same() {
        let mut e = compare_engine();
        let a = filled_world(&mut e, 0);
        let b = filled_world(&mut e, 0);
        assert!(!e.call(0x0058_5250, &args![a, b]).bool());
        // Both with an impact swap that compares equal.
        e.set(a, TESWorldSpace::pImpactSwap, Ptr::new(0x5151));
        e.set(b, TESWorldSpace::pImpactSwap, Ptr::new(0x5252));
        logged(&mut e);
        assert!(!e.call(0x0058_5250, &args![a, b]).bool());
        assert_eq!(
            calls_to(&e, IMPACT_SWAP_COMPARE),
            vec![vec![0x5151, 0x5252]]
        );
    }

    #[test]
    fn compare_notices_each_difference() {
        type Change = (&'static str, fn(&mut Engine, Ptr<TESWorldSpace>));
        let changes: Vec<Change> = vec![
            ("flags", |e, w| e.set(w, TESWorldSpace::cFlags, 0x12)),
            ("parent flags", |e, w| {
                e.set(w, TESWorldSpace::sParentUseFlags, 0x23)
            }),
            ("parent", |e, w| {
                e.set(w, TESWorldSpace::pParentWorld, Ptr::new(1))
            }),
            ("zone", |e, w| {
                e.set(w, TESWorldSpace::pEncounterZone, Ptr::new(1))
            }),
            ("climate", |e, w| {
                e.set(w, TESWorldSpace::pClimate, Ptr::new(1))
            }),
            ("image space", |e, w| {
                e.set(w, TESWorldSpace::pImageSpace, Ptr::new(1))
            }),
            ("impact swap", |e, w| {
                e.set(w, TESWorldSpace::pImpactSwap, Ptr::new(1))
            }),
            ("water", |e, w| {
                e.set(w, TESWorldSpace::pWorldWater, Ptr::new(1))
            }),
            ("lod water", |e, w| {
                e.set(w, TESWorldSpace::pLODWater, Ptr::new(1))
            }),
            ("lod height", |e, w| {
                e.set(w, TESWorldSpace::fLODWaterHeight, 1.0)
            }),
            ("land height", |e, w| {
                e.set(w, TESWorldSpace::fDefaultLandHeight, 1.0)
            }),
            ("water height", |e, w| {
                e.set(w, TESWorldSpace::fDefaultWaterHeight, 1.0)
            }),
            ("map data", |e, w| e.mem.set_u8(w.addr() + 0x8F, 0x77)),
            ("map offset data", |e, w| {
                e.mem.set_u8(w.addr() + 0x9B, 0x77)
            }),
            ("music", |e, w| {
                e.set(w, TESWorldSpace::pMusicType, Ptr::new(1))
            }),
        ];
        for (name, change) in changes {
            let mut e = compare_engine();
            let a = filled_world(&mut e, 0);
            let b = filled_world(&mut e, 0);
            change(&mut e, b);
            assert!(e.call(0x0058_5250, &args![a, b]).bool(), "{name}");
        }
    }

    #[test]
    fn compare_asks_the_components_the_impact_swaps_and_the_textures() {
        // Components differ.
        let mut e = compare_engine();
        let (a, b) = (filled_world(&mut e, 0), filled_world(&mut e, 0));
        stub(&mut e, FORM_COMPARE_COMPONENTS, 1);
        assert!(e.call(0x0058_5250, &args![a, b]).bool());
        // Impact swaps that differ.
        let mut e = compare_engine();
        let (a, b) = (filled_world(&mut e, 0), filled_world(&mut e, 0));
        e.set(a, TESWorldSpace::pImpactSwap, Ptr::new(1));
        e.set(b, TESWorldSpace::pImpactSwap, Ptr::new(2));
        stub(&mut e, IMPACT_SWAP_COMPARE, 1);
        assert!(e.call(0x0058_5250, &args![a, b]).bool());
        // Only one of them has one.
        let mut e = compare_engine();
        let (a, b) = (filled_world(&mut e, 0), filled_world(&mut e, 0));
        e.set(a, TESWorldSpace::pImpactSwap, Ptr::new(1));
        assert!(e.call(0x0058_5250, &args![a, b]).bool());
        assert!(e.call(0x0058_5250, &args![b, a]).bool());
        // Textures: the first one, then the second.
        for (which, expected_calls) in [(0xD4u32, 1), (0xE0, 2)] {
            let mut e = compare_engine();
            let (a, b) = (filled_world(&mut e, 0), filled_world(&mut e, 0));
            let hit = a.addr() + which;
            e.register_double(TEXTURE_COMPARE, move |_, args| ret((args[0] == hit) as u32));
            logged(&mut e);
            assert!(e.call(0x0058_5250, &args![a, b]).bool(), "{which:#x}");
            assert_eq!(calls_to(&e, TEXTURE_COMPARE).len(), expected_calls);
        }
        // Not a world space: different.
        let mut e = compare_engine();
        let a = filled_world(&mut e, 0);
        e.register(RT_DYNAMIC_CAST, |_, _| ret(0));
        assert!(e.call(0x0058_5250, &args![a, 0x1234u32]).bool());
    }

    #[test]
    fn compare_does_not_equate_nan_heights() {
        let mut e = compare_engine();
        let (a, b) = (filled_world(&mut e, 0), filled_world(&mut e, 0));
        e.set(a, TESWorldSpace::fDefaultLandHeight, f32::NAN);
        e.set(b, TESWorldSpace::fDefaultLandHeight, f32::NAN);
        assert!(e.call(0x0058_5250, &args![a, b]).bool());
    }
}
