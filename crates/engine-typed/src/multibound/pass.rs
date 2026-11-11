//! The two passes main runs over the loaded world once a frame (the frame function `0086e650`, between
//! the interface's update and the scene's draw): [`remove`] (`0045bc80`) takes every object's
//! multibound shape back to "not tested" and [`add`] (`0045b070`) tests each again from the camera
//! ([`super::occlusion::add_object`]). The objects: an interior cell's multibound nodes (the 3D root's
//! child 7), or each loaded grid cell's multibound nodes (child 7) and land (child 2); the children of
//! the shadow scene node's child 6; and the water objects (each water group's entries whose first child
//! is a water entry). Then the portal graph's held and queued objects, when the scene node has a graph.

use super::getters;
use super::occlusion;
use super::FAMILY;
use super::{GridCellArray, TES};
use crate::{culling, dispatch, draw, pathing};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The global the passes hand to an empty function (`00483710`) before and after.
const WALK_MARKER: u32 = 0x011c_3c0f;
/// The visibility pass's frame: the culling process (an `NiCullingProcess`, 0x90 bytes) and right
/// after it the camera position's three words, as in the exe's frame (EBP-0xb4 and EBP-0x24), so the
/// process's word +0x90 that the object test reads as its mode is the position's x.
const PROCESS_SIZE: u32 = 0x90;
const FRAME_SIZE: u32 = PROCESS_SIZE + 12;
/// The culling process's constructor (`thiscall(process, 0)`), camera set-up and destructor (its
/// frustum set-up is the culling family's), and the address of its planes (`005d8a70`; a word pushed
/// before it stays as the next call's last).
const PROCESS_CONSTRUCT: u32 = 0x00a6_9400;
const PROCESS_SET_CAMERA: u32 = 0x0041_fd00;
const PROCESS_DESTRUCT: u32 = 0x00a6_93e0;
const PROCESS_PLANES: u32 = 0x005d_8a70;
/// `uGridsToLoad`, the loaded grid's side.
const SETTING_GRIDS_TO_LOAD: u32 = 0x011c_63cc;
/// The world space's terrain manager (`00586170`, recursive over parent world spaces) and the manager's
/// calls before each pass (`006fd080` with the camera, `006fd130`).
const WORLD_SPACE_TERRAIN: u32 = 0x0058_6170;
const TERRAIN_BEFORE_ADD: u32 = 0x006f_d080;
const TERRAIN_BEFORE_REMOVE: u32 = 0x006f_d130;
/// The `NiRTTI` a water entry's first child has.
const WATER_ENTRY_RTTI: u32 = 0x0120_2e74;
/// Vtable slots: an interior object's +0x104 (the passes take those that answer 0), `UpdateWorldBound`
/// (+0xbc), the object as a node (+0xc, null for another class), a water entry's node (+0x1d0), and
/// whether an object accepts a point (+0x108, a byte).
const SLOT_104: u32 = 0x104;
const SLOT_UPDATE_WORLD_BOUND: u32 = 0xbc;
const SLOT_AS_NODE: u32 = 0xc;
const SLOT_WATER_NODE: u32 = 0x1d0;
const SLOT_ACCEPTS_POINT: u32 = 0x108;
/// The portal graph's calls: its list +0x30 emptied (`0045bc60`), its `NiPointer` +0x2c set
/// (`0045bc40`), its held object (`0056c7f0`), its tracked list (`00413f40`) and its queue
/// (`00891170`); the queue's emptiness (`0076b610`) and the list walk (`0057cbe0`, `thiscall(list,
/// &position)`, the address of the item it passed).
const GRAPH_CLEAR_LIST: u32 = 0x0045_bc60;
const GRAPH_SET_HELD: u32 = 0x0045_bc40;
const GRAPH_HELD: u32 = 0x0056_c7f0;
const GRAPH_TRACKED: u32 = 0x0041_3f40;
const GRAPH_QUEUE: u32 = 0x0089_1170;
const QUEUE_IS_EMPTY: u32 = 0x0076_b610;
const LIST_ITERATE: u32 = 0x0057_cbe0;
/// `NiPointer` constructor (`00633c90`), assignment (`0066b0d0`) and copy constructor (`00559a40`).
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
const NI_POINTER_COPY: u32 = 0x0055_9a40;
/// The shadow scene node's candidate (`00b5f170`, `thiscall(scene, object, word, planes, flag)`).
const ADD_CANDIDATE: u32 = 0x00b5_f170;
/// A tracked item's word +0x40 cleared (`0045c4a0`), its target's cell (`0045c4c0`); a queued object's
/// state (`0045c570`), its object at +0xcc (`0045c4e0`), its two item lists (`006ab360`, `00514f30`);
/// the object behind a cell (`004f9bf0`), a compound frustum whose camera is then cleared
/// (`BSCompoundFrustum::SetCamera`, `00c47980`, the culling family's, with no camera).
const ITEM_CLEAR: u32 = 0x0045_c4a0;
const ITEM_TARGET_CELL: u32 = 0x0045_c4c0;
const OBJECT_SET_STATE: u32 = 0x0045_c570;
const OBJECT_SET_CC: u32 = 0x0045_c4e0;
const OBJECT_ITEMS_A: u32 = 0x006a_b360;
const OBJECT_ITEMS_B: u32 = 0x0051_4f30;
const CELL_OBJECT: u32 = 0x004f_9bf0;

