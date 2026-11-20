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
//! (whatever its real bus), the two ids, version 0; SDL compares it without the version when an
//! exact match fails, so a pad is found by its two ids. A line that also names a checksum of the
//! pad's name (in its GUID or a `crc:` field) fits, in SDL, only the pad whose name has that
//! checksum, and otherwise SDL falls back to a line without one; nv-rs does not work out SDL's
//! checksum of a pad's name, so it leaves those built-in lines out; a player's own line is taken
//! by the ids alone, checksum or not. Of the built-in 858, three such lines are left out (their
//! pads have no plain line, so they are not read), one whose GUID no DirectInput pad can have
//! (bytes after the vendor id that SDL always leaves 0), and four for devices with no face button
//! and no start (a foot controller, a throttle, a phone's half pad), and the ids SDL takes for
//! Xbox pads, which XInput reads: 764 pads. A line is used only if it fits the pad found
//! (`dinput_order::Layout::fits`: what it names exists and its sticks are not on pedals), so one
//! may still be refused on a given pad. The inputs
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

    /// Whether the line binds a face button or start: a device with none is no pad the game is
    /// played with.
    pub fn is_a_pad(&self) -> bool {
        self.bindings.iter().any(|&(_, output)| {
            matches!(
                output,
                Output::Button(
                    Button::South | Button::East | Button::West | Button::North | Button::Start
                )
            )
        })
    }

    /// The line as it fits a pad with `buttons` buttons, `axes` axes (`true` for each one on the
    /// Generic Desktop page: X, Y, Z, Rx, Ry, Rz, Wheel or a slider) and `hats` hats, as counted
    /// for the line's platform (review of #804). A binding of the sticks, the face buttons, start,
    /// the shoulders or the d-pad must name an input the pad has, and a stick must be on a Generic
    /// Desktop axis, never a pedal (a stick on a Simulation-page axis means the line's numbering is
    /// not this pad's): otherwise `Err` says why, for the log, and the line is not used. A binding
    /// of an optional control (back, the stick clicks, the triggers) naming an input the pad lacks
    /// is dropped alone, each with a note for the log. (The guide button, `misc1`-`misc6`, the
    /// paddles and the touch pad are never read: the game has no XInput control for them.)
    pub fn fitted(
        &self,
        buttons: usize,
        axes: &[bool],
        hats: usize,
    ) -> Result<(Mapping, Vec<String>), String> {
        let mut kept = Vec::with_capacity(self.bindings.len());
        let mut notes = Vec::new();
        for &(input, output) in &self.bindings {
            let missing = match input {
                Input::Button(n) if n >= buttons => {
                    Some(format!("button b{n} (the pad has {buttons})"))
                }
                Input::Hat { hat, .. } if hat >= hats => {
                    Some(format!("hat h{hat} (the pad has {hats})"))
                }
                Input::Axis { axis: n, .. } if n >= axes.len() => {
                    Some(format!("axis a{n} (the pad has {})", axes.len()))
                }
                _ => None,
            };
            let required = match output {
                Output::Axis { axis, .. } => {
                    matches!(
                        axis,
                        Axis::LeftX | Axis::LeftY | Axis::RightX | Axis::RightY
                    )
                }
                Output::Button(b) => {
                    !matches!(b, Button::Back | Button::LeftStick | Button::RightStick)
                }
            };
            if let Some(what) = missing {
                if required {
                    return Err(format!("it names {what} for {output:?}"));
                }
                notes.push(format!("its {output:?} names {what}, so it is left out"));
                continue;
            }
            if let (Input::Axis { axis: n, .. }, true, Output::Axis { axis, .. }) =
                (input, required, output)
            {
                if !axes[n] {
                    return Err(format!(
                        "its {axis:?} is a{n}, a pedal (Simulation page) on this pad"
                    ));
                }
            }
            kept.push((input, output));
        }
        Ok((
            Mapping {
                name: self.name.clone(),
                bindings: kept,
            },
            notes,
        ))
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
/// product ids, nothing else but the version, which a match leaves out) with no name checksum.
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
    (word(0) == 0x0003 && zero(2) && zero(6) && zero(10) && zero(14)).then(|| (word(4), word(8)))
}

/// [`key`] for a player's line: the pad's ids whatever name checksum the GUID carries.
pub fn key_by_ids(guid: &str) -> Option<(u16, u16)> {
    if guid.len() != 32 || !guid.is_ascii() {
        return None;
    }
    key(&format!("{}0000{}", &guid[..4], &guid[8..]))
}

