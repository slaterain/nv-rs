//! A Linux pad's kernel events (evdev) read into [`Controls`]: pure code, built and tested on every
//! system; the Linux reader (`linux.rs`) opens the devices and hands their ranges and events here.
//!
//! The kernel's codes are linux/input-event-codes.h's. Its gamepad rules (the kernel's
//! Documentation/input/gamepad.rst) name the face buttons by position: `BTN_SOUTH`, `BTN_EAST`,
//! `BTN_NORTH` and `BTN_WEST`. The same codes also carry the letter names `BTN_A`, `BTN_B`, `BTN_X`
//! and `BTN_Y`, and the drivers differ in which of the two they follow. Sony's (hid-playstation,
//! hid-sony) and Nintendo's (hid-nintendo) go by position: the top button is `BTN_NORTH` (0x133),
//! the left one `BTN_WEST` (0x134). The Xbox drivers (xpad, and Microsoft's Bluetooth pads) and
//! Steam Input's virtual pad go by letter: X, on the left, is `BTN_X` (0x133), and Y, on top, is
//! `BTN_Y` (0x134). So 0x133 and 0x134 trade places between the two ([`Faces`]), and which pads are
//! read at all is [`faces`]'s answer.
//!
//! The sticks are `ABS_X`/`ABS_Y` and `ABS_RX`/`ABS_RY`, y growing downward. The triggers are
//! `ABS_Z`/`ABS_RZ`; on a pad whose right stick is on `ABS_Z`/`ABS_RZ` (no `ABS_RX`: Microsoft's
//! Bluetooth pads) they are `ABS_BRAKE`/`ABS_GAS`; on a pad with neither (the Switch Pro
//! Controller) only the buttons `BTN_TL2`/`BTN_TR2`, all the way when pressed. The d-pad is the hat
//! `ABS_HAT0X`/`ABS_HAT0Y` (when its range is -1 to 1: a hat on a wider range is a touch pad's
//! position) or the buttons `BTN_DPAD_*`. Each axis is scaled from the range the device gives (its
//! `input_absinfo`) as [`layout`](super::layout)'s byte axes are, so a DualSense reads the same here
//! as through HID on Windows and an Xbox pad's x axes come through unchanged; no dead zone is
//! applied (the game applies its own, as with XInput).

use super::layout::Controls;

/// Event types and codes (linux/input-event-codes.h).
pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const EV_FF: u16 = 0x15;
pub const SYN_REPORT: u16 = 0;
pub const SYN_DROPPED: u16 = 3;
pub const BTN_SOUTH: u16 = 0x130;
pub const BTN_EAST: u16 = 0x131;
pub const BTN_NORTH: u16 = 0x133;
pub const BTN_WEST: u16 = 0x134;
pub const BTN_TL: u16 = 0x136;
pub const BTN_TR: u16 = 0x137;
pub const BTN_TL2: u16 = 0x138;
pub const BTN_TR2: u16 = 0x139;
pub const BTN_SELECT: u16 = 0x13a;
pub const BTN_START: u16 = 0x13b;
pub const BTN_MODE: u16 = 0x13c;
pub const BTN_THUMBL: u16 = 0x13d;
pub const BTN_THUMBR: u16 = 0x13e;
pub const BTN_DPAD_UP: u16 = 0x220;
pub const BTN_DPAD_DOWN: u16 = 0x221;
pub const BTN_DPAD_LEFT: u16 = 0x222;
pub const BTN_DPAD_RIGHT: u16 = 0x223;
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_Z: u16 = 0x02;
pub const ABS_RX: u16 = 0x03;
pub const ABS_RY: u16 = 0x04;
pub const ABS_RZ: u16 = 0x05;
pub const ABS_GAS: u16 = 0x09;
pub const ABS_BRAKE: u16 = 0x0a;
pub const ABS_HAT0X: u16 = 0x10;
pub const ABS_HAT0Y: u16 = 0x11;
/// The kernel's rumble effect.
pub const FF_RUMBLE: u16 = 0x50;
/// The highest key, axis and effect codes (`KEY_MAX`, `ABS_MAX`, `FF_MAX`).
pub const KEY_MAX: u16 = 0x2ff;
pub const ABS_MAX: u16 = 0x3f;
pub const FF_MAX: u16 = 0x7f;

