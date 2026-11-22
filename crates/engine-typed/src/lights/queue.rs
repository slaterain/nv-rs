//! The scene node's queue drain (`00b5fd60`, from `00b60040` once a frame before the draw) and the
//! object half under it: every object whose lighting must be redone (a character that moved or
//! animated, a cell attached, a script move) gets its light lists redone against the candidate
//! lights (`UpdateQueuedObjectLightingStart` `00b5dac0`, the object walks `00b5c490` and
//! `00b5c5c0`, the per-geometry updates `00b5b260` and `00b5b410`). Every moving or animating
//! character goes through it every frame.
//!
//! Native code with the exe's calls: the bound test (`BoundWithinLightRadius`, typed) and the add
//! (`AddTriGeom`, typed) are reached directly, the rest by address; the candidates in the exe's
//! order, the CRT's `qsort` called with the exe's comparator (`00b5aa70`) and `GetLuminanceToBound`
//! (`00b9dbe0`, which writes each gathered light's luminance) called as the exe calls them.
//!
//! **The light table (`NV_LIGHT_TABLE`, off by default).** The exe's bound test asks each lamp's
//! `NiLight` for its kind (a virtual call) and reads its place and radius again for every geometry
//! it tests. With the switch on, each drain's object half keeps one table of the lamps it met:
//! each `NiLight`'s kind answer, place and radius, read the first time the drain meets it, and the
//! scene node's lighting offset. Each geometry is then tested against the table with the typed
//! bound test's float steps. Nothing in the object half moves a lamp or changes its kind (its
//! callees change light lists, fences, flags and luminances only), so every geometry gets the
//! same lamps in the same order; what goes is the repeated kind call per geometry and lamp.
//!
//! **Floats.** Main thread only. The drain's own: the near test's one ordered comparison (150.0
//! above the bound's radius). The table's bound test: native `f32` in the exe's order, as the
//! typed bound test ([`walk::bound_within_radius`]).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::OnceLock;

use super::{walk, ShadowSceneLight};
use engine_core::ptr::Ptr;
use engine_core::{args, draw_thread, typed_call, Engine};

/// An `NiLight`'s world translation (+0x8c) and radius (+0xe0); the scene node's lighting offset
/// (+0x1e4); the `NiRTTI` record the bound test accepts outright (`011f4a68`).
const WORLD_TRANSLATE: u32 = 0x8c;
const NI_LIGHT_RADIUS: u32 = 0xe0;
const LIGHTING_OFFSET: u32 = 0x1e4;
const RADIUS_FREE_RTTI: u32 = 0x011f_4a68;

/// One `NiLight` as the bound test reads it.
#[derive(Clone, Copy)]
struct Sphere {
    free: bool,
    position: [f32; 3],
    radius: f32,
}

/// The lamps one drain's object half met, by `NiLight`, and the lighting offset.
struct Table {
    offset: [f32; 3],
    known: HashMap<u32, Sphere>,
}

impl Table {
    fn new(e: &Engine) -> Table {
        let node = e.global::<u32>(SCENE_NODE);
        Table {
            offset: [0u32, 4, 8].map(|i| e.mem.f32(node.wrapping_add(LIGHTING_OFFSET + i))),
            known: HashMap::new(),
        }
    }

