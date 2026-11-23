//! The parallel task manager itself on real game threads (`NV_PAR_TASKS`; the review of #961): the game's
//! constructor (`00c44980`) makes the manager's threads, the game's worker loop runs them
//! (`BSLinearTaskThread::ThreadProc` `008c7720`: its `StartProcessing` and `FinishedProcessing` semaphores, the
//! manager's task through `RunLinearTasks`), the game's split run (`00c44c90`) hands the items out, and the game's
//! destructor body (`00c44b70`) stops and deletes the threads. The items are the game's own item functions over built
//! scenes:
//! - the moved nodes' update (`0087cf50`: each node's `UpdateTransformAndBounds`, world transforms from the parent's
//!   and bounds merged from the children's), over a list as `0087d030` leaves it: a parent of more than ten children
//!   listed by each child, one of those children listed again on its own, a listed node inside another listed node's
//!   subtree, and nodes listed alone;
//! - the actors' Havok sync (`0087cfb0`: its gates the game's, the sync itself, `bhkNiCollisionObject::Synchronize`
//!   `00c6c3d0`, a recording stand-in: the Havok side is not the subject here).
//!
//! With four and with eight threads every node's world transform, every bound but those of the nodes two items update
//! at once (there within the last bits: each item resets the bound and merges into it in place, and the exe's two
//! runners race the same way), and the actors synchronized (each once, with 2), are those of the exe's one thread;
//! more than two runners took items. No game files.
use engine as _;
use engine_bs::units::bsmain::bsparalleltaskmanager::set_threads_for_tests;
use engine_core::{args, exe::Import, real_threads, Engine, Ret};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::Mutex;

/// The manager's constructor, split run and destructor body.
const MANAGER_CONSTRUCT: u32 = 0x00c4_4980;
const RUN_SPLIT: u32 = 0x00c4_4c90;
const MANAGER_DESTROY: u32 = 0x00c4_4b70;
/// The parallel-array task descriptor's maker and the two item functions.
const DESCRIPTOR: u32 = 0x0087_cf10;
const NODE_ITEM: u32 = 0x0087_cf50;
const ACTOR_ITEM: u32 = 0x0087_cfb0;
/// `bhkNiCollisionObject::Synchronize` (a stand-in that records).
const SYNCHRONIZE: u32 = 0x00c6_c3d0;
/// `BSParallelTaskThread`'s vtable (the exe's data: its scalar deleting destructor, the linear task thread's
/// `ThreadProc`, its `OnStartup`, `OnShutdown` and `RunLinearTasks`), the thread name's format and the processor
/// table's word.
const THREAD_VTABLE: u32 = 0x010c_1af0;
const THREAD_VTABLE_SLOTS: [u32; 5] = [
    0x00c4_4df0,
    0x008c_7720,
    0x00c4_48f0,
    0x00c4_4940,
    0x00c4_4960,
];
const NAME_FORMAT: u32 = 0x010c_1ad4;
const PROCESSORS: u32 = 0x011a_f714;
/// Stand-ins: `MemoryManager::Allocate` / `Deallocate`, `sprintf_s`, `DoThreadSetup` / `DoThreadShutdown` (Havok's
/// thread memory is not the subject).
const MEMORY_ALLOCATE: u32 = 0x00aa_3e40;
const MEMORY_FREE: u32 = 0x00aa_4060;
const SPRINTF_S: u32 = 0x0040_6d00;
const THREAD_SETUP: u32 = 0x00c8_0130;
const THREAD_SHUTDOWN: u32 = 0x00c8_0240;
/// The Havok memory system's word and its free-list vtable; its thread memory records (32 at +0xc6c, 0x138 apart,
/// in use when the byte at +0x12c is set), which the constructor counts for its cap.
const HAVOK_MEMORY_SYSTEM: u32 = 0x0126_8418;
const FREE_LIST_VTABLE: u32 = 0x010c_ad74;
/// `Interface::InDialog`'s player word (0: not in a dialogue), read by the actors' gate `00933840`.
const DIALOGUE_PLAYER: u32 = 0x011d_8a80;
/// The node vtable's slots: `UpdateTransformAndBounds` (the item's call), `UpdateWorldData`, the bound setter.
const UPDATE_TRANSFORM_AND_BOUNDS: u32 = 0x00a5_e0b0;
const UPDATE_WORLD_DATA: u32 = 0x00a6_8c60;
/// A recording stand-in for every other slot of the test's vtables.
const OTHER_FN: u32 = 0x00ff_2000;
/// The test's wrapper item function: notes the runner, then calls the item function the work names.
const WRAPPER: u32 = 0x00ff_2004;
const ONE: u32 = 0x3f80_0000;