/// The stored pointer of the `NiPointer` cell at `cell` (`00559450`).
fn pointer_at(e: &mut Engine, cell: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, cell))
}

/// `uGridsToLoad`, read through `0043d4d0` at each use, as the loops do.
fn grids_to_load(e: &mut Engine) -> u32 {
    let value = typed_call!(
        FAMILY,
        0x0043_d4d0,
        getters::setting_value(e, Ptr::new(SETTING_GRIDS_TO_LOAD))
    );
    e.mem.u32(value)
}

/// The scene graph's first shadow scene node (`00450b80(0)`).
fn first_scene(e: &mut Engine) -> u32 {
    typed_call!(FAMILY, 0x0045_0b80, getters::shadow_scene_node(e, 0))
}

/// The portal graph of `scene` (`00454b30`).
fn graph_of(e: &mut Engine, scene: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x0045_4b30,
        getters::portal_graph(e, Ptr::new(scene))
    )
}

/// The node's child count (`0043b480`), read again before every step.
fn child_count(e: &mut Engine, node: u32) -> u32 {
    u32::from(typed_call!(
        dispatch::FAMILY,
        0x0043_b480,
        dispatch::scene::child_count(e, Ptr::new(node))
    ))
}

/// The node's child `index` (`0043b4a0`).
fn child(e: &mut Engine, node: u32, index: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0043_b4a0,
        dispatch::scene::child(e, Ptr::new(node), index)
    )
    .addr()
}

/// Calls `visit` with each child of `node` that is not null, the count read again before each step.
fn visit_children(e: &mut Engine, node: u32, visit: &mut dyn FnMut(&mut Engine, u32)) {
    let mut index = 0;
    while index < child_count(e, node) {
        let object = child(e, node, index);
        if object != 0 {
            visit(e, object);
        }
        index += 1;
    }
}

/// Calls `visit` with each child of an interior cell's list `node` that is not null and whose slot
/// +0x104 answers 0.
fn visit_interior(e: &mut Engine, node: u32, visit: &mut dyn FnMut(&mut Engine, u32)) {
    let mut index = 0;
    while index < child_count(e, node) {
        let object = child(e, node, index);
        if object != 0 && e.vcall(object, SLOT_104, &[]).u32() == 0 {
            visit(e, object);
        }
        index += 1;
    }
}

/// The cell list `index` of `cell` (`00456fc0`).
fn cell_child(e: &mut Engine, cell: u32, index: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x0045_6fc0,
        getters::cell_child(e, Ptr::new(cell), index)
    )
}

