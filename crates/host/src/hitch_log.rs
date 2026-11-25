//! The hitch log, on in every run (`NV_HITCH_LOG=0` turns it off): each frame a player could feel as a jump writes
//! one `hitch:` line to the log, saying how long the frame was against the frames around it, where the player was,
//! where in the frame the time went, the one-off work counted in it (`engine_core::hitch`, `engine_platform::stream_hitch`) and
//! a plain-word guess of the main cause. An F12 report carries the session's lines (`hitches.txt`). Nothing in the
//! game changes, or when it runs: the log only reads counters and memory between two of main's frames.
//!
//! **A hitch** is a frame of at least [`FLOOR_MS`] (8 ms; 6 ms where the typical frame is under 4 ms, above 250 fps) and at least twice the typical frame (the median of the
//! last [`WINDOW`] frames that were not loading screens). Twice the typical frame is the goal's own test (a frame
//! twice as long as those around it is a visible jump at any frame rate); the 8 ms floor keeps out the doubling of
//! a 3 ms frame, which no screen shows, while a 10-15 ms frame in a 5 ms stream (the 200 fps target on a fast
//! screen) is still caught. Lines of [`BIG_MS`] (16 ms, a whole 60 Hz refresh) or more say `big`. Loading-screen
//! frames are neither judged nor counted in the median; the first frame after one is judged and its line says so. Play
//! and menus keep a typical frame each (a menu runs at its own frame limit); a frame is judged against the kind the
//! frame before it showed, so the frame that opens the Pip-Boy is judged against play. A loading screen is the
//! game's own (`uncap::screen`), a game loaded in the frame, or the loading screen's frames shown inside one of
//! main's frames (a `coc`, a door, fast travel: more than two presents in the frame).
//! At most [`BURST`] lines are written in any [`BURST_SECS`] seconds; the next line written says how many were not.
//! A measured run (`NV_FRAME_LOG` set) writes every one.
//!
//! **What it costs** (`a_normal_frame_costs_microseconds`: about 1.9 microseconds in a debug build): a frame that is not a hitch
//! takes two counter snapshots (about 60 relaxed loads), the stage marks' take and a median every 16 frames over
//! 128 values: a few microseconds. The counters cost one relaxed add at each counted event (heap blocks: a plain
//! load and store under the heap's lock) and, for waits and reads, a clock read pair around a call that already
//! blocks or reads the disk. The stage split costs about ten clock reads a frame (`engine_core::stage_time`'s
//! points), the frame tail's split sixteen more, main's own processor clock twice (about a microsecond each)
//! and two for each task main drains from the task queues there. Main's time
//! in the guest heap and in each script run is timed only with `NV_HITCH_HEAP=1` (two clock reads a block or a run:
//! about 4 ms on the pause menu's 80,000-block opening frame). A hitch frame adds one formatted line, handed to the
//! log's own thread, which writes it ([`Lines`]: a write that blocks never holds main), and, when the player's cell
//! changed since the last line, one call of the cell's `GetEditorID`.

use engine::Engine;
use engine_core::hitch::{Count, WaitKind, COUNTS};
use engine_core::stage_time::Column;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

/// A frame shorter than this is never a hitch (ms).
pub const FLOOR_MS: f64 = 8.0;
/// The floor where the typical frame is under [`FAST_MS`] (above 250 fps: a small interior): a 6 ms frame there is
/// a jump of three frames or more, which a fast screen shows.
pub const FAST_FLOOR_MS: f64 = 6.0;
pub const FAST_MS: f64 = 4.0;
/// A hitch at least this long is called big (ms).
pub const BIG_MS: f64 = 16.0;
/// A frame that showed more than this many presents showed the loading screen's frames (a normal frame shows one).
pub const LOADING_PRESENTS: u32 = 2;
/// The frames the typical frame is the median of.
pub const WINDOW: usize = 128;
/// How often the median is worked out again (frames).
const MEDIAN_EVERY: u32 = 16;
/// The frames needed before any is judged.
const WARM: usize = 16;
/// At most this many lines in any [`BURST_SECS`] seconds.
pub const BURST: usize = 20;
pub const BURST_SECS: f64 = 10.0;

/// `PlayerCharacter` and its parent cell (`008d6f30`).
const PLAYER: u32 = 0x011d_ea3c;
/// `Tile::iHibernatingTileCount` (Xbox PDB): the interface's hibernating tiles.
const HIBERNATING_TILE_COUNT: u32 = 0x011f_32d8;
const PARENT_CELL: u32 = 0x40;
const POSITION: u32 = 0x30;

/// The recent frames a lasting change of rate is told by, and how many of them past the threshold make one.
const RECENT: usize = 8;
const LASTING: usize = 4;

/// What a frame is, against the typical frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    /// Not a hitch (or not judged yet).
    Normal,
    /// A hitch: the typical frame before it.
    Hitch(f64),
    /// [`LASTING`] of the last [`RECENT`] frames were past the threshold: not single jumps but a new rate (a new
    /// scene after a load, the window losing focus, a slow stretch). The typical frame starts again from the recent
    /// frames: (the typical frame before, the new one).
    Slowed(f64, f64),
}

/// The typical frame and the hitch test.
#[derive(Clone)]
pub struct Detector {
    ring: Vec<f64>,
    at: usize,
    median: f64,
    since: u32,
    /// The last [`RECENT`] frames and whether each was past the threshold.
    recent: VecDeque<(f64, bool)>,
}

impl Default for Detector {
    fn default() -> Detector {
        Detector {
            ring: Vec::with_capacity(WINDOW),
            at: 0,
            median: 0.0,
            since: MEDIAN_EVERY,
            recent: VecDeque::with_capacity(RECENT + 1),
        }
    }
}

impl Detector {
    /// A frame of `ms` that was not a loading screen: whether it is a hitch against the typical frame before it (or
    /// the start of a new rate), then it joins the frames the median is taken over.
    pub fn frame(&mut self, ms: f64) -> Verdict {
        let over = self.ring.len() >= WARM && ms >= floor(self.median) && ms >= 2.0 * self.median;
        self.recent.push_back((ms, over));
        if self.recent.len() > RECENT {
            self.recent.pop_front();
        }
        if over && self.recent.iter().filter(|r| r.1).count() >= LASTING {
            let before = self.median;
            self.ring = self.recent.iter().map(|r| r.0).collect();
            self.recent.clear();
            self.at = 0;
            self.since = 0;
            self.median = median(&self.ring);
            return Verdict::Slowed(before, self.median);
        }
        let verdict = if over {
            Verdict::Hitch(self.median)
        } else {
            Verdict::Normal
        };
        if self.ring.len() < WINDOW {
            self.ring.push(ms);
        } else {
            self.ring[self.at] = ms;
            self.at = (self.at + 1) % WINDOW;
        }
        self.since += 1;
        if self.since >= MEDIAN_EVERY || self.ring.len() <= WARM {
            self.since = 0;
            self.median = median(&self.ring);
        }
        verdict
    }

    /// The typical frame now (ms; 0 before any frame).
    pub fn typical(&self) -> f64 {
        self.median
    }
}

/// The shortest hitch where the typical frame is `median` ms.
fn floor(median: f64) -> f64 {
    if median < FAST_MS {
        FAST_FLOOR_MS
    } else {
        FLOOR_MS
    }
}

/// The median of `values` (the upper of the two middle ones for an even count; 0 for none).
fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    let mid = v.len() / 2;
    let (_, m, _) = v.select_nth_unstable_by(mid, f64::total_cmp);
    *m
}

/// At most [`BURST`] lines in any [`BURST_SECS`], a big frame ([`BIG_MS`] or more) always: the times of the last
/// lines written, the lines left out since the last written one, and how many were left out in the session.
#[derive(Default)]
pub struct Cap {
    written: VecDeque<f64>,
    left_out: u64,
    /// The longest frame left out since the last line written (ms).
    longest: f64,
    pub left_out_in_all: u64,
}

impl Cap {
    /// Whether a line for a frame of `ms` at `secs` (session seconds) is written: `Some((n, longest))` with the lines
    /// left out before it and the longest of them (ms). A big frame is always written.
    pub fn admit(&mut self, secs: f64, ms: f64) -> Option<(u64, f64)> {
        while self
            .written
            .front()
            .is_some_and(|&t| secs - t >= BURST_SECS)
        {
            self.written.pop_front();
        }
        if self.written.len() >= BURST && ms < BIG_MS {
            self.left_out += 1;
            self.left_out_in_all += 1;
            self.longest = self.longest.max(ms);
            return None;
        }
        self.written.push_back(secs);
        Some((
            std::mem::take(&mut self.left_out),
            std::mem::take(&mut self.longest),
        ))
    }
}

/// One frame's measures, what the line is made from.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub ms: f64,
    pub typical: f64,
    pub frame: u64,
    pub secs: f64,
    pub after_load: bool,
    pub menu: bool,
    pub place: String,
    /// The stages (ms): update, before the draw, the draw before the world, the world draw, after it, Present (the
    /// swap's tail), after Present (main's waits for its threads included), the game's tail after `WaitForThreads`,
    /// the host's time between the last frame and this one.
    pub stages: Option<[f64; 9]>,
    pub game: Vec<u64>,
    pub stream: Vec<u64>,
    pub longest_wait: (u64, WaitKind, u32),
    pub largest_read: (u64, String),
    /// Who held the kernel lock in main's longest wait for it of 1 ms or more (`engine_core::hitch::LockHolder`).
    pub lock_holder: Option<engine_core::hitch::LockHolder>,
    /// Who owned the game critical section in main's longest wait for one of 1 ms or more.
    pub section_holder: Option<engine_core::hitch::LockHolder>,
    /// The interface's hibernating tiles at the frame's end (`Tile::iHibernatingTileCount`, `011f32d8`).
    pub hibernating: u32,
    /// The longest completed load task main ran (ns, its vtable).
    pub longest_task: (u64, u32),
    /// The render thread's and the queue thread's longest blocking call of the frame, with its time (ns)
    /// (`engine_platform::stream_hitch::take_longest`).
    pub longest_calls: [Option<(engine_platform::stream_hitch::Call, u64)>; 2],
    /// The game's frame tail split at its marks (`engine_core::stage_time::take_tail`): printed when the tail is
    /// [`TAIL_SPLIT_MS`] or more.
    pub tail: Option<engine_core::stage_time::Tail>,
    /// The host's time between the last frame and this one split at its marks (`engine_core::stage_time::take_host_gap`):
    /// printed when that time is [`TAIL_SPLIT_MS`] or more.
    pub host_gap: Option<engine_core::stage_time::HostGap>,
    pub left_out: u64,
    /// The longest of those left out (ms).
    pub longest_left_out: f64,
}

const STAGE_NAMES: [&str; 10] = [
    "update",
    "before draw",
    "culling and set-up",
    "world draw",
    "after world",
    "present",
    "after present",
    "frame tail",
    "host between frames",
    "unmeasured",
];

/// The index of the frame tail in [`STAGE_NAMES`].
const TAIL_STAGE: usize = 7;
/// The index of the host's time between frames in [`STAGE_NAMES`].
const HOST_STAGE: usize = 8;
/// A frame tail at least this long (ms) is split into its parts in the line.
pub const TAIL_SPLIT_MS: f64 = 2.0;

/// The frame tail's parts in words (`engine_core::stage_time::Tail`, the order main runs them): each part's time;
/// for the two task queues' drains, what each ran; for the free-list sort, the pool and its
/// free list's count.
fn tail_words(t: &engine_core::stage_time::Tail) -> String {
    use engine_core::stage_time::{DrainQueue, TailMark, TAIL_PART_NAMES};
    let mut parts = Vec::new();
    for (i, p) in t.parts.iter().enumerate() {
        let Some(ns) = p else { continue };
        let extra = if i == TailMark::AiQueue as usize {
            t.drains[DrainQueue::First as usize].map(|d| {
                let sync = t.havok_sync.map_or_else(String::new, |h| {
                    format!(
                        "; tag 0x04 is the physics results applied: {} actors' Havok sync {:.1} ms, {} nodes \
                         updated {:.1} ms, the cells' update {:.1} ms",
                        h.actors,
                        ms(h.actors_ns),
                        h.nodes,
                        ms(h.nodes_ns),
                        ms(h.cells_ns)
                    )
                });
                format!(" ({}{sync})", drain_words(&d))
            })
        } else if i == TailMark::QueueDrain as usize {
            t.drains[DrainQueue::Secondary as usize].map(|d| format!(" ({})", drain_words(&d)))
        } else if i == TailMark::TickOut as usize {
            t.tick.map(|(pool, free)| match free {
                Some(n) => format!(" (pool {pool}, {} in its free list)", thousands(n.into())),
                None => format!(" (pool {pool}: none in the table)"),
            })
        } else {
            None
        };
        parts.push(format!(
            "{} {:.1}{}",
            TAIL_PART_NAMES[i],
            ms(*ns),
            extra.unwrap_or_default()
        ));
    }
    let mut out = parts.join(", ");
    // Main's own processor clock over its part of the tail: little of it means main was pushed off the processor
    // (other threads or programs), not working.
    if let Some(share) = t.main_share {
        let span: u64 = t.parts[..=TailMark::IdleEnd as usize]
            .iter()
            .flatten()
            .sum();
        out += &format!(
            "; main was on the processor for {:.1} of the {:.1} ms to OnIdle's end",
            ms(span) * f64::from(share) / 1000.0,
            ms(span)
        );
    }
    out
}

