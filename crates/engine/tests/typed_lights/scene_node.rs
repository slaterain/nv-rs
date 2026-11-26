//! The shadow scene node's functions (`engine_typed::lights::scene_node`) against their
//! translations. One world for the node functions: a scene node whose `lLightList` holds three
//! lights and a null element and whose `lShadowLightList` holds two lights, each light counted,
//! with its `NiLight` (an RTTI of its own or one of the exe's `GetRTTI` bodies), flags, cull word,
//! shadow scene, shadow map and point; its portal graph and remove-queue flag; and a tree of nodes,
//! geometries and lights for the walks. The seed word picks each choice (see [`Seed`]). The arrays
//! and the list move get worlds of their own.
//!
//! What stands in (logging their words): the queues' lock, the counted list's add, the allocation,
//! the light's constructor, `SetLight`, shadow flag and geometry removal (their own Diff tests are
//! the lights family's), the set-up family's shadow drop, the list's `RemoveAll`, the object update,
//! the cast, `NiLight`'s constructor, the light update and cull, the renderer's map release and end,
//! the property's light walk, the arrays' moves, reallocation and slots, and the search's compare.

use super::{
    block, log, logged, object, ret, set_logged_answer, stand_ins, words, EXE_BODIES, HALF, LOG,
    MINUS_ONE, ONE, OTHER_RTTI, TWO, VT_FN, VT_SLOTS,
};
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::scene_node::{lists, SCENE_NODE};

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
/// `SetLight`, shadow flag and `RemoveTriGeom`, the object update and the cast.
const CROSS: [u32; 9] = [
    0x0040_fbf0,
    0x00a5_a310,
    0x00b5_c3c0,
    0x00b9_fda0,
    0x00b9_dda0,
    0x00b9_dcb0,
    0x00ba_01a0,
    0x00a5_9c60,
    0x0065_3290,
];
/// What the typed code does in place: `NiPoint3`'s `!=` and `==`.
const IN_PLACE: [u32; 2] = [0x0043_9090, 0x0043_90c0];

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
    e.register(0x00e7_4d40, logged::<0x30>);
    e.register(0x00a5_9c60, update);
    e.register(0x0065_3290, cast);
    e.register(0x00a7_d670, this_logged::<0x33>);
    e.register(0x00b5_d300, logged::<0x34>);
    e.register(0x00b9_ef30, logged::<0x35>);
    e.register(0x00b6_d4c0, release_map);
    e.register(0x00b6_d130, logged::<0x37>);
    e.register(0x00b7_0490, next_light);
    e.register(0x00cb_9930, logged::<0x39>);
    e.register(0x00cb_9ce0, logged::<0x3a>);
    e.register(COMPARE, compare);
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
    /// An `NiLight` no light has.
    stray: u32,
    /// A tree: a node with a geometry, a light, a node with a light and a geometry, and a null child.
    tree: u32,
}

/// A shadow scene light of the seed's choices `i..`: its `NiLight`, flags, cull word, shadow scene,
/// map and point.
fn shadow_light(e: &mut Engine, s: &Seed, i: u32, scene: u32) -> u32 {
    let l = counted(e, 0x250, 1 + s.pick(i, 2));
    let at = [ONE, HALF, MINUS_ONE];
    let culled = s.pick(i + 1, 4) == 3;
    let ni = ni_light(e, s.pick(i + 2, 4), at, u32::from(culled) | 0x10);
    e.mem.set_u32(l + 0xf8, ni);
    e.mem.set_u8(l + 0xec, u8::from(s.pick(i + 3, 4) == 0));
    e.mem.set_u8(l + 0xed, 1);
    e.mem.set_u8(l + 0xf4, u8::from(s.pick(i + 4, 4) != 0));
    e.mem.set_u8(l + 0xfc, u8::from(s.pick(i + 5, 4) != 1));
    let point = match s.pick(i + 6, 4) {
        0 => at,
        1 => [ONE, HALF, 0x7fc0_0000],
        2 => [ONE, TWO, MINUS_ONE],
        _ => [0x8000_0000 | ONE, HALF, MINUS_ONE],
    };
    words(e, l + 0x100, &point);
    e.mem
        .set_u16(l + 0x110, if s.pick(i + 1, 3) == 2 { 0xff } else { 3 });
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
    list(e, node + 0xb4, &[lights[0], 0, lights[1], lights[2]]);
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
/// words `0x100 + i` (none when `capacity` is 0 and `with_buffer` is false).
fn array(e: &mut Engine, count: u32, capacity: u32, with_buffer: bool) -> u32 {
    let a = object(e, 0x10, 0, 0, OTHER_RTTI);
    let buffer = if with_buffer {
        let words: Vec<u32> = (0..capacity.clamp(1, 0x40)).map(|i| 0x100 + i).collect();
        block(e, 4 * words.len() as u32, &words)
    } else {
        0
    };
    words(e, a + 4, &[buffer, count, capacity]);
    // The allocate slot's answer.
    let fresh = block(e, 0x100, &[]);
    set_logged_answer(e, 0x3b, fresh);
    a
}

#[test]
fn scene_node_reserve_slot() {
    // Full with no capacity, full at 4, 0x400 and 0x401, not full.
    Diff::new(entry!(0x00b5_c360, lists::reserve_slot(Ptr<_>) -> u32))
        .node()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4])])
        .setup(|e, case| {
            world(e);
            let (count, capacity) =
                [(0, 0), (4, 4), (0x400, 0x400), (0x401, 0x401), (2, 8)][case.word(0) as usize];
            vec![array(e, count, capacity, capacity != 0)]
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
    // Capacities each side of 0x400 and 0x800, counts each side of a quarter (and of the
    // capacity less 0x800); indexes; the shrink byte (0, 1, a word whose low byte is 0).
    Diff::new(entry!(0x00b5_ade0, lists::remove_at(Ptr<_>, u32, u8)))
        .node()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
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
            ][case.word(0) as usize];
            let a = array(e, count, capacity, true);
            vec![a, case.word(1), case.word(2)]
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
/// `GetLight` finds a light by its `NiLight` and `00b5ac60` counts the lights.
#[test]
fn scene_node_members_run_with_every_family_on() {
    let mut e = Engine::new();
    let w = node_world(&mut e, 0);
    let ni = e.mem.u32(w.lights[1] + 0xf8);
    assert_eq!(e.call(0x00b5_b4a0, &args![w.node, ni]).u32(), w.lights[1]);
    assert_eq!(e.call(0x00b5_b4a0, &args![w.node, w.stray]).u32(), 0);
    let culled = w
        .lights
        .iter()
        .filter(|&&l| e.mem.u16(l + 0x110) == 0xff)
        .count() as u32;
    assert_eq!(e.call(0x00b5_ac60, &args![w.node]).u32(), 3 - culled);
}

/// The node world's seeds reach the branches, read on the world: lights with and without
/// `bCastShadow`, point and not, dynamic and not, culled `NiLight`s and not, cull words 0xff and
/// not, maps none, deleted and kept, the remove-queue flag on and off, the four `NiLight` kinds,
/// the point equal, unequal and unordered, the exe's bodies in the tree and stand-ins, the
/// allocation failing and not.
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
        }
        reached.insert(format!("allow {}", e.mem.u8(w.node + 0x1fc)));
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
