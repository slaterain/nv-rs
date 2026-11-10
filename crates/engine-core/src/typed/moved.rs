//! Stage 2 of the typed migration (ADR-0014; docs/ENGINE_CRATE.md, "Moved families"): a typed
//! family's objects moved out of the game's heap into host memory, with the parts every family uses.
//!
//! - **The moved area** ([`MOVED_AREA`], `0x10000000..0x20000000`, below the heap: ordinary play never
//!   used it, and `VirtualAlloc` at a chosen address is refused there). Its lower half
//!   ([`SLAB_RANGE`]) names the objects in the [`Slab`]: a moved object's handle is an address there,
//!   and the guest page table never maps one, so an untyped read or write of a moved object stops the
//!   run naming the address. Its upper half ([`STUB_RANGE`]) holds 16-byte stubs ([`Stubs`]) for
//!   objects untyped code still reaches through their vtable. Below 0x80000000, so neither ever has
//!   its top bit set.
//! - **The slab**: host memory outside the guest space, addressed by those handles, every word an
//!   atomic (loads acquire, stores release, compare-and-swap sequentially consistent: what real
//!   threads need where the game uses interlocked operations, from the first step). A moved object
//!   keeps its PC layout, and its pointers to other moved objects are handles, so a pointer into an
//!   object (a link field, an array's element) works unchanged.
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
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The moved area: below the heap ([`crate::mem::HEAP_BASE`]) and above everything the image and
/// the tests use; `VirtualAlloc` at a chosen address fails inside it.
pub const MOVED_AREA: Range<u32> = 0x1000_0000..0x2000_0000;
/// The slab's handles ([`Slab`]): never mapped as guest memory.
pub const SLAB_RANGE: Range<u32> = 0x1000_0000..0x1800_0000;
/// The 16-byte stubs ([`Stubs`]): mapped page by page as stubs are handed out.
pub const STUB_RANGE: Range<u32> = 0x1800_0000..0x2000_0000;
/// A stub's bytes.
pub const STUB_BYTES: u32 = 16;
const _: () = assert!(
    MOVED_AREA.start == SLAB_RANGE.start
        && SLAB_RANGE.end == STUB_RANGE.start
        && STUB_RANGE.end == MOVED_AREA.end
        && MOVED_AREA.end <= crate::mem::HEAP_BASE
);

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

