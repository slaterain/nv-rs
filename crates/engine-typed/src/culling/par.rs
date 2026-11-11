//! `NV_PAR_RENDER`: the culling walk on several cores, registered in the original order (the
//! parallel-render design, P1 to P5 of DESIGN.md in the measure folder parallel-render). On by
//! default with real threads, with the helpers the cores allow (`default_helpers`; none below 8
//! logical processors); `NV_PAR_RENDER=off` turns it off, `=1` turns it on with real or stepped
//! threads alike (the helpers are engines like a game thread's,
//! `engine_core::real_threads::fork_join_first`; with stepped threads they run only inside main's
//! turn, while it is in the fork, so a repeatable run stays repeatable; they take no memory from
//! the game's heap, so the run records the same bytes with the switch on as off). Only with the
//! typed culling family on (the split is in its `NiNode::OnVisible`; with `NV_TYPED=off` nothing
//! splits). A host change, not exe behaviour: what the game sees happens in the order one thread
//! makes it.
//!
//! **The split.** On main, a node whose `OnVisible` walks at least `NV_PAR_RENDER_MIN` (default 16)
//! children, with a Bethesda culling process whose `Process` and `AppendVirtual` are the typed ones
//! and no compound frustum in use, makes tasks of its children (contiguous chunks) for the helpers
//! (`NV_PAR_RENDER_THREADS`, default 3) and itself. Each walker takes the next task not yet taken
//! and walks it with a copy of the process on its own guest stack (at a child's `Cull` the process
//! is in the state the node's loop gives every child, since each child's walk puts the planes and
//! mode back). A walker that meets another such node inside its task does not walk that node's
//! children itself: it makes tasks of them too, with the process as its own copy is at that node,
//! and goes on; whoever is free takes them. So one fork serves the whole subtree, and a subtree
//! that holds most of the work (the cell the player stands in) is shared out as well. Nothing the
//! logs hold runs before every task is walked.
//!
//! **The log.** While a walker (a helper, or main in its part) walks a task it collects:
//! - an object whose `OnVisible` is a geometry's (it would register itself with the accumulator),
//!   or a function not audited for other cores (every one but the typed `NiNode` and
//!   `BSMultiBoundNode` walks and the fade node's), is not run but logged with the process's mode
//!   and active planes at that point;
//! - the fade node's `OnVisible` runs on the walker: it writes its own fade state, and its three
//!   calls into the accumulator's side (start and stop grouping alphas, the fade alpha down its
//!   subtree's properties) are logged with their words instead of run (the walker's engine has
//!   logging functions at those addresses: a helper's for good,
//!   `engine_core::real_threads::helper_register`; main's only during its part of a split,
//!   `caller_swap`, with what they stood in for put back before the replay);
//! - a node below whose children became tasks leaves a mark in the log where their entries belong.
//!
//! Otherwise the walk writes only the process copy and the multi-bound shapes' kept verdicts (each
//! shape is one node's, so one task's).
//!
//! **The replay.** Main runs the tasks' logs in child order, a marked node's tasks at its mark: an
//! `OnVisible` with the logged mode and active planes set on its own process and put back after
//! (a geometry registers; a particle system or another unaudited object runs here on main; a
//! tree is walked on its walker, `tree_on_walker`); a logged call with its words. So every
//! registration, accumulator call and unaudited override runs on main, in the serial order, with
//! the process state the serial walk had.
//!
//! **The check** (`NV_PAR_RENDER=check`): after the tasks, the fade nodes they ran get back the
//! bytes their `OnVisible` writes (flags and fade fields, saved before each ran: a second walk would
//! step their fade again), then one walker walks all the children in order as one task, nodes
//! below included, into a log of its own; the two are compared entry by entry (the tasks' logs
//! laid out in order), a difference is printed (the first 20, with the objects there), and the
//! one-task log is the one replayed. The flags word is put back whole: a change another game
//! thread made to it within the split is lost (check mode only). `NV_PAR_RENDER=recheck` is the
//! check's control: its first walk is one walker's walk of all the children too, so a difference
//! there is the scene changing between the two walks (other game threads run beside them with
//! real threads), not the split.

use super::layout::{
    BSCompoundFrustum, BSCullingProcess, NiCullingProcess, NiFrustumPlanes, NiNode,
};
use super::probe::Refused;
use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{typed_call, AbiFn, Engine, Ret};
use std::cell::RefCell;
use std::sync::atomic::{
    AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering::Relaxed,
};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

