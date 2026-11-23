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
//! stand-in that records and goes on with the `NiNode` pass, so the helpers log it for main). Two
//! animated actors, each at its own animation time, carry controllers as the game loads them (the
//! scaled-time flag, `ComputeScaledTime`): transform and visibility controllers whose interpolators
//! read one shared transform data (Euler rotation keys, whose search hints each interpolation
//! writes) and bool data, and a transform controller without an interpolator. Then
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
/// An interpolator update of a class a helper does not know (recorded).
const ODD_INTERPOLATOR_FN: u32 = VT_FN + 0x24;
/// The controllers and interpolators a helper may run: `NiTransformController::Update`,
/// `NiVisController::Update`, `NiTransformInterpolator::Update`, `NiBoolInterpolator::Update`.
const TRANSFORM_CONTROLLER_UPDATE: u32 = 0x00a3_6680;
const VIS_CONTROLLER_UPDATE: u32 = 0x00a4_4e10;
const TRANSFORM_INTERPOLATOR_UPDATE: u32 = 0x00a3_fdb0;
const BOOL_INTERPOLATOR_UPDATE: u32 = 0x00a5_33c0;
/// What the game's controller vtables hold at slot 0xa4 (`NiTimeController::ComputeScaledTime`, in
/// `NiTransformController`'s `0109687c` and `NiVisController`'s `01097e8c`) and its interpolators'
/// at slot 0xd4 (answering 0: not always updated; `NiTransformInterpolator`'s `01097314`,
/// `NiBoolInterpolator`'s `0109a0b4`).
const COMPUTE_SCALED_TIME: u32 = 0x00a6_cd60;
const ALWAYS_UPDATE: u32 = 0x00ee_8990;
/// The key interpolations the game's start-up puts in its tables by key type: linear float keys
/// (`011f3c90` + 4: `00a2a560`), Euler rotation keys (`011f3cc0` + 0x10: `00a29b10`) and step bool
/// keys (`011f3d08` + 0x14: `00a2cc40`).
const KEY_TABLES: [(u32, u32); 3] = [
    (0x011f_3c94, 0x00a2_a560),
    (0x011f_3cd0, 0x00a2_9b10),
    (0x011f_3d1c, 0x00a2_cc40),
];
/// `MemoryManager::Allocate` (the bound pool's block for an object without a world bound): the next
/// block of a fixed area, the same each pass.
const POOL_ALLOCATE: u32 = 0x00aa_3e40;
/// What a node's bound allocation (`005467e0`, the culling family's) calls: `operator new` and the
/// bound block's set-up (`00546890`).
const OPERATOR_NEW: u32 = 0x0040_1000;
const BOUND_BLOCK_SETUP: u32 = 0x0054_6890;
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
    /// The transform data and bool data the animated actors share (`animation_data`).
    animation: Option<(u32, u32)>,
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

/// A node vtable with the bound setter (slot 0x78) a bound allocation (`005467e0`) calls.
fn bound_node_vt(w: &mut World, e: &mut Engine) -> u32 {
    w.vtable(
        e,
        &[
            (0xa4, NODE_DOWNWARD),
            (0xb8, WORLD_DATA),
            (0x78, SET_BOUND_FN),
        ],
    )
}

/// A node without a world bound, with the bound-from-children flag (0x80000) every node of a
/// loaded model has, and room for `children`: it takes a bound only when a child ends its pass
/// with one.
fn bare_node(
    w: &mut World,
    e: &mut Engine,
    vt: u32,
    parent: u32,
    local: [f32; 3],
    children: u32,
) -> u32 {
    let n = node(w, e, vt, parent, local, children);
    e.mem.set_u32(n + 0x20, 0);
    e.mem.set_u32(n + 0x30, 0x0008_0000);
    n
}

