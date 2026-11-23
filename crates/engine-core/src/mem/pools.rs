//! The thread heap (the host's row: on in the fixes preset, `bThreadHeap`, `NV_THREAD_HEAP`):
//! blocks of the exe's memory pools (8 to 596 bytes, [`super::heap::pool_index`]) handed out from
//! runs of one pool each, through a cache of free blocks per thread, so that an allocation or a
//! free takes no lock in the common case.
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
//! - A run whose every slot is back in the stock (none live, none in a thread's stack) gives its
//!   heap block back when its pool's stock still holds [`SPARE_BYTES`] in blocks of other runs:
//!   a one-off peak in one pool does not keep its memory for the rest of the session. Its number
//!   is used again for the pool's next run.
//! - As with the heap's quick stacks, a freed block comes back only for a request of its own pool (a
//!   12-byte object's block never for a 16-byte one): a stale pointer finds an object of its own
//!   pool or the old bytes, as in the original. Blocks are 8-byte aligned and zero-filled when
//!   handed out; nothing else is written into guest memory.
//! - Runs are live heap blocks while they are held (the heap's checks and census see them; a free of
//!   a run's own heap block stops the run, as a free of any block that is not live does).
//! - With threads stepped there is one `Mem`, one cache and the same order every run.

use super::{HEAP_BASE, HEAP_END, PAGE, PAGES, PAGE_BITS};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::OnceLock;

/// The bytes of one run.
pub(super) const RUN_BYTES: u32 = 0x4_0000;
/// The heap block a run is cut from: a page more, so the run can start on a page boundary and own
/// every page it covers.
pub(super) const RUN_BLOCK: u32 = RUN_BYTES + PAGE as u32;
/// The most runs there can be at once: the heap's two ranges in runs (the run numbers fit 16 bits).
const MAX_RUNS: usize = ((HEAP_END - HEAP_BASE) / RUN_BYTES) as usize;
const _: () = assert!(MAX_RUNS < u16::MAX as usize);
/// Where a run cut from the heap block at `block` starts: the first page boundary past the block's
/// start (so no slot of the run ever has the block's own address, and a stray free of the block is
/// told from a free of the first slot).
pub(super) fn run_start(block: u32) -> u32 {
    (block + PAGE as u32) & !(PAGE as u32 - 1)
}
/// The blocks a thread's stack takes from the stock at once, and gives back at once.
pub(super) const BATCH: usize = 32;
/// The most free blocks a thread's stack of one pool holds before it gives [`BATCH`] back.
pub(super) const CACHE_MAX: usize = 2 * BATCH;
/// The bytes of other runs' free blocks a pool's stock keeps before an empty run gives its memory
/// back: a menu's blocks (a few MB over all pools) are not given back between its openings, a
/// one-off peak of many MB in one pool is.
pub(super) const SPARE_BYTES: usize = 4 << 20;
/// The census line is written again each time the runs hold this many more bytes than at the last.
pub(super) const REPORT_STEP: u64 = 32 << 20;

/// One run: slots of `stride` bytes from `start`, of pool `class`, cut from the heap block at
/// `block` (0 while the run is given back and its number unused).
pub(super) struct Run {
    start: AtomicU32,
    block: AtomicU32,
    pub(super) class: usize,
    pub(super) stride: u32,
    /// The size each slot was asked for while it is live, 0 while it is free.
    asked: Box<[AtomicU16]>,
    /// How many slots are live.
    live: AtomicU32,
}

impl Run {
    /// The first slot.
    #[inline]
    pub(super) fn start(&self) -> u32 {
        self.start.load(Ordering::Relaxed)
    }

    /// The heap block the run is cut from (0 while given back).
    #[inline]
    pub(super) fn block(&self) -> u32 {
        self.block.load(Ordering::Relaxed)
    }

    /// How many slots the run has.
    fn slots(&self) -> usize {
        self.asked.len()
    }

    /// The slot `addr` starts; a stop for an address inside the run that starts none.
    fn slot(&self, addr: u32) -> usize {
        let offset = addr - self.start();
        let slot = (offset / self.stride) as usize;
        assert!(
            offset % self.stride == 0 && slot < self.slots(),
            "free of {addr:08x}, which is not a live heap block (inside a run of {}-byte blocks)",
            self.stride
        );
        slot
    }

