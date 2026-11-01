//! The heap's free space: which addresses below the heap's top are free, and where a new block
//! goes. A platform layer like [`super::Mem::alloc`] itself (the game's
//! `MemoryManager::Allocate`, `HeapAlloc` and `VirtualAlloc` reservations are all served from
//! it), so its addresses never had to match the exe's.
//!
//! The heap runs from [`HEAP_BASE`] up to its top ([`Heap::top`]). Below the top every byte is
//! in a live block, a free block in a bin, or a block waiting in a quick stack. The policy is a
//! two-level segregated fit (the scheme known as TLSF):
//!
//! - Free blocks sit in bins by size: one bin per 8-byte size below [`EXACT_LIMIT`], then
//!   [`SUB_BINS`] bins per power of two, each holding sizes within about 3% of one another. A
//!   bin is a list with the newest block first, and a bitmap of the bins that hold a block finds
//!   the smallest such bin at or above a size in a few instructions.
//! - A request takes the newest block of its own bin that fits (one of exactly its size first,
//!   so in a bin of one size it is the newest freed block of that size), else the newest block
//!   of the next bin above that holds one (every block there fits), else fresh memory at the
//!   top. It looks at the first [`OWN_BIN_LOOK`] blocks of its own bin; only when nothing else
//!   is left does it search the whole bin, so the heap reports itself full only when no free
//!   block fits.
//! - A bigger block is split: the request takes its start and the rest stays free, so a block
//!   handed out always has exactly the size asked for.
//! - A freed block merges with its free neighbours on both sides; a free block that reaches the
//!   top lowers the top instead of going into a bin, so the top can come down again.
//! - In front of the bins, freed blocks of up to [`QUICK_MAX`] bytes wait in stacks of one size
//!   each, newest first, for the next request of their size, without any merging: the game makes
//!   and drops tens of thousands of small objects a frame, nearly all of sizes it has just
//!   freed, and a stack serves them as cheaply as the old exact-size lists did. When more than
//!   [`QUICK_BYTES`] wait, the older half of each stack goes on to the bins and merges there;
//!   when the heap is about to report itself full, all of them do.
//!
//! All the bookkeeping is in host tables keyed by address. Nothing is written into guest memory
//! (a freed block keeps what the freed-block check filled it with, and the write watch sees
//! only the game's own stores).

use super::{HEAP_BASE, HEAP_END};
use crate::engine::AddrMap;

/// Free blocks below this size have a bin of their own size each.
const EXACT_LIMIT: u32 = 512;
/// The bins of one size each (sizes 0 to [`EXACT_LIMIT`] - 8 by 8; size 0 is never used).
const EXACT_BINS: usize = (EXACT_LIMIT / 8) as usize;
/// The bins per power of two above [`EXACT_LIMIT`], as a shift (32 bins).
const SUB_SHIFT: u32 = 5;
const SUB_BINS: usize = 1 << SUB_SHIFT;
/// The power of two [`EXACT_LIMIT`] is.
const FIRST_POWER: u32 = EXACT_LIMIT.trailing_zeros();
/// Every bin: the exact ones, then the sub-bins of each power of two up to 2^31.
const BINS: usize = EXACT_BINS + (32 - FIRST_POWER as usize) * SUB_BINS;
/// The words of the bitmap of bins that hold a block.
const WORDS: usize = BINS.div_ceil(64);
const _: () = assert!(WORDS <= 32, "the bitmap of words must fit in a u32");
/// Freed blocks of up to this size wait in the quick stacks: the sizes the old exact-size lists
/// were indexed for. Through the bins, a free and an allocation of 264 bytes to 4 KB took
/// about half as long again as with those lists (`heap_speed` in `mem.rs`).
pub(super) const QUICK_MAX: u32 = 4096;
/// The bytes the quick stacks hold before the older half of each goes on to the bins. With
/// every multiple of 8 from 264 to 4096 in use, 1 MB holds about one block of each size, so
/// nearly every block went on and the stacks seldom served a request; 8 MB keeps a free and an
/// allocation of those sizes within about 12% of the old lists' time.
pub(super) const QUICK_BYTES: u64 = 8 << 20;
/// How many blocks of its own bin a request looks at for one that fits.
const OWN_BIN_LOOK: u32 = 8;
/// No node.
const NONE: u32 = u32::MAX;

