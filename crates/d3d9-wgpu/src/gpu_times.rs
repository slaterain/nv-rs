//! `NV_WGPU_GPU_TIMES=1`, a measuring aid (off by default): the graphics card's own time for each kind
//! of work the backend gives it, from timestamps the card writes at the start and end of each pass (and,
//! where the driver can write them between passes, around copies and the backend's other work), summed
//! into one line every 5 s in plain words, for players to send:
//!
//! `NV_D3D9 wgpu: GPU time a frame (300 frames): world 6.10, occlusion count 0.20, resolves 0.40,
//! clears 0.10, copies 0.30, HDR 0.05, sun shadow maps 0.00, other 0.00; total 7.15 ms`
//!
//! - world: the game's own draws (each pass the draws go in; a partial clear drawn in the pass too).
//! - occlusion count: the passes that add up an occlusion query's samples.
//! - resolves: multisampled targets resolved (the hardware's and the gamma-correct one).
//! - clears: passes that only clear the targets (a whole-target clear with no draw after it).
//! - copies: `StretchRect`'s blits, texture and buffer copies, uploads staged among the draws,
//!   read-backs.
//! - HDR: the bright-pixel watch's reads.
//! - sun shadow maps: the new lighting's sun shadow maps (all their depth passes).
//! - other: the rest of the new lighting (the scene light, the sun shadows' step), the frame-rate
//!   meter.
//!
//! Needs the adapter's timestamp queries (`TIMESTAMP_QUERY`; between passes also
//! `TIMESTAMP_QUERY_INSIDE_ENCODERS`, without which the copy commands, the HDR watch, the sun shadow
//! maps and other are not timed, and the line says so). Where the driver has none (some Metal and
//! older drivers), one line says so and nothing is timed. Off, the device asks for neither feature,
//! makes no query set and records nothing: no pass, query or command is added ([`GpuTimes`] is
//! never made). On, the timestamps and their resolve are no uses of the encoders (the submit
//! bound's count), so the times change neither when a batch is sent nor how it is cut into encoders.
//!
//! A pair that reads zero at either end means a driver that gives no times: the line then says so
//! once and shows no numbers. AMD's Windows driver does that (Radeon 890M, driver 24.30: some
//! timestamps read zero, raw wgpu too, a new query set's first ones always), so there the times
//! are not shown.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Whether `NV_WGPU_GPU_TIMES` asks for the times (read as a device is made).
pub(crate) fn enabled() -> bool {
    std::env::var("NV_WGPU_GPU_TIMES").is_ok_and(|v| v.trim() == "1")
}

/// The features the device asks for when the times are on (none when off).
pub(crate) fn wanted_features() -> wgpu::Features {
    if enabled() {
        wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
    } else {
        wgpu::Features::empty()
    }
}

/// A kind of the card's work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    World,
    Count,
    Resolves,
    Clears,
    Copies,
    Hdr,
    SunShadows,
    Other,
}

/// The kinds in the line's order, with their words.
pub const KINDS: [(Kind, &str); 8] = [
    (Kind::World, "world"),
    (Kind::Count, "occlusion count"),
    (Kind::Resolves, "resolves"),
    (Kind::Clears, "clears"),
    (Kind::Copies, "copies"),
    (Kind::Hdr, "HDR"),
    (Kind::SunShadows, "sun shadow maps"),
    (Kind::Other, "other"),
];

fn slot(kind: Kind) -> usize {
    KINDS.iter().position(|(k, _)| *k == kind).unwrap_or(0)
}

/// Timestamps one query set holds (wgpu's most).
const QUERIES: u32 = 4096;

/// Query sets made at most; a batch finding none free is not timed (counted).
const SETS: usize = 32;

/// How often the line is said.
const EVERY: Duration = Duration::from_secs(5);

/// One batch's timestamps: the query set, the buffers its values are resolved into and read from,
/// each pair's kind, and the read's state (0 on its way, 1 mapped, 2 failed).
struct Set {
    query: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    kinds: Vec<Kind>,
    state: Arc<AtomicU8>,
}

