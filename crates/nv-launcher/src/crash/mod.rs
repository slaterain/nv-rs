//! Crash reports: when the game's run ends with a stop or a crash, the launcher writes a report
//! the player can attach to an issue, in nv-rs's own `reports` folder beside the F12 reports
//! (`crash-<UTC time>`: `report.txt` and `log.txt`).
//!
//! A report holds: nv-rs's version and commit (the package's `BUILD.txt`), the system (its name
//! and version, processor, memory, and the graphics card and renderer the run's log or the
//! machine table names), the played profile's nv-rs settings (no paths), the mods' plugin names,
//! how the run ended (the stop line with its exe addresses, a panic, a signal or an exit code)
//! and a fingerprint of it, the log's last lines, and the place and note of an F12 report made
//! during the run. Never a save, a picture or anything of the game's memory. Everything in it but
//! the system's own lines (its name, processor, memory, card, renderer) is scrubbed first
//! ([`scrub`]): the home folder, user and computer names, save names, e-mail and network
//! addresses, and every absolute path but its part below the game's folder or nv-rs's.
//!
//! The same on every system: Windows' Play reads the game's exit code, Linux's and macOS's
//! play.sh passes the program's status on (a signal as 128 + its number, `sh`'s rule).

pub mod consent;
mod scrub;
pub mod send;
mod system;

