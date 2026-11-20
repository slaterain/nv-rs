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
//! points). A hitch frame adds one formatted line, one write to the log and, when the player's cell changed since
//! the last line, one call of the cell's `GetEditorID`.

use engine::Engine;
use engine_core::hitch::{Count, WaitKind, COUNTS};
use engine_core::stage_time::Column;
use std::collections::VecDeque;
use std::time::Instant;

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
const PARENT_CELL: u32 = 0x40;
const POSITION: u32 = 0x30;

/// The typical frame and the hitch test.
#[derive(Clone)]
pub struct Detector {
    ring: Vec<f64>,
    at: usize,
    median: f64,
    since: u32,
}

impl Default for Detector {
    fn default() -> Detector {
        Detector {
            ring: Vec::with_capacity(WINDOW),
            at: 0,
            median: 0.0,
            since: MEDIAN_EVERY,
        }
    }
}

impl Detector {
    /// A frame of `ms` that was not a loading screen: the typical frame before it when it is a hitch, then it joins
    /// the frames the median is taken over.
    pub fn frame(&mut self, ms: f64) -> Option<f64> {
        let hitch =
            (self.ring.len() >= WARM && ms >= floor(self.median) && ms >= 2.0 * self.median)
                .then_some(self.median);
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
        hitch
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

/// At most [`BURST`] lines in any [`BURST_SECS`]: the times of the last lines written, and those left out since.
#[derive(Default)]
pub struct Cap {
    written: VecDeque<f64>,
    left_out: u64,
}

impl Cap {
    /// Whether a line at `secs` (session seconds) is written; `Some(n)` with the lines left out before it.
    pub fn admit(&mut self, secs: f64) -> Option<u64> {
        while self
            .written
            .front()
            .is_some_and(|&t| secs - t >= BURST_SECS)
        {
            self.written.pop_front();
        }
        if self.written.len() >= BURST {
            self.left_out += 1;
            return None;
        }
        self.written.push_back(secs);
        Some(std::mem::take(&mut self.left_out))
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
    /// swap's tail), after Present (main's waits for its threads included).
    pub stages: Option<[f64; 7]>,
    pub game: Vec<u64>,
    pub stream: Vec<u64>,
    pub longest_wait: (u64, WaitKind, u32),
    pub largest_read: (u64, String),
    pub left_out: u64,
}

const STAGE_NAMES: [&str; 8] = [
    "update",
    "before draw",
    "culling and set-up",
    "world draw",
    "after world",
    "present",
    "after present",
    "time between frames",
];

/// The stages (ms) from the stage marks' take (µs per column).
pub fn stages_of(v: &[(f64, u64)]) -> [f64; 7] {
    let us = |c: Column| v.get(c as usize).map_or(0.0, |x| x.0);
    [
        us(Column::U),
        us(Column::S1),
        us(Column::RBeforeCut),
        us(Column::CutToDrawEnd),
        us(Column::PassTail) + us(Column::AfterPass),
        us(Column::S2),
        us(Column::P1) + us(Column::W),
    ]
    .map(|x| x / 1000.0)
}

/// The stage marks of the frame just ended (taken once a frame for the hitch log and the frame log), when the
/// points are timed.
pub fn take_stages() -> Option<[(f64, u64); engine_core::stage_time::COLUMNS]> {
    engine_core::stage_time::points_on().then(engine_core::stage_time::take)
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

    /// The stages with the time between the game's frames (the frame less the stages: the host's work between two
    /// of main's frames, the hand-back, and the part of main's frame after `WaitForThreads`).
    fn stages8(&self, st: [f64; 7]) -> [f64; 8] {
        let between = (self.ms - st.iter().sum::<f64>()).max(0.0);
        std::array::from_fn(|i| if i < 7 { st[i] } else { between })
    }

    /// Main's wait for the render thread (ns): in Present, for replies, for room in the queue.
    fn render_wait(&self) -> u64 {
        use engine_platform::stream_hitch::Count as S;
        self.s(S::PresentWaitNs) + self.s(S::ReplyWaitNs) + self.s(S::QueueFullNs)
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
            said.push((
                ms(section),
                format!(
                    "waited {:.0} ms for a lock another game thread held{at}",
                    ms(section)
                ),
            ));
        }
        let locks = self.g(Count::KernelWaitNs) + self.g(Count::DeviceWaitNs);
        if locks > 0 {
            said.push((
                ms(locks),
                format!("waited {:.0} ms for the platform's locks", ms(locks)),
            ));
        }
        let render = self.render_wait();
        if render > 0 {
            said.push((
                ms(render),
                format!("waited {:.0} ms for the render thread", ms(render)),
            ));
        }
        let made = self.s(S::PipelineNs) + self.s(S::ModuleNs);
        if made > 0 {
            // Made on the render thread or a helper: main feels it through its wait for the render thread.
            said.push((
                ms(made).min(ms(render).max(ms(made) / 2.0)),
                format!(
                    "made {} pipelines and {} shader modules ({:.0} ms)",
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
        let blocks = self.g(Count::FreesMain) + self.g(Count::AllocsMain);
        if blocks >= 2_000 {
            let freed = self.g(Count::FreesMain);
            let words = if freed * 2 >= blocks {
                format!("freed {} memory blocks", thousands(freed))
            } else {
                format!(
                    "allocated {} memory blocks",
                    thousands(self.g(Count::AllocsMain))
                )
            };
            said.push((blocks as f64 * 0.0004, words));
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
                    .stages8(st)
                    .into_iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .expect("eight stages");
                let none = if strong.is_empty() {
                    " with no one-off work counted"
                } else {
                    ""
                };
                out.push(format!("the {} took {m:.0} ms{none}", STAGE_NAMES[i]));
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
                .zip(self.stages8(st))
                .map(|(n, m)| format!("{n} {m:.1}"))
                .collect();
            l += &format!("; stages ms: {}", parts.join(", "));
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
            g(Count::AllocsMain) + g(Count::FreesMain),
            format!(
                "main's memory blocks {} allocated, {} freed",
                thousands(g(Count::AllocsMain)),
                thousands(g(Count::FreesMain))
            ),
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
            g(Count::HandBackNs) / TENTH_MS,
            format!(
                "memory handed back at the frame's end {:.1} ms",
                ms(g(Count::HandBackNs))
            ),
        );
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
                " ({} hitches before this one not written: too many in 10 s)",
                self.left_out
            );
        }
        l
    }
}

/// The player's cell as the line names it, kept while the cell stays the same.
struct Place {
    cell: u32,
    name: String,
}

impl Default for Place {
    /// No cell named yet (not even 'no cell').
    fn default() -> Place {
        Place {
            cell: u32::MAX,
            name: String::new(),
        }
    }
}

/// The hitch log of a run.
pub struct HitchLog {
    /// The typical frame in play and in menus, apart: a menu runs at its own frame limit (`MenuFrameLimit`), so a
    /// menu frame is judged against menu frames and a play frame against play frames.
    play: Detector,
    menu: Detector,
    cap: Cap,
    /// The frame before showed a menu.
    was_menu: bool,
    /// A measured run (`NV_FRAME_LOG`): every hitch is written, none left out.
    uncapped: bool,
    /// The presents submitted so far.
    presents: u32,
    last: Option<Instant>,
    start: Instant,
    frame: u64,
    was_loading: bool,
    game: [u64; COUNTS],
    stream: [u64; engine_platform::stream_hitch::COUNTS],
    place: Place,
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
        if off {
            return None;
        }
        Some(HitchLog {
            play: Detector::default(),
            menu: Detector::default(),
            cap: Cap::default(),
            uncapped: std::env::var_os("NV_FRAME_LOG").is_some(),
            presents: 0,
            was_menu: false,
            last: None,
            start: Instant::now(),
            frame: 0,
            was_loading: false,
            game: engine_core::hitch::snapshot(),
            stream: engine_platform::stream_hitch::snapshot(),
            place: Place::default(),
            hitches: 0,
            big: 0,
        })
    }

    /// After one of main's frames: `stages` the stage marks' take of this frame (none when nothing took them).
    pub fn record(&mut self, e: &mut Engine, stages: Option<&[(f64, u64)]>) {
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
        let screen = crate::uncap::screen(e);
        // A game loaded in the frame (from a menu: no loading screen is up) counts as a loading screen too.
        let presents = engine_platform::d3d9real::presents(e).map_or(0, |p| p.0);
        let shown = presents.wrapping_sub(self.presents);
        self.presents = presents;
        let (judged, after_load) = judge(
            screen == crate::uncap::Screen::Loading,
            game[Count::Loads as usize],
            shown,
            self.was_loading,
        );
        self.was_loading = !judged;
        let (Some(ms), true) = (frame_ms, judged) else {
            return;
        };
        let menu = screen == crate::uncap::Screen::Menu;
        // Judged by what the frame before showed: the frame that opens a menu (the Pip-Boy, the console) is play's
        // frame, the frames after it run at the menu's limit.
        let was_menu = std::mem::replace(&mut self.was_menu, menu);
        let detector = if was_menu {
            &mut self.menu
        } else {
            &mut self.play
        };
        let Some(typical) = detector.frame(ms) else {
            return;
        };
        self.hitches += 1;
        if ms >= BIG_MS {
            self.big += 1;
        }
        let secs = now.duration_since(self.start).as_secs_f64();
        let left_out = if self.uncapped {
            0
        } else {
            let Some(n) = self.cap.admit(secs) else {
                return;
            };
            n
        };
        let f = Frame {
            ms,
            typical,
            frame,
            secs,
            after_load,
            menu,
            place: self.place(e),
            stages: stages.map(stages_of),
            game: game.to_vec(),
            stream,
            longest_wait,
            largest_read,
            left_out,
        };
        let line = f.line();
        eprintln!("{line}");
        engine_platform::d3d9real::note_report_hitch(line);
    }

    /// Where the player is: the cell (editor id looked up only when it changed) and, outside, the grid cell.
    fn place(&mut self, e: &mut Engine) -> String {
        let player = e.mem.u32(PLAYER);
        if player == 0 {
            return "no player yet".into();
        }
        let cell = e.mem.u32(player + PARENT_CELL);
        if cell != self.place.cell {
            self.place = Place {
                cell,
                name: crate::report_place::cell_name(e, cell),
            };
        }
        let position = [0, 4, 8].map(|o| e.mem.f32(player + POSITION + o));
        let (gx, gy) = crate::report_place::grid(position);
        if self.place.name.starts_with("interior ") {
            self.place.name.clone()
        } else {
            format!("{} grid ({gx}, {gy})", self.place.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warm(d: &mut Detector, ms: f64, n: usize) {
        for _ in 0..n {
            assert_eq!(d.frame(ms), None);
        }
    }

    /// A frame of twice the typical one and at least 8 ms is a hitch; one just under either is not; the first
    /// frames (no typical frame yet) are never judged.
    #[test]
    fn a_hitch_is_twice_the_typical_frame_and_at_least_8_ms() {
        let mut d = Detector::default();
        assert_eq!(d.frame(100.0), None, "nothing judged before the warm-up");
        let mut d = Detector::default();
        warm(&mut d, 5.0, 40);
        assert_eq!(d.typical(), 5.0);
        assert_eq!(d.frame(9.9), None, "under twice the typical");
        assert_eq!(d.frame(10.0), Some(5.0));
        let mut d = Detector::default();
        warm(&mut d, 3.4, 40);
        assert_eq!(d.frame(6.7), None, "under twice the typical");
        assert_eq!(d.frame(6.8), Some(3.4));
        let mut d = Detector::default();
        warm(&mut d, 2.5, 40);
        assert_eq!(d.frame(5.9), None, "under the fast floor");
        assert_eq!(d.frame(6.0), Some(2.5));
        let mut d = Detector::default();
        warm(&mut d, 3.9, 40);
        assert_eq!(
            d.frame(7.9),
            Some(3.9),
            "a typical frame under 4 ms: the 6 ms floor"
        );
        let mut d = Detector::default();
        warm(&mut d, 4.0, 40);
        assert_eq!(d.frame(7.9), None, "4 ms and over: the 8 ms floor");
        assert_eq!(d.frame(8.0), Some(4.0));
        let mut d = Detector::default();
        warm(&mut d, 12.0, 40);
        assert_eq!(d.frame(23.9), None);
        assert_eq!(d.frame(24.0), Some(12.0));
    }

    /// The typical frame follows the last 128 frames (the median, so a few long ones do not move it), and is
    /// worked out again every 16 frames.
    #[test]
    fn the_typical_frame_is_the_median_of_the_last_128() {
        let mut d = Detector::default();
        warm(&mut d, 5.0, 128);
        for _ in 0..10 {
            d.frame(60.0);
        }
        assert_eq!(d.typical(), 5.0, "a few long frames do not move it");
        for _ in 0..128 {
            d.frame(20.0);
        }
        assert_eq!(d.typical(), 20.0, "the window moved on");
        assert_eq!(d.frame(39.0), None);
        assert_eq!(d.frame(40.0), Some(20.0));
        assert_eq!(median(&[3.0, 1.0, 2.0, 10.0]), 3.0);
        assert_eq!(median(&[]), 0.0);
    }

    /// At most 20 lines in any 10 s; the first line after a burst says how many were left out.
    #[test]
    fn the_cap_writes_20_lines_in_10_s_and_counts_the_rest() {
        let mut c = Cap::default();
        for i in 0..20 {
            assert_eq!(c.admit(i as f64 * 0.1), Some(0));
        }
        assert_eq!(c.admit(2.0), None);
        assert_eq!(c.admit(5.0), None);
        // 10 s after the first line it leaves the window: one more fits.
        assert_eq!(c.admit(10.0), Some(2));
        assert_eq!(c.admit(10.05), None);
        assert_eq!(c.admit(30.0), Some(1));
    }

    fn frame() -> Frame {
        Frame {
            ms: 64.0,
            typical: 11.9,
            frame: 15431,
            secs: 312.4,
            place: "GoodspringsWorld? WastelandNV (0x000da726) grid (-19, 1)".into(),
            stages: Some([40.1, 0.2, 3.1, 6.0, 1.2, 2.3, 11.0]),
            game: vec![0; COUNTS],
            stream: vec![0; engine_platform::stream_hitch::COUNTS],
            ..Frame::default()
        }
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
        f.stages = Some([2.0, 0.0, 1.0, 1.0, 0.5, 0.2, 0.3]);
        assert_eq!(
            f.cause(),
            "the time between frames took 25 ms with no one-off work counted"
        );

        let mut f = frame();
        f.game[Count::ReadNsMain as usize] = 30_000_000;
        f.game[Count::BytesReadMain as usize] = 3 << 20;
        f.largest_read = (2 << 20, "Fallout - Meshes.bsa".into());
        assert_eq!(
            f.cause(),
            "read 3.0 MB from disk on the main thread (30 ms, the largest 2.0 MB of Fallout - Meshes.bsa)"
        );
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

    #[test]
    fn stages_come_from_the_stage_columns() {
        let mut v = vec![(0.0, 0u64); engine_core::stage_time::COLUMNS];
        v[Column::U as usize] = (4000.0, 1);
        v[Column::PassTail as usize] = (500.0, 1);
        v[Column::AfterPass as usize] = (250.0, 1);
        v[Column::P1 as usize] = (1000.0, 1);
        v[Column::W as usize] = (2000.0, 1);
        assert_eq!(stages_of(&v), [4.0, 0.0, 0.0, 0.0, 0.75, 0.0, 3.0]);
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
}
