//! `NV_PAR_RENDER=1`: the culling walk on several cores, registered in the original order (the
//! parallel-render design, P1 to P5 of DESIGN.md in the measure folder parallel-render). Off by
//! default; with real threads only (the helpers are engines like a game thread's,
//! `engine_core::real_threads::fork_join`). A host change, not exe behaviour: what the game sees
//! happens in the order one thread makes it.
//!
//! **The split.** On main, a node whose `OnVisible` walks at least `NV_PAR_RENDER_MIN` (default 16)
//! children, with a Bethesda culling process whose `Process` and `AppendVirtual` are the typed ones
//! and no compound frustum in use, hands its children in contiguous chunks to the helpers
//! (`NV_PAR_RENDER_THREADS`, default 3) and to main. Each walks its chunks with its own copy of the
//! process (its bytes on the walker's guest stack; at a child's `Cull` the process is in the state
//! the node's loop gives every child, since each child's walk puts the planes and mode back).
//!
//! **The log.** While a thread walks a chunk it collects: an object whose `OnVisible` is a
//! geometry's (it would register itself with the accumulator), or a function not audited for other
//! cores (every one but the typed `NiNode` and `BSMultiBoundNode` walks), is not run but logged with
//! the process's mode and active planes at that point. The walk itself writes only the process
//! copy and the multi-bound shapes' kept verdicts (each shape is one node's, so one chunk's).
//!
//! **The replay.** Main runs the chunks' logs in child order: for each entry it sets its own
//! process's mode and active planes to the logged ones, runs the object's `OnVisible` as the serial
//! walk would have (a geometry registers, a fade node or tree walks its subtree here on main), and
//! puts them back. So every registration, accumulator call and override runs on main, in the
//! serial order, with the process state the serial walk had.
//!
//! **The check** (`NV_PAR_RENDER=check`): the chunks are walked on the helpers, then serially on
//! main into a log of its own, and the two are compared entry by entry; a difference is printed
//! (the first few) and the serial log is the one replayed.

use super::layout::{
    BSCompoundFrustum, BSCullingProcess, NiCullingProcess, NiFrustumPlanes, NiNode,
};
use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{typed_call, Engine};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;

/// The process's `Process` and `AppendVirtual` slots and the typed functions a split needs there.
const SLOT_PROCESS: u32 = 0x44;
const SLOT_APPEND_VIRTUAL: u32 = 0x4c;
/// The size of `BSCullingProcess`: the bytes a helper's copy takes.
const PROCESS_SIZE: u32 = 0xc8;

/// One logged `OnVisible`: the object and the process's mode and active planes when it was reached.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Entry {
    object: u32,
    mode: u32,
    planes: u32,
}

/// 0: not read, 1: off, 2: on, 3: check.
static MODE: AtomicU8 = AtomicU8::new(0);
static HELPERS: AtomicUsize = AtomicUsize::new(3);
static MIN_CHILDREN: AtomicU32 = AtomicU32::new(16);
/// Differences the check mode has printed (it stops at 20).
static PRINTED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    /// While this thread walks a chunk: its log.
    static LOG: RefCell<Option<Vec<Entry>>> = const { RefCell::new(None) };
    /// The `NiNode` walks open on this thread (the root's is never split: its children are few and
    /// uneven, the cells and the sky).
    static NEST: Cell<u32> = const { Cell::new(0) };
}

/// Held while a node walks its children (switch on only).
pub(crate) struct Nest;

/// A node starts walking its children.
#[inline(always)]
pub(crate) fn nest() -> Option<Nest> {
    if mode() < 2 {
        return None;
    }
    NEST.with(|n| n.set(n.get() + 1));
    Some(Nest)
}

impl Drop for Nest {
    fn drop(&mut self) {
        NEST.with(|n| n.set(n.get().saturating_sub(1)));
    }
}

#[inline(always)]
fn mode() -> u8 {
    match MODE.load(Relaxed) {
        0 => read_switch(),
        m => m,
    }
}

#[cold]
fn read_switch() -> u8 {
    let value = std::env::var("NV_PAR_RENDER").unwrap_or_default();
    let parts = || value.split(',').map(str::trim);
    let m = if parts().any(|p| p == "check") {
        3
    } else if parts().any(|p| p == "1" || p == "on") {
        2
    } else {
        1
    };
    if m >= 2 {
        if let Some(n) = std::env::var("NV_PAR_RENDER_THREADS")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
        {
            HELPERS.store(n.clamp(1, 15), Relaxed);
        }
        if let Some(n) = std::env::var("NV_PAR_RENDER_MIN")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
        {
            MIN_CHILDREN.store(n.max(2), Relaxed);
        }
        println!(
            "NV_PAR_RENDER={}: culling walks split over {} helpers and main at nodes of {}+ children",
            value.trim(),
            HELPERS.load(Relaxed),
            MIN_CHILDREN.load(Relaxed)
        );
    }
    MODE.store(m, Relaxed);
    m
}

/// Whether a node of `count` children may be split (the switch is on and the node is big enough).
#[inline(always)]
pub(crate) fn wants_split(count: u32) -> bool {
    count >= 2 && mode() >= 2 && count >= MIN_CHILDREN.load(Relaxed)
}

/// While this thread walks a chunk: logs `object`'s `OnVisible` with `process`'s state and answers
/// true (the caller doesn't run it). Otherwise false.
#[inline(always)]
pub(crate) fn defer(e: &Engine, object: u32, process: u32) -> bool {
    if mode() < 2 {
        return false;
    }
    LOG.with(|log| match log.borrow_mut().as_mut() {
        Some(log) => {
            log.push(Entry {
                object,
                mode: e.get(
                    Ptr::<BSCullingProcess>::new(process),
                    BSCullingProcess::kCullMode,
                ),
                planes: e.get(planes(process), NiFrustumPlanes::m_uiActivePlanes),
            });
            true
        }
        None => false,
    })
}

