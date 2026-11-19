//! Sony's PlayStation pads as HID devices: the DualSense (PS5) and DualSense Edge, and the
//! DualShock 4 (PS4, both versions and Sony's wireless adapter), over USB and Bluetooth. The input
//! reports read into [`Controls`] by position, and the output reports that run the two rumble
//! motors and set a steady light bar (the first player's blue) and the DualSense's first player
//! light. Pure code, the same on every system: the HID reader ([`super::hid`]) hands it the
//! reports.
//!
//! The report layouts are the community's open descriptions of the pads, checked between two
//! independent sources: SDL's controller code (zlib licence, `SDL_hidapi_ps5.c` and
//! `SDL_hidapi_ps4.c`) and pydualsense (MIT). Nothing is taken from code under the GPL. Not yet
//! tried with a real pad.
//! - DualSense over USB, input report 0x01 (64 bytes): from byte 1 the left stick's x and y, the
//!   right stick's x and y (0 to 255, 128 the middle, y growing downward), the left and right
//!   trigger (0 to 255), a counter, then the button bytes: the hat (low four bits: 0 up, then
//!   clockwise to 7, 8 let go) with Square, Cross, Circle, Triangle in the high four bits; L1, R1,
//!   L2, R2, Create, Options, L3, R3; PS, the touchpad's click, mute. Over Bluetooth the same
//!   layout comes one byte later in report 0x31 (78 bytes), once a feature report has been read
//!   from the pad ([`feature_reports`]); before that it sends the short report 0x01, laid out as
//!   the DualShock 4's.
//! - DualShock 4 over USB, input report 0x01 (64 bytes): from byte 1 the four stick bytes, then
//!   the hat and face buttons and L1 to R3 as above (Share for Create), PS and the touchpad's
//!   click, then the left and right trigger. Over Bluetooth the same comes two bytes later in the
//!   reports 0x11 to 0x19 (once an output report has been sent), or in the short report 0x01.
//! - Output: the DualSense's report 0x02 over USB (48 bytes), 0x31 over Bluetooth (78 bytes: a
//!   sequence byte and 0x10, the same 47 bytes, a CRC-32 at the end); the DualShock 4's 0x05 over
//!   USB (32 bytes), 0x11 over Bluetooth (78 bytes: 0xC0 with the report interval, then the same
//!   bytes two later, the CRC-32). The Bluetooth CRC-32 (zlib's) runs over the byte 0xA2 and the
//!   report's first 74.

use super::layout::{axis_byte, axis_byte_down, Controls};

/// Sony's USB vendor id.
pub const SONY: u16 = 0x054c;

/// Which pad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    DualSense,
    DualShock4,
}

/// How the pad is connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Usb,
    Bluetooth,
}

/// The pad a USB vendor and product id name: the DualSense (0x0CE6) and DualSense Edge (0x0DF2),
/// the DualShock 4's first (0x05C4) and second (0x09CC) version and Sony's wireless adapter for it
/// (0x0BA0).
pub fn model(vendor: u16, product: u16) -> Option<Model> {
    match (vendor, product) {
        (SONY, 0x0ce6 | 0x0df2) => Some(Model::DualSense),
        (SONY, 0x05c4 | 0x09cc | 0x0ba0) => Some(Model::DualShock4),
        _ => None,
    }
}

/// The link a pad's longest input report gives: 64 bytes over USB (and through Sony's adapter),
/// longer over Bluetooth.
pub fn link(input_report_length: usize) -> Link {
    if input_report_length == 64 {
        Link::Usb
    } else {
        Link::Bluetooth
    }
}

/// The feature reports read from a pad when it is opened. The DualSense's serial number (0x09)
/// turns on its full input reports over Bluetooth, and 0x20 holds its firmware version
/// ([`firmware_version`]). The DualShock 4's calibration (0x02 over USB, 0x05 over Bluetooth); its
/// full Bluetooth reports come with the first output report.
pub fn feature_reports(model: Model) -> &'static [u8] {
    match model {
        Model::DualSense => &[0x09, 0x20],
        Model::DualShock4 => &[0x02, 0x05],
    }
}

