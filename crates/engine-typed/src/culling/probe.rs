//! `NV_PAR_RENDER=time`: the first piece of the parallel-render design (P0 of DESIGN.md in the
//! measure folder parallel-render). A diagnostic of the host, not exe behaviour: it times the
//! culling walks and the registration inside them and changes nothing they do.
//!
//! A walk is an outermost `NiAVObject::Cull` or `BSCullingProcess::Process` on a thread (the
//! scene's root handed to the process), with everything under it. Each walk is filed by its root's
//! vtable and its process's (the main world's root is the shadow scene node; the water reflection,
//! the shadow maps and first person have their own roots or the same one, and some walk with
//! another process class), with its time, the time spent registering what it found (the typed
//! `AppendVirtual`, inclusive) and the geometries it registered. With the walk split over the
//! helpers (`NV_PAR_RENDER=1,time`) it also counts the splits: the children and log entries of
//! each, the time from fork to join, the replay's time, the time the walkers spent walking added
//! up, and the splits refused (for the process, for a compound frustum in use, for want of
//! helpers). Every five seconds the table so far is printed and cleared.
//!
//! Of a split it also counts what main's own walking took, how many helpers joined in and how
//! long after the fork each took its first task.
//!
//! **The outline.** After each table, the next walk of the table's longest row is recorded object
//! by object on main (every `OnVisible` but a geometry's, outside the split's tasks): its depth,
//! vtable, `OnVisible`, children and time, inclusive, whether it split, and whether the replay ran
//! it. The outline prints the objects that took 1% of the walk or more, and a sum of the nodes of
//! `NV_PAR_RENDER_MIN` or more children by `OnVisible` and whether they split (what the split
//! does not reach).
//!
//! Off (the default), each `Cull`, `Process`, `AppendVirtual` and `OnVisible` the typed process
//! hands on pays one relaxed load.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 0: not read yet, 1: off, 2: on.
static STATE: AtomicU8 = AtomicU8::new(0);

/// Whether `NV_PAR_RENDER` asks for the probe (`time`).
#[inline(always)]
pub(crate) fn on() -> bool {
    match STATE.load(Relaxed) {
        1 => false,
        2 => true,
        _ => read_switch(),
    }
}

#[cold]
fn read_switch() -> bool {
    let on = std::env::var("NV_PAR_RENDER")
        .is_ok_and(|v| v.split(',').any(|part| part.trim() == "time"));
    STATE.store(if on { 2 } else { 1 }, Relaxed);
    if on {
        println!(
            "NV_PAR_RENDER=time: culling walks and their registration timed, a table every 5 s"
        );
    }
    on
}

/// Why a split was refused.
#[derive(Clone, Copy)]
pub(crate) enum Refused {
    /// The process is not the Bethesda process with the typed `Process` and `AppendVirtual`.
    Process = 0,
    /// A compound frustum is in use.
    Frustum = 1,
    /// The helpers could not be had (a memory diagnostic keeps memory unshared).
    Helpers = 2,
}

/// What the splits of one walk add up to.
#[derive(Clone, Copy)]
struct Splits {
    count: u64,
    children: u64,
    entries: u64,
    fork_ns: u64,
    replay_ns: u64,
    busy_ns: u64,
    main_ns: u64,
    tasks: u64,
    joined: u64,
    wake_ns: u64,
    refused: [u64; 3],
}

impl Splits {
    const NONE: Splits = Splits {
        count: 0,
        children: 0,
        entries: 0,
        fork_ns: 0,
        replay_ns: 0,
        busy_ns: 0,
        main_ns: 0,
        tasks: 0,
        joined: 0,
        wake_ns: 0,
        refused: [0; 3],
    };
}

/// What one split took (`par::split`): its children, tasks and log entries; fork to join and the
/// replay; the walkers' time walking added up, main's part of it; the helpers that joined in and
/// the time from the fork to each one's first task, added up.
pub(crate) struct SplitTimes {
    pub children: usize,
    pub tasks: usize,
    pub entries: usize,
    pub fork: Duration,
    pub replay: Duration,
    pub busy_ns: u64,
    pub main_ns: u64,
    pub joined: u64,
    pub wake_ns: u64,
}

/// One object of an outline.
#[derive(Clone, Copy)]
struct Node {
    depth: u32,
    vtable: u32,
    target: u32,
    children: u32,
    ns: u64,
    split: bool,
    replayed: bool,
}

/// The outline being recorded on this thread: the objects so far, the open ones (index, start).
struct Outline {
    nodes: Vec<Node>,
    open: Vec<(usize, Instant)>,
}

