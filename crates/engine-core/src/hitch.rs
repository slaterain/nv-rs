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
    /// Main's time finishing background loads (`IOManager::UpdateQueue`, `00c3dbf0`, the whole call) (ns), and the
    /// completed load tasks it ran (each task's run, slot `0x14`).
    LoadFinishNs,
    LoadFinishTasks,
    /// Main's time in `TESObjectCELL::AttachToWorld` (`00552970`), `TESObjectCELL::Detach` (`00552bd0`) and
    /// `TESDataHandler::UnloadCell` (`00462290`) (ns). Nested calls count in each (an unload detaches).
    CellAttachNs,
    CellDetachNs,
    CellUnloadNs,
    /// Main's time in the frame's grid-cell update (`0086fbe0`: the player's cell tracker, which shifts the grid at a
    /// border) (ns).
    GridUpdateNs,
    /// Main's time in the model loader's two checks that `IOManager::UpdateQueue` runs first every frame, outside its
    /// budget (`00446e40`): the drain of the loader's queue (`00448c10`, every task run) and the walk over its queued
    /// references (`00448cc0`, each one's `QueueMe`, slot `0x20`, which starts its loads) (ns).
    LoaderDrainNs,
    LoaderWalkNs,
    /// The parts of a grid shift in `TES::UpdateCurrentGridCell` (`00452580`), main's time in each (ns): the cancel of
    /// the exterior cell loader's queued cells (`00528110`, `005281f0`, `005282d0`), the unused textures' clean-up
    /// (`00454d50` / `TES::CleanUpUnusedTextures`), the grid's `SetCenter` (slot `0x10`: cells leaving detached, the
    /// grid moved), the Havok world's lock counts dropped after it (the deferred additions run), the grid array's load
    /// (`TES::GridArrayLoad`: the new cells loaded and attached) and the distant terrain's update at the player.
    GridCancelNs,
    GridTexturesNs,
    GridShiftNs,
    GridHavokNs,
    GridLoadNs,
    GridTerrainNs,
    /// Inside the grid array's load, per grid cell (`TES::LoadGridCell`, `00451a50`), main's time in each part (ns):
    /// finding the cell (`00585b30`, which reads it from the plugin when it is not in memory) or making a missing one
    /// (`00461c20`, its persistent references and temporary data), the land (its mesh loaded or updated, the lowest
    /// height for the water bit) and the placeable water. The cells made count in `GridCellsMade`.
    GridCellFindNs,
    GridCellsMade,
    GridLandNs,
    GridWaterNs,
    /// The rest of the grid array's load (`TES::GridArrayLoad`, `00452ff0`), main's time in each part (ns): the first
    /// loop over the grid (each slot's `TES::LoadGridCell` with the exterior buffer and the load screen's refresh), the
    /// Havok world's `+0x18` lock count dropped after each loop (`00c6b540`: only when it reaches zero are the bodies,
    /// phantoms, actions and constraints queued while the world was locked added, the bodies' broad-phase inserts),
    /// the save/load object's and the script context's pass between the loops (`008727b0`'s branch, `008492b0`), and
    /// the second loop's two parts for each cell in load state 5: its references attached (`0054bcf0`: the cell's
    /// scripts, each reference's model queued and its 3D attached) with the buffer entry, and its 3D's properties and
    /// update (`NiAVObject::UpdateProperties`, `00a59c60`).
    GridLoadCellsNs,
    GridLoadHavokNs,
    GridLoadPassNs,
    GridLoadRefsNs,
    GridLoad3dNs,
    /// Main's batches of queued bodies added to the Havok world (`00c674d0`: `hkpWorld::addEntityBatch`, the
    /// broad-phase inserts, then each body's island and activation): the bodies and the time (ns). A body is queued
    /// while the world's lock count is up (`bhkWorld::Add`, `00c6b0a0`); the batches run when the count drops to
    /// zero, at most 100 a frame with the drop's flag set, and from `bhkWorld::Update` (on the AI thread, not counted).
    HavokBodiesAdded,
    HavokBodiesAddNs,
    /// Main's time inside the guest heap's allocate and free (ns; the heap's own lock wait included).
    HeapNsMain,
    /// Interface tiles made (`Tile::Init`, `009ff600`: every tile a menu or the HUD builds).
    TilesMade,
    /// Interface tiles destroyed (`Tile::~Tile`, `009ff340`).
    TilesDeleted,
    /// References given a 3D (`TESObjectREFR::Set3D`, `005702e0`, with a node other than none and other than the
    /// one they had).
    Refs3dAttached,
    /// Main's time in the physics world's update (`bhkWorld::Update`, `00c6ae70`) (ns), and the Havok steps it took
    /// (`hkpWorld::stepDeltaTime`, `00c91b10`, and `hkpWorld::stepMultithreaded`: one per sub-step).
    HavokNsMain,
    HavokSteps,
    /// Main's Havok reference changes that took the one global reference-count lock (`hkReferencedObject`'s
    /// `addReference` `00c90510`, `removeReference` `00c905b0`, `addReferences` `00c90670`, `removeReferences`
    /// `00c90740` on a thread that does not hold it, with the lock mode 1 the game always has), and main's time
    /// waiting for it there (ns): a physics step holds it for the whole step (`hkpWorld::stepDeltaTime` takes the
    /// world's lock, `00c91180`). With the "Havok reference counts without the global lock" row both stay 0.
    HavokRefLocks,
    HavokRefWaitNs,
    /// Main's time waiting for the same lock taken whole (`lockAll`, `00c90490`: the world's lock for a change or
    /// a step on main) (ns): the waits the row leaves.
    HavokLockAllWaitNs,
    /// Main's time running scripts (`ScriptRunner::Run`, `005e0d20`, outermost only: a script a script runs is in
    /// its caller's time) (ns), and the scripts run (every run, nested ones included).
    ScriptNsMain,
    ScriptRuns,
}

