//! `NV_STAGE_TIME=1`, a measuring aid for the drawing thread's plan (off by default; docs/FIXES.md,
//! "Measuring"): the frame's stages timed on main, the drawing sequence after the world pass timed step by step,
//! the world draw's two halves and the drawing job wherever they run, and main's waits at each join of the drawing
//! thread (`crate::draw_thread`) by the join's exe address. `NV_FRAME_LOG` appends the values of each frame as
//! columns ([`NAMES`], microseconds and calls); nothing else reads them.
//!
//! **Off** (the variable unset, `0` or empty): every mark is one relaxed load and a branch. No clock is read,
//! nothing is written, no guest memory is touched, and nothing calls into the game.
//!
//! Two kinds of marks:
//! - [`point`]: a moment in main's frame (`Main::OnIdle`'s entry, `Main::Swap` around its draw call, the world
//!   draw's cut and end, `WaitForThreads`). Each point closes the stages that end there ([`SEGMENTS`]): the time
//!   since the point that starts the stage, if that point was passed in the same frame. `Main::OnIdle`'s entry
//!   ([`Point::Idle`]) closes the previous frame's last stages (so those land in the next frame's line) and starts a
//!   new frame.
//! - [`scope`]: a step's body, timed until the guard drops (inclusive of what it calls), on any thread.
//!
//! The marks' places and what each stage covers are in `measure/draw-thread/P0-MEASURE.md`.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering::Relaxed};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Instant;

/// A moment in main's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Point {
    /// `Main::OnIdle`'s entry (`0086e650`), before its join: a frame starts.
    Idle,
    /// The call of `Main::Swap` (`0086ff70`) in `0086e650`.
    SwapIn,
    /// The frame's draw (`008706b0`) starts.
    DrawIn,
    /// The world pass (`00873200`) calls the world draw's first half (`00b6ba20`): the cut.
    Cut,
    /// The world draw's second half (`00b6b930`) returned to the world pass (or was skipped: the job took it).
    DrawEnd,
    /// The world pass (`00873200`) returns.
    PassEnd,
    /// The frame's draw (`008706b0`) returns.
    DrawOut,
    /// `Main::Swap` returned to `0086e650`.
    SwapOut,
    /// `0086e650` calls `WaitForThreads` (`008c7990`).
    WaitIn,
    /// `WaitForThreads` returned.
    WaitOut,
    /// The host took the frame's marks (`host::hitch_log::take_stages`, after main's frame handed the engine back):
    /// the game's tail after `WaitForThreads` ends here, the host's own time between two frames starts.
    Host,
}

/// How many points there are.
pub const POINTS: usize = 11;

/// The stages: (column, the point that starts it, the point that ends it).
pub const SEGMENTS: [(Column, Point, Point); 15] = [
    (Column::U, Point::Idle, Point::SwapIn),
    (Column::S1, Point::SwapIn, Point::DrawIn),
    (Column::R, Point::DrawIn, Point::DrawOut),
    (Column::RBeforeCut, Point::DrawIn, Point::Cut),
    (Column::CutToDrawEnd, Point::Cut, Point::DrawEnd),
    (Column::PassTail, Point::DrawEnd, Point::PassEnd),
    (Column::AfterPass, Point::PassEnd, Point::DrawOut),
    (Column::S2, Point::DrawOut, Point::SwapOut),
    (Column::P1, Point::SwapOut, Point::WaitIn),
    (Column::W, Point::WaitIn, Point::WaitOut),
    (Column::P2, Point::WaitOut, Point::Idle),
    (Column::CutToIdle, Point::Cut, Point::Idle),
    (Column::DrawEndToIdle, Point::DrawEnd, Point::Idle),
    (Column::GameTail, Point::WaitOut, Point::Host),
    (Column::HostGap, Point::Host, Point::Idle),
];

/// The joins of the drawing thread with a column of their own (exe addresses of the functions whose entry joins,
/// `crate::draw_thread::finish_at`); any other lands in [`JOIN_OTHER`].
pub const JOIN_POINTS: [u32; 20] = [
    0x0086_e650, // Main::OnIdle
    0x0087_3200, // the world pass (a second post in one frame)
    0x0087_06b0, // the frame's draw
    0x0087_1dc0, // the menu background
    0x0087_46d0, // the first-person node into the offscreen target
    0x0087_4920, // the refraction normals
    0x0087_5110, // the rest of the frame's draw
    0x0087_5e40, // the water pass
    0x0087_6850, // the wind-down
    0x00b6_3a90, // the world accumulator's check
    0x0064_26a0, // the sun query
    0x00b5_fd60, // a light's object lists rebuilt
    0x00b5_d300, // the shadow scene node's lists
    0x0045_3dc0, // a cell's objects
    0x0087_8250, // the memory manager
    0x0093_c200, // the player
    0x0093_cdf0, // the player
    0x0093_d500, // the player
    0x0084_7df0, // a save
    0x0085_03b0, // a load
];

/// The columns, in [`NAMES`]' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Column {
    // Stages from points (one a frame).
    U,
    S1,
    R,
    RBeforeCut,
    CutToDrawEnd,
    PassTail,
    AfterPass,
    S2,
    P1,
    W,
    P2,
    CutToIdle,
    DrawEndToIdle,
    /// The game's frame after `WaitForThreads` until the host took the marks.
    GameTail,
    /// The host's time between two of main's frames (from its take to the next `OnIdle`).
    HostGap,
    // Steps (scopes).
    WorldPass,
    WorldDrawFirst,
    WorldDrawSecond,
    ExtraPass1,
    ExtraPass2,
    CameraMotion,
    SunQueryInPass,
    FirstPersonNode,
    Targets,
    AccumulatorCheck,
    RefractionNormals,
    FirstPersonSetup,
    DrawRest,
    WaterPass,
    TargetMarker,
    Interface,
    InterfaceTarget,
    InterfaceEnd,
    Margins,
    WindDown,
    LightUpdate,
    MenuPass,
    // The drawing thread.
    Job,
    JoinWait,
    /// The first of [`JOIN_POINTS`]' columns; then one for each, then [`JOIN_OTHER`].
    JoinFirst,
}

/// The join columns follow [`Column::JoinFirst`] in [`JOIN_POINTS`]' order; the last is the other joins'.
const JOIN_COLUMNS: usize = JOIN_POINTS.len() + 1;