    /// The bound test of `00b9dae0` with the lamp's own `NiLight` and the scale 1.0, from the
    /// table: the steps of [`walk::bound_within_radius`].
    fn within(&mut self, e: &mut Engine, ni_light: u32, bound: u32) -> bool {
        let sphere = match self.known.get(&ni_light) {
            Some(s) => *s,
            None => {
                let s = Sphere {
                    free: e.vcall(ni_light, SLOT_RTTI, &args![]).u32() == RADIUS_FREE_RTTI,
                    position: [0u32, 4, 8]
                        .map(|i| e.mem.f32(ni_light.wrapping_add(WORLD_TRANSLATE + i))),
                    radius: e.mem.f32(ni_light.wrapping_add(NI_LIGHT_RADIUS)),
                };
                self.known.insert(ni_light, s);
                s
            }
        };
        if sphere.free {
            return true;
        }
        let b: [f32; 4] = [0u32, 4, 8, 0xc].map(|i| e.mem.f32(bound.wrapping_add(i)));
        let moved: [f32; 3] = [0usize, 1, 2].map(|i| self.offset[i] + (sphere.position[i] - b[i]));
        let squared = (moved[1] * moved[1] + moved[0] * moved[0]) + moved[2] * moved[2];
        let gap = squared.sqrt() - b[3] * 1.0;
        sphere.radius > gap
    }
}

thread_local! {
    /// The running drain's table (only while a drain's object half runs, with the switch on).
    static TABLE: RefCell<Option<Table>> = const { RefCell::new(None) };
    /// The switch as a test sets it, over `NV_LIGHT_TABLE`.
    static FORCED: Cell<Option<bool>> = const { Cell::new(None) };
}

/// `NV_LIGHT_TABLE` (`1` or `on`): the drain's light table. Off by default.
fn table_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    FORCED.get().unwrap_or_else(|| {
        *ON.get_or_init(|| {
            matches!(
                std::env::var("NV_LIGHT_TABLE").as_deref(),
                Ok("1") | Ok("on")
            )
        })
    })
}

/// For tests: the light table on or off on this thread whatever `NV_LIGHT_TABLE` says (`None`:
/// the switch again).
pub fn force_light_table(on: Option<bool>) {
    FORCED.set(on);
}

