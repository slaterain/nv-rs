//! A pad's controls by position, mapped to the XInput state the game reads, the same table on
//! every system and for every pad: the bottom face button is A, the right one B, the left one X,
//! the top one Y (Cross, Circle, Square, Triangle on a PlayStation pad; B, A, Y, X on a Nintendo
//! one), the two shoulder buttons, the two triggers, the left of the two middle buttons Back
//! (Create, Share, Minus, View), the right one Start (Options, Plus, Menu), the stick clicks and the
//! d-pad. A pad's home button (PS, Home, the Xbox button), a touchpad and a mute button have no
//! XInput button the game reads (`XInputGetState` never reports the guide button), so they are not
//! mapped. The game draws Xbox buttons whatever the pad.

use super::{buttons, PadState};

/// One reading of a pad's controls: the buttons by position, the sticks as XInput gives them
/// (-32768 to 32767, right and up positive) and the triggers (0 to 255).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Controls {
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub left_shoulder: bool,
    pub right_shoulder: bool,
    pub back: bool,
    pub start: bool,
    pub left_stick: bool,
    pub right_stick: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
    pub left_trigger: u8,
    pub right_trigger: u8,
}

impl Controls {
    /// The XInput state with the packet number `packet`.
    pub fn state(&self, packet: u32) -> PadState {
        let bits = [
            (self.up, buttons::DPAD_UP),
            (self.down, buttons::DPAD_DOWN),
            (self.left, buttons::DPAD_LEFT),
            (self.right, buttons::DPAD_RIGHT),
            (self.start, buttons::START),
            (self.back, buttons::BACK),
            (self.left_stick, buttons::LEFT_THUMB),
            (self.right_stick, buttons::RIGHT_THUMB),
            (self.left_shoulder, buttons::LEFT_SHOULDER),
            (self.right_shoulder, buttons::RIGHT_SHOULDER),
            (self.south, buttons::A),
            (self.east, buttons::B),
            (self.west, buttons::X),
            (self.north, buttons::Y),
        ];
        PadState {
            packet,
            buttons: bits
                .iter()
                .filter(|(on, _)| *on)
                .fold(0, |mask, (_, bit)| mask | bit),
            left_trigger: self.left_trigger,
            right_trigger: self.right_trigger,
            thumb_lx: self.left_x,
            thumb_ly: self.left_y,
            thumb_rx: self.right_x,
            thumb_ry: self.right_y,
        }
    }

    /// The d-pad from a hat's direction: 0 up, then clockwise in eighths to 7 (up and left); any
    /// other value (8 or 15 on the pads that report one) is the hat let go.
    pub fn set_hat(&mut self, hat: u8) {
        let (up, right, down, left) = match hat {
            0 => (true, false, false, false),
            1 => (true, true, false, false),
            2 => (false, true, false, false),
            3 => (false, true, true, false),
            4 => (false, false, true, false),
            5 => (false, false, true, true),
            6 => (false, false, false, true),
            7 => (true, false, false, true),
            _ => (false, false, false, false),
        };
        self.up = up;
        self.right = right;
        self.down = down;
        self.left = left;
    }
}

/// A stick's byte (0 to 255, 128 the middle; 0 left, or up for [`axis_byte_down`]) as XInput's
/// value: the middle 0, the ends -32768 and 32767, each half scaled on its own and rounded.
pub fn axis_byte(v: u8) -> i16 {
    scale_halves(i32::from(v) - 128, 128, 127)
}

/// [`axis_byte`] for a stick's up-down byte, which grows downward: XInput's grows upward.
pub fn axis_byte_down(v: u8) -> i16 {
    scale_halves(128 - i32::from(v), 127, 128)
}

/// A stick's axis given from -1 to 1 (up and right positive, as macOS gives them) as XInput's
/// value, each side scaled on its own and rounded; NaN is the middle.
pub fn axis_unit(v: f32) -> i16 {
    let v = if v.is_nan() { 0.0 } else { v.clamp(-1.0, 1.0) };
    let scaled = if v >= 0.0 { v * 32767.0 } else { v * 32768.0 };
    scaled.round() as i16
}

/// A trigger given from 0 to 1 as XInput's 0 to 255, rounded; NaN is let go.
pub fn trigger_unit(v: f32) -> u8 {
    let v = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    (v * 255.0).round() as u8
}

