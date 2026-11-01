//! The game's 32-bit address space.
//!
//! Game objects live here at their real field offsets, and pointers are the
//! 32-bit addresses the game uses. Memory is paged (4 KiB) and sparse: a
//! page exists once something maps or writes it. Reading an unmapped
//! address is a fault that names the address, so a translation that reads
//! data nobody provided stops at once instead of computing with zeros.
//!
//! Regions:
//! - the executable's data sections at their own addresses
//!   (`.rdata` 0x00FDF000, `.data` 0x01183000, ...), mapped from the
//!   player's installed FalloutNV.exe by [`crate::exe`] or set up by a test;
//! - the heap, from [`HEAP_BASE`], handed out by [`Mem::alloc`] (the
//!   translation of the game's allocator is `platform`, see
//!   `units/platform.rs`); its free space and the choice of where a block goes
//!   are in `mem/heap.rs`.

use std::collections::{BTreeMap, HashMap};

mod heap;

const PAGE_BITS: u32 = 12;
const PAGE: usize = 1 << PAGE_BITS;
const PAGES: usize = 1 << (32 - PAGE_BITS);

/// First heap address. Above the executable image (which ends below
/// 0x01410000) and clear of it, so a heap pointer never aliases image data.
pub const HEAP_BASE: u32 = 0x2000_0000;
/// End of the heap region.
pub const HEAP_END: u32 = 0x7FFF_0000;

/// A read or write of memory nobody mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fault(pub u32);

pub struct Mem {
    /// One slot per page of the 4 GB space: a fixed-size table, so an index made from a `u32`
    /// address (`a >> PAGE_BITS`) is in range by its type and needs no bounds check.
    pages: Box<[Option<Box<[u8; PAGE]>>; PAGES]>,
    /// The heap's live blocks (start -> size, rounded and with any guard) and its free space.
    live: crate::engine::AddrMap<u32, u32>,
    heap: heap::Heap,
    /// The write watch ([`Mem::watch`]); `None` in normal runs (one test per store).
    watch: Option<Box<Watch>>,
    /// The block check ([`Mem::enable_block_check`]); `None` in normal runs.
    check: Option<Box<BlockCheck>>,
    /// The block check looks at every store ([`Mem::enable_block_check`] with `store`).
    check_store: bool,
    /// A store diagnostic is on: the write watch or the block check's store mode.
    store_diag: bool,
}

/// What a watched range saw ([`Mem::watch`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchKind {
    /// A store that changed the bytes.
    Write,
    /// [`Mem::zero`] changed the bytes.
    Zero,
    /// [`Mem::alloc`] handed (part of) the range out again (zero-filled).
    Alloc,
    /// [`Mem::free`] freed the block holding (part of) the range.
    Free,
}

/// One event in a watched range: the range's start, the address and length touched (at
/// most 8 bytes of a store are kept), the bytes before and after (little-endian).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchHit {
    pub start: u32,
    pub addr: u32,
    pub len: u32,
    pub old: u64,
    pub new: u64,
    pub kind: WatchKind,
}

/// The ranges a diagnostic watches (start -> length and whether only frees count), a
/// count of ranges per page (the store's quick test) and the events not yet taken.
struct Watch {
    ranges: BTreeMap<u32, (u32, bool)>,
    pages: Vec<u16>,
    hits: Vec<WatchHit>,
}

/// The most events kept before [`Mem::take_watch_hits`] takes them (later ones dropped).
const WATCH_HITS_KEEP: usize = 1 << 16;

/// Bytes of guard after every block while the block check is on ([`Mem::enable_block_check`]).
pub const GUARD: u32 = 16;
/// The byte guards are filled with.
pub const GUARD_BYTE: u8 = 0xFD;

/// The block check (a diagnostic, not game behaviour): every heap block (and so every
/// `Engine::with_stack` local) gets [`GUARD`] bytes of [`GUARD_BYTE`] right after the size
/// asked for; a store into them (with `store`) or guard bytes found changed when the block is
/// freed is a write past the block's end, reported with the functions involved.
struct BlockCheck {
    /// Check every store (else only at free).
    store: bool,
    /// Stop (panic) at the first finding instead of only reporting it.
    stop: bool,
    /// Live blocks: size asked for and the function that was running when it was allocated.
    blocks: HashMap<u32, (u32, u32)>,
    /// Guard ranges: start -> (end, block).
    guards: BTreeMap<u32, (u32, u32)>,
    /// Guard ranges per page (the store's quick test).
    pages: Vec<u16>,
    /// The innermost function entered through `Engine::call` (the engine keeps it up to date).
    current_fn: u32,
    /// Findings not yet taken ([`Mem::take_block_reports`]).
    reports: Vec<String>,
    /// With [`Mem::enable_freed_check`]: freed blocks held back from reuse, oldest first
    /// (address, rounded size), each guarded whole so that a store into it is reported, until
    /// more than `quarantine_limit` bytes ([`QUARANTINE_BYTES`]) wait.
    quarantine: Option<std::collections::VecDeque<(u32, u32)>>,
    quarantine_bytes: u64,
    quarantine_limit: u64,
    /// The quarantined blocks: size asked for, the function running when it was allocated and
    /// the one running when it was freed.
    freed: HashMap<u32, (u32, u32, u32)>,
    /// The engine's calls (innermost first) the game's allocator or deallocator gave for the next
    /// block it hands out or frees ([`Mem::note_path`]), the allocation paths of live blocks, and
    /// both paths of quarantined ones.
    pending_path: Vec<u32>,
    alloc_paths: HashMap<u32, Vec<u32>>,
    freed_paths: HashMap<u32, (Vec<u32>, Vec<u32>)>,
}

/// Bytes of freed blocks the freed-block check keeps from reuse at most.
const QUARANTINE_BYTES: u64 = 64 << 20;
/// The word a freed block at `addr` is filled with while it waits: `0xD0000000` with the
/// block's address shifted down by 4, above the heap, so a pointer taken from the block leads to
/// unmapped memory, and the read through it names the block ([`Mem::freed_block_of`]).
pub fn freed_word(addr: u32) -> u32 {
    0xd000_0000 | (addr >> 4)
}

impl Mem {
    /// Turns the block check on (only blocks allocated from now on are guarded). `store`:
    /// look at every store and name the function that writes into a guard; else the guard is
    /// checked when its block is freed. `stop`: panic at the first finding.
    pub fn enable_block_check(&mut self, store: bool, stop: bool) {
        self.check = Some(Box::new(BlockCheck {
            store,
            stop,
            blocks: HashMap::new(),
            guards: BTreeMap::new(),
            pages: if store { vec![0; PAGES] } else { Vec::new() },
            current_fn: 0,
            reports: Vec::new(),
            quarantine: None,
            quarantine_bytes: 0,
            quarantine_limit: QUARANTINE_BYTES,
            freed: HashMap::new(),
            pending_path: Vec::new(),
            alloc_paths: HashMap::new(),
            freed_paths: HashMap::new(),
        }));
        self.check_store = store;
        self.store_diag |= store;
    }

    /// With the block check on: the engine's calls `calls` (outermost first, as
    /// `Engine::call_stack` holds them) for the next block the game's allocator hands out or its
    /// deallocator frees, kept (the innermost 10) to name where a freed block came from.
    pub fn note_path(&mut self, calls: &[u32]) {
        if let Some(c) = self.check.as_deref_mut() {
            c.pending_path.clear();
            c.pending_path.extend(calls.iter().rev().take(10));
        }
    }

    /// Adds the freed-block check to the block check (turning that on with `store` when it is
    /// off): a freed block is not handed out again at once but waits, guarded whole, behind
    /// [`QUARANTINE_BYTES`] of later frees; a store into it meanwhile is reported with the
    /// functions that allocated, freed and wrote it (a write through a stale pointer, which a
    /// reused block would take silently; also one through a pointer kept to an ended
    /// `Engine::with_stack` frame, which in the exe would land on the dead stack instead).
    pub fn enable_freed_check(&mut self, stop: bool) {
        if !self.check.as_deref().is_some_and(|c| c.store) {
            self.enable_block_check(true, stop);
        }
        if let Some(c) = self.check.as_deref_mut() {
            c.quarantine.get_or_insert_with(Default::default);
        }
    }

