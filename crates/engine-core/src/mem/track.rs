//! The store track ([`Mem::enable_track`]): which tracked pages of guest memory were written since
//! their owner last looked. Off unless enabled (then every store tests one byte of a page table).
//!
//! A reader that keeps its own copy of some of the game's objects (the culling copy) marks the
//! pages holding them as tracked; every write of guest memory through [`Mem`] (stores of every
//! width, [`Mem::write`], [`Mem::zero`], frees, the interlocked operations' words) to a tracked
//! page sets that page's dirty bit; [`Track::take_dirty`] hands the dirty pages over and clears
//! them. The game's bytes are never changed or held back: the track only says where to look again.
//!
//! **Threads.** One track serves the shared memory and all its views (made before
//! [`Mem::start_sharing`], carried into every view). The tracked flags change only while the owner
//! has the walk to itself (between frames: [`Track::set_tracked`]); the dirty bits are atomic, set
//! from any thread, so a store that lands while the owner takes the dirty pages is seen either this
//! time or the next.
//!
//! [`Mem`]: super::Mem
//! [`Mem::enable_track`]: super::Mem::enable_track
//! [`Mem::write`]: super::Mem::write
//! [`Mem::zero`]: super::Mem::zero
//! [`Mem::start_sharing`]: super::Mem::start_sharing

use super::{PAGES, PAGE_BITS};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering::Relaxed};

/// The pages tracked and the pages written since they were last taken.
pub struct Track {
    /// One byte a page (and one past the last, always 0): nonzero when tracked.
    tracked: Box<[AtomicU8]>,
    /// One bit a page: written since [`Track::take_dirty`] last took it.
    dirty: Box<[AtomicU64]>,
}

impl Default for Track {
    fn default() -> Self {
        Self::new()
    }
}

impl Track {
    pub fn new() -> Track {
        Track {
            tracked: (0..=PAGES).map(|_| AtomicU8::new(0)).collect(),
            dirty: (0..PAGES.div_ceil(64)).map(|_| AtomicU64::new(0)).collect(),
        }
    }

    /// Tracks (or stops tracking) the pages `[addr, addr + len)` touches.
    pub fn set_tracked(&self, addr: u32, len: u32, on: bool) {
        if len == 0 {
            return;
        }
        let first = (addr >> PAGE_BITS) as usize;
        let last = ((u64::from(addr) + u64::from(len) - 1) >> PAGE_BITS) as usize;
        for p in first..=last.min(PAGES - 1) {
            self.tracked[p].store(u8::from(on), Relaxed);
        }
    }

    /// Whether page `p` is tracked.
    pub fn is_tracked(&self, p: usize) -> bool {
        self.tracked[p].load(Relaxed) != 0
    }

    /// A write of `len` bytes (at least 1) at `a`: marks each tracked page it touches.
    #[inline(always)]
    pub(super) fn note(&self, a: u32, len: u32) {
        let first = (a >> PAGE_BITS) as usize;
        let last = ((u64::from(a) + u64::from(len.max(1)) - 1) >> PAGE_BITS) as usize;
        if self.tracked[first].load(Relaxed) != 0 {
            self.mark(first);
        }
        if last != first {
            self.note_rest(first + 1, last);
        }
    }

    /// [`Track::note`]'s pages after the first (a write across a page end, or a long one).
    #[cold]
    #[inline(never)]
    fn note_rest(&self, from: usize, last: usize) {
        for p in from..=last.min(PAGES - 1) {
            if self.tracked[p].load(Relaxed) != 0 {
                self.mark(p);
            }
        }
    }

    /// Sets page `p`'s dirty bit (a plain load first: a page written again and again pays the
    /// atomic once).
    #[inline(always)]
    fn mark(&self, p: usize) {
        let word = &self.dirty[p / 64];
        let bit = 1u64 << (p % 64);
        if word.load(Relaxed) & bit == 0 {
            word.fetch_or(bit, Relaxed);
        }
    }

    /// Hands every page written since the last call to `f` (lowest first) and clears its bit.
    pub fn take_dirty(&self, mut f: impl FnMut(u32)) {
        for (i, word) in self.dirty.iter().enumerate() {
            if word.load(Relaxed) == 0 {
                continue;
            }
            let mut bits = word.swap(0, Relaxed);
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                f(((i * 64 + b) as u32) << PAGE_BITS);
            }
        }
    }
}