/// The process's `Process` and `AppendVirtual` slots and the typed functions a split needs there.
const SLOT_PROCESS: u32 = 0x44;
const SLOT_APPEND_VIRTUAL: u32 = 0x4c;
/// The size of `BSCullingProcess`: the bytes a helper's copy takes.
const PROCESS_SIZE: u32 = 0xc8;
/// `BSFadeNode::OnVisible`, which runs on the helpers. What it reads, from the exe: its own node's
/// fields; globals (fading on, the frame time, the thresholds and multipliers); the camera through
/// the process; the accumulator's class (`0043b300` is an `IsKindOf` walk of its RTTI); and the most
/// alpha through the callback `0086a050`, the reference's actor alpha (an actor's virtual +0x420).
/// What the replayed calls write: the batch renderer's grouping state (start and stop), the
/// subtree's shader properties' fade alpha at +0x2c (`RecursiveSetPropertyFadeAlpha`, `00b6bb30`),
/// and registration's lists and property flags. The two sets don't meet, so walking it before the
/// replay reads what the serial walk reads.
const FADE_NODE_ON_VISIBLE: u32 = 0x00b4_e490;
/// The fade node's calls into the accumulator's side, logged on the helpers: start grouping alphas
/// (`thiscall(accumulator, bound, 1)`), stop (`thiscall(accumulator)`), the fade alpha down a
/// subtree (`(node, alpha)`).
const START_GROUPING_ALPHAS: u32 = 0x00b6_3500;
const STOP_GROUPING_ALPHAS: u32 = 0x00b6_3090;
const SET_FADE_ALPHA: u32 = 0x00b6_bb30;

/// One logged step of a walker's walk.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Entry {
    /// An object's `OnVisible`, with the process's mode and active planes when it was reached.
    Visible { object: u32, mode: u32, planes: u32 },
    /// A call by address with its words.
    Call { addr: u32, words: Vec<u32> },
    /// A node below whose children were walked as tasks of their own: the logs of the split's
    /// tasks `first..first + count` belong here, in that order.
    Tasks { first: usize, count: usize },
}

/// Some of a node's children, to be walked with the process as it was at that node.
struct Task {
    children: Vec<u32>,
    /// The process's bytes at the node ([`PROCESS_SIZE`]).
    image: Arc<[u8]>,
}

/// The tasks of one split and their logs, by task number.
#[derive(Default)]
struct Queue {
    /// Taken (`None`) when a walker takes the task.
    tasks: Vec<Option<Task>>,
    logs: Vec<Option<Vec<Entry>>>,
    /// The next task to take.
    next: usize,
    /// Tasks being walked now (they may still add tasks).
    walking: usize,
    /// A walker stopped (a panic in a task): the others stop taking tasks.
    stopped: bool,
}

/// What the walkers of one split share.
struct Split {
    queue: Mutex<Queue>,
    /// Whether a node met inside a task makes tasks of its children (not in the check mode's
    /// one-task walk).
    nested: bool,
    /// The check mode: each fade node's state before it runs is kept ([`FadeState`]).
    check: bool,
    saved: Mutex<Vec<FadeState>>,
    /// The probe is on: the walkers' time walking is added up (main's also on its own), and the
    /// helpers that joined in with the time from the split's making to each one's first task.
    timed: bool,
    born: Instant,
    busy_ns: AtomicU64,
    main_ns: AtomicU64,
    joined: AtomicU64,
    wake_ns: AtomicU64,
}

impl Split {
    fn new(nested: bool, check: bool, timed: bool) -> Arc<Split> {
        Arc::new(Split {
            queue: Mutex::new(Queue::default()),
            nested,
            check,
            saved: Mutex::new(Vec::new()),
            timed,
            born: Instant::now(),
            busy_ns: AtomicU64::new(0),
            main_ns: AtomicU64::new(0),
            joined: AtomicU64::new(0),
            wake_ns: AtomicU64::new(0),
        })
    }

    fn queue(&self) -> MutexGuard<'_, Queue> {
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Adds `children` as tasks of at most `chunk` children each, to be walked with the process
    /// bytes `image`; answers the first task's number and how many were added.
    fn add(&self, children: &[u32], chunk: usize, image: &Arc<[u8]>) -> (usize, usize) {
        let mut queue = self.queue();
        let first = queue.tasks.len();
        for part in children.chunks(chunk.max(1)) {
            queue.tasks.push(Some(Task {
                children: part.to_vec(),
                image: Arc::clone(image),
            }));
            queue.logs.push(None);
        }
        (first, queue.tasks.len() - first)
    }