    /// With the freed-block check, keeps the freed block `addr` (rounded `size`, `info` the
    /// size asked for and the allocating function) back and answers the oldest blocks that
    /// leave the quarantine for the free lists; `None` without the check.
    fn quarantine_block(
        &mut self,
        addr: u32,
        size: u32,
        info: Option<(u32, u32)>,
    ) -> Option<Vec<(u32, u32)>> {
        self.check.as_deref()?.quarantine.as_ref()?;
        // Filled with `freed_word`, so a pointer read from a freed object points nowhere and the
        // next access through it stops the run in the function that kept using the object.
        let fill: Vec<u8> = std::iter::repeat(freed_word(addr).to_le_bytes())
            .take(size as usize / 4)
            .flatten()
            .collect();
        let _ = self.try_write_raw(addr, &fill);
        let c = self.check.as_deref_mut()?;
        let queue = c.quarantine.as_mut()?;
        let (asked, owner) = info.unwrap_or((size, 0));
        queue.push_back((addr, size));
        c.quarantine_bytes += u64::from(size);
        c.freed.insert(addr, (asked, owner, c.current_fn));
        let alloc_path = c.alloc_paths.remove(&addr).unwrap_or_default();
        let free_path = std::mem::take(&mut c.pending_path);
        c.freed_paths.insert(addr, (alloc_path, free_path));
        c.guards.insert(addr, (addr + size, addr));
        for p in (addr >> PAGE_BITS)..=((addr + size - 1) >> PAGE_BITS) {
            c.pages[p as usize] = c.pages[p as usize].saturating_add(1);
        }
        let mut released = Vec::new();
        while c.quarantine_bytes > c.quarantine_limit {
            let Some((a, s)) = queue.pop_front() else {
                break;
            };
            c.quarantine_bytes -= u64::from(s);
            c.freed.remove(&a);
            c.freed_paths.remove(&a);
            c.guards.remove(&a);
            for p in (a >> PAGE_BITS)..=((a + s - 1) >> PAGE_BITS) {
                c.pages[p as usize] = c.pages[p as usize].saturating_sub(1);
            }
            released.push((a, s));
        }
        Some(released)
    }

    /// Whether the block check is on.
    #[inline(always)]
    pub fn block_check_on(&self) -> bool {
        self.check.is_some()
    }

    /// Sets the function the block check names as running; returns the one before.
    pub fn set_check_fn(&mut self, f: u32) -> u32 {
        match self.check.as_deref_mut() {
            Some(c) => std::mem::replace(&mut c.current_fn, f),
            None => 0,
        }
    }

    /// Takes the block check's findings, oldest first.
    pub fn take_block_reports(&mut self) -> Vec<String> {
        self.check
            .as_deref_mut()
            .map(|c| std::mem::take(&mut c.reports))
            .unwrap_or_default()
    }

    fn check_report(&mut self, text: String) {
        let Some(c) = self.check.as_deref_mut() else {
            return;
        };
        eprintln!("NV_BLOCK_CHECK: {text}");
        if c.stop {
            panic!("NV_BLOCK_CHECK: {text}");
        }
        c.reports.push(text);
    }

    /// After a guarded block of `asked` bytes at `addr` (`total` bytes in all) was handed out.
    fn check_guard_block(&mut self, addr: u32, asked: u32, total: u32) {
        let start = addr + asked;
        let end = addr + total;
        let fill = vec![GUARD_BYTE; (end - start) as usize];
        let _ = self.try_write_raw(start, &fill);
        let Some(c) = self.check.as_deref_mut() else {
            return;
        };
        c.blocks.insert(addr, (asked, c.current_fn));
        if !c.pending_path.is_empty() {
            let path = std::mem::take(&mut c.pending_path);
            c.alloc_paths.insert(addr, path);
        }
        c.guards.insert(start, (end, addr));
        if c.store {
            for p in (start >> PAGE_BITS)..=((end - 1) >> PAGE_BITS) {
                c.pages[p as usize] = c.pages[p as usize].saturating_add(1);
            }
        }
    }

    /// Before a guarded block is freed: its guard checked and dropped.
    fn check_free_block(&mut self, addr: u32) {
        let Some(c) = self.check.as_deref_mut() else {
            return;
        };
        let Some((asked, owner)) = c.blocks.remove(&addr) else {
            return;
        };
        let start = addr + asked;
        let Some((end, _)) = c.guards.remove(&start) else {
            return;
        };
        if c.store {
            for p in (start >> PAGE_BITS)..=((end - 1) >> PAGE_BITS) {
                c.pages[p as usize] = c.pages[p as usize].saturating_sub(1);
            }
        }
        let freer = c.current_fn;
        let mut guard = vec![0u8; (end - start) as usize];
        if self.try_read(start, &mut guard).is_err() {
            return;
        }
        if let Some(i) = guard.iter().position(|&b| b != GUARD_BYTE) {
            let last = guard.iter().rposition(|&b| b != GUARD_BYTE).unwrap_or(i);
            self.check_report(format!(
                "block {addr:08x} ({asked:#x} bytes, allocated while {owner:08x} ran) was written \
                 past its end: +{:#x}..+{:#x} changed (found when it was freed, while {freer:08x} ran)",
                asked + i as u32,
                asked + last as u32 + 1
            ));
        }
    }

    /// A store of `len` bytes at `a` on a page with guards: one that reaches a guard is
    /// reported with the running function.
    #[cold]
    #[inline(never)]
    fn check_store_at(&mut self, a: u32, len: u32) {
        let Some(c) = self.check.as_deref() else {
            return;
        };
        let last = a.wrapping_add(len.max(1) - 1);
        if c.pages[(a >> PAGE_BITS) as usize] == 0 && c.pages[(last >> PAGE_BITS) as usize] == 0 {
            return;
        }
        let end = (a as u64 + len.max(1) as u64).min(u32::MAX as u64) as u32;
        // The first guard the store reaches: one that starts at or below `a` and runs past it,
        // else the lowest one starting inside the store (a store past a live block's end that
        // also reaches a freed neighbour is named for the live block's guard first).
        let hit = c
            .guards
            .range(..=a)
            .next_back()
            .filter(|(_, &(gend, _))| gend > a)
            .or_else(|| c.guards.range(a.saturating_add(1)..end).next())
            .map(|(&gstart, &(_, block))| (gstart, block));
        let Some((gstart, block)) = hit else {
            return;
        };
        let writer = c.current_fn;
        if let Some(&(asked, owner, freer)) = c.freed.get(&block) {
            self.check_report(format!(
                "{writer:08x} wrote {len} byte(s) at {a:08x}, +{:#x} into block {block:08x} \
                 ({asked:#x} bytes, allocated while {owner:08x} ran) after it was freed while \
                 {freer:08x} ran",
                a.max(gstart) - block
            ));
            return;
        }
        let (asked, owner) = c.blocks.get(&block).copied().unwrap_or((0, 0));
        self.check_report(format!(
            "{writer:08x} wrote {len} byte(s) at {a:08x}, +{:#x} into block {block:08x} \
             ({asked:#x} bytes, allocated while {owner:08x} ran): past its end",
            (a.max(gstart)) - block
        ));
    }
}

impl Default for Mem {
    fn default() -> Self {
        Self::new()
    }
}

impl Mem {
    pub fn new() -> Self {
        let mut pages = Vec::with_capacity(PAGES);
        pages.resize_with(PAGES, || None);
        let pages: Box<[Option<Box<[u8; PAGE]>>; PAGES]> =
            pages.into_boxed_slice().try_into().expect("PAGES slots");
        Mem {
            pages,
            live: Default::default(),
            heap: heap::Heap::new(),
            watch: None,
            check: None,
            check_store: false,
            store_diag: false,
        }
    }

    /// Diagnostic, not game behaviour: records events in `[start, start + len)` (stores
    /// that change bytes, zeroing, re-allocation, frees; with `frees_only` just frees) for
    /// [`Mem::take_watch_hits`]. A range already watched at `start` is replaced.
    pub fn watch(&mut self, start: u32, len: u32, frees_only: bool) {
        if len == 0 {
            return;
        }
        self.unwatch(start);
        let w = self.watch.get_or_insert_with(|| {
            Box::new(Watch {
                ranges: BTreeMap::new(),
                pages: vec![0; PAGES],
                hits: Vec::new(),
            })
        });
        w.ranges.insert(start, (len, frees_only));
        self.store_diag = true;
        let last = (start as u64 + len as u64 - 1) >> PAGE_BITS;
        for p in (start >> PAGE_BITS) as u64..=last {
            w.pages[p as usize] = w.pages[p as usize].saturating_add(1);
        }
    }