thread_local! {
    /// This OS thread's runner number (1 up), given at its first item.
    static RUNNER_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}
static NEXT_RUNNER: AtomicU32 = AtomicU32::new(1);
/// The runners that took items in the current run, the item function the wrapper calls, the syncs recorded and
/// the other slots called.
static RUNNERS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static ITEM_FN: AtomicU32 = AtomicU32::new(0);
static SYNCS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
static OTHERS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn runner_id() -> u32 {
    RUNNER_ID.with(|r| {
        if r.get() == 0 {
            r.set(NEXT_RUNNER.fetch_add(1, Relaxed));
        }
        r.get()
    })
}

/// The wrapper item function (cdecl(data, item)).
fn wrapper(e: &mut Engine, a: &[u32]) -> Ret {
    let id = runner_id();
    if let Ok(mut r) = RUNNERS.lock() {
        if !r.contains(&id) {
            r.push(id);
        }
    }
    let f = ITEM_FN.load(Relaxed);
    e.call(f, &args![a[0], a[1]]);
    Ret::default()
}

/// Every block the scene allocates, to compare after the runs.
#[derive(Default)]
struct Scene {
    blocks: Vec<(u32, u32)>,
    nodes: Vec<u32>,
    vt: u32,
}

impl Scene {
    fn alloc(&mut self, e: &mut Engine, size: u32) -> u32 {
        let a = e.mem.alloc(size);
        for k in (0..size).step_by(4) {
            e.mem.set_u32(a + k, 0);
        }
        self.blocks.push((a, size));
        a
    }

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

    /// A node under `parent` (0: a root) with a local translation and rotation about z by `turn` radians, its own
    /// world bound (`center`, `radius`), room for `children`, and its flags: a node with children updates its world
    /// data (bit 2) and merges its bound from its children's (bit 19); a leaf updates its world data and keeps its
    /// bound.
    fn node(
        &mut self,
        e: &mut Engine,
        parent: u32,
        local: [f32; 3],
        turn: f32,
        children: u32,
        bound: ([f32; 3], f32),
    ) -> u32 {
        let n = self.alloc(e, 0xac);
        e.mem.set_u32(n, self.vt);
        e.mem.set_u32(n + 4, 1);
        e.mem.set_u32(n + 0x18, parent);
        let b = self.alloc(e, 0x10);
        for (i, c) in bound.0.iter().enumerate() {
            e.mem.set_f32(b + 4 * i as u32, *c);
        }
        e.mem.set_f32(b + 0xc, bound.1);
        e.mem.set_u32(n + 0x20, b);
        e.mem.set_u32(
            n + 0x30,
            if children > 0 {
                0x0008_0004
            } else {
                0x0000_0004
            },
        );
        let (s, c) = turn.sin_cos();
        let rotation = [c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0];
        for (i, r) in rotation.iter().enumerate() {
            e.mem.set_f32(n + 0x34 + 4 * i as u32, *r);
        }
        for (i, t) in local.iter().enumerate() {
            e.mem.set_f32(n + 0x58 + 4 * i as u32, *t);
        }
        e.mem.set_u32(n + 0x64, ONE);
        // The world transform: identity until the update (a root keeps it).
        for (i, w) in [ONE, 0, 0, 0, ONE, 0, 0, 0, ONE].iter().enumerate() {
            e.mem.set_u32(n + 0x68 + 4 * i as u32, *w);
        }
        e.mem.set_u32(n + 0x98, ONE);
        let array = self.alloc(e, 4 * children.max(1));
        e.mem.set_u32(n + 0xa0, array);
        e.mem.set_u16(n + 0xa4, children as u16);
        if parent != 0 {
            let count = e.mem.u16(parent + 0xa6);
            let base = e.mem.u32(parent + 0xa0);
            e.mem.set_u32(base + 4 * u32::from(count), n);
            e.mem.set_u16(parent + 0xa6, count + 1);
        }
        self.nodes.push(n);
        n
    }