/// The locks the drain holds (`0040fbf0` with 0 on each) and their entry counts.
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LIGHT_LOCK: u32 = 0x011f_9ea0;
const LIGHT_LOCK_COUNT: u32 = 0x011f_9ea4;
const OBJECT_LOCK: u32 = 0x011f_9ec0;
const OBJECT_LOCK_COUNT: u32 = 0x011f_9ec4;
/// The light half's callees (`thiscall`, one stack word), by address: a new shadow caster, a new
/// light, a dynamic light's update, a light's update, a removal; the renderer (`011f91a8`) and its
/// map release; the portal graph's update; the list's remove-all; the reference counts.
const LIGHT_UPDATE_SHADOW: u32 = 0x00b5_cd00;
const LIGHT_ADD: u32 = 0x00b5_eca0;
const LIGHT_UPDATE_DYNAMIC: u32 = 0x00b5_d930;
const LIGHT_UPDATE: u32 = 0x00b5_d300;
const REMOVE_QUEUED_LIGHT: u32 = 0x00b5_ef90;
const RENDERER: u32 = 0x011f_91a8;
const RELEASE_MAP: u32 = 0x00b6_d4c0;
const PORTAL_GRAPH_UPDATE: u32 = 0x00c5_b8d0;
const LIST_REMOVE_ALL: u32 = 0x00e7_4d40;
const INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// The node's queues: lights to add (+0xe4), to remove (+0xf0), to update (+0xfc), objects
/// (+0x108) and objects for active lights only (+0x114); the portal graph (+0x1e0), the drained
/// flag (+0x1fc); its light list (`lLightList` +0xb4, count +0xbc).
const QUEUE_ADD: u32 = 0xe4;
const QUEUE_REMOVE: u32 = 0xf0;
const QUEUE_UPDATE: u32 = 0xfc;
const QUEUE_OBJECTS: u32 = 0x108;
const QUEUE_OBJECTS_ACTIVE: u32 = 0x114;
const PORTAL_GRAPH: u32 = 0x1e0;
const DRAINED: u32 = 0x1fc;
const LIGHT_LIST: u32 = 0xb4;
const LIGHT_COUNT: u32 = 0xbc;
/// A light's `bCastShadow` (+0xec), `bDynamicLight` (+0xfc), shadow map (+0x10c), `spLight`
/// (+0xf8) and view-cull word (+0x110, 0xff culled).
const LIGHT_CAST_SHADOW: u32 = 0xec;
const LIGHT_DYNAMIC: u32 = 0xfc;
const LIGHT_SHADOW_MAP: u32 = 0x10c;
const LIGHT_SP_LIGHT: u32 = 0xf8;
const LIGHT_FRUSTUM_CULL: u32 = 0x110;
/// The near test's 150.0 (`010231d0`) and the null bound (`011f4288`).
const NEAR_LIMIT: u32 = 0x0102_31d0;
const NULL_BOUND: u32 = 0x011f_4288;
/// `NiAVObject`: the parent (+0x18), the world bound (+0x20), the flags (+0x30); a node's
/// children (+0xa0, count +0xa6); a geometry's property word (+0xa8, read through +0x9c) and the
/// property's type (+0x1c).
const PARENT: u32 = 0x18;
const WORLD_BOUND: u32 = 0x20;
const FLAGS: u32 = 0x30;
const CHILDREN_BASE: u32 = 0xa0;
const CHILDREN_SIZE: u32 = 0xa6;
const PROPERTY_BASE: u32 = 0x9c;
const PROPERTY: u32 = 0xa8;
const PROPERTY_TYPE: u32 = 0x1c;
/// Virtual slots: the RTTI (+8), `IsNode` (+0xc), `IsTriBasedGeom` (+0x1c).
const SLOT_RTTI: u32 = 8;
const SLOT_IS_NODE: u32 = 0xc;
const SLOT_IS_TRI_BASED_GEOM: u32 = 0x1c;
/// The scene node pointer (`011f91c8`).
const SCENE_NODE: u32 = 0x011f_91c8;
/// The parents whose exact RTTI gives the candidates (`01202828`: lights at +0xe0, count +0xe4;
/// `012030b8`: lights at +0xb4, count +0xb8).
const ROOM_RTTI: u32 = 0x0120_2828;
const OTHER_RTTI: u32 = 0x0120_30b8;
/// The property's fence and fenced removal, the shader manager's accumulator (its scene node at
/// +0x194), the multibound cast (to the record at `011f5eb4`) and its indexed child.
const FENCE_ALL_LIGHTS: u32 = 0x00b7_18b0;
const REMOVE_FENCED_LIGHTS: u32 = 0x00b7_1600;
const GET_ACCUMULATOR: u32 = 0x00b4_f5c0;
const ACCUMULATOR_SCENE_NODE: u32 = 0x194;
const CAST: u32 = 0x0065_3290;
const MULTIBOUND_RTTI: u32 = 0x011f_5eb4;
const INDEXED_CHILD: u32 = 0x00b5_aa40;
/// The scrap heap (on the memory manager `011f6238`) and its alignment word, the CRT `qsort`, the
/// exe's comparator of luminances and `GetLuminanceToBound`.
const MEMORY_MANAGER: u32 = 0x011f_6238;
const GET_THREAD_SCRAP_HEAP: u32 = 0x00aa_42e0;
const SCRAP_ALLOCATE: u32 = 0x00aa_54a0;
const SCRAP_DEALLOCATE: u32 = 0x00aa_5610;
const SCRAP_ALIGNMENT: u32 = 0x010a_2720;
const QSORT: u32 = 0x00ec_6f20;
const LUMINANCE_COMPARE: u32 = 0x00b5_aa70;
const LUMINANCE_TO_BOUND: u32 = 0x00b9_dbe0;
/// The flag a geometry gets once its lights were redone.
const LIGHTING_DONE: u32 = 0x40_0000;

/// The candidates of a geometry update: a room's lights, the other parent's, or (both 0) the
/// node's whole list.
#[derive(Clone, Copy)]
struct Candidates {
    rooms: u32,
    others: u32,
}