use scrub::{Known, Scrubber};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The game's stop line (host `player_move.rs`): `Game's main stopped in frame N: <why>`, where
/// `<why>` often ends with the exe's call path, `(in 00a1b2c3 < 00400000)`.
pub const STOP: &str = "Game's main stopped in frame";

/// The lines of a clean end: main returned, or the game ended its own process.
const CLEAN_ENDS: [&str; 2] = ["Game's main returned", "The game ended its process"];

/// How near the log's end a panic must be to be the run's end: its line, its message, the
/// backtrace note, the host's own last lines.
const PANIC_LINES: usize = 12;

/// play.sh's status for the expected stop on a processor nv-rs is not built for: no crash.
const NOT_PROVIDED_STATUS: i32 = 69;

/// How many of the log's last lines a report keeps, each cut at [`LINE_CHARS`] characters.
const LOG_LINES: usize = 300;
const LINE_CHARS: usize = 1000;

/// How many crash reports stay in the reports folder (the oldest go; F12 reports never).
const KEPT_REPORTS: usize = 20;

/// The note F12 puts in its report for the player's words (engine-platform `report_with`).
const F12_NOTE_TEMPLATE: &str = "Write here what went wrong and what you did just before it:";

/// Whose rules read the run's status.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shell {
    /// Windows: the game's own exit code.
    #[cfg_attr(not(windows), allow(dead_code))]
    Windows,
    /// Linux and macOS: play.sh's status, which is the program's (`sh`'s 128 + N for a signal).
    Unix,
}

/// How a run ended badly.
#[derive(Clone, Debug, PartialEq)]
pub enum Ending {
    /// The game's code stopped: the log's stop line.
    Stop(String),
    /// nv-rs's own code panicked: the panic's line and, after it, its message.
    Panic(String),
    /// The system ended the program with this signal (Linux, macOS).
    Signal(i32),
    /// Any other status that is not a clean end.
    Exit(i32),
}

/// How the run ended, from the log's last part `tail` and the status: none for a good end.
pub fn ending(tail: &str, status: Option<i32>, shell: Shell) -> Option<Ending> {
    if status == Some(0) || (shell == Shell::Unix && status == Some(NOT_PROVIDED_STATUS)) {
        return None;
    }
    let lines: Vec<&str> = tail
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.trim().is_empty())
        .collect();
    // The run's own end first: a clean end is one, whatever was said before it.
    if lines
        .last()
        .is_some_and(|l| CLEAN_ENDS.iter().any(|c| l.trim_start().starts_with(c)))
    {
        return None;
    }
    // Main's stop ends the game's code for good: its line is the run's last word.
    if let Some(line) = lines.iter().rev().find(|l| l.contains(STOP)) {
        return Some(Ending::Stop(line.trim().to_string()));
    }
    // A panic only as the run's end (its last lines): one caught and printed earlier (a save
    // copy's, say) is not why a later end happened.
    // A backtrace after it (RUST_BACKTRACE set) is not counted.
    let lines: Vec<&str> = lines.into_iter().filter(|l| !is_backtrace(l)).collect();
    let last = lines.len().saturating_sub(PANIC_LINES);
    if let Some(at) = lines[last..]
        .iter()
        .rposition(|l| l.contains(" panicked at "))
    {
        let at = last + at;
        let mut line = lines[at].trim().to_string();
        if let Some(message) = lines.get(at + 1) {
            line.push(' ');
            line.push_str(message.trim());
        }
        return Some(Ending::Panic(line));
    }
    match (shell, status) {
        (Shell::Unix, Some(s)) if (129..=192).contains(&s) => Some(Ending::Signal(s - 128)),
        (_, Some(s)) => Some(Ending::Exit(s)),
        (_, None) => None,
    }
}

/// Whether `line` is a Rust backtrace's: its heading, a numbered frame, a frame's `at` place, or
/// the `note:` about backtraces.
fn is_backtrace(line: &str) -> bool {
    let t = line.trim_start();
    let numbered = t
        .split_once(':')
        .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    t.starts_with("stack backtrace:") || t.starts_with("at ") || t.starts_with("note: ") || numbered
}

/// An exit code as people look it up: Windows' crash codes in hex (`0xc0000005`).
pub fn code_text(code: i32) -> String {
    if code as u32 >= 0xc000_0000 {
        format!("{:#010x}", code as u32)
    } else {
        code.to_string()
    }
}

/// The plain name of a few crash codes and signals everyone meets.
fn known_name(ending: &Ending) -> Option<&'static str> {
    match ending {
        Ending::Exit(c) => match *c as u32 {
            0xc000_0005 => Some("an access violation"),
            0xc000_00fd => Some("a stack overflow"),
            0xc000_0409 => Some("a fast fail"),
            _ => None,
        },
        Ending::Signal(n) => match n {
            4 => Some("SIGILL"),
            6 => Some("SIGABRT"),
            7 => Some("SIGBUS"),
            8 => Some("SIGFPE"),
            9 => Some("SIGKILL"),
            11 => Some("SIGSEGV"),
            15 => Some("SIGTERM"),
            _ => None,
        },
        _ => None,
    }
}

impl Ending {
    /// What happened, in plain words.
    pub fn plain(&self, status: Option<i32>) -> String {
        let status = status.map_or("unknown".to_string(), code_text);
        match self {
            Ending::Stop(_) => format!("The game's code stopped (status {status})"),
            Ending::Panic(_) => {
                format!("nv-rs stopped on an error in its own code (status {status})")
            }
            Ending::Signal(n) => match known_name(self) {
                Some(name) => format!("The system ended the game (signal {n}, {name})"),
                None => format!("The system ended the game (signal {n})"),
            },
            Ending::Exit(c) => match known_name(self) {
                Some(name) => format!("The game ended with status {} ({name})", code_text(*c)),
                None => format!("The game ended with status {}", code_text(*c)),
            },
        }
    }

    /// The line that says where it stopped, if the log has one.
    pub fn line(&self) -> Option<&str> {
        match self {
            Ending::Stop(l) | Ending::Panic(l) => Some(l),
            _ => None,
        }
    }

    fn scrubbed(&self, s: &Scrubber) -> Ending {
        match self {
            Ending::Stop(l) => Ending::Stop(s.text(l)),
            Ending::Panic(l) => Ending::Panic(s.text(l)),
            other => other.clone(),
        }
    }
}

/// Whether `word` is an exe address as the stop lines write them: eight hex digits.
fn is_address(word: &str) -> bool {
    word.len() == 8 && word.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Whether a stop line's last `(in ` opens its call path: an exe address right after it
/// (`(in 00412345 < 0040cafe)`), not words (`Mods (in progress)`).
fn has_call_path(line: &str) -> bool {
    line.rfind("(in ").is_some_and(|at| {
        let rest = &line[at + 4..];
        is_address(rest.split([' ', ')']).next().unwrap_or(""))
    })
}

/// What makes crashes the same crash, so a crash service puts them in one group: a stop's kind
/// (its text with every word holding a digit as `#`) and the exe addresses of its call path
/// (the last `(in A < B)`; without one, the addresses in its text); a panic's place in the code;
/// the signal; the exit code.
pub fn fingerprint(ending: &Ending) -> Vec<String> {
    match ending {
        Ending::Stop(line) => {
            let why = line
                .find(STOP)
                .map(|at| &line[at + STOP.len()..])
                .and_then(|rest| rest.split_once(": ").map(|(_, why)| why))
                .unwrap_or(line);
            let (text, path) = match why.rfind("(in ") {
                Some(at) => (&why[..at], &why[at + 4..]),
                None => (why, ""),
            };
            let words = |s: &str| -> Vec<String> {
                s.split(|c: char| !c.is_ascii_hexdigit())
                    .filter(|w| is_address(w))
                    .map(str::to_string)
                    .collect()
            };
            let kind: Vec<&str> = text
                .split_whitespace()
                .map(|w| {
                    if w.bytes().any(|b| b.is_ascii_digit()) {
                        "#"
                    } else {
                        w
                    }
                })
                .collect();
            let mut out = vec!["stop".to_string(), kind.join(" ")];
            let addresses = words(path);
            out.extend(if addresses.is_empty() {
                words(text)
            } else {
                addresses
            });
            out
        }
        Ending::Panic(line) => {
            let place = line
                .split_once(" panicked at ")
                .map(|(_, rest)| panic_place(rest))
                .unwrap_or_default();
            vec!["panic".to_string(), place]
        }
        Ending::Signal(n) => vec!["signal".to_string(), n.to_string()],
        Ending::Exit(c) => vec!["exit".to_string(), code_text(*c)],
    }
}

/// Where a panic happened, from its `FILE:LINE:COLUMN:` (`rest`, the message after it): the file
/// from its crate on and the line, the same on every machine and with no folder of the
/// player's: a dependency's `wgpu-core-30.0.1/src/device/mod.rs:123` (from the crate's
/// versioned folder), nv-rs's own `host/src/lib.rs:12` (from the folder before `src`), else
/// the file's name.
pub fn panic_place(rest: &str) -> String {
    // The first `:LINE:COLUMN` (a drive's `C:` has no digits after it).
    let b = rest.as_bytes();
    let digits = |from: usize| {
        b[from.min(b.len())..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count()
    };
    let at = (0..b.len()).find(|&i| {
        b[i] == b':' && {
            let n = digits(i + 1);
            n > 0 && b.get(i + 1 + n) == Some(&b':') && digits(i + 2 + n) > 0
        }
    });
    let (file, number) = match at {
        Some(i) => (&rest[..i], &rest[i + 1..i + 1 + digits(i + 1)]),
        None => (rest.split(':').next().unwrap_or(""), ""),
    };
    let parts: Vec<&str> = file.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
    let versioned = |p: &str| {
        p.rsplit_once('-').is_some_and(|(name, v)| {
            !name.is_empty()
                && v.split('.').count() >= 3
                && v.split('.')
                    .all(|x| !x.is_empty() && x.bytes().all(|c| c.is_ascii_alphanumeric()))
                && v.starts_with(|c: char| c.is_ascii_digit())
        })
    };
    let from = parts
        .iter()
        .rposition(|p| versioned(p))
        .or_else(|| {
            parts
                .iter()
                .rposition(|p| *p == "src")
                .map(|s| s.saturating_sub(1))
        })
        .unwrap_or(parts.len().saturating_sub(1));
    format!("{}:{number}", parts[from.min(parts.len())..].join("/"))
}

/// One run of the game, as the launcher saw it end.
pub struct Run<'a> {
    pub status: Option<i32>,
    pub shell: Shell,
    /// The game's log.
    pub log: &'a Path,
    /// When the game was started (an F12 report made after it belongs to this run).
    pub started: SystemTime,
    /// Played with mods (the `mods` profile).
    pub mods: bool,
    /// nv-rs's own folder (Windows: the package's `userdata`).
    pub own: &'a Path,
    /// The package's folder (the launcher's own), which holds `BUILD.txt`.
    pub program: &'a Path,
    /// The game's folders (as found, and the mods' install of it).
    pub games: Vec<PathBuf>,
    /// The card the machine table names (Windows' adapter table), if any.
    pub table_card: Option<String>,
}

/// A crash report, scrubbed.
pub struct Report {
    pub ending: Ending,
    pub status: Option<i32>,
    pub when: SystemTime,
    pub fingerprint: Vec<String>,
    /// nv-rs's build: its name and version, commit, the launcher's version.
    pub build: Vec<(&'static str, String)>,
    /// The system: name and version, processor, memory, card, renderer.
    pub system: Vec<(&'static str, String)>,
    /// The played profile (`game` or `mods`) and its `[nv-rs ...]` lines.
    pub profile: &'static str,
    pub settings: Vec<String>,
    pub mods: Vec<String>,
    pub f12: Option<F12>,
    /// The log's last lines.
    pub log: Vec<String>,
}

/// An F12 report made during the run: its folder's name, the player's note and the place.
pub struct F12 {
    pub folder: String,
    pub note: Option<String>,
    pub place: Option<String>,
}

/// The first `limit` bytes and the last `limit` bytes of the file at `path`, as text (empty
/// when it cannot be read); the tail starts at a whole line.
fn ends_of(path: &Path, limit: u64) -> (String, String) {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return (String::new(), String::new());
    };
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut head = Vec::new();
    let _ = (&mut file).take(limit).read_to_end(&mut head);
    let mut tail = Vec::new();
    if size > limit {
        let from = size.saturating_sub(limit);
        if file.seek(SeekFrom::Start(from)).is_ok() {
            let _ = file.read_to_end(&mut tail);
            let line = tail.iter().position(|&b| b == b'\n').map_or(0, |at| at + 1);
            tail.drain(..line);
        }
    } else {
        tail = head.clone();
    }
    (
        String::from_utf8_lossy(&head).into_owned(),
        String::from_utf8_lossy(&tail).into_owned(),
    )
}

/// The renderer and card the run's log names (`NV_D3D9=wgpu: drawing with <card (API)>`, the
/// real Direct3D 9's device or its fallback), the machine table's card otherwise.
fn renderer(log: &str, table_card: Option<&str>) -> (String, Option<String>) {
    let table = table_card.map(str::to_string);
    for line in log.lines() {
        if let Some(card) = line
            .split_once("NV_D3D9=wgpu: drawing with ")
            .map(|(_, c)| c)
        {
            return ("wgpu".into(), Some(card.trim().to_string()));
        }
    }
    if log.contains("the real Direct3D 9 draws instead") {
        return (
            "Direct3D 9, the system's (no card wgpu could draw with)".into(),
            table,
        );
    }
    if log.contains("NV_D3D9=real: the game's device") {
        return ("Direct3D 9, the system's".into(), table);
    }
    ("not named in the log".into(), table)
}

/// `BUILD.txt`'s first line (`nv-rs <system> preview <version>`) and its commit.
fn build_lines(program: &Path) -> Vec<(&'static str, String)> {
    let text = std::fs::read(program.join("BUILD.txt"))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let mut lines = text.lines().map(str::trim);
    let mut out = vec![(
        "nv-rs",
        lines
            .next()
            .filter(|l| !l.is_empty())
            .unwrap_or("built from source (no BUILD.txt)")
            .to_string(),
    )];
    if let Some(commit) = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("Source commit: "))
    {
        out.push(("Commit", commit.trim().to_string()));
    }
    out.push(("Launcher", env!("CARGO_PKG_VERSION").to_string()));
    out
}

/// Whether an INI value names a file or folder (left out of the report).
fn looks_like_path(value: &str) -> bool {
    let v = value.trim();
    v.contains('\\')
        || v.contains('/')
        || (v.len() >= 2 && v.as_bytes()[1] == b':' && v.as_bytes()[0].is_ascii_alphabetic())
}

/// The `[nv-rs ...]` sections' lines of `ini` (any case), `[section] key=value`, paths left out.
fn nv_rs_settings(ini: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut section: Option<String> = None;
    for line in ini.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name
                .trim()
                .to_ascii_lowercase()
                .starts_with("nv-rs")
                .then(|| name.trim().to_string());
            continue;
        }
        let Some(section) = &section else {
            continue;
        };
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = if looks_like_path(value) {
                "(a path, left out)"
            } else {
                value.trim()
            };
            out.push(format!("[{section}] {}={value}", key.trim()));
        }
    }
    out
}

