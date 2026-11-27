//! The `NavMeshObstacleManager`'s navmesh queues, the update flag and the reference/obstacle
//! bookkeeping (`006c1190`, `006c1590`, `006c1760`).

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

use super::{navmesh, FAMILY};

/// Callees.
const NI_POINTER_INIT: u32 = 0x0063_3c90;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
const NI_POINTER_COPY: u32 = 0x0055_9a40;
const NI_POINTER_SET: u32 = 0x0066_b0d0;
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
const ARRAY_ADD: u32 = 0x007c_b2e0;
const ARRAY_CLEAR: u32 = 0x0084_54f0;
const ARRAY_IS_EMPTY: u32 = 0x0076_b610;
const MAP_GET: u32 = 0x006c_62d0;
const MAP_SET: u32 = 0x006c_6920;
const MAP_REMOVE: u32 = 0x0040_5430;
const OPERATOR_NEW_SIZED: u32 = 0x00aa_13e0;
const REFERENCE_OBSTACLE_ARRAY_CTOR: u32 = 0x006c_1480;
const FILL_REFERENCE_OBSTACLES: u32 = 0x006c_34f0;
const LIST_REMOVE_OBSTACLE_DATA: u32 = 0x0063_1620;
const REGISTER_LISTENERS: u32 = 0x006c_1d60;
const QUEUE_OBSTACLE_ADD: u32 = 0x006c_1c70;
const RIGID_BODY_IS_ACTIVE: u32 = 0x0056_09b0;
const OBSTACLE_PREPARE: u32 = 0x006c_2650;
const OBSTACLE_REFRESH: u32 = 0x006c_3440;
const ADD_BOUNDING_BOX_NODE: u32 = 0x006c_5b20;
const REMOVE_BOUNDING_BOX_NODE: u32 = 0x006c_5c50;
const NAVMESH_ARRAY_CTOR: u32 = 0x006c_6c60;
const NAVMESH_ARRAY_DTOR: u32 = 0x006c_6ce0;
const OBSTACLE_OVERLAPPING_NAVMESHES: u32 = 0x006c_2050;
const OBSTACLE_TASK_FOR_NAVMESH: u32 = 0x006c_5ef0;
const OBSTACLE_TASK_ADD_OPERATION: u32 = 0x006c_7a30;

/// The `NavMeshObstacleManager` field offsets.
const B_UPDATE_ALL_OBSTACLES: u32 = 0x18;
const FORM_ID_MAP: u32 = 0x1c;
const OBSTACLES_UPDATE_LIST: u32 = 0x3c;
const RIGID_BODY_TO_OBSTACLE_MAP: u32 = 0xe0;
const QUEUED_NAVMESHES_TO_ENABLE: u32 = 0x180;
const QUEUED_NAVMESHES_TO_DISABLE: u32 = 0x188;
const B_DRAW_OBSTACLES: u32 = 0x198;

/// The `ReferenceObstacleArray` and `ObstacleData` field offsets.
const OBSTACLES: u32 = 0x0c;
const P_RIGID_BODY: u32 = 0x0c;
const B_ACTIVE: u32 = 0x74;
const NAVMESHES: u32 = 0x78;

/// `NiPointer::operator->`: the pointer held in the slot.
fn pointer_get(e: &mut Engine, slot: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot))
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
    typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, array))
}

/// `BSSimpleArray::operator[]`: the address of element `index`.
fn array_at(e: &mut Engine, array: u32, index: u32) -> u32 {
    typed_call!(
        leaf::FAMILY,
        0x0087_7a30,
        leaf::array_element(e, array, index)
    )
}

/// The `Obstacles` array of the `ReferenceObstacleArray` held by `slot`.
fn obstacles_of(e: &mut Engine, slot: u32) -> u32 {
    pointer_get(e, slot) + OBSTACLES
}

/// The `Navmeshes` array of the `ObstacleData` held by `slot`.
fn navmeshes_of(e: &mut Engine, slot: u32) -> u32 {
    pointer_get(e, slot) + NAVMESHES
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
    pointer_get(e, obstacle + P_RIGID_BODY)
}

/// Whether the `ObstacleData` held by `slot` is active.
fn obstacle_is_active(e: &mut Engine, slot: u32) -> bool {
    let obstacle = pointer_get(e, slot);
    e.mem.u8(obstacle + B_ACTIVE) != 0
}

/// A list or queue operation that takes the address of a word value.
fn list_operation(e: &mut Engine, operation: u32, list: u32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(operation, &args![list, slot]);
    })
}

/// `map.GetAt(key, &out)`: true when the key is present; the value is put in the `NiPointer` at
/// `out`.
fn map_get(e: &mut Engine, map: u32, key: u32, out: u32) -> bool {
    e.call(MAP_GET, &args![map, key, out]).bool()
}