/// An outline is wanted of the next walk with this key (set after each table).
static OUTLINE_WANTED: AtomicBool = AtomicBool::new(false);
static OUTLINE_KEY: Mutex<(u32, u32)> = Mutex::new((0, 0));

thread_local! {
    /// The walks open on this thread, the outermost one's start and key (root vtable, process
    /// vtable), and the registration time and count under it so far.
    static OPEN: Cell<u32> = const { Cell::new(0) };
    static START: Cell<Option<Instant>> = const { Cell::new(None) };
    static KEY: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
    static REG_NS: Cell<u64> = const { Cell::new(0) };
    static REG_COUNT: Cell<u64> = const { Cell::new(0) };
    static REG_OPEN: Cell<u32> = const { Cell::new(0) };
    /// The splits of the outermost walk so far.
    static SPLITS: Cell<Splits> = const { Cell::new(Splits::NONE) };
    /// The outline of the outermost walk, when one is recorded.
    static OUTLINE: RefCell<Option<Outline>> = const { RefCell::new(None) };
    /// A split's replay runs on this thread.
    static REPLAYING: Cell<bool> = const { Cell::new(false) };
}

/// The walks of one root and process since the last table.
#[derive(Clone, Copy)]
struct Row {
    walks: u64,
    walk_ns: u64,
    reg_ns: u64,
    registered: u64,
    longest_ns: u64,
    splits: Splits,
}

struct Table {
    since: Instant,
    rows: HashMap<(u32, u32), Row>,
}

static TABLE: Mutex<Option<Table>> = Mutex::new(None);

/// Held across a `Cull` or `Process` call while the probe is on.
pub(crate) struct Walk(bool);

/// A walk step starts on `root` with `process` (their vtables file the walk when it is the
/// outermost one).
pub(crate) fn walk_begins(e: &engine_core::Engine, root: u32, process: u32) -> Walk {
    // A chunk of a parallel walk is part of main's walk, timed there.
    if super::par::collecting() {
        return Walk(false);
    }
    let open = OPEN.with(|o| {
        let n = o.get();
        o.set(n + 1);
        n
    });
    if open == 0 {
        let key = (e.mem.u32(root), e.mem.u32(process));
        KEY.with(|k| k.set(key));
        if OUTLINE_WANTED.load(Relaxed)
            && *OUTLINE_KEY
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                == key
            && OUTLINE_WANTED.swap(false, Relaxed)
        {
            OUTLINE.with(|o| {
                *o.borrow_mut() = Some(Outline {
                    nodes: Vec::new(),
                    open: Vec::new(),
                })
            });
        }
        REG_NS.with(|r| r.set(0));
        REG_COUNT.with(|r| r.set(0));
        SPLITS.with(|s| s.set(Splits::NONE));
        START.with(|s| s.set(Some(Instant::now())));
    }
    Walk(true)
}

impl Drop for Walk {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        let open = OPEN.with(|o| {
            let n = o.get().saturating_sub(1);
            o.set(n);
            n
        });
        if open != 0 {
            return;
        }
        let Some(start) = START.with(Cell::take) else {
            return;
        };
        let walk_ns = nanos(start.elapsed());
        let key = KEY.with(Cell::get);
        let reg_ns = REG_NS.with(Cell::get);
        let registered = REG_COUNT.with(Cell::get);
        let splits = SPLITS.with(Cell::get);
        if let Some(outline) = OUTLINE.with(|o| o.borrow_mut().take()) {
            print_outline(&outline.nodes, key, walk_ns);
        }
        let mut guard = TABLE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let table = guard.get_or_insert_with(|| Table {
            since: Instant::now(),
            rows: HashMap::new(),
        });
        let row = table.rows.entry(key).or_insert(Row {
            walks: 0,
            walk_ns: 0,
            reg_ns: 0,
            registered: 0,
            longest_ns: 0,
            splits: Splits::NONE,
        });
        row.walks += 1;
        row.walk_ns += walk_ns;
        row.reg_ns += reg_ns;
        row.registered += registered;
        row.longest_ns = row.longest_ns.max(walk_ns);
        row.splits.count += splits.count;
        row.splits.children += splits.children;
        row.splits.entries += splits.entries;
        row.splits.fork_ns += splits.fork_ns;
        row.splits.replay_ns += splits.replay_ns;
        row.splits.busy_ns += splits.busy_ns;
        row.splits.main_ns += splits.main_ns;
        row.splits.tasks += splits.tasks;
        row.splits.joined += splits.joined;
        row.splits.wake_ns += splits.wake_ns;
        for (sum, n) in row.splits.refused.iter_mut().zip(splits.refused) {
            *sum += n;
        }
        if table.since.elapsed() >= Duration::from_secs(5) {
            print_table(table);
            table.rows.clear();
            table.since = Instant::now();
        }
    }
}

