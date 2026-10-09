//! `fallout shared/tesland.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds `TESObjectLAND`, the landscape record of one exterior
//! cell (152 functions to translate). The first session translated the
//! first 40 open or traced functions by address, `00533120` to `005374f0`:
//! the constructor and destructors, `InitializeStatics` (the shared default
//! triangle list, colours, normals, texture coordinates and vertex blocks),
//! the cell accessors and the flag bits of the record data, the vertex,
//! normal and colour readers, the form-record `Load`, `LoadVertices` with
//! `LoadVerticesIntoArrays` (the loader that reads the land chunks straight
//! from the plugin file), `UnLoadVertices`, `MakeLandTriStrips` and the
//! function that builds the four quadrant meshes (`005374f0`). The next
//! session continues with the first `open` function after `005374f0`
//! (`00537b10`).
//!
//! ## Layout (PC build)
//!
//! `TESObjectLAND` is 0x2C bytes on the PC (`TESForm` 0x18, the
//! `TESChildCell` vtable at +0x18, then the Xbox fields from `Data` on); the
//! Xbox PDB has them 0x10 higher. `LoadedLandData` (0xA4 bytes) holds the
//! four quadrants of the land: `ppMesh`, `ppVertices`, `ppNormals`,
//! `ppColorsA` and `ppNormalsSet` point to arrays of four pointers (one per
//! quadrant, 0x121 = 17 x 17 entries each); the texture, percent and grass
//! arrays have four entries on the PC where the PDB has sixteen.
//!
//! ## Chunks
//!
//! The land record chunks are `DATA` (flags), `VHGT` (heights), `VNML`
//! (normals), `VCLR` (vertex colours), `MPCD` (Havok MOPP code), `BTXT` (base
//! texture of a quadrant), `ATXT` (additional texture layer) and `VTXT`
//! (the layer's per-vertex opacity). A chunk holds a 33 x 33 grid; each
//! quadrant takes the 17 x 17 block that starts at
//! `(quadrant / 2) * 16 * 33 + (quadrant % 2) * 16`. `Load` (the record
//! loader) and `LoadVerticesIntoArrays` (the loader that finds the record
//! in the file again) have the same chunk code inlined; the translations
//! share it in the private functions below (`read_height_chunk` and the
//! others) and keep what differs (the flags that gate a chunk, the
//! messages, the opacity scale of `VTXT`, the `DATA` handling) in the two
//! loaders.
//!
//! ## Helpers the translations call by address
//!
//! `00559450` reads a smart-pointer slot (`*this`); `00401000` and `00401030`
//! are the allocator; `00401460` is `memcpy` and `00403d30` `memset`;
//! `00416870`, `00452dc0` and `00414430` construct an `NiPoint3`, `NiPoint2`
//! and `NiColorA` and return it; `00404eb0`/`00404ee0` are the scope guard
//! that tags allocations with the source line; `007af430` is the folded
//! getter of the `+0x20` field (`pParentCell` here); `0084e3a0` reads the
//! form id (`+0x0C`, and the count of an `NiTPointerMap`).
//!
//! x87: the code stores `float`s; the translations compute in `f64` where
//! the code does and round to `f32` at each store (the sum of two floats is
//! the same either way).
//!
//! Not translated: the C++ exception-unwinding frames (`FS:[0]` chains) of
//! the constructor, destructor, `InitializeStatics`, `Load`, `LoadVertices`,
//! `LoadVerticesIntoArrays`, `MakeLandTriStrips` and the `LoadedLandData`
//! destructor body, and the stack-cookie checks.

#[allow(unused_imports)]
use crate::prelude::*;

// ---- Data of the exe ------------------------------------------------------

/// `TESObjectLAND`'s vtable and the `TESChildCell` vtable at +0x18, and the
/// vtable the `TESChildCell` constructor stores before the land overwrites
/// it (a class with a pure virtual function).
pub(crate) const LAND_VTABLE: u32 = 0x0102_dcd4;
pub(crate) const CHILD_CELL_VTABLE: u32 = 0x0102_dccc;
pub(crate) const CHILD_CELL_ABSTRACT_VTABLE: u32 = 0x0102_de10;

/// `TESObjectLAND`'s statics (Xbox PDB names): the triangle list, colour,
/// normal and texture coordinate blocks every land shares, the "normal set"
/// flags, `bStaticsDefined`, `iLANDsinuse`, the default land texture and its
/// texture set, and the default texturing property.
pub(crate) const DEFAULT_TRIANGLE_LIST: u32 = 0x011c_9ee8;
pub(crate) const DEFAULT_COLORS: u32 = 0x011c_9eec;
pub(crate) const DEFAULT_NORMALS: u32 = 0x011c_9ef0;
pub(crate) const DEFAULT_TEXTURE_COORDINATES: u32 = 0x011c_9ef4;
pub(crate) const DEFAULT_NORMAL_SET: u32 = 0x011c_9ef8;
pub(crate) const STATICS_DEFINED: u32 = 0x011c_9efc;
pub(crate) const LANDS_IN_USE: u32 = 0x011c_9f00;
pub(crate) const DEFAULT_LAND_TEXTURE: u32 = 0x011c_9f04;
pub(crate) const DEFAULT_TEXTURE_SET: u32 = 0x011c_9f08;
/// `NiPointer<NiTexturingProperty>` `spDefTexProp`; the INI setting
/// `sDefaultLandDiffuseTexture` follows it, `sDefaultLandNormalTexture` is at
/// `011c9f8c`.
pub(crate) const DEFAULT_TEXTURING_PROPERTY: u32 = 0x011c_a000;
pub(crate) const DEFAULT_DIFFUSE_TEXTURE_SETTING: u32 = 0x011c_a004;
pub(crate) const DEFAULT_NORMAL_TEXTURE_SETTING: u32 = 0x011c_9f8c;
/// The INI setting `fLandTextureTilingMult` (a float setting).
pub(crate) const TEXTURE_TILING_SETTING: u32 = 0x011c_9f2c;
/// `ppDefVertexBLOCK[4]`, `pXOffsetBLOCK[4]`, `pYOffsetBLOCK[4]`.
pub(crate) const DEFAULT_VERTEX_BLOCKS: u32 = 0x011c_9ed4;
pub(crate) const X_OFFSETS: u32 = 0x011c_9ec4;
pub(crate) const Y_OFFSETS: u32 = 0x011c_9eb4;
/// `pTriStripListBLOCK`: the triangle strip of a quadrant (`u16[1021]`,
/// 0x7FA bytes).
pub(crate) const TRIANGLE_STRIP_LIST: u32 = 0x0118_ae90;
/// The menu manager singleton whose method receives the default texture set.
const MENU_MANAGER: u32 = 0x011c_3f2c;
/// The default colour (four floats) the colour reader returns when the land
/// has no vertex arrays.
const DEFAULT_COLOR_WORDS: u32 = 0x011a_9be0;

/// The source file name passed to the allocation-tag scope guard, and the
/// guard's tag and size.
const SOURCE_FILE: u32 = 0x0102_de60;
const GUARD_TAG: u32 = 0x1b;
const GUARD_SIZE: u32 = 4;

/// Strings: `"Landscape\\%s"`, `"Block (%i, %i)"`, the INI warning,
/// `"UNKNOWN"` (the file name when a land has no file) and the messages of
/// `LoadVerticesIntoArrays`.
const LANDSCAPE_PATH_FORMAT: u32 = 0x0102_de50;
const BLOCK_NAME_FORMAT: u32 = 0x0102_e1c8;
const TILING_WARNING: u32 = 0x0102_de14;
const UNKNOWN_FILE_NAME: u32 = 0x0101_5890;
/// `"CELLS: Failed to load landscape data for LAND (%08X) in Cell (%i, %i)
/// from file '%s'."`.
const LAND_LOAD_FAILED: u32 = 0x0102_e170;
/// `"CELLS: Land for cell (%i, %i) in file '%s' does not contain Normal
/// Data."`.
const NO_NORMAL_DATA: u32 = 0x0102_dfe8;

/// Constants the code reads from the exe's data: the floats `FLT_MAX`,
/// `-FLT_MAX`, `16.0`, `1024.0`, `-2048.0`, and the doubles `0.0`, `1.0`,
/// `0.5`, `2.0`, `4.0`, `100.0`, `127.0`, `255.0`, `1024.0` and `2048.0`.
const LARGEST_F32: u32 = 0x0101_6970;
const LOWEST_F32: u32 = 0x0101_5f5c;
const SIXTEEN_F32: u32 = 0x0101_e57c;
const F32_1024: u32 = 0x0102_36e0;
const F32_MINUS_2048: u32 = 0x0102_dcc4;
const F64_ZERO: u32 = 0x0101_2060;
const F64_ONE: u32 = 0x0101_2070;
const F64_HALF: u32 = 0x0101_1588;
const F64_TWO: u32 = 0x0101_1590;
const F64_FOUR: u32 = 0x0101_db80;
const F64_HUNDRED: u32 = 0x0101_7a40;
const F64_127: u32 = 0x0102_dfe0;
const F64_255: u32 = 0x0101_e568;
const F64_1024: u32 = 0x0101_ecb8;
const F64_2048: u32 = 0x0101_6968;

// ---- Callees outside this file ---------------------------------------------

/// The allocator (`MemoryManager::Allocate`) and its `Deallocate`, `memcpy`
/// (`__cdecl(destination, source, size)`), `memset`
/// (`__cdecl(destination, value, size)`) and the vector constructor
/// iterator (`(array, element size, count, constructor)`).
const ALLOCATE: u32 = 0x0040_1000;
const DEALLOCATE: u32 = 0x0040_1030;
const MEMORY_COPY: u32 = 0x0040_1460;
const MEMORY_SET: u32 = 0x0040_3d30;
const VECTOR_CONSTRUCT: u32 = 0x0040_1050;
/// The Gamebryo allocator: `NiAlloc(size)` and the object allocator.
const NI_ALLOCATE: u32 = 0x00aa_1070;
const NI_ALLOCATE_OBJECT: u32 = 0x00aa_13e0;
/// `sprintf(buffer, size, format, ...)` and the message log
/// (`(format, ...)`).
const FORMAT_STRING: u32 = 0x0040_6d00;
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// The allocation-tag scope guard: `__thiscall(guard, tag, 1, file, line)`
/// and its destructor.
const SCOPE_GUARD_ENTER: u32 = 0x0040_4eb0;
const SCOPE_GUARD_LEAVE: u32 = 0x0040_4ee0;
/// `__RTDynamicCast` and the type descriptors of `TESForm`,
/// `TESObjectLAND` and `TESLandTexture`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_OBJECT_LAND: u32 = 0x0118_ac10;
const RTTI_TES_LAND_TEXTURE: u32 = 0x0118_6350;

/// Constructors of the vector types: `NiPoint3(x, y, z)`, `NiPoint2(x, y)`,
/// `NiColorA(r, g, b, a)`, each returning its `this`, and the default
/// constructors the vector constructor iterator calls.
const NI_POINT3_CONSTRUCT: u32 = 0x0041_6870;
const NI_POINT2_CONSTRUCT: u32 = 0x0045_2dc0;
const NI_COLOR_CONSTRUCT: u32 = 0x0041_4430;
const NI_POINT3_DEFAULT_CONSTRUCT: u32 = 0x0068_15c0;
const NI_COLOR_DEFAULT_CONSTRUCT: u32 = 0x004a_7800;
/// Reads the smart-pointer slot at `this` (`*this`).
const SMART_POINTER_GET: u32 = 0x0055_9450;
/// `NiPointer<NiTexturingProperty>::operator=(pointer)`, and the
/// `NiPointer<QueuedFile>` assignment.
const TEXTURING_PROPERTY_SET: u32 = 0x0066_b0d0;
const QUEUED_FILE_POINTER_SET: u32 = 0x006f_74f0;
/// Releases the `NiPointer` at `this`, and constructs one from a pointer.
const SMART_POINTER_RELEASE: u32 = 0x0045_cec0;
const SMART_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
/// `BGSTextureSet::BGSTextureSet`, `BGSTextureSet::SetTexturePath(index,
/// path)`, `TESLandTexture::TESLandTexture`, `TESLandTexture::SetTextureSet`
/// (`this` = texture, texture set), the menu manager method that receives
/// the default texture set and the call that follows it.
const TEXTURE_SET_CONSTRUCT: u32 = 0x0059_22e0;
const TEXTURE_SET_SET_PATH: u32 = 0x0059_2cc0;
const LAND_TEXTURE_CONSTRUCT: u32 = 0x0054_0c50;
const LAND_TEXTURE_SET_TEXTURE_SET: u32 = 0x0098_4f60;
const MENU_MANAGER_REGISTER_TEXTURE_SET: u32 = 0x0072_6070;
const MENU_MANAGER_AFTER_REGISTER: u32 = 0x0051_0130;
/// The string value of a string setting, and the address of the value of a
/// float setting.
const SETTING_GET_STRING: u32 = 0x0040_3df0;
const SETTING_GET_FLOAT_ADDRESS: u32 = 0x0040_3e20;
/// `NiTexturingProperty::Map::Map(this, texture, 0, 3, 5, 0)`, the
/// `NiTexturingProperty` constructor, the element address `(this, index)` and
/// element setter `(this, index, &value)` of the property's map table, and
/// `(this, value, 0xe, 1)` (`00439360`), which the default property gets with
/// value 2.
const TEXTURE_MAP_CONSTRUCT: u32 = 0x00a6_9e00;
const TEXTURING_PROPERTY_CONSTRUCT: u32 = 0x00a6_aa40;
const ARRAY_SET_AT: u32 = 0x0096_ae90;
const ARRAY_ELEMENT_ADDRESS: u32 = 0x0087_7a30;
const PROPERTY_SET_MODE: u32 = 0x0043_9360;
/// The form's constructor and destructors, `SetFormType`, `LoadForm`, the
/// form-id lookup, `AddCompileIndex(&id, file)`, `GetFile(index)`.
const FORM_CONSTRUCT: u32 = 0x0048_3370;
const FORM_DESTRUCT: u32 = 0x0048_3630;
const FORM_BASE_DESTRUCT: u32 = 0x0048_3710;
const FORM_SET_FORM_TYPE: u32 = 0x004f_15a0;
const FORM_ID: u32 = 0x0084_e3a0;
const FORM_LOAD: u32 = 0x0048_5110;
const FORM_LOOKUP_BY_ID: u32 = 0x0048_39c0;
const FORM_ADD_COMPILE_INDEX: u32 = 0x0048_5d50;
const FORM_GET_FILE: u32 = 0x0048_4e60;
const NI_POINTER_SLOT_CONSTRUCT: u32 = 0x0052_8cb0;
const QUEUED_FILE_POINTER_DESTRUCT: u32 = 0x0044_cbf0;
/// The parent cell getter (`+0x20`) and the cell's accessors.
const PARENT_CELL: u32 = 0x007a_f430;
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
const CELL_GET_3D: u32 = 0x0054_5cb0;
/// Tests bit `quadrant` of a byte of the cell's data (`005445d0` gives the
/// data, the byte is at `+8`), `(cell, quadrant) -> bool`.
const CELL_QUADRANT_TEST: u32 = 0x0054_4590;
const CELL_GET_NODE: u32 = 0x0045_c9a0;
const WORLD_SPACE_HAS_LAND: u32 = 0x0058_6390;
const WORLD_SPACE_FIND_LAND_DATA: u32 = 0x0058_5ee0;
/// `TESFile` methods.
const FILE_GET_TES_FORM: u32 = 0x0047_2660;
const FILE_GET_CHUNK: u32 = 0x0047_26b0;
const FILE_NEXT_CHUNK: u32 = 0x0047_26f0;
const FILE_GET_CHUNK_DATA: u32 = 0x0047_2890;
const FILE_GET_CHUNK_SIZE: u32 = 0x0040_1660;
const FILE_NEEDS_SWAP: u32 = 0x0040_1680;
const FILE_GET_ACTIVE: u32 = 0x0047_1d60;
const FILE_GET_NAME: u32 = 0x0089_1170;
const FILE_GET_THREAD_SAFE_FILE: u32 = 0x0047_39b0;
const FILE_FIND_FORM: u32 = 0x0047_34d0;
const FILE_SET_OFFSET: u32 = 0x0047_23a0;
const FILE_RECORD_OFFSET: u32 = 0x0046_7bb0;
/// Byte swap of the 32-bit word at `this`.
const SWAP_WORD: u32 = 0x0050_3210;
/// Whether the loading menu is up, the warning counter (`(enable)`), the
/// current thread's "loading master files" flag.
const LOADING_MENU_VISIBLE: u32 = 0x0070_5e80;
const DISABLE_WARNINGS: u32 = 0x0043_b2b0;
const LOADING_MASTER_FILE: u32 = 0x0055_1680;
/// `float` to integer conversion (`__cdecl(float) -> int`), and the
/// normalization of the `NiPoint3` at `this`.
const FLOAT_TO_INT: u32 = 0x0040_6d90;
const NI_POINT3_NORMALIZE: u32 = 0x004a_0c10;
/// Functions of the rest of this unit that later sessions translate: the
/// default height of a land without vertex data, "has vertex arrays", the
/// "vertices loaded" flag test and setter, the allocator of the loaded
/// data (`this`, source land), the setter of a vertex's layer opacity, the
/// flush of the warnings, and the Havok MOPP builder.
const LAND_DEFAULT_HEIGHT: u32 = 0x0053_a550;
const LAND_HAS_VERTEX_ARRAYS: u32 = 0x0045_cb90;
const LAND_IS_LOADED: u32 = 0x0053_94a0;
const LAND_SET_LOADED: u32 = 0x0053_94c0;
const LAND_ALLOCATE_DATA: u32 = 0x0053_9500;
const LAND_SET_VERTEX_OPACITY: u32 = 0x0053_a8a0;
const LAND_FLUSH_WARNINGS: u32 = 0x0053_a940;
const LAND_BUILD_MOPP: u32 = 0x0053_8c00;
/// What the quadrant meshes builder and the unload call in the rest of the
/// unit: the release of the border lines, the height extents of a quadrant
/// `(this, &out, quadrant)` (min, max), the three builds run after the
/// Havok bodies are made, the strips' bound (the object at `+0xb8`, which
/// `NiBound::ComputeFromData` is run on), the cell's MOPP holder (`cell +
/// 0x28` run through `0041bb10`), the check on the cell's reference list
/// (`!ListIsEnd(cell + 0xac)`), the render task of a scheduler (`+0x78`)
/// and the constructor of a local the builder keeps (`+8` = 0, `+0xc` =
/// 1.0).
const LAND_RELEASE_BORDER: u32 = 0x0053_7eb0;
const LAND_QUADRANT_EXTENTS: u32 = 0x0053_f390;
const LAND_BUILD_FOLLOW_UPS: [u32; 3] = [0x0053_bc10, 0x0053_aeb0, 0x0053_9960];
const LAND_BOUND_SOURCE: u32 = 0x0053_7b10;
const LAND_CELL_MOPP: u32 = 0x0053_7b30;
const CELL_REFERENCE_LIST_CHECK: u32 = 0x0053_7b50;
const LAND_RENDER_TASK: u32 = 0x0053_7bd0;
const LAND_MOPP_LOCAL_CONSTRUCT: u32 = 0x0053_7b80;
/// The float getter of `fBaseHeight` (`0049db00`: the loaded data's, 0 without), the renderer getter, the form lookup
/// `(this, 0, id)` and the two byte swaps `(pointer, 0)`.
const LAND_BASE_HEIGHT: u32 = 0x0049_db00;
const RENDERER_GET: u32 = 0x0043_c4b0;
const FORM_LOOKUP: u32 = 0x0048_67a0;
const SWAP_BYTES_32: u32 = 0x0040_1080;
const SWAP_BYTES_16: u32 = 0x0040_7a90;
/// `NiGeometry` and `NiGeometryData` calls. The node's data accessors:
/// `(this, 0)` gives the geometry of mesh node, then the vertex, normal and
/// colour arrays of a geometry.
const GEOMETRY_GET_MODEL_DATA: u32 = 0x0043_b4a0;
const GEOMETRY_GET_POSITIONS: u32 = 0x0049_ec60;
const GEOMETRY_GET_NORMALS: u32 = 0x004a_8030;
const GEOMETRY_GET_COLORS: u32 = 0x004a_8050;
const GEOMETRY_GET_DATA: u32 = 0x0054_95f0;
const GEOMETRY_DATA_GET_OWNER: u32 = 0x004a_8a90;
/// The lock-supported slot of the owner (`0x8c`), the geometry data's
/// `Begin(1)` and the position and colour locks, and `End`.
const OWNER_CAN_LOCK_SLOT: u32 = 0x8c;
const GEOMETRY_DATA_BEGIN: u32 = 0x00a6_7360;
const GEOMETRY_DATA_LOCK_POSITIONS: u32 = 0x00a6_75b0;
const GEOMETRY_DATA_LOCK_COLORS: u32 = 0x00a6_7660;
const GEOMETRY_DATA_END: u32 = 0x00a6_73d0;
/// The vertex lock (`{base, stride, packed}`): its constructor and the index
/// check of its readers.
const LOCK_CONSTRUCT: u32 = 0x0054_0720;
const LOCK_CHECK_INDEX: u32 = 0x0045_34f0;
/// Scene node calls of the mesh builder: the culled flag getter and setter,
/// `AttachProperty`, `AttachChild` (virtual slot `0xf8`), `Update`,
/// `UpdateProperties`, and the update data constructor.
const NODE_GET_CULLED: u32 = 0x0045_6610;
const NODE_SET_CULLED: u32 = 0x0045_0f90;
const NODE_ATTACH_PROPERTY: u32 = 0x0043_9410;
const NODE_ATTACH_CHILD_SLOT: u32 = 0xf8;
const NODE_UPDATE: u32 = 0x00a5_9c60;
const NODE_UPDATE_PROPERTIES: u32 = 0x00a5_a040;
const UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
/// `NiTriStrips::NiTriStrips`, its translate setter, its vertex count, its
/// bound computation (`(bound, count, positions)`), its model data
/// `MarkAsChanged`, `SetConsistency`, the two flag setters and the three
/// mesh node accessors.
const TRI_STRIPS_CONSTRUCT: u32 = 0x00a7_1c40;
const NODE_SET_TRANSLATE: u32 = 0x0044_0460;
const TRI_STRIPS_VERTEX_COUNT: u32 = 0x0045_6650;
const BOUND_COMPUTE_FROM_DATA: u32 = 0x00a7_ee30;
const GEOMETRY_DATA_MARK_CHANGED: u32 = 0x00a6_7090;
const GEOMETRY_DATA_SET_CONSISTENCY: u32 = 0x00a6_7050;
const GEOMETRY_DATA_SET_KEEP_FLAGS: u32 = 0x0044_10f0;
const GEOMETRY_DATA_SET_COMPRESS_FLAGS: u32 = 0x0044_10d0;
const FACE_GEN_NODE_GET_ANIMATION_DATA: u32 = 0x0066_29f0;
const ANIMATION_DATA_GET_ROOT: u32 = 0x0043_b230;
const ROOT_SET_LOCAL_TRANSLATE: u32 = 0x0043_9680;
const ROOT_SET_WORLD_TRANSLATE_SLOT: u32 = 0xb8;
const OBJECT_SET_NAME: u32 = 0x00a5_b950;
const FIXED_STRING_CONSTRUCT: u32 = 0x0043_8170;
const FIXED_STRING_DESTRUCT: u32 = 0x0043_81b0;
/// `CellMopp::Create(this, strips, 4, base height)`, `CellMopp` preparation,
/// the geometry's scheduler, the scheduler's saved acquire object, the
/// renderer's queue slot (`0xec`) and the call after it.
const CELL_MOPP_CREATE: u32 = 0x0062_1f60;
const CELL_MOPP_PREPARE: u32 = 0x0062_1e60;
const GEOMETRY_GET_SCHEDULER: u32 = 0x0040_30b0;
const SCHEDULER_GET_SAVED_ACQUIRE: u32 = 0x008d_8520;
const RENDERER_QUEUE_SLOT: u32 = 0xec;
const RENDERER_AFTER_QUEUE: u32 = 0x00e7_4120;
/// `~LoadedLandData`'s pieces: the vector destructor iterator, the
/// destructor of one grass map, `hkReferencedObject::removeReference` and
/// `addReference`, the `NiTPointerMap` calls (first position, next, remove
/// all).
const VECTOR_DESTRUCT_ITERATOR: u32 = 0x00ec_5fce;
const GRASS_MAP_DESTRUCT: u32 = 0x0054_09b0;
const HAVOK_REMOVE_REFERENCE: u32 = 0x00c9_05b0;
const HAVOK_ADD_REFERENCE: u32 = 0x00c9_0510;
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
const MAP_GET_NEXT: u32 = 0x006b_7f20;
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;
/// `QueuedFile::QueuedFile(this, 5)` and `TESLandTexture::QueueTexture(
/// texture, 5, queued file)`.
const QUEUED_FILE_CONSTRUCT: u32 = 0x00c3_c590;
const LAND_TEXTURE_QUEUE: u32 = 0x0054_1540;
/// The render task priority the loaders pass.
const QUEUE_PRIORITY: u32 = 5;

// ---- Layouts ---------------------------------------------------------------

layout! {
    /// `TESObjectLAND` (Xbox PDB), 0x2C bytes on the PC: `TESForm` (0x18),
    /// the `TESChildCell` vtable at +0x18, then the fields below.
    pub struct TESObjectLAND: 0x2c {
        /// `Data` (Xbox PDB, `OBJ_LAND`): the land's flags. Bit 0x01: height
        /// and normal data present; 0x02: vertex colours; 0x04: textures;
        /// 0x10: set by the editor; 0x400: the data is read from the world
        /// space's land file; 0x800: the Havok MOPP code was built.
        0x1C Data: u32,
        /// `pParentCell` (Xbox PDB): the cell the land belongs to.
        0x20 pParentCell: Ptr,
        /// `spQueuedTextures` (Xbox PDB): `NiPointer<QueuedFile>` slot.
        0x24 spQueuedTextures: Ptr,
        /// `pLoadedData` (Xbox PDB): `LoadedLandData*`.
        0x28 pLoadedData: Ptr,
    }

    /// `TESObjectLAND::LoadedLandData` (Xbox PDB), 0xA4 bytes on both builds.
    pub struct LoadedLandData: 0xa4 {
        /// `ppMesh` (Xbox PDB): four `NiNode*`.
        0x00 ppMesh: Ptr,
        /// `ppVertices` (Xbox PDB): four arrays of 0x121 `NiPoint3`.
        0x04 ppVertices: Ptr,
        /// `ppNormals` (Xbox PDB): four arrays of 0x121 `NiPoint3`.
        0x08 ppNormals: Ptr,
        /// `ppColorsA` (Xbox PDB): four arrays of 0x121 `NiColorA`.
        0x0C ppColorsA: Ptr,
        /// `ppNormalsSet` (Xbox PDB).
        0x10 ppNormalsSet: Ptr,
        /// `spBorder` (Xbox PDB): `NiPointer<NiLines>` slot.
        0x14 spBorder: Ptr,
        /// `HeightExtents.x` (Xbox PDB `NiPoint2`): the lowest vertex height.
        0x18 HeightExtentsMin: f32,
        /// `HeightExtents.y` (Xbox PDB): the highest vertex height.
        0x1C HeightExtentsMax: f32,
        /// `pDefQuadTexture` (Xbox PDB): four `TESLandTexture*`.
        0x20 pDefQuadTexture: Ptr,
        /// `pQuadTextureArray` (Xbox PDB): four arrays of layer textures.
        0x30 pQuadTextureArray: Ptr,
        /// `ppPercentArrays` (Xbox PDB): four arrays of 0x121-entry opacity
        /// blocks, one per layer.
        0x40 ppPercentArrays: Ptr,
        /// `pMoppCode` (Xbox PDB): `hkpMoppCode*`.
        0x50 pMoppCode: Ptr,
        /// `pmGrassMap` (Xbox PDB): four `NiTPointerMap`s (0x10 bytes each).
        0x54 pmGrassMap: Ptr,
        /// `spLandRB` (Xbox PDB): `NiPointer<bhkRigidBody>` slot.
        0x94 spLandRB: Ptr,
        /// `iCellX` (Xbox PDB).
        0x98 iCellX: i32,
        /// `iCellY` (Xbox PDB).
        0x9C iCellY: i32,
        /// `fBaseHeight` (Xbox PDB): the mid height of the land; vertex
        /// heights are stored relative to it.
        0xA0 fBaseHeight: f32,
    }
}

/// The offset of the `TESChildCell` sub-object in the land.
pub(crate) const CHILD_CELL_OFFSET: u32 = 0x18;
/// The offset of the `pmGrassMap` maps (0x10 bytes each, four).
pub(crate) const GRASS_MAP_OFFSET: u32 = 0x54;
/// Quadrants of a land and the vertices of a quadrant (17 x 17).
pub(crate) const QUADRANTS: u32 = 4;
pub(crate) const QUADRANT_VERTICES: u32 = 0x121;
/// Vertices of a chunk's grid (33 x 33).
const CHUNK_VERTICES: usize = 0x441;

