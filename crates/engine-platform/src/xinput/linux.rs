//! Pads on Linux (the Steam Deck among them), read from the kernel's event devices
//! (`/dev/input/event*`). A thread looks through them every second for pads ([`evdev::is_pad`],
//! [`evdev::faces`]), opens each new one without taking it from other programs (no grab), and
//! reads every pad's events as they come ([`evdev::Pad`]); another thread sends the rumble (the
//! kernel's `FF_RUMBLE` effect, uploaded, then played or stopped), so a device call never runs on
//! the game's thread, which only reads and writes the shared list.
//!
//! Which pad comes first ([`Rank`]): when Steam started the game (it sets
//! `SDL_GAMECONTROLLER_ALLOW_STEAM_VIRTUAL_GAMEPAD`, read by SDL's rules for a yes or no), Steam
//! Input's virtual pad ([`evdev::STEAM_VIRTUAL`]), the pad such a game is meant to read; then the
//! other pads in the order they were found, each look's new ones by their device numbers. Without
//! that variable (the game started outside Steam while Steam runs) a virtual pad comes after every
//! other pad: SDL leaves it out there, and it is kept, last, only so that a Steam Deck in desktop
//! mode still has its built-in controls. The pads Steam reads itself and stands in for are left
//! out: for the games it starts, Steam lists them in `SDL_GAMECONTROLLER_IGNORE_DEVICES`, and a
//! list in `SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT` leaves out every pad not in it, as SDL reads
//! the two ([`SteamLists`]); as in SDL, neither list ever leaves out a virtual pad. So on the Steam
//! Deck the Deck's own controls do not take the first place from Steam's virtual pad, and a pad is
//! not read twice. A pad whose place among the users changes (another put before it, or one before
//! it gone) has its motors stopped, as the game's rumble for its old place is not its own.
//!
//! The devices a user may read: systemd gives the person at the machine the joysticks' event
//! devices (Steam's virtual pad among them); a pad that cannot be opened is tried again at each
//! look. A pad opened only for reading gets no rumble.

use super::chain::PadList;
use super::evdev::{self, Axes, Range, Step};
use super::layout::Controls;
use super::sdl_hints::SteamLists;
use super::{ERROR_DEVICE_NOT_CONNECTED, ERROR_SUCCESS};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How often the event devices are looked through for new pads.
const LOOK_EVERY: Duration = Duration::from_secs(1);
/// How long the reader waits for events before it looks again (milliseconds).
const POLL_MS: i32 = 250;
/// A rumble effect's length: the most the kernel takes (milliseconds). XInput's motors run until
/// they are set again, so a rumble still running is sent again after [`RUMBLE_AGAIN`].
const RUMBLE_LENGTH: u16 = 0xffff;
const RUMBLE_AGAIN: Duration = Duration::from_secs(30);

/// The kernel's `_IOC` (asm-generic/ioctl.h: x86-64 and ARM64 alike).
const fn ioc(dir: u64, kind: u8, nr: u64, size: usize) -> u64 {
    (dir << 30) | ((size as u64) << 16) | ((kind as u64) << 8) | nr
}
const WRITE: u64 = 1;
const READ: u64 = 2;
const EVIOCGID: u64 = ioc(READ, b'E', 0x02, std::mem::size_of::<libc::input_id>());
const fn eviocgbit(ev: u16, len: usize) -> u64 {
    ioc(READ, b'E', 0x20 + ev as u64, len)
}
const fn eviocgkey(len: usize) -> u64 {
    ioc(READ, b'E', 0x18, len)
}
const fn eviocgabs(abs: u16) -> u64 {
    ioc(
        READ,
        b'E',
        0x40 + abs as u64,
        std::mem::size_of::<libc::input_absinfo>(),
    )
}
const EVIOCSFF: u64 = ioc(WRITE, b'E', 0x80, std::mem::size_of::<libc::ff_effect>());

/// The bits an `EVIOCGBIT` or `EVIOCGKEY` request fills, for the codes up to `max`.
fn bits(fd: i32, request: impl Fn(usize) -> u64, max: u16) -> Option<Vec<u8>> {
    let mut b = vec![0u8; usize::from(max) / 8 + 1];
    // SAFETY: the request's size is the buffer's length.
    let r = unsafe { libc::ioctl(fd, request(b.len()) as _, b.as_mut_ptr()) };
    (r >= 0).then_some(b)
}