/// A split of the walk open on this thread was refused.
pub(crate) fn split_refused(why: Refused) {
    SPLITS.with(|s| {
        let mut splits = s.get();
        splits.refused[why as usize] += 1;
        s.set(splits);
    });
}

/// A split of the walk open on this thread was walked ([`SplitTimes`]).
pub(crate) fn split_walked(t: &SplitTimes) {
    SPLITS.with(|s| {
        let mut splits = s.get();
        splits.count += 1;
        splits.children += t.children as u64;
        splits.tasks += t.tasks as u64;
        splits.entries += t.entries as u64;
        splits.fork_ns += nanos(t.fork);
        splits.replay_ns += nanos(t.replay);
        splits.busy_ns += t.busy_ns;
        splits.main_ns += t.main_ns;
        splits.joined += t.joined;
        splits.wake_ns += t.wake_ns;
        s.set(splits);
    });
    // The outline's innermost open object is the node that split.
    OUTLINE.with(|o| {
        if let Some(o) = o.borrow_mut().as_mut() {
            if let Some(&(at, _)) = o.open.last() {
                o.nodes[at].split = true;
            }
        }
    });
}

/// Held across an object's `OnVisible` while the probe is on (see [`object_begins`]).
pub(crate) struct Object(bool);

/// `object`'s `OnVisible` (`target`) starts on this thread: recorded when an outline is being
/// recorded here and this thread does not walk a split's task.
pub(crate) fn object_begins(e: &engine_core::Engine, object: u32, target: u32) -> Object {
    if super::par::collecting() {
        return Object(false);
    }
    OUTLINE.with(|o| {
        let mut o = o.borrow_mut();
        let Some(o) = o.as_mut() else {
            return Object(false);
        };
        let children = u32::from(e.get(
            engine_core::ptr::Ptr::<super::layout::NiNode>::new(object),
            super::layout::NiNode::m_usSize,
        ));
        let at = o.nodes.len();
        o.nodes.push(Node {
            depth: u32::try_from(o.open.len()).unwrap_or(u32::MAX),
            vtable: e.mem.u32(object),
            target,
            children,
            ns: 0,
            split: false,
            replayed: REPLAYING.with(Cell::get),
        });
        o.open.push((at, Instant::now()));
        Object(true)
    })
}

impl Drop for Object {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        OUTLINE.with(|o| {
            if let Some(o) = o.borrow_mut().as_mut() {
                if let Some((at, start)) = o.open.pop() {
                    o.nodes[at].ns = nanos(start.elapsed());
                }
            }
        });
    }
}

/// Held across a split's replay while the probe is on: what it runs is marked in an outline.
pub(crate) struct Replay(bool);

pub(crate) fn replay_begins() -> Replay {
    Replay(REPLAYING.with(|r| r.replace(true)))
}

impl Drop for Replay {
    fn drop(&mut self) {
        REPLAYING.with(|r| r.set(self.0));
    }
}

/// Prints an outline: the objects of 1% of the walk or more (at most 60), then the nodes of
/// `NV_PAR_RENDER_MIN`+ children walked on main, by vtable and `OnVisible`, split or not.
fn print_outline(nodes: &[Node], (root, process): (u32, u32), walk_ns: u64) {
    let least = super::par::least_children();
    let mut out = format!(
        "NV_PAR_RENDER outline of a walk (root vtable {root:08x} process {process:08x}): {:.1} us, {} objects on main \
         (geometries not counted); depth, vtable, OnVisible, children, us inclusive:",
        walk_ns as f64 / 1000.0,
        nodes.len()
    );
    let mut shown = 0;
    for n in nodes {
        if n.ns * 100 < walk_ns || shown == 60 {
            continue;
        }
        shown += 1;
        out.push_str(&format!(
            "\n  {:indent$}{} {:08x} {:08x} {} {:.1}{}{}",
            "",
            n.depth,
            n.vtable,
            n.target,
            n.children,
            n.ns as f64 / 1000.0,
            if n.split { " SPLIT" } else { "" },
            if n.replayed { " (replay)" } else { "" },
            indent = 2 * n.depth.min(12) as usize,
        ));
    }
    // The big nodes by kind: (vtable, OnVisible, split) -> (count, children, ns).
    let mut big: HashMap<(u32, u32, bool), (u64, u64, u64)> = HashMap::new();
    for n in nodes.iter().filter(|n| n.children >= least && !n.replayed) {
        let sum = big.entry((n.vtable, n.target, n.split)).or_default();
        sum.0 += 1;
        sum.1 += u64::from(n.children);
        sum.2 += n.ns;
    }
    let mut big: Vec<_> = big.into_iter().collect();
    big.sort_by_key(|(_, (_, _, ns))| std::cmp::Reverse(*ns));
    out.push_str(&format!(
        "\n  nodes of {least}+ children on main, by vtable and OnVisible:"
    ));
    for ((vtable, target, split), (count, children, ns)) in big {
        out.push_str(&format!(
            "\n    {vtable:08x} {target:08x} {}: {count} nodes, {:.0} children each, {:.1} us",
            if split { "split" } else { "NOT split" },
            children as f64 / count as f64,
            ns as f64 / 1000.0
        ));
    }
    println!("{out}");
}

