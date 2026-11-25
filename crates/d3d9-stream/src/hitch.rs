//! The hitch log's counts on the drawing side (`host::hitch_log`; the game side's are `engine_core::hitch`): how
//! long main waited for the render thread, and the pipelines and shader modules the backend made, with their time.
//! Each is one relaxed atomic add where the stream already reads the clock (its waits) or where the backend makes a
//! pipeline (milliseconds of work each). The host reads them after each of main's frames ([`snapshot`]).
//!
//! What the render thread and the backend's queue thread spent a frame on: the render thread's time carrying out
//! the game's commands, and both threads' calls into the graphics API that can block ([`Call`]: submits, uploads,
//! resources made, polls, waits for the GPU or for another of the backend's threads), with the longest single call
//! of each ([`take_longest`]). Those calls are timed only on the two threads ([`set_side`]) and only while the hitch
//! log is on ([`set_on`]); each thread adds them up in its own counts and hands them over once a chunk of the game's
//! commands (the render thread) or a job (the queue thread) is done ([`publish`]), before main is told that a
//! `Present` it waits for has been carried out. Main's own thread times none of them.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

/// What is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Count {
    /// Main's wait in `Present` for the render thread (the frames in flight), ns.
    PresentWaitNs,
    /// Main's waits for a reply (a read-back, a query's answer, a lock): ns and how many.
    ReplyWaitNs,
    ReplyWaits,
    /// Main's waits for room in the command queue (the render thread behind), ns.
    QueueFullNs,
    /// Render pipelines the backend made (on any of its threads) and their time (ns), shader modules made and
    /// their time (ns, the conversion included).
    Pipelines,
    PipelineNs,
    Modules,
    ModuleNs,
    /// The render thread's present, split (ns): waiting for the queue thread to finish the last frame's work and
    /// present (`sync`), waiting for the window's next frame (`get_current_texture`: with the Fifo present mode the
    /// wait for a free swap-chain image, so for the display's refresh), and the queue thread's present call itself.
    RenderSyncNs,
    AcquireNs,
    SwapNs,
    /// The sun shadows' caster pipelines made where the render thread waits for them (a newly seen still object's
    /// depth-only pipeline on the render thread; the kinds seen before, made before play on the worker threads) and
    /// their time (ns). Not in [`Count::Pipelines`].
    ShadowPipelines,
    ShadowPipelineNs,
    /// The sun shadows' caster pipelines made on threads of their own while play goes on (a character's; with
    /// `NV_SUN_CASTERS_AHEAD=1` a still object's too) and their time (ns): no wait of the render thread's.
    ShadowPipelinesAhead,
    ShadowPipelineAheadNs,
    /// The render thread's time carrying out the game's commands (ns): its calls and waits below included, the rest
    /// is its own work on the commands.
    RenderBusyNs,
    /// The render thread's idle work between the game's commands (the occlusion counts it polls for), ns: its calls are in
    /// none of the parts below (they are not its time on the commands), only named when one is the longest.
    RenderIdleNs,
    /// The render thread's waits at a cut's mark for the drawing side's commands, ns.
    SlotWaitNs,
    /// The render thread's blocking calls, by [`Part`] (ns; in [`Part`]'s order).
    RenderSubmitNs,
    RenderUploadNs,
    RenderCreateNs,
    RenderPollNs,
    RenderGpuWaitNs,
    RenderThreadWaitNs,
    RenderFinishNs,
    /// The queue thread's time running the jobs the render thread hands it (ns).
    QueueBusyNs,
    /// The queue thread's blocking calls, by [`Part`] (ns; in [`Part`]'s order).
    QueueSubmitNs,
    QueueUploadNs,
    QueueCreateNs,
    QueuePollNs,
    QueueGpuWaitNs,
    QueueThreadWaitNs,
    QueueFinishNs,
}

/// How many counts there are.
pub const COUNTS: usize = Count::QueueFinishNs as usize + 1;