fn has(b: &[u8], code: u16) -> bool {
    b.get(usize::from(code) / 8)
        .is_some_and(|byte| byte & (1 << (code % 8)) != 0)
}

/// An axis's `input_absinfo` (its value now and its range).
fn absinfo(fd: i32, code: u16) -> Option<libc::input_absinfo> {
    // SAFETY: a plain C structure, all zeros a valid value.
    let mut info: libc::input_absinfo = unsafe { std::mem::zeroed() };
    // SAFETY: the structure the request names.
    let r = unsafe { libc::ioctl(fd, eviocgabs(code) as _, &mut info) };
    (r >= 0).then_some(info)
}

impl SteamLists {
    /// Where a pad `id` comes among the users.
    fn rank(&self, id: (u16, u16)) -> Rank {
        match (id == evdev::STEAM_VIRTUAL, self.allow_virtual) {
            (true, true) => Rank::SteamFirst,
            (true, false) => Rank::SteamLast,
            (false, _) => Rank::Pad,
        }
    }
}

/// Where a pad comes among the users: Steam's virtual pad first when Steam started the game, the
/// other pads, then Steam's virtual pad when it did not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    SteamFirst,
    Pad,
    SteamLast,
}

/// The kernel driver of the event device `node` (`event7`'s: `<sys>/event7/device/device/driver`,
/// the device the input device belongs to), `None` for a virtual device (made by a program through
/// uinput, as Steam's virtual pad is), which has none.
fn driver(sys: &Path, node: &Path) -> Option<String> {
    let link = sys.join(node.file_name()?).join("device/device/driver");
    let target = std::fs::read_link(link).ok()?;
    Some(target.file_name()?.to_str()?.to_string())
}

