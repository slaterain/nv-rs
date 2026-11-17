//! The machine table off Windows (PR 5 of the Linux and macOS track, decision 2): what answers
//! the game's questions about the machine where Windows is not there to ask. On Windows the table
//! (`d3d9_adapter.tsv`, read by [`super::d3d9::Adapter::parse`]) is made by `scripts/d3d9-capture`
//! and `scripts/nvapi-capture` with Windows' own functions; on Linux and macOS the host makes the
//! same lines at every start from:
//!
//! - **the system** (read only, never written): the time zone from the zone file
//!   (`/etc/localtime`, or the `TZ` variable as the C library takes it), the machine's memory
//!   (`/proc/meminfo` on Linux; `sysctl` on macOS, `super::process`), the processor (the real
//!   `CPUID` on x86-64);
//! - **nv-rs's own folder** for the player's Documents and AppData, on drive `Z:` as the game
//!   sees this system's folders (decision 3);
//! - **Windows' fixed answers**, constants from one capture on Windows 11
//!   (`data/machine/windows-fixed.tsv`): only values that are the same on every Windows 10/11 PC
//!   (window styles and borders of a program without a DPI-awareness manifest, a local session's
//!   `SM_REMOTESESSION`, the Windows, ProgramData and Public Documents folders), and the facts of
//!   the code pages named below (`IsValidCodePage`, `GetCPInfo`); nothing personal or per machine;
//! - **declared choices**: the ANSI code page 1252 and the OEM code page 437 (the pair of an
//!   English Windows; on Windows they follow the PC's language setting); no NVIDIA library (what a
//!   PC without NVIDIA's driver answers); and on a processor without `CPUID` (Apple silicon) the
//!   answer of an x86-64 PC (`engine_core::processor`).
//!
//! - **the display and the graphics card** ([`display`]): the monitors from winit, the card from
//!   wgpu, and the Direct3D 9 answers of a modern card captured once on Windows, with the
//!   multisample counts and float blending the card there draws with. They are a part of their
//!   own ([`this_display`], [`display_part`]), which the host makes in its start after the run's
//!   renderer choice (the folders' part is read earlier, by the settings' reading, and must not
//!   fix that choice). They are asked only when the run draws; a run that draws nothing has
//!   none, and a question about them stops it there, saying it was not captured. A card without
//!   DXT textures refuses the start ([`Card::refusal`]).
//!
//! **The time zone.** Windows answers `TIME_ZONE_INFORMATION`: the standard offset (`Bias`,
//! minutes, UTC = local + bias), and two change-over dates in its "the n-th (5: last) weekday of
//! a month at a time of day" form. A zone file (TZif) lists the zone's change-overs up to some
//! year and then, on its last line, a POSIX rule for the times after them. When the file's list
//! still covers now (zones whose dates move with the moon, such as Gaza's and Morocco's, are
//! listed decades ahead), the current year's change-overs from the list are used ([`Zone`]: the
//! daylight start and end nearest now, with the offsets they change to). Otherwise the POSIX rule
//! (`EST5EDT,M3.2.0,M11.1.0`) has the same parts; its `Mm.w.d` dates at a time within the day
//! are Windows' form as they are. A rule Windows' form cannot hold (a Julian day `Jn` or `n`, or a
//! time outside 0..24 h such as Israel's `M3.4.4/26`) is worked out for the current year to its
//! real change-over moment and written in Windows' form for this year (the month, the weekday of
//! the real local date, which such weekday of the month it is, 5 when it is the last, the hour
//! and minute), as Windows does for such zones. The table is made at every start, so the year is
//! always the current one. The names are the zone's own abbreviations (`EST`, `EDT`): the game
//! reads them only into the C runtime's `_tzname`, which nothing in the exe prints (no `strftime`,
//! no `%Z`; `__tzname` is called only by `__tzset_nolock`). Windows' per-year rules
//! (`GetTimeZoneInformationForYear`, which the save list's file times use) are the current rule
//! for every year: past changes of a zone's rules are not kept (Windows keeps some). A zone whose
//! daylight time is "negative" in the zone file (Dublin: standard IST, +1 h, in summer; GMT as its
//! daylight time in winter) answers that way: `DaylightBias` +60 and daylight time in winter,
//! where Windows' entry says the reverse; the offset at every moment is the same. (Linux's zone
//! files have that form; macOS's have Windows' own, GMT standard and IST daylight, and answer
//! so.) The game itself
//! is never shown the `TZ` variable (`super::process::shown_to_the_game`); [`this_zone`] reads it.
// The pure parts run in the tests on every system; only Linux and macOS use them in a run.
#![cfg_attr(windows, allow(dead_code))]

use super::zone::{self, ZoneInfo};

pub mod display;
pub use display::{Card, Mode, Monitor};

/// Windows' fixed answers (see the module's documentation).
pub const WINDOWS_FIXED: &str = include_str!("../data/machine/windows-fixed.tsv");

/// `ERROR_MOD_NOT_FOUND`: what `LoadLibraryA("nvapi.dll")` leaves on a PC without NVIDIA's driver.
const ERROR_MOD_NOT_FOUND: u32 = 126;

/// The first and last year `GetTimeZoneInformationForYear` takes (1601..=30827).
const FIRST_YEAR: u32 = 1601;
const LAST_YEAR: u32 = 30827;

