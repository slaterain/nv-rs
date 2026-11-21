//! The hitch log's counters (`host::hitch_log`, on in every run unless `NV_HITCH_LOG=0`): one-off work a frame did,
//! counted where it happens so a long frame can say what it was doing. Every count is one relaxed atomic add (or, for
//! the guest heap, a plain load and store under the heap's own lock) at a place that runs rarely (a file opened, a
//! texture created, a cell attached) or already pays far more than the add (a read from disk, a blocking wait, a heap
//! block allocated and zero-filled). No count reads a clock except the waits and reads, which time what is already a
//! blocking call. Nothing here changes what the game does, or when.
//!
//! The host takes a [`snapshot`] after each of main's frames and subtracts the one before; the difference is the
//! frame's work. Counts from other threads land in the frame in which they were made, which for a loader or the render
//! thread is the frame main was in at the time.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

/// What is counted. `Main` counts are the game's main thread's (with the host's and the render helpers' work done for
/// main's frame); `Other` the game's other threads' (AI, the loaders).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Count {
    /// Guest files opened (`CreateFileA`/`CreateFileW` that succeeded).
    FilesOpenedMain,
    FilesOpenedOther,
    /// Bytes read from guest files (`ReadFile`), and the time the reads took (ns).
    BytesReadMain,
    BytesReadOther,
    ReadNsMain,
    ReadNsOther,
    /// Main's waits for the kernel lock (another game thread held it), and for the device lock (ns, count).
    KernelWaitNs,
    KernelWaits,
    DeviceWaitNs,
    DeviceWaits,
    /// Main's waits for a game critical section another thread held (ns, count).
    SectionWaitNs,
    SectionWaits,
    /// Main's waits on events, semaphores and threads (`WaitForSingleObject` and friends: a loader's work, a task's
    /// end) (ns, count).
    ObjectWaitNs,
    ObjectWaits,
    /// Main's `Sleep`s (ns, count): the frame limiter's and the game's own.
    SleepNs,
    Sleeps,
    /// Main's waits for the render thread: in `Present` (frames in flight), for a reply (a read-back, a query), for
    /// room in the command queue (ns).
    PresentWaitNs,
    ReplyWaitNs,
    ReplyWaits,
    QueueFullNs,
    /// Guest heap blocks allocated and freed (`Mem::alloc`/`Mem::free`).
    AllocsMain,
    FreesMain,
    AllocsOther,
    FreesOther,
    /// Direct3D 9 objects the game created (any thread): textures (cube ones included), surfaces (render targets,
    /// depth, off-screen), vertex and index buffers, shaders.
    Textures,
    Surfaces,
    Buffers,
    Shaders,
    /// Bytes the game uploaded into textures and surfaces, and into vertex and index buffers.
    TextureBytes,
    BufferBytes,
    /// Sound buffers created (`CreateSoundBuffer`, secondary) and their bytes; Ogg Vorbis files opened for decoding.
    SoundBuffers,
    SoundBytes,
    SoundsOpened,
    /// Cells attached to the world (`TESObjectCELL::AttachToWorld`, `00552970`), detached (`TESObjectCELL::Detach`,
    /// `00552bd0`) and unloaded (`TESDataHandler::UnloadCell`, `00462290`).
    CellsAttached,
    CellsDetached,
    CellsUnloaded,
    /// The frame's end handed freed memory back with every game thread stopped at a safe point (ns).
    HandBackNs,
    /// Saves (`008503b0`, BGSSaveLoadManager::SaveGame) and loads (`00847df0`, BGSSaveLoadGame::LoadGame) begun.
    Saves,
    Loads,
    /// Scripts compiled (each compile builds a `ScriptCompileData`, `005ae490`): the console's lines, mods' scripts.
    ScriptCompiles,
    /// Main's time inside the guest heap's allocate and free (ns; the heap's own lock wait included).
    HeapNsMain,
    /// Interface tiles made (`Tile::Init`, `009ff600`: every tile a menu or the HUD builds).
    TilesMade,
    /// Interface tiles destroyed (`Tile::~Tile`, `009ff340`).
    TilesDeleted,
    /// References given a 3D (`TESObjectREFR::Set3D`, `005702e0`, with a node other than none and other than the
    /// one they had).
    Refs3dAttached,
    /// Main's time in `TESObjectCELL::AttachToWorld` and `TESObjectCELL::Detach` (ns).
    CellAttachNs,
    CellDetachNs,
    /// Main's time in the queued-task hand-over (`IOManager::UpdateQueue`, `00c3dbf0`: finished loads, the queued
    /// references' and models' completion, run on main once a frame) (ns).
    HandOverNs,
    /// Main's time in the physics world's update (`bhkWorld::Update`, `00c6ae70`) (ns), and the Havok steps it took
    /// (`hkpWorld::stepDeltaTime`, `00c91b10`, and `hkpWorld::stepMultithreaded`: one per sub-step).
    HavokNsMain,
    HavokSteps,
    /// Main's time running scripts (`ScriptRunner::Run`, `005e0d20`, outermost only: a script a script runs is in
    /// its caller's time) (ns), and the scripts run (every run, nested ones included).
    ScriptNsMain,
    ScriptRuns,
}

