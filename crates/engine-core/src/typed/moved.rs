//! Stage 2 of the typed migration (ADR-0014; docs/ENGINE_CRATE.md, "Moved families"): a typed
//! family's objects moved out of the game's heap into host memory, with the parts every family uses.
//!
//! - **The moved area** ([`MOVED_AREA`], `0x10000000..0x20000000`, below the heap: ordinary play never
//!   used it, and `VirtualAlloc` at a chosen address is refused there), in three parts, none of them
//!   ever mapped as the game's memory but the stubs: [`SLAB_RANGE`] names the bytes of the [`Slab`]
//!   (a moved object's handle is an address there, so an untyped read or write of a moved object
//!   stops the run naming the address), [`TOKEN_RANGE`] the starts of the [`Buffers`], and
//!   [`STUB_RANGE`] holds 16-byte stubs ([`Stubs`]) for objects untyped code still reaches through
//!   their vtable. Below 0x80000000, so none ever has its top bit set.
//! - **Two shapes of host memory.** The **slab**: small objects that keep their PC layout and point
//!   into each other (a link field, an element of a little array), every word an atomic (loads
//!   acquire, stores release, compare-and-swap sequentially consistent: what real threads need where
//!   the game uses interlocked operations, from the first step); its handles cover its bytes, so it
//!   is for families whose bytes are few. The **buffers**: big arrays (vertex arrays, keys) behind
//!   start-only tokens, 16 bytes of the 32-bit space each whatever their size, so no 32-bit pointer
//!   into one exists and typed code indexes the host bytes; plain bytes copied plainly, ordered by
//!   the game's own hand-off between threads as guest memory's plain accesses are (`mem.rs`,
//!   "Threads"). A freed buffer's bytes go back to the host only when no thread can be inside an
//!   access ([`Buffers::reclaim`]).
//! - **Reuse.** Freed slab blocks wait in a quarantine filled with the guest heap's freed words
//!   ([`Slab::free`], [`SLAB_QUARANTINE`]); freed stubs and tokens wait too
//!   ([`ADDRESS_QUARANTINE`]). A stale handle, stub or token so reads freed words or poison, or
//!   stops, instead of reaching the next object given its address: a moved family does not
//!   reproduce the exe's reuse order, it makes a use after free show.
//! - **K1 stubs** ([`K1`]): the object's original guest block stays, holding what untyped code still
//!   reads; a spare word holds the handle of its moved record, and the moved fields hold
//!   [`poison`]: a word in the last 64 KB of the space (never mapped) naming the family and the
//!   field, so a missed reader that follows it stops there, and one that counts with it gets about
//!   four billion. The poison is checked again ([`K1::poison_intact`]), which catches a missed writer.
//! - **The store** ([`Store`]): what a family's typed code reads and writes its objects through, the
//!   same code in every form: [`Store::Guest`] (stage 1), [`Store::Dual`] (the guest objects the
//!   truth, each one mirrored in the slab and every access compared: the dual run) and
//!   [`Store::Moved`] (the slab the truth). With `shadows` (K1) every guest allocation the family
//!   made before is still made, of the same size and in the same order, filled with poison and freed
//!   at the same point, so the guest heap, and every guest address handed out after (the Direct3D
//!   model's objects, which the recorded stream names by address), stays as it was; without (K2) the
//!   guest bytes are freed.
//! - **The dual run** ([`Dual`]): the mirror table (a guest block and its slab mirror), a word's
//!   translation (a word inside a mirrored block becomes the same place in its mirror, so pointer
//!   fields compare without knowing their types), and the count of comparisons and differences, the
//!   first [`DUAL_PRINT`] of them printed with the family, the place and the function running.
//!
//! What a family decides: which of its objects move, which fields stay in a K1 stub and which word
//! is spare, which allocations are its own (made through [`Store::alloc`]), and which calls by address
//! of its stage-1 code act on its own objects (made through [`Store::cas`] and the like, so the moved
//! form does them in place). Handles cross calls by address unchanged: a typed function reached by
//! address tests the range ([`is_handle`]).

use super::FamilyId;
use crate::mem::Mem;
use crate::ptr::{Field, Ptr};
use crate::Engine;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ops::Range;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The moved area: below the heap ([`crate::mem::HEAP_BASE`]) and above everything the image and
/// the tests use; `VirtualAlloc` at a chosen address fails inside it.
pub const MOVED_AREA: Range<u32> = 0x1000_0000..0x2000_0000;
/// The slab's handles ([`Slab`]), 64 MB: never mapped as guest memory.
pub const SLAB_RANGE: Range<u32> = 0x1000_0000..0x1400_0000;
/// The buffers' start-only tokens ([`Buffers`]), 4 M of them: never mapped as guest memory.
pub const TOKEN_RANGE: Range<u32> = 0x1400_0000..0x1800_0000;
/// The 16-byte stubs ([`Stubs`]), 8 M of them: mapped page by page as stubs are handed out.
pub const STUB_RANGE: Range<u32> = 0x1800_0000..0x2000_0000;
/// A stub's bytes, and the step between two tokens.
pub const STUB_BYTES: u32 = 16;
const _: () = assert!(
    MOVED_AREA.start == SLAB_RANGE.start
        && SLAB_RANGE.end == TOKEN_RANGE.start
        && TOKEN_RANGE.end == STUB_RANGE.start
        && STUB_RANGE.end == MOVED_AREA.end
        && MOVED_AREA.end <= crate::mem::HEAP_BASE
);

/// Whether `a` is a buffer's token.
#[inline(always)]
pub fn is_token(a: u32) -> bool {
    a.wrapping_sub(TOKEN_RANGE.start) < TOKEN_RANGE.end - TOKEN_RANGE.start
}

/// Whether `a` is a slab handle (or a place inside a moved object).
#[inline(always)]
pub fn is_handle(a: u32) -> bool {
    a.wrapping_sub(SLAB_RANGE.start) < SLAB_RANGE.end - SLAB_RANGE.start
}

/// Whether `a` is in the stub range.
#[inline(always)]
pub fn is_stub(a: u32) -> bool {
    a.wrapping_sub(STUB_RANGE.start) < STUB_RANGE.end - STUB_RANGE.start
}

/// The first word of the poison: the last 64 KB of the guest space, which is never mapped.
pub const POISON_BASE: u32 = 0xFFFF_0000;

/// The poison of `family`'s field at `offset` (its low 8 bits, word-aligned): `0xFFFF_ffoo`.
pub const fn poison(family: FamilyId, offset: u32) -> u32 {
    POISON_BASE | (family.number() as u32) << 8 | (offset & 0xfc)
}

/// The family number and offset a poison word names; `None` for any other word (one whose
/// family byte is no family's number, as -1, -4 or -256 are).
pub fn poison_parts(word: u32) -> Option<(u8, u8)> {
    let family = byte(word >> 8);
    (word & 0xffff_0000 == POISON_BASE && word & 3 == 0 && usize::from(family) < FAMILIES)
        .then(|| (family, byte(word)))
}

/// A word of the guest space as an index (widening: the guest space needs a 64-bit host).
#[allow(clippy::cast_possible_truncation)]
#[inline(always)]
fn index(x: u32) -> usize {
    x as usize
}

/// The low byte of `w`: where the cut is what the code means.
#[allow(clippy::cast_possible_truncation)]
fn byte(w: u32) -> u8 {
    w as u8
}

/// A field value the store moves as one word: a number or a pointer (a family adds its own, such
/// as a pointer with a mark bit).
pub trait Word: Copy {
    fn from_word(w: u32) -> Self;
    fn to_word(self) -> u32;
}

impl Word for u32 {
    #[inline(always)]
    fn from_word(w: u32) -> Self {
        w
    }
    #[inline(always)]
    fn to_word(self) -> u32 {
        self
    }
}

impl Word for i32 {
    #[inline(always)]
    fn from_word(w: u32) -> Self {
        i32::from_ne_bytes(w.to_ne_bytes())
    }
    #[inline(always)]
    fn to_word(self) -> u32 {
        u32::from_ne_bytes(self.to_ne_bytes())
    }
}

impl<T> Word for Ptr<T> {
    #[inline(always)]
    fn from_word(w: u32) -> Self {
        Ptr::new(w)
    }
    #[inline(always)]
    fn to_word(self) -> u32 {
        self.addr()
    }
}

/// `m` locked; a lock a panic left poisoned is taken as it is (the panic stops the run anyway).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A lock-free array of `T`s in chunks that never move once made: `get` is one load of the chunk's
/// pointer and an index; `make` (under its owner's lock) adds the chunks a range of entries needs.
struct Chunks<T> {
    chunks: Box<[AtomicPtr<T>]>,
    /// Entries a chunk (a power of two).
    per: usize,
}

impl<T: Default> Chunks<T> {
    fn new(entries: usize, per: usize) -> Self {
        debug_assert!(per.is_power_of_two() && entries % per == 0);
        Chunks {
            chunks: (0..entries / per)
                .map(|_| AtomicPtr::new(std::ptr::null_mut()))
                .collect(),
            per,
        }
    }

    /// Entry `i`, if its chunk has been made.
    #[inline(always)]
    fn get(&self, i: usize) -> Option<&T> {
        let chunk = self.chunks.get(i / self.per)?.load(Ordering::Acquire);
        if chunk.is_null() {
            return None;
        }
        // SAFETY: a non-null chunk pointer is a leaked `Box<[T]>` of `per` entries, freed only when
        // `self` is dropped, and `i % per` is below `per`.
        Some(unsafe { &*chunk.add(i % self.per) })
    }

    /// Makes the chunks entries `range` lie in. Its caller holds the lock that orders every `make`.
    fn make(&self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        for c in range.start / self.per..=(range.end - 1) / self.per {
            if self.chunks[c].load(Ordering::Acquire).is_null() {
                let chunk: Box<[T]> = (0..self.per).map(|_| T::default()).collect();
                self.chunks[c].store(Box::into_raw(chunk).cast::<T>(), Ordering::Release);
            }
        }
    }
}

impl<T> Drop for Chunks<T> {
    fn drop(&mut self) {
        for c in self.chunks.iter() {
            let p = c.load(Ordering::Acquire);
            if !p.is_null() {
                // SAFETY: made by `make` from a `Box<[T]>` of `per` entries; nothing uses it after
                // the drop.
                drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, self.per)) });
            }
        }
    }
}

/// Bytes of the slab's chunks (1 MB).
const SLAB_CHUNK_WORDS: usize = 1 << 18;

/// A block of the slab, as its allocator keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    /// Its bytes (a multiple of 16).
    pub size: u32,
    /// What it is, for the census ([`Slab::census`]): the family's own number for the kind.
    pub tag: u16,
    /// The family that made it.
    pub family: u8,
    /// K1: the guest block made in its place, to be freed with it (0: none).
    pub shadow: u32,
}

/// Bytes of freed slab blocks held back from reuse ([`Slab::free`]).
pub const SLAB_QUARANTINE: u64 = 4 << 20;