/// The day a POSIX rule names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Day {
    /// `Mm.w.d`: the `week`-th (5: the last) `weekday` (0 Sunday) of `month`.
    MonthWeek { month: u32, week: u32, weekday: u32 },
    /// `Jn`: day 1..=365 of the year, February 29 never counted.
    Julian(u32),
    /// `n`: day 0..=365 of the year, February 29 counted.
    Zero(u32),
}

/// A change-over: the day and the local time of day in seconds (the time then in effect; POSIX
/// allows -167..=167 hours).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub day: Day,
    pub time: i32,
}

/// Daylight time's part of a POSIX rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Daylight {
    pub name: String,
    /// Seconds to add to local daylight time for UTC.
    pub offset: i32,
    pub start: Rule,
    pub end: Rule,
}

/// A POSIX time-zone rule (`TZ`, a zone file's last line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosixZone {
    pub name: String,
    /// Seconds to add to local standard time for UTC (west of Greenwich positive, as `Bias`).
    pub offset: i32,
    pub daylight: Option<Daylight>,
}

/// Parses a POSIX time-zone rule: `std offset [dst [offset] ,start[/time] ,end[/time]]`.
pub fn parse_posix_tz(tz: &str) -> Result<PosixZone, String> {
    let mut p = Parser {
        s: tz.as_bytes(),
        at: 0,
    };
    let fail = |why: String| format!("the time-zone rule {tz:?}: {why}");
    let name = p.name().map_err(fail)?;
    let offset = p
        .offset(24)
        .map_err(fail)?
        .ok_or_else(|| fail("no standard offset".into()))?;
    if p.done() {
        return Ok(PosixZone {
            name,
            offset,
            daylight: None,
        });
    }
    let daylight_name = p.name().map_err(fail)?;
    let daylight_offset = p.offset(24).map_err(fail)?.unwrap_or(offset - 3600);
    if p.done() {
        return Err(fail("daylight time without its rules: not provided".into()));
    }
    p.expect(b',').map_err(fail)?;
    let start = p.rule().map_err(fail)?;
    p.expect(b',').map_err(fail)?;
    let end = p.rule().map_err(fail)?;
    if !p.done() {
        return Err(fail(format!("more after the rules at {}", p.at)));
    }
    Ok(PosixZone {
        name,
        offset,
        daylight: Some(Daylight {
            name: daylight_name,
            offset: daylight_offset,
            start,
            end,
        }),
    })
}

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn done(&self) -> bool {
        self.at >= self.s.len()
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("{:?} expected at {}", c as char, self.at))
        }
    }
    /// `EST`, or `<+0545>` (the name without its brackets): at least 3 characters.
    fn name(&mut self) -> Result<String, String> {
        let start = self.at;
        let text = if self.peek() == Some(b'<') {
            self.at += 1;
            while self
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'+' || c == b'-')
            {
                self.at += 1;
            }
            let inner = &self.s[start + 1..self.at];
            self.expect(b'>')?;
            inner
        } else {
            while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                self.at += 1;
            }
            &self.s[start..self.at]
        };
        if text.len() < 3 {
            return Err(format!("a zone name of fewer than 3 characters at {start}"));
        }
        Ok(String::from_utf8_lossy(text).into_owned())
    }
    /// `[+-]hh[:mm[:ss]]` in seconds (hours up to `max_hours`), `None` when none follows.
    fn offset(&mut self, max_hours: i32) -> Result<Option<i32>, String> {
        let sign = match self.peek() {
            Some(b'-') => {
                self.at += 1;
                -1
            }
            Some(b'+') => {
                self.at += 1;
                1
            }
            Some(c) if c.is_ascii_digit() => 1,
            _ => return Ok(None),
        };
        Ok(Some(sign * self.hms(max_hours)?))
    }
    fn number(&mut self) -> Result<i32, String> {
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) && self.at - start < 4 {
            self.at += 1;
        }
        std::str::from_utf8(&self.s[start..self.at])
            .ok()
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| format!("a number expected at {start}"))
    }
    fn hms(&mut self, max_hours: i32) -> Result<i32, String> {
        let start = self.at;
        let hours = self.number()?;
        let mut minutes = 0;
        let mut seconds = 0;
        if self.peek() == Some(b':') {
            self.at += 1;
            minutes = self.number()?;
            if self.peek() == Some(b':') {
                self.at += 1;
                seconds = self.number()?;
            }
        }
        if hours > max_hours || minutes > 59 || seconds > 59 {
            return Err(format!("a time out of range at {start}"));
        }
        Ok(hours * 3600 + minutes * 60 + seconds)
    }
    /// `Mm.w.d`, `Jn` or `n`, then `[/time]` (default 02:00).
    fn rule(&mut self) -> Result<Rule, String> {
        let start = self.at;
        let day = match self.peek() {
            Some(b'M') => {
                self.at += 1;
                let month = self.number()? as u32;
                self.expect(b'.')?;
                let week = self.number()? as u32;
                self.expect(b'.')?;
                let weekday = self.number()? as u32;
                if !(1..=12).contains(&month) || !(1..=5).contains(&week) || weekday > 6 {
                    return Err(format!("the rule at {start} is out of range"));
                }
                Day::MonthWeek {
                    month,
                    week,
                    weekday,
                }
            }
            Some(b'J') => {
                self.at += 1;
                let n = self.number()? as u32;
                if !(1..=365).contains(&n) {
                    return Err(format!("the rule at {start} is out of range"));
                }
                Day::Julian(n)
            }
            _ => {
                let n = self.number()? as u32;
                if n > 365 {
                    return Err(format!("the rule at {start} is out of range"));
                }
                Day::Zero(n)
            }
        };
        let time = if self.peek() == Some(b'/') {
            self.at += 1;
            self.offset(167)?
                .ok_or_else(|| format!("a time expected at {}", self.at))?
        } else {
            7200
        };
        Ok(Rule { day, time })
    }
}