/// The bin of a free block of `size` bytes (a multiple of 8, at least 8).
#[inline]
fn bin_of(size: u32) -> usize {
    if size < EXACT_LIMIT {
        (size >> 3) as usize
    } else {
        let power = 31 - size.leading_zeros();
        let sub = (size >> (power - SUB_SHIFT)) as usize & (SUB_BINS - 1);
        EXACT_BINS + (power - FIRST_POWER) as usize * SUB_BINS + sub
    }
}

/// A free block in a bin, linked into its bin's list by node number.
#[derive(Clone, Copy, Debug)]
struct Node {
    start: u32,
    size: u32,
    prev: u32,
    next: u32,
}

/// The heap's top and free space (see the module's description).
pub(super) struct Heap {
    /// The next fresh address: everything at or above it is unused.
    top: u32,
    /// The highest the top has been.
    peak: u32,
    /// The free blocks in bins, by node number, and the node numbers not in use.
    nodes: Vec<Node>,
    spare: Vec<u32>,
    /// The node of each free block in a bin, by its start and by its end (the address after
    /// it), to find a freed block's neighbours.
    by_start: AddrMap<u32, u32>,
    by_end: AddrMap<u32, u32>,
    /// The newest block of each bin.
    heads: [u32; BINS],
    /// The bins that hold a block (bit `b % 64` of word `b / 64`), and the words that are not 0.
    words: [u64; WORDS],
    nonempty: u32,
    /// Bytes in the bins.
    free_bytes: u64,
    /// The quick stacks by size / 8, newest last, and the bytes in them.
    quick: Vec<Vec<u32>>,
    quick_bytes: u64,
    /// The largest size the quick stacks take (0: none, for tests of the bins alone).
    quick_max: u32,
}

impl Heap {
    pub(super) fn new() -> Self {
        Heap {
            top: HEAP_BASE,
            peak: HEAP_BASE,
            nodes: Vec::new(),
            spare: Vec::new(),
            by_start: AddrMap::default(),
            by_end: AddrMap::default(),
            heads: [NONE; BINS],
            words: [0; WORDS],
            nonempty: 0,
            free_bytes: 0,
            quick: vec![Vec::new(); QUICK_MAX as usize / 8 + 1],
            quick_bytes: 0,
            quick_max: QUICK_MAX,
        }
    }

    /// The next fresh address. It comes down when the block below it is freed.
    pub(super) fn top(&self) -> u32 {
        self.top
    }

    /// The highest address the heap has reached: no block was ever handed out above it.
    pub(super) fn peak(&self) -> u32 {
        self.peak
    }

    /// A block of `size` bytes (a multiple of 8, at least 8), or `None` when the heap has no
    /// room for it.
    #[inline]
    pub(super) fn take(&mut self, size: u32) -> Option<u32> {
        if size <= self.quick_max {
            if let Some(a) = self.quick[(size >> 3) as usize].pop() {
                self.quick_bytes -= u64::from(size);
                return Some(a);
            }
        }
        self.take_free(size)
    }

    /// [`Heap::take`] past the quick stacks: the bins, then the top. When both fail, the whole
    /// of the request's own bin is searched (a fitting block can sit behind more than
    /// [`OWN_BIN_LOOK`] newer, smaller ones), then every quick stack goes to the bins and the
    /// bins and the top are tried again.
    fn take_free(&mut self, size: u32) -> Option<u32> {
        if let Some(n) = self.fit(size, OWN_BIN_LOOK) {
            return Some(self.carve(n, size));
        }
        if let Some(a) = self.grow(size) {
            return Some(a);
        }
        if let Some(n) = self.fit(size, u32::MAX) {
            return Some(self.carve(n, size));
        }
        if self.quick_bytes == 0 {
            return None;
        }
        self.flush_quick(true);
        if let Some(n) = self.fit(size, u32::MAX) {
            return Some(self.carve(n, size));
        }
        self.grow(size)
    }

