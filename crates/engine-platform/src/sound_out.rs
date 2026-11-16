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
//! serve through their ALSA plugins; CoreAudio on macOS). The mixer is the same; only the device
//! differs. The default output device's default stream gives the rate, as the mix format does on
//! Windows ([`mix_of`]). These systems name no speaker positions through cpal (and ALSA orders a
//! 5.1 device's channels otherwise than Windows does), so a device of one or two channels is
//! mixed for as it is (front centre; front left and right: Windows' rule for a mix format without
//! a speaker mask), and a larger one is given two channels, which the system spreads as it does
//! for any stereo program: no surround placement off Windows. Only 32-bit float streams are
//! opened, as on Windows ([`open_as`]); the system converts to the hardware as Windows' audio
//! engine does. The device calls back for each period it wants ([`fill`]: the frames it asks for,
//! straight from the mixer). Differences from the WASAPI arm, all of the system's own choosing:
//! the period and buffer size are the system's defaults (cpal's `BufferSize::Default`), not
//! WASAPI's shared-mode period; the callback runs on the system's thread (real-time priority
//! where the system allows it, cpal's `realtime`), not a thread of this module's at Windows'
//! "Audio" priority; underruns are counted as the system reports them (ALSA's xruns; CoreAudio
//! reports none) rather than as wakes that find the device empty; the device stopping answering,
//! or the default device changing, is named once and the stream is then silent and not pulled,
//! as on Windows ([`Watch`]).

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
            // Off Windows the thread holding the stream waits parked ([`device::start`]).
            #[cfg(not(windows))]
            t.thread().unpark();
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

/// What a device off Windows offers by default (cpal's default output stream): its rate and
/// channel count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    pub rate: u32,
    pub channels: u16,
}