    /// The size `addr` was asked for, if it is a live block.
    pub(super) fn asked_of(&self, addr: u32) -> Option<u32> {
        let offset = addr.wrapping_sub(self.start());
        if offset % self.stride != 0 {
            return None;
        }
        let asked = self.asked.get((offset / self.stride) as usize)?;
        match asked.load(Ordering::Relaxed) {
            0 => None,
            n => Some(u32::from(n)),
        }
    }

    /// Marks `addr` live, asked for `asked` bytes (at least 1). A slot is only ever touched by the
    /// one thread that holds it (in its stack, or live), so a load and a store do (no locked swap).
    pub(super) fn take(&self, addr: u32, asked: u32) {
        let slot = self.slot(addr);
        let was = self.asked[slot].load(Ordering::Relaxed);
        self.asked[slot].store(asked.max(1) as u16, Ordering::Relaxed);
        assert!(
            was == 0,
            "the thread heap handed out {addr:08x}, which is live"
        );
        self.live.fetch_add(1, Ordering::Relaxed);
    }

    /// Marks `addr` free; a stop when it is not live (a second free).
    pub(super) fn give(&self, addr: u32) {
        let slot = self.slot(addr);
        let was = self.asked[slot].load(Ordering::Relaxed);
        assert!(
            was != 0,
            "free of {addr:08x}, which is not a live heap block"
        );
        self.asked[slot].store(0, Ordering::Relaxed);
        self.live.fetch_sub(1, Ordering::Relaxed);
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
        let start = self.start();
        self.asked.iter().enumerate().filter_map(move |(i, a)| {
            let asked = a.load(Ordering::Relaxed);
            (asked != 0).then(|| (start + i as u32 * self.stride, u32::from(asked)))
        })
    }
}

/// What every thread reads without the lock: the runs, and the run of each page.
pub(super) struct Tables {
    /// For each page of the 4 GB space, the number of the run that owns it plus 1 (0: none).
    by_page: Box<[AtomicU16]>,
    /// The runs by number. A number keeps its pool for good: a run given back leaves its number
    /// to the pool's next run (`start` and `block` set anew).
    runs: Box<[OnceLock<Run>]>,
    /// How many numbers have been used.
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

    /// The number of the run `addr` lies in, if any.
    #[inline]
    fn number_of(&self, addr: u32) -> Option<usize> {
        // Acquire: a run is complete before its pages name it (`add`).
        match self.by_page[(addr >> PAGE_BITS) as usize].load(Ordering::Acquire) {
            0 => None,
            n => Some(usize::from(n - 1)),
        }
    }

    /// The run `addr` lies in, if any.
    #[inline]
    pub(super) fn run_of(&self, addr: u32) -> Option<&Run> {
        self.runs[self.number_of(addr)?].get()
    }

    /// Whether `addr` is the heap block of a run that is held (a free of it is a stray free).
    pub(super) fn is_run_block(&self, addr: u32) -> bool {
        let start = run_start(addr);
        self.run_of(start).is_some_and(|r| r.block() == addr)
    }

    /// Every run held now.
    pub(super) fn runs(&self) -> impl Iterator<Item = &Run> + '_ {
        let n = self.count.load(Ordering::Acquire) as usize;
        self.runs[..n]
            .iter()
            .filter_map(OnceLock::get)
            .filter(|r| r.block() != 0)
    }

    /// Records run `number` of pool `class` (block size `stride`) at `start` (page-aligned, cut
    /// from heap block `block`) and names it on its pages: a new number, or one a run of the same
    /// pool gave back. Called under the shared memory's lock.
    fn add(&self, number: usize, start: u32, block: u32, class: usize, stride: u32) {
        let run = self.runs[number].get_or_init(|| Run {
            start: AtomicU32::new(0),
            block: AtomicU32::new(0),
            class,
            stride,
            asked: (0..(RUN_BYTES / stride) as usize)
                .map(|_| AtomicU16::new(0))
                .collect(),
            live: AtomicU32::new(0),
        });
        assert!(
            run.class == class && run.block() == 0,
            "run {number} is held, or another pool's"
        );
        run.start.store(start, Ordering::Relaxed);
        run.block.store(block, Ordering::Relaxed);
        let tag = u16::try_from(number + 1).expect("run numbers fit 16 bits");
        self.tag(start, tag);
        let count = self.count.load(Ordering::Relaxed);
        self.count
            .store(count.max(number as u32 + 1), Ordering::Release);
    }

    /// Names run `tag - 1` (0: none) on the pages from `start`.
    fn tag(&self, start: u32, tag: u16) {
        let first = (start >> PAGE_BITS) as usize;
        for page in first..first + (RUN_BYTES as usize >> PAGE_BITS) {
            self.by_page[page].store(tag, Ordering::Release);
        }
    }

    /// Takes run `number`'s pages back (no live slot, none in a thread's stack); answers its heap
    /// block, which the caller frees.
    fn remove(&self, number: usize) -> u32 {
        let run = self.runs[number].get().expect("a held run");
        assert_eq!(
            run.live.load(Ordering::Relaxed),
            0,
            "run {number} given back with live blocks"
        );
        self.tag(run.start(), 0);
        run.block.swap(0, Ordering::Relaxed)
    }
}

