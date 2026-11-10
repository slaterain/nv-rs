//! `NV_RENDER_THREAD_STATS` (a diagnostic, not the game's behaviour): the window's presents timed
//! for `d3d9_stream::latency`: the wait for a frame to draw into and the present call, the frames
//! the GPU has not finished, and, on Direct3D 12, when each frame started to scan out (DXGI's
//! frame statistics: the vertical blank at which the present was shown).

use d3d9_stream::latency;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// One window's present timings.
#[derive(Default)]
pub(crate) struct Timing {
    /// The present being carried out: when it started and how long after the game's `Present`.
    current: Option<(Instant, Duration)>,
    blocked: Duration,
    /// Frames submitted, and finished (counted by the queue's callbacks).
    sent: u64,
    done: Arc<AtomicU64>,
    /// Presents not yet seen on screen: DXGI's present count, the performance counter at the
    /// present call, and the time since the game's `Present` then.
    waiting: VecDeque<(u32, i64, Duration)>,
    /// The last present count the statistics showed.
    shown: u32,
}

impl Timing {
    /// The backend starts carrying out `Present` number `frame`; false when not timed.
    pub(crate) fn start(&mut self, frame: u32) -> bool {
        self.blocked = Duration::ZERO;
        self.current = latency::carried_out(frame).map(|since| (Instant::now(), since));
        self.current.is_some()
    }

    /// `d` spent waiting for the window's frame or in the present call.
    pub(crate) fn add_blocked(&mut self, d: Duration) {
        self.blocked += d;
    }

    /// The frame's work was submitted: counted until the queue says it is done.
    pub(crate) fn submitted(&mut self, queue: &wgpu::Queue) {
        if self.current.is_none() {
            return;
        }
        let done = self.done.load(Ordering::Relaxed);
        latency::gpu_frames(self.sent.saturating_sub(done));
        self.sent += 1;
        let counter = self.done.clone();
        queue.on_submitted_work_done(move || {
            counter.fetch_add(1, Ordering::Relaxed);
        });
    }

    /// After the present call: its blocking time, and on Direct3D 12 the frame statistics.
    pub(crate) fn presented(&mut self, surface: &wgpu::Surface<'_>) {
        let Some((started, since)) = self.current.take() else {
            return;
        };
        latency::blocked(self.blocked);
        #[cfg(windows)]
        self.dxgi(surface, since + started.elapsed());
        #[cfg(not(windows))]
        let _ = (surface, since, started);
    }

    #[cfg(windows)]
    fn dxgi(&mut self, surface: &wgpu::Surface<'_>, since: Duration) {
        use windows::Win32::Graphics::Dxgi::DXGI_FRAME_STATISTICS;
        use windows::Win32::System::Performance::{
            QueryPerformanceCounter, QueryPerformanceFrequency,
        };
        // SAFETY: the swap chain is only read (its counts and statistics), on the thread that
        // owns the surface, while the surface lives.
        let Some(sc) =
            (unsafe { surface.as_hal::<wgpu::hal::api::Dx12>() }).and_then(|s| s.swap_chain())
        else {
            return;
        };
        let (mut now, mut frequency) = (0i64, 0i64);
        // SAFETY: two out-parameters of this frame.
        unsafe {
            let _ = QueryPerformanceCounter(&mut now);
            let _ = QueryPerformanceFrequency(&mut frequency);
        }
        // SAFETY: as above.
        if let Ok(count) = unsafe { sc.GetLastPresentCount() } {
            self.waiting.push_back((count, now, since));
            if self.waiting.len() > 64 {
                self.waiting.pop_front();
                latency::not_shown();
            }
        }
        let mut st = DXGI_FRAME_STATISTICS::default();
        // SAFETY: an out-parameter; fails (no statistics yet, or disjoint) without writing.
        if unsafe { sc.GetFrameStatistics(&mut st) }.is_err() || st.PresentCount == self.shown {
            return;
        }
        self.shown = st.PresentCount;
        while let Some(&(count, at, since)) = self.waiting.front() {
            if count > st.PresentCount {
                break;
            }
            self.waiting.pop_front();
            if count < st.PresentCount {
                // Replaced before it was shown, or shown at a blank no statistics were read for.
                latency::not_shown();
                continue;
            }
            let after = (st.SyncQPCTime - at) as f64 / frequency.max(1) as f64;
            if after >= 0.0 {
                latency::on_screen(since + Duration::from_secs_f64(after));
            } else {
                latency::not_shown();
            }
        }
    }
}
