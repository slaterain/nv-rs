//! `NV_PAR_RENDER` (the culling walk split over helper engines, `engine_typed::culling::par`): the
//! registrations and the accumulator calls a chunked walk replays on main are the serial walk's, in
//! the same order and with the same words, however the helpers finish.
//!
//! The tree, in guest memory, in the style of the typed culling test (`typed_culling.rs`): a
//! `BSCullingProcess` (the typed `Process` and `AppendVirtual` in its slots, six planes, the box
//! `|x|, |y|, |z| <= 10`), a root with a node of 40 children and a node of 20, under them geometries
//! (inside, outside and on the planes), empty slots, plain nodes, objects whose `OnVisible` is not
//! audited for other cores (a stand-in that records), and fade nodes running the translated
//! `BSFadeNode::OnVisible` (some step their fade in, some group their alphas with the accumulator),
//! and multi-bound nodes (the typed `BSMultiBoundNode::OnVisible`) whose culling mode differs from
//! the process's: all visible (1), planes only (3) and ask the shapes (4), nested in each other, with
//! shapes whose verdict is kept, asked for or outside; one of them is the root's child, so a node
//! under it splits while main's process is in that node's mode.
//!
//! What is recorded, on the host, with the thread it ran on: the accumulator's `RegisterObject`
//! (with the main process's mode and active planes at that moment), the unaudited `OnVisible`
//! (object, process, mode, planes), and the fade node's three accumulator-side calls (start and stop
//! grouping alphas, the fade alpha down a subtree). The serial walk (switch off) is the reference;
//! each chunked walk (several helper counts, so several chunkings, two split sizes, the check and
//! serial modes) must record the same list, all of it on main.
//!
//! Out-of-order completion is forced: the most-alpha callback sleeps 100 ms on a helper for the
//! fade node that is the big node's first child, so the helper walking the first chunk finishes
//! after the others have walked the last one (a sequence number taken on the helpers shows it).
//!
//! One test: the helper pool is one per process, made from the first engine that forks.

use engine as _;
use engine_core::prelude::*;
use engine_typed::culling;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
use std::sync::Mutex;
use std::time::Duration;

/// Where the test's virtual functions are registered: past `.text`, one per vtable slot.
const VT_FN: u32 = 0x00ff_2000;
/// The most-alpha callback the fade nodes call (`pMaxAlphaAccessFunc`).
const MOST_ALPHA_FN: u32 = 0x00ff_3000;
/// A multi-bound shape's `WithinFrustum` stand-in (slot 0x9c): the answer is the shape's byte +0x24.
const WITHIN_FRUSTUM_FN: u32 = 0x00ff_3004;

const POOL_ALLOCATE: u32 = 0x00aa_3e40;
const SCRAP_HEAP: u32 = 0x00aa_42e0;
const SCRAP_ALLOCATE: u32 = 0x00aa_54a0;
const SCRAP_FREE: u32 = 0x00aa_5610;
const SKIN_BOUND: u32 = 0x00a8_6530;
const INCREMENT: u32 = 0x00fd_f1c0;
const DECREMENT: u32 = 0x00fd_f1c4;
const GETPTD_NOEXIT: u32 = 0x00ed_1165;
const HEAP: u32 = 0x0300_2000;

const PROCESS: u32 = 0x00c4_ee90;
const APPEND_VIRTUAL: u32 = 0x00c4_f1d0;
const NODE_ON_VISIBLE: u32 = 0x00a5_dbe0;
const GEOMETRY_ON_VISIBLE: u32 = 0x00a7_fd90;
const FADE_NODE_ON_VISIBLE: u32 = 0x00b4_e490;
const MULTI_BOUND_NODE_ON_VISIBLE: u32 = 0x00c4_6f60;
/// `NiAVObject::Cull`: the walk's entry.
const CULL: u32 = 0x00a5_9e00;

/// The fade node's accumulator-side calls (logged on the helpers, replayed on main).
const START_GROUPING_ALPHAS: u32 = 0x00b6_3500;
const STOP_GROUPING_ALPHAS: u32 = 0x00b6_3090;
const SET_FADE_ALPHA: u32 = 0x00b6_bb30;
/// The fade node's accumulator class check (an `IsKindOf` walk of its RTTI): yes.
const ACCUMULATOR_CHECK: u32 = 0x0043_b300;