/// One pool's stock, kept by the shared memory under its lock: the runs holding blocks the threads
/// gave back (the last given to first; a run whose blocks were all taken again, or that gave its
/// memory back, is skipped when met), how many blocks those are, the newest run with the next of its
/// slots never handed out, and the numbers of its runs that gave their memory back.
#[derive(Default)]
struct Stock {
    with_free: Vec<usize>,
    free_count: usize,
    newest: Option<(usize, u32)>,
    unused: Vec<usize>,
}

/// Every pool's stock and the runs made (the shared memory's, under its lock).
pub(super) struct Stocks {
    pools: Vec<Stock>,
    /// How many run numbers have been used.
    numbers: usize,
    /// For each run number, the blocks of it given back, the newest last.
    free: Vec<Vec<u32>>,
    /// How many runs gave their memory back.
    pub(super) runs_given_back: u64,
    /// The bytes held in runs at which the census line is written next.
    pub(super) next_line: u64,
}

impl Default for Stocks {
    fn default() -> Self {
        Stocks {
            pools: (0..super::heap::POOL_COUNT)
                .map(|_| Stock::default())
                .collect(),
            numbers: 0,
            free: Vec::new(),
            runs_given_back: 0,
            next_line: 0,
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
    /// Takes the blocks given back to pool `class`. Answers the heap blocks of the pool's runs that
    /// gave their memory back meanwhile (every slot in the stock, and [`SPARE_BYTES`] in blocks of
    /// other runs left in it besides), for the caller to free.
    pub(super) fn give_back(
        &mut self,
        tables: &Tables,
        class: usize,
        blocks: impl IntoIterator<Item = u32>,
    ) -> Vec<u32> {
        let mut full = Vec::new();
        for b in blocks {
            let number = tables
                .number_of(b)
                .expect("a thread heap block lies in a run");
            let list = &mut self.free[number];
            list.push(b);
            let stock = &mut self.pools[class];
            if list.len() == 1 {
                stock.with_free.push(number);
            }
            stock.free_count += 1;
            if list.len() == tables.runs[number].get().map_or(0, Run::slots) {
                full.push(number);
            }
        }
        let mut freed = Vec::new();
        for number in full {
            let run = tables.runs[number].get().expect("a held run");
            let mine = self.free[number].len();
            let stock = &mut self.pools[class];
            if mine != run.slots()
                || (stock.free_count - mine) * (run.stride as usize) < SPARE_BYTES
            {
                continue;
            }
            // Its entry in `with_free` is skipped when met (its list is empty, or another run's
            // of the pool by then): nothing is scanned under the lock.
            self.free[number] = Vec::new();
            stock.free_count -= mine;
            stock.unused.push(number);
            self.runs_given_back += 1;
            freed.push(tables.remove(number));
        }
        freed
    }

    /// Fills `out` (empty) with up to [`BATCH`] blocks of pool `class` (at least one), so that the
    /// last is handed out first: given-back blocks, the newest first, else fresh slots of the newest
    /// run in address order. `new_run` takes a heap block for a run (`(block, start)`) when the
    /// newest has no fresh slot left; the run takes a number the pool gave back, else a new one.
    pub(super) fn restock(
        &mut self,
        tables: &Tables,
        class: usize,
        stride: u32,
        out: &mut Vec<u32>,
        mut new_run: impl FnMut() -> (u32, u32),
    ) {
        let stock = &mut self.pools[class];
        while out.len() < BATCH {
            let Some(&number) = stock.with_free.last() else {
                break;
            };
            let list = &mut self.free[number];
            let take = (BATCH - out.len()).min(list.len());
            out.extend(list.drain(list.len() - take..));
            stock.free_count -= take;
            if list.is_empty() {
                stock.with_free.pop();
            }
        }
        if !out.is_empty() {
            return;
        }
        let slots = RUN_BYTES / stride;
        let (number, next) = match stock.newest {
            Some((number, next)) if next < slots => (number, next),
            _ => {
                let (block, start) = new_run();
                let number = stock.unused.pop().unwrap_or_else(|| {
                    self.numbers += 1;
                    self.numbers - 1
                });
                if self.free.len() <= number {
                    self.free.resize_with(number + 1, Vec::new);
                }
                tables.add(number, start, block, class, stride);
                (number, 0)
            }
        };
        let start = tables.runs[number].get().expect("the newest run").start();
        let end = (next + BATCH as u32).min(slots);
        out.extend((next..end).rev().map(|slot| start + slot * stride));
        self.pools[class].newest = Some((number, end));
    }
}

/// The thread heap's census: the runs held and their bytes, the blocks live in them and the bytes
/// those hold, and how many runs gave their memory back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThreadHeapCensus {
    pub runs: u32,
    pub run_bytes: u64,
    pub live_blocks: u64,
    pub live_bytes: u64,
    pub runs_given_back: u64,
}

impl ThreadHeapCensus {
    pub(super) fn of(tables: &Tables, stocks: Option<&Stocks>) -> Self {
        let mut c = ThreadHeapCensus {
            runs_given_back: stocks.map_or(0, |s| s.runs_given_back),
            ..Default::default()
        };
        for run in tables.runs() {
            let live = u64::from(run.live.load(Ordering::Relaxed));
            c.runs += 1;
            c.run_bytes += u64::from(RUN_BLOCK);
            c.live_blocks += live;
            c.live_bytes += live * u64::from(run.stride);
        }
        c
    }
}

impl std::fmt::Display for ThreadHeapCensus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "thread heap: {} runs hold {:.1} MB, {} live blocks {:.1} MB in them; {} runs gave their \
             memory back so far",
            self.runs,
            self.run_bytes as f64 / 1_048_576.0,
            self.live_blocks,
            self.live_bytes as f64 / 1_048_576.0,
            self.runs_given_back
        )
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
    fn a_peak_freed_on_another_thread_gives_runs_back_and_comes_back_whole() {
        let mut m = on();
        m.start_sharing();
        // Over 4 MB in one pool, tagged, freed on another thread whose memory then goes away.
        let per_run = (super::RUN_BYTES / 600) as usize;
        let n = 24 * per_run;
        let peak: Vec<u32> = (0..n)
            .map(|i| {
                let a = m.alloc_asked(596);
                m.set_u32(a, i as u32 | 0x8000_0000);
                a
            })
            .collect();
        let mut other = m.view();
        let freed = peak.clone();
        std::thread::spawn(move || {
            for (i, a) in freed.into_iter().enumerate() {
                assert_eq!(
                    other.u32(a),
                    i as u32 | 0x8000_0000,
                    "block {a:08x} changed hands while live"
                );
                other.free_asked(a);
            }
        })
        .join()
        .expect("the other thread");
        let census = m.thread_heap_census().expect("on");
        assert!(census.runs_given_back >= 1, "{census}");
        assert_eq!(census.live_blocks, 0);
        // The first thread takes as many again: each block once, zero-filled, its own while live.
        let mut seen = std::collections::HashSet::new();
        let again: Vec<u32> = (0..n)
            .map(|i| {
                let a = m.alloc_asked(596);
                assert!(seen.insert(a), "{a:08x} handed out twice");
                assert!(m.bytes(a, 596).iter().all(|&b| b == 0), "zero-filled");
                m.set_u32(a, i as u32);
                a
            })
            .collect();
        for (i, a) in again.into_iter().enumerate() {
            assert_eq!(m.u32(a), i as u32, "block {a:08x} changed hands while live");
            m.free_asked(a);
        }
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

    #[test]
    #[should_panic(expected = "not a live heap block")]
    fn a_free_of_a_runs_own_heap_block_stops() {
        let mut m = on();
        let a = m.alloc_asked(12);
        let block = m
            .pools
            .as_deref()
            .and_then(|t| t.run_of(a))
            .map(super::Run::block);
        m.free(block.expect("a run"));
    }

    /// How many run numbers have been used.
    fn numbers(m: &Mem) -> u32 {
        m.pools
            .as_deref()
            .map_or(0, |t| t.count.load(std::sync::atomic::Ordering::Relaxed))
    }

    #[test]
    fn an_empty_run_gives_its_memory_back_and_another_pool_uses_it() {
        let mut m = on();
        // A one-off peak: 24 runs of the 596-byte pool (436 blocks a run, 6.1 MB), then all freed: the
        // runs that fill up once the pool keeps 4 MB of others give their memory back.
        let per_run = (super::RUN_BYTES / 600) as usize;
        let peak: Vec<u32> = (0..24 * per_run).map(|_| m.alloc_asked(596)).collect();
        let held = m.thread_heap_census().expect("on");
        assert_eq!((held.runs, held.live_blocks), (24, 24 * per_run as u64));
        assert_eq!(numbers(&m), 24);
        let top = m.heap_top();
        for &b in &peak {
            m.free_asked(b);
        }
        let after = m.thread_heap_census().expect("on");
        assert!(after.runs_given_back >= 1, "{after}");
        assert_eq!(after.runs as u64 + after.runs_given_back, 24);
        assert_eq!(after.live_blocks, 0);
        // A run given back names none of its pages any more: its addresses are no thread heap blocks.
        let gone: Vec<u32> = peak
            .iter()
            .copied()
            .filter(|&a| run_of(&m, a).is_none())
            .collect();
        assert_eq!(gone.len(), after.runs_given_back as usize * per_run);
        assert!(gone.iter().all(|&a| m.asked_size(a).is_none()));
        let (low, high) = (
            gone.iter().min().copied().unwrap_or(0),
            gone.iter().max().copied().unwrap_or(0) + 600,
        );
        // Another pool's new run takes the memory given back: the heap does not grow.
        let small = m.alloc_asked(8);
        assert!(m.heap_top() <= top, "{:08x} > {top:08x}", m.heap_top());
        m.free_asked(small);
        // The heap's own blocks land in the pages given back, and their frees and sizes are the
        // heap's (a page still naming its old run would take them for a thread heap block).
        let big: Vec<u32> = (0..800).map(|_| m.alloc_asked(2000)).collect();
        assert!(big.iter().any(|&a| (low..high).contains(&a)));
        for &a in &big {
            assert_eq!(run_of(&m, a), None);
            assert_eq!(m.asked_size(a), Some(2000));
            m.free_asked(a);
        }
        // And the pool's own numbers are used again for its next runs: no new number.
        let used = numbers(&m);
        let again: Vec<u32> = (0..24 * per_run).map(|_| m.alloc_asked(596)).collect();
        assert_eq!(
            m.thread_heap_census().expect("on").live_blocks,
            24 * per_run as u64
        );
        assert_eq!(numbers(&m), used, "no new run number");
        for b in again {
            m.free_asked(b);
        }
    }

    #[test]
    fn a_pool_keeps_its_last_run_when_no_other_blocks_are_spare() {
        let mut m = on();
        let per_run = (super::RUN_BYTES / 600) as usize;
        let one: Vec<u32> = (0..per_run).map(|_| m.alloc_asked(596)).collect();
        for b in one {
            m.free_asked(b);
        }
        let c = m.thread_heap_census().expect("on");
        assert_eq!((c.runs, c.runs_given_back), (1, 0));
    }

    #[test]
    fn a_watch_started_after_the_allocation_sees_the_free() {
        let mut m = on();
        let a = m.alloc_asked(0x10);
        m.watch(a, 0x10, true);
        m.free_asked(a);
        let hits = m.take_watch_hits();
        assert!(
            hits.iter()
                .any(|h| h.addr == a && h.kind == super::super::WatchKind::Free),
            "{} hits",
            hits.len()
        );
    }

    #[test]
    fn sharing_keeps_the_threads_free_blocks_with_it() {
        let mut m = on();
        let a = m.alloc_asked(20);
        m.free_asked(a);
        m.start_sharing();
        assert_eq!(m.alloc_asked(20), a);
    }

    #[test]
    fn the_census_counts_runs_and_live_blocks() {
        let mut m = on();
        assert_eq!(m.thread_heap_census(), Some(Default::default()));
        let a = m.alloc_asked(12);
        let b = m.alloc_asked(100);
        let c = m.thread_heap_census().expect("on");
        assert_eq!((c.runs, c.live_blocks, c.live_bytes), (2, 2, 16 + 104));
        assert_eq!(c.run_bytes, 2 * u64::from(super::RUN_BLOCK));
        m.free_asked(a);
        m.free_asked(b);
        assert_eq!(m.thread_heap_census().expect("on").live_blocks, 0);
        assert_eq!(Mem::new().thread_heap_census(), None);
    }
}