/// How many counts there are.
pub const COUNTS: usize = Count::ScriptRuns as usize + 1;

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU64 = AtomicU64::new(0);
static COUNTERS: [AtomicU64; COUNTS] = [ZERO; COUNTS];

/// A counter on a cache line of its own: the heap's counts change on every block from any thread, so they share
/// no line with each other or with the rest.
#[repr(align(64))]
struct Line(AtomicU64);

/// The heap's four counts ([`heap`]), in [`Count::AllocsMain`]'s order: allocs main, frees main, allocs other,
/// frees other.
static HEAP: [Line; 4] = [
    Line(AtomicU64::new(0)),
    Line(AtomicU64::new(0)),
    Line(AtomicU64::new(0)),
    Line(AtomicU64::new(0)),
];

/// Count `i`'s counter: the heap's four live in [`HEAP`].
fn counter(i: usize) -> &'static AtomicU64 {
    let first = Count::AllocsMain as usize;
    if (first..first + 4).contains(&i) {
        &HEAP[i - first].0
    } else {
        &COUNTERS[i]
    }
}

/// The longest wait main made since the last [`take_longest_wait`]: ns, kind ([`WaitKind`] as a number) and the game
/// function that waited (the caller of the import, 0 none). Main only writes it, so the three need not be one step.
static LONGEST_NS: AtomicU64 = AtomicU64::new(0);
static LONGEST_KIND: AtomicU64 = AtomicU64::new(0);
static LONGEST_SITE: AtomicU64 = AtomicU64::new(0);

/// The largest single read main made since the last [`take_largest_read`]: bytes and the file's name.
static LARGEST_READ: AtomicU64 = AtomicU64::new(0);
static LARGEST_READ_NAME: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Off: the host's `NV_HITCH_LOG=0` (nothing is counted that needs more than an add; the waits read no clock).
static OFF: AtomicBool = AtomicBool::new(false);

/// Whether the counters are on (the default).
#[inline]
pub fn on() -> bool {
    !OFF.load(Relaxed)
}

/// The host's switch (`NV_HITCH_LOG=0` turns the counting off).
pub fn set_on(on: bool) {
    OFF.store(!on, Relaxed);
}

/// Adds `n` to `c`.
#[inline]
pub fn add(c: Count, n: u64) {
    counter(c as usize).fetch_add(n, Relaxed);
}

/// Adds 1 to `c`.
#[inline]
pub fn one(c: Count) {
    add(c, 1);
}

/// [`Count`]'s main or other form, by whether the running OS thread is another game thread.
#[inline]
pub fn side(main: Count, other: Count) -> Count {
    if crate::real_threads::on_other_game_thread() {
        other
    } else {
        main
    }
}

/// Where a heap block's count is in [`HEAP`] (and so which [`Count`] it is).
fn heap_index(free: bool, other: bool) -> usize {
    usize::from(other) * 2 + usize::from(free)
}

/// A heap block allocated (`free` false) or freed: one relaxed add on a cache line of its own ([`HEAP`]); nothing
/// with the log off.
#[inline]
pub fn heap(free: bool) {
    if !on() {
        return;
    }
    let other = crate::real_threads::on_other_game_thread();
    HEAP[heap_index(free, other)].0.fetch_add(1, Relaxed);
}

/// Whether the running OS thread is main's side (not another game thread) and the counters are on: where the timed
/// counts ([`span`], [`heap_timer`]) read the clock.
#[inline]
fn timed_here() -> bool {
    on() && !crate::real_threads::on_other_game_thread()
}

/// A timed stretch of main's work: its time (ns) goes to its count when it ends. Two clock readings, so only for
/// work that costs far more (a cell attached, the hand-over, the physics update, a script run). Nothing on another
/// game thread or with the log off.
#[must_use = "the time is counted when the span is dropped"]
pub struct Span(Option<(Count, std::time::Instant)>);

