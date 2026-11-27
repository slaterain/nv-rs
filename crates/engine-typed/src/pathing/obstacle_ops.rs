//! The `NavMeshObstacleManager`'s door handlers (`006c0dc0`, `006c1060`), the reference
//! replacement (`006c1920`) and the node-tree obstacle builders (`006c2210`, `006c2550`).

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

use super::FAMILY;

/// Callees.
const NI_POINTER_INIT: u32 = 0x0063_3c90;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
const NI_POINTER_COPY: u32 = 0x0055_9a40;
const NI_POINTER_SET: u32 = 0x0066_b0d0;
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
const LIST_REMOVE: u32 = 0x0090_5330;
const LIST_CONTAINS: u32 = 0x005f_65d0;
const ARRAY_ADD: u32 = 0x007c_b2e0;
const ARRAY_CLEAR: u32 = 0x0084_54f0;
const MAP_GET: u32 = 0x006c_62d0;
const MAP_REMOVE: u32 = 0x0040_5430;
const OPERATOR_NEW_SIZED: u32 = 0x00aa_13e0;
const ADD_OBSTACLE_FOR_REFERENCE: u32 = 0x006c_0c30;
const REMOVE_OBSTACLE_FOR_REFERENCE: u32 = 0x006c_0c80;
const REGISTER_LISTENERS: u32 = 0x006c_1d60;
const REMOVE_RIGID_BODY: u32 = 0x006c_1e00;
const LIST_REMOVE_OBSTACLE_DATA: u32 = 0x0063_1620;
const OBSTACLE_DATA_CTOR: u32 = 0x006c_23c0;
const OBSTACLE_PREPARE: u32 = 0x006c_2650;
const OBSTACLE_REFRESH: u32 = 0x006c_3440;
const OBSTACLE_ARRAY_REFRESH: u32 = 0x006c_3520;
const OBSTACLE_ARRAY_ADD: u32 = 0x006c_61b0;
const ADD_BOUNDING_BOX_NODE: u32 = 0x006c_5b20;
const REMOVE_BOUNDING_BOX_NODE: u32 = 0x006c_5c50;
const FORM_TYPE: u32 = 0x0040_1170;
const IS_SLIDING_DOOR: u32 = 0x0051_8080;
const COLLISION_OBJECT_OF_NODE: u32 = 0x0043_b610;
const COLLISION_OBJECT_RIGID_BODY: u32 = 0x006f_a820;
const RIGID_BODY_MOTION_INFO: u32 = 0x0043_b4f0;
const MOTION_INFO_TYPE: u32 = 0x0043_b4d0;
const CHILDREN_COUNT: u32 = 0x0043_b480;
const CHILDREN_COUNT_SHALLOW: u32 = 0x0045_3470;
const CHILDREN_AT: u32 = 0x0043_b4a0;
const OBSTACLE_TASK_FOR_NAVMESH: u32 = 0x006c_5ef0;
const OBSTACLE_TASK_ADD_OPERATION: u32 = 0x006c_7a30;
const NAVMESH_ARRAY_CTOR: u32 = 0x006c_6c60;
const NAVMESH_ARRAY_DTOR: u32 = 0x006c_6ce0;
const OBSTACLE_OVERLAPPING_NAVMESHES: u32 = 0x006c_2050;

/// The `NavMeshObstacleManager` field offsets.
const QUEUED_REFS_TO_ADD: u32 = 0x2c;
const FORM_ID_MAP: u32 = 0x1c;
const OBSTACLES_UPDATE_LIST: u32 = 0x3c;
const RIGID_BODY_TO_OBSTACLE_MAP: u32 = 0xe0;
const QUEUED_DOORS_TO_ADD: u32 = 0xf0;
const QUEUED_DOORS_TO_REMOVE: u32 = 0xf8;
const OPEN_DOOR_MAP: u32 = 0x100;
const CLOSED_DOOR_MAP: u32 = 0x110;
const B_DRAW_OBSTACLES: u32 = 0x198;