fn days_in_month(year: i64, month: u32) -> i64 {
    let next = if month == 12 {
        zone::days_from_civil(year + 1, 1, 1)
    } else {
        zone::days_from_civil(year, month + 1, 1)
    };
    next - zone::days_from_civil(year, month, 1)
}

/// A change-over as Windows' `SYSTEMTIME` words in its day-in-month form (`wYear` 0, `wMonth`,
/// `wDayOfWeek`, `wDay` = which such weekday 1..=5 with 5 the last, hour, minute, second, 0).
/// An `Mm.w.d` rule at a time within the day is that form as it is; any other is the real
/// change-over in `year` written in the form (see the module's documentation).
pub fn windows_date(rule: Rule, year: i64) -> Result<[u16; 8], String> {
    if let Day::MonthWeek {
        month,
        week,
        weekday,
    } = rule.day
    {
        if (0..86_400).contains(&rule.time) {
            let t = rule.time as u32;
            return Ok([
                0,
                month as u16,
                weekday as u16,
                week as u16,
                (t / 3600) as u16,
                (t / 60 % 60) as u16,
                (t % 60) as u16,
                0,
            ]);
        }
    }
    let first = zone::days_from_civil(year, 1, 1);
    let leap = days_in_month(year, 2) == 29;
    let day = match rule.day {
        Day::MonthWeek {
            month,
            week,
            weekday,
        } => {
            let words = [0, month as u16, weekday as u16, week as u16, 0, 0, 0, 0];
            zone::transition_seconds(year, &words).div_euclid(86_400)
        }
        Day::Julian(n) => first + i64::from(n) - 1 + i64::from(leap && n >= 60),
        Day::Zero(n) => first + i64::from(n),
    };
    windows_form(day * 86_400 + i64::from(rule.time), year)
}

/// The real local moment `at` (seconds since 1970 as if UTC) in Windows' day-in-month form, which
/// gives that moment in `year` (an error when it falls in another year).
fn windows_form(at: i64, year: i64) -> Result<[u16; 8], String> {
    let (days, seconds) = (at.div_euclid(86_400), at.rem_euclid(86_400));
    let (y, month, date) = zone::civil_from_days(days);
    if y != year {
        return Err(format!(
            "a change-over falls in {y}, not in {year}: no Windows form"
        ));
    }
    let week = if i64::from(date) + 7 > days_in_month(y, month) {
        5
    } else {
        (date - 1) / 7 + 1
    };
    Ok([
        0,
        month as u16,
        zone::weekday(days) as u16,
        week as u16,
        (seconds / 3600) as u16,
        (seconds / 60 % 60) as u16,
        (seconds % 60) as u16,
        0,
    ])
}

