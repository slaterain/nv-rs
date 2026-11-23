//! The drawing thread's value packet (step 2, piece 1: `measure/scope-1111/PLAN.md`): the values the world draw
//! reads that the rest of the frame may change, copied as the draw starts.
//!
//! The drawing thread will run the world draw (`00b6ba20`, `00b6b930`) beside the frame's tail: the
//! first-person and reflection draws, the interface, the present and the post-swap step, up to the next frame's
//! input. The tail moves objects (their world transforms and bounds), runs particles and changes lights and fog;
//! the draw must see them as they were when it started. So at the cut ([`super::cut`]) their bytes are copied
//! into this packet, object by object, a range of offsets each, and the draw reads them through it: [`word`] and
//! its kin answer the copy while the packet is open, and a helper that takes a pointer to a copied object gets
//! a guest-stack image of it ([`image`]). The packet closes at the join ([`end`]); with the draw still on main
//! (this step) that is the second half's end.
//!
//! The packet is the drawing thread's own (a thread-local): the drawing engine reads it without a lock, and
//! main's other draws, on main's thread, read guest memory. A read through it of an object range the cut did
//! not copy answers guest memory and is counted as a miss ([`Report::misses`]). The objects the cut copied are
//! held ([`hold`], `NiRefObject::IncRefCount`) until the join, so the tail cannot free one the draw still reads.
//!
//! **POISON** (`NV_DRAW_THREAD=poison`, a check, not for play): the guest bytes the packet holds are scribbled
//! while the draw runs ([`poison`]) and put back as it ends. A read that bypasses the packet then sees the
//! scribble, so the draw with POISON equals the draw with the switch off only when every read of a copied value
//! goes through the packet. A scribbled byte the draw wrote is left as written and counted
//! ([`Report::written`]): such a value is the draw's own state, not the packet's.
//!
//! **Own state** (step 2, piece 3, [`own`]): what the draw writes that main's tail writes too (the shader statics,
//! the render-state cache: [`super::statics`]) is taken as own blocks by address. The draw reads and writes them
//! in the packet ([`own_at`], [`set_own_at`], [`own_image`] for a helper handed a pointer); main keeps the guest
//! bytes, and its writes through the same setters while another thread holds the packet are noted. At the join
//! each byte takes main's value when main set it after the cut (noted, or changed), else the draw's: the device
//! model's rule ([`end`]). Between the halves ([`pause`]) the own state is main's too.
//!
//! Off by default (`NV_DRAW_THREAD` unset); off, the accessors are the guest reads they replace behind one
//! relaxed load.

use engine_core::ptr::{Field, Ptr, Scalar};
use engine_core::{typed_call, AddrMap, Engine};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::OnceLock;

/// What `NV_DRAW_THREAD` asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Unset: no packet.
    Off,
    /// `1` or `on`: the packet filled at the cut and read by the draw (still on main).
    On,
    /// `poison`: as `On`, with the packet's guest bytes scribbled while the draw runs (a check).
    Poison,
}

/// `NV_DRAW_THREAD`, read once; a thread's own mode instead while one is set ([`set_thread_mode`]), else the run-wide
/// mode when one is set ([`set_mode`]: the host's choice, so the game's `Fallout.ini` line turns the packet on too).
pub fn mode() -> Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    if let Some(mode) = THREAD_MODE.with(Cell::get) {
        return mode;
    }
    match SET_MODE.load(Relaxed) {
        1 => return Mode::Off,
        2 => return Mode::On,
        3 => return Mode::Poison,
        _ => {}
    }
    *MODE.get_or_init(|| match std::env::var("NV_DRAW_THREAD").as_deref() {
        Ok("1") | Ok("on") => Mode::On,
        Ok("poison") => Mode::Poison,
        _ => Mode::Off,
    })
}

thread_local! {
    static THREAD_MODE: Cell<Option<Mode>> = const { Cell::new(None) };
}

/// Sets this thread's mode in place of `NV_DRAW_THREAD` (`None`: the variable's again). For the checks, whose
/// engines each run on a thread of their own.
pub fn set_thread_mode(mode: Option<Mode>) {
    THREAD_MODE.with(|m| m.set(mode));
}

/// This thread's own mode ([`set_thread_mode`]), `None` when it has none.
pub fn thread_mode() -> Option<Mode> {
    THREAD_MODE.with(Cell::get)
}

/// The run-wide mode ([`set_mode`]): 0 none (the variable's), 1 off, 2 on, 3 POISON.
static SET_MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

// Export E5 of the drawing thread's seam.
/// Sets the mode for every thread of the process in place of `NV_DRAW_THREAD` (`None`: the variable's again); a
/// thread's own mode ([`set_thread_mode`]) still wins.
pub fn set_mode(mode: Option<Mode>) {
    let value = match mode {
        None => 0,
        Some(Mode::Off) => 1,
        Some(Mode::On) => 2,
        Some(Mode::Poison) => 3,
    };
    SET_MODE.store(value, Relaxed);
}

/// The byte POISON fills the packet's guest bytes with (0x6e: a word of them is a float of about 1.8e28 and an
/// address in no block). [`SCRIBBLE_OTHER`] is its complement, for a second run that flips every bit the first
/// left alone.
pub const SCRIBBLE: u8 = 0x6e;
/// [`SCRIBBLE`]'s complement.
pub const SCRIBBLE_OTHER: u8 = !SCRIBBLE;

// One copied range of an object (`Range`: its offsets, where its bytes start, the object's next range) and a block
// of the draw's own state (`OwnBlock`, [`own`], step 2, piece 3: the guest addresses `[lo, hi)`, held at `at` in the
// packet's own bytes, and at the same place in `own_cut`, their values at the cut, and `own_main`, main's): the
// engine's copied-state view's types, since the packet hands them to the drawing engine for each half ([`serve`]).
use engine_core::own_view::{OwnBlock, OwnView, Range, NONE};

struct Packet {
    /// While a half draws on an engine ([`serve`]): the own blocks and bytes and the copied ranges and bytes are on
    /// that engine (`Engine::own_view`), and this packet's fields for them are empty until [`unserve`].
    served: bool,
    /// The view's allocation kept between halves.
    spare: Option<Box<OwnView>>,
    /// The run that opened it (the kernel table's address): another run's engine on this thread reads guest
    /// memory.
    run: usize,
    /// Each object's first range.
    objects: AddrMap<u32, u32>,
    ranges: Vec<Range>,
    /// The ranges' objects, in the order they were taken (for [`poison`] and [`end`]).
    order: Vec<(u32, u32)>,
    bytes: Vec<u8>,
    /// POISON: the scribble and the guest bytes it covered (laid out as `bytes`).
    scribble: Option<u8>,
    saved: Vec<u8>,
    held: Vec<u32>,
    misses: Cell<u64>,
    reads: Cell<u64>,
    /// Between the world draw's two halves ([`pause`]): the accessors read guest memory and, with POISON, the
    /// guest bytes are back.
    paused: bool,
    /// POISON: scribbled bytes found written at a [`pause`].
    written: u64,
    /// The draw's own state: its blocks (by address, apart from each other), their bytes as the draw reads and
    /// writes them, as they were at the cut, and (POISON) main's values while the scribble covers guest memory.
    own: Vec<OwnBlock>,
    own_bytes: Vec<u8>,
    own_cut: Vec<u8>,
    own_main: Vec<u8>,
    /// The own words that are counted pointers ([`own_counted`]): the drawing side's holds move their objects' counts
    /// at the join, in the serial order ([`end`]).
    counted: Vec<u32>,
    /// The object [`Packet::find`] looked up last and its first range (`NONE`: none): the draw reads an object's
    /// fields one after another, so most look-ups skip the map.
    last_object: Cell<(u32, u32)>,
    /// The own block [`Packet::own_at`] found last, tried first.
    last_own: Cell<u32>,
    /// The own blocks' indices in the order of their addresses ([`Packet::own_at`]'s search).
    own_by_lo: Vec<u32>,
    /// The world draw's per-frame list words handed to the drawing side at the cut ([`take_lists`]), by guest
    /// address: guest memory holds zeros for them meanwhile, and the join puts them back ([`end`]).
    lists: AddrMap<u32, u32>,
}

impl Packet {
    /// Where the own bytes of `[addr, addr + len)` start, when one own block holds them all.
    #[inline(always)]
    fn own_at(&self, addr: u32, len: u32) -> Option<usize> {
        let holds =
            |b: &OwnBlock| addr >= b.lo && addr.checked_add(len).is_some_and(|end| end <= b.hi);
        let last = self.last_own.get() as usize;
        if let Some(b) = self.own.get(last) {
            if holds(b) {
                return Some((b.at + (addr - b.lo)) as usize);
            }
        }
        // The blocks are apart from each other, so the one that could hold `addr` is the last starting at or
        // before it, found by halving over their order by address.
        let k = self
            .own_by_lo
            .partition_point(|&i| self.own[i as usize].lo <= addr);
        let i = *self.own_by_lo.get(k.checked_sub(1)?)?;
        let b = &self.own[i as usize];
        if !holds(b) {
            return None;
        }
        self.last_own.set(i);
        Some((b.at + (addr - b.lo)) as usize)
    }

    /// Whether the accessors read this packet for `e`: its run, and not paused.
    #[inline(always)]
    fn serves(&self, e: &Engine) -> bool {
        self.run == run(e) && !self.paused
    }