/// The kinds of blocking call a thread's time is split into ([`Call::part`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Part {
    /// Command buffers handed to the GPU's queue (`queue.submit`).
    Submit,
    /// Bytes written into buffers and textures through the queue (`write_buffer`, `write_texture`).
    Upload,
    /// Resources made: textures, buffers, views, bind groups, samplers.
    Create,
    /// The device polled without waiting (finished work let go, read-backs mapped).
    Poll,
    /// Waits for the GPU to finish its work (a read-back that waits).
    GpuWait,
    /// Waits for another of the backend's threads: the render thread's for the queue thread (outside its present's,
    /// [`Count::RenderSyncNs`]), the queue thread's for the threads that finish encoders.
    ThreadWait,
    /// Command encoders finished (the graphics API's checking and the driver's recording).
    Finish,
}

/// How many parts there are.
pub const PARTS: usize = Part::Finish as usize + 1;

/// The parts' names, in [`Part`]'s order.
pub const PART_NAMES: [&str; PARTS] = [
    "submits",
    "uploads",
    "resources made",
    "polls",
    "GPU waits",
    "waits for its other threads",
    "encoders finished",
];

/// One blocking call of the backend's, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Call {
    Submit = 1,
    WriteBuffer,
    WriteTexture,
    CreateTexture,
    CreateBuffer,
    CreateView,
    CreateBindGroup,
    CreateSampler,
    CreateEncoder,
    Poll,
    GpuWait,
    /// The render thread's wait for the queue thread to run what it was handed (outside the present's).
    QueueWait,
    /// The queue thread's wait for a finishing thread's encoder.
    FinishWait,
    Finish,
    /// The present's own waits and call, counted in [`Count::RenderSyncNs`], [`Count::AcquireNs`] and
    /// [`Count::SwapNs`]: named here only when one is the longest call.
    FrameSync,
    Acquire,
    Present,
    /// Pipelines and shader modules made, counted in [`Count::PipelineNs`] and [`Count::ModuleNs`]: the same.
    Pipeline,
    Module,
    /// A sun-shadow caster pipeline made, counted in [`Count::ShadowPipelineNs`]: the same.
    ShadowPipeline,
}

/// The calls in their number's order (from 1).
const CALLS: [Call; 20] = [
    Call::Submit,
    Call::WriteBuffer,
    Call::WriteTexture,
    Call::CreateTexture,
    Call::CreateBuffer,
    Call::CreateView,
    Call::CreateBindGroup,
    Call::CreateSampler,
    Call::CreateEncoder,
    Call::Poll,
    Call::GpuWait,
    Call::QueueWait,
    Call::FinishWait,
    Call::Finish,
    Call::FrameSync,
    Call::Acquire,
    Call::Present,
    Call::Pipeline,
    Call::Module,
    Call::ShadowPipeline,
];

impl Call {
    /// The part of a thread's time the call is counted in; none for the ones counted elsewhere.
    pub fn part(self) -> Option<Part> {
        Some(match self {
            Call::Submit => Part::Submit,
            Call::WriteBuffer | Call::WriteTexture => Part::Upload,
            Call::CreateTexture
            | Call::CreateBuffer
            | Call::CreateView
            | Call::CreateBindGroup
            | Call::CreateSampler
            | Call::CreateEncoder => Part::Create,
            Call::Poll => Part::Poll,
            Call::GpuWait => Part::GpuWait,
            Call::QueueWait | Call::FinishWait => Part::ThreadWait,
            Call::Finish => Part::Finish,
            Call::FrameSync
            | Call::Acquire
            | Call::Present
            | Call::Pipeline
            | Call::Module
            | Call::ShadowPipeline => return None,
        })
    }