/// How many columns there are.
pub const COLUMNS: usize = Column::JoinFirst as usize + JOIN_COLUMNS;

/// The column for joins not in [`JOIN_POINTS`] (and the unnamed join, address 0).
pub const JOIN_OTHER: usize = COLUMNS - 1;

/// The columns' titles (the frame log adds `_us` and `_n`).
pub const NAMES: [&str; COLUMNS] = [
    "st_u",
    "st_s1",
    "st_r",
    "st_r_before_cut",
    "st_cut_to_draw_end",
    "st_pass_tail",
    "st_after_pass",
    "st_s2",
    "st_p1",
    "st_w",
    "st_p2",
    "st_cut_to_idle",
    "st_draw_end_to_idle",
    "st_game_tail",
    "st_host_gap",
    "world_pass_00873200",
    "world_draw_00b6ba20",
    "world_draw_00b6b930",
    "extra_pass_1",
    "extra_pass_2",
    "camera_motion_008744a0",
    "sun_query_006426a0",
    "first_person_node_008746d0",
    "targets_00874b50",
    "accumulator_check_00b63a90",
    "refraction_normals_00874920",
    "first_person_setup_00874c10",
    "draw_rest_00875110",
    "water_00875e40",
    "target_marker_00a6fdb0",
    "interface_00875fd0",
    "interface_target_00876830",
    "interface_end_00876a20",
    "margins_00b540d0",
    "wind_down_00876850",
    "light_update_00714c20",
    "menu_pass_00875bf0",
    "job",
    "join_wait",
    "join_0086e650",
    "join_00873200",
    "join_008706b0",
    "join_00871dc0",
    "join_008746d0",
    "join_00874920",
    "join_00875110",
    "join_00875e40",
    "join_00876850",
    "join_00b63a90",
    "join_006426a0",
    "join_00b5fd60",
    "join_00b5d300",
    "join_00453dc0",
    "join_00878250",
    "join_0093c200",
    "join_0093cdf0",
    "join_0093d500",
    "join_00847df0",
    "join_008503b0",
    "join_other",
];

/// Marks inside the game's frame tail (from `WaitForThreads`' return, [`Point::WaitOut`], to the host's take,
/// [`Point::Host`]), in the order main passes them in `Main::OnIdle` (`0086e650`). Each ends one part of the tail:
/// the time since the last mark passed before it in the frame (or since `WaitForThreads` returned). The hitch log
/// prints the parts when the tail is long ([`take_tail`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum TailMark {
    /// In `Main::UpdateNonRenderSafeAITasks` (`0086f6a0`), the process lists' four steps one by one: the projectiles
    /// and explosions (`0096c240`; with the open dialogue's timer before it) returned.
    AiProjectiles,
    /// The actors' animation scene graphs (their bones: `ProcessLists::ParallelUpdateAnimationScenegraph`,
    /// `0096c970`, with more than one thread, else `UpdateAnimationScenegraph`, `0096c860`) returned.
    AiBones,
    /// The detection lists merged (`0096c710`) returned.
    AiDetection,
    /// The actors' process step and ragdolls (`009781d0`) returned.
    AiLists,
    /// The Pip-Boy's step and, with more than one thread, `TESObjectCELL::PerformQueuedChildAttaches` (`0054ae30`)
    /// returned.
    AiAttaches,
    /// The first packed task queue's end of main's input and its drain (`0087a6d0`, `0087a790`: the tasks the AI
    /// threads queued for main) returned.
    AiQueue,
    /// The queued node updates (`00552570`: the queued nodes' geometry, their morphs) returned.
    AiNodes,
    /// `Main::UpdateNonRenderSafeAITasks` returned: the rest (main's input to the first queue begun again, the
    /// interface's steps `007034c0` and `00703490`, the AI groups' clean-up `00991600`).
    AiTasks,
    /// `Script::RunDelayedScriptActionsOnReferences` (`005aa720`), first in `00870610`, returned.
    DelayedScripts,
    /// The secondary packed task queue's end of main's input (`0087a710`) returned.
    QueueEndInput,
    /// Its drain (`0087a850`: `0087a7d0` on the secondary queue) returned.
    QueueDrain,
    /// Main's input to it begun again (`0087a6f0`) returned.
    QueueBeginInput,
    /// `00870610` returned: the queued menu open (`Interface::HandleQueuedMenuOpen`, `007094f0`), the support menus
    /// closed when flagged (`00870680`), Tweaks' and `DebugText`'s queued prints.
    PostThreads,
    /// Every frame, at the 256-step tick's test: the other steps done (`0070ed10`'s menu check, `00a29680`,
    /// `005ae270`, `005a9d60`, the test itself).
    TickIn,
    /// The tick (`00aa7290`, when the test passed: in a menu, in the vanity camera, else once every 45 s) returned:
    /// the memory manager's free-list sort of one pool, under the pool's spin lock.
    TickOut,
    /// `Main::OnIdle` returned (from the tick, or from the test on a frame with no tick).
    IdleEnd,
}

/// How many tail marks there are.
pub const TAIL_MARKS: usize = 16;
/// The tail's parts: one ending at each mark, then the last to the host's take.
pub const TAIL_PARTS: usize = TAIL_MARKS + 1;

/// The tail's parts in words, in [`TailMark`]'s order, then the last mark to the host's take.
pub const TAIL_PART_NAMES: [&str; TAIL_PARTS] = [
    "AI tasks: projectiles and explosions",
    "AI tasks: animation scene graphs (bones)",
    "AI tasks: detection lists",
    "AI tasks: actors' process and ragdolls",
    "AI tasks: Pip-Boy and child attaches",
    "AI tasks: first task queue",
    "AI tasks: queued node updates",
    "AI tasks: the rest",
    "delayed script actions",
    "task queue end input",
    "task queue drain",
    "task queue begin input",
    "queued menu open, support menus and prints",
    "other steps",
    "free-list sort",
    "the end of OnIdle",
    "the game's loop to the host",
];

/// The packed task queues main drains in its frame tail ([`drain`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum DrainQueue {
    /// The first queue, drained in `Main::UpdateNonRenderSafeAITasks` (`0087a790`).
    First,
    /// The secondary queue, drained in `00870610` (`0087a850`).
    Secondary,
}

/// How many kinds of task a drain keeps apart; the rest are counted together.
pub const DRAIN_KINDS: usize = 6;