/// How many counts there are.
pub const COUNTS: usize = Count::ScriptRuns as usize + 1;

/// Times main's work from [`Timed::main`] to its drop into a `...Ns` count: nothing on another game thread or with
/// the log off. One clock read at each end, at places that run a few times a frame at most and do far more work.
pub struct Timed {
    count: Count,
    start: Option<std::time::Instant>,
}

impl Timed {
    /// Starts timing into `count` when the log is on and this is main.
    #[inline]
    pub fn main(count: Count) -> Timed {
        let start =
            (on() && !crate::real_threads::on_other_game_thread()).then(std::time::Instant::now);
        Timed { count, start }
    }

    /// The time since the start (ns), 0 when not timing.
    pub fn ns(&self) -> u64 {
        self.start.map_or(0, |s| {
            s.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
        })
    }
}

impl Drop for Timed {
    fn drop(&mut self) {
        if self.start.is_some() {
            add(self.count, self.ns());
        }
    }
}

/// The longest completed load task main ran since the last [`take_longest_task`]: ns and its vtable (its type).
static LONGEST_TASK_NS: AtomicU64 = AtomicU64::new(0);
static LONGEST_TASK_VTABLE: AtomicU64 = AtomicU64::new(0);

/// Main ran one completed load task of type `vtable` for `ns`: counted, and kept when the longest.
pub fn main_load_task(vtable: u32, ns: u64) {
    one(Count::LoadFinishTasks);
    if ns > LONGEST_TASK_NS.load(Relaxed) {
        LONGEST_TASK_NS.store(ns, Relaxed);
        LONGEST_TASK_VTABLE.store(u64::from(vtable), Relaxed);
    }
}

/// The longest load task since the last take (ns, vtable), then none.
pub fn take_longest_task() -> (u64, u32) {
    let ns = LONGEST_TASK_NS.swap(0, Relaxed);
    let vtable = LONGEST_TASK_VTABLE.swap(0, Relaxed) as u32;
    (ns, vtable)
}

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

/// A script run: counted ([`Count::ScriptRuns`], every run) and, with `NV_HITCH_HEAP=1` ([`heap_timed`]: two clock
/// reads a run), timed into [`Count::ScriptNsMain`] when it is the outermost on this thread, so a script another
/// script runs is not counted twice.
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
        timed: (outermost && heap_timed() && timed_here()).then(std::time::Instant::now),
    }
}