/// USB vendor ids.
pub const MICROSOFT: u16 = 0x045e;
pub const SONY: u16 = 0x054c;
pub const NINTENDO: u16 = 0x057e;
pub const VALVE: u16 = 0x28de;
/// Steam Input's virtual pad: the pad a game started from Steam reads (on the Steam Deck, the
/// Deck's own controls as Steam sets them up).
pub const STEAM_VIRTUAL: (u16, u16) = (VALVE, 0x11ff);

/// How a pad's driver names its four face buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faces {
    /// `BTN_X` (0x133) is the left button, `BTN_Y` (0x134) the top one.
    ByLetter,
    /// `BTN_NORTH` (0x133) is the top button, `BTN_WEST` (0x134) the left one.
    ByPosition,
}

/// How the pad `vendor`:`product`, driven by the kernel driver `driver` (`None` for a virtual
/// device, which has none), names its face buttons, or `None` when it is not read: Steam's virtual
/// pad, Microsoft's pads and the pads xpad drives (other makers' Xbox-style pads) by letter, Sony's
/// and Nintendo's by position. Not read: Valve's own devices other than the virtual pad (the Steam
/// Deck's and the Steam Controller's own controls, which Steam reads and stands in for), and the
/// pads the kernel's generic HID driver reads, whose buttons come in each maker's own order.
pub fn faces(vendor: u16, product: u16, driver: Option<&str>) -> Option<Faces> {
    match (vendor, product) {
        STEAM_VIRTUAL => Some(Faces::ByLetter),
        (VALVE, _) => None,
        (MICROSOFT, _) => Some(Faces::ByLetter),
        (SONY | NINTENDO, _) => Some(Faces::ByPosition),
        _ if driver == Some("xpad") => Some(Faces::ByLetter),
        _ => None,
    }
}

/// An axis's range as the device gives it (`input_absinfo`'s minimum and maximum).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Range {
    pub min: i32,
    pub max: i32,
}

/// The range of a d-pad's hat.
const HAT: Range = Range { min: -1, max: 1 };

/// Whether a device with these keys and axes is a pad: the bottom face button and a stick.
pub fn is_pad(has_key: impl Fn(u16) -> bool, has_abs: impl Fn(u16) -> bool) -> bool {
    has_key(BTN_SOUTH) && has_abs(ABS_X) && has_abs(ABS_Y)
}

/// Which axis each control is on, worked out once from the axes the device has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axes {
    pub right_x: u16,
    pub right_y: u16,
    /// The triggers' axes, `None` when the pad has only the trigger buttons.
    pub triggers: Option<(u16, u16)>,
}

impl Axes {
    pub fn of(has_abs: impl Fn(u16) -> bool) -> Axes {
        if has_abs(ABS_RX) && has_abs(ABS_RY) {
            Axes {
                right_x: ABS_RX,
                right_y: ABS_RY,
                triggers: (has_abs(ABS_Z) && has_abs(ABS_RZ)).then_some((ABS_Z, ABS_RZ)),
            }
        } else {
            Axes {
                right_x: ABS_Z,
                right_y: ABS_RZ,
                triggers: (has_abs(ABS_BRAKE) && has_abs(ABS_GAS)).then_some((ABS_BRAKE, ABS_GAS)),
            }
        }
    }
}

/// What an event asks of the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Nothing yet.
    Nothing,
    /// A whole report has come: the controls are to be read out.
    Report,
    /// Events were lost and the report that ends the loss has come: the device's keys and axes
    /// are to be read again as they are now ([`Pad::set_key`], [`Pad::set_abs`]), then the
    /// controls read out.
    Resync,
}

/// One pad's events folded into its controls: the key and axis values as last reported, read out
/// at each `SYN_REPORT`.
#[derive(Clone, Debug)]
pub struct Pad {
    faces: Faces,
    axes: Axes,
    ranges: Vec<Range>,
    values: Vec<i32>,
    keys: Vec<bool>,
    /// The kernel said it dropped events (`SYN_DROPPED`): the events up to and including the next
    /// `SYN_REPORT` are passed over (the kernel's evdev rules), then the device is read again.
    dropping: bool,
}