/// The process's frustum planes.
#[inline(always)]
fn planes(process: u32) -> Ptr<NiFrustumPlanes> {
    Ptr::<NiCullingProcess>::new(process).at(NiCullingProcess::m_kPlanes)
}

pub(crate) fn collecting() -> bool {
    LOG.with(|log| log.borrow().is_some())
}

/// Whether `process` is one a split can copy: the typed `Process` and `AppendVirtual`, no compound
/// frustum in use.
fn plain_process(e: &Engine, process: u32) -> bool {
    if crate::slot_target(e, process, SLOT_PROCESS) != 0x00c4_ee90
        || crate::slot_target(e, process, SLOT_APPEND_VIRTUAL) != 0x00c4_f1d0
    {
        return false;
    }
    let frustum = e.get(
        Ptr::<BSCullingProcess>::new(process),
        BSCullingProcess::pCompoundFrustum,
    );
    frustum.is_null() || e.get(frustum, BSCompoundFrustum::iFreeOp) == 0
}

/// The main thread (the split runs only there).
fn on_main(e: &Engine) -> bool {
    e.real
        .as_ref()
        .is_some_and(|r| r.id == engine_core::thread_host::MAIN_THREAD_ID)
}

/// Walks `children` (a chunk) with `process` into a log.
fn walk_chunk(e: &mut Engine, children: &[u32], process: u32) -> Vec<Entry> {
    LOG.with(|log| *log.borrow_mut() = Some(Vec::new()));
    struct Take;
    impl Drop for Take {
        fn drop(&mut self) {
            LOG.with(|log| *log.borrow_mut() = None);
        }
    }
    let take = Take;
    for &child in children {
        if child != 0 {
            typed_call!(
                FAMILY,
                0x00a5_9e00,
                super::visible::cull(e, Ptr::new(child), Ptr::new(process))
            );
        }
    }
    let log = LOG.with(|log| log.borrow_mut().take()).unwrap_or_default();
    drop(take);
    log
}

/// Splits `node`'s `count` children over the helpers and main, then replays their logs. False
/// (nothing done; the caller walks them as before) when a split isn't possible here.
pub(crate) fn split(e: &mut Engine, node: Ptr<NiNode>, process: u32, count: u32) -> bool {
    if NEST.with(Cell::get) == 0 || collecting() || !on_main(e) || !plain_process(e, process) {
        return false;
    }
    let base = e.get(node, NiNode::m_pBase);
    let children: Vec<u32> = (0..count)
        .map(|index| e.mem.u32(base.index(index, 4).addr()))
        .collect();
    let helpers = HELPERS.load(Relaxed);
    let chunk = children.len().div_ceil((helpers + 1) * 4).max(1);
    let chunks: Vec<&[u32]> = children.chunks(chunk).collect();
    let logs: Vec<Mutex<Option<Vec<Entry>>>> = chunks.iter().map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let image = e.mem.bytes(process, PROCESS_SIZE);
    let job = |w: &mut Engine, number: usize| {
        let work = |w: &mut Engine, walk_process: u32| loop {
            let k = next.fetch_add(1, Relaxed);
            let Some(children) = chunks.get(k) else {
                break;
            };
            let log = walk_chunk(w, children, walk_process);
            *logs[k]
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(log);
        };
        if number == 0 {
            work(w, process);
        } else {
            w.with_stack(PROCESS_SIZE, |w, local| {
                w.mem.write(local.addr(), &image);
                work(w, local.addr());
            });
        }
    };
    if !engine_core::real_threads::fork_join(e, helpers, &job) {
        return false;
    }
    let logs: Vec<Vec<Entry>> = logs
        .into_iter()
        .map(|l| {
            l.into_inner()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .unwrap_or_default()
        })
        .collect();
    let replayed = if mode() == 3 {
        let serial = walk_chunk(e, &children, process);
        compare(&logs, &serial);
        vec![serial]
    } else {
        logs
    };
    for entry in replayed.iter().flatten() {
        replay(e, process, *entry);
    }
    true
}

/// One entry run on main as the serial walk ran it.
fn replay(e: &mut Engine, process: u32, entry: Entry) {
    let cull = Ptr::<BSCullingProcess>::new(process);
    let mode = e.get(cull, BSCullingProcess::kCullMode);
    let planes_now = e.get(planes(process), NiFrustumPlanes::m_uiActivePlanes);
    e.set(cull, BSCullingProcess::kCullMode, entry.mode);
    e.set(
        planes(process),
        NiFrustumPlanes::m_uiActivePlanes,
        entry.planes,
    );
    super::process::object_on_visible(e, Ptr::new(entry.object), process);
    e.set(cull, BSCullingProcess::kCullMode, mode);
    e.set(
        planes(process),
        NiFrustumPlanes::m_uiActivePlanes,
        planes_now,
    );
}

/// The check mode: the helpers' logs, in order, against the serial walk's.
fn compare(logs: &[Vec<Entry>], serial: &[Entry]) {
    let parallel: Vec<Entry> = logs.iter().flatten().copied().collect();
    if parallel == serial {
        return;
    }
    if PRINTED.fetch_add(1, Relaxed) >= 20 {
        return;
    }
    let at = parallel
        .iter()
        .zip(serial)
        .position(|(a, b)| a != b)
        .unwrap_or(parallel.len().min(serial.len()));
    println!(
        "NV_PAR_RENDER=check: the logs differ at entry {at} of {} (parallel) and {} (serial): {:?} against {:?}",
        parallel.len(),
        serial.len(),
        parallel.get(at),
        serial.get(at)
    );
}