    /// A walker's loop: takes tasks and walks them with its process copy at `local` until none is
    /// left and none is being walked (a task being walked may still add tasks).
    fn work(self: &Arc<Split>, w: &mut Engine, number: usize, local: u32) {
        struct Current;
        impl Drop for Current {
            fn drop(&mut self) {
                SPLIT.with(|s| *s.borrow_mut() = None);
            }
        }
        SPLIT.with(|s| *s.borrow_mut() = Some(Arc::clone(self)));
        let _current = Current;
        // Held while a task is walked: on a stop the task is given up and the others told.
        struct Walking<'a>(&'a Split, bool);
        impl Drop for Walking<'_> {
            fn drop(&mut self) {
                let mut queue = self.0.queue();
                queue.walking -= 1;
                if !self.1 {
                    queue.stopped = true;
                }
            }
        }
        let mut idle = 0u32;
        let mut first = true;
        loop {
            let taken = {
                let mut queue = self.queue();
                if queue.stopped {
                    break;
                }
                let k = queue.next;
                match queue.tasks.get_mut(k).and_then(Option::take) {
                    Some(task) => {
                        queue.next += 1;
                        queue.walking += 1;
                        Some((k, task))
                    }
                    None if queue.walking == 0 => break,
                    None => None,
                }
            };
            let Some((k, task)) = taken else {
                // Tasks being walked may add tasks: look again shortly.
                idle += 1;
                if idle % 64 == 0 {
                    std::thread::yield_now();
                } else {
                    std::hint::spin_loop();
                }
                continue;
            };
            idle = 0;
            if first && self.timed && number != 0 {
                self.joined.fetch_add(1, Relaxed);
                self.wake_ns
                    .fetch_add(super::probe::nanos(self.born.elapsed()), Relaxed);
            }
            first = false;
            let mut walking = Walking(self, false);
            if number == 0 {
                MAIN_CHUNKS.fetch_add(1, Relaxed);
            }
            w.mem.write(local, &task.image);
            if self.check {
                SAVED.with(|s| *s.borrow_mut() = Some(Vec::new()));
            }
            let started = self.timed.then(Instant::now);
            let log = walk_chunk(w, &task.children, local);
            if let Some(started) = started {
                let ns = super::probe::nanos(started.elapsed());
                self.busy_ns.fetch_add(ns, Relaxed);
                if number == 0 {
                    self.main_ns.fetch_add(ns, Relaxed);
                }
            }
            if self.check {
                let mine = SAVED.with(|s| s.borrow_mut().take()).unwrap_or_default();
                self.saved
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .extend(mine);
            }
            self.queue().logs[k] = Some(log);
            walking.1 = true;
        }
    }

    /// Main's part of the fork when it walks no task itself: waits until every task is walked
    /// (or a walker stopped). `fork_join_first` takes the end of the caller's part for the end of
    /// the work.
    fn wait(&self) {
        loop {
            {
                let queue = self.queue();
                if queue.stopped || (queue.next >= queue.tasks.len() && queue.walking == 0) {
                    return;
                }
            }
            std::thread::yield_now();
        }
    }

    /// The tasks' logs, by task number, once every walker is done.
    fn logs(&self) -> Vec<Vec<Entry>> {
        std::mem::take(&mut self.queue().logs)
            .into_iter()
            .map(Option::unwrap_or_default)
            .collect()
    }
}

