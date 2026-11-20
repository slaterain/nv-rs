//! The Linux reader's tests. The pads are virtual ones the tests make through the kernel's uinput
//! (`/dev/uinput`, which only root may open on most systems): the reader opens, reads and rumbles
//! them as it would a real pad's event device. Without uinput those tests say so and pass.

use super::*;
use crate::xinput::buttons;
use crate::xinput::evdev::*;
use crate::xinput::sdl_hints::{hint, ids, yes};

/// The request numbers as linux/input.h and linux/uinput.h make them on x86-64 and ARM64
/// (printed by a C program built against the kernel's headers, 6.18).
#[test]
fn the_ioctl_numbers() {
    assert_eq!(EVIOCGID, 0x8008_4502);
    assert_eq!(eviocgbit(0x01, 96), 0x8060_4521);
    assert_eq!(eviocgkey(96), 0x8060_4518);
    assert_eq!(eviocgabs(0x01), 0x8018_4541);
    assert_eq!(EVIOCSFF, 0x4030_4580);
    assert_eq!(UI_DEV_CREATE, 0x5501);
    assert_eq!(UI_DEV_DESTROY, 0x5502);
    assert_eq!(UI_DEV_SETUP, 0x405c_5503);
    assert_eq!(UI_ABS_SETUP, 0x401c_5504);
    assert_eq!(UI_SET_EVBIT, 0x4004_5564);
    assert_eq!(UI_SET_KEYBIT, 0x4004_5565);
    assert_eq!(UI_SET_ABSBIT, 0x4004_5567);
    assert_eq!(UI_SET_FFBIT, 0x4004_556b);
    assert_eq!(UI_GET_SYSNAME_64, 0x8040_552c);
    assert_eq!(UI_BEGIN_FF_UPLOAD, 0xc068_55c8);
    assert_eq!(UI_END_FF_UPLOAD, 0x4068_55c9);
    assert_eq!(std::mem::size_of::<libc::input_event>(), 24);
    assert_eq!(std::mem::size_of::<libc::ff_effect>(), 48);
}

/// Steam's lists as SDL reads them: pairs of `0x` numbers whatever lies between, or a file's.
#[test]
fn steams_lists() {
    assert_eq!(
        ids("0x28de/0x1205,0x045e/0x028e"),
        vec![(0x28de, 0x1205), (0x045e, 0x028e)]
    );
    assert_eq!(ids(" 0x28DE/0x11FF ; 0x054c"), vec![(0x28de, 0x11ff)]);
    assert_eq!(ids("28de/1205"), vec![]);
    assert_eq!(ids("0x12345/0x0001,0x045e/0x02ea"), vec![(0x045e, 0x02ea)]);
    assert_eq!(ids(""), vec![]);
    assert_eq!(hint(None), vec![]);
    let file = std::env::temp_dir().join(format!("nv-rs-pad-list-{}", std::process::id()));
    std::fs::write(&file, "0x057e/0x2009\n").unwrap();
    assert_eq!(
        hint(Some(format!("@{}", file.display()))),
        vec![(0x057e, 0x2009)]
    );
    std::fs::remove_file(&file).unwrap();
    assert_eq!(hint(Some(format!("@{}", file.display()))), vec![]);

    let lists = SteamLists {
        ignore: vec![(0x28de, 0x1205)],
        ..SteamLists::default()
    };
    assert!(lists.leave_out((0x28de, 0x1205)));
    assert!(!lists.leave_out((0x28de, 0x11ff)));
    let lists = SteamLists {
        except: vec![(0x28de, 0x11ff)],
        ..SteamLists::default()
    };
    assert!(lists.leave_out((0x045e, 0x028e)));
    assert!(!lists.leave_out((0x28de, 0x11ff)));
    // Neither list ever leaves out Steam's virtual pad, as in SDL.
    let lists = SteamLists {
        ignore: vec![(0x28de, 0x11ff)],
        ..SteamLists::default()
    };
    assert!(!lists.leave_out((0x28de, 0x11ff)));
    let lists = SteamLists {
        except: vec![(0x045e, 0x028e)],
        ..SteamLists::default()
    };
    assert!(!lists.leave_out((0x28de, 0x11ff)));
    assert!(!SteamLists::default().leave_out((0x28de, 0x11ff)));
}