/// The four-character tag of a land chunk as the record reader compares it
/// (the bytes in file order read as a little-endian word).
pub(crate) const fn tag(name: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*name)
}

pub(crate) const CHUNK_DATA: u32 = tag(b"DATA");
pub(crate) const CHUNK_HEIGHTS: u32 = tag(b"VHGT");
pub(crate) const CHUNK_NORMALS: u32 = tag(b"VNML");
pub(crate) const CHUNK_COLORS: u32 = tag(b"VCLR");
pub(crate) const CHUNK_MOPP: u32 = tag(b"MPCD");
pub(crate) const CHUNK_BASE_TEXTURE: u32 = tag(b"BTXT");
pub(crate) const CHUNK_ADDITIONAL_TEXTURE: u32 = tag(b"ATXT");
pub(crate) const CHUNK_VERTEX_TEXTURE: u32 = tag(b"VTXT");

/// The land's flag bits (see [`TESObjectLAND::Data`]).
pub(crate) const FLAG_HEIGHTS: u32 = 0x1;
pub(crate) const FLAG_COLORS: u32 = 0x2;
pub(crate) const FLAG_TEXTURES: u32 = 0x4;
pub(crate) const FLAG_EDITED: u32 = 0x10;
pub(crate) const FLAG_FROM_WORLD_SPACE: u32 = 0x400;
pub(crate) const FLAG_MOPP_BUILT: u32 = 0x800;

// ---- Helpers ---------------------------------------------------------------

/// `this->pLoadedData`.
pub(crate) fn loaded_data(e: &Engine, this: Ptr<TESObjectLAND>) -> Ptr<LoadedLandData> {
    e.get(this, TESObjectLAND::pLoadedData).cast()
}

/// Element `index` of an array of pointers.
pub(crate) fn element(e: &Engine, array: Ptr, index: u32) -> u32 {
    e.mem.u32(array.addr().wrapping_add(index.wrapping_mul(4)))
}

/// Reads `count` words at `at`.
fn read_words(e: &Engine, at: u32, count: u32) -> Vec<u32> {
    (0..count)
        .map(|i| e.mem.u32(at.wrapping_add(4 * i)))
        .collect()
}

/// Writes `words` at `at`.
fn write_words(e: &mut Engine, at: u32, words: &[u32]) {
    for (i, word) in words.iter().enumerate() {
        e.mem.set_u32(at.wrapping_add(4 * i as u32), *word);
    }
}

/// Copies `count` words from `from` to `to`.
fn copy_words(e: &mut Engine, from: u32, to: u32, count: u32) {
    let words = read_words(e, from, count);
    write_words(e, to, &words);
}

/// Runs a constructor of a vector type on a temporary of `size` bytes and
/// returns the words of the value it returns a pointer to.
fn construct_words(e: &mut Engine, construct: u32, size: u32, arguments: &[u32]) -> Vec<u32> {
    e.with_stack(size, |e, temporary| {
        let mut words = vec![temporary.addr()];
        words.extend_from_slice(arguments);
        let result = e.call(construct, &words).u32();
        read_words(e, result, size / 4)
    })
}

fn ni_point3(e: &mut Engine, x: f32, y: f32, z: f32) -> Vec<u32> {
    construct_words(e, NI_POINT3_CONSTRUCT, 12, &args![x, y, z])
}

fn ni_point2(e: &mut Engine, x: f32, y: f32) -> Vec<u32> {
    construct_words(e, NI_POINT2_CONSTRUCT, 8, &args![x, y])
}

fn ni_color(e: &mut Engine, r: f32, g: f32, b: f32, a: f32) -> Vec<u32> {
    construct_words(e, NI_COLOR_CONSTRUCT, 16, &args![r, g, b, a])
}

/// The allocation-tag scope guard the loaders run under: entered with the
/// source line, left when `body` returns.
fn scoped<R>(e: &mut Engine, line: u32, body: impl FnOnce(&mut Engine) -> R) -> R {
    e.with_stack(GUARD_SIZE, |e, guard| {
        e.call(
            SCOPE_GUARD_ENTER,
            &args![guard, GUARD_TAG, 1u32, SOURCE_FILE, line],
        );
        let result = body(e);
        e.call(SCOPE_GUARD_LEAVE, &args![guard]);
        result
    })
}

/// A vector of `count` elements built by the vector constructor iterator
/// when the allocation succeeded (0 otherwise).
fn allocate_vector(
    e: &mut Engine,
    size: u32,
    element_size: u32,
    count: u32,
    construct: u32,
) -> u32 {
    let memory = e.call(ALLOCATE, &args![size]).u32();
    if memory != 0 {
        e.call(
            VECTOR_CONSTRUCT,
            &args![memory, element_size, count, construct],
        );
    }
    memory
}

/// A `__RTDynamicCast` from `TESForm`.
fn form_cast(e: &mut Engine, form: u32, target: u32) -> u32 {
    e.call(
        RT_DYNAMIC_CAST,
        &args![form, 0u32, RTTI_TES_FORM, target, 0u32],
    )
    .u32()
}

/// The cell coordinates of the land `(x, y)`, evaluated y first as the
/// message calls do.
fn cell_coordinates(e: &mut Engine, this: Ptr<TESObjectLAND>) -> (i32, i32) {
    let y = fn_00534010(e, this);
    let x = fn_00533fd0(e, this);
    (x, y)
}

// ---- Constructor, destructors and statics ---------------------------------

// Translated from 00533120 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::TESObjectLAND` (Xbox PDB): builds the form and the
/// `TESChildCell` part, stores the two vtables, constructs the queued
/// textures slot, defines the statics when no land has yet, clears the
/// flags, the cell and the loaded data, sets the form type to 0x42 and
/// counts the land in `iLANDsinuse`. Returns `this`.
pub fn tes_object_land_tes_object_land(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
) -> Ptr<TESObjectLAND> {
    e.call(FORM_CONSTRUCT, &args![this]);
    fn_00533240(e, this.byte_add(CHILD_CELL_OFFSET));
    e.mem.set_u32(this.addr(), LAND_VTABLE);
    e.mem
        .set_u32(this.addr() + CHILD_CELL_OFFSET, CHILD_CELL_VTABLE);
    e.call(
        NI_POINTER_SLOT_CONSTRUCT,
        &args![this.byte_add(TESObjectLAND::spQueuedTextures.off), 0u32],
    );
    if e.global::<u8>(STATICS_DEFINED) == 0 {
        tes_object_land_initialize_statics(e);
    }
    e.call(
        MEMORY_SET,
        &args![this.byte_add(TESObjectLAND::Data.off), 0u32, 4u32],
    );
    e.set(this, TESObjectLAND::pParentCell, Ptr::NULL);
    e.call(FORM_SET_FORM_TYPE, &args![this, 0x42u32]);
    e.set(this, TESObjectLAND::pLoadedData, Ptr::NULL);
    let in_use = e.global::<u32>(LANDS_IN_USE).wrapping_add(1);
    e.set_global(LANDS_IN_USE, in_use);
    this
}

// Translated from 005331f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESChildCell::GetSaveParentCell` for a land (slot 0 of the vtable
/// `0102dccc`; `this` is the `TESChildCell` part at +0x18): the land's
/// parent cell.
pub fn fn_005331f0(e: &mut Engine, this: Ptr) -> Ptr {
    let land = this.addr().wrapping_sub(CHILD_CELL_OFFSET);
    e.call(PARENT_CELL, &args![land]).ptr()
}

// Translated from 00533210 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::__vecDelDtor` (Xbox PDB, "scalar deleting destructor",
/// slot 0x10): runs the destructor and frees the land when bit 0 of `flags`
/// is set. Returns `this`.
pub fn tes_object_land_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
    flags: u32,
) -> Ptr<TESObjectLAND> {
    tes_object_land_destructor(e, this);
    if flags & 1 != 0 {
        e.call(DEALLOCATE, &args![this]);
    }
    this
}

// Translated from 00533240 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESChildCell::TESChildCell`: stores the vtable of the abstract class.
/// Returns `this`.
pub fn fn_00533240(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), CHILD_CELL_ABSTRACT_VTABLE);
    this
}

// Translated from 00533260 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::~TESObjectLAND` (Xbox PDB): resets the vtables, unloads
/// the vertices, and when this was the last land frees the shared blocks
/// (triangle list, colours, normals, texture coordinates, normal set and
/// the four default vertex blocks), clears `bStaticsDefined`, releases the
/// default texturing property and deletes the default land texture and
/// texture set. Then destroys the queued textures slot and the form.
pub fn tes_object_land_destructor(e: &mut Engine, this: Ptr<TESObjectLAND>) {
    e.mem.set_u32(this.addr(), LAND_VTABLE);
    e.mem
        .set_u32(this.addr() + CHILD_CELL_OFFSET, CHILD_CELL_VTABLE);
    tes_object_land_un_load_vertices(e, this);
    let in_use = e.global::<u32>(LANDS_IN_USE).wrapping_sub(1);
    e.set_global(LANDS_IN_USE, in_use);
    if in_use == 0 {
        for block in [
            DEFAULT_TRIANGLE_LIST,
            DEFAULT_COLORS,
            DEFAULT_NORMALS,
            DEFAULT_TEXTURE_COORDINATES,
            DEFAULT_NORMAL_SET,
        ] {
            let memory = e.global::<u32>(block);
            e.call(DEALLOCATE, &args![memory]);
        }
        for quadrant in 0..QUADRANTS {
            let memory = e.global::<u32>(DEFAULT_VERTEX_BLOCKS + 4 * quadrant);
            e.call(DEALLOCATE, &args![memory]);
        }
        e.set_global(STATICS_DEFINED, 0u8);
        e.call(
            TEXTURING_PROPERTY_SET,
            &args![DEFAULT_TEXTURING_PROPERTY, 0u32],
        );
        for slot in [DEFAULT_LAND_TEXTURE, DEFAULT_TEXTURE_SET] {
            let object = e.global::<u32>(slot);
            if object != 0 {
                // The virtual deleting destructor (slot 4).
                e.vcall(object, 0x10, &args![1u32]);
            }
            e.set_global(slot, 0u32);
        }
    }
    e.call(FORM_BASE_DESTRUCT, &args![this]);
    e.call(
        QUEUED_FILE_POINTER_DESTRUCT,
        &args![this.byte_add(TESObjectLAND::spQueuedTextures.off)],
    );
    e.call(FORM_DESTRUCT, &args![this]);
}

// Translated from 00533420 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::InitializeStatics` (Xbox PDB): builds what all lands
/// share. The default texture set with the two Landscape textures from the
/// INI settings, the default land texture, the default texturing property
/// (diffuse map, mode 2) and the triangle list of one quadrant (16 x 16
/// cells of two triangles, the diagonal alternating with the parity of the
/// cell), then 0x121-entry blocks of white colours (alpha 0), up normals,
/// "set" flags and texture coordinates (scaled by `4 /
/// fLandTextureTilingMult`, 16 when the setting is 0), the offsets of the
/// four quadrants (+-0x800) and four default vertex blocks at height -2048.
pub fn tes_object_land_initialize_statics(e: &mut Engine) {
    scoped(e, 0xc6, |e| {
        // The default texture set and its two texture paths.
        let memory = e.call(NI_ALLOCATE_OBJECT, &args![0xa0u32]).u32();
        let texture_set = if memory == 0 {
            0
        } else {
            e.call(TEXTURE_SET_CONSTRUCT, &args![memory]).u32()
        };
        e.set_global(DEFAULT_TEXTURE_SET, texture_set);
        e.with_stack(0x104, |e, path| {
            for (index, setting) in [
                (0u32, DEFAULT_DIFFUSE_TEXTURE_SETTING),
                (1u32, DEFAULT_NORMAL_TEXTURE_SETTING),
            ] {
                let name = e.call(SETTING_GET_STRING, &args![setting]).u32();
                e.call(
                    FORMAT_STRING,
                    &args![path, 0x104u32, LANDSCAPE_PATH_FORMAT, name],
                );
                let texture_set = e.global::<u32>(DEFAULT_TEXTURE_SET);
                e.call(TEXTURE_SET_SET_PATH, &args![texture_set, index, path]);
            }
        });
        let texture_set = e.global::<u32>(DEFAULT_TEXTURE_SET);
        let menu_manager = e.global::<u32>(MENU_MANAGER);
        let result = e
            .call(
                MENU_MANAGER_REGISTER_TEXTURE_SET,
                &args![menu_manager, texture_set],
            )
            .u32();
        e.call(MENU_MANAGER_AFTER_REGISTER, &args![result]);

        // The default land texture.
        let memory = e.call(ALLOCATE, &args![0x28u32]).u32();
        let land_texture = if memory == 0 {
            0
        } else {
            e.call(LAND_TEXTURE_CONSTRUCT, &args![memory]).u32()
        };
        e.set_global(DEFAULT_LAND_TEXTURE, land_texture);
        let texture_set = e.global::<u32>(DEFAULT_TEXTURE_SET);
        e.call(
            LAND_TEXTURE_SET_TEXTURE_SET,
            &args![land_texture, texture_set],
        );

        // The diffuse texture of the set (slot 0x90 of the base at +0x30)
        // goes into a smart pointer that lives until the end of the function.
        let diffuse = e.mem.alloc(4);
        e.call(SMART_POINTER_CONSTRUCT, &args![diffuse, 0u32]);
        let texture_set = e.global::<u32>(DEFAULT_TEXTURE_SET);
        e.vcall(texture_set + 0x30, 0x90, &args![0u32, diffuse]);
        let memory = e.call(ALLOCATE, &args![0x10u32]).u32();
        let map = if memory == 0 {
            0
        } else {
            let texture = e.call(SMART_POINTER_GET, &args![diffuse]).u32();
            e.call(
                TEXTURE_MAP_CONSTRUCT,
                &args![memory, texture, 0u32, 3u32, 5u32, 0u32],
            )
            .u32()
        };
        let memory = e.call(NI_ALLOCATE_OBJECT, &args![0x30u32]).u32();
        let property = if memory == 0 {
            0
        } else {
            e.call(TEXTURING_PROPERTY_CONSTRUCT, &args![memory]).u32()
        };
        e.call(
            TEXTURING_PROPERTY_SET,
            &args![DEFAULT_TEXTURING_PROPERTY, property],
        );
        let property = e
            .call(SMART_POINTER_GET, &args![DEFAULT_TEXTURING_PROPERTY])
            .u32();
        fn_00533fb0(e, Ptr::new(property), 2);
        let property = e
            .call(SMART_POINTER_GET, &args![DEFAULT_TEXTURING_PROPERTY])
            .u32();
        fn_00533f40(e, Ptr::new(property), Ptr::new(map));

        // The triangle list of a quadrant: 16 x 16 cells, six indices each.
        let triangles = e.call(ALLOCATE, &args![0xc00u32]).u32();
        e.set_global(DEFAULT_TRIANGLE_LIST, triangles);
        let mut count = 0u32;
        for row in 0..16u32 {
            for column in 0..16u32 {
                let next = (row + 1) * 17;
                let current = row * 17;
                let indices = if row % 2 == column % 2 {
                    [
                        next + column + 1,
                        next + column,
                        current + column,
                        current + column,
                        current + column + 1,
                        next + column + 1,
                    ]
                } else {
                    [
                        next + column,
                        current + column,
                        current + column + 1,
                        current + column + 1,
                        next + column + 1,
                        next + column,
                    ]
                };
                for index in indices {
                    e.mem.set_u16(triangles + 2 * count, index as u16);
                    count += 1;
                }
            }
        }

        // The texture coordinate scale.
        let setting = e
            .call(SETTING_GET_FLOAT_ADDRESS, &args![TEXTURE_TILING_SETTING])
            .u32();
        let mut tiling = e.mem.f32(setting);
        let zero: f64 = e.global(F64_ZERO);
        if tiling as f64 == zero {
            e.call(LOG_MESSAGE, &args![TILING_WARNING]);
            tiling = e.global(SIXTEEN_F32);
        }
        let four: f64 = e.global(F64_FOUR);
        let scale = (four / tiling as f64) as f32;

        // The blocks of 0x121 colours, normals, flags and coordinates.
        let colors = allocate_vector(
            e,
            0x1210,
            0x10,
            QUADRANT_VERTICES,
            NI_COLOR_DEFAULT_CONSTRUCT,
        );
        e.set_global(DEFAULT_COLORS, colors);
        let normals = allocate_vector(e, 0xd8c, 12, QUADRANT_VERTICES, NI_POINT3_DEFAULT_CONSTRUCT);
        e.set_global(DEFAULT_NORMALS, normals);
        let flags = e.call(ALLOCATE, &args![0x121u32]).u32();
        e.set_global(DEFAULT_NORMAL_SET, flags);
        let coordinates =
            allocate_vector(e, 0x908, 8, QUADRANT_VERTICES, NI_POINT3_DEFAULT_CONSTRUCT);
        e.set_global(DEFAULT_TEXTURE_COORDINATES, coordinates);
        let mut index = 0u32;
        for row in 0..17u32 {
            for column in 0..17u32 {
                let color = ni_color(e, 1.0, 1.0, 1.0, 0.0);
                write_words(e, colors + 0x10 * index, &color);
                let normal = ni_point3(e, 0.0, 0.0, 1.0);
                write_words(e, normals + 12 * index, &normal);
                e.mem.set_u8(flags + index, 1);
                let u = (column as f64 / scale as f64) as f32;
                let v = (row as f64 / scale as f64) as f32;
                let coordinate = ni_point2(e, u, v);
                write_words(e, coordinates + 8 * index, &coordinate);
                index += 1;
            }
        }

        // The offsets of the four quadrants and the default vertex blocks.
        for quadrant in 0..QUADRANTS {
            let x = ((quadrant % 2) << 11) as i32 - 0x800;
            e.set_global(X_OFFSETS + 4 * quadrant, x as f32);
            let y = ((quadrant / 2) << 11) as i32 - 0x800;
            e.set_global(Y_OFFSETS + 4 * quadrant, y as f32);
        }
        for quadrant in 0..QUADRANTS {
            let block =
                allocate_vector(e, 0xd8c, 12, QUADRANT_VERTICES, NI_POINT3_DEFAULT_CONSTRUCT);
            e.set_global(DEFAULT_VERTEX_BLOCKS + 4 * quadrant, block);
            let mut index = 0u32;
            for row in 0..17u32 {
                for column in 0..17u32 {
                    let z: f32 = e.global(F32_MINUS_2048);
                    let y_offset: f32 = e.global(Y_OFFSETS + 4 * quadrant);
                    let y = ((row << 7) as f32) + y_offset;
                    let x_offset: f32 = e.global(X_OFFSETS + 4 * quadrant);
                    let x = ((column << 7) as f32) + x_offset;
                    let point = ni_point3(e, x, y, z);
                    write_words(e, block + 12 * index, &point);
                    index += 1;
                }
            }
        }
        e.set_global(STATICS_DEFINED, 1u8);
        e.call(SMART_POINTER_RELEASE, &args![diffuse]);
        e.mem.free(diffuse);
    })
}

// Translated from 00533f40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the map at slot 0 of the texturing property's table (`this + 0x1c`):
/// when it differs from the current one, deletes the current map (virtual
/// destructor, slot 0) and stores the new one.
pub fn fn_00533f40(e: &mut Engine, this: Ptr, map: Ptr) {
    let table = this.byte_add(0x1c);
    let slot = e.call(ARRAY_ELEMENT_ADDRESS, &args![table, 0u32]).u32();
    let current = e.mem.u32(slot);
    if map.addr() != current {
        if current != 0 {
            e.vcall(current, 0, &args![1u32]);
        }
        e.with_stack(4, |e, value| {
            e.mem.set_u32(value.addr(), map.addr());
            e.call(ARRAY_SET_AT, &args![table, 0u32, value]);
        });
    }
}

// Translated from 00533fb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the 16-bit `value` of a texturing property through `00439360(value,
/// 0xe, 1)`.
pub fn fn_00533fb0(e: &mut Engine, this: Ptr, value: u16) {
    e.call(PROPERTY_SET_MODE, &args![this, value as u32, 0xeu32, 1u32]);
}

// ---- Cell accessors and flags ---------------------------------------------

// Translated from 00533fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The land's cell X: from the loaded data when there is any, else from the
/// parent cell, else 0.
pub fn fn_00533fd0(e: &mut Engine, this: Ptr<TESObjectLAND>) -> i32 {
    let data = loaded_data(e, this);
    if !data.is_null() {
        return e.get(data, LoadedLandData::iCellX);
    }
    let cell = e.get(this, TESObjectLAND::pParentCell);
    if cell.is_null() {
        return 0;
    }
    e.call(CELL_GET_DATA_X, &args![cell]).i32()
}

// Translated from 00534010 (decompiled, FalloutNV.exe 1.4.0.525)
/// The land's cell Y, like [`fn_00533fd0`].
pub fn fn_00534010(e: &mut Engine, this: Ptr<TESObjectLAND>) -> i32 {
    let data = loaded_data(e, this);
    if !data.is_null() {
        return e.get(data, LoadedLandData::iCellY);
    }
    let cell = e.get(this, TESObjectLAND::pParentCell);
    if cell.is_null() {
        return 0;
    }
    e.call(CELL_GET_DATA_Y, &args![cell]).i32()
}

// Translated from 00534050 (decompiled, FalloutNV.exe 1.4.0.525)
/// The world X of the land's cell corner: the cell X times 4096, returned
/// in `ST0` (the `FILD` of an integer, so an exact `f64`).
pub fn fn_00534050(e: &mut Engine, this: Ptr<TESObjectLAND>) -> f64 {
    let x = fn_00533fd0(e, this);
    x.wrapping_shl(12) as f64
}

// Translated from 00534080 (decompiled, FalloutNV.exe 1.4.0.525)
/// The world Y of the land's cell corner, like [`fn_00534050`].
pub fn fn_00534080(e: &mut Engine, this: Ptr<TESObjectLAND>) -> f64 {
    let y = fn_00534010(e, this);
    y.wrapping_shl(12) as f64
}

// Translated from 005340b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The world space of the parent cell, or null without a cell.
pub fn fn_005340b0(e: &mut Engine, this: Ptr<TESObjectLAND>) -> Ptr {
    let cell = e.get(this, TESObjectLAND::pParentCell);
    if cell.is_null() {
        return Ptr::NULL;
    }
    e.call(CELL_GET_WORLD_SPACE, &args![cell]).ptr()
}

// Translated from 005340e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::SetCell` (Xbox PDB): stores the parent cell; for a cell
/// whose world space has land data (`00586390(0)`) sets flag 0x400 (the data
/// comes from the world space's land file), otherwise clears it.
pub fn tes_object_land_set_cell(e: &mut Engine, this: Ptr<TESObjectLAND>, cell: Ptr) {
    e.set(this, TESObjectLAND::pParentCell, cell);
    if cell.is_null() {
        return;
    }
    let world_space = e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32();
    if world_space != 0
        && e.call(WORLD_SPACE_HAS_LAND, &args![world_space, 0u32])
            .u32()
            != 0
    {
        fn_00534160(e, this, true);
        return;
    }
    fn_00534160(e, this, false);
}

/// Sets or clears `bit` in the land's flags.
fn set_flag(e: &mut Engine, this: Ptr<TESObjectLAND>, bit: u32, on: bool) {
    let flags = e.get(this, TESObjectLAND::Data);
    let flags = if on { flags | bit } else { flags & !bit };
    e.set(this, TESObjectLAND::Data, flags);
}

fn flag(e: &Engine, this: Ptr<TESObjectLAND>, bit: u32) -> bool {
    e.get(this, TESObjectLAND::Data) & bit != 0
}

// Translated from 00534140 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the land's data comes from the world space's land file (flag
/// 0x400).
pub fn fn_00534140(e: &mut Engine, this: Ptr<TESObjectLAND>) -> bool {
    flag(e, this, FLAG_FROM_WORLD_SPACE)
}

// Translated from 00534160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears flag 0x400.
pub fn fn_00534160(e: &mut Engine, this: Ptr<TESObjectLAND>, on: bool) {
    set_flag(e, this, FLAG_FROM_WORLD_SPACE, on);
}

// Translated from 005341a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears flag 0x800 (the Havok MOPP code was built).
pub fn fn_005341a0(e: &mut Engine, this: Ptr<TESObjectLAND>, on: bool) {
    set_flag(e, this, FLAG_MOPP_BUILT, on);
}

// Translated from 005341e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether flag 0x10 is set.
pub fn fn_005341e0(e: &mut Engine, this: Ptr<TESObjectLAND>) -> bool {
    flag(e, this, FLAG_EDITED)
}

// Translated from 00534200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears flag 0x10.
pub fn fn_00534200(e: &mut Engine, this: Ptr<TESObjectLAND>, on: bool) {
    set_flag(e, this, FLAG_EDITED, on);
}

// ---- Vertex, normal and colour readers --------------------------------------

/// The default `NiPoint3` of a reader written to `out`.
fn write_point3(e: &mut Engine, out: Ptr, x: f32, y: f32, z: f32) {
    let point = ni_point3(e, x, y, z);
    write_words(e, out.addr(), &point);
}

/// The default colour of the colour reader: the four floats at `011a9be0`.
fn write_default_color(e: &mut Engine, out: Ptr) {
    copy_words(e, DEFAULT_COLOR_WORDS, out.addr(), 4);
}

/// The vertex arrays exist (`0045cb90`: loaded data with `ppVertices`).
fn has_vertex_arrays(e: &mut Engine, this: Ptr<TESObjectLAND>) -> bool {
    e.call(LAND_HAS_VERTEX_ARRAYS, &args![this]).bool()
}

// Translated from 00534240 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the position of vertex `index` of quadrant `block` into `out` (an
/// `NiPoint3`), adding the land's base height to Z. Without vertex arrays
/// the result is `(0, 0, default height)` and no base height is added. The
/// vertex comes from `ppVertices` when that block exists, else from the
/// mesh node's geometry of the quadrant, else it is `(0, 0, default
/// height)` plus the base height.
pub fn fn_00534240(e: &mut Engine, this: Ptr<TESObjectLAND>, block: u32, index: i32, out: Ptr) {
    if !has_vertex_arrays(e, this) {
        let height = e.call(LAND_DEFAULT_HEIGHT, &args![this]).f32();
        let point = ni_point3(e, 0.0, 0.0, height);
        write_words(e, out.addr(), &point);
        return;
    }
    let data = loaded_data(e, this);
    let offset = (index as u32).wrapping_mul(12);
    let vertices = e.get(data, LoadedLandData::ppVertices);
    let direct = if vertices.is_null() {
        0
    } else {
        element(e, vertices, block)
    };
    if direct != 0 {
        copy_words(e, direct.wrapping_add(offset), out.addr(), 3);
    } else {
        let meshes = e.get(data, LoadedLandData::ppMesh);
        let mesh = if meshes.is_null() {
            0
        } else {
            element(e, meshes, block)
        };
        if mesh != 0 {
            let geometry = e.call(GEOMETRY_GET_MODEL_DATA, &args![mesh, 0u32]).u32();
            let positions = e.call(GEOMETRY_GET_POSITIONS, &args![geometry]).u32();
            copy_words(e, positions.wrapping_add(offset), out.addr(), 3);
        } else {
            let height = e.call(LAND_DEFAULT_HEIGHT, &args![this]).f32();
            let point = ni_point3(e, 0.0, 0.0, height);
            write_words(e, out.addr(), &point);
        }
    }
    let base_height = e.get(data, LoadedLandData::fBaseHeight);
    let z = e.mem.f32(out.addr() + 8);
    e.mem.set_f32(out.addr() + 8, z + base_height);
}

// Translated from 00534390 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the normal of vertex `index` of quadrant `block` into `out`: from
/// `ppNormals`, else from the normal array of the quadrant's mesh geometry,
/// else (when the geometry has no normal stream) locks the geometry data
/// and reads the packed vertex through [`fn_00534570`]. When nothing gives
/// one, and without vertex arrays, the normal is `(0, 0, 1)`.
pub fn fn_00534390(e: &mut Engine, this: Ptr<TESObjectLAND>, block: u32, index: i32, out: Ptr) {
    if !has_vertex_arrays(e, this) {
        write_point3(e, out, 0.0, 0.0, 1.0);
        return;
    }
    let data = loaded_data(e, this);
    let offset = (index as u32).wrapping_mul(12);
    let mut found = false;
    let normals = e.get(data, LoadedLandData::ppNormals);
    let direct = if normals.is_null() {
        0
    } else {
        element(e, normals, block)
    };
    if direct != 0 {
        copy_words(e, direct.wrapping_add(offset), out.addr(), 3);
        found = true;
    } else {
        let meshes = e.get(data, LoadedLandData::ppMesh);
        let mesh = if meshes.is_null() {
            0
        } else {
            element(e, meshes, block)
        };
        if mesh != 0 {
            let geometry = e.call(GEOMETRY_GET_MODEL_DATA, &args![mesh, 0u32]).u32();
            if e.call(GEOMETRY_GET_NORMALS, &args![geometry]).u32() != 0 {
                let normals = e.call(GEOMETRY_GET_NORMALS, &args![geometry]).u32();
                copy_words(e, normals.wrapping_add(offset), out.addr(), 3);
                found = true;
            } else {
                let geometry_data = e.call(GEOMETRY_GET_DATA, &args![geometry]).u32();
                found = e.with_stack(12, |e, lock| {
                    fn_00534550(e, lock);
                    let owner = e.call(GEOMETRY_DATA_GET_OWNER, &args![geometry_data]).u32();
                    if owner != 0
                        && e.vcall(owner, OWNER_CAN_LOCK_SLOT, &[]).bool()
                        && e.call(GEOMETRY_DATA_BEGIN, &args![geometry_data, 1u32])
                            .bool()
                    {
                        e.call(GEOMETRY_DATA_LOCK_POSITIONS, &args![geometry_data, lock]);
                        fn_00534570(e, lock, index, out);
                        e.call(GEOMETRY_DATA_END, &args![geometry_data]);
                        true
                    } else {
                        false
                    }
                });
            }
        }
    }
    if !found {
        write_point3(e, out, 0.0, 0.0, 1.0);
    }
}