/// `d` from `-below ..= above` onto -32768 ..= 32767, each side scaled on its own, rounded to the
/// nearest (halves away from zero).
fn scale_halves(d: i32, below: i32, above: i32) -> i16 {
    let scaled = if d >= 0 {
        (d * 2 * 32767 + above) / (2 * above)
    } else {
        -((-d * 2 * 32768 + below) / (2 * below))
    };
    scaled.clamp(-32768, 32767) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every position lands on its XInput bit and no other.
    #[test]
    fn each_control_is_one_xinput_button() {
        let all = Controls {
            south: true,
            east: true,
            west: true,
            north: true,
            left_shoulder: true,
            right_shoulder: true,
            back: true,
            start: true,
            left_stick: true,
            right_stick: true,
            up: true,
            down: true,
            left: true,
            right: true,
            ..Controls::default()
        };
        assert_eq!(all.state(0).buttons, 0xf3ff);
        let one = |c: Controls| c.state(0).buttons;
        let base = Controls::default();
        assert_eq!(
            one(Controls {
                south: true,
                ..base
            }),
            buttons::A
        );
        assert_eq!(one(Controls { east: true, ..base }), buttons::B);
        assert_eq!(one(Controls { west: true, ..base }), buttons::X);
        assert_eq!(
            one(Controls {
                north: true,
                ..base
            }),
            buttons::Y
        );
        assert_eq!(one(Controls { back: true, ..base }), buttons::BACK);
        assert_eq!(
            one(Controls {
                start: true,
                ..base
            }),
            buttons::START
        );
        assert_eq!(
            one(Controls {
                left_shoulder: true,
                ..base
            }),
            buttons::LEFT_SHOULDER
        );
        assert_eq!(
            one(Controls {
                right_stick: true,
                ..base
            }),
            buttons::RIGHT_THUMB
        );
    }

    #[test]
    fn the_hat_is_the_dpad() {
        let mut c = Controls::default();
        let want = [
            buttons::DPAD_UP,
            buttons::DPAD_UP | buttons::DPAD_RIGHT,
            buttons::DPAD_RIGHT,
            buttons::DPAD_RIGHT | buttons::DPAD_DOWN,
            buttons::DPAD_DOWN,
            buttons::DPAD_DOWN | buttons::DPAD_LEFT,
            buttons::DPAD_LEFT,
            buttons::DPAD_LEFT | buttons::DPAD_UP,
        ];
        for (hat, bits) in want.iter().enumerate() {
            c.set_hat(hat as u8);
            assert_eq!(c.state(0).buttons, *bits, "hat {hat}");
        }
        for released in [8, 9, 15, 0xff] {
            c.set_hat(released);
            assert_eq!(c.state(0).buttons, 0);
        }
    }

    /// The stick bytes' ends are XInput's ends, the middle 0, and the scale never goes back.
    #[test]
    fn stick_bytes_cover_xinputs_range() {
        assert_eq!(
            (axis_byte(0), axis_byte(128), axis_byte(255)),
            (-32768, 0, 32767)
        );
        assert_eq!(
            (axis_byte_down(0), axis_byte_down(128), axis_byte_down(255)),
            (32767, 0, -32768)
        );
        for v in 0..255u8 {
            assert!(axis_byte(v) < axis_byte(v + 1));
            assert!(axis_byte_down(v) > axis_byte_down(v + 1));
        }
        // 64 of 127 above the middle: 16512.5 rounded up; 64 of 128 below: exactly half.
        assert_eq!(axis_byte(192), 16513);
        assert_eq!(axis_byte(64), -16384);
    }

    #[test]
    fn unit_axes_and_triggers() {
        assert_eq!(
            (axis_unit(-1.0), axis_unit(0.0), axis_unit(1.0)),
            (-32768, 0, 32767)
        );
        assert_eq!((axis_unit(0.5), axis_unit(-0.5)), (16384, -16384));
        assert_eq!(
            (axis_unit(2.0), axis_unit(-7.0), axis_unit(f32::NAN)),
            (32767, -32768, 0)
        );
        assert_eq!(
            (trigger_unit(0.0), trigger_unit(1.0), trigger_unit(0.5)),
            (0, 255, 128)
        );
        assert_eq!(
            (
                trigger_unit(-1.0),
                trigger_unit(3.0),
                trigger_unit(f32::NAN)
            ),
            (0, 255, 0)
        );
    }

    #[test]
    fn the_state_carries_sticks_triggers_and_packet() {
        let c = Controls {
            left_x: -5,
            left_y: 6,
            right_x: -32768,
            right_y: 32767,
            left_trigger: 1,
            right_trigger: 254,
            ..Controls::default()
        };
        let s = c.state(42);
        assert_eq!(
            (
                s.packet,
                s.thumb_lx,
                s.thumb_ly,
                s.thumb_rx,
                s.thumb_ry,
                s.left_trigger,
                s.right_trigger
            ),
            (42, -5, 6, -32768, 32767, 1, 254)
        );
    }
}
