//! The world's draw on a thread of its own (step 2) in the device model: two producers
//! (`measure/scope-1111/PLAN.md` piece 5; the interface agreed with the draw agent in
//! `measure/typed-draw/TWO-PRODUCERS.md`). Off until the drawing thread (piece 8) posts the draw.
//!
//! Main makes a cut at the world draw ([`draw_cut`]): its device state (render, stage and
//! sampler states, textures, transforms, viewport, scissor, clip planes, render targets and depth
//! surface, material and lights, streams, indices, declaration and FVF, shaders and their
//! constants) is copied for the drawing side, and the command stream is cut
//! (`d3d9_stream::Stream::cut`). From then on a call from the drawing helper
//! (`engine_core::real_threads::on_draw_helper`) reads and changes the drawing side's copy and
//! records into the cut's slot; main's calls keep main's state and record after the slot. Both
//! take the device lock, one call at a time.
//!
//! - What the drawing side's copy binds it holds, as main's state holds what it binds (the holds
//!   are counted at the cut), so main letting go of an object does not end it while the drawing
//!   side draws with it.
//! - The drawing side's buffer locks hand out memory of its own, filled with the buffer's bytes
//!   and its own earlier writes in the cut; its uploads go into the slot; the blocks are freed at
//!   the join, and its writes become the buffer's bytes there (except where main locked the
//!   buffer after the cut, for writing: main's bytes come later in the serial order).
//! - Buffers both sides touch in one cut. The serial order is the drawing side's locks, then
//!   main's after the cut. How the game locks decides what matters: its buffer manager's lock
//!   (00e8bff0) locks the buffer and hands out its own scratch block, copying the buffer's bytes
//!   into it only when the flags have neither DISCARD nor NOOVERWRITE; its unlock (00e8c0d0) copies
//!   the whole scratch range back and unlocks. So a lock with DISCARD or NOOVERWRITE reads nothing
//!   from the buffer and writes the whole range; only a lock without them reads the bytes.
//!   - Main locks, without DISCARD or NOOVERWRITE, a buffer the drawing side has locked or written
//!     in this cut: main reads bytes the drawing side wrote earlier in the serial order, so main's
//!     lock waits for the drawing job, as a sync call does, and the drawing side's bytes become the
//!     buffer's first. Counted, with a note the first time in a run. With DISCARD or NOOVERWRITE
//!     it reads nothing and does not wait; the join keeps main's bytes in its range.
//!   - The drawing side locks, without DISCARD or NOOVERWRITE, bytes main locked after the cut: it
//!     would read main's later bytes, and it cannot wait for main. With piece 4 in this does not
//!     happen: the world draw's locks go through the manager into the drawing side's own scratch,
//!     vertex staging and dynamic geometry blocks (per side; `measure/smooth-200/PROBE.md`), and
//!     a world-draw lock that reads the buffer is of the world's own buffers, which the frame's
//!     tail does not lock. A debug assertion in the drawing side's lock catches it. With DISCARD
//!     or NOOVERWRITE the drawing side reads nothing, so the bytes its block starts from do not
//!     matter.
//! - [`draw_finish`] is the one call for main's waits (before a reset, a save, a load and piece
//!   8's other wait points): the job joined, then the join. Reset with a cut open does it itself.
//! - At the end of the process the job is finished first (`crate::run_exit_hooks`,
//!   `crate::end_process`), within a time limit, so the device and the stream never go while it
//!   is inside a call, and the process never hangs at exit.
//! - Creations on the drawing side are made as on main (counted, with a note the first time); a
//!   release there that would end an object leaves it to the join.
//! - Present's freeing of the frame's unlocked staging blocks waits for the join while a cut is
//!   open.
//! - A sync call of main's while the cut is open (a read-back, a query read that waits) first
//!   waits for the drawing job (`engine_core::real_threads::wait_draw`), with the device lock given
//!   back: its answer comes after the slot's commands, which the drawing side records under the
//!   lock.
//!
//! [`draw_join`] makes main's state the serial order's: for each state main set after the cut,
//! main's value; for every other, the drawing side's. The holds follow.

use crate::Engine;
#[cfg(test)]
use std::cell::Cell;

/// A piece of a device's state, for what main set after the cut ([`DrawSide::main_set`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Key {
    Render(u32),
    Stage(u32),
    Sampler(u32),
    Texture(u32),
    Transform(u32),
    Viewport,
    Scissor,
    ClipPlane(u32),
    InScene,
    Material,
    Light(u32),
    LightEnable(u32),
    SoftwareVertexProcessing,
    GammaRamp,
    RenderTarget(u32),
    DepthStencil,
    Stream(u32),
    Indices,
    Declaration,
    Fvf,
    VertexShader,
    PixelShader,
    DrawnFromMemory,
}