impl Drop for Span {
    fn drop(&mut self) {
        if let Some((c, t)) = self.0 {
            add(c, t.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64);
        }
    }
}

/// Times main's work into `c` until the returned span is dropped.
#[inline]
pub fn span(c: Count) -> Span {
    Span(timed_here().then(|| (c, std::time::Instant::now())))
}

thread_local! {
    /// How deep this OS thread is in [`script_span`]s and [`heap_timer`]s (only the outermost is timed).
    static SCRIPT_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static IN_HEAP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A script run: counted ([`Count::ScriptRuns`], every run) and timed into [`Count::ScriptNsMain`] when it is the
/// outermost on this thread, so a script another script runs is not counted twice.
pub struct ScriptSpan {
    timed: Option<std::time::Instant>,
}

impl Drop for ScriptSpan {
    fn drop(&mut self) {
        SCRIPT_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        if let Some(t) = self.timed {
            add(
                Count::ScriptNsMain,
                t.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
            );
        }
    }
}

/// Starts a script run's span ([`ScriptSpan`]); the run count goes up on any thread.
#[inline]
pub fn script_span() -> ScriptSpan {
    if on() {
        one(Count::ScriptRuns);
    }
    let outermost = SCRIPT_DEPTH.with(|d| {
        let n = d.get();
        d.set(n + 1);
        n == 0
    });
    ScriptSpan {
        timed: (outermost && timed_here()).then(std::time::Instant::now),
    }
}

/// The heap's time on main ([`Count::HeapNsMain`]): the start of an allocate or free, `None` on another game thread,
/// with the log off, or inside an outer one (the shared heap's call reaches the plain heap's).
#[inline]
pub fn heap_timer() -> Option<std::time::Instant> {
    if !timed_here() || IN_HEAP.with(std::cell::Cell::get) {
        return None;
    }
    IN_HEAP.with(|h| h.set(true));
    Some(std::time::Instant::now())
}

/// The end of a [`heap_timer`]'s allocate or free.
#[inline]
pub fn heap_timer_end(started: Option<std::time::Instant>) {
    if let Some(t) = started {
        IN_HEAP.with(|h| h.set(false));
        add(
            Count::HeapNsMain,
            t.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
        );
    }
}

/// Who held the kernel lock while main waited for it, for main's longest such wait of the frame (of 1 ms or more):
/// the holder's game thread id and start address, the platform function it was in (0 and an empty name when it held
/// the lock outside one: the kernel's own bookkeeping, the host between frames), main's wait, and the holder's
/// processor time over the measured part of that wait (`None` where the system does not tell it: macOS). A holder with far less processor time
/// than the wait was pushed off the processor (other programs); one with about as much was working under the lock.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LockHolder {
    /// The game critical section main waited for (its address), 0 for the kernel lock.
    pub section: u32,
    pub thread: u32,
    pub start: u32,
    pub call: u32,
    pub call_name: String,
    pub wait_ns: u64,
    /// The part of the wait the processor time covers (from the moment the wait passed 1 ms to its end).
    pub measured_ns: u64,
    pub cpu_ns: Option<u64>,
}

/// The waits main makes that are worth naming a holder for (1 ms).
pub const HOLDER_WAIT_NS: u64 = 1_000_000;

static HOLDER: std::sync::Mutex<Option<LockHolder>> = std::sync::Mutex::new(None);

/// Main waited `holder.wait_ns` for the kernel lock that `holder` held: kept when it is the frame's longest.
pub fn lock_holder(holder: LockHolder) {
    if holder.wait_ns < HOLDER_WAIT_NS {
        return;
    }
    if let Ok(mut h) = HOLDER.lock() {
        if h.as_ref().map_or(true, |k| holder.wait_ns > k.wait_ns) {
            *h = Some(holder);
        }
    }
}

/// The frame's longest named wait for the kernel lock ([`lock_holder`]), then none.
pub fn take_lock_holder() -> Option<LockHolder> {
    HOLDER.lock().ok().and_then(|mut h| h.take())
}

static SECTION_HOLDER: std::sync::Mutex<Option<LockHolder>> = std::sync::Mutex::new(None);

/// Main waited `holder.wait_ns` for the game critical section `holder.section` that `holder.thread` owned (the
/// platform function the owner was in then, its processor time over the whole wait): kept when it is the frame's
/// longest of 1 ms or more.
pub fn section_holder(holder: LockHolder) {
    if holder.wait_ns < HOLDER_WAIT_NS {
        return;
    }
    if let Ok(mut h) = SECTION_HOLDER.lock() {
        if h.as_ref().map_or(true, |k| holder.wait_ns > k.wait_ns) {
            *h = Some(holder);
        }
    }
}

