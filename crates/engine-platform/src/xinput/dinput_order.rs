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
//! Which of a HID pad's inputs DirectInput makes which object is taken here from the HID usage
//! tables and, for the Simulation page, Wine's DirectInput as #804's review gives it; it is not
//! read from DirectInput itself (no such pad here to compare). Generic Desktop's X, Y, Z, Rx, Ry
//! and Rz (0x30 to 0x35) are those axes; the Simulation page's Steering takes X's place,
//! Accelerator Y's, and Brake and Rudder Rz's, and Generic Desktop's Wheel (0x38) Z's (each after
//! the Generic Desktop axis of that place, when there is one); Generic Desktop's Slider and Dial
//! (0x36, 0x37) and the Simulation page's Throttle are sliders; the Hat switch (0x39) is a hat; every one-bit input is a button;
//! other values are none. Windows' HID parser numbers each input of a pad (its data index) in the
//! order its report descriptor gives them; the buttons, and the sliders and hats among
//! themselves, are counted in that order. An axis is scaled from its logical range onto -32768 to
//! 32767, a hat's value onto its circle (a value outside its range is the middle).

use super::sdl_db::{Joystick, Mapping, HAT_DOWN, HAT_LEFT, HAT_RIGHT, HAT_UP};

/// HID usage pages and Generic Desktop usages (HID Usage Tables).
pub const GENERIC_DESKTOP: u16 = 0x01;
pub const X: u16 = 0x30;
pub const RZ: u16 = 0x35;
pub const SLIDER: u16 = 0x36;
pub const DIAL: u16 = 0x37;
pub const WHEEL: u16 = 0x38;
pub const HAT_SWITCH: u16 = 0x39;
/// The Simulation Controls page and the usages DirectInput gives axis places (as Wine's DirectInput,
/// modelled on Windows', does): Steering is X's, Accelerator Y's, Brake and Rudder Rz's, Throttle a slider.
pub const SIMULATION: u16 = 0x02;
pub const RUDDER: u16 = 0xba;
pub const THROTTLE: u16 = 0xbb;
pub const ACCELERATOR: u16 = 0xc4;
pub const BRAKE: u16 = 0xc5;
pub const STEERING: u16 = 0xc8;

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
        // (DIJOYSTATE2's place, Generic Desktop before the Simulation page at one place, the
        // input): the six axes at their own places, sliders after.
        let mut axes: Vec<(u16, u8, Value)> = Vec::new();
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
                    let rank = u8::from(e.page != GENERIC_DESKTOP);
                    match (e.page, e.usage) {
                        (GENERIC_DESKTOP, X..=RZ) => axes.push((e.usage - X, rank, value)),
                        (SIMULATION, STEERING) => axes.push((0, rank, value)),
                        (SIMULATION, ACCELERATOR) => axes.push((1, rank, value)),
                        (SIMULATION, BRAKE | RUDDER) => axes.push((5, rank, value)),
                        // Wine's DirectInput gives Generic Desktop's Wheel Z's place, after Z itself.
                        (GENERIC_DESKTOP, WHEEL) => axes.push((2, 1, value)),
                        (GENERIC_DESKTOP, SLIDER..=DIAL) | (SIMULATION, THROTTLE)
                            if sliders < 2 =>
                        {
                            axes.push((6 + sliders, rank, value));
                            sliders += 1;
                        }
                        (GENERIC_DESKTOP, HAT_SWITCH) if hats.len() < 4 => hats.push(value),
                        _ => {}
                    }
                }
            }
        }
        // Stable: two inputs at one place and of one page keep their own order.
        axes.sort_by_key(|&(place, rank, _)| (place, rank));
        Layout {
            buttons,
            axes: axes.into_iter().map(|(_, _, v)| v).collect(),
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

impl Layout {
    /// The usage pages of the pad's buttons, each once, in order.
    pub fn button_pages(&self) -> Vec<u16> {
        let mut pages: Vec<u16> = self.buttons.iter().map(|&(page, _)| page).collect();
        pages.sort_unstable();
        pages.dedup();
        pages
    }

    /// A database line as it fits this pad ([`Mapping::fitted`]): the sticks must be on Generic
    /// Desktop axes, never on a Simulation-page one; `Err` when the line is not to be used.
    pub fn fits(&self, mapping: &Mapping) -> Result<(Mapping, Vec<String>), String> {
        let desktop: Vec<bool> = self
            .axes
            .iter()
            .map(|v| v.page == GENERIC_DESKTOP)
            .collect();
        mapping.fitted(self.buttons.len(), &desktop, self.hats.len())
    }

    /// The pad's values, the axes then the hats, as `(page, usage)`.
    pub fn value_ids(&self) -> Vec<(u16, u16)> {
        self.axes
            .iter()
            .chain(&self.hats)
            .map(|v| (v.page, v.usage))
            .collect()
    }
}

/// What a pad's input reports have said, kept from one report to the next: the buttons held on
/// each page a report carried, by the report's id, and each value's last reading. Windows' HID
/// parser answers "not in this report" for the pages and values another report id carries; those
/// keep what they had, and a page whose buttons two report ids share is released only by the id
/// that holds them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Readings {
    /// (report id, page, usage).
    pressed: std::collections::BTreeSet<(u8, u16, u16)>,
    values: std::collections::BTreeMap<(u16, u16), u32>,
}

