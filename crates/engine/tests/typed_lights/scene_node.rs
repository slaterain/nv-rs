//! The shadow scene node's functions (`engine_typed::scene_node`) against their translations. One
//! world for the node functions: a scene node whose `lLightList` holds three lights and a null
//! element and whose `lShadowLightList` holds two lights, each light counted, with its `NiLight` (an
//! RTTI of its own or one of the exe's `GetRTTI` bodies), flags, cull word, shadow scene, shadow map
//! and point; its portal graph and remove-queue flag; and a tree of nodes, geometries and lights for
//! the walks. The seed word picks each choice (see [`Seed`]). The arrays, the list move, the counted
//! list's `RemoveAll` and the node's passes (sun light, occlusion planes, queued removals, light
//! value, dynamic shadows, destructors) add worlds of their own on top.
//!
//! What stands in (logging their words): the queues' lock, the counted list's add, the allocation,
//! the light's constructor, `SetLight`, shadow flag, luminance, depth map and geometry removal (their
//! own Diff tests are the lights family's), the set-up family's shadow drop, the object update, the
//! cast, the shader accumulator, the culling process's frustum, `NiLight`'s, `NiDirectionalLight`'s,
//! the culling process's and the cube camera's constructors, the light update and cull, the
//! occlusion plane's tests, the lists' `RemovePos` and `Remove`, the renderer's map calls, the
//! rendering system's call, the property's light walk, the arrays' allocator slots (their moves
//! and growth run as the exe has them), the searches' compares, the scene graph slot release, the
//! compound frustum's and `NiNode`'s destructors, the free and the deleting destructor slot.

use super::{
    block, log, logged, object, ret, ret_f, set_logged_answer, stand_ins, words, EXE_BODIES, HALF,
    LOG, MINUS_ONE, ONE, OTHER_RTTI, THIRD, TWO, VT_FN, VT_SLOTS,
};
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::scene_node::{lists, node, SCENE_NODE};

/// The records: `NiLight` (`011f4a28`), `NiPointLight` (`011f4a98`), the radius-free light
/// (`011f4a68`, `NiDirectionalLight`'s), and the exe's `GetRTTI` bodies of the point and directional
/// lights.
const LIGHT_RTTI: u32 = 0x011f_4a28;
const POINT_RTTI: u32 = 0x011f_4a98;
const RADIUS_FREE_RTTI: u32 = 0x011f_4a68;
const POINT_BODY: u32 = 0x00a7_d790;
const DIRECTIONAL_BODY: u32 = 0x00a7_5c70;
/// The light queues' lock.
const QUEUE_LOCK: u32 = 0x011f_9ea0;
/// The renderer object's word.
const RENDERER: u32 = 0x011f_91a8;
/// The search's compare stand-in.
const COMPARE: u32 = 0x00ff_3000;
/// The other families' functions these call with `typed_call!` (stand-ins here; their own tests run
/// them): the queues' lock, the counted list's add, the shadow drop, the light's constructor,
/// `SetLight`, shadow flag and `RemoveTriGeom`, the object update, the cast, the culling process's
/// frustum, the light's luminance at a point and depth map draw, and the shader accumulator.
const CROSS: [u32; 13] = [
    0x0040_fbf0,
    0x00a5_a310,
    0x00b5_c3c0,
    0x00b9_fda0,
    0x00b9_dda0,
    0x00b9_dcb0,
    0x00ba_01a0,
    0x00a5_9c60,
    0x0065_3290,
    0x00a6_94a0,
    0x00b9_bc50,
    0x00b9_f780,
    0x00b4_f5c0,
];
/// What the typed code does in place: `NiPoint3`'s `!=` and `==`, `NiPlane`'s constructor, the
/// culling process's destructor, the arrays' move (with its `memmove`) and the renderer's getter.
const IN_PLACE: [u32; 7] = [
    0x0043_9090,
    0x0043_90c0,
    0x00a6_9940,
    0x00a6_93e0,
    0x00cb_9930,
    0x00ec_7230,
    0x00b6_d120,
];

fn count_up(e: &mut Engine, a: &[u32]) -> Ret {
    let n = e.mem.u32(a[0]).wrapping_add(1);
    e.mem.set_u32(a[0], n);
    ret(n)
}

fn count_down(e: &mut Engine, a: &[u32]) -> Ret {
    let n = e.mem.u32(a[0]).wrapping_sub(1);
    e.mem.set_u32(a[0], n);
    ret(n)
}

/// Logs its words under `TAG` and answers its first (a constructor's `this`).
fn this_logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    for w in a {
        log(e, TAG, *w);
    }
    ret(a.first().copied().unwrap_or(0))
}

/// The counted list's add: logs the list and the object the pointer holds, and counts the object
/// (the list's node holds it, as the real add does).
fn add_tail(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x29, a[0]);
    let held = e.mem.u32(a[1]);
    log(e, 0x29, held);
    if held != 0 {
        let n = e.mem.u32(held + 4);
        e.mem.set_u32(held + 4, n + 1);
    }
    ret(0)
}

/// The object update: logs the object and its data (a float and five bytes).
fn update(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x31, a[0]);
    log(e, 0x31, e.mem.u32(a[1]));
    for byte in 4..9 {
        log(e, 0x31, u32::from(e.mem.u8(a[1] + byte)));
    }
    ret(0)
}

/// The cast to `NiPointLight`: the object when its word +0x60 is set, else 0.
fn cast(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x32, a[0]);
    log(e, 0x32, a[1]);
    ret(if e.mem.u32(a[0] + 0x60) != 0 { a[0] } else { 0 })
}

/// The property's light walk: the next word of the property's list at +0x80 (its place counted at
/// `LOG + 0x3d00`), 0 at its end; logs the iterator word.
fn next_light(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x38, a[0]);
    log(e, 0x38, e.mem.u32(a[1]));
    let n = e.mem.u32(LOG + 0x3d00);
    e.mem.set_u32(LOG + 0x3d00, n + 1);
    let light = if n < 4 {
        e.mem.u32(a[0] + 0x80 + 4 * n)
    } else {
        0
    };
    e.mem.set_u32(a[1], light ^ 0x5a5a);
    ret(light)
}

/// The renderer's map release: logs and, when `LOG + 0x3e00` names a word, clears it (the light's
/// map, which the typed code must read again).
fn release_map(e: &mut Engine, a: &[u32]) -> Ret {
    for w in a {
        log(e, 0x36, *w);
    }
    let at = e.mem.u32(LOG + 0x3e00);
    if at != 0 {
        e.mem.set_u32(at, 0);
    }
    ret(0)
}

/// The search's compare: 1 when the element's word is the value.
fn compare(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x3c, a[0]);
    ret(u32::from(e.mem.u32(a[0]) == a[1]))
}

/// The data and stand-ins every test here uses.
fn world(e: &mut Engine) {
    stand_ins(e);
    e.mem.set_u32(RENDERER, 0x0abc_0000);
    words(e, POINT_RTTI, &[0, LIGHT_RTTI]);
    words(e, RADIUS_FREE_RTTI, &[0, LIGHT_RTTI]);
    e.register(0x00fd_f1c0, count_up);
    e.register(0x00fd_f1c4, count_down);
    e.register(VT_FN + 4, logged::<0x3b>);
    e.register(0x0040_fbf0, logged::<0x28>);
    e.register(0x00a5_a310, add_tail);
    e.register(0x00aa_13e0, logged::<0x2a>);
    e.register(0x00b9_fda0, this_logged::<0x2b>);
    e.register(0x00b9_dda0, logged::<0x2c>);
    e.register(0x00b9_dcb0, logged::<0x2d>);
    e.register(0x00ba_01a0, logged::<0x2e>);
    e.register(0x00b5_c3c0, logged::<0x2f>);
    e.register(0x00a5_9c60, update);
    e.register(0x0065_3290, cast);
    e.register(0x00a7_d670, this_logged::<0x33>);
    e.register(0x00b5_d300, logged::<0x34>);
    e.register(0x00b9_ef30, logged::<0x35>);
    e.register(0x00b6_d4c0, release_map);
    e.register(0x00b6_d130, logged::<0x37>);
    e.register(0x00b7_0490, next_light);
    e.register(COMPARE, compare);
    // The item pool of the counted list's `RemoveAll`: its lock (held twice) and free head.
    e.map(0x011c_5000, 0x1000);
    words(e, 0x011c_5f80, &[0x0077_0000, 2]);
    e.mem.set_u32(0x011c_5f58, 0x0bad_0000);
    passes_stand_ins(e);
}