    /// The range of `object` that holds `[offset, offset + len)`, if one does.
    #[inline(always)]
    fn find(&self, object: u32, offset: u32, len: u32) -> Option<Range> {
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

    /// Fills `out` with the bytes at `object + offset` as the packet holds them, from one range or several;
    /// false when one of them is in none.
    fn read_into(&self, object: u32, offset: u32, out: &mut [u8]) -> bool {
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

    /// [`Packet::read_into`], counted: a read, or a miss when it fails.
    fn read_or_miss(&self, object: u32, offset: u32, out: &mut [u8]) -> bool {
        let found = self.read_into(object, offset, out);
        let count = if found { &self.reads } else { &self.misses };
        count.set(count.get() + 1);
        if !found {
            missed(object, offset, self.scribble.is_some());
        }
        found
    }
}

/// How many misses [`missed`] prints, for the whole run.
const MISSES_SHOWN: u32 = 8;

/// A read through the packet that found no copy: the run's first few are printed (object and offset), so an
/// on run shows what the cut left out; with POISON, a stop (a check run's coverage signal: the draw read live
/// bytes the frame's tail may change).
#[cold]
#[inline(never)]
fn missed(object: u32, offset: u32, poison: bool) {
    static SHOWN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    assert!(
        !poison,
        "NV_DRAW_THREAD=poison: the draw read {object:#010x}+{offset:#x} through the value packet, which the cut did not take"
    );
    if SHOWN.fetch_add(1, Relaxed) < MISSES_SHOWN {
        eprintln!("nv-draw packet: a read the cut did not take: {object:#010x}+{offset:#x}");
    }
}

thread_local! {
    static PACKET: RefCell<Option<Packet>> = const { RefCell::new(None) };
    /// True whenever [`PACKET`] holds a packet on this thread (it may stay true a moment longer, never shorter):
    /// lets the read accessors of a thread with no packet (main while the drawing side draws) go straight to
    /// guest memory, without the packet's borrow and look-ups on every read ([`held`]).
    static HOLDS: Cell<bool> = const { Cell::new(false) };
    /// This thread's packet is paused between the world draw's halves ([`pause`]): what runs meanwhile (the VATS target's draw)
    /// reads and writes guest memory, whose cached device state is the cut's, not what the first half left ([`beside`]).
    static PAUSED: Cell<bool> = const { Cell::new(false) };
    /// The last write [`mark`] noted on this thread, with the packet generation it was noted in: the same
    /// write again in the same generation is not pushed twice.
    static LAST_MARK: Cell<(u64, usize, u32, u32)> = const { Cell::new((0, 0, 0, 0)) };
}

/// How many threads hold an open packet: the accessors' one check while none does.
static OPEN: AtomicUsize = AtomicUsize::new(0);

/// Counts the packets opened ([`begin`]), so a mark noted for an earlier packet is never taken as noted for a
/// later one.
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Whether this thread may hold an open packet: the read accessors' first test. False on every thread while no
/// packet is open, and on a thread that holds none (main while the drawing side draws), whose reads are guest
/// memory's either way.
#[inline(always)]
fn held() -> bool {
    OPEN.load(Relaxed) != 0 && HOLDS.with(Cell::get)
}

/// Whether this thread runs beside a packet another thread draws from: a packet is open and this thread holds none
/// (main while the drawing thread draws). Its cached device state (the render-state cache, the current shaders and
/// declaration) then holds the values at the cut, while serially it would hold what the draw left; a setter must not
/// skip its device call on such a value ([`main_must_send`]). One relaxed load while no packet is open.
#[inline(always)]
pub fn beside(e: &Engine) -> bool {
    OPEN.load(Relaxed) != 0
        && (!HOLDS.with(Cell::get) || PAUSED.with(Cell::get))
        && open_for(run(e))
}

/// The runs with an open packet, one slot each (a run opens one packet at a time; 0 is a free slot): what
/// [`beside`] asks after its cheap tests, so that one run's packet never makes another run's setters send (tests run
/// many engines at once; play has one).
static OPEN_RUNS: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];

/// Whether `run` has an open packet ([`OPEN_RUNS`]).
#[cold]
fn open_for(run: usize) -> bool {
    OPEN_RUNS.iter().any(|r| r.load(Relaxed) == run)
}

/// `run`'s packet opened ([`begin`]): its run in a free slot (with every slot taken it is not noted, and its
/// setters beside it skip as before).
fn note_open(run: usize) {
    for r in &OPEN_RUNS {
        if r.compare_exchange(0, run, Relaxed, Relaxed).is_ok() {
            return;
        }
    }
}

/// `run`'s packet closed ([`end`]).
fn note_closed(run: usize) {
    for r in &OPEN_RUNS {
        if r.compare_exchange(run, 0, Relaxed, Relaxed).is_ok() {
            return;
        }
    }
}

/// A typed setter's "already the cached value, skip" decision: `same` is what its cache says. Beside a packet
/// ([`beside`]) the cache is main's value at the cut, not what the draw left (which serially comes first), so the
/// setter sends and caches its value anyway: the device gets it after the draw's commands as serially, and the
/// cache's write is noted as main's for the join (`set_own_at`), which then keeps main's value over the draw's. The
/// one difference from the serial order: when the draw left the same value, the device gets it once more.
#[inline(always)]
pub fn main_must_send(e: &Engine, same: bool) -> bool {
    !same || beside(e)
}

fn run(e: &Engine) -> usize {
    e.kernel.id()
}

/// Whether this thread holds an open packet for `e`'s run.
pub fn is_open(e: &Engine) -> bool {
    OPEN.load(Relaxed) != 0 && PACKET.with(|p| p.borrow().as_ref().is_some_and(|p| p.run == run(e)))
}

/// Opens an empty packet on this thread for `e`'s run. False (nothing done) when this thread holds one already.
pub fn begin(e: &Engine) -> bool {
    let opened = PACKET.with(|p| {
        let mut p = p.borrow_mut();
        if p.is_some() {
            return false;
        }
        *p = Some(Packet {
            served: false,
            spare: None,
            run: run(e),
            objects: AddrMap::default(),
            ranges: Vec::new(),
            order: Vec::new(),
            bytes: Vec::new(),
            scribble: None,
            saved: Vec::new(),
            held: Vec::new(),
            misses: Cell::new(0),
            reads: Cell::new(0),
            paused: false,
            written: 0,
            own: Vec::new(),
            own_bytes: Vec::new(),
            own_cut: Vec::new(),
            own_main: Vec::new(),
            counted: Vec::new(),
            last_object: Cell::new((0, NONE)),
            last_own: Cell::new(0),
            own_by_lo: Vec::new(),
            lists: AddrMap::default(),
        });
        HOLDS.with(|h| h.set(true));
        true
    });
    if opened {
        GENERATION.fetch_add(1, Relaxed);
        // Main's writes noted before this cut belong to no packet.
        drop(take_marks(run(e)));
        OPEN.fetch_add(1, Relaxed);
        note_open(run(e));
    }
    opened
}

/// A half of the world draw starts on `e`: this thread's open packet for `e`'s run (not paused, not served already)
/// hands its own blocks and bytes and its copied ranges and bytes to `e` (`Engine::own_view`, with the own state's
/// page table built), so that the accessors read and write them as fields of the engine. Nothing without one. The
/// packet takes them back at [`unserve`] (the half's end, a pause, the join), before anything else reads them.
pub fn serve(e: &mut Engine) {
    if !active() {
        return;
    }
    let id = run(e);
    let view = PACKET.with(|p| {
        let mut p = p.borrow_mut();
        let p = p.as_mut()?;
        if p.run != id || p.paused || p.served {
            return None;
        }
        let mut v = p.spare.take().unwrap_or_default();
        v.own = std::mem::take(&mut p.own);
        v.own_bytes = std::mem::take(&mut p.own_bytes);
        v.objects = std::mem::take(&mut p.objects);
        v.ranges = std::mem::take(&mut p.ranges);
        v.bytes = std::mem::take(&mut p.bytes);
        v.lists = std::mem::take(&mut p.lists);
        v.reads.set(0);
        v.misses.set(0);
        v.poison = p.scribble.is_some();
        v.index();
        p.served = true;
        Some(v)
    });
    if let Some(v) = view {
        e.own_view = Some(v);
        SERVING.with(|s| s.set(true));
    }
}

thread_local! {
    /// This thread put the view on its engine ([`serve`]) and has not taken it back. Stepped game threads share one
    /// engine and take turns on it, so another game thread running while a half is served must not see the draw's copy:
    /// the accessors take the view only on the thread that served it ([`served`]).
    static SERVING: Cell<bool> = const { Cell::new(false) };
}

/// The engine's served view ([`serve`]) when this thread served it; `None` on any other thread (it reads guest
/// memory, as before the half began).
#[inline(always)]
fn served(view: &Option<Box<OwnView>>) -> Option<&OwnView> {
    let v = view.as_deref()?;
    SERVING.with(Cell::get).then_some(v)
}

/// [`served`], to write the draw's own state.
#[inline(always)]
fn served_mut(view: &mut Option<Box<OwnView>>) -> Option<&mut OwnView> {
    let v = view.as_deref_mut()?;
    SERVING.with(Cell::get).then_some(v)
}

/// Whether `e` holds a view this thread served ([`served`]).
#[inline(always)]
fn viewing(e: &Engine) -> bool {
    e.own_view.is_some() && SERVING.with(Cell::get)
}

/// The half has ended (or pauses, or joins): what [`serve`] put on `e` goes back into this thread's packet, the
/// view's reads and misses added to the packet's. Nothing when `e` holds none. A view whose packet is gone (a stop
/// that closed it elsewhere) is dropped.
pub fn unserve(e: &mut Engine) {
    // Only the thread that served it takes it back (its packet is this thread's).
    if !viewing(e) {
        return;
    }
    SERVING.with(|s| s.set(false));
    let Some(mut v) = e.own_view.take() else {
        return;
    };
    PACKET.with(|p| {
        let mut p = p.borrow_mut();
        let Some(p) = p.as_mut() else {
            return;
        };
        if !p.served {
            return;
        }
        p.own = std::mem::take(&mut v.own);
        p.own_bytes = std::mem::take(&mut v.own_bytes);
        p.objects = std::mem::take(&mut v.objects);
        p.ranges = std::mem::take(&mut v.ranges);
        p.bytes = std::mem::take(&mut v.bytes);
        p.lists = std::mem::take(&mut v.lists);
        p.reads.set(p.reads.get() + v.reads.get());
        p.misses.set(p.misses.get() + v.misses.get());
        p.last_object.set((0, NONE));
        p.served = false;
        p.spare = Some(v);
    });
}

/// Copies `object`'s bytes `[from, to)` into the open packet: the parts no range of it holds yet, each its own
/// range. Nothing without an open packet, after [`poison`], or for a null object.
pub fn take(e: &Engine, object: u32, from: u32, to: u32) {
    if object == 0 || from >= to {
        return;
    }
    PACKET.with(|p| {
        let mut p = p.borrow_mut();
        let Some(p) = p.as_mut() else {
            return;
        };
        if p.run != run(e) || p.scribble.is_some() {
            return;
        }
        let first = p.objects.get(&object).copied().unwrap_or(NONE);
        // The common case, an object not taken before: one part, read straight into the packet's bytes.
        if first == NONE {
            let at = p.bytes.len();
            p.bytes.resize(at + (to - from) as usize, 0);
            if e.mem
                .try_read(object.wrapping_add(from), &mut p.bytes[at..])
                .is_err()
            {
                p.bytes.truncate(at);
                return;
            }
            let index = u32::try_from(p.ranges.len()).expect("the packet stays under 4 GB");
            p.ranges.push(Range {
                from,
                to,
                at: u32::try_from(at).expect("the packet stays under 4 GB"),
                next: NONE,
            });
            p.order.push((object, index));
            p.objects.insert(object, index);
            p.last_object.set((0, NONE));
            return;
        }
        // The parts of [from, to) no range of the object holds.
        let mut parts = vec![(from, to)];
        let mut index = first;
        while index != NONE {
            let r = p.ranges[index as usize];
            parts = parts
                .into_iter()
                .flat_map(|(a, b)| {
                    let mut left = Vec::with_capacity(2);
                    if a < r.from.min(b) {
                        left.push((a, r.from.min(b)));
                    }
                    if r.to.max(a) < b {
                        left.push((r.to.max(a), b));
                    }
                    left
                })
                .collect();
            index = r.next;
        }
        let mut head = first;
        for (a, b) in parts {
            // A range not mapped is not taken (a read of it through the packet is a miss).
            let start = p.bytes.len();
            p.bytes.resize(start + (b - a) as usize, 0);
            if e.mem
                .try_read(object.wrapping_add(a), &mut p.bytes[start..])
                .is_err()
            {
                p.bytes.truncate(start);
                continue;
            }
            let at = u32::try_from(start).expect("the packet stays under 4 GB");
            let index = u32::try_from(p.ranges.len()).expect("the packet stays under 4 GB");
            p.ranges.push(Range {
                from: a,
                to: b,
                at,
                next: head,
            });
            p.order.push((object, index));
            head = index;
        }
        if head != NONE {
            p.objects.insert(object, head);
            // The object's first range is a new one now: the look-up cache must not keep the old head.
            p.last_object.set((0, NONE));
        }
    });
}

/// Takes the guest bytes `object + [from, to)` into the open packet as the draw's own state (step 2, piece 3):
/// the draw reads and writes them through the packet ([`own_at`], [`set_own_at`] and their kin) while main keeps
/// the guest bytes, and at [`end`] they go back to guest memory by the join's rule ([`end`]). The parts another
/// own block holds already stay as they were taken. Own state is kept by address, apart from the read ranges of
/// [`take`]: the cut never takes the same bytes both ways. Nothing without an open packet, after [`poison`], for
/// a null object, or for bytes not mapped.
pub fn own(e: &Engine, object: u32, from: u32, to: u32) {
    if object == 0 || from >= to {
        return;
    }
    let (Some(lo), Some(hi)) = (object.checked_add(from), object.checked_add(to)) else {
        return;
    };
    PACKET.with(|p| {
        let mut p = p.borrow_mut();
        let Some(p) = p.as_mut() else {
            return;
        };
        if p.run != run(e) || p.scribble.is_some() {
            return;
        }
        // The parts of [lo, hi) no block holds: the gaps between the blocks that meet it. The blocks are apart
        // and in address order (`own_by_lo`), so those are the run starting at the first that ends after `lo`.
        let mut parts: Vec<(u32, u32)> = Vec::new();
        let mut a = lo;
        let mut k = p.own_by_lo.partition_point(|&i| p.own[i as usize].hi <= lo);
        while a < hi {
            match p.own_by_lo.get(k).map(|&i| p.own[i as usize]) {
                Some(b) if b.lo < hi => {
                    if a < b.lo {
                        parts.push((a, b.lo));
                    }
                    a = a.max(b.hi);
                    k += 1;
                }
                _ => {
                    parts.push((a, hi));
                    break;
                }
            }
        }
        for (a, z) in parts {
            let at = p.own_bytes.len();
            p.own_bytes.resize(at + (z - a) as usize, 0);
            if e.mem.try_read(a, &mut p.own_bytes[at..]).is_err() {
                p.own_bytes.truncate(at);
                continue;
            }
            let (bytes, cut) = (&p.own_bytes, &mut p.own_cut);
            cut.extend_from_slice(&bytes[at..]);
            let index = u32::try_from(p.own.len()).expect("the packet stays under 4 GB");
            let k = p.own_by_lo.partition_point(|&i| p.own[i as usize].lo < a);
            p.own_by_lo.insert(k, index);
            let at = u32::try_from(at).expect("the packet stays under 4 GB");
            p.own.push(OwnBlock { lo: a, hi: z, at });
        }
    });
}
/// Marks the own word at `addr` (taken already, [`own`]) as a counted pointer: the drawing side's holds of it leave
/// the counts to the join ([`end`]). Nothing without an open packet for `e`'s run.
pub fn own_counted(e: &Engine, addr: u32) {
    PACKET.with(|p| {
        if let Some(p) = p.borrow_mut().as_mut() {
            if p.run == run(e) && p.own_at(addr, 4).is_some() && !p.counted.contains(&addr) {
                p.counted.push(addr);
            }
        }
    });
}

/// Notes that main decided the own state's bytes `[addr, addr + len)` after the cut although it wrote nothing there (a
/// hold of the object already held): the join keeps main's value for them, as a write would. Nothing on the thread
/// whose packet serves `e`, or while no packet is open.
pub fn note_write(e: &Engine, addr: u32, len: u32) {
    if !active() {
        return;
    }
    let mine = PACKET.with(|p| p.borrow().as_ref().is_some_and(|p| p.serves(e)));
    if !mine {
        mark(run(e), addr, len);
    }
}

/// Whether this thread's open packet owns all of `[addr, addr + len)` in one own block (for the checks).
pub fn owns(e: &Engine, addr: u32, len: u32) -> bool {
    if let Some(v) = served(&e.own_view) {
        return v.own_at(addr, len).is_some();
    }
    PACKET.with(|p| {
        p.borrow()
            .as_ref()
            .is_some_and(|p| p.serves(e) && p.own_at(addr, len).is_some())
    })
}

/// Holds a reference on `object` (`NiRefObject::IncRefCount`) until [`end`] gives it back, so the frame's tail
/// cannot free an object the draw still reads. Nothing without an open packet.
pub fn hold(e: &mut Engine, object: u32) {
    if object == 0 || !is_open(e) {
        return;
    }
    typed_call!(
        crate::dispatch::FAMILY,
        0x0040_f6e0,
        crate::dispatch::refs::add_reference(e, Ptr::new(object))
    );
    PACKET.with(|p| {
        if let Some(p) = p.borrow_mut().as_mut() {
            p.held.push(object);
        }
    });
}

/// Runs `f` on this thread's open packet for `e`'s run, taken off the thread for the call (so `f` may use the
/// engine's memory freely). `None` without one.
fn with_packet<R>(e: &mut Engine, f: impl FnOnce(&mut Engine, &mut Packet) -> R) -> Option<R> {
    let mut packet = PACKET.with(|p| {
        let mut p = p.borrow_mut();
        if p.as_ref().is_some_and(|p| p.run == run(e)) {
            p.take()
        } else {
            None
        }
    })?;
    let r = f(e, &mut packet);
    PACKET.with(|p| *p.borrow_mut() = Some(packet));
    Some(r)
}

/// Every guest byte the packet holds saved and set to `scribble`, in the order taken; then the own blocks', main's
/// values kept aside while the scribble covers them.
fn scribble_all(e: &mut Engine, p: &mut Packet, scribble: u8) {
    p.saved = vec![0; p.bytes.len()];
    for &(object, index) in &p.order {
        let r = p.ranges[index as usize];
        let addr = object.wrapping_add(r.from);
        let at = r.at as usize;
        let len = (r.to - r.from) as usize;
        e.mem.read(addr, &mut p.saved[at..at + len]);
        e.mem.write(addr, &vec![scribble; len]);
    }
    p.own_main = vec![0; p.own_bytes.len()];
    for b in &p.own {
        let at = b.at as usize;
        let len = (b.hi - b.lo) as usize;
        e.mem.read(b.lo, &mut p.own_main[at..at + len]);
        e.mem.write(b.lo, &vec![scribble; len]);
    }
}

/// The scribbled guest bytes put back: the own blocks' main's values, then the ranges' in the reverse order. A byte
/// no longer the scribble (written past the packet while the draw ran) is counted, and left as written in a range.
fn unscribble_all(e: &mut Engine, p: &Packet, scribble: u8) -> u64 {
    let mut written = 0;
    for b in p.own.iter().rev() {
        let at = b.at as usize;
        let mut live = vec![0u8; (b.hi - b.lo) as usize];
        e.mem.read(b.lo, &mut live);
        written += live.iter().filter(|&&byte| byte != scribble).count() as u64;
        e.mem.write(b.lo, &p.own_main[at..at + live.len()]);
    }
    for &(object, index) in p.order.iter().rev() {
        let r = p.ranges[index as usize];
        let addr = object.wrapping_add(r.from);
        let mut live = vec![0u8; (r.to - r.from) as usize];
        e.mem.read(addr, &mut live);
        let saved = &p.saved[r.at as usize..r.at as usize + live.len()];
        for (byte, old) in live.iter_mut().zip(saved) {
            if *byte == scribble {
                *byte = *old;
            } else {
                written += 1;
            }
        }
        e.mem.write(addr, &live);
    }
    written
}

/// POISON: every guest byte the packet holds is saved and set to `scribble`, in the order taken. Once a packet;
/// nothing without one.
pub fn poison(e: &mut Engine, scribble: u8) {
    unserve(e);
    with_packet(e, |e, p| {
        if p.scribble.is_none() {
            p.scribble = Some(scribble);
            if !p.paused {
                scribble_all(e, p, scribble);
            }
        }
    });
}

/// The world draw's first half has returned: until [`resume`], the packet is not read (the accessors answer guest
/// memory: what runs between the halves, such as the VATS target's draw, is main's), and with POISON the guest
/// bytes are put back (those written meanwhile counted). Nothing without an open packet, or when paused already.
pub fn pause(e: &mut Engine) {
    unserve(e);
    with_packet(e, |e, p| {
        if p.paused {
            return;
        }
        p.paused = true;
        PAUSED.with(|f| f.set(true));
        if let Some(scribble) = p.scribble {
            p.written += unscribble_all(e, p, scribble);
        }
    });
}

/// POISON's scribble put back for good (the bytes written meanwhile counted, as [`end`] counts them): the drawing
/// thread gives the packet back to main while main still waits, so main never reads the scribble. Nothing without an
/// open packet or a scribble.
pub fn unpoison(e: &mut Engine) {
    unserve(e);
    with_packet(e, |e, p| {
        if let Some(scribble) = p.scribble.take() {
            if !p.paused {
                p.written += unscribble_all(e, p, scribble);
            }
        }
    });
}

/// The world draw's second half starts: the packet is read again ([`pause`]), and with POISON scribbled again.
pub fn resume(e: &mut Engine) {
    unserve(e);
    with_packet(e, |e, p| {
        if !p.paused {
            return;
        }
        p.paused = false;
        PAUSED.with(|f| f.set(false));
        if let Some(scribble) = p.scribble {
            scribble_all(e, p, scribble);
        }
    });
}

/// Main's writes of own-state addresses while another thread's packet is open (the join's rule, [`end`]): the run,
/// the address and the length of each write a routed setter ([`set_own_at`] and its kin) made into guest memory
/// on a thread holding no packet for that run. Only pushed to while some packet is open.
static MARKS: std::sync::Mutex<Vec<(usize, u32, u32)>> = std::sync::Mutex::new(Vec::new());

/// Notes a write main made at `[addr, addr + len)` while a packet is open (see [`MARKS`]).
#[cold]
fn mark(run: usize, addr: u32, len: u32) {
    // The same write again (a setter in a loop) adds nothing to the set the join reads: skip the lock.
    let noted = (GENERATION.load(Relaxed), run, addr, len);
    if LAST_MARK.with(|m| m.replace(noted)) == noted {
        return;
    }
    MARKS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push((run, addr, len));
}

/// The marks of `run` taken out of [`MARKS`].
fn take_marks(run: usize) -> Vec<(u32, u32)> {
    let mut marks = MARKS.lock().unwrap_or_else(|p| p.into_inner());
    let mut out = Vec::new();
    marks.retain(|&(r, addr, len)| {
        if r == run {
            out.push((addr, len));
            false
        } else {
            true
        }
    });
    out
}
/// What a packet saw, given back by [`end`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Reads through the packet of an object range it did not hold.
    pub misses: u64,
    /// Reads the packet answered.
    pub reads: u64,
    /// POISON: scribbled bytes found changed as the packet closed (written while the draw ran).
    pub written: u64,
    /// The ranges and bytes it held, and the references.
    pub ranges: usize,
    pub bytes: usize,
    pub held: usize,
    /// The own state's bytes ([`own`]), and those of them the join gave main's value ([`end`]).
    pub owned: usize,
    pub kept_main: u64,
}

