//! `fallout shared/pathfinding/navmeshobstaclemanager.cpp` (Xbox PDB source unit), subsystem `fallout shared/pathfinding`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds `NavMeshObstacleManager` (one instance, a function-local
//! static at `011d71e0` made by `006c0720`), its helper classes
//! (`ReferenceObstacleArray`, `ObstacleData`, the two Havok listeners) and
//! the template instances they use (`NiTMap`, `BSSimpleList`, lock-free
//! queues).
//!
//! Session 1 (b0328) translated the 40 lowest-address open functions, up to
//! and including `006c1dd0`; the queue continues at `006c1e00`.
//!
//! Conventions of this file:
//!
//! - The compiler built this unit without optimisation, so every accessor
//!   (`NiPointer::operator->`, `BSSimpleArray::operator[]`, `GetSize`, ...)
//!   is a real call. The translations call each of them by address through
//!   the small wrappers below, as the game does; the docs of the constants
//!   say what the body of the callee does, which is all that is known of it.
//! - A local the game keeps on its stack and passes by address (a
//!   `NiPointer`, an iterator cursor) is a slot in game memory
//!   ([`Engine::with_stack`]). `NiPointer<T>` is one word: the pointer.
//! - Several callees take no parameters although the caller pushed a word
//!   before calling them (`006c0720`, `00b00a00`): the word stays on the
//!   stack and is the argument of the next call
//!   (`PUSH x; CALL getter; MOV ECX,EAX; CALL method`). The translations
//!   pass it to that next call.
//! - The compiler's exception-unwinding frames are not translated.
//!
//! Offsets of the classes are the PC ones: the Xbox PDB's with the 0x10
//! bytes of 16-byte alignment padding removed after the first fields.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleArray, BSSimpleList, NiTMap, NiTPointerMap};

layout! {
    /// `NavMeshObstacleManager` (Xbox PDB), PC offsets. The real object is a
    /// static of `0x1c0` bytes (`011d71e0`..`011d73a0`, the word after it is
    /// the initialisation guard of `006c0720`); the size here is the end of
    /// the last field.
    pub struct NavMeshObstacleManager: 0x1a4 {
        /// `UpdateCriticalSection` (Xbox PDB): built by `0044fae0`, torn
        /// down by `0044fb00`.
        0x00 UpdateCriticalSection: Inline<()>,
        /// `bUpdateAllObstacles` (Xbox PDB).
        0x18 bUpdateAllObstacles: bool,
        /// `FormIDMap` (Xbox PDB): `NiTMap<unsigned int, NiPointer<ReferenceObstacleArray>>`
        /// (`0x25` hash buckets).
        0x1c FormIDMap: Inline<NiTMap>,
        /// `QueuedListOfRefsToAdd` (Xbox PDB): `BSSimpleList<TESObjectREFR *>`.
        0x2c QueuedListOfRefsToAdd: Inline<BSSimpleList>,
        /// `QueuedListOfRefsToRemove` (Xbox PDB): `BSSimpleList<unsigned int>`.
        0x34 QueuedListOfRefsToRemove: Inline<BSSimpleList>,
        /// `ObstaclesUpdateList` (Xbox PDB): `BSSimpleList<NiPointer<ObstacleData>>`.
        0x3c ObstaclesUpdateList: Inline<BSSimpleList>,
        /// `ObstaclesToAddToUpdateList` (Xbox PDB): a `LockFreeQueue<bhkRigidBody *>`
        /// (built by `006c6450(7, 8)`, emptied by `006c6700`).
        0x60 ObstaclesToAddToUpdateList: Inline<()>,
        /// `ObstaclesToRemoveFromUpdateList` (Xbox PDB): a `LockFreeQueue<bhkRigidBody *>`.
        0xa0 ObstaclesToRemoveFromUpdateList: Inline<()>,
        /// `RigidBodyToObstacleMap` (Xbox PDB): `NiTMap<bhkRigidBody *, NiPointer<ObstacleData>>`.
        0xe0 RigidBodyToObstacleMap: Inline<NiTMap>,
        /// `QueuedListOfDoorsToAdd` (Xbox PDB): `BSSimpleList<TESObjectREFR *>`.
        0xf0 QueuedListOfDoorsToAdd: Inline<BSSimpleList>,
        /// `QueuedListOfDoorsToRemove` (Xbox PDB): `BSSimpleList<unsigned int>`.
        0xf8 QueuedListOfDoorsToRemove: Inline<BSSimpleList>,
        /// `OpenDoorMap` (Xbox PDB): `NiTMap<unsigned int, NiPointer<ReferenceObstacleArray>>`.
        0x100 OpenDoorMap: Inline<NiTMap>,
        /// `ClosedDoorMap` (Xbox PDB): the same kind of map.
        0x110 ClosedDoorMap: Inline<NiTMap>,
        /// `pListener` (Xbox PDB): `bhkObstacleDeactivationListener *`.
        0x120 pListener: u32,
        /// `pRemoveListener` (Xbox PDB): `bhkObstacleRemovalListener *`.
        0x124 pRemoveListener: u32,
        /// `ObstacleTaskletGroup` (Xbox PDB): a `BSTaskletGroup`, made by
        /// `BSTaskletManager::CreateTaskGroup` (`00b00a80`).
        0x128 ObstacleTaskletGroup: Inline<()>,
        /// `CurrentNavMeshTaskMap` (Xbox PDB): `NiTMap<unsigned int, ObstacleTaskData *>`.
        0x12c CurrentNavMeshTaskMap: Inline<NiTMap>,
        /// `BackgroundTasks` (Xbox PDB): `BSSimpleArray<ObstacleTaskData *, 1024>`.
        0x13c BackgroundTasks: Inline<BSSimpleArray>,
        /// `ProcessedTasks` (Xbox PDB): `BSSimpleArray<ObstacleTaskData *, 1024>`.
        0x14c ProcessedTasks: Inline<BSSimpleArray>,
        /// Built with `0096a2d0` (the list constructor) and never torn down;
        /// the Xbox PDB has a `ProcessedTaskLock` (`BSSpinLock`) between the
        /// tasks and the navmesh lists, which this may be.
        0x160 ProcessedTaskLock: Inline<()>,
        /// `QueuedListOfNavmeshesToEnable` (Xbox PDB): `BSSimpleList<unsigned int>`.
        0x180 QueuedListOfNavmeshesToEnable: Inline<BSSimpleList>,
        /// `QueuedListOfNavmeshesToDisable` (Xbox PDB): `BSSimpleList<unsigned int>`.
        0x188 QueuedListOfNavmeshesToDisable: Inline<BSSimpleList>,
        /// `eState` (Xbox PDB): `OBSTACLE_MANAGER_BACKGROUND_STATE`.
        0x190 eState: u32,
        /// `fTimeToNextSwap` (Xbox PDB).
        0x194 fTimeToNextSwap: f32,
        /// `bDrawObstacles` (Xbox PDB).
        0x198 bDrawObstacles: bool,
        /// `spObstacleRootNode` (Xbox PDB): `NiPointer<NiNode>`.
        0x19c spObstacleRootNode: u32,
        /// `MainThreadPerformaceTimer` (Xbox PDB), registered by the
        /// constructor with `BSPerformanceTimerManager::Register`.
        0x1a0 MainThreadPerformaceTimer: Inline<()>,
        /// `TaskletsPerformaceTimer` (Xbox PDB).
        0x1a1 TaskletsPerformaceTimer: Inline<()>,
    }

    /// `ReferenceObstacleArray` (Xbox PDB), `0x1c` bytes on both builds: a
    /// reference-counted base, the form id of the reference and the array of
    /// its obstacles.
    pub struct ReferenceObstacleArray: 0x1c {
        /// `iFormID` (Xbox PDB).
        0x08 iFormID: u32,
        /// `Obstacles` (Xbox PDB): `BSSimpleArray<NiPointer<ObstacleData>, 1024>`.
        0x0c Obstacles: Inline<BSSimpleArray>,
    }

    /// `ObstacleData` (Xbox PDB), `0x8c` bytes on PC (the allocation size in
    /// `006c2490`); the fields used so far, at PC offsets (the Xbox PDB's
    /// `+0x84`/`+0x88` are `0x10` lower here).
    pub struct ObstacleData: 0x8c {
        /// `spParentArray` (Xbox PDB): `ReferenceObstacleArray *`.
        0x08 spParentArray: u32,
        /// `pRigidBody` (Xbox PDB): `NiPointer<bhkRigidBody>`.
        0x0c pRigidBody: u32,
        /// `bActive` (Xbox PDB).
        0x74 bActive: bool,
        /// `Navmeshes` (Xbox PDB): `BSSimpleArray<NavMeshInfo *, 1024>`.
        0x78 Navmeshes: Inline<BSSimpleArray>,
    }

    /// One item of an `NiTMap` bucket chain (`NiTMapItem<K, V>`): next item,
    /// key, value.
    pub struct NiTMapItem: 0x0c {
        /// `m_pkNext`.
        0x00 m_pkNext: u32,
        /// `m_key`.
        0x04 m_key: u32,
        /// `m_val`.
        0x08 m_val: u32,
    }
}

/// The manager instance (a function-local static) and the word holding the
/// bit that says it has been constructed.
const MANAGER_INSTANCE: u32 = 0x011d_71e0;
const MANAGER_INSTANCE_GUARD: u32 = 0x011d_73a0;
/// The atexit destructor registered for the instance.
const MANAGER_ATEXIT_DESTRUCTOR: u32 = 0x00fd_7810;
/// `_atexit` (`cdecl`).
const ATEXIT: u32 = 0x00ec_658f;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\Pathfinding\NavMeshObstacleManager.cpp"`.
const SOURCE_PATH: u32 = 0x0106_c3f0;
/// Debug scope guard (a 4-byte local): constructor (`thiscall`: category
/// byte, flag, file name, line), destructor.
const SCOPE_GUARD_CTOR: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DTOR: u32 = 0x0040_4ee0;
const SCOPE_CATEGORY: u32 = 0x2e;
/// `BSPerformanceTimerManager::Register` (Xbox PDB), on the timer byte:
/// category string, timer name, two flags.
const PERFORMANCE_TIMER_REGISTER: u32 = 0x0045_d260;
/// `"Pathing"`, `"Obstacle Manager Update"`, `"Navmesh Background Update"`.
const PATHING_CATEGORY_NAME: u32 = 0x0103_a1bc;
const MAIN_TIMER_NAME: u32 = 0x0106_c468;
const TASKLET_TIMER_NAME: u32 = 0x0106_c44c;
/// `fTimeToNextSwap`'s initial value (`float`).
const INITIAL_TIME_TO_NEXT_SWAP: u32 = 0x0106_c398;

/// Vtables set by the constructors of the listeners and of
/// `ReferenceObstacleArray`.
const DEACTIVATION_LISTENER_VTABLE: u32 = 0x0106_c3b4;
const REMOVAL_LISTENER_VTABLE: u32 = 0x0106_c3d4;
const REFERENCE_OBSTACLE_ARRAY_VTABLE: u32 = 0x0106_c484;
/// Constructors of the bases of the listeners (`hkpEntityActivationListener`
/// and `hkpEntityListener`, as `thiscall` on the object) and the destructor
/// of the second.
const ACTIVATION_LISTENER_BASE_CTOR: u32 = 0x006c_0100;
const ENTITY_LISTENER_BASE_CTOR: u32 = 0x0061_f580;
const ENTITY_LISTENER_BASE_DTOR: u32 = 0x0055_4650;

/// `operator new(size)` and `operator delete(ptr)` (`cdecl`), and the
/// sized forms `00aa13e0(size)` / `00aa1460(ptr, size)` (`cdecl`).
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const OPERATOR_NEW_SIZED: u32 = 0x00aa_13e0;
const OPERATOR_DELETE_SIZED: u32 = 0x00aa_1460;

/// `NiPointer<T>`: constructor from a raw pointer (`thiscall`, adds a
/// reference), release (drops one), `operator=(T *)` and
/// `operator=(const NiPointer &)`. The engine map names the last one
/// `NiPointer<PathingDebugGeometryData>::operator=`; it takes the address of
/// the source pointer.
const NI_POINTER_INIT: u32 = 0x0063_3c90;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
const NI_POINTER_SET: u32 = 0x0066_b0d0;
const NI_POINTER_ASSIGN: u32 = 0x006e_5cc0;
/// Copy constructor `NiPointer(const NiPointer &)`: takes the address of the
/// source pointer and adds a reference.
const NI_POINTER_COPY: u32 = 0x0055_9a40;
/// `NiPointer::operator->` / `operator T *`: the first word of the object.
const NI_POINTER_GET: u32 = 0x0055_9450;
/// `00822510`: the first word equals the argument.
const FIRST_WORD_EQUALS: u32 = 0x0082_2510;

/// `BSSimpleArray<T, 1024>`: size (`+8`), element address (`buffer + 4 *
/// index`, argument: the index), `Add(const T &)` (the engine map names
/// `007cb2e0` `AddUninitialized` of another instance; the body adds the
/// element at the argument's address), `Clear(bool)` (`008454f0`) and
/// `IsEmpty` (`0076b610`: the size is zero).
const ARRAY_SIZE: u32 = 0x0044_ddc0;
const ARRAY_AT: u32 = 0x0087_7a30;
const ARRAY_ADD: u32 = 0x007c_b2e0;
const ARRAY_CLEAR: u32 = 0x0084_54f0;
const ARRAY_IS_EMPTY: u32 = 0x0076_b610;
/// `BSSimpleArray<NavMesh *, 1024>` local (`0x10` bytes): constructor and
/// destructor.
const NAVMESH_ARRAY_CTOR: u32 = 0x006c_6c60;
const NAVMESH_ARRAY_DTOR: u32 = 0x006c_6ce0;
/// `BSSimpleList<T>` on a head node: `Contains(const T &)`, `AddHead`,
/// `AddTail` and `Remove` (observed bodies; each takes the address of the
/// value), and `IsEmpty` (`004a4460`: no next node and no item).
const LIST_CONTAINS: u32 = 0x005f_65d0;
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
const LIST_ADD_TAIL: u32 = 0x0090_5820;
const LIST_REMOVE: u32 = 0x0090_5330;
const LIST_IS_EMPTY: u32 = 0x004a_4460;
/// `00631820`: the node's item differs from the value at the argument's
/// address. `004ee360`: the node's deleting destructor (flag 1 frees it).
const LIST_NODE_DIFFERS: u32 = 0x0063_1820;
const LIST_NODE_DELETE: u32 = 0x004e_e360;
/// `NiTMap<K, V>` instances of this unit: `GetAt(key, out)` (`006c62d0`, the
/// out argument is the address of a `NiPointer`; `AL` says whether the key
/// is present), `SetAt(key, value)` (`006c6920`, the value is an `NiPointer`
/// passed by value, which the callee destroys), `RemoveAt(key)` (`00405430`)
/// and the first item of the map (`004b9ba0`).
const MAP_GET: u32 = 0x006c_62d0;
const MAP_SET: u32 = 0x006c_6920;
const MAP_REMOVE: u32 = 0x0040_5430;
const MAP_FIRST_ITEM: u32 = 0x004b_9ba0;
/// `NiTMap` allocator: frees an item (`00659080`, argument: the item).
const MAP_FREE_ITEM: u32 = 0x0065_9080;
/// `TESForm::GetFormID` (`0084e3a0`: the word at `+0x0c`), the form lookup
/// by id (`004839c0`, `cdecl`), the base form of a reference (`007af430`:
/// the word at `+0x20`), the form type byte (`00401170`: the byte at `+4`)
/// and `TESObjectDOOR::IsSlidingDoor` (`00518080`).
const FORM_ID: u32 = 0x0084_e3a0;
const FORM_LOOKUP_BY_ID: u32 = 0x0048_39c0;
const REFERENCE_BASE_FORM: u32 = 0x007a_f430;
const FORM_TYPE: u32 = 0x0040_1170;
const IS_SLIDING_DOOR: u32 = 0x0051_8080;
/// The form type that `006c0dc0`, `006c0f10` and `006c1060` check before
/// asking `IsSlidingDoor`.
const DOOR_FORM_TYPE: u32 = 0x1c;
/// `00408d60` on the setting object at `011d73e4`: the address of its value
/// (`+4`); the obstacle queues do nothing while that byte is zero.
const SETTING_VALUE_ADDRESS: u32 = 0x0040_8d60;
const OBSTACLES_SETTING: u32 = 0x011d_73e4;
/// `004b5a80` (`cdecl`): the word at `+0x0c` of the entity a Havok listener
/// callback gets (null for null), which is the entity's user data.
const ENTITY_USER_DATA: u32 = 0x004b_5a80;
/// `004ae750` on a rigid body: the Havok entity it wraps (null for null),
/// and the two registrations on it (`hkpEntity` functions of
/// `hkpentity.cpp`; the argument is the listener). `bhkRigidBody::IsActive`
/// (Xbox PDB, `005609b0`).
const RIGID_BODY_ENTITY: u32 = 0x004a_e750;
const ENTITY_ADD_ACTIVATION_LISTENER: u32 = 0x00c9_c700;
const ENTITY_ADD_ENTITY_LISTENER: u32 = 0x00c9_c5b0;
const RIGID_BODY_IS_ACTIVE: u32 = 0x0056_09b0;
/// `BSTaskletManager`: the instance getter (`00b00a00`, no parameters),
/// `CreateTaskGroup`, `DestroyTaskGroup` (Xbox PDB) and the two group
/// functions `00b00ae0` / `00b00bc0`, all taking the group's address.
const TASKLET_MANAGER_INSTANCE: u32 = 0x00b0_0a00;
const TASKLET_CREATE_TASK_GROUP: u32 = 0x00b0_0a80;
const TASKLET_GROUP_CHECK: u32 = 0x00b0_0ae0;
const TASKLET_GROUP_FLUSH: u32 = 0x00b0_0bc0;
const TASKLET_DESTROY_TASK_GROUP: u32 = 0x00b0_0c20;
/// `006c5ef0(this, navmeshinfo *)`: the `ObstacleTaskData` for a navmesh
/// (made and entered into `CurrentNavMeshTaskMap` when missing; the map key
/// is the first word of the argument, `NavMeshInfo::NavMeshID`), and
/// `ObstacleTaskData::AddOperation` (Xbox PDB, `006c7a30`): the address of
/// the obstacle's `NiPointer` and the operation number.
const OBSTACLE_TASK_FOR_NAVMESH: u32 = 0x006c_5ef0;
const OBSTACLE_TASK_ADD_OPERATION: u32 = 0x006c_7a30;
/// `0068efb0` on a navmesh: the word at `+0x104` (its `NavMeshInfo`).
const NAVMESH_INFO: u32 = 0x0068_efb0;
/// Navmeshes overlapped by an obstacle: `006c2050(&obstacle, &array)`
/// (`cdecl`, `AL`).
const OBSTACLE_OVERLAPPING_NAVMESHES: u32 = 0x006c_2050;
/// `006c2650(obstacle)`, `006c3440(&obstacle)` and `006c3520(arg, &array)`
/// (all `cdecl`): obstacle updates of later sessions.
const OBSTACLE_PREPARE: u32 = 0x006c_2650;
const OBSTACLE_REFRESH: u32 = 0x006c_3440;
const OBSTACLE_ARRAY_REFRESH: u32 = 0x006c_3520;
/// Debug drawing: `DrawObstacles(this, flag)`, `AddBoundingBox3DNode(this,
/// &obstacle)` and the removal of a node (`006c5c50(this, &obstacle)`).
const DRAW_OBSTACLES: u32 = 0x006c_5460;
const ADD_BOUNDING_BOX_NODE: u32 = 0x006c_5b20;
const REMOVE_BOUNDING_BOX_NODE: u32 = 0x006c_5c50;
/// `006c1e00` (`this`, rigid body): not translated in this session.
const REMOVE_RIGID_BODY: u32 = 0x006c_1e00;
/// The lock-free queue operations: push (`006c5ff0`, address of the value)
/// and clear (`006c6700`, byte flag).
const QUEUE_PUSH: u32 = 0x006c_5ff0;
const QUEUE_CLEAR: u32 = 0x006c_6700;
/// A navmesh holder local (`0042fb00` constructs it, `0042fa40` destroys
/// it), the `NavMeshInfo` function filling one from `info + 0x54`
/// (`00690860(info, &holder)`: `AL` true when the info has a navmesh) and
/// a function on the navmesh (`006993c0`).
const NAVMESH_HOLDER_CTOR: u32 = 0x0042_fb00;
const NAVMESH_HOLDER_DTOR: u32 = 0x0042_fa40;
const NAVMESH_INFO_LOAD: u32 = 0x0069_0860;
const NAVMESH_UTIL_CALL: u32 = 0x0069_93c0;
/// `ReferenceObstacleArray` helpers: base constructor/destructor (`004968b0`
/// / `00496910`), `BSSimpleArray` constructor/destructor (`006c6160` /
/// `006c6190`).
const REFERENCE_COUNTED_CTOR: u32 = 0x0049_68b0;
const REFERENCE_COUNTED_DTOR: u32 = 0x0049_6910;
const OBSTACLE_ARRAY_CTOR: u32 = 0x006c_6160;
const OBSTACLE_ARRAY_DTOR: u32 = 0x006c_6190;
/// Fills a `NiPointer<ReferenceObstacleArray>` for a reference:
/// `006c34f0(reference, &pointer)` (`cdecl`).
const FILL_REFERENCE_OBSTACLES: u32 = 0x006c_34f0;
/// Constructors and destructors of the manager's members.
const CRITICAL_SECTION_CTOR: u32 = 0x0044_fae0;
const CRITICAL_SECTION_DTOR: u32 = 0x0044_fb00;
const MAP_CTOR: u32 = 0x006c_5fc0;
const MAP_DTOR: u32 = 0x006c_6350;
const TASK_MAP_CTOR: u32 = 0x006c_60a0;
const TASK_MAP_DTOR: u32 = 0x006c_6b80;
const RIGID_BODY_MAP_CTOR: u32 = 0x006c_6070;
const RIGID_BODY_MAP_DTOR: u32 = 0x006c_6a80;
const REFS_LIST_CTOR: u32 = 0x0096_a2d0;
const REFS_LIST_DTOR: u32 = 0x0046_ffb0;
const UPDATE_LIST_CTOR: u32 = 0x004e_e810;
const UPDATE_LIST_DTOR: u32 = 0x004e_e840;
const QUEUE_CTOR: u32 = 0x006c_6450;
const QUEUE_DTOR: u32 = 0x006c_6680;
const TASK_ARRAY_CTOR: u32 = 0x006c_6c10;
const TASK_ARRAY_DTOR: u32 = 0x006c_6c40;
const TASKLET_GROUP_CTOR: u32 = 0x0044_dee0;
const TASKLET_GROUP_DTOR: u32 = 0x0048_3710;
/// Clears of `ClearAllObstacles`: maps (`00438af0`), lists of ids
/// (`00470470`), the update list (`004ee920`).
const MAP_CLEAR: u32 = 0x0043_8af0;
const LIST_CLEAR: u32 = 0x0047_0470;
const UPDATE_LIST_CLEAR: u32 = 0x004e_e920;