#[derive(Default)]
struct SlabAlloc {
    /// The first handle never handed out.
    top: u32,
    /// Freed blocks ready to be handed out again, by size.
    free: HashMap<u32, Vec<u32>>,
    /// Freed blocks held back, oldest first (handle, size), and their bytes.
    quarantine: VecDeque<(u32, u32)>,
    quarantine_bytes: u64,
    /// The live blocks.
    live: HashMap<u32, Block>,
    /// Live blocks, their bytes, and those of them with a K1 shadow, by (family, tag).
    census: BTreeMap<(u8, u16), (u64, u64, u64)>,
}

/// Host memory for moved objects, addressed by handles in [`SLAB_RANGE`] (module documentation).
pub struct Slab {
    words: Chunks<AtomicU32>,
    alloc: Mutex<SlabAlloc>,
}

impl Default for Slab {
    fn default() -> Self {
        Slab::new()
    }
}

impl Slab {
    pub fn new() -> Self {
        let bytes = index(SLAB_RANGE.end - SLAB_RANGE.start);
        Slab {
            words: Chunks::new(bytes / 4, SLAB_CHUNK_WORDS),
            alloc: Mutex::new(SlabAlloc {
                top: SLAB_RANGE.start,
                ..SlabAlloc::default()
            }),
        }
    }

    /// The word at handle `a`. Stops naming the address when it is no word of a chunk made.
    #[inline(always)]
    fn cell(&self, a: u32) -> &AtomicU32 {
        let off = a.wrapping_sub(SLAB_RANGE.start);
        match self.words.get(index(off / 4)) {
            Some(c) if a % 4 == 0 => c,
            _ => panic!("moved object: {a:08x} is no word the slab handed out"),
        }
    }

    /// A word (acquire).
    #[inline]
    pub fn load(&self, a: u32) -> u32 {
        self.cell(a).load(Ordering::Acquire)
    }

    /// Stores a word (release).
    #[inline]
    pub fn store(&self, a: u32, v: u32) {
        self.cell(a).store(v, Ordering::Release)
    }

    /// A word with the store-load order (sequentially consistent): what a thread publishing what
    /// it reads (a hazard slot) and reading back a link needs, which plain moves don't give.
    #[inline]
    pub fn load_seq(&self, a: u32) -> u32 {
        self.cell(a).load(Ordering::SeqCst)
    }

    /// Stores a word sequentially consistent ([`Slab::load_seq`]).
    #[inline]
    pub fn store_seq(&self, a: u32, v: u32) {
        self.cell(a).store(v, Ordering::SeqCst)
    }

