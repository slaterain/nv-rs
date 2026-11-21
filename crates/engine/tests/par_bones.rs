//! `NV_PAR_BONES` (the actors' postponed bone updates walked on helper engines,
//! `engine_typed::process::bones`): the pass with the switch on ends in the serial pass's state and
//! makes the serial pass's outside calls in the serial order, however many helpers walk; and the
//! check mode finds no difference.
//!
//! The world, in guest memory: the process lists with levels 0 and 1 holding the actors (and a null
//! slot and a non-actor), each actor with its animation (the pending mark, its roots, the time) and a
//! tree of the typed classes: a root node, a spine of bone nodes merging their bounds, geometries,
//! bones with blend collision objects (fully keyframed: `hktoNiTransform` writes the body through
//! `NitohkTransform`; and node-moved: `NitohkTransform`, then the bit cleared), an attached weapon
//! (a node with a geometry under a hand bone), and a head (`BSFaceGenNiNode::UpdateDownwardPass`, a
//! stand-in that records and goes on with the `NiNode` pass, so the helpers log it for main). Then
//! the actors that must run whole on main: one with a controller on a bone, one ragdolled (its blend
//! object reads the body into the node), one whose animation is not pending (a dead actor that asks
//! for its lighting), and one whose geometry has no world bound yet (allocated on main).
//!
//! Recorded on the host, with whether main made it: the Havok parts (the hard keyframe, the
//! by-address `hktoNiTransform` of the ragdoll), the head's pass (with its parent's world
//! translation), the controller's update and the lighting queue. Compared after each pass: the
//! records, and every object's bytes the pass may write.
//!
//! One test: the helper pool is one per process, made from the first engine that forks.

use engine as _;
use engine_core::prelude::*;
use engine_typed::process::bones;
use std::sync::Mutex;

/// `ProcessLists::ParallelUpdateAnimationScenegraph`, the pass under test.
const PASS: u32 = 0x0096_c970;

/// Typed passes put in the vtables.
const NODE_DOWNWARD: u32 = 0x00a5_dd70;
const OBJECT_DOWNWARD: u32 = 0x00a5_9f90;
const WORLD_DATA: u32 = 0x00a6_8c60;
const GEOMETRY_BOUND: u32 = 0x00a8_0710;
const BLEND_UPDATE: u32 = 0x00c8_17d0;
const BLEND_HK_TO_NI: u32 = 0x00c8_1e30;
const NI_TO_HK: u32 = 0x00c6_5b00;
/// The head's pass (a stand-in here).
const HEAD_DOWNWARD: u32 = 0x0066_3a70;

/// Stand-ins.
const VT_FN: u32 = 0x00ff_2000;
const IS_ACTOR_FN: u32 = VT_FN;
const ANIMATION_FN: u32 = VT_FN + 4;
const NODE_FN: u32 = VT_FN + 8;
const BODY_POSITION_FN: u32 = VT_FN + 0xc;
const CONTROLLER_FN: u32 = VT_FN + 0x10;
const OTHER_FN: u32 = VT_FN + 0x14;
const NOT_ACTOR_FN: u32 = VT_FN + 0x18;
const SET_BOUND_FN: u32 = VT_FN + 0x1c;
const MOVE_BODY_FN: u32 = VT_FN + 0x20;
/// `MemoryManager::Allocate` (the bound pool's block for an object without a world bound): the next
/// block of a fixed area, the same each pass.
const POOL_ALLOCATE: u32 = 0x00aa_3e40;
const POOL_AREA: u32 = 0x0300_0800;
const SETTING_VALUE: u32 = 0x0040_8d60;
const TASKLET_VALUE: u32 = 0x0300_0000;
const OBJECT_TEST: u32 = 0x0047_b470;
const APPLY_HARD_KEYFRAME: u32 = 0x00c8_e160;
const COLLISION_HK_TO_NI: u32 = 0x00c6_5a80;
const OBJECT_LIGHTING: u32 = 0x00b5_d9f0;
const INCREMENT: u32 = 0x00fd_f1c0;
const DECREMENT: u32 = 0x00fd_f1c4;

const ONE: u32 = 0x3f80_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Rec {
    tag: &'static str,
    words: Vec<u32>,
    main: bool,
}

static RECORD: Mutex<Vec<Rec>> = Mutex::new(Vec::new());
static MAIN: Mutex<Option<std::thread::ThreadId>> = Mutex::new(None);