/// The fade node's globals (as in its own unit tests, `bsfadenode.rs`).
const FADE_ENABLED: u32 = 0x011a_d7b4;
const MULTIPLIERS: u32 = 0x011a_d7b8;
const FADE_IN_TIME: u32 = 0x011a_d7e4;
const FADE_OUT_TIME: u32 = 0x011a_d7e8;
const FADE_IN_THRESHOLD: u32 = 0x011a_d7ec;
const FADE_OUT_THRESHOLD: u32 = 0x011a_d7f0;
const DISTANCE_MULTIPLIER: u32 = 0x011a_d7f4;
const MAX_FADE_INCREMENT: u32 = 0x011a_d7f8;
const FRAME_TIME: u32 = 0x011f_63a0;
const FADE_COUNTER: u32 = 0x011f_9138;
const MAX_ALPHA_ACCESS: u32 = 0x011f_9134;
const UNSCALED_DISTANCE: u32 = 0x0101_df44;
const SNAP_RADIUS: u32 = 0x0101_786c;
const CAPPED_FAR: u32 = 0x010a_90a4;
const CAPPED_NEAR: u32 = 0x010a_90a0;
const SCENE_GLOBAL: u32 = 0x011f_4748;
const CAMERA_LOD_ADJUST: u32 = 0x110;

/// A fade node's size and its fade fields.
const FADE_NODE_SIZE: u32 = 0xe4;
const NEAR: u32 = 0xac;
const FAR: u32 = 0xb0;
const LAST_FADE: u32 = 0xb4;
const CURRENT_FADE: u32 = 0xb8;
const BOUND_RADIUS: u32 = 0xbc;
const FRAME_COUNTER: u32 = 0xc8;
/// `m_uFlags`' faded-in bit.
const FADED_IN: u32 = 0x4000;

/// A multi-bound node's size, its multi-bound and its culling mode; a shape's size here, its kept
/// verdict (0 none yet, 1 inside, 2 outside) and the stand-in's answer.
const MULTI_BOUND_NODE_SIZE: u32 = 0xb4;
const MULTI_BOUND: u32 = 0xac;
const CULLING_MODE: u32 = 0xb0;
const SHAPE_SIZE: u32 = 0x28;
const SHAPE_VERDICT: u32 = 0x8;
const SHAPE_ANSWER: u32 = 0x24;
const INSIDE: u32 = 1;
const OUTSIDE: u32 = 2;
/// The process's culling modes the multi-bound nodes set.
const ALL_VISIBLE: u32 = 1;
const PLANES_ONLY: u32 = 3;
const ASK_SHAPES: u32 = 4;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const MINUS_TEN: u32 = 0xc120_0000;
const ROTATION: [u32; 9] = [
    0x3f7e_8a6e,
    0xbdcc_6c67,
    0,
    0x3dcc_6c67,
    0x3f7e_8a6e,
    0,
    0,
    0,
    ONE,
];

/// What a recorded call was.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Register,
    Unaudited,
    StartGrouping,
    StopGrouping,
    FadeAlpha,
    Other(u32),
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Rec {
    tag: Tag,
    words: Vec<u32>,
    /// Ran on the test's own thread (main), not a render helper.
    main: bool,
}

static RECORD: Mutex<Vec<Rec>> = Mutex::new(Vec::new());
/// The main process (its mode and planes are read at each registration).
static MAIN_PROCESS: AtomicU32 = AtomicU32::new(0);
/// The fade nodes whose most alpha is 0.5 (they group their alphas); the rest answer 1.
static GROUPING: Mutex<Vec<u32>> = Mutex::new(Vec::new());
/// The first chunk's slow fade node and the last chunk's, and the sequence numbers taken on the
/// helpers when the slow one's sleep ended and when the last one ran (`u32::MAX`: not on a helper).
static SLOW: AtomicU32 = AtomicU32::new(0);
static LAST: AtomicU32 = AtomicU32::new(0);
static SEQUENCE: AtomicU32 = AtomicU32::new(0);
static SLOW_DONE: AtomicU32 = AtomicU32::new(u32::MAX);
static LAST_SEEN: AtomicU32 = AtomicU32::new(u32::MAX);

fn on_helper() -> bool {
    std::thread::current()
        .name()
        .is_some_and(|n| n.starts_with("render helper"))
}

