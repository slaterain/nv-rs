//! A HID pad's inputs counted as SDL's DirectInput code counts them, which is what the controller
//! database's `b0`, `a0` and `h0` mean on Windows ([`super::sdl_db`]); pure code, on every system.
//!
//! SDL3's `src/joystick/windows/SDL_dinputjoystick.c` (zlib) takes DirectInput's objects for the
//! pad: the axes DirectInput calls X, Y, Z, Rx, Ry, Rz and up to two sliders (anything else is
//! left out), up to four hats (points of view) and the buttons. It sorts them by where each lies
//! in `DIJOYSTATE2` (the six axes in that order, then the two sliders, the hats, the buttons) and
//! numbers each kind from 0 in that order. Its axes run from -32768 to 32767 with no dead zone;
//! a hat's angle is rounded to the nearest of eight directions, and no angle is the middle.
//!
//! Which of a HID pad's inputs DirectInput makes which object is taken here as the HID usage
//! tables name them, not read from DirectInput itself (no such pad here to compare): the Generic
//! Desktop page's X, Y, Z, Rx, Ry and Rz (0x30 to 0x35) are those axes; its Slider, Dial and Wheel
//! (0x36 to 0x38) sliders; its Hat switch (0x39) a hat; every one-bit input a button; other values
//! (the Simulation page's pedals and wheels among them) none. Windows' HID parser numbers each
//! input of a pad (its data index) in the order its report descriptor gives them; the buttons, and
//! the sliders and hats among themselves, are counted in that order. An axis is scaled from its
//! logical range onto -32768 to 32767, a hat's value onto its circle (a value outside its range is
//! the middle).

use super::sdl_db::{Joystick, HAT_DOWN, HAT_LEFT, HAT_RIGHT, HAT_UP};

/// HID usage pages and Generic Desktop usages (HID Usage Tables).
pub const GENERIC_DESKTOP: u16 = 0x01;
pub const X: u16 = 0x30;
pub const RZ: u16 = 0x35;
pub const SLIDER: u16 = 0x36;
pub const WHEEL: u16 = 0x38;
pub const HAT_SWITCH: u16 = 0x39;

/// One input of a pad as Windows' HID parser describes it (one usage of a button or value
/// capability).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Element {
    pub page: u16,
    pub usage: u16,
    /// Windows' number for it (`DataIndex`), in the report descriptor's order.
    pub index: u16,
    pub kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A one-bit input.
    Button,
    /// A value of `bits` bits, its logical range `min` to `max` (signed when `min` is below 0).
    Value { min: i32, max: i32, bits: u16 },
}

/// A value input: its usage and how to read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value {
    pub page: u16,
    pub usage: u16,
    pub min: i32,
    pub max: i32,
    pub bits: u16,
}

/// A pad's inputs in SDL's order: `buttons[n]` is SDL's `bn`, `axes[n]` its `an`, `hats[n]` its
/// `hn`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub buttons: Vec<(u16, u16)>,
    pub axes: Vec<Value>,
    pub hats: Vec<Value>,
}

impl Layout {
    pub fn of(elements: &[Element]) -> Layout {
        let mut ordered: Vec<&Element> = elements.iter().collect();
        ordered.sort_by_key(|e| e.index);
        let mut buttons = Vec::new();
        // (DIJOYSTATE2's place, the input): the six axes at their own places, sliders after.
        let mut axes: Vec<(u16, Value)> = Vec::new();
        let mut sliders = 0u16;
        let mut hats = Vec::new();
        for e in ordered {
            match e.kind {
                Kind::Button => buttons.push((e.page, e.usage)),
                Kind::Value { min, max, bits } => {
                    let value = Value {
                        page: e.page,
                        usage: e.usage,
                        min,
                        max,
                        bits,
                    };
                    if e.page != GENERIC_DESKTOP {
                        continue;
                    }
                    match e.usage {
                        X..=RZ => axes.push((e.usage - X, value)),
                        SLIDER..=WHEEL if sliders < 2 => {
                            axes.push((6 + sliders, value));
                            sliders += 1;
                        }
                        HAT_SWITCH if hats.len() < 4 => hats.push(value),
                        _ => {}
                    }
                }
            }
        }
        // Stable: two inputs at one place keep their own order.
        axes.sort_by_key(|&(place, _)| place);
        Layout {
            buttons,
            axes: axes.into_iter().map(|(_, v)| v).collect(),
            hats,
        }
    }

