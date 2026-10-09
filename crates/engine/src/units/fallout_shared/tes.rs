//! `fallout shared/tes.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `TES` is the game's world manager: the loaded exterior/interior cells, the
//! scene graph roots, the sky and the singletons the world needs (the
//! `TESDataHandler`, the `ModelLoader`, the save/load object...). Session 1
//! of this unit (the first 40 functions in address order, `0044fb20` to
//! `00450bf0`) holds `TES::TES`, `TES::~TES` and the small static helpers
//! next to them: they copy INI settings into the globals of other
//! subsystems (Havok, LOD fade) and read or delete the singletons.
//! Session 2 (the next 40 functions, `00450c20` to `00452420`) holds the
//! water system's creation and deletion, the culling and lighting of the
//! grid of loaded exterior cells, the grid-position tests,
//! `TES::LoadGridCell`, and the function that loads one reference into the
//! scene (`00451ef0`) with its small helpers. The next session continues at
//! `00452440`.
//!
//! The PC build lays `TES` out like the Xbox build up to `0xC4` bytes; the
//! offsets used so far are declared in [`TES`] below with the Xbox PDB names.
//! The INI setting objects (`SettingT<INISettingCollection>`: vtable at +0,
//! value at +4, which `0043d4d0`/`00403e20`/`00408d60` return a pointer to)
//! are named after the string their initializer in the exe's tail passes to
//! the setting constructor.
//!
//! Not translated: the compiler's exception-unwinding frames of `TES::TES`
//! and `TES::~TES` (the `__CxxFrameHandler` states stored in the frame).

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleList;

layout! {
    /// `TES` (Xbox PDB), 0xC4 bytes on both builds.
    pub struct TES: 0xC4 {
        /// `pGridCellA` (Xbox PDB): `GridCellArray*`, 0x28 bytes.
        0x08 pGridCellA: Ptr,
        /// `pObjRoot` (Xbox PDB): `NiNode*`.
        0x0C pObjRoot: Ptr,
        /// `pObjLandRoot` (Xbox PDB): `NiNode*`.
        0x10 pObjLandRoot: Ptr,
        /// `pObjLODWaterRoot` (Xbox PDB): `NiNode*`.
        0x14 pObjLODWaterRoot: Ptr,
        /// `pTempNodeManager` (Xbox PDB): `BSTempNodeManager*` (0xB0 bytes on PC).
        0x18 pTempNodeManager: Ptr,
        /// `pObjLight` (Xbox PDB): taken from the sky's `pSun` by `004505a0`.
        0x1C pObjLight: Ptr,
        /// `pObjFog` (Xbox PDB): taken from the sky's `pAtmosphere` by `00450570`.
        0x20 pObjFog: Ptr,
        /// `iCurrentGridX` (Xbox PDB): x of the centre grid cell, in cell units.
        0x24 iCurrentGridX: i32,
        /// `iCurrentGridY` (Xbox PDB).
        0x28 iCurrentGridY: i32,
        /// `iCurrentQueuedX` (Xbox PDB).
        0x2C iCurrentQueuedX: i32,
        /// `iCurrentQueuedY` (Xbox PDB).
        0x30 iCurrentQueuedY: i32,
        /// `pInteriorCell` (Xbox PDB): `TESObjectCELL*`. `005f36f0` (named
        /// `ActorMover::GetPreferredMoveMode` by the map; identical code)
        /// returns it.
        0x34 pInteriorCell: Ptr,
        /// `pInteriorBuffer` (Xbox PDB): `TESObjectCELL**`, one slot per
        /// `uInterior Cell Buffer`.
        0x38 pInteriorBuffer: Ptr,
        /// `pExteriorBuffer` (Xbox PDB): `TESObjectCELL**`, one slot per
        /// `uExterior Cell Buffer`.
        0x3C pExteriorBuffer: Ptr,
        /// `iTempInteriorBufferSize` (Xbox PDB).
        0x40 iTempInteriorBufferSize: u32,
        /// `iTempExteriorBufferSize` (Xbox PDB).
        0x44 iTempExteriorBufferSize: u32,
        /// `iSaveGridX` (Xbox PDB).
        0x48 iSaveGridX: i32,
        /// `iSaveGridY` (Xbox PDB).
        0x4C iSaveGridY: i32,
        /// `bRunningCellTests` (Xbox PDB).
        0x51 bRunningCellTests: bool,
        /// `bRunningCellTests2` (Xbox PDB).
        0x52 bRunningCellTests2: bool,
        /// `pTACRegionFilter` (Xbox PDB).
        0x5C pTACRegionFilter: Ptr,
        /// `bShowLANDborders` (Xbox PDB).
        0x60 bShowLANDborders: bool,
        /// `pWaterSystem` (Xbox PDB): `TESWaterSystem*` (0xA0 bytes), built by
        /// `00450c50` and deleted by `00450d00`.
        0x64 pWaterSystem: Ptr,
        /// `pSky` (Xbox PDB): `Sky*`.
        0x68 pSky: Ptr,
        /// `listActiveImageSpaceModifiers` (Xbox PDB).
        0x6C listActiveImageSpaceModifiers: Inline<BSSimpleList>,
        /// `bUpdateGridString` (Xbox PDB).
        0x7D bUpdateGridString: bool,
        /// `pWorldSpace` (Xbox PDB).
        0x88 pWorldSpace: Ptr,
        /// `listLastLoadedExteriors` (Xbox PDB).
        0x8C listLastLoadedExteriors: Inline<BSSimpleList>,
        /// `ListofBedsAndChairs` (Xbox PDB).
        0x94 ListofBedsAndChairs: Inline<BSSimpleList>,
        /// `DeadCount` (Xbox PDB).
        0x9C DeadCount: Inline<BSSimpleList>,
        /// `spPreloadedAddonNodes` (Xbox PDB): `NiPointer<QueuedFile>`.
        0xA4 spPreloadedAddonNodes: Ptr,
        /// `BloodDecalPreload1` (Xbox PDB): `NiPointer<NiSourceTexture>`.
        0xA8 BloodDecalPreload1: Ptr,
        /// `spPreloadedForms` (Xbox PDB): `NiPointer<QueuedFile>`.
        0xAC spPreloadedForms: Ptr,
        /// `pParticleCacheHead` (Xbox PDB).
        0xB0 pParticleCacheHead: Ptr,
        /// `bFadeWhenLoading` (Xbox PDB).
        0xB4 bFadeWhenLoading: bool,
        /// `bAllowUnusedPurge` (Xbox PDB).
        0xB5 bAllowUnusedPurge: bool,
        /// `pNavMeshInfoMap` (Xbox PDB).
        0xBC pNavMeshInfoMap: Ptr,
        /// `spLoadedAreaBound` (Xbox PDB): `NiPointer<LoadedAreaBound>`.
        0xC0 spLoadedAreaBound: Ptr,
    }
}

/// `TES`'s vtable (`GetMapNameForLocation` first).
const TES_VTABLE: u32 = 0x0101_78b4;
/// `BSTempNodeManager`'s vtable.
const BS_TEMP_NODE_MANAGER_VTABLE: u32 = 0x0101_78bc;
/// `BSTempNodeManager`'s `NiRTTI` (what `GetRTTI` returns).
const BS_TEMP_NODE_MANAGER_RTTI: u32 = 0x0120_2df8;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\TES.cpp"`.
const TES_CPP_PATH: u32 = 0x0101_7874;

// The singletons the world manager creates and destroys.

/// The `TESDataHandler` singleton pointer (the object is 0x63C bytes).
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// The `ModelLoader` singleton pointer (0x30 bytes).
const MODEL_LOADER: u32 = 0x011c_3b3c;
/// The `IOManager` singleton pointer (0xA0 bytes on PC; inferred from its
/// readers in `main` and the queued-file code).
const IO_MANAGER: u32 = 0x0120_2d98;
/// The `BSParallelTaskManager` singleton pointer (0x10 bytes), created only
/// when `iNumHWThreads:General` is above 1.
const PARALLEL_TASK_MANAGER: u32 = 0x0120_2df4;
/// The `TESSaveLoadGame` singleton pointer (0x1C8 bytes).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
/// An object whose vtable slot `0x1CC` `TES::~TES` calls (with two zeros).
const OBJECT_WITH_SLOT_1CC: u32 = 0x011d_ea3c;
/// An object `TES::~TES` passes to `00867840` as `this` (inline, not a pointer).
const OBJECT_011DE7B8: u32 = 0x011d_e7b8;
/// An object `TES::~TES` passes to `00977540` as `this` (inline).
const OBJECT_011E0E80: u32 = 0x011e_0e80;
/// The `VATS` object `TES::~TES` passes to `VATS::ClearData` (`009c6ba0`).
const VATS_OBJECT: u32 = 0x011f_2250;
/// Table of four `BSShadowSceneNode` pointers (`00450b80` reads entry `n`).
const SHADOW_SCENE_NODE_TABLE: u32 = 0x011f_91c8;
/// Number of entries in [`SHADOW_SCENE_NODE_TABLE`].
const SHADOW_SCENE_NODE_COUNT: u32 = 4;
/// High-water mark of `uGridsToLoad` squared, raised by `TES::TES`.
const GRID_CELL_COUNT_HIGH_WATER_MARK: u32 = 0x011c_3c00;

// INI settings (`SettingT<INISettingCollection>` objects).

/// `iUpdateType:HAVOK`.
const SETTING_HAVOK_UPDATE_TYPE: u32 = 0x011c_3c30;
/// `bAddBipedWhenKeyframed:HAVOK`.
const SETTING_ADD_BIPED_WHEN_KEYFRAMED: u32 = 0x011c_3da0;
/// `fDebrisMaxVelocity`.
const SETTING_DEBRIS_MAX_VELOCITY: u32 = 0x011c_3c74;
/// `fDebrisMinExtent`.
const SETTING_DEBRIS_MIN_EXTENT: u32 = 0x011c_3db0;
/// `fChaseDeltaMult:HAVOK`.
const SETTING_CHASE_DELTA_MULT: u32 = 0x011c_3d28;
/// `fMaxPickTime:HAVOK`.
const SETTING_MAX_PICK_TIME: u32 = 0x011c_3e54;
/// `fMaxPickTimeVATS:HAVOK`.
const SETTING_MAX_PICK_TIME_VATS: u32 = 0x011c_3e94;
/// `iEntityBatchRemoveRate:HAVOK`.
const SETTING_ENTITY_BATCH_REMOVE_RATE: u32 = 0x011c_3c3c;
/// `fMoveLimitMass:HAVOK`.
const SETTING_MOVE_LIMIT_MASS: u32 = 0x011c_3d4c;
/// `fCharacterControllerMultipleStepSpeed`.
const SETTING_CHARACTER_CONTROLLER_MULTIPLE_STEP_SPEED: u32 = 0x011c_3ce8;
/// `fPhysicsDamage1Mass`.
const SETTING_PHYSICS_DAMAGE_1_MASS: u32 = 0x011c_fb38;
/// `fFadeOutThreshold:LOD`.
const SETTING_FADE_OUT_THRESHOLD: u32 = 0x011c_3e18;
/// `fFadeInThreshold:LOD`.
const SETTING_FADE_IN_THRESHOLD: u32 = 0x011c_3d00;
/// `fFadeInTime:LOD`.
const SETTING_FADE_IN_TIME: u32 = 0x011c_3df4;
/// `fFadeOutTime:LOD`.
const SETTING_FADE_OUT_TIME: u32 = 0x011c_3c5c;
/// `fDistanceMultiplier:LOD`.
const SETTING_DISTANCE_MULTIPLIER: u32 = 0x011c_3d60;
/// `bQueueWarnings:General`.
const SETTING_QUEUE_WARNINGS: u32 = 0x011c_3e64;
/// `bCheckPurgedTextureList:General`.
const SETTING_CHECK_PURGED_TEXTURE_LIST: u32 = 0x011c_3ccc;
/// `iNumHWThreads:General`.
const SETTING_NUM_HW_THREADS: u32 = 0x011c_3ea4;
/// `uExterior Cell Buffer:General`.
const SETTING_EXTERIOR_CELL_BUFFER: u32 = 0x011c_3c90;
/// `uInterior Cell Buffer:General`.
const SETTING_INTERIOR_CELL_BUFFER: u32 = 0x011c_3e38;
/// `uGridsToLoad:General`.
const SETTING_GRIDS_TO_LOAD: u32 = 0x011c_63cc;
/// `fLODFadeOutMultItems:LOD`.
const SETTING_LOD_FADE_OUT_MULT_ITEMS: u32 = 0x011c_3d0c;
/// `fLODFadeOutMultActors:LOD`.
const SETTING_LOD_FADE_OUT_MULT_ACTORS: u32 = 0x011c_3ec0;
/// `fLODFadeOutMultObjects:LOD`.
const SETTING_LOD_FADE_OUT_MULT_OBJECTS: u32 = 0x011c_3d78;

// Callees outside this unit.

/// Setting value pointer, `int` flavour (`this + 4`).
const SETTING_VALUE_ADDRESS_INT: u32 = 0x0043_d4d0;
/// Setting value pointer, `float` flavour (identical code).
const SETTING_VALUE_ADDRESS_FLOAT: u32 = 0x0040_3e20;
/// Setting value pointer, `bool`/byte flavour (identical code).
const SETTING_VALUE_ADDRESS_BYTE: u32 = 0x0040_8d60;
/// Sets a `uint` setting's value (`this` = the setting).
const SETTING_SET_UINT: u32 = 0x0045_ce80;
/// `_memset` through the `std::_Fiopen` name the linker folded onto it.
const MEMSET: u32 = 0x0040_3d30;
/// `operator new` (`MemoryManager::Allocate`).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete`.
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `NiMemObject::operator new` (the block comes back in EAX).
const NI_OPERATOR_NEW: u32 = 0x00aa_13e0;
/// Sized `NiMemObject::operator delete(block, size)`.
const NI_OPERATOR_DELETE: u32 = 0x00aa_1460;
/// The 4-byte scope guard the constructor keeps on its stack (sets the
/// memory-allocation tag to its first argument for the guard's lifetime).
const SCOPE_GUARD_CTOR: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DTOR: u32 = 0x0040_4ee0;
/// Size of that guard.
const SCOPE_GUARD_SIZE: u32 = 4;

// Globals and settings used by the grid code (session 2).

/// `bUseWater:Water`.
const SETTING_USE_WATER: u32 = 0x011c_7adc;
/// `fFadeToBlackFadeSeconds`.
const SETTING_FADE_TO_BLACK_FADE_SECONDS: u32 = 0x011c_3e7c;
/// The `TES` singleton pointer.
const TES_SINGLETON: u32 = 0x011d_ea10;
/// The `PlayerCharacter` singleton pointer.
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
/// The `FaderManager` singleton pointer.
const FADER_MANAGER: u32 = 0x011d_8804;
/// The object `00451ef0` passes as `this` to `0042ce10` (a flag getter).
const SCRIPT_CONTEXT: u32 = 0x011d_df38;
/// `TESObjectCELL::spExteriorWorld` (Xbox PDB): a `NiPointer<bhkWorldM>`;
/// `00559450` (the `NiPointer` getter, returns the first word) reads it.
const EXTERIOR_WORLD: u32 = 0x011c_a0d8;
/// Byte `004512c0` sets while it runs and clears at its end; it returns at
/// once when the byte is already set (a re-entrancy guard).
const GRID_LOAD_IN_PROGRESS: u32 = 0x011c_3ed0;
/// Byte written by `00451520` (also written by `ClearCanopyShadowMaskTexture`
/// and read by `008705c0`).
const GRID_FLAG_01189184: u32 = 0x0118_9184;
/// Byte written by `00451590` (read by `00b61980`).
const GRID_FLAG_011AD86C: u32 = 0x011a_d86c;
/// Three words (a `NiPoint3`) `004512c0` passes to `0057d0a0` after the
/// camera position.
const POINT_011A9478: u32 = 0x011a_9478;
/// A `double` (200.0 in the exe) that `00451ef0` compares two floats with
/// (address of the constant; read at run time).
const LARGE_CELL_LIMIT: u32 = 0x0101_79e0;
/// `"Loading cell...%s (%i, %i) (%08X)"`.
const LOADING_CELL_FORMAT: u32 = 0x0101_79bc;
/// `__RTDynamicCast` source and target type descriptors (RTTI data in the
/// exe) that `00451ef0` casts the reference's base form with.
const RTTI_TESFORM: u32 = 0x0118_3108;
const RTTI_CAST_TARGET: u32 = 0x0118_6060;

// Callees outside this unit that several grid functions use.

/// `TESObjectCELL::GetDataX`/`GetDataY` (Xbox PDB).
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
/// `TES::pInteriorCell` getter (the map calls it
/// `ActorMover::GetPreferredMoveMode`; the body returns `this + 0x34`).
const GET_INTERIOR_CELL: u32 = 0x005f_36f0;
/// `TES::GetWorldSpace` (Xbox PDB): `this + 0x88`.
const GET_WORLD_SPACE: u32 = 0x004f_d3e0;
/// `GridCellArray::Get(x, y)` (Xbox PDB): the address of the slot holding
/// the cell pointer.
const GRID_CELL_ARRAY_GET: u32 = 0x004b_a490;
/// `GridCellArray::AttachToWorld(root, x, y)` (Xbox PDB).
const GRID_CELL_ARRAY_ATTACH_TO_WORLD: u32 = 0x004b_a960;
/// `TESWorldSpace::GetTerrainManager` (Xbox PDB).
const GET_TERRAIN_MANAGER: u32 = 0x0058_6170;
/// A reference's base form (`this + 0x20`; the map names it
/// `BGSSaveFormBuffer::GetForm`).
const REFERENCE_GET_BASE_FORM: u32 = 0x007a_f430;
/// The form type byte (`this + 4`).
const FORM_GET_TYPE: u32 = 0x0040_1170;
/// `_ftol2_sse`: truncates the `ST0` float to an integer (here an `f64`
/// argument, see the engine guide).
const FTOL: u32 = 0x00ec_62c0;
/// `fistp` of a `float` pushed on the stack (round to nearest), returns it.
const FLOAT_TO_INT_ROUNDED: u32 = 0x0040_6d90;
/// Constructor of the 12-byte stack object (a `float` at +0, bytes at +4 and
/// +5, zeros at +6..+8) the grid code passes to [`NODE_UPDATE`]; arguments
/// `(float, byte, byte)`.
const UPDATE_OBJECT_CTOR: u32 = 0x0043_d410;
/// Takes a node and that 12-byte object and calls the node's vtable slot
/// `0xA4` with the object and 0 (then slot `0xFC` of the object at +0x18, when
/// set).
const NODE_UPDATE: u32 = 0x00a5_9c60;

/// Reads an INI setting's value as an unsigned integer.
fn setting_uint(e: &mut Engine, setting: u32) -> u32 {
    let value = e.call(SETTING_VALUE_ADDRESS_INT, &args![setting]).u32();
    e.mem.u32(value)
}

/// Reads an INI setting's value as a float.
fn setting_float(e: &mut Engine, setting: u32) -> f32 {
    let value = e.call(SETTING_VALUE_ADDRESS_FLOAT, &args![setting]).u32();
    e.mem.f32(value)
}

/// Reads an INI setting's value as a byte.
fn setting_byte(e: &mut Engine, setting: u32) -> u8 {
    let value = e.call(SETTING_VALUE_ADDRESS_BYTE, &args![setting]).u32();
    e.mem.u8(value)
}

/// `uGridsToLoad`, as the constructor reads it: one call per use.
fn grids_to_load(e: &mut Engine) -> u32 {
    setting_uint(e, SETTING_GRIDS_TO_LOAD)
}

/// `uGridsToLoad * uGridsToLoad + uGridsToLoad + uGridsToLoad + 1`, reading
/// the setting four times in the order the code does.
fn exterior_cell_buffer_needed(e: &mut Engine) -> u32 {
    let first = e
        .call(SETTING_VALUE_ADDRESS_INT, &args![SETTING_GRIDS_TO_LOAD])
        .u32();
    let second = e
        .call(SETTING_VALUE_ADDRESS_INT, &args![SETTING_GRIDS_TO_LOAD])
        .u32();
    let squared = e.mem.u32(first).wrapping_mul(e.mem.u32(second));
    let third = grids_to_load(e);
    let fourth = grids_to_load(e);
    squared
        .wrapping_add(third)
        .wrapping_add(fourth)
        .wrapping_add(1)
}

/// `count * 4` as the compiler computes an array allocation size: all ones
/// when the multiplication overflows 32 bits.
fn array_bytes(count: u32) -> u32 {
    count.saturating_mul(4)
}

