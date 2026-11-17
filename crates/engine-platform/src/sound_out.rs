//! The sound the platform's mixer makes, played on the player's default output device through
//! WASAPI in shared mode, on a host thread of its own per stream (the game never sees those
//! threads). The endpoint's mix format gives the rate and the speaker layout ([`Layout`]: its
//! channels and speaker mask; on this machine 8 channels, mask 0x63F, for a program named
//! `FalloutNV.exe` and 2 channels, mask 0x3, for any other name, measured). A stream is opened
//! either in that format as it is, or in another layout at the same rate, 32-bit float, the way
//! DirectSound opens its two-channel stream on a larger endpoint: `IsFormatSupported` first, then
//! `Initialize` with that format (measured as `FalloutNV.exe`, 2026-11-03: `S_OK`). A mix format
//! that is not 32-bit float, or a layout DirectSound does not place 3D buffers on
//! (`super::speakers::Layout::provided`), is not provided: [`start`] then refuses, named, and
//! nothing is heard.
//!
//! The thread asks the [`Source`] for as many frames as the device can take each time the device
//! signals it (event driven, the device's period), so what plays is at most one device buffer
//! behind what the mixer was asked for.
//!
//! **Linux and macOS** (`mod device`, through cpal: ALSA on Linux, which PipeWire and PulseAudio
//! serve through their ALSA plugins; CoreAudio's default output unit on macOS). The mixer is the
//! same; only the device differs, and nothing of it is measured against Windows: the same mixed
//! samples are handed to the system, and what the system does after is not measured. Nothing here
//! has been run on Linux or verified on a Mac.
//! - Rate: the device's default stream as cpal gives it ([`mix_of`]). On Linux that is cpal's
//!   preference among the rates ALSA lists (48000 Hz first), not necessarily the rate PipeWire,
//!   PulseAudio or the hardware runs at: they may convert it after us. On macOS it is the
//!   device's own nominal rate.
//! - Channels: these systems name no speaker positions through cpal (and ALSA orders a 5.1
//!   device's channels otherwise than Windows does), so a device of one or two channels is mixed
//!   for as it is (front centre; front left and right: Windows' rule for a mix format without a
//!   speaker mask), and a larger one is mixed for in two channels; what the system does with them
//!   (the front pair only, or an upmix) is not measured: no surround placement off Windows. Only
//!   32-bit float streams are opened. Linux asks the device's list first (ALSA would otherwise
//!   open the nearest rate without saying so) and opens the mix's own channels. macOS opens the
//!   device's own offered channel count at its own nominal rate ([`stream_channels`]), the mix in
//!   the first channels and silence in the rest ([`widen`]): cpal sets the device's physical
//!   format before it opens a stream (`set_physical_format`, coreaudio-rs's
//!   `find_matching_physical_format`), and asking for what the device already offers is the
//!   request least able to change the hardware for every other program. It can still change it
//!   (a device whose current hardware format is integer but which also lists a 32-bit float one
//!   of as many channels; a device whose list holds a larger float format first): not verified on
//!   a Mac. Another layout than the mix is not asked for on macOS ([`open_as`]: DirectSound's
//!   device then locks to the mix, as on Windows when the endpoint refuses a layout).
//! - Period: on Linux DirectSound's 10 ms where the device's period sizes include it, else the
//!   system's own ([`fixed_period`]); on macOS CoreAudio's own IO buffer. The stream's line on
//!   stderr says the device's name, the rate and channels asked, and the period the system
//!   actually opened ([`opened_line`]); a 10 ms request that failed says why.
//! - Thread: the device calls back on the system's thread ([`period`], [`fill`]). On Linux that is
//!   cpal's ALSA worker at normal priority, said once per run ("sound thread: normal priority"):
//!   no real-time attempt, because cpal's worker retries errors at once, so a device left in a bad
//!   state would spin at real-time priority and take a core from the desktop (cpal itself refuses
//!   real-time for the PipeWire and PulseAudio plugin devices most desktops use); a later step may
//!   add it for hardware devices only, with a way back. On macOS it is CoreAudio's IO thread, which
//!   the system runs at real-time priority. Windows: a thread of this module's at time-critical
//!   priority and MMCSS "Audio".
//! - Reports ([`Watch`], [`react`]): ALSA's xruns are counted as underruns once frames were given;
//!   CoreAudio's processor overloads are counted under their own name; a device that stops
//!   answering is named once and the stream is then silent and not pulled (as the WASAPI arm
//!   does); a changed default device is said and the stream plays on (cpal 0.18.2: CoreAudio's
//!   default output unit has already moved it; ALSA reports none). Lines said "once" are once per
//!   run ([`Said`]).
//! - Linux needs ALSA's library (`libasound.so.2`) to start at all, sound or not (cpal links it).

pub use super::speakers::Layout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// What the output thread plays: frames at the device's rate and channel count.
pub trait Source: Send {
    /// Fills `out` (interleaved, `channels` per frame) with the next frames.
    fn render(&mut self, rate: u32, channels: u16, out: &mut [f32]);
    /// The device had played everything it was given when the thread woke (an underrun: what
    /// it plays next comes late): the `count`th of this stream.
    fn starved(&mut self, count: u64) {
        let _ = count;
    }
}

/// A running output: the thread and its stop flag.
pub struct Output {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// The stream's rate, and its channels and speakers.
    pub rate: u32,
    pub layout: Layout,
}

/// Why [`start`] opened no stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// No default output device (or it would not open).
    NoDevice,
    /// The endpoint's mix format is not provided (named).
    Format(String),
    /// The endpoint does not take the layout asked for (`IsFormatSupported` other than `S_OK`).
    Layout,
}

impl Output {
    /// A stream with no device and no thread, as an endpoint opened it: the tests', and
    /// `NV_REPEATABLE`'s, whose frames are asked for on the virtual clock
    /// (`super::dsound::repeatable_pull`).
    pub fn idle(rate: u32, layout: Layout) -> Output {
        Output {
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
            rate,
            layout,
        }
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            // Off Windows the thread holding the stream waits parked ([`hold`]).
            #[cfg(not(windows))]
            release(&self.stop, t.thread());
            let _ = t.join();
        }
    }
}

/// The default output device's mix rate (its shared-mode format, the rate DirectSound mixes at),
/// asked once on a thread of its own; `None` when there is no device.
pub fn device_rate() -> Option<u32> {
    device_mix().map(|(rate, _)| rate)
}

/// The default output device's mix rate and layout (its shared-mode format; off Windows
/// [`mix_of`] its default stream), asked once on a thread of its own; `None` when there is no
/// device (off Windows also when its stream is not provided: the reason is said once).
pub fn device_mix() -> Option<(u32, Layout)> {
    #[cfg(windows)]
    {
        static MIX: std::sync::OnceLock<Option<(u32, Layout)>> = std::sync::OnceLock::new();
        *MIX.get_or_init(|| {
            std::thread::spawn(|| unsafe { wasapi::mix_format() })
                .join()
                .ok()
                .flatten()
        })
    }
    #[cfg(not(windows))]
    {
        static MIX: std::sync::OnceLock<Option<(u32, Layout)>> = std::sync::OnceLock::new();
        *MIX.get_or_init(|| std::thread::spawn(device::mix).join().ok().flatten())
    }
}

/// The system off Windows whose rules the stream follows (cpal's backend there).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    /// ALSA (PipeWire's and PulseAudio's ALSA plugins among its devices).
    Linux,
    /// CoreAudio's default output unit (macOS).
    Apple,
}

/// This build's [`System`]: macOS's rules there, ALSA's on every other system off Windows.
pub const SYSTEM: System = if cfg!(target_os = "macos") {
    System::Apple
} else {
    System::Linux
};

/// What a device off Windows offers by default (cpal's default output config): its rate and
/// channel count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    pub rate: u32,
    pub channels: u16,
}

/// One entry of a device's list of streams (cpal's `supported_output_configs`): 32-bit float or
/// not, the channel count, the rates, and the period sizes (frames) when the system says them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub f32: bool,
    pub channels: u16,
    pub min_rate: u32,
    pub max_rate: u32,
    pub period: Option<(u32, u32)>,
}

/// The entry of `ranges` that opens a 32-bit float stream of `channels` at `rate`, if any.
pub fn range_for(ranges: &[Range], rate: u32, channels: u16) -> Option<&Range> {
    ranges
        .iter()
        .find(|r| r.f32 && r.channels == channels && (r.min_rate..=r.max_rate).contains(&rate))
}