fn put_i32(out: &mut [u8], at: usize, v: i32) {
    out[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// A name in a 32-character UTF-16 field: at most 31 characters and the zero after them.
fn put_name(out: &mut [u8], at: usize, name: &str) {
    for (i, u) in name.encode_utf16().take(31).enumerate() {
        out[at + 2 * i..at + 2 * i + 2].copy_from_slice(&u.to_le_bytes());
    }
}

fn put_date(out: &mut [u8], at: usize, words: [u16; 8]) {
    for (i, w) in words.iter().enumerate() {
        out[at + 2 * i..at + 2 * i + 2].copy_from_slice(&w.to_le_bytes());
    }
}

/// Seconds as whole minutes, the unit of Windows' biases.
fn minutes(seconds: i32) -> Result<i32, String> {
    if seconds % 60 == 0 {
        Ok(seconds / 60)
    } else {
        Err(format!(
            "an offset of {seconds} s is not whole minutes: no Windows form"
        ))
    }
}

/// `TIME_ZONE_INFORMATION`'s 172 bytes for `zone` in `year`: `Bias` +0, `StandardName` +4,
/// `StandardDate` +0x44 (the end of daylight time), `StandardBias` +0x54 (0), `DaylightName`
/// +0x58, `DaylightDate` +0x98 (its start), `DaylightBias` +0xA8 (daylight less standard offset).
/// A zone without daylight time has no dates, `DaylightBias` 0 and its standard name as both
/// names (nothing reads them without a daylight date: the C runtime's `__tzset_nolock` and the
/// platform's time calls look at the daylight date's month first).
pub fn time_zone_information(zone: &PosixZone, year: i64) -> Result<[u8; 172], String> {
    let mut out = [0u8; 172];
    put_i32(&mut out, 0, minutes(zone.offset)?);
    put_name(&mut out, 4, &zone.name);
    match &zone.daylight {
        None => put_name(&mut out, 0x58, &zone.name),
        Some(daylight) => {
            put_date(&mut out, 0x44, windows_date(daylight.end, year)?);
            put_name(&mut out, 0x58, &daylight.name);
            put_date(&mut out, 0x98, windows_date(daylight.start, year)?);
            put_i32(&mut out, 0xa8, minutes(daylight.offset - zone.offset)?);
        }
    }
    Ok(out)
}

/// The POSIX rule of a TZif zone file (version 2 and later): its last line.
pub fn tzif_footer(bytes: &[u8]) -> Result<&str, String> {
    if !bytes.starts_with(b"TZif") {
        return Err("not a zone file (TZif)".into());
    }
    if !matches!(bytes.get(4), Some(b'2'..=b'9')) {
        return Err("a zone file of version 1 has no rule line".into());
    }
    let body = bytes
        .strip_suffix(b"\n")
        .ok_or("the zone file does not end with its rule line")?;
    let start = body
        .iter()
        .rposition(|&b| b == b'\n')
        .ok_or("the zone file has no rule line")?
        + 1;
    let line =
        std::str::from_utf8(&body[start..]).map_err(|_| "the zone file's rule line is not text")?;
    if line.is_empty() {
        return Err("the zone file's rule line is empty (no rule after its table)".into());
    }
    Ok(line)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The year of local standard time at `utc` (seconds since 1970).
fn local_year(zone: &PosixZone, utc: i64) -> i64 {
    zone::civil_from_days((utc - i64::from(zone.offset)).div_euclid(86_400)).0
}

/// One local time type of a zone file: its offset east of UTC in seconds, whether it is daylight
/// time, and its abbreviation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    pub offset_east: i32,
    pub daylight: bool,
    pub name: String,
}

/// A time zone: its POSIX rule and, from a zone file, its list of change-overs (UTC seconds, the
/// index of the [`Kind`] it changes to) with the kinds (before the first change-over, the first
/// kind is in force).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub rule: PosixZone,
    pub changes: Vec<(i64, usize)>,
    pub kinds: Vec<Kind>,
}

impl Zone {
    /// A zone given by a POSIX rule alone (the `TZ` variable's form).
    pub fn from_rule(rule: PosixZone) -> Zone {
        Zone {
            rule,
            changes: Vec::new(),
            kinds: Vec::new(),
        }
    }
}

const CUT_SHORT: &str = "the zone file is cut short";

fn be32(bytes: &[u8], at: usize) -> Result<usize, String> {
    let b = bytes.get(at..at + 4).ok_or(CUT_SHORT)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

/// A TZif zone file (version 2 and later, RFC 8536): the rule line, and the 64-bit list of
/// change-overs with their local time types.
pub fn parse_tzif(bytes: &[u8]) -> Result<Zone, String> {
    let rule = parse_posix_tz(tzif_footer(bytes)?)?;
    // The counts after each header's 20 bytes: UT/local and standard/wall indicators, leap
    // seconds, change-overs, local time types, abbreviation characters.
    let counts = |at: usize| -> Result<[usize; 6], String> {
        let mut c = [0usize; 6];
        for (i, v) in c.iter_mut().enumerate() {
            *v = be32(bytes, at + 20 + 4 * i)?;
        }
        Ok(c)
    };
    let [ut, standard, leap, times, kinds, chars] = counts(0)?;
    // The version 1 block (32-bit times) is skipped: its size from its own counts.
    let second = 44 + times * 5 + kinds * 6 + chars + leap * 8 + standard + ut;
    if bytes.get(second..second + 4) != Some(b"TZif".as_slice()) {
        return Err("the zone file's second header is missing".into());
    }
    let [_, _, _, times, kinds, chars] = counts(second)?;
    if kinds == 0 {
        return Err("the zone file has no local time types".into());
    }
    let start = second + 44;
    let data = bytes
        .get(start..start + times * 9 + kinds * 6 + chars)
        .ok_or(CUT_SHORT)?;
    let (moments, rest) = data.split_at(times * 8);
    let (indices, rest) = rest.split_at(times);
    let (types, names) = rest.split_at(kinds * 6);
    let kinds: Vec<Kind> = types
        .chunks(6)
        .map(|t| {
            let at = usize::from(t[5]);
            let name = names
                .get(at..)
                .ok_or("an abbreviation outside the zone file's list")?;
            let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            Ok(Kind {
                offset_east: i32::from_be_bytes([t[0], t[1], t[2], t[3]]),
                daylight: t[4] != 0,
                name: String::from_utf8_lossy(&name[..end]).into_owned(),
            })
        })
        .collect::<Result<_, String>>()?;
    let changes = moments
        .chunks(8)
        .zip(indices)
        .map(|(m, &k)| {
            let at = i64::from_be_bytes([m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7]]);
            if usize::from(k) < kinds.len() {
                Ok((at, usize::from(k)))
            } else {
                Err("a change-over to a local time type the zone file does not have".to_string())
            }
        })
        .collect::<Result<_, String>>()?;
    Ok(Zone {
        rule,
        changes,
        kinds,
    })
}

/// The answer for the year of `utc` from the zone file's list of change-overs, when the list
/// still covers `utc` (`None` past its end: the rule line answers then). Daylight time's start
/// and end are those of this year nearest `utc` (the two around it), each at its local time in
/// the time then in effect, with the offsets they change to; a year without either gives the
/// kind in force with no dates; a year where daylight time only starts or only ends (the zone
/// changed for good) is left to the rule line too (`None`).
fn from_changes(zone: &Zone, utc: i64) -> Result<Option<[u8; 172]>, String> {
    match zone.changes.last() {
        Some(&(last, _)) if utc < last => {}
        _ => return Ok(None),
    }
    let before = |i: usize| if i == 0 { 0 } else { zone.changes[i - 1].1 };
    let now = zone
        .changes
        .iter()
        .rposition(|&(t, _)| t <= utc)
        .map_or(0, |i| zone.changes[i].1);
    let now = &zone.kinds[now];
    let local_year = |at: i64| zone::civil_from_days(at.div_euclid(86_400)).0;
    let year = local_year(utc + i64::from(now.offset_east));
    // (UTC moment, local moment in the time before, the kind after) of this year's starts and
    // ends of daylight time.
    let (mut starts, mut ends) = (Vec::new(), Vec::new());
    for (i, &(t, k)) in zone.changes.iter().enumerate() {
        let (old, new) = (&zone.kinds[before(i)], &zone.kinds[k]);
        let local = t + i64::from(old.offset_east);
        if old.daylight == new.daylight || local_year(local) != year {
            continue;
        }
        if new.daylight {
            starts.push((t, local, k));
        } else {
            ends.push((t, local, k));
        }
    }
    let nearest = |v: &[(i64, i64, usize)]| v.iter().copied().min_by_key(|c| (c.0 - utc).abs());
    let mut out = [0u8; 172];
    match (nearest(&starts), nearest(&ends)) {
        (None, None) if !now.daylight => {
            put_i32(&mut out, 0, minutes(-now.offset_east)?);
            put_name(&mut out, 4, &now.name);
            put_name(&mut out, 0x58, &now.name);
        }
        (Some(start), Some(end)) => {
            let (standard, daylight) = (&zone.kinds[end.2], &zone.kinds[start.2]);
            put_i32(&mut out, 0, minutes(-standard.offset_east)?);
            put_name(&mut out, 4, &standard.name);
            put_date(&mut out, 0x44, windows_form(end.1, year)?);
            put_name(&mut out, 0x58, &daylight.name);
            put_date(&mut out, 0x98, windows_form(start.1, year)?);
            put_i32(
                &mut out,
                0xa8,
                minutes(standard.offset_east - daylight.offset_east)?,
            );
        }
        _ => return Ok(None),
    }
    Ok(Some(out))
}

/// `TIME_ZONE_INFORMATION`'s bytes for `zone` at `utc`: from the zone file's change-overs while
/// they cover `utc` ([`from_changes`]), else from its rule for the current year.
pub fn zone_information_at(zone: &Zone, utc: i64) -> Result<[u8; 172], String> {
    match from_changes(zone, utc)? {
        Some(bytes) => Ok(bytes),
        None => time_zone_information(&zone.rule, local_year(&zone.rule, utc)),
    }
}

/// The table's time-zone lines for `zone` at `utc` (seconds since 1970): `GetTimeZoneInformation`
/// (its answer: 0 without daylight time, else 1 standard or 2 daylight at `utc`, as
/// `super::time` works it out) and one `GetTimeZoneInformationForYear` line for every year, both
/// with the bytes for the current year.
pub fn zone_lines(zone: &Zone, utc: i64) -> Result<String, String> {
    let bytes = zone_information_at(zone, utc)?;
    let answer = match zone::in_daylight(&ZoneInfo::from_bytes(&bytes), utc) {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    };
    let hex = hex(&bytes);
    Ok(format!(
        "GetTimeZoneInformation\t\t{answer:x}\t{hex}\n\
         GetTimeZoneInformationForYear\t{FIRST_YEAR:x} {LAST_YEAR:x}\t1\t{hex}\n"
    ))
}

/// The processor's answers for `__get_sse2_info` (`CpuIdFlagTest`, `Cpuid` leaves 0 and 1).
pub fn processor_lines(id_flag_test: u32, leaf_0: [u32; 4], leaf_1: [u32; 4]) -> String {
    let leaf = |n: u32, r: [u32; 4]| {
        format!(
            "Cpuid\t{n:x}\t{:x} {:x} {:x} {:x}\n",
            r[0], r[1], r[2], r[3]
        )
    };
    format!(
        "CpuIdFlagTest\t\t{id_flag_test:x}\n{}{}",
        leaf(0, leaf_0),
        leaf(1, leaf_1)
    )
}

/// The fixed processor answer of a machine without `CPUID` (`engine_core::processor`).
pub fn fixed_processor() -> (u32, [u32; 4], [u32; 4]) {
    let leaf = |n| engine_core::processor::fixed_cpuid(n).expect("a leaf the game asks");
    (engine_core::processor::FIXED_ID_FLAG_TEST, leaf(0), leaf(1))
}

/// No `nvapi.dll`: `LoadLibraryA` fails with `ERROR_MOD_NOT_FOUND`, as on a PC without NVIDIA's
/// driver (the `scripts/nvapi-capture` lines of such a PC).
pub fn no_nvapi_lines() -> String {
    format!("NvApiLoad\t\t0\t{ERROR_MOD_NOT_FOUND:x}\nNvApiEnd\n")
}

/// The longest name the game makes under Documents (a save, up to 60 characters) must fit the
/// 259 characters of its paths (the launcher's rule on Windows).
fn documents_too_long(documents: &str) -> bool {
    documents.len() + "\\My Games\\FalloutNV\\Saves\\".len() + 60 >= 260
}

/// The game's Documents, roaming and local AppData folders inside nv-rs's own folder `own` (a
/// full path of this system), as the Windows launcher lays its profile out (`game\Documents`,
/// `game\AppData\Roaming`, `game\AppData\Local`): this system's paths.
pub fn game_folders(own: &std::path::Path) -> [std::path::PathBuf; 3] {
    let game = own.join("game");
    [
        game.join("Documents"),
        game.join("AppData").join("Roaming"),
        game.join("AppData").join("Local"),
    ]
}

/// The game's name of a full path of this system: on drive `Z:`, each `/` a `\` (decision 3;
/// `engine_core::exe::game_view` once the paths work, #633, is merged).
fn on_drive_z(path: &std::path::Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or_else(|| format!("{}: not UTF-8, which the game cannot name", path.display()))?;
    if !text.starts_with('/') {
        return Err(format!("{text}: not a full path"));
    }
    Ok(format!("Z:{}", text.replace('/', "\\")))
}

/// The `SHGetFolderPathA` lines for the Documents (5), roaming (0x1a) and local (0x1c) AppData
/// folders inside nv-rs's own folder `own` ([`game_folders`]), as the game names them on `Z:`.
pub fn folder_lines(own: &std::path::Path) -> Result<String, String> {
    let mut lines = String::new();
    for (csidl, folder) in [5u32, 0x1a, 0x1c].into_iter().zip(game_folders(own)) {
        let game = on_drive_z(&folder)?;
        if csidl == 5 && documents_too_long(&game) {
            return Err(format!(
                "{game}: too long for the game's save names (paths of up to 259 characters)"
            ));
        }
        lines.push_str(&format!(
            "SHGetFolderPathA\t{csidl:x} 0\t0\t{}\n",
            hex(game.as_bytes())
        ));
    }
    Ok(lines)
}

/// The inputs of a table: each part read from the system, or why it could not be.
pub struct Inputs {
    pub zone: Result<Zone, String>,
    /// Now, seconds since 1970 (UTC).
    pub now: i64,
    /// The `ID` flag test and `CPUID` leaves 0 and 1.
    pub processor: (u32, [u32; 4], [u32; 4]),
    /// nv-rs's own folder (this system's full path).
    pub own_folder: Result<std::path::PathBuf, String>,
}

/// The table's text and what could not be answered (each a line for the log; such a question
/// then stops the run where the game asks it, saying it was not captured).
pub fn write_table(inputs: &Inputs) -> (String, Vec<String>) {
    let mut table = String::from(WINDOWS_FIXED);
    let mut problems = Vec::new();
    match inputs
        .own_folder
        .as_ref()
        .map_err(String::clone)
        .and_then(|own| folder_lines(own))
    {
        Ok(lines) => table.push_str(&lines),
        Err(why) => problems.push(format!(
            "machine table: no Documents or AppData folders: {why}"
        )),
    }
    match inputs
        .zone
        .as_ref()
        .map_err(String::clone)
        .and_then(|zone| zone_lines(zone, inputs.now))
    {
        Ok(lines) => table.push_str(&lines),
        Err(why) => problems.push(format!("machine table: no time zone: {why}")),
    }
    let (id, leaf_0, leaf_1) = inputs.processor;
    table.push_str(&processor_lines(id, leaf_0, leaf_1));
    table.push_str(&no_nvapi_lines());
    (table, problems)
}

/// The display and graphics-card half of the table ([`display`]), made apart from the rest: it is
/// asked only once the run has chosen whether it draws (the host makes it in its start, after the
/// renderer choice), never when an earlier step only wants the folders.
pub struct DisplayInputs {
    /// The monitors.
    pub monitors: Result<Vec<Monitor>, String>,
    /// The graphics card.
    pub card: Result<Card, String>,
}

/// The display half's lines, what could not be answered (lines for the log), and why the game
/// cannot draw on this card, if it cannot ([`Card::refusal`]: the start is then refused).
pub fn display_part(inputs: &DisplayInputs) -> (String, Vec<String>, Option<String>) {
    let mut table = String::new();
    let mut problems = Vec::new();
    match &inputs.card {
        Ok(card) => {
            table.push_str(&display::card_lines(card));
            let answers = display::honour_samples(display::CARD, &card.sample_counts);
            table.push_str(&display::honour_blending(&answers, card.float32_blendable));
        }
        Err(why) => {
            problems.push(format!("machine table: no graphics card: {why}"));
            table.push_str(display::CARD);
        }
    }
    let card_name = inputs.card.as_ref().map_or("", |c| c.name.as_str());
    match inputs
        .monitors
        .as_ref()
        .map_err(String::clone)
        .and_then(|m| display::display_lines(m, card_name))
    {
        Ok(lines) => table.push_str(&lines),
        Err(why) => problems.push(format!("machine table: no display: {why}")),
    }
    let refusal = inputs.card.as_ref().ok().and_then(Card::refusal);
    (table, problems, refusal)
}

/// The machine's memory as `GlobalMemoryStatusEx` reports it (bytes; the load in percent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    pub load: u32,
    pub total_phys: u64,
    pub avail_phys: u64,
    pub total_page_file: u64,
    pub avail_page_file: u64,
}