/// The word of game memory at `address` (an `NiPointer` slot, a field).
fn word(e: &Engine, address: u32) -> u32 {
    e.mem.u32(address)
}

/// `NiPointer::operator->`: the pointer held in the slot.
fn pointer_get(e: &mut Engine, slot: u32) -> u32 {
    e.call(NI_POINTER_GET, &args![slot]).u32()
}

fn pointer_init(e: &mut Engine, slot: u32, value: u32) {
    e.call(NI_POINTER_INIT, &args![slot, value]);
}

fn pointer_release(e: &mut Engine, slot: u32) {
    e.call(NI_POINTER_RELEASE, &args![slot]);
}

/// `NiPointer(const NiPointer &)` into a fresh slot.
fn pointer_copy(e: &mut Engine, slot: u32, source: u32) {
    e.call(NI_POINTER_COPY, &args![slot, source]);
}

/// `BSSimpleArray::GetSize`.
fn array_size(e: &mut Engine, array: u32) -> u32 {
    e.call(ARRAY_SIZE, &args![array]).u32()
}

/// `BSSimpleArray::operator[]`: the address of element `index`.
fn array_at(e: &mut Engine, array: u32, index: u32) -> u32 {
    e.call(ARRAY_AT, &args![array, index]).u32()
}

/// The `Obstacles` array of the `ReferenceObstacleArray` held by `slot`.
fn obstacles_of(e: &mut Engine, slot: u32) -> u32 {
    pointer_get(e, slot) + ReferenceObstacleArray::Obstacles.off
}

/// The `Navmeshes` array of the `ObstacleData` held by `slot`.
fn navmeshes_of(e: &mut Engine, slot: u32) -> u32 {
    pointer_get(e, slot) + ObstacleData::Navmeshes.off
}

/// Number of obstacles in the `ReferenceObstacleArray` held by `slot`.
fn obstacle_count(e: &mut Engine, slot: u32) -> u32 {
    let obstacles = obstacles_of(e, slot);
    array_size(e, obstacles)
}

/// Number of navmeshes recorded by the `ObstacleData` held by `slot`.
fn navmesh_count(e: &mut Engine, slot: u32) -> u32 {
    let navmeshes = navmeshes_of(e, slot);
    array_size(e, navmeshes)
}

/// The rigid body of the `ObstacleData` held by `slot`.
fn rigid_body_of(e: &mut Engine, slot: u32) -> u32 {
    let obstacle = pointer_get(e, slot);
    pointer_get(e, obstacle + ObstacleData::pRigidBody.off)
}

/// Whether the `ObstacleData` held by `slot` is active.
fn obstacle_is_active(e: &mut Engine, slot: u32) -> bool {
    let obstacle = pointer_get(e, slot);
    e.mem.u8(obstacle + ObstacleData::bActive.off) != 0
}

/// `list.Contains(value)` for a word value (the game passes its address).
fn list_contains(e: &mut Engine, list: u32, value: u32) -> bool {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(LIST_CONTAINS, &args![list, slot]).bool()
    })
}

/// A list or queue operation that takes the address of a word value.
fn list_operation(e: &mut Engine, operation: u32, list: u32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(operation, &args![list, slot]);
    })
}

/// `map.GetAt(key, &out)`: true when the key is present; the value is put
/// in the `NiPointer` at `out`.
fn map_get(e: &mut Engine, map: u32, key: u32, out: u32) -> bool {
    e.call(MAP_GET, &args![map, key, out]).bool()
}

/// `map.SetAt(key, value)` with the value a fresh copy of the `NiPointer` at
/// `source` (the game builds the by-value argument on its stack with the
/// copy constructor; `SetAt` destroys it).
fn map_set_copy(e: &mut Engine, map: u32, key: u32, source: u32) {
    e.with_stack(4, |e, temporary| {
        pointer_copy(e, temporary.addr(), source);
        let value = word(e, temporary.addr());
        e.call(MAP_SET, &args![map, key, value]);
    })
}

/// The value of the setting at `011d73e4` (a byte); the queues do nothing
/// while it is zero.
fn obstacles_enabled(e: &mut Engine) -> bool {
    let value = e
        .call(SETTING_VALUE_ADDRESS, &args![OBSTACLES_SETTING])
        .u32();
    e.mem.u8(value) != 0
}

/// A scope guard around `body`, as `00404eb0` / `00404ee0` make it.
fn with_scope_guard<R>(e: &mut Engine, line: u32, body: impl FnOnce(&mut Engine) -> R) -> R {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, SCOPE_CATEGORY, 1u32, SOURCE_PATH, line],
        );
        let result = body(e);
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
        result
    })
}

/// `ObstacleTaskData::AddOperation` on the task of `nav_mesh_info`.
fn queue_operation(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
    nav_mesh_info: u32,
    obstacle: u32,
    operation: u32,
) {
    let task = e
        .call(OBSTACLE_TASK_FOR_NAVMESH, &args![this, nav_mesh_info])
        .u32();
    e.call(
        OBSTACLE_TASK_ADD_OPERATION,
        &args![task, obstacle, operation],
    );
}

/// For an obstacle that is active: asks `006c2050` for the navmeshes it
/// overlaps; for each one queues `operation` on its task and records its
/// `NavMeshInfo` in the obstacle's `Navmeshes`.
fn queue_overlapping_navmeshes(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
    obstacle: u32,
    operation: u32,
) {
    e.with_stack(0x10, |e, found| {
        let found = found.addr();
        e.call(NAVMESH_ARRAY_CTOR, &args![found]);
        if e.call(OBSTACLE_OVERLAPPING_NAVMESHES, &args![obstacle, found])
            .bool()
        {
            let mut index = 0;
            while index < array_size(e, found) {
                let element = array_at(e, found, index);
                let navmesh = pointer_get(e, element);
                let info = e.call(NAVMESH_INFO, &args![navmesh]).u32();
                queue_operation(e, this, info, obstacle, operation);
                let element = array_at(e, found, index);
                let navmesh = pointer_get(e, element);
                let info = e.call(NAVMESH_INFO, &args![navmesh]).u32();
                let navmeshes = navmeshes_of(e, obstacle);
                list_operation(e, ARRAY_ADD, navmeshes, info);
                index += 1;
            }
        }
        e.call(NAVMESH_ARRAY_DTOR, &args![found]);
    });
}

/// Queues `operation` on the task of every `NavMeshInfo` in the obstacle's
/// `Navmeshes`, then clears that array (`Clear(1)`).
fn queue_recorded_navmeshes(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
    obstacle: u32,
    operation: u32,
) {
    let mut index = 0;
    while index < navmesh_count(e, obstacle) {
        let navmeshes = navmeshes_of(e, obstacle);
        let element = array_at(e, navmeshes, index);
        let info = word(e, element);
        queue_operation(e, this, info, obstacle, operation);
        index += 1;
    }
    let navmeshes = navmeshes_of(e, obstacle);
    e.call(ARRAY_CLEAR, &args![navmeshes, 1u32]);
}

// Translated from 00559120 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTMapBase<DFALL<NiTMapItem<unsigned int, NiPointer<ReferenceObstacleArray>>>, unsigned int,
/// NiPointer<ReferenceObstacleArray>>::GetNext` (Xbox PDB): reads the item
/// at `*position`, gives its key and (through `NiPointer::operator=`) its
/// value, and moves `*position` to the next item: the rest of the bucket
/// chain, else the first item of a later bucket (the bucket of the key comes
/// from the map's hash function, vtable slot `+4`), else null.
pub fn ni_tmap_base_reference_obstacle_array_get_next(
    e: &mut Engine,
    this: Ptr<NiTMap>,
    position: Ptr,
    key_out: Ptr,
    value_out: Ptr,
) {
    let item: Ptr<NiTMapItem> = Ptr::new(word(e, position.addr()));
    let key = e.get(item, NiTMapItem::m_key);
    e.mem.set_u32(key_out.addr(), key);
    e.call(
        NI_POINTER_ASSIGN,
        &args![value_out, item.addr() + NiTMapItem::m_val.off],
    );
    let next = e.get(item, NiTMapItem::m_pkNext);
    if next != 0 {
        e.mem.set_u32(position.addr(), next);
        return;
    }
    let mut index = e.vcall(this.addr(), 4, &args![key]).u32();
    loop {
        index = index.wrapping_add(1);
        if index >= e.get(this, NiTPointerMap::m_uiHashSize) {
            e.mem.set_u32(position.addr(), 0);
            return;
        }
        let table = e.get(this, NiTPointerMap::m_ppkHashTable);
        let candidate = word(e, table + index * 4);
        if candidate != 0 {
            e.mem.set_u32(position.addr(), candidate);
            return;
        }
    }
}

// Translated from 00631620 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSSimpleList<NiPointer<ObstacleData>>::Remove` (Xbox PDB): removes the
/// first node whose item equals the pointer at `item`. Does nothing when
/// that pointer is null or the list is empty. The head node stays; when it
/// is the match it takes the next node's item and that node is freed.
pub fn bs_simple_list_obstacle_data_remove(e: &mut Engine, this: Ptr<BSSimpleList>, item: Ptr) {
    if e.call(FIRST_WORD_EQUALS, &args![item, 0u32]).bool() {
        return;
    }
    if e.call(LIST_IS_EMPTY, &args![this]).bool() {
        return;
    }
    let head = this.addr();
    let mut previous = head;
    let mut node = head;
    while node != 0 && e.call(LIST_NODE_DIFFERS, &args![node, item]).bool() {
        previous = node;
        node = e.mem.u32(node + BSSimpleList::m_pkNext.off);
    }
    if node == 0 {
        return;
    }
    if node == head {
        let next = e.get(this, BSSimpleList::m_pkNext);
        if next == 0 {
            e.call(NI_POINTER_SET, &args![this, 0u32]);
        } else {
            let after = e.mem.u32(next + BSSimpleList::m_pkNext.off);
            e.set(this, BSSimpleList::m_pkNext, after);
            e.call(NI_POINTER_ASSIGN, &args![this, next]);
            e.mem.set_u32(next + BSSimpleList::m_pkNext.off, 0);
            e.call(LIST_NODE_DELETE, &args![next, 1u32]);
        }
    } else {
        let after = e.mem.u32(node + BSSimpleList::m_pkNext.off);
        e.mem.set_u32(previous + BSSimpleList::m_pkNext.off, after);
        e.mem.set_u32(node + BSSimpleList::m_pkNext.off, 0);
        e.call(LIST_NODE_DELETE, &args![node, 1u32]);
    }
}

// Translated from 00691a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTMap<unsigned int, NiPointer<ReferenceObstacleArray>>::DeleteItem`
/// (Xbox PDB): releases the item's value and frees the item through the
/// map's allocator (`+0x0c`).
pub fn ni_tmap_reference_obstacle_array_delete_item(
    e: &mut Engine,
    this: Ptr<NiTMap>,
    item: Ptr<NiTMapItem>,
) {
    e.call(
        NI_POINTER_SET,
        &args![item.addr() + NiTMapItem::m_val.off, 0u32],
    );
    e.call(MAP_FREE_ITEM, &args![this.addr() + 0x0c, item]);
}

// Translated from 006a63e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTMapBase<DFALL<NiTMapItem<bhkRigidBody *, NiPointer<ObstacleData>>>,
/// bhkRigidBody *, NiPointer<ObstacleData>>::SetValue` (Xbox PDB): stores the
/// key and assigns the value to the item. The value is an `NiPointer` passed
/// by value, which the function destroys at its end.
pub fn ni_tmap_base_rigid_body_obstacle_data_set_value(
    e: &mut Engine,
    _this: Ptr<NiTMap>,
    item: Ptr<NiTMapItem>,
    key: u32,
    value: u32,
) {
    e.set(item, NiTMapItem::m_key, key);
    e.with_stack(4, |e, argument| {
        e.mem.set_u32(argument.addr(), value);
        e.call(
            NI_POINTER_ASSIGN,
            &args![item.addr() + NiTMapItem::m_val.off, argument],
        );
        e.call(NI_POINTER_RELEASE, &args![argument]);
    });
}

// Translated from 006c00b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `bhkObstacleDeactivationListener` (engine map: unnamed):
/// the base constructor `006c0100`, then the class's vtable. Returns `this`.
pub fn fn_006c00b0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(ACTIVATION_LISTENER_BASE_CTOR, &args![this]);
    e.mem.set_u32(this.addr(), DEACTIVATION_LISTENER_VTABLE);
    this
}

// Translated from 006c0190 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkObstacleDeactivationListener::entityDeactivatedCallback` (Xbox PDB):
/// for an entity with user data, `006c1c90` on the manager with that data.
pub fn bhk_obstacle_deactivation_listener_entity_deactivated_callback(
    e: &mut Engine,
    _this: Ptr,
    entity: u32,
) {
    let user_data = e.call(ENTITY_USER_DATA, &args![entity]).u32();
    if user_data != 0 {
        let manager = fn_006c0720(e);
        fn_006c1c90(e, manager, user_data);
    }
}