impl Pad {
    /// A pad whose face buttons are named `faces`, with these axes and their ranges (by code, up
    /// to [`ABS_MAX`]). Every key starts let go and every axis at 0, until the device's own
    /// current values are set.
    pub fn new(faces: Faces, axes: Axes, ranges: Vec<Range>) -> Pad {
        let n = usize::from(ABS_MAX) + 1;
        let mut ranges = ranges;
        ranges.resize(n, Range::default());
        Pad {
            faces,
            axes,
            ranges,
            values: vec![0; n],
            keys: vec![false; usize::from(KEY_MAX) + 1],
            dropping: false,
        }
    }

    /// One event read from the device.
    pub fn event(&mut self, kind: u16, code: u16, value: i32) -> Step {
        if self.dropping {
            if kind == EV_SYN && code == SYN_REPORT {
                self.dropping = false;
                return Step::Resync;
            }
            return Step::Nothing;
        }
        match (kind, code) {
            (EV_KEY, _) => self.set_key(code, value != 0),
            (EV_ABS, _) => self.set_abs(code, value),
            (EV_SYN, SYN_REPORT) => return Step::Report,
            (EV_SYN, SYN_DROPPED) => self.dropping = true,
            _ => {}
        }
        Step::Nothing
    }

    /// Sets a key as the device says it is now.
    pub fn set_key(&mut self, code: u16, down: bool) {
        if let Some(k) = self.keys.get_mut(usize::from(code)) {
            *k = down;
        }
    }

    /// Sets an axis as the device says it is now.
    pub fn set_abs(&mut self, code: u16, value: i32) {
        if let Some(v) = self.values.get_mut(usize::from(code)) {
            *v = value;
        }
    }

    fn key(&self, code: u16) -> bool {
        self.keys[usize::from(code)]
    }

    fn axis(&self, code: u16) -> (i32, Range) {
        (
            self.values[usize::from(code)],
            self.ranges[usize::from(code)],
        )
    }

    /// The hat's direction on one axis (-1, 0 or 1), 0 when the hat is not a d-pad's.
    fn hat(&self, code: u16) -> i32 {
        let (v, r) = self.axis(code);
        if r == HAT {
            v.signum()
        } else {
            0
        }
    }

    fn trigger(&self, axis: Option<u16>, button: u16) -> u8 {
        match axis {
            Some(code) => {
                let (v, r) = self.axis(code);
                trigger(v, r)
            }
            None => {
                if self.key(button) {
                    255
                } else {
                    0
                }
            }
        }
    }

    /// The controls now.
    pub fn controls(&self) -> Controls {
        let (north, west) = match self.faces {
            Faces::ByPosition => (self.key(BTN_NORTH), self.key(BTN_WEST)),
            Faces::ByLetter => (self.key(BTN_WEST), self.key(BTN_NORTH)),
        };
        let (hat_x, hat_y) = (self.hat(ABS_HAT0X), self.hat(ABS_HAT0Y));
        let (lx, lx_range) = self.axis(ABS_X);
        let (ly, ly_range) = self.axis(ABS_Y);
        let (rx, rx_range) = self.axis(self.axes.right_x);
        let (ry, ry_range) = self.axis(self.axes.right_y);
        Controls {
            south: self.key(BTN_SOUTH),
            east: self.key(BTN_EAST),
            west,
            north,
            left_shoulder: self.key(BTN_TL),
            right_shoulder: self.key(BTN_TR),
            back: self.key(BTN_SELECT),
            start: self.key(BTN_START),
            left_stick: self.key(BTN_THUMBL),
            right_stick: self.key(BTN_THUMBR),
            up: hat_y < 0 || self.key(BTN_DPAD_UP),
            down: hat_y > 0 || self.key(BTN_DPAD_DOWN),
            left: hat_x < 0 || self.key(BTN_DPAD_LEFT),
            right: hat_x > 0 || self.key(BTN_DPAD_RIGHT),
            left_x: stick(lx, lx_range),
            left_y: stick_down(ly, ly_range),
            right_x: stick(rx, rx_range),
            right_y: stick_down(ry, ry_range),
            left_trigger: self.trigger(self.axes.triggers.map(|t| t.0), BTN_TL2),
            right_trigger: self.trigger(self.axes.triggers.map(|t| t.1), BTN_TR2),
        }
    }
}