/// The cell in slot (x, y) of the loaded grid (`TES::pGridCellA`, re-read each time; the slot through
/// `004ba490`, read as the exe reads it, without a null test).
fn grid_cell_at(e: &mut Engine, this: Ptr<TES>, x: u32, y: u32) -> u32 {
    let grid: Ptr<GridCellArray> = Ptr::new(e.get(this, TES::pGridCellA));
    let slot = typed_call!(FAMILY, 0x004b_a490, getters::grid_cell(e, grid, x, y));
    e.mem.u32(slot)
}

/// The terrain manager of the world space, or null without a world space (each read through
/// `004fd3e0`, the manager by address).
fn terrain(e: &mut Engine, this: Ptr<TES>) -> u32 {
    if typed_call!(FAMILY, 0x004f_d3e0, getters::world_space(e, this)) == 0 {
        return 0;
    }
    let world_space = typed_call!(FAMILY, 0x004f_d3e0, getters::world_space(e, this));
    e.call(WORLD_SPACE_TERRAIN, &args![world_space]).u32()
}

/// The water system (`TES::pWaterSystem`, through `0070ec90`).
fn water_system(e: &mut Engine, this: Ptr<TES>) -> u32 {
    typed_call!(FAMILY, 0x0070_ec90, getters::word_at_64(e, this.cast()))
}

/// The water system's list of water groups (its address, `005a8080`).
fn water_groups(e: &mut Engine, this: Ptr<TES>) -> u32 {
    let system = water_system(e, this);
    typed_call!(
        FAMILY,
        0x005a_8080,
        getters::address_3c(e, Ptr::new(system))
    )
}

/// The position after `position` in `list` (`007b52d0`).
fn next_position(e: &mut Engine, list: u32, position: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x007b_52d0,
        getters::next_position(e, Ptr::new(list), Ptr::new(position))
    )
}

/// The item of `position` in `list` (the word at `006317a0`'s address).
fn item_at(e: &mut Engine, list: u32, position: u32) -> u32 {
    let address = typed_call!(
        FAMILY,
        0x0063_17a0,
        getters::item_address(e, Ptr::new(list), Ptr::new(position))
    );
    e.mem.u32(address)
}

/// A water group's water objects (its list at +0x24): for each entry (after `007af430`, whose answer
/// is not used), the first child of its slot +0x1d0 node when that child is a water entry
/// (`0045bad0`), read again for `visit`.
fn visit_water_group(e: &mut Engine, group: u32, visit: &mut dyn FnMut(&mut Engine, u32)) {
    let list = group.wrapping_add(0x24);
    let mut position = pointer_at(e, list);
    while position != 0 {
        let next = next_position(e, list, position);
        let entry = item_at(e, list, position);
        typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, entry));
        let node = e.vcall(entry, SLOT_WATER_NODE, &[]).u32();
        let first = child(e, node, 0);
        if typed_call!(
            FAMILY,
            0x0045_bad0,
            getters::is_exactly(e, WATER_ENTRY_RTTI, Ptr::new(first))
        ) {
            let object = child(e, node, 0);
            visit(e, object);
        }
        position = next;
    }
}

/// The water objects of every water group of the water system's list, then of its LOD water group
/// (`006733e0`), when there is a water system; the list re-read at each step, as the exe reads it.
fn visit_water(e: &mut Engine, this: Ptr<TES>, visit: &mut dyn FnMut(&mut Engine, u32)) {
    if water_system(e, this) == 0 {
        return;
    }
    let list = water_groups(e, this);
    let mut position = pointer_at(e, list);
    while position != 0 {
        let list = water_groups(e, this);
        let next = next_position(e, list, position);
        let list = water_groups(e, this);
        let group = item_at(e, list, position);
        visit_water_group(e, group, visit);
        position = next;
    }
    let system = water_system(e, this);
    let lod = typed_call!(
        FAMILY,
        0x0067_33e0,
        getters::word_at_48(e, Ptr::new(system))
    );
    if lod != 0 {
        visit_water_group(e, lod, visit);
    }
}