/// The `ReferenceObstacleArray` and `ObstacleData` field offsets.
const OBSTACLES: u32 = 0x0c;
const SP_PARENT_ARRAY: u32 = 0x08;
const P_RIGID_BODY: u32 = 0x0c;
const B_ACTIVE: u32 = 0x74;
const NAVMESHES: u32 = 0x78;

/// The node's children vtable slot (byte offset).
const NODE_CHILDREN_SLOT: u32 = 0x0c;
/// The door form type.
const DOOR_FORM_TYPE: u32 = 0x1c;
/// The `ObstacleData` allocation size.
const OBSTACLE_DATA_SIZE: u32 = 0x8c;

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

/// `map.GetAt(key, &out)`: true when the key is present; the value is put in the `NiPointer` at
/// `out`.
fn map_get(e: &mut Engine, map: u32, key: u32, out: u32) -> bool {
    e.call(MAP_GET, &args![map, key, out]).bool()
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
                let info: u32 = typed_call!(
                    FAMILY,
                    0x0068_efb0,
                    super::navmesh::info(e, Ptr::new(navmesh))
                );
                queue_operation(e, this, info, obstacle, operation);
                let element = array_at(e, found, index);
                let navmesh = pointer_get(e, element);
                let info: u32 = typed_call!(
                    FAMILY,
                    0x0068_efb0,
                    super::navmesh::info(e, Ptr::new(navmesh))
                );
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

/// The test the door handlers open with: `Some(true)` for a reference whose base form is a door
/// that is not a sliding door (the form type is `0x1c` and `IsSlidingDoor` is false).
fn is_ordinary_door(e: &mut Engine, reference: u32) -> Option<bool> {
    let form = typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, reference));
    if e.call(FORM_TYPE, &args![form]).u32() == DOOR_FORM_TYPE {
        return Some(!e.call(IS_SLIDING_DOOR, &args![form]).bool());
    }
    Some(false)
}

/// Whether the collision rigid body `006fa820` finds under a node has one of the motion types
/// `types`.
fn rigid_body_has_motion_type(e: &mut Engine, rigid_body: u32, types: &[u32]) -> bool {
    e.with_stack(0x10, |e, record| {
        let record = e
            .call(RIGID_BODY_MOTION_INFO, &args![rigid_body, record.addr()])
            .u32();
        let kind = e.call(MOTION_INFO_TYPE, &args![record]).u32();
        types.contains(&kind)
    })
}

// Typed from 006c0dc0
/// Door handler (between `RemoveObstacleForReference` and `OnDoorClose`; whether this is the
/// "door opened" handler is inferred, not confirmed): for a non-sliding door reference calls
/// `AddObstacleForReference` (`006c0c30`); then, unless the reference is in `ClosedDoorMap`,
/// takes it out of `QueuedListOfDoorsToAdd` when it is there; then, unless it is in
/// `OpenDoorMap`, queues its form id in `QueuedListOfDoorsToRemove` (`AddHead`) when not
/// queued yet.
pub fn door_opened(e: &mut Engine, this: Ptr, reference: u32) {
    let base = this.addr();
    let Some(ordinary) = is_ordinary_door(e, reference) else {
        return;
    };
    if ordinary {
        e.call(ADD_OBSTACLE_FOR_REFERENCE, &args![this, reference]);
    }
    e.with_stack(4, |e, array| {
        let array = array.addr();
        pointer_init(e, array, 0);
        let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
        let in_closed = map_get(e, base + CLOSED_DOOR_MAP, form_id, array);
        let doors_to_add = base + QUEUED_DOORS_TO_ADD;
        if !in_closed && list_contains(e, doors_to_add, reference) {
            list_operation(e, LIST_REMOVE, doors_to_add, reference);
        }
        let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
        let in_open = map_get(e, base + OPEN_DOOR_MAP, form_id, array);
        if !in_open {
            let queued = base + QUEUED_DOORS_TO_REMOVE;
            let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
            if !list_contains(e, queued, form_id) {
                let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
                list_operation(e, LIST_ADD_HEAD, queued, form_id);
            }
        }
        pointer_release(e, array);
    });
}

