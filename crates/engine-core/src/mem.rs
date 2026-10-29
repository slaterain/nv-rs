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
//!   `units/platform.rs`).

use std::collections::{BTreeMap, HashMap};

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
    pages: Vec<Option<Box<[u8; PAGE]>>>,
    /// Heap: next fresh address, and freed blocks by size for reuse.
    brk: u32,
    live: BTreeMap<u32, u32>,
    free: BTreeMap<u32, Vec<u32>>,
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
        }));
        self.check_store = store;
        self.store_diag |= store;
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
        let end = a as u64 + len.max(1) as u64;
        let hit = c
            .guards
            .range(..(end.min(u32::MAX as u64) as u32))
            .next_back()
            .filter(|(_, &(gend, _))| gend > a)
            .map(|(&gstart, &(_, block))| (gstart, block));
        let Some((gstart, block)) = hit else {
            return;
        };
        let (asked, owner) = c.blocks.get(&block).copied().unwrap_or((0, 0));
        let writer = c.current_fn;
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
        Mem {
            pages,
            brk: HEAP_BASE,
            live: BTreeMap::new(),
            free: BTreeMap::new(),
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
            panic!("read of unmapped memory at {a:08x}");
        }
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

    /// The heap's current top: the next fresh address [`Mem::alloc`] hands
    /// out (reused freed blocks lie below it). Tools that map their own memory
    /// inside the heap range above it (the oracle check's buffer arena at
    /// 0x30000000) check that the heap has not reached them.
    pub fn heap_top(&self) -> u32 {
        self.brk
    }

    /// Allocates `size` bytes (8-byte aligned, zero-filled) on the heap.
    /// Freed blocks of the same rounded size are reused, newest first.
    pub fn alloc(&mut self, size: u32) -> u32 {
        let asked = size;
        let guard = if self.check.is_some() { GUARD } else { 0 };
        let size = size.max(1).div_ceil(8) * 8 + guard;
        let addr = match self.free.get_mut(&size).and_then(Vec::pop) {
            Some(a) => a,
            None => {
                let a = self.brk;
                assert!(
                    (a as u64 + size as u64) <= HEAP_END as u64,
                    "heap exhausted allocating {size} bytes"
                );
                self.brk += size;
                a
            }
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
        if self.check.is_some() {
            self.check_free_block(addr);
        }
        if self.watched_page(addr, size) {
            self.watch_note(addr, size, None, WatchKind::Free);
        }
        self.free.entry(size).or_default().push(addr);
    }

    /// Size of a live heap block.
    pub fn block_size(&self, addr: u32) -> Option<u32> {
        if let Some(&(asked, _)) = self.check.as_deref().and_then(|c| c.blocks.get(&addr)) {
            return Some(asked.max(1).div_ceil(8) * 8);
        }
        self.live.get(&addr).copied()
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
}