/// A window message of the game's by its id: the ones the platform layer posts (`engine_platform::window`) by name.
fn message_name(id: u32) -> String {
    match id {
        0x0006 => "WM_ACTIVATE".into(),
        0x0012 => "WM_QUIT".into(),
        0x0200 => "WM_MOUSEMOVE".into(),
        _ => format!("message {id:#06x}"),
    }
}

/// The host's time between two of main's frames in words (`engine_core::stage_time::HostGap`, the order its parts
/// come): each part's time; for the last, the game's window messages main ran in it and the longest of them.
fn host_gap_words(g: &engine_core::stage_time::HostGap) -> String {
    use engine_core::stage_time::{HOST_MARKS, HOST_PART_NAMES};
    let mut parts = Vec::new();
    for (i, p) in g.parts.iter().enumerate() {
        let Some(ns) = p else { continue };
        let extra = if i == HOST_MARKS && g.messages > 0 {
            let longest = g.longest_message.map_or_else(String::new, |(id, ns)| {
                format!(", the longest {} {:.1} ms", message_name(id), ms(ns))
            });
            format!(
                " ({} messages run by the game's window procedure in {:.1} ms{longest})",
                g.messages,
                ms(g.messages_ns)
            )
        } else {
            String::new()
        };
        parts.push(format!("{} {:.1}{extra}", HOST_PART_NAMES[i], ms(*ns)));
    }
    parts.join(", ")
}

/// One frame's drains of a packed task queue in words: the tasks, the three kinds (the record's tag) that took
/// longest, the time outside the tasks (the pops, and the turns the loop took on an empty queue while another
/// thread's input to it was still open: main spinning).
fn drain_words(d: &engine_core::stage_time::DrainStats) -> String {
    let mut kinds: Vec<(String, u32, u64)> = d
        .kinds
        .iter()
        .filter(|k| k.1 != 0)
        .map(|k| (format!("tag {:#04x}", k.0), k.1, k.2))
        .collect();
    if d.other.0 > 0 {
        kinds.push(("other kinds".into(), d.other.0, d.other.1));
    }
    kinds.sort_by_key(|k| std::cmp::Reverse(k.2));
    let shown: Vec<String> = kinds
        .iter()
        .take(3)
        .map(|(name, n, ns)| format!("{name} x{n} {:.1} ms", ms(*ns)))
        .collect();
    let mut s = format!("{} tasks", d.tasks);
    if !shown.is_empty() {
        s += &format!(": {}", shown.join(", "));
        if kinds.len() > 3 {
            s += &format!(" and {} more kinds", kinds.len() - 3);
        }
    }
    s += &format!(
        "; {:.1} ms outside the tasks",
        ms(d.ns.saturating_sub(d.task_ns()))
    );
    if d.waits > 0 {
        s += &format!(
            ", {} turns on an empty queue waiting for another thread's input",
            thousands(d.waits.into())
        );
    }
    s
}

/// The stages (ms) from the stage marks' take (µs per column).
pub fn stages_of(v: &[(f64, u64)]) -> [f64; 9] {
    let us = |c: Column| v.get(c as usize).map_or(0.0, |x| x.0);
    [
        us(Column::U),
        us(Column::S1),
        us(Column::RBeforeCut),
        us(Column::CutToDrawEnd),
        us(Column::PassTail) + us(Column::AfterPass),
        us(Column::S2),
        us(Column::P1) + us(Column::W),
        us(Column::GameTail),
        us(Column::HostGap),
    ]
    .map(|x| x / 1000.0)
}

/// The stage marks of the frame just ended (taken once a frame for the hitch log and the frame log), when the
/// points are timed.
pub fn take_stages() -> Option<[(f64, u64); engine_core::stage_time::COLUMNS]> {
    if !engine_core::stage_time::points_on() {
        return None;
    }
    // The game's frame tail ends and the host's time between frames starts here.
    engine_core::stage_time::point(engine_core::stage_time::Point::Host);
    Some(engine_core::stage_time::take())
}

/// Whether a frame is judged, and whether it is the first after a load: a frame that ends on a loading screen, in
/// which a game was loaded (`loads`; from a menu no loading screen is up), or in which the loading screen's own frames
/// were shown (`presents` above [`LOADING_PRESENTS`]: a `coc`, a door or fast travel loads inside one of main's frames
/// while the loading screen draws) is not judged nor counted in the median; the first frame after one
/// (`was_loading`) is.
pub fn judge(loading_screen: bool, loads: u64, presents: u32, was_loading: bool) -> (bool, bool) {
    let loading = loading_screen || loads > 0 || presents > LOADING_PRESENTS;
    (!loading, was_loading && !loading)
}

/// What one of main's frames showed, as the hitch log tells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown {
    /// The frame ends on the game's own loading screen (`uncap::screen`).
    pub loading_screen: bool,
    /// It ends in a menu (the console, the Pip-Boy, a dialogue, the main menu).
    pub menu: bool,
    /// Games loaded in it.
    pub loads: u64,
    /// Presents submitted in it.
    pub presents: u32,
}

/// The hitch test of a run's frames, play and menus apart (a menu runs at its own frame limit): the pure part of
/// [`HitchLog::record`].
#[derive(Clone, Default)]
pub struct Judge {
    play: Detector,
    menu: Detector,
    was_loading: bool,
    was_menu: bool,
}

/// What [`Judge::frame`] tells of a judged frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Judged {
    pub verdict: Verdict,
    pub after_load: bool,
    pub menu: bool,
}

impl Judge {
    /// A frame of `ms` (none for the run's first) that showed `shown`: none when it is not judged (a loading screen
    /// or a load in it, the run's first frame). A frame is judged against the kind the frame before it showed, so the
    /// frame that opens a menu is judged against play and the frames after it against the menu's.
    pub fn frame(&mut self, ms: Option<f64>, shown: Shown) -> Option<Judged> {
        let (judged, after_load) = judge(
            shown.loading_screen,
            shown.loads,
            shown.presents,
            self.was_loading,
        );
        self.was_loading = !judged;
        let ms = ms.filter(|_| judged)?;
        let was_menu = std::mem::replace(&mut self.was_menu, shown.menu);
        let detector = if was_menu {
            &mut self.menu
        } else {
            &mut self.play
        };
        Some(Judged {
            verdict: detector.frame(ms),
            after_load,
            menu: shown.menu,
        })
    }
}

/// What main waited for, in words.
fn wait_words(k: WaitKind) -> &'static str {
    match k {
        WaitKind::None => "nothing",
        WaitKind::KernelLock => "the kernel lock",
        WaitKind::DeviceLock => "the device lock",
        WaitKind::Section => "a lock another game thread held",
        WaitKind::Object => "another game thread",
        WaitKind::Sleep => "a sleep",
        WaitKind::Present => "Present",
        WaitKind::Reply => "a render-thread reply",
        WaitKind::QueueFull => "room in the render queue",
    }
}

/// The game thread that held the kernel lock, the platform function it held it for, and whether it was working or
/// pushed off the processor meanwhile, in words.
fn holder_words(h: &engine_core::hitch::LockHolder) -> String {
    let who = match (h.thread, h.start) {
        (0, _) => "the host or main".to_string(),
        // OEI's IO thread: every file read and the queued loads go through it.
        (_, 0x00aa_8430) => format!("the IO thread (game thread {})", h.thread),
        (t, 0) => format!("game thread {t}"),
        (t, s) => format!("game thread {t} (starts at {s:08x})"),
    };
    let what = match (h.call, h.call_name.as_str()) {
        // A section's owner notes only its locked platform calls: in none, it may be running game code, in a sync
        // import or a device call, or blocked.
        (0, _) if h.section != 0 => "in no locked platform call".to_string(),
        (0, _) if !h.site.is_empty() => format!("in {}", h.site),
        (0, _) => "outside a platform call".to_string(),
        (_, "") => format!("in the platform call at {:08x}", h.call),
        (_, name) => format!("in {name}"),
    };
    let ran = match h.cpu_ns {
        Some(cpu) if h.measured_ns > 0 => {
            let share = cpu as f64 / h.measured_ns as f64;
            format!(
                "; it ran {:.1} of the last {:.1} ms waited ({})",
                ms(cpu),
                ms(h.measured_ns),
                if share >= 0.5 {
                    "working under the lock"
                } else if h.section != 0 {
                    "blocked or off the processor"
                } else {
                    "mostly off the processor"
                }
            )
        }
        _ => String::new(),
    };
    let section = if h.section != 0 {
        format!("section {:08x} owned by ", h.section)
    } else {
        String::new()
    };
    let render = match h.render_wait {
        1 => "; it was waiting for a reply from the render thread",
        2 => "; it was waiting in Present for the frames in flight",
        3 => "; it was waiting for room in the render queue",
        _ => "",
    };
    let ran = format!("{ran}{render}");
    let hands = if h.handoffs > 0 {
        format!(
            "; not one hold: the lock changed hands {} times while main waited",
            h.handoffs
        )
    } else {
        String::new()
    };
    format!(
        "{section}{who} {what}, main waited {:.1} ms{ran}{hands}",
        ms(h.wait_ns)
    )
}

/// A wait under this (ns) is left out of the line: too short to matter.
const TENTH_MS: u64 = 100_000;

fn ms(ns: u64) -> f64 {
    ns as f64 / 1e6
}

/// A count with thousands separated (22,093).
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn bytes(n: u64) -> String {
    if n >= 1 << 20 {
        format!("{:.1} MB", n as f64 / f64::from(1u32 << 20))
    } else {
        format!("{} KB", n.div_ceil(1024))
    }
}

impl Frame {
    fn g(&self, c: Count) -> u64 {
        self.game.get(c as usize).copied().unwrap_or(0)
    }
    fn s(&self, c: engine_platform::stream_hitch::Count) -> u64 {
        self.stream.get(c as usize).copied().unwrap_or(0)
    }

    /// The stages with what they leave unmeasured (the frame less the stages: the clock's grain, and a frame whose
    /// points were not all passed, such as a load's).
    fn stages_all(&self, st: [f64; 9]) -> [f64; 10] {
        let rest = (self.ms - st.iter().sum::<f64>()).max(0.0);
        std::array::from_fn(|i| if i < 9 { st[i] } else { rest })
    }

    /// Main's wait for the render thread (ns): in Present, for replies, for room in the queue.
    fn render_wait(&self) -> u64 {
        use engine_platform::stream_hitch::Count as S;
        self.s(S::PresentWaitNs) + self.s(S::ReplyWaitNs) + self.s(S::QueueFullNs)
    }

    /// Where the render thread's time went while main waited for it, each part said when it is 1 ms or more, the largest
    /// first: what it waited for (the window's next frame: with the Fifo present mode the display's refresh; the queue
    /// thread's last frame at its present, its present call included; the queue thread within the frame; the GPU; the
    /// drawing side at a cut), what kept it busy (its own work on the game's commands, and its blocking calls by part),
    /// and the longest single call on it and on the queue thread.
    fn render_reason(&self) -> String {
        use engine_platform::stream_hitch::{Count as S, Part};
        let render = |p: Part| self.render_part(p);
        let mut waits = [
            (self.s(S::AcquireNs), "the window's next frame"),
            (self.s(S::RenderSyncNs), "the queue thread's last frame"),
            (render(Part::ThreadWait), "the queue thread"),
            (render(Part::GpuWait), "the GPU"),
            (self.s(S::SlotWaitNs), "the drawing side"),
        ];
        let mut busy = [
            (self.render_own(), "the game's commands"),
            (render(Part::Poll), "polls"),
            (render(Part::Submit), "submits"),
            (render(Part::Upload), "uploads"),
            (render(Part::Create), "resources made"),
            (render(Part::Finish), "encoders finished"),
            (self.s(S::RenderIdleNs), "idle work"),
        ];
        let said = |parts: &mut [(u64, &str)]| -> Vec<String> {
            parts.sort_by_key(|a| std::cmp::Reverse(a.0));
            parts
                .iter()
                .filter(|(ns, _)| *ns >= 1_000_000)
                .map(|(ns, w)| format!("{w} {:.0} ms", ms(*ns)))
                .collect()
        };
        let mut out = Vec::new();
        let waited = said(&mut waits);
        if !waited.is_empty() {
            out.push(format!("it waited for {}", waited.join(", ")));
        }
        let worked = said(&mut busy);
        if !worked.is_empty() {
            out.push(format!("busy with {}", worked.join(", ")));
        }
        let longest = self.longest_calls_words(1_000_000);
        if !longest.is_empty() {
            out.push(format!("the longest call: {longest}"));
        }
        if out.is_empty() {
            String::new()
        } else {
            format!(" ({})", out.join("; "))
        }
    }

    /// The render thread's blocking calls of part `p` (ns).
    fn render_part(&self, p: engine_platform::stream_hitch::Part) -> u64 {
        use engine_platform::stream_hitch::Count as S;
        self.stream
            .get(S::RenderSubmitNs as usize + p as usize)
            .copied()
            .unwrap_or(0)
    }

    /// The queue thread's blocking calls of part `p` (ns).
    fn queue_part(&self, p: engine_platform::stream_hitch::Part) -> u64 {
        use engine_platform::stream_hitch::Count as S;
        self.stream
            .get(S::QueueSubmitNs as usize + p as usize)
            .copied()
            .unwrap_or(0)
    }

