//! The thread heap (`NV_THREAD_HEAP=1`; off by default): blocks of the exe's memory pools (8 to
//! 596 bytes, [`super::heap::pool_index`]) handed out from runs of one pool each, through a cache of
//! free blocks per thread, so that an allocation or a free takes no lock in the common case.
//!
//! - A run is [`RUN_BYTES`] of the heap, page-aligned inside one live heap block taken for it
//!   ([`super::Mem::alloc`]), cut into slots of its pool's block size (the pool's size rounded up to
//!   8, what the heap holds for such a block). The table of runs by page ([`Tables::run_of`]) finds a
//!   block's run with one load: a free looks nothing up in a hash table, and neither does `_msize`,
//!   since each run keeps the size each of its live slots was asked for (0 for a free slot).
//! - Each `Mem` (with real threads: each thread's view) keeps a stack of free blocks per pool,
//!   newest on top ([`Cache`]). An allocation takes the top of its pool's stack, a free puts the
//!   block on top. An empty stack takes [`BATCH`] blocks from the pool's stock (the blocks the
//!   threads gave back, newest first, then fresh slots of the pool's newest run, then a new run)
//!   under the shared memory's lock; a stack over [`CACHE_MAX`] gives its oldest [`BATCH`] back to
//!   the stock. So a block freed on another thread than the one that took it goes back in a batch.
//! - As with the heap's quick stacks, a freed block comes back only for a request of its own pool (a
//!   12-byte object's block never for a 16-byte one): a stale pointer finds an object of its own
//!   pool or the old bytes, as in the original. Blocks are 8-byte aligned and zero-filled when
//!   handed out; nothing else is written into guest memory.
//! - Runs stay live heap blocks for good (the heap's checks and census count them as such).
//! - With threads stepped there is one `Mem`, one cache and the same order every run.

use super::{HEAP_BASE, HEAP_END, PAGE, PAGES, PAGE_BITS};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::OnceLock;

/// The bytes of one run.
pub(super) const RUN_BYTES: u32 = 0x4_0000;
/// The heap block a run is cut from: a page more, so the run can start on a page boundary and own
/// every page it covers.
pub(super) const RUN_BLOCK: u32 = RUN_BYTES + PAGE as u32;
/// The most runs there can be: the heap's two ranges in runs (the run numbers fit 16 bits).
const MAX_RUNS: usize = ((HEAP_END - HEAP_BASE) / RUN_BYTES) as usize;
const _: () = assert!(MAX_RUNS < u16::MAX as usize);
/// The blocks a thread's stack takes from the stock at once, and gives back at once.
pub(super) const BATCH: usize = 32;
/// The most free blocks a thread's stack of one pool holds before it gives [`BATCH`] back.
pub(super) const CACHE_MAX: usize = 2 * BATCH;

/// One run: `slots` blocks of `stride` bytes from `start`, of pool `class`.
pub(super) struct Run {
    pub(super) start: u32,
    pub(super) class: usize,
    pub(super) stride: u32,
    /// The size each slot was asked for while it is live, 0 while it is free.
    asked: Box<[AtomicU16]>,
}

impl Run {
    /// The slot `addr` starts; a stop for an address inside the run that starts none.
    fn slot(&self, addr: u32) -> usize {
        let offset = addr - self.start;
        let slot = (offset / self.stride) as usize;
        assert!(
            offset % self.stride == 0 && slot < self.asked.len(),
            "free of {addr:08x}, which is not a live heap block (inside a run of {}-byte blocks)",
            self.stride
        );
        slot
    }

    /// The size `addr` was asked for, if it is a live block.
    pub(super) fn asked_of(&self, addr: u32) -> Option<u32> {
        let offset = addr.wrapping_sub(self.start);
        if offset % self.stride != 0 {
            return None;
        }
        let asked = self.asked.get((offset / self.stride) as usize)?;
        match asked.load(Ordering::Relaxed) {
            0 => None,
            n => Some(u32::from(n)),
        }
    }

    /// Marks `addr` live, asked for `asked` bytes (at least 1).
    pub(super) fn take(&self, addr: u32, asked: u32) {
        let slot = self.slot(addr);
        let was = self.asked[slot].swap(asked.max(1) as u16, Ordering::Relaxed);
        assert!(
            was == 0,
            "the thread heap handed out {addr:08x}, which is live"
        );
    }

    /// Marks `addr` free; a stop when it is not live (a second free).
    pub(super) fn give(&self, addr: u32) {
        let slot = self.slot(addr);
        let was = self.asked[slot].swap(0, Ordering::Relaxed);
        assert!(
            was != 0,
            "free of {addr:08x}, which is not a live heap block"
        );
    }

