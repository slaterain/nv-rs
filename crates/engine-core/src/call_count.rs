//! `NV_CALL_COUNT=1`: counts the calls that really go by address in play, through
//! [`Engine::call`](crate::Engine::call) and [`Engine::vcall`](crate::Engine::vcall) (a translated
//! caller, a virtual call, a `typed_call!` whose family is off or not direct, an import), and
//! leaves the typed families' direct calls direct. `NV_PROFILE` cannot tell these apart: it is a
//! diagnostic, and while a diagnostic is on every typed call goes by address so that the
//! diagnostic sees it. This counter is not a diagnostic: it changes nothing the game or the
//! engine does, only adds to its own tables.
//!
//! Each call by address is counted by its target and by its edge (the nearest caller that was
//! itself called by address, the top of the engine's call stack, and the target), on main and on
//! the other game threads apart. The calls made directly but found by their address
//! ([`count_direct`]: `Engine::call_typed`, the drawing's direct device calls) are counted by
//! target only, in rows of their own marked "direct". Every `NV_CALL_COUNT_SECS` seconds (default
//! 5) it prints the `NV_CALL_COUNT_ROWS` (default 60) targets and edges main calls most, in calls a
//! frame (a frame being one of the host's main frames, [`frame_ended`]), and starts counting again.
//! The counts are relaxed atomic adds in two tables every thread shares (65,536 slots each, open
//! addressing; a key that finds no free slot in 64 steps is counted in the header's "not counted"
//! figures, by target and by edge).
//!
//! The switch is read once per engine, when it is made: it sets a bit in the byte of diagnostics
//! [`Engine::call`]'s fast path already tests, so off (the default) the call path is exactly what it
//! is without the counter. On, every call by address takes the full path, where it is counted
//! (calls cost more, so the game runs slower while counting; the counts a frame are what they
//! are). With `NV_PROFILE` on as well the typed calls go by address again and are counted too.

use std::fmt::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::OnceLock;
use std::time::Duration;

use crate::Engine;

/// `NV_CALL_COUNT=1`: read by each engine when it is made, and at each frame's end.
static ON: AtomicBool = AtomicBool::new(false);
/// The host's main frames ended since the last report ([`frame_ended`]).
static FRAMES: AtomicU64 = AtomicU64::new(0);
const SLOTS: usize = 1 << 16;
const PROBES: usize = 64;
/// A slot no key holds yet (no edge or target is this: the top bit is a caller's, and no caller
/// and target are both `ffffffff`).
const EMPTY: u64 = u64::MAX;
/// A target key's mark for a call made directly but found by its address ([`count_direct`]).
const DIRECT: u64 = 1 << 32;

struct Slot {
    key: AtomicU64,
    main: AtomicU64,
    other: AtomicU64,
}

/// One table of counts by key: targets (the exe address, [`DIRECT`] for a direct call) or edges
/// (caller << 32 | target).
struct Table {
    slots: Box<[Slot]>,
    lost_main: AtomicU64,
    lost_other: AtomicU64,
}

impl Table {
    fn new() -> Table {
        Table {
            slots: (0..SLOTS)
                .map(|_| Slot {
                    key: AtomicU64::new(EMPTY),
                    main: AtomicU64::new(0),
                    other: AtomicU64::new(0),
                })
                .collect(),
            lost_main: AtomicU64::new(0),
            lost_other: AtomicU64::new(0),
        }
    }

    /// Counts one call under `key`, on main or on another thread.
    fn add(&self, key: u64, main: bool) {
        let mut i = (key.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 48) as usize & (SLOTS - 1);
        for _ in 0..PROBES {
            let slot = &self.slots[i];
            let held = slot.key.load(Relaxed);
            let mine = held == key
                || (held == EMPTY
                    && match slot.key.compare_exchange(EMPTY, key, Relaxed, Relaxed) {
                        Ok(_) => true,
                        Err(now) => now == key,
                    });
            if mine {
                let n = if main { &slot.main } else { &slot.other };
                n.fetch_add(1, Relaxed);
                return;
            }
            i = (i + 1) & (SLOTS - 1);
        }
        let n = if main {
            &self.lost_main
        } else {
            &self.lost_other
        };
        n.fetch_add(1, Relaxed);
    }