/// The DualSense's firmware version in its feature report 0x20 (bytes 44 and 45, low byte first).
pub fn firmware_version(report: &[u8]) -> Option<u16> {
    (report.first() == Some(&0x20) && report.len() >= 46)
        .then(|| u16::from_le_bytes([report[44], report[45]]))
}

/// How a DualSense runs its haptic motors as two rumble motors: firmware before 0x0224 the older
/// way (at half the values, to match an Xbox pad's strength); later firmware and an unknown one
/// (0, as a Bluetooth pad may answer) the newer way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rumble {
    Older,
    Newer,
}

/// [`Rumble`] for a firmware version (0: unknown).
pub fn rumble_for(firmware: u16) -> Rumble {
    if firmware != 0 && firmware < 0x0224 {
        Rumble::Older
    } else {
        Rumble::Newer
    }
}

/// Where the parts of a report are: the first stick byte, then the DualSense's order (triggers
/// before the buttons) or the DualShock 4's (buttons first).
#[derive(Clone, Copy)]
enum Layout {
    /// Sticks at `at`, triggers at `at + 4`, buttons at `at + 7`.
    Triggers(usize),
    /// Sticks at `at`, buttons at `at + 4`, triggers at `at + 7`.
    Buttons(usize),
}

/// The controls one input report holds, `None` for a report of another kind or one too short.
pub fn read(model: Model, link: Link, report: &[u8]) -> Option<Controls> {
    let layout = match (model, link, *report.first()?) {
        (Model::DualSense, Link::Usb, 0x01) => Layout::Triggers(1),
        (Model::DualSense, Link::Bluetooth, 0x31) => Layout::Triggers(2),
        (Model::DualShock4, Link::Usb, 0x01) => Layout::Buttons(1),
        (Model::DualShock4, Link::Bluetooth, 0x11..=0x19) => Layout::Buttons(3),
        (_, Link::Bluetooth, 0x01) => Layout::Buttons(1),
        _ => return None,
    };
    let (sticks, buttons, triggers) = match layout {
        Layout::Triggers(at) => (at, at + 7, at + 4),
        Layout::Buttons(at) => (at, at + 4, at + 7),
    };
    if report.len() < sticks + 9 {
        return None;
    }
    let face = report[buttons];
    let middle = report[buttons + 1];
    let bit = |byte: u8, n: u32| byte & (1 << n) != 0;
    let mut c = Controls {
        west: bit(face, 4),
        south: bit(face, 5),
        east: bit(face, 6),
        north: bit(face, 7),
        left_shoulder: bit(middle, 0),
        right_shoulder: bit(middle, 1),
        back: bit(middle, 4),
        start: bit(middle, 5),
        left_stick: bit(middle, 6),
        right_stick: bit(middle, 7),
        left_x: axis_byte(report[sticks]),
        left_y: axis_byte_down(report[sticks + 1]),
        right_x: axis_byte(report[sticks + 2]),
        right_y: axis_byte_down(report[sticks + 3]),
        left_trigger: report[triggers],
        right_trigger: report[triggers + 1],
        ..Controls::default()
    };
    c.set_hat(face & 0x0f);
    Some(c)
}

/// The light bar's colour (red, green, blue): the first player's blue, as SDL sets it.
pub const LIGHT_BAR: [u8; 3] = [0, 0, 0x40];
/// The DualSense's five player lights, a bit each from the left: the middle one, player 1's.
const PLAYER_ONE_LIGHT: u8 = 0x04;

/// The DualShock 4's Bluetooth output report's second byte: 0xC0 (the report is a HID one with a
/// CRC) with the interval of the pad's input reports, 4 ms, the rate it reports at over USB.
const DS4_BLUETOOTH_HEADER: u8 = 0xc0 | 4;

/// The DualSense's report that ends its light bar's start-up pattern (a fade out), sent once
/// before the first [`output`] so that the colour set after it holds (over Bluetooth the pad plays
/// a pattern when it connects and keeps its own colour otherwise; over USB it does no harm). Only
/// the light-bar set-up flag (byte 38's 0x02) and its "fade out" (byte 41's 0x02) are set.
pub fn light_setup(link: Link, sequence: u8) -> Vec<u8> {
    let mut common = [0u8; 47];
    common[38] = 0x02;
    common[41] = 0x02;
    dualsense_report(link, sequence, &common)
}