/// `dwMemoryLoad`: the share of physical memory in use, rounded down (measured on Windows 11,
/// 2026-11-16: `floor((total - avail) * 100 / total)` over 40 readings crossing 55 to 56).
pub fn memory_load(total: u64, avail: u64) -> u32 {
    if total == 0 {
        return 0;
    }
    (u128::from(total.saturating_sub(avail)) * 100 / u128::from(total)) as u32
}

/// The page-file figures from the memory and swap: the total is memory plus swap; available is
/// what the system says can really be had now, the available memory plus the free swap, never
/// below the available memory and never above the total. (Sizes add with saturation; a 32-bit
/// program's `GlobalMemoryStatus` clamps them further, `super::process`.)
fn page_file(total: u64, avail: u64, swap_total: u64, swap_free: u64) -> (u64, u64) {
    let limit = total.saturating_add(swap_total);
    (
        limit,
        avail.saturating_add(swap_free.min(swap_total)).min(limit),
    )
}

/// Linux, from `/proc/meminfo` (kB): physical memory `MemTotal`, available `MemAvailable` (what
/// can be given to programs without swapping, as Windows' free and standby memory); the page file
/// [`page_file`] with `SwapTotal` and `SwapFree`. Not `Committed_AS`: under Linux's default
/// overcommit it is not held against memory plus swap, and is often above it, which as Windows'
/// commit charge would leave nothing available (the exe's one reader of the available page file,
/// the memory manager's constructor `00aa3880`, ends the game with "Not enough memory" below
/// 512 MB). `None` when a field is missing.
pub fn memory_from_meminfo(text: &str) -> Option<Memory> {
    let field = |name: &str| -> Option<u64> {
        text.lines().find_map(|l| {
            let rest = l.strip_prefix(name)?.strip_prefix(':')?;
            let kb: u64 = rest.trim().strip_suffix("kB")?.trim().parse().ok()?;
            kb.checked_mul(1024)
        })
    };
    let total = field("MemTotal")?;
    let avail = field("MemAvailable")?.min(total);
    let (total_page_file, avail_page_file) =
        page_file(total, avail, field("SwapTotal")?, field("SwapFree")?);
    Some(Memory {
        load: memory_load(total, avail),
        total_phys: total,
        avail_phys: avail,
        total_page_file,
        avail_page_file,
    })
}

