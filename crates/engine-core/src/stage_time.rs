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

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering::Relaxed};
use std::sync::OnceLock;
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
    /// The world pass (`00873200`) calls the world draw's first half (`00b6ba20`): the cut. Only the first of a
    /// frame counts.
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
}

/// How many points there are.
pub const POINTS: usize = 10;

/// The stages: (column, the point that starts it, the point that ends it).
pub const SEGMENTS: [(Column, Point, Point); 13] = [
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

/// The values of one window: nanoseconds and calls per column.
pub struct Marks {
    ns: [AtomicU64; COLUMNS],
    calls: [AtomicU64; COLUMNS],
    /// When each point was passed this frame (nanoseconds since the clock's start plus 1), 0 not yet.
    at: [AtomicU64; POINTS],
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
        }
    }

    /// Adds `ns` and one call to `column`.
    pub fn add(&self, column: usize, ns: u64) {
        self.ns[column].fetch_add(ns, Relaxed);
        self.calls[column].fetch_add(1, Relaxed);
    }

    /// Point `p` passed at `now` (nanoseconds, never 0): the stages ending there closed, then the point noted.
    /// [`Point::Idle`] forgets the frame's other points after closing its stages; only a frame's first
    /// [`Point::Cut`] counts (a later one closes nothing and moves nothing).
    pub fn point(&self, p: Point, now: u64) {
        if p == Point::Cut && self.at[p as usize].load(Relaxed) != 0 {
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
            for a in &self.at {
                a.store(0, Relaxed);
            }
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
/// 0: the variable not read yet; 1: off; 2: on.
static STATE: AtomicU8 = AtomicU8::new(0);
static START: OnceLock<Instant> = OnceLock::new();

/// Whether the aid is on (`NV_STAGE_TIME` set to anything but `0` or empty; read once).
#[inline]
pub fn on() -> bool {
    match STATE.load(Relaxed) {
        0 => read_switch(),
        s => s == 2,
    }
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

/// Nanoseconds since the clock's start, plus 1 (so never 0).
fn now_ns() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64 + 1
}

/// Main passed `p` (see [`Point`]). Off: nothing.
#[inline]
pub fn point(p: Point) {
    if on() {
        MARKS.point(p, now_ns());
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
    start: Option<Instant>,
}

/// Starts timing `column` (a step). Off: a guard that does nothing.
#[inline]
pub fn scope(column: Column) -> Scope {
    Scope {
        column,
        start: on().then(Instant::now),
    }
}

impl Drop for Scope {
    #[inline]
    fn drop(&mut self) {
        if let Some(start) = self.start {
            MARKS.add(self.column as usize, start.elapsed().as_nanos() as u64);
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
        m.point(Point::DrawEnd, 9_000);
        m.point(Point::PassEnd, 9_400);
        m.point(Point::DrawOut, 10_000);
        m.point(Point::SwapOut, 10_100);
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
        point(Point::Idle);
        let _ = take();
    }
}