// Translated from 006c01d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkObstacleDeactivationListener::entityActivatedCallback` (Xbox PDB):
/// for an entity with user data, `006c1c70` on the manager with that data.
pub fn bhk_obstacle_deactivation_listener_entity_activated_callback(
    e: &mut Engine,
    _this: Ptr,
    entity: u32,
) {
    let user_data = e.call(ENTITY_USER_DATA, &args![entity]).u32();
    if user_data != 0 {
        let manager = fn_006c0720(e);
        fn_006c1c70(e, manager, user_data);
    }
}

// Translated from 006c0210 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `bhkObstacleRemovalListener` (engine map: unnamed): the
/// base constructor `0061f580`, then the class's vtable. Returns `this`.
pub fn fn_006c0210(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(ENTITY_LISTENER_BASE_CTOR, &args![this]);
    e.mem.set_u32(this.addr(), REMOVAL_LISTENER_VTABLE);
    this
}

// Translated from 006c0230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of `bhkObstacleRemovalListener` (engine map:
/// unnamed): the destructor, then `operator delete` when bit 0 of `flags` is
/// set. Returns `this`.
pub fn fn_006c0230(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_006c0260(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 006c0260 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of `bhkObstacleRemovalListener` (engine map: unnamed): sets the
/// class's vtable and runs the base destructor (`00554650`).
pub fn fn_006c0260(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), REMOVAL_LISTENER_VTABLE);
    e.call(ENTITY_LISTENER_BASE_DTOR, &args![this]);
}

// Translated from 006c0280 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkObstacleRemovalListener::entityRemovedCallback` (Xbox PDB): for an
/// entity with user data, `OnObstacleLost3D` on the manager with that data.
pub fn bhk_obstacle_removal_listener_entity_removed_callback(
    e: &mut Engine,
    _this: Ptr,
    entity: u32,
) {
    let user_data = e.call(ENTITY_USER_DATA, &args![entity]).u32();
    if user_data != 0 {
        let manager = fn_006c0720(e);
        nav_mesh_obstacle_manager_on_obstacle_lost3d(e, manager, user_data);
    }
}

// Translated from 006c02c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::NavMeshObstacleManager` (Xbox PDB): builds every
/// member (critical section, the maps with `0x25` buckets, the lists, the two
/// lock-free queues with `(7, 8)`, the tasklet group, a null
/// `spObstacleRootNode`), registers the two performance timers
/// (`"Pathing"`: `"Obstacle Manager Update"` and `"Navmesh Background
/// Update"`), then, inside a debug scope (line `0xb8`), clears the listener
/// pointers, the draw flag, the state and the update-all flag and sets
/// `fTimeToNextSwap` from the float at `0106c398`. Returns `this`.
pub fn nav_mesh_obstacle_manager_nav_mesh_obstacle_manager(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
) -> Ptr<NavMeshObstacleManager> {
    let base = this.addr();
    e.call(CRITICAL_SECTION_CTOR, &args![this]);
    e.call(MAP_CTOR, &args![base + 0x1c, 0x25u32]);
    e.call(REFS_LIST_CTOR, &args![base + 0x2c]);
    e.call(REFS_LIST_CTOR, &args![base + 0x34]);
    e.call(UPDATE_LIST_CTOR, &args![base + 0x3c]);
    e.call(QUEUE_CTOR, &args![base + 0x60, 7u32, 8u32]);
    e.call(QUEUE_CTOR, &args![base + 0xa0, 7u32, 8u32]);
    e.call(RIGID_BODY_MAP_CTOR, &args![base + 0xe0, 0x25u32]);
    e.call(REFS_LIST_CTOR, &args![base + 0xf0]);
    e.call(REFS_LIST_CTOR, &args![base + 0xf8]);
    e.call(MAP_CTOR, &args![base + 0x100, 0x25u32]);
    e.call(MAP_CTOR, &args![base + 0x110, 0x25u32]);
    e.call(TASKLET_GROUP_CTOR, &args![base + 0x128]);
    e.call(TASK_MAP_CTOR, &args![base + 0x12c, 0x25u32]);
    e.call(TASK_ARRAY_CTOR, &args![base + 0x13c]);
    e.call(TASK_ARRAY_CTOR, &args![base + 0x14c]);
    e.call(REFS_LIST_CTOR, &args![base + 0x160]);
    e.call(REFS_LIST_CTOR, &args![base + 0x180]);
    e.call(REFS_LIST_CTOR, &args![base + 0x188]);
    e.call(NI_POINTER_INIT, &args![base + 0x19c, 0u32]);
    e.call(
        PERFORMANCE_TIMER_REGISTER,
        &args![
            base + 0x1a0,
            PATHING_CATEGORY_NAME,
            MAIN_TIMER_NAME,
            1u32,
            1u32
        ],
    );
    e.call(
        PERFORMANCE_TIMER_REGISTER,
        &args![
            base + 0x1a1,
            PATHING_CATEGORY_NAME,
            TASKLET_TIMER_NAME,
            0u32,
            1u32
        ],
    );
    with_scope_guard(e, 0xb8, |e| {
        e.set(this, NavMeshObstacleManager::pListener, 0);
        e.set(this, NavMeshObstacleManager::pRemoveListener, 0);
        e.set(this, NavMeshObstacleManager::bDrawObstacles, false);
        e.set(this, NavMeshObstacleManager::eState, 0);
        let time_to_next_swap = e.global::<f32>(INITIAL_TIME_TO_NEXT_SWAP);
        e.set(
            this,
            NavMeshObstacleManager::fTimeToNextSwap,
            time_to_next_swap,
        );
        e.set(this, NavMeshObstacleManager::bUpdateAllObstacles, false);
    });
    this
}

// Translated from 006c0510 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of `NavMeshObstacleManager` (engine map: unnamed): clears all
/// obstacles, deletes the two listeners (virtual deleting destructor, slot 0,
/// flag 1; the fields keep the dead pointers), then destroys the members in
/// reverse order. The two performance timers and the lock at `+0x160` are
/// not destroyed.
pub fn fn_006c0510(e: &mut Engine, this: Ptr<NavMeshObstacleManager>) {
    nav_mesh_obstacle_manager_clear_all_obstacles(e, this);
    for field in [
        NavMeshObstacleManager::pListener,
        NavMeshObstacleManager::pRemoveListener,
    ] {
        let listener = e.get(this, field);
        if listener != 0 {
            e.vcall(listener, 0, &args![1u32]);
        }
    }
    let base = this.addr();
    e.call(NI_POINTER_RELEASE, &args![base + 0x19c]);
    e.call(REFS_LIST_DTOR, &args![base + 0x188]);
    e.call(REFS_LIST_DTOR, &args![base + 0x180]);
    e.call(TASK_ARRAY_DTOR, &args![base + 0x14c]);
    e.call(TASK_ARRAY_DTOR, &args![base + 0x13c]);
    e.call(TASK_MAP_DTOR, &args![base + 0x12c]);
    e.call(TASKLET_GROUP_DTOR, &args![base + 0x128]);
    e.call(MAP_DTOR, &args![base + 0x110]);
    e.call(MAP_DTOR, &args![base + 0x100]);
    e.call(REFS_LIST_DTOR, &args![base + 0xf8]);
    e.call(REFS_LIST_DTOR, &args![base + 0xf0]);
    e.call(RIGID_BODY_MAP_DTOR, &args![base + 0xe0]);
    e.call(QUEUE_DTOR, &args![base + 0xa0]);
    e.call(QUEUE_DTOR, &args![base + 0x60]);
    e.call(UPDATE_LIST_DTOR, &args![base + 0x3c]);
    e.call(REFS_LIST_DTOR, &args![base + 0x34]);
    e.call(REFS_LIST_DTOR, &args![base + 0x2c]);
    e.call(MAP_DTOR, &args![base + 0x1c]);
    e.call(CRITICAL_SECTION_DTOR, &args![this]);
}

// Translated from 006c0720 (decompiled, FalloutNV.exe 1.4.0.525)
/// The manager's accessor (engine map: unnamed; the function-local static of
/// `NavMeshObstacleManager::GetInstance`): inside a debug scope (line
/// `0xd2`), constructs the instance at `011d71e0` on first use (guard bit 0
/// of `011d73a0`) and registers its `atexit` destructor. Takes no
/// parameters (callers push a word that stays on the stack for their next
/// call).
pub fn fn_006c0720(e: &mut Engine) -> Ptr<NavMeshObstacleManager> {
    with_scope_guard(e, 0xd2, |e| {
        let guard = e.global::<u32>(MANAGER_INSTANCE_GUARD);
        if guard & 1 == 0 {
            let guard = e.global::<u32>(MANAGER_INSTANCE_GUARD);
            e.set_global(MANAGER_INSTANCE_GUARD, guard | 1);
            nav_mesh_obstacle_manager_nav_mesh_obstacle_manager(e, Ptr::new(MANAGER_INSTANCE));
            e.call(ATEXIT, &args![MANAGER_ATEXIT_DESTRUCTOR]);
        }
    });
    Ptr::new(MANAGER_INSTANCE)
}

// Translated from 006c07d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::Initialize` (Xbox PDB): creates the tasklet group
/// (`BSTaskletManager::CreateTaskGroup`), checks it (`006c08e0`), then makes
/// the two listeners (`0x4` bytes each). Returns false when the group
/// cannot be created or checked.
pub fn nav_mesh_obstacle_manager_initialize(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
) -> bool {
    let group = this.addr() + NavMeshObstacleManager::ObstacleTaskletGroup.off;
    let manager = e.call(TASKLET_MANAGER_INSTANCE, &[]).u32();
    if !e
        .call(TASKLET_CREATE_TASK_GROUP, &args![manager, group])
        .bool()
    {
        return false;
    }
    if !fn_006c08e0(e, Ptr::new(group)) {
        return false;
    }
    let memory = e.call(OPERATOR_NEW, &args![4u32]).u32();
    let listener = if memory != 0 {
        fn_006c00b0(e, Ptr::new(memory)).addr()
    } else {
        0
    };
    e.set(this, NavMeshObstacleManager::pListener, listener);
    let memory = e.call(OPERATOR_NEW, &args![4u32]).u32();
    let remove_listener = if memory != 0 {
        fn_006c0210(e, Ptr::new(memory)).addr()
    } else {
        0
    };
    e.set(
        this,
        NavMeshObstacleManager::pRemoveListener,
        remove_listener,
    );
    true
}

// Translated from 006c08e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Check of the manager's tasklet group (engine map: unnamed): the result of
/// `00b00ae0` on the tasklet manager with the group's address.
pub fn fn_006c08e0(e: &mut Engine, this: Ptr) -> bool {
    let manager = e.call(TASKLET_MANAGER_INSTANCE, &[]).u32();
    e.call(TASKLET_GROUP_CHECK, &args![manager, this]).bool()
}

// Translated from 006c0900 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::Release` (Xbox PDB): clears all obstacles,
/// deletes the two listeners (virtual deleting destructor) and nulls the
/// fields, runs `006c09d0` on the tasklet group and destroys it
/// (`BSTaskletManager::DestroyTaskGroup`).
pub fn nav_mesh_obstacle_manager_release(e: &mut Engine, this: Ptr<NavMeshObstacleManager>) {
    nav_mesh_obstacle_manager_clear_all_obstacles(e, this);
    for field in [
        NavMeshObstacleManager::pListener,
        NavMeshObstacleManager::pRemoveListener,
    ] {
        let listener = e.get(this, field);
        if listener != 0 {
            e.vcall(listener, 0, &args![1u32]);
            e.set(this, field, 0);
        }
    }
    let group = this.addr() + NavMeshObstacleManager::ObstacleTaskletGroup.off;
    fn_006c09d0(e, Ptr::new(group));
    let manager = e.call(TASKLET_MANAGER_INSTANCE, &[]).u32();
    e.call(TASKLET_DESTROY_TASK_GROUP, &args![manager, group]);
}

// Translated from 006c09d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Flush of the manager's tasklet group (engine map: unnamed): `00b00bc0` on
/// the tasklet manager with the group's address.
pub fn fn_006c09d0(e: &mut Engine, this: Ptr) {
    let manager = e.call(TASKLET_MANAGER_INSTANCE, &[]).u32();
    e.call(TASKLET_GROUP_FLUSH, &args![manager, this]);
}

// Translated from 006c09f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::ClearAllObstacles` (Xbox PDB): walks `FormIDMap`
/// (`GetNext`), and for every obstacle of every reference array whose form id
/// is not queued for removal calls `006c1e00` with its rigid body; for each
/// navmesh the obstacle recorded asks it to load (`00690860`) and runs
/// `006993c0` on the loaded navmesh. Then clears the maps (`FormIDMap`, both
/// door maps, the rigid body map), the id lists, the update list and the two
/// queues, and, when `bDrawObstacles` is set, calls `DrawObstacles(0)`.
pub fn nav_mesh_obstacle_manager_clear_all_obstacles(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
) {
    let base = this.addr();
    let form_id_map = base + NavMeshObstacleManager::FormIDMap.off;
    let queued_to_remove = base + NavMeshObstacleManager::QueuedListOfRefsToRemove.off;
    e.with_stack(0x20, |e, frame| {
        let cursor = frame.addr();
        let key = frame.addr() + 4;
        let current = frame.addr() + 8;
        let obstacle = frame.addr() + 0x0c;
        let holder = frame.addr() + 0x10;
        pointer_init(e, current, 0);
        let first = e.call(MAP_FIRST_ITEM, &args![form_id_map]).u32();
        e.mem.set_u32(cursor, first);
        while word(e, cursor) != 0 {
            ni_tmap_base_reference_obstacle_array_get_next(
                e,
                Ptr::new(form_id_map),
                Ptr::new(cursor),
                Ptr::new(key),
                Ptr::new(current),
            );
            let mut index = 0;
            while index < obstacle_count(e, current) {
                let obstacles = obstacles_of(e, current);
                let element = array_at(e, obstacles, index);
                pointer_copy(e, obstacle, element);
                if !e.call(LIST_CONTAINS, &args![queued_to_remove, key]).bool() {
                    let rigid_body = rigid_body_of(e, obstacle);
                    e.call(REMOVE_RIGID_BODY, &args![this, rigid_body]);
                }
                let mut nav_index = 0;
                while nav_index < navmesh_count(e, obstacle) {
                    e.call(NAVMESH_HOLDER_CTOR, &args![holder]);
                    let navmeshes = navmeshes_of(e, obstacle);
                    let element = array_at(e, navmeshes, nav_index);
                    let info = word(e, element);
                    if e.call(NAVMESH_INFO_LOAD, &args![info, holder]).bool() {
                        let navmesh = pointer_get(e, holder);
                        e.call(NAVMESH_UTIL_CALL, &args![navmesh]);
                    }
                    e.call(NAVMESH_HOLDER_DTOR, &args![holder]);
                    nav_index += 1;
                }
                pointer_release(e, obstacle);
                index += 1;
            }
        }
        pointer_release(e, current);
    });
    for field in [
        NavMeshObstacleManager::FormIDMap.off,
        NavMeshObstacleManager::OpenDoorMap.off,
        NavMeshObstacleManager::ClosedDoorMap.off,
        NavMeshObstacleManager::RigidBodyToObstacleMap.off,
    ] {
        e.call(MAP_CLEAR, &args![base + field]);
    }
    for field in [
        NavMeshObstacleManager::QueuedListOfDoorsToAdd.off,
        NavMeshObstacleManager::QueuedListOfDoorsToRemove.off,
        NavMeshObstacleManager::QueuedListOfRefsToAdd.off,
        NavMeshObstacleManager::QueuedListOfRefsToRemove.off,
    ] {
        e.call(LIST_CLEAR, &args![base + field]);
    }
    e.call(
        UPDATE_LIST_CLEAR,
        &args![base + NavMeshObstacleManager::ObstaclesUpdateList.off],
    );
    e.call(
        QUEUE_CLEAR,
        &args![
            base + NavMeshObstacleManager::ObstaclesToAddToUpdateList.off,
            0u32
        ],
    );
    e.call(
        QUEUE_CLEAR,
        &args![
            base + NavMeshObstacleManager::ObstaclesToRemoveFromUpdateList.off,
            0u32
        ],
    );
    if e.get(this, NavMeshObstacleManager::bDrawObstacles) {
        e.call(DRAW_OBSTACLES, &args![this, 0u32]);
    }
}

// Translated from 006c0c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::AddObstacleForReference` (Xbox PDB): when the
/// obstacle setting (`011d73e4`) is on and the reference is not in
/// `QueuedListOfRefsToAdd` yet, adds it at the head.
pub fn nav_mesh_obstacle_manager_add_obstacle_for_reference(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
    reference: u32,
) {
    if !obstacles_enabled(e) {
        return;
    }
    let queue = this.addr() + NavMeshObstacleManager::QueuedListOfRefsToAdd.off;
    if !list_contains(e, queue, reference) {
        list_operation(e, LIST_ADD_HEAD, queue, reference);
    }
}

// Translated from 006c0c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::RemoveObstacleForReference` (Xbox PDB): when the
/// obstacle setting is on, takes the reference out of
/// `QueuedListOfRefsToAdd`; unless its form id is already queued for removal,
/// asks `006c1e00` to remove the rigid body of every obstacle recorded for
/// it in `FormIDMap` (when there is an entry) and queues the form id in
/// `QueuedListOfRefsToRemove` (`AddTail`).
pub fn nav_mesh_obstacle_manager_remove_obstacle_for_reference(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
    reference: u32,
) {
    if !obstacles_enabled(e) {
        return;
    }
    let base = this.addr();
    list_operation(
        e,
        LIST_REMOVE,
        base + NavMeshObstacleManager::QueuedListOfRefsToAdd.off,
        reference,
    );
    let queued_to_remove = base + NavMeshObstacleManager::QueuedListOfRefsToRemove.off;
    let form_id = e.call(FORM_ID, &args![reference]).u32();
    if list_contains(e, queued_to_remove, form_id) {
        return;
    }
    e.with_stack(4, |e, array| {
        let array = array.addr();
        pointer_init(e, array, 0);
        let form_id = e.call(FORM_ID, &args![reference]).u32();
        if map_get(
            e,
            base + NavMeshObstacleManager::FormIDMap.off,
            form_id,
            array,
        ) {
            let mut index = 0;
            while index < obstacle_count(e, array) {
                let obstacles = obstacles_of(e, array);
                let element = array_at(e, obstacles, index);
                let obstacle = pointer_get(e, element);
                let rigid_body = pointer_get(e, obstacle + ObstacleData::pRigidBody.off);
                e.call(REMOVE_RIGID_BODY, &args![this, rigid_body]);
                index += 1;
            }
            let form_id = e.call(FORM_ID, &args![reference]).u32();
            list_operation(e, LIST_ADD_TAIL, queued_to_remove, form_id);
        }
        pointer_release(e, array);
    });
}

