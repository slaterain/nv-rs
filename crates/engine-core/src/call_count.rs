//! `NV_CALL_COUNT=1`: counts the calls that really go by address in play, through
//! [`Engine::call`](crate::Engine::call) and [`Engine::vcall`](crate::Engine::vcall) (a translated
//! caller, a virtual call, a `typed_call!` whose family is off or not direct, an import), and
//! leaves the typed families' direct calls direct. `NV_PROFILE` cannot tell these apart: it is a
//! diagnostic, and while a diagnostic is on every typed call goes by address so that the
//! diagnostic sees it. This counter is not a diagnostic: it changes nothing the game or the
//! engine does, only adds to its own table.
//!
//! Each call is counted by its target and by its edge (the nearest caller that was itself called
//! by address, the top of the engine's call stack, and the target), on main and on the other game
//! threads apart. Every `NV_CALL_COUNT_SECS` seconds (default 5) it prints the
//! `NV_CALL_COUNT_ROWS` (default 60) targets and edges main calls most, in calls a frame (a frame
//! being one of the host's main frames, [`frame_ended`]), and starts counting again. The counts are
//! relaxed atomic adds in two tables every thread shares (65,536 slots each, open addressing; a key
//! that finds no free slot in 64 steps is counted as "not counted by target").
//!
//! Off (the default) a call pays one relaxed load of a static and a branch not taken; the counting
//! itself is out of line.

use std::fmt::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::OnceLock;
use std::time::Duration;

use crate::Engine;

static ON: AtomicBool = AtomicBool::new(false);
/// The host's main frames ended since the last report ([`frame_ended`]).
static FRAMES: AtomicU64 = AtomicU64::new(0);
const SLOTS: usize = 1 << 16;
const PROBES: usize = 64;
/// A slot no key holds yet (no edge or target is this: the top bit is a caller's, and no caller
/// and target are both `ffffffff`).
const EMPTY: u64 = u64::MAX;

struct Slot {
    key: AtomicU64,
    main: AtomicU64,
    other: AtomicU64,
}

/// One table of counts by key: targets (the exe address) or edges (caller << 32 | target).
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

/// Whether the counter is on: one relaxed load.
#[inline(always)]
pub fn on() -> bool {
    ON.load(Relaxed)
}

/// Counts one call of `addr` by address on `e`'s thread (out of line: only reached with the
/// counter on), before the engine pushes it on its call stack.
#[cold]
#[inline(never)]
pub(crate) fn count(e: &Engine, addr: u32) {
    let main = crate::thread_host::current_id(e) == crate::thread_host::MAIN_THREAD_ID;
    let caller = e.call_stack.last().copied().unwrap_or(0);
    targets().add(u64::from(addr), main);
    edges().add(u64::from(caller) << 32 | u64::from(addr), main);
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
    let sum = |t: &Taken| {
        t.rows
            .iter()
            .fold(t.lost, |(m, o), &(_, main, other)| (m + main, o + other))
    };
    let (main, other) = sum(&targets);
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
         {:.0} a frame on main, {:.0} on the other threads, {} targets ({:.0} a frame not counted \
         by target)",
        per_frame(main),
        per_frame(other),
        targets.rows.len(),
        per_frame(targets.lost.0 + targets.lost.1),
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
            "  {:10.1} {:10.1} {:6.2}%  {addr:08x}{}",
            per_frame(m),
            per_frame(o),
            m as f64 * 100.0 / main.max(1) as f64,
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

    /// Targets no other test calls.
    const A: u32 = 0x7ffe_0010;
    const B: u32 = 0x7ffe_0020;

    fn target_calls(addr: u32) -> (u64, u64) {
        targets()
            .slots
            .iter()
            .find(|s| s.key.load(Relaxed) == u64::from(addr))
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

    /// B calls A by address twice.
    fn b_calls_a(e: &mut Engine, _: &[u32]) -> Ret {
        e.call(A, &[]);
        e.call(A, &[]);
        Ret::default()
    }

    /// Off, a call counts nothing; on, each call by address (`call` and `vcall`) counts once on
    /// main by target and by edge (the caller on the call stack); off again, nothing more.
    #[test]
    fn counts_calls_by_address_only_while_on() {
        let mut e = Engine::new();
        e.register(A, |_, _| Ret::default());
        e.register(B, b_calls_a);
        let object = e.mem.alloc(8);
        let vtable = e.mem.alloc(8);
        e.mem.set_u32(object, vtable);
        e.mem.set_u32(vtable + 4, A);
        e.call(A, &[]);
        assert_eq!(target_calls(A), (0, 0), "off: nothing is counted");
        // The tables made first: the other tests' calls counted while this one has the counter
        // on allocate nothing on their threads.
        let _ = (targets(), edges());
        ON.store(true, Relaxed);
        e.call(A, &[]);
        e.call(B, &[]);
        e.vcall(object, 4, &[]);
        ON.store(false, Relaxed);
        e.call(B, &[]);
        assert_eq!(
            target_calls(A),
            (4, 0),
            "one plain, two from B, one virtual"
        );
        assert_eq!(target_calls(B), (1, 0));
        assert_eq!(edge_calls(B, A), 2, "B -> A");
        assert_eq!(
            edge_calls(0, A),
            2,
            "the test -> A (no caller on the stack)"
        );
    }

    /// The report reads calls a frame on main and elsewhere, sorted by main's calls, with every
    /// call (those no slot took too) in the totals.
    #[test]
    fn the_report_is_calls_a_frame_sorted_by_mains() {
        let targets = Taken {
            rows: vec![
                (0x0055_9450, 20, 4),
                (0x00fd_f1c0, 60, 0),
                (0x006c_3640, 0, 80),
            ],
            lost: (20, 0),
        };
        let edges = Taken {
            rows: vec![(0x0086_e650_0000_0000 | 0x0055_9450, 20, 4)],
            lost: (0, 0),
        };
        let text = render(targets, edges, 4, 2);
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            lines[0].contains("4 frames: 25 a frame on main, 21 on the other threads, 3 targets (5 a frame not counted"),
            "{text}"
        );
        assert!(
            lines[2].contains("15.0        0.0  60.00%  00fdf1c0"),
            "{text}"
        );
        assert!(
            lines[3].contains("5.0        1.0  20.00%  00559450"),
            "{text}"
        );
        assert!(lines[4].starts_with("  top 2 edges"), "{text}");
        assert!(
            lines[5].contains("5.0        1.0  0086e650 -> 00559450"),
            "{text}"
        );
        assert_eq!(lines.len(), 6, "{text}");
    }
}