/// `NV_HITCH_HEAP=1` (a diagnostic switch, off by default): main's time in the guest heap ([`heap_timer`]) and in
/// each script run ([`script_span`]) is timed. Two clock reads around every allocate and free on main cost about
/// 4 ms on the pause menu's opening frame (80,000 blocks), and two more each script run, so a normal run counts
/// the blocks and the runs only. 0: the variable not read yet; 1: off; 2: on.
static HEAP_TIMED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Whether main's heap time is timed (`NV_HITCH_HEAP=1`; read once).
#[inline]
pub fn heap_timed() -> bool {
    match HEAP_TIMED.load(Relaxed) {
        0 => read_heap_switch(),
        2 => true,
        _ => false,
    }
}

#[cold]
#[inline(never)]
fn read_heap_switch() -> bool {
    let on = std::env::var("NV_HITCH_HEAP").is_ok_and(|v| v.trim() == "1");
    let _ = HEAP_TIMED.compare_exchange(0, if on { 2 } else { 1 }, Relaxed, Relaxed);
    HEAP_TIMED.load(Relaxed) == 2
}

/// Sets [`heap_timed`], whatever the variable says (tests).
#[doc(hidden)]
pub fn set_heap_timed(on: bool) {
    HEAP_TIMED.store(if on { 2 } else { 1 }, Relaxed);
}