/// One frame's drains of a packed task queue on main (`0087a7d0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrainStats {
    /// The drains' whole time (ns).
    pub ns: u64,
    /// The tasks run.
    pub tasks: u32,
    /// The turns of the loop that found the queue empty while another thread's input to it was still open: the
    /// drain goes round again until every input is closed (main spins, nothing blocks).
    pub waits: u32,
    /// The kinds of task (the record's first word, its tag) in the order first seen: (tag, tasks, ns in them); an
    /// entry with no tasks is unused.
    pub kinds: [(u32, u32, u64); DRAIN_KINDS],
    /// The tasks of the kinds past [`DRAIN_KINDS`]: (tasks, ns).
    pub other: (u32, u64),
}

impl DrainStats {
    /// The time in the tasks themselves (ns): the rest of [`DrainStats::ns`] is the pops and the waits for input.
    pub fn task_ns(&self) -> u64 {
        self.kinds.iter().map(|k| k.2).sum::<u64>() + self.other.1
    }

    /// Adds `n` tasks of kind `tag` that ran `ns` in all.
    fn add_tasks(&mut self, tag: u32, n: u32, ns: u64) {
        self.tasks += n;
        let slot = self
            .kinds
            .iter()
            .position(|k| k.1 != 0 && k.0 == tag)
            .or_else(|| self.kinds.iter().position(|k| k.1 == 0));
        match slot {
            Some(i) => {
                let k = &mut self.kinds[i];
                *k = (tag, k.1 + n, k.2 + ns);
            }
            None => self.other = (self.other.0 + n, self.other.1 + ns),
        }
    }

    /// Adds another drain of the same queue in the frame.
    fn merge(&mut self, other: &DrainStats) {
        self.ns += other.ns;
        self.waits += other.waits;
        for &(tag, n, ns) in other.kinds.iter().filter(|k| k.1 != 0) {
            self.add_tasks(tag, n, ns);
        }
        self.tasks += other.other.0;
        self.other = (self.other.0 + other.other.0, self.other.1 + other.other.1);
    }
}

/// A drain being timed ([`drain`]): kept on main's stack while the queue is drained.
pub struct Drain {
    queue: DrainQueue,
    /// [`now_ns`] at the start; 0 while off.
    start: u64,
    stats: DrainStats,
}

/// Starts timing a drain of `queue` (main's, in the frame tail). Off: a recorder that reads no clock.
#[inline]
pub fn drain(queue: DrainQueue) -> Drain {
    Drain {
        queue,
        start: if points_on() { now_ns() } else { 0 },
        stats: DrainStats::default(),
    }
}

impl Drain {
    /// Whether the drain is timed (the points on).
    #[inline]
    pub fn on(&self) -> bool {
        self.start != 0
    }

    /// A task's start (the clock; 0 while off).
    #[inline]
    pub fn task_start(&self) -> u64 {
        if self.on() {
            now_ns()
        } else {
            0
        }
    }

    /// The task of kind `tag` started at `started` ([`Drain::task_start`]) ended.
    #[inline]
    pub fn task_end(&mut self, tag: u32, started: u64) {
        if self.on() {
            self.stats
                .add_tasks(tag, 1, now_ns().saturating_sub(started));
        }
    }

    /// The loop went round again on an empty queue (another thread's input still open).
    #[inline]
    pub fn waited(&mut self) {
        if self.on() {
            self.stats.waits += 1;
        }
    }

    /// The drain ended: its time and tasks join the frame's for its queue.
    pub fn end(mut self) {
        if !self.on() {
            return;
        }
        self.stats.ns = now_ns().saturating_sub(self.start);
        let mut drains = DRAINS.lock().unwrap_or_else(PoisonError::into_inner);
        match &mut drains[self.queue as usize] {
            Some(d) => d.merge(&self.stats),
            none => *none = Some(self.stats),
        }
    }
}

/// The frame's drains on main, by [`DrainQueue`].
static DRAINS: Mutex<[Option<DrainStats>; 2]> = Mutex::new([None, None]);

/// The 256-step tick of the frame: its pool index, whether the pool exists, and its free list's count (`+0x54`)
/// before the sort, packed (bit 63 set when passed).
static TICK: AtomicU64 = AtomicU64::new(0);

/// The 256-step tick is about to sort pool `index` of the memory manager's table (`011f67b8`): `free` is its free
/// list's count before the sort, `None` when the table has no pool there. Off: nothing.
#[inline]
pub fn tail_tick(index: u32, free: Option<u32>) {
    if points_on() {
        let present = u64::from(free.is_some()) << 62;
        TICK.store(
            (1 << 63) | present | (u64::from(index & 0xff) << 32) | u64::from(free.unwrap_or(0)),
            Relaxed,
        );
    }
}

/// Main passed tail mark `m` (see [`TailMark`]). Off: nothing. At `OnIdle`'s end main's own processor clock is read
/// too ([`main_clock`]).
#[inline]
pub fn tail_mark(m: TailMark) {
    if points_on() {
        MARKS.tail[m as usize].store(now_ns(), Relaxed);
        if m == TailMark::IdleEnd {
            main_clock(1);
        }
    }
}

/// Main's own processor clock and the wall clock in the same unit (`crate::thread_cpu::own_clock`), at
/// `WaitForThreads`' return (0) and at `OnIdle`'s end (1): how much of its tail main spent on the processor.
static MAIN_CLOCK: [[AtomicU64; 2]; 2] = [
    [AtomicU64::new(0), AtomicU64::new(0)],
    [AtomicU64::new(0), AtomicU64::new(0)],
];

/// Reads main's processor clock into [`MAIN_CLOCK`]`[at]` (none where the system does not tell it cheaply).
fn main_clock(at: usize) {
    let (cpu, wall) = crate::thread_cpu::own_clock().unwrap_or((0, 0));
    MAIN_CLOCK[at][0].store(cpu, Relaxed);
    MAIN_CLOCK[at][1].store(wall, Relaxed);
}

/// Main's share of the processor over its tail from `WaitForThreads`' return to `OnIdle`'s end, in thousandths
/// (`None` where the clock is not read or the frame passed neither point).
fn main_share() -> Option<u32> {
    let load = |at: usize, i: usize| MAIN_CLOCK[at][i].load(Relaxed);
    let (wall0, wall1) = (load(0, 1), load(1, 1));
    if wall0 == 0 || wall1 <= wall0 {
        return None;
    }
    let cpu = load(1, 0).saturating_sub(load(0, 0));
    Some((cpu.saturating_mul(1000) / (wall1 - wall0)).min(1000) as u32)
}