/// An object's bound (its own, or the null bound) copied into the stack local at `local`.
fn copy_bound(e: &mut Engine, object: u32, local: u32) {
    let source = match e.mem.u32(object.wrapping_add(WORLD_BOUND)) {
        0 => NULL_BOUND,
        bound => bound,
    };
    for word in 0..4u32 {
        let value = e.mem.u32(source.wrapping_add(4 * word));
        e.mem.set_u32(local + 4 * word, value);
    }
}

/// The property word when it is a lighting property: its type not -1, and 1 to 12.
fn lighting_property(e: &Engine, word: u32) -> u32 {
    let kind = e.mem.u32(word.wrapping_add(PROPERTY_TYPE));
    if kind.wrapping_add(1) == 0 || kind.wrapping_sub(1) > 0xb {
        0
    } else {
        word
    }
}

/// The lights of an array of `holder` (the array at `array_at`, its count at `count_at`, both read
/// at each step as the exe does).
fn each_array_light(
    e: &mut Engine,
    holder: u32,
    array_at: u32,
    count_at: u32,
    visit: &mut dyn FnMut(&mut Engine, u32),
) {
    let mut index = 0u32;
    while index < e.mem.u32(holder.wrapping_add(count_at)) {
        let array = e.mem.u32(holder.wrapping_add(array_at));
        let light = e.mem.u32(array.wrapping_add(4 * index));
        visit(e, light);
        index += 1;
    }
}

/// The lights of the node's list, each item's next taken before its light is visited.
fn each_list_light(e: &mut Engine, node: u32, visit: &mut dyn FnMut(&mut Engine, u32)) {
    let mut item = e.mem.u32(node.wrapping_add(LIGHT_LIST));
    while item != 0 {
        let light = e.mem.u32(item.wrapping_add(8));
        item = e.mem.u32(item);
        visit(e, light);
    }
}

/// The candidates of `from` (or of `node`'s list), in the exe's order.
fn each_candidate(
    e: &mut Engine,
    node: u32,
    from: Candidates,
    visit: &mut dyn FnMut(&mut Engine, u32),
) {
    if from.rooms != 0 {
        each_array_light(e, from.rooms, 0xe0, 0xe4, visit);
    } else if from.others != 0 {
        each_array_light(e, from.others, 0xb4, 0xb8, visit);
    } else {
        each_list_light(e, node, visit);
    }
}

/// `BoundWithinLightRadius` (typed) with the light's own `NiLight` and the scale 1.0.
fn within(e: &mut Engine, light: u32, bound: u32) -> bool {
    let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
    if let Some(mut table) = TABLE.take() {
        let answer = table.within(e, ni_light, bound);
        TABLE.set(Some(table));
        return answer;
    }
    let answer: u8 = typed_call!(
        super::FAMILY,
        0x00b9_dae0,
        walk::bound_within_radius(
            e,
            Ptr::<ShadowSceneLight>::new(light),
            Ptr::new(bound),
            ni_light,
            1.0
        )
    );
    answer != 0
}

/// `AddTriGeom` (typed): the light gets the geometry.
fn add(e: &mut Engine, light: u32, geometry: u32) {
    typed_call!(
        super::FAMILY,
        0x00b9_f480,
        walk::add_tri_geom(e, Ptr::<ShadowSceneLight>::new(light), geometry)
    );
}

/// Flag 0x400000 set on the geometry.
fn mark_done(e: &mut Engine, geometry: u32) {
    let flags = e.mem.u32(geometry.wrapping_add(FLAGS));
    e.mem
        .set_u32(geometry.wrapping_add(FLAGS), flags | LIGHTING_DONE);
}