    /// A node with `kids` children, each with two leaves under it, all at offsets made from `seed`.
    fn branch(&mut self, e: &mut Engine, parent: u32, seed: u32, kids: u32) -> u32 {
        let f = |k: u32| (seed.wrapping_mul(2654435761).wrapping_add(k * 97) % 1000) as f32 / 100.0;
        let n = self.node(
            e,
            parent,
            [f(1), f(2), f(3)],
            f(4) / 10.0,
            kids,
            ([0.0; 3], 0.0),
        );
        for k in 0..kids {
            let child = self.node(
                e,
                n,
                [f(5 + k), -f(6 + k), f(7 + k) / 2.0],
                f(8 + k) / 10.0,
                2,
                ([0.0; 3], 0.0),
            );
            for l in 0..2 {
                self.node(
                    e,
                    child,
                    [f(9 + k + l), f(10 + l), -f(11 + k)],
                    0.0,
                    0,
                    ([f(12 + l), f(13 + k), f(14)], 0.5 + f(15 + k + l) / 10.0),
                );
            }
        }
        n
    }
}

/// A word array as the game keeps its lists (`006a7ad0`: data at +4, the count at +8) holding `words`.
fn word_array(s: &mut Scene, e: &mut Engine, words: &[u32]) -> u32 {
    let data = s.alloc(e, 4 * words.len().max(1) as u32);
    for (i, w) in words.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, *w);
    }
    let array = s.alloc(e, 0x10);
    e.mem.set_u32(array + 4, data);
    e.mem.set_u32(array + 8, words.len() as u32);
    e.mem.set_u32(array + 0xc, words.len() as u32);
    array
}