/// True for a reference whose base form is a door that is not a sliding
/// door (the test `006c0dc0`, `006c0f10` and `006c1060` open with): the
/// form type is `0x1c` and `IsSlidingDoor` is false.
fn is_ordinary_door(e: &mut Engine, reference: u32) -> bool {
    let form = e.call(REFERENCE_BASE_FORM, &args![reference]).u32();
    if e.call(FORM_TYPE, &args![form]).u32() == DOOR_FORM_TYPE {
        return !e.call(IS_SLIDING_DOOR, &args![form]).bool();
    }
    false
}

// Translated from 006c0dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Door handler (engine map: unnamed, between `RemoveObstacleForReference`
/// and `OnDoorClose`): for a non-sliding door reference calls
/// `AddObstacleForReference`; then, unless the reference is in
/// `ClosedDoorMap`, takes it out of `QueuedListOfDoorsToAdd` when it is
/// there; then, unless it is in `OpenDoorMap`, queues its form id in
/// `QueuedListOfDoorsToRemove` (`AddHead`) when not queued yet. That this is
/// the "door opened" handler is inferred from its neighbour `OnDoorClose`
/// (Xbox PDB) and not confirmed.
pub fn fn_006c0dc0(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, reference: u32) {
    let base = this.addr();
    if is_ordinary_door(e, reference) {
        nav_mesh_obstacle_manager_add_obstacle_for_reference(e, this, reference);
    }
    e.with_stack(4, |e, array| {
        let array = array.addr();
        pointer_init(e, array, 0);
        let form_id = e.call(FORM_ID, &args![reference]).u32();
        let in_closed = map_get(
            e,
            base + NavMeshObstacleManager::ClosedDoorMap.off,
            form_id,
            array,
        );
        let doors_to_add = base + NavMeshObstacleManager::QueuedListOfDoorsToAdd.off;
        if !in_closed && list_contains(e, doors_to_add, reference) {
            list_operation(e, LIST_REMOVE, doors_to_add, reference);
        }
        let form_id = e.call(FORM_ID, &args![reference]).u32();
        let in_open = map_get(
            e,
            base + NavMeshObstacleManager::OpenDoorMap.off,
            form_id,
            array,
        );
        if !in_open {
            let queued = base + NavMeshObstacleManager::QueuedListOfDoorsToRemove.off;
            let form_id = e.call(FORM_ID, &args![reference]).u32();
            if !list_contains(e, queued, form_id) {
                let form_id = e.call(FORM_ID, &args![reference]).u32();
                list_operation(e, LIST_ADD_HEAD, queued, form_id);
            }
        }
        pointer_release(e, array);
    });
}

// Translated from 006c0f10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::OnDoorClose` (Xbox PDB): for a non-sliding door
/// reference calls `RemoveObstacleForReference`; then, unless the reference is
/// in `OpenDoorMap`, takes its form id out of `QueuedListOfDoorsToRemove`
/// when it is there, and, unless the reference is in `ClosedDoorMap`, queues
/// the reference in `QueuedListOfDoorsToAdd` (`AddHead`) when it is not there.
pub fn nav_mesh_obstacle_manager_on_door_close(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
    reference: u32,
) {
    let base = this.addr();
    if is_ordinary_door(e, reference) {
        nav_mesh_obstacle_manager_remove_obstacle_for_reference(e, this, reference);
    }
    e.with_stack(4, |e, array| {
        let array = array.addr();
        pointer_init(e, array, 0);
        let form_id = e.call(FORM_ID, &args![reference]).u32();
        let in_open = map_get(
            e,
            base + NavMeshObstacleManager::OpenDoorMap.off,
            form_id,
            array,
        );
        if !in_open {
            let queued = base + NavMeshObstacleManager::QueuedListOfDoorsToRemove.off;
            let form_id = e.call(FORM_ID, &args![reference]).u32();
            if list_contains(e, queued, form_id) {
                let form_id = e.call(FORM_ID, &args![reference]).u32();
                list_operation(e, LIST_REMOVE, queued, form_id);
            }
        }
        let form_id = e.call(FORM_ID, &args![reference]).u32();
        let in_closed = map_get(
            e,
            base + NavMeshObstacleManager::ClosedDoorMap.off,
            form_id,
            array,
        );
        if !in_closed {
            let queued = base + NavMeshObstacleManager::QueuedListOfDoorsToAdd.off;
            if !list_contains(e, queued, reference) {
                list_operation(e, LIST_ADD_HEAD, queued, reference);
            }
        }
        pointer_release(e, array);
    });
}

// Translated from 006c1060 (decompiled, FalloutNV.exe 1.4.0.525)
/// Door-gone handler (engine map: unnamed): for a non-sliding door reference
/// calls `RemoveObstacleForReference`; then takes the reference out of
/// `QueuedListOfRefsToAdd`; when it is in `QueuedListOfDoorsToAdd` takes it
/// out of there, else queues its form id in `QueuedListOfDoorsToRemove`
/// (`AddHead`) unless already queued.
pub fn fn_006c1060(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, reference: u32) {
    let base = this.addr();
    if is_ordinary_door(e, reference) {
        nav_mesh_obstacle_manager_remove_obstacle_for_reference(e, this, reference);
    }
    list_operation(
        e,
        LIST_REMOVE,
        base + NavMeshObstacleManager::QueuedListOfRefsToAdd.off,
        reference,
    );
    let doors_to_add = base + NavMeshObstacleManager::QueuedListOfDoorsToAdd.off;
    if list_contains(e, doors_to_add, reference) {
        list_operation(e, LIST_REMOVE, doors_to_add, reference);
    } else {
        let queued = base + NavMeshObstacleManager::QueuedListOfDoorsToRemove.off;
        let form_id = e.call(FORM_ID, &args![reference]).u32();
        if !list_contains(e, queued, form_id) {
            let form_id = e.call(FORM_ID, &args![reference]).u32();
            list_operation(e, LIST_ADD_HEAD, queued, form_id);
        }
    }
}

// Translated from 006c1130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Queues a navmesh form id in `QueuedListOfNavmeshesToEnable` (engine map:
/// unnamed): `AddHead` of the value (no duplicate check).
pub fn fn_006c1130(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, navmesh_id: u32) {
    let list = this.addr() + NavMeshObstacleManager::QueuedListOfNavmeshesToEnable.off;
    list_operation(e, LIST_ADD_HEAD, list, navmesh_id);
}

// Translated from 006c1150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Queues a navmesh form id in `QueuedListOfNavmeshesToDisable` (engine map:
/// unnamed): `AddHead` of the value (no duplicate check).
pub fn fn_006c1150(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, navmesh_id: u32) {
    let list = this.addr() + NavMeshObstacleManager::QueuedListOfNavmeshesToDisable.off;
    list_operation(e, LIST_ADD_HEAD, list, navmesh_id);
}

// Translated from 006c1170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Requests an update of all obstacles (engine map: unnamed): sets
/// `bUpdateAllObstacles`.
pub fn fn_006c1170(e: &mut Engine, this: Ptr<NavMeshObstacleManager>) {
    e.set(this, NavMeshObstacleManager::bUpdateAllObstacles, true);
}

// Translated from 006c1190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds the obstacles of a reference (engine map: unnamed): finds or builds
/// the reference's `ReferenceObstacleArray` in `FormIDMap` (a new one is
/// filled by `006c34f0`); then, for each of its obstacles: enters it in
/// `RigidBodyToObstacleMap` under its rigid body, registers the Havok
/// listeners (`006c1d60`), queues the rigid body when it is active
/// (`006c1c70`), refreshes the obstacle (`006c3440`), adds its debug node
/// when `bDrawObstacles` is set and, when the obstacle is active, queues
/// operation `0x10` on the task of every navmesh it overlaps and records
/// those navmeshes in its `Navmeshes`.
pub fn fn_006c1190(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, reference: u32) {
    let base = this.addr();
    let form_id_map = base + NavMeshObstacleManager::FormIDMap.off;
    e.with_stack(0x10, |e, frame| {
        let array = frame.addr();
        let obstacle = frame.addr() + 4;
        pointer_init(e, array, 0);
        let form_id = e.call(FORM_ID, &args![reference]).u32();
        if !map_get(e, form_id_map, form_id, array) {
            let memory = e.call(OPERATOR_NEW_SIZED, &args![0x1cu32]).u32();
            let created = if memory != 0 {
                reference_obstacle_array_construct(e, Ptr::new(memory)).addr()
            } else {
                0
            };
            e.call(NI_POINTER_SET, &args![array, created]);
            e.call(FILL_REFERENCE_OBSTACLES, &args![reference, array]);
            let form_id = e.call(FORM_ID, &args![reference]).u32();
            map_set_copy(e, form_id_map, form_id, array);
        }
        let mut index = 0;
        while index < obstacle_count(e, array) {
            let obstacles = obstacles_of(e, array);
            let element = array_at(e, obstacles, index);
            pointer_copy(e, obstacle, element);
            let rigid_body = rigid_body_of(e, obstacle);
            map_set_copy(
                e,
                base + NavMeshObstacleManager::RigidBodyToObstacleMap.off,
                rigid_body,
                obstacle,
            );
            let rigid_body = rigid_body_of(e, obstacle);
            fn_006c1d60(e, this, rigid_body);
            let rigid_body = rigid_body_of(e, obstacle);
            if e.call(RIGID_BODY_IS_ACTIVE, &args![rigid_body]).bool() {
                let rigid_body = rigid_body_of(e, obstacle);
                fn_006c1c70(e, this, rigid_body);
            }
            e.call(OBSTACLE_REFRESH, &args![obstacle]);
            if e.get(this, NavMeshObstacleManager::bDrawObstacles) {
                e.call(ADD_BOUNDING_BOX_NODE, &args![this, obstacle]);
            }
            if obstacle_is_active(e, obstacle) {
                queue_overlapping_navmeshes(e, this, obstacle, 0x10);
            }
            pointer_release(e, obstacle);
            index += 1;
        }
        pointer_release(e, array);
    });
}

// Translated from 006c1480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ReferenceObstacleArray` (engine map: unnamed): the
/// reference-counted base constructor (`004968b0`), the class's vtable, the
/// `Obstacles` array constructor (`006c6160`) and a zero `iFormID`. Returns
/// `this`.
pub fn reference_obstacle_array_construct(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(REFERENCE_COUNTED_CTOR, &args![this]);
    e.mem.set_u32(this.addr(), REFERENCE_OBSTACLE_ARRAY_VTABLE);
    e.call(
        OBSTACLE_ARRAY_CTOR,
        &args![this.addr() + ReferenceObstacleArray::Obstacles.off],
    );
    e.mem
        .set_u32(this.addr() + ReferenceObstacleArray::iFormID.off, 0);
    this
}

// Translated from 006c14f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ReferenceObstacleArray::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then the sized `operator delete` (`0x1c` bytes) when bit 0 of
/// `flags` is set. Returns `this`.
pub fn reference_obstacle_array_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    reference_obstacle_array_destructor(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE_SIZED, &args![this, 0x1cu32]);
    }
    this
}

// Translated from 006c1520 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ReferenceObstacleArray::~ReferenceObstacleArray` (Xbox PDB): the class's
/// vtable, a zero `iFormID`, the `Obstacles` array destructor (`006c6190`)
/// and the base destructor (`00496910`).
pub fn reference_obstacle_array_destructor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), REFERENCE_OBSTACLE_ARRAY_VTABLE);
    e.mem
        .set_u32(this.addr() + ReferenceObstacleArray::iFormID.off, 0);
    e.call(
        OBSTACLE_ARRAY_DTOR,
        &args![this.addr() + ReferenceObstacleArray::Obstacles.off],
    );
    e.call(REFERENCE_COUNTED_DTOR, &args![this]);
}

// Translated from 006c1590 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes the obstacles of a reference by form id (engine map: unnamed): if
/// `FormIDMap` has the id, removes the entry and, for each of its
/// obstacles, the `RigidBodyToObstacleMap` entry of its rigid body, the node
/// in `ObstaclesUpdateList`, the debug node when `bDrawObstacles` is set and,
/// when it recorded navmeshes, queues operation `0x20` on each of their tasks
/// and clears the record.
pub fn fn_006c1590(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, form_id: u32) {
    let base = this.addr();
    e.with_stack(8, |e, frame| {
        let array = frame.addr();
        let obstacle = frame.addr() + 4;
        pointer_init(e, array, 0);
        if !map_get(
            e,
            base + NavMeshObstacleManager::FormIDMap.off,
            form_id,
            array,
        ) {
            pointer_release(e, array);
            return;
        }
        e.call(
            MAP_REMOVE,
            &args![base + NavMeshObstacleManager::FormIDMap.off, form_id],
        );
        let mut index = 0;
        while index < obstacle_count(e, array) {
            let obstacles = obstacles_of(e, array);
            let element = array_at(e, obstacles, index);
            pointer_copy(e, obstacle, element);
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(
                MAP_REMOVE,
                &args![
                    base + NavMeshObstacleManager::RigidBodyToObstacleMap.off,
                    rigid_body
                ],
            );
            bs_simple_list_obstacle_data_remove(
                e,
                Ptr::new(base + NavMeshObstacleManager::ObstaclesUpdateList.off),
                Ptr::new(obstacle),
            );
            if e.get(this, NavMeshObstacleManager::bDrawObstacles) {
                e.call(REMOVE_BOUNDING_BOX_NODE, &args![this, obstacle]);
            }
            let navmeshes = navmeshes_of(e, obstacle);
            if !e.call(ARRAY_IS_EMPTY, &args![navmeshes]).bool() {
                queue_recorded_navmeshes(e, this, obstacle, 0x20);
            }
            pointer_release(e, obstacle);
            index += 1;
        }
        pointer_release(e, array);
    });
}

// Translated from 006c1760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Re-registers one obstacle (engine map: unnamed; `obstacle` is the address
/// of its `NiPointer<ObstacleData>`): for an active obstacle queues operation
/// `0x21` on the task of each navmesh it recorded and clears the record;
/// then refreshes it (`006c2650`, `006c3440`), adds its debug node when
/// `bDrawObstacles` is set and, when it is active, queues operation `0x11` on
/// the tasks of the navmeshes it now overlaps and records them.
pub fn fn_006c1760(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, obstacle: u32) {
    if obstacle_is_active(e, obstacle) {
        queue_recorded_navmeshes(e, this, obstacle, 0x21);
    }
    let data = pointer_get(e, obstacle);
    e.call(OBSTACLE_PREPARE, &args![data]);
    e.call(OBSTACLE_REFRESH, &args![obstacle]);
    if e.get(this, NavMeshObstacleManager::bDrawObstacles) {
        e.call(ADD_BOUNDING_BOX_NODE, &args![this, obstacle]);
    }
    if obstacle_is_active(e, obstacle) {
        queue_overlapping_navmeshes(e, this, obstacle, 0x11);
    }
}

// Translated from 006c1920 (decompiled, FalloutNV.exe 1.4.0.525)
/// Replaces the obstacles of a reference (engine map: unnamed). If
/// `FormIDMap` has `form_id`: for each old obstacle queues operation `0x21`
/// on the tasks of its recorded navmeshes (when it is active, then clears the
/// record), removes its debug node when `bDrawObstacles` is set, calls
/// `006c1e00` for its rigid body and removes the entry from
/// `RigidBodyToObstacleMap` and the node from `ObstaclesUpdateList`. Then
/// `006c3520(other, &array)` refills the array and for each of its obstacles:
/// refreshes it (`006c3440`), queues operation `0x11` on the navmeshes it
/// overlaps (when active), adds its debug node when `bDrawObstacles` is set
/// and registers the Havok listeners (`006c1d60`). `other` is the second
/// parameter, passed unchanged to `006c3520`.
pub fn fn_006c1920(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, form_id: u32, other: u32) {
    let base = this.addr();
    e.with_stack(8, |e, frame| {
        let array = frame.addr();
        let obstacle = frame.addr() + 4;
        pointer_init(e, array, 0);
        if !map_get(
            e,
            base + NavMeshObstacleManager::FormIDMap.off,
            form_id,
            array,
        ) {
            pointer_release(e, array);
            return;
        }
        let mut index = 0;
        while index < obstacle_count(e, array) {
            let obstacles = obstacles_of(e, array);
            let element = array_at(e, obstacles, index);
            pointer_copy(e, obstacle, element);
            if obstacle_is_active(e, obstacle) {
                queue_recorded_navmeshes(e, this, obstacle, 0x21);
            }
            if e.get(this, NavMeshObstacleManager::bDrawObstacles) {
                e.call(REMOVE_BOUNDING_BOX_NODE, &args![this, obstacle]);
            }
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(REMOVE_RIGID_BODY, &args![this, rigid_body]);
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(
                MAP_REMOVE,
                &args![
                    base + NavMeshObstacleManager::RigidBodyToObstacleMap.off,
                    rigid_body
                ],
            );
            bs_simple_list_obstacle_data_remove(
                e,
                Ptr::new(base + NavMeshObstacleManager::ObstaclesUpdateList.off),
                Ptr::new(obstacle),
            );
            pointer_release(e, obstacle);
            index += 1;
        }
        e.call(OBSTACLE_ARRAY_REFRESH, &args![other, array]);
        let mut index = 0;
        while index < obstacle_count(e, array) {
            let obstacles = obstacles_of(e, array);
            let element = array_at(e, obstacles, index);
            pointer_copy(e, obstacle, element);
            e.call(OBSTACLE_REFRESH, &args![obstacle]);
            if obstacle_is_active(e, obstacle) {
                queue_overlapping_navmeshes(e, this, obstacle, 0x11);
            }
            if e.get(this, NavMeshObstacleManager::bDrawObstacles) {
                e.call(ADD_BOUNDING_BOX_NODE, &args![this, obstacle]);
            }
            let rigid_body = rigid_body_of(e, obstacle);
            fn_006c1d60(e, this, rigid_body);
            pointer_release(e, obstacle);
            index += 1;
        }
        pointer_release(e, array);
    });
}