/// The heap's time on main ([`Count::HeapNsMain`]): the start of an allocate or free, `None` unless
/// `NV_HITCH_HEAP=1` ([`heap_timed`]), on another game thread, with the log off, or inside an outer one (the shared
/// heap's call reaches the plain heap's).
#[inline]
pub fn heap_timer() -> Option<std::time::Instant> {
    if !heap_timed() || !timed_here() || IN_HEAP.with(std::cell::Cell::get) {
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
    /// Where outside a platform call the holder took the lock (`file:line` of the port's own code; empty: none
    /// known).
    pub site: String,
    /// How many times other threads took the lock while main waited: 0 is one long hold, more a lock that changed
    /// hands (short holds re-taken before main woke).
    pub handoffs: u32,
    /// What the holder was waiting for on the render thread when it was seen (0 nothing, 1 a reply, 2 the frames in
    /// flight at `Present`, 3 room in the queue).
    pub render_wait: u32,
}

/// The waits main makes that are worth naming a holder for (1 ms).
pub const HOLDER_WAIT_NS: u64 = 1_000_000;

/// A run's longest named wait of the frame, by the run (`crate::units::platform::KernelRef::id`): a test process runs
/// several runs at once, each with its own main.
type Holders = std::sync::Mutex<Vec<(usize, LockHolder)>>;

/// At most this many runs keep a record (a run that never takes its record: a test's).
const HOLDER_RUNS: usize = 64;

/// Keeps `holder` for `run` when it is that run's longest wait of the frame (1 ms or more).
fn keep_longest(slots: &Holders, run: usize, holder: LockHolder) {
    if holder.wait_ns < HOLDER_WAIT_NS {
        return;
    }
    if let Ok(mut h) = slots.lock() {
        match h.iter_mut().find(|(r, _)| *r == run) {
            Some((_, k)) if holder.wait_ns > k.wait_ns => *k = holder,
            Some(_) => {}
            None => {
                if h.len() >= HOLDER_RUNS {
                    h.remove(0);
                }
                h.push((run, holder));
            }
        }
    }
}

/// `run`'s record ([`keep_longest`]), then none.
fn take_run(slots: &Holders, run: usize) -> Option<LockHolder> {
    let mut h = slots.lock().ok()?;
    let i = h.iter().position(|(r, _)| *r == run)?;
    Some(h.swap_remove(i).1)
}

static HOLDER: Holders = std::sync::Mutex::new(Vec::new());

/// Main of `run` waited `holder.wait_ns` for the kernel lock that `holder` held: kept when it is the frame's longest.
pub fn lock_holder(run: usize, holder: LockHolder) {
    keep_longest(&HOLDER, run, holder);
}

/// `run`'s longest named wait of the frame for the kernel lock ([`lock_holder`]), then none.
pub fn take_lock_holder(run: usize) -> Option<LockHolder> {
    take_run(&HOLDER, run)
}

static SECTION_HOLDER: Holders = std::sync::Mutex::new(Vec::new());

/// Main of `run` waited `holder.wait_ns` for the game critical section `holder.section` that `holder.thread` owned
/// (the platform function the owner was in then, its processor time over the whole wait): kept when it is the
/// frame's longest of 1 ms or more.
pub fn section_holder(run: usize, holder: LockHolder) {
    keep_longest(&SECTION_HOLDER, run, holder);
}

/// `run`'s longest named wait of the frame for a game critical section ([`section_holder`]), then none.
pub fn take_section_holder(run: usize) -> Option<LockHolder> {
    take_run(&SECTION_HOLDER, run)
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

/// The functions whose waits for a critical section are main's waits for Havok's global reference-count lock:
/// `addReference`, `removeReference`, `addReferences`, `removeReferences`, and `lockAll` (the lock taken whole).
const HAVOK_REF_ADD: u32 = 0x00c9_0510;
const HAVOK_REF_REMOVE: u32 = 0x00c9_05b0;
const HAVOK_REFS_ADD: u32 = 0x00c9_0670;
const HAVOK_REFS_REMOVE: u32 = 0x00c9_0740;
const HAVOK_LOCK_ALL: u32 = 0x00c9_0490;

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
    if kind == WaitKind::Section {
        match site {
            HAVOK_REF_ADD | HAVOK_REF_REMOVE | HAVOK_REFS_ADD | HAVOK_REFS_REMOVE => {
                add(Count::HavokRefWaitNs, ns)
            }
            HAVOK_LOCK_ALL => add(Count::HavokLockAllWaitNs, ns),
            _ => {}
        }
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

    /// Main's waits for a critical section in Havok's reference functions are its waits for the global
    /// reference-count lock; in `lockAll`, for the lock taken whole; any other wait or site is neither.
    #[test]
    fn main_s_waits_for_havok_s_reference_lock_are_told_apart() {
        let _l = WAIT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = snapshot();
        const HOUR: u64 = 3_600_000_000_000;
        for site in [
            HAVOK_REF_ADD,
            HAVOK_REF_REMOVE,
            HAVOK_REFS_ADD,
            HAVOK_REFS_REMOVE,
        ] {
            main_wait(WaitKind::Section, site, HOUR);
        }
        main_wait(WaitKind::Section, HAVOK_LOCK_ALL, 2 * HOUR);
        main_wait(WaitKind::Object, HAVOK_REF_ADD, 16 * HOUR);
        main_wait(WaitKind::Section, 0x00e7_ea00, 32 * HOUR);
        let d = since(&snapshot(), &before);
        assert!((4 * HOUR..5 * HOUR).contains(&d[Count::HavokRefWaitNs as usize]));
        assert!((2 * HOUR..3 * HOUR).contains(&d[Count::HavokLockAllWaitNs as usize]));
        let _ = take_longest_wait();
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

    /// A timed stretch adds its time to its count; the longest load task is kept with its type until taken.
    #[test]
    fn timed_work_and_the_longest_load_task() {
        let _l = WAIT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = snapshot();
        {
            let t = Timed::main(Count::CellDetachNs);
            std::thread::sleep(std::time::Duration::from_millis(2));
            assert!(t.ns() >= 2_000_000);
        }
        let d = since(&snapshot(), &before);
        assert!(d[Count::CellDetachNs as usize] >= 2_000_000);
        let _ = take_longest_task();
        main_load_task(0x0101_0000, 5);
        main_load_task(0x0102_0000, 9);
        main_load_task(0x0103_0000, 7);
        assert!(since(&snapshot(), &before)[Count::LoadFinishTasks as usize] >= 3);
        assert_eq!(take_longest_task(), (9, 0x0102_0000));
        assert_eq!(take_longest_task(), (0, 0));
    }

    /// Main's heap time reads the clock only behind `NV_HITCH_HEAP=1`: off (the default), an allocate or free
    /// starts no timer; on, the outermost one on main does, and a nested one (the shared heap's call reaching the
    /// plain heap's) does not.
    #[test]
    fn the_heap_is_timed_only_behind_its_switch() {
        let _l = WAIT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set_heap_timed(false);
        assert!(heap_timer().is_none(), "off: no clock read");
        set_heap_timed(true);
        if on() {
            let outer = heap_timer();
            assert!(outer.is_some(), "on: main's allocate is timed");
            assert!(heap_timer().is_none(), "a nested one is not");
            heap_timer_end(outer);
            heap_timer_end(heap_timer());
        }
        set_heap_timed(false);
        assert!(!heap_timed());
    }

    static WAIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
