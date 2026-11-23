//! The drawing thread's copied state as the engine that draws holds it (`Engine::own_view`): the bytes the cut
//! copied from guest memory, by object and by address, while the world draw's halves run on that engine.
//!
//! The draw family's value packet (`engine_typed::draw::capture`) keeps the copy between the cut and the join on a
//! thread of its own. While a half draws, the parts its accessors read (the own state's blocks and bytes, the
//! copied object ranges and their bytes) are moved onto the drawing engine here, so that every read and write is
//! a field of the engine and an index: no thread-local, no borrow, no run check, and for the own state no search
//! (a page table built once per half finds the one or two blocks an address can lie in). The packet takes them
//! back when the half ends. Nothing here knows the game: it is bytes by guest address.

use crate::AddrMap;
use std::cell::Cell;

/// One copied range of an object: its offsets `[from, to)`, where its bytes start in [`OwnView::bytes`], and the
/// next range of the same object ([`NONE`] for none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub from: u32,
    pub to: u32,
    pub at: u32,
    pub next: u32,
}

/// No range.
pub const NONE: u32 = u32::MAX;

/// A block of the draw's own state: the guest addresses `[lo, hi)`, held at `at` in [`OwnView::own_bytes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnBlock {
    pub lo: u32,
    pub hi: u32,
    pub at: u32,
}

/// A page's entry in the own state's page table: the page number plus one (0: empty), and where its blocks'
/// indices start in the list and how many there are.
#[derive(Clone, Copy, Default)]
struct Slot {
    tag: u32,
    first: u32,
    count: u32,
}

/// The guest page size the own state's table is kept by.
const PAGE_SHIFT: u32 = 12;

/// The copied state one engine draws from (see the module's words).
#[derive(Default)]
pub struct OwnView {
    /// The own state's blocks (apart from each other) and their bytes, as the draw reads and writes them.
    pub own: Vec<OwnBlock>,
    pub own_bytes: Vec<u8>,
    /// The copied objects' first ranges, the ranges and their bytes (read only).
    pub objects: AddrMap<u32, u32>,
    pub ranges: Vec<Range>,
    pub bytes: Vec<u8>,
    /// Reads answered and reads of a range not copied, while this engine drew.
    pub reads: Cell<u64>,
    pub misses: Cell<u64>,
    /// POISON: the guest bytes are scribbled meanwhile (a read not copied is a stop).
    pub poison: bool,
    /// The world draw's per-frame list words handed to the drawing side at the cut (`engine_typed::draw::detach`),
    /// by guest address: the draw's list walks and clears read and write them here.
    pub lists: AddrMap<u32, u32>,
    /// The object [`OwnView::find`] looked up last and its first range.
    last_object: Cell<(u32, u32)>,
    /// The own state's page table (open addressing, a power of two long) and the block indices it points into.
    slots: Vec<Slot>,
    list: Vec<u32>,
}

impl OwnView {
    /// Builds the own state's page table from [`OwnView::own`] (after the blocks were put in), and clears the
    /// object look-up's cache.
    pub fn index(&mut self) {
        self.last_object.set((0, NONE));
        // Every (page, block) pair, in page order.
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for (i, b) in (0u32..).zip(&self.own) {
            if b.lo >= b.hi {
                continue;
            }
            for page in (b.lo >> PAGE_SHIFT)..=((b.hi - 1) >> PAGE_SHIFT) {
                pairs.push((page, i));
            }
        }
        pairs.sort_unstable();
        let mut pages = pairs.len();
        pages -= pairs.windows(2).filter(|w| w[0].0 == w[1].0).count();
        let len = (2 * pages).next_power_of_two().max(16);
        self.slots.clear();
        self.slots.resize(len, Slot::default());
        self.list.clear();
        let mut k = 0;
        while k < pairs.len() {
            let page = pairs[k].0;
            let first = self.list.len() as u32;
            while k < pairs.len() && pairs[k].0 == page {
                self.list.push(pairs[k].1);
                k += 1;
            }
            let count = self.list.len() as u32 - first;
            let mut i = hash(page) & (len - 1);
            while self.slots[i].tag != 0 {
                i = (i + 1) & (len - 1);
            }
            self.slots[i] = Slot {
                tag: page + 1,
                first,
                count,
            };
        }
    }

    /// Where the own bytes of `[addr, addr + len)` start, when one own block holds them all.
    #[inline(always)]
    pub fn own_at(&self, addr: u32, len: u32) -> Option<usize> {
        let mask = self.slots.len().wrapping_sub(1);
        if self.slots.is_empty() {
            return None;
        }
        let page = addr >> PAGE_SHIFT;
        let mut i = hash(page) & mask;
        loop {
            let s = self.slots[i];
            if s.tag == page + 1 {
                let end = addr.checked_add(len)?;
                for &b in &self.list[s.first as usize..(s.first + s.count) as usize] {
                    let b = self.own[b as usize];
                    if addr >= b.lo && end <= b.hi {
                        return Some((b.at + (addr - b.lo)) as usize);
                    }
                }
                return None;
            }
            if s.tag == 0 {
                return None;
            }
            i = (i + 1) & mask;
        }
    }