/// A geometry under `parent` whose skin instance (+0xbc) has `bones` (each with a model bound in
/// the skin data), the skin's root parent the geometry's parent; its bound flag set, so its world
/// bound is the skin's each pass.
fn skinned(w: &mut World, e: &mut Engine, k: &Kinds, parent: u32, bones: &[u32]) -> u32 {
    let g = geometry(w, e, k, parent, 1.0, true);
    e.mem.set_u32(g + 0x30, 0x0008_0000);
    let count = bones.len() as u32;
    let list = w.alloc(e, 4 * count);
    for (i, &b) in bones.iter().enumerate() {
        e.mem.set_u32(list + 4 * i as u32, b);
    }
    let data = w.alloc(e, 0x48);
    identity(e, data + 0xc, [0.0, 0.0, 0.0]);
    let bone_data = w.alloc(e, 0x4c * count);
    for i in 0..count {
        let bound = bone_data + 0x4c * i + 0x34;
        for (axis, v) in [0.1f32, 0.2 * i as f32, 0.05].iter().enumerate() {
            e.mem.set_f32(bound + 4 * axis as u32, *v);
        }
        e.mem.set_f32(bound + 0xc, 0.3 + 0.1 * i as f32);
    }
    e.mem.set_u32(data + 0x40, bone_data);
    e.mem.set_u32(data + 0x44, count);
    let skin = w.alloc(e, 0x20);
    e.mem.set_u32(skin + 8, data);
    e.mem.set_u32(skin + 0x10, parent);
    e.mem.set_u32(skin + 0x14, list);
    e.mem.set_u32(g + 0xbc, skin);
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
    /// A skeleton of nodes without a world bound that never take one (as a creature's bones), and a
    /// skin on them walked after it.
    Skinned,
    /// A node without a world bound that takes one from its children (a geometry with a bound under
    /// it): its pass allocates the bound, on main.
    NodeNoBound,
    /// A skin inside the head (a deferred subtree) reading a node walked after the head (the weapon):
    /// the replay would read it in another state than the serial pass.
    LateSkin,
    /// A walked skin (under the root, after the skeleton) reading the head, which is deferred: the
    /// helper would read it before main's replay moves it.
    SkinOnHead,
    /// A node without a bound flagged 0x80000|0x2000 (it takes one at once).
    BoundNow,
    /// A head without a bound (over a bounded geometry) under a bone without a bound.
    HeadNoBound,
    /// A skinned geometry without a bound reading the weapon, walked after it.
    SkinNoBound,
    /// A skin reading another listed actor's bone (set by `world`).
    SkinOther,
    /// A transform controller (a pose interpolator) on a bone and a visibility controller (a bool
    /// interpolator) on the weapon, active, each driving its own node: walked on a helper.
    Animated,
    /// A transform controller whose interpolator is of a class a helper does not know.
    OddInterpolator,
    /// A transform controller on a bone whose target `world` points at another actor's bone.
    ForeignTarget,
}

/// A controller as the game has one after loading it (`NiTimeController::LoadBinary`, `00a6d5e0`,
/// sets the scaled-time flag 0x40; `Start` the active flag 8 and the last time `-FLT_MAX`): flags
/// 0x4c (cycle type 2, clamped: a looped one's `fmod` needs the runtime library's tables, which
/// this world does not have; both take the same scaled-time path and cache), frequency 1, phase 0,
/// keys from 0 to 2 s, the start, last and scaled times unset, `ComputeScaledTime` at slot 0xa4;
/// `update` at slot 0x94.
fn loaded_controller(
    w: &mut World,
    e: &mut Engine,
    update: u32,
    target: u32,
    interpolator: u32,
) -> u32 {
    let vt = w.vtable(e, &[(0x94, update), (0xa4, COMPUTE_SCALED_TIME)]);
    let c = w.alloc(e, 0x38);
    e.mem.set_u32(c, vt);
    e.mem.set_u32(c + 4, 5);
    e.mem.set_u16(c + 8, 0x4c);
    e.mem.set_f32(c + 0xc, 1.0);
    e.mem.set_f32(c + 0x18, 2.0);
    for at in [0x1c, 0x20, 0x28] {
        e.mem.set_f32(c + at, f32::MIN);
    }
    e.mem.set_u32(c + 0x2c, target);
    e.mem.set_u32(c + 0x34, interpolator);
    c
}

/// Linear float keys (type 1, 8 bytes: time, value).
fn float_keys(w: &mut World, e: &mut Engine, keys: &[(f32, f32)]) -> u32 {
    let at = w.alloc(e, 8 * keys.len() as u32);
    for (i, (time, value)) in keys.iter().enumerate() {
        e.mem.set_f32(at + 8 * i as u32, *time);
        e.mem.set_f32(at + 8 * i as u32 + 4, *value);
    }
    at
}