/// The engine of one run, with the imports the manager's threads use and the exe data they read.
fn engine() -> Engine {
    let mut e = Engine::new();
    let slots = [
        (0x00fd_f054, "InitializeCriticalSection"),
        (0x00fd_f058, "DeleteCriticalSection"),
        (0x00fd_f05c, "EnterCriticalSection"),
        (0x00fd_f100, "LeaveCriticalSection"),
        (0x00fd_f07c, "CloseHandle"),
        (0x00fd_f0b4, "CreateSemaphoreA"),
        (0x00fd_f104, "Sleep"),
        (0x00fd_f11c, "SetThreadPriority"),
        (0x00fd_f120, "GetCurrentThread"),
        (0x00fd_f124, "CreateThread"),
        (0x00fd_f158, "ResumeThread"),
        (0x00fd_f1b4, "ReleaseSemaphore"),
        (0x00fd_f1c0, "InterlockedIncrement"),
        (0x00fd_f1c4, "InterlockedDecrement"),
        (0x00fd_f1c8, "WaitForSingleObject"),
        (0x00fd_f1d0, "GetCurrentThreadId"),
    ];
    let imports: Vec<Import> = slots
        .iter()
        .map(|&(slot, name)| Import {
            slot,
            dll: "KERNEL32.dll".into(),
            name: name.into(),
        })
        .collect();
    e.map(0x00fd_f000, 0x1000);
    assert_eq!(e.register_imports(&imports), slots.len());
    // The exe's data the code reads: the thread vtable and name format, the processor table, the dialogue word,
    // Havok's memory system word; the pages the node update's maths reads (as `par_bones` maps them).
    for (page, len) in [
        (0x010c_1000u32, 0x1000u32),
        (0x011a_f000, 0x1000),
        (0x011d_8000, 0x1000),
        (0x0126_8000, 0x1000),
        (0x011a_b000, 0x1000),
        (0x011f_4000, 0x1000),
        (0x011a_9000, 0x1000),
        (0x0101_6000, 0x1000),
        (0x0101_7000, 0x1000),
        (0x0101_1000, 0x2000),
        (0x011c_3000, 0x1000),
        (0x011c_5000, 0x1000),
        (0x011c_1000, 0x1000),
        (0x011e_0000, 0x1000),
        (0x011d_5000, 0x1000),
        (0x011d_d000, 0x1000),
        (0x0120_2000, 0x1000),
        (0x0126_7000, 0x1000),
        (0x011f_9000, 0x1000),
        (0x0102_3000, 0x1000),
    ] {
        e.map(page, len);
    }
    for (i, f) in THREAD_VTABLE_SLOTS.iter().enumerate() {
        e.mem.set_u32(THREAD_VTABLE + 4 * i as u32, *f);
    }
    e.mem.set_cstr(NAME_FORMAT, b"Parallel Task Thread %i");
    e.mem.set_u32(PROCESSORS, 5);
    e.mem.set_u32(DIALOGUE_PLAYER, 0);
    // The bound merge's threshold (as the culling tests set it).
    e.mem.set_u32(0x011a_b3c0, 0x3586_37bd);
    e.mem.set_f32(0x0101_6248, 0.5);
    e.mem.set_f32(0x0101_7718, 3.0);
    // The doubles 0.5 and 1.0, the next-index table of the quaternion and 1/7 (as `par_bones` sets them).
    e.mem.set_u64(0x0101_1588, 0.5f64.to_bits());
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    for (i, v) in [1u32, 2, 0].iter().enumerate() {
        e.mem.set_u32(0x011a_9e98 + 4 * i as u32, *v);
    }
    e.mem.set_f32(0x011c_582c, 0.142_857_15);
    e.mem.set_u64(0x0102_31b0, f64::from(f32::MAX).to_bits());
    // Havok's free-list memory system with two thread memory records in use: room for the extra threads.
    let system = e.mem.alloc(0xc6c + 33 * 0x138);
    for k in (0..0xc6c + 33 * 0x138).step_by(4) {
        e.mem.set_u32(system + k, 0);
    }
    e.mem.set_u32(system, FREE_LIST_VTABLE);
    for i in 0..2 {
        e.mem.set_u8(system + 0xc6c + i * 0x138 + 0x12c, 1);
    }
    e.mem.set_u32(HAVOK_MEMORY_SYSTEM, system);
    e.register(MEMORY_ALLOCATE, |e, a| {
        let size = a[1].max(4);
        let block = e.mem.alloc(size);
        for k in (0..size).step_by(4) {
            e.mem.set_u32(block + k, 0);
        }
        ret(block)
    });
    e.register(MEMORY_FREE, |_, _| Ret::default());
    e.register(SPRINTF_S, |e, a| {
        let text = format!("Parallel Task Thread {}", a[3]);
        e.mem.set_cstr(a[0], text.as_bytes());
        ret(text.len() as u32)
    });
    e.register(THREAD_SETUP, |_, _| Ret::default());
    e.register(THREAD_SHUTDOWN, |_, _| Ret::default());
    e.register(SYNCHRONIZE, |_, a| {
        if let Ok(mut s) = SYNCS.lock() {
            s.push((a[0], a[1]));
        }
        Ret::default()
    });
    e.register(OTHER_FN, |_, a| {
        if let Ok(mut o) = OTHERS.lock() {
            o.push(a.first().copied().unwrap_or(0));
        }
        Ret::default()
    });
    e.register(WRAPPER, wrapper);
    e.start_real_clock();
    e
}