// Translated from 0044fb20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::TES` (Xbox PDB): builds the world manager. The roots, the sky and
/// the data-handler argument come from the caller (`main`).
///
/// Order of events: the member lists and `NiPointer`s are constructed, the
/// whole object is zeroed with `memset` (which also clears the vtable pointer
/// stored first, and the lists just built; the game does exactly this), the
/// Havok and LOD settings are copied into their globals, the singletons are
/// created (`GridCellArray`, `IOManager`, `ModelLoader`, the parallel task
/// manager when `iNumHWThreads` is above 1, `TESDataHandler`,
/// `TESSaveLoadGame`), the cell buffers are sized from `uExterior Cell
/// Buffer`, `uInterior Cell Buffer` and `uGridsToLoad` (raising the settings
/// when `uGridsToLoad` needs more), the temp node manager is attached to
/// `pObjRoot`, and the sky's light and fog objects are fetched.
pub fn tes_tes(
    e: &mut Engine,
    this: Ptr<TES>,
    data_handler_arg: u32,
    obj_root: Ptr,
    obj_land_root: Ptr,
    sky: Ptr,
    obj_lod_water_root: Ptr,
) -> Ptr<TES> {
    let base = this.addr();
    e.mem.set_u32(base, TES_VTABLE);
    e.call(0x004e_e810, &args![base + 0x6c]);
    e.call(0x0096_a2d0, &args![base + 0x8c]);
    e.call(0x0096_a2d0, &args![base + 0x94]);
    e.call(0x0096_a2d0, &args![base + 0x9c]);
    e.call(0x0052_8cb0, &args![base + 0xa4, 0u32]);
    e.call(0x0063_3c90, &args![base + 0xa8, 0u32]);
    e.call(0x0052_8cb0, &args![base + 0xac, 0u32]);
    e.call(0x0063_3c90, &args![base + 0xc0, 0u32]);
    let guard = e.mem.alloc(SCOPE_GUARD_SIZE);
    e.call(
        SCOPE_GUARD_CTOR,
        &args![guard, 0xau32, 1u32, TES_CPP_PATH, 0x16au32],
    );
    e.call(MEMSET, &args![base, 0u32, 0xc4u32]);

    fn_00450430(e, 1);
    e.call(0x0062_4bf0, &args![]);
    let update_type = fn_004503f0(e, Ptr::new(SETTING_HAVOK_UPDATE_TYPE));
    fn_00450450(e, update_type);
    fn_00450470(e, 0, 0x0057_64f0);
    let add_biped = setting_byte(e, SETTING_ADD_BIPED_WHEN_KEYFRAMED);
    fn_00450490(e, add_biped);
    let value = setting_float(e, SETTING_DEBRIS_MAX_VELOCITY);
    fn_004504a0(e, value);
    let value = setting_float(e, SETTING_DEBRIS_MIN_EXTENT);
    fn_004504b0(e, value);
    let value = setting_float(e, SETTING_CHASE_DELTA_MULT);
    fn_00450460(e, value);
    let value = setting_float(e, SETTING_MAX_PICK_TIME);
    fn_004504c0(e, value);
    let value = setting_float(e, SETTING_MAX_PICK_TIME_VATS);
    fn_004504d0(e, value);
    let value = setting_uint(e, SETTING_ENTITY_BATCH_REMOVE_RATE);
    fn_004504e0(e, value);
    let value = fn_00450410(e, Ptr::new(SETTING_MOVE_LIMIT_MASS));
    fn_00450670(e, value);
    // Both reads are separate calls; the product is stored as a `float`.
    let first = e
        .call(
            SETTING_VALUE_ADDRESS_FLOAT,
            &args![SETTING_CHARACTER_CONTROLLER_MULTIPLE_STEP_SPEED],
        )
        .u32();
    let second = e
        .call(
            SETTING_VALUE_ADDRESS_FLOAT,
            &args![SETTING_CHARACTER_CONTROLLER_MULTIPLE_STEP_SPEED],
        )
        .u32();
    let product = e.mem.f32(first) * e.mem.f32(second);
    fn_00450680(e, product);
    let value = setting_float(e, SETTING_PHYSICS_DAMAGE_1_MASS);
    fn_00450690(e, value);
    fn_004506b0(e);
    fn_004504f0(e, 6, 0.0);
    let minus_one: f32 = e.global(0x0101_2054);
    fn_004504f0(e, 7, minus_one);
    let value = setting_float(e, SETTING_FADE_OUT_THRESHOLD);
    fn_00450540(e, value);
    let value = setting_float(e, SETTING_FADE_IN_THRESHOLD);
    fn_00450530(e, value);
    let value = setting_float(e, SETTING_FADE_IN_TIME);
    fn_00450510(e, value);
    let value = setting_float(e, SETTING_FADE_OUT_TIME);
    fn_00450520(e, value);
    let value = setting_float(e, SETTING_DISTANCE_MULTIPLIER);
    fn_00450550(e, value);
    let value = setting_byte(e, SETTING_QUEUE_WARNINGS);
    fn_00450440(e, value);
    let value = setting_byte(e, SETTING_CHECK_PURGED_TEXTURE_LIST);
    fn_00450560(e, value);
    fn_004505d0(e, 0x0057_7450);
    fn_004505e0(e, 0x0057_7ca0);

    // `GridCellArray`: constructed, then its second virtual (reference
    // count increment) is called.
    let grid_cells = e.call(OPERATOR_NEW, &args![0x28u32]).u32();
    let grid_cells = if grid_cells == 0 {
        0
    } else {
        e.call(0x004b_a280, &args![grid_cells]).u32()
    };
    e.set(this, TES::pGridCellA, Ptr::new(grid_cells));
    e.vcall(grid_cells, 4, &args![]);

    let guard_for_singletons = e.mem.alloc(SCOPE_GUARD_SIZE);
    e.call(
        SCOPE_GUARD_CTOR,
        &args![guard_for_singletons, 0x12u32, 1u32, TES_CPP_PATH, 0x1aeu32],
    );
    e.call(0x0052_7eb0, &args![]);
    let io_manager = e.call(OPERATOR_NEW, &args![0xa0u32]).u32();
    let io_manager = if io_manager == 0 {
        0
    } else {
        e.call(0x00c3_da50, &args![io_manager]).u32()
    };
    e.set_global(IO_MANAGER, io_manager);
    let model_loader = e.call(OPERATOR_NEW, &args![0x30u32]).u32();
    let model_loader = if model_loader == 0 {
        0
    } else {
        e.call(0x0044_2650, &args![model_loader]).u32()
    };
    e.set_global(MODEL_LOADER, model_loader);
    e.call(SCOPE_GUARD_DTOR, &args![guard_for_singletons]);
    e.mem.free(guard_for_singletons);

    let hardware_threads = setting_uint(e, SETTING_NUM_HW_THREADS) as i32;
    if hardware_threads > 1 {
        let manager = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
        let manager = if manager == 0 {
            0
        } else {
            e.call(0x00c4_4980, &args![manager]).u32()
        };
        e.set_global(PARALLEL_TASK_MANAGER, manager);
    }
    let data_handler = e.call(OPERATOR_NEW, &args![0x63cu32]).u32();
    let data_handler = if data_handler == 0 {
        0
    } else {
        e.call(0x0045_d270, &args![data_handler]).u32()
    };
    e.set_global(DATA_HANDLER, data_handler);
    let data_handler: u32 = e.global(DATA_HANDLER);
    e.call(0x0046_24b0, &args![data_handler, data_handler_arg]);
    let save_load = e.call(OPERATOR_NEW, &args![0x1c8u32]).u32();
    let save_load = if save_load == 0 {
        0
    } else {
        e.call(0x0085_6380, &args![save_load]).u32()
    };
    e.set_global(SAVE_LOAD_GAME, save_load);
    e.call(0x0084_7480, &args![]);
    e.call(0x0084_fc70, &args![]);

    // Make sure the exterior cell buffer can hold the loaded grid.
    let exterior = fn_004503f0(e, Ptr::new(SETTING_EXTERIOR_CELL_BUFFER));
    let needed = exterior_cell_buffer_needed(e);
    if exterior < needed {
        let needed = exterior_cell_buffer_needed(e);
        e.call(
            SETTING_SET_UINT,
            &args![SETTING_EXTERIOR_CELL_BUFFER, needed],
        );
    }
    let interior = fn_004503f0(e, Ptr::new(SETTING_INTERIOR_CELL_BUFFER));
    if interior < 1 {
        e.call(SETTING_SET_UINT, &args![SETTING_INTERIOR_CELL_BUFFER, 1u32]);
    }

    let count = setting_uint(e, SETTING_INTERIOR_CELL_BUFFER);
    let interior_buffer = e.call(OPERATOR_NEW, &args![array_bytes(count)]).u32();
    e.set(this, TES::pInteriorBuffer, Ptr::new(interior_buffer));
    let count = setting_uint(e, SETTING_EXTERIOR_CELL_BUFFER);
    let exterior_buffer = e.call(OPERATOR_NEW, &args![array_bytes(count)]).u32();
    e.set(this, TES::pExteriorBuffer, Ptr::new(exterior_buffer));

    let mut index = 0u32;
    while index < fn_004503f0(e, Ptr::new(SETTING_INTERIOR_CELL_BUFFER)) {
        let buffer = e.get(this, TES::pInteriorBuffer).addr();
        e.mem.set_u32(buffer.wrapping_add(index.wrapping_mul(4)), 0);
        index = index.wrapping_add(1);
    }
    let mut index = 0u32;
    while index < fn_004503f0(e, Ptr::new(SETTING_EXTERIOR_CELL_BUFFER)) {
        let buffer = e.get(this, TES::pExteriorBuffer).addr();
        e.mem.set_u32(buffer.wrapping_add(index.wrapping_mul(4)), 0);
        index = index.wrapping_add(1);
    }
    e.set(this, TES::iTempExteriorBufferSize, 0);
    e.set(this, TES::iTempInteriorBufferSize, 0);
    e.call(0x0045_0d80, &args![this]);

    e.set(this, TES::pObjRoot, obj_root);
    let temp_node_manager = e.call(NI_OPERATOR_NEW, &args![0xb0u32]).u32();
    let temp_node_manager = if temp_node_manager == 0 {
        0
    } else {
        fn_004505f0(e, Ptr::new(temp_node_manager)).addr()
    };
    e.set(this, TES::pTempNodeManager, Ptr::new(temp_node_manager));
    let root = e.get(this, TES::pObjRoot);
    if !root.is_null() {
        // `NiNode::AttachChild(pTempNodeManager, true)`.
        let manager = e.get(this, TES::pTempNodeManager);
        e.vcall(root.addr(), 0xdc, &args![manager, 1u32]);
    }
    e.set(this, TES::pObjLandRoot, obj_land_root);
    e.set(this, TES::pObjLODWaterRoot, obj_lod_water_root);
    e.set(this, TES::pSky, sky);
    let sky = e.get(this, TES::pSky);
    let light = if sky.is_null() {
        Ptr::NULL
    } else {
        fn_004505a0(e, sky)
    };
    e.set(this, TES::pObjLight, light);
    let sky = e.get(this, TES::pSky);
    let fog = if sky.is_null() {
        Ptr::NULL
    } else {
        fn_00450570(e, sky)
    };
    e.set(this, TES::pObjFog, fog);
    e.set(this, TES::bUpdateGridString, false);
    e.set(this, TES::pWorldSpace, Ptr::NULL);
    e.set(this, TES::pParticleCacheHead, Ptr::NULL);
    e.set(this, TES::bFadeWhenLoading, true);

    let first = e
        .call(SETTING_VALUE_ADDRESS_INT, &args![SETTING_GRIDS_TO_LOAD])
        .u32();
    let second = e
        .call(SETTING_VALUE_ADDRESS_INT, &args![SETTING_GRIDS_TO_LOAD])
        .u32();
    let cells = e.mem.u32(first).wrapping_mul(e.mem.u32(second));
    if e.global::<u32>(GRID_CELL_COUNT_HIGH_WATER_MARK) < cells {
        let first = e
            .call(SETTING_VALUE_ADDRESS_INT, &args![SETTING_GRIDS_TO_LOAD])
            .u32();
        let second = e
            .call(SETTING_VALUE_ADDRESS_INT, &args![SETTING_GRIDS_TO_LOAD])
            .u32();
        let cells = e.mem.u32(first).wrapping_mul(e.mem.u32(second));
        e.set_global(GRID_CELL_COUNT_HIGH_WATER_MARK, cells);
    }
    e.set(this, TES::bShowLANDborders, false);
    e.set(this, TES::bRunningCellTests, false);
    e.set(this, TES::bRunningCellTests2, false);
    e.set(this, TES::bAllowUnusedPurge, true);
    e.set(this, TES::pNavMeshInfoMap, Ptr::NULL);
    fn_004506a0(e, 0x0045_c680);

    let loaded_area_bound = e.call(NI_OPERATOR_NEW, &args![0x44u32]).u32();
    let loaded_area_bound = if loaded_area_bound == 0 {
        0
    } else {
        // The four floats are the constants at 01017868, 0101786c, 01013974
        // and 01017870, in that order.
        let a: f32 = e.global(0x0101_7868);
        let b: f32 = e.global(0x0101_786c);
        let c: f32 = e.global(0x0101_3974);
        let d: f32 = e.global(0x0101_7870);
        e.call(0x0062_6fd0, &args![loaded_area_bound, a, b, c, d])
            .u32()
    };
    e.call(
        0x0066_b0d0,
        &args![base + TES::spLoadedAreaBound.off, loaded_area_bound],
    );
    e.set(this, TES::pTACRegionFilter, Ptr::NULL);
    e.call(SCOPE_GUARD_DTOR, &args![guard]);
    e.mem.free(guard);
    this
}

// Translated from 004503f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The integer value of the INI setting `this` (`*(this + 4)`, through the
/// setting's value-pointer function).
pub fn fn_004503f0(e: &mut Engine, this: Ptr) -> u32 {
    let value = e.call(SETTING_VALUE_ADDRESS_INT, &args![this]).u32();
    e.mem.u32(value)
}

// Translated from 00450410 (decompiled, FalloutNV.exe 1.4.0.525)
/// The float value of the INI setting `this` (returned in ST0).
pub fn fn_00450410(e: &mut Engine, this: Ptr) -> f32 {
    let value = e.call(SETTING_VALUE_ADDRESS_FLOAT, &args![this]).u32();
    e.mem.f32(value)
}

// Translated from 00450430 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a flag in `011f4300` (read by `004ad1b0` in `bipedanim.cpp`).
pub fn fn_00450430(e: &mut Engine, value: u32) {
    e.set_global(0x011f_4300, value);
}

// Translated from 00450440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `bQueueWarnings:General` in `01202d62`.
pub fn fn_00450440(e: &mut Engine, value: u8) {
    e.set_global(0x0120_2d62, value);
}

// Translated from 00450450 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the Havok update type (`iUpdateType:HAVOK`) in `012677b8`.
pub fn fn_00450450(e: &mut Engine, value: u32) {
    e.set_global(0x0126_77b8, value);
}

// Translated from 00450460 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fChaseDeltaMult:HAVOK` in `011afe70`.
pub fn fn_00450460(e: &mut Engine, value: f32) {
    e.set_global(0x011a_fe70, value);
}

// Translated from 00450470 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a function pointer in entry `index` of the Havok object callback
/// table at `011afe88`.
pub fn fn_00450470(e: &mut Engine, index: u32, callback: u32) {
    e.set_global(0x011a_fe88u32.wrapping_add(index.wrapping_mul(4)), callback);
}

// Translated from 00450490 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `bAddBipedWhenKeyframed:HAVOK` in `011afe59`.
pub fn fn_00450490(e: &mut Engine, value: u8) {
    e.set_global(0x011a_fe59, value);
}

// Translated from 004504a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fDebrisMaxVelocity` in `012677c0`.
pub fn fn_004504a0(e: &mut Engine, value: f32) {
    e.set_global(0x0126_77c0, value);
}

// Translated from 004504b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fDebrisMinExtent` in `012677bc`.
pub fn fn_004504b0(e: &mut Engine, value: f32) {
    e.set_global(0x0126_77bc, value);
}

// Translated from 004504c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fMaxPickTime:HAVOK` in `011afe74`.
pub fn fn_004504c0(e: &mut Engine, value: f32) {
    e.set_global(0x011a_fe74, value);
}

// Translated from 004504d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fMaxPickTimeVATS:HAVOK` in `011afe78`.
pub fn fn_004504d0(e: &mut Engine, value: f32) {
    e.set_global(0x011a_fe78, value);
}

// Translated from 004504e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `iEntityBatchRemoveRate:HAVOK` in `011afe7c`.
pub fn fn_004504e0(e: &mut Engine, value: u32) {
    e.set_global(0x011a_fe7c, value);
}

// Translated from 004504f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a float in entry `index` of the LOD fade-out multiplier table at
/// `011ad7b8` (read by `BSFadeNode`'s `CheckFadeRadius`).
pub fn fn_004504f0(e: &mut Engine, index: u32, value: f32) {
    e.set_global(0x011a_d7b8u32.wrapping_add(index.wrapping_mul(4)), value);
}

// Translated from 00450510 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fFadeInTime:LOD` in `011ad7e4`.
pub fn fn_00450510(e: &mut Engine, value: f32) {
    e.set_global(0x011a_d7e4, value);
}

// Translated from 00450520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fFadeOutTime:LOD` in `011ad7e8`.
pub fn fn_00450520(e: &mut Engine, value: f32) {
    e.set_global(0x011a_d7e8, value);
}

// Translated from 00450530 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fFadeInThreshold:LOD` in `011ad7ec`.
pub fn fn_00450530(e: &mut Engine, value: f32) {
    e.set_global(0x011a_d7ec, value);
}

// Translated from 00450540 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fFadeOutThreshold:LOD` in `011ad7f0`.
pub fn fn_00450540(e: &mut Engine, value: f32) {
    e.set_global(0x011a_d7f0, value);
}

// Translated from 00450550 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fDistanceMultiplier:LOD` in `011ad7f4`.
pub fn fn_00450550(e: &mut Engine, value: f32) {
    e.set_global(0x011a_d7f4, value);
}

// Translated from 00450560 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `bCheckPurgedTextureList:General` in `011f4460`.
pub fn fn_00450560(e: &mut Engine, value: u8) {
    e.set_global(0x011f_4460, value);
}

// Translated from 00450570 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a `Sky`: the object `0043b230` returns for its `pAtmosphere` (+0x20),
/// or null when there is no atmosphere. `TES::TES` stores it as `pObjFog`.
pub fn fn_00450570(e: &mut Engine, this: Ptr) -> Ptr {
    let atmosphere = e.mem.u32(this.addr() + 0x20); // Sky::pAtmosphere (Xbox PDB)
    if atmosphere == 0 {
        Ptr::NULL
    } else {
        e.call(0x0043_b230, &args![atmosphere]).ptr()
    }
}

// Translated from 004505a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a `Sky`: the object `006838b0` returns for its `pSun` (+0x28), or
/// null when there is no sun. `TES::TES` stores it as `pObjLight`.
pub fn fn_004505a0(e: &mut Engine, this: Ptr) -> Ptr {
    let sun = e.mem.u32(this.addr() + 0x28); // Sky::pSun (Xbox PDB)
    if sun == 0 {
        Ptr::NULL
    } else {
        e.call(0x0068_38b0, &args![sun]).ptr()
    }
}

// Translated from 004505d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `TESObjectREFR::TransChangeCallback` (the Havok transform-change
/// callback, read by `SetTransform`) in `01267b68`.
pub fn fn_004505d0(e: &mut Engine, callback: u32) {
    e.set_global(0x0126_7b68, callback);
}

// Translated from 004505e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `TESObjectREFR::DebugDisplayCallback` (read by `PrepareDebugGeom`)
/// in `01267b64`.
pub fn fn_004505e0(e: &mut Engine, callback: u32) {
    e.set_global(0x0126_7b64, callback);
}

// Translated from 004505f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTempNodeManager`'s constructor: the `NiNode` constructor with a
/// capacity of 0, then the class's vtable.
pub fn fn_004505f0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x00a5_ecb0, &args![this, 0u32]);
    e.mem.set_u32(this.addr(), BS_TEMP_NODE_MANAGER_VTABLE);
    this
}

// Translated from 00450620 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTempNodeManager::GetRTTI` (Xbox PDB): the class's `NiRTTI`.
pub fn bs_temp_node_manager_get_rtti(_e: &mut Engine, _this: Ptr) -> Ptr {
    Ptr::new(BS_TEMP_NODE_MANAGER_RTTI)
}

// Translated from 00450630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTempNodeManager::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00c44e20`) and, when bit 0 of `flags` is set, frees the
/// 0xB0-byte object.
pub fn bs_temp_node_manager_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    e.call(0x00c4_4e20, &args![this]);
    if flags & 1 != 0 {
        e.call(NI_OPERATOR_DELETE, &args![this, 0xb0u32]);
    }
    this
}

// Translated from 00450670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fMoveLimitMass:HAVOK` in `011b0128`.
pub fn fn_00450670(e: &mut Engine, value: f32) {
    e.set_global(0x011b_0128, value);
}

// Translated from 00450680 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the square of `fCharacterControllerMultipleStepSpeed` in
/// `011b0150`.
pub fn fn_00450680(e: &mut Engine, value: f32) {
    e.set_global(0x011b_0150, value);
}

// Translated from 00450690 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `fPhysicsDamage1Mass` in `01267bc0`.
pub fn fn_00450690(e: &mut Engine, value: f32) {
    e.set_global(0x0126_7bc0, value);
}

// Translated from 004506a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the ranged-node visibility callback
/// (`TES::RangedNodeVisibilityChangedCB`, `0045c680`) in `01202e08`; read by
/// `SetCurrent`.
pub fn fn_004506a0(e: &mut Engine, callback: u32) {
    e.set_global(0x0120_2e08, callback);
}

// Translated from 004506b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills the LOD fade-out multiplier table (`011ad7b8`): entries 2 and 3 from
/// `fLODFadeOutMultItems` and `fLODFadeOutMultActors`, entries 0, 1, 4, 5, 8
/// and 10 from `fLODFadeOutMultObjects`.
pub fn fn_004506b0(e: &mut Engine) {
    let items = setting_float(e, SETTING_LOD_FADE_OUT_MULT_ITEMS);
    fn_004504f0(e, 2, items);
    let actors = setting_float(e, SETTING_LOD_FADE_OUT_MULT_ACTORS);
    fn_004504f0(e, 3, actors);
    let objects = setting_float(e, SETTING_LOD_FADE_OUT_MULT_OBJECTS);
    for index in [0, 1, 4, 5, 8, 10] {
        fn_004504f0(e, index, objects);
    }
}

// Translated from 00450770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::~TES`: tears the world down. Restores the vtable, flags the data
/// handler as clearing, clears the cell, form and Havok state of the other
/// subsystems, deletes the singletons the constructor created (clearing their
/// globals; `pGridCellA` keeps its dangling pointer), frees the two cell
/// buffers and runs the member destructors.
///
/// Callees in this unit that nobody has translated yet (`004539a0`,
/// `00458f90`, `0045ac80`, `0045a520`, `0045a1c0`, `0045a350`, `00450d00`,
/// `00450c20`, `0045bb80`) are called by address.
pub fn fn_00450770(e: &mut Engine, this: Ptr<TES>) {
    let base = this.addr();
    e.mem.set_u32(base, TES_VTABLE);
    fn_004506a0(e, 0);
    let data_handler: u32 = e.global(DATA_HANDLER);
    fn_00450b40(e, Ptr::new(data_handler), 1);
    e.call(0x0086_7840, &args![OBJECT_011DE7B8]);
    e.call(0x0052_7f40, &args![]);
    let object: u32 = e.global(OBJECT_WITH_SLOT_1CC);
    e.vcall(object, 0x1cc, &args![0u32, 0u32]);
    let save_load: u32 = e.global(SAVE_LOAD_GAME);
    e.call(0x008a_8150, &args![save_load, 0u32]);
    e.set(this, TES::pObjFog, Ptr::NULL);
    e.set(this, TES::pObjLight, Ptr::NULL);
    e.set(this, TES::pSky, Ptr::NULL);
    e.call(0x0097_7540, &args![OBJECT_011E0E80]);
    e.call(0x0045_39a0, &args![this, 0u32, 0u32]);
    fn_00450b60(e, this);
    e.call(0x0045_8f90, &args![this]);
    e.call(0x0065_1e30, &args![]);
    e.call(0x0066_4cd0, &args![0u32]);
    e.call(0x0045_ac80, &args![this]);
    e.call(0x0045_a520, &args![this]);
    e.call(0x0045_a1c0, &args![this]);
    e.call(0x0045_a350, &args![this]);
    e.call(0x0068_b4a0, &args![]);
    e.call(0x004d_5d50, &args![]);
    e.call(0x009c_6ba0, &args![VATS_OBJECT]);
    // `TESDataHandler::ClearData`.
    let data_handler: u32 = e.global(DATA_HANDLER);
    e.call(0x0045_dfe0, &args![data_handler]);
    e.call(0x0045_0d00, &args![this]);
    let data_handler: u32 = e.global(DATA_HANDLER);
    fn_00450b40(e, Ptr::new(data_handler), 1);
    e.call(0x00c4_59d0, &args![0u32]);

    let model_loader: u32 = e.global(MODEL_LOADER);
    if model_loader != 0 {
        fn_00450b90(e, Ptr::new(model_loader), 1);
    }
    e.set_global(MODEL_LOADER, 0u32);
    let io_manager: u32 = e.global(IO_MANAGER);
    if io_manager != 0 {
        e.vcall(io_manager, 0, &args![1u32]);
    }
    e.set_global(IO_MANAGER, 0u32);
    let task_manager: u32 = e.global(PARALLEL_TASK_MANAGER);
    if task_manager != 0 {
        fn_00450bc0(e, Ptr::new(task_manager), 1);
    }
    e.set_global(PARALLEL_TASK_MANAGER, 0u32);
    let grid_cells = e.get(this, TES::pGridCellA);
    if !grid_cells.is_null() {
        e.vcall(grid_cells.addr(), 0, &args![1u32]);
    }
    let save_load: u32 = e.global(SAVE_LOAD_GAME);
    if save_load != 0 {
        fn_00450bf0(e, Ptr::new(save_load), 1);
    }
    e.set_global(SAVE_LOAD_GAME, 0u32);
    e.call(0x0084_7500, &args![]);
    e.call(0x0084_fd30, &args![]);
    e.call(0x0066_b0d0, &args![base + TES::spLoadedAreaBound.off, 0u32]);
    let data_handler: u32 = e.global(DATA_HANDLER);
    if data_handler != 0 {
        // `TESDataHandler`'s scalar deleting destructor.
        e.call(0x0045_0c20, &args![data_handler, 1u32]);
    }
    e.set_global(DATA_HANDLER, 0u32);
    let interior_buffer = e.get(this, TES::pInteriorBuffer);
    e.call(OPERATOR_DELETE, &args![interior_buffer]);
    let exterior_buffer = e.get(this, TES::pExteriorBuffer);
    e.call(OPERATOR_DELETE, &args![exterior_buffer]);
    let list = e.call(0x0045_bb80, &args![this]).u32();
    e.call(0x0047_0470, &args![list]);
    e.call(0x004e_8000, &args![]);

    for index in 0..SHADOW_SCENE_NODE_COUNT {
        if fn_00450b80(e, index) != 0 {
            let node = fn_00450b80(e, index);
            e.call(0x00b5_d180, &args![node]);
            fn_00450b80(e, index);
            e.call(0x00b6_0040, &args![]);
        }
    }
    e.call(0x0062_5fb0, &args![]);
    e.call(0x004e_e920, &args![base + 0x6c]);
    e.call(0x0045_cec0, &args![base + TES::spLoadedAreaBound.off]);
    e.call(0x0044_cbf0, &args![base + TES::spPreloadedForms.off]);
    e.call(0x0045_cec0, &args![base + TES::BloodDecalPreload1.off]);
    e.call(0x0044_cbf0, &args![base + TES::spPreloadedAddonNodes.off]);
    e.call(0x0046_ffb0, &args![base + TES::DeadCount.off]);
    e.call(0x0046_ffb0, &args![base + TES::ListofBedsAndChairs.off]);
    e.call(0x0046_ffb0, &args![base + TES::listLastLoadedExteriors.off]);
    e.call(0x004e_e840, &args![base + 0x6c]);
}

// Translated from 00450b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `TESDataHandler::bClearingData` (+0x61D, Xbox PDB) on the data handler
/// `this`.
pub fn fn_00450b40(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x61d, value); // TESDataHandler::bClearingData (Xbox PDB)
}

// Translated from 00450b60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00470470` on `TES::ListofBedsAndChairs` (+0x94).
pub fn fn_00450b60(e: &mut Engine, this: Ptr<TES>) {
    e.call(
        0x0047_0470,
        &args![this.addr() + TES::ListofBedsAndChairs.off],
    );
}

// Translated from 00450b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Entry `index` of the table of four shadow scene nodes at `011f91c8`.
pub fn fn_00450b80(e: &mut Engine, index: u32) -> u32 {
    e.global(SHADOW_SCENE_NODE_TABLE.wrapping_add(index.wrapping_mul(4)))
}