// Typed from 006c1060
/// Door-gone handler: for a non-sliding door reference calls `RemoveObstacleForReference`
/// (`006c0c80`); then takes the reference out of `QueuedListOfRefsToAdd`; when it is in
/// `QueuedListOfDoorsToAdd` takes it out of there, else queues its form id in
/// `QueuedListOfDoorsToRemove` (`AddHead`) unless already queued.
pub fn door_gone(e: &mut Engine, this: Ptr, reference: u32) {
    let base = this.addr();
    let Some(ordinary) = is_ordinary_door(e, reference) else {
        return;
    };
    if ordinary {
        e.call(REMOVE_OBSTACLE_FOR_REFERENCE, &args![this, reference]);
    }
    list_operation(e, LIST_REMOVE, base + QUEUED_REFS_TO_ADD, reference);
    let doors_to_add = base + QUEUED_DOORS_TO_ADD;
    if list_contains(e, doors_to_add, reference) {
        list_operation(e, LIST_REMOVE, doors_to_add, reference);
    } else {
        let queued = base + QUEUED_DOORS_TO_REMOVE;
        let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
        if !list_contains(e, queued, form_id) {
            let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
            list_operation(e, LIST_ADD_HEAD, queued, form_id);
        }
    }
}

// Typed from 006c1920
/// Replaces the obstacles of a reference. If `FormIDMap` has `form_id`: for each old obstacle
/// queues operation `0x21` on the tasks of its recorded navmeshes (when it is active, then
/// clears the record), removes its debug node when `bDrawObstacles` is set, calls `006c1e00`
/// for its rigid body and removes the entry from `RigidBodyToObstacleMap` and the node from
/// `ObstaclesUpdateList`. Then `006c3520(other, &array)` refills the array and for each of its
/// obstacles: refreshes it (`006c3440`), queues operation `0x11` on the navmeshes it overlaps
/// (when active), adds its debug node when `bDrawObstacles` is set and registers the Havok
/// listeners (`006c1d60`). `other` is passed unchanged to `006c3520`.
pub fn replace_reference_obstacles(e: &mut Engine, this: Ptr, form_id: u32, other: u32) {
    let base = this.addr();
    e.with_stack(8, |e, frame| {
        let array = frame.addr();
        let obstacle = frame.addr() + 4;
        pointer_init(e, array, 0);
        if !map_get(e, base + FORM_ID_MAP, form_id, array) {
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
            if e.mem.u8(base + B_DRAW_OBSTACLES) != 0 {
                e.call(REMOVE_BOUNDING_BOX_NODE, &args![this, obstacle]);
            }
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(REMOVE_RIGID_BODY, &args![this, rigid_body]);
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(
                MAP_REMOVE,
                &args![base + RIGID_BODY_TO_OBSTACLE_MAP, rigid_body],
            );
            e.call(
                LIST_REMOVE_OBSTACLE_DATA,
                &args![base + OBSTACLES_UPDATE_LIST, obstacle],
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
            if e.mem.u8(base + B_DRAW_OBSTACLES) != 0 {
                e.call(ADD_BOUNDING_BOX_NODE, &args![this, obstacle]);
            }
            let rigid_body = rigid_body_of(e, obstacle);
            e.call(REGISTER_LISTENERS, &args![this, rigid_body]);
            pointer_release(e, obstacle);
            index += 1;
        }
        pointer_release(e, array);
    });
}

// Typed from 006c2210
/// Builds the obstacles of a node's tree (`cdecl`; `array` is the address of the
/// `NiPointer<ReferenceObstacleArray>` that receives them). If the node has a collision object
/// (`0043b610`) with a rigid body (`006fa820`) whose motion type is 2, 4, 10, 20 or 28, a new
/// `ObstacleData` is made (`0x8c` bytes, `006c23c0`), its `spParentArray` set to the array
/// (without taking a reference), its rigid body set, filled by `006c2650` and added to the
/// array's `Obstacles` (`006c61b0`). The children of the node (virtual slot `0x0c`) are then
/// visited the same way. Always returns true.
pub fn collect_node_obstacles(e: &mut Engine, node: u32, array: u32) -> bool {
    let collision_object = e.call(COLLISION_OBJECT_OF_NODE, &args![node]).u32();
    if collision_object != 0 {
        let rigid_body = e
            .call(COLLISION_OBJECT_RIGID_BODY, &args![collision_object])
            .u32();
        if rigid_body != 0 && rigid_body_has_motion_type(e, rigid_body, &[2, 4, 0x0a, 0x14, 0x1c]) {
            let memory = e.call(OPERATOR_NEW_SIZED, &args![OBSTACLE_DATA_SIZE]).u32();
            let created = if memory != 0 {
                e.call(OBSTACLE_DATA_CTOR, &args![memory]).u32()
            } else {
                0
            };
            e.with_stack(4, |e, slot| {
                let slot = slot.addr();
                pointer_init(e, slot, created);
                let parent = pointer_get(e, array);
                let data = pointer_get(e, slot);
                e.mem.set_u32(data + SP_PARENT_ARRAY, parent);
                let data = pointer_get(e, slot);
                e.call(NI_POINTER_SET, &args![data + P_RIGID_BODY, rigid_body]);
                let data = pointer_get(e, slot);
                e.call(OBSTACLE_PREPARE, &args![data]);
                let parent = pointer_get(e, array);
                e.call(OBSTACLE_ARRAY_ADD, &args![parent + OBSTACLES, slot]);
                pointer_release(e, slot);
            });
        }
    }
    let children = e.vcall(node, NODE_CHILDREN_SLOT, &args![]).u32();
    if children != 0 {
        let mut index = 0;
        while index < e.call(CHILDREN_COUNT, &args![children]).u32() {
            if e.call(CHILDREN_AT, &args![children, index]).u32() != 0 {
                let child = e.call(CHILDREN_AT, &args![children, index]).u32();
                let _: bool =
                    typed_call!(FAMILY, 0x006c_2210, collect_node_obstacles(e, child, array));
            }
            index += 1;
        }
    }
    true
}

// Typed from 006c2550
/// Fills one `ObstacleData` from a node's tree (`cdecl`): the first node of the tree whose
/// collision rigid body has motion type 1, 2, 4, 10, 20 or 28 clears `spParentArray`, takes that
/// rigid body, is filled by `006c2650` and ends the search with true. Otherwise the children
/// (virtual slot `0x0c`, counted by `00453470`) are tried in turn; false when none qualifies.
pub fn fill_obstacle_from_node(e: &mut Engine, node: u32, data: u32) -> bool {
    let collision_object = e.call(COLLISION_OBJECT_OF_NODE, &args![node]).u32();
    if collision_object != 0 {
        let rigid_body = e
            .call(COLLISION_OBJECT_RIGID_BODY, &args![collision_object])
            .u32();
        if rigid_body != 0
            && rigid_body_has_motion_type(e, rigid_body, &[2, 1, 4, 0x0a, 0x14, 0x1c])
        {
            e.mem.set_u32(data + SP_PARENT_ARRAY, 0);
            e.call(NI_POINTER_SET, &args![data + P_RIGID_BODY, rigid_body]);
            e.call(OBSTACLE_PREPARE, &args![data]);
            return true;
        }
    }
    let children = e.vcall(node, NODE_CHILDREN_SLOT, &args![]).u32();
    if children != 0 {
        let mut index = 0;
        while index < e.call(CHILDREN_COUNT_SHALLOW, &args![children]).u32() {
            if e.call(CHILDREN_AT, &args![children, index]).u32() != 0 {
                let child = e.call(CHILDREN_AT, &args![children, index]).u32();
                if typed_call!(FAMILY, 0x006c_2550, fill_obstacle_from_node(e, child, data)) {
                    return true;
                }
            }
            index += 1;
        }
    }
    false
}
