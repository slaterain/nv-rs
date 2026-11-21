//! The store track ([`Mem::enable_track`]): which tracked pages of guest memory were written since
//! their owner last looked. Off unless enabled; off, no store pays more than the one flag test it
//! already made (`store_diag`).
//!
//! A reader that keeps its own copy of some of the game's objects (the culling copy) marks the
//! pages holding them as tracked ([`Track::set_tracked`], which also marks them dirty, so a copy
//! made before or after tracking starts is checked once); every write of guest memory through
//! [`Mem`] (stores of every width, [`Mem::write`] and `try_write`, [`Mem::zero`], frees, `map` and
//! `unmap`, the interlocked operations' words) to a tracked page sets that page's dirty bit;
//! [`Track::take_dirty`] hands the dirty pages over and clears them. The game's bytes are never
//! changed or held back: the track only says where to look again.
//!
//! **Threads.** One track serves the shared memory and all its views (made before
//! [`Mem::start_sharing`], carried into every view). The tracked flags change only while the owner
//! has the walk to itself (between frames). Every write marks its pages AFTER its bytes are written,
//! with a release `fetch_or`; [`Track::take_dirty`] clears the bits with an acquire swap. So when
//! the owner takes a page and then re-reads it, every write whose mark it took is visible to it, and
//! a write it did not see marks the page again for the next take: no store is lost.
//!
//! The interlocked words are the one exception to "after": [`Mem::atomic_u32`] hands out the word
//! and the caller's operation follows outside this module. Such a page is marked when the word is
//! handed out and marked "late" too: the take that clears its dirty bit marks it dirty again, so the
//! page comes out in the next take as well, by which time the operation (nanoseconds after the hand
//! out) has long been done.
//!
//! [`Mem`]: super::Mem
//! [`Mem::enable_track`]: super::Mem::enable_track
//! [`Mem::write`]: super::Mem::write
//! [`Mem::zero`]: super::Mem::zero
//! [`Mem::atomic_u32`]: super::Mem::atomic_u32
//! [`Mem::start_sharing`]: super::Mem::start_sharing

use super::{PAGES, PAGE_BITS};
use std::sync::atomic::{
    AtomicU64, AtomicU8,
    Ordering::{Acquire, Relaxed, Release},
};

/// The pages tracked and the pages written since they were last taken.
pub struct Track {
    /// One byte a page (and one past the last, always 0): nonzero when tracked.
    tracked: Box<[AtomicU8]>,
    /// One bit a page: written since [`Track::take_dirty`] last took it.
    dirty: Box<[AtomicU64]>,
    /// One bit a page: an interlocked word was handed out there ([`Track::note_atomic`]); the next
    /// take marks the page dirty again.
    late: Box<[AtomicU64]>,
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
            late: (0..PAGES.div_ceil(64)).map(|_| AtomicU64::new(0)).collect(),
        }
    }

    /// Tracks (or stops tracking) the pages `[addr, addr + len)` touches. A page tracked from now on
    /// is marked dirty, so the owner's next take checks it whatever was written before.
    pub fn set_tracked(&self, addr: u32, len: u32, on: bool) {
        if len == 0 {
            return;
        }
        let first = (addr >> PAGE_BITS) as usize;
        let last = ((u64::from(addr) + u64::from(len) - 1) >> PAGE_BITS) as usize;
        for p in first..=last.min(PAGES - 1) {
            self.tracked[p].store(u8::from(on), Relaxed);
            if on {
                self.mark(p);
            }
        }
    }

    /// Whether page `p` is tracked.
    pub fn is_tracked(&self, p: usize) -> bool {
        self.tracked[p].load(Relaxed) != 0
    }

    /// A write of `len` bytes (at least 1) at `a`, made: marks each tracked page it touched. Called
    /// after the bytes are written.
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

    /// The interlocked word at `a` handed out (its operation follows): marks its page dirty and
    /// late, when tracked.
    #[inline]
    pub(super) fn note_atomic(&self, a: u32) {
        let p = (a >> PAGE_BITS) as usize;
        if self.tracked[p].load(Relaxed) != 0 {
            self.late[p / 64].fetch_or(1u64 << (p % 64), Release);
            self.mark(p);
        }
    }

    /// Sets page `p`'s dirty bit, releasing the bytes written before it. Always the read-modify-
    /// write: skipping it when the bit looks set already could miss the owner clearing it at that
    /// moment, and the write would never be seen.
    #[inline(always)]
    fn mark(&self, p: usize) {
        self.dirty[p / 64].fetch_or(1u64 << (p % 64), Release);
    }

    /// Hands every page written since the last call to `f` (lowest first) and clears its bit. A page
    /// with an interlocked word handed out since is handed over and marked dirty again for the next
    /// call ([`Track::note_atomic`]).
    pub fn take_dirty(&self, mut f: impl FnMut(u32)) {
        for (i, (word, late)) in self.dirty.iter().zip(self.late.iter()).enumerate() {
            if word.load(Relaxed) == 0 && late.load(Relaxed) == 0 {
                continue;
            }
            let late_bits = late.swap(0, Acquire);
            let mut bits = word.swap(0, Acquire) | late_bits;
            if late_bits != 0 {
                word.fetch_or(late_bits, Release);
            }
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                f(((i * 64 + b) as u32) << PAGE_BITS);
            }
        }
    }
}