    /// The free block a request of `size` bytes takes: in its own bin the newest of exactly
    /// that size or else the newest that fits (among the first `look` blocks of the bin), else
    /// the newest of the next bin above that holds one. Requests look at the first
    /// [`OWN_BIN_LOOK`], so that a bin of many smaller blocks does not slow every request down;
    /// only a request the heap would otherwise fail looks at the whole bin.
    #[inline]
    fn fit(&self, size: u32, look: u32) -> Option<u32> {
        let bin = bin_of(size);
        let mut n = self.heads[bin];
        let mut fits = NONE;
        let mut looked = 0;
        while n != NONE && looked < look {
            let node = &self.nodes[n as usize];
            if node.size == size {
                return Some(n);
            }
            if node.size > size && fits == NONE {
                fits = n;
            }
            n = node.next;
            looked += 1;
        }
        if fits != NONE {
            return Some(fits);
        }
        self.first_bin_from(bin + 1).map(|b| self.heads[b])
    }

    /// The first bin at or after `bin` that holds a block.
    #[inline]
    fn first_bin_from(&self, bin: usize) -> Option<usize> {
        let word = bin / 64;
        if word >= WORDS {
            return None;
        }
        let here = self.words[word] & (!0u64 << (bin % 64));
        if here != 0 {
            return Some(word * 64 + here.trailing_zeros() as usize);
        }
        let later = self.nonempty & (!0u32 << word << 1);
        if later == 0 {
            return None;
        }
        let word = later.trailing_zeros() as usize;
        Some(word * 64 + self.words[word].trailing_zeros() as usize)
    }

    /// Hands out the first `size` bytes of the free block `n`; the rest of it stays free.
    fn carve(&mut self, n: u32, size: u32) -> u32 {
        self.unlink(n);
        let Node {
            start, size: have, ..
        } = self.nodes[n as usize];
        self.by_start.remove(&start);
        self.free_bytes -= u64::from(size);
        if have == size {
            self.by_end.remove(&(start + have));
            self.spare.push(n);
        } else {
            // The rest keeps its end, so only its start moves; its right neighbour is not free
            // (free blocks are merged), so there is nothing to merge.
            let rest = start + size;
            let node = &mut self.nodes[n as usize];
            node.start = rest;
            node.size = have - size;
            self.by_start.insert(rest, n);
            self.link(n);
        }
        start
    }

    /// `size` fresh bytes at the top, or `None` past [`HEAP_END`].
    #[inline]
    fn grow(&mut self, size: u32) -> Option<u32> {
        let a = self.top;
        if u64::from(a) + u64::from(size) > u64::from(HEAP_END) {
            return None;
        }
        self.top = a + size;
        self.peak = self.peak.max(self.top);
        Some(a)
    }

    /// Takes back the block `[addr, addr + size)`: into its quick stack when it is small,
    /// else into the bins ([`Heap::release`]).
    #[inline]
    pub(super) fn give(&mut self, addr: u32, size: u32) {
        if size <= self.quick_max {
            self.quick[(size >> 3) as usize].push(addr);
            self.quick_bytes += u64::from(size);
            if self.quick_bytes > QUICK_BYTES {
                self.flush_quick(false);
            }
            return;
        }
        self.release(addr, size);
    }

    /// Puts the block `[addr, addr + size)` into the bins, merged with the free blocks on
    /// either side; a block that reaches the top lowers the top instead.
    pub(super) fn release(&mut self, addr: u32, size: u32) {
        let end = addr + size;
        if end == self.top {
            // No free block in a bin ends at the top, so only the left one can merge.
            let mut start = addr;
            if let Some(l) = self.by_end.remove(&addr) {
                self.unlink(l);
                let left = self.nodes[l as usize];
                self.by_start.remove(&left.start);
                self.free_bytes -= u64::from(left.size);
                self.spare.push(l);
                start = left.start;
            }
            self.top = start;
            return;
        }
        self.free_bytes += u64::from(size);
        let left = self.by_end.remove(&addr);
        let right = self.by_start.remove(&end);
        let n = match (left, right) {
            (None, None) => {
                let n = self.new_node(addr, size);
                self.by_start.insert(addr, n);
                self.by_end.insert(end, n);
                n
            }
            (Some(l), None) => {
                // The left block keeps its start and grows to the freed block's end.
                self.unlink(l);
                self.nodes[l as usize].size += size;
                self.by_end.insert(end, l);
                l
            }
            (None, Some(r)) => {
                // The right block keeps its end and grows down to the freed block's start.
                self.unlink(r);
                let node = &mut self.nodes[r as usize];
                node.start = addr;
                node.size += size;
                self.by_start.insert(addr, r);
                r
            }
            (Some(l), Some(r)) => {
                self.unlink(l);
                self.unlink(r);
                let right = self.nodes[r as usize];
                self.nodes[l as usize].size += size + right.size;
                *self
                    .by_end
                    .get_mut(&(right.start + right.size))
                    .expect("a free block's end is listed") = l;
                self.spare.push(r);
                l
            }
        };
        self.link(n);
    }