/// `map.SetAt(key, value)` with the value a fresh copy of the `NiPointer` at `source`.
fn map_set_copy(e: &mut Engine, map: u32, key: u32, source: u32) {
    e.with_stack(4, |e, temporary| {
        pointer_copy(e, temporary.addr(), source);
        let value = e.mem.u32(temporary.addr());
        e.call(MAP_SET, &args![map, key, value]);
    })
}

/// `ObstacleTaskData::AddOperation` on the task of `nav_mesh_info`.
fn queue_operation(e: &mut Engine, this: Ptr, nav_mesh_info: u32, obstacle: u32, operation: u32) {
    let task = e
        .call(OBSTACLE_TASK_FOR_NAVMESH, &args![this, nav_mesh_info])
        .u32();
    e.call(
        OBSTACLE_TASK_ADD_OPERATION,
        &args![task, obstacle, operation],
    );
}

/// For an obstacle: asks `006c2050` for the navmeshes it overlaps; for each one queues
/// `operation` on its task and records its `NavMeshInfo` in the obstacle's `Navmeshes`.
fn queue_overlapping_navmeshes(e: &mut Engine, this: Ptr, obstacle: u32, operation: u32) {
    // The navmesh scrap array is 0x14 bytes (006c6c60 sets its scrap heap at +0x10).
    e.with_stack(0x14, |e, found| {
        let found = found.addr();
        e.call(NAVMESH_ARRAY_CTOR, &args![found]);
        if e.call(OBSTACLE_OVERLAPPING_NAVMESHES, &args![obstacle, found])
            .bool()
        {
            let mut index = 0;
            while index < array_size(e, found) {
                let element = array_at(e, found, index);
                let navmesh = pointer_get(e, element);
                let info: u32 =
                    typed_call!(FAMILY, 0x0068_efb0, navmesh::info(e, Ptr::new(navmesh)));
                queue_operation(e, this, info, obstacle, operation);
                let element = array_at(e, found, index);
                let navmesh = pointer_get(e, element);
                let info: u32 =
                    typed_call!(FAMILY, 0x0068_efb0, navmesh::info(e, Ptr::new(navmesh)));
                let navmeshes = navmeshes_of(e, obstacle);
                list_operation(e, ARRAY_ADD, navmeshes, info);
                index += 1;
            }
        }
        e.call(NAVMESH_ARRAY_DTOR, &args![found]);
    });
}

/// Queues `operation` on the task of every `NavMeshInfo` in the obstacle's `Navmeshes`, then
/// clears that array (`Clear(1)`).
fn queue_recorded_navmeshes(e: &mut Engine, this: Ptr, obstacle: u32, operation: u32) {
    let mut index = 0;
    while index < navmesh_count(e, obstacle) {
        let navmeshes = navmeshes_of(e, obstacle);
        let element = array_at(e, navmeshes, index);
        let info = e.mem.u32(element);
        queue_operation(e, this, info, obstacle, operation);
        index += 1;
    }
    let navmeshes = navmeshes_of(e, obstacle);
    e.call(ARRAY_CLEAR, &args![navmeshes, 1u32]);
}

// Typed from 006c1130
/// Queues a navmesh form id in `QueuedListOfNavmeshesToEnable`: `AddHead` of the value (no
/// duplicate check).
pub fn queue_navmesh_enable(e: &mut Engine, this: Ptr, navmesh_id: u32) {
    list_operation(
        e,
        LIST_ADD_HEAD,
        this.addr() + QUEUED_NAVMESHES_TO_ENABLE,
        navmesh_id,
    );
}

// Typed from 006c1150
/// Queues a navmesh form id in `QueuedListOfNavmeshesToDisable`: `AddHead` of the value (no
/// duplicate check).
pub fn queue_navmesh_disable(e: &mut Engine, this: Ptr, navmesh_id: u32) {
    list_operation(
        e,
        LIST_ADD_HEAD,
        this.addr() + QUEUED_NAVMESHES_TO_DISABLE,
        navmesh_id,
    );
}

// Typed from 006c1170
/// Requests an update of all obstacles: sets `bUpdateAllObstacles`.
pub fn request_update_all(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + B_UPDATE_ALL_OBSTACLES, 1);
}