trait Node {
    fn node(self) -> Diff;
}

impl Node for Diff {
    fn node(self) -> Diff {
        // The lights family stays off: its members these reach are stand-ins on both sides.
        self.family(&SCENE_NODE)
            .family(&leaf::LEAVES)
            .goes_by_address(&CROSS)
            .ignore_calls(&EXE_BODIES)
            .ignore_calls(&IN_PLACE)
            .max_calls(20_000)
    }
}

/// A seed's choices, two bits each from a mixed word.
struct Seed(u32);

impl Seed {
    fn new(seed: u32) -> Seed {
        Seed(seed.wrapping_mul(0x9e37_79b9) ^ 0x5bd1_e995)
    }

    /// Choice `i` of `n`.
    fn pick(&self, i: u32, n: u32) -> u32 {
        let x = (self.0 ^ i.wrapping_mul(0x85eb_ca6b)).wrapping_mul(0xc2b2_ae35);
        ((x ^ (x >> 15)) >> 8) % n
    }
}

const SEEDS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
];

/// A counted object of `size` bytes with its own vtable, count `count`.
fn counted(e: &mut Engine, size: u32, count: u32) -> u32 {
    let o = object(e, size, 0, 0, OTHER_RTTI);
    e.mem.set_u32(o + 4, count);
    o
}

/// An `NiLight` of `kind`: 0 an unrelated RTTI, 1 a record whose base is `NiLight`, 2 the point
/// light's exe body, 3 the directional light's; its world translation `at`, its flags `flags`.
fn ni_light(e: &mut Engine, kind: u32, at: [u32; 3], flags: u32) -> u32 {
    let record = match kind {
        1 => block(e, 8, &[0, LIGHT_RTTI]),
        _ => OTHER_RTTI,
    };
    let l = object(e, 0x100, 0, 0, record);
    let vt = e.mem.u32(l);
    match kind {
        2 => e.mem.set_u32(vt + 8, POINT_BODY),
        3 => e.mem.set_u32(vt + 8, DIRECTIONAL_BODY),
        _ => {}
    }
    e.mem.set_u32(l + 4, 1);
    e.mem.set_u32(l + 0x30, flags);
    words(e, l + 0x8c, &at);
    l
}

/// A list (head, tail, count) of `elements` at `list`; answers the items.
fn list(e: &mut Engine, list: u32, elements: &[u32]) -> Vec<u32> {
    let items: Vec<u32> = elements.iter().map(|_| block(e, 0xc, &[])).collect();
    for (i, &item) in items.iter().enumerate() {
        let next = items.get(i + 1).copied().unwrap_or(0);
        let previous = if i == 0 { 0 } else { items[i - 1] };
        words(e, item, &[next, previous, elements[i]]);
    }
    let head = items.first().copied().unwrap_or(0);
    let tail = items.last().copied().unwrap_or(0);
    words(e, list, &[head, tail, elements.len() as u32]);
    items
}

/// The node world: what the tests' arguments come from.
struct NodeWorld {
    node: u32,
    /// The lights of `lLightList` (a null element after the first) and of `lShadowLightList`.
    lights: [u32; 3],
    shadow_lights: [u32; 2],
    /// The light without an `NiLight` at the end of `lLightList`, or 0.
    dark: u32,
    /// An `NiLight` no light has.
    stray: u32,
    /// A tree: a node with a geometry, a light, a node with a light and a geometry, and a null child.
    tree: u32,
}

/// A shadow scene light of the seed's choices `i..`: its `NiLight` (at a point whose last float is
/// -1.0, +0.0 or a NaN), flags, cull word (3, 0xff, or 0x1ff, which only a 16-bit compare tells
/// from 0xff), shadow scene, map and point (the `NiLight`'s, or one differing from it, its last
/// float's sign flipped among them: so +0/-0 and NaN/NaN pairs too).
fn shadow_light(e: &mut Engine, s: &Seed, i: u32, scene: u32) -> u32 {
    let l = counted(e, 0x250, 1 + s.pick(i, 2));
    let at = match s.pick(i + 9, 3) {
        0 => [ONE, HALF, MINUS_ONE],
        1 => [ONE, HALF, 0],
        _ => [ONE, HALF, 0x7fc0_0000],
    };
    let culled = s.pick(i + 1, 4) == 3;
    let ni = ni_light(e, s.pick(i + 2, 4), at, u32::from(culled) | 0x10);
    e.mem.set_u32(l + 0xf8, ni);
    e.mem.set_u8(l + 0xec, u8::from(s.pick(i + 3, 4) == 0));
    e.mem.set_u8(l + 0xed, 1);
    e.mem.set_u8(l + 0xf4, u8::from(s.pick(i + 4, 4) != 0));
    e.mem.set_u8(l + 0xfc, u8::from(s.pick(i + 5, 4) != 1));
    let point = match s.pick(i + 6, 5) {
        0 => at,
        1 => [ONE, HALF, 0x7fc0_0000],
        2 => [ONE, TWO, MINUS_ONE],
        3 => [0x8000_0000 | ONE, HALF, MINUS_ONE],
        _ => [at[0], at[1], at[2] ^ 0x8000_0000],
    };
    words(e, l + 0x100, &point);
    let cull = match s.pick(i + 10, 4) {
        0 | 1 => 3,
        2 => 0xff,
        _ => 0x1ff,
    };
    e.mem.set_u16(l + 0x110, cull);
    if s.pick(i + 7, 3) != 0 {
        let map = counted(e, 0x40, 1 + s.pick(i + 7, 3) / 2);
        e.mem.set_u32(l + 0x10c, map);
    }
    e.mem.set_u32(l + 0x128, scene);
    e.mem.set_u32(l + 0x240, 0x0ddd_0000);
    l
}

/// A node object of `children` (their `IsNode` answering the node, through an exe body or a
/// stand-in).
fn tree_node(e: &mut Engine, children: &[u32], exe: bool) -> u32 {
    let n = object(e, 0x100, 0, 0, OTHER_RTTI);
    let vt = e.mem.u32(n);
    e.mem.set_u32(vt + VT_SLOTS, n);
    if exe {
        e.mem.set_u32(vt + 0xc, EXE_BODIES[0]);
        e.mem.set_u32(vt + 0x1c, EXE_BODIES[2]);
    }
    let base = block(e, 4 * children.len() as u32 + 4, children);
    e.mem.set_u32(n + 0xa0, base);
    e.mem.set_u16(n + 0xa6, children.len() as u16);
    n
}

/// A geometry with a lighting property of `kind` listing `lights`.
/// (The word through +0x9c's +0xc that the exe reads first is +0xa8 itself.)
fn geometry(e: &mut Engine, kind: u32, lights: &[u32], exe: bool) -> u32 {
    let g = object(e, 0x100, 0, 0, OTHER_RTTI);
    let vt = e.mem.u32(g);
    e.mem.set_u32(vt + VT_SLOTS + 4, g);
    if exe {
        e.mem.set_u32(vt + 0x1c, EXE_BODIES[1]);
        e.mem.set_u32(vt + 0xc, EXE_BODIES[2]);
    }
    let property = block(e, 0x100, &[]);
    e.mem.set_u32(property + 0x1c, kind);
    words(e, property + 0x80, lights);
    e.mem.set_u32(g + 0xa8, property);
    g
}