/// The children of the scene node's child 6 (`0045bc00`, as a node through its slot +0xc).
fn visit_scene_child_6(e: &mut Engine, scene: u32, visit: &mut dyn FnMut(&mut Engine, u32)) {
    let child = typed_call!(
        FAMILY,
        0x0045_bc00,
        getters::node_child(e, Ptr::new(scene), 6)
    );
    let node = if child != 0 {
        e.vcall(child, SLOT_AS_NODE, &[]).u32()
    } else {
        0
    };
    if node != 0 {
        visit_children(e, node, visit);
    }
}

// Typed from 0045b070
/// The visibility pass (`thiscall(tes, camera)`, `RET 4`): with a culling process on the stack set to
/// the camera and its frustum (`0045bbe0`), and the camera's position (`0045bb80`, three words kept
/// after the process), each object of the loaded world is tested by the first shadow scene node
/// ([`occlusion::add_object`]): an interior cell's list 7 entries whose slot +0x104 answers 0; outdoors,
/// after the terrain manager's set-up for the camera, each loaded grid cell's lists 7 (when there is
/// one) and 2; then the scene node's child 6's children and the water objects. With a portal graph,
/// its held object is offered again ([`offer_held_object`]). The empty function `00483710` is called
/// with `011c3c0f` before and after.
pub fn add(e: &mut Engine, this: Ptr<TES>, camera: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0048_3710,
        dispatch::refs::nothing_1(e, WALK_MARKER)
    );
    e.with_stack(FRAME_SIZE, |e, process| {
        let process = process.addr();
        let position = process.wrapping_add(PROCESS_SIZE);
        e.call(PROCESS_CONSTRUCT, &args![process, 0u32]);
        e.call(PROCESS_SET_CAMERA, &args![process, camera]);
        let frustum = typed_call!(
            draw::FAMILY,
            0x0045_bbe0,
            draw::packet::camera_port(e, camera)
        );
        typed_call!(
            culling::FAMILY,
            0x00a6_94a0,
            culling::process::set_frustum(e, Ptr::new(process), Ptr::new(frustum))
        );
        let location = typed_call!(
            draw::FAMILY,
            0x0045_bb80,
            draw::packet::world_translate(e, camera)
        );
        for word in 0..3u32 {
            let value = e.mem.u32(location.wrapping_add(4 * word));
            e.mem.set_u32(position.wrapping_add(4 * word), value);
        }
        let scene = first_scene(e);
        let mut test = |e: &mut Engine, object: u32| {
            let [x, y, z] = [0u32, 4, 8].map(|k| e.mem.u32(position.wrapping_add(k)));
            typed_call!(
                FAMILY,
                0x00b5_bbe0,
                occlusion::add_object(e, Ptr::new(scene), object, x, y, z, process)
            );
        };
        let interior = e.get(this, TES::pInteriorCell);
        if interior != 0 {
            let list = cell_child(e, interior, 7);
            if list != 0 {
                visit_interior(e, list, &mut test);
            }
        } else {
            let manager = terrain(e, this);
            if manager != 0 {
                e.call(TERRAIN_BEFORE_ADD, &args![manager, camera]);
            }
            let mut x = 0;
            while x < grids_to_load(e) {
                let mut y = 0;
                while y < grids_to_load(e) {
                    let cell = grid_cell_at(e, this, x, y);
                    if cell != 0 {
                        let list = cell_child(e, cell, 7);
                        if list != 0 {
                            visit_children(e, list, &mut test);
                        }
                        let list = cell_child(e, cell, 2);
                        visit_children(e, list, &mut test);
                    }
                    y += 1;
                }
                x += 1;
            }
        }
        visit_scene_child_6(e, scene, &mut test);
        visit_water(e, this, &mut test);
        let first = first_scene(e);
        if graph_of(e, first) != 0 {
            offer_held_object(e, camera, scene, Ptr::new(process));
        }
        typed_call!(
            dispatch::FAMILY,
            0x0048_3710,
            dispatch::refs::nothing_1(e, WALK_MARKER)
        );
        e.call(PROCESS_DESTRUCT, &args![process]);
    });
}

