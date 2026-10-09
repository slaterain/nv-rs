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
//! reference walks that move references between cells. The next session
//! continues with the first function after `0054b5b0` that the ledger queue
//! (`queue "fallout shared/tesobjectcell.cpp" 00547650 00552470`) lists as
//! not done.
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
        assert_eq!(funcs.len(), 40);
        let addresses: Vec<u32> = funcs.iter().map(|(a, _)| *a).collect();
        let mut sorted = addresses.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, addresses, "addresses are unique and in order");
        assert!(addresses
            .iter()
            .all(|a| (0x0054_7650..0x0055_2470).contains(a)));
        assert_eq!(addresses[0], 0x0054_7650);
        assert_eq!(*addresses.last().unwrap(), 0x0054_b5b0);
    }
}