/// The open cut ([`draw_cut`]), in the device state (`d3d9device::Objects`).
#[derive(Debug, Default)]
pub(crate) struct DrawSide {
    /// The device of the open cut; `None` without one.
    pub(super) device: Option<u32>,
    /// The cuts made so far.
    pub(super) cuts: u64,
    /// What main set since the cut.
    pub(super) main_set: std::collections::HashSet<Key>,
    /// The constant registers main set since the cut, a bit each by their place in the bindings'
    /// table (`kind * CONSTANT_SLOTS + register`).
    pub(super) main_constants: Vec<u64>,
    /// Objects the drawing side made in this cut.
    pub(super) creations: u32,
    /// Objects a release on the drawing side would have ended: ended at the join, if still unheld.
    pub(super) ending: Vec<u32>,
    /// The drawing side's lock memory: its open locks by buffer (block, offset, size), innermost
    /// last, and every block, freed at the join.
    pub(super) locks: engine_core::AddrMap<u32, Vec<(u32, u32, u32)>>,
    pub(super) blocks: Vec<u32>,
    /// What the drawing side wrote into buffers, by buffer: each unlocked range's offset and
    /// bytes, in unlock order. Its later locks start from them; the join writes them into the
    /// buffers' bytes.
    pub(super) written: engine_core::AddrMap<u32, Vec<(u32, Vec<u8>)>>,
    /// The ranges (offset, size) main locked in buffers after the cut: main's bytes there come
    /// after the drawing side's in the serial order, so the join leaves them.
    pub(super) main_locked: engine_core::AddrMap<u32, Vec<(u32, u32)>>,
    /// Main presented during the cut: the frame's unlocked staging blocks are freed at the join.
    pub(super) presented: bool,
    /// The note about creations is printed once a run.
    pub(super) noted: bool,
}

#[cfg(test)]
thread_local! {
    /// Tests: this thread calls as the drawing side, without the drawing helper.
    static AS_DRAWING_SIDE: Cell<bool> = const { Cell::new(false) };
}

/// Whether this OS thread calls as the drawing side: the drawing helper (or a test standing in
/// for it).
#[inline]
pub(super) fn drawing_thread() -> bool {
    #[cfg(test)]
    if AS_DRAWING_SIDE.with(Cell::get) {
        return true;
    }
    engine_core::real_threads::on_draw_helper()
}

/// Whether a call with `side` open comes from the drawing side.
#[inline]
pub(super) fn drawing_side(side: &DrawSide) -> bool {
    side.device.is_some() && drawing_thread()
}

/// The cuts open in the process ([`draw_cut`] to [`draw_join`]): main's setters note what they set only
/// while one is, so without the drawing thread a setter pays one load ([`any_cut`]).
static OPEN_CUTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Whether a cut is open anywhere ([`OPEN_CUTS`]): the setters' test before they note anything.
#[inline]
pub(super) fn any_cut() -> bool {
    OPEN_CUTS.load(std::sync::atomic::Ordering::Relaxed) != 0
}

/// Notes that main set `key` while a cut is open (nothing otherwise).
#[inline]
pub(super) fn main_set(side: &mut DrawSide, key: Key) {
    if side.device.is_some() && !drawing_thread() {
        side.main_set.insert(key);
    }
}

/// Notes that main set the constant registers `first..first + count` (by their place in the
/// bindings' table) while a cut is open.
#[inline]
pub(super) fn main_set_constants(side: &mut DrawSide, first: usize, count: usize) {
    if side.device.is_none() || drawing_thread() {
        return;
    }
    for slot in first..first + count {
        if side.main_constants.len() <= slot / 64 {
            side.main_constants.resize(slot / 64 + 1, 0);
        }
        side.main_constants[slot / 64] |= 1 << (slot % 64);
    }
}

/// Tests: `f` called as the drawing side.
#[cfg(test)]
pub(crate) fn as_drawing_side<R>(f: impl FnOnce() -> R) -> R {
    AS_DRAWING_SIDE.with(|d| d.set(true));
    struct Back;
    impl Drop for Back {
        fn drop(&mut self) {
            AS_DRAWING_SIDE.with(|d| d.set(false));
        }
    }
    let _back = Back;
    f()
}

