//! Not a check: the game's heap as the game calls it, timed. `MemoryManager::Allocate`
//! (`00aa3e40`) and `Deallocate` (`00aa4060`) by address through the dispatch table, and the
//! port's heap below them (`Mem::alloc_asked`, `Mem::free_asked`) on its own, with the memory
//! private (stepped threads) and shared (real threads: one lock, with and without a second
//! thread allocating at the same time). Two patterns: a pause-menu open and close (80,000
//! blocks of a menu-tile mix allocated, then all freed in the same order, timed on the second
//! round) and a steady churn (50,000 live blocks, one freed and one allocated per step).
//!
//! `cargo test --release -p engine-core --test heap_call_speed -- --ignored --nocapture`

use engine_core::Engine;
use std::time::Instant;

engine_core::assemble!();

const MEMORY_MANAGER: u32 = 0x011f_6238;
const ALLOCATE: u32 = 0x00aa_3e40;
const DEALLOCATE: u32 = 0x00aa_4060;

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (((self.0 >> 32) * u64::from(n)) >> 32) as u32
    }
}

/// A menu's blocks: tiles' value records and short strings (12-32 bytes), tiles and their
/// lists (33-64), tile objects with their members (65-256), name and text buffers (257-1024).
fn menu_size(r: &mut Rng) -> u32 {
    match r.below(100) {
        0..=39 => 12 + r.below(21),
        40..=69 => 33 + r.below(32),
        70..=89 => 65 + r.below(192),
        _ => 257 + r.below(768),
    }
}

/// How a block is allocated and freed.
trait Heap {
    fn alloc(&mut self, size: u32) -> u32;
    fn free(&mut self, addr: u32);
}

/// By address, through the dispatch table, as translated code calls it.
struct ByAddress<'a>(&'a mut Engine);

impl Heap for ByAddress<'_> {
    #[inline]
    fn alloc(&mut self, size: u32) -> u32 {
        self.0.call(ALLOCATE, &[MEMORY_MANAGER, size]).u32()
    }
    #[inline]
    fn free(&mut self, addr: u32) {
        self.0.call(DEALLOCATE, &[MEMORY_MANAGER, addr]);
    }
}

/// The port's heap alone (what `00aa3e40` and `00aa4060` run).
struct Direct<'a>(&'a mut engine_core::Mem);

impl Heap for Direct<'_> {
    #[inline]
    fn alloc(&mut self, size: u32) -> u32 {
        self.0.alloc_asked(size)
    }
    #[inline]
    fn free(&mut self, addr: u32) {
        self.0.free_asked(addr)
    }
}

/// ns per allocation and per free of an open and close of `n` blocks (the second of two).
fn open_close(h: &mut impl Heap, n: usize) -> (f64, f64) {
    let mut best = (f64::MAX, f64::MAX);
    for _ in 0..3 {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut blocks = Vec::with_capacity(n);
        let mut times = (0.0, 0.0);
        for _round in 0..2 {
            blocks.clear();
            let t = Instant::now();
            for _ in 0..n {
                blocks.push(h.alloc(menu_size(&mut rng)));
            }
            let a = t.elapsed().as_nanos() as f64 / n as f64;
            let t = Instant::now();
            for &b in &blocks {
                h.free(b);
            }
            times = (a, t.elapsed().as_nanos() as f64 / n as f64);
        }
        best = (best.0.min(times.0), best.1.min(times.1));
    }
    best
}

/// ns per free plus allocation of a churn over `slots` live blocks.
fn churn(h: &mut impl Heap, slots: usize, steps: u32) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..3 {
        let mut rng = Rng(0x5851_f42d_4c95_7f2d);
        let mut live: Vec<u32> = (0..slots).map(|_| h.alloc(menu_size(&mut rng))).collect();
        let t = Instant::now();
        for _ in 0..steps {
            let i = rng.below(slots as u32) as usize;
            h.free(live[i]);
            live[i] = h.alloc(menu_size(&mut rng));
        }
        best = best.min(t.elapsed().as_nanos() as f64 / f64::from(steps));
        for a in live {
            h.free(a);
        }
    }
    best
}

fn report(what: &str, h: &mut impl Heap) {
    let (a, f) = open_close(h, 80_000);
    let c = churn(h, 50_000, 1_000_000);
    eprintln!(
        "heap call speed, {what}: open {a:.0} ns per allocation, close {f:.0} ns per free \
         (80,000 blocks: open {:.1} ms, close {:.1} ms); churn {c:.0} ns per free and allocation",
        a * 80_000.0 / 1e6,
        f * 80_000.0 / 1e6
    );
}

#[test]
#[ignore]
fn heap_call_speed() {
    let mut e = Engine::new();
    report("private memory, port heap alone", &mut Direct(&mut e.mem));
    report("private memory, by address", &mut ByAddress(&mut e));

    let mut e = Engine::new();
    e.mem.start_sharing();
    report(
        "shared memory (real threads), port heap alone",
        &mut Direct(&mut e.mem),
    );
    report(
        "shared memory (real threads), by address",
        &mut ByAddress(&mut e),
    );

    // A second thread allocating and freeing the whole time, on its own view.
    let mut other = e.mem.view();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop2 = stop.clone();
    let busy = std::thread::spawn(move || {
        let mut rng = Rng(7);
        let mut live: Vec<u32> = (0..1000)
            .map(|_| other.alloc_asked(menu_size(&mut rng)))
            .collect();
        let mut n = 0u64;
        while !stop2.load(std::sync::atomic::Ordering::Relaxed) {
            let i = rng.below(1000) as usize;
            other.free_asked(live[i]);
            live[i] = other.alloc_asked(menu_size(&mut rng));
            n += 1;
        }
        for a in live {
            other.free_asked(a);
        }
        n
    });
    report(
        "shared memory, a second thread allocating, by address",
        &mut ByAddress(&mut e),
    );
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let n = busy.join().expect("the second thread");
    eprintln!("heap call speed: the second thread made {n} frees and allocations meanwhile");
}