/// 0: not read, 1: off, 2: on, 3: check, 4: serial (the helpers made and given each split's process
/// copy as when on, the walk serial on main: the off side of the stream gate).
static MODE: AtomicU8 = AtomicU8::new(0);
/// On by default (the switch unset): the walk splits only with real threads (a repeatable run, a
/// recorded run and the tests step their threads, and keep the serial walk).
static AUTO: AtomicBool = AtomicBool::new(false);
static HELPERS: AtomicUsize = AtomicUsize::new(3);
static MIN_CHILDREN: AtomicU32 = AtomicU32::new(16);
/// `NV_PAR_RENDER=recheck`: the check mode's control. Its first walk is one walker's walk of all
/// the children too (one task, the nodes below not made tasks), so it compares the serial walk
/// with the check's serial re-walk at the same two moments: what the scene changing between the
/// two walks does alone, with no split.
static RECHECK: AtomicBool = AtomicBool::new(false);
/// Differences the check mode has printed (it stops at 20), and splits it compared.
static PRINTED: AtomicU32 = AtomicU32::new(0);
/// Splits walked in tasks (printed at 1024, 2048, 4096, ...: the proof they ran).
static WALKS: AtomicU32 = AtomicU32::new(0);
static COMPARED: AtomicU32 = AtomicU32::new(0);
/// Tasks main walked itself.
static MAIN_CHUNKS: AtomicU32 = AtomicU32::new(0);
/// Nodes met inside a task whose children became tasks.
static BELOW: AtomicU32 = AtomicU32::new(0);

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
    /// While this thread walks a task: its log.
    static LOG: RefCell<Option<Vec<Entry>>> = const { RefCell::new(None) };
    /// The check mode's first walk on this walker: each fade node's state before it runs.
    static SAVED: RefCell<Option<Vec<FadeState>>> = const { RefCell::new(None) };
    /// While this thread is a split's walker: the split, for the tasks a node below adds.
    static SPLIT: RefCell<Option<Arc<Split>>> = const { RefCell::new(None) };
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
    let threads = std::env::var("NV_PAR_RENDER_THREADS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok());
    let m = if parts().any(|p| p == "off" || p == "0") {
        1
    } else if parts().any(|p| p == "serial") {
        4
    } else if parts().any(|p| p == "check") {
        3
    } else if parts().any(|p| p == "recheck") {
        RECHECK.store(true, Relaxed);
        3
    } else if parts().any(|p| p == "1" || p == "on") {
        2
    } else {
        // Unset (or only `time`): on by default with real threads, with the helpers the
        // machine's cores allow (`default_helpers`); never with stepped threads (`split`).
        let helpers = threads.unwrap_or_else(|| {
            default_helpers(std::thread::available_parallelism().map_or(1, usize::from))
        });
        if helpers == 0 {
            1
        } else {
            AUTO.store(true, Relaxed);
            2
        }
    };
    if m >= 2 {
        let helpers = threads.unwrap_or_else(|| {
            default_helpers(std::thread::available_parallelism().map_or(1, usize::from)).max(1)
        });
        HELPERS.store(helpers.clamp(1, 15), Relaxed);
        if let Some(n) = std::env::var("NV_PAR_RENDER_MIN")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
        {
            MIN_CHILDREN.store(n.max(2), Relaxed);
        }
        println!(
            "NV_PAR_RENDER={}: culling walks split over {} helpers at nodes of {}+ children{}",
            if value.trim().is_empty() {
                "(default)"
            } else {
                value.trim()
            },
            HELPERS.load(Relaxed),
            MIN_CHILDREN.load(Relaxed),
            if AUTO.load(Relaxed) {
                ", with real threads only"
            } else {
                ""
            }
        );
    }
    MODE.store(m, Relaxed);
    m
}

/// The helpers the parallel walk gets by default on a machine of `logical` logical processors:
/// three from 8 up, none (the walk stays serial) below 8. Measured on the maintainer's PC (12
/// cores, 24 logical processors), the crowded street: pinned to 8 logical processors, three
/// helpers -1.29 ms a frame (+9.3%) and one -0.47 ms (+3.2%); pinned to 4, one helper +0.16 ms
/// (-1.0%), and +0.33 ms (-4.2%) in the view of the sky. The game keeps main, the render thread,
/// its AI threads and loaders busy, so a helper needs a processor of its own to pay.
fn default_helpers(logical: usize) -> usize {
    if logical >= 8 {
        3
    } else {
        0
    }
}

/// For the replay-order test (`crates/engine/tests/par_render.rs`): the switch's mode (`off`, `on`,
/// `check`, `serial`), helpers and least children set as `NV_PAR_RENDER`, `NV_PAR_RENDER_THREADS`
/// and `NV_PAR_RENDER_MIN` would (the environment is then not read).
#[doc(hidden)]
pub fn set_for_tests(mode: &str, helpers: usize, min_children: u32) {
    let m = match mode {
        "on" => 2,
        "check" => 3,
        "serial" => 4,
        _ => 1,
    };
    AUTO.store(false, Relaxed);
    HELPERS.store(helpers.clamp(1, 15), Relaxed);
    MIN_CHILDREN.store(min_children.max(2), Relaxed);
    MODE.store(m, Relaxed);
}

/// For the replay-order test: splits walked in tasks, splits the check mode compared, the
/// comparisons that differed, the tasks main walked itself, and the nodes inside tasks whose
/// children became tasks, so far.
#[doc(hidden)]
pub fn counts_for_tests() -> (u32, u32, u32, u32, u32) {
    (
        WALKS.load(Relaxed),
        COMPARED.load(Relaxed),
        PRINTED.load(Relaxed),
        MAIN_CHUNKS.load(Relaxed),
        BELOW.load(Relaxed),
    )
}

/// The least children a node must have to split (`NV_PAR_RENDER_MIN`).
pub(crate) fn least_children() -> u32 {
    MIN_CHILDREN.load(Relaxed)
}

/// Whether a node of `count` children may be split or, inside a task, made tasks of (the switch
/// is on and the node is big enough).
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