/// A range's middle: half way between its ends, the upper of the two middle values when there is
/// an even number of them (128 for 0 to 255, 0 for -32768 to 32767), as the byte axes' 128.
fn middle(r: Range) -> i64 {
    i64::from(r.min) + (i64::from(r.max) - i64::from(r.min) + 1) / 2
}

/// A stick's axis (right positive) onto XInput's -32768 ..= 32767: the middle 0, each side scaled
/// on its own and rounded (halves away from zero), values past the ends at the ends. A range of
/// one value is 0.
pub fn stick(v: i32, r: Range) -> i16 {
    if r.max <= r.min {
        return 0;
    }
    let m = middle(r);
    scale(i64::from(v) - m, m - i64::from(r.min), i64::from(r.max) - m)
}

/// [`stick`] for an up-down axis, which grows downward: XInput's grows upward.
pub fn stick_down(v: i32, r: Range) -> i16 {
    if r.max <= r.min {
        return 0;
    }
    let m = middle(r);
    scale(m - i64::from(v), i64::from(r.max) - m, m - i64::from(r.min))
}

/// `d` from `-below ..= above` onto -32768 ..= 32767, each side scaled on its own, rounded to the
/// nearest (halves away from zero), as [`layout`](super::layout)'s byte axes are; a side of no
/// length is its end.
fn scale(d: i64, below: i64, above: i64) -> i16 {
    let scaled = if d > 0 {
        if above == 0 {
            32767
        } else {
            (d * 2 * 32767 + above) / (2 * above)
        }
    } else if d < 0 {
        if below == 0 {
            -32768
        } else {
            -((-d * 2 * 32768 + below) / (2 * below))
        }
    } else {
        0
    };
    scaled.clamp(-32768, 32767) as i16
}