fn node_world(e: &mut Engine, seed: u32) -> NodeWorld {
    world(e);
    let s = Seed::new(seed);
    let node = block(e, 0x200, &[]);
    let scene = counted(e, 0x40, 2);
    let lights = [0u32, 4, 8].map(|i| shadow_light(e, &s, i, scene));
    let shadow_lights = [9u32, 11].map(|i| shadow_light(e, &s, i, if i == 9 { 0 } else { scene }));
    // For some seeds a light without an `NiLight` ends the list (what `GetLight` of no `NiLight`
    // finds).
    let dark = if s.pick(18, 3) == 0 {
        let l = shadow_light(e, &s, 16, scene);
        e.mem.set_u32(l + 0xf8, 0);
        l
    } else {
        0
    };
    let mut listed = vec![lights[0], 0, lights[1], lights[2]];
    if dark != 0 {
        listed.push(dark);
    }
    list(e, node + 0xb4, &listed);
    list(e, node + 0xc0, &shadow_lights);
    words(e, node + 0xd0, &[0x0f0f_0000, 0x0e0e_0000]);
    e.mem.set_u32(node + 0x1e0, 0x0ddd_0000);
    e.mem.set_u8(node + 0x1fc, u8::from(s.pick(13, 4) != 0));
    let stray = ni_light(e, 1, [0; 3], 0);
    // The queues' lock: free or held (its count).
    words(e, QUEUE_LOCK, &[0, s.pick(14, 3)]);
    // The tree.
    let exe = s.pick(15, 2) == 0;
    let lit = [lights[0], lights[2], 0, 0];
    let g1 = geometry(e, 1 + s.pick(12, 4) * 4, &lit, exe);
    let g2 = geometry(e, 12, &lit[1..], exe);
    let point = ni_light(e, 2, [ONE; 3], 0);
    e.mem.set_u32(point + 0x60, 1);
    words(e, point + 0xa8, &[7]);
    let directional = ni_light(e, 3, [ONE; 3], 0);
    let inner = tree_node(e, &[directional, g2, 0], exe);
    let tree = tree_node(e, &[g1, point, inner, 0, stray], exe);
    // The allocations' block (the exe writes a new light's fields at once: a failed allocation
    // would fault in the exe).
    let fresh = block(e, 0x250, &[]);
    set_logged_answer(e, 0x2a, fresh);
    NodeWorld {
        node,
        lights,
        shadow_lights,
        dark,
        stray,
        tree,
    }
}

/// The world's light at `k`: the list lights (0 to 2), the shadow lights (3, 4).
fn light_at(w: &NodeWorld, k: u32) -> u32 {
    match k {
        0..=2 => w.lights[k as usize],
        _ => w.shadow_lights[(k - 3) as usize],
    }
}

/// The `NiLight` asked for by `k`: a list light's (0 to 2), a shadow light's (3), one no light
/// has (4), none (5).
fn ni_light_at(e: &Engine, w: &NodeWorld, k: u32) -> u32 {
    match k {
        0..=2 => e.mem.u32(w.lights[k as usize] + 0xf8),
        3 => e.mem.u32(w.shadow_lights[0] + 0xf8),
        4 => w.stray,
        _ => 0,
    }
}

fn seeds_and(rest: &[Input]) -> Vec<Input> {
    let mut v = vec![Input::Of(SEEDS)];
    v.extend_from_slice(rest);
    v
}