    /// The render thread's own work on the game's commands (ns): its time on them less its blocking calls and its
    /// present's waits, which are counted apart.
    fn render_own(&self) -> u64 {
        use engine_platform::stream_hitch::{Count as S, PARTS};
        let calls: u64 = (0..PARTS)
            .map(|i| {
                self.stream
                    .get(S::RenderSubmitNs as usize + i)
                    .copied()
                    .unwrap_or(0)
            })
            .sum();
        self.s(S::RenderBusyNs)
            .saturating_sub(calls + self.s(S::RenderSyncNs) + self.s(S::AcquireNs))
    }

    /// The two threads' longest blocking calls of `at_least` ns or more, in words.
    fn longest_calls_words(&self, at_least: u64) -> String {
        let said: Vec<String> = self
            .longest_calls
            .iter()
            .zip(["on the render thread", "on the queue thread"])
            .filter_map(|(c, on)| {
                c.filter(|&(_, ns)| ns >= at_least)
                    .map(|(call, ns)| format!("{} {on} {:.1} ms", call.name(), ms(ns)))
            })
            .collect();
        said.join(", ")
    }

    /// The render and queue threads' frame in the work list (main waited 1 ms or more for the render thread): the
    /// render thread's time on the game's commands split into its own work and its blocking calls, its idle work and
    /// its waits for the drawing side; the queue thread's time on its jobs split the same way; the longest calls.
    fn render_frame_words(&self) -> Option<String> {
        use engine_platform::stream_hitch::{Count as S, Part, PARTS, PART_NAMES};
        if self.render_wait() < 1_000_000 {
            return None;
        }
        let parts = |of: &dyn Fn(Part) -> u64, first: Option<(u64, &str)>| -> String {
            let all = [
                Part::Submit,
                Part::Upload,
                Part::Create,
                Part::Poll,
                Part::GpuWait,
                Part::ThreadWait,
                Part::Finish,
            ];
            debug_assert_eq!(all.len(), PARTS);
            let said: Vec<String> = first
                .into_iter()
                .chain(all.iter().map(|&p| (of(p), PART_NAMES[p as usize])))
                .filter(|(ns, _)| *ns >= TENTH_MS)
                .map(|(ns, w)| format!("{w} {:.1}", ms(ns)))
                .collect();
            if said.is_empty() {
                String::new()
            } else {
                format!(" ({})", said.join(", "))
            }
        };
        let mut l = format!(
            "the render thread on the game's commands {:.1} ms{}",
            ms(self.s(S::RenderBusyNs)),
            parts(
                &|p| self.render_part(p),
                Some((self.render_own(), "its own work"))
            )
        );
        for (c, w) in [
            (S::RenderIdleNs, "idle work"),
            (S::SlotWaitNs, "waiting for the drawing side"),
        ] {
            if self.s(c) >= TENTH_MS {
                l += &format!(", {w} {:.1} ms", ms(self.s(c)));
            }
        }
        if self.s(S::QueueBusyNs) >= TENTH_MS {
            l += &format!(
                ", the queue thread on its jobs {:.1} ms{}",
                ms(self.s(S::QueueBusyNs)),
                parts(&|p| self.queue_part(p), None)
            );
        }
        let longest = self.longest_calls_words(TENTH_MS);
        if !longest.is_empty() {
            l += &format!(", the longest call {longest}");
        }
        Some(l)
    }

    /// The plain-word guess of the frame's main cause, by simple rules over its counts.
    pub fn cause(&self) -> String {
        use engine_platform::stream_hitch::Count as S;
        let mut said: Vec<(f64, String)> = Vec::new();
        let site = |s: u32| {
            if s == 0 {
                String::new()
            } else {
                format!(" (at {s:08x})")
            }
        };
        let (_, wait_kind, wait_site) = self.longest_wait;
        let object = self.g(Count::ObjectWaitNs);
        if object > 0 {
            let at = if wait_kind == WaitKind::Object {
                site(wait_site)
            } else {
                String::new()
            };
            said.push((
                ms(object),
                format!(
                    "waited {:.0} ms for another game thread (a loader or a task){at}",
                    ms(object)
                ),
            ));
        }
        let section = self.g(Count::SectionWaitNs);
        if section > 0 {
            let at = if wait_kind == WaitKind::Section {
                site(wait_site)
            } else {
                String::new()
            };
            let owner = self
                .section_holder
                .as_ref()
                .map_or_else(String::new, |h| format!(" ({})", holder_words(h)));
            said.push((
                ms(section),
                format!(
                    "waited {:.0} ms for a lock another game thread held{at}{owner}",
                    ms(section)
                ),
            ));
        }
        let locks = self.g(Count::KernelWaitNs) + self.g(Count::DeviceWaitNs);
        if locks > 0 {
            let holder = self
                .lock_holder
                .as_ref()
                .map_or_else(String::new, |h| format!(" ({})", holder_words(h)));
            said.push((
                ms(locks),
                format!(
                    "waited {:.0} ms for the platform's locks{holder}",
                    ms(locks)
                ),
            ));
        }
        // Main's timed work: a stretch that explains the frame is named.
        for (count, words) in [
            (Count::CellAttachNs, "attaching cells"),
            (Count::CellDetachNs, "detaching cells"),
            (Count::LoadFinishNs, "finishing background loads on main"),
            (Count::HavokNsMain, "in the physics update"),
            (Count::ScriptNsMain, "running scripts"),
        ] {
            let ns = self.g(count);
            if ns > 0 {
                said.push((ms(ns), format!("{:.0} ms {words}", ms(ns))));
            }
        }
        let render = self.render_wait();
        if render > 0 {
            said.push((
                ms(render),
                format!(
                    "waited {:.0} ms for the render thread{}",
                    ms(render),
                    self.render_reason()
                ),
            ));
        }
        let made = self.s(S::PipelineNs) + self.s(S::ModuleNs) + self.s(S::ShadowPipelineNs);
        if made > 0 {
            // Made on the render thread or a helper: main feels it through its wait for the render thread.
            let casters = if self.s(S::ShadowPipelines) > 0 {
                format!(
                    ", {} sun-shadow caster pipelines",
                    self.s(S::ShadowPipelines)
                )
            } else {
                String::new()
            };
            said.push((
                ms(made).min(ms(render).max(ms(made) / 2.0)),
                format!(
                    "made {} pipelines{casters} and {} shader modules ({:.0} ms)",
                    self.s(S::Pipelines),
                    self.s(S::Modules),
                    ms(made)
                ),
            ));
        }
        let read = self.g(Count::ReadNsMain);
        if read > 0 {
            let largest = if self.largest_read.0 > 0 {
                format!(
                    ", the largest {} of {}",
                    bytes(self.largest_read.0),
                    self.largest_read.1
                )
            } else {
                String::new()
            };
            said.push((
                ms(read),
                format!(
                    "read {} from disk on the main thread ({:.0} ms{largest})",
                    bytes(self.g(Count::BytesReadMain)),
                    ms(read)
                ),
            ));
        }
        // Main's own sleeps (the frame limiter's, the game's): a frame they lengthen is said so.
        let slept = self.g(Count::SleepNs);
        if slept > 0 {
            said.push((
                ms(slept),
                format!(
                    "main slept {:.0} ms (a frame limit or the game's own wait)",
                    ms(slept)
                ),
            ));
        }
        let hand_back = self.g(Count::HandBackNs);
        if hand_back > 0 {
            said.push((
                ms(hand_back),
                format!(
                    "handed freed memory back with every game thread stopped ({:.0} ms)",
                    ms(hand_back)
                ),
            ));
        }
        // About 0.4 µs a guest heap block on main (measure/smooth-200/HITCHES.md H-d1: 22,093 frees in 9.0 ms).
        // The heap's own time on main when measured, else that estimate.
        let blocks = self.g(Count::FreesMain) + self.g(Count::AllocsMain);
        let heap = self.g(Count::HeapNsMain);
        if blocks >= 2_000 || ms(heap) >= 5.0 {
            let freed = self.g(Count::FreesMain);
            let words = if freed * 2 >= blocks {
                format!("freed {} memory blocks", thousands(freed))
            } else {
                format!(
                    "allocated {} memory blocks",
                    thousands(self.g(Count::AllocsMain))
                )
            };
            if heap > 0 {
                said.push((
                    ms(heap),
                    format!("{words} ({:.0} ms in the heap)", ms(heap)),
                ));
            } else {
                said.push((blocks as f64 * 0.0004, words));
            }
        }
        said.sort_by(|a, b| b.0.total_cmp(&a.0));
        let excess = self.ms - self.typical;
        let mut out: Vec<String> = Vec::new();
        if self.g(Count::Saves) > 0 {
            out.push("saved the game".into());
        }
        if self.g(Count::Loads) > 0 {
            out.push("loaded a game".into());
        }
        if self.after_load {
            out.push("first frame after a load".into());
        }
        let cells = (self.g(Count::CellsAttached), self.g(Count::CellsDetached));
        // What explains a quarter of the frame's excess or 5 ms, the biggest first (two at most).
        let strong: Vec<&(f64, String)> = said
            .iter()
            .filter(|(m, _)| *m >= excess / 4.0 || *m >= 5.0)
            .take(2)
            .collect();
        let explained: f64 = strong.iter().map(|(m, _)| m).sum();
        for (_, w) in &strong {
            out.push(w.clone());
        }
        if cells != (0, 0) {
            out.push(match cells {
                (a, 0) => format!("attached {a} cells"),
                (0, d) => format!("detached {d} cells"),
                (a, d) => format!("attached {a} and detached {d} cells"),
            });
        }
        // Less than half explained (a save says enough): the stage that took the time.
        if self.g(Count::Saves) == 0 && explained < excess / 2.0 {
            if let Some(st) = self.stages {
                let (i, m) = self
                    .stages_all(st)
                    .into_iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .expect("ten stages");
                let none = if strong.is_empty() {
                    " with no one-off work counted"
                } else {
                    ""
                };
                // The frame tail's longest part, when it was split.
                let part = match (i, self.tail.as_ref().and_then(|t| t.longest())) {
                    (TAIL_STAGE, Some((p, ns))) => {
                        // Main mostly off the processor: the time is not the part's work.
                        let off = self
                            .tail
                            .as_ref()
                            .and_then(|t| t.main_share)
                            .filter(|s| *s < 600)
                            .map_or_else(String::new, |s| {
                                format!("; main on the processor for {}% of the tail", s / 10)
                            });
                        format!(
                            " (most in the {}, {:.1} ms{off})",
                            engine_core::stage_time::TAIL_PART_NAMES[p],
                            ms(ns)
                        )
                    }
                    // The host's time between frames: its longest part, and for the game's window messages the
                    // longest message.
                    (HOST_STAGE, _) => match self
                        .host_gap
                        .as_ref()
                        .and_then(|g| g.longest().map(|l| (g, l)))
                    {
                        Some((g, (p, ns))) => {
                            let message = (p == engine_core::stage_time::HOST_MARKS)
                                .then_some(g.longest_message)
                                .flatten()
                                .map_or_else(String::new, |(id, ns)| {
                                    format!(
                                        "; the longest message {} {:.1} ms",
                                        message_name(id),
                                        ms(ns)
                                    )
                                });
                            format!(
                                " (most in {}, {:.1} ms{message})",
                                engine_core::stage_time::HOST_PART_NAMES[p],
                                ms(ns)
                            )
                        }
                        None => String::new(),
                    },
                    _ => String::new(),
                };
                out.push(if i == 9 {
                    format!("{m:.0} ms fell outside the measured stages{none}")
                } else {
                    format!("the {} took {m:.0} ms{part}{none}", STAGE_NAMES[i])
                });
            }
        }
        if out.is_empty() {
            out.push("no one-off work counted".into());
        }
        out.join("; ")
    }

