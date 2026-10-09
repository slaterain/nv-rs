//! `fallout shared/tes.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `TES` is the game's world manager: the loaded exterior/interior cells, the
//! scene graph roots, the sky and the singletons the world needs (the
//! `TESDataHandler`, the `ModelLoader`, the save/load object...). Session 1
//! of this unit (the first 40 functions in address order, `0044fb20` to
//! `00450bf0`) holds `TES::TES`, `TES::~TES` and the small static helpers
//! next to them: they copy INI settings into the globals of other
//! subsystems (Havok, LOD fade) and read or delete the singletons. The next
//! session continues at `00450c20`.
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
        /// `bRunningCellTests` (Xbox PDB).
        0x51 bRunningCellTests: bool,
        /// `bRunningCellTests2` (Xbox PDB).
        0x52 bRunningCellTests2: bool,
        /// `pTACRegionFilter` (Xbox PDB).
        0x5C pTACRegionFilter: Ptr,
        /// `bShowLANDborders` (Xbox PDB).
        0x60 bShowLANDborders: bool,
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
}