    /// Sets the asked size of the live block `addr`.
    pub(super) fn set_asked(&self, addr: u32, asked: u32) {
        let slot = self.slot(addr);
        assert!(
            asked <= self.stride,
            "{addr:08x} holds {} bytes, not {asked}",
            self.stride
        );
        self.asked[slot].store(asked.max(1) as u16, Ordering::Relaxed);
    }

    /// The live blocks of the run and the sizes they were asked for.
    pub(super) fn live(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.asked.iter().enumerate().filter_map(|(i, a)| {
            let asked = a.load(Ordering::Relaxed);
            (asked != 0).then(|| (self.start + i as u32 * self.stride, u32::from(asked)))
        })
    }
}

/// What every thread reads without the lock: the runs, and the run of each page.
pub(super) struct Tables {
    /// For each page of the 4 GB space, the number of the run that owns it plus 1 (0: none).
    by_page: Box<[AtomicU16]>,
    runs: Box<[OnceLock<Run>]>,
    /// How many runs there are.
    count: AtomicU32,
}

impl Tables {
    pub(super) fn new() -> Self {
        Tables {
            by_page: (0..PAGES).map(|_| AtomicU16::new(0)).collect(),
            runs: (0..MAX_RUNS).map(|_| OnceLock::new()).collect(),
            count: AtomicU32::new(0),
        }
    }

    /// The run `addr` lies in, if any.
    #[inline]
    pub(super) fn run_of(&self, addr: u32) -> Option<&Run> {
        // Acquire: a run is complete before its pages name it (`add`).
        match self.by_page[(addr >> PAGE_BITS) as usize].load(Ordering::Acquire) {
            0 => None,
            n => self.runs[usize::from(n - 1)].get(),
        }
    }

    /// Every run made so far.
    pub(super) fn runs(&self) -> impl Iterator<Item = &Run> + '_ {
        let n = self.count.load(Ordering::Acquire) as usize;
        self.runs[..n].iter().filter_map(OnceLock::get)
    }

    /// Records run `number` of pool `class` (block size `stride`) at `start` (page-aligned) and
    /// names it on its pages. Called under the shared memory's lock, so numbers come in order.
    fn add(&self, number: usize, start: u32, class: usize, stride: u32) {
        let slots = (RUN_BYTES / stride) as usize;
        let run = Run {
            start,
            class,
            stride,
            asked: (0..slots).map(|_| AtomicU16::new(0)).collect(),
        };
        assert!(
            self.runs[number].set(run).is_ok(),
            "run {number} made twice"
        );
        let first = (start >> PAGE_BITS) as usize;
        let tag = u16::try_from(number + 1).expect("run numbers fit 16 bits");
        for page in first..first + (RUN_BYTES as usize >> PAGE_BITS) {
            self.by_page[page].store(tag, Ordering::Release);
        }
        self.count.store(number as u32 + 1, Ordering::Release);
    }
}

/// One pool's stock, kept by the shared memory under its lock: blocks the threads gave back (the
/// newest last), and the newest run with the next of its slots never handed out.
#[derive(Default)]
struct Stock {
    given_back: Vec<u32>,
    newest: Option<(usize, u32)>,
}

/// Every pool's stock and the runs made (the shared memory's, under its lock).
pub(super) struct Stocks {
    pools: Vec<Stock>,
    runs: usize,
}

impl Default for Stocks {
    fn default() -> Self {
        Stocks {
            pools: (0..super::heap::POOL_COUNT)
                .map(|_| Stock::default())
                .collect(),
            runs: 0,
        }
    }
}

/// A thread's free blocks of each pool, the newest last.
pub(super) struct Cache {
    pools: Vec<Vec<u32>>,
}

impl Default for Cache {
    fn default() -> Self {
        Cache {
            pools: (0..super::heap::POOL_COUNT).map(|_| Vec::new()).collect(),
        }
    }
}

impl Cache {
    #[inline]
    pub(super) fn pop(&mut self, class: usize) -> Option<u32> {
        self.pools[class].pop()
    }

    /// Puts `addr` on its pool's stack; answers the oldest [`BATCH`] when the stack went over
    /// [`CACHE_MAX`], to be given back.
    #[inline]
    pub(super) fn push(&mut self, class: usize, addr: u32) -> Option<Vec<u32>> {
        let stack = &mut self.pools[class];
        stack.push(addr);
        (stack.len() > CACHE_MAX).then(|| stack.drain(..BATCH).collect())
    }

    /// The stack of `class`, to be filled.
    pub(super) fn stack(&mut self, class: usize) -> &mut Vec<u32> {
        &mut self.pools[class]
    }