    /// The log line.
    pub fn line(&self) -> String {
        use engine_platform::stream_hitch::Count as S;
        let mut l = format!(
            "hitch: {:.1} ms{} (typical {:.1}) frame {} at {:.1} s{}{}, {}",
            self.ms,
            if self.ms >= BIG_MS { " big" } else { "" },
            self.typical,
            self.frame,
            self.secs,
            if self.menu { " in a menu" } else { "" },
            if self.after_load {
                ", first frame after a load"
            } else {
                ""
            },
            self.place
        );
        if let Some(st) = self.stages {
            let parts: Vec<String> = STAGE_NAMES
                .iter()
                .zip(self.stages_all(st))
                .map(|(n, m)| format!("{n} {m:.1}"))
                .collect();
            l += &format!("; stages ms: {}", parts.join(", "));
            if st[TAIL_STAGE] >= TAIL_SPLIT_MS {
                if let Some(t) = &self.tail {
                    l += &format!("; frame tail parts ms: {}", tail_words(t));
                }
            }
            if st[HOST_STAGE] >= TAIL_SPLIT_MS {
                if let Some(g) = &self.host_gap {
                    l += &format!("; host between frames parts ms: {}", host_gap_words(g));
                }
            }
        }
        let mut work: Vec<String> = Vec::new();
        let mut put = |n: u64, w: String| {
            if n > 0 {
                work.push(w);
            }
        };
        let g = |c| self.g(c);
        put(
            g(Count::FilesOpenedMain),
            format!("{} files opened on main", g(Count::FilesOpenedMain)),
        );
        put(
            g(Count::BytesReadMain),
            format!(
                "{} read on main in {:.1} ms",
                bytes(g(Count::BytesReadMain)),
                ms(g(Count::ReadNsMain))
            ),
        );
        put(
            self.largest_read.0,
            format!(
                "largest read {} of {}",
                bytes(self.largest_read.0),
                self.largest_read.1
            ),
        );
        put(
            g(Count::FilesOpenedOther),
            format!(
                "{} files opened by other threads",
                g(Count::FilesOpenedOther)
            ),
        );
        put(
            g(Count::BytesReadOther),
            format!("{} read by other threads", bytes(g(Count::BytesReadOther))),
        );
        put(
            g(Count::ObjectWaitNs) / TENTH_MS,
            format!(
                "main waited for other threads {:.1} ms ({}x)",
                ms(g(Count::ObjectWaitNs)),
                g(Count::ObjectWaits)
            ),
        );
        put(
            g(Count::SectionWaitNs) / TENTH_MS,
            format!(
                "for locks they held {:.1} ms ({}x)",
                ms(g(Count::SectionWaitNs)),
                g(Count::SectionWaits)
            ),
        );
        put(
            g(Count::KernelWaitNs) / TENTH_MS,
            format!(
                "for the kernel lock {:.1} ms ({}x)",
                ms(g(Count::KernelWaitNs)),
                g(Count::KernelWaits)
            ),
        );
        put(
            g(Count::DeviceWaitNs) / TENTH_MS,
            format!(
                "for the device lock {:.1} ms ({}x)",
                ms(g(Count::DeviceWaitNs)),
                g(Count::DeviceWaits)
            ),
        );
        put(
            g(Count::SleepNs) / TENTH_MS,
            format!(
                "slept {:.1} ms ({}x)",
                ms(g(Count::SleepNs)),
                g(Count::Sleeps)
            ),
        );
        let (wns, wkind, wsite) = self.longest_wait;
        put(
            wns / TENTH_MS,
            format!(
                "longest wait {:.1} ms (for {}{})",
                ms(wns),
                wait_words(wkind),
                if wsite != 0 {
                    format!(" at {wsite:08x}")
                } else {
                    String::new()
                }
            ),
        );
        put(
            self.s(S::PresentWaitNs) / TENTH_MS,
            format!("waited in Present {:.1} ms", ms(self.s(S::PresentWaitNs))),
        );
        put(
            // The present call runs on the queue thread: its time is part of the render thread's wait for that thread.
            (self.s(S::AcquireNs) + self.s(S::RenderSyncNs)) / TENTH_MS,
            format!(
                "the render thread waited {:.1} ms for the window's frame, {:.1} for the queue thread (whose present call took {:.1})",
                ms(self.s(S::AcquireNs)),
                ms(self.s(S::RenderSyncNs)),
                ms(self.s(S::SwapNs))
            ),
        );
        if let Some(w) = self.render_frame_words() {
            put(1, w);
        }
        put(
            self.s(S::ReplyWaitNs) / TENTH_MS,
            format!(
                "for render-thread replies {:.1} ms ({}x)",
                ms(self.s(S::ReplyWaitNs)),
                self.s(S::ReplyWaits)
            ),
        );
        put(
            self.s(S::QueueFullNs) / TENTH_MS,
            format!(
                "for room in the render queue {:.1} ms",
                ms(self.s(S::QueueFullNs))
            ),
        );
        put(
            self.s(S::Pipelines),
            format!(
                "{} pipelines made ({:.1} ms)",
                self.s(S::Pipelines),
                ms(self.s(S::PipelineNs))
            ),
        );
        put(
            self.s(S::Modules),
            format!(
                "{} shader modules made ({:.1} ms)",
                self.s(S::Modules),
                ms(self.s(S::ModuleNs))
            ),
        );
        put(
            self.s(S::ShadowPipelines),
            format!(
                "{} sun-shadow caster pipelines made ({:.1} ms)",
                self.s(S::ShadowPipelines),
                ms(self.s(S::ShadowPipelineNs))
            ),
        );
        put(
            self.s(S::ShadowPipelinesAhead),
            format!(
                "{} sun-shadow caster pipelines made on threads of their own ({:.1} ms)",
                self.s(S::ShadowPipelinesAhead),
                ms(self.s(S::ShadowPipelineAheadNs))
            ),
        );
        put(
            g(Count::AllocsMain) + g(Count::FreesMain),
            format!(
                "main's memory blocks {} allocated, {} freed",
                thousands(g(Count::AllocsMain)),
                thousands(g(Count::FreesMain))
            ),
        );
        put(
            g(Count::HeapNsMain) / TENTH_MS,
            format!("main in the heap {:.1} ms", ms(g(Count::HeapNsMain))),
        );
        put(
            g(Count::AllocsOther) + g(Count::FreesOther),
            format!(
                "other threads' {} allocated, {} freed",
                thousands(g(Count::AllocsOther)),
                thousands(g(Count::FreesOther))
            ),
        );
        put(
            g(Count::Textures),
            format!("{} textures created", g(Count::Textures)),
        );
        put(
            g(Count::Surfaces),
            format!("{} surfaces created", g(Count::Surfaces)),
        );
        put(
            g(Count::Buffers),
            format!("{} buffers created", g(Count::Buffers)),
        );
        put(
            g(Count::Shaders),
            format!("{} shaders created", g(Count::Shaders)),
        );
        put(
            g(Count::TextureBytes),
            format!("{} uploaded to textures", bytes(g(Count::TextureBytes))),
        );
        put(
            g(Count::BufferBytes),
            format!("{} uploaded to buffers", bytes(g(Count::BufferBytes))),
        );
        put(
            g(Count::SoundBuffers),
            format!(
                "{} sound buffers made ({})",
                g(Count::SoundBuffers),
                bytes(g(Count::SoundBytes))
            ),
        );
        put(
            g(Count::SoundsOpened),
            format!("{} sounds opened for decoding", g(Count::SoundsOpened)),
        );
        put(
            g(Count::CellsAttached),
            format!("{} cells attached", g(Count::CellsAttached)),
        );
        put(
            g(Count::CellsDetached),
            format!("{} cells detached", g(Count::CellsDetached)),
        );
        put(
            g(Count::CellsUnloaded),
            format!("{} cells unloaded", g(Count::CellsUnloaded)),
        );
        put(
            g(Count::LoadFinishNs) / TENTH_MS,
            format!(
                "finished {} background loads on main in {:.1} ms (the longest {:.1} ms, type {:08x})",
                g(Count::LoadFinishTasks),
                ms(g(Count::LoadFinishNs)),
                ms(self.longest_task.0),
                self.longest_task.1
            ),
        );
        put(
            g(Count::LoaderDrainNs) / TENTH_MS,
            format!(
                "the model loader's drain {:.1} ms",
                ms(g(Count::LoaderDrainNs))
            ),
        );
        put(
            g(Count::LoaderWalkNs) / TENTH_MS,
            format!(
                "the model loader's walk {:.1} ms",
                ms(g(Count::LoaderWalkNs))
            ),
        );
        put(
            g(Count::GridUpdateNs) / TENTH_MS,
            format!("grid update {:.1} ms", ms(g(Count::GridUpdateNs))),
        );
        // A set of timed parts, in their order, as one entry when any is over a tenth of a millisecond.
        let mut parts = |title: &str, parts: &[(&str, Count)]| {
            let shown: Vec<String> = parts
                .iter()
                .filter(|(_, c)| g(*c) >= TENTH_MS)
                .map(|(name, c)| format!("{name} {:.1}", ms(g(*c))))
                .collect();
            put(
                shown.len() as u64,
                format!("{title} ms: {}", shown.join(", ")),
            );
        };
        parts(
            "grid shift parts",
            &[
                ("cancel", Count::GridCancelNs),
                ("textures", Count::GridTexturesNs),
                ("shift", Count::GridShiftNs),
                ("Havok", Count::GridHavokNs),
                ("load", Count::GridLoadNs),
                ("terrain", Count::GridTerrainNs),
            ],
        );
        // The grid array's load (`TES::GridArrayLoad`) and, inside its first loop, each grid cell's
        // (`TES::LoadGridCell`): entries of their own, since both also run outside a grid shift.
        parts(
            "grid load parts",
            &[
                ("cells", Count::GridLoadCellsNs),
                ("Havok drop", Count::GridLoadHavokNs),
                ("pass", Count::GridLoadPassNs),
                ("references", Count::GridLoadRefsNs),
                ("3D update", Count::GridLoad3dNs),
            ],
        );
        parts(
            "grid cell parts",
            &[
                ("cell find", Count::GridCellFindNs),
                ("land", Count::GridLandNs),
                ("water", Count::GridWaterNs),
            ],
        );
        put(
            g(Count::GridCellsMade),
            format!("{} grid cells made", g(Count::GridCellsMade)),
        );
        put(
            g(Count::HavokBodiesAddNs) / TENTH_MS,
            format!(
                "{} queued Havok bodies added on main in {:.1} ms",
                g(Count::HavokBodiesAdded),
                ms(g(Count::HavokBodiesAddNs))
            ),
        );
        put(
            g(Count::CellAttachNs) / TENTH_MS,
            format!("cell attach {:.1} ms", ms(g(Count::CellAttachNs))),
        );
        put(
            g(Count::CellDetachNs) / TENTH_MS,
            format!("cell detach {:.1} ms", ms(g(Count::CellDetachNs))),
        );
        put(
            g(Count::CellUnloadNs) / TENTH_MS,
            format!("cell unload {:.1} ms", ms(g(Count::CellUnloadNs))),
        );
        put(
            g(Count::HandBackNs) / TENTH_MS,
            format!(
                "memory handed back at the frame's end {:.1} ms",
                ms(g(Count::HandBackNs))
            ),
        );
        put(
            g(Count::Refs3dAttached),
            format!("{} references given their 3D", g(Count::Refs3dAttached)),
        );
        put(
            g(Count::HavokNsMain) / TENTH_MS,
            format!(
                "the physics update {:.1} ms ({} steps)",
                ms(g(Count::HavokNsMain)),
                g(Count::HavokSteps)
            ),
        );
        put(
            g(Count::HavokRefLocks),
            format!(
                "{} Havok reference changes on main took the global lock (waited {:.1} ms)",
                g(Count::HavokRefLocks),
                ms(g(Count::HavokRefWaitNs))
            ),
        );
        put(
            g(Count::HavokLockAllWaitNs) / TENTH_MS,
            format!(
                "main waited {:.1} ms for the Havok world lock",
                ms(g(Count::HavokLockAllWaitNs))
            ),
        );
        put(
            g(Count::ScriptNsMain) / TENTH_MS,
            format!(
                "scripts {:.1} ms ({} runs)",
                ms(g(Count::ScriptNsMain)),
                g(Count::ScriptRuns)
            ),
        );
        put(
            g(Count::TilesMade) + g(Count::TilesDeleted),
            format!(
                "interface tiles {} made, {} destroyed, {} hibernating",
                g(Count::TilesMade),
                g(Count::TilesDeleted),
                self.hibernating
            ),
        );
        if let Some(h) = &self.section_holder {
            put(
                1,
                format!("the critical section's owner: {}", holder_words(h)),
            );
        }
        if let Some(h) = &self.lock_holder {
            put(1, format!("the kernel lock's holder: {}", holder_words(h)));
        }
        put(g(Count::Saves), "a save".to_string());
        put(g(Count::Loads), "a load".to_string());
        put(
            g(Count::ScriptCompiles),
            format!("{} scripts compiled", g(Count::ScriptCompiles)),
        );
        if !work.is_empty() {
            l += &format!("; work: {}", work.join(", "));
        }
        l += &format!("; likely cause: {}", self.cause());
        if self.left_out > 0 {
            l += &format!(
                " ({} hitches before this one not written: too many in 10 s; the longest {:.1} ms)",
                self.left_out, self.longest_left_out
            );
        }
        l
    }
}

/// Where the log's lines leave main: main formats a line, keeps the F12 report's copy of it and hands it over
/// ([`Lines::send`], a channel push that never waits); a thread of the log's own writes it. The write can block for as
/// long as the file, pipe or console behind the stream makes it wait (a pipe whose reader a busy machine starves, a
/// disk flushing other writes): in the lead's route run with mods on 2026-11-23, while the Vulkan driver compiled a
/// cold pipeline cache, main spent 360 ms writing one hitch line, a stutter of the log's own making. Every other input
/// of the line is a counter or a record only main and the counting touch.
pub struct Lines {
    /// The writer thread's channel (none when the thread could not be started: main writes, as before).
    tx: Option<Sender<String>>,
    /// Lines handed over ([`Lines::send`]).
    sent: AtomicU64,
    /// Lines written, and the condition variable [`Lines::flush`] waits on.
    written: Arc<(Mutex<u64>, Condvar)>,
    /// What keeps a line on main before the hand-over (the F12 report's copy), so a report built after
    /// [`Lines::send`] has it whatever the writer thread is doing.
    keep: fn(String),
}

impl Lines {
    /// A writer thread over `sink`: each line written whole ([`write_line`]), in the order sent; `keep` takes each
    /// line on the caller's thread first.
    pub fn new(mut sink: Box<dyn Write + Send>, keep: fn(String)) -> Lines {
        let written = Arc::new((Mutex::new(0u64), Condvar::new()));
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let done = written.clone();
        let thread = std::thread::Builder::new()
            .name("hitch log".into())
            .spawn(move || {
                for line in rx {
                    write_line(&mut *sink, line);
                    count_written(&done);
                }
            });
        Lines {
            tx: thread.is_ok().then_some(tx),
            sent: AtomicU64::new(0),
            written,
            keep,
        }
    }