// Translated from 00534550 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructs a vertex lock (`{base, stride, packed}` zeroed) and returns
/// it.
pub fn fn_00534550(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(LOCK_CONSTRUCT, &args![this]);
    this
}

// Translated from 00534570 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the three floats of vertex `index` of a locked vertex stream
/// (`this` = `{base, stride, packed}`) into `out`.
pub fn fn_00534570(e: &mut Engine, this: Ptr, index: i32, out: Ptr) {
    e.call(LOCK_CHECK_INDEX, &args![this, index]);
    let base = e.mem.u32(this.addr());
    let stride = e.mem.u32(this.addr() + 4);
    let vertex = (index as u32).wrapping_mul(stride).wrapping_add(base);
    copy_words(e, vertex, out.addr(), 3);
}

// Translated from 005345c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the colour of vertex `index` of quadrant `block` into `out` (an
/// `NiColorA`): from `ppColorsA`, else from the colour array of the
/// quadrant's mesh geometry, else from its locked colour stream through
/// [`fn_00534780`]. Without vertex arrays, or when nothing gives one, the
/// colour is the exe's default (four floats at `011a9be0`).
pub fn fn_005345c0(e: &mut Engine, this: Ptr<TESObjectLAND>, block: u32, index: i32, out: Ptr) {
    if !has_vertex_arrays(e, this) {
        write_default_color(e, out);
        return;
    }
    let data = loaded_data(e, this);
    let mut found = false;
    let colors = e.get(data, LoadedLandData::ppColorsA);
    let direct = if colors.is_null() {
        0
    } else {
        element(e, colors, block)
    };
    if direct != 0 {
        copy_words(e, direct.wrapping_add((index as u32) << 4), out.addr(), 4);
        found = true;
    } else {
        let meshes = e.get(data, LoadedLandData::ppMesh);
        let mesh = if meshes.is_null() {
            0
        } else {
            element(e, meshes, block)
        };
        if mesh != 0 {
            let geometry = e.call(GEOMETRY_GET_MODEL_DATA, &args![mesh, 0u32]).u32();
            if e.call(GEOMETRY_GET_NORMALS, &args![geometry]).u32() != 0 {
                let colors = e.call(GEOMETRY_GET_COLORS, &args![geometry]).u32();
                copy_words(e, colors.wrapping_add((index as u32) << 4), out.addr(), 4);
                found = true;
            } else {
                let geometry_data = e.call(GEOMETRY_GET_DATA, &args![geometry]).u32();
                found = e.with_stack(12, |e, lock| {
                    fn_00534550(e, lock);
                    let owner = e.call(GEOMETRY_DATA_GET_OWNER, &args![geometry_data]).u32();
                    if owner != 0
                        && e.vcall(owner, OWNER_CAN_LOCK_SLOT, &[]).bool()
                        && e.call(GEOMETRY_DATA_BEGIN, &args![geometry_data, 1u32])
                            .bool()
                    {
                        e.call(GEOMETRY_DATA_LOCK_COLORS, &args![geometry_data, lock]);
                        fn_00534780(e, lock, index, out);
                        e.call(GEOMETRY_DATA_END, &args![geometry_data]);
                        true
                    } else {
                        false
                    }
                });
            }
        }
    }
    if !found {
        write_default_color(e, out);
    }
}

// Translated from 00534780 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the colour of vertex `index` of a locked colour stream (`this` =
/// `{base, stride, packed}`) into `out`. A packed stream holds one 32-bit
/// `0xAARRGGBB` word per vertex, which becomes `(r, g, b, a) / 255`;
/// otherwise the four floats are copied.
pub fn fn_00534780(e: &mut Engine, this: Ptr, index: i32, out: Ptr) {
    e.call(LOCK_CHECK_INDEX, &args![this, index]);
    let base = e.mem.u32(this.addr());
    let stride = e.mem.u32(this.addr() + 4);
    let vertex = (index as u32).wrapping_mul(stride).wrapping_add(base);
    if e.mem.u8(this.addr() + 8) != 0 {
        let word = e.mem.u32(vertex);
        let red = (word >> 16) & 0xff;
        let green = (word >> 8) & 0xff;
        let blue = word & 0xff;
        let alpha = (word >> 24) & 0xff;
        let divisor: f64 = e.global(F64_255);
        for (i, channel) in [red, green, blue, alpha].into_iter().enumerate() {
            e.mem
                .set_f32(out.addr() + 4 * i as u32, (channel as f64 / divisor) as f32);
        }
    } else {
        copy_words(e, vertex, out.addr(), 4);
    }
}

// Translated from 00534880 (decompiled, FalloutNV.exe 1.4.0.525)
/// Looks the form `id` up (`004867a0(0, id)`) and casts it to
/// `TESObjectLAND`; the first stack word is not read.
pub fn fn_00534880(e: &mut Engine, this: Ptr, _unused_1: u32, id: u32) -> Ptr {
    let form = e.call(FORM_LOOKUP, &args![this, 0u32, id]).u32();
    e.call(
        RT_DYNAMIC_CAST,
        &args![form, 0u32, RTTI_TES_FORM, RTTI_TES_OBJECT_LAND, 0u32],
    )
    .ptr()
}

// Translated from 005348c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies another land's data into this one: casts `form` to
/// `TESObjectLAND` (nothing happens when it is not one), loads the source's
/// vertices when they are not loaded, takes its parent cell, allocates this
/// land's data as a copy, and copies the quadrants' vertices (0xd8c bytes),
/// normals (0xd8c), colours (0x1210), normal-set flags (0x121), percent
/// block (0x2420), layer textures (0x18) and base texture, then the flags
/// and the height extents. The source's vertices are unloaded again if this
/// loaded them.
pub fn fn_005348c0(e: &mut Engine, this: Ptr<TESObjectLAND>, form: Ptr) {
    let source = form_cast_from(e, form.addr(), RTTI_TES_OBJECT_LAND);
    if source == 0 {
        return;
    }
    let source = Ptr::<TESObjectLAND>::new(source);
    let was_loaded = e.call(LAND_IS_LOADED, &args![source]).bool();
    if !was_loaded {
        tes_object_land_load_vertices(e, source, false);
    }
    let cell = e.call(PARENT_CELL, &args![source]).ptr::<()>();
    e.set(this, TESObjectLAND::pParentCell, cell);
    e.call(LAND_ALLOCATE_DATA, &args![this, source]);
    let to = loaded_data(e, this);
    let from = loaded_data(e, source);
    for quadrant in 0..QUADRANTS {
        for (field, size) in [
            (LoadedLandData::ppVertices.off, 0xd8cu32),
            (LoadedLandData::ppNormals.off, 0xd8c),
            (LoadedLandData::ppColorsA.off, 0x1210),
            (LoadedLandData::ppNormalsSet.off, 0x121),
        ] {
            let destination = e.mem.u32(to.addr() + field);
            let source_block = e.mem.u32(from.addr() + field);
            let destination = e.mem.u32(destination + 4 * quadrant);
            let source_block = e.mem.u32(source_block + 4 * quadrant);
            e.call(MEMORY_COPY, &args![destination, source_block, size]);
        }
        let destination = e
            .mem
            .u32(to.addr() + LoadedLandData::ppPercentArrays.off + 4 * quadrant);
        let source_block = e
            .mem
            .u32(from.addr() + LoadedLandData::ppPercentArrays.off + 4 * quadrant);
        let destination = e.mem.u32(destination);
        let source_block = e.mem.u32(source_block);
        e.call(MEMORY_COPY, &args![destination, source_block, 0x2420u32]);
        let destination = e
            .mem
            .u32(to.addr() + LoadedLandData::pQuadTextureArray.off + 4 * quadrant);
        let source_block = e
            .mem
            .u32(from.addr() + LoadedLandData::pQuadTextureArray.off + 4 * quadrant);
        e.call(MEMORY_COPY, &args![destination, source_block, 0x18u32]);
        let base_texture = e
            .mem
            .u32(from.addr() + LoadedLandData::pDefQuadTexture.off + 4 * quadrant);
        e.mem.set_u32(
            to.addr() + LoadedLandData::pDefQuadTexture.off + 4 * quadrant,
            base_texture,
        );
    }
    e.call(
        MEMORY_COPY,
        &args![
            this.byte_add(TESObjectLAND::Data.off),
            source.byte_add(TESObjectLAND::Data.off),
            4u32
        ],
    );
    e.call(
        MEMORY_COPY,
        &args![
            to.byte_add(LoadedLandData::HeightExtentsMin.off),
            from.byte_add(LoadedLandData::HeightExtentsMin.off),
            8u32
        ],
    );
    if !was_loaded {
        tes_object_land_un_load_vertices(e, source);
    }
    e.call(LAND_SET_LOADED, &args![this, 1u32]);
}

/// A `__RTDynamicCast` of `object` from `TESForm` to `target`.
fn form_cast_from(e: &mut Engine, object: u32, target: u32) -> u32 {
    form_cast(e, object, target)
}

// ---- Chunk readers shared by Load and LoadVerticesIntoArrays ---------------

/// The messages the texture chunks log; the two loaders have their own
/// ("MASTERFILE: ..." for `Load`, "TEXTURES: ..." for
/// `LoadVerticesIntoArrays`).
struct TextureMessages {
    /// `Land (%i, %i) clamping invalid index %i for block %i.`
    clamped_layer: u32,
    /// `Land (%i, %i) unable to find additional texture ID (%08X) for block
    /// %i.`
    missing_additional: u32,
    /// `Land (%i, %i) unable to find base texture ID (%08X) for block %i.`
    missing_base: u32,
    /// `Land (%i, %i) found unrecognized vertex texture data in file %s.`
    unrecognized_data: u32,
}

const MASTER_FILE_MESSAGES: TextureMessages = TextureMessages {
    clamped_layer: 0x0102_df48,
    missing_additional: 0x0102_def0,
    missing_base: 0x0102_df90,
    unrecognized_data: 0x0102_dea0,
};

const CELL_MESSAGES: TextureMessages = TextureMessages {
    clamped_layer: 0x0102_e0dc,
    missing_additional: 0x0102_e088,
    missing_base: 0x0102_e120,
    unrecognized_data: 0x0102_e038,
};

/// The state the chunk loop of a loader keeps: whether a message was logged
/// (so the warnings get flushed at the end), and the quadrant and layer of
/// the last `ATXT` chunk (`-1` when none), which the `VTXT` chunk that
/// follows belongs to.
struct ChunkState {
    warned: bool,
    quadrant: i32,
    layer: i32,
}

impl ChunkState {
    fn new() -> Self {
        ChunkState {
            warned: false,
            quadrant: -1,
            layer: -1,
        }
    }
}

fn file_needs_swap(e: &mut Engine, file: Ptr) -> bool {
    e.call(FILE_NEEDS_SWAP, &args![file]).bool()
}

/// The first index of quadrant `quadrant`'s 17 x 17 block in a 33 x 33
/// chunk grid, and the grid index of its vertex `k`.
fn quadrant_origin(quadrant: u32) -> u32 {
    (quadrant / 2) * 0x10 * 0x21 + (quadrant % 2) * 0x10
}

fn grid_index(origin: u32, k: u32) -> u32 {
    (k / 0x11) * 0x21 + origin + k % 0x11
}

/// The `VHGT` chunk at `buffer` (a float base height and 0x441 signed byte
/// steps; each step adds to the running height, which restarts from the
/// first height of the row at the end of a row): resets the height
/// extents, builds the vertices of the four quadrants (x and y from the
/// quadrant offsets and the grid position, z = the height rounded to an
/// integer, times 8), keeping the lowest and highest, sets the base height
/// to their mean and makes the heights relative to it.
fn read_height_chunk(e: &mut Engine, this: Ptr<TESObjectLAND>, buffer: u32) {
    let data = loaded_data(e, this);
    let float_max: f32 = e.global(LARGEST_F32);
    let negative_float_max: f32 = e.global(LOWEST_F32);
    let extents = ni_point2(e, float_max, negative_float_max);
    e.mem.set_u32(
        data.addr() + LoadedLandData::HeightExtentsMin.off,
        extents[0],
    );
    e.mem.set_u32(
        data.addr() + LoadedLandData::HeightExtentsMax.off,
        extents[1],
    );
    let mut offset = e.mem.f32(buffer);
    let mut heights = vec![0f32; CHUNK_VERTICES];
    for i in 0..CHUNK_VERTICES {
        let step = e.mem.i8(buffer + 4 + i as u32);
        heights[i] = step as f32 + offset;
        offset = if (i + 1) % 0x21 == 0 {
            heights[i - 0x20]
        } else {
            heights[i]
        };
    }
    let vertices = e.get(data, LoadedLandData::ppVertices);
    for quadrant in 0..QUADRANTS {
        let origin = quadrant_origin(quadrant);
        for k in 0..QUADRANT_VERTICES {
            let sample = grid_index(origin, k) as usize;
            let rounded = e.call(FLOAT_TO_INT, &args![heights[sample]]).i32();
            let z = rounded.wrapping_shl(3) as f32;
            let x_offset: f32 = e.global(X_OFFSETS + 4 * quadrant);
            let x = ((k % 0x11) << 7) as f32 + x_offset;
            let y_offset: f32 = e.global(Y_OFFSETS + 4 * quadrant);
            let y = ((k / 0x11) << 7) as f32 + y_offset;
            let vertex = element(e, vertices, quadrant) + 12 * k;
            e.mem.set_f32(vertex, x);
            e.mem.set_f32(vertex + 4, y);
            e.mem.set_f32(vertex + 8, z);
            let lowest = e.get(data, LoadedLandData::HeightExtentsMin);
            if z < lowest {
                e.set(data, LoadedLandData::HeightExtentsMin, z);
            } else {
                let highest = e.get(data, LoadedLandData::HeightExtentsMax);
                if highest < z {
                    e.set(data, LoadedLandData::HeightExtentsMax, z);
                }
            }
        }
    }
    let lowest = e.get(data, LoadedLandData::HeightExtentsMin);
    let highest = e.get(data, LoadedLandData::HeightExtentsMax);
    let half: f64 = e.global(F64_HALF);
    let base_height = ((lowest as f64 + highest as f64) * half) as f32;
    e.set(data, LoadedLandData::fBaseHeight, base_height);
    for quadrant in 0..QUADRANTS {
        for k in 0..QUADRANT_VERTICES {
            let vertex = element(e, vertices, quadrant) + 12 * k;
            let base_height = e.get(data, LoadedLandData::fBaseHeight);
            let z = e.mem.f32(vertex + 8);
            e.mem.set_f32(vertex + 8, z - base_height);
        }
    }
}

/// The `VNML` chunk at `buffer` (33 x 33 signed byte normals, three bytes
/// each): the normals of the four quadrants, each component divided by 127,
/// then normalized.
fn read_normal_chunk(e: &mut Engine, this: Ptr<TESObjectLAND>, buffer: u32) {
    let data = loaded_data(e, this);
    let normals = e.get(data, LoadedLandData::ppNormals);
    for quadrant in 0..QUADRANTS {
        let origin = quadrant_origin(quadrant);
        for k in 0..QUADRANT_VERTICES {
            let sample = buffer + grid_index(origin, k) * 3;
            let normal = element(e, normals, quadrant) + 12 * k;
            for component in 0..3u32 {
                let divisor: f64 = e.global(F64_127);
                let value = e.mem.i8(sample + component) as f64 / divisor;
                e.mem.set_f32(normal + 4 * component, value as f32);
            }
            e.call(NI_POINT3_NORMALIZE, &args![normal]);
        }
    }
}

/// The `VCLR` chunk at `buffer` (33 x 33 colours of three bytes): the
/// colours of the four quadrants, each byte divided by 255, alpha 1.
fn read_color_chunk(e: &mut Engine, this: Ptr<TESObjectLAND>, buffer: u32) {
    let data = loaded_data(e, this);
    let colors = e.get(data, LoadedLandData::ppColorsA);
    for quadrant in 0..QUADRANTS {
        let origin = quadrant_origin(quadrant);
        for k in 0..QUADRANT_VERTICES {
            let sample = buffer + grid_index(origin, k) * 3;
            let divisor: f64 = e.global(F64_255);
            let r = (e.mem.u8(sample) as f64 / divisor) as f32;
            let g = (e.mem.u8(sample + 1) as f64 / divisor) as f32;
            let b = (e.mem.u8(sample + 2) as f64 / divisor) as f32;
            let color = ni_color(e, r, g, b, 1.0);
            let at = element(e, colors, quadrant) + 0x10 * k;
            write_words(e, at, &color);
        }
    }
}

/// Queues the texture for loading: creates the land's `QueuedFile` first
/// when it has none.
fn queue_texture(e: &mut Engine, this: Ptr<TESObjectLAND>, texture: u32) {
    let slot = this.byte_add(TESObjectLAND::spQueuedTextures.off);
    if e.call(SMART_POINTER_GET, &args![slot]).u32() == 0 {
        let memory = e.call(ALLOCATE, &args![0x28u32]).u32();
        let queued = if memory == 0 {
            0
        } else {
            e.call(QUEUED_FILE_CONSTRUCT, &args![memory, QUEUE_PRIORITY])
                .u32()
        };
        e.call(QUEUED_FILE_POINTER_SET, &args![slot, queued]);
    }
    let queued = e.call(SMART_POINTER_GET, &args![slot]).u32();
    e.call(LAND_TEXTURE_QUEUE, &args![texture, QUEUE_PRIORITY, queued]);
}

/// The `ATXT` chunk (an 8-byte record: texture form id, quadrant byte, a
/// byte, layer word): looks the `TESLandTexture` up (the default texture
/// for id 0), stores it in the quadrant's layer (layers above 5 are
/// clamped to 5 with a message), queues it, and remembers quadrant and
/// layer for the `VTXT` chunk.
fn read_additional_texture_chunk(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
    file: Ptr,
    messages: &TextureMessages,
    state: &mut ChunkState,
) {
    e.with_stack(8, |e, record| {
        let record = record.addr();
        e.call(MEMORY_SET, &args![record, 0u32, 8u32]);
        e.call(FILE_GET_CHUNK_DATA, &args![file, record, 8u32]);
        if file_needs_swap(e, file) {
            fn_00535a60(e, Ptr::new(record));
        }
        let quadrant = e.mem.u8(record + 4) as u32;
        if e.mem.u16(record + 6) > 5 {
            let layer = e.mem.u16(record + 6) as u32;
            let (x, y) = cell_coordinates(e, this);
            e.call(
                LOG_MESSAGE,
                &args![messages.clamped_layer, x, y, layer, quadrant],
            );
            e.mem.set_u16(record + 6, 5);
            state.warned = true;
        }
        let texture = if e.mem.u32(record) == 0 {
            fn_00535ae0(e).addr()
        } else {
            e.call(FORM_ADD_COMPILE_INDEX, &args![record, file]);
            let id = e.mem.u32(record);
            let form = e.call(FORM_LOOKUP_BY_ID, &args![id]).u32();
            let texture = form_cast(e, form, RTTI_TES_LAND_TEXTURE);
            if texture == 0 {
                let id = e.mem.u32(record);
                let (x, y) = cell_coordinates(e, this);
                e.call(
                    LOG_MESSAGE,
                    &args![messages.missing_additional, x, y, id, quadrant],
                );
                state.warned = true;
            }
            texture
        };
        let data = loaded_data(e, this);
        let layers = e
            .mem
            .u32(data.addr() + LoadedLandData::pQuadTextureArray.off + 4 * quadrant);
        let layer = e.mem.u16(record + 6) as u32;
        e.mem.set_u32(layers.wrapping_add(4 * layer), texture);
        if texture != 0 {
            queue_texture(e, this, texture);
        }
        state.quadrant = quadrant as i32;
        state.layer = layer as i32;
    });
}

/// The `BTXT` chunk (an 8-byte record: texture form id, quadrant byte):
/// looks the `TESLandTexture` up and makes it the quadrant's base texture
/// (a message when it is not found), queues it.
fn read_base_texture_chunk(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
    file: Ptr,
    messages: &TextureMessages,
    state: &mut ChunkState,
) {
    e.with_stack(8, |e, record| {
        let record = record.addr();
        e.call(MEMORY_SET, &args![record, 0u32, 8u32]);
        e.call(FILE_GET_CHUNK_DATA, &args![file, record, 8u32]);
        if file_needs_swap(e, file) {
            fn_00535a60(e, Ptr::new(record));
        }
        e.call(FORM_ADD_COMPILE_INDEX, &args![record, file]);
        let id = e.mem.u32(record);
        let form = e.call(FORM_LOOKUP_BY_ID, &args![id]).u32();
        let texture = form_cast(e, form, RTTI_TES_LAND_TEXTURE);
        let quadrant = e.mem.u8(record + 4) as u32;
        let data = loaded_data(e, this);
        let slot = data.addr() + LoadedLandData::pDefQuadTexture.off + 4 * quadrant;
        e.mem.set_u32(slot, texture);
        if e.mem.u32(slot) == 0 {
            let id = e.mem.u32(record);
            let (x, y) = cell_coordinates(e, this);
            e.call(
                LOG_MESSAGE,
                &args![messages.missing_base, x, y, id, quadrant],
            );
            state.warned = true;
        } else {
            let texture = e.mem.u32(slot);
            queue_texture(e, this, texture);
        }
    });
}

/// The `VTXT` chunk (8-byte entries: position word, two bytes, opacity
/// float) of the layer the last `ATXT` chunk set up: hands every entry to
/// `SetVertexOpacity` (`0053a8a0`). `Load` divides an opacity above 1 by
/// 100 first (`scale_opacity`). A size that is not a multiple of 8 logs a
/// message. Quadrant and layer are reset afterwards.
fn read_vertex_texture_chunk(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
    file: Ptr,
    messages: &TextureMessages,
    scale_opacity: bool,
    state: &mut ChunkState,
) {
    if state.quadrant >= 0 && state.layer >= 0 {
        let size = e.call(FILE_GET_CHUNK_SIZE, &args![file]).u32();
        if size % 8 == 0 {
            let entries = e.call(ALLOCATE, &args![size]).u32();
            e.call(FILE_GET_CHUNK_DATA, &args![file, entries, size]);
            for i in 0..size >> 3 {
                let entry = entries.wrapping_add(8 * i);
                if file_needs_swap(e, file) {
                    fn_00535aa0(e, Ptr::new(entry));
                }
                if scale_opacity {
                    let one: f64 = e.global(F64_ONE);
                    let opacity = e.mem.f32(entry + 4);
                    if opacity as f64 > one {
                        let hundred: f64 = e.global(F64_HUNDRED);
                        e.mem.set_f32(entry + 4, (opacity as f64 / hundred) as f32);
                    }
                }
                let position = e.mem.u16(entry) as u32;
                let opacity = e.mem.f32(entry + 4);
                e.call(
                    LAND_SET_VERTEX_OPACITY,
                    &args![
                        this,
                        state.quadrant as u32 & 0xff,
                        position,
                        state.layer as u32 & 0xffff,
                        opacity
                    ],
                );
            }
            e.call(DEALLOCATE, &args![entries]);
        } else {
            let name = e.call(FILE_GET_NAME, &args![file]).u32();
            let (x, y) = cell_coordinates(e, this);
            e.call(LOG_MESSAGE, &args![messages.unrecognized_data, x, y, name]);
            state.warned = true;
        }
    }
    state.quadrant = -1;
    state.layer = -1;
}

/// The `MPCD` chunk: the Havok MOPP code. Replaces the land's code (the old
/// one is released), has it built from the chunk (`00538c00`) and, when
/// that gave one, takes a reference and sets flag 0x800.
fn read_mopp_chunk(e: &mut Engine, this: Ptr<TESObjectLAND>, file: Ptr) {
    let size = e.call(FILE_GET_CHUNK_SIZE, &args![file]).u32();
    let code = e.call(ALLOCATE, &args![size]).u32();
    e.call(FILE_GET_CHUNK_DATA, &args![file, code, size]);
    let data = loaded_data(e, this);
    let old = e.get(data, LoadedLandData::pMoppCode);
    if !old.is_null() {
        e.call(HAVOK_REMOVE_REFERENCE, &args![old]);
        e.set(data, LoadedLandData::pMoppCode, Ptr::NULL);
    }
    e.call(
        LAND_BUILD_MOPP,
        &args![
            this,
            code,
            size,
            data.byte_add(LoadedLandData::pMoppCode.off)
        ],
    );
    let new = e.get(data, LoadedLandData::pMoppCode);
    if !new.is_null() {
        e.call(HAVOK_ADD_REFERENCE, &args![new]);
        fn_005341a0(e, this, true);
    }
    e.call(DEALLOCATE, &args![code]);
}

// ---- Loading ---------------------------------------------------------------

// Translated from 00534ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::Load` (Xbox PDB): loads the land record from `file`.
/// Returns false when the record is not a land record (type `B`). The
/// geometry chunks are only read when the file is the active one or a
/// master is being loaded, and the data does not come from the world
/// space's land file; then the loaded data is allocated first. `DATA` sets
/// the flags (keeping the world space bit), `VHGT`, `VNML` and `VCLR` build
/// the vertices, normals and colours of the quadrants, `MPCD` the Havok
/// code, `BTXT`, `ATXT` and `VTXT` the textures. The warnings are flushed
/// if any message was logged, and the "vertices loaded" state follows
/// whether the geometry was read.
///
/// The cell's id read at the start (`0084e3a0`) is not used.
pub fn tes_object_land_load(e: &mut Engine, this: Ptr<TESObjectLAND>, file: Ptr) -> bool {
    scoped(e, 0x2c2, |e| {
        if e.call(FILE_GET_TES_FORM, &args![file]).u8() != b'B' {
            return false;
        }
        e.call(FORM_ID, &args![this]);
        e.call(FORM_LOAD, &args![this, file]);
        let mut state = ChunkState::new();
        let active = e.call(FILE_GET_ACTIVE, &args![file]).bool();
        let load_geometry =
            (active || e.call(LOADING_MASTER_FILE, &[]).bool()) && !fn_00534140(e, this);
        if load_geometry {
            e.call(LAND_ALLOCATE_DATA, &args![this, 0u32]);
        }
        loop {
            let chunk = e.call(FILE_GET_CHUNK, &args![file]).u32();
            match chunk {
                CHUNK_DATA => {
                    let from_world_space = fn_00534140(e, this);
                    e.call(
                        FILE_GET_CHUNK_DATA,
                        &args![file, this.byte_add(TESObjectLAND::Data.off), 4u32],
                    );
                    if file_needs_swap(e, file) {
                        e.call(SWAP_WORD, &args![this.byte_add(TESObjectLAND::Data.off)]);
                    }
                    e.call(LAND_SET_LOADED, &args![this, 0u32]);
                    fn_00534160(e, this, from_world_space);
                }
                CHUNK_HEIGHTS => {
                    if load_geometry && flag(e, this, FLAG_HEIGHTS) {
                        e.with_stack(0x448, |e, buffer| {
                            e.call(FILE_GET_CHUNK_DATA, &args![file, buffer, 0u32]);
                            if file_needs_swap(e, file) {
                                e.call(SWAP_WORD, &args![buffer]);
                            }
                            read_height_chunk(e, this, buffer.addr());
                        });
                    }
                }
                CHUNK_NORMALS => {
                    if load_geometry && flag(e, this, FLAG_HEIGHTS) {
                        e.with_stack(0xcc3, |e, buffer| {
                            e.call(FILE_GET_CHUNK_DATA, &args![file, buffer, 0u32]);
                            read_normal_chunk(e, this, buffer.addr());
                        });
                    }
                }
                CHUNK_MOPP => {
                    if load_geometry {
                        read_mopp_chunk(e, this, file);
                    }
                }
                CHUNK_COLORS => {
                    if load_geometry && flag(e, this, FLAG_COLORS) {
                        e.with_stack(0xcc3, |e, buffer| {
                            e.call(FILE_GET_CHUNK_DATA, &args![file, buffer, 0u32]);
                            read_color_chunk(e, this, buffer.addr());
                        });
                    }
                }
                CHUNK_ADDITIONAL_TEXTURE => {
                    if load_geometry {
                        read_additional_texture_chunk(
                            e,
                            this,
                            file,
                            &MASTER_FILE_MESSAGES,
                            &mut state,
                        );
                    }
                }
                CHUNK_BASE_TEXTURE => {
                    if load_geometry {
                        read_base_texture_chunk(e, this, file, &MASTER_FILE_MESSAGES, &mut state);
                    }
                }
                CHUNK_VERTEX_TEXTURE if load_geometry => {
                    read_vertex_texture_chunk(
                        e,
                        this,
                        file,
                        &MASTER_FILE_MESSAGES,
                        true,
                        &mut state,
                    );
                }
                _ => {}
            }
            if !e.call(FILE_NEXT_CHUNK, &args![file]).bool() {
                break;
            }
        }
        if state.warned {
            e.call(LAND_FLUSH_WARNINGS, &args![this]);
        }
        e.call(LAND_SET_LOADED, &args![this, load_geometry as u32]);
        true
    })
}