/// The mix rate and layout off Windows for a device whose default stream is `offer`, where
/// `takes(rate, channels)` says whether the device opens a 32-bit float stream so: the device's
/// own rate; one or two channels as offered, with Windows' layout for a mix format without a
/// speaker mask ([`Layout::of_mix`]); more channels, whose speakers the system does not name here,
/// as two (the system spreads them). The reason, for stderr, when there is none.
pub fn mix_of(offer: Offer, takes: impl Fn(u32, u16) -> bool) -> Result<(u32, Layout), String> {
    let Offer { rate, channels } = offer;
    let channels_used = match channels {
        0 => return Err("the sound device offers a stream of no channels".to_string()),
        1 | 2 => channels,
        _ => 2,
    };
    if !takes(rate, channels_used) {
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
/// rate when the device takes it, else [`Refused::Layout`]); a mix layout DirectSound does not
/// place sound on is [`Refused::Format`].
pub fn open_as(
    mix: (u32, Layout),
    want: Option<Layout>,
    takes: impl Fn(u32, u16) -> bool,
) -> Result<(u32, Layout), Refused> {
    let (rate, mix) = mix;
    if !mix.provided() {
        return Err(Refused::Format(not_provided(mix)));
    }
    match want {
        Some(l) if l != mix => {
            if takes(rate, l.channels) {
                Ok((rate, l))
            } else {
                Err(Refused::Layout)
            }
        }
        _ => Ok((rate, mix)),
    }
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

/// What the system says about a running stream off Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The device ran out of frames (an underrun).
    Underrun,
    /// The device stopped answering (unplugged, disabled; the stream is gone).
    Lost(String),
    /// The default device changed under the stream.
    Changed,
    /// The system refused the device thread real-time priority.
    NoRealtime,
    /// Anything else, named.
    Other(String),
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

/// A stream's events off Windows, as the WASAPI arm treats its device's answers: underruns
/// counted once the first frames were given (the stream starts empty); the device lost said
/// once (a changed default device is lost too: WASAPI's stream is invalidated then); other
/// errors said once each kind.
#[derive(Debug, Default)]
pub struct Watch {
    starved: u64,
    lost: bool,
    said_realtime: bool,
    said_other: bool,
}

impl Watch {
    /// The reaction to `e`, after the device was (`given`) or was not yet given frames.
    pub fn on(&mut self, e: Event, given: bool) -> Reaction {
        match e {
            Event::Underrun if given && !self.lost => {
                self.starved += 1;
                Reaction::Starved(self.starved)
            }
            Event::Underrun => Reaction::None,
            Event::Lost(_) | Event::Changed if self.lost => Reaction::None,
            Event::Lost(why) => {
                self.lost = true;
                Reaction::Lose(format!(
                    "sound: the sound device stopped answering ({why}: unplugged or disabled?); \
                     this stream is not heard any more"
                ))
            }
            Event::Changed => {
                self.lost = true;
                Reaction::Lose(
                    "sound: the default sound device changed; this stream is not heard any more \
                     (as on Windows, it is not opened again)"
                        .to_string(),
                )
            }
            Event::NoRealtime if self.said_realtime => Reaction::None,
            Event::NoRealtime => {
                self.said_realtime = true;
                Reaction::Say(
                    "sound: the system refused the sound thread real-time priority; it runs at \
                     normal priority (busy processors may make the sound skip)"
                        .to_string(),
                )
            }
            Event::Other(_) if self.said_other => Reaction::None,
            Event::Other(why) => {
                self.said_other = true;
                Reaction::Say(format!(
                    "sound: the sound device reports: {why} (further reports not shown)"
                ))
            }
        }
    }
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

/// The default output device off Windows, through cpal ([`mix_of`], [`open_as`], [`fill`],
/// [`Watch`] decide; this only asks the system and hands over).
#[cfg(not(windows))]
mod device {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    /// Whether `device` opens a 32-bit float stream at a rate with a channel count.
    fn takes(device: &cpal::Device) -> impl Fn(u32, u16) -> bool {
        let ranges: Vec<cpal::SupportedStreamConfigRange> = device
            .supported_output_configs()
            .map(|c| c.collect())
            .unwrap_or_default();
        move |rate, channels| {
            ranges.iter().any(|r| {
                r.sample_format() == cpal::SampleFormat::F32
                    && r.channels() == channels
                    && (r.min_sample_rate()..=r.max_sample_rate()).contains(&rate)
            })
        }
    }

    /// The default device, its offer and what it takes.
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
        match mix_of(offer, takes(&device)) {
            Ok(m) => Some(m),
            Err(why) => {
                eprintln!("sound: {why}");
                None
            }
        }
    }

    fn event(e: cpal::Error) -> Event {
        use cpal::ErrorKind as K;
        match e.kind() {
            K::Xrun => Event::Underrun,
            K::DeviceNotAvailable | K::StreamInvalidated => Event::Lost(e.to_string()),
            K::DeviceChanged => Event::Changed,
            K::RealtimeDenied => Event::NoRealtime,
            _ => Event::Other(e.to_string()),
        }
    }

    /// Opens the stream and starts it; the stream lives on the calling thread.
    fn open(
        source: Arc<Mutex<dyn Source>>,
        want: Option<Layout>,
    ) -> Result<(cpal::Stream, u32, Layout), Refused> {
        let (device, offer) = default_device().ok_or(Refused::NoDevice)?;
        let takes = takes(&device);
        let mix = mix_of(offer, &takes).map_err(Refused::Format)?;
        let (rate, layout) = open_as(mix, want, &takes)?;
        let channels = layout.channels;
        let given = Arc::new(AtomicBool::new(false));
        let lost = Arc::new(AtomicBool::new(false));
        let (given_cb, lost_cb, source_cb) = (given.clone(), lost.clone(), source.clone());
        let data = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if lost_cb.load(Ordering::Relaxed) {
                out.fill(0.0);
                return;
            }
            fill(
                &*source_cb,
                rate,
                channels,
                out,
                MUTED.load(Ordering::Relaxed),
            );
            given_cb.store(true, Ordering::Relaxed);
        };
        let mut watch = Watch::default();
        let error = move |e: cpal::Error| match watch.on(event(e), given.load(Ordering::Relaxed)) {
            Reaction::None => {}
            Reaction::Starved(n) => {
                if let Ok(mut s) = source.lock() {
                    s.starved(n);
                }
            }
            Reaction::Say(why) => eprintln!("{why}"),
            Reaction::Lose(why) => {
                lost.store(true, Ordering::Relaxed);
                eprintln!("{why}");
            }
        };
        let config = cpal::StreamConfig {
            channels,
            sample_rate: rate,
            buffer_size: cpal::BufferSize::Default,
        };
        let stream = device
            .build_output_stream::<f32, _, _>(config, data, error, None)
            .map_err(|e| match e.kind() {
                cpal::ErrorKind::UnsupportedConfig => Refused::Layout,
                _ => Refused::NoDevice,
            })?;
        stream.play().map_err(|_| Refused::NoDevice)?;
        Ok((stream, rate, layout))
    }

    /// [`super::start`]: the stream opened on a thread of its own, which holds it until the
    /// [`Output`] is dropped (the device calls back on the system's thread).
    pub fn start(source: Arc<Mutex<dyn Source>>, want: Option<Layout>) -> Result<Output, Refused> {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("nv sound out".into())
            .spawn(move || match open(source, want) {
                Ok((stream, rate, layout)) => {
                    let _ = tx.send(Ok((rate, layout)));
                    while !flag.load(Ordering::Relaxed) {
                        std::thread::park_timeout(std::time::Duration::from_millis(200));
                    }
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

    /// A device that takes 32-bit float streams of these channel counts at these rates.
    fn takes(ok: &'static [(u32, u16)]) -> impl Fn(u32, u16) -> bool {
        move |rate, channels| ok.contains(&(rate, channels))
    }

    /// Off Windows the device's own rate and channels decide the mix: one or two channels as
    /// offered (Windows' layout for a mix format without a mask), more as two, never another
    /// rate; a stream the device does not take in 32-bit float is refused, named.
    #[test]
    fn the_device_offer_decides_the_mix() {
        let offer = |rate, channels| Offer { rate, channels };
        let all = takes(&[(48000, 1), (48000, 2), (44100, 2), (96000, 2), (96000, 6)]);
        assert_eq!(mix_of(offer(48000, 2), &all), Ok((48000, Layout::STEREO)));
        assert_eq!(mix_of(offer(44100, 2), &all), Ok((44100, Layout::STEREO)));
        assert_eq!(mix_of(offer(48000, 1), &all), Ok((48000, Layout::MONO)));
        // Six channels whose speakers are not named: two, at the device's own rate.
        assert_eq!(mix_of(offer(96000, 6), &all), Ok((96000, Layout::STEREO)));
        // Not taken in float (or not at all): refused, saying why.
        let why = mix_of(offer(22050, 2), &all).unwrap_err();
        assert!(
            why.contains("32-bit float") && why.contains("22050"),
            "{why}"
        );
        let none = takes(&[(48000, 8)]);
        let why = mix_of(offer(48000, 8), &none).unwrap_err();
        assert!(why.contains("8 channels") && why.contains("two"), "{why}");
        assert!(mix_of(offer(48000, 0), &all).is_err());
    }

    /// The stream opens as the WASAPI arm's does: the mix as it is, another layout only when the
    /// device takes it (else `Layout`, so DirectSound's device locks to the mix), a layout
    /// DirectSound does not place sound on refused as a format.
    #[test]
    fn the_stream_opens_as_on_windows() {
        let stereo_only = takes(&[(48000, 2)]);
        let both = takes(&[(48000, 1), (48000, 2)]);
        let stereo = (48000, Layout::STEREO);
        let mono = (48000, Layout::MONO);
        assert_eq!(open_as(stereo, None, &stereo_only), Ok(stereo));
        assert_eq!(
            open_as(stereo, Some(Layout::STEREO), &stereo_only),
            Ok(stereo)
        );
        assert_eq!(open_as(mono, Some(Layout::STEREO), &both), Ok(stereo));
        assert_eq!(
            open_as(mono, Some(Layout::STEREO), &takes(&[(48000, 1)])),
            Err(Refused::Layout)
        );
        let odd = (
            48000,
            Layout {
                channels: 3,
                mask: 0x3,
            },
        );
        assert!(matches!(open_as(odd, None, &both), Err(Refused::Format(_))));
    }

    /// A source that writes a ramp.
    #[derive(Default)]
    struct Ramp {
        asked: Vec<(u32, u16, usize)>,
    }

    impl Source for Ramp {
        fn render(&mut self, rate: u32, channels: u16, out: &mut [f32]) {
            assert!(out.iter().all(|&s| s == 0.0), "given a zeroed period");
            self.asked.push((rate, channels, out.len()));
            for (i, s) in out.iter_mut().enumerate() {
                *s = i as f32 + 1.0;
            }
        }
    }

    /// Each period the device asks for is the mixer's frames as they are (or silence when
    /// muted, the mixer still pulled).
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
    }

    /// Underruns count once frames were given; a lost device (or a changed default device) is
    /// said once and silences the stream; other reports are said once.
    #[test]
    fn the_device_reports_as_on_windows() {
        let mut w = Watch::default();
        assert_eq!(w.on(Event::Underrun, false), Reaction::None);
        assert_eq!(w.on(Event::Underrun, true), Reaction::Starved(1));
        assert_eq!(w.on(Event::Underrun, true), Reaction::Starved(2));
        assert!(matches!(w.on(Event::NoRealtime, true), Reaction::Say(_)));
        assert_eq!(w.on(Event::NoRealtime, true), Reaction::None);
        assert!(
            matches!(w.on(Event::Other("x".into()), true), Reaction::Say(s) if s.contains('x'))
        );
        assert_eq!(w.on(Event::Other("y".into()), true), Reaction::None);
        assert!(
            matches!(w.on(Event::Lost("gone".into()), true), Reaction::Lose(s) if s.contains("gone"))
        );
        assert_eq!(w.on(Event::Lost("gone".into()), true), Reaction::None);
        assert_eq!(w.on(Event::Changed, true), Reaction::None);
        // Once lost, no more underruns are counted.
        assert_eq!(w.on(Event::Underrun, true), Reaction::None);
        let mut w = Watch::default();
        assert!(matches!(w.on(Event::Changed, true), Reaction::Lose(_)));
        assert_eq!(w.on(Event::Lost("gone".into()), true), Reaction::None);
    }
}