/// Tag 0x04 of the first packed task queue as main ran it in the frame: queued by `TES`'s update at its end on the
/// AI thread (`00453550` through `0087aa90`), run by `TaskQueueInterface::TaskUnpackFunc` (`0087b990`): the
/// physics results applied to the scene graph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HavokSync {
    /// The actors' Havok sync (`ActorHavokSyncParallelArrayTaskFunc`, `0087cfb0`, over the list at `011df244`, split
    /// between main and the parallel task thread by `BSParallelTaskManager`, `00c44c90`): its time (ns) and entries.
    pub actors_ns: u64,
    pub actors: u32,
    /// The nodes updated (`0087cf50` over the list at `011df1c4`, a node of more than ten children by each child,
    /// split the same way): its time (ns) and the nodes.
    pub nodes_ns: u64,
    pub nodes: u32,
    /// The cells' update after it (`00453900` on `TES`, when the list's gate is off) (ns).
    pub cells_ns: u64,
    /// How many times it ran in the frame.
    pub runs: u32,
}

/// The frame's tag 0x04 runs on main ([`havok_sync`]).
static HAVOK_SYNC: Mutex<Option<HavokSync>> = Mutex::new(None);

/// Times tag 0x04's three steps on main ([`HavokSync`]); off (the points off, or another thread running it when
/// the queue is not active) it reads no clock.
pub struct HavokSyncTimer {
    /// [`now_ns`] at the last step's end; 0 while off.
    last: u64,
    s: HavokSync,
}

/// Starts timing a run of tag 0x04 (main's, in its frame tail).
#[inline]
pub fn havok_sync() -> HavokSyncTimer {
    let on = points_on() && !crate::real_threads::on_other_game_thread();
    HavokSyncTimer {
        last: if on { now_ns() } else { 0 },
        s: HavokSync::default(),
    }
}

impl HavokSyncTimer {
    /// The time since the last step (ns), and the clock moved on.
    fn step(&mut self) -> u64 {
        let now = now_ns();
        let was = std::mem::replace(&mut self.last, now);
        now.saturating_sub(was)
    }

    /// The actors' Havok sync ended: `n` entries in its list.
    pub fn actors(&mut self, n: u32) {
        if self.last != 0 {
            self.s.actors_ns = self.step();
            self.s.actors = n;
        }
    }

    /// The node updates ended: `n` nodes.
    pub fn nodes(&mut self, n: u32) {
        if self.last != 0 {
            self.s.nodes_ns = self.step();
            self.s.nodes = n;
        }
    }

    /// The cells' update ended: the run joins the frame's.
    pub fn end(mut self) {
        if self.last == 0 {
            return;
        }
        self.s.cells_ns = self.step();
        let mut h = HAVOK_SYNC.lock().unwrap_or_else(PoisonError::into_inner);
        let t = h.get_or_insert_with(HavokSync::default);
        t.actors_ns += self.s.actors_ns;
        t.actors += self.s.actors;
        t.nodes_ns += self.s.nodes_ns;
        t.nodes += self.s.nodes;
        t.cells_ns += self.s.cells_ns;
        t.runs += 1;
    }
}

/// The game's frame tail of the frame just ended, split at its [`TailMark`]s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tail {
    /// Each part's time (ns), [`TAIL_PART_NAMES`]' order; `None` where its mark was not passed this frame.
    pub parts: [Option<u64>; TAIL_PARTS],
    /// The drains of the two packed task queues ([`DrainQueue`]'s order).
    pub drains: [Option<DrainStats>; 2],
    /// The 256-step tick: the pool's index and its free list's count before the sort (`None`: no pool there).
    pub tick: Option<(u32, Option<u32>)>,
    /// Tag 0x04's runs on main ([`HavokSync`]).
    pub havok_sync: Option<HavokSync>,
    /// Main's share of the processor from `WaitForThreads`' return to `OnIdle`'s end, in thousandths (`None` where
    /// the system does not tell it cheaply: Windows on ARM).
    pub main_share: Option<u32>,
}

impl Tail {
    /// The longest part: (its index, ns).
    pub fn longest(&self) -> Option<(usize, u64)> {
        self.parts
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.map(|ns| (i, ns)))
            .max_by_key(|p| p.1)
    }
}

/// The tail split of the frame just ended (after the host's [`Point::Host`], before the next frame's
/// [`Point::Idle`]): `None` when the points are off or the frame passed no `WaitForThreads` (a menu, one processor).
/// The drains are taken: a second call in the frame has none.
pub fn take_tail() -> Option<Tail> {
    if !points_on() {
        return None;
    }
    let drains = std::mem::take(&mut *DRAINS.lock().unwrap_or_else(PoisonError::into_inner));
    let tick = TICK.swap(0, Relaxed);
    let havok_sync = HAVOK_SYNC
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    let mut tail = MARKS.tail_parts()?;
    tail.drains = drains;
    tail.havok_sync = havok_sync;
    tail.main_share = main_share();
    tail.tick = (tick >> 63 != 0).then(|| {
        (
            ((tick >> 32) as u32) & 0xff,
            ((tick >> 62) & 1 != 0).then_some(tick as u32),
        )
    });
    Some(tail)
}

/// The values of one window: nanoseconds and calls per column.
pub struct Marks {
    ns: [AtomicU64; COLUMNS],
    calls: [AtomicU64; COLUMNS],
    /// When each point was passed this frame (nanoseconds since the clock's start plus 1), 0 not yet.
    at: [AtomicU64; POINTS],
    /// When each [`TailMark`] was passed this frame, 0 not yet.
    tail: [AtomicU64; TAIL_MARKS],
    /// How deep main is in the frame's draw (`008706b0` calls open now).
    depth: AtomicU64,
}

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU64 = AtomicU64::new(0);

impl Marks {
    /// No values, no points passed.
    pub const fn new() -> Marks {
        Marks {
            ns: [ZERO; COLUMNS],
            calls: [ZERO; COLUMNS],
            at: [ZERO; POINTS],
            tail: [ZERO; TAIL_MARKS],
            depth: AtomicU64::new(0),
        }
    }

