//! Pads read as raw HID reports (the PlayStation pads, [`super::sony`]), the same on every system:
//! the system's part (`hid_win` on Windows) finds a pad and opens it twice, one handle read and one
//! written, so that a rumble write never waits behind a read; [`HidPads::add`] then starts a reader
//! and a writer thread for it. The reader turns each input report into the pad's [`Controls`]; the
//! writer sends the output report whenever the motor speeds the game asks for change, and once at
//! the start (after the DualSense's light set-up), which sets the light bar steady. The game's
//! thread only reads and writes the shared state, and reads the clock: no device call on it.
//!
//! A pad reads as not connected while Sony's wireless adapter says no pad is behind it, and a
//! Bluetooth pad also once it has been silent for [`SILENCE`] (out of battery or range: its last
//! reading would hold a stick held). The writer then sends it one report, as SDL3 does, and a
//! write that fails ends the pad. A pad's reader runs until its read fails; until then its device
//! is not opened again, so one device never has two readers.

use super::layout::Controls;
use super::sony::{self, Link, Model, Rumble};
use super::{PadList, ERROR_DEVICE_NOT_CONNECTED, ERROR_SUCCESS};
use std::io::{Read, Write};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How long a Bluetooth pad may send nothing before it reads as not connected.
pub const SILENCE: Duration = Duration::from_millis(500);
/// How often the writer looks whether its pad has gone silent.
const WATCH: Duration = Duration::from_millis(100);

/// One pad and what the game and its threads share about it.
struct Pad {
    /// This pad's number among all added, which its threads know it by (a pad pulled out and
    /// plugged in again is a new one at the same path).
    id: u64,
    /// The system's name for the device (its path), to know it again when looking for pads.
    path: String,
    link: Link,
    /// The last reading, `None` before the first report and while the adapter says no pad.
    controls: Option<Controls>,
    /// The XInput packet number: one more at each change of `controls`.
    packet: u32,
    /// When the last report with a reading came.
    last_report: Option<Instant>,
    /// The motor speeds the game asked for last (XInput's left, right), and those sent.
    wanted: (u16, u16),
    sent: Option<(u16, u16)>,
    /// The writer has sent its one report for this silence (the next reading clears it).
    probed: bool,
    /// The reader saw the pad go (pulled out, switched off) or the writer could not write: its
    /// threads end and the next look for pads forgets it.
    gone: bool,
    /// Its reader thread still runs (it ends when its read fails).
    reading: bool,
}

impl Pad {
    /// Whether no reading has come for [`SILENCE`] at `now`.
    fn silent_at(&self, now: Instant) -> bool {
        self.last_report
            .map_or(true, |t| now.saturating_duration_since(t) >= SILENCE)
    }

    /// Whether the game sees the pad at `now`: it has a reading, has not gone, and over Bluetooth
    /// has not gone silent.
    fn readable_at(&self, now: Instant) -> bool {
        !self.gone && self.controls.is_some() && (self.link == Link::Usb || !self.silent_at(now))
    }
}

#[derive(Default)]
struct List {
    pads: Vec<Pad>,
    /// The next pad's [`Pad::id`].
    next_id: u64,
}

impl List {
    /// The pad `id` while it has not gone.
    fn live(&mut self, id: u64) -> Option<&mut Pad> {
        self.pads.iter_mut().find(|p| p.id == id && !p.gone)
    }
}

#[derive(Default)]
struct Shared {
    list: Mutex<List>,
    /// Woken when a pad's wanted speeds change or a pad goes.
    changed: Condvar,
}

impl Shared {
    fn list(&self) -> MutexGuard<'_, List> {
        self.list.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The pad `id` has gone: its threads end.
    fn end(&self, id: u64) {
        if let Some(pad) = self.list().live(id) {
            pad.gone = true;
        }
        self.changed.notify_all();
    }
}

/// The pads read as HID reports, in the order they were found; a clone shares them.
#[derive(Clone, Default)]
pub struct HidPads {
    shared: Arc<Shared>,
}