/// The portal graph part of [`add`]: the graph's list +0x30 is emptied and a point `near` in front of
/// the camera (its rotation's first column times the frustum's near distance, plus its position) is
/// made. The held object (an `NiPointer` cell from `0056c7f0`) stays only if its slot +0x108 accepts the
/// point (the cell is cleared otherwise) and is offered with flag 1 (`00b5f170`); with none, each
/// queued object that accepts the point is offered, the first filling the cell and turning the flag
/// to 0; with still none, a null object is offered. The held object becomes the graph's (`0045bc40`).
fn offer_held_object(e: &mut Engine, camera: u32, scene: u32, process: Ptr) {
    let first = first_scene(e);
    let graph = graph_of(e, first);
    e.call(GRAPH_CLEAR_LIST, &args![graph]);
    let first = first_scene(e);
    let graph = graph_of(e, first);
    let held_object = e.call(GRAPH_HELD, &args![graph]).u32();
    // The held cell (4 bytes), then three points of 12 bytes: the column, the scaled column, the point.
    e.with_stack(4 + 36, |e, cell| {
        e.call(NI_POINTER_CONSTRUCT, &args![cell, held_object]);
        let held = cell.addr();
        let column_point = held.wrapping_add(4);
        let scaled_point = held.wrapping_add(16);
        let point = held.wrapping_add(28);
        let frustum = typed_call!(
            draw::FAMILY,
            0x0045_bbe0,
            draw::packet::camera_port(e, camera)
        );
        // `FLD [frustum + 0x10]; FSTP [ESP]`: the near distance pushed through the x87 stack.
        let near = x87_quiet_bits(e.mem.u32(frustum.wrapping_add(0x10)));
        let column = typed_call!(
            draw::FAMILY,
            0x0045_bba0,
            draw::packet::rotation_column_0(e, camera, column_point)
        );
        let scaled = typed_call!(
            dispatch::FAMILY,
            0x0045_bb20,
            dispatch::math::times(
                e,
                Ptr::new(column),
                Ptr::new(scaled_point),
                f32::from_bits(near)
            )
        );
        let location = typed_call!(
            draw::FAMILY,
            0x0045_bb80,
            draw::packet::world_translate(e, camera)
        );
        typed_call!(
            pathing::FAMILY,
            0x0043_9e90,
            pathing::point::add(e, Ptr::new(location), Ptr::new(point), scaled)
        );
        let mut flag = 1u32;
        if pointer_at(e, held) != 0 {
            let object = pointer_at(e, held);
            if e.vcall(object, SLOT_ACCEPTS_POINT, &args![point]).u8() == 0 {
                e.call(NI_POINTER_ASSIGN, &args![held, 0u32]);
            } else {
                let object = pointer_at(e, held);
                offer(e, scene, process, object, 1);
            }
        }
        if pointer_at(e, held) == 0 {
            let graph = graph_of(e, scene);
            let queue = e.call(GRAPH_QUEUE, &args![graph]).u32();
            if e.call(QUEUE_IS_EMPTY, &args![queue]).u8() == 0 {
                e.with_stack(4, |e, position| {
                    let start = pointer_at(e, queue);
                    e.mem.set_u32(position.addr(), start);
                    while e.mem.u32(position.addr()) != 0 {
                        let address = e.call(LIST_ITERATE, &args![queue, position]).u32();
                        let item = pointer_at(e, address);
                        if item != 0 && e.vcall(item, SLOT_ACCEPTS_POINT, &args![point]).u8() != 0 {
                            offer(e, scene, process, item, flag);
                            if pointer_at(e, held) == 0 {
                                e.call(NI_POINTER_ASSIGN, &args![held, item]);
                                flag = 0;
                            }
                        }
                    }
                });
            }
        }
        if pointer_at(e, held) == 0 {
            offer(e, scene, process, 0, flag);
        }
        let value = pointer_at(e, held);
        let graph = graph_of(e, scene);
        e.call(GRAPH_SET_HELD, &args![graph, value]);
        typed_call!(
            dispatch::FAMILY,
            0x0045_cec0,
            dispatch::refs::release_held(e, Ptr::new(held))
        );
    });
}