/// `BSTreeNode::OnVisible` (`0066bf50`): `NiNode::OnVisible` when trees are on (`0054ee20`).
pub(crate) const TREE_NODE_ON_VISIBLE: u32 = 0x0066_bf50;
/// What `0054ee20` reads: the trees setting's byte (`0051d740`: the byte at +4 of the setting at
/// `011d5ce8`, through `00408d60`) and the tree manager singleton (`00664840(1)`, the global at
/// `011d5c48`), whose byte at +0x18 must be set too.
const TREES_SETTING_BYTE: u32 = 0x011d_5cec;
const TREE_MANAGER: u32 = 0x011d_5c48;
const TREE_MANAGER_ON: u32 = 0x18;

/// While this thread walks a task: a tree's `OnVisible` (`TREE_NODE_ON_VISIBLE`) run here, as the
/// exe runs it, instead of logged for main: when trees are on, the node's own `OnVisible` (so its
/// children are walked by this walker and its geometries logged). What `0054ee20` reads are two
/// bytes and a global, which nothing writes during a walk; only when the setting is on and the
/// manager does not exist yet (`00664840(1)` would make it, under a lock) is the tree logged for
/// main as before (false).
pub(crate) fn tree_on_walker(e: &mut Engine, node: Ptr<NiNode>, process: u32) -> bool {
    if mode() < 2 || !collecting() {
        return false;
    }
    if e.mem.u8(TREES_SETTING_BYTE) != 0 {
        let manager = e.mem.u32(TREE_MANAGER);
        if manager == 0 {
            return false;
        }
        if e.mem.u8(manager.wrapping_add(TREE_MANAGER_ON)) != 0 {
            typed_call!(
                FAMILY,
                0x00a5_dbe0,
                super::visible::node_on_visible(e, node, Ptr::new(process))
            );
        }
    }
    true
}