/// SDL's yes or no for a variable, and where each pad comes by it.
#[test]
fn steams_variable_and_the_order() {
    for (value, expected) in [
        (None, false),
        (Some(""), false),
        (Some("1"), true),
        (Some("true"), true),
        (Some("yes"), true),
        (Some("0"), false),
        (Some("00"), false),
        (Some("0x1"), false),
        (Some("false"), false),
        (Some("FALSE"), false),
        (Some("False "), true),
    ] {
        assert_eq!(yes(value, false), expected, "{value:?}");
    }
    assert!(yes(None, true) && yes(Some(""), true));
    let started_by_steam = SteamLists {
        allow_virtual: true,
        ..SteamLists::default()
    };
    assert_eq!(started_by_steam.rank((0x28de, 0x11ff)), Rank::SteamFirst);
    assert_eq!(started_by_steam.rank((0x045e, 0x028e)), Rank::Pad);
    assert_eq!(
        SteamLists::default().rank((0x28de, 0x11ff)),
        Rank::SteamLast
    );
    assert!(Rank::SteamFirst < Rank::Pad && Rank::Pad < Rank::SteamLast);
}

/// Pads added in [`Rank`] order whatever order they come in; the pads a new one goes before, and
/// those after one gone, have their motors stopped.
#[test]
fn the_list_order_and_stopped_motors() {
    let mut list = List::default();
    let pad = |rank: Rank, wanted: (u16, u16)| {
        let mut p = stand_in(Path::new("/dev/null"));
        p.rank = rank;
        p.wanted = wanted;
        p
    };
    list.add(pad(Rank::SteamLast, (1, 1)));
    list.add(pad(Rank::Pad, (2, 2)));
    list.add(pad(Rank::Pad, (3, 3)));
    list.add(pad(Rank::SteamFirst, (4, 4)));
    let ranks: Vec<Rank> = list.pads.iter().map(|p| p.rank).collect();
    assert_eq!(
        ranks,
        [Rank::SteamFirst, Rank::Pad, Rank::Pad, Rank::SteamLast]
    );
    let keys: Vec<u64> = list.pads.iter().map(|p| p.key).collect();
    assert_eq!(keys, [3, 1, 2, 0]);
    // The first Steam pad went before all three: their rumble stopped; its own kept.
    let wanted: Vec<(u16, u16)> = list.pads.iter().map(|p| p.wanted).collect();
    assert_eq!(wanted, [(4, 4), (0, 0), (0, 0), (0, 0)]);
    for p in &mut list.pads {
        p.wanted = (9, 9);
    }
    list.gone(1);
    let wanted: Vec<(u16, u16)> = list.pads.iter().map(|p| p.wanted).collect();
    assert_eq!(wanted, [(9, 9), (9, 9), (0, 0), (0, 0)]);
    assert!(list.pads[1].gone);
    // The last pad gone: nobody moves.
    for p in &mut list.pads {
        p.wanted = (9, 9);
    }
    list.gone(0);
    assert!(list.pads.iter().all(|p| p.wanted == (9, 9)));
}

/// A device's driver is the one its parent device names; a virtual device has none.
#[test]
fn the_driver_named_in_sys() {
    let sys = std::env::temp_dir().join(format!("nv-rs-pad-sys-{}", std::process::id()));
    let parent = sys.join("event7/device/device");
    std::fs::create_dir_all(&parent).unwrap();
    std::os::unix::fs::symlink("../../../../bus/usb/drivers/xpad", parent.join("driver")).unwrap();
    std::fs::create_dir_all(sys.join("event8/device")).unwrap();
    assert_eq!(
        driver(&sys, Path::new("/dev/input/event7")).as_deref(),
        Some("xpad")
    );
    assert_eq!(driver(&sys, Path::new("/dev/input/event8")), None);
    assert_eq!(driver(&sys, Path::new("/dev/input/event9")), None);
    std::fs::remove_dir_all(&sys).unwrap();
}