    /// The call in plain words.
    pub fn name(self) -> &'static str {
        match self {
            Call::Submit => "a submit",
            Call::WriteBuffer => "a buffer write",
            Call::WriteTexture => "a texture write",
            Call::CreateTexture => "a texture made",
            Call::CreateBuffer => "a buffer made",
            Call::CreateView => "a texture view made",
            Call::CreateBindGroup => "a bind group made",
            Call::CreateSampler => "a sampler made",
            Call::CreateEncoder => "an encoder made",
            Call::Poll => "a poll",
            Call::GpuWait => "a wait for the GPU",
            Call::QueueWait => "a wait for the queue thread",
            Call::FinishWait => "a wait for an encoder being finished",
            Call::Finish => "an encoder finished",
            Call::FrameSync => "the present's wait for the queue thread",
            Call::Acquire => "the wait for the window's frame",
            Call::Present => "the present call",
            Call::Pipeline => "a pipeline made",
            Call::Module => "a shader module made",
            Call::ShadowPipeline => "a sun-shadow caster pipeline made",
        }
    }

    fn from_number(n: u8) -> Option<Call> {
        CALLS.get(usize::from(n).checked_sub(1)?).copied()
    }
}

/// The thread a blocking call is timed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Side {
    /// The render thread (`d3d9_stream`'s, which carries out the game's commands).
    Render = 1,
    /// The backend's queue thread (its submits, uploads and presents in order).
    Queue,
}

impl Side {
    /// The side's first [`Part`] count.
    fn first(self) -> usize {
        match self {
            Side::Render => Count::RenderSubmitNs as usize,
            Side::Queue => Count::QueueSubmitNs as usize,
        }
    }
}

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU64 = AtomicU64::new(0);
static COUNTERS: [AtomicU64; COUNTS] = [ZERO; COUNTS];
/// Each side's longest call since the host last took it ([`pack`]ed).
static LONGEST: [AtomicU64; 2] = [ZERO; 2];
/// Whether the calls are timed (the hitch log is on).
static ON: AtomicBool = AtomicBool::new(false);

/// Adds `n` to `c`.
#[inline]
pub fn add(c: Count, n: u64) {
    COUNTERS[c as usize].fetch_add(n, Relaxed);
}

/// Adds the duration `d` in ns to `c`.
#[inline]
pub fn add_time(c: Count, d: Duration) {
    add(c, nanos(d));
}

fn nanos(d: Duration) -> u64 {
    d.as_nanos().min(u128::from(u64::MAX)) as u64
}

/// Turns the timing of the blocking calls on or off (the host: on with its hitch log).
pub fn set_on(on: bool) {
    ON.store(on, Relaxed);
}

/// Whether the blocking calls are timed now: the hitch log is on and this thread is one of the two
/// ([`set_side`]).
#[inline]
pub fn timing() -> bool {
    ON.load(Relaxed) && THREAD.with(|t| t.side.get() != 0)
}

/// What this thread has timed since it last handed it over.
struct Times {
    /// 0 (not timed), else a [`Side`].
    side: Cell<u8>,
    parts: [Cell<u64>; PARTS],
    longest: Cell<u64>,
    any: Cell<bool>,
}

#[allow(clippy::declare_interior_mutable_const)]
const ZERO_CELL: Cell<u64> = Cell::new(0);

thread_local! {
    static THREAD: Times = const {
        Times {
            side: Cell::new(0),
            parts: [ZERO_CELL; PARTS],
            longest: Cell::new(0),
            any: Cell::new(false),
        }
    };
}

/// This thread's blocking calls are timed as `side`'s from now (its start).
pub fn set_side(side: Side) {
    THREAD.with(|t| t.side.set(side as u8));
}

/// `f`, timed as `call` when [`timing`].
#[inline]
pub fn timed<R>(call: Call, f: impl FnOnce() -> R) -> R {
    if !timing() {
        return f();
    }
    let started = Instant::now();
    let r = f();
    note(call, started.elapsed());
    r
}

/// A call timed from here to the guard's drop, as [`timed`] times a closure (for a call whose borrows a closure
/// would not take).
pub struct Timed {
    call: Call,
    started: Option<Instant>,
}