/// A call the helper's engine logs instead of running (see [`install_logging`]). Its words are
/// replayed on main after the helper's walk, so no pointer among them may point into the helper's
/// own stack; in the exe none does (the accumulator, the fade node, its world bound or the empty
/// bound), and one that does stops the run here rather than replay a dead local. Only the first
/// `pointers` words are pointers: the rest (a flag, an alpha's bits) are values, which may look
/// like a stack address (1.0 is `3f800000`, inside the heap the stacks come from).
/// On main (its own part of a split) the region is main's whole stack, the outer frames under
/// the walk included; it cannot fire falsely there either, since none of the logged pointers is
/// ever a stack local (the accumulator comes from the scene global, the bound is the node's).
fn log_call(e: &Engine, addr: u32, args: &[u32], pointers: usize) -> Ret {
    if let Some((low, high)) = e.guest_stack_region() {
        let pointer_words = &args[..pointers.min(args.len())];
        if let Some(word) = pointer_words.iter().find(|w| (low..high).contains(*w)) {
            panic!("NV_PAR_RENDER: a logged call to {addr:08x} passes {word:08x}, in the helper's stack");
        }
    }
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

/// `(accumulator, bound, flag)`: two pointers.
fn log_start_grouping(e: &mut Engine, args: &[u32]) -> Ret {
    log_call(e, START_GROUPING_ALPHAS, args, 2)
}

/// `(accumulator)`.
fn log_stop_grouping(e: &mut Engine, args: &[u32]) -> Ret {
    log_call(e, STOP_GROUPING_ALPHAS, args, 1)
}

/// `(node, alpha)`: the alpha is a float's bits, not a pointer.
fn log_fade_alpha(e: &mut Engine, args: &[u32]) -> Ret {
    log_call(e, SET_FADE_ALPHA, args, 1)
}

/// The functions that log the fade node's three accumulator-side calls, by the address each
/// stands in for while a walker collects.
const LOGGING: [(u32, AbiFn); 3] = [
    (START_GROUPING_ALPHAS, log_start_grouping),
    (STOP_GROUPING_ALPHAS, log_stop_grouping),
    (SET_FADE_ALPHA, log_fade_alpha),
];

/// The logging functions on a helper's own engine (each job: a function registered for every
/// engine since would otherwise replace them).
fn install_logging(e: &mut Engine) {
    for (addr, f) in LOGGING {
        engine_core::real_threads::helper_register(e, addr, f);
    }
}

/// Runs `fork` (a split's fork and join) with the logging functions on main's own engine, and
/// puts back what they stood in for after it, also when it stops. `fork` is told whether they are
/// there: they cannot go on main's engine when one of the three addresses has become a typed
/// family's member (`engine_core::real_threads::caller_swap`); main then walks no chunk, and the
/// helpers walk them all.
fn with_logging_on_main<R>(e: &mut Engine, fork: impl FnOnce(&mut Engine, bool) -> R) -> R {
    use engine_core::real_threads::caller_swap;
    let mut put_back: Vec<(u32, AbiFn)> = Vec::with_capacity(LOGGING.len());
    for (addr, f) in LOGGING {
        match caller_swap(e, addr, f) {
            Some(old) => put_back.push((addr, old)),
            None => break,
        }
    }
    let all = put_back.len() == LOGGING.len();
    if !all {
        for (addr, old) in put_back.drain(..) {
            caller_swap(e, addr, old);
        }
    }
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| fork(e, all)));
    for (addr, old) in put_back {
        caller_swap(e, addr, old);
    }
    match out {
        Ok(out) => out,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// The process's frustum planes.
#[inline(always)]
fn planes(process: u32) -> Ptr<NiFrustumPlanes> {
    Ptr::<NiCullingProcess>::new(process).at(NiCullingProcess::m_kPlanes)
}

/// Whether this thread walks a task now.
pub(crate) fn collecting() -> bool {
    LOG.with(|log| log.borrow().is_some())
}

/// Why `process` is not one a split can copy (none: it is): it needs the typed `Process` and
/// `AppendVirtual`, and no compound frustum in use.
fn not_plain(e: &Engine, process: u32) -> Option<Refused> {
    if crate::slot_target(e, process, SLOT_PROCESS) != 0x00c4_ee90
        || crate::slot_target(e, process, SLOT_APPEND_VIRTUAL) != 0x00c4_f1d0
    {
        return Some(Refused::Process);
    }
    let frustum = e.get(
        Ptr::<BSCullingProcess>::new(process),
        BSCullingProcess::pCompoundFrustum,
    );
    if frustum.is_null() || e.get(frustum, BSCompoundFrustum::iFreeOp) == 0 {
        None
    } else {
        Some(Refused::Frustum)
    }
}

/// A split refused for `why`: counted by the probe when it is on. Answers false.
#[cold]
fn refused(why: Refused) -> bool {
    if super::probe::on() {
        super::probe::split_refused(why);
    }
    false
}

/// The main thread (the split runs only there), with real or stepped threads.
fn on_main(e: &Engine) -> bool {
    engine_core::thread_host::on_main(e)
}

/// Walks `children` (a task) with `process` into a log (on a helper, or on main in its part).
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

/// Runs the split's walkers: the helpers and main (number 0), each with a process copy on its own
/// guest stack and the logging functions on its own engine, until the split's tasks are all
/// walked. Main takes its copy in any case, and walks only with the logging functions there
/// ([`with_logging_on_main`]); without them it waits for the helpers to walk every task. A helper
/// that has not started when the tasks are all walked does not join in
/// (`engine_core::real_threads::fork_join_first`). False when the helpers could not be had.
fn on_walkers(e: &mut Engine, helpers: usize, split: &Arc<Split>) -> bool {
    with_logging_on_main(e, |e, main_walks| {
        let job = |w: &mut Engine, number: usize| {
            if number != 0 {
                install_logging(w);
            }
            w.with_stack(PROCESS_SIZE, |w, local| {
                if number != 0 || main_walks {
                    split.work(w, number, local.addr());
                } else {
                    split.wait();
                }
            });
        };
        engine_core::real_threads::fork_join_first(e, helpers, &job)
    })
}

/// The children of `node` (its first `count` slots; 0: an empty slot).
fn children_of(e: &Engine, node: Ptr<NiNode>, count: u32) -> Vec<u32> {
    let base = e.get(node, NiNode::m_pBase);
    (0..count)
        .map(|index| e.mem.u32(base.index(index, 4).addr()))
        .collect()
}

/// How many children a task of a node of `count` children gets: enough tasks that every walker
/// has several to take, and no task smaller than `least`.
fn task_size(count: usize, helpers: usize, least: usize) -> usize {
    count.div_ceil((helpers + 1) * 4).max(least)
}

/// A node of `count` children met while this thread walks a task: its children become tasks of
/// the split, walked with the process as this walker's copy is now, and the log gets the mark
/// where their entries belong. False (the walker walks them itself) when the split takes no
/// tasks from below, or the process is not one to copy.
fn split_below(e: &mut Engine, node: Ptr<NiNode>, process: u32, count: u32) -> bool {
    let Some(split) = SPLIT.with(|s| s.borrow().clone()) else {
        return false;
    };
    if !split.nested || not_plain(e, process).is_some() {
        return false;
    }
    let children = children_of(e, node, count);
    let image: Arc<[u8]> = e.mem.bytes(process, PROCESS_SIZE).into();
    let chunk = task_size(children.len(), HELPERS.load(Relaxed), 8);
    let (first, count) = split.add(&children, chunk, &image);
    BELOW.fetch_add(1, Relaxed);
    LOG.with(|log| {
        if let Some(log) = log.borrow_mut().as_mut() {
            log.push(Entry::Tasks { first, count });
        }
    });
    true
}

/// Splits `node`'s `count` children over the helpers and main, then replays their logs. False
/// (nothing done; the caller walks them as before) when a split isn't possible here.
pub(crate) fn split(e: &mut Engine, node: Ptr<NiNode>, process: u32, count: u32) -> bool {
    if collecting() {
        return split_below(e, node, process, count);
    }
    if !on_main(e) || (AUTO.load(Relaxed) && e.real.is_none()) {
        return false;
    }
    if let Some(why) = not_plain(e, process) {
        return refused(why);
    }
    let timed = super::probe::on();
    let children = children_of(e, node, count);
    let helpers = HELPERS.load(Relaxed);
    let image: Arc<[u8]> = e.mem.bytes(process, PROCESS_SIZE).into();
    let check = mode() == 3;
    if mode() == 4 {
        // Serial: the helpers and main take their process copies as when on, and there is no
        // task; main then walks the children as with the switch off.
        on_walkers(e, helpers, &Split::new(false, false, false));
        return false;
    }
    // The check's control (`recheck`): the first walk is one walker's walk of all the children
    // too, with no tasks from the nodes below.
    let recheck = check && RECHECK.load(Relaxed);
    let tasks = Split::new(!recheck, check, timed);
    let chunk = if recheck {
        children.len()
    } else {
        task_size(children.len(), helpers, 1)
    };
    let (_, mut top) = tasks.add(&children, chunk, &image);
    let forked = timed.then(Instant::now);
    if !on_walkers(e, helpers, &tasks) {
        return refused(Refused::Helpers);
    }
    let joined = timed.then(Instant::now);
    let walks = WALKS.fetch_add(1, Relaxed) + 1;
    if walks.is_power_of_two() && walks >= 1024 {
        println!("NV_PAR_RENDER: {walks} splits walked in tasks");
    }
    let mut logs = tasks.logs();
    let task_count = logs.len();
    if check {
        // The fade nodes as they were before the first walk: the second steps them once again.
        let saved = std::mem::take(
            &mut *tasks
                .saved
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        for state in saved {
            e.mem
                .set_u32(state.node.wrapping_add(FADE_FLAGS), state.flags);
            e.mem
                .write(state.node.wrapping_add(FADE_FIELDS), &state.fields);
        }
        // All the children as one task, the nodes below walked in it.
        let one = Split::new(false, false, false);
        one.add(&children, children.len(), &image);
        on_walkers(e, helpers, &one);
        let serial = one.logs();
        let mut parallel = Vec::new();
        flatten(&logs, 0..top, &mut parallel);
        compare(e, &parallel, serial.first().map_or(&[][..], Vec::as_slice));
        logs = serial;
        top = logs.len();
    }
    let replaying = timed.then(super::probe::replay_begins);
    let entries = replay_tasks(e, process, &logs, 0..top);
    drop(replaying);
    if let (Some(forked), Some(joined)) = (forked, joined) {
        super::probe::split_walked(&super::probe::SplitTimes {
            children: children.len(),
            tasks: task_count,
            entries,
            fork: joined.duration_since(forked),
            replay: joined.elapsed(),
            busy_ns: tasks.busy_ns.load(Relaxed),
            main_ns: tasks.main_ns.load(Relaxed),
            joined: tasks.joined.load(Relaxed),
            wake_ns: tasks.wake_ns.load(Relaxed),
        });
    }
    true
}

/// The entries of tasks `range` in order, a marked node's tasks at its mark.
fn flatten<'a>(logs: &'a [Vec<Entry>], range: std::ops::Range<usize>, out: &mut Vec<&'a Entry>) {
    for log in logs.get(range).unwrap_or_default() {
        for entry in log {
            match entry {
                Entry::Tasks { first, count } => flatten(logs, *first..*first + *count, out),
                entry => out.push(entry),
            }
        }
    }
}

/// Replays tasks `range` in order, a marked node's tasks at its mark; how many entries it ran.
fn replay_tasks(
    e: &mut Engine,
    process: u32,
    logs: &[Vec<Entry>],
    range: std::ops::Range<usize>,
) -> usize {
    let mut entries = 0;
    for log in logs.get(range).unwrap_or_default() {
        for entry in log {
            match entry {
                Entry::Tasks { first, count } => {
                    entries += replay_tasks(e, process, logs, *first..*first + *count);
                }
                entry => {
                    replay(e, process, entry);
                    entries += 1;
                }
            }
        }
    }
    entries
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
        // A mark is replayed by its tasks ([`replay_tasks`]).
        Entry::Tasks { .. } => {}
    }
}

