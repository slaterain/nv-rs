//! `fallout shared/tesobjectcell.cpp` (Xbox PDB source unit), part 2: its functions from `00547650` up to
//! (not including) `00552470` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesobjectcell`]; anything public there may be used here.
//!
//! The first session of this part translated the 40 functions from
//! `00547650` to `0054b5b0`: the extra data readers (image space and water
//! type of a cell, the canopy shadow mask), `CreateCanopyShadowMaskForCell`
//! and the code that paints a tree's shadow into the mask of the cell and
//! its eight neighbours, `AddReference`, `AttachReference3D`, the node
//! attach helpers, the queue that attaches references later, and the
//! reference walks that move references between cells.
//!
//! The second session translated the 40 functions from `0054b750` to
//! `0054ee20`: the reference walks of the cell (map markers, lights, tree
//! hiding, the second attach pass of `fn_0054bcf0`, `CalcRefCenterPoint`,
//! `RunScripts`, `RemoveReference`, the clearing of the list), the
//! change-of-cell placement search (`GetCOCPlacementInfo`,
//! `DetermineCOCPlacement`), the range walks with a callback, the world space
//! accessors, `fn_0054df30` (what happens to the references of a cell that
//! was detached) and the local map texture functions with
//! `TakeLocalMapPicture`. The next session continues with the first function
//! after `0054ee20` that the ledger queue (`queue "fallout
//! shared/tesobjectcell.cpp" 00547650 00552470`) lists as not done.
//!
//! Calls to functions outside this file go by address (`e.call`), including
//! the functions of the unit's main file. The constants below name the
//! callees by what their bodies do; the exe has no symbol for most of them.
//!
//! Not translated: the C++ exception-unwinding frames (`FS:[0]` chains) and
//! the stack cookie of the functions that have them.

#[allow(unused_imports)]
use super::tesobjectcell::*;
#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------
// Callees outside this file, named by what their bodies do.
// ---------------------------------------------------------------------------

/// The cell's `ExtraDataList` (`this + 0x28`).
const CELL_EXTRA_DATA_LIST: u32 = 0x0046_10d0;
/// `TESObjectCELL::GetWorldSpace` (Xbox PDB): `pWorldSpace` of an exterior
/// cell, null for an interior one.
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
/// `TESObjectCELL::GetLand` (Xbox PDB).
const CELL_GET_LAND: u32 = 0x0054_6fb0;
/// `TESObjectCELL::GetDataX` / `GetDataY` (Xbox PDB): the exterior cell's
/// grid coordinates.
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
/// "Cell is interior": `cCellFlags & 1`.
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
/// `cCellFlags & 0x80`.
const CELL_FLAG_80: u32 = 0x0045_4b10;
/// `CellRefLockEnter` and the matching leave (the cell's reference lock).
const CELL_LOCK_ENTER: u32 = 0x0054_1ac0;
const CELL_LOCK_LEAVE: u32 = 0x0054_1ae0;
/// The list of references of the cell (`this + 0xAC`).
const CELL_REFERENCE_LIST: u32 = 0x0096_04f0;
/// The persistent-reference test of the cell (`iFormFlags & 0x400`).
const CELL_PERSISTENT_FLAG: u32 = 0x0055_16c0;
/// `cCellState` (`+0x26`) accessor, and the tests `== 6`, `== 5`, `== 2`
/// built on it.
const CELL_STATE: u32 = 0x0045_0fd0;
const CELL_STATE_IS_6: u32 = 0x0045_0ff0;
const CELL_STATE_IS_5: u32 = 0x0045_23c0;
/// `TESObjectCELL::RemoveReference` (Xbox PDB): `(cell, reference)`.
const CELL_REMOVE_REFERENCE: u32 = 0x0054_ca90;
/// `TESObjectCELL::AddActivatingRef` (Xbox PDB).
const CELL_ADD_ACTIVATING_REF: u32 = 0x0054_55c0;
/// Adds a scripted reference to the loaded-cell data (`pLoadedData`).
const CELL_ADD_SCRIPTED_REF: u32 = 0x0054_5560;
/// The cell's `pLoadedData` handler for a reference with external emittance.
const CELL_ADD_EMITTANCE_REF: u32 = 0x0054_53b0;
/// Adds a reference to the multibound node bookkeeping of the loaded data.
const CELL_ADD_MULTI_BOUND_REF: u32 = 0x0054_52c0;
/// Water reference bookkeeping (placeable water).
const CELL_ADD_WATER_REF: u32 = 0x0054_56b0;
/// The child `index` of the cell's 3D node (`ret 4`).
const CELL_CHILD_NODE: u32 = 0x0045_6fc0;

/// Reference-counted slot constructor (`(slot, value)`), assignment and
/// destructor, and the slot reader.
const SLOT_CONSTRUCT: u32 = 0x0063_3c90;
const SLOT_ASSIGN: u32 = 0x0066_b0d0;
const SLOT_RELEASE: u32 = 0x0045_cec0;
const SLOT_GET: u32 = 0x0055_9450;
/// `BSSimpleList<T>::PushFront(&item)` on the list in `ECX`.
const LIST_PUSH_FRONT: u32 = 0x005a_e3d0;
/// `BSSimpleList` iterator pieces: end test, address of the item, next node.
const LIST_IS_END: u32 = 0x0082_56d0;
const LIST_ITEM_ADDRESS: u32 = 0x0068_15c0;
const LIST_NEXT: u32 = 0x0072_6070;
/// List constructor, clear and destructor (on a local list).
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
const LIST_CLEAR: u32 = 0x0047_0470;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// Membership test of a pointer list: `(list, &item)`.
const LIST_CONTAINS: u32 = 0x005f_65d0;

/// `TESForm` accessors: `iFormFlags` bit tests (`& 0x800`, `& 0x820`,
/// `& 0x20000000`), the form id and the type byte.
const FORM_FLAG_800: u32 = 0x0044_0da0;
const FORM_FLAG_20: u32 = 0x0044_0d80;
const FORM_FLAG_820: u32 = 0x0043_7b90;
const FORM_FLAG_4000: u32 = 0x0040_77c0;
const FORM_FLAG_20000000: u32 = 0x0088_5a30;
const FORM_FLAG_800000: u32 = 0x0047_7ba0;
const FORM_FLAG_8: u32 = 0x0040_13e0;
const FORM_TYPE: u32 = 0x0040_1170;
const FORM_ID: u32 = 0x0084_e3a0;
const FORM_LOOK_UP: u32 = 0x0048_39c0;

/// The base form of a reference (`BGSSaveFormBuffer::GetForm` is a folded
/// name for this accessor).
const REFERENCE_GET_BASE_FORM: u32 = 0x007a_f430;
/// The parent cell of a reference (`+0x40`).
const REFERENCE_PARENT_CELL: u32 = 0x008d_6f30;
/// The reference's `ExtraDataList` (`this + 0x44`).
const REFERENCE_EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// The reference's 3D (the loaded model node), or null.
const REFERENCE_GET_3D: u32 = 0x0043_fcd0;
/// `TESObjectREFR::GetWorldSpace` (Xbox PDB).
const REFERENCE_GET_WORLD_SPACE: u32 = 0x0057_5d70;
/// True when `iFormFlags & 0x8000`, else the base form is asked (`00564e40`).
const REFERENCE_HAS_VISIBLE_DISTANT: u32 = 0x0056_4e60;
/// `TESObjectREFR::IsActivatingChildren` (Xbox PDB).
const REFERENCE_IS_ACTIVATING_CHILDREN: u32 = 0x0056_a250;
/// True when the base form has a script, or the reference has a container
/// with changes (`004d0490`).
const REFERENCE_IS_SCRIPTED: u32 = 0x0056_56d0;
/// Marks the reference's persistent cell (`ExtraDataList::SetPersistentCell`,
/// Xbox PDB): `(extra list, cell)`.
const EXTRA_LIST_SET_PERSISTENT_CELL: u32 = 0x0041_d390;
/// `TESWorldSpace::AddToPersistentRefData` (Xbox PDB).
const WORLD_SPACE_ADD_TO_PERSISTENT_REF_DATA: u32 = 0x0058_7d10;
/// `(reference, flag)`: when the reference has a light in its extra data (the
/// spell effect light when `flag` is set), registers it with the shadow scene
/// node.
const REFERENCE_ADD_LIGHT_TO_SCENE: u32 = 0x0057_28c0;

/// `__RTDynamicCast(object, 0, from, to, 0)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `ftol`: truncates the `ST0` argument (passed as an `f64`).
const FTOL: u32 = 0x00ec_62c0;
/// `sqrtf`-style square root of a `float` argument.
const SQUARE_ROOT: u32 = 0x0040_19b0;
/// `RandomFloat(low, high)`.
const RANDOM_FLOAT: u32 = 0x0047_6b70;
/// `printf`-style diagnostic output.
const DEBUG_PRINT: u32 = 0x005b_5e40;
/// Heap allocation of the memory manager and of the scene graph.
const ALLOCATE: u32 = 0x0040_1000;
const NODE_ALLOCATE: u32 = 0x00aa_13e0;

/// The `TES` singleton pointer (`TES::IsCellLoaded` and
/// `TES::GetCellPriority` are methods of it) and the player reference.
const TES_POINTER: u32 = 0x011d_ea10;
const PLAYER_POINTER: u32 = 0x011d_ea3c;
/// The save-game object, with its `00 return 0` test (`0047c850`).
const SAVE_GAME_POINTER: u32 = 0x011d_e45c;
/// The object at `011c3f2c`: its byte at `+0x61A` is read by `004516b0`, and
/// `00461c20` finds a cell by its grid coordinates.
const DATA_HANDLER_POINTER: u32 = 0x011c_3f2c;
/// The model loader whose `QueueReference` queues a reference.
const MODEL_LOADER_POINTER: u32 = 0x011c_3b3c;
/// The process lists.
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// Another global object, handed to `00526f20` for a reference of a cell in
/// state 6.
const STATE_6_OBJECT_POINTER: u32 = 0x011c_95c8;
/// The two pointer lists `AddReference` keeps: references with `iFormFlags &
/// 0x10000000` and with `& 0x20000000` in non-interior cells.
const KIND_LIST_A: u32 = 0x011c_a13c;
const KIND_LIST_B: u32 = 0x011c_a144;

/// `TESObjectCELL` virtual slots used here.
const CELL_SLOT_SET_LOADED: u32 = 0xc8;
/// `TESObjectREFR` virtual slots used here.
const REFERENCE_SLOT_IS_ACTOR: u32 = 0x100;
const REFERENCE_SLOT_GET_3D: u32 = 0x1d0;
const REFERENCE_SLOT_ADD_TO_CELL: u32 = 0x228;

/// `RTTI` type descriptors: `TESForm` (the source of every cast) and the
/// type a reference's 3D is tested against.
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_REFERENCE: u32 = 0x0118_41cc;
const RTTI_REFERENCE_CAST_TARGET: u32 = 0x0118_4920;

// --- Extra data of the cell and of references ------------------------------

/// `ExtraDataList::GetWaterType` (Xbox PDB).
const EXTRA_LIST_GET_WATER_TYPE: u32 = 0x0041_e130;
/// Extra data entry type `0x59`, field `+0xC`: read and write.
const EXTRA_LIST_GET_CELL_FORM: u32 = 0x0041_c360;
const EXTRA_LIST_SET_CELL_FORM: u32 = 0x0041_c290;
/// Extra data entry type 10 (canopy shadow mask): `(list, texture_out,
/// rect_out)` returns the entry's field `+0xC`; `ExtraDataList::
/// SetCanopyShadowMask` (Xbox PDB) takes `(list, flag, texture, rect_out)`.
const EXTRA_LIST_GET_CANOPY_MASK: u32 = 0x0041_c580;
const EXTRA_LIST_SET_CANOPY_MASK: u32 = 0x0041_c490;
/// Entry type `0x63`, field `+0xC`: the room key of a reference.
const EXTRA_LIST_GET_ROOM_KEY: u32 = 0x0042_1e10;
/// `ExtraDataList::GetRoomIsMaster`, `GetRoom`, `SetRoom`, `GetMasterRoom`,
/// `GetPrimitive`, `SetPortal` (Xbox PDB names) and the portal, link,
/// occlusion plane, merchant container and container changes accessors.
const EXTRA_LIST_GET_ROOM_IS_MASTER: u32 = 0x0042_0810;
const EXTRA_LIST_GET_ROOM: u32 = 0x0042_0ed0;
const EXTRA_LIST_SET_ROOM: u32 = 0x0042_0f00;
const EXTRA_LIST_GET_MASTER_ROOM: u32 = 0x0042_08b0;
const EXTRA_LIST_GET_PRIMITIVE: u32 = 0x0041_fbe0;
const EXTRA_LIST_GET_PORTAL: u32 = 0x0042_0dd0;
const EXTRA_LIST_SET_PORTAL: u32 = 0x0042_0e00;
const EXTRA_LIST_GET_LINKS: u32 = 0x0042_0410;
const EXTRA_LIST_GET_LINK_LIST: u32 = 0x0042_07b0;
const EXTRA_LIST_GET_LINKED_KEY: u32 = 0x0042_0bc0;
const EXTRA_LIST_GET_OCCLUSION_PLANE: u32 = 0x0042_2120;
const EXTRA_LIST_SET_OCCLUSION_PLANE: u32 = 0x0042_2150;
const EXTRA_LIST_GET_MERCHANT_CONTAINER: u32 = 0x0042_1400;
const EXTRA_LIST_GET_CONTAINER_CHANGES: u32 = 0x0041_8520;
const EXTRA_LIST_GET_COUNT: u32 = 0x0041_8770;
const EXTRA_LIST_SET_COUNT: u32 = 0x0041_9ad0;
const EXTRA_LIST_REMOVE_OWNERSHIP: u32 = 0x0041_aed0;
const EXTRA_LIST_CONSTRUCT: u32 = 0x0041_0360;
const EXTRA_LIST_DUPLICATE_FOR_CONTAINER: u32 = 0x0041_2380;
/// Sets the object the extra data list's entry type `0x20` points at.
const EXTRA_LIST_SET_REFERENCE: u32 = 0x0041_8550;

// --- Worlds, the TES object and the data handler ---------------------------

/// `TESWorldSpace` readers: the extra-data-0x59 form, the water type, the
/// terrain manager and the persistent cell (the word at `+0x34`).
const WORLD_SPACE_GET_CELL_FORM: u32 = 0x0058_6020;
const WORLD_SPACE_GET_WATER_TYPE: u32 = 0x0058_6070;
const WORLD_SPACE_GET_TERRAIN_MANAGER: u32 = 0x0058_6170;
const WORLD_SPACE_PERSISTENT_CELL: u32 = 0x005f_36f0;
/// The world space's name string lives at this offset.
const WORLD_SPACE_NAME_OFFSET: u32 = 0xd4;
/// `BGSTerrainManager::HideTree` (Xbox PDB name), `(manager, reference, 1)`;
/// and the setter of the byte at `+0x28`.
const TERRAIN_MANAGER_HIDE_TREE: u32 = 0x006f_cfa0;
const TERRAIN_MANAGER_SET_FLAG_28: u32 = 0x0092_9260;
/// `TES::IsCellLoaded(cell, 0)` and `TES::GetCellPriority(cell, 0)`
/// (Xbox PDB), `TES` being the object at [`TES_POINTER`].
const TES_IS_CELL_LOADED: u32 = 0x0045_11e0;
const TES_GET_CELL_PRIORITY: u32 = 0x0045_8be0;
/// Increments the word at `+0xB8`, and the water system pointer (`+0x64`).
const INCREMENT_FIELD_B8: u32 = 0x0045_ce40;
const TES_WATER_SYSTEM: u32 = 0x0070_ec90;
/// `TESWaterSystem::AddPlaceableWater` (Xbox PDB).
const WATER_SYSTEM_ADD_PLACEABLE_WATER: u32 = 0x004e_46b0;
/// The data handler: finds the exterior cell at grid `(x, y)` in a world
/// space `(this, x, y, world, 0)`; its loading byte (`+0x61A`).
const DATA_HANDLER_FIND_CELL: u32 = 0x0046_1c20;
const DATA_HANDLER_LOADING_FLAG: u32 = 0x0045_16b0;
/// The game loader's pointer and its flag test (`+0x244` bit 2).
const GAME_LOADER_POINTER: u32 = 0x011d_df38;
const GAME_LOADER_FLAG_244_2: u32 = 0x0042_ce10;
/// `QueueReference(reference, priority, 0)` of the model loader.
const MODEL_LOADER_QUEUE_REFERENCE: u32 = 0x0044_4850;
/// `ProcessLists` methods: temp change list, and the two reference-list
/// handlers.
const PROCESS_LISTS_ADD_ACTOR_TO_TEMP_CHANGE_LIST: u32 = 0x0096_e870;
const PROCESS_LISTS_HANDLE_CELL_REFERENCES: u32 = 0x0096_e150;
const PROCESS_LISTS_REMOVE_CELL_REFERENCES: u32 = 0x0096_dcb0;
/// The save-game object's test (`0047c850`, which always answers no).
const SAVE_GAME_FLAG: u32 = 0x0047_c850;
/// `008c7aa0`: when it answers yes the callers queue their scene graph work
/// (`fn_0054abd0`) instead of doing it at once.
const ATTACHES_ARE_QUEUED: u32 = 0x008c_7aa0;
/// `PlayerCharacter::IsSleepingorResting` (Xbox PDB), and the 3D getters:
/// `(player, first_person)` and the no-argument one.
const PLAYER_IS_SLEEPING_OR_RESTING: u32 = 0x0094_df60;
const PLAYER_GET_CURRENT_3D: u32 = 0x0095_0bb0;
const PLAYER_GET_CURRENT_3D_FIRST: u32 = 0x0095_0be0;
/// A test on the object `00891170` points at (its virtual slot `0x154`).
const GLOBAL_VIRTUAL_TEST: u32 = 0x0044_4ed0;
/// Task queue: the object and `QueueSet3DNull(queue, reference)`; the task
/// manager at [`TASK_MANAGER_POINTER`] takes a task in its virtual slot 0x48.
const TASK_QUEUE_GETTER: u32 = 0x0045_37b0;
const TASK_QUEUE_SET_3D_NULL: u32 = 0x0087_b400;
const TASK_MANAGER_POINTER: u32 = 0x0120_2d98;
const TASK_MANAGER_SLOT_QUEUE: u32 = 0x48;
/// `Interface::GetLoadingMenuVisible` (Xbox PDB).
const LOADING_MENU_VISIBLE: u32 = 0x0070_5e80;
/// `Actor::GetEssential` and `Actor::Kill` (Xbox PDB names of the callees).
const ACTOR_IS_ESSENTIAL: u32 = 0x0087_f3d0;
const ACTOR_KILL: u32 = 0x0089_d900;

// --- References, forms and cells --------------------------------------------

/// Reference accessors and tests.
const REFERENCE_GET_REF_PERSISTS: u32 = 0x0056_53d0;
const REFERENCE_GET_MULTI_BOUND: u32 = 0x0056_9920;
const REFERENCE_GET_MULTI_BOUND_ROOM: u32 = 0x0056_99b0;
const REFERENCE_GET_ORIENTATION: u32 = 0x0056_fa00;
const REFERENCE_GET_OWNER: u32 = 0x0056_7790;
const REFERENCE_IS_AN_OWNER: u32 = 0x0057_85e0;
const REFERENCE_NOTE_ROOM: u32 = 0x0056_9ae0;
const REFERENCE_POSITION_ADDRESS: u32 = 0x0043_0830;
const REFERENCE_USES_EXTERNAL_EMITTANCE: u32 = 0x0056_94c0;
const REFERENCE_FADES_OUT: u32 = 0x0056_4f00;
const REFERENCE_BLOCKED: u32 = 0x0056_8e50;
const REFERENCE_CHECKS_ROOM_DATA: u32 = 0x0056_95a0;
const REFERENCE_FITS_ROOM_DATA: u32 = 0x0056_9620;
const REFERENCE_GET_LOCK: u32 = 0x0056_9160;
const REFERENCE_UNLOCK: u32 = 0x0056_92a0;
const HAS_TIME_CONTROLLERS: u32 = 0x0056_5580;
const FIND_REFERENCE_FOR_3D: u32 = 0x0056_f930;
/// Reference virtual slots: `+0x1c4` (unload 3D), `+0x1cc` (set 3D, two
/// zero arguments), `+0x1f4` (the position, three floats), `+0x224`,
/// `+0x240` / `+0x24c` (state 6 or 5 / other cells).
const REFERENCE_SLOT_UNLOAD_3D: u32 = 0x1c4;
const REFERENCE_SLOT_SET_3D: u32 = 0x1cc;
const REFERENCE_SLOT_POSITION: u32 = 0x1f4;
const REFERENCE_SLOT_224: u32 = 0x224;
const REFERENCE_SLOT_STATE_6_OR_5: u32 = 0x240;
const REFERENCE_SLOT_OTHER_STATE: u32 = 0x24c;
/// Actor virtual slots called by `SaveGameTest`.
const ACTOR_SLOT_22C: u32 = 0x22c;
const ACTOR_SLOT_424: u32 = 0x424;
const ACTOR_SLOT_460: u32 = 0x460;
/// `TESObjectCELL` virtual slot `+0x130`: the cell's name.
const CELL_SLOT_GET_NAME: u32 = 0x130;
/// `cCellFlags & 2`.
const CELL_FLAG_2: u32 = 0x0045_18e0;
/// The cell's portal graph (`spPortalGraph`, `+0xD4`), and the version that
/// creates it when missing.
const CELL_PORTAL_GRAPH: u32 = 0x009d_9f20;
const CELL_PORTAL_GRAPH_CREATE: u32 = 0x0054_5b30;
/// The water object of the cell (interior or exterior kind).
const CELL_WATER_OBJECT: u32 = 0x0045_43c0;
/// The node the cell uses for a reference that fits `target`:
/// `(parent_cell, target)`.
const CELL_NODE_FOR_TARGET: u32 = 0x0054_5960;
/// Starts the fade-in; tests `cCellState != 0`.
const CELL_START_FADE_IN: u32 = 0x0055_7aa0;
const CELL_STATE_NONZERO: u32 = 0x0055_7d10;
/// The state-6 handler `AddReference` gives each reference.
const STATE_6_ADD_REFERENCE: u32 = 0x0052_6f20;
/// The water type of a water form: the replacement it names (word `+0x80`).
const WATER_REPLACEMENT: u32 = 0x004f_b070;
/// The text accessors: a string object's characters, and a setting's first
/// character / text.
const STRING_GET_TEXT: u32 = 0x0040_8da0;
const SETTING_TEXT: u32 = 0x0040_8d60;
const SETTING_FIRST_CHAR: u32 = 0x0045_4af0;
/// A mask test `word at +0xC & argument`.
const MASK_TEST: u32 = 0x0044_8a60;
/// The four `+0xA8` flag tests `fn_00549630` combines.
const FLAG_A8_8: u32 = 0x0050_d1d0;
const FLAG_A8_40: u32 = 0x0050_d1f0;
const FLAG_A8_80: u32 = 0x0050_d210;
const FLAG_A8_100: u32 = 0x0050_d230;
/// `ItemChange::ItemChange(this, form, 0)` (Xbox PDB), stores a count at
/// `+4`, and `InventoryChanges::` add / merge.
const ITEM_CHANGE_CONSTRUCT: u32 = 0x004b_c550;
const ITEM_CHANGE_SET_COUNT_DELTA: u32 = 0x006e_cd40;
const CHANGES_ADD: u32 = 0x004c_3380;
const CONTAINER_CHANGES_MERGE: u32 = 0x004d_26d0;
const CONTAINER_CHANGES_UPDATE: u32 = 0x004c_e380;

// --- Scene graph -----------------------------------------------------------

/// The 3D's parent (`+0x18`), world bound and bound center.
const NODE_PARENT: u32 = 0x0096_11e0;
const NODE_GET_WORLD_BOUND: u32 = 0x0043_d450;
const BOUND_CENTER_ADDRESS: u32 = 0x0068_15c0;
/// `NiAVObject::UpdateProperties` (Xbox PDB), `SetName`, `GetExtraData`.
const NODE_UPDATE_PROPERTIES: u32 = 0x00a5_a040;
const OBJECT_SET_NAME: u32 = 0x00a5_b950;
const OBJECT_GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
/// Key of the extra data `AttachReference3D` looks for.
const EXTRA_DATA_KEY: u32 = 0x0044_8a80;
/// `NiNode` virtual slots: `+0x10` the fade node, `+0x14`, `+0xDC`
/// `AttachChild(child, replace)`, `+0xE8` detach.
const NODE_SLOT_GET_FADE_NODE: u32 = 0x10;
const NODE_SLOT_14: u32 = 0x14;
const NODE_SLOT_ATTACH_CHILD: u32 = 0xdc;
const NODE_SLOT_DETACH_CHILD: u32 = 0xe8;
/// The first word of an item (the node a slot holds).
const ITEM_SLOT_GET: u32 = 0x0045_c4c0;
/// `BSFadeNode` calls: kind setter, range, tree flag, fade check.
const FADE_NODE_SET_KIND: u32 = 0x0070_5b10;
const FADE_NODE_SET_RANGE: u32 = 0x00b4_dfd0;
const FADE_NODE_SET_TREE_FLAG: u32 = 0x0049_ee60;
const FADE_NODE_CHECK_FADE_RADIUS: u32 = 0x00b4_eb10;
const FADE_NODE_PREPARE: u32 = 0x0047_6ab0;
const SET_PROPERTY_FADE_ALPHA: u32 = 0x00b6_bb30;
/// `ShadowSceneNode::UpdateObjectLighting` and the table lookup that finds
/// the scene node (`global[0x011f91c8 + 4 * i]`).
const SHADOW_SCENE_NODE_GETTER: u32 = 0x0045_0b80;
const SHADOW_SCENE_NODE_UPDATE_LIGHTING: u32 = 0x00b5_d9f0;
/// Portal graph: nodes, planes, portals, rooms.
const PORTAL_GRAPH_LIST: u32 = 0x007d_6bb0;
const PORTAL_GRAPH_NODE: u32 = 0x0054_6930;
const PORTAL_GRAPH_ADD_NODE: u32 = 0x00c5_b370;
const PORTAL_GRAPH_REMOVE_NODE: u32 = 0x00c5_b1c0;
const PORTAL_GRAPH_ADD_PLANE: u32 = 0x00c5_ae70;
const PORTAL_GRAPH_ADD_PORTAL: u32 = 0x00c5_b140;
const PORTAL_GRAPH_ADD_ROOM: u32 = 0x00c5_aef0;
/// The word at `+8` of a list is zero.
const LIST_COUNT_IS_ZERO: u32 = 0x0076_b610;
/// Address `+0xB4` of a room node (its children container); the container's
/// count (word `+8`) and the iteration step that fills the cursor.
const ADDRESS_PLUS_B4: u32 = 0x006a_b360;
const ARRAY_COUNT: u32 = 0x0044_ddc0;
const CHILD_ARRAY_NEXT: u32 = 0x0057_cbe0;
/// `BSOcclusionPlane::TestIntersection(item, bound)` (Xbox PDB).
const ROOM_TEST_INTERSECTION: u32 = 0x00c3_4890;
/// Room node virtual slots: children (+0x104), contains point (+0x108),
/// contains bound (+0x110).
const ROOM_NODE_SLOT_CHILDREN: u32 = 0x104;
const ROOM_NODE_SLOT_CONTAINS_POINT: u32 = 0x108;
const ROOM_NODE_SLOT_CONTAINS_BOUND: u32 = 0x110;
const ROOM_CHILDREN_ADD_PLANE: u32 = 0x00c3_9a80;
/// Constructors: `BSMultiBoundRoom`, `BSMultiBound`, `BSPortalSharedNode`,
/// `BSOcclusionPlane` (Xbox PDB names).
const MULTI_BOUND_ROOM_CONSTRUCT: u32 = 0x00c3_9640;
const MULTI_BOUND_CONSTRUCT: u32 = 0x00c3_5e00;
const PORTAL_SHARED_NODE_CONSTRUCT: u32 = 0x00c4_f460;
const OCCLUSION_PLANE_CONSTRUCT: u32 = 0x00c3_35d0;
/// Slot setters: the room's multibound (`+0xAC`) and the multibound's shape
/// (`+0xC`); readers of the same slots.
const ROOM_SET_MULTI_BOUND: u32 = 0x0043_9920;
const MULTI_BOUND_SET_SHAPE: u32 = 0x004a_ddc0;
const SLOT_AT_AC_GET: u32 = 0x0066_29f0;
const SLOT_AT_C_GET: u32 = 0x0043_b230;
/// Virtual slots: the multibound's position setter (`+0x90`), the shape's
/// (`+0xB8`), the primitive's shape makers (`+0x14` with a position, `+0x1C`).
const MULTI_BOUND_SLOT_SET_POSITION: u32 = 0x90;
const SHAPE_SLOT_SET_POSITION: u32 = 0xb8;
const PRIMITIVE_SLOT_MAKE_SHAPE: u32 = 0x14;
const PRIMITIVE_SLOT_SHAPE: u32 = 0x1c;
/// Shape / plane setters: position, orientation matrix, plane size, the
/// primitive's dimensions and a two-float vector constructor.
const SHAPE_SET_POSITION: u32 = 0x0041_69f0;
const SHAPE_SET_ORIENTATION: u32 = 0x0041_6a70;
const PLANE_SET_SIZE: u32 = 0x0041_6a30;
const PLANE_CENTER_ADDRESS: u32 = 0x0041_3f40;
const PRIMITIVE_GET_DIMENSIONS: u32 = 0x0041_3fc0;
const VECTOR2_CONSTRUCT: u32 = 0x0045_2dc0;
/// Portal connections: the two room setters and the room's portal list.
const PORTAL_SET_FIRST_ROOM: u32 = 0x0097_07f0;
const PORTAL_SET_SECOND_ROOM: u32 = 0x0042_0ba0;
const PORTAL_TARGET_ADD: u32 = 0x004e_d8c0;
/// A smart pointer slot copy constructor.
const SLOT_COPY_CONSTRUCT: u32 = 0x0055_9a40;
/// The global slot at `011deb7c`.
const GLOBAL_SLOT_GET: u32 = 0x0045_c670;
/// Sets the byte at `+8`.
const SET_BYTE_8: u32 = 0x0093_7090;
/// `NiFixedString` constructor (`ecx = text`) and destructor.
const FIXED_STRING_CONSTRUCT: u32 = 0x0043_8170;
const FIXED_STRING_DESTRUCT: u32 = 0x0043_81b0;
/// The word at `+0x24` of an object (the map names this
/// `D3DTexture_LockRect` because the code is folded).
const OBJECT_FIELD_24: u32 = 0x0059_bb30;
/// Renderer: the object `0043c4b0` returns, whether a mask can be made
/// (`004bc3f0`), the rendered texture constructor and the format record.
const RENDERER_GLOBAL: u32 = 0x0043_c4b0;
const RENDERER_CAN_CREATE_MASK: u32 = 0x004b_c3f0;
const RENDERED_TEXTURE_CREATE: u32 = 0x00a7_fc00;
/// Virtual slots of the texture (+0x98 rows) and its Direct3D owner
/// (+0x9C surface, whose +0x4C is `LockRect`).
const TEXTURE_SLOT_HEIGHT: u32 = 0x98;
const SURFACE_OWNER_SLOT_SURFACE: u32 = 0x9c;
const SURFACE_SLOT_LOCK_RECT: u32 = 0x4c;

// --- Queues and maps --------------------------------------------------------

const LOADED_DATA_ROOM_MAP: u32 = 0x3c;
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
const MAP_GET_AT: u32 = 0x006c_62d0;
const MAP_GET_NEXT: u32 = 0x0055_9120;
const QUEUED_ATTACH_MAP: u32 = 0x011c_a0f4;
const QUEUED_ATTACH_MAP_CLEAR: u32 = 0x0043_8af0;
const QUEUED_ATTACH_MAP_GET_NEXT: u32 = 0x0055_93a0;
const QUEUED_ATTACH_MAP_REMOVE_AT: u32 = 0x0040_5430;
const QUEUED_ATTACH_MAP_SET_AT: u32 = 0x0055_9260;
const QUEUED_ROOM_MAP_SET_AT: u32 = 0x0084_4700;
const ROOM_TASK_MAP: u32 = 0x011c_a0e0;
/// The scrap map of visited keys: constructor `(0x25, 0)`, set, get, release.
const SCRAP_MAP_CONSTRUCT: u32 = 0x0055_95e0;
const SCRAP_MAP_SET: u32 = 0x0055_94e0;
const SCRAP_MAP_GET: u32 = 0x0057_c850;
const SCRAP_MAP_RELEASE: u32 = 0x0055_96e0;
/// The queue's semaphore: wrapper object, init guard word, destructor
/// registered with `atexit`, wait and release; Windows `CreateSemaphoreA`
/// (import slot).
const QUEUE_SEMAPHORE: u32 = 0x011c_a1d4;
const QUEUE_SEMAPHORE_GUARD: u32 = 0x011c_a1e0;
const QUEUE_SEMAPHORE_DESTRUCTOR: u32 = 0x00fc_af50;
const SEMAPHORE_WAIT: u32 = 0x0044_24e0;
const SEMAPHORE_RELEASE: u32 = 0x0044_2550;
const CREATE_SEMAPHORE: u32 = 0x00fd_f0b4;
const CRT_ATEXIT: u32 = 0x00ec_658f;
/// Destructor of a queue entry.
const ATTACH_ENTRY_DESTRUCT: u32 = 0x0062_08d0;
/// `CheckWithinMultiBoundTask`: vtable, base constructor `(this, 4)` and
/// destructor.
const CHECK_WITHIN_MULTI_BOUND_TASK_VTABLE: u32 = 0x0102_ee48;
const TASK_BASE_CONSTRUCT: u32 = 0x0044_0540;
const TASK_BASE_DESTRUCT: u32 = 0x0044_0640;
/// `RandomInt(low, high)`-style pick used by `fn_0054afb0`.
const RANDOM_BELOW: u32 = 0x0094_4460;
/// `ObjectFilter` calls of `fn_0054afb0`.
const FILTER_OBJECT_USABLE: u32 = 0x0051_94d0;
const FILTER_OBJECT_ACCEPTS: u32 = 0x0051_9500;

// --- Data -------------------------------------------------------------------

/// Marker base forms compared with a reference's base form.
const MARKER_FORM_230: u32 = 0x011c_a230;
const ROOM_MARKER_FORM_238: u32 = 0x011c_a238;
const PORTAL_MARKER_FORM_23C: u32 = 0x011c_a23c;
const OCCLUSION_MARKER_FORM_234: u32 = 0x011c_a234;
const MARKER_FORM_240: u32 = 0x011c_a240;
/// Form type numbers compared by `AttachReference3D` and others (what the
/// kinds are is not asserted here).
const FORM_TYPE_0D: u32 = 0x0d;
const FORM_TYPE_0E: u32 = 0x0e;
const FORM_TYPE_1C: u32 = 0x1c;
const FORM_TYPE_1E: u32 = 0x1e;
const FORM_TYPE_23: u32 = 0x23;
const FORM_TYPE_25: u32 = 0x25;
/// Settings and constants.
const FEATURE_SETTING: u32 = 0x011c_a118;
const WATER_WARNING_SETTING: u32 = 0x011c_7adc;
const DEFAULT_WATER_TYPE: u32 = 0x011c_a53c;
const WATERLESS_CELL_MESSAGE: u32 = 0x0102_ede0;
const PORTAL_SHARED_GEOMETRY_NAME: u32 = 0x0102_edc4;
const CANOPY_MASK_NAME: u32 = 0x0102_edac;
/// The mask texture's creation parameters: format record (`0x32` in the
/// word at `+4`) and the two busy flags around the creation.
const MASK_FORMAT: u32 = 0x011a_b3cc;
const MASK_FORMAT_WORD: u32 = 0x011a_b3c8;
const MASK_FORMAT_FLAG: u32 = 0x011a_b3c4;
const RENDER_TARGET_BUSY_FLAG: u32 = 0x011f_4ac0;
/// `FLT_MAX`, the fade-in start value, and the constant `SaveGameTest` hands
/// to the actor's slot `0x460`.
const FLOAT_MAX: u32 = 0x0101_6970;
const LOD_FADE_IN_START: u32 = 0x0118_b694;
const SAVE_TEST_CONSTANT: u32 = 0x0101_7b78;
/// `00555c20`, called at the end of each `SaveGameTest` step.
const SAVE_TEST_FINISH: u32 = 0x0055_5c20;
/// RTTI type descriptors: cast targets and the filter pair of
/// `fn_0054afb0`.
const RTTI_CELL: u32 = 0x0118_3fb4;
const RTTI_WORLD_SPACE: u32 = 0x0118_3fd0;
const RTTI_CELL_FORM_TARGET: u32 = 0x0118_4154;
const RTTI_FORM_FILTER_FROM: u32 = 0x0118_3108;
const RTTI_FORM_FILTER_TO: u32 = 0x0118_6530;
const RTTI_ACTOR_LIKE: u32 = 0x0118_46d4;
/// Frees a heap block (`ret`, one argument).
const DEALLOCATE: u32 = 0x0040_1030;
/// `fn_00547ad0`: the flag byte that disables canopy shadows, the getter of
/// the tree update flag (`0119b8c8`) and the tree manager call that follows
/// it, the coordinate record constructor, `TESObjectLAND::GetCoordData`
/// (Xbox PDB name), the renderer lock and unlock, and the upper bound of the
/// random shade.
const CANOPY_DISABLED_FLAG: u32 = 0x011c_a090;
const TREE_UPDATE_FLAG: u32 = 0x0045_2480;
const TREE_MANAGER_REFRESH: u32 = 0x0066_5610;
const COORDINATE_DATA_CONSTRUCT: u32 = 0x0053_a510;
const LAND_GET_COORD_DATA: u32 = 0x0053_b550;
const RENDERER_LOCK: u32 = 0x004a_0370;
const RENDERER_UNLOCK: u32 = 0x004a_03c0;
const CANOPY_RANDOM_RANGE: u32 = 0x0102_edc0;
/// `64.0`, `255.0` and `60.0` as `double`s: the pixels per cell of the
/// mask, the brightest shade and the fall-off over the radius.
const MASK_PIXEL_DIVISOR: u32 = 0x0102_40c0;
const SHADE_BASE: u32 = 0x0101_e568;
const SHADE_FALLOFF: u32 = 0x0101_2638;

// --- Second session: callees of 0054b750 .. 0054ee20 ------------------------

/// A cell method that `fn_0054b750` runs first; its body works on the
/// portal graph slot (`spPortalGraph`, `+0xD4`). The map gives it no name.
const CELL_PREPARE_UNLOAD: u32 = 0x0054_5c10;
/// Called on a fade node (`this`, 0) after its two floats are cleared.
const FADE_NODE_RESET: u32 = 0x0047_6ae0;
/// Base forms compared by the two collectors of map markers.
const MARKER_FORM_224: u32 = 0x011c_a224;
const MARKER_FORM_228: u32 = 0x011c_a228;
/// `TESObjectREFR::GetMapMarkerData` (Xbox PDB) and the sibling accessor the
/// second collector uses.
const REFERENCE_GET_MAP_MARKER_DATA: u32 = 0x0056_9060;
const REFERENCE_GET_MARKER_DATA_228: u32 = 0x0056_9080;
/// A test on the object at [`DATA_HANDLER_POINTER`] (an `ExtraDataList`
/// source file the map files it under), used as `ECX = [011c3f2c]`.
const DATA_HANDLER_FLAG_4226E0: u32 = 0x0042_26e0;
/// Reference tests and the update that `fn_0054b950` and `fn_0054ba00` run on
/// every reference that passes them; the map names none of the three.
const REFERENCE_TEST_57A2F0: u32 = 0x0057_a2f0;
const REFERENCE_TEST_57A370: u32 = 0x0057_a370;
const REFERENCE_UPDATE_579AC0: u32 = 0x0057_9ac0;
/// `TESObjectREFR::RemoveLight` (Xbox PDB), `(reference, 0)`.
const REFERENCE_REMOVE_LIGHT: u32 = 0x0057_29e0;
/// The extra data entry of type `0x29` of a list (field `+0xC`), and the call
/// `fn_0054baf0` makes with it on the base form: `(base form, entry, 0)`.
const EXTRA_LIST_GET_ENTRY_29: u32 = 0x0041_8250;
const BASE_FORM_APPLY_ENTRY: u32 = 0x0050_de20;
/// Prepares a reference before `fn_0054bc10` hides its tree.
const REFERENCE_PREPARE_56F700: u32 = 0x0056_f700;
/// `TESObjectCELL::Load3D` (Xbox PDB).
const CELL_LOAD_3D: u32 = 0x0054_5cf0;
/// Run by `fn_0054bcf0` with the answer of `fn_0054df30`.
const CELL_AFTER_SCRIPTS: u32 = 0x0055_0c60;
/// The `TES` method that `fn_0054bcf0` asks about the cell (`(cell)`).
const TES_CELL_TEST_453490: u32 = 0x0045_3490;
/// The three-float vector constructor `fn_0054bcf0` uses, and the call that
/// hands the vector to the cell's 3D.
const VECTOR3_CONSTRUCT_43D410: u32 = 0x0043_d410;
const NODE_SET_LOCAL_TRANSLATE: u32 = 0x00a5_9c60;
/// `Script::InitActionList` and `Script::SetActionFlag` (Xbox PDB):
/// `(reference, extra data list)` and `(reference, extra data list, flag)`.
const SCRIPT_INIT_ACTION_LIST: u32 = 0x005a_c190;
const SCRIPT_SET_ACTION_FLAG: u32 = 0x005a_c750;
/// `ProcessLists::RemoveActorFromTempChangeList` (Xbox PDB), on
/// [`PROCESS_LISTS`].
const PROCESS_LISTS_REMOVE_ACTOR_FROM_TEMP_CHANGE_LIST: u32 = 0x0096_f400;
/// A `TES` test of a form that `fn_0054bcf0` and `fn_0054cd20` ask of a base
/// form of type `0x23`.
const FORM_TEST_452440: u32 = 0x0045_2440;
/// Run by `fn_0054bcf0` when its flag is set.
const CELL_AFTER_ATTACH: u32 = 0x0055_35f0;
/// Writes the byte `cCellState` (`+0x26`) from its argument.
const CELL_SET_STATE: u32 = 0x0045_12a0;
/// Virtual slots of an actor (`+0x240`) and of a reference (`+0x260`).
const ACTOR_SLOT_240: u32 = 0x240;
const REFERENCE_SLOT_260: u32 = 0x260;

/// Vector helpers: the three-float constructor `(this, x, y, z)`, `this + other`
/// and `this - other` written to a result buffer `(this, result, other)`, the
/// division of `this` by a float `(this, result, divisor)`, the smaller and
/// the larger of two floats, the absolute value (through `fabs`), and the
/// multiplication of a float by the float at `011c3d18`.
const VECTOR3_CONSTRUCT: u32 = 0x0041_6870;
const VECTOR_ADD: u32 = 0x0043_9e90;
const VECTOR_SUBTRACT: u32 = 0x0043_9ef0;
const VECTOR_DIVIDE: u32 = 0x0053_d280;
const FLOAT_SMALLER: u32 = 0x0040_ebd0;
const FLOAT_LARGER: u32 = 0x0040_4010;
const FLOAT_ABS: u32 = 0x0040_8840;
const WORLD_SCALE: u32 = 0x0045_87d0;
/// `ret this`: the default constructor the compiler calls on a vector.
const TRIVIAL_CONSTRUCT: u32 = 0x0068_15c0;
/// The three zero floats the code uses as the default position.
const DEFAULT_VECTOR: u32 = 0x011f_426c;
/// Floats: `2.0`, the limit `fn_0054c030` scales (`3250.0`), the start value
/// of its running maximum (`FLT_MIN`), and the double `30000.0` it compares
/// positions with.
const TWO: u32 = 0x0101_62c0;
const EXTENT_LIMIT: u32 = 0x0102_efc0;
const SMALLEST_NORMAL_FLOAT: u32 = 0x0102_efc4;
const POSITION_LIMIT: u32 = 0x0102_efb8;
/// Messages of `fn_0054c030`: a reference at an invalid position that is put
/// back at its start position, one that is moved to a safe place, and the
/// interior that is too large for the Havok world; and the one of
/// `AssignPersistentRefsToCellsInWorld`.
const MESSAGE_RETURN_TO_START: u32 = 0x0102_ef68;
const MESSAGE_MOVE_TO_SAFE_PLACE: u32 = 0x0102_ef18;
const MESSAGE_INTERIOR_TOO_LARGE: u32 = 0x0102_ee68;
const MESSAGE_CELL_NOT_FOUND: u32 = 0x0102_efc8;
/// `BaseExtraList::GetExtraData(list, type)`.
const EXTRA_LIST_GET_EXTRA_DATA: u32 = 0x0041_0220;
/// The call that puts a reference at a position `(reference, position)`
/// (the decompiler names it `SetLocationOnReference`).
const REFERENCE_SET_LOCATION: u32 = 0x0057_5830;
/// `TESObjectREFR::RunScript` and `UpdateChildActivates` (Xbox PDB),
/// `Interface::IsInMenuMode` (Xbox PDB, no `this`).
const REFERENCE_RUN_SCRIPT: u32 = 0x0056_5870;
const REFERENCE_UPDATE_CHILD_ACTIVATES: u32 = 0x0056_a380;
const INTERFACE_IS_IN_MENU_MODE: u32 = 0x0070_2360;
/// List calls: removes the first node (the head takes the next item), and
/// removes an item `(list, &item)`.
const LIST_POP_FRONT: u32 = 0x0063_f7b0;
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
/// Rounds a float to an `int` (`fistp`).
const FLOAT_TO_INT: u32 = 0x0040_6d90;
/// `TESWorldSpace::GetCellFromCellCoord` (Xbox PDB): `(world, x, y)`.
const WORLD_SPACE_GET_CELL_FROM_COORD: u32 = 0x0058_75a0;
/// `TESObjectREFR::IsFurniture` (Xbox PDB name of the decompiler's callee).
const REFERENCE_IS_FURNITURE: u32 = 0x0056_8680;
/// `TESObjectREFR::GetEncounterZone` and the setter, the cell's encounter
/// zone, and the default zone (a global).
const REFERENCE_GET_ENCOUNTER_ZONE: u32 = 0x0056_7d20;
const REFERENCE_SET_ENCOUNTER_ZONE: u32 = 0x0056_7dd0;
const CELL_ENCOUNTER_ZONE: u32 = 0x0054_6c20;
const DEFAULT_ENCOUNTER_ZONE: u32 = 0x0054_6a90;
/// Removes a reference from the object at [`STATE_6_OBJECT_POINTER`].
const STATE_6_REMOVE_REFERENCE: u32 = 0x0052_70b0;
/// A test on the game loader (`ECX = [011ddf38]`).
const GAME_LOADER_FLAG_4121B0: u32 = 0x0041_21b0;
/// Loaded-data bookkeeping `RemoveReference` undoes: scripted references,
/// activating children, emittance (`TESObjectCELL::RemoveEmittanceRef`, Xbox
/// PDB) and multibound references.
const CELL_REMOVE_SCRIPTED_REF: u32 = 0x0054_5590;
const CELL_REMOVE_ACTIVATING_REF: u32 = 0x0054_5670;
const CELL_REMOVE_EMITTANCE_REF: u32 = 0x0054_5360;
const CELL_REMOVE_MULTI_BOUND_REF: u32 = 0x0054_54f0;
/// `TESWorldSpace::RemoveFromPersistentRefData` (Xbox PDB).
const WORLD_SPACE_REMOVE_FROM_PERSISTENT_REF_DATA: u32 = 0x0058_7e40;
/// `ExtraDataList::GetOcclusionPlaneLinkedRefData` (Xbox PDB): an array of
/// four references.
const EXTRA_LIST_GET_OCCLUSION_LINKED_REFS: u32 = 0x0042_0100;
/// A cell method `fn_0054cd20` runs first (the map gives no name), and the
/// one it runs on a reference whose base form passes [`FORM_TEST_452440`].
const CELL_FN_5576C0: u32 = 0x0055_76c0;
const CELL_FN_558BA0: u32 = 0x0055_8ba0;
/// `GarbageCollector::Add` (Xbox PDB), cdecl `(reference)`.
const GARBAGE_COLLECTOR_ADD: u32 = 0x0086_7f90;
/// A reference virtual slot called with 1 on a persistent reference of a
/// persistent cell (the compiler's `delete` call looks like this).
const REFERENCE_SLOT_10: u32 = 0x10;

/// The teleport data of a door reference (`00568e50`, the map gives no name)
/// and the readers of `DoorTeleportData` (Xbox PDB): world space, cell, and
/// the position (`this + 4`, the map's `GetActorPackageThatIsRunning` is a
/// folded name).
const REFERENCE_GET_TELEPORT_DATA: u32 = 0x0056_8e50;
const TELEPORT_GET_WORLD_SPACE: u32 = 0x0043_a320;
const TELEPORT_GET_CELL: u32 = 0x0043_a2b0;
const TELEPORT_POSITION: u32 = 0x0071_7e50;
/// Base forms the placement search compares with: three kinds of marker and
/// the door base form it leaves out.
const COC_MARKER_FORM_244: u32 = 0x011c_a244;
const COC_MARKER_FORM_248: u32 = 0x011c_a248;
const COC_MARKER_FORM_24C: u32 = 0x011c_a24c;
const TELEPORT_EXCLUDED_FORM_258: u32 = 0x011c_a258;
/// `(cell, &position)`: true when the position is inside the cell.
const CELL_POSITION_FITS: u32 = 0x0055_0200;
/// `TESObjectLAND::GetLandHeight`, `TESObjectCELL::GetWaterHeight`,
/// `TESObjectCELL::LoadAllTempData` (Xbox PDB).
const LAND_GET_HEIGHT: u32 = 0x0053_f180;
const CELL_GET_WATER_HEIGHT: u32 = 0x0054_71e0;
const CELL_LOAD_ALL_TEMP_DATA: u32 = 0x0055_0340;
/// The setting object `GetCOCPlacementInfo` reads (its text accessor is
/// [`SETTING_TEXT`]).
const COC_SETTING: u32 = 0x011c_a0a8;
/// The distance test of the range walks `(position, point, limit)`: the
/// callers skip a reference when it answers a value `>= 0` (read as `int`).
const DISTANCE_TEST: u32 = 0x004b_5440;
/// True when the three floats of two points are equal `(point, other)`.
const POINTS_EQUAL: u32 = 0x0043_90c0;
/// A float derived from a reference and a point `(reference, point)`: the
/// body subtracts the reference's position (`+0x30`) from the point.
const REFERENCE_DISTANCE: u32 = 0x0057_2380;
/// State of `fn_0054d4b0`: the number of nested walks, the best door world
/// space found, its position (three floats) and distance, the container of
/// visited cells (twelve bytes), and the guard word of its static
/// initialisation.
const WALK_DEPTH: u32 = 0x011c_a1e8;
const WALK_BEST_WORLD: u32 = 0x011c_a1e4;
const WALK_BEST_POSITION: u32 = 0x011c_a1ec;
const WALK_BEST_DISTANCE: u32 = 0x0118_b6f4;
const WALK_VISITED: u32 = 0x011c_a1f8;
const WALK_GUARD: u32 = 0x011c_a204;
/// The destructor registered with `atexit` for [`WALK_VISITED`].
const WALK_VISITED_DESTRUCTOR: u32 = 0x00fc_af60;
/// `FLT_MAX` as the double the comparisons use.
const FLOAT_MAX_DOUBLE: u32 = 0x0102_31b0;
/// The container of visited cells: construct, add `(container, &item)`,
/// find `(container, &item, 0)`, clear, and the destructor body.
const VISITED_CONSTRUCT: u32 = 0x0048_f200;
const VISITED_ADD: u32 = 0x0055_97d0;
const VISITED_FIND: u32 = 0x0049_c680;
const VISITED_CLEAR: u32 = 0x0055_9c30;
const VISITED_DESTRUCT: u32 = 0x0055_9810;
/// The world space's own search `(world, position, limit, point, limit,
/// callback, context)`.
const WORLD_SPACE_WALK: u32 = 0x0058_85f0;
/// `(position)` vector length squared, as `fn_0054dc00` compares it.
const VECTOR_LENGTH_SQUARED: u32 = 0x004a_7290;
/// Appends the word at `*item` to an array `(array, &item)` (the map names it
/// `BSSimpleArray<BGSBodyPart_P_1024>::AddUninitialized`, a folded name).
const ARRAY_APPEND: u32 = 0x007c_b2e0;
/// The time object and its reader, the cell's detach time and its setter
/// (`TESObjectCELL::GetDetachTime`, Xbox PDB).
const GAME_TIME_OBJECT: u32 = 0x011d_e7b8;
const GAME_TIME_NOW: u32 = 0x0086_7e30;
const CELL_GET_DETACH_TIME: u32 = 0x0054_6af0;
const CELL_SET_DETACH_TIME: u32 = 0x0054_6b10;
/// Encounter zone calls of `fn_0054df30`: the delay (`int`), the test of a
/// zone and the test of a zone against a cell.
const ENCOUNTER_ZONE_DELAY: u32 = 0x0052_6100;
const ENCOUNTER_ZONE_TEST: u32 = 0x0052_62a0;
const ENCOUNTER_ZONE_TEST_CELL: u32 = 0x0052_6120;
/// `(actor, time)`: whether the actor's time check passes.
const ACTOR_TIME_CHECK: u32 = 0x0088_1c90;
/// The data handler's test of a form id `(handler, id)` (true below
/// `0xFF000000`).
const DATA_HANDLER_ID_TEST: u32 = 0x0046_9860;
/// Reference tests of `fn_0054df30`.
const REFERENCE_TEST_577DE0: u32 = 0x0057_7de0;
const REFERENCE_FLAG_1000000: u32 = 0x0045_2370;
const REFERENCE_TEST_56AE60: u32 = 0x0056_ae60;
const REFERENCE_TEST_565450: u32 = 0x0056_5450;
/// Destruction: `BGSDestructibleObjectForm::GetDestructionForm` (Xbox PDB,
/// cdecl `(base form)`) and the call that applies it `(destruction, reference)`.
const BASE_GET_DESTRUCTION_FORM: u32 = 0x0047_5400;
const DESTRUCTION_APPLY: u32 = 0x0047_7090;
/// `ExtraDataList::GetModelSwap` (Xbox PDB) and the setter of form flag
/// `0x2000` `(form, on)`.
const EXTRA_LIST_GET_MODEL_SWAP: u32 = 0x0042_e250;
const FORM_SET_FLAG_2000: u32 = 0x0048_4610;
/// Dropped items and ash pile of a list (Xbox PDB): `GetDroppedItemList`,
/// `AddDroppedItem(list, 0)`, `RemoveDroppedItemList`, `GetAshPileRef`;
/// `TESObjectREFR::MarkAsDeleted` and `TESObjectREFR::Lock` (Xbox PDB).
const EXTRA_LIST_GET_DROPPED_ITEM_LIST: u32 = 0x0041_df90;
const EXTRA_LIST_ADD_DROPPED_ITEM: u32 = 0x0041_de40;
const EXTRA_LIST_REMOVE_DROPPED_ITEM_LIST: u32 = 0x0041_dfd0;
const EXTRA_LIST_GET_ASH_PILE_REF: u32 = 0x0041_e310;
const REFERENCE_MARK_AS_DELETED: u32 = 0x0057_2270;
const REFERENCE_LOCK: u32 = 0x0056_9250;
/// The test on container changes `fn_0054df30` makes before locking.
const CHANGES_TEST_42CDE0: u32 = 0x0042_cde0;
/// `TESForm::SetEmpty` (Xbox PDB): `(form, 0)`.
const FORM_SET_EMPTY: u32 = 0x0048_4580;
/// Sets the rotation of an actor: three floats by value.
const REFERENCE_SET_ROTATION: u32 = 0x0057_5700;
/// Actor and reference slots of `fn_0054df30`.
const REFERENCE_SLOT_160: u32 = 0x160;
const REFERENCE_SLOT_C4: u32 = 0xc4;
const REFERENCE_SLOT_208: u32 = 0x208;
const REFERENCE_SLOT_16C: u32 = 0x16c;
const REFERENCE_SLOT_170: u32 = 0x170;
const REFERENCE_SLOT_294: u32 = 0x294;
const REFERENCE_SLOT_298: u32 = 0x298;
const ACTOR_SLOT_324: u32 = 0x324;
/// `Actor` test (dead or dying) of `fn_0054df30`.
const ACTOR_TEST_87F4A0: u32 = 0x0087_f4a0;
/// Cast target pair `fn_0054df30` uses on a base form of type `0x1d`
/// (the source is [`RTTI_FORM_FILTER_FROM`]).
const RTTI_CAST_11839DC: u32 = 0x0118_39dc;

/// The local map textures: `TESObjectCELL::TakeExteriorLocalMapPicture` (Xbox
/// PDB) `(cell, &picture)` and the interior counterpart `(cell, a, b,
/// &picture)`; `BSRenderedTexture::GetTexture` (Xbox PDB); the smart pointer
/// assignment `(destination, &source)`.
const TAKE_EXTERIOR_LOCAL_MAP_PICTURE: u32 = 0x0054_ee80;
const TAKE_INTERIOR_LOCAL_MAP_PICTURE: u32 = 0x0054_f500;
const RENDERED_TEXTURE_GET_TEXTURE: u32 = 0x004b_c320;
const SMART_POINTER_ASSIGN: u32 = 0x006e_5cc0;

/// `TakeLocalMapPicture`: a scope guard `(file, line)` and its destructor;
/// `BSShaderManager::GetAccumulator` / `SetAccumulator` (Xbox PDB); the
/// culling process constructor `(this, 0)` and
/// `BSCullingProcess::~BSCullingProcess` (Xbox PDB).
const PROFILE_SCOPE_CONSTRUCT: u32 = 0x0040_4eb0;
const PROFILE_SCOPE_DESTRUCT: u32 = 0x0040_4ee0;
const SHADER_GET_ACCUMULATOR: u32 = 0x00b4_f5c0;
const SHADER_SET_ACCUMULATOR: u32 = 0x00b5_4ac0;
const CULLING_PROCESS_CONSTRUCT: u32 = 0x004a_0eb0;
const CULLING_PROCESS_DESTRUCT: u32 = 0x004a_0f60;
/// Getters of the objects the picture is drawn with: the render target
/// holder (the word of the slot at `011f9508`) and the texture manager (the
/// global at `011f91a8`), and the texture manager's creators and
/// `BSTextureManager::ReturnRenderedTexture` (Xbox PDB).
const RENDER_TARGET_HOLDER: u32 = 0x004a_0e90;
const TEXTURE_MANAGER: u32 = 0x004a_0ea0;
const TEXTURE_MANAGER_CREATE_A: u32 = 0x00b6_e110;
const TEXTURE_MANAGER_CREATE_B: u32 = 0x00b6_d370;
const TEXTURE_MANAGER_RETURN: u32 = 0x00b6_da10;
/// A four-float constructor `(this, r, g, b, a)`; the holder's virtual slots
/// that read (`+0xB4`) and write (`+0xAC`) its colour.
const COLOR_CONSTRUCT: u32 = 0x0041_4430;
const HOLDER_SLOT_GET_COLOR: u32 = 0xb4;
const HOLDER_SLOT_SET_COLOR: u32 = 0xac;
/// The byte at `+0x130` of the shadow scene node (the object
/// [`SHADOW_SCENE_NODE_GETTER`] returns): read and write.
const SHADOW_SCENE_NODE_GET_FLAG: u32 = 0x005f_9ad0;
const SHADOW_SCENE_NODE_SET_FLAG: u32 = 0x005f_9af0;
/// The water code's flag (no `this`), the object it keeps in the slot at
/// `011c7c28`, and that object's flag reader and setter.
const WATER_RENDER_FLAG: u32 = 0x004e_9510;
const WATER_OBJECT: u32 = 0x004e_7ff0;
const WATER_OBJECT_GET_FLAG: u32 = 0x0045_6610;
const WATER_OBJECT_SET_FLAG: u32 = 0x0045_0f90;
/// Run before and after the picture: `fn_0054ee60(flag)`.
const MAP_PICTURE_SET_FLAG: u32 = 0x0054_ee60;
/// The render target of a rendered texture (the map names the body
/// `BSRenderedTexture::StopOffscreen`, a folded name), the call that sets it
/// `(7, target)` (cdecl), and the two calls that end an offscreen pass.
const RENDERED_TEXTURE_TARGET: u32 = 0x00b6_b260;
const SET_RENDER_TARGET: u32 = 0x00b6_b8d0;
const END_OFFSCREEN_MAIN: u32 = 0x00b6_b790;
const END_OFFSCREEN: u32 = 0x00b6_b840;
/// The shader accumulator: `BSShaderAccumulator::BSShaderAccumulator` (Xbox
/// PDB, `(this, 0x63, 1, 0x2f7)`) and its setters, the culling process calls
/// and the accumulation of a scene.
const SHADER_ACCUMULATOR_CONSTRUCT: u32 = 0x00b6_60d0;
const ACCUMULATOR_SET_SCENE_NODE: u32 = 0x004a_1020;
const CULLING_PROCESS_SET_ACCUMULATOR: u32 = 0x004a_0fd0;
const ACCUMULATOR_SET_MODE: u32 = 0x004a_1040;
const CULLING_PROCESS_BEGIN: u32 = 0x00c4_f270;
const CULLING_PROCESS_END: u32 = 0x00c4_f2d0;
/// `BSShaderUtil::AccumulateScene` (Xbox PDB) `(root, scene node, culling
/// process)` and the call that finishes it `(root, accumulator, 0)`.
const ACCUMULATE_SCENE: u32 = 0x00b6_bee0;
const FINISH_ACCUMULATION: u32 = 0x00b6_c0d0;
/// The image space manager, its effect getter `(manager, 0xE)` and
/// `ImageSpaceManager::RenderEffect_ov2` (Xbox PDB).
const IMAGE_SPACE_MANAGER: u32 = 0x004e_3270;
const IMAGE_SPACE_GET_EFFECT: u32 = 0x004e_bbc0;
const IMAGE_SPACE_RENDER_EFFECT: u32 = 0x00b8_c830;
/// The accumulator's flag setter `(accumulator, on)`.
const ACCUMULATOR_SET_FLAG: u32 = 0x008a_5cf0;
/// The test `fn_0054ee20` makes (no `this`), and the object it then asks for
/// the byte at `+0x18` (`(1)`).
const MAP_PICTURE_TEST: u32 = 0x0051_d740;
const MAP_PICTURE_OBJECT: u32 = 0x0066_4840;
/// The rendered-texture call `fn_0054ede0` makes `(target, flag)` (cdecl).
const SET_RENDER_TARGET_FLAGGED: u32 = 0x00b6_b7d0;
/// The cell's interior data object (the map gives this method no name);
/// `fn_0054deb0` / `fn_0054def0` use the word at its `+0x28`.
const CELL_INTERIOR_DATA: u32 = 0x0054_4600;

/// The surface of the texture's owner object (virtual slot `0x9c`), locked
/// with `LockRect(0, rect, 0, 0)` (COM slot `0x4c`, `this` on the stack).
fn lock_surface(e: &mut Engine, surface_owner: u32, rect: u32) {
    let surface = e
        .vcall(surface_owner, SURFACE_OWNER_SLOT_SURFACE, &args![])
        .u32();
    e.vcall(
        surface,
        SURFACE_SLOT_LOCK_RECT,
        &args![0u32, rect, 0u32, 0u32],
    );
}

/// `D3DTexture_LockRect` (the owner of the texture's surface) then
/// [`lock_surface`].
fn lock_texture_rect(e: &mut Engine, texture: u32, rect: u32) {
    let surface_owner = e.call(OBJECT_FIELD_24, &args![texture]).u32();
    lock_surface(e, surface_owner, rect);
}
fn dynamic_cast(e: &mut Engine, object: Ptr, from: u32, to: u32) -> Ptr {
    e.call(RT_DYNAMIC_CAST, &args![object, 0u32, from, to, 0u32])
        .ptr()
}

fn extra_list(e: &mut Engine, cell: Ptr<TESObjectCELL>) -> Ptr {
    e.call(CELL_EXTRA_DATA_LIST, &args![cell]).ptr()
}

fn is_interior(e: &mut Engine, cell: Ptr<TESObjectCELL>) -> bool {
    e.call(CELL_IS_INTERIOR, &args![cell]).bool()
}

fn world_space(e: &mut Engine, cell: Ptr<TESObjectCELL>) -> Ptr {
    e.call(CELL_GET_WORLD_SPACE, &args![cell]).ptr()
}

fn base_form(e: &mut Engine, reference: Ptr) -> Ptr {
    e.call(REFERENCE_GET_BASE_FORM, &args![reference]).ptr()
}

fn form_type(e: &mut Engine, form: Ptr) -> u32 {
    e.call(FORM_TYPE, &args![form]).u32()
}

fn lock_enter(e: &mut Engine, cell: Ptr<TESObjectCELL>) {
    e.call(CELL_LOCK_ENTER, &args![cell]);
}

fn lock_leave(e: &mut Engine, cell: Ptr<TESObjectCELL>) {
    e.call(CELL_LOCK_LEAVE, &args![cell]);
}

/// `PushFront` of `item` on the cell's reference list (the item is passed by
/// address, as the game does with its stack copy).
fn push_reference(e: &mut Engine, cell: Ptr<TESObjectCELL>, item: u32) {
    let list = e.call(CELL_REFERENCE_LIST, &args![cell]).ptr::<()>();
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        e.call(LIST_PUSH_FRONT, &args![list, slot]);
    });
}

// Translated from 00547650 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `ExtraDataList` entry of type `0x59` of the cell, field `+0xC`: a
/// form the cell carries (0 when the entry is absent). Read by
/// [`fn_00547680`] as the cell's own value before it falls back on the
/// world space and the defaults.
pub fn fn_00547650(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u32 {
    let list = extra_list(e, this);
    e.call(EXTRA_LIST_GET_CELL_FORM, &args![list]).u32()
}

// Translated from 00547680 (decompiled, FalloutNV.exe 1.4.0.525)
/// The form of extra data type `0x59` that applies to the cell: its own,
/// else (exterior cell) the world space's, else the default form `0x160`
/// (interior cell without the `0x80` cell flag) or `0x161`, looked up by
/// form id and cast from `TESForm`.
pub fn fn_00547680(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    let mut found = Ptr::new(fn_00547650(e, this));
    if found.is_null() && !is_interior(e, this) && !world_space(e, this).is_null() {
        let world = world_space(e, this);
        found = e.call(WORLD_SPACE_GET_CELL_FORM, &args![world]).ptr();
    }
    if found.is_null() && is_interior(e, this) && !e.call(CELL_FLAG_80, &args![this]).bool() {
        let form = e.call(FORM_LOOK_UP, &args![0x160u32]).ptr();
        return dynamic_cast(e, form, RTTI_TES_FORM, RTTI_CELL_FORM_TARGET);
    }
    if found.is_null() {
        let form = e.call(FORM_LOOK_UP, &args![0x161u32]).ptr();
        found = dynamic_cast(e, form, RTTI_TES_FORM, RTTI_CELL_FORM_TARGET);
    }
    found
}

// Translated from 00547750 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the cell's extra data entry of type `0x59` (adds,
/// replaces or, for 0, removes it).
pub fn fn_00547750(e: &mut Engine, this: Ptr<TESObjectCELL>, value: u32) {
    let list = extra_list(e, this);
    e.call(EXTRA_LIST_SET_CELL_FORM, &args![list, value]);
}

// Translated from 00547770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::GetWaterType` (Xbox PDB): the cell's own water type, else
/// its world space's, else the global default; when the cell has form flag 8
/// and the water has a replacement (the word at `+0x80`), that one.
pub fn tes_object_cell_get_water_type(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    let list = extra_list(e, this);
    let mut water: Ptr = e.call(EXTRA_LIST_GET_WATER_TYPE, &args![list]).ptr();
    if water.is_null() {
        let world = world_space(e, this);
        if !world.is_null() {
            water = e.call(WORLD_SPACE_GET_WATER_TYPE, &args![world]).ptr();
        }
    }
    if water.is_null() {
        water = Ptr::new(e.global::<u32>(DEFAULT_WATER_TYPE));
    }
    if !water.is_null()
        && e.call(FORM_FLAG_8, &args![this]).bool()
        && !e
            .call(WATER_REPLACEMENT, &args![water])
            .ptr::<()>()
            .is_null()
    {
        water = e.call(WATER_REPLACEMENT, &args![water]).ptr();
    }
    water
}

// Translated from 005477f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the cell's canopy shadow mask: for an exterior cell, `ExtraDataList`
/// entry type 10 (the texture goes to `texture_out`, the address of its
/// locked-rect pointer to `rect_out`); returns the entry's field `+0xC`, 0
/// for an interior cell or without the entry.
pub fn fn_005477f0(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    texture_out: Ptr,
    rect_out: Ptr,
) -> u32 {
    if is_interior(e, this) {
        return 0;
    }
    let list = extra_list(e, this);
    e.call(
        EXTRA_LIST_GET_CANOPY_MASK,
        &args![list, texture_out, rect_out],
    )
    .u32()
}

// Translated from 00547830 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::CreateCanopyShadowMaskForCell` (Xbox PDB), `cdecl`: for an
/// exterior `cell` (when the renderer reports it can), creates the 64 x 64
/// "Canopy Shadow Mask" texture into `*texture_out`, registers it in the
/// cell's extra data (entry type 10, which fills `*rect_out` with the
/// address of the locked-rect record), locks the surface and zeroes every
/// byte. Returns false (and a null texture) otherwise.
///
/// The compiler's exception frame is not translated.
pub fn tes_object_cell_create_canopy_shadow_mask_for_cell(
    e: &mut Engine,
    cell: Ptr<TESObjectCELL>,
    texture_out: Ptr,
    rect_out: Ptr,
) -> bool {
    e.mem.set_u32(texture_out.addr(), 0);
    if cell.is_null()
        || is_interior(e, cell)
        || e.call(RENDERER_CAN_CREATE_MASK, &args![]).u32() == 0
    {
        return false;
    }
    e.set_global(RENDER_TARGET_BUSY_FLAG, 1u8);
    e.set_global(MASK_FORMAT_WORD, 0x32u32);
    e.set_global(MASK_FORMAT_FLAG, 0u8);
    // The arguments the original pushes for the first `0043c4b0` are extra
    // words the callee ignores (it takes none).
    let renderer = e.call(RENDERER_GLOBAL, &args![]).u32();
    let texture = e.with_stack(4, |e, name| {
        e.call(FIXED_STRING_CONSTRUCT, &args![name, CANOPY_MASK_NAME]);
        let texture = e
            .call(
                RENDERED_TEXTURE_CREATE,
                &args![
                    name,
                    0x40u32,
                    0x40u32,
                    renderer,
                    MASK_FORMAT,
                    0u32,
                    0u32,
                    0u32
                ],
            )
            .u32();
        e.mem.set_u32(texture_out.addr(), texture);
        e.call(FIXED_STRING_DESTRUCT, &args![name]);
        texture
    });
    e.set_global(RENDER_TARGET_BUSY_FLAG, 0u8);
    e.set_global(MASK_FORMAT_FLAG, 1u8);
    let list = extra_list(e, cell);
    e.call(
        EXTRA_LIST_SET_CANOPY_MASK,
        &args![list, 1u32, e.mem.u32(texture_out.addr()), rect_out],
    );
    // Lock the surface of the new texture into the rect record.
    let rect = e.mem.u32(rect_out.addr());
    lock_texture_rect(e, texture, rect);
    let rect = e.mem.u32(rect_out.addr());
    let texture = e.mem.u32(texture_out.addr());
    let rows = e.vcall(texture, TEXTURE_SLOT_HEIGHT, &args![]).i32();
    let pitch = e.mem.i32(rect);
    let count = rows.wrapping_mul(pitch);
    let bits = e.mem.u32(rect + 4);
    let mut i = 0i32;
    while i < count {
        e.mem.set_u8(bits.wrapping_add(i as u32), 0);
        i += 1;
    }
    true
}

// Translated from 005479c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Paints one byte of the canopy shadow mask of the exterior cell at grid
/// `(x, y)` (a neighbour of the tree's cell): finds the cell through the
/// data handler when `*slot` is not set yet, reads or creates its mask,
/// locks the surface when the rect has no bits, and writes `value` at byte
/// `index`, or `0xFF` when that byte is already non-zero.
///
/// `slot` is the address of the caller's local rect pointer for that
/// neighbour (it stays set between calls). `ret 0x14`.
pub fn fn_005479c0(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    x: i32,
    y: i32,
    slot: Ptr,
    index: u32,
    value: u8,
) {
    // The function's local texture pointer (`[ebp-0xC]`), zero on entry.
    e.with_stack(4, |e, texture_slot| {
        if e.mem.u32(slot.addr()) == 0 {
            let world = world_space(e, this);
            let data_handler = e.global::<u32>(DATA_HANDLER_POINTER);
            let neighbour: Ptr<TESObjectCELL> = e
                .call(
                    DATA_HANDLER_FIND_CELL,
                    &args![data_handler, x, y, world, 0u32],
                )
                .ptr();
            if neighbour.is_null() {
                return;
            }
            let list = extra_list(e, neighbour);
            e.call(EXTRA_LIST_GET_CANOPY_MASK, &args![list, texture_slot, slot]);
            if e.mem.u32(texture_slot.addr()) == 0 {
                tes_object_cell_create_canopy_shadow_mask_for_cell(
                    e,
                    neighbour,
                    texture_slot,
                    slot,
                );
            }
        }
        let rect = e.mem.u32(slot.addr());
        if rect != 0 {
            if e.mem.u32(rect + 4) == 0 {
                // The original locks through the local texture pointer,
                // which is still null when the slot was already set.
                let texture = e.mem.u32(texture_slot.addr());
                lock_texture_rect(e, texture, rect);
            }
            let bits = e.mem.u32(e.mem.u32(slot.addr()) + 4);
            let at = bits.wrapping_add(index);
            if e.mem.u8(at) == 0 {
                e.mem.set_u8(at, value);
            } else {
                e.mem.set_u8(at, 0xff);
            }
        }
    });
}

/// The unsigned compare the loops of [`fn_00547ad0`] use (`JNC` after a
/// `cmp`): true while `value < limit` as unsigned numbers.
fn below(value: u32, limit: u32) -> bool {
    value < limit
}

// Translated from 00547ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Paints the canopy shadow of a tree at `(x, y, z)` into the canopy shadow
/// mask of the exterior cell and of its eight neighbours, `ret 0x14` (the
/// fifth stack word is never read).
///
/// Does nothing while the flag byte `011ca090` is set, for an interior
/// cell, when the world space's name is empty, or when the cell has no land
/// or the land cannot give the coordinates of the position. A radius of 0 or
/// more than 0x400 becomes 300. The cell's mask comes from its extra data
/// (made with [`tes_object_cell_create_canopy_shadow_mask_for_cell`] when
/// missing). The position becomes mask coordinates (`/ 64`), the radius
/// shrinks by 64, and for every mask pixel within a radius of it the byte
/// `255 - random(0, 55) - 60 * distance / radius` is stored, or `0xFF` when
/// the byte is already set; pixels outside 0..64 go to the neighbour cell
/// they fall into (`fn_005479c0`), keeping that neighbour's mask in a
/// local slot between pixels. The renderer's lock (`004a0370` /
/// `004a03c0`) brackets the painting.
///
/// The loops compare their counters as unsigned numbers, so a counter that
/// starts below zero never runs, as in the original. The float math runs in
/// `f64` and rounds to `f32` where the original stores a `float`.
pub fn fn_00547ad0(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    x: f32,
    y: f32,
    z: f32,
    radius: u32,
    _unused_5: u32,
) {
    if e.global::<u8>(CANOPY_DISABLED_FLAG) != 0 || is_interior(e, this) {
        return;
    }
    let world = world_space(e, this);
    let name = fn_00548210(e, world);
    if e.mem.u8(name.addr()) == 0 {
        return;
    }
    if e.call(TREE_UPDATE_FLAG, &args![]).bool() {
        e.call(TREE_MANAGER_REFRESH, &args![]);
    }
    // Locals: the texture (+0) and rect (+4) slots and the eight neighbour
    // rect slots (+8..+0x28).
    e.with_stack(0x2c, |e, locals| {
        let texture_slot = locals;
        let rect_slot = locals.byte_add(4);
        let list = extra_list(e, this);
        e.call(
            EXTRA_LIST_GET_CANOPY_MASK,
            &args![list, texture_slot, rect_slot],
        );
        let mut radius = radius;
        if radius == 0 || radius > 0x400 {
            radius = 300;
        }
        if e.mem.u32(texture_slot.addr()) == 0 {
            tes_object_cell_create_canopy_shadow_mask_for_cell(e, this, texture_slot, rect_slot);
        }
        e.with_stack(0x78, |e, coordinates| {
            e.call(COORDINATE_DATA_CONSTRUCT, &args![coordinates]);
            if e.call(CELL_GET_LAND, &args![this]).u32() == 0 {
                return;
            }
            let land = e.call(CELL_GET_LAND, &args![this]).u32();
            let covered = e.with_stack(12, |e, position| {
                e.mem.set_f32(position.addr(), x);
                e.mem.set_f32(position.addr() + 4, y);
                e.mem.set_f32(position.addr() + 8, z);
                e.call(
                    LAND_GET_COORD_DATA,
                    &args![land, coordinates, position, 0u32],
                )
                .bool()
            });
            if !covered {
                return;
            }
            // The mask has 64 pixels per cell: the world position over `64.0`.
            let divisor = e.global::<f64>(MASK_PIXEL_DIVISOR);
            let center_x_f = (e.mem.f32(coordinates.addr()) as f64 / divisor) as f32;
            let center_y_f = (e.mem.f32(coordinates.addr() + 4) as f64 / divisor) as f32;
            let radius = radius >> 6;
            if e.mem.u32(texture_slot.addr()) == 0 {
                return;
            }
            paint_canopy_mask(e, this, locals, center_x_f, center_y_f, radius);
        });
    });
}

/// The painting of [`fn_00547ad0`] once the mask and the coordinates exist.
fn paint_canopy_mask(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    locals: Ptr,
    center_x: f32,
    center_y: f32,
    radius: u32,
) {
    let texture_slot = locals;
    let rect_slot = locals.byte_add(4);
    let renderer = e.call(RENDERER_GLOBAL, &args![]).u32();
    e.call(RENDERER_LOCK, &args![renderer]);
    let rect = e.mem.u32(rect_slot.addr());
    if e.mem.u32(rect + 4) == 0 {
        let texture = e.mem.u32(texture_slot.addr());
        let owner = e.call(OBJECT_FIELD_24, &args![texture]).u32();
        if owner != 0 {
            lock_surface(e, owner, rect);
        }
    }
    let bits = e.mem.u32(e.mem.u32(rect_slot.addr()) + 4);
    let texture = e.mem.u32(texture_slot.addr());
    let stride = e.vcall(texture, TEXTURE_SLOT_HEIGHT, &args![]).i32();
    // (The original also multiplies the rect's pitch by the row count and
    // stores the product without using it.)
    let cell_x = e.call(FTOL, &args![center_x as f64]).i32() as u32;
    let cell_y = e.call(FTOL, &args![center_y as f64]).i32() as u32;
    let radius_float = radius as f64;
    let mut row = cell_y.wrapping_sub(radius);
    while below(row, cell_y.wrapping_add(radius)) {
        let mut column = cell_x.wrapping_sub(radius);
        while below(column, cell_x.wrapping_add(radius)) {
            let dx = center_x as f64 - column as i32 as f64;
            let dy = center_y as f64 - row as i32 as f64;
            let squared = (dx * dx + dy * dy) as f32;
            let distance = e.call(SQUARE_ROOT, &args![squared]).f32();
            if distance as f64 <= radius_float {
                let r = row as i32;
                let c = column as i32;
                let index = r.wrapping_mul(stride).wrapping_add(c);
                let low = 0.0f32;
                let high = e.global::<f32>(CANOPY_RANDOM_RANGE);
                let random = e.call(RANDOM_FLOAT, &args![low, high]).f32();
                let base = e.global::<f64>(SHADE_BASE);
                let falloff = e.global::<f64>(SHADE_FALLOFF);
                let shade = ((base - random as f64) - (distance as f64 / radius_float) * falloff)
                    as i32 as u8;
                if (0..0x40).contains(&r) && (0..0x40).contains(&c) {
                    let at = bits.wrapping_add(index as u32);
                    if e.mem.u8(at) != 0 {
                        e.mem.set_u8(at, 0xff);
                    } else {
                        e.mem.set_u8(at, shade);
                    }
                } else {
                    // Offsets into the neighbour's mask, the neighbour's
                    // grid offset and the slot (a local) that keeps its rect.
                    let (index, dx, dy, slot) = if r >= 0x40 && c < 0 {
                        ((r - 0x40) * stride + c + 0x40, -1, 1, 0x1c)
                    } else if r >= 0x40 && c >= 0x40 {
                        ((r - 0x40) * stride + c - 0x40, 1, 1, 0x18)
                    } else if r < 0 && c < 0 {
                        ((r + 0x40) * stride + c + 0x40, -1, -1, 0x20)
                    } else if r < 0 && c >= 0x40 {
                        ((r + 0x40) * stride + c - 0x40, 1, -1, 0x24)
                    } else if r >= 0x40 {
                        ((r - 0x40) * stride + c, 0, 1, 0x08)
                    } else if r < 0 {
                        ((r + 0x40) * stride + c, 0, -1, 0x0c)
                    } else if c >= 0x40 {
                        (r * stride + c - 0x40, 1, 0, 0x10)
                    } else {
                        (r * stride + c + 0x40, -1, 0, 0x14)
                    };
                    let slot = locals.byte_add(slot);
                    let neighbour_y = e.call(CELL_GET_DATA_Y, &args![this]).i32();
                    let neighbour_x = e.call(CELL_GET_DATA_X, &args![this]).i32();
                    fn_005479c0(
                        e,
                        this,
                        neighbour_x + dx,
                        neighbour_y + dy,
                        slot,
                        index as u32,
                        shade,
                    );
                }
            }
            column = column.wrapping_add(1);
        }
        row = row.wrapping_add(1);
    }
    let renderer = e.call(RENDERER_GLOBAL, &args![]).u32();
    e.call(RENDERER_UNLOCK, &args![renderer]);
}

// Translated from 00548210 (decompiled, FalloutNV.exe 1.4.0.525)
/// The text of the world space's name (`this + 0xD4`): a pointer to its
/// characters. The engine map names this function
/// `MapMarkerData::GetLocationName` because the linker folded the identical
/// code.
pub fn fn_00548210(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(
        STRING_GET_TEXT,
        &args![this.byte_add(WORLD_SPACE_NAME_OFFSET)],
    )
    .ptr()
}

// Translated from 00548230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::AddReference` (Xbox PDB), `ret 8`: puts `reference` in the
/// cell. A persistent cell only records it (list, persistent cell of the
/// reference, the world space's persistent data). Otherwise the reference
/// leaves its previous cell, is pushed on the cell's list, and, depending on
/// whether the cell is loaded, is attached to the cell's 3D at once, queued
/// for the model loader, or has its 3D cleared (virtual slot `0x1CC(0, 0)`,
/// or the task queue when attaches are queued). Then the reference's light is
/// registered with the shadow scene node (`005728c0`) and, for a reference
/// in a non-interior cell, it is put once on the kind list of its form flag
/// `0x10000000` and of its `0x20000000`. `flag` goes to
/// [`tes_object_cell_attach_reference_3d`] (its unused second word).
///
/// The local flag the compiler keeps at `ebp-1` (always 0) is folded away.
pub fn tes_object_cell_add_reference(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    flag: u8,
) {
    if reference.is_null() || base_form(e, reference).is_null() {
        return;
    }
    if e.call(REFERENCE_HAS_VISIBLE_DISTANT, &args![reference])
        .bool()
        && !e.call(FORM_FLAG_800, &args![reference]).bool()
    {
        let count = e.get(this, TESObjectCELL::sNumRefsWithVisibleDistant);
        e.set(
            this,
            TESObjectCELL::sNumRefsWithVisibleDistant,
            count.wrapping_add(1),
        );
    }
    if e.call(CELL_PERSISTENT_FLAG, &args![this]).bool() {
        lock_enter(e, this);
        push_reference(e, this, reference.addr());
        lock_leave(e, this);
        let list = e
            .call(REFERENCE_EXTRA_DATA_LIST, &args![reference])
            .ptr::<()>();
        e.call(EXTRA_LIST_SET_PERSISTENT_CELL, &args![list, this]);
        let world = world_space(e, this);
        if !world.is_null() {
            e.call(
                WORLD_SPACE_ADD_TO_PERSISTENT_REF_DATA,
                &args![world, reference],
            );
        }
        if !data_handler_flag(e) {
            e.vcall(this.addr(), CELL_SLOT_SET_LOADED, &args![1u32]);
        }
        return;
    }

    let previous = e.call(REFERENCE_PARENT_CELL, &args![reference]).ptr::<()>();
    if !previous.is_null() {
        e.call(CELL_REMOVE_REFERENCE, &args![previous, reference]);
    }
    lock_enter(e, this);
    push_reference(e, this, reference.addr());
    e.vcall(reference.addr(), REFERENCE_SLOT_ADD_TO_CELL, &args![this]);
    lock_leave(e, this);
    if e.call(CELL_STATE_IS_6, &args![this]).bool() {
        let object = e.global::<u32>(STATE_6_OBJECT_POINTER);
        e.call(STATE_6_ADD_REFERENCE, &args![object, reference]);
    }
    if !e.get(this, TESObjectCELL::pLoadedData).is_null() {
        let actor = is_actor(e, reference);
        if (!actor || e.call(FORM_FLAG_800, &args![reference]).bool())
            && e.call(REFERENCE_IS_SCRIPTED, &args![reference]).bool()
        {
            e.call(CELL_ADD_SCRIPTED_REF, &args![this, reference]);
        }
        if e.call(REFERENCE_IS_ACTIVATING_CHILDREN, &args![reference])
            .bool()
        {
            e.call(CELL_ADD_ACTIVATING_REF, &args![this, reference]);
        }
    }
    if !e.call(FORM_FLAG_800, &args![reference]).bool()
        && e.vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
            .u32()
            != 0
    {
        let cast = dynamic_cast(e, reference, RTTI_REFERENCE, RTTI_REFERENCE_CAST_TARGET);
        if cast.is_null() {
            e.vcall(reference.addr(), REFERENCE_SLOT_UNLOAD_3D, &args![]);
        }
    }
    if !data_handler_flag(e) && !e.call(FORM_FLAG_820, &args![reference]).bool() {
        let node = e
            .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
            .u32();
        let tes = e.global::<u32>(TES_POINTER);
        if e.call(TES_IS_CELL_LOADED, &args![tes, this, 0u32]).bool() {
            let mut deferred = false;
            if node != 0 {
                tes_object_cell_attach_reference_3d(e, this, reference, flag as u32);
            } else {
                if !is_actor(e, reference) {
                    deferred = true;
                } else {
                    let player = e.global::<u32>(PLAYER_POINTER);
                    if !e.call(PLAYER_IS_SLEEPING_OR_RESTING, &args![player]).bool()
                        && !fn_00548720(e, this)
                    {
                        deferred = true;
                    }
                }
                if deferred {
                    if is_actor(e, reference) {
                        let save = e.global::<u32>(SAVE_GAME_POINTER);
                        let first = e.call(SAVE_GAME_FLAG, &args![save]).bool();
                        let run = !first || e.call(SAVE_GAME_FLAG, &args![save]).bool();
                        if run {
                            if e.call(CELL_STATE_IS_6, &args![this]).bool()
                                || e.call(CELL_STATE_IS_5, &args![this]).bool()
                            {
                                e.vcall(reference.addr(), REFERENCE_SLOT_STATE_6_OR_5, &args![]);
                            } else {
                                e.vcall(reference.addr(), REFERENCE_SLOT_OTHER_STATE, &args![]);
                            }
                        }
                    }
                    let priority = e.call(TES_GET_CELL_PRIORITY, &args![tes, this, 0u32]).u32();
                    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
                    e.call(
                        MODEL_LOADER_QUEUE_REFERENCE,
                        &args![loader, reference, priority, 0u32],
                    );
                }
            }
            if !deferred && is_actor(e, reference) {
                e.call(
                    PROCESS_LISTS_ADD_ACTOR_TO_TEMP_CHANGE_LIST,
                    &args![PROCESS_LISTS, reference],
                );
            }
        } else {
            let key = e
                .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
                .u32();
            e.call(QUEUED_ATTACH_MAP_REMOVE_AT, &args![QUEUED_ATTACH_MAP, key]);
            if is_actor(e, reference) {
                let player = e.global::<u32>(PLAYER_POINTER);
                let save = e.global::<u32>(SAVE_GAME_POINTER);
                if !e.call(PLAYER_IS_SLEEPING_OR_RESTING, &args![player]).bool()
                    && !e.call(SAVE_GAME_FLAG, &args![save]).bool()
                {
                    if !e.call(ATTACHES_ARE_QUEUED, &args![]).bool() {
                        e.vcall(reference.addr(), REFERENCE_SLOT_SET_3D, &args![0u32, 0u32]);
                    } else {
                        let queue = e.call(TASK_QUEUE_GETTER, &args![]).u32();
                        e.call(TASK_QUEUE_SET_3D_NULL, &args![queue, reference]);
                    }
                }
            }
        }
    }
    e.call(REFERENCE_ADD_LIGHT_TO_SCENE, &args![reference, 0u32]);
    let parent = e.call(REFERENCE_PARENT_CELL, &args![reference]).ptr::<()>();
    if !e.call(CELL_IS_INTERIOR, &args![parent]).bool() {
        if fn_00548700(e, reference.cast()) && !list_contains(e, KIND_LIST_A, reference.addr()) {
            push_front_value(e, KIND_LIST_A, reference.addr());
        }
        if e.call(FORM_FLAG_20000000, &args![reference]).bool()
            && !list_contains(e, KIND_LIST_B, reference.addr())
        {
            push_front_value(e, KIND_LIST_B, reference.addr());
        }
    }
}

// Translated from 00548700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Form flag test: `iFormFlags & 0x10000000`.
pub fn fn_00548700(e: &mut Engine, this: Ptr<TESObjectCELL>) -> bool {
    e.get(this, TESObjectCELL::iFormFlags) & 0x1000_0000 != 0
}

// Translated from 00548720 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cCellState == 2` (through the state accessor `00450fd0`).
pub fn fn_00548720(e: &mut Engine, this: Ptr<TESObjectCELL>) -> bool {
    e.call(CELL_STATE, &args![this]).u32() as u8 == 2
}

// Translated from 00548740 (decompiled, FalloutNV.exe 1.4.0.525)
/// Moves `reference` into the cell's list without the attach work of
/// [`tes_object_cell_add_reference`] (`ret 4`): a persistent cell records
/// the reference and its persistent cell; another cell first removes the
/// reference from its previous parent cell, then pushes it, and lets the
/// reference's virtual slot `0x228` take the cell. Unless the reference is
/// persistent (form flag `0x4000` or `GetRefPersists`) or the data handler's
/// loading flag is set, the cell's virtual slot `0xC8` is then called with 1;
/// last, the reference's light is registered (`005728c0`).
pub fn fn_00548740(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr) {
    if reference.is_null() || base_form(e, reference).is_null() {
        return;
    }
    if e.call(CELL_PERSISTENT_FLAG, &args![this]).bool() {
        lock_enter(e, this);
        push_reference(e, this, reference.addr());
        lock_leave(e, this);
        let list = e
            .call(REFERENCE_EXTRA_DATA_LIST, &args![reference])
            .ptr::<()>();
        e.call(EXTRA_LIST_SET_PERSISTENT_CELL, &args![list, this]);
        if !data_handler_flag(e) {
            e.vcall(this.addr(), CELL_SLOT_SET_LOADED, &args![1u32]);
        }
        return;
    }
    let previous = e.call(REFERENCE_PARENT_CELL, &args![reference]).ptr::<()>();
    if !previous.is_null() {
        e.call(CELL_REMOVE_REFERENCE, &args![previous, reference]);
    }
    lock_enter(e, this);
    push_reference(e, this, reference.addr());
    e.vcall(reference.addr(), REFERENCE_SLOT_ADD_TO_CELL, &args![this]);
    lock_leave(e, this);
    if !e.call(FORM_FLAG_4000, &args![reference]).bool()
        && !e.call(REFERENCE_GET_REF_PERSISTS, &args![reference]).bool()
        && !data_handler_flag(e)
    {
        e.vcall(this.addr(), CELL_SLOT_SET_LOADED, &args![1u32]);
    }
    e.call(REFERENCE_ADD_LIGHT_TO_SCENE, &args![reference, 0u32]);
}

/// Allocation of a scene graph object followed by its constructor
/// (`ecx = block`, then `extra` as stack words); a null block stays null and
/// skips the constructor, as the original does.
fn construct_scene_object(e: &mut Engine, size: u32, constructor: u32, extra: &[u32]) -> u32 {
    let block = e.call(NODE_ALLOCATE, &args![size]).u32();
    if block == 0 {
        return 0;
    }
    let mut words = vec![block];
    words.extend_from_slice(extra);
    e.call(constructor, &words).u32()
}

/// The child node `index` of the cell's 3D (`ret 4`).
fn child_node(e: &mut Engine, cell: Ptr<TESObjectCELL>, index: u32) -> u32 {
    e.call(CELL_CHILD_NODE, &args![cell, index]).u32()
}

fn is_actor(e: &mut Engine, reference: Ptr) -> bool {
    e.vcall(reference.addr(), REFERENCE_SLOT_IS_ACTOR, &args![])
        .bool()
}

fn data_handler_flag(e: &mut Engine) -> bool {
    let handler = e.global::<u32>(DATA_HANDLER_POINTER);
    e.call(DATA_HANDLER_LOADING_FLAG, &args![handler]).bool()
}

/// `BSSimpleList::PushFront` of the word `value` on the list at `list`.
fn push_front_value(e: &mut Engine, list: u32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(LIST_PUSH_FRONT, &args![list, slot]);
    });
}

/// `LIST_CONTAINS(list, &value)`.
fn list_contains(e: &mut Engine, list: u32, value: u32) -> bool {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(LIST_CONTAINS, &args![list, slot]).bool()
    })
}

/// The `ExtraDataList` of a reference (`this + 0x44`).
fn reference_extra_list(e: &mut Engine, reference: Ptr) -> u32 {
    e.call(REFERENCE_EXTRA_DATA_LIST, &args![reference]).u32()
}

/// The reference's orientation matrix (`GetOrientation` copies nine words
/// into the buffer it is given and returns it).
fn reference_orientation(e: &mut Engine, reference: Ptr) -> u32 {
    e.with_stack(0x24, |e, buffer| {
        e.call(REFERENCE_GET_ORIENTATION, &args![reference, buffer])
            .u32()
    })
}

// Translated from 00548880 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::AttachReference3D` (Xbox PDB), `ret 8` (the second stack
/// word, which `AddReference` fills from its flag, is never read): puts the
/// loaded 3D of `reference` under the right child node of the cell's 3D and
/// does the per-kind setup the form needs.
///
/// 1. A reference without a 3D does nothing. A tree (base form flag 0x40)
///    hides itself in the world space's terrain manager. A placeable water
///    (form type 0x23) bumps a counter of the `TES` object and, in an
///    interior cell without the water flag, prints a warning and stops
///    unless the string setting at `011c7adc` has text.
/// 2. The node to attach to depends on the base form: child 0 for an
///    actor, child 1 for the forms at `011ca230` and `011ca238` (the latter
///    first builds the multibound room, see `attach_room`), child 6 for
///    the form at `011ca23c` (portal, see `attach_portal`), child 5 for the
///    form at `011ca234` (occlusion plane, see `attach_occlusion_plane`),
///    child 8 for the form at `011ca240`, the light node for form type
///    0x1E, and so on down to the default, child 3.
/// 3. External emittance and placeable water are registered with the cell.
/// 4. The 3D is attached: for the player both of its 3D roots go under the
///    node; for everything else
///    [`tes_object_cell_perform_cell_node_attach`] runs (or the queued
///    version [`fn_0054abd0`] while `008c7aa0` says so), after which the
///    fade settings are applied (tree flag, kind 6 or 10, the plain fade
///    check, or the alpha reset with `fLodFadeInPercent` while the loading
///    menu is up) and the cell's light markers are refreshed.
///
/// The compiler's exception frame is not translated.
pub fn tes_object_cell_attach_reference_3d(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    _unused_2: u32,
) {
    let mut time_controllers = false;
    if reference.is_null() || e.call(REFERENCE_GET_3D, &args![reference]).u32() == 0 {
        return;
    }
    let base = base_form(e, reference);
    if fn_00549580(e, base) {
        let world = e
            .call(REFERENCE_GET_WORLD_SPACE, &args![reference])
            .ptr::<()>();
        if !world.is_null() {
            let manager = e.call(WORLD_SPACE_GET_TERRAIN_MANAGER, &args![world]).u32();
            e.call(TERRAIN_MANAGER_HIDE_TREE, &args![manager, reference, 1u32]);
            let manager = e.call(WORLD_SPACE_GET_TERRAIN_MANAGER, &args![world]).u32();
            e.call(TERRAIN_MANAGER_SET_FLAG_28, &args![manager, 1u32]);
        }
    }
    let base = base_form(e, reference);
    if !base.is_null() && form_type(e, base) == FORM_TYPE_23 {
        let tes = e.global::<u32>(TES_POINTER);
        e.call(INCREMENT_FIELD_B8, &args![tes]);
        let water_object = e.call(CELL_WATER_OBJECT, &args![this]).u32();
        if e.call(OBJECT_FIELD_24, &args![water_object]).u32() != 0 {
            let water_object = e.call(CELL_WATER_OBJECT, &args![this]).u32();
            let inner = e.call(OBJECT_FIELD_24, &args![water_object]).u32();
            e.call(SET_BYTE_8, &args![inner, 1u32]);
        }
        if is_interior(e, this) && !e.call(CELL_FLAG_2, &args![this]).bool() {
            let name = e.vcall(this.addr(), CELL_SLOT_GET_NAME, &args![]).u32();
            let cell_id = e.call(FORM_ID, &args![this]).u32();
            let reference_id = e.call(FORM_ID, &args![reference]).u32();
            e.call(
                DEBUG_PRINT,
                &args![WATERLESS_CELL_MESSAGE, reference_id, cell_id, name],
            );
            let text = e.call(SETTING_TEXT, &args![WATER_WARNING_SETTING]).u32();
            if e.mem.u8(text) == 0 {
                return;
            }
        }
    }

    let mut node = 0u32;
    'select: {
        if is_actor(e, reference) {
            node = child_node(e, this, 0);
            break 'select;
        }
        if base_form(e, reference).addr() == e.global::<u32>(MARKER_FORM_230) {
            node = child_node(e, this, 1);
            break 'select;
        }
        if base_form(e, reference).addr() == e.global::<u32>(ROOM_MARKER_FORM_238) {
            attach_room(e, this, reference);
            node = child_node(e, this, 1);
            break 'select;
        }
        if base_form(e, reference).addr() == e.global::<u32>(PORTAL_MARKER_FORM_23C)
            && !e.call(FORM_FLAG_4000, &args![reference]).bool()
        {
            attach_portal(e, this, reference);
            node = child_node(e, this, 6);
            break 'select;
        }
        if base_form(e, reference).addr() == e.global::<u32>(OCCLUSION_MARKER_FORM_234) {
            node = child_node(e, this, 5);
            attach_occlusion_plane(e, reference);
            break 'select;
        }
        if base_form(e, reference).addr() == e.global::<u32>(MARKER_FORM_240) {
            node = child_node(e, this, 8);
            break 'select;
        }
        if e.call(GLOBAL_VIRTUAL_TEST, &args![reference]).bool() {
            let base = base_form(e, reference);
            if form_type(e, base) == FORM_TYPE_1E {
                node = fn_005495d0(e, this).addr();
                let base = base_form(e, reference);
                if !fn_00549630(e, base) {
                    e.call(CELL_ADD_MULTI_BOUND_REF, &args![this, reference]);
                }
                break 'select;
            }
        }
        let active = e.call(GLOBAL_VIRTUAL_TEST, &args![reference]).bool();
        let attached_shape = active || {
            let list = reference_extra_list(e, reference);
            e.call(EXTRA_LIST_GET_PRIMITIVE, &args![list]).u32() != 0
        };
        if attached_shape {
            let base = base_form(e, reference);
            node = if form_type(e, base) == FORM_TYPE_0E {
                fn_005495f0(e, this).addr()
            } else {
                child_node(e, this, 1)
            };
            break 'select;
        }
        let base = base_form(e, reference);
        let kind = form_type(e, base);
        if kind == FORM_TYPE_0D {
            node = fn_005495f0(e, this).addr();
            break 'select;
        }
        if kind == FORM_TYPE_25 {
            e.call(CELL_ADD_MULTI_BOUND_REF, &args![this, reference]);
        } else if e
            .vcall(reference.addr(), REFERENCE_SLOT_224, &args![])
            .bool()
        {
            node = child_node(e, this, 4);
        } else {
            let mut handled = false;
            let base = base_form(e, reference);
            if !base.is_null() && form_type(e, base) == FORM_TYPE_1E {
                let base = base_form(e, reference);
                if !fn_00549630(e, base) {
                    e.call(CELL_ADD_MULTI_BOUND_REF, &args![this, reference]);
                    handled = true;
                }
            }
            if !handled {
                let key = e.call(EXTRA_DATA_KEY, &args![]).u32();
                let node_3d = e
                    .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
                    .u32();
                let extra = e.call(OBJECT_GET_EXTRA_DATA, &args![node_3d, key]).u32();
                if extra != 0 && fn_00549690(e, Ptr::new(extra)) {
                    let node_3d = e
                        .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
                        .u32();
                    if e.call(HAS_TIME_CONTROLLERS, &args![node_3d]).bool() {
                        time_controllers = true;
                        e.call(CELL_ADD_MULTI_BOUND_REF, &args![this, reference]);
                    }
                }
            }
        }
        if node == 0 && e.call(SETTING_FIRST_CHAR, &args![FEATURE_SETTING]).bool() {
            let list = reference_extra_list(e, reference);
            let target = e.call(EXTRA_LIST_GET_ROOM_KEY, &args![list]).u32();
            if target != 0 {
                let parent = e
                    .call(REFERENCE_PARENT_CELL, &args![reference])
                    .ptr::<TESObjectCELL>();
                node = e.call(CELL_NODE_FOR_TARGET, &args![parent, target]).u32();
            }
        }
        if node == 0 {
            node = child_node(e, this, 3);
        }
    }

    if e.call(REFERENCE_USES_EXTERNAL_EMITTANCE, &args![reference])
        .bool()
    {
        e.call(CELL_ADD_EMITTANCE_REF, &args![this, reference]);
    }
    let base = base_form(e, reference);
    if form_type(e, base) == FORM_TYPE_23 {
        e.call(CELL_ADD_WATER_REF, &args![this, reference]);
        let tes = e.global::<u32>(TES_POINTER);
        let water_system = e.call(TES_WATER_SYSTEM, &args![tes]).u32();
        e.call(
            WATER_SYSTEM_ADD_PLACEABLE_WATER,
            &args![water_system, reference],
        );
    }
    if node == 0 {
        return;
    }
    let player = e.global::<u32>(PLAYER_POINTER);
    if reference.addr() == player {
        // The player: both 3D roots (first and third person) go under the
        // node and get their properties updated.
        let root = e.call(PLAYER_GET_CURRENT_3D, &args![player, 0u32]).u32();
        e.vcall(node, NODE_SLOT_ATTACH_CHILD, &args![root, 1u32]);
        let root = e.call(PLAYER_GET_CURRENT_3D, &args![player, 0u32]).u32();
        e.call(NODE_UPDATE_PROPERTIES, &args![root]);
        let root = e.call(PLAYER_GET_CURRENT_3D, &args![player, 0u32]).u32();
        let fade_node = e.vcall(root, NODE_SLOT_GET_FADE_NODE, &args![]).u32();
        if fade_node != 0 {
            e.call(FADE_NODE_PREPARE, &args![fade_node]);
        }
        let root = e.call(PLAYER_GET_CURRENT_3D, &args![player, 1u32]).u32();
        e.vcall(node, NODE_SLOT_ATTACH_CHILD, &args![root, 1u32]);
        let root = e.call(PLAYER_GET_CURRENT_3D, &args![player, 1u32]).u32();
        e.call(NODE_UPDATE_PROPERTIES, &args![root]);
        return;
    }

    if !e.call(ATTACHES_ARE_QUEUED, &args![]).bool() {
        tes_object_cell_perform_cell_node_attach(e, this, reference, Ptr::new(node));
    } else {
        fn_0054abd0(e, reference, Ptr::new(node));
    }
    let node_3d = e.call(REFERENCE_GET_3D, &args![reference]).u32();
    let fade_node = e.vcall(node_3d, NODE_SLOT_GET_FADE_NODE, &args![]).u32();
    if fade_node != 0 {
        let base = base_form(e, reference);
        if fn_00549580(e, base) {
            e.call(FADE_NODE_SET_TREE_FLAG, &args![fade_node, 1u32]);
        } else if e
            .call(REFERENCE_HAS_VISIBLE_DISTANT, &args![reference])
            .bool()
        {
            e.call(FADE_NODE_SET_KIND, &args![fade_node, 6u32]);
        } else if e.call(REFERENCE_FADES_OUT, &args![reference]).bool() {
            e.call(FADE_NODE_SET_KIND, &args![fade_node, 10u32]);
            let range = e.global::<f32>(FLOAT_MAX);
            e.call(FADE_NODE_SET_RANGE, &args![fade_node, range, range]);
        }
        let loading_menu = e.call(LOADING_MENU_VISIBLE, &args![]).bool();
        let loader = e.global::<u32>(GAME_LOADER_POINTER);
        if loading_menu || e.call(GAME_LOADER_FLAG_244_2, &args![loader]).bool() {
            e.call(FADE_NODE_PREPARE, &args![fade_node]);
            e.call(SET_PROPERTY_FADE_ALPHA, &args![fade_node, 1.0f32]);
            let percent = e.global::<f32>(LOD_FADE_IN_START);
            e.set(this, TESObjectCELL::fLodFadeInPercent, percent);
            e.set(this, TESObjectCELL::bFadingToHighDetail, false);
            e.set(this, TESObjectCELL::bFadingToLowDetail, false);
            e.set(this, TESObjectCELL::bDisplayHighDetail, true);
        } else {
            let slot_holder = e.call(GLOBAL_SLOT_GET, &args![]).u32();
            let data = e.call(SLOT_AT_AC_GET, &args![slot_holder]).u32();
            e.call(
                FADE_NODE_CHECK_FADE_RADIUS,
                &args![fade_node, data, time_controllers as u32],
            );
        }
        if e.get(this, TESObjectCELL::sNumLoadedRefsWithVisibleDistant) > 0
            && fn_005495a0(e, this)
            && e.call(CELL_STATE_NONZERO, &args![this]).bool()
        {
            e.call(CELL_START_FADE_IN, &args![this]);
        }
    }
}

/// The multibound room case of [`tes_object_cell_attach_reference_3d`]:
/// when the reference's extra data marks a master room without a room yet
/// (and the reference has no multibound room), builds a `BSMultiBoundRoom`
/// and a `BSMultiBound` sized from the primitive at the reference's
/// position, links them, and stores the room in the reference's extra data.
/// Then (for any master room) the room's shape gets the reference's position
/// and the cell's portal graph gets the room.
fn attach_room(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr) {
    let list = reference_extra_list(e, reference);
    if !e.call(EXTRA_LIST_GET_ROOM_IS_MASTER, &args![list]).bool() {
        return;
    }
    let list = reference_extra_list(e, reference);
    if e.call(EXTRA_LIST_GET_ROOM, &args![list]).u32() == 0
        && e.call(REFERENCE_GET_MULTI_BOUND_ROOM, &args![reference])
            .u32()
            == 0
    {
        let position = e.call(REFERENCE_POSITION_ADDRESS, &args![reference]).u32();
        let list = reference_extra_list(e, reference);
        let primitive = e.call(EXTRA_LIST_GET_PRIMITIVE, &args![list]).u32();
        let shape = e.with_stack(12, |e, copy| {
            for i in 0..3 {
                let word = e.mem.u32(position + 4 * i);
                e.mem.set_u32(copy.addr() + 4 * i, word);
            }
            e.vcall(primitive, PRIMITIVE_SLOT_MAKE_SHAPE, &args![copy])
                .u32()
        });
        let room = construct_scene_object(e, 0xec, MULTI_BOUND_ROOM_CONSTRUCT, &[]);
        let multi_bound = construct_scene_object(e, 0x10, MULTI_BOUND_CONSTRUCT, &[]);
        e.call(ROOM_SET_MULTI_BOUND, &args![room, multi_bound]);
        e.call(MULTI_BOUND_SET_SHAPE, &args![multi_bound, shape]);
        let position = e
            .vcall(reference.addr(), REFERENCE_SLOT_POSITION, &args![])
            .u32();
        e.vcall(multi_bound, MULTI_BOUND_SLOT_SET_POSITION, &args![position]);
        let list = reference_extra_list(e, reference);
        e.call(EXTRA_LIST_SET_ROOM, &args![list, room]);
    }
    let list = reference_extra_list(e, reference);
    let room = e.call(EXTRA_LIST_GET_ROOM, &args![list]).u32();
    let multi_bound = e.call(SLOT_AT_AC_GET, &args![room]).u32();
    let shape = e.call(SLOT_AT_C_GET, &args![multi_bound]).u32();
    let position = e
        .vcall(reference.addr(), REFERENCE_SLOT_POSITION, &args![])
        .u32();
    e.vcall(shape, SHAPE_SLOT_SET_POSITION, &args![position]);
    let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
    e.call(PORTAL_GRAPH_ADD_ROOM, &args![graph, room]);
}

/// The portal case of [`tes_object_cell_attach_reference_3d`] (a portal
/// reference whose extra data has no portal yet): takes the shape of the
/// primitive, gives it the reference's position and orientation, stores it
/// as the reference's portal, builds a `BSPortalSharedNode` named "Portal
/// Shared Geometry" for it under the portal graph's node, then connects the
/// portal to the multibound rooms of the two references linked to it (an
/// unlinked end is handed to the portal graph).
fn attach_portal(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr) {
    let list = reference_extra_list(e, reference);
    if e.call(EXTRA_LIST_GET_PORTAL, &args![list]).u32() != 0 {
        return;
    }
    let list = reference_extra_list(e, reference);
    let primitive = e.call(EXTRA_LIST_GET_PRIMITIVE, &args![list]).u32();
    let shape = e.vcall(primitive, PRIMITIVE_SLOT_SHAPE, &args![]).u32();
    if shape == 0 {
        return;
    }
    let position = e
        .vcall(reference.addr(), REFERENCE_SLOT_POSITION, &args![])
        .u32();
    e.call(SHAPE_SET_POSITION, &args![shape, position]);
    let orientation = reference_orientation(e, reference);
    e.call(SHAPE_SET_ORIENTATION, &args![shape, orientation]);
    let list = reference_extra_list(e, reference);
    e.call(EXTRA_LIST_SET_PORTAL, &args![list, shape]);
    let node = construct_scene_object(e, 0xc4, PORTAL_SHARED_NODE_CONSTRUCT, &[shape]);
    e.with_stack(4, |e, name| {
        e.call(
            FIXED_STRING_CONSTRUCT,
            &args![name, PORTAL_SHARED_GEOMETRY_NAME],
        );
        e.call(OBJECT_SET_NAME, &args![node, name]);
        e.call(FIXED_STRING_DESTRUCT, &args![name]);
    });
    fn_00549610(e, Ptr::new(shape), node);
    let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
    let graph_node = e.call(PORTAL_GRAPH_NODE, &args![graph]).u32();
    e.vcall(graph_node, NODE_SLOT_ATTACH_CHILD, &args![node, 1u32]);
    let list = reference_extra_list(e, reference);
    let links = e.call(EXTRA_LIST_GET_LINKS, &args![list]).u32();
    if links == 0 {
        return;
    }
    for i in 0..2u32 {
        let linked = e.mem.u32(links + 4 * i);
        if linked == 0 {
            let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
            e.call(PORTAL_GRAPH_ADD_PORTAL, &args![graph, shape]);
            continue;
        }
        let mut room = e.call(REFERENCE_GET_MULTI_BOUND_ROOM, &args![linked]).u32();
        if room == 0 {
            let list = reference_extra_list(e, Ptr::new(linked));
            let master = e.call(EXTRA_LIST_GET_MASTER_ROOM, &args![list]).u32();
            room = e.call(REFERENCE_GET_MULTI_BOUND_ROOM, &args![master]).u32();
        }
        if room != 0 {
            e.with_stack(4, |e, shape_slot| {
                e.mem.set_u32(shape_slot.addr(), shape);
                let target = e.call(ADDRESS_PLUS_B4, &args![room]).u32();
                e.call(PORTAL_TARGET_ADD, &args![target, shape_slot]);
            });
            if i == 0 {
                e.call(PORTAL_SET_FIRST_ROOM, &args![shape, room]);
            } else {
                e.call(PORTAL_SET_SECOND_ROOM, &args![shape, room]);
            }
        }
    }
}

/// The occlusion plane case of [`tes_object_cell_attach_reference_3d`]:
/// takes the reference's `BSOcclusionPlane` from its extra data (creating
/// and storing one on first use), gives it the reference's orientation and
/// position and, from the primitive's dimensions, its width and depth.
fn attach_occlusion_plane(e: &mut Engine, reference: Ptr) {
    let list = reference_extra_list(e, reference);
    let mut plane = e.call(EXTRA_LIST_GET_OCCLUSION_PLANE, &args![list]).u32();
    if plane == 0 {
        plane = construct_scene_object(e, 0xfc, OCCLUSION_PLANE_CONSTRUCT, &[]);
        let list = reference_extra_list(e, reference);
        e.call(EXTRA_LIST_SET_OCCLUSION_PLANE, &args![list, plane]);
    }
    let orientation = reference_orientation(e, reference);
    e.call(SHAPE_SET_ORIENTATION, &args![plane, orientation]);
    let position = e
        .vcall(reference.addr(), REFERENCE_SLOT_POSITION, &args![])
        .u32();
    e.call(SHAPE_SET_POSITION, &args![plane, position]);
    let list = reference_extra_list(e, reference);
    let primitive = e.call(EXTRA_LIST_GET_PRIMITIVE, &args![list]).u32();
    if primitive != 0 {
        e.with_stack(0x18, |e, buffer| {
            e.call(PRIMITIVE_GET_DIMENSIONS, &args![primitive, buffer]);
            let width = e.mem.f32(buffer.addr());
            let depth = e.mem.f32(buffer.addr() + 8);
            let size = buffer.byte_add(0xc);
            e.call(VECTOR2_CONSTRUCT, &args![size, width, depth]);
            e.call(PLANE_SET_SIZE, &args![plane, size]);
        });
    }
}

// Translated from 00549580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `iFormFlags & 0x40` of the form.
pub fn fn_00549580(e: &mut Engine, this: Ptr) -> bool {
    // TESForm::iFormFlags (Xbox PDB) +0x08
    e.mem.u32(this.addr() + 8) & 0x40 != 0
}

// Translated from 005495a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `sNumRefsWithVisibleDistant <= sNumLoadedRefsWithVisibleDistant` (signed
/// shorts).
pub fn fn_005495a0(e: &mut Engine, this: Ptr<TESObjectCELL>) -> bool {
    e.get(this, TESObjectCELL::sNumRefsWithVisibleDistant)
        <= e.get(this, TESObjectCELL::sNumLoadedRefsWithVisibleDistant)
}

// Translated from 005495d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The light marker node (`spLightMarkerNode`, `this + 0xB4`) of the cell.
pub fn fn_005495d0(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    e.call(SLOT_GET, &args![this.byte_add(0xb4)]).ptr()
}

// Translated from 005495f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The sound marker node (`spSoundMarkerNode`, `this + 0xB8`) of the cell.
pub fn fn_005495f0(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    e.call(SLOT_GET, &args![this.byte_add(0xb8)]).ptr()
}

// Translated from 00549610 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the smart pointer slot at `this + 0x104` (`ret 4`). The
/// object is the portal shape `AttachReference3D` builds, not a cell.
pub fn fn_00549610(e: &mut Engine, this: Ptr, value: u32) {
    e.call(SLOT_ASSIGN, &args![this.byte_add(0x104), value]);
}

// Translated from 00549630 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when none of the four `+0xA8` flag tests (`& 0x8`, `& 0x40`,
/// `& 0x80`, `& 0x100`) of the form is set; the tests stop at the first set
/// one.
pub fn fn_00549630(e: &mut Engine, this: Ptr) -> bool {
    for test in [FLAG_A8_8, FLAG_A8_40, FLAG_A8_80, FLAG_A8_100] {
        if e.call(test, &args![this]).bool() {
            return false;
        }
    }
    true
}

// Translated from 00549690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `(word at +0xC & 1) != 0` (through the mask accessor `00448a60`).
pub fn fn_00549690(e: &mut Engine, this: Ptr) -> bool {
    e.call(MASK_TEST, &args![this, 1u32]).u32() != 0
}

// Translated from 005496b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::PerformCellNodeAttach` (Xbox PDB), `ret 8`: under the
/// cell's reference lock, attaches the 3D of `reference` as a child of
/// `node` (or, with no node, detaches it from its parent), re-runs the
/// parent search ([`fn_0054a070`]) when `node` is null or neither child 4
/// of the cell's 3D nor a node whose virtual slot `0x14` answers non-zero,
/// and has the shadow scene node update the object's lighting.
pub fn tes_object_cell_perform_cell_node_attach(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    node: Ptr,
) {
    if reference.is_null() || e.call(REFERENCE_GET_3D, &args![reference]).u32() == 0 {
        return;
    }
    lock_enter(e, this);
    let node_3d = e.call(REFERENCE_GET_3D, &args![reference]).u32();
    if !node.is_null() {
        e.vcall(node.addr(), NODE_SLOT_ATTACH_CHILD, &args![node_3d, 1u32]);
    } else if e.call(NODE_PARENT, &args![node_3d]).u32() != 0 {
        let parent = e.call(NODE_PARENT, &args![node_3d]).u32();
        e.vcall(parent, NODE_SLOT_DETACH_CHILD, &args![node_3d]);
    }
    if node.is_null()
        || (node.addr() != fn_005497a0(e, this).addr()
            && e.vcall(node.addr(), NODE_SLOT_14, &args![]).u32() == 0)
    {
        fn_0054a070(e, this, reference, 1, 0);
    }
    let node_3d = e
        .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
        .u32();
    let scene = e.call(SHADOW_SCENE_NODE_GETTER, &args![0u32]).u32();
    e.call(
        SHADOW_SCENE_NODE_UPDATE_LIGHTING,
        &args![scene, node_3d, 0u32],
    );
    lock_leave(e, this);
}

// Translated from 005497a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Child 4 of the cell's 3D node.
pub fn fn_005497a0(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    Ptr::new(child_node(e, this, 4))
}

/// `attach_or_queue`: attaches `node` under the node `target` returns (as its
/// child, replacing), or, while attaches are queued, hands the target to the
/// queue instead.
fn attach_or_queue(e: &mut Engine, reference: Ptr, node: u32, target: impl Fn(&mut Engine) -> u32) {
    if !e.call(ATTACHES_ARE_QUEUED, &args![]).bool() {
        let target = target(e);
        e.vcall(target, NODE_SLOT_ATTACH_CHILD, &args![node, 1u32]);
    } else {
        let target = target(e);
        fn_0054abd0(e, reference, Ptr::new(target));
    }
}

/// Takes `node` out of the portal graph and off its parent (or, while
/// attaches are queued, queues the removal).
fn leave_portal_graph(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr, node: u32) {
    let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
    e.call(PORTAL_GRAPH_REMOVE_NODE, &args![graph, node]);
    if e.call(NODE_PARENT, &args![node]).u32() != 0 {
        if !e.call(ATTACHES_ARE_QUEUED, &args![]).bool() {
            let parent = e.call(NODE_PARENT, &args![node]).u32();
            e.vcall(parent, NODE_SLOT_DETACH_CHILD, &args![node]);
        } else {
            fn_0054abd0(e, reference, Ptr::NULL);
        }
    }
}

/// Puts `node` under child `index` of the cell's 3D (or queues it).
fn attach_under_child(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    node: u32,
    index: u32,
) {
    attach_or_queue(e, reference, node, |e| child_node(e, this, index));
}

/// Adds `node` to the portal graph (the graph's other add, `00c5b370`).
fn add_to_portal_graph(e: &mut Engine, this: Ptr<TESObjectCELL>, node: u32) {
    let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
    e.with_stack(4, |e, slot| {
        // A local smart pointer slot the original builds (and destroys) around
        // the call.
        e.call(SLOT_CONSTRUCT, &args![slot, 0u32]);
        e.call(PORTAL_GRAPH_ADD_NODE, &args![graph, node]);
        e.call(SLOT_RELEASE, &args![slot]);
    });
}

/// Either of the two placements `fn_005497c0` and `fn_0054a070` repeat: an
/// actor goes under child 0 of the cell's 3D, anything else leaves the
/// portal graph. Reads the actor test twice, as the original does.
fn place_by_kind(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr, node: u32) {
    if !is_actor(e, reference) {
        leave_portal_graph(e, this, reference, node);
    } else {
        attach_under_child(e, this, reference, node, 0);
    }
}

// Translated from 005497c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds the multibound room node of the cell's loaded data whose bound
/// contains `node` (a reference's 3D) and attaches `node` under it, using
/// `key`, the room's key in the loaded data's map at `+0x3C` (`ret 0x14`).
///
/// The position tested is the center of `node`'s world bound, or the center
/// of the reference's occlusion plane when it has one. Without
/// `check_bounds` an entry counts as found as soon as the room's slot `0x108`
/// test accepts the position; with it the room's slot `0x110` test of the
/// world bound may accept too. `*found_out` is set (and the result is true
/// unless `check_bounds` is set and no second room contained the node).
///
/// The unreachable frustum test (its flag is never set) and the NiFrustum
/// copy that only feeds it are left out.
pub fn fn_005497c0(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    node: u32,
    key: u32,
    found_out: Ptr,
    check_bounds: bool,
) -> bool {
    let bound = e.call(NODE_GET_WORLD_BOUND, &args![node]).u32();
    let center = e.call(BOUND_CENTER_ADDRESS, &args![bound]).u32();
    let mut position = [
        e.mem.u32(center),
        e.mem.u32(center + 4),
        e.mem.u32(center + 8),
    ];
    let mut from_plane = false;
    let list = reference_extra_list(e, reference);
    if e.call(EXTRA_LIST_GET_OCCLUSION_PLANE, &args![list]).u32() != 0 {
        let list = reference_extra_list(e, reference);
        let plane = e.call(EXTRA_LIST_GET_OCCLUSION_PLANE, &args![list]).u32();
        let center = e.call(PLANE_CENTER_ADDRESS, &args![plane]).u32();
        position = [
            e.mem.u32(center),
            e.mem.u32(center + 4),
            e.mem.u32(center + 8),
        ];
        from_plane = true;
    }
    e.with_stack(16, |e, block| {
        // `block` holds the room slot (+0) and the position (+4..+0x10).
        let room_slot = block;
        let position_ptr = block.byte_add(4);
        for (i, word) in position.iter().enumerate() {
            e.mem.set_u32(position_ptr.addr() + 4 * i as u32, *word);
        }
        e.call(SLOT_CONSTRUCT, &args![room_slot, 0u32]);
        let result = multi_bound_attach(
            e,
            this,
            reference,
            node,
            key,
            found_out,
            check_bounds,
            from_plane,
            room_slot,
            position_ptr,
        );
        e.call(SLOT_RELEASE, &args![room_slot]);
        result
    })
}

/// The body of [`fn_005497c0`] once the room slot and the position exist.
#[allow(clippy::too_many_arguments)]
fn multi_bound_attach(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    node: u32,
    key: u32,
    found_out: Ptr,
    check_bounds: bool,
    from_plane: bool,
    room_slot: Ptr,
    position: Ptr,
) -> bool {
    let loaded = e.get(this, TESObjectCELL::pLoadedData);
    e.call(
        MAP_GET_AT,
        &args![loaded.byte_add(LOADED_DATA_ROOM_MAP), key, room_slot],
    );
    let room_node = slot_get(e, room_slot);
    if room_node == 0 || e.vcall(room_node, ROOM_NODE_SLOT_CHILDREN, &args![]).u32() == 0 {
        return false;
    }
    if check_bounds {
        let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
        let list = e.call(PORTAL_GRAPH_LIST, &args![graph]).u32();
        if !e.call(LIST_COUNT_IS_ZERO, &args![list]).bool() {
            place_by_kind(e, this, reference, node);
            e.mem.set_u8(found_out.addr(), 1);
            return true;
        }
    }
    let room_node = slot_get(e, room_slot);
    if room_node == 0 {
        return false;
    }
    let room_node = slot_get(e, room_slot);
    if !e
        .vcall(room_node, ROOM_NODE_SLOT_CONTAINS_POINT, &args![position])
        .bool()
    {
        if !check_bounds {
            return false;
        }
        let room_node = slot_get(e, room_slot);
        let bound = e.call(NODE_GET_WORLD_BOUND, &args![node]).u32();
        if e.vcall(room_node, ROOM_NODE_SLOT_CONTAINS_BOUND, &args![bound])
            .u32()
            == 0
        {
            return false;
        }
    }
    e.call(REFERENCE_NOTE_ROOM, &args![reference, key]);
    if check_bounds && e.mem.u8(found_out.addr()) != 0 {
        place_by_kind(e, this, reference, node);
        return true;
    }

    let room_node = slot_get(e, room_slot);
    let children = e.vcall(room_node, ROOM_NODE_SLOT_CHILDREN, &args![]).u32();
    if from_plane {
        let list = reference_extra_list(e, reference);
        let plane = e.call(EXTRA_LIST_GET_OCCLUSION_PLANE, &args![list]).u32();
        e.call(ROOM_CHILDREN_ADD_PLANE, &args![children, plane]);
        return true;
    }
    let mut found = false;
    let mut multiple = false;
    let mut best = 0u32;
    let array = e.call(ADDRESS_PLUS_B4, &args![children]).u32();
    if e.call(ARRAY_COUNT, &args![array]).u32() != 0 {
        // The original builds a `NiFrustumPlanes` and copies the node's
        // frustum here; both only feed a frustum test whose flag is never
        // set, so they are not translated.
        let array = e.call(ADDRESS_PLUS_B4, &args![children]).u32();
        let mut cursor = e.call(SLOT_GET, &args![array]).u32();
        while cursor != 0 {
            let array = e.call(ADDRESS_PLUS_B4, &args![children]).u32();
            let item = e.with_stack(4, |e, cursor_slot| {
                e.mem.set_u32(cursor_slot.addr(), cursor);
                let item_address = e.call(CHILD_ARRAY_NEXT, &args![array, cursor_slot]).u32();
                cursor = e.mem.u32(cursor_slot.addr());
                e.mem.u32(item_address)
            });
            if item == 0 {
                continue;
            }
            let bound = e.call(NODE_GET_WORLD_BOUND, &args![node]).u32();
            if !e.call(ROOM_TEST_INTERSECTION, &args![item, bound]).bool() {
                continue;
            }
            if !found {
                best = item;
                found = true;
                continue;
            }
            multiple = true;
            let parent = e.call(NODE_PARENT, &args![node]).u32();
            if parent != fn_005497a0(e, this).addr() && !is_actor(e, reference) {
                place_by_kind(e, this, reference, node);
            }
            break;
        }
        if found && !multiple {
            let node_3d = e
                .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
                .u32();
            let parent = e.call(NODE_PARENT, &args![node_3d]).u32();
            if parent != e.call(ITEM_SLOT_GET, &args![best]).u32() {
                add_to_portal_graph(e, this, node);
                attach_or_queue(e, reference, node, |e| {
                    e.call(ITEM_SLOT_GET, &args![best]).u32()
                });
            }
        }
    }
    if !found {
        let parent = e.call(NODE_PARENT, &args![node]).u32();
        if parent != slot_get(e, room_slot) {
            add_to_portal_graph(e, this, node);
            attach_or_queue(e, reference, node, |e| slot_get(e, room_slot));
        }
    }
    e.mem.set_u8(found_out.addr(), 1);
    !check_bounds || multiple
}

// Translated from 0054a050 (decompiled, FalloutNV.exe 1.4.0.525)
/// Child 0 of the cell's 3D node.
pub fn fn_0054a050(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    Ptr::new(child_node(e, this, 0))
}

fn slot_get(e: &mut Engine, slot: Ptr) -> u32 {
    e.call(SLOT_GET, &args![slot]).u32()
}

// Translated from 0054a070 (decompiled, FalloutNV.exe 1.4.0.525)
/// Re-parents the loaded 3D of `reference` under the node of the cell's 3D
/// that fits it (`ret 0xc`). `update_all` forces the search even when the
/// reference does not need it; `check_bounds` (a byte) tests rooms by bound
/// instead of looking at the room the reference is keyed to.
///
/// Does nothing without a 3D, for the forms at `011ca23c` and `011ca238`,
/// or when the 3D already hangs under child 4. Otherwise, when the setting
/// at `011ca118` has text, the reference has no room key (extra data entry
/// `0x63`) and its base form type is not one of 0x2A..0x2D, 0x33 or 0x51 (it
/// "needs" a room), or when `update_all` is set, the 3D is placed by
/// trying, in order:
///
/// 1. the room of the reference's key and of the references linked to it
///    ([`fn_005497c0`]), unless `check_bounds`;
/// 2. every room of the loaded data's map at `+0x3C` that was not visited
///    yet (`walk_room_map`);
/// 3. the portal graph's rooms: the single one whose bound intersects the
///    3D takes it, several send it under child 4;
/// 4. child 0 of the cell's 3D for an actor, child 3 otherwise.
///
/// Finally a reference with an occlusion plane and no key hands the plane
/// to the portal graph. The compiler's exception frame is not translated.
pub fn fn_0054a070(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    update_all: u8,
    check_bounds: u8,
) {
    if reference.is_null()
        || e.vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
            .u32()
            == 0
    {
        return;
    }
    if base_form(e, reference).addr() == e.global::<u32>(PORTAL_MARKER_FORM_23C)
        || base_form(e, reference).addr() == e.global::<u32>(ROOM_MARKER_FORM_238)
    {
        return;
    }
    let node_3d = e
        .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
        .u32();
    let parent = e.call(NODE_PARENT, &args![node_3d]).u32();
    if parent == fn_005497a0(e, this).addr() {
        return;
    }
    let mut needs = false;
    if e.call(SETTING_FIRST_CHAR, &args![FEATURE_SETTING]).bool() {
        let list = reference_extra_list(e, reference);
        if e.call(EXTRA_LIST_GET_ROOM_KEY, &args![list]).u32() == 0 {
            let base = base_form(e, reference);
            let kind = form_type(e, base);
            // The compiler's switch: a byte table over `kind - 0x2A` says
            // "no" for these kinds and "needs" for every other.
            needs = !matches!(kind, 0x2a..=0x2d | 0x33 | 0x51);
        }
    }
    if needs || update_all != 0 {
        place_loaded_3d(
            e,
            this,
            reference,
            update_all != 0,
            needs,
            check_bounds != 0,
        );
    }
    let list = reference_extra_list(e, reference);
    if e.call(EXTRA_LIST_GET_OCCLUSION_PLANE, &args![list]).u32() != 0 {
        let list = reference_extra_list(e, reference);
        if e.call(EXTRA_LIST_GET_ROOM_KEY, &args![list]).u32() == 0 {
            let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
            let list = reference_extra_list(e, reference);
            let plane = e.call(EXTRA_LIST_GET_OCCLUSION_PLANE, &args![list]).u32();
            e.call(PORTAL_GRAPH_ADD_PLANE, &args![graph, plane]);
        }
    }
}

/// The part of [`fn_0054a070`] under `update_all || needs`.
fn place_loaded_3d(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    update_all: bool,
    needs: bool,
    check_bounds: bool,
) {
    let node_3d = e
        .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
        .u32();
    // The function's locals: the node slot (+0), the scrap map of visited
    // keys (+4, 0x38 bytes), the found byte (+0x3C) and the scratch byte
    // (+0x3D).
    e.with_stack(0x40, |e, block| {
        let node_slot = block;
        let scrap_map = block.byte_add(4);
        let found_out = block.byte_add(0x3c);
        let scratch = block.byte_add(0x3d);
        e.call(SLOT_CONSTRUCT, &args![node_slot, node_3d]);
        let player = e.global::<u32>(PLAYER_POINTER);
        if reference.addr() == player {
            let current = e.call(PLAYER_GET_CURRENT_3D_FIRST, &args![player]).u32();
            e.call(SLOT_ASSIGN, &args![node_slot, current]);
        }
        let mut placed = false;
        e.call(SCRAP_MAP_CONSTRUCT, &args![scrap_map, 0x25u32, 0u32]);
        let list = reference_extra_list(e, reference);
        let key = e.call(EXTRA_LIST_GET_ROOM_KEY, &args![list]).u32();
        if key != 0 && !check_bounds {
            e.call(SCRAP_MAP_SET, &args![scrap_map, key, 1u32]);
            let node = slot_get(e, node_slot);
            placed = fn_005497c0(e, this, reference, node, key, found_out, check_bounds);
            if !placed {
                let owner = reference_extra_list(e, Ptr::new(key));
                let mut cursor = e.call(EXTRA_LIST_GET_LINK_LIST, &args![owner]).u32();
                while cursor != 0 {
                    let item_address = e.call(LIST_ITEM_ADDRESS, &args![cursor]).u32();
                    let item = e.mem.u32(item_address);
                    if item != 0 {
                        let list = reference_extra_list(e, Ptr::new(item));
                        let linked = e.call(EXTRA_LIST_GET_LINKED_KEY, &args![list, 0u32]).u32();
                        if linked != 0 {
                            e.call(SCRAP_MAP_SET, &args![scrap_map, linked, 1u32]);
                            let node = slot_get(e, node_slot);
                            placed = fn_005497c0(
                                e,
                                this,
                                reference,
                                node,
                                linked,
                                found_out,
                                check_bounds,
                            );
                        }
                        if !placed {
                            let list = reference_extra_list(e, Ptr::new(item));
                            let linked =
                                e.call(EXTRA_LIST_GET_LINKED_KEY, &args![list, 1u32]).u32();
                            if linked != 0 {
                                let node = slot_get(e, node_slot);
                                placed = fn_005497c0(
                                    e,
                                    this,
                                    reference,
                                    node,
                                    linked,
                                    found_out,
                                    check_bounds,
                                );
                                e.call(SCRAP_MAP_SET, &args![scrap_map, linked, 1u32]);
                            }
                        }
                    }
                    cursor = e.call(LIST_NEXT, &args![cursor]).u32();
                }
            }
        }
        if !placed {
            placed = walk_room_map(
                e,
                this,
                reference,
                RoomWalk {
                    update_all,
                    needs,
                    check_bounds,
                    node_slot,
                    scrap_map,
                    found_out,
                    scratch,
                },
            );
        }
        if !placed && e.mem.u8(found_out.addr()) == 0 {
            placed = search_portal_graph(e, this, reference);
        }
        if !placed && e.mem.u8(found_out.addr()) == 0 {
            if is_actor(e, reference) {
                let node_3d = e
                    .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
                    .u32();
                attach_under_child(e, this, reference, node_3d, 0);
            } else {
                let node_3d = e
                    .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
                    .u32();
                attach_under_child(e, this, reference, node_3d, 3);
            }
        }
        e.call(SCRAP_MAP_RELEASE, &args![scrap_map]);
        e.call(SLOT_RELEASE, &args![node_slot]);
    });
}

/// The locals `walk_room_map` shares with [`place_loaded_3d`].
struct RoomWalk {
    update_all: bool,
    needs: bool,
    check_bounds: bool,
    node_slot: Ptr,
    scrap_map: Ptr,
    found_out: Ptr,
    scratch: Ptr,
}

/// Walks the loaded data's room map (`+0x3C`, room key to room node) until a
/// room takes the reference's 3D. Returns whether one did.
fn walk_room_map(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr, w: RoomWalk) -> bool {
    let loaded = e.get(this, TESObjectCELL::pLoadedData);
    let map = loaded.byte_add(LOADED_DATA_ROOM_MAP);
    let mut placed = false;
    // The map position (+0) and the key (+4) the iteration fills.
    e.with_stack(8, |e, position| {
        let key_ptr = position.byte_add(4);
        let first = e.call(MAP_FIRST_POSITION, &args![map]).u32();
        e.mem.set_u32(position.addr(), first);
        while e.mem.u32(position.addr()) != 0 && !placed {
            // The entry's room goes to a local smart pointer slot.
            e.with_stack(4, |e, slot| {
                e.call(SLOT_CONSTRUCT, &args![slot, 0u32]);
                e.mem.set_u32(key_ptr.addr(), 0);
                e.call(MAP_GET_NEXT, &args![map, position, key_ptr, slot]);
                let key = e.mem.u32(key_ptr.addr());
                walk_room_entry(e, this, reference, &w, key, slot, &mut placed);
                e.call(SLOT_RELEASE, &args![slot]);
            });
        }
    });
    placed
}

/// One entry of [`walk_room_map`]; sets `placed` when a room took the 3D (the
/// walk then ends).
fn walk_room_entry(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    reference: Ptr,
    w: &RoomWalk,
    key: u32,
    slot: Ptr,
    placed: &mut bool,
) {
    if key == 0 || slot_get(e, slot) == 0 {
        return;
    }
    if e.call(SCRAP_MAP_GET, &args![w.scrap_map, key, w.scratch])
        .bool()
    {
        return;
    }
    if !w.needs && w.update_all {
        let room = slot_get(e, slot);
        if e.vcall(room, ROOM_NODE_SLOT_CHILDREN, &args![]).u32() == 0 {
            return;
        }
    }
    let room = slot_get(e, slot);
    let data = e.call(SLOT_AT_AC_GET, &args![room]).u32();
    if data == 0 {
        return;
    }
    if e.call(REFERENCE_CHECKS_ROOM_DATA, &args![reference, data])
        .bool()
        && !w.update_all
    {
        let parent = e.call(REFERENCE_PARENT_CELL, &args![reference]).u32();
        let target = e.call(CELL_NODE_FOR_TARGET, &args![parent, key]).u32();
        if target != 0 {
            let node_3d = e
                .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
                .u32();
            e.vcall(target, NODE_SLOT_ATTACH_CHILD, &args![node_3d, 1u32]);
            *placed = true;
            e.call(REFERENCE_NOTE_ROOM, &args![reference, key]);
        }
        return;
    }
    let room = slot_get(e, slot);
    if e.vcall(room, ROOM_NODE_SLOT_CHILDREN, &args![]).u32() != 0 {
        let node = slot_get(e, w.node_slot);
        if fn_005497c0(e, this, reference, node, key, w.found_out, w.check_bounds) {
            *placed = true;
            return;
        }
        return;
    }
    if e.call(REFERENCE_FITS_ROOM_DATA, &args![reference, data])
        .bool()
    {
        e.call(
            QUEUED_ROOM_MAP_SET_AT,
            &args![ROOM_TASK_MAP, reference, key],
        );
        let node_3d = e
            .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
            .u32();
        let task = e.call(ALLOCATE, &args![0x28u32]).u32();
        let task = if task == 0 {
            Ptr::NULL
        } else {
            fn_0054aa80(e, Ptr::new(task), key, node_3d)
        };
        let manager = e.global::<u32>(TASK_MANAGER_POINTER);
        e.vcall(manager, TASK_MANAGER_SLOT_QUEUE, &args![task]);
        add_to_portal_graph_of_cell(e, this, reference);
    }
}

/// The tail of a room entry that queued a task: adds the reference's 3D to
/// the cell's portal graph (`00c5b370`).
fn add_to_portal_graph_of_cell(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr) {
    let graph = e.call(CELL_PORTAL_GRAPH, &args![this]).u32();
    e.with_stack(4, |e, slot| {
        e.call(SLOT_CONSTRUCT, &args![slot, 0u32]);
        let node_3d = e
            .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
            .u32();
        e.call(PORTAL_GRAPH_ADD_NODE, &args![graph, node_3d]);
        e.call(SLOT_RELEASE, &args![slot]);
    });
}

/// The portal graph search of [`fn_0054a070`]: the single room of the cell's
/// portal graph whose bound intersects the 3D takes it; if several do it goes
/// under child 4. Returns whether it placed the 3D.
fn search_portal_graph(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr) -> bool {
    let graph = e.call(CELL_PORTAL_GRAPH_CREATE, &args![this]).u32();
    let list = e.call(PORTAL_GRAPH_LIST, &args![graph]).u32();
    let first = e.call(SLOT_GET, &args![list]).u32();
    let mut best = 0u32;
    let mut seen = false;
    let mut multiple = false;
    let node_3d = e
        .vcall(reference.addr(), REFERENCE_SLOT_GET_3D, &args![])
        .u32();
    e.with_stack(4, |e, cursor| {
        e.mem.set_u32(cursor.addr(), first);
        while e.mem.u32(cursor.addr()) != 0 && !multiple {
            let list = e.call(PORTAL_GRAPH_LIST, &args![graph]).u32();
            let item_address = e.call(CHILD_ARRAY_NEXT, &args![list, cursor]).u32();
            let item = e.mem.u32(item_address);
            let bound = e.call(NODE_GET_WORLD_BOUND, &args![node_3d]).u32();
            if e.call(ROOM_TEST_INTERSECTION, &args![item, bound]).bool() {
                if !seen {
                    seen = true;
                    best = item;
                } else {
                    best = 0;
                    multiple = true;
                }
            }
        }
    });
    if best != 0 {
        e.with_stack(4, |e, slot| {
            e.call(SLOT_CONSTRUCT, &args![slot, 0u32]);
            e.call(PORTAL_GRAPH_ADD_NODE, &args![graph, node_3d]);
            attach_or_queue(e, reference, node_3d, |e| {
                e.call(ITEM_SLOT_GET, &args![best]).u32()
            });
            e.call(SLOT_RELEASE, &args![slot]);
        });
        return true;
    }
    if multiple {
        if e.call(NODE_PARENT, &args![node_3d]).u32() != fn_005497a0(e, this).addr()
            && !is_actor(e, reference)
        {
            place_by_kind(e, this, reference, node_3d);
        }
        return true;
    }
    false
}

// Translated from 0054aa60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Child 3 of the cell's 3D node.
pub fn fn_0054aa60(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    Ptr::new(child_node(e, this, 3))
}

// Translated from 0054aa80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CheckWithinMultiBoundTask::CheckWithinMultiBoundTask` (the class named
/// by the vtable's RTTI; the map names the destructor): `reference` goes to
/// `+0x18`, `node` to the smart pointer slot at `+0x1C`, the slot at `+0x20`
/// takes the reference's multibound, and the byte at `+0x24` is cleared
/// (`ret 8`). Returns `this`.
///
/// The compiler's exception frame is not translated.
pub fn check_within_multi_bound_task_check_within_multi_bound_task(
    e: &mut Engine,
    this: Ptr,
    reference: Ptr,
    node: u32,
) -> Ptr {
    e.call(TASK_BASE_CONSTRUCT, &args![this, 4u32]);
    e.mem
        .set_u32(this.addr(), CHECK_WITHIN_MULTI_BOUND_TASK_VTABLE);
    e.mem.set_u32(this.addr() + 0x18, reference.addr());
    e.call(SLOT_CONSTRUCT, &args![this.byte_add(0x1c), node]);
    e.call(SLOT_CONSTRUCT, &args![this.byte_add(0x20), 0u32]);
    e.mem.set_u8(this.addr() + 0x24, 0);
    let multi_bound = e.call(REFERENCE_GET_MULTI_BOUND, &args![reference]).u32();
    e.call(SLOT_ASSIGN, &args![this.byte_add(0x20), multi_bound]);
    this
}

/// The constructor under its address-form name (see
/// [`check_within_multi_bound_task_check_within_multi_bound_task`]).
fn fn_0054aa80(e: &mut Engine, this: Ptr, reference: u32, node: u32) -> Ptr {
    check_within_multi_bound_task_check_within_multi_bound_task(e, this, Ptr::new(reference), node)
}

// Translated from 0054ab30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CheckWithinMultiBoundTask::_scalar_deleting_destructor_` (Xbox PDB), `ret 4`:
/// runs the destructor body and frees the object when bit 0 of `flags` is
/// set. Returns `this`.
pub fn check_within_multi_bound_task_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    fn_0054ab60(e, this);
    if flags & 1 != 0 {
        e.call(DEALLOCATE, &args![this]);
    }
    this
}

// Translated from 0054ab60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor body of `CheckWithinMultiBoundTask`: releases the slots at
/// `+0x20` and `+0x1C`, then runs the base destructor. The compiler's
/// exception frame is not translated.
pub fn fn_0054ab60(e: &mut Engine, this: Ptr) {
    e.call(SLOT_RELEASE, &args![this.byte_add(0x20)]);
    e.call(SLOT_RELEASE, &args![this.byte_add(0x1c)]);
    e.call(TASK_BASE_DESTRUCT, &args![this]);
}

// Translated from 0054abd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cdecl`: queues the attach of `reference`'s 3D under `node` for
/// [`fn_0054ae30`]. Under the queue's semaphore (`011ca1d4`, built on the
/// first use with count 1 and registered for destruction at exit): if the 3D
/// already hangs under `node` its entry is removed from the queue map
/// (`011ca0f4`); otherwise the map entry for that 3D becomes
/// `(3D, node, true)`.
///
/// The compiler's exception frame is not translated.
pub fn fn_0054abd0(e: &mut Engine, reference: Ptr, node: Ptr) {
    if reference.is_null() || e.call(REFERENCE_GET_3D, &args![reference]).u32() == 0 {
        return;
    }
    let initialized = e.global::<u32>(QUEUE_SEMAPHORE_GUARD);
    if initialized & 1 == 0 {
        e.set_global(QUEUE_SEMAPHORE_GUARD, initialized | 1);
        fn_0054ad60(e, Ptr::new(QUEUE_SEMAPHORE), 1);
        e.call(CRT_ATEXIT, &args![QUEUE_SEMAPHORE_DESTRUCTOR]);
    }
    e.call(SEMAPHORE_WAIT, &args![QUEUE_SEMAPHORE]);
    let node_3d = e.call(REFERENCE_GET_3D, &args![reference]).u32();
    if e.call(NODE_PARENT, &args![node_3d]).u32() == node.addr() {
        e.call(
            QUEUED_ATTACH_MAP_REMOVE_AT,
            &args![QUEUED_ATTACH_MAP, node_3d],
        );
    } else {
        e.with_stack(12, |e, entry| {
            fn_0054acf0(e, entry);
            e.call(SLOT_ASSIGN, &args![entry.byte_add(4), node.addr()]);
            e.call(SLOT_ASSIGN, &args![entry, node_3d]);
            e.with_stack(12, |e, copy| {
                fn_0054adb0(e, copy, entry);
                let words = [
                    e.mem.u32(copy.addr()),
                    e.mem.u32(copy.addr() + 4),
                    e.mem.u32(copy.addr() + 8),
                ];
                e.call(
                    QUEUED_ATTACH_MAP_SET_AT,
                    &args![QUEUED_ATTACH_MAP, node_3d, words[0], words[1], words[2]],
                );
            });
            e.call(ATTACH_ENTRY_DESTRUCT, &args![entry]);
        });
    }
    e.call(SEMAPHORE_RELEASE, &args![QUEUE_SEMAPHORE]);
}

// Translated from 0054acf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructs a queue entry at `this`: two empty smart pointer slots (`+0`
/// and `+4`) and the byte at `+8` set. Returns `this`. The compiler's
/// exception frame is not translated.
pub fn fn_0054acf0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(SLOT_CONSTRUCT, &args![this, 0u32]);
    e.call(SLOT_CONSTRUCT, &args![this.byte_add(4), 0u32]);
    e.mem.set_u8(this.addr() + 8, 1);
    this
}

// Translated from 0054ad60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the semaphore wrapper at `this` (`ret 4`): the count goes to `+0`,
/// `count + 1` to `+8`, and the Windows semaphore (`0, count, count + 1, 0`)
/// to `+4`. Returns `this`.
pub fn fn_0054ad60(e: &mut Engine, this: Ptr, count: u32) -> Ptr {
    e.mem.set_u32(this.addr(), count);
    e.mem.set_u32(this.addr() + 8, count.wrapping_add(1));
    let initial = e.mem.u32(this.addr());
    let maximum = e.mem.u32(this.addr() + 8);
    let handle = e
        .call(CREATE_SEMAPHORE, &args![0u32, initial, maximum, 0u32])
        .u32();
    e.mem.set_u32(this.addr() + 4, handle);
    this
}

// Translated from 0054adb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copy constructor of the queue entry (`ret 4`): copies both slots and the
/// byte at `+8` from `source`. Returns `this`. The compiler's exception
/// frame is not translated.
pub fn fn_0054adb0(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    e.call(SLOT_COPY_CONSTRUCT, &args![this, source]);
    e.call(
        SLOT_COPY_CONSTRUCT,
        &args![this.byte_add(4), source.byte_add(4)],
    );
    let flag = e.mem.u8(source.addr() + 8);
    e.mem.set_u8(this.addr() + 8, flag);
    this
}

// Translated from 0054ae30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::PerformQueuedChildAttaches` (Xbox PDB): walks the queue
/// map (`011ca0f4`); for every entry whose 3D (first slot) hangs under
/// something other than the queued node (second slot), looks the reference
/// of that 3D up and, when it has a parent cell, runs that cell's
/// [`tes_object_cell_perform_cell_node_attach`] with the queued node. Then
/// the map is emptied. The compiler's exception frame is not translated.
pub fn tes_object_cell_perform_queued_child_attaches(e: &mut Engine) {
    let first = e.call(MAP_FIRST_POSITION, &args![QUEUED_ATTACH_MAP]).u32();
    // The map position (+0) and the key (+4) the iteration fills.
    e.with_stack(8, |e, cursor| {
        e.mem.set_u32(cursor.addr(), first);
        while e.mem.u32(cursor.addr()) != 0 {
            e.with_stack(12, |e, entry| {
                fn_0054acf0(e, entry);
                e.mem.set_u32(cursor.addr() + 4, 0);
                e.call(
                    QUEUED_ATTACH_MAP_GET_NEXT,
                    &args![QUEUED_ATTACH_MAP, cursor, cursor.byte_add(4), entry],
                );

                let node_3d = slot_get(e, entry);
                if node_3d != 0 {
                    let parent = e.call(NODE_PARENT, &args![node_3d]).u32();
                    let queued = slot_get(e, entry.byte_add(4));
                    if parent != queued {
                        let node_3d = slot_get(e, entry);
                        let reference = e.call(FIND_REFERENCE_FOR_3D, &args![node_3d]).ptr::<()>();
                        if !reference.is_null()
                            && !e
                                .call(REFERENCE_PARENT_CELL, &args![reference])
                                .ptr::<()>()
                                .is_null()
                        {
                            let node = slot_get(e, entry.byte_add(4));
                            let cell = e
                                .call(REFERENCE_PARENT_CELL, &args![reference])
                                .ptr::<TESObjectCELL>();
                            tes_object_cell_perform_cell_node_attach(
                                e,
                                cell,
                                reference,
                                Ptr::new(node),
                            );
                        }
                    }
                }
                e.call(ATTACH_ENTRY_DESTRUCT, &args![entry]);
            });
        }
    });
    e.call(QUEUED_ATTACH_MAP_CLEAR, &args![QUEUED_ATTACH_MAP]);
}

// Translated from 0054af40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Under the cell's reference lock, hands the cell's list of references and
/// `flag` to the process lists (`0096e150`), `ret 4`.
pub fn fn_0054af40(e: &mut Engine, this: Ptr<TESObjectCELL>, flag: u8) {
    lock_enter(e, this);
    let list = e.call(CELL_REFERENCE_LIST, &args![this]).u32();
    e.call(
        PROCESS_LISTS_HANDLE_CELL_REFERENCES,
        &args![PROCESS_LISTS, list, flag as u32],
    );
    lock_leave(e, this);
}

// Translated from 0054af80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Under the cell's reference lock, hands the cell's list of references to
/// the process lists (`0096dcb0`).
pub fn fn_0054af80(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    lock_enter(e, this);
    let list = e.call(CELL_REFERENCE_LIST, &args![this]).u32();
    e.call(
        PROCESS_LISTS_REMOVE_CELL_REFERENCES,
        &args![PROCESS_LISTS, list],
    );
    lock_leave(e, this);
}

// Translated from 0054afb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cdecl(a, b, c)`: a random reference of the cell (or the world space's
/// persistent cell) `a` that passes the filter, or null. `c` is only tested
/// for null. A reference qualifies when its base form has type 0x1C, it has
/// none of the form flags `0x20`, `0x800` and `0x800000`, its base form casts
/// to the type at `01186530` and that object accepts `b` (`00519500`), and
/// `00568e50` finds nothing for it. One of the qualifying references is
/// picked at random (`00944460(0, count)`).
///
/// The compiler's exception frame is not translated.
pub fn fn_0054afb0(e: &mut Engine, a: Ptr, b: Ptr, c: Ptr) -> Ptr {
    let mut result = Ptr::NULL;
    if a.is_null() || b.is_null() || c.is_null() || a == b {
        return result;
    }
    e.with_stack(8, |e, list| {
        e.call(LIST_CONSTRUCT, &args![list]);
        let mut count = 0u32;
        let mut cursor = 0u32;
        let mut owner = 0u32;
        let cell = dynamic_cast(e, a, RTTI_TES_FORM, RTTI_CELL);
        if !cell.is_null() {
            cursor = e.call(CELL_REFERENCE_LIST, &args![cell]).u32();
            owner = cell.addr();
        } else {
            let world = dynamic_cast(e, a, RTTI_TES_FORM, RTTI_WORLD_SPACE);
            if !world.is_null() && e.call(WORLD_SPACE_PERSISTENT_CELL, &args![world]).u32() != 0 {
                let persistent = e.call(WORLD_SPACE_PERSISTENT_CELL, &args![world]).u32();
                cursor = e.call(CELL_REFERENCE_LIST, &args![persistent]).u32();
                owner = e.call(WORLD_SPACE_PERSISTENT_CELL, &args![world]).u32();
            }
        }
        lock_enter(e, Ptr::new(owner));
        while cursor != 0 && !e.call(LIST_IS_END, &args![cursor]).bool() {
            let item_address = e.call(LIST_ITEM_ADDRESS, &args![cursor]).u32();
            let current = Ptr::new(e.mem.u32(item_address));
            let base = base_form(e, current);
            if form_type(e, base) == FORM_TYPE_1C
                && !e.call(FORM_FLAG_20, &args![current]).bool()
                && !e.call(FORM_FLAG_800, &args![current]).bool()
                && !e.call(FORM_FLAG_800000, &args![current]).bool()
            {
                let base = base_form(e, current);
                let cast = dynamic_cast(e, base, RTTI_FORM_FILTER_FROM, RTTI_FORM_FILTER_TO);
                if !cast.is_null()
                    && e.call(FILTER_OBJECT_USABLE, &args![cast]).bool()
                    && e.call(FILTER_OBJECT_ACCEPTS, &args![cast, b]).bool()
                    && e.call(REFERENCE_BLOCKED, &args![current]).u32() == 0
                {
                    push_front_value(e, list.addr(), current.addr());
                    count += 1;
                }
            }
            cursor = e.call(LIST_NEXT, &args![cursor]).u32();
        }
        lock_leave(e, Ptr::new(owner));
        if count > 0 && !e.call(LIST_IS_END, &args![list]).bool() {
            let pick = e.call(RANDOM_BELOW, &args![0u32, count]).u32();
            let mut index = 0u32;
            let mut cursor = list.addr();
            while cursor != 0 && !e.call(LIST_IS_END, &args![cursor]).bool() {
                if index == pick {
                    let item_address = e.call(LIST_ITEM_ADDRESS, &args![cursor]).u32();
                    result = Ptr::new(e.mem.u32(item_address));
                    break;
                }
                index += 1;
                cursor = e.call(LIST_NEXT, &args![cursor]).u32();
            }
            e.call(LIST_CLEAR, &args![list]);
        }
        e.call(LIST_DESTRUCT, &args![list]);
    });
    result
}

// Translated from 0054b260 (decompiled, FalloutNV.exe 1.4.0.525)
/// Under the cell's reference lock, for every reference of the cell that is
/// owned by `item` (not temporary, not a merchant container of the item),
/// copies what it holds into `destination`, `ret 8`: for the container-like
/// base form types (0x18, 0x19, 0x1A, 0x1D, 0x1E, 0x1F, 0x28, 0x29, 0x2E,
/// 0x2F, 0x32, 0x67, 0x6C, 0x73, 0x74) a new `ItemChange` of the base form
/// holding a copy of the reference's extra data (count and ownership
/// cleared); type 0x73 first sets the reference's count to 1. For type 0x1B
/// the container changes of the reference are merged into `destination`.
///
/// The compiler's switch maps `type - 0x18` through a byte table to four
/// targets; the table is spelled out in the `match`.
pub fn fn_0054b260(e: &mut Engine, this: Ptr<TESObjectCELL>, item: Ptr, destination: Ptr) {
    if destination.is_null() || item.is_null() {
        return;
    }
    let list = reference_extra_list(e, item);
    let merchant = e
        .call(EXTRA_LIST_GET_MERCHANT_CONTAINER, &args![list])
        .u32();
    lock_enter(e, this);
    let mut cursor = e.call(CELL_REFERENCE_LIST, &args![this]).u32();
    while cursor != 0 {
        let item_address = e.call(LIST_ITEM_ADDRESS, &args![cursor]).u32();
        if e.mem.u32(item_address) == 0 {
            break;
        }
        let item_address = e.call(LIST_ITEM_ADDRESS, &args![cursor]).u32();
        let current = Ptr::new(e.mem.u32(item_address));
        if !e.call(FORM_FLAG_800, &args![current]).bool()
            && !e.call(FORM_FLAG_20, &args![current]).bool()
            && e.call(REFERENCE_GET_OWNER, &args![current]).u32() != 0
            && e.call(REFERENCE_IS_AN_OWNER, &args![current, item, 0u32])
                .bool()
            && current.addr() != merchant
        {
            let base = base_form(e, current);
            let kind = form_type(e, base);
            match kind {
                0x73 => {
                    let list = reference_extra_list(e, current);
                    if list != 0 {
                        let list = reference_extra_list(e, current);
                        e.call(EXTRA_LIST_SET_COUNT, &args![list, 1u32]);
                    }
                    copy_to_destination(e, current, base, destination);
                }
                0x18 | 0x19 | 0x1a | 0x1d | 0x1e | 0x1f | 0x28 | 0x29 | 0x2e | 0x2f | 0x32
                | 0x67 | 0x6c | 0x74 => {
                    copy_to_destination(e, current, base, destination);
                }
                0x1b => {
                    let list = reference_extra_list(e, current);
                    let changes = e.call(EXTRA_LIST_GET_CONTAINER_CHANGES, &args![list]).u32();
                    if changes != 0 {
                        e.call(
                            CONTAINER_CHANGES_MERGE,
                            &args![changes, destination, current, 0u32],
                        );
                    }
                }
                _ => {}
            }
        }
        cursor = e.call(LIST_NEXT, &args![cursor]).u32();
    }
    lock_leave(e, this);
}

/// The container-like case of [`fn_0054b260`]: builds an `ItemChange` for
/// `base` (with its list created on demand), gives it a copy of the
/// reference's extra data, and adds it to `destination` with the
/// reference's count.
fn copy_to_destination(e: &mut Engine, reference: Ptr, base: Ptr, destination: Ptr) {
    let block = e.call(ALLOCATE, &args![0xcu32]).u32();
    let change = if block != 0 {
        e.call(ITEM_CHANGE_CONSTRUCT, &args![block, base, 0u32])
            .u32()
    } else {
        0
    };
    if e.mem.u32(change) == 0 {
        let block = e.call(ALLOCATE, &args![8u32]).u32();
        let new_list = if block != 0 {
            e.call(LIST_CONSTRUCT, &args![block]).u32()
        } else {
            0
        };
        e.mem.set_u32(change, new_list);
    }
    let block = e.call(ALLOCATE, &args![0x20u32]).u32();
    let extras = if block != 0 {
        e.call(EXTRA_LIST_CONSTRUCT, &args![block]).u32()
    } else {
        0
    };
    let source = reference_extra_list(e, reference);
    e.call(EXTRA_LIST_DUPLICATE_FOR_CONTAINER, &args![extras, source]);
    e.call(EXTRA_LIST_SET_REFERENCE, &args![extras, reference]);
    e.call(EXTRA_LIST_REMOVE_OWNERSHIP, &args![extras]);
    let change_list = e.mem.u32(change);
    push_front_value(e, change_list, extras);
    let source = reference_extra_list(e, reference);
    let count = e.call(EXTRA_LIST_GET_COUNT, &args![source]).u32() as u16 as i16 as i32;
    e.call(ITEM_CHANGE_SET_COUNT_DELTA, &args![change, count]);
    e.call(CHANGES_ADD, &args![destination, change, 1u32]);
}

// Translated from 0054b5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::SaveGameTest` (Xbox PDB): under the cell's reference lock,
/// for every reference other than the player (the global at `011dea3c`):
/// lets the reference's container changes act on the player (`004ce380`);
/// casts the reference to the type at `011846d4`; for such an object without
/// form flag `0x800` or `0x20` calls its virtual slot `0x460` with the player
/// and the constant at `01017b78`; if it is also not essential
/// (`0087f3d0`, named `Actor::GetEssential` by the map) and slot `0x22c(0)`
/// answers no, calls slot `0x424` and `0089d900` (named `Actor::Kill`); unlocks
/// the reference if it is locked; finally calls `00555c20`.
pub fn tes_object_cell_save_game_test(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    lock_enter(e, this);
    let mut cursor = e.call(CELL_REFERENCE_LIST, &args![this]).u32();
    while cursor != 0 {
        let item_address = e.call(LIST_ITEM_ADDRESS, &args![cursor]).u32();
        let current = Ptr::new(e.mem.u32(item_address));
        let player = e.global::<u32>(PLAYER_POINTER);
        if !current.is_null() && current.addr() != player {
            let actor = dynamic_cast(e, current, RTTI_REFERENCE, RTTI_ACTOR_LIKE);
            let list = reference_extra_list(e, current);
            let changes = e.call(EXTRA_LIST_GET_CONTAINER_CHANGES, &args![list]).u32();
            if changes != 0 {
                let player = e.global::<u32>(PLAYER_POINTER);
                e.call(
                    CONTAINER_CHANGES_UPDATE,
                    &args![
                        changes,
                        current,
                        player,
                        0u32,
                        0u32,
                        1u32,
                        0u32,
                        u32::MAX,
                        0u32
                    ],
                );
            }
            if !actor.is_null()
                && !e.call(FORM_FLAG_800, &args![actor]).bool()
                && !e.call(FORM_FLAG_20, &args![actor]).bool()
            {
                let constant = e.global::<f32>(SAVE_TEST_CONSTANT);
                let player = e.global::<u32>(PLAYER_POINTER);
                e.vcall(actor.addr(), ACTOR_SLOT_460, &args![player, constant]);
            }
            if !actor.is_null()
                && !e.call(ACTOR_IS_ESSENTIAL, &args![actor]).bool()
                && !e.call(FORM_FLAG_800, &args![actor]).bool()
                && !e.call(FORM_FLAG_20, &args![actor]).bool()
                && !e.vcall(actor.addr(), ACTOR_SLOT_22C, &args![0u32]).bool()
            {
                let player = e.global::<u32>(PLAYER_POINTER);
                e.vcall(
                    actor.addr(),
                    ACTOR_SLOT_424,
                    &args![player, 0u32, 0u32, 0u32, 0u32, 0u32, 1u32, 0u32],
                );
                e.call(ACTOR_KILL, &args![actor, 0u32, 0.0f32]);
            }
            if e.call(REFERENCE_GET_LOCK, &args![current]).u32() != 0 {
                e.call(REFERENCE_UNLOCK, &args![current]);
            }
            e.call(SAVE_TEST_FINISH, &args![]);
        }
        cursor = e.call(LIST_NEXT, &args![cursor]).u32();
    }
    lock_leave(e, this);
}

// ---------------------------------------------------------------------------
// Second session: the functions from 0054b750 to 0054ee20.
// ---------------------------------------------------------------------------

/// First node of the cell's list of references (`009604f0`).
fn first_node(e: &mut Engine, cell: Ptr<TESObjectCELL>) -> u32 {
    e.call(CELL_REFERENCE_LIST, &args![cell]).u32()
}

/// The reference a list node holds.
fn node_item(e: &mut Engine, node: u32) -> Ptr {
    let address = e.call(LIST_ITEM_ADDRESS, &args![node]).u32();
    Ptr::new(e.mem.u32(address))
}

/// The node after `node`.
fn node_next(e: &mut Engine, node: u32) -> u32 {
    e.call(LIST_NEXT, &args![node]).u32()
}

/// The list's end test (`008256d0`).
fn node_is_end(e: &mut Engine, node: u32) -> bool {
    e.call(LIST_IS_END, &args![node]).bool()
}

/// The position of a reference: the address its virtual slot `0x1F4`
/// answers (three floats).
fn reference_position(e: &mut Engine, reference: Ptr) -> u32 {
    e.vcall(reference.addr(), REFERENCE_SLOT_POSITION, &args![])
        .u32()
}

/// Copies the three words of a vector.
fn copy_vector(e: &mut Engine, destination: u32, source: u32) {
    for i in 0..3 {
        let word = e.mem.u32(source + 4 * i);
        e.mem.set_u32(destination + 4 * i, word);
    }
}

/// The name of an object (its virtual slot `0x130`) and its form id.
fn name_and_id(e: &mut Engine, object: Ptr) -> (u32, u32) {
    let id = form_id(e, object);
    let name = e.vcall(object.addr(), CELL_SLOT_GET_NAME, &args![]).u32();
    (name, id)
}

// Translated from 0054b750 (decompiled, FalloutNV.exe 1.4.0.525)
/// Under the cell's reference lock: runs the cell method `00545c10`, then for
/// every reference other than the player (the global at `011dea3c`) that has
/// a 3D, resets the fade node of that 3D ([`fn_0054b800`]); then calls the
/// reference's virtual slot `0x1CC(0, 0)` (set 3D) on each of them.
pub fn fn_0054b750(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    lock_enter(e, this);
    e.call(CELL_PREPARE_UNLOAD, &args![this]);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        let player = e.global::<u32>(PLAYER_POINTER);
        if !item.is_null() && item.addr() != player {
            let node_3d = e.call(REFERENCE_GET_3D, &args![item]).u32();
            if node_3d != 0 {
                let fade_node = e
                    .vcall(node_3d, NODE_SLOT_GET_FADE_NODE, &args![])
                    .ptr::<()>();
                if !fade_node.is_null() {
                    fn_0054b800(e, fade_node);
                }
            }
            e.vcall(item.addr(), REFERENCE_SLOT_SET_3D, &args![0u32, 0u32]);
        }
    }
    lock_leave(e, this);
}

// Translated from 0054b800 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the two floats at `+0xB4` and `+0xB8` of a fade node and calls
/// `00476ae0(node, 0)`.
pub fn fn_0054b800(e: &mut Engine, this: Ptr) {
    e.mem.set_f32(this.addr() + 0xb8, 0.0);
    e.mem.set_f32(this.addr() + 0xb4, 0.0);
    e.call(FADE_NODE_RESET, &args![this, 0u32]);
}

// Translated from 0054b830 (decompiled, FalloutNV.exe 1.4.0.525)
/// Collects the map markers: pushes onto `list` (`ret 4`) every reference of
/// the cell whose base form is the global at `011ca224` and that has map
/// marker data (`TESObjectREFR::GetMapMarkerData`). Does nothing for a null
/// list.
pub fn fn_0054b830(e: &mut Engine, this: Ptr<TESObjectCELL>, list: Ptr) {
    if list.is_null() {
        return;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null()
            && base_form(e, item).addr() == e.global::<u32>(MARKER_FORM_224)
            && e.call(REFERENCE_GET_MAP_MARKER_DATA, &args![item]).u32() != 0
        {
            push_front_value(e, list.addr(), item.addr());
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
}

// Translated from 0054b8c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like [`fn_0054b830`] for the base form in the global at `011ca228` and
/// the marker data accessor `00569080`.
pub fn fn_0054b8c0(e: &mut Engine, this: Ptr<TESObjectCELL>, list: Ptr) {
    if list.is_null() {
        return;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null()
            && base_form(e, item).addr() == e.global::<u32>(MARKER_FORM_228)
            && e.call(REFERENCE_GET_MARKER_DATA_228, &args![item]).u32() != 0
        {
            push_front_value(e, list.addr(), item.addr());
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
}

// Translated from 0054b950 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unless the data handler's test `004226e0` or the game loader's flag test
/// `0042ce10` answers yes, walks the cell's references under the lock; every
/// reference that passes `0057a2f0` (and `0057a370` when `flag` is set) is
/// handed to `00579ac0(reference, flag)`. `ret 4`.
pub fn fn_0054b950(e: &mut Engine, this: Ptr<TESObjectCELL>, flag: u8) {
    let handler = e.global::<u32>(DATA_HANDLER_POINTER);
    if e.call(DATA_HANDLER_FLAG_4226E0, &args![handler]).bool() {
        return;
    }
    let loader = e.global::<u32>(GAME_LOADER_POINTER);
    if e.call(GAME_LOADER_FLAG_244_2, &args![loader]).bool() {
        return;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if !item.is_null()
            && e.call(REFERENCE_TEST_57A2F0, &args![item]).bool()
            && (flag == 0 || e.call(REFERENCE_TEST_57A370, &args![item]).bool())
        {
            e.call(REFERENCE_UPDATE_579AC0, &args![item, flag as u32]);
        }
    }
    lock_leave(e, this);
}

// Translated from 0054ba00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks the cell's references under the lock; every reference that passes
/// `0057a2f0` is handed to `00579ac0(reference, answer of 0057a370)`.
pub fn fn_0054ba00(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if !item.is_null() && e.call(REFERENCE_TEST_57A2F0, &args![item]).bool() {
            let answer = e.call(REFERENCE_TEST_57A370, &args![item]).u8();
            e.call(REFERENCE_UPDATE_579AC0, &args![item, answer as u32]);
        }
    }
    lock_leave(e, this);
}

// Translated from 0054ba80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::AttachLights` (Xbox PDB), `ret 4`: for every reference of
/// the cell, registers its light with the shadow scene node (`005728c0`)
/// when `attach` is set, else removes it (`TESObjectREFR::RemoveLight`).
pub fn tes_object_cell_attach_lights(e: &mut Engine, this: Ptr<TESObjectCELL>, attach: u8) {
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if !item.is_null() {
            if attach != 0 {
                e.call(REFERENCE_ADD_LIGHT_TO_SCENE, &args![item, 0u32]);
            } else {
                e.call(REFERENCE_REMOVE_LIGHT, &args![item, 0u32]);
            }
        }
    }
    lock_leave(e, this);
}

// Translated from 0054baf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every reference of the cell whose base form has type `0x1E`: reads
/// the extra data entry of type `0x29` of the reference (`00418250`) and,
/// when there is one, calls `0050de20(base form, entry, 0)`.
pub fn fn_0054baf0(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if !item.is_null() {
            let base = base_form(e, item);
            if form_type(e, base) == FORM_TYPE_1E {
                let list = reference_extra_list(e, item);
                let entry = e.call(EXTRA_LIST_GET_ENTRY_29, &args![list]).u32();
                if entry != 0 {
                    let base = base_form(e, item);
                    e.call(BASE_FORM_APPLY_ENTRY, &args![base, entry, 0u32]);
                }
            }
        }
    }
    lock_leave(e, this);
}

// Translated from 0054bb90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The number of references of the cell whose base form has type `0x1E`.
pub fn fn_0054bb90(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u32 {
    lock_enter(e, this);
    let mut count = 0u32;
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if !item.is_null() {
            let base = base_form(e, item);
            if form_type(e, base) == FORM_TYPE_1E {
                count += 1;
            }
        }
    }
    lock_leave(e, this);
    count
}

// Translated from 0054bc10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Hands the cell's references to the process lists ([`fn_0054af40`] with 0),
/// then for every reference whose base form passes [`fn_00549580`] (form flag
/// `0x40`) and that has a 3D, and a cell with a world space: asks the world
/// space's terrain manager to hide the tree of the reference
/// (`BGSTerrainManager::HideTree`) and sets the manager's flag `00929260(1)`.
/// Each reference is prepared with `0056f700` first.
pub fn fn_0054bc10(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    fn_0054af40(e, this, 0);
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null() {
            e.call(REFERENCE_PREPARE_56F700, &args![item]);
            let base = base_form(e, item);
            if fn_00549580(e, base)
                && e.vcall(item.addr(), REFERENCE_SLOT_GET_3D, &args![]).u32() != 0
                && !tes_object_cell_get_world_space(e, this).is_null()
            {
                let world = tes_object_cell_get_world_space(e, this);
                let manager = e.call(WORLD_SPACE_GET_TERRAIN_MANAGER, &args![world]).u32();
                e.call(TERRAIN_MANAGER_HIDE_TREE, &args![manager, item, 1u32]);
                let world = tes_object_cell_get_world_space(e, this);
                let manager = e.call(WORLD_SPACE_GET_TERRAIN_MANAGER, &args![world]).u32();
                e.call(TERRAIN_MANAGER_SET_FLAG_28, &args![manager, 1u32]);
            }
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
}

// Translated from 0054bcf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Brings the references of the cell up after its 3D is loaded (`ret 4`).
/// Loads the cell's 3D (`TESObjectCELL::Load3D`), runs the cell's scripts
/// ([`tes_object_cell_run_scripts`]), asks [`fn_0054df30`] and hands its
/// answer to `00550c60`. If the `TES` object accepts the cell (`00453490`)
/// the 3D is moved to the origin and [`fn_0054bc10`] runs. Otherwise every
/// reference gets one of two passes over the list, markers (base form
/// `011ca238` or `011ca230`) in the second and everything else in the first:
/// unless the game loader flag is set it gets its action list set up
/// (`Script::InitActionList`, `SetActionFlag(0x1000)`); a reference without
/// form flags `0x800` / `0x20` is un-queued from the process lists (an actor)
/// or given its virtual slots `0x224` / `0x260`, queued with the model loader
/// at the cell's priority, and, when it has a 3D, attached with
/// [`tes_object_cell_attach_reference_3d`] unless its base form has type
/// `0x23` and `00452440` accepts it. The two tails are `005535f0` when
/// `flag` is set and `cCellState = 6` (`004512a0(6)`).
///
/// The compiler's exception frame is not translated.
pub fn fn_0054bcf0(e: &mut Engine, this: Ptr<TESObjectCELL>, flag: u8) {
    let node_3d = e.call(CELL_LOAD_3D, &args![this]).u32();
    tes_object_cell_run_scripts(e, this, 0, 0);
    let scripts = fn_0054df30(e, this);
    e.call(CELL_AFTER_SCRIPTS, &args![this, scripts as u32]);
    let tes = e.global::<u32>(TES_POINTER);
    if e.call(TES_CELL_TEST_453490, &args![tes, this]).bool() {
        if node_3d != 0 {
            e.with_stack(12, |e, vector| {
                e.call(VECTOR3_CONSTRUCT_43D410, &args![vector, 0.0f32, 0u32, 0u32]);
                e.call(NODE_SET_LOCAL_TRANSLATE, &args![node_3d, vector]);
            });
        }
        fn_0054bc10(e, this);
    } else {
        lock_enter(e, this);
        let tes = e.global::<u32>(TES_POINTER);
        let priority = e.call(TES_GET_CELL_PRIORITY, &args![tes, this, 0u32]).u32();
        let mut cursor = first_node(e, this);
        let mut pass = 0u32;
        while pass < 2 && cursor != 0 {
            let item = node_item(e, cursor);
            cursor = node_next(e, cursor);
            if cursor == 0 {
                pass += 1;
                if pass < 2 {
                    cursor = first_node(e, this);
                }
            }
            if item.is_null() {
                continue;
            }
            let mut marker = false;
            if !base_form(e, item).is_null() {
                marker = base_form(e, item).addr() == e.global::<u32>(ROOM_MARKER_FORM_238)
                    || base_form(e, item).addr() == e.global::<u32>(MARKER_FORM_230);
            }
            if (pass == 0 && !marker) || (pass == 1 && marker) {
                continue;
            }
            let loader = e.global::<u32>(GAME_LOADER_POINTER);
            if !e.call(GAME_LOADER_FLAG_244_2, &args![loader]).bool() {
                let list = reference_extra_list(e, item);
                e.call(SCRIPT_INIT_ACTION_LIST, &args![item, list]);
                let list = reference_extra_list(e, item);
                e.call(SCRIPT_SET_ACTION_FLAG, &args![item, list, 0x1000u32]);
            }
            if e.call(FORM_FLAG_800, &args![item]).bool()
                || e.call(FORM_FLAG_20, &args![item]).bool()
            {
                continue;
            }
            let save = e.global::<u32>(SAVE_GAME_POINTER);
            let as_actor = is_actor(e, item)
                && (!e.call(SAVE_GAME_FLAG, &args![save]).bool()
                    || e.call(SAVE_GAME_FLAG, &args![save]).bool());
            if as_actor {
                let actor = dynamic_cast(e, item, RTTI_REFERENCE, RTTI_ACTOR_LIKE);
                if !actor.is_null() {
                    e.vcall(actor.addr(), ACTOR_SLOT_240, &args![]);
                    e.call(
                        PROCESS_LISTS_REMOVE_ACTOR_FROM_TEMP_CHANGE_LIST,
                        &args![PROCESS_LISTS, actor],
                    );
                }
            } else if e.vcall(item.addr(), REFERENCE_SLOT_224, &args![]).bool() {
                e.vcall(item.addr(), REFERENCE_SLOT_260, &args![]);
            }
            let model_loader = e.global::<u32>(MODEL_LOADER_POINTER);
            e.call(
                MODEL_LOADER_QUEUE_REFERENCE,
                &args![model_loader, item, priority, 0u32],
            );
            if e.vcall(item.addr(), REFERENCE_SLOT_GET_3D, &args![]).u32() != 0 {
                let mut attach = true;
                let base = base_form(e, item);
                if form_type(e, base) == FORM_TYPE_23 {
                    let base = base_form(e, item);
                    if e.call(FORM_TEST_452440, &args![base]).bool() {
                        attach = false;
                    }
                }
                if attach {
                    tes_object_cell_attach_reference_3d(e, this, item, 0);
                }
            }
        }
        lock_leave(e, this);
    }
    if flag != 0 {
        e.call(CELL_AFTER_ATTACH, &args![this]);
    }
    e.call(CELL_SET_STATE, &args![this, 6u32]);
}

/// `|component|` scaled against `limit`: true when the scaled limit
/// (`004587d0`) is below the absolute value (`fabs`) of `component`.
fn component_exceeds(e: &mut Engine, component: f32, limit: f32) -> bool {
    let magnitude = e.call(FLOAT_ABS, &args![component]).f32();
    let scaled = e.call(WORLD_SCALE, &args![limit]).f32();
    f64::from(scaled) < f64::from(magnitude)
}

/// [`component_exceeds`] on the three floats at `vector`, stopping at the
/// first that does.
fn vector_exceeds(e: &mut Engine, vector: u32, limit: f32) -> bool {
    for i in 0..3 {
        let component = e.mem.f32(vector + 4 * i);
        if component_exceeds(e, component, limit) {
            return true;
        }
    }
    false
}

/// `|component| > 30000.0` (the double at `0102efb8`) for the float at
/// `address`, through `fabs`.
fn above_position_limit(e: &mut Engine, address: u32) -> bool {
    let component = e.mem.f32(address);
    let magnitude = e.call(FLOAT_ABS, &args![component]).f32();
    f64::from(magnitude) > e.global::<f64>(POSITION_LIMIT)
}

/// The opposite test, `|component| < 30000.0`.
fn below_position_limit(e: &mut Engine, address: u32) -> bool {
    let component = e.mem.f32(address);
    let magnitude = e.call(FLOAT_ABS, &args![component]).f32();
    f64::from(magnitude) < e.global::<f64>(POSITION_LIMIT)
}

// Translated from 0054c030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::CalcRefCenterPoint` (Xbox PDB), `ret 4`: writes to `out`
/// the centre of the box that holds the references of the cell. `out` starts
/// as the default vector (`011f426c`). Under the reference lock, every
/// reference with a base form whose type is neither `0x1E` nor in `0x2A..=
/// 0x2D` takes part: when it lies more than 30000.0 away on x or y, or (from
/// the second reference on) outside the box scaled by the limit at
/// `0102efc0`, and has the extra data entry of type `0x0F`, the message at
/// `0102ef68` is printed and the reference is put back at its start position
/// (`virtual 0x170`); when it still is out of range afterwards, or has no such
/// entry, the message at `0102ef18` is printed and the reference is moved to
/// the placement [`tes_object_cell_get_coc_placement_info`] finds and left
/// out of the box. Every other reference widens the box by its position.
/// With at least one reference, the box is checked against the limit (an
/// error message with the cell's name and extents when it is too large) and
/// `out` becomes its centre.
///
/// The `006815c0` calls the compiler makes on its vector temporaries (they
/// return `this`) are kept.
pub fn tes_object_cell_calc_ref_center_point(e: &mut Engine, this: Ptr<TESObjectCELL>, out: Ptr) {
    copy_vector(e, out.addr(), DEFAULT_VECTOR);
    e.with_stack(0x100, |e, frame| {
        let f = frame.addr();
        // The running box: minimum (+0x00) and maximum (+0x10); scratch
        // vectors at +0x20 .. +0xE0.
        let (min, max) = (f, f + 0x10);
        let two = e.global::<f32>(TWO);
        let limit = e.global::<f32>(EXTENT_LIMIT);
        lock_enter(e, this);
        let mut count = 0u32;
        let flt_max = e.global::<f32>(FLOAT_MAX);
        e.call(VECTOR3_CONSTRUCT, &args![min, flt_max, flt_max, flt_max]);
        let smallest = e.global::<f32>(SMALLEST_NORMAL_FLOAT);
        e.call(VECTOR3_CONSTRUCT, &args![max, smallest, smallest, smallest]);
        let mut cursor = first_node(e, this);
        while cursor != 0 {
            let item = node_item(e, cursor);
            if !item.is_null() && !base_form(e, item).is_null() {
                let mut include = true;
                let base = base_form(e, item);
                let kind = form_type(e, base);
                if kind == FORM_TYPE_1E || (kind > 0x29 && kind <= 0x2d) {
                    include = false;
                } else {
                    let mut outside = false;
                    if count != 0 {
                        let sum = e.call(VECTOR_ADD, &args![min, f + 0x20, max]).u32();
                        let center = e.call(VECTOR_DIVIDE, &args![sum, f + 0x30, two]).u32();
                        e.call(TRIVIAL_CONSTRUCT, &args![f + 0x40]);
                        let position = reference_position(e, item);
                        let offset = e
                            .call(VECTOR_SUBTRACT, &args![center, f + 0x50, position])
                            .u32();
                        outside = vector_exceeds(e, offset, limit);
                    }
                    let position = reference_position(e, item);
                    let far = above_position_limit(e, position) || {
                        let position = reference_position(e, item);
                        above_position_limit(e, position + 4)
                    };
                    if far || outside {
                        let mut fix = true;
                        let list = reference_extra_list(e, item);
                        if e.call(EXTRA_LIST_GET_EXTRA_DATA, &args![list, 0xfu32])
                            .u32()
                            != 0
                        {
                            let (name, id) = name_and_id(e, item);
                            e.call(DEBUG_PRINT, &args![MESSAGE_RETURN_TO_START, name, id]);
                            let start = e
                                .vcall(item.addr(), REFERENCE_SLOT_170, &args![f + 0x60])
                                .u32();
                            e.call(REFERENCE_SET_LOCATION, &args![item, start]);
                            if outside {
                                let sum = e.call(VECTOR_ADD, &args![min, f + 0x70, max]).u32();
                                let center =
                                    e.call(VECTOR_DIVIDE, &args![sum, f + 0x80, two]).u32();
                                e.call(TRIVIAL_CONSTRUCT, &args![f + 0x90]);
                                let position = reference_position(e, item);
                                let offset = e
                                    .call(VECTOR_SUBTRACT, &args![center, f + 0xa0, position])
                                    .u32();
                                outside = vector_exceeds(e, offset, limit);
                            }
                            let position = reference_position(e, item);
                            if below_position_limit(e, position) {
                                let position = reference_position(e, item);
                                if below_position_limit(e, position + 4) && !outside {
                                    fix = false;
                                }
                            }
                        }
                        if fix {
                            let (name, id) = name_and_id(e, item);
                            e.call(DEBUG_PRINT, &args![MESSAGE_MOVE_TO_SAFE_PLACE, name, id]);
                            e.call(TRIVIAL_CONSTRUCT, &args![f + 0xb0]);
                            e.call(TRIVIAL_CONSTRUCT, &args![f + 0xbc]);
                            tes_object_cell_get_coc_placement_info(
                                e,
                                this,
                                Ptr::new(f + 0xb0),
                                Ptr::new(f + 0xbc),
                            );
                            e.call(REFERENCE_SET_LOCATION, &args![item, f + 0xb0]);
                            include = false;
                        }
                    }
                }
                if include {
                    for axis in 0..3u32 {
                        let position = reference_position(e, item);
                        let value = e.mem.f32(position + 4 * axis);
                        let current = e.mem.f32(min + 4 * axis);
                        let smaller = e.call(FLOAT_SMALLER, &args![current, value]).f32();
                        e.mem.set_f32(min + 4 * axis, smaller);
                    }
                    for axis in 0..3u32 {
                        let position = reference_position(e, item);
                        let value = e.mem.f32(position + 4 * axis);
                        let current = e.mem.f32(max + 4 * axis);
                        let larger = e.call(FLOAT_LARGER, &args![current, value]).f32();
                        e.mem.set_f32(max + 4 * axis, larger);
                    }
                    count += 1;
                }
            }
            cursor = node_next(e, cursor);
        }
        if count > 0 {
            let extent = e.call(VECTOR_SUBTRACT, &args![max, f + 0xd0, min]).u32();
            let mut too_large = false;
            for axis in 0..3u32 {
                let scaled = e.call(WORLD_SCALE, &args![limit]).f32();
                if f64::from(e.mem.f32(extent + 4 * axis)) > f64::from(scaled) {
                    too_large = true;
                    break;
                }
            }
            if too_large {
                let scaled = e.call(WORLD_SCALE, &args![limit]).f32();
                let (name, id) = name_and_id(e, this.cast());
                let (x, y, z) = (
                    f64::from(e.mem.f32(extent)),
                    f64::from(e.mem.f32(extent + 4)),
                    f64::from(e.mem.f32(extent + 8)),
                );
                e.call(
                    DEBUG_PRINT,
                    &args![
                        MESSAGE_INTERIOR_TOO_LARGE,
                        name,
                        id,
                        x,
                        y,
                        z,
                        f64::from(scaled)
                    ],
                );
            }
            let sum = e.call(VECTOR_ADD, &args![min, f + 0xe0, max]).u32();
            let center = e.call(VECTOR_DIVIDE, &args![sum, f + 0xf0, two]).u32();
            copy_vector(e, out.addr(), center);
        }
        lock_leave(e, this);
    });
}

// Translated from 0054c740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::RunScripts` (Xbox PDB), `ret 8` (the second word is never
/// read): under the reference lock, runs the scripts of the references
/// (`TESObjectREFR::RunScript`) of the loaded data's list (`pLoadedData +
/// 0x4C`), or, with no loaded data and `all` set, of the cell's list. Without
/// `all` only references with a 3D or form flag `0x800` run, and the walk
/// stops at the first script that ran (answer 1). If nothing ran, there is
/// loaded data and the menus are closed, the loaded data's second list
/// (`+0x54`) drops every reference whose `UpdateChildActivates` says no.
pub fn tes_object_cell_run_scripts(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    all: u8,
    _unused_2: u32,
) -> u8 {
    let mut ran = 0u8;
    lock_enter(e, this);
    let loaded = e.get(this, TESObjectCELL::pLoadedData);
    let mut cursor = 0u32;
    if !loaded.is_null() {
        cursor = loaded.addr() + 0x4c;
    } else if all != 0 {
        cursor = first_node(e, this);
    }
    while cursor != 0 {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if item.is_null() {
            continue;
        }
        if all == 0
            && e.call(REFERENCE_GET_3D, &args![item]).u32() == 0
            && !e.call(FORM_FLAG_800, &args![item]).bool()
        {
            continue;
        }
        if !e.call(REFERENCE_RUN_SCRIPT, &args![item]).bool() || all != 0 {
            continue;
        }
        ran = 1;
        break;
    }
    let loaded = e.get(this, TESObjectCELL::pLoadedData);
    if !loaded.is_null() && ran == 0 && !e.call(INTERFACE_IS_IN_MENU_MODE, &args![]).bool() {
        let mut node = loaded.addr() + 0x54;
        let mut previous = 0u32;
        while node != 0 && !node_is_end(e, node) {
            let item = node_item(e, node);
            if e.call(REFERENCE_UPDATE_CHILD_ACTIVATES, &args![item])
                .bool()
            {
                previous = node;
                node = node_next(e, node);
            } else if previous == 0 {
                e.call(LIST_POP_FRONT, &args![node]);
            } else {
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), item.addr());
                    e.call(LIST_REMOVE_ITEM, &args![previous, slot]);
                });
                node = node_next(e, previous);
            }
        }
    }
    lock_leave(e, this);
    ran
}

// Translated from 0054c8c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::AssignPersistentRefsToCellsInWorld` (Xbox PDB), `ret 4`:
/// for a persistent cell and a world space, finds for every reference the
/// exterior cell its position falls into (`TESWorldSpace::GetCellFromCellCoord`
/// on the rounded coordinates shifted right by 12) and adds the reference to
/// it ([`tes_object_cell_add_reference`]); a reference with no such cell is
/// reported with the message at `0102efc8`.
pub fn tes_object_cell_assign_persistent_refs_to_cells_in_world(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    world: Ptr,
) {
    if world.is_null() || !e.call(CELL_PERSISTENT_FLAG, &args![this]).bool() {
        return;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null() {
            let position = reference_position(e, item);
            let x = e.mem.f32(position);
            let cell_x = (e.call(FLOAT_TO_INT, &args![x]).u32() as i32) >> 12;
            let position = reference_position(e, item);
            let y = e.mem.f32(position + 4);
            let cell_y = (e.call(FLOAT_TO_INT, &args![y]).u32() as i32) >> 12;
            let cell = e
                .call(
                    WORLD_SPACE_GET_CELL_FROM_COORD,
                    &args![world, cell_x, cell_y],
                )
                .ptr::<TESObjectCELL>();
            if cell.is_null() {
                let (item_name, item_id) = name_and_id(e, item);
                let cell_id = form_id(e, this.cast());
                let world_name = e.vcall(world.addr(), CELL_SLOT_GET_NAME, &args![]).u32();
                e.call(
                    DEBUG_PRINT,
                    &args![
                        MESSAGE_CELL_NOT_FOUND,
                        cell_x,
                        cell_y,
                        world_name,
                        cell_id,
                        item_name,
                        item_id
                    ],
                );
            } else {
                tes_object_cell_add_reference(e, cell, item, 0);
            }
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
}

// Translated from 0054ca00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::AddFurnitureToList` (Xbox PDB), `ret 4`: pushes onto
/// `list` every furniture reference (`TESObjectREFR::IsFurniture`) of the
/// cell that has neither form flag `0x20` nor `0x800`. Does nothing for a
/// null list.
pub fn tes_object_cell_add_furniture_to_list(e: &mut Engine, this: Ptr<TESObjectCELL>, list: Ptr) {
    if list.is_null() {
        return;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null()
            && e.call(REFERENCE_IS_FURNITURE, &args![item]).bool()
            && !e.call(FORM_FLAG_20, &args![item]).bool()
            && !e.call(FORM_FLAG_800, &args![item]).bool()
        {
            push_front_value(e, list.addr(), item.addr());
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
}

// Translated from 0054ca90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::RemoveReference` (Xbox PDB), `ret 4`: takes `reference`
/// out of the cell. Counts it out of `sNumRefsWithVisibleDistant` when it has
/// visible distant data; gives it the cell's encounter zone (or the default
/// one) when it has none and the data handler is not loading; removes it from
/// the cell's list under the lock; tells the state 6 object when the cell is in
/// state 6 or 5; with loaded data undoes the scripted, activating-children,
/// emittance and multibound bookkeeping. A persistent cell then clears the
/// reference's persistent cell and removes it from the world space's
/// persistent data. Any other cell lets the reference leave through its
/// virtual slot `0x228(0)`, clears the occlusion plane links that point at it
/// (for a reference of the base form in `011ca234`) and, unless the
/// reference is persistent or the data handler is loading, calls the cell's
/// virtual slot `0xC8(1)`.
pub fn tes_object_cell_remove_reference(e: &mut Engine, this: Ptr<TESObjectCELL>, reference: Ptr) {
    if reference.is_null() {
        return;
    }
    if e.call(REFERENCE_HAS_VISIBLE_DISTANT, &args![reference])
        .bool()
    {
        let count = e.get(this, TESObjectCELL::sNumRefsWithVisibleDistant);
        e.set(
            this,
            TESObjectCELL::sNumRefsWithVisibleDistant,
            count.wrapping_sub(1),
        );
    }
    if e.call(REFERENCE_GET_ENCOUNTER_ZONE, &args![reference])
        .u32()
        == 0
        && !data_handler_flag(e)
    {
        if e.call(CELL_ENCOUNTER_ZONE, &args![this]).u32() != 0 {
            let zone = e.call(CELL_ENCOUNTER_ZONE, &args![this]).u32();
            e.call(REFERENCE_SET_ENCOUNTER_ZONE, &args![reference, zone]);
        } else {
            let zone = e.call(DEFAULT_ENCOUNTER_ZONE, &args![]).u32();
            e.call(REFERENCE_SET_ENCOUNTER_ZONE, &args![reference, zone]);
        }
    }
    lock_enter(e, this);
    let list = first_node(e, this);
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), reference.addr());
        e.call(LIST_REMOVE_ITEM, &args![list, slot]);
    });
    lock_leave(e, this);
    if e.call(CELL_STATE_IS_6, &args![this]).bool() || e.call(CELL_STATE_IS_5, &args![this]).bool()
    {
        let object = e.global::<u32>(STATE_6_OBJECT_POINTER);
        e.call(STATE_6_REMOVE_REFERENCE, &args![object, reference]);
    }
    if !e.get(this, TESObjectCELL::pLoadedData).is_null() {
        let loader = e.global::<u32>(GAME_LOADER_POINTER);
        let loading = e.call(GAME_LOADER_FLAG_4121B0, &args![loader]).bool();
        if loading || e.call(REFERENCE_IS_SCRIPTED, &args![reference]).bool() {
            e.call(CELL_REMOVE_SCRIPTED_REF, &args![this, reference]);
        }
        if e.call(REFERENCE_IS_ACTIVATING_CHILDREN, &args![reference])
            .bool()
        {
            e.call(CELL_REMOVE_ACTIVATING_REF, &args![this, reference]);
        }
        e.call(CELL_REMOVE_EMITTANCE_REF, &args![this, reference]);
        e.call(CELL_REMOVE_MULTI_BOUND_REF, &args![this, reference]);
    }
    if e.call(CELL_PERSISTENT_FLAG, &args![this]).bool() {
        let list = reference_extra_list(e, reference);
        e.call(EXTRA_LIST_SET_PERSISTENT_CELL, &args![list, 0u32]);
        let world = tes_object_cell_get_world_space(e, this);
        if !world.is_null() {
            e.call(
                WORLD_SPACE_REMOVE_FROM_PERSISTENT_REF_DATA,
                &args![world, reference],
            );
        }
        return;
    }
    e.vcall(reference.addr(), REFERENCE_SLOT_ADD_TO_CELL, &args![0u32]);
    if base_form(e, reference).addr() == e.global::<u32>(OCCLUSION_MARKER_FORM_234) {
        let list = reference_extra_list(e, reference);
        let planes = e
            .call(EXTRA_LIST_GET_OCCLUSION_LINKED_REFS, &args![list])
            .u32();
        if planes != 0 {
            for i in 0..4u32 {
                let linked = e.mem.u32(planes + 4 * i);
                if linked != 0 {
                    let list = reference_extra_list(e, Ptr::new(linked));
                    let links = e
                        .call(EXTRA_LIST_GET_OCCLUSION_LINKED_REFS, &args![list])
                        .u32();
                    if links != 0 {
                        for j in 0..4u32 {
                            if e.mem.u32(links + 4 * j) == reference.addr() {
                                e.mem.set_u32(links + 4 * j, 0);
                            }
                        }
                    }
                }
            }
        }
    }
    let handler = e.global::<u32>(DATA_HANDLER_POINTER);
    if !e.call(FORM_FLAG_4000, &args![reference]).bool()
        && !e.call(REFERENCE_GET_REF_PERSISTS, &args![reference]).bool()
        && !e.call(DATA_HANDLER_FLAG_4226E0, &args![handler]).bool()
    {
        e.vcall(this.addr(), CELL_SLOT_SET_LOADED, &args![1u32]);
    }
}

// Translated from 0054cd20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Empties the cell's list of references. Under the lock: runs the cell
/// method `005576c0`, clears `sNumRefsWithVisibleDistant`, then repeatedly
/// takes the first reference (the walk restarts at the head each time) and
/// drops its node. The player only leaves the cell (virtual slot `0x228(0)`).
/// A persistent reference of an exterior cell (unless the cell is persistent
/// and the data handler is not loading) is deleted through its virtual slot
/// `0x10(1)` in a persistent cell, or leaves through `0x228(0)`. Any other
/// reference is passed to `00558ba0` when its base form has type `0x23` and
/// `00452440` accepts it, and then to `GarbageCollector::Add`.
pub fn fn_0054cd20(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    lock_enter(e, this);
    e.call(CELL_FN_5576C0, &args![this]);
    e.set(this, TESObjectCELL::sNumRefsWithVisibleDistant, 0);
    let mut cursor = first_node(e, this);
    while cursor != 0 && !node_is_end(e, cursor) {
        let item = node_item(e, cursor);
        if item.addr() == e.global::<u32>(PLAYER_POINTER) {
            let head = first_node(e, this);
            e.call(LIST_POP_FRONT, &args![head]);
            e.vcall(item.addr(), REFERENCE_SLOT_ADD_TO_CELL, &args![0u32]);
        } else {
            let mut keep = false;
            let handler = e.global::<u32>(DATA_HANDLER_POINTER);
            if e.call(CELL_PERSISTENT_FLAG, &args![this]).bool()
                && handler != 0
                && !e.call(DATA_HANDLER_FLAG_4226E0, &args![handler]).bool()
            {
                keep = true;
            }
            if e.call(REFERENCE_GET_REF_PERSISTS, &args![item]).bool()
                && !is_interior(e, this)
                && !keep
            {
                if e.call(CELL_PERSISTENT_FLAG, &args![this]).bool() {
                    e.vcall(item.addr(), REFERENCE_SLOT_10, &args![1u32]);
                } else {
                    e.vcall(item.addr(), REFERENCE_SLOT_ADD_TO_CELL, &args![0u32]);
                }
            } else {
                if !base_form(e, item).is_null() {
                    let base = base_form(e, item);
                    if form_type(e, base) == FORM_TYPE_23 {
                        let base = base_form(e, item);
                        if e.call(FORM_TEST_452440, &args![base]).bool() {
                            e.call(CELL_FN_558BA0, &args![this, item]);
                        }
                    }
                }
                e.call(GARBAGE_COLLECTOR_ADD, &args![item]);
            }
            let head = first_node(e, this);
            e.call(LIST_POP_FRONT, &args![head]);
        }
        cursor = first_node(e, this);
    }
    lock_leave(e, this);
}

// Translated from 0054cee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::GetFirstRefr` (Xbox PDB): the first reference of the cell,
/// read under the lock without a test for an empty list.
pub fn tes_object_cell_get_first_refr(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    lock_enter(e, this);
    let head = first_node(e, this);
    let first = node_item(e, head);
    lock_leave(e, this);
    first
}

// Translated from 0054cf20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first reference of the cell whose base form is `form` (`ret 8`), or
/// null; with `skip_flagged` set, references with form flag `0x20` are
/// passed over. Null for a null `form`.
pub fn fn_0054cf20(e: &mut Engine, this: Ptr<TESObjectCELL>, form: Ptr, skip_flagged: u8) -> Ptr {
    if form.is_null() {
        return Ptr::NULL;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 && !node_is_end(e, cursor) {
        let item = node_item(e, cursor);
        let base = base_form(e, item);
        cursor = node_next(e, cursor);
        if !base.is_null()
            && !(skip_flagged != 0 && e.call(FORM_FLAG_20, &args![item]).bool())
            && base == form
        {
            lock_leave(e, this);
            return item;
        }
    }
    lock_leave(e, this);
    Ptr::NULL
}

// Translated from 0054cfd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::GetCOCPlacementInfo` (Xbox PDB), `ret 8`: `out_position`
/// becomes the middle of the cell (`x * 4096 + 2048`, `y * 4096 + 2048`, 0
/// from the cell's grid coordinates) and `out_rotation` the default vector.
/// When the setting object at `011ca0a8` starts with a zero byte, or the cell
/// is an interior, the temporary data is loaded
/// (`TESObjectCELL::LoadAllTempData`) and
/// [`tes_object_cell_determine_coc_placement`] refines both.
pub fn tes_object_cell_get_coc_placement_info(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    out_position: Ptr,
    out_rotation: Ptr,
) {
    let grid_y = e.call(CELL_GET_DATA_Y, &args![this]).u32() as i32;
    let y = grid_y.wrapping_shl(12).wrapping_add(0x800);
    let grid_x = e.call(CELL_GET_DATA_X, &args![this]).u32() as i32;
    let x = grid_x.wrapping_shl(12).wrapping_add(0x800);
    e.with_stack(12, |e, vector| {
        let result = e
            .call(
                VECTOR3_CONSTRUCT,
                &args![vector, x as f32, y as f32, 0.0f32],
            )
            .u32();
        copy_vector(e, out_position.addr(), result);
    });
    copy_vector(e, out_rotation.addr(), DEFAULT_VECTOR);
    let text = e.call(SETTING_TEXT, &args![COC_SETTING]).u32();
    if e.mem.u8(text) == 0 || is_interior(e, this) {
        e.call(CELL_LOAD_ALL_TEMP_DATA, &args![this]);
        tes_object_cell_determine_coc_placement(e, this, out_position, out_rotation);
    }
}

/// Whether the position at `position` is usable in the cell: an interior
/// cell takes any, an exterior one asks `00550200`.
fn usable_position(e: &mut Engine, this: Ptr<TESObjectCELL>, item: Ptr) -> bool {
    is_interior(e, this) || {
        let position = reference_position(e, item);
        e.call(CELL_POSITION_FITS, &args![this, position]).bool()
    }
}

// Translated from 0054d090 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::DetermineCOCPlacement` (Xbox PDB), `ret 8`: picks where
/// to put the player when the cell is entered by "center on cell". Under
/// the reference lock, among the references without form flag `0x20`, in
/// list order, it remembers the first of each wanted kind that is usable in
/// the cell: base form `011ca24c`, `011ca244` and `011ca248` (markers), a base
/// form of type `0x20`, and any reference at all; the first door-like
/// reference whose teleport data leads on to a position becomes
/// `out_position` at once. Then `out_position` (and `out_rotation` for the
/// `011ca24c` and `011ca248` markers) is taken from the first of those found
/// in that order (`011ca24c`, `011ca248`, `011ca244`, the teleport position
/// already written, the type `0x20` reference, any reference). In an exterior
/// cell with land, the height is at least the land's and the water's.
pub fn tes_object_cell_determine_coc_placement(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    out_position: Ptr,
    out_rotation: Ptr,
) {
    lock_enter(e, this);
    let mut marker_244 = Ptr::NULL;
    let mut marker_248 = Ptr::NULL;
    let mut marker_24c = Ptr::NULL;
    let mut type_20 = Ptr::NULL;
    let mut any = Ptr::NULL;
    let mut found_package = false;
    let mut cursor = first_node(e, this);
    while cursor != 0 && !node_is_end(e, cursor) {
        let item = node_item(e, cursor);
        if !e.call(FORM_FLAG_20, &args![item]).bool() {
            if marker_24c.is_null()
                && base_form(e, item).addr() == e.global::<u32>(COC_MARKER_FORM_24C)
                && usable_position(e, this, item)
            {
                marker_24c = item;
            }
            if marker_244.is_null()
                && base_form(e, item).addr() == e.global::<u32>(COC_MARKER_FORM_244)
                && usable_position(e, this, item)
            {
                marker_244 = item;
            }
            if marker_248.is_null()
                && base_form(e, item).addr() == e.global::<u32>(COC_MARKER_FORM_248)
                && usable_position(e, this, item)
            {
                marker_248 = item;
            }
            if !found_package {
                let teleport = e.call(REFERENCE_GET_TELEPORT_DATA, &args![item]).u32();
                if teleport != 0 && e.call(SLOT_GET, &args![teleport]).u32() != 0 {
                    let target = e.call(SLOT_GET, &args![teleport]).u32();
                    let data = e.call(REFERENCE_GET_TELEPORT_DATA, &args![target]).u32();
                    if data != 0 {
                        let position = e.call(TELEPORT_POSITION, &args![data]).u32();
                        e.with_stack(12, |e, local| {
                            copy_vector(e, local.addr(), position);
                            if is_interior(e, this)
                                || e.call(CELL_POSITION_FITS, &args![this, local]).bool()
                            {
                                copy_vector(e, out_position.addr(), local.addr());
                                found_package = true;
                            }
                        });
                    }
                }
            }
            if type_20.is_null() {
                let base = base_form(e, item);
                if form_type(e, base) == 0x20 && usable_position(e, this, item) {
                    type_20 = item;
                }
            }
            if usable_position(e, this, item) {
                any = item;
            }
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
    if !marker_24c.is_null() {
        let position = reference_position(e, marker_24c);
        copy_vector(e, out_position.addr(), position);
        let rotation = e.call(REFERENCE_POSITION_ADDRESS, &args![marker_24c]).u32();
        copy_vector(e, out_rotation.addr(), rotation);
    } else if !marker_248.is_null() {
        let position = reference_position(e, marker_248);
        copy_vector(e, out_position.addr(), position);
        let rotation = e.call(REFERENCE_POSITION_ADDRESS, &args![marker_248]).u32();
        copy_vector(e, out_rotation.addr(), rotation);
    } else if !marker_244.is_null() {
        let position = reference_position(e, marker_244);
        copy_vector(e, out_position.addr(), position);
    } else if !found_package {
        if !type_20.is_null() {
            let position = reference_position(e, type_20);
            copy_vector(e, out_position.addr(), position);
        } else if !any.is_null() {
            let position = reference_position(e, any);
            copy_vector(e, out_position.addr(), position);
        }
    }
    if !is_interior(e, this) {
        let land = e.call(CELL_GET_LAND, &args![this]).u32();
        if land != 0 {
            e.with_stack(4, |e, height| {
                e.mem.set_f32(height.addr(), 0.0);
                e.call(LAND_GET_HEIGHT, &args![land, out_position, height]);
                let water = e.call(CELL_GET_WATER_HEIGHT, &args![this]).f32();
                let mut value = e.mem.f32(height.addr());
                if value < water {
                    value = water;
                }
                let z = out_position.addr() + 8;
                if e.mem.f32(z) < value {
                    e.mem.set_f32(z, value);
                }
            });
        }
    }
}

/// Whether the limit differs from `FLT_MAX` (the double at `010231b0`).
fn limit_is_set(e: &Engine, limit: f32) -> bool {
    f64::from(limit) != e.global::<f64>(FLOAT_MAX_DOUBLE)
}

/// True when the reference is out of range of `point` for `limit`: the
/// distance test answers a value `>= 0`.
fn out_of_range(e: &mut Engine, item: Ptr, point: Ptr, limit: f32) -> bool {
    let position = reference_position(e, item);
    (e.call(DISTANCE_TEST, &args![position, point, limit]).u32() as i32) >= 0
}

/// True when the reference is skipped by the two range tests of the walks:
/// out of range of the first point (when its limit is set), or out of range
/// of the second (when its limit is set and differs from the first limit or
/// the points differ).
fn skipped_by_ranges(
    e: &mut Engine,
    item: Ptr,
    point_a: Ptr,
    limit_a: f32,
    point_b: Ptr,
    limit_b: f32,
) -> bool {
    if limit_is_set(e, limit_a) && out_of_range(e, item, point_a, limit_a) {
        return true;
    }
    if limit_is_set(e, limit_b) {
        let same_limit = f64::from(limit_b) == f64::from(limit_a);
        if (!same_limit || !e.call(POINTS_EQUAL, &args![point_a, point_b]).bool())
            && out_of_range(e, item, point_b, limit_b)
        {
            return true;
        }
    }
    false
}

/// The smaller of `FLT_MAX` and the room left in the two limits: `limit -
/// distance` for each limit that is set (the second only when it differs
/// from the first), as `float`s.
fn remaining_range(
    e: &mut Engine,
    item: Ptr,
    point_a: Ptr,
    limit_a: f32,
    point_b: Ptr,
    limit_b: f32,
) -> f32 {
    let mut remaining = e.global::<f32>(FLOAT_MAX);
    if limit_is_set(e, limit_a) {
        let distance = e.call(REFERENCE_DISTANCE, &args![item, point_a]).f32();
        let room = (f64::from(limit_a) - f64::from(distance)) as f32;
        if room < remaining {
            remaining = room;
        }
    }
    if limit_is_set(e, limit_b) && f64::from(limit_b) != f64::from(limit_a) {
        let distance = e.call(REFERENCE_DISTANCE, &args![item, point_b]).f32();
        let room = (f64::from(limit_b) - f64::from(distance)) as f32;
        if room < remaining {
            remaining = room;
        }
    }
    remaining
}

// Translated from 0054d4b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl `(cell, point_a, limit_a, point_b, limit_b, callback, context)`:
/// walks the references of `cell` that lie within the limits of the two
/// points (`FLT_MAX` for a limit that is not set) and calls the cdecl
/// `callback(reference, context)` on each, stopping when it answers yes;
/// returns 1 when it never did. A door reference (base form type `0x1C` but
/// not the form in `011ca258`) that leads to another place is followed:
/// to a world space, remembering the nearest door seen (the static state at
/// `011ca1e4` / `011ca1ec` / `0118b6f4`), or to an interior cell not yet in
/// the visited container (`011ca1f8`), whose doors are walked recursively
/// with the range left. When the outermost walk (`011ca1e8` back to 0) found
/// nothing, it hands the remembered door position to the world space's own
/// walk `005885f0`. The static container and its `atexit` destructor are
/// initialised on first use.
///
/// The compiler's exception frame is not translated.
#[allow(clippy::too_many_arguments)]
pub fn fn_0054d4b0(
    e: &mut Engine,
    cell: Ptr<TESObjectCELL>,
    point_a: Ptr,
    limit_a: f32,
    point_b: Ptr,
    limit_b: f32,
    callback: u32,
    context: u32,
) -> bool {
    if !cell.is_null() {
        lock_enter(e, cell);
    }
    let guard = e.global::<u32>(WALK_GUARD);
    if guard & 1 == 0 {
        e.set_global(WALK_GUARD, guard | 1);
        e.call(VISITED_CONSTRUCT, &args![WALK_VISITED]);
        e.call(CRT_ATEXIT, &args![WALK_VISITED_DESTRUCTOR]);
    }
    let guard = e.global::<u32>(WALK_GUARD);
    if guard & 2 == 0 {
        e.set_global(WALK_GUARD, guard | 2);
        e.call(TRIVIAL_CONSTRUCT, &args![WALK_BEST_POSITION]);
    }
    let mut found = false;
    if e.global::<u32>(WALK_DEPTH) == 0 {
        copy_vector(e, WALK_BEST_POSITION, DEFAULT_VECTOR);
        e.set_global(WALK_BEST_WORLD, 0u32);
        let flt_max = e.global::<f32>(FLOAT_MAX);
        e.set_global(WALK_BEST_DISTANCE, flt_max);
    }
    if !cell.is_null() && callback != 0 {
        let depth = e.global::<u32>(WALK_DEPTH);
        e.set_global(WALK_DEPTH, depth.wrapping_add(1));
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), cell.addr());
            e.call(VISITED_ADD, &args![WALK_VISITED, slot]);
        });
        e.with_stack(12, |e, doors| {
            e.call(VISITED_CONSTRUCT, &args![doors]);
            let mut cursor = first_node(e, cell);
            while cursor != 0 && !found {
                let item = node_item(e, cursor);
                cursor = node_next(e, cursor);
                if item.is_null() {
                    continue;
                }
                if skipped_by_ranges(e, item, point_a, limit_a, point_b, limit_b) {
                    continue;
                }
                if e.call(callback, &args![item, context]).u8() != 0 {
                    found = true;
                }
                let base = base_form(e, item);
                if form_type(e, base) != FORM_TYPE_1C {
                    continue;
                }
                if base_form(e, item).addr() == e.global::<u32>(TELEPORT_EXCLUDED_FORM_258) {
                    continue;
                }
                let teleport = e.call(REFERENCE_GET_TELEPORT_DATA, &args![item]).u32();
                if teleport == 0 {
                    continue;
                }
                let target_world = e.call(TELEPORT_GET_WORLD_SPACE, &args![teleport]).u32();
                let mut target_cell = e.call(TELEPORT_GET_CELL, &args![teleport]).u32();
                if target_cell != 0 && !is_interior(e, Ptr::new(target_cell)) {
                    target_cell = 0;
                }
                if target_world != 0 {
                    let room = remaining_range(e, item, point_a, limit_a, point_b, limit_b);
                    if e.global::<u32>(WALK_BEST_WORLD) == 0
                        || e.global::<f32>(WALK_BEST_DISTANCE) < room
                    {
                        e.set_global(WALK_BEST_DISTANCE, room);
                        let position = e.call(TELEPORT_POSITION, &args![teleport]).u32();
                        copy_vector(e, WALK_BEST_POSITION, position);
                        e.set_global(WALK_BEST_WORLD, target_world);
                    }
                } else if target_cell != 0 {
                    let visited = e.with_stack(4, |e, slot| {
                        e.mem.set_u32(slot.addr(), target_cell);
                        e.call(VISITED_FIND, &args![WALK_VISITED, slot, 0u32]).u32()
                    });
                    if visited == 0 {
                        e.with_stack(4, |e, slot| {
                            e.mem.set_u32(slot.addr(), item.addr());
                            e.call(VISITED_ADD, &args![doors, slot]);
                        });
                    }
                }
            }
            let mut position = e.call(SLOT_GET, &args![doors]).u32();
            while position != 0 && !found {
                let door = e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), position);
                    let entry = e.call(CHILD_ARRAY_NEXT, &args![doors, slot]).u32();
                    position = e.mem.u32(slot.addr());
                    e.mem.u32(entry)
                });
                if door != 0 {
                    let door = Ptr::new(door);
                    let room = remaining_range(e, door, point_a, limit_a, point_b, limit_b);
                    let teleport = e.call(REFERENCE_GET_TELEPORT_DATA, &args![door]).u32();
                    let target_position = e.call(TELEPORT_POSITION, &args![teleport]).u32();
                    let target_cell = e.call(TELEPORT_GET_CELL, &args![teleport]).u32();
                    let flt_max = e.global::<f32>(FLOAT_MAX);
                    if !fn_0054d4b0(
                        e,
                        Ptr::new(target_cell),
                        Ptr::new(target_position),
                        room,
                        point_b,
                        flt_max,
                        callback,
                        context,
                    ) {
                        found = true;
                    }
                }
            }
            let depth = e.global::<u32>(WALK_DEPTH);
            e.set_global(WALK_DEPTH, depth.wrapping_sub(1));
            fn_0054da00(e, doors);
        });
    }
    if e.global::<u32>(WALK_DEPTH) == 0 {
        e.call(VISITED_CLEAR, &args![WALK_VISITED]);
        let world = e.global::<u32>(WALK_BEST_WORLD);
        if !found && world != 0 {
            let distance = e.global::<f32>(WALK_BEST_DISTANCE);
            let flt_max = e.global::<f32>(FLOAT_MAX);
            e.call(
                WORLD_SPACE_WALK,
                &args![
                    world,
                    WALK_BEST_POSITION,
                    distance,
                    point_b,
                    flt_max,
                    callback,
                    context
                ],
            );
            found = true;
        }
    }
    if !cell.is_null() {
        lock_leave(e, cell);
    }
    !found
}

// Translated from 0054da00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor of the container of visited cells: `00559810(this)`.
pub fn fn_0054da00(e: &mut Engine, this: Ptr) {
    e.call(VISITED_DESTRUCT, &args![this]);
}

// Translated from 0054da20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `(point_a, limit_a, point_b, limit_b, callback, context)`, `ret 0x18`:
/// calls the cdecl `callback(reference, context)` on every reference of the
/// cell that lies within the limits (the same two range tests as
/// [`fn_0054d4b0`]) and returns 0 at the first yes, else 1. 0 for a null
/// callback.
#[allow(clippy::too_many_arguments)]
pub fn fn_0054da20(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    point_a: Ptr,
    limit_a: f32,
    point_b: Ptr,
    limit_b: f32,
    callback: u32,
    context: u32,
) -> bool {
    if callback == 0 {
        return false;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 && !node_is_end(e, cursor) {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if skipped_by_ranges(e, item, point_a, limit_a, point_b, limit_b) {
            continue;
        }
        if e.call(callback, &args![item, context]).bool() {
            lock_leave(e, this);
            return false;
        }
    }
    lock_leave(e, this);
    true
}

/// A door reference worth listing: no form flag `0x800` / `0x20`, a base
/// form of type `0x1C` other than the form in `011ca258`.
fn is_listed_door(e: &mut Engine, item: Ptr) -> Option<Ptr> {
    if e.call(FORM_FLAG_800, &args![item]).bool() || e.call(FORM_FLAG_20, &args![item]).bool() {
        return None;
    }
    let base = base_form(e, item);
    if form_type(e, base) == FORM_TYPE_1C
        && base.addr() != e.global::<u32>(TELEPORT_EXCLUDED_FORM_258)
    {
        Some(base)
    } else {
        None
    }
}

// Translated from 0054db50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Appends to the array `list` (`ret 4`) every door reference of the cell
/// (no form flag `0x800` or `0x20`, base form of type `0x1C` other than the
/// form in `011ca258`) that has teleport data.
pub fn fn_0054db50(e: &mut Engine, this: Ptr<TESObjectCELL>, list: Ptr) {
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null()
            && is_listed_door(e, item).is_some()
            && e.call(REFERENCE_GET_TELEPORT_DATA, &args![item]).u32() != 0
        {
            push_front_item(e, list, item);
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
}

/// Appends `item` to the array (`007cb2e0(array, &item)`).
fn push_front_item(e: &mut Engine, array: Ptr, item: Ptr) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item.addr());
        e.call(ARRAY_APPEND, &args![array, slot]);
    });
}

// Translated from 0054dc00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `(point, radius, list)`, `ret 0xC`: appends to the array `list` every
/// door reference of the cell (no form flag `0x820`, base form of type
/// `0x1C` other than the form in `011ca258`, with teleport data) whose
/// position is nearer to `point` than `radius` (squared distance below
/// `radius * radius`). Returns the array's size (`0044ddc0`), 0 for a null
/// list.
pub fn fn_0054dc00(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    point: Ptr,
    radius: f32,
    list: Ptr,
) -> u32 {
    if list.is_null() {
        return 0;
    }
    let radius_squared = (f64::from(radius) * f64::from(radius)) as f32;
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null() && !e.call(FORM_FLAG_820, &args![item]).bool() {
            let base = base_form(e, item);
            if form_type(e, base) == FORM_TYPE_1C
                && base.addr() != e.global::<u32>(TELEPORT_EXCLUDED_FORM_258)
                && e.call(REFERENCE_GET_TELEPORT_DATA, &args![item]).u32() != 0
            {
                let position = reference_position(e, item);
                e.with_stack(12, |e, offset| {
                    let difference = e
                        .call(VECTOR_SUBTRACT, &args![point, offset, position])
                        .u32();
                    let squared = e.call(VECTOR_LENGTH_SQUARED, &args![difference]).f32();
                    if f64::from(squared) < f64::from(radius_squared) {
                        push_front_item(e, list, item);
                    }
                });
            }
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
    e.call(ARRAY_COUNT, &args![list]).u32()
}

// Translated from 0054dcf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Appends to the array `list` (`ret 4`) every door reference of the cell
/// (no form flag `0x800` or `0x20`, base form of type `0x1C` other than the
/// form in `011ca258`) that has teleport data or whose base form passes the
/// object filter `005194d0`. Does nothing for a null list.
pub fn fn_0054dcf0(e: &mut Engine, this: Ptr<TESObjectCELL>, list: Ptr) {
    if list.is_null() {
        return;
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 {
        let item = node_item(e, cursor);
        if !item.is_null() && is_listed_door(e, item).is_some() {
            let base = base_form(e, item);
            if e.call(REFERENCE_GET_TELEPORT_DATA, &args![item]).u32() != 0
                || e.call(FILTER_OBJECT_USABLE, &args![base]).bool()
            {
                push_front_item(e, list, item);
            }
        }
        cursor = node_next(e, cursor);
    }
    lock_leave(e, this);
}

// Translated from 0054ddd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::GetWorldSpace` (Xbox PDB): `pWorldSpace` (`+0xC0`) of an
/// exterior cell, null for an interior one.
pub fn tes_object_cell_get_world_space(e: &mut Engine, this: Ptr<TESObjectCELL>) -> Ptr {
    if is_interior(e, this) {
        Ptr::NULL
    } else {
        e.get(this, TESObjectCELL::pWorldSpace)
    }
}

// Translated from 0054de10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::SetWorldSpace` (Xbox PDB), `ret 4`: stores `pWorldSpace`
/// (`+0xC0`) unless the cell is an interior.
pub fn tes_object_cell_set_world_space(e: &mut Engine, this: Ptr<TESObjectCELL>, world: Ptr) {
    if !is_interior(e, this) {
        e.set(this, TESObjectCELL::pWorldSpace, world);
    }
}

// Translated from 0054de40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `+0xC0` of an interior cell (the same field as
/// `pWorldSpace`, which the PDB shares with `iTempDataOffset`), 0 for an
/// exterior cell.
pub fn fn_0054de40(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u32 {
    if is_interior(e, this) {
        e.mem.u32(this.addr() + 0xc0)
    } else {
        0
    }
}

// Translated from 0054de80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the word at `+0xC0` of an interior cell (`ret 4`); nothing for an
/// exterior cell.
pub fn fn_0054de80(e: &mut Engine, this: Ptr<TESObjectCELL>, value: u32) {
    if is_interior(e, this) {
        e.mem.set_u32(this.addr() + 0xc0, value);
    }
}

// Translated from 0054deb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For an interior cell, the word at `+0x28` of the object `00544600`
/// returns (the cell's interior data); 0 otherwise or when there is none.
pub fn fn_0054deb0(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u32 {
    if is_interior(e, this) {
        let data = e.call(CELL_INTERIOR_DATA, &args![this]).u32();
        if data != 0 {
            return e.mem.u32(data + 0x28);
        }
    }
    0
}

// Translated from 0054def0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` at `+0x28` of the interior data of an interior cell
/// (`ret 4`), when there is any.
pub fn fn_0054def0(e: &mut Engine, this: Ptr<TESObjectCELL>, value: u32) {
    if is_interior(e, this) {
        let data = e.call(CELL_INTERIOR_DATA, &args![this]).u32();
        if data != 0 {
            e.mem.set_u32(data + 0x28, value);
        }
    }
}

/// Clears the dropped items of the list node `list` (an `ExtraDataList`
/// dropped item list) one by one: each item's own entry is cleared
/// (`AddDroppedItem(extra, 0)`), the item is marked deleted unless its
/// virtual slot `0x160` says no, and the head node is dropped.
fn drop_items(e: &mut Engine, list: u32) {
    while list != 0 && !node_is_end(e, list) {
        let dropped = node_item(e, list);
        let extra = reference_extra_list(e, dropped);
        e.call(EXTRA_LIST_ADD_DROPPED_ITEM, &args![extra, 0u32]);
        if !e.vcall(dropped.addr(), REFERENCE_SLOT_160, &args![]).bool() {
            e.call(REFERENCE_MARK_AS_DELETED, &args![dropped]);
        }
        e.call(LIST_POP_FRONT, &args![list]);
    }
}

/// The dropped item list of the actor, cleared by [`drop_items`] and then
/// removed from the actor's extra data (nothing when it has none).
fn clear_dropped_items(e: &mut Engine, actor: Ptr) {
    let extra = reference_extra_list(e, actor);
    let list = e
        .call(EXTRA_LIST_GET_DROPPED_ITEM_LIST, &args![extra])
        .u32();
    if list != 0 {
        drop_items(e, list);
        let extra = reference_extra_list(e, actor);
        e.call(EXTRA_LIST_REMOVE_DROPPED_ITEM_LIST, &args![extra]);
    }
}

// Translated from 0054df30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Goes through the references of the cell when it is loaded again after
/// having been detached, answering whether the detach lasted long enough
/// (0 at once when the game loader flag `0042ce10` is set).
///
/// With no detach time (`TESObjectCELL::GetDetachTime`) the persistent
/// references of type `0x1B` (and those of types `0x2A..=0x2B` that pass the
/// actor time check `00881c90`) get their virtual slot `0x208` called with
/// "`0056ae60` says no", and the answer is 1.
///
/// Otherwise the elapsed game time (`00867e30`) is compared with the delay
/// (`00526100`): a detach time of `-1` makes `answer` and the "long enough"
/// flag true, as does an elapsed time over the delay; with an encounter zone
/// on the cell `answer` is what the zone says (`005262a0`, `00526120`), else
/// the flag. Every reference then goes through these steps in order: if the
/// detach time was `-1` and the data handler knows the reference's id
/// (below `0xFF000000`), its virtual slot `0xC4(1)` runs unless slot `0x160`
/// or `00577de0` say no, and the next reference follows. An actor that fails
/// the time check is skipped. Any other reference with form flag
/// `0x1000000` gets its destruction applied and (with a model swap) form
/// flag `0x2000` set when the delay was long enough; it is skipped when its
/// encounter zone differs from the cell's and fails `005262a0`, or equals it
/// while `answer` is false. Then an actor that `0087f4a0` calls dead is
/// emptied (slot `0x324(1, 0, 0)`, dropped items cleared) and put back where
/// it was if it belongs to the cell (`0x298`) or to the cell's world space at
/// a position inside the cell; an actor for whom slot `0x22C` says yes is
/// deleted with its ash pile and dropped items. A container (type `0x1B`)
/// is unlocked and locked again, a type `0x1D` form with flag `0x20` may be
/// re-attached (slot `0xC4(0)`), a type `0x26` one is emptied. Last, each
/// reference gets its action list reset (`InitActionList`,
/// `SetActionFlag(0x80000000)`), and `00546b10(0, 0)` clears the detach time.
///
/// The compiler's exception frame is not translated.
pub fn fn_0054df30(e: &mut Engine, this: Ptr<TESObjectCELL>) -> u8 {
    let loader = e.global::<u32>(GAME_LOADER_POINTER);
    if e.call(GAME_LOADER_FLAG_244_2, &args![loader]).bool() {
        return 0;
    }
    let now = e.call(GAME_TIME_NOW, &args![GAME_TIME_OBJECT]).u32();
    let detach = e.call(CELL_GET_DETACH_TIME, &args![this]).u32();
    if detach == 0 {
        lock_enter(e, this);
        let mut cursor = first_node(e, this);
        while cursor != 0 && !node_is_end(e, cursor) {
            let item = node_item(e, cursor);
            cursor = node_next(e, cursor);
            let base = base_form(e, item);
            let kind = form_type(e, base);
            if kind == 0x1b {
                if e.call(REFERENCE_GET_REF_PERSISTS, &args![item]).bool() {
                    let none = !e.call(REFERENCE_TEST_56AE60, &args![item]).bool();
                    e.vcall(item.addr(), REFERENCE_SLOT_208, &args![none as u32]);
                }
            } else if kind > 0x29
                && kind <= 0x2b
                && e.call(REFERENCE_GET_REF_PERSISTS, &args![item]).bool()
                && e.call(ACTOR_TIME_CHECK, &args![item, now]).bool()
            {
                let none = !e.call(REFERENCE_TEST_56AE60, &args![item]).bool();
                e.vcall(item.addr(), REFERENCE_SLOT_208, &args![none as u32]);
            }
        }
        lock_leave(e, this);
        return 1;
    }
    let answer: u8;
    let mut forever = false;
    let mut long_enough = false;
    let zone = e.call(CELL_ENCOUNTER_ZONE, &args![this]).u32();
    if detach == u32::MAX {
        answer = 1;
        forever = true;
        long_enough = true;
    } else {
        let elapsed = now.wrapping_sub(detach);
        let delay = e.call(ENCOUNTER_ZONE_DELAY, &args![]).u32();
        if elapsed > delay {
            long_enough = true;
        }
        if zone != 0 {
            answer = if e.call(ENCOUNTER_ZONE_TEST, &args![zone]).bool() {
                e.call(ENCOUNTER_ZONE_TEST_CELL, &args![zone, this]).u8()
            } else {
                0
            };
        } else {
            answer = long_enough as u8;
        }
    }
    lock_enter(e, this);
    let mut cursor = first_node(e, this);
    while cursor != 0 && !node_is_end(e, cursor) {
        let item = node_item(e, cursor);
        cursor = node_next(e, cursor);
        if forever {
            let id = form_id(e, item);
            let handler = e.global::<u32>(DATA_HANDLER_POINTER);
            if e.call(DATA_HANDLER_ID_TEST, &args![handler, id]).bool() {
                if !e.vcall(item.addr(), REFERENCE_SLOT_160, &args![]).bool()
                    && !e.call(REFERENCE_TEST_577DE0, &args![item]).bool()
                {
                    e.vcall(item.addr(), REFERENCE_SLOT_C4, &args![1u32]);
                }
                continue;
            }
        }
        let actor = if is_actor(e, item) { item } else { Ptr::NULL };
        if !actor.is_null() {
            if !e.call(ACTOR_TIME_CHECK, &args![actor, now]).bool() {
                continue;
            }
        } else {
            if long_enough && e.call(REFERENCE_FLAG_1000000, &args![item]).bool() {
                let base = base_form(e, item);
                let destruction = e.call(BASE_GET_DESTRUCTION_FORM, &args![base]).u32();
                e.call(DESTRUCTION_APPLY, &args![destruction, item]);
                let list = reference_extra_list(e, item);
                if e.call(EXTRA_LIST_GET_MODEL_SWAP, &args![list]).u32() != 0 {
                    e.call(FORM_SET_FLAG_2000, &args![item, 1u32]);
                }
            }
            let item_zone = e.call(REFERENCE_GET_ENCOUNTER_ZONE, &args![item]).u32();
            if item_zone == zone {
                if answer == 0 {
                    continue;
                }
            } else if item_zone != 0 && !e.call(ENCOUNTER_ZONE_TEST, &args![item_zone]).bool() {
                continue;
            }
        }
        if !actor.is_null() {
            if e.call(ACTOR_TEST_87F4A0, &args![actor]).bool() {
                e.vcall(actor.addr(), ACTOR_SLOT_324, &args![1u32, 0u32, 0u32]);
                clear_dropped_items(e, actor);
                e.with_stack(12, |e, placed| {
                    e.vcall(actor.addr(), REFERENCE_SLOT_170, &args![placed]);
                    let mut back = false;
                    if e.vcall(actor.addr(), REFERENCE_SLOT_298, &args![]).u32() == this.addr() {
                        back = true;
                    } else {
                        let world = e
                            .vcall(actor.addr(), REFERENCE_SLOT_294, &args![])
                            .ptr::<()>();
                        if world == tes_object_cell_get_world_space(e, this)
                            && e.call(CELL_POSITION_FITS, &args![this, placed]).bool()
                        {
                            back = true;
                        }
                    }
                    if back {
                        e.call(REFERENCE_SET_LOCATION, &args![actor, placed]);
                        e.with_stack(12, |e, buffer| {
                            let rotation = e
                                .vcall(actor.addr(), REFERENCE_SLOT_16C, &args![buffer])
                                .u32();
                            let words = [
                                e.mem.u32(rotation),
                                e.mem.u32(rotation + 4),
                                e.mem.u32(rotation + 8),
                            ];
                            e.call(
                                REFERENCE_SET_ROTATION,
                                &args![actor, words[0], words[1], words[2]],
                            );
                        });
                    }
                });
            } else if e.vcall(actor.addr(), ACTOR_SLOT_22C, &args![0u32]).bool() {
                if !e.vcall(actor.addr(), REFERENCE_SLOT_160, &args![]).bool()
                    && !e.call(REFERENCE_TEST_577DE0, &args![actor]).bool()
                {
                    e.call(REFERENCE_MARK_AS_DELETED, &args![actor]);
                    let extra = reference_extra_list(e, actor);
                    let ash = e.call(EXTRA_LIST_GET_ASH_PILE_REF, &args![extra]).u32();
                    if ash != 0 {
                        e.call(REFERENCE_MARK_AS_DELETED, &args![ash]);
                    }
                }
                clear_dropped_items(e, actor);
            }
        } else {
            let base = base_form(e, item);
            let kind = form_type(e, base);
            if kind == 0x1b {
                if e.call(REFERENCE_TEST_56AE60, &args![item]).bool() {
                    e.vcall(item.addr(), REFERENCE_SLOT_208, &args![0u32]);
                    let list = reference_extra_list(e, item);
                    if e.call(EXTRA_LIST_GET_CONTAINER_CHANGES, &args![list]).u32() != 0 {
                        let list = reference_extra_list(e, item);
                        let changes = e.call(EXTRA_LIST_GET_CONTAINER_CHANGES, &args![list]).u32();
                        if !e.call(CHANGES_TEST_42CDE0, &args![changes]).bool() {
                            e.call(REFERENCE_LOCK, &args![item]);
                        }
                    }
                }
            } else if kind == 0x1d {
                if e.call(FORM_FLAG_20, &args![item]).bool()
                    && !e.call(REFERENCE_TEST_565450, &args![item]).bool()
                    && !e.vcall(item.addr(), REFERENCE_SLOT_160, &args![]).bool()
                {
                    let base = base_form(e, item);
                    let cast = e
                        .call(
                            RT_DYNAMIC_CAST,
                            &args![base, 0u32, RTTI_FORM_FILTER_FROM, RTTI_CAST_11839DC, 0u32],
                        )
                        .u32();
                    if cast != 0 && e.vcall(cast + 0x3c, 4, &args![]).bool() {
                        e.vcall(item.addr(), REFERENCE_SLOT_C4, &args![0u32]);
                    }
                }
            } else if kind == 0x26 {
                e.call(FORM_SET_EMPTY, &args![item, 0u32]);
            }
        }
        let list = reference_extra_list(e, item);
        e.call(SCRIPT_INIT_ACTION_LIST, &args![item, list]);
        let list = reference_extra_list(e, item);
        e.call(SCRIPT_SET_ACTION_FLAG, &args![item, list, 0x8000_0000u32]);
    }
    lock_leave(e, this);
    e.call(CELL_SET_DETACH_TIME, &args![this, 0u32, 0u32]);
    answer
}

// Translated from 0054e640 (decompiled, FalloutNV.exe 1.4.0.525)
/// The exterior counterpart of [`tes_object_cell_get_interior_local_map_texture`]
/// (the exe's string is "GetExteriorLocalMapTexture"), `ret 4`: for an
/// exterior cell takes the local map picture
/// (`TESObjectCELL::TakeExteriorLocalMapPicture`), reads its texture
/// (`BSRenderedTexture::GetTexture(0)`), empties the picture's four texture
/// slots ([`fn_0054e710`]) and puts the texture into `out` (a smart pointer
/// assignment). Without a picture `out` is assigned an empty pointer.
pub fn fn_0054e640(e: &mut Engine, this: Ptr<TESObjectCELL>, out: Ptr) {
    e.with_stack(8, |e, frame| {
        let texture = frame;
        let picture = frame.byte_add(4);
        e.call(SLOT_CONSTRUCT, &args![texture, 0u32]);
        if !is_interior(e, this) {
            e.call(SLOT_CONSTRUCT, &args![picture, 0u32]);
            e.call(TAKE_EXTERIOR_LOCAL_MAP_PICTURE, &args![this, picture]);
            let rendered = slot_get(e, picture);
            let target = e
                .call(RENDERED_TEXTURE_GET_TEXTURE, &args![rendered, 0u32])
                .u32();
            e.call(SLOT_ASSIGN, &args![texture, target]);
            let rendered = slot_get(e, picture);
            fn_0054e710(e, Ptr::new(rendered));
            e.call(SLOT_RELEASE, &args![picture]);
        }
        e.call(SMART_POINTER_ASSIGN, &args![out, texture]);
        e.call(SLOT_RELEASE, &args![texture]);
    });
}

// Translated from 0054e710 (decompiled, FalloutNV.exe 1.4.0.525)
/// Empties the four smart pointer slots at `this + 0x30 .. 0x40` (the
/// rendered textures a local map picture holds).
pub fn fn_0054e710(e: &mut Engine, this: Ptr) {
    for i in 0..4u32 {
        e.call(SLOT_ASSIGN, &args![this.byte_add(0x30 + 4 * i), 0u32]);
    }
}

// Translated from 0054e750 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::GetInteriorLocalMapTexture` (Xbox PDB), `ret 0xC`: for an
/// interior cell takes the interior local map picture (`0054f500(cell, a, b,
/// &picture)`), reads its texture, empties the picture's four texture slots
/// and puts the texture into `out`, exactly as [`fn_0054e640`] does for an
/// exterior cell. `a` and `b` are only passed on.
pub fn tes_object_cell_get_interior_local_map_texture(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    a: u32,
    b: u32,
    out: Ptr,
) {
    e.with_stack(8, |e, frame| {
        let texture = frame;
        let picture = frame.byte_add(4);
        e.call(SLOT_CONSTRUCT, &args![texture, 0u32]);
        if is_interior(e, this) {
            e.call(SLOT_CONSTRUCT, &args![picture, 0u32]);
            e.call(TAKE_INTERIOR_LOCAL_MAP_PICTURE, &args![this, a, b, picture]);
            let rendered = slot_get(e, picture);
            let target = e
                .call(RENDERED_TEXTURE_GET_TEXTURE, &args![rendered, 0u32])
                .u32();
            e.call(SLOT_ASSIGN, &args![texture, target]);
            let rendered = slot_get(e, picture);
            fn_0054e710(e, Ptr::new(rendered));
            e.call(SLOT_RELEASE, &args![picture]);
        }
        e.call(SMART_POINTER_ASSIGN, &args![out, texture]);
        e.call(SLOT_RELEASE, &args![texture]);
    });
}

/// Ends or starts the offscreen pass of the picture: with `main_pass` set
/// the target of the rendered texture is handed over (`(7, target)`),
/// otherwise [`fn_0054ede0`] does it.
fn set_picture_target(e: &mut Engine, main_pass: bool, texture: Ptr) {
    if main_pass {
        let target = e.call(RENDERED_TEXTURE_TARGET, &args![texture]).u32();
        e.call(SET_RENDER_TARGET, &args![7u32, target]);
    } else {
        fn_0054ede0(e, texture, 7);
    }
}

// Translated from 0054e830 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::TakeLocalMapPicture` (Xbox PDB), `ret 8`: draws the scene
/// below `root` (and the cell's child node 6) from above into a rendered
/// texture and puts that texture into the smart pointer `out`.
///
/// It makes a scope guard, saves the shader manager's accumulator, builds a
/// culling process, and gets two textures from the texture manager (kinds
/// `0x1E` and `0x1C`); if both exist it clears the colour of the render
/// target holder, switches the shadow scene node's flag at `+0x130` on and
/// sets the water object's flag, creates a shader accumulator, accumulates
/// `root` (culling mode 3) and the cell's child node 6 (culling mode 1) into
/// it, finishes the accumulation, applies the image space effect `0xE`
/// from one texture to the other, then restores everything (flags, colour,
/// accumulator). The second texture is put into `out` and the first goes back
/// to the texture manager. Both end offscreen passes depend on the water
/// code's flag (`004e9510`).
///
/// The compiler's exception frame is not translated.
pub fn tes_object_cell_take_local_map_picture(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    root: Ptr,
    out: Ptr,
) {
    e.with_stack(0x1c0, |e, frame| {
        // Local objects of the game's frame, laid out here as: the scope
        // guard, the saved accumulator slot, the unused slot, the two
        // textures, the shader accumulator slot, two colours and the
        // culling process.
        let guard = frame;
        let saved = frame.byte_add(0x20);
        let unused = frame.byte_add(0x24);
        let second = frame.byte_add(0x28);
        let first = frame.byte_add(0x2c);
        let accumulator = frame.byte_add(0x30);
        let color = frame.byte_add(0x40);
        let color_2 = frame.byte_add(0x50);
        let culling = frame.byte_add(0x60);
        e.call(
            PROFILE_SCOPE_CONSTRUCT,
            &args![guard, 0xeu32, 1u32, 0x0102_ed68u32, 0x2251u32],
        );
        let current = e.call(SHADER_GET_ACCUMULATOR, &args![]).u32();
        e.call(SLOT_CONSTRUCT, &args![saved, current]);
        e.call(CULLING_PROCESS_CONSTRUCT, &args![culling, 0u32]);
        e.call(SLOT_CONSTRUCT, &args![unused, 0u32]);
        e.call(SLOT_CONSTRUCT, &args![second, 0u32]);
        e.call(SLOT_CONSTRUCT, &args![first, 0u32]);
        let holder = e.call(RENDER_TARGET_HOLDER, &args![]).u32();
        let manager = e.call(TEXTURE_MANAGER, &args![]).u32();
        let created = e
            .call(
                TEXTURE_MANAGER_CREATE_A,
                &args![manager, holder, 0x1eu32, 0u32, 0u32, 0u32],
            )
            .u32();
        e.call(SLOT_ASSIGN, &args![first, created]);
        let holder = e.call(RENDER_TARGET_HOLDER, &args![]).u32();
        let manager = e.call(TEXTURE_MANAGER, &args![]).u32();
        let created = e
            .call(
                TEXTURE_MANAGER_CREATE_B,
                &args![manager, holder, 0x1cu32, 0u32, 0u32],
            )
            .u32();
        e.call(SLOT_ASSIGN, &args![second, created]);
        if slot_get(e, second) != 0 && slot_get(e, first) != 0 {
            let zero = 0.0f32;
            e.call(COLOR_CONSTRUCT, &args![color, zero, zero, zero, zero]);
            let holder = e.call(RENDER_TARGET_HOLDER, &args![]).u32();
            e.vcall(holder, HOLDER_SLOT_GET_COLOR, &args![color]);
            let holder = e.call(RENDER_TARGET_HOLDER, &args![]).u32();
            let clear = e
                .call(COLOR_CONSTRUCT, &args![color_2, zero, zero, zero, 1.0f32])
                .u32();
            e.vcall(holder, HOLDER_SLOT_SET_COLOR, &args![clear]);
            let scene_node = e.call(SHADOW_SCENE_NODE_GETTER, &args![0u32]).u32();
            let old_flag = e.call(SHADOW_SCENE_NODE_GET_FLAG, &args![scene_node]).u8();
            e.call(SHADOW_SCENE_NODE_SET_FLAG, &args![scene_node, 1u32]);
            let main_pass = e.call(WATER_RENDER_FLAG, &args![]).bool();
            let water = e.call(WATER_OBJECT, &args![]).u32();
            let mut water_flag = 0u8;
            if water != 0 {
                water_flag = e.call(WATER_OBJECT_GET_FLAG, &args![water]).u8();
                e.call(WATER_OBJECT_SET_FLAG, &args![water, 1u32]);
            }
            let picture_flag = fn_0054ee20(e);
            e.call(MAP_PICTURE_SET_FLAG, &args![0u32]);
            let target = slot_get(e, first);
            set_picture_target(e, main_pass, Ptr::new(target));
            let memory = e.call(NODE_ALLOCATE, &args![0x280u32]).u32();
            let made = if memory != 0 {
                e.call(
                    SHADER_ACCUMULATOR_CONSTRUCT,
                    &args![memory, 0x63u32, 1u32, 0x2f7u32],
                )
                .u32()
            } else {
                0
            };
            e.call(SLOT_CONSTRUCT, &args![accumulator, made]);
            let object = slot_get(e, accumulator);
            e.call(ACCUMULATOR_SET_SCENE_NODE, &args![object, scene_node]);
            let object = slot_get(e, accumulator);
            e.call(CULLING_PROCESS_SET_ACCUMULATOR, &args![culling, object]);
            let object = slot_get(e, accumulator);
            e.call(ACCUMULATOR_SET_MODE, &args![object, 9u32]);
            e.call(CULLING_PROCESS_BEGIN, &args![culling, 3u32]);
            e.call(ACCUMULATE_SCENE, &args![root, scene_node, culling]);
            e.call(CULLING_PROCESS_END, &args![culling]);
            if e.call(CELL_CHILD_NODE, &args![this, 6u32]).u32() != 0 {
                e.call(CULLING_PROCESS_BEGIN, &args![culling, 1u32]);
                let node = e.call(CELL_CHILD_NODE, &args![this, 6u32]).u32();
                e.call(ACCUMULATE_SCENE, &args![root, node, culling]);
                e.call(CULLING_PROCESS_END, &args![culling]);
            }
            let object = slot_get(e, accumulator);
            e.call(FINISH_ACCUMULATION, &args![root, object, 0u32]);
            e.call(CULLING_PROCESS_SET_ACCUMULATOR, &args![culling, 0u32]);
            e.call(SLOT_ASSIGN, &args![accumulator, 0u32]);
            if main_pass {
                e.call(END_OFFSCREEN_MAIN, &args![]);
            } else {
                e.call(END_OFFSCREEN, &args![]);
            }
            e.call(MAP_PICTURE_SET_FLAG, &args![picture_flag as u32]);
            if water != 0 {
                e.call(WATER_OBJECT_SET_FLAG, &args![water, water_flag as u32]);
            }
            let target = slot_get(e, second);
            set_picture_target(e, main_pass, Ptr::new(target));
            let manager = e.call(IMAGE_SPACE_MANAGER, &args![]).u32();
            let effect = e
                .call(IMAGE_SPACE_GET_EFFECT, &args![manager, 0xeu32])
                .u32();
            let source_b = slot_get(e, second);
            let source_a = slot_get(e, first);
            let holder = e.call(RENDER_TARGET_HOLDER, &args![]).u32();
            let manager = e.call(IMAGE_SPACE_MANAGER, &args![]).u32();
            e.call(
                IMAGE_SPACE_RENDER_EFFECT,
                &args![manager, effect, holder, source_a, source_b, 0u32, 0u32],
            );
            if main_pass {
                e.call(END_OFFSCREEN_MAIN, &args![]);
            } else {
                e.call(END_OFFSCREEN, &args![]);
            }
            e.call(
                SHADOW_SCENE_NODE_SET_FLAG,
                &args![scene_node, old_flag as u32],
            );
            let current = e.call(SHADER_GET_ACCUMULATOR, &args![]).u32();
            e.call(ACCUMULATOR_SET_FLAG, &args![current, 1u32]);
            let holder = e.call(RENDER_TARGET_HOLDER, &args![]).u32();
            e.vcall(holder, HOLDER_SLOT_SET_COLOR, &args![color]);
            e.call(SLOT_RELEASE, &args![accumulator]);
        }
        let returned = slot_get(e, first);
        let manager = e.call(TEXTURE_MANAGER, &args![]).u32();
        e.call(TEXTURE_MANAGER_RETURN, &args![manager, returned]);
        e.call(SMART_POINTER_ASSIGN, &args![out, second]);
        let previous = slot_get(e, saved);
        e.call(SHADER_SET_ACCUMULATOR, &args![previous]);
        e.call(SLOT_RELEASE, &args![first]);
        e.call(SLOT_RELEASE, &args![second]);
        e.call(SLOT_RELEASE, &args![unused]);
        e.call(CULLING_PROCESS_DESTRUCT, &args![culling]);
        e.call(SLOT_RELEASE, &args![saved]);
        e.call(PROFILE_SCOPE_DESTRUCT, &args![guard]);
    });
}

// Translated from 0054ede0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl `(texture, flag)`: hands the target of the rendered `texture` (0 for
/// a null texture) and `flag` to `00b6b7d0`.
pub fn fn_0054ede0(e: &mut Engine, texture: Ptr, flag: u32) {
    let target = if texture.is_null() {
        0
    } else {
        e.call(RENDERED_TEXTURE_TARGET, &args![texture]).u32()
    };
    e.call(SET_RENDER_TARGET_FLAGGED, &args![target, flag]);
}

// Translated from 0054ee20 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `0051d740` says yes and the byte at `+0x18` of the object
/// `00664840(1)` returns is set.
pub fn fn_0054ee20(e: &mut Engine) -> u8 {
    if e.call(MAP_PICTURE_TEST, &args![]).bool() {
        let object = e.call(MAP_PICTURE_OBJECT, &args![1u32]).u32();
        if e.mem.u8(object + 0x18) != 0 {
            return 1;
        }
    }
    0
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00547650, fn_00547650(Ptr<TESObjectCELL>) -> u32),
        entry!(0x00547680, fn_00547680(Ptr<TESObjectCELL>) -> Ptr),
        entry!(0x00547750, fn_00547750(Ptr<TESObjectCELL>, u32)),
        entry!(
            0x00547770,
            tes_object_cell_get_water_type(Ptr<TESObjectCELL>) -> Ptr
        ),
        entry!(0x005477f0, fn_005477f0(Ptr<TESObjectCELL>, Ptr, Ptr) -> u32),
        entry!(
            0x00547830,
            tes_object_cell_create_canopy_shadow_mask_for_cell(
                Ptr<TESObjectCELL>,
                Ptr,
                Ptr,
            ) -> bool
        ),
        entry!(
            0x005479c0,
            fn_005479c0(Ptr<TESObjectCELL>, i32, i32, Ptr, u32, u8)
        ),
        entry!(
            0x00547ad0,
            fn_00547ad0(Ptr<TESObjectCELL>, f32, f32, f32, u32, u32)
        ),
        entry!(0x00548210, fn_00548210(Ptr) -> Ptr),
        entry!(
            0x00548230,
            tes_object_cell_add_reference(Ptr<TESObjectCELL>, Ptr, u8)
        ),
        entry!(0x00548700, fn_00548700(Ptr<TESObjectCELL>) -> bool),
        entry!(0x00548720, fn_00548720(Ptr<TESObjectCELL>) -> bool),
        entry!(0x00548740, fn_00548740(Ptr<TESObjectCELL>, Ptr)),
        entry!(
            0x00548880,
            tes_object_cell_attach_reference_3d(Ptr<TESObjectCELL>, Ptr, u32)
        ),
        entry!(0x00549580, fn_00549580(Ptr) -> bool),
        entry!(0x005495a0, fn_005495a0(Ptr<TESObjectCELL>) -> bool),
        entry!(0x005495d0, fn_005495d0(Ptr<TESObjectCELL>) -> Ptr),
        entry!(0x005495f0, fn_005495f0(Ptr<TESObjectCELL>) -> Ptr),
        entry!(0x00549610, fn_00549610(Ptr, u32)),
        entry!(0x00549630, fn_00549630(Ptr) -> bool),
        entry!(0x00549690, fn_00549690(Ptr) -> bool),
        entry!(
            0x005496b0,
            tes_object_cell_perform_cell_node_attach(Ptr<TESObjectCELL>, Ptr, Ptr)
        ),
        entry!(0x005497a0, fn_005497a0(Ptr<TESObjectCELL>) -> Ptr),
        entry!(
            0x005497c0,
            fn_005497c0(Ptr<TESObjectCELL>, Ptr, u32, u32, Ptr, bool) -> bool
        ),
        entry!(0x0054a050, fn_0054a050(Ptr<TESObjectCELL>) -> Ptr),
        entry!(0x0054a070, fn_0054a070(Ptr<TESObjectCELL>, Ptr, u8, u8)),
        entry!(0x0054aa60, fn_0054aa60(Ptr<TESObjectCELL>) -> Ptr),
        entry!(
            0x0054aa80,
            check_within_multi_bound_task_check_within_multi_bound_task(Ptr, Ptr, u32) -> Ptr
        ),
        entry!(
            0x0054ab30,
            check_within_multi_bound_task_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x0054ab60, fn_0054ab60(Ptr)),
        entry!(0x0054abd0, fn_0054abd0(Ptr, Ptr)),
        entry!(0x0054acf0, fn_0054acf0(Ptr) -> Ptr),
        entry!(0x0054ad60, fn_0054ad60(Ptr, u32) -> Ptr),
        entry!(0x0054adb0, fn_0054adb0(Ptr, Ptr) -> Ptr),
        entry!(0x0054ae30, tes_object_cell_perform_queued_child_attaches()),
        entry!(0x0054af40, fn_0054af40(Ptr<TESObjectCELL>, u8)),
        entry!(0x0054af80, fn_0054af80(Ptr<TESObjectCELL>)),
        entry!(0x0054afb0, fn_0054afb0(Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x0054b260, fn_0054b260(Ptr<TESObjectCELL>, Ptr, Ptr)),
        entry!(
            0x0054b5b0,
            tes_object_cell_save_game_test(Ptr<TESObjectCELL>)
        ),
        entry!(0x0054b750, fn_0054b750(Ptr<TESObjectCELL>)),
        entry!(0x0054b800, fn_0054b800(Ptr)),
        entry!(0x0054b830, fn_0054b830(Ptr<TESObjectCELL>, Ptr)),
        entry!(0x0054b8c0, fn_0054b8c0(Ptr<TESObjectCELL>, Ptr)),
        entry!(0x0054b950, fn_0054b950(Ptr<TESObjectCELL>, u8)),
        entry!(0x0054ba00, fn_0054ba00(Ptr<TESObjectCELL>)),
        entry!(
            0x0054ba80,
            tes_object_cell_attach_lights(Ptr<TESObjectCELL>, u8)
        ),
        entry!(0x0054baf0, fn_0054baf0(Ptr<TESObjectCELL>)),
        entry!(0x0054bb90, fn_0054bb90(Ptr<TESObjectCELL>) -> u32),
        entry!(0x0054bc10, fn_0054bc10(Ptr<TESObjectCELL>)),
        entry!(0x0054bcf0, fn_0054bcf0(Ptr<TESObjectCELL>, u8)),
        entry!(
            0x0054c030,
            tes_object_cell_calc_ref_center_point(Ptr<TESObjectCELL>, Ptr)
        ),
        entry!(
            0x0054c740,
            tes_object_cell_run_scripts(Ptr<TESObjectCELL>, u8, u32) -> u8
        ),
        entry!(
            0x0054c8c0,
            tes_object_cell_assign_persistent_refs_to_cells_in_world(Ptr<TESObjectCELL>, Ptr)
        ),
        entry!(
            0x0054ca00,
            tes_object_cell_add_furniture_to_list(Ptr<TESObjectCELL>, Ptr)
        ),
        entry!(
            0x0054ca90,
            tes_object_cell_remove_reference(Ptr<TESObjectCELL>, Ptr)
        ),
        entry!(0x0054cd20, fn_0054cd20(Ptr<TESObjectCELL>)),
        entry!(
            0x0054cee0,
            tes_object_cell_get_first_refr(Ptr<TESObjectCELL>) -> Ptr
        ),
        entry!(0x0054cf20, fn_0054cf20(Ptr<TESObjectCELL>, Ptr, u8) -> Ptr),
        entry!(
            0x0054cfd0,
            tes_object_cell_get_coc_placement_info(Ptr<TESObjectCELL>, Ptr, Ptr)
        ),
        entry!(
            0x0054d090,
            tes_object_cell_determine_coc_placement(Ptr<TESObjectCELL>, Ptr, Ptr)
        ),
        entry!(
            0x0054d4b0,
            fn_0054d4b0(Ptr<TESObjectCELL>, Ptr, f32, Ptr, f32, u32, u32) -> bool
        ),
        entry!(0x0054da00, fn_0054da00(Ptr)),
        entry!(
            0x0054da20,
            fn_0054da20(Ptr<TESObjectCELL>, Ptr, f32, Ptr, f32, u32, u32) -> bool
        ),
        entry!(0x0054db50, fn_0054db50(Ptr<TESObjectCELL>, Ptr)),
        entry!(
            0x0054dc00,
            fn_0054dc00(Ptr<TESObjectCELL>, Ptr, f32, Ptr) -> u32
        ),
        entry!(0x0054dcf0, fn_0054dcf0(Ptr<TESObjectCELL>, Ptr)),
        entry!(
            0x0054ddd0,
            tes_object_cell_get_world_space(Ptr<TESObjectCELL>) -> Ptr
        ),
        entry!(
            0x0054de10,
            tes_object_cell_set_world_space(Ptr<TESObjectCELL>, Ptr)
        ),
        entry!(0x0054de40, fn_0054de40(Ptr<TESObjectCELL>) -> u32),
        entry!(0x0054de80, fn_0054de80(Ptr<TESObjectCELL>, u32)),
        entry!(0x0054deb0, fn_0054deb0(Ptr<TESObjectCELL>) -> u32),
        entry!(0x0054def0, fn_0054def0(Ptr<TESObjectCELL>, u32)),
        entry!(0x0054df30, fn_0054df30(Ptr<TESObjectCELL>) -> u8),
        entry!(0x0054e640, fn_0054e640(Ptr<TESObjectCELL>, Ptr)),
        entry!(0x0054e710, fn_0054e710(Ptr)),
        entry!(
            0x0054e750,
            tes_object_cell_get_interior_local_map_texture(Ptr<TESObjectCELL>, u32, u32, Ptr)
        ),
        entry!(
            0x0054e830,
            tes_object_cell_take_local_map_picture(Ptr<TESObjectCELL>, Ptr, Ptr)
        ),
        entry!(0x0054ede0, fn_0054ede0(Ptr, u32)),
        entry!(0x0054ee20, fn_0054ee20() -> u8),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// The vtable of every test object. Slot `s` leads to the double at
    /// `fake(s)`, which answers with the word at `ANSWERS + s` of the object,
    /// so one object type serves as a cell, a reference, a node or a room.
    const TEST_VTABLE: u32 = 0x0130_0000;
    const ANSWERS: u32 = 0x400;
    const OBJECT_SIZE: u32 = 0x900;
    const LAST_SLOT: u32 = 0x470;

    type Log = Vec<(u32, Vec<u32>)>;

    fn fake(slot: u32) -> u32 {
        0x7000_0000 + slot
    }

    fn calls_to(log: &Log, address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The addresses called, in order, without the call under test.
    fn sequence(log: &Log) -> Vec<u32> {
        log.iter().skip(1).map(|(a, _)| *a).collect()
    }

    /// Doubles that do nothing and return 0.
    fn quiet(e: &mut Engine, addresses: &[u32]) {
        for address in addresses {
            e.register(*address, |_, _| Ret::default());
        }
    }

    /// A double that returns `value`.
    fn returns(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| value.into_ret());
    }

    /// A double that tests `iFormFlags & mask` of its object.
    fn flag_test(e: &mut Engine, address: u32, mask: u32) {
        e.register_double(address, move |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & mask != 0).into_ret()
        });
    }

    /// A zeroed object of the test type.
    fn object(e: &mut Engine) -> u32 {
        let o = e.mem.alloc(OBJECT_SIZE);
        e.mem.set_u32(o, TEST_VTABLE);
        o
    }

    fn cell(e: &mut Engine) -> Ptr<TESObjectCELL> {
        Ptr::new(object(e))
    }

    /// Makes slot `slot` of `o` answer `value`.
    fn answer(e: &mut Engine, o: u32, slot: u32, value: u32) {
        e.mem.set_u32(o + ANSWERS + slot, value);
    }

    /// A form object: type byte at `+4`, flags at `+8`, id at `+0xC`.
    fn form(e: &mut Engine, form_type: u8, flags: u32, id: u32) -> u32 {
        let f = object(e);
        e.mem.set_u8(f + 4, form_type);
        e.mem.set_u32(f + 8, flags);
        e.mem.set_u32(f + 0xc, id);
        f
    }

    /// A reference whose base form is `base`, with its 3D (`+0x64`) and
    /// actor answer (virtual slot `0x100`).
    fn reference(e: &mut Engine, base: u32, node_3d: u32, actor: bool) -> u32 {
        let r = object(e);
        e.mem.set_u32(r + 0x20, base);
        e.mem.set_u32(r + 0x64, node_3d);
        answer(e, r, 0x100, u32::from(actor));
        answer(e, r, 0x1d0, node_3d);
        r
    }

    /// An engine with the pages the code reads globals from, the test
    /// vtable and doubles for the accessors that only read a field,
    /// behaving as the game's do.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_6000,
            0x0101_2000,
            0x0101_7000,
            0x0101_e000,
            0x0102_4000,
            0x0102_e000,
            0x0118_b000,
            0x011a_b000,
            0x011c_3000,
            0x011c_7000,
            0x011c_9000,
            0x011c_a000,
            0x011d_d000,
            0x011d_e000,
            0x011f_4000,
            0x0120_2000,
        ] {
            e.map(page, 0x1000);
        }
        let slots: Vec<u32> = (0..=LAST_SLOT / 4).map(|i| fake(4 * i)).collect();
        e.put_vtable(TEST_VTABLE, &slots);
        for slot in (0..=LAST_SLOT).step_by(4) {
            e.register_double(fake(slot), move |e, a| {
                e.mem.u32(a[0] + ANSWERS + slot).into_ret()
            });
        }
        e.register(CELL_EXTRA_DATA_LIST, |_, a| (a[0] + 0x28).into_ret());
        e.register(REFERENCE_EXTRA_DATA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.register(CELL_IS_INTERIOR, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x24) & 1 != 0).into_ret()
        });
        e.register(CELL_GET_WORLD_SPACE, |e, a| {
            if e.mem.u8(a[0] + 0x24) & 1 != 0 {
                0u32.into_ret()
            } else {
                e.mem.u32(a[0] + 0xc0).into_ret()
            }
        });
        e.register(CELL_FLAG_80, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x24) & 0x80 != 0).into_ret()
        });
        e.register(CELL_FLAG_2, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x24) & 2 != 0).into_ret()
        });
        e.register(CELL_PERSISTENT_FLAG, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) & 0x400 != 0).into_ret()
        });
        e.register(CELL_STATE, |e, a| (e.mem.u8(a[0] + 0x26) as u32).into_ret());
        e.register(CELL_STATE_IS_6, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x26) == 6).into_ret()
        });
        e.register(CELL_STATE_IS_5, |e, a| {
            u32::from(e.mem.u8(a[0] + 0x26) == 5).into_ret()
        });
        e.register(CELL_REFERENCE_LIST, |_, a| (a[0] + 0xac).into_ret());
        e.register(REFERENCE_GET_BASE_FORM, |e, a| {
            e.mem.u32(a[0] + 0x20).into_ret()
        });
        e.register(FORM_TYPE, |e, a| (e.mem.u8(a[0] + 4) as u32).into_ret());
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        flag_test(&mut e, FORM_FLAG_800, 0x800);
        flag_test(&mut e, FORM_FLAG_20, 0x20);
        flag_test(&mut e, FORM_FLAG_820, 0x820);
        flag_test(&mut e, FORM_FLAG_4000, 0x4000);
        flag_test(&mut e, FORM_FLAG_20000000, 0x2000_0000);
        flag_test(&mut e, FORM_FLAG_800000, 0x80_0000);
        flag_test(&mut e, FORM_FLAG_8, 0x8);
        e.register(NODE_PARENT, |e, a| e.mem.u32(a[0] + 0x18).into_ret());
        e.register(REFERENCE_PARENT_CELL, |e, a| {
            e.mem.u32(a[0] + 0x40).into_ret()
        });
        e.register(REFERENCE_GET_3D, |e, a| e.mem.u32(a[0] + 0x64).into_ret());
        e.register(SLOT_GET, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(SLOT_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(SLOT_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(SLOT_COPY_CONSTRUCT, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            a[0].into_ret()
        });
        quiet(&mut e, &[SLOT_RELEASE, CELL_LOCK_ENTER, CELL_LOCK_LEAVE]);
        // The list of pointers: node `{item, next}`; `PushFront` copies the
        // head into a new node and puts the item in the head.
        e.register(LIST_IS_END, |e, a| {
            u32::from(e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0).into_ret()
        });
        e.register(LIST_ITEM_ADDRESS, |_, a| a[0].into_ret());
        e.register(LIST_NEXT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(LIST_PUSH_FRONT, |e, a| {
            let item = e.mem.u32(a[1]);
            if item != 0 {
                if e.mem.u32(a[0]) != 0 {
                    let node = e.mem.alloc(8);
                    let head_item = e.mem.u32(a[0]);
                    let head_next = e.mem.u32(a[0] + 4);
                    e.mem.set_u32(node, head_item);
                    e.mem.set_u32(node + 4, head_next);
                    e.mem.set_u32(a[0] + 4, node);
                }
                e.mem.set_u32(a[0], item);
            }
            Ret::default()
        });
        e.register(LIST_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            a[0].into_ret()
        });
        quiet(&mut e, &[LIST_CLEAR, LIST_DESTRUCT]);
        e.register(RT_DYNAMIC_CAST, |_, a| a[0].into_ret());
        e.register(ALLOCATE, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(DEALLOCATE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(NODE_ALLOCATE, |e, a| e.mem.alloc(a[0]).into_ret());
        e
    }

    /// The addresses of the doubles an `Rc` log collects: wraps `address`
    /// so that every call is recorded in the returned vector.
    fn record(e: &mut Engine, address: u32, result: u32) -> Rc<RefCell<Vec<Vec<u32>>>> {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let seen = calls.clone();
        e.register_double(address, move |_, a| {
            seen.borrow_mut().push(a.to_vec());
            result.into_ret()
        });
        calls
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    fn take_log(e: &mut Engine) -> Log {
        e.call_log.take().unwrap()
    }

    #[test]
    fn cell_form_reads_the_extra_entry() {
        let mut e = engine();
        let c = cell(&mut e);
        e.register(EXTRA_LIST_GET_CELL_FORM, |e, a| e.mem.u32(a[0]).into_ret());
        e.mem.set_u32(c.addr() + 0x28, 0x1234);
        assert_eq!(e.call(0x0054_7650, &args![c]).u32(), 0x1234);
    }

    #[test]
    fn cell_form_falls_back_on_the_world_then_on_default_forms() {
        let mut e = engine();
        e.register(EXTRA_LIST_GET_CELL_FORM, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(WORLD_SPACE_GET_CELL_FORM, |e, a| {
            e.mem.u32(a[0] + 0x44).into_ret()
        });
        // The look-up gives `id + 0xA000`; the cast adds 1.
        e.register(FORM_LOOK_UP, |_, a| (a[0] + 0xa000).into_ret());
        e.register(RT_DYNAMIC_CAST, |_, a| (a[0] + 1).into_ret());

        // Own form.
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0x28, 0x77);
        start_log(&mut e);
        assert_eq!(e.call(0x0054_7680, &args![c]).u32(), 0x77);
        assert!(calls_to(&take_log(&mut e), RT_DYNAMIC_CAST).is_empty());

        // Exterior cell: the world space's form.
        let c = cell(&mut e);
        let world = object(&mut e);
        e.mem.set_u32(world + 0x44, 0x88);
        e.mem.set_u32(c.addr() + 0xc0, world);
        assert_eq!(e.call(0x0054_7680, &args![c]).u32(), 0x88);

        // Exterior cell with nothing: default form 0x161.
        let c = cell(&mut e);
        let world = object(&mut e);
        e.mem.set_u32(c.addr() + 0xc0, world);
        start_log(&mut e);
        assert_eq!(e.call(0x0054_7680, &args![c]).u32(), 0x161 + 0xa000 + 1);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, RT_DYNAMIC_CAST),
            vec![vec![
                0x161 + 0xa000,
                0,
                RTTI_TES_FORM,
                RTTI_CELL_FORM_TARGET,
                0
            ]]
        );

        // Interior cell without flag 0x80: default form 0x160; with the flag
        // 0x161.
        let c = cell(&mut e);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(e.call(0x0054_7680, &args![c]).u32(), 0x160 + 0xa000 + 1);
        e.mem.set_u8(c.addr() + 0x24, 0x81);
        assert_eq!(e.call(0x0054_7680, &args![c]).u32(), 0x161 + 0xa000 + 1);
    }

    #[test]
    fn set_cell_form_hands_the_value_to_the_extra_list() {
        let mut e = engine();
        let c = cell(&mut e);
        let calls = record(&mut e, EXTRA_LIST_SET_CELL_FORM, 0);
        e.call(0x0054_7750, &args![c, 0x55u32]);
        assert_eq!(*calls.borrow(), vec![vec![c.addr() + 0x28, 0x55]]);
    }

    #[test]
    fn water_type_prefers_the_cell_then_world_then_default() {
        let mut e = engine();
        e.register(EXTRA_LIST_GET_WATER_TYPE, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(WORLD_SPACE_GET_WATER_TYPE, |e, a| {
            e.mem.u32(a[0] + 0x74).into_ret()
        });
        e.register(WATER_REPLACEMENT, |e, a| e.mem.u32(a[0] + 0x80).into_ret());
        e.set_global(DEFAULT_WATER_TYPE, 0x5000u32);

        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0x28, 0x1000);
        assert_eq!(e.call(0x0054_7770, &args![c]).u32(), 0x1000);

        let c = cell(&mut e);
        let world = object(&mut e);
        e.mem.set_u32(world + 0x74, 0x2000);
        e.mem.set_u32(c.addr() + 0xc0, world);
        assert_eq!(e.call(0x0054_7770, &args![c]).u32(), 0x2000);

        let c = cell(&mut e);
        assert_eq!(e.call(0x0054_7770, &args![c]).u32(), 0x5000);

        // A cell with form flag 8 takes the replacement the water names.
        let water = object(&mut e);
        e.mem.set_u32(water + 0x80, 0x6000);
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0x28, water);
        e.mem.set_u32(c.addr() + 8, 8);
        assert_eq!(e.call(0x0054_7770, &args![c]).u32(), 0x6000);
        // ... unless the water names none.
        let water = object(&mut e);
        e.mem.set_u32(c.addr() + 0x28, water);
        assert_eq!(e.call(0x0054_7770, &args![c]).u32(), water);
    }

    #[test]
    fn canopy_mask_read_skips_interior_cells() {
        let mut e = engine();
        let calls = record(&mut e, EXTRA_LIST_GET_CANOPY_MASK, 0x99);
        let c = cell(&mut e);
        let (texture_out, rect_out) = (0x1111u32, 0x2222u32);
        assert_eq!(
            e.call(0x0054_77f0, &args![c, texture_out, rect_out]).u32(),
            0x99
        );
        assert_eq!(
            *calls.borrow(),
            vec![vec![c.addr() + 0x28, texture_out, rect_out]]
        );
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(
            e.call(0x0054_77f0, &args![c, texture_out, rect_out]).u32(),
            0
        );
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn world_space_name_text_is_read_at_0xd4() {
        let mut e = engine();
        let calls = record(&mut e, STRING_GET_TEXT, 0x4242);
        assert_eq!(e.call(0x0054_8210, &args![0x1000u32]).u32(), 0x4242);
        assert_eq!(*calls.borrow(), vec![vec![0x1000 + 0xd4]]);
    }

    #[test]
    fn small_flag_tests() {
        let mut e = engine();
        let c = cell(&mut e);
        assert!(!e.call(0x0054_8700, &args![c]).bool());
        e.mem.set_u32(c.addr() + 8, 0x1000_0000);
        assert!(e.call(0x0054_8700, &args![c]).bool());

        // `cCellState == 2`, read through the state accessor.
        assert!(!e.call(0x0054_8720, &args![c]).bool());
        e.mem.set_u8(c.addr() + 0x26, 2);
        assert!(e.call(0x0054_8720, &args![c]).bool());
        e.mem.set_u8(c.addr() + 0x26, 3);
        assert!(!e.call(0x0054_8720, &args![c]).bool());

        // `iFormFlags & 0x40`.
        let f = form(&mut e, 0, 0x40, 0);
        assert!(e.call(0x0054_9580, &args![f]).bool());
        e.mem.set_u32(f + 8, 0x3f);
        assert!(!e.call(0x0054_9580, &args![f]).bool());
    }

    #[test]
    fn visible_distant_counts_compare_as_signed_shorts() {
        let mut e = engine();
        let c = cell(&mut e);
        e.set(c, TESObjectCELL::sNumRefsWithVisibleDistant, 2);
        e.set(c, TESObjectCELL::sNumLoadedRefsWithVisibleDistant, 2);
        assert!(e.call(0x0054_95a0, &args![c]).bool());
        e.set(c, TESObjectCELL::sNumRefsWithVisibleDistant, 3);
        assert!(!e.call(0x0054_95a0, &args![c]).bool());
        e.set(c, TESObjectCELL::sNumRefsWithVisibleDistant, -1);
        assert!(e.call(0x0054_95a0, &args![c]).bool());
    }

    #[test]
    fn marker_node_slots_and_the_portal_shape_slot() {
        let mut e = engine();
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0xb4, 0xaaa0);
        e.mem.set_u32(c.addr() + 0xb8, 0xbbb0);
        assert_eq!(e.call(0x0054_95d0, &args![c]).u32(), 0xaaa0);
        assert_eq!(e.call(0x0054_95f0, &args![c]).u32(), 0xbbb0);

        let shape = object(&mut e);
        e.call(0x0054_9610, &args![shape, 0xccc0u32]);
        assert_eq!(e.mem.u32(shape + 0x104), 0xccc0);
    }

    #[test]
    fn form_kind_flags_all_clear() {
        let mut e = engine();
        for (address, mask) in [
            (FLAG_A8_8, 8u32),
            (FLAG_A8_40, 0x40),
            (FLAG_A8_80, 0x80),
            (FLAG_A8_100, 0x100),
        ] {
            e.register_double(address, move |e, a| {
                u32::from(e.mem.u32(a[0] + 0xa8) & mask != 0).into_ret()
            });
        }
        let f = object(&mut e);
        assert!(e.call(0x0054_9630, &args![f]).bool());
        for bit in [8u32, 0x40, 0x80, 0x100] {
            e.mem.set_u32(f + 0xa8, bit);
            assert!(!e.call(0x0054_9630, &args![f]).bool(), "bit {bit:#x}");
        }
        // The tests stop at the first set one.
        e.mem.set_u32(f + 0xa8, 0x8 | 0x100);
        start_log(&mut e);
        assert!(!e.call(0x0054_9630, &args![f]).bool());
        assert_eq!(sequence(&take_log(&mut e)), vec![FLAG_A8_8]);
    }

    #[test]
    fn mask_test_is_nonzero() {
        let mut e = engine();
        e.register(MASK_TEST, |e, a| (e.mem.u32(a[0] + 0xc) & a[1]).into_ret());
        let f = object(&mut e);
        assert!(!e.call(0x0054_9690, &args![f]).bool());
        e.mem.set_u32(f + 0xc, 3);
        assert!(e.call(0x0054_9690, &args![f]).bool());
    }

    #[test]
    fn child_node_accessors() {
        let mut e = engine();
        let c = cell(&mut e);
        e.register(CELL_CHILD_NODE, |_, a| (0x9000 + a[1]).into_ret());
        assert_eq!(e.call(0x0054_97a0, &args![c]).u32(), 0x9004);
        assert_eq!(e.call(0x0054_a050, &args![c]).u32(), 0x9000);
        assert_eq!(e.call(0x0054_aa60, &args![c]).u32(), 0x9003);
    }

    /// What the mask doubles hand out: per extra data list address, the
    /// texture and the rect record of a cell that already has a mask.
    type Masks = Rc<RefCell<HashMap<u32, (u32, u32)>>>;

    /// The argument words of the calls a recording double saw.
    type Calls = Rc<RefCell<Vec<Vec<u32>>>>;

    /// Sets up a mask for the cell: a texture (`+0x98` answers `rows`), a
    /// 4-byte-pitch rect record whose bits are `rows * 4` bytes filled with
    /// `fill`. Returns `(texture, rect, bits)`.
    fn give_mask(
        e: &mut Engine,
        masks: &Masks,
        c: Ptr<TESObjectCELL>,
        rows: u32,
        fill: u8,
    ) -> (u32, u32, u32) {
        let texture = object(e);
        answer(e, texture, 0x98, rows);
        let rect = e.mem.alloc(8);
        let bits = e.mem.alloc(rows * 64 + 16);
        for i in 0..rows * 64 + 16 {
            e.mem.set_u8(bits + i, fill);
        }
        e.mem.set_u32(rect, 64);
        e.mem.set_u32(rect + 4, bits);
        masks.borrow_mut().insert(c.addr() + 0x28, (texture, rect));
        (texture, rect, bits)
    }

    /// An engine with the doubles of the canopy shadow mask code: the mask
    /// table, the renderer, the texture creation and the surface lock.
    fn canopy_engine() -> (Engine, Masks, Calls, Calls) {
        let mut e = engine();
        let masks: Masks = Rc::new(RefCell::new(HashMap::new()));
        let table = masks.clone();
        e.register_double(EXTRA_LIST_GET_CANOPY_MASK, move |e, a| {
            match table.borrow().get(&a[0]) {
                Some((texture, rect)) => {
                    e.mem.set_u32(a[1], *texture);
                    e.mem.set_u32(a[2], *rect);
                    1u32.into_ret()
                }
                None => {
                    e.mem.set_u32(a[1], 0);
                    0u32.into_ret()
                }
            }
        });
        returns(&mut e, RENDERER_CAN_CREATE_MASK, 1);
        returns(&mut e, RENDERER_GLOBAL, 0x5555);
        e.register(FIXED_STRING_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        quiet(&mut e, &[FIXED_STRING_DESTRUCT]);
        let creates = Rc::new(RefCell::new(Vec::new()));
        let seen = creates.clone();
        e.register_double(RENDERED_TEXTURE_CREATE, move |e, a| {
            // The name is read now: the temporary is freed afterwards.
            let mut call = a.to_vec();
            call.push(e.mem.u32(a[0]));
            seen.borrow_mut().push(call);
            let texture = object(e);
            answer(e, texture, 0x98, 2);
            texture.into_ret()
        });
        let table = masks.clone();
        e.register_double(EXTRA_LIST_SET_CANOPY_MASK, move |e, a| {
            // (list, 1, texture, rect_out): the entry owns a rect record.
            let rect = e.mem.alloc(8);
            let bits = e.mem.alloc(512);
            for i in 0..512 {
                e.mem.set_u8(bits + i, 0xaa);
            }
            e.mem.set_u32(rect, 4);
            e.mem.set_u32(rect + 4, bits);
            e.mem.set_u32(a[3], rect);
            table.borrow_mut().insert(a[0], (a[2], rect));
            Ret::default()
        });
        // The texture's Direct3D object: its slot 0x9c gives a surface whose
        // slot 0x4c is `LockRect`.
        let locks = record(&mut e, fake(SURFACE_SLOT_LOCK_RECT), 0);
        e.register(OBJECT_FIELD_24, |e, a| {
            let owner = object(e);
            let surface = object(e);
            answer(e, owner, SURFACE_OWNER_SLOT_SURFACE, surface);
            let _ = a;
            owner.into_ret()
        });
        (e, masks, locks, creates)
    }

    #[test]
    fn create_canopy_mask_builds_registers_locks_and_clears_the_texture() {
        let (mut e, masks, locks, creates) = canopy_engine();
        let c = cell(&mut e);
        let texture_out = e.mem.alloc(4);
        let rect_out = e.mem.alloc(4);
        e.mem.set_u32(texture_out, 0xdead);
        start_log(&mut e);
        assert!(e.call(0x0054_7830, &args![c, texture_out, rect_out]).bool());
        let log = take_log(&mut e);
        let texture = e.mem.u32(texture_out);
        assert_ne!(texture, 0);
        // The new texture: 0x40 x 0x40 with the renderer and the format
        // record, named "Canopy Shadow Mask" through a temporary.
        let create = creates.borrow()[0].clone();
        assert_eq!(create.len(), 9);
        assert_eq!(&create[1..8], &[0x40, 0x40, 0x5555, MASK_FORMAT, 0, 0, 0]);
        // (the ninth word is the name the temporary held at the call)
        assert_eq!(create[8], CANOPY_MASK_NAME);
        // Registered in the cell's extra data, which hands out the rect.
        assert_eq!(
            calls_to(&log, EXTRA_LIST_SET_CANOPY_MASK),
            vec![vec![c.addr() + 0x28, 1, texture, rect_out]]
        );
        let (_, rect) = masks.borrow()[&(c.addr() + 0x28)];
        assert_eq!(e.mem.u32(rect_out), rect);
        // Locked once with `(surface, 0, rect, 0, 0)`.
        assert_eq!(locks.borrow().len(), 1);
        assert_eq!(&locks.borrow()[0][1..], &[0, rect, 0, 0]);
        // pitch 4 x 2 rows = 8 bytes cleared, the rest untouched.
        let bits = e.mem.u32(rect + 4);
        for i in 0..8 {
            assert_eq!(e.mem.u8(bits + i), 0, "byte {i}");
        }
        assert_eq!(e.mem.u8(bits + 8), 0xaa);
        // The format record and the busy flags.
        assert_eq!(e.global::<u32>(MASK_FORMAT_WORD), 0x32);
        assert_eq!(e.global::<u8>(MASK_FORMAT_FLAG), 1);
        assert_eq!(e.global::<u8>(RENDER_TARGET_BUSY_FLAG), 0);
    }

    #[test]
    fn create_canopy_mask_refuses_interior_cells_and_unready_renderers() {
        let (mut e, _, _, _) = canopy_engine();
        let texture_out = e.mem.alloc(4);
        let rect_out = e.mem.alloc(4);
        e.mem.set_u32(texture_out, 0xdead);
        // No cell.
        assert!(!e
            .call(0x0054_7830, &args![0u32, texture_out, rect_out])
            .bool());
        assert_eq!(e.mem.u32(texture_out), 0);
        // Interior cell.
        let c = cell(&mut e);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert!(!e.call(0x0054_7830, &args![c, texture_out, rect_out]).bool());
        // The renderer cannot make the texture.
        let c = cell(&mut e);
        returns(&mut e, RENDERER_CAN_CREATE_MASK, 0);
        start_log(&mut e);
        assert!(!e.call(0x0054_7830, &args![c, texture_out, rect_out]).bool());
        assert!(calls_to(&take_log(&mut e), RENDERED_TEXTURE_CREATE).is_empty());
    }

    /// A neighbour table for the data handler's cell search.
    fn neighbours(e: &mut Engine) -> Rc<RefCell<HashMap<(i32, i32), u32>>> {
        let table = Rc::new(RefCell::new(HashMap::new()));
        let seen = table.clone();
        e.register_double(DATA_HANDLER_FIND_CELL, move |_, a| {
            seen.borrow()
                .get(&(a[1] as i32, a[2] as i32))
                .copied()
                .unwrap_or(0)
                .into_ret()
        });
        table
    }

    #[test]
    fn paint_pixel_writes_the_value_or_saturates() {
        let (mut e, masks, _, _) = canopy_engine();
        let neighbour_table = neighbours(&mut e);
        let c = cell(&mut e);
        let n = cell(&mut e);
        neighbour_table.borrow_mut().insert((5, 6), n.addr());
        let (_, rect, bits) = give_mask(&mut e, &masks, n, 2, 0);
        let slot = e.mem.alloc(4);
        // Slot unset, no cell at (9, 9): nothing happens.
        e.call(0x0054_79c0, &args![c, 9i32, 9i32, slot, 3u32, 7u32]);
        assert_eq!(e.mem.u32(slot), 0);
        // The neighbour at (5, 6) has a mask: the byte is written and the
        // slot keeps the rect for the next pixels.
        e.call(0x0054_79c0, &args![c, 5i32, 6i32, slot, 3u32, 7u32]);
        assert_eq!(e.mem.u32(slot), rect);
        assert_eq!(e.mem.u8(bits + 3), 7);
        // A byte that is already set becomes 0xFF; the slot is reused, so
        // no look-up happens (the table could not answer (0, 0)).
        e.call(0x0054_79c0, &args![c, 0i32, 0i32, slot, 3u32, 9u32]);
        assert_eq!(e.mem.u8(bits + 3), 0xff);
        e.call(0x0054_79c0, &args![c, 0i32, 0i32, slot, 4u32, 9u32]);
        assert_eq!(e.mem.u8(bits + 4), 9);
    }

    #[test]
    fn paint_pixel_creates_the_neighbours_mask_when_it_has_none() {
        let (mut e, masks, _, _) = canopy_engine();
        let neighbour_table = neighbours(&mut e);
        let c = cell(&mut e);
        let n = cell(&mut e);
        neighbour_table.borrow_mut().insert((1, 2), n.addr());
        let slot = e.mem.alloc(4);
        e.call(0x0054_79c0, &args![c, 1i32, 2i32, slot, 5u32, 0x33u32]);
        let (_, rect) = masks.borrow()[&(n.addr() + 0x28)];
        assert_eq!(e.mem.u32(slot), rect);
        let bits = e.mem.u32(rect + 4);
        // The creation cleared 4 x 2 bytes; the pixel is within them? No:
        // index 5 is inside the cleared 8 bytes, so it holds the value.
        assert_eq!(e.mem.u8(bits + 5), 0x33);
        assert_eq!(e.mem.u8(bits + 9), 0xaa);
    }

    #[test]
    fn paint_pixel_locks_a_rect_without_bits() {
        let (mut e, masks, locks, _) = canopy_engine();
        let c = cell(&mut e);
        let (_, rect, bits) = give_mask(&mut e, &masks, c, 2, 0);
        // The slot is already set but the rect has no bits yet: the surface
        // is locked (through the local texture pointer, which is null).
        e.mem.set_u32(rect + 4, 0);
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, rect);
        let lock_calls = record(&mut e, OBJECT_FIELD_24, 0);
        let surface = object(&mut e);
        let owner = object(&mut e);
        answer(&mut e, owner, SURFACE_OWNER_SLOT_SURFACE, surface);
        e.register_double(OBJECT_FIELD_24, move |_, _| owner.into_ret());
        let _ = (lock_calls, bits);
        // The lock double sets the bits as the real lock would.
        let new_bits = e.mem.alloc(16);
        e.register_double(fake(SURFACE_SLOT_LOCK_RECT), move |e, a| {
            e.mem.set_u32(a[2] + 4, new_bits);
            Ret::default()
        });
        e.call(0x0054_79c0, &args![c, 0i32, 0i32, slot, 2u32, 0x44u32]);
        assert_eq!(e.mem.u8(new_bits + 2), 0x44);
        assert_eq!(locks.borrow().len(), 0);
    }

    /// The doubles of `fn_00547ad0` on top of the canopy engine: a land that
    /// reports mask coordinates `(32, 32)` (as `2048 / 64`), a texture whose
    /// stride is 64, square root, truncation and a random number of 10.
    fn painting_engine(coordinate: f32) -> (Engine, Masks, Ptr<TESObjectCELL>, u32) {
        let (mut e, masks, _, _) = canopy_engine();
        returns(&mut e, TREE_UPDATE_FLAG, 0);
        quiet(&mut e, &[TREE_MANAGER_REFRESH]);
        let c = cell(&mut e);
        let world = object(&mut e);
        e.mem.set_u32(c.addr() + 0xc0, world);
        e.mem.set_u32(c.addr() + 0x4c, 0x7777); // the land
        e.mem.set_u32(c.addr() + 0x1f0, 100); // grid x
        e.mem.set_u32(c.addr() + 0x1f4, 200); // grid y
        let name = e.mem.alloc(8);
        e.mem.set_u8(name, b'W');
        e.register_double(STRING_GET_TEXT, move |_, _| name.into_ret());
        e.register(CELL_GET_LAND, |e, a| e.mem.u32(a[0] + 0x4c).into_ret());
        e.register(CELL_GET_DATA_X, |e, a| e.mem.u32(a[0] + 0x1f0).into_ret());
        e.register(CELL_GET_DATA_Y, |e, a| e.mem.u32(a[0] + 0x1f4).into_ret());
        quiet(
            &mut e,
            &[COORDINATE_DATA_CONSTRUCT, RENDERER_LOCK, RENDERER_UNLOCK],
        );
        e.register_double(LAND_GET_COORD_DATA, move |e, a| {
            e.mem.set_f32(a[1], coordinate * 64.0);
            e.mem.set_f32(a[1] + 4, 32.0 * 64.0);
            1u32.into_ret()
        });
        e.register(FTOL, |_, a| {
            let value = f64::take(a, &mut 0);
            (value as i32).into_ret()
        });
        e.register(SQUARE_ROOT, |_, a| {
            let value = f32::from_bits(a[0]);
            Ret {
                st0: (value as f64).sqrt(),
                ..Ret::default()
            }
        });
        e.register(RANDOM_FLOAT, |_, _| Ret {
            st0: 10.0,
            ..Ret::default()
        });
        e.set_global(CANOPY_RANDOM_RANGE, 55.0f32);
        e.set_global(MASK_PIXEL_DIVISOR, 64.0f64);
        e.set_global(SHADE_BASE, 255.0f64);
        e.set_global(SHADE_FALLOFF, 60.0f64);
        let (_, _, bits) = give_mask(&mut e, &masks, c, 64, 0);
        (e, masks, c, bits)
    }

    #[test]
    fn canopy_shadow_paints_the_pixels_within_the_radius() {
        let (mut e, _, c, bits) = painting_engine(32.0);
        start_log(&mut e);
        // Radius 64 becomes 1 mask pixel.
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 64u32, 0u32]);
        let log = take_log(&mut e);
        // The three pixels within distance 1 of (32, 32): the centre gets
        // 255 - 10 = 245, the two at distance 1 get 255 - 10 - 60 = 185.
        let at = |row: u32, column: u32| e.mem.u8(bits + row * 64 + column);
        assert_eq!(at(32, 32), 245);
        assert_eq!(at(31, 32), 185);
        assert_eq!(at(32, 31), 185);
        // The diagonal one is at 1.41 and stays unpainted.
        assert_eq!(at(31, 31), 0);
        // The renderer was locked around the painting.
        assert_eq!(calls_to(&log, RENDERER_LOCK).len(), 1);
        assert_eq!(calls_to(&log, RENDERER_UNLOCK).len(), 1);
        // The land was asked for the coordinates of the position.
        let coord = &calls_to(&log, LAND_GET_COORD_DATA)[0];
        assert_eq!(coord[0], 0x7777);
        assert_eq!(coord[3], 0);
    }

    #[test]
    fn canopy_shadow_saturates_set_pixels_and_defaults_the_radius() {
        let (mut e, _, c, bits) = painting_engine(32.0);
        e.mem.set_u8(bits + 32 * 64 + 32, 9);
        // Radius 0 becomes 300 (4 pixels after the shift): a wide disc.
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 0u32, 0u32]);
        assert_eq!(e.mem.u8(bits + 32 * 64 + 32), 0xff);
        // 255 - 10 - 60 * 3 / 4 = 200 three pixels from the centre.
        assert_eq!(e.mem.u8(bits + 32 * 64 + 35), 200);
        // Beyond four pixels nothing is painted.
        assert_eq!(e.mem.u8(bits + 32 * 64 + 37), 0);
        // More than 0x400 also becomes 300.
        let (mut e, _, c, bits) = painting_engine(32.0);
        e.call(
            0x0054_7ad0,
            &args![c, 0.0f32, 0.0f32, 0.0f32, 0x401u32, 0u32],
        );
        assert_eq!(e.mem.u8(bits + 32 * 64 + 35), 200);
    }

    #[test]
    fn canopy_shadow_hands_pixels_beyond_the_edge_to_the_neighbour() {
        let (mut e, masks, c, bits) = painting_engine(63.0);
        let neighbour_table = neighbours(&mut e);
        let east = cell(&mut e);
        neighbour_table.borrow_mut().insert((101, 200), east.addr());
        let (_, _, east_bits) = give_mask(&mut e, &masks, east, 64, 0);
        // Centre (63, 32), radius 128 >> 6 = 2: the column 64 pixel at
        // distance 1 belongs to the cell to the east: row 32, column 0 there.
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 128u32, 0u32]);
        // 255 - 10 - 60 * 1 / 2 = 215.
        assert_eq!(e.mem.u8(east_bits + 32 * 64), 215);
        // In this cell the column 63 pixel is the centre line.
        assert_eq!(e.mem.u8(bits + 32 * 64 + 63), 245);
    }

    #[test]
    fn canopy_shadow_is_skipped_for_interiors_missing_names_and_land() {
        // Interior cell.
        let (mut e, _, c, bits) = painting_engine(32.0);
        e.mem.set_u8(c.addr() + 0x24, 1);
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 64u32, 0u32]);
        assert_eq!(e.mem.u8(bits + 32 * 64 + 32), 0);
        // The disable flag.
        let (mut e, _, c, bits) = painting_engine(32.0);
        e.set_global(CANOPY_DISABLED_FLAG, 1u8);
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 64u32, 0u32]);
        assert_eq!(e.mem.u8(bits + 32 * 64 + 32), 0);
        // An empty world space name.
        let (mut e, _, c, bits) = painting_engine(32.0);
        let empty = e.mem.alloc(8);
        e.register_double(STRING_GET_TEXT, move |_, _| empty.into_ret());
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 64u32, 0u32]);
        assert_eq!(e.mem.u8(bits + 32 * 64 + 32), 0);
        // No land.
        let (mut e, _, c, bits) = painting_engine(32.0);
        e.mem.set_u32(c.addr() + 0x4c, 0);
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 64u32, 0u32]);
        assert_eq!(e.mem.u8(bits + 32 * 64 + 32), 0);
        // The tree update flag calls the tree manager first.
        let (mut e, _, c, _) = painting_engine(32.0);
        returns(&mut e, TREE_UPDATE_FLAG, 1);
        let refresh = record(&mut e, TREE_MANAGER_REFRESH, 0);
        e.call(0x0054_7ad0, &args![c, 0.0f32, 0.0f32, 0.0f32, 64u32, 0u32]);
        assert_eq!(refresh.borrow().len(), 1);
    }

    /// Everything the reference and scene graph code reaches: the global
    /// objects, the cell's child nodes and word-driven doubles for the
    /// reference tests (`+0x180` visible distant, `+0x184` scripted, `+0x188`
    /// activating children, `+0x18C` fades out, `+0x190` world space, `+0x194`
    /// external emittance).
    struct World {
        e: Engine,
        cell: Ptr<TESObjectCELL>,
        tes: u32,
        player: u32,
        handler: u32,
        save: u32,
        nodes: Vec<u32>,
    }

    fn world() -> World {
        let mut e = engine();
        let tes = object(&mut e);
        let player = object(&mut e);
        let handler = object(&mut e);
        let save = object(&mut e);
        e.set_global(TES_POINTER, tes);
        e.set_global(PLAYER_POINTER, player);
        e.set_global(DATA_HANDLER_POINTER, handler);
        e.set_global(SAVE_GAME_POINTER, save);
        e.set_global(GAME_LOADER_POINTER, tes);
        let nodes: Vec<u32> = (0..10).map(|_| object(&mut e)).collect();
        let table = nodes.clone();
        e.register_double(CELL_CHILD_NODE, move |_, a| table[a[1] as usize].into_ret());
        quiet(
            &mut e,
            &[
                EXTRA_LIST_SET_PERSISTENT_CELL,
                WORLD_SPACE_ADD_TO_PERSISTENT_REF_DATA,
                CELL_REMOVE_REFERENCE,
                STATE_6_ADD_REFERENCE,
                CELL_ADD_SCRIPTED_REF,
                CELL_ADD_ACTIVATING_REF,
                MODEL_LOADER_QUEUE_REFERENCE,
                PROCESS_LISTS_ADD_ACTOR_TO_TEMP_CHANGE_LIST,
                QUEUED_ATTACH_MAP_REMOVE_AT,
                REFERENCE_ADD_LIGHT_TO_SCENE,
                TASK_QUEUE_SET_3D_NULL,
                TERRAIN_MANAGER_HIDE_TREE,
                TERRAIN_MANAGER_SET_FLAG_28,
                INCREMENT_FIELD_B8,
                SET_BYTE_8,
                DEBUG_PRINT,
                CELL_ADD_EMITTANCE_REF,
                CELL_ADD_WATER_REF,
                CELL_ADD_MULTI_BOUND_REF,
                WATER_SYSTEM_ADD_PLACEABLE_WATER,
                NODE_UPDATE_PROPERTIES,
                FADE_NODE_PREPARE,
                FADE_NODE_SET_KIND,
                FADE_NODE_SET_RANGE,
                FADE_NODE_SET_TREE_FLAG,
                FADE_NODE_CHECK_FADE_RADIUS,
                SET_PROPERTY_FADE_ALPHA,
                SHADOW_SCENE_NODE_UPDATE_LIGHTING,
                CELL_START_FADE_IN,
            ],
        );
        for (address, offset) in [
            (REFERENCE_HAS_VISIBLE_DISTANT, 0x180u32),
            (REFERENCE_IS_SCRIPTED, 0x184),
            (REFERENCE_IS_ACTIVATING_CHILDREN, 0x188),
            (REFERENCE_FADES_OUT, 0x18c),
            (REFERENCE_GET_WORLD_SPACE, 0x190),
            (REFERENCE_USES_EXTERNAL_EMITTANCE, 0x194),
            (TES_IS_CELL_LOADED, 0x180),
            (PLAYER_IS_SLEEPING_OR_RESTING, 0x180),
            (SAVE_GAME_FLAG, 0x180),
        ] {
            e.register_double(address, move |e, a| e.mem.u32(a[0] + offset).into_ret());
        }
        e.register(DATA_HANDLER_LOADING_FLAG, |e, a| {
            (e.mem.u8(a[0] + 0x61a) as u32).into_ret()
        });
        returns(&mut e, ATTACHES_ARE_QUEUED, 0);
        returns(&mut e, TES_GET_CELL_PRIORITY, 7);
        returns(&mut e, TASK_QUEUE_GETTER, 0x6000);
        returns(&mut e, WORLD_SPACE_GET_TERRAIN_MANAGER, 0x7000);
        returns(&mut e, GLOBAL_VIRTUAL_TEST, 0);
        returns(&mut e, EXTRA_LIST_GET_PRIMITIVE, 0);
        returns(&mut e, SETTING_FIRST_CHAR, 0);
        returns(&mut e, LOADING_MENU_VISIBLE, 0);
        returns(&mut e, GAME_LOADER_FLAG_244_2, 0);
        returns(&mut e, REFERENCE_GET_REF_PERSISTS, 0);
        returns(&mut e, REFERENCE_IS_AN_OWNER, 0);
        returns(&mut e, GLOBAL_SLOT_GET, 0x5000);
        returns(&mut e, SLOT_AT_AC_GET, 0x5100);
        returns(&mut e, SHADOW_SCENE_NODE_GETTER, 0x5200);
        e.register(LIST_CONTAINS, |e, a| {
            let mut node = a[0];
            let item = e.mem.u32(a[1]);
            while node != 0 {
                if e.mem.u32(node) == item {
                    return 1u32.into_ret();
                }
                node = e.mem.u32(node + 4);
            }
            0u32.into_ret()
        });
        let c = cell(&mut e);
        World {
            e,
            cell: c,
            tes,
            player,
            handler,
            save,
            nodes,
        }
    }

    /// A reference with a base form of the given type, a 3D (an object whose
    /// slot `0x10` answers `fade`), and `cell` as parent cell.
    fn placed_reference(
        w: &mut World,
        form_type: u8,
        actor: bool,
        with_3d: bool,
    ) -> (u32, u32, u32) {
        let base = form(&mut w.e, form_type, 0, 0xb000 + form_type as u32);
        let fade = object(&mut w.e);
        let node_3d = object(&mut w.e);
        answer(&mut w.e, node_3d, 0x10, fade);
        let r = reference(&mut w.e, base, if with_3d { node_3d } else { 0 }, actor);
        w.e.mem.set_u32(r + 0x40, w.cell.addr());
        (r, base, node_3d)
    }

    #[test]
    fn add_reference_ignores_null_and_formless_references() {
        let mut w = world();
        start_log(&mut w.e);
        w.e.call(0x0054_8230, &args![w.cell, 0u32, 0u32]);
        let r = reference(&mut w.e, 0, 0, false);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        assert_eq!(calls_to(&log, REFERENCE_GET_BASE_FORM).len(), 1);
        assert!(calls_to(&log, CELL_LOCK_ENTER).is_empty());
        assert!(calls_to(&log, CELL_PERSISTENT_FLAG).is_empty());
    }

    #[test]
    fn add_reference_to_a_persistent_cell_only_records_it() {
        let mut w = world();
        let world_space = object(&mut w.e);
        w.e.mem.set_u32(w.cell.addr() + 0xc0, world_space);
        w.e.mem.set_u32(w.cell.addr() + 8, 0x400);
        let (r, _, _) = placed_reference(&mut w, 0x15, false, true);
        let vcalls = record(&mut w.e, fake(0xc8), 0);
        start_log(&mut w.e);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        // The reference is pushed on the cell's list, remembers the cell and
        // goes to the world space's persistent data; the cell is told.
        assert_eq!(w.e.mem.u32(w.cell.addr() + 0xac), r);
        assert_eq!(
            calls_to(&log, EXTRA_LIST_SET_PERSISTENT_CELL),
            vec![vec![r + 0x44, w.cell.addr()]]
        );
        assert_eq!(
            calls_to(&log, WORLD_SPACE_ADD_TO_PERSISTENT_REF_DATA),
            vec![vec![world_space, r]]
        );
        assert_eq!(*vcalls.borrow(), vec![vec![w.cell.addr(), 1]]);
        // None of the attach machinery ran.
        assert!(calls_to(&log, CELL_REMOVE_REFERENCE).is_empty());
        assert!(calls_to(&log, REFERENCE_ADD_LIGHT_TO_SCENE).is_empty());
        // With the data handler's loading flag set the cell is not told.
        w.e.mem.set_u8(w.handler + 0x61a, 1);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!(vcalls.borrow().len(), 1);
    }

    #[test]
    fn add_reference_counts_visible_distant_references() {
        let mut w = world();
        let (r, _, _) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x180, 1);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!(
            w.e.get(w.cell, TESObjectCELL::sNumRefsWithVisibleDistant),
            1
        );
        // A form with flag 0x800 does not count.
        let (r, base, _) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x180, 1);
        w.e.mem.set_u32(r + 8, 0x800);
        let _ = base;
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!(
            w.e.get(w.cell, TESObjectCELL::sNumRefsWithVisibleDistant),
            1
        );
    }

    #[test]
    fn add_reference_moves_the_reference_and_registers_it() {
        let mut w = world();
        let previous = cell(&mut w.e);
        let (r, _, _) = placed_reference(&mut w, 0x15, true, true);
        w.e.mem.set_u32(r + 0x40, previous.addr());
        w.e.mem.set_u32(r + 0x184, 1); // scripted
        w.e.mem.set_u32(r + 0x188, 1); // activating children
        w.e.set(w.cell, TESObjectCELL::pLoadedData, Ptr::new(0x3000));
        w.e.mem.set_u8(w.cell.addr() + 0x26, 6); // cell state 6
        let object_6 = object(&mut w.e);
        w.e.set_global(STATE_6_OBJECT_POINTER, object_6);
        // The parent cell after the virtual slot 0x228 is the new one.
        let slot_228 = record(&mut w.e, fake(0x228), 0);
        start_log(&mut w.e);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        assert_eq!(
            calls_to(&log, CELL_REMOVE_REFERENCE),
            vec![vec![previous.addr(), r]]
        );
        assert_eq!(w.e.mem.u32(w.cell.addr() + 0xac), r);
        assert_eq!(*slot_228.borrow(), vec![vec![r, w.cell.addr()]]);
        assert_eq!(
            calls_to(&log, STATE_6_ADD_REFERENCE),
            vec![vec![object_6, r]]
        );
        // An actor with a loaded cell: the scripted test is only asked for a
        // non-actor or a temporary one, but the activating test is.
        assert!(calls_to(&log, CELL_ADD_SCRIPTED_REF).is_empty());
        assert_eq!(
            calls_to(&log, CELL_ADD_ACTIVATING_REF),
            vec![vec![w.cell.addr(), r]]
        );
        // The lock brackets the push and the slot call.
        let order = sequence(&log);
        let enter = order.iter().position(|&a| a == CELL_LOCK_ENTER).unwrap();
        let leave = order.iter().position(|&a| a == CELL_LOCK_LEAVE).unwrap();
        assert!(enter < leave);
        assert_eq!(order[enter + 1], CELL_REFERENCE_LIST);
        assert_eq!(order[leave - 1], fake(0x228));
    }

    #[test]
    fn add_reference_adds_scripted_non_actors_and_unloads_foreign_3d() {
        let mut w = world();
        let (r, _, _) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x184, 1);
        w.e.set(w.cell, TESObjectCELL::pLoadedData, Ptr::new(0x3000));
        // The reference casts to nothing: its slot 0x1c4 runs.
        w.e.register(RT_DYNAMIC_CAST, |_, _| Ret::default());
        let unload = record(&mut w.e, fake(0x1c4), 0);
        start_log(&mut w.e);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        assert_eq!(
            calls_to(&log, CELL_ADD_SCRIPTED_REF),
            vec![vec![w.cell.addr(), r]]
        );
        assert_eq!(
            calls_to(&log, RT_DYNAMIC_CAST),
            vec![vec![r, 0, RTTI_REFERENCE, RTTI_REFERENCE_CAST_TARGET, 0]]
        );
        assert_eq!(unload.borrow().len(), 1);
    }

    #[test]
    fn add_reference_attaches_a_loaded_3d_in_a_loaded_cell() {
        let mut w = world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, true, false);
        // The slot answers a node, but `REFERENCE_GET_3D` (the field) is
        // empty, so the attach returns at once.
        answer(&mut w.e, r, 0x1d0, node_3d);
        w.e.mem.set_u32(w.tes + 0x180, 1); // the cell is loaded
        start_log(&mut w.e);
        w.e.call(0x0054_8230, &args![w.cell, r, 7u32]);
        let log = take_log(&mut w.e);
        assert!(calls_to(&log, MODEL_LOADER_QUEUE_REFERENCE).is_empty());
        // An actor is added to the temporary change list.
        assert_eq!(
            calls_to(&log, PROCESS_LISTS_ADD_ACTOR_TO_TEMP_CHANGE_LIST),
            vec![vec![PROCESS_LISTS, r]]
        );
        assert_eq!(
            calls_to(&log, TES_IS_CELL_LOADED),
            vec![vec![w.tes, w.cell.addr(), 0]]
        );
    }

    #[test]
    fn add_reference_queues_a_reference_without_3d_for_the_model_loader() {
        let mut w = world();
        let loader = object(&mut w.e);
        w.e.set_global(MODEL_LOADER_POINTER, loader);
        let (r, _, _) = placed_reference(&mut w, 0x15, false, false);
        w.e.mem.set_u32(w.tes + 0x180, 1);
        start_log(&mut w.e);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        // Priority 7 from the cell, then `(reference, priority, 0)`.
        assert_eq!(
            calls_to(&log, TES_GET_CELL_PRIORITY),
            vec![vec![w.tes, w.cell.addr(), 0]]
        );
        assert_eq!(
            calls_to(&log, MODEL_LOADER_QUEUE_REFERENCE),
            vec![vec![loader, r, 7, 0]]
        );
        // A non-actor is not added to the temporary change list.
        assert!(calls_to(&log, PROCESS_LISTS_ADD_ACTOR_TO_TEMP_CHANGE_LIST).is_empty());
    }

    #[test]
    fn add_reference_wakes_actors_by_cell_state_before_queueing() {
        let mut w = world();
        let (r, _, _) = placed_reference(&mut w, 0x15, true, false);
        w.e.mem.set_u32(w.tes + 0x180, 1);
        let state_6 = record(&mut w.e, fake(0x240), 0);
        let other = record(&mut w.e, fake(0x24c), 0);
        // The player is not resting, the cell state is not 2: the actor is
        // queued; the save object answers no, so slot 0x24c runs.
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!((state_6.borrow().len(), other.borrow().len()), (0, 1));
        // In state 6 (or 5) slot 0x240 runs instead.
        w.e.mem.set_u8(w.cell.addr() + 0x26, 5);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!((state_6.borrow().len(), other.borrow().len()), (1, 1));
        // The save object answering yes on the first call and no on the
        // second skips both.
        w.e.mem.set_u32(w.save + 0x180, 1);
        w.e.register_double(SAVE_GAME_FLAG, {
            let mut answers = vec![1u32, 0].into_iter();
            move |_, _| answers.next().unwrap_or(0).into_ret()
        });
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!((state_6.borrow().len(), other.borrow().len()), (1, 1));
    }

    #[test]
    fn add_reference_to_an_unloaded_cell_drops_the_queue_entry_and_the_3d() {
        let mut w = world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, true, true);
        answer(&mut w.e, r, 0x1d0, node_3d);
        let set_3d = record(&mut w.e, fake(0x1cc), 0);
        start_log(&mut w.e);
        // The cell is not loaded (the TES word is 0).
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        assert_eq!(
            calls_to(&log, QUEUED_ATTACH_MAP_REMOVE_AT),
            vec![vec![QUEUED_ATTACH_MAP, node_3d]]
        );
        // An actor with nothing blocking: slot 0x1cc(0, 0) clears its 3D.
        assert_eq!(*set_3d.borrow(), vec![vec![r, 0, 0]]);
        // While attaches are queued the task queue does it instead.
        returns(&mut w.e, ATTACHES_ARE_QUEUED, 1);
        start_log(&mut w.e);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        assert_eq!(set_3d.borrow().len(), 1);
        assert_eq!(
            calls_to(&log, TASK_QUEUE_SET_3D_NULL),
            vec![vec![0x6000, r]]
        );
        // A resting player blocks both.
        returns(&mut w.e, ATTACHES_ARE_QUEUED, 0);
        w.e.mem.set_u32(w.player + 0x180, 1);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!(set_3d.borrow().len(), 1);
    }

    #[test]
    fn add_reference_keeps_the_kind_lists() {
        let mut w = world();
        let (r, _, _) = placed_reference(&mut w, 0x15, false, false);
        // Form flags 0x10000000 (cell flag test on the reference) and
        // 0x20000000 put the reference on one list each, once.
        w.e.mem.set_u32(r + 8, 0x3000_0000);
        // The parent cell after the call is the new cell (exterior).
        w.e.register_double(fake(0x228), {
            let new_cell = w.cell.addr();
            move |e, a| {
                e.mem.set_u32(a[0] + 0x40, new_cell);
                Ret::default()
            }
        });
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        w.e.call(0x0054_8230, &args![w.cell, r, 0u32]);
        assert_eq!(w.e.mem.u32(KIND_LIST_A), r);
        assert_eq!(w.e.mem.u32(KIND_LIST_A + 4), 0);
        assert_eq!(w.e.mem.u32(KIND_LIST_B), r);
        assert_eq!(w.e.mem.u32(KIND_LIST_B + 4), 0);
        // Interior parent cells do not use the lists.
        let (r2, _, _) = placed_reference(&mut w, 0x15, false, false);
        w.e.mem.set_u32(r2 + 8, 0x3000_0000);
        w.e.mem.set_u8(w.cell.addr() + 0x24, 1);
        w.e.call(0x0054_8230, &args![w.cell, r2, 0u32]);
        assert_eq!(w.e.mem.u32(KIND_LIST_A), r);
    }

    #[test]
    fn move_reference_into_a_persistent_cell() {
        let mut w = world();
        w.e.mem.set_u32(w.cell.addr() + 8, 0x400);
        let (r, _, _) = placed_reference(&mut w, 0x15, false, true);
        let vcalls = record(&mut w.e, fake(0xc8), 0);
        start_log(&mut w.e);
        w.e.call(0x0054_8740, &args![w.cell, r]);
        let log = take_log(&mut w.e);
        assert_eq!(w.e.mem.u32(w.cell.addr() + 0xac), r);
        assert_eq!(
            calls_to(&log, EXTRA_LIST_SET_PERSISTENT_CELL),
            vec![vec![r + 0x44, w.cell.addr()]]
        );
        assert!(calls_to(&log, WORLD_SPACE_ADD_TO_PERSISTENT_REF_DATA).is_empty());
        assert_eq!(*vcalls.borrow(), vec![vec![w.cell.addr(), 1]]);
    }

    #[test]
    fn move_reference_into_an_ordinary_cell() {
        let mut w = world();
        let previous = cell(&mut w.e);
        let (r, _, _) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x40, previous.addr());
        let vcalls = record(&mut w.e, fake(0xc8), 0);
        let slot_228 = record(&mut w.e, fake(0x228), 0);
        start_log(&mut w.e);
        w.e.call(0x0054_8740, &args![w.cell, r]);
        let log = take_log(&mut w.e);
        assert_eq!(
            calls_to(&log, CELL_REMOVE_REFERENCE),
            vec![vec![previous.addr(), r]]
        );
        assert_eq!(*slot_228.borrow(), vec![vec![r, w.cell.addr()]]);
        assert_eq!(*vcalls.borrow(), vec![vec![w.cell.addr(), 1]]);
        assert_eq!(
            calls_to(&log, REFERENCE_ADD_LIGHT_TO_SCENE),
            vec![vec![r, 0]]
        );
        // A persistent reference (flag 0x4000) keeps the cell quiet.
        w.e.mem.set_u32(r + 8, 0x4000);
        w.e.call(0x0054_8740, &args![w.cell, r]);
        assert_eq!(vcalls.borrow().len(), 1);
        // So does `GetRefPersists`.
        w.e.mem.set_u32(r + 8, 0);
        returns(&mut w.e, REFERENCE_GET_REF_PERSISTS, 1);
        w.e.call(0x0054_8740, &args![w.cell, r]);
        assert_eq!(vcalls.borrow().len(), 1);
        // A reference without a base form is ignored.
        let bare = reference(&mut w.e, 0, 0, false);
        w.e.call(0x0054_8740, &args![w.cell, bare]);
        assert_eq!(slot_228.borrow().len(), 3);
    }

    /// The world with the doubles `AttachReference3D` adds: the child nodes
    /// answer slot `0x14` (so `PerformCellNodeAttach` does not search), a
    /// recorder for `AttachChild` (slot `0xdc`), and the form-flag tests of
    /// the node kinds.
    fn attach_world() -> (World, Calls) {
        let mut w = world();
        for node in w.nodes.clone() {
            answer(&mut w.e, node, 0x14, 1);
        }
        let attaches = record(&mut w.e, fake(0xdc), 0);
        for (address, mask) in [
            (FLAG_A8_8, 8u32),
            (FLAG_A8_40, 0x40),
            (FLAG_A8_80, 0x80),
            (FLAG_A8_100, 0x100),
        ] {
            w.e.register_double(address, move |e, a| {
                u32::from(e.mem.u32(a[0] + 0xa8) & mask != 0).into_ret()
            });
        }
        w.e.register(MASK_TEST, |e, a| (e.mem.u32(a[0] + 0xc) & a[1]).into_ret());
        returns(&mut w.e, OBJECT_GET_EXTRA_DATA, 0);
        returns(&mut w.e, EXTRA_DATA_KEY, 0x77);
        returns(&mut w.e, HAS_TIME_CONTROLLERS, 0);
        returns(&mut w.e, CELL_WATER_OBJECT, 0);
        returns(&mut w.e, TES_WATER_SYSTEM, 0x8000);
        returns(&mut w.e, EXTRA_LIST_GET_ROOM_KEY, 0);
        (w, attaches)
    }

    /// Runs `AttachReference3D` and returns the node the 3D went under.
    fn attached_under(w: &mut World, attaches: &Calls, r: u32, node_3d: u32) -> Option<usize> {
        attaches.borrow_mut().clear();
        w.e.call(0x0054_8880, &args![w.cell, r, 0u32]);
        let seen = attaches.borrow().clone();
        seen.iter()
            .find(|call| call[1] == node_3d)
            .and_then(|call| w.nodes.iter().position(|&n| n == call[0]))
    }

    #[test]
    fn attach_3d_does_nothing_without_a_3d() {
        let (mut w, attaches) = attach_world();
        let (r, _, _) = placed_reference(&mut w, 0x15, false, false);
        start_log(&mut w.e);
        w.e.call(0x0054_8880, &args![w.cell, r, 0u32]);
        let log = take_log(&mut w.e);
        assert!(calls_to(&log, CELL_CHILD_NODE).is_empty());
        assert!(attaches.borrow().is_empty());
        w.e.call(0x0054_8880, &args![w.cell, 0u32, 0u32]);
    }

    #[test]
    fn attach_3d_picks_the_node_by_kind() {
        let (mut w, attaches) = attach_world();
        let light = w.nodes[1];
        let sound = object(&mut w.e);
        w.e.mem.set_u32(w.cell.addr() + 0xb4, light);
        w.e.mem.set_u32(w.cell.addr() + 0xb8, sound);
        let marker_230 = form(&mut w.e, 0x15, 0, 0);
        let marker_240 = form(&mut w.e, 0x15, 0, 0);
        w.e.set_global(MARKER_FORM_230, marker_230);
        w.e.set_global(MARKER_FORM_240, marker_240);

        // Actors go under child 0.
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, true, true);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(0));
        // The two marker forms under children 1 and 8.
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x20, marker_230);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(1));
        w.e.mem.set_u32(r + 0x20, marker_240);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(8));
        // Anything else under child 3.
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(3));
        // A reference whose virtual slot 0x224 answers yes: child 4.
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        answer(&mut w.e, r, 0x224, 1);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(4));
        // Form type 0x0D under the sound marker node (an object that is not
        // one of the cell's children), 0x0E likewise when the global test
        // says yes.
        let (r, _, node_3d) = placed_reference(&mut w, 0x0d, false, true);
        w.e.mem.set_u32(w.cell.addr() + 0xb8, w.nodes[2]);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(2));
        returns(&mut w.e, GLOBAL_VIRTUAL_TEST, 1);
        let (r, _, node_3d) = placed_reference(&mut w, 0x0e, false, true);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(2));
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(1));
        let _ = sound;
    }

    #[test]
    fn attach_3d_light_forms_use_the_light_node() {
        let (mut w, attaches) = attach_world();
        w.e.mem.set_u32(w.cell.addr() + 0xb4, w.nodes[7]);
        returns(&mut w.e, GLOBAL_VIRTUAL_TEST, 1);
        let adds = record(&mut w.e, CELL_ADD_MULTI_BOUND_REF, 0);
        let (r, base, node_3d) = placed_reference(&mut w, 0x1e, false, true);
        // No flag at +0xA8: the multibound bookkeeping is skipped.
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(7));
        assert!(adds.borrow().is_empty());
        // A flag makes `fn_00549630` answer no, so the cell adds the
        // reference.
        w.e.mem.set_u32(base + 0xa8, 0x40);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(7));
        assert_eq!(*adds.borrow(), vec![vec![w.cell.addr(), r]]);
    }

    #[test]
    fn attach_3d_multibound_marker_forms_and_time_controllers() {
        let (mut w, attaches) = attach_world();
        let adds = record(&mut w.e, CELL_ADD_MULTI_BOUND_REF, 0);
        // Form type 0x25 is added to the cell's multibound nodes.
        let (r, _, node_3d) = placed_reference(&mut w, 0x25, false, true);
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(3));
        assert_eq!(adds.borrow().len(), 1);
        // A light with no kind flags that is not a marker: added, no node.
        let (r, _, node_3d) = placed_reference(&mut w, 0x1e, false, true);
        w.e.mem.set_u32(w.cell.addr() + 0xb4, w.nodes[7]);
        let _ = (r, node_3d);
        // Extra data on the 3D with the mask bit and time controllers.
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let extra = object(&mut w.e);
        w.e.mem.set_u32(extra + 0xc, 1);
        returns(&mut w.e, OBJECT_GET_EXTRA_DATA, extra);
        returns(&mut w.e, HAS_TIME_CONTROLLERS, 1);
        let fade_checks = record(&mut w.e, FADE_NODE_CHECK_FADE_RADIUS, 0);
        adds.borrow_mut().clear();
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(3));
        assert_eq!(*adds.borrow(), vec![vec![w.cell.addr(), r]]);
        // The fade check gets the time controller flag.
        let fade = w.e.mem.u32(node_3d + ANSWERS + 0x10);
        assert_eq!(*fade_checks.borrow(), vec![vec![fade, 0x5100, 1]]);
    }

    #[test]
    fn attach_3d_uses_the_node_of_the_room_target_when_nothing_else_fits() {
        let (mut w, attaches) = attach_world();
        returns(&mut w.e, SETTING_FIRST_CHAR, 1);
        returns(&mut w.e, EXTRA_LIST_GET_ROOM_KEY, 0x4444);
        let target_node = object(&mut w.e);
        answer(&mut w.e, target_node, 0x14, 1);
        let finds = record(&mut w.e, CELL_NODE_FOR_TARGET, target_node);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x40, w.cell.addr());
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(*finds.borrow(), vec![vec![w.cell.addr(), 0x4444]]);
        // The node found is used for the attach (not child 3).
        let seen = attaches.borrow().clone();
        assert!(seen
            .iter()
            .any(|call| call[0] == target_node && call[1] == node_3d));
    }

    #[test]
    fn attach_3d_hides_trees_in_the_terrain_manager() {
        let (mut w, attaches) = attach_world();
        let hides = record(&mut w.e, TERRAIN_MANAGER_HIDE_TREE, 0);
        let flags = record(&mut w.e, TERRAIN_MANAGER_SET_FLAG_28, 0);
        let tree_flag = record(&mut w.e, FADE_NODE_SET_TREE_FLAG, 0);
        let (r, base, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(base + 8, 0x40);
        w.e.mem.set_u32(r + 0x190, 0x1234); // the world space
        assert_eq!(attached_under(&mut w, &attaches, r, node_3d), Some(3));
        assert_eq!(*hides.borrow(), vec![vec![0x7000, r, 1]]);
        assert_eq!(*flags.borrow(), vec![vec![0x7000, 1]]);
        // The fade node is marked as a tree.
        let fade = w.e.mem.u32(node_3d + ANSWERS + 0x10);
        assert_eq!(*tree_flag.borrow(), vec![vec![fade, 1]]);
        // Without a world space nothing is hidden.
        w.e.mem.set_u32(r + 0x190, 0);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(hides.borrow().len(), 1);
    }

    #[test]
    fn attach_3d_sets_the_fade_kind_of_distant_and_fading_references() {
        let (mut w, attaches) = attach_world();
        let kinds = record(&mut w.e, FADE_NODE_SET_KIND, 0);
        let ranges = record(&mut w.e, FADE_NODE_SET_RANGE, 0);
        w.e.mem.set_u32(0x0101_6970, 0x7f7f_ffff);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let fade = w.e.mem.u32(node_3d + ANSWERS + 0x10);
        // Visible-distant: kind 6.
        w.e.mem.set_u32(r + 0x180, 1);
        returns(&mut w.e, REFERENCE_HAS_VISIBLE_DISTANT, 1);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(*kinds.borrow(), vec![vec![fade, 6]]);
        // Otherwise a reference that fades out: kind 10 and FLT_MAX range.
        returns(&mut w.e, REFERENCE_HAS_VISIBLE_DISTANT, 0);
        returns(&mut w.e, REFERENCE_FADES_OUT, 1);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(kinds.borrow()[1], vec![fade, 10]);
        assert_eq!(*ranges.borrow(), vec![vec![fade, 0x7f7f_ffff, 0x7f7f_ffff]]);
    }

    #[test]
    fn attach_3d_resets_the_fade_while_the_loading_menu_is_up() {
        let (mut w, attaches) = attach_world();
        returns(&mut w.e, LOADING_MENU_VISIBLE, 1);
        w.e.set_global(LOD_FADE_IN_START, 0.25f32);
        let alpha = record(&mut w.e, SET_PROPERTY_FADE_ALPHA, 0);
        let checks = record(&mut w.e, FADE_NODE_CHECK_FADE_RADIUS, 0);
        w.e.set(w.cell, TESObjectCELL::bFadingToHighDetail, true);
        w.e.set(w.cell, TESObjectCELL::bFadingToLowDetail, true);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        let fade = w.e.mem.u32(node_3d + ANSWERS + 0x10);
        assert_eq!(*alpha.borrow(), vec![vec![fade, 1.0f32.to_bits()]]);
        assert!(checks.borrow().is_empty());
        assert_eq!(w.e.get(w.cell, TESObjectCELL::fLodFadeInPercent), 0.25);
        assert!(!w.e.get(w.cell, TESObjectCELL::bFadingToHighDetail));
        assert!(!w.e.get(w.cell, TESObjectCELL::bFadingToLowDetail));
        assert!(w.e.get(w.cell, TESObjectCELL::bDisplayHighDetail));
    }

    #[test]
    fn attach_3d_starts_the_fade_in_for_cells_with_loaded_distant_references() {
        let (mut w, attaches) = attach_world();
        let starts = record(&mut w.e, CELL_START_FADE_IN, 0);
        returns(&mut w.e, CELL_STATE_NONZERO, 1);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        // Loaded count 1 > 0 and loaded >= wanted (0 <= 1): the fade starts.
        w.e.set(w.cell, TESObjectCELL::sNumLoadedRefsWithVisibleDistant, 1);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(*starts.borrow(), vec![vec![w.cell.addr()]]);
        // Fewer loaded than wanted: not yet.
        w.e.set(w.cell, TESObjectCELL::sNumRefsWithVisibleDistant, 5);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(starts.borrow().len(), 1);
    }

    #[test]
    fn attach_3d_queues_the_attach_while_the_queue_is_on() {
        let (mut w, attaches) = attach_world();
        returns(&mut w.e, ATTACHES_ARE_QUEUED, 1);
        // The queue entry goes to the semaphore-guarded map.
        quiet(
            &mut w.e,
            &[
                SEMAPHORE_WAIT,
                SEMAPHORE_RELEASE,
                CRT_ATEXIT,
                ATTACH_ENTRY_DESTRUCT,
                QUEUED_ATTACH_MAP_SET_AT,
            ],
        );
        w.e.register(CREATE_SEMAPHORE, |_, _| 0x9999u32.into_ret());
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, true, true);
        start_log(&mut w.e);
        let under = attached_under(&mut w, &attaches, r, node_3d);
        let log = take_log(&mut w.e);
        // Nothing is attached at once; the queue got the child-0 node.
        assert_eq!(under, None);
        assert_eq!(calls_to(&log, QUEUED_ATTACH_MAP_SET_AT).len(), 1);
        assert_eq!(calls_to(&log, QUEUED_ATTACH_MAP_SET_AT)[0][1], node_3d);
    }

    #[test]
    fn attach_3d_attaches_both_roots_of_the_player() {
        let (mut w, attaches) = attach_world();
        let first = object(&mut w.e);
        let third = object(&mut w.e);
        w.e.register_double(PLAYER_GET_CURRENT_3D, move |_, a| {
            (if a[1] == 0 { first } else { third }).into_ret()
        });
        let updates = record(&mut w.e, NODE_UPDATE_PROPERTIES, 0);
        let prepares = record(&mut w.e, FADE_NODE_PREPARE, 0);
        let base = form(&mut w.e, 0x2a, 0, 0);
        let player_3d = object(&mut w.e);
        w.e.mem.set_u32(w.player + 0x20, base);
        w.e.mem.set_u32(w.player + 0x64, player_3d);
        answer(&mut w.e, w.player, 0x100, 1);
        // The first root has a fade node, the second does not.
        let fade = object(&mut w.e);
        answer(&mut w.e, first, 0x10, fade);
        w.e.call(0x0054_8880, &args![w.cell, w.player, 0u32]);
        // The actor's node (child 0) receives both roots.
        let seen = attaches.borrow().clone();
        assert_eq!(
            seen,
            vec![vec![w.nodes[0], first, 1], vec![w.nodes[0], third, 1]]
        );
        assert_eq!(*updates.borrow(), vec![vec![first], vec![third]]);
        assert_eq!(*prepares.borrow(), vec![vec![fade]]);
    }

    #[test]
    fn attach_3d_water_warns_about_waterless_interior_cells() {
        let (mut w, attaches) = attach_world();
        let prints = record(&mut w.e, DEBUG_PRINT, 0);
        let counts = record(&mut w.e, INCREMENT_FIELD_B8, 0);
        let bytes = record(&mut w.e, SET_BYTE_8, 0);
        let adds = record(&mut w.e, WATER_SYSTEM_ADD_PLACEABLE_WATER, 0);
        let water_adds = record(&mut w.e, CELL_ADD_WATER_REF, 0);
        // The cell's water object has an inner object at +0x24.
        let inner = object(&mut w.e);
        let water_object = object(&mut w.e);
        w.e.mem.set_u32(water_object + 0x24, inner);
        returns(&mut w.e, CELL_WATER_OBJECT, water_object);
        w.e.register(OBJECT_FIELD_24, |e, a| e.mem.u32(a[0] + 0x24).into_ret());
        // An interior cell without the water flag; the cell's name slot.
        w.e.mem.set_u8(w.cell.addr() + 0x24, 1);
        let c_id = 0xc011u32;
        w.e.mem.set_u32(w.cell.addr() + 0xc, c_id);
        answer(&mut w.e, w.cell.addr(), 0x130, 0x4a4a);
        // The setting's text is empty: the attach stops after the warning.
        let setting = w.e.mem.alloc(8);
        w.e.register_double(SETTING_TEXT, move |_, _| setting.into_ret());
        let (r, base, node_3d) = placed_reference(&mut w, 0x23, false, true);
        w.e.mem.set_u32(r + 0xc, 0xaa);
        let _ = base;
        let under = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(under, None);
        assert_eq!(
            *prints.borrow(),
            vec![vec![WATERLESS_CELL_MESSAGE, 0xaa, c_id, 0x4a4a]]
        );
        assert_eq!(*counts.borrow(), vec![vec![w.tes]]);
        assert_eq!(*bytes.borrow(), vec![vec![inner, 1]]);
        assert!(adds.borrow().is_empty());
        // With text in the setting the attach goes on and the water is
        // registered with the cell and the water system.
        w.e.mem.set_u8(setting, b'y');
        let under = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(under, Some(3));
        assert_eq!(*adds.borrow(), vec![vec![0x8000, r]]);
        assert_eq!(*water_adds.borrow(), vec![vec![w.cell.addr(), r]]);
        // A cell with the water flag does not warn.
        w.e.mem.set_u8(w.cell.addr() + 0x24, 3);
        let before = prints.borrow().len();
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(prints.borrow().len(), before);
    }

    #[test]
    fn attach_3d_registers_external_emittance() {
        let (mut w, attaches) = attach_world();
        let emits = record(&mut w.e, CELL_ADD_EMITTANCE_REF, 0);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x194, 1);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(*emits.borrow(), vec![vec![w.cell.addr(), r]]);
    }

    /// `attach_world` whose scene graph allocations are test objects, so
    /// that virtual calls on freshly built nodes work.
    fn scene_world() -> (World, Calls) {
        let (mut w, attaches) = attach_world();
        w.e.register(NODE_ALLOCATE, |e, _| object(e).into_ret());
        (w, attaches)
    }

    #[test]
    fn attach_3d_builds_the_room_of_a_master_room_marker() {
        let (mut w, attaches) = scene_world();
        let base = form(&mut w.e, 0x15, 0, 0);
        w.e.set_global(ROOM_MARKER_FORM_238, base);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x20, base);
        // The room table of the extra data lists.
        let rooms: Rc<RefCell<HashMap<u32, u32>>> = Rc::new(RefCell::new(HashMap::new()));
        let table = rooms.clone();
        w.e.register_double(EXTRA_LIST_GET_ROOM, move |_, a| {
            table.borrow().get(&a[0]).copied().unwrap_or(0).into_ret()
        });
        let table = rooms.clone();
        let set_rooms = Rc::new(RefCell::new(Vec::new()));
        let seen = set_rooms.clone();
        w.e.register_double(EXTRA_LIST_SET_ROOM, move |_, a| {
            table.borrow_mut().insert(a[0], a[1]);
            seen.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        returns(&mut w.e, EXTRA_LIST_GET_ROOM_IS_MASTER, 1);
        returns(&mut w.e, REFERENCE_GET_MULTI_BOUND_ROOM, 0);
        // The reference's position (three words) and the primitive's shape.
        let position = w.e.mem.alloc(12);
        for (i, word) in [1.5f32, 2.5, 3.5].iter().enumerate() {
            w.e.mem.set_f32(position + 4 * i as u32, *word);
        }
        returns(&mut w.e, REFERENCE_POSITION_ADDRESS, position);
        answer(&mut w.e, r, 0x1f4, 0x6666);
        let shape = object(&mut w.e);
        let primitive = object(&mut w.e);
        answer(&mut w.e, primitive, 0x14, shape);
        returns(&mut w.e, EXTRA_LIST_GET_PRIMITIVE, primitive);
        let make_shape = record(&mut w.e, fake(0x14), shape);
        let room_links = record(&mut w.e, ROOM_SET_MULTI_BOUND, 0);
        let shapes = record(&mut w.e, MULTI_BOUND_SET_SHAPE, 0);
        let multi_bound_positions = record(&mut w.e, fake(0x90), 0);
        let shape_positions = record(&mut w.e, fake(0xb8), 0);
        w.e.register(SLOT_AT_AC_GET, |e, a| e.mem.u32(a[0] + 0xac).into_ret());
        let holder = object(&mut w.e);
        returns(&mut w.e, GLOBAL_SLOT_GET, holder);
        w.e.register(SLOT_AT_C_GET, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        let graph = object(&mut w.e);
        returns(&mut w.e, CELL_PORTAL_GRAPH, graph);
        let graph_rooms = record(&mut w.e, PORTAL_GRAPH_ADD_ROOM, 0);
        // The room is made with its multibound; the multibound's slot
        // answers (read through the room's `+0xAC` slot by the double) are
        // filled in by the setter doubles.
        w.e.register_double(ROOM_SET_MULTI_BOUND, |e, a| {
            e.mem.set_u32(a[0] + 0xac, a[1]);
            Ret::default()
        });
        w.e.register_double(MULTI_BOUND_SET_SHAPE, |e, a| {
            e.mem.set_u32(a[0] + 0xc, a[1]);
            Ret::default()
        });
        w.e.register(MULTI_BOUND_ROOM_CONSTRUCT, |_, a| a[0].into_ret());
        w.e.register(MULTI_BOUND_CONSTRUCT, |_, a| a[0].into_ret());
        let _ = (room_links, shapes);
        start_log(&mut w.e);
        let under = attached_under(&mut w, &attaches, r, node_3d);
        let log = take_log(&mut w.e);
        // Child 1 of the cell's 3D takes the reference's 3D.
        assert_eq!(under, Some(1));
        // The room (0xEC bytes) and the multibound (0x10) were allocated.
        let allocations: Vec<u32> = calls_to(&log, NODE_ALLOCATE)
            .iter()
            .map(|call| call[0])
            .collect();
        assert_eq!(allocations, vec![0xec, 0x10]);
        // The primitive was asked for a shape at the reference's position
        // (the copy has the same three words).
        let asked = make_shape.borrow()[0].clone();
        assert_eq!(asked[0], primitive);
        for (i, word) in [1.5f32, 2.5, 3.5].iter().enumerate() {
            assert_eq!(w.e.mem.f32(asked[1] + 4 * i as u32), *word);
        }
        // The multibound got the shape and the position; the room was stored.
        let room = set_rooms.borrow()[0][1];
        assert_eq!(set_rooms.borrow()[0][0], r + 0x44);
        assert_ne!(w.e.mem.u32(room + 0xac), 0);
        let multi_bound = w.e.mem.u32(room + 0xac);
        assert_eq!(w.e.mem.u32(multi_bound + 0xc), shape);
        assert_eq!(
            *multi_bound_positions.borrow(),
            vec![vec![multi_bound, 0x6666]]
        );
        // The room's shape gets the position again and the graph the room.
        assert_eq!(*shape_positions.borrow(), vec![vec![shape, 0x6666]]);
        assert_eq!(*graph_rooms.borrow(), vec![vec![graph, room]]);

        // A reference that already has a room builds nothing new.
        set_rooms.borrow_mut().clear();
        let allocations = Rc::new(RefCell::new(0u32));
        let counter = allocations.clone();
        w.e.register_double(NODE_ALLOCATE, move |e, _| {
            *counter.borrow_mut() += 1;
            object(e).into_ret()
        });
        let under = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(under, Some(1));
        assert_eq!(*allocations.borrow(), 0);
        assert!(set_rooms.borrow().is_empty());
        assert_eq!(graph_rooms.borrow().len(), 2);

        // A room marker that is not a master room is only given child 1.
        returns(&mut w.e, EXTRA_LIST_GET_ROOM_IS_MASTER, 0);
        let under = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(under, Some(1));
        assert_eq!(graph_rooms.borrow().len(), 2);
    }

    #[test]
    fn attach_3d_builds_the_shape_and_node_of_a_portal() {
        let (mut w, attaches) = scene_world();
        let base = form(&mut w.e, 0x15, 0, 0);
        w.e.set_global(PORTAL_MARKER_FORM_23C, base);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x20, base);
        returns(&mut w.e, EXTRA_LIST_GET_PORTAL, 0);
        let shape = object(&mut w.e);
        let primitive = object(&mut w.e);
        answer(&mut w.e, primitive, 0x1c, shape);
        returns(&mut w.e, EXTRA_LIST_GET_PRIMITIVE, primitive);
        answer(&mut w.e, r, 0x1f4, 0x6666);
        let positions = record(&mut w.e, SHAPE_SET_POSITION, 0);
        let orientations = record(&mut w.e, SHAPE_SET_ORIENTATION, 0);
        w.e.register(REFERENCE_GET_ORIENTATION, |_, a| a[1].into_ret());
        let portals = record(&mut w.e, EXTRA_LIST_SET_PORTAL, 0);
        w.e.register(PORTAL_SHARED_NODE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0] + 0xb0, a[1]);
            a[0].into_ret()
        });
        w.e.register(FIXED_STRING_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        quiet(&mut w.e, &[FIXED_STRING_DESTRUCT]);
        let names = Rc::new(RefCell::new(Vec::new()));
        let seen = names.clone();
        w.e.register_double(OBJECT_SET_NAME, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        let graph = object(&mut w.e);
        returns(&mut w.e, CELL_PORTAL_GRAPH, graph);
        let graph_node = object(&mut w.e);
        returns(&mut w.e, PORTAL_GRAPH_NODE, graph_node);
        answer(&mut w.e, graph_node, 0x14, 1);
        // The linked references: the first has a room, the second none.
        let links = w.e.mem.alloc(8);
        let linked = object(&mut w.e);
        w.e.mem.set_u32(links, linked);
        returns(&mut w.e, EXTRA_LIST_GET_LINKS, links);
        let room = object(&mut w.e);
        w.e.register_double(REFERENCE_GET_MULTI_BOUND_ROOM, move |_, a| {
            (if a[0] == linked { room } else { 0 }).into_ret()
        });
        w.e.register(ADDRESS_PLUS_B4, |_, a| (a[0] + 0xb4).into_ret());
        let targets = record(&mut w.e, PORTAL_TARGET_ADD, 0);
        let first_rooms = record(&mut w.e, PORTAL_SET_FIRST_ROOM, 0);
        let second_rooms = record(&mut w.e, PORTAL_SET_SECOND_ROOM, 0);
        let unlinked = record(&mut w.e, PORTAL_GRAPH_ADD_PORTAL, 0);
        let under = attached_under(&mut w, &attaches, r, node_3d);
        // Child 6 of the cell's 3D takes the reference's 3D.
        assert_eq!(under, Some(6));
        assert_eq!(*positions.borrow(), vec![vec![shape, 0x6666]]);
        assert_eq!(orientations.borrow().len(), 1);
        assert_eq!(orientations.borrow()[0][0], shape);
        assert_eq!(*portals.borrow(), vec![vec![r + 0x44, shape]]);
        // The shared node is named and attached to the portal graph's node;
        // the shape remembers it in its slot at +0x104.
        let node = w.e.mem.u32(shape + 0x104);
        assert_eq!(*names.borrow(), vec![(node, PORTAL_SHARED_GEOMETRY_NAME)]);
        assert_eq!(w.e.mem.u32(node + 0xb0), shape);
        // Linked reference 0 gets the first room, the missing one the graph.
        assert_eq!(targets.borrow().len(), 1);
        assert_eq!(targets.borrow()[0][0], room + 0xb4);
        assert_eq!(*first_rooms.borrow(), vec![vec![shape, room]]);
        assert!(second_rooms.borrow().is_empty());
        assert_eq!(*unlinked.borrow(), vec![vec![graph, shape]]);
        // A reference whose extra data already has a portal skips it all.
        returns(&mut w.e, EXTRA_LIST_GET_PORTAL, 0x1111);
        let under = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(under, Some(6));
        assert_eq!(portals.borrow().len(), 1);
    }

    #[test]
    fn attach_3d_builds_the_plane_of_an_occlusion_plane_marker() {
        let (mut w, attaches) = scene_world();
        let base = form(&mut w.e, 0x15, 0, 0);
        w.e.set_global(OCCLUSION_MARKER_FORM_234, base);
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r + 0x20, base);
        answer(&mut w.e, r, 0x1f4, 0x6666);
        // First use: the extra data has no plane yet.
        let planes: Rc<RefCell<HashMap<u32, u32>>> = Rc::new(RefCell::new(HashMap::new()));
        let table = planes.clone();
        w.e.register_double(EXTRA_LIST_GET_OCCLUSION_PLANE, move |_, a| {
            table.borrow().get(&a[0]).copied().unwrap_or(0).into_ret()
        });
        let table = planes.clone();
        w.e.register_double(EXTRA_LIST_SET_OCCLUSION_PLANE, move |_, a| {
            table.borrow_mut().insert(a[0], a[1]);
            Ret::default()
        });
        w.e.register(OCCLUSION_PLANE_CONSTRUCT, |_, a| a[0].into_ret());
        let primitive = object(&mut w.e);
        returns(&mut w.e, EXTRA_LIST_GET_PRIMITIVE, primitive);
        w.e.register(PRIMITIVE_GET_DIMENSIONS, |e, a| {
            e.mem.set_f32(a[1], 2.0);
            e.mem.set_f32(a[1] + 4, 3.0);
            e.mem.set_f32(a[1] + 8, 5.0);
            a[1].into_ret()
        });
        w.e.register(VECTOR2_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            e.mem.set_u32(a[0] + 4, a[2]);
            a[0].into_ret()
        });
        w.e.register(REFERENCE_GET_ORIENTATION, |_, a| a[1].into_ret());
        let sizes = Rc::new(RefCell::new(Vec::new()));
        let seen = sizes.clone();
        w.e.register_double(PLANE_SET_SIZE, move |e, a| {
            seen.borrow_mut()
                .push((a[0], e.mem.u32(a[1]), e.mem.u32(a[1] + 4)));
            Ret::default()
        });
        let positions = record(&mut w.e, SHAPE_SET_POSITION, 0);
        let orientations = record(&mut w.e, SHAPE_SET_ORIENTATION, 0);
        let under = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(under, Some(5));
        let plane = *planes.borrow().get(&(r + 0x44)).unwrap();
        // Width and depth (words 0 and 2 of the dimensions) become the size.
        assert_eq!(
            *sizes.borrow(),
            vec![(plane, 2.0f32.to_bits(), 5.0f32.to_bits())]
        );
        assert_eq!(*positions.borrow(), vec![vec![plane, 0x6666]]);
        assert_eq!(orientations.borrow().len(), 1);
        assert_eq!(orientations.borrow()[0][0], plane);
        // The next time the plane is reused, not built again.
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(planes.borrow().len(), 1);
        assert_eq!(*planes.borrow().get(&(r + 0x44)).unwrap(), plane);
        assert_eq!(sizes.borrow().len(), 2);
        assert_eq!(sizes.borrow()[1].0, plane);
        // A primitive-less reference leaves the plane's size alone.
        returns(&mut w.e, EXTRA_LIST_GET_PRIMITIVE, 0);
        let _ = attached_under(&mut w, &attaches, r, node_3d);
        assert_eq!(sizes.borrow().len(), 2);
    }

    /// A linked list `{item, next}` of the given items (0 when empty).
    fn linked(e: &mut Engine, items: &[u32]) -> u32 {
        let mut next = 0;
        for item in items.iter().rev() {
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, *item);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        next
    }

    /// A container with the list of children at `+0xB4` and its count at
    /// `+0xBC`; returns the container.
    fn container(e: &mut Engine, items: &[u32]) -> u32 {
        let c = object(e);
        let head = linked(e, items);
        e.mem.set_u32(c + 0xb4, head);
        e.mem.set_u32(c + 0xbc, items.len() as u32);
        c
    }

    /// Rooms of the loaded data: `(key, room node)` pairs, and the keys the
    /// scrap map has seen.
    struct Rooms {
        entries: Vec<(u32, u32)>,
        visited: Vec<u32>,
    }

    /// The scene graph doubles of the room search: the loaded data's map of
    /// rooms, the portal graph's list, the children containers and the
    /// bounds.
    fn room_world() -> (World, Calls, Rc<RefCell<Rooms>>, u32) {
        let (mut w, attaches) = scene_world();
        let rooms = Rc::new(RefCell::new(Rooms {
            entries: vec![],
            visited: vec![],
        }));
        let table = rooms.clone();
        w.e.register_double(MAP_GET_AT, move |e, a| {
            if let Some((_, room)) = table.borrow().entries.iter().find(|(k, _)| *k == a[1]) {
                e.mem.set_u32(a[2], *room);
            }
            Ret::default()
        });
        let table = rooms.clone();
        w.e.register_double(MAP_FIRST_POSITION, move |_, _| {
            u32::from(!table.borrow().entries.is_empty()).into_ret()
        });
        let table = rooms.clone();
        w.e.register_double(MAP_GET_NEXT, move |e, a| {
            let entries = &table.borrow().entries;
            let index = e.mem.u32(a[1]) as usize - 1;
            e.mem.set_u32(a[2], entries[index].0);
            e.mem.set_u32(a[3], entries[index].1);
            let next = if index + 1 < entries.len() {
                index + 2
            } else {
                0
            };
            e.mem.set_u32(a[1], next as u32);
            Ret::default()
        });
        quiet(&mut w.e, &[SCRAP_MAP_CONSTRUCT, SCRAP_MAP_RELEASE]);
        let table = rooms.clone();
        w.e.register_double(SCRAP_MAP_SET, move |_, a| {
            table.borrow_mut().visited.push(a[1]);
            Ret::default()
        });
        let table = rooms.clone();
        w.e.register_double(SCRAP_MAP_GET, move |_, a| {
            u32::from(table.borrow().visited.contains(&a[1])).into_ret()
        });
        // The portal graph, its list at `+0x14` (count at list `+8`).
        let graph = object(&mut w.e);
        returns(&mut w.e, CELL_PORTAL_GRAPH, graph);
        returns(&mut w.e, CELL_PORTAL_GRAPH_CREATE, graph);
        w.e.register(PORTAL_GRAPH_LIST, |_, a| (a[0] + 0x14).into_ret());
        w.e.register(LIST_COUNT_IS_ZERO, |e, a| {
            u32::from(e.mem.u32(a[0] + 8) == 0).into_ret()
        });
        w.e.register(ADDRESS_PLUS_B4, |_, a| (a[0] + 0xb4).into_ret());
        w.e.register(ARRAY_COUNT, |e, a| e.mem.u32(a[0] + 8).into_ret());
        w.e.register(CHILD_ARRAY_NEXT, |e, a| {
            let node = e.mem.u32(a[1]);
            let next = e.mem.u32(node + 4);
            e.mem.set_u32(a[1], next);
            node.into_ret()
        });
        w.e.register(ITEM_SLOT_GET, |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        w.e.register(ROOM_TEST_INTERSECTION, |e, a| {
            e.mem.u32(a[0] + 0x20).into_ret()
        });
        w.e.register(NODE_GET_WORLD_BOUND, |_, a| (a[0] + 0x100).into_ret());
        w.e.register(BOUND_CENTER_ADDRESS, |_, a| a[0].into_ret());
        returns(&mut w.e, EXTRA_LIST_GET_OCCLUSION_PLANE, 0);
        returns(&mut w.e, REFERENCE_NOTE_ROOM, 0);
        returns(&mut w.e, EXTRA_LIST_GET_LINK_LIST, 0);
        returns(&mut w.e, EXTRA_LIST_GET_LINKED_KEY, 0);
        quiet(
            &mut w.e,
            &[
                PORTAL_GRAPH_ADD_NODE,
                PORTAL_GRAPH_REMOVE_NODE,
                PORTAL_GRAPH_ADD_PLANE,
            ],
        );
        (w, attaches, rooms, graph)
    }

    /// A room node whose children container holds `items` (each an object
    /// whose `+0x20` word says whether it contains the bound and whose
    /// `+0x30` word is the node it stands for); virtual slot `0x108` answers
    /// `contains_point`.
    fn room_node(e: &mut Engine, items: &[(u32, u32)], contains_point: bool) -> (u32, Vec<u32>) {
        let room = object(e);
        let mut objects = vec![];
        for (hit, target) in items {
            let item = object(e);
            e.mem.set_u32(item + 0x20, *hit);
            e.mem.set_u32(item + 0x30, *target);
            objects.push(item);
        }
        let children = container(e, &objects);
        answer(e, room, 0x104, children);
        answer(e, room, 0x108, u32::from(contains_point));
        (room, objects)
    }

    fn lock_log(e: &mut Engine) -> Log {
        take_log(e)
    }

    #[test]
    fn perform_cell_node_attach_puts_the_3d_under_the_node() {
        let (mut w, attaches, _, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let node = w.nodes[2];
        let updates = record(&mut w.e, SHADOW_SCENE_NODE_UPDATE_LIGHTING, 0);
        let scene_lookups = record(&mut w.e, SHADOW_SCENE_NODE_GETTER, 0x5200);
        let searches = record(&mut w.e, SETTING_FIRST_CHAR, 0);
        start_log(&mut w.e);
        w.e.call(0x0054_96b0, &args![w.cell, r, node]);
        let log = take_log(&mut w.e);
        // The node answers slot 0x14 with yes: no parent search.
        assert_eq!(attaches.borrow().len(), 1);
        assert_eq!(attaches.borrow()[0], vec![node, node_3d, 1]);
        assert!(searches.borrow().is_empty());
        // The scene node updates the lighting of the 3D; the lock brackets
        // everything.
        assert_eq!(*scene_lookups.borrow(), vec![vec![0]]);
        assert_eq!(*updates.borrow(), vec![vec![0x5200, node_3d, 0]]);
        let order = sequence(&log);
        assert!(order.contains(&CELL_LOCK_ENTER));
        assert_eq!(*order.last().unwrap(), CELL_LOCK_LEAVE);
    }

    #[test]
    fn perform_cell_node_attach_searches_unless_the_node_is_child_4_or_accepts() {
        let (mut w, _, _, _) = room_world();
        let (r, _, _) = placed_reference(&mut w, 0x15, false, true);
        quiet(&mut w.e, &[SHADOW_SCENE_NODE_UPDATE_LIGHTING]);
        let searches = record(&mut w.e, SETTING_FIRST_CHAR, 0);
        // A node whose slot 0x14 says no triggers the parent search.
        let node = object(&mut w.e);
        w.e.call(0x0054_96b0, &args![w.cell, r, node]);
        assert_eq!(searches.borrow().len(), 1);
        // Child 4 never does, nor does a node that says yes.
        w.e.call(0x0054_96b0, &args![w.cell, r, w.nodes[4]]);
        assert_eq!(searches.borrow().len(), 1);
        answer(&mut w.e, node, 0x14, 1);
        w.e.call(0x0054_96b0, &args![w.cell, r, node]);
        assert_eq!(searches.borrow().len(), 1);
        // Without a node it always searches.
        w.e.call(0x0054_96b0, &args![w.cell, r, 0u32]);
        assert_eq!(searches.borrow().len(), 2);
    }

    #[test]
    fn perform_cell_node_attach_without_a_node_detaches_the_3d() {
        let (mut w, attaches, _, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        quiet(&mut w.e, &[SHADOW_SCENE_NODE_UPDATE_LIGHTING]);
        returns(&mut w.e, SETTING_FIRST_CHAR, 0);
        let parent = object(&mut w.e);
        w.e.mem.set_u32(node_3d + 0x18, parent);
        let detaches = record(&mut w.e, fake(0xe8), 0);
        w.e.call(0x0054_96b0, &args![w.cell, r, 0u32]);
        assert_eq!(*detaches.borrow(), vec![vec![parent, node_3d]]);
        // The parent search that follows (no node) puts it under child 3.
        assert_eq!(*attaches.borrow(), vec![vec![w.nodes[3], node_3d, 1]]);
        // Nothing to do without a reference or a 3D.
        start_log(&mut w.e);
        let (bare, _, _) = placed_reference(&mut w, 0x15, false, false);
        w.e.call(0x0054_96b0, &args![w.cell, bare, 0u32]);
        w.e.call(0x0054_96b0, &args![w.cell, 0u32, 0u32]);
        assert!(calls_to(&lock_log(&mut w.e), CELL_LOCK_ENTER).is_empty());
    }

    #[test]
    fn room_attach_needs_the_room_and_its_children() {
        let (mut w, attaches, rooms, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let found = w.e.mem.alloc(4);
        // No room for the key: nothing happens.
        assert!(!fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        // A room whose children slot (0x104) answers no cannot host it.
        let (room, _) = room_node(&mut w.e, &[], true);
        answer(&mut w.e, room, 0x104, 0);
        rooms.borrow_mut().entries.push((0x40, room));
        assert!(!fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        assert!(attaches.borrow().is_empty());
        assert_eq!(w.e.mem.u8(found), 0);
    }

    #[test]
    fn room_attach_attaches_under_the_room_that_holds_the_point() {
        let (mut w, attaches, rooms, graph) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let found = w.e.mem.alloc(4);
        let (room, _) = room_node(&mut w.e, &[], true);
        rooms.borrow_mut().entries.push((0x40, room));
        let notes = record(&mut w.e, REFERENCE_NOTE_ROOM, 0);
        let adds = record(&mut w.e, PORTAL_GRAPH_ADD_NODE, 0);
        // The room slot of the loaded data is `+0x3C`.
        w.e.set(w.cell, TESObjectCELL::pLoadedData, Ptr::new(0x3000));
        let table = rooms.clone();
        w.e.register_double(MAP_GET_AT, move |e, a| {
            assert_eq!(a[0], 0x3000 + 0x3c);
            let room = table.borrow().entries[0].1;
            e.mem.set_u32(a[2], room);
            Ret::default()
        });
        assert!(fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        assert_eq!(w.e.mem.u8(found), 1);
        assert_eq!(*notes.borrow(), vec![vec![r, 0x40]]);
        // With no children to choose from, the 3D goes under the room node
        // itself, after being added to the portal graph.
        assert_eq!(*adds.borrow(), vec![vec![graph, node_3d]]);
        assert_eq!(attaches.borrow()[0], vec![room, node_3d, 1]);
    }

    #[test]
    fn room_attach_rejects_a_point_outside_unless_the_bound_fits() {
        let (mut w, attaches, rooms, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let found = w.e.mem.alloc(4);
        let (room, _) = room_node(&mut w.e, &[], false);
        rooms.borrow_mut().entries.push((0x40, room));
        assert!(!fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        // With the bounds check the room's slot 0x110 is asked with the
        // 3D's world bound; the portal graph's list is empty (count 0).
        assert!(!fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            true
        ));
        let asks = record(&mut w.e, fake(0x110), 1);
        assert!(!fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            true
        ));
        assert_eq!(*asks.borrow(), vec![vec![room, node_3d + 0x100]]);
        assert_eq!(w.e.mem.u8(found), 1);
        assert!(!attaches.borrow().is_empty());
    }

    #[test]
    fn room_attach_with_one_child_hit_goes_under_that_child() {
        let (mut w, attaches, rooms, graph) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let found = w.e.mem.alloc(4);
        let (miss_target, hit_target) = (object(&mut w.e), object(&mut w.e));
        let (room, _) = room_node(&mut w.e, &[(0, miss_target), (1, hit_target)], true);
        rooms.borrow_mut().entries.push((0x40, room));
        let adds = record(&mut w.e, PORTAL_GRAPH_ADD_NODE, 0);
        assert!(fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        assert_eq!(*adds.borrow(), vec![vec![graph, node_3d]]);
        assert_eq!(attaches.borrow()[0], vec![hit_target, node_3d, 1]);
        // When the 3D already hangs under that child nothing is attached.
        attaches.borrow_mut().clear();
        w.e.mem.set_u32(node_3d + 0x18, hit_target);
        assert!(fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        assert!(attaches.borrow().is_empty());
    }

    #[test]
    fn room_attach_with_two_hits_leaves_the_portal_graph() {
        let (mut w, attaches, rooms, graph) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let found = w.e.mem.alloc(4);
        let (a, b) = (object(&mut w.e), object(&mut w.e));
        let (room, _) = room_node(&mut w.e, &[(1, a), (1, b)], true);
        rooms.borrow_mut().entries.push((0x40, room));
        let removes = record(&mut w.e, PORTAL_GRAPH_REMOVE_NODE, 0);
        let parent = object(&mut w.e);
        w.e.mem.set_u32(node_3d + 0x18, parent);
        let detaches = record(&mut w.e, fake(0xe8), 0);
        // Without the bounds check the result is true even with two hits.
        assert!(fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        assert_eq!(*removes.borrow(), vec![vec![graph, node_3d]]);
        assert_eq!(*detaches.borrow(), vec![vec![parent, node_3d]]);
        assert!(attaches.borrow().is_empty());
        // An actor goes under child 0 instead.
        // An actor is skipped by the outer test (nothing moves) ...
        let actor = placed_reference(&mut w, 0x15, true, true);
        assert!(fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(actor.0),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        assert!(attaches.borrow().is_empty());
        assert_eq!(removes.borrow().len(), 1);
        // ... unless the second actor test (the original asks twice) says
        // yes: then it goes under child 0.
        let answers = Rc::new(RefCell::new(vec![false, true].into_iter()));
        let sequence_answers = answers.clone();
        w.e.register_double(fake(0x100), move |_, _| {
            u32::from(sequence_answers.borrow_mut().next().unwrap_or(false)).into_ret()
        });
        let flip = placed_reference(&mut w, 0x15, false, true);
        assert!(fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(flip.0),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        assert_eq!(attaches.borrow()[0], vec![w.nodes[0], node_3d, 1]);
    }

    #[test]
    fn room_attach_with_an_occlusion_plane_hands_over_the_plane() {
        let (mut w, attaches, rooms, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let found = w.e.mem.alloc(4);
        let (room, _) = room_node(&mut w.e, &[], true);
        rooms.borrow_mut().entries.push((0x40, room));
        returns(&mut w.e, EXTRA_LIST_GET_OCCLUSION_PLANE, 0x9999);
        let center = w.e.mem.alloc(12);
        returns(&mut w.e, PLANE_CENTER_ADDRESS, center);
        let adds = record(&mut w.e, ROOM_CHILDREN_ADD_PLANE, 0);
        // The room slot 0x108 test gets the plane's center.
        let _tests = record(&mut w.e, fake(0x108), 1);
        assert!(fn_005497c0(
            &mut w.e,
            w.cell,
            Ptr::new(r),
            node_3d,
            0x40,
            Ptr::new(found),
            false
        ));
        let children = w.e.mem.u32(room + ANSWERS + 0x104);
        assert_eq!(*adds.borrow(), vec![vec![children, 0x9999]]);
        assert!(attaches.borrow().is_empty());
        // That path does not set the found byte.
        assert_eq!(w.e.mem.u8(found), 0);
    }

    fn reparent(w: &mut World, r: u32, update_all: u8, check_bounds: u8) {
        w.e.call(0x0054_a070, &args![w.cell, r, update_all, check_bounds]);
    }

    /// Sets the portal graph's list of rooms to `items` (each `(hit,
    /// target)`), returning the item objects.
    fn graph_rooms(w: &mut World, graph: u32, items: &[(u32, u32)]) -> Vec<u32> {
        let mut objects = vec![];
        for (hit, target) in items {
            let item = object(&mut w.e);
            w.e.mem.set_u32(item + 0x20, *hit);
            w.e.mem.set_u32(item + 0x30, *target);
            objects.push(item);
        }
        let head = linked(&mut w.e, &objects);
        w.e.mem.set_u32(graph + 0x14, head);
        objects
    }

    #[test]
    fn reparent_ignores_references_it_must_not_move() {
        let (mut w, attaches, _, _) = room_world();
        let searches = record(&mut w.e, SETTING_FIRST_CHAR, 0);
        // No reference, then no 3D.
        reparent(&mut w, 0, 1, 0);
        let (bare, _, _) = placed_reference(&mut w, 0x15, false, false);
        reparent(&mut w, bare, 1, 0);
        // The portal and room marker forms.
        for global in [PORTAL_MARKER_FORM_23C, ROOM_MARKER_FORM_238] {
            let (r, base, _) = placed_reference(&mut w, 0x15, false, true);
            w.e.set_global(global, base);
            reparent(&mut w, r, 1, 0);
            w.e.set_global(global, 0u32);
        }
        // A 3D that already hangs under child 4.
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(node_3d + 0x18, w.nodes[4]);
        reparent(&mut w, r, 1, 0);
        assert!(searches.borrow().is_empty());
        assert!(attaches.borrow().is_empty());
    }

    #[test]
    fn reparent_only_searches_for_references_that_need_a_room_or_are_forced() {
        let (mut w, attaches, _, _) = room_world();
        let (r, base, node_3d) = placed_reference(&mut w, 0x15, false, true);
        // Neither forced nor wanted (the feature setting is off).
        reparent(&mut w, r, 0, 0);
        assert!(attaches.borrow().is_empty());
        // The setting has text and the reference has no key: kind 0x15 needs
        // a room; the kinds 0x2A..0x2D, 0x33 and 0x51 do not.
        returns(&mut w.e, SETTING_FIRST_CHAR, 1);
        reparent(&mut w, r, 0, 0);
        assert_eq!(attaches.borrow().len(), 1);
        for (kind, needs) in [
            (0x29u8, true),
            (0x2a, false),
            (0x2b, false),
            (0x2c, false),
            (0x2d, false),
            (0x2e, true),
            (0x33, false),
            (0x34, true),
            (0x51, false),
            (0x52, true),
            (0x00, true),
        ] {
            attaches.borrow_mut().clear();
            w.e.mem.set_u8(base + 4, kind);
            w.e.mem.set_u32(node_3d + 0x18, 0);
            reparent(&mut w, r, 0, 0);
            assert_eq!(!attaches.borrow().is_empty(), needs, "kind {kind:#x}");
        }
        // A reference with a key does not need the search either.
        attaches.borrow_mut().clear();
        w.e.mem.set_u8(base + 4, 0x15);
        returns(&mut w.e, EXTRA_LIST_GET_ROOM_KEY, 0x40);
        reparent(&mut w, r, 0, 0);
        assert!(attaches.borrow().is_empty());
        // Forced, it searches (and falls through to child 3).
        reparent(&mut w, r, 1, 0);
        assert_eq!(attaches.borrow().len(), 1);
    }

    #[test]
    fn reparent_defaults_to_child_3_or_child_0_for_actors() {
        let (mut w, attaches, _, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        reparent(&mut w, r, 1, 0);
        assert_eq!(*attaches.borrow(), vec![vec![w.nodes[3], node_3d, 1]]);
        attaches.borrow_mut().clear();
        let (a, _, node_3d) = placed_reference(&mut w, 0x15, true, true);
        reparent(&mut w, a, 1, 0);
        assert_eq!(*attaches.borrow(), vec![vec![w.nodes[0], node_3d, 1]]);
        // While attaches are queued the queue gets the node instead.
        attaches.borrow_mut().clear();
        returns(&mut w.e, ATTACHES_ARE_QUEUED, 1);
        quiet(
            &mut w.e,
            &[
                SEMAPHORE_WAIT,
                SEMAPHORE_RELEASE,
                CRT_ATEXIT,
                ATTACH_ENTRY_DESTRUCT,
                QUEUED_ATTACH_MAP_SET_AT,
            ],
        );
        w.e.register(CREATE_SEMAPHORE, |_, _| 0x9999u32.into_ret());
        let queued = record(&mut w.e, QUEUED_ATTACH_MAP_SET_AT, 0);
        reparent(&mut w, a, 1, 0);
        assert!(attaches.borrow().is_empty());
        assert_eq!(queued.borrow().len(), 1);
        assert_eq!(queued.borrow()[0][1], node_3d);
    }

    #[test]
    fn reparent_uses_the_portal_graph_room_that_holds_the_3d() {
        let (mut w, attaches, _, graph) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let (miss, hit) = (object(&mut w.e), object(&mut w.e));
        graph_rooms(&mut w, graph, &[(0, miss), (1, hit)]);
        let adds = record(&mut w.e, PORTAL_GRAPH_ADD_NODE, 0);
        reparent(&mut w, r, 1, 0);
        // One room: the graph gets the 3D and the room's node takes it.
        assert_eq!(*adds.borrow(), vec![vec![graph, node_3d]]);
        assert_eq!(*attaches.borrow(), vec![vec![hit, node_3d, 1]]);
    }

    #[test]
    fn reparent_with_several_graph_rooms_leaves_the_portal_graph() {
        let (mut w, attaches, _, graph) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        let (a, b) = (object(&mut w.e), object(&mut w.e));
        graph_rooms(&mut w, graph, &[(1, a), (1, b)]);
        let parent = object(&mut w.e);
        w.e.mem.set_u32(node_3d + 0x18, parent);
        let removes = record(&mut w.e, PORTAL_GRAPH_REMOVE_NODE, 0);
        let detaches = record(&mut w.e, fake(0xe8), 0);
        reparent(&mut w, r, 1, 0);
        assert_eq!(*removes.borrow(), vec![vec![graph, node_3d]]);
        assert_eq!(*detaches.borrow(), vec![vec![parent, node_3d]]);
        // Placed: nothing under child 3 any more.
        assert!(attaches.borrow().is_empty());
        // An actor (skipped by the outer test) is not moved either.
        let (actor, _, _) = placed_reference(&mut w, 0x15, true, true);
        reparent(&mut w, actor, 1, 0);
        assert_eq!(removes.borrow().len(), 1);
        assert!(attaches.borrow().is_empty());
    }

    #[test]
    fn reparent_tries_the_key_room_and_the_rooms_linked_to_it() {
        let (mut w, attaches, rooms, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        returns(&mut w.e, EXTRA_LIST_GET_ROOM_KEY, 0x40);
        // Key 0x40 has no room. Its extra data links one reference whose two
        // keys are 0x41 (no room) and 0x42 (a room that holds the point).
        let (room, _) = room_node(&mut w.e, &[], true);
        rooms.borrow_mut().entries.push((0x42, room));
        let linked_ref = object(&mut w.e);
        let link_list = linked(&mut w.e, &[linked_ref]);
        returns(&mut w.e, EXTRA_LIST_GET_LINK_LIST, link_list);
        w.e.register(EXTRA_LIST_GET_LINKED_KEY, |_, a| (0x41 + a[1]).into_ret());
        reparent(&mut w, r, 1, 0);
        assert_eq!(rooms.borrow().visited, vec![0x40, 0x41, 0x42]);
        assert_eq!(*attaches.borrow(), vec![vec![room, node_3d, 1]]);
        // A room found for the key itself ends the search at once.
        attaches.borrow_mut().clear();
        rooms.borrow_mut().visited.clear();
        rooms.borrow_mut().entries.push((0x40, room));
        reparent(&mut w, r, 1, 0);
        assert_eq!(rooms.borrow().visited, vec![0x40]);
        assert_eq!(attaches.borrow().len(), 1);
        // The bounds check skips the keyed search.
        attaches.borrow_mut().clear();
        rooms.borrow_mut().visited.clear();
        rooms.borrow_mut().entries.clear();
        reparent(&mut w, r, 1, 1);
        assert!(rooms.borrow().visited.is_empty());
    }

    #[test]
    fn reparent_walks_the_room_map() {
        let (mut w, attaches, rooms, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        returns(&mut w.e, SLOT_AT_AC_GET, 0xdada);
        returns(&mut w.e, REFERENCE_CHECKS_ROOM_DATA, 0);
        returns(&mut w.e, REFERENCE_FITS_ROOM_DATA, 0);
        // Entry 0x50 was visited already, 0x51 has a room that holds the
        // point (the first room that takes the 3D wins).
        let (hosting, _) = room_node(&mut w.e, &[], true);
        let (second, _) = room_node(&mut w.e, &[], true);
        rooms.borrow_mut().entries = vec![(0x50, second), (0x51, hosting), (0x52, second)];
        rooms.borrow_mut().visited.push(0x50);
        w.e.set(w.cell, TESObjectCELL::pLoadedData, Ptr::new(0x3000));
        reparent(&mut w, r, 1, 0);
        assert_eq!(*attaches.borrow(), vec![vec![hosting, node_3d, 1]]);
    }

    #[test]
    fn reparent_walk_hands_the_3d_to_the_node_of_a_room_target() {
        let (mut w, attaches, rooms, _) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        returns(&mut w.e, SLOT_AT_AC_GET, 0xdada);
        returns(&mut w.e, REFERENCE_CHECKS_ROOM_DATA, 1);
        let (room, _) = room_node(&mut w.e, &[], true);
        rooms.borrow_mut().entries = vec![(0x60, room)];
        w.e.set(w.cell, TESObjectCELL::pLoadedData, Ptr::new(0x3000));
        let target = object(&mut w.e);
        let finds = record(&mut w.e, CELL_NODE_FOR_TARGET, target);
        let notes = record(&mut w.e, REFERENCE_NOTE_ROOM, 0);
        reparent(&mut w, r, 0, 0);
        // `update_all` is 0 and the setting is off, so nothing is searched.
        assert!(attaches.borrow().is_empty());
        returns(&mut w.e, SETTING_FIRST_CHAR, 1);
        reparent(&mut w, r, 0, 0);
        assert_eq!(*finds.borrow(), vec![vec![w.cell.addr(), 0x60]]);
        assert_eq!(*attaches.borrow(), vec![vec![target, node_3d, 1]]);
        assert_eq!(*notes.borrow(), vec![vec![r, 0x60]]);
    }

    #[test]
    fn reparent_walk_queues_a_task_for_a_room_without_children() {
        let (mut w, attaches, rooms, graph) = room_world();
        let (r, _, node_3d) = placed_reference(&mut w, 0x15, false, true);
        returns(&mut w.e, SLOT_AT_AC_GET, 0xdada);
        returns(&mut w.e, REFERENCE_CHECKS_ROOM_DATA, 0);
        returns(&mut w.e, REFERENCE_FITS_ROOM_DATA, 1);
        // A room whose children slot answers no.
        let (room, _) = room_node(&mut w.e, &[], true);
        answer(&mut w.e, room, 0x104, 0);
        rooms.borrow_mut().entries = vec![(0x70, room)];
        w.e.set(w.cell, TESObjectCELL::pLoadedData, Ptr::new(0x3000));
        let set_ats = record(&mut w.e, QUEUED_ROOM_MAP_SET_AT, 0);
        // The task manager and the multibound of the reference.
        let manager = object(&mut w.e);
        w.e.set_global(TASK_MANAGER_POINTER, manager);
        let queued = record(&mut w.e, fake(0x48), 0);
        w.e.register(TASK_BASE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            a[0].into_ret()
        });
        returns(&mut w.e, REFERENCE_GET_MULTI_BOUND, 0x7777);
        let adds = record(&mut w.e, PORTAL_GRAPH_ADD_NODE, 0);
        returns(&mut w.e, SETTING_FIRST_CHAR, 1);
        reparent(&mut w, r, 1, 0);
        assert_eq!(*set_ats.borrow(), vec![vec![ROOM_TASK_MAP, r, 0x70]]);
        // The task is a `CheckWithinMultiBoundTask` for the key and the 3D.
        let task = queued.borrow()[0][1];
        assert_eq!(w.e.mem.u32(task), CHECK_WITHIN_MULTI_BOUND_TASK_VTABLE);
        assert_eq!(w.e.mem.u32(task + 0x18), 0x70);
        assert_eq!(w.e.mem.u32(task + 0x1c), node_3d);
        assert_eq!(w.e.mem.u32(task + 0x20), 0x7777);
        assert_eq!(*adds.borrow(), vec![vec![graph, node_3d]]);
        // The walk found nothing to place, so the default placement ran.
        assert_eq!(attaches.borrow().last().unwrap()[0], w.nodes[3]);
    }

    #[test]
    fn reparent_hands_a_planes_without_key_to_the_portal_graph() {
        let (mut w, _, _, graph) = room_world();
        let (r, _, _) = placed_reference(&mut w, 0x15, false, true);
        let planes = record(&mut w.e, PORTAL_GRAPH_ADD_PLANE, 0);
        returns(&mut w.e, EXTRA_LIST_GET_OCCLUSION_PLANE, 0x9999);
        reparent(&mut w, r, 0, 0);
        assert_eq!(*planes.borrow(), vec![vec![graph, 0x9999]]);
        // With a key the plane is not handed over.
        returns(&mut w.e, EXTRA_LIST_GET_ROOM_KEY, 0x40);
        reparent(&mut w, r, 0, 0);
        assert_eq!(planes.borrow().len(), 1);
    }

    /// Writes a list whose first node is embedded at `at` (as the cell's
    /// reference list is) with the given items.
    fn embedded_list(e: &mut Engine, at: u32, items: &[u32]) {
        let mut next = 0;
        for (i, item) in items.iter().enumerate().rev() {
            let node = if i == 0 { at } else { e.mem.alloc(8) };
            e.mem.set_u32(node, *item);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
    }

    #[test]
    fn task_constructor_sets_up_the_slots_and_the_multibound() {
        let mut e = engine();
        let this = e.mem.alloc(0x28);
        let base = record(&mut e, TASK_BASE_CONSTRUCT, 0);
        returns(&mut e, REFERENCE_GET_MULTI_BOUND, 0x7777);
        e.mem.set_u8(this + 0x24, 9);
        let back = e
            .call(0x0054_aa80, &args![this, 0x1234u32, 0x5678u32])
            .u32();
        assert_eq!(back, this);
        assert_eq!(*base.borrow(), vec![vec![this, 4]]);
        assert_eq!(e.mem.u32(this), CHECK_WITHIN_MULTI_BOUND_TASK_VTABLE);
        assert_eq!(e.mem.u32(this + 0x18), 0x1234);
        assert_eq!(e.mem.u32(this + 0x1c), 0x5678);
        // The slot at +0x20 starts empty and takes the reference's multibound.
        assert_eq!(e.mem.u32(this + 0x20), 0x7777);
        assert_eq!(e.mem.u8(this + 0x24), 0);
    }

    #[test]
    fn task_destructor_releases_the_slots_and_frees_on_request() {
        let mut e = engine();
        let this = e.mem.alloc(0x28);
        let released = record(&mut e, SLOT_RELEASE, 0);
        let base = record(&mut e, TASK_BASE_DESTRUCT, 0);
        let freed = Rc::new(RefCell::new(Vec::new()));
        let seen = freed.clone();
        e.register_double(DEALLOCATE, move |_, a| {
            seen.borrow_mut().push(a[0]);
            Ret::default()
        });
        // `_scalar_deleting_destructor_(0)`: only the body.
        assert_eq!(e.call(0x0054_ab30, &args![this, 0u32]).u32(), this);
        assert_eq!(
            *released.borrow(),
            vec![vec![this + 0x20], vec![this + 0x1c]]
        );
        assert_eq!(*base.borrow(), vec![vec![this]]);
        assert!(freed.borrow().is_empty());
        // Bit 0 of the flags frees the object as well.
        e.call(0x0054_ab30, &args![this, 3u32]);
        assert_eq!(*freed.borrow(), vec![this]);
        // The body alone does not free.
        e.call(0x0054_ab60, &args![this]);
        assert_eq!(freed.borrow().len(), 1);
        assert_eq!(released.borrow().len(), 6);
    }

    /// The doubles of the attach queue: its semaphore and map.
    fn queue_engine() -> (Engine, Calls, Calls, Calls) {
        let mut e = engine();
        let order = Rc::new(RefCell::new(Vec::new()));
        for (address, tag) in [(SEMAPHORE_WAIT, 1u32), (SEMAPHORE_RELEASE, 2)] {
            let seen = order.clone();
            e.register_double(address, move |_, a| {
                seen.borrow_mut().push(vec![tag, a[0]]);
                Ret::default()
            });
        }
        let removes = record(&mut e, QUEUED_ATTACH_MAP_REMOVE_AT, 0);
        let sets = record(&mut e, QUEUED_ATTACH_MAP_SET_AT, 0);
        quiet(&mut e, &[ATTACH_ENTRY_DESTRUCT, CRT_ATEXIT]);
        e.register(CREATE_SEMAPHORE, |_, _| 0x4321u32.into_ret());
        (e, order, removes, sets)
    }

    #[test]
    fn queue_attach_initializes_the_semaphore_once_and_records_the_entry() {
        let (mut e, order, removes, sets) = queue_engine();
        let atexit = record(&mut e, CRT_ATEXIT, 0);
        let node = object(&mut e);
        let node_3d = object(&mut e);
        let r = reference(&mut e, 0, node_3d, false);
        e.call(0x0054_abd0, &args![r, node]);
        // The first use built the semaphore and registered its destructor.
        assert_eq!(e.global::<u32>(QUEUE_SEMAPHORE_GUARD) & 1, 1);
        assert_eq!(e.mem.u32(QUEUE_SEMAPHORE), 1);
        assert_eq!(e.mem.u32(QUEUE_SEMAPHORE + 4), 0x4321);
        assert_eq!(e.mem.u32(QUEUE_SEMAPHORE + 8), 2);
        assert_eq!(*atexit.borrow(), vec![vec![QUEUE_SEMAPHORE_DESTRUCTOR]]);
        // The map entry for the 3D is `(3D, node, true)` as three words.
        assert_eq!(
            *sets.borrow(),
            vec![vec![QUEUED_ATTACH_MAP, node_3d, node_3d, node, 1]]
        );
        // The wait and the release bracket it.
        assert_eq!(
            *order.borrow(),
            vec![vec![1, QUEUE_SEMAPHORE], vec![2, QUEUE_SEMAPHORE]]
        );
        assert!(removes.borrow().is_empty());
        // The second use does not initialize again.
        e.call(0x0054_abd0, &args![r, node]);
        assert_eq!(atexit.borrow().len(), 1);
        assert_eq!(sets.borrow().len(), 2);
    }

    #[test]
    fn queue_attach_drops_the_entry_when_the_3d_is_already_there() {
        let (mut e, _, removes, sets) = queue_engine();
        let node = object(&mut e);
        let node_3d = object(&mut e);
        e.mem.set_u32(node_3d + 0x18, node);
        let r = reference(&mut e, 0, node_3d, false);
        e.call(0x0054_abd0, &args![r, node]);
        assert_eq!(*removes.borrow(), vec![vec![QUEUED_ATTACH_MAP, node_3d]]);
        assert!(sets.borrow().is_empty());
        // Nothing for a null reference or one without a 3D.
        let bare = reference(&mut e, 0, 0, false);
        e.call(0x0054_abd0, &args![bare, node]);
        e.call(0x0054_abd0, &args![0u32, node]);
        assert_eq!(removes.borrow().len(), 1);
    }

    #[test]
    fn queue_entry_and_semaphore_helpers() {
        let mut e = engine();
        // The default entry: two empty slots and the flag byte set.
        let entry = e.mem.alloc(12);
        e.mem.set_u32(entry, 0xdead);
        assert_eq!(e.call(0x0054_acf0, &args![entry]).u32(), entry);
        assert_eq!(
            (e.mem.u32(entry), e.mem.u32(entry + 4), e.mem.u8(entry + 8)),
            (0, 0, 1)
        );
        // The copy: both slots and the flag.
        let source = e.mem.alloc(12);
        e.mem.set_u32(source, 0x11);
        e.mem.set_u32(source + 4, 0x22);
        e.mem.set_u8(source + 8, 0);
        let copy = e.mem.alloc(12);
        assert_eq!(e.call(0x0054_adb0, &args![copy, source]).u32(), copy);
        assert_eq!(
            (e.mem.u32(copy), e.mem.u32(copy + 4), e.mem.u8(copy + 8)),
            (0x11, 0x22, 0)
        );
        // The semaphore wrapper: `(0, count, count + 1, 0)` to the import.
        let calls = record(&mut e, CREATE_SEMAPHORE, 0x55);
        let sem = e.mem.alloc(12);
        assert_eq!(e.call(0x0054_ad60, &args![sem, 5u32]).u32(), sem);
        assert_eq!(*calls.borrow(), vec![vec![0, 5, 6, 0]]);
        assert_eq!(
            (e.mem.u32(sem), e.mem.u32(sem + 4), e.mem.u32(sem + 8)),
            (5, 0x55, 6)
        );
    }

    #[test]
    fn queued_attaches_run_for_entries_whose_3d_moved() {
        let (mut w, attaches, _, _) = room_world();
        quiet(&mut w.e, &[SHADOW_SCENE_NODE_UPDATE_LIGHTING]);
        // Four queued entries: (3D, queued node).
        let queued_node = w.nodes[5];
        let (r1, _, moved) = placed_reference(&mut w, 0x15, false, true);
        let (r2, _, settled) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(settled + 0x18, queued_node);
        let (r3, _, orphan) = placed_reference(&mut w, 0x15, false, true);
        w.e.mem.set_u32(r3 + 0x40, 0);
        let entries = vec![(1u32, moved), (2, settled), (3, orphan), (4, 0)];
        let refs = [(moved, r1), (settled, r2), (orphan, r3)];
        let table = Rc::new(RefCell::new(entries.clone()));
        let seen = table.clone();
        w.e.register_double(MAP_FIRST_POSITION, move |_, _| {
            u32::from(!seen.borrow().is_empty()).into_ret()
        });
        let seen = table.clone();
        w.e.register_double(QUEUED_ATTACH_MAP_GET_NEXT, move |e, a| {
            let index = e.mem.u32(a[1]) as usize - 1;
            let (key, node_3d) = seen.borrow()[index];
            e.mem.set_u32(a[2], key);
            e.mem.set_u32(a[3], node_3d);
            e.mem.set_u32(a[3] + 4, queued_node);
            let next = if index + 1 < seen.borrow().len() {
                index + 2
            } else {
                0
            };
            e.mem.set_u32(a[1], next as u32);
            Ret::default()
        });
        w.e.register_double(FIND_REFERENCE_FOR_3D, move |_, a| {
            refs.iter()
                .find(|(node, _)| *node == a[0])
                .map(|(_, r)| *r)
                .unwrap_or(0)
                .into_ret()
        });
        let clears = record(&mut w.e, QUEUED_ATTACH_MAP_CLEAR, 0);
        quiet(&mut w.e, &[ATTACH_ENTRY_DESTRUCT]);
        answer(&mut w.e, queued_node, 0x14, 1);
        w.e.call(0x0054_ae30, &args![]);
        // Only the moved 3D is attached, under the queued node, by its own
        // cell (the parent cell of the reference).
        assert_eq!(*attaches.borrow(), vec![vec![queued_node, moved, 1]]);
        assert_eq!(*clears.borrow(), vec![vec![QUEUED_ATTACH_MAP]]);
    }

    #[test]
    fn process_list_handlers_run_under_the_lock() {
        let mut e = engine();
        let c = cell(&mut e);
        let handled = record(&mut e, PROCESS_LISTS_HANDLE_CELL_REFERENCES, 0);
        let removed = record(&mut e, PROCESS_LISTS_REMOVE_CELL_REFERENCES, 0);
        start_log(&mut e);
        e.call(0x0054_af40, &args![c, 1u32]);
        let order = sequence(&take_log(&mut e));
        assert_eq!(
            *handled.borrow(),
            vec![vec![PROCESS_LISTS, c.addr() + 0xac, 1]]
        );
        assert_eq!(order.first(), Some(&CELL_LOCK_ENTER));
        assert_eq!(order.last(), Some(&CELL_LOCK_LEAVE));
        start_log(&mut e);
        e.call(0x0054_af80, &args![c]);
        let order = sequence(&take_log(&mut e));
        assert_eq!(
            *removed.borrow(),
            vec![vec![PROCESS_LISTS, c.addr() + 0xac]]
        );
        assert_eq!(order.first(), Some(&CELL_LOCK_ENTER));
        assert_eq!(order.last(), Some(&CELL_LOCK_LEAVE));
    }

    /// The doubles of the random reference pick: a cast that sees cells and
    /// world spaces by their own address, and word-driven filters.
    fn pick_engine() -> Engine {
        let mut e = engine();
        e.register(RT_DYNAMIC_CAST, |e, a| {
            // `+0x300` of an object says what it casts to: 1 the cell type,
            // 2 the world space type, 3 the filter type.
            let kind = e.mem.u32(a[0] + 0x300);
            let wanted = a[3];
            let matches = (wanted == RTTI_CELL && kind == 1)
                || (wanted == RTTI_WORLD_SPACE && kind == 2)
                || (wanted == RTTI_FORM_FILTER_TO && kind == 3);
            (if matches { a[0] } else { 0 }).into_ret()
        });
        e.register(FILTER_OBJECT_USABLE, |_, _| 1u32.into_ret());
        e.register(FILTER_OBJECT_ACCEPTS, |e, a| {
            e.mem.u32(a[0] + 0x304).into_ret()
        });
        e.register(REFERENCE_BLOCKED, |e, a| e.mem.u32(a[0] + 0x308).into_ret());
        e.register(WORLD_SPACE_PERSISTENT_CELL, |e, a| {
            e.mem.u32(a[0] + 0x34).into_ret()
        });
        e
    }

    /// A reference for the pick: base form of type `form_type` that casts to
    /// the filter type, accepting (`+0x304`) and not blocked (`+0x308`).
    fn candidate(e: &mut Engine, form_type: u8, flags: u32, accepts: bool, blocked: bool) -> u32 {
        let base = form(e, form_type, 0, 0);
        e.mem.set_u32(base + 0x300, 3);
        e.mem.set_u32(base + 0x304, u32::from(accepts));
        let r = object(e);
        e.mem.set_u32(r + 0x20, base);
        e.mem.set_u32(r + 8, flags);
        e.mem.set_u32(r + 0x308, u32::from(blocked));
        r
    }

    #[test]
    fn pick_returns_null_for_missing_arguments() {
        let mut e = pick_engine();
        let a = cell(&mut e);
        start_log(&mut e);
        for (x, y, z) in [
            (0, 0x20, 0x30),
            (a.addr(), 0, 0x30),
            (a.addr(), 0x20, 0),
            (a.addr(), a.addr(), 0x30),
        ] {
            assert_eq!(e.call(0x0054_afb0, &args![x, y, z]).u32(), 0);
        }
        assert_eq!(take_log(&mut e).len(), 4);
    }

    #[test]
    fn pick_chooses_among_the_qualifying_references_of_a_cell() {
        let mut e = pick_engine();
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0x300, 1);
        let ok1 = candidate(&mut e, 0x1c, 0, true, false);
        let wrong_type = candidate(&mut e, 0x1d, 0, true, false);
        let flagged = candidate(&mut e, 0x1c, 0x20, true, false);
        let refused = candidate(&mut e, 0x1c, 0, false, false);
        let blocked = candidate(&mut e, 0x1c, 0, true, true);
        let temporary = candidate(&mut e, 0x1c, 0x800, true, false);
        let ok2 = candidate(&mut e, 0x1c, 0, true, false);
        embedded_list(
            &mut e,
            c.addr() + 0xac,
            &[ok1, wrong_type, flagged, refused, blocked, temporary, ok2],
        );
        let draws = Rc::new(RefCell::new(Vec::new()));
        let seen = draws.clone();
        e.register_double(RANDOM_BELOW, move |_, a| {
            seen.borrow_mut().push((a[0], a[1]));
            // Index 1 of the qualifying list (the newest push comes first).
            1u32.into_ret()
        });
        let b = object(&mut e);
        let anything = object(&mut e);
        let picked = e.call(0x0054_afb0, &args![c, b, anything]).u32();
        // Two qualify (ok1, ok2); the local list holds them newest first, so
        // index 1 is the first one found.
        assert_eq!(*draws.borrow(), vec![(0, 2)]);
        assert_eq!(picked, ok1);
        // With a single candidate any draw is that one's.
        embedded_list(&mut e, c.addr() + 0xac, &[wrong_type, ok2]);
        e.register_double(RANDOM_BELOW, |_, _| 0u32.into_ret());
        assert_eq!(e.call(0x0054_afb0, &args![c, b, anything]).u32(), ok2);
        // Nothing qualifies: null and no draw.
        embedded_list(&mut e, c.addr() + 0xac, &[wrong_type, flagged]);
        assert_eq!(e.call(0x0054_afb0, &args![c, b, anything]).u32(), 0);
        assert_eq!(draws.borrow().len(), 1);
    }

    #[test]
    fn pick_reads_the_persistent_cell_of_a_world_space() {
        let mut e = pick_engine();
        let world = object(&mut e);
        e.mem.set_u32(world + 0x300, 2);
        let persistent = cell(&mut e);
        e.mem.set_u32(world + 0x34, persistent.addr());
        let ok = candidate(&mut e, 0x1c, 0, true, false);
        embedded_list(&mut e, persistent.addr() + 0xac, &[ok]);
        e.register_double(RANDOM_BELOW, |_, _| 0u32.into_ret());
        let locks = record(&mut e, CELL_LOCK_ENTER, 0);
        let b = object(&mut e);
        let anything = object(&mut e);
        assert_eq!(e.call(0x0054_afb0, &args![world, b, anything]).u32(), ok);
        // The persistent cell's lock is the one taken.
        assert_eq!(*locks.borrow(), vec![vec![persistent.addr()]]);
        // An object that is neither: the lock is taken on nothing.
        let other = object(&mut e);
        assert_eq!(e.call(0x0054_afb0, &args![other, b, anything]).u32(), 0);
        assert_eq!(locks.borrow()[1], vec![0]);
    }

    /// The doubles of the container copy: items owned by the item
    /// (`+0x310` owner, `+0x314` is an owner), the extra data and the
    /// inventory changes.
    fn copy_engine() -> (Engine, Calls, Calls, Calls, Calls) {
        let mut e = engine();
        e.register(EXTRA_LIST_GET_MERCHANT_CONTAINER, |e, a| {
            e.mem.u32(a[0]).into_ret()
        });
        e.register(REFERENCE_GET_OWNER, |e, a| {
            e.mem.u32(a[0] + 0x310).into_ret()
        });
        e.register(REFERENCE_IS_AN_OWNER, |e, a| {
            e.mem.u32(a[0] + 0x314).into_ret()
        });
        // `ItemChange::ItemChange(this, form, 0)`: the list is made later.
        e.register(ITEM_CHANGE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, a[2]);
            e.mem.set_u32(a[0] + 8, a[1]);
            a[0].into_ret()
        });
        e.register(EXTRA_LIST_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(EXTRA_LIST_GET_COUNT, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(EXTRA_LIST_GET_CONTAINER_CHANGES, |e, a| {
            e.mem.u32(a[0]).into_ret()
        });
        let duplicates = record(&mut e, EXTRA_LIST_DUPLICATE_FOR_CONTAINER, 0);
        let counts = record(&mut e, EXTRA_LIST_SET_COUNT, 0);
        let adds = record(&mut e, CHANGES_ADD, 0);
        let merges = record(&mut e, CONTAINER_CHANGES_MERGE, 0);
        quiet(
            &mut e,
            &[
                EXTRA_LIST_SET_REFERENCE,
                EXTRA_LIST_REMOVE_OWNERSHIP,
                ITEM_CHANGE_SET_COUNT_DELTA,
            ],
        );
        (e, duplicates, counts, adds, merges)
    }

    /// An owned reference of the given base form type: `+0x310` owner,
    /// `+0x314` owned by the item, `+0x2D4` the count word, `+0x2D8` the
    /// container changes.
    fn owned(e: &mut Engine, form_type: u8, flags: u32) -> u32 {
        let base = form(e, form_type, 0, 0);
        let r = object(e);
        e.mem.set_u32(r + 0x20, base);
        e.mem.set_u32(r + 8, flags);
        e.mem.set_u32(r + 0x310, 0x1000);
        e.mem.set_u32(r + 0x314, 1);
        r
    }

    #[test]
    fn copy_owned_references_builds_an_item_change_for_container_kinds() {
        let (mut e, duplicates, counts, adds, _) = copy_engine();
        let sets = record(&mut e, ITEM_CHANGE_SET_COUNT_DELTA, 0);
        let c = cell(&mut e);
        let item = object(&mut e);
        let destination = object(&mut e);
        let first = owned(&mut e, 0x18, 0);
        e.mem.set_u32(first + 0x44, 0xfffe); // -2 as a short
        let counted = owned(&mut e, 0x73, 0);
        e.mem.set_u32(counted + 0x44, 3);
        embedded_list(&mut e, c.addr() + 0xac, &[first, counted]);
        let referenced = record(&mut e, EXTRA_LIST_SET_REFERENCE, 0);
        start_log(&mut e);
        e.call(0x0054_b260, &args![c, item, destination]);
        let log = take_log(&mut e);
        // Each qualifying reference: an `ItemChange` of its base form with a
        // list, a copy of its extra data and the count; added to the
        // destination with 1.
        let constructs = calls_to(&log, ITEM_CHANGE_CONSTRUCT);
        assert_eq!(constructs.len(), 2);
        assert_eq!(constructs[0][1], e.mem.u32(first + 0x20));
        assert_eq!(constructs[0][2], 0);
        let first_change = adds.borrow()[0][1];
        assert_eq!(adds.borrow()[0], vec![destination, first_change, 1]);
        let first_list = e.mem.u32(first_change);
        assert_ne!(first_list, 0);
        // The copied extra data was pushed on the change's list and made from
        // the reference's own list (+0x44).
        let extras = e.mem.u32(first_list);
        assert_eq!(duplicates.borrow()[0], vec![extras, first + 0x44]);
        assert_eq!(referenced.borrow()[0], vec![extras, first]);
        // The count is the short in the extra data, sign extended.
        assert_eq!(sets.borrow()[0], vec![first_change, (-2i32) as u32]);
        // Type 0x73 first sets the reference's own count to 1.
        assert_eq!(*counts.borrow(), vec![vec![counted + 0x44, 1]]);
        assert_eq!(sets.borrow()[1][1], 3);
        assert_eq!(adds.borrow().len(), 2);
        // The lock brackets the walk.
        let order = sequence(&log);
        assert!(order.contains(&CELL_LOCK_ENTER) && order.contains(&CELL_LOCK_LEAVE));
    }

    #[test]
    fn copy_owned_references_skips_what_the_item_does_not_own() {
        let (mut e, _, counts, adds, merges) = copy_engine();
        let c = cell(&mut e);
        let item = object(&mut e);
        let destination = object(&mut e);
        // The item's merchant container is excluded.
        e.mem.set_u32(item + 0x44, 0);
        let excluded = owned(&mut e, 0x18, 0);
        e.mem.set_u32(item + 0x44, excluded);
        let temporary = owned(&mut e, 0x18, 0x800);
        let flag_20 = owned(&mut e, 0x18, 0x20);
        let ownerless = owned(&mut e, 0x18, 0);
        e.mem.set_u32(ownerless + 0x310, 0);
        let not_an_owner = owned(&mut e, 0x18, 0);
        e.mem.set_u32(not_an_owner + 0x314, 0);
        let other_kind = owned(&mut e, 0x15, 0);
        let qualifying = owned(&mut e, 0x18, 0);
        embedded_list(
            &mut e,
            c.addr() + 0xac,
            &[
                excluded,
                temporary,
                flag_20,
                ownerless,
                not_an_owner,
                other_kind,
                qualifying,
            ],
        );
        e.call(0x0054_b260, &args![c, item, destination]);
        // Only the last one qualifies.
        assert_eq!(adds.borrow().len(), 1);
        assert!(counts.borrow().is_empty());
        assert!(merges.borrow().is_empty());
        // A null item or destination does nothing at all.
        start_log(&mut e);
        e.call(0x0054_b260, &args![c, 0u32, destination]);
        e.call(0x0054_b260, &args![c, item, 0u32]);
        assert_eq!(take_log(&mut e).len(), 2);
    }

    #[test]
    fn copy_owned_references_merges_the_container_changes_of_type_0x1b() {
        let (mut e, _, _, adds, merges) = copy_engine();
        let c = cell(&mut e);
        let item = object(&mut e);
        let destination = object(&mut e);
        let with_changes = owned(&mut e, 0x1b, 0);
        e.mem.set_u32(with_changes + 0x44, 0x4444);
        let without = owned(&mut e, 0x1b, 0);
        embedded_list(&mut e, c.addr() + 0xac, &[with_changes, without]);
        e.call(0x0054_b260, &args![c, item, destination]);
        assert_eq!(
            *merges.borrow(),
            vec![vec![0x4444, destination, with_changes, 0]]
        );
        assert!(adds.borrow().is_empty());
    }

    #[test]
    fn copy_owned_references_stops_at_an_empty_list_entry() {
        let (mut e, _, _, adds, _) = copy_engine();
        let c = cell(&mut e);
        let item = object(&mut e);
        let destination = object(&mut e);
        let ok = owned(&mut e, 0x18, 0);
        // A node holding null ends the walk before the next one.
        embedded_list(&mut e, c.addr() + 0xac, &[0, ok]);
        e.call(0x0054_b260, &args![c, item, destination]);
        assert!(adds.borrow().is_empty());
    }

    /// The doubles of `SaveGameTest`: actors cast by `+0x300 == 4`,
    /// container changes at `+0x2D8`, essential at `+0x320`, locked at
    /// `+0x324`.
    fn save_test_engine() -> (Engine, Ptr<TESObjectCELL>, u32, Calls, Calls, Calls) {
        let mut e = engine();
        let player = object(&mut e);
        e.set_global(PLAYER_POINTER, player);
        e.set_global(SAVE_TEST_CONSTANT, 10.0f32);
        e.register(RT_DYNAMIC_CAST, |e, a| {
            (if e.mem.u32(a[0] + 0x300) == 4 {
                a[0]
            } else {
                0
            })
            .into_ret()
        });
        e.register(EXTRA_LIST_GET_CONTAINER_CHANGES, |e, a| {
            e.mem.u32(a[0]).into_ret()
        });
        e.register(ACTOR_IS_ESSENTIAL, |e, a| {
            e.mem.u32(a[0] + 0x320).into_ret()
        });
        e.register(REFERENCE_GET_LOCK, |e, a| {
            e.mem.u32(a[0] + 0x324).into_ret()
        });
        let updates = record(&mut e, CONTAINER_CHANGES_UPDATE, 0);
        let kills = record(&mut e, ACTOR_KILL, 0);
        let unlocks = record(&mut e, REFERENCE_UNLOCK, 0);
        quiet(&mut e, &[SAVE_TEST_FINISH]);
        let c = cell(&mut e);
        (e, c, player, updates, kills, unlocks)
    }

    fn actor(e: &mut Engine, flags: u32) -> u32 {
        let r = object(e);
        e.mem.set_u32(r + 8, flags);
        e.mem.set_u32(r + 0x300, 4);
        r
    }

    #[test]
    fn save_game_test_updates_and_kills_ordinary_actors() {
        let (mut e, c, player, updates, kills, unlocks) = save_test_engine();
        let first = actor(&mut e, 0);
        e.mem.set_u32(first + 0x44, 0x4444);
        let essential = actor(&mut e, 0);
        e.mem.set_u32(essential + 0x320, 1);
        let temporary = actor(&mut e, 0x800);
        let locked = actor(&mut e, 0);
        e.mem.set_u32(locked + 0x324, 1);
        embedded_list(
            &mut e,
            c.addr() + 0xac,
            &[player, 0, first, essential, temporary, locked],
        );
        let slot_460 = record(&mut e, fake(0x460), 0);
        let slot_424 = record(&mut e, fake(0x424), 0);
        let finishes = record(&mut e, SAVE_TEST_FINISH, 0);
        start_log(&mut e);
        e.call(0x0054_b5b0, &args![c]);
        let log = take_log(&mut e);
        // The first actor: its container changes update with the player and
        // a fixed argument list; slot 0x460 gets the player and the constant;
        // it is then killed through slots 0x424 and `Actor::Kill`.
        assert_eq!(
            *updates.borrow(),
            vec![vec![0x4444, first, player, 0, 0, 1, 0, u32::MAX, 0]]
        );
        let constant = 10.0f32.to_bits();
        assert_eq!(slot_460.borrow()[0], vec![first, player, constant]);
        assert_eq!(
            slot_424.borrow()[0],
            vec![first, player, 0, 0, 0, 0, 0, 1, 0]
        );
        assert_eq!(kills.borrow()[0], vec![first, 0, 0.0f32.to_bits()]);
        // The essential actor is not killed, the temporary one not touched,
        // the locked one is unlocked.
        let killed: Vec<u32> = kills.borrow().iter().map(|k| k[0]).collect();
        assert_eq!(killed, vec![first, locked]);
        let touched: Vec<u32> = slot_460.borrow().iter().map(|k| k[0]).collect();
        assert_eq!(touched, vec![first, essential, locked]);
        assert_eq!(*unlocks.borrow(), vec![vec![locked]]);
        // The finishing call runs for every reference but the player and the
        // empty entry.
        assert_eq!(finishes.borrow().len(), 4);
        let order = sequence(&log);
        assert_eq!(order.first(), Some(&CELL_LOCK_ENTER));
        assert_eq!(order.last(), Some(&CELL_LOCK_LEAVE));
    }

    #[test]
    fn save_game_test_leaves_non_actors_and_the_player_alone() {
        let (mut e, c, player, updates, kills, _) = save_test_engine();
        let plain = object(&mut e);
        e.mem.set_u32(plain + 0x44, 0x5555);
        embedded_list(&mut e, c.addr() + 0xac, &[player, plain]);
        let slot_460 = record(&mut e, fake(0x460), 0);
        e.call(0x0054_b5b0, &args![c]);
        // The reference that is no actor still updates its container
        // changes, but no slot is called and nothing is killed.
        assert_eq!(updates.borrow().len(), 1);
        assert_eq!(updates.borrow()[0][1], plain);
        assert!(slot_460.borrow().is_empty());
        assert!(kills.borrow().is_empty());
    }

    #[test]
    fn the_function_table_covers_exactly_the_translated_functions() {
        let funcs = funcs();
        assert_eq!(funcs.len(), 80);
        let addresses: Vec<u32> = funcs.iter().map(|(a, _)| *a).collect();
        let mut sorted = addresses.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, addresses, "addresses are unique and in order");
        assert!(addresses
            .iter()
            .all(|a| (0x0054_7650..0x0055_2470).contains(a)));
        assert_eq!(addresses[0], 0x0054_7650);
        assert_eq!(*addresses.last().unwrap(), 0x0054_ee20);
    }

    // --- Second session: 0054b750 .. 0054bc10 -------------------------------

    /// A cell whose list of references holds `items`.
    fn cell_with(e: &mut Engine, items: &[u32]) -> Ptr<TESObjectCELL> {
        let c = cell(e);
        embedded_list(e, c.addr() + 0xac, items);
        c
    }

    /// A reference whose base form (word `+0x20`) is `base`.
    fn with_base(e: &mut Engine, base: u32) -> u32 {
        let r = object(e);
        e.mem.set_u32(r + 0x20, base);
        r
    }

    #[test]
    fn unload_references_resets_fade_nodes_and_clears_the_3d() {
        let mut e = engine();
        let player = object(&mut e);
        e.set_global(PLAYER_POINTER, player);
        let fade = object(&mut e);
        e.mem.set_f32(fade + 0xb4, 1.0);
        e.mem.set_f32(fade + 0xb8, 2.0);
        let node = object(&mut e);
        answer(&mut e, node, 0x10, fade);
        let with_3d = object(&mut e);
        e.mem.set_u32(with_3d + 0x64, node);
        let without_3d = object(&mut e);
        let c = cell_with(&mut e, &[player, 0, with_3d, without_3d]);
        let prepared = record(&mut e, CELL_PREPARE_UNLOAD, 0);
        let resets = record(&mut e, FADE_NODE_RESET, 0);
        let slot = record(&mut e, fake(0x1cc), 0);
        start_log(&mut e);
        e.call(0x0054_b750, &args![c]);
        let log = take_log(&mut e);
        assert_eq!(*prepared.borrow(), vec![vec![c.addr()]]);
        assert_eq!(*resets.borrow(), vec![vec![fade, 0]]);
        assert_eq!(e.mem.f32(fade + 0xb4), 0.0);
        assert_eq!(e.mem.f32(fade + 0xb8), 0.0);
        // The player is left alone; the other two lose their 3D.
        assert_eq!(
            *slot.borrow(),
            vec![vec![with_3d, 0, 0], vec![without_3d, 0, 0]]
        );
        let order = sequence(&log);
        assert_eq!(order.first(), Some(&CELL_LOCK_ENTER));
        assert_eq!(order.last(), Some(&CELL_LOCK_LEAVE));
    }

    #[test]
    fn fade_node_reset_clears_the_floats_and_calls_the_reset() {
        let mut e = engine();
        let node = object(&mut e);
        e.mem.set_f32(node + 0xb4, 3.0);
        e.mem.set_f32(node + 0xb8, 4.0);
        let calls = record(&mut e, FADE_NODE_RESET, 0);
        e.call(0x0054_b800, &args![node]);
        assert_eq!(e.mem.f32(node + 0xb4), 0.0);
        assert_eq!(e.mem.f32(node + 0xb8), 0.0);
        assert_eq!(*calls.borrow(), vec![vec![node, 0]]);
    }

    /// Marker collectors: a cell with a marker that has data, one without
    /// data, and an unrelated reference; returns the cell and the first.
    fn marker_cell(e: &mut Engine, form_global: u32, data: u32) -> (Ptr<TESObjectCELL>, u32) {
        let marker_base = form(e, 5, 0, 1);
        e.set_global(form_global, marker_base);
        let other_base = form(e, 5, 0, 2);
        e.register(data, |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        let hit = with_base(e, marker_base);
        e.mem.set_u32(hit + 0x30, 1);
        let no_data = with_base(e, marker_base);
        let other = with_base(e, other_base);
        e.mem.set_u32(other + 0x30, 1);
        (cell_with(e, &[hit, 0, no_data, other]), hit)
    }

    #[test]
    fn map_marker_collector_pushes_markers_with_data() {
        let mut e = engine();
        let (c, hit) = marker_cell(&mut e, MARKER_FORM_224, REFERENCE_GET_MAP_MARKER_DATA);
        let list = e.mem.alloc(8);
        e.call(0x0054_b830, &args![c, list]);
        assert_eq!(e.mem.u32(list), hit);
        assert_eq!(e.mem.u32(list + 4), 0, "only the marker with data");
        // A null list does nothing, not even take the lock.
        start_log(&mut e);
        e.call(0x0054_b830, &args![c, 0u32]);
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
    }

    #[test]
    fn second_marker_collector_uses_its_own_form_and_accessor() {
        let mut e = engine();
        let (c, hit) = marker_cell(&mut e, MARKER_FORM_228, REFERENCE_GET_MARKER_DATA_228);
        let list = e.mem.alloc(8);
        e.call(0x0054_b8c0, &args![c, list]);
        assert_eq!(e.mem.u32(list), hit);
        assert_eq!(e.mem.u32(list + 4), 0);
        start_log(&mut e);
        e.call(0x0054_b8c0, &args![c, 0u32]);
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
    }

    /// References for the update walks: `+0x30` bit 0 passes `0057a2f0`,
    /// `+0x34` is the answer of `0057a370`.
    fn update_cell(e: &mut Engine) -> (Ptr<TESObjectCELL>, [u32; 3]) {
        e.register(REFERENCE_TEST_57A2F0, |e, a| {
            u32::from(e.mem.u32(a[0] + 0x30) & 1 != 0).into_ret()
        });
        e.register(REFERENCE_TEST_57A370, |e, a| {
            e.mem.u32(a[0] + 0x34).into_ret()
        });
        quiet(e, &[DATA_HANDLER_FLAG_4226E0, GAME_LOADER_FLAG_244_2]);
        let passes = object(e);
        e.mem.set_u32(passes + 0x30, 1);
        let fails = object(e);
        let both = object(e);
        e.mem.set_u32(both + 0x30, 1);
        e.mem.set_u32(both + 0x34, 7);
        (
            cell_with(e, &[passes, 0, fails, both]),
            [passes, fails, both],
        )
    }

    #[test]
    fn update_walk_hands_references_to_the_update() {
        let mut e = engine();
        let (c, [passes, _, both]) = update_cell(&mut e);
        let updates = record(&mut e, REFERENCE_UPDATE_579AC0, 0);
        e.call(0x0054_b950, &args![c, 0u32]);
        assert_eq!(*updates.borrow(), vec![vec![passes, 0], vec![both, 0]]);
        updates.borrow_mut().clear();
        // With the flag the second test has to pass too.
        e.call(0x0054_b950, &args![c, 1u32]);
        assert_eq!(*updates.borrow(), vec![vec![both, 1]]);
    }

    #[test]
    fn update_walk_does_nothing_while_loading() {
        let mut e = engine();
        let (c, _) = update_cell(&mut e);
        let updates = record(&mut e, REFERENCE_UPDATE_579AC0, 0);
        returns(&mut e, DATA_HANDLER_FLAG_4226E0, 1);
        start_log(&mut e);
        e.call(0x0054_b950, &args![c, 0u32]);
        assert_eq!(sequence(&take_log(&mut e)), vec![DATA_HANDLER_FLAG_4226E0]);
        returns(&mut e, DATA_HANDLER_FLAG_4226E0, 0);
        returns(&mut e, GAME_LOADER_FLAG_244_2, 1);
        e.call(0x0054_b950, &args![c, 0u32]);
        assert!(updates.borrow().is_empty());
    }

    #[test]
    fn second_update_walk_passes_the_answer_of_the_second_test() {
        let mut e = engine();
        let (c, [passes, _, both]) = update_cell(&mut e);
        let updates = record(&mut e, REFERENCE_UPDATE_579AC0, 0);
        e.call(0x0054_ba00, &args![c]);
        assert_eq!(*updates.borrow(), vec![vec![passes, 0], vec![both, 7]]);
    }

    #[test]
    fn attach_lights_adds_or_removes_the_light() {
        let mut e = engine();
        let a = object(&mut e);
        let b = object(&mut e);
        let c = cell_with(&mut e, &[a, 0, b]);
        let adds = record(&mut e, REFERENCE_ADD_LIGHT_TO_SCENE, 0);
        let removes = record(&mut e, REFERENCE_REMOVE_LIGHT, 0);
        e.call(0x0054_ba80, &args![c, 1u32]);
        assert_eq!(*adds.borrow(), vec![vec![a, 0], vec![b, 0]]);
        assert!(removes.borrow().is_empty());
        adds.borrow_mut().clear();
        e.call(0x0054_ba80, &args![c, 0u32]);
        assert!(adds.borrow().is_empty());
        assert_eq!(*removes.borrow(), vec![vec![a, 0], vec![b, 0]]);
    }

    #[test]
    fn type_1e_references_apply_their_extra_entry() {
        let mut e = engine();
        let kind_1e = form(&mut e, 0x1e, 0, 1);
        let other = form(&mut e, 0x10, 0, 2);
        let with_entry = with_base(&mut e, kind_1e);
        e.mem.set_u32(with_entry + 0x44, 0x5151);
        let without_entry = with_base(&mut e, kind_1e);
        let unrelated = with_base(&mut e, other);
        e.mem.set_u32(unrelated + 0x44, 0x6262);
        // The entry accessor reads the word the list holds.
        e.register(EXTRA_LIST_GET_ENTRY_29, |e, a| e.mem.u32(a[0]).into_ret());
        let c = cell_with(&mut e, &[with_entry, 0, without_entry, unrelated]);
        let applied = record(&mut e, BASE_FORM_APPLY_ENTRY, 0);
        e.call(0x0054_baf0, &args![c]);
        assert_eq!(*applied.borrow(), vec![vec![kind_1e, 0x5151, 0]]);
    }

    #[test]
    fn counting_type_1e_references() {
        let mut e = engine();
        let kind_1e = form(&mut e, 0x1e, 0, 1);
        let other = form(&mut e, 0x10, 0, 2);
        let a = with_base(&mut e, kind_1e);
        let b = with_base(&mut e, other);
        let d = with_base(&mut e, kind_1e);
        let c = cell_with(&mut e, &[a, b, 0, d]);
        assert_eq!(e.call(0x0054_bb90, &args![c]).u32(), 2);
        let empty = cell_with(&mut e, &[0]);
        assert_eq!(e.call(0x0054_bb90, &args![empty]).u32(), 0);
    }

    #[test]
    fn tree_hiding_needs_the_flag_a_3d_and_a_world_space() {
        let mut e = engine();
        e.register(REFERENCE_PREPARE_56F700, |_, _| Ret::default());
        e.register(WORLD_SPACE_GET_TERRAIN_MANAGER, |_, a| {
            (a[0] + 0x1000).into_ret()
        });
        let hides = record(&mut e, TERRAIN_MANAGER_HIDE_TREE, 0);
        let flags = record(&mut e, TERRAIN_MANAGER_SET_FLAG_28, 0);
        let handled = record(&mut e, PROCESS_LISTS_HANDLE_CELL_REFERENCES, 0);
        let tree_base = form(&mut e, 5, 0x40, 1);
        let plain_base = form(&mut e, 5, 0, 2);
        let tree = with_base(&mut e, tree_base);
        answer(&mut e, tree, 0x1d0, 0x9999);
        let no_3d = with_base(&mut e, tree_base);
        let plain = with_base(&mut e, plain_base);
        answer(&mut e, plain, 0x1d0, 0x9999);
        let c = cell_with(&mut e, &[tree, no_3d, plain]);
        let world = object(&mut e);
        e.mem.set_u32(c.addr() + 0xc0, world);
        e.call(0x0054_bc10, &args![c]);
        assert_eq!(
            *handled.borrow(),
            vec![vec![PROCESS_LISTS, c.addr() + 0xac, 0]]
        );
        assert_eq!(*hides.borrow(), vec![vec![world + 0x1000, tree, 1]]);
        assert_eq!(*flags.borrow(), vec![vec![world + 0x1000, 1]]);
        // An interior cell has no world space: nothing is hidden.
        hides.borrow_mut().clear();
        e.mem.set_u8(c.addr() + 0x24, 1);
        e.call(0x0054_bc10, &args![c]);
        assert!(hides.borrow().is_empty());
    }

    // --- Second session: 0054bcf0 and 0054c030 -------------------------------

    #[test]
    fn cell_load_moves_the_3d_to_the_origin_when_tes_accepts_the_cell() {
        let mut e = engine();
        // `fn_0054df30` stops at once while the game loader flag is set.
        returns(&mut e, GAME_LOADER_FLAG_244_2, 1);
        returns(&mut e, CELL_LOAD_3D, 0x8000);
        returns(&mut e, TES_CELL_TEST_453490, 1);
        e.set_global(TES_POINTER, 0x7000u32);
        let after_scripts = record(&mut e, CELL_AFTER_SCRIPTS, 0);
        let vectors = record(&mut e, VECTOR3_CONSTRUCT_43D410, 0);
        let translates = record(&mut e, NODE_SET_LOCAL_TRANSLATE, 0);
        let after_attach = record(&mut e, CELL_AFTER_ATTACH, 0);
        let states = record(&mut e, CELL_SET_STATE, 0);
        let handled = record(&mut e, PROCESS_LISTS_HANDLE_CELL_REFERENCES, 0);
        let c = cell_with(&mut e, &[]);
        start_log(&mut e);
        e.call(0x0054_bcf0, &args![c, 0u32]);
        let log = take_log(&mut e);
        assert_eq!(*after_scripts.borrow(), vec![vec![c.addr(), 0]]);
        assert_eq!(
            calls_to(&log, TES_CELL_TEST_453490),
            vec![vec![0x7000, c.addr()]]
        );
        assert_eq!(vectors.borrow().len(), 1);
        assert_eq!(vectors.borrow()[0][1..], [0, 0, 0]);
        let vector = vectors.borrow()[0][0];
        assert_eq!(*translates.borrow(), vec![vec![0x8000, vector]]);
        assert_eq!(
            *handled.borrow(),
            vec![vec![PROCESS_LISTS, c.addr() + 0xac, 0]]
        );
        assert!(after_attach.borrow().is_empty());
        assert_eq!(*states.borrow(), vec![vec![c.addr(), 6]]);
        // With the flag the extra step follows.
        e.call(0x0054_bcf0, &args![c, 1u32]);
        assert_eq!(*after_attach.borrow(), vec![vec![c.addr()]]);
    }

    #[test]
    fn cell_load_queues_every_reference_in_two_passes() {
        let mut e = engine();
        // The first loader test belongs to `fn_0054df30`; the rest say no.
        let asked = Rc::new(RefCell::new(0u32));
        let seen = asked.clone();
        e.register_double(GAME_LOADER_FLAG_244_2, move |_, _| {
            *seen.borrow_mut() += 1;
            u32::from(*seen.borrow() == 1).into_ret()
        });
        returns(&mut e, TES_CELL_TEST_453490, 0);
        quiet(
            &mut e,
            &[CELL_AFTER_SCRIPTS, CELL_AFTER_ATTACH, CELL_LOAD_3D],
        );
        returns(&mut e, TES_GET_CELL_PRIORITY, 0x33);
        returns(&mut e, SAVE_GAME_FLAG, 0);
        e.set_global(TES_POINTER, 0x7000u32);
        e.set_global(MODEL_LOADER_POINTER, 0x7100u32);
        let marker_base = form(&mut e, 5, 0, 1);
        e.set_global(ROOM_MARKER_FORM_238, marker_base);
        let plain_base = form(&mut e, 5, 0, 2);
        let marker = with_base(&mut e, marker_base);
        let actor = with_base(&mut e, plain_base);
        answer(&mut e, actor, 0x100, 1);
        let other = with_base(&mut e, plain_base);
        answer(&mut e, other, 0x224, 1);
        answer(&mut e, other, 0x1d0, 0x9999);
        let c = cell_with(&mut e, &[marker, actor, other]);
        let queued = record(&mut e, MODEL_LOADER_QUEUE_REFERENCE, 0);
        let inits = record(&mut e, SCRIPT_INIT_ACTION_LIST, 0);
        let flags = record(&mut e, SCRIPT_SET_ACTION_FLAG, 0);
        let slot_240 = record(&mut e, fake(0x240), 0);
        let slot_260 = record(&mut e, fake(0x260), 0);
        let removed = record(&mut e, PROCESS_LISTS_REMOVE_ACTOR_FROM_TEMP_CHANGE_LIST, 0);
        let states = record(&mut e, CELL_SET_STATE, 0);
        start_log(&mut e);
        e.call(0x0054_bcf0, &args![c, 0u32]);
        let log = take_log(&mut e);
        // Markers go in the first pass, the rest in the second; the last
        // reference of each pass is judged with the next pass's number, so
        // `other` is handled twice.
        let order: Vec<u32> = queued.borrow().iter().map(|a| a[1]).collect();
        assert_eq!(order, vec![marker, other, actor, other]);
        assert!(queued
            .borrow()
            .iter()
            .all(|a| a[0] == 0x7100 && a[2] == 0x33 && a[3] == 0));
        let initialised: Vec<u32> = inits.borrow().iter().map(|a| a[0]).collect();
        assert_eq!(initialised, order);
        assert_eq!(flags.borrow()[0][0], marker);
        assert!(flags.borrow().iter().all(|a| a[2] == 0x1000));
        // The actor is cast and dropped from the temporary change list; the
        // plain reference with the slot answer is handed slot 0x260.
        assert_eq!(*slot_240.borrow(), vec![vec![actor]]);
        assert_eq!(*removed.borrow(), vec![vec![PROCESS_LISTS, actor]]);
        assert_eq!(*slot_260.borrow(), vec![vec![other], vec![other]]);
        // `other` has a 3D, so the attach looks at it each time.
        assert_eq!(
            calls_to(&log, REFERENCE_GET_3D),
            vec![vec![other], vec![other]]
        );
        assert_eq!(*states.borrow(), vec![vec![c.addr(), 6]]);
    }

    #[test]
    fn cell_load_skips_flagged_references_and_forms_that_tes_refuses() {
        let mut e = engine();
        returns(&mut e, GAME_LOADER_FLAG_244_2, 1);
        returns(&mut e, TES_CELL_TEST_453490, 0);
        quiet(
            &mut e,
            &[
                CELL_AFTER_SCRIPTS,
                CELL_AFTER_ATTACH,
                CELL_SET_STATE,
                CELL_LOAD_3D,
            ],
        );
        returns(&mut e, TES_GET_CELL_PRIORITY, 0);
        e.set_global(TES_POINTER, 0x7000u32);
        e.set_global(MODEL_LOADER_POINTER, 0x7100u32);
        // The test of the type 0x23 form reads the word at +0x30.
        e.register(FORM_TEST_452440, |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        let accepted_base = form(&mut e, 0x23, 0, 1);
        e.mem.set_u32(accepted_base + 0x30, 1);
        let refused_base = form(&mut e, 0x23, 0, 2);
        let flagged_base = form(&mut e, 5, 0, 3);
        let flagged = with_base(&mut e, flagged_base);
        e.mem.set_u32(flagged + 8, 0x800);
        let accepted = with_base(&mut e, accepted_base);
        answer(&mut e, accepted, 0x1d0, 0x9999);
        let refused = with_base(&mut e, refused_base);
        answer(&mut e, refused, 0x1d0, 0x9999);
        let c = cell_with(&mut e, &[flagged, accepted, refused]);
        let queued = record(&mut e, MODEL_LOADER_QUEUE_REFERENCE, 0);
        start_log(&mut e);
        e.call(0x0054_bcf0, &args![c, 0u32]);
        let log = take_log(&mut e);
        let order: Vec<u32> = queued.borrow().iter().map(|a| a[1]).collect();
        // The game loader flag is set: no action lists. The flagged reference
        // is never queued; the refused one is, twice, and attached twice.
        assert_eq!(order, vec![refused, accepted, refused]);
        assert!(calls_to(&log, SCRIPT_INIT_ACTION_LIST).is_empty());
        assert_eq!(
            calls_to(&log, REFERENCE_GET_3D),
            vec![vec![refused], vec![refused]]
        );
    }

    /// Doubles for the vector helpers of `CalcRefCenterPoint`.
    fn vector_engine() -> Engine {
        let mut e = engine();
        e.register(VECTOR3_CONSTRUCT, |e, a| {
            for i in 1..4 {
                e.mem.set_u32(a[0] + 4 * (i as u32 - 1), a[i]);
            }
            a[0].into_ret()
        });
        fn combine(e: &mut Engine, a: &[u32], f: fn(f32, f32) -> f32) -> Ret {
            for i in 0..3u32 {
                let value = f(e.mem.f32(a[0] + 4 * i), e.mem.f32(a[2] + 4 * i));
                e.mem.set_f32(a[1] + 4 * i, value);
            }
            a[1].into_ret()
        }
        e.register(VECTOR_ADD, |e, a| combine(e, a, |x, y| x + y));
        e.register(VECTOR_SUBTRACT, |e, a| combine(e, a, |x, y| x - y));
        e.register(VECTOR_DIVIDE, |e, a| {
            let divisor = f32::from_bits(a[2]);
            for i in 0..3u32 {
                let value = e.mem.f32(a[0] + 4 * i) / divisor;
                e.mem.set_f32(a[1] + 4 * i, value);
            }
            a[1].into_ret()
        });
        e.register(FLOAT_SMALLER, |_, a| {
            f32::from_bits(a[0]).min(f32::from_bits(a[1])).into_ret()
        });
        e.register(FLOAT_LARGER, |_, a| {
            f32::from_bits(a[0]).max(f32::from_bits(a[1])).into_ret()
        });
        e.register(FLOAT_ABS, |_, a| f32::from_bits(a[0]).abs().into_ret());
        e.register(WORLD_SCALE, |_, a| f32::from_bits(a[0]).into_ret());
        e.set_global(FLOAT_MAX, f32::MAX);
        e.set_global(TWO, 2.0f32);
        e.set_global(EXTENT_LIMIT, 1000.0f32);
        e.set_global(SMALLEST_NORMAL_FLOAT, f32::MIN_POSITIVE);
        e.set_global(POSITION_LIMIT, 30000.0f64);
        for (i, v) in [9.0f32, 9.0, 9.0].iter().enumerate() {
            e.set_global(DEFAULT_VECTOR + 4 * i as u32, *v);
        }
        e
    }

    /// A reference of base type `kind` at `(x, y, z)`.
    fn placed(e: &mut Engine, kind: u8, x: f32, y: f32, z: f32) -> u32 {
        let base = form(e, kind, 0, 1);
        placed_on(e, base, x, y, z)
    }

    /// A reference on the base form `base` at `(x, y, z)`.
    fn placed_on(e: &mut Engine, base: u32, x: f32, y: f32, z: f32) -> u32 {
        let r = with_base(e, base);
        let position = e.mem.alloc(12);
        e.mem.set_f32(position, x);
        e.mem.set_f32(position + 4, y);
        e.mem.set_f32(position + 8, z);
        answer(e, r, 0x1f4, position);
        r
    }

    fn center_of(e: &mut Engine, c: Ptr<TESObjectCELL>) -> [f32; 3] {
        let out = e.mem.alloc(12);
        e.call(0x0054_c030, &args![c, out]);
        [e.mem.f32(out), e.mem.f32(out + 4), e.mem.f32(out + 8)]
    }

    #[test]
    fn center_point_is_the_middle_of_the_box() {
        let mut e = vector_engine();
        let a = placed(&mut e, 0x10, 0.0, 0.0, 0.0);
        let b = placed(&mut e, 0x10, 100.0, 200.0, 300.0);
        let c = cell_with(&mut e, &[a, 0, b]);
        assert_eq!(center_of(&mut e, c), [50.0, 100.0, 150.0]);
    }

    #[test]
    fn center_point_leaves_out_the_excluded_types() {
        let mut e = vector_engine();
        // Types 0x1E and 0x2A..=0x2D do not count; 0x29 and 0x2E do.
        let skipped: Vec<u32> = [0x1e, 0x2a, 0x2d]
            .iter()
            .map(|k| placed(&mut e, *k, 5000.0, 5000.0, 5000.0))
            .collect();
        let c = cell_with(&mut e, &skipped);
        assert_eq!(center_of(&mut e, c), [9.0, 9.0, 9.0], "the default vector");
        let counted_a = placed(&mut e, 0x29, 10.0, 10.0, 10.0);
        let counted_b = placed(&mut e, 0x2e, 30.0, 30.0, 30.0);
        let mut all = skipped.clone();
        all.extend([counted_a, counted_b]);
        let c = cell_with(&mut e, &all);
        assert_eq!(center_of(&mut e, c), [20.0, 20.0, 20.0]);
    }

    #[test]
    fn center_point_reports_a_box_that_is_too_large() {
        let mut e = vector_engine();
        let prints = record(&mut e, DEBUG_PRINT, 0);
        let refs: Vec<u32> = [0.0f32, 900.0, 1200.0]
            .iter()
            .map(|x| placed(&mut e, 0x10, *x, 0.0, 0.0))
            .collect();
        let c = cell_with(&mut e, &refs);
        answer(&mut e, c.addr(), 0x130, 0xabc);
        e.mem.set_u32(c.addr() + 0xc, 0x77);
        let center = center_of(&mut e, c);
        assert_eq!(center[0], 600.0);
        let tiny = f64::from(f32::MIN_POSITIVE);
        assert_eq!(
            *prints.borrow(),
            vec![args![
                MESSAGE_INTERIOR_TOO_LARGE,
                0xabcu32,
                0x77u32,
                1200.0f64,
                tiny,
                tiny,
                1000.0f64
            ]]
        );
    }

    #[test]
    fn center_point_moves_a_reference_that_is_far_away() {
        let mut e = vector_engine();
        let prints = record(&mut e, DEBUG_PRINT, 0);
        // The placement the cell offers: grid (3, 4), the setting says
        // "do not refine" (a non-zero first byte), exterior cell.
        returns(&mut e, CELL_GET_DATA_X, 3);
        returns(&mut e, CELL_GET_DATA_Y, 4);
        let setting = e.mem.alloc(4);
        e.mem.set_u8(setting, 1);
        returns(&mut e, SETTING_TEXT, setting);
        let start = e.mem.alloc(12);
        let starts = record(&mut e, fake(0x170), start);
        let locations = Rc::new(RefCell::new(Vec::new()));
        let seen = locations.clone();
        e.register_double(REFERENCE_SET_LOCATION, move |e, a| {
            let at = (e.mem.f32(a[1]), e.mem.f32(a[1] + 4), e.mem.f32(a[1] + 8));
            seen.borrow_mut().push((a[0], a[1], at));
            Ret::default()
        });
        let far = placed(&mut e, 0x10, 40000.0, 0.0, 0.0);
        e.mem.set_u32(far + 0xc, 0x55);
        answer(&mut e, far, 0x130, 0xdef);
        // The extra data entry (type 0xF) is there: first back to the start
        // position, which is still far, then to the safe place.
        returns(&mut e, EXTRA_LIST_GET_EXTRA_DATA, 1);
        let c = cell_with(&mut e, &[far]);
        assert_eq!(center_of(&mut e, c), [9.0, 9.0, 9.0], "it left the box out");
        assert_eq!(
            *prints.borrow(),
            vec![
                args![MESSAGE_RETURN_TO_START, 0xdefu32, 0x55u32],
                args![MESSAGE_MOVE_TO_SAFE_PLACE, 0xdefu32, 0x55u32]
            ]
        );
        assert_eq!(starts.borrow().len(), 1);
        {
            let seen = locations.borrow();
            assert_eq!(seen.len(), 2);
            assert_eq!((seen[0].0, seen[0].1), (far, start));
            assert_eq!((seen[1].0, seen[1].2), (far, (14336.0, 18432.0, 0.0)));
        }

        // Without the extra data entry only the second message appears.
        prints.borrow_mut().clear();
        returns(&mut e, EXTRA_LIST_GET_EXTRA_DATA, 0);
        center_of(&mut e, c);
        assert_eq!(
            *prints.borrow(),
            vec![args![MESSAGE_MOVE_TO_SAFE_PLACE, 0xdefu32, 0x55u32]]
        );
    }

    // --- Second session: 0054c740 .. 0054cf20 --------------------------------

    /// Doubles that really change a list: `00905330(node, &item)` unlinks the
    /// node holding `item` (searching from `node`), `0063f7b0(node)` drops
    /// `node` by copying the next one into it.
    fn list_edit_engine() -> Engine {
        let mut e = engine();
        e.register(LIST_POP_FRONT, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next == 0 {
                e.mem.set_u32(a[0], 0);
            } else {
                let (item, after) = (e.mem.u32(next), e.mem.u32(next + 4));
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(a[0] + 4, after);
            }
            Ret::default()
        });
        e.register(LIST_REMOVE_ITEM, |e, a| {
            let target = e.mem.u32(a[1]);
            let mut previous = 0;
            let mut node = a[0];
            while node != 0 {
                if e.mem.u32(node) == target {
                    let next = e.mem.u32(node + 4);
                    if previous == 0 {
                        let item = if next == 0 { 0 } else { e.mem.u32(next) };
                        let after = if next == 0 { 0 } else { e.mem.u32(next + 4) };
                        e.mem.set_u32(node, item);
                        e.mem.set_u32(node + 4, after);
                    } else {
                        e.mem.set_u32(previous + 4, next);
                    }
                    break;
                }
                previous = node;
                node = e.mem.u32(node + 4);
            }
            Ret::default()
        });
        e
    }

    /// The items of the list that starts at `head`.
    fn list_items(e: &Engine, head: u32) -> Vec<u32> {
        let mut items = vec![];
        let mut node = head;
        while node != 0 && !(e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0) {
            items.push(e.mem.u32(node));
            node = e.mem.u32(node + 4);
        }
        items
    }

    #[test]
    fn run_scripts_stops_at_the_first_script_that_ran() {
        let mut e = engine();
        let ran = Rc::new(RefCell::new(Vec::new()));
        let seen = ran.clone();
        e.register_double(REFERENCE_RUN_SCRIPT, move |e, a| {
            seen.borrow_mut().push(a[0]);
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        returns(&mut e, INTERFACE_IS_IN_MENU_MODE, 1);
        let no_3d = object(&mut e);
        let flagged = object(&mut e);
        e.mem.set_u32(flagged + 8, 0x800);
        let silent = object(&mut e);
        e.mem.set_u32(silent + 0x64, 1);
        let runs = object(&mut e);
        e.mem.set_u32(runs + 0x64, 1);
        e.mem.set_u32(runs + 0x30, 1);
        let later = object(&mut e);
        e.mem.set_u32(later + 0x64, 1);
        e.mem.set_u32(later + 0x30, 1);
        let loaded = e.mem.alloc(0x80);
        embedded_list(
            &mut e,
            loaded + 0x4c,
            &[no_3d, flagged, silent, runs, later],
        );
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0xc4, loaded);
        start_log(&mut e);
        assert_eq!(e.call(0x0054_c740, &args![c, 0u32, 0u32]).u8(), 1);
        let log = take_log(&mut e);
        // The reference without 3D or flag is not even tried.
        assert_eq!(*ran.borrow(), vec![flagged, silent, runs]);
        // The reference list is the loaded data's, so the cell's is not read.
        assert!(calls_to(&log, CELL_REFERENCE_LIST).is_empty());
        assert_eq!(sequence(&log).last(), Some(&CELL_LOCK_LEAVE));
    }

    #[test]
    fn run_scripts_of_all_references_never_answers_yes() {
        let mut e = engine();
        let ran = Rc::new(RefCell::new(Vec::new()));
        let seen = ran.clone();
        e.register_double(REFERENCE_RUN_SCRIPT, move |_, a| {
            seen.borrow_mut().push(a[0]);
            1u32.into_ret()
        });
        let x = object(&mut e);
        let y = object(&mut e);
        let c = cell_with(&mut e, &[x, 0, y]);
        assert_eq!(e.call(0x0054_c740, &args![c, 1u32, 0u32]).u8(), 0);
        assert_eq!(*ran.borrow(), vec![x, y]);
        // Without the flag and without loaded data nothing is walked.
        ran.borrow_mut().clear();
        assert_eq!(e.call(0x0054_c740, &args![c, 0u32, 0u32]).u8(), 0);
        assert!(ran.borrow().is_empty());
    }

    #[test]
    fn run_scripts_drops_children_that_no_longer_activate() {
        let mut e = list_edit_engine();
        e.register(REFERENCE_UPDATE_CHILD_ACTIVATES, |e, a| {
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        quiet(&mut e, &[INTERFACE_IS_IN_MENU_MODE]);
        let items: Vec<u32> = [0u32, 1, 0, 1]
            .iter()
            .map(|keep| {
                let r = object(&mut e);
                e.mem.set_u32(r + 0x30, *keep);
                r
            })
            .collect();
        let loaded = e.mem.alloc(0x80);
        embedded_list(&mut e, loaded + 0x54, &items);
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0xc4, loaded);
        assert_eq!(e.call(0x0054_c740, &args![c, 0u32, 0u32]).u8(), 0);
        // The first (at the head) and the third are gone.
        assert_eq!(list_items(&e, loaded + 0x54), vec![items[1], items[3]]);
        // While the menus are open the list is left alone.
        let items: Vec<u32> = items.iter().map(|_| object(&mut e)).collect();
        embedded_list(&mut e, loaded + 0x54, &items);
        returns(&mut e, INTERFACE_IS_IN_MENU_MODE, 1);
        e.call(0x0054_c740, &args![c, 0u32, 0u32]);
        assert_eq!(list_items(&e, loaded + 0x54), items);
    }

    #[test]
    fn persistent_references_go_to_the_exterior_cell_of_their_position() {
        let mut e = engine();
        e.register(FLOAT_TO_INT, |_, a| {
            (f32::from_bits(a[0]).round() as i32 as u32).into_ret()
        });
        let found = object(&mut e);
        e.register_double(WORLD_SPACE_GET_CELL_FROM_COORD, move |_, a| {
            if a[1] == 2 && a[2] == 3 {
                found.into_ret()
            } else {
                0u32.into_ret()
            }
        });
        let prints = record(&mut e, DEBUG_PRINT, 0);
        let world = object(&mut e);
        answer(&mut e, world, 0x130, 0x111);
        let at = |e: &mut Engine, x: f32, y: f32, id: u32, name: u32| {
            let r = object(e);
            let position = e.mem.alloc(12);
            e.mem.set_f32(position, x);
            e.mem.set_f32(position + 4, y);
            answer(e, r, 0x1f4, position);
            answer(e, r, 0x130, name);
            e.mem.set_u32(r + 0xc, id);
            r
        };
        let inside = at(&mut e, 8200.0, 12300.0, 1, 0x211);
        let west = at(&mut e, -5000.0, 100.0, 2, 0x222);
        let c = cell_with(&mut e, &[inside, west]);
        e.mem.set_u32(c.addr() + 8, 0x400);
        e.mem.set_u32(c.addr() + 0xc, 0x99);
        start_log(&mut e);
        e.call(0x0054_c8c0, &args![c, world]);
        let log = take_log(&mut e);
        // `AddReference` of the found cell starts by asking for the base form.
        assert_eq!(calls_to(&log, REFERENCE_GET_BASE_FORM), vec![vec![inside]]);
        // (-5000 >> 12) = -2: no such cell.
        assert_eq!(
            *prints.borrow(),
            vec![args![
                MESSAGE_CELL_NOT_FOUND,
                -2i32,
                0u32,
                0x111u32,
                0x99u32,
                0x222u32,
                2u32
            ]]
        );
        // No world space, or a cell that is not persistent: nothing at all.
        start_log(&mut e);
        e.call(0x0054_c8c0, &args![c, 0u32]);
        e.mem.set_u32(c.addr() + 8, 0);
        e.call(0x0054_c8c0, &args![c, world]);
        assert!(!sequence(&take_log(&mut e)).contains(&CELL_LOCK_ENTER));
    }

    #[test]
    fn furniture_is_added_unless_flagged() {
        let mut e = engine();
        e.register(REFERENCE_IS_FURNITURE, |e, a| {
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        let make = |e: &mut Engine, furniture: u32, flags: u32| {
            let r = object(e);
            e.mem.set_u32(r + 0x30, furniture);
            e.mem.set_u32(r + 8, flags);
            r
        };
        let chair = make(&mut e, 1, 0);
        let table = make(&mut e, 0, 0);
        let disabled = make(&mut e, 1, 0x20);
        let deleted = make(&mut e, 1, 0x800);
        let c = cell_with(&mut e, &[chair, table, 0, disabled, deleted]);
        let list = e.mem.alloc(8);
        e.call(0x0054_ca00, &args![c, list]);
        assert_eq!(list_items(&e, list), vec![chair]);
        start_log(&mut e);
        e.call(0x0054_ca00, &args![c, 0u32]);
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
    }

    /// Doubles for `RemoveReference`: `+0x30` bit 0 of a reference is "has
    /// visible distant", `+0x34` its encounter zone, `+0x38` scripted, `+0x3C`
    /// activating children; the cell's zone is the word at `+0x30`.
    fn removal_engine() -> Engine {
        let mut e = engine();
        e.register(REFERENCE_HAS_VISIBLE_DISTANT, |e, a| {
            u32::from(e.mem.u32(a[0] + 0x30) & 1 != 0).into_ret()
        });
        e.register(REFERENCE_GET_ENCOUNTER_ZONE, |e, a| {
            e.mem.u32(a[0] + 0x34).into_ret()
        });
        e.register(REFERENCE_IS_SCRIPTED, |e, a| {
            e.mem.u32(a[0] + 0x38).into_ret()
        });
        e.register(REFERENCE_IS_ACTIVATING_CHILDREN, |e, a| {
            e.mem.u32(a[0] + 0x3c).into_ret()
        });
        e.register(CELL_ENCOUNTER_ZONE, |e, a| {
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        e.register(DATA_HANDLER_LOADING_FLAG, |_, _| 0u32.into_ret());
        e.register(REFERENCE_GET_REF_PERSISTS, |e, a| {
            e.mem.u32(a[0] + 0x40).into_ret()
        });
        returns(&mut e, DEFAULT_ENCOUNTER_ZONE, 0x1111);
        e.set_global(OCCLUSION_MARKER_FORM_234, 0x7234u32);
        quiet(
            &mut e,
            &[
                DATA_HANDLER_FLAG_4226E0,
                GAME_LOADER_FLAG_4121B0,
                FORM_FLAG_4000,
                REFERENCE_SET_ENCOUNTER_ZONE,
            ],
        );
        e
    }

    #[test]
    fn removal_takes_the_reference_out_and_lets_it_leave() {
        let mut e = removal_engine();
        let zones = record(&mut e, REFERENCE_SET_ENCOUNTER_ZONE, 0);
        let removed = Rc::new(RefCell::new(Vec::new()));
        let seen = removed.clone();
        e.register_double(LIST_REMOVE_ITEM, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        let leaves = record(&mut e, fake(0x228), 0);
        let loaded_slot = record(&mut e, fake(0xc8), 0);
        let c = cell(&mut e);
        e.mem.set_i16(c.addr() + 0xa8, 5);
        let r = object(&mut e);
        e.mem.set_u32(r + 0x30, 1);
        start_log(&mut e);
        e.call(0x0054_ca90, &args![c, r]);
        let log = take_log(&mut e);
        // Counted out of the visible-distant total, given the default zone.
        assert_eq!(e.mem.i16(c.addr() + 0xa8), 4);
        assert_eq!(*zones.borrow(), vec![vec![r, 0x1111]]);
        assert_eq!(*removed.borrow(), vec![(c.addr() + 0xac, r)]);
        assert_eq!(*leaves.borrow(), vec![vec![r, 0]]);
        assert_eq!(*loaded_slot.borrow(), vec![vec![c.addr(), 1]]);
        let order = sequence(&log);
        let lock = order.iter().position(|a| *a == CELL_LOCK_ENTER).unwrap();
        let unlock = order.iter().position(|a| *a == CELL_LOCK_LEAVE).unwrap();
        assert!(lock < unlock);
        // A persistent reference does not make the cell call slot 0xC8.
        loaded_slot.borrow_mut().clear();
        e.mem.set_u32(r + 0x40, 1);
        e.call(0x0054_ca90, &args![c, r]);
        assert!(loaded_slot.borrow().is_empty());
        // A null reference does nothing.
        start_log(&mut e);
        e.call(0x0054_ca90, &args![c, 0u32]);
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
    }

    #[test]
    fn removal_undoes_the_loaded_data_bookkeeping() {
        let mut e = removal_engine();
        let zones = record(&mut e, REFERENCE_SET_ENCOUNTER_ZONE, 0);
        quiet(&mut e, &[LIST_REMOVE_ITEM]);
        let state_6 = record(&mut e, STATE_6_REMOVE_REFERENCE, 0);
        let scripted = record(&mut e, CELL_REMOVE_SCRIPTED_REF, 0);
        let activating = record(&mut e, CELL_REMOVE_ACTIVATING_REF, 0);
        let emittance = record(&mut e, CELL_REMOVE_EMITTANCE_REF, 0);
        let multi_bound = record(&mut e, CELL_REMOVE_MULTI_BOUND_REF, 0);
        e.set_global(STATE_6_OBJECT_POINTER, 0x7600u32);
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 0x30, 0x2222);
        e.mem.set_u8(c.addr() + 0x26, 6);
        e.mem.set_u32(c.addr() + 0xc4, 0x1234);
        let r = object(&mut e);
        e.mem.set_u32(r + 0x38, 1);
        e.mem.set_u32(r + 0x3c, 1);
        e.call(0x0054_ca90, &args![c, r]);
        // The reference takes the cell's own zone.
        assert_eq!(*zones.borrow(), vec![vec![r, 0x2222]]);
        assert_eq!(*state_6.borrow(), vec![vec![0x7600, r]]);
        for calls in [&scripted, &activating, &emittance, &multi_bound] {
            assert_eq!(*calls.borrow(), vec![vec![c.addr(), r]]);
        }
        // A plain reference in a state-5 cell: only the state object and the
        // two unconditional calls.
        scripted.borrow_mut().clear();
        activating.borrow_mut().clear();
        state_6.borrow_mut().clear();
        e.mem.set_u8(c.addr() + 0x26, 5);
        let plain = object(&mut e);
        e.call(0x0054_ca90, &args![c, plain]);
        assert_eq!(*state_6.borrow(), vec![vec![0x7600, plain]]);
        assert!(scripted.borrow().is_empty());
        assert!(activating.borrow().is_empty());
        assert_eq!(emittance.borrow().len(), 2);
    }

    #[test]
    fn removal_from_a_persistent_cell_clears_the_persistent_cell() {
        let mut e = removal_engine();
        quiet(&mut e, &[LIST_REMOVE_ITEM]);
        let persistent_cells = record(&mut e, EXTRA_LIST_SET_PERSISTENT_CELL, 0);
        let world_data = record(&mut e, WORLD_SPACE_REMOVE_FROM_PERSISTENT_REF_DATA, 0);
        let leaves = record(&mut e, fake(0x228), 0);
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 8, 0x400);
        let world = object(&mut e);
        e.mem.set_u32(c.addr() + 0xc0, world);
        let r = object(&mut e);
        e.call(0x0054_ca90, &args![c, r]);
        assert_eq!(*persistent_cells.borrow(), vec![vec![r + 0x44, 0]]);
        assert_eq!(*world_data.borrow(), vec![vec![world, r]]);
        assert!(leaves.borrow().is_empty());
        // An interior cell has no world space to tell.
        world_data.borrow_mut().clear();
        e.mem.set_u8(c.addr() + 0x24, 1);
        e.call(0x0054_ca90, &args![c, r]);
        assert!(world_data.borrow().is_empty());
    }

    #[test]
    fn removal_clears_the_links_of_occlusion_planes() {
        let mut e = removal_engine();
        quiet(&mut e, &[LIST_REMOVE_ITEM]);
        let marker = form(&mut e, 5, 0, 1);
        e.set_global(OCCLUSION_MARKER_FORM_234, marker);
        // The link accessor answers the array at the extra list's word.
        e.register(EXTRA_LIST_GET_OCCLUSION_LINKED_REFS, |e, a| {
            e.mem.u32(a[0]).into_ret()
        });
        let r = with_base(&mut e, marker);
        let other = object(&mut e);
        let neighbour = object(&mut e);
        let own_links = e.mem.alloc(16);
        let neighbour_links = e.mem.alloc(16);
        e.mem.set_u32(r + 0x44, own_links);
        e.mem.set_u32(neighbour + 0x44, neighbour_links);
        // This reference links to the neighbour (slot 1), which links back
        // to it (slots 0 and 3) and to another one (slot 2).
        e.mem.set_u32(own_links + 4, neighbour);
        for (slot, value) in [(0, r), (1, 0), (2, other), (3, r)] {
            e.mem.set_u32(neighbour_links + 4 * slot, value);
        }
        let c = cell(&mut e);
        e.call(0x0054_ca90, &args![c, r]);
        let words: Vec<u32> = (0..4).map(|i| e.mem.u32(neighbour_links + 4 * i)).collect();
        assert_eq!(words, vec![0, 0, other, 0]);
    }

    #[test]
    fn clearing_the_cell_lets_the_references_go() {
        let mut e = list_edit_engine();
        let player = object(&mut e);
        e.set_global(PLAYER_POINTER, player);
        e.register(REFERENCE_GET_REF_PERSISTS, |e, a| {
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        e.register(FORM_TEST_452440, |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        quiet(&mut e, &[CELL_FN_5576C0, DATA_HANDLER_FLAG_4226E0]);
        let leaves = record(&mut e, fake(0x228), 0);
        let deleted = record(&mut e, fake(0x10), 0);
        let collected = record(&mut e, GARBAGE_COLLECTOR_ADD, 0);
        let filed = record(&mut e, CELL_FN_558BA0, 0);
        let persistent = object(&mut e);
        e.mem.set_u32(persistent + 0x30, 1);
        let accepted_base = form(&mut e, 0x23, 0, 1);
        e.mem.set_u32(accepted_base + 0x30, 1);
        let accepted = with_base(&mut e, accepted_base);
        let other_base = form(&mut e, 5, 0, 2);
        let plain = with_base(&mut e, other_base);
        let c = cell_with(&mut e, &[player, persistent, accepted, plain]);
        e.mem.set_u16(c.addr() + 0xa8, 9);
        e.call(0x0054_cd20, &args![c]);
        assert_eq!(e.mem.u16(c.addr() + 0xa8), 0);
        assert_eq!(list_items(&e, c.addr() + 0xac), Vec::<u32>::new());
        // The player and the persistent reference only leave the cell.
        assert_eq!(*leaves.borrow(), vec![vec![player, 0], vec![persistent, 0]]);
        assert!(deleted.borrow().is_empty());
        assert_eq!(*filed.borrow(), vec![vec![c.addr(), accepted]]);
        assert_eq!(*collected.borrow(), vec![vec![accepted], vec![plain]]);
    }

    #[test]
    fn clearing_a_persistent_cell_deletes_what_it_keeps() {
        let mut e = list_edit_engine();
        e.set_global(PLAYER_POINTER, 0x5555u32);
        e.register(REFERENCE_GET_REF_PERSISTS, |_, _| 1u32.into_ret());
        quiet(&mut e, &[CELL_FN_5576C0]);
        let deleted = record(&mut e, fake(0x10), 0);
        let collected = record(&mut e, GARBAGE_COLLECTOR_ADD, 0);
        let c = cell(&mut e);
        e.mem.set_u32(c.addr() + 8, 0x400);
        let r = object(&mut e);
        embedded_list(&mut e, c.addr() + 0xac, &[r]);
        // The data handler is not loading: the persistent reference of the
        // persistent cell is deleted (slot 0x10 with 1) and the cell's list
        // is emptied.
        e.set_global(DATA_HANDLER_POINTER, 0x6000u32);
        returns(&mut e, DATA_HANDLER_FLAG_4226E0, 1);
        e.call(0x0054_cd20, &args![c]);
        assert_eq!(*deleted.borrow(), vec![vec![r, 1]]);
        assert!(collected.borrow().is_empty());
        // While the data handler is loading the reference is kept apart and
        // goes to the garbage collector instead.
        deleted.borrow_mut().clear();
        returns(&mut e, DATA_HANDLER_FLAG_4226E0, 0);
        embedded_list(&mut e, c.addr() + 0xac, &[r]);
        e.call(0x0054_cd20, &args![c]);
        assert_eq!(*collected.borrow(), vec![vec![r]]);
    }

    #[test]
    fn first_reference_of_the_cell() {
        let mut e = engine();
        let a = object(&mut e);
        let b = object(&mut e);
        let c = cell_with(&mut e, &[a, b]);
        start_log(&mut e);
        assert_eq!(e.call(0x0054_cee0, &args![c]).u32(), a);
        let order = sequence(&take_log(&mut e));
        assert_eq!(order.first(), Some(&CELL_LOCK_ENTER));
        assert_eq!(order.last(), Some(&CELL_LOCK_LEAVE));
    }

    #[test]
    fn find_reference_by_base_form() {
        let mut e = engine();
        let wanted = form(&mut e, 5, 0, 1);
        let other = form(&mut e, 5, 0, 2);
        let flagged = with_base(&mut e, wanted);
        e.mem.set_u32(flagged + 8, 0x20);
        let wrong = with_base(&mut e, other);
        let right = with_base(&mut e, wanted);
        let c = cell_with(&mut e, &[flagged, wrong, right]);
        // Without the skip flag the first match wins, flagged or not.
        assert_eq!(e.call(0x0054_cf20, &args![c, wanted, 0u32]).u32(), flagged);
        // With it the flagged reference is passed over.
        assert_eq!(e.call(0x0054_cf20, &args![c, wanted, 1u32]).u32(), right);
        let missing = form(&mut e, 5, 0, 3);
        start_log(&mut e);
        assert_eq!(e.call(0x0054_cf20, &args![c, missing, 0u32]).u32(), 0);
        assert_eq!(sequence(&take_log(&mut e)).last(), Some(&CELL_LOCK_LEAVE));
        // A null form is not looked up.
        start_log(&mut e);
        assert_eq!(e.call(0x0054_cf20, &args![c, 0u32, 0u32]).u32(), 0);
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
    }

    // --- Second session: 0054cfd0 .. 0054def0 --------------------------------

    /// The three floats at `address`.
    fn vector_at(e: &Engine, address: u32) -> [f32; 3] {
        [
            e.mem.f32(address),
            e.mem.f32(address + 4),
            e.mem.f32(address + 8),
        ]
    }

    fn set_vector(e: &mut Engine, address: u32, value: [f32; 3]) {
        for (i, v) in value.iter().enumerate() {
            e.mem.set_f32(address + 4 * i as u32, *v);
        }
    }

    #[test]
    fn coc_placement_info_starts_in_the_middle_of_the_cell() {
        let mut e = vector_engine();
        returns(&mut e, CELL_GET_DATA_X, 3);
        returns(&mut e, CELL_GET_DATA_Y, 4);
        returns(&mut e, CELL_GET_LAND, 0);
        let setting = e.mem.alloc(4);
        e.mem.set_u8(setting, 1);
        returns(&mut e, SETTING_TEXT, setting);
        let loads = record(&mut e, CELL_LOAD_ALL_TEMP_DATA, 0);
        let c = cell(&mut e);
        let out = e.mem.alloc(24);
        e.call(0x0054_cfd0, &args![c, out, out + 12]);
        assert_eq!(vector_at(&e, out), [14336.0, 18432.0, 0.0]);
        assert_eq!(vector_at(&e, out + 12), [9.0, 9.0, 9.0]);
        assert!(loads.borrow().is_empty());
        // A zero first byte of the setting loads the temporary data and
        // refines the placement.
        e.mem.set_u8(setting, 0);
        e.call(0x0054_cfd0, &args![c, out, out + 12]);
        assert_eq!(*loads.borrow(), vec![vec![c.addr()]]);
        // So does an interior cell.
        e.mem.set_u8(setting, 1);
        loads.borrow_mut().clear();
        e.mem.set_u8(c.addr() + 0x24, 1);
        e.call(0x0054_cfd0, &args![c, out, out + 12]);
        assert_eq!(loads.borrow().len(), 1);
    }

    /// Doubles of the placement search: `+0x50` of a reference is the
    /// address of its rotation, `+0x54` its teleport data.
    fn coc_engine() -> Engine {
        let mut e = vector_engine();
        for (global, id) in [
            (COC_MARKER_FORM_244, 0x244u32),
            (COC_MARKER_FORM_248, 0x248),
            (COC_MARKER_FORM_24C, 0x24c),
            (TELEPORT_EXCLUDED_FORM_258, 0x258),
        ] {
            let base = form(&mut e, 5, 0, id);
            e.set_global(global, base);
        }
        e.register(REFERENCE_POSITION_ADDRESS, |e, a| {
            e.mem.u32(a[0] + 0x50).into_ret()
        });
        e.register(REFERENCE_GET_TELEPORT_DATA, |e, a| {
            e.mem.u32(a[0] + 0x54).into_ret()
        });
        e.register(TELEPORT_POSITION, |_, a| (a[0] + 4).into_ret());
        returns(&mut e, CELL_GET_LAND, 0);
        returns(&mut e, CELL_POSITION_FITS, 1);
        e
    }

    /// A reference at `(v, v, v)` with the rotation `(v, v, v)` times 10.
    fn coc_ref(e: &mut Engine, base: u32, v: f32) -> u32 {
        let r = placed_on(e, base, v, v, v);
        let rotation = e.mem.alloc(12);
        set_vector(e, rotation, [v * 10.0; 3]);
        e.mem.set_u32(r + 0x50, rotation);
        r
    }

    fn coc_search(e: &mut Engine, c: Ptr<TESObjectCELL>) -> ([f32; 3], [f32; 3]) {
        let out = e.mem.alloc(24);
        set_vector(e, out, [7.0; 3]);
        set_vector(e, out + 12, [8.0; 3]);
        e.call(0x0054_d090, &args![c, out, out + 12]);
        (vector_at(e, out), vector_at(e, out + 12))
    }

    #[test]
    fn coc_placement_prefers_the_markers_in_order() {
        let mut e = coc_engine();
        let any_base = form(&mut e, 5, 0, 1);
        let (b244, b248, b24c) = (
            e.global::<u32>(COC_MARKER_FORM_244),
            e.global::<u32>(COC_MARKER_FORM_248),
            e.global::<u32>(COC_MARKER_FORM_24C),
        );
        let any = coc_ref(&mut e, any_base, 1.0);
        let m244 = coc_ref(&mut e, b244, 2.0);
        let m248 = coc_ref(&mut e, b248, 3.0);
        let m24c = coc_ref(&mut e, b24c, 4.0);
        let second_24c = coc_ref(&mut e, b24c, 5.0);
        let flagged_24c = coc_ref(&mut e, b24c, 6.0);
        e.mem.set_u32(flagged_24c + 8, 0x20);
        let list = [flagged_24c, any, m244, m248, m24c, second_24c];
        let mut c = cell_with(&mut e, &list);
        e.mem.set_u8(c.addr() + 0x24, 1);
        // The first usable 011ca24c marker wins and brings its rotation.
        assert_eq!(coc_search(&mut e, c), ([4.0; 3], [40.0; 3]));
        // Without it the 011ca248 marker, rotation included.
        c = cell_with(&mut e, &[any, m244, m248]);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(coc_search(&mut e, c), ([3.0; 3], [30.0; 3]));
        // Then the 011ca244 marker, which leaves the rotation alone.
        c = cell_with(&mut e, &[any, m244]);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(coc_search(&mut e, c), ([2.0; 3], [8.0; 3]));
    }

    #[test]
    fn coc_placement_falls_back_on_a_type_20_reference_then_any() {
        let mut e = coc_engine();
        let plain_base = form(&mut e, 5, 0, 1);
        let type_20_base = form(&mut e, 0x20, 0, 2);
        let any = coc_ref(&mut e, plain_base, 1.0);
        let last = coc_ref(&mut e, plain_base, 2.0);
        let type_20 = coc_ref(&mut e, type_20_base, 3.0);
        let c = cell_with(&mut e, &[any, type_20, last]);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(coc_search(&mut e, c).0, [3.0; 3]);
        // Nothing but plain references: the last usable one.
        let c = cell_with(&mut e, &[any, last]);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(coc_search(&mut e, c).0, [2.0; 3]);
        // An empty cell leaves the position alone.
        let c = cell_with(&mut e, &[]);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(coc_search(&mut e, c).0, [7.0; 3]);
        // In an exterior cell a position outside the cell does not count.
        returns(&mut e, CELL_POSITION_FITS, 0);
        let c = cell_with(&mut e, &[any, type_20, last]);
        assert_eq!(coc_search(&mut e, c).0, [7.0; 3]);
    }

    #[test]
    fn coc_placement_follows_a_door_to_its_target_position() {
        let mut e = coc_engine();
        let plain_base = form(&mut e, 5, 0, 1);
        let b24c = e.global::<u32>(COC_MARKER_FORM_24C);
        let door = coc_ref(&mut e, plain_base, 1.0);
        let target = coc_ref(&mut e, plain_base, 5.0);
        let first = e.mem.alloc(16);
        let second = e.mem.alloc(16);
        e.mem.set_u32(door + 0x54, first);
        e.mem.set_u32(first, target);
        e.mem.set_u32(target + 0x54, second);
        set_vector(&mut e, second + 4, [11.0, 12.0, 13.0]);
        let c = cell_with(&mut e, &[door]);
        e.mem.set_u8(c.addr() + 0x24, 1);
        // The teleport position beats the plain reference.
        assert_eq!(coc_search(&mut e, c).0, [11.0, 12.0, 13.0]);
        // A marker in the cell still beats it.
        let marker = coc_ref(&mut e, b24c, 4.0);
        let c = cell_with(&mut e, &[door, marker]);
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(coc_search(&mut e, c), ([4.0; 3], [40.0; 3]));
    }

    #[test]
    fn coc_placement_raises_the_height_to_land_and_water() {
        let mut e = coc_engine();
        let land = e.mem.alloc(4);
        returns(&mut e, CELL_GET_LAND, land);
        e.register(LAND_GET_HEIGHT, |e, a| {
            e.mem.set_f32(a[2], 100.0);
            Ret::default()
        });
        let water = Rc::new(RefCell::new(150.0f32));
        let seen = water.clone();
        e.register_double(CELL_GET_WATER_HEIGHT, move |_, _| {
            (*seen.borrow()).into_ret()
        });
        let plain_base = form(&mut e, 5, 0, 1);
        let low = coc_ref(&mut e, plain_base, 1.0);
        let c = cell_with(&mut e, &[low]);
        // The land is at 100, the water at 150: the higher one.
        assert_eq!(coc_search(&mut e, c).0, [1.0, 1.0, 150.0]);
        *water.borrow_mut() = 50.0;
        assert_eq!(coc_search(&mut e, c).0, [1.0, 1.0, 100.0]);
        // A position above both stays.
        let high = coc_ref(&mut e, plain_base, 1000.0);
        let c = cell_with(&mut e, &[high]);
        assert_eq!(coc_search(&mut e, c).0, [1000.0; 3]);
    }

    /// The callback at `CALLBACK` records `(reference, context)` of its calls.
    type Visits = Rc<RefCell<Vec<(u32, u32)>>>;
    const CALLBACK: u32 = 0x00a0_0000;

    /// Doubles of the range walks: the distance test answers 1 (out of
    /// range) when the x of the position is above the limit, else -1; the
    /// callback records its calls and says yes for the references listed in
    /// the returned `stop`.
    fn walk_engine() -> (Engine, Visits, Rc<RefCell<Vec<u32>>>) {
        let mut e = vector_engine();
        e.map(0x0102_3000, 0x1000);
        e.set_global(FLOAT_MAX_DOUBLE, f64::from(f32::MAX));
        e.register(DISTANCE_TEST, |e, a| {
            let x = e.mem.f32(a[0]);
            ((if x > f32::from_bits(a[2]) {
                1i32
            } else {
                -1i32
            }) as u32)
                .into_ret()
        });
        e.register(POINTS_EQUAL, |e, a| {
            u32::from((0..3).all(|i| e.mem.u32(a[0] + 4 * i) == e.mem.u32(a[1] + 4 * i))).into_ret()
        });
        let calls = Rc::new(RefCell::new(Vec::new()));
        let stop = Rc::new(RefCell::new(Vec::new()));
        let (seen, stops) = (calls.clone(), stop.clone());
        e.register_double(CALLBACK, move |_, a| {
            seen.borrow_mut().push((a[0], a[1]));
            u32::from(stops.borrow().contains(&a[0])).into_ret()
        });
        (e, calls, stop)
    }

    #[test]
    fn range_walk_of_a_cell_hands_over_what_is_in_range() {
        let (mut e, calls, _) = walk_engine();
        let near = placed(&mut e, 5, 5.0, 0.0, 0.0);
        let far = placed(&mut e, 5, 50.0, 0.0, 0.0);
        let c = cell_with(&mut e, &[near, far]);
        let point = e.mem.alloc(12);
        // One limit, two references: only the near one is handed over.
        let done = e
            .call(
                0x0054_da20,
                &args![c, point, 10.0f32, point, f32::MAX, CALLBACK, 0x77u32],
            )
            .bool();
        assert!(done, "no yes: the walk completes");
        assert_eq!(*calls.borrow(), vec![(near, 0x77)]);
        // Without limits both are handed over.
        calls.borrow_mut().clear();
        e.call(
            0x0054_da20,
            &args![c, point, f32::MAX, point, f32::MAX, CALLBACK, 1u32],
        );
        assert_eq!(*calls.borrow(), vec![(near, 1), (far, 1)]);
    }

    #[test]
    fn range_walk_checks_the_second_point_unless_it_repeats_the_first() {
        let (mut e, _, _) = walk_engine();
        // The test answers -1: always in range, so the second test is reached.
        let tests = record(&mut e, DISTANCE_TEST, u32::MAX);
        let item = placed(&mut e, 5, 5.0, 0.0, 0.0);
        let c = cell_with(&mut e, &[item]);
        let point_a = e.mem.alloc(12);
        let point_b = e.mem.alloc(12);
        set_vector(&mut e, point_a, [1.0, 2.0, 3.0]);
        set_vector(&mut e, point_b, [1.0, 2.0, 3.0]);
        // Same limit and equal points: one test.
        e.call(
            0x0054_da20,
            &args![c, point_a, 10.0f32, point_b, 10.0f32, CALLBACK, 0u32],
        );
        assert_eq!(tests.borrow().len(), 1);
        // Same limit, different points: two tests, the second on point B.
        tests.borrow_mut().clear();
        set_vector(&mut e, point_b, [9.0, 2.0, 3.0]);
        e.call(
            0x0054_da20,
            &args![c, point_a, 10.0f32, point_b, 10.0f32, CALLBACK, 0u32],
        );
        assert_eq!(tests.borrow().len(), 2);
        assert_eq!(tests.borrow()[1][1], point_b);
        // A different limit also tests twice; an unset second one never.
        tests.borrow_mut().clear();
        e.call(
            0x0054_da20,
            &args![c, point_a, 10.0f32, point_a, 20.0f32, CALLBACK, 0u32],
        );
        assert_eq!(tests.borrow().len(), 2);
        tests.borrow_mut().clear();
        e.call(
            0x0054_da20,
            &args![c, point_a, 10.0f32, point_b, f32::MAX, CALLBACK, 0u32],
        );
        assert_eq!(tests.borrow().len(), 1);
    }

    #[test]
    fn range_walk_answers_no_for_a_null_callback_and_stops_on_yes() {
        let (mut e, calls, stop) = walk_engine();
        let first = placed(&mut e, 5, 1.0, 0.0, 0.0);
        let second = placed(&mut e, 5, 2.0, 0.0, 0.0);
        let c = cell_with(&mut e, &[first, second]);
        let point = e.mem.alloc(12);
        start_log(&mut e);
        let done = e
            .call(
                0x0054_da20,
                &args![c, point, 10.0f32, point, f32::MAX, 0u32, 0u32],
            )
            .bool();
        assert!(!done);
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
        // A yes ends the walk with 0 after the lock is released.
        stop.borrow_mut().push(first);
        start_log(&mut e);
        let done = e
            .call(
                0x0054_da20,
                &args![c, point, f32::MAX, point, f32::MAX, CALLBACK, 5u32],
            )
            .bool();
        assert!(!done);
        assert_eq!(*calls.borrow(), vec![(first, 5)]);
        assert_eq!(sequence(&take_log(&mut e)).last(), Some(&CELL_LOCK_LEAVE));
    }

    /// Doubles of the container of visited cells (`011ca1f8`) and of the
    /// local container of doors: the first word points at a zero-terminated
    /// array of items.
    fn container_doubles(e: &mut Engine) {
        e.register(VISITED_ADD, |e, a| {
            let item = e.mem.u32(a[1]);
            let mut array = e.mem.u32(a[0]);
            if array == 0 {
                array = e.mem.alloc(0x40);
                e.mem.set_u32(a[0], array);
            }
            let mut end = array;
            while e.mem.u32(end) != 0 {
                end += 4;
            }
            e.mem.set_u32(end, item);
            Ret::default()
        });
        e.register(CHILD_ARRAY_NEXT, |e, a| {
            let at = e.mem.u32(a[1]);
            let next = at + 4;
            let following = if e.mem.u32(next) != 0 { next } else { 0 };
            e.mem.set_u32(a[1], following);
            at.into_ret()
        });
        e.register(VISITED_FIND, |e, a| {
            let wanted = e.mem.u32(a[1]);
            let mut at = e.mem.u32(a[0]);
            while at != 0 && e.mem.u32(at) != 0 {
                if e.mem.u32(at) == wanted {
                    return 1u32.into_ret();
                }
                at += 4;
            }
            0u32.into_ret()
        });
        e.register(VISITED_CLEAR, |e, a| {
            e.mem.set_u32(a[0], 0);
            Ret::default()
        });
        quiet(e, &[VISITED_CONSTRUCT, VISITED_DESTRUCT, CRT_ATEXIT]);
    }

    #[test]
    fn deep_range_walk_initialises_its_statics_once() {
        let (mut e, calls, _) = walk_engine();
        container_doubles(&mut e);
        let constructs = record(&mut e, VISITED_CONSTRUCT, 0);
        let exits = record(&mut e, CRT_ATEXIT, 0);
        let clears = record(&mut e, VISITED_CLEAR, 0);
        let near = placed(&mut e, 5, 5.0, 0.0, 0.0);
        let far = placed(&mut e, 5, 50.0, 0.0, 0.0);
        let c = cell_with(&mut e, &[near, 0, far]);
        let point = e.mem.alloc(12);
        let walk = |e: &mut Engine| {
            e.call(
                0x0054_d4b0,
                &args![c, point, 10.0f32, point, f32::MAX, CALLBACK, 0x42u32],
            )
            .bool()
        };
        assert!(walk(&mut e), "nothing said yes");
        assert_eq!(*calls.borrow(), vec![(near, 0x42)]);
        assert_eq!(e.global::<u32>(WALK_GUARD), 3);
        assert_eq!(e.global::<u32>(WALK_DEPTH), 0);
        assert_eq!(*exits.borrow(), vec![vec![WALK_VISITED_DESTRUCTOR]]);
        assert_eq!(constructs.borrow()[0], vec![WALK_VISITED]);
        assert_eq!(clears.borrow().len(), 1);
        // The statics are not built twice.
        walk(&mut e);
        assert_eq!(exits.borrow().len(), 1);
        // A null callback or cell: no walk, and the answer is "nothing found".
        calls.borrow_mut().clear();
        let none = e
            .call(
                0x0054_d4b0,
                &args![0u32, point, 10.0f32, point, f32::MAX, CALLBACK, 0u32],
            )
            .bool();
        assert!(none && calls.borrow().is_empty());
    }

    #[test]
    fn deep_range_walk_stops_when_the_callback_says_yes() {
        let (mut e, calls, stop) = walk_engine();
        container_doubles(&mut e);
        let first = placed(&mut e, 5, 1.0, 0.0, 0.0);
        let second = placed(&mut e, 5, 2.0, 0.0, 0.0);
        let c = cell_with(&mut e, &[first, second]);
        let point = e.mem.alloc(12);
        stop.borrow_mut().push(first);
        let done = e
            .call(
                0x0054_d4b0,
                &args![c, point, f32::MAX, point, f32::MAX, CALLBACK, 9u32],
            )
            .bool();
        assert!(!done);
        assert_eq!(*calls.borrow(), vec![(first, 9)]);
    }

    /// A door reference at `x` on a door base form with teleport data at
    /// `+0x54`: position at `+4`, world space at `+0x20`, cell at `+0x24`.
    fn door_to(e: &mut Engine, x: f32, world: u32, target_cell: u32) -> u32 {
        let base = form(e, 0x1c, 0, 0x50);
        let door = placed_on(e, base, x, 0.0, 0.0);
        let teleport = e.mem.alloc(0x40);
        set_vector(e, teleport + 4, [1.0, 2.0, 3.0]);
        e.mem.set_u32(teleport + 0x20, world);
        e.mem.set_u32(teleport + 0x24, target_cell);
        e.mem.set_u32(door + 0x54, teleport);
        door
    }

    fn door_engine() -> (Engine, Visits) {
        let (mut e, calls, _) = walk_engine();
        container_doubles(&mut e);
        let excluded = form(&mut e, 5, 0, 0x58);
        e.set_global(TELEPORT_EXCLUDED_FORM_258, excluded);
        e.register(REFERENCE_GET_TELEPORT_DATA, |e, a| {
            e.mem.u32(a[0] + 0x54).into_ret()
        });
        e.register(TELEPORT_GET_WORLD_SPACE, |e, a| {
            e.mem.u32(a[0] + 0x20).into_ret()
        });
        e.register(TELEPORT_GET_CELL, |e, a| e.mem.u32(a[0] + 0x24).into_ret());
        e.register(TELEPORT_POSITION, |_, a| (a[0] + 4).into_ret());
        e.register(REFERENCE_DISTANCE, |_, _| 2.0f32.into_ret());
        (e, calls)
    }

    #[test]
    fn deep_range_walk_hands_the_nearest_door_to_the_world_space() {
        let (mut e, calls) = door_engine();
        let walks = record(&mut e, WORLD_SPACE_WALK, 0);
        let world = object(&mut e);
        let door = door_to(&mut e, 100.0, world, 0);
        let c = cell_with(&mut e, &[door]);
        let point = e.mem.alloc(12);
        let done = e
            .call(
                0x0054_d4b0,
                &args![c, point, f32::MAX, point, f32::MAX, CALLBACK, 3u32],
            )
            .bool();
        // The callback saw the door; the world space walk takes over from
        // the door's target and the answer is "found".
        assert_eq!(*calls.borrow(), vec![(door, 3)]);
        assert!(!done);
        assert_eq!(
            *walks.borrow(),
            vec![args![
                world,
                WALK_BEST_POSITION,
                f32::MAX,
                point,
                f32::MAX,
                CALLBACK,
                3u32
            ]]
        );
        assert_eq!(vector_at(&e, WALK_BEST_POSITION), [1.0, 2.0, 3.0]);
        assert_eq!(e.global::<u32>(WALK_BEST_WORLD), world);

        // With a limit the walk starts with what is left of it: 20 - 2.
        let (mut e, _) = door_engine();
        let walks = record(&mut e, WORLD_SPACE_WALK, 0);
        let world = object(&mut e);
        let door = door_to(&mut e, 5.0, world, 0);
        let c = cell_with(&mut e, &[door]);
        let point = e.mem.alloc(12);
        e.call(
            0x0054_d4b0,
            &args![c, point, 20.0f32, point, f32::MAX, CALLBACK, 3u32],
        );
        assert_eq!(e.global::<f32>(WALK_BEST_DISTANCE), 18.0);
        assert_eq!(walks.borrow().len(), 1);
        assert_eq!(walks.borrow()[0][2], 18.0f32.to_bits());
    }

    #[test]
    fn deep_range_walk_follows_a_door_into_an_interior_cell() {
        let (mut e, calls) = door_engine();
        let inner = placed(&mut e, 5, 1.0, 0.0, 0.0);
        let target = cell_with(&mut e, &[inner]);
        e.mem.set_u8(target.addr() + 0x24, 1);
        let door = door_to(&mut e, 5.0, 0, target.addr());
        let own = placed(&mut e, 5, 2.0, 0.0, 0.0);
        let c = cell_with(&mut e, &[own, door]);
        let point = e.mem.alloc(12);
        let done = e
            .call(
                0x0054_d4b0,
                &args![c, point, 20.0f32, point, f32::MAX, CALLBACK, 3u32],
            )
            .bool();
        // The cell's own references come first, then the target cell is
        // walked with what is left of the limit.
        assert!(done);
        assert_eq!(*calls.borrow(), vec![(own, 3), (door, 3), (inner, 3)]);
        assert_eq!(e.global::<u32>(WALK_DEPTH), 0);
        assert_eq!(e.global::<u32>(WALK_BEST_WORLD), 0);
    }

    #[test]
    fn the_door_walk_destructor_forwards() {
        let mut e = engine();
        let calls = record(&mut e, VISITED_DESTRUCT, 0);
        e.call(0x0054_da00, &args![0x1234u32]);
        assert_eq!(*calls.borrow(), vec![vec![0x1234]]);
    }

    /// An array double that records `(array, item)` of every append.
    fn append_log(e: &mut Engine) -> Rc<RefCell<Vec<(u32, u32)>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let seen = log.clone();
        e.register_double(ARRAY_APPEND, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        log
    }

    /// Doors for the door lists: `+0x54` is the teleport data.
    fn door_list_cell(e: &mut Engine) -> (Ptr<TESObjectCELL>, [u32; 5]) {
        let excluded = form(e, 0x1c, 0, 0x58);
        e.set_global(TELEPORT_EXCLUDED_FORM_258, excluded);
        e.register(REFERENCE_GET_TELEPORT_DATA, |e, a| {
            e.mem.u32(a[0] + 0x54).into_ret()
        });
        let door_base = form(e, 0x1c, 0, 1);
        let with_data = placed_on(e, door_base, 3.0, 0.0, 0.0);
        e.mem.set_u32(with_data + 0x54, 1);
        let without_data = placed_on(e, door_base, 4.0, 0.0, 0.0);
        let hidden = placed_on(e, door_base, 5.0, 0.0, 0.0);
        e.mem.set_u32(hidden + 0x54, 1);
        e.mem.set_u32(hidden + 8, 0x800);
        let excluded_door = placed_on(e, excluded, 6.0, 0.0, 0.0);
        e.mem.set_u32(excluded_door + 0x54, 1);
        let plain_base = form(e, 5, 0, 2);
        let plain = placed_on(e, plain_base, 7.0, 0.0, 0.0);
        e.mem.set_u32(plain + 0x54, 1);
        let c = cell_with(
            e,
            &[with_data, without_data, 0, hidden, excluded_door, plain],
        );
        (c, [with_data, without_data, hidden, excluded_door, plain])
    }

    #[test]
    fn door_list_takes_doors_with_teleport_data() {
        let mut e = engine();
        let appended = append_log(&mut e);
        let (c, [with_data, ..]) = door_list_cell(&mut e);
        let list = e.mem.alloc(16);
        e.call(0x0054_db50, &args![c, list]);
        assert_eq!(*appended.borrow(), vec![(list, with_data)]);
    }

    #[test]
    fn nearby_doors_are_listed_with_the_array_size() {
        let mut e = vector_engine();
        let appended = append_log(&mut e);
        e.register(VECTOR_LENGTH_SQUARED, |e, a| {
            let v: Vec<f32> = (0..3).map(|i| e.mem.f32(a[0] + 4 * i)).collect();
            (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).into_ret()
        });
        returns(&mut e, ARRAY_COUNT, 5);
        let (c, [with_data, ..]) = door_list_cell(&mut e);
        // A second door with data, too far away.
        let door_base = form(&mut e, 0x1c, 0, 3);
        let far = placed_on(&mut e, door_base, 30.0, 0.0, 0.0);
        e.mem.set_u32(far + 0x54, 1);
        let c2 = cell_with(&mut e, &[with_data, far]);
        let _ = c;
        let point = e.mem.alloc(12);
        let list = e.mem.alloc(16);
        let count = e.call(0x0054_dc00, &args![c2, point, 10.0f32, list]).u32();
        assert_eq!(count, 5);
        assert_eq!(*appended.borrow(), vec![(list, with_data)]);
        // A null list answers 0 without taking the lock.
        start_log(&mut e);
        assert_eq!(
            e.call(0x0054_dc00, &args![c2, point, 10.0f32, 0u32]).u32(),
            0
        );
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
    }

    #[test]
    fn nearby_doors_skip_flagged_and_excluded_references() {
        let mut e = vector_engine();
        let appended = append_log(&mut e);
        e.register(VECTOR_LENGTH_SQUARED, |_, _| 1.0f32.into_ret());
        returns(&mut e, ARRAY_COUNT, 0);
        let (c, [with_data, without_data, ..]) = door_list_cell(&mut e);
        let _ = without_data;
        let point = e.mem.alloc(12);
        let list = e.mem.alloc(16);
        e.call(0x0054_dc00, &args![c, point, 10.0f32, list]);
        // Only the door with data: the hidden, excluded and plain ones go.
        assert_eq!(*appended.borrow(), vec![(list, with_data)]);
    }

    #[test]
    fn door_or_filtered_list() {
        let mut e = engine();
        let appended = append_log(&mut e);
        e.register(FILTER_OBJECT_USABLE, |e, a| {
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        let (c, [with_data, without_data, ..]) = door_list_cell(&mut e);
        let list = e.mem.alloc(16);
        e.call(0x0054_dcf0, &args![c, list]);
        // The base form of the door without data fails the filter.
        assert_eq!(*appended.borrow(), vec![(list, with_data)]);
        // With a passing filter it joins.
        let base = e.mem.u32(without_data + 0x20);
        e.mem.set_u32(base + 0x30, 1);
        appended.borrow_mut().clear();
        e.call(0x0054_dcf0, &args![c, list]);
        assert_eq!(
            *appended.borrow(),
            vec![(list, with_data), (list, without_data)]
        );
        // A null list does nothing.
        start_log(&mut e);
        e.call(0x0054_dcf0, &args![c, 0u32]);
        assert_eq!(sequence(&take_log(&mut e)), Vec::<u32>::new());
    }

    #[test]
    fn world_space_accessors_follow_the_interior_flag() {
        let mut e = engine();
        let exterior = cell(&mut e);
        let interior = cell(&mut e);
        e.mem.set_u8(interior.addr() + 0x24, 1);
        let world = object(&mut e);
        e.mem.set_u32(exterior.addr() + 0xc0, world);
        e.mem.set_u32(interior.addr() + 0xc0, 0x5150);
        // `GetWorldSpace`: the field of an exterior cell only.
        assert_eq!(e.call(0x0054_ddd0, &args![exterior]).u32(), world);
        assert_eq!(e.call(0x0054_ddd0, &args![interior]).u32(), 0);
        // The interior counterpart.
        assert_eq!(e.call(0x0054_de40, &args![interior]).u32(), 0x5150);
        assert_eq!(e.call(0x0054_de40, &args![exterior]).u32(), 0);
        // The setters only write where the getters read.
        e.call(0x0054_de10, &args![exterior, 0x11u32]);
        e.call(0x0054_de10, &args![interior, 0x22u32]);
        assert_eq!(e.mem.u32(exterior.addr() + 0xc0), 0x11);
        assert_eq!(e.mem.u32(interior.addr() + 0xc0), 0x5150);
        e.call(0x0054_de80, &args![exterior, 0x33u32]);
        e.call(0x0054_de80, &args![interior, 0x44u32]);
        assert_eq!(e.mem.u32(exterior.addr() + 0xc0), 0x11);
        assert_eq!(e.mem.u32(interior.addr() + 0xc0), 0x44);
    }

    #[test]
    fn interior_data_word_is_read_and_written_through_the_data_object() {
        let mut e = engine();
        let data = object(&mut e);
        e.mem.set_u32(data + 0x28, 0x77);
        let c = cell(&mut e);
        e.register_double(CELL_INTERIOR_DATA, move |_, _| data.into_ret());
        let exterior = c;
        // Exterior cell: nothing read, nothing written.
        assert_eq!(e.call(0x0054_deb0, &args![exterior]).u32(), 0);
        e.call(0x0054_def0, &args![exterior, 5u32]);
        assert_eq!(e.mem.u32(data + 0x28), 0x77);
        // Interior cell.
        e.mem.set_u8(c.addr() + 0x24, 1);
        assert_eq!(e.call(0x0054_deb0, &args![c]).u32(), 0x77);
        e.call(0x0054_def0, &args![c, 5u32]);
        assert_eq!(e.mem.u32(data + 0x28), 5);
        // No data object: 0 and nothing written.
        e.register(CELL_INTERIOR_DATA, |_, _| 0u32.into_ret());
        assert_eq!(e.call(0x0054_deb0, &args![c]).u32(), 0);
        e.call(0x0054_def0, &args![c, 9u32]);
        assert_eq!(e.mem.u32(data + 0x28), 5);
    }

    // --- Second session: 0054df30 .. 0054ee20 --------------------------------

    /// Doubles for `fn_0054df30`. Words of a reference: `+0x30` actor time
    /// check, `+0x34` encounter zone, `+0x38` persists, `+0x3C` test
    /// `0056ae60`, `+0x40` dead (`0087f4a0`), `+0x48` form flag
    /// `0x1000000`. The cell's zone is the word at `+0x30`. The extra list of
    /// a reference holds the ash pile (`+0`), the dropped item list (`+4`),
    /// the container changes (`+8`) and the model swap (`+0xC`).
    fn detach_engine(detach: u32) -> Engine {
        let mut e = list_edit_engine();
        returns(&mut e, GAME_LOADER_FLAG_244_2, 0);
        returns(&mut e, GAME_TIME_NOW, 1000);
        returns(&mut e, CELL_GET_DETACH_TIME, detach);
        returns(&mut e, ENCOUNTER_ZONE_DELAY, 100);
        e.register(CELL_ENCOUNTER_ZONE, |e, a| {
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        e.register(REFERENCE_GET_ENCOUNTER_ZONE, |e, a| {
            e.mem.u32(a[0] + 0x34).into_ret()
        });
        e.register(ACTOR_TIME_CHECK, |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        e.register(REFERENCE_GET_REF_PERSISTS, |e, a| {
            e.mem.u32(a[0] + 0x38).into_ret()
        });
        e.register(REFERENCE_TEST_56AE60, |e, a| {
            e.mem.u32(a[0] + 0x3c).into_ret()
        });
        e.register(ACTOR_TEST_87F4A0, |e, a| e.mem.u32(a[0] + 0x40).into_ret());
        e.register(REFERENCE_FLAG_1000000, |e, a| {
            e.mem.u32(a[0] + 0x48).into_ret()
        });
        e.register(EXTRA_LIST_GET_ASH_PILE_REF, |e, a| {
            e.mem.u32(a[0]).into_ret()
        });
        e.register(EXTRA_LIST_GET_DROPPED_ITEM_LIST, |e, a| {
            e.mem.u32(a[0] + 4).into_ret()
        });
        e.register(EXTRA_LIST_GET_CONTAINER_CHANGES, |e, a| {
            e.mem.u32(a[0] + 8).into_ret()
        });
        e.register(EXTRA_LIST_GET_MODEL_SWAP, |e, a| {
            e.mem.u32(a[0] + 0xc).into_ret()
        });
        quiet(
            &mut e,
            &[
                REFERENCE_TEST_577DE0,
                CHANGES_TEST_42CDE0,
                REFERENCE_TEST_565450,
                REFERENCE_SET_LOCATION,
                REFERENCE_SET_ROTATION,
                SCRIPT_INIT_ACTION_LIST,
                SCRIPT_SET_ACTION_FLAG,
                CELL_SET_DETACH_TIME,
                REFERENCE_MARK_AS_DELETED,
                EXTRA_LIST_ADD_DROPPED_ITEM,
                EXTRA_LIST_REMOVE_DROPPED_ITEM_LIST,
                REFERENCE_LOCK,
                DESTRUCTION_APPLY,
                BASE_GET_DESTRUCTION_FORM,
                FORM_SET_FLAG_2000,
                FORM_SET_EMPTY,
            ],
        );
        e.register(DATA_HANDLER_ID_TEST, |_, a| {
            u32::from(a[1] < 0xff00_0000).into_ret()
        });
        e.register(ENCOUNTER_ZONE_TEST, |_, _| 1u32.into_ret());
        e
    }

    /// A reference on a base form of type `kind`.
    fn typed(e: &mut Engine, kind: u8) -> u32 {
        let base = form(e, kind, 0, 1);
        with_base(e, base)
    }

    #[test]
    fn detached_cell_does_nothing_while_the_loader_flag_is_set() {
        let mut e = detach_engine(500);
        returns(&mut e, GAME_LOADER_FLAG_244_2, 1);
        let c = cell(&mut e);
        start_log(&mut e);
        assert_eq!(e.call(0x0054_df30, &args![c]).u8(), 0);
        assert_eq!(sequence(&take_log(&mut e)), vec![GAME_LOADER_FLAG_244_2]);
    }

    #[test]
    fn cell_without_detach_time_updates_the_persistent_references() {
        let mut e = detach_engine(0);
        let slot = record(&mut e, fake(0x208), 0);
        let times = record(&mut e, CELL_SET_DETACH_TIME, 0);
        // A persistent container (type 0x1B) is told "not 0056ae60"; a
        // persistent body (type 0x2B) too when its time check passes.
        let chest = typed(&mut e, 0x1b);
        e.mem.set_u32(chest + 0x38, 1);
        let loose_chest = typed(&mut e, 0x1b);
        let body = typed(&mut e, 0x2b);
        e.mem.set_u32(body + 0x38, 1);
        e.mem.set_u32(body + 0x30, 1);
        e.mem.set_u32(body + 0x3c, 1);
        let late_body = typed(&mut e, 0x2b);
        e.mem.set_u32(late_body + 0x38, 1);
        let other_type = typed(&mut e, 0x2c);
        e.mem.set_u32(other_type + 0x38, 1);
        e.mem.set_u32(other_type + 0x30, 1);
        let c = cell_with(&mut e, &[chest, loose_chest, body, late_body, other_type]);
        assert_eq!(e.call(0x0054_df30, &args![c]).u8(), 1);
        assert_eq!(*slot.borrow(), vec![vec![chest, 1], vec![body, 0]]);
        assert!(times.borrow().is_empty(), "the detach time is not cleared");
    }

    /// Builds the objects `fn_0054df30` needs for a cell detached long ago.
    fn detach_world(e: &mut Engine) -> Ptr<TESObjectCELL> {
        let c = cell(e);
        // A reference that is skipped: an actor that fails the time check.
        let skipped = typed(e, 5);
        answer(e, skipped, 0x100, 1);
        // A dead actor that belongs to the cell comes back.
        let restored = typed(e, 5);
        answer(e, restored, 0x100, 1);
        e.mem.set_u32(restored + 0x30, 1);
        e.mem.set_u32(restored + 0x40, 1);
        answer(e, restored, 0x298, c.addr());
        let rotation = e.mem.alloc(12);
        for (i, w) in [1.0f32, 2.0, 3.0].iter().enumerate() {
            e.mem.set_f32(rotation + 4 * i as u32, *w);
        }
        answer(e, restored, 0x16c, rotation);
        // An actor that slot 0x22C marks is deleted with its ash pile and
        // its dropped items.
        let killed = typed(e, 5);
        answer(e, killed, 0x100, 1);
        answer(e, killed, 0x22c, 1);
        e.mem.set_u32(killed + 0x30, 1);
        let ash = object(e);
        let dropped = object(e);
        let dropped_nodes = e.mem.alloc(8);
        embedded_list(e, dropped_nodes, &[dropped]);
        e.mem.set_u32(killed + 0x44, ash);
        e.mem.set_u32(killed + 0x48, dropped_nodes);
        // A container is unlocked and locked again.
        let chest = typed(e, 0x1b);
        e.mem.set_u32(chest + 0x3c, 1);
        e.mem.set_u32(chest + 0x4c, 0x9000);
        // A destructible object with a model swap.
        let destructible = typed(e, 5);
        e.mem.set_u32(destructible + 0x48, 1);
        e.mem.set_u32(destructible + 0x44 + 0xc, 0x1);
        // A type 0x1D form with form flag 0x20 whose cast accepts it.
        let door_form = form(e, 0x1d, 0, 9);
        e.mem.set_u32(door_form + 0x3c, TEST_VTABLE);
        e.mem.set_u32(door_form + 0x3c + ANSWERS + 4, 1);
        let door_like = with_base(e, door_form);
        e.mem.set_u32(door_like + 8, 0x20);
        // A type 0x26 reference is emptied.
        let emptied = typed(e, 0x26);
        embedded_list(
            e,
            c.addr() + 0xac,
            &[
                skipped,
                restored,
                killed,
                chest,
                destructible,
                door_like,
                emptied,
            ],
        );
        c
    }

    #[test]
    fn long_detached_cell_restores_deletes_and_resets_its_references() {
        let mut e = detach_engine(500);
        let c = detach_world(&mut e);
        let refs = list_items(&e, c.addr() + 0xac);
        let (skipped, restored, killed, chest, destructible, door_like, emptied) = (
            refs[0], refs[1], refs[2], refs[3], refs[4], refs[5], refs[6],
        );
        let killed_info = (e.mem.u32(killed + 0x44), e.mem.u32(killed + 0x48));
        // Calls on the actors and containers.
        let slot_324 = record(&mut e, fake(0x324), 0);
        let slot_170 = record(&mut e, fake(0x170), 0);
        let slot_208 = record(&mut e, fake(0x208), 0);
        let slot_c4 = record(&mut e, fake(0xc4), 0);
        let locations = record(&mut e, REFERENCE_SET_LOCATION, 0);
        let rotations = record(&mut e, REFERENCE_SET_ROTATION, 0);
        let deleted = record(&mut e, REFERENCE_MARK_AS_DELETED, 0);
        let dropped = record(&mut e, EXTRA_LIST_ADD_DROPPED_ITEM, 0);
        let removed = record(&mut e, EXTRA_LIST_REMOVE_DROPPED_ITEM_LIST, 0);
        let locks = record(&mut e, REFERENCE_LOCK, 0);
        let applied = record(&mut e, DESTRUCTION_APPLY, 0);
        returns(&mut e, BASE_GET_DESTRUCTION_FORM, 0xd5);
        let swaps = record(&mut e, FORM_SET_FLAG_2000, 0);
        let emptied_calls = record(&mut e, FORM_SET_EMPTY, 0);
        let inits = record(&mut e, SCRIPT_INIT_ACTION_LIST, 0);
        let flags = record(&mut e, SCRIPT_SET_ACTION_FLAG, 0);
        let times = record(&mut e, CELL_SET_DETACH_TIME, 0);
        assert_eq!(e.call(0x0054_df30, &args![c]).u8(), 1);

        assert_eq!(*slot_324.borrow(), vec![vec![restored, 1, 0, 0]]);
        // The restored actor is put back at the position slot 0x170 filled in
        // with the rotation of slot 0x16C.
        assert_eq!(slot_170.borrow().len(), 1);
        assert_eq!(
            *locations.borrow(),
            vec![vec![restored, slot_170.borrow()[0][1]]]
        );
        assert_eq!(
            *rotations.borrow(),
            vec![vec![
                restored,
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits()
            ]]
        );
        // The marked actor, its ash pile and its dropped item are deleted.
        let dropped_item = list_items(&e, e.mem.u32(killed + 0x48)).first().copied();
        assert_eq!(dropped_item, None, "the dropped item list is emptied");
        let (ash, _) = killed_info;
        assert_eq!(deleted.borrow()[0], vec![killed]);
        assert_eq!(deleted.borrow()[1], vec![e.mem.u32(killed + 0x44)]);
        assert_eq!(deleted.borrow()[1], vec![ash]);
        assert_eq!(deleted.borrow().len(), 3);
        assert_eq!(dropped.borrow().len(), 1);
        assert_eq!(dropped.borrow()[0][1], 0);
        assert_eq!(*removed.borrow(), vec![vec![killed + 0x44]]);
        // The container: unlocked, then locked.
        assert_eq!(*slot_208.borrow(), vec![vec![chest, 0]]);
        assert_eq!(*locks.borrow(), vec![vec![chest]]);
        // The destructible object gets its destruction and the model swap flag.
        assert_eq!(*applied.borrow(), vec![vec![0xd5, destructible]]);
        assert_eq!(*swaps.borrow(), vec![vec![destructible, 1]]);
        // The type 0x1D form re-attaches (slot 0xC4 with 0); the type 0x26 one
        // is emptied.
        assert_eq!(*slot_c4.borrow(), vec![vec![door_like, 0]]);
        assert_eq!(*emptied_calls.borrow(), vec![vec![emptied, 0]]);
        // Everything but the skipped actor has its action list reset.
        let reset: Vec<u32> = inits.borrow().iter().map(|a| a[0]).collect();
        assert_eq!(
            reset,
            vec![restored, killed, chest, destructible, door_like, emptied]
        );
        assert!(!reset.contains(&skipped));
        assert!(flags.borrow().iter().all(|a| a[2] == 0x8000_0000));
        assert_eq!(*times.borrow(), vec![vec![c.addr(), 0, 0]]);
    }

    #[test]
    fn detached_cell_with_a_zone_skips_references_of_other_zones() {
        let mut e = detach_engine(500);
        e.register(ENCOUNTER_ZONE_TEST, |_, a| u32::from(a[0] != 11).into_ret());
        returns(&mut e, ENCOUNTER_ZONE_TEST_CELL, 0);
        let flags = record(&mut e, SCRIPT_SET_ACTION_FLAG, 0);
        let same = typed(&mut e, 5);
        e.mem.set_u32(same + 0x34, 7);
        let other_ok = typed(&mut e, 5);
        e.mem.set_u32(other_ok + 0x34, 9);
        let other_bad = typed(&mut e, 5);
        e.mem.set_u32(other_bad + 0x34, 11);
        let none = typed(&mut e, 5);
        let c = cell_with(&mut e, &[same, other_ok, other_bad, none]);
        e.mem.set_u32(c.addr() + 0x30, 7);
        // The zone says no for this cell: answer 0, the cell's own zone skipped.
        assert_eq!(e.call(0x0054_df30, &args![c]).u8(), 0);
        let handled: Vec<u32> = flags.borrow().iter().map(|a| a[0]).collect();
        assert_eq!(handled, vec![other_ok, none]);
        // The zone says yes for this cell: the cell's own zone is handled too.
        flags.borrow_mut().clear();
        returns(&mut e, ENCOUNTER_ZONE_TEST_CELL, 1);
        assert_eq!(e.call(0x0054_df30, &args![c]).u8(), 1);
        let handled: Vec<u32> = flags.borrow().iter().map(|a| a[0]).collect();
        assert_eq!(handled, vec![same, other_ok, none]);
    }

    #[test]
    fn briefly_detached_cell_answers_no_without_a_zone() {
        let mut e = detach_engine(950);
        let flags = record(&mut e, SCRIPT_SET_ACTION_FLAG, 0);
        let plain = typed(&mut e, 5);
        let destructible = typed(&mut e, 5);
        e.mem.set_u32(destructible + 0x48, 1);
        let applied = record(&mut e, DESTRUCTION_APPLY, 0);
        let c = cell_with(&mut e, &[plain, destructible]);
        // 50 is not more than the delay of 100: no answer, no destruction, and
        // the references of the cell's (empty) zone are skipped.
        assert_eq!(e.call(0x0054_df30, &args![c]).u8(), 0);
        assert!(flags.borrow().is_empty());
        assert!(applied.borrow().is_empty());
    }

    #[test]
    fn cell_detached_for_good_reattaches_known_references() {
        let mut e = detach_engine(u32::MAX);
        let slot_c4 = record(&mut e, fake(0xc4), 0);
        let flags = record(&mut e, SCRIPT_SET_ACTION_FLAG, 0);
        let times = record(&mut e, CELL_SET_DETACH_TIME, 0);
        let known = typed(&mut e, 5);
        e.mem.set_u32(known + 0xc, 0x10);
        let known_deleted = typed(&mut e, 5);
        e.mem.set_u32(known_deleted + 0xc, 0x11);
        answer(&mut e, known_deleted, 0x160, 1);
        let dynamic = typed(&mut e, 5);
        e.mem.set_u32(dynamic + 0xc, 0xff00_0001);
        let c = cell_with(&mut e, &[known, known_deleted, dynamic]);
        assert_eq!(e.call(0x0054_df30, &args![c]).u8(), 1);
        // Known references are re-attached (unless slot 0x160 says no) and
        // not looked at further; the dynamic one gets its action list reset.
        assert_eq!(*slot_c4.borrow(), vec![vec![known, 1]]);
        let handled: Vec<u32> = flags.borrow().iter().map(|a| a[0]).collect();
        assert_eq!(handled, vec![dynamic]);
        assert_eq!(times.borrow().len(), 1);
    }

    /// The local map texture functions: slots at `0` and `4` of a frame, a
    /// rendered texture object whose slots at `+0x30` are non-zero.
    fn map_texture_engine() -> (Engine, Visits, u32) {
        let mut e = engine();
        let rendered = object(&mut e);
        for i in 0..4 {
            e.mem.set_u32(rendered + 0x30 + 4 * i, 0x100 + i);
        }
        e.register_double(TAKE_EXTERIOR_LOCAL_MAP_PICTURE, move |e, a| {
            e.mem.set_u32(a[1], rendered);
            Ret::default()
        });
        e.register_double(TAKE_INTERIOR_LOCAL_MAP_PICTURE, move |e, a| {
            e.mem.set_u32(a[3], rendered);
            Ret::default()
        });
        e.register(RENDERED_TEXTURE_GET_TEXTURE, |_, a| {
            (a[0] + 0x1000).into_ret()
        });
        let assigned = Rc::new(RefCell::new(Vec::new()));
        let seen = assigned.clone();
        e.register_double(SMART_POINTER_ASSIGN, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        (e, assigned, rendered)
    }

    #[test]
    fn exterior_local_map_texture_is_taken_from_the_picture() {
        let (mut e, assigned, rendered) = map_texture_engine();
        let c = cell(&mut e);
        let out = e.mem.alloc(4);
        e.call(0x0054_e640, &args![c, out]);
        assert_eq!(*assigned.borrow(), vec![(out, rendered + 0x1000)]);
        // The picture's four slots were emptied.
        assert!((0..4).all(|i| e.mem.u32(rendered + 0x30 + 4 * i) == 0));
        // An interior cell has no exterior picture: the pointer is emptied.
        assigned.borrow_mut().clear();
        e.mem.set_u8(c.addr() + 0x24, 1);
        e.call(0x0054_e640, &args![c, out]);
        assert_eq!(*assigned.borrow(), vec![(out, 0)]);
    }

    #[test]
    fn interior_local_map_texture_is_taken_from_the_picture() {
        let (mut e, assigned, rendered) = map_texture_engine();
        let c = cell(&mut e);
        let out = e.mem.alloc(4);
        // An exterior cell has no interior picture.
        e.call(0x0054_e750, &args![c, 1u32, 2u32, out]);
        assert_eq!(*assigned.borrow(), vec![(out, 0)]);
        assigned.borrow_mut().clear();
        e.mem.set_u8(c.addr() + 0x24, 1);
        let taken = Rc::new(RefCell::new(Vec::new()));
        let seen = taken.clone();
        e.register_double(TAKE_INTERIOR_LOCAL_MAP_PICTURE, move |e, a| {
            seen.borrow_mut().push(a.to_vec());
            e.mem.set_u32(a[3], rendered);
            Ret::default()
        });
        e.call(0x0054_e750, &args![c, 1u32, 2u32, out]);
        assert_eq!(taken.borrow()[0][..3], [c.addr(), 1, 2]);
        assert_eq!(*assigned.borrow(), vec![(out, rendered + 0x1000)]);
        assert!((0..4).all(|i| e.mem.u32(rendered + 0x30 + 4 * i) == 0));
    }

    #[test]
    fn rendered_texture_slots_are_emptied() {
        let mut e = engine();
        let texture = object(&mut e);
        for i in 0..4 {
            e.mem.set_u32(texture + 0x30 + 4 * i, 0x100 + i);
        }
        e.mem.set_u32(texture + 0x2c, 0x5);
        e.mem.set_u32(texture + 0x40, 0x6);
        e.call(0x0054_e710, &args![texture]);
        assert!((0..4).all(|i| e.mem.u32(texture + 0x30 + 4 * i) == 0));
        assert_eq!(e.mem.u32(texture + 0x2c), 5);
        assert_eq!(e.mem.u32(texture + 0x40), 6);
    }

    #[test]
    fn picture_target_goes_to_the_render_call() {
        let mut e = engine();
        e.register(RENDERED_TEXTURE_TARGET, |_, a| (a[0] + 7).into_ret());
        let calls = record(&mut e, SET_RENDER_TARGET_FLAGGED, 0);
        e.call(0x0054_ede0, &args![0x1000u32, 7u32]);
        e.call(0x0054_ede0, &args![0u32, 7u32]);
        assert_eq!(*calls.borrow(), vec![vec![0x1007, 7], vec![0, 7]]);
    }

    #[test]
    fn picture_flag_needs_the_test_and_the_object_byte() {
        let mut e = engine();
        let object_with_byte = e.mem.alloc(0x20);
        returns(&mut e, MAP_PICTURE_TEST, 0);
        returns(&mut e, MAP_PICTURE_OBJECT, object_with_byte);
        e.mem.set_u8(object_with_byte + 0x18, 1);
        assert_eq!(e.call(0x0054_ee20, &args![]).u8(), 0);
        returns(&mut e, MAP_PICTURE_TEST, 1);
        assert_eq!(e.call(0x0054_ee20, &args![]).u8(), 1);
        e.mem.set_u8(object_with_byte + 0x18, 0);
        assert_eq!(e.call(0x0054_ee20, &args![]).u8(), 0);
    }

    /// Everything `TakeLocalMapPicture` calls, as doubles that record into
    /// the call log; returns the engine and the objects the test names.
    struct PictureWorld {
        e: Engine,
        cell: Ptr<TESObjectCELL>,
        holder: u32,
        first: u32,
        second: u32,
        scene_node: u32,
        node_6: u32,
        out: u32,
        sets: Rc<RefCell<Vec<(u32, u32)>>>,
    }

    fn picture_world(main_pass: bool, textures: bool, node_6: bool) -> PictureWorld {
        let mut e = engine();
        let holder = object(&mut e);
        let (first, second) = (0x4001u32, 0x4002u32);
        let scene_node = object(&mut e);
        let water = object(&mut e);
        let node_6_object = if node_6 { object(&mut e) } else { 0 };
        returns(&mut e, SHADER_GET_ACCUMULATOR, 0x1111);
        returns(&mut e, RENDER_TARGET_HOLDER, holder);
        returns(&mut e, TEXTURE_MANAGER, 0x2222);
        returns(&mut e, TEXTURE_MANAGER_CREATE_A, first);
        returns(
            &mut e,
            TEXTURE_MANAGER_CREATE_B,
            if textures { second } else { 0 },
        );
        e.register(COLOR_CONSTRUCT, |_, a| a[0].into_ret());
        returns(&mut e, SHADOW_SCENE_NODE_GETTER, scene_node);
        returns(&mut e, SHADOW_SCENE_NODE_GET_FLAG, 5);
        returns(&mut e, WATER_RENDER_FLAG, u32::from(main_pass));
        returns(&mut e, WATER_OBJECT, water);
        returns(&mut e, WATER_OBJECT_GET_FLAG, 3);
        returns(&mut e, MAP_PICTURE_TEST, 0);
        e.register(RENDERED_TEXTURE_TARGET, |_, a| (a[0] + 1).into_ret());
        e.register(SHADER_ACCUMULATOR_CONSTRUCT, |_, a| a[0].into_ret());
        returns(&mut e, CELL_CHILD_NODE, node_6_object);
        returns(&mut e, IMAGE_SPACE_MANAGER, 0x3333);
        returns(&mut e, IMAGE_SPACE_GET_EFFECT, 0xee);
        quiet(
            &mut e,
            &[
                PROFILE_SCOPE_CONSTRUCT,
                PROFILE_SCOPE_DESTRUCT,
                CULLING_PROCESS_CONSTRUCT,
                CULLING_PROCESS_DESTRUCT,
                SHADOW_SCENE_NODE_SET_FLAG,
                WATER_OBJECT_SET_FLAG,
                MAP_PICTURE_SET_FLAG,
                SET_RENDER_TARGET,
                SET_RENDER_TARGET_FLAGGED,
                ACCUMULATOR_SET_SCENE_NODE,
                CULLING_PROCESS_SET_ACCUMULATOR,
                ACCUMULATOR_SET_MODE,
                CULLING_PROCESS_BEGIN,
                CULLING_PROCESS_END,
                ACCUMULATE_SCENE,
                FINISH_ACCUMULATION,
                END_OFFSCREEN_MAIN,
                END_OFFSCREEN,
                IMAGE_SPACE_RENDER_EFFECT,
                ACCUMULATOR_SET_FLAG,
                TEXTURE_MANAGER_RETURN,
                SHADER_SET_ACCUMULATOR,
            ],
        );
        let sets = Rc::new(RefCell::new(Vec::new()));
        let seen = sets.clone();
        e.register_double(SMART_POINTER_ASSIGN, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        let cell = cell(&mut e);
        let out = e.mem.alloc(4);
        PictureWorld {
            e,
            cell,
            holder,
            first,
            second,
            scene_node,
            node_6: node_6_object,
            out,
            sets,
        }
    }

    #[test]
    fn local_map_picture_draws_the_scene_into_the_second_texture() {
        let mut w = picture_world(true, true, true);
        let root = object(&mut w.e);
        start_log(&mut w.e);
        w.e.call(0x0054_e830, &args![w.cell, root, w.out]);
        let log = take_log(&mut w.e);
        let calls = |address: u32| calls_to(&log, address);
        // The scope guard, then the two textures from the manager.
        assert_eq!(calls(PROFILE_SCOPE_CONSTRUCT).len(), 1);
        assert_eq!(
            calls(PROFILE_SCOPE_CONSTRUCT)[0][1..],
            [0xe, 1, 0x0102_ed68, 0x2251]
        );
        assert_eq!(
            calls(TEXTURE_MANAGER_CREATE_A),
            vec![vec![0x2222, w.holder, 0x1e, 0, 0, 0]]
        );
        assert_eq!(
            calls(TEXTURE_MANAGER_CREATE_B),
            vec![vec![0x2222, w.holder, 0x1c, 0, 0]]
        );
        // The root and the cell's child node 6 are accumulated, each through
        // the culling process (modes 3 and 1).
        let scenes = calls(ACCUMULATE_SCENE);
        assert_eq!(scenes.len(), 2);
        assert_eq!((scenes[0][0], scenes[0][1]), (root, w.scene_node));
        assert_eq!((scenes[1][0], scenes[1][1]), (root, w.node_6));
        let modes: Vec<u32> = calls(CULLING_PROCESS_BEGIN).iter().map(|a| a[1]).collect();
        assert_eq!(modes, vec![3, 1]);
        // The effect draws the first texture into the second.
        assert_eq!(
            calls(IMAGE_SPACE_RENDER_EFFECT),
            vec![vec![0x3333, 0xee, w.holder, w.first, w.second, 0, 0]]
        );
        // The first texture goes back, the second is handed out, the saved
        // accumulator is restored.
        assert_eq!(calls(TEXTURE_MANAGER_RETURN), vec![vec![0x2222, w.first]]);
        assert_eq!(w.sets.borrow().last().unwrap().1, w.second);
        assert_eq!(w.sets.borrow().last().unwrap().0, w.out);
        assert_eq!(calls(SHADER_SET_ACCUMULATOR), vec![vec![0x1111]]);
        // The shadow scene node's flag is set to 1 and restored to 5; the
        // water object's flag likewise to 1 and back to 3.
        let flags: Vec<u32> = calls(SHADOW_SCENE_NODE_SET_FLAG)
            .iter()
            .map(|a| a[1])
            .collect();
        assert_eq!(flags, vec![1, 5]);
        let water: Vec<u32> = calls(WATER_OBJECT_SET_FLAG).iter().map(|a| a[1]).collect();
        assert_eq!(water, vec![1, 3]);
        // The water code's flag makes the targets go through `(7, target)`.
        assert_eq!(calls(SET_RENDER_TARGET).len(), 2);
        assert!(calls(SET_RENDER_TARGET_FLAGGED).is_empty());
        assert_eq!(calls(END_OFFSCREEN_MAIN).len(), 2);
    }

    #[test]
    fn local_map_picture_other_pass_and_missing_node() {
        let mut w = picture_world(false, true, false);
        let root = object(&mut w.e);
        start_log(&mut w.e);
        w.e.call(0x0054_e830, &args![w.cell, root, w.out]);
        let log = take_log(&mut w.e);
        // No child node 6: one accumulation. The other pass uses the flagged
        // call and the other end call.
        assert_eq!(calls_to(&log, ACCUMULATE_SCENE).len(), 1);
        assert_eq!(calls_to(&log, SET_RENDER_TARGET_FLAGGED).len(), 2);
        assert!(calls_to(&log, SET_RENDER_TARGET).is_empty());
        assert_eq!(calls_to(&log, END_OFFSCREEN).len(), 2);
        assert!(calls_to(&log, END_OFFSCREEN_MAIN).is_empty());
        assert_eq!(
            calls_to(&log, SET_RENDER_TARGET_FLAGGED)[0],
            vec![w.first + 1, 7]
        );
    }

    #[test]
    fn local_map_picture_without_textures_draws_nothing() {
        let mut w = picture_world(true, false, true);
        let root = object(&mut w.e);
        start_log(&mut w.e);
        w.e.call(0x0054_e830, &args![w.cell, root, w.out]);
        let log = take_log(&mut w.e);
        assert!(calls_to(&log, ACCUMULATE_SCENE).is_empty());
        assert!(calls_to(&log, IMAGE_SPACE_RENDER_EFFECT).is_empty());
        // The first texture still goes back and the (empty) second is handed out.
        assert_eq!(
            calls_to(&log, TEXTURE_MANAGER_RETURN),
            vec![vec![0x2222, w.first]]
        );
        assert_eq!(w.sets.borrow().last().unwrap().1, 0);
        assert_eq!(calls_to(&log, SHADER_SET_ACCUMULATOR), vec![vec![0x1111]]);
        let order = sequence(&log);
        assert_eq!(order.first(), Some(&PROFILE_SCOPE_CONSTRUCT));
        assert_eq!(order.last(), Some(&PROFILE_SCOPE_DESTRUCT));
    }
}