    /// The pad's state from its inputs' raw values now: `pressed(page, usage)` for a button,
    /// `raw(page, usage)` for a value (its bits as the report carries them; `None` when the
    /// report did not carry it, read as the middle).
    pub fn joystick(
        &self,
        pressed: impl Fn(u16, u16) -> bool,
        raw: impl Fn(u16, u16) -> Option<u32>,
    ) -> Joystick {
        Joystick {
            buttons: self.buttons.iter().map(|&(p, u)| pressed(p, u)).collect(),
            axes: self
                .axes
                .iter()
                .map(|v| raw(v.page, v.usage).map_or(0, |r| axis(r, v)))
                .collect(),
            hats: self
                .hats
                .iter()
                .map(|v| raw(v.page, v.usage).map_or(0, |r| hat(r, v)))
                .collect(),
        }
    }
}

/// A raw value as the number it means: sign-extended from its bits when its range goes below 0.
pub fn signed(raw: u32, v: &Value) -> i64 {
    let bits = u32::from(v.bits.clamp(1, 32));
    let raw = if bits < 32 {
        raw & ((1u32 << bits) - 1)
    } else {
        raw
    };
    if v.min < 0 && bits < 32 && raw & (1 << (bits - 1)) != 0 {
        i64::from(raw) - (1i64 << bits)
    } else if v.min < 0 && bits == 32 {
        i64::from(raw as i32)
    } else {
        i64::from(raw)
    }
}

/// An axis's value from its logical range onto -32768 ..= 32767, rounded, past the ends at the
/// ends; a range of one value is the middle.
pub fn axis(raw: u32, v: &Value) -> i16 {
    let (min, max) = (i64::from(v.min), i64::from(v.max));
    if max <= min {
        return 0;
    }
    let x = signed(raw, v).clamp(min, max) - min;
    let span = max - min;
    (-32768 + (x * 65535 + span / 2) / span) as i16
}

/// A hat's value as SDL's directions: its range is a circle of that many steps from up,
/// clockwise; a value outside the range is the middle (SDL's `TranslatePOV` on the angle
/// DirectInput gives, in hundredths of a degree).
pub fn hat(raw: u32, v: &Value) -> u8 {
    let (min, max) = (i64::from(v.min), i64::from(v.max));
    let x = signed(raw, v);
    if max < min || x < min || x > max {
        return 0;
    }
    let angle = (x - min) * 36000 / (max - min + 1);
    const DIRECTIONS: [u8; 8] = [
        HAT_UP,
        HAT_UP | HAT_RIGHT,
        HAT_RIGHT,
        HAT_DOWN | HAT_RIGHT,
        HAT_DOWN,
        HAT_DOWN | HAT_LEFT,
        HAT_LEFT,
        HAT_UP | HAT_LEFT,
    ];
    DIRECTIONS[(((angle + 2250) % 36000) / 4500) as usize]
}