    /// Moves quick-stack blocks into the bins (merging there): all of them, or the older half
    /// of each stack.
    fn flush_quick(&mut self, all: bool) {
        for i in 1..self.quick.len() {
            let size = (i as u32) << 3;
            let mut stack = std::mem::take(&mut self.quick[i]);
            let n = if all {
                stack.len()
            } else {
                stack.len().div_ceil(2)
            };
            for a in stack.drain(..n) {
                self.quick_bytes -= u64::from(size);
                self.release(a, size);
            }
            self.quick[i] = stack;
        }
    }

    fn new_node(&mut self, start: u32, size: u32) -> u32 {
        let node = Node {
            start,
            size,
            prev: NONE,
            next: NONE,
        };
        match self.spare.pop() {
            Some(n) => {
                self.nodes[n as usize] = node;
                n
            }
            None => {
                self.nodes.push(node);
                (self.nodes.len() - 1) as u32
            }
        }
    }

    /// Puts node `n` at the head of its size's bin.
    #[inline]
    fn link(&mut self, n: u32) {
        let bin = bin_of(self.nodes[n as usize].size);
        let head = self.heads[bin];
        let node = &mut self.nodes[n as usize];
        node.prev = NONE;
        node.next = head;
        if head != NONE {
            self.nodes[head as usize].prev = n;
        }
        self.heads[bin] = n;
        self.words[bin / 64] |= 1 << (bin % 64);
        self.nonempty |= 1 << (bin / 64);
    }

    /// Takes node `n` out of its bin (its size must be the one it was linked with).
    #[inline]
    fn unlink(&mut self, n: u32) {
        let Node {
            size, prev, next, ..
        } = self.nodes[n as usize];
        if next != NONE {
            self.nodes[next as usize].prev = prev;
        }
        if prev != NONE {
            self.nodes[prev as usize].next = next;
            return;
        }
        let bin = bin_of(size);
        self.heads[bin] = next;
        if next == NONE {
            self.words[bin / 64] &= !(1 << (bin % 64));
            if self.words[bin / 64] == 0 {
                self.nonempty &= !(1 << (bin / 64));
            }
        }
    }

    /// Bytes and blocks in the bins.
    pub(super) fn free_bytes(&self) -> u64 {
        self.free_bytes
    }
    pub(super) fn free_blocks(&self) -> usize {
        self.by_start.len()
    }

    /// Bytes and blocks waiting in the quick stacks.
    pub(super) fn quick_bytes(&self) -> u64 {
        self.quick_bytes
    }
    pub(super) fn quick_blocks(&self) -> usize {
        self.quick.iter().map(Vec::len).sum()
    }

    /// The largest free block in the bins (0 when there is none).
    pub(super) fn largest_free(&self) -> u32 {
        let Some(top_bin) = (0..WORDS).rev().find(|&w| self.words[w] != 0) else {
            return 0;
        };
        let bin = top_bin * 64 + 63 - self.words[top_bin].leading_zeros() as usize;
        let mut largest = 0;
        let mut n = self.heads[bin];
        while n != NONE {
            largest = largest.max(self.nodes[n as usize].size);
            n = self.nodes[n as usize].next;
        }
        largest
    }