    /// Stops watching the range that starts at `start` ([`Mem::watch`]).
    pub fn unwatch(&mut self, start: u32) {
        let Some(w) = self.watch.as_deref_mut() else {
            return;
        };
        let Some((len, _)) = w.ranges.remove(&start) else {
            return;
        };
        let last = (start as u64 + len as u64 - 1) >> PAGE_BITS;
        for p in (start >> PAGE_BITS) as u64..=last {
            w.pages[p as usize] = w.pages[p as usize].saturating_sub(1);
        }
    }

    /// Whether watched ranges have events not yet taken.
    #[inline]
    pub fn has_watch_hits(&self) -> bool {
        self.watch.as_deref().is_some_and(|w| !w.hits.is_empty())
    }

    /// Takes the watched ranges' events, oldest first.
    pub fn take_watch_hits(&mut self) -> Vec<WatchHit> {
        self.watch
            .as_deref_mut()
            .map(|w| std::mem::take(&mut w.hits))
            .unwrap_or_default()
    }

    /// Whether `[a, a + len)` touches a page with a watched range.
    #[inline(always)]
    fn watched_page(&self, a: u32, len: u32) -> bool {
        match self.watch.as_deref() {
            None => false,
            Some(w) => {
                let last = a.wrapping_add(len.max(1) - 1);
                w.pages[(a >> PAGE_BITS) as usize] != 0
                    || w.pages[(last >> PAGE_BITS) as usize] != 0
            }
        }
    }

    /// Records `kind` over `[a, a + len)` for each watched range it overlaps; `new` is the
    /// bytes a store writes (zeros otherwise). Unchanged bytes are not an event.
    #[cold]
    #[inline(never)]
    fn watch_note(&mut self, a: u32, len: u32, new: Option<&[u8]>, kind: WatchKind) {
        let end = a as u64 + len.max(1) as u64;
        let Some(w) = self.watch.as_deref() else {
            return;
        };
        let mut overlaps = Vec::new();
        for (&start, &(rlen, frees_only)) in w
            .ranges
            .range(..=((end - 1).min(u32::MAX as u64) as u32))
            .rev()
        {
            let rend = start as u64 + rlen as u64;
            if rend <= a as u64 {
                // Ranges do not overlap one another, so none further down reaches `a`.
                break;
            }
            if frees_only && kind != WatchKind::Free {
                continue;
            }
            overlaps.push((start, rend));
        }
        for (start, rend) in overlaps {
            let from = (a as u64).max(start as u64) as u32;
            let to = end.min(rend);
            let n = ((to - from as u64) as usize).min(8);
            let mut old = [0u8; 8];
            if self.try_read(from, &mut old[..n]).is_err() {
                continue;
            }
            let mut fresh = [0u8; 8];
            if let Some(bytes) = new {
                let skip = (from - a) as usize;
                let take = n.min(bytes.len().saturating_sub(skip));
                fresh[..take].copy_from_slice(&bytes[skip..skip + take]);
            }
            let (old, fresh) = (u64::from_le_bytes(old), u64::from_le_bytes(fresh));
            if matches!(kind, WatchKind::Write | WatchKind::Zero) && old == fresh {
                continue;
            }
            let w = self.watch.as_deref_mut().expect("watch");
            if w.hits.len() < WATCH_HITS_KEEP {
                w.hits.push(WatchHit {
                    start,
                    addr: from,
                    len: (to - from as u64) as u32,
                    old,
                    new: fresh,
                    kind,
                });
            }
        }
    }

    /// Makes `[addr, addr + len)` addressable (zero-filled where new).
    pub fn map(&mut self, addr: u32, len: u32) {
        if len == 0 {
            return;
        }
        let first = addr >> PAGE_BITS;
        let last = (addr as u64 + len as u64 - 1) >> PAGE_BITS;
        for p in first as u64..=last {
            let slot = &mut self.pages[p as usize];
            if slot.is_none() {
                *slot = Some(Box::new([0; PAGE]));
            }
        }
    }

    /// Makes the pages of `[addr, addr + len)` unaddressable again; their
    /// contents are dropped. `addr` and `len` must be page multiples (the
    /// `VirtualFree` stand-in in `units/platform.rs` decommits this way).
    pub fn unmap(&mut self, addr: u32, len: u32) {
        assert!(
            addr % PAGE as u32 == 0 && len % PAGE as u32 == 0,
            "unmap of {addr:08x}+{len:x}: not whole pages"
        );
        let first = (addr >> PAGE_BITS) as u64;
        for p in first..first + (len >> PAGE_BITS) as u64 {
            self.pages[p as usize] = None;
        }
    }

    pub fn is_mapped(&self, addr: u32) -> bool {
        self.pages[(addr >> PAGE_BITS) as usize].is_some()
    }

    fn page(&self, addr: u32) -> Result<&[u8; PAGE], Fault> {
        self.pages[(addr >> PAGE_BITS) as usize]
            .as_deref()
            .ok_or(Fault(addr))
    }

    fn page_mut(&mut self, addr: u32) -> Result<&mut [u8; PAGE], Fault> {
        self.pages[(addr >> PAGE_BITS) as usize]
            .as_deref_mut()
            .ok_or(Fault(addr))
    }

    /// Reads `buf.len()` bytes; faults on the first unmapped byte.
    pub fn try_read(&self, addr: u32, buf: &mut [u8]) -> Result<(), Fault> {
        let mut a = addr;
        let mut done = 0;
        while done < buf.len() {
            let off = (a as usize) & (PAGE - 1);
            let n = (PAGE - off).min(buf.len() - done);
            let p = self.page(a)?;
            buf[done..done + n].copy_from_slice(&p[off..off + n]);
            done += n;
            a = a.wrapping_add(n as u32);
        }
        Ok(())
    }

    pub fn try_write(&mut self, addr: u32, data: &[u8]) -> Result<(), Fault> {
        if self.check_store {
            self.check_store_at(addr, data.len() as u32);
        }
        if self.watched_page(addr, data.len() as u32) {
            self.watch_note(addr, data.len() as u32, Some(data), WatchKind::Write);
        }
        self.try_write_raw(addr, data)
    }

    /// [`Mem::try_write`] without the write watch.
    fn try_write_raw(&mut self, addr: u32, data: &[u8]) -> Result<(), Fault> {
        let mut a = addr;
        let mut done = 0;
        while done < data.len() {
            let off = (a as usize) & (PAGE - 1);
            let n = (PAGE - off).min(data.len() - done);
            let p = self.page_mut(a)?;
            p[off..off + n].copy_from_slice(&data[done..done + n]);
            done += n;
            a = a.wrapping_add(n as u32);
        }
        Ok(())
    }

    /// Zeroes `[addr, addr + len)` page by page (what writing `len` zero bytes does, without
    /// a buffer of them). Panics as [`Mem::write`] does on the first unmapped byte, with the
    /// bytes before it zeroed.
    pub fn zero(&mut self, addr: u32, len: u32) {
        if self.check_store {
            self.check_store_at(addr, len);
        }
        if self.watched_page(addr, len) {
            self.watch_note(addr, len, None, WatchKind::Zero);
        }
        self.zero_raw(addr, len);
    }

    /// [`Mem::zero`] without the write watch.
    fn zero_raw(&mut self, addr: u32, len: u32) {
        let mut a = addr;
        let mut left = len as usize;
        while left > 0 {
            let off = (a as usize) & (PAGE - 1);
            let n = (PAGE - off).min(left);
            match self.page_mut(a) {
                Ok(p) => p[off..off + n].fill(0),
                Err(Fault(at)) => panic!("write to unmapped memory at {at:08x}"),
            }
            left -= n;
            a = a.wrapping_add(n as u32);
        }
    }

    /// Reads bytes. Panics with the address on a fault: a translated
    /// function touched memory that was never provided.
    pub fn read(&self, addr: u32, buf: &mut [u8]) {
        if let Err(Fault(a)) = self.try_read(addr, buf) {
            match self.freed_block_of(a) {
                Some(block) => panic!("read of unmapped memory at {a:08x} ({block})"),
                None => panic!("read of unmapped memory at {a:08x}"),
            }
        }
    }