/// The pads' lines by their ids, the later of two for one pad kept.
#[derive(Clone, Debug, Default)]
pub struct Database {
    lines: HashMap<(u16, u16), String>,
}

impl Database {
    /// The built-in Windows lines, less those for devices with no face button and no start (a
    /// foot controller, a throttle, a phone's half pad: review of #804), which are not pads the
    /// game can be played with, and less the ids SDL takes for Xbox pads
    /// ([`super::sdl_hints::XBOX`]): XInput reads those, so their lines are never reached (review
    /// of #804), and the database's size is the pads it can read.
    pub fn windows() -> Database {
        let mut db = Database::default();
        db.add(WINDOWS, Some("Windows"));
        db.lines.retain(|id, rest| {
            super::sdl_hints::XBOX.binary_search(id).is_err()
                && Mapping::parse(rest).is_some_and(|m| m.is_a_pad())
        });
        db
    }

    /// The lines of `text` (one a line, `#` comments and blank lines passed over); with a
    /// `platform`, only the lines whose `platform:` field names it (as SDL reads a mappings file),
    /// without one every line (as SDL reads the hint). Answers how many were taken.
    pub fn add(&mut self, text: &str, platform: Option<&str>) -> usize {
        self.add_lines(text, platform, false).taken
    }

    /// [`add`](Self::add) for the player's own lines, which are taken by the pad's ids alone,
    /// with or without a name checksum (SDL's tools write one now): the player wrote the line for
    /// their pad. Answers what was taken and the names of the lines another platform's.
    pub fn add_players(&mut self, text: &str, platform: Option<&str>) -> Added {
        self.add_lines(text, platform, true)
    }

    fn add_lines(&mut self, text: &str, platform: Option<&str>, players: bool) -> Added {
        let mut added = Added::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((guid, rest)) = line.split_once(',') else {
                continue;
            };
            let id = if players {
                key_by_ids(guid.trim())
            } else {
                key(guid.trim())
            };
            let Some(id) = id else {
                continue;
            };
            if let Some(platform) = platform {
                let named = rest
                    .split(',')
                    .filter_map(|f| f.trim().strip_prefix("platform:"))
                    .any(|p| p.eq_ignore_ascii_case(platform));
                if !named {
                    added
                        .other_platform
                        .push(rest.split(',').next().unwrap_or("").to_string());
                    continue;
                }
            }
            // A built-in line for one pad name only (its checksum): left out, see the notes.
            if !players && rest.split(',').any(|f| f.trim().starts_with("crc:")) {
                continue;
            }
            self.lines.insert(id, rest.to_string());
            added.taken += 1;
        }
        added
    }

    /// The mapping of the pad `vendor`:`product`, if it has a line.
    pub fn mapping(&self, vendor: u16, product: u16) -> Option<Mapping> {
        Mapping::parse(self.lines.get(&(vendor, product))?)
    }

    /// The built-in lines with the player's read over them: `gamecontrollerdb.txt` in `folder`
    /// (the port's own folder; its Windows lines; UTF-8, with or without its mark, or UTF-16),
    /// then `hint`'s lines (`SDL_GAMECONTROLLERCONFIG`'s, all of them). Answers the lines for the
    /// log too: how many each gave, and each line of the file left out for another platform.
    pub fn with_players_lines(
        folder: Option<&std::path::Path>,
        hint: Option<&str>,
    ) -> (Database, Vec<String>) {
        let mut db = Database::windows();
        let mut log = Vec::new();
        if let Some(path) = folder.map(|f| f.join(FILE)) {
            if let Ok(bytes) = std::fs::read(&path) {
                let added = db.add_players(&text_of(&bytes), Some("Windows"));
                log.push(format!(
                    "nv-rs: game pads: {} lines taken from {}",
                    added.taken,
                    path.display()
                ));
                for name in added.other_platform {
                    log.push(format!(
                        "nv-rs: game pads: the line for {name} in {} has no platform:Windows and is \
                         not taken",
                        path.display()
                    ));
                }
            }
        }
        if let Some(hint) = hint {
            let added = db.add_players(hint, None);
            log.push(format!(
                "nv-rs: game pads: {} lines taken from SDL_GAMECONTROLLERCONFIG",
                added.taken
            ));
        }
        (db, log)
    }

    /// How many pads have a line.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// What [`Database::add_players`] took: how many lines, and the names of those for another