// Translated from 006c1c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Pushes a rigid body on `ObstaclesToAddToUpdateList` (engine map: unnamed;
/// the lock-free queue push `006c5ff0` takes the value's address).
pub fn fn_006c1c70(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, rigid_body: u32) {
    let queue = this.addr() + NavMeshObstacleManager::ObstaclesToAddToUpdateList.off;
    list_operation(e, QUEUE_PUSH, queue, rigid_body);
}

// Translated from 006c1c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Pushes a rigid body on `ObstaclesToRemoveFromUpdateList` (engine map:
/// unnamed).
pub fn fn_006c1c90(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, rigid_body: u32) {
    let queue = this.addr() + NavMeshObstacleManager::ObstaclesToRemoveFromUpdateList.off;
    list_operation(e, QUEUE_PUSH, queue, rigid_body);
}

// Translated from 006c1cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshObstacleManager::OnObstacleLost3D` (Xbox PDB): looks the rigid
/// body up in `RigidBodyToObstacleMap`; for an obstacle found, takes the form
/// id of its parent `ReferenceObstacleArray`, looks the form up (`cdecl`
/// `004839c0`) and, when it exists, calls `RemoveObstacleForReference` with
/// it.
pub fn nav_mesh_obstacle_manager_on_obstacle_lost3d(
    e: &mut Engine,
    this: Ptr<NavMeshObstacleManager>,
    rigid_body: u32,
) {
    e.with_stack(4, |e, obstacle| {
        let obstacle = obstacle.addr();
        pointer_init(e, obstacle, 0);
        let map = this.addr() + NavMeshObstacleManager::RigidBodyToObstacleMap.off;
        if map_get(e, map, rigid_body, obstacle) {
            let data = pointer_get(e, obstacle);
            let parent = word(e, data + ObstacleData::spParentArray.off);
            let form_id = word(e, parent + ReferenceObstacleArray::iFormID.off);
            let form = e.call(FORM_LOOKUP_BY_ID, &args![form_id]).u32();
            if form != 0 {
                nav_mesh_obstacle_manager_remove_obstacle_for_reference(e, this, form);
            }
        }
        pointer_release(e, obstacle);
    });
}

// Translated from 006c1d60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Registers the manager's two listeners on a rigid body's Havok entity
/// (engine map: unnamed): `006c1dd0` with `pListener`, then `006c1da0` with
/// `pRemoveListener`.
pub fn fn_006c1d60(e: &mut Engine, this: Ptr<NavMeshObstacleManager>, rigid_body: u32) {
    let listener = e.get(this, NavMeshObstacleManager::pListener);
    fn_006c1dd0(e, Ptr::new(rigid_body), listener);
    let remove_listener = e.get(this, NavMeshObstacleManager::pRemoveListener);
    fn_006c1da0(e, Ptr::new(rigid_body), remove_listener);
}

// Translated from 006c1da0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds an entity listener to a rigid body's Havok entity (engine map:
/// unnamed): `00c9c5b0` on the entity (`004ae750` of the rigid body) when
/// there is one.
pub fn fn_006c1da0(e: &mut Engine, this: Ptr, listener: u32) {
    let entity = e.call(RIGID_BODY_ENTITY, &args![this]).u32();
    if entity != 0 {
        e.call(ENTITY_ADD_ENTITY_LISTENER, &args![entity, listener]);
    }
}