/// macOS, from `sysctl`: physical memory `hw.memsize`; available the kernel's own share of free
/// memory (`kern.memorystatus_level`, percent); the page file [`page_file`] with `vm.swapusage`'s
/// total and free swap. macOS makes swap files on demand and compresses memory, so its swap
/// figures are the swap there is now, not a limit: the available page file is a lower bound of
/// what can be had, and never below the available memory (on a Mac with no swap file yet, the
/// available memory itself).
pub fn memory_from_sysctl(
    memsize: u64,
    level_percent: u32,
    swap_total: u64,
    swap_free: u64,
) -> Memory {
    let avail = (u128::from(memsize) * u128::from(level_percent.min(100)) / 100) as u64;
    let (total_page_file, avail_page_file) = page_file(memsize, avail, swap_total, swap_free);
    Memory {
        load: memory_load(memsize, avail),
        total_phys: memsize,
        avail_phys: avail,
        total_page_file,
        avail_page_file,
    }
}

/// The machine's memory now.
#[cfg(target_os = "linux")]
pub fn this_memory() -> Option<Memory> {
    memory_from_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

#[cfg(target_os = "macos")]
mod sysctl {
    use std::ffi::{c_char, c_int, c_void};
    extern "C" {
        fn sysctlbyname(
            name: *const c_char,
            old: *mut c_void,
            old_length: *mut usize,
            new: *mut c_void,
            new_length: usize,
        ) -> c_int;
    }

    /// The value of `name` (a zero-ended name) into `out`, which must be exactly its size.
    pub fn read<T: Copy>(name: &[u8], out: &mut T) -> Option<()> {
        debug_assert!(name.ends_with(&[0]));
        let mut length = std::mem::size_of::<T>();
        // SAFETY: `name` ends with a zero byte; `out` is a live `T` of `length` bytes, and the
        // length is checked after the call.
        let r = unsafe {
            sysctlbyname(
                name.as_ptr().cast(),
                (out as *mut T).cast(),
                &mut length,
                std::ptr::null_mut(),
                0,
            )
        };
        (r == 0 && length == std::mem::size_of::<T>()).then_some(())
    }
}

/// The machine's memory now.
#[cfg(target_os = "macos")]
pub fn this_memory() -> Option<Memory> {
    let mut memsize = 0u64;
    sysctl::read(b"hw.memsize\0", &mut memsize)?;
    let mut level = 0u32;
    sysctl::read(b"kern.memorystatus_level\0", &mut level)?;
    // `struct xsw_usage`: total, available, used (bytes), page size, encrypted.
    let mut swap = [0u64; 4];
    sysctl::read(b"vm.swapusage\0", &mut swap)?;
    Some(memory_from_sysctl(memsize, level, swap[0], swap[1]))
}

/// The machine's memory now: not known on this system.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub fn this_memory() -> Option<Memory> {
    None
}