// Typed from 00b5b260
/// `UpdateQueuedObjectLighting` (`cdecl(geometry, node, unused, rooms, others)`): 1 for no
/// property word, 0 for one that is not a lighting property. Otherwise (a null node: the shader
/// manager accumulator's, else the one at `011f91c8`) the property's lights are fenced, each
/// candidate light the geometry's bound is within gets the geometry, the fenced lights are removed,
/// flag 0x400000 is set and 1 answered.
pub fn update_geometry(
    e: &mut Engine,
    geometry: u32,
    node: u32,
    _unused: u32,
    rooms: u32,
    others: u32,
) -> u8 {
    let word = e.mem.u32(geometry.wrapping_add(PROPERTY));
    if word == 0 {
        return 1;
    }
    let property = lighting_property(e, word);
    if property == 0 {
        return 0;
    }
    let mut node = node;
    if node == 0 {
        let accumulator = e.call(GET_ACCUMULATOR, &args![]).u32();
        node = e.mem.u32(accumulator.wrapping_add(ACCUMULATOR_SCENE_NODE));
        if node == 0 {
            node = e.global::<u32>(SCENE_NODE);
        }
    }
    e.call(FENCE_ALL_LIGHTS, &args![property]);
    e.with_stack(0x10, |e, local| {
        let bound = local.addr();
        copy_bound(e, geometry, bound);
        each_candidate(e, node, Candidates { rooms, others }, &mut |e, light| {
            if light != 0 && within(e, light, bound) {
                add(e, light, geometry);
            }
        });
    });
    e.call(REMOVE_FENCED_LIGHTS, &args![property, geometry]);
    mark_done(e, geometry);
    1
}

// Typed from 00b5b410
/// `UpdateQueuedObjectLightingList` (`cdecl(geometry, unused, lights, unused)`): as
/// [`update_geometry`] with every light of the zero-ended array added, untested.
pub fn update_geometry_list(
    e: &mut Engine,
    geometry: u32,
    _unused: u32,
    lights: u32,
    _unused_3: u32,
) -> u8 {
    let base = geometry.wrapping_add(PROPERTY_BASE);
    if base == 0 {
        return 1;
    }
    let word = e.mem.u32(base.wrapping_add(0xc));
    if word == 0 {
        return 1;
    }
    let property = lighting_property(e, word);
    if property == 0 {
        return 0;
    }
    e.call(FENCE_ALL_LIGHTS, &args![property]);
    let mut index = 0u32;
    loop {
        let light = e.mem.u32(lights.wrapping_add(4 * index));
        if light == 0 {
            break;
        }
        add(e, light, geometry);
        index += 1;
    }
    e.call(REMOVE_FENCED_LIGHTS, &args![property, geometry]);
    mark_done(e, geometry);
    1
}

/// The tests both object walks start with: 1 for no object, 0 without a bound, for a zero radius
/// (a NaN passes), a culled object (flag 1) or one that is not a node; otherwise the node
/// (`IsNode` asked again, as the exe does).
fn walk_start(e: &mut Engine, object: u32) -> Result<u32, u8> {
    if object == 0 {
        return Err(1);
    }
    let bound = e.mem.u32(object.wrapping_add(WORLD_BOUND));
    if bound == 0 || e.mem.f32(bound.wrapping_add(0xc)) == 0.0 {
        return Err(0);
    }
    if e.mem.u32(object.wrapping_add(FLAGS)) & 1 != 0 {
        return Err(0);
    }
    if e.vcall(object, SLOT_IS_NODE, &args![]).u32() == 0 {
        return Err(0);
    }
    Ok(e.vcall(object, SLOT_IS_NODE, &args![]).u32())
}

/// The multibound step: the node cast to the record at `011f5eb4` and its indexed child; `None`
/// when the cast fails.
fn multibound_child(e: &mut Engine, node: u32) -> Option<u32> {
    if node == 0 {
        return None;
    }
    let cast = e.call(CAST, &args![node, MULTIBOUND_RTTI]).u32();
    if cast == 0 {
        return None;
    }
    Some(e.call(INDEXED_CHILD, &args![cast]).u32())
}