/// Closes this thread's packet: POISON's scribbled bytes go back (in the reverse order, those written meanwhile
/// left as written); the own state goes back to guest memory; then the held references are given back
/// (`NiRefObject::DecRefCount`). The default report when none is open.
///
/// **The join's rule for own state** (the device model's, `measure/typed-draw/TWO-PRODUCERS.md`: main keeps its
/// value for every state it set after the cut): a byte keeps main's value when main wrote it after the cut
/// through a routed setter ([`set_own_at`] and its kin note each such write, also one that sets a value back to
/// what it was, or writes an equal value), or when it differs from its value at the cut (a write by any other
/// path); every other byte takes the draw's value. In serial order the draw runs before the tail, so this is the
/// value the original leaves. With POISON, a byte found no longer the scribble is a write past the packet,
/// counted in [`Report::written`].
pub fn end(e: &mut Engine) -> Report {
    if OPEN.load(Relaxed) == 0 {
        return Report::default();
    }
    unserve(e);
    let packet = PACKET.with(|p| {
        let mut p = p.borrow_mut();
        if p.as_ref().is_some_and(|p| p.run == run(e)) {
            p.take()
        } else {
            None
        }
    });
    let Some(packet) = packet else {
        return Report::default();
    };
    PAUSED.with(|f| f.set(false));
    HOLDS.with(|h| h.set(false));
    OPEN.fetch_sub(1, Relaxed);
    note_closed(packet.run);
    give_lists_back(e, &packet.lists);
    let mut written = packet.written;
    if let Some(scribble) = packet.scribble {
        if !packet.paused {
            written += unscribble_all(e, &packet, scribble);
        }
    }
    // The own state, by the join's rule (guest memory holds main's values now).
    let mut marked = vec![false; packet.own_bytes.len()];
    for (addr, len) in take_marks(packet.run) {
        for a in (0..len).filter_map(|i| addr.checked_add(i)) {
            if let Some(at) = packet.own_at(a, 1) {
                marked[at] = true;
            }
        }
    }
    let mut kept_main = 0;
    let mut kept = vec![false; packet.own_bytes.len()];
    for b in &packet.own {
        let len = (b.hi - b.lo) as usize;
        let at = b.at as usize;
        let mut live = vec![0u8; len];
        e.mem.read(b.lo, &mut live);
        let draw = &packet.own_bytes[at..at + len];
        let cut = &packet.own_cut[at..at + len];
        for (i, byte) in live.iter_mut().enumerate() {
            if marked[at + i] || *byte != cut[i] {
                kept_main += 1;
                kept[at + i] = true;
            } else {
                *byte = draw[i];
            }
        }
        e.mem.write(b.lo, &live);
    }
    // The counted own words ([`own_counted`]): the drawing side's holds left the counts alone (a hold on main meanwhile
    // let the cut's object go too); where the join takes the drawing side's object, its counts move now as the draw's
    // own hold would have moved them in the serial order: the cut's object let go, the drawn one held. Where main held
    // another (or the same, noted), main's holds already moved them as in the serial order.
    for &addr in &packet.counted {
        let Some(at) = packet.own_at(addr, 4) else {
            continue;
        };
        if kept[at..at + 4].iter().any(|&k| k) {
            continue;
        }
        let word = |b: &[u8]| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
        let (cut, draw) = (word(&packet.own_cut), word(&packet.own_bytes));
        if cut != draw {
            super::own::move_count(e, cut, draw);
        }
    }
    for object in &packet.held {
        typed_call!(
            crate::dispatch::FAMILY,
            0x0040_1970,
            crate::dispatch::refs::release(e, Ptr::new(*object))
        );
    }
    Report {
        misses: packet.misses.get(),
        reads: packet.reads.get(),
        written,
        ranges: packet.ranges.len(),
        bytes: packet.bytes.len(),
        held: packet.held.len(),
        owned: packet.own_bytes.len(),
        kept_main,
    }
}

