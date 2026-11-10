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
//! Off (the default), each `Cull`, `Process` and `AppendVirtual` pays one relaxed load.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};
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
        refused: [0; 3],
    };
}

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
        KEY.with(|k| k.set((e.mem.u32(root), e.mem.u32(process))));
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

/// A split of the walk open on this thread was walked: its children and log entries, the time
/// from fork to join, the replay's, and the walkers' time walking added up.
pub(crate) fn split_walked(
    children: usize,
    entries: usize,
    fork: Duration,
    replay: Duration,
    busy_ns: u64,
) {
    SPLITS.with(|s| {
        let mut splits = s.get();
        splits.count += 1;
        splits.children += children as u64;
        splits.entries += entries as u64;
        splits.fork_ns += nanos(fork);
        splits.replay_ns += nanos(replay);
        splits.busy_ns += busy_ns;
        s.set(splits);
    });
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
        }
    }
    println!("{out}");
}