/// A pad found and opened: what [`HidPads::add`] needs to run it.
pub struct Opened<R, W> {
    pub path: String,
    pub model: Model,
    pub link: Link,
    /// The handle read and its input reports' length (the longest one's, as the system gives it).
    pub reader: R,
    pub input_length: usize,
    /// The handle written and its output reports' length (each report is padded to it).
    pub writer: W,
    pub output_length: usize,
    /// How a DualSense runs its motors (its firmware's way).
    pub rumble: Rumble,
}

impl HidPads {
    pub fn new() -> HidPads {
        HidPads::default()
    }

    /// Whether the device at `path` is one of these whose reader still runs (a pad that has gone
    /// keeps its path until its reader ends, so the device is not opened twice); the paths are
    /// compared without regard to case, as Windows' are.
    pub fn knows(&self, path: &str) -> bool {
        self.shared
            .list()
            .pads
            .iter()
            .any(|p| p.reading && p.path.eq_ignore_ascii_case(path))
    }

    /// Forgets the pads that have gone and whose readers have ended.
    pub fn forget_gone(&self) {
        self.shared.list().pads.retain(|p| !p.gone || p.reading);
    }

    /// Runs the pad `opened`: its reader and writer threads start.
    pub fn add<R, W>(&self, opened: Opened<R, W>) -> std::io::Result<()>
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        let Opened {
            path,
            model,
            link,
            reader,
            input_length,
            writer,
            output_length,
            rumble,
        } = opened;
        let id = {
            let mut list = self.shared.list();
            let id = list.next_id;
            list.next_id += 1;
            list.pads.push(Pad {
                id,
                path,
                link,
                controls: None,
                packet: 0,
                last_report: None,
                wanted: (0, 0),
                sent: None,
                probed: false,
                gone: false,
                reading: true,
            });
            id
        };
        let s = self.shared.clone();
        let read = std::thread::Builder::new()
            .name("nv-rs pad reader".into())
            .spawn(move || read_pad(&s, id, reader, model, link, input_length));
        if let Err(e) = read {
            self.shared.end(id);
            if let Some(pad) = self.shared.list().pads.iter_mut().find(|p| p.id == id) {
                pad.reading = false;
            }
            return Err(e);
        }
        let s = self.shared.clone();
        let write = std::thread::Builder::new()
            .name("nv-rs pad writer".into())
            .spawn(move || write_pad(&s, id, writer, model, link, output_length, rumble));
        if let Err(e) = write {
            self.shared.end(id);
            return Err(e);
        }
        Ok(())
    }

    /// [`PadList::count`] at `now`.
    fn count_at(&self, now: Instant) -> usize {
        self.shared
            .list()
            .pads
            .iter()
            .filter(|p| p.readable_at(now))
            .count()
    }
}

impl PadList for HidPads {
    fn count(&mut self) -> usize {
        self.count_at(Instant::now())
    }

    fn get_state(&mut self, n: usize, state: &mut [u8; 16]) -> u32 {
        let now = Instant::now();
        let list = self.shared.list();
        match list.pads.iter().filter(|p| p.readable_at(now)).nth(n) {
            Some(pad) => {
                *state = pad
                    .controls
                    .unwrap_or_default()
                    .state(pad.packet)
                    .to_bytes();
                ERROR_SUCCESS
            }
            None => ERROR_DEVICE_NOT_CONNECTED,
        }
    }