// Translated from 00450b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader`'s scalar deleting destructor: `ModelLoader::~ModelLoader`
/// (`00442aa0`), then `operator delete` when bit 0 of `flags` is set.
pub fn fn_00450b90(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x0044_2aa0, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 00450bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSParallelTaskManager`'s scalar deleting destructor (`00c44b70`, then
/// `operator delete` when bit 0 of `flags` is set).
pub fn fn_00450bc0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x00c4_4b70, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 00450bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESSaveLoadGame`'s scalar deleting destructor (`00856870`, then
/// `operator delete` when bit 0 of `flags` is set).
pub fn fn_00450bf0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x0085_6870, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 00450c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler`'s scalar deleting destructor: the destructor (`0045d970`),
/// then `operator delete` when bit 0 of `flags` is set. `TES::~TES` calls it by
/// address.
pub fn fn_00450c20(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x0045_d970, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 00450c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the water system: a `TESWaterSystem` (0xA0 bytes, constructor
/// `004e1650`) allocated under the memory tag `0x1d`, stored in
/// `TES::pWaterSystem` (a failed allocation stores null).
///
/// Not translated: the compiler's exception-unwinding frame.
pub fn fn_00450c50(e: &mut Engine, this: Ptr<TES>) {
    e.with_stack(SCOPE_GUARD_SIZE, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0x1du32, 1u32, TES_CPP_PATH, 0x286u32],
        );
        let block = e.call(OPERATOR_NEW, &args![0xa0u32]).u32();
        let water_system = if block == 0 {
            0
        } else {
            e.call(0x004e_1650, &args![block]).u32()
        };
        e.set(this, TES::pWaterSystem, Ptr::new(water_system));
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
    });
}

// Translated from 00450d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the water system (through its scalar deleting destructor
/// `00450d50`, with the delete flag) and clears `TES::pWaterSystem`.
pub fn fn_00450d00(e: &mut Engine, this: Ptr<TES>) {
    let water_system = e.get(this, TES::pWaterSystem);
    if !water_system.is_null() {
        fn_00450d50(e, water_system, 1);
    }
    e.set(this, TES::pWaterSystem, Ptr::NULL);
}

// Translated from 00450d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterSystem`'s scalar deleting destructor: the destructor (`004e19c0`),
/// then `operator delete` when bit 0 of `flags` is set.
pub fn fn_00450d50(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x004e_19c0, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 00450d80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resets the centre cell and the queued and saved centre cells to
/// `0x7fffffff` (no cell): `iCurrentGridX/Y`, `iCurrentQueuedX/Y`,
/// `iSaveGridX/Y`.
pub fn fn_00450d80(e: &mut Engine, this: Ptr<TES>) {
    e.set(this, TES::iCurrentGridX, i32::MAX);
    e.set(this, TES::iCurrentGridY, i32::MAX);
    e.set(this, TES::iCurrentQueuedX, i32::MAX);
    e.set(this, TES::iCurrentQueuedY, i32::MAX);
    e.set(this, TES::iSaveGridX, i32::MAX);
    e.set(this, TES::iSaveGridY, i32::MAX);
}

// Translated from 00450dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::CullGridCells` (Xbox PDB): attaches (`detach` = 0) or detaches
/// (`detach` != 0) every loaded grid cell, with the bookkeeping around it:
/// the exterior Havok world is told first (`00c66300`/`00c66310`) and last
/// (`00c6b540`/`00c68f00`), `pObjLandRoot` is flagged (`0043b370`), the
/// distant terrain is switched (`006fd1f0`) and the land root is updated.
/// The second stack word is not used. Does nothing without a data handler.
///
/// A cell is attached when its load state is 3 and detached when it is 6.
pub fn tes_cull_grid_cells(e: &mut Engine, this: Ptr<TES>, detach: u8, _unused: u32) {
    let data_handler: u32 = e.global(DATA_HANDLER);
    if data_handler == 0 {
        return;
    }
    let world = fn_00451010(e);
    if world != 0 {
        let world = fn_00451010(e);
        if detach != 0 {
            e.call(0x00c6_6310, &args![world]);
        } else {
            e.call(0x00c6_6300, &args![world]);
        }
    }
    let mut x = 0u32;
    while x < grids_to_load(e) {
        let mut y = 0u32;
        while y < grids_to_load(e) {
            let grid_cells = e.get(this, TES::pGridCellA);
            let slot = e.call(GRID_CELL_ARRAY_GET, &args![grid_cells, x, y]).u32();
            let cell = e.mem.u32(slot);
            if detach != 0 {
                if cell != 0 && fn_00450ff0(e, Ptr::new(cell)) {
                    e.call(0x0055_2bd0, &args![cell, 0u32]);
                }
            } else if cell != 0 && fn_00450fb0(e, Ptr::new(cell)) {
                let root = e.get(this, TES::pObjRoot);
                let grid_cells = e.get(this, TES::pGridCellA);
                e.call(
                    GRID_CELL_ARRAY_ATTACH_TO_WORLD,
                    &args![grid_cells, root, x, y],
                );
                e.call(0x0054_bcf0, &args![cell, 0u32]);
            }
            y += 1;
        }
        x += 1;
    }
    let world = fn_00451010(e);
    if world != 0 {
        let world = fn_00451010(e);
        if detach != 0 {
            e.call(0x00c6_8f00, &args![world, 0u32]);
        } else {
            e.call(0x00c6_b540, &args![world, 0u32]);
        }
    }
    if detach == 0 {
        e.call(0x0097_5f90, &args![OBJECT_011E0E80]);
    }
    let land_root = e.get(this, TES::pObjLandRoot);
    let node = e.call(0x0096_11e0, &args![land_root]).u32();
    if node != 0 {
        fn_00450f90(e, Ptr::new(node), detach);
        e.call(0x006f_d1f0, &args![(detach == 0) as u32]);
    }
    e.with_stack(12, |e, update| {
        e.call(UPDATE_OBJECT_CTOR, &args![update, 0.0f32, 0u8, 0u8]);
        let land_root = e.get(this, TES::pObjLandRoot);
        e.call(NODE_UPDATE, &args![land_root, update]);
    });
}

// Translated from 00450f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` != 0) or clears bit 0 of the flags word at +0x30 of `this`
/// (a node), through `0043b370(flag, 1)`.
pub fn fn_00450f90(e: &mut Engine, this: Ptr, flag: u8) {
    e.call(0x0043_b370, &args![this, flag, 1u32]);
}

// Translated from 00450fb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether a `TESObjectCELL`'s load state is 3.
pub fn fn_00450fb0(e: &mut Engine, this: Ptr) -> bool {
    fn_00450fd0(e, this) == 3
}

// Translated from 00450fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `TESObjectCELL`'s load state: `cCellState` (Xbox PDB `+0x36`), at +0x26
/// on the PC build.
pub fn fn_00450fd0(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x26) // TESObjectCELL::cCellState (Xbox PDB)
}

// Translated from 00450ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether a `TESObjectCELL`'s load state is 6.
pub fn fn_00450ff0(e: &mut Engine, this: Ptr) -> bool {
    fn_00450fd0(e, this) == 6
}

// Translated from 00451010 (decompiled, FalloutNV.exe 1.4.0.525)
/// The exterior Havok world: the first word of `TESObjectCELL::spExteriorWorld`
/// (a `NiPointer<bhkWorldM>`, read through `00559450`).
pub fn fn_00451010(e: &mut Engine) -> u32 {
    e.call(0x0055_9450, &args![EXTERIOR_WORLD]).u32()
}

// Translated from 00451020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the world-space rectangle of the loaded grid into `out` (four
/// words): `[0]` and `[3]` are the x and y of the first loaded cell's corner
/// plus `margin`, `[2]` and `[1]` the x and y of the far corner (one cell
/// further) minus `margin`. Cells are 0x1000 units; `uGridsToLoad` is read
/// four times, as the code does, and each corner is truncated with `_ftol2`.
pub fn fn_00451020(e: &mut Engine, this: Ptr<TES>, out: Ptr, margin: f32) {
    let half_x = grids_to_load(e) >> 1;
    let first_x = e.get(this, TES::iCurrentGridX).wrapping_sub(half_x as i32);
    let half_y = grids_to_load(e) >> 1;
    let first_y = e.get(this, TES::iCurrentGridY).wrapping_sub(half_y as i32);
    let half_x = grids_to_load(e) >> 1;
    let last_x = (half_x as i32).wrapping_add(e.get(this, TES::iCurrentGridX));
    let half_y = grids_to_load(e) >> 1;
    let last_y = (half_y as i32).wrapping_add(e.get(this, TES::iCurrentGridY));
    let margin = margin as f64;
    let corner = first_x.wrapping_shl(12);
    let value = e.call(FTOL, &args![corner as f64 + margin]).i32();
    e.mem.set_i32(out.addr(), value);
    let corner = first_y.wrapping_shl(12);
    let value = e.call(FTOL, &args![corner as f64 + margin]).i32();
    e.mem.set_i32(out.addr() + 0xc, value);
    let corner = last_x.wrapping_shl(12).wrapping_add(0x1000);
    let value = e.call(FTOL, &args![corner as f64 - margin]).i32();
    e.mem.set_i32(out.addr() + 8, value);
    let corner = last_y.wrapping_shl(12).wrapping_add(0x1000);
    let value = e.call(FTOL, &args![corner as f64 - margin]).i32();
    e.mem.set_i32(out.addr() + 4, value);
}

// Translated from 00451110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the position `pos` (two floats, x and y) lies in the loaded grid:
/// always true while an interior cell is loaded; otherwise `pos` is rounded
/// (`00406d90`), divided by 0x1000 into cell coordinates, and compared with
/// the `uGridsToLoad` square around the centre cell.
pub fn fn_00451110(e: &mut Engine, this: Ptr<TES>, pos: Ptr) -> bool {
    if e.call(GET_INTERIOR_CELL, &args![this]).u32() != 0 {
        return true;
    }
    let x_value = e.mem.f32(pos.addr());
    let cell_x = e.call(FLOAT_TO_INT_ROUNDED, &args![x_value]).i32() >> 12;
    let y_value = e.mem.f32(pos.addr() + 4);
    let cell_y = e.call(FLOAT_TO_INT_ROUNDED, &args![y_value]).i32() >> 12;
    let half = grids_to_load(e) >> 1;
    let first_x = e.get(this, TES::iCurrentGridX).wrapping_sub(half as i32);
    let half = grids_to_load(e) >> 1;
    let first_y = e.get(this, TES::iCurrentGridY).wrapping_sub(half as i32);
    if cell_x < first_x {
        return false;
    }
    let count = grids_to_load(e);
    if cell_x >= first_x.wrapping_add(count as i32) {
        return false;
    }
    if cell_y < first_y {
        return false;
    }
    let count = grids_to_load(e);
    cell_y < first_y.wrapping_add(count as i32)
}

// Translated from 004511e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::IsCellLoaded` (Xbox PDB): false for a null `cell`; by load state
/// (`this` is not used): states 2 to 4 give `flag == 0`, states 5 and 6 give
/// true, every other state false.
pub fn tes_is_cell_loaded(e: &mut Engine, _this: Ptr<TES>, cell: Ptr, flag: u8) -> bool {
    if cell.is_null() {
        return false;
    }
    let index = (fn_00450fd0(e, cell) as u32).wrapping_sub(2);
    match index {
        0..=2 => flag == 0,
        3 | 4 => true,
        _ => false,
    }
}

// Translated from 00451250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` != 0) or clears bit 1 of a `TESObjectCELL`'s `cCellFlags`
/// (+0x24 on PC; Xbox PDB `+0x34`): the "has water" bit, which `004518e0`
/// reads.
pub fn fn_00451250(e: &mut Engine, cell: Ptr, flag: u8) {
    let flags = e.mem.u8(cell.addr() + 0x24); // TESObjectCELL::cCellFlags (Xbox PDB)
    let flags = if flag != 0 { flags | 2 } else { flags & !2 };
    e.mem.set_u8(cell.addr() + 0x24, flags);
}

// Translated from 004512a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a `TESObjectCELL`'s load state (`cCellState`, +0x26).
pub fn fn_004512a0(e: &mut Engine, cell: Ptr, state: u8) {
    e.mem.set_u8(cell.addr() + 0x26, state); // TESObjectCELL::cCellState (Xbox PDB)
}

// Translated from 004512c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Moves the loaded grid to the position `pos` (two floats) and loads it.
/// Does nothing without a data handler, with an interior cell loaded, or
/// while it runs (the byte at `011c3ed0` guards against re-entry).
///
/// With `reload` != 0 (a full reload): the grid cell array is told (slot 8),
/// the terrain manager is updated from the player's position, the grid is
/// repositioned (slot 0x10 with the centre cell), the models and textures
/// are reset and the grid loaded (`00452ff0`), the player's camera
/// position goes to `0057d0a0`, a fader is created from
/// `fFadeToBlackFadeSeconds` unless cell tests are running, and the
/// furniture and bed lists are rebuilt. Without it: the grid is
/// repositioned and loaded. Both finish by updating the land root.
pub fn fn_004512c0(e: &mut Engine, this: Ptr<TES>, pos: Ptr, reload: u8) {
    let data_handler: u32 = e.global(DATA_HANDLER);
    if data_handler == 0 {
        return;
    }
    if e.call(GET_INTERIOR_CELL, &args![this]).u32() != 0 {
        return;
    }
    if e.global::<u8>(GRID_LOAD_IN_PROGRESS) != 0 {
        return;
    }
    e.set_global(GRID_LOAD_IN_PROGRESS, 1u8);
    if reload != 0 {
        let grid_cells = e.get(this, TES::pGridCellA);
        e.vcall(grid_cells.addr(), 0x8, &args![]);
        fn_00451520(e, 1);
        fn_00451570(e, 0);
        let world_space = e.get(this, TES::pWorldSpace);
        let terrain = e.call(GET_TERRAIN_MANAGER, &args![world_space]).u32();
        e.call(0x006f_ce00, &args![terrain]);
        let player: u32 = e.global(PLAYER_CHARACTER);
        // `PlayerCharacter` slot 0x1f4 returns the position (three floats).
        let player_position = e.vcall(player, 0x1f4, &args![]).u32();
        let world_space = e.get(this, TES::pWorldSpace);
        let terrain = e.call(GET_TERRAIN_MANAGER, &args![world_space]).u32();
        e.call(0x006f_ca90, &args![terrain, player_position, 0xfu32]);
        let grid_x = e.get(this, TES::iCurrentGridX);
        let grid_y = e.get(this, TES::iCurrentGridY);
        let grid_cells = e.get(this, TES::pGridCellA);
        e.vcall(grid_cells.addr(), 0x10, &args![grid_x, grid_y]);
        e.call(0x0045_5200, &args![this, 0u32, 0u32]);
        e.call(0x0045_2490, &args![this, 0u32]);
        e.call(0x0045_7be0, &args![this, 0u32]);
        e.call(0x0045_2ff0, &args![this]);
        let io_manager: u32 = e.global(IO_MANAGER);
        e.call(0x00c3_dfa0, &args![io_manager, 4u32]);
        // A 12-byte local: the position (x, y, 0.0) built by `00416870`; the
        // third float is then overwritten by `004572e0`.
        e.with_stack(12, |e, point| {
            let x = e.mem.f32(pos.addr());
            let y = e.mem.f32(pos.addr() + 4);
            e.call(0x0041_6870, &args![point, x, y, 0.0f32]);
            e.call(0x0045_72e0, &args![this, pos, point.addr() + 8]);
            fn_00451590(e, 0);
            let point_words = [
                e.mem.u32(point.addr()),
                e.mem.u32(point.addr() + 4),
                e.mem.u32(point.addr() + 8),
            ];
            let global_words = [
                e.global::<u32>(POINT_011A9478),
                e.global::<u32>(POINT_011A9478 + 4),
                e.global::<u32>(POINT_011A9478 + 8),
            ];
            e.call(
                0x0057_d0a0,
                &args![
                    point_words[0],
                    point_words[1],
                    point_words[2],
                    global_words[0],
                    global_words[1],
                    global_words[2],
                    1.0f32
                ],
            );
        });
        fn_00451590(e, 1);
        if !fn_00451530(e, this) {
            let seconds = setting_float(e, SETTING_FADE_TO_BLACK_FADE_SECONDS);
            let fader_manager: u32 = e.global(FADER_MANAGER);
            // `FaderManager::CreateFader` (Xbox PDB): the trailing 0 is the
            // word the code pushed before reading the setting.
            e.call(0x0070_0960, &args![fader_manager, 1u32, seconds, 0u32]);
        }
        e.call(0x0045_9870, &args![this]);
        e.call(0x0097_2d30, &args![OBJECT_011E0E80]);
        e.call(0x0097_2bb0, &args![OBJECT_011E0E80]);
    } else {
        let grid_x = e.get(this, TES::iCurrentGridX);
        let grid_y = e.get(this, TES::iCurrentGridY);
        let grid_cells = e.get(this, TES::pGridCellA);
        e.vcall(grid_cells.addr(), 0x10, &args![grid_x, grid_y]);
        e.call(0x0045_7be0, &args![this, 1u32]);
        e.call(0x0045_2ff0, &args![this]);
    }
    e.call(0x0045_5490, &args![this]);
    e.with_stack(12, |e, update| {
        e.call(UPDATE_OBJECT_CTOR, &args![update, 0.0f32, 0u8, 0u8]);
        let land_root = e.get(this, TES::pObjLandRoot);
        e.call(NODE_UPDATE, &args![land_root, update]);
    });
    e.call(0x0045_2490, &args![this, 0u32]);
    let data_handler: u32 = e.global(DATA_HANDLER);
    e.call(0x0046_0360, &args![data_handler]);
    e.set_global(GRID_LOAD_IN_PROGRESS, 0u8);
}

// Translated from 00451520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a byte in `01189184` (also written by `ClearCanopyShadowMaskTexture`).
pub fn fn_00451520(e: &mut Engine, value: u8) {
    e.set_global(GRID_FLAG_01189184, value);
}

// Translated from 00451530 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `bRunningCellTests` or `bRunningCellTests2` is set.
pub fn fn_00451530(e: &mut Engine, this: Ptr<TES>) -> bool {
    e.get(this, TES::bRunningCellTests) || e.get(this, TES::bRunningCellTests2)
}

// Translated from 00451570 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `0066b0d0` on the object at `011f95e8` with `value`.
pub fn fn_00451570(e: &mut Engine, value: u32) {
    e.call(0x0066_b0d0, &args![0x011f_95e8u32, value]);
}

// Translated from 00451590 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a byte in `011ad86c`.
pub fn fn_00451590(e: &mut Engine, value: u8) {
    e.set_global(GRID_FLAG_011AD86C, value);
}

// Translated from 004515a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Centres the grid on `pos` (two floats; null keeps the current centre) and
/// loads it (`004512c0` with `flag`), then calls `0062f460`. Runs when a data
/// handler exists, no interior cell is loaded and the data handler's
/// `bSaveLoad` (+0x61a) is clear, or when `0047c850` (called on the
/// `TESSaveLoadGame` object) says so. Without a world space, the data
/// handler's current one (the object `00460140` returns) is set first.
pub fn fn_004515a0(e: &mut Engine, this: Ptr<TES>, pos: Ptr, flag: u8) {
    let save_load: u32 = e.global(SAVE_LOAD_GAME);
    let forced = e.call(0x0047_c850, &args![save_load]).u8();
    let data_handler: u32 = e.global(DATA_HANDLER);
    let normal = data_handler != 0
        && e.call(GET_INTERIOR_CELL, &args![this]).u32() == 0
        && !fn_004516b0(e, Ptr::new(data_handler));
    if !normal && forced == 0 {
        return;
    }
    let saved_x = e.get(this, TES::iCurrentGridX);
    let saved_y = e.get(this, TES::iCurrentGridY);
    if e.call(GET_WORLD_SPACE, &args![this]).u32() == 0 {
        let data_handler: u32 = e.global(DATA_HANDLER);
        let holder = e.call(0x0046_0140, &args![data_handler]).u32();
        let slot = e.call(0x0068_15c0, &args![holder]).u32();
        let world_space = e.mem.u32(slot);
        e.call(0x0045_8200, &args![this, world_space]);
    }
    if pos.is_null() {
        e.set(this, TES::iCurrentGridX, saved_x);
        e.set(this, TES::iCurrentGridY, saved_y);
    } else {
        let x_value = e.mem.f32(pos.addr());
        let cell_x = e.call(FLOAT_TO_INT_ROUNDED, &args![x_value]).i32() >> 12;
        e.set(this, TES::iCurrentGridX, cell_x);
        let y_value = e.mem.f32(pos.addr() + 4);
        let cell_y = e.call(FLOAT_TO_INT_ROUNDED, &args![y_value]).i32() >> 12;
        e.set(this, TES::iCurrentGridY, cell_y);
    }
    let grid_x = e.get(this, TES::iCurrentGridX);
    e.set(this, TES::iCurrentQueuedX, grid_x);
    let grid_y = e.get(this, TES::iCurrentGridY);
    e.set(this, TES::iCurrentQueuedY, grid_y);
    fn_004512c0(e, this, pos, flag);
    e.call(0x0062_f460, &args![]);
}

// Translated from 004516b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `TESDataHandler`'s `bSaveLoad` (+0x61a, Xbox PDB).
pub fn fn_004516b0(e: &mut Engine, data_handler: Ptr) -> bool {
    e.mem.u8(data_handler.addr() + 0x61a) != 0 // TESDataHandler::bSaveLoad (Xbox PDB)
}

// Translated from 004516d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::FindFirstGridCellWithWater` (Xbox PDB): searches the loaded grid
/// outwards from its centre, ring by ring (the centre cell, then the square
/// ring `n` cells away: first the two rows, then the two columns), and
/// returns the address of the grid slot (`GridCellArray::Get`) of the first
/// loaded cell whose "has water" bit (`004518e0`) is set, or null.
pub fn tes_find_first_grid_cell_with_water(e: &mut Engine, this: Ptr<TES>) -> Ptr {
    let grids = grids_to_load(e);
    let half = grids >> 1;
    let center = half;
    let mut found = 0u32;
    let mut ring = 0u32;
    while ring < half {
        if ring == 0 {
            found = grid_slot_with_water(e, this, center, center);
        } else {
            let high = center.wrapping_add(ring);
            let low = center.wrapping_sub(ring);
            let mut x = low;
            while x <= high {
                found = grid_slot_with_water(e, this, x, high);
                if found != 0 {
                    break;
                }
                found = grid_slot_with_water(e, this, x, low);
                if found != 0 {
                    break;
                }
                x = x.wrapping_add(1);
            }
            if found == 0 {
                let mut y = low.wrapping_add(1);
                while y < high {
                    found = grid_slot_with_water(e, this, low, y);
                    if found != 0 {
                        break;
                    }
                    found = grid_slot_with_water(e, this, high, y);
                    if found != 0 {
                        break;
                    }
                    y = y.wrapping_add(1);
                }
            }
        }
        if found != 0 {
            break;
        }
        ring += 1;
    }
    Ptr::new(found)
}

/// The grid slot `(x, y)` when it holds a cell with the "has water" bit set,
/// else 0 (the test the water search repeats).
fn grid_slot_with_water(e: &mut Engine, this: Ptr<TES>, x: u32, y: u32) -> u32 {
    let grid_cells = e.get(this, TES::pGridCellA);
    let slot = e.call(GRID_CELL_ARRAY_GET, &args![grid_cells, x, y]).u32();
    if slot != 0 {
        let cell = e.mem.u32(slot);
        if cell != 0 && fn_004518e0(e, Ptr::new(cell)) {
            return slot;
        }
    }
    0
}

// Translated from 004518e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether a `TESObjectCELL`'s "has water" bit (bit 1 of `cCellFlags`, +0x24)
/// is set.
pub fn fn_004518e0(e: &mut Engine, cell: Ptr) -> bool {
    e.mem.u8(cell.addr() + 0x24) & 2 != 0 // TESObjectCELL::cCellFlags (Xbox PDB)
}

// Translated from 00451900 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cell at cell coordinates `(x, y)`: the loaded grid's cell when
/// `(x, y)` is inside the `uGridsToLoad` square around the centre cell and
/// the slot is set; otherwise, when there is a world space, what
/// `00461c20(data handler, x, y, world space, 0)` returns; else null.
pub fn fn_00451900(e: &mut Engine, this: Ptr<TES>, x: i32, y: i32) -> u32 {
    let half = grids_to_load(e) >> 1;
    let relative_x =
        (x as u32).wrapping_sub(e.get(this, TES::iCurrentGridX).wrapping_sub(half as i32) as u32);
    let half = grids_to_load(e) >> 1;
    let relative_y =
        (y as u32).wrapping_sub(e.get(this, TES::iCurrentGridY).wrapping_sub(half as i32) as u32);
    let mut cell = 0u32;
    if relative_x < grids_to_load(e)
        && relative_y < grids_to_load(e)
        && (relative_x as i32) >= 0
        && (relative_y as i32) >= 0
    {
        let grid_cells = e.get(this, TES::pGridCellA);
        let slot = e
            .call(
                GRID_CELL_ARRAY_GET,
                &args![grid_cells, relative_x, relative_y],
            )
            .u32();
        cell = e.mem.u32(slot);
    }
    if cell == 0 && e.call(GET_WORLD_SPACE, &args![this]).u32() != 0 {
        let world_space = e.call(GET_WORLD_SPACE, &args![this]).u32();
        let data_handler: u32 = e.global(DATA_HANDLER);
        cell = e
            .call(0x0046_1c20, &args![data_handler, x, y, world_space, 0u32])
            .u32();
    }
    cell
}

// Translated from 004519d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The loaded cell at the world position `(x, y)`: the interior cell when one
/// is loaded; otherwise the exterior cell (`00451900` at `x / 0x1000`,
/// `y / 0x1000`, truncated) when it is loaded (`TES::IsCellLoaded` on the
/// `TES` singleton with `flag` 1), else null. The third stack word is not
/// used.
pub fn fn_004519d0(e: &mut Engine, this: Ptr<TES>, x: f32, y: f32, _unused: u32) -> u32 {
    if e.call(GET_INTERIOR_CELL, &args![this]).u32() != 0 {
        return e.call(GET_INTERIOR_CELL, &args![this]).u32();
    }
    let cell_x = e.call(FTOL, &args![x as f64]).i32() >> 12;
    let cell_y = e.call(FTOL, &args![y as f64]).i32() >> 12;
    let cell = fn_00451900(e, this, cell_x, cell_y);
    if cell != 0 {
        let tes: u32 = e.global(TES_SINGLETON);
        if tes_is_cell_loaded(e, Ptr::new(tes), Ptr::new(cell), 1) {
            return cell;
        }
    }
    0
}

// Translated from 00451a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::LoadGridCell` (Xbox PDB): loads the cell at world cell
/// `(cell_x, cell_y)` and puts it in grid slot `(array_x, array_y)`.
/// Does nothing without a world space. A cell the world space does not have
/// is created by `00461c20` and gets its persistent references
/// (`TESWorldSpace::AssignPersistentRefsToCell`) and temp data. A slot that
/// is already attached only has the cell added to the buffer
/// (`TES::AddToBuffer`). Otherwise the cell is stored in the slot and
/// attached to the scene root, its land mesh is loaded (or updated) and its
/// "has water" bit set from the lowest land height against the water
/// height; with `bUseWater`, a cell with water gets its placeable water
/// generated; the land is then updated with `bShowLANDborders`.
///
/// The "Loading cell..." message is formatted into a 268-byte buffer and not
/// used further, as in the game.
pub fn tes_load_grid_cell(
    e: &mut Engine,
    this: Ptr<TES>,
    array_x: u32,
    array_y: u32,
    cell_x: i32,
    cell_y: i32,
) {
    if e.call(GET_WORLD_SPACE, &args![this]).u32() == 0 {
        return;
    }
    let world_space = e.call(GET_WORLD_SPACE, &args![this]).u32();
    let mut cell = e
        .call(0x0058_5b30, &args![world_space, cell_x, cell_y])
        .u32();
    if cell == 0 {
        let world_space = e.call(GET_WORLD_SPACE, &args![this]).u32();
        let data_handler: u32 = e.global(DATA_HANDLER);
        cell = e
            .call(
                0x0046_1c20,
                &args![data_handler, cell_x, cell_y, world_space, 1u32],
            )
            .u32();
        let world_space = e.call(GET_WORLD_SPACE, &args![this]).u32();
        e.call(0x0058_8150, &args![world_space, cell]);
        e.call(0x0055_1440, &args![cell, 1u32]);
    }
    let grid_cells = e.get(this, TES::pGridCellA);
    if e.call(0x004b_a5a0, &args![grid_cells, array_x, array_y])
        .bool()
    {
        e.call(0x0045_4b90, &args![this, cell]);
        return;
    }
    let form_id = e.call(0x0084_e3a0, &args![cell]).u32();
    let data_y = e.call(CELL_GET_DATA_Y, &args![cell]).u32();
    let data_x = e.call(CELL_GET_DATA_X, &args![cell]).u32();
    let name = fn_00451cb0(e, Ptr::new(cell));
    e.with_stack(268, |e, message| {
        e.call(
            0x00ec_623a,
            &args![message, LOADING_CELL_FORMAT, name, data_x, data_y, form_id],
        );
    });
    let grid_cells = e.get(this, TES::pGridCellA);
    e.call(0x004b_a550, &args![grid_cells, array_x, array_y, cell]);
    let root = e.get(this, TES::pObjRoot);
    let grid_cells = e.get(this, TES::pGridCellA);
    e.call(
        GRID_CELL_ARRAY_ATTACH_TO_WORLD,
        &args![grid_cells, root, array_x, array_y],
    );
    let land = e.call(0x0054_6fb0, &args![cell]).u32();
    if !e.call(0x0053_5b90, &args![land, 1u32]).bool() {
        e.call(0x0053_a090, &args![land, 0u32, 0u32]);
    }
    e.with_stack(8, |e, heights| {
        let cell_land = e.call(0x0054_6fb0, &args![cell]).u32();
        e.call(0x0053_f440, &args![cell_land, heights]);
        let water_height = e.call(0x0054_71e0, &args![cell]).f64();
        let lowest = e.mem.f32(heights.addr()) as f64;
        // `FCOMPP`: ordered and lowest land height below the water height.
        fn_00451250(e, Ptr::new(cell), (lowest < water_height) as u8);
    });
    if setting_byte(e, SETTING_USE_WATER) != 0
        && fn_004518e0(e, Ptr::new(cell))
        && fn_00451cd0(e, Ptr::new(cell)) == 0
    {
        e.call(0x0049_c860, &args![cell]);
    }
    let show_borders = e.get(this, TES::bShowLANDborders);
    e.call(0x0053_fa40, &args![land, show_borders]);
}

// Translated from 00451cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00401280` on the cell and returns its result (the map names it
/// `BaseProcess::GetCurrentHeadTrackTypeString`; the body returns a constant
/// string pointer, `01011584`).
pub fn fn_00451cb0(e: &mut Engine, cell: Ptr) -> u32 {
    e.call(0x0040_1280, &args![cell]).u32()
}

// Translated from 00451cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `TESObjectCELL`'s `bAutoWaterLoaded` (+0x54 on PC; Xbox PDB `+0x64`).
pub fn fn_00451cd0(e: &mut Engine, cell: Ptr) -> u8 {
    e.mem.u8(cell.addr() + 0x54) // TESObjectCELL::bAutoWaterLoaded (Xbox PDB)
}

// Translated from 00451cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::UpdateGridLights` (Xbox PDB): attaches the lights (`TESObjectCELL::
/// AttachLights`, flag 1) of every loaded cell whose load state is 6, then
/// updates shadow scene node 0 (`00b5ddb0(1, 1)`).
pub fn tes_update_grid_lights(e: &mut Engine, this: Ptr<TES>) {
    let mut x = 0u32;
    while x < grids_to_load(e) {
        let mut y = 0u32;
        while y < grids_to_load(e) {
            let grid_cells = e.get(this, TES::pGridCellA);
            let slot = e.call(GRID_CELL_ARRAY_GET, &args![grid_cells, x, y]).u32();
            let cell = e.mem.u32(slot);
            if cell != 0 && fn_00450ff0(e, Ptr::new(cell)) {
                e.call(0x0054_ba80, &args![cell, 1u32]);
            }
            y += 1;
        }
        x += 1;
    }
    let shadow_scene_node = fn_00450b80(e, 0);
    e.call(0x00b5_ddb0, &args![shadow_scene_node, 1u32, 1u32]);
}

// Translated from 00451da0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a reference `refr` (`this` is not used) for which `00564e60` and
/// `005651e0` hold and whose virtual slot 0x1d0 returns an object: gives that
/// object to slot 0xe8 of the object `009611e0` derives from it (when there
/// is one), then calls `00570f70(refr, 0)` and
/// `TESActorBase::SetStartsDead(refr, 0)` (`00565210`).
pub fn fn_00451da0(e: &mut Engine, _this: Ptr<TES>, refr: Ptr) {
    let r = refr.addr();
    if !e.call(0x0056_4e60, &args![r]).bool() {
        return;
    }
    if e.vcall(r, 0x1d0, &args![]).u32() == 0 {
        return;
    }
    if !e.call(0x0056_51e0, &args![r]).bool() {
        return;
    }
    let parent = e.vcall(r, 0x1d0, &args![]).u32();
    let handler = e.call(0x0096_11e0, &args![parent]).u32();
    if handler != 0 {
        let parent = e.vcall(r, 0x1d0, &args![]).u32();
        e.vcall(handler, 0xe8, &args![parent]);
    }
    e.call(0x0057_0f70, &args![r, 0u32]);
    e.call(0x0056_5210, &args![r, 0u32]);
}

// Translated from 00451e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the reference `refr` (`this` is not used) is to be loaded: neither
/// `00440d80` nor `00440da0` says no. (The form type the code also reads
/// through `00401170` is not used.)
pub fn fn_00451e40(e: &mut Engine, _this: Ptr<TES>, refr: Ptr) -> bool {
    let form = e.call(REFERENCE_GET_BASE_FORM, &args![refr]).u32();
    e.call(FORM_GET_TYPE, &args![form]);
    if e.call(0x0044_0d80, &args![refr]).bool() {
        return false;
    }
    !e.call(0x0044_0da0, &args![refr]).bool()
}

// Translated from 00451ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For an actor reference `refr` (`this` is not used; virtual slot 0x100
/// true): runs slot 0x260 on it and takes it off the temp-change list of
/// `ProcessLists` (`0096f400` on `011e0e80`).
pub fn fn_00451ea0(e: &mut Engine, _this: Ptr<TES>, refr: Ptr) {
    let r = refr.addr();
    if e.vcall(r, 0x100, &args![]).bool() {
        e.vcall(r, 0x260, &args![]);
        e.call(0x0096_f400, &args![OBJECT_011E0E80, r]);
    }
}

// Translated from 00451ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Loads one reference `refr` of `cell` into the scene (`this` is the `TES`).
/// `source` is an optional object that can supply the reference's 3D
/// (`005d8710` returns it); when `flag` is 0 the actor is first taken off the
/// temp-change list (`00451ea0`).
///
/// A null reference does nothing; a reference whose virtual slot 0x224 or
/// 0x220 is true is handed to slot 0x308 or 0x304 and nothing else is done.
/// The 3D (`node`) comes from `source`, or from slot 0x1c8. With a node: for
/// form types 0x2c and 0x2d it is given to slot 0xe8 of the object
/// `009611e0` derives from it; otherwise the cell's 3D is loaded, the node is
/// added to shadow scene node 0 (and registered for actors), the reference
/// is attached (`TESObjectCELL::AttachReference3D`) unless it is a type 0x23
/// form `00452440` rejects, the master particle addon nodes are removed
/// (form flag `0x1000000` and `0057b200`) or added (loaded-data flag 2, in a
/// cell of load state 6 or 5), and the object lighting is updated (form type
/// 0x1e takes `004534f0` instead). Always: the script is initialised and,
/// unless `0042ce10` says otherwise, its action list set up. For a type 0x25
/// form with the cast and size conditions, `00547ad0` is called with the
/// position and a scaled integer. An actor finally gets
/// `Actor::UpdateAlpha` and, unless it is the player, its reference ID set on
/// the scene graph.
///
/// Argument counts come from the disassembly: the words pushed before the
/// virtual calls `0x1bc` and `0x1f4` are arguments of the call that follows
/// (`00547ad0`, `RET 0x14`), not of the virtual calls.
pub fn fn_00451ef0(e: &mut Engine, this: Ptr<TES>, refr: Ptr, cell: Ptr, source: Ptr, flag: u8) {
    if refr.is_null() {
        return;
    }
    let r = refr.addr();
    if e.vcall(r, 0x224, &args![]).bool() {
        e.vcall(r, 0x308, &args![]);
        return;
    }
    if e.vcall(r, 0x220, &args![]).bool() {
        e.vcall(r, 0x304, &args![]);
        return;
    }
    fn_00451da0(e, this, refr);
    let form = e.call(REFERENCE_GET_BASE_FORM, &args![r]).u32();
    let kind = e.call(FORM_GET_TYPE, &args![form]).u8();
    if !fn_00451e40(e, this, refr) {
        return;
    }
    if flag == 0 {
        fn_00451ea0(e, this, refr);
    }
    let node;
    if !source.is_null() && e.call(0x005d_8710, &args![source]).u32() != 0 {
        node = e.call(0x005d_8710, &args![source]).u32();
        if e.call(0x0043_fcd0, &args![r]).u32() == 0 {
            e.call(0x0057_0f70, &args![r, node]);
        }
    } else {
        node = e.vcall(r, 0x1c8, &args![0u32]).u32();
    }
    if !source.is_null() {
        e.vcall(source.addr(), 0x40, &args![]);
    }
    if node != 0 {
        if kind == 0x2c || kind == 0x2d {
            let handler = e.call(0x0096_11e0, &args![node]).u32();
            if handler != 0 {
                let handler = e.call(0x0096_11e0, &args![node]).u32();
                e.vcall(handler, 0xe8, &args![node]);
            }
        } else {
            e.call(0x0054_5cf0, &args![cell]);
            let shadow_scene_node = fn_00450b80(e, 0);
            e.call(0x00b5_eeb0, &args![shadow_scene_node, node]);
            if e.vcall(r, 0x100, &args![]).bool() {
                let shadow_scene_node = fn_00450b80(e, 0);
                e.call(0x00b5_cbd0, &args![shadow_scene_node, node]);
            }
            if e.call(0x0056_4e60, &args![r]).bool() {
                fn_00452390(e, cell);
            }
            let mut attach = true;
            let form = e.call(REFERENCE_GET_BASE_FORM, &args![r]).u32();
            if e.call(FORM_GET_TYPE, &args![form]).u32() == 0x23 {
                let form = e.call(REFERENCE_GET_BASE_FORM, &args![r]).u32();
                if e.call(0x0045_2440, &args![form]).bool() {
                    attach = false;
                }
            }
            if attach {
                e.call(0x0054_8880, &args![cell, r, 0u32]);
                e.call(0x0056_f700, &args![r]);
            }
            if fn_00452370(e, refr) && e.call(0x0057_b200, &args![r]).bool() {
                e.call(0x0057_8170, &args![node]);
            } else if fn_004523e0(e, refr, 2) && (fn_00450ff0(e, cell) || fn_004523c0(e, cell)) {
                e.call(0x0057_8060, &args![node]);
            }
            if e.vcall(r, 0x100, &args![]).bool() {
                e.call(0x0048_3710, &args![r]);
            }
            if kind == 0x1e {
                let extra = e.call(0x0043_0830, &args![r]).u32();
                let form = e.call(REFERENCE_GET_BASE_FORM, &args![r]).u32();
                e.call(0x0045_34f0, &args![form, extra]);
            } else {
                let shadow_scene_node = fn_00450b80(e, 0);
                e.call(0x00b5_d9f0, &args![shadow_scene_node, node, 0u32]);
            }
        }
    }
    e.call(0x0056_5730, &args![r]);
    let script_context: u32 = e.global(SCRIPT_CONTEXT);
    if !e.call(0x0042_ce10, &args![script_context]).bool() {
        let actions = e.call(0x005d_43c0, &args![r]).u32();
        e.call(0x005a_c190, &args![r, actions]);
        let actions = e.call(0x005d_43c0, &args![r]).u32();
        e.call(0x005a_c750, &args![r, actions, 0x1000u32]);
    }
    if kind == 0x25 {
        let form = e.call(REFERENCE_GET_BASE_FORM, &args![r]).u32();
        let cast = e
            .call(
                0x00ec_43fb,
                &args![form, 0u32, RTTI_TESFORM, RTTI_CAST_TARGET, 0u32],
            )
            .u32();
        if cast != 0 && e.call(0x0045_2480, &args![]).bool() {
            let sizes = e.call(0x0045_bb80, &args![cast]).u32();
            let first = e.mem.f32(sizes);
            let second = e.mem.f32(sizes + 4);
            let limit: f64 = e.global(LARGE_CELL_LIMIT);
            if first as f64 > limit && second as f64 > limit {
                let radius = e.vcall(cast, 0x1bc, &args![]).f64();
                let scale = e.call(0x0056_7400, &args![r]).f64();
                // `fistp` with the rounding mode set to truncate; only the
                // low word of the 64-bit result is used.
                let count = (scale * radius) as i64 as i32;
                let position = e.vcall(r, 0x1f4, &args![]).u32();
                let x = e.mem.u32(position);
                let y = e.mem.u32(position + 4);
                let z = e.mem.u32(position + 8);
                e.call(0x0054_7ad0, &args![cell, x, y, z, count, 1.0f32]);
            }
        }
    }
    if e.vcall(r, 0x100, &args![]).bool() {
        e.call(0x008c_4640, &args![r]);
        let player: u32 = e.global(PLAYER_CHARACTER);
        if r != player {
            let form_id = e.call(0x0084_e3a0, &args![r]).u32();
            let node_3d = e.call(0x0043_fcd0, &args![r]).u32();
            e.call(0x004b_6dc0, &args![node_3d, form_id]);
        }
    }
}

// Translated from 00452370 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x1000000` of a form's `iFormFlags` (+8) is set.
pub fn fn_00452370(e: &mut Engine, form: Ptr) -> bool {
    e.mem.u32(form.addr() + 8) & 0x0100_0000 != 0 // TESForm::iFormFlags (Xbox PDB)
}

// Translated from 00452390 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds 1 to the 16-bit counter at +0xaa of a `TESObjectCELL`; by its use
/// (called for references visible when distant) it counts the loaded
/// references with visible distant (`sNumLoadedRefsWithVisibleDistant`, Xbox
/// PDB `+0x8a`).
pub fn fn_00452390(e: &mut Engine, cell: Ptr) {
    let count = e.mem.u16(cell.addr() + 0xaa);
    e.mem.set_u16(cell.addr() + 0xaa, count.wrapping_add(1));
}

// Translated from 004523c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether a `TESObjectCELL`'s load state is 5.
pub fn fn_004523c0(e: &mut Engine, cell: Ptr) -> bool {
    fn_00450fd0(e, cell) == 5
}

// Translated from 004523e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a `TESObjectREFR`: false without loaded data (`pLoadedData`, +0x64 on
/// PC; Xbox PDB `+0x74`), else whether the loaded data has any of the bits
/// `mask` (`00452420`).
pub fn fn_004523e0(e: &mut Engine, refr: Ptr, mask: u32) -> bool {
    let loaded_data = e.mem.u32(refr.addr() + 0x64); // TESObjectREFR::pLoadedData (Xbox PDB)
    if loaded_data == 0 {
        false
    } else {
        fn_00452420(e, Ptr::new(loaded_data), mask)
    }
}

// Translated from 00452420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the `LOADED_REF_DATA` flags word (+0x10) has any of the bits
/// `mask`.
pub fn fn_00452420(e: &mut Engine, loaded_data: Ptr, mask: u32) -> bool {
    e.mem.u32(loaded_data.addr() + 0x10) & mask != 0
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x0044fb20,
            tes_tes(Ptr<TES>, u32, Ptr, Ptr, Ptr, Ptr) -> Ptr<TES>
        ),
        entry!(0x004503f0, fn_004503f0(Ptr) -> u32),
        entry!(0x00450410, fn_00450410(Ptr) -> f32),
        entry!(0x00450430, fn_00450430(u32)),
        entry!(0x00450440, fn_00450440(u8)),
        entry!(0x00450450, fn_00450450(u32)),
        entry!(0x00450460, fn_00450460(f32)),
        entry!(0x00450470, fn_00450470(u32, u32)),
        entry!(0x00450490, fn_00450490(u8)),
        entry!(0x004504a0, fn_004504a0(f32)),
        entry!(0x004504b0, fn_004504b0(f32)),
        entry!(0x004504c0, fn_004504c0(f32)),
        entry!(0x004504d0, fn_004504d0(f32)),
        entry!(0x004504e0, fn_004504e0(u32)),
        entry!(0x004504f0, fn_004504f0(u32, f32)),
        entry!(0x00450510, fn_00450510(f32)),
        entry!(0x00450520, fn_00450520(f32)),
        entry!(0x00450530, fn_00450530(f32)),
        entry!(0x00450540, fn_00450540(f32)),
        entry!(0x00450550, fn_00450550(f32)),
        entry!(0x00450560, fn_00450560(u8)),
        entry!(0x00450570, fn_00450570(Ptr) -> Ptr),
        entry!(0x004505a0, fn_004505a0(Ptr) -> Ptr),
        entry!(0x004505d0, fn_004505d0(u32)),
        entry!(0x004505e0, fn_004505e0(u32)),
        entry!(0x004505f0, fn_004505f0(Ptr) -> Ptr),
        entry!(0x00450620, bs_temp_node_manager_get_rtti(Ptr) -> Ptr),
        entry!(
            0x00450630,
            bs_temp_node_manager_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x00450670, fn_00450670(f32)),
        entry!(0x00450680, fn_00450680(f32)),
        entry!(0x00450690, fn_00450690(f32)),
        entry!(0x004506a0, fn_004506a0(u32)),
        entry!(0x004506b0, fn_004506b0()),
        entry!(0x00450770, fn_00450770(Ptr<TES>)),
        entry!(0x00450b40, fn_00450b40(Ptr, u8)),
        entry!(0x00450b60, fn_00450b60(Ptr<TES>)),
        entry!(0x00450b80, fn_00450b80(u32) -> u32),
        entry!(0x00450b90, fn_00450b90(Ptr, u32) -> Ptr),
        entry!(0x00450bc0, fn_00450bc0(Ptr, u32) -> Ptr),
        entry!(0x00450bf0, fn_00450bf0(Ptr, u32) -> Ptr),
        entry!(0x00450c20, fn_00450c20(Ptr, u32) -> Ptr),
        entry!(0x00450c50, fn_00450c50(Ptr<TES>)),
        entry!(0x00450d00, fn_00450d00(Ptr<TES>)),
        entry!(0x00450d50, fn_00450d50(Ptr, u32) -> Ptr),
        entry!(0x00450d80, fn_00450d80(Ptr<TES>)),
        entry!(0x00450dd0, tes_cull_grid_cells(Ptr<TES>, u8, u32)),
        entry!(0x00450f90, fn_00450f90(Ptr, u8)),
        entry!(0x00450fb0, fn_00450fb0(Ptr) -> bool),
        entry!(0x00450fd0, fn_00450fd0(Ptr) -> u8),
        entry!(0x00450ff0, fn_00450ff0(Ptr) -> bool),
        entry!(0x00451010, fn_00451010() -> u32),
        entry!(0x00451020, fn_00451020(Ptr<TES>, Ptr, f32)),
        entry!(0x00451110, fn_00451110(Ptr<TES>, Ptr) -> bool),
        entry!(0x004511e0, tes_is_cell_loaded(Ptr<TES>, Ptr, u8) -> bool),
        entry!(0x00451250, fn_00451250(Ptr, u8)),
        entry!(0x004512a0, fn_004512a0(Ptr, u8)),
        entry!(0x004512c0, fn_004512c0(Ptr<TES>, Ptr, u8)),
        entry!(0x00451520, fn_00451520(u8)),
        entry!(0x00451530, fn_00451530(Ptr<TES>) -> bool),
        entry!(0x00451570, fn_00451570(u32)),
        entry!(0x00451590, fn_00451590(u8)),
        entry!(0x004515a0, fn_004515a0(Ptr<TES>, Ptr, u8)),
        entry!(0x004516b0, fn_004516b0(Ptr) -> bool),
        entry!(
            0x004516d0,
            tes_find_first_grid_cell_with_water(Ptr<TES>) -> Ptr
        ),
        entry!(0x004518e0, fn_004518e0(Ptr) -> bool),
        entry!(0x00451900, fn_00451900(Ptr<TES>, i32, i32) -> u32),
        entry!(0x004519d0, fn_004519d0(Ptr<TES>, f32, f32, u32) -> u32),
        entry!(0x00451a50, tes_load_grid_cell(Ptr<TES>, u32, u32, i32, i32)),
        entry!(0x00451cb0, fn_00451cb0(Ptr) -> u32),
        entry!(0x00451cd0, fn_00451cd0(Ptr) -> u8),
        entry!(0x00451cf0, tes_update_grid_lights(Ptr<TES>)),
        entry!(0x00451da0, fn_00451da0(Ptr<TES>, Ptr)),
        entry!(0x00451e40, fn_00451e40(Ptr<TES>, Ptr) -> bool),
        entry!(0x00451ea0, fn_00451ea0(Ptr<TES>, Ptr)),
        entry!(0x00451ef0, fn_00451ef0(Ptr<TES>, Ptr, Ptr, Ptr, u8)),
        entry!(0x00452370, fn_00452370(Ptr) -> bool),
        entry!(0x00452390, fn_00452390(Ptr)),
        entry!(0x004523c0, fn_004523c0(Ptr) -> bool),
        entry!(0x004523e0, fn_004523e0(Ptr, u32) -> bool),
        entry!(0x00452420, fn_00452420(Ptr, u32) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// An engine where the three setting value-pointer functions are the
    /// game's (`this + 4`).
    fn settings_engine() -> Engine {
        let mut e = Engine::new();
        for function in [
            SETTING_VALUE_ADDRESS_INT,
            SETTING_VALUE_ADDRESS_FLOAT,
            SETTING_VALUE_ADDRESS_BYTE,
        ] {
            e.register(function, |_, a| Ret {
                eax: a[0] + 4,
                ..Ret::default()
            });
        }
        e
    }

    /// A setting object at `addr` holding `value` at +4.
    fn put_setting<T: Scalar>(e: &mut Engine, addr: u32, value: T) {
        e.map(addr, 8);
        e.set_global(addr + 4, value);
    }

    /// Makes the callees do nothing.
    fn noop(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// A setter of a one-word global.
    fn check_word_setter(function: u32, global: u32) {
        let mut e = Engine::new();
        e.map(global, 4);
        e.call(function, &args![0x1234_5678u32]);
        assert_eq!(e.global::<u32>(global), 0x1234_5678);
    }

    fn check_byte_setter(function: u32, global: u32) {
        let mut e = Engine::new();
        e.map(global, 4);
        e.call(function, &args![0x5au8]);
        assert_eq!(e.global::<u32>(global), 0x5a);
    }

    fn check_float_setter(function: u32, global: u32) {
        let mut e = Engine::new();
        e.map(global, 4);
        e.call(function, &args![1.75f32]);
        assert_eq!(e.global::<f32>(global), 1.75);
    }

    #[test]
    fn setting_uint_reads_the_value_through_the_pointer_function() {
        let mut e = settings_engine();
        put_setting(&mut e, 0x011c_3c30, 7u32);
        assert_eq!(e.call(0x0045_03f0, &args![0x011c_3c30u32]).u32(), 7);
    }

    #[test]
    fn setting_float_comes_back_in_st0() {
        let mut e = settings_engine();
        put_setting(&mut e, 0x011c_3d4c, 12.5f32);
        assert_eq!(e.call(0x0045_0410, &args![0x011c_3d4cu32]).f32(), 12.5);
    }

    #[test]
    fn flag_setter_stores_a_word() {
        check_word_setter(0x0045_0430, 0x011f_4300);
    }

    #[test]
    fn queue_warnings_setter_stores_a_byte() {
        check_byte_setter(0x0045_0440, 0x0120_2d62);
    }

    #[test]
    fn havok_update_type_setter_stores_a_word() {
        check_word_setter(0x0045_0450, 0x0126_77b8);
    }

    #[test]
    fn chase_delta_setter_stores_a_float() {
        check_float_setter(0x0045_0460, 0x011a_fe70);
    }

    #[test]
    fn callback_table_setter_stores_at_the_index() {
        let mut e = Engine::new();
        e.map(0x011a_fe88, 0x20);
        e.call(0x0045_0470, &args![0u32, 0x0057_64f0u32]);
        e.call(0x0045_0470, &args![3u32, 0x1234u32]);
        assert_eq!(e.global::<u32>(0x011a_fe88), 0x0057_64f0);
        assert_eq!(e.global::<u32>(0x011a_fe88 + 12), 0x1234);
        assert_eq!(e.global::<u32>(0x011a_fe88 + 4), 0);
    }

    #[test]
    fn add_biped_setter_stores_a_byte() {
        check_byte_setter(0x0045_0490, 0x011a_fe59);
    }

    #[test]
    fn debris_max_velocity_setter_stores_a_float() {
        check_float_setter(0x0045_04a0, 0x0126_77c0);
    }

    #[test]
    fn debris_min_extent_setter_stores_a_float() {
        check_float_setter(0x0045_04b0, 0x0126_77bc);
    }

    #[test]
    fn max_pick_time_setter_stores_a_float() {
        check_float_setter(0x0045_04c0, 0x011a_fe74);
    }

    #[test]
    fn max_pick_time_vats_setter_stores_a_float() {
        check_float_setter(0x0045_04d0, 0x011a_fe78);
    }

    #[test]
    fn entity_batch_remove_rate_setter_stores_a_word() {
        check_word_setter(0x0045_04e0, 0x011a_fe7c);
    }

    #[test]
    fn lod_table_setter_stores_a_float_at_the_index() {
        let mut e = Engine::new();
        e.map(0x011a_d7b8, 0x40);
        e.call(0x0045_04f0, &args![2u32, 0.5f32]);
        e.call(0x0045_04f0, &args![10u32, -1.0f32]);
        assert_eq!(e.global::<f32>(0x011a_d7b8 + 8), 0.5);
        assert_eq!(e.global::<f32>(0x011a_d7b8 + 40), -1.0);
        assert_eq!(e.global::<f32>(0x011a_d7b8 + 12), 0.0);
    }

    #[test]
    fn fade_in_time_setter_stores_a_float() {
        check_float_setter(0x0045_0510, 0x011a_d7e4);
    }

    #[test]
    fn fade_out_time_setter_stores_a_float() {
        check_float_setter(0x0045_0520, 0x011a_d7e8);
    }

    #[test]
    fn fade_in_threshold_setter_stores_a_float() {
        check_float_setter(0x0045_0530, 0x011a_d7ec);
    }

    #[test]
    fn fade_out_threshold_setter_stores_a_float() {
        check_float_setter(0x0045_0540, 0x011a_d7f0);
    }

    #[test]
    fn distance_multiplier_setter_stores_a_float() {
        check_float_setter(0x0045_0550, 0x011a_d7f4);
    }

    #[test]
    fn check_purged_texture_list_setter_stores_a_byte() {
        check_byte_setter(0x0045_0560, 0x011f_4460);
    }

    #[test]
    fn sky_fog_is_null_without_an_atmosphere() {
        let mut e = Engine::new();
        let sky = e.mem.alloc(0x138);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0045_0570, &args![sky]).u32(), 0);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    #[test]
    fn sky_fog_asks_the_atmosphere() {
        let mut e = Engine::new();
        let sky = e.mem.alloc(0x138);
        e.mem.set_u32(sky + 0x20, 0x0a7a_0000);
        e.register(0x0043_b230, |_, a| Ret {
            eax: a[0] + 1,
            ..Ret::default()
        });
        assert_eq!(e.call(0x0045_0570, &args![sky]).u32(), 0x0a7a_0001);
    }

    #[test]
    fn sky_light_is_null_without_a_sun() {
        let mut e = Engine::new();
        let sky = e.mem.alloc(0x138);
        assert_eq!(e.call(0x0045_05a0, &args![sky]).u32(), 0);
    }

    #[test]
    fn sky_light_asks_the_sun() {
        let mut e = Engine::new();
        let sky = e.mem.alloc(0x138);
        e.mem.set_u32(sky + 0x28, 0x5000_0000);
        e.register(0x0068_38b0, |_, a| Ret {
            eax: a[0] + 2,
            ..Ret::default()
        });
        assert_eq!(e.call(0x0045_05a0, &args![sky]).u32(), 0x5000_0002);
    }

    #[test]
    fn trans_change_callback_setter_stores_a_word() {
        check_word_setter(0x0045_05d0, 0x0126_7b68);
    }

    #[test]
    fn debug_display_callback_setter_stores_a_word() {
        check_word_setter(0x0045_05e0, 0x0126_7b64);
    }

    #[test]
    fn temp_node_manager_constructor_sets_the_vtable() {
        let mut e = Engine::new();
        e.register(0x00a5_ecb0, |e, a| {
            // The `NiNode` constructor writes its own vtable and gets 0.
            assert_eq!(a[1], 0);
            e.mem.set_u32(a[0], 0xdead_beef);
            Ret::default()
        });
        let node = e.mem.alloc(0xb0);
        assert_eq!(e.call(0x0045_05f0, &args![node]).u32(), node);
        assert_eq!(e.mem.u32(node), 0x0101_78bc);
    }

    #[test]
    fn temp_node_manager_rtti_is_a_constant() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x0045_0620, &args![0u32]).u32(), 0x0120_2df8);
    }

    #[test]
    fn temp_node_manager_deleting_destructor_frees_only_with_bit_0() {
        let mut e = Engine::new();
        noop(&mut e, &[0x00c4_4e20]);
        let freed = Rc::new(RefCell::new(vec![]));
        let log = freed.clone();
        e.register_double(0x00aa_1460, move |_, a| {
            log.borrow_mut().push((a[0], a[1]));
            Ret::default()
        });
        let node = e.mem.alloc(0xb0);
        assert_eq!(e.call(0x0045_0630, &args![node, 0u32]).u32(), node);
        assert!(freed.borrow().is_empty());
        assert_eq!(e.call(0x0045_0630, &args![node, 1u32]).u32(), node);
        assert_eq!(*freed.borrow(), vec![(node, 0xb0)]);
    }

    #[test]
    fn move_limit_mass_setter_stores_a_float() {
        check_float_setter(0x0045_0670, 0x011b_0128);
    }

    #[test]
    fn step_speed_setter_stores_a_float() {
        check_float_setter(0x0045_0680, 0x011b_0150);
    }

    #[test]
    fn physics_damage_mass_setter_stores_a_float() {
        check_float_setter(0x0045_0690, 0x0126_7bc0);
    }

    #[test]
    fn ranged_node_callback_setter_stores_a_word() {
        check_word_setter(0x0045_06a0, 0x0120_2e08);
    }

    #[test]
    fn lod_fade_multipliers_fill_the_table_from_three_settings() {
        let mut e = settings_engine();
        put_setting(&mut e, 0x011c_3d0c, 1.5f32);
        put_setting(&mut e, 0x011c_3ec0, 2.5f32);
        put_setting(&mut e, 0x011c_3d78, 3.5f32);
        e.map(0x011a_d7b8, 0x40);
        e.call(0x0045_06b0, &args![]);
        let table: Vec<f32> = (0..11).map(|i| e.global(0x011a_d7b8 + 4 * i)).collect();
        assert_eq!(
            table,
            vec![3.5, 3.5, 1.5, 2.5, 3.5, 3.5, 0.0, 0.0, 3.5, 0.0, 3.5]
        );
    }

    #[test]
    fn data_handler_flag_is_a_byte_at_61d() {
        let mut e = Engine::new();
        let handler = e.mem.alloc(0x63c);
        e.call(0x0045_0b40, &args![handler, 1u32]);
        assert_eq!(e.mem.u8(handler + 0x61d), 1);
        e.call(0x0045_0b40, &args![handler, 0u32]);
        assert_eq!(e.mem.u8(handler + 0x61d), 0);
    }

    #[test]
    fn bed_list_call_uses_the_list_at_94() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        e.register(0x0047_0470, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0045_0b60, &args![tes]);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (0x0047_0470, vec![tes.addr() + 0x94]));
    }

    #[test]
    fn shadow_scene_node_table_is_indexed_by_four() {
        let mut e = Engine::new();
        e.map(0x011f_91c8, 0x10);
        e.set_global(0x011f_91c8 + 8, 0xabcdu32);
        assert_eq!(e.call(0x0045_0b80, &args![2u32]).u32(), 0xabcd);
        assert_eq!(e.call(0x0045_0b80, &args![1u32]).u32(), 0);
    }

    /// The three scalar deleting destructors share one shape: destructor
    /// call, then `operator delete` (`00401030`) only with bit 0 set.
    fn check_deleting_destructor(function: u32, destructor: u32) {
        let mut e = Engine::new();
        noop(&mut e, &[destructor]);
        let object = e.mem.alloc(0x30);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(function, &args![object, 0u32]).u32(), object);
        let log = e.call_log.take().unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[1], (destructor, vec![object]));
        assert!(e.mem.block_size(object).is_some());

        e.call_log = Some(vec![]);
        assert_eq!(e.call(function, &args![object, 1u32]).u32(), object);
        let log = e.call_log.take().unwrap();
        assert!(log.iter().any(|(a, w)| *a == 0x0040_1030 && w[0] == object));
        assert_eq!(e.mem.block_size(object), None);
    }

    #[test]
    fn model_loader_deleting_destructor() {
        check_deleting_destructor(0x0045_0b90, 0x0044_2aa0);
    }

    #[test]
    fn parallel_task_manager_deleting_destructor() {
        check_deleting_destructor(0x0045_0bc0, 0x00c4_4b70);
    }

    #[test]
    fn save_load_deleting_destructor() {
        check_deleting_destructor(0x0045_0bf0, 0x0085_6870);
    }

    // The constructor and destructor.

    /// Everything `TES::TES` touches, with doubles for the callees that are
    /// other units' work. Values: five grids to load, 10 exterior cells,
    /// no interior cells, `threads` hardware threads.
    fn constructor_engine(threads: i32) -> Engine {
        let mut e = settings_engine();
        put_setting(&mut e, SETTING_HAVOK_UPDATE_TYPE, 2u32);
        put_setting(&mut e, SETTING_ADD_BIPED_WHEN_KEYFRAMED, 1u8);
        put_setting(&mut e, SETTING_DEBRIS_MAX_VELOCITY, 3.5f32);
        put_setting(&mut e, SETTING_DEBRIS_MIN_EXTENT, 0.25f32);
        put_setting(&mut e, SETTING_CHASE_DELTA_MULT, 1.5f32);
        put_setting(&mut e, SETTING_MAX_PICK_TIME, 0.5f32);
        put_setting(&mut e, SETTING_MAX_PICK_TIME_VATS, 0.75f32);
        put_setting(&mut e, SETTING_ENTITY_BATCH_REMOVE_RATE, 7u32);
        put_setting(&mut e, SETTING_MOVE_LIMIT_MASS, 90.0f32);
        put_setting(
            &mut e,
            SETTING_CHARACTER_CONTROLLER_MULTIPLE_STEP_SPEED,
            3.0f32,
        );
        put_setting(&mut e, SETTING_PHYSICS_DAMAGE_1_MASS, 12.0f32);
        put_setting(&mut e, SETTING_FADE_OUT_THRESHOLD, 2.0f32);
        put_setting(&mut e, SETTING_FADE_IN_THRESHOLD, 4.0f32);
        put_setting(&mut e, SETTING_FADE_IN_TIME, 0.125f32);
        put_setting(&mut e, SETTING_FADE_OUT_TIME, 0.0625f32);
        put_setting(&mut e, SETTING_DISTANCE_MULTIPLIER, 8.0f32);
        put_setting(&mut e, SETTING_QUEUE_WARNINGS, 1u8);
        put_setting(&mut e, SETTING_CHECK_PURGED_TEXTURE_LIST, 1u8);
        put_setting(&mut e, SETTING_NUM_HW_THREADS, threads);
        put_setting(&mut e, SETTING_EXTERIOR_CELL_BUFFER, 10u32);
        put_setting(&mut e, SETTING_INTERIOR_CELL_BUFFER, 0u32);
        put_setting(&mut e, SETTING_GRIDS_TO_LOAD, 5u32);
        put_setting(&mut e, SETTING_LOD_FADE_OUT_MULT_ITEMS, 1.5f32);
        put_setting(&mut e, SETTING_LOD_FADE_OUT_MULT_ACTORS, 2.5f32);
        put_setting(&mut e, SETTING_LOD_FADE_OUT_MULT_OBJECTS, 3.5f32);
        // The globals, tables and constants.
        for addr in [
            0x011f_4300,
            0x0120_2d62,
            0x0126_77b8,
            0x011a_fe58,
            0x011a_fe70,
            0x011a_fe88,
            0x0126_77bc,
            0x011a_d7b8,
            0x011a_d7e0,
            0x011f_4460,
            0x0126_7b64,
            0x011b_0128,
            0x011b_0150,
            0x0126_7bc0,
            0x0120_2e08,
            0x0120_2d98,
            0x0120_2df4,
            0x011c_3b3c,
            0x011c_3f2c,
            0x011d_e45c,
            0x011c_3c00,
            0x0101_2054,
            0x0101_7868,
            0x0101_3974,
        ] {
            e.map(addr, 0x40);
        }
        e.set_global(0x0101_2054, -1.0f32);
        e.set_global(0x0101_7868, 20.0f32);
        e.set_global(0x0101_786c, 600.0f32);
        e.set_global(0x0101_3974, 1000.0f32);
        e.set_global(0x0101_7870, 0.3f32);
        noop(
            &mut e,
            &[
                0x004e_e810,
                0x0096_a2d0,
                0x0052_8cb0,
                0x0063_3c90,
                SCOPE_GUARD_CTOR,
                SCOPE_GUARD_DTOR,
                0x0062_4bf0,
                0x0052_7eb0,
                0x0084_7480,
                0x0084_fc70,
                0x0046_24b0,
                0x0045_0d80,
                0x0066_b0d0,
                0x00a5_ecb0,
            ],
        );
        e.register(MEMSET, |e, a| {
            e.mem.write(a[0], &vec![a[1] as u8; a[2] as usize]);
            Ret::default()
        });
        for allocate in [OPERATOR_NEW, NI_OPERATOR_NEW] {
            e.register(allocate, |e, a| Ret {
                eax: e.mem.alloc(a[0]),
                ..Ret::default()
            });
        }
        // Constructors of the singletons return `this`.
        for constructor in [
            0x00c3_da50,
            0x0044_2650,
            0x00c4_4980,
            0x0045_d270,
            0x0085_6380,
            0x0062_6fd0,
        ] {
            e.register(constructor, |_, a| Ret {
                eax: a[0],
                ..Ret::default()
            });
        }
        // `GridCellArray`: its constructor writes a vtable whose second slot
        // is the reference-count increment.
        e.put_vtable(0x0300_0000, &[0x0300_1000, 0x0300_1004]);
        noop(&mut e, &[0x0300_1004]);
        e.register(0x004b_a280, |e, a| {
            e.mem.set_u32(a[0], 0x0300_0000);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        // Setting a `uint` setting writes its value.
        e.register(SETTING_SET_UINT, |e, a| {
            e.mem.set_u32(a[0] + 4, a[1]);
            Ret::default()
        });
        e
    }

    /// A root node whose vtable slot `0xdc` is `0x0300_3000`.
    fn root_node(e: &mut Engine) -> Ptr {
        e.put_vtable(0x0300_2000, &[0x0300_3000; 60]);
        noop(e, &[0x0300_3000]);
        let root = e.mem.alloc(0x100);
        e.mem.set_u32(root, 0x0300_2000);
        Ptr::new(root)
    }

    fn run_constructor(e: &mut Engine, root: Ptr, sky: Ptr) -> Ptr<TES> {
        let tes: Ptr<TES> = e.new_object();
        // A stale list node, to show the `memset`.
        e.mem.set_u32(tes.addr() + 0x8c, 0x1111);
        e.call_log = Some(vec![]);
        let back = e
            .call(
                0x0044_fb20,
                &args![
                    tes,
                    0x77u32,
                    root,
                    Ptr::<()>::new(0x0400_0010),
                    sky,
                    Ptr::<()>::new(0x0400_0020)
                ],
            )
            .ptr::<TES>();
        assert_eq!(back, tes);
        tes
    }

    #[test]
    fn constructor_zeroes_the_object_after_building_the_members() {
        let mut e = constructor_engine(4);
        let root = root_node(&mut e);
        let tes = run_constructor(&mut e, root, Ptr::NULL);
        let log = e.call_log.take().unwrap();
        let position = |addr: u32| log.iter().position(|(a, _)| *a == addr).unwrap();
        // Members are built, then the object is zeroed, then Havok starts.
        assert!(position(0x004e_e810) < position(MEMSET));
        assert!(position(0x0096_a2d0) < position(MEMSET));
        assert!(position(MEMSET) < position(0x0062_4bf0));
        assert_eq!(log[position(MEMSET)].1, vec![tes.addr(), 0, 0xc4]);
        // The vtable pointer written first is gone, as in the game.
        assert_eq!(e.mem.u32(tes.addr()), 0);
        assert_eq!(e.mem.u32(tes.addr() + 0x8c), 0);
        // The first scope guard (0xa, line 0x16a) lives to the end.
        assert_eq!(
            log[position(SCOPE_GUARD_CTOR)].1[1..],
            [0xa, 1, TES_CPP_PATH, 0x16a]
        );
        assert_eq!(log.last().unwrap().0, SCOPE_GUARD_DTOR);
    }

    #[test]
    fn constructor_copies_the_settings_into_the_globals() {
        let mut e = constructor_engine(4);
        let root = root_node(&mut e);
        run_constructor(&mut e, root, Ptr::NULL);
        assert_eq!(e.global::<u32>(0x011f_4300), 1);
        assert_eq!(e.global::<u32>(0x0126_77b8), 2);
        assert_eq!(e.global::<u32>(0x011a_fe88), 0x0057_64f0);
        assert_eq!(e.global::<u8>(0x011a_fe59), 1);
        assert_eq!(e.global::<f32>(0x0126_77c0), 3.5);
        assert_eq!(e.global::<f32>(0x0126_77bc), 0.25);
        assert_eq!(e.global::<f32>(0x011a_fe70), 1.5);
        assert_eq!(e.global::<f32>(0x011a_fe74), 0.5);
        assert_eq!(e.global::<f32>(0x011a_fe78), 0.75);
        assert_eq!(e.global::<u32>(0x011a_fe7c), 7);
        assert_eq!(e.global::<f32>(0x011b_0128), 90.0);
        // The step speed is squared.
        assert_eq!(e.global::<f32>(0x011b_0150), 9.0);
        assert_eq!(e.global::<f32>(0x0126_7bc0), 12.0);
        // LOD table: items, actors, objects, and the two constants.
        assert_eq!(e.global::<f32>(0x011a_d7b8 + 8), 1.5);
        assert_eq!(e.global::<f32>(0x011a_d7b8 + 12), 2.5);
        assert_eq!(e.global::<f32>(0x011a_d7b8), 3.5);
        assert_eq!(e.global::<f32>(0x011a_d7b8 + 24), 0.0);
        assert_eq!(e.global::<f32>(0x011a_d7b8 + 28), -1.0);
        assert_eq!(e.global::<f32>(0x011a_d7f0), 2.0);
        assert_eq!(e.global::<f32>(0x011a_d7ec), 4.0);
        assert_eq!(e.global::<f32>(0x011a_d7e4), 0.125);
        assert_eq!(e.global::<f32>(0x011a_d7e8), 0.0625);
        assert_eq!(e.global::<f32>(0x011a_d7f4), 8.0);
        assert_eq!(e.global::<u8>(0x0120_2d62), 1);
        assert_eq!(e.global::<u8>(0x011f_4460), 1);
        assert_eq!(e.global::<u32>(0x0126_7b68), 0x0057_7450);
        assert_eq!(e.global::<u32>(0x0126_7b64), 0x0057_7ca0);
        assert_eq!(e.global::<u32>(0x0120_2e08), 0x0045_c680);
    }

    #[test]
    fn constructor_creates_the_singletons() {
        let mut e = constructor_engine(4);
        let root = root_node(&mut e);
        let sizes = Rc::new(RefCell::new(vec![]));
        let seen = sizes.clone();
        e.register_double(OPERATOR_NEW, move |e, a| {
            seen.borrow_mut().push(a[0]);
            Ret {
                eax: e.mem.alloc(a[0]),
                ..Ret::default()
            }
        });
        let tes = run_constructor(&mut e, root, Ptr::NULL);
        // Sizes in order: grid cell array, IO manager, model loader, task
        // manager (four threads), data handler, save/load, then the two
        // cell buffers (one interior slot after the raise, 36 exterior).
        assert_eq!(
            *sizes.borrow(),
            vec![0x28, 0xa0, 0x30, 0x10, 0x63c, 0x1c8, 4, 144]
        );
        for singleton in [
            0x0120_2d98,
            0x011c_3b3c,
            0x0120_2df4,
            0x011c_3f2c,
            0x011d_e45c,
        ] {
            assert_ne!(e.global::<u32>(singleton), 0);
        }
        let handler = e.global::<u32>(0x011c_3f2c);
        let log = e.call_log.take().unwrap();
        let init = log.iter().find(|(a, _)| *a == 0x0046_24b0).unwrap();
        assert_eq!(init.1, vec![handler, 0x77]);
        // The grid cell array is stored and its second virtual was called.
        let cells = e.get(tes, TES::pGridCellA);
        assert_eq!(e.mem.u32(cells.addr()), 0x0300_0000);
        assert!(log
            .iter()
            .any(|(a, w)| *a == 0x0300_1004 && w[0] == cells.addr()));
    }

    #[test]
    fn constructor_skips_the_task_manager_on_one_thread() {
        let mut e = constructor_engine(1);
        let root = root_node(&mut e);
        run_constructor(&mut e, root, Ptr::NULL);
        assert_eq!(e.global::<u32>(0x0120_2df4), 0);
        assert_ne!(e.global::<u32>(0x011c_3f2c), 0);
    }

    #[test]
    fn constructor_raises_the_cell_buffer_settings() {
        let mut e = constructor_engine(4);
        let root = root_node(&mut e);
        let tes = run_constructor(&mut e, root, Ptr::NULL);
        // 5 * 5 + 5 + 5 + 1 = 36 exterior cells, and at least one interior.
        assert_eq!(e.global::<u32>(SETTING_EXTERIOR_CELL_BUFFER + 4), 36);
        assert_eq!(e.global::<u32>(SETTING_INTERIOR_CELL_BUFFER + 4), 1);
        let interior = e.get(tes, TES::pInteriorBuffer).addr();
        let exterior = e.get(tes, TES::pExteriorBuffer).addr();
        assert_eq!(e.mem.block_size(interior), Some(8));
        assert_eq!(e.mem.block_size(exterior), Some(144));
        assert_eq!(e.mem.u32(interior), 0);
        assert_eq!(e.mem.u32(exterior + 140), 0);
        assert_eq!(e.get(tes, TES::iTempInteriorBufferSize), 0);
        assert_eq!(e.get(tes, TES::iTempExteriorBufferSize), 0);
    }

    #[test]
    fn constructor_leaves_large_cell_buffers_alone() {
        let mut e = constructor_engine(4);
        put_setting(&mut e, SETTING_EXTERIOR_CELL_BUFFER, 100u32);
        put_setting(&mut e, SETTING_INTERIOR_CELL_BUFFER, 3u32);
        let root = root_node(&mut e);
        let tes = run_constructor(&mut e, root, Ptr::NULL);
        assert_eq!(e.global::<u32>(SETTING_EXTERIOR_CELL_BUFFER + 4), 100);
        assert_eq!(e.global::<u32>(SETTING_INTERIOR_CELL_BUFFER + 4), 3);
        let log = e.call_log.take().unwrap();
        assert!(log.iter().all(|(a, _)| *a != SETTING_SET_UINT));
        let exterior = e.get(tes, TES::pExteriorBuffer).addr();
        assert_eq!(e.mem.block_size(exterior), Some(400));
    }

    #[test]
    fn constructor_raises_the_grid_high_water_mark_only_upwards() {
        let mut e = constructor_engine(4);
        let root = root_node(&mut e);
        run_constructor(&mut e, root, Ptr::NULL);
        assert_eq!(e.global::<u32>(0x011c_3c00), 25);

        let mut e = constructor_engine(4);
        e.set_global(0x011c_3c00, 100u32);
        let root = root_node(&mut e);
        run_constructor(&mut e, root, Ptr::NULL);
        assert_eq!(e.global::<u32>(0x011c_3c00), 100);
    }

    #[test]
    fn constructor_stores_the_arguments_and_attaches_the_temp_node_manager() {
        let mut e = constructor_engine(4);
        let root = root_node(&mut e);
        let tes = run_constructor(&mut e, root, Ptr::NULL);
        assert_eq!(e.get(tes, TES::pObjRoot), root);
        assert_eq!(e.get(tes, TES::pObjLandRoot), Ptr::new(0x0400_0010));
        assert_eq!(e.get(tes, TES::pObjLODWaterRoot), Ptr::new(0x0400_0020));
        assert_eq!(e.get(tes, TES::pSky), Ptr::NULL);
        assert_eq!(e.get(tes, TES::pObjLight), Ptr::NULL);
        assert_eq!(e.get(tes, TES::pObjFog), Ptr::NULL);
        let manager = e.get(tes, TES::pTempNodeManager);
        assert_eq!(e.mem.u32(manager.addr()), 0x0101_78bc);
        let log = e.call_log.take().unwrap();
        // `pObjRoot->vtable[0xdc](pTempNodeManager, true)`.
        let attach = log.iter().find(|(a, _)| *a == 0x0300_3000).unwrap();
        assert_eq!(attach.1, vec![root.addr(), manager.addr(), 1]);
        assert!(e.get(tes, TES::bFadeWhenLoading));
        assert!(e.get(tes, TES::bAllowUnusedPurge));
        assert!(!e.get(tes, TES::bRunningCellTests));
        assert!(!e.get(tes, TES::bRunningCellTests2));
        assert!(!e.get(tes, TES::bShowLANDborders));
        assert!(!e.get(tes, TES::bUpdateGridString));
        // The loaded area bound is built with the four constants and stored.
        let bound = log.iter().find(|(a, _)| *a == 0x0062_6fd0).unwrap();
        assert_eq!(
            bound.1[1..],
            [
                20.0f32.to_bits(),
                600.0f32.to_bits(),
                1000.0f32.to_bits(),
                0.3f32.to_bits()
            ]
        );
        let store = log.iter().find(|(a, _)| *a == 0x0066_b0d0).unwrap();
        assert_eq!(store.1, vec![tes.addr() + 0xc0, bound.1[0]]);
    }

    #[test]
    fn constructor_without_a_root_does_not_attach() {
        let mut e = constructor_engine(4);
        let tes = run_constructor(&mut e, Ptr::NULL, Ptr::NULL);
        let log = e.call_log.take().unwrap();
        assert!(log.iter().all(|(a, _)| *a != 0x0300_3000));
        assert_ne!(e.get(tes, TES::pTempNodeManager), Ptr::NULL);
    }

    #[test]
    fn constructor_takes_light_and_fog_from_the_sky() {
        let mut e = constructor_engine(4);
        let root = root_node(&mut e);
        let sky = e.mem.alloc(0x138);
        e.mem.set_u32(sky + 0x20, 0x0500_0000);
        e.mem.set_u32(sky + 0x28, 0x0600_0000);
        e.register(0x0043_b230, |_, a| Ret {
            eax: a[0] + 1,
            ..Ret::default()
        });
        e.register(0x0068_38b0, |_, a| Ret {
            eax: a[0] + 2,
            ..Ret::default()
        });
        let tes = run_constructor(&mut e, root, Ptr::new(sky));
        assert_eq!(e.get(tes, TES::pSky), Ptr::new(sky));
        assert_eq!(e.get(tes, TES::pObjFog), Ptr::new(0x0500_0001));
        assert_eq!(e.get(tes, TES::pObjLight), Ptr::new(0x0600_0002));
    }

    /// `TES::~TES` with a double for every callee that is another unit's
    /// work.
    fn destructor_engine() -> (Engine, Ptr<TES>) {
        let mut e = settings_engine();
        for addr in [
            0x011f_4300,
            0x0120_2d98,
            0x0120_2df4,
            0x0120_2e08,
            0x011c_3b3c,
            0x011c_3f2c,
            0x011d_e45c,
            0x011d_ea3c,
            0x011f_91c8,
        ] {
            e.map(addr, 0x40);
        }
        noop(
            &mut e,
            &[
                0x0086_7840,
                0x0052_7f40,
                0x008a_8150,
                0x0097_7540,
                0x0045_39a0,
                0x0047_0470,
                0x0045_8f90,
                0x0065_1e30,
                0x0066_4cd0,
                0x0045_ac80,
                0x0045_a520,
                0x0045_a1c0,
                0x0045_a350,
                0x0068_b4a0,
                0x004d_5d50,
                0x009c_6ba0,
                0x0045_dfe0,
                0x0045_0d00,
                0x00c4_59d0,
                0x0084_7500,
                0x0084_fd30,
                0x0066_b0d0,
                0x0045_bb80,
                0x004e_8000,
                0x00b5_d180,
                0x00b6_0040,
                0x0062_5fb0,
                0x004e_e920,
                0x0045_cec0,
                0x0044_cbf0,
                0x0046_ffb0,
                0x004e_e840,
                0x0044_2aa0,
                0x00c4_4b70,
                0x0085_6870,
                0x0300_1000,
                0x0300_1100,
            ],
        );
        // Vtables: slot 0 is a deleting destructor; slot 0x1cc is called
        // with two zeros.
        e.put_vtable(0x0300_0000, &[0x0300_1000]);
        let mut slots = vec![0x0300_1000u32; 0x1d0 / 4];
        slots[0x1cc / 4] = 0x0300_1100;
        e.put_vtable(0x0300_0100, &slots);
        let tes: Ptr<TES> = e.new_object();
        (e, tes)
    }

    fn object_with_vtable(e: &mut Engine, vtable: u32) -> u32 {
        let object = e.mem.alloc(0x40);
        e.mem.set_u32(object, vtable);
        object
    }

    fn run_destructor(e: &mut Engine, tes: Ptr<TES>) -> Vec<(u32, Vec<u32>)> {
        e.call_log = Some(vec![]);
        e.call(0x0045_0770, &args![tes]);
        e.call_log.take().unwrap()
    }

    #[test]
    fn destructor_with_nothing_loaded_clears_state_and_calls_the_cleanups() {
        let (mut e, tes) = destructor_engine();
        let handler = e.mem.alloc(0x63c);
        e.set_global(0x011c_3f2c, handler);
        e.set_global(0x0120_2e08, 0x1234u32);
        let object = object_with_vtable(&mut e, 0x0300_0100);
        e.set_global(0x011d_ea3c, object);
        // The data handler is deleted by `00450c20` (a double here).
        let deleted = Rc::new(RefCell::new(vec![]));
        let log = deleted.clone();
        e.register_double(0x0045_0c20, move |_, a| {
            log.borrow_mut().push((a[0], a[1]));
            Ret::default()
        });
        let calls = run_destructor(&mut e, tes);
        // The vtable is restored and the callback cleared.
        assert_eq!(e.mem.u32(tes.addr()), 0x0101_78b4);
        assert_eq!(e.global::<u32>(0x0120_2e08), 0);
        // `bClearingData` was set while the data was cleared, and the
        // handler is deleted through `00450c20`.
        assert_eq!(e.mem.u8(handler + 0x61d), 1);
        assert_eq!(*deleted.borrow(), vec![(handler, 1)]);
        assert_eq!(e.global::<u32>(0x011c_3f2c), 0);
        // Slot 0x1cc of the object at 011dea3c is called with two zeros.
        let slot = calls.iter().find(|(a, _)| *a == 0x0300_1100).unwrap();
        assert_eq!(slot.1, vec![object, 0, 0]);
        // Order of the main steps.
        let position = |addr: u32| calls.iter().position(|(a, _)| *a == addr).unwrap();
        assert!(position(0x0086_7840) < position(0x0052_7f40));
        assert!(position(0x0045_39a0) < position(0x0045_8f90));
        assert!(position(0x0045_dfe0) < position(0x0045_0d00));
        assert!(position(0x0084_7500) < position(0x0066_b0d0));
        assert!(position(0x0066_b0d0) < position(0x0045_0c20));
        assert!(position(0x0045_0c20) < position(0x004e_8000));
        assert!(position(0x004e_e920) < position(0x004e_e840));
        // The member destructors run last, in reverse order of the fields.
        let tail: Vec<(u32, u32)> = calls[position(0x004e_e920)..]
            .iter()
            .map(|(a, w)| (*a, w[0] - tes.addr()))
            .collect();
        assert_eq!(
            tail,
            vec![
                (0x004e_e920, 0x6c),
                (0x0045_cec0, 0xc0),
                (0x0044_cbf0, 0xac),
                (0x0045_cec0, 0xa8),
                (0x0044_cbf0, 0xa4),
                (0x0046_ffb0, 0x9c),
                (0x0046_ffb0, 0x94),
                (0x0046_ffb0, 0x8c),
                (0x004e_e840, 0x6c),
            ]
        );
        // Without a ModelLoader its destructor is not called.
        assert!(calls.iter().all(|(a, _)| *a != 0x0044_2aa0));
    }

    #[test]
    fn destructor_deletes_the_singletons_and_clears_their_globals() {
        let (mut e, tes) = destructor_engine();
        noop(&mut e, &[0x0045_0c20]);
        let handler = e.mem.alloc(0x63c);
        e.set_global(0x011c_3f2c, handler);
        let object = object_with_vtable(&mut e, 0x0300_0100);
        e.set_global(0x011d_ea3c, object);
        let model_loader = e.mem.alloc(0x30);
        e.set_global(0x011c_3b3c, model_loader);
        let io_manager = object_with_vtable(&mut e, 0x0300_0000);
        e.set_global(0x0120_2d98, io_manager);
        let task_manager = e.mem.alloc(0x10);
        e.set_global(0x0120_2df4, task_manager);
        let save_load = e.mem.alloc(0x1c8);
        e.set_global(0x011d_e45c, save_load);
        let grid_cells = object_with_vtable(&mut e, 0x0300_0000);
        e.set(tes, TES::pGridCellA, Ptr::new(grid_cells));
        let interior = e.mem.alloc(8);
        let exterior = e.mem.alloc(8);
        e.set(tes, TES::pInteriorBuffer, Ptr::new(interior));
        e.set(tes, TES::pExteriorBuffer, Ptr::new(exterior));

        let calls = run_destructor(&mut e, tes);
        for global in [0x011c_3b3c, 0x0120_2d98, 0x0120_2df4, 0x011d_e45c] {
            assert_eq!(e.global::<u32>(global), 0);
        }
        // Blocks freed with `operator delete`.
        assert_eq!(e.mem.block_size(model_loader), None);
        assert_eq!(e.mem.block_size(task_manager), None);
        assert_eq!(e.mem.block_size(save_load), None);
        assert_eq!(e.mem.block_size(interior), None);
        assert_eq!(e.mem.block_size(exterior), None);
        // The IO manager and grid cell array are deleted through slot 0 with
        // `1`; the pointer in `pGridCellA` is left as it was.
        let deleting: Vec<&Vec<u32>> = calls
            .iter()
            .filter(|(a, _)| *a == 0x0300_1000)
            .map(|(_, w)| w)
            .collect();
        assert_eq!(*deleting[0], vec![io_manager, 1]);
        assert_eq!(*deleting[1], vec![grid_cells, 1]);
        assert_eq!(e.get(tes, TES::pGridCellA), Ptr::new(grid_cells));
        // The destructors of the singletons were called.
        for destructor in [0x0044_2aa0, 0x00c4_4b70, 0x0085_6870] {
            assert!(calls.iter().any(|(a, _)| *a == destructor));
        }
    }

    #[test]
    fn destructor_cleans_up_each_shadow_scene_node_present() {
        let (mut e, tes) = destructor_engine();
        noop(&mut e, &[0x0045_0c20]);
        let object = object_with_vtable(&mut e, 0x0300_0100);
        e.set_global(0x011d_ea3c, object);
        let handler = e.mem.alloc(0x63c);
        e.set_global(0x011c_3f2c, handler);
        // Entries 1 and 3 are set.
        e.set_global(0x011f_91c8 + 4, 0x7001u32);
        e.set_global(0x011f_91c8 + 12, 0x7003u32);
        let calls = run_destructor(&mut e, tes);
        let nodes: Vec<u32> = calls
            .iter()
            .filter(|(a, _)| *a == 0x00b5_d180)
            .map(|(_, w)| w[0])
            .collect();
        assert_eq!(nodes, vec![0x7001, 0x7003]);
        let static_cleanups = calls.iter().filter(|(a, _)| *a == 0x00b6_0040).count();
        assert_eq!(static_cleanups, 2);
    }

    // Session 2: the water system, the grid of loaded cells and loading
    // references.

    /// A cell object (0xC0 bytes) with the given load state (+0x26) and flags
    /// byte (+0x24).
    fn new_cell(e: &mut Engine, state: u8, flags: u8) -> u32 {
        let cell = e.mem.alloc(0xc0);
        e.mem.set_u8(cell + 0x26, state);
        e.mem.set_u8(cell + 0x24, flags);
        cell
    }

    /// Makes `addr` return `value` in EAX.
    fn returns(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| Ret {
            eax: value,
            ..Ret::default()
        });
    }

    /// Makes `addr` return `value` in ST0.
    fn returns_float(e: &mut Engine, addr: u32, value: f64) {
        e.register_double(addr, move |_, _| Ret {
            st0: value,
            ..Ret::default()
        });
    }

    /// `_ftol2_sse` on its `f64` argument: truncation.
    fn install_ftol(e: &mut Engine) {
        e.register(FTOL, |_, a| {
            let value = f64::from_bits(a[0] as u64 | (a[1] as u64) << 32);
            Ret {
                eax: value as i32 as u32,
                ..Ret::default()
            }
        });
    }

    /// The `fistp` helper: the float rounded to the nearest integer.
    fn install_float_to_int(e: &mut Engine) {
        e.register(FLOAT_TO_INT_ROUNDED, |_, a| Ret {
            eax: f32::from_bits(a[0]).round() as i32 as u32,
            ..Ret::default()
        });
    }

    /// `TES::GetWorldSpace` (`this + 0x88`) and the `NiPointer` getter
    /// (`00559450`, the first word of `this`).
    fn install_getters(e: &mut Engine) {
        e.register(GET_WORLD_SPACE, |e, a| Ret {
            eax: e.mem.u32(a[0] + 0x88),
            ..Ret::default()
        });
        e.register(0x0055_9450, |e, a| Ret {
            eax: e.mem.u32(a[0]),
            ..Ret::default()
        });
    }

    /// A world manager with a `grids` x `grids` table of cell slots behind a
    /// `GridCellArray::Get` double (slot `(x, y)` is at
    /// `table + (x * grids + y) * 4`), `uGridsToLoad` set to `grids` and a
    /// non-null data handler. Returns the engine, the `TES` and the table.
    fn grid_engine(grids: u32) -> (Engine, Ptr<TES>, u32) {
        let mut e = settings_engine();
        put_setting(&mut e, SETTING_GRIDS_TO_LOAD, grids);
        install_getters(&mut e);
        e.map(DATA_HANDLER, 4);
        e.set_global(DATA_HANDLER, 0x1234u32);
        let tes: Ptr<TES> = e.new_object();
        let array = e.mem.alloc(0x28);
        e.set(tes, TES::pGridCellA, Ptr::new(array));
        let table = e.mem.alloc(grids * grids * 4);
        e.register_double(GRID_CELL_ARRAY_GET, move |_, a| Ret {
            eax: table + (a[1] * grids + a[2]) * 4,
            ..Ret::default()
        });
        (e, tes, table)
    }

    fn put_cell(e: &mut Engine, table: u32, grids: u32, x: u32, y: u32, cell: u32) {
        e.mem.set_u32(table + (x * grids + y) * 4, cell);
    }

    /// Runs `f` with the call log on and returns the log.
    fn run_logged(e: &mut Engine, f: impl FnOnce(&mut Engine)) -> Vec<(u32, Vec<u32>)> {
        e.call_log = Some(vec![]);
        f(e);
        e.call_log.take().unwrap()
    }

    /// The argument lists of the calls to `addr` in `log`.
    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    }

    /// The callee addresses of `log`, in order.
    fn addresses(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        log.iter().map(|(a, _)| *a).collect()
    }

    /// An object whose vtable (at `vtable`) has the `(byte offset, function)`
    /// entries filled in.
    fn object_with_slots(e: &mut Engine, vtable: u32, entries: &[(u32, u32)]) -> u32 {
        e.map(vtable, 0x400);
        for (slot, function) in entries {
            e.mem.set_u32(vtable + slot, *function);
        }
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(object, vtable);
        object
    }

    #[test]
    fn data_handler_deleting_destructor() {
        check_deleting_destructor(0x0045_0c20, 0x0045_d970);
    }

    #[test]
    fn water_system_is_built_under_the_memory_tag_and_stored() {
        let mut e = Engine::new();
        noop(&mut e, &[SCOPE_GUARD_CTOR, SCOPE_GUARD_DTOR]);
        e.register(OPERATOR_NEW, |e, a| Ret {
            eax: e.mem.alloc(a[0]),
            ..Ret::default()
        });
        e.register(0x004e_1650, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        let tes: Ptr<TES> = e.new_object();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0c50, &args![tes]);
        });
        let water = e.get(tes, TES::pWaterSystem);
        assert_eq!(e.mem.block_size(water.addr()), Some(0xa0));
        let guard = calls_to(&log, SCOPE_GUARD_CTOR);
        assert_eq!(guard[0][1..], [0x1d, 1, TES_CPP_PATH, 0x286]);
        assert_eq!(calls_to(&log, 0x004e_1650), vec![vec![water.addr()]]);
        // The guard is destroyed last, with the block the constructor got.
        let (last, words) = log.last().unwrap();
        assert_eq!(*last, SCOPE_GUARD_DTOR);
        assert_eq!(words[0], guard[0][0]);
    }

    #[test]
    fn water_system_with_a_failed_allocation_stores_null() {
        let mut e = Engine::new();
        noop(&mut e, &[SCOPE_GUARD_CTOR, SCOPE_GUARD_DTOR]);
        returns(&mut e, OPERATOR_NEW, 0);
        let tes: Ptr<TES> = e.new_object();
        e.set(tes, TES::pWaterSystem, Ptr::new(0x1234));
        // `004e1650` is not registered: calling it would panic.
        e.call(0x0045_0c50, &args![tes]);
        assert_eq!(e.get(tes, TES::pWaterSystem), Ptr::NULL);
    }

    #[test]
    fn water_system_deletion_deletes_and_clears_the_field() {
        let mut e = Engine::new();
        noop(&mut e, &[0x004e_19c0]);
        let tes: Ptr<TES> = e.new_object();
        let water = e.mem.alloc(0xa0);
        e.set(tes, TES::pWaterSystem, Ptr::new(water));
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0d00, &args![tes]);
        });
        assert_eq!(calls_to(&log, 0x004e_19c0), vec![vec![water]]);
        assert_eq!(e.mem.block_size(water), None);
        assert_eq!(e.get(tes, TES::pWaterSystem), Ptr::NULL);
        // Nothing to delete the second time.
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0d00, &args![tes]);
        });
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn water_system_deleting_destructor() {
        check_deleting_destructor(0x0045_0d50, 0x004e_19c0);
    }

    #[test]
    fn grid_positions_are_reset_to_the_no_cell_value() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        e.mem.set_u32(tes.addr() + 0x34, 0x1234);
        e.mem.set_u32(tes.addr() + 0x24, 5);
        e.call(0x0045_0d80, &args![tes]);
        for offset in [0x24, 0x28, 0x2c, 0x30, 0x48, 0x4c] {
            assert_eq!(e.mem.u32(tes.addr() + offset), 0x7fff_ffff, "+{offset:#x}");
        }
        assert_eq!(e.mem.u32(tes.addr() + 0x34), 0x1234);
    }

    /// An engine for `tes_cull_grid_cells`: a 2 x 2 grid, a Havok world, and
    /// doubles for every callee. Cells: (0,0) state 3, (0,1) state 6, (1,0)
    /// none, (1,1) state 3.
    fn cull_engine() -> (Engine, Ptr<TES>, [u32; 3]) {
        let (mut e, tes, table) = grid_engine(2);
        e.map(EXTERIOR_WORLD, 4);
        e.set_global(EXTERIOR_WORLD, 0x5000u32);
        noop(
            &mut e,
            &[
                0x00c6_6300,
                0x00c6_6310,
                0x00c6_8f00,
                0x00c6_b540,
                0x0097_5f90,
                0x0043_b370,
                0x006f_d1f0,
                0x0043_d410,
                0x00a5_9c60,
                0x0055_2bd0,
                0x0054_bcf0,
                GRID_CELL_ARRAY_ATTACH_TO_WORLD,
            ],
        );
        returns(&mut e, 0x0096_11e0, 0x6000);
        e.set(tes, TES::pObjRoot, Ptr::new(0x7000));
        e.set(tes, TES::pObjLandRoot, Ptr::new(0x7100));
        let cells = [
            new_cell(&mut e, 3, 0),
            new_cell(&mut e, 6, 0),
            new_cell(&mut e, 3, 0),
        ];
        put_cell(&mut e, table, 2, 0, 0, cells[0]);
        put_cell(&mut e, table, 2, 0, 1, cells[1]);
        put_cell(&mut e, table, 2, 1, 1, cells[2]);
        (e, tes, cells)
    }

    #[test]
    fn culling_attaches_the_cells_in_state_3() {
        let (mut e, tes, cells) = cull_engine();
        let array = e.get(tes, TES::pGridCellA).addr();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0dd0, &args![tes, 0u8, 0u32]);
        });
        assert_eq!(
            calls_to(&log, GRID_CELL_ARRAY_ATTACH_TO_WORLD),
            vec![vec![array, 0x7000, 0, 0], vec![array, 0x7000, 1, 1]]
        );
        assert_eq!(
            calls_to(&log, 0x0054_bcf0),
            vec![vec![cells[0], 0], vec![cells[2], 0]]
        );
        assert!(calls_to(&log, 0x0055_2bd0).is_empty());
        // The Havok world first and last, the shadow flag, the terrain switch.
        let at = |address: u32| addresses(&log).iter().position(|a| *a == address).unwrap();
        assert_eq!(calls_to(&log, 0x00c6_6300), vec![vec![0x5000]]);
        assert!(at(0x00c6_6300) < at(0x0054_bcf0));
        assert!(at(0x0054_bcf0) < at(0x00c6_b540));
        assert_eq!(calls_to(&log, 0x00c6_b540), vec![vec![0x5000, 0]]);
        assert_eq!(calls_to(&log, 0x0097_5f90), vec![vec![OBJECT_011E0E80]]);
        assert_eq!(calls_to(&log, 0x0043_b370), vec![vec![0x6000, 0, 1]]);
        assert_eq!(calls_to(&log, 0x006f_d1f0), vec![vec![1]]);
        // The land root is updated with a freshly built 12-byte object.
        let update = calls_to(&log, 0x00a5_9c60);
        assert_eq!(update[0][0], 0x7100);
        let built = calls_to(&log, 0x0043_d410);
        assert_eq!(built[0], vec![update[0][1], 0, 0, 0]);
    }

    #[test]
    fn culling_with_the_detach_flag_detaches_the_cells_in_state_6() {
        let (mut e, tes, cells) = cull_engine();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0dd0, &args![tes, 1u8, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x0055_2bd0), vec![vec![cells[1], 0]]);
        assert!(calls_to(&log, GRID_CELL_ARRAY_ATTACH_TO_WORLD).is_empty());
        assert_eq!(calls_to(&log, 0x00c6_6310), vec![vec![0x5000]]);
        assert!(calls_to(&log, 0x00c6_6300).is_empty());
        assert_eq!(calls_to(&log, 0x00c6_8f00), vec![vec![0x5000, 0]]);
        assert!(calls_to(&log, 0x0097_5f90).is_empty());
        assert_eq!(calls_to(&log, 0x0043_b370), vec![vec![0x6000, 1, 1]]);
        assert_eq!(calls_to(&log, 0x006f_d1f0), vec![vec![0]]);
    }

    #[test]
    fn culling_without_a_data_handler_does_nothing() {
        let (mut e, tes, _) = cull_engine();
        e.set_global(DATA_HANDLER, 0u32);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0dd0, &args![tes, 0u8, 0u32]);
        });
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn culling_without_a_havok_world_skips_the_world_calls() {
        let (mut e, tes, _) = cull_engine();
        e.set_global(EXTERIOR_WORLD, 0u32);
        e.set(tes, TES::pObjLandRoot, Ptr::NULL);
        returns(&mut e, 0x0096_11e0, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0dd0, &args![tes, 0u8, 0u32]);
        });
        for world_call in [0x00c6_6300, 0x00c6_b540, 0x0043_b370, 0x006f_d1f0] {
            assert!(calls_to(&log, world_call).is_empty(), "{world_call:08x}");
        }
        // The attaching still happens.
        assert_eq!(calls_to(&log, 0x0054_bcf0).len(), 2);
    }

    #[test]
    fn node_flag_setter_passes_the_flag_and_the_mask() {
        let mut e = Engine::new();
        noop(&mut e, &[0x0043_b370]);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_0f90, &args![0x6000u32, 1u8]);
        });
        assert_eq!(calls_to(&log, 0x0043_b370), vec![vec![0x6000, 1, 1]]);
    }

    #[test]
    fn cell_state_tests_compare_the_load_state() {
        let mut e = Engine::new();
        for (state, is_3, is_5, is_6) in [
            (0u8, false, false, false),
            (3, true, false, false),
            (5, false, true, false),
            (6, false, false, true),
        ] {
            let cell = new_cell(&mut e, state, 0);
            assert_eq!(e.call(0x0045_0fd0, &args![cell]).u32(), state as u32);
            assert_eq!(e.call(0x0045_0fb0, &args![cell]).bool(), is_3);
            assert_eq!(e.call(0x0045_23c0, &args![cell]).bool(), is_5);
            assert_eq!(e.call(0x0045_0ff0, &args![cell]).bool(), is_6);
        }
    }

    #[test]
    fn exterior_world_is_the_first_word_of_the_ni_pointer() {
        let mut e = Engine::new();
        install_getters(&mut e);
        e.map(EXTERIOR_WORLD, 4);
        e.set_global(EXTERIOR_WORLD, 0x5555u32);
        assert_eq!(e.call(0x0045_1010, &args![]).u32(), 0x5555);
    }

    #[test]
    fn loaded_grid_rectangle_is_cell_corners_inset_by_the_margin() {
        let (mut e, tes, _) = grid_engine(5);
        install_ftol(&mut e);
        e.set(tes, TES::iCurrentGridX, 10);
        e.set(tes, TES::iCurrentGridY, -3);
        let out = e.mem.alloc(16);
        e.call(0x0045_1020, &args![tes, out, 100.5f32]);
        // Cells 8..=12 in x and -5..=-1 in y; each cell is 0x1000 wide. The
        // near corner gets the margin added, the far corner (one cell
        // further) has it subtracted; `_ftol2` truncates towards zero.
        assert_eq!(e.mem.i32(out), 8 * 0x1000 + 100);
        // -20480 + 100.5 = -20379.5, truncated to -20379.
        assert_eq!(e.mem.i32(out + 12), -5 * 0x1000 + 101);
        // 53248 - 100.5 = 53147.5.
        assert_eq!(e.mem.i32(out + 8), 13 * 0x1000 - 101);
        // 0 - 100.5 = -100.5.
        assert_eq!(e.mem.i32(out + 4), -100);
    }

    /// Cell coordinates 10, 20 at the centre of a 5 x 5 grid: cells 8..13 and
    /// 18..23 are loaded.
    fn position_engine() -> (Engine, Ptr<TES>) {
        let (mut e, tes, _) = grid_engine(5);
        install_float_to_int(&mut e);
        returns(&mut e, GET_INTERIOR_CELL, 0);
        e.set(tes, TES::iCurrentGridX, 10);
        e.set(tes, TES::iCurrentGridY, 20);
        (e, tes)
    }

    fn position(e: &mut Engine, x: f32, y: f32) -> u32 {
        let pos = e.mem.alloc(12);
        e.mem.set_f32(pos, x);
        e.mem.set_f32(pos + 4, y);
        pos
    }

    #[test]
    fn position_test_accepts_cells_inside_the_loaded_square() {
        let (mut e, tes) = position_engine();
        let inside = position(&mut e, 8.0 * 4096.0, 22.0 * 4096.0 + 4095.0);
        assert!(e.call(0x0045_1110, &args![tes, inside]).bool());
        let inside = position(&mut e, 12.0 * 4096.0 + 2000.0, 18.0 * 4096.0);
        assert!(e.call(0x0045_1110, &args![tes, inside]).bool());
    }

    #[test]
    fn position_test_rejects_cells_outside_the_loaded_square() {
        let (mut e, tes) = position_engine();
        for (x, y) in [
            (7.0 * 4096.0 + 4000.0, 20.0 * 4096.0),
            (13.0 * 4096.0, 20.0 * 4096.0),
            (10.0 * 4096.0, 17.0 * 4096.0 + 100.0),
            (10.0 * 4096.0, 23.0 * 4096.0),
        ] {
            let pos = position(&mut e, x, y);
            assert!(!e.call(0x0045_1110, &args![tes, pos]).bool(), "{x} {y}");
        }
    }

    #[test]
    fn position_test_is_true_while_an_interior_cell_is_loaded() {
        let (mut e, tes) = position_engine();
        returns(&mut e, GET_INTERIOR_CELL, 0x4444);
        let far = position(&mut e, 1.0e9, -1.0e9);
        assert!(e.call(0x0045_1110, &args![tes, far]).bool());
    }

    #[test]
    fn is_cell_loaded_by_load_state() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        let check = |e: &mut Engine, cell: u32, flag: u8| {
            e.call(0x0045_11e0, &args![tes, cell, flag]).bool()
        };
        assert!(!check(&mut e, 0, 0));
        for (state, flag_0, flag_1) in [
            (0u8, false, false),
            (1, false, false),
            (2, true, false),
            (3, true, false),
            (4, true, false),
            (5, true, true),
            (6, true, true),
            (7, false, false),
            (255, false, false),
        ] {
            let cell = new_cell(&mut e, state, 0);
            assert_eq!(check(&mut e, cell, 0), flag_0, "state {state} flag 0");
            assert_eq!(check(&mut e, cell, 1), flag_1, "state {state} flag 1");
        }
    }

    #[test]
    fn water_bit_setter_changes_only_bit_1() {
        let mut e = Engine::new();
        let cell = new_cell(&mut e, 0, 0b1010_0101);
        e.call(0x0045_1250, &args![cell, 1u8]);
        assert_eq!(e.mem.u8(cell + 0x24), 0b1010_0111);
        assert!(e.call(0x0045_18e0, &args![cell]).bool());
        e.call(0x0045_1250, &args![cell, 0u8]);
        assert_eq!(e.mem.u8(cell + 0x24), 0b1010_0101);
        assert!(!e.call(0x0045_18e0, &args![cell]).bool());
    }

    #[test]
    fn cell_state_setter_stores_the_byte() {
        let mut e = Engine::new();
        let cell = new_cell(&mut e, 1, 0xff);
        e.call(0x0045_12a0, &args![cell, 6u8]);
        assert_eq!(e.mem.u8(cell + 0x26), 6);
        assert_eq!(e.mem.u8(cell + 0x24), 0xff);
    }

    /// An engine for `fn_004512c0`: a 5 x 5 grid centred on (10, 20), a
    /// `GridCellArray` whose vtable has slots 8 and 0x10, a player whose slot
    /// 0x1f4 returns a position, and doubles for every callee.
    fn load_engine() -> (Engine, Ptr<TES>) {
        let (mut e, tes, _) = grid_engine(5);
        for (addr, len) in [
            (GRID_LOAD_IN_PROGRESS, 1),
            (GRID_FLAG_01189184, 1),
            (GRID_FLAG_011AD86C, 1),
            (POINT_011A9478, 12),
            (IO_MANAGER, 4),
            (PLAYER_CHARACTER, 4),
            (FADER_MANAGER, 4),
        ] {
            e.map(addr, len);
        }
        e.set_global(IO_MANAGER, 0x5a5au32);
        e.set_global(FADER_MANAGER, 0x6b6bu32);
        for (i, value) in [11.0f32, 12.0, 13.0].into_iter().enumerate() {
            e.set_global(POINT_011A9478 + 4 * i as u32, value);
        }
        put_setting(&mut e, SETTING_FADE_TO_BLACK_FADE_SECONDS, 0.3f32);
        let array = e.get(tes, TES::pGridCellA).addr();
        e.put_vtable(0x0300_1000, &[0, 0, 0x0300_0008, 0, 0x0300_0010]);
        e.mem.set_u32(array, 0x0300_1000);
        let player = object_with_slots(&mut e, 0x0300_2000, &[(0x1f4, 0x0300_01f4)]);
        e.set_global(PLAYER_CHARACTER, player);
        returns(&mut e, 0x0300_01f4, 0x8888);
        noop(&mut e, &[0x0300_0008, 0x0300_0010]);
        returns(&mut e, GET_INTERIOR_CELL, 0);
        returns(&mut e, GET_TERRAIN_MANAGER, 0x7000);
        noop(
            &mut e,
            &[
                0x006f_ce00,
                0x006f_ca90,
                0x0045_5200,
                0x0045_2490,
                0x0045_7be0,
                0x0045_2ff0,
                0x00c3_dfa0,
                0x0057_d0a0,
                0x0070_0960,
                0x0045_9870,
                0x0097_2d30,
                0x0097_2bb0,
                0x0045_5490,
                0x0043_d410,
                0x00a5_9c60,
                0x0046_0360,
                0x0066_b0d0,
            ],
        );
        // `00416870` stores its three floats.
        e.register(0x0041_6870, |e, a| {
            for i in 0..3 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            Ret::default()
        });
        // `004572e0` stores a float in the third word of its output.
        e.register(0x0045_72e0, |e, a| {
            e.mem.set_f32(a[2], 9.0);
            Ret::default()
        });
        e.set(tes, TES::iCurrentGridX, 10);
        e.set(tes, TES::iCurrentGridY, 20);
        e.set(tes, TES::pWorldSpace, Ptr::new(0x6000));
        (e, tes)
    }

    #[test]
    fn full_grid_reload_runs_the_steps_in_order() {
        let (mut e, tes) = load_engine();
        let pos = position(&mut e, 1.5, 2.5);
        let array = e.get(tes, TES::pGridCellA).addr();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_12c0, &args![tes, pos, 1u8]);
        });
        assert_eq!(
            addresses(&log),
            vec![
                0x0045_12c0,
                GET_INTERIOR_CELL,
                0x0300_0008,
                0x0066_b0d0,
                GET_TERRAIN_MANAGER,
                0x006f_ce00,
                0x0300_01f4,
                GET_TERRAIN_MANAGER,
                0x006f_ca90,
                0x0300_0010,
                0x0045_5200,
                0x0045_2490,
                0x0045_7be0,
                0x0045_2ff0,
                0x00c3_dfa0,
                0x0041_6870,
                0x0045_72e0,
                0x0057_d0a0,
                SETTING_VALUE_ADDRESS_FLOAT,
                0x0070_0960,
                0x0045_9870,
                0x0097_2d30,
                0x0097_2bb0,
                0x0045_5490,
                0x0043_d410,
                0x00a5_9c60,
                0x0045_2490,
                0x0046_0360,
            ]
        );
        assert_eq!(calls_to(&log, 0x0300_0008), vec![vec![array]]);
        assert_eq!(calls_to(&log, 0x0066_b0d0), vec![vec![0x011f_95e8, 0]]);
        assert_eq!(calls_to(&log, 0x006f_ce00), vec![vec![0x7000]]);
        // The terrain is updated with the player's position and 0xf.
        assert_eq!(calls_to(&log, 0x006f_ca90), vec![vec![0x7000, 0x8888, 0xf]]);
        // The grid is repositioned at the centre cell.
        assert_eq!(calls_to(&log, 0x0300_0010), vec![vec![array, 10, 20]]);
        assert_eq!(calls_to(&log, 0x0045_5200), vec![vec![tes.addr(), 0, 0]]);
        assert_eq!(
            calls_to(&log, 0x0045_2490),
            vec![vec![tes.addr(), 0], vec![tes.addr(), 0]]
        );
        assert_eq!(calls_to(&log, 0x0045_7be0), vec![vec![tes.addr(), 0]]);
        assert_eq!(calls_to(&log, 0x00c3_dfa0), vec![vec![0x5a5a, 4]]);
        // The point is (x, y, 0), whose third float `004572e0` then replaces.
        let point = calls_to(&log, 0x0041_6870);
        assert_eq!(point[0][1..], [1.5f32.to_bits(), 2.5f32.to_bits(), 0]);
        assert_eq!(
            calls_to(&log, 0x0045_72e0),
            vec![vec![tes.addr(), pos, point[0][0] + 8]]
        );
        assert_eq!(
            calls_to(&log, 0x0057_d0a0),
            vec![vec![
                1.5f32.to_bits(),
                2.5f32.to_bits(),
                9.0f32.to_bits(),
                11.0f32.to_bits(),
                12.0f32.to_bits(),
                13.0f32.to_bits(),
                1.0f32.to_bits()
            ]]
        );
        // The fader: one second argument 1, the setting, and the stray 0.
        assert_eq!(
            calls_to(&log, 0x0070_0960),
            vec![vec![0x6b6b, 1, 0.3f32.to_bits(), 0]]
        );
        assert_eq!(e.global::<u8>(GRID_FLAG_01189184), 1);
        assert_eq!(e.global::<u8>(GRID_FLAG_011AD86C), 1);
        assert_eq!(e.global::<u8>(GRID_LOAD_IN_PROGRESS), 0);
    }

    #[test]
    fn grid_reload_does_not_fade_while_cell_tests_run() {
        let (mut e, tes) = load_engine();
        e.set(tes, TES::bRunningCellTests2, true);
        let pos = position(&mut e, 1.5, 2.5);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_12c0, &args![tes, pos, 1u8]);
        });
        assert!(calls_to(&log, 0x0070_0960).is_empty());
        assert_eq!(calls_to(&log, 0x0045_9870).len(), 1);
    }

    #[test]
    fn partial_grid_load_repositions_and_loads() {
        let (mut e, tes) = load_engine();
        let pos = position(&mut e, 1.5, 2.5);
        let array = e.get(tes, TES::pGridCellA).addr();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_12c0, &args![tes, pos, 0u8]);
        });
        assert_eq!(
            addresses(&log),
            vec![
                0x0045_12c0,
                GET_INTERIOR_CELL,
                0x0300_0010,
                0x0045_7be0,
                0x0045_2ff0,
                0x0045_5490,
                0x0043_d410,
                0x00a5_9c60,
                0x0045_2490,
                0x0046_0360,
            ]
        );
        assert_eq!(calls_to(&log, 0x0300_0010), vec![vec![array, 10, 20]]);
        assert_eq!(calls_to(&log, 0x0045_7be0), vec![vec![tes.addr(), 1]]);
        assert_eq!(e.global::<u8>(GRID_LOAD_IN_PROGRESS), 0);
    }

    #[test]
    fn grid_load_is_skipped_without_a_data_handler_with_an_interior_or_when_running() {
        let (mut e, tes) = load_engine();
        let pos = position(&mut e, 1.5, 2.5);
        e.set_global(DATA_HANDLER, 0u32);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_12c0, &args![tes, pos, 1u8]);
        });
        assert_eq!(log.len(), 1);

        e.set_global(DATA_HANDLER, 0x1234u32);
        returns(&mut e, GET_INTERIOR_CELL, 0x4444);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_12c0, &args![tes, pos, 1u8]);
        });
        assert_eq!(addresses(&log), vec![0x0045_12c0, GET_INTERIOR_CELL]);

        returns(&mut e, GET_INTERIOR_CELL, 0);
        e.set_global(GRID_LOAD_IN_PROGRESS, 1u8);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_12c0, &args![tes, pos, 1u8]);
        });
        assert_eq!(addresses(&log), vec![0x0045_12c0, GET_INTERIOR_CELL]);
        // The guard byte is left as it was.
        assert_eq!(e.global::<u8>(GRID_LOAD_IN_PROGRESS), 1);
    }

    #[test]
    fn grid_flag_and_helper_functions() {
        let mut e = Engine::new();
        e.map(GRID_FLAG_01189184, 1);
        e.map(GRID_FLAG_011AD86C, 1);
        e.call(0x0045_1520, &args![1u8]);
        e.call(0x0045_1590, &args![1u8]);
        assert_eq!(e.global::<u8>(GRID_FLAG_01189184), 1);
        assert_eq!(e.global::<u8>(GRID_FLAG_011AD86C), 1);
        e.call(0x0045_1520, &args![0u8]);
        assert_eq!(e.global::<u8>(GRID_FLAG_01189184), 0);

        noop(&mut e, &[0x0066_b0d0]);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1570, &args![0x55u32]);
        });
        assert_eq!(calls_to(&log, 0x0066_b0d0), vec![vec![0x011f_95e8, 0x55]]);
    }

    #[test]
    fn running_cell_tests_is_either_flag() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        assert!(!e.call(0x0045_1530, &args![tes]).bool());
        e.set(tes, TES::bRunningCellTests, true);
        assert!(e.call(0x0045_1530, &args![tes]).bool());
        e.set(tes, TES::bRunningCellTests, false);
        e.set(tes, TES::bRunningCellTests2, true);
        assert!(e.call(0x0045_1530, &args![tes]).bool());
    }

    /// The engine for `fn_004515a0`: a data handler whose `bSaveLoad` is
    /// clear, the `TESSaveLoadGame` object and doubles for the callees. The
    /// guard byte is set, so `004512c0` returns at once.
    fn centre_engine() -> (Engine, Ptr<TES>) {
        let (mut e, tes) = position_engine();
        let handler = e.mem.alloc(0x63c);
        e.set_global(DATA_HANDLER, handler);
        e.map(SAVE_LOAD_GAME, 4);
        e.set_global(SAVE_LOAD_GAME, 0x1111u32);
        e.map(GRID_LOAD_IN_PROGRESS, 1);
        e.set_global(GRID_LOAD_IN_PROGRESS, 1u8);
        returns(&mut e, 0x0047_c850, 0);
        returns(&mut e, GET_WORLD_SPACE, 0x6000);
        noop(&mut e, &[0x0062_f460, 0x0045_8200]);
        (e, tes)
    }

    #[test]
    fn centring_on_a_position_sets_the_grid_and_queued_cells() {
        let (mut e, tes) = centre_engine();
        let pos = position(&mut e, 3.0 * 4096.0 + 10.0, -4096.0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_15a0, &args![tes, pos, 1u8]);
        });
        assert_eq!(e.get(tes, TES::iCurrentGridX), 3);
        assert_eq!(e.get(tes, TES::iCurrentGridY), -1);
        assert_eq!(e.get(tes, TES::iCurrentQueuedX), 3);
        assert_eq!(e.get(tes, TES::iCurrentQueuedY), -1);
        assert_eq!(
            addresses(&log),
            vec![
                0x0045_15a0,
                0x0047_c850,
                GET_INTERIOR_CELL,
                GET_WORLD_SPACE,
                FLOAT_TO_INT_ROUNDED,
                FLOAT_TO_INT_ROUNDED,
                GET_INTERIOR_CELL,
                0x0062_f460,
            ]
        );
        assert_eq!(calls_to(&log, 0x0047_c850), vec![vec![0x1111]]);
    }

    #[test]
    fn centring_without_a_position_keeps_the_cell_and_sets_a_missing_world_space() {
        let (mut e, tes) = centre_engine();
        returns(&mut e, GET_WORLD_SPACE, 0);
        let holder = e.mem.alloc(8);
        let world_space_slot = e.mem.alloc(8);
        e.mem.set_u32(world_space_slot, 0x9000);
        returns(&mut e, 0x0046_0140, holder);
        returns(&mut e, 0x0068_15c0, world_space_slot);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_15a0, &args![tes, 0u32, 0u8]);
        });
        assert_eq!(e.get(tes, TES::iCurrentGridX), 10);
        assert_eq!(e.get(tes, TES::iCurrentGridY), 20);
        assert_eq!(e.get(tes, TES::iCurrentQueuedX), 10);
        assert_eq!(e.get(tes, TES::iCurrentQueuedY), 20);
        assert_eq!(calls_to(&log, 0x0045_8200), vec![vec![tes.addr(), 0x9000]]);
        assert!(calls_to(&log, FLOAT_TO_INT_ROUNDED).is_empty());
    }

    #[test]
    fn centring_is_skipped_during_a_save_load_unless_forced() {
        let (mut e, tes) = centre_engine();
        let handler: u32 = e.global(DATA_HANDLER);
        e.mem.set_u8(handler + 0x61a, 1);
        let pos = position(&mut e, 3.0 * 4096.0, 0.0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_15a0, &args![tes, pos, 0u8]);
        });
        assert_eq!(
            addresses(&log),
            vec![0x0045_15a0, 0x0047_c850, GET_INTERIOR_CELL]
        );
        assert_eq!(e.get(tes, TES::iCurrentGridX), 10);

        // `0047c850` says to go on regardless.
        returns(&mut e, 0x0047_c850, 1);
        e.call(0x0045_15a0, &args![tes, pos, 0u8]);
        assert_eq!(e.get(tes, TES::iCurrentGridX), 3);
    }

    #[test]
    fn save_load_flag_is_read_from_the_data_handler() {
        let mut e = Engine::new();
        let handler = e.mem.alloc(0x63c);
        assert!(!e.call(0x0045_16b0, &args![handler]).bool());
        e.mem.set_u8(handler + 0x61a, 1);
        assert!(e.call(0x0045_16b0, &args![handler]).bool());
    }

    /// A 5 x 5 grid (the centre is (2, 2)) with the given cells (x, y, flags).
    fn water_engine(cells: &[(u32, u32, u8)]) -> (Engine, Ptr<TES>, Vec<u32>) {
        let (mut e, tes, table) = grid_engine(5);
        let mut made = vec![];
        for (x, y, flags) in cells {
            let cell = new_cell(&mut e, 3, *flags);
            put_cell(&mut e, table, 5, *x, *y, cell);
            made.push(table + (x * 5 + y) * 4);
        }
        (e, tes, made)
    }

    #[test]
    fn water_search_prefers_the_centre_cell() {
        let (mut e, tes, slots) = water_engine(&[(2, 2, 2), (2, 3, 2), (1, 1, 2)]);
        assert_eq!(
            e.call(0x0045_16d0, &args![tes]).u32(),
            slots[0],
            "the centre"
        );
    }

    #[test]
    fn water_search_goes_ring_by_ring_and_stops_before_the_outermost_ring() {
        // The first ring wins over a cell further out.
        let (mut e, tes, slots) = water_engine(&[(3, 3, 2), (1, 2, 0)]);
        assert_eq!(e.call(0x0045_16d0, &args![tes]).u32(), slots[0]);
        // `ring < uGridsToLoad / 2`: the outermost ring of a 5 x 5 grid
        // (ring 2) is not searched.
        let (mut e, tes, _) = water_engine(&[(0, 4, 2), (0, 0, 2), (2, 0, 2)]);
        assert_eq!(e.call(0x0045_16d0, &args![tes]).u32(), 0);
    }

    #[test]
    fn water_search_scans_the_rows_then_the_columns_of_a_ring() {
        // Ring 1: rows y = 3 and y = 1 for x = 1..=3, then the columns x = 1
        // and x = 3 for y = 2.
        let (mut e, tes, slots) = water_engine(&[(3, 2, 2)]);
        assert_eq!(e.call(0x0045_16d0, &args![tes]).u32(), slots[0]);
        // At the same x the row at the high y comes before the low one.
        let (mut e, tes, slots) = water_engine(&[(1, 1, 2), (1, 3, 2)]);
        assert_eq!(e.call(0x0045_16d0, &args![tes]).u32(), slots[1]);
        // Rows come before columns.
        let (mut e, tes, slots) = water_engine(&[(3, 2, 2), (3, 1, 2)]);
        assert_eq!(e.call(0x0045_16d0, &args![tes]).u32(), slots[1]);
        // In the columns the low x comes before the high x.
        let (mut e, tes, slots) = water_engine(&[(3, 2, 2), (1, 2, 2)]);
        assert_eq!(e.call(0x0045_16d0, &args![tes]).u32(), slots[1]);
    }

    #[test]
    fn water_search_finds_nothing_without_water() {
        let (mut e, tes, _) = water_engine(&[(2, 2, 1), (1, 1, 0)]);
        assert_eq!(e.call(0x0045_16d0, &args![tes]).u32(), 0);
    }

    /// An engine for `fn_00451900`: a 5 x 5 grid centred on (10, 20) (cells
    /// 8..13, 18..23), the cell at (9, 19) loaded.
    fn lookup_engine() -> (Engine, Ptr<TES>, u32) {
        let (mut e, tes, table) = grid_engine(5);
        e.set(tes, TES::iCurrentGridX, 10);
        e.set(tes, TES::iCurrentGridY, 20);
        e.set(tes, TES::pWorldSpace, Ptr::new(0x6000));
        let cell = new_cell(&mut e, 3, 0);
        put_cell(&mut e, table, 5, 1, 1, cell);
        (e, tes, cell)
    }

    #[test]
    fn cell_lookup_inside_the_grid_uses_the_slot() {
        let (mut e, tes, cell) = lookup_engine();
        let log = run_logged(&mut e, |e| {
            assert_eq!(e.call(0x0045_1900, &args![tes, 9i32, 19i32]).u32(), cell);
        });
        assert!(calls_to(&log, 0x0046_1c20).is_empty());
        // Relative coordinates are passed to `GridCellArray::Get`.
        let array = e.get(tes, TES::pGridCellA).addr();
        assert_eq!(calls_to(&log, GRID_CELL_ARRAY_GET), vec![vec![array, 1, 1]]);
    }

    #[test]
    fn cell_lookup_falls_back_to_the_data_handler() {
        let (mut e, tes, _) = lookup_engine();
        returns(&mut e, 0x0046_1c20, 0x7777);
        // Outside the grid, an empty slot (8, 18 is empty) and below zero.
        for (x, y) in [(40i32, 19i32), (8, 18), (7, 18), (9, 40)] {
            let log = run_logged(&mut e, |e| {
                assert_eq!(e.call(0x0045_1900, &args![tes, x, y]).u32(), 0x7777);
            });
            assert_eq!(
                calls_to(&log, 0x0046_1c20),
                vec![vec![0x1234, x as u32, y as u32, 0x6000, 0]]
            );
        }
    }

    #[test]
    fn cell_lookup_without_a_world_space_is_null() {
        let (mut e, tes, _) = lookup_engine();
        e.set(tes, TES::pWorldSpace, Ptr::NULL);
        assert_eq!(e.call(0x0045_1900, &args![tes, 40i32, 19i32]).u32(), 0);
    }

    /// `fn_004519d0` on the lookup grid, with the `TES` singleton and the
    /// cell's load state in `state`.
    fn loaded_cell_engine(state: u8) -> (Engine, Ptr<TES>, u32) {
        let (mut e, tes, cell) = lookup_engine();
        install_ftol(&mut e);
        returns(&mut e, GET_INTERIOR_CELL, 0);
        e.map(TES_SINGLETON, 4);
        e.set_global(TES_SINGLETON, tes.addr());
        e.mem.set_u8(cell + 0x26, state);
        (e, tes, cell)
    }

    #[test]
    fn loaded_cell_at_a_position_is_the_exterior_cell_when_loaded() {
        // `TES::IsCellLoaded` with flag 1 accepts the states 5 and 6 only.
        let (mut e, tes, cell) = loaded_cell_engine(6);
        let x = 9.0f32 * 4096.0 + 100.0;
        let y = 19.0f32 * 4096.0 + 200.0;
        assert_eq!(e.call(0x0045_19d0, &args![tes, x, y, 0u32]).u32(), cell);
    }

    #[test]
    fn loaded_cell_at_a_position_is_null_when_not_loaded_or_unknown() {
        let (mut e, tes, _) = loaded_cell_engine(3);
        let x = 9.0f32 * 4096.0 + 100.0;
        let y = 19.0f32 * 4096.0 + 200.0;
        assert_eq!(e.call(0x0045_19d0, &args![tes, x, y, 0u32]).u32(), 0);
        // A cell outside the grid that the data handler does not have.
        let (mut e, tes, _) = loaded_cell_engine(6);
        returns(&mut e, 0x0046_1c20, 0);
        assert_eq!(e.call(0x0045_19d0, &args![tes, 4.0e6f32, y, 0u32]).u32(), 0);
    }

    #[test]
    fn loaded_cell_at_a_position_is_the_interior_cell_when_there_is_one() {
        let (mut e, tes, _) = loaded_cell_engine(3);
        returns(&mut e, GET_INTERIOR_CELL, 0x4444);
        assert_eq!(
            e.call(0x0045_19d0, &args![tes, 1.0f32, 2.0f32, 0u32]).u32(),
            0x4444
        );
    }

    /// An engine for `tes_load_grid_cell`: world space 0x6000, an existing
    /// cell, land at 0x7100 with lowest height 5.0 and water height 10.0.
    fn load_cell_engine() -> (Engine, Ptr<TES>, u32) {
        let (mut e, tes, _) = grid_engine(5);
        install_getters(&mut e);
        e.set(tes, TES::pWorldSpace, Ptr::new(0x6000));
        e.set(tes, TES::pObjRoot, Ptr::new(0x7000));
        let cell = new_cell(&mut e, 1, 0);
        returns(&mut e, 0x0058_5b30, cell);
        returns(&mut e, 0x004b_a5a0, 0);
        returns(&mut e, 0x0084_e3a0, 0xabcd);
        returns(&mut e, CELL_GET_DATA_X, 3);
        returns(&mut e, CELL_GET_DATA_Y, 4);
        returns(&mut e, 0x0040_1280, 0x0101_1584);
        returns(&mut e, 0x0054_6fb0, 0x7100);
        returns(&mut e, 0x0053_5b90, 1);
        e.register(0x0053_f440, |e, a| {
            e.mem.set_f32(a[1], 5.0);
            e.mem.set_f32(a[1] + 4, 99.0);
            Ret::default()
        });
        returns_float(&mut e, 0x0054_71e0, 10.0);
        noop(
            &mut e,
            &[
                0x0045_4b90,
                0x00ec_623a,
                0x004b_a550,
                GRID_CELL_ARRAY_ATTACH_TO_WORLD,
                0x0053_a090,
                0x0049_c860,
                0x0053_fa40,
                0x0058_8150,
                0x0055_1440,
            ],
        );
        put_setting(&mut e, SETTING_USE_WATER, 1u8);
        (e, tes, cell)
    }

    #[test]
    fn grid_cell_load_without_a_world_space_does_nothing() {
        let (mut e, tes, _) = load_cell_engine();
        e.set(tes, TES::pWorldSpace, Ptr::NULL);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1a50, &args![tes, 1u32, 2u32, 30i32, 40i32]);
        });
        assert_eq!(addresses(&log), vec![0x0045_1a50, GET_WORLD_SPACE]);
    }

    #[test]
    fn grid_cell_load_into_an_attached_slot_only_adds_to_the_buffer() {
        let (mut e, tes, cell) = load_cell_engine();
        returns(&mut e, 0x004b_a5a0, 1);
        let array = e.get(tes, TES::pGridCellA).addr();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1a50, &args![tes, 1u32, 2u32, 30i32, 40i32]);
        });
        assert_eq!(calls_to(&log, 0x0058_5b30), vec![vec![0x6000, 30, 40]]);
        assert_eq!(calls_to(&log, 0x004b_a5a0), vec![vec![array, 1, 2]]);
        assert_eq!(calls_to(&log, 0x0045_4b90), vec![vec![tes.addr(), cell]]);
        assert!(calls_to(&log, 0x004b_a550).is_empty());
        assert!(calls_to(&log, 0x00ec_623a).is_empty());
    }

    #[test]
    fn grid_cell_load_attaches_the_cell_and_marks_water() {
        let (mut e, tes, cell) = load_cell_engine();
        let array = e.get(tes, TES::pGridCellA).addr();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1a50, &args![tes, 1u32, 2u32, 30i32, 40i32]);
        });
        // Not created: the world space had the cell.
        assert!(calls_to(&log, 0x0046_1c20).is_empty());
        // The message: name, x, y, form id.
        let message = calls_to(&log, 0x00ec_623a);
        assert_eq!(
            message[0][1..],
            [LOADING_CELL_FORMAT, 0x0101_1584, 3, 4, 0xabcd]
        );
        assert_eq!(calls_to(&log, 0x004b_a550), vec![vec![array, 1, 2, cell]]);
        assert_eq!(
            calls_to(&log, GRID_CELL_ARRAY_ATTACH_TO_WORLD),
            vec![vec![array, 0x7000, 1, 2]]
        );
        assert_eq!(calls_to(&log, 0x0053_5b90), vec![vec![0x7100, 1]]);
        // Vertices loaded: no mesh update.
        assert!(calls_to(&log, 0x0053_a090).is_empty());
        // Lowest land height 5.0 below the water at 10.0: the water bit.
        assert_eq!(e.mem.u8(cell + 0x24) & 2, 2);
        // `bUseWater`, a water cell without auto water: placeable water.
        assert_eq!(calls_to(&log, 0x0049_c860), vec![vec![cell]]);
        assert_eq!(calls_to(&log, 0x0053_fa40), vec![vec![0x7100, 0]]);
        // The message buffer was a stack block, freed again.
        assert_eq!(e.mem.block_size(message[0][0]), None);
    }

    #[test]
    fn grid_cell_load_creates_a_missing_cell() {
        let (mut e, tes, _) = load_cell_engine();
        returns(&mut e, 0x0058_5b30, 0);
        let created = new_cell(&mut e, 1, 0);
        returns(&mut e, 0x0046_1c20, created);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1a50, &args![tes, 1u32, 2u32, 30i32, 40i32]);
        });
        assert_eq!(
            calls_to(&log, 0x0046_1c20),
            vec![vec![0x1234, 30, 40, 0x6000, 1]]
        );
        assert_eq!(calls_to(&log, 0x0058_8150), vec![vec![0x6000, created]]);
        assert_eq!(calls_to(&log, 0x0055_1440), vec![vec![created, 1]]);
        let array = e.get(tes, TES::pGridCellA).addr();
        assert_eq!(
            calls_to(&log, 0x004b_a550),
            vec![vec![array, 1, 2, created]]
        );
    }

    #[test]
    fn grid_cell_load_clears_the_water_bit_above_the_water_and_updates_the_mesh() {
        let (mut e, tes, cell) = load_cell_engine();
        e.mem.set_u8(cell + 0x24, 2);
        returns_float(&mut e, 0x0054_71e0, 5.0);
        returns(&mut e, 0x0053_5b90, 0);
        e.set(tes, TES::bShowLANDborders, true);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1a50, &args![tes, 1u32, 2u32, 30i32, 40i32]);
        });
        // 5.0 < 5.0 is false.
        assert_eq!(e.mem.u8(cell + 0x24) & 2, 0);
        assert!(calls_to(&log, 0x0049_c860).is_empty());
        assert_eq!(calls_to(&log, 0x0053_a090), vec![vec![0x7100, 0, 0]]);
        assert_eq!(calls_to(&log, 0x0053_fa40), vec![vec![0x7100, 1]]);
    }

    #[test]
    fn placeable_water_is_not_generated_twice_or_without_the_setting() {
        let (mut e, tes, cell) = load_cell_engine();
        e.mem.set_u8(cell + 0x54, 1); // bAutoWaterLoaded
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1a50, &args![tes, 1u32, 2u32, 30i32, 40i32]);
        });
        assert!(calls_to(&log, 0x0049_c860).is_empty());

        let (mut e, tes, _) = load_cell_engine();
        put_setting(&mut e, SETTING_USE_WATER, 0u8);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1a50, &args![tes, 1u32, 2u32, 30i32, 40i32]);
        });
        assert!(calls_to(&log, 0x0049_c860).is_empty());
    }

    #[test]
    fn cell_name_and_auto_water_accessors() {
        let mut e = Engine::new();
        returns(&mut e, 0x0040_1280, 0x0101_1584);
        let cell = new_cell(&mut e, 0, 0);
        assert_eq!(e.call(0x0045_1cb0, &args![cell]).u32(), 0x0101_1584);
        assert_eq!(e.call(0x0045_1cd0, &args![cell]).u32(), 0);
        e.mem.set_u8(cell + 0x54, 1);
        assert_eq!(e.call(0x0045_1cd0, &args![cell]).u32(), 1);
    }

    #[test]
    fn grid_lights_are_attached_for_cells_in_state_6() {
        let (mut e, tes, table) = grid_engine(2);
        noop(&mut e, &[0x0054_ba80, 0x00b5_ddb0]);
        e.map(SHADOW_SCENE_NODE_TABLE, 16);
        e.set_global(SHADOW_SCENE_NODE_TABLE, 0x7777u32);
        let six = new_cell(&mut e, 6, 0);
        let three = new_cell(&mut e, 3, 0);
        put_cell(&mut e, table, 2, 0, 1, six);
        put_cell(&mut e, table, 2, 1, 0, three);
        put_cell(&mut e, table, 2, 1, 1, six);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1cf0, &args![tes]);
        });
        assert_eq!(
            calls_to(&log, 0x0054_ba80),
            vec![vec![six, 1], vec![six, 1]]
        );
        assert_eq!(calls_to(&log, 0x00b5_ddb0), vec![vec![0x7777, 1, 1]]);
        assert_eq!(log.last().unwrap().0, 0x00b5_ddb0);
    }

    /// An actor-like reference for `fn_00451da0`: slot 0x1d0 returns the
    /// parent `0x5000`.
    fn parent_engine() -> (Engine, Ptr<TES>, u32, u32) {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        returns(&mut e, 0x0056_4e60, 1);
        returns(&mut e, 0x0056_51e0, 1);
        returns(&mut e, 0x0300_01d0, 0x5000);
        noop(&mut e, &[0x0057_0f70, 0x0056_5210, 0x0300_00e8]);
        let handler = object_with_slots(&mut e, 0x0300_4000, &[(0xe8, 0x0300_00e8)]);
        returns(&mut e, 0x0096_11e0, handler);
        let refr = object_with_slots(&mut e, 0x0300_3000, &[(0x1d0, 0x0300_01d0)]);
        (e, tes, refr, handler)
    }

    #[test]
    fn reference_cleanup_hands_the_parent_to_the_handler_and_clears_the_state() {
        let (mut e, tes, refr, handler) = parent_engine();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1da0, &args![tes, refr]);
        });
        assert_eq!(calls_to(&log, 0x0096_11e0), vec![vec![0x5000]]);
        assert_eq!(calls_to(&log, 0x0300_00e8), vec![vec![handler, 0x5000]]);
        assert_eq!(calls_to(&log, 0x0057_0f70), vec![vec![refr, 0]]);
        assert_eq!(calls_to(&log, 0x0056_5210), vec![vec![refr, 0]]);
    }

    #[test]
    fn reference_cleanup_without_a_handler_skips_only_the_handler_call() {
        let (mut e, tes, refr, _) = parent_engine();
        returns(&mut e, 0x0096_11e0, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1da0, &args![tes, refr]);
        });
        assert!(calls_to(&log, 0x0300_00e8).is_empty());
        assert_eq!(calls_to(&log, 0x0056_5210), vec![vec![refr, 0]]);
    }

    #[test]
    fn reference_cleanup_stops_at_the_first_failed_condition() {
        // Not visible when distant.
        let (mut e, tes, refr, _) = parent_engine();
        returns(&mut e, 0x0056_4e60, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1da0, &args![tes, refr]);
        });
        assert_eq!(addresses(&log), vec![0x0045_1da0, 0x0056_4e60]);
        // No parent.
        let (mut e, tes, refr, _) = parent_engine();
        returns(&mut e, 0x0300_01d0, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1da0, &args![tes, refr]);
        });
        assert_eq!(addresses(&log), vec![0x0045_1da0, 0x0056_4e60, 0x0300_01d0]);
        // The flag is not set.
        let (mut e, tes, refr, _) = parent_engine();
        returns(&mut e, 0x0056_51e0, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1da0, &args![tes, refr]);
        });
        assert!(calls_to(&log, 0x0056_5210).is_empty());
    }

    #[test]
    fn reference_is_loadable_unless_one_of_the_two_tests_says_no() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        returns(&mut e, REFERENCE_GET_BASE_FORM, 0x5000);
        returns(&mut e, FORM_GET_TYPE, 0x10);
        returns(&mut e, 0x0044_0d80, 0);
        returns(&mut e, 0x0044_0da0, 0);
        assert!(e.call(0x0045_1e40, &args![tes, 0x6000u32]).bool());
        returns(&mut e, 0x0044_0da0, 1);
        assert!(!e.call(0x0045_1e40, &args![tes, 0x6000u32]).bool());
        returns(&mut e, 0x0044_0da0, 0);
        returns(&mut e, 0x0044_0d80, 1);
        let log = run_logged(&mut e, |e| {
            assert!(!e.call(0x0045_1e40, &args![tes, 0x6000u32]).bool());
        });
        // The second test is not asked.
        assert!(calls_to(&log, 0x0044_0da0).is_empty());
    }

    #[test]
    fn actor_is_taken_off_the_temp_change_list() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        returns(&mut e, 0x0300_0100, 1);
        noop(&mut e, &[0x0300_0260, 0x0096_f400]);
        let actor = object_with_slots(
            &mut e,
            0x0300_3000,
            &[(0x100, 0x0300_0100), (0x260, 0x0300_0260)],
        );
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ea0, &args![tes, actor]);
        });
        assert_eq!(
            addresses(&log),
            vec![0x0045_1ea0, 0x0300_0100, 0x0300_0260, 0x0096_f400]
        );
        assert_eq!(
            calls_to(&log, 0x0096_f400),
            vec![vec![OBJECT_011E0E80, actor]]
        );
        // Not an actor: nothing more.
        returns(&mut e, 0x0300_0100, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ea0, &args![tes, actor]);
        });
        assert_eq!(addresses(&log), vec![0x0045_1ea0, 0x0300_0100]);
    }

    /// An engine for `fn_00451ef0` with a non-actor reference of form type
    /// 0x10 whose slot 0x1c8 returns the node `0x9001`, in a cell of state 6.
    /// Returns the engine, the `TES`, the reference and the cell.
    fn reference_engine() -> (Engine, Ptr<TES>, u32, u32) {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        e.map(SHADOW_SCENE_NODE_TABLE, 16);
        e.set_global(SHADOW_SCENE_NODE_TABLE, 0x7777u32);
        for addr in [SCRIPT_CONTEXT, PLAYER_CHARACTER] {
            e.map(addr, 4);
        }
        e.set_global(PLAYER_CHARACTER, 0x3333u32);
        e.map(LARGE_CELL_LIMIT, 8);
        e.set_global(LARGE_CELL_LIMIT, 200.0f64);
        noop(
            &mut e,
            &[
                0x0057_0f70,
                0x0054_5cf0,
                0x00b5_eeb0,
                0x00b5_cbd0,
                0x0054_8880,
                0x0056_f700,
                0x0057_8170,
                0x0057_8060,
                0x0048_3710,
                0x0045_34f0,
                0x00b5_d9f0,
                0x0056_5730,
                0x005a_c190,
                0x005a_c750,
                0x008c_4640,
                0x004b_6dc0,
                0x0054_7ad0,
                0x0300_0308,
                0x0300_0304,
                0x0300_0260,
                0x0300_0040,
                0x0300_00e8,
                0x0096_f400,
            ],
        );
        returns(&mut e, REFERENCE_GET_BASE_FORM, 0x5000);
        returns(&mut e, FORM_GET_TYPE, 0x10);
        for addr in [
            0x0056_4e60,
            0x0044_0d80,
            0x0044_0da0,
            0x0057_b200,
            0x0045_2440,
            0x0300_0224,
            0x0300_0220,
            0x0300_0100,
            0x0096_11e0,
            0x005d_8710,
            0x0056_51e0,
            0x0300_01d0,
        ] {
            returns(&mut e, addr, 0);
        }
        returns(&mut e, 0x0043_fcd0, 0x9100);
        returns(&mut e, 0x0300_01c8, 0x9001);
        returns(&mut e, 0x0042_ce10, 1);
        returns(&mut e, 0x0084_e3a0, 0x1010);
        returns(&mut e, 0x0043_0830, 0x2020);
        returns(&mut e, 0x005d_43c0, 0x44);
        returns(&mut e, 0x0045_2480, 1);
        returns(&mut e, 0x00ec_43fb, 0);
        let refr = object_with_slots(
            &mut e,
            0x0300_3000,
            &[
                (0x100, 0x0300_0100),
                (0x1c8, 0x0300_01c8),
                (0x1d0, 0x0300_01d0),
                (0x220, 0x0300_0220),
                (0x224, 0x0300_0224),
                (0x260, 0x0300_0260),
                (0x304, 0x0300_0304),
                (0x308, 0x0300_0308),
            ],
        );
        // Loaded data with flag 2, read through `pLoadedData` (+0x64).
        let loaded_data = e.mem.alloc(0x20);
        e.mem.set_u32(loaded_data + 0x10, 2);
        e.mem.set_u32(refr + 0x64, loaded_data);
        let cell = new_cell(&mut e, 6, 0);
        (e, tes, refr, cell)
    }

    #[test]
    fn loading_a_null_reference_does_nothing() {
        let (mut e, tes, _, cell) = reference_engine();
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, 0u32, cell, 0u32, 1u8]);
        });
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn loading_a_reference_with_slot_0x224_or_0x220_calls_the_matching_slot_only() {
        let (mut e, tes, refr, cell) = reference_engine();
        returns(&mut e, 0x0300_0224, 1);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(addresses(&log), vec![0x0045_1ef0, 0x0300_0224, 0x0300_0308]);
        returns(&mut e, 0x0300_0224, 0);
        returns(&mut e, 0x0300_0220, 1);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(
            addresses(&log),
            vec![0x0045_1ef0, 0x0300_0224, 0x0300_0220, 0x0300_0304]
        );
    }

    #[test]
    fn loading_a_plain_reference_runs_the_whole_sequence() {
        let (mut e, tes, refr, cell) = reference_engine();
        // The action list is set up when `0042ce10` says no.
        returns(&mut e, 0x0042_ce10, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(
            addresses(&log),
            vec![
                0x0045_1ef0,
                0x0300_0224,
                0x0300_0220,
                // The cleanup (`fn_00451da0`) asks `00564e60` and stops.
                0x0056_4e60,
                // The base form and its type.
                REFERENCE_GET_BASE_FORM,
                FORM_GET_TYPE,
                // Whether to load it (`fn_00451e40`).
                REFERENCE_GET_BASE_FORM,
                FORM_GET_TYPE,
                0x0044_0d80,
                0x0044_0da0,
                // The 3D from slot 0x1c8.
                0x0300_01c8,
                // The cell's 3D, then the node into the shadow scene node.
                0x0054_5cf0,
                0x00b5_eeb0,
                0x0300_0100,
                0x0056_4e60,
                REFERENCE_GET_BASE_FORM,
                FORM_GET_TYPE,
                0x0054_8880,
                0x0056_f700,
                // The master particle addon nodes (loaded-data flag 2, state 6).
                0x0057_8060,
                0x0300_0100,
                0x00b5_d9f0,
                // The script.
                0x0056_5730,
                0x0042_ce10,
                0x005d_43c0,
                0x005a_c190,
                0x005d_43c0,
                0x005a_c750,
                0x0300_0100,
            ]
        );
        assert_eq!(calls_to(&log, 0x0300_01c8), vec![vec![refr, 0]]);
        assert_eq!(calls_to(&log, 0x0054_5cf0), vec![vec![cell]]);
        assert_eq!(calls_to(&log, 0x00b5_eeb0), vec![vec![0x7777, 0x9001]]);
        assert_eq!(calls_to(&log, 0x0054_8880), vec![vec![cell, refr, 0]]);
        assert_eq!(calls_to(&log, 0x0056_f700), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x0057_8060), vec![vec![0x9001]]);
        assert_eq!(calls_to(&log, 0x00b5_d9f0), vec![vec![0x7777, 0x9001, 0]]);
        assert_eq!(calls_to(&log, 0x0056_5730), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x005a_c190), vec![vec![refr, 0x44]]);
        // The third argument is the 0x1000 pushed before the second
        // `005d43c0` call.
        assert_eq!(calls_to(&log, 0x005a_c750), vec![vec![refr, 0x44, 0x1000]]);
    }

    #[test]
    fn loading_an_actor_registers_its_node_and_sets_the_reference_id() {
        let (mut e, tes, refr, cell) = reference_engine();
        returns(&mut e, 0x0300_0100, 1);
        returns(&mut e, 0x0056_4e60, 1);
        let log = run_logged(&mut e, |e| {
            // With the flag 0 the actor is first taken off the list.
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 0u8]);
        });
        assert_eq!(calls_to(&log, 0x0300_0260), vec![vec![refr]]);
        assert_eq!(
            calls_to(&log, 0x0096_f400),
            vec![vec![OBJECT_011E0E80, refr]]
        );
        assert_eq!(calls_to(&log, 0x00b5_cbd0), vec![vec![0x7777, 0x9001]]);
        assert_eq!(calls_to(&log, 0x0048_3710), vec![vec![refr]]);
        // The counter of references visible when distant goes up.
        assert_eq!(e.mem.u16(cell + 0xaa), 1);
        assert_eq!(calls_to(&log, 0x008c_4640), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x0084_e3a0), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x004b_6dc0), vec![vec![0x9100, 0x1010]]);
        // The last call is the reference-ID call.
        assert_eq!(log.last().unwrap().0, 0x004b_6dc0);
    }

    #[test]
    fn the_player_does_not_get_a_reference_id_on_the_scene_graph() {
        let (mut e, tes, refr, cell) = reference_engine();
        returns(&mut e, 0x0300_0100, 1);
        e.set_global(PLAYER_CHARACTER, refr);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(calls_to(&log, 0x008c_4640), vec![vec![refr]]);
        assert!(calls_to(&log, 0x004b_6dc0).is_empty());
    }

    #[test]
    fn a_reference_that_is_not_to_be_loaded_stops_after_the_cleanup() {
        let (mut e, tes, refr, cell) = reference_engine();
        returns(&mut e, 0x0044_0d80, 1);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert!(calls_to(&log, 0x0300_01c8).is_empty());
        assert!(calls_to(&log, 0x0056_5730).is_empty());
    }

    #[test]
    fn a_node_from_the_source_is_attached_to_the_reference_and_the_source_is_notified() {
        let (mut e, tes, refr, cell) = reference_engine();
        let source = object_with_slots(&mut e, 0x0300_5000, &[(0x40, 0x0300_0040)]);
        returns(&mut e, 0x005d_8710, 0xa001);
        returns(&mut e, 0x0043_fcd0, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, source, 1u8]);
        });
        assert_eq!(
            calls_to(&log, 0x005d_8710),
            vec![vec![source], vec![source]]
        );
        assert_eq!(calls_to(&log, 0x0057_0f70), vec![vec![refr, 0xa001]]);
        assert_eq!(calls_to(&log, 0x0300_0040), vec![vec![source]]);
        assert!(calls_to(&log, 0x0300_01c8).is_empty());
        assert_eq!(calls_to(&log, 0x0054_8880), vec![vec![cell, refr, 0]]);
        // The node already attached to the reference is not set again.
        returns(&mut e, 0x0043_fcd0, 0x9100);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, source, 1u8]);
        });
        assert!(calls_to(&log, 0x0057_0f70).is_empty());
    }

    #[test]
    fn a_source_without_a_node_falls_back_to_slot_0x1c8() {
        let (mut e, tes, refr, cell) = reference_engine();
        let source = object_with_slots(&mut e, 0x0300_5000, &[(0x40, 0x0300_0040)]);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, source, 1u8]);
        });
        assert_eq!(calls_to(&log, 0x0300_01c8), vec![vec![refr, 0]]);
        assert_eq!(calls_to(&log, 0x0300_0040), vec![vec![source]]);
    }

    #[test]
    fn form_types_0x2c_and_0x2d_hand_the_node_to_the_handler() {
        for kind in [0x2c, 0x2d] {
            let (mut e, tes, refr, cell) = reference_engine();
            returns(&mut e, FORM_GET_TYPE, kind);
            let handler = object_with_slots(&mut e, 0x0300_4000, &[(0xe8, 0x0300_00e8)]);
            returns(&mut e, 0x0096_11e0, handler);
            let log = run_logged(&mut e, |e| {
                e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
            });
            assert_eq!(calls_to(&log, 0x0300_00e8), vec![vec![handler, 0x9001]]);
            assert!(calls_to(&log, 0x0054_5cf0).is_empty());
            assert!(calls_to(&log, 0x0054_8880).is_empty());
            // The script is still initialised.
            assert_eq!(calls_to(&log, 0x0056_5730), vec![vec![refr]]);
        }
    }

    #[test]
    fn type_0x23_forms_that_00452440_rejects_are_not_attached() {
        let (mut e, tes, refr, cell) = reference_engine();
        returns(&mut e, FORM_GET_TYPE, 0x23);
        returns(&mut e, 0x0045_2440, 1);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert!(calls_to(&log, 0x0054_8880).is_empty());
        assert!(calls_to(&log, 0x0056_f700).is_empty());
        assert_eq!(calls_to(&log, 0x0045_2440), vec![vec![0x5000]]);
        // Accepted by `00452440`: attached as usual.
        returns(&mut e, 0x0045_2440, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(calls_to(&log, 0x0054_8880).len(), 1);
    }

    #[test]
    fn type_0x1e_forms_use_the_extra_data_call_instead_of_the_lighting_update() {
        let (mut e, tes, refr, cell) = reference_engine();
        returns(&mut e, FORM_GET_TYPE, 0x1e);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(calls_to(&log, 0x0043_0830), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x0045_34f0), vec![vec![0x5000, 0x2020]]);
        assert!(calls_to(&log, 0x00b5_d9f0).is_empty());
    }

    #[test]
    fn master_particle_addon_nodes_are_removed_or_added() {
        // Removed: the form flag and `0057b200`.
        let (mut e, tes, refr, cell) = reference_engine();
        e.mem.set_u32(refr + 8, 0x0100_0000);
        returns(&mut e, 0x0057_b200, 1);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(calls_to(&log, 0x0057_8170), vec![vec![0x9001]]);
        assert!(calls_to(&log, 0x0057_8060).is_empty());
        // The flag without `0057b200`: the add condition applies.
        returns(&mut e, 0x0057_b200, 0);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert!(calls_to(&log, 0x0057_8170).is_empty());
        assert_eq!(calls_to(&log, 0x0057_8060), vec![vec![0x9001]]);
        // Added in a cell of state 5, not in one of state 4 or without the
        // loaded-data bit.
        e.mem.set_u8(cell + 0x26, 5);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert_eq!(calls_to(&log, 0x0057_8060).len(), 1);
        e.mem.set_u8(cell + 0x26, 4);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert!(calls_to(&log, 0x0057_8060).is_empty());
        e.mem.set_u8(cell + 0x26, 6);
        let loaded_data = e.mem.u32(refr + 0x64);
        e.mem.set_u32(loaded_data + 0x10, 4);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        assert!(calls_to(&log, 0x0057_8060).is_empty());
    }

    /// `fn_00451ef0` on a form of type 0x25 whose base form casts to `0xC000`
    /// (slot 0x1bc returns 2.5) with the two floats `first` and `second` at
    /// `+0x8c`; the reference's slot 0x1f4 returns the position (1, 2, 3) and
    /// `00567400` its scale 2.0.
    fn run_type_0x25(first: f32, second: f32, cast_exists: bool) -> Vec<(u32, Vec<u32>)> {
        let (mut e, tes, refr, cell) = reference_engine();
        returns(&mut e, FORM_GET_TYPE, 0x25);
        let cast = object_with_slots(&mut e, 0x0300_6000, &[(0x1bc, 0x0300_01bc)]);
        returns_float(&mut e, 0x0300_01bc, 2.5);
        returns(&mut e, 0x00ec_43fb, if cast_exists { cast } else { 0 });
        let sizes = e.mem.alloc(8);
        e.mem.set_f32(sizes, first);
        e.mem.set_f32(sizes + 4, second);
        returns(&mut e, 0x0045_bb80, sizes);
        returns_float(&mut e, 0x0056_7400, 2.0);
        let position = e.mem.alloc(12);
        for (i, value) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
            e.mem.set_f32(position + 4 * i as u32, value);
        }
        e.mem.set_u32(0x0300_3000 + 0x1f4, 0x0300_01f4);
        returns(&mut e, 0x0300_01f4, position);
        let log = run_logged(&mut e, |e| {
            e.call(0x0045_1ef0, &args![tes, refr, cell, 0u32, 1u8]);
        });
        let casts = calls_to(&log, 0x00ec_43fb);
        assert_eq!(
            casts,
            vec![vec![0x5000, 0, RTTI_TESFORM, RTTI_CAST_TARGET, 0]]
        );
        // Every call is made with the cell the caller passed.
        for words in calls_to(&log, 0x0054_7ad0) {
            assert_eq!(words[0], cell);
        }
        log
    }

    #[test]
    fn type_0x25_forms_call_00547ad0_with_the_position_and_the_scaled_radius() {
        let log = run_type_0x25(300.0, 250.0, true);
        let calls = calls_to(&log, 0x0054_7ad0);
        assert_eq!(calls.len(), 1);
        // 2.0 * 2.5 = 5, then the literal 1.0.
        assert_eq!(
            calls[0][1..],
            [
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits(),
                5,
                1.0f32.to_bits()
            ]
        );
    }

    #[test]
    fn type_0x25_forms_with_small_sizes_or_no_cast_make_no_call() {
        for (first, second, cast) in [
            (200.0, 300.0, true),
            (300.0, 200.0, true),
            (f32::NAN, 300.0, true),
            (300.0, 300.0, false),
        ] {
            let log = run_type_0x25(first, second, cast);
            assert!(calls_to(&log, 0x0054_7ad0).is_empty(), "{first} {second}");
        }
    }

    #[test]
    fn small_reference_and_cell_accessors() {
        let mut e = Engine::new();
        let form = e.mem.alloc(0x20);
        assert!(!e.call(0x0045_2370, &args![form]).bool());
        e.mem.set_u32(form + 8, 0x0100_0000);
        assert!(e.call(0x0045_2370, &args![form]).bool());
        e.mem.set_u32(form + 8, 0xfeff_ffff);
        assert!(!e.call(0x0045_2370, &args![form]).bool());

        let cell = new_cell(&mut e, 0, 0);
        e.mem.set_u16(cell + 0xaa, 0xffff);
        e.call(0x0045_2390, &args![cell]);
        assert_eq!(e.mem.u16(cell + 0xaa), 0);
        e.call(0x0045_2390, &args![cell]);
        assert_eq!(e.mem.u16(cell + 0xaa), 1);
    }

    #[test]
    fn loaded_data_flags_are_tested_with_a_mask() {
        let mut e = Engine::new();
        let refr = e.mem.alloc(0x80);
        // No loaded data.
        assert!(!e.call(0x0045_23e0, &args![refr, 0xffff_ffffu32]).bool());
        let data = e.mem.alloc(0x20);
        e.mem.set_u32(refr.wrapping_add(0x64), data);
        e.mem.set_u32(data + 0x10, 0b0110);
        assert!(e.call(0x0045_23e0, &args![refr, 2u32]).bool());
        assert!(e.call(0x0045_23e0, &args![refr, 0b1100u32]).bool());
        assert!(!e.call(0x0045_23e0, &args![refr, 0b1001u32]).bool());
        assert!(!e.call(0x0045_2420, &args![data, 0u32]).bool());
        assert!(e.call(0x0045_2420, &args![data, 4u32]).bool());
    }
}