    /// With the freed-block check: the waiting freed block a fault at `a` came from, when `a` is
    /// a [`freed_word`] plus less than 64 KB: the block and what the check knows of it.
    #[cold]
    fn freed_block_of(&self, a: u32) -> Option<String> {
        let c = self.check.as_deref()?;
        c.quarantine.as_ref()?;
        if a < 0xd000_0000 {
            return None;
        }
        let word = a & 0x0fff_ffff;
        (0..0x1_0000u32.min(word + 1)).find_map(|offset| {
            let base = (word - offset) << 4;
            [base, base + 8].into_iter().find_map(|block| {
                c.freed.get(&block).map(|&(asked, owner, freer)| {
                    let path = |p: &[u32]| {
                        p.iter()
                            .map(|f| format!("{f:08x}"))
                            .collect::<Vec<_>>()
                            .join(" < ")
                    };
                    let (alloc, free) = c.freed_paths.get(&block).cloned().unwrap_or_default();
                    format!(
                        "a pointer read from the freed block {block:08x}, plus {offset:#x}: \
                         {asked:#x} bytes, allocated while {owner:08x} ran (in {}), freed while \
                         {freer:08x} ran (in {})",
                        path(&alloc),
                        path(&free)
                    )
                })
            })
        })
    }

    pub fn write(&mut self, addr: u32, data: &[u8]) {
        if let Err(Fault(a)) = self.try_write(addr, data) {
            panic!("write to unmapped memory at {a:08x}");
        }
    }

    pub fn bytes(&self, addr: u32, len: u32) -> Vec<u8> {
        let mut v = vec![0; len as usize];
        self.read(addr, &mut v);
        v
    }

    /// The `N` bytes at `a`: a fixed-size copy out of the page when they lie in one page (no
    /// call to `memcpy`, which a variable-length copy costs; the translated code reads memory
    /// hundreds of millions of times a second), else [`Mem::read`] across the page boundary.
    /// Faults (panics) exactly as [`Mem::read`] does.
    #[inline(always)]
    fn load<const N: usize>(&self, a: u32) -> [u8; N] {
        let off = (a as usize) & (PAGE - 1);
        if off <= PAGE - N {
            if let Some(p) = self.pages[(a >> PAGE_BITS) as usize].as_deref() {
                let mut b = [0; N];
                b.copy_from_slice(&p[off..off + N]);
                return b;
            }
        }
        self.load_slow(a)
    }

    /// The word at `a` when [`Mem::u32`] reads it without its slow path (it lies in one mapped
    /// page), else `None`: for a caller that keeps the slow path out of its own body
    /// (`Engine::vcall`). Reads as [`Mem::load`]'s fast path does, with no other effect.
    #[inline(always)]
    pub(crate) fn u32_in_page(&self, a: u32) -> Option<u32> {
        let off = (a as usize) & (PAGE - 1);
        if off <= PAGE - 4 {
            if let Some(p) = self.pages[(a >> PAGE_BITS) as usize].as_deref() {
                let mut b = [0; 4];
                b.copy_from_slice(&p[off..off + 4]);
                return Some(u32::from_le_bytes(b));
            }
        }
        None
    }

    #[cold]
    #[inline(never)]
    fn load_slow<const N: usize>(&self, a: u32) -> [u8; N] {
        let mut b = [0; N];
        self.read(a, &mut b);
        b
    }

    /// The store diagnostics of [`Mem::store`], out of its hot path.
    #[cold]
    #[inline(never)]
    fn store_diagnostics(&mut self, a: u32, b: &[u8]) {
        if self.check_store {
            self.check_store_at(a, b.len() as u32);
        }
        if self.watched_page(a, b.len() as u32) {
            self.watch_note(a, b.len() as u32, Some(b), WatchKind::Write);
        }
    }

    /// Writes the `N` bytes `b` at `a`, as [`Mem::load`] reads them.
    #[inline(always)]
    fn store<const N: usize>(&mut self, a: u32, b: [u8; N]) {
        // One test for both diagnostics (the write watch, the block check's store mode), so a
        // normal run pays a single predictable branch per store.
        if self.store_diag {
            self.store_diagnostics(a, &b);
        }
        let off = (a as usize) & (PAGE - 1);
        if off <= PAGE - N {
            if let Some(p) = self.pages[(a >> PAGE_BITS) as usize].as_deref_mut() {
                p[off..off + N].copy_from_slice(&b);
                return;
            }
        }
        self.store_slow(a, b)
    }

    #[cold]
    #[inline(never)]
    fn store_slow<const N: usize>(&mut self, a: u32, b: [u8; N]) {
        if let Err(Fault(at)) = self.try_write_raw(a, &b) {
            panic!("write to unmapped memory at {at:08x}");
        }
    }

    #[inline]
    pub fn u8(&self, a: u32) -> u8 {
        self.load::<1>(a)[0]
    }
    #[inline]
    pub fn u16(&self, a: u32) -> u16 {
        u16::from_le_bytes(self.load(a))
    }
    #[inline]
    pub fn u32(&self, a: u32) -> u32 {
        u32::from_le_bytes(self.load(a))
    }
    #[inline]
    pub fn u64(&self, a: u32) -> u64 {
        u64::from_le_bytes(self.load(a))
    }
    #[inline]
    pub fn i8(&self, a: u32) -> i8 {
        self.u8(a) as i8
    }
    #[inline]
    pub fn i16(&self, a: u32) -> i16 {
        self.u16(a) as i16
    }
    #[inline]
    pub fn i32(&self, a: u32) -> i32 {
        self.u32(a) as i32
    }
    #[inline]
    pub fn f32(&self, a: u32) -> f32 {
        f32::from_bits(self.u32(a))
    }
    #[inline]
    pub fn f64(&self, a: u32) -> f64 {
        f64::from_bits(self.u64(a))
    }

    #[inline]
    pub fn set_u8(&mut self, a: u32, v: u8) {
        self.store(a, [v]);
    }
    #[inline]
    pub fn set_u16(&mut self, a: u32, v: u16) {
        self.store(a, v.to_le_bytes());
    }
    #[inline]
    pub fn set_u32(&mut self, a: u32, v: u32) {
        self.store(a, v.to_le_bytes());
    }
    #[inline]
    pub fn set_u64(&mut self, a: u32, v: u64) {
        self.store(a, v.to_le_bytes());
    }
    #[inline]
    pub fn set_i8(&mut self, a: u32, v: i8) {
        self.set_u8(a, v as u8);
    }
    #[inline]
    pub fn set_i16(&mut self, a: u32, v: i16) {
        self.set_u16(a, v as u16);
    }
    #[inline]
    pub fn set_i32(&mut self, a: u32, v: i32) {
        self.set_u32(a, v as u32);
    }
    #[inline]
    pub fn set_f32(&mut self, a: u32, v: f32) {
        self.set_u32(a, v.to_bits());
    }
    #[inline]
    pub fn set_f64(&mut self, a: u32, v: f64) {
        self.set_u64(a, v.to_bits());
    }

    /// NUL-terminated byte string at `a` (without the NUL).
    pub fn cstr(&self, a: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut p = a;
        loop {
            let c = self.u8(p);
            if c == 0 {
                return out;
            }
            out.push(c);
            p = p.wrapping_add(1);
        }
    }

    /// Writes `s` and a NUL at `a`.
    pub fn set_cstr(&mut self, a: u32, s: &[u8]) {
        self.write(a, s);
        self.set_u8(a.wrapping_add(s.len() as u32), 0);
    }

    /// The highest address the heap has reached: no block was ever handed out at or above it.
    /// Tools that map their own memory inside the heap range above it (the oracle check's
    /// buffer arena at 0x30000000) check that the heap has not reached them. The heap's next
    /// fresh address can be lower: it comes down again when the blocks at the top are freed
    /// (`mem/heap.rs`), and this stays at the highest it has been.
    pub fn heap_top(&self) -> u32 {
        self.heap.peak()
    }

    /// Allocates `size` bytes (8-byte aligned, zero-filled) on the heap. A freed block of the
    /// same rounded size is reused first, newest first; else part of a bigger free block, else
    /// fresh memory (`mem/heap.rs` has the policy).
    pub fn alloc(&mut self, size: u32) -> u32 {
        let asked = size;
        let guard = if self.check.is_some() { GUARD } else { 0 };
        let size = size.max(1).div_ceil(8) * 8 + guard;
        let addr = match self.heap.take(size) {
            Some(a) => a,
            None => self.heap_exhausted(size),
        };
        self.map(addr, size);
        if self.watched_page(addr, size) {
            self.watch_note(addr, size, None, WatchKind::Alloc);
        }
        self.zero_raw(addr, size);
        self.live.insert(addr, size);
        if guard != 0 {
            self.check_guard_block(addr, asked, size);
        }
        addr
    }