/// Whether the list is asked before a stream is opened. Linux: yes, because cpal's ALSA arm sets
/// the rate with ALSA's "nearest" (`snd_pcm_hw_params_set_rate_near`), so a device without the
/// rate would open at another one with no error and play at the wrong speed; the list (ALSA's
/// own hardware ranges) proves the exact rate and channel count first. macOS: no, because cpal's
/// list names a single channel count (the sum over all the device's output streams), never two
/// for a 6- or 8-channel device; the stream asked for there is the device's own offer
/// ([`stream_channels`]), and only a refusal counts.
fn asks_list(system: System) -> bool {
    system == System::Linux
}

/// The mix rate and layout off Windows for a device whose default stream is `offer` and whose
/// list is `ranges`: the offer's rate; one or two channels as offered, with Windows' layout for
/// a mix format without a speaker mask ([`Layout::of_mix`]); more channels, whose speakers the
/// system does not name here, as two (what the system does with them is not measured). On Linux
/// the list must hold that stream in 32-bit float ([`asks_list`]). The reason, for stderr, when
/// there is none.
pub fn mix_of(offer: Offer, ranges: &[Range], system: System) -> Result<(u32, Layout), String> {
    let Offer { rate, channels } = offer;
    let channels_used = match channels {
        0 => return Err("the sound device offers a stream of no channels".to_string()),
        1 | 2 => channels,
        _ => 2,
    };
    if asks_list(system) && range_for(ranges, rate, channels_used).is_none() {
        return Err(if channels_used == channels {
            format!(
                "the sound device takes no 32-bit float stream of {channels} channels at {rate} \
                 Hz: not provided (32-bit float only)"
            )
        } else {
            format!(
                "the sound device mixes {channels} channels, whose speakers this system does not \
                 name, and takes no 32-bit float stream of two at {rate} Hz: not provided"
            )
        });
    }
    Ok((rate, Layout::of_mix(channels_used, None)))
}

/// The stream opened off Windows on a device that mixes `mix`, for the layout `want` (as the
/// WASAPI arm: the mix as it is for `None` or the mix's own layout; another layout at the mix
/// rate when the device's list holds it, else [`Refused::Layout`]); a mix layout DirectSound does
/// not place sound on is [`Refused::Format`]. On macOS another layout is never asked for (it
/// would be a request other than the device's own offer, [`stream_channels`]): always
/// [`Refused::Layout`], so DirectSound's device locks to the mix.
pub fn open_as(
    mix: (u32, Layout),
    want: Option<Layout>,
    ranges: &[Range],
    system: System,
) -> Result<(u32, Layout), Refused> {
    let (rate, mix) = mix;
    if !mix.provided() {
        return Err(Refused::Format(not_provided(mix)));
    }
    match want {
        Some(l) if l != mix => {
            if asks_list(system) && range_for(ranges, rate, l.channels).is_some() {
                Ok((rate, l))
            } else {
                Err(Refused::Layout)
            }
        }
        _ => Ok((rate, mix)),
    }
}

/// The channel count of the stream asked of the device for a mix `layout` on a device that offered
/// `offer`: on Linux the layout's own; on macOS the device's own offered count (never fewer than
/// the layout's), so that cpal, which sets the device's physical format to the first float format
/// of at least that many channels before it opens the stream, finds the format the device is
/// already in where it can. The mix goes in the first channels ([`widen`]).
pub fn stream_channels(offer: Offer, layout: Layout, system: System) -> u16 {
    match system {
        System::Linux => layout.channels,
        System::Apple => offer.channels.max(layout.channels),
    }
}

/// Copies `mix` (interleaved, `mix_channels` per frame) into `out` (interleaved, `out_channels`
/// per frame, at least as many): each frame's mix in its first channels, silence in the rest.
/// Frames beyond either buffer are left silent.
pub fn widen(mix: &[f32], mix_channels: u16, out: &mut [f32], out_channels: u16) {
    let (m, o) = (usize::from(mix_channels), usize::from(out_channels));
    out.fill(0.0);
    if m == 0 || o < m {
        return;
    }
    for (src, dst) in mix.chunks_exact(m).zip(out.chunks_exact_mut(o)) {
        dst[..m].copy_from_slice(src);
    }
}

/// What is asked of the device off Windows for the layout `want`: the mix (rate and layout), the
/// stream's layout, its channel count ([`stream_channels`]) and the period asked (Linux:
/// [`fixed_period`] from the list entry of that stream; macOS: the system's own).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub mix: (u32, Layout),
    pub rate: u32,
    pub layout: Layout,
    pub channels: u16,
    pub period: Option<u32>,
}

/// The [`Plan`] for a device that offered `offer` with the list `ranges`, or why there is none
/// ([`mix_of`], [`open_as`]).
pub fn plan(
    offer: Offer,
    ranges: &[Range],
    system: System,
    want: Option<Layout>,
) -> Result<Plan, Refused> {
    let mix = mix_of(offer, ranges, system).map_err(Refused::Format)?;
    let (rate, layout) = open_as(mix, want, ranges, system)?;
    let channels = stream_channels(offer, layout, system);
    let period = match system {
        System::Linux => fixed_period(
            rate,
            range_for(ranges, rate, channels).and_then(|r| r.period),
        ),
        System::Apple => None,
    };
    Ok(Plan {
        mix,
        rate,
        layout,
        channels,
        period,
    })
}

/// What the system answered when the stream was opened or ran, by kind (cpal's `ErrorKind`,
/// those this module tells apart).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An underrun (ALSA's xrun) or, on macOS, a processor overload.
    Xrun,
    /// The device is gone.
    DeviceNotAvailable,
    /// The stream must be built again.
    StreamInvalidated,
    /// The default device changed and the system moved the stream to it, still playing.
    DeviceChanged,
    /// The stream asked for is not taken.
    UnsupportedConfig,
    /// Anything else.
    Other,
}

/// Why a stream of `asked` was not opened on a device that mixes `mix` (it offered `offer`), from
/// the system's answer: another layout than the mix refused is [`Refused::Layout`] (DirectSound's
/// device then locks to the mix and asks again); the mix itself refused is not provided, named
/// with the device's offer; anything else is no device.
pub fn refused(kind: Kind, asked: Layout, mix: (u32, Layout), offer: Offer) -> Refused {
    match kind {
        Kind::UnsupportedConfig if asked != mix.1 => Refused::Layout,
        Kind::UnsupportedConfig => Refused::Format(format!(
            "the sound device offers {} channels at {} Hz and refused a 32-bit float stream of {} \
             channels at {} Hz",
            offer.channels, offer.rate, asked.channels, mix.0
        )),
        _ => Refused::NoDevice,
    }
}

/// The period asked of the device off Windows: DirectSound's mixer period (10 ms: `rate / 100`
/// frames) where the device's period sizes include it, else the system's own (`None`).
pub fn fixed_period(rate: u32, period: Option<(u32, u32)>) -> Option<u32> {
    let frames = rate / 100;
    period
        .filter(|&(min, max)| (min..=max).contains(&frames))
        .map(|_| frames)
}

/// How the stream's period came about off Windows: asked (`asked`, DirectSound's 10 ms, or the
/// system's own when `None`), why a 10 ms request failed and the system's own was opened instead
/// (`fell_back`), and what the system actually opened (`opened`, its period in frames, when it
/// says).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Periods {
    pub asked: Option<u32>,
    pub fell_back: Option<String>,
    pub opened: Option<u32>,
}

/// The line said when the stream opened off Windows: the device's name, the rate and channels
/// asked (and the mix's channels when fewer), and the period opened.
pub fn opened_line(
    system: System,
    name: &str,
    rate: u32,
    channels: u16,
    mix_channels: u16,
    periods: &Periods,
) -> String {
    let opened = match periods.opened {
        Some(frames) => format!("a period of {frames} frames opened"),
        None => "the period not said by the system".to_string(),
    };
    let asked = match (&periods.asked, &periods.fell_back) {
        (Some(n), None) => format!(" ({n} frames asked: 10 ms, DirectSound's mixer period)"),
        (Some(n), Some(why)) => {
            format!(" (the system's own: {n} frames, 10 ms, were refused: {why})")
        }
        (None, _) => " (the system's own)".to_string(),
    };
    let mix = if mix_channels < channels {
        format!(
            "; the mix is in the first {mix_channels} channels, the other {} silent",
            channels - mix_channels
        )
    } else {
        String::new()
    };
    let after = match system {
        System::Linux => {
            "the rate is the one cpal prefers for this device, not necessarily the system's own, \
             and PipeWire, PulseAudio or ALSA's plug may convert it"
        }
        System::Apple => {
            "these are the device's own offered channels and nominal rate, the request least \
             able to change its hardware format (not verified on a Mac)"
        }
    };
    format!(
        "sound: the stream is open on \"{name}\": {rate} Hz, {channels} channels asked{mix}, \
         {opened}{asked}. The same mixed samples are handed to the system; {after}; what the \
         system does after is not measured"
    )
}