/// The folder of the system's zone files: `TZDIR`, else `/usr/share/zoneinfo`.
#[cfg(not(windows))]
fn zone_folder() -> std::path::PathBuf {
    std::env::var_os("TZDIR")
        .filter(|d| !d.is_empty())
        .map_or_else(|| "/usr/share/zoneinfo".into(), std::path::PathBuf::from)
}

/// This system's time zone, as the C library takes it: `TZ` (a POSIX rule, or a zone name or
/// `:file` read from the zone files), else `/etc/localtime` ([`read_zone`]).
#[cfg(not(windows))]
pub fn this_zone() -> Result<Zone, String> {
    let tz = std::env::var("TZ").ok();
    read_zone(
        tz.as_deref(),
        &zone_folder(),
        std::path::Path::new("/etc/localtime"),
    )
}

/// The largest zone file read (a real one is a few KB).
const ZONE_FILE_LIMIT: u64 = 1 << 20;

/// A zone file's bytes: only a regular file (a device or a pipe named by `TZ` could block the
/// start or never end), of at most [`ZONE_FILE_LIMIT`] bytes.
fn read_zone_file(path: &std::path::Path) -> std::io::Result<Result<Vec<u8>, String>> {
    use std::io::Read;
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Ok(Err("not a regular file, so not a zone file".into()));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(ZONE_FILE_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > ZONE_FILE_LIMIT {
        return Ok(Err("larger than 1 MB, so not a zone file".into()));
    }
    Ok(Ok(bytes))
}

/// The time zone `tz` (the `TZ` variable's value, if set) names, as the C library takes it: a
/// POSIX rule; else a zone file, `:file` or `file`, a full path or a name in the zone folder
/// `folder`; without `TZ` (or with it empty) the zone file `localtime`. A zone file that is not
/// there gives UTC (the C library's answer then), said once in the log.
pub fn read_zone(
    tz: Option<&str>,
    folder: &std::path::Path,
    localtime: &std::path::Path,
) -> Result<Zone, String> {
    let path = match tz.filter(|t| !t.is_empty()) {
        Some(tz) => {
            if !tz.starts_with(':') {
                if let Ok(rule) = parse_posix_tz(tz) {
                    return Ok(Zone::from_rule(rule));
                }
            }
            let name = std::path::Path::new(tz.trim_start_matches(':'));
            if name.has_root() {
                name.to_path_buf()
            } else {
                folder.join(name)
            }
        }
        None => localtime.to_path_buf(),
    };
    match read_zone_file(&path) {
        Ok(Ok(bytes)) => parse_tzif(&bytes),
        Ok(Err(why)) => Err(why),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "machine table: {} is not there: the time zone is UTC, as the C library takes it",
                path.display()
            );
            parse_posix_tz("UTC0").map(Zone::from_rule)
        }
        Err(e) => Err(e.to_string()),
    }
    .map_err(|why| format!("{}: {why}", path.display()))
}