    /// Frees a block from [`Mem::alloc`]. Freeing 0 does nothing (as the
    /// game's `Deallocate`); freeing anything else that is not a live block
    /// panics.
    pub fn free(&mut self, addr: u32) {
        if addr == 0 {
            return;
        }
        let size = self
            .live
            .remove(&addr)
            .unwrap_or_else(|| panic!("free of {addr:08x}, which is not a live heap block"));
        // The watch sees the bytes the block held, before the freed check fills it.
        if self.watched_page(addr, size) {
            self.watch_note(addr, size, None, WatchKind::Free);
        }
        let mut released = None;
        if let Some(c) = self.check.as_deref() {
            let info = c.blocks.get(&addr).copied();
            self.check_free_block(addr);
            released = self.quarantine_block(addr, size, info);
            if released.is_none() {
                if let Some(c) = self.check.as_deref_mut() {
                    c.alloc_paths.remove(&addr);
                    c.pending_path.clear();
                }
            }
        }
        match released {
            // Blocks leaving the freed-block check's quarantine go straight to the bins, where
            // they merge with their free neighbours.
            Some(blocks) => {
                for (a, s) in blocks {
                    self.heap.release(a, s);
                }
            }
            None => self.heap.give(addr, size),
        }
    }

    /// Stops with the heap's state: live blocks and bytes, free bytes and blocks with the
    /// largest free block, the bytes waiting in the quick stacks and in the block check's
    /// quarantine, and the sizes holding the most live and the most free bytes.
    #[cold]
    #[inline(never)]
    fn heap_exhausted(&self, size: u32) -> ! {
        fn most(by_size: HashMap<u32, (u64, u64)>) -> String {
            let mut top: Vec<_> = by_size.into_iter().collect();
            top.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(a.0.cmp(&b.0)));
            top.iter()
                .take(8)
                .map(|(s, (n, b))| format!("{n}x{s}={}MB", b >> 20))
                .collect::<Vec<_>>()
                .join(" ")
        }
        let tally = |sizes: &mut dyn Iterator<Item = u32>| {
            let mut by_size: HashMap<u32, (u64, u64)> = HashMap::new();
            for s in sizes {
                let e = by_size.entry(s).or_default();
                e.0 += 1;
                e.1 += u64::from(s);
            }
            by_size
        };
        let live: u64 = self.live.values().map(|&s| u64::from(s)).sum();
        let quarantine = self.check.as_deref().map_or(0, |c| c.quarantine_bytes);
        let h = &self.heap;
        panic!(
            "heap exhausted allocating {size} bytes (live {} blocks {}MB, free {}MB in {} blocks, \
             the largest {} bytes, quick stacks {}MB in {} blocks, quarantine {}MB, top {:08x}; \
             most live {}; most free {})",
            self.live.len(),
            live >> 20,
            h.free_bytes() >> 20,
            h.free_blocks(),
            h.largest_free(),
            h.quick_bytes() >> 20,
            h.quick_blocks(),
            quarantine >> 20,
            h.top(),
            most(tally(&mut self.live.values().copied())),
            most(tally(&mut h.free_sizes()))
        )
    }

    /// Size of a live heap block.
    pub fn block_size(&self, addr: u32) -> Option<u32> {
        if let Some(&(asked, _)) = self.check.as_deref().and_then(|c| c.blocks.get(&addr)) {
            return Some(asked.max(1).div_ceil(8) * 8);
        }
        self.live.get(&addr).copied()
    }

    /// Checks the heap's bookkeeping against the live blocks and those the freed-block check
    /// holds back (see `heap::Heap::check`).
    #[cfg(test)]
    fn check_heap(&self) {
        let held = self
            .check
            .as_deref()
            .and_then(|c| c.quarantine.as_ref())
            .into_iter()
            .flatten()
            .copied();
        self.heap
            .check(self.live.iter().map(|(&a, &s)| (a, s)).chain(held));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A word written just past a guarded block is found when the block is freed, with the
    /// functions that allocated and freed it; writes inside it are not.
    #[test]
    fn the_block_check_finds_a_write_past_the_end_at_free() {
        let mut m = Mem::new();
        m.enable_block_check(false, false);
        m.set_check_fn(0x0055_3bb0);
        let a = m.alloc(0x20);
        assert_eq!(m.block_size(a), Some(0x20));
        m.set_u32(a + 0x1c, 7);
        m.free(a);
        assert!(m.take_block_reports().is_empty());
        let b = m.alloc(0x20);
        m.set_u32(b + 0x20, 0);
        m.set_check_fn(0x0040_1000);
        m.free(b);
        let r = m.take_block_reports();
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].contains("+0x20..+0x24 changed"), "{}", r[0]);
        assert!(r[0].contains("allocated while 00553bb0 ran"), "{}", r[0]);
        assert!(r[0].contains("while 00401000 ran"), "{}", r[0]);
    }

    /// With `store`, the store into the guard is reported at once, naming the writer; the
    /// slack between the size asked and the 8-byte rounding is guarded too.
    #[test]
    fn the_block_check_names_the_writer_with_store() {
        let mut m = Mem::new();
        m.enable_block_check(true, false);
        let a = m.alloc(0x1c);
        m.set_check_fn(0x004e_75d0);
        m.set_u32(a + 0x18, 1);
        assert!(m.take_block_reports().is_empty());
        m.set_u32(a + 0x1c, 2);
        let r = m.take_block_reports();
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].starts_with("004e75d0 wrote 4 byte(s)"), "{}", r[0]);
        assert!(r[0].contains("+0x1c into block"), "{}", r[0]);
        // A write well outside any block is not a finding.
        let c = m.alloc(0x100);
        m.write(c, &[1; 0x100]);
        assert!(m.take_block_reports().is_empty());
    }

    /// With the check on, `stop` panics at the finding.
    #[test]
    #[should_panic(expected = "NV_BLOCK_CHECK")]
    fn the_block_check_stops_when_asked() {
        let mut m = Mem::new();
        m.enable_block_check(true, true);
        let a = m.alloc(8);
        m.set_u8(a + 8, 1);
    }

    /// With the freed check, a store that runs past a live block's end and on into a freed
    /// neighbour is named for the live block's own guard, which it reaches first.
    #[test]
    fn an_overrun_into_a_freed_neighbour_names_the_overrun_block() {
        let mut m = Mem::new();
        m.enable_freed_check(false);
        let live = m.alloc(0x20);
        let neighbour = m.alloc(0x20);
        assert_eq!(neighbour, live + 0x20 + GUARD);
        m.free(neighbour);
        m.write(live, &[1; 0x34]);
        let reports = m.take_block_reports();
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            reports[0].contains(&format!("+0x20 into block {live:08x}"))
                && reports[0].contains("past its end"),
            "{}",
            reports[0]
        );
    }

    /// The write watch records changing stores, frees and re-allocation of a watched range
    /// and nothing outside it; a frees-only range sees only its free.
    #[test]
    fn watch_records_changes_frees_and_reuse() {
        let mut m = Mem::new();
        let a = m.alloc(0x18);
        let b = m.alloc(0x10);
        assert!(!m.has_watch_hits());
        m.watch(a, 0x14, false);
        m.watch(b, 0x10, true);
        m.set_u32(a + 4, 0x1234);
        m.set_u32(a + 4, 0x1234); // unchanged: no event
        m.set_u32(a + 0x14, 9); // past the range
        m.set_u32(b, 7); // frees only
        let hits = m.take_watch_hits();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            (
                hits[0].start,
                hits[0].addr,
                hits[0].len,
                hits[0].old,
                hits[0].new,
                hits[0].kind
            ),
            (a, a + 4, 4, 0, 0x1234, WatchKind::Write)
        );
        m.write(a, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(m.take_watch_hits()[0].new, 0x0807_0605_0403_0201);
        m.free(b);
        m.free(a);
        let kinds: Vec<(u32, WatchKind)> = m
            .take_watch_hits()
            .iter()
            .map(|h| (h.start, h.kind))
            .collect();
        assert_eq!(kinds, vec![(b, WatchKind::Free), (a, WatchKind::Free)]);
        assert_eq!(m.alloc(0x18), a);
        let hits = m.take_watch_hits();
        assert_eq!(
            (hits[0].kind, hits[0].old),
            (WatchKind::Alloc, 0x0807_0605_0403_0201)
        );
        m.unwatch(a);
        m.unwatch(b);
        m.set_u32(a, 5);
        assert!(!m.has_watch_hits());
    }

    #[test]
    fn unmapped_reads_fault() {
        let m = Mem::new();
        let mut b = [0; 4];
        assert_eq!(m.try_read(0x0118_3000, &mut b), Err(Fault(0x0118_3000)));
    }

    #[test]
    fn values_cross_pages() {
        let mut m = Mem::new();
        m.map(0x1000, 0x2000);
        m.set_u32(0x1ffe, 0xdead_beef);
        assert_eq!(m.u32(0x1ffe), 0xdead_beef);
        m.set_f64(0x1ffc, 1.25);
        assert_eq!(m.f64(0x1ffc), 1.25);
    }

    /// The fixed-size accessors read and write every width at every offset of a page, and at
    /// the page's end (where they go through the general path) as bytes would.
    #[test]
    fn scalar_accessors_match_byte_reads_at_every_offset() {
        let mut m = Mem::new();
        m.map(0x1000, 0x2000);
        for (i, a) in (0x1ff0..0x2010u32).enumerate() {
            m.set_u8(a, i as u8 ^ 0x5a);
        }
        for a in 0x1ff0..0x2008u32 {
            let b = m.bytes(a, 8);
            assert_eq!(m.u8(a), b[0]);
            assert_eq!(m.u16(a), u16::from_le_bytes(b[..2].try_into().unwrap()));
            assert_eq!(m.u32(a), u32::from_le_bytes(b[..4].try_into().unwrap()));
            assert_eq!(m.u64(a), u64::from_le_bytes(b.try_into().unwrap()));
        }
        for a in 0x1ff8..0x2001u32 {
            m.set_u64(a, 0x0102_0304_0506_0708);
            assert_eq!(m.bytes(a, 8), [8, 7, 6, 5, 4, 3, 2, 1]);
            m.set_u16(a, 0xa1b2);
            assert_eq!(m.bytes(a, 2), [0xb2, 0xa1]);
        }
    }

    /// A read of an unmapped page faults with the address, from the fast path and from a
    /// read that crosses into the unmapped page (with the first unmapped byte's address).
    #[test]
    #[should_panic(expected = "read of unmapped memory at 00005000")]
    fn unmapped_scalar_reads_name_the_address() {
        Mem::new().u32(0x5000);
    }

    #[test]
    #[should_panic(expected = "read of unmapped memory at 00002000")]
    fn reads_into_an_unmapped_page_name_its_first_byte() {
        let mut m = Mem::new();
        m.map(0x1000, 0x1000);
        m.u32(0x1ffe);
    }

    /// A write that crosses into an unmapped page writes the mapped part, then faults there.
    #[test]
    fn writes_into_an_unmapped_page_fault_after_the_mapped_part() {
        let mut m = Mem::new();
        m.map(0x1000, 0x1000);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            m.set_u32(0x1ffe, 0xaabb_ccdd)
        }));
        assert!(r.is_err());
        assert_eq!(m.bytes(0x1ffe, 2), [0xdd, 0xcc]);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut m = Mem::new();
            m.set_u8(0x7000, 1)
        }));
        let why = r.unwrap_err();
        assert_eq!(
            why.downcast_ref::<String>().map(String::as_str),
            Some("write to unmapped memory at 00007000")
        );
    }

    /// `zero` clears a range across pages.
    #[test]
    fn zero_clears_across_pages() {
        let mut m = Mem::new();
        m.map(0x1000, 0x2000);
        m.write(0x1ff0, &[0xff; 0x20]);
        m.zero(0x1ff4, 0x18);
        let b = m.bytes(0x1ff0, 0x20);
        assert_eq!(&b[..4], &[0xff; 4]);
        assert!(b[4..0x1c].iter().all(|&x| x == 0));
        assert_eq!(&b[0x1c..], &[0xff; 4]);
    }

    #[test]
    fn heap_reuses_freed_blocks() {
        let mut m = Mem::new();
        let a = m.alloc(12);
        let b = m.alloc(12);
        assert_eq!(b, a + 16);
        m.set_u32(a, 7);
        m.free(a);
        let c = m.alloc(16);
        assert_eq!(c, a);
        assert_eq!(m.u32(c), 0, "reused blocks are zeroed");
        m.free(0);
    }

    #[test]
    fn strings() {
        let mut m = Mem::new();
        let a = m.alloc(16);
        m.set_cstr(a, b"Goodsprings");
        assert_eq!(m.cstr(a), b"Goodsprings");
    }

    /// Freeing 0 does nothing: no block, no change to the heap.
    #[test]
    fn freeing_null_does_nothing() {
        let mut m = Mem::new();
        let a = m.alloc(0x40);
        m.free(0);
        assert_eq!(m.live.len(), 1);
        assert_eq!(m.heap.top(), a + 0x40);
        m.check_heap();
    }

    /// Several freed blocks of one size come back newest first, as before the bins, for sizes
    /// the quick stacks hold and bigger ones (the bins) alike.
    #[test]
    fn same_size_blocks_come_back_newest_first() {
        for size in [0x18, 0x1f8, 0x1000, 0x2000] {
            let mut m = Mem::new();
            let blocks: Vec<u32> = (0..5).map(|_| m.alloc(size)).collect();
            m.free(blocks[1]);
            m.free(blocks[3]);
            assert_eq!(m.alloc(size), blocks[3]);
            assert_eq!(m.alloc(size), blocks[1]);
            assert_eq!(m.alloc(size), blocks[4] + size);
            m.check_heap();
        }
    }

    /// A big freed block serves smaller requests from its start, zero-filled, and the rest of it
    /// serves the next.
    #[test]
    fn a_big_freed_block_serves_smaller_requests() {
        let mut m = Mem::new();
        let big = m.alloc(4 << 20);
        let after = m.alloc(0x40);
        m.write(big, &[0xab; 0x100]);
        m.free(big);
        let a = m.alloc(1 << 20);
        assert_eq!(a, big);
        assert_eq!(m.bytes(a, 0x100), vec![0; 0x100]);
        assert_eq!(m.alloc(0x300), big + (1 << 20));
        assert_eq!(m.alloc((3 << 20) - 0x300), big + (1 << 20) + 0x300);
        assert_eq!(m.heap_top(), after + 0x40);
        m.check_heap();
    }

    /// Freed blocks at the top lower the heap's next fresh address (merging on the way down);
    /// `heap_top` keeps the highest it reached.
    #[test]
    fn freed_blocks_at_the_top_lower_it() {
        let mut m = Mem::new();
        // Bigger than the quick stacks take, so a freed block goes straight to the bins.
        let a = m.alloc(0x2000);
        let b = m.alloc(0x2000);
        let c = m.alloc(0x2000);
        let peak = m.heap_top();
        m.free(a);
        m.free(c);
        assert_eq!(m.heap.top(), c);
        m.free(b);
        assert_eq!(m.heap.top(), HEAP_BASE);
        assert_eq!(m.heap_top(), peak);
        m.check_heap();
        assert_eq!(m.alloc(0x1000), a);
    }

    /// Blocks leaving the freed-block check's quarantine merge with free neighbours in the bins,
    /// even small ones.
    #[test]
    fn blocks_leaving_the_quarantine_merge() {
        let mut m = Mem::new();
        m.enable_freed_check(false);
        m.check.as_deref_mut().unwrap().quarantine_limit = 0x90;
        let blocks: Vec<u32> = (0..3).map(|_| m.alloc(0x40)).collect();
        let keep = m.alloc(8);
        assert_eq!(blocks[1], blocks[0] + 0x40 + GUARD);
        m.free(blocks[0]);
        assert_eq!(m.heap.free_blocks(), 0, "held back");
        m.free(blocks[1]);
        assert_eq!(m.heap.free_block_at(blocks[0]), Some((blocks[0], 0x50)));
        m.free(blocks[2]);
        assert_eq!(m.heap.free_block_at(blocks[0]), Some((blocks[0], 0xa0)));
        assert_eq!(m.heap.quick_blocks(), 0);
        m.check_heap();
        assert_eq!(m.alloc(0x90), blocks[0]);
        m.free(keep);
        m.check_heap();
        assert!(m.take_block_reports().is_empty());
    }

    /// A full heap stops with its state: live and free bytes, the largest free block, the sizes
    /// holding the most.
    #[test]
    fn heap_exhausted_reports_the_heap() {
        let mut m = Mem::new();
        let _a = m.alloc(0x100);
        let b = m.alloc(1 << 20);
        let _c = m.alloc(0x100);
        m.free(b);
        // The rest of the heap as one block, taken without mapping its pages.
        let rest = HEAP_END - m.heap.top();
        let filler = m.heap.take(rest).unwrap();
        m.live.insert(filler, rest);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.alloc(2 << 20)));
        let why = r.unwrap_err();
        let text = why.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(
            text.starts_with("heap exhausted allocating 2097152 bytes (live 3 blocks "),
            "{text}"
        );
        assert!(
            text.contains("free 1MB in 1 blocks, the largest 1048576 bytes"),
            "{text}"
        );
        assert!(text.contains(&format!("most live 1x{rest}=")), "{text}");
        assert!(text.contains("2x256=0MB"), "{text}");
        assert!(text.contains("most free 1x1048576=1MB)"), "{text}");
    }

    /// A nearly full, fragmented heap serves a request from a free block that fits even when
    /// eight newer, smaller blocks of the same bin sit in front of it (0x2000, 0x2080 and 0x20f8
    /// share a bin).
    #[test]
    fn a_full_heap_serves_a_request_from_deep_in_its_bin() {
        let mut m = Mem::new();
        let fitting = m.alloc(0x20f8);
        m.alloc(8);
        let smaller: Vec<u32> = (0..8)
            .map(|_| {
                let a = m.alloc(0x2000);
                m.alloc(8);
                a
            })
            .collect();
        let rest = HEAP_END - m.heap.top();
        let filler = m.heap.take(rest).unwrap();
        m.live.insert(filler, rest);
        m.free(fitting);
        for a in smaller {
            m.free(a);
        }
        m.check_heap();
        assert_eq!(m.alloc(0x2080), fitting);
        assert_eq!(
            m.heap.free_block_at(fitting + 0x2080),
            Some((fitting + 0x2080, 0x78))
        );
        m.check_heap();
    }

    /// A small pseudo-random generator (xorshift64) for the tests below.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 32) as u32
        }
        fn below(&mut self, n: u32) -> u32 {
            self.next() % n
        }
    }

    /// Random allocations and frees of mixed sizes, checked against a shadow list of the live
    /// blocks: every block is 8-byte aligned, inside `[HEAP_BASE, top)`, disjoint from every
    /// other live block and zero-filled when handed out, and keeps what was written into it;
    /// the heap's own bookkeeping is checked as it goes. With `freed_check`, the freed-block
    /// check holds blocks back and lets them go into the bins.
    fn stress(freed_check: bool, seed: u64) {
        let mut m = Mem::new();
        if freed_check {
            m.enable_freed_check(false);
            m.check.as_deref_mut().unwrap().quarantine_limit = 0x4_0000;
        }
        let big = if freed_check { 0x1_0000 } else { 0x10_0000 };
        let mut rng = Rng(seed);
        let mut shadow: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        let mut order: Vec<u32> = Vec::new();
        for step in 0..20_000u32 {
            let grow = order.len() < 50 || (order.len() < 3000 && rng.below(100) < 55);
            if grow {
                let asked = match rng.below(100) {
                    0..=4 => 0,
                    5..=74 => rng.below(300),
                    75..=97 => 300 + rng.below(6000),
                    _ => 0x1_0000 + rng.below(big),
                };
                let a = m.alloc(asked);
                let size = m.live[&a];
                assert_eq!(a % 8, 0);
                assert!(
                    a >= HEAP_BASE && a + size <= m.heap.top(),
                    "{a:08x}+{size:x}"
                );
                if let Some((&p, &(ps, _))) = shadow.range(..a).next_back() {
                    assert!(p + ps <= a, "{a:08x} overlaps {p:08x}+{ps:x}");
                }
                if let Some((&n, _)) = shadow.range(a..).next() {
                    assert!(a + size <= n, "{a:08x}+{size:x} overlaps {n:08x}");
                }
                assert!(m.bytes(a, asked.min(64)).iter().all(|&x| x == 0));
                let words = asked / 4;
                if words > 0 {
                    m.set_u32(a, step);
                    m.set_u32(a + (words - 1) * 4, step);
                }
                shadow.insert(a, (size, words));
                order.push(a);
            } else {
                let a = order.swap_remove(rng.below(order.len() as u32) as usize);
                let (_, words) = shadow.remove(&a).unwrap();
                if words > 0 {
                    let tag = m.u32(a);
                    assert_eq!(
                        m.u32(a + (words - 1) * 4),
                        tag,
                        "block {a:08x} was overwritten"
                    );
                }
                m.free(a);
            }
            if step % 1000 == 0 {
                m.check_heap();
            }
        }
        m.check_heap();
        for a in order {
            m.free(a);
        }
        m.check_heap();
        assert!(m.take_block_reports().is_empty());
    }

    #[test]
    fn random_allocs_and_frees_never_overlap() {
        stress(false, 0x2545_f491_4f6c_dd1d);
    }

    #[test]
    fn random_allocs_and_frees_never_overlap_with_the_freed_check() {
        stress(true, 0x9e37_79b9_7f4a_7c15);
    }

    /// The allocator before the bins, kept for the comparisons below: fresh blocks from a bump
    /// pointer, and a freed block reused only by a later request of exactly its size, newest
    /// first (indexed by size / 8 up to 4096 bytes, a map above).
    struct OldHeap {
        brk: u32,
        small_free: Vec<Vec<u32>>,
        free: crate::engine::AddrMap<u32, Vec<u32>>,
    }

    impl OldHeap {
        fn new() -> Self {
            OldHeap {
                brk: HEAP_BASE,
                small_free: vec![Vec::new(); 4096 / 8 + 1],
                free: Default::default(),
            }
        }
        #[inline]
        fn list(&mut self, size: u32) -> &mut Vec<u32> {
            if size <= 4096 {
                &mut self.small_free[(size / 8) as usize]
            } else {
                self.free.entry(size).or_default()
            }
        }
        #[inline]
        fn take(&mut self, size: u32) -> u32 {
            if let Some(a) = self.list(size).pop() {
                return a;
            }
            let a = self.brk;
            assert!(
                (a as u64 + size as u64) <= HEAP_END as u64,
                "old heap exhausted"
            );
            self.brk += size;
            a
        }
    }

    /// One step of a replayed pattern: allocate object `id` with `size` bytes, or free it.
    enum Op {
        Alloc(usize, u32),
        Free(usize),
    }

    /// Cells loaded and unloaded as the open world does: each round allocates a cell of 300
    /// objects of varied sizes (70% up to 256 bytes, 25% 1-16 KB, 5% 16-256 KB, any multiple of
    /// 8), unloads the oldest when more than four are loaded, and replaces 200 random objects of
    /// the loaded cells. Answers the steps, each object's size, the most bytes live at once and
    /// the step where the second half of the rounds starts.
    fn cell_churn(rounds: usize) -> (Vec<Op>, Vec<u32>, u64, usize) {
        fn size(rng: &mut Rng) -> u32 {
            match rng.below(100) {
                0..=69 => 8 + rng.below(32) * 8,
                70..=94 => 1024 + rng.below(1920) * 8,
                _ => 16384 + rng.below(30720) * 8,
            }
        }
        let mut rng = Rng(0x0123_4567_89ab_cdef);
        let mut ops = Vec::new();
        let mut sizes: Vec<u32> = Vec::new();
        let mut cells: std::collections::VecDeque<Vec<usize>> = Default::default();
        let (mut live, mut most) = (0u64, 0u64);
        let mut half = 0;
        for round in 0..rounds {
            if round == rounds / 2 {
                half = ops.len();
            }
            let mut cell = Vec::new();
            for _ in 0..300 {
                let s = size(&mut rng);
                ops.push(Op::Alloc(sizes.len(), s));
                cell.push(sizes.len());
                sizes.push(s);
                live += u64::from(s);
                most = most.max(live);
            }
            cells.push_back(cell);
            if cells.len() > 4 {
                for id in cells.pop_front().unwrap() {
                    ops.push(Op::Free(id));
                    live -= u64::from(sizes[id]);
                }
            }
            for _ in 0..200 {
                let c = rng.below(cells.len() as u32) as usize;
                let k = rng.below(cells[c].len() as u32) as usize;
                let old = cells[c][k];
                ops.push(Op::Free(old));
                live -= u64::from(sizes[old]);
                let s = size(&mut rng);
                ops.push(Op::Alloc(sizes.len(), s));
                cells[c][k] = sizes.len();
                sizes.push(s);
                live += u64::from(s);
                most = most.max(live);
            }
        }
        (ops, sizes, most, half)
    }

    /// The fragmentation the open world showed ("heap exhausted" with 901 MB on the free lists):
    /// with sizes that seldom repeat exactly, the old allocator's top keeps climbing with every
    /// cell loaded, while the heap's stays within a small multiple of the bytes live at once and
    /// stops climbing.
    #[test]
    fn churn_of_varied_sizes_keeps_the_heap_bounded() {
        let (ops, sizes, most, half) = cell_churn(60);
        let mut m = Mem::new();
        let mut addrs = vec![0u32; sizes.len()];
        let mut at_half = 0;
        for (i, op) in ops.iter().enumerate() {
            if i == half {
                at_half = m.heap_top() - HEAP_BASE;
            }
            match *op {
                Op::Alloc(id, s) => addrs[id] = m.alloc(s),
                Op::Free(id) => m.free(addrs[id]),
            }
        }
        m.check_heap();
        let new = m.heap_top() - HEAP_BASE;
        let mut old = OldHeap::new();
        let mut old_half = 0;
        for (i, op) in ops.iter().enumerate() {
            if i == half {
                old_half = old.brk - HEAP_BASE;
            }
            match *op {
                Op::Alloc(id, s) => addrs[id] = old.take(s),
                Op::Free(id) => old.list(sizes[id]).push(addrs[id]),
            }
        }
        let old_top = old.brk - HEAP_BASE;
        let mb = |b: u32| f64::from(b) / f64::from(1 << 20);
        eprintln!(
            "most live {:.1} MB; heap top {:.1} MB (half way {:.1}); old allocator {:.1} MB (half \
             way {:.1})",
            most as f64 / f64::from(1 << 20),
            mb(new),
            mb(at_half),
            mb(old_top),
            mb(old_half)
        );
        assert!(
            u64::from(new) <= most * 3 / 2 + heap::QUICK_BYTES,
            "the heap reached {new:#x} bytes for {most:#x} live"
        );
        assert!(
            u64::from(new) <= u64::from(at_half) * 5 / 4 + heap::QUICK_BYTES,
            "the heap kept climbing: {at_half:#x} half way, {new:#x} at the end"
        );
        // The pattern is one the old allocator could not hold: it climbs without bound.
        assert!(u64::from(old_top) > most * 5, "{old_top:#x}");
        assert!(old_top > old_half * 3 / 2, "{old_half:#x} -> {old_top:#x}");
    }

    /// What the speed comparison runs on.
    trait Allocator {
        fn alloc(&mut self, size: u32) -> u32;
        fn free(&mut self, addr: u32);
        fn top(&self) -> u32;
    }

    impl Allocator for Mem {
        #[inline]
        fn alloc(&mut self, size: u32) -> u32 {
            Mem::alloc(self, size)
        }
        #[inline]
        fn free(&mut self, addr: u32) {
            Mem::free(self, addr)
        }
        fn top(&self) -> u32 {
            self.heap_top()
        }
    }

    /// [`Mem::alloc`] and [`Mem::free`] as they were with the old allocator (the same steps
    /// around it: rounding, mapping, the watch test, zero-fill, the live map), on a `Mem` used
    /// for its pages.
    struct OldMem {
        mem: Mem,
        heap: OldHeap,
        live: crate::engine::AddrMap<u32, u32>,
    }

    impl Allocator for OldMem {
        #[inline]
        fn alloc(&mut self, size: u32) -> u32 {
            let guard = if self.mem.check.is_some() { GUARD } else { 0 };
            let size = size.max(1).div_ceil(8) * 8 + guard;
            let addr = self.heap.take(size);
            self.mem.map(addr, size);
            if self.mem.watched_page(addr, size) {
                self.mem.watch_note(addr, size, None, WatchKind::Alloc);
            }
            self.mem.zero_raw(addr, size);
            self.live.insert(addr, size);
            addr
        }
        #[inline]
        fn free(&mut self, addr: u32) {
            if addr == 0 {
                return;
            }
            let size = self.live.remove(&addr).expect("live block");
            if self.mem.watched_page(addr, size) {
                self.mem.watch_note(addr, size, None, WatchKind::Free);
            }
            assert!(self.mem.check.is_none());
            self.heap.list(size).push(addr);
        }
        fn top(&self) -> u32 {
            self.heap.brk
        }
    }

    /// `steps` frees of a random one of `slots` live blocks, each followed by an allocation of a
    /// size from `size`, after filling the slots; answers ns per free and allocation.
    fn churn_ns(
        h: &mut impl Allocator,
        slots: usize,
        steps: u32,
        size: fn(&mut Rng) -> u32,
    ) -> f64 {
        let mut rng = Rng(0x5851_f42d_4c95_7f2d);
        let mut live: Vec<u32> = (0..slots).map(|_| h.alloc(size(&mut rng))).collect();
        let t = std::time::Instant::now();
        for _ in 0..steps {
            let i = rng.below(slots as u32) as usize;
            h.free(live[i]);
            live[i] = h.alloc(size(&mut rng));
        }
        let ns = t.elapsed().as_nanos() as f64 / f64::from(steps);
        for a in live {
            h.free(a);
        }
        ns
    }

    /// Not a check: ns per free and allocation of the heap against the allocator before the
    /// bins, the best of three runs each, on small blocks alone, on blocks of 264 bytes to 4 KB
    /// alone (every size, and five sizes) and on a game-like mix (`cargo test --release -p
    /// engine-core --lib heap_speed -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn heap_speed() {
        fn small(rng: &mut Rng) -> u32 {
            8 * (1 + rng.below(32).min(rng.below(32)))
        }
        // Every multiple of 8 from 264 to 4096.
        fn mid(rng: &mut Rng) -> u32 {
            264 + rng.below(480) * 8
        }
        fn five(rng: &mut Rng) -> u32 {
            [264, 512, 1024, 2048, 4096][rng.below(5) as usize]
        }
        // 89% up to 256 bytes, 10% 264 bytes to 4 KB, 1% 1-64 KB (64 sizes of each), 0.005%
        // multi-MB (8454160 is one of the open world's).
        fn game(rng: &mut Rng) -> u32 {
            match rng.below(100_000) {
                0..=88_999 => small(rng),
                89_000..=98_999 => 264 + rng.below(64) * 60,
                99_000..=99_994 => 1024 + rng.below(64) * 1008,
                _ => [1 << 20, (2 << 20) + 16, 4 << 20, 8_454_160][rng.below(4) as usize],
            }
        }
        let mixes: [(&str, fn(&mut Rng) -> u32); 4] = [
            ("small 8-256", small),
            ("mid 264-4096", mid),
            ("five sizes 264-4096", five),
            ("game mix", game),
        ];
        for (name, size) in mixes {
            let (mut old, mut new, mut bins) = (f64::MAX, f64::MAX, f64::MAX);
            let (mut old_top, mut new_top) = (0, 0);
            for _ in 0..3 {
                let mut o = OldMem {
                    mem: Mem::new(),
                    heap: OldHeap::new(),
                    live: Default::default(),
                };
                old = old.min(churn_ns(&mut o, 50_000, 5_000_000, size));
                old_top = o.top() - HEAP_BASE;
                drop(o);
                let mut m = Mem::new();
                new = new.min(churn_ns(&mut m, 50_000, 5_000_000, size));
                new_top = m.top() - HEAP_BASE;
                drop(m);
                let mut m = Mem::new();
                m.heap = heap::Heap::new().without_quick();
                bins = bins.min(churn_ns(&mut m, 50_000, 5_000_000, size));
            }
            eprintln!(
                "heap speed, {name}: old {old:.1} ns, new {new:.1} ns ({:+.0}%), bins alone \
                 {bins:.1} ns ({:+.0}%) per free and allocation; top old {:.1} MB, new {:.1} MB",
                (new / old - 1.0) * 100.0,
                (bins / old - 1.0) * 100.0,
                f64::from(old_top) / f64::from(1 << 20),
                f64::from(new_top) / f64::from(1 << 20)
            );
        }
    }
}