/// The plugins a `plugins.txt` switches on, in its order (`*` marks taken off).
fn plugin_names(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim().trim_start_matches('*').trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .take(500)
        .map(str::to_string)
        .collect()
}

/// How much of an F12 report's note or place is read, and how much of it a report keeps
/// (after scrubbing: a cut path would keep a part of a name).
const READ_CHARS: usize = 64 * 1024;
const NOTE_CHARS: usize = 4000;

/// `text` scrubbed by `s`, then cut at `chars` characters.
fn scrubbed_cut(s: &Scrubber, text: &str, chars: usize) -> String {
    s.text(text).chars().take(chars).collect()
}

/// A file as text, at most `chars` characters, trimmed; none when missing or empty.
fn short_text(path: &Path, chars: usize) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text: String = String::from_utf8_lossy(&bytes)
        .chars()
        .take(chars)
        .collect();
    let text = text.trim().replace("\r\n", "\n");
    (!text.is_empty()).then_some(text)
}

/// The newest F12 report folder (`NNN`) in `reports` whose note was written at or after
/// `started`.
fn f12_during(reports: &Path, started: SystemTime) -> Option<F12> {
    let mut found: Option<(u32, PathBuf)> = None;
    for entry in std::fs::read_dir(reports).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(number) = name.parse::<u32>() else {
            continue;
        };
        let path = entry.path();
        let new = std::fs::metadata(path.join("note.txt"))
            .and_then(|m| m.modified())
            .is_ok_and(|t| t >= started);
        if new && found.as_ref().is_none_or(|(n, _)| number > *n) {
            found = Some((number, path));
        }
    }
    let (_, path) = found?;
    let note = short_text(&path.join("note.txt"), READ_CHARS)
        .map(|n| n.replacen(F12_NOTE_TEMPLATE, "", 1).trim().to_string())
        .filter(|n| !n.is_empty());
    Some(F12 {
        folder: path.file_name()?.to_string_lossy().into_owned(),
        note,
        place: short_text(&path.join("place.txt"), READ_CHARS),
    })
}

/// The report of `run`, which ended as `ending`, on a system of `facts`, scrubbed by `s`.
fn build(
    run: &Run,
    ending: &Ending,
    facts: &system::Facts,
    s: &Scrubber,
    now: SystemTime,
) -> Report {
    let (head, tail) = ends_of(run.log, 1024 * 1024);
    let raw = ending;
    let ending = ending.scrubbed(s);
    let (renderer, card) = renderer(&head, run.table_card.as_deref());
    let mut machine = vec![("System", facts.os.clone())];
    machine.push((
        "Processor",
        match facts.threads {
            Some(n) => format!("{} ({n} threads)", facts.cpu),
            None => facts.cpu.clone(),
        },
    ));
    if let Some(bytes) = facts.memory {
        machine.push(("Memory", system::memory_text(bytes)));
    }
    machine.push(("Graphics card", card.unwrap_or_else(|| "not named".into())));
    machine.push(("Renderer", renderer));
    // The system's own words (its name, the processor's, the card's), not scrubbed: a host named
    // like the system ("fedora") would turn them into placeholders.
    let system = machine;
    let profile = if run.mods { "mods" } else { "game" };
    let ini = std::fs::read(crate::profiles::profile_dir(run.own, profile).join("Fallout.ini"))
        .map(|b| b.iter().map(|&c| c as char).collect::<String>())
        .unwrap_or_default();
    let settings = nv_rs_settings(&ini).iter().map(|l| s.text(l)).collect();
    let mods = if run.mods {
        let plugins = run
            .own
            .join("mods")
            .join("AppData")
            .join("Local")
            .join("FalloutNV")
            .join("plugins.txt");
        let text = std::fs::read(plugins)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        plugin_names(&text).iter().map(|m| s.file_name(m)).collect()
    } else {
        Vec::new()
    };
    let f12 = f12_during(&run.own.join("reports"), run.started).map(|f| F12 {
        folder: f.folder,
        note: f.note.map(|n| scrubbed_cut(s, &n, NOTE_CHARS)),
        place: f.place.map(|p| scrubbed_cut(s, &p, NOTE_CHARS)),
    });
    let lines: Vec<&str> = tail.lines().collect();
    let log = lines[lines.len().saturating_sub(LOG_LINES)..]
        .iter()
        .map(|l| scrubbed_cut(s, l.trim_end_matches('\r'), LINE_CHARS))
        .collect();
    Report {
        // A panic's place from the line as it was, cut to its crate's part (scrubbing would turn a
        // dependency's folder into `<path>` and group every crate's `mod.rs:12` together).
        // A stop's kind from the scrubbed line (its paths the same on every machine), its call
        // path's exe addresses from the line as it was (scrubbing never reaches into them then).
        fingerprint: match raw {
            Ending::Panic(_) => fingerprint(raw).iter().map(|p| s.text(p)).collect(),
            // Only a call path's addresses, `(in A < B)`: without one (`Mods (in progress)`
            // is none), the eight-hex words of the text could be a user's number, so those come
            // from the scrubbed line.
            Ending::Stop(line) if has_call_path(line) => {
                let mut f = fingerprint(&ending);
                f.truncate(2);
                f.extend(fingerprint(raw).into_iter().skip(2));
                f
            }
            _ => fingerprint(&ending),
        },
        ending,
        status: run.status,
        when: now,
        build: build_lines(run.program)
            .into_iter()
            .map(|(k, v)| (k, s.text(&v)))
            .collect(),
        system,
        profile,
        settings,
        mods,
        f12,
        log,
    }
}