/// `00b5f170(scene, object, word, planes, flag)` with the culling process's planes (`005d8a70`) and
/// its word +0xc (`0084e3a0`), read in that order.
fn offer(e: &mut Engine, scene: u32, process: Ptr, object: u32, flag: u32) {
    let planes = e.call(PROCESS_PLANES, &args![process]).u32();
    let word = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, process));
    e.call(ADD_CANDIDATE, &args![scene, object, word, planes, flag]);
}

/// The reset of one object: its multibound's shape (`006629f0`, `0043b230`) back to "not tested"
/// (`00528820`).
fn reset(e: &mut Engine, object: u32) {
    let bound = typed_call!(
        FAMILY,
        0x0066_29f0,
        getters::multibound_of(e, Ptr::new(object))
    );
    let shape = typed_call!(
        dispatch::FAMILY,
        0x0043_b230,
        dispatch::scene::first_controller(e, Ptr::new(bound))
    );
    typed_call!(FAMILY, 0x0052_8820, getters::clear_cull_result(e, shape));
}

// Typed from 0045bc80
/// The reset pass (`thiscall(tes, with_land, with_queue)`, `RET 8`): every object of the loaded world
/// gets its multibound shape back to "not tested": an interior cell's list 7 entries whose slot +0x104
/// answers 0; outdoors, after the terrain manager's call, each loaded grid cell's list 7 entries (each
/// first updating its world bound, slot +0xbc) and, with `with_land`, its list 2's; then the scene
/// node's child 6's children. With a portal graph, its tracked items are released
/// ([`release_tracked`]); then the water objects.
pub fn remove(e: &mut Engine, this: Ptr<TES>, with_land: u8, with_queue: u8) {
    let interior = e.get(this, TES::pInteriorCell);
    if interior != 0 {
        let list = cell_child(e, interior, 7);
        if list != 0 {
            visit_interior(e, list, &mut reset);
        }
    } else {
        let manager = terrain(e, this);
        if manager != 0 {
            e.call(TERRAIN_BEFORE_REMOVE, &args![manager]);
        }
        let mut x = 0;
        while x < grids_to_load(e) {
            let mut y = 0;
            while y < grids_to_load(e) {
                let cell = grid_cell_at(e, this, x, y);
                if cell != 0 {
                    let list = cell_child(e, cell, 7);
                    if list != 0 {
                        visit_children(e, list, &mut |e, object| {
                            e.vcall(object, SLOT_UPDATE_WORLD_BOUND, &[]);
                            reset(e, object);
                        });
                    }
                    if with_land != 0 {
                        let list = cell_child(e, cell, 2);
                        visit_children(e, list, &mut reset);
                    }
                }
                y += 1;
            }
            x += 1;
        }
    }
    let scene = first_scene(e);
    visit_scene_child_6(e, scene, &mut reset);
    let first = first_scene(e);
    if graph_of(e, first) != 0 {
        release_tracked(e, with_queue != 0);
    }
    visit_water(e, this, &mut reset);
}