    /// Every free block's size, in the bins and in the quick stacks (for the heap's report).
    pub(super) fn free_sizes(&self) -> impl Iterator<Item = u32> + '_ {
        let binned = self.by_start.values().map(|&n| self.nodes[n as usize].size);
        let waiting = self
            .quick
            .iter()
            .enumerate()
            .flat_map(|(i, s)| std::iter::repeat((i as u32) << 3).take(s.len()));
        binned.chain(waiting)
    }

    /// Turns the quick stacks off (tests of the bins alone).
    #[cfg(test)]
    pub(super) fn without_quick(mut self) -> Self {
        self.quick_max = 0;
        self
    }

    /// The free block in a bin that starts at `start`, as (start, size).
    #[cfg(test)]
    pub(super) fn free_block_at(&self, start: u32) -> Option<(u32, u32)> {
        self.by_start
            .get(&start)
            .map(|&n| (self.nodes[n as usize].start, self.nodes[n as usize].size))
    }

    /// Checks the bookkeeping: the bins, their lists and bitmaps, both maps and the counts agree;
    /// no two free blocks in bins touch and none ends at the top; and the free blocks, the quick
    /// stacks and `held` (the live blocks and any held back by the freed-block check) cover
    /// `[HEAP_BASE, top)` exactly, without overlap. Panics on the first disagreement.
    #[cfg(test)]
    pub(super) fn check(&self, held: impl IntoIterator<Item = (u32, u32)>) {
        let mut listed = 0;
        let mut binned = 0u64;
        for bin in 0..BINS {
            let mut prev = NONE;
            let mut n = self.heads[bin];
            assert_eq!(
                n != NONE,
                (self.words[bin / 64] >> (bin % 64)) & 1 == 1,
                "bitmap of bin {bin}"
            );
            while n != NONE {
                let node = self.nodes[n as usize];
                assert_eq!(
                    bin_of(node.size),
                    bin,
                    "block {:08x} in bin {bin}",
                    node.start
                );
                assert_eq!(node.prev, prev, "back link of node {n}");
                assert_eq!(self.by_start.get(&node.start), Some(&n));
                assert_eq!(self.by_end.get(&(node.start + node.size)), Some(&n));
                assert!(node.size >= 8 && node.size % 8 == 0 && node.start % 8 == 0);
                listed += 1;
                binned += u64::from(node.size);
                prev = n;
                n = node.next;
            }
        }
        for w in 0..WORDS {
            assert_eq!(
                self.words[w] != 0,
                (self.nonempty >> w) & 1 == 1,
                "word {w}"
            );
        }
        assert_eq!(listed, self.by_start.len());
        assert_eq!(listed, self.by_end.len());
        assert_eq!(binned, self.free_bytes);
        assert_eq!(listed + self.spare.len(), self.nodes.len());
        // 0 live, 1 free in a bin, 2 waiting in a quick stack.
        let mut all: Vec<(u32, u32, u8)> = held.into_iter().map(|(a, s)| (a, s, 0)).collect();
        all.extend(self.by_start.values().map(|&n| {
            let node = self.nodes[n as usize];
            (node.start, node.size, 1)
        }));
        let mut waiting = 0u64;
        for (i, stack) in self.quick.iter().enumerate() {
            for &a in stack {
                all.push((a, (i as u32) << 3, 2));
                waiting += u64::from(i as u32) << 3;
            }
        }
        assert_eq!(waiting, self.quick_bytes);
        all.sort_unstable();
        let mut at = HEAP_BASE;
        let mut last_kind = 0;
        for &(a, s, kind) in &all {
            assert_eq!(
                a, at,
                "a gap or an overlap at {at:08x} (block {a:08x}+{s:x})"
            );
            assert!(
                !(kind == 1 && last_kind == 1),
                "free blocks touch at {a:08x} without being merged"
            );
            at = a + s;
            last_kind = kind;
        }
        assert_eq!(
            at, self.top,
            "the blocks end at {at:08x}, the top is {:08x}",
            self.top
        );
        assert!(last_kind != 1, "a free block in a bin ends at the top");
        assert!(self.peak >= self.top);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bins: one per size below 512, then 32 per power of two, in increasing order of size.
    #[test]
    fn bins_are_exact_below_512_then_32_per_power_of_two() {
        assert_eq!(bin_of(8), 1);
        assert_eq!(bin_of(504), 63);
        assert_eq!(bin_of(512), 64);
        assert_eq!(bin_of(520), 64);
        assert_eq!(bin_of(528), 65);
        assert_eq!(bin_of(1016), 95);
        assert_eq!(bin_of(1024), 96);
        assert_eq!(bin_of(1056), 97);
        assert_eq!(bin_of(0xffff_fff8), BINS - 1);
        let mut last = 0;
        for size in (8..1 << 20).step_by(8) {
            let b = bin_of(size);
            assert!(b == last || b == last + 1, "{size}: {b} after {last}");
            last = b;
        }
    }

    /// A bigger free block is split: the request takes its start, the rest stays free.
    #[test]
    fn a_bigger_free_block_is_split_from_its_start() {
        let mut h = Heap::new().without_quick();
        let a = h.take(0x1000).unwrap();
        let b = h.take(0x40).unwrap();
        h.release(a, 0x1000);
        assert_eq!(h.take(0x300), Some(a));
        assert_eq!(h.free_block_at(a + 0x300), Some((a + 0x300, 0xd00)));
        assert_eq!(h.take(0xd00), Some(a + 0x300));
        assert_eq!(h.free_blocks(), 0);
        h.check([(a, 0x300), (a + 0x300, 0xd00), (b, 0x40)]);
    }

    /// A freed block merges with a free block on its left, on its right, and on both sides.
    #[test]
    fn freed_blocks_merge_with_free_neighbours() {
        let mut h = Heap::new().without_quick();
        let blocks: Vec<u32> = (0..7).map(|_| h.take(0x100).unwrap()).collect();
        // Left: 0 then 1.
        h.release(blocks[0], 0x100);
        h.release(blocks[1], 0x100);
        assert_eq!(h.free_block_at(blocks[0]), Some((blocks[0], 0x200)));
        // Right: 4 then 3.
        h.release(blocks[4], 0x100);
        h.release(blocks[3], 0x100);
        assert_eq!(h.free_block_at(blocks[3]), Some((blocks[3], 0x200)));
        assert_eq!(h.free_block_at(blocks[4]), None);
        // Both: 2 joins [0, 2) and [3, 5).
        h.release(blocks[2], 0x100);
        assert_eq!(h.free_block_at(blocks[0]), Some((blocks[0], 0x500)));
        assert_eq!(h.free_blocks(), 1);
        h.check([(blocks[5], 0x100), (blocks[6], 0x100)]);
        // The merged block serves one request of its whole size.
        assert_eq!(h.take(0x500), Some(blocks[0]));
        assert_eq!(h.free_blocks(), 0);
    }

    /// A freed block at the top lowers the top, together with a free block below it.
    #[test]
    fn a_freed_block_at_the_top_lowers_it() {
        let mut h = Heap::new().without_quick();
        let a = h.take(0x800).unwrap();
        let b = h.take(0x800).unwrap();
        let c = h.take(0x800).unwrap();
        assert_eq!(h.top(), c + 0x800);
        h.release(b, 0x800);
        h.release(c, 0x800);
        assert_eq!(h.top(), b);
        assert_eq!(h.free_blocks(), 0);
        assert_eq!(h.peak(), c + 0x800);
        h.release(a, 0x800);
        assert_eq!(h.top(), HEAP_BASE);
        h.check([]);
        // Fresh blocks come from the lowered top again.
        assert_eq!(h.take(0x10), Some(HEAP_BASE));
    }

    /// Blocks of one size come back newest first, as the old exact-size lists gave them, both
    /// from the quick stacks and from exact bins (which a block reaches when the quick stacks
    /// let it go).
    #[test]
    fn same_size_reuse_is_newest_first() {
        for (size, quick) in [(0x20, true), (0x180, true), (0x180, false)] {
            let mut h = if quick {
                Heap::new()
            } else {
                Heap::new().without_quick()
            };
            let blocks: Vec<u32> = (0..6).map(|_| h.take(size).unwrap()).collect();
            // Every other block, so none merge.
            for a in [blocks[0], blocks[2], blocks[4]] {
                h.give(a, size);
            }
            assert_eq!(h.take(size), Some(blocks[4]));
            assert_eq!(h.take(size), Some(blocks[2]));
            assert_eq!(h.take(size), Some(blocks[0]));
            assert_eq!(h.take(size), Some(blocks[5] + size));
        }
    }

    /// In a bin of several sizes, a block of exactly the size asked for is taken before a newer
    /// one that is bigger.
    #[test]
    fn an_exact_block_is_taken_before_a_bigger_newer_one_in_the_bin() {
        let mut h = Heap::new().without_quick();
        let a = h.take(592).unwrap();
        let _ = h.take(8).unwrap();
        let b = h.take(600).unwrap();
        let _ = h.take(8).unwrap();
        assert_eq!(bin_of(592), bin_of(600));
        h.release(a, 592);
        h.release(b, 600);
        assert_eq!(h.take(592), Some(a));
        assert_eq!(h.take(592), Some(b));
        assert_eq!(h.free_block_at(b + 592), Some((b + 592, 8)));
    }

    /// Freed blocks of up to 4 KB (the old exact-size lists' sizes) wait in the quick stacks;
    /// bigger ones go to the bins.
    #[test]
    fn blocks_up_to_4k_wait_in_the_quick_stacks() {
        let mut h = Heap::new();
        let a = h.take(0x1000).unwrap();
        let b = h.take(0x1008).unwrap();
        let last = h.take(8).unwrap();
        h.give(a, 0x1000);
        h.give(b, 0x1008);
        assert_eq!((h.quick_blocks(), h.quick_bytes()), (1, 0x1000));
        assert_eq!(h.free_block_at(b), Some((b, 0x1008)));
        assert_eq!(h.take(0x1000), Some(a));
        h.check([(a, 0x1000), (last, 8)]);
    }

    /// Past [`QUICK_BYTES`] the older half of each quick stack goes to the bins and merges there.
    #[test]
    fn the_quick_stacks_spill_their_older_half_into_the_bins() {
        let mut h = Heap::new();
        let n = (QUICK_BYTES / 0x100) as u32 + 1;
        let blocks: Vec<u32> = (0..n).map(|_| h.take(0x100).unwrap()).collect();
        let last = h.take(0x10).unwrap();
        for &a in &blocks {
            h.give(a, 0x100);
        }
        // The first half (the oldest) merged into one free block; the newest wait.
        assert!(
            h.quick_bytes() <= QUICK_BYTES / 2 + 0x100,
            "{}",
            h.quick_bytes()
        );
        assert_eq!(h.free_blocks(), 1);
        let (start, size) = h.free_block_at(blocks[0]).unwrap();
        assert_eq!((start, size), (blocks[0], n.div_ceil(2) * 0x100));
        assert_eq!(h.take(0x100), Some(*blocks.last().unwrap()));
        h.check([(last, 0x10), (*blocks.last().unwrap(), 0x100)]);
    }

    /// When nothing else fits, the quick stacks go to the bins, merge, and serve the request.
    #[test]
    fn a_full_heap_merges_the_quick_stacks_before_giving_up() {
        let mut h = Heap::new();
        let a = h.take(0x40).unwrap();
        let b = h.take(0x40).unwrap();
        let rest = HEAP_END - h.top();
        let c = h.take(rest).unwrap();
        assert_eq!(h.take(8), None);
        h.give(a, 0x40);
        h.give(b, 0x40);
        assert_eq!(h.quick_bytes(), 0x80);
        assert_eq!(h.take(0x80), Some(a));
        assert_eq!(h.quick_bytes(), 0);
        h.check([(a, 0x80), (c, rest)]);
    }

    /// On a full heap, a free block that fits is found even behind more than [`OWN_BIN_LOOK`]
    /// newer blocks of its bin that are too small.
    #[test]
    fn a_full_heap_finds_a_fitting_block_deep_in_its_bin() {
        let mut h = Heap::new().without_quick();
        let fitting = h.take(600).unwrap();
        let mut held = vec![(fitting, 600)];
        let mut small = Vec::new();
        held.push((h.take(8).unwrap(), 8));
        for _ in 0..OWN_BIN_LOOK {
            small.push(h.take(592).unwrap());
            held.push((h.take(8).unwrap(), 8));
        }
        assert_eq!(bin_of(592), bin_of(600));
        let rest = HEAP_END - h.top();
        held.push((h.take(rest).unwrap(), rest));
        assert_eq!(h.take(8), None);
        // The 8-byte blocks between them keep the freed blocks apart.
        h.release(fitting, 600);
        for &a in &small {
            h.release(a, 592);
        }
        held.retain(|&(a, _)| a != fitting && !small.contains(&a));
        h.check(held.iter().copied());
        assert_eq!(h.take(600), Some(fitting));
        assert_eq!(h.take(600), None);
        held.push((fitting, 600));
        h.check(held.iter().copied());
    }
}