    /// The frame's tail split at the marks passed (see [`TailMark`]): `None` when `WaitForThreads` did not return
    /// this frame or the host has not taken it. Read between [`Point::Host`] and the next [`Point::Idle`].
    pub fn tail_parts(&self) -> Option<Tail> {
        let from = self.at[Point::WaitOut as usize].load(Relaxed);
        let to = self.at[Point::Host as usize].load(Relaxed);
        if from == 0 || to == 0 {
            return None;
        }
        let mut tail = Tail::default();
        let mut last = from;
        for (i, m) in self.tail.iter().enumerate() {
            let at = m.load(Relaxed);
            // A mark of this frame's tail only (passed after `WaitForThreads` returned).
            if at >= last && at != 0 {
                tail.parts[i] = Some(at - last);
                last = at;
            }
        }
        tail.parts[TAIL_MARKS] = Some(to.saturating_sub(last));
        Some(tail)
    }

    /// Adds `ns` and one call to `column`.
    pub fn add(&self, column: usize, ns: u64) {
        self.ns[column].fetch_add(ns, Relaxed);
        self.calls[column].fetch_add(1, Relaxed);
    }

    /// Point `p` passed at `now` (nanoseconds, never 0): the stages ending there closed, then the point noted.
    /// [`Point::Idle`] forgets the frame's other points after closing its stages. Every other point counts once a
    /// frame, the first time (a second draw sequence in the frame, a save's screenshot or a menu opened in the tail,
    /// closes nothing and moves nothing; its steps still count, so a step's `_n` above 1 marks such a frame), and a
    /// draw (`008706b0`) nested inside the frame's draw neither starts nor ends it.
    pub fn point(&self, p: Point, now: u64) {
        // Points are main's only: the depth's read and write need not be one step.
        let depth = self.depth.load(Relaxed);
        match p {
            // A nested draw's start is not the frame's first: the once-a-frame rule below drops it.
            Point::DrawIn => self.depth.store(depth + 1, Relaxed),
            Point::DrawOut => {
                self.depth.store(depth.saturating_sub(1), Relaxed);
                if depth != 1 {
                    return;
                }
            }
            _ => {}
        }
        if p != Point::Idle && self.at[p as usize].load(Relaxed) != 0 {
            return;
        }
        for (column, from, to) in SEGMENTS {
            if to == p {
                let since = self.at[from as usize].load(Relaxed);
                if since != 0 {
                    self.add(column as usize, now.saturating_sub(since));
                }
            }
        }
        if p == Point::Idle {
            for a in self.at.iter().chain(&self.tail) {
                a.store(0, Relaxed);
            }
            self.depth.store(0, Relaxed);
        }
        self.at[p as usize].store(now, Relaxed);
    }

    /// A join at the entry of `at` waited `ns`: its column and the total.
    pub fn join(&self, at: u32, ns: u64) {
        let column = JOIN_POINTS
            .iter()
            .position(|&a| a == at)
            .map_or(JOIN_OTHER, |i| Column::JoinFirst as usize + i);
        self.add(column, ns);
        self.add(Column::JoinWait as usize, ns);
    }

    /// No values and no points (a fresh window).
    pub fn clear(&self) {
        let _ = self.take();
        for a in self.at.iter().chain(&self.tail) {
            a.store(0, Relaxed);
        }
        self.depth.store(0, Relaxed);
    }

    /// The values since the last take, per column: (microseconds, calls). The points stay.
    pub fn take(&self) -> [(f64, u64); COLUMNS] {
        let mut out = [(0.0, 0); COLUMNS];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (
                self.ns[i].swap(0, Relaxed) as f64 / 1000.0,
                self.calls[i].swap(0, Relaxed),
            );
        }
        out
    }
}

impl Default for Marks {
    fn default() -> Marks {
        Marks::new()
    }
}

/// The run's marks.
static MARKS: Marks = Marks::new();
/// 0: the variable not read yet; 1: off; 2: on; 3: on for [`on_here`]'s thread only (tests); 4: the points only (the
/// hitch log's stage split, [`set_points`]: about ten clock reads a frame, no step or join timed).
static STATE: AtomicU8 = AtomicU8::new(0);
static START: OnceLock<Instant> = OnceLock::new();

/// Whether the aid is on (`NV_STAGE_TIME` set to anything but `0` or empty; read once).
#[inline]
pub fn on() -> bool {
    match STATE.load(Relaxed) {
        0 => read_switch(),
        2 => true,
        3 => HERE.with(Cell::get),
        _ => false,
    }
}

/// Whether the points are timed ([`point`]): the aid on, or its points only ([`set_points`]).
#[inline]
pub fn points_on() -> bool {
    match STATE.load(Relaxed) {
        0 => read_switch(),
        2 | 4 => true,
        3 => HERE.with(Cell::get),
        _ => false,
    }
}

/// The hitch log's stage split: with the aid off (`NV_STAGE_TIME` unset) its points are timed all the same
/// ([`points_on`]); `on` false turns that off again. With the aid on, nothing changes.
pub fn set_points(on: bool) {
    if STATE.load(Relaxed) == 0 {
        read_switch();
    }
    let (from, to) = if on { (1, 4) } else { (4, 1) };
    let _ = STATE.compare_exchange(from, to, Relaxed, Relaxed);
}

#[cold]
#[inline(never)]
fn read_switch() -> bool {
    let on = std::env::var("NV_STAGE_TIME").is_ok_and(|v| !v.is_empty() && v != "0");
    let _ = STATE.compare_exchange(0, if on { 2 } else { 1 }, Relaxed, Relaxed);
    STATE.load(Relaxed) == 2
}

/// Sets the switch, whatever the variable says (tests).
#[doc(hidden)]
pub fn set_on(on: bool) {
    STATE.store(if on { 2 } else { 1 }, Relaxed);
}