    /// The range of `object` that holds `[offset, offset + len)`, if one does.
    #[inline(always)]
    pub fn find(&self, object: u32, offset: u32, len: u32) -> Option<Range> {
        let (last, first) = self.last_object.get();
        let mut index = if last == object && first != NONE {
            first
        } else {
            let first = *self.objects.get(&object)?;
            self.last_object.set((object, first));
            first
        };
        while index != NONE {
            let r = self.ranges[index as usize];
            if offset >= r.from && offset.checked_add(len).is_some_and(|end| end <= r.to) {
                return Some(r);
            }
            index = r.next;
        }
        None
    }

    /// Fills `out` with the bytes at `object + offset` as the copy holds them, from one range or several; false
    /// when one of them is in none. Counted: a read, or a miss.
    pub fn read_into(&self, object: u32, offset: u32, out: &mut [u8]) -> bool {
        let found = self.read_uncounted(object, offset, out);
        let count = if found { &self.reads } else { &self.misses };
        count.set(count.get() + 1);
        found
    }

    fn read_uncounted(&self, object: u32, offset: u32, out: &mut [u8]) -> bool {
        let len = u32::try_from(out.len()).unwrap_or(u32::MAX);
        if let Some(r) = self.find(object, offset, len) {
            let at = (r.at + (offset - r.from)) as usize;
            out.copy_from_slice(&self.bytes[at..at + out.len()]);
            return true;
        }
        for (i, slot) in (0u32..).zip(out.iter_mut()) {
            let Some((o, r)) = offset
                .checked_add(i)
                .and_then(|o| self.find(object, o, 1).map(|r| (o, r)))
            else {
                return false;
            };
            *slot = self.bytes[(r.at + (o - r.from)) as usize];
        }
        true
    }
}

/// The page table's hash of a page number.
#[inline(always)]
fn hash(page: u32) -> usize {
    (page.wrapping_mul(0x9e37_79b1) >> 16) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(blocks: &[(u32, u32)]) -> OwnView {
        let mut v = OwnView::default();
        let mut at = 0;
        for &(lo, hi) in blocks {
            v.own.push(OwnBlock { lo, hi, at });
            at += hi - lo;
        }
        v.own_bytes = (0..at).map(|i| i as u8).collect();
        v.index();
        v
    }

    #[test]
    fn an_address_is_found_in_its_block_and_nowhere_else() {
        let v = view(&[
            (0x011f_a000, 0x011f_fe40),
            (0x0200_0010, 0x0200_0020),
            (0x0200_0f00, 0x0200_1100),
        ]);
        assert_eq!(v.own_at(0x011f_a000, 4), Some(0));
        assert_eq!(v.own_at(0x011f_c004, 4), Some(0x2004));
        // The last bytes of a block, and one past it.
        assert_eq!(v.own_at(0x011f_fe3c, 4), Some(0x5e3c));
        assert_eq!(v.own_at(0x011f_fe3d, 4), None);
        // Two blocks in one page: each found, the gap between them not.
        let second = 0x5e40;
        assert_eq!(v.own_at(0x0200_0010, 4), Some(second));
        assert_eq!(v.own_at(0x0200_0f00, 4), Some(second + 0x10));
        assert_eq!(v.own_at(0x0200_0020, 4), None);
        // A block over a page border is found from both pages.
        assert_eq!(v.own_at(0x0200_0ffe, 4), Some(second + 0x10 + 0xfe));
        assert_eq!(v.own_at(0x0200_1000, 4), Some(second + 0x10 + 0x100));
        // A value across a block's end is in none; addresses near the top do not wrap.
        assert_eq!(v.own_at(0x0200_10fe, 4), None);
        assert_eq!(v.own_at(0xffff_fffe, 4), None);
        assert_eq!(v.own_at(0x0300_0000, 4), None);
    }

    #[test]
    fn many_pages_and_an_empty_view() {
        let blocks: Vec<(u32, u32)> = (0..300u32)
            .map(|i| (0x1000_0000 + i * 0x3000, 0x1000_0000 + i * 0x3000 + 8))
            .collect();
        let v = view(&blocks);
        for (i, &(lo, _)) in blocks.iter().enumerate() {
            assert_eq!(v.own_at(lo + 4, 4), Some(i * 8 + 4));
            assert_eq!(v.own_at(lo + 0x1000, 4), None);
        }
        let empty = OwnView::default();
        assert_eq!(empty.own_at(0x1000, 4), None);
    }

    #[test]
    fn copied_ranges_are_read_by_object_and_counted() {
        let mut v = OwnView::default();
        // Object 0x5000: offsets [0, 8) at 0 and [8, 0x10) at 8 (two ranges, chained).
        v.ranges.push(Range {
            from: 8,
            to: 0x10,
            at: 8,
            next: NONE,
        });
        v.ranges.push(Range {
            from: 0,
            to: 8,
            at: 0,
            next: 0,
        });
        v.objects.insert(0x5000, 1);
        v.bytes = (0..0x10u8).collect();
        let mut four = [0u8; 4];
        assert!(v.read_into(0x5000, 4, &mut four));
        assert_eq!(four, [4, 5, 6, 7]);
        // Across the two ranges.
        assert!(v.read_into(0x5000, 6, &mut four));
        assert_eq!(four, [6, 7, 8, 9]);
        // Past the copy, and another object.
        assert!(!v.read_into(0x5000, 0xe, &mut four));
        assert!(!v.read_into(0x6000, 0, &mut four));
        // One more answered read, so that reads and misses differ (a swap of the two counts shows).
        assert!(v.read_into(0x5000, 0, &mut four));
        assert_eq!((v.reads.get(), v.misses.get()), (3, 2));
    }
}