/// The `NiTransformData` and `NiBoolData` every animated actor's interpolators share, as clones
/// of one model share their data: one Euler rotation key (type 4, 0x48 bytes: X three linear keys,
/// Y two, Z none; each axis's search hint at +0x3c, written by every interpolation), two linear
/// scale keys, no translation keys; three step bool keys (type 5: false from 0 s, true from 0.5 s).
fn animation_data(w: &mut World, e: &mut Engine) -> (u32, u32) {
    if let Some(d) = w.animation {
        return d;
    }
    let x = float_keys(w, e, &[(0.0, 0.0), (0.5, 0.4), (1.5, -0.2)]);
    let y = float_keys(w, e, &[(0.0, 0.1), (2.0, 0.9)]);
    let euler = w.alloc(e, 0x48);
    e.mem.set_f32(euler + 4, 1.0);
    for (axis, (count, keys)) in [(3u32, x), (2, y), (0, 0)].iter().enumerate() {
        let axis = axis as u32;
        e.mem.set_u32(euler + 0x14 + 4 * axis, *count);
        if *count != 0 {
            e.mem.set_u32(euler + 0x20 + 4 * axis, 1);
            e.mem.set_u8(euler + 0x2c + axis, 8);
            e.mem.set_u32(euler + 0x30 + 4 * axis, *keys);
        }
    }
    let scale = float_keys(w, e, &[(0.0, 1.0), (2.0, 1.5)]);
    let data = w.alloc(e, 0x30);
    e.mem.set_u32(data + 4, 2);
    e.mem.set_u16(data + 8, 1);
    e.mem.set_u16(data + 0xc, 2);
    e.mem.set_u32(data + 0x10, 4);
    e.mem.set_u32(data + 0x18, 1);
    e.mem.set_u8(data + 0x1c, 0x48);
    e.mem.set_u8(data + 0x1e, 8);
    e.mem.set_u32(data + 0x20, euler);
    e.mem.set_u32(data + 0x28, scale);
    let bool_keys = w.alloc(e, 8 * 3);
    for (i, (time, value)) in [(0.0f32, 0u8), (0.5, 1), (2.0, 1)].iter().enumerate() {
        e.mem.set_f32(bool_keys + 8 * i as u32, *time);
        e.mem.set_u8(bool_keys + 8 * i as u32 + 4, *value);
    }
    let bool_data = w.alloc(e, 0x18);
    e.mem.set_u32(bool_data + 4, 2);
    e.mem.set_u32(bool_data + 8, 3);
    e.mem.set_u32(bool_data + 0xc, bool_keys);
    e.mem.set_u32(bool_data + 0x10, 5);
    e.mem.set_u8(bool_data + 0x14, 8);
    w.animation = Some((data, bool_data));
    (data, bool_data)
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
    geometry(
        w,
        e,
        k,
        accum,
        1.5,
        !matches!(extra, Extra::NoBound | Extra::HeadNoBound),
    );
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
    let head = node(w, e, k.head, bones[4], [0.0, 0.0, 0.1], 2);
    geometry(w, e, k, head, 0.2, true);
    // The weapon in the "hand" (the third bone): a node with a geometry.
    let weapon = node(w, e, k.node, bones[2], [0.3, 0.0, 0.0], 1);
    geometry(w, e, k, weapon, 0.6, true);
    if extra == Extra::LateSkin {
        skinned(w, e, k, head, &[weapon]);
    }
    if extra == Extra::SkinOnHead {
        skinned(w, e, k, root, &[head]);
    }
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
    if extra == Extra::Skinned {
        // A skeleton under the accumulation root whose nodes have no world bound but the
        // bound-from-children flag, as a creature's bones (no bounded descendant, so they never
        // take one): a chain of three bones and a sibling branch two deep. Then a skinned geometry
        // under the root, walked after the skeleton: its bound reads the bones of this pass.
        let skeleton = bare_node(w, e, k.node, accum, [0.0, 0.1, 0.0], 2);
        let mut bone = skeleton;
        let mut skin_bones = Vec::new();
        for i in 0..3 {
            bone = bare_node(w, e, k.node, bone, [0.2, 0.1 * i as f32, 0.05], 1);
            skin_bones.push(bone);
        }
        let twig = bare_node(w, e, k.node, skeleton, [0.0, 0.3, 0.0], 1);
        bare_node(w, e, k.node, twig, [0.0, 0.1, 0.1], 1);
        skinned(w, e, k, root, &skin_bones);
    }
    if extra == Extra::NodeNoBound {
        // Two nested nodes without a bound that take theirs from their children, a bounded geometry
        // under the inner one: both children loops would allocate a bound (`005467e0`), so the
        // actor is a `bound` barrier and the allocations run on main in the serial order
        // (`order_check` sees the bounded geometry two levels down).
        let vt = bound_node_vt(w, e);
        let outer = bare_node(w, e, vt, accum, [0.0, -0.1, 0.0], 1);
        let inner = bare_node(w, e, vt, outer, [0.0, 0.0, 0.1], 1);
        geometry(w, e, k, inner, 0.4, true);
    }
    if extra == Extra::BoundNow {
        // A node without a bound flagged to take one without looking at its children (0x80000 and
        // 0x2000), with only a bound-less child: its children loop allocates a bound anyway
        // (`005467e0`): a `bound` barrier, the allocation on main.
        let vt = bound_node_vt(w, e);
        let n = bare_node(w, e, vt, accum, [0.0, -0.2, 0.0], 1);
        e.mem.set_u32(n + 0x30, 0x0008_2000);
        bare_node(w, e, k.node, n, [0.0, 0.0, 0.1], 1);
    }
    if extra == Extra::HeadNoBound {
        // A second head, built without a bound, with a bounded geometry under it, under a bone
        // without a bound: the head's children loop allocates its bound, so the bone's does too
        // (right after the head, in the serial order): a `bound` barrier. A geometry without a
        // bound walked after them (the one under the accumulation root) allocates later still, so
        // a late bone allocation would come out of order.
        let vt = bound_node_vt(w, e);
        let bone = bare_node(w, e, vt, bones[4], [0.0, 0.05, 0.0], 1);
        let head_vt = w.vtable(
            e,
            &[
                (0xa4, HEAD_DOWNWARD),
                (0xb8, WORLD_DATA),
                (0x78, SET_BOUND_FN),
            ],
        );
        let head = bare_node(w, e, head_vt, bone, [0.0, 0.0, 0.05], 1);
        geometry(w, e, k, head, 0.3, true);
    }
    if extra == Extra::SkinNoBound {
        // A skinned geometry without a world bound (deferred: its pass allocates one) under the
        // fourth bone, reading the weapon, which is walked after it: main's replay would read the
        // weapon's new transform where the serial pass reads last frame's: a `skin` barrier.
        let g = skinned(w, e, k, bones[3], &[weapon]);
        e.mem.set_u32(g + 0x20, 0);
    }
    if matches!(extra, Extra::Animated | Extra::OddInterpolator) {
        // The transform controller on the fourth bone, its interpolator keyed by the shared
        // transform data (`animation_data`): its update interpolates the rotation (Euler keys) and
        // the scale at the controller's scaled time and copies the value out (translation +0xc,
        // rotation +0x18, scale +0x28), which the controller writes into the bone's local
        // transform.
        let odd = extra == Extra::OddInterpolator;
        let (data, bool_data) = animation_data(w, e);
        let ivt = w.vtable(
            e,
            &[
                (
                    0x8c,
                    if odd {
                        ODD_INTERPOLATOR_FN
                    } else {
                        TRANSFORM_INTERPOLATOR_UPDATE
                    },
                ),
                (0xd4, ALWAYS_UPDATE),
            ],
        );
        let interpolator = w.alloc(e, 0x48);
        e.mem.set_u32(interpolator, ivt);
        e.mem.set_u32(interpolator + 4, 1);
        e.mem.set_f32(interpolator + 8, -1.0);
        for (i, v) in [0.3f32, 0.1, 0.2, 1.0, 0.0, 0.0, 0.0, 1.25]
            .iter()
            .enumerate()
        {
            e.mem.set_f32(interpolator + 0xc + 4 * i as u32, *v);
        }
        e.mem.set_u32(interpolator + 0x2c, data);
        let c = loaded_controller(w, e, TRANSFORM_CONTROLLER_UPDATE, bones[3], interpolator);
        e.mem.set_u32(bones[3] + 0xc, c);
        if !odd {
            // The visibility controller on the weapon, its interpolator keyed by the shared bool
            // data (hidden before 0.5 s, shown after).
            let bvt = w.vtable(
                e,
                &[(0xa0, BOOL_INTERPOLATOR_UPDATE), (0xd4, ALWAYS_UPDATE)],
            );
            let b = w.alloc(e, 0x18);
            e.mem.set_u32(b, bvt);
            e.mem.set_u32(b + 4, 1);
            e.mem.set_f32(b + 8, -1.0);
            e.mem.set_u8(b + 0xc, 2);
            e.mem.set_u32(b + 0x10, bool_data);
            let v = loaded_controller(w, e, VIS_CONTROLLER_UPDATE, weapon, b);
            e.mem.set_u32(weapon + 0xc, v);
            // The census's commonest: a transform controller without an interpolator on the
            // first bone (its update only keeps its own times).
            let c = loaded_controller(w, e, TRANSFORM_CONTROLLER_UPDATE, bones[0], 0);
            e.mem.set_u32(bones[0] + 0xc, c);
        }
    }
    if extra == Extra::ForeignTarget {
        // An active transform controller without interpolator on the fourth bone; `world` points
        // its target at the previous actor's first bone: a `controllers` barrier.
        let c = loaded_controller(w, e, TRANSFORM_CONTROLLER_UPDATE, bones[3], 0);
        e.mem.set_u32(bones[3] + 0xc, c);
    }
    if extra == Extra::SkinOther {
        // A skin under the root whose bone the world points at another actor's (`world`): a `skin`
        // barrier (a helper would read it while another helper walks that actor).
        skinned(w, e, k, root, &[accum]);
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
    // Each actor its own time (the controllers' values differ by actor).
    e.mem.set_f32(animation + 0xd0, 0.25 + x / 64.0);
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
    e.register(ODD_INTERPOLATOR_FN, |_, a| {
        record("odd interpolator", a.to_vec());
        ret(0)
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
    // A node's bound allocation (`005467e0`): `operator new` gives the next block of the same pool,
    // recorded (it must run on main); the block's set-up answers the block.
    e.register(OPERATOR_NEW, |e, _| {
        let n = e.mem.u32(POOL_AREA - 4);
        e.mem.set_u32(POOL_AREA - 4, n + 1);
        record("allocate", vec![n]);
        ret(POOL_AREA + 0x10 * n)
    });
    e.register(BOUND_BLOCK_SETUP, |_, a| ret(a[0]));
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
        // The controllers' constants: the unset translation (011a8400), rotation (011f3704) and
        // scale (01096b8c), -FLT_MAX in the exe, and the double they are compared with (010241b0).
        (0x0109_6000, 0x1000),
        (0x011a_8000, 0x1000),
        (0x011f_3000, 0x1000),
        (0x0102_4000, 0x1000),
        // The unset bool (0109a914: 2 in the exe) the visibility controller starts its value with.
        (0x0109_a000, 0x1000),
        // The double FLT_MAX (010231b0) `ComputeScaledTime` tests the low key time against.
        (0x0102_3000, 0x1000),
    ] {
        e.map(page, len);
    }
    e.mem.set_u64(0x0102_31b0, f64::from(f32::MAX).to_bits());
    for (at, f) in KEY_TABLES {
        e.mem.set_u32(at, f);
    }
    for (at, words) in [(0x011a_8400u32, 3u32), (0x011f_3704, 4), (0x0109_6b8c, 1)] {
        for i in 0..words {
            e.mem.set_u32(at + 4 * i, f32::MIN.to_bits());
        }
    }
    e.mem.set_u64(0x0102_41b0, f64::from(f32::MIN).to_bits());
    e.mem.set_u8(0x0109_a914, 2);
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
        animation: None,
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
        (Extra::Skinned, true),
        (Extra::None, true),
        (Extra::Animated, true),
        (Extra::None, true),
        // The same model's transform data again: the stretch ends before it.
        (Extra::Animated, true),
        (Extra::None, true),
        (Extra::OddInterpolator, true),
        (Extra::Controller, true),
        (Extra::None, true),
        (Extra::None, false),
        (Extra::Ragdoll, true),
        (Extra::NotPending, true),
        (Extra::None, true),
        (Extra::NoBound, true),
        (Extra::NodeNoBound, true),
        (Extra::LateSkin, true),
        (Extra::None, true),
        (Extra::SkinOnHead, true),
        (Extra::None, true),
        (Extra::None, false),
        (Extra::BoundNow, true),
        (Extra::HeadNoBound, true),
        // Next to the head without a bound: were that actor walked, they would share a stretch.
        (Extra::None, true),
        (Extra::SkinNoBound, true),
        (Extra::None, true),
        (Extra::None, true),
        (Extra::SkinOther, true),
        (Extra::None, true),
        (Extra::None, false),
        (Extra::ForeignTarget, true),
        (Extra::None, true),
        (Extra::None, true),
    ];
    let mut made = Vec::new();
    for (i, &(extra, lighting)) in plan.iter().enumerate() {
        let a = actor(&mut w, e, &k, i as f32 * 3.0, extra, lighting);
        actors.push(a);
        made.push(a);
        if i == 2 {
            actors.push(0);
        }
        if i == 5 {
            actors.push(non_actor);
        }
    }
    // The skin of the actor reading another's bone: its bone is the first bone of the actor listed
    // just before it (walked in the same stretch were it not a barrier).
    let skin_other = plan
        .iter()
        .position(|p| p.0 == Extra::SkinOther)
        .expect("planned");
    let first_bone = |e: &Engine, a: u32| {
        let animation = e.mem.u32(a + 0x10);
        let accum = e.mem.u32(animation + 0xc);
        e.mem.u32(e.mem.u32(accum + 0xa0))
    };
    let other_bone = first_bone(&*e, made[skin_other - 1]);
    let root = e.mem.u32(e.mem.u32(made[skin_other] + 0x10) + 8);
    let skinned_geometry = e.mem.u32(e.mem.u32(root + 0xa0) + 4);
    let skin = e.mem.u32(skinned_geometry + 0xbc);
    e.mem.set_u32(e.mem.u32(skin + 0x14), other_bone);
    // The foreign controller's target: the previous actor's first bone (bones[3] is three first
    // children below the accumulation root's first child).
    let foreign = plan
        .iter()
        .position(|p| p.0 == Extra::ForeignTarget)
        .expect("planned");
    let mut bone = first_bone(&*e, made[foreign]);
    for _ in 0..3 {
        bone = e.mem.u32(e.mem.u32(bone + 0xa0));
    }
    let controller = e.mem.u32(bone + 0xc);
    e.mem
        .set_u32(controller + 0x2c, first_bone(&*e, made[foreign - 1]));
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
    // The animated actors' controllers took the game's path: the shared Euler key's X hint is the
    // second animated actor's (its scaled time is past the second key), Y's at the first key.
    let (data, _) = w.animation.expect("animated actors");
    let euler = e.mem.u32(data + 0x20);
    assert_eq!([e.mem.u32(euler + 0x3c), e.mem.u32(euler + 0x40)], [1, 0]);
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
        // Exact: twenty actors walked (the skinned one among them: its flagged bones without
        // bounds are walked, not deferred; the two animated ones: their transform and visibility
        // controllers run on the helpers, in two stretches, as they share their Euler keys; the
        // two alone between two barriers run serially), eleven whole on main (three controllers:
        // an unknown controller, an unknown interpolator, one driving another actor's bone; a
        // ragdoll; three nodes that allocate their bound; four skins read out of order),
        // twenty-one subtrees deferred (twenty heads and the geometry without a bound).
        assert_eq!(counts.walked, 20, "{counts:?}");
        assert_eq!(counts.barriers, 11, "{counts:?} {reasons:?}");
        assert_eq!(counts.subtrees, 21, "{counts:?}");
        assert_eq!(counts.splits, 1, "{counts:?}");
        let count_of = |name: &str| reasons.iter().find(|r| r.0 == name).map_or(0, |r| r.1);
        assert_eq!(count_of("bound"), 3, "{reasons:?}");
        assert_eq!(count_of("skin"), 4, "{reasons:?}");
        assert_eq!(count_of("controllers"), 3, "{reasons:?}");
        assert!(counts.havok >= 20, "{counts:?}");
        assert_eq!(counts.mismatches, 0, "{counts:?}");
        let names: Vec<&str> = reasons.iter().map(|r| r.0).collect();
        assert_eq!(
            names,
            vec!["controllers", "collision", "bound", "skin"],
            "{reasons:?}"
        );
    }

    // The check: no difference, every walked actor compared, the serial pass's end state kept.
    let (got, state) = pass(&mut e, &w, "check", 3);
    let (counts, _) = bones::take_counts();
    let end_state = state == serial_state;
    assert_eq!(
        counts.differences, 0,
        "{counts:?}; the serial pass's end state kept: {end_state}"
    );
    assert_eq!(counts.checked_actors, 20, "{counts:?}");
    assert_eq!(counts.splits, 1, "{counts:?}");
    assert_eq!(
        got, serial,
        "check: the outside calls are the serial pass's"
    );
    assert_eq!(state, serial_state, "check: the serial pass's end state");
}