/// The check mode: the tasks' entries, in order, against one walker's walk of all the children
/// (with `recheck`, the first walk is one walker's too). The count so far is printed at 1024,
/// 2048, 4096, ... and every 16,384; a difference with the objects at it.
fn compare(e: &Engine, parallel: &[&Entry], serial: &[Entry]) {
    let label = if RECHECK.load(Relaxed) {
        "recheck"
    } else {
        "check"
    };
    let compared = COMPARED.fetch_add(1, Relaxed) + 1;
    if compared >= 1024 && (compared.is_power_of_two() || compared % 16_384 == 0) {
        println!(
            "NV_PAR_RENDER={label}: {compared} splits compared, {} differed",
            PRINTED.load(Relaxed)
        );
    }
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
    let first = if RECHECK.load(Relaxed) {
        "first walk"
    } else {
        "tasks"
    };
    println!(
        "NV_PAR_RENDER={label}: the logs differ at entry {at} of {} ({first}) and {} (one task): {:?} against {:?}",
        parallel.len(),
        serial.len(),
        parallel.get(at),
        serial.get(at)
    );
    for (side, entry) in [
        (first, parallel.get(at).copied()),
        ("one task", serial.get(at)),
    ] {
        if let Some(Entry::Visible { object, .. }) = entry {
            println!("  {side}: {}", describe(e, *object));
        }
    }
}