impl Readings {
    /// One report with id `report_id` (its first byte; 0 for a pad without ids): `pages` each
    /// button page with the usages it holds down, `None` when the report does not carry the page;
    /// `values` each value with its raw reading, `None` when the report does not carry it. Answers
    /// the pad's state after it, `None` when the report carried none of the pad's inputs (nothing
    /// for the controls).
    pub fn report(
        &mut self,
        layout: &Layout,
        report_id: u8,
        pages: &[(u16, Option<Vec<u16>>)],
        values: &[((u16, u16), Option<u32>)],
    ) -> Option<Joystick> {
        let mut carried = false;
        for (page, held) in pages {
            if let Some(held) = held {
                carried = true;
                self.pressed
                    .retain(|&(id, p, _)| !(id == report_id && p == *page));
                for &usage in held {
                    self.pressed.insert((report_id, *page, usage));
                }
            }
        }
        for &(id, raw) in values {
            if let Some(raw) = raw {
                carried = true;
                self.values.insert(id, raw);
            }
        }
        carried.then(|| {
            layout.joystick(
                |page, usage| {
                    self.pressed
                        .iter()
                        .any(|&(_, p, u)| p == page && u == usage)
                },
                |page, usage| self.values.get(&(page, usage)).copied(),
            )
        })
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
    /// after them in their own order (two at most), a wheel after Z, other usages left out; hats
    /// and buttons in the descriptor's order.
    #[test]
    fn the_order_of_directinputs_state() {
        let elements = vec![
            value(0x35, 0, 0, 255, 8),
            value(0x31, 1, 0, 255, 8),
            value(0x37, 2, 0, 255, 8), // Dial: the first slider
            value(0x32, 3, 0, 255, 8),
            value(0x36, 4, 0, 255, 8), // Slider: the second slider
            value(0x30, 5, 0, 255, 8),
            value(0x38, 6, 0, 255, 8), // Wheel: at Z's place, after Z (Wine's DirectInput)
            value(0x40, 7, 0, 255, 8), // Vx: not an axis SDL takes
            value(HAT_SWITCH, 9, 0, 7, 4),
            value(HAT_SWITCH, 8, 0, 3, 2),
            Element {
                page: 0x02, // Simulation Controls' accelerator: at Y's place, after Y
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
        assert_eq!(usages, [0x30, 0x31, 0xc4, 0x32, 0x38, 0x35, 0x37, 0x36]);
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

    /// A pad laid out as #804's review gives the 8BitDo Pro 2's in its DirectInput mode (2dc8:6006):
    /// X, Y, Z and Rz, then the Simulation page's Brake and Accelerator. DirectInput puts the
    /// accelerator at Y's place and the brake at Rz's, so the axes are X, Y, Accelerator, Z, Rz,
    /// Brake, and the built-in line's rightx:a3 and righty:a4 are Z and Rz.
    #[test]
    fn a_pad_with_simulation_triggers() {
        let simulation = |usage: u16, index: u16| Element {
            page: SIMULATION,
            usage,
            index,
            kind: Kind::Value {
                min: 0,
                max: 255,
                bits: 8,
            },
        };
        let mut elements = vec![
            value(0x30, 0, 0, 255, 8),
            value(0x31, 1, 0, 255, 8),
            value(0x32, 2, 0, 255, 8),
            value(0x35, 3, 0, 255, 8),
            simulation(BRAKE, 4),
            simulation(ACCELERATOR, 5),
            value(HAT_SWITCH, 6, 0, 7, 4),
        ];
        elements.extend(buttons(7, 15));
        let layout = Layout::of(&elements);
        let usages: Vec<u16> = layout.axes.iter().map(|v| v.usage).collect();
        assert_eq!(usages, [0x30, 0x31, ACCELERATOR, 0x32, 0x35, BRAKE]);
        let m = Database::windows()
            .mapping(0x2dc8, 0x6006)
            .expect("the 8BitDo Pro 2's line");
        // The right stick pushed right (Z at its top end) and up (Rz at its bottom end).
        let j = layout.joystick(
            |_, _| false,
            |page, usage| match (page, usage) {
                (GENERIC_DESKTOP, 0x32) => Some(255),
                (GENERIC_DESKTOP, 0x35) => Some(0),
                _ => Some(128),
            },
        );
        let c = m.controls(&j);
        assert_eq!((c.right_x, c.right_y), (32767, 32767));
        assert!(c.left_x.abs() < 300 && c.left_y.abs() < 300);
        // Steering and Rudder at X's and Rz's places, after the Generic Desktop ones; Throttle a
        // slider.
        let more = vec![
            simulation(RUDDER, 0),
            value(0x35, 1, 0, 255, 8),
            simulation(STEERING, 2),
            value(0x30, 3, 0, 255, 8),
            simulation(THROTTLE, 4),
        ];
        let usages: Vec<u16> = Layout::of(&more).axes.iter().map(|v| v.usage).collect();
        assert_eq!(usages, [0x30, STEERING, 0x35, RUDDER, THROTTLE]);
    }

    /// One report after another (review of #804): a button let go reads up at the next report
    /// that carries its page; a report of another id leaves every input as it was; a report with
    /// values only still counts; a value keeps its reading until a report carries it again.
    #[test]
    fn readings_from_report_to_report() {
        let mut elements = vec![value(0x30, 0, 0, 255, 8), value(HAT_SWITCH, 1, 0, 7, 4)];
        elements.extend(buttons(2, 4));
        let layout = Layout::of(&elements);
        assert_eq!(layout.button_pages(), [BUTTONS]);
        assert_eq!(layout.value_ids(), [(1, 0x30), (1, HAT_SWITCH)]);
        let mut r = Readings::default();
        let x = (GENERIC_DESKTOP, 0x30);
        let hat = (GENERIC_DESKTOP, HAT_SWITCH);
        let j = r
            .report(
                &layout,
                0,
                &[(BUTTONS, Some(vec![1, 3]))],
                &[(x, Some(255)), (hat, Some(2))],
            )
            .unwrap();
        assert_eq!(j.buttons, [true, false, true, false]);
        assert_eq!((j.axes[0], j.hats[0]), (32767, HAT_RIGHT));
        // Button 1 let go.
        let j = r
            .report(
                &layout,
                0,
                &[(BUTTONS, Some(vec![3]))],
                &[(x, None), (hat, None)],
            )
            .unwrap();
        assert_eq!(j.buttons, [false, false, true, false]);
        assert_eq!((j.axes[0], j.hats[0]), (32767, HAT_RIGHT), "values kept");
        // Another report id: nothing carried, nothing changes.
        assert_eq!(
            r.report(&layout, 0, &[(BUTTONS, None)], &[(x, None), (hat, None)]),
            None
        );
        // Values only.
        let j = r
            .report(&layout, 0, &[(BUTTONS, None)], &[(x, Some(0)), (hat, None)])
            .unwrap();
        assert_eq!(j.buttons, [false, false, true, false], "buttons kept");
        assert_eq!((j.axes[0], j.hats[0]), (-32768, HAT_RIGHT));
        // Before any reading a value is the middle.
        let mut fresh = Readings::default();
        let j = fresh
            .report(
                &layout,
                0,
                &[(BUTTONS, Some(vec![]))],
                &[(x, None), (hat, None)],
            )
            .unwrap();
        assert_eq!((j.axes[0], j.hats[0]), (0, 0));
    }

    /// Generic Desktop's Wheel takes Z's place, after Z (Wine's DirectInput; review of #804), not a
    /// slider's; Slider and Dial stay sliders.
    #[test]
    fn a_wheel_is_at_zs_place() {
        let elements = vec![
            value(0x36, 0, 0, 255, 8),
            value(WHEEL, 1, 0, 255, 8),
            value(0x35, 2, 0, 255, 8),
            value(0x32, 3, 0, 255, 8),
            value(DIAL, 4, 0, 255, 8),
            value(0x30, 5, 0, 255, 8),
        ];
        let usages: Vec<u16> = Layout::of(&elements).axes.iter().map(|v| v.usage).collect();
        assert_eq!(usages, [0x30, 0x32, WHEEL, 0x35, 0x36, DIAL]);
    }

    /// Two report ids that both carry Button-page usages (review of #804): a report of one id
    /// releases only what that id held, not the other id's buttons.
    #[test]
    fn two_report_ids_sharing_the_button_page() {
        let layout = Layout::of(&buttons(0, 4));
        let mut r = Readings::default();
        let j = r
            .report(&layout, 1, &[(BUTTONS, Some(vec![1]))], &[])
            .unwrap();
        assert_eq!(j.buttons, [true, false, false, false]);
        // Id 2 holds button 3 and says nothing of button 1.
        let j = r
            .report(&layout, 2, &[(BUTTONS, Some(vec![3]))], &[])
            .unwrap();
        assert_eq!(j.buttons, [true, false, true, false]);
        // Id 2 lets go of its button; id 1's stays down until id 1 says otherwise.
        let j = r
            .report(&layout, 2, &[(BUTTONS, Some(vec![]))], &[])
            .unwrap();
        assert_eq!(j.buttons, [true, false, false, false]);
        let j = r
            .report(&layout, 1, &[(BUTTONS, Some(vec![]))], &[])
            .unwrap();
        assert_eq!(j.buttons, [false; 4]);
    }

    /// A line is used only if it fits the pad (review of #804): a PS4-style layout with six Generic
    /// Desktop axes (X, Y, Z, Rx, Ry, Rz) fits a line with the right stick on a2/a5; the same line
    /// on X, Y, Z, Rz plus Accelerator and Brake (X, Y, Accelerator, Z, Rz, Brake) puts the right
    /// stick on a pedal and does not fit; on four axes alone a5 does not exist; a button or hat
    /// past the pad's does not fit either.
    #[test]
    fn a_line_fits_the_pads_own_inputs_or_is_not_used() {
        let line = "PS4-style,a:b1,b:b2,x:b0,y:b3,start:b9,leftx:a0,lefty:a1,rightx:a2,righty:a5,\
                    lefttrigger:a3,righttrigger:a4,dpup:h0.1";
        let m = Mapping::parse(line).unwrap();
        let mut six: Vec<Element> = [0x30, 0x31, 0x32, 0x33, 0x34, 0x35]
            .iter()
            .enumerate()
            .map(|(i, &u)| value(u, i as u16, 0, 255, 8))
            .collect();
        six.push(value(HAT_SWITCH, 6, 0, 7, 4));
        six.extend(buttons(7, 14));
        assert!(Layout::of(&six)
            .fits(&m)
            .is_ok_and(|(_, notes)| notes.is_empty()));
        let pedal = |usage: u16, index: u16| Element {
            page: SIMULATION,
            usage,
            index,
            kind: Kind::Value {
                min: 0,
                max: 255,
                bits: 8,
            },
        };
        let mut pedals = vec![
            value(0x30, 0, 0, 255, 8),
            value(0x31, 1, 0, 255, 8),
            value(0x32, 2, 0, 255, 8),
            value(0x35, 3, 0, 255, 8),
            pedal(BRAKE, 4),
            pedal(ACCELERATOR, 5),
            value(HAT_SWITCH, 6, 0, 7, 4),
        ];
        pedals.extend(buttons(7, 16));
        let why = Layout::of(&pedals).fits(&m).unwrap_err();
        assert!(
            why.contains("RightX is a2") && why.contains("pedal"),
            "{why}"
        );
        let mut four: Vec<Element> = pedals[..4].to_vec();
        four.push(value(HAT_SWITCH, 6, 0, 7, 4));
        four.extend(buttons(7, 14));
        let why = Layout::of(&four).fits(&m).unwrap_err();
        assert!(
            why.contains("a3") || why.contains("a4") || why.contains("a5"),
            "{why}"
        );
        // The 8BitDo Pro 2's own line fits the pedals layout (its sticks on Z and Rz).
        let pro2 = Database::windows().mapping(0x2dc8, 0x6006).unwrap();
        assert!(Layout::of(&pedals).fits(&pro2).is_ok());
        let mut few = six[..7].to_vec();
        few.extend(buttons(7, 8));
        assert!(Layout::of(&few).fits(&m).unwrap_err().contains("b9"));
        let no_hat: Vec<Element> = six
            .iter()
            .copied()
            .filter(|e| e.usage != HAT_SWITCH || e.page != GENERIC_DESKTOP)
            .collect();
        assert!(Layout::of(&no_hat).fits(&m).unwrap_err().contains("h0"));
    }
}