/// Held across an `AppendVirtual` call while the probe is on.
pub(crate) struct Register(Option<Instant>);

/// A registration starts (only the outermost one on this thread is timed).
pub(crate) fn register_begins() -> Register {
    let open = REG_OPEN.with(|o| {
        let n = o.get();
        o.set(n + 1);
        n
    });
    Register((open == 0).then(Instant::now))
}

impl Drop for Register {
    fn drop(&mut self) {
        REG_OPEN.with(|o| o.set(o.get().saturating_sub(1)));
        if let Some(start) = self.0 {
            let ns = nanos(start.elapsed());
            REG_NS.with(|r| r.set(r.get() + ns));
            REG_COUNT.with(|r| r.set(r.get() + 1));
        }
    }
}

pub(crate) fn nanos(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

fn print_table(table: &Table) {
    let secs = table.since.elapsed().as_secs_f64();
    let mut rows: Vec<((u32, u32), Row)> = table.rows.iter().map(|(k, v)| (*k, *v)).collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.1.walk_ns));
    let mut out = format!("NV_PAR_RENDER probe, {secs:.1} s:");
    for ((root, process), r) in rows {
        let per = |ns: u64| ns as f64 / r.walks.max(1) as f64 / 1000.0;
        out.push_str(&format!(
            "\n  root vtable {root:08x} process {process:08x}: {:.1} walks a second, a walk {:.1} us (without registration {:.1} us, \
             registration {:.1} us for {:.0} geometries), longest {:.1} us",
            r.walks as f64 / secs,
            per(r.walk_ns),
            per(r.walk_ns.saturating_sub(r.reg_ns)),
            per(r.reg_ns),
            r.registered as f64 / r.walks.max(1) as f64,
            r.longest_ns as f64 / 1000.0,
        ));
        let s = r.splits;
        if s.count != 0 || s.refused.iter().any(|n| *n != 0) {
            let each = |n: u64| n as f64 / s.count.max(1) as f64;
            let a_walk = |n: u64| n as f64 / r.walks.max(1) as f64;
            out.push_str(&format!(
                "\n    splits {:.2} a walk ({:.0} children, {:.0} entries each): fork to join {:.1} us a walk, \
                 replay {:.1} us (its registration included), walkers busy {:.1} us; refused a walk: \
                 process {:.2}, compound frustum {:.2}, no helpers {:.2}",
                a_walk(s.count),
                each(s.children),
                each(s.entries),
                per(s.fork_ns),
                per(s.replay_ns),
                per(s.busy_ns),
                a_walk(s.refused[0]),
                a_walk(s.refused[1]),
                a_walk(s.refused[2]),
            ));
            out.push_str(&format!(
                "\n      tasks {:.1} a split; main walked {:.1} us a walk of it; helpers that took a task {:.2} a split, \
                 each its first {:.1} us after the split was made",
                each(s.tasks),
                per(s.main_ns),
                each(s.joined),
                s.wake_ns as f64 / s.joined.max(1) as f64 / 1000.0,
            ));
        }
    }
    // The next walk of the longest row gets an outline.
    if let Some(((root, process), _)) = rows_longest(table) {
        *OUTLINE_KEY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = (root, process);
        OUTLINE_WANTED.store(true, Relaxed);
    }
    println!("{out}");
}

/// The table's row with the most walking time.
fn rows_longest(table: &Table) -> Option<((u32, u32), Row)> {
    table
        .rows
        .iter()
        .max_by_key(|(_, r)| r.walk_ns)
        .map(|(k, r)| (*k, *r))
}