/// The portal graph part of [`remove`]: each tracked item (the graph's tracked list, re-read at each
/// step) gets its word +0x40 cleared (`0045c4a0`); then, with `with_queue` and a queue that is not
/// empty, each queued object held in a copied `NiPointer` ([`release_queued`]).
fn release_tracked(e: &mut Engine, with_queue: bool) {
    let tracked = |e: &mut Engine| {
        let first = first_scene(e);
        let graph = graph_of(e, first);
        e.call(GRAPH_TRACKED, &args![graph]).u32()
    };
    let list = tracked(e);
    e.with_stack(4, |e, position| {
        let start = pointer_at(e, list);
        e.mem.set_u32(position.addr(), start);
        while e.mem.u32(position.addr()) != 0 {
            let list = tracked(e);
            let address = e.call(LIST_ITERATE, &args![list, position]).u32();
            let item = e.mem.u32(address);
            if item != 0 {
                e.call(ITEM_CLEAR, &args![item]);
            }
        }
    });
    let first = first_scene(e);
    let graph = graph_of(e, first);
    let queue = e.call(GRAPH_QUEUE, &args![graph]).u32();
    if e.call(QUEUE_IS_EMPTY, &args![queue]).u8() != 0 || !with_queue {
        return;
    }
    e.with_stack(4, |e, position| {
        let start = pointer_at(e, queue);
        e.mem.set_u32(position.addr(), start);
        while e.mem.u32(position.addr()) != 0 {
            let address = e.call(LIST_ITERATE, &args![queue, position]).u32();
            e.with_stack(4, |e, held| {
                e.call(NI_POINTER_COPY, &args![held, address]);
                if pointer_at(e, held.addr()) != 0 {
                    release_queued(e, held.addr());
                }
                typed_call!(
                    dispatch::FAMILY,
                    0x0045_cec0,
                    dispatch::refs::release_held(e, held)
                );
            });
        }
    });
}

/// One queued object of [`release_tracked`], held by the `NiPointer` cell `held`: its world bound
/// (slot +0xbc), its state 3 (`0045c570`), its object at +0xcc cleared (`0045c4e0(0)`); each item of
/// its first list gets its word +0x40 cleared and, when the object behind its target's cell is not
/// null, that object's compound frustum without a camera; each item of its second list gets its word +0x40 cleared.
fn release_queued(e: &mut Engine, held: u32) {
    let object = pointer_at(e, held);
    e.vcall(object, SLOT_UPDATE_WORLD_BOUND, &[]);
    let object = pointer_at(e, held);
    e.call(OBJECT_SET_STATE, &args![object, 3u32]);
    let object = pointer_at(e, held);
    e.call(OBJECT_SET_CC, &args![object, 0u32]);
    let object = pointer_at(e, held);
    let list = e.call(OBJECT_ITEMS_A, &args![object]).u32();
    e.with_stack(4, |e, position| {
        let start = pointer_at(e, list);
        e.mem.set_u32(position.addr(), start);
        while e.mem.u32(position.addr()) != 0 {
            let object = pointer_at(e, held);
            let list = e.call(OBJECT_ITEMS_A, &args![object]).u32();
            let address = e.call(LIST_ITERATE, &args![list, position]).u32();
            let item = e.mem.u32(address);
            if item != 0 {
                e.call(ITEM_CLEAR, &args![item]);
                let cell = e.call(ITEM_TARGET_CELL, &args![item]).u32();
                if e.call(CELL_OBJECT, &args![cell]).u32() != 0 {
                    let cell = e.call(ITEM_TARGET_CELL, &args![item]).u32();
                    let target = e.call(CELL_OBJECT, &args![cell]).u32();
                    typed_call!(
                        culling::FAMILY,
                        0x00c4_7980,
                        culling::compound::set_camera(e, Ptr::new(target), 0)
                    );
                }
            }
        }
    });
    let object = pointer_at(e, held);
    let list = e.call(OBJECT_ITEMS_B, &args![object]).u32();
    e.with_stack(4, |e, position| {
        let start = pointer_at(e, list);
        e.mem.set_u32(position.addr(), start);
        while e.mem.u32(position.addr()) != 0 {
            let object = pointer_at(e, held);
            let list = e.call(OBJECT_ITEMS_B, &args![object]).u32();
            let address = e.call(LIST_ITERATE, &args![list, position]).u32();
            let item = e.mem.u32(address);
            if item != 0 {
                e.call(ITEM_CLEAR, &args![item]);
            }
        }
    });
}