    fn set_state(&mut self, n: usize, left: u16, right: u16) -> u32 {
        let now = Instant::now();
        let mut list = self.shared.list();
        match list.pads.iter_mut().filter(|p| p.readable_at(now)).nth(n) {
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

/// The reader: each input report read into the pad's controls until the pad goes (a failed or
/// empty read, or the writer ended it); then its device may be opened again.
fn read_pad(
    shared: &Shared,
    id: u64,
    mut file: impl Read,
    model: Model,
    link: Link,
    length: usize,
) {
    let mut buffer = vec![0u8; length.max(1)];
    loop {
        let read = file.read(&mut buffer);
        let mut list = shared.list();
        let Some(pad) = list.live(id) else {
            break;
        };
        match read {
            Ok(n) if n > 0 => {
                let report = &buffer[..n];
                if sony::no_pad(model, link, report) {
                    // Sony's adapter with no pad behind it: no pad until a reading comes.
                    pad.controls = None;
                } else if let Some(controls) = sony::read(model, link, report) {
                    if pad.controls != Some(controls) {
                        pad.controls = Some(controls);
                        pad.packet = pad.packet.wrapping_add(1);
                    }
                    pad.last_report = Some(Instant::now());
                    pad.probed = false;
                }
            }
            _ => {
                pad.gone = true;
                shared.changed.notify_all();
                break;
            }
        }
    }
    if let Some(pad) = shared.list().pads.iter_mut().find(|p| p.id == id) {
        pad.reading = false;
    }
}

/// One report written, padded to `length`; the sequence counts it.
fn send(file: &mut impl Write, mut report: Vec<u8>, length: usize, sequence: &mut u8) -> bool {
    *sequence = sequence.wrapping_add(1);
    report.resize(report.len().max(length), 0);
    file.write_all(&report).is_ok()
}

/// The writer: the DualSense's light set-up first, then the output report at the start, whenever
/// the wanted speeds change, and once when a Bluetooth pad goes silent. A failed write ends the
/// pad.
fn write_pad(
    shared: &Shared,
    id: u64,
    mut file: impl Write,
    model: Model,
    link: Link,
    length: usize,
    rumble: Rumble,
) {
    let mut sequence = 0u8;
    if model == Model::DualSense {
        let setup = sony::light_setup(link, sequence);
        if !send(&mut file, setup, length, &mut sequence) {
            shared.end(id);
            return;
        }
    }
    loop {
        let (left, right) = {
            let mut list = shared.list();
            loop {
                let Some(pad) = list.live(id) else {
                    return;
                };
                if pad.sent != Some(pad.wanted) {
                    pad.sent = Some(pad.wanted);
                    break pad.wanted;
                }
                if link == Link::Bluetooth
                    && !pad.probed
                    && pad.last_report.is_some()
                    && pad.silent_at(Instant::now())
                {
                    pad.probed = true;
                    break pad.wanted;
                }
                list = shared
                    .changed
                    .wait_timeout(list, WATCH)
                    .unwrap_or_else(|p| p.into_inner())
                    .0;
            }
        };
        let report = sony::output(model, link, left, right, sequence, rumble);
        if !send(&mut file, report, length, &mut sequence) {
            shared.end(id);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xinput::{buttons, PadState};
    use std::sync::mpsc;

    /// A pad's reading end: the reports sent down the channel, one a read; the channel closed is
    /// the pad pulled out (a failed read). Nothing sent is a pad gone silent (the read blocks).
    struct FakeReader(mpsc::Receiver<Vec<u8>>);

    impl Read for FakeReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.0.recv() {
                Ok(report) => {
                    let n = report.len().min(buf.len());
                    buf[..n].copy_from_slice(&report[..n]);
                    Ok(n)
                }
                Err(_) => Err(std::io::ErrorKind::BrokenPipe.into()),
            }
        }
    }

    /// A pad's writing end: each write kept; `fail` makes the writes fail.
    #[derive(Clone, Default)]
    struct FakeWriter {
        written: Arc<Mutex<Vec<Vec<u8>>>>,
        fail: Arc<Mutex<bool>>,
    }

    impl FakeWriter {
        fn written(&self) -> Vec<Vec<u8>> {
            self.written.lock().unwrap().clone()
        }
    }

    impl Write for FakeWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if *self.fail.lock().unwrap() {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            self.written.lock().unwrap().push(buf.to_vec());
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Waits up to five seconds for `done`.
    fn wait_for(mut done: impl FnMut() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(start.elapsed() < Duration::from_secs(5), "timed out");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn usb_dualsense_report(cross: bool, left_x: u8) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0x01;
        r[1..5].copy_from_slice(&[left_x, 128, 128, 128]);
        r[8] = 0x08 | if cross { 0x20 } else { 0 };
        r
    }

    /// A DualSense's sealed Bluetooth report, the left stick at `left_x`.
    fn bt_dualsense_report(left_x: u8) -> Vec<u8> {
        let mut bt = vec![0u8; 78];
        bt[0] = 0x31;
        bt[2..6].copy_from_slice(&[left_x, 128, 128, 128]);
        bt[9] = 0x08;
        sony::seal_input(&mut bt);
        bt
    }

    fn add(
        pads: &HidPads,
        path: &str,
        model: Model,
        link: Link,
        lengths: (usize, usize),
    ) -> (mpsc::Sender<Vec<u8>>, FakeWriter) {
        let (tx, rx) = mpsc::channel();
        let writer = FakeWriter::default();
        pads.add(Opened {
            path: path.into(),
            model,
            link,
            reader: FakeReader(rx),
            input_length: lengths.0,
            writer: writer.clone(),
            output_length: lengths.1,
            rumble: Rumble::Newer,
        })
        .unwrap();
        (tx, writer)
    }

    fn add_dualsense(pads: &HidPads, path: &str) -> (mpsc::Sender<Vec<u8>>, FakeWriter) {
        add(pads, path, Model::DualSense, Link::Usb, (64, 48))
    }

    fn state(pads: &mut HidPads, n: usize) -> (u32, PadState) {
        let mut bytes = [0xaa; 16];
        let result = pads.get_state(n, &mut bytes);
        (result, PadState::from_bytes(&bytes))
    }

    /// A pad is not connected before its first report; then each changed report is a new packet
    /// with its controls, an unchanged one the same packet; pulled out, it is gone, and its path
    /// is forgotten once its reader has ended.
    #[test]
    fn a_pad_reads_and_goes() {
        let mut pads = HidPads::new();
        let (tx, writer) = add_dualsense(&pads, r"\\?\HID#PAD1");
        assert!(pads.knows(r"\\?\hid#pad1"));
        assert!(!pads.knows(r"\\?\hid#pad2"));
        assert_eq!(pads.count(), 0);
        let mut bytes = [0xaa; 16];
        assert_eq!(pads.get_state(0, &mut bytes), ERROR_DEVICE_NOT_CONNECTED);
        assert_eq!(bytes, [0xaa; 16]);
        assert_eq!(pads.set_state(0, 1, 2), ERROR_DEVICE_NOT_CONNECTED);

        tx.send(usb_dualsense_report(true, 0)).unwrap();
        wait_for(|| pads.count() == 1);
        let (result, first) = state(&mut pads, 0);
        assert_eq!(result, ERROR_SUCCESS);
        assert_eq!(
            (first.packet, first.buttons, first.thumb_lx),
            (1, buttons::A, -32768)
        );
        // The same reading again: the same packet. A report of another kind changes nothing.
        tx.send(usb_dualsense_report(true, 0)).unwrap();
        let mut other = usb_dualsense_report(false, 255);
        other[0] = 0x20;
        tx.send(other).unwrap();
        tx.send(usb_dualsense_report(false, 255)).unwrap();
        wait_for(|| state(&mut pads, 0).1.thumb_lx == 32767);
        let (_, second) = state(&mut pads, 0);
        assert_eq!((second.packet, second.buttons), (2, 0));
        assert_eq!(pads.get_state(1, &mut bytes), ERROR_DEVICE_NOT_CONNECTED);

        // The light set-up and the first output went out at the start, padded to 48 bytes.
        wait_for(|| writer.written().len() == 2);
        let written = writer.written();
        assert_eq!(written[0], sony::light_setup(Link::Usb, 0));
        assert_eq!(
            written[1],
            sony::output(Model::DualSense, Link::Usb, 0, 0, 1, Rumble::Newer)
        );

        drop(tx);
        wait_for(|| pads.count() == 0);
        wait_for(|| !pads.knows(r"\\?\HID#PAD1"));
        assert_eq!(pads.get_state(0, &mut bytes), ERROR_DEVICE_NOT_CONNECTED);
        pads.forget_gone();
        assert!(pads.shared.list().pads.is_empty());
    }

    /// The game's motor speeds reach the pad once per change, the Bluetooth reports numbered in
    /// turn and padded to the output length; asking for the same speeds again sends nothing. The
    /// pad keeps reporting all along, so it is never silent.
    #[test]
    fn rumble_is_sent_when_it_changes() {
        let mut pads = HidPads::new();
        let (tx, writer) = add(&pads, "bt", Model::DualSense, Link::Bluetooth, (78, 80));
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let feeding = {
            let (tx, done) = (tx.clone(), done.clone());
            std::thread::spawn(move || {
                while !done.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = tx.send(bt_dualsense_report(128));
                    std::thread::sleep(Duration::from_millis(10));
                }
            })
        };
        wait_for(|| pads.count() == 1);
        wait_for(|| writer.written().len() == 2);
        assert_eq!(pads.set_state(0, 0xffff, 0x8000), ERROR_SUCCESS);
        wait_for(|| writer.written().len() == 3);
        assert_eq!(pads.set_state(0, 0xffff, 0x8000), ERROR_SUCCESS);
        assert_eq!(pads.set_state(0, 0, 0), ERROR_SUCCESS);
        wait_for(|| writer.written().len() == 4);
        std::thread::sleep(Duration::from_millis(50));
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        feeding.join().unwrap();
        let written = writer.written();
        assert_eq!(written.len(), 4);
        let padded = |mut r: Vec<u8>| {
            r.resize(80, 0);
            r
        };
        let run = |l, r, seq| {
            padded(sony::output(
                Model::DualSense,
                Link::Bluetooth,
                l,
                r,
                seq,
                Rumble::Newer,
            ))
        };
        assert_eq!(written[0], padded(sony::light_setup(Link::Bluetooth, 0)));
        assert_eq!(written[1], run(0, 0, 1));
        assert_eq!(written[2], run(0xffff, 0x8000, 2));
        assert_eq!(written[3], run(0, 0, 3));
        drop(tx);
    }

    /// A write that fails ends the pad, as a failed read does; its device's path stays known
    /// (no second reader) until its reader ends.
    #[test]
    fn a_failed_write_ends_the_pad() {
        let mut pads = HidPads::new();
        let (tx, writer) = add_dualsense(&pads, "usb");
        tx.send(usb_dualsense_report(false, 128)).unwrap();
        wait_for(|| pads.count() == 1);
        wait_for(|| writer.written().len() == 2);
        *writer.fail.lock().unwrap() = true;
        assert_eq!(pads.set_state(0, 5000, 0), ERROR_SUCCESS);
        wait_for(|| pads.count() == 0);
        assert!(pads.knows("usb"), "its reader still runs");
        pads.forget_gone();
        assert_eq!(pads.shared.list().pads.len(), 1);
        drop(tx);
        wait_for(|| !pads.knows("usb"));
        pads.forget_gone();
        assert!(pads.shared.list().pads.is_empty());
    }

    /// Two pads are the list's first and second, in the order found; the first one gone, the
    /// second is first. A pad plugged in again at the same path is a new one, which the old
    /// one's reader does not touch.
    #[test]
    fn two_pads_in_order_and_one_plugged_in_again() {
        let mut pads = HidPads::new();
        let (one, _w1) = add_dualsense(&pads, "one");
        let (two, _w2) = add_dualsense(&pads, "two");
        two.send(usb_dualsense_report(false, 255)).unwrap();
        wait_for(|| pads.count() == 1);
        // Only the second has sent a reading: it is the first the game sees.
        assert_eq!(state(&mut pads, 0).1.thumb_lx, 32767);
        one.send(usb_dualsense_report(true, 0)).unwrap();
        wait_for(|| pads.count() == 2);
        assert_eq!(state(&mut pads, 0).1.buttons, buttons::A);
        assert_eq!(state(&mut pads, 1).1.thumb_lx, 32767);
        drop(one);
        wait_for(|| pads.count() == 1);
        assert_eq!(state(&mut pads, 0).1.thumb_lx, 32767);
        wait_for(|| !pads.knows("one"));
        pads.forget_gone();
        let (again, _w3) = add_dualsense(&pads, "one");
        again.send(usb_dualsense_report(false, 0)).unwrap();
        wait_for(|| pads.count() == 2);
        assert_eq!(state(&mut pads, 1).1.packet, 1);
        drop(two);
        drop(again);
    }

    /// A DualShock 4 gets no light set-up: its first write is the output report.
    #[test]
    fn a_dualshock4_starts_with_its_output() {
        let pads = HidPads::new();
        let (tx, writer) = add(&pads, "ds4", Model::DualShock4, Link::Usb, (64, 32));
        wait_for(|| !writer.written().is_empty());
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(
            writer.written(),
            [sony::output(
                Model::DualShock4,
                Link::Usb,
                0,
                0,
                0,
                Rumble::Newer
            )]
        );
        drop(tx);
    }

    /// Sony's wireless adapter (review of #783): with no pad behind it the pad is not connected
    /// (the game then reads the keyboard and mouse), whether it had a reading before or not; a
    /// reading brings it back.
    #[test]
    fn the_adapter_without_its_pad_reads_as_no_pad() {
        let mut pads = HidPads::new();
        let (tx, _writer) = add(&pads, "adapter", Model::DualShock4, Link::Usb, (64, 32));
        let reading = || {
            let mut r = vec![0u8; 64];
            r[0] = 0x01;
            r[1..5].copy_from_slice(&[128; 4]);
            r[5] = 0x08;
            r
        };
        let mut nobody = reading();
        nobody[31] = 0x04;
        tx.send(nobody.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(pads.count(), 0);
        tx.send(reading()).unwrap();
        wait_for(|| pads.count() == 1);
        tx.send(nobody).unwrap();
        wait_for(|| pads.count() == 0);
        let mut bytes = [0xaa; 16];
        assert_eq!(pads.get_state(0, &mut bytes), ERROR_DEVICE_NOT_CONNECTED);
        tx.send(reading()).unwrap();
        wait_for(|| pads.count() == 1);
        assert!(pads.knows("adapter"));
        drop(tx);
    }

    /// A Bluetooth pad that goes silent (review of #783: its held stick stayed held) reads as not
    /// connected once [`SILENCE`] has passed, and the writer sends it one report; a reading brings
    /// it back. Silent again with its writes failing, that report ends it. A USB pad is not timed.
    #[test]
    fn a_silent_bluetooth_pad_reads_as_gone_and_is_asked_once() {
        let mut pads = HidPads::new();
        let (tx, writer) = add(&pads, "bt", Model::DualSense, Link::Bluetooth, (78, 78));
        tx.send(bt_dualsense_report(0)).unwrap();
        wait_for(|| pads.count() == 1);
        assert_eq!(state(&mut pads, 0).1.thumb_lx, -32768);
        wait_for(|| writer.written().len() == 2);
        // Silent: not connected after SILENCE, and one report sent to ask.
        wait_for(|| pads.count() == 0);
        wait_for(|| writer.written().len() == 3);
        std::thread::sleep(SILENCE + WATCH * 3);
        assert_eq!(writer.written().len(), 3, "one report a silence");
        assert_eq!(
            writer.written()[2],
            sony::output(Model::DualSense, Link::Bluetooth, 0, 0, 2, Rumble::Newer)
        );
        tx.send(bt_dualsense_report(128)).unwrap();
        wait_for(|| pads.count() == 1);
        *writer.fail.lock().unwrap() = true;
        wait_for(|| pads.shared.list().pads[0].gone);
        assert!(pads.knows("bt"), "its reader still runs");
        drop(tx);
        wait_for(|| !pads.knows("bt"));
        // Over USB, silence is no sign: the pad stays.
        let (usb, _w) = add_dualsense(&pads, "usb");
        usb.send(usb_dualsense_report(false, 128)).unwrap();
        wait_for(|| pads.count() == 1);
        std::thread::sleep(SILENCE + WATCH);
        assert_eq!(pads.count(), 1);
        drop(usb);
    }
}