/// The times so far (tests, and the line).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Totals {
    /// Nanoseconds of each kind ([`KINDS`]' order).
    pub ns: [u64; KINDS.len()],
    /// Pairs read back of each kind.
    pub spans: [u64; KINDS.len()],
    /// Pairs read back, and pairs whose end came before their start (left out).
    pub pairs: u64,
    pub left_out: u64,
    /// Pairs that read zero at either end (a driver that gives no times).
    pub zeros: u64,
    /// Batches not timed (no free query set, or more spans than a set holds).
    pub untimed: u64,
    /// Query sets made.
    pub sets_made: u64,
}

/// The device's timing ([`enabled`]): made only when on and the adapter can.
pub(crate) struct GpuTimes {
    device: wgpu::Device,
    /// Nanoseconds a tick.
    period: f64,
    /// Whether timestamps can be written between passes.
    inside: bool,
    free: Vec<Set>,
    current: Option<Set>,
    flying: Vec<Set>,
    totals: Totals,
    /// What the last line covered from.
    said: Totals,
    frames: u64,
    since: Instant,
    /// The open batch asked for a span it did not get.
    missed: bool,
    /// Whether the line has said this driver gives no times.
    told: bool,
}

impl GpuTimes {
    /// The timing for `device`, or why not (said once by the caller).
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<GpuTimes, String> {
        let features = device.features();
        if !features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Err(
                "GPU times (NV_WGPU_GPU_TIMES): this graphics driver cannot time passes; nothing is timed"
                    .into(),
            );
        }
        Ok(GpuTimes {
            device: device.clone(),
            period: f64::from(queue.get_timestamp_period()),
            inside: features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
            free: Vec::new(),
            current: None,
            flying: Vec::new(),
            totals: Totals::default(),
            said: Totals::default(),
            frames: 0,
            since: Instant::now(),
            missed: false,
            told: false,
        })
    }

    /// Whether timestamps can be written between passes (copies, HDR, sun shadow maps, other).
    pub(crate) fn inside(&self) -> bool {
        self.inside
    }

    /// The open batch's set, made or taken from the free ones; none when all are in use.
    fn set(&mut self) -> Option<&mut Set> {
        if self.current.is_none() {
            let set = match self.free.pop() {
                Some(s) => s,
                None if self.free.len() + self.flying.len() < SETS => {
                    self.totals.sets_made += 1;
                    let bytes = u64::from(QUERIES) * 8;
                    Set {
                        query: self.device.create_query_set(&wgpu::QuerySetDescriptor {
                            label: Some("GPU times"),
                            ty: wgpu::QueryType::Timestamp,
                            count: QUERIES,
                        }),
                        resolve: self.device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("GPU times resolved"),
                            size: bytes,
                            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                            mapped_at_creation: false,
                        }),
                        read: self.device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("GPU times read"),
                            size: bytes,
                            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                            mapped_at_creation: false,
                        }),
                        kinds: Vec::new(),
                        state: Arc::new(AtomicU8::new(0)),
                    }
                }
                None => return None,
            };
            self.current = Some(set);
        }
        self.current.as_mut()
    }

    /// A pair of timestamps of `kind` in the open batch: the query set (a handle) and the pair's
    /// first index; none when this batch's set is full or none is free.
    pub(crate) fn span(&mut self, kind: Kind) -> Option<(wgpu::QuerySet, u32)> {
        let Some(set) = self.set() else {
            self.missed = true;
            return None;
        };
        let i = set.kinds.len() as u32 * 2;
        if i + 2 > QUERIES {
            self.missed = true;
            return None;
        }
        set.kinds.push(kind);
        Some((set.query.clone(), i))
    }

    /// The pair starting at `index` of the open batch is of `kind` after all (a pass that only
    /// cleared).
    pub(crate) fn set_kind(&mut self, index: u32, kind: Kind) {
        if let Some(k) = self
            .current
            .as_mut()
            .and_then(|s| s.kinds.get_mut(index as usize / 2))
        {
            *k = kind;
        }
    }

    /// Whether the open batch has timestamps to read.
    pub(crate) fn pending(&self) -> bool {
        self.current.as_ref().is_some_and(|s| !s.kinds.is_empty())
    }

    /// The open batch ends with nothing to read ([`GpuTimes::pending`] false).
    pub(crate) fn end_empty_batch(&mut self) {
        if std::mem::take(&mut self.missed) {
            self.totals.untimed += 1;
        }
        if let Some(set) = self.current.take() {
            self.free.push(set);
        }
    }

    /// The open set's timestamps (this batch's, and those of batches before it that ended with no
    /// encoder open) resolved and copied out at the end of the batch's last encoder (no pass open);
    /// the buffer to map and its state, once the batch is submitted ([`map`]).
    pub(crate) fn end_batch(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Option<(wgpu::Buffer, Arc<AtomicU8>)> {
        if std::mem::take(&mut self.missed) {
            self.totals.untimed += 1;
        }
        let set = self.current.take()?;
        if set.kinds.is_empty() {
            self.free.push(set);
            return None;
        }
        let n = set.kinds.len() as u32 * 2;
        encoder.resolve_query_set(&set.query, 0..n, &set.resolve, 0);
        encoder.copy_buffer_to_buffer(&set.resolve, 0, &set.read, 0, u64::from(n) * 8);
        set.state.store(0, Ordering::Relaxed);
        let out = (set.read.clone(), set.state.clone());
        self.flying.push(set);
        Some(out)
    }

    /// The batches' timestamps read so far added up (no waiting); on a `Present`, the line when
    /// [`EVERY`] has passed since the last.
    pub(crate) fn frame(&mut self) -> Option<String> {
        self.frames += 1;
        self.collect();
        if self.since.elapsed() < EVERY {
            return None;
        }
        self.since = Instant::now();
        let (t, s) = (self.totals, self.said);
        self.said = t;
        let frames = std::mem::take(&mut self.frames).max(1) as f64;
        report(&mut self.told, t, s, frames, self.inside)
    }

    /// The flying sets whose values are in, added up and their sets freed.
    pub(crate) fn collect(&mut self) {
        let _ = self.device.poll(wgpu::PollType::Poll);
        let mut i = 0;
        while i < self.flying.len() {
            match self.flying[i].state.load(Ordering::Acquire) {
                0 => i += 1,
                state => {
                    let mut set = self.flying.swap_remove(i);
                    if state == 1 {
                        self.add(&set);
                        set.read.unmap();
                    }
                    set.kinds.clear();
                    self.free.push(set);
                }
            }
        }
    }

    fn add(&mut self, set: &Set) {
        let n = set.kinds.len() * 2;
        let Ok(data) = set.read.slice(..(n as u64 * 8)).get_mapped_range() else {
            return;
        };
        let values: Vec<u64> = data
            .chunks_exact(8)
            .map(|v| u64::from_le_bytes(v.try_into().unwrap_or_default()))
            .collect();
        tally(&mut self.totals, self.period, &values, &set.kinds);
    }

    /// The times so far, the values in now added first (tests).
    pub(crate) fn totals(&mut self) -> Totals {
        self.collect();
        self.totals
    }
}