#[test]
fn scene_node_get_light() {
    Diff::new(entry!(0x00b5_b4a0, lists::get_light(Ptr<_>, u32) -> u32))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 2, 3, 4, 5])]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            vec![w.node, ni_light_at(e, &w, case.word(1))]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_set_light_cast_shadow() {
    Diff::new(entry!(
        0x00b5_c450,
        lists::set_light_cast_shadow(Ptr<_>, u32, u32)
    ))
    .node()
    .inputs(&seeds_and(&[
        Input::Of(&[0, 2, 3, 4]),
        // The flag's byte (by address only the byte goes on: the typed `SetCastShadow` takes it).
        Input::Of(&[0, 1, 0xff]),
    ]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        vec![w.node, ni_light_at(e, &w, case.word(1)), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_count_frustum_lights() {
    Diff::new(entry!(
        0x00b5_ac60,
        lists::count_frustum_lights(Ptr<_>) -> u32
    ))
    .node()
    .inputs(&seeds_and(&[]))
    .setup(|e, case| vec![node_world(e, case.word(0)).node])
    .run()
    .assert_same();
}

#[test]
fn scene_node_queue_update() {
    Diff::new(entry!(0x00b5_d270, lists::queue_update(Ptr<_>, u32)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 5])]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            let light = if case.word(1) == 5 {
                0
            } else {
                light_at(&w, case.word(1))
            };
            vec![w.node, light]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_queue_update_of() {
    Diff::new(entry!(0x00b5_f080, lists::queue_update_of(Ptr<_>, u32)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 2, 3, 4])]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            vec![w.node, ni_light_at(e, &w, case.word(1))]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_queue_removal() {
    Diff::new(entry!(0x00b5_cff0, lists::queue_removal(Ptr<_>, Ptr<_>)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 5])]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            let light = if case.word(1) == 5 {
                0
            } else {
                light_at(&w, case.word(1))
            };
            let pointer = block(e, 4, &[light]);
            vec![w.node, pointer]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_queue_removal_of_scene() {
    Diff::new(entry!(
        0x00b5_edf0,
        lists::queue_removal_of_scene(Ptr<_>, u32)
    ))
    .node()
    .inputs(&seeds_and(&[Input::Of(&[0, 1, 2])]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        let object = match case.word(1) {
            0 => 0,
            1 => e.mem.u32(w.shadow_lights[1] + 0x128),
            _ => w.stray,
        };
        vec![w.node, object]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_remove_light() {
    Diff::new(entry!(0x00b5_eed0, lists::remove_light(Ptr<_>, u32)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 2, 3, 4, 5])]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            vec![w.node, ni_light_at(e, &w, case.word(1))]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_queue_all_removals() {
    Diff::new(entry!(0x00b5_d180, lists::queue_all_removals(Ptr<_>)))
        .node()
        .inputs(&seeds_and(&[]))
        .setup(|e, case| vec![node_world(e, case.word(0)).node])
        .run()
        .assert_same();
}

#[test]
fn scene_node_queue_all_updates() {
    Diff::new(entry!(
        0x00b5_ddb0,
        lists::queue_all_updates(Ptr<_>, u8, u8)
    ))
    .node()
    .inputs(&seeds_and(&[
        Input::Of(&[0, 1, 0x100]),
        Input::Of(&[0, 1, 0x100]),
    ]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        vec![w.node, case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_add_light() {
    // A light of each kind (an unrelated RTTI, a record based on `NiLight`, the point and the
    // directional bodies, a stand-in answering the point record), none; the dynamic byte.
    Diff::new(entry!(
        0x00b5_c940,
        lists::add_light(Ptr<_>, u32, u8) -> u32
    ))
    .node()
    .inputs(&seeds_and(&[
        Input::Of(&[0, 1, 2, 3, 4, 5]),
        Input::Of(&[0, 1, 0x100]),
    ]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        let light = match case.word(1) {
            5 => 0,
            4 => object(e, 0x100, 0, 0, POINT_RTTI),
            kind => ni_light(e, kind, [0; 3], 0),
        };
        vec![w.node, light, case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_add_all_lights_from_node() {
    Diff::new(entry!(
        0x00b5_ca70,
        lists::add_all_lights_from_node(Ptr<_>, u32, f32) -> u32
    ))
    .node()
    .inputs(&seeds_and(&[Input::F32]))
    .float_sweep(16)
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        vec![w.node, w.tree, case.f32(1).to_bits()]
    })
    .run()
    .assert_same();
}

/// The tree or none, the remove word (0, 1, a word whose low byte is 0).
fn cull_inputs() -> Vec<Input> {
    seeds_and(&[Input::Of(&[0, 1]), Input::Of(&[0, 1, 0x100])])
}

fn cull_setup(e: &mut Engine, case: &Case) -> Vec<u32> {
    let w = node_world(e, case.word(0));
    let parent = if case.word(1) == 0 { 0 } else { w.tree };
    vec![w.node, parent, case.word(2)]
}

#[test]
fn scene_node_find_and_cull_light() {
    Diff::new(entry!(
        0x00b5_f150,
        lists::find_and_cull_light(Ptr<_>, u32, u32)
    ))
    .node()
    .inputs(&cull_inputs())
    .setup(cull_setup)
    .run()
    .assert_same();
}

#[test]
fn scene_node_find_and_cull_light_recurse() {
    Diff::new(entry!(
        0x00b5_f0c0,
        lists::find_and_cull_light_recurse(Ptr<_>, u32, u32)
    ))
    .node()
    .inputs(&seeds_and(&[Input::Of(&[1]), Input::Of(&[0, 1, 0x100])]))
    .setup(cull_setup)
    .run()
    .assert_same();
}

/// The H03 seam's handler of the test: JIP's own size.
fn at_h03(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0x110)
}

#[test]
fn scene_node_make_dynamic() {
    use engine_core::extend::jip_patches;
    // JIP's patches off, the fixes row only, installed, both; the light made dynamic. (A failed
    // allocation would have the exe write the null light's flags: not run.)
    Diff::new(entry!(0x00b5_cd00, lists::make_dynamic(Ptr<_>, u32) -> u32))
        .node()
        .inputs(&seeds_and(&[
            Input::Of(&[
                0,
                jip_patches::FIXES,
                jip_patches::INSTALLED,
                jip_patches::INSTALLED | jip_patches::FIXES,
            ]),
            Input::Of(&[0, 1, 2]),
        ]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            // A fresh `NiLight`'s memory, its flags all set.
            let fresh = block(e, 0x110, &[]);
            e.mem.set_u32(fresh + 0x30, 0xff);
            set_logged_answer(e, 0x2a, fresh);
            e.on_site_for_test(0x00b5_cd2a, at_h03);
            e.set_jip_patches(case.word(1));
            vec![w.node, w.lights[(case.word(2) % 3) as usize]]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_follow_moved_light() {
    Diff::new(entry!(0x00b5_d930, lists::follow_moved_light(Ptr<_>, u32)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 2, 3, 4])]))
        .max_cases(400)
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            vec![w.node, light_at(&w, case.word(1))]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_drop_all_shadow_maps() {
    Diff::new(entry!(0x00b5_b1f0, lists::drop_all_shadow_maps(Ptr<_>)))
        .node()
        .inputs(&seeds_and(&[Input::Flag]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            // The release clears the second shadow light's map: read again, nothing let go.
            if case.word(1) != 0 {
                e.mem.set_u32(LOG + 0x3e00, w.shadow_lights[1] + 0x10c);
            }
            vec![w.node]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_remove_object() {
    // The tree, a node, a geometry, a light (not a node), none.
    Diff::new(entry!(0x00b5_b1c0, lists::remove_object(Ptr<_>, u32)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 2, 3])]))
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            let base = e.mem.u32(w.tree + 0xa0);
            let object = match case.word(1) {
                0 => w.tree,
                1 => e.mem.u32(base),
                2 => w.stray,
                _ => 0,
            };
            vec![w.node, object]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_remove_object_recurse() {
    Diff::new(entry!(
        0x00b5_b150,
        lists::remove_object_recurse(Ptr<_>, u32)
    ))
    .node()
    .inputs(&seeds_and(&[]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        vec![w.node, w.tree]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_remove_geometry_from_property_lights() {
    // A geometry with its property through +0x9c or at +0xa8 (types 1, 5, 9, 13 by the seed: 13
    // is not a lighting property), one without a property (the word through +0x9c and +0xa8 both
    // 0), none.
    Diff::new(entry!(
        0x00b5_b0e0,
        lists::remove_geometry_from_property_lights(u32)
    ))
    .node()
    .inputs(&seeds_and(&[Input::Of(&[0, 1, 2])]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        let base = e.mem.u32(w.tree + 0xa0);
        let g = e.mem.u32(base);
        match case.word(1) {
            0 => vec![g],
            1 => {
                e.mem.set_u32(g + 0xa8, 0);
                e.mem.set_u32(g + 0x9c + 0xc, 0);
                vec![g]
            }
            _ => vec![0],
        }
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_add_unique() {
    // The graph's list holding the object or not (or empty); the object or none; its count 1 or 2.
    Diff::new(entry!(0x00b5_c6e0, lists::add_unique(Ptr<_>, u32)))
        .node()
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1]),
            Input::Of(&[1, 2]),
        ])
        .setup(|e, case| {
            world(e);
            let graph = block(e, 0x40, &[]);
            let object = if case.word(1) == 0 {
                0
            } else {
                counted(e, 0x40, case.word(2))
            };
            let other = counted(e, 0x40, 1);
            match case.word(0) {
                0 => {}
                1 => {
                    list(e, graph + 0x30, &[other, object]);
                }
                _ => {
                    list(e, graph + 0x30, &[other, 0]);
                }
            }
            vec![graph, object]
        })
        .run()
        .assert_same();
}

/// The old pointer: none, the same, another let go (deleted), another kept; the new: none, one.
fn setter_inputs() -> Vec<Input> {
    vec![Input::Of(&[0, 1, 2, 3]), Input::Flag]
}

/// A light whose pointer at `at` holds the case's old object; the words: the light, the new one.
fn setter_setup(e: &mut Engine, case: &Case, at: u32) -> Vec<u32> {
    world(e);
    let l = block(e, 0x250, &[]);
    let new = if case.word(1) != 0 {
        counted(e, 0x40, 1)
    } else {
        0
    };
    let old = match case.word(0) {
        0 => 0,
        1 => new,
        2 => counted(e, 0x40, 1),
        _ => counted(e, 0x40, 2),
    };
    e.mem.set_u32(l + at, old);
    vec![l, new]
}

#[test]
fn scene_node_set_shadow_map() {
    Diff::new(entry!(0x00b5_aee0, lists::set_shadow_map(Ptr<_>, u32)))
        .node()
        .inputs(&setter_inputs())
        .setup(|e, case| setter_setup(e, case, 0x10c))
        .run()
        .assert_same();
}

#[test]
fn scene_node_set_shadow_scene() {
    Diff::new(entry!(0x00b5_af30, lists::set_shadow_scene(Ptr<_>, u32)))
        .node()
        .inputs(&setter_inputs())
        .setup(|e, case| setter_setup(e, case, 0x128))
        .run()
        .assert_same();
}
#[test]
fn scene_node_set_point_position() {
    Diff::new(entry!(
        0x00b5_a9d0,
        lists::set_point_position(Ptr<_>, u32, u32, u32)
    ))
    .node()
    .inputs(&[Input::Word, Input::Word, Input::Word])
    .setup(|e, case| {
        world(e);
        let l = block(e, 0x250, &[]);
        vec![l, case.word(0), case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_compare_luminance() {
    Diff::new(entry!(
        0x00b5_aa70,
        lists::compare_luminance(Ptr<_>, Ptr<_>) -> i32
    ))
    .node()
    .inputs(&[Input::F32, Input::F32Swept(ONE)])
    .float_sweep(64)
    .setup(|e, case| {
        world(e);
        let a = block(e, 0x10, &[0, 0, 0, case.f32(0).to_bits()]);
        let b = block(e, 0x10, &[0, 0, 0, case.f32(1).to_bits()]);
        vec![block(e, 4, &[a]), block(e, 4, &[b])]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_indexed_child() {
    Diff::new(entry!(0x00b5_aa40, lists::indexed_child(Ptr<_>) -> u32))
        .node()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 0xffff_ffff, 0x8000_0000]),
            Input::Of(&[0, 1, 3, 0x8000]),
        ])
        .setup(|e, case| {
            world(e);
            let children = block(e, 0x10, &[0x1111, 0x2222, 0x3333, 0x4444]);
            let n = block(e, 0x100, &[]);
            e.mem.set_u32(n + 0xa0, children);
            e.mem.set_u16(n + 0xa6, case.word(1) as u16);
            e.mem.set_u32(n + 0xb0, case.word(0));
            vec![n]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_get_rtti() {
    Diff::new(entry!(0x00b5_e590, lists::get_rtti() -> Ptr))
        .node()
        .setup(|e, _| {
            world(e);
            vec![]
        })
        .run()
        .assert_same();
}

/// An array (`BSSimpleArray`-shaped) with its own vtable: `count` of `capacity`, its buffer holding
/// words `0x100 + i` for the larger of the two (none when `with_buffer` is false); the allocate
/// slot answers a block as large (the moves into it run as the exe's do).
fn array(e: &mut Engine, count: u32, capacity: u32, with_buffer: bool) -> u32 {
    let a = object(e, 0x10, 0, 0, OTHER_RTTI);
    let size = count.max(capacity).clamp(1, 0x1000);
    let buffer = if with_buffer {
        let words: Vec<u32> = (0..size).map(|i| 0x100 + i).collect();
        block(e, 4 * words.len() as u32, &words)
    } else {
        0
    };
    words(e, a + 4, &[buffer, count, capacity]);
    // The allocate slot's answer.
    let fresh = block(e, 4 * size.max(0x40), &[]);
    set_logged_answer(e, 0x3b, fresh);
    a
}

#[test]
fn scene_node_reserve_slot() {
    // Full with no capacity, full at 4 (with a buffer or none), 0x400 and 0x401, not full.
    Diff::new(entry!(0x00b5_c360, lists::reserve_slot(Ptr<_>) -> u32))
        .node()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5])])
        .setup(|e, case| {
            world(e);
            let (count, capacity, buffer) = [
                (0, 0, false),
                (4, 4, true),
                (0x400, 0x400, true),
                (0x401, 0x401, true),
                (2, 8, true),
                (4, 4, false),
            ][case.word(0) as usize];
            vec![array(e, count, capacity, buffer)]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_grow() {
    // No buffer, or one with a count equal to the capacity (4) or not (0, 2: moved into the new
    // buffer); the capacity asked for.
    Diff::new(entry!(0x00cb_9ce0, lists::grow(Ptr<_>, u32, u32)))
        .node()
        .inputs(&[Input::Flag, Input::Of(&[0, 2, 4]), Input::Of(&[8, 0x800])])
        .setup(|e, case| {
            world(e);
            let a = array(e, 2, 4, case.word(0) != 0);
            vec![a, case.word(2), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_find() {
    // The value at index 0, 2 or nowhere; the start before, at and past it; a null buffer.
    Diff::new(entry!(
        0x00b5_a930,
        lists::find(Ptr<_>, u32, u32, u32) -> u32
    ))
    .node()
    .inputs(&[
        Input::Of(&[0x100, 0x102, 0x999]),
        Input::Of(&[0, 1, 2, 5]),
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let a = array(e, 4, 4, case.word(2) == 0);
        vec![a, case.word(0), case.word(1), COMPARE]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_remove_at() {
    // Capacities each side of 0x400 and 0x800 (0x600: the capacity less 0x800 wraps, so any count
    // shrinks; 0x800: it is 0, so none does), counts each side of a quarter (and of the capacity
    // less 0x800); indexes; the shrink byte (0, 1, a word whose low byte is 0).
    Diff::new(entry!(0x00b5_ade0, lists::remove_at(Ptr<_>, u32, u8)))
        .node()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 0x100]),
        ])
        .setup(|e, case| {
            world(e);
            let (count, capacity) = [
                (2, 8),
                (3, 8),
                (0x100, 0x400),
                (0x101, 0x400),
                (0x101, 0x901),
                (0x102, 0x901),
                (0x10, 0x801),
                (3, 0),
                (0x300, 0x600),
                (0x10, 0x800),
            ][case.word(0) as usize];
            let a = array(e, count, capacity, true);
            // An index below the count (past it the exe's move runs away, as the exe would).
            vec![a, case.word(1).min(count - 1), case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_remove_run() {
    Diff::new(entry!(0x00b5_aea0, lists::remove_run(Ptr<_>, u32, u32)))
        .node()
        .inputs(&[Input::Of(&[0, 1, 3, 7]), Input::Of(&[0, 1, 2, 5])])
        .setup(|e, case| {
            world(e);
            vec![array(e, 8, 8, true), case.word(0), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_clear() {
    Diff::new(entry!(0x00b5_af80, lists::clear(Ptr<_>, u8)))
        .node()
        .inputs(&[Input::Flag, Input::Of(&[0, 1, 0x100])])
        .setup(|e, case| {
            world(e);
            vec![array(e, 3, 4, case.word(0) == 0), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_move_after() {
    // Four items: each item moved after each (the same included), so the head and tail pass on.
    Diff::new(entry!(0x00b5_a8d0, lists::move_after(Ptr<_>, u32, u32)))
        .node()
        .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            world(e);
            let l = block(e, 0xc, &[]);
            let items = list(e, l, &[0x10, 0x20, 0x30, 0x40]);
            vec![
                l,
                items[case.word(0) as usize],
                items[case.word(1) as usize],
            ]
        })
        .run()
        .assert_same();
}

/// With every family on (`Engine::new()`), the scene node's members are registered once and run:
/// `GetLight` finds a light by its `NiLight`, `00b5ac60` counts the lights, a light with a listed
/// light's `NiLight` added (`00b5eca0`) answers the listed one, and `RemoveAll` (`00e74d40`)
/// empties a list, letting each light go once.
#[test]
fn scene_node_members_run_with_every_family_on() {
    let mut e = Engine::new();
    let w = node_world(&mut e, 0);
    let ni = e.mem.u32(w.lights[1] + 0xf8);
    assert_eq!(e.call(0x00b5_b4a0, &args![w.node, ni]).u32(), w.lights[1]);
    assert_eq!(e.call(0x00b5_b4a0, &args![w.node, w.stray]).u32(), 0);
    let in_frustum = w
        .lights
        .iter()
        .chain(std::iter::once(&w.dark))
        .filter(|&&l| l != 0 && e.mem.u16(l + 0x110) != 0xff)
        .count() as u32;
    assert_eq!(e.call(0x00b5_ac60, &args![w.node]).u32(), in_frustum);
    let again = counted(&mut e, 0x250, 1);
    e.mem.set_u32(again + 0xf8, ni);
    assert_eq!(
        e.call(0x00b5_eca0, &args![w.node, again]).u32(),
        w.lights[1]
    );
    let before: Vec<u32> = w.shadow_lights.iter().map(|&l| e.mem.u32(l + 4)).collect();
    e.call(0x00e7_4d40, &args![w.node + 0xc0]);
    assert_eq!(e.mem.bytes(w.node + 0xc0, 12), vec![0; 12]);
    for (&l, n) in w.shadow_lights.iter().zip(before) {
        assert_eq!(e.mem.u32(l + 4), n - 1);
    }
}

/// The node world's seeds reach the branches, read on the world: lights with and without
/// `bCastShadow`, point and not, dynamic and not, culled `NiLight`s and not, cull words 3, 0xff and
/// 0x1ff, maps none, deleted and kept, the remove-queue flag on and off, the four `NiLight` kinds,
/// the point equal, unequal and unordered (+0/-0 and NaN/NaN pairs among them), a listed light
/// without an `NiLight`, the exe's bodies in the tree and stand-ins. (A failed allocation is an
/// input of the tests that take one: `SetSunLight`'s and `UpdateDynamicShadows`'.)
#[test]
fn the_node_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeSet::<String>::new();
    for &seed in SEEDS {
        let mut e = Engine::new_untyped_for_test();
        let w = node_world(&mut e, seed);
        for &l in w.lights.iter().chain(&w.shadow_lights) {
            let ni = e.mem.u32(l + 0xf8);
            let vt = e.mem.u32(ni);
            reached.insert(format!("cast {}", e.mem.u8(l + 0xec)));
            reached.insert(format!("point {}", e.mem.u8(l + 0xf4)));
            reached.insert(format!("dynamic {}", e.mem.u8(l + 0xfc)));
            reached.insert(format!("culled {}", e.mem.u32(ni + 0x30) & 1));
            reached.insert(format!("cull word {:x}", e.mem.u16(l + 0x110)));
            let map = e.mem.u32(l + 0x10c);
            reached.insert(format!(
                "map {}",
                if map == 0 { 0 } else { e.mem.u32(map + 4) }
            ));
            reached.insert(format!("rtti slot {:x}", e.mem.u32(vt + 8)));
            let same = (0..3).all(|i| e.mem.f32(l + 0x100 + 4 * i) == e.mem.f32(ni + 0x8c + 4 * i));
            reached.insert(format!("point same {same}"));
            let (z, nz) = (e.mem.u32(l + 0x108), e.mem.u32(ni + 0x94));
            if z != nz && z & 0x7fff_ffff == 0 && nz & 0x7fff_ffff == 0 {
                reached.insert("point pair zeros".into());
            }
            if f32::from_bits(z).is_nan() && f32::from_bits(nz).is_nan() {
                reached.insert("point pair nans".into());
            }
        }
        reached.insert(format!("allow {}", e.mem.u8(w.node + 0x1fc)));
        reached.insert(format!("dark {}", u8::from(w.dark != 0)));
        let vt = e.mem.u32(w.tree);
        reached.insert(format!("tree slot {:x}", e.mem.u32(vt + 0xc)));
    }
    for want in [
        "cast 0",
        "cast 1",
        "point 0",
        "point 1",
        "dynamic 0",
        "dynamic 1",
        "culled 0",
        "culled 1",
        "cull word 3",
        "cull word ff",
        "cull word 1ff",
        "point pair zeros",
        "point pair nans",
        "dark 0",
        "dark 1",
        "map 0",
        "map 1",
        "map 2",
        "allow 0",
        "allow 1",
        "point same true",
        "point same false",
        "tree slot 6815c0",
    ] {
        assert!(
            reached.contains(want),
            "not reached: {want} (reached {reached:?})"
        );
    }
    assert!(
        reached
            .iter()
            .filter(|r| r.starts_with("rtti slot"))
            .count()
            >= 3,
        "{reached:?}"
    );
    assert!(
        reached.iter().any(|r| r.starts_with("tree slot ff")),
        "{reached:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// The node's passes (`engine_typed::scene_node::node`) and the counted list's `RemoveAll`.

/// The answers of [`answer`] (`ANSWERS + 4 * K`) and of the allocation in turn (`ALLOCATIONS`,
/// the turn at `ALLOCATION_TURN`).
const ANSWERS: u32 = LOG + 0x3e80;
const ALLOCATIONS: u32 = LOG + 0x3e40;
const ALLOCATION_TURN: u32 = LOG + 0x3e3c;

/// Logs its words under `0x60 + K` and answers the word at `ANSWERS + 4 * K`.
fn answer<const K: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    for w in a {
        log(e, 0x60 + K, *w);
    }
    ret(e.mem.u32(ANSWERS + 4 * K))
}

fn set_answer(e: &mut Engine, k: u32, word: u32) {
    e.mem.set_u32(ANSWERS + 4 * k, word);
}

/// The allocation: logs the size and answers the next of the words [`set_allocations`] set.
fn allocation(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x2a, a[0]);
    let turn = e.mem.u32(ALLOCATION_TURN);
    e.mem.set_u32(ALLOCATION_TURN, turn + 1);
    ret(e.mem.u32(ALLOCATIONS + 4 * turn.min(3)))
}

fn set_allocations(e: &mut Engine, blocks: &[u32]) {
    e.register(0x00aa_13e0, allocation);
    words(e, ALLOCATIONS, blocks);
}

/// The culling process's constructor: logs, gives the local a vtable word and answers it.
fn culler_construct(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x54, a[0]);
    log(e, 0x54, a[1]);
    words(e, a[0], &[0x0109_c2a0, 0x5555]);
    ret(a[0])
}

/// An occlusion plane's test: logs its words, answers the plane's byte at `AT`.
fn plane_test<const AT: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    for w in a {
        log(e, 0x50 + AT, *w);
    }
    ret(u32::from(e.mem.u8(a[0] + AT)))
}

/// A list removal with an out `NiPointer`: logs the list, the out word and the argument, then
/// writes answer `K` (a counted light or none) into the out word.
fn out_removed<const K: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x60 + K, a[0]);
    log(e, 0x60 + K, e.mem.u32(a[1]));
    log(e, 0x60 + K, a[2]);
    let removed = e.mem.u32(ANSWERS + 4 * K);
    e.mem.set_u32(a[1], removed);
    ret(0)
}

/// The arrays' compare: 1 when the element's word is the value's.
fn compare_words(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x5f, a[0]);
    ret(u32::from(e.mem.u32(a[0]) == e.mem.u32(a[1])))
}

/// The light's luminance at a point: logs its words, answers the float at the light's +0x1f0.
fn luminance(e: &mut Engine, a: &[u32]) -> Ret {
    for w in a {
        log(e, 0x5e, *w);
    }
    ret_f(f64::from(f32::from_bits(e.mem.u32(a[0] + 0x1f0))))
}

/// The passes' stand-ins and data (part of [`world`]).
fn passes_stand_ins(e: &mut Engine) {
    // The sun colour, the zero point, the renderer's map word.
    words(e, 0x011f_4998, &[ONE, HALF, TWO]);
    words(e, 0x011f_426c, &[HALF, 0, MINUS_ONE]);
    e.mem.set_u32(0x011f_9508, 0x0909);
    // The renderer: its word +0xa8 (what `00b6d120` answers).
    let renderer = block(e, 0x100, &[]);
    e.mem.set_u32(renderer + 0xa8, 7);
    e.mem.set_u32(RENDERER, renderer);
    e.register(VT_FN, logged::<0x53>);
    e.register(VT_FN + 0xac, logged::<0x55>);
    e.register(VT_FN + 0xb0, logged::<0x56>);
    e.register(0x00a7_5c20, this_logged::<0x52>);
    e.register(0x00a6_9400, culler_construct);
    e.register(0x00a6_94a0, logged::<0x57>);
    e.register(0x00c3_3cf0, plane_test::<0x44>);
    e.register(0x00c3_58a0, plane_test::<0x45>);
    e.register(0x00b6_e110, answer::<0>);
    e.register(0x00b6_b260, answer::<1>);
    e.register(0x00b6_d3f0, answer::<3>);
    e.register(0x00b4_f5c0, answer::<4>);
    e.register(0x00b9_efa0, out_removed::<5>);
    e.register(0x00b9_fcc0, out_removed::<6>);
    e.register(0x00b5_a980, compare_words);
    e.register(0x00b9_bc50, luminance);
    e.register(0x00bd_8290, this_logged::<0x58>);
    e.register(0x00b9_f780, logged::<0x59>);
    e.register(0x00ba_3130, logged::<0x5a>);
    e.register(0x00b4_f2f0, logged::<0x5b>);
    e.register(0x00b6_da10, logged::<0x5c>);
    e.register(0x00c4_7770, logged::<0x5d>);
    e.register(0x00a5_e930, logged::<0x4e>);
    e.register(0x00aa_1460, logged::<0x4f>);
}

/// A counted list at `at` of `n` elements (index 1 null when `n > 2`), each counted once or twice
/// by the seed; an empty one keeps a stale tail and count.
fn counted_list(e: &mut Engine, s: &Seed, at: u32, n: u32) -> Vec<u32> {
    let elements: Vec<u32> = (0..n)
        .map(|i| {
            if i == 1 && n > 2 {
                0
            } else {
                counted(e, 0x40, 1 + s.pick(20 + i, 2))
            }
        })
        .collect();
    list(e, at, &elements);
    if n == 0 {
        words(e, at + 4, &[0x1234, 5]);
    }
    elements
}

fn remove_all_setup(e: &mut Engine, case: &Case) -> Vec<u32> {
    world(e);
    let s = Seed::new(case.word(0));
    // The pool's count reaches zero at the first item, the second, or not at all.
    e.mem
        .set_u32(0x011c_5f84, [1, 2, 9][s.pick(19, 3) as usize]);
    let at = block(e, 0xc, &[]);
    counted_list(e, &s, at, case.word(1));
    vec![at]
}

#[test]
fn scene_node_remove_all() {
    Diff::new(entry!(0x00e7_4d40, lists::remove_all(Ptr<_>)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 2, 4])]))
        .setup(remove_all_setup)
        .run()
        .assert_same();
}

#[test]
fn scene_node_remove_all_jump() {
    Diff::new(entry!(0x00b5_c7c0, node::remove_all_jump(Ptr<_>)))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 3])]))
        .setup(remove_all_setup)
        .run()
        .assert_same();
}

#[test]
fn scene_node_set_sun_light() {
    // A sun light or none; a light given or none; the allocations all made, the first failing,
    // the second failing (a failed `NiDirectionalLight` writes the low page, where the exe would
    // fault).
    Diff::new(entry!(0x00b5_aac0, node::set_sun_light(Ptr<_>, u32)))
        .node()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            let w = node_world(e, case.word(0));
            e.map(0, 0x1000);
            if case.word(1) != 0 {
                let sun = counted(e, 0x250, 1);
                e.mem.set_u32(w.node + 0xe0, sun);
            }
            let light = if case.word(2) != 0 {
                ni_light(e, 1, [0; 3], 0)
            } else {
                0
            };
            let blocks = [block(e, 0x250, &[]), block(e, 0x250, &[])];
            e.mem.set_u32(blocks[0] + 0xa8, 5);
            let answers = match case.word(3) {
                0 => [blocks[0], blocks[1]],
                1 => [0, blocks[1]],
                _ => [blocks[0], 0],
            };
            set_allocations(e, &answers);
            vec![w.node, light]
        })
        .run()
        .assert_same();
}

#[test]
fn scene_node_update_occlusion_plane_visibility() {
    // No graph, an empty plane list, or four planes (one null) whose two tests answer 0, 1 or 2
    // (2 and 1 meet in no bit).
    Diff::new(entry!(
        0x00b5_ac90,
        node::update_occlusion_plane_visibility(Ptr<_>, u32)
    ))
    .node()
    .inputs(&seeds_and(&[Input::Of(&[0, 1, 2])]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        let s = Seed::new(case.word(0));
        let graph = if case.word(1) == 0 {
            0
        } else {
            block(e, 0x40, &[])
        };
        e.mem.set_u32(w.node + 0x1e0, graph);
        if case.word(1) == 2 {
            let planes: Vec<u32> = (0..4u32)
                .map(|i| {
                    if i == 2 {
                        return 0;
                    }
                    let p = block(e, 0x50, &[]);
                    e.mem.set_u32(p + 0x40, 9);
                    e.mem.set_u8(p + 0x44, s.pick(30 + i, 3) as u8);
                    e.mem.set_u8(p + 0x45, s.pick(34 + i, 3) as u8);
                    p
                })
                .collect();
            list(e, graph + 8, &planes);
        }
        let camera = block(e, 0x200, &[]);
        words(e, camera + 0x8c, &[ONE, TWO, HALF]);
        words(
            e,
            camera + 0xdc,
            &[MINUS_ONE, ONE, ONE, MINUS_ONE, HALF, TWO],
        );
        vec![w.node, camera]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_add_listed_light() {
    // A new light with the `NiLight` of the first or the last listed light, or one of its own;
    // casting shadows or not; the byte at 0xed set or not.
    Diff::new(entry!(
        0x00b5_eca0,
        node::add_listed_light(Ptr<_>, u32) -> u32
    ))
    .node()
    .inputs(&seeds_and(&[
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Flag,
    ]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        let s = Seed::new(case.word(0));
        e.map(0, 0x1000);
        e.mem.set_u8(0xed, case.word(3) as u8);
        let light = shadow_light(e, &s, 24, 0);
        let ni = match case.word(1) {
            0 => e.mem.u32(w.lights[0] + 0xf8),
            1 => e.mem.u32(w.lights[2] + 0xf8),
            _ => ni_light(e, 1, [0; 3], 0),
        };
        e.mem.set_u32(light + 0xf8, ni);
        e.mem.set_u8(light + 0xec, case.word(2) as u8);
        e.mem.set_u8(light + 0xfc, 2 + s.pick(17, 2) as u8);
        let map = counted(e, 0x40, 1);
        let target = object(e, 0x40, 0, 0, OTHER_RTTI);
        let other = counted(e, 0x40, 2);
        set_answer(e, 0, map);
        set_answer(e, 1, target);
        set_answer(e, 3, other);
        vec![w.node, light]
    })
    .run()
    .assert_same();
}

/// An array (`BSSimpleArray`-shaped, vtable `vt`) at `at`: `count` words of four, the light at
/// `place` when it is below the count.
fn light_array(e: &mut Engine, at: u32, vt: u32, count: u32, place: u32, light: u32) {
    let buffer = block(e, 0x20, &[0x11, 0x22, 0x33, 0x44]);
    if place < count {
        e.mem.set_u32(buffer + 4 * place, light);
    }
    words(e, at, &[vt, buffer, count, 8]);
}

/// The queued-removal world: the node's portal graph (input 1: none, one whose word +0x28 is
/// clear, one where it is set), a light of `lLightList` or `lShadowLightList` held in a word, its
/// rooms (one null, one with an empty array, one whose array holds the light at the seed's place
/// or not) and its portals (one null, two whose object's array holds the light or not, one with a
/// null buffer for some seeds), and the out word's answers (a counted light or none). Answers the
/// world, the holder and the light.
fn queued_world(e: &mut Engine, case: &Case) -> (NodeWorld, u32, u32) {
    let w = node_world(e, case.word(0));
    let s = Seed::new(case.word(0));
    let graph = block(e, 0x40, &[]);
    e.mem.set_u32(graph + 0x28, u32::from(case.word(1) == 2));
    let node_graph = if case.word(1) == 0 { 0 } else { graph };
    e.mem.set_u32(w.node + 0x1e0, node_graph);
    let light = if s.pick(40, 2) == 0 {
        w.lights[0]
    } else {
        w.shadow_lights[1]
    };
    let holder = block(e, 4, &[light]);
    let owner = object(e, 0x10, 0, 0, OTHER_RTTI);
    let vt = e.mem.u32(owner);
    // A shrinking removal's new buffer (the allocate slot's answer).
    let fresh = block(e, 0x40, &[]);
    set_logged_answer(e, 0x3b, fresh);
    let mut rooms = vec![0u32; 3];
    for (i, room) in (0u32..).zip(rooms.iter_mut()) {
        if i == 1 {
            continue;
        }
        let r = block(e, 0x100, &[]);
        let count = if i == 0 { 0 } else { 2 + s.pick(41, 3) };
        light_array(e, r + 0xdc, vt, count, s.pick(42, 4), light);
        *room = r;
    }
    let mut portals = vec![0u32; 3];
    for (i, portal) in (0u32..).zip(portals.iter_mut()) {
        if i == 0 {
            continue;
        }
        let object = block(e, 0x100, &[]);
        light_array(
            e,
            object + 0xb0,
            vt,
            1 + s.pick(43 + i, 3),
            s.pick(45 + i, 3),
            light,
        );
        if i == 2 && s.pick(47, 4) == 0 {
            e.mem.set_u32(object + 0xb4, 0);
        }
        let p = block(e, 0x110, &[]);
        e.mem.set_u32(p + 0x104, object);
        *portal = p;
    }
    let rooms = block(e, 12, &rooms);
    let portals = block(e, 12, &portals);
    words(e, light + 0x214, &[rooms, 3]);
    words(e, light + 0x224, &[portals, 3]);
    // The light's graph: the node's or another.
    let light_graph = if s.pick(48, 2) == 0 {
        node_graph
    } else {
        0x0ddd_0000
    };
    e.mem.set_u32(light + 0x240, light_graph);
    for k in [5, 6] {
        let removed = if s.pick(49, 3) == 0 {
            0
        } else {
            counted(e, 0x40, s.pick(50, 2) + 1)
        };
        set_answer(e, k, removed);
    }
    (w, holder, light)
}

#[test]
fn scene_node_remove_queued_light() {
    Diff::new(entry!(
        0x00b5_d040,
        node::remove_queued_light(Ptr<_>, Ptr<_>, u32)
    ))
    .node()
    .inputs(&seeds_and(&[Input::Of(&[0, 1, 2])]))
    .setup(|e, case| {
        let (w, holder, _) = queued_world(e, case);
        let item = e.mem.u32(w.node + 0xb4);
        let position = block(e, 4, &[item]);
        vec![w.node, holder, position]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_remove_queued_light_of() {
    // The light listed or not (a light of no list), of the node's graph or another.
    Diff::new(entry!(
        0x00b5_ef90,
        node::remove_queued_light_of(Ptr<_>, Ptr<_>)
    ))
    .node()
    .inputs(&seeds_and(&[Input::Of(&[0, 1, 2]), Input::Flag]))
    .setup(|e, case| {
        let (w, holder, light) = queued_world(e, case);
        if case.word(2) != 0 {
            let s = Seed::new(case.word(0));
            let unlisted = shadow_light(e, &s, 32, 0);
            let (rooms, portals) = (e.mem.u32(light + 0x214), e.mem.u32(light + 0x224));
            words(e, unlisted + 0x214, &[rooms, 3]);
            words(e, unlisted + 0x224, &[portals, 3]);
            let graph = e.mem.u32(light + 0x240);
            e.mem.set_u32(unlisted + 0x240, graph);
            e.mem.set_u32(holder, unlisted);
        }
        vec![w.node, holder]
    })
    .run()
    .assert_same();
}

/// The lights' luminances (a float of each light's +0x1f0) and the sun colours' components.
const LUMINANCES: [u32; 5] = [0, ONE, MINUS_ONE, THIRD, 0x8000_0000];
const COMPONENTS: [u32; 6] = [ONE, HALF, TWO, 0x7fc0_0000, 0x7f80_0001, 0x8000_0000];

#[test]
fn scene_node_light_value_at() {
    // The point; the excluded `NiLight` (none, or the first light's); the seed's luminances (a
    // running sum that goes above, to and below 0.0) and colours (NaNs, a signalling one, -0.0).
    Diff::new(entry!(
        0x00b5_b4d0,
        node::light_value_at(Ptr<_>, u32, u32, u32, Ptr<_>, Ptr<_>, Ptr<_>, u32) -> f32
    ))
    .node()
    .inputs(&seeds_and(&[Input::Flag]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        let s = Seed::new(case.word(0));
        for (i, &l) in (0u32..).zip(&w.lights) {
            e.mem
                .set_u32(l + 0x1f0, LUMINANCES[s.pick(50 + i, 5) as usize]);
        }
        let sun = counted(e, 0x250, 1);
        let sun_light = block(e, 0x100, &[]);
        e.mem.set_u32(sun + 0xf8, sun_light);
        for (i, at) in (0u32..).zip([0xc8u32, 0xcc, 0xd0, 0xd4, 0xd8, 0xdc]) {
            let component = COMPONENTS[s.pick(53 + i, 6) as usize];
            e.mem.set_u32(sun_light + at, component);
        }
        e.mem.set_u32(w.node + 0xe0, sun);
        let count = block(e, 4, &[s.pick(59, 3)]);
        let ambient = block(e, 4, &[]);
        let diffuse = block(e, 4, &[]);
        let excluded = if case.word(1) != 0 {
            e.mem.u32(w.lights[0] + 0xf8)
        } else {
            0
        };
        vec![w.node, ONE, HALF, TWO, count, ambient, diffuse, excluded]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_node_update_dynamic_shadows() {
    // A camera or none (made: allocated, or the allocation failing); the shadow lights as they
    // are, none, the first without an `NiLight`, or all five lights drawn (no rendering system
    // call left).
    Diff::new(entry!(
        0x00b5_b880,
        node::update_dynamic_shadows(Ptr<_>, u32)
    ))
    .node()
    .inputs(&seeds_and(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2, 3]),
    ]))
    .setup(|e, case| {
        let w = node_world(e, case.word(0));
        let s = Seed::new(case.word(0));
        e.map(0, 0x1000);
        let camera = counted(e, 0x140, 1 + s.pick(60, 2));
        let held = if s.pick(61, 3) == 0 {
            0
        } else {
            counted(e, 0x40, 1 + s.pick(62, 2))
        };
        e.mem.set_u32(camera + 0x130, held);
        match case.word(1) {
            0 => e.mem.set_u32(w.node + 0x128, camera),
            1 => {
                e.mem.set_u32(camera + 4, 0);
                set_allocations(e, &[camera]);
            }
            _ => set_allocations(e, &[0]),
        }
        match case.word(2) {
            1 => {
                list(e, w.node + 0xc0, &[]);
            }
            2 => e.mem.set_u32(w.shadow_lights[0] + 0xf8, 0),
            3 => {
                let all = [
                    w.lights[0],
                    w.lights[1],
                    w.lights[2],
                    w.shadow_lights[0],
                    w.shadow_lights[1],
                ];
                for l in all {
                    e.mem.set_u16(l + 0x110, 3);
                    let ni = e.mem.u32(l + 0xf8);
                    let flags = e.mem.u32(ni + 0x30) & !1;
                    e.mem.set_u32(ni + 0x30, flags);
                }
                list(e, w.node + 0xc0, &all);
            }
            _ => {}
        }
        let tls = e.tls();
        e.mem.set_u32(tls + 0x2b4, 0x55);
        vec![w.node, 0x1234]
    })
    .run()
    .assert_same();
}

/// The doomed node: its sun light, fence objects, reflection map, cube camera and fog (each
/// counted or none), its queues (counted lists of up to two), its scene graph index (the node at
/// that index or another), and the accumulator (none, one whose scene node is this, another).
fn doomed_world(e: &mut Engine, seed: u32) -> u32 {
    let w = node_world(e, seed);
    let s = Seed::new(seed);
    let node = w.node;
    for (i, member) in (0u32..).zip([0xe0u32, 0xd8, 0xdc, 0x124, 0x128, 0x134]) {
        let held = if s.pick(70 + i, 3) == 0 {
            0
        } else {
            counted(e, 0x40, 1 + s.pick(76 + i, 2))
        };
        e.mem.set_u32(node + member, held);
    }
    for (i, queue) in (0u32..).zip([0xe4u32, 0xf0, 0xfc, 0x108, 0x114]) {
        counted_list(e, &s, node + queue, s.pick(82 + i, 3));
    }
    let index = s.pick(87, 3);
    e.mem.set_u8(node + 0x120, index as u8);
    let at_index = if s.pick(88, 2) == 0 {
        node
    } else {
        0x0abc_d000
    };
    e.mem.set_u32(0x011f_91c8 + 4 * index, at_index);
    let accumulator = match s.pick(89, 3) {
        0 => 0,
        k => {
            let a = block(e, 0x200, &[]);
            e.mem
                .set_u32(a + 0x194, if k == 1 { node } else { 0x0abc_e000 });
            a
        }
    };
    set_answer(e, 4, accumulator);
    node
}

#[test]
fn scene_node_destruct() {
    Diff::new(entry!(0x00b5_e5a0, node::destruct(Ptr<_>)))
        .node()
        .inputs(&seeds_and(&[]))
        .setup(|e, case| vec![doomed_world(e, case.word(0))])
        .run()
        .assert_same();
}

#[test]
fn scene_node_delete() {
    Diff::new(entry!(0x00b5_f980, node::delete(Ptr<_>, u32) -> Ptr))
        .node()
        .inputs(&seeds_and(&[Input::Of(&[0, 1, 2, 3])]))
        .setup(|e, case| vec![doomed_world(e, case.word(0)), case.word(1)])
        .run()
        .assert_same();
}

/// The passes' seeds reach their branches: the plane tests' answers, the queued-removal world's
/// choices (the light's list and graph, the arrays' places, a null buffer, the removed pointer),
/// the luminance sums' signs and the colours' NaNs, the doomed node's members, queues, index and
/// accumulator.
#[test]
fn the_pass_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeSet::<String>::new();
    for &seed in SEEDS {
        let s = Seed::new(seed);
        for i in 0..4 {
            reached.insert(format!("plane within {}", s.pick(30 + i, 3)));
            reached.insert(format!("plane side {}", s.pick(34 + i, 3)));
        }
        reached.insert(format!("queued light {}", s.pick(40, 2)));
        reached.insert(format!(
            "room place below {}",
            s.pick(42, 4) < 2 + s.pick(41, 3)
        ));
        reached.insert(format!("null buffer {}", s.pick(47, 4) == 0));
        reached.insert(format!("light graph {}", s.pick(48, 2)));
        reached.insert(format!("removed {}", s.pick(49, 3).min(1)));
        let mut sum = 0.0f32;
        for i in 0..3 {
            sum += f32::from_bits(LUMINANCES[s.pick(50 + i, 5) as usize]);
            reached.insert(format!("sum above {}", sum > 0.0));
        }
        for i in 0..6 {
            reached.insert(format!("component {}", s.pick(53 + i, 6)));
        }
        for i in 0..6 {
            reached.insert(format!("member {}", s.pick(70 + i, 3).min(1)));
        }
        reached.insert(format!("at index {}", s.pick(88, 2)));
        reached.insert(format!("accumulator {}", s.pick(89, 3)));
    }
    let mut want: Vec<String> = Vec::new();
    for v in 0..3 {
        want.push(format!("plane within {v}"));
        want.push(format!("plane side {v}"));
        want.push(format!("accumulator {v}"));
    }
    for v in 0..2 {
        want.push(format!("queued light {v}"));
        want.push(format!("light graph {v}"));
        want.push(format!("removed {v}"));
        want.push(format!("member {v}"));
        want.push(format!("at index {v}"));
    }
    for v in 0..6 {
        want.push(format!("component {v}"));
    }
    for b in [false, true] {
        want.push(format!("room place below {b}"));
        want.push(format!("null buffer {b}"));
        want.push(format!("sum above {b}"));
    }
    for w in want {
        assert!(
            reached.contains(&w),
            "not reached: {w} (reached {reached:?})"
        );
    }
}