fn side(e: &mut Engine) -> &mut DrawSide {
    &mut device!(e).device_objects.draw
}

/// Main, at the world draw: the cut. The drawing side's state is main's for `device` from here,
/// and the command stream is cut. False (nothing done, the draw stays on main) while a cut is
/// open, from the drawing side, or for a device the layer does not have.
pub fn draw_cut(e: &mut Engine, device: u32) -> bool {
    let _device = engine_core::real_threads::device_guard(e);
    if drawing_thread() || side(e).device.is_some() || !super::d3d9device::has_device(e, device) {
        return false;
    }
    let cuts = side(e).cuts + 1;
    *side(e) = DrawSide {
        device: Some(device),
        cuts,
        noted: side(e).noted,
        ..DrawSide::default()
    };
    super::d3d9device::copy_for_drawing(e, device);
    super::d3d9resource::copy_for_drawing(e, device);
    OPEN_CUTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    #[cfg(windows)]
    super::d3d9real::cut(e, device);
    true
}

/// The drawing side, at the end of its draw: its commands are complete (main's after the cut
/// follow on the backend). Nothing without an open cut.
pub fn draw_close(e: &mut Engine) {
    let _device = engine_core::real_threads::device_guard(e);
    #[cfg(windows)]
    super::d3d9real::close(e);
    let _ = e;
}

/// Main, once the drawing job is joined: main's state becomes the serial order's (main's value
/// for each state it set after the cut, the drawing side's for every other), the drawing side's
/// lock memory is freed, the objects its releases left are ended if unheld, and what Present
/// waited for is done. Nothing without an open cut.
pub fn draw_join(e: &mut Engine) {
    let _device = engine_core::real_threads::device_guard(e);
    let Some(device) = side(e).device else {
        return;
    };
    #[cfg(windows)]
    {
        super::d3d9real::close(e);
        super::d3d9real::settle(e);
    }
    let set = std::mem::take(&mut side(e).main_set);
    let constants = std::mem::take(&mut side(e).main_constants);
    super::d3d9device::merge_drawing(e, device, &set);
    super::d3d9resource::merge_drawing(e, device, &set, &constants);
    let written = std::mem::take(&mut side(e).written);
    let main_locked = std::mem::take(&mut side(e).main_locked);
    super::d3d9resource::merge_buffer_bytes(e, written, &main_locked);
    // The objects the drawing side's releases left, ended now if nothing holds them (on main:
    // the cut is still open until below, so they are ended here, not deferred again).
    let ending = std::mem::take(&mut side(e).ending);
    let device_open = side(e).device.take();
    for object in ending {
        if !super::d3d9resource::end_if_unheld(e, object) {
            super::d3d9device::end_if_unheld(e, object);
        }
    }
    side(e).device = device_open;
    for block in std::mem::take(&mut side(e).blocks) {
        e.mem.free(block);
    }
    let presented = side(e).presented;
    let (cuts, noted) = (side(e).cuts, side(e).noted);
    *side(e) = DrawSide {
        cuts,
        noted,
        ..DrawSide::default()
    };
    OPEN_CUTS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    if presented {
        super::d3d9device::free_frame_staging(e);
    }
}

/// Main's wait for the drawing thread (piece 8's wait points: before 008746d0, 00874920, 00875e40
/// and 006426a0, a device reset, a save, a load): the drawing job joined
/// (`engine_core::real_threads::join_draw`, the device and kernel locks given back while it waits),
/// then [`draw_join`]. A panic of the job's is resumed after the join, so the device state is
/// whole either way. Nothing without a posted job or an open cut.
pub fn draw_finish(e: &mut Engine) {
    if drawing_thread() {
        // Not from the drawing side: it would wait for itself.
        return;
    }
    let joined = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine_core::real_threads::join_draw(e);
    }));
    draw_join(e);
    if let Err(payload) = joined {
        std::panic::resume_unwind(payload);
    }
}

/// Whether a cut is open (for main: the drawing side may be recording).
pub fn draw_open(e: &mut Engine) -> bool {
    let _device = engine_core::real_threads::device_guard(e);
    side(e).device.is_some()
}

/// The drawing side made an object: counted, with a note the first time in a run.
pub(super) fn created_on_drawing_side(e: &mut Engine, what: &str) {
    let s = side(e);
    s.creations += 1;
    if !s.noted {
        s.noted = true;
        eprintln!(
            "NV_DRAW_THREAD: the drawing side made a {what} (made as on main; counted from here)"
        );
    }
}