/// Both object walks: `leaf` for each tri-based geometry child, the walk again for any other child,
/// the answers and-ed; a multibound node passes on to its indexed child (1 when it has none).
fn walk_tree(e: &mut Engine, object: u32, leaf: &mut dyn FnMut(&mut Engine, u32) -> u8) -> u8 {
    let node = match walk_start(e, object) {
        Ok(node) => node,
        Err(answer) => return answer,
    };
    if let Some(child) = multibound_child(e, node) {
        if child == 0 {
            return 1;
        }
        return walk_tree(e, child, leaf) & 1;
    }
    let mut answer = 1u8;
    let mut index = 0u32;
    while index < u32::from(e.mem.u16(node.wrapping_add(CHILDREN_SIZE))) {
        let children = e.mem.u32(node.wrapping_add(CHILDREN_BASE));
        let child = e.mem.u32(children.wrapping_add(4 * index));
        if child != 0 {
            answer &= if e.vcall(child, SLOT_IS_TRI_BASED_GEOM, &args![]).u32() != 0 {
                leaf(e, child)
            } else {
                walk_tree(e, child, leaf)
            };
        }
        index += 1;
    }
    answer
}

// Typed from 00b5c490
/// `UpdateQueuedObjectLighting` overload 2 (`cdecl(object, node, unused, rooms, others)`): the
/// object walk with [`update_geometry`] for the geometries.
pub fn update_tree(
    e: &mut Engine,
    object: u32,
    node: u32,
    unused: u32,
    rooms: u32,
    others: u32,
) -> u8 {
    walk_tree(e, object, &mut |e, geometry| {
        update_geometry(e, geometry, node, unused, rooms, others)
    })
}

// Typed from 00b5c5c0
/// `UpdateQueuedObjectLightingList` overload 2 (`cdecl(object, unused, lights, unused)`): the
/// object walk with [`update_geometry_list`] for the geometries.
pub fn update_tree_list(e: &mut Engine, object: u32, first: u32, lights: u32, third: u32) -> u8 {
    walk_tree(e, object, &mut |e, geometry| {
        update_geometry_list(e, geometry, first, lights, third)
    })
}