/// The frame's longest named wait for a game critical section ([`section_holder`]), then none.
pub fn take_section_holder() -> Option<LockHolder> {
    SECTION_HOLDER.lock().ok().and_then(|mut h| h.take())
}

/// What main waited for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum WaitKind {
    #[default]
    None,
    KernelLock,
    DeviceLock,
    Section,
    Object,
    Sleep,
    Present,
    Reply,
    QueueFull,
}

impl WaitKind {
    fn from(n: u64) -> WaitKind {
        use WaitKind::*;
        [
            None, KernelLock, DeviceLock, Section, Object, Sleep, Present, Reply, QueueFull,
        ]
        .get(n as usize)
        .copied()
        .unwrap_or(None)
    }

    /// The counts this kind adds its time and number to.
    fn counts(self) -> Option<(Count, Option<Count>)> {
        Some(match self {
            WaitKind::None => return Option::None,
            WaitKind::KernelLock => (Count::KernelWaitNs, Some(Count::KernelWaits)),
            WaitKind::DeviceLock => (Count::DeviceWaitNs, Some(Count::DeviceWaits)),
            WaitKind::Section => (Count::SectionWaitNs, Some(Count::SectionWaits)),
            WaitKind::Object => (Count::ObjectWaitNs, Some(Count::ObjectWaits)),
            WaitKind::Sleep => (Count::SleepNs, Some(Count::Sleeps)),
            WaitKind::Present => (Count::PresentWaitNs, Option::None),
            WaitKind::Reply => (Count::ReplyWaitNs, Some(Count::ReplyWaits)),
            WaitKind::QueueFull => (Count::QueueFullNs, Option::None),
        })
    }
}

/// Main waited `ns` for `kind` at the game function `site` (0: none known): its time and count, and the frame's
/// longest wait. Called only by main (the caller checks [`crate::real_threads::on_other_game_thread`]).
pub fn main_wait(kind: WaitKind, site: u32, ns: u64) {
    let Some((time, count)) = kind.counts() else {
        return;
    };
    add(time, ns);
    if let Some(c) = count {
        one(c);
    }
    // Sleeps are main's own choice (the frame limiter), not something it was held up by.
    if kind != WaitKind::Sleep && ns > LONGEST_NS.load(Relaxed) {
        LONGEST_NS.store(ns, Relaxed);
        LONGEST_KIND.store(kind as u64, Relaxed);
        LONGEST_SITE.store(u64::from(site), Relaxed);
    }
}

/// The longest wait since the last take (ns, kind, site), then none.
pub fn take_longest_wait() -> (u64, WaitKind, u32) {
    let ns = LONGEST_NS.swap(0, Relaxed);
    let kind = WaitKind::from(LONGEST_KIND.swap(0, Relaxed));
    let site = LONGEST_SITE.swap(0, Relaxed) as u32;
    (ns, kind, site)
}

/// Main read `bytes` from the file `name` (a guest path) in one read: kept when it is the largest since the last take.
pub fn main_read(bytes: u64, name: &dyn Fn() -> String) {
    if bytes > LARGEST_READ.load(Relaxed) {
        LARGEST_READ.store(bytes, Relaxed);
        if let Ok(mut n) = LARGEST_READ_NAME.lock() {
            *n = name();
        }
    }
}

/// The largest single read since the last take (bytes and the file's name), then none.
pub fn take_largest_read() -> (u64, String) {
    let bytes = LARGEST_READ.swap(0, Relaxed);
    let name = LARGEST_READ_NAME
        .lock()
        .map(|mut n| std::mem::take(&mut *n))
        .unwrap_or_default();
    (bytes, name)
}

/// Every count now (the host subtracts the one before).
pub fn snapshot() -> [u64; COUNTS] {
    std::array::from_fn(|i| counter(i).load(Relaxed))
}