/// One batch's timestamps (`values`, a start and an end for each of `kinds`) added to `totals`:
/// a pair that reads zero at either end counts as a driver that gives no times (a zero start would
/// add the clock's whole count, a zero end nothing), one whose end comes before its start is left
/// out.
fn tally(totals: &mut Totals, period: f64, values: &[u64], kinds: &[Kind]) {
    for (pair, kind) in values.chunks_exact(2).zip(kinds) {
        let (start, end) = (pair[0], pair[1]);
        if start == 0 || end == 0 {
            totals.zeros += 1;
            continue;
        }
        if end < start {
            totals.left_out += 1;
            continue;
        }
        totals.pairs += 1;
        totals.spans[slot(*kind)] += 1;
        totals.ns[slot(*kind)] += ((end - start) as f64 * period) as u64;
    }
}

/// The line for the times from `s` to `t` over `frames` frames; once a pair has read zero at either
/// end, the plain words that this driver gives no times, said once (`told`), and no numbers.
fn report(told: &mut bool, t: Totals, s: Totals, frames: f64, inside: bool) -> Option<String> {
    if t.zeros > 0 {
        if std::mem::replace(told, true) {
            return None;
        }
        return Some(
            "NV_D3D9 wgpu: GPU times (NV_WGPU_GPU_TIMES): this graphics driver does not report GPU times (its timestamps read zero), so none are shown"
                .into(),
        );
    }
    let ms = |i: usize| (t.ns[i] - s.ns[i]) as f64 / 1e6 / frames;
    let total: f64 = (0..KINDS.len()).map(ms).sum();
    let parts: Vec<String> = KINDS
        .iter()
        .enumerate()
        .map(|(i, (_, words))| format!("{words} {:.2}", ms(i)))
        .collect();
    let mut line = format!(
        "NV_D3D9 wgpu: GPU time a frame ({frames} frames): {}; total {total:.2} ms",
        parts.join(", ")
    );
    if !inside {
        line.push_str(
            " (copy commands, the HDR watch, the sun shadow maps and other not timed: this driver times passes only)",
        );
    }
    if t.untimed > s.untimed {
        line.push_str(&format!("; {} batches not timed", t.untimed - s.untimed));
    }
    Some(line)
}

