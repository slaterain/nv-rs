//! `NV_PAR_RENDER=1`: the culling walk on several cores, registered in the original order (the
//! parallel-render design, P1 to P5 of DESIGN.md in the measure folder parallel-render). Off by
//! default; with real threads only (the helpers are engines like a game thread's,
//! `engine_core::real_threads::fork_join`). A host change, not exe behaviour: what the game sees
//! happens in the order one thread makes it.
//!
//! **The split.** On main, a node whose `OnVisible` walks at least `NV_PAR_RENDER_MIN` (default 16)
//! children (not the walk's root), with a Bethesda culling process whose `Process` and
//! `AppendVirtual` are the typed ones and no compound frustum in use, hands its children in
//! contiguous chunks to the helpers (`NV_PAR_RENDER_THREADS`, default 3). Each walks its chunks with
//! its own copy of the process (its bytes on the helper's guest stack; at a child's `Cull` the
//! process is in the state the node's loop gives every child, since each child's walk puts the
//! planes and mode back). Main waits: only it runs what the log holds.
//!
//! **The log.** While a helper walks a chunk it collects:
//! - an object whose `OnVisible` is a geometry's (it would register itself with the accumulator),
//!   or a function not audited for other cores (every one but the typed `NiNode` and
//!   `BSMultiBoundNode` walks and the fade node's), is not run but logged with the process's mode
//!   and active planes at that point;
//! - the fade node's `OnVisible` runs on the helper: it writes its own fade state, and its three
//!   calls into the accumulator's side (start and stop grouping alphas, the fade alpha down its
//!   subtree's properties) are logged with their words instead of run (the helper's engine has
//!   logging functions at those addresses, `engine_core::real_threads::helper_register`).
//!
//! Otherwise the walk writes only the process copy and the multi-bound shapes' kept verdicts (each
//! shape is one node's, so one chunk's).
//!
//! **The replay.** Main runs the chunks' logs in child order: an `OnVisible` with the logged mode
//! and active planes set on its own process (a geometry registers, a tree or particle system walks
//! here on main) and put back after; a logged call with its words. So every registration,
//! accumulator call and unaudited override runs on main, in the serial order, with the process
//! state the serial walk had.
//!
//! **The check** (`NV_PAR_RENDER=check`): after the chunks, the fade nodes they ran get back the
//! bytes their `OnVisible` writes (flags and fade fields, saved before each ran: a second walk would
//! step their fade again), then one helper walks all the children in order into a log of its own;
//! the two are compared entry by entry, a difference is printed (the first 20), and the one-helper
//! log is the one replayed. The flags word is put back whole: a change another game thread made to
//! it within the split is lost (check mode only).

use super::layout::{
    BSCompoundFrustum, BSCullingProcess, NiCullingProcess, NiFrustumPlanes, NiNode,
};
use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{typed_call, Engine, Ret};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;

/// The process's `Process` and `AppendVirtual` slots and the typed functions a split needs there.
const SLOT_PROCESS: u32 = 0x44;
const SLOT_APPEND_VIRTUAL: u32 = 0x4c;
/// The size of `BSCullingProcess`: the bytes a helper's copy takes.
const PROCESS_SIZE: u32 = 0xc8;
/// `BSFadeNode::OnVisible`, which runs on the helpers.
const FADE_NODE_ON_VISIBLE: u32 = 0x00b4_e490;
/// The fade node's calls into the accumulator's side, logged on the helpers: start grouping alphas
/// (`thiscall(accumulator, bound, 1)`), stop (`thiscall(accumulator)`), the fade alpha down a
/// subtree (`(node, alpha)`).
const START_GROUPING_ALPHAS: u32 = 0x00b6_3500;
const STOP_GROUPING_ALPHAS: u32 = 0x00b6_3090;
const SET_FADE_ALPHA: u32 = 0x00b6_bb30;

/// One logged step of a helper's walk.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Entry {
    /// An object's `OnVisible`, with the process's mode and active planes when it was reached.
    Visible { object: u32, mode: u32, planes: u32 },
    /// A call by address with its words.
    Call { addr: u32, words: Vec<u32> },
}

/// 0: not read, 1: off, 2: on, 3: check, 4: serial (the helpers made and given each split's process
/// copy as when on, the walk serial on main: the off side of the stream gate).
static MODE: AtomicU8 = AtomicU8::new(0);
static HELPERS: AtomicUsize = AtomicUsize::new(3);
static MIN_CHILDREN: AtomicU32 = AtomicU32::new(16);
/// Differences the check mode has printed (it stops at 20), and splits it compared.
static PRINTED: AtomicU32 = AtomicU32::new(0);
/// Splits the helpers walked (printed at 1024, 2048, 4096, ...: the proof they ran).
static WALKS: AtomicU32 = AtomicU32::new(0);
static COMPARED: AtomicU32 = AtomicU32::new(0);