/// A value's logical range as a pad gives it, read the way a pad means it: a maximum below a
/// minimum of 0 or more is the field's full unsigned range (a byte's 0 to 255 written as 0 to -1,
/// as some pads' descriptors do).
pub fn logical_range(min: i32, max: i32, bits: u16) -> (i32, i32) {
    if max < min && min >= 0 && (1..32).contains(&bits) {
        (min, ((1u32 << bits) - 1) as i32)
    } else {
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_written_as_signed() {
        assert_eq!(logical_range(0, -1, 8), (0, 255));
        assert_eq!(logical_range(0, -1, 16), (0, 65535));
        assert_eq!(logical_range(0, 255, 8), (0, 255));
        assert_eq!(logical_range(-127, 127, 8), (-127, 127));
        assert_eq!(logical_range(5, 1, 0), (5, 1));
    }
    use crate::xinput::sdl_db::Database;

    const BUTTONS: u16 = 0x09;

    fn buttons(first_index: u16, count: u16) -> Vec<Element> {
        (0..count)
            .map(|n| Element {
                page: BUTTONS,
                usage: n + 1,
                index: first_index + n,
                kind: Kind::Button,
            })
            .collect()
    }

    fn value(usage: u16, index: u16, min: i32, max: i32, bits: u16) -> Element {
        Element {
            page: GENERIC_DESKTOP,
            usage,
            index,
            kind: Kind::Value { min, max, bits },
        }
    }

    /// A pad laid out as the Logitech Dual Action (046d:c216) reports itself: four axes, X, Y, Z
    /// and Rz (bytes here), a hat of eight steps, then twelve buttons. The four axes are SDL's a0
    /// to a3 (Z and Rz the right stick, as the database's line has it), the hat h0, the buttons b0
    /// to b11; the line's own bindings then give the controls.
    #[test]
    fn a_pad_with_z_and_rz_for_its_right_stick() {
        let mut elements = vec![
            value(0x30, 0, 0, 255, 8),
            value(0x31, 1, 0, 255, 8),
            value(0x32, 2, 0, 255, 8),
            value(0x35, 3, 0, 255, 8),
            value(HAT_SWITCH, 4, 0, 7, 4),
        ];
        elements.extend(buttons(5, 12));
        let layout = Layout::of(&elements);
        let usages: Vec<u16> = layout.axes.iter().map(|v| v.usage).collect();
        assert_eq!(usages, [0x30, 0x31, 0x32, 0x35]);
        assert_eq!(layout.hats.len(), 1);
        assert_eq!(
            layout.buttons,
            (1..=12).map(|u| (BUTTONS, u)).collect::<Vec<_>>()
        );
        let m = Database::windows()
            .mapping(0x046d, 0xc216)
            .expect("the Dual Action's line");
        // Button 2 (b1) held, the hat at 2 (right), the right stick pushed up (Rz at 0).
        let j = layout.joystick(
            |page, usage| page == BUTTONS && usage == 2,
            |_, usage| match usage {
                HAT_SWITCH => Some(2),
                0x35 => Some(0),
                _ => Some(128),
            },
        );
        assert_eq!(j.buttons.iter().filter(|&&b| b).count(), 1);
        assert_eq!(j.hats, [HAT_RIGHT]);
        assert_eq!(j.axes, [128, 128, 128, -32768]);
        let c = m.controls(&j);
        assert!(c.right && !c.up && !c.down && !c.left);
        assert_eq!(c.right_y, 32767);
        assert!(c.left_x.abs() < 300 && c.left_y.abs() < 300);
        // Its b1 is the bottom button in the line.
        assert!(c.south);
    }

    /// The axes in DIJOYSTATE2's order whatever the descriptor's (Rz, then Y, Z, X here), sliders
    /// after them in their own order (two at most), other usages left out; hats and buttons in
    /// the descriptor's order.
    #[test]
    fn the_order_of_directinputs_state() {
        let elements = vec![
            value(0x35, 0, 0, 255, 8),
            value(0x31, 1, 0, 255, 8),
            value(0x37, 2, 0, 255, 8), // Dial: the first slider
            value(0x32, 3, 0, 255, 8),
            value(0x36, 4, 0, 255, 8), // Slider: the second slider
            value(0x30, 5, 0, 255, 8),
            value(0x38, 6, 0, 255, 8), // Wheel: a third slider, left out
            value(0x40, 7, 0, 255, 8), // Vx: not an axis SDL takes
            value(HAT_SWITCH, 9, 0, 7, 4),
            value(HAT_SWITCH, 8, 0, 3, 2),
            Element {
                page: 0x02, // Simulation Controls' accelerator: left out
                usage: 0xc4,
                index: 10,
                kind: Kind::Value {
                    min: 0,
                    max: 255,
                    bits: 8,
                },
            },
        ];
        let mut with_buttons = elements.clone();
        with_buttons.extend(buttons(20, 2));
        // A button of another page, before them in the descriptor.
        with_buttons.push(Element {
            page: 0x0c,
            usage: 0x223,
            index: 11,
            kind: Kind::Button,
        });
        let layout = Layout::of(&with_buttons);
        let usages: Vec<u16> = layout.axes.iter().map(|v| v.usage).collect();
        assert_eq!(usages, [0x30, 0x31, 0x32, 0x35, 0x37, 0x36]);
        let hats: Vec<i32> = layout.hats.iter().map(|v| v.max).collect();
        assert_eq!(hats, [3, 7]);
        assert_eq!(layout.buttons, [(0x0c, 0x223), (BUTTONS, 1), (BUTTONS, 2)]);
    }

    /// Axes from their logical range, signed ones too; hats of eight and of four steps, a hat
    /// counted from 1, and its null value.
    #[test]
    fn values_as_sdl_reads_them() {
        let byte = Value {
            page: 1,
            usage: X,
            min: 0,
            max: 255,
            bits: 8,
        };
        assert_eq!((axis(0, &byte), axis(255, &byte)), (-32768, 32767));
        assert_eq!(axis(128, &byte), 128);
        let signed16 = Value {
            min: -32768,
            max: 32767,
            bits: 16,
            ..byte
        };
        assert_eq!(axis(0x8000, &signed16), -32768);
        assert_eq!(axis(0x7fff, &signed16), 32767);
        assert_eq!(axis(0xffff, &signed16), -1);
        let signed8 = Value {
            min: -127,
            max: 127,
            bits: 8,
            ..byte
        };
        assert_eq!(axis(0x81, &signed8), -32768);
        assert_eq!(axis(0x80, &signed8), -32768, "past the end");
        assert_eq!(axis(0, &signed8), 0);
        let ten = Value {
            min: 0,
            max: 1023,
            bits: 10,
            ..byte
        };
        assert_eq!(axis(1023, &ten), 32767);
        assert_eq!(
            axis(
                5,
                &Value {
                    min: 5,
                    max: 5,
                    ..byte
                }
            ),
            0
        );
        let eight = Value {
            usage: HAT_SWITCH,
            min: 0,
            max: 7,
            bits: 4,
            ..byte
        };
        let seen: Vec<u8> = (0..9).map(|r| hat(r, &eight)).collect();
        assert_eq!(
            seen,
            [
                HAT_UP,
                HAT_UP | HAT_RIGHT,
                HAT_RIGHT,
                HAT_DOWN | HAT_RIGHT,
                HAT_DOWN,
                HAT_DOWN | HAT_LEFT,
                HAT_LEFT,
                HAT_UP | HAT_LEFT,
                0
            ]
        );
        assert_eq!(hat(15, &eight), 0);
        let four = Value { max: 3, ..eight };
        let seen: Vec<u8> = (0..4).map(|r| hat(r, &four)).collect();
        assert_eq!(seen, [HAT_UP, HAT_RIGHT, HAT_DOWN, HAT_LEFT]);
        let from_one = Value {
            min: 1,
            max: 8,
            ..eight
        };
        assert_eq!(
            (hat(1, &from_one), hat(5, &from_one), hat(0, &from_one)),
            (HAT_UP, HAT_DOWN, 0)
        );
        let layout = Layout {
            buttons: vec![(BUTTONS, 1)],
            axes: vec![byte],
            hats: vec![eight],
        };
        // A report that does not carry an input reads it as the middle.
        assert_eq!(
            layout.joystick(|_, _| false, |_, _| None),
            Joystick {
                buttons: vec![false],
                axes: vec![0],
                hats: vec![0]
            }
        );
    }
}