impl Timed {
    #[inline]
    pub fn new(call: Call) -> Timed {
        Timed {
            call,
            started: timing().then(Instant::now),
        }
    }
}

impl Drop for Timed {
    #[inline]
    fn drop(&mut self) {
        if let Some(started) = self.started {
            note(self.call, started.elapsed());
        }
    }
}

/// A call already timed (`d`), noted as [`timed`] notes it: added to its part, and the thread's longest if it is.
/// Nothing on a thread that is not timed.
#[inline]
pub fn note(call: Call, d: Duration) {
    THREAD.with(|t| {
        if t.side.get() == 0 || !ON.load(Relaxed) {
            return;
        }
        let ns = nanos(d);
        if let Some(p) = call.part() {
            let c = &t.parts[p as usize];
            c.set(c.get().saturating_add(ns));
        }
        t.longest.set(t.longest.get().max(pack(ns, call)));
        t.any.set(true);
    });
}

/// What this thread timed since it last handed it over, added to the counts the host reads.
#[inline]
pub fn publish() {
    THREAD.with(|t| {
        if !t.any.replace(false) {
            return;
        }
        let side = match t.side.get() {
            1 => Side::Render,
            2 => Side::Queue,
            _ => return,
        };
        for (i, c) in t.parts.iter().enumerate() {
            let ns = c.replace(0);
            if ns > 0 {
                COUNTERS[side.first() + i].fetch_add(ns, Relaxed);
            }
        }
        LONGEST[side as usize - 1].fetch_max(t.longest.replace(0), Relaxed);
    });
}

/// [`publish`] after the render thread's idle work ([`crate::Backend::idle`]): its calls are not
/// part of its time on the game's commands (that time is [`Count::RenderIdleNs`], counted whole),
/// so only the longest of them is handed over, and their parts are dropped.
#[inline]
pub fn publish_idle() {
    THREAD.with(|t| {
        for c in &t.parts {
            c.set(0);
        }
    });
    publish();
}

/// A call and its time in one word that orders by the time.
fn pack(ns: u64, call: Call) -> u64 {
    (ns.min(u64::MAX >> 8) << 8) | u64::from(call as u8)
}

/// Each side's longest call since the last take ([`Side`]'s order: the render thread's, the queue thread's), with
/// its time (ns); taken, so the next frame starts from none.
pub fn take_longest() -> [Option<(Call, u64)>; 2] {
    std::array::from_fn(|i| {
        let w = LONGEST[i].swap(0, Relaxed);
        Call::from_number((w & 0xff) as u8).map(|c| (c, w >> 8))
    })
}

/// Told when a thread starts (`1` a reply, `2` the frames in flight at `Present`, `3` room in the queue) and ends (`0`)
/// a wait for the render thread: the engine notes it for the thread, so a thread that waits for the platform's lock
/// while this one holds it can say what the holder was waiting for (installed by the platform layer).
static WAIT_HOOK: std::sync::OnceLock<fn(u32)> = std::sync::OnceLock::new();

/// Installs [`WAIT_HOOK`] (the first one set stays).
pub fn set_wait_hook(f: fn(u32)) {
    let _ = WAIT_HOOK.set(f);
}

/// This thread starts (`what` > 0) or ends (0) a wait for the render thread.
#[inline]
pub(crate) fn note_wait(what: u32) {
    if let Some(f) = WAIT_HOOK.get() {
        f(what);
    }
}