/// The event devices by number, not by name.
#[test]
fn event_devices_by_number() {
    let dir = std::env::temp_dir().join(format!("nv-rs-pad-dev-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["event10", "event2", "js0", "mouse0", "event1", "eventx"] {
        std::fs::write(dir.join(name), "").unwrap();
    }
    let names: Vec<String> = event_devices(&dir)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["event1", "event2", "event10"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

const NONE: u64 = 0;
const UI_DEV_CREATE: u64 = ioc(NONE, b'U', 1, 0);
const UI_DEV_DESTROY: u64 = ioc(NONE, b'U', 2, 0);
const UI_DEV_SETUP: u64 = ioc(WRITE, b'U', 3, std::mem::size_of::<libc::uinput_setup>());
const UI_ABS_SETUP: u64 = ioc(
    WRITE,
    b'U',
    4,
    std::mem::size_of::<libc::uinput_abs_setup>(),
);
const UI_SET_EVBIT: u64 = ioc(WRITE, b'U', 100, 4);
const UI_SET_KEYBIT: u64 = ioc(WRITE, b'U', 101, 4);
const UI_SET_ABSBIT: u64 = ioc(WRITE, b'U', 103, 4);
const UI_SET_FFBIT: u64 = ioc(WRITE, b'U', 107, 4);
const UI_GET_SYSNAME_64: u64 = ioc(READ, b'U', 44, 64);
const UI_BEGIN_FF_UPLOAD: u64 = ioc(
    READ | WRITE,
    b'U',
    200,
    std::mem::size_of::<libc::uinput_ff_upload>(),
);
const UI_END_FF_UPLOAD: u64 = ioc(
    WRITE,
    b'U',
    201,
    std::mem::size_of::<libc::uinput_ff_upload>(),
);
const EV_UINPUT: u16 = 0x0101;
const UI_FF_UPLOAD: u16 = 1;

/// A virtual pad made through uinput, laid out as xpad lays out an Xbox 360 pad (as Steam's
/// virtual pad is); destroyed when dropped.
struct Virtual {
    uinput: File,
    node: PathBuf,
}

impl Virtual {
    /// `None` when uinput cannot be opened here (not root, or no uinput).
    fn make(vendor: u16, product: u16, rumble: bool) -> Option<Virtual> {
        let uinput = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open("/dev/uinput")
            .ok()?;
        let fd = uinput.as_raw_fd();
        let set = |request: u64, value: u16| {
            // SAFETY: the uinput requests that take an int.
            assert!(unsafe { libc::ioctl(fd, request as _, libc::c_int::from(value)) } >= 0);
        };
        set(UI_SET_EVBIT, EV_KEY);
        for key in [
            BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST, BTN_TL, BTN_TR, BTN_SELECT, BTN_START,
            BTN_MODE, BTN_THUMBL, BTN_THUMBR,
        ] {
            set(UI_SET_KEYBIT, key);
        }
        set(UI_SET_EVBIT, EV_ABS);
        for (code, min, max) in [
            (ABS_X, -32768, 32767),
            (ABS_Y, -32768, 32767),
            (ABS_RX, -32768, 32767),
            (ABS_RY, -32768, 32767),
            (ABS_Z, 0, 255),
            (ABS_RZ, 0, 255),
            (ABS_HAT0X, -1, 1),
            (ABS_HAT0Y, -1, 1),
        ] {
            set(UI_SET_ABSBIT, code);
            // SAFETY: plain C structures.
            let mut abs: libc::uinput_abs_setup = unsafe { std::mem::zeroed() };
            abs.code = code;
            abs.absinfo.minimum = min;
            abs.absinfo.maximum = max;
            // SAFETY: the structure the request names.
            assert!(unsafe { libc::ioctl(fd, UI_ABS_SETUP as _, &abs) } >= 0);
        }
        if rumble {
            set(UI_SET_EVBIT, EV_FF);
            set(UI_SET_FFBIT, FF_RUMBLE);
        }
        // SAFETY: a plain C structure.
        let mut setup: libc::uinput_setup = unsafe { std::mem::zeroed() };
        setup.id = libc::input_id {
            bustype: 0x03,
            vendor,
            product,
            version: 1,
        };
        for (to, from) in setup.name.iter_mut().zip(b"nv-rs test pad") {
            *to = libc::c_char::from_ne_bytes([*from]);
        }
        // Room for two effects: a failed upload tried again as a new effect has a place.
        setup.ff_effects_max = 2 * u32::from(rumble);
        // SAFETY: the structure the request names, then the request without an argument.
        assert!(unsafe { libc::ioctl(fd, UI_DEV_SETUP as _, &setup) } >= 0);
        assert!(unsafe { libc::ioctl(fd, UI_DEV_CREATE as _) } >= 0);
        let mut name = [0u8; 64];
        // SAFETY: the buffer is the request's 64 bytes.
        assert!(unsafe { libc::ioctl(fd, UI_GET_SYSNAME_64 as _, name.as_mut_ptr()) } >= 0);
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        let input = std::str::from_utf8(&name[..end]).unwrap().to_string();
        // Closing the uinput file (a panic here) takes the device away too.
        let node = wait_for(|| {
            let entries = std::fs::read_dir(Path::new("/sys/class/input").join(&input)).ok()?;
            let event = entries
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .find(|n| n.starts_with("event"))?;
            let node = Path::new("/dev/input").join(event);
            node.exists().then_some(node)
        })
        .expect("the virtual pad's event device");
        Some(Virtual { uinput, node })
    }

    fn emit(&self, kind: u16, code: u16, value: i32) {
        // SAFETY: a plain C structure.
        let mut event: libc::input_event = unsafe { std::mem::zeroed() };
        event.type_ = kind;
        event.code = code;
        event.value = value;
        assert!(play_like_write(self.uinput.as_raw_fd(), &event));
    }

    /// The next event the kernel hands the pad's maker (a rumble request or a played effect),
    /// within two seconds.
    fn next(&self) -> Option<libc::input_event> {
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            let mut fds = [libc::pollfd {
                fd: self.uinput.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            }];
            // SAFETY: one live `pollfd`.
            if unsafe { libc::poll(fds.as_mut_ptr(), 1, 100) } <= 0 {
                continue;
            }
            // SAFETY: a plain C structure, read whole.
            let mut event: libc::input_event = unsafe { std::mem::zeroed() };
            let size = std::mem::size_of::<libc::input_event>();
            // SAFETY: the structure's own bytes.
            let n = unsafe {
                libc::read(
                    self.uinput.as_raw_fd(),
                    std::ptr::addr_of_mut!(event).cast(),
                    size,
                )
            };
            if n == size as isize {
                return Some(event);
            }
        }
        None
    }

    /// Answers the next rumble upload: the effect the reader uploaded.
    fn take_upload(&self) -> libc::ff_effect {
        self.answer_upload(0)
    }

    /// Answers the next rumble upload with `retval` (0 taken, a negative error refused).
    fn answer_upload(&self, retval: i32) -> libc::ff_effect {
        let request = self.next().expect("an upload request");
        assert_eq!((request.type_, request.code), (EV_UINPUT, UI_FF_UPLOAD));
        // SAFETY: a plain C structure.
        let mut upload: libc::uinput_ff_upload = unsafe { std::mem::zeroed() };
        upload.request_id = request.value as u32;
        let fd = self.uinput.as_raw_fd();
        // SAFETY: the structure the requests name.
        assert!(unsafe { libc::ioctl(fd, UI_BEGIN_FF_UPLOAD as _, &mut upload) } >= 0);
        upload.retval = retval;
        assert!(unsafe { libc::ioctl(fd, UI_END_FF_UPLOAD as _, &upload) } >= 0);
        upload.effect
    }
}

impl Drop for Virtual {
    fn drop(&mut self) {
        // SAFETY: the request without an argument.
        unsafe { libc::ioctl(self.uinput.as_raw_fd(), UI_DEV_DESTROY as _) };
    }
}

fn play_like_write(fd: i32, event: &libc::input_event) -> bool {
    let size = std::mem::size_of::<libc::input_event>();
    // SAFETY: one whole event from a live structure.
    let n = unsafe { libc::write(fd, (event as *const libc::input_event).cast(), size) };
    n == size as isize
}

/// `f`'s first answer within three seconds.
fn wait_for<T>(mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The reader over only `nodes` (not the machine's own devices).
fn reader(nodes: Vec<PathBuf>, lists: SteamLists) -> LinuxPads {
    reader_over(Arc::new(Mutex::new(nodes)), lists)
}

/// [`reader`] over a list the test may add to.
fn reader_over(nodes: Arc<Mutex<Vec<PathBuf>>>, lists: SteamLists) -> LinuxPads {
    start_with(Config {
        devices: Box::new(move || nodes.lock().unwrap().clone()),
        sys: PathBuf::from("/sys/class/input"),
        lists,
    })
}

fn state(pads: &mut LinuxPads, n: usize) -> Option<crate::xinput::PadState> {
    let mut s = [0u8; 16];
    (pads.get_state(n, &mut s) == ERROR_SUCCESS).then(|| crate::xinput::PadState::from_bytes(&s))
}

/// No uinput here: the test says so and passes, unless `NV_REQUIRE_UINPUT=1` asks for it to fail
/// (the runs that have uinput set it, so a test that cannot make its pad never passes unseen).
fn no_uinput() {
    assert!(
        std::env::var("NV_REQUIRE_UINPUT").as_deref() != Ok("1"),
        "NV_REQUIRE_UINPUT=1 and no /dev/uinput to make a virtual pad with"
    );
    eprintln!("skipped: no /dev/uinput to make a virtual pad with (run as root where it is)");
}

/// The pads Steam starts a game with: its virtual pad first.
fn started_by_steam() -> SteamLists {
    SteamLists {
        allow_virtual: true,
        ..SteamLists::default()
    }
}

/// Steam's virtual pad, read end to end: found, its face buttons by letter, its sticks and
/// triggers, a new packet at each change, rumble uploaded, played and stopped, and gone when it
/// is unplugged.
#[test]
fn a_virtual_steam_pad_end_to_end() {
    let Some(pad) = Virtual::make(0x28de, 0x11ff, true) else {
        return no_uinput();
    };
    let mut pads = reader(vec![pad.node.clone()], SteamLists::default());
    assert!(wait_for(|| (pads.count() == 1).then_some(())).is_some());
    let first = state(&mut pads, 0).unwrap();
    assert_eq!(first.buttons, 0);
    assert_eq!(state(&mut pads, 1), None);

    pad.emit(EV_KEY, BTN_NORTH, 1);
    pad.emit(EV_ABS, ABS_Y, -32768);
    pad.emit(EV_ABS, ABS_RX, 1234);
    pad.emit(EV_ABS, ABS_RZ, 255);
    pad.emit(EV_SYN, SYN_REPORT, 0);
    let s = wait_for(|| state(&mut pads, 0).filter(|s| s.buttons != 0)).unwrap();
    assert_eq!(s.buttons, buttons::X);
    assert_eq!(
        (s.thumb_ly, s.thumb_rx, s.right_trigger),
        (32767, 1234, 255)
    );
    assert_eq!(s.packet, first.packet.wrapping_add(1));
    // An event without its report changes nothing yet.
    pad.emit(EV_KEY, BTN_SOUTH, 1);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(state(&mut pads, 0).unwrap().buttons, buttons::X);
    pad.emit(EV_SYN, SYN_REPORT, 0);
    let s = wait_for(|| state(&mut pads, 0).filter(|s| s.buttons & buttons::A != 0)).unwrap();
    assert_eq!(s.buttons, buttons::X | buttons::A);

    // Rumble: the game's speeds (strong, weak) uploaded as the kernel's rumble effect, played,
    // then stopped.
    assert_eq!(pads.set_state(0, 0xc000, 0x2000), ERROR_SUCCESS);
    let effect = pad.take_upload();
    assert_eq!(effect.type_, FF_RUMBLE);
    assert_eq!(effect.replay.length, 0xffff);
    // SAFETY: the union's first two 16-bit words (`ff_rumble_effect`).
    let magnitudes = unsafe {
        let u = std::ptr::addr_of!(effect.u).cast::<u16>();
        (u.read(), u.add(1).read())
    };
    assert_eq!(magnitudes, (0xc000, 0x2000));
    let played = pad.next().expect("the effect played");
    assert_eq!(
        (played.type_, played.code, played.value),
        (EV_FF, effect.id as u16, 1)
    );
    // The same speeds again send nothing.
    assert_eq!(pads.set_state(0, 0xc000, 0x2000), ERROR_SUCCESS);
    assert_eq!(pads.set_state(0, 0, 0), ERROR_SUCCESS);
    let stopped = pad.next().expect("the effect stopped");
    assert_eq!(
        (stopped.type_, stopped.code, stopped.value),
        (EV_FF, effect.id as u16, 0)
    );
    // New speeds update the same effect.
    assert_eq!(pads.set_state(0, 0x0100, 0), ERROR_SUCCESS);
    let again = pad.take_upload();
    assert_eq!(again.id, effect.id);
    assert_eq!(pad.next().map(|e| (e.type_, e.value)), Some((EV_FF, 1)));
    assert_eq!(pads.set_state(1, 1, 1), ERROR_DEVICE_NOT_CONNECTED);

    drop(pad);
    assert!(wait_for(|| (pads.count() == 0).then_some(())).is_some());
    assert_eq!(state(&mut pads, 0), None);
}

/// Steam's virtual pad comes first, before a pad found earlier; a pad Steam's list names is not
/// read; a pad without the rumble effect takes the game's rumble and sends nothing.
#[test]
fn steams_pad_first_and_its_lists() {
    let Some(xbox) = Virtual::make(0x045e, 0x028e, false) else {
        return no_uinput();
    };
    let mut pads = reader(vec![xbox.node.clone()], SteamLists::default());
    assert!(wait_for(|| (pads.count() == 1).then_some(())).is_some());
    xbox.emit(EV_KEY, BTN_WEST, 1);
    xbox.emit(EV_SYN, SYN_REPORT, 0);
    assert!(wait_for(|| state(&mut pads, 0).filter(|s| s.buttons == buttons::Y)).is_some());
    assert_eq!(pads.set_state(0, 0xffff, 0xffff), ERROR_SUCCESS);
    assert!(xbox.next().is_none(), "no rumble for a pad without it");
    drop(pads);

    // Started by Steam: the Xbox pad found first (its Y still held, read as the device has it at
    // the start), a PlayStation pad Steam's list names, then Steam's virtual pad at a later look,
    // which comes first; the ignore list naming the virtual pad too leaves it in.
    let sony = Virtual::make(0x054c, 0x0ce6, false).unwrap();
    let nodes = Arc::new(Mutex::new(vec![xbox.node.clone(), sony.node.clone()]));
    let mut pads = reader_over(
        nodes.clone(),
        SteamLists {
            ignore: vec![(0x054c, 0x0ce6), (0x28de, 0x11ff)],
            ..started_by_steam()
        },
    );
    assert!(wait_for(|| (pads.count() == 1).then_some(())).is_some());
    assert_eq!(state(&mut pads, 0).unwrap().buttons, buttons::Y);
    let steam = Virtual::make(0x28de, 0x11ff, false).unwrap();
    nodes.lock().unwrap().push(steam.node.clone());
    assert!(wait_for(|| (pads.count() == 2).then_some(())).is_some());
    std::thread::sleep(Duration::from_millis(1200));
    assert_eq!(pads.count(), 2, "the pad Steam's list names is left out");
    steam.emit(EV_KEY, BTN_START, 1);
    steam.emit(EV_SYN, SYN_REPORT, 0);
    assert!(wait_for(|| state(&mut pads, 0).filter(|s| s.buttons == buttons::START)).is_some());
    assert_eq!(state(&mut pads, 1).unwrap().buttons, buttons::Y);

    // Started by Steam with an except list without the virtual pad: it is still read, first.
    let mut pads = reader(
        vec![xbox.node.clone(), steam.node.clone()],
        SteamLists {
            except: vec![(0x045e, 0x028e)],
            ..started_by_steam()
        },
    );
    assert!(wait_for(|| (pads.count() == 2).then_some(())).is_some());
    assert_eq!(state(&mut pads, 0).unwrap().buttons, buttons::START);
    assert_eq!(state(&mut pads, 1).unwrap().buttons, buttons::Y);

    // Not started by Steam (no variable): the virtual pad after the Xbox pad, which is user 0
    // even though the virtual pad was found first.
    let mut pads = reader(
        vec![steam.node.clone(), xbox.node.clone()],
        SteamLists::default(),
    );
    assert!(wait_for(|| (pads.count() == 2).then_some(())).is_some());
    assert_eq!(state(&mut pads, 0).unwrap().buttons, buttons::Y);
    assert_eq!(state(&mut pads, 1).unwrap().buttons, buttons::START);
}

/// A pad pushed off user 0 by Steam's virtual pad (Steam started after the game) has its motors
/// stopped: the game's rumble now goes to the virtual pad.
#[test]
fn a_pad_pushed_off_user_0_stops_rumbling() {
    let Some(xbox) = Virtual::make(0x045e, 0x028e, true) else {
        return no_uinput();
    };
    let nodes = Arc::new(Mutex::new(vec![xbox.node.clone()]));
    let mut pads = reader_over(nodes.clone(), started_by_steam());
    assert!(wait_for(|| (pads.count() == 1).then_some(())).is_some());
    assert_eq!(pads.set_state(0, 0x8000, 0x8000), ERROR_SUCCESS);
    let effect = xbox.take_upload();
    assert_eq!(xbox.next().map(|e| (e.type_, e.value)), Some((EV_FF, 1)));
    let steam = Virtual::make(0x28de, 0x11ff, false).unwrap();
    nodes.lock().unwrap().push(steam.node.clone());
    let stopped = xbox.next().expect("the pushed pad's motors stopped");
    assert_eq!(
        (stopped.type_, stopped.code, stopped.value),
        (EV_FF, effect.id as u16, 0)
    );
    assert_eq!(pads.count(), 2);
}

/// A device that is not a pad, and another maker's pad that no driver of the known ones reads,
/// are passed over.
#[test]
fn not_pads_and_pads_not_read() {
    let Some(other) = Virtual::make(0x2dc8, 0x6001, false) else {
        return no_uinput();
    };
    let mut pads = reader(vec![other.node.clone()], SteamLists::default());
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(pads.count(), 0);
    let sys = Path::new("/sys/class/input");
    assert!(matches!(
        open(&other.node, sys, &SteamLists::default()),
        Found::Passed
    ));
    assert!(matches!(
        open(Path::new("/dev/null"), sys, &SteamLists::default()),
        Found::Passed
    ));
    assert!(matches!(
        open(
            Path::new("/dev/input/no-such-device"),
            sys,
            &SteamLists::default()
        ),
        Found::Failed
    ));
}

/// After dropped events the keys and axes are read again from the device as they are now, not
/// from the events (which the reader passed over).
#[test]
fn a_resync_reads_the_device_as_it_is() {
    let Some(pad) = Virtual::make(0x045e, 0x028e, false) else {
        return no_uinput();
    };
    let sys = Path::new("/sys/class/input");
    let Found::Pad(mut opened) = open(&pad.node, sys, &SteamLists::default()) else {
        panic!("the virtual pad is read");
    };
    assert!(!opened.events.controls().south);
    pad.emit(EV_KEY, BTN_SOUTH, 1);
    pad.emit(EV_ABS, ABS_RY, -32768);
    pad.emit(EV_SYN, SYN_REPORT, 0);
    Snapshot::read(&opened.file, &opened.axes).apply(&mut opened.events);
    let c = opened.events.controls();
    assert!(c.south);
    assert_eq!(c.right_y, 32767);
}

/// What the reading thread does for one pad: its events read, then, after dropped events, its
/// state read again (outside the list's lock there).
fn read_as_the_reader(pad: &mut super::Pad) {
    assert!(read_events(pad));
    if pad.resync {
        Snapshot::read(&pad.file, &pad.axes).apply(&mut pad.events);
        pad.resync = false;
        pad.report();
    }
}

/// A pad in the list with no device behind it (its file `/dev/null`), at `path`.
fn stand_in(path: &Path) -> super::Pad {
    let events = evdev::Pad::new(evdev::Faces::ByLetter, Axes::of(|_| true), Vec::new());
    super::Pad {
        key: 0,
        path: path.to_path_buf(),
        file: Arc::new(File::open("/dev/null").unwrap()),
        axes: Vec::new(),
        controls: events.controls(),
        events,
        packet: 0,
        rank: Rank::Pad,
        resync: false,
        can_rumble: true,
        wanted: (0, 0),
        sent: (0, 0),
        sent_at: None,
        effect: None,
        gone: false,
    }
}

/// A pad gone is forgotten at the next look, so another device under its path is opened; a
/// device passed over is not opened again while its file stays the same, and a new file under
/// the same path (another device plugged in) is looked at again.
#[test]
fn a_device_passed_over_is_looked_at_again_when_its_file_changes() {
    let dir = std::env::temp_dir().join(format!("nv-rs-pad-look-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let node = dir.join("event0");
    std::fs::write(&node, "").unwrap();
    let shared = Shared::default();
    let mut gone = stand_in(&node);
    gone.gone = true;
    shared.list().add(gone);
    let listed = Arc::new(Mutex::new(vec![node.clone()]));
    let devices = listed.clone();
    let config = Config {
        devices: Box::new(move || devices.lock().unwrap().clone()),
        sys: dir.clone(),
        lists: SteamLists::default(),
    };
    let mut passed = HashMap::new();
    look(&shared, &config, &mut passed);
    let first = std::fs::metadata(&node).unwrap().ino();
    assert_eq!(passed.get(&node), Some(&first));
    // A new file made before the old one goes, so it is another inode, then put in its place.
    let other = dir.join("event0.new");
    std::fs::write(&other, "").unwrap();
    std::fs::rename(&other, &node).unwrap();
    let second = std::fs::metadata(&node).unwrap().ino();
    assert_ne!(first, second);
    look(&shared, &config, &mut passed);
    assert_eq!(passed.get(&node), Some(&second));
    std::fs::remove_dir_all(&dir).unwrap();
    // Gone from the device list: forgotten.
    listed.lock().unwrap().clear();
    look(&shared, &config, &mut passed);
    assert!(passed.is_empty());
    assert!(shared.list().pads.is_empty());
}

/// When the rumble thread sends: the speeds changed, or a rumble still running due again after
/// [`RUMBLE_AGAIN`]; never for a pad gone or without rumble.
#[test]
fn when_a_rumble_is_sent() {
    let mut pad = stand_in(Path::new("/dev/null"));
    pad.key = 3;
    let now = Instant::now();
    assert!(pad.rumble_due(now).is_none());
    pad.wanted = (5, 6);
    let due = pad.rumble_due(now).unwrap();
    assert_eq!((due.key, due.speeds, due.effect), (3, (5, 6), None));
    pad.sent = (5, 6);
    pad.sent_at = Some(now);
    pad.effect = Some(0);
    assert!(pad
        .rumble_due(now + RUMBLE_AGAIN - Duration::from_millis(1))
        .is_none());
    assert_eq!(pad.resend_at(), Some(now + RUMBLE_AGAIN));
    let again = pad.rumble_due(now + RUMBLE_AGAIN).unwrap();
    assert_eq!((again.speeds, again.effect), ((5, 6), Some(0)));
    // Stopped: nothing to send again.
    pad.wanted = (0, 0);
    pad.sent = (0, 0);
    assert!(pad.rumble_due(now + RUMBLE_AGAIN * 2).is_none());
    assert_eq!(pad.resend_at(), None);
    pad.wanted = (1, 1);
    pad.can_rumble = false;
    assert!(pad.rumble_due(now).is_none());
    pad.can_rumble = true;
    pad.gone = true;
    assert!(pad.rumble_due(now).is_none());
}

/// Events the kernel drops (a reader that fell behind: the device's queue overflowed) are read
/// again from the device: a button pressed in the lost part still counts.
#[test]
fn dropped_events_are_read_again_from_the_device() {
    let Some(pad) = Virtual::make(0x045e, 0x028e, false) else {
        return no_uinput();
    };
    let sys = Path::new("/sys/class/input");
    let Found::Pad(mut opened) = open(&pad.node, sys, &SteamLists::default()) else {
        panic!("the virtual pad is read");
    };
    // A second reader of the same device, to see that the kernel did drop events.
    let raw = File::open(&pad.node).unwrap();
    pad.emit(EV_KEY, BTN_SOUTH, 1);
    pad.emit(EV_SYN, SYN_REPORT, 0);
    for v in 1..=1000 {
        pad.emit(EV_ABS, ABS_X, v);
        pad.emit(EV_SYN, SYN_REPORT, 0);
    }
    // SAFETY: a plain C structure, read whole.
    let mut first: libc::input_event = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::input_event>();
    // SAFETY: the structure's own bytes.
    let n = unsafe { libc::read(raw.as_raw_fd(), std::ptr::addr_of_mut!(first).cast(), size) };
    assert_eq!(n, size as isize);
    assert_eq!(
        (first.type_, first.code),
        (EV_SYN, SYN_DROPPED),
        "the kernel dropped events"
    );
    read_as_the_reader(&mut opened);
    assert!(opened.controls.south, "the press in the lost events");
    assert_eq!(opened.controls.left_x, 1000);
}

/// After dropped events, a press among the first events read and its release later in the queue
/// read as released: the rest of the queue is thrown away before the device's state is read
/// again, so the older press is not put over the newer state (review of #791).
#[test]
fn a_press_and_release_after_a_drop_read_released() {
    let Some(pad) = Virtual::make(0x045e, 0x028e, false) else {
        return no_uinput();
    };
    let sys = Path::new("/sys/class/input");
    let Found::Pad(mut opened) = open(&pad.node, sys, &SteamLists::default()) else {
        panic!("the virtual pad is read");
    };
    let raw = File::open(&pad.node).unwrap();
    // The kernel's queue for this pad holds 127 events (2 to the 7th, less one): the 128th
    // overflows it, leaving the drop and that event; 64 reports are exactly 128 events.
    for v in 1..=64 {
        pad.emit(EV_ABS, ABS_X, v);
        pad.emit(EV_SYN, SYN_REPORT, 0);
    }
    // Then 92 events, which fit: the press within the first 64 read, the release after them.
    for v in 1..=3 {
        pad.emit(EV_ABS, ABS_Y, v);
        pad.emit(EV_SYN, SYN_REPORT, 0);
    }
    pad.emit(EV_KEY, BTN_SOUTH, 1);
    pad.emit(EV_SYN, SYN_REPORT, 0);
    for v in 1..=40 {
        pad.emit(EV_ABS, ABS_RX, v);
        pad.emit(EV_SYN, SYN_REPORT, 0);
    }
    pad.emit(EV_KEY, BTN_SOUTH, 0);
    pad.emit(EV_SYN, SYN_REPORT, 0);
    // SAFETY: a plain C structure, read whole.
    let mut first: libc::input_event = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::input_event>();
    // SAFETY: the structure's own bytes.
    let n = unsafe { libc::read(raw.as_raw_fd(), std::ptr::addr_of_mut!(first).cast(), size) };
    assert_eq!(n, size as isize);
    assert_eq!((first.type_, first.code), (EV_SYN, SYN_DROPPED));
    read_as_the_reader(&mut opened);
    assert!(!opened.controls.south, "the button reads released");
    assert_eq!(opened.controls.right_x, 40);
}

/// An upload of the pad's effect that fails is tried once more as a new effect (as SDL does), and
/// the pad keeps its rumble (review of #791).
#[test]
fn a_failed_rumble_upload_is_tried_again_as_a_new_effect() {
    let Some(pad) = Virtual::make(0x045e, 0x028e, true) else {
        return no_uinput();
    };
    let mut pads = reader(vec![pad.node.clone()], SteamLists::default());
    assert!(wait_for(|| (pads.count() == 1).then_some(())).is_some());
    assert_eq!(pads.set_state(0, 0x1000, 0), ERROR_SUCCESS);
    let first = pad.take_upload();
    assert_eq!(
        pad.next().map(|e| (e.code, e.value)),
        Some((first.id as u16, 1))
    );
    assert_eq!(pads.set_state(0, 0x2000, 0), ERROR_SUCCESS);
    let refused = pad.answer_upload(-libc::EINVAL);
    assert_eq!(refused.id, first.id);
    let new = pad.take_upload();
    assert_ne!(new.id, first.id);
    assert_eq!(
        pad.next().map(|e| (e.code, e.value)),
        Some((new.id as u16, 1))
    );
    // Still rumbling: the next change updates the new effect.
    assert_eq!(pads.set_state(0, 0x3000, 0), ERROR_SUCCESS);
    assert_eq!(pad.take_upload().id, new.id);
    assert_eq!(pad.next().map(|e| e.value), Some(1));
    // Refused again, and its new effect has no room (both places taken): this change is lost, but
    // the pad keeps its rumble, and the next change is sent.
    assert_eq!(pads.set_state(0, 0x4000, 0), ERROR_SUCCESS);
    assert_eq!(pad.answer_upload(-libc::EINVAL).id, new.id);
    assert_eq!(pads.set_state(0, 0x5000, 0), ERROR_SUCCESS);
    assert_eq!(pad.take_upload().id, new.id);
}