/// One period for the device off Windows: `out` (interleaved, `channels` per frame) zeroed, the
/// mixer's frames as it renders them, or silence when the device is muted ([`mute_device`]).
pub fn fill(source: &Mutex<dyn Source>, rate: u32, channels: u16, out: &mut [f32], muted: bool) {
    out.fill(0.0);
    if let Ok(mut s) = source.lock() {
        s.render(rate, channels, out);
    }
    if muted {
        out.fill(0.0);
    }
}

/// A stream's state shared by the system's two callbacks off Windows: whether the device was
/// given frames yet, and whether it is lost (from then on silence, the mixer not pulled).
#[derive(Debug, Default)]
pub struct Gate {
    pub given: AtomicBool,
    pub lost: AtomicBool,
}

/// The device's call for one period off Windows (`out` of `out_channels` per frame): silence
/// once the stream is lost; else the mix of `mix_channels` per frame ([`fill`]), straight into
/// `out` when the counts match, else through `scratch` into the first channels ([`widen`]); from
/// then on the device counts as given frames. `scratch` grows only when a period is longer than
/// any before.
#[allow(clippy::too_many_arguments)]
pub fn period(
    gate: &Gate,
    source: &Mutex<dyn Source>,
    rate: u32,
    mix_channels: u16,
    out: &mut [f32],
    out_channels: u16,
    scratch: &mut Vec<f32>,
    muted: bool,
) {
    if gate.lost.load(Ordering::Relaxed) {
        out.fill(0.0);
        return;
    }
    if mix_channels == out_channels || out_channels == 0 {
        fill(source, rate, mix_channels, out, muted);
    } else {
        let frames = out.len() / usize::from(out_channels);
        let n = frames * usize::from(mix_channels);
        if scratch.len() < n {
            scratch.resize(n, 0.0);
        }
        fill(source, rate, mix_channels, &mut scratch[..n], muted);
        widen(&scratch[..n], mix_channels, out, out_channels);
    }
    gate.given.store(true, Ordering::Relaxed);
}

/// What the system says about a running stream off Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The device ran out of frames (an underrun: ALSA's xrun).
    Underrun,
    /// macOS: CoreAudio's processing was overloaded (`kAudioDeviceProcessorOverload`).
    Overload,
    /// The device stopped answering (unplugged, disabled; the stream is gone).
    Lost(String),
    /// The default device changed and the system moved the stream to it (still playing).
    Changed,
    /// Anything else, named.
    Other(String),
}

/// The [`Event`] of a system answer of `kind` (its text `why`) on `system`.
pub fn event(kind: Kind, why: String, system: System) -> Event {
    match kind {
        Kind::Xrun if system == System::Apple => Event::Overload,
        Kind::Xrun => Event::Underrun,
        Kind::DeviceNotAvailable | Kind::StreamInvalidated => Event::Lost(why),
        Kind::DeviceChanged => Event::Changed,
        Kind::UnsupportedConfig | Kind::Other => Event::Other(why),
    }
}

/// What [`Watch::on`] makes of an [`Event`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reaction {
    /// Nothing to do.
    None,
    /// Tell the source of its `n`th underrun ([`Source::starved`]).
    Starved(u64),
    /// Say this on stderr.
    Say(String),
    /// Say this, and from now on give the device silence without pulling the mixer (the WASAPI
    /// arm's rule for a device that stopped answering).
    Lose(String),
}

/// What was said once in this run, whichever stream said it (the device module keeps one for the
/// whole run; each test its own).
#[derive(Debug, Default)]
pub struct Said {
    /// Another report of the system's ([`Event::Other`]).
    pub other: AtomicBool,
    /// The Linux sound thread's priority ([`thread_line`]).
    pub thread: AtomicBool,
}

impl Said {
    /// Whether `flag` is said now: true the first time in the run only.
    fn first(flag: &AtomicBool) -> bool {
        !flag.swap(true, Ordering::Relaxed)
    }
}

/// The Linux sound thread's line, the first time in the run (`None` after, and on macOS, where
/// CoreAudio's IO thread is the system's own).
pub fn thread_line(system: System, said: &Said) -> Option<String> {
    (system == System::Linux && Said::first(&said.thread))
        .then(|| "sound thread: normal priority (no real-time attempt off Windows yet)".to_string())
}

/// A stream's events off Windows: underruns counted once the first frames were given (the stream
/// starts empty) and until the device is lost; macOS's overloads counted under their own name
/// (the 1st, 10th, 100th ... of the stream said with the count); the device lost said once, after
/// which nothing more is said of it; a changed default device said each time, the stream playing
/// on; other reports said once per run ([`Said`]).
#[derive(Debug, Default)]
pub struct Watch {
    starved: u64,
    overloads: u64,
    lost: bool,
}

impl Watch {
    /// The reaction to `e`, after the device was (`given`) or was not yet given frames.
    pub fn on(&mut self, e: Event, given: bool, said: &Said) -> Reaction {
        match e {
            Event::Underrun if given && !self.lost => {
                self.starved += 1;
                Reaction::Starved(self.starved)
            }
            Event::Underrun => Reaction::None,
            Event::Overload if self.lost => Reaction::None,
            Event::Overload => {
                self.overloads += 1;
                let n = self.overloads;
                if n == 10u64.pow(n.ilog10()) {
                    Reaction::Say(format!(
                        "sound: CoreAudio reported its sound processing overloaded ({n} so far; a \
                         skip may be heard)"
                    ))
                } else {
                    Reaction::None
                }
            }
            Event::Lost(_) | Event::Changed if self.lost => Reaction::None,
            Event::Lost(why) => {
                self.lost = true;
                Reaction::Lose(format!(
                    "sound: the sound device stopped answering ({why}: unplugged or disabled?); \
                     this stream is not heard any more"
                ))
            }
            Event::Changed => Reaction::Say(
                "sound: the default sound device changed; the system moved this stream to it and \
                 it plays on"
                    .to_string(),
            ),
            Event::Other(why) if Said::first(&said.other) => Reaction::Say(format!(
                "sound: the sound device reports: {why} (further reports not shown)"
            )),
            Event::Other(_) => Reaction::None,
        }
    }
}

/// Carries out a [`Reaction`] for the stream of `gate`: an underrun told to `source`, a lost
/// device marked in `gate`; the line to say, if any.
pub fn react(r: Reaction, gate: &Gate, source: &Mutex<dyn Source>) -> Option<String> {
    match r {
        Reaction::None => None,
        Reaction::Starved(n) => {
            if let Ok(mut s) = source.lock() {
                s.starved(n);
            }
            None
        }
        Reaction::Say(line) => Some(line),
        Reaction::Lose(line) => {
            gate.lost.store(true, Ordering::Relaxed);
            Some(line)
        }
    }
}

/// The thread holding a stream off Windows: waits parked until `stop` is set (each wait at most
/// `wait`, a safety net; [`release`] wakes it at once).
pub fn hold(stop: &AtomicBool, wait: std::time::Duration) {
    while !stop.load(Ordering::Relaxed) {
        std::thread::park_timeout(wait);
    }
}

/// Ends [`hold`] on `thread`.
pub fn release(stop: &AtomicBool, thread: &std::thread::Thread) {
    stop.store(true, Ordering::Relaxed);
    thread.unpark();
}