/// An open packet taken off its thread ([`detach`]), to be put on another ([`attach`]): the drawing thread hands
/// its packet to main at the job's end so that main closes it at the join (piece 8). It stays counted as open.
pub struct Detached(Box<Packet>);

/// Takes this thread's open packet for `e`'s run off the thread: until it is attached again, this thread's
/// routed setters write guest memory and are noted for the join ([`end`]). `None` without one.
pub fn detach(e: &Engine) -> Option<Detached> {
    PACKET.with(|p| {
        let mut p = p.borrow_mut();
        if p.as_ref().is_some_and(|p| p.run == run(e)) {
            assert!(
                !p.as_ref().is_some_and(|p| p.served),
                "NV_DRAW_THREAD: a packet detached while a half draws from it (unserve first)"
            );
            PAUSED.with(|f| f.set(false));
            HOLDS.with(|h| h.set(false));
            p.take().map(|p| Detached(Box::new(p)))
        } else {
            None
        }
    })
}

/// Puts a detached packet on this thread. Given back when this thread holds a packet already.
pub fn attach(packet: Detached) -> Result<(), Detached> {
    PACKET.with(|p| {
        let mut p = p.borrow_mut();
        if p.is_some() {
            return Err(packet);
        }
        let paused = packet.0.paused;
        *p = Some(*packet.0);
        HOLDS.with(|h| h.set(true));
        PAUSED.with(|f| f.set(paused));
        Ok(())
    })
}

/// The `N` bytes at `object + offset` as the open packet holds them: `None` with no packet open for `e`'s run,
/// and `None` counted as a miss when the packet holds no range of the object that covers them. A null object is not
/// a miss (as for [`span`]): the cut never copies one, and its read is of the zero page, which nothing on main changes.
#[inline(always)]
fn copied<const N: usize>(e: &Engine, object: u32, offset: u32) -> Option<[u8; N]> {
    if object == 0 {
        return None;
    }
    if let Some(v) = served(&e.own_view) {
        let mut out = [0u8; N];
        if v.read_into(object, offset, &mut out) {
            return Some(out);
        }
        missed(object, offset, v.poison);
        return None;
    }
    PACKET.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        if !p.serves(e) {
            return None;
        }
        let mut out = [0u8; N];
        p.read_or_miss(object, offset, &mut out).then_some(out)
    })
}

/// The word at `object + offset`: the packet's copy while one is open, else guest memory's.
#[inline(always)]
pub fn word(e: &Engine, object: u32, offset: u32) -> u32 {
    if viewing(e) || held() {
        if let Some(bytes) = copied::<4>(e, object, offset) {
            return u32::from_le_bytes(bytes);
        }
    }
    e.global::<u32>(object.wrapping_add(offset))
}

/// The 16-bit half at `object + offset`, as [`word`].
#[inline(always)]
pub fn half(e: &Engine, object: u32, offset: u32) -> u16 {
    if viewing(e) || held() {
        if let Some(bytes) = copied::<2>(e, object, offset) {
            return u16::from_le_bytes(bytes);
        }
    }
    e.global::<u16>(object.wrapping_add(offset))
}

/// The byte at `object + offset`, as [`word`].
#[inline(always)]
pub fn byte(e: &Engine, object: u32, offset: u32) -> u8 {
    if viewing(e) || held() {
        if let Some([b]) = copied::<1>(e, object, offset) {
            return b;
        }
    }
    e.global::<u8>(object.wrapping_add(offset))
}

/// `N` words from `object + offset` on, as [`word`] (one look-up for all of them).
#[inline(always)]
pub fn words<const N: usize>(e: &Engine, object: u32, offset: u32) -> [u32; N] {
    if let Some(v) = served(&e.own_view) {
        let len = u32::try_from(4 * N).unwrap_or(u32::MAX);
        if let Some(r) = v.find(object, offset, len) {
            v.reads.set(v.reads.get() + 1);
            let at = (r.at + (offset - r.from)) as usize;
            return std::array::from_fn(|i| {
                let b = &v.bytes[at + 4 * i..at + 4 * i + 4];
                u32::from_le_bytes([b[0], b[1], b[2], b[3]])
            });
        }
        // Spread over several ranges.
        let mut out = [0u32; N];
        let mut whole = true;
        for (i, w) in (0u32..).zip(out.iter_mut()) {
            let mut b = [0u8; 4];
            let at = offset.wrapping_add(4 * i);
            if !v.read_into(object, at, &mut b) {
                missed(object, at, v.poison);
                whole = false;
                break;
            }
            *w = u32::from_le_bytes(b);
        }
        if whole {
            return out;
        }
    } else if held() {
        let held = PACKET.with(|p| {
            let p = p.borrow();
            let p = p.as_ref()?;
            if !p.serves(e) {
                return None;
            }
            let len = u32::try_from(4 * N).unwrap_or(u32::MAX);
            if let Some(r) = p.find(object, offset, len) {
                p.reads.set(p.reads.get() + 1);
                let at = (r.at + (offset - r.from)) as usize;
                return Some(std::array::from_fn(|i| {
                    let b = &p.bytes[at + 4 * i..at + 4 * i + 4];
                    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                }));
            }
            // Spread over several ranges.
            let mut out = [0u32; N];
            for (i, w) in (0u32..).zip(out.iter_mut()) {
                let mut b = [0u8; 4];
                if !p.read_or_miss(object, offset.wrapping_add(4 * i), &mut b) {
                    return None;
                }
                *w = u32::from_le_bytes(b);
            }
            Some(out)
        });
        if let Some(words) = held {
            return words;
        }
    }
    std::array::from_fn(|i| {
        e.global::<u32>(
            object
                .wrapping_add(offset)
                .wrapping_add(4 * u32::try_from(i).unwrap_or(0)),
        )
    })
}