/// Every count now (the host subtracts the one before).
pub fn snapshot() -> [u64; COUNTS] {
    std::array::from_fn(|i| COUNTERS[i].load(Relaxed))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The tests that turn the timing on or off, or read the longest calls, one at a time (the switch and the
    /// longest are the whole program's).
    pub(crate) static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn adds_land_in_their_count() {
        let before = snapshot();
        add(Count::Pipelines, 3);
        add_time(Count::PipelineNs, std::time::Duration::from_millis(18));
        let now = snapshot();
        assert!(now[Count::Pipelines as usize] - before[Count::Pipelines as usize] >= 3);
        assert!(now[Count::PipelineNs as usize] - before[Count::PipelineNs as usize] >= 18_000_000);
    }

    /// The calls' numbers name them back, every one, and each part's counts follow [`Part`]'s order on both sides.
    #[test]
    fn calls_and_parts_keep_their_order() {
        for (i, c) in CALLS.iter().enumerate() {
            assert_eq!(*c as u8 as usize, i + 1, "{c:?}");
            assert_eq!(Call::from_number(*c as u8), Some(*c));
        }
        assert_eq!(Call::from_number(0), None);
        assert_eq!(Call::from_number(CALLS.len() as u8 + 1), None);
        assert_eq!(
            Side::Render.first() + Part::Finish as usize,
            Count::RenderFinishNs as usize
        );
        assert_eq!(
            Side::Queue.first() + Part::Finish as usize,
            Count::QueueFinishNs as usize
        );
        assert_eq!(
            Side::Render.first() + Part::ThreadWait as usize,
            Count::RenderThreadWaitNs as usize
        );
        assert_eq!(
            Side::Queue.first() + Part::Upload as usize,
            Count::QueueUploadNs as usize
        );
    }

    /// A timed thread's calls land in its side's parts once it publishes, with its longest call named; a thread
    /// that is not one of the two (main) times nothing, and nothing is timed with the log off.
    #[test]
    fn calls_are_timed_on_their_side_and_published() {
        let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // Other tests in this binary may publish too: this one's own counts are on threads of its own, and the
        // longest is checked as at least this one's.
        set_on(true);
        let queue = std::thread::spawn(|| {
            set_side(Side::Queue);
            assert!(timing());
            let before = snapshot();
            note(Call::Submit, Duration::from_millis(7));
            note(Call::Submit, Duration::from_millis(2));
            note(Call::WriteTexture, Duration::from_millis(3));
            note(Call::Present, Duration::from_millis(1));
            assert_eq!(
                snapshot()[Count::QueueSubmitNs as usize],
                before[Count::QueueSubmitNs as usize],
                "nothing before the publish"
            );
            publish();
            let now = snapshot();
            let d = |c: Count| now[c as usize] - before[c as usize];
            assert!(d(Count::QueueSubmitNs) >= 9_000_000);
            assert!(d(Count::QueueUploadNs) >= 3_000_000);
            // The present is counted in SwapNs where it is timed, not again here.
            assert_eq!(d(Count::QueueFinishNs), 0);
            // timed() runs its call and counts it.
            assert_eq!(timed(Call::Poll, || 5), 5);
        });
        queue.join().unwrap();
        let [_, q] = take_longest();
        let (call, ns) = q.expect("the queue thread's longest");
        assert!(ns >= 7_000_000, "{call:?} {ns}");
        // Not a timed thread: nothing noted.
        assert!(!timing());
        note(Call::Submit, Duration::from_secs(5));
        publish();
        assert!(take_longest()[0].map_or(true, |(_, ns)| ns < 5_000_000_000));
        // The log off: a timed thread times nothing.
        let off = std::thread::spawn(|| {
            set_side(Side::Render);
            set_on(false);
            assert!(!timing());
            note(Call::GpuWait, Duration::from_secs(9));
            publish();
            set_on(true);
        });
        off.join().unwrap();
        assert!(take_longest()[0].map_or(true, |(_, ns)| ns < 9_000_000_000));
    }

    #[test]
    fn a_packed_call_keeps_its_time_and_name() {
        let w = pack(181_000_000, Call::Submit);
        assert_eq!(w >> 8, 181_000_000);
        assert_eq!(Call::from_number((w & 0xff) as u8), Some(Call::Submit));
        assert!(pack(2, Call::Poll) > pack(1, Call::Module));
    }
}