    /// The counts so far, cleared (the keys keep their slots): `(key, main, other)` of every key
    /// counted since the last time, and the calls no slot took on main and elsewhere.
    fn take(&self) -> Taken {
        let rows = self
            .slots
            .iter()
            .filter_map(|slot| {
                let key = slot.key.load(Relaxed);
                let main = slot.main.swap(0, Relaxed);
                let other = slot.other.swap(0, Relaxed);
                (key != EMPTY && main + other != 0).then_some((key, main, other))
            })
            .collect();
        Taken {
            rows,
            lost: (
                self.lost_main.swap(0, Relaxed),
                self.lost_other.swap(0, Relaxed),
            ),
        }
    }
}

/// One table's counts since the last report ([`Table::take`]).
struct Taken {
    rows: Vec<(u64, u64, u64)>,
    lost: (u64, u64),
}

fn targets() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(Table::new)
}

fn edges() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(Table::new)
}

/// Whether the counter is on (`NV_CALL_COUNT=1`).
#[inline(always)]
pub fn on() -> bool {
    ON.load(Relaxed)
}

/// Whether `e`'s thread is the game's main thread.
fn on_main(e: &Engine) -> bool {
    crate::thread_host::current_id(e) == crate::thread_host::MAIN_THREAD_ID
}

/// Counts one call of `addr` by address on `e`'s thread (the full path, `Engine::run`, of an engine
/// counting its calls), before the engine pushes it on its call stack.
#[cold]
#[inline(never)]
pub(crate) fn count(e: &Engine, addr: u32) {
    let main = on_main(e);
    let caller = e.call_stack.last().copied().unwrap_or(0);
    targets().add(u64::from(addr), main);
    edges().add(u64::from(caller) << 32 | u64::from(addr), main);
}

/// Counts one call of `addr` made directly but found by its address on `e`'s thread: the call
/// cache's look-up of [`Engine::call_typed`], the drawing's direct device calls
/// (`crate::real_threads`). By target only, in a row of its own.
#[cold]
#[inline(never)]
pub(crate) fn count_direct(e: &Engine, addr: u32) {
    targets().add(u64::from(addr) | DIRECT, on_main(e));
}

/// The host has a main frame back ([`crate::thread_host::finish_main_frame`]): one frame more.
#[inline]
pub fn frame_ended() {
    if on() {
        FRAMES.fetch_add(1, Relaxed);
    }
}

/// The table of the counts since the last one, `rows` targets and edges, and starts again.
pub fn report(rows: usize) -> String {
    let frames = FRAMES.swap(0, Relaxed);
    render(targets().take(), edges().take(), frames, rows)
}

/// [`report`]'s text from the counts taken: targets and edges sorted by main's calls (then the
/// other threads', then the key), each with main's and the other threads' calls a frame.
fn render(targets: Taken, edges: Taken, frames: u64, rows: usize) -> String {
    let per_frame = |calls: u64| calls as f64 / frames.max(1) as f64;
    // Main's and the other threads' calls by address (the lost ones too), then the direct ones.
    let mut by_address = targets.lost;
    let mut direct = (0, 0);
    for &(key, main, other) in &targets.rows {
        let sum = if key & DIRECT != 0 {
            &mut direct
        } else {
            &mut by_address
        };
        *sum = (sum.0 + main, sum.1 + other);
    }
    let all_main = by_address.0 + direct.0;
    let sorted = |mut t: Vec<(u64, u64, u64)>| {
        t.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
        t
    };
    let kind =
        |addr: u32| crate::profile::kind_of(addr).map_or(String::new(), |k| format!("  {k}"));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "NV_CALL_COUNT: calls by address only (typed direct calls are not counted), {frames} frames: \
         {:.0} a frame on main, {:.0} on the other threads; found by address and made directly \
         {:.0} on main, {:.0} on the other threads; {} targets ({:.0} a frame not counted by \
         target, {:.0} by edge)",
        per_frame(by_address.0),
        per_frame(by_address.1),
        per_frame(direct.0),
        per_frame(direct.1),
        targets.rows.len(),
        per_frame(targets.lost.0 + targets.lost.1),
        per_frame(edges.lost.0 + edges.lost.1),
    );
    let _ = writeln!(
        out,
        "  top {rows} targets by main's calls (calls a frame on main, on the other threads, % of \
         main's, target):"
    );
    for (key, m, o) in sorted(targets.rows).into_iter().take(rows) {
        let addr = key as u32;
        let _ = writeln!(
            out,
            "  {:10.1} {:10.1} {:6.2}%  {addr:08x}{}{}",
            per_frame(m),
            per_frame(o),
            m as f64 * 100.0 / all_main.max(1) as f64,
            if key & DIRECT != 0 { "  direct" } else { "" },
            kind(addr),
        );
    }
    let _ = writeln!(
        out,
        "  top {rows} edges by main's calls (calls a frame on main, on the other threads, nearest \
         caller called by address or 00000000 for none, target):"
    );
    for (key, m, o) in sorted(edges.rows).into_iter().take(rows) {
        let _ = writeln!(
            out,
            "  {:10.1} {:10.1}  {:08x} -> {:08x}",
            per_frame(m),
            per_frame(o),
            (key >> 32) as u32,
            key as u32,
        );
    }
    out
}