/// What one run leaves: every node's world transform and world bound (bytes), the syncs (sorted), and how many
/// runners took items.
#[derive(Debug, PartialEq)]
struct Outcome {
    nodes: Vec<Vec<u8>>,
    syncs: Vec<(u32, u32)>,
    /// The nodes two items update at once (the child listed twice, the listed node inside a listed subtree) and the
    /// listed node whose bound merges one of them: their indices in `nodes`.
    raced: Vec<usize>,
}

/// The scene built, the manager made with `threads` threads, both item sets run through it `rounds` times, the
/// manager ended.
fn run(threads: u32, rounds: u32) -> (Outcome, usize) {
    set_threads_for_tests(Some(threads));
    let mut e = engine();
    let mut s = Scene::default();
    s.vt = s.vtable(
        &mut e,
        &[
            (0xc0, UPDATE_TRANSFORM_AND_BOUNDS),
            (0xb8, UPDATE_WORLD_DATA),
        ],
    );
    // The scene: a root with a parent of twelve children (listed by each child, as `0087d030` expands it, and its
    // fourth child listed again), a branch whose child is listed too (a listed node inside a listed node's
    // subtree), and twenty branches listed alone.
    let root = s.node(&mut e, 0, [1.0, 2.0, 3.0], 0.3, 24, ([0.0; 3], 0.0));
    let big = s.branch(&mut e, root, 1, 12);
    let big_children: Vec<u32> = (0..12)
        .map(|k| e.mem.u32(e.mem.u32(big + 0xa0) + 4 * k))
        .collect();
    let outer = s.branch(&mut e, root, 2, 3);
    let inner = e.mem.u32(e.mem.u32(outer + 0xa0) + 4);
    let alone: Vec<u32> = (0..20).map(|k| s.branch(&mut e, root, 10 + k, 2)).collect();
    let mut listed = big_children.clone();
    listed.push(big_children[3]);
    listed.push(outer);
    listed.push(inner);
    listed.extend(&alone);
    let nodes = word_array(&mut s, &mut e, &listed);
    // The actors: entries of (actor, collision object); some gated out (no 3D, its loaded flag clear, the
    // collision object's flag 0x100000 set, no actor).
    let mut entries = Vec::new();
    for k in 0..40u32 {
        let actor = s.alloc(&mut e, 0x100);
        let loaded = s.alloc(&mut e, 0x50);
        e.mem.set_u8(loaded + 0x41, u8::from(k % 7 != 3));
        if k % 11 != 5 {
            e.mem.set_u32(actor + 0xac, loaded);
        }
        let collision = s.alloc(&mut e, 0x40);
        if k % 9 == 4 {
            e.mem.set_u32(collision + 0x30, 0x0010_0000);
        }
        entries.push(if k % 13 == 6 { 0 } else { actor });
        entries.push(collision);
    }
    let actors = s.alloc(&mut e, 4 * entries.len() as u32);
    for (i, w) in entries.iter().enumerate() {
        e.mem.set_u32(actors + 4 * i as u32, *w);
    }
    let actor_list = s.alloc(&mut e, 0x10);
    e.mem.set_u32(actor_list + 4, actors);
    e.mem.set_u32(actor_list + 8, 40);

    real_threads::start(&mut e).unwrap();
    real_threads::end_start_hold_for_tests(&e);
    let manager = s.alloc(&mut e, 0x10);
    e.call(MANAGER_CONSTRUCT, &args![manager]);
    // The workers are made now and wait on their start; main lets them reach the wait first.
    e.call(0x00fd_f104, &args![30u32]);
    RUNNERS.lock().unwrap().clear();
    SYNCS.lock().unwrap().clear();
    let work = s.alloc(&mut e, 0x10);
    for _ in 0..rounds {
        for (item, array, count) in [
            (NODE_ITEM, nodes, listed.len() as u32),
            (ACTOR_ITEM, actor_list, 40),
        ] {
            ITEM_FN.store(item, Relaxed);
            e.call(DESCRIPTOR, &args![work, WRAPPER, array, count]);
            e.call(RUN_SPLIT, &args![manager, work]);
        }
    }
    e.call(MANAGER_DESTROY, &args![manager]);
    set_threads_for_tests(None);
    let mut syncs = SYNCS.lock().unwrap().clone();
    syncs.sort_unstable();
    let nodes = s
        .nodes
        .iter()
        .map(|&n| {
            let mut bytes = e.mem.bytes(n + 0x68, 0x34);
            bytes.extend(e.mem.bytes(e.mem.u32(n + 0x20), 0x10));
            bytes
        })
        .collect();
    let runners = RUNNERS.lock().unwrap().len();
    let raced = [big_children[3], inner, outer]
        .iter()
        .map(|n| s.nodes.iter().position(|m| m == n).unwrap())
        .collect();
    (
        Outcome {
            nodes,
            syncs,
            raced,
        },
        runners,
    )
}