// Translated from 00535a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Byte swaps an `ATXT`/`BTXT` record: the form id (a 32-bit word at +0)
/// and the layer (a 16-bit word at +6).
pub fn fn_00535a60(e: &mut Engine, this: Ptr) {
    e.call(SWAP_BYTES_32, &args![this, 0u32]);
    e.call(SWAP_BYTES_16, &args![this.byte_add(6), 0u32]);
}

// Translated from 00535aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Byte swaps a `VTXT` entry: the position (a 16-bit word at +0) and the
/// opacity (a 32-bit word at +4).
pub fn fn_00535aa0(e: &mut Engine, this: Ptr) {
    e.call(SWAP_BYTES_16, &args![this, 0u32]);
    e.call(SWAP_BYTES_32, &args![this.byte_add(4), 0u32]);
}

// Translated from 00535ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::pDefText` (Xbox PDB): the default land texture.
pub fn fn_00535ae0(e: &mut Engine) -> Ptr {
    Ptr::new(e.global::<u32>(DEFAULT_LAND_TEXTURE))
}

// Translated from 00535af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scene node of quadrant `index` (0 to 3) of the parent cell's 3D
/// (`0045c9a0`), or null without a cell or for an index of 4 or more.
pub fn fn_00535af0(e: &mut Engine, this: Ptr<TESObjectLAND>, index: i32) -> Ptr {
    let cell = e.get(this, TESObjectLAND::pParentCell);
    if !cell.is_null() && index < 4 {
        return e.call(CELL_GET_NODE, &args![cell, index]).ptr();
    }
    Ptr::NULL
}

// Translated from 00535b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::GetWorldOffsetForBlock` (Xbox PDB): writes the world
/// position of the land's centre into `out` (an `NiPoint3`): the cell
/// corner plus 2048 in x and y, and the base height. The block number is
/// not used. Returns `out`.
pub fn tes_object_land_get_world_offset_for_block(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
    out: Ptr,
    _unused_2: u32,
) -> Ptr {
    e.with_stack(12, |e, local| {
        e.call(NI_POINT3_DEFAULT_CONSTRUCT, &args![local]);
        let offset: f64 = e.global(F64_2048);
        let x = (fn_00534050(e, this) + offset) as f32;
        e.mem.set_f32(local.addr(), x);
        let offset: f64 = e.global(F64_2048);
        let y = (fn_00534080(e, this) + offset) as f32;
        e.mem.set_f32(local.addr() + 4, y);
        let z = e.call(LAND_BASE_HEIGHT, &args![this]).f32();
        e.mem.set_f32(local.addr() + 8, z);
        copy_words(e, local.addr(), out.addr(), 3);
    });
    out
}

// Translated from 00535b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::LoadVertices` (Xbox PDB): makes sure the land's vertex
/// arrays are loaded, and builds the quadrant meshes (`005374f0`) when
/// `build_meshes` is set (never while the loading menu is up). A land whose
/// vertices are already loaded only builds the meshes it lacks. Otherwise the
/// loaded data is allocated, and filled by `LoadVerticesIntoArrays` when the
/// land has a file to read it from (the world space's land data, or a
/// record with heights, colours or textures that belongs to a file).
/// Returns that function's result, true when nothing needed reading.
pub fn tes_object_land_load_vertices(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
    build_meshes: bool,
) -> bool {
    scoped(e, 0x455, |e| {
        let build_meshes = if e.call(LOADING_MENU_VISIBLE, &[]).bool() {
            false
        } else {
            build_meshes
        };
        if e.call(LAND_IS_LOADED, &args![this]).bool() {
            let data = loaded_data(e, this);
            if e.get(data, LoadedLandData::ppMesh).is_null() && build_meshes {
                fn_005374f0(e, this);
            }
            return true;
        }
        if !fn_00534140(e, this) {
            let flags = e.get(this, TESObjectLAND::Data);
            let has_file = flags & (FLAG_HEIGHTS | FLAG_COLORS | FLAG_TEXTURES) != 0
                && e.call(FORM_GET_FILE, &args![this, 0xffff_ffffu32]).u32() != 0;
            if !has_file {
                e.call(LAND_ALLOCATE_DATA, &args![this, 0u32]);
                if build_meshes {
                    fn_005374f0(e, this);
                }
                return true;
            }
        }
        e.call(LAND_ALLOCATE_DATA, &args![this, 0u32]);
        let loaded = tes_object_land_load_vertices_into_arrays(e, this);
        e.call(LAND_SET_LOADED, &args![this, 1u32]);
        if build_meshes {
            fn_005374f0(e, this);
        }
        loaded
    })
}

// Translated from 00535d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::LoadVerticesIntoArrays` (Xbox PDB): reads the land's
/// chunks from its plugin file into the (already allocated) loaded data.
/// The file and the record offset come from the form (found in the file) or,
/// for a land of the world space's land file, from `FindLandDataInFile`;
/// without them the function returns true, with the form missing from its
/// file, or not a land record, it logs "Failed to load landscape data" and
/// returns false (warnings are switched off around the message). Chunks are
/// gated by the land's flags (heights and normals 1, colours 2, textures 4)
/// instead of a "load" flag; `DATA` only merges the low three flag bits and
/// `VTXT` does not scale the opacity. Logs "does not contain Normal Data"
/// when heights came without normals. Returns true.
pub fn tes_object_land_load_vertices_into_arrays(e: &mut Engine, this: Ptr<TESObjectLAND>) -> bool {
    if e.call(LAND_IS_LOADED, &args![this]).bool() {
        return true;
    }
    let mut state = ChunkState::new();
    let mut failed = false;
    let file;
    let offset;
    if !fn_00534140(e, this) {
        let flags = e.get(this, TESObjectLAND::Data);
        if flags & (FLAG_HEIGHTS | FLAG_COLORS | FLAG_TEXTURES) == 0 {
            return true;
        }
        let form_file = e.call(FORM_GET_FILE, &args![this, 0xffff_ffffu32]).u32();
        file = e.call(FILE_GET_THREAD_SAFE_FILE, &args![form_file]).u32();
        if !e.call(FILE_FIND_FORM, &args![file, this]).bool() {
            failed = true;
        }
        if !failed && e.call(FILE_GET_TES_FORM, &args![file]).u8() != b'B' {
            failed = true;
        }
        offset = if failed {
            0
        } else {
            e.call(FILE_RECORD_OFFSET, &args![file]).u32()
        };
    } else {
        let world_space = fn_005340b0(e, this);
        if world_space.is_null() {
            return true;
        }
        let found = e.with_stack(8, |e, found_at| {
            let y = fn_00534010(e, this);
            let x = fn_00533fd0(e, this);
            let found = e
                .call(
                    WORLD_SPACE_FIND_LAND_DATA,
                    &args![world_space, x, y, found_at, found_at.byte_add(4)],
                )
                .bool();
            (
                found,
                e.mem.u32(found_at.addr()),
                e.mem.u32(found_at.addr() + 4),
            )
        });
        if !found.0 {
            return true;
        }
        file = found.1;
        offset = found.2;
    }
    if failed {
        e.call(DISABLE_WARNINGS, &args![0u32]);
        let name = if file == 0 {
            UNKNOWN_FILE_NAME
        } else {
            e.call(FILE_GET_NAME, &args![file]).u32()
        };
        let cell = e.call(PARENT_CELL, &args![this]).u32();
        let y = e.call(CELL_GET_DATA_Y, &args![cell]).u32();
        let cell = e.call(PARENT_CELL, &args![this]).u32();
        let x = e.call(CELL_GET_DATA_X, &args![cell]).u32();
        let id = e.call(FORM_ID, &args![this]).u32();
        e.call(LOG_MESSAGE, &args![LAND_LOAD_FAILED, id, x, y, name]);
        e.call(DISABLE_WARNINGS, &args![1u32]);
        return false;
    }
    let file = Ptr::new(file);
    if !e.call(FILE_SET_OFFSET, &args![file, offset]).bool() {
        return false;
    }
    let mut got_heights = false;
    let mut got_normals = false;
    loop {
        let chunk = e.call(FILE_GET_CHUNK, &args![file]).u32();
        match chunk {
            CHUNK_DATA => {
                if fn_00534140(e, this) {
                    e.with_stack(4, |e, value| {
                        e.call(FILE_GET_CHUNK_DATA, &args![file, value, 4u32]);
                        if file_needs_swap(e, file) {
                            e.call(SWAP_WORD, &args![value]);
                        }
                        let low_bits = e.mem.u32(value.addr()) & 7;
                        e.mem.set_u32(value.addr(), low_bits);
                        let flags = e.get(this, TESObjectLAND::Data) & !7;
                        e.set(this, TESObjectLAND::Data, flags);
                        let flags = e.get(this, TESObjectLAND::Data) | low_bits;
                        e.set(this, TESObjectLAND::Data, flags);
                    });
                }
            }
            CHUNK_MOPP => read_mopp_chunk(e, this, file),
            CHUNK_HEIGHTS => {
                if flag(e, this, FLAG_HEIGHTS) {
                    e.with_stack(0x448, |e, buffer| {
                        e.call(FILE_GET_CHUNK_DATA, &args![file, buffer, 0x448u32]);
                        if file_needs_swap(e, file) {
                            e.call(SWAP_WORD, &args![buffer]);
                        }
                        read_height_chunk(e, this, buffer.addr());
                    });
                    got_heights = true;
                }
            }
            CHUNK_NORMALS => {
                if flag(e, this, FLAG_HEIGHTS) {
                    e.with_stack(0xcc3, |e, buffer| {
                        e.call(FILE_GET_CHUNK_DATA, &args![file, buffer, 0xcc3u32]);
                        read_normal_chunk(e, this, buffer.addr());
                    });
                    got_normals = true;
                }
            }
            CHUNK_COLORS => {
                if flag(e, this, FLAG_COLORS) {
                    e.with_stack(0xcc3, |e, buffer| {
                        e.call(FILE_GET_CHUNK_DATA, &args![file, buffer, 0xcc3u32]);
                        read_color_chunk(e, this, buffer.addr());
                    });
                }
            }
            CHUNK_ADDITIONAL_TEXTURE => {
                if flag(e, this, FLAG_TEXTURES) {
                    read_additional_texture_chunk(e, this, file, &CELL_MESSAGES, &mut state);
                }
            }
            CHUNK_BASE_TEXTURE => {
                if flag(e, this, FLAG_TEXTURES) {
                    read_base_texture_chunk(e, this, file, &CELL_MESSAGES, &mut state);
                }
            }
            CHUNK_VERTEX_TEXTURE if flag(e, this, FLAG_TEXTURES) => {
                read_vertex_texture_chunk(e, this, file, &CELL_MESSAGES, false, &mut state);
            }
            _ => {}
        }
        if !e.call(FILE_NEXT_CHUNK, &args![file]).bool() {
            break;
        }
    }
    if state.warned {
        e.call(LAND_FLUSH_WARNINGS, &args![this]);
    }
    if got_heights && !got_normals {
        let name = e.call(FILE_GET_NAME, &args![file]).u32();
        let (x, y) = cell_coordinates(e, this);
        e.call(LOG_MESSAGE, &args![NO_NORMAL_DATA, x, y, name]);
    }
    true
}

// ---- Unloading and meshes ----------------------------------------------------

// Translated from 00536d80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::UnLoadVertices` (Xbox PDB): marks the vertices as not
/// loaded, releases the border lines, and frees the loaded data. For each
/// quadrant it frees the percent block (and its first array), the layer
/// textures, the vertex, normal, colour and normal-set blocks, and empties
/// the grass map (every item holds 16 pointers, each zeroed in its first
/// word and freed; the item is freed too). Then it frees the four block
/// arrays, releases the MOPP code, and deletes the loaded data. Returns
/// true.
pub fn tes_object_land_un_load_vertices(e: &mut Engine, this: Ptr<TESObjectLAND>) -> bool {
    e.call(LAND_SET_LOADED, &args![this, 0u32]);
    e.call(LAND_RELEASE_BORDER, &args![this]);
    let data = loaded_data(e, this);
    if !data.is_null() {
        let at = data.addr();
        // The iteration state the function keeps in locals: position, key
        // and value of the grass map walk.
        e.with_stack(12, |e, frame| {
            let position_cell = frame.addr();
            let key_cell = frame.addr() + 4;
            let value_cell = frame.addr() + 8;
            for quadrant in 0..QUADRANTS {
                let percent_slot = at + LoadedLandData::ppPercentArrays.off + 4 * quadrant;
                let percent = e.mem.u32(percent_slot);
                if percent != 0 {
                    let first = e.mem.u32(percent);
                    e.call(DEALLOCATE, &args![first]);
                    e.call(DEALLOCATE, &args![percent]);
                    e.mem.set_u32(percent_slot, 0);
                }
                let layers_slot = at + LoadedLandData::pQuadTextureArray.off + 4 * quadrant;
                let layers = e.mem.u32(layers_slot);
                if layers != 0 {
                    e.call(DEALLOCATE, &args![layers]);
                    e.mem.set_u32(layers_slot, 0);
                }
                for field in [
                    LoadedLandData::ppVertices.off,
                    LoadedLandData::ppNormals.off,
                    LoadedLandData::ppColorsA.off,
                    LoadedLandData::ppNormalsSet.off,
                ] {
                    let array = e.mem.u32(at + field);
                    if array != 0 {
                        let block = e.mem.u32(array + 4 * quadrant);
                        e.call(DEALLOCATE, &args![block]);
                        e.mem.set_u32(array + 4 * quadrant, 0);
                    }
                }
                let map = at + GRASS_MAP_OFFSET + 0x10 * quadrant;
                if e.call(FORM_ID, &args![map]).u32() > 0 {
                    let first = e.call(MAP_FIRST_POSITION, &args![map]).u32();
                    e.mem.set_u32(position_cell, first);
                    while e.mem.u32(position_cell) != 0 {
                        e.call(
                            MAP_GET_NEXT,
                            &args![map, position_cell, key_cell, value_cell],
                        );
                        let value = e.mem.u32(value_cell);
                        if value != 0 {
                            for item in 0..16u32 {
                                let pointer = e.mem.u32(value + 4 * item);
                                if pointer != 0 {
                                    e.mem.set_u32(pointer, 0);
                                    e.call(DEALLOCATE, &args![pointer]);
                                }
                            }
                            e.call(DEALLOCATE, &args![value]);
                        }
                    }
                }
                e.call(MAP_REMOVE_ALL, &args![map]);
            }
        });
        for field in [
            LoadedLandData::ppNormalsSet.off,
            LoadedLandData::ppVertices.off,
            LoadedLandData::ppColorsA.off,
            LoadedLandData::ppNormals.off,
        ] {
            let array = e.mem.u32(at + field);
            e.call(DEALLOCATE, &args![array]);
        }
        let mopp = e.get(data, LoadedLandData::pMoppCode);
        if !mopp.is_null() {
            e.call(HAVOK_REMOVE_REFERENCE, &args![mopp]);
            e.set(data, LoadedLandData::pMoppCode, Ptr::NULL);
        }
    }
    let data = loaded_data(e, this);
    if !data.is_null() {
        fn_00537100(e, data, 1);
    }
    e.set(this, TESObjectLAND::pLoadedData, Ptr::NULL);
    true
}

// Translated from 00537100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadedLandData`'s scalar deleting destructor: runs [`fn_00537130`] and
/// frees the block when bit 0 of `flags` is set. Returns `this`.
pub fn fn_00537100(e: &mut Engine, this: Ptr<LoadedLandData>, flags: u32) -> Ptr<LoadedLandData> {
    fn_00537130(e, this);
    if flags & 1 != 0 {
        e.call(DEALLOCATE, &args![this]);
    }
    this
}

// Translated from 00537130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadedLandData`'s destructor body: releases the land rigid body, runs
/// the destructors of the four grass maps (16 bytes each), releases the
/// border lines.
pub fn fn_00537130(e: &mut Engine, this: Ptr<LoadedLandData>) {
    e.call(
        SMART_POINTER_RELEASE,
        &args![this.byte_add(LoadedLandData::spLandRB.off)],
    );
    e.call(
        VECTOR_DESTRUCT_ITERATOR,
        &args![
            this.byte_add(GRASS_MAP_OFFSET),
            0x10u32,
            QUADRANTS,
            GRASS_MAP_DESTRUCT
        ],
    );
    e.call(
        SMART_POINTER_RELEASE,
        &args![this.byte_add(LoadedLandData::spBorder.off)],
    );
}

// Translated from 005371b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectLAND::MakeLandTriStrips` (Xbox PDB): builds the `NiTriStrips`
/// of quadrant `block` (0 to 3, with loaded data, else null is returned):
/// copies of the default texture coordinates and of the quadrant's normals,
/// a one-strip list of 1021 indices (the strip length word `0x3fd`, the
/// indices from `0118ae90`), the quadrant's vertices and colours; moves the
/// mesh to the land's world offset and names it `"Block (%i, %i)"` (block
/// mod 4, block / 4).
pub fn tes_object_land_make_land_tri_strips(
    e: &mut Engine,
    this: Ptr<TESObjectLAND>,
    block: u8,
) -> Ptr {
    let data = loaded_data(e, this);
    if block >= 4 || data.is_null() {
        return Ptr::NULL;
    }
    let block = block as u32;
    let coordinates = allocate_vector(e, 0x908, 8, QUADRANT_VERTICES, NI_POINT3_DEFAULT_CONSTRUCT);
    let default_coordinates = e.global::<u32>(DEFAULT_TEXTURE_COORDINATES);
    e.call(
        MEMORY_COPY,
        &args![coordinates, default_coordinates, 0x908u32],
    );
    let lengths = e.call(NI_ALLOCATE, &args![2u32]).u32();
    let indices = e.call(NI_ALLOCATE, &args![0x7fau32]).u32();
    e.mem.set_u16(lengths, 0x3fd);
    e.call(MEMORY_COPY, &args![indices, TRIANGLE_STRIP_LIST, 0x7fau32]);
    let normals = allocate_vector(e, 0xd8c, 12, QUADRANT_VERTICES, NI_POINT3_DEFAULT_CONSTRUCT);
    let source_normals = e.get(data, LoadedLandData::ppNormals);
    let source_normals = element(e, source_normals, block);
    e.call(MEMORY_COPY, &args![normals, source_normals, 0xd8cu32]);
    let memory = e.call(NI_ALLOCATE_OBJECT, &args![0xc4u32]).u32();
    let strips = if memory == 0 {
        0
    } else {
        let vertices = e.get(data, LoadedLandData::ppVertices);
        let vertices = element(e, vertices, block);
        let colors = e.get(data, LoadedLandData::ppColorsA);
        let colors = element(e, colors, block);
        let strip_length = e.mem.u16(lengths) as u32;
        e.call(
            TRI_STRIPS_CONSTRUCT,
            &args![
                memory,
                QUADRANT_VERTICES,
                vertices,
                normals,
                colors,
                coordinates,
                1u32,
                0u32,
                strip_length.wrapping_sub(2),
                1u32,
                lengths,
                indices
            ],
        )
        .u32()
    };
    e.with_stack(12, |e, offset| {
        tes_object_land_get_world_offset_for_block(e, this, offset, block);
        e.call(NODE_SET_TRANSLATE, &args![strips, offset]);
    });
    e.with_stack(0x104, |e, name| {
        e.call(
            FORMAT_STRING,
            &args![name, 0x104u32, BLOCK_NAME_FORMAT, block & 3, block >> 2],
        );
        e.with_stack(4, |e, fixed| {
            let handle = e.call(FIXED_STRING_CONSTRUCT, &args![fixed, name]).u32();
            e.call(OBJECT_SET_NAME, &args![strips, handle]);
            e.call(FIXED_STRING_DESTRUCT, &args![fixed]);
        });
    });
    Ptr::new(strips)
}