    /// Compare-and-swap (sequentially consistent, as the interlocked import): whether the word held
    /// `expected` and now holds `new`.
    #[inline]
    pub fn cas(&self, a: u32, expected: u32, new: u32) -> bool {
        self.cell(a)
            .compare_exchange(expected, new, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Adds `v` (sequentially consistent, as the interlocked increment); answers the new value.
    #[inline]
    pub fn add(&self, a: u32, v: u32) -> u32 {
        self.cell(a).fetch_add(v, Ordering::SeqCst).wrapping_add(v)
    }

    /// A byte, read from its word.
    pub fn load_u8(&self, a: u32) -> u8 {
        byte(self.load(a & !3) >> ((a & 3) * 8))
    }

    /// Stores a byte into its word, the other three kept (one atomic change of the word).
    pub fn store_u8(&self, a: u32, v: u8) {
        let shift = (a & 3) * 8;
        let cell = self.cell(a & !3);
        let mut w = cell.load(Ordering::Acquire);
        loop {
            let new = w & !(0xff << shift) | u32::from(v) << shift;
            match cell.compare_exchange_weak(w, new, Ordering::Release, Ordering::Acquire) {
                Ok(_) => break,
                Err(now) => w = now,
            }
        }
    }

    /// A new block of `size` bytes (rounded up to 16), all zero, its handle 16-aligned; `None` when
    /// the range is full. `shadow`: K1's guest block made in its place, handed back by [`Slab::free`].
    pub fn alloc(&self, size: u32, family: FamilyId, tag: u16, shadow: u32) -> Option<u32> {
        let size = size.max(1).checked_add(15)? & !15;
        let mut a = lock(&self.alloc);
        let h = match a.free.get_mut(&size).and_then(Vec::pop) {
            Some(h) => {
                // Out of the quarantine: it still holds the freed words.
                for w in (h..h + size).step_by(4) {
                    self.store(w, 0);
                }
                h
            }
            None => {
                let h = a.top;
                let end = h.checked_add(size).filter(|&e| e <= SLAB_RANGE.end)?;
                let off = |x: u32| index((x - SLAB_RANGE.start) / 4);
                self.words.make(off(h)..off(end));
                a.top = end;
                h
            }
        };
        let block = Block {
            size,
            tag,
            family: family.number(),
            shadow,
        };
        a.live.insert(h, block);
        let row = a.census.entry((block.family, tag)).or_default();
        row.0 += 1;
        row.1 += u64::from(size);
        row.2 += u64::from(shadow != 0);
        Some(h)
    }

    /// Frees block `h` and answers what it was. Stops when `h` is no live block's start (freed
    /// twice, or never handed out).
    ///
    /// The block is not handed out again at once: it waits in a quarantine, oldest first, until
    /// more than [`SLAB_QUARANTINE`] bytes of later frees have joined it, every word the freed
    /// word of the guest heap's freed-block check ([`crate::mem::freed_word`] of the handle: above
    /// the heap, never mapped). So a stale handle reads the freed words and a pointer taken from
    /// them stops at its first read, rather than reading whatever object the slab handed out next.
    /// This, not the exe's reuse order, is what a moved family gets: the guest heap reproduces the
    /// exe's pools for the objects that stay there (a use after free in the game reads what the exe
    /// would have read), while a moved family's stale read stops and names itself.
    pub fn free(&self, h: u32) -> Block {
        let mut a = lock(&self.alloc);
        let Some(block) = a.live.remove(&h) else {
            panic!("moved object: {h:08x} freed, but it is no live block of the slab");
        };
        let freed = crate::mem::freed_word(h);
        for w in (h..h + block.size).step_by(4) {
            self.store(w, freed);
        }
        a.quarantine.push_back((h, block.size));
        a.quarantine_bytes += u64::from(block.size);
        while a.quarantine_bytes > SLAB_QUARANTINE {
            let (q, size) = a.quarantine.pop_front().expect("bytes wait");
            a.quarantine_bytes -= u64::from(size);
            a.free.entry(size).or_default().push(q);
        }
        if let Some(row) = a.census.get_mut(&(block.family, block.tag)) {
            row.0 -= 1;
            row.1 -= u64::from(block.size);
            row.2 -= u64::from(block.shadow != 0);
        }
        block
    }

    /// The live block that starts at `h`.
    pub fn block(&self, h: u32) -> Option<Block> {
        lock(&self.alloc).live.get(&h).copied()
    }

    /// The live blocks and their bytes by family number and tag, for the heap census.
    pub fn census(&self) -> Vec<(u8, u16, u64, u64)> {
        lock(&self.alloc)
            .census
            .iter()
            .filter(|(_, n)| n.0 != 0)
            .map(|(&(f, t), &(n, b, _))| (f, t, n, b))
            .collect()
    }

    /// Of [`Slab::census`]'s rows, the blocks that keep a K1 shadow in the heap, by family number
    /// and tag (rows with none left out).
    pub fn shadowed(&self) -> Vec<(u8, u16, u64)> {
        lock(&self.alloc)
            .census
            .iter()
            .filter(|(_, n)| n.2 != 0)
            .map(|(&(f, t), &(_, _, s))| (f, t, s))
            .collect()
    }
}

/// Stubs a chunk of the stub table holds.
const STUB_CHUNK: usize = 1 << 16;
/// The bytes of a guest page, which the stubs map one at a time.
const PAGE: u32 = 0x1000;

/// Freed stubs, and freed buffers' tokens, held back from reuse ([`Stubs::free`],
/// [`Buffers::free`]): a stale one then reads poison, or stops, rather than reaching the next
/// object handed the same address.
pub const ADDRESS_QUARANTINE: usize = 1 << 16;

#[derive(Default)]
struct StubAlloc {
    top: u32,
    /// Freed stubs, oldest first: handed out again only past [`ADDRESS_QUARANTINE`] of them.
    free: VecDeque<u32>,
}

/// 16-byte stubs in [`STUB_RANGE`] for moved objects that untyped code still reaches through their
/// vtable: each holds the object's vtable and then three words of poison, on a mapped guest page; a
/// table in host memory gives each stub's family and slab handle ([`Stubs::resolve`]).
pub struct Stubs {
    table: Chunks<AtomicU64>,
    alloc: Mutex<StubAlloc>,
}

impl Default for Stubs {
    fn default() -> Self {
        Stubs::new()
    }
}

impl Stubs {
    pub fn new() -> Self {
        let stubs = index((STUB_RANGE.end - STUB_RANGE.start) / STUB_BYTES);
        Stubs {
            table: Chunks::new(stubs, STUB_CHUNK),
            alloc: Mutex::new(StubAlloc {
                top: STUB_RANGE.start,
                free: VecDeque::new(),
            }),
        }
    }

    /// A stub for `family`'s object `handle`: [`vtable`, poison, poison, poison] in guest memory
    /// (its page mapped when new); `None` when the range is full.
    pub fn alloc(&self, mem: &mut Mem, family: FamilyId, vtable: u32, handle: u32) -> Option<u32> {
        let stub = {
            let mut a = lock(&self.alloc);
            let reuse = if a.free.len() > ADDRESS_QUARANTINE {
                a.free.pop_front()
            } else {
                None
            };
            match reuse {
                Some(s) => s,
                None => {
                    let s = a.top;
                    if s >= STUB_RANGE.end {
                        return None;
                    }
                    let i = Self::slot(s);
                    self.table.make(i..i + 1);
                    a.top = s + STUB_BYTES;
                    s
                }
            }
        };
        if !mem.is_mapped(stub) {
            mem.map(stub & !(PAGE - 1), PAGE);
        }
        mem.set_u32(stub, vtable);
        for off in [4, 8, 12] {
            mem.set_u32(stub + off, poison(family, off));
        }
        let entry = u64::from(family.number()) << 32 | u64::from(handle);
        self.table
            .get(Self::slot(stub))
            .expect("made")
            .store(entry, Ordering::Release);
        Some(stub)
    }

    /// The family number and slab handle of stub `stub`; `None` for an address that is no live stub.
    pub fn resolve(&self, stub: u32) -> Option<(u8, u32)> {
        if !is_stub(stub) || stub % STUB_BYTES != 0 {
            return None;
        }
        let entry = self.table.get(Self::slot(stub))?.load(Ordering::Acquire);
        let handle = u32::try_from(entry & 0xffff_ffff).expect("low word");
        (handle != 0).then(|| (byte(u32::try_from(entry >> 32).expect("high word")), handle))
    }

    /// Frees `family`'s stub `stub`: all four words poison (a stale holder's virtual call then
    /// stops), the table entry cleared, the stub held back from reuse ([`ADDRESS_QUARANTINE`]).
    /// Stops when `stub` is no live stub (freed twice, or never handed out) or another family's.
    ///
    /// Stubs lie packed, 16 bytes apart: an untyped read at +0x10 or past it lands on the next
    /// stub without stopping. A family gets stubs only where the touch census shows its untyped
    /// readers read the vtable alone.
    pub fn free(&self, mem: &Mem, family: FamilyId, stub: u32) {
        self.claim(family, stub);
        for off in [0, 4, 8, 12] {
            mem.set_u32(stub + off, poison(family, off));
        }
        lock(&self.alloc).free.push_back(stub);
    }

    /// [`Stubs::free`]'s check and table change: the live stub's entry cleared, by one thread
    /// only (of two freeing it at once, the other stops).
    fn claim(&self, family: FamilyId, stub: u32) {
        let entry = (is_stub(stub) && stub % STUB_BYTES == 0)
            .then(|| self.table.get(Self::slot(stub)))
            .flatten();
        let Some(entry) = entry else {
            panic!("stub {stub:08x} freed, but it was never handed out");
        };
        let was = entry.load(Ordering::Acquire);
        if was == 0 {
            panic!("stub {stub:08x} freed, but it is no live stub (freed twice?)");
        }
        let owner = was >> 32;
        if owner != u64::from(family.number()) {
            panic!(
                "stub {stub:08x} of family {owner} freed by family {}",
                family.number()
            );
        }
        if entry
            .compare_exchange(was, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            panic!("stub {stub:08x} freed by two threads at once");
        }
    }

    fn slot(stub: u32) -> usize {
        index((stub - STUB_RANGE.start) / STUB_BYTES)
    }
}

/// Tokens a chunk of the buffers' table holds.
const TOKEN_CHUNK: usize = 1 << 14;

/// A buffer's host bytes as `Box::into_raw` left them, with their length: one load of the table's
/// pointer gives both (a free and a new buffer under the same token between two loads cannot pair
/// one buffer's bytes with the other's length). Neither field changes after `new`.
struct HostBytes {
    ptr: *mut u8,
    len: u32,
}

// SAFETY: a `HostBytes` owns its bytes alone (a leaked box, dropped once, by its `Drop`); other
// threads reach the bytes only through the buffers' own accesses, as module documentation says.
unsafe impl Send for HostBytes {}

impl HostBytes {
    fn new(len: u32) -> HostBytes {
        let raw = Box::into_raw(vec![0u8; index(len)].into_boxed_slice());
        HostBytes {
            ptr: raw.cast::<u8>(),
            len,
        }
    }
}

impl Drop for HostBytes {
    fn drop(&mut self) {
        let raw = std::ptr::slice_from_raw_parts_mut(self.ptr, index(self.len));
        // SAFETY: `new` made it from a boxed slice of `len` bytes; dropped once, here.
        drop(unsafe { Box::from_raw(raw) });
    }
}

/// One buffer's place in the table: its bytes (null: no buffer) and its family and tag for the
/// census.
#[derive(Default)]
struct BufferEntry {
    bytes: AtomicPtr<HostBytes>,
    owner: AtomicU32,
}

#[derive(Default)]
struct TokenAlloc {
    top: u32,
    /// Freed tokens, oldest first: handed out again only past [`ADDRESS_QUARANTINE`] of them.
    free: VecDeque<u32>,
    /// Live buffers and bytes by (family, tag).
    census: BTreeMap<(u8, u16), (u64, u64)>,
    /// Freed buffers' bytes not yet handed back to the host ([`Buffers::reclaim`]), and their
    /// count of bytes. Kept boxed: the table pointed at each box, and an access that loaded that
    /// pointer before the free may still read through it until `reclaim`.
    #[allow(clippy::vec_box)]
    retired: Vec<Box<HostBytes>>,
    retired_bytes: u64,
}

/// Bytes of freed buffers that make real threads hand them back at a barrier
/// (`real_threads`, at a frame's end).
pub const RECLAIM_AT: u64 = 64 << 20;

/// Big arrays moved out of the game's heap ([module documentation](self)): a buffer of host bytes
/// behind a start-only token in [`TOKEN_RANGE`]. A typed function reads and writes it by token and
/// offset ([`Buffers::read`], [`Buffers::write`]); no 32-bit address of a place inside it exists.
pub struct Buffers {
    table: Chunks<BufferEntry>,
    alloc: Mutex<TokenAlloc>,
}

impl Default for Buffers {
    fn default() -> Self {
        Buffers::new()
    }
}

impl Buffers {
    pub fn new() -> Self {
        let tokens = index((TOKEN_RANGE.end - TOKEN_RANGE.start) / STUB_BYTES);
        Buffers {
            table: Chunks::new(tokens, TOKEN_CHUNK),
            alloc: Mutex::new(TokenAlloc {
                top: TOKEN_RANGE.start,
                ..TokenAlloc::default()
            }),
        }
    }

    fn slot(token: u32) -> usize {
        index((token - TOKEN_RANGE.start) / STUB_BYTES)
    }

    /// The table entry of `token`, if it is a token handed out at some time.
    fn table_entry(&self, token: u32) -> Option<&BufferEntry> {
        (is_token(token) && token % STUB_BYTES == 0)
            .then(|| self.table.get(Self::slot(token)))
            .flatten()
    }

    /// The bytes of the live buffer `token` (pointer and length from one load); stops naming the
    /// token when it is none. The bytes stay valid until [`Buffers::reclaim`], which runs only
    /// while no game thread is inside a buffer access: so a read racing a free reads the freed
    /// bytes (the game's own use after free, as with a guest block), never host memory handed
    /// back.
    fn live(&self, token: u32) -> (*mut u8, u32) {
        if let Some(entry) = self.table_entry(token) {
            let host = entry.bytes.load(Ordering::Acquire);
            if !host.is_null() {
                // SAFETY: a non-null pointer in the table is a `HostBytes` that `alloc` leaked;
                // `free` takes it out of the table and keeps it until `reclaim`, which no access
                // overlaps (above). Its fields never change.
                let h = unsafe { &*host };
                return (h.ptr, h.len);
            }
        }
        panic!("moved array: {token:08x} is no live buffer's token");
    }

    /// A new buffer of `len` bytes, all zero, for `family` (tagged `tag`); `None` when the tokens
    /// run out.
    pub fn alloc(&self, len: u32, family: FamilyId, tag: u16) -> Option<u32> {
        let token = {
            let mut a = lock(&self.alloc);
            let reuse = if a.free.len() > ADDRESS_QUARANTINE {
                a.free.pop_front()
            } else {
                None
            };
            let token = match reuse {
                Some(t) => t,
                None => {
                    let t = a.top;
                    if t >= TOKEN_RANGE.end {
                        return None;
                    }
                    let i = Self::slot(t);
                    self.table.make(i..i + 1);
                    a.top = t + STUB_BYTES;
                    t
                }
            };
            let row = a.census.entry((family.number(), tag)).or_default();
            row.0 += 1;
            row.1 += u64::from(len);
            token
        };
        let host = Box::new(HostBytes::new(len));
        let entry = self.table.get(Self::slot(token)).expect("made");
        entry.owner.store(
            u32::from(family.number()) << 16 | u32::from(tag),
            Ordering::Release,
        );
        entry.bytes.store(Box::into_raw(host), Ordering::Release);
        Some(token)
    }

    /// Frees buffer `token` and answers its length. Its bytes leave the table at once (one swap:
    /// of two frees racing, one finds the buffer and the other stops, as a second free does), and
    /// go back to the host at [`Buffers::reclaim`]; the token waits in a quarantine
    /// ([`ADDRESS_QUARANTINE`]).
    pub fn free(&self, token: u32) -> u32 {
        let Some(entry) = self.table_entry(token) else {
            panic!("moved array: {token:08x} freed, but it is no buffer's token");
        };
        let taken = entry.bytes.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if taken.is_null() {
            panic!("moved array: {token:08x} freed, but it is no live buffer (freed twice?)");
        }
        // SAFETY: `alloc` leaked it with `Box::into_raw`; the swap took it out of the table, so
        // this is its only owner (a racing access may still read it: it stays allocated until
        // `reclaim`).
        let host = unsafe { Box::from_raw(taken) };
        let len = host.len;
        let owner = entry.owner.load(Ordering::Acquire);
        let mut a = lock(&self.alloc);
        let key = (
            byte(owner >> 16),
            u16::try_from(owner & 0xffff).expect("a tag"),
        );
        if let Some(row) = a.census.get_mut(&key) {
            row.0 -= 1;
            row.1 -= u64::from(len);
        }
        a.retired_bytes += u64::from(len);
        a.retired.push(host);
        a.free.push_back(token);
        len
    }

    /// The bytes of freed buffers waiting for [`Buffers::reclaim`].
    pub fn retired_bytes(&self) -> u64 {
        lock(&self.alloc).retired_bytes
    }

    /// Hands the freed buffers' bytes back to the host. Only while no game thread can be inside a
    /// buffer access: at the thread host's barrier with real threads, between frames with stepped
    /// ones (`real_threads`, `thread_host`). Answers the bytes handed back.
    pub fn reclaim(&self) -> u64 {
        let retired = {
            let mut a = lock(&self.alloc);
            a.retired_bytes = 0;
            std::mem::take(&mut a.retired)
        };
        retired.iter().map(|h| u64::from(h.len)).sum()
    }

    /// The length of buffer `token`, `None` when it is no live buffer.
    pub fn len(&self, token: u32) -> Option<u32> {
        let host = self.table_entry(token)?.bytes.load(Ordering::Acquire);
        // SAFETY: as in `live`.
        (!host.is_null()).then(|| unsafe { (*host).len })
    }

    /// Copies `into.len()` bytes of buffer `token` from `offset`; stops past its end.
    pub fn read(&self, token: u32, offset: u32, into: &mut [u8]) {
        let (bytes, len) = self.live(token);
        let end = u64::from(offset) + into.len() as u64;
        assert!(
            end <= u64::from(len),
            "moved array {token:08x}: bytes {offset}..{end} of {len}"
        );
        // SAFETY: inside the live buffer (checked above); plain bytes, ordered by the game's own
        // synchronisation as guest memory's plain accesses are.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.add(index(offset)), into.as_mut_ptr(), into.len())
        }
    }

    /// Copies `from` into buffer `token` at `offset`; stops past its end.
    pub fn write(&self, token: u32, offset: u32, from: &[u8]) {
        let (bytes, len) = self.live(token);
        let end = u64::from(offset) + from.len() as u64;
        assert!(
            end <= u64::from(len),
            "moved array {token:08x}: bytes {offset}..{end} of {len}"
        );
        // SAFETY: as in `read`.
        unsafe {
            std::ptr::copy_nonoverlapping(from.as_ptr(), bytes.add(index(offset)), from.len())
        }
    }

    /// The live buffers and their bytes by family number and tag, for the heap census.
    pub fn census(&self) -> Vec<(u8, u16, u64, u64)> {
        lock(&self.alloc)
            .census
            .iter()
            .filter(|(_, n)| n.0 != 0)
            .map(|(&(f, t), &(n, b))| (f, t, n, b))
            .collect()
    }
}

impl Drop for Buffers {
    fn drop(&mut self) {
        for i in 0..index((TOKEN_RANGE.end - TOKEN_RANGE.start) / STUB_BYTES) {
            let Some(entry) = self.table.get(i) else {
                continue;
            };
            let host = entry.bytes.swap(std::ptr::null_mut(), Ordering::AcqRel);
            if !host.is_null() {
                // SAFETY: a live buffer `alloc` leaked; nothing uses it after the drop. (The
                // freed ones not yet reclaimed go with the table's lock.)
                drop(unsafe { Box::from_raw(host) });
            }
        }
    }
}

/// The first differences the dual run prints (the rest are counted).
pub const DUAL_PRINT: u32 = 20;

/// The line the dual run prints for a difference ([`Dual::check`]).
pub fn difference_line(
    family: FamilyId,
    at: u32,
    guest: u32,
    seen: u32,
    running: Option<u32>,
) -> String {
    let running = running.map_or(String::new(), |r| format!(" in {r:08x}"));
    format!(
        "dual run: family {} differs at {at:08x}{running}: the guest form holds {guest:08x} (as the \
         mirror would), the moved form {seen:08x}",
        family.number()
    )
}
/// Families' numbers are below 32 ([`FamilyId::new`]).
const FAMILIES: usize = 32;

/// The dual run's state ([module documentation](self)).
pub struct Dual {
    /// Guest block start -> (bytes, its mirror's handle).
    mirror: Mutex<BTreeMap<u32, (u32, u32)>>,
    compared: [AtomicU64; FAMILIES],
    differed: [AtomicU64; FAMILIES],
    printed: AtomicU32,
}

impl Default for Dual {
    fn default() -> Self {
        Dual {
            mirror: Mutex::new(BTreeMap::new()),
            compared: std::array::from_fn(|_| AtomicU64::new(0)),
            differed: std::array::from_fn(|_| AtomicU64::new(0)),
            printed: AtomicU32::new(0),
        }
    }
}

impl Dual {
    /// Guest block `[guest, guest + len)` is mirrored by the slab block `handle`. Mirrored ranges
    /// never overlap (a guest block is mirrored whole, once): [`Dual::mirror_of`] finds a place by
    /// the nearest start below it.
    pub fn add(&self, guest: u32, len: u32, handle: u32) {
        let mut m = lock(&self.mirror);
        let end = guest.saturating_add(len);
        debug_assert!(
            m.range(..end)
                .next_back()
                .map_or(true, |(&s, &(l, _))| s.saturating_add(l) <= guest),
            "dual run: guest block {guest:08x}+{len:x} overlaps one already mirrored"
        );
        m.insert(guest, (len, handle));
    }