thread_local! {
    /// The switch is on for this thread ([`on_here`]).
    static HERE: Cell<bool> = const { Cell::new(false) };
    /// [`on_here`]'s clock for this thread (nanoseconds plus 1), moved only by [`advance`]; `None`: the real
    /// clock.
    static CLOCK: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Serialises the tests that switch the aid on for their own thread.
static HERE_LOCK: Mutex<()> = Mutex::new(());

/// The run's marks, the frame tail's drains and its tick forgotten.
fn clear_all() {
    MARKS.clear();
    forget_tail_work();
}

/// The frame tail's drains and tick forgotten.
fn forget_tail_work() {
    *DRAINS.lock().unwrap_or_else(PoisonError::into_inner) = [None, None];
    TICK.store(0, Relaxed);
    *HAVOK_SYNC.lock().unwrap_or_else(PoisonError::into_inner) = None;
    for a in MAIN_CLOCK.iter().flatten() {
        a.store(0, Relaxed);
    }
}

/// The switch on for the calling thread only, until the guard drops (the engine crates' tests of the marks'
/// places: other tests in the same process, on other threads, record nothing meanwhile). The values start empty.
/// The thread's marks read a test clock that stands still until [`advance`] moves it, so the tests' times are
/// exact (a real sleep rounds up to the timer tick and oversleeps on a loaded machine).
#[doc(hidden)]
pub fn on_here() -> Here {
    let lock = HERE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    HERE.with(|h| h.set(true));
    CLOCK.with(|c| c.set(Some(1)));
    STATE.store(3, Relaxed);
    clear_all();
    Here { _lock: lock }
}

/// [`on_here`]'s guard: the switch back off when dropped.
#[doc(hidden)]
pub struct Here {
    _lock: MutexGuard<'static, ()>,
}

/// Moves the calling thread's [`on_here`] test clock on by `ns` (no test clock: nothing).
#[doc(hidden)]
pub fn advance(ns: u64) {
    CLOCK.with(|c| c.set(c.get().map(|t| t + ns)));
}

impl Drop for Here {
    fn drop(&mut self) {
        HERE.with(|h| h.set(false));
        CLOCK.with(|c| c.set(None));
        STATE.store(1, Relaxed);
        clear_all();
    }
}

/// Nanoseconds since the clock's start, plus 1 (so never 0); [`on_here`]'s test clock on its thread. Read only
/// while the aid is on. Only [`on_here`] stores state 3, so a run (state 1 or 2) never reads the test clock.
fn now_ns() -> u64 {
    if STATE.load(Relaxed) == 3 {
        if let Some(t) = CLOCK.with(Cell::get) {
            return t;
        }
    }
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64 + 1
}

/// Main passed `p` (see [`Point`]). Off: nothing.
#[inline]
pub fn point(p: Point) {
    if points_on() {
        MARKS.point(p, now_ns());
        if p == Point::WaitOut {
            main_clock(0);
        }
        if p == Point::Idle {
            // A frame starts: the last frame's drains and tick are forgotten if nothing took them.
            forget_tail_work();
        }
    }
}

/// A join of the drawing thread at the entry of `at` (0: none named) made main wait `ns`. Off: nothing.
#[inline]
pub fn join_wait(at: u32, ns: u64) {
    if on() {
        MARKS.join(at, ns);
    }
}

/// The drawing job ran `ns` (on the helper). Off: nothing.
#[inline]
pub fn job(ns: u64) {
    if on() {
        MARKS.add(Column::Job as usize, ns);
    }
}

/// A step's body: timed into its column until dropped.
pub struct Scope {
    column: Column,
    /// [`now_ns`] at the start; `None` while off.
    start: Option<u64>,
}

/// Starts timing `column` (a step). Off: a guard that does nothing.
#[inline]
pub fn scope(column: Column) -> Scope {
    Scope {
        column,
        start: on().then(now_ns),
    }
}

impl Drop for Scope {
    #[inline]
    fn drop(&mut self) {
        if let Some(start) = self.start {
            MARKS.add(self.column as usize, now_ns().saturating_sub(start));
        }
    }
}

/// The run's values since the last take (the frame log's columns, [`NAMES`]' order).
pub fn take() -> [(f64, u64); COLUMNS] {
    MARKS.take()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_match_the_columns() {
        assert_eq!(NAMES.len(), COLUMNS);
        assert_eq!(NAMES[Column::JoinFirst as usize], "join_0086e650");
        assert_eq!(NAMES[JOIN_OTHER], "join_other");
        for (i, a) in JOIN_POINTS.iter().enumerate() {
            assert_eq!(
                NAMES[Column::JoinFirst as usize + i],
                format!("join_{a:08x}")
            );
        }
        let mut seen = std::collections::HashSet::new();
        assert!(NAMES.iter().all(|n| seen.insert(*n)), "a title twice");
    }

    /// A frame's points close its stages; OnIdle's entry closes the previous frame's last ones and forgets the
    /// rest; a stage whose start was not passed this frame adds nothing.
    #[test]
    fn points_close_the_stages_that_end_there() {
        let m = Marks::new();
        m.point(Point::Idle, 1_000);
        m.point(Point::SwapIn, 3_000);
        m.point(Point::DrawIn, 3_500);
        m.point(Point::Cut, 5_000);
        m.point(Point::Cut, 5_200); // a second cut in the frame: the first counts
                                    // A draw nested inside the frame's draw neither starts nor ends it.
        m.point(Point::DrawIn, 5_300);
        m.point(Point::DrawOut, 5_400);
        m.point(Point::DrawEnd, 9_000);
        m.point(Point::PassEnd, 9_400);
        m.point(Point::DrawOut, 10_000);
        m.point(Point::SwapOut, 10_100);
        // A second draw sequence in the tail (a save's screenshot): no stage moves.
        for (p, at) in [
            (Point::DrawIn, 10_200),
            (Point::Cut, 10_300),
            (Point::DrawEnd, 10_500),
            (Point::PassEnd, 10_600),
            (Point::DrawOut, 10_700),
        ] {
            m.point(p, at);
        }
        m.point(Point::WaitIn, 12_000);
        m.point(Point::WaitOut, 12_000);
        let v = m.take();
        let us = |c: Column| v[c as usize];
        assert_eq!(us(Column::U), (2.0, 1));
        assert_eq!(us(Column::S1), (0.5, 1));
        assert_eq!(us(Column::R), (6.5, 1));
        assert_eq!(us(Column::RBeforeCut), (1.5, 1));
        assert_eq!(us(Column::CutToDrawEnd), (4.0, 1));
        assert_eq!(us(Column::PassTail), (0.4, 1));
        assert_eq!(us(Column::AfterPass), (0.6, 1));
        assert_eq!(us(Column::S2), (0.1, 1));
        assert_eq!(us(Column::P1), (1.9, 1));
        assert_eq!(us(Column::W), (0.0, 1));
        // Not closed yet: they end at the next OnIdle.
        assert_eq!(us(Column::P2), (0.0, 0));
        assert_eq!(us(Column::CutToIdle), (0.0, 0));
        m.point(Point::Idle, 15_000);
        let v = m.take();
        assert_eq!(v[Column::P2 as usize], (3.0, 1));
        assert_eq!(v[Column::CutToIdle as usize], (10.0, 1));
        assert_eq!(v[Column::DrawEndToIdle as usize], (6.0, 1));
        assert_eq!(v[Column::U as usize], (0.0, 0));
        // A frame with no draw: Swap's stages close, the draw's do not, and nothing from the last frame leaks in.
        m.point(Point::SwapIn, 16_000);
        m.point(Point::SwapOut, 17_000);
        m.point(Point::Idle, 20_000);
        let v = m.take();
        assert_eq!(v[Column::U as usize], (1.0, 1));
        assert_eq!(v[Column::S2 as usize], (0.0, 0));
        assert_eq!(v[Column::CutToIdle as usize], (0.0, 0));
        assert_eq!(v[Column::P2 as usize], (0.0, 0));
        // A take empties the values.
        assert!(m.take().iter().all(|&(us, n)| us == 0.0 && n == 0));
    }

    /// Each join counts in its own column (by address) and in the total; an unlisted one in the other joins'.
    #[test]
    fn joins_count_by_address_and_in_the_total() {
        let m = Marks::new();
        m.join(0x0087_5110, 2_000);
        m.join(0x0087_5110, 500);
        m.join(0x0086_e650, 1_000);
        m.join(0, 7_000);
        m.join(0x1234_5678, 1_000);
        let v = m.take();
        let col =
            |a: u32| Column::JoinFirst as usize + JOIN_POINTS.iter().position(|&x| x == a).unwrap();
        assert_eq!(v[col(0x0087_5110)], (2.5, 2));
        assert_eq!(v[col(0x0086_e650)], (1.0, 1));
        assert_eq!(v[JOIN_OTHER], (8.0, 2));
        assert_eq!(v[Column::JoinWait as usize], (11.5, 5));
    }

    /// The switch, on the run's own marks (one test, so no other test flips it meanwhile): off, no point, scope,
    /// join or job leaves a value, and a scope reads no clock; on, they do.
    #[test]
    fn switch_off_marks_nothing_and_on_marks() {
        // Not while `on_here_reads_its_test_clock` holds the switch.
        let _lock = HERE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        set_on(false);
        let _ = take();
        point(Point::Idle);
        point(Point::SwapIn);
        {
            let s = scope(Column::DrawRest);
            assert!(s.start.is_none(), "off: no clock read");
        }
        join_wait(0x0087_5110, 1_000);
        job(1_000);
        point(Point::Idle);
        assert!(take().iter().all(|&(us, n)| us == 0.0 && n == 0));
        assert!(
            MARKS.at.iter().all(|a| a.load(Relaxed) == 0),
            "off: no point noted"
        );

        set_on(true);
        point(Point::Idle);
        point(Point::SwapIn);
        drop(scope(Column::DrawRest));
        join_wait(0x0087_5110, 1_000);
        job(2_000);
        // A frame start clears the points (nothing ends there in this frame).
        point(Point::Idle);
        assert!(MARKS
            .at
            .iter()
            .enumerate()
            .all(|(i, a)| (i == Point::Idle as usize) == (a.load(Relaxed) != 0)));
        let v = take();
        set_on(false);
        // The seam's own tests may join or run a job meanwhile: only columns they never touch are exact.
        assert_eq!(v[Column::U as usize].1, 1);
        assert_eq!(v[Column::DrawRest as usize].1, 1);
        let j = Column::JoinFirst as usize
            + JOIN_POINTS.iter().position(|&a| a == 0x0087_5110).unwrap();
        assert_eq!(v[j], (1.0, 1));
        assert!(v[Column::JoinWait as usize].1 >= 1);
        assert!(v[Column::Job as usize].1 >= 1);
    }

    /// The points-only level (the hitch log's): points close their stages, scopes, joins and jobs read no clock and
    /// leave nothing; the aid's full switch on is not lowered by it, and turning it off gives the aid's state back.
    #[test]
    fn points_only_times_the_points_and_nothing_else() {
        let _lock = HERE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        set_on(false);
        let _ = take();
        set_points(true);
        assert!(points_on() && !on());
        point(Point::Idle);
        point(Point::SwapIn);
        {
            let s = scope(Column::DrawRest);
            assert!(s.start.is_none(), "points only: a scope reads no clock");
        }
        join_wait(0x0087_5110, 1_000);
        point(Point::Idle);
        let v = take();
        assert_eq!(v[Column::U as usize].1, 1, "the update stage closed");
        assert_eq!(v[Column::DrawRest as usize], (0.0, 0));
        let j = Column::JoinFirst as usize
            + JOIN_POINTS.iter().position(|&a| a == 0x0087_5110).unwrap();
        assert_eq!(v[j], (0.0, 0));
        set_points(false);
        assert!(!points_on());
        // With the aid on, the hitch log's switch neither lowers nor raises it.
        set_on(true);
        set_points(true);
        assert!(on());
        set_points(false);
        assert!(on());
        set_on(false);
        MARKS.clear();
    }

    /// [`on_here`]'s marks read its test clock: points and scopes measure exactly what [`advance`] moved,
    /// nothing while it stands still; the guard's drop gives the real clock back.
    #[test]
    fn on_here_reads_its_test_clock() {
        {
            let _here = on_here();
            point(Point::Idle);
            point(Point::SwapIn);
            {
                let _s = scope(Column::DrawRest);
                advance(7_000);
            }
            advance(1_500);
            point(Point::DrawIn);
            point(Point::Idle);
            let v = take();
            assert_eq!(v[Column::U as usize], (0.0, 1));
            assert_eq!(v[Column::DrawRest as usize], (7.0, 1));
            assert_eq!(v[Column::S1 as usize], (8.5, 1));
            assert_eq!(v[Column::P2 as usize], (0.0, 0));
        }
        assert!(CLOCK.with(Cell::get).is_none(), "the real clock again");
        let a = now_ns();
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(now_ns() > a);
    }

    /// The frame tail splits at its marks in the order passed: each part runs from the mark passed before it (or
    /// `WaitForThreads`' return), a mark not passed after that return gives no part, and the last part runs to the
    /// host's take. The drains and the tick are taken with it; a frame with no `WaitForThreads` has no tail.
    #[test]
    fn the_frame_tail_splits_at_its_marks() {
        let _here = on_here();
        point(Point::Idle);
        // Before `WaitForThreads` returned: not part of the tail.
        tail_mark(TailMark::QueueDrain);
        advance(1_000);
        point(Point::WaitOut);
        advance(2_000);
        let mut d = drain(DrainQueue::First);
        assert!(d.on());
        let t = d.task_start();
        advance(300);
        d.task_end(0x07, t);
        d.waited();
        advance(100);
        d.end();
        tail_mark(TailMark::AiQueue);
        tail_mark(TailMark::AiTasks);
        advance(500);
        tail_mark(TailMark::DelayedScripts);
        // One processor: no task queue marks.
        advance(4_000);
        tail_mark(TailMark::PostThreads);
        advance(100);
        tail_tick(37, Some(1_203));
        tail_mark(TailMark::TickIn);
        advance(8_000);
        tail_mark(TailMark::TickOut);
        tail_mark(TailMark::IdleEnd);
        advance(50);
        point(Point::Host);
        let tail = take_tail().expect("a tail");
        assert_eq!(
            tail.parts,
            [
                None,
                None,
                None,
                None,
                None,
                Some(2_400),
                None,
                Some(0),
                Some(500),
                None,
                None,
                None,
                Some(4_000),
                Some(100),
                Some(8_000),
                Some(0),
                Some(50)
            ]
        );
        assert_eq!(tail.longest(), Some((14, 8_000)));
        assert_eq!(tail.tick, Some((37, Some(1_203))));
        let first = tail.drains[DrainQueue::First as usize].expect("the first queue's drain");
        assert_eq!(
            (first.tasks, first.waits, first.ns, first.task_ns()),
            (1, 1, 400, 300)
        );
        assert_eq!(first.kinds[0], (0x07, 1, 300));
        assert!(tail.drains[DrainQueue::Secondary as usize].is_none());
        let again = take_tail().expect("the marks stay until the next frame");
        assert_eq!((again.drains, again.tick), ([None, None], None), "taken");
        point(Point::Idle);
        tail_mark(TailMark::AiTasks);
        point(Point::Host);
        assert!(take_tail().is_none(), "no WaitForThreads in this frame");
        tail_tick(3, None);
        point(Point::WaitOut);
        point(Point::Host);
        assert_eq!(
            take_tail().expect("a tail").tick,
            Some((3, None)),
            "no pool at that index"
        );
        // A frame with no tick (the usual one: outside menus the tick runs once every 45 s): the other steps end at
        // the tick's test, the sort is not passed, and OnIdle ends right after the test.
        point(Point::Idle);
        point(Point::WaitOut);
        tail_mark(TailMark::PostThreads);
        advance(300);
        tail_mark(TailMark::TickIn);
        advance(5);
        tail_mark(TailMark::IdleEnd);
        point(Point::Host);
        let t = take_tail().expect("a tail");
        assert_eq!(
            (t.parts[13], t.parts[14], t.parts[15]),
            (Some(300), None, Some(5))
        );
        assert_eq!(TAIL_PART_NAMES[13], "other steps");
        assert_eq!(t.tick, None);
    }

    /// A drain keeps six kinds of task apart (the first seen) and counts the rest together; a second drain of the
    /// same queue in the frame adds to the first.
    #[test]
    fn a_drain_keeps_six_kinds_apart_and_adds_a_second_drain() {
        let mut s = DrainStats::default();
        for tag in 1..=8u32 {
            s.add_tasks(tag, 1, u64::from(tag) * 10);
        }
        s.add_tasks(3, 2, 5);
        assert_eq!(s.tasks, 10);
        assert_eq!(s.kinds[2], (3, 3, 35));
        assert_eq!(s.other, (2, 150));
        let mut t = DrainStats {
            ns: 1_000,
            waits: 4,
            ..DrainStats::default()
        };
        t.add_tasks(3, 1, 1);
        t.add_tasks(9, 1, 2);
        s.ns = 500;
        s.merge(&t);
        assert_eq!((s.tasks, s.ns, s.waits), (12, 1_500, 4));
        assert_eq!(s.kinds[2], (3, 4, 36));
        assert_eq!(s.other, (3, 152));
        assert_eq!(s.task_ns(), 10 + 20 + 36 + 40 + 50 + 60 + 152);
    }

    /// Off, a drain reads no clock and leaves nothing; no tail is taken.
    #[test]
    fn off_a_drain_records_nothing() {
        let _lock = HERE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        set_on(false);
        let mut d = drain(DrainQueue::Secondary);
        assert!(!d.on());
        assert_eq!(d.task_start(), 0);
        d.task_end(1, 0);
        d.waited();
        d.end();
        tail_mark(TailMark::AiTasks);
        tail_tick(1, Some(2));
        assert!(DRAINS.lock().unwrap().iter().all(Option::is_none));
        assert_eq!(TICK.load(Relaxed), 0);
        let mut h = havok_sync();
        h.actors(1);
        h.end();
        assert!(HAVOK_SYNC.lock().unwrap().is_none());
        assert!(take_tail().is_none());
    }

    /// Tag 0x04's three steps are timed into the frame's tail, two runs added; main's processor share of the tail
    /// is a thousandth count where the system tells it.
    #[test]
    fn the_havok_sync_steps_and_main_s_share_come_with_the_tail() {
        let _here = on_here();
        point(Point::Idle);
        point(Point::WaitOut);
        for _ in 0..2 {
            let mut h = havok_sync();
            advance(100);
            h.actors(3);
            advance(5_000);
            h.nodes(40);
            advance(10);
            h.end();
        }
        tail_mark(TailMark::IdleEnd);
        point(Point::Host);
        let t = take_tail().expect("a tail");
        assert_eq!(
            t.havok_sync,
            Some(HavokSync {
                actors_ns: 200,
                actors: 6,
                nodes_ns: 10_000,
                nodes: 80,
                cells_ns: 20,
                runs: 2,
            })
        );
        if crate::thread_cpu::own_clock().is_some() {
            assert!(t.main_share.is_some_and(|s| s <= 1000), "{t:?}");
        } else {
            assert_eq!(t.main_share, None);
        }
        point(Point::Idle);
        point(Point::WaitOut);
        point(Point::Host);
        assert_eq!(take_tail().expect("a tail").havok_sync, None, "per frame");
    }
}