/// Whether any thread holds an open packet: the one test every routed accessor makes first. With
/// `NV_DRAW_THREAD` unset it is always false, so an accessor costs this one relaxed load and a branch that never
/// goes the other way.
#[inline(always)]
pub fn active() -> bool {
    OPEN.load(Relaxed) != 0
}

/// A value the draw's own state holds ([`own_at`]): its bytes, little-endian, as guest memory keeps them.
pub trait OwnValue: Scalar {
    const N: usize;
    /// [`OwnValue::N`] as a guest length.
    const L: u32;
    fn from_bytes(b: &[u8]) -> Self;
    fn to_bytes(self, b: &mut [u8]);
}

macro_rules! own_value {
    ($($t:ty = $l:literal),*) => {$(
        impl OwnValue for $t {
            const N: usize = std::mem::size_of::<$t>();
            const L: u32 = $l;
            #[inline(always)]
            fn from_bytes(b: &[u8]) -> Self {
                let mut a = [0u8; std::mem::size_of::<$t>()];
                a.copy_from_slice(b);
                <$t>::from_le_bytes(a)
            }
            #[inline(always)]
            fn to_bytes(self, b: &mut [u8]) {
                b.copy_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}
own_value!(
    u8 = 1,
    i8 = 1,
    u16 = 2,
    i16 = 2,
    u32 = 4,
    i32 = 4,
    u64 = 8,
    f32 = 4,
    f64 = 8
);

impl OwnValue for bool {
    const N: usize = 1;
    const L: u32 = 1;
    #[inline(always)]
    fn from_bytes(b: &[u8]) -> Self {
        b[0] != 0
    }
    #[inline(always)]
    fn to_bytes(self, b: &mut [u8]) {
        b[0] = u8::from(self);
    }
}

/// The value at guest address `addr` of the draw's own state: the packet's copy while this thread's packet owns
/// it ([`own`]), else guest memory (own state the cut has not taken is guest memory's, as without a packet).
#[inline(always)]
pub fn own_at<T: OwnValue>(e: &Engine, addr: u32) -> T {
    if let Some(v) = served(&e.own_view) {
        if let Some(at) = v.own_at(addr, T::L) {
            return T::from_bytes(&v.own_bytes[at..at + T::N]);
        }
    } else if held() {
        if let Some(v) = own_lookup::<T>(e, addr) {
            return v;
        }
    }
    e.global::<T>(addr)
}

#[cold]
fn own_lookup<T: OwnValue>(e: &Engine, addr: u32) -> Option<T> {
    PACKET.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        if !p.serves(e) {
            return None;
        }
        let at = p.own_at(addr, T::L)?;
        Some(T::from_bytes(&p.own_bytes[at..at + T::N]))
    })
}

/// Writes the value at guest address `addr` of the draw's own state: into the packet's copy while this thread's
/// packet owns it ([`own`]), else into guest memory. A write into guest memory while another thread's packet is
/// open is noted for the join's rule ([`end`]).
#[inline(always)]
pub fn set_own_at<T: OwnValue>(e: &mut Engine, addr: u32, value: T) {
    if let Some(v) = served_mut(&mut e.own_view) {
        // The drawing side: its own state in the view; anything else is guest memory, not noted (it is no write of
        // main's).
        if let Some(at) = v.own_at(addr, T::L) {
            value.to_bytes(&mut v.own_bytes[at..at + T::N]);
            return;
        }
    } else if active() && own_store(e, addr, value) {
        return;
    }
    e.set_global(addr, value);
}

/// [`set_own_at`] while a packet is open: true when this thread's packet took the value.
#[cold]
fn own_store<T: OwnValue>(e: &Engine, addr: u32, value: T) -> bool {
    let id = run(e);
    let mine = PACKET.with(|p| {
        let mut p = p.borrow_mut();
        let p = p.as_mut()?;
        if p.run != id || p.paused {
            return None;
        }
        let Some(at) = p.own_at(addr, T::L) else {
            return Some(false);
        };
        value.to_bytes(&mut p.own_bytes[at..at + T::N]);
        Some(true)
    });
    match mine {
        Some(stored) => stored,
        // Not the drawing side: main writes guest memory, noted.
        None => {
            mark(id, addr, T::L);
            false
        }
    }
}

/// Runs `f` with an address that stands for the own state's bytes `[addr, addr + len)`: while this thread's packet
/// owns them all, a guest-stack image of them, copied back into the packet after `f` (a helper that writes through
/// the pointer writes the draw's own state); else `addr` itself (a write through it by main is noted for the join).
/// For a helper that takes a pointer into the own state (a constant upload, a matrix written in place).
pub fn own_image<R>(
    e: &mut Engine,
    addr: u32,
    len: u32,
    f: impl FnOnce(&mut Engine, u32) -> R,
) -> R {
    if !active() || len == 0 {
        return f(e, addr);
    }
    if viewing(e) {
        let Some(at) = served(&e.own_view).and_then(|v| v.own_at(addr, len)) else {
            // The drawing side's pointer outside its own state: guest memory, nothing noted.
            return f(e, addr);
        };
        let n = len as usize;
        return e.with_stack((len + 3) & !3, |e, block| {
            let block = block.addr();
            if let Some(v) = served(&e.own_view) {
                e.mem.write(block, &v.own_bytes[at..at + n]);
            }
            let r = f(e, block);
            if let Some(v) = served_mut(&mut e.own_view) {
                if v.own_bytes.len() >= at + n {
                    e.mem.read(block, &mut v.own_bytes[at..at + n]);
                }
            }
            r
        });
    }
    let held = own_place(e, addr, len);
    let Some(at) = held else {
        let id = run(e);
        let mine = PACKET.with(|p| p.borrow().as_ref().is_some_and(|p| p.serves(e)));
        if !mine {
            mark(id, addr, len);
        }
        return f(e, addr);
    };
    let size = (len + 3) & !3;
    let n = len as usize;
    e.with_stack(size, |e, block| {
        let block = block.addr();
        // Straight from the packet's bytes to the image and back: no copy in between. The own blocks only grow
        // at their end while the packet is open, so `at` still names these bytes after `f`.
        PACKET.with(|p| {
            if let Some(p) = p.borrow().as_ref() {
                e.mem.write(block, &p.own_bytes[at..at + n]);
            }
        });
        let r = f(e, block);
        PACKET.with(|p| {
            if let Some(p) = p.borrow_mut().as_mut() {
                if p.own_bytes.len() >= at + n {
                    e.mem.read(block, &mut p.own_bytes[at..at + n]);
                }
            }
        });
        r
    })
}

/// Where this thread's packet holds the own bytes `[addr, addr + len)`, if it serves `e` and one block holds them.
#[inline(always)]
fn own_place(e: &Engine, addr: u32, len: u32) -> Option<usize> {
    PACKET.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        if !p.serves(e) {
            return None;
        }
        p.own_at(addr, len)
    })
}

/// Runs `f` with an address that stands for the own state's bytes `[addr, addr + len)` for reading only: while this
/// thread's packet owns them all, a guest-stack image of them (nothing is copied back); else `addr` itself, and
/// nothing is noted (a read is no write of main's). For a helper handed a pointer it only reads through (a matrix
/// product's operand, a point a rotation reads).
pub fn own_image_read<R>(
    e: &mut Engine,
    addr: u32,
    len: u32,
    f: impl FnOnce(&mut Engine, u32) -> R,
) -> R {
    if len == 0 {
        return f(e, addr);
    }
    if viewing(e) {
        let Some(at) = served(&e.own_view).and_then(|v| v.own_at(addr, len)) else {
            return f(e, addr);
        };
        let n = len as usize;
        return e.with_stack((len + 3) & !3, |e, block| {
            let block = block.addr();
            if let Some(v) = served(&e.own_view) {
                e.mem.write(block, &v.own_bytes[at..at + n]);
            }
            f(e, block)
        });
    }
    if !held() {
        return f(e, addr);
    }
    let Some(at) = own_place(e, addr, len) else {
        return f(e, addr);
    };
    let n = len as usize;
    e.with_stack((len + 3) & !3, |e, block| {
        let block = block.addr();
        PACKET.with(|p| {
            if let Some(p) = p.borrow().as_ref() {
                e.mem.write(block, &p.own_bytes[at..at + n]);
            }
        });
        f(e, block)
    })
}

/// The own state's bytes `[addr, addr + len)` as the draw reads them ([`own_at`] byte by byte, one look-up).
pub fn own_bytes(e: &Engine, addr: u32, len: u32) -> Vec<u8> {
    if let Some(v) = served(&e.own_view) {
        if let Some(at) = v.own_at(addr, len) {
            return v.own_bytes[at..at + len as usize].to_vec();
        }
    } else if held() {
        let held = PACKET.with(|p| {
            let p = p.borrow();
            let p = p.as_ref()?;
            if !p.serves(e) {
                return None;
            }
            let at = p.own_at(addr, len)?;
            Some(p.own_bytes[at..at + len as usize].to_vec())
        });
        if let Some(bytes) = held {
            return bytes;
        }
    }
    e.mem.bytes(addr, len)
}

/// A field of the draw's own state through a typed pointer, as [`own_at`].
#[inline(always)]
pub fn own_get<S, T: OwnValue>(e: &Engine, p: Ptr<S>, f: Field<S, T>) -> T {
    own_at(e, p.addr().wrapping_add(f.off))
}

/// A field of the draw's own state written through a typed pointer, as [`set_own_at`].
#[inline(always)]
pub fn own_set<S, T: OwnValue>(e: &mut Engine, p: Ptr<S>, f: Field<S, T>, value: T) {
    set_own_at(e, p.addr().wrapping_add(f.off), value);
}

/// The word at `object + offset` of the draw's own state, as [`own_at`].
#[inline(always)]
pub fn own_word(e: &Engine, object: u32, offset: u32) -> u32 {
    own_at(e, object.wrapping_add(offset))
}

/// Writes the word at `object + offset` of the draw's own state, as [`set_own_at`].
#[inline(always)]
pub fn set_word(e: &mut Engine, object: u32, offset: u32, value: u32) {
    set_own_at(e, object.wrapping_add(offset), value);
}

/// Writes the 16-bit half at `object + offset`, as [`set_word`].
#[inline(always)]
pub fn set_half(e: &mut Engine, object: u32, offset: u32, value: u16) {
    set_own_at(e, object.wrapping_add(offset), value);
}

/// Writes the byte at `object + offset`, as [`set_word`].
#[inline(always)]
pub fn set_byte(e: &mut Engine, object: u32, offset: u32, value: u8) {
    set_own_at(e, object.wrapping_add(offset), value);
}