/// The frame's counts: `now` minus `before`, count by count.
pub fn since(now: &[u64; COUNTS], before: &[u64; COUNTS]) -> [u64; COUNTS] {
    std::array::from_fn(|i| now[i].wrapping_sub(before[i]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adds land in their own count only; a frame's counts are the difference of two snapshots.
    #[test]
    fn a_frame_s_counts_are_the_difference_of_two_snapshots() {
        let before = snapshot();
        add(Count::BytesReadMain, 4096);
        one(Count::CellsAttached);
        one(Count::CellsAttached);
        add(Count::TextureBytes, 7);
        let d = since(&snapshot(), &before);
        // Other tests in this binary may count meanwhile: only counts no other test touches are exact.
        assert_eq!(d[Count::CellsAttached as usize], 2);
        assert!(d[Count::BytesReadMain as usize] >= 4096);
        assert!(d[Count::TextureBytes as usize] >= 7);
        assert_eq!(since(&before, &before), [0; COUNTS]);
        // A counter that wrapped still gives the right difference.
        let mut old = [0u64; COUNTS];
        old[0] = u64::MAX;
        let mut new = [0u64; COUNTS];
        new[0] = 2;
        assert_eq!(since(&new, &old)[0], 3);
    }

    /// Main's waits add to their kind's time and count; the longest (a sleep excepted) is kept with its site until
    /// taken.
    #[test]
    fn main_s_waits_add_up_and_the_longest_is_kept() {
        let _l = WAIT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = take_longest_wait();
        let before = snapshot();
        // The real-thread tests beside this one wait too (milliseconds at most): times of hours here, so only this
        // test's waits can be the longest, and the counts compared as at least.
        const HOUR: u64 = 3_600_000_000_000;
        main_wait(WaitKind::Object, 0x00c3_dfa0, 3 * HOUR);
        main_wait(WaitKind::Section, 0x00e7_ea00, HOUR);
        main_wait(WaitKind::Sleep, 0x0086_e650, 9 * HOUR);
        main_wait(WaitKind::Present, 0, 2 * HOUR);
        main_wait(WaitKind::None, 0, 99 * HOUR);
        let d = since(&snapshot(), &before);
        assert!((3 * HOUR..4 * HOUR).contains(&d[Count::ObjectWaitNs as usize]));
        assert!(d[Count::ObjectWaits as usize] >= 1);
        assert!(d[Count::SectionWaits as usize] >= 1);
        assert!((9 * HOUR..10 * HOUR).contains(&d[Count::SleepNs as usize]));
        assert!((2 * HOUR..3 * HOUR).contains(&d[Count::PresentWaitNs as usize]));
        assert_eq!(
            take_longest_wait(),
            (3 * HOUR, WaitKind::Object, 0x00c3_dfa0),
            "the longest, a sleep and a none left out"
        );
        assert!(
            take_longest_wait().0 < HOUR,
            "taken: none of this test's waits left"
        );
    }

    /// The largest read is kept with its name, a smaller one after it does not replace it, and a take empties it.
    #[test]
    fn the_largest_read_is_kept_with_its_name() {
        let _l = WAIT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = take_largest_read();
        main_read(100, &|| "a.nif".into());
        main_read(5_000, &|| "Fallout - Meshes.bsa".into());
        main_read(200, &|| "b.kf".into());
        assert_eq!(take_largest_read(), (5_000, "Fallout - Meshes.bsa".into()));
        assert_eq!(take_largest_read(), (0, String::new()));
    }

    /// The heap's counts go to main's side on a thread that is not another game thread.
    #[test]
    fn heap_blocks_count_on_main_s_side_here() {
        let _l = WAIT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = snapshot();
        heap(false);
        heap(false);
        heap(true);
        let d = since(&snapshot(), &before);
        assert!(d[Count::AllocsMain as usize] >= 2);
        assert!(d[Count::FreesMain as usize] >= 1);
        // Each block's count is its own Count (other tests count meanwhile, so the mapping is checked directly).
        for (free, other, c) in [
            (false, false, Count::AllocsMain),
            (true, false, Count::FreesMain),
            (false, true, Count::AllocsOther),
            (true, true, Count::FreesOther),
        ] {
            assert!(std::ptr::eq(
                counter(c as usize),
                &HEAP[heap_index(free, other)].0
            ));
        }
        assert_eq!(
            side(Count::FilesOpenedMain, Count::FilesOpenedOther),
            Count::FilesOpenedMain
        );
    }

    /// The per-event cost of the heap's count with two threads counting at once (`--ignored`, release: `cargo test
    /// --release -p engine-core --lib heap_count_costs -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn heap_count_costs_nanoseconds() {
        let n = 20_000_000u32;
        let t = std::time::Instant::now();
        std::thread::scope(|s| {
            for _ in 0..2 {
                s.spawn(|| {
                    for i in 0..n {
                        heap(i & 1 == 0);
                    }
                });
            }
        });
        println!(
            "{:.2} ns a counted block (two threads at once)",
            t.elapsed().as_secs_f64() * 1e9 / f64::from(n)
        );
    }

    static WAIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