/// The output report that runs the two motors (`left` the strong, low-frequency one, `right` the
/// weak one: XInput's 0 to 65535, the pad's 0 to 255, the high byte) and sets the light bar steady.
/// `sequence` is the DualSense's Bluetooth report count (its low four bits are sent); `rumble` is
/// how a DualSense runs its motors.
pub fn output(
    model: Model,
    link: Link,
    left: u16,
    right: u16,
    sequence: u8,
    rumble: Rumble,
) -> Vec<u8> {
    let (strong, weak) = ((left >> 8) as u8, (right >> 8) as u8);
    match model {
        Model::DualSense => {
            let mut common = [0u8; 47];
            // The motors as two rumble motors with the audio-driven haptics off; the older
            // firmware's emulation at half the values, the newer one's (its flag in byte 38) as
            // they are.
            let (strong, weak) = match rumble {
                Rumble::Older => {
                    common[0] = 0x01 | 0x02;
                    (strong >> 1, weak >> 1)
                }
                Rumble::Newer => {
                    common[0] = 0x02;
                    common[38] = 0x04;
                    (strong, weak)
                }
            };
            // The light bar's colour and the player lights set by this report.
            common[1] = 0x04 | 0x10;
            common[2] = weak;
            common[3] = strong;
            common[43] = PLAYER_ONE_LIGHT;
            common[44..47].copy_from_slice(&LIGHT_BAR);
            dualsense_report(link, sequence, &common)
        }
        Model::DualShock4 => {
            let mut common = [0u8; 10];
            // The motors and the light bar set by this report (no blinking).
            common[0] = 0x01 | 0x02;
            common[3] = weak;
            common[4] = strong;
            common[5..8].copy_from_slice(&LIGHT_BAR);
            match link {
                Link::Usb => {
                    let mut r = vec![0u8; 32];
                    r[0] = 0x05;
                    r[1..11].copy_from_slice(&common);
                    r
                }
                Link::Bluetooth => {
                    let mut r = vec![0u8; 78];
                    r[0] = 0x11;
                    r[1] = DS4_BLUETOOTH_HEADER;
                    r[3..13].copy_from_slice(&common);
                    seal(&mut r);
                    r
                }
            }
        }
    }
}

/// A DualSense output report around its 47 common bytes: 0x02 before them over USB; over
/// Bluetooth 0x31, the sequence in the high four bits, 0x10, then them, sealed with the CRC-32.
fn dualsense_report(link: Link, sequence: u8, common: &[u8; 47]) -> Vec<u8> {
    match link {
        Link::Usb => {
            let mut r = vec![0x02];
            r.extend_from_slice(common);
            r
        }
        Link::Bluetooth => {
            let mut r = vec![0u8; 78];
            r[0] = 0x31;
            r[1] = (sequence & 0x0f) << 4;
            r[2] = 0x10;
            r[3..50].copy_from_slice(common);
            seal(&mut r);
            r
        }
    }
}

/// A Bluetooth output report's CRC-32 in its last four bytes (low byte first): over 0xA2 and the
/// bytes before.
fn seal(report: &mut [u8]) {
    let n = report.len() - 4;
    let crc = crc32(&[&[0xa2], &report[..n]]);
    report[n..].copy_from_slice(&crc.to_le_bytes());
}

