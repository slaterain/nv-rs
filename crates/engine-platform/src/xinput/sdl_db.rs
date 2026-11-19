//! SDL's controller database (SDL_GameControllerDB, zlib): for a pad that is neither an Xbox-style
//! pad (XInput) nor a Sony one, which of its buttons, axes and hats is which control. Pure code, on
//! every system.
//!
//! The data is `data/gamecontrollerdb-windows.txt`: the Windows lines of the database at commit
//! cbdb3678 (its header says where it comes from; its licence is
//! `data/LICENSE-SDL_GameControllerDB`). Read over it, later lines replacing earlier ones for the
//! same pad: a `gamecontrollerdb.txt` in the port's own folder (its Windows lines, as SDL reads a
//! mappings file), then the `SDL_GAMECONTROLLERCONFIG` variable's lines (all of them, as SDL reads
//! the hint), so a player can add or fix a pad without a new build.
//!
//! A line is `GUID,name,mapping...` with the mapping's `output:input` pairs. The GUID's Windows
//! form is SDL's DirectInput one: bus 0x0003 whenever the pad's vendor and product ids are known
//! (whatever its real bus), the two ids, version 0; SDL compares it without the name checksum and,
//! failing an exact match, without the version, so a pad is found by its two ids. The inputs
//! count the pad's buttons, axes and hats as SDL's DirectInput code does (`dinput_order`).
//!
//! How each binding turns into the controls follows SDL3's `src/joystick/SDL_gamepad.c`
//! (`SDL_PrivateParseGamepadElement`, `SDL_GetGamepadAxis`, `SDL_GetGamepadButton`): an output
//! `+x`/`-x` is half an axis, a trigger runs from 0 to 32767; an input `+aN`/`-aN` is half an axis
//! and `aN~` inverts it; an axis read into another range is scaled between the two ranges (only
//! when the value lies inside the input's), the first binding with a value not 0 wins; a button or
//! hat read into an axis is the axis's far end; an axis read into a button is pressed from half
//! way. SDL's `a`, `b`, `x` and `y` are the bottom, right, left and top buttons, so the controls
//! are by position as for every other pad ([`super::layout`]).

use super::layout::Controls;
use std::collections::HashMap;

/// The Windows lines of the database, built in.
pub const WINDOWS: &str = include_str!("../../data/gamecontrollerdb-windows.txt");
/// The player's own lines' file in the port's folder.
pub const FILE: &str = "gamecontrollerdb.txt";

/// SDL's hat directions (a hat's value is their sum).
pub const HAT_UP: u8 = 0x01;
pub const HAT_RIGHT: u8 = 0x02;
pub const HAT_DOWN: u8 = 0x04;
pub const HAT_LEFT: u8 = 0x08;

/// A pad as SDL's joystick layer gives it: its buttons, its axes (-32768 to 32767) and its hats
/// (sums of `HAT_*`), each counted from 0.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Joystick {
    pub buttons: Vec<bool>,
    pub axes: Vec<i16>,
    pub hats: Vec<u8>,
}

impl Joystick {
    fn button(&self, n: usize) -> bool {
        self.buttons.get(n).copied().unwrap_or(false)
    }

    fn hat(&self, n: usize) -> u8 {
        self.hats.get(n).copied().unwrap_or(0)
    }
}