/// The processor's answers: the real ones on x86-64.
#[cfg(all(not(windows), target_arch = "x86_64"))]
pub fn this_processor() -> (u32, [u32; 4], [u32; 4]) {
    let change: u64;
    // SAFETY: flips EFLAGS.ID, reads the flags back and puts the entry flags back (the same
    // sequence as `scripts/d3d9-capture`'s and the exe's `00ed193c`).
    unsafe {
        std::arch::asm!(
            "pushfq", "pop rax", "mov rcx, rax", "xor rax, 0x200000", "push rax", "popfq",
            "pushfq", "pop rdx", "push rcx", "popfq", "sub rdx, rcx",
            out("rax") _, out("rcx") _, out("rdx") change,
        );
    }
    let leaf = |n: u32| {
        // SAFETY: CPUID is available on every x86-64 processor.
        #[allow(unused_unsafe)] // `__cpuid` is unsafe on older toolchains.
        let r = unsafe { std::arch::x86_64::__cpuid(n) };
        [r.eax, r.ebx, r.ecx, r.edx]
    };
    (change as u32, leaf(0), leaf(1))
}

/// The processor's answers: an x86-64 PC's fixed ones where there is no `CPUID`.
#[cfg(all(not(windows), not(target_arch = "x86_64")))]
pub fn this_processor() -> (u32, [u32; 4], [u32; 4]) {
    fixed_processor()
}

/// This machine's table (Linux and macOS), with nv-rs's own folder `own_folder` for the game's
/// Documents and AppData: the text and what could not be answered. The display and the card are
/// not in it ([`this_display`]): this part asks nothing of the renderer choice, so a step that
/// runs before the choice is made (the settings' reading) cannot fix it.
#[cfg(not(windows))]
pub fn this_machine(own_folder: Result<std::path::PathBuf, String>) -> (String, Vec<String>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    write_table(&Inputs {
        zone: this_zone(),
        now,
        processor: this_processor(),
        own_folder,
    })
}

/// This machine's display and card ([`display_part`]), for a run that draws (`draws`: the host
/// asks it in its start, after the run's renderer choice): the window layer's thread reads the
/// monitors (the window system must be there), wgpu the card. A run that draws nothing asks
/// neither.
#[cfg(not(windows))]
pub fn this_display(draws: bool) -> (String, Vec<String>, Option<String>) {
    let inputs = if draws {
        DisplayInputs {
            monitors: super::d3d9real::monitors(),
            card: d3d9_wgpu::device::card_facts()
                .map(|c| Card {
                    name: c.name,
                    vendor: c.vendor,
                    device: c.device,
                    driver: c.driver,
                    driver_info: c.driver_info,
                    sample_counts: c.sample_counts,
                    bc: c.bc,
                    float32_blendable: c.float32_blendable,
                })
                .ok_or_else(|| "wgpu finds no graphics card".to_string()),
        }
    } else {
        let why = "the run draws nothing (no renderer chosen)".to_string();
        DisplayInputs {
            monitors: Err(why.clone()),
            card: Err(why),
        }
    };
    display_part(&inputs)
}

#[cfg(test)]
mod tests;