/// `t` in UTC: `2026-11-25 10:04:05` and `2026-11-25_10-04-05` (for a folder's name).
pub fn utc(t: SystemTime) -> (String, String) {
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days to a civil date (the proleptic Gregorian calendar).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let (h, m, s) = (rest / 3600, rest % 3600 / 60, rest % 60);
    (
        format!("{year:04}-{month:02}-{day:02} {h:02}:{m:02}:{s:02}"),
        format!("{year:04}-{month:02}-{day:02}_{h:02}-{m:02}-{s:02}"),
    )
}

impl Report {
    /// The report as the player reads it (`report.txt`): [`Report::body`] and where its log is.
    pub fn text(&self) -> String {
        let mut t = self.body();
        t.push_str(&format!(
            "
The log's last {} lines are in log.txt beside this file.
",
            self.log.len()
        ));
        t
    }

    /// Everything the report says but where its log is.
    pub fn body(&self) -> String {
        self.body_with(true)
    }

    /// What of it is sent ([`send`]): the body without an F12 report's note and place, which
    /// stay on this PC (said when there is one).
    pub fn sent_body(&self) -> String {
        let mut t = self.body_with(false);
        if self.f12.is_some() {
            t.push_str(
                "\nThe note and place of the F12 report made during this run stay on this PC.\n",
            );
        }
        t
    }

    /// The log lines that are sent: the game's own `F12: ` lines (the place, the report's
    /// folder) left out, as the F12 report's note and place are; the local copy keeps them.
    pub fn sent_log(&self) -> Vec<&str> {
        self.log
            .iter()
            .map(String::as_str)
            .filter(|l| !l.trim_start().starts_with("F12:"))
            .collect()
    }

    /// The body, with the F12 report's part or without it.
    fn body_with(&self, with_f12: bool) -> String {
        let mut t = String::new();
        let mut line = |s: &str| {
            t.push_str(s);
            t.push('\n');
        };
        line("nv-rs crash report");
        line("");
        line("nv-rs's launcher made this report when the game stopped. Taken out of it: your user");
        line("name (<user>), your computer's name (<computer>), your home folder (<home>, and");
        line("<onedrive>), the game's and nv-rs's folders (<game>, <nv-rs>, <nv-rs-data>), other");
        line("folders (<path>), save names (<save>), account numbers (<id>), e-mail addresses");
        line("(<e-mail>) and network addresses (<ip>). An F12 note is kept as you wrote it, with");
        line("only those taken out. It holds no saves, no pictures and nothing of the game's memory.");
        line("");
        line(&format!(
            "What happened: {}",
            self.ending.plain(self.status)
        ));
        if let Some(stop) = self.ending.line() {
            line(&format!("Stop line:     {stop}"));
        }
        line(&format!("Grouped as:    {}", self.fingerprint.join(" | ")));
        line(&format!("When (UTC):    {}", utc(self.when).0));
        line("");
        for (k, v) in self.build.iter().chain(&self.system) {
            line(&format!("{:<15}{v}", format!("{k}:")));
        }
        line("");
        line(&format!(
            "Settings (the nv-rs sections of the {} profile's Fallout.ini):",
            self.profile
        ));
        if self.settings.is_empty() {
            line("  none found");
        }
        for s in &self.settings {
            line(&format!("  {s}"));
        }
        line("");
        if self.profile == "mods" {
            line(&format!("Mods (plugins.txt, {}):", self.mods.len()));
            for m in &self.mods {
                line(&format!("  {m}"));
            }
        } else {
            line("Mods: played without mods");
        }
        if let Some(f12) = self.f12.as_ref().filter(|_| with_f12) {
            line("");
            line(&format!(
                "F12 report made during this run: reports/{}",
                f12.folder
            ));
            if let Some(place) = &f12.place {
                line("  Where you were:");
                for l in place.lines() {
                    line(&format!("    {l}"));
                }
            }
            match &f12.note {
                Some(note) => {
                    line("  Your note:");
                    for l in note.lines() {
                        line(&format!("    {l}"));
                    }
                }
                None => line("  Your note: (not written)"),
            }
        }
        t
    }

    /// The report kept in `reports` (nv-rs's own reports folder): a new `crash-<UTC time>`
    /// folder with `report.txt` and `log.txt`. The oldest crash reports past [`KEPT_REPORTS`] go.
    pub fn save(&self, reports: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(reports)?;
        let stamp = utc(self.when).1;
        let mut folder = None;
        for n in 1..100 {
            let name = if n == 1 {
                format!("crash-{stamp}")
            } else {
                format!("crash-{stamp}-{n}")
            };
            let path = reports.join(name);
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    folder = Some(path);
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        let folder = folder.ok_or_else(|| std::io::Error::other("no free report name"))?;
        let lines = |text: String| -> Vec<u8> {
            if cfg!(windows) {
                text.replace('\n', "\r\n").into_bytes()
            } else {
                text.into_bytes()
            }
        };
        std::fs::write(folder.join("report.txt"), lines(self.text()))?;
        let mut log = self.log.join("\n");
        log.push('\n');
        std::fs::write(folder.join("log.txt"), lines(log))?;
        tidy(reports, KEPT_REPORTS);
        Ok(folder)
    }
}

/// The newest `keep` crash reports in `reports` stay; older `crash-*` folders go.
fn tidy(reports: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(reports) else {
        return;
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("crash-"))
        .collect();
    names.sort();
    let old = names.len().saturating_sub(keep);
    for name in &names[..old] {
        let _ = std::fs::remove_dir_all(reports.join(name));
    }
}

/// What became of a bad end's report: the folder it was kept in (or why it was not), and what
/// became of sending it (a line for the player; none when nothing was to be sent).
pub struct Outcome {
    pub kept: Result<PathBuf, String>,
    pub sent: Option<String>,
}

/// After a run: when it ended badly, its report built from this system, kept in nv-rs's own
/// reports folder, and sent as the player chose ([`deliver`]). None for a good end.
pub fn after_run(run: &Run) -> Option<Outcome> {
    let (_, tail) = ends_of(run.log, 1024 * 1024);
    let ending = ending(&tail, run.status, run.shell)?;
    let games: Vec<&Path> = run.games.iter().map(PathBuf::as_path).collect();
    let scrubber = Scrubber::new(&Known::from_system(&games, run.program, run.own));
    let report = build(run, &ending, &system::facts(), &scrubber, SystemTime::now());
    let reports = run.own.join("reports");
    let kept = report
        .save(&reports)
        .map_err(|e| format!("{}: {e}", reports.display()));
    let sent = kept.as_ref().ok().and_then(|folder| {
        deliver(
            &report,
            folder,
            run.own,
            run.program,
            &send::curl(),
            consent::session(run.own),
            ask_player,
        )
    });
    Some(Outcome { kept, sent })
}

/// What happens to a kept report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Next {
    Send,
    Ask,
    /// Kept on this PC only.
    Keep,
}

/// What happens to a report with the player's `answer`, where the package can send
/// (`can_send`: it has the crash service's file): nothing is sent without one, or without a yes.
pub fn next_step(answer: consent::Consent, can_send: bool) -> Next {
    match (can_send, answer) {
        (true, consent::Consent::Send) => Next::Send,
        (true, consent::Consent::Ask) => Next::Ask,
        _ => Next::Keep,
    }
}

/// The question after a crash with "Ask me each time": what would go under it (the report
/// without an F12 report's note and place, and how many of its log's last lines go, without the
/// game's `F12: ` lines).
fn question(report: &Report) -> String {
    format!(
        "The game stopped, and nv-rs kept the crash report below. Send it to nv-rs's developers?\n\n{}\nThe log's last {} lines go with it.\n",
        report.sent_body(),
        report.sent_log().len()
    )
}