/// The CRC-32 zlib computes (polynomial 0xEDB88320 reflected, starting from and ending with all
/// bits flipped) over the parts in order.
pub fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for part in parts {
        for &byte in *part {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xinput::{buttons, PadState};

    #[test]
    fn the_crc_is_zlibs() {
        assert_eq!(crc32(&[b"123456789"]), 0xcbf4_3926);
        assert_eq!(crc32(&[b"1234", b"56789"]), 0xcbf4_3926);
        assert_eq!(crc32(&[b"a"]), 0xe8b7_be43);
        assert_eq!(crc32(&[]), 0);
    }

    #[test]
    fn the_pads_by_id() {
        assert_eq!(model(0x054c, 0x0ce6), Some(Model::DualSense));
        assert_eq!(model(0x054c, 0x0df2), Some(Model::DualSense));
        for product in [0x05c4, 0x09cc, 0x0ba0] {
            assert_eq!(model(0x054c, product), Some(Model::DualShock4));
        }
        assert_eq!(model(0x054c, 0x0268), None);
        assert_eq!(model(0x045e, 0x0ce6), None);
        assert_eq!(
            (link(64), link(78), link(547)),
            (Link::Usb, Link::Bluetooth, Link::Bluetooth)
        );
    }

    /// A DualSense USB report: sticks (left at the far left and top, right in the middle), the
    /// triggers, Cross and Triangle, the hat to the right, R1, Options and L3, PS and the touchpad.
    fn dualsense_usb() -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0x01;
        r[1..5].copy_from_slice(&[0, 0, 128, 128]);
        r[5] = 10;
        r[6] = 255;
        r[7] = 0x33;
        r[8] = 0x20 | 0x80 | 2;
        r[9] = 0x02 | 0x20 | 0x40;
        r[10] = 0x01 | 0x02;
        r
    }

    fn want_dualsense(state: PadState) {
        assert_eq!(
            state.buttons,
            buttons::A
                | buttons::Y
                | buttons::DPAD_RIGHT
                | buttons::RIGHT_SHOULDER
                | buttons::START
                | buttons::LEFT_THUMB
        );
        assert_eq!(
            (
                state.thumb_lx,
                state.thumb_ly,
                state.thumb_rx,
                state.thumb_ry
            ),
            (-32768, 32767, 0, 0)
        );
        assert_eq!((state.left_trigger, state.right_trigger), (10, 255));
    }

    #[test]
    fn a_dualsense_report_over_usb_and_bluetooth() {
        let usb = dualsense_usb();
        want_dualsense(read(Model::DualSense, Link::Usb, &usb).unwrap().state(0));
        let mut bt = vec![0u8; 78];
        bt[0] = 0x31;
        bt[1] = 0x5a;
        bt[2..65].copy_from_slice(&usb[1..64]);
        want_dualsense(
            read(Model::DualSense, Link::Bluetooth, &bt)
                .unwrap()
                .state(0),
        );
        // The other kinds of report are not controls; nor is a short one, nor a USB pad's report
        // id over the other link.
        assert!(read(Model::DualSense, Link::Usb, &bt).is_none());
        assert!(read(Model::DualSense, Link::Usb, &usb[..9]).is_none());
        assert!(read(Model::DualSense, Link::Usb, &usb[..10]).is_some());
        assert!(read(Model::DualSense, Link::Usb, &[]).is_none());
        assert!(read(Model::DualSense, Link::Bluetooth, &bt[..10]).is_none());
        let mut feature = usb.clone();
        feature[0] = 0x20;
        assert!(read(Model::DualSense, Link::Usb, &feature).is_none());
    }

    /// Each button bit of the DualSense's two button bytes is its own control: Square, Cross,
    /// Circle, Triangle; L1, R1, (L2, R2: the triggers' clicks, not buttons in XInput), Create,
    /// Options, L3, R3.
    #[test]
    fn each_dualsense_button_bit() {
        let want = [
            (8, 0x10, buttons::X),
            (8, 0x20, buttons::A),
            (8, 0x40, buttons::B),
            (8, 0x80, buttons::Y),
            (9, 0x01, buttons::LEFT_SHOULDER),
            (9, 0x02, buttons::RIGHT_SHOULDER),
            (9, 0x04, 0),
            (9, 0x08, 0),
            (9, 0x10, buttons::BACK),
            (9, 0x20, buttons::START),
            (9, 0x40, buttons::LEFT_THUMB),
            (9, 0x80, buttons::RIGHT_THUMB),
            (10, 0x01, 0),
            (10, 0x02, 0),
            (10, 0x04, 0),
        ];
        for (at, bit, xinput) in want {
            let mut r = vec![0u8; 64];
            r[0] = 0x01;
            r[1..5].copy_from_slice(&[128; 4]);
            r[8] = 0x08;
            r[at] |= bit;
            let state = read(Model::DualSense, Link::Usb, &r).unwrap().state(0);
            assert_eq!(state.buttons, xinput, "byte {at} bit {bit:#x}");
        }
    }

    /// The DualShock 4's layout (buttons before the triggers), over USB, over Bluetooth (two bytes
    /// later, in each of the reports 0x11 to 0x19) and in the short Bluetooth report both pads
    /// send first.
    #[test]
    fn a_dualshock4_report_and_the_short_bluetooth_one() {
        let mut usb = vec![0u8; 64];
        usb[0] = 0x01;
        usb[1..5].copy_from_slice(&[255, 255, 0, 64]);
        usb[5] = 0x10 | 0x40 | 6;
        usb[6] = 0x01 | 0x10 | 0x80;
        usb[7] = 0x01 | 0x02 | 0xfc;
        usb[8] = 1;
        usb[9] = 2;
        let want = |c: Controls| {
            let s = c.state(0);
            assert_eq!(
                s.buttons,
                buttons::X
                    | buttons::B
                    | buttons::DPAD_LEFT
                    | buttons::LEFT_SHOULDER
                    | buttons::BACK
                    | buttons::RIGHT_THUMB
            );
            assert_eq!(
                (s.thumb_lx, s.thumb_ly, s.thumb_rx, s.thumb_ry),
                (32767, -32768, -32768, 16384)
            );
            assert_eq!((s.left_trigger, s.right_trigger), (1, 2));
        };
        want(read(Model::DualShock4, Link::Usb, &usb).unwrap());
        for id in 0x11..=0x19 {
            let mut bt = vec![0u8; 547];
            bt[0] = id;
            bt[1] = 0xc0;
            bt[3..66].copy_from_slice(&usb[1..64]);
            want(read(Model::DualShock4, Link::Bluetooth, &bt).unwrap());
        }
        let short = &usb[..10];
        want(read(Model::DualShock4, Link::Bluetooth, short).unwrap());
        want(read(Model::DualSense, Link::Bluetooth, short).unwrap());
        assert!(read(Model::DualShock4, Link::Usb, &usb[..9]).is_none());
        let mut other = usb.clone();
        other[0] = 0x1a;
        assert!(read(Model::DualShock4, Link::Bluetooth, &other).is_none());
        other[0] = 0x11;
        assert!(read(Model::DualShock4, Link::Usb, &other).is_none());
    }

    /// The hat's "let go" (8) is no d-pad.
    #[test]
    fn the_hat_let_go() {
        let mut r = dualsense_usb();
        r[8] = 0x08;
        assert_eq!(
            read(Model::DualSense, Link::Usb, &r)
                .unwrap()
                .state(0)
                .buttons
                & (buttons::DPAD_UP
                    | buttons::DPAD_DOWN
                    | buttons::DPAD_LEFT
                    | buttons::DPAD_RIGHT),
            0
        );
    }

    #[test]
    fn the_dualsense_output_over_usb_and_bluetooth() {
        let new = Rumble::Newer;
        let usb = output(Model::DualSense, Link::Usb, 0xffff, 0x8000, 0, new);
        assert_eq!(usb.len(), 48);
        // Flags (no audio haptics; colour and player lights), the weak and the strong motor.
        assert_eq!(&usb[..5], &[0x02, 0x02, 0x14, 0x80, 0xff]);
        // The newer rumble's flag, no light-bar set-up, player 1's light, the colour.
        assert_eq!((usb[39], usb[42], usb[44]), (0x04, 0, 0x04));
        assert_eq!(&usb[45..48], &LIGHT_BAR);
        let stop = output(Model::DualSense, Link::Usb, 0, 0, 0, new);
        assert_eq!((stop[1], stop[3], stop[4], stop[39]), (0x02, 0, 0, 0x04));
        // Only the high byte reaches the pad.
        let low = output(Model::DualSense, Link::Usb, 0x00ff, 0x01ff, 0, new);
        assert_eq!((low[3], low[4]), (0x01, 0));
        // Older firmware: the other flag, the values halved, no flag at 38.
        let old = output(
            Model::DualSense,
            Link::Usb,
            0xffff,
            0x8000,
            0,
            Rumble::Older,
        );
        assert_eq!((old[1], old[3], old[4], old[39]), (0x03, 0x40, 0x7f, 0));
        let bt = output(Model::DualSense, Link::Bluetooth, 0xffff, 0x8000, 0x13, new);
        assert_eq!(bt.len(), 78);
        assert_eq!(&bt[..3], &[0x31, 0x30, 0x10]);
        assert_eq!(&bt[3..50], &usb[1..48]);
        assert!(bt[50..74].iter().all(|&b| b == 0));
        let crc = crc32(&[&[0xa2], &bt[..74]]);
        assert_eq!(&bt[74..], &crc.to_le_bytes());
    }

    #[test]
    fn the_dualsense_light_setup() {
        let usb = light_setup(Link::Usb, 0);
        assert_eq!(usb.len(), 48);
        let set: Vec<(usize, u8)> = usb
            .iter()
            .enumerate()
            .filter(|(_, &b)| b != 0)
            .map(|(i, &b)| (i, b))
            .collect();
        assert_eq!(set, [(0, 0x02), (39, 0x02), (42, 0x02)]);
        let bt = light_setup(Link::Bluetooth, 5);
        assert_eq!(&bt[..3], &[0x31, 0x50, 0x10]);
        assert_eq!(&bt[3..50], &usb[1..48]);
        let crc = crc32(&[&[0xa2], &bt[..74]]);
        assert_eq!(&bt[74..], &crc.to_le_bytes());
    }

    #[test]
    fn the_dualsense_firmware_decides_the_rumble() {
        let mut report = vec![0u8; 64];
        report[0] = 0x20;
        report[44] = 0x24;
        report[45] = 0x02;
        assert_eq!(firmware_version(&report), Some(0x0224));
        assert_eq!(firmware_version(&report[..46]), Some(0x0224));
        assert_eq!(firmware_version(&report[..45]), None);
        report[0] = 0x05;
        assert_eq!(firmware_version(&report), None);
        assert_eq!(rumble_for(0x0224), Rumble::Newer);
        assert_eq!(rumble_for(0x0300), Rumble::Newer);
        assert_eq!(rumble_for(0x0223), Rumble::Older);
        assert_eq!(rumble_for(1), Rumble::Older);
        assert_eq!(rumble_for(0), Rumble::Newer);
        assert_eq!(feature_reports(Model::DualSense), &[0x09, 0x20]);
        assert_eq!(feature_reports(Model::DualShock4), &[0x02, 0x05]);
    }

    #[test]
    fn the_dualshock4_output_over_usb_and_bluetooth() {
        let usb = output(
            Model::DualShock4,
            Link::Usb,
            0x1234,
            0xff00,
            0,
            Rumble::Newer,
        );
        assert_eq!(usb.len(), 32);
        assert_eq!(&usb[..9], &[0x05, 0x03, 0, 0, 0xff, 0x12, 0, 0, 0x40]);
        assert!(usb[9..].iter().all(|&b| b == 0));
        // The DualSense's rumble kind does not change a DualShock 4's report.
        assert_eq!(
            usb,
            output(
                Model::DualShock4,
                Link::Usb,
                0x1234,
                0xff00,
                9,
                Rumble::Older
            )
        );
        let bt = output(
            Model::DualShock4,
            Link::Bluetooth,
            0x1234,
            0xff00,
            0,
            Rumble::Newer,
        );
        assert_eq!(bt.len(), 78);
        assert_eq!(&bt[..3], &[0x11, 0xc4, 0]);
        assert_eq!(&bt[3..13], &usb[1..11]);
        assert!(bt[13..74].iter().all(|&b| b == 0));
        let crc = crc32(&[&[0xa2], &bt[..74]]);
        assert_eq!(&bt[74..], &crc.to_le_bytes());
    }
}