    /// Keeps `line` (on this thread) and hands it to the writer thread; never waits for its write.
    pub fn send(&self, line: String) {
        (self.keep)(line.clone());
        self.sent.fetch_add(1, SeqCst);
        let line = match &self.tx {
            Some(tx) => match tx.send(line) {
                Ok(()) => return,
                Err(std::sync::mpsc::SendError(line)) => line,
            },
            None => line,
        };
        // No writer thread: written here, as before.
        write_line(&mut std::io::stderr(), line);
        count_written(&self.written);
    }

    /// Waits until every line sent so far is written, at most `limit`; whether they all were.
    pub fn flush(&self, limit: Duration) -> bool {
        let want = self.sent.load(SeqCst);
        let deadline = Instant::now() + limit;
        let (count, wake) = &*self.written;
        let mut written = count.lock().unwrap_or_else(PoisonError::into_inner);
        while *written < want {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            written = wake
                .wait_timeout(written, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

/// One more line written: counted, and [`Lines::flush`] woken.
fn count_written(written: &(Mutex<u64>, Condvar)) {
    let (count, wake) = written;
    *count.lock().unwrap_or_else(PoisonError::into_inner) += 1;
    wake.notify_all();
}

/// A line written with one call, its text and its newline together, so another thread's write to the same stream (the
/// viewer's 5-second summary, a panic's message) cannot land between them.
fn write_line(sink: &mut dyn Write, line: String) {
    let mut line = line;
    line.push('\n');
    let _ = sink.write_all(line.as_bytes());
}

/// The standard error stream as the run's line writer writes it ([`lines`]): each line in one `write_all` under the
/// stream's lock. (Tests can hold its writes: [`tests::hold_point`].)
struct ErrStream;

impl Write for ErrStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.write_all(bytes).map(|()| bytes.len())
    }

    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        #[cfg(test)]
        tests::hold_point();
        std::io::stderr().write_all(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stderr().flush()
    }
}

/// The standard output stream for [`print_line`].
struct OutStream;

impl Write for OutStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.write_all(bytes).map(|()| bytes.len())
    }

    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        let mut out = std::io::stdout().lock();
        out.write_all(bytes)?;
        out.flush()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

static LINES: OnceLock<Lines> = OnceLock::new();
static OUT_LINES: OnceLock<Lines> = OnceLock::new();

/// How long a panic on the host's main thread waits for the lines on their way ([`lines`]' panic hook).
const PANIC_FLUSH: Duration = Duration::from_millis(250);

/// The run's line writer, started at the first line. Its lines are written out before the process ends: every end
/// goes through the platform layer's `end_process`, which runs [`flush_lines`] first (`engine_platform::on_end`); and
/// a panic on the host's main thread (the one that started it) waits a moment for them before its own message (a game
/// thread's panic is the game's, caught and kept quiet on purpose: nothing is waited for there).
fn lines() -> &'static Lines {
    LINES.get_or_init(|| {
        engine_platform::on_end(flush_lines);
        let host = std::thread::current().id();
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if std::thread::current().id() == host {
                for lines in [&LINES, &OUT_LINES] {
                    if let Some(lines) = lines.get() {
                        lines.flush(PANIC_FLUSH);
                    }
                }
            }
            before(info)
        }));
        Lines::new(
            Box::new(ErrStream),
            engine_platform::d3d9real::note_report_hitch,
        )
    })
}

/// A line for the standard output, written off main as the hitch lines are (the viewer's 5-second summary).
pub fn print_line(line: String) {
    // Started with the hitch lines' writer, whose end flush and panic hook cover both.
    let _ = lines();
    OUT_LINES
        .get_or_init(|| Lines::new(Box::new(OutStream), |_| {}))
        .send(line);
}

/// Waits (at most a second each) until every line handed over is written: before the process ends.
pub fn flush_lines() {
    for lines in [&LINES, &OUT_LINES] {
        if let Some(lines) = lines.get() {
            lines.flush(Duration::from_secs(1));
        }
    }
}

/// The hitch log of a run.
pub struct HitchLog {
    judge: Judge,
    cap: Cap,
    /// A measured run (`NV_FRAME_LOG`): every hitch is written, none left out.
    uncapped: bool,
    /// The presents submitted so far.
    presents: u32,
    last: Option<Instant>,
    start: Instant,
    frame: u64,
    game: [u64; COUNTS],
    stream: [u64; engine_platform::stream_hitch::COUNTS],
    /// Hitches this session: all, and the big ones.
    pub hitches: u64,
    pub big: u64,
}

impl HitchLog {
    /// The log, unless `NV_HITCH_LOG=0` (which also stops the counting and the stage split).
    pub fn from_env() -> Option<HitchLog> {
        let off = std::env::var("NV_HITCH_LOG").is_ok_and(|v| v.trim() == "0");
        engine_core::hitch::set_on(!off);
        engine_core::stage_time::set_points(!off);
        // The render and queue threads' blocking calls are timed with the log on.
        engine_platform::stream_hitch::set_on(!off);
        if off {
            engine_platform::d3d9real::set_report_hitch_totals(None);
            return None;
        }
        engine_platform::d3d9real::set_report_hitch_totals(Some((0, 0, 0)));
        Some(HitchLog {
            judge: Judge::default(),
            cap: Cap::default(),
            uncapped: std::env::var_os("NV_FRAME_LOG").is_some(),
            presents: 0,
            last: None,
            start: Instant::now(),
            frame: 0,
            game: engine_core::hitch::snapshot(),
            stream: engine_platform::stream_hitch::snapshot(),
            hitches: 0,
            big: 0,
        })
    }

    /// After one of main's frames: `stages` the stage marks' take of this frame (none when nothing took them). Reads
    /// counters and guest memory only: no guest code runs here, so a recorded or repeatable run is the same with the
    /// log on or off.
    pub fn record(&mut self, e: &mut Engine, stages: Option<&[(f64, u64)]>) {
        self.record_frame(e, stages);
        // The record's own time (its line built and written) is the first part of the host's time to the next frame.
        engine_core::stage_time::host_mark(engine_core::stage_time::HostMark::HitchLog);
    }

    /// [`HitchLog::record`]'s work.
    fn record_frame(&mut self, e: &mut Engine, stages: Option<&[(f64, u64)]>) {
        let now = Instant::now();
        let frame_ms = self.last.map(|t| now.duration_since(t).as_secs_f64() * 1e3);
        self.last = Some(now);
        let frame = self.frame;
        self.frame += 1;
        let game_now = engine_core::hitch::snapshot();
        let stream_now = engine_platform::stream_hitch::snapshot();
        let game = engine_core::hitch::since(&game_now, &self.game);
        let stream: Vec<u64> = stream_now
            .iter()
            .zip(&self.stream)
            .map(|(a, b)| a.wrapping_sub(*b))
            .collect();
        self.game = game_now;
        self.stream = stream_now;
        let longest_wait = engine_core::hitch::take_longest_wait();
        let largest_read = engine_core::hitch::take_largest_read();
        let lock_holder = engine_core::hitch::take_lock_holder(e.kernel.id());
        let section_holder = engine_core::hitch::take_section_holder(e.kernel.id());
        let hibernating = if e.mem.is_mapped(HIBERNATING_TILE_COUNT) {
            e.mem.u32(HIBERNATING_TILE_COUNT)
        } else {
            0
        };
        let longest_task = engine_core::hitch::take_longest_task();
        let longest_calls = engine_platform::stream_hitch::take_longest();
        // Taken every frame (its drains are the frame's), after `take_stages` passed the host's mark.
        let tail = engine_core::stage_time::take_tail();
        // The host's time that ended at this frame's `OnIdle`, taken every frame too.
        let host_gap = engine_core::stage_time::take_host_gap();
        let screen = crate::uncap::screen(e);
        let presents = engine_platform::d3d9real::present_count(e).unwrap_or(0);
        let shown = Shown {
            loading_screen: screen == crate::uncap::Screen::Loading,
            menu: screen == crate::uncap::Screen::Menu,
            loads: game[Count::Loads as usize],
            presents: presents.wrapping_sub(self.presents),
        };
        self.presents = presents;
        let Some(judged) = self.judge.frame(frame_ms, shown) else {
            return;
        };
        let ms = frame_ms.unwrap_or(0.0);
        let secs = now.duration_since(self.start).as_secs_f64();
        let typical = match judged.verdict {
            Verdict::Normal => return,
            Verdict::Slowed(from, to) => {
                let line = format!(
                    "hitch log: frames slowed at frame {frame} ({secs:.1} s): the typical frame went from {from:.1} to \
                     {to:.1} ms (a new scene, the window out of focus or a slow stretch), measured again from here"
                );
                lines().send(line);
                return;
            }
            Verdict::Hitch(typical) => typical,
        };
        self.hitches += 1;
        if ms >= BIG_MS {
            self.big += 1;
        }
        let admitted = if self.uncapped {
            Some((0, 0.0))
        } else {
            self.cap.admit(secs, ms)
        };
        engine_platform::d3d9real::set_report_hitch_totals(Some((
            self.hitches,
            self.big,
            self.cap.left_out_in_all,
        )));
        let Some((left_out, longest_left_out)) = admitted else {
            return;
        };
        let f = Frame {
            ms,
            typical,
            frame,
            secs,
            after_load: judged.after_load,
            menu: judged.menu,
            place: place(e),
            stages: stages.map(stages_of),
            game: game.to_vec(),
            stream,
            longest_wait,
            largest_read,
            lock_holder,
            section_holder,
            hibernating,
            longest_task,
            longest_calls,
            tail,
            host_gap,
            left_out,
            longest_left_out,
        };
        lines().send(f.line());
    }
}