/// The per-crash question's entries: "Don't send" first, so it is the highlighted one, a message
/// box's default button and the answer without a window; "Send" is the only yes.
pub const ASK_ENTRIES: [&str; 2] = ["Don't send", "Send"];

/// The player asked through `send` (the launcher's window, [`crate::session::send`]): a yes only
/// for "Send". Esc and a closed window answer "Don't send"; a reply dropped without an answer and
/// no window at all are a no.
fn ask_with(send: impl FnOnce(crate::session::Event) -> bool, text: &str) -> bool {
    let (reply, answer) = std::sync::mpsc::channel();
    let asked = send(crate::session::Event::Message {
        tone: crate::session::Tone::Question,
        text: text.to_string(),
        entries: ASK_ENTRIES.iter().map(|e| e.to_string()).collect(),
        reply,
    });
    asked && answer.recv() == Ok(1)
}

/// [`ask_with`] the launcher's window.
fn ask_player(text: &str) -> bool {
    ask_with(crate::session::send, text)
}

/// What the player sees while curl posts a report (up to 30 seconds).
pub const SENDING: &str = "Sending the crash report...";

/// `report`, kept in `folder`, sent or not as the player answered (nv-rs's own folder `own`
/// holds the saved answer, `session` this launcher's own if it gave one, and the stricter of the
/// two is in force; the launcher's folder `program` holds the crash service's file), by the
/// program `curl`, `ask` asking first where the answer is "ask each time". The line for the
/// player, none when nothing was to be sent.
pub fn deliver(
    report: &Report,
    folder: &Path,
    own: &Path,
    program: &Path,
    curl: &Path,
    session: Option<consent::Consent>,
    ask: impl FnOnce(&str) -> bool,
) -> Option<String> {
    let dsn = send::package_dsn(program);
    let answer = consent::in_force(consent::read(own), session);
    match next_step(answer, dsn.is_some()) {
        Next::Keep => return None,
        Next::Ask if !ask(&question(report)) => {
            return Some("The crash report was not sent: it stays on this PC.".into())
        }
        Next::Ask | Next::Send => {}
    }
    // Seen while it goes: Windows' waiting screen, a line after play.sh's elsewhere.
    crate::session::send(if cfg!(windows) {
        crate::session::Event::Status(SENDING.into())
    } else {
        crate::session::Event::Output(format!("\n{SENDING}\n"))
    });
    Some(match send::send(report, folder, dsn.as_ref()?, curl) {
        Ok(_) => "The crash report was sent to nv-rs's developers. Thank you.".into(),
        Err(why) => {
            format!("The crash report could not be sent ({why}); it stays on this PC.")
        }
    })
}

/// The example stop line of [`sample`]: said to be an example, with made-up addresses.
const EXAMPLE_STOP: &str =
    "Game's main stopped in frame 1234: (an example: what the game's code was doing) (in 00a1b2c3 < 00400000)";

/// How many of the log's last lines [`sample`] shows.
const SAMPLE_LOG_LINES: usize = 20;

/// The newest game log in `logs` (`game-<time>.log`: the names sort by time).
fn newest_log(logs: &Path) -> Option<PathBuf> {
    let mut names: Vec<String> = std::fs::read_dir(logs)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("game-") && n.ends_with(".log"))
        .collect();
    names.sort();
    names.pop().map(|n| logs.join(n))
}

/// What a crash report holds, for the player to read before answering whether reports may be
/// sent: one made now from this PC around an example stop line, as a real one is made and
/// scrubbed: its system, the played profile's settings and mods (`mods`: the mods profile, when
/// mods are installed), the card the machine table names (`card`), the newest game log's end, and
/// the newest F12 report's note and place when there is one. Nothing is written.
pub fn sample(
    own: &Path,
    program: &Path,
    game: Option<&Path>,
    mods: bool,
    card: Option<String>,
) -> String {
    let log = newest_log(&own.join("logs")).unwrap_or_default();
    let run = Run {
        status: Some(3),
        shell: if cfg!(windows) {
            Shell::Windows
        } else {
            Shell::Unix
        },
        log: &log,
        // Any F12 report counts here: the example shows what one adds.
        started: SystemTime::UNIX_EPOCH,
        mods,
        own,
        program,
        games: game.into_iter().map(Path::to_path_buf).collect(),
        table_card: card,
    };
    let games: Vec<&Path> = game.into_iter().collect();
    let scrubber = Scrubber::new(&Known::from_system(&games, program, own));
    let report = build(
        &run,
        &Ending::Stop(EXAMPLE_STOP.into()),
        &system::facts(),
        &scrubber,
        SystemTime::now(),
    );
    sample_text(&report)
}

