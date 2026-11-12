//! `NV_FRAME_LOG=FILE`: one line for each of main's frames, a measuring aid (docs/FIXES.md): the
//! microsecond clock, the main timer's step, the Havok accumulator, the player's position and
//! whether the frame shows play, a menu or a loading screen (`uncap::screen`), and the Windows
//! performance counter (`qpc`), so an outside sampler's timestamps line up with the frames (0 off
//! Windows).
//! The delay log: when the frame polled the controls (`poll_qpc`, the keyboard's `GetDeviceState`),
//! the frame's Present (`present`, its number), the last Present the backend carried out
//! (`shown`, `shown_qpc`), and for that frame the delay from its poll to its Present carried out
//! (`input_to_present_us`); every 1000 frames a line `# input to present ...` with the median and
//! the 95th percentile since the last such line.
//! It reads guest memory between frames, so unlike a trace it sends no call down the slow path and
//! the frame rate stays the player's. Off, nothing runs.

use engine::Engine;
use std::collections::VecDeque;
use std::io::Write;
use std::time::Instant;

/// The main timer's `fDelta`.
const MAIN_TIMER_DELTA: u32 = 0x011f_6394 + 0xc;
/// The Havok step count and the time the frame's marker moves by.
const STEP_COUNT: u32 = 0x0126_77b4;
const STEP_TIME: u32 = 0x011a_fe64;
/// The player (`PlayerCharacter`), and where a reference keeps its position.
const PLAYER: u32 = 0x011d_ea3c;
const POSITION: u32 = 0x30;
/// Frames between two `# input to present` lines.
const SUMMARY_EVERY: u64 = 1000;

/// The Windows performance counter now (`QueryPerformanceCounter`) and its ticks a second; 0 and
/// 1 elsewhere.
fn performance_counter() -> (i64, i64) {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn QueryPerformanceCounter(count: *mut i64) -> i32;
            fn QueryPerformanceFrequency(frequency: *mut i64) -> i32;
        }
        let (mut count, mut frequency) = (0i64, 0i64);
        // SAFETY: each call writes the one i64 it is given.
        unsafe {
            QueryPerformanceCounter(&mut count);
            QueryPerformanceFrequency(&mut frequency);
        }
        (count, frequency.max(1))
    }
    #[cfg(not(windows))]
    {
        (0, 1)
    }
}

/// The counter's value at `at`, from its value `qpc` at `now`.
fn qpc_at(at: Instant, now: Instant, (qpc, frequency): (i64, i64)) -> i64 {
    let back = now.saturating_duration_since(at).as_nanos() as i128;
    qpc - (back * i128::from(frequency) / 1_000_000_000) as i64
}

/// The input-to-present delays: each frame's poll by its Present's number, until the backend has
/// carried that Present out.
#[derive(Default)]
struct Delays {
    /// (the frame's Present number, when its controls were polled), oldest first.
    waiting: VecDeque<(u32, Instant)>,
    /// The delays measured since the last summary, microseconds.
    since: Vec<u64>,
}

impl Delays {
    /// A frame: its Present's number and its poll (if it polled); `shown` the last Present the
    /// backend carried out by now. Answers that frame's delay when its poll is known (each frame's
    /// once: the frames between two looks are skipped).
    fn frame(
        &mut self,
        present: u32,
        polled: Option<Instant>,
        shown: Option<(u32, Instant)>,
    ) -> Option<u64> {
        if let Some(at) = polled {
            if self.waiting.back().map_or(true, |&(p, _)| p != present) {
                self.waiting.push_back((present, at));
                if self.waiting.len() > 8 {
                    self.waiting.pop_front();
                }
            }
        }
        let (shown, at) = shown?;
        let mut delay = None;
        while let Some(&(p, polled)) = self.waiting.front() {
            if p > shown {
                break;
            }
            self.waiting.pop_front();
            if p == shown {
                let us = at.saturating_duration_since(polled).as_micros() as u64;
                self.since.push(us);
                delay = Some(us);
            }
        }
        delay
    }
}

/// The median and the 95th percentile of `delays` (sorted here); none for no delays.
fn percentiles(delays: &mut [u64]) -> Option<(u64, u64)> {
    if delays.is_empty() {
        return None;
    }
    delays.sort_unstable();
    let at = |q: f64| delays[((delays.len() - 1) as f64 * q).round() as usize];
    Some((at(0.5), at(0.95)))
}

/// The open log.
pub struct FrameLog {
    out: std::io::BufWriter<std::fs::File>,
    frame: u64,
    delays: Delays,
}