fn env_number(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(default)
}

/// Turns the counter on when `NV_CALL_COUNT=1` (each `Engine::new` asks; only the first reads it),
/// with its reporting thread.
pub fn init_from_env() {
    static READ: OnceLock<()> = OnceLock::new();
    READ.get_or_init(|| {
        if std::env::var("NV_CALL_COUNT").as_deref() != Ok("1") {
            return;
        }
        let secs = env_number("NV_CALL_COUNT_SECS", 5);
        let rows = env_number("NV_CALL_COUNT_ROWS", 60) as usize;
        let _ = (targets(), edges());
        ON.store(true, Relaxed);
        let _ = std::thread::Builder::new()
            .name("nv-call-count".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(secs));
                print!("{}", report(rows));
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::Ret;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    /// Targets no other test calls.
    const A: u32 = 0x7ffe_0010;
    const B: u32 = 0x7ffe_0020;
    const C: u32 = 0x7ffe_0030;
    const D: u32 = 0x7ffe_0040;

    /// One of these tests at a time: the tables and the frames are the whole program's, and a
    /// report clears them.
    fn one_at_a_time() -> MutexGuard<'static, ()> {
        static ONE: Mutex<()> = Mutex::new(());
        ONE.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn target_calls(key: u64) -> (u64, u64) {
        targets()
            .slots
            .iter()
            .find(|s| s.key.load(Relaxed) == key)
            .map_or((0, 0), |s| (s.main.load(Relaxed), s.other.load(Relaxed)))
    }

    fn edge_calls(caller: u32, addr: u32) -> u64 {
        let key = u64::from(caller) << 32 | u64::from(addr);
        edges()
            .slots
            .iter()
            .find(|s| s.key.load(Relaxed) == key)
            .map_or(0, |s| s.main.load(Relaxed))
    }

    fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
        Ret::default()
    }

    /// B calls A by address twice.
    fn b_calls_a(e: &mut Engine, _: &[u32]) -> Ret {
        e.call(A, &[]);
        e.call(A, &[]);
        Ret::default()
    }

    /// Off, a call counts nothing and the engine's fast path is its own (no diagnostic bit); on,
    /// each call by address (`call` and `vcall`) counts once on main by target and by edge (the
    /// caller on the call stack), and the typed seam still sees no diagnostic; off again, nothing.
    #[test]
    fn counts_calls_by_address_only_while_on() {
        let _one = one_at_a_time();
        let mut e = Engine::new();
        e.register(A, nothing);
        e.register(B, b_calls_a);
        let object = e.mem.alloc(8);
        let vtable = e.mem.alloc(8);
        e.mem.set_u32(object, vtable);
        e.mem.set_u32(vtable + 4, A);
        e.call(A, &[]);
        assert!(!e.diagnosing());
        assert_eq!(target_calls(A.into()), (0, 0), "off: nothing is counted");
        e.set_count_calls(true);
        assert!(e.diagnosing() && !e.diagnosing_direct());
        e.call(A, &[]);
        e.call(B, &[]);
        e.vcall(object, 4, &[]);
        e.set_count_calls(false);
        e.call(B, &[]);
        assert_eq!(
            target_calls(A.into()),
            (4, 0),
            "one plain, two from B, one virtual"
        );
        assert_eq!(target_calls(B.into()), (1, 0));
        assert_eq!(edge_calls(B, A), 2, "B -> A");
        assert_eq!(
            edge_calls(0, A),
            2,
            "the test -> A (no caller on the stack)"
        );
    }

    /// A game thread's calls count as the other threads', main's as main's.
    #[test]
    fn a_game_threads_calls_count_apart_from_mains() {
        let _one = one_at_a_time();
        let mut e = Engine::new();
        e.register(C, nothing);
        crate::thread_host::create(&mut e, 0, 0, 0, true);
        e.set_count_calls(true);
        e.call(C, &[]);
        e.kernel.threads.current = 1;
        assert_ne!(
            crate::thread_host::current_id(&e),
            crate::thread_host::MAIN_THREAD_ID
        );
        for _ in 0..3 {
            e.call(C, &[]);
        }
        e.kernel.threads.current = 0;
        assert_eq!(target_calls(C.into()), (1, 3));
    }

    /// A call `Engine::call_typed` makes directly once the cache has its address stays direct
    /// while counting and counts in a direct row of its own, by target only; its first call (not
    /// cached yet) goes by address and counts as one.
    #[test]
    fn a_call_typed_call_stays_direct_and_counts_as_direct() {
        let _one = one_at_a_time();
        let mut e = Engine::new();
        e.register(D, nothing);
        e.set_count_calls(true);
        let mut direct = 0;
        for _ in 0..3 {
            e.call_typed(D, nothing, &[], |_| {
                direct += 1;
                0
            });
        }
        assert_eq!(direct, 2, "the first by address, the rest direct");
        assert_eq!(target_calls(D.into()), (1, 0), "the first, by address");
        assert_eq!(target_calls(u64::from(D) | DIRECT), (2, 0), "the rest");
        assert_eq!(edge_calls(0, D), 1);
    }

    /// `frame_ended` counts frames only while on, and the report divides by them and starts again.
    #[test]
    fn frames_count_while_on_and_divide_the_report() {
        let _one = one_at_a_time();
        let _ = report(1);
        frame_ended();
        ON.store(true, Relaxed);
        for _ in 0..4 {
            frame_ended();
        }
        ON.store(false, Relaxed);
        frame_ended();
        targets().add(0x7ffe_0050, true);
        targets().add(0x7ffe_0050, true);
        let text = report(100_000);
        let again = report(1);
        assert!(text.contains(" 4 frames: "), "{text}");
        let row = text.lines().find(|l| l.contains("7ffe0050")).unwrap_or("");
        assert!(
            row.starts_with("         0.5        0.0"),
            "two calls over 4 frames\n{text}"
        );
        assert!(again.contains(" 0 frames: "), "started again\n{again}");
    }

    /// The report reads calls a frame on main and elsewhere, sorted by main's calls, with every
    /// call (those no slot took too) in the totals, and the direct rows marked.
    #[test]
    fn the_report_is_calls_a_frame_sorted_by_mains() {
        let targets = Taken {
            rows: vec![
                (0x0055_9450, 20, 4),
                (0x00fd_f1c0, 60, 0),
                (0x006c_3640, 0, 80),
                (0x00fd_f1c4 | DIRECT, 40, 8),
            ],
            lost: (20, 0),
        };
        let edges = Taken {
            rows: vec![(0x0086_e650_0000_0000 | 0x0055_9450, 20, 4)],
            lost: (8, 4),
        };
        let text = render(targets, edges, 4, 3);
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            lines[0].contains(
                "4 frames: 25 a frame on main, 21 on the other threads; found by address and \
                 made directly 10 on main, 2 on the other threads; 4 targets (5 a frame not \
                 counted by target, 3 by edge)"
            ),
            "{text}"
        );
        assert!(
            lines[2].contains("15.0        0.0  42.86%  00fdf1c0"),
            "{text}"
        );
        assert!(
            lines[3].contains("10.0        2.0  28.57%  00fdf1c4  direct"),
            "{text}"
        );
        assert!(
            lines[4].contains("5.0        1.0  14.29%  00559450"),
            "{text}"
        );
        assert!(lines[5].starts_with("  top 3 edges"), "{text}");
        assert!(
            lines[6].contains("5.0        1.0  0086e650 -> 00559450"),
            "{text}"
        );
        assert_eq!(lines.len(), 7, "{text}");
    }
}