// Typed from 00b5dac0
/// `UpdateQueuedObjectLightingStart` (`cdecl(object, node, skip_culled, collect)`): the object's
/// parent of exact RTTI `01202828` or `012030b8` gives the candidates. Without the collect flag a
/// geometry goes to [`update_geometry`], anything else to [`update_tree`]. With it, the candidate
/// lights the object's bound is within (and, with the skip flag, not view-culled) are gathered in a
/// zero-ended scrap-heap array with `GetLuminanceToBound` on each, sorted by the CRT's `qsort` with
/// `00b5aa70` when there were candidates and any was gathered, and handed to
/// [`update_tree_list`]. Answers 1.
pub fn update_start(e: &mut Engine, object: u32, node: u32, skip_culled: u32, collect: u32) -> u8 {
    if object == 0 {
        return 1;
    }
    let parent = e.mem.u32(object.wrapping_add(PARENT));
    let mut from = Candidates {
        rooms: 0,
        others: 0,
    };
    if parent != 0 {
        if e.vcall(parent, SLOT_RTTI, &args![]).u32() == ROOM_RTTI {
            from.rooms = parent;
        } else if e.vcall(parent, SLOT_RTTI, &args![]).u32() == OTHER_RTTI {
            from.others = parent;
        }
    }
    if collect & 0xff == 0 {
        if e.vcall(object, SLOT_IS_TRI_BASED_GEOM, &args![]).u32() != 0 {
            update_geometry(e, object, node, skip_culled, from.rooms, from.others);
        } else {
            update_tree(e, object, node, skip_culled, from.rooms, from.others);
        }
        return 1;
    }
    let heap = e.call(GET_THREAD_SCRAP_HEAP, &args![MEMORY_MANAGER]).u32();
    e.with_stack(0x10, |e, local| {
        let bound = local.addr();
        copy_bound(e, object, bound);
        let alignment = e.global::<u32>(SCRAP_ALIGNMENT);
        let (holder, count_at) = if from.rooms != 0 {
            (from.rooms, 0xe4)
        } else if from.others != 0 {
            (from.others, 0xb8)
        } else {
            (node, LIGHT_COUNT)
        };
        let total = e.mem.u32(holder.wrapping_add(count_at));
        let array = e
            .call(
                SCRAP_ALLOCATE,
                &args![heap, total.wrapping_mul(4).wrapping_add(4), alignment],
            )
            .u32();
        // The sort runs only when the candidates were not empty to begin with.
        let any = if from.rooms != 0 || from.others != 0 {
            e.mem.u32(holder.wrapping_add(count_at)) != 0
        } else {
            e.mem.u32(node.wrapping_add(LIGHT_LIST)) != 0
        };
        let mut count = 0u32;
        each_candidate(e, node, from, &mut |e, light| {
            if light == 0 {
                return;
            }
            if skip_culled & 0xff != 0 && e.mem.u16(light.wrapping_add(LIGHT_FRUSTUM_CULL)) == 0xff
            {
                return;
            }
            if !within(e, light, bound) {
                return;
            }
            e.mem.set_u32(array.wrapping_add(4 * count), light);
            count = count.wrapping_add(1);
            e.call(LUMINANCE_TO_BOUND, &args![light, bound]);
        });
        if any && (count as i32) > 0 {
            e.call(QSORT, &args![array, count, 4u32, LUMINANCE_COMPARE]);
        }
        e.mem.set_u32(array.wrapping_add(4 * count), 0);
        update_tree_list(e, object, node, array, skip_culled);
        e.call(SCRAP_DEALLOCATE, &args![heap, array]);
    });
    1
}

/// The near flag of an object: `IsNode` answers and 150.0 is above its bound's radius (the null
/// bound without one; not when unordered).
fn near_flag(e: &mut Engine, object: u32) -> u32 {
    if e.vcall(object, SLOT_IS_NODE, &args![]).u32() == 0 {
        return 0;
    }
    let bound = match e.mem.u32(object.wrapping_add(WORLD_BOUND)) {
        0 => NULL_BOUND,
        bound => bound,
    };
    u32::from(e.global::<f32>(NEAR_LIMIT) > e.mem.f32(bound.wrapping_add(0xc)))
}

/// An `NiPointer`'s release: the count at +4 down, the object deleted (virtual slot 4) at zero.
fn release(e: &mut Engine, object: u32) {
    if e.call(INTERLOCKED_DECREMENT, &args![object.wrapping_add(4)])
        .u32()
        == 0
    {
        e.vcall(object, 4, &args![]);
    }
}

/// An inlined lock leave: the count down, the owner word cleared at zero.
fn leave(e: &mut Engine, lock: u32, count: u32) {
    let left = e.mem.u32(count).wrapping_sub(1);
    e.mem.set_u32(count, left);
    if left == 0 {
        e.mem.set_u32(lock, 0);
    }
}

/// The lights of one of the node's queues, each item's next taken before its light is visited.
fn each_queued(e: &mut Engine, node: u32, queue: u32, visit: &mut dyn FnMut(&mut Engine, u32)) {
    let mut item = e.mem.u32(node.wrapping_add(queue));
    while item != 0 {
        let element = e.mem.u32(item.wrapping_add(8));
        item = e.mem.u32(item);
        if element != 0 {
            visit(e, element);
        }
    }
}