/// A trigger's axis onto XInput's 0 ..= 255, rounded; values past the ends at the ends, a range of
/// one value 0.
pub fn trigger(v: i32, r: Range) -> u8 {
    if r.max <= r.min {
        return 0;
    }
    let (v, min, max) = (i64::from(v), i64::from(r.min), i64::from(r.max));
    let span = max - min;
    (((v - min).clamp(0, span) * 255 + span / 2) / span) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xinput::buttons;
    use crate::xinput::layout::{axis_byte, axis_byte_down};

    const XBOX: Range = Range {
        min: -32768,
        max: 32767,
    };

    /// xpad's ranges (and Steam's virtual pad's): sticks as XInput's, triggers 0 to 255.
    fn xpad_ranges() -> Vec<Range> {
        let mut r = vec![Range::default(); usize::from(ABS_MAX) + 1];
        for code in [ABS_X, ABS_Y, ABS_RX, ABS_RY] {
            r[usize::from(code)] = XBOX;
        }
        for code in [ABS_Z, ABS_RZ] {
            r[usize::from(code)] = Range { min: 0, max: 255 };
        }
        for code in [ABS_HAT0X, ABS_HAT0Y] {
            r[usize::from(code)] = HAT;
        }
        r
    }

    fn has(codes: &'static [u16]) -> impl Fn(u16) -> bool {
        move |c| codes.contains(&c)
    }

    fn xpad_axes() -> Axes {
        Axes::of(has(&[ABS_X, ABS_Y, ABS_RX, ABS_RY, ABS_Z, ABS_RZ]))
    }

    /// The pads read, and how their face buttons are named.
    #[test]
    fn which_pads_are_read() {
        assert_eq!(faces(0x28de, 0x11ff, None), Some(Faces::ByLetter));
        // The Steam Deck's and the Steam Controller's own controls: Steam stands in for them.
        assert_eq!(faces(0x28de, 0x1205, Some("steam")), None);
        assert_eq!(faces(0x28de, 0x1142, None), None);
        assert_eq!(faces(0x045e, 0x028e, Some("xpad")), Some(Faces::ByLetter));
        assert_eq!(
            faces(0x045e, 0x0b13, Some("microsoft")),
            Some(Faces::ByLetter)
        );
        assert_eq!(
            faces(0x045e, 0x0b13, Some("hid-generic")),
            Some(Faces::ByLetter)
        );
        assert_eq!(
            faces(0x054c, 0x0ce6, Some("playstation")),
            Some(Faces::ByPosition)
        );
        assert_eq!(faces(0x054c, 0x09cc, Some("sony")), Some(Faces::ByPosition));
        assert_eq!(
            faces(0x057e, 0x2009, Some("nintendo")),
            Some(Faces::ByPosition)
        );
        // Another maker's Xbox-style pad through xpad; the same pad through the generic driver
        // (its buttons in its maker's order) is not read.
        assert_eq!(faces(0x046d, 0xc21d, Some("xpad")), Some(Faces::ByLetter));
        assert_eq!(faces(0x2dc8, 0x6001, Some("hid-generic")), None);
        assert_eq!(faces(0x2dc8, 0x6001, None), None);
    }

    /// xpad's range is XInput's: the x axes come through unchanged, the y axes flipped about the
    /// middle with both ends kept.
    #[test]
    fn an_xpad_stick_is_xinputs_value() {
        for v in -32768..=32767 {
            assert_eq!(i32::from(stick(v, XBOX)), v);
            let down = i32::from(stick_down(v, XBOX));
            assert!((down + v).abs() <= 1, "{v}: {down}");
        }
        assert_eq!(stick_down(-32768, XBOX), 32767);
        assert_eq!(stick_down(32767, XBOX), -32768);
        assert_eq!(stick_down(0, XBOX), 0);
        let mut pad = Pad::new(Faces::ByLetter, xpad_axes(), xpad_ranges());
        pad.event(EV_ABS, ABS_Y, -32768);
        pad.event(EV_ABS, ABS_RY, 32767);
        pad.event(EV_ABS, ABS_RX, -32768);
        assert_eq!(pad.event(EV_SYN, SYN_REPORT, 0), Step::Report);
        let c = pad.controls();
        assert_eq!((c.left_y, c.right_y, c.right_x), (32767, -32768, -32768));
    }

    /// A range of 0 to 255 (hid-playstation's) reads as the HID path's bytes do on Windows.
    #[test]
    fn a_byte_range_reads_as_the_hid_bytes() {
        let r = Range { min: 0, max: 255 };
        for v in 0..=255u8 {
            assert_eq!(stick(i32::from(v), r), axis_byte(v), "{v}");
            assert_eq!(stick_down(i32::from(v), r), axis_byte_down(v), "{v}");
            assert_eq!(trigger(i32::from(v), r), v);
        }
    }

    /// Other ranges: the ends are XInput's ends, the middle 0, past the ends the ends.
    #[test]
    fn other_ranges_scale_to_xinputs() {
        for r in [
            Range { min: 0, max: 255 },
            Range {
                min: -127,
                max: 127,
            },
            Range { min: 0, max: 65535 },
            Range {
                min: -32767,
                max: 32767,
            },
            Range {
                min: i32::MIN,
                max: i32::MAX,
            },
        ] {
            assert_eq!(stick(r.min, r), -32768, "{r:?}");
            assert_eq!(stick(r.max, r), 32767, "{r:?}");
            assert_eq!(stick_down(r.min, r), 32767, "{r:?}");
            assert_eq!(stick_down(r.max, r), -32768, "{r:?}");
            let m = middle(r) as i32;
            assert_eq!((stick(m, r), stick_down(m, r)), (0, 0), "{r:?}");
            let (below, above) = (r.min.saturating_sub(10), r.max.saturating_add(10));
            assert_eq!((stick(below, r), stick(above, r)), (-32768, 32767), "{r:?}");
            assert_eq!(trigger(r.min, r), 0, "{r:?}");
            assert_eq!(trigger(r.max, r), 255, "{r:?}");
        }
        assert_eq!(
            stick(
                0,
                Range {
                    min: -127,
                    max: 127
                }
            ),
            0
        );
        assert_eq!(stick(64, Range { min: 0, max: 255 }), -16384);
        assert_eq!(stick(5, Range { min: 5, max: 5 }), 0);
        assert_eq!(stick_down(5, Range { min: 5, max: 5 }), 0);
        // Two values: the lower is the bottom end, the upper the middle.
        assert_eq!(stick(0, Range { min: 0, max: 1 }), -32768);
        assert_eq!(stick(1, Range { min: 0, max: 1 }), 0);
        assert_eq!(stick(2, Range { min: 0, max: 1 }), 32767);
        assert_eq!(trigger(1023, Range { min: 0, max: 1023 }), 255);
        assert_eq!(trigger(0, Range { min: 0, max: 1023 }), 0);
        assert_eq!(trigger(512, Range { min: 0, max: 1023 }), 128);
        assert_eq!(trigger(2000, Range { min: 0, max: 1023 }), 255);
        assert_eq!(trigger(-5, Range { min: 0, max: 255 }), 0);
        assert_eq!(trigger(7, Range { min: 7, max: 7 }), 0);
    }

    /// Face buttons: by letter for an Xbox pad and Steam's virtual one, by position for Sony's
    /// and Nintendo's drivers; the rest the same for all.
    #[test]
    fn face_buttons_by_letter_or_by_position() {
        for (faces, north_is) in [
            (Faces::ByLetter, buttons::X),
            (Faces::ByPosition, buttons::Y),
        ] {
            let mut pad = Pad::new(faces, xpad_axes(), xpad_ranges());
            pad.event(EV_KEY, BTN_NORTH, 1);
            pad.event(EV_SYN, SYN_REPORT, 0);
            assert_eq!(pad.controls().state(0).buttons, north_is, "{faces:?}");
            pad.event(EV_KEY, BTN_NORTH, 0);
            pad.event(EV_KEY, BTN_WEST, 1);
            let west_is = buttons::X ^ buttons::Y ^ north_is;
            assert_eq!(pad.controls().state(0).buttons, west_is, "{faces:?}");
        }
        let mut pad = Pad::new(Faces::ByLetter, xpad_axes(), xpad_ranges());
        for (code, bit) in [
            (BTN_SOUTH, buttons::A),
            (BTN_EAST, buttons::B),
            (BTN_TL, buttons::LEFT_SHOULDER),
            (BTN_TR, buttons::RIGHT_SHOULDER),
            (BTN_SELECT, buttons::BACK),
            (BTN_START, buttons::START),
            (BTN_THUMBL, buttons::LEFT_THUMB),
            (BTN_THUMBR, buttons::RIGHT_THUMB),
            (BTN_DPAD_UP, buttons::DPAD_UP),
            (BTN_DPAD_DOWN, buttons::DPAD_DOWN),
            (BTN_DPAD_LEFT, buttons::DPAD_LEFT),
            (BTN_DPAD_RIGHT, buttons::DPAD_RIGHT),
        ] {
            pad.event(EV_KEY, code, 1);
            assert_eq!(pad.controls().state(0).buttons, bit, "{code:#x}");
            pad.event(EV_KEY, code, 0);
            assert_eq!(pad.controls().state(0).buttons, 0, "{code:#x}");
        }
        // A key's value 2 is the kernel's auto-repeat: still down.
        pad.event(EV_KEY, BTN_SOUTH, 2);
        assert_eq!(pad.controls().state(0).buttons, buttons::A);
        pad.event(EV_KEY, BTN_SOUTH, 0);
        // The guide button is not XInputGetState's, nor are the trigger buttons when the
        // triggers have axes.
        for code in [BTN_MODE, BTN_TL2, BTN_TR2] {
            pad.event(EV_KEY, code, 1);
        }
        let c = pad.controls();
        assert_eq!(
            (c.state(0).buttons, c.left_trigger, c.right_trigger),
            (0, 0, 0)
        );
    }

    #[test]
    fn the_hat_is_the_dpad() {
        let mut pad = Pad::new(Faces::ByPosition, xpad_axes(), xpad_ranges());
        pad.event(EV_ABS, ABS_HAT0X, -1);
        pad.event(EV_ABS, ABS_HAT0Y, 1);
        assert_eq!(
            pad.controls().state(0).buttons,
            buttons::DPAD_LEFT | buttons::DPAD_DOWN
        );
        pad.event(EV_ABS, ABS_HAT0X, 1);
        pad.event(EV_ABS, ABS_HAT0Y, -1);
        assert_eq!(
            pad.controls().state(0).buttons,
            buttons::DPAD_RIGHT | buttons::DPAD_UP
        );
        // A hat on a wider range is a touch pad's position, not a d-pad.
        let mut ranges = xpad_ranges();
        ranges[usize::from(ABS_HAT0X)] = XBOX;
        ranges[usize::from(ABS_HAT0Y)] = XBOX;
        let mut pad = Pad::new(Faces::ByLetter, xpad_axes(), ranges);
        pad.event(EV_ABS, ABS_HAT0X, -20000);
        pad.event(EV_ABS, ABS_HAT0Y, 20000);
        assert_eq!(pad.controls().state(0).buttons, 0);
    }

    /// The axes a pad has decide where its right stick and triggers are.
    #[test]
    fn which_axes() {
        assert_eq!(
            xpad_axes(),
            Axes {
                right_x: ABS_RX,
                right_y: ABS_RY,
                triggers: Some((ABS_Z, ABS_RZ))
            }
        );
        let bluetooth = Axes::of(has(&[ABS_X, ABS_Y, ABS_Z, ABS_RZ, ABS_BRAKE, ABS_GAS]));
        assert_eq!(
            bluetooth,
            Axes {
                right_x: ABS_Z,
                right_y: ABS_RZ,
                triggers: Some((ABS_BRAKE, ABS_GAS))
            }
        );
        let mut ranges = xpad_ranges();
        for code in [ABS_Z, ABS_RZ] {
            ranges[usize::from(code)] = XBOX;
        }
        for code in [ABS_BRAKE, ABS_GAS] {
            ranges[usize::from(code)] = Range { min: 0, max: 1023 };
        }
        let mut pad = Pad::new(Faces::ByLetter, bluetooth, ranges);
        pad.event(EV_ABS, ABS_Z, 32767);
        pad.event(EV_ABS, ABS_RZ, -32768);
        pad.event(EV_ABS, ABS_BRAKE, 1023);
        pad.event(EV_ABS, ABS_GAS, 512);
        let c = pad.controls();
        assert_eq!(
            (c.right_x, c.right_y, c.left_trigger, c.right_trigger),
            (32767, 32767, 255, 128)
        );
        // Digital triggers only (the Switch Pro Controller's ZL and ZR).
        let axes = Axes::of(has(&[ABS_X, ABS_Y, ABS_RX, ABS_RY]));
        assert_eq!(axes.triggers, None);
        let mut pad = Pad::new(Faces::ByPosition, axes, xpad_ranges());
        pad.event(EV_KEY, BTN_TR2, 1);
        let c = pad.controls();
        assert_eq!((c.left_trigger, c.right_trigger), (0, 255));
        assert!(is_pad(has(&[BTN_SOUTH]), has(&[ABS_X, ABS_Y])));
        // A touch pad (a mouse button, no face button) and a motion sensor (no stick's y) are not.
        assert!(!is_pad(has(&[0x110]), has(&[ABS_X, ABS_Y])));
        assert!(!is_pad(has(&[BTN_SOUTH]), has(&[ABS_X])));
    }

    /// After the kernel drops events, the events up to and including the next report are passed
    /// over and the device is to be read again; codes past the ends are ignored.
    #[test]
    fn dropped_events_and_codes_past_the_ends() {
        let mut pad = Pad::new(Faces::ByLetter, xpad_axes(), xpad_ranges());
        assert_eq!(pad.event(EV_KEY, BTN_SOUTH, 1), Step::Nothing);
        assert_eq!(pad.event(EV_SYN, SYN_DROPPED, 0), Step::Nothing);
        assert_eq!(pad.event(EV_KEY, BTN_EAST, 1), Step::Nothing);
        assert_eq!(pad.event(EV_ABS, ABS_X, 1000), Step::Nothing);
        assert_eq!(pad.event(EV_SYN, SYN_REPORT, 0), Step::Resync);
        let c = pad.controls();
        assert!(c.south && !c.east && c.left_x == 0);
        pad.set_key(BTN_EAST, true);
        pad.set_abs(ABS_X, 1000);
        let c = pad.controls();
        assert!(c.east && c.left_x == 1000);
        assert_eq!(pad.event(EV_SYN, SYN_REPORT, 0), Step::Report);
        assert_eq!(pad.event(EV_KEY, 0xffff, 1), Step::Nothing);
        assert_eq!(pad.event(EV_ABS, 0xffff, 1), Step::Nothing);
        assert_eq!(pad.event(0x04, 0x04, 1), Step::Nothing);
        pad.set_key(0xffff, true);
        pad.set_abs(0xffff, 1);
        assert_eq!(pad.controls(), c);
    }
}