// Translated from 005374f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the four quadrant meshes of the land in its cell's 3D, when the
/// cell has one. Without meshes yet: for each quadrant it takes the cell's
/// node, culls it (`NiAVObject` flag bit 0, `APP_CULLED`), attaches the default texturing property, builds the
/// tri-strips ([`tes_object_land_make_land_tri_strips`]) and attaches them
/// (virtual slot `0xf8`, arguments `0` and the strips), computes the bound
/// from the vertices, marks the geometry data changed, and moves the
/// face-gen root node to the quadrant's centre (the cell corner plus 1024
/// plus the quadrant's 2048 step in x and y, the middle of its height
/// extents in z) after giving it the half height extents as its local
/// translation. Then it builds the Havok cell MOPP (`CellMopp::Create`)
/// from the four meshes and the base height, runs the three follow-up
/// builds, and for each mesh sets the data's consistency (0x4000) and
/// flags, queues the geometry with the renderer when its scheduler's saved
/// acquire object is 1 to 5, restores the node's culled flag, and updates
/// the node and its properties. With meshes: sets each node's culled flag:
/// to true for all four when the land has flag 0x400 and `00537b50(cell)` is
/// false, otherwise per quadrant to what `00544590(cell, quadrant)` says.
///
/// Locals the code constructs but never reads (the translate zero point, the
/// MOPP code pointer copy) are left out; the height extents it keeps in a
/// local are updated but not used.
pub fn fn_005374f0(e: &mut Engine, this: Ptr<TESObjectLAND>) {
    let cell = e.get(this, TESObjectLAND::pParentCell);
    if cell.is_null() || e.call(CELL_GET_3D, &args![cell]).u32() == 0 {
        return;
    }
    let data = loaded_data(e, this);
    if e.get(data, LoadedLandData::ppMesh).is_null() {
        let memory = e.call(ALLOCATE, &args![0x10u32]).ptr();
        e.set(data, LoadedLandData::ppMesh, memory);
        // Locals: the four hidden flags at +0 and the four strips at +0x10.
        e.with_stack(0x20, |e, frame| {
            let hidden = frame.addr();
            let strips = frame.addr() + 0x10;
            e.call(MEMORY_SET, &args![hidden, 0u32, 4u32]);
            e.call(MEMORY_SET, &args![strips, 0u32, 0x10u32]);
            let float_max: f32 = e.global(LARGEST_F32);
            let negative_float_max: f32 = e.global(LOWEST_F32);
            let extents = ni_point2(e, float_max, negative_float_max);
            let mut lowest = f32::from_bits(extents[0]);
            let mut highest = f32::from_bits(extents[1]);
            let meshes = e.get(data, LoadedLandData::ppMesh);
            for quadrant in 0..QUADRANTS {
                let node = fn_00535af0(e, this, quadrant as i32).addr();
                e.mem.set_u32(meshes.addr() + 4 * quadrant, node);
                if element(e, meshes, quadrant) == 0 {
                    continue;
                }
                let node = element(e, meshes, quadrant);
                let was_hidden = e.call(NODE_GET_CULLED, &args![node]).u8();
                e.mem.set_u8(hidden + quadrant, was_hidden);
                let node = element(e, meshes, quadrant);
                e.call(NODE_SET_CULLED, &args![node, 1u32]);
                let property = e
                    .call(SMART_POINTER_GET, &args![DEFAULT_TEXTURING_PROPERTY])
                    .u32();
                let node = element(e, meshes, quadrant);
                e.call(NODE_ATTACH_PROPERTY, &args![node, property]);
                let tri_strips =
                    tes_object_land_make_land_tri_strips(e, this, quadrant as u8).addr();
                e.mem.set_u32(strips + 4 * quadrant, tri_strips);
                let node = element(e, meshes, quadrant);
                e.vcall(node, NODE_ATTACH_CHILD_SLOT, &args![0u32, tri_strips]);
                let vertex_count = e.call(TRI_STRIPS_VERTEX_COUNT, &args![tri_strips]).u16();
                let positions = e.call(GEOMETRY_GET_POSITIONS, &args![tri_strips]).u32();
                let bound = e.call(LAND_BOUND_SOURCE, &args![tri_strips]).u32();
                e.call(
                    BOUND_COMPUTE_FROM_DATA,
                    &args![bound, vertex_count as u32, positions],
                );
                let geometry_data = e.call(GEOMETRY_GET_DATA, &args![tri_strips]).u32();
                e.call(GEOMETRY_DATA_MARK_CHANGED, &args![geometry_data, 0xfu32]);
                let node = element(e, meshes, quadrant);
                let animation = e.call(FACE_GEN_NODE_GET_ANIMATION_DATA, &args![node]).u32();
                let root = e.call(ANIMATION_DATA_GET_ROOT, &args![animation]).u32();
                let (low, high) = e.with_stack(8, |e, extents| {
                    e.call(LAND_QUADRANT_EXTENTS, &args![this, extents, quadrant]);
                    (e.mem.f32(extents.addr()), e.mem.f32(extents.addr() + 4))
                });
                let half_height = ((high as f64 - low as f64) / e.global::<f64>(F64_TWO)) as f32;
                let size: f32 = e.global(F32_1024);
                e.with_stack(12, |e, local| {
                    let point = e
                        .call(NI_POINT3_CONSTRUCT, &args![local, size, size, half_height])
                        .u32();
                    e.call(ROOT_SET_LOCAL_TRANSLATE, &args![root, point]);
                });
                let corner_x = fn_00534050(e, this) + e.global::<f64>(F64_1024);
                let x = (corner_x + ((quadrant % 2) << 11) as f64) as f32;
                let corner_y = fn_00534080(e, this) + e.global::<f64>(F64_1024);
                let y = (corner_y + ((quadrant / 2) << 11) as f64) as f32;
                let z = ((low as f64 + high as f64) / e.global::<f64>(F64_TWO)) as f32;
                e.with_stack(12, |e, translation| {
                    e.mem.set_f32(translation.addr(), x);
                    e.mem.set_f32(translation.addr() + 4, y);
                    e.mem.set_f32(translation.addr() + 8, z);
                    e.vcall(root, ROOT_SET_WORLD_TRANSLATE_SLOT, &args![translation]);
                });
                if low < lowest {
                    lowest = low;
                }
                if highest < high {
                    highest = high;
                }
            }
            let _ = (lowest, highest);
            let cell = e.get(this, TESObjectLAND::pParentCell);
            let cell_mopp = e.call(LAND_CELL_MOPP, &args![cell]).u32();
            e.call(CELL_MOPP_PREPARE, &args![cell_mopp]);
            e.with_stack(0x10, |e, local| {
                e.call(LAND_MOPP_LOCAL_CONSTRUCT, &args![local]);
            });
            let base_height = e.get(data, LoadedLandData::fBaseHeight);
            e.call(
                CELL_MOPP_CREATE,
                &args![cell_mopp, strips, QUADRANTS, base_height],
            );
            for follow_up in LAND_BUILD_FOLLOW_UPS {
                e.call(follow_up, &args![this]);
            }
            for quadrant in 0..QUADRANTS {
                let tri_strips = e.mem.u32(strips + 4 * quadrant);
                if tri_strips != 0 {
                    let geometry_data = e.call(GEOMETRY_GET_DATA, &args![tri_strips]).u32();
                    e.call(
                        GEOMETRY_DATA_SET_CONSISTENCY,
                        &args![geometry_data, 0x4000u32],
                    );
                    e.call(GEOMETRY_DATA_SET_KEEP_FLAGS, &args![geometry_data, 1u32]);
                    e.call(
                        GEOMETRY_DATA_SET_COMPRESS_FLAGS,
                        &args![geometry_data, 0x11u32 | 6],
                    );
                    let scheduler = e.call(GEOMETRY_GET_SCHEDULER, &args![tri_strips]).u32();
                    // Only a scheduler whose saved acquire object (asked twice, as the
                    // code does) is 1 to 5 gets the geometry queued.
                    let accepted = scheduler != 0
                        && e.call(SCHEDULER_GET_SAVED_ACQUIRE, &args![scheduler]).i32() >= 1
                        && e.call(SCHEDULER_GET_SAVED_ACQUIRE, &args![scheduler]).i32() <= 5;
                    if accepted {
                        let renderer = e.call(RENDERER_GET, &[]).u32();
                        let task = e.call(LAND_RENDER_TASK, &args![scheduler]).u32();
                        e.vcall(
                            renderer,
                            RENDERER_QUEUE_SLOT,
                            &args![tri_strips, 0u32, 0u32, task],
                        );
                        e.call(RENDERER_AFTER_QUEUE, &args![renderer]);
                    }
                }
                let node = element(e, meshes, quadrant);
                if node != 0 {
                    let was_hidden = e.mem.u8(hidden + quadrant) as u32;
                    e.call(NODE_SET_CULLED, &args![node, was_hidden]);
                    e.with_stack(0x10, |e, update| {
                        e.call(UPDATE_DATA_CONSTRUCT, &args![update, 0.0f32, 0u32, 0u32]);
                        e.call(NODE_UPDATE, &args![node, update]);
                    });
                    e.call(NODE_UPDATE_PROPERTIES, &args![node]);
                }
            }
        });
    }
    let meshes = e.get(data, LoadedLandData::ppMesh);
    if meshes.is_null() {
        return;
    }
    if e.call(PARENT_CELL, &args![this]).u32() == 0 {
        return;
    }
    let mut hide_all = false;
    if fn_00534140(e, this) {
        let cell = e.call(PARENT_CELL, &args![this]).u32();
        hide_all = !e.call(CELL_REFERENCE_LIST_CHECK, &args![cell]).bool();
    }
    for quadrant in 0..QUADRANTS {
        let hide = if hide_all {
            true
        } else {
            let cell = e.call(PARENT_CELL, &args![this]).u32();
            e.call(CELL_QUADRANT_TEST, &args![cell, quadrant]).bool()
        };
        let node = element(e, meshes, quadrant);
        e.call(NODE_SET_CULLED, &args![node, hide as u32]);
    }
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00533120,
            tes_object_land_tes_object_land(Ptr<TESObjectLAND>) -> Ptr<TESObjectLAND>
        ),
        entry!(0x005331f0, fn_005331f0(Ptr) -> Ptr),
        entry!(
            0x00533210,
            tes_object_land_scalar_deleting_destructor(
                Ptr<TESObjectLAND>,
                u32,
            ) -> Ptr<TESObjectLAND>
        ),
        entry!(0x00533240, fn_00533240(Ptr) -> Ptr),
        entry!(0x00533260, tes_object_land_destructor(Ptr<TESObjectLAND>)),
        entry!(0x00533420, tes_object_land_initialize_statics()),
        entry!(0x00533f40, fn_00533f40(Ptr, Ptr)),
        entry!(0x00533fb0, fn_00533fb0(Ptr, u16)),
        entry!(0x00533fd0, fn_00533fd0(Ptr<TESObjectLAND>) -> i32),
        entry!(0x00534010, fn_00534010(Ptr<TESObjectLAND>) -> i32),
        entry!(0x00534050, fn_00534050(Ptr<TESObjectLAND>) -> f64),
        entry!(0x00534080, fn_00534080(Ptr<TESObjectLAND>) -> f64),
        entry!(0x005340b0, fn_005340b0(Ptr<TESObjectLAND>) -> Ptr),
        entry!(
            0x005340e0,
            tes_object_land_set_cell(Ptr<TESObjectLAND>, Ptr)
        ),
        entry!(0x00534140, fn_00534140(Ptr<TESObjectLAND>) -> bool),
        entry!(0x00534160, fn_00534160(Ptr<TESObjectLAND>, bool)),
        entry!(0x005341a0, fn_005341a0(Ptr<TESObjectLAND>, bool)),
        entry!(0x005341e0, fn_005341e0(Ptr<TESObjectLAND>) -> bool),
        entry!(0x00534200, fn_00534200(Ptr<TESObjectLAND>, bool)),
        entry!(0x00534240, fn_00534240(Ptr<TESObjectLAND>, u32, i32, Ptr)),
        entry!(0x00534390, fn_00534390(Ptr<TESObjectLAND>, u32, i32, Ptr)),
        entry!(0x00534550, fn_00534550(Ptr) -> Ptr),
        entry!(0x00534570, fn_00534570(Ptr, i32, Ptr)),
        entry!(0x005345c0, fn_005345c0(Ptr<TESObjectLAND>, u32, i32, Ptr)),
        entry!(0x00534780, fn_00534780(Ptr, i32, Ptr)),
        entry!(0x00534880, fn_00534880(Ptr, u32, u32) -> Ptr),
        entry!(0x005348c0, fn_005348c0(Ptr<TESObjectLAND>, Ptr)),
        entry!(
            0x00534ad0,
            tes_object_land_load(Ptr<TESObjectLAND>, Ptr) -> bool
        ),
        entry!(0x00535a60, fn_00535a60(Ptr)),
        entry!(0x00535aa0, fn_00535aa0(Ptr)),
        entry!(0x00535ae0, fn_00535ae0() -> Ptr),
        entry!(0x00535af0, fn_00535af0(Ptr<TESObjectLAND>, i32) -> Ptr),
        entry!(
            0x00535b30,
            tes_object_land_get_world_offset_for_block(Ptr<TESObjectLAND>, Ptr, u32) -> Ptr
        ),
        entry!(
            0x00535b90,
            tes_object_land_load_vertices(Ptr<TESObjectLAND>, bool) -> bool
        ),
        entry!(
            0x00535d00,
            tes_object_land_load_vertices_into_arrays(Ptr<TESObjectLAND>) -> bool
        ),
        entry!(
            0x00536d80,
            tes_object_land_un_load_vertices(Ptr<TESObjectLAND>) -> bool
        ),
        entry!(
            0x00537100,
            fn_00537100(Ptr<LoadedLandData>, u32) -> Ptr<LoadedLandData>
        ),
        entry!(0x00537130, fn_00537130(Ptr<LoadedLandData>)),
        entry!(
            0x005371b0,
            tes_object_land_make_land_tri_strips(Ptr<TESObjectLAND>, u8) -> Ptr
        ),
        entry!(0x005374f0, fn_005374f0(Ptr<TESObjectLAND>)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    // ---- Test support ----------------------------------------------------------

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn float_ret(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    /// The pages that hold the exe's constants and the statics.
    const PAGES: [u32; 18] = [
        0x0101_1000,
        0x0101_2000,
        0x0101_5000,
        0x0101_6000,
        0x0101_7000,
        0x0101_d000,
        0x0101_e000,
        0x0102_3000,
        0x0102_d000,
        0x0118_3000,
        0x0118_a000,
        0x0118_b000,
        0x011a_9000,
        0x011c_3000,
        0x011c_9000,
        0x011c_a000,
        0x011f_4000,
        0x011f_5000,
    ];

    /// An engine with the exe's constants the code reads and doubles for the
    /// helpers nearly every function goes through (allocator, `memcpy`,
    /// `memset`, scope guard, smart-pointer reader and the vector
    /// constructors).
    fn engine() -> Engine {
        let mut e = Engine::new();
        // A cast that finds the object (tests of the cast replace it).
        e.register(RT_DYNAMIC_CAST, |_, a| ret(a[0]));
        for page in PAGES {
            e.map(page, 0x1000);
        }
        e.set_global(LARGEST_F32, f32::MAX);
        e.set_global(LOWEST_F32, -f32::MAX);
        e.set_global(SIXTEEN_F32, 16.0f32);
        e.set_global(F32_1024, 1024.0f32);
        e.set_global(F32_MINUS_2048, -2048.0f32);
        e.set_global(F64_ZERO, 0.0f64);
        e.set_global(F64_ONE, 1.0f64);
        e.set_global(F64_HALF, 0.5f64);
        e.set_global(F64_TWO, 2.0f64);
        e.set_global(F64_FOUR, 4.0f64);
        e.set_global(F64_HUNDRED, 100.0f64);
        e.set_global(F64_127, 127.0f64);
        e.set_global(F64_255, 255.0f64);
        e.set_global(F64_1024, 1024.0f64);
        e.set_global(F64_2048, 2048.0f64);
        for (i, value) in [1.0f32, 1.0, 1.0, 1.0].into_iter().enumerate() {
            e.set_global(DEFAULT_COLOR_WORDS + 4 * i as u32, value);
        }
        e.register(ALLOCATE, |e, a| ret(e.mem.alloc(a[0])));
        e.register(DEALLOCATE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(MEMORY_COPY, |e, a| {
            let bytes = e.mem.bytes(a[1], a[2]);
            e.mem.write(a[0], &bytes);
            ret(a[0])
        });
        e.register(MEMORY_SET, |e, a| {
            let bytes = vec![a[1] as u8; a[2] as usize];
            e.mem.write(a[0], &bytes);
            ret(a[0])
        });
        e.register(SCOPE_GUARD_ENTER, |_, _| Ret::default());
        e.register(SCOPE_GUARD_LEAVE, |_, _| Ret::default());
        e.register(SMART_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        e.register(NI_POINT3_CONSTRUCT, |e, a| {
            for i in 0..3 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            ret(a[0])
        });
        e.register(NI_POINT2_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            e.mem.set_u32(a[0] + 4, a[2]);
            ret(a[0])
        });
        e.register(NI_COLOR_CONSTRUCT, |e, a| {
            for i in 0..4 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            ret(a[0])
        });
        e.register(NI_POINT3_DEFAULT_CONSTRUCT, |_, a| ret(a[0]));
        e.register(NI_COLOR_DEFAULT_CONSTRUCT, |_, a| ret(a[0]));
        e.register(VECTOR_CONSTRUCT, |e, a| {
            for i in 0..a[2] {
                e.call(a[3], &args![a[0] + i * a[1]]);
            }
            ret(a[0])
        });
        // `0045cb90`: loaded data with vertex arrays.
        e.register(LAND_HAS_VERTEX_ARRAYS, |e, a| {
            let data = e.mem.u32(a[0] + 0x28);
            ret((data != 0 && e.mem.u32(data + 4) != 0) as u32)
        });
        e.register(PARENT_CELL, |e, a| ret(e.mem.u32(a[0] + 0x20)));
        e.register(FORM_ID, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e
    }

    /// Starts recording calls.
    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The calls recorded since `start_log`, without the first one (the call
    /// of the function under test), and stops recording.
    fn end_log(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.take().unwrap().into_iter().skip(1).collect()
    }

    fn called(e: &mut Engine) -> Vec<u32> {
        end_log(e).into_iter().map(|(address, _)| address).collect()
    }

    fn arguments_of(log: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// A shared list the doubles append to.
    fn recorder<T: 'static>() -> Rc<RefCell<Vec<T>>> {
        Rc::new(RefCell::new(Vec::new()))
    }

    fn land(e: &mut Engine) -> Ptr<TESObjectLAND> {
        e.new_object()
    }

    /// An object with a vtable whose `(offset, target)` slots are given.
    fn object_with_vtable(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x800);
        for (offset, target) in slots {
            e.mem.set_u32(vtable + offset, *target);
        }
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, vtable);
        object
    }

    fn read_cstring(e: &Engine, address: u32) -> String {
        String::from_utf8(e.mem.cstr(address)).unwrap()
    }

    /// Floats at `address`.
    fn floats(e: &Engine, address: u32, count: u32) -> Vec<f32> {
        (0..count).map(|i| e.mem.f32(address + 4 * i)).collect()
    }

    /// A land with loaded data whose arrays are all allocated like the
    /// game's: four blocks of 0x121 vertices, normals (0xd8c bytes each),
    /// colours (0x1210), normal-set flags (0x121), the percent arrays and
    /// the layer texture arrays. Returns the land and its data.
    fn land_with_data(e: &mut Engine) -> (Ptr<TESObjectLAND>, Ptr<LoadedLandData>) {
        let this = land(e);
        let data: Ptr<LoadedLandData> = e.new_object();
        e.set(this, TESObjectLAND::pLoadedData, data.cast());
        for (field, size) in [
            (LoadedLandData::ppVertices.off, 0xd8cu32),
            (LoadedLandData::ppNormals.off, 0xd8c),
            (LoadedLandData::ppColorsA.off, 0x1210),
            (LoadedLandData::ppNormalsSet.off, 0x121),
        ] {
            let array = e.mem.alloc(16);
            for quadrant in 0..4 {
                let block = e.mem.alloc(size);
                e.mem.set_u32(array + 4 * quadrant, block);
            }
            e.mem.set_u32(data.addr() + field, array);
        }
        for quadrant in 0..4 {
            let layers = e.mem.alloc(0x18);
            e.mem.set_u32(
                data.addr() + LoadedLandData::pQuadTextureArray.off + 4 * quadrant,
                layers,
            );
            let percent = e.mem.alloc(4);
            let block = e.mem.alloc(0x2420);
            e.mem.set_u32(percent, block);
            e.mem.set_u32(
                data.addr() + LoadedLandData::ppPercentArrays.off + 4 * quadrant,
                percent,
            );
        }
        (this, data)
    }

    // ---- Constructor, destructors and statics ---------------------------------

    #[test]
    fn constructor_sets_up_the_land() {
        let mut e = engine();
        for address in [
            FORM_CONSTRUCT,
            NI_POINTER_SLOT_CONSTRUCT,
            FORM_SET_FORM_TYPE,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        e.set_global(STATICS_DEFINED, 1u8);
        e.set_global(LANDS_IN_USE, 2u32);
        let this = land(&mut e);
        e.set(this, TESObjectLAND::Data, 0xffff_ffff);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x1234));
        e.set(this, TESObjectLAND::pLoadedData, Ptr::new(0x5678));
        start_log(&mut e);
        let result = e.call(0x0053_3120, &args![this]);
        assert_eq!(result.u32(), this.addr());
        let log = end_log(&mut e);
        let addresses: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            addresses,
            vec![
                FORM_CONSTRUCT,
                NI_POINTER_SLOT_CONSTRUCT,
                MEMORY_SET,
                FORM_SET_FORM_TYPE
            ]
        );
        assert_eq!(
            log[1].1,
            vec![this.addr() + 0x24, 0],
            "the queued textures slot"
        );
        assert_eq!(log[3].1, vec![this.addr(), 0x42]);
        assert_eq!(e.mem.u32(this.addr()), LAND_VTABLE);
        assert_eq!(e.mem.u32(this.addr() + 0x18), CHILD_CELL_VTABLE);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0);
        assert!(e.get(this, TESObjectLAND::pParentCell).is_null());
        assert!(e.get(this, TESObjectLAND::pLoadedData).is_null());
        assert_eq!(e.global::<u32>(LANDS_IN_USE), 3);
    }

    #[test]
    fn child_cell_constructor_stores_the_abstract_vtable() {
        let mut e = engine();
        let object = e.mem.alloc(8);
        let result = e.call(0x0053_3240, &args![object]);
        assert_eq!(result.u32(), object);
        assert_eq!(e.mem.u32(object), CHILD_CELL_ABSTRACT_VTABLE);
    }

    #[test]
    fn save_parent_cell_is_the_lands_cell() {
        let mut e = engine();
        let this = land(&mut e);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x7000));
        let child = this.addr() + 0x18;
        assert_eq!(e.call(0x0053_31f0, &args![child]).u32(), 0x7000);
    }

    /// Doubles for what the destructor calls besides the unload.
    fn destructor_engine() -> Engine {
        let mut e = engine();
        for address in [
            LAND_SET_LOADED,
            LAND_RELEASE_BORDER,
            FORM_BASE_DESTRUCT,
            QUEUED_FILE_POINTER_DESTRUCT,
            FORM_DESTRUCT,
            TEXTURING_PROPERTY_SET,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        e
    }

    #[test]
    fn destructor_keeps_the_statics_while_other_lands_exist() {
        let mut e = destructor_engine();
        e.set_global(LANDS_IN_USE, 2u32);
        let this = land(&mut e);
        start_log(&mut e);
        e.call(0x0053_3260, &args![this]);
        assert_eq!(
            called(&mut e),
            vec![
                LAND_SET_LOADED,
                LAND_RELEASE_BORDER,
                FORM_BASE_DESTRUCT,
                QUEUED_FILE_POINTER_DESTRUCT,
                FORM_DESTRUCT
            ]
        );
        assert_eq!(e.global::<u32>(LANDS_IN_USE), 1);
        assert_eq!(e.mem.u32(this.addr()), LAND_VTABLE);
        assert_eq!(e.mem.u32(this.addr() + 0x18), CHILD_CELL_VTABLE);
    }

    #[test]
    fn destructor_of_the_last_land_frees_the_statics() {
        let mut e = destructor_engine();
        e.set_global(LANDS_IN_USE, 1u32);
        e.set_global(STATICS_DEFINED, 1u8);
        let mut blocks = vec![];
        for global in [
            DEFAULT_TRIANGLE_LIST,
            DEFAULT_COLORS,
            DEFAULT_NORMALS,
            DEFAULT_TEXTURE_COORDINATES,
            DEFAULT_NORMAL_SET,
        ] {
            let block = e.mem.alloc(16);
            e.set_global(global, block);
            blocks.push(block);
        }
        for quadrant in 0..4 {
            let block = e.mem.alloc(16);
            e.set_global(DEFAULT_VERTEX_BLOCKS + 4 * quadrant, block);
            blocks.push(block);
        }
        // The default land texture has a destructor in slot 4; the texture
        // set is absent.
        let texture = object_with_vtable(&mut e, 0x28, &[(0x10, 0x00aa_0001)]);
        e.set_global(DEFAULT_LAND_TEXTURE, texture);
        e.set_global(DEFAULT_TEXTURE_SET, 0u32);
        let deleted = recorder::<Vec<u32>>();
        let sink = deleted.clone();
        e.register_double(0x00aa_0001, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let this = land(&mut e);
        start_log(&mut e);
        e.call(0x0053_3260, &args![this]);
        let log = end_log(&mut e);
        let freed: Vec<u32> = arguments_of(&log, DEALLOCATE)
            .into_iter()
            .map(|a| a[0])
            .collect();
        assert_eq!(freed, blocks);
        assert_eq!(e.global::<u8>(STATICS_DEFINED), 0);
        assert_eq!(
            arguments_of(&log, TEXTURING_PROPERTY_SET),
            vec![vec![DEFAULT_TEXTURING_PROPERTY, 0]]
        );
        assert_eq!(*deleted.borrow(), vec![vec![texture, 1]]);
        assert_eq!(e.global::<u32>(DEFAULT_LAND_TEXTURE), 0);
        assert_eq!(e.global::<u32>(DEFAULT_TEXTURE_SET), 0);
        assert_eq!(e.global::<u32>(LANDS_IN_USE), 0);
    }

    #[test]
    fn scalar_deleting_destructor_frees_only_with_bit_zero() {
        let mut e = destructor_engine();
        e.set_global(LANDS_IN_USE, 5u32);
        let this = land(&mut e);
        start_log(&mut e);
        let kept = e.call(0x0053_3210, &args![this, 0u32]);
        assert_eq!(kept.u32(), this.addr());
        assert!(!called(&mut e).contains(&DEALLOCATE));
        let other = land(&mut e);
        start_log(&mut e);
        let freed = e.call(0x0053_3210, &args![other, 1u32]);
        assert_eq!(freed.u32(), other.addr());
        let log = end_log(&mut e);
        assert_eq!(log.last().unwrap(), &(DEALLOCATE, vec![other.addr()]));
    }

    /// What `InitializeStatics` calls, with the texture set object it
    /// builds, the map and the texturing property recorded.
    struct StaticsRecord {
        paths: Rc<RefCell<Vec<(u32, String)>>>,
        slot_sets: Rc<RefCell<Vec<Vec<u32>>>>,
    }

    /// The engine for `InitializeStatics`: `tiling` is the value of the
    /// INI setting.
    fn statics_engine(tiling: f32) -> (Engine, StaticsRecord) {
        let mut e = engine();
        // Object allocator.
        e.register(NI_ALLOCATE_OBJECT, |e, a| ret(e.mem.alloc(a[0].max(0x200))));
        // The texture set: a vtable at +0x30 whose slot 0x90 stores a
        // texture into the smart pointer it is given.
        let texture_vtable = e.mem.alloc(0x100);
        e.mem.set_u32(texture_vtable + 0x90, 0x00aa_0002);
        e.register(0x00aa_0002, |e, a| {
            e.mem.set_u32(a[2], 0x7777_0000);
            Ret::default()
        });
        e.register_double(TEXTURE_SET_CONSTRUCT, move |e, a| {
            e.mem.set_u32(a[0] + 0x30, texture_vtable);
            ret(a[0])
        });
        e.register(SETTING_GET_STRING, |e, a| {
            let text = if a[0] == DEFAULT_DIFFUSE_TEXTURE_SETTING {
                "Diffuse.dds"
            } else {
                "Normal.dds"
            };
            let address = e.mem.alloc(16);
            e.mem.set_cstr(address, text.as_bytes());
            ret(address)
        });
        e.register(FORMAT_STRING, |e, a| {
            let name = e.mem.cstr(a[3]);
            let mut text = b"Landscape\\".to_vec();
            text.extend_from_slice(&name);
            e.mem.set_cstr(a[0], &text);
            ret(text.len() as u32)
        });
        let paths = recorder::<(u32, String)>();
        let sink = paths.clone();
        e.register_double(TEXTURE_SET_SET_PATH, move |e, a| {
            sink.borrow_mut()
                .push((a[1], String::from_utf8(e.mem.cstr(a[2])).unwrap()));
            Ret::default()
        });
        e.set_global(MENU_MANAGER, 0x0900_0000u32);
        e.register(MENU_MANAGER_REGISTER_TEXTURE_SET, |_, a| ret(a[0] + 1));
        e.register(MENU_MANAGER_AFTER_REGISTER, |_, _| Ret::default());
        e.register(LAND_TEXTURE_CONSTRUCT, |_, a| ret(a[0]));
        e.register(LAND_TEXTURE_SET_TEXTURE_SET, |_, _| Ret::default());
        e.register(SMART_POINTER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(SMART_POINTER_RELEASE, |e, a| {
            e.mem.set_u32(a[0], 0);
            Ret::default()
        });
        e.register(TEXTURE_MAP_CONSTRUCT, |_, a| ret(a[0] + 1));
        e.register(TEXTURING_PROPERTY_CONSTRUCT, |_, a| ret(a[0]));
        e.register(TEXTURING_PROPERTY_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(PROPERTY_SET_MODE, |_, _| Ret::default());
        // The property's map table: slot 0 lives at `table + 0x100`.
        e.register(ARRAY_ELEMENT_ADDRESS, |_, a| ret(a[0] + 0x100));
        let slot_sets = recorder::<Vec<u32>>();
        let sink = slot_sets.clone();
        e.register_double(ARRAY_SET_AT, move |e, a| {
            sink.borrow_mut().push(vec![a[0], a[1], e.mem.u32(a[2])]);
            Ret::default()
        });
        let setting = e.mem.alloc(4);
        e.mem.set_f32(setting, tiling);
        e.register_double(SETTING_GET_FLOAT_ADDRESS, move |_, _| ret(setting));
        e.register(LOG_MESSAGE, |_, _| Ret::default());
        (e, StaticsRecord { paths, slot_sets })
    }

    #[test]
    fn initialize_statics_builds_the_shared_blocks() {
        let (mut e, record) = statics_engine(8.0);
        start_log(&mut e);
        e.call(0x0053_3420, &[]);
        let log = end_log(&mut e);
        // The guard is entered with the tag, the source file and the line.
        assert_eq!(
            log[0],
            (
                SCOPE_GUARD_ENTER,
                vec![log[0].1[0], 0x1b, 1, SOURCE_FILE, 0xc6]
            )
        );
        assert_eq!(log.last().unwrap().0, SCOPE_GUARD_LEAVE);
        // The texture set and its two paths.
        let texture_set = e.global::<u32>(DEFAULT_TEXTURE_SET);
        assert_ne!(texture_set, 0);
        assert_eq!(
            *record.paths.borrow(),
            vec![
                (0, "Landscape\\Diffuse.dds".to_string()),
                (1, "Landscape\\Normal.dds".to_string())
            ]
        );
        assert_eq!(
            arguments_of(&log, MENU_MANAGER_REGISTER_TEXTURE_SET),
            vec![vec![0x0900_0000, texture_set]]
        );
        assert_eq!(
            arguments_of(&log, MENU_MANAGER_AFTER_REGISTER),
            vec![vec![0x0900_0001]]
        );
        let land_texture = e.global::<u32>(DEFAULT_LAND_TEXTURE);
        assert_eq!(
            arguments_of(&log, LAND_TEXTURE_SET_TEXTURE_SET),
            vec![vec![land_texture, texture_set]]
        );
        // The map is built from the texture the set's slot 0x90 produced
        // and becomes slot 0 of the property; the property gets mode 2.
        let maps = arguments_of(&log, TEXTURE_MAP_CONSTRUCT);
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0][1..], [0x7777_0000, 0, 3, 5, 0]);
        let map = maps[0][0] + 1;
        let property = e.mem.u32(DEFAULT_TEXTURING_PROPERTY);
        assert_ne!(property, 0);
        assert_eq!(
            arguments_of(&log, PROPERTY_SET_MODE),
            vec![vec![property, 2, 0xe, 1]]
        );
        assert_eq!(
            *record.slot_sets.borrow(),
            vec![vec![property + 0x1c, 0, map]]
        );
        // The triangle list: cell (0, 0) has the same parity in both
        // directions, cell (0, 1) does not.
        let triangles = e.global::<u32>(DEFAULT_TRIANGLE_LIST);
        let cell = |row: u32, column: u32| -> Vec<u16> {
            (0..6)
                .map(|i| e.mem.u16(triangles + 2 * ((row * 16 + column) * 6 + i)))
                .collect()
        };
        assert_eq!(cell(0, 0), vec![18, 17, 0, 0, 1, 18]);
        assert_eq!(cell(0, 1), vec![18, 1, 2, 2, 19, 18]);
        assert_eq!(cell(1, 0), vec![34, 17, 18, 18, 35, 34]);
        assert_eq!(cell(15, 15), vec![288, 287, 270, 270, 271, 288]);
        // Colours, normals, flags and texture coordinates (4 / 8 = 0.5).
        let colors = e.global::<u32>(DEFAULT_COLORS);
        let normals = e.global::<u32>(DEFAULT_NORMALS);
        let flags = e.global::<u32>(DEFAULT_NORMAL_SET);
        let coordinates = e.global::<u32>(DEFAULT_TEXTURE_COORDINATES);
        let k = 17 + 2;
        assert_eq!(floats(&e, colors + 0x10 * k, 4), vec![1.0, 1.0, 1.0, 0.0]);
        assert_eq!(floats(&e, normals + 12 * k, 3), vec![0.0, 0.0, 1.0]);
        assert_eq!(e.mem.u8(flags + k), 1);
        assert_eq!(e.mem.u8(flags + 0x120), 1);
        assert_eq!(floats(&e, coordinates + 8 * k, 2), vec![4.0, 2.0]);
        // Quadrant offsets and the default vertex blocks at -2048.
        let x_offsets = floats(&e, X_OFFSETS, 4);
        let y_offsets = floats(&e, Y_OFFSETS, 4);
        assert_eq!(x_offsets, vec![-2048.0, 0.0, -2048.0, 0.0]);
        assert_eq!(y_offsets, vec![-2048.0, -2048.0, 0.0, 0.0]);
        let block = e.global::<u32>(DEFAULT_VERTEX_BLOCKS + 4);
        assert_eq!(
            floats(&e, block + 12 * (2 * 17 + 3), 3),
            vec![384.0, -1792.0, -2048.0]
        );
        let last = e.global::<u32>(DEFAULT_VERTEX_BLOCKS + 12);
        assert_eq!(
            floats(&e, last + 12 * 0x120, 3),
            vec![2048.0, 2048.0, -2048.0]
        );
        assert_eq!(e.global::<u8>(STATICS_DEFINED), 1);
        // The smart pointer that held the texture is released at the end.
        assert!(arguments_of(&log, SMART_POINTER_RELEASE).len() == 1);
    }

    #[test]
    fn initialize_statics_replaces_a_zero_tiling_with_sixteen() {
        let (mut e, _) = statics_engine(0.0);
        start_log(&mut e);
        e.call(0x0053_3420, &[]);
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, LOG_MESSAGE), vec![vec![TILING_WARNING]]);
        // 4 / 16 = 0.25, so column 1 is at u = 4.
        let coordinates = e.global::<u32>(DEFAULT_TEXTURE_COORDINATES);
        assert_eq!(floats(&e, coordinates + 8, 2), vec![4.0, 0.0]);
    }

    #[test]
    fn constructor_defines_the_statics_once() {
        let (mut e, _) = statics_engine(16.0);
        for address in [
            FORM_CONSTRUCT,
            NI_POINTER_SLOT_CONSTRUCT,
            FORM_SET_FORM_TYPE,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        let this = land(&mut e);
        e.call(0x0053_3120, &args![this]);
        assert_eq!(e.global::<u8>(STATICS_DEFINED), 1);
        assert_eq!(e.global::<u32>(LANDS_IN_USE), 1);
        let first = e.global::<u32>(DEFAULT_TRIANGLE_LIST);
        let other = land(&mut e);
        e.call(0x0053_3120, &args![other]);
        assert_eq!(e.global::<u32>(DEFAULT_TRIANGLE_LIST), first);
        assert_eq!(e.global::<u32>(LANDS_IN_USE), 2);
    }

    #[test]
    fn texture_map_slot_is_replaced_only_when_it_differs() {
        let mut e = engine();
        e.register(ARRAY_ELEMENT_ADDRESS, |_, a| ret(a[0] + 0x100));
        let sets = recorder::<Vec<u32>>();
        let sink = sets.clone();
        e.register_double(ARRAY_SET_AT, move |e, a| {
            sink.borrow_mut().push(vec![a[0], a[1], e.mem.u32(a[2])]);
            Ret::default()
        });
        let deleted = recorder::<Vec<u32>>();
        let sink = deleted.clone();
        e.register_double(0x00aa_0003, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let property = e.mem.alloc(0x200);
        let old = object_with_vtable(&mut e, 0x10, &[(0, 0x00aa_0003)]);
        e.mem.set_u32(property + 0x1c + 0x100, old);
        // The same map: nothing happens.
        e.call(0x0053_3f40, &args![property, old]);
        assert!(sets.borrow().is_empty() && deleted.borrow().is_empty());
        // A different map: the old one is deleted and the slot set.
        e.call(0x0053_3f40, &args![property, 0x4444u32]);
        assert_eq!(*deleted.borrow(), vec![vec![old, 1]]);
        assert_eq!(*sets.borrow(), vec![vec![property + 0x1c, 0, 0x4444]]);
        // An empty slot: no deletion.
        e.mem.set_u32(property + 0x1c + 0x100, 0);
        e.call(0x0053_3f40, &args![property, 0x5555u32]);
        assert_eq!(deleted.borrow().len(), 1);
        assert_eq!(sets.borrow().len(), 2);
    }

    #[test]
    fn property_mode_setter_passes_the_value() {
        let mut e = engine();
        e.register(PROPERTY_SET_MODE, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0053_3fb0, &args![0x1000u32, 0xffff_0002u32]);
        let log = end_log(&mut e);
        assert_eq!(log, vec![(PROPERTY_SET_MODE, vec![0x1000, 2, 0xe, 1])]);
    }

    // ---- Cell accessors and flags ---------------------------------------------

    #[test]
    fn cell_coordinates_come_from_the_data_then_the_cell() {
        let mut e = engine();
        e.register(CELL_GET_DATA_X, |_, _| ret(-7i32 as u32));
        e.register(CELL_GET_DATA_Y, |_, _| ret(9));
        let this = land(&mut e);
        // No data and no cell: zero.
        assert_eq!(e.call(0x0053_3fd0, &args![this]).i32(), 0);
        assert_eq!(e.call(0x0053_4010, &args![this]).i32(), 0);
        // The cell answers.
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x3000));
        assert_eq!(e.call(0x0053_3fd0, &args![this]).i32(), -7);
        assert_eq!(e.call(0x0053_4010, &args![this]).i32(), 9);
        // The data wins.
        let data: Ptr<LoadedLandData> = e.new_object();
        e.set(data, LoadedLandData::iCellX, 12);
        e.set(data, LoadedLandData::iCellY, -13);
        e.set(this, TESObjectLAND::pLoadedData, data.cast());
        assert_eq!(e.call(0x0053_3fd0, &args![this]).i32(), 12);
        assert_eq!(e.call(0x0053_4010, &args![this]).i32(), -13);
    }

    #[test]
    fn world_corner_is_the_cell_times_4096() {
        let mut e = engine();
        let this = land(&mut e);
        let data: Ptr<LoadedLandData> = e.new_object();
        e.set(data, LoadedLandData::iCellX, -3);
        e.set(data, LoadedLandData::iCellY, 5);
        e.set(this, TESObjectLAND::pLoadedData, data.cast());
        assert_eq!(e.call(0x0053_4050, &args![this]).f64(), -12288.0);
        assert_eq!(e.call(0x0053_4080, &args![this]).f64(), 20480.0);
    }

    #[test]
    fn world_space_is_asked_of_the_cell() {
        let mut e = engine();
        e.register(CELL_GET_WORLD_SPACE, |_, a| ret(a[0] + 1));
        let this = land(&mut e);
        assert_eq!(e.call(0x0053_40b0, &args![this]).u32(), 0);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x3000));
        assert_eq!(e.call(0x0053_40b0, &args![this]).u32(), 0x3001);
    }

    #[test]
    fn set_cell_sets_flag_400_from_the_world_space() {
        let mut e = engine();
        // The cell's world space is `cell + 1` unless the cell is 0x5000
        // (none); the world space has land data when its address is odd.
        e.register(CELL_GET_WORLD_SPACE, |_, a| {
            ret(if a[0] == 0x5000 { 0 } else { a[0] + 1 })
        });
        e.register(WORLD_SPACE_HAS_LAND, |_, a| ret(a[0] & 1));
        let this = land(&mut e);
        e.set(this, TESObjectLAND::Data, 0x401);
        // A null cell only stores.
        e.call(0x0053_40e0, &args![this, 0u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x401);
        // A world space with land data sets the bit...
        e.set(this, TESObjectLAND::Data, 0);
        e.call(0x0053_40e0, &args![this, 0x3000u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x400);
        assert_eq!(e.get(this, TESObjectLAND::pParentCell).addr(), 0x3000);
        // ...one without clears it, and so does a cell without a world space.
        e.call(0x0053_40e0, &args![this, 0x3001u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0);
        e.set(this, TESObjectLAND::Data, 0x400);
        e.call(0x0053_40e0, &args![this, 0x5000u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0);
    }

    #[test]
    fn flag_bits_are_tested_and_set() {
        let mut e = engine();
        let this = land(&mut e);
        e.set(this, TESObjectLAND::Data, 0x2);
        assert_eq!(e.call(0x0053_4140, &args![this]).u32() & 0xff, 0);
        e.call(0x0053_4160, &args![this, 1u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x402);
        assert_eq!(e.call(0x0053_4140, &args![this]).u32() & 0xff, 1);
        e.call(0x0053_4160, &args![this, 0u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x2);
        e.call(0x0053_41a0, &args![this, 7u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x802);
        e.call(0x0053_41a0, &args![this, 0u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x2);
        assert_eq!(e.call(0x0053_41e0, &args![this]).u32() & 0xff, 0);
        e.call(0x0053_4200, &args![this, 1u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x12);
        assert_eq!(e.call(0x0053_41e0, &args![this]).u32() & 0xff, 1);
        e.call(0x0053_4200, &args![this, 0u32]);
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x2);
    }

    // ---- Vertex, normal and colour readers --------------------------------------

    /// Doubles for the mesh geometry the readers fall back to: the node's
    /// geometry is at `node + 4`, its positions at `geometry + 0`, normals
    /// at `+8`, colours at `+0xc`, its geometry data at `+0x10`; the data's
    /// owner (at `data + 0`) reports whether it can be locked.
    fn geometry_engine() -> Engine {
        let mut e = engine();
        e.register(LAND_DEFAULT_HEIGHT, |_, _| float_ret(7.0));
        e.register(GEOMETRY_GET_MODEL_DATA, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(GEOMETRY_GET_POSITIONS, |e, a| ret(e.mem.u32(a[0])));
        e.register(GEOMETRY_GET_NORMALS, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(GEOMETRY_GET_COLORS, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register(GEOMETRY_GET_DATA, |e, a| ret(e.mem.u32(a[0] + 0x10)));
        e.register(GEOMETRY_DATA_GET_OWNER, |e, a| ret(e.mem.u32(a[0])));
        e.register(GEOMETRY_DATA_BEGIN, |_, _| ret(1));
        e.register(GEOMETRY_DATA_END, |_, _| Ret::default());
        e.register(LOCK_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            e.mem.set_u8(a[0] + 8, 0);
            ret(a[0])
        });
        e.register(LOCK_CHECK_INDEX, |_, _| Ret::default());
        e
    }

    /// A byte as the code turns it into a colour channel.
    fn byte_fraction(byte: u32) -> f32 {
        (byte as f64 / 255.0) as f32
    }

    /// A mesh node whose geometry is `geometry`.
    fn mesh_node(e: &mut Engine, geometry: u32) -> u32 {
        let node = e.mem.alloc(16);
        e.mem.set_u32(node + 4, geometry);
        node
    }

    /// A geometry with its positions, normals, colours and data pointers.
    fn geometry(e: &mut Engine, positions: u32, normals: u32, colors: u32, data: u32) -> u32 {
        let geometry = e.mem.alloc(0x20);
        e.mem.set_u32(geometry, positions);
        e.mem.set_u32(geometry + 8, normals);
        e.mem.set_u32(geometry + 0xc, colors);
        e.mem.set_u32(geometry + 0x10, data);
        geometry
    }

    /// Geometry data whose owner can (or cannot) be locked; the owner's
    /// slot 0x8c answers.
    fn lockable_data(e: &mut Engine, can_lock: bool) -> u32 {
        e.register(0x00aa_0010, |_, _| ret(1));
        e.register(0x00aa_0011, |_, _| ret(0));
        let owner = object_with_vtable(
            e,
            16,
            &[(0x8c, if can_lock { 0x00aa_0010 } else { 0x00aa_0011 })],
        );
        let data = e.mem.alloc(16);
        e.mem.set_u32(data, owner);
        data
    }

    #[test]
    fn vertex_reader_follows_the_available_sources() {
        let mut e = geometry_engine();
        let out = e.mem.alloc(12);
        // No vertex arrays: (0, 0, default height), no base height added.
        let this = land(&mut e);
        e.call(0x0053_4240, &args![this, 1u32, 5u32, out]);
        assert_eq!(floats(&e, out, 3), vec![0.0, 0.0, 7.0]);

        let (this, data) = land_with_data(&mut e);
        e.set(data, LoadedLandData::fBaseHeight, 10.0);
        let vertices = e.get(data, LoadedLandData::ppVertices);
        // From ppVertices.
        let block = element(&e, vertices, 1);
        for (i, value) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
            e.mem.set_f32(block + 12 * 5 + 4 * i as u32, value);
        }
        e.call(0x0053_4240, &args![this, 1u32, 5u32, out]);
        assert_eq!(floats(&e, out, 3), vec![1.0, 2.0, 13.0]);

        // From the mesh geometry when the block is missing.
        e.mem.set_u32(vertices.addr() + 4, 0);
        let positions = e.mem.alloc(12 * 8);
        for (i, value) in [4.0f32, 5.0, 6.0].into_iter().enumerate() {
            e.mem.set_f32(positions + 12 * 6 + 4 * i as u32, value);
        }
        let g = geometry(&mut e, positions, 0, 0, 0);
        let node = mesh_node(&mut e, g);
        let meshes = e.mem.alloc(16);
        e.mem.set_u32(meshes + 4, node);
        e.set(data, LoadedLandData::ppMesh, Ptr::new(meshes));
        e.call(0x0053_4240, &args![this, 1u32, 6u32, out]);
        assert_eq!(floats(&e, out, 3), vec![4.0, 5.0, 16.0]);

        // Neither: (0, 0, default height) plus the base height.
        e.mem.set_u32(vertices.addr() + 8, 0);
        e.call(0x0053_4240, &args![this, 2u32, 6u32, out]);
        assert_eq!(floats(&e, out, 3), vec![0.0, 0.0, 17.0]);
    }

    /// Fills the vertex `index` of a locked stream: `{base, stride, packed}`.
    fn locking_geometry(e: &mut Engine, packed: bool) -> (u32, u32) {
        let stream = e.mem.alloc(64);
        let data = lockable_data(e, true);
        let geometry = geometry(e, 0, 0, 0, data);
        let node = mesh_node(e, geometry);
        let lock_to_stream = stream;
        let stride = if packed { 4 } else { 16 };
        e.register_double(GEOMETRY_DATA_LOCK_POSITIONS, move |e, a| {
            e.mem.set_u32(a[1], lock_to_stream);
            e.mem.set_u32(a[1] + 4, 12);
            Ret::default()
        });
        e.register_double(GEOMETRY_DATA_LOCK_COLORS, move |e, a| {
            e.mem.set_u32(a[1], lock_to_stream);
            e.mem.set_u32(a[1] + 4, stride);
            e.mem.set_u8(a[1] + 8, packed as u8);
            Ret::default()
        });
        (node, stream)
    }

    #[test]
    fn normal_reader_follows_the_available_sources() {
        let mut e = geometry_engine();
        let out = e.mem.alloc(12);
        let this = land(&mut e);
        e.call(0x0053_4390, &args![this, 0u32, 1u32, out]);
        assert_eq!(floats(&e, out, 3), vec![0.0, 0.0, 1.0]);

        // ppNormals.
        let (this, data) = land_with_data(&mut e);
        let normals = e.get(data, LoadedLandData::ppNormals);
        let block = element(&e, normals, 2);
        for (i, value) in [0.5f32, 0.25, 0.75].into_iter().enumerate() {
            e.mem.set_f32(block + 12 * 3 + 4 * i as u32, value);
        }
        e.call(0x0053_4390, &args![this, 2u32, 3u32, out]);
        assert_eq!(floats(&e, out, 3), vec![0.5, 0.25, 0.75]);

        // The mesh geometry's normal array.
        e.mem.set_u32(normals.addr(), 0);
        let array = e.mem.alloc(12 * 4);
        for (i, value) in [0.0f32, 1.0, 0.0].into_iter().enumerate() {
            e.mem.set_f32(array + 12 * 2 + 4 * i as u32, value);
        }
        let g = geometry(&mut e, 0, array, 0, 0);
        let node = mesh_node(&mut e, g);
        let meshes = e.mem.alloc(16);
        e.mem.set_u32(meshes, node);
        e.set(data, LoadedLandData::ppMesh, Ptr::new(meshes));
        e.call(0x0053_4390, &args![this, 0u32, 2u32, out]);
        assert_eq!(floats(&e, out, 3), vec![0.0, 1.0, 0.0]);

        // No normal stream: the locked geometry data is read at the
        // lock's base and stride.
        let (node, stream) = locking_geometry(&mut e, false);
        for (i, value) in [0.6f32, 0.0, 0.8].into_iter().enumerate() {
            e.mem.set_f32(stream + 12 * 4 + 4 * i as u32, value);
        }
        e.mem.set_u32(meshes, node);
        start_log(&mut e);
        e.call(0x0053_4390, &args![this, 0u32, 4u32, out]);
        let calls = called(&mut e);
        assert_eq!(floats(&e, out, 3), vec![0.6, 0.0, 0.8]);
        assert_eq!(
            calls,
            vec![
                LAND_HAS_VERTEX_ARRAYS,
                GEOMETRY_GET_MODEL_DATA,
                GEOMETRY_GET_NORMALS,
                GEOMETRY_GET_DATA,
                LOCK_CONSTRUCT,
                GEOMETRY_DATA_GET_OWNER,
                0x00aa_0010,
                GEOMETRY_DATA_BEGIN,
                GEOMETRY_DATA_LOCK_POSITIONS,
                LOCK_CHECK_INDEX,
                GEOMETRY_DATA_END
            ]
        );

        // A geometry that cannot be locked gives the default normal.
        let unlockable = lockable_data(&mut e, false);
        let g = geometry(&mut e, 0, 0, 0, unlockable);
        let node = mesh_node(&mut e, g);
        e.mem.set_u32(meshes, node);
        e.call(0x0053_4390, &args![this, 0u32, 4u32, out]);
        assert_eq!(floats(&e, out, 3), vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn lock_helpers_read_the_vertex_stream() {
        let mut e = geometry_engine();
        let lock = e.mem.alloc(16);
        e.mem.set_u32(lock + 8, 0xffff_ffff);
        assert_eq!(e.call(0x0053_4550, &args![lock]).u32(), lock);
        assert_eq!(e.mem.u32(lock), 0);
        assert_eq!(e.mem.u8(lock + 8), 0);
        let stream = e.mem.alloc(64);
        for (i, value) in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0].into_iter().enumerate() {
            e.mem.set_f32(stream + 4 * i as u32, value);
        }
        e.mem.set_u32(lock, stream);
        e.mem.set_u32(lock + 4, 12);
        let out = e.mem.alloc(12);
        start_log(&mut e);
        e.call(0x0053_4570, &args![lock, 1u32, out]);
        assert_eq!(end_log(&mut e), vec![(LOCK_CHECK_INDEX, vec![lock, 1])]);
        assert_eq!(floats(&e, out, 3), vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn color_reader_follows_the_available_sources() {
        let mut e = geometry_engine();
        let out = e.mem.alloc(16);
        // No vertex arrays: the exe's default colour.
        let this = land(&mut e);
        e.call(0x0053_45c0, &args![this, 0u32, 1u32, out]);
        assert_eq!(floats(&e, out, 4), vec![1.0, 1.0, 1.0, 1.0]);

        // ppColorsA.
        let (this, data) = land_with_data(&mut e);
        let colors = e.get(data, LoadedLandData::ppColorsA);
        let block = element(&e, colors, 3);
        for (i, value) in [0.1f32, 0.2, 0.3, 0.4].into_iter().enumerate() {
            e.mem.set_f32(block + 0x10 * 2 + 4 * i as u32, value);
        }
        e.call(0x0053_45c0, &args![this, 3u32, 2u32, out]);
        assert_eq!(floats(&e, out, 4), vec![0.1, 0.2, 0.3, 0.4]);

        // The mesh geometry's colour array.
        e.mem.set_u32(colors.addr() + 12, 0);
        let array = e.mem.alloc(0x40);
        for (i, value) in [0.9f32, 0.8, 0.7, 0.6].into_iter().enumerate() {
            e.mem.set_f32(array + 0x10 + 4 * i as u32, value);
        }
        let g = geometry(&mut e, 0, 0x1000, array, 0);
        let node = mesh_node(&mut e, g);
        let meshes = e.mem.alloc(16);
        e.mem.set_u32(meshes + 12, node);
        e.set(data, LoadedLandData::ppMesh, Ptr::new(meshes));
        e.call(0x0053_45c0, &args![this, 3u32, 1u32, out]);
        assert_eq!(floats(&e, out, 4), vec![0.9, 0.8, 0.7, 0.6]);

        // A locked float stream, and a packed 0xAARRGGBB stream.
        e.register(GEOMETRY_GET_NORMALS, |_, _| ret(0));
        let (node, stream) = locking_geometry(&mut e, false);
        for (i, value) in [0.5f32, 0.4, 0.3, 0.2].into_iter().enumerate() {
            e.mem.set_f32(stream + 0x10 * 2 + 4 * i as u32, value);
        }
        e.mem.set_u32(meshes + 12, node);
        e.call(0x0053_45c0, &args![this, 3u32, 2u32, out]);
        assert_eq!(floats(&e, out, 4), vec![0.5, 0.4, 0.3, 0.2]);
        let (node, stream) = locking_geometry(&mut e, true);
        e.mem.set_u32(stream + 4 * 2, 0x80ff_4000);
        e.mem.set_u32(meshes + 12, node);
        e.call(0x0053_45c0, &args![this, 3u32, 2u32, out]);
        assert_eq!(
            floats(&e, out, 4),
            vec![1.0, byte_fraction(64), 0.0, byte_fraction(128)]
        );

        // Nothing usable: the default colour.
        let unlockable = lockable_data(&mut e, false);
        let g = geometry(&mut e, 0, 0, 0, unlockable);
        let node = mesh_node(&mut e, g);
        e.mem.set_u32(meshes + 12, node);
        e.call(0x0053_45c0, &args![this, 3u32, 2u32, out]);
        assert_eq!(floats(&e, out, 4), vec![1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn packed_color_stream_unpacks_argb() {
        let mut e = geometry_engine();
        e.register(LOCK_CHECK_INDEX, |_, _| Ret::default());
        let lock = e.mem.alloc(16);
        let stream = e.mem.alloc(16);
        e.mem.set_u32(stream + 4, 0x40_ff_80_00);
        e.mem.set_u32(lock, stream);
        e.mem.set_u32(lock + 4, 4);
        e.mem.set_u8(lock + 8, 1);
        let out = e.mem.alloc(16);
        e.call(0x0053_4780, &args![lock, 1u32, out]);
        assert_eq!(
            floats(&e, out, 4),
            vec![1.0, byte_fraction(128), 0.0, byte_fraction(64)]
        );
        // Unpacked: four floats are copied.
        e.mem.set_u8(lock + 8, 0);
        e.mem.set_u32(lock + 4, 16);
        for (i, value) in [0.1f32, 0.2, 0.3, 0.4].into_iter().enumerate() {
            e.mem.set_f32(stream + 16 + 4 * i as u32, value);
        }
        e.mem.set_u32(lock, stream);
        e.call(0x0053_4780, &args![lock, 1u32, out]);
        assert_eq!(floats(&e, out, 4), vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn form_lookup_is_cast_to_a_land() {
        let mut e = engine();
        e.register(FORM_LOOKUP, |_, a| ret(a[2] + 0x100));
        let casts = recorder::<Vec<u32>>();
        let sink = casts.clone();
        e.register_double(RT_DYNAMIC_CAST, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            ret(a[0] + 1)
        });
        let result = e.call(0x0053_4880, &args![0x1000u32, 0xdeadu32, 0x42u32]);
        assert_eq!(result.u32(), 0x143);
        assert_eq!(
            *casts.borrow(),
            vec![vec![0x142, 0, RTTI_TES_FORM, RTTI_TES_OBJECT_LAND, 0]]
        );
    }

    // ---- Copying a land ---------------------------------------------------------

    /// Fills `data`'s blocks with a pattern: block `b` of quadrant `q` gets
    /// the byte `16 * (b + 1) + q`.
    fn fill_blocks(e: &mut Engine, data: Ptr<LoadedLandData>) {
        for (index, (field, size)) in [
            (LoadedLandData::ppVertices.off, 0xd8cu32),
            (LoadedLandData::ppNormals.off, 0xd8c),
            (LoadedLandData::ppColorsA.off, 0x1210),
            (LoadedLandData::ppNormalsSet.off, 0x121),
        ]
        .into_iter()
        .enumerate()
        {
            let array = e.mem.u32(data.addr() + field);
            for quadrant in 0..4u32 {
                let block = e.mem.u32(array + 4 * quadrant);
                let bytes = vec![(16 * (index as u32 + 1) + quadrant) as u8; size as usize];
                e.mem.write(block, &bytes);
            }
        }
        for quadrant in 0..4u32 {
            let percent = e
                .mem
                .u32(data.addr() + LoadedLandData::ppPercentArrays.off + 4 * quadrant);
            let block = e.mem.u32(percent);
            e.mem.write(block, &[0xa0 + quadrant as u8; 0x2420]);
            let layers = e
                .mem
                .u32(data.addr() + LoadedLandData::pQuadTextureArray.off + 4 * quadrant);
            e.mem.write(layers, &[0xb0 + quadrant as u8; 0x18]);
            e.mem.set_u32(
                data.addr() + LoadedLandData::pDefQuadTexture.off + 4 * quadrant,
                0x7000 + quadrant,
            );
        }
    }

    /// Doubles for the unload: no grass map items, no border.
    fn unload_doubles(e: &mut Engine) {
        e.register(SMART_POINTER_RELEASE, |_, _| Ret::default());
        e.register(VECTOR_DESTRUCT_ITERATOR, |_, _| Ret::default());
        e.register(LAND_SET_LOADED, |_, _| Ret::default());
        e.register(LAND_RELEASE_BORDER, |_, _| Ret::default());
        e.register(MAP_REMOVE_ALL, |_, _| Ret::default());
        e.register(HAVOK_REMOVE_REFERENCE, |_, _| Ret::default());
    }

    /// A double for the data allocator that gives its land the arrays.
    fn allocate_data_double(e: &mut Engine) {
        e.register(LAND_ALLOCATE_DATA, |e, a| {
            let (donor, data) = land_with_data(e);
            let _ = donor;
            // The real allocator takes the cell coordinates from the cell.
            e.set(data, LoadedLandData::iCellX, -7);
            e.set(data, LoadedLandData::iCellY, 9);
            e.mem.set_u32(a[0] + 0x28, data.addr());
            Ret::default()
        });
    }

    #[test]
    fn copy_takes_the_blocks_flags_and_extents_of_another_land() {
        let mut e = geometry_engine();
        unload_doubles(&mut e);
        allocate_data_double(&mut e);
        e.register(LAND_IS_LOADED, |_, _| ret(1));
        e.register(LAND_SET_LOADED, |_, _| Ret::default());
        let (source, from) = land_with_data(&mut e);
        fill_blocks(&mut e, from);
        e.set(source, TESObjectLAND::Data, 0x1234);
        e.set(source, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        e.set(from, LoadedLandData::HeightExtentsMin, -5.0);
        e.set(from, LoadedLandData::HeightExtentsMax, 9.0);
        let this = land(&mut e);
        start_log(&mut e);
        e.call(0x0053_48c0, &args![this, source]);
        let log = end_log(&mut e);
        let to = loaded_data(&e, this);
        assert!(!to.is_null());
        assert_ne!(to.addr(), from.addr());
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x1234);
        assert_eq!(e.get(this, TESObjectLAND::pParentCell).addr(), 0x6000);
        assert_eq!(e.get(to, LoadedLandData::HeightExtentsMin), -5.0);
        assert_eq!(e.get(to, LoadedLandData::HeightExtentsMax), 9.0);
        for (index, (field, size)) in [
            (LoadedLandData::ppVertices.off, 0xd8cu32),
            (LoadedLandData::ppNormals.off, 0xd8c),
            (LoadedLandData::ppColorsA.off, 0x1210),
            (LoadedLandData::ppNormalsSet.off, 0x121),
        ]
        .into_iter()
        .enumerate()
        {
            let array = e.mem.u32(to.addr() + field);
            for quadrant in 0..4u32 {
                let block = e.mem.u32(array + 4 * quadrant);
                let bytes = e.mem.bytes(block, size);
                assert!(bytes
                    .iter()
                    .all(|b| *b == (16 * (index as u32 + 1) + quadrant) as u8));
            }
        }
        for quadrant in 0..4u32 {
            let percent = e
                .mem
                .u32(to.addr() + LoadedLandData::ppPercentArrays.off + 4 * quadrant);
            let block = e.mem.u32(percent);
            assert!(e
                .mem
                .bytes(block, 0x2420)
                .iter()
                .all(|b| *b == 0xa0 + quadrant as u8));
            let layers = e
                .mem
                .u32(to.addr() + LoadedLandData::pQuadTextureArray.off + 4 * quadrant);
            assert!(e
                .mem
                .bytes(layers, 0x18)
                .iter()
                .all(|b| *b == 0xb0 + quadrant as u8));
            assert_eq!(
                e.mem
                    .u32(to.addr() + LoadedLandData::pDefQuadTexture.off + 4 * quadrant),
                0x7000 + quadrant
            );
        }
        // The source was loaded, so it is neither loaded nor unloaded here.
        assert_eq!(
            arguments_of(&log, LAND_ALLOCATE_DATA),
            vec![vec![this.addr(), source.addr()]]
        );
        assert_eq!(
            arguments_of(&log, LAND_SET_LOADED),
            vec![vec![this.addr(), 1]]
        );
        assert!(!loaded_data(&e, source).is_null());
    }

    #[test]
    fn copy_loads_and_unloads_a_source_that_was_not_loaded() {
        let mut e = geometry_engine();
        unload_doubles(&mut e);
        allocate_data_double(&mut e);
        e.register(LAND_IS_LOADED, |_, _| ret(0));
        e.register(LOADING_MENU_VISIBLE, |_, _| ret(0));
        let source = land(&mut e);
        let this = land(&mut e);
        start_log(&mut e);
        e.call(0x0053_48c0, &args![this, source]);
        let log = end_log(&mut e);
        // The source got data to copy from (no flags: nothing to read), and
        // lost it again.
        assert_eq!(
            arguments_of(&log, LAND_ALLOCATE_DATA),
            vec![vec![source.addr(), 0], vec![this.addr(), source.addr()]]
        );
        assert!(loaded_data(&e, source).is_null());
        assert!(!loaded_data(&e, this).is_null());
        // A form that is not a land is ignored.
        e.register(RT_DYNAMIC_CAST, |_, _| ret(0));
        let other = land(&mut e);
        start_log(&mut e);
        e.call(0x0053_48c0, &args![other, 0x1234u32]);
        assert_eq!(called(&mut e), vec![RT_DYNAMIC_CAST]);
    }

    // ---- Chunk helpers ------------------------------------------------------------

    #[test]
    fn byte_swaps_cover_the_record_fields() {
        let mut e = engine();
        e.register(SWAP_BYTES_32, |_, _| Ret::default());
        e.register(SWAP_BYTES_16, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0053_5a60, &args![0x1000u32]);
        assert_eq!(
            end_log(&mut e),
            vec![
                (SWAP_BYTES_32, vec![0x1000, 0]),
                (SWAP_BYTES_16, vec![0x1006, 0])
            ]
        );
        start_log(&mut e);
        e.call(0x0053_5aa0, &args![0x2000u32]);
        assert_eq!(
            end_log(&mut e),
            vec![
                (SWAP_BYTES_16, vec![0x2000, 0]),
                (SWAP_BYTES_32, vec![0x2004, 0])
            ]
        );
    }

    #[test]
    fn default_texture_getter_reads_the_global() {
        let mut e = engine();
        e.set_global(DEFAULT_LAND_TEXTURE, 0x9999u32);
        assert_eq!(e.call(0x0053_5ae0, &[]).u32(), 0x9999);
    }

    #[test]
    fn quadrant_node_comes_from_the_cell() {
        let mut e = engine();
        e.register(CELL_GET_NODE, |_, a| ret(a[0].wrapping_add(a[1])));
        let this = land(&mut e);
        assert_eq!(e.call(0x0053_5af0, &args![this, 1u32]).u32(), 0);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x8000));
        assert_eq!(e.call(0x0053_5af0, &args![this, 3u32]).u32(), 0x8003);
        assert_eq!(e.call(0x0053_5af0, &args![this, 4u32]).u32(), 0);
        assert_eq!(e.call(0x0053_5af0, &args![this, -1i32]).u32(), 0x7fff);
    }

    #[test]
    fn world_offset_is_the_cell_centre_at_base_height() {
        let mut e = engine();
        e.register(LAND_BASE_HEIGHT, |_, _| float_ret(33.5));
        let this = land(&mut e);
        let data: Ptr<LoadedLandData> = e.new_object();
        e.set(data, LoadedLandData::iCellX, -2);
        e.set(data, LoadedLandData::iCellY, 3);
        e.set(this, TESObjectLAND::pLoadedData, data.cast());
        let out = e.mem.alloc(12);
        let result = e.call(0x0053_5b30, &args![this, out, 77u32]);
        assert_eq!(result.u32(), out);
        assert_eq!(
            floats(&e, out, 3),
            vec![-8192.0 + 2048.0, 12288.0 + 2048.0, 33.5]
        );
    }

    // ---- Loading ---------------------------------------------------------------

    /// A plugin file scripted as a list of chunks. The doubles answer the
    /// `TESFile` calls: the record type, the current chunk's tag, size and
    /// data, the move to the next chunk, the active flag and the name.
    struct ScriptedFile {
        chunks: Vec<(u32, Vec<u8>)>,
        current: usize,
        /// The sizes the loader asked `GetChunkData` for.
        requested: Vec<u32>,
        record_type: u8,
        swap: bool,
    }

    fn script_file(
        e: &mut Engine,
        chunks: Vec<(u32, Vec<u8>)>,
    ) -> (u32, Rc<RefCell<ScriptedFile>>) {
        let file = e.mem.alloc(16);
        let script = Rc::new(RefCell::new(ScriptedFile {
            chunks,
            current: 0,
            requested: vec![],
            record_type: b'B',
            swap: false,
        }));
        let state = script.clone();
        e.register_double(FILE_GET_TES_FORM, move |_, _| {
            ret(state.borrow().record_type as u32)
        });
        let state = script.clone();
        e.register_double(FILE_GET_CHUNK, move |_, _| {
            let s = state.borrow();
            ret(s.chunks[s.current].0)
        });
        let state = script.clone();
        e.register_double(FILE_NEXT_CHUNK, move |_, _| {
            let mut s = state.borrow_mut();
            s.current += 1;
            ret((s.current < s.chunks.len()) as u32)
        });
        let state = script.clone();
        e.register_double(FILE_GET_CHUNK_SIZE, move |_, _| {
            let s = state.borrow();
            ret(s.chunks[s.current].1.len() as u32)
        });
        let state = script.clone();
        e.register_double(FILE_GET_CHUNK_DATA, move |e, a| {
            let mut s = state.borrow_mut();
            s.requested.push(a[2]);
            let bytes = s.chunks[s.current].1.clone();
            e.mem.write(a[1], &bytes);
            Ret::default()
        });
        let state = script.clone();
        e.register_double(FILE_NEEDS_SWAP, move |_, _| ret(state.borrow().swap as u32));
        e.register(FILE_GET_ACTIVE, |_, _| ret(1));
        e.register(FILE_GET_NAME, |e, _| {
            let name = e.mem.alloc(16);
            e.mem.set_cstr(name, b"Fallout.esm");
            ret(name)
        });
        (file, script)
    }

    /// A `VHGT` chunk: the base height and 0x441 signed steps, all zero
    /// except the `(index, step)` pairs given.
    fn height_chunk(base: f32, steps: &[(usize, i8)]) -> Vec<u8> {
        let mut bytes = base.to_le_bytes().to_vec();
        bytes.resize(4 + CHUNK_VERTICES, 0);
        for (index, step) in steps {
            bytes[4 + index] = *step as u8;
        }
        bytes.resize(0x448, 0);
        bytes
    }

    /// A `VNML` or `VCLR` chunk of 33 x 33 triples, zero except the
    /// `(grid index, triple)` pairs given.
    fn triple_chunk(entries: &[(usize, [u8; 3])]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0xcc3];
        for (index, triple) in entries {
            bytes[3 * index..3 * index + 3].copy_from_slice(triple);
        }
        bytes
    }

    fn record(form_id: u32, quadrant: u8, layer: u16) -> Vec<u8> {
        let mut bytes = form_id.to_le_bytes().to_vec();
        bytes.push(quadrant);
        bytes.push(0);
        bytes.extend_from_slice(&layer.to_le_bytes());
        bytes
    }

    fn vertex_texture_chunk(entries: &[(u16, f32)]) -> Vec<u8> {
        let mut bytes = vec![];
        for (position, opacity) in entries {
            bytes.extend_from_slice(&position.to_le_bytes());
            bytes.extend_from_slice(&[0, 0]);
            bytes.extend_from_slice(&opacity.to_le_bytes());
        }
        bytes
    }

    /// The doubles the chunk readers call.
    fn loader_engine() -> Engine {
        let mut e = geometry_engine();
        allocate_data_double(&mut e);
        unload_doubles(&mut e);
        e.set_global(X_OFFSETS, -2048.0f32);
        e.set_global(X_OFFSETS + 4, 0.0f32);
        e.set_global(X_OFFSETS + 8, -2048.0f32);
        e.set_global(X_OFFSETS + 12, 0.0f32);
        e.set_global(Y_OFFSETS, -2048.0f32);
        e.set_global(Y_OFFSETS + 4, -2048.0f32);
        e.set_global(Y_OFFSETS + 8, 0.0f32);
        e.set_global(Y_OFFSETS + 12, 0.0f32);
        // `FISTP`: round to nearest.
        e.register(FLOAT_TO_INT, |_, a| {
            ret(f32::from_bits(a[0]).round() as i32 as u32)
        });
        e.register(NI_POINT3_NORMALIZE, |_, _| Ret::default());
        e.register(LAND_SET_LOADED, |_, _| Ret::default());
        e.register(LAND_FLUSH_WARNINGS, |_, _| Ret::default());
        e.register(LOADING_MASTER_FILE, |_, _| ret(0));
        e.register(FORM_LOAD, |_, _| Ret::default());
        e.register(CELL_GET_DATA_X, |_, _| ret(-7i32 as u32));
        e.register(CELL_GET_DATA_Y, |_, _| ret(9));
        e.register(LOG_MESSAGE, |_, _| Ret::default());
        e.register(HAVOK_ADD_REFERENCE, |_, _| Ret::default());
        e.register(FORM_ADD_COMPILE_INDEX, |_, _| Ret::default());
        e.register(QUEUED_FILE_CONSTRUCT, |_, a| ret(a[0]));
        e.register(QUEUED_FILE_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(LAND_TEXTURE_QUEUE, |_, _| Ret::default());
        e.register(LAND_SET_VERTEX_OPACITY, |_, _| Ret::default());
        e.register(LAND_BUILD_MOPP, |e, a| {
            e.mem.set_u32(a[3], 0x5555);
            Ret::default()
        });
        e
    }

    /// The quadrant vertex `k` of the block of `quadrant` of `array`.
    fn block_vertex(e: &Engine, array: Ptr, quadrant: u32, k: u32, size: u32) -> u32 {
        element(e, array, quadrant) + size * k
    }

    #[test]
    fn load_reads_geometry_and_textures_from_the_record() {
        let mut e = loader_engine();
        e.set_global(DEFAULT_LAND_TEXTURE, 0x8888u32);
        // `FindForm` is unused here: the form ids resolve through lookups.
        e.register(FORM_LOOKUP_BY_ID, |_, a| {
            ret(if a[0] == 0x1234 { 0x2000 } else { 0 })
        });
        let queued = recorder::<Vec<u32>>();
        let sink = queued.clone();
        e.register_double(LAND_TEXTURE_QUEUE, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let opacities = recorder::<Vec<u32>>();
        let sink = opacities.clone();
        e.register_double(LAND_SET_VERTEX_OPACITY, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let messages = recorder::<Vec<u32>>();
        let sink = messages.clone();
        e.register_double(LOG_MESSAGE, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let mopp = recorder::<Vec<u32>>();
        let sink = mopp.clone();
        e.register_double(LAND_BUILD_MOPP, move |e, a| {
            sink.borrow_mut().push(a.to_vec());
            e.mem.set_u32(a[3], 0x5555);
            Ret::default()
        });
        let chunks = vec![
            (CHUNK_DATA, 7u32.to_le_bytes().to_vec()),
            // Row 5 from column 3 on is 4 higher than the rest.
            (
                CHUNK_HEIGHTS,
                height_chunk(100.0, &[(0, 2), (5 * 33 + 3, 4)]),
            ),
            // The normal at grid 16 (quadrant 1, vertex 0) is (127, 0, -127).
            (CHUNK_NORMALS, triple_chunk(&[(16, [127, 0, 0x81])])),
            // The colour at grid 529 (quadrant 2, vertex 1).
            (CHUNK_COLORS, triple_chunk(&[(529, [255, 0, 51])])),
            (CHUNK_MOPP, vec![1, 2, 3, 4, 5, 6, 7, 8]),
            // The base texture of quadrant 3 is unknown.
            (CHUNK_BASE_TEXTURE, record(0x4321, 3, 0)),
            // A layer of quadrant 2 with its vertex opacities.
            (CHUNK_ADDITIONAL_TEXTURE, record(0x1234, 2, 3)),
            (
                CHUNK_VERTEX_TEXTURE,
                vertex_texture_chunk(&[(7, 50.0), (8, 0.75)]),
            ),
            // A layer without texture (the default one) with an invalid
            // index, and vertex data of the wrong size.
            (CHUNK_ADDITIONAL_TEXTURE, record(0, 1, 9)),
            (CHUNK_VERTEX_TEXTURE, vec![0; 7]),
            (tag(b"ZZZZ"), vec![]),
        ];
        let (file, script) = script_file(&mut e, chunks);
        let this = land(&mut e);
        start_log(&mut e);
        let loaded = e.call(0x0053_4ad0, &args![this, file]);
        assert_eq!(loaded.u32() & 0xff, 1);
        let log = end_log(&mut e);
        let data = loaded_data(&e, this);
        // DATA replaced the flags (the world space bit stays clear), and the
        // MOPP chunk set bit 0x800.
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x807);
        assert_eq!(*script.borrow().requested.first().unwrap(), 4);
        // The heights: 102 everywhere, 106 in row 5 from column 3; each
        // rounded and times 8, relative to their mean.
        assert_eq!(e.get(data, LoadedLandData::HeightExtentsMin), 816.0);
        assert_eq!(e.get(data, LoadedLandData::HeightExtentsMax), 848.0);
        assert_eq!(e.get(data, LoadedLandData::fBaseHeight), 832.0);
        let vertices = e.get(data, LoadedLandData::ppVertices);
        let vertex = block_vertex(&e, vertices, 0, 5 * 17 + 3, 12);
        assert_eq!(floats(&e, vertex, 3), vec![-1664.0, -1408.0, 16.0]);
        let vertex = block_vertex(&e, vertices, 0, 0, 12);
        assert_eq!(floats(&e, vertex, 3), vec![-2048.0, -2048.0, -16.0]);
        let vertex = block_vertex(&e, vertices, 3, 0x120, 12);
        assert_eq!(floats(&e, vertex, 3), vec![2048.0, 2048.0, -16.0]);
        // Normals and colours.
        let normals = e.get(data, LoadedLandData::ppNormals);
        let normal = block_vertex(&e, normals, 1, 0, 12);
        assert_eq!(floats(&e, normal, 3), vec![1.0, 0.0, -1.0]);
        let colors = e.get(data, LoadedLandData::ppColorsA);
        let color = block_vertex(&e, colors, 2, 1, 0x10);
        assert_eq!(floats(&e, color, 4), vec![1.0, 0.0, byte_fraction(51), 1.0]);
        // The MOPP code was built from the chunk and referenced.
        assert_eq!(e.get(data, LoadedLandData::pMoppCode).addr(), 0x5555);
        assert_eq!(mopp.borrow().len(), 1);
        assert_eq!(mopp.borrow()[0][2], 8);
        // The textures: the unknown base texture is null, the found layer
        // texture is in quadrant 2 layer 3, the clamped one in quadrant 1
        // layer 5 holds the default texture.
        assert_eq!(
            e.mem
                .u32(data.addr() + LoadedLandData::pDefQuadTexture.off + 12),
            0
        );
        let layers = e
            .mem
            .u32(data.addr() + LoadedLandData::pQuadTextureArray.off + 8);
        assert_eq!(e.mem.u32(layers + 12), 0x2000);
        let layers = e
            .mem
            .u32(data.addr() + LoadedLandData::pQuadTextureArray.off + 4);
        assert_eq!(e.mem.u32(layers + 20), 0x8888);
        let queued_file = e.mem.u32(this.addr() + 0x24);
        assert_ne!(queued_file, 0);
        assert_eq!(
            *queued.borrow(),
            vec![vec![0x2000, 5, queued_file], vec![0x8888, 5, queued_file]]
        );
        // The vertex opacities of the layer: 50 is divided by 100.
        assert_eq!(
            *opacities.borrow(),
            vec![
                vec![this.addr(), 2, 7, 3, 0.5f32.to_bits()],
                vec![this.addr(), 2, 8, 3, 0.75f32.to_bits()]
            ]
        );
        // Messages: base texture missing, clamped layer, bad vertex data.
        let (x, y) = (-7i32 as u32, 9u32);
        let messages = messages.borrow();
        assert_eq!(messages.len(), 3);
        assert_eq!(
            messages[0],
            vec![MASTER_FILE_MESSAGES.missing_base, x, y, 0x4321, 3]
        );
        assert_eq!(
            messages[1],
            vec![MASTER_FILE_MESSAGES.clamped_layer, x, y, 9, 1]
        );
        assert_eq!(
            messages[2][..3],
            [MASTER_FILE_MESSAGES.unrecognized_data, x, y]
        );
        assert_eq!(read_cstring(&e, messages[2][3]), "Fallout.esm");
        assert_eq!(
            arguments_of(&log, LAND_FLUSH_WARNINGS),
            vec![vec![this.addr()]]
        );
        // The land counts as loaded with its geometry.
        assert_eq!(
            arguments_of(&log, LAND_SET_LOADED).last().unwrap(),
            &vec![this.addr(), 1]
        );
        assert_eq!(arguments_of(&log, NI_POINT3_NORMALIZE).len(), 4 * 0x121);
    }

    #[test]
    fn load_ignores_records_of_other_types() {
        let mut e = loader_engine();
        let (file, script) = script_file(&mut e, vec![(CHUNK_DATA, vec![0; 4])]);
        script.borrow_mut().record_type = b'C';
        let this = land(&mut e);
        start_log(&mut e);
        let result = e.call(0x0053_4ad0, &args![this, file]);
        assert_eq!(result.u32() & 0xff, 0);
        let log = end_log(&mut e);
        assert_eq!(
            log.iter().map(|(a, _)| *a).collect::<Vec<_>>(),
            vec![SCOPE_GUARD_ENTER, FILE_GET_TES_FORM, SCOPE_GUARD_LEAVE]
        );
        assert_eq!(log[0].1[4], 0x2c2);
    }

    #[test]
    fn load_skips_geometry_for_a_land_of_the_world_space_file() {
        let mut e = loader_engine();
        let (file, _) = script_file(
            &mut e,
            vec![
                (CHUNK_DATA, (0x7u32).to_le_bytes().to_vec()),
                (CHUNK_HEIGHTS, height_chunk(0.0, &[])),
                (CHUNK_BASE_TEXTURE, record(0x1, 0, 0)[..8].to_vec()),
            ],
        );
        let this = land(&mut e);
        e.set(this, TESObjectLAND::Data, 0x400);
        start_log(&mut e);
        e.call(0x0053_4ad0, &args![this, file]);
        let log = end_log(&mut e);
        // Flag 0x400 is kept by DATA; no data was allocated and nothing
        // else was read.
        assert_eq!(e.get(this, TESObjectLAND::Data), 0x407 | 0x400);
        assert!(arguments_of(&log, LAND_ALLOCATE_DATA).is_empty());
        assert_eq!(arguments_of(&log, FILE_GET_CHUNK_DATA).len(), 1);
        assert_eq!(
            arguments_of(&log, LAND_SET_LOADED).last().unwrap(),
            &vec![this.addr(), 0]
        );
    }

    #[test]
    fn load_swaps_the_chunks_of_a_big_endian_file() {
        let mut e = loader_engine();
        e.register(SWAP_WORD, |e, a| {
            let bytes = e.mem.u32(a[0]).swap_bytes();
            e.mem.set_u32(a[0], bytes);
            Ret::default()
        });
        e.register(SWAP_BYTES_32, |e, a| {
            let value = e.mem.u32(a[0]).swap_bytes();
            e.mem.set_u32(a[0], value);
            Ret::default()
        });
        e.register(SWAP_BYTES_16, |e, a| {
            let value = e.mem.u16(a[0]).swap_bytes();
            e.mem.set_u16(a[0], value);
            Ret::default()
        });
        e.register(FORM_LOOKUP_BY_ID, |_, a| ret(a[0] + 1));
        let mut flags = 1u32.to_be_bytes().to_vec();
        flags.truncate(4);
        let mut heights = 100.0f32.to_be_bytes().to_vec();
        heights.resize(0x448, 0);
        let mut layer = 0x0102_0304u32.to_be_bytes().to_vec();
        layer.extend_from_slice(&[1, 0]);
        layer.extend_from_slice(&2u16.to_be_bytes());
        let (file, script) = script_file(
            &mut e,
            vec![
                (CHUNK_DATA, flags),
                (CHUNK_HEIGHTS, heights),
                (CHUNK_ADDITIONAL_TEXTURE, layer),
            ],
        );
        script.borrow_mut().swap = true;
        let this = land(&mut e);
        e.call(0x0053_4ad0, &args![this, file]);
        let data = loaded_data(&e, this);
        // The flags were swapped, and so was the base height (all heights are
        // 100, which is 800 after the scaling).
        assert_eq!(e.get(this, TESObjectLAND::Data), 1);
        assert_eq!(e.get(data, LoadedLandData::fBaseHeight), 800.0);
        // The layer record: form id swapped, layer swapped; texture 1 + id.
        let layers = e
            .mem
            .u32(data.addr() + LoadedLandData::pQuadTextureArray.off + 4);
        assert_eq!(e.mem.u32(layers + 8), 0x0102_0305);
    }

    #[test]
    fn load_stores_the_vertices_of_each_quadrant_on_its_own_block() {
        let mut e = loader_engine();
        // Steps that make the rows differ: row r is 10 * r higher than the
        // first (each row starts from the first height of the previous one,
        // so the row's first step is the difference).
        let steps: Vec<(usize, i8)> = (1..33).map(|row| (row * 33, 10)).collect();
        let (file, _) = script_file(
            &mut e,
            vec![
                (CHUNK_DATA, 1u32.to_le_bytes().to_vec()),
                (CHUNK_HEIGHTS, height_chunk(0.0, &steps)),
            ],
        );
        let this = land(&mut e);
        e.call(0x0053_4ad0, &args![this, file]);
        let data = loaded_data(&e, this);
        let vertices = e.get(data, LoadedLandData::ppVertices);
        // Row r (0..33) is at height 10 r, so z = 80 r minus the mean
        // (80 * 16 = 1280 for the full grid: min 0, max 2560).
        for (quadrant, row, column) in [(0u32, 3u32, 4u32), (1, 3, 4), (2, 3, 4), (3, 16, 16)] {
            let k = row * 17 + column;
            let vertex = block_vertex(&e, vertices, quadrant, k, 12);
            let grid_row = (quadrant / 2) * 16 + row;
            assert_eq!(e.mem.f32(vertex + 8), 80.0 * grid_row as f32 - 1280.0);
        }
    }

    // ---- LoadVerticesIntoArrays and LoadVertices ---------------------------------

    /// The doubles around the file lookups `LoadVerticesIntoArrays` makes.
    fn file_lookup_engine(file: u32) -> Engine {
        let mut e = loader_engine();
        e.register(LAND_IS_LOADED, |_, _| ret(0));
        e.register(FORM_GET_FILE, |_, _| ret(0x7700));
        e.register_double(FILE_GET_THREAD_SAFE_FILE, move |_, _| ret(file));
        e.register(FILE_FIND_FORM, |_, _| ret(1));
        e.register(FILE_RECORD_OFFSET, |_, _| ret(0x1000));
        e.register(FILE_SET_OFFSET, |_, _| ret(1));
        e.register(DISABLE_WARNINGS, |_, _| Ret::default());
        e.register(CELL_GET_WORLD_SPACE, |_, _| ret(0x4000));
        e
    }

    #[test]
    fn arrays_loader_returns_early_when_there_is_nothing_to_read() {
        let mut e = file_lookup_engine(0);
        // Already loaded.
        e.register(LAND_IS_LOADED, |_, _| ret(1));
        let this = land(&mut e);
        start_log(&mut e);
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 1);
        assert_eq!(called(&mut e), vec![LAND_IS_LOADED]);
        // No flags: no file is looked up.
        e.register(LAND_IS_LOADED, |_, _| ret(0));
        start_log(&mut e);
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 1);
        assert_eq!(called(&mut e), vec![LAND_IS_LOADED]);
        // A land of the world space's land file without a world space.
        e.register(CELL_GET_WORLD_SPACE, |_, _| ret(0));
        e.set(this, TESObjectLAND::Data, 0x400);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 1);
        // The world space has no data for the cell.
        e.register(CELL_GET_WORLD_SPACE, |_, _| ret(0x4000));
        e.register(WORLD_SPACE_FIND_LAND_DATA, |_, _| ret(0));
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 1);
    }

    #[test]
    fn arrays_loader_reports_a_form_that_is_missing_from_its_file() {
        let mut e = file_lookup_engine(0);
        let (this, _) = land_with_data(&mut e);
        e.set(this, TESObjectLAND::Data, 1);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        e.mem.set_u32(this.addr() + 0xc, 0xabc);
        let messages = recorder::<Vec<u32>>();
        let sink = messages.clone();
        e.register_double(LOG_MESSAGE, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        // The form's file has no thread-safe copy: the name is "UNKNOWN".
        e.register(FILE_FIND_FORM, |_, _| ret(0));
        start_log(&mut e);
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 0);
        let log = end_log(&mut e);
        let addresses: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            addresses,
            vec![
                LAND_IS_LOADED,
                FORM_GET_FILE,
                FILE_GET_THREAD_SAFE_FILE,
                FILE_FIND_FORM,
                DISABLE_WARNINGS,
                PARENT_CELL,
                CELL_GET_DATA_Y,
                PARENT_CELL,
                CELL_GET_DATA_X,
                FORM_ID,
                LOG_MESSAGE,
                DISABLE_WARNINGS
            ]
        );
        assert_eq!(log[1].1, vec![this.addr(), 0xffff_ffff]);
        assert_eq!(log[4].1, vec![0]);
        assert_eq!(log[11].1, vec![1]);
        assert_eq!(
            *messages.borrow(),
            vec![vec![
                LAND_LOAD_FAILED,
                0xabc,
                -7i32 as u32,
                9,
                UNKNOWN_FILE_NAME
            ]]
        );
        // A file that is not a land record names the file.
        let (file, script) = script_file(&mut e, vec![(CHUNK_DATA, vec![0; 4])]);
        e.register_double(FILE_GET_THREAD_SAFE_FILE, move |_, _| ret(file));
        e.register(FILE_FIND_FORM, |_, _| ret(1));
        script.borrow_mut().record_type = b'X';
        messages.borrow_mut().clear();
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 0);
        let messages = messages.borrow();
        assert_eq!(messages.len(), 1);
        assert_eq!(read_cstring(&e, messages[0][4]), "Fallout.esm");
    }

    #[test]
    fn arrays_loader_stops_when_the_offset_cannot_be_set() {
        let mut e = file_lookup_engine(0);
        let (file, _) = script_file(&mut e, vec![(CHUNK_DATA, vec![0; 4])]);
        e.register_double(FILE_GET_THREAD_SAFE_FILE, move |_, _| ret(file));
        let seen = recorder::<Vec<u32>>();
        let sink = seen.clone();
        e.register_double(FILE_SET_OFFSET, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            ret(0)
        });
        let (this, _) = land_with_data(&mut e);
        e.set(this, TESObjectLAND::Data, 4);
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 0);
        assert_eq!(*seen.borrow(), vec![vec![file, 0x1000]]);
    }

    #[test]
    fn arrays_loader_reads_the_chunks_of_a_world_space_land() {
        let mut e = loader_engine();
        e.register(FORM_LOOKUP_BY_ID, |_, a| {
            ret(if a[0] == 0x1234 { 0x2000 } else { 0 })
        });
        let chunks = vec![
            // Low bits 5: heights and textures.
            (CHUNK_DATA, 0xffff_fff5u32.to_le_bytes().to_vec()),
            (CHUNK_MOPP, vec![9; 4]),
            (CHUNK_HEIGHTS, height_chunk(100.0, &[(0, 2)])),
            // Colours are not in the flags: not read.
            (CHUNK_COLORS, triple_chunk(&[(0, [255, 255, 255])])),
            (CHUNK_BASE_TEXTURE, record(0x4321, 0, 0)),
            (CHUNK_ADDITIONAL_TEXTURE, record(0x1234, 1, 2)),
            (CHUNK_VERTEX_TEXTURE, vertex_texture_chunk(&[(3, 50.0)])),
            (tag(b"ZZZZ"), vec![]),
        ];
        let (file, script) = script_file(&mut e, chunks);
        e.register(LAND_IS_LOADED, |_, _| ret(0));
        e.register(CELL_GET_WORLD_SPACE, |_, _| ret(0x4000));
        e.register(FILE_SET_OFFSET, |_, _| ret(1));
        e.register(DISABLE_WARNINGS, |_, _| Ret::default());
        let found = recorder::<Vec<u32>>();
        let sink = found.clone();
        e.register_double(WORLD_SPACE_FIND_LAND_DATA, move |e, a| {
            sink.borrow_mut().push(a.to_vec());
            e.mem.set_u32(a[3], file);
            e.mem.set_u32(a[4], 0x1000);
            ret(1)
        });
        let opacities = recorder::<Vec<u32>>();
        let sink = opacities.clone();
        e.register_double(LAND_SET_VERTEX_OPACITY, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let messages = recorder::<Vec<u32>>();
        let sink = messages.clone();
        e.register_double(LOG_MESSAGE, move |_, a| {
            sink.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let mopp = recorder::<Vec<u32>>();
        let sink = mopp.clone();
        e.register_double(LAND_BUILD_MOPP, move |e, a| {
            sink.borrow_mut().push(a.to_vec());
            e.mem.set_u32(a[3], 0x5555);
            Ret::default()
        });
        let (this, data) = land_with_data(&mut e);
        e.set(data, LoadedLandData::iCellX, -7);
        e.set(data, LoadedLandData::iCellY, 9);
        e.set(this, TESObjectLAND::Data, 0x410);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        start_log(&mut e);
        assert_eq!(e.call(0x0053_5d00, &args![this]).u32() & 0xff, 1);
        let log = end_log(&mut e);
        // The cell's land data was looked up with its coordinates.
        assert_eq!(found.borrow().len(), 1);
        assert_eq!(found.borrow()[0][..3], [0x4000, -7i32 as u32, 9]);
        assert_eq!(
            arguments_of(&log, FILE_SET_OFFSET),
            vec![vec![file, 0x1000]]
        );
        // DATA only merges the low three bits; the MOPP chunk sets 0x800.
        assert_eq!(e.get(this, TESObjectLAND::Data), 0xc15);
        // The MOPP code was read without a flag.
        assert_eq!(mopp.borrow().len(), 1);
        assert_eq!(e.get(data, LoadedLandData::pMoppCode).addr(), 0x5555);
        // Heights were read (102 everywhere, 816 after the scaling).
        assert_eq!(e.get(data, LoadedLandData::fBaseHeight), 816.0);
        // The colour chunk was skipped: the data requests of the file were
        // DATA (4), MPCD (4), VHGT (0x448), BTXT (8), ATXT (8), VTXT (8).
        assert_eq!(script.borrow().requested, vec![4, 4, 0x448, 8, 8, 8]);
        // The layer texture of quadrant 1, layer 2, and its opacity (the
        // value is not divided by 100).
        let layers = e
            .mem
            .u32(data.addr() + LoadedLandData::pQuadTextureArray.off + 4);
        assert_eq!(e.mem.u32(layers + 8), 0x2000);
        assert_eq!(
            *opacities.borrow(),
            vec![vec![this.addr(), 1, 3, 2, 50.0f32.to_bits()]]
        );
        // The missing base texture is logged with the "TEXTURES" message and
        // flushes the warnings; heights without normals are logged too.
        let messages = messages.borrow();
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[0],
            vec![CELL_MESSAGES.missing_base, -7i32 as u32, 9, 0x4321, 0]
        );
        assert_eq!(messages[1][..3], [NO_NORMAL_DATA, -7i32 as u32, 9]);
        assert_eq!(read_cstring(&e, messages[1][3]), "Fallout.esm");
        assert_eq!(
            arguments_of(&log, LAND_FLUSH_WARNINGS),
            vec![vec![this.addr()]]
        );
        // Unlike `Load`, this loader does not mark the land loaded.
        assert!(arguments_of(&log, LAND_SET_LOADED).is_empty());
    }

    #[test]
    fn load_vertices_builds_meshes_and_reads_what_is_needed() {
        let mut e = loader_engine();
        e.register(LOADING_MENU_VISIBLE, |_, _| ret(0));
        e.register(LAND_IS_LOADED, |_, _| ret(0));
        e.register(FORM_GET_FILE, |_, _| ret(0));
        e.register(CELL_GET_3D, |_, _| ret(0));
        // A land with no flags: data is allocated, nothing is read.
        let this = land(&mut e);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        start_log(&mut e);
        let result = e.call(0x0053_5b90, &args![this, 1u32]);
        assert_eq!(result.u32() & 0xff, 1);
        let log = end_log(&mut e);
        let addresses: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            addresses,
            vec![
                SCOPE_GUARD_ENTER,
                LOADING_MENU_VISIBLE,
                LAND_IS_LOADED,
                LAND_ALLOCATE_DATA,
                CELL_GET_3D,
                SCOPE_GUARD_LEAVE
            ]
        );
        assert_eq!(log[0].1[4], 0x455);
        assert_eq!(log[3].1, vec![this.addr(), 0]);
        // Without the mesh request nothing is built.
        start_log(&mut e);
        e.call(0x0053_5b90, &args![this, 0u32]);
        assert!(!called(&mut e).contains(&CELL_GET_3D));
        // With a land whose record has heights and a file, the arrays are
        // read (here the world space has no data: nothing to read) and the
        // land is marked loaded.
        e.register(FORM_GET_FILE, |_, _| ret(0x7700));
        e.set(this, TESObjectLAND::Data, 1);
        e.register(LAND_SET_LOADED, |_, _| Ret::default());
        e.register(FILE_GET_THREAD_SAFE_FILE, |_, _| ret(0));
        e.register(FILE_FIND_FORM, |_, _| ret(0));
        e.register(DISABLE_WARNINGS, |_, _| Ret::default());
        e.register(CELL_GET_DATA_X, |_, _| ret(1));
        e.register(CELL_GET_DATA_Y, |_, _| ret(2));
        start_log(&mut e);
        let result = e.call(0x0053_5b90, &args![this, 1u32]);
        assert_eq!(result.u32() & 0xff, 0, "the arrays loader failed");
        let log = end_log(&mut e);
        let addresses: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert!(addresses.contains(&FILE_FIND_FORM));
        assert_eq!(
            arguments_of(&log, LAND_SET_LOADED),
            vec![vec![this.addr(), 1]]
        );
        assert!(
            addresses.contains(&CELL_GET_3D),
            "the meshes are still built"
        );
    }

    #[test]
    fn load_vertices_of_a_loaded_land_only_builds_missing_meshes() {
        let mut e = loader_engine();
        e.register(LOADING_MENU_VISIBLE, |_, _| ret(0));
        e.register(LAND_IS_LOADED, |_, _| ret(1));
        e.register(CELL_GET_3D, |_, _| ret(0));
        let (this, data) = land_with_data(&mut e);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        start_log(&mut e);
        assert_eq!(e.call(0x0053_5b90, &args![this, 1u32]).u32() & 0xff, 1);
        assert!(called(&mut e).contains(&CELL_GET_3D));
        // The meshes exist already.
        e.set(data, LoadedLandData::ppMesh, Ptr::new(0x1234));
        start_log(&mut e);
        e.call(0x0053_5b90, &args![this, 1u32]);
        assert!(!called(&mut e).contains(&CELL_GET_3D));
        // The loading menu forbids building them.
        e.set(data, LoadedLandData::ppMesh, Ptr::NULL);
        e.register(LOADING_MENU_VISIBLE, |_, _| ret(1));
        start_log(&mut e);
        e.call(0x0053_5b90, &args![this, 1u32]);
        assert!(!called(&mut e).contains(&CELL_GET_3D));
    }

    #[test]
    fn load_vertices_reads_a_world_space_land_through_the_arrays_loader() {
        let mut e = loader_engine();
        e.register(LOADING_MENU_VISIBLE, |_, _| ret(0));
        e.register(LAND_IS_LOADED, |_, _| ret(0));
        e.register(CELL_GET_WORLD_SPACE, |_, _| ret(0));
        e.register(CELL_GET_3D, |_, _| ret(0));
        let this = land(&mut e);
        e.set(this, TESObjectLAND::Data, 0x400);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        start_log(&mut e);
        assert_eq!(e.call(0x0053_5b90, &args![this, 1u32]).u32() & 0xff, 1);
        let addresses = called(&mut e);
        assert_eq!(
            addresses,
            vec![
                SCOPE_GUARD_ENTER,
                LOADING_MENU_VISIBLE,
                LAND_IS_LOADED,
                LAND_ALLOCATE_DATA,
                LAND_IS_LOADED,
                CELL_GET_WORLD_SPACE,
                LAND_SET_LOADED,
                CELL_GET_3D,
                SCOPE_GUARD_LEAVE
            ]
        );
    }

    // ---- Unloading ----------------------------------------------------------------

    #[test]
    fn unload_frees_every_block_and_the_data() {
        let mut e = loader_engine();
        e.register(HAVOK_REMOVE_REFERENCE, |_, _| Ret::default());
        let (this, data) = land_with_data(&mut e);
        e.set(data, LoadedLandData::pMoppCode, Ptr::new(0x5555));
        // The grass map of quadrant 1 has two items of 16 pointers; the
        // first pointer of each item is a live block.
        let items: Vec<(u32, u32)> = (0..2)
            .map(|_| {
                let item = e.mem.alloc(64);
                let pointer = e.mem.alloc(8);
                e.mem.set_u32(item + 8, pointer);
                (item, pointer)
            })
            .collect();
        let map = data.addr() + GRASS_MAP_OFFSET + 0x10;
        e.mem.set_u32(map + 0xc, 2);
        e.register(MAP_FIRST_POSITION, |_, _| ret(1));
        let walk = items.clone();
        e.register_double(MAP_GET_NEXT, move |e, a| {
            // `a` = map, &position, &key, &value.
            let position = e.mem.u32(a[1]);
            e.mem.set_u32(a[3], walk[position as usize - 1].0);
            let next = if position < 2 { position + 1 } else { 0 };
            e.mem.set_u32(a[1], next);
            Ret::default()
        });
        let removed = recorder::<u32>();
        let sink = removed.clone();
        e.register_double(MAP_REMOVE_ALL, move |_, a| {
            sink.borrow_mut().push(a[0]);
            Ret::default()
        });
        let vertices_array = e.get(data, LoadedLandData::ppVertices);
        let vertices_block = element(&e, vertices_array, 2);
        let percent = e
            .mem
            .u32(data.addr() + LoadedLandData::ppPercentArrays.off + 12);
        let percent_block = e.mem.u32(percent);
        let arrays: Vec<u32> = [
            LoadedLandData::ppNormalsSet.off,
            LoadedLandData::ppVertices.off,
            LoadedLandData::ppColorsA.off,
            LoadedLandData::ppNormals.off,
        ]
        .iter()
        .map(|field| e.mem.u32(data.addr() + field))
        .collect();
        // The data's destructor body: nothing to release.
        e.register(SMART_POINTER_RELEASE, |_, _| Ret::default());
        e.register(VECTOR_DESTRUCT_ITERATOR, |_, _| Ret::default());
        start_log(&mut e);
        let result = e.call(0x0053_6d80, &args![this]);
        assert_eq!(result.u32() & 0xff, 1);
        let log = end_log(&mut e);
        assert_eq!(log[0], (LAND_SET_LOADED, vec![this.addr(), 0]));
        assert_eq!(log[1], (LAND_RELEASE_BORDER, vec![this.addr()]));
        assert!(loaded_data(&e, this).is_null());
        // The grass map items were emptied and freed.
        let freed: Vec<u32> = arguments_of(&log, DEALLOCATE)
            .into_iter()
            .map(|a| a[0])
            .collect();
        for (item, pointer) in &items {
            assert!(freed.contains(item) && freed.contains(pointer));
        }
        assert_eq!(
            *removed.borrow(),
            (0..4).map(|q| map - 0x10 + 0x10 * q).collect::<Vec<_>>()
        );
        // Percent: its first array, then the array itself.
        let at = freed.iter().position(|a| *a == percent_block).unwrap();
        assert_eq!(freed[at + 1], percent);
        assert!(freed.contains(&vertices_block));
        // The four block arrays are freed in this order after the quadrants,
        // then the data itself last.
        let tail: Vec<u32> = freed[freed.len() - 5..].to_vec();
        assert_eq!(tail[..4], arrays[..]);
        assert_eq!(tail[4], data.addr());
        // The MOPP code was released.
        assert_eq!(
            arguments_of(&log, HAVOK_REMOVE_REFERENCE),
            vec![vec![0x5555]]
        );
    }

    #[test]
    fn unload_of_a_land_without_data_only_clears_the_state() {
        let mut e = loader_engine();
        let this = land(&mut e);
        start_log(&mut e);
        assert_eq!(e.call(0x0053_6d80, &args![this]).u32() & 0xff, 1);
        assert_eq!(called(&mut e), vec![LAND_SET_LOADED, LAND_RELEASE_BORDER]);
    }

    #[test]
    fn data_destructor_releases_its_members() {
        let mut e = engine();
        e.register(SMART_POINTER_RELEASE, |_, _| Ret::default());
        e.register(VECTOR_DESTRUCT_ITERATOR, |_, _| Ret::default());
        let data: Ptr<LoadedLandData> = e.new_object();
        start_log(&mut e);
        e.call(0x0053_7130, &args![data]);
        assert_eq!(
            end_log(&mut e),
            vec![
                (SMART_POINTER_RELEASE, vec![data.addr() + 0x94]),
                (
                    VECTOR_DESTRUCT_ITERATOR,
                    vec![data.addr() + 0x54, 0x10, 4, GRASS_MAP_DESTRUCT]
                ),
                (SMART_POINTER_RELEASE, vec![data.addr() + 0x14])
            ]
        );
        // The deleting form frees the block only with bit 0.
        start_log(&mut e);
        assert_eq!(e.call(0x0053_7100, &args![data, 0u32]).u32(), data.addr());
        assert!(!called(&mut e).contains(&DEALLOCATE));
        e.register(DEALLOCATE, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0053_7100, &args![data, 1u32]);
        let log = end_log(&mut e);
        assert_eq!(log.last().unwrap(), &(DEALLOCATE, vec![data.addr()]));
    }

    // ---- Meshes -------------------------------------------------------------------

    /// What the strips builder calls: allocators, the name formatter, the
    /// fixed string (the handle holds the address of the text), and the
    /// strips constructor, which hands out a fresh object each time.
    fn strips_engine() -> Engine {
        let mut e = loader_engine();
        e.register(NI_ALLOCATE, |e, a| ret(e.mem.alloc(a[0])));
        e.register(NI_ALLOCATE_OBJECT, |e, a| ret(e.mem.alloc(a[0].max(0x100))));
        e.register(FORMAT_STRING, |e, a| {
            let text = format!("Block ({}, {})", a[3], a[4]);
            e.mem.set_cstr(a[0], text.as_bytes());
            ret(text.len() as u32)
        });
        e.register(FIXED_STRING_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(FIXED_STRING_DESTRUCT, |_, _| Ret::default());
        e.register(OBJECT_SET_NAME, |_, _| Ret::default());
        e.register(NODE_SET_TRANSLATE, |_, _| Ret::default());
        e.register(LAND_BASE_HEIGHT, |_, _| float_ret(3.0));
        e.register(TRI_STRIPS_CONSTRUCT, |e, _| ret(e.mem.alloc(0x40)));
        e
    }

    #[test]
    fn tri_strips_are_built_from_the_blocks_of_a_quadrant() {
        let mut e = strips_engine();
        let constructed = recorder::<(Vec<u32>, Vec<u8>, Vec<u8>, Vec<u8>, u16)>();
        let sink = constructed.clone();
        e.register_double(TRI_STRIPS_CONSTRUCT, move |e, a| {
            // Snapshot what the constructor is given: the normals and
            // texture coordinate copies, the strip index list and the
            // length.
            sink.borrow_mut().push((
                a.to_vec(),
                e.mem.bytes(a[3], 0xd8c),
                e.mem.bytes(a[5], 0x908),
                e.mem.bytes(a[11], 0x7fa),
                e.mem.u16(a[10]),
            ));
            ret(e.mem.alloc(0x40))
        });
        let translated = recorder::<(u32, Vec<f32>)>();
        let sink = translated.clone();
        e.register_double(NODE_SET_TRANSLATE, move |e, a| {
            sink.borrow_mut().push((a[0], floats(e, a[1], 3)));
            Ret::default()
        });
        let names = recorder::<(u32, String)>();
        let sink = names.clone();
        e.register_double(OBJECT_SET_NAME, move |e, a| {
            let text = e.mem.u32(a[1]);
            sink.borrow_mut()
                .push((a[0], String::from_utf8(e.mem.cstr(text)).unwrap()));
            Ret::default()
        });
        let defaults = e.mem.alloc(0x908);
        e.mem.write(defaults, &vec![0x5a; 0x908]);
        e.set_global(DEFAULT_TEXTURE_COORDINATES, defaults);
        let strip_list: Vec<u8> = (0..0x7fa).map(|i| (i % 251) as u8).collect();
        e.mem.write(TRIANGLE_STRIP_LIST, &strip_list);
        let (this, data) = land_with_data(&mut e);
        fill_blocks(&mut e, data);
        e.set(data, LoadedLandData::iCellX, 1);
        e.set(data, LoadedLandData::iCellY, 2);
        let vertices = e.get(data, LoadedLandData::ppVertices);
        let colors = e.get(data, LoadedLandData::ppColorsA);
        let result = e.call(0x0053_71b0, &args![this, 2u32]);
        let strips = result.u32();
        assert_ne!(strips, 0);
        let constructed = constructed.borrow();
        assert_eq!(constructed.len(), 1);
        let (arguments, normals, coordinates, indices, length) = &constructed[0];
        assert_eq!(
            arguments[1..9],
            [
                0x121,
                element(&e, vertices, 2),
                arguments[3],
                element(&e, colors, 2),
                arguments[5],
                1,
                0,
                0x3fb
            ]
        );
        assert_eq!(arguments[9], 1);
        // The normals are a copy of the quadrant's, the coordinates a copy
        // of the defaults.
        assert!(normals.iter().all(|b| *b == 32 + 2));
        assert_ne!(
            arguments[3],
            element(&e, e.get(data, LoadedLandData::ppNormals), 2)
        );
        assert!(coordinates.iter().all(|b| *b == 0x5a));
        assert_eq!(*length, 0x3fd);
        assert_eq!(*indices, strip_list);
        // Moved to the land's centre and named after the block.
        assert_eq!(
            *translated.borrow(),
            vec![(strips, vec![6144.0, 10240.0, 3.0])]
        );
        assert_eq!(*names.borrow(), vec![(strips, "Block (2, 0)".to_string())]);
        // Blocks from 4 on, and lands without data, give nothing.
        assert_eq!(e.call(0x0053_71b0, &args![this, 4u32]).u32(), 0);
        let bare = land(&mut e);
        assert_eq!(e.call(0x0053_71b0, &args![bare, 0u32]).u32(), 0);
        assert_eq!(constructed.len(), 1);
    }

    // ---- The quadrant meshes ----------------------------------------------------------

    type EventLog = Rc<RefCell<Vec<(&'static str, Vec<u32>)>>>;

    /// Everything the mesh builder touches, recorded as `(name, words)`.
    struct MeshWorld {
        events: EventLog,
        nodes: Vec<u32>,
        roots: Vec<u32>,
        strips: Rc<RefCell<Vec<u32>>>,
        renderer: u32,
    }

    fn event(world: &EventLog, name: &'static str, words: &[u32]) {
        world.borrow_mut().push((name, words.to_vec()));
    }

    /// An engine with a cell of four nodes and doubles for every scene graph
    /// and Havok call of the mesh builder.
    fn mesh_world() -> (Engine, Ptr<TESObjectLAND>, Ptr<LoadedLandData>, MeshWorld) {
        let mut e = strips_engine();
        let events = recorder::<(&'static str, Vec<u32>)>();
        let strips = recorder::<u32>();
        // Nodes with `AttachChild` in slot 0xf8, roots with the world
        // translate setter in slot 0xb8.
        let mut nodes = vec![];
        let mut roots = vec![];
        let sink = events.clone();
        e.register_double(0x00aa_0020, move |_, a| {
            event(&sink, "attach", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(0x00aa_0021, move |e, a| {
            let mut words = vec![a[0]];
            words.extend(floats(e, a[1], 3).into_iter().map(f32::to_bits));
            event(&sink, "world translate", &words);
            Ret::default()
        });
        for _ in 0..4 {
            nodes.push(object_with_vtable(&mut e, 0x20, &[(0xf8, 0x00aa_0020)]));
            roots.push(object_with_vtable(&mut e, 0x20, &[(0xb8, 0x00aa_0021)]));
        }
        let renderer = object_with_vtable(&mut e, 0x20, &[(0xec, 0x00aa_0022)]);
        let sink = events.clone();
        e.register_double(0x00aa_0022, move |_, a| {
            event(&sink, "queue", a);
            Ret::default()
        });
        e.register(CELL_GET_3D, |_, _| ret(1));
        let list = nodes.clone();
        e.register_double(CELL_GET_NODE, move |_, a| ret(list[a[1] as usize]));
        let list = nodes.clone();
        e.register_double(NODE_GET_CULLED, move |_, a| {
            ret(list.iter().position(|n| *n == a[0]).unwrap() as u32 % 2)
        });
        let sink = events.clone();
        e.register_double(NODE_SET_CULLED, move |_, a| {
            event(&sink, "culled", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(NODE_ATTACH_PROPERTY, move |_, a| {
            event(&sink, "property", a);
            Ret::default()
        });
        e.set_global(DEFAULT_TEXTURING_PROPERTY, 0x9999u32);
        let list = strips.clone();
        e.register_double(TRI_STRIPS_CONSTRUCT, move |e, _| {
            let object = e.mem.alloc(0x40);
            list.borrow_mut().push(object);
            ret(object)
        });
        e.register(TRI_STRIPS_VERTEX_COUNT, |_, _| ret(0x121));
        e.register(GEOMETRY_GET_POSITIONS, |_, a| ret(a[0] + 1));
        e.register(LAND_BOUND_SOURCE, |_, a| ret(a[0] + 2));
        e.register(GEOMETRY_GET_DATA, |_, a| ret(a[0] + 3));
        let sink = events.clone();
        e.register_double(BOUND_COMPUTE_FROM_DATA, move |_, a| {
            event(&sink, "bound", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(GEOMETRY_DATA_MARK_CHANGED, move |_, a| {
            event(&sink, "changed", a);
            Ret::default()
        });
        e.register(FACE_GEN_NODE_GET_ANIMATION_DATA, |_, a| ret(a[0] + 8));
        let list: Vec<(u32, u32)> = nodes.iter().map(|n| n + 8).zip(roots.clone()).collect();
        e.register_double(ANIMATION_DATA_GET_ROOT, move |_, a| {
            ret(list.iter().find(|(anim, _)| *anim == a[0]).unwrap().1)
        });
        e.register(LAND_QUADRANT_EXTENTS, |e, a| {
            e.mem.set_f32(a[1], -4.0);
            e.mem.set_f32(a[1] + 4, 12.0 + a[2] as f32);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(ROOT_SET_LOCAL_TRANSLATE, move |e, a| {
            let mut words = vec![a[0]];
            words.extend(floats(e, a[1], 3).into_iter().map(f32::to_bits));
            event(&sink, "local translate", &words);
            Ret::default()
        });
        e.register(LAND_CELL_MOPP, |_, _| ret(0x6600));
        let sink = events.clone();
        e.register_double(CELL_MOPP_PREPARE, move |_, a| {
            event(&sink, "mopp prepare", a);
            Ret::default()
        });
        e.register(LAND_MOPP_LOCAL_CONSTRUCT, |_, _| Ret::default());
        let sink = events.clone();
        e.register_double(CELL_MOPP_CREATE, move |e, a| {
            let mut words = a.to_vec();
            words.extend(read_words(e, a[1], 4));
            event(&sink, "mopp create", &words);
            Ret::default()
        });
        for address in LAND_BUILD_FOLLOW_UPS {
            let sink = events.clone();
            e.register_double(address, move |_, a| {
                event(&sink, "follow-up", a);
                Ret::default()
            });
        }
        let sink = events.clone();
        e.register_double(GEOMETRY_DATA_SET_CONSISTENCY, move |_, a| {
            event(&sink, "consistency", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(GEOMETRY_DATA_SET_KEEP_FLAGS, move |_, a| {
            event(&sink, "keep", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(GEOMETRY_DATA_SET_COMPRESS_FLAGS, move |_, a| {
            event(&sink, "compress", a);
            Ret::default()
        });
        // The scheduler of strips `n` (in construction order): none, state
        // 3 (valid), state 0 and state 6 (both invalid).
        let list = strips.clone();
        e.register_double(GEOMETRY_GET_SCHEDULER, move |_, a| {
            let index = list.borrow().iter().position(|s| *s == a[0]).unwrap();
            ret(if index == 0 { 0 } else { 0x7000 + index as u32 })
        });
        e.register(SCHEDULER_GET_SAVED_ACQUIRE, |_, a| {
            ret([0, 3, 0, 6][(a[0] - 0x7000) as usize])
        });
        e.register(LAND_RENDER_TASK, |_, a| ret(a[0] + 0x100));
        e.register_double(RENDERER_GET, move |_, _| ret(renderer));
        let sink = events.clone();
        e.register_double(RENDERER_AFTER_QUEUE, move |_, a| {
            event(&sink, "after queue", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(UPDATE_DATA_CONSTRUCT, move |_, a| {
            event(&sink, "update data", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(NODE_UPDATE, move |_, a| {
            event(&sink, "update", a);
            Ret::default()
        });
        let sink = events.clone();
        e.register_double(NODE_UPDATE_PROPERTIES, move |_, a| {
            event(&sink, "update properties", a);
            Ret::default()
        });
        e.register(CELL_QUADRANT_TEST, |_, a| ret((a[1] == 2) as u32));
        e.register(CELL_REFERENCE_LIST_CHECK, |_, _| ret(0));
        let defaults = e.mem.alloc(0x908);
        e.set_global(DEFAULT_TEXTURE_COORDINATES, defaults);
        let (this, data) = land_with_data(&mut e);
        fill_blocks(&mut e, data);
        e.set(data, LoadedLandData::iCellX, 2);
        e.set(data, LoadedLandData::iCellY, -1);
        e.set(data, LoadedLandData::fBaseHeight, 3.0);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        (
            e,
            this,
            data,
            MeshWorld {
                events,
                nodes,
                roots,
                strips,
                renderer,
            },
        )
    }

    fn events_named(world: &MeshWorld, name: &str) -> Vec<Vec<u32>> {
        world
            .events
            .borrow()
            .iter()
            .filter(|(n, _)| *n == name)
            .map(|(_, words)| words.clone())
            .collect()
    }

    #[test]
    fn meshes_are_built_for_the_four_quadrants() {
        let (mut e, this, data, world) = mesh_world();
        e.call(0x0053_74f0, &args![this]);
        let strips = world.strips.borrow().clone();
        assert_eq!(strips.len(), 4);
        // The mesh array holds the cell's four nodes.
        let meshes = e.get(data, LoadedLandData::ppMesh);
        assert_eq!(read_words(&e, meshes.addr(), 4), world.nodes);
        // Hidden while built (node 1 and 3 were hidden before), the default
        // property attached, the strips attached with arguments (0, strips).
        let culled = events_named(&world, "culled");
        for (i, node) in world.nodes.iter().enumerate() {
            assert_eq!(culled[i], vec![*node, 1]);
            assert_eq!(events_named(&world, "property")[i], vec![*node, 0x9999]);
            assert_eq!(events_named(&world, "attach")[i], vec![*node, 0, strips[i]]);
            // The bound is computed from the strips' vertices.
            assert_eq!(
                events_named(&world, "bound")[i],
                vec![strips[i] + 2, 0x121, strips[i] + 1]
            );
            assert_eq!(events_named(&world, "changed")[i], vec![strips[i] + 3, 0xf]);
            // The root node: local translation from the height extents
            // (-4 to 12 + quadrant), world translation at the quadrant's
            // centre.
            let root = world.roots[i];
            let half = (16.0 + i as f32) / 2.0;
            assert_eq!(
                events_named(&world, "local translate")[i],
                vec![
                    root,
                    1024.0f32.to_bits(),
                    1024.0f32.to_bits(),
                    half.to_bits()
                ]
            );
            let x = 2.0 * 4096.0 + 1024.0 + (i % 2) as f32 * 2048.0;
            let y = -4096.0 + 1024.0 + (i / 2) as f32 * 2048.0;
            let z = (8.0 + i as f32) / 2.0;
            assert_eq!(
                events_named(&world, "world translate")[i],
                vec![root, x.to_bits(), y.to_bits(), z.to_bits()]
            );
        }
        // The Havok MOPP of the cell from the four strips and the base
        // height, followed by the three follow-up builds.
        assert_eq!(events_named(&world, "mopp prepare"), vec![vec![0x6600]]);
        let create = &events_named(&world, "mopp create")[0];
        assert_eq!(create[0], 0x6600);
        assert_eq!(create[2], 4);
        assert_eq!(create[3], 3.0f32.to_bits());
        assert_eq!(create[4..], strips[..]);
        assert_eq!(
            events_named(&world, "follow-up"),
            vec![vec![this.addr()]; 3]
        );
        // Geometry data flags, for all four.
        assert_eq!(events_named(&world, "consistency").len(), 4);
        assert_eq!(
            events_named(&world, "consistency")[0],
            vec![strips[0] + 3, 0x4000]
        );
        assert_eq!(events_named(&world, "keep")[0], vec![strips[0] + 3, 1]);
        assert_eq!(
            events_named(&world, "compress")[0],
            vec![strips[0] + 3, 0x17]
        );
        // Only the strips whose scheduler is in state 1 to 5 are queued with
        // the renderer.
        let queued = events_named(&world, "queue");
        assert_eq!(
            queued,
            vec![vec![world.renderer, strips[1], 0, 0, 0x7001 + 0x100]]
        );
        assert_eq!(
            events_named(&world, "after queue"),
            vec![vec![world.renderer]]
        );
        // The nodes get their hidden state back, are updated with a fresh
        // update data (time 0), and their properties updated.
        let updates = events_named(&world, "update");
        let data_objects = events_named(&world, "update data");
        assert_eq!(updates.len(), 4);
        for i in 0..4 {
            assert_eq!(updates[i][0], world.nodes[i]);
            assert_eq!(updates[i][1], data_objects[i][0]);
            assert_eq!(data_objects[i][1..], [0, 0, 0]);
            assert_eq!(
                events_named(&world, "update properties")[i],
                vec![world.nodes[i]]
            );
            // After the first four (hiding) and before the last four, the
            // restoring of the culled flag: node 1 and 3 were hidden before.
            assert_eq!(culled[4 + i], vec![world.nodes[i], (i % 2) as u32]);
        }
        // Finally the nodes are hidden as the cell says: quadrant 2 only.
        for i in 0..4 {
            assert_eq!(culled[8 + i], vec![world.nodes[i], (i == 2) as u32]);
        }
    }

    #[test]
    fn existing_meshes_only_get_their_hidden_flags() {
        let (mut e, this, _data, world) = mesh_world();
        e.call(0x0053_74f0, &args![this]);
        let before = world.events.borrow().len();
        // A land of the world space's land file whose cell fails the check:
        // everything hidden.
        e.set(this, TESObjectLAND::Data, 0x400);
        e.call(0x0053_74f0, &args![this]);
        let culled: Vec<Vec<u32>> = world.events.borrow()[before..]
            .iter()
            .filter(|(n, _)| *n == "culled")
            .map(|(_, w)| w.clone())
            .collect();
        assert_eq!(
            culled,
            world.nodes.iter().map(|n| vec![*n, 1]).collect::<Vec<_>>()
        );
        // When the cell passes the check, the quadrants are asked.
        e.register(CELL_REFERENCE_LIST_CHECK, |_, _| ret(1));
        let before = world.events.borrow().len();
        e.call(0x0053_74f0, &args![this]);
        let culled: Vec<Vec<u32>> = world.events.borrow()[before..]
            .iter()
            .filter(|(n, _)| *n == "culled")
            .map(|(_, w)| w.clone())
            .collect();
        assert_eq!(
            culled,
            world
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| vec![*n, (i == 2) as u32])
                .collect::<Vec<_>>()
        );
        // No strips were built again.
        assert_eq!(world.strips.borrow().len(), 4);
        // Without a parent cell, or when it has no 3D, nothing happens.
        let before = world.events.borrow().len();
        e.set(this, TESObjectLAND::pParentCell, Ptr::NULL);
        e.call(0x0053_74f0, &args![this]);
        e.set(this, TESObjectLAND::pParentCell, Ptr::new(0x6000));
        e.register(CELL_GET_3D, |_, _| ret(0));
        e.call(0x0053_74f0, &args![this]);
        assert_eq!(world.events.borrow().len(), before);
    }
}