/// An object's bytes from offset `from` on, read through the packet: its copy when the open packet holds them
/// (one look-up for every read), else guest memory. For an array walked element by element.
pub struct Span {
    object: u32,
    from: u32,
    copy: Option<Vec<u8>>,
}

/// The [`Span`] of `object`'s offsets `[from, to)`. Without an open packet it reads guest memory; with one that
/// does not hold the whole range, a miss is counted and it reads guest memory.
pub fn span(e: &Engine, object: u32, from: u32, to: u32) -> Span {
    let copy = if object == 0 || from >= to {
        None
    } else if let Some(v) = served(&e.own_view) {
        let mut bytes = vec![0u8; (to - from) as usize];
        if v.read_into(object, from, &mut bytes) {
            Some(bytes)
        } else {
            missed(object, from, v.poison);
            None
        }
    } else if !held() {
        None
    } else {
        PACKET.with(|p| {
            let p = p.borrow();
            let p = p.as_ref()?;
            if !p.serves(e) {
                return None;
            }
            let mut bytes = vec![0u8; (to - from) as usize];
            p.read_or_miss(object, from, &mut bytes).then_some(bytes)
        })
    };
    Span { object, from, copy }
}

impl Span {
    /// The bytes at `offset` (from the object), `N` of them, from the copy when it holds them.
    #[inline(always)]
    fn bytes<const N: usize>(&self, offset: u32) -> Option<[u8; N]> {
        let copy = self.copy.as_ref()?;
        let at = offset.checked_sub(self.from)? as usize;
        let mut out = [0u8; N];
        out.copy_from_slice(copy.get(at..at + N)?);
        Some(out)
    }

    /// The word at the object's `offset`.
    #[inline(always)]
    pub fn word(&self, e: &Engine, offset: u32) -> u32 {
        match self.bytes::<4>(offset) {
            Some(b) => u32::from_le_bytes(b),
            None => e.global::<u32>(self.object.wrapping_add(offset)),
        }
    }

    /// The 16-bit half at the object's `offset`.
    #[inline(always)]
    pub fn half(&self, e: &Engine, offset: u32) -> u16 {
        match self.bytes::<2>(offset) {
            Some(b) => u16::from_le_bytes(b),
            None => e.global::<u16>(self.object.wrapping_add(offset)),
        }
    }

    /// The byte at the object's `offset`.
    #[inline(always)]
    pub fn byte(&self, e: &Engine, offset: u32) -> u8 {
        match self.bytes::<1>(offset) {
            Some([b]) => b,
            None => e.global::<u8>(self.object.wrapping_add(offset)),
        }
    }
}

/// Runs `f` with an address that stands for `object` over its offsets `[from, to)`: while the packet holds
/// that range, a guest-stack image of it (`f`'s address plus an offset in the range points into the image),
/// else `object` itself. For a helper that takes a pointer into a copied object.
pub fn image<R>(
    e: &mut Engine,
    object: u32,
    from: u32,
    to: u32,
    f: impl FnOnce(&mut Engine, u32) -> R,
) -> R {
    if from >= to {
        return f(e, object);
    }
    let held = if let Some(v) = served(&e.own_view) {
        let mut bytes = vec![0u8; (to - from) as usize];
        if v.read_into(object, from, &mut bytes) {
            Some(bytes)
        } else {
            missed(object, from, v.poison);
            None
        }
    } else if !held() {
        return f(e, object);
    } else {
        image_copy(e, object, from, to)
    };
    let Some(bytes) = held else {
        return f(e, object);
    };
    let size = (to - from + 3) & !3;
    e.with_stack(size, |e, block| {
        let block = block.addr();
        e.mem.write(block, &bytes);
        f(e, block.wrapping_sub(from))
    })
}

/// [`image`]'s copy from this thread's packet.
fn image_copy(e: &Engine, object: u32, from: u32, to: u32) -> Option<Vec<u8>> {
    PACKET.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        if !p.serves(e) {
            return None;
        }
        let mut bytes = vec![0u8; (to - from) as usize];
        p.read_or_miss(object, from, &mut bytes).then_some(bytes)
    })
}

// ---- The world draw's list words, handed to the drawing side at the cut (step 1 of `super::detach`, in play). ----
//
// The world pass fills its accumulator's lists (the active pass type lists, the pass groups' lists, the register group
// holders', the alpha groups'); the draw walks them, draws and empties them. For the drawing thread the cut takes their
// words into the packet and leaves zeros in guest memory, as a fresh frame's would be ([`take_lists`]); the draw's list
// accessors (`super::detach::word` and the rest) answer from the packet, served on the drawing engine like the rest of
// the copy; the join puts the words back ([`give_lists_back`]). Until the joins that guard the lists move, main does not
// fill the world accumulator's lists between the cut and the join, so the frame ends with the memory of the serial order;
// a list read that bypassed the packet would read a zero instead, which the stepped gate would show.

/// Takes the words at `addrs` (4-byte aligned) into this thread's open packet for `e`'s run: each word's value is kept,
/// the guest word zeroed. A word taken already, or of value 0, is left. Nothing without an open packet.
pub fn take_lists(e: &mut Engine, addrs: &[u32]) {
    let id = run(e);
    let mut taken = Vec::new();
    PACKET.with(|p| {
        let mut p = p.borrow_mut();
        let Some(p) = p.as_mut().filter(|p| p.run == id && !p.served) else {
            return;
        };
        for &addr in addrs {
            if p.lists.contains_key(&addr) {
                continue;
            }
            let value = e.global::<u32>(addr);
            if value != 0 {
                p.lists.insert(addr, value);
                taken.push(addr);
            }
        }
    });
    for addr in taken {
        e.set_global(addr, 0u32);
    }
}

/// The join: the list words go back to guest memory, in address order. A guest word main wrote meanwhile (not zero any
/// more) keeps main's value: with the joins that guard the lists in place that does not happen; it is noted once.
fn give_lists_back(e: &mut Engine, lists: &AddrMap<u32, u32>) {
    if lists.is_empty() {
        return;
    }
    let mut words: Vec<(u32, u32)> = lists.iter().map(|(a, v)| (*a, *v)).collect();
    words.sort_unstable();
    for (addr, value) in words {
        if e.global::<u32>(addr) != 0 {
            static NOTED: std::sync::Once = std::sync::Once::new();
            NOTED.call_once(|| {
                eprintln!(
                    "nv-draw packet: main wrote a list word the drawing side held: {addr:#010x}"
                );
            });
            continue;
        }
        e.set_global(addr, value);
    }
}

/// Whether this thread draws from a packet that holds list words (the drawing side, served or not).
pub fn lists_here(e: &Engine) -> bool {
    if let Some(v) = served(&e.own_view) {
        return !v.lists.is_empty();
    }
    held()
        && PACKET.with(|p| {
            p.borrow()
                .as_ref()
                .is_some_and(|p| p.serves(e) && !p.lists.is_empty())
        })
}

/// The list word at `addr` as the drawing side holds it ([`take_lists`]): `None` when this thread's packet holds no such
/// word (then guest memory has it).
#[inline(always)]
pub fn list_word(e: &Engine, addr: u32) -> Option<u32> {
    if let Some(v) = served(&e.own_view) {
        return v.lists.get(&addr).copied();
    }
    if !held() {
        return None;
    }
    PACKET.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        if !p.serves(e) {
            return None;
        }
        p.lists.get(&addr).copied()
    })
}

/// Writes the list word at `addr` where the drawing side holds it; false (nothing done) where it does not.
#[inline(always)]
pub fn set_list_word(e: &mut Engine, addr: u32, value: u32) -> bool {
    if let Some(v) = served_mut(&mut e.own_view) {
        return v.lists.get_mut(&addr).map(|w| *w = value).is_some();
    }
    if !held() {
        return false;
    }
    let id = run(e);
    PACKET.with(|p| {
        let mut p = p.borrow_mut();
        let Some(p) = p.as_mut().filter(|p| p.run == id && !p.paused) else {
            return false;
        };
        p.lists.get_mut(&addr).map(|w| *w = value).is_some()
    })
}

/// For a callee that reads or writes the lists in place (a translation by address): the list words `addrs` names (all
/// with `None`) back in guest memory for it, taken again after it (their values then). Answers what it put back.
pub fn lists_out(e: &mut Engine, addrs: Option<&[u32]>) -> Vec<u32> {
    if !lists_here(e) {
        return Vec::new();
    }
    let mut out: Vec<(u32, u32)> = Vec::new();
    let pick = |a: &u32| addrs.map_or(true, |s| s.contains(a));
    if let Some(v) = served_mut(&mut e.own_view) {
        out = v
            .lists
            .iter()
            .filter(|(a, _)| pick(a))
            .map(|(a, w)| (*a, *w))
            .collect();
        for (a, _) in &out {
            v.lists.remove(a);
        }
    } else {
        PACKET.with(|p| {
            if let Some(p) = p.borrow_mut().as_mut() {
                out = p
                    .lists
                    .iter()
                    .filter(|(a, _)| pick(a))
                    .map(|(a, w)| (*a, *w))
                    .collect();
                for (a, _) in &out {
                    p.lists.remove(a);
                }
            }
        });
    }
    out.sort_unstable();
    for (a, w) in &out {
        e.set_global(*a, *w);
    }
    out.into_iter().map(|(a, _)| a).collect()
}

