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
//! Off by default (`NV_DRAW_THREAD` unset); off, the accessors are the guest reads they replace behind one
//! relaxed load.

use engine_core::ptr::Ptr;
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

/// `NV_DRAW_THREAD`, read once; a thread's own mode instead while one is set ([`set_thread_mode`]).
pub fn mode() -> Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    if let Some(mode) = THREAD_MODE.with(Cell::get) {
        return mode;
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

/// The byte POISON fills the packet's guest bytes with (0x6e: a word of them is a float of about 1.8e28 and an
/// address in no block). [`SCRIBBLE_OTHER`] is its complement, for a second run that flips every bit the first
/// left alone.
pub const SCRIBBLE: u8 = 0x6e;
/// [`SCRIBBLE`]'s complement.
pub const SCRIBBLE_OTHER: u8 = !SCRIBBLE;

/// One copied range of an object: its offsets `[from, to)`, where its bytes start in the packet, and the next
/// range of the same object (`u32::MAX` for none).
#[derive(Clone, Copy)]
struct Range {
    from: u32,
    to: u32,
    at: u32,
    next: u32,
}

const NONE: u32 = u32::MAX;

struct Packet {
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
}

impl Packet {
    /// Whether the accessors read this packet for `e`: its run, and not paused.
    #[inline(always)]
    fn serves(&self, e: &Engine) -> bool {
        self.run == run(e) && !self.paused
    }

    /// The range of `object` that holds `[offset, offset + len)`, if one does.
    #[inline(always)]
    fn find(&self, object: u32, offset: u32, len: u32) -> Option<Range> {
        let mut index = *self.objects.get(&object)?;
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
}

/// How many threads hold an open packet: the accessors' one check while none does.
static OPEN: AtomicUsize = AtomicUsize::new(0);

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
        });
        true
    });
    if opened {
        OPEN.fetch_add(1, Relaxed);
    }
    opened
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
        // The parts of [from, to) no range of the object holds.
        let mut parts = vec![(from, to)];
        let first = p.objects.get(&object).copied().unwrap_or(NONE);
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
            let mut copy = vec![0u8; (b - a) as usize];
            // A range not mapped is not taken (a read of it through the packet is a miss).
            if e.mem.try_read(object.wrapping_add(a), &mut copy).is_err() {
                continue;
            }
            let at = u32::try_from(p.bytes.len()).expect("the packet stays under 4 GB");
            p.bytes.extend_from_slice(&copy);
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
        }
    });
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

/// Every guest byte the packet holds saved and set to `scribble`, in the order taken.
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
}

/// The scribbled guest bytes put back, in the reverse order; a byte no longer the scribble (written while the
/// draw ran) is left as written and counted.
fn unscribble_all(e: &mut Engine, p: &Packet, scribble: u8) -> u64 {
    let mut written = 0;
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
    with_packet(e, |e, p| {
        if p.paused {
            return;
        }
        p.paused = true;
        if let Some(scribble) = p.scribble {
            p.written += unscribble_all(e, p, scribble);
        }
    });
}

/// The world draw's second half starts: the packet is read again ([`pause`]), and with POISON scribbled again.
pub fn resume(e: &mut Engine) {
    with_packet(e, |e, p| {
        if !p.paused {
            return;
        }
        p.paused = false;
        if let Some(scribble) = p.scribble {
            scribble_all(e, p, scribble);
        }
    });
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
}

/// Closes this thread's packet: POISON's scribbled bytes go back (in the reverse order, those written meanwhile
/// left as written), then the held references are given back (`NiRefObject::DecRefCount`). The default report
/// when none is open.
pub fn end(e: &mut Engine) -> Report {
    if OPEN.load(Relaxed) == 0 {
        return Report::default();
    }
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
    OPEN.fetch_sub(1, Relaxed);
    let mut written = packet.written;
    if let Some(scribble) = packet.scribble {
        if !packet.paused {
            written += unscribble_all(e, &packet, scribble);
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
    }
}

/// The `N` bytes at `object + offset` as the open packet holds them: `None` with no packet open for `e`'s run,
/// and `None` counted as a miss when the packet holds no range of the object that covers them.
#[inline(always)]
fn copied<const N: usize>(e: &Engine, object: u32, offset: u32) -> Option<[u8; N]> {
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
    if OPEN.load(Relaxed) != 0 {
        if let Some(bytes) = copied::<4>(e, object, offset) {
            return u32::from_le_bytes(bytes);
        }
    }
    e.global::<u32>(object.wrapping_add(offset))
}

/// The 16-bit half at `object + offset`, as [`word`].
#[inline(always)]
pub fn half(e: &Engine, object: u32, offset: u32) -> u16 {
    if OPEN.load(Relaxed) != 0 {
        if let Some(bytes) = copied::<2>(e, object, offset) {
            return u16::from_le_bytes(bytes);
        }
    }
    e.global::<u16>(object.wrapping_add(offset))
}

/// The byte at `object + offset`, as [`word`].
#[inline(always)]
pub fn byte(e: &Engine, object: u32, offset: u32) -> u8 {
    if OPEN.load(Relaxed) != 0 {
        if let Some([b]) = copied::<1>(e, object, offset) {
            return b;
        }
    }
    e.global::<u8>(object.wrapping_add(offset))
}

/// `N` words from `object + offset` on, as [`word`] (one look-up for all of them).
#[inline(always)]
pub fn words<const N: usize>(e: &Engine, object: u32, offset: u32) -> [u32; N] {
    if OPEN.load(Relaxed) != 0 {
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
    let copy = if OPEN.load(Relaxed) == 0 || object == 0 || from >= to {
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
    if OPEN.load(Relaxed) == 0 || from >= to {
        return f(e, object);
    }
    let held = PACKET.with(|p| {
        let p = p.borrow();
        let p = p.as_ref()?;
        if !p.serves(e) {
            return None;
        }
        let mut bytes = vec![0u8; (to - from) as usize];
        p.read_or_miss(object, from, &mut bytes).then_some(bytes)
    });
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
                held: 0
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
