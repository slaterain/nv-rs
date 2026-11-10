//! How long a frame takes from the game's `Present` to the screen, for `NV_RENDER_THREAD_STATS`
//! (a diagnostic, not the game's behaviour): the stream notes when each `Present` is submitted;
//! the backend notes when it carries it out, how long its own present call blocked, how many
//! frames the GPU still had to finish, and (where the graphics API says) when the frame reached
//! the screen. The recording side prints the sums once a second with the render thread's line.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Whether the timings are taken (`NV_RENDER_THREAD_STATS=1`).
pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("NV_RENDER_THREAD_STATS").is_ok_and(|v| v.trim() == "1"))
}

fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// Nanoseconds since the first timing (0 is never a time: one is added).
fn now() -> u64 {
    epoch().elapsed().as_nanos() as u64 + 1
}

/// The presents remembered (by frame number): more than any queue holds.
const RING: usize = 64;

static SUBMITTED_FRAME: [AtomicU64; RING] = [const { AtomicU64::new(0) }; RING];
static SUBMITTED_AT: [AtomicU64; RING] = [const { AtomicU64::new(0) }; RING];

/// The stream: the game submitted `Present` number `frame` now.
pub(crate) fn submitted(frame: u32) {
    if !on() {
        return;
    }
    let i = frame as usize % RING;
    SUBMITTED_AT[i].store(now(), Ordering::Relaxed);
    SUBMITTED_FRAME[i].store(u64::from(frame), Ordering::Relaxed);
}

/// A sum, a count and a largest value.
struct Measure {
    sum: AtomicU64,
    count: AtomicU64,
    max: AtomicU64,
}

impl Measure {
    const fn new() -> Measure {
        Measure {
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            max: AtomicU64::new(0),
        }
    }

    fn add(&self, v: u64) {
        self.sum.fetch_add(v, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.max.fetch_max(v, Ordering::Relaxed);
    }

    /// (mean, largest, count) since the last take.
    fn take(&self) -> (f64, u64, u64) {
        let sum = self.sum.swap(0, Ordering::Relaxed);
        let count = self.count.swap(0, Ordering::Relaxed);
        let max = self.max.swap(0, Ordering::Relaxed);
        (sum as f64 / count.max(1) as f64, max, count)
    }
}

/// Game's Present submitted -> the backend starts carrying it out (ns).
static QUEUED: Measure = Measure::new();
/// The backend's own present call (and the wait for a frame to draw into) (ns).
static BLOCKED: Measure = Measure::new();
/// Game's Present submitted -> the frame starts to scan out (ns), where the API says.
static TO_SCREEN: Measure = Measure::new();
/// Frames handed to the GPU and not finished, at each present.
static GPU_FRAMES: Measure = Measure::new();
/// Presents the API answered for (on screen) and those it did not (dropped or not known).
static NOT_SHOWN: AtomicU64 = AtomicU64::new(0);

/// The backend starts carrying out `Present` number `frame`: the time since the game submitted
/// it, none when not known (timings off, or a frame from before the ring).
pub fn carried_out(frame: u32) -> Option<Duration> {
    if !on() {
        return None;
    }
    let i = frame as usize % RING;
    if SUBMITTED_FRAME[i].load(Ordering::Relaxed) != u64::from(frame) {
        return None;
    }
    let at = SUBMITTED_AT[i].load(Ordering::Relaxed);
    let since = now().saturating_sub(at);
    QUEUED.add(since);
    Some(Duration::from_nanos(since))
}

/// The backend's present call blocked for `d`.
pub fn blocked(d: Duration) {
    BLOCKED.add(d.as_nanos() as u64);
}

/// A frame reached the screen `d` after the game submitted it.
pub fn on_screen(d: Duration) {
    TO_SCREEN.add(d.as_nanos() as u64);
}

/// A present the API says was not shown (or whose time it no longer knows).
pub fn not_shown() {
    NOT_SHOWN.fetch_add(1, Ordering::Relaxed);
}

/// `n` frames were handed to the GPU and not finished when a present was carried out.
pub fn gpu_frames(n: u64) {
    GPU_FRAMES.add(n);
}

/// The line's part since the last call ("" with nothing measured).
pub(crate) fn take_line() -> String {
    let ms = |ns: f64| ns / 1e6;
    let (q, qmax, qn) = QUEUED.take();
    let (b, bmax, bn) = BLOCKED.take();
    let (s, smax, sn) = TO_SCREEN.take();
    let (g, gmax, gn) = GPU_FRAMES.take();
    let lost = NOT_SHOWN.swap(0, Ordering::Relaxed);
    let mut line = String::new();
    if qn > 0 {
        line += &format!(
            "; Present to the backend {:.2} ms (max {:.2})",
            ms(q),
            ms(qmax as f64)
        );
    }
    if bn > 0 {
        line += &format!(
            ", blocked in present {:.2} ms (max {:.2})",
            ms(b),
            ms(bmax as f64)
        );
    }
    if gn > 0 {
        line += &format!(", GPU frames queued {g:.2} (max {gmax})");
    }
    if sn > 0 || lost > 0 {
        line += &format!(
            ", Present to the screen {:.2} ms (max {:.2}; {sn} frames timed, {lost} not)",
            ms(s),
            ms(smax as f64)
        );
    }
    line
}