    /// Guest block `guest` is gone; answers its mirror's handle.
    pub fn remove(&self, guest: u32) -> Option<u32> {
        lock(&self.mirror).remove(&guest).map(|(_, h)| h)
    }

    /// The place in the slab that mirrors guest address `a` (inside a mirrored block).
    pub fn mirror_of(&self, a: u32) -> Option<u32> {
        let m = lock(&self.mirror);
        let (&start, &(len, handle)) = m.range(..=a).next_back()?;
        (a - start < len).then(|| handle + (a - start))
    }

    /// `w` as the mirror holds it: a word inside a mirrored guest block is the same place in its
    /// mirror; any other word stays.
    pub fn translate(&self, w: u32) -> u32 {
        self.mirror_of(w).unwrap_or(w)
    }

    /// Counts one comparison of `family`: `guest` (translated) against what the slab holds, `seen`.
    /// A difference is counted, and printed while fewer than [`DUAL_PRINT`] have been, with `at`
    /// (the guest address compared) and the function running. Answers whether they agree.
    pub fn check(
        &self,
        family: FamilyId,
        at: u32,
        guest: u32,
        seen: u32,
        running: Option<u32>,
    ) -> bool {
        let f = usize::from(family.number());
        self.compared[f].fetch_add(1, Ordering::Relaxed);
        if guest == seen {
            return true;
        }
        self.differed[f].fetch_add(1, Ordering::Relaxed);
        if self.printed.fetch_add(1, Ordering::Relaxed) < DUAL_PRINT {
            eprintln!("{}", difference_line(family, at, guest, seen, running));
        }
        false
    }

    /// Counts a comparison that is not of a word (a compare-and-swap's outcome).
    pub fn check_bool(
        &self,
        family: FamilyId,
        at: u32,
        guest: bool,
        seen: bool,
        running: Option<u32>,
    ) -> bool {
        self.check(family, at, u32::from(guest), u32::from(seen), running)
    }

    /// `family`'s comparisons and differences so far.
    pub fn counts(&self, family: FamilyId) -> (u64, u64) {
        let f = usize::from(family.number());
        (
            self.compared[f].load(Ordering::Relaxed),
            self.differed[f].load(Ordering::Relaxed),
        )
    }
}

/// What an engine keeps for its moved families: the slab, the stubs, the dual run. Made by
/// `typed::apply` when a family is on in a stage-2 form, before any game thread starts, and shared
/// by every real thread's engine (leaked: it lives as long as the guest memory it serves).
#[derive(Default)]
pub struct Moved {
    pub slab: Slab,
    pub buffers: Buffers,
    pub stubs: Stubs,
    pub dual: Dual,
}

impl Moved {
    /// A new one, for the life of the process.
    pub fn leak() -> &'static Moved {
        Box::leak(Box::default())
    }
}

/// Where a family's typed code reads and writes its own objects ([module documentation](self)).
/// One code path for every form: each access names a guest address or a handle, and the store
/// knows which is which.
#[derive(Clone, Copy)]
pub enum Store {
    /// Stage 1: every object in guest memory.
    Guest,
    /// Guest objects the truth; each block the family allocates mirrored in the slab, every access
    /// made on both and compared. Game threads stay stepped meanwhile and the render helpers do not
    /// fork (`real_threads`), so one access at a time changes the guest word and then its mirror.
    /// For the same reason the `by_address` call a store method makes in this form (the family's
    /// compare-and-swap, interlocked add, allocator, copy) must never reach a wait: a thread switch
    /// between the guest word and its mirror would let another thread's access in between, and
    /// the two would then differ in order alone.
    Dual {
        moved: &'static Moved,
        family: FamilyId,
    },
    /// The family's new objects in the slab, the truth. `shadows` (K1): each allocation still makes
    /// its guest block, filled with poison and freed with it.
    Moved {
        moved: &'static Moved,
        family: FamilyId,
        shadows: bool,
    },
}

/// A word at or above [`POISON_BASE`] read through the moved store ([`Store::load`]) at the guest
/// address `a`: stops the run when it is `family`'s own poison, naming the field. (Any other such
/// word, -1 say, is a value.)
#[cold]
#[inline(never)]
fn poison_read(family: FamilyId, a: u32, w: u32) {
    if let Some((f, off)) = poison_parts(w) {
        if f == family.number() {
            panic!(
                "moved object: family {f} read {a:08x} through its store and found the poison of \
                 its moved field +{off:x}: typed code read a K1 stub's moved field without K1::place"
            );
        }
    }
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Store::Guest => write!(f, "Guest"),
            Store::Dual { family, .. } => write!(f, "Dual({})", family.number()),
            Store::Moved {
                family, shadows, ..
            } => write!(f, "Moved({}, shadows {shadows})", family.number()),
        }
    }
}

impl Store {
    /// Whether this is stage 1's store.
    #[inline(always)]
    pub fn is_guest(self) -> bool {
        matches!(self, Store::Guest)
    }

    /// The word at `a`: a guest address or a handle. In the moved form a guest word that is the
    /// family's own poison stops the run: typed code read a moved field of a K1 stub without
    /// [`K1::place`], and would have taken the poison for the value.
    #[inline]
    pub fn load(self, e: &Engine, a: u32) -> u32 {
        match self {
            Store::Guest => e.mem.u32(a),
            Store::Moved { moved, family, .. } => {
                if is_handle(a) {
                    moved.slab.load(a)
                } else {
                    let w = e.mem.u32(a);
                    if w >= POISON_BASE {
                        poison_read(family, a, w);
                    }
                    w
                }
            }
            Store::Dual { moved, family } => {
                let g = e.mem.u32(a);
                if let Some(m) = moved.dual.mirror_of(a) {
                    let seen = moved.slab.load(m);
                    let running = e.call_stack.last().copied();
                    moved
                        .dual
                        .check(family, a, moved.dual.translate(g), seen, running);
                }
                g
            }
        }
    }

    /// Stores `v` at `a`.
    #[inline]
    pub fn store(self, e: &Engine, a: u32, v: u32) {
        match self {
            Store::Guest => e.mem.set_u32(a, v),
            Store::Moved { moved, .. } => {
                if is_handle(a) {
                    moved.slab.store(a, v)
                } else {
                    e.mem.set_u32(a, v)
                }
            }
            Store::Dual { moved, .. } => {
                e.mem.set_u32(a, v);
                if let Some(m) = moved.dual.mirror_of(a) {
                    moved.slab.store(m, moved.dual.translate(v));
                }
            }
        }
    }

    /// [`Store::load`] with the store-load order where the word is moved ([`Slab::load_seq`]); a
    /// guest word is a plain load, as the exe's.
    #[inline]
    pub fn load_seq(self, e: &Engine, a: u32) -> u32 {
        match self {
            Store::Moved { moved, .. } if is_handle(a) => moved.slab.load_seq(a),
            _ => self.load(e, a),
        }
    }

    /// [`Store::store`] with the store-load order where the word is moved.
    #[inline]
    pub fn store_seq(self, e: &Engine, a: u32, v: u32) {
        match self {
            Store::Moved { moved, .. } if is_handle(a) => moved.slab.store_seq(a, v),
            _ => self.store(e, a, v),
        }
    }

    /// Field `f` of the object at `p`.
    #[inline]
    pub fn get<S, T: Word>(self, e: &Engine, p: Ptr<S>, f: Field<S, T>) -> T {
        T::from_word(self.load(e, p.addr().wrapping_add(f.off)))
    }

    /// Sets field `f` of the object at `p`.
    #[inline]
    pub fn set<S, T: Word>(self, e: &Engine, p: Ptr<S>, f: Field<S, T>, v: T) {
        self.store(e, p.addr().wrapping_add(f.off), v.to_word())
    }