// Translated from 006c1dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds an activation listener to a rigid body's Havok entity (engine map:
/// unnamed): `00c9c700` on the entity (`004ae750` of the rigid body) when
/// there is one.
pub fn fn_006c1dd0(e: &mut Engine, this: Ptr, listener: u32) {
    let entity = e.call(RIGID_BODY_ENTITY, &args![this]).u32();
    if entity != 0 {
        e.call(ENTITY_ADD_ACTIVATION_LISTENER, &args![entity, listener]);
    }
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00559120,
            ni_tmap_base_reference_obstacle_array_get_next(Ptr<NiTMap>, Ptr, Ptr, Ptr)
        ),
        entry!(
            0x00631620,
            bs_simple_list_obstacle_data_remove(Ptr<BSSimpleList>, Ptr)
        ),
        entry!(
            0x00691a30,
            ni_tmap_reference_obstacle_array_delete_item(Ptr<NiTMap>, Ptr<NiTMapItem>)
        ),
        entry!(
            0x006a63e0,
            ni_tmap_base_rigid_body_obstacle_data_set_value(Ptr<NiTMap>, Ptr<NiTMapItem>, u32, u32)
        ),
        entry!(0x006c00b0, fn_006c00b0(Ptr) -> Ptr),
        entry!(
            0x006c0190,
            bhk_obstacle_deactivation_listener_entity_deactivated_callback(Ptr, u32)
        ),
        entry!(
            0x006c01d0,
            bhk_obstacle_deactivation_listener_entity_activated_callback(Ptr, u32)
        ),
        entry!(0x006c0210, fn_006c0210(Ptr) -> Ptr),
        entry!(0x006c0230, fn_006c0230(Ptr, u32) -> Ptr),
        entry!(0x006c0260, fn_006c0260(Ptr)),
        entry!(
            0x006c0280,
            bhk_obstacle_removal_listener_entity_removed_callback(Ptr, u32)
        ),
        entry!(
            0x006c02c0,
            nav_mesh_obstacle_manager_nav_mesh_obstacle_manager(
                Ptr<NavMeshObstacleManager>,
            )
                -> Ptr<NavMeshObstacleManager>
        ),
        entry!(0x006c0510, fn_006c0510(Ptr<NavMeshObstacleManager>)),
        entry!(0x006c0720, fn_006c0720() -> Ptr<NavMeshObstacleManager>),
        entry!(
            0x006c07d0,
            nav_mesh_obstacle_manager_initialize(Ptr<NavMeshObstacleManager>) -> bool
        ),
        entry!(0x006c08e0, fn_006c08e0(Ptr) -> bool),
        entry!(
            0x006c0900,
            nav_mesh_obstacle_manager_release(Ptr<NavMeshObstacleManager>)
        ),
        entry!(0x006c09d0, fn_006c09d0(Ptr)),
        entry!(
            0x006c09f0,
            nav_mesh_obstacle_manager_clear_all_obstacles(Ptr<NavMeshObstacleManager>)
        ),
        entry!(
            0x006c0c30,
            nav_mesh_obstacle_manager_add_obstacle_for_reference(Ptr<NavMeshObstacleManager>, u32)
        ),
        entry!(
            0x006c0c80,
            nav_mesh_obstacle_manager_remove_obstacle_for_reference(
                Ptr<NavMeshObstacleManager>,
                u32,
            )
        ),
        entry!(0x006c0dc0, fn_006c0dc0(Ptr<NavMeshObstacleManager>, u32)),
        entry!(
            0x006c0f10,
            nav_mesh_obstacle_manager_on_door_close(Ptr<NavMeshObstacleManager>, u32)
        ),
        entry!(0x006c1060, fn_006c1060(Ptr<NavMeshObstacleManager>, u32)),
        entry!(0x006c1130, fn_006c1130(Ptr<NavMeshObstacleManager>, u32)),
        entry!(0x006c1150, fn_006c1150(Ptr<NavMeshObstacleManager>, u32)),
        entry!(0x006c1170, fn_006c1170(Ptr<NavMeshObstacleManager>)),
        entry!(0x006c1190, fn_006c1190(Ptr<NavMeshObstacleManager>, u32)),
        entry!(0x006c1480, reference_obstacle_array_construct(Ptr) -> Ptr),
        entry!(0x006c14f0, reference_obstacle_array_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x006c1520, reference_obstacle_array_destructor(Ptr)),
        entry!(0x006c1590, fn_006c1590(Ptr<NavMeshObstacleManager>, u32)),
        entry!(0x006c1760, fn_006c1760(Ptr<NavMeshObstacleManager>, u32)),
        entry!(
            0x006c1920,
            fn_006c1920(Ptr<NavMeshObstacleManager>, u32, u32)
        ),
        entry!(0x006c1c70, fn_006c1c70(Ptr<NavMeshObstacleManager>, u32)),
        entry!(0x006c1c90, fn_006c1c90(Ptr<NavMeshObstacleManager>, u32)),
        entry!(
            0x006c1cb0,
            nav_mesh_obstacle_manager_on_obstacle_lost3d(Ptr<NavMeshObstacleManager>, u32)
        ),
        entry!(0x006c1d60, fn_006c1d60(Ptr<NavMeshObstacleManager>, u32)),
        entry!(0x006c1da0, fn_006c1da0(Ptr, u32)),
        entry!(0x006c1dd0, fn_006c1dd0(Ptr, u32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Log = Rc<RefCell<Vec<Vec<u32>>>>;

    fn ret(eax: u32) -> Ret {
        Ret {
            eax,
            ..Ret::default()
        }
    }

    /// Doubles that do nothing and return zero.
    fn stub(e: &mut Engine, addresses: &[u32]) {
        for address in addresses {
            e.register(*address, |_, _| Ret::default());
        }
    }

    /// A double that returns a fixed value.
    fn stub_ret(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| ret(value));
    }

    /// A double that records the argument words of every call and returns
    /// `value`.
    fn record(e: &mut Engine, address: u32, value: u32) -> Log {
        let log: Log = Rc::default();
        let shared = log.clone();
        e.register_double(address, move |_, a| {
            shared.borrow_mut().push(a.to_vec());
            ret(value)
        });
        log
    }

    /// A double for a function that takes a list or queue and the address of
    /// a word: records (first argument, the word).
    fn record_word(e: &mut Engine, address: u32) -> Log {
        let log: Log = Rc::default();
        let shared = log.clone();
        e.register_double(address, move |e, a| {
            let value = e.mem.u32(a[1]);
            shared.borrow_mut().push(vec![a[0], value]);
            Ret::default()
        });
        log
    }

    /// An engine with the pages the code reads mapped and the accessors of
    /// `NiPointer`, `BSSimpleArray` and `TESForm` working on game memory.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x011d_7000, 0x1000);
        e.map(0x0106_c000, 0x1000);
        e.register(NI_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        e.register(NI_POINTER_INIT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        stub(&mut e, &[NI_POINTER_RELEASE]);
        for address in [NI_POINTER_COPY, NI_POINTER_ASSIGN] {
            e.register(address, |e, a| {
                let value = e.mem.u32(a[1]);
                e.mem.set_u32(a[0], value);
                ret(a[0])
            });
        }
        e.register(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(ARRAY_SIZE, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(ARRAY_AT, |e, a| ret(e.mem.u32(a[0] + 4) + a[1] * 4));
        e.register(FORM_ID, |e, a| ret(e.mem.u32(a[0] + 0x0c)));
        e
    }

    /// The recorded calls to `addresses`, in order.
    fn calls(e: &Engine, addresses: &[u32]) -> Vec<(u32, Vec<u32>)> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| addresses.contains(a))
            .cloned()
            .collect()
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// A `BSSimpleArray` header written at `at` (buffer at `+4`, size at
    /// `+8`).
    fn set_array(e: &mut Engine, at: u32, elements: &[u32]) {
        let buffer = e.mem.alloc(4 * elements.len().max(1) as u32);
        for (i, value) in elements.iter().enumerate() {
            e.mem.set_u32(buffer + 4 * i as u32, *value);
        }
        e.mem.set_u32(at + 4, buffer);
        e.mem.set_u32(at + 8, elements.len() as u32);
    }

    /// A `NiPointer` slot holding `value`.
    fn slot_with(e: &mut Engine, value: u32) -> u32 {
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, value);
        slot
    }

    /// An `ObstacleData`: rigid body, parent array, active flag and the
    /// recorded `NavMeshInfo` pointers.
    fn obstacle(e: &mut Engine, rigid_body: u32, parent: u32, active: bool, infos: &[u32]) -> u32 {
        let data = e.mem.alloc(0x8c);
        e.mem.set_u32(data + 8, parent);
        e.mem.set_u32(data + 0x0c, rigid_body);
        e.mem.set_u8(data + 0x74, active as u8);
        set_array(e, data + 0x78, infos);
        data
    }

    /// A `ReferenceObstacleArray` with the given form id and obstacles.
    fn reference_array(e: &mut Engine, form_id: u32, obstacles: &[u32]) -> u32 {
        let array = e.mem.alloc(0x1c);
        e.mem.set_u32(array + 8, form_id);
        set_array(e, array + 0x0c, obstacles);
        array
    }

    fn manager(e: &mut Engine) -> Ptr<NavMeshObstacleManager> {
        e.new_object()
    }

    /// A bare `TESForm`-like object: only the word at `+0x0c` (the id).
    fn reference_with_id(e: &mut Engine, form_id: u32) -> u32 {
        let reference = e.mem.alloc(0x30);
        e.mem.set_u32(reference + 0x0c, form_id);
        reference
    }

    /// A map item `[next, key, value]`.
    fn map_item(e: &mut Engine, next: u32, key: u32, value: u32) -> u32 {
        let item = e.mem.alloc(12);
        e.mem.set_u32(item, next);
        e.mem.set_u32(item + 4, key);
        e.mem.set_u32(item + 8, value);
        item
    }

    /// A list node `[item, next]`.
    fn list_node(e: &mut Engine, item: u32, next: u32) -> u32 {
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, item);
        e.mem.set_u32(node + 4, next);
        node
    }

    /// The obstacle setting: a byte the setting getter points at.
    fn set_setting(e: &mut Engine, on: bool) {
        let value = e.mem.alloc(4);
        e.mem.set_u8(value, on as u8);
        stub_ret(e, SETTING_VALUE_ADDRESS, value);
    }

    /// A map whose hash function is `key % 4`, with four buckets.
    fn hashed_map(e: &mut Engine, at: u32) -> u32 {
        e.put_vtable(0x0080_0000, &[0, 0x0070_0000]);
        e.register(0x0070_0000, |_, a| ret(a[1] % 4));
        let table = e.mem.alloc(16);
        e.mem.set_u32(at, 0x0080_0000);
        e.mem.set_u32(at + 4, 4);
        e.mem.set_u32(at + 8, table);
        table
    }

    #[test]
    fn get_next_moves_along_the_chain_then_through_the_buckets() {
        let mut e = engine();
        let map = e.mem.alloc(0x10);
        let table = hashed_map(&mut e, map);
        let last = map_item(&mut e, 0, 3, 0xccc);
        let second = map_item(&mut e, 0, 1, 0xbbb);
        let first = map_item(&mut e, second, 5, 0xaaa);
        e.mem.set_u32(table + 12, last);
        let frame = e.mem.alloc(12);
        let (position, key, value) = (frame, frame + 4, frame + 8);
        let step = |e: &mut Engine| {
            ni_tmap_base_reference_obstacle_array_get_next(
                e,
                Ptr::new(map),
                Ptr::new(position),
                Ptr::new(key),
                Ptr::new(value),
            )
        };

        // An item with a successor in its chain.
        e.mem.set_u32(position, first);
        step(&mut e);
        assert_eq!(e.mem.u32(position), second);
        assert_eq!(e.mem.u32(key), 5);
        assert_eq!(e.mem.u32(value), 0xaaa);

        // The last of its chain: the next non-empty bucket after bucket 1.
        step(&mut e);
        assert_eq!(e.mem.u32(position), last);
        assert_eq!(e.mem.u32(key), 1);
        assert_eq!(e.mem.u32(value), 0xbbb);

        // The last bucket: the walk ends.
        step(&mut e);
        assert_eq!(e.mem.u32(position), 0);
        assert_eq!(e.mem.u32(key), 3);
    }

    /// The doubles `BSSimpleList<NiPointer<ObstacleData>>::Remove` needs.
    fn list_engine() -> (Engine, Log) {
        let mut e = engine();
        e.register(FIRST_WORD_EQUALS, |e, a| {
            ret((e.mem.u32(a[0]) == a[1]) as u32)
        });
        e.register(LIST_IS_EMPTY, |e, a| {
            ret((e.mem.u32(a[0] + 4) == 0 && e.mem.u32(a[0]) == 0) as u32)
        });
        e.register(LIST_NODE_DIFFERS, |e, a| {
            ret((e.mem.u32(a[0]) != e.mem.u32(a[1])) as u32)
        });
        let deleted = record(&mut e, LIST_NODE_DELETE, 0);
        (e, deleted)
    }

    #[test]
    fn list_remove_unlinks_a_middle_node() {
        let (mut e, deleted) = list_engine();
        let third = list_node(&mut e, 30, 0);
        let second = list_node(&mut e, 20, third);
        let head = list_node(&mut e, 10, second);
        let item = slot_with(&mut e, 20);
        bs_simple_list_obstacle_data_remove(&mut e, Ptr::new(head), Ptr::new(item));
        assert_eq!(e.mem.u32(head + 4), third);
        assert_eq!(e.mem.u32(second + 4), 0);
        assert_eq!(*deleted.borrow(), vec![vec![second, 1]]);
    }

    #[test]
    fn list_remove_of_the_head_takes_the_next_nodes_item() {
        let (mut e, deleted) = list_engine();
        let third = list_node(&mut e, 30, 0);
        let second = list_node(&mut e, 20, third);
        let head = list_node(&mut e, 10, second);
        let item = slot_with(&mut e, 10);
        bs_simple_list_obstacle_data_remove(&mut e, Ptr::new(head), Ptr::new(item));
        assert_eq!(e.mem.u32(head), 20);
        assert_eq!(e.mem.u32(head + 4), third);
        assert_eq!(*deleted.borrow(), vec![vec![second, 1]]);
    }

    #[test]
    fn list_remove_of_the_only_node_clears_its_item() {
        let (mut e, deleted) = list_engine();
        let head = list_node(&mut e, 10, 0);
        let item = slot_with(&mut e, 10);
        bs_simple_list_obstacle_data_remove(&mut e, Ptr::new(head), Ptr::new(item));
        assert_eq!(e.mem.u32(head), 0);
        assert!(deleted.borrow().is_empty());
    }

    #[test]
    fn list_remove_ignores_null_items_and_missing_ones() {
        let (mut e, deleted) = list_engine();
        let second = list_node(&mut e, 20, 0);
        let head = list_node(&mut e, 10, second);
        let null_item = slot_with(&mut e, 0);
        bs_simple_list_obstacle_data_remove(&mut e, Ptr::new(head), Ptr::new(null_item));
        let missing = slot_with(&mut e, 99);
        bs_simple_list_obstacle_data_remove(&mut e, Ptr::new(head), Ptr::new(missing));
        assert_eq!(e.mem.u32(head + 4), second);
        assert_eq!(e.mem.u32(head), 10);
        assert!(deleted.borrow().is_empty());
    }

    #[test]
    fn delete_item_releases_the_value_and_frees_the_item() {
        let mut e = engine();
        let free = record(&mut e, MAP_FREE_ITEM, 0);
        let map = e.mem.alloc(0x10);
        let item = map_item(&mut e, 0, 7, 0x1234);
        ni_tmap_reference_obstacle_array_delete_item(&mut e, Ptr::new(map), Ptr::new(item));
        assert_eq!(e.mem.u32(item + 8), 0);
        assert_eq!(*free.borrow(), vec![vec![map + 0x0c, item]]);
    }

    #[test]
    fn set_value_stores_the_key_and_assigns_then_destroys_the_argument() {
        let mut e = engine();
        start_log(&mut e);
        let item = map_item(&mut e, 0, 0, 0);
        ni_tmap_base_rigid_body_obstacle_data_set_value(
            &mut e,
            Ptr::new(0),
            Ptr::new(item),
            0x77,
            0x1234,
        );
        assert_eq!(e.mem.u32(item + 4), 0x77);
        assert_eq!(e.mem.u32(item + 8), 0x1234);
        let log = calls(&e, &[NI_POINTER_ASSIGN, NI_POINTER_RELEASE]);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].0, NI_POINTER_ASSIGN);
        assert_eq!(log[0].1[0], item + 8);
        assert_eq!(log[1].0, NI_POINTER_RELEASE);
        assert_eq!(log[1].1[0], log[0].1[1]);
    }

    #[test]
    fn the_listener_constructors_set_their_vtables() {
        let mut e = engine();
        let activation_base = record(&mut e, ACTIVATION_LISTENER_BASE_CTOR, 0);
        let entity_base = record(&mut e, ENTITY_LISTENER_BASE_CTOR, 0);
        let deactivation = e.mem.alloc(4);
        assert_eq!(
            fn_006c00b0(&mut e, Ptr::new(deactivation)).addr(),
            deactivation
        );
        assert_eq!(e.mem.u32(deactivation), DEACTIVATION_LISTENER_VTABLE);
        assert_eq!(*activation_base.borrow(), vec![vec![deactivation]]);
        let removal = e.mem.alloc(4);
        assert_eq!(fn_006c0210(&mut e, Ptr::new(removal)).addr(), removal);
        assert_eq!(e.mem.u32(removal), REMOVAL_LISTENER_VTABLE);
        assert_eq!(*entity_base.borrow(), vec![vec![removal]]);
    }

    #[test]
    fn the_removal_listener_destructor_runs_the_base_destructor() {
        let mut e = engine();
        let base = record(&mut e, ENTITY_LISTENER_BASE_DTOR, 0);
        let listener = e.mem.alloc(4);
        fn_006c0260(&mut e, Ptr::new(listener));
        assert_eq!(e.mem.u32(listener), REMOVAL_LISTENER_VTABLE);
        assert_eq!(*base.borrow(), vec![vec![listener]]);
    }

    #[test]
    fn the_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        stub(&mut e, &[ENTITY_LISTENER_BASE_DTOR]);
        let freed = record(&mut e, OPERATOR_DELETE, 0);
        let listener = e.mem.alloc(4);
        assert_eq!(fn_006c0230(&mut e, Ptr::new(listener), 0).addr(), listener);
        assert!(freed.borrow().is_empty());
        assert_eq!(fn_006c0230(&mut e, Ptr::new(listener), 3).addr(), listener);
        assert_eq!(*freed.borrow(), vec![vec![listener]]);
    }

    /// An engine whose manager instance exists (so `006c0720` only returns
    /// it), with the queue pushes recorded by value and the entity user
    /// data `0x4321`.
    fn callback_engine() -> (Engine, Log) {
        let mut e = engine();
        e.set_global(MANAGER_INSTANCE_GUARD, 1u32);
        stub(&mut e, &[SCOPE_GUARD_CTOR, SCOPE_GUARD_DTOR]);
        let pushes = record_word(&mut e, QUEUE_PUSH);
        stub_ret(&mut e, ENTITY_USER_DATA, 0x4321);
        (e, pushes)
    }

    #[test]
    fn the_deactivation_callback_queues_the_rigid_body_for_removal() {
        let (mut e, pushes) = callback_engine();
        bhk_obstacle_deactivation_listener_entity_deactivated_callback(&mut e, Ptr::NULL, 0x99);
        assert_eq!(
            *pushes.borrow(),
            vec![vec![MANAGER_INSTANCE + 0xa0, 0x4321]]
        );
        // No user data: nothing is queued.
        stub_ret(&mut e, ENTITY_USER_DATA, 0);
        bhk_obstacle_deactivation_listener_entity_deactivated_callback(&mut e, Ptr::NULL, 0x99);
        assert_eq!(pushes.borrow().len(), 1);
    }

    #[test]
    fn the_activation_callback_queues_the_rigid_body_for_adding() {
        let (mut e, pushes) = callback_engine();
        bhk_obstacle_deactivation_listener_entity_activated_callback(&mut e, Ptr::NULL, 0x99);
        assert_eq!(
            *pushes.borrow(),
            vec![vec![MANAGER_INSTANCE + 0x60, 0x4321]]
        );
        stub_ret(&mut e, ENTITY_USER_DATA, 0);
        bhk_obstacle_deactivation_listener_entity_activated_callback(&mut e, Ptr::NULL, 0x99);
        assert_eq!(pushes.borrow().len(), 1);
    }

    #[test]
    fn the_removal_callback_reports_the_lost_obstacle() {
        let (mut e, _) = callback_engine();
        // OnObstacleLost3D: the rigid body is not in the map, so only the
        // lookup happens.
        let lookup = record(&mut e, MAP_GET, 0);
        bhk_obstacle_removal_listener_entity_removed_callback(&mut e, Ptr::NULL, 0x99);
        assert_eq!(lookup.borrow().len(), 1);
        assert_eq!(lookup.borrow()[0][..2], [MANAGER_INSTANCE + 0xe0, 0x4321]);
        stub_ret(&mut e, ENTITY_USER_DATA, 0);
        bhk_obstacle_removal_listener_entity_removed_callback(&mut e, Ptr::NULL, 0x99);
        assert_eq!(lookup.borrow().len(), 1);
    }

    /// Every constructor and destructor the manager's members use, and the
    /// calls around them.
    const MEMBER_FUNCTIONS: [u32; 24] = [
        CRITICAL_SECTION_CTOR,
        CRITICAL_SECTION_DTOR,
        MAP_CTOR,
        MAP_DTOR,
        TASK_MAP_CTOR,
        TASK_MAP_DTOR,
        RIGID_BODY_MAP_CTOR,
        RIGID_BODY_MAP_DTOR,
        REFS_LIST_CTOR,
        REFS_LIST_DTOR,
        UPDATE_LIST_CTOR,
        UPDATE_LIST_DTOR,
        QUEUE_CTOR,
        QUEUE_DTOR,
        TASK_ARRAY_CTOR,
        TASK_ARRAY_DTOR,
        TASKLET_GROUP_CTOR,
        TASKLET_GROUP_DTOR,
        PERFORMANCE_TIMER_REGISTER,
        SCOPE_GUARD_CTOR,
        SCOPE_GUARD_DTOR,
        MAP_CLEAR,
        LIST_CLEAR,
        UPDATE_LIST_CLEAR,
    ];

    /// Doubles for the member functions and `ClearAllObstacles`' callees,
    /// with an empty `FormIDMap`.
    fn member_engine() -> Engine {
        let mut e = engine();
        stub(&mut e, &MEMBER_FUNCTIONS);
        stub(&mut e, &[QUEUE_CLEAR, MAP_FIRST_ITEM, DRAW_OBSTACLES]);
        e
    }

    #[test]
    fn the_constructor_builds_the_members_in_order_and_sets_the_defaults() {
        let mut e = member_engine();
        e.set_global(INITIAL_TIME_TO_NEXT_SWAP, 0.5f32);
        start_log(&mut e);
        let this = manager(&mut e);
        let base = this.addr();
        // Junk the constructor must overwrite.
        e.mem.set_u32(base + 0x19c, 0xbeef);
        e.mem.set_u32(base + 0x120, 0xdead);
        e.mem.set_u32(base + 0x124, 0xdead);
        e.mem.set_u32(base + 0x190, 0xdead);
        e.mem.set_u8(base + 0x198, 1);
        e.mem.set_u8(base + 0x18, 1);
        let result = nav_mesh_obstacle_manager_nav_mesh_obstacle_manager(&mut e, this);
        assert_eq!(result, this);
        let expected = vec![
            (CRITICAL_SECTION_CTOR, vec![base]),
            (MAP_CTOR, vec![base + 0x1c, 0x25]),
            (REFS_LIST_CTOR, vec![base + 0x2c]),
            (REFS_LIST_CTOR, vec![base + 0x34]),
            (UPDATE_LIST_CTOR, vec![base + 0x3c]),
            (QUEUE_CTOR, vec![base + 0x60, 7, 8]),
            (QUEUE_CTOR, vec![base + 0xa0, 7, 8]),
            (RIGID_BODY_MAP_CTOR, vec![base + 0xe0, 0x25]),
            (REFS_LIST_CTOR, vec![base + 0xf0]),
            (REFS_LIST_CTOR, vec![base + 0xf8]),
            (MAP_CTOR, vec![base + 0x100, 0x25]),
            (MAP_CTOR, vec![base + 0x110, 0x25]),
            (TASKLET_GROUP_CTOR, vec![base + 0x128]),
            (TASK_MAP_CTOR, vec![base + 0x12c, 0x25]),
            (TASK_ARRAY_CTOR, vec![base + 0x13c]),
            (TASK_ARRAY_CTOR, vec![base + 0x14c]),
            (REFS_LIST_CTOR, vec![base + 0x160]),
            (REFS_LIST_CTOR, vec![base + 0x180]),
            (REFS_LIST_CTOR, vec![base + 0x188]),
            (
                PERFORMANCE_TIMER_REGISTER,
                vec![base + 0x1a0, PATHING_CATEGORY_NAME, MAIN_TIMER_NAME, 1, 1],
            ),
            (
                PERFORMANCE_TIMER_REGISTER,
                vec![
                    base + 0x1a1,
                    PATHING_CATEGORY_NAME,
                    TASKLET_TIMER_NAME,
                    0,
                    1,
                ],
            ),
        ];
        let mut observed = calls(&e, &MEMBER_FUNCTIONS);
        observed.retain(|(a, _)| *a != SCOPE_GUARD_CTOR && *a != SCOPE_GUARD_DTOR);
        assert_eq!(observed, expected);
        // The shared-object pointer is initialised to null.
        assert_eq!(e.mem.u32(base + 0x19c), 0);
        // The scope guard wraps the field defaults (category 0x2e, line 0xb8).
        let guards = calls(&e, &[SCOPE_GUARD_CTOR, SCOPE_GUARD_DTOR]);
        assert_eq!(guards.len(), 2);
        assert_eq!(guards[0].1[1..], [0x2e, 1, SOURCE_PATH, 0xb8]);
        assert_eq!(e.mem.u32(base + 0x120), 0);
        assert_eq!(e.mem.u32(base + 0x124), 0);
        assert_eq!(e.mem.u32(base + 0x190), 0);
        assert_eq!(e.mem.f32(base + 0x194), 0.5);
        assert_eq!(e.mem.u8(base + 0x198), 0);
        assert_eq!(e.mem.u8(base + 0x18), 0);
    }

    #[test]
    fn the_destructor_clears_deletes_the_listeners_and_destroys_the_members() {
        let mut e = member_engine();
        // Two listeners whose virtual deleting destructor is recorded.
        let deleted = record(&mut e, 0x0070_0100, 0);
        e.put_vtable(0x0080_0100, &[0x0070_0100]);
        let this = manager(&mut e);
        let base = this.addr();
        let first = slot_with(&mut e, 0x0080_0100);
        let second = slot_with(&mut e, 0x0080_0100);
        e.mem.set_u32(base + 0x120, first);
        e.mem.set_u32(base + 0x124, second);
        start_log(&mut e);
        fn_006c0510(&mut e, this);
        assert_eq!(*deleted.borrow(), vec![vec![first, 1], vec![second, 1]]);
        // The fields keep the dead pointers.
        assert_eq!(e.mem.u32(base + 0x120), first);
        let destroyed: Vec<_> = calls(
            &e,
            &[
                NI_POINTER_RELEASE,
                REFS_LIST_DTOR,
                TASK_ARRAY_DTOR,
                TASK_MAP_DTOR,
                TASKLET_GROUP_DTOR,
                MAP_DTOR,
                RIGID_BODY_MAP_DTOR,
                QUEUE_DTOR,
                UPDATE_LIST_DTOR,
                CRITICAL_SECTION_DTOR,
            ],
        )
        .into_iter()
        .filter(|(a, args)| *a != NI_POINTER_RELEASE || args[0] == base + 0x19c)
        .collect();
        let expected = vec![
            (NI_POINTER_RELEASE, vec![base + 0x19c]),
            (REFS_LIST_DTOR, vec![base + 0x188]),
            (REFS_LIST_DTOR, vec![base + 0x180]),
            (TASK_ARRAY_DTOR, vec![base + 0x14c]),
            (TASK_ARRAY_DTOR, vec![base + 0x13c]),
            (TASK_MAP_DTOR, vec![base + 0x12c]),
            (TASKLET_GROUP_DTOR, vec![base + 0x128]),
            (MAP_DTOR, vec![base + 0x110]),
            (MAP_DTOR, vec![base + 0x100]),
            (REFS_LIST_DTOR, vec![base + 0xf8]),
            (REFS_LIST_DTOR, vec![base + 0xf0]),
            (RIGID_BODY_MAP_DTOR, vec![base + 0xe0]),
            (QUEUE_DTOR, vec![base + 0xa0]),
            (QUEUE_DTOR, vec![base + 0x60]),
            (UPDATE_LIST_DTOR, vec![base + 0x3c]),
            (REFS_LIST_DTOR, vec![base + 0x34]),
            (REFS_LIST_DTOR, vec![base + 0x2c]),
            (MAP_DTOR, vec![base + 0x1c]),
            (CRITICAL_SECTION_DTOR, vec![base]),
        ];
        assert_eq!(destroyed, expected);
    }

    #[test]
    fn the_accessor_constructs_the_instance_once() {
        let mut e = member_engine();
        let exit = record(&mut e, ATEXIT, 0);
        start_log(&mut e);
        assert_eq!(fn_006c0720(&mut e).addr(), MANAGER_INSTANCE);
        assert_eq!(e.global::<u32>(MANAGER_INSTANCE_GUARD) & 1, 1);
        assert_eq!(*exit.borrow(), vec![vec![MANAGER_ATEXIT_DESTRUCTOR]]);
        assert_eq!(
            calls(&e, &[CRITICAL_SECTION_CTOR]),
            vec![(CRITICAL_SECTION_CTOR, vec![MANAGER_INSTANCE])]
        );
        let guards = calls(&e, &[SCOPE_GUARD_CTOR]);
        assert_eq!(guards[0].1[1..], [0x2e, 1, SOURCE_PATH, 0xd2]);
        // Second use: no second construction.
        assert_eq!(fn_006c0720(&mut e).addr(), MANAGER_INSTANCE);
        assert_eq!(exit.borrow().len(), 1);
        assert_eq!(calls(&e, &[CRITICAL_SECTION_CTOR]).len(), 1);
    }

    /// Doubles for `Initialize`: the tasklet manager, its group functions and
    /// an allocator that hands out four-byte blocks.
    fn initialize_engine(create: u32, check: u32) -> (Engine, Log, Log) {
        let mut e = engine();
        stub_ret(&mut e, TASKLET_MANAGER_INSTANCE, 0x7000);
        let create_log = record(&mut e, TASKLET_CREATE_TASK_GROUP, create);
        stub_ret(&mut e, TASKLET_GROUP_CHECK, check);
        let allocated: Log = Rc::default();
        let shared = allocated.clone();
        e.register_double(OPERATOR_NEW, move |e, a| {
            let block = e.mem.alloc(a[0]);
            shared.borrow_mut().push(vec![a[0], block]);
            ret(block)
        });
        stub(
            &mut e,
            &[ACTIVATION_LISTENER_BASE_CTOR, ENTITY_LISTENER_BASE_CTOR],
        );
        (e, create_log, allocated)
    }

    #[test]
    fn initialize_stops_when_the_group_cannot_be_made() {
        let (mut e, create, allocated) = initialize_engine(0, 1);
        let this = manager(&mut e);
        assert!(!nav_mesh_obstacle_manager_initialize(&mut e, this));
        assert_eq!(*create.borrow(), vec![vec![0x7000, this.addr() + 0x128]]);
        assert!(allocated.borrow().is_empty());
    }

    #[test]
    fn initialize_stops_when_the_group_check_fails() {
        let (mut e, _, allocated) = initialize_engine(1, 0);
        let this = manager(&mut e);
        assert!(!nav_mesh_obstacle_manager_initialize(&mut e, this));
        assert!(allocated.borrow().is_empty());
    }

    #[test]
    fn initialize_makes_both_listeners() {
        let (mut e, _, allocated) = initialize_engine(1, 1);
        let this = manager(&mut e);
        assert!(nav_mesh_obstacle_manager_initialize(&mut e, this));
        let blocks = allocated.borrow().clone();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0][0], 4);
        assert_eq!(e.mem.u32(this.addr() + 0x120), blocks[0][1]);
        assert_eq!(e.mem.u32(this.addr() + 0x124), blocks[1][1]);
        assert_eq!(e.mem.u32(blocks[0][1]), DEACTIVATION_LISTENER_VTABLE);
        assert_eq!(e.mem.u32(blocks[1][1]), REMOVAL_LISTENER_VTABLE);
    }

    #[test]
    fn the_group_check_and_flush_pass_the_group_to_the_tasklet_manager() {
        let mut e = engine();
        stub_ret(&mut e, TASKLET_MANAGER_INSTANCE, 0x7000);
        let check = record(&mut e, TASKLET_GROUP_CHECK, 1);
        let flush = record(&mut e, TASKLET_GROUP_FLUSH, 0);
        assert!(fn_006c08e0(&mut e, Ptr::new(0x1230)));
        assert_eq!(*check.borrow(), vec![vec![0x7000, 0x1230]]);
        fn_006c09d0(&mut e, Ptr::new(0x1230));
        assert_eq!(*flush.borrow(), vec![vec![0x7000, 0x1230]]);
        stub_ret(&mut e, TASKLET_GROUP_CHECK, 0);
        assert!(!fn_006c08e0(&mut e, Ptr::new(0x1230)));
    }

    #[test]
    fn release_deletes_the_listeners_and_destroys_the_group() {
        let mut e = member_engine();
        stub_ret(&mut e, TASKLET_MANAGER_INSTANCE, 0x7000);
        let flush = record(&mut e, TASKLET_GROUP_FLUSH, 0);
        let destroy = record(&mut e, TASKLET_DESTROY_TASK_GROUP, 0);
        let deleted = record(&mut e, 0x0070_0100, 0);
        e.put_vtable(0x0080_0100, &[0x0070_0100]);
        let this = manager(&mut e);
        let base = this.addr();
        let listener = slot_with(&mut e, 0x0080_0100);
        e.mem.set_u32(base + 0x120, listener);
        // The removal listener is already null.
        nav_mesh_obstacle_manager_release(&mut e, this);
        assert_eq!(*deleted.borrow(), vec![vec![listener, 1]]);
        assert_eq!(e.mem.u32(base + 0x120), 0);
        assert_eq!(*flush.borrow(), vec![vec![0x7000, base + 0x128]]);
        assert_eq!(*destroy.borrow(), vec![vec![0x7000, base + 0x128]]);
    }

    /// A double that records the argument words plus the word the argument
    /// at `index` points at, and returns `value`.
    fn record_deref(e: &mut Engine, address: u32, value: u32, index: usize) -> Log {
        let log: Log = Rc::default();
        let shared = log.clone();
        e.register_double(address, move |e, a| {
            let mut row = a.to_vec();
            row.push(e.mem.u32(a[index]));
            shared.borrow_mut().push(row);
            ret(value)
        });
        log
    }

    /// One reference array in `FormIDMap`, with two obstacles; the first has
    /// one recorded navmesh.
    struct ClearScene {
        e: Engine,
        this: Ptr<NavMeshObstacleManager>,
        removed: Log,
        loaded: Log,
        util: Log,
    }

    fn clear_scene(queued_to_remove: bool, draw: bool) -> ClearScene {
        let mut e = member_engine();
        let this = manager(&mut e);
        let base = this.addr();
        let table = hashed_map(&mut e, base + 0x1c);
        let _ = table;
        let key = 0x42;
        let first = obstacle(&mut e, 0x501, 0, false, &[0x6001]);
        let second = obstacle(&mut e, 0x502, 0, false, &[]);
        let array = reference_array(&mut e, key, &[first, second]);
        let item = map_item(&mut e, 0, key, array);
        stub_ret(&mut e, MAP_FIRST_ITEM, item);
        stub_ret(&mut e, LIST_CONTAINS, queued_to_remove as u32);
        let removed = record(&mut e, REMOVE_RIGID_BODY, 0);
        e.register(NAVMESH_HOLDER_CTOR, |e, a| {
            e.mem.set_u32(a[0], 0x9999);
            Ret::default()
        });
        let loaded = record(&mut e, NAVMESH_INFO_LOAD, 1);
        let util = record(&mut e, NAVMESH_UTIL_CALL, 0);
        stub(&mut e, &[NAVMESH_HOLDER_DTOR]);
        e.mem.set_u8(base + 0x198, draw as u8);
        ClearScene {
            e,
            this,
            removed,
            loaded,
            util,
        }
    }

    #[test]
    fn clear_all_obstacles_removes_rigid_bodies_and_notifies_navmeshes() {
        let mut scene = clear_scene(false, true);
        let base = scene.this.addr();
        start_log(&mut scene.e);
        nav_mesh_obstacle_manager_clear_all_obstacles(&mut scene.e, scene.this);
        assert_eq!(
            *scene.removed.borrow(),
            vec![vec![base, 0x501], vec![base, 0x502]]
        );
        assert_eq!(scene.loaded.borrow().len(), 1);
        assert_eq!(scene.loaded.borrow()[0][0], 0x6001);
        assert_eq!(*scene.util.borrow(), vec![vec![0x9999]]);
        let tail = calls(
            &scene.e,
            &[
                MAP_CLEAR,
                LIST_CLEAR,
                UPDATE_LIST_CLEAR,
                QUEUE_CLEAR,
                DRAW_OBSTACLES,
            ],
        );
        assert_eq!(
            tail,
            vec![
                (MAP_CLEAR, vec![base + 0x1c]),
                (MAP_CLEAR, vec![base + 0x100]),
                (MAP_CLEAR, vec![base + 0x110]),
                (MAP_CLEAR, vec![base + 0xe0]),
                (LIST_CLEAR, vec![base + 0xf0]),
                (LIST_CLEAR, vec![base + 0xf8]),
                (LIST_CLEAR, vec![base + 0x2c]),
                (LIST_CLEAR, vec![base + 0x34]),
                (UPDATE_LIST_CLEAR, vec![base + 0x3c]),
                (QUEUE_CLEAR, vec![base + 0x60, 0]),
                (QUEUE_CLEAR, vec![base + 0xa0, 0]),
                (DRAW_OBSTACLES, vec![base, 0]),
            ]
        );
    }

    #[test]
    fn clear_all_obstacles_keeps_queued_removals_and_skips_the_drawing() {
        let mut scene = clear_scene(true, false);
        start_log(&mut scene.e);
        nav_mesh_obstacle_manager_clear_all_obstacles(&mut scene.e, scene.this);
        assert!(scene.removed.borrow().is_empty());
        assert_eq!(scene.loaded.borrow().len(), 1);
        assert!(calls(&scene.e, &[DRAW_OBSTACLES]).is_empty());
    }

    #[test]
    fn adding_a_reference_queues_it_once_when_the_setting_is_on() {
        let mut e = engine();
        let this = manager(&mut e);
        let queue = this.addr() + 0x2c;
        let added = record_word(&mut e, LIST_ADD_HEAD);
        let contains = record(&mut e, LIST_CONTAINS, 0);
        // Setting off: nothing happens.
        set_setting(&mut e, false);
        nav_mesh_obstacle_manager_add_obstacle_for_reference(&mut e, this, 0xabc);
        assert!(contains.borrow().is_empty());
        // Setting on, not queued yet.
        set_setting(&mut e, true);
        nav_mesh_obstacle_manager_add_obstacle_for_reference(&mut e, this, 0xabc);
        assert_eq!(*added.borrow(), vec![vec![queue, 0xabc]]);
        // Already queued.
        stub_ret(&mut e, LIST_CONTAINS, 1);
        nav_mesh_obstacle_manager_add_obstacle_for_reference(&mut e, this, 0xabc);
        assert_eq!(added.borrow().len(), 1);
    }

    struct RemoveScene {
        e: Engine,
        this: Ptr<NavMeshObstacleManager>,
        reference: u32,
        from_add_queue: Log,
        rigid_bodies: Log,
        tail: Log,
    }

    /// `RemoveObstacleForReference` with the setting on; `FormIDMap` has an
    /// entry (two obstacles) when `entry` is set.
    fn remove_scene(already_queued: bool, entry: bool) -> RemoveScene {
        let mut e = engine();
        set_setting(&mut e, true);
        let from_add_queue = record_word(&mut e, LIST_REMOVE);
        stub_ret(&mut e, LIST_CONTAINS, already_queued as u32);
        let rigid_bodies = record(&mut e, REMOVE_RIGID_BODY, 0);
        let tail = record_word(&mut e, LIST_ADD_TAIL);
        let first = obstacle(&mut e, 0x501, 0, false, &[]);
        let second = obstacle(&mut e, 0x502, 0, false, &[]);
        let array = reference_array(&mut e, 0x44, &[first, second]);
        e.register_double(MAP_GET, move |e, a| {
            if entry {
                e.mem.set_u32(a[2], array);
            }
            ret(entry as u32)
        });
        let this = manager(&mut e);
        let reference = reference_with_id(&mut e, 0x44);
        RemoveScene {
            e,
            this,
            reference,
            from_add_queue,
            rigid_bodies,
            tail,
        }
    }

    #[test]
    fn removing_a_reference_asks_for_each_rigid_body_and_queues_the_id() {
        let mut s = remove_scene(false, true);
        let base = s.this.addr();
        nav_mesh_obstacle_manager_remove_obstacle_for_reference(&mut s.e, s.this, s.reference);
        assert_eq!(
            *s.from_add_queue.borrow(),
            vec![vec![base + 0x2c, s.reference]]
        );
        assert_eq!(
            *s.rigid_bodies.borrow(),
            vec![vec![base, 0x501], vec![base, 0x502]]
        );
        assert_eq!(*s.tail.borrow(), vec![vec![base + 0x34, 0x44]]);
    }

    #[test]
    fn removing_a_reference_without_an_entry_queues_nothing() {
        let mut s = remove_scene(false, false);
        nav_mesh_obstacle_manager_remove_obstacle_for_reference(&mut s.e, s.this, s.reference);
        assert_eq!(s.from_add_queue.borrow().len(), 1);
        assert!(s.rigid_bodies.borrow().is_empty());
        assert!(s.tail.borrow().is_empty());
    }

    #[test]
    fn removing_a_reference_already_queued_for_removal_stops_early() {
        let mut s = remove_scene(true, true);
        nav_mesh_obstacle_manager_remove_obstacle_for_reference(&mut s.e, s.this, s.reference);
        assert_eq!(s.from_add_queue.borrow().len(), 1);
        assert!(s.rigid_bodies.borrow().is_empty());
        assert!(s.tail.borrow().is_empty());
        // Setting off: not even the add queue is touched.
        let mut s = remove_scene(false, true);
        set_setting(&mut s.e, false);
        nav_mesh_obstacle_manager_remove_obstacle_for_reference(&mut s.e, s.this, s.reference);
        assert!(s.from_add_queue.borrow().is_empty());
    }

    /// The door handlers' scene: a reference of form id `0x44` whose base
    /// form has the given type; the obstacle setting is off (so the
    /// add/remove calls stop at the setting, which is counted).
    struct DoorScene {
        e: Engine,
        this: Ptr<NavMeshObstacleManager>,
        reference: u32,
        setting_reads: Log,
        added: Log,
        removed: Log,
    }

    fn door_scene(
        form_type: u32,
        sliding: bool,
        in_closed: bool,
        in_open: bool,
        contained_offsets: &[u32],
    ) -> DoorScene {
        let mut e = engine();
        let this = manager(&mut e);
        let base = this.addr();
        let reference = reference_with_id(&mut e, 0x44);
        stub_ret(&mut e, REFERENCE_BASE_FORM, 0x3333);
        stub_ret(&mut e, FORM_TYPE, form_type);
        stub_ret(&mut e, IS_SLIDING_DOOR, sliding as u32);
        let value = e.mem.alloc(4);
        let setting_reads = record(&mut e, SETTING_VALUE_ADDRESS, value);
        e.register_double(MAP_GET, move |_, a| {
            ret(((a[0] == base + 0x110 && in_closed) || (a[0] == base + 0x100 && in_open)) as u32)
        });
        let contained: Vec<u32> = contained_offsets
            .iter()
            .map(|offset| base + offset)
            .collect();
        e.register_double(LIST_CONTAINS, move |_, a| {
            ret(contained.contains(&a[0]) as u32)
        });
        let added = record_word(&mut e, LIST_ADD_HEAD);
        let removed = record_word(&mut e, LIST_REMOVE);
        DoorScene {
            e,
            this,
            reference,
            setting_reads,
            added,
            removed,
        }
    }

    #[test]
    fn the_navmesh_queues_take_the_id_at_the_head() {
        let mut e = engine();
        let this = manager(&mut e);
        let base = this.addr();
        let added = record_word(&mut e, LIST_ADD_HEAD);
        fn_006c1130(&mut e, this, 0x51);
        fn_006c1150(&mut e, this, 0x52);
        assert_eq!(
            *added.borrow(),
            vec![vec![base + 0x180, 0x51], vec![base + 0x188, 0x52]]
        );
    }

    #[test]
    fn the_update_request_sets_the_flag() {
        let mut e = engine();
        let this = manager(&mut e);
        assert_eq!(e.mem.u8(this.addr() + 0x18), 0);
        fn_006c1170(&mut e, this);
        assert_eq!(e.mem.u8(this.addr() + 0x18), 1);
    }

    #[test]
    fn the_rigid_body_queues_push_the_value() {
        let mut e = engine();
        let this = manager(&mut e);
        let base = this.addr();
        let pushes = record_word(&mut e, QUEUE_PUSH);
        fn_006c1c70(&mut e, this, 0x501);
        fn_006c1c90(&mut e, this, 0x502);
        assert_eq!(
            *pushes.borrow(),
            vec![vec![base + 0x60, 0x501], vec![base + 0xa0, 0x502]]
        );
    }

    #[test]
    fn the_reference_obstacle_array_constructor_builds_the_base_and_the_array() {
        let mut e = engine();
        let base = record(&mut e, REFERENCE_COUNTED_CTOR, 0);
        let array = record(&mut e, OBSTACLE_ARRAY_CTOR, 0);
        let this = e.mem.alloc(0x1c);
        e.mem.set_u32(this + 8, 0xdead);
        assert_eq!(
            reference_obstacle_array_construct(&mut e, Ptr::new(this)).addr(),
            this
        );
        assert_eq!(e.mem.u32(this), REFERENCE_OBSTACLE_ARRAY_VTABLE);
        assert_eq!(e.mem.u32(this + 8), 0);
        assert_eq!(*base.borrow(), vec![vec![this]]);
        assert_eq!(*array.borrow(), vec![vec![this + 0x0c]]);
    }

    #[test]
    fn the_reference_obstacle_array_destructor_tears_down_the_array_and_the_base() {
        let mut e = engine();
        let base = record(&mut e, REFERENCE_COUNTED_DTOR, 0);
        let array = record(&mut e, OBSTACLE_ARRAY_DTOR, 0);
        let this = e.mem.alloc(0x1c);
        e.mem.set_u32(this + 8, 0xdead);
        reference_obstacle_array_destructor(&mut e, Ptr::new(this));
        assert_eq!(e.mem.u32(this), REFERENCE_OBSTACLE_ARRAY_VTABLE);
        assert_eq!(e.mem.u32(this + 8), 0);
        assert_eq!(*array.borrow(), vec![vec![this + 0x0c]]);
        assert_eq!(*base.borrow(), vec![vec![this]]);
    }

    #[test]
    fn the_reference_obstacle_array_scalar_deleting_destructor_frees_when_asked() {
        let mut e = engine();
        stub(&mut e, &[REFERENCE_COUNTED_DTOR, OBSTACLE_ARRAY_DTOR]);
        let freed = record(&mut e, OPERATOR_DELETE_SIZED, 0);
        let this = e.mem.alloc(0x1c);
        assert_eq!(
            reference_obstacle_array_scalar_deleting_destructor(&mut e, Ptr::new(this), 0).addr(),
            this
        );
        assert!(freed.borrow().is_empty());
        reference_obstacle_array_scalar_deleting_destructor(&mut e, Ptr::new(this), 1);
        assert_eq!(*freed.borrow(), vec![vec![this, 0x1c]]);
    }

    #[test]
    fn the_havok_registrations_need_an_entity() {
        let mut e = engine();
        let activation = record(&mut e, ENTITY_ADD_ACTIVATION_LISTENER, 0);
        let entity_listener = record(&mut e, ENTITY_ADD_ENTITY_LISTENER, 0);
        stub_ret(&mut e, RIGID_BODY_ENTITY, 0);
        fn_006c1dd0(&mut e, Ptr::new(0x501), 0x111);
        fn_006c1da0(&mut e, Ptr::new(0x501), 0x222);
        assert!(activation.borrow().is_empty());
        assert!(entity_listener.borrow().is_empty());
        stub_ret(&mut e, RIGID_BODY_ENTITY, 0x9000);
        fn_006c1dd0(&mut e, Ptr::new(0x501), 0x111);
        fn_006c1da0(&mut e, Ptr::new(0x501), 0x222);
        assert_eq!(*activation.borrow(), vec![vec![0x9000, 0x111]]);
        assert_eq!(*entity_listener.borrow(), vec![vec![0x9000, 0x222]]);
    }

    #[test]
    fn both_listeners_are_registered_on_a_rigid_body() {
        let mut e = engine();
        let this = manager(&mut e);
        e.mem.set_u32(this.addr() + 0x120, 0x111);
        e.mem.set_u32(this.addr() + 0x124, 0x222);
        let activation = record(&mut e, ENTITY_ADD_ACTIVATION_LISTENER, 0);
        let entity_listener = record(&mut e, ENTITY_ADD_ENTITY_LISTENER, 0);
        let entities = record(&mut e, RIGID_BODY_ENTITY, 0x9000);
        fn_006c1d60(&mut e, this, 0x501);
        assert_eq!(*entities.borrow(), vec![vec![0x501], vec![0x501]]);
        assert_eq!(*activation.borrow(), vec![vec![0x9000, 0x111]]);
        assert_eq!(*entity_listener.borrow(), vec![vec![0x9000, 0x222]]);
    }

    #[test]
    fn on_obstacle_lost_removes_the_reference_the_obstacle_belongs_to() {
        let mut e = engine();
        let this = manager(&mut e);
        let array = reference_array(&mut e, 0x44, &[]);
        let data = obstacle(&mut e, 0x501, array, false, &[]);
        e.register_double(MAP_GET, move |e, a| {
            e.mem.set_u32(a[2], data);
            ret(1)
        });
        let lookups = record(&mut e, FORM_LOOKUP_BY_ID, 0x7777);
        // The setting is off, so RemoveObstacleForReference stops at its
        // first read of it.
        let value = e.mem.alloc(4);
        let reads = record(&mut e, SETTING_VALUE_ADDRESS, value);
        nav_mesh_obstacle_manager_on_obstacle_lost3d(&mut e, this, 0x501);
        assert_eq!(*lookups.borrow(), vec![vec![0x44]]);
        assert_eq!(reads.borrow().len(), 1);
        // The form is gone: nothing more.
        stub_ret(&mut e, FORM_LOOKUP_BY_ID, 0);
        nav_mesh_obstacle_manager_on_obstacle_lost3d(&mut e, this, 0x501);
        assert_eq!(reads.borrow().len(), 1);
        // The obstacle is not known: no form lookup at all.
        e.register_double(MAP_GET, |_, _| ret(0));
        let lookups = record(&mut e, FORM_LOOKUP_BY_ID, 0x7777);
        nav_mesh_obstacle_manager_on_obstacle_lost3d(&mut e, this, 0x501);
        assert!(lookups.borrow().is_empty());
    }

    #[test]
    fn the_first_door_handler_takes_a_waiting_door_off_and_queues_its_id() {
        // The door waits in QueuedListOfDoorsToAdd; it is in neither map.
        let mut s = door_scene(0x1c, false, false, false, &[0xf0]);
        let base = s.this.addr();
        fn_006c0dc0(&mut s.e, s.this, s.reference);
        assert_eq!(s.setting_reads.borrow().len(), 1);
        assert_eq!(*s.removed.borrow(), vec![vec![base + 0xf0, s.reference]]);
        assert_eq!(*s.added.borrow(), vec![vec![base + 0xf8, 0x44]]);
        // Not waiting: only the removal queue changes.
        let mut s = door_scene(0x1c, false, false, false, &[]);
        let base = s.this.addr();
        fn_006c0dc0(&mut s.e, s.this, s.reference);
        assert!(s.removed.borrow().is_empty());
        assert_eq!(*s.added.borrow(), vec![vec![base + 0xf8, 0x44]]);
    }

    #[test]
    fn the_first_door_handler_leaves_doors_in_the_maps_alone() {
        let mut s = door_scene(0x1c, false, true, true, &[0xf0]);
        fn_006c0dc0(&mut s.e, s.this, s.reference);
        assert!(s.removed.borrow().is_empty());
        assert!(s.added.borrow().is_empty());
        // Already queued for removal.
        let mut s = door_scene(0x1c, false, false, false, &[0xf8]);
        fn_006c0dc0(&mut s.e, s.this, s.reference);
        assert!(s.added.borrow().is_empty());
    }

    #[test]
    fn the_first_door_handler_skips_the_obstacle_for_other_forms_and_sliding_doors() {
        let mut s = door_scene(0x1b, false, true, true, &[]);
        fn_006c0dc0(&mut s.e, s.this, s.reference);
        assert!(s.setting_reads.borrow().is_empty());
        let mut s = door_scene(0x1c, true, true, true, &[]);
        fn_006c0dc0(&mut s.e, s.this, s.reference);
        assert!(s.setting_reads.borrow().is_empty());
    }

    #[test]
    fn on_door_close_dequeues_the_removal_and_queues_the_door_for_adding() {
        // The form id is queued for removal; the door is not queued for
        // adding; it is in neither map.
        let mut s = door_scene(0x1c, false, false, false, &[0xf8]);
        let base = s.this.addr();
        nav_mesh_obstacle_manager_on_door_close(&mut s.e, s.this, s.reference);
        assert_eq!(s.setting_reads.borrow().len(), 1);
        assert_eq!(*s.removed.borrow(), vec![vec![base + 0xf8, 0x44]]);
        assert_eq!(*s.added.borrow(), vec![vec![base + 0xf0, s.reference]]);
    }

    #[test]
    fn on_door_close_changes_nothing_for_doors_in_the_maps() {
        let mut s = door_scene(0x1b, false, true, true, &[0xf8, 0xf0]);
        nav_mesh_obstacle_manager_on_door_close(&mut s.e, s.this, s.reference);
        assert!(s.setting_reads.borrow().is_empty());
        assert!(s.removed.borrow().is_empty());
        assert!(s.added.borrow().is_empty());
        // Not in the maps, already queued for adding: nothing is added.
        let mut s = door_scene(0x1b, false, false, false, &[0xf0]);
        nav_mesh_obstacle_manager_on_door_close(&mut s.e, s.this, s.reference);
        assert!(s.added.borrow().is_empty());
    }

    #[test]
    fn the_third_door_handler_forgets_a_door_waiting_to_be_added() {
        let mut s = door_scene(0x1c, false, false, false, &[0xf0]);
        let base = s.this.addr();
        fn_006c1060(&mut s.e, s.this, s.reference);
        assert_eq!(s.setting_reads.borrow().len(), 1);
        assert_eq!(
            *s.removed.borrow(),
            vec![
                vec![base + 0x2c, s.reference],
                vec![base + 0xf0, s.reference]
            ]
        );
        assert!(s.added.borrow().is_empty());
    }

    #[test]
    fn the_third_door_handler_queues_other_doors_for_removal_once() {
        let mut s = door_scene(0x1b, false, false, false, &[]);
        let base = s.this.addr();
        fn_006c1060(&mut s.e, s.this, s.reference);
        assert!(s.setting_reads.borrow().is_empty());
        assert_eq!(*s.removed.borrow(), vec![vec![base + 0x2c, s.reference]]);
        assert_eq!(*s.added.borrow(), vec![vec![base + 0xf8, 0x44]]);
        // Already queued.
        let mut s = door_scene(0x1b, false, false, false, &[0xf8]);
        fn_006c1060(&mut s.e, s.this, s.reference);
        assert!(s.added.borrow().is_empty());
    }

    /// A manager with every callee of the obstacle (re)registration
    /// functions replaced by a recording double.
    struct Scene {
        e: Engine,
        this: Ptr<NavMeshObstacleManager>,
        map_set: Log,
        map_remove: Log,
        queue_push: Log,
        refresh: Log,
        add_node: Log,
        remove_node: Log,
        operations: Log,
        array_add: Log,
        array_clear: Log,
        remove_rigid_body: Log,
        entities: Log,
        list_checks: Log,
    }

    /// `draw` sets `bDrawObstacles`; every rigid body is active; the
    /// overlap query finds one navmesh (`0x8000`, whose info is `0x8100`
    /// and whose task is `0x9100`).
    fn scene(draw: bool) -> Scene {
        let mut e = engine();
        let this = manager(&mut e);
        let base = this.addr();
        e.mem.set_u8(base + 0x198, draw as u8);
        e.mem.set_u32(base + 0x120, 0x111);
        e.mem.set_u32(base + 0x124, 0x222);
        let map_set = record(&mut e, MAP_SET, 0);
        let map_remove = record(&mut e, MAP_REMOVE, 0);
        let queue_push = record_word(&mut e, QUEUE_PUSH);
        let refresh = record_deref(&mut e, OBSTACLE_REFRESH, 0, 0);
        let add_node = record_deref(&mut e, ADD_BOUNDING_BOX_NODE, 0, 1);
        let remove_node = record_deref(&mut e, REMOVE_BOUNDING_BOX_NODE, 0, 1);
        e.register(OBSTACLE_TASK_FOR_NAVMESH, |_, a| ret(a[1] + 0x1000));
        let operations = record_deref(&mut e, OBSTACLE_TASK_ADD_OPERATION, 0, 1);
        let array_add = record_word(&mut e, ARRAY_ADD);
        let array_clear = record(&mut e, ARRAY_CLEAR, 0);
        e.register(
            ARRAY_IS_EMPTY,
            |e, a| ret((e.mem.u32(a[0] + 8) == 0) as u32),
        );
        let remove_rigid_body = record(&mut e, REMOVE_RIGID_BODY, 0);
        let entities = record(&mut e, RIGID_BODY_ENTITY, 0);
        stub_ret(&mut e, RIGID_BODY_IS_ACTIVE, 1);
        stub(
            &mut e,
            &[NAVMESH_ARRAY_CTOR, NAVMESH_ARRAY_DTOR, OBSTACLE_PREPARE],
        );
        e.register(OBSTACLE_OVERLAPPING_NAVMESHES, |e, a| {
            set_array(e, a[1], &[0x8000]);
            ret(1)
        });
        e.register(NAVMESH_INFO, |_, a| ret(a[0] + 0x100));
        e.register(FIRST_WORD_EQUALS, |_, _| ret(0));
        let list_checks = record(&mut e, LIST_IS_EMPTY, 1);
        Scene {
            e,
            this,
            map_set,
            map_remove,
            queue_push,
            refresh,
            add_node,
            remove_node,
            operations,
            array_add,
            array_clear,
            remove_rigid_body,
            entities,
            list_checks,
        }
    }

    /// Makes `MAP_GET` find `array` for any key.
    fn find_array(e: &mut Engine, array: u32) {
        e.register_double(MAP_GET, move |e, a| {
            e.mem.set_u32(a[2], array);
            ret(1)
        });
    }

    #[test]
    fn registering_a_reference_with_an_existing_array_enters_its_obstacles() {
        let mut s = scene(true);
        let base = s.this.addr();
        let o1 = obstacle(&mut s.e, 0x501, 0, true, &[]);
        let array = reference_array(&mut s.e, 0x44, &[o1]);
        find_array(&mut s.e, array);
        let reference = reference_with_id(&mut s.e, 0x44);
        fn_006c1190(&mut s.e, s.this, reference);
        // No new array: the only SetAt is the rigid body map's.
        assert_eq!(*s.map_set.borrow(), vec![vec![base + 0xe0, 0x501, o1]]);
        // Havok listeners on the rigid body, and it is queued (it is active).
        assert_eq!(*s.entities.borrow(), vec![vec![0x501], vec![0x501]]);
        assert_eq!(*s.queue_push.borrow(), vec![vec![base + 0x60, 0x501]]);
        // Refresh and the debug node get the address of the obstacle's pointer.
        let slot = s.refresh.borrow()[0][0];
        assert_eq!(*s.refresh.borrow(), vec![vec![slot, o1]]);
        assert_eq!(*s.add_node.borrow(), vec![vec![base, slot, o1]]);
        // Operation 0x10 on the overlapped navmesh's task, and the navmesh's
        // info recorded in the obstacle.
        assert_eq!(*s.operations.borrow(), vec![vec![0x9100, slot, 0x10, o1]]);
        assert_eq!(*s.array_add.borrow(), vec![vec![o1 + 0x78, 0x8100]]);
    }

    #[test]
    fn registering_a_reference_builds_its_array_when_there_is_none() {
        let mut s = scene(false);
        let base = s.this.addr();
        let o1 = obstacle(&mut s.e, 0x501, 0, false, &[]);
        let block = s.e.mem.alloc(0x1c);
        let allocated = record(&mut s.e, OPERATOR_NEW_SIZED, block);
        stub(&mut s.e, &[REFERENCE_COUNTED_CTOR, OBSTACLE_ARRAY_CTOR]);
        s.e.register_double(FILL_REFERENCE_OBSTACLES, move |e, a| {
            let array = e.mem.u32(a[1]);
            set_array(e, array + 0x0c, &[o1]);
            Ret::default()
        });
        stub_ret(&mut s.e, MAP_GET, 0);
        let reference = reference_with_id(&mut s.e, 0x44);
        fn_006c1190(&mut s.e, s.this, reference);
        assert_eq!(*allocated.borrow(), vec![vec![0x1c]]);
        assert_eq!(s.e.mem.u32(block), REFERENCE_OBSTACLE_ARRAY_VTABLE);
        assert_eq!(
            *s.map_set.borrow(),
            vec![vec![base + 0x1c, 0x44, block], vec![base + 0xe0, 0x501, o1]]
        );
        // Not active, no drawing: no navmesh work.
        assert!(s.operations.borrow().is_empty());
        assert!(s.add_node.borrow().is_empty());
        assert_eq!(s.refresh.borrow().len(), 1);
    }

    #[test]
    fn unregistering_a_reference_by_id_removes_its_obstacles() {
        let mut s = scene(true);
        let base = s.this.addr();
        let o1 = obstacle(&mut s.e, 0x501, 0, true, &[0x6001]);
        let o2 = obstacle(&mut s.e, 0x502, 0, true, &[]);
        let array = reference_array(&mut s.e, 0x44, &[o1, o2]);
        find_array(&mut s.e, array);
        fn_006c1590(&mut s.e, s.this, 0x44);
        assert_eq!(
            *s.map_remove.borrow(),
            vec![
                vec![base + 0x1c, 0x44],
                vec![base + 0xe0, 0x501],
                vec![base + 0xe0, 0x502],
            ]
        );
        // The update list is asked to drop each obstacle (it is empty here).
        assert_eq!(
            *s.list_checks.borrow(),
            vec![vec![base + 0x3c], vec![base + 0x3c]]
        );
        assert_eq!(s.remove_node.borrow().len(), 2);
        // Only the obstacle with recorded navmeshes queues operation 0x20 and
        // is cleared.
        let slot = s.remove_node.borrow()[0][1];
        assert_eq!(*s.operations.borrow(), vec![vec![0x7001, slot, 0x20, o1]]);
        assert_eq!(*s.array_clear.borrow(), vec![vec![o1 + 0x78, 1]]);
    }

    #[test]
    fn unregistering_an_unknown_reference_does_nothing() {
        let mut s = scene(true);
        stub_ret(&mut s.e, MAP_GET, 0);
        fn_006c1590(&mut s.e, s.this, 0x44);
        assert!(s.map_remove.borrow().is_empty());
        assert!(s.operations.borrow().is_empty());
    }

    #[test]
    fn re_registering_an_active_obstacle_cancels_and_redoes_its_navmesh_work() {
        let mut s = scene(true);
        let base = s.this.addr();
        let o1 = obstacle(&mut s.e, 0x501, 0, true, &[0x6001]);
        let slot = slot_with(&mut s.e, o1);
        let prepare = record(&mut s.e, OBSTACLE_PREPARE, 0);
        fn_006c1760(&mut s.e, s.this, slot);
        assert_eq!(
            *s.operations.borrow(),
            vec![vec![0x7001, slot, 0x21, o1], vec![0x9100, slot, 0x11, o1],]
        );
        assert_eq!(*s.array_clear.borrow(), vec![vec![o1 + 0x78, 1]]);
        assert_eq!(*prepare.borrow(), vec![vec![o1]]);
        assert_eq!(*s.refresh.borrow(), vec![vec![slot, o1]]);
        assert_eq!(*s.add_node.borrow(), vec![vec![base, slot, o1]]);
        assert_eq!(*s.array_add.borrow(), vec![vec![o1 + 0x78, 0x8100]]);
    }

    #[test]
    fn re_registering_an_inactive_obstacle_only_refreshes_it() {
        let mut s = scene(false);
        let o1 = obstacle(&mut s.e, 0x501, 0, false, &[0x6001]);
        let slot = slot_with(&mut s.e, o1);
        let prepare = record(&mut s.e, OBSTACLE_PREPARE, 0);
        fn_006c1760(&mut s.e, s.this, slot);
        assert!(s.operations.borrow().is_empty());
        assert!(s.array_clear.borrow().is_empty());
        assert!(s.add_node.borrow().is_empty());
        assert_eq!(*prepare.borrow(), vec![vec![o1]]);
        assert_eq!(*s.refresh.borrow(), vec![vec![slot, o1]]);
    }

    #[test]
    fn replacing_the_obstacles_of_a_reference_removes_the_old_and_registers_the_new() {
        let mut s = scene(true);
        let base = s.this.addr();
        let o1 = obstacle(&mut s.e, 0x501, 0, true, &[0x6001]);
        let o2 = obstacle(&mut s.e, 0x502, 0, false, &[]);
        let array = reference_array(&mut s.e, 0x44, &[o1, o2]);
        find_array(&mut s.e, array);
        let n1 = obstacle(&mut s.e, 0x503, 0, false, &[]);
        let refill = record(&mut s.e, OBSTACLE_ARRAY_REFRESH, 0);
        s.e.register_double(OBSTACLE_ARRAY_REFRESH, move |e, a| {
            let array = e.mem.u32(a[1]);
            set_array(e, array + 0x0c, &[n1]);
            Ret::default()
        });
        let _ = refill;
        fn_006c1920(&mut s.e, s.this, 0x44, 0xabcd);
        // Old obstacles: navmesh work of the active one, debug nodes, rigid
        // bodies, map entries, update list.
        assert_eq!(s.operations.borrow().len(), 1);
        assert_eq!(s.operations.borrow()[0][2..], [0x21, o1]);
        assert_eq!(s.operations.borrow()[0][0], 0x7001);
        assert_eq!(*s.array_clear.borrow(), vec![vec![o1 + 0x78, 1]]);
        assert_eq!(s.remove_node.borrow().len(), 2);
        assert_eq!(
            *s.remove_rigid_body.borrow(),
            vec![vec![base, 0x501], vec![base, 0x502]]
        );
        assert_eq!(
            *s.map_remove.borrow(),
            vec![vec![base + 0xe0, 0x501], vec![base + 0xe0, 0x502]]
        );
        assert_eq!(s.list_checks.borrow().len(), 2);
        // New obstacle: refreshed, drawn, its rigid body gets the listeners.
        assert_eq!(s.refresh.borrow().len(), 1);
        assert_eq!(s.refresh.borrow()[0][1], n1);
        assert_eq!(s.add_node.borrow().len(), 1);
        assert_eq!(*s.entities.borrow(), vec![vec![0x503], vec![0x503]]);
    }

    #[test]
    fn replacing_the_obstacles_of_an_unknown_reference_does_nothing() {
        let mut s = scene(true);
        stub_ret(&mut s.e, MAP_GET, 0);
        let refill = record(&mut s.e, OBSTACLE_ARRAY_REFRESH, 0);
        fn_006c1920(&mut s.e, s.this, 0x44, 0xabcd);
        assert!(refill.borrow().is_empty());
        assert!(s.remove_rigid_body.borrow().is_empty());
    }
}