// Typed from 006c1190
/// Adds the obstacles of a reference: finds or builds the reference's `ReferenceObstacleArray`
/// in `FormIDMap` (a new one is filled by `006c34f0`); then, for each of its obstacles: enters
/// it in `RigidBodyToObstacleMap` under its rigid body, registers the Havok listeners
/// (`006c1d60`), queues the rigid body when it is active (`006c1c70`), refreshes the obstacle
/// (`006c3440`), adds its debug node when `bDrawObstacles` is set and, when the obstacle is
/// active, queues operation `0x10` on the task of every navmesh it overlaps and records those
/// navmeshes in its `Navmeshes`.
pub fn add_reference_obstacles(e: &mut Engine, this: Ptr, reference: u32) {
    let base = this.addr();
    let form_id_map = base + FORM_ID_MAP;
    e.with_stack(0x10, |e, frame| {
        let array = frame.addr();
        let obstacle = frame.addr() + 4;
        pointer_init(e, array, 0);
        let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
        if !map_get(e, form_id_map, form_id, array) {
            let memory = e.call(OPERATOR_NEW_SIZED, &args![0x1cu32]).u32();
            let created = if memory != 0 {
                e.call(REFERENCE_OBSTACLE_ARRAY_CTOR, &args![memory]).u32()
            } else {
                0
            };
            e.call(NI_POINTER_SET, &args![array, created]);
            e.call(FILL_REFERENCE_OBSTACLES, &args![reference, array]);
            let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
            map_set_copy(e, form_id_map, form_id, array);
        }
        let mut index = 0;
        while index < obstacle_count(e, array) {
            let obstacles = obstacles_of(e, array);
            let element = array_at(e, obstacles, index);
            pointer_copy(e, obstacle, element);
            let rigid_body = rigid_body_of(e, obstacle);
            map_set_copy(e, base + RIGID_BODY_TO_OBSTACLE_MAP, rigid_body, obstacle);
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(REGISTER_LISTENERS, &args![this, rigid_body]);
            let rigid_body = rigid_body_of(e, obstacle);
            if e.call(RIGID_BODY_IS_ACTIVE, &args![rigid_body]).bool() {
                let rigid_body = rigid_body_of(e, obstacle);
                e.call(QUEUE_OBSTACLE_ADD, &args![this, rigid_body]);
            }
            e.call(OBSTACLE_REFRESH, &args![obstacle]);
            if e.mem.u8(base + B_DRAW_OBSTACLES) != 0 {
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

// Typed from 006c1590
/// Removes the obstacles of a reference by form id: if `FormIDMap` has the id, removes the
/// entry and, for each of its obstacles, the `RigidBodyToObstacleMap` entry of its rigid body,
/// the node in `ObstaclesUpdateList`, the debug node when `bDrawObstacles` is set and, when it
/// recorded navmeshes, queues operation `0x20` on each of their tasks and clears the record.
pub fn remove_reference_obstacles(e: &mut Engine, this: Ptr, form_id: u32) {
    let base = this.addr();
    e.with_stack(8, |e, frame| {
        let array = frame.addr();
        let obstacle = frame.addr() + 4;
        pointer_init(e, array, 0);
        if !map_get(e, base + FORM_ID_MAP, form_id, array) {
            pointer_release(e, array);
            return;
        }
        e.call(MAP_REMOVE, &args![base + FORM_ID_MAP, form_id]);
        let mut index = 0;
        while index < obstacle_count(e, array) {
            let obstacles = obstacles_of(e, array);
            let element = array_at(e, obstacles, index);
            pointer_copy(e, obstacle, element);
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(
                MAP_REMOVE,
                &args![base + RIGID_BODY_TO_OBSTACLE_MAP, rigid_body],
            );
            e.call(
                LIST_REMOVE_OBSTACLE_DATA,
                &args![base + OBSTACLES_UPDATE_LIST, obstacle],
            );
            if e.mem.u8(base + B_DRAW_OBSTACLES) != 0 {
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

// Typed from 006c1760
/// Re-registers one obstacle (`obstacle` is the address of its `NiPointer<ObstacleData>`): for
/// an active obstacle queues operation `0x21` on the task of each navmesh it recorded and
/// clears the record; then refreshes it (`006c2650`, `006c3440`), adds its debug node when
/// `bDrawObstacles` is set and, when it is active, queues operation `0x11` on the tasks of the
/// navmeshes it now overlaps and records them.
pub fn reregister_obstacle(e: &mut Engine, this: Ptr, obstacle: u32) {
    if obstacle_is_active(e, obstacle) {
        queue_recorded_navmeshes(e, this, obstacle, 0x21);
    }
    let data = pointer_get(e, obstacle);
    e.call(OBSTACLE_PREPARE, &args![data]);
    e.call(OBSTACLE_REFRESH, &args![obstacle]);
    if e.mem.u8(this.addr() + B_DRAW_OBSTACLES) != 0 {
        e.call(ADD_BOUNDING_BOX_NODE, &args![this, obstacle]);
    }
    if obstacle_is_active(e, obstacle) {
        queue_overlapping_navmeshes(e, this, obstacle, 0x11);
    }
}