    /// A compare-and-swap of the word at `a` from `expected` to `new`. A guest word goes through
    /// `by_address` (the family's call by address that does it, as in stage 1); a moved word is a
    /// slab compare-and-swap; the dual run does both and compares the outcomes.
    pub fn cas(
        self,
        e: &mut Engine,
        a: u32,
        expected: u32,
        new: u32,
        by_address: impl FnOnce(&mut Engine) -> bool,
    ) -> bool {
        match self {
            Store::Guest => by_address(e),
            Store::Moved { moved, .. } => {
                if is_handle(a) {
                    moved.slab.cas(a, expected, new)
                } else {
                    by_address(e)
                }
            }
            Store::Dual { moved, family } => {
                let done = by_address(e);
                if let Some(m) = moved.dual.mirror_of(a) {
                    let d = &moved.dual;
                    let seen = moved.slab.cas(m, d.translate(expected), d.translate(new));
                    d.check_bool(family, a, done, seen, e.call_stack.last().copied());
                }
                done
            }
        }
    }

    /// An interlocked add of `v` to the word at `a`, answering the new value, as the game's
    /// interlocked increment (`v` 1) and decrement (`v` -1 as a word) do. A guest word goes through
    /// `by_address` (the family's call by address, as in stage 1); a moved word is a slab atomic; the
    /// dual run does both and compares the results.
    pub fn add(
        self,
        e: &mut Engine,
        a: u32,
        v: u32,
        by_address: impl FnOnce(&mut Engine) -> u32,
    ) -> u32 {
        match self {
            Store::Guest => by_address(e),
            Store::Moved { moved, .. } => {
                if is_handle(a) {
                    moved.slab.add(a, v)
                } else {
                    by_address(e)
                }
            }
            Store::Dual { moved, family } => {
                let done = by_address(e);
                if let Some(m) = moved.dual.mirror_of(a) {
                    let seen = moved.slab.add(m, v);
                    moved
                        .dual
                        .check(family, a, done, seen, e.call_stack.last().copied());
                }
                done
            }
        }
    }

    /// The dual run after a call by address that wrote `words` words of the family's own object at
    /// `a` (one the family has not typed yet, whose result the mirror must take): the mirror takes
    /// the guest words (translated). Nothing in the other forms, where such a call is done in place.
    pub fn resync(self, e: &Engine, a: u32, words: u32) {
        if let Store::Dual { moved, .. } = self {
            for i in 0..words {
                let at = a.wrapping_add(4 * i);
                if let Some(m) = moved.dual.mirror_of(at) {
                    moved.slab.store(m, moved.dual.translate(e.mem.u32(at)));
                }
            }
        }
    }

    /// Copies `words` words from `from` to `to`. Guest to guest goes through `by_address` (the
    /// family's call by address, as in stage 1); the dual run then has each mirrored word of the
    /// destination take the guest word, word by word as [`Store::resync`] does (the source need not
    /// be mirrored, and the copy may cross from one mirrored block into the next); moved words are
    /// copied in place.
    pub fn copy(
        self,
        e: &mut Engine,
        to: u32,
        from: u32,
        words: u32,
        by_address: impl FnOnce(&mut Engine),
    ) {
        match self {
            Store::Guest => by_address(e),
            Store::Moved { .. } if !is_handle(to) && !is_handle(from) => by_address(e),
            Store::Moved { .. } => {
                for i in 0..words {
                    let w = self.load(e, from + 4 * i);
                    self.store(e, to + 4 * i, w);
                }
            }
            Store::Dual { .. } => {
                by_address(e);
                self.resync(e, to, words);
            }
        }
    }

    /// A new object of the family, `size` bytes, made where the store keeps them. `by_address` makes
    /// it as stage 1 does (its call by address of the allocator, answering the block or 0): the
    /// guest store and the dual run use that block (the dual run mirrors it); the moved store makes
    /// a slab block, and with shadows calls `by_address` too, for the same guest heap, and fills
    /// its block with poison. Answers 0 when the allocation fails, as the guest one does.
    pub fn alloc(
        self,
        e: &mut Engine,
        size: u32,
        tag: u16,
        by_address: impl FnOnce(&mut Engine) -> u32,
    ) -> u32 {
        match self {
            Store::Guest => by_address(e),
            Store::Dual { moved, family } => {
                let g = by_address(e);
                if g != 0 {
                    let h = moved
                        .slab
                        .alloc(size, family, tag, 0)
                        .expect("the slab range is full");
                    for off in (0..size & !3).step_by(4) {
                        let w = moved.dual.translate(e.mem.u32(g + off));
                        moved.slab.store(h + off, w);
                    }
                    moved.dual.add(g, size, h);
                }
                g
            }
            Store::Moved {
                moved,
                family,
                shadows,
            } => {
                let shadow = if shadows {
                    let s = by_address(e);
                    if s == 0 {
                        return 0;
                    }
                    for off in (0..size & !3).step_by(4) {
                        e.mem.set_u32(s + off, poison(family, off));
                    }
                    s
                } else {
                    0
                };
                moved
                    .slab
                    .alloc(size, family, tag, shadow)
                    .expect("the slab range is full")
            }
        }
    }

    /// Frees an object [`Store::alloc`] made (or a guest block: stage 1's objects, and the shadow of
    /// a moved one), through `by_address` (the family's call by address of the deallocator).
    pub fn free(self, e: &mut Engine, a: u32, by_address: impl FnOnce(&mut Engine, u32)) {
        match self {
            Store::Guest => by_address(e, a),
            Store::Moved { moved, .. } => {
                if is_handle(a) {
                    let block = moved.slab.free(a);
                    if block.shadow != 0 {
                        by_address(e, block.shadow);
                    }
                } else {
                    by_address(e, a)
                }
            }
            Store::Dual { moved, .. } => {
                if let Some(h) = moved.dual.remove(a) {
                    moved.slab.free(h);
                }
                by_address(e, a)
            }
        }
    }
}

/// A class's K1 stub ([module documentation](self)): its original guest block keeps every field
/// untyped code reads; `spare` is the offset of a word nothing reads, which holds the slab handle
/// of the object's moved record (the class's layout, `size` bytes, in the slab), and `moved` the
/// offsets of the fields that live there, poison in the guest block. The record is new data, so it
/// makes no guest allocation in any form: the guest heap stays as stage 1 leaves it.
pub struct K1 {
    pub size: u32,
    pub spare: u32,
    pub moved: &'static [u32],
}

impl K1 {
    /// The moved record of the stub at `obj`, if it has one.
    #[inline]
    pub fn record(&self, e: &Engine, obj: u32) -> Option<u32> {
        let h = e.mem.u32(obj.wrapping_add(self.spare));
        is_handle(h).then_some(h)
    }

    /// Gives the object at `obj` its record (tagged `tag`), first thing in its constructor. The
    /// moved form: the record's handle in the spare word and poison in the moved fields, whose
    /// values from now on are the record's. The dual run: the record mirrors each moved field (the
    /// guest fields stay the truth, compared at every access) and the spare word is left alone.
    /// Stage 1: nothing. Answers the record.
    pub fn attach(&self, e: &Engine, s: Store, obj: u32, tag: u16) -> Option<u32> {
        let (moved, family) = match s {
            Store::Guest => return None,
            Store::Dual { moved, family } | Store::Moved { moved, family, .. } => (moved, family),
        };
        let record = moved
            .slab
            .alloc(self.size, family, tag, 0)
            .expect("the slab range is full");
        for &off in self.moved {
            let at = obj.wrapping_add(off);
            if let Store::Dual { .. } = s {
                moved
                    .slab
                    .store(record + off, moved.dual.translate(e.mem.u32(at)));
                moved.dual.add(at, 4, record + off);
            } else {
                e.mem.set_u32(at, poison(family, off));
            }
        }
        if let Store::Moved { .. } = s {
            e.mem.set_u32(obj.wrapping_add(self.spare), record);
        }
        Some(record)
    }

    /// Takes the record away again, last thing in the object's destructor (and, in the dual run,
    /// its mirrors, which name the record there). In the moved form the poison is checked first
    /// ([`K1::poison_intact`]: an untyped writer of a moved field stops the run here, naming the
    /// field and what it wrote), and the spare word is cleared, so a second destruction finds no
    /// record and frees nothing (a stale handle there would free whatever block the slab handed
    /// out since).
    pub fn detach(&self, e: &Engine, s: Store, obj: u32) {
        let (moved, record) = match s {
            Store::Guest => return,
            Store::Moved { moved, family, .. } => {
                let record = self.record(e, obj);
                if record.is_some() {
                    if let Err((off, w)) = self.poison_intact(e, family, obj) {
                        panic!(
                            "moved object: family {}'s object {obj:08x} is destroyed with \
                             {w:08x} in its moved field +{off:x}, not the poison: untyped code \
                             wrote it",
                            family.number()
                        );
                    }
                }
                e.mem.set_u32(obj.wrapping_add(self.spare), 0);
                (moved, record)
            }
            Store::Dual { moved, .. } => {
                let record = self.moved.first().and_then(|&off| {
                    let m = moved.dual.mirror_of(obj.wrapping_add(off))?;
                    Some(m - off)
                });
                for &off in self.moved {
                    moved.dual.remove(obj.wrapping_add(off));
                }
                (moved, record)
            }
        };
        if let Some(r) = record {
            moved.slab.free(r);
        }
    }

    /// Where field `off` of the object at `obj` is read and written: in the moved form a moved
    /// field is the same offset in the record; everything else (and every field in stage 1 and the
    /// dual run, where the store compares the moved ones with the record) is `obj + off`.
    #[inline]
    pub fn place(&self, e: &Engine, s: Store, obj: u32, off: u32) -> u32 {
        if let Store::Moved { .. } = s {
            if self.moved.contains(&off) {
                if let Some(r) = self.record(e, obj) {
                    return r.wrapping_add(off);
                }
            }
        }
        obj.wrapping_add(off)
    }