/// [`sample`]'s text: the report as `report.txt` has it, then the end of its log (a report keeps
/// its last [`LOG_LINES`]; the last [`SAMPLE_LOG_LINES`] show here).
fn sample_text(report: &Report) -> String {
    // Any F12 report is the example's: said so (a real report holds only that run's).
    let mut text = report.body().replace(
        "F12 report made during this run:",
        "Your newest F12 report, as an example (a report holds only one made in its run):",
    );
    let shown = report.log.len().min(SAMPLE_LOG_LINES);
    if shown == 0 {
        text.push_str(&format!(
            "\nThe game's log: a report keeps its last {LOG_LINES} lines (there is no game log here yet).\n"
        ));
    } else {
        text.push_str(&format!(
            "\nThe game's log: a report keeps its last {} lines; the last {shown} of your newest log, as a report holds them:\n",
            report.log.len()
        ));
        for line in &report.log[report.log.len() - shown..] {
            text.push_str(line);
            text.push('\n');
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example is made from this PC and says it is an example; it writes nothing, shows the
    /// newest log's end scrubbed (and how much of it a report keeps), says so when there is no
    /// log yet, and shows mods, the machine table's card and an F12 report's note when there are
    /// some.
    #[test]
    fn the_example_report_is_made_from_this_pc_and_writes_nothing() {
        let own = std::env::temp_dir().join(format!("nv-launcher-sample-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&own);
        std::fs::create_dir_all(&own).unwrap();
        let text = sample(&own, &own, None, false, None);
        assert!(
            text.contains("(an example: what the game's code was doing)"),
            "{text}"
        );
        assert!(text.contains("System:"));
        assert!(text.contains("Mods: played without mods"));
        assert!(text.contains("there is no game log here yet"), "{text}");
        assert!(!text.contains("log.txt"), "{text}");
        let logs = own.join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("game-1.log"), "an older run's line\n").unwrap();
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap();
        std::fs::write(
            logs.join("game-2.log"),
            format!("first\nat {home}/a/x.ini\n"),
        )
        .unwrap();
        let plugins = own
            .join("mods")
            .join("AppData")
            .join("Local")
            .join("FalloutNV");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(plugins.join("plugins.txt"), "FalloutNV.esm\nSome Mod.esp\n").unwrap();
        let f12 = own.join("reports").join("001");
        std::fs::create_dir_all(&f12).unwrap();
        std::fs::write(f12.join("note.txt"), "the door would not open\n").unwrap();
        let text = sample(&own, &own, None, true, Some("Table Card".into()));
        assert!(
            text.contains("a report keeps its last 2 lines; the last 2 of your newest log"),
            "{text}"
        );
        assert!(text.contains("at <home>/.../x.ini"), "{text}");
        assert!(!text.contains("an older run's line"));
        assert!(text.contains("  Some Mod.esp"), "{text}");
        assert!(text.contains("Graphics card: Table Card"), "{text}");
        assert!(text.contains("    the door would not open"), "{text}");
        assert!(
            text.contains("Your newest F12 report, as an example"),
            "{text}"
        );
        assert!(!text.contains("made during this run"), "{text}");
        assert!(!text.contains("log.txt"), "{text}");
        let made: Vec<_> = std::fs::read_dir(own.join("reports")).unwrap().collect();
        assert_eq!(made.len(), 1, "only the F12 report: nothing written");
        let _ = std::fs::remove_dir_all(&own);
    }

    #[test]
    fn a_stop_line_a_panic_a_signal_and_a_code_are_crashes_a_clean_end_is_not() {
        let stop =
            "x\nGame's main stopped in frame 12: open function 00123456 (in 00a1b2c3 < 00400000)\n";
        assert_eq!(
            ending(stop, Some(3), Shell::Windows),
            Some(Ending::Stop(
                "Game's main stopped in frame 12: open function 00123456 (in 00a1b2c3 < 00400000)"
                    .into()
            ))
        );
        let panic = "a\nthread 'main' panicked at nv-rs/crates/host/src/lib.rs:12:5:\nno table\nnote: run with RUST_BACKTRACE=1\n";
        assert_eq!(
            ending(panic, Some(101), Shell::Unix),
            Some(Ending::Panic(
                "thread 'main' panicked at nv-rs/crates/host/src/lib.rs:12:5: no table".into()
            ))
        );
        assert_eq!(
            ending("x\n", Some(139), Shell::Unix),
            Some(Ending::Signal(11))
        );
        // The same number from Windows is a code, not a signal.
        assert_eq!(
            ending("x\n", Some(139), Shell::Windows),
            Some(Ending::Exit(139))
        );
        assert_eq!(
            ending("x\n", Some(0xc000_0005_u32 as i32), Shell::Windows),
            Some(Ending::Exit(0xc000_0005_u32 as i32))
        );
        // Good ends: status 0, play.sh's expected stop, the game's own end with a code.
        assert_eq!(ending(stop, Some(0), Shell::Windows), None);
        assert_eq!(ending("x\n", Some(69), Shell::Unix), None);
        assert_eq!(
            ending("x\n", Some(69), Shell::Windows),
            Some(Ending::Exit(69))
        );
        assert_eq!(
            ending(
                "The game ended its process in frame 9: ExitProcess(1)\r\n",
                Some(1),
                Shell::Windows
            ),
            None
        );
        assert_eq!(
            ending("Game's main returned 2 in frame 9\n", Some(2), Shell::Unix),
            None
        );
        assert_eq!(ending("x\n", None, Shell::Unix), None);
        // A panic caught and printed long before the end is not the end: the run's own end is.
        let mut caught = String::from(
            "thread 'saves' panicked at nv-rs/crates/host/src/save_out.rs:871:9:\nfull disk\n",
        );
        for n in 0..PANIC_LINES {
            caught.push_str(&format!("line {n}\n"));
        }
        assert_eq!(
            ending(&caught, Some(0xc000_0005_u32 as i32), Shell::Windows),
            Some(Ending::Exit(0xc000_0005_u32 as i32))
        );
        caught.push_str("Game's main returned 2 in frame 9\n");
        assert_eq!(ending(&caught, Some(2), Shell::Windows), None);
        // A clean end after a panic near the end is a clean end; a panic at the end is a panic.
        assert_eq!(
            ending(
                "thread 'x' panicked at x.rs:1:2:\nboom\nGame's main returned 0 in frame 3\n",
                Some(2),
                Shell::Unix
            ),
            None
        );
        assert!(matches!(
            ending(
                "a\nthread 'main' panicked at x.rs:1:2:\nboom\n",
                Some(101),
                Shell::Windows
            ),
            Some(Ending::Panic(_))
        ));
    }

    #[test]
    fn the_same_crash_groups_alike_and_another_apart() {
        let a = Ending::Stop(
            "Game's main stopped in frame 12: read of 1a2b3c00 not mapped (in 00a1b2c3 < 00400000)"
                .into(),
        );
        let b = Ending::Stop("Game's main stopped in frame 9000: read of 7f000010 not mapped (in 00a1b2c3 < 00400000)".into());
        assert_eq!(fingerprint(&a), fingerprint(&b));
        assert_eq!(
            fingerprint(&a),
            vec!["stop", "read of # not mapped", "00a1b2c3", "00400000"]
        );
        let other = Ending::Stop(
            "Game's main stopped in frame 12: read of 1a2b3c00 not mapped (in 00a1b2c4 < 00400000)"
                .into(),
        );
        assert_ne!(fingerprint(&a), fingerprint(&other));
        // Without a call path: the addresses in its text.
        assert_eq!(
            fingerprint(&Ending::Stop(
                "Game's main stopped in frame 3: open function 00abcdef".into()
            )),
            vec!["stop", "open function #", "00abcdef"]
        );
        assert_eq!(
            fingerprint(&Ending::Panic(
                "thread 'main' panicked at nv-rs/crates/host/src/lib.rs:12:5: no table".into()
            )),
            vec!["panic", "host/src/lib.rs:12"]
        );
        // A dependency's panic keeps its crate (from its versioned folder), never the folders above.
        assert_eq!(
            panic_place("C:\\x\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\wgpu-core-30.0.1\\src\\device\\mod.rs:123:4:\nmsg"),
            "wgpu-core-30.0.1/src/device/mod.rs:123"
        );
        assert_ne!(
            panic_place("/r/naga-26.0.0/src/mod.rs:12:1:"),
            panic_place("/r/wgpu-hal-26.0.0/src/mod.rs:12:1:")
        );
        assert_eq!(panic_place("/home/x/lone.rs:7:1:"), "lone.rs:7");
        // A dependency's file outside its `src` (a build script): still from its crate on.
        assert_eq!(
            panic_place("/r/registry/src/index.crates.io-x/foo-1.2.3/build.rs:10:5:"),
            "foo-1.2.3/build.rs:10"
        );
        assert_eq!(fingerprint(&Ending::Signal(11)), vec!["signal", "11"]);
        assert_eq!(
            fingerprint(&Ending::Exit(0xc000_0005_u32 as i32)),
            vec!["exit", "0xc0000005"]
        );
        assert_eq!(fingerprint(&Ending::Exit(1)), vec!["exit", "1"]);
    }

    #[test]
    fn endings_are_said_plainly() {
        assert_eq!(
            Ending::Signal(11).plain(Some(139)),
            "The system ended the game (signal 11, SIGSEGV)"
        );
        assert_eq!(
            Ending::Exit(0xc000_0005_u32 as i32).plain(None),
            "The game ended with status 0xc0000005 (an access violation)"
        );
        assert_eq!(
            Ending::Stop(String::new()).plain(Some(3)),
            "The game's code stopped (status 3)"
        );
    }

    #[test]
    fn utc_time_is_written_as_people_read_it() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        assert_eq!(
            utc(t),
            (
                "2001-09-09 01:46:40".to_string(),
                "2001-09-09_01-46-40".to_string()
            )
        );
        assert_eq!(utc(SystemTime::UNIX_EPOCH).0, "1970-01-01 00:00:00");
        let leap = SystemTime::UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(utc(leap).0, "2000-02-29 00:00:00");
    }

    #[test]
    fn settings_keep_the_nv_rs_sections_and_leave_paths_out() {
        let ini = "[Display]\r\niSize H=720\r\n[nv-rs General]\r\nsPreset=fixes\r\n; a comment\r\n[NV-RS Display]\r\nsRenderer=wgpu\r\nsShotDir=D:\\shots\r\nsOther=/tmp/x\r\n[General]\r\nsLanguage=ENGLISH\r\n";
        assert_eq!(
            nv_rs_settings(ini),
            vec![
                "[nv-rs General] sPreset=fixes",
                "[NV-RS Display] sRenderer=wgpu",
                "[NV-RS Display] sShotDir=(a path, left out)",
                "[NV-RS Display] sOther=(a path, left out)",
            ]
        );
        assert_eq!(
            plugin_names("# comment\r\nFalloutNV.esm\r\n*YUP.esm\r\n\r\n"),
            vec!["FalloutNV.esm", "YUP.esm"]
        );
    }

    /// A panic followed by its backtrace (RUST_BACKTRACE set) is still the run's end.
    #[test]
    fn a_panic_with_its_backtrace_is_the_end() {
        let mut log = String::from(
            "x\nthread 'main' panicked at nv-rs/crates/host/src/lib.rs:12:5:\nno table\nstack backtrace:\n",
        );
        for n in 0..40 {
            log.push_str(&format!(
                "  {n}: std::panicking::frame_{n}\n             at /rustc/abc/library/std/src/panicking.rs:{n}:5\n"
            ));
        }
        log.push_str("note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.\n");
        assert_eq!(
            ending(&log, Some(101), Shell::Windows),
            Some(Ending::Panic(
                "thread 'main' panicked at nv-rs/crates/host/src/lib.rs:12:5: no table".into()
            ))
        );
    }

    /// A stop's exe addresses group the same crash on every machine: a call path's taken from
    /// the line as it was, even with a user name made of digits or hex letters.
    #[test]
    fn a_stops_addresses_are_kept_for_grouping() {
        let root = std::env::temp_dir().join(format!("nv-launcher-group-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("logs")).unwrap();
        let log = root.join("logs").join("game-1.log");
        let line =
            "Game's main stopped in frame 3: read of 1a2b3c00 not mapped (in 00412345 < 0040cafe)";
        std::fs::write(&log, format!("{line}\n")).unwrap();
        let run = Run {
            status: Some(3),
            shell: Shell::Windows,
            log: &log,
            started: SystemTime::now(),
            mods: false,
            own: &root,
            program: &root,
            games: Vec::new(),
            table_card: None,
        };
        let s = Scrubber::new(&Known {
            roots: Vec::new(),
            words: vec![("12345".into(), "<user>"), ("cafe".into(), "<user>")],
        });
        let facts = system::Facts {
            os: "x".into(),
            cpu: "x".into(),
            threads: None,
            memory: None,
        };
        let report = build(
            &run,
            &Ending::Stop(line.into()),
            &facts,
            &s,
            SystemTime::now(),
        );
        assert_eq!(
            report.fingerprint,
            ["stop", "read of # not mapped", "00412345", "0040cafe"]
        );
        // Without a call path the text's eight-hex words come from the scrubbed line: one of
        // them could be the user's number.
        let num = Scrubber::new(&Known {
            roots: Vec::new(),
            words: vec![("20231234".into(), "<user>")],
        });
        let bare = "Game's main stopped in frame 3: no save for 20231234 at 00412345";
        let report = build(
            &run,
            &Ending::Stop(bare.into()),
            &facts,
            &num,
            SystemTime::now(),
        );
        assert_eq!(
            report.fingerprint,
            ["stop", "no save for <user> at #", "00412345"]
        );
        // Words in brackets are no call path: `(in progress)`, `(in a script)`.
        for (line, want) in [
            (
                "Game's main stopped in frame 3: could not read G:\\Backups\\20231234\\Mods (in progress)\\x.esp",
                "could not read <path>\\x.esp",
            ),
            (
                "Game's main stopped in frame 3: could not read G:\\Steam\\userdata\\87654321\\x.vdf (in a script)",
                "could not read <path>\\x.vdf",
            ),
        ] {
            let report = build(
                &run,
                &Ending::Stop(line.into()),
                &facts,
                &num,
                SystemTime::now(),
            );
            assert_eq!(report.fingerprint, ["stop", want], "{line}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_renderer_is_read_from_the_log_else_the_table() {
        assert_eq!(
            renderer("NV_D3D9=wgpu: drawing with Some Card (Vulkan)\n", Some("T")),
            ("wgpu".to_string(), Some("Some Card (Vulkan)".to_string()))
        );
        assert_eq!(
            renderer(
                "NV_D3D9=real: the game's device is on the real window (1x1)",
                Some("T")
            ),
            (
                "Direct3D 9, the system's".to_string(),
                Some("T".to_string())
            )
        );
        assert_eq!(renderer("", None).1, None);
    }

    /// A made-up player's whole run, on disk: the report says what happened and holds no trace
    /// of the user's name or folders, in any file it writes.
    #[test]
    fn a_report_is_built_scrubbed_and_kept() {
        let root = std::env::temp_dir().join(format!("nv-launcher-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("Profiles").join("Jo Tester");
        let program = root.join("Games").join("nv-rs");
        let own = program.join("userdata");
        let game = root.join("Steam").join("common").join("Fallout New Vegas");
        let ini = crate::profiles::profile_dir(&own, "mods");
        std::fs::create_dir_all(&ini).unwrap();
        std::fs::create_dir_all(own.join("logs")).unwrap();
        std::fs::create_dir_all(&game).unwrap();
        let home_text = home.display().to_string();
        std::fs::write(
            ini.join("Fallout.ini"),
            format!("[nv-rs General]\nsPreset=fixes\nsDir={home_text}\\x\nsName=JTest\n"),
        )
        .unwrap();
        let plugins = own
            .join("mods")
            .join("AppData")
            .join("Local")
            .join("FalloutNV");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(
            plugins.join("plugins.txt"),
            "FalloutNV.esm\njtest's patch.esp\n",
        )
        .unwrap();
        std::fs::write(
            program.join("BUILD.txt"),
            "nv-rs Windows preview 2026-11-16.1\nSource commit: 0123456789abcdef\n",
        )
        .unwrap();
        // An F12 report of this run (its picture is never read) between two of earlier runs (the
        // newer number too): only this run's goes in.
        let started = SystemTime::now() - Duration::from_secs(5);
        for (n, note) in [("001", "old one"), ("003", "older one"), ("002", "Write here what went wrong and what you did just before it:\r\n\r\nI was near jtest's house\r\n")] {
            let f = own.join("reports").join(n);
            std::fs::create_dir_all(&f).unwrap();
            std::fs::write(f.join("note.txt"), note).unwrap();
            std::fs::write(f.join("place.txt"), "Goodsprings 1 2 3\r\n").unwrap();
            std::fs::write(f.join("picture.bmp"), "BM").unwrap();
            if n != "002" {
                let note = std::fs::File::options().write(true).open(f.join("note.txt")).unwrap();
                note.set_modified(started - Duration::from_secs(3600)).unwrap();
            }
        }
        let log = own.join("logs").join("game-1.log");
        let mut text = String::from("NV_D3D9=wgpu: drawing with Some Card (Vulkan)\n");
        for n in 0..400 {
            text.push_str(&format!(
                "line {n} at {home_text}\\AppData\\x.tmp by JTEST on JT-DESK\n"
            ));
        }
        // A save's name (the character's name, place and time), and a user name at the line's cut.
        text.push_str("NV_SAVE_OUT: copied Save 7   Jo Hero  Goodsprings  01 02 03.fos\n");
        text.push_str(&format!("{} jtest-was-here\n", "x".repeat(LINE_CHARS - 3)));
        text.push_str(&format!(
            "Game's main stopped in frame 12: could not read {}\\Data\\x.esm (in 00a1b2c3 < 00400000)\n",
            game.display()
        ));
        std::fs::write(&log, &text).unwrap();
        let run = Run {
            status: Some(3),
            shell: Shell::Windows,
            log: &log,
            started,
            mods: true,
            own: &own,
            program: &program,
            games: vec![game.clone()],
            table_card: Some("Table Card".into()),
        };
        let mut known = Known::default();
        known.roots.push((game.display().to_string(), "<game>"));
        known.roots.push((program.display().to_string(), "<nv-rs>"));
        known.roots.push((home_text.clone(), "<home>"));
        known.words.push(("jtest".into(), "<user>"));
        known.words.push(("Jo Tester".into(), "<user>"));
        known.words.push(("JT-DESK".into(), "<computer>"));
        // A computer named like a word of the system's own name: the system line keeps it.
        known.words.push(("Workstation".into(), "<computer>"));
        let s = Scrubber::new(&known);
        let facts = system::Facts {
            os: "Some OS 1 (Workstation Edition)".into(),
            cpu: "Some CPU".into(),
            threads: Some(8),
            memory: Some(8 * 1024 * 1024 * 1024),
        };
        let (_, tail) = ends_of(&log, 1024 * 1024);
        let end = ending(&tail, Some(3), Shell::Windows).unwrap();
        let report = build(&run, &end, &facts, &s, SystemTime::UNIX_EPOCH);
        let folder = report.save(&own.join("reports")).unwrap();
        assert_eq!(folder.file_name().unwrap(), "crash-1970-01-01_00-00-00");
        let body = std::fs::read_to_string(folder.join("report.txt")).unwrap();
        let log_copy = std::fs::read_to_string(folder.join("log.txt")).unwrap();
        for (name, written) in [("report", &body), ("log", &log_copy)] {
            for secret in [
                "jtest",
                "JTEST",
                "Jo Tester",
                "JT-DESK",
                "Profiles",
                root.to_str().unwrap(),
            ] {
                assert!(!written.contains(secret), "{name} has {secret}:\n{written}");
            }
        }
        assert!(body.contains("Stop line:     Game's main stopped in frame 12: could not read <game>\\Data\\x.esm (in 00a1b2c3 < 00400000)"), "{body}");
        assert!(
            body.contains(
                "Grouped as:    stop | could not read <game>\\Data\\x.esm | 00a1b2c3 | 00400000"
            ),
            "{body}"
        );
        // The header names every placeholder and says how an F12 note is kept.
        for mark in [
            "<user>",
            "<computer>",
            "<home>",
            "<onedrive>",
            "<game>",
            "<nv-rs>",
            "<nv-rs-data>",
            "<path>",
            "<save>",
            "<id>",
            "<e-mail>",
            "<ip>",
        ] {
            assert!(body.contains(mark), "{mark} not in the header");
        }
        assert!(body.contains("An F12 note is kept as you wrote it"));
        assert!(body.contains("nv-rs:         nv-rs Windows preview 2026-11-16.1"));
        assert!(body.contains("Commit:        0123456789abcdef"));
        assert!(body.contains("Processor:     Some CPU (8 threads)"));
        assert!(body.contains("Memory:        8.0 GB"));
        assert!(body.contains("Graphics card: Some Card (Vulkan)"));
        assert!(body.contains("[nv-rs General] sPreset=fixes"));
        assert!(body.contains("[nv-rs General] sDir=(a path, left out)"));
        assert!(body.contains("[nv-rs General] sName=<user>"), "{body}");
        assert!(body.contains("  <user>'s patch.esp"), "{body}");
        assert!(body.contains("F12 report made during this run: reports/002"));
        assert!(body.contains("    I was near <user>'s house"), "{body}");
        assert!(!body.contains("Write here"), "{body}");
        assert!(body.contains("    Goodsprings 1 2 3"));
        assert!(
            !body.contains("older one") && !body.contains("old one"),
            "{body}"
        );
        assert!(
            body.contains("System:        Some OS 1 (Workstation Edition)"),
            "{body}"
        );
        // Only the report and the log are written: nothing of the F12 report's picture.
        let mut files: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(files, ["log.txt", "report.txt"]);
        assert!(
            log_copy.contains("NV_SAVE_OUT: copied <save>.fos"),
            "{log_copy}"
        );
        assert!(!log_copy.contains("Jo Hero"));
        // Scrubbed, then cut: no part of the name is left at the cut.
        assert!(!log_copy.contains(" jt"), "{log_copy}");
        assert!(log_copy.lines().all(|l| l.chars().count() <= LINE_CHARS));
        assert_eq!(log_copy.lines().count(), LOG_LINES);
        assert!(log_copy.contains("line 399 at <home>\\...\\x.tmp by <user> on <computer>"));
        // A second report the same second gets its own folder; the oldest go past the limit.
        let second = report.save(&own.join("reports")).unwrap();
        assert_eq!(second.file_name().unwrap(), "crash-1970-01-01_00-00-00-2");
        tidy(&own.join("reports"), 1);
        assert!(!folder.exists());
        assert!(second.exists());
        assert!(own.join("reports").join("001").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A good end makes no report; a stop makes one in nv-rs's own reports folder.
    #[test]
    fn after_a_run_only_a_bad_end_is_kept() {
        let root = std::env::temp_dir().join(format!("nv-launcher-after-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("logs")).unwrap();
        let log = root.join("logs").join("game.log");
        std::fs::write(&log, "a line of the game's\n").unwrap();
        let run = |status| Run {
            status: Some(status),
            shell: Shell::Unix,
            log: &log,
            started: SystemTime::now(),
            mods: false,
            own: &root,
            program: &root,
            games: Vec::new(),
            table_card: None,
        };
        assert!(after_run(&run(0)).is_none());
        let outcome = after_run(&run(139)).unwrap();
        // No crash service file beside the launcher (a build from source): nothing sent.
        assert_eq!(outcome.sent, None);
        let kept = outcome.kept.unwrap();
        assert!(kept.starts_with(root.join("reports")));
        let body = std::fs::read_to_string(kept.join("report.txt")).unwrap();
        assert!(body.contains("signal 11, SIGSEGV"), "{body}");
        assert!(body.contains("built from source"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// After a run, with the crash service's file beside the launcher and "Ask me each time":
    /// asked in the launcher's window, and with no window (a test has none) nothing is sent.
    /// (The file names a port on this PC nothing listens on, should anything try.)
    #[test]
    fn after_a_run_nothing_goes_without_the_players_yes() {
        let root =
            std::env::temp_dir().join(format!("nv-launcher-after-ask-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("logs")).unwrap();
        let log = root.join("logs").join("game.log");
        std::fs::write(&log, "a line of the game's\n").unwrap();
        std::fs::write(
            root.join(send::DSN_FILE),
            "http://0123456789abcdef0123456789abcdef@127.0.0.1:9/1\n",
        )
        .unwrap();
        consent::write(&root, consent::Consent::Ask).unwrap();
        let outcome = after_run(&Run {
            status: Some(139),
            shell: Shell::Unix,
            log: &log,
            started: SystemTime::now(),
            mods: false,
            own: &root,
            program: &root,
            games: Vec::new(),
            table_card: None,
        })
        .unwrap();
        let kept = outcome.kept.unwrap();
        assert_eq!(
            outcome.sent.as_deref(),
            Some("The crash report was not sent: it stays on this PC.")
        );
        assert!(!kept.join("envelope.txt").exists());
        assert!(!kept.join("sent.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