/// The controls SDL's gamepad layer has that the game's XInput state reads (the guide button,
/// the paddles, `misc1` and the touch pad have none, so their bindings are left out).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Button {
    South,
    East,
    West,
    North,
    Back,
    Start,
    LeftStick,
    RightStick,
    LeftShoulder,
    RightShoulder,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    LeftTrigger,
    RightTrigger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Input {
    Button(usize),
    /// An axis read from `min` to `max` (`min` above `max` when inverted).
    Axis {
        axis: usize,
        min: i32,
        max: i32,
    },
    Hat {
        hat: usize,
        mask: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Output {
    Button(Button),
    /// An axis written from `min` to `max` (`max` below `min` for a negative half).
    Axis {
        axis: Axis,
        min: i32,
        max: i32,
    },
}

const AXIS_MIN: i32 = -32768;
const AXIS_MAX: i32 = 32767;

/// One pad's mapping: its name and its bindings, in the line's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub name: String,
    bindings: Vec<(Input, Output)>,
}

fn button(name: &str) -> Option<Button> {
    Some(match name {
        "a" => Button::South,
        "b" => Button::East,
        "x" => Button::West,
        "y" => Button::North,
        "back" => Button::Back,
        "start" => Button::Start,
        "leftstick" => Button::LeftStick,
        "rightstick" => Button::RightStick,
        "leftshoulder" => Button::LeftShoulder,
        "rightshoulder" => Button::RightShoulder,
        "dpup" => Button::Up,
        "dpdown" => Button::Down,
        "dpleft" => Button::Left,
        "dpright" => Button::Right,
        _ => return None,
    })
}

fn axis(name: &str) -> Option<Axis> {
    Some(match name {
        "leftx" => Axis::LeftX,
        "lefty" => Axis::LeftY,
        "rightx" => Axis::RightX,
        "righty" => Axis::RightY,
        "lefttrigger" => Axis::LeftTrigger,
        "righttrigger" => Axis::RightTrigger,
        _ => return None,
    })
}

/// The number at the start of `s` (SDL's `atoi`: digits until the first other character).
fn number(s: &str) -> Option<usize> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    s[..end].parse().ok()
}

/// One `output:input` pair, as `SDL_PrivateParseGamepadElement` reads it; `None` for an output
/// the game has no use for or an input it cannot read.
fn element(output: &str, input: &str) -> Option<(Input, Output)> {
    let (half_out, output) = match output.strip_prefix('+') {
        Some(rest) => (Some('+'), rest),
        None => match output.strip_prefix('-') {
            Some(rest) => (Some('-'), rest),
            None => (None, output),
        },
    };
    let out = if let Some(a) = axis(output) {
        let (min, max) = if matches!(a, Axis::LeftTrigger | Axis::RightTrigger) {
            (0, AXIS_MAX)
        } else {
            match half_out {
                Some('+') => (0, AXIS_MAX),
                Some(_) => (0, AXIS_MIN),
                None => (AXIS_MIN, AXIS_MAX),
            }
        };
        Output::Axis { axis: a, min, max }
    } else {
        Output::Button(button(output)?)
    };
    let (half_in, input) = match input.strip_prefix('+') {
        Some(rest) => (Some('+'), rest),
        None => match input.strip_prefix('-') {
            Some(rest) => (Some('-'), rest),
            None => (None, input),
        },
    };
    let invert = input.ends_with('~');
    let mut chars = input.chars();
    let kind = chars.next()?;
    let rest = chars.as_str();
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let inp = match kind {
        'a' => {
            let (mut min, mut max) = match half_in {
                Some('+') => (0, AXIS_MAX),
                Some(_) => (0, AXIS_MIN),
                None => (AXIS_MIN, AXIS_MAX),
            };
            if invert {
                std::mem::swap(&mut min, &mut max);
            }
            Input::Axis {
                axis: number(rest)?,
                min,
                max,
            }
        }
        'b' => Input::Button(number(rest)?),
        'h' => {
            let (hat, mask) = rest.split_once('.')?;
            if hat.len() != 1 || !mask.starts_with(|c: char| c.is_ascii_digit()) {
                return None;
            }
            Input::Hat {
                hat: number(hat)?,
                mask: number(mask)? as u8,
            }
        }
        _ => return None,
    };
    Some((inp, out))
}

/// Whether `v` lies between `a` and `b` (either way round).
fn within(v: i32, a: i32, b: i32) -> bool {
    if a < b {
        v >= a && v <= b
    } else {
        v >= b && v <= a
    }
}