    /// Whether every moved field of the stub at `obj` still holds its poison; else the first offset
    /// that does not and what it holds (a missed writer).
    pub fn poison_intact(&self, e: &Engine, family: FamilyId, obj: u32) -> Result<(), (u32, u32)> {
        for &off in self.moved {
            let w = e.mem.u32(obj.wrapping_add(off));
            if w != poison(family, off) {
                return Err((off, w));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! The machinery with a made-up family ("toy"): a guest object (vtable, a count untyped code
    //! reads, the head of a list of nodes, a spare word) whose list moves; the toy's own functions
    //! push and sum through one store; untyped stand-ins read and write where they should not.
    use super::*;
    use crate::abi::Ret;
    use crate::typed::{apply_families, Family, Threads};

    const TOY: FamilyId = FamilyId::new(27);
    static TOY_FAMILY: Family = Family {
        name: "toy",
        id: TOY,
        past_gate: false,
        threads: Threads::Both,
        stage2: true,
        funcs: toy_funcs,
    };
    /// The toy object: +0 vtable, +4 count (stays: untyped code reads it), +8 head (moves),
    /// +0xc spare. A node: +0 value, +4 next.
    const TOY_SIZE: u32 = 0x10;
    const COUNT: u32 = 4;
    const HEAD: u32 = 8;
    const STUB: K1 = K1 {
        size: TOY_SIZE,
        spare: 0xc,
        moved: &[HEAD],
    };
    const NODE_TAG: u16 = 1;
    const RECORD_TAG: u16 = 2;
    /// The stand-ins' addresses: the allocator and deallocator the toy calls by address, the
    /// toy's typed entries, an untyped reader of the head, an untyped writer of a node.
    const NEW: u32 = 0x0040_1000;
    const DELETE: u32 = 0x0040_1030;
    const SUM: u32 = 0x0040_5000;
    const READS_HEAD: u32 = 0x0040_6000;
    const WRITES_NODE: u32 = 0x0040_7000;
    const VTABLE: u32 = 0x0010_0000;
    const VSLOT: u32 = 0x0040_8000;

    fn toy_funcs() -> Vec<(u32, crate::AbiFn)> {
        vec![crate::entry!(0x0040_5000, toy_sum_entry(u32) -> u32)]
    }

    /// The toy's store for an operation on `toy` (the root every operation has): the family's
    /// form, which a stub made before it took (none here) would not have.
    fn store_of(e: &Engine, _toy: u32) -> Store {
        e.typed_store(TOY)
    }

    fn engine(mode: &str) -> Engine {
        let mut e = Engine::new_untyped();
        e.map(0x0010_0000, 0x1000);
        e.mem.set_u32(VTABLE, VSLOT);
        e.register(NEW, |e, a| Ret {
            eax: e.mem.alloc(a[0]),
            ..Ret::default()
        });
        e.register(DELETE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(VSLOT, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        e.register(READS_HEAD, |e, a| {
            let head = e.mem.u32(a[0] + HEAD);
            Ret {
                eax: if head == 0 { 0 } else { e.mem.u32(head) },
                ..Ret::default()
            }
        });
        e.register(WRITES_NODE, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        apply_families(&mut e, &[&TOY_FAMILY], Some(mode));
        e
    }

    /// The toy's constructor: made by untyped code (a guest block), then its record attached and
    /// its fields set through the store.
    fn toy_new(e: &mut Engine) -> u32 {
        let toy = e.call(NEW, &[TOY_SIZE]).u32();
        let s = store_of(e, toy);
        STUB.attach(e, s, toy, RECORD_TAG);
        e.mem.set_u32(toy, VTABLE);
        s.store(e, STUB.place(e, s, toy, HEAD), 0);
        toy
    }

    fn head(e: &Engine, s: Store, toy: u32) -> u32 {
        s.load(e, STUB.place(e, s, toy, HEAD))
    }

    fn toy_push(e: &mut Engine, toy: u32, value: u32) {
        let s = store_of(e, toy);
        let node = s.alloc(e, 8, NODE_TAG, |e| e.call(NEW, &[8]).u32());
        s.store(e, node, value);
        loop {
            let old = head(e, s, toy);
            s.store(e, node + 4, old);
            let at = STUB.place(e, s, toy, HEAD);
            // Stage 1 swaps the head by address (here: a plain store standing in for the call).
            if s.cas(e, at, old, node, |e| {
                let ok = e.mem.u32(at) == old;
                if ok {
                    e.mem.set_u32(at, node);
                }
                ok
            }) {
                break;
            }
        }
        let count = e.mem.u32(toy + COUNT);
        e.mem.set_u32(toy + COUNT, count + 1);
    }

    fn toy_sum(e: &mut Engine, toy: u32) -> u32 {
        let s = store_of(e, toy);
        let mut node = head(e, s, toy);
        let mut sum = 0u32;
        while node != 0 {
            sum = sum.wrapping_add(s.load(e, node));
            node = s.load(e, node + 4);
        }
        sum
    }

    fn toy_sum_entry(e: &mut Engine, toy: u32) -> u32 {
        toy_sum(e, toy)
    }

    fn toy_pop_all(e: &mut Engine, toy: u32) {
        let s = store_of(e, toy);
        let mut node = head(e, s, toy);
        while node != 0 {
            let next = s.load(e, node + 4);
            s.free(e, node, |e, b| {
                e.call(DELETE, &[b]);
            });
            node = next;
        }
        s.store(e, STUB.place(e, s, toy, HEAD), 0);
        e.mem.set_u32(toy + COUNT, 0);
    }

    /// What one engine's run of the toy shows: the sums by Rust call and by address, the count
    /// untyped code reads, the guest heap's top after.
    fn run(e: &mut Engine) -> (u32, u32, u32, u32) {
        let toy = toy_new(e);
        for v in [3, 5, 7] {
            toy_push(e, toy, v);
        }
        let by_address = e.call(SUM, &[toy]).u32();
        let sum = toy_sum(e, toy);
        let count = e.mem.u32(toy + COUNT);
        toy_pop_all(e, toy);
        toy_push(e, toy, 11);
        (sum, by_address, count + toy_sum(e, toy), e.mem.heap_top())
    }

    #[test]
    fn every_form_gives_the_same_answers_and_k1_the_same_guest_heap() {
        let guest = run(&mut engine("toy"));
        assert_eq!((guest.0, guest.1, guest.2), (15, 15, 14));
        let mut dual = engine("toy:dual");
        assert_eq!(run(&mut dual), guest);
        let (compared, differed) = dual.moved().expect("made").dual.counts(TOY);
        assert!(compared > 10, "the dual run compared {compared} words");
        assert_eq!(differed, 0);
        // K1: the same guest allocations in the same order, so the same heap.
        let mut k1 = engine("toy:moved");
        let k1_run = run(&mut k1);
        assert_eq!(k1_run, guest);
        // K2: the same answers, the nodes and the record out of the guest heap.
        let mut k2 = engine("toy:freed");
        let k2_run = run(&mut k2);
        assert_eq!((k2_run.0, k2_run.1, k2_run.2), (15, 15, 14));
        assert!(k2_run.3 < guest.3, "{:08x} {:08x}", k2_run.3, guest.3);
        let census = k2.moved().expect("made").slab.census();
        assert_eq!(census, [(27, NODE_TAG, 1, 16), (27, RECORD_TAG, 1, 16)]);
    }

    /// A destroyed object takes its record and its nodes out of the slab, and the dual run's
    /// mirrors with them.
    #[test]
    fn a_destroyed_object_leaves_nothing_in_the_slab() {
        for mode in ["toy:dual", "toy:moved", "toy:freed"] {
            let mut e = engine(mode);
            let toy = toy_new(&mut e);
            toy_push(&mut e, toy, 4);
            toy_pop_all(&mut e, toy);
            let s = store_of(&e, toy);
            STUB.detach(&e, s, toy);
            if mode != "toy:dual" {
                // The spare word no longer names the record.
                assert_eq!(STUB.record(&e, toy), None, "{mode}");
            }
            e.call(DELETE, &[toy]);
            let moved = e.moved().expect("made");
            assert_eq!(moved.slab.census(), [], "{mode}");
            assert_eq!(moved.dual.mirror_of(toy + HEAD), None, "{mode}");
            assert_eq!(moved.dual.counts(TOY).1, 0, "{mode}");
        }
    }

    #[test]
    fn a_k1_stub_keeps_what_untyped_code_reads_and_poisons_the_rest() {
        let mut e = engine("toy:moved");
        let toy = toy_new(&mut e);
        toy_push(&mut e, toy, 9);
        assert_eq!(e.mem.u32(toy), VTABLE);
        assert_eq!(e.mem.u32(toy + COUNT), 1);
        assert_eq!(e.mem.u32(toy + HEAD), poison(TOY, HEAD));
        assert_eq!(poison_parts(e.mem.u32(toy + HEAD)), Some((27, 8)));
        assert!(STUB.poison_intact(&e, TOY, toy).is_ok());
        // An untyped reader of the moved field follows the poison and stops, naming it.
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.call(READS_HEAD, &[toy]);
        }))
        .expect_err("a missed reader stops");
        let why = stopped
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(why.contains("ffff1b08"), "{why}");
        // A missed writer shows when the poison is checked.
        e.mem.set_u32(toy + HEAD, 0);
        assert_eq!(STUB.poison_intact(&e, TOY, toy), Err((HEAD, 0)));
    }

    #[test]
    fn a_moved_object_cannot_be_reached_as_guest_memory() {
        let mut e = engine("toy:moved");
        let toy = toy_new(&mut e);
        toy_push(&mut e, toy, 9);
        let node = head(&e, store_of(&e, toy), toy);
        assert!(is_handle(node) && !e.mem.is_mapped(node));
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.mem.u32(node)));
        assert!(stopped.is_err());
    }

    #[test]
    fn the_dual_run_finds_an_untyped_writer() {
        let mut e = engine("toy:dual");
        let toy = toy_new(&mut e);
        toy_push(&mut e, toy, 9);
        let node = e.mem.u32(toy + HEAD);
        assert!(!is_handle(node), "the guest form is the truth");
        e.call(WRITES_NODE, &[node, 10]);
        // The game sees the guest form; the comparison sees the difference.
        assert_eq!(toy_sum(&mut e, toy), 10);
        let (_, differed) = e.moved().expect("made").dual.counts(TOY);
        assert_eq!(differed, 1);
    }

    #[test]
    fn a_stub_holds_its_vtable_for_untyped_calls_and_poison_after() {
        let mut e = engine("toy:freed");
        let moved = e.moved().expect("made");
        let h = moved.slab.alloc(0x20, TOY, 3, 0).expect("room");
        let stub = moved.stubs.alloc(&mut e.mem, TOY, VTABLE, h).expect("room");
        assert!(is_stub(stub) && stub % STUB_BYTES == 0);
        // An untyped virtual call through the stub reaches the class's function.
        assert_eq!(e.vcall(stub, 0, &[]).u32(), stub);
        assert_eq!(e.mem.u32(stub + 4), poison(TOY, 4));
        assert_eq!(moved.stubs.resolve(stub), Some((27, h)));
        assert_eq!(moved.stubs.resolve(stub + 4), None);
        moved.stubs.free(&e.mem, TOY, stub);
        assert_eq!(moved.stubs.resolve(stub), None);
        assert_eq!(e.mem.u32(stub), poison(TOY, 0));
        // The next one does not take the freed place (it waits: `tokens_and_stubs_wait_before_reuse`).
        let next = moved.stubs.alloc(&mut e.mem, TOY, VTABLE, h);
        assert_eq!(next, Some(stub + STUB_BYTES));
    }

    /// A buffer takes one token, 16 bytes of the 32-bit space, whatever its size; its bytes start
    /// zero, are read and written by offset, and are gone with it (back to the host at `reclaim`);
    /// reads past the end and of a freed token stop, and so does a second free.
    #[test]
    fn a_buffer_takes_one_token_whatever_its_size() {
        let buffers = Buffers::new();
        let big = buffers.alloc(40 << 20, TOY, 5).expect("a token");
        let small = buffers.alloc(12, TOY, 5).expect("a token");
        assert_eq!(big, TOKEN_RANGE.start);
        assert_eq!(small, big + STUB_BYTES);
        assert!(is_token(big) && !is_handle(big) && !is_stub(big));
        assert_eq!(buffers.len(big), Some(40 << 20));
        let mut got = [0xffu8; 8];
        buffers.read(big, (40 << 20) - 8, &mut got);
        assert_eq!(got, [0; 8]);
        buffers.write(small, 4, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let mut got = [0u8; 12];
        buffers.read(small, 0, &mut got);
        assert_eq!(got, [0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(buffers.census(), [(27, 5, 2, (40 << 20) + 12)]);
        assert!(std::panic::catch_unwind(|| buffers.read(small, 8, &mut [0; 8])).is_err());
        assert_eq!(buffers.free(big), 40 << 20);
        assert_eq!(buffers.len(big), None);
        assert!(std::panic::catch_unwind(|| buffers.read(big, 0, &mut [0; 1])).is_err());
        assert!(
            std::panic::catch_unwind(|| buffers.free(big)).is_err(),
            "freed twice"
        );
        assert_eq!(buffers.census(), [(27, 5, 1, 12)]);
        // The freed bytes wait for the barrier; the token for the quarantine.
        assert_eq!(buffers.retired_bytes(), 40 << 20);
        assert_eq!(buffers.reclaim(), 40 << 20);
        assert_eq!(buffers.retired_bytes(), 0);
        assert_eq!(buffers.alloc(4, TOY, 6), Some(small + STUB_BYTES));
        assert_eq!(buffers.len(small + 4), None);
    }

    /// The token comes back only past [`ADDRESS_QUARANTINE`] freed tokens, oldest first; so does a
    /// stub, and a stub freed twice, or by another family, stops.
    #[test]
    fn tokens_and_stubs_wait_before_reuse() {
        let buffers = Buffers::new();
        let tokens: Vec<u32> = (0..=ADDRESS_QUARANTINE)
            .map(|_| buffers.alloc(1, TOY, 1).expect("a token"))
            .collect();
        for &t in &tokens {
            buffers.free(t);
        }
        assert_eq!(buffers.alloc(1, TOY, 1), Some(tokens[0]));
        let mut e = engine("toy:freed");
        let stubs = &e.moved().expect("made").stubs;
        let first = stubs
            .alloc(&mut e.mem, TOY, VTABLE, SLAB_RANGE.start)
            .expect("a stub");
        let second = stubs
            .alloc(&mut e.mem, TOY, VTABLE, SLAB_RANGE.start + 16)
            .expect("a stub");
        stubs.free(&e.mem, TOY, first);
        let mut third = stubs
            .alloc(&mut e.mem, TOY, VTABLE, SLAB_RANGE.start)
            .expect("a stub");
        assert_eq!(third, second + STUB_BYTES, "not the freed one");
        let twice = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            stubs.free(&e.mem, TOY, first)
        }));
        assert!(twice.is_err(), "a stub freed twice stops");
        let other = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            stubs.free(&e.mem, FamilyId::new(26), second)
        }));
        assert!(other.is_err(), "another family's free stops");
        for _ in 0..ADDRESS_QUARANTINE {
            stubs.free(&e.mem, TOY, third);
            third = stubs
                .alloc(&mut e.mem, TOY, VTABLE, SLAB_RANGE.start)
                .expect("a stub");
        }
        assert_eq!(third, first, "back, oldest first");
        assert_eq!(e.mem.u32(third), VTABLE);
    }

    #[test]
    fn the_slab_hands_out_zeroed_blocks_and_counts_them() {
        let slab = Slab::new();
        let a = slab.alloc(12, TOY, 1, 0x2000_0000).expect("room");
        assert_eq!(a, SLAB_RANGE.start);
        let b = slab.alloc(17, TOY, 2, 0).expect("room");
        assert_eq!(b, a + 16);
        slab.store(a + 8, 7);
        slab.store_u8(b + 1, 0xab);
        assert_eq!((slab.load(a + 8), slab.load(b)), (7, 0xab00));
        assert_eq!(slab.load_u8(b + 1), 0xab);
        assert!(slab.cas(a + 8, 7, 8) && !slab.cas(a + 8, 7, 9));
        assert_eq!(slab.add(a + 8, 2), 10);
        assert_eq!(slab.census(), [(27, 1, 1, 16), (27, 2, 1, 32)]);
        assert_eq!(slab.shadowed(), [(27, 1, 1)]);
        let freed = slab.free(a);
        assert_eq!((freed.size, freed.shadow), (16, 0x2000_0000));
        assert_eq!(slab.shadowed(), []);
        // Freed: the freed words, and not handed out again yet.
        assert_eq!(slab.load(a + 8), crate::mem::freed_word(a));
        let again = slab.alloc(16, TOY, 1, 0).expect("room");
        assert_eq!((again, slab.load(again + 8)), (b + 32, 0));
        assert!(
            std::panic::catch_unwind(|| slab.free(a)).is_err(),
            "freed twice"
        );
        assert!(std::panic::catch_unwind(|| slab.load(SLAB_RANGE.end - 4)).is_err());
        assert!(std::panic::catch_unwind(|| slab.load(a + 2)).is_err());
    }

    /// A freed block comes back, zeroed, only once more than [`SLAB_QUARANTINE`] bytes freed after
    /// it have joined it, oldest first; until then a stale handle reads the freed words, and a
    /// pointer taken from them stops the run naming a freed moved object.
    #[test]
    fn freed_blocks_wait_in_a_quarantine() {
        let e = engine("toy:freed");
        let moved = e.moved().expect("made");
        let slab = &moved.slab;
        let stale = slab.alloc(16, TOY, NODE_TAG, 0).expect("room");
        slab.store(stale, 0x1234);
        slab.free(stale);
        assert_eq!(slab.load(stale), crate::mem::freed_word(stale));
        let why = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.mem.u32(slab.load(stale) + 4)
        }))
        .expect_err("a pointer from a freed block stops");
        let why = why.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(why.contains("freed moved object"), "{why}");
        // A block a quarter of the quarantine at a time: the fifth free lets the first one out.
        let big = u32::try_from(SLAB_QUARANTINE / 4).expect("a word");
        let mut blocks: Vec<u32> = (0..5)
            .map(|_| slab.alloc(big, TOY, NODE_TAG, 0).expect("room"))
            .collect();
        for &h in &blocks {
            slab.store(h, 7);
        }
        let first = blocks.remove(0);
        slab.free(first);
        for h in blocks {
            slab.free(h);
        }
        assert_eq!(
            slab.alloc(16, TOY, NODE_TAG, 0),
            Some(stale),
            "out of the quarantine"
        );
        assert_eq!(slab.load(stale), 0);
        assert_eq!(slab.alloc(big, TOY, NODE_TAG, 0), Some(first));
        assert_eq!(slab.load(first), 0, "zeroed again");
    }

    /// Real threads: a word counted up by four threads at once through compare-and-swap loses
    /// nothing, and blocks made on several threads never overlap.
    #[test]
    fn the_slab_is_shared_by_threads() {
        let moved = Moved::leak();
        let counter = moved.slab.alloc(4, TOY, 1, 0).expect("room");
        let handles: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    let mut mine = Vec::new();
                    for _ in 0..2000 {
                        loop {
                            let old = moved.slab.load(counter);
                            if moved.slab.cas(counter, old, old + 1) {
                                break;
                            }
                        }
                        mine.push(moved.slab.alloc(8, TOY, 2, 0).expect("room"));
                    }
                    mine
                })
            })
            .collect();
        let mut blocks: Vec<u32> = handles
            .into_iter()
            .flat_map(|h| h.join().expect("thread"))
            .collect();
        assert_eq!(moved.slab.load(counter), 8000);
        blocks.sort_unstable();
        blocks.dedup();
        assert_eq!(blocks.len(), 8000);
    }

    #[test]
    fn the_dual_mirror_translates_places_inside_blocks() {
        let d = Dual::default();
        d.add(0x2000_0100, 0x20, 0x1000_0040);
        assert_eq!(d.mirror_of(0x2000_0108), Some(0x1000_0048));
        assert_eq!(d.translate(0x2000_0101), 0x1000_0041, "a marked pointer");
        assert_eq!(d.translate(0x2000_0120), 0x2000_0120, "past the block");
        assert_eq!(d.translate(7), 7);
        assert!(d.check(TOY, 0x2000_0100, 1, 1, None));
        assert!(!d.check(TOY, 0x2000_0104, 1, 2, Some(0x0040_5000)));
        assert_eq!(d.counts(TOY), (2, 1));
        assert_eq!(d.remove(0x2000_0100), Some(0x1000_0040));
        assert_eq!(d.mirror_of(0x2000_0108), None);
    }

    #[test]
    fn poison_names_its_family_and_field() {
        assert_eq!(poison(TOY, 0x14), 0xffff_1b14);
        assert_eq!(poison_parts(0xffff_1b14), Some((27, 0x14)));
        assert_eq!(poison_parts(0xffff_1b15), None);
        assert_eq!(poison_parts(0x2000_0000), None);
        assert!(!is_handle(poison(TOY, 0)) && !is_stub(poison(TOY, 0)));
        assert!(is_handle(SLAB_RANGE.start) && !is_handle(SLAB_RANGE.end));
        assert!(is_stub(STUB_RANGE.start) && !is_stub(STUB_RANGE.end));
    }

    /// The status line's dual-run report names the family with its counts.
    #[test]
    fn the_dual_run_reports_its_counts() {
        crate::typed::provide(&TOY_FAMILY);
        let mut e = engine("toy:dual");
        assert_eq!(run(&mut e).0, 15);
        let line = crate::typed::dual_line(&e).expect("a dual run");
        assert!(line.starts_with("Dual run: toy: "), "{line}");
        assert!(line.ends_with(" compared, 0 differed."), "{line}");
        assert_eq!(crate::typed::dual_line(&engine("toy:moved")), None);
    }

    /// Whatever else the shell's environment turns on (`NV_PROFILE`, `NV_CENSUS`, ...): a dual run
    /// refuses real threads, and names itself as the reason when nothing else is one.
    #[test]
    fn a_dual_run_keeps_threads_stepped() {
        let mut e = engine("toy:dual");
        assert!(e.typed_any_dual());
        let why = crate::real_threads::start(&mut e).expect_err("stepped");
        let other = crate::real_threads::diagnostic_on(&engine("toy:moved"));
        if other.is_none() {
            assert!(why.contains("dual run"), "{why}");
        }
        let e = engine("toy:moved");
        assert!(!e.typed_any_dual());
    }

    /// Nor do the render helpers fork in a dual run: the caller does the work alone.
    #[test]
    fn a_dual_run_does_not_fork_the_render_helpers() {
        let mut e = engine("toy:dual");
        let ran = std::sync::atomic::AtomicUsize::new(0);
        let job = |_: &mut Engine, _: usize| {
            ran.fetch_add(1, Ordering::SeqCst);
        };
        assert!(!crate::real_threads::fork_join(&mut e, 2, &job));
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }

    /// The dual run prints a difference as one line naming the family, the place, both words and
    /// the function running.
    #[test]
    fn a_difference_prints_one_line() {
        assert_eq!(
            difference_line(TOY, 0x2000_0108, 0x1000_0040, 0x1000_0050, Some(0x0040_5000)),
            "dual run: family 27 differs at 20000108 in 00405000: the guest form holds 10000040 (as \
             the mirror would), the moved form 10000050"
        );
        assert!(!difference_line(TOY, 4, 1, 2, None).contains(" in "));
    }

    /// -1, -4, -256 and the like are values: only a family's number makes a poison word.
    #[test]
    fn words_near_the_top_are_poison_only_with_a_family() {
        for w in [0xffff_ffff, 0xffff_fffc, 0xffff_ff00, 0xffff_2000] {
            assert_eq!(poison_parts(w), None, "{w:08x}");
        }
        assert_eq!(poison_parts(0xffff_1f00), Some((31, 0)));
    }

    /// The store's other operations give the same answers in every form, and the dual run finds
    /// nothing to report: the interlocked add, the store-load ordered load and store, a copy (also
    /// from a word the family does not own, and across two of its blocks), and a resync after a
    /// call by address that wrote the family's word.
    #[test]
    fn the_store_adds_copies_and_resyncs_alike_in_every_form() {
        let mut seen = Vec::new();
        for mode in ["toy", "toy:dual", "toy:moved", "toy:freed"] {
            let mut e = engine(mode);
            let s = e.typed_store(TOY);
            let new = |e: &mut Engine| e.call(NEW, &[8]).u32();
            let a = s.alloc(&mut e, 8, NODE_TAG, new);
            let b = s.alloc(&mut e, 8, NODE_TAG, new);
            s.store(&e, a, 5);
            s.store_seq(&e, a + 4, 6);
            let added = s.add(&mut e, a, 3, |e| {
                let n = e.mem.u32(a) + 3;
                e.mem.set_u32(a, n);
                n
            });
            // A word of the caller's into the family's own: by address in the guest forms.
            let outside = e.mem.alloc(8);
            e.mem.set_u32(outside, 0x77);
            e.mem.set_u32(outside + 4, 0x78);
            let copy_in = |e: &mut Engine| {
                let w = [e.mem.u32(outside), e.mem.u32(outside + 4)];
                e.mem.set_u32(b, w[0]);
                e.mem.set_u32(b + 4, w[1]);
            };
            // (Moved: `b` is a handle and `outside` a guest word, copied in place.)
            s.copy(&mut e, b, outside, 2, copy_in);
            // The family's own block into its own (in place when moved).
            s.copy(&mut e, a, b, 2, |e| {
                let w = [e.mem.u32(b), e.mem.u32(b + 4)];
                e.mem.set_u32(a, w[0]);
                e.mem.set_u32(a + 4, w[1]);
            });
            // A call by address that wrote the family's word, and the resync after it.
            if !is_handle(b) {
                e.mem.set_u32(b, 0x99);
            } else {
                s.store(&e, b, 0x99);
            }
            s.resync(&e, b, 1);
            seen.push((
                mode,
                added,
                s.load(&e, a),
                s.load_seq(&e, a + 4),
                s.load(&e, b),
            ));
            if let Some(m) = e.moved() {
                assert_eq!(m.dual.counts(TOY).1, 0, "{mode}: no differences");
            }
        }
        for row in &seen {
            assert_eq!(
                (row.1, row.2, row.3, row.4),
                (8, 0x77, 0x78, 0x99),
                "{}",
                row.0
            );
        }
    }

    /// The dual run's copy from a word it does not mirror updates the destination's mirror, and a
    /// copy that runs from one mirrored block into the next keeps each in its own mirror.
    #[test]
    fn the_dual_copy_updates_every_mirrored_word() {
        let mut e = engine("toy:dual");
        let s = e.typed_store(TOY);
        let moved = e.moved().expect("made");
        // Guest bytes 0..8 mirrored by one slab block, 16..24 by the next, 8..16 by none.
        let g = e.mem.alloc(32);
        let first = moved.slab.alloc(8, TOY, NODE_TAG, 0).expect("room");
        let second = moved.slab.alloc(8, TOY, NODE_TAG, 0).expect("room");
        assert_eq!(second, first + 16, "the slab's blocks lie side by side");
        moved.dual.add(g, 8, first);
        moved.dual.add(g + 16, 8, second);
        let src = e.mem.alloc(24);
        for i in 0..6 {
            e.mem.set_u32(src + 4 * i, 0x50 + i);
        }
        // Six words from a source the dual run does not mirror, across both blocks.
        s.copy(&mut e, g, src, 6, |e| {
            for i in 0..6 {
                let w = e.mem.u32(src + 4 * i);
                e.mem.set_u32(g + 4 * i, w);
            }
        });
        let slab = |at: u32| moved.slab.load(at);
        assert_eq!((slab(first), slab(first + 4)), (0x50, 0x51));
        assert_eq!(
            (slab(first + 8), slab(first + 12)),
            (0, 0),
            "no word past the block"
        );
        assert_eq!((slab(second), slab(second + 4)), (0x54, 0x55));
        for i in 0..6 {
            assert_eq!(s.load(&e, g + 4 * i), 0x50 + i);
        }
        assert_eq!(moved.dual.counts(TOY).1, 0);
    }

    /// An untyped read or write through a buffer's token stops the run and says what it hit.
    #[test]
    fn an_untyped_access_through_a_token_stops() {
        let e = engine("toy:freed");
        let token = e
            .moved()
            .expect("made")
            .buffers
            .alloc(64, TOY, 1)
            .expect("a token");
        let why = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.mem.u32(token + 8)))
            .expect_err("a token is no guest memory");
        let why = why.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(why.contains("buffer token"), "{why}");
        let why = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.mem.set_u32(SLAB_RANGE.start, 1)
        }))
        .expect_err("a handle is no guest memory");
        let why = why.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(why.contains("slab handle"), "{why}");
    }

    /// Slab handles and tokens are never mapped as guest memory; stub pages are.
    #[test]
    fn the_slab_and_tokens_are_never_mapped() {
        for at in [
            SLAB_RANGE.start,
            TOKEN_RANGE.end - 0x1000,
            SLAB_RANGE.start - 0x1000,
        ] {
            let mut e = Engine::new_untyped();
            let len = if at < SLAB_RANGE.start {
                0x2000
            } else {
                0x1000
            };
            let tried =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.mem.map(at, len)));
            assert!(tried.is_err(), "{at:08x}");
        }
        let mut e = Engine::new_untyped();
        e.mem.map(STUB_RANGE.start, 0x1000);
        e.mem.map(SLAB_RANGE.start - 0x1000, 0x1000);
    }

    /// Buffers freed from several threads while others read them: each free happens once (a
    /// second free of the same token stops), reads see the bytes or stop, and the bytes go back
    /// to the host only at `reclaim`.
    #[test]
    fn buffers_are_freed_once_across_threads() {
        let moved = Moved::leak();
        let tokens: Vec<u32> = (0..64)
            .map(|i| {
                let t = moved.buffers.alloc(256, TOY, 1).expect("a token");
                moved
                    .buffers
                    .write(t, 0, &[u8::try_from(i).expect("small"); 4]);
                t
            })
            .collect();
        let frees = std::sync::atomic::AtomicUsize::new(0);
        let stops = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    for &t in &tokens {
                        let mut got = [0u8; 4];
                        let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            moved.buffers.read(t, 0, &mut got)
                        }));
                        assert!(read.is_err() || got[0] == got[3]);
                        let free = std::panic::AssertUnwindSafe(|| moved.buffers.free(t));
                        match std::panic::catch_unwind(free) {
                            Ok(len) => {
                                assert_eq!(len, 256);
                                frees.fetch_add(1, Ordering::SeqCst);
                            }
                            Err(_) => {
                                stops.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                });
            }
        });
        assert_eq!(frees.load(Ordering::SeqCst), 64, "each buffer freed once");
        assert_eq!(
            stops.load(Ordering::SeqCst),
            3 * 64,
            "every other free stopped"
        );
        assert_eq!(moved.buffers.retired_bytes(), 64 * 256);
        assert_eq!(moved.buffers.reclaim(), 64 * 256);
        assert_eq!(moved.buffers.census(), []);
    }

    /// Stubs freed from several threads at once: each exactly once (the table's part of the free;
    /// the poison words go into guest memory, which one thread's engine owns here).
    #[test]
    fn stubs_are_freed_once_across_threads() {
        let mut e = engine("toy:freed");
        let stubs = &e.moved().expect("made").stubs;
        let all: Vec<u32> = (0..64)
            .map(|i| {
                stubs
                    .alloc(&mut e.mem, TOY, VTABLE, SLAB_RANGE.start + 16 * i)
                    .expect("a stub")
            })
            .collect();
        let frees = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    for &s in &all {
                        let claim = std::panic::AssertUnwindSafe(|| stubs.claim(TOY, s));
                        if std::panic::catch_unwind(claim).is_ok() {
                            frees.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                });
            }
        });
        assert_eq!(frees.load(Ordering::SeqCst), 64);
        assert!(all.iter().all(|&s| stubs.resolve(s).is_none()));
    }

    /// K1's detach: an untyped writer of a moved field stops the destruction, naming the field; a
    /// second destruction finds no record and frees nothing; and typed code that reads a moved
    /// field without `K1::place` stops at the poison.
    #[test]
    fn k1_checks_its_poison_and_its_record() {
        let mut e = engine("toy:moved");
        let toy = toy_new(&mut e);
        let s = store_of(&e, toy);
        let why = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.load(&e, toy + HEAD)))
            .expect_err("the poison is no value");
        let why = why.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(why.contains("without K1::place"), "{why}");
        STUB.detach(&e, s, toy);
        let census = e.moved().expect("made").slab.census();
        STUB.detach(&e, s, toy);
        assert_eq!(
            e.moved().expect("made").slab.census(),
            census,
            "nothing freed twice"
        );
        let toy = toy_new(&mut e);
        e.call(WRITES_NODE, &[toy + HEAD, 0x1234]);
        let why =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| STUB.detach(&e, s, toy)))
                .expect_err("a missed writer");
        let why = why.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(why.contains("+8") && why.contains("00001234"), "{why}");
    }

    /// Mirrored ranges never overlap: a block mirrored inside another stops a debug build.
    #[cfg(debug_assertions)]
    #[test]
    fn mirrored_blocks_never_overlap() {
        let d = Dual::default();
        d.add(0x2000_0100, 0x20, 0x1000_0040);
        d.add(0x2000_0120, 0x10, 0x1000_0080);
        let add = |at: u32, len: u32| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| d.add(at, len, 0x1000_00c0)))
        };
        assert!(add(0x2000_0110, 4).is_err(), "inside one");
        assert!(add(0x2000_00f0, 0x20).is_err(), "across a start");
        assert!(add(0x2000_0130, 4).is_ok(), "after both");
    }
}