/// The family number and offset a poison word names; `None` for any other word.
pub fn poison_parts(word: u32) -> Option<(u8, u8)> {
    (word & 0xffff_0000 == POISON_BASE && word & 3 == 0)
        .then(|| (byte(word >> 8), byte(word)))
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

#[derive(Default)]
struct SlabAlloc {
    /// The first handle never handed out.
    top: u32,
    /// Freed blocks by size.
    free: HashMap<u32, Vec<u32>>,
    /// The live blocks.
    live: HashMap<u32, Block>,
    /// Live blocks and bytes by (family, tag).
    census: BTreeMap<(u8, u16), (u64, u64)>,
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
        self.cell(a)
            .fetch_add(v, Ordering::SeqCst)
            .wrapping_add(v)
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
            Some(h) => h,
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
        Some(h)
    }

    /// Frees block `h` (its words cleared, so a block handed out again reads zero) and answers what
    /// it was. Stops when `h` is no live block's start.
    pub fn free(&self, h: u32) -> Block {
        let mut a = lock(&self.alloc);
        let Some(block) = a.live.remove(&h) else {
            panic!("moved object: {h:08x} freed, but it is no live block of the slab");
        };
        for w in (h..h + block.size).step_by(4) {
            self.store(w, 0);
        }
        a.free.entry(block.size).or_default().push(h);
        if let Some(row) = a.census.get_mut(&(block.family, block.tag)) {
            row.0 -= 1;
            row.1 -= u64::from(block.size);
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
            .map(|(&(f, t), &(n, b))| (f, t, n, b))
            .collect()
    }
}

/// Stubs a chunk of the stub table holds.
const STUB_CHUNK: usize = 1 << 16;
/// The bytes of a guest page, which the stubs map one at a time.
const PAGE: u32 = 0x1000;

#[derive(Default)]
struct StubAlloc {
    top: u32,
    free: Vec<u32>,
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
                free: Vec::new(),
            }),
        }
    }

    /// A stub for `family`'s object `handle`: [`vtable`, poison, poison, poison] in guest memory
    /// (its page mapped when new); `None` when the range is full.
    pub fn alloc(&self, mem: &mut Mem, family: FamilyId, vtable: u32, handle: u32) -> Option<u32> {
        let stub = {
            let mut a = lock(&self.alloc);
            match a.free.pop() {
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

    /// Frees stub `stub`: all four words poison (a stale holder's virtual call then stops), the
    /// table entry cleared.
    pub fn free(&self, mem: &Mem, family: FamilyId, stub: u32) {
        let Some(entry) = self.table.get(Self::slot(stub)) else {
            panic!("stub {stub:08x} freed, but it was never handed out");
        };
        entry.store(0, Ordering::Release);
        for off in [0, 4, 8, 12] {
            mem.set_u32(stub + off, poison(family, off));
        }
        lock(&self.alloc).free.push(stub);
    }

    fn slot(stub: u32) -> usize {
        index((stub - STUB_RANGE.start) / STUB_BYTES)
    }
}

/// The first differences the dual run prints (the rest are counted).
pub const DUAL_PRINT: u32 = 20;
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
    /// Guest block `[guest, guest + len)` is mirrored by the slab block `handle`.
    pub fn add(&self, guest: u32, len: u32, handle: u32) {
        lock(&self.mirror).insert(guest, (len, handle));
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
            let running = running.map_or(String::new(), |r| format!(" in {r:08x}"));
            eprintln!(
                "dual run: family {} differs at {at:08x}{running}: the guest form holds {guest:08x} \
                 (as the mirror would), the moved form {seen:08x}",
                family.number()
            );
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
    /// made on both and compared.
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

    /// The word at `a`: a guest address or a handle.
    #[inline]
    pub fn load(self, e: &Engine, a: u32) -> u32 {
        match self {
            Store::Guest => e.mem.u32(a),
            Store::Moved { moved, .. } => {
                if is_handle(a) {
                    moved.slab.load(a)
                } else {
                    e.mem.u32(a)
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

    /// Copies `words` words from `from` to `to`. Guest to guest goes through `by_address` (the
    /// family's call by address, as in stage 1); the dual run also copies the mirrors; moved words
    /// are copied in place.
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
            Store::Dual { moved, .. } => {
                by_address(e);
                let d = &moved.dual;
                if let (Some(t), Some(f)) = (d.mirror_of(to), d.mirror_of(from)) {
                    for i in 0..words {
                        moved.slab.store(t + 4 * i, moved.slab.load(f + 4 * i));
                    }
                }
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
    /// its mirrors, which name the record there).
    pub fn detach(&self, e: &Engine, s: Store, obj: u32) {
        let (moved, record) = match s {
            Store::Guest => return,
            Store::Moved { moved, .. } => (moved, self.record(e, obj)),
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
        // The next one takes the freed place.
        assert_eq!(moved.stubs.alloc(&mut e.mem, TOY, VTABLE, h), Some(stub));
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
        let freed = slab.free(a);
        assert_eq!((freed.size, freed.shadow), (16, 0x2000_0000));
        let again = slab.alloc(16, TOY, 1, 0).expect("room");
        assert_eq!((again, slab.load(again + 8)), (a, 0));
        assert!(std::panic::catch_unwind(|| slab.load(SLAB_RANGE.end - 4)).is_err());
        assert!(std::panic::catch_unwind(|| slab.load(a + 2)).is_err());
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

    #[test]
    fn a_dual_run_keeps_threads_stepped() {
        let mut e = engine("toy:dual");
        assert!(e.typed_any_dual());
        let why = crate::real_threads::start(&mut e).expect_err("stepped");
        assert!(why.contains("dual run"), "{why}");
        let e = engine("toy:moved");
        assert!(!e.typed_any_dual());
    }
}