    /// Every block waiting, by pool (a thread's memory going away gives them back).
    pub(super) fn take_all(&mut self) -> Vec<(usize, Vec<u32>)> {
        self.pools
            .iter_mut()
            .enumerate()
            .filter(|(_, s)| !s.is_empty())
            .map(|(c, s)| (c, std::mem::take(s)))
            .collect()
    }
}

impl Stocks {
    /// Takes the blocks given back to pool `class`.
    pub(super) fn give_back(&mut self, class: usize, blocks: impl IntoIterator<Item = u32>) {
        self.pools[class].given_back.extend(blocks);
    }

    /// Fills `out` (empty) with up to [`BATCH`] blocks of pool `class` (at least one), so that the
    /// last is handed out first: given-back blocks, newest first, then fresh slots of the newest
    /// run in address order. `new_run` makes a run of the pool (`(number, start)`) when the newest
    /// has no fresh slot left.
    pub(super) fn restock(
        &mut self,
        tables: &Tables,
        class: usize,
        stride: u32,
        out: &mut Vec<u32>,
        mut new_run: impl FnMut() -> u32,
    ) {
        let stock = &mut self.pools[class];
        let from = stock.given_back.len().saturating_sub(BATCH);
        out.extend(stock.given_back.drain(from..));
        if !out.is_empty() {
            return;
        }
        let slots = RUN_BYTES / stride;
        let (number, next) = match stock.newest {
            Some((number, next)) if next < slots => (number, next),
            _ => {
                let start = new_run();
                let number = self.runs;
                tables.add(number, start, class, stride);
                self.runs += 1;
                (number, 0)
            }
        };
        let start = tables.runs[number].get().expect("the newest run").start;
        let end = (next + BATCH as u32).min(slots);
        out.extend((next..end).rev().map(|slot| start + slot * stride));
        self.pools[class].newest = Some((number, end));
    }
}
#[cfg(test)]
mod tests {
    use super::super::Mem;
    use super::{BATCH, CACHE_MAX, RUN_BYTES};

    fn on() -> Mem {
        let mut m = Mem::new();
        m.enable_thread_heap();
        m
    }

    /// The run `addr` lies in: its block size and pool.
    fn run_of(m: &Mem, addr: u32) -> Option<(u32, usize)> {
        let run = m.pools.as_deref()?.run_of(addr)?;
        Some((run.stride, run.class))
    }

    #[test]
    fn a_pool_request_is_a_block_of_its_pool_in_a_run() {
        let mut m = on();
        let a = m.alloc_asked(12);
        let b = m.alloc_asked(16);
        let c = m.alloc_asked(0x41);
        assert_eq!(
            run_of(&m, a).map(|r| r.0),
            Some(16),
            "the 12-byte pool's blocks hold 16"
        );
        assert_ne!(run_of(&m, a), run_of(&m, b), "12 and 16 are two pools");
        assert_eq!(
            run_of(&m, c).map(|r| r.0),
            Some(80),
            "0x41 is the 80-byte pool's"
        );
        assert_eq!(a % 8, 0);
        assert_eq!(m.asked_size(a), Some(12));
        assert_eq!(m.asked_size(c), Some(0x41));
        assert_eq!(m.block_size(a), Some(16));
        // Past the pools (596 and 0x3fc up), the heap's own blocks.
        let big = m.alloc_asked(2000);
        assert_eq!(run_of(&m, big), None);
        assert_eq!(m.asked_size(big), Some(2000));
        for x in [a, b, c, big] {
            m.free_asked(x);
        }
        assert_eq!(m.asked_size(a), None);
        assert_eq!(m.block_size(a), None);
    }

    #[test]
    fn a_freed_block_comes_back_newest_first_zeroed_and_only_for_its_pool() {
        let mut m = on();
        let a = m.alloc_asked(20);
        let b = m.alloc_asked(20);
        m.write(a, &[0xAB; 20]);
        m.free_asked(b);
        m.free_asked(a);
        // A request of the 24-byte pool (the same 24 bytes held) does not get them.
        let other = m.alloc_asked(24);
        assert!(other != a && other != b);
        assert_eq!(
            m.alloc_asked(17),
            a,
            "newest first (17..20 is the 20-byte pool)"
        );
        assert_eq!(m.bytes(a, 24), vec![0; 24], "zero-filled");
        assert_eq!(m.alloc_asked(20), b);
    }

    #[test]
    fn fresh_slots_go_in_address_order_and_a_full_run_starts_another() {
        let mut m = on();
        let first = m.alloc_asked(596);
        let stride = 600;
        let slots = RUN_BYTES / stride;
        let mut last = first;
        for i in 1..slots {
            let a = m.alloc_asked(596);
            assert_eq!(a, first + i * stride);
            last = a;
        }
        let next = m.alloc_asked(596);
        assert!(next < first || next >= last + stride, "a new run");
        assert_eq!(next % super::PAGE as u32, 0, "runs start on a page");
    }