/// The bytes of a fade node its `OnVisible` writes: `m_uFlags` (+0x30) and the fade fields
/// (+0xac to the end, 0xe4).
const FADE_FLAGS: u32 = 0x30;
const FADE_FIELDS: u32 = 0xac;
const FADE_FIELDS_SIZE: u32 = 0xe4 - 0xac;

/// A fade node's written bytes before the check mode's first walk, put back before its second.
struct FadeState {
    node: u32,
    flags: u32,
    fields: Vec<u8>,
}

thread_local! {
    /// While this thread walks a chunk: its log.
    static LOG: RefCell<Option<Vec<Entry>>> = const { RefCell::new(None) };
    /// The check mode's first walk on this helper: each fade node's state before it runs.
    static SAVED: RefCell<Option<Vec<FadeState>>> = const { RefCell::new(None) };
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
    let m = if parts().any(|p| p == "serial") {
        4
    } else if parts().any(|p| p == "check") {
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
            "NV_PAR_RENDER={}: culling walks split over {} helpers at nodes of {}+ children",
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

/// While this thread walks a chunk: logs `object`'s `OnVisible` (`target`, the function its slot
/// holds) with `process`'s state and answers true (the caller doesn't run it), unless it is the
/// fade node's, which runs here. Otherwise false.
#[inline(always)]
pub(crate) fn defer(e: &Engine, object: u32, process: u32, target: u32) -> bool {
    if mode() < 2 {
        return false;
    }
    if target == FADE_NODE_ON_VISIBLE {
        if mode() == 3 {
            SAVED.with(|saved| {
                if let Some(saved) = saved.borrow_mut().as_mut() {
                    saved.push(FadeState {
                        node: object,
                        flags: e.mem.u32(object.wrapping_add(FADE_FLAGS)),
                        fields: e
                            .mem
                            .bytes(object.wrapping_add(FADE_FIELDS), FADE_FIELDS_SIZE),
                    });
                }
            });
        }
        return false;
    }
    LOG.with(|log| match log.borrow_mut().as_mut() {
        Some(log) => {
            log.push(Entry::Visible {
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

/// A call the helper's engine logs instead of running (see [`install_logging`]).
fn log_call(addr: u32, args: &[u32]) -> Ret {
    LOG.with(|log| {
        if let Some(log) = log.borrow_mut().as_mut() {
            log.push(Entry::Call {
                addr,
                words: args.to_vec(),
            });
        }
    });
    Ret::default()
}

fn log_start_grouping(_e: &mut Engine, args: &[u32]) -> Ret {
    log_call(START_GROUPING_ALPHAS, args)
}

fn log_stop_grouping(_e: &mut Engine, args: &[u32]) -> Ret {
    log_call(STOP_GROUPING_ALPHAS, args)
}

fn log_fade_alpha(_e: &mut Engine, args: &[u32]) -> Ret {
    log_call(SET_FADE_ALPHA, args)
}

/// The logging functions on a helper's own engine (each job: a function registered for every
/// engine since would otherwise replace them).
fn install_logging(e: &mut Engine) {
    use engine_core::real_threads::helper_register;
    helper_register(e, START_GROUPING_ALPHAS, log_start_grouping);
    helper_register(e, STOP_GROUPING_ALPHAS, log_stop_grouping);
    helper_register(e, SET_FADE_ALPHA, log_fade_alpha);
}

/// The process's frustum planes.
#[inline(always)]
fn planes(process: u32) -> Ptr<NiFrustumPlanes> {
    Ptr::<NiCullingProcess>::new(process).at(NiCullingProcess::m_kPlanes)
}

/// Whether this thread walks a chunk now.
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

/// The main thread (the split runs only there), with real or stepped threads.
fn on_main(e: &Engine) -> bool {
    engine_core::thread_host::on_main(e)
}

/// Walks `children` (a chunk) with `process` into a log (on a helper).
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

/// Runs `walk` on the helpers (not main) with a copy of the process `image` each.
fn on_helpers(
    e: &mut Engine,
    helpers: usize,
    image: &[u8],
    walk: &(dyn Fn(&mut Engine, usize, u32) + Sync),
) -> bool {
    let job = |w: &mut Engine, number: usize| {
        if number == 0 {
            return;
        }
        install_logging(w);
        w.with_stack(PROCESS_SIZE, |w, local| {
            w.mem.write(local.addr(), image);
            walk(w, number, local.addr());
        });
    };
    engine_core::real_threads::fork_join(e, helpers, &job)
}

/// Splits `node`'s `count` children over the helpers, then replays their logs. False (nothing
/// done; the caller walks them as before) when a split isn't possible here.
pub(crate) fn split(e: &mut Engine, node: Ptr<NiNode>, process: u32, count: u32) -> bool {
    if NEST.with(Cell::get) == 0 || collecting() || !on_main(e) || !plain_process(e, process) {
        return false;
    }
    let base = e.get(node, NiNode::m_pBase);
    let children: Vec<u32> = (0..count)
        .map(|index| e.mem.u32(base.index(index, 4).addr()))
        .collect();
    let helpers = HELPERS.load(Relaxed);
    let chunk = children.len().div_ceil(helpers * 4).max(1);
    let chunks: Vec<&[u32]> = children.chunks(chunk).collect();
    let logs: Vec<Mutex<Option<Vec<Entry>>>> = chunks.iter().map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let image = e.mem.bytes(process, PROCESS_SIZE);
    let check = mode() == 3;
    let saved: Mutex<Vec<FadeState>> = Mutex::new(Vec::new());
    let walk = |w: &mut Engine, _number: usize, walk_process: u32| loop {
        let k = next.fetch_add(1, Relaxed);
        let Some(children) = chunks.get(k) else {
            break;
        };
        if check {
            SAVED.with(|s| *s.borrow_mut() = Some(Vec::new()));
        }
        let log = walk_chunk(w, children, walk_process);
        if check {
            let mine = SAVED.with(|s| s.borrow_mut().take()).unwrap_or_default();
            saved
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend(mine);
        }
        *logs[k]
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(log);
    };
    if mode() == 4 {
        // Serial: the helpers take their process copies as when on, and walk nothing; main walks.
        on_helpers(e, helpers, &image, &|_: &mut Engine, _: usize, _: u32| {});
        return false;
    }
    if !on_helpers(e, helpers, &image, &walk) {
        return false;
    }
    let walks = WALKS.fetch_add(1, Relaxed) + 1;
    if walks.is_power_of_two() && walks >= 1024 {
        println!("NV_PAR_RENDER: {walks} splits walked on the helpers");
    }
    let logs: Vec<Vec<Entry>> = logs
        .into_iter()
        .map(|l| {
            l.into_inner()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .unwrap_or_default()
        })
        .collect();
    let replayed = if check {
        // The fade nodes as they were before the first walk: the second steps them once again.
        for state in saved
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            e.mem
                .set_u32(state.node.wrapping_add(FADE_FLAGS), state.flags);
            e.mem
                .write(state.node.wrapping_add(FADE_FIELDS), &state.fields);
        }
        let serial: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
        let one = |w: &mut Engine, number: usize, walk_process: u32| {
            if number == 1 {
                let log = walk_chunk(w, &children, walk_process);
                *serial
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = log;
            }
        };
        on_helpers(e, helpers, &image, &one);
        let serial = serial
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        compare(&logs, &serial);
        vec![serial]
    } else {
        logs
    };
    for entry in replayed.iter().flatten() {
        replay(e, process, entry);
    }
    true
}

/// One entry run on main as the serial walk ran it.
fn replay(e: &mut Engine, process: u32, entry: &Entry) {
    match entry {
        Entry::Visible {
            object,
            mode,
            planes: active,
        } => {
            let cull = Ptr::<BSCullingProcess>::new(process);
            let mode_now = e.get(cull, BSCullingProcess::kCullMode);
            let planes_now = e.get(planes(process), NiFrustumPlanes::m_uiActivePlanes);
            e.set(cull, BSCullingProcess::kCullMode, *mode);
            e.set(planes(process), NiFrustumPlanes::m_uiActivePlanes, *active);
            super::process::object_on_visible(e, Ptr::new(*object), process);
            e.set(cull, BSCullingProcess::kCullMode, mode_now);
            e.set(
                planes(process),
                NiFrustumPlanes::m_uiActivePlanes,
                planes_now,
            );
        }
        Entry::Call { addr, words } => {
            e.call(*addr, words);
        }
    }
}

/// The check mode: the helpers' logs, in order, against one helper's walk of all the children.
fn compare(logs: &[Vec<Entry>], serial: &[Entry]) {
    let compared = COMPARED.fetch_add(1, Relaxed) + 1;
    if compared.is_power_of_two() && compared >= 1024 {
        println!(
            "NV_PAR_RENDER=check: {compared} splits compared, {} differed",
            PRINTED.load(Relaxed)
        );
    }
    let parallel: Vec<&Entry> = logs.iter().flatten().collect();
    if parallel.len() == serial.len() && parallel.iter().zip(serial).all(|(a, b)| *a == b) {
        return;
    }
    if PRINTED.fetch_add(1, Relaxed) >= 20 {
        return;
    }
    let at = parallel
        .iter()
        .zip(serial)
        .position(|(a, b)| *a != b)
        .unwrap_or(parallel.len().min(serial.len()));
    println!(
        "NV_PAR_RENDER=check: the logs differ at entry {at} of {} (parallel) and {} (one helper): {:?} against {:?}",
        parallel.len(),
        serial.len(),
        parallel.get(at),
        serial.get(at)
    );
}