fn record(tag: &'static str, words: Vec<u32>) {
    let main = *MAIN.lock().unwrap() == Some(std::thread::current().id());
    RECORD.lock().unwrap().push(Rec { tag, words, main });
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The scene's guest blocks, to put back before each pass.
struct World {
    lists: u32,
    blocks: Vec<(u32, u32)>,
    reset: Vec<(u32, Vec<u8>)>,
    objects: Vec<u32>,
}

impl World {
    fn alloc(&mut self, e: &mut Engine, size: u32) -> u32 {
        let a = e.mem.alloc(size);
        for k in (0..size).step_by(4) {
            e.mem.set_u32(a + k, 0);
        }
        self.blocks.push((a, size));
        a
    }

    /// A vtable whose slots hold `slots` (others a recording stand-in).
    fn vtable(&mut self, e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vt = self.alloc(e, 0x400);
        for s in (0..0x400).step_by(4) {
            e.mem.set_u32(vt + s, OTHER_FN);
        }
        for &(slot, f) in slots {
            e.mem.set_u32(vt + slot, f);
        }
        vt
    }
}

fn identity(e: &mut Engine, at: u32, translation: [f32; 3]) {
    for (i, w) in [ONE, 0, 0, 0, ONE, 0, 0, 0, ONE].iter().enumerate() {
        e.mem.set_u32(at + 4 * i as u32, *w);
    }
    for (i, t) in translation.iter().enumerate() {
        e.mem.set_f32(at + 0x24 + 4 * i as u32, *t);
    }
    e.mem.set_u32(at + 0x30, ONE);
}

/// The kinds of object a tree holds.
struct Kinds {
    node: u32,
    geometry: u32,
    head: u32,
    blend: u32,
}

/// A node (`vt`) with a local translation, a world bound, flags (0x80000: its bound merged from
/// its children's) and room for `children`.
fn node(
    w: &mut World,
    e: &mut Engine,
    vt: u32,
    parent: u32,
    local: [f32; 3],
    children: u32,
) -> u32 {
    let n = w.alloc(e, 0xac);
    e.mem.set_u32(n, vt);
    e.mem.set_u32(n + 4, 1);
    e.mem.set_u32(n + 0x18, parent);
    let bound = w.alloc(e, 0x10);
    e.mem.set_u32(n + 0x20, bound);
    e.mem.set_u32(n + 0x30, 0x0008_0000);
    identity(e, n + 0x34, local);
    identity(e, n + 0x68, [0.0; 3]);
    let array = w.alloc(e, 4 * children.max(1));
    e.mem.set_u32(n + 0xa0, array);
    e.mem.set_u16(n + 0xa4, children as u16);
    if parent != 0 {
        let count = e.mem.u16(parent + 0xa6);
        let base = e.mem.u32(parent + 0xa0);
        e.mem.set_u32(base + 4 * u32::from(count), n);
        e.mem.set_u16(parent + 0xa6, count + 1);
    }
    w.objects.push(n);
    n
}

/// A geometry with a model bound of `radius` under `parent` (`bound`: give it a world bound).
fn geometry(
    w: &mut World,
    e: &mut Engine,
    k: &Kinds,
    parent: u32,
    radius: f32,
    bound: bool,
) -> u32 {
    let g = w.alloc(e, 0xc0);
    e.mem.set_u32(g, k.geometry);
    e.mem.set_u32(g + 4, 1);
    e.mem.set_u32(g + 0x18, parent);
    if bound {
        let b = w.alloc(e, 0x10);
        e.mem.set_u32(g + 0x20, b);
    }
    identity(e, g + 0x34, [0.5, 0.25, 0.0]);
    identity(e, g + 0x68, [0.0; 3]);
    let data = w.alloc(e, 0x20);
    e.mem.set_f32(data + 0x10, 0.1);
    e.mem.set_f32(data + 0x14, 0.2);
    e.mem.set_f32(data + 0x18, 0.3);
    e.mem.set_f32(data + 0x1c, radius);
    e.mem.set_u32(g + 0xb8, data);
    let count = e.mem.u16(parent + 0xa6);
    let base = e.mem.u32(parent + 0xa0);
    e.mem.set_u32(base + 4 * u32::from(count), g);
    e.mem.set_u16(parent + 0xa6, count + 1);
    w.objects.push(g);
    g
}

/// A blend collision object on `bone`: `keyframed` (fully, motion mode 4) or node-moved; `amount`
/// the blend (1.0 keyframed; 0.0 a ragdoll's).
fn blend(w: &mut World, e: &mut Engine, k: &Kinds, bone: u32, keyframed: bool, amount: f32) {
    let c = w.alloc(e, 0x28);
    e.mem.set_u32(c, k.blend);
    e.mem.set_u32(c + 8, bone);
    e.mem.set_u16(c + 0xc, if keyframed { 1 } else { 0x40 });
    let body_vt = w.vtable(e, &[(0xd4, BODY_POSITION_FN), (0xe8, MOVE_BODY_FN)]);
    let body = w.alloc(e, 0x40);
    e.mem.set_u32(body, body_vt);
    let object = w.alloc(e, 0x100);
    e.mem.set_u8(object + 0xe8, 4);
    e.mem.set_u32(body + 8, object);
    e.mem.set_u32(c + 0x10, body);
    e.mem.set_f32(c + 0x18, amount);
    e.mem.set_u32(c + 0x20, if amount == 1.0 { 4 } else { 1 });
    e.mem.set_u32(bone + 0x1c, c);
}

/// What an actor holds besides the common tree.
#[derive(Clone, Copy, PartialEq)]
enum Extra {
    None,
    Controller,
    Ragdoll,
    NotPending,
    NoBound,
}

/// An actor at offset `x` with its animation and tree.
fn actor(w: &mut World, e: &mut Engine, k: &Kinds, x: f32, extra: Extra, lighting: bool) -> u32 {
    let actor_vt = w.vtable(
        e,
        &[
            (0x100, IS_ACTOR_FN),
            (0x1e4, ANIMATION_FN),
            (0x1d0, NODE_FN),
        ],
    );
    let a = w.alloc(e, 0x160);
    e.mem.set_u32(a, actor_vt);
    e.mem.set_u8(a + 0x14e, u8::from(lighting));
    // The scene node the actors hang from (not walked; its transform is the root's parent's).
    let holder = node(w, e, k.node, 0, [x, 1.0, 0.0], 1);
    identity(e, holder + 0x68, [x, 1.0, 0.0]);
    let root = node(w, e, k.node, holder, [0.0, 0.0, 0.5], 3);
    let accum = node(w, e, k.node, root, [0.0, 0.0, 0.25], 3);
    let mut bone = accum;
    let mut bones = Vec::new();
    for i in 0..5 {
        let b = node(w, e, k.node, bone, [0.1 * i as f32, 0.2, 0.3], 3);
        bones.push(b);
        bone = b;
    }
    geometry(w, e, k, accum, 1.5, extra != Extra::NoBound);
    geometry(w, e, k, bones[2], 0.75, true);
    blend(w, e, k, bones[1], true, 1.0);
    blend(
        w,
        e,
        k,
        bones[3],
        true,
        if extra == Extra::Ragdoll { 0.0 } else { 1.0 },
    );
    blend(w, e, k, bones[4], false, 1.0);
    // The head under the last bone, with a geometry under it.
    let head = node(w, e, k.head, bones[4], [0.0, 0.0, 0.1], 1);
    geometry(w, e, k, head, 0.2, true);
    // The weapon in the "hand" (the third bone): a node with a geometry.
    let weapon = node(w, e, k.node, bones[2], [0.3, 0.0, 0.0], 1);
    geometry(w, e, k, weapon, 0.6, true);
    // Idle Havok controllers, as every actor of the game has them on its bones: a blend controller
    // (its update leaves at once) and a force controller (its update stores a word of its own);
    // the translations run, on the helpers.
    for (bone, update) in [(bones[1], 0x00c9_b090u32), (bones[2], 0x00cb_9770)] {
        let cvt = w.vtable(e, &[(0x94, update)]);
        let c = w.alloc(e, 0x60);
        e.mem.set_u32(c, cvt);
        e.mem.set_u32(c + 4, 5);
        e.mem.set_u32(c + 0x2c, bone);
        e.mem.set_u32(bone + 0xc, c);
    }
    if extra == Extra::Controller {
        let cvt = w.vtable(e, &[(0x94, CONTROLLER_FN)]);
        let c = w.alloc(e, 0x40);
        e.mem.set_u32(c, cvt);
        e.mem.set_u32(c + 4, 5);
        e.mem.set_u32(bones[0] + 0xc, c);
    }
    let animation = w.alloc(e, 0x100);
    e.mem.set_u32(animation + 0x8, root);
    e.mem.set_u32(animation + 0xc, accum);
    e.mem.set_f32(animation + 0xd0, 0.25);
    e.mem
        .set_u8(animation, if extra == Extra::NotPending { 0 } else { 2 });
    e.mem.set_u32(a + 0x10, animation);
    e.mem.set_u32(a + 0x14, root);
    a
}

fn stand_ins(e: &mut Engine) {
    e.register(IS_ACTOR_FN, |e, a| ret(u32::from(e.mem.u32(a[0]) != 0)));
    e.register(ANIMATION_FN, |e, a| ret(e.mem.u32(a[0] + 0x10)));
    e.register(NODE_FN, |e, a| ret(e.mem.u32(a[0] + 0x14)));
    e.register(OTHER_FN, |_, a| {
        record("other", a.to_vec());
        ret(0)
    });
    e.register(BODY_POSITION_FN, |e, a| {
        for (i, v) in [100.0f32, 100.0, 100.0, 0.0].iter().enumerate() {
            e.mem.set_f32(a[1] + 4 * i as u32, *v);
        }
        ret(a[1])
    });
    e.register(CONTROLLER_FN, |_, a| {
        record("controller", a.to_vec());
        ret(0)
    });
    e.register(SETTING_VALUE, |_, _| ret(TASKLET_VALUE));
    e.register(NOT_ACTOR_FN, |_, _| ret(0));
    e.register(SET_BOUND_FN, |e, a| {
        e.mem.set_u32(a[0] + 0x20, a[1]);
        ret(0)
    });
    e.register(MOVE_BODY_FN, |e, a| {
        let words: Vec<u32> = (0..4)
            .map(|k| e.mem.u32(a[1] + 4 * k))
            .chain((0..4).map(|k| e.mem.u32(a[2] + 4 * k)))
            .collect();
        record("move body", [vec![a[0]], words].concat());
        ret(0)
    });
    e.register(POOL_ALLOCATE, |e, _| {
        let n = e.mem.u32(POOL_AREA - 4);
        e.mem.set_u32(POOL_AREA - 4, n + 1);
        record("allocate", vec![n]);
        ret(POOL_AREA + 0x10 * n)
    });
    // The Havok lock's imports: the interlocked compare-exchange and the thread id.
    e.register(0x00fd_f1bc, |e, a| {
        let old = e.mem.u32(a[0]);
        if old == a[2] {
            e.mem.set_u32(a[0], a[1]);
        }
        ret(old)
    });
    e.register(0x00fd_f1d0, |_, _| ret(0x1234));
    e.register(OBJECT_TEST, |_, _| ret(1));
    e.register(APPLY_HARD_KEYFRAME, |e, a| {
        let mut words = vec![a[0]];
        for k in 0..3 {
            words.push(e.mem.u32(a[1] + 4 * k));
        }
        for k in 0..4 {
            words.push(e.mem.u32(a[2] + 4 * k));
        }
        record("hard keyframe", words);
        ret(0)
    });
    e.register(COLLISION_HK_TO_NI, |e, a| {
        let node = e.mem.u32(a[0] + 8);
        record("ragdoll hktoNi", vec![a[0], node]);
        ret(0)
    });
    e.register(HEAD_DOWNWARD, |e, a| {
        let parent = e.mem.u32(a[0] + 0x18);
        let t: Vec<u32> = (0..3).map(|k| e.mem.u32(parent + 0x8c + 4 * k)).collect();
        record("head", [vec![a[0]], t].concat());
        e.call(NODE_DOWNWARD, &args![a[0], a[1], a[2]]);
        ret(0)
    });
    e.register(OBJECT_LIGHTING, |_, a| {
        record("lighting", vec![a[1]]);
        ret(0)
    });
    e.register(INCREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(DECREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    // The scrap array and the task manager (not reached with the game's switch off but for the
    // array itself).
    e.register(0x0097_89e0, |e, a| {
        e.mem.set_u32(a[0] + 8, 0);
        ret(a[0])
    });
    e.register(0x0097_8a80, |_, _| ret(0));
    e.register(0x007c_b2e0, |_, _| ret(0));
    // The exe's data the passes read: the update state's thread block word is the engine's own; the
    // thread setting (2: animations postponed, so the update runs when pending), the always-update
    // byte, the doubles and the next-index table of the quaternion, the Havok scale.
    for (page, len) in [
        (0x0300_0000u32, 0x1000u32),
        (0x011c_3000, 0x1000),
        (0x011c_5000, 0x1000),
        (0x011e_0000, 0x1000),
        (0x011d_5000, 0x1000),
        (0x011d_d000, 0x1000),
        (0x011a_9000, 0x1000),
        (0x0101_1000, 0x2000),
        (0x0120_2000, 0x1000),
        (0x0126_7000, 0x1000),
        (0x0101_7000, 0x1000),
        (0x011f_4000, 0x1000),
        (0x011f_9000, 0x1000),
        (0x011a_b000, 0x1000),
        (0x011a_f000, 0x1000),
        (0x011c_1000, 0x1000),
        (0x0101_6000, 0x1000),
        (0x010c_8000, 0x1000),
    ] {
        e.map(page, len);
    }
    e.mem.set_u64(0x0101_1588, 0.5f64.to_bits());
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    for (i, v) in [1u32, 2, 0].iter().enumerate() {
        e.mem.set_u32(0x011a_9e98 + 4 * i as u32, *v);
    }
    e.mem.set_f32(0x011c_582c, 0.142_857_15);
    // The bound merge's threshold (as the culling tests set it).
    e.mem.set_u32(0x011a_b3c0, 0x3586_37bd);
    // The quaternion normalisation's constants (`005612a0`): 0.5 and 3.0, as in the exe.
    e.mem.set_f32(0x0101_6248, 0.5);
    e.mem.set_f32(0x0101_7718, 3.0);
    // The force controller's starting time left (`010c8ae4`, 1/60 s as the exe holds it).
    e.mem.set_u32(0x010c_8ae4, 0x3c83_126f);
    // The shadow scene node the lighting queue goes to (`00450b80(0)` reads the table).
    e.mem.set_u32(0x011f_91c8, 0x0300_0100);
}

/// The process lists with the actors in levels 0 and 1 (a null slot and a non-actor among them).
fn world(e: &mut Engine) -> World {
    let mut w = World {
        lists: 0,
        blocks: Vec::new(),
        reset: Vec::new(),
        objects: Vec::new(),
    };
    let node_vt = w.vtable(e, &[(0xa4, NODE_DOWNWARD), (0xb8, WORLD_DATA)]);
    let geometry_vt = w.vtable(
        e,
        &[
            (0xa4, OBJECT_DOWNWARD),
            (0xb8, WORLD_DATA),
            (0xbc, GEOMETRY_BOUND),
            (0x78, SET_BOUND_FN),
        ],
    );
    let head_vt = w.vtable(e, &[(0xa4, HEAD_DOWNWARD), (0xb8, WORLD_DATA)]);
    let blend_vt = w.vtable(
        e,
        &[
            (0x90, BLEND_UPDATE),
            (0xa4, BLEND_HK_TO_NI),
            (0xa8, NI_TO_HK),
        ],
    );
    let k = Kinds {
        node: node_vt,
        geometry: geometry_vt,
        head: head_vt,
        blend: blend_vt,
    };
    let non_actor_vt = w.vtable(e, &[(0x100, IS_ACTOR_FN)]);
    let non_actor = w.alloc(e, 0x20);
    e.mem.set_u32(non_actor, non_actor_vt);
    // `IsActor` answers whether the word +0 (the vtable) is set: give the non-actor a vtable whose
    // `IsActor` answers 0.
    let zero_vt = w.vtable(e, &[(0x100, NOT_ACTOR_FN)]);
    e.mem.set_u32(non_actor, zero_vt);
    let mut actors = Vec::new();
    let plan = [
        (Extra::None, true),
        (Extra::None, false),
        (Extra::None, true),
        (Extra::Controller, true),
        (Extra::None, true),
        (Extra::None, false),
        (Extra::Ragdoll, true),
        (Extra::NotPending, true),
        (Extra::None, true),
        (Extra::NoBound, true),
        (Extra::None, true),
        (Extra::None, false),
    ];
    for (i, &(extra, lighting)) in plan.iter().enumerate() {
        let a = actor(&mut w, e, &k, i as f32 * 3.0, extra, lighting);
        actors.push(a);
        if i == 2 {
            actors.push(0);
        }
        if i == 5 {
            actors.push(non_actor);
        }
    }
    // The process array at the lists' +4: level 0 the first half, level 1 the rest, then an actor
    // of level 2 the pass does not walk.
    let split = actors.len() / 2;
    let unwalked = actor(&mut w, e, &k, 99.0, Extra::None, true);
    let elements = w.alloc(e, 4 * (actors.len() as u32 + 1));
    for (i, &a) in actors.iter().enumerate() {
        e.mem.set_u32(elements + 4 * i as u32, a);
    }
    e.mem.set_u32(elements + 4 * actors.len() as u32, unwalked);
    let lists = w.alloc(e, 0x100);
    let array = lists + 4;
    e.mem.set_u32(array + 4, elements);
    let n = actors.len() as u32 + 1;
    e.mem.set_u16(array + 8, n as u16);
    e.mem.set_u16(array + 0xa, n as u16);
    e.mem.set_u16(array + 0xc, n as u16);
    let ends = [split as u32, actors.len() as u32, n, n];
    let heads = [0, split as u32, actors.len() as u32, n];
    for level in 0..4 {
        e.mem
            .set_u32(array + 0x10 + 4 * level, heads[level as usize]);
        e.mem
            .set_u32(array + 0x20 + 4 * level, ends[level as usize]);
    }
    w.lists = lists;
    w.reset = w
        .blocks
        .iter()
        .map(|&(a, s)| (a, e.mem.bytes(a, s)))
        .collect();
    w
}

/// One pass in `mode`: the world put back, the records and the world's bytes after it.
fn pass(e: &mut Engine, w: &World, mode: &str, helpers: usize) -> (Vec<Rec>, Vec<Vec<u8>>) {
    for (at, bytes) in &w.reset {
        e.mem.write(*at, bytes);
    }
    e.mem.set_u32(POOL_AREA - 4, 0);
    RECORD.lock().unwrap().clear();
    bones::set_for_tests(mode, helpers);
    e.call(PASS, &args![w.lists]);
    bones::set_for_tests("off", 1);
    let after = w.blocks.iter().map(|&(a, s)| e.mem.bytes(a, s)).collect();
    (std::mem::take(&mut *RECORD.lock().unwrap()), after)
}

#[test]
fn parallel_bones_end_in_the_serial_state() {
    let mut e = Engine::new();
    *MAIN.lock().unwrap() = Some(std::thread::current().id());
    stand_ins(&mut e);
    let w = world(&mut e);
    let _ = bones::take_counts();
    let (serial, serial_state) = pass(&mut e, &w, "off", 1);
    let tags = |r: &[Rec], tag: &str| r.iter().filter(|x| x.tag == tag).count();
    // The reference holds every kind of outside call the replay must order.
    assert!(tags(&serial, "hard keyframe") >= 20, "{serial:?}");
    assert!(tags(&serial, "head") >= 9, "{serial:?}");
    assert_eq!(tags(&serial, "controller"), 1, "{serial:?}");
    assert_eq!(tags(&serial, "ragdoll hktoNi"), 1, "{serial:?}");
    assert!(tags(&serial, "lighting") >= 8, "{serial:?}");
    assert_eq!(
        tags(&serial, "other"),
        0,
        "no other slot is called: {serial:?}"
    );
    assert!(serial.iter().all(|r| r.main));
    assert_ne!(
        serial_state,
        w.reset.iter().map(|(_, b)| b.clone()).collect::<Vec<_>>()
    );

    for helpers in [3, 2, 1] {
        let (got, state) = pass(&mut e, &w, "on", helpers);
        let (counts, reasons) = bones::take_counts();
        assert!(
            got.iter().all(|r| r.main),
            "{helpers}: everything recorded ran on main"
        );
        assert_eq!(
            got, serial,
            "{helpers} helpers: the outside calls are the serial pass's"
        );
        for (i, (a, b)) in state.iter().zip(&serial_state).enumerate() {
            assert_eq!(
                a, b,
                "{helpers} helpers: block {i} ({:08x}) after the pass",
                w.blocks[i].0
            );
        }
        assert_eq!(counts.frames, 1, "{counts:?}");
        assert!(counts.walked >= 8, "{counts:?}");
        assert_eq!(counts.barriers, 2, "{counts:?} {reasons:?}");
        assert!(counts.subtrees >= 9, "{counts:?}");
        assert!(counts.havok >= 20, "{counts:?}");
        assert_eq!(counts.mismatches, 0, "{counts:?}");
        let names: Vec<&str> = reasons.iter().map(|r| r.0).collect();
        assert_eq!(names, vec!["controllers", "collision"], "{reasons:?}");
    }

    // The check: no difference, every walked actor compared, the serial pass's end state kept.
    let (got, state) = pass(&mut e, &w, "check", 3);
    let (counts, _) = bones::take_counts();
    assert_eq!(counts.differences, 0, "{counts:?}");
    assert!(counts.checked_actors >= 8, "{counts:?}");
    assert_eq!(
        got, serial,
        "check: the outside calls are the serial pass's"
    );
    assert_eq!(state, serial_state, "check: the serial pass's end state");
}