impl Mapping {
    /// A line's mapping, from its name on (`name,a:b0,...`): unknown fields (`platform:`, `crc:`,
    /// `hint:`, outputs the game has no use for) are passed over, and a binding seen twice is kept
    /// once.
    pub fn parse(after_guid: &str) -> Option<Mapping> {
        let mut fields = after_guid.split(',');
        let name = fields.next()?.to_string();
        let mut bindings = Vec::new();
        for field in fields {
            let Some((output, input)) = field.split_once(':') else {
                continue;
            };
            if let Some(b) = element(output.trim(), input.trim()) {
                if !bindings.contains(&b) {
                    bindings.push(b);
                }
            }
        }
        Some(Mapping { name, bindings })
    }

    /// `SDL_GetGamepadAxis`.
    fn axis(&self, j: &Joystick, axis: Axis) -> i32 {
        for &(input, output) in &self.bindings {
            let Output::Axis {
                axis: a,
                min: out_min,
                max: out_max,
            } = output
            else {
                continue;
            };
            if a != axis {
                continue;
            }
            let mut value = 0;
            match input {
                Input::Axis { axis: n, min, max } => {
                    // A missing axis would read 0: half a trigger.
                    let Some(&v) = j.axes.get(n) else {
                        continue;
                    };
                    value = i32::from(v);
                    if within(value, min, max) {
                        if min != out_min || max != out_max {
                            let normalized = (value - min) as f32 / (max - min) as f32;
                            value = out_min + (normalized * (out_max - out_min) as f32) as i32;
                        }
                    } else {
                        value = 0;
                    }
                }
                Input::Button(n) => {
                    if j.button(n) {
                        value = out_max;
                    }
                }
                Input::Hat { hat, mask } => {
                    if j.hat(hat) & mask != 0 {
                        value = out_max;
                    }
                }
            }
            if value != 0 && within(value, out_min, out_max) {
                return value;
            }
        }
        0
    }

    /// `SDL_GetGamepadButton`.
    fn button(&self, j: &Joystick, button: Button) -> bool {
        let mut down = false;
        for &(input, output) in &self.bindings {
            if output != Output::Button(button) {
                continue;
            }
            down |= match input {
                Input::Axis { axis, min, max } => {
                    let v = j.axes.get(axis).map_or(0, |&v| i32::from(v));
                    // C's division: toward zero.
                    let threshold = min + (max - min) / 2;
                    if min < max {
                        within(v, min, max) && v >= threshold
                    } else {
                        within(v, min, max) && v <= threshold
                    }
                }
                Input::Button(n) => j.button(n),
                Input::Hat { hat, mask } => j.hat(hat) & mask != 0,
            };
        }
        down
    }

    /// The pad's controls as the game's XInput state reads them: SDL's sticks with y turned up
    /// positive, its triggers (0 to 32767) as 0 to 255.
    pub fn controls(&self, j: &Joystick) -> Controls {
        let b = |button| self.button(j, button);
        let stick = |axis| self.axis(j, axis).clamp(AXIS_MIN, AXIS_MAX) as i16;
        let trigger = |axis| {
            let v = self.axis(j, axis).clamp(0, AXIS_MAX);
            ((v * 255 + AXIS_MAX / 2) / AXIS_MAX) as u8
        };
        Controls {
            south: b(Button::South),
            east: b(Button::East),
            west: b(Button::West),
            north: b(Button::North),
            left_shoulder: b(Button::LeftShoulder),
            right_shoulder: b(Button::RightShoulder),
            back: b(Button::Back),
            start: b(Button::Start),
            left_stick: b(Button::LeftStick),
            right_stick: b(Button::RightStick),
            up: b(Button::Up),
            down: b(Button::Down),
            left: b(Button::Left),
            right: b(Button::Right),
            left_x: stick(Axis::LeftX),
            left_y: up_positive(stick(Axis::LeftY)),
            right_x: stick(Axis::RightX),
            right_y: up_positive(stick(Axis::RightY)),
            left_trigger: trigger(Axis::LeftTrigger),
            right_trigger: trigger(Axis::RightTrigger),
        }
    }
}

/// SDL's up-down axis (down positive) as XInput's (up positive): flipped, each side scaled onto
/// the other's length, so both ends stay ends.
fn up_positive(v: i16) -> i16 {
    let v = i32::from(v);
    let flipped = if v < 0 {
        (-v * 32767 + 16384) / 32768
    } else {
        -((v * 32768 + 16383) / 32767)
    };
    flipped.clamp(AXIS_MIN, AXIS_MAX) as i16
}