/// Maps `read` once the batch that resolved into it is submitted (the caller runs this after the
/// batch's submit, in queue order); `state` says when it is in.
pub(crate) fn map(read: wgpu::Buffer, state: Arc<AtomicU8>) {
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        state.store(if r.is_ok() { 1 } else { 2 }, Ordering::Release);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The line's words, in the order players read them.
    /// A driver whose timestamps read zero (fake answers here) is said once in plain words, and no
    /// numbers are shown after it; a good answer gives each kind's time a frame.
    #[test]
    fn a_driver_whose_times_read_zero_is_said_once_with_no_numbers() {
        let mut t = Totals::default();
        tally(
            &mut t,
            1.0,
            &[1_000_000, 3_000_000, 5, 6],
            &[Kind::World, Kind::Count],
        );
        assert_eq!((t.pairs, t.zeros), (2, 0));
        let mut told = false;
        let line = report(&mut told, t, Totals::default(), 2.0, true).expect("a line");
        assert!(line.contains("world 1.00, occlusion count 0.00"), "{line}");
        tally(
            &mut t,
            1.0,
            &[0, 0, 7, 9, 4, 2],
            &[Kind::World, Kind::Copies, Kind::Hdr],
        );
        assert_eq!((t.zeros, t.pairs, t.left_out), (1, 3, 1));
        let line = report(&mut told, t, Totals::default(), 2.0, true).expect("said");
        assert!(line.contains("does not report GPU times"), "{line}");
        assert!(!line.contains("world"), "no numbers: {line}");
        assert_eq!(report(&mut told, t, t, 2.0, true), None, "said once");
        // A zero at either end alone is a driver zero too, and adds no time.
        for pair in [[0, 5_000_000], [5_000_000, 0]] {
            let mut t = Totals::default();
            tally(&mut t, 1.0, &pair, &[Kind::World]);
            assert_eq!((t.zeros, t.pairs, t.left_out), (1, 0, 0), "{pair:?}");
            assert_eq!(t.ns, [0; KINDS.len()], "{pair:?}");
            let mut told = false;
            let line = report(&mut told, t, Totals::default(), 1.0, true).expect("said");
            assert!(
                line.contains("does not report GPU times"),
                "{pair:?}: {line}"
            );
        }
    }

    /// A driver that times passes only says which kinds it leaves out, in plain words.
    #[test]
    fn a_driver_that_times_passes_only_says_what_is_not_timed() {
        let mut t = Totals::default();
        tally(&mut t, 1.0, &[1_000_000, 2_000_000], &[Kind::World]);
        let mut told = false;
        let line = report(&mut told, t, Totals::default(), 1.0, false).expect("a line");
        assert!(line.contains("world 1.00"), "{line}");
        assert!(
            line.contains("copy commands, the HDR watch, the sun shadow maps and other not timed"),
            "{line}"
        );
        let line = report(&mut told, t, Totals::default(), 1.0, true).expect("a line");
        assert!(!line.contains("not timed"), "{line}");
    }

    #[test]
    fn the_kinds_have_plain_words() {
        let words: Vec<&str> = KINDS.iter().map(|(_, w)| *w).collect();
        assert_eq!(
            words,
            [
                "world",
                "occlusion count",
                "resolves",
                "clears",
                "copies",
                "HDR",
                "sun shadow maps",
                "other"
            ]
        );
        assert_eq!(slot(Kind::Hdr), 5);
        assert_eq!(slot(Kind::SunShadows), 6);
    }
}