    #[test]
    #[should_panic(expected = "not a live heap block")]
    fn a_second_free_stops() {
        let mut m = on();
        let a = m.alloc_asked(32);
        m.free_asked(a);
        m.free_asked(a);
    }

    #[test]
    #[should_panic(expected = "not a live heap block")]
    fn a_free_inside_a_block_stops() {
        let mut m = on();
        let a = m.alloc_asked(32);
        m.free_asked(a + 8);
    }

    #[test]
    fn a_thread_keeps_at_most_its_bound_and_gives_batches_back() {
        let mut m = on();
        let n = 6 * BATCH;
        let blocks: Vec<u32> = (0..n).map(|_| m.alloc_asked(40)).collect();
        for &b in &blocks {
            m.free_asked(b);
        }
        let class = super::super::heap::pool_index(40).expect("a pool");
        let cached = m.pool_cache.stack(class).len();
        assert!(cached <= CACHE_MAX, "{cached} cached");
        let given_back = n - cached;
        assert_eq!(given_back % BATCH, 0);
        // Taken again: every block comes back before a fresh slot.
        let again: std::collections::HashSet<u32> = (0..n).map(|_| m.alloc_asked(40)).collect();
        assert_eq!(again, blocks.iter().copied().collect());
    }

    #[test]
    fn the_crt_size_questions_answer_for_pool_blocks() {
        let mut m = on();
        let a = m.alloc_asked(30);
        // `HeapReAlloc`'s order: the old size forgotten, then the block freed.
        assert_eq!(m.forget_asked(a), Some(30));
        m.set_asked(a, 31);
        assert_eq!(m.asked_size(a), Some(31));
        m.free(a);
        assert_eq!(m.asked_size(a), None);
    }

    #[test]
    fn off_it_changes_nothing() {
        let mut m = Mem::new();
        let a = m.alloc_asked(12);
        assert!(m.pools.is_none());
        assert_eq!(m.asked_size(a), Some(12));
        m.free_asked(a);
    }

    #[test]
    fn with_the_block_check_on_the_heap_serves_pool_requests() {
        let mut m = on();
        m.enable_block_check(false, true);
        let a = m.alloc_asked(12);
        assert_eq!(run_of(&m, a), None);
        m.free_asked(a);
    }

    // ---- Real threads: each thread on its own view of shared memory ----

    #[test]
    fn blocks_freed_on_another_thread_come_back_to_the_first() {
        let mut m = on();
        m.start_sharing();
        let mut other = m.view();
        let blocks: Vec<u32> = (0..10 * BATCH).map(|_| m.alloc_asked(20)).collect();
        let freed = blocks.clone();
        std::thread::spawn(move || {
            for b in freed {
                other.free_asked(b);
            }
            // `other` goes away: its stack goes back to the stock too.
        })
        .join()
        .expect("the other thread");
        let again: std::collections::HashSet<u32> =
            (0..10 * BATCH).map(|_| m.alloc_asked(20)).collect();
        assert_eq!(again, blocks.into_iter().collect());
    }

    #[test]
    fn many_threads_never_share_a_live_block() {
        let mut m = on();
        m.start_sharing();
        let threads: Vec<_> = (0..8u32)
            .map(|t| {
                let mut v = m.view();
                std::thread::spawn(move || {
                    let mut rng = 0x9e37_79b9u32 ^ (t + 1).wrapping_mul(0x85eb_ca6b);
                    let mut next = || {
                        rng ^= rng << 13;
                        rng ^= rng >> 17;
                        rng ^= rng << 5;
                        rng
                    };
                    let mut live: Vec<(u32, u32, u32)> = Vec::new();
                    for serial in 0..40_000u32 {
                        if live.len() > 300 || (!live.is_empty() && next() % 3 == 0) {
                            let i = next() as usize % live.len();
                            let (a, size, tag) = live.swap_remove(i);
                            assert_eq!(v.u32(a), tag, "block {a:08x} changed hands while live");
                            assert_eq!(v.asked_size(a), Some(size));
                            v.free_asked(a);
                        } else {
                            let size = 4 + next() % 700;
                            let a = v.alloc_asked(size);
                            assert!(v.bytes(a, size).iter().all(|&b| b == 0), "zero-filled");
                            let tag = t << 24 | serial;
                            v.set_u32(a, tag);
                            live.push((a, size, tag));
                        }
                    }
                    for (a, _, tag) in live {
                        assert_eq!(v.u32(a), tag);
                        v.free_asked(a);
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().expect("a thread");
        }
    }
}
