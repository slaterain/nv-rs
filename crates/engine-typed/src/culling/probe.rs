//! `NV_PAR_RENDER=time`: the first piece of the parallel-render design (P0 of DESIGN.md in the
//! measure folder parallel-render). A diagnostic of the host, not exe behaviour: it times the
//! culling walks and the registration inside them and changes nothing they do.
//!
//! A walk is an outermost `BSCullingProcess::Process` on a thread (the scene's root handed to the
//! process), with every `Process` under it. Each walk is filed by its root's vtable (the main world's
//! root is the shadow scene node; the water reflection, the shadow maps and first person have their
//! own roots or the same one), with its time, the time spent registering what it found
//! (`AppendVirtual`, inclusive) and the geometries it registered. Every five seconds the table so far
//! is printed and cleared: walks a second, and per walk the time with and without registration.
//!
//! Off (the default), each `Process` and `AppendVirtual` pays one relaxed load.

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
    let on = std::env::var("NV_PAR_RENDER").is_ok_and(|v| v.trim() == "time");
    STATE.store(if on { 2 } else { 1 }, Relaxed);
    if on {
        println!(
            "NV_PAR_RENDER=time: culling walks and their registration timed, a table every 5 s"
        );
    }
    on
}

thread_local! {
    /// The `Process` calls open on this thread, the outermost one's start, root vtable, and the
    /// registration time and count under it so far.
    static OPEN: Cell<u32> = const { Cell::new(0) };
    static START: Cell<Option<Instant>> = const { Cell::new(None) };
    static ROOT: Cell<u32> = const { Cell::new(0) };
    static REG_NS: Cell<u64> = const { Cell::new(0) };
    static REG_COUNT: Cell<u64> = const { Cell::new(0) };
    static REG_OPEN: Cell<u32> = const { Cell::new(0) };
}

/// The walks of one root vtable since the last table.
#[derive(Default, Clone, Copy)]
struct Root {
    walks: u64,
    walk_ns: u64,
    reg_ns: u64,
    registered: u64,
    longest_ns: u64,
}

struct Table {
    since: Instant,
    roots: HashMap<u32, Root>,
}

static TABLE: Mutex<Option<Table>> = Mutex::new(None);

/// Held across a `Process` call while the probe is on.
pub(crate) struct Walk(());

/// A `Process` call starts on `root` (the object handed to the process; its vtable files the walk
/// when it is the outermost one).
pub(crate) fn walk_begins(e: &engine_core::Engine, root: u32) -> Walk {
    let open = OPEN.with(|o| {
        let n = o.get();
        o.set(n + 1);
        n
    });
    if open == 0 {
        ROOT.with(|r| r.set(e.mem.u32(root)));
        REG_NS.with(|r| r.set(0));
        REG_COUNT.with(|r| r.set(0));
        START.with(|s| s.set(Some(Instant::now())));
    }
    Walk(())
}

impl Drop for Walk {
    fn drop(&mut self) {
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
        let root = ROOT.with(Cell::get);
        let reg_ns = REG_NS.with(Cell::get);
        let registered = REG_COUNT.with(Cell::get);
        let mut guard = TABLE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let table = guard.get_or_insert_with(|| Table {
            since: Instant::now(),
            roots: HashMap::new(),
        });
        let row = table.roots.entry(root).or_default();
        row.walks += 1;
        row.walk_ns += walk_ns;
        row.reg_ns += reg_ns;
        row.registered += registered;
        row.longest_ns = row.longest_ns.max(walk_ns);
        if table.since.elapsed() >= Duration::from_secs(5) {
            print_table(table);
            table.roots.clear();
            table.since = Instant::now();
        }
    }
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

fn nanos(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

fn print_table(table: &Table) {
    let secs = table.since.elapsed().as_secs_f64();
    let mut rows: Vec<(u32, Root)> = table.roots.iter().map(|(k, v)| (*k, *v)).collect();
    rows.sort_by(|a, b| b.1.walk_ns.cmp(&a.1.walk_ns));
    let mut out = format!("NV_PAR_RENDER probe, {secs:.1} s:");
    for (root, r) in rows {
        let per = |ns: u64| ns as f64 / r.walks.max(1) as f64 / 1000.0;
        out.push_str(&format!(
            "\n  root vtable {root:08x}: {:.1} walks a second, a walk {:.1} us (without registration {:.1} us, \
             registration {:.1} us for {:.0} geometries), longest {:.1} us",
            r.walks as f64 / secs,
            per(r.walk_ns),
            per(r.walk_ns.saturating_sub(r.reg_ns)),
            per(r.reg_ns),
            r.registered as f64 / r.walks.max(1) as f64,
            r.longest_ns as f64 / 1000.0,
        ));
    }
    println!("{out}");
}