impl FrameLog {
    /// The log `NV_FRAME_LOG` names, if any. A file that can't be made is said once and no log.
    pub fn from_env() -> Option<FrameLog> {
        let path = std::env::var_os("NV_FRAME_LOG")?;
        let file = match std::fs::File::create(&path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("NV_FRAME_LOG: {e}");
                return None;
            }
        };
        let mut out = std::io::BufWriter::new(file);
        let _ = writeln!(
            out,
            "frame\tus\tm_fdelta\thk_count\thk_time\tpx\tpy\tpz\tscreen\tqpc\tpoll_qpc\tpresent\tshown\t\
             shown_qpc\tinput_to_present_us"
        );
        Some(FrameLog {
            out,
            frame: 0,
            delays: Delays::default(),
        })
    }

    /// One frame's line.
    pub fn record(&mut self, e: &mut Engine) {
        let us = e.clock_now_us();
        let delta = e.mem.f32(MAIN_TIMER_DELTA);
        let count = e.mem.u32(STEP_COUNT);
        let time = e.mem.f32(STEP_TIME);
        let player = e.mem.u32(PLAYER);
        let at = if player == 0 {
            [0.0; 3]
        } else {
            [0, 4, 8].map(|o| e.mem.f32(player + POSITION + o))
        };
        let now = Instant::now();
        let counter = performance_counter();
        let polled = engine_platform::dinput::controls_polled_at(e);
        let (present, shown) = presents(e);
        let delay = self.delays.frame(present, polled, shown);
        let qpc = |t: Option<Instant>| t.map_or(0, |t| qpc_at(t, now, counter));
        let _ = writeln!(
            self.out,
            "{}\t{us}\t{delta}\t{count}\t{time}\t{}\t{}\t{}\t{}\t{}\t{}\t{present}\t{}\t{}\t{}",
            self.frame,
            at[0],
            at[1],
            at[2],
            crate::uncap::screen(e).name(),
            counter.0,
            qpc(polled),
            shown.map_or(0, |s| s.0),
            qpc(shown.map(|s| s.1)),
            delay.map_or(String::new(), |d| d.to_string()),
        );
        self.frame += 1;
        if self.frame % SUMMARY_EVERY == 0 {
            let n = self.delays.since.len();
            if let Some((p50, p95)) = percentiles(&mut self.delays.since) {
                let _ = writeln!(
                    self.out,
                    "# input to present over the last {n} measured frames: p50 {:.3} ms, p95 {:.3} ms",
                    p50 as f64 / 1e3,
                    p95 as f64 / 1e3
                );
            }
            self.delays.since.clear();
            let _ = self.out.flush();
        }
    }
}

/// The Presents submitted so far (this frame's number) and the last one the backend carried out.
fn presents(e: &mut Engine) -> (u32, Option<(u32, Instant)>) {
    #[cfg(windows)]
    {
        engine_platform::d3d9real::presents(e).unwrap_or((0, None))
    }
    #[cfg(not(windows))]
    {
        let _ = e;
        (0, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Each frame's delay runs from its poll to its own Present carried out, matched by number,
    /// also when the backend is a frame behind; frames between two looks are skipped, and a frame
    /// that did not poll has none.
    #[test]
    fn a_frame_s_delay_runs_from_its_poll_to_its_present_carried_out() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut d = Delays::default();
        // Frame 1 polls at 0 and presents; nothing carried out yet.
        assert_eq!(d.frame(1, Some(ms(0)), None), None);
        // Frame 2 polls at 10; Present 1 was carried out at 12.
        assert_eq!(d.frame(2, Some(ms(10)), Some((1, ms(12)))), Some(12_000));
        // Frame 3 polls at 20; Present 3 (and 2 before it) carried out at 26: frame 3's delay.
        assert_eq!(d.frame(3, Some(ms(20)), Some((3, ms(26)))), Some(6_000));
        // Frame 4 did not poll: no delay for it when its Present is carried out.
        assert_eq!(d.frame(4, None, Some((4, ms(35)))), None);
        assert_eq!(d.since, vec![12_000, 6_000]);
    }

    #[test]
    fn the_percentiles_are_the_median_and_the_95th() {
        assert_eq!(percentiles(&mut []), None);
        let mut v: Vec<u64> = (1..=100).rev().collect();
        assert_eq!(percentiles(&mut v), Some((51, 95)));
        assert_eq!(percentiles(&mut [7]), Some((7, 7)));
    }
}