fn record(tag: Tag, words: Vec<u32>) {
    RECORD.lock().unwrap().push(Rec {
        tag,
        words,
        main: !on_helper(),
    });
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The process's mode (+0x90) and active planes (+0x8c).
fn process_state(e: &Engine, process: u32) -> [u32; 2] {
    [e.mem.u32(process + 0x90), e.mem.u32(process + 0x8c)]
}

fn stand_ins(e: &mut Engine) {
    // An unaudited OnVisible: recorded with the process it got and that process's state.
    e.register(VT_FN + 0xd4, |e, a| {
        let [mode, planes] = process_state(e, a[1]);
        record(Tag::Unaudited, vec![a[0], a[1], mode, planes]);
        ret(0)
    });
    // The accumulator's RegisterObject: recorded with the main process's state; its word at
    // +0x10 says whether it accepts.
    e.register(VT_FN + 0x98, |e, a| {
        let [mode, planes] = process_state(e, MAIN_PROCESS.load(SeqCst));
        record(Tag::Register, vec![a[0], a[1], mode, planes]);
        ret(e.mem.u32(a[0] + 0x10))
    });
    fn other<const SLOT: u32>(_: &mut Engine, a: &[u32]) -> Ret {
        record(Tag::Other(SLOT), a.to_vec());
        ret(0)
    }
    e.register(VT_FN + 0x44, other::<0x44>);
    e.register(VT_FN + 0x4c, other::<0x4c>);
    e.register(VT_FN + 0x90, other::<0x90>);
    e.register(VT_FN + 0x8c, other::<0x8c>);
    e.register(VT_FN + 0x04, other::<0x04>);
    e.register(VT_FN + 0x9c, other::<0x9c>);
    e.register(VT_FN + 0x78, |e, a| {
        e.mem.set_u32(a[0] + 0x20, a[1]);
        ret(0)
    });
    e.register(VT_FN + 0x10, |e, a| ret(e.mem.u32(a[0] + 0x10)));
    e.register(POOL_ALLOCATE, |e, a| ret(e.mem.alloc(a[1])));
    e.register(SCRAP_HEAP, |_, _| ret(HEAP));
    e.register(SCRAP_ALLOCATE, |e, a| ret(e.mem.alloc(a[1].max(4))));
    e.register(SCRAP_FREE, |_, a| {
        record(Tag::Other(SCRAP_FREE), a.to_vec());
        ret(0)
    });
    e.register(SKIN_BOUND, |_, a| {
        record(Tag::Other(SKIN_BOUND), a.to_vec());
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
    e.register(GETPTD_NOEXIT, |_, _| ret(0));
    e.map(0x011c_1000, 0x1000);
    e.map(0x0101_1000, 0x2000);
    e.mem.set_u64(0x0101_1588, 0.5f64.to_bits());
    e.mem.set_u64(0x0101_2060, 0);
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    e.map(0x011a_b000, 0x1000);
    e.mem.set_u32(0x011a_b3c0, 0x3586_37bd);
    e.map(0x011a_9000, 0x1000);
    for (i, w) in [ONE, 0, 0, 0, ONE, 0, 0, 0, ONE].iter().enumerate() {
        e.mem.set_u32(0x011a_946c + 4 * i as u32, *w);
    }
    e.map(0x010a_2000, 0x1000);
    e.mem.set_u32(0x010a_2720, 0x10);
    e.map(0x011f_4000, 0x3000);
    e.map(0x0101_f000, 0x1000);
    e.map(0x0109_c000, 0x1000);

    // The fade node's side.
    e.register(START_GROUPING_ALPHAS, |_, a| {
        record(Tag::StartGrouping, a[..3].to_vec());
        ret(0)
    });
    e.register(STOP_GROUPING_ALPHAS, |_, a| {
        record(Tag::StopGrouping, a[..1].to_vec());
        ret(0)
    });
    e.register(SET_FADE_ALPHA, |_, a| {
        record(Tag::FadeAlpha, a[..2].to_vec());
        ret(0)
    });
    e.register(ACCUMULATOR_CHECK, |_, _| ret(1));
    e.register(WITHIN_FRUSTUM_FN, |e, a| {
        ret(u32::from(e.mem.u8(a[0] + SHAPE_ANSWER)))
    });
    // The most alpha: 0.5 for the grouping nodes, 1 for the others. On a helper the first chunk's
    // fade node sleeps first, and both marked nodes take a sequence number.
    e.register(MOST_ALPHA_FN, |_, a| {
        let node = a[0];
        if on_helper() {
            if node == SLOW.load(SeqCst) {
                std::thread::sleep(Duration::from_millis(100));
                SLOW_DONE.store(SEQUENCE.fetch_add(1, SeqCst), SeqCst);
            } else if node == LAST.load(SeqCst) {
                LAST_SEEN.store(SEQUENCE.fetch_add(1, SeqCst), SeqCst);
            }
        }
        let most = if GROUPING.lock().unwrap().contains(&node) {
            0.5
        } else {
            1.0
        };
        Ret {
            st0: most,
            ..Ret::default()
        }
    });
    for (addr, len) in [
        (0x011a_d000, 0x1000),
        (0x011f_9000, 0x1000),
        (0x0101_d000, 0x1000),
        (0x0101_7000, 0x1000),
        (0x010a_9000, 0x1000),
        (0x0101_6000, 0x1000),
        (0x0109_b000, 0x1000),
    ] {
        e.map(addr, len);
    }
    e.mem.set_u8(FADE_ENABLED, 1);
    for i in 0..11u32 {
        e.mem.set_u32(MULTIPLIERS + 4 * i, ONE);
    }
    e.mem.set_u32(FADE_IN_TIME, TWO);
    e.mem.set_u32(FADE_OUT_TIME, TWO);
    e.mem.set_u32(FADE_IN_THRESHOLD, 0.7f32.to_bits());
    e.mem.set_u32(FADE_OUT_THRESHOLD, 0.3f32.to_bits());
    e.mem.set_u32(DISTANCE_MULTIPLIER, ONE);
    e.mem.set_u32(MAX_FADE_INCREMENT, 0.1f32.to_bits());
    e.mem.set_u32(FRAME_TIME, HALF);
    e.mem.set_u32(FADE_COUNTER, 100);
    e.mem.set_u32(UNSCALED_DISTANCE, 0.2f32.to_bits());
    e.mem.set_u32(SNAP_RADIUS, 600.0f32.to_bits());
    e.mem.set_u32(CAPPED_FAR, 2600.0f32.to_bits());
    e.mem.set_u32(CAPPED_NEAR, 1300.0f32.to_bits());
    e.mem.set_u32(MAX_ALPHA_ACCESS, MOST_ALPHA_FN);
}

fn vtable(e: &mut Engine, real: &[(u32, u32)]) -> u32 {
    let vt = e.mem.alloc(0x100);
    for slot in (0..0x100).step_by(4) {
        let target = real
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or(VT_FN + slot, |(_, a)| *a);
        e.mem.set_u32(vt + slot, target);
    }
    vt
}

fn transform_at(e: &mut Engine, at: u32, t: [u32; 3], s: u32) {
    for (i, w) in ROTATION.iter().enumerate() {
        e.mem.set_u32(at + 4 * i as u32, *w);
    }
    for (i, w) in t.iter().enumerate() {
        e.mem.set_u32(at + 0x24 + 4 * i as u32, *w);
    }
    e.mem.set_u32(at + 0x30, s);
}

/// A bound: centre and radius as floats.
fn bound(e: &mut Engine, x: f32, y: f32, z: f32, r: f32) -> u32 {
    let b = e.mem.alloc(0x10);
    for (i, v) in [x, y, z, r].iter().enumerate() {
        e.mem.set_u32(b + 4 * i as u32, v.to_bits());
    }
    b
}

/// A small bound of kind `k`: inside every plane, on one of four planes, or outside.
fn small_bound(e: &mut Engine, k: usize) -> u32 {
    let (x, y, z) = [
        (0.0, 0.0, 0.0),
        (9.5, 0.0, 0.0),
        (-9.5, 1.0, 0.0),
        (0.0, 9.5, 0.0),
        (1.0, 0.0, -9.5),
        (30.0, 0.0, 0.0),
    ][k % 6];
    bound(e, x, y, z, 1.0)
}

fn object(e: &mut Engine, size: u32, vt: u32, bound: u32, flags: u32) -> u32 {
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    e.mem.set_u32(o + 4, 1);
    e.mem.set_u32(o + 0x20, bound);
    e.mem.set_u32(o + 0x30, flags);
    transform_at(e, o + 0x34, [ONE, TWO, HALF], ONE);
    transform_at(e, o + 0x68, [TWO, ONE, MINUS_TEN], TWO);
    o
}

struct Kinds {
    node_vt: u32,
    geometry_vt: u32,
    fade_vt: u32,
    multi_bound_vt: u32,
    shape_vt: u32,
    stand_in_vt: u32,
    data: u32,
}

impl Kinds {
    fn geometry(&self, e: &mut Engine, k: usize) -> u32 {
        let b = small_bound(e, k);
        let g = object(e, 0xc8, self.geometry_vt, b, 0);
        e.mem.set_u32(g + 0xb8, self.data);
        g
    }

    fn unaudited(&self, e: &mut Engine, k: usize) -> u32 {
        let b = small_bound(e, k);
        object(e, 0xc8, self.stand_in_vt, b, 0)
    }

    /// A node (`size` bytes, `vt`) over `children` (0: an empty slot).
    fn node(&self, e: &mut Engine, size: u32, vt: u32, radius: f32, children: &[u32]) -> u32 {
        let b = bound(e, 0.0, 0.0, 0.0, radius);
        let n = object(e, size, vt, b, 0);
        let count = u32::try_from(children.len()).unwrap();
        let slots = e.mem.alloc(4 * count.max(1));
        for (i, &c) in children.iter().enumerate() {
            e.mem.set_u32(slots + 4 * i as u32, c);
            if c != 0 {
                e.mem.set_u32(c + 0x18, n);
            }
        }
        let used = children.iter().filter(|c| **c != 0).count();
        e.mem.set_u32(n + 0x9c, self.stand_in_vt);
        e.mem.set_u32(n + 0xa0, slots);
        e.mem.set_u16(n + 0xa4, count as u16);
        e.mem.set_u16(n + 0xa6, count as u16);
        e.mem.set_u16(n + 0xa8, used as u16);
        n
    }

    /// A fade node: faded in and grouping its alphas (most alpha 0.5), or stepping its fade in.
    fn fade(&self, e: &mut Engine, grouping: bool, children: &[u32], fades: &mut Vec<u32>) -> u32 {
        let n = self.node(e, FADE_NODE_SIZE, self.fade_vt, 12.0, children);
        e.mem.set_u32(n + NEAR, 1000.0f32.to_bits());
        e.mem.set_u32(n + FAR, 2000.0f32.to_bits());
        e.mem.set_u32(n + BOUND_RADIUS, 10.0f32.to_bits());
        e.mem.set_u32(n + FRAME_COUNTER, 100);
        if grouping {
            e.mem.set_u32(n + 0x30, FADED_IN);
            e.mem.set_u32(n + CURRENT_FADE, ONE);
            e.mem.set_u32(n + LAST_FADE, ONE);
            GROUPING.lock().unwrap().push(n);
        }
        fades.push(n);
        n
    }

    /// A multi-bound with a shape whose kept verdict is `verdict` and whose `WithinFrustum`
    /// answers `answer`.
    fn multi_bound(
        &self,
        e: &mut Engine,
        verdict: u32,
        answer: bool,
        shapes: &mut Vec<u32>,
    ) -> u32 {
        let shape = e.mem.alloc(SHAPE_SIZE);
        e.mem.set_u32(shape, self.shape_vt);
        e.mem.set_u32(shape + 4, 1);
        e.mem.set_u32(shape + SHAPE_VERDICT, verdict);
        e.mem.set_u32(shape + SHAPE_ANSWER, u32::from(answer));
        shapes.push(shape);
        let multi_bound = e.mem.alloc(0x10);
        e.mem.set_u32(multi_bound + 4, 1);
        e.mem.set_u32(multi_bound + 0xc, shape);
        multi_bound
    }

    /// A multi-bound node that culls its children in `mode`, with `multi_bound` (0: none).
    fn multi_bound_node(
        &self,
        e: &mut Engine,
        mode: u32,
        multi_bound: u32,
        children: &[u32],
    ) -> u32 {
        let n = self.node(
            e,
            MULTI_BOUND_NODE_SIZE,
            self.multi_bound_vt,
            12.0,
            children,
        );
        e.mem.set_u32(n + MULTI_BOUND, multi_bound);
        e.mem.set_u32(n + CULLING_MODE, mode);
        n
    }
}

struct Tree {
    root: u32,
    process: u32,
    /// Every fade node and multi-bound shape with its bytes as built (put back before each walk: a
    /// walk steps fades and keeps shapes' verdicts).
    reset: Vec<(u32, Vec<u8>)>,
    /// The shapes with no verdict at first that a walk asks once: one answers inside, one outside.
    asked_inside: u32,
    asked_outside: u32,
}

fn tree(e: &mut Engine) -> Tree {
    stand_ins(e);
    let stand_in_vt = vtable(e, &[]);
    let kinds = Kinds {
        node_vt: vtable(e, &[(0xd4, NODE_ON_VISIBLE)]),
        geometry_vt: vtable(e, &[(0xd4, GEOMETRY_ON_VISIBLE)]),
        fade_vt: vtable(e, &[(0xd4, FADE_NODE_ON_VISIBLE)]),
        multi_bound_vt: vtable(e, &[(0xd4, MULTI_BOUND_NODE_ON_VISIBLE)]),
        shape_vt: vtable(e, &[(0x9c, WITHIN_FRUSTUM_FN)]),
        stand_in_vt,
        data: e.mem.alloc(0x40),
    };
    for (i, w) in [ONE, TWO, MINUS_TEN, HALF].iter().enumerate() {
        e.mem.set_u32(kinds.data + 0x10 + 4 * i as u32, *w);
    }
    let process_vt = vtable(e, &[(0x44, PROCESS), (0x4c, APPEND_VIRTUAL)]);
    let accumulator = e.mem.alloc(0x20);
    e.mem.set_u32(accumulator, stand_in_vt);
    e.mem.set_u32(accumulator + 4, 1);
    e.mem.set_u32(accumulator + 0x10, 1);
    // The scene the fade node takes its accumulator from.
    let scene = e.mem.alloc(0x10);
    e.mem.set_u32(scene + 8, accumulator);
    e.mem.set_u32(SCENE_GLOBAL, scene);
    // The camera at the objects' world translation, LOD adjust 1.
    let camera = object(e, 0x120, stand_in_vt, 0, 0);
    e.mem.set_u32(camera + CAMERA_LOD_ADJUST, ONE);
    let process = e.mem.alloc(0xc8);
    e.mem.set_u32(process, process_vt);
    e.mem.set_u32(process + 0xc, camera);
    let normals: [[f32; 3]; 6] = [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    for (i, n) in normals.iter().enumerate() {
        let p = process + 0x2c + 0x10 * i as u32;
        for (j, v) in n.iter().enumerate() {
            e.mem.set_u32(p + 4 * j as u32, v.to_bits());
        }
        e.mem.set_u32(p + 0xc, MINUS_TEN);
    }
    e.mem.set_u32(process + 0x8c, 0x3f);
    e.mem.set_u32(process + 0xc4, accumulator);
    MAIN_PROCESS.store(process, SeqCst);

    let mut fades = Vec::new();
    let mut shapes = Vec::new();
    let (mut asked_inside, mut asked_outside) = (0, 0);
    let mut big = Vec::new();
    for i in 0..40usize {
        let child = if i == 12 {
            // All visible, its shape's verdict kept from before (inside; asked, it would say
            // outside): the outside geometry registers; a fade node; and a planes-only node in it.
            let inner = [
                kinds.geometry(e, 1),
                kinds.geometry(e, 5),
                kinds.unaudited(e, 3),
            ];
            let inner = kinds.multi_bound_node(e, PLANES_ONLY, 0, &inner);
            let faded = [kinds.geometry(e, 1), kinds.unaudited(e, 2)];
            let kids = [
                kinds.geometry(e, 5),
                kinds.unaudited(e, 5),
                kinds.fade(e, true, &faded, &mut fades),
                inner,
                kinds.geometry(e, 0),
            ];
            let bound = kinds.multi_bound(e, INSIDE, false, &mut shapes);
            kinds.multi_bound_node(e, ALL_VISIBLE, bound, &kids)
        } else if i == 26 {
            // Planes only, its shape asked (inside, kept): an all-visible node in it changes
            // nothing (planes only stays), so its outside geometry is culled.
            let inner = [kinds.geometry(e, 5), kinds.unaudited(e, 1)];
            let inner = kinds.multi_bound_node(e, ALL_VISIBLE, 0, &inner);
            let faded = [kinds.geometry(e, 3)];
            let kids = [
                kinds.geometry(e, 2),
                kinds.unaudited(e, 4),
                kinds.geometry(e, 5),
                inner,
                kinds.fade(e, false, &faded, &mut fades),
            ];
            let bound = kinds.multi_bound(e, 0, true, &mut shapes);
            asked_inside = *shapes.last().unwrap();
            kinds.multi_bound_node(e, PLANES_ONLY, bound, &kids)
        } else if i == 33 {
            // Its shape asked: outside (kept), so nothing under it is reached.
            let kids = [kinds.geometry(e, 0), kinds.unaudited(e, 0)];
            let bound = kinds.multi_bound(e, 0, false, &mut shapes);
            asked_outside = *shapes.last().unwrap();
            kinds.multi_bound_node(e, ALL_VISIBLE, bound, &kids)
        } else if i == 0 || i == 39 {
            let kids = [
                kinds.geometry(e, 0),
                kinds.unaudited(e, 1),
                kinds.geometry(e, 3),
                0,
                kinds.unaudited(e, 4),
            ];
            let f = kinds.fade(e, true, &kids, &mut fades);
            let mark = if i == 0 { &SLOW } else { &LAST };
            mark.store(f, SeqCst);
            f
        } else {
            match i % 7 {
                0 => kinds.geometry(e, i),
                1 => kinds.unaudited(e, i),
                2 => kinds.geometry(e, 5),
                3 => {
                    let kids = [kinds.geometry(e, i), kinds.unaudited(e, i + 1)];
                    kinds.fade(e, false, &kids, &mut fades)
                }
                4 => {
                    let kids: Vec<u32> = (0..18)
                        .map(|k| {
                            if k % 3 == 2 {
                                kinds.unaudited(e, k)
                            } else {
                                kinds.geometry(e, k)
                            }
                        })
                        .collect();
                    let radius = if i % 2 == 0 { 2.0 } else { 12.0 };
                    kinds.node(e, 0xac, kinds.node_vt, radius, &kids)
                }
                5 => 0,
                _ => {
                    let kids = [kinds.geometry(e, i + 2), kinds.geometry(e, 1)];
                    kinds.fade(e, true, &kids, &mut fades)
                }
            }
        };
        big.push(child);
    }
    let big = kinds.node(e, 0xac, kinds.node_vt, 15.0, &big);
    // The node of 20 ends on an all-visible multi-bound node whose last entry registers: the split's
    // last replayed entry has mode 1 and no active planes, and the root's geometry after it (`last`)
    // and the multi-bound node after that see main's own mode and planes only if the replay put
    // them back.
    let second: Vec<u32> = (0..20)
        .map(|k| match k % 4 {
            _ if k == 19 => {
                let kids = [kinds.unaudited(e, 1), kinds.geometry(e, 0)];
                kinds.multi_bound_node(e, ALL_VISIBLE, 0, &kids)
            }
            0 | 2 => kinds.geometry(e, k),
            1 => kinds.unaudited(e, k),
            _ => {
                let kids = [kinds.geometry(e, k), kinds.unaudited(e, k + 2)];
                kinds.fade(e, k % 8 == 3, &kids, &mut fades)
            }
        })
        .collect();
    let second = kinds.node(e, 0xac, kinds.node_vt, 12.0, &second);
    let last = kinds.geometry(e, 0);
    // The root's own multi-bound child, in the ask-the-shapes mode: the node of 24 under it splits
    // on main while main's process is in that mode; in it all-visible nodes whose shapes are asked
    // at every walk (inside for some, outside for others).
    let third: Vec<u32> = (0..24)
        .map(|k| match k % 6 {
            0 | 2 => kinds.geometry(e, k),
            1 => kinds.unaudited(e, k),
            3 => {
                let kids = [kinds.geometry(e, 5), kinds.unaudited(e, 5)];
                let bound = kinds.multi_bound(e, 0, k % 12 == 3, &mut shapes);
                kinds.multi_bound_node(e, ALL_VISIBLE, bound, &kids)
            }
            4 => {
                let kids = [kinds.geometry(e, k), kinds.unaudited(e, k + 1)];
                kinds.fade(e, k % 12 == 4, &kids, &mut fades)
            }
            _ => 0,
        })
        .collect();
    let third = kinds.node(e, 0xac, kinds.node_vt, 12.0, &third);
    let asking = kinds.multi_bound_node(e, ASK_SHAPES, 0, &[third]);
    let root = kinds.node(e, 0xac, kinds.node_vt, 20.0, &[big, second, last, asking]);
    let mut reset: Vec<(u32, Vec<u8>)> = fades
        .into_iter()
        .map(|f| (f, e.mem.bytes(f, FADE_NODE_SIZE)))
        .collect();
    reset.extend(shapes.into_iter().map(|s| (s, e.mem.bytes(s, SHAPE_SIZE))));
    Tree {
        root,
        process,
        reset,
        asked_inside,
        asked_outside,
    }
}

/// One walk of the tree with the switch as given: what it recorded.
fn walk(e: &mut Engine, t: &Tree, mode: &str, helpers: usize, min_children: u32) -> Vec<Rec> {
    for (at, bytes) in &t.reset {
        e.mem.write(*at, bytes);
    }
    RECORD.lock().unwrap().clear();
    SLOW_DONE.store(u32::MAX, SeqCst);
    LAST_SEEN.store(u32::MAX, SeqCst);
    culling::par_set_for_tests(mode, helpers, min_children);
    let before = e.mem.bytes(t.process, 0xc8);
    e.call(CULL, &args![t.root, t.process]);
    assert_eq!(
        e.mem.bytes(t.process, 0xc8),
        before,
        "{mode}: the walk leaves the process as it found it"
    );
    // A shape asked once keeps its answer, wherever the walk asked it.
    assert_eq!(e.mem.u32(t.asked_inside + SHAPE_VERDICT), INSIDE, "{mode}");
    assert_eq!(
        e.mem.u32(t.asked_outside + SHAPE_VERDICT),
        OUTSIDE,
        "{mode}"
    );
    culling::par_set_for_tests("off", 1, 16);
    std::mem::take(&mut *RECORD.lock().unwrap())
}

#[test]
fn chunked_walks_replay_the_serial_order() {
    let mut e = Engine::new();
    let t = tree(&mut e);
    let serial = walk(&mut e, &t, "off", 1, 16);
    let count = |tag: Tag| serial.iter().filter(|r| r.tag == tag).count();
    // The reference has every kind of entry the replay must order.
    assert!(count(Tag::Register) >= 40, "registrations: {serial:?}");
    assert!(count(Tag::Unaudited) >= 15);
    assert!(count(Tag::StartGrouping) >= 5);
    assert_eq!(count(Tag::StartGrouping), count(Tag::StopGrouping));
    assert!(count(Tag::FadeAlpha) > count(Tag::StartGrouping));
    assert!(serial.iter().all(|r| r.main));
    assert!(
        !serial.iter().any(|r| matches!(r.tag, Tag::Other(_))),
        "no other stand-in is called: {serial:?}"
    );
    // The active planes the entries carry differ (the replay must put each back).
    let mut planes: Vec<u32> = serial
        .iter()
        .filter(|r| matches!(r.tag, Tag::Register | Tag::Unaudited))
        .map(|r| r.words[3])
        .collect();
    planes.sort_unstable();
    planes.dedup();
    assert!(planes.len() >= 3, "planes seen: {planes:x?}");
    // So do the culling modes: the process's own (0) and each multi-bound node's (the replay must
    // set each entry's, and put main's back: the node of 20's last entry is in mode 1, and what the
    // root walks after it is in mode 0). Each mode is seen by registrations and by unaudited objects
    // alike.
    for tag in [Tag::Register, Tag::Unaudited] {
        let mut modes: Vec<u32> = serial
            .iter()
            .filter(|r| r.tag == tag)
            .map(|r| r.words[2])
            .collect();
        modes.sort_unstable();
        modes.dedup();
        assert_eq!(
            modes,
            vec![0, ALL_VISIBLE, PLANES_ONLY, ASK_SHAPES],
            "{tag:?}: modes seen"
        );
    }
    // The node of 20's last registration (mode 1, no active planes) is followed by the root's
    // geometry `last` (mode 0, main's own).
    assert!(
        serial.windows(2).any(|w| w[0].tag == Tag::Register
            && w[1].tag == Tag::Register
            && w[0].words[2] == ALL_VISIBLE
            && w[1].words[2] == 0),
        "a split ends on an entry in another mode than main's"
    );

    // The first fork makes the pool with 4 helpers; later ones use 1 to 4 of them, so the big
    // node's 40 children are cut into 4, 8, 10 or 14 chunks. Three nodes split at 16+ children (the
    // 40, the 20, and the 24 under the root's multi-bound node), one at 30+.
    for (helpers, min_children) in [(4, 16), (3, 16), (2, 16), (1, 16), (3, 30), (2, 2)] {
        let (walks, _, _) = culling::par_counts_for_tests();
        let started = std::time::Instant::now();
        let got = walk(&mut e, &t, "on", helpers, min_children);
        let (walks_now, _, _) = culling::par_counts_for_tests();
        let splits = if min_children <= 20 { 3 } else { 1 };
        assert_eq!(
            walks_now - walks,
            splits,
            "{helpers} helpers, nodes of {min_children}+: splits walked on the helpers"
        );
        assert!(
            got.iter().all(|r| r.main),
            "{helpers} helpers: everything recorded ran on main"
        );
        assert_eq!(
            got, serial,
            "{helpers} helpers, nodes of {min_children}+: the replay is the serial walk"
        );
        let (slow, last) = (SLOW_DONE.load(SeqCst), LAST_SEEN.load(SeqCst));
        assert!(
            slow != u32::MAX && last != u32::MAX,
            "{helpers} helpers: both marked fade nodes ran on helpers"
        );
        if helpers >= 2 {
            assert!(
                last < slow,
                "{helpers} helpers: the last chunk was walked before the first one finished"
            );
        }
        eprintln!(
            "{helpers} helpers, nodes of {min_children}+: {} entries, same as serial, {:?}",
            got.len(),
            started.elapsed()
        );
    }

    // No split: a node must have this many children.
    let (walks, _, _) = culling::par_counts_for_tests();
    assert_eq!(walk(&mut e, &t, "on", 3, 41), serial);
    assert_eq!(culling::par_counts_for_tests().0, walks);

    // The serial mode (the stream gate's off side): helpers handed their copies, the walk on main.
    let got = walk(&mut e, &t, "serial", 3, 16);
    assert_eq!(got, serial);
    assert_eq!(culling::par_counts_for_tests().0, walks);

    // The check mode: the chunked logs against one helper's walk of the same children.
    let (_, compared, differed) = culling::par_counts_for_tests();
    let got = walk(&mut e, &t, "check", 3, 16);
    let (_, compared_now, differed_now) = culling::par_counts_for_tests();
    assert_eq!(
        compared_now - compared,
        3,
        "the check mode compared every split"
    );
    assert_eq!(differed_now, differed, "the check mode found no difference");
    assert_eq!(differed_now, 0);
    assert_eq!(got, serial);

    // And the reference again: the tree reset makes it repeat.
    assert_eq!(walk(&mut e, &t, "off", 1, 16), serial);
}