/// The event devices in `dir`, by their numbers.
fn event_devices(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<(u32, PathBuf)> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let name = entry.file_name();
                    let n = name.to_str()?.strip_prefix("event")?.parse().ok()?;
                    Some((n, entry.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found.into_iter().map(|(_, p)| p).collect()
}

/// Where the reader looks.
struct Config {
    /// The event devices to look at, each time.
    devices: Box<dyn Fn() -> Vec<PathBuf> + Send>,
    /// The kernel's `/sys/class/input`, where the devices' drivers are named.
    sys: PathBuf,
    lists: SteamLists,
}

impl Config {
    fn system() -> Config {
        Config {
            devices: Box::new(|| event_devices(Path::new("/dev/input"))),
            sys: PathBuf::from("/sys/class/input"),
            lists: SteamLists::from_env(),
        }
    }
}

/// One open pad.
struct Pad {
    /// Its number in the list, never used again.
    key: u64,
    path: PathBuf,
    file: Arc<File>,
    /// The axes it has, read again after the kernel dropped events.
    axes: Vec<u16>,
    events: evdev::Pad,
    /// The reading at the last report, and its packet number (one more at each change).
    controls: Controls,
    packet: u32,
    rank: Rank,
    /// The kernel dropped events: the reader reads the device's state again, outside the list's
    /// lock.
    resync: bool,
    /// Opened for writing and has the rumble effect; false too once the device is gone.
    can_rumble: bool,
    /// The motor speeds the game set, and the ones last sent (and when).
    wanted: (u16, u16),
    sent: (u16, u16),
    sent_at: Option<Instant>,
    /// The kernel's id of its rumble effect, once uploaded.
    effect: Option<i16>,
    gone: bool,
}

impl Pad {
    /// The controls read out at a report; a change is a new packet.
    fn report(&mut self) {
        let controls = self.events.controls();
        if controls != self.controls {
            self.controls = controls;
            self.packet = self.packet.wrapping_add(1);
        }
    }

    /// The rumble to send now, if any: the speeds changed, or a rumble still running is due again.
    fn rumble_due(&self, now: Instant) -> Option<Rumble> {
        if self.gone || !self.can_rumble {
            return None;
        }
        let again = self.resend_at().is_some_and(|at| now >= at);
        (self.wanted != self.sent || again).then(|| Rumble {
            key: self.key,
            file: self.file.clone(),
            effect: self.effect,
            speeds: self.wanted,
        })
    }

    /// When a rumble still running is sent again.
    fn resend_at(&self) -> Option<Instant> {
        if self.gone || !self.can_rumble || self.sent == (0, 0) {
            return None;
        }
        self.sent_at.map(|at| at + RUMBLE_AGAIN)
    }
}

/// A rumble to send, taken out of the list so the device call runs without the list's lock.
struct Rumble {
    key: u64,
    file: Arc<File>,
    effect: Option<i16>,
    speeds: (u16, u16),
}

#[derive(Default)]
struct List {
    pads: Vec<Pad>,
    next_key: u64,
}

impl List {
    fn live(&mut self) -> impl Iterator<Item = &mut Pad> {
        self.pads.iter_mut().filter(|p| !p.gone)
    }

    /// Adds a new pad after those of its [`Rank`] and before those of a later one; the pads after
    /// it, whose users change, have their motors stopped.
    fn add(&mut self, mut pad: Pad) {
        pad.key = self.next_key;
        self.next_key += 1;
        let at = self
            .pads
            .iter()
            .position(|p| p.rank > pad.rank)
            .unwrap_or(self.pads.len());
        self.pads.insert(at, pad);
        self.moved_from(at + 1);
    }

    /// The pad `key` has gone: the pads after it, whose users change, have their motors stopped.
    fn gone(&mut self, key: u64) {
        if let Some(at) = self.pads.iter().position(|p| p.key == key) {
            self.pads[at].gone = true;
            self.moved_from(at + 1);
        }
    }

    /// The pads from `at` on are other users now: the game's rumble for their old places stops.
    fn moved_from(&mut self, at: usize) {
        for pad in self.pads.iter_mut().skip(at) {
            pad.wanted = (0, 0);
        }
    }
}

#[derive(Default)]
struct Shared {
    list: Mutex<List>,
    /// Told when the game sets a pad's motors.
    changed: Condvar,
}

impl Shared {
    fn list(&self) -> MutexGuard<'_, List> {
        self.list.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// The Linux pads, in their order ([`self`]); a clone shares them.
#[derive(Clone)]
pub struct LinuxPads {
    shared: Arc<Shared>,
}

/// Starts the threads that find, read and rumble the system's pads.
pub fn start() -> LinuxPads {
    start_with(Config::system())
}

fn start_with(config: Config) -> LinuxPads {
    let shared = Arc::new(Shared::default());
    let reading = shared.clone();
    let _ = std::thread::Builder::new()
        .name("nv-rs pads".into())
        .spawn(move || read_pads(&reading, &config));
    let rumbling = shared.clone();
    let _ = std::thread::Builder::new()
        .name("nv-rs pad rumble".into())
        .spawn(move || rumble_pads(&rumbling));
    LinuxPads { shared }
}

impl PadList for LinuxPads {
    fn count(&mut self) -> usize {
        self.shared.list().live().count()
    }

    fn get_state(&mut self, n: usize, state: &mut [u8; 16]) -> u32 {
        match self.shared.list().live().nth(n) {
            Some(pad) => {
                *state = pad.controls.state(pad.packet).to_bytes();
                ERROR_SUCCESS
            }
            None => ERROR_DEVICE_NOT_CONNECTED,
        }
    }

    fn set_state(&mut self, n: usize, left: u16, right: u16) -> u32 {
        match self.shared.list().live().nth(n) {
            Some(pad) => {
                if pad.wanted != (left, right) {
                    pad.wanted = (left, right);
                    self.shared.changed.notify_all();
                }
                ERROR_SUCCESS
            }
            None => ERROR_DEVICE_NOT_CONNECTED,
        }
    }
}

/// What a look made of a device.
enum Found {
    Pad(Box<Pad>),
    /// Not a pad, left out by Steam's lists, or a pad not read: not opened again while the device
    /// stays plugged in.
    Passed,
    /// Could not be opened (not the user's to read, or not there): tried again at the next look.
    Failed,
}

/// Opens the device `path` and reads what it is.
fn open(path: &Path, sys: &Path, lists: &SteamLists) -> Found {
    let flags = libc::O_NONBLOCK | libc::O_CLOEXEC;
    let (file, writable) = match OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(flags)
        .open(path)
    {
        Ok(file) => (file, true),
        Err(_) => match OpenOptions::new().read(true).custom_flags(flags).open(path) {
            Ok(file) => (file, false),
            Err(_) => return Found::Failed,
        },
    };
    let fd = file.as_raw_fd();
    let keys = bits(fd, |len| eviocgbit(evdev::EV_KEY, len), evdev::KEY_MAX);
    let abs = bits(fd, |len| eviocgbit(evdev::EV_ABS, len), evdev::ABS_MAX);
    let (Some(keys), Some(abs)) = (keys, abs) else {
        return Found::Passed;
    };
    if !evdev::is_pad(|c| has(&keys, c), |c| has(&abs, c)) {
        return Found::Passed;
    }
    // SAFETY: a plain C structure, all zeros a valid value.
    let mut id: libc::input_id = unsafe { std::mem::zeroed() };
    // SAFETY: the structure the request names.
    if unsafe { libc::ioctl(fd, EVIOCGID as _, &mut id) } < 0 {
        return Found::Passed;
    }
    let (vendor, product) = (id.vendor, id.product);
    if lists.leave_out((vendor, product)) {
        return Found::Passed;
    }
    let driver = driver(sys, path);
    let Some(faces) = evdev::faces(vendor, product, driver.as_deref()) else {
        eprintln!(
            "nv-rs: game pads: the pad {vendor:04x}:{product:04x} ({}) is not read yet: so far \
             only Xbox-style, PlayStation and Nintendo pads and Steam's virtual pad are",
            driver.as_deref().unwrap_or("no driver")
        );
        return Found::Passed;
    };
    let can_rumble = writable
        && bits(fd, |len| eviocgbit(evdev::EV_FF, len), evdev::FF_MAX)
            .is_some_and(|ff| has(&ff, evdev::FF_RUMBLE));
    let axes: Vec<u16> = (0..=evdev::ABS_MAX).filter(|&c| has(&abs, c)).collect();
    let mut ranges = vec![Range::default(); usize::from(evdev::ABS_MAX) + 1];
    let mut now = Vec::new();
    for &code in &axes {
        if let Some(info) = absinfo(fd, code) {
            ranges[usize::from(code)] = Range {
                min: info.minimum,
                max: info.maximum,
            };
            now.push((code, info.value));
        }
    }
    let mut events = evdev::Pad::new(faces, Axes::of(|c| has(&abs, c)), ranges);
    for (code, value) in now {
        events.set_abs(code, value);
    }
    read_keys(fd, &mut events);
    let controls = events.controls();
    Found::Pad(Box::new(Pad {
        key: 0,
        path: path.to_path_buf(),
        file: Arc::new(file),
        axes,
        events,
        controls,
        packet: 0,
        rank: lists.rank((vendor, product)),
        resync: false,
        can_rumble,
        wanted: (0, 0),
        sent: (0, 0),
        sent_at: None,
        effect: None,
        gone: false,
    }))
}

/// The keys as they are now (`EVIOCGKEY`).
fn read_keys(fd: i32, events: &mut evdev::Pad) {
    if let Some(down) = bits(fd, eviocgkey, evdev::KEY_MAX) {
        for code in 0..=evdev::KEY_MAX {
            events.set_key(code, has(&down, code));
        }
    }
}

/// A pad's keys and axes as the device has them now, read after the kernel dropped events.
struct Snapshot {
    keys: Option<Vec<u8>>,
    axes: Vec<(u16, i32)>,
}

impl Snapshot {
    fn read(file: &File, axes: &[u16]) -> Snapshot {
        let fd = file.as_raw_fd();
        Snapshot {
            keys: bits(fd, eviocgkey, evdev::KEY_MAX),
            axes: axes
                .iter()
                .filter_map(|&code| absinfo(fd, code).map(|info| (code, info.value)))
                .collect(),
        }
    }

    fn apply(self, events: &mut evdev::Pad) {
        if let Some(down) = self.keys {
            for code in 0..=evdev::KEY_MAX {
                events.set_key(code, has(&down, code));
            }
        }
        for (code, value) in self.axes {
            events.set_abs(code, value);
        }
    }
}

/// The pads waiting for a [`Snapshot`] read again, outside the list's lock (an `EVIOCG*` request
/// can wait behind a rumble upload to Steam's virtual pad), then applied and read out.
fn resync_pads(shared: &Shared) {
    let waiting: Vec<(u64, Arc<File>, Vec<u16>)> = shared
        .list()
        .live()
        .filter(|p| p.resync)
        .map(|p| (p.key, p.file.clone(), p.axes.clone()))
        .collect();
    for (key, file, axes) in waiting {
        let snapshot = Snapshot::read(&file, &axes);
        let mut list = shared.list();
        if let Some(pad) = list.pads.iter_mut().find(|p| p.key == key) {
            snapshot.apply(&mut pad.events);
            pad.resync = false;
            pad.report();
        }
    }
}

/// One look through the event devices: the pads gone forgotten, the new ones opened and added.
/// `passed` holds the devices passed over, by path, with the file's inode (a device plugged in
/// later under the same path is another file).
fn look(shared: &Shared, config: &Config, passed: &mut HashMap<PathBuf, u64>) {
    let known: Vec<PathBuf> = {
        let mut list = shared.list();
        list.pads.retain(|p| !p.gone);
        list.pads.iter().map(|p| p.path.clone()).collect()
    };
    let devices = (config.devices)();
    passed.retain(|path, _| devices.contains(path));
    let mut new = Vec::new();
    for path in devices {
        if known.contains(&path) {
            continue;
        }
        let Ok(inode) = std::fs::metadata(&path).map(|m| m.ino()) else {
            continue;
        };
        if passed.get(&path) == Some(&inode) {
            continue;
        }
        match open(&path, &config.sys, &config.lists) {
            Found::Pad(pad) => new.push(pad),
            Found::Passed => {
                passed.insert(path, inode);
            }
            Found::Failed => {}
        }
    }
    if !new.is_empty() {
        let mut list = shared.list();
        for pad in new {
            list.add(*pad);
        }
        // Motors to stop for the pads whose users changed.
        shared.changed.notify_all();
    }
}

/// The reading thread: a look every [`LOOK_EVERY`], and in between each pad's events as they come.
fn read_pads(shared: &Shared, config: &Config) {
    let mut passed = HashMap::new();
    let mut last_look: Option<Instant> = None;
    loop {
        if !matches!(last_look, Some(at) if at.elapsed() < LOOK_EVERY) {
            look(shared, config, &mut passed);
            last_look = Some(Instant::now());
        }
        let keys_and_fds: Vec<(u64, i32)> = shared
            .list()
            .live()
            .map(|p| (p.key, p.file.as_raw_fd()))
            .collect();
        if keys_and_fds.is_empty() {
            std::thread::sleep(LOOK_EVERY);
            continue;
        }
        let mut fds: Vec<libc::pollfd> = keys_and_fds
            .iter()
            .map(|&(_, fd)| libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        // SAFETY: a live array of `pollfd`s and its length. The files stay open: only this
        // thread takes pads out of the list.
        let n = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, POLL_MS) };
        if n <= 0 {
            continue;
        }
        {
            let mut list = shared.list();
            let mut gone = Vec::new();
            for (polled, &(key, _)) in fds.iter().zip(&keys_and_fds) {
                if polled.revents == 0 {
                    continue;
                }
                if let Some(pad) = list.pads.iter_mut().find(|p| p.key == key) {
                    let error = libc::POLLERR | libc::POLLHUP | libc::POLLNVAL;
                    if polled.revents & error != 0 || !read_events(pad) {
                        gone.push(key);
                    }
                }
            }
            for key in &gone {
                list.gone(*key);
            }
            if !gone.is_empty() {
                shared.changed.notify_all();
            }
        }
        resync_pads(shared);
    }
}

/// Reads all the pad has sent; `false` when the device has gone (pulled out). After the kernel
/// dropped events, the report that ends them leaves the rest unread: the events after it in this
/// read, and those still queued, are thrown away (as libevdev does) and the pad waits for its
/// device's state to be read again ([`resync_pads`]), as that state already holds them.
fn read_events(pad: &mut Pad) -> bool {
    let size = std::mem::size_of::<libc::input_event>();
    let mut buffer = vec![0u8; size * 64];
    loop {
        // SAFETY: a buffer of the length passed.
        let n = unsafe {
            libc::read(
                pad.file.as_raw_fd(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };
        if n < 0 {
            match std::io::Error::last_os_error().kind() {
                std::io::ErrorKind::WouldBlock => return true,
                std::io::ErrorKind::Interrupted => continue,
                _ => return false,
            }
        }
        if n == 0 {
            return false;
        }
        if pad.resync {
            // Throwing away what was queued before the state is read again.
            continue;
        }
        for chunk in buffer[..n as usize].chunks_exact(size) {
            // SAFETY: `chunk` holds one whole event the kernel wrote.
            let event: libc::input_event =
                unsafe { std::ptr::read_unaligned(chunk.as_ptr().cast()) };
            match pad.events.event(event.type_, event.code, event.value) {
                Step::Nothing => {}
                Step::Report => pad.report(),
                Step::Resync => {
                    pad.resync = true;
                    break;
                }
            }
        }
    }
}

/// The rumble thread: sends each pad's motor speeds when the game changes them, and a rumble
/// still running again before the kernel ends it.
fn rumble_pads(shared: &Shared) {
    loop {
        let due = {
            let mut list = shared.list();
            loop {
                let now = Instant::now();
                let due: Vec<Rumble> = list.pads.iter().filter_map(|p| p.rumble_due(now)).collect();
                if !due.is_empty() {
                    break due;
                }
                let wait = list
                    .pads
                    .iter()
                    .filter_map(Pad::resend_at)
                    .min()
                    .map_or(Duration::from_secs(3600), |at| {
                        at.saturating_duration_since(now)
                    });
                list = shared
                    .changed
                    .wait_timeout(list, wait)
                    .unwrap_or_else(|p| p.into_inner())
                    .0;
            }
        };
        for rumble in due {
            let (effect, sent) = send(&rumble.file, rumble.effect, rumble.speeds);
            let mut list = shared.list();
            if let Some(pad) = list.pads.iter_mut().find(|p| p.key == rumble.key) {
                // A failed send is tried again at the next change or resend, not at once.
                pad.sent = rumble.speeds;
                pad.sent_at = Some(Instant::now());
                pad.effect = effect;
                pad.can_rumble &= sent != Sent::Gone;
            }
        }
    }
}

/// How a send went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sent {
    Done,
    Failed,
    /// The device has gone (`ENODEV`): its rumble ends.
    Gone,
}

/// The last call's failure as a [`Sent`].
fn failure() -> Sent {
    if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENODEV) {
        Sent::Gone
    } else {
        Sent::Failed
    }
}

/// Sends the motor speeds `left` (the strong motor) and `right` (the weak one) to the pad: the
/// effect uploaded (`EVIOCSFF`; `effect` is its id once it has one) and played, or stopped for 0
/// and 0. An upload of the effect the pad had that fails is tried once more as a new effect, as
/// SDL does (the effect may have been taken away). Answers the effect's id and how it went.
fn send(file: &File, effect: Option<i16>, (left, right): (u16, u16)) -> (Option<i16>, Sent) {
    let fd = file.as_raw_fd();
    if (left, right) == (0, 0) {
        return match effect {
            Some(id) => (effect, play(fd, id, false)),
            None => (effect, Sent::Done),
        };
    }
    // SAFETY: a plain C structure, all zeros a valid value (no direction, trigger or delay).
    let mut e: libc::ff_effect = unsafe { std::mem::zeroed() };
    e.type_ = evdev::FF_RUMBLE;
    e.id = effect.unwrap_or(-1);
    e.replay.length = RUMBLE_LENGTH;
    // `ff_rumble_effect`, at the start of the union: the strong and the weak motor's magnitudes.
    let magnitudes = std::ptr::addr_of_mut!(e.u).cast::<u16>();
    // SAFETY: two 16-bit words inside the union, which is at least eight bytes and aligned to 8.
    unsafe {
        magnitudes.write(left);
        magnitudes.add(1).write(right);
    }
    // SAFETY: the structure the request names; the kernel writes the effect's id into it.
    if unsafe { libc::ioctl(fd, EVIOCSFF as _, &mut e) } < 0 {
        let first = failure();
        if effect.is_none() || first == Sent::Gone {
            return (effect, first);
        }
        e.id = -1;
        // SAFETY: as above, as a new effect.
        if unsafe { libc::ioctl(fd, EVIOCSFF as _, &mut e) } < 0 {
            return (effect, failure());
        }
    }
    (Some(e.id), play(fd, e.id, true))
}

/// Plays (`on`) or stops the effect `id`: one `EV_FF` event written to the device.
fn play(fd: i32, id: i16, on: bool) -> Sent {
    // SAFETY: a plain C structure, all zeros a valid value.
    let mut event: libc::input_event = unsafe { std::mem::zeroed() };
    event.type_ = evdev::EV_FF;
    event.code = id as u16;
    event.value = i32::from(on);
    let size = std::mem::size_of::<libc::input_event>();
    // SAFETY: one whole event from a live structure.
    let n = unsafe { libc::write(fd, std::ptr::addr_of!(event).cast(), size) };
    if n == size as isize {
        Sent::Done
    } else {
        failure()
    }
}

#[cfg(test)]
mod tests;