/// platform.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Added {
    pub taken: usize,
    pub other_platform: Vec<String>,
}

/// A text file's text: UTF-16 (little or big endian) after its byte order mark, else UTF-8 with
/// or without its mark (a byte that is not UTF-8 read as the replacement character).
pub fn text_of(bytes: &[u8]) -> String {
    let utf16 = |rest: &[u8], word: fn([u8; 2]) -> u16| {
        let words: Vec<u16> = rest.chunks_exact(2).map(|c| word([c[0], c[1]])).collect();
        String::from_utf16_lossy(&words)
    };
    match bytes {
        [0xff, 0xfe, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xfe, 0xff, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xef, 0xbb, 0xbf, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
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

    /// The built-in lines: the copy's Windows lines, found by their two ids: 764 pads of its 858
    /// GUID lines (three name a name checksum too, one has a GUID no pad can have, four bind no face
    /// button or start, and the Xbox-type ids XInput reads are left out; the `xinput` line names no pad).
    #[test]
    fn the_built_in_lines() {
        let db = Database::windows();
        assert_eq!(db.len(), 764);
        let guid_lines = WINDOWS.lines().filter(|l| l.starts_with("0300")).count();
        assert_eq!(guid_lines, 858);
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
        // The version is left out of a match; a name checksum fits one pad name only.
        assert_eq!(
            key("03000000c82d00000660000001000000"),
            Some((0x2dc8, 0x6006))
        );
        assert_eq!(key("0300abcdc82d00000660000000000000"), None);
        let mut db = Database::default();
        let crc = "03000000c82d00000660000000000000,Named,a:b0,crc:abcd,platform:Windows,";
        assert_eq!(db.add(crc, None), 0);
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
        // Each side onto the other's length, rounded: the small values and the middle of each
        // side, where the two sides' scales differ.
        assert_eq!((up_positive(-1), up_positive(1)), (1, -1));
        assert_eq!((up_positive(-3), up_positive(3)), (3, -3));
        assert_eq!(up_positive(-16385), 16384);
        assert_eq!(up_positive(16383), -16383);
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
        // UTF-16 with its mark, as some editors save; one line with a name checksum (SDL's tools
        // write them), one for another platform.
        let text = "03000000c82d00000660000000000000,File,a:b0,platform:Windows,\n\
                    0300abcdaaaa0000bbbb000000000000,New,a:b0,crc:abcd,platform:Windows,\n\
                    03000000cccc0000dddd000000000000,Mac,a:b0,platform:Mac OS X,\n";
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        std::fs::write(dir.join(FILE), bytes).unwrap();
        let (db, log) = Database::with_players_lines(Some(&dir), None);
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "File");
        assert_eq!(db.mapping(0xaaaa, 0xbbbb).unwrap().name, "New");
        assert_eq!(db.mapping(0xcccc, 0xdddd), None);
        assert_eq!(db.len(), 765);
        assert_eq!(log.len(), 2);
        assert!(log[0].starts_with("nv-rs: game pads: 2 lines taken from "));
        assert!(log[1].contains("the line for Mac") && log[1].contains("no platform:Windows"));
        let hint = "03000000c82d00000660000000000000,Hint,a:b0";
        let (db, log) = Database::with_players_lines(Some(&dir), Some(hint));
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "Hint");
        assert_eq!(
            log.last().unwrap(),
            "nv-rs: game pads: 1 lines taken from SDL_GAMECONTROLLERCONFIG"
        );
        std::fs::remove_dir_all(&dir).unwrap();
        let (db, log) = Database::with_players_lines(Some(&dir), None);
        assert_eq!(db.mapping(0x2dc8, 0x6006).unwrap().name, "8BitDo Pro 2");
        assert!(log.is_empty(), "no file: nothing to say");
        assert_eq!(Database::with_players_lines(None, None).0.len(), 764);
    }

    /// A file's text whatever its encoding mark.
    #[test]
    fn text_in_its_encoding() {
        assert_eq!(text_of(b"\xef\xbb\xbfab"), "ab");
        assert_eq!(text_of(b"ab"), "ab");
        assert_eq!(text_of(&[0xfe, 0xff, 0, b'a', 0, b'b']), "ab");
        assert_eq!(text_of(&[0xff, 0xfe, b'a', 0, b'b', 0]), "ab");
        assert_eq!(
            key_by_ids("0300abcdc82d00000660000000000000"),
            Some((0x2dc8, 0x6006))
        );
        assert_eq!(key_by_ids("short"), None);
    }

    /// Built-in lines for devices with no face button and no start are left out (review of #804):
    /// the 3dRudder, the TWCS Throttle, the iGUGU Gamecore, the Xiaomi Black Shark (L).
    #[test]
    fn no_pad_without_face_buttons() {
        let db = Database::windows();
        for id in [
            (0x2dfa, 0x0001),
            (0x1d51, 0x3002),
            (0x044f, 0xb687),
            (0xffe0, 0x0102),
        ] {
            assert_eq!(db.mapping(id.0, id.1), None, "{id:04x?}");
        }
        assert!(!Mapping::parse("Foot,leftx:a0,lefty:a1").unwrap().is_a_pad());
        assert!(Mapping::parse("Pad,start:b9").unwrap().is_a_pad());
    }

    /// Lines are judged by the pad found (`dinput_order::Layout::fits`), not from other systems'
    /// lines (review of #804, round 3): a pad whose Windows line has the right stick on a2/a5, such
    /// as the Hori Fighting Commander 4 PS4 (0f0d:005e) or the Moga Pro (20d6:6271), keeps its
    /// built-in line. An Xbox-type id such as the NVIDIA Shield (0955:7210) has none: XInput reads
    /// it, so no database line is ever used for it.
    #[test]
    fn built_in_lines_are_judged_by_the_pad_and_xbox_ids_have_none() {
        use super::super::sdl_hints::is_xinput_device;
        let db = Database::windows();
        for (id, name) in [((0x0f0d, 0x005e), "Hori"), ((0x20d6, 0x6271), "Moga Pro")] {
            let m = db.mapping(id.0, id.1).expect("its built-in line");
            assert!(m.name.contains(name), "{}", m.name);
            assert!(!is_xinput_device(id, r"\?\hid#vid_0000", false));
        }
        let shield = (0x0955, 0x7210);
        let mut copy = Database::default();
        copy.add(WINDOWS, Some("Windows"));
        assert!(copy.mapping(shield.0, shield.1).is_some());
        assert!(is_xinput_device(shield, "", false));
        assert_eq!(db.mapping(shield.0, shield.1), None);
    }

    /// A line fits a pad only if every required input it names is there, counted from 0, and its
    /// sticks are on Generic Desktop axes (review of #804); a trigger may be on a pedal.
    #[test]
    fn a_line_fits_at_the_edges() {
        let m = Mapping::parse("Edge,a:b8,dpup:h1.1,leftx:a1,lefttrigger:a2").unwrap();
        let (fitted, notes) = m.fitted(9, &[true, true, false], 2).unwrap();
        assert_eq!((fitted, notes.len()), (m.clone(), 0));
        assert!(m
            .fitted(8, &[true, true, false], 2)
            .unwrap_err()
            .contains("b8"));
        assert!(m
            .fitted(9, &[true, true, false], 1)
            .unwrap_err()
            .contains("h1"));
        assert!(m
            .fitted(9, &[true, false, true], 2)
            .unwrap_err()
            .contains("pedal"));
    }

    /// An optional control (back, a stick click, a trigger) that names an input the pad lacks is
    /// left out alone, with a note for the log, and the rest of the line is used (review of #804,
    /// round 3); the guide button and the paddles are never read at all.
    #[test]
    fn an_optional_binding_past_the_pad_is_left_out_alone() {
        let line = "Pad,a:b0,b:b1,x:b2,y:b3,start:b4,back:b12,rightstick:b13,lefttrigger:a5,\
                    leftx:a0,lefty:a1,guide:b20,paddle1:b21";
        let m = Mapping::parse(line).unwrap();
        let (fitted, notes) = m.fitted(6, &[true, true], 0).unwrap();
        assert_eq!(notes.len(), 3, "{notes:?}");
        assert!(notes[0].contains("Back") && notes[0].contains("b12"));
        assert!(notes[2].contains("LeftTrigger") && notes[2].contains("a5"));
        let all = Mapping::parse("Pad,a:b0,b:b1,x:b2,y:b3,start:b4,leftx:a0,lefty:a1").unwrap();
        assert_eq!(fitted.bindings, all.bindings);
        // A required one past the pad still refuses the line.
        assert!(m.fitted(4, &[true, true], 0).unwrap_err().contains("b4"));
    }
}