// Typed from 00b5fd60
/// The scene node's queue drain (`thiscall(node)`): under the light lock, each queued new light
/// goes to `00b5cd00` (shadow casters) or `00b5eca0`, each queued update to `00b5d930` (dynamic)
/// or `00b5d300`, each queued removal (a shadow caster's map given back and its pointer cleared
/// first) to `RemoveQueuedLight`, held by a local reference; under the object lock, each queued
/// object (the active-only queue first, with the skip flag) to `UpdateQueuedObjectLightingStart`
/// with the near flag. Then the portal graph's update, the five queues emptied, the drained flag
/// set, both locks left (the object lock first).
pub fn drain(e: &mut Engine, this: Ptr) {
    draw_thread::finish_at(e, 0x00b5_fd60);
    let node = this.addr();
    e.call(LOCK_ENTER, &args![LIGHT_LOCK, 0u32]);
    each_queued(e, node, QUEUE_ADD, &mut |e, light| {
        let callee = if e.mem.u8(light.wrapping_add(LIGHT_CAST_SHADOW)) != 0 {
            LIGHT_UPDATE_SHADOW
        } else {
            LIGHT_ADD
        };
        e.call(callee, &args![node, light]);
    });
    each_queued(e, node, QUEUE_UPDATE, &mut |e, light| {
        let callee = if e.mem.u8(light.wrapping_add(LIGHT_DYNAMIC)) != 0 {
            LIGHT_UPDATE_DYNAMIC
        } else {
            LIGHT_UPDATE
        };
        e.call(callee, &args![node, light]);
    });
    each_queued(e, node, QUEUE_REMOVE, &mut |e, light| {
        let shadowed = e.mem.u8(light.wrapping_add(LIGHT_CAST_SHADOW)) != 0;
        e.with_stack(4, |e, local| {
            e.mem.set_u32(local.addr(), light);
            e.call(INTERLOCKED_INCREMENT, &args![light.wrapping_add(4)]);
            if shadowed {
                let map = e.mem.u32(light.wrapping_add(LIGHT_SHADOW_MAP));
                let renderer = e.global::<u32>(RENDERER);
                e.call(RELEASE_MAP, &args![renderer, map]);
                let old = e.mem.u32(light.wrapping_add(LIGHT_SHADOW_MAP));
                if old != 0 {
                    release(e, old);
                    e.mem.set_u32(light.wrapping_add(LIGHT_SHADOW_MAP), 0);
                }
            }
            e.call(REMOVE_QUEUED_LIGHT, &args![node, local]);
            let kept = e.mem.u32(local.addr());
            if kept != 0 {
                release(e, kept);
            }
        });
    });
    e.call(LOCK_ENTER, &args![OBJECT_LOCK, 0u32]);
    // The light table for this object half (the lamps' adds, updates and removals are done).
    let table = table_on().then(|| Table::new(e));
    let outer = TABLE.replace(table);
    for (queue, skip_culled) in [(QUEUE_OBJECTS_ACTIVE, 1u32), (QUEUE_OBJECTS, 0)] {
        each_queued(e, node, queue, &mut |e, object| {
            let near = near_flag(e, object);
            let _: u8 = typed_call!(
                super::FAMILY,
                0x00b5_dac0,
                update_start(e, object, node, skip_culled, near)
            );
        });
    }
    TABLE.set(outer);
    let graph = e.mem.u32(node.wrapping_add(PORTAL_GRAPH));
    if graph != 0 {
        e.call(PORTAL_GRAPH_UPDATE, &args![graph]);
    }
    for queue in [
        QUEUE_ADD,
        QUEUE_REMOVE,
        QUEUE_UPDATE,
        QUEUE_OBJECTS,
        QUEUE_OBJECTS_ACTIVE,
    ] {
        e.call(LIST_REMOVE_ALL, &args![node.wrapping_add(queue)]);
    }
    e.mem.set_u8(node.wrapping_add(DRAINED), 1);
    leave(e, OBJECT_LOCK, OBJECT_LOCK_COUNT);
    leave(e, LIGHT_LOCK, LIGHT_LOCK_COUNT);
}