/// The pad a line's GUID names, when it is in SDL's DirectInput form (bus 0x0003, the vendor and
/// product ids, nothing else but the name checksum and the version, which a match leaves out).
pub fn key(guid: &str) -> Option<(u16, u16)> {
    if guid.len() != 32 || !guid.is_ascii() {
        return None;
    }
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&guid[2 * i..2 * i + 2], 16).ok()?;
    }
    let word = |at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
    let zero = |at: usize| word(at) == 0;
    (word(0) == 0x0003 && zero(6) && zero(10) && zero(14)).then(|| (word(4), word(8)))
}

/// The pads' lines by their ids, the later of two for one pad kept.
#[derive(Clone, Debug, Default)]
pub struct Database {
    lines: HashMap<(u16, u16), String>,
}

impl Database {
    /// The built-in Windows lines.
    pub fn windows() -> Database {
        let mut db = Database::default();
        db.add(WINDOWS, Some("Windows"));
        db
    }

    /// The lines of `text` (one a line, `#` comments and blank lines passed over); with a
    /// `platform`, only the lines whose `platform:` field names it (as SDL reads a mappings file),
    /// without one every line (as SDL reads the hint). Answers how many were taken.
    pub fn add(&mut self, text: &str, platform: Option<&str>) -> usize {
        let mut taken = 0;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((guid, rest)) = line.split_once(',') else {
                continue;
            };
            let Some(id) = key(guid.trim()) else {
                continue;
            };
            if let Some(platform) = platform {
                let named = rest
                    .split(',')
                    .filter_map(|f| f.trim().strip_prefix("platform:"))
                    .any(|p| p.eq_ignore_ascii_case(platform));
                if !named {
                    continue;
                }
            }
            self.lines.insert(id, rest.to_string());
            taken += 1;
        }
        taken
    }

    /// The mapping of the pad `vendor`:`product`, if it has a line.
    pub fn mapping(&self, vendor: u16, product: u16) -> Option<Mapping> {
        Mapping::parse(self.lines.get(&(vendor, product))?)
    }

    /// The built-in lines with the player's read over them: `gamecontrollerdb.txt` in `folder`
    /// (the port's own folder; its Windows lines), then `hint`'s lines (`SDL_GAMECONTROLLERCONFIG`'s,
    /// all of them).
    pub fn with_players_lines(folder: Option<&std::path::Path>, hint: Option<&str>) -> Database {
        let mut db = Database::windows();
        if let Some(text) = folder.and_then(|f| std::fs::read_to_string(f.join(FILE)).ok()) {
            db.add(&text, Some("Windows"));
        }
        if let Some(hint) = hint {
            db.add(hint, None);
        }
        db
    }

    /// How many pads have a line.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(buttons: usize, axes: usize, hats: usize) -> Joystick {
        Joystick {
            buttons: vec![false; buttons],
            axes: vec![0; axes],
            hats: vec![0; hats],
        }
    }

    /// The built-in lines: every Windows line of the copy (858 pads; the `xinput` line names no
    /// pad), found by their two ids.
    #[test]
    fn the_built_in_lines() {
        let db = Database::windows();
        assert_eq!(db.len(), 858);
        assert!(WINDOWS.starts_with("# SDL_GameControllerDB"));
        assert!(WINDOWS.contains("cbdb3678270cf5bf14a127934fd7ab332ecd3cbc"));
        // 8BitDo Pro 2 (2dc8:6006): its line as the copy has it.
        let m = db.mapping(0x2dc8, 0x6006).expect("the 8BitDo Pro 2's line");
        assert_eq!(m.name, "8BitDo Pro 2");
        assert_eq!(db.mapping(0x2dc8, 0xffff), None);
    }

    #[test]
    fn guids_in_sdls_directinput_form() {
        assert_eq!(
            key("03000000c82d00000660000000000000"),
            Some((0x2dc8, 0x6006))
        );
        // The name checksum and the version are left out of a match.
        assert_eq!(
            key("0300abcdc82d00000660000001000000"),
            Some((0x2dc8, 0x6006))
        );
        // Another bus, a driver's mark, the old DirectInput form, or not 32 hex digits: no pad.
        assert_eq!(key("05000000c82d00000660000000000000"), None);
        assert_eq!(key("03000000c82d00000660000000006800"), None);
        assert_eq!(key("c82d0660000000000000504944564944"), None);
        assert_eq!(key("xinput"), None);
        assert_eq!(key("03000000c82d0000066000000000000g"), None);
    }

    /// Later lines for one pad replace earlier ones; a mappings file's lines are taken for their
    /// own platform only, the hint's whatever they say.
    #[test]
    fn lines_read_over_the_built_in_ones() {
        let mut db = Database::windows();
        let line = "03000000c82d00000660000000000000,Mine,a:b9,platform:Windows,";
        assert_eq!(db.add(line, Some("windows")), 1);
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "Mine");
        let linux = "03000000c82d00000660000000000000,Linux,a:b8,platform:Linux,";
        assert_eq!(db.add(linux, Some("Windows")), 0);
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "Mine");
        let hint = "# a comment\n\n03000000c82d00000660000000000000,Hint,a:b7\r\n";
        assert_eq!(db.add(hint, None), 1);
        let m = db.mapping(0x2dc8, 0x6006).unwrap();
        assert_eq!(m.name, "Hint");
        let mut j = pad(10, 0, 0);
        j.buttons[7] = true;
        assert!(m.controls(&j).south);
        assert_eq!(db.add("not a line\nxinput,XInput,a:b0", None), 0);
    }

    /// Buttons, hats and full axes, as most lines bind them; SDL's y down is XInput's y up.
    #[test]
    fn buttons_hats_and_sticks() {
        let m = Mapping::parse(
            "Pad,a:b1,b:b2,x:b0,y:b3,back:b8,start:b9,leftshoulder:b4,rightshoulder:b5,\
             leftstick:b10,rightstick:b11,dpup:h0.1,dpright:h0.2,dpdown:h0.4,dpleft:h0.8,\
             leftx:a0,lefty:a1,rightx:a2,righty:a3,lefttrigger:b6,righttrigger:b7,guide:b12,\
             platform:Windows,",
        )
        .unwrap();
        let mut j = pad(13, 4, 1);
        assert_eq!(m.controls(&j), Controls::default());
        j.buttons[1] = true;
        j.buttons[3] = true;
        j.buttons[7] = true;
        j.buttons[12] = true;
        j.hats[0] = HAT_UP | HAT_LEFT;
        j.axes = vec![-32768, -32768, 32767, 32767];
        let c = m.controls(&j);
        assert!(c.south && c.north && !c.east && !c.west);
        assert!(c.up && c.left && !c.down && !c.right);
        assert_eq!((c.left_trigger, c.right_trigger), (0, 255));
        assert_eq!(
            (c.left_x, c.left_y, c.right_x, c.right_y),
            (-32768, 32767, 32767, -32768)
        );
        assert_eq!(up_positive(0), 0);
    }

    /// Triggers on a full axis run from its bottom end (0) to its top end (255), as SDL scales
    /// them (in floats, cut toward zero); an inverted half axis runs the other way.
    #[test]
    fn triggers_and_half_axes() {
        let m = Mapping::parse("Pad,lefttrigger:a2,righttrigger:-a3~,").unwrap();
        let mut j = pad(0, 4, 0);
        j.axes[2] = -32768;
        j.axes[3] = -32768;
        let c = m.controls(&j);
        assert_eq!((c.left_trigger, c.right_trigger), (0, 0));
        j.axes[2] = 0;
        assert_eq!(m.controls(&j).left_trigger, 127);
        j.axes[2] = 32767;
        assert_eq!(m.controls(&j).left_trigger, 255);
        // -a3~: the negative half inverted, -32768 to 0 read as the trigger's 0 to 32767.
        j.axes[3] = -16384;
        assert_eq!(m.controls(&j).right_trigger, 127);
        j.axes[3] = 0;
        assert_eq!(m.controls(&j).right_trigger, 255);
        // Outside the half: nothing.
        j.axes[3] = 1;
        assert_eq!(m.controls(&j).right_trigger, 0);
        // A stick from two half axes (the "+righty:a4,-righty:a3" kind) and from two buttons.
        let m = Mapping::parse("Pad,+righty:+a4,-righty:-a3,-leftx:b0,+leftx:b1,").unwrap();
        let mut j = pad(2, 5, 0);
        j.axes[4] = 32767;
        assert_eq!(m.controls(&j).right_y, -32768);
        j.axes[4] = 0;
        j.axes[3] = -32768;
        assert_eq!(m.controls(&j).right_y, 32767);
        j.buttons[1] = true;
        assert_eq!(m.controls(&j).left_x, 32767);
        j.buttons = vec![true, false];
        assert_eq!(m.controls(&j).left_x, -32768);
    }

    /// An axis read into a button is pressed from half way (on a full axis that is -1, as in
    /// SDL); a hat read into an axis is its far end; a binding to a missing axis is passed over.
    #[test]
    fn axes_into_buttons_and_hats_into_axes() {
        let m = Mapping::parse("Pad,a:+a0,b:a1,x:-a2,leftx:h0.2,lefty:a9,").unwrap();
        let mut j = pad(0, 3, 1);
        j.axes = vec![16382, -2, -16384];
        let c = m.controls(&j);
        assert!(!c.south && !c.east && c.west);
        j.axes = vec![16383, -1, -16383];
        let c = m.controls(&j);
        assert!(c.south && c.east && !c.west);
        j.hats[0] = HAT_RIGHT;
        let c = m.controls(&j);
        assert_eq!((c.left_x, c.left_y), (32767, 0));
    }

    /// Fields the game has no use for and inputs that are not SDL's are passed over; a binding
    /// given twice counts once.
    #[test]
    fn what_is_passed_over() {
        let m = Mapping::parse(
            "Pad,guide:b5,misc1:b6,paddle1:b7,touchpad:b8,a:b0,a:b0,b:q1,x:b,y:h0,crc:1234,\
             hint:!SDL_X:=1,platform:Windows",
        )
        .unwrap();
        assert_eq!(m.bindings.len(), 1);
        assert_eq!(m.name, "Pad");
        let mut j = pad(9, 0, 0);
        for n in 5..9 {
            j.buttons[n] = true;
        }
        assert_eq!(m.controls(&j), Controls::default());
        // Inputs past the pad's own count read as nothing.
        let m = Mapping::parse("Pad,a:b40,dpup:h3.1").unwrap();
        assert_eq!(m.controls(&pad(1, 0, 1)), Controls::default());
    }

    /// The player's file in the port's folder, then the hint, over the built-in lines; no file
    /// or no hint leaves the built-in ones.
    #[test]
    fn the_players_lines() {
        let dir = std::env::temp_dir().join(format!("nv-rs-pad-db-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(FILE),
            "03000000c82d00000660000000000000,File,a:b0,platform:Windows,\n\
             03000000aaaa0000bbbb000000000000,New,a:b0,platform:Windows,\n\
             03000000cccc0000dddd000000000000,Mac,a:b0,platform:Mac OS X,\n",
        )
        .unwrap();
        let db = Database::with_players_lines(Some(&dir), None);
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "File");
        assert_eq!(db.mapping(0xaaaa, 0xbbbb).unwrap().name, "New");
        assert_eq!(db.mapping(0xcccc, 0xdddd), None);
        assert_eq!(db.len(), 859);
        let hint = "03000000c82d00000660000000000000,Hint,a:b0";
        let db = Database::with_players_lines(Some(&dir), Some(hint));
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "Hint");
        std::fs::remove_dir_all(&dir).unwrap();
        let db = Database::with_players_lines(Some(&dir), None);
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "8BitDo Pro 2");
        assert_eq!(Database::with_players_lines(None, None).len(), 858);
    }
}