/// Four and eight threads leave every node's world transform, every bound (but for the nodes two items update at once:
/// there within the last bits, as with the exe's two runners) and the actors synchronized as the exe's one thread
/// does; each actor that passes the gates is synchronized once a round, with 2; more runners took items.
#[test]
fn more_threads_leave_the_scene_as_the_exe_s_one() {
    let rounds = 3;
    let (one, _) = run(1, rounds);
    // The gates: 40 actors less those without an actor, without 3D, not loaded, or flagged.
    let passing = (0..40u32)
        .filter(|k| k % 13 != 6 && k % 11 != 5 && k % 7 != 3 && k % 9 != 4)
        .count();
    assert_eq!(one.syncs.len(), passing * rounds as usize);
    assert!(one.syncs.iter().all(|&(_, how)| how == 2));
    let mut per: BTreeMap<u32, u32> = BTreeMap::new();
    for &(c, _) in &one.syncs {
        *per.entry(c).or_default() += 1;
    }
    assert!(
        per.values().all(|&n| n == rounds),
        "each passing actor once a round"
    );
    // The updates moved the nodes (not the identity they started with).
    let moved = one
        .nodes
        .iter()
        .filter(|b| b[0x24..0x30] != [0u8; 12])
        .count();
    assert!(
        moved > one.nodes.len() / 2,
        "{moved} of {} nodes moved",
        one.nodes.len()
    );
    assert!(OTHERS.lock().unwrap().is_empty(), "no other slot called");
    let mut most = 0;
    for threads in [4, 8, 4, 8] {
        let (many, runners) = run(threads, rounds);
        assert_eq!(many.syncs, one.syncs, "{threads} threads: the same syncs");
        for (i, (a, b)) in many.nodes.iter().zip(&one.nodes).enumerate() {
            assert_eq!(
                a[..0x34],
                b[..0x34],
                "{threads} threads: node {i}'s world transform"
            );
            if one.raced.contains(&i) {
                // Two items update this node at once (or its bound merges such a node's): each resets its bound and
                // merges its children's into it in place, so the interleaving can move the last bits (the exe's two
                // runners race the same way). The bound is still the merge of the same children's.
                let floats = |v: &[u8]| -> Vec<f32> {
                    v[0x34..0x44]
                        .chunks(4)
                        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                        .collect()
                };
                for (x, y) in floats(a).iter().zip(floats(b)) {
                    assert!(
                        (x - y).abs() <= 1e-5 * y.abs().max(1.0),
                        "{threads} threads: node {i}'s bound {:?} against {:?}",
                        floats(a),
                        floats(b)
                    );
                }
            } else {
                assert_eq!(a, b, "{threads} threads: node {i}'s bound");
            }
        }
        most = most.max(runners);
    }
    assert!(most > 2, "at most {most} runners took items");
}