/// The layout of a mix format (`WAVEFORMATEX` bytes, and what follows them): a
/// `WAVEFORMATEXTENSIBLE`'s (tag 0xFFFE, at least 22 more bytes) channel mask, else DirectSound's
/// rule (`super::speakers::Layout::of_mix`).
pub fn layout_of(format: &[u8]) -> Option<Layout> {
    let w16 = |o: usize| {
        format
            .get(o..o + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let (tag, channels) = (w16(0)?, w16(2)?);
    let mask = if tag == 0xfffe && w16(16)? >= 22 {
        let b = format.get(20..24)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    } else {
        None
    };
    Some(Layout::of_mix(channels, mask))
}

/// Why a mix layout is not provided, for stderr.
pub fn not_provided(l: Layout) -> String {
    if l.mask == 0 {
        // `10056ab3`: on a device with mask 0 a buffer must have the device's channels and may
        // not have `DSBCAPS_CTRL3D` or `DSBCAPS_CTRLPAN`; every buffer the game makes has one or
        // the other (`00aeb270` in the exe: 0x40 or 0x30010, each with 0x2a0).
        return format!(
            "the sound device mixes {} channels with speaker mask 0 (direct out): DirectSound \
             refuses every buffer the game makes there (each is 3D or panned), so the original \
             is silent on it too",
            l.channels
        );
    }
    format!(
        "the sound device mixes {} channels, speaker mask {:#x}: not provided (DirectSound places \
         3D sound only on mono, stereo, quad, surround, 5.1 and 7.1 layouts)",
        l.channels, l.mask
    )
}

/// Whether the device is fed silence instead of the mix ([`mute_device`]).
static MUTED: AtomicBool = AtomicBool::new(false);

/// From now on the device is fed silence while the mix is still made (and recorded, with
/// `NV_SOUND_RECORD`): a test run whose window is never shown never plays on the speakers.
pub fn mute_device() {
    MUTED.store(true, Ordering::Relaxed);
}

/// Starts playing `source` on the default output device, in its mix format as it is (`want`
/// `None`, or the mix format's own layout) or in the layout `want` at the mix rate, 32-bit float
/// (when the endpoint takes it).
pub fn start(source: Arc<Mutex<dyn Source>>, want: Option<Layout>) -> Result<Output, Refused> {
    #[cfg(windows)]
    {
        wasapi::start(source, want)
    }
    #[cfg(not(windows))]
    {
        device::start(source, want)
    }
}

/// The `WAVEFORMATEXTENSIBLE` of 32-bit float at `rate` in `layout`, as DirectSound gives
/// `IsFormatSupported` and `Initialize` (measured: `feff0200 80bb0000 00dc0500 0800 2000 1600
/// 2000 03000000` and the float subformat for two channels at 48000 Hz).
pub fn float_format(rate: u32, layout: Layout) -> [u8; 40] {
    let block = 4 * layout.channels;
    let mut f = [0u8; 40];
    f[0..2].copy_from_slice(&0xfffeu16.to_le_bytes());
    f[2..4].copy_from_slice(&layout.channels.to_le_bytes());
    f[4..8].copy_from_slice(&rate.to_le_bytes());
    f[8..12].copy_from_slice(&(rate * u32::from(block)).to_le_bytes());
    f[12..14].copy_from_slice(&block.to_le_bytes());
    f[14..16].copy_from_slice(&32u16.to_le_bytes());
    f[16..18].copy_from_slice(&22u16.to_le_bytes());
    f[18..20].copy_from_slice(&32u16.to_le_bytes());
    f[20..24].copy_from_slice(&layout.mask.to_le_bytes());
    // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT {00000003-0000-0010-8000-00aa00389b71}.
    f[24..40].copy_from_slice(&[
        3, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71,
    ]);
    f
}

/// The default output device off Windows, through cpal ([`mix_of`], [`open_as`], [`refused`],
/// [`period`], [`Watch`] and [`react`] decide; this only asks the system and hands over).
#[cfg(not(windows))]
mod device {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    /// cpal's list entry as [`Range`].
    pub(super) fn range_of(r: &cpal::SupportedStreamConfigRange) -> Range {
        Range {
            f32: r.sample_format() == cpal::SampleFormat::F32,
            channels: r.channels(),
            min_rate: r.min_sample_rate(),
            max_rate: r.max_sample_rate(),
            period: match r.buffer_size() {
                cpal::SupportedBufferSize::Range { min, max } => Some((*min, *max)),
                _ => None,
            },
        }
    }

    /// cpal's error kind as [`Kind`].
    pub(super) fn kind_of(k: cpal::ErrorKind) -> Kind {
        use cpal::ErrorKind as K;
        match k {
            K::Xrun => Kind::Xrun,
            K::DeviceNotAvailable => Kind::DeviceNotAvailable,
            K::StreamInvalidated => Kind::StreamInvalidated,
            K::DeviceChanged => Kind::DeviceChanged,
            K::UnsupportedConfig => Kind::UnsupportedConfig,
            _ => Kind::Other,
        }
    }

    /// The device's list of streams (empty when the system gives none).
    fn ranges(device: &cpal::Device) -> Vec<Range> {
        device
            .supported_output_configs()
            .map(|c| c.map(|r| range_of(&r)).collect())
            .unwrap_or_default()
    }

    /// The default device and its offer.
    fn default_device() -> Option<(cpal::Device, Offer)> {
        let device = cpal::default_host().default_output_device()?;
        let c = device.default_output_config().ok()?;
        let offer = Offer {
            rate: c.sample_rate(),
            channels: c.channels(),
        };
        Some((device, offer))
    }

    /// [`super::device_mix`]'s answer.
    pub fn mix() -> Option<(u32, Layout)> {
        let (device, offer) = default_device()?;
        match mix_of(offer, &ranges(&device), SYSTEM) {
            Ok(m) => Some(m),
            Err(why) => {
                eprintln!("sound: {why}");
                None
            }
        }
    }

    /// What this run has said once, whichever stream said it.
    static SAID: Said = Said {
        other: AtomicBool::new(false),
        thread: AtomicBool::new(false),
    };

    /// Builds the stream (`channels` per frame, the mix's `mix_channels` in the first; a period of
    /// `period_frames`, or the system's own when `None`) and starts it.
    fn build(
        device: &cpal::Device,
        source: &Arc<Mutex<dyn Source>>,
        rate: u32,
        mix_channels: u16,
        channels: u16,
        period_frames: Option<u32>,
    ) -> Result<cpal::Stream, cpal::Error> {
        let gate = Arc::new(Gate::default());
        let mut watch = Watch::default();
        let (gate_cb, source_cb) = (gate.clone(), source.clone());
        let mut scratch: Vec<f32> = Vec::new();
        let data = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
            period(
                &gate_cb,
                &*source_cb,
                rate,
                mix_channels,
                out,
                channels,
                &mut scratch,
                MUTED.load(Ordering::Relaxed),
            );
        };
        let source_err = source.clone();
        let error = move |e: cpal::Error| {
            let ev = event(kind_of(e.kind()), e.to_string(), SYSTEM);
            let r = watch.on(ev, gate.given.load(Ordering::Relaxed), &SAID);
            if let Some(line) = react(r, &gate, &*source_err) {
                eprintln!("{line}");
            }
        };
        let config = cpal::StreamConfig {
            channels,
            sample_rate: rate,
            buffer_size: match period_frames {
                Some(n) => cpal::BufferSize::Fixed(n),
                None => cpal::BufferSize::Default,
            },
        };
        let stream = device.build_output_stream::<f32, _, _>(config, data, error, None)?;
        stream.play()?;
        Ok(stream)
    }

    /// Opens the stream and starts it; the stream lives on the calling thread.
    fn open(
        source: Arc<Mutex<dyn Source>>,
        want: Option<Layout>,
    ) -> Result<(cpal::Stream, u32, Layout), Refused> {
        let (device, offer) = default_device().ok_or(Refused::NoDevice)?;
        let p = plan(offer, &ranges(&device), SYSTEM, want)?;
        let mix_channels = p.layout.channels;
        let mut periods = Periods {
            asked: p.period,
            ..Periods::default()
        };
        let built = match p.period {
            Some(n) => match build(&device, &source, p.rate, mix_channels, p.channels, Some(n)) {
                Ok(s) => Ok(s),
                Err(e) => {
                    periods.fell_back = Some(e.to_string());
                    build(&device, &source, p.rate, mix_channels, p.channels, None)
                }
            },
            None => build(&device, &source, p.rate, mix_channels, p.channels, None),
        };
        let stream = built.map_err(|e| refused(kind_of(e.kind()), p.layout, p.mix, offer))?;
        periods.opened = stream.buffer_size().ok();
        let name = device
            .description()
            .map(|d| d.to_string())
            .unwrap_or_else(|_| "the default device (no name given)".to_string());
        eprintln!(
            "{}",
            opened_line(SYSTEM, &name, p.rate, p.channels, mix_channels, &periods)
        );
        if let Some(line) = thread_line(SYSTEM, &SAID) {
            eprintln!("{line}");
        }
        Ok((stream, p.rate, p.layout))
    }

    /// [`super::start`]: the stream opened on a thread of its own, which holds it until the
    /// [`Output`] is dropped ([`hold`], [`release`]; the device calls back on the system's
    /// thread).
    pub fn start(source: Arc<Mutex<dyn Source>>, want: Option<Layout>) -> Result<Output, Refused> {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("nv sound out".into())
            .spawn(move || match open(source, want) {
                Ok((stream, rate, layout)) => {
                    let _ = tx.send(Ok((rate, layout)));
                    hold(&flag, std::time::Duration::from_millis(200));
                    drop(stream);
                }
                Err(why) => {
                    let _ = tx.send(Err(why));
                }
            })
            .map_err(|_| Refused::NoDevice)?;
        let (rate, layout) = rx.recv().map_err(|_| Refused::NoDevice)??;
        Ok(Output {
            stop,
            thread: Some(thread),
            rate,
            layout,
        })
    }
}

#[cfg(windows)]
mod wasapi {
    use super::*;
    use std::ffi::c_void;

    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> u32;
        fn CoCreateInstance(
            clsid: *const [u8; 16],
            outer: *mut c_void,
            context: u32,
            iid: *const [u8; 16],
            out: *mut *mut Obj,
        ) -> u32;
        fn CoTaskMemFree(p: *mut c_void);
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateEventW(
            attributes: *mut c_void,
            manual: i32,
            initial: i32,
            name: *const u16,
        ) -> *mut c_void;
        fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetCurrentThread() -> *mut c_void;
        fn SetThreadPriority(thread: *mut c_void, priority: i32) -> i32;
    }
    #[link(name = "avrt")]
    extern "system" {
        fn AvSetMmThreadCharacteristicsW(task: *const u16, index: *mut u32) -> *mut c_void;
        fn AvSetMmThreadPriority(task: *mut c_void, priority: i32) -> i32;
        fn AvRevertMmThreadCharacteristics(task: *mut c_void) -> i32;
    }

    /// Runs the calling thread as the player's 32-bit dsound.dll runs its mixing thread (read
    /// from the DLL: `1005cbd8`..`1005cbff`, `1005ceb3`..`1005cee0`): time-critical priority (15),
    /// registered with the multimedia class scheduler as an "Audio" task at high priority
    /// (`AVRT_PRIORITY_HIGH`), so busy processors do not hold up the device's refills. Answers
    /// the task to give back when the thread ends (null when it was refused; nothing changes
    /// then).
    unsafe fn audio_priority() -> *mut c_void {
        SetThreadPriority(GetCurrentThread(), 15);
        let task: Vec<u16> = "Audio\0".encode_utf16().collect();
        let mut index = 0u32;
        let handle = AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut index);
        if handle.is_null() || handle as isize == -1 {
            return std::ptr::null_mut();
        }
        AvSetMmThreadPriority(handle, 1);
        handle
    }

    #[repr(C)]
    struct Obj {
        vtable: *const usize,
    }

    unsafe fn slot<F: Copy>(o: *mut Obj, offset: usize) -> F {
        let f = *(*o).vtable.add(offset / std::mem::size_of::<usize>());
        std::mem::transmute_copy(&f)
    }

    type Fn0 = unsafe extern "system" fn(*mut Obj) -> u32;
    type Fn1 = unsafe extern "system" fn(*mut Obj, usize) -> u32;
    type Fn2 = unsafe extern "system" fn(*mut Obj, usize, usize) -> u32;
    type Fn3 = unsafe extern "system" fn(*mut Obj, usize, usize, usize) -> u32;
    type Fn4 = unsafe extern "system" fn(*mut Obj, usize, usize, usize, usize) -> u32;
    type FnInit =
        unsafe extern "system" fn(*mut Obj, u32, u32, i64, i64, *const u8, *const c_void) -> u32;

    fn guid(s: &str) -> [u8; 16] {
        let h: Vec<u8> = s
            .bytes()
            .filter(u8::is_ascii_hexdigit)
            .map(|c| (c as char).to_digit(16).unwrap_or(0) as u8)
            .collect();
        let b = |i: usize| h[2 * i] << 4 | h[2 * i + 1];
        [
            b(3),
            b(2),
            b(1),
            b(0),
            b(5),
            b(4),
            b(7),
            b(6),
            b(8),
            b(9),
            b(10),
            b(11),
            b(12),
            b(13),
            b(14),
            b(15),
        ]
    }

    /// The default device's mix rate and layout (`GetMixFormat`), on a thread with COM of its
    /// own.
    pub unsafe fn mix_format() -> Option<(u32, Layout)> {
        CoInitializeEx(std::ptr::null_mut(), 0);
        let mut en: *mut Obj = std::ptr::null_mut();
        let r = CoCreateInstance(
            &guid("bcde0395-e52f-467c-8e3d-c4579291692e"),
            std::ptr::null_mut(),
            0x17,
            &guid("a95664d2-9614-4f35-a746-de8db63617e6"),
            &mut en,
        );
        if r != 0 {
            return None;
        }
        let w = std::mem::size_of::<usize>();
        let mut dev: *mut Obj = std::ptr::null_mut();
        let r = slot::<Fn3>(en, 4 * w)(en, 0, 0, &mut dev as *mut _ as usize);
        slot::<Fn0>(en, 2 * w)(en);
        if r != 0 {
            return None;
        }
        let mut client: *mut Obj = std::ptr::null_mut();
        let iid = guid("1cb9ad4c-dbfa-4c32-b178-c2f568a703b2");
        let r = slot::<Fn4>(dev, 3 * w)(
            dev,
            iid.as_ptr() as usize,
            0x17,
            0,
            &mut client as *mut _ as usize,
        );
        slot::<Fn0>(dev, 2 * w)(dev);
        if r != 0 {
            return None;
        }
        let mut fmt: *mut u8 = std::ptr::null_mut();
        let r = slot::<Fn1>(client, 8 * w)(client, &mut fmt as *mut _ as usize);
        let mix = if r == 0 && !fmt.is_null() {
            let rate = u32::from_le_bytes([*fmt.add(4), *fmt.add(5), *fmt.add(6), *fmt.add(7)]);
            layout_of(&format_bytes(fmt)).map(|l| (rate, l))
        } else {
            None
        };
        if !fmt.is_null() {
            CoTaskMemFree(fmt as *mut c_void);
        }
        slot::<Fn0>(client, 2 * w)(client);
        mix
    }

    /// A `WAVEFORMATEX` and the `cbSize` bytes after it (a PCM one has none).
    unsafe fn format_bytes(fmt: *const u8) -> Vec<u8> {
        let tag = u16::from_le_bytes([*fmt, *fmt.add(1)]);
        let extra = if tag == 1 {
            0
        } else {
            usize::from(u16::from_le_bytes([*fmt.add(16), *fmt.add(17)]))
        };
        std::slice::from_raw_parts(fmt, 18 + extra).to_vec()
    }

    /// The device's client, render client, event, buffer size, rate and layout.
    struct Device {
        client: *mut Obj,
        render: *mut Obj,
        event: *mut c_void,
        frames: u32,
        rate: u32,
        layout: Layout,
    }

    unsafe fn open(want: Option<Layout>) -> Result<Device, Refused> {
        CoInitializeEx(std::ptr::null_mut(), 0);
        let mut en: *mut Obj = std::ptr::null_mut();
        let r = CoCreateInstance(
            &guid("bcde0395-e52f-467c-8e3d-c4579291692e"),
            std::ptr::null_mut(),
            0x17,
            &guid("a95664d2-9614-4f35-a746-de8db63617e6"),
            &mut en,
        );
        if r != 0 {
            return Err(Refused::NoDevice);
        }
        let mut dev: *mut Obj = std::ptr::null_mut();
        let r = slot::<Fn3>(en, 0x10 * std::mem::size_of::<usize>() / 4)(
            en,
            0,
            0,
            &mut dev as *mut _ as usize,
        );
        slot::<Fn0>(en, 2 * std::mem::size_of::<usize>())(en);
        if r != 0 {
            return Err(Refused::NoDevice);
        }
        let w = std::mem::size_of::<usize>();
        let mut client: *mut Obj = std::ptr::null_mut();
        let iid = guid("1cb9ad4c-dbfa-4c32-b178-c2f568a703b2");
        let r = slot::<Fn4>(dev, 3 * w)(
            dev,
            iid.as_ptr() as usize,
            0x17,
            0,
            &mut client as *mut _ as usize,
        );
        slot::<Fn0>(dev, 2 * w)(dev);
        if r != 0 {
            return Err(Refused::NoDevice);
        }
        let mut fmt: *mut u8 = std::ptr::null_mut();
        if slot::<Fn1>(client, 8 * w)(client, &mut fmt as *mut _ as usize) != 0 || fmt.is_null() {
            slot::<Fn0>(client, 2 * w)(client);
            return Err(Refused::NoDevice);
        }
        let tag = u16::from_le_bytes([*fmt, *fmt.add(1)]);
        let rate = u32::from_le_bytes([*fmt.add(4), *fmt.add(5), *fmt.add(6), *fmt.add(7)]);
        let bits = u16::from_le_bytes([*fmt.add(14), *fmt.add(15)]);
        // IEEE float, directly (3) or through WAVE_FORMAT_EXTENSIBLE (0xfffe) with the float
        // subformat {00000003-0000-0010-8000-00aa00389b71}.
        let float = tag == 3 || (tag == 0xfffe && *fmt.add(24) == 3);
        let mix = match layout_of(&format_bytes(fmt)) {
            _ if !float || bits != 32 => Err(format!(
                "the sound device mixes {bits}-bit format {tag:#x}: not provided (32-bit float only)"
            )),
            Some(l) if l.provided() => Ok(l),
            Some(l) => Err(not_provided(l)),
            None => Err("the sound device's mix format cannot be read".to_string()),
        };
        let mix = match mix {
            Ok(l) => l,
            Err(why) => {
                CoTaskMemFree(fmt as *mut c_void);
                slot::<Fn0>(client, 2 * w)(client);
                return Err(Refused::Format(why));
            }
        };
        // Another layout than the mix format's (DirectSound's two channels on a larger
        // endpoint): asked with `IsFormatSupported` first, as DirectSound does; only `S_OK`
        // opens it.
        let (layout, other) = match want {
            Some(l) if l != mix => {
                let f = float_format(rate, l);
                let mut closest: *mut u8 = std::ptr::null_mut();
                let r = slot::<Fn3>(client, 7 * w)(
                    client,
                    0,
                    f.as_ptr() as usize,
                    &mut closest as *mut _ as usize,
                );
                if !closest.is_null() {
                    CoTaskMemFree(closest as *mut c_void);
                }
                if r != 0 {
                    CoTaskMemFree(fmt as *mut c_void);
                    slot::<Fn0>(client, 2 * w)(client);
                    return Err(Refused::Layout);
                }
                (l, Some(f))
            }
            _ => (mix, None),
        };
        // Shared, event driven, the engine's default period.
        let format = other.as_ref().map_or(fmt as *const u8, |f| f.as_ptr());
        let r = slot::<FnInit>(client, 3 * w)(client, 0, 0x4_0000, 0, 0, format, std::ptr::null());
        CoTaskMemFree(fmt as *mut c_void);
        if r != 0 {
            slot::<Fn0>(client, 2 * w)(client);
            return Err(Refused::NoDevice);
        }
        let event = CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null());
        slot::<Fn1>(client, 13 * w)(client, event as usize);
        let mut frames = 0u32;
        slot::<Fn1>(client, 4 * w)(client, &mut frames as *mut _ as usize);
        let mut render: *mut Obj = std::ptr::null_mut();
        let iid = guid("f294acfc-3146-4483-a7bf-addca7c260e2");
        slot::<Fn2>(client, 14 * w)(
            client,
            iid.as_ptr() as usize,
            &mut render as *mut _ as usize,
        );
        if render.is_null() {
            CloseHandle(event);
            slot::<Fn0>(client, 2 * w)(client);
            return Err(Refused::NoDevice);
        }
        Ok(Device {
            client,
            render,
            event,
            frames,
            rate,
            layout,
        })
    }

    pub fn start(source: Arc<Mutex<dyn Source>>, want: Option<Layout>) -> Result<Output, Refused> {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("nv sound out".into())
            .spawn(move || unsafe {
                qos::thread_started();
                let task = audio_priority();
                match open(want) {
                    Ok(d) => {
                        let _ = tx.send(Ok((d.rate, d.layout)));
                        run(&d, &source, &flag);
                    }
                    Err(why) => {
                        let _ = tx.send(Err(why));
                    }
                }
                // As dsound.dll does before its thread ends (`AvRevertMmThreadCharacteristics`).
                if !task.is_null() {
                    AvRevertMmThreadCharacteristics(task);
                }
            })
            .map_err(|_| Refused::NoDevice)?;
        let (rate, layout) = rx.recv().map_err(|_| Refused::NoDevice)??;
        Ok(Output {
            stop,
            thread: Some(thread),
            rate,
            layout,
        })
    }

    unsafe fn run(d: &Device, source: &Arc<Mutex<dyn Source>>, stop: &AtomicBool) {
        let w = std::mem::size_of::<usize>();
        let channels = d.layout.channels;
        let mut scratch = vec![0f32; d.frames as usize * usize::from(channels)];
        slot::<Fn0>(d.client, 10 * w)(d.client);
        // Underruns: a wake that finds the device with nothing left to play, after the first
        // frames were given (the stream starts empty).
        let (mut given, mut starved, mut lost) = (false, 0u64, false);
        while !stop.load(Ordering::Relaxed) {
            WaitForSingleObject(d.event, 200);
            let mut padding = 0u32;
            let hr = slot::<Fn1>(d.client, 6 * w)(d.client, &mut padding as *mut _ as usize);
            if hr != 0 {
                // The device no longer answers (unplugged, disabled, or the default device
                // changed: AUDCLNT_E_DEVICE_INVALIDATED): not an underrun. Named once; nothing
                // is given to it while it does not answer (the stream is not opened again).
                if !lost {
                    lost = true;
                    eprintln!(
                        "sound: the sound device stopped answering (GetCurrentPadding {hr:#010x}: \
                         unplugged, disabled or no longer the default?); this stream is not heard \
                         while it does not answer"
                    );
                }
                continue;
            }
            if given && padding == 0 {
                starved += 1;
                if let Ok(mut s) = source.lock() {
                    s.starved(starved);
                }
            }
            let free = d.frames.saturating_sub(padding) as usize;
            if free == 0 {
                continue;
            }
            let mut data: *mut f32 = std::ptr::null_mut();
            if slot::<Fn2>(d.render, 3 * w)(d.render, free, &mut data as *mut _ as usize) != 0 {
                continue;
            }
            let n = free * usize::from(channels);
            let out = &mut scratch[..n];
            out.fill(0.0);
            if let Ok(mut s) = source.lock() {
                s.render(d.rate, channels, out);
            }
            if MUTED.load(Ordering::Relaxed) {
                std::ptr::write_bytes(data, 0, n);
            } else {
                std::ptr::copy_nonoverlapping(out.as_ptr(), data, n);
            }
            slot::<Fn2>(d.render, 4 * w)(d.render, free, 0);
            given = true;
        }
        slot::<Fn0>(d.client, 11 * w)(d.client);
        slot::<Fn0>(d.render, 2 * w)(d.render);
        slot::<Fn0>(d.client, 2 * w)(d.client);
        CloseHandle(d.event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A list entry of 32-bit float (or not) at one rate.
    fn at(rate: u32, channels: u16) -> Range {
        Range {
            f32: true,
            channels,
            min_rate: rate,
            max_rate: rate,
            period: None,
        }
    }

    fn offer(rate: u32, channels: u16) -> Offer {
        Offer { rate, channels }
    }

    /// Linux: the device's own rate and channels decide the mix: one or two channels as offered
    /// (Windows' layout for a mix format without a mask), more as two, never another rate; a
    /// stream the device's list does not hold in 32-bit float is refused, named.
    #[test]
    fn the_device_offer_decides_the_mix() {
        let l = System::Linux;
        let all = [
            at(48000, 1),
            at(48000, 2),
            at(44100, 2),
            at(96000, 2),
            at(96000, 6),
        ];
        assert_eq!(
            mix_of(offer(48000, 2), &all, l),
            Ok((48000, Layout::STEREO))
        );
        assert_eq!(
            mix_of(offer(44100, 2), &all, l),
            Ok((44100, Layout::STEREO))
        );
        assert_eq!(mix_of(offer(48000, 1), &all, l), Ok((48000, Layout::MONO)));
        // Six channels whose speakers are not named: two, at the device's own rate.
        assert_eq!(
            mix_of(offer(96000, 6), &all, l),
            Ok((96000, Layout::STEREO))
        );
        // Not in the list (or not in float): refused, saying why.
        let why = mix_of(offer(22050, 2), &all, l).unwrap_err();
        assert!(
            why.contains("32-bit float") && why.contains("22050"),
            "{why}"
        );
        let int16 = [Range {
            f32: false,
            ..at(48000, 2)
        }];
        assert!(mix_of(offer(48000, 2), &int16, l).is_err());
        let why = mix_of(offer(48000, 8), &[at(48000, 8)], l).unwrap_err();
        assert!(why.contains("8 channels") && why.contains("two"), "{why}");
        assert!(mix_of(offer(48000, 0), &all, l).is_err());
        // A rate range holds every rate in it.
        let wide = [Range {
            min_rate: 8000,
            max_rate: 192000,
            ..at(0, 2)
        }];
        assert_eq!(
            mix_of(offer(44100, 2), &wide, l),
            Ok((44100, Layout::STEREO))
        );
    }

    /// macOS: the list is not asked (cpal names one channel count there, the sum of the device's
    /// output streams). The mix is the offer's (two channels for a larger device), and the stream
    /// asked of the device is its own offered channel count at its own rate: 2, 6 and 8 channel
    /// offers, and a two-stream device (offer 2, list 4) asked for its first stream's 2.
    #[test]
    fn on_macos_the_device_is_asked_for_its_own_offer() {
        let a = System::Apple;
        let cases = [
            // (offer rate, offer channels, list, mix layout, stream channels)
            (44100, 2, vec![at(44100, 2)], Layout::STEREO, 2),
            (48000, 6, vec![at(48000, 6)], Layout::STEREO, 6),
            (96000, 8, vec![at(96000, 8)], Layout::STEREO, 8),
            (44100, 2, vec![at(44100, 4)], Layout::STEREO, 2),
            (48000, 1, vec![], Layout::MONO, 1),
        ];
        for (rate, ch, list, layout, channels) in cases {
            let p = plan(offer(rate, ch), &list, a, None).unwrap();
            assert_eq!(p.mix, (rate, layout), "{ch}");
            assert_eq!(
                (p.rate, p.layout, p.channels),
                (rate, layout, channels),
                "{ch}"
            );
            assert_eq!(p.period, None, "macOS asks no period");
            // The game's device asking for the mix's own layout gets the same.
            assert_eq!(plan(offer(rate, ch), &list, a, Some(layout)), Ok(p));
        }
        assert!(mix_of(offer(48000, 0), &[], a).is_err());
        // Another layout than the mix is never asked for on macOS: DirectSound locks to the mix.
        let stereo = (48000, Layout::STEREO);
        assert_eq!(
            open_as(stereo, Some(Layout::MONO), &[at(48000, 6)], a),
            Err(Refused::Layout)
        );
        assert_eq!(
            plan(offer(48000, 6), &[at(48000, 1)], a, Some(Layout::MONO)),
            Err(Refused::Layout)
        );
        // Linux opens the mix's own channels.
        let l = System::Linux;
        assert_eq!(stream_channels(offer(48000, 6), Layout::STEREO, l), 2);
        assert_eq!(stream_channels(offer(48000, 6), Layout::STEREO, a), 6);
        assert_eq!(stream_channels(offer(48000, 1), Layout::STEREO, a), 2);
    }

    /// Which channels carry the mix: the first ones of each frame, the rest silent.
    #[test]
    fn the_mix_is_in_the_first_channels() {
        let mix = [1.0, 2.0, 3.0, 4.0];
        let mut out = [9.0f32; 12];
        widen(&mix, 2, &mut out, 6);
        assert_eq!(
            out,
            [1.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0]
        );
        let mut out = [9.0f32; 4];
        widen(&mix, 2, &mut out, 2);
        assert_eq!(out, mix);
        let mut out = [9.0f32; 3];
        widen(&[5.0], 1, &mut out, 3);
        assert_eq!(out, [5.0, 0.0, 0.0]);
    }

    /// The stream opens as the WASAPI arm's does: the mix as it is, another layout only when the
    /// device takes it (else `Layout`, so DirectSound's device locks to the mix), a layout
    /// DirectSound does not place sound on refused as a format.
    #[test]
    fn the_stream_opens_as_on_windows() {
        let l = System::Linux;
        let stereo_only = [at(48000, 2)];
        let both = [at(48000, 1), at(48000, 2)];
        let stereo = (48000, Layout::STEREO);
        let mono = (48000, Layout::MONO);
        assert_eq!(open_as(stereo, None, &stereo_only, l), Ok(stereo));
        assert_eq!(
            open_as(stereo, Some(Layout::STEREO), &stereo_only, l),
            Ok(stereo)
        );
        assert_eq!(open_as(mono, Some(Layout::STEREO), &both, l), Ok(stereo));
        assert_eq!(
            open_as(mono, Some(Layout::STEREO), &[at(48000, 1)], l),
            Err(Refused::Layout)
        );
        let odd = (
            48000,
            Layout {
                channels: 3,
                mask: 0x3,
            },
        );
        assert!(matches!(
            open_as(odd, None, &both, l),
            Err(Refused::Format(_))
        ));
        assert!(matches!(
            open_as(odd, None, &both, System::Apple),
            Err(Refused::Format(_))
        ));
        // Linux's plan: the wanted layout's own list entry gives the period.
        let mut mono_entry = at(48000, 1);
        mono_entry.period = Some((64, 4096));
        let p = plan(
            offer(48000, 2),
            &[mono_entry, at(48000, 2)],
            l,
            Some(Layout::MONO),
        )
        .unwrap();
        assert_eq!(
            (p.layout, p.channels, p.period),
            (Layout::MONO, 1, Some(480))
        );
        let p = plan(offer(48000, 2), &[mono_entry, at(48000, 2)], l, None).unwrap();
        assert_eq!((p.layout, p.channels, p.period), (Layout::STEREO, 2, None));
    }

    /// A stream the system refuses: another layout than the mix is `Layout` (asked again in the
    /// mix), the mix itself not provided, named with the device's offer; anything else no device.
    #[test]
    fn a_refused_stream_is_named() {
        let mix = (48000, Layout::STEREO);
        let o = offer(48000, 6);
        assert_eq!(
            refused(Kind::UnsupportedConfig, Layout::MONO, mix, o),
            Refused::Layout
        );
        match refused(Kind::UnsupportedConfig, Layout::STEREO, mix, o) {
            Refused::Format(why) => assert!(
                why.contains("offers 6 channels at 48000 Hz") && why.contains("2 channels"),
                "{why}"
            ),
            r => panic!("{r:?}"),
        }
        for k in [
            Kind::Xrun,
            Kind::DeviceNotAvailable,
            Kind::StreamInvalidated,
            Kind::DeviceChanged,
            Kind::Other,
        ] {
            assert_eq!(
                refused(k, Layout::STEREO, mix, o),
                Refused::NoDevice,
                "{k:?}"
            );
        }
    }

    /// Linux's period: DirectSound's 10 ms where the device takes it, else the system's; the
    /// stream's line names the device, the rate and channels asked, the period the system opened
    /// (and why a 10 ms request fell back), and says that what the system does after is not
    /// measured.
    #[test]
    fn the_period_is_the_mixers_where_taken() {
        assert_eq!(fixed_period(48000, Some((64, 8192))), Some(480));
        assert_eq!(fixed_period(44100, Some((441, 441))), Some(441));
        assert_eq!(fixed_period(48000, Some((512, 8192))), None);
        assert_eq!(fixed_period(48000, Some((64, 479))), None);
        assert_eq!(fixed_period(48000, None), None);
        let opened = Periods {
            asked: Some(441),
            fell_back: None,
            opened: Some(448),
        };
        let line = opened_line(System::Linux, "HDA Intel PCH", 44100, 2, 2, &opened);
        for piece in [
            "\"HDA Intel PCH\"",
            "44100 Hz",
            "2 channels asked",
            "448 frames opened",
            "441 frames asked",
            "cpal prefers",
            "not measured",
        ] {
            assert!(line.contains(piece), "{piece}: {line}");
        }
        let fell = Periods {
            asked: Some(480),
            fell_back: Some("Buffer size 480 is not in the supported range".into()),
            opened: Some(1024),
        };
        let line = opened_line(System::Linux, "pipewire", 48000, 2, 2, &fell);
        assert!(
            line.contains("1024 frames opened")
                && line.contains("were refused: Buffer size 480 is not in the supported range"),
            "{line}"
        );
        let mac = Periods {
            opened: Some(512),
            ..Periods::default()
        };
        let line = opened_line(System::Apple, "HDMI", 48000, 6, 2, &mac);
        for piece in [
            "6 channels asked",
            "the mix is in the first 2 channels, the other 4 silent",
            "512 frames opened",
            "the system's own",
            "device's own offered channels",
            "not verified on a Mac",
        ] {
            assert!(line.contains(piece), "{piece}: {line}");
        }
        assert!(!line.contains("converts"), "{line}");
    }

    /// A source that writes a ramp.
    #[derive(Default)]
    struct Ramp {
        asked: Vec<(u32, u16, usize)>,
        starved: Vec<u64>,
    }

    impl Source for Ramp {
        fn render(&mut self, rate: u32, channels: u16, out: &mut [f32]) {
            assert!(out.iter().all(|&s| s == 0.0), "given a zeroed period");
            self.asked.push((rate, channels, out.len()));
            for (i, s) in out.iter_mut().enumerate() {
                *s = i as f32 + 1.0;
            }
        }
        fn starved(&mut self, count: u64) {
            self.starved.push(count);
        }
    }

    /// Each period the device asks for is the mixer's frames as they are (or silence when
    /// muted, the mixer still pulled), widened into a larger stream's first channels; once lost,
    /// silence and the mixer not pulled.
    #[test]
    fn a_period_is_the_mixers_frames() {
        let ramp = Arc::new(Mutex::new(Ramp::default()));
        let source: Arc<Mutex<dyn Source>> = ramp.clone();
        let mut out = [9.0f32; 6];
        fill(&source, 44100, 2, &mut out, false);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let mut out = [9.0f32; 4];
        fill(&source, 44100, 2, &mut out, true);
        assert_eq!(out, [0.0; 4]);
        assert_eq!(ramp.lock().unwrap().asked, [(44100, 2, 6), (44100, 2, 4)]);

        let gate = Gate::default();
        let mut scratch = Vec::new();
        let mut out = [9.0f32; 2];
        period(&gate, &source, 48000, 1, &mut out, 1, &mut scratch, false);
        assert_eq!(out, [1.0, 2.0]);
        assert!(gate.given.load(Ordering::Relaxed));
        // A six-channel stream: two frames of the stereo mix in channels 1 and 2.
        let mut out = [9.0f32; 12];
        period(&gate, &source, 48000, 2, &mut out, 6, &mut scratch, false);
        assert_eq!(
            out,
            [1.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(ramp.lock().unwrap().asked[3], (48000, 2, 4));
        gate.lost.store(true, Ordering::Relaxed);
        let mut out = [9.0f32; 2];
        period(&gate, &source, 48000, 1, &mut out, 1, &mut scratch, false);
        assert_eq!(out, [0.0; 2]);
        assert_eq!(ramp.lock().unwrap().asked.len(), 4, "not pulled once lost");
        // Not given before the first period.
        assert!(!Gate::default().given.load(Ordering::Relaxed));
    }

    /// The system's answers by kind: an xrun is an underrun on Linux and an overload on macOS; a
    /// device gone or a stream to build again is lost; a changed default device is a change.
    #[test]
    fn the_systems_answers_are_told_apart() {
        let w = || "why".to_string();
        assert_eq!(event(Kind::Xrun, w(), System::Linux), Event::Underrun);
        assert_eq!(event(Kind::Xrun, w(), System::Apple), Event::Overload);
        for s in [System::Linux, System::Apple] {
            assert_eq!(
                event(Kind::DeviceNotAvailable, w(), s),
                Event::Lost("why".into())
            );
            assert_eq!(
                event(Kind::StreamInvalidated, w(), s),
                Event::Lost("why".into())
            );
            assert_eq!(event(Kind::DeviceChanged, w(), s), Event::Changed);
            assert_eq!(
                event(Kind::UnsupportedConfig, w(), s),
                Event::Other("why".into())
            );
            assert_eq!(event(Kind::Other, w(), s), Event::Other("why".into()));
        }
        assert_eq!(SYSTEM == System::Apple, cfg!(target_os = "macos"));
    }

    /// Underruns count once frames were given and until the device is lost; a lost device is said
    /// once and silences the stream; a changed default device is said and the stream plays on;
    /// overloads are counted apart; other reports and the Linux thread line are said once per run,
    /// whichever stream says them.
    #[test]
    fn the_device_reports_as_on_windows() {
        let said = Said::default();
        let mut w = Watch::default();
        assert_eq!(w.on(Event::Underrun, false, &said), Reaction::None);
        assert_eq!(w.on(Event::Underrun, true, &said), Reaction::Starved(1));
        assert_eq!(w.on(Event::Underrun, true, &said), Reaction::Starved(2));
        assert!(
            matches!(w.on(Event::Other("x".into()), true, &said), Reaction::Say(s) if s.contains('x'))
        );
        assert_eq!(w.on(Event::Other("y".into()), true, &said), Reaction::None);
        // A changed default device: said each time, not lost; underruns still counted.
        assert!(
            matches!(w.on(Event::Changed, true, &said), Reaction::Say(s) if s.contains("plays on"))
        );
        assert!(matches!(
            w.on(Event::Changed, true, &said),
            Reaction::Say(_)
        ));
        assert_eq!(w.on(Event::Underrun, true, &said), Reaction::Starved(3));
        assert!(
            matches!(w.on(Event::Lost("gone".into()), true, &said), Reaction::Lose(s) if s.contains("gone"))
        );
        assert_eq!(
            w.on(Event::Lost("gone".into()), true, &said),
            Reaction::None
        );
        assert_eq!(w.on(Event::Changed, true, &said), Reaction::None);
        // Once lost, no more underruns or overloads.
        assert_eq!(w.on(Event::Underrun, true, &said), Reaction::None);
        assert_eq!(w.on(Event::Overload, true, &said), Reaction::None);

        // A second stream of the same run: its own counts, but "other" was said already.
        let mut w = Watch::default();
        assert_eq!(w.on(Event::Other("z".into()), true, &said), Reaction::None);
        // Overloads: counted apart from underruns, said at 1, 10, 100.
        let said_at: Vec<u64> = (1..=150)
            .filter(|_| matches!(w.on(Event::Overload, true, &said), Reaction::Say(_)))
            .collect();
        assert_eq!(said_at.len(), 3);
        assert!(
            matches!(w.on(Event::Overload, true, &said), Reaction::None),
            "the 151st is not said"
        );
        assert_eq!(w.on(Event::Underrun, true, &said), Reaction::Starved(1));

        // The Linux thread line: once per run; none on macOS.
        let run = Said::default();
        assert_eq!(thread_line(System::Apple, &run), None);
        assert_eq!(
            thread_line(System::Linux, &run).as_deref(),
            Some("sound thread: normal priority (no real-time attempt off Windows yet)")
        );
        assert_eq!(thread_line(System::Linux, &run), None);
    }

    /// A reaction carried out: an underrun told to the source, a lost device silencing the
    /// stream's gate, lines handed back to say.
    #[test]
    fn reactions_reach_the_source_and_the_gate() {
        let ramp = Arc::new(Mutex::new(Ramp::default()));
        let source: Arc<Mutex<dyn Source>> = ramp.clone();
        let gate = Gate::default();
        assert_eq!(react(Reaction::None, &gate, &source), None);
        assert_eq!(react(Reaction::Starved(4), &gate, &source), None);
        assert_eq!(ramp.lock().unwrap().starved, [4]);
        assert_eq!(
            react(Reaction::Say("a".into()), &gate, &source),
            Some("a".into())
        );
        assert!(!gate.lost.load(Ordering::Relaxed));
        assert_eq!(
            react(Reaction::Lose("b".into()), &gate, &source),
            Some("b".into())
        );
        assert!(gate.lost.load(Ordering::Relaxed));
    }

    /// The thread holding a stream ends as soon as it is released (not after its wait).
    #[test]
    fn a_held_stream_is_released_at_once() {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let flag = stop.clone();
        let t = std::thread::spawn(move || {
            hold(&flag, std::time::Duration::from_secs(120));
            let _ = tx.send(());
        });
        // Let it park first (a release before the park also ends it: the flag is read first).
        std::thread::sleep(std::time::Duration::from_millis(50));
        release(&stop, t.thread());
        assert!(
            rx.recv_timeout(std::time::Duration::from_secs(20)).is_ok(),
            "still held after release"
        );
        t.join().unwrap();
    }

    /// Off Windows: cpal's answers and list entries as this module reads them.
    #[cfg(not(windows))]
    #[test]
    fn cpals_answers_are_read_as_its_kinds() {
        use cpal::ErrorKind as K;
        assert_eq!(device::kind_of(K::Xrun), Kind::Xrun);
        assert_eq!(
            device::kind_of(K::DeviceNotAvailable),
            Kind::DeviceNotAvailable
        );
        assert_eq!(
            device::kind_of(K::StreamInvalidated),
            Kind::StreamInvalidated
        );
        assert_eq!(device::kind_of(K::DeviceChanged), Kind::DeviceChanged);
        assert_eq!(
            device::kind_of(K::UnsupportedConfig),
            Kind::UnsupportedConfig
        );
        for k in [K::RealtimeDenied, K::DeviceBusy, K::BackendError, K::Other] {
            assert_eq!(device::kind_of(k), Kind::Other, "{k:?}");
        }
        let r = cpal::SupportedStreamConfigRange::new(
            6,
            44100,
            96000,
            cpal::SupportedBufferSize::Range { min: 64, max: 4096 },
            cpal::SampleFormat::F32,
        );
        assert_eq!(
            device::range_of(&r),
            Range {
                f32: true,
                channels: 6,
                min_rate: 44100,
                max_rate: 96000,
                period: Some((64, 4096)),
            }
        );
        let r = cpal::SupportedStreamConfigRange::new(
            2,
            48000,
            48000,
            cpal::SupportedBufferSize::Unknown,
            cpal::SampleFormat::I16,
        );
        assert_eq!(
            device::range_of(&r),
            Range {
                f32: false,
                channels: 2,
                min_rate: 48000,
                max_rate: 48000,
                period: None,
            }
        );
    }
}