/// What the check prints of an object at a difference: its vtable, flags and name, and its
/// parent's (read without faulting: the object may have gone since).
fn describe(e: &Engine, object: u32) -> String {
    let word = |a: u32| {
        let mut b = [0u8; 4];
        e.mem.peek(a, &mut b).map(|()| u32::from_le_bytes(b)).ok()
    };
    let name = |o: u32| {
        let mut b = [0u8; 40];
        match word(o.wrapping_add(8)) {
            Some(p) if p != 0 && e.mem.peek(p, &mut b).is_ok() => {
                let end = b.iter().position(|c| *c == 0).unwrap_or(b.len());
                String::from_utf8_lossy(&b[..end]).into_owned()
            }
            _ => String::new(),
        }
    };
    let one = |o: u32| match (word(o), word(o.wrapping_add(FADE_FLAGS))) {
        (Some(vtable), Some(flags)) => {
            format!(
                "{o:08x} vtable {vtable:08x} flags {flags:08x} name {:?}",
                name(o)
            )
        }
        _ => format!("{o:08x} (not readable)"),
    };
    let parent = word(object.wrapping_add(0x18)).unwrap_or(0);
    if parent == 0 {
        format!("{}; no parent", one(object))
    } else {
        format!("{}; parent {}", one(object), one(parent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A logged call's pointers may not lie in the helper's stack; its values (an alpha's bits)
    /// may, whatever they look like.
    #[test]
    fn logged_calls_check_only_their_pointers() {
        let mut e = Engine::new();
        e.with_stack(4, |_, _| ());
        let (low, high) = e.guest_stack_region().expect("a guest stack");
        let inside = low + (high - low) / 2;
        let node = 0x0123_4560;
        LOG.with(|log| *log.borrow_mut() = Some(Vec::new()));
        log_fade_alpha(&mut e, &[node, inside]);
        log_start_grouping(&mut e, &[0x0200_0000, 0x0200_0100, inside]);
        let logged = LOG.with(|log| log.borrow_mut().take()).unwrap_or_default();
        assert_eq!(
            logged,
            vec![
                Entry::Call {
                    addr: SET_FADE_ALPHA,
                    words: vec![node, inside],
                },
                Entry::Call {
                    addr: START_GROUPING_ALPHAS,
                    words: vec![0x0200_0000, 0x0200_0100, inside],
                },
            ]
        );
        for (f, args) in [
            (
                log_fade_alpha as fn(&mut Engine, &[u32]) -> Ret,
                vec![inside, ONE_BITS],
            ),
            (log_start_grouping, vec![0x0200_0000, inside, 1]),
            (log_stop_grouping, vec![inside]),
        ] {
            let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                f(&mut e, &args);
            }));
            assert!(
                stopped.is_err(),
                "a pointer into the stack stops: {args:x?}"
            );
        }
    }

    const ONE_BITS: u32 = 0x3f80_0000;

    /// By default three helpers from 8 logical processors up, none below (one helper did not
    /// pay on 4).
    #[test]
    fn default_helpers_need_eight_logical_processors() {
        for (logical, helpers) in [(1, 0), (4, 0), (7, 0), (8, 3), (24, 3)] {
            assert_eq!(
                default_helpers(logical),
                helpers,
                "{logical} logical processors"
            );
        }
    }
}