/// [`lists_out`]'s words taken again: their values now into the drawing side's packet, zeros in guest memory.
pub fn lists_in(e: &mut Engine, addrs: &[u32]) {
    let words: Vec<(u32, u32)> = addrs.iter().map(|a| (*a, e.global::<u32>(*a))).collect();
    if let Some(v) = served_mut(&mut e.own_view) {
        v.lists.extend(words.iter().copied());
    } else {
        PACKET.with(|p| {
            if let Some(p) = p.borrow_mut().as_mut() {
                p.lists.extend(words.iter().copied());
            }
        });
    }
    for (a, _) in words {
        e.set_global(a, 0u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block of `words` guest words holding 1, 2, 3, ..
    fn block(e: &mut Engine, words: u32) -> u32 {
        let at = e.mem.alloc(4 * words);
        for i in 0..words {
            e.mem.set_u32(at + 4 * i, i + 1);
        }
        at
    }

    #[test]
    fn the_packet_answers_its_copy_and_counts_what_it_lacks() {
        let mut e = Engine::new();
        let at = block(&mut e, 6);
        assert!(begin(&e));
        assert!(!begin(&e));
        take(&e, at, 4, 12);
        take(&e, at, 0, 16);
        take(&e, at, 8, 12);
        e.mem.set_u32(at + 4, 0x55);
        assert_eq!(
            [word(&e, at, 0), word(&e, at, 4), word(&e, at, 12)],
            [1, 2, 4]
        );
        assert_eq!(half(&e, at, 6), 0);
        assert_eq!(byte(&e, at, 8), 3);
        assert_eq!(words::<3>(&e, at, 4), [2, 3, 4]);
        // Beyond the copied ranges: guest memory, and a miss each.
        assert_eq!(word(&e, at, 16), 5);
        assert_eq!(word(&e, at, 14), 0x0005_0000);
        let seen = image(&mut e, at, 4, 12, |e, base| {
            (e.mem.u32(base + 4), e.mem.u32(base + 8), base == at)
        });
        assert_eq!(seen, (2, 3, false));
        let report = end(&mut e);
        assert_eq!(
            report,
            Report {
                misses: 2,
                reads: 9,
                written: 0,
                ranges: 3,
                bytes: 16,
                held: 0,
                owned: 0,
                kept_main: 0
            }
        );
        assert!(!is_open(&e));
        assert_eq!(word(&e, at, 4), 0x55);
        assert_eq!(image(&mut e, at, 4, 12, |_, base| base), at);
    }

    #[test]
    fn poison_scribbles_the_copied_bytes_and_puts_back_the_unwritten() {
        let mut e = Engine::new();
        let at = block(&mut e, 4);
        let other = block(&mut e, 2);
        assert!(begin(&e));
        take(&e, at, 0, 8);
        take(&e, at, 12, 16);
        take(&e, other, 0, 4);
        poison(&mut e, SCRIBBLE);
        take(&e, other, 4, 8);
        let s = u32::from_le_bytes([SCRIBBLE; 4]);
        assert_eq!(
            [
                e.mem.u32(at),
                e.mem.u32(at + 4),
                e.mem.u32(at + 8),
                e.mem.u32(at + 12),
                e.mem.u32(other),
                e.mem.u32(other + 4)
            ],
            [s, s, 3, s, s, 2]
        );
        assert_eq!(
            [word(&e, at, 0), word(&e, at, 12), word(&e, other, 0)],
            [1, 4, 1]
        );
        // The draw writes one copied word.
        e.mem.set_u32(at + 4, 0x77);
        let report = end(&mut e);
        assert_eq!((report.written, report.misses), (4, 0));
        assert_eq!(
            [
                e.mem.u32(at),
                e.mem.u32(at + 4),
                e.mem.u32(at + 8),
                e.mem.u32(at + 12),
                e.mem.u32(other),
                e.mem.u32(other + 4)
            ],
            [1, 0x77, 3, 4, 1, 2]
        );
    }

    /// Between the halves the packet is not read and the scribble is off guest memory; a write there is main's
    /// (kept, counted with POISON); the second half reads the copy and sees the scribble again.
    #[test]
    fn a_paused_packet_answers_guest_memory_and_puts_the_scribble_back() {
        let mut e = Engine::new();
        let at = block(&mut e, 2);
        assert!(begin(&e));
        take(&e, at, 0, 8);
        poison(&mut e, SCRIBBLE);
        let s = u32::from_le_bytes([SCRIBBLE; 4]);
        assert_eq!([word(&e, at, 0), e.mem.u32(at)], [1, s]);
        pause(&mut e);
        pause(&mut e);
        assert_eq!(
            [word(&e, at, 0), e.mem.u32(at), e.mem.u32(at + 4)],
            [1, 1, 2]
        );
        e.mem.set_u32(at + 4, 0x99);
        assert_eq!(word(&e, at, 4), 0x99);
        resume(&mut e);
        assert_eq!([word(&e, at, 0), word(&e, at, 4), e.mem.u32(at)], [1, 2, s]);
        let report = end(&mut e);
        assert_eq!((report.misses, report.written), (0, 0));
        assert_eq!([e.mem.u32(at), e.mem.u32(at + 4)], [1, 0x99]);
        // Paused at the end: nothing scribbled is left.
        assert!(begin(&e));
        take(&e, at, 0, 4);
        poison(&mut e, SCRIBBLE_OTHER);
        e.mem.set_u32(at, 0x55);
        pause(&mut e);
        let report = end(&mut e);
        assert_eq!((report.written, e.mem.u32(at)), (4, 0x55));
    }

    /// A read the cut did not take stops a POISON packet.
    #[test]
    #[should_panic(expected = "which the cut did not take")]
    fn a_miss_stops_a_poison_packet() {
        let mut e = Engine::new();
        let at = block(&mut e, 2);
        assert!(begin(&e));
        take(&e, at, 0, 4);
        poison(&mut e, SCRIBBLE);
        let _ = word(&e, at, 4);
    }

    /// The copy and its counts as [`the_packet_answers_its_copy_and_counts_what_it_lacks`] has them, with the
    /// packet's ranges served on the engine (`serve`): every answer, the misses and the report the same.
    #[test]
    fn a_served_packet_answers_as_the_packet_does() {
        let mut e = Engine::new();
        let at = block(&mut e, 6);
        assert!(begin(&e));
        take(&e, at, 4, 12);
        take(&e, at, 0, 16);
        take(&e, at, 8, 12);
        serve(&mut e);
        assert!(e.own_view.is_some());
        // Served twice: the second does nothing (the view keeps its bytes).
        serve(&mut e);
        e.mem.set_u32(at + 4, 0x55);
        assert_eq!(
            [word(&e, at, 0), word(&e, at, 4), word(&e, at, 12)],
            [1, 2, 4]
        );
        assert_eq!(half(&e, at, 6), 0);
        assert_eq!(byte(&e, at, 8), 3);
        assert_eq!(words::<3>(&e, at, 4), [2, 3, 4]);
        assert_eq!(word(&e, at, 16), 5);
        assert_eq!(word(&e, at, 14), 0x0005_0000);
        let seen = image(&mut e, at, 4, 12, |e, base| {
            (e.mem.u32(base + 4), e.mem.u32(base + 8), base == at)
        });
        assert_eq!(seen, (2, 3, false));
        let spanned = span(&e, at, 0, 16);
        assert_eq!(spanned.word(&e, 4), 2);
        let report = end(&mut e);
        assert!(e.own_view.is_none(), "the join took the view back");
        assert_eq!(
            report,
            Report {
                misses: 2,
                reads: 10,
                written: 0,
                ranges: 3,
                bytes: 16,
                held: 0,
                owned: 0,
                kept_main: 0
            }
        );
        assert!(!is_open(&e));
    }

    /// Own state served on the engine: the draw's writes go into the view, guest memory keeps main's, a pause puts
    /// them back into the packet (and guest reads in between), and the join's rule sees the draw's values.
    #[test]
    fn served_own_state_is_the_draws_and_goes_back_at_the_end() {
        for scribble in [None, Some(SCRIBBLE)] {
            let mut e = Engine::new();
            let at = block(&mut e, 3);
            assert!(begin(&e));
            own(&e, at, 0, 8);
            take(&e, at, 8, 12);
            if let Some(s) = scribble {
                poison(&mut e, s);
            }
            serve(&mut e);
            set_word(&mut e, at, 0, 0x55);
            set_half(&mut e, at, 6, 0x7777);
            set_byte(&mut e, at, 8, 0x99);
            assert_eq!(
                [own_word(&e, at, 0), own_word(&e, at, 4), word(&e, at, 8)],
                [0x55, 0x7777_0002, 3]
            );
            assert!(owns(&e, at, 8) && !owns(&e, at + 4, 8));
            assert_eq!(own_bytes(&e, at, 4), vec![0x55, 0, 0, 0]);
            // A helper writing through a pointer to the own state writes the view.
            own_image(&mut e, at + 4, 4, |e, p| e.mem.set_u32(p, 0x1234_5678));
            assert_eq!(own_word(&e, at, 4), 0x1234_5678);
            let read = own_image_read(&mut e, at, 8, |e, p| e.mem.u32(p + 4));
            assert_eq!(read, 0x1234_5678);
            if scribble.is_none() {
                assert_eq!([e.mem.u32(at), e.mem.u32(at + 4)], [1, 2]);
                // Another game thread on the same engine while the half is served (stepped threads take turns on
                // one engine): it reads guest memory and cannot take the view (its writes are main's, noted as
                // main's writes are: `own_store`).
                struct Stepped(Engine);
                // SAFETY: the engine is used by one thread at a time (moved to the other thread and back by the
                // join, as the stepped threads hand it over), and nothing of it is kept on this thread meanwhile.
                unsafe impl Send for Stepped {}
                let moved = Stepped(e);
                let back = std::thread::spawn(move || {
                    // The whole wrapper moves (a pattern would move only its field, the engine).
                    let moved = moved;
                    let mut e = moved.0;
                    assert_eq!(own_word(&e, at, 0), 1, "another thread: guest memory");
                    assert_eq!(own_bytes(&e, at, 4), vec![1, 0, 0, 0]);
                    assert!(!owns(&e, at, 8), "another thread owns nothing");
                    unserve(&mut e);
                    assert!(
                        e.own_view.is_some(),
                        "only the serving thread takes the view back"
                    );
                    Stepped(e)
                })
                .join()
                .unwrap()
                .0;
                assert_eq!(
                    own_word(&back, at, 0),
                    0x55,
                    "the serving thread: the draw's"
                );
                assert_eq!(own_word(&back, at, 4), 0x1234_5678);
                e = back;
            }
            pause(&mut e);
            assert!(e.own_view.is_none(), "a pause takes the view back");
            if scribble.is_none() {
                assert_eq!(own_word(&e, at, 0), 1, "between the halves: main's");
            }
            resume(&mut e);
            serve(&mut e);
            assert_eq!(
                own_word(&e, at, 0),
                0x55,
                "the second half: the draw's again"
            );
            let report = end(&mut e);
            assert_eq!(
                [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
                [0x55, 0x1234_5678, 0x99]
            );
            assert_eq!(report.written, u64::from(scribble.is_some()));
            assert_eq!((report.owned, report.kept_main), (8, 0));
        }
    }

    #[test]
    fn a_served_packet_is_never_detached() {
        let mut e = Engine::new();
        let at = block(&mut e, 2);
        assert!(begin(&e));
        own(&e, at, 0, 8);
        serve(&mut e);
        let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| detach(&e).is_some()));
        let stop = stop.expect_err("a stop");
        let why = stop
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| stop.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        assert!(why.contains("unserve first"), "{why}");
        unserve(&mut e);
        let back = detach(&e).expect("detached once the view is back");
        assert!(attach(back).is_ok());
        assert_eq!(end(&mut e).owned, 8);
    }

    /// The world draw's list words handed over at the cut: guest memory holds zeros (a zero word is not taken), the
    /// drawing side reads and writes the packet's (served or not), main beside it reads guest memory, a callee by address
    /// gets the words back in place and gives them back, and the join puts them back in guest memory.
    #[test]
    fn the_list_words_go_with_the_packet_and_come_back_at_the_join() {
        use super::super::detach;
        let mut e = Engine::new();
        let at = block(&mut e, 4);
        e.mem.set_u32(at + 12, 0);
        assert!(begin(&e));
        take_lists(&mut e, &[at, at + 4, at + 12]);
        assert_eq!(
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
            [0, 0, 3]
        );
        assert!(lists_here(&e));
        assert_eq!(list_word(&e, at + 12), None, "a zero word is not taken");
        assert_eq!([detach::word(&e, at), detach::word(&e, at + 4)], [1, 2]);
        detach::set_word(&mut e, at, 0x11);
        detach::set_word(&mut e, at + 8, 0x33);
        assert_eq!([e.mem.u32(at), e.mem.u32(at + 8)], [0, 0x33]);
        serve(&mut e);
        assert_eq!(detach::word(&e, at), 0x11, "served on the engine");
        detach::set_word(&mut e, at + 4, 0x22);
        // A callee by address: the named word in place for it, taken again after.
        let seen = detach::suspended_for(
            &mut e,
            |_| vec![at + 4],
            |e| {
                let seen = e.mem.u32(at + 4);
                e.mem.set_u32(at + 4, 0x44);
                seen
            },
        );
        assert_eq!((seen, e.mem.u32(at + 4)), (0x22, 0));
        assert_eq!(detach::word(&e, at + 4), 0x44);
        // A callee by address that reads every list word in place, with a nested one naming one word inside it: all
        // words in guest memory for the outer (the nested one finds them there and takes nothing), all taken again
        // after it with what the callees left.
        let inner = detach::suspended(&mut e, |e| {
            assert_eq!(
                [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
                [0x11, 0x44, 0x33]
            );
            assert!(!lists_here(e), "every word in place");
            let seen = detach::suspended_for(
                e,
                |_| vec![at + 4],
                |e| {
                    let seen = e.mem.u32(at + 4);
                    e.mem.set_u32(at + 4, 0x66);
                    seen
                },
            );
            assert_eq!(
                e.mem.u32(at + 4),
                0x66,
                "the nested callee's word stays in place"
            );
            e.mem.set_u32(at, 0x77);
            seen
        });
        assert_eq!(inner, 0x44);
        assert_eq!(
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
            [0, 0, 0x33],
            "taken again (the word never taken stays in guest memory)"
        );
        assert_eq!(
            [detach::word(&e, at), detach::word(&e, at + 4)],
            [0x77, 0x66]
        );
        unserve(&mut e);
        // Main beside the drawing side's packet reads guest memory, and a word main writes meanwhile is main's.
        let packet = detach(&e).expect("handed over");
        assert_eq!(detach::word(&e, at), 0);
        assert!(!lists_here(&e));
        e.mem.set_u32(at + 4, 0x55);
        assert!(attach(packet).is_ok());
        let _ = end(&mut e);
        assert_eq!(
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
            [0x77, 0x55, 0x33],
            "the join puts the drawing side's words back where main wrote nothing"
        );
        assert_eq!(detach::word(&e, at), 0x77, "guest memory after the join");
    }

    #[test]
    fn own_state_is_written_through_the_packet_and_goes_back_at_the_end() {
        for scribble in [None, Some(SCRIBBLE)] {
            let mut e = Engine::new();
            let at = block(&mut e, 3);
            assert!(begin(&e));
            own(&e, at, 0, 8);
            take(&e, at, 8, 12);
            if let Some(s) = scribble {
                poison(&mut e, s);
            }
            set_word(&mut e, at, 0, 0x55);
            set_half(&mut e, at, 6, 0x7777);
            // A taken (read-only) range is not written through the packet.
            set_byte(&mut e, at, 8, 0x99);
            assert_eq!(
                [own_word(&e, at, 0), own_word(&e, at, 4), word(&e, at, 8)],
                [0x55, 0x7777_0002, 3]
            );
            assert_eq!(own_at::<u16>(&e, at + 6), 0x7777);
            if scribble.is_none() {
                assert_eq!([e.mem.u32(at), e.mem.u32(at + 4)], [1, 2]);
            }
            let report = end(&mut e);
            assert_eq!(
                [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
                [0x55, 0x7777_0002, 0x99]
            );
            assert_eq!(report.written, u64::from(scribble.is_some()));
            assert_eq!((report.owned, report.kept_main), (8, 0));
        }
    }

    #[test]
    fn own_blocks_do_not_overlap_and_cover_by_address() {
        let mut e = Engine::new();
        let at = block(&mut e, 4);
        assert!(begin(&e));
        own(&e, at, 4, 8);
        // The same bytes through another base, and a wider range: only the parts not owned yet are taken.
        own(&e, at + 4, 0, 4);
        own(&e, at, 0, 12);
        e.mem.set_u32(at + 4, 0x66);
        set_own_at(&mut e, at + 8, 0x77u32);
        assert_eq!(
            [
                own_word(&e, at, 0),
                own_word(&e, at + 4, 0),
                own_word(&e, at, 8)
            ],
            [1, 2, 0x77]
        );
        // Across two blocks: guest memory.
        assert_eq!(own_at::<u32>(&e, at + 2), e.mem.u32(at + 2));
        let report = end(&mut e);
        assert_eq!(report.owned, 12);
        // +4 was changed by main (0x66 against 2 at the cut): main's value stays.
        assert_eq!(
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
            [1, 0x66, 0x77]
        );
        assert_eq!(report.kept_main, 1);
    }

    /// The join's rule against the device model's (TWO-PRODUCERS.md): main sets a state A -> B -> A after the cut
    /// while the draw sets it to C; main set it, so main's A stays, though the byte equals its value at the cut.
    /// A byte main neither set nor changed takes the draw's value.
    #[test]
    fn the_join_keeps_what_main_set_after_the_cut_even_set_back() {
        let mut e = Engine::new();
        let at = block(&mut e, 3);
        let (a, b, c) = (1u32, 0xbbu32, 0xccu32);
        assert!(begin(&e));
        own(&e, at, 0, 12);
        // The draw (holding the packet) sets all three words.
        set_own_at(&mut e, at, c);
        set_own_at(&mut e, at + 4, c);
        set_word(&mut e, at, 8, c);
        let packet = detach(&e).expect("open");
        assert!(active());
        // Main, holding no packet: the first word A -> B -> A through the setter, the second an equal value.
        set_own_at(&mut e, at, b);
        set_own_at(&mut e, at, a);
        set_own_at(&mut e, at + 4, 2u32);
        assert_eq!(own_word(&e, at, 0), a);
        assert!(attach(packet).is_ok());
        let report = end(&mut e);
        assert_eq!(
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
            [a, 2, c]
        );
        assert_eq!(report.kept_main, 8);
        // A later packet does not see the earlier marks.
        assert!(begin(&e));
        own(&e, at, 0, 4);
        set_own_at(&mut e, at, c);
        end(&mut e);
        assert_eq!(e.mem.u32(at), c);
    }

    /// The look-up caches: a read between two takes of one object (the second puts a new first range in front)
    /// still finds both ranges, two objects read in turn each answer their own copy, and a thread holding no
    /// packet reads guest memory while another's is open.
    #[test]
    fn the_look_up_caches_follow_new_ranges_and_objects() {
        let mut e = Engine::new();
        let at = block(&mut e, 4);
        let other = block(&mut e, 4);
        assert!(begin(&e));
        take(&e, at, 0, 4);
        assert_eq!(word(&e, at, 0), 1);
        take(&e, at, 8, 12);
        take(&e, other, 0, 16);
        e.mem.set_u32(at, 0x11);
        e.mem.set_u32(at + 8, 0x33);
        e.mem.set_u32(other + 4, 0x22);
        assert_eq!(
            [
                word(&e, at, 8),
                word(&e, other, 4),
                word(&e, at, 0),
                word(&e, other, 4)
            ],
            [3, 2, 1, 2]
        );
        // A miss on the cached object's other offsets is still a miss.
        assert_eq!(word(&e, at, 4), 2);
        let packet = detach(&e).expect("open");
        assert!(active());
        assert!(!held());
        assert_eq!(word(&e, at, 0), 0x11);
        assert!(attach(packet).is_ok());
        assert!(held());
        assert_eq!(word(&e, at, 0), 1);
        let report = end(&mut e);
        assert_eq!(report.misses, 1);
        assert!(!held());
    }

    /// Main's repeated writes are noted once, and noted again in the next packet: each join keeps main's value.
    #[test]
    fn a_repeated_write_by_main_is_noted_in_every_packet() {
        let mut e = Engine::new();
        let at = block(&mut e, 1);
        for round in 0..2u32 {
            assert!(begin(&e));
            own(&e, at, 0, 4);
            set_own_at(&mut e, at, 0xd0 + round);
            let packet = detach(&e).expect("open");
            set_own_at(&mut e, at, 0x50);
            set_own_at(&mut e, at, 0x50);
            assert!(attach(packet).is_ok());
            let report = end(&mut e);
            assert_eq!(e.mem.u32(at), 0x50, "round {round}");
            assert_eq!(report.kept_main, 4, "round {round}");
        }
    }

    /// Between the halves the own state is main's: what runs there reads guest memory (main's values, the scribble
    /// off with POISON) and its writes through the setters are main's at the join.
    #[test]
    fn between_the_halves_the_own_state_is_mains() {
        for scribble in [None, Some(SCRIBBLE)] {
            let mut e = Engine::new();
            let at = block(&mut e, 2);
            assert!(begin(&e));
            own(&e, at, 0, 8);
            if let Some(s) = scribble {
                poison(&mut e, s);
            }
            set_own_at(&mut e, at, 0xc1u32);
            set_own_at(&mut e, at + 4, 0xc2u32);
            pause(&mut e);
            assert_eq!([own_word(&e, at, 0), e.mem.u32(at + 4)], [1, 2]);
            // Main sets the first word back to its value at the cut.
            set_own_at(&mut e, at, 1u32);
            resume(&mut e);
            assert_eq!(own_word(&e, at, 0), 0xc1);
            let report = end(&mut e);
            assert_eq!([e.mem.u32(at), e.mem.u32(at + 4)], [1, 0xc2]);
            assert_eq!((report.written, report.kept_main), (0, 4));
        }
    }

    #[test]
    fn with_no_packet_the_accessors_are_guest_memory() {
        let mut e = Engine::new();
        let at = block(&mut e, 3);
        take(&e, at, 0, 12);
        poison(&mut e, SCRIBBLE);
        assert_eq!(
            [
                word(&e, at, 0),
                u32::from(half(&e, at, 4)),
                u32::from(byte(&e, at, 8))
            ],
            [1, 2, 3]
        );
        assert_eq!(words::<2>(&e, at, 4), [2, 3]);
        assert_eq!(end(&mut e), Report::default());
        assert_eq!(e.mem.u32(at), 1);
    }
}