/// Where the player is, read from guest memory only (no guest code runs: the names are the forms' full names, as
/// `TESFullName` keeps them): an interior's name and form id, or outside the worldspace's and the grid cell.
fn place(e: &Engine) -> String {
    let player = e.mem.u32(PLAYER);
    if player == 0 {
        return "no player yet".into();
    }
    let cell = e.mem.u32(player + PARENT_CELL);
    let position = [0, 4, 8].map(|o| e.mem.f32(player + POSITION + o));
    crate::report_place::cell_place(e, cell, position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Verdict::{Hitch, Normal};

    fn warm(d: &mut Detector, ms: f64, n: usize) {
        for _ in 0..n {
            assert_eq!(d.frame(ms), Normal);
        }
    }

    /// A frame of twice the typical one and at least 8 ms (6 ms under a 4 ms typical frame) is a hitch; one just
    /// under either is not; the first frames (no typical frame yet) are never judged.
    #[test]
    fn a_hitch_is_twice_the_typical_frame_and_at_least_8_ms() {
        let mut d = Detector::default();
        assert_eq!(d.frame(100.0), Normal, "nothing judged before the warm-up");
        let mut d = Detector::default();
        warm(&mut d, 5.0, 40);
        assert_eq!(d.typical(), 5.0);
        assert_eq!(d.frame(9.9), Normal, "under twice the typical");
        assert_eq!(d.frame(10.0), Hitch(5.0));
        let mut d = Detector::default();
        warm(&mut d, 3.4, 40);
        assert_eq!(d.frame(6.7), Normal, "under twice the typical");
        assert_eq!(d.frame(6.8), Hitch(3.4));
        let mut d = Detector::default();
        warm(&mut d, 2.5, 40);
        assert_eq!(d.frame(5.9), Normal, "under the fast floor");
        assert_eq!(d.frame(6.0), Hitch(2.5));
        let mut d = Detector::default();
        warm(&mut d, 3.9, 40);
        assert_eq!(
            d.frame(7.9),
            Hitch(3.9),
            "a typical frame under 4 ms: the 6 ms floor"
        );
        let mut d = Detector::default();
        warm(&mut d, 4.0, 40);
        assert_eq!(d.frame(7.9), Normal, "4 ms and over: the 8 ms floor");
        assert_eq!(d.frame(8.0), Hitch(4.0));
        let mut d = Detector::default();
        warm(&mut d, 12.0, 40);
        assert_eq!(d.frame(23.9), Normal);
        assert_eq!(d.frame(24.0), Hitch(12.0));
    }

    /// The typical frame follows the last 128 frames (the median, so single long frames do not move it), and is
    /// worked out again every 16 frames.
    #[test]
    fn the_typical_frame_is_the_median_of_the_last_128() {
        let mut d = Detector::default();
        warm(&mut d, 5.0, 128);
        for _ in 0..10 {
            assert_eq!(
                d.frame(60.0),
                Hitch(5.0),
                "single long frames, each a hitch"
            );
            warm(&mut d, 5.0, 9);
        }
        assert_eq!(d.typical(), 5.0, "they do not move it");
        for _ in 0..128 {
            d.frame(9.0);
        }
        assert_eq!(d.typical(), 9.0, "the window moved on");
        assert_eq!(d.frame(17.9), Normal);
        assert_eq!(d.frame(18.0), Hitch(9.0));
        assert_eq!(median(&[3.0, 1.0, 2.0, 10.0]), 3.0);
        assert_eq!(median(&[]), 0.0);
    }

    /// A lasting change of rate (an 8 ms interior, then a 20 ms exterior; an alt-tab) is a few hitches and then a
    /// new typical frame, not a hitch every frame until the window moves on.
    #[test]
    fn a_lasting_change_of_rate_is_a_few_lines_then_a_new_typical_frame() {
        let mut d = Detector::default();
        warm(&mut d, 5.0, 200);
        let verdicts: Vec<Verdict> = (0..200).map(|_| d.frame(20.0)).collect();
        let hitches = verdicts.iter().filter(|v| matches!(v, Hitch(_))).count();
        assert_eq!(hitches, LASTING - 1, "{verdicts:?}");
        assert_eq!(verdicts[LASTING - 1], Verdict::Slowed(5.0, 20.0));
        assert!(verdicts[LASTING..].iter().all(|v| *v == Normal));
        assert_eq!(d.typical(), 20.0);
        // Judged again on the new rate: a jump from 20 ms is a hitch.
        assert_eq!(d.frame(45.0), Hitch(20.0));
        // Back to 5 ms: faster frames are never hitches.
        let verdicts: Vec<Verdict> = (0..200).map(|_| d.frame(5.0)).collect();
        assert!(verdicts.iter().all(|v| *v == Normal));
        assert_eq!(d.typical(), 5.0);
        // A stutter now and then (one in eight frames) stays a hitch each time.
        for _ in 0..20 {
            assert_eq!(d.frame(30.0), Hitch(5.0));
            warm(&mut d, 5.0, 7);
        }
    }

    /// At most 20 lines in any 10 s; the first line after a burst says how many were left out and the longest of
    /// them; a big frame (16 ms or more) is always written, even after a full window.
    #[test]
    fn the_cap_writes_20_lines_in_10_s_and_every_big_frame() {
        let mut c = Cap::default();
        for i in 0..20 {
            assert_eq!(c.admit(i as f64 * 0.1, 9.0), Some((0, 0.0)));
        }
        assert_eq!(c.admit(2.0, 9.5), None);
        assert_eq!(c.admit(5.0, 12.0), None);
        // A 300 ms freeze after the window is full: written, with the two left out before it.
        assert_eq!(c.admit(5.5, 300.0), Some((2, 12.0)));
        assert_eq!(c.admit(6.0, 15.9), None);
        assert_eq!(c.admit(6.1, 16.0), Some((1, 15.9)));
        // 10 s after the first lines they leave the window: small ones fit again.
        assert_eq!(c.admit(11.0, 9.0), Some((0, 0.0)));
        assert_eq!(c.left_out_in_all, 3);
    }

    fn shown(menu: bool) -> Shown {
        Shown {
            loading_screen: false,
            menu,
            loads: 0,
            presents: 1,
        }
    }

    /// The menu/play split and the loads, as `HitchLog::record` drives them.
    #[test]
    fn play_menus_and_loads_are_judged_apart() {
        let mut j = Judge::default();
        assert_eq!(j.frame(None, shown(false)), None, "the run's first frame");
        let verdict = |r: Option<Judged>| r.map(|r| r.verdict);
        for _ in 0..100 {
            assert_eq!(verdict(j.frame(Some(5.0), shown(false))), Some(Normal));
        }
        // The frame that opens a menu is judged against play.
        assert_eq!(verdict(j.frame(Some(30.0), shown(true))), Some(Hitch(5.0)));
        // The menu at its own limit (120 fps), then a 12 ms and a 30 ms menu frame: judged against the menu.
        for _ in 0..100 {
            assert_eq!(verdict(j.frame(Some(8.3), shown(true))), Some(Normal));
        }
        assert_eq!(verdict(j.frame(Some(12.0), shown(true))), Some(Normal));
        assert_eq!(verdict(j.frame(Some(30.0), shown(true))), Some(Hitch(8.3)));
        // Back to play: the frame that closes the menu is the menu's, the next ones play's.
        assert_eq!(verdict(j.frame(Some(5.0), shown(false))), Some(Normal));
        assert_eq!(verdict(j.frame(Some(10.0), shown(false))), Some(Hitch(5.0)));
        // A load (the loading screen's frames inside a frame, a loading screen, a game loaded): not judged, and
        // neither typical frame moves; the frame after it is judged and marked.
        let loading = Shown {
            presents: 14,
            ..shown(false)
        };
        assert_eq!(j.frame(Some(1500.0), loading), None);
        let screen = Shown {
            loading_screen: true,
            ..shown(false)
        };
        assert_eq!(j.frame(Some(900.0), screen), None);
        let after = j.frame(Some(30.0), shown(false)).expect("judged");
        assert_eq!((after.verdict, after.after_load), (Hitch(5.0), true));
        let next = j.frame(Some(5.0), shown(false)).expect("judged");
        assert!(!next.after_load);
        let load = Shown {
            loads: 1,
            ..shown(true)
        };
        assert_eq!(
            j.frame(Some(2000.0), load),
            None,
            "a game loaded from a menu"
        );
        assert!(j.frame(Some(5.0), shown(false)).expect("judged").after_load);
    }

    /// A loading screen or a load in the frame is not judged; the next frame is, as the first after a load.
    #[test]
    fn loads_are_not_judged_and_the_frame_after_one_is() {
        assert_eq!(judge(false, 0, 1, false), (true, false));
        assert_eq!(judge(true, 0, 1, false), (false, false));
        assert_eq!(
            judge(false, 1, 1, false),
            (false, false),
            "a load from a menu"
        );
        assert_eq!(
            judge(false, 0, 2, false),
            (true, false),
            "two presents: not yet a loading screen"
        );
        assert_eq!(
            judge(false, 0, 14, false),
            (false, false),
            "a coc: the loading screen's frames"
        );
        assert_eq!(judge(false, 0, 1, true), (true, true));
        assert_eq!(judge(true, 0, 1, true), (false, false));
    }

    fn frame() -> Frame {
        Frame {
            ms: 64.0,
            typical: 11.9,
            frame: 15431,
            secs: 312.4,
            place: "GoodspringsWorld? WastelandNV (0x000da726) grid (-19, 1)".into(),
            stages: Some([40.1, 0.2, 3.1, 6.0, 1.2, 2.3, 11.0, 0.0, 0.0]),
            game: vec![0; COUNTS],
            stream: vec![0; engine_platform::stream_hitch::COUNTS],
            ..Frame::default()
        }
    }

    /// The background-load and cell timers are written in the work list with the longest task's type.
    #[test]
    fn the_load_finish_and_cell_timers_are_written() {
        let mut f = frame();
        f.game[Count::LoadFinishNs as usize] = 9_400_000;
        f.game[Count::LoadFinishTasks as usize] = 3;
        f.longest_task = (8_100_000, 0x0101_6a4c);
        f.game[Count::GridUpdateNs as usize] = 50_000;
        f.game[Count::CellAttachNs as usize] = 6_200_000;
        let line = f.line();
        assert!(
            line.contains(
                "finished 3 background loads on main in 9.4 ms (the longest 8.1 ms, type 01016a4c)"
            ),
            "{line}"
        );
        assert!(line.contains("cell attach 6.2 ms"), "{line}");
        assert!(!line.contains("grid shift parts"), "none counted: {line}");
        f.game[Count::GridShiftNs as usize] = 3_800_000;
        f.game[Count::GridLoadNs as usize] = 5_100_000;
        f.game[Count::GridTerrainNs as usize] = 20_000;
        let line = f.line();
        assert!(
            line.contains("grid shift parts ms: shift 3.8, load 5.1"),
            "{line}"
        );
        assert!(!line.contains("grid cell parts"), "none counted: {line}");
        f.game[Count::GridCellFindNs as usize] = 4_200_000;
        f.game[Count::GridWaterNs as usize] = 300_000;
        f.game[Count::GridCellsMade as usize] = 2;
        let line = f.line();
        // The per-cell parts are their own entry: LoadGridCell also runs outside a grid shift.
        assert!(
            line.contains(
                "grid shift parts ms: shift 3.8, load 5.1, grid cell parts ms: cell find 4.2, water 0.3, "
            ),
            "{line}"
        );
        assert!(line.contains("2 grid cells made"), "{line}");
        // The grid array load's parts come between the shift's and the cells', in the load's order.
        assert!(!line.contains("grid load parts"), "none counted: {line}");
        f.game[Count::GridLoadCellsNs as usize] = 600_000;
        f.game[Count::GridLoadHavokNs as usize] = 1_700_000;
        f.game[Count::GridLoadPassNs as usize] = 90_000;
        f.game[Count::GridLoadRefsNs as usize] = 2_500_000;
        f.game[Count::GridLoad3dNs as usize] = 400_000;
        let line = f.line();
        assert!(
            line.contains(
                "load 5.1, grid load parts ms: cells 0.6, Havok drop 1.7, references 2.5, 3D update 0.4, \
                 grid cell parts ms: cell find 4.2"
            ),
            "{line}"
        );
        // The queued Havok bodies added on main: with their count, from a tenth of a millisecond.
        f.game[Count::HavokBodiesAdded as usize] = 100;
        f.game[Count::HavokBodiesAddNs as usize] = 90_000;
        assert!(!f.line().contains("Havok bodies"), "{}", f.line());
        f.game[Count::HavokBodiesAddNs as usize] = 1_300_000;
        let line = f.line();
        assert!(
            line.contains("2 grid cells made, 100 queued Havok bodies added on main in 1.3 ms, "),
            "{line}"
        );
        assert!(
            !line.contains("grid update"),
            "under a tenth of a ms: {line}"
        );
        assert!(!line.contains("cell detach"), "{line}");
    }

    /// The cause rules: the counted work that explains the most of the frame's excess is named, with the save, a
    /// load's first frame and cells attached always said; with nothing counted, the stage that took the time.
    #[test]
    fn the_cause_names_what_explains_the_frame() {
        let mut f = frame();
        f.game[Count::FreesMain as usize] = 22_093;
        f.game[Count::AllocsMain as usize] = 7_177;
        f.after_load = true;
        assert_eq!(
            f.cause(),
            "first frame after a load; freed 22,093 memory blocks; the update took 40 ms"
        );

        let mut f = frame();
        f.game[Count::ObjectWaitNs as usize] = 33_000_000;
        f.game[Count::ObjectWaits as usize] = 1;
        f.longest_wait = (33_000_000, WaitKind::Object, 0x00c3_dfa0);
        f.game[Count::CellsAttached as usize] = 3;
        assert_eq!(
            f.cause(),
            "waited 33 ms for another game thread (a loader or a task) (at 00c3dfa0); attached 3 cells"
        );

        let mut f = frame();
        f.game[Count::Saves as usize] = 1;
        assert_eq!(f.cause(), "saved the game");

        let mut f = frame();
        f.stream[engine_platform::stream_hitch::Count::Pipelines as usize] = 3;
        f.stream[engine_platform::stream_hitch::Count::PipelineNs as usize] = 18_000_000;
        f.stream[engine_platform::stream_hitch::Count::PresentWaitNs as usize] = 20_000_000;
        assert_eq!(
            f.cause(),
            "waited 20 ms for the render thread; made 3 pipelines and 0 shader modules (18 ms)"
        );

        // A small wait that explains little: the stage instead.
        let mut f = frame();
        f.game[Count::SectionWaitNs as usize] = 1_000_000;
        assert_eq!(
            f.cause(),
            "the update took 40 ms with no one-off work counted"
        );
        f.stages = None;
        assert_eq!(f.cause(), "no one-off work counted");
        // The time outside the stages (the host between two of main's frames) named when it is the most.
        let mut f = frame();
        f.ms = 30.0;
        f.typical = 10.0;
        f.stages = Some([2.0, 0.0, 1.0, 1.0, 0.5, 0.2, 0.3, 0.0, 0.0]);
        assert_eq!(
            f.cause(),
            "25 ms fell outside the measured stages with no one-off work counted"
        );
        // The host's own time between two frames.
        f.stages = Some([2.0, 0.0, 1.0, 1.0, 0.5, 0.2, 0.3, 0.5, 24.0]);
        assert_eq!(
            f.cause(),
            "the host between frames took 24 ms with no one-off work counted"
        );
        // Split at its marks (the frame log and a report not passed): the longest part named, and for the game's
        // window messages the longest message; the parts in the line.
        let mut parts = [None; engine_core::stage_time::HOST_PARTS];
        parts[0] = Some(300_000);
        parts[3] = Some(100_000);
        parts[4] = Some(200_000);
        parts[5] = Some(10_000);
        parts[6] = Some(23_540_000);
        f.host_gap = Some(engine_core::stage_time::HostGap {
            parts,
            messages: 2,
            messages_ns: 23_500_000,
            longest_message: Some((6, 23_400_000)),
        });
        assert_eq!(
            f.cause(),
            "the host between frames took 24 ms (most in the game's window messages and its loop to OnIdle, 23.5 ms; \
             the longest message WM_ACTIVATE 23.4 ms) with no one-off work counted"
        );
        let line = f.line();
        assert!(
            line.contains(
                "; host between frames parts ms: the hitch log's line 0.3, the host's own steps and the hand-over \
                 0.1, main's thread waking 0.2, main taking the platform lock back 0.0, the game's window messages \
                 and its loop to OnIdle 23.5 (2 messages run by the game's window procedure in 23.5 ms, the longest \
                 WM_ACTIVATE 23.4 ms)"
            ),
            "{line}"
        );
        // Main's thread waking the longest: named without a message.
        parts[4] = Some(23_000_000);
        parts[6] = Some(500_000);
        f.host_gap = Some(engine_core::stage_time::HostGap {
            parts,
            messages: 0,
            messages_ns: 0,
            longest_message: None,
        });
        assert_eq!(
            f.cause(),
            "the host between frames took 24 ms (most in main's thread waking, 23.0 ms) with no one-off work counted"
        );
        assert!(
            f.line()
                .contains("the game's window messages and its loop to OnIdle 0.5"),
            "{}",
            f.line()
        );
        // Under the split's threshold: no parts in the line.
        f.stages = Some([2.0, 0.0, 1.0, 1.0, 0.5, 0.2, 0.3, 0.5, 1.9]);
        assert!(!f.line().contains("host between frames parts"));

        let mut f = frame();
        f.game[Count::ReadNsMain as usize] = 30_000_000;
        f.game[Count::BytesReadMain as usize] = 3 << 20;
        f.largest_read = (2 << 20, "Fallout - Meshes.bsa".into());
        assert_eq!(
            f.cause(),
            "read 3.0 MB from disk on the main thread (30 ms, the largest 2.0 MB of Fallout - Meshes.bsa)"
        );
    }

    /// The new names: the kernel lock's holder (who, in which call, working or pushed off the processor), the heap's
    /// measured time in place of the per-block estimate, and main's timed work (cells, the hand-over, the physics,
    /// scripts) named in the cause when it explains the frame and listed in the work.
    #[test]
    fn the_line_names_the_lock_holder_the_heap_time_and_main_s_timed_work() {
        use engine_core::hitch::LockHolder;
        let holder = LockHolder {
            section: 0,
            thread: 3,
            start: 0x00aa_8430,
            call: 0x00fd_f080,
            call_name: "ReadFile".into(),
            wait_ns: 286_000_000,
            measured_ns: 285_000_000,
            cpu_ns: Some(280_000_000),
            site: String::new(),
            handoffs: 0,
            render_wait: 0,
        };
        let mut f = frame();
        f.stages = Some([2.8, 0.1, 4.7, 5.7, 1.3, 0.3, 405.1, 3.0, 0.0]);
        f.ms = 423.1;
        f.typical = 18.1;
        f.game[Count::KernelWaitNs as usize] = 399_100_000;
        f.game[Count::KernelWaits as usize] = 4;
        f.lock_holder = Some(holder.clone());
        assert_eq!(
            f.cause(),
            "waited 399 ms for the platform's locks (the IO thread (game thread 3) in ReadFile, main waited 286.0 ms; \
             it ran 280.0 of the last 285.0 ms waited (working under the lock))"
        );
        assert!(
            f.line().contains("the kernel lock's holder: the IO thread"),
            "{}",
            f.line()
        );
        let off_cpu = LockHolder {
            thread: 5,
            start: 0x0084_0000,
            call: 0,
            call_name: String::new(),
            cpu_ns: Some(3_000_000),
            ..holder.clone()
        };
        assert_eq!(
            holder_words(&off_cpu),
            "game thread 5 (starts at 00840000) outside a platform call, main waited 286.0 ms; it ran 3.0 of the last \
             285.0 ms waited (mostly off the processor)"
        );
        let unknown = LockHolder {
            cpu_ns: None,
            thread: 0,
            ..holder
        };
        assert_eq!(
            holder_words(&unknown),
            "the host or main in ReadFile, main waited 286.0 ms"
        );

        // Taken outside a platform call by the port's own code (a `kernel_guard` site), and many short holds.
        let replying = LockHolder {
            thread: 4,
            start: 0x00aa_8430,
            call: 0x00fd_f2a0,
            call_name: "D3DXCreateTextureFromFileInMemory".into(),
            wait_ns: 18_300_000,
            render_wait: 1,
            ..LockHolder::default()
        };
        assert_eq!(
            holder_words(&replying),
            "the IO thread (game thread 4) in D3DXCreateTextureFromFileInMemory, main waited 18.3 ms; it was waiting for \
             a reply from the render thread"
        );
        let guarded = LockHolder {
            site: "crates/engine-core/src/mod_files.rs:120".into(),
            handoffs: 3,
            wait_ns: 18_300_000,
            ..LockHolder::default()
        };
        assert_eq!(
            holder_words(&guarded),
            "the host or main in crates/engine-core/src/mod_files.rs:120, main waited 18.3 ms; not one hold: the lock \
             changed hands 3 times while main waited"
        );

        // A game critical section's owner, named in the section wait's cause.
        let mut f = frame();
        f.ms = 72.0;
        f.typical = 16.7;
        f.stages = Some([22.0, 0.0, 2.9, 44.7, 0.4, 0.3, 0.0, 0.0, 0.1]);
        f.game[Count::SectionWaitNs as usize] = 35_200_000;
        f.game[Count::SectionWaits as usize] = 48;
        f.longest_wait = (3_600_000, WaitKind::Section, 0x00b6_6520);
        f.section_holder = Some(LockHolder {
            section: 0x0120_0010,
            thread: 4,
            start: 0x00aa_8430,
            call: 0x00fd_f2a0,
            call_name: "D3DXCreateTextureFromFileInMemory".into(),
            wait_ns: 3_600_000,
            measured_ns: 3_600_000,
            cpu_ns: Some(3_400_000),
            site: String::new(),
            handoffs: 0,
            render_wait: 0,
        });
        assert_eq!(
            f.cause(),
            "waited 35 ms for a lock another game thread held (at 00b66520) (section 01200010 owned by the IO thread \
             (game thread 4) in D3DXCreateTextureFromFileInMemory, main waited 3.6 ms; it ran 3.4 of the last 3.6 ms \
             waited (working under the lock))"
        );
        assert!(
            f.line()
                .contains("the critical section's owner: section 01200010"),
            "{}",
            f.line()
        );

        // A wait for the render thread named by what its present waited on.
        let mut f = frame();
        f.ms = 35.2;
        f.typical = 16.4;
        f.stages = Some([1.6, 0.1, 3.8, 4.3, 0.4, 19.9, 2.4, 2.7, 0.1]);
        {
            use engine_platform::stream_hitch::Count as S;
            f.stream[S::PresentWaitNs as usize] = 19_700_000;
            f.stream[S::AcquireNs as usize] = 16_200_000;
            f.stream[S::RenderSyncNs as usize] = 2_100_000;
            f.stream[S::SwapNs as usize] = 300_000;
        }
        assert_eq!(
            f.cause(),
            "waited 20 ms for the render thread (it waited for the window's next frame 16 ms, the queue thread's last \
             frame 2 ms)"
        );
        assert!(
            f.line().contains(
                "the render thread waited 16.2 ms for the window's frame, 2.1 for the queue thread"
            ),
            "{}",
            f.line()
        );

        // The maintainer's 181 ms wait in Present with the render thread's own waits near nothing: where its frame
        // went (a poll held behind the queue thread's submit, in this made-up split) and the longest calls by name.
        let mut f = frame();
        f.ms = 194.1;
        f.typical = 10.6;
        f.stages = Some([2.0, 0.1, 2.5, 3.2, 0.6, 180.8, 3.3, 1.6, 0.0]);
        {
            use engine_platform::stream_hitch::{Call, Count as S};
            f.stream[S::PresentWaitNs as usize] = 180_600_000;
            f.stream[S::RenderBusyNs as usize] = 182_000_000;
            f.stream[S::RenderPollNs as usize] = 150_000_000;
            f.stream[S::RenderCreateNs as usize] = 40_000;
            f.stream[S::QueueBusyNs as usize] = 152_000_000;
            f.stream[S::QueueSubmitNs as usize] = 149_800_000;
            f.stream[S::QueueUploadNs as usize] = 400_000;
            f.longest_calls = [
                Some((Call::Poll, 150_100_000)),
                Some((Call::Submit, 149_800_000)),
            ];
        }
        assert_eq!(
            f.cause(),
            "waited 181 ms for the render thread (busy with polls 150 ms, the game's commands 32 ms; the longest \
             call: a poll on the render thread 150.1 ms, a submit on the queue thread 149.8 ms)"
        );
        assert!(
            f.line().contains(
                "the render thread on the game's commands 182.0 ms (its own work 32.0, polls 150.0), the queue \
                 thread on its jobs 152.0 ms (submits 149.8, uploads 0.4), the longest call a poll on the render \
                 thread 150.1 ms, a submit on the queue thread 149.8 ms"
            ),
            "{}",
            f.line()
        );
        // Its present's waits are not its own work: counted apart, as before.
        {
            use engine_platform::stream_hitch::Count as S;
            f.stream[S::AcquireNs as usize] = 20_000_000;
            f.stream[S::RenderSyncNs as usize] = 2_000_000;
        }
        assert_eq!(f.render_own(), 9_960_000);
        assert!(
            f.cause().starts_with(
                "waited 181 ms for the render thread (it waited for the window's next frame 20 ms, the queue \
                 thread's last frame 2 ms; busy with polls 150 ms, the game's commands 10 ms;"
            ),
            "{}",
            f.cause()
        );
        // The sun shadows' caster pipelines made on the render thread: counted and named apart from the game's.
        {
            use engine_platform::stream_hitch::{Call, Count as S};
            f.stream[S::ShadowPipelines as usize] = 24;
            f.stream[S::ShadowPipelineNs as usize] = 160_000_000;
            f.longest_calls[0] = Some((Call::ShadowPipeline, 31_000_000));
        }
        assert!(
            f.cause().contains(
                "made 0 pipelines, 24 sun-shadow caster pipelines and 0 shader modules (160 ms)"
            ),
            "{}",
            f.cause()
        );
        assert!(
            f.line()
                .contains("24 sun-shadow caster pipelines made (160.0 ms)"),
            "{}",
            f.line()
        );
        // Those made on threads of their own while play goes on: named apart, no part of the wait.
        {
            use engine_platform::stream_hitch::Count as S;
            f.stream[S::ShadowPipelinesAhead as usize] = 3;
            f.stream[S::ShadowPipelineAheadNs as usize] = 21_000_000;
        }
        assert!(
            f.line()
                .contains("3 sun-shadow caster pipelines made on threads of their own (21.0 ms)"),
            "{}",
            f.line()
        );
        assert!(
            f.cause()
                .contains("24 sun-shadow caster pipelines and 0 shader modules (160 ms)"),
            "{}",
            f.cause()
        );
        assert!(
            f.line().contains(
                "the longest call a sun-shadow caster pipeline made on the render thread 31.0 ms"
            ),
            "{}",
            f.line()
        );
        // Main hardly waited for the render thread: its frame is not written out.
        let mut f = frame();
        {
            use engine_platform::stream_hitch::Count as S;
            f.stream[S::PresentWaitNs as usize] = 900_000;
            f.stream[S::RenderBusyNs as usize] = 8_000_000;
            f.stream[S::QueueBusyNs as usize] = 3_000_000;
        }
        assert!(f.render_frame_words().is_none());
        assert!(!f.line().contains("on the game's commands"), "{}", f.line());

        // The pause menu's open: the heap's measured time instead of the estimate, and the tiles made.
        let mut f = frame();
        f.stages = Some([1.8, 0.1, 2.2, 3.2, 0.5, 0.2, 1.8, 104.3, 0.0]);
        f.ms = 114.0;
        f.typical = 13.8;
        f.game[Count::AllocsMain as usize] = 80_640;
        f.game[Count::FreesMain as usize] = 15_963;
        f.game[Count::HeapNsMain as usize] = 61_000_000;
        f.game[Count::TilesMade as usize] = 2_100;
        assert_eq!(
            f.cause(),
            "allocated 80,640 memory blocks (61 ms in the heap)"
        );
        let l = f.line();
        assert!(l.contains("main in the heap 61.0 ms"), "{l}");
        assert!(
            l.contains("interface tiles 2100 made, 0 destroyed, 0 hibernating"),
            "{l}"
        );

        // Cells attached on a running route: their time, the hand-over and the references given their 3D.
        let mut f = frame();
        f.ms = 21.0;
        f.typical = 10.0;
        f.stages = Some([12.6, 0.1, 3.2, 2.8, 0.8, 0.3, 1.0, 0.2, 0.0]);
        f.game[Count::CellsAttached as usize] = 9;
        f.game[Count::CellAttachNs as usize] = 6_400_000;
        f.game[Count::LoadFinishNs as usize] = 3_100_000;
        f.game[Count::Refs3dAttached as usize] = 140;
        f.game[Count::HavokNsMain as usize] = 1_200_000;
        f.game[Count::HavokSteps as usize] = 2;
        f.game[Count::ScriptNsMain as usize] = 900_000;
        f.game[Count::ScriptRuns as usize] = 310;
        assert_eq!(
            f.cause(),
            "6 ms attaching cells; 3 ms finishing background loads on main; attached 9 cells"
        );
        let l = f.line();
        for part in [
            "cell attach 6.4 ms",
            "140 references given their 3D",
            "background loads on main in 3.1 ms",
            "the physics update 1.2 ms (2 steps)",
            "scripts 0.9 ms (310 runs)",
        ] {
            assert!(l.contains(part), "{part}: {l}");
        }
    }

    /// Main's reference changes that took Havok's global lock are named with their wait, and its wait for the
    /// lock taken whole (the world's lock); with none of them (the lock-free row on) neither part is there.
    #[test]
    fn main_s_havok_lock_waits_are_named() {
        let mut f = frame();
        f.ms = 18.0;
        f.typical = 5.4;
        f.game[Count::HavokRefLocks as usize] = 412;
        f.game[Count::HavokRefWaitNs as usize] = 7_000_000;
        f.game[Count::HavokLockAllWaitNs as usize] = 1_300_000;
        let l = f.line();
        for part in [
            "412 Havok reference changes on main took the global lock (waited 7.0 ms)",
            "main waited 1.3 ms for the Havok world lock",
        ] {
            assert!(l.contains(part), "{part}: {l}");
        }
        let mut f = frame();
        f.ms = 18.0;
        f.typical = 5.4;
        let l = f.line();
        assert!(!l.contains("Havok reference changes"), "{l}");
        assert!(!l.contains("Havok world lock"), "{l}");
    }

    /// The line: the frame against the typical one, big from 16 ms, the place, the stages, only the counts that are
    /// not zero, the cause, and the lines the cap left out.
    #[test]
    fn the_line_says_the_frame_the_place_the_stages_the_work_and_the_cause() {
        let mut f = frame();
        f.game[Count::FreesMain as usize] = 22_093;
        f.game[Count::Textures as usize] = 4;
        f.left_out = 3;
        let l = f.line();
        assert!(
            l.starts_with(
                "hitch: 64.0 ms big (typical 11.9) frame 15431 at 312.4 s, GoodspringsWorld"
            ),
            "{l}"
        );
        assert!(l.contains("; stages ms: update 40.1, before draw 0.2, culling and set-up 3.1, world draw 6.0, after world 1.2, present 2.3, after present 11.0"), "{l}");
        assert!(
            l.contains(
                "; work: main's memory blocks 0 allocated, 22,093 freed, 4 textures created;"
            ),
            "{l}"
        );
        assert!(!l.contains("pipelines"), "zero counts are left out: {l}");
        assert!(
            l.contains(
                "; likely cause: freed 22,093 memory blocks; the update took 40 ms (3 hitches before this one not written"
            ),
            "{l}"
        );
        let mut f = frame();
        f.ms = 9.0;
        f.typical = 3.4;
        f.menu = true;
        f.stages = None;
        let l = f.line();
        assert!(
            l.starts_with("hitch: 9.0 ms (typical 3.4) frame 15431 at 312.4 s in a menu, "),
            "{l}"
        );
        assert!(!l.contains("stages"), "{l}");
    }

    /// A long frame tail is split into its parts in the order main runs them, with what the two task queues' drains
    /// ran (the three kinds that took longest, the time outside the tasks, the turns spent waiting for another
    /// thread's input) and the free-list sort's pool; the cause names the longest part. A short tail is not split.
    #[test]
    fn a_long_frame_tail_is_split_into_its_parts() {
        use engine_core::stage_time::{DrainStats, Tail};
        let mut f = frame();
        f.ms = 14.0;
        f.typical = 5.7;
        f.stages = Some([1.0, 0.0, 0.8, 2.5, 0.3, 0.1, 0.8, 8.1, 0.0]);
        let first = Some(DrainStats {
            ns: 100_000,
            tasks: 2,
            kinds: [
                (0x14, 2, 60_000),
                (0, 0, 0),
                (0, 0, 0),
                (0, 0, 0),
                (0, 0, 0),
                (0, 0, 0),
            ],
            ..DrainStats::default()
        });
        let secondary = Some(DrainStats {
            ns: 7_590_000,
            tasks: 12,
            waits: 1_500,
            kinds: [
                (0x07, 3, 300_000),
                (0x22, 2, 200_000),
                (0x14, 4, 100_000),
                (0x0e, 1, 7_000_000),
                (0, 0, 0),
                (0, 0, 0),
            ],
            other: (2, 50_000),
        });
        f.tail = Some(Tail {
            parts: [
                Some(100_000),
                Some(60_000),
                Some(20_000),
                Some(30_000),
                Some(10_000),
                Some(90_000),
                Some(0),
                Some(0),
                Some(20_000),
                Some(10_000),
                Some(7_600_000),
                Some(10_000),
                Some(100_000),
                Some(10_000),
                Some(40_000),
                Some(0),
                Some(10_000),
            ],
            drains: [first, secondary],
            tick: Some((37, Some(1_203))),
            havok_sync: Some(engine_core::stage_time::HavokSync {
                actors_ns: 20_000,
                actors: 3,
                nodes_ns: 40_000,
                nodes: 40,
                cells_ns: 10_000,
                runs: 1,
            }),
            main_share: Some(250),
        });
        let l = f.line();
        assert!(
            l.contains(
                "; frame tail parts ms: AI tasks: projectiles and explosions 0.1, AI tasks: animation scene graphs \
                 (bones) 0.1, AI tasks: detection lists 0.0, AI tasks: actors' process and ragdolls 0.0, AI tasks: \
                 Pip-Boy and child attaches 0.0, AI tasks: first task queue 0.1 (2 tasks: tag 0x14 x2 0.1 ms; 0.0 \
                 ms outside the tasks; tag 0x04 is the physics results applied: 3 actors' Havok sync 0.0 ms, 40 \
                 nodes updated 0.0 ms, the cells' update 0.0 ms), AI tasks: queued node updates 0.0, AI tasks: the \
                 rest 0.0, delayed script actions 0.0, task queue end input 0.0, task queue drain 7.6 (12 tasks: tag \
                 0x0e x1 7.0 ms, tag 0x07 x3 0.3 ms, tag 0x22 x2 0.2 ms and 2 more kinds; 0.0 ms outside the tasks, \
                 1,500 turns on an empty queue waiting for another thread's input), task queue begin input 0.0, \
                 queued menu open, support menus and prints 0.1, other steps 0.0, free-list sort 0.0 (pool 37, 1,203 \
                 in its free list), the end of OnIdle 0.0, the game's loop to the host 0.0; main was on the \
                 processor for 2.0 of the 8.1 ms to OnIdle's end;"
            ),
            "{l}"
        );
        assert!(
            l.contains(
                "likely cause: the frame tail took 8 ms (most in the task queue drain, 7.6 ms; main on the \
                 processor for 25% of the tail) with no one-off work counted"
            ),
            "{l}"
        );
        // A part not passed (one processor: no task queue) is left out; a pool not in the table is said so.
        let t = f.tail.as_mut().unwrap();
        t.parts[9..12].fill(None);
        t.drains[1] = None;
        t.tick = Some((5, None));
        let l = f.line();
        assert!(
            l.contains("delayed script actions 0.0, queued menu open"),
            "{l}"
        );
        assert!(
            l.contains("free-list sort 0.0 (pool 5: none in the table)"),
            "{l}"
        );
        // A short tail is not split.
        f.stages = Some([1.0, 0.0, 0.8, 2.5, 0.3, 0.1, 0.8, 1.9, 0.0]);
        assert!(!f.line().contains("frame tail parts"));
    }

    /// A section's owner in no locked platform call is said so (it may be running game code, in a sync import or a
    /// device call, which it does not note), and a low share of processor time as blocked or off the processor.
    #[test]
    fn a_section_owner_in_no_locked_call_is_said_so() {
        let h = engine_core::hitch::LockHolder {
            section: 0x0120_0010,
            thread: 4,
            start: 0,
            call: 0,
            call_name: String::new(),
            wait_ns: 5_000_000,
            measured_ns: 5_000_000,
            cpu_ns: Some(500_000),
            site: String::new(),
            handoffs: 0,
            render_wait: 0,
        };
        assert_eq!(
            holder_words(&h),
            "section 01200010 owned by game thread 4 in no locked platform call, main waited 5.0 ms; it ran 0.5 of \
             the last 5.0 ms waited (blocked or off the processor)"
        );
    }

    #[test]
    fn stages_come_from_the_stage_columns() {
        let mut v = vec![(0.0, 0u64); engine_core::stage_time::COLUMNS];
        v[Column::U as usize] = (4000.0, 1);
        v[Column::PassTail as usize] = (500.0, 1);
        v[Column::AfterPass as usize] = (250.0, 1);
        v[Column::P1 as usize] = (1000.0, 1);
        v[Column::W as usize] = (2000.0, 1);
        v[Column::GameTail as usize] = (1500.0, 1);
        v[Column::HostGap as usize] = (250.0, 1);
        assert_eq!(
            stages_of(&v),
            [4.0, 0.0, 0.0, 0.0, 0.75, 0.0, 3.0, 1.5, 0.25]
        );
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(thousands(999), "999");
        assert_eq!(bytes(1536), "2 KB");
    }

    /// The cost of a normal frame's work (`--ignored`, release: `cargo test --release -p host --lib
    /// a_normal_frame_costs -- --ignored --nocapture`): the snapshots, the differences and the detector.
    #[test]
    #[ignore]
    fn a_normal_frame_costs_microseconds() {
        let mut d = Detector::default();
        let mut g = engine_core::hitch::snapshot();
        let n = 100_000;
        let t = Instant::now();
        for i in 0..n {
            let now = engine_core::hitch::snapshot();
            let s = engine_platform::stream_hitch::snapshot();
            std::hint::black_box(engine_core::hitch::since(&now, &g));
            std::hint::black_box(s);
            g = now;
            std::hint::black_box(engine_core::stage_time::take());
            std::hint::black_box(d.frame(5.0 + (i % 7) as f64 * 0.1));
        }
        println!(
            "{:.2} µs a frame",
            t.elapsed().as_secs_f64() * 1e6 / n as f64
        );
    }

    /// A stream whose writes wait until the test opens it (a pipe whose reader a busy machine starves, a console or
    /// file that holds the write): each write call it took, in order.
    struct HeldStream {
        open: Arc<(Mutex<bool>, Condvar)>,
        took: Arc<Mutex<Vec<String>>>,
    }

    impl Write for HeldStream {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let (open, wake) = &*self.open;
            let mut o = open.lock().unwrap();
            while !*o {
                o = wake.wait(o).unwrap();
            }
            self.took
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(bytes).into_owned());
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// The lines [`keep_for_test`] kept.
    static KEPT: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn keep_for_test(line: String) {
        KEPT.lock().unwrap().push(line);
    }

    /// A write that blocks (one hitch line's write took 360 ms in a route run) does not hold up main: each line is
    /// kept (the report's copy) and handed over at once while the stream holds every write, nothing is written
    /// meanwhile, and once the stream takes writes again each line is written in one call, text and newline together,
    /// in order, and a flush waits for them.
    #[test]
    fn a_write_that_blocks_does_not_hold_up_main() {
        let open = Arc::new((Mutex::new(false), Condvar::new()));
        let took = Arc::new(Mutex::new(Vec::new()));
        let lines = Lines::new(
            Box::new(HeldStream {
                open: open.clone(),
                took: took.clone(),
            }),
            keep_for_test,
        );
        // The stream opens by itself after 5 s, so a send that waited for the write would show as a slow send
        // (not a test that never ends).
        let (done, ended) = std::sync::mpsc::channel::<()>();
        let opener = {
            let open = open.clone();
            std::thread::spawn(move || {
                let _ = ended.recv_timeout(Duration::from_secs(5));
                *open.0.lock().unwrap() = true;
                open.1.notify_all();
            })
        };
        let t = Instant::now();
        for i in 0..3 {
            lines.send(format!("hitch {i}"));
        }
        let sent_in = t.elapsed();
        assert!(
            sent_in < Duration::from_secs(2),
            "the sends waited for the write: {sent_in:?}"
        );
        assert_eq!(
            *KEPT.lock().unwrap(),
            ["hitch 0", "hitch 1", "hitch 2"],
            "kept on the sending thread before any write"
        );
        assert!(
            !lines.flush(Duration::from_millis(100)),
            "nothing written while the stream holds its writes"
        );
        assert!(took.lock().unwrap().is_empty());
        *open.0.lock().unwrap() = true;
        open.1.notify_all();
        let _ = done.send(());
        assert!(lines.flush(Duration::from_secs(30)), "all written");
        assert_eq!(
            *took.lock().unwrap(),
            ["hitch 0\n", "hitch 1\n", "hitch 2\n"],
            "one write a line"
        );
        opener.join().unwrap();
    }

    /// The run's writer's stream held ([`hold_point`]; the global writer's test sets it).
    static HOLD: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

    /// [`ErrStream`]'s writes wait here while [`HOLD`] is set.
    pub(super) fn hold_point() {
        let mut held = HOLD.0.lock().unwrap_or_else(PoisonError::into_inner);
        while *held {
            held = HOLD.1.wait(held).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// The run's own writer ([`lines`]): a line sent is in the F12 report's hitch lines at once, while its write to the
    /// standard error stream is still held; and the platform layer's end of the process (its end hooks, which
    /// `end_process` runs first) waits for the held write before it goes on.
    #[test]
    fn the_run_s_lines_are_in_the_report_at_once_and_written_at_the_end() {
        engine_platform::d3d9real::set_report_hitch_totals(Some((0, 0, 0)));
        let line = format!("hitch log test line {}", std::process::id());
        *HOLD.0.lock().unwrap() = true;
        lines().send(line.clone());
        assert!(
            engine_platform::d3d9real::report_hitches_for_test().contains(&line),
            "in the report while its write is held"
        );
        let writer = LINES.get().expect("the run's writer");
        assert!(!writer.flush(Duration::ZERO), "its write is held");
        // The stream lets the write go 300 ms from now; the end hooks wait for it.
        let opener = std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(300));
            *HOLD.0.lock().unwrap() = false;
            HOLD.1.notify_all();
        });
        let t = Instant::now();
        engine_platform::run_end_hooks();
        assert!(
            writer.flush(Duration::ZERO),
            "written when the end hooks return (after {:?})",
            t.elapsed()
        );
        opener.join().unwrap();
    }
}
