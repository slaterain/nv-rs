//! nv-rs's own sections in a profile's `Fallout.ini` (host `game_ini.rs`; measure/ini/DESIGN.md).
//!
//! Play.exe puts the four `[nv-rs ...]` sections (`distribution/nv-rs-sections.ini`) into each
//! profile's `Fallout.ini` ([`with_sections`]): the whole block once, and in a file that has only
//! some of them the sections and keys it lacks. It moves a `settings.ini` into the files
//! ([`apply_choices`]); after that Settings.cmd edits the files in place. In a test run, which
//! never reads the file, it passes the file's choices to the game as variables ([`switches`]).
//! The text is handled one character per byte, so every byte outside the lines written comes back
//! as it was, line ends included.
use crate::settings::{ini_value, Settings};

/// The four sections, with a note above each key and the fix lines commented out.
pub const SECTIONS: &str = include_str!("../../../distribution/nv-rs-sections.ini");

/// The host's table of our keys: section, key, the variable it replaces, default, note.
const TABLE: &str = include_str!("../../../crates/host/data/game_ini.tsv");

/// Our sections' names: a file with any of them already has keys of ours.
const SECTION_NAMES: [&str; 4] = [
    "nv-rs General",
    "nv-rs Display",
    "nv-rs Threads",
    "nv-rs Fixes",
];

/// The section of the fix rows: each key turns one fix on or off, and each row's variable makes
/// the game's run a test.
const FIXES_SECTION: &str = "nv-rs Fixes";

/// A UTF-8 byte order mark, one character per byte as the text is held.
const MARK: &str = "\u{ef}\u{bb}\u{bf}";

/// The variables that make the game's run a test (host `fixes.rs`, `TEST_VARIABLES`; a host test
/// keeps the two lists alike). Each fix's own variable does too (the table's fix rows).
pub const TEST_SWITCHES: &[&str] = &[
    "NV_TRACE_LIST",
    "NV_MOVE_KEYS",
    "NV_MOVE_MOUSE",
    "NV_MOVE_REAL_INPUT",
    "NV_MOVE_TRACE",
    "NV_MOVE_PROBE",
    "NV_MOVE_TSV",
    "NV_MOVE_WATCH",
    "NV_MOVE_CALL_PROFILE",
    "NV_MOVE_CALL_BUDGET",
    "NV_MOVE_BUDGET_FROM",
    "NV_SOUND_RECORD",
    "NV_REPEATABLE",
    "NV_SHOT",
    "NV_FRAME_LOG",
    "NV_LOCK_STATS",
    "NV_REAL_DUMP",
    "NV_PHYS_BLEND_LOG",
    "NV_BENCH_CALLS",
    "NV_BENCH_CALL_COUNTS",
    "NV_PROFILE",
    "NV_WATCH",
    "NV_WORD_CHECK",
    "NV_SKIN_CHECK",
    "NV_QUEUE_CHECK",
    "NV_FACE_CHECK",
    "NV_SPEECH_CHECK",
    "NV_LIGHT_CHECK",
    "NV_HKMEM_CHECK",
    "NV_HEAD_CHECK",
    "NV_BLOCK_CHECK",
    "NV_CENSUS",
    "NV_TOUCH",
    "NV_PROFILE_THREAD_CALLS",
    "NV_AUDIO_CHECK",
    "NV_PASS_CHECK",
    "NV_TOPIC_INDEX_CHECK",
    "NV_NVSE_EVENT_LOG",
];

/// `(section, key, variable, default)` for each of our keys, in the table's order.
fn rows() -> impl Iterator<Item = (&'static str, &'static str, &'static str, &'static str)> {
    TABLE
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?, f.next()?, f.next()?, f.next()?))
        })
}

/// The section a line opens, if it is a `[...]` line.
fn section_of(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix('[')?;
    Some(rest.split(']').next().unwrap_or(rest).trim())
}

/// The line end a text uses: `\r\n` unless it has only bare `\n` ends.
fn line_end(text: &str) -> &'static str {
    if !text.contains("\r\n") && text.contains('\n') {
        "\n"
    } else {
        "\r\n"
    }
}

/// Whether `name` is one of our sections' names.
fn is_ours(name: &str) -> bool {
    SECTION_NAMES
        .iter()
        .any(|ours| ours.eq_ignore_ascii_case(name))
}

/// Whether `text` already has one of our sections.
pub fn has_sections(text: &str) -> bool {
    text.lines().filter_map(section_of).any(is_ours)
}

/// Whether the text is UTF-16: it starts with that mark, in either byte order.
pub fn is_utf16(text: &str) -> bool {
    text.starts_with("\u{ff}\u{fe}") || text.starts_with("\u{fe}\u{ff}")
}

/// One section of the block: its name and the lines under its heading (notes and keys).
struct BlockSection {
    name: &'static str,
    lines: Vec<&'static str>,
}

/// The block's sections, in order. The two note lines above the first heading belong to none.
fn block() -> Vec<BlockSection> {
    let mut sections: Vec<BlockSection> = Vec::new();
    for line in SECTIONS.lines() {
        if let Some(name) = section_of(line) {
            sections.push(BlockSection {
                name,
                lines: Vec::new(),
            });
        } else if let Some(section) = sections.last_mut().filter(|_| !line.trim().is_empty()) {
            section.lines.push(line);
        }
    }
    sections
}

/// The key a line of the block holds: `key=value`, or `;key=value` for a fix line. A note line
/// (`; ...`) holds none.
fn block_key(line: &str) -> Option<&str> {
    if line.starts_with("; ") {
        return None;
    }
    line.trim_start_matches(';')
        .split_once('=')
        .map(|(key, _)| key)
}

/// Whether `text` has `key` in a section named `section`, set or commented out (`;key=`): names
/// compared without case, every section of that name looked in, as the game finds a key.
fn has_key(text: &str, section: &str, key: &str) -> bool {
    let mut inside = false;
    text.lines().any(|line| {
        if let Some(name) = section_of(line) {
            inside = name.eq_ignore_ascii_case(section);
            return false;
        }
        inside
            && line.split_once('=').is_some_and(|(name, _)| {
                let name = name.trim();
                name.strip_prefix(';')
                    .map_or(name, str::trim)
                    .eq_ignore_ascii_case(key)
            })
    })
}

/// A `Fallout.ini` with all of our sections and keys in it ([`with_sections`]).
#[derive(Debug, PartialEq)]
pub struct Seeded {
    pub text: String,
    /// The file had none of our sections: the whole block went in.
    pub fresh: bool,
    /// Our keys added just now, by name: all of them when `fresh`.
    pub added: Vec<&'static str>,
}

/// `text` with our sections in it, in the file's own line ends, every byte already there as it
/// was. `None` for a UTF-16 file, which is left alone (an ASCII block in it helps nobody).
///
/// A file with none of our sections gets the whole block: at the top with an empty line after
/// it, or at the end when the file starts with a UTF-8 byte order mark. The game (and Windows'
/// `GetPrivateProfileStringA`, measured 2026-11-09) does not take a heading glued to that mark
/// for a heading, so the keys under the player's first heading are not read; a block after the
/// mark would free that heading and make them count. At the end, every key of the player's reads
/// as it did.
///
/// A file that has some of our sections gets what it lacks ([`complete`]); one that has
/// everything comes back unchanged.
pub fn with_sections(text: &str) -> Option<Seeded> {
    if is_utf16(text) {
        return None;
    }
    let eol = line_end(text);
    if has_sections(text) {
        let (text, added) = complete(text, eol);
        return Some(Seeded {
            text,
            fresh: false,
            added,
        });
    }
    let mut ours = String::with_capacity(SECTIONS.len() + 64);
    for line in SECTIONS.lines() {
        ours.push_str(line);
        ours.push_str(eol);
    }
    let text = if text.starts_with(MARK) {
        let mut out = String::with_capacity(text.len() + ours.len() + 4);
        out.push_str(text);
        if text.len() > MARK.len() {
            if !text.ends_with('\n') {
                out.push_str(eol);
            }
            out.push_str(eol);
        }
        out.push_str(&ours);
        out
    } else if text.is_empty() {
        ours
    } else {
        format!("{ours}{eol}{text}")
    };
    let added = block()
        .iter()
        .flat_map(|section| section.lines.iter().copied().filter_map(block_key))
        .collect();
    Some(Seeded {
        text,
        fresh: true,
        added,
    })
}

/// `text`, which has some of our sections, with the sections and keys of the block it lacks, so
/// no file stays half made. A missing key goes at the end of its section (the first of that
/// name), its note above it, as the block has it; a missing section goes after the last of our
/// sections in the file, an empty line before it. Also the keys added.
fn complete(text: &str, eol: &str) -> (String, Vec<&'static str>) {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let headings: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| section_of(line).map(|name| (at, name)))
        .collect();
    // The line after the last line of the section whose heading is line `at`: its empty lines
    // before the next heading stay behind what is added.
    let end_of = |at: usize| {
        let next = headings
            .iter()
            .map(|&(line, _)| line)
            .find(|&line| line > at)
            .unwrap_or(lines.len());
        (at + 1..=next)
            .rev()
            .find(|&line| !lines[line - 1].trim().is_empty())
            .unwrap_or(at + 1)
    };
    let mut inserts: Vec<(usize, String)> = Vec::new();
    let mut added = Vec::new();
    let mut missing = Vec::new();
    for section in block() {
        let heading = headings
            .iter()
            .find(|(_, name)| name.eq_ignore_ascii_case(section.name));
        let Some(&(at, _)) = heading else {
            missing.push(section);
            continue;
        };
        let mut new = String::new();
        for (index, line) in section.lines.iter().copied().enumerate() {
            let Some(key) = block_key(line).filter(|key| !has_key(text, section.name, key)) else {
                continue;
            };
            if let Some(note) = index.checked_sub(1).map(|above| section.lines[above]) {
                if block_key(note).is_none() {
                    new.push_str(note);
                    new.push_str(eol);
                }
            }
            new.push_str(line);
            new.push_str(eol);
            added.push(key);
        }
        if !new.is_empty() {
            inserts.push((end_of(at), new));
        }
    }
    let after_ours = headings
        .iter()
        .filter(|(_, name)| is_ours(name))
        .map(|&(at, _)| end_of(at))
        .max()
        .unwrap_or(lines.len());
    for section in missing {
        let mut new = format!("{eol}[{}]{eol}", section.name);
        for &line in &section.lines {
            new.push_str(line);
            new.push_str(eol);
            added.extend(block_key(line));
        }
        inserts.push((after_ours, new));
    }
    if inserts.is_empty() {
        return (text.to_string(), added);
    }
    let mut out = String::with_capacity(text.len() + SECTIONS.len());
    for at in 0..=lines.len() {
        for (_, new) in inserts.iter().filter(|(line, _)| *line == at) {
            // Only the file's last line can lack its end.
            if !out.is_empty() && !out.ends_with('\n') {
                out.push_str(eol);
            }
            out.push_str(new);
        }
        if let Some(line) = lines.get(at) {
            out.push_str(line);
        }
    }
    (out, added)
}

/// `text` with `key=value` in `section`: the first such key's value replaced where it stands
/// (names compared without case, as the game finds them); else a `;key=` line of that section
/// turned into the key; else the line added under the section's first heading; else the section
/// added at the end. Every other byte stays.
pub fn set_key(text: &str, section: &str, key: &str, value: &str) -> String {
    let mut inside = false;
    let mut under_heading = None;
    let mut commented = None;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let body = line.trim_end_matches(['\r', '\n']);
        if let Some(name) = section_of(body) {
            inside = name.eq_ignore_ascii_case(section);
            if inside && under_heading.is_none() {
                under_heading = Some(at);
            }
            continue;
        }
        let Some((name, _)) = body.split_once('=').filter(|_| inside) else {
            continue;
        };
        let name = name.trim();
        if name.eq_ignore_ascii_case(key) {
            let equals = start + body.find('=').unwrap_or(0);
            return format!(
                "{}{value}{}",
                &text[..equals + 1],
                &text[start + body.len()..]
            );
        }
        let off = name.strip_prefix(';').map(str::trim);
        if commented.is_none() && off.is_some_and(|n| n.eq_ignore_ascii_case(key)) {
            commented = Some((start, start + body.len()));
        }
    }
    let eol = line_end(text);
    if let Some((from, to)) = commented {
        return format!("{}{key}={value}{}", &text[..from], &text[to..]);
    }
    if let Some(at) = under_heading {
        let gap = if text[..at].ends_with('\n') { "" } else { eol };
        return format!("{}{gap}{key}={value}{eol}{}", &text[..at], &text[at..]);
    }
    let gap = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        eol
    };
    format!("{text}{gap}[{section}]{eol}{key}={value}{eol}")
}

/// `fallout_ini` (with our sections: [`with_sections`]) with the player's choices in our keys.
///
/// `settings` is a `settings.ini` being moved in, when there is one. Each line it has wins over
/// what the file says, the same rule Settings.cmd follows while that file exists; a key no line
/// names keeps the file's value. V-sync goes to `[Display] iPresentInterval`, where the original
/// reads it.
///
/// The borderless window is Play.exe's own default: with no `Borderless=` line, `bBorderless=1`
/// goes into a key that was added just now (`added`) or that is empty, and nowhere else.
///
/// Also the names of settings.ini's lines that are not used any more. `settings` must have
/// passed [`Settings::game_switches`].
pub fn apply_choices(
    fallout_ini: &str,
    settings: Option<&Settings>,
    added: &[&str],
) -> (String, Vec<String>) {
    let none = Settings::default();
    let settings = settings.unwrap_or(&none);
    let line = |name: &str| {
        settings
            .0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
            .filter(|v| !v.is_empty())
    };
    let switches = settings.game_switches().unwrap_or_default();
    let switch = |name: &str| {
        switches
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.clone())
    };
    let flag = |v: &str| match v {
        "1" => Some("1".to_string()),
        "0" => Some("0".to_string()),
        _ => None,
    };
    let mut text = fallout_ini.to_string();
    let mut used: Vec<&str> = Vec::new();
    for (section, key, variable, _) in rows() {
        let value = match variable {
            "NV_PRESET" => line("Preset").and(switch(variable)),
            "NV_FRAME_LIMIT" => line("FrameLimit").and(switch(variable)),
            "NV_MENU_FRAME_LIMIT" => line("MenuFrameLimit").and(switch(variable)),
            "NV_BORDERLESS" => settings
                .borderless()
                .map(|on| if on { "1" } else { "0" }.to_string()),
            // Settings.cmd writes `stepped` or `real`; any other word chooses nothing here.
            "NV_THREADS" => line(variable).and_then(|v| {
                if v.eq_ignore_ascii_case("stepped") {
                    Some("1".to_string())
                } else if v.eq_ignore_ascii_case("real") {
                    Some("0".to_string())
                } else {
                    None
                }
            }),
            _ => line(variable).and_then(flag),
        };
        if variable != "NV_BORDERLESS" && value.is_some() {
            used.push(variable);
        }
        match value {
            Some(value) => text = set_key(&text, section, key, &value),
            None if key == "bBorderless"
                && (added.contains(&key)
                    || ini_value(&text, section, key).is_none_or(str::is_empty)) =>
            {
                text = set_key(&text, section, key, "1");
            }
            None => {}
        }
    }
    match line("VSync") {
        Some("On") => text = set_key(&text, "Display", "iPresentInterval", "1"),
        Some("Off") => text = set_key(&text, "Display", "iPresentInterval", "0"),
        _ => {}
    }
    let unused = settings
        .0
        .keys()
        .filter(|k| k.starts_with("NV_") && !used.iter().any(|u| u.eq_ignore_ascii_case(k)))
        .cloned()
        .collect();
    (text, unused)
}

/// What Play.exe does with the game's variables for the player's options at one start.
#[derive(Debug, Default, PartialEq)]
pub struct Switches {
    /// The variable set before Play.exe started that makes the game's run a test, if one does.
    pub test: Option<&'static str>,
    /// Taken out of the game's environment: in play the file decides, so nothing may beat it.
    pub removed: Vec<&'static str>,
    /// Passed on with the file's choices: a test never reads the file.
    pub passed: Vec<(&'static str, String)>,
    /// Set before Play.exe started and left as they are, in a test.
    pub kept: Vec<&'static str>,
}

/// The game's variables for the player's options (host `fixes.rs` reads them over the file).
/// `inherited` says whether a variable is set in Play.exe's own environment.
///
/// In play, all six are taken out, so the profile's `Fallout.ini` decides. When a variable that
/// was set before makes the run a test ([`TEST_SWITCHES`], or a fix's own), the game ignores the
/// file and would run the original's behaviour with a test's defaults. So that a diagnostic run
/// plays like the player's game, the file's choices are passed on then: its value for each of
/// the six, or the value a play run has when the file does not say, and each fix line that is
/// not commented out. A variable set before the start stays as it was set: nothing is taken out.
pub fn switches(fallout_ini: &str, inherited: impl Fn(&str) -> bool) -> Switches {
    let test = TEST_SWITCHES
        .iter()
        .copied()
        .chain(rows().filter(|r| r.0 == FIXES_SECTION).map(|r| r.2))
        .find(|name| inherited(name));
    let mut out = Switches {
        test,
        ..Switches::default()
    };
    if test.is_none() {
        out.removed = rows()
            .filter(|r| r.0 != FIXES_SECTION)
            .map(|r| r.2)
            .collect();
        return out;
    }
    for (section, key, variable, default) in rows() {
        if inherited(variable) {
            if section != FIXES_SECTION {
                out.kept.push(variable);
            }
            continue;
        }
        let value = ini_value(fallout_ini, section, key).filter(|v| !v.is_empty());
        let flag = value.filter(|v| matches!(*v, "0" | "1"));
        let passed = match key {
            "sPreset" => Some(
                value
                    .filter(|v| {
                        v.eq_ignore_ascii_case("fixes") || v.eq_ignore_ascii_case("vanilla")
                    })
                    .map_or(default.to_string(), str::to_ascii_lowercase),
            ),
            "iFrameLimit" | "iMenuFrameLimit" => Some(
                value
                    .filter(|v| v.parse::<u32>().is_ok())
                    .unwrap_or(default)
                    .to_string(),
            ),
            // Play.exe's own default: borderless.
            "bBorderless" => Some(flag.unwrap_or("1").to_string()),
            "bSteppedThreads" => Some(
                if flag.unwrap_or(default) == "1" {
                    "stepped"
                } else {
                    "real"
                }
                .to_string(),
            ),
            "bRenderThread" => Some(flag.unwrap_or(default).to_string()),
            // A fix line: only one that is there and not commented out.
            _ => flag.map(str::to_string),
        };
        if let Some(value) = passed {
            out.passed.push((variable, value));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Made-up text in the shape of a game's INI file (not the game's own).
    const MADE_UP: &str = "[General]\r\nsTestName=Abc\r\niTestCount=3\r\n\r\n[Display]\r\nfTestSize=1.5000\r\niPresentInterval=1\r\n";

    /// The block in the line ends `eol`.
    fn ours(eol: &str) -> String {
        SECTIONS
            .lines()
            .map(|line| format!("{line}{eol}"))
            .collect()
    }

    /// Every key of the block, in its order.
    fn all_keys() -> Vec<&'static str> {
        rows().map(|row| row.1).collect()
    }

    #[test]
    fn the_sections_go_on_top_once_and_the_rest_stays() {
        let seeded = with_sections(MADE_UP).unwrap();
        assert!(seeded.fresh);
        assert_eq!(seeded.added, all_keys());
        let out = seeded.text;
        assert_eq!(out, format!("{}\r\n{MADE_UP}", ours("\r\n")));
        // CRLF throughout, the sections' lines too.
        assert!(!out.replace("\r\n", "").contains('\n'));
        assert_eq!(out.matches("[nv-rs General]").count(), 1);
        // A second time changes nothing.
        let again = with_sections(&out).unwrap();
        assert_eq!(
            again,
            Seeded {
                text: out,
                fresh: false,
                added: Vec::new()
            }
        );
        // A file with LF ends gets LF lines.
        let lf = MADE_UP.replace("\r\n", "\n");
        let out = with_sections(&lf).unwrap().text;
        assert_eq!(out, format!("{}\n{lf}", ours("\n")));
        // A file that has every key (any case, set or commented out) is left as it is.
        let whole = ours("\r\n").to_ascii_uppercase();
        assert_eq!(with_sections(&whole).unwrap().text, whole);
        // Nothing to put them before.
        assert_eq!(with_sections("").unwrap().text, ours("\r\n"));
    }

    /// A file that starts with a UTF-8 byte order mark: the game does not take the heading glued
    /// to the mark for a heading (Windows' reader measured 2026-11-09, measure/ini/DESIGN.md), so
    /// the block goes at the end and the player's bytes, the mark and that first line with them,
    /// stay where they were.
    #[test]
    fn a_file_with_a_byte_order_mark_gets_the_sections_at_its_end() {
        let marked = format!("{MARK}{MADE_UP}");
        let seeded = with_sections(&marked).unwrap();
        assert!(seeded.fresh);
        assert_eq!(seeded.text, format!("{marked}\r\n{}", ours("\r\n")));
        // The game's reader, and ours: the glued heading's keys are in no section, as before; the
        // next section's and our own are read.
        assert_eq!(ini_value(&seeded.text, "General", "sTestName"), None);
        assert_eq!(
            ini_value(&seeded.text, "Display", "fTestSize"),
            Some("1.5000")
        );
        assert_eq!(
            ini_value(&seeded.text, "nv-rs General", "sPreset"),
            Some("fixes")
        );
        assert_eq!(with_sections(&seeded.text).unwrap().text, seeded.text);
        // No line end at the end of the file, LF ends: one is added first.
        let open = format!("{MARK}[General]\nsTestName=Abc");
        assert_eq!(
            with_sections(&open).unwrap().text,
            format!("{open}\n\n{}", ours("\n"))
        );
        // Only the mark.
        assert_eq!(
            with_sections(MARK).unwrap().text,
            format!("{MARK}{}", ours("\r\n"))
        );
    }

    #[test]
    fn a_utf16_file_is_left_alone() {
        let little: String = "\u{ff}\u{fe}[\0G\0e\0n\0e\0r\0a\0l\0]\0\r\0\n\0".into();
        assert!(is_utf16(&little));
        assert_eq!(with_sections(&little), None);
        assert_eq!(with_sections("\u{fe}\u{ff}\0[\0G"), None);
        assert!(!is_utf16(MADE_UP) && !is_utf16(MARK));
    }

    /// A file with one section of ours made by hand, or one that lost lines: every section and
    /// key of the block it lacks is added, with its note; what is there stays byte for byte.
    #[test]
    fn a_file_with_some_of_the_sections_gets_the_rest() {
        // One hand-made section at the end of the file, one key in it.
        let hand = format!("{MADE_UP}\r\n[nv-rs Display]\r\niFrameLimit=60\r\n");
        let seeded = with_sections(&hand).unwrap();
        assert!(!seeded.fresh);
        let out = &seeded.text;
        assert!(out.starts_with(&hand), "what was there stays first");
        let block = ours("\r\n");
        let rest = &out[hand.len()..];
        // A key's two lines in the block: its note and the key.
        let with_note = |key: &str| {
            let at = block.find(&format!("\r\n{key}=")).unwrap() + 2;
            let note = block[..at - 2].rfind("\r\n").unwrap() + 2;
            let end = at + block[at..].find("\r\n").unwrap() + 2;
            block[note..end].to_string()
        };
        // The section's own missing keys under it, each with its note, then the other sections
        // as the block has them, an empty line before each.
        let display_rest = format!(
            "{}{}",
            with_note("bBorderless"),
            with_note("iMenuFrameLimit")
        );
        assert!(
            display_rest.starts_with("; 1 = the game's window")
                && display_rest.contains("\r\n; Most frames a second in menus")
        );
        let general =
            &block[block.find("[nv-rs General]").unwrap()..block.find("[nv-rs Display]").unwrap()];
        let threads_on = &block[block.find("[nv-rs Threads]").unwrap()..];
        assert_eq!(rest, format!("{display_rest}\r\n{general}{threads_on}"));
        let mut keys = seeded.added.clone();
        keys.push("iFrameLimit");
        keys.sort_unstable();
        let mut all = all_keys();
        all.sort_unstable();
        assert_eq!(keys, all, "each key of the block once");
        assert_eq!(ini_value(out, "nv-rs Display", "iFrameLimit"), Some("60"));
        for (section, key, _, default) in rows().filter(|r| r.0 != FIXES_SECTION) {
            let want = if key == "iFrameLimit" { "60" } else { default };
            assert_eq!(ini_value(out, section, key), Some(want), "{key}");
        }
        assert_eq!(ini_value(out, "nv-rs Fixes", "bUncap"), None, "commented");
        assert!(out.contains("\r\n;bUncap=1\r\n") && out.contains("; Each fix follows sPreset."));
        assert_eq!(
            with_sections(out).unwrap().text,
            *out,
            "nothing more to add"
        );

        // A section in the middle of the file, LF ends, a key commented out by the player and
        // the last line without its end: the key stays commented, the rest comes in.
        let middle = "[nv-rs Threads]\n;bSteppedThreads=1\n\n[General]\nx=1\n[NV-RS GENERAL]\nspreset=vanilla";
        let seeded = with_sections(middle).unwrap();
        let out = &seeded.text;
        assert!(out.starts_with("[nv-rs Threads]\n;bSteppedThreads=1\n; 1 = the Direct3D 9 calls go to a drawing thread of their own; 0 = on the game's threads, as the original makes them.\nbRenderThread=1\n\n[General]\nx=1\n[NV-RS GENERAL]\nspreset=vanilla\n\n[nv-rs Display]\n"), "{out}");
        assert!(!out.contains('\r'));
        assert!(!seeded.added.contains(&"bSteppedThreads") && !seeded.added.contains(&"sPreset"));
        assert_eq!(ini_value(out, "nv-rs General", "sPreset"), Some("vanilla"));
        assert_eq!(ini_value(out, "nv-rs Threads", "bSteppedThreads"), None);
        assert!(out.ends_with(";bNVSEListGuard=1\n"));
        assert_eq!(with_sections(out).unwrap().text, *out);
    }

    #[test]
    fn a_key_is_set_where_it_stands() {
        let ini = "[A]\r\nx=1\r\n[nv-rs Display]\r\n; note\r\n  bBorderless = 0 \r\n[B]\r\nbBorderless=7\r\n";
        assert_eq!(
            set_key(ini, "NV-RS DISPLAY", "bborderless", "1"),
            "[A]\r\nx=1\r\n[nv-rs Display]\r\n; note\r\n  bBorderless =1\r\n[B]\r\nbBorderless=7\r\n"
        );
        // A commented line of that section becomes the key.
        let ini = "[nv-rs Fixes]\n; Uncapped.\n;bUncap=1\n";
        assert_eq!(
            set_key(ini, "nv-rs Fixes", "bUncap", "0"),
            "[nv-rs Fixes]\n; Uncapped.\nbUncap=0\n"
        );
        // Missing key: under the heading; missing section: at the end, in the file's line ends.
        assert_eq!(
            set_key("[Display]\r\nfX=1\r\n", "Display", "iPresentInterval", "0"),
            "[Display]\r\niPresentInterval=0\r\nfX=1\r\n"
        );
        assert_eq!(
            set_key("[Display]", "Display", "iPresentInterval", "0"),
            "[Display]\r\niPresentInterval=0\r\n"
        );
        assert_eq!(
            set_key("[General]\nx=1", "Display", "iPresentInterval", "1"),
            "[General]\nx=1\n[Display]\niPresentInterval=1\n"
        );
    }

    #[test]
    fn settings_ini_choices_move_into_the_keys() {
        let seeded = with_sections(MADE_UP).unwrap();
        let fresh = seeded.text;
        let settings = Settings::parse(
            "Borderless=1\nPreset=vanilla\nFrameLimit=90\nMenuFrameLimit=\nVSync=Off\nNV_THREADS=stepped\nNV_RENDER_THREAD=0\nNV_UNCAP=0\nNV_TRACE_LIST=x\nNV_FIXES=vnv\nResolution=1920x1080\n",
        );
        let (out, unused) = apply_choices(&fresh, Some(&settings), &seeded.added);
        let get = |s: &str, k: &str| ini_value(&out, s, k);
        assert_eq!(get("nv-rs General", "sPreset"), Some("vanilla"));
        assert_eq!(get("nv-rs Display", "bBorderless"), Some("1"));
        assert_eq!(get("nv-rs Display", "iFrameLimit"), Some("90"));
        assert_eq!(get("nv-rs Display", "iMenuFrameLimit"), Some("120"));
        assert_eq!(get("nv-rs Threads", "bSteppedThreads"), Some("1"));
        assert_eq!(get("nv-rs Threads", "bRenderThread"), Some("0"));
        assert_eq!(get("nv-rs Fixes", "bUncap"), Some("0"));
        assert_eq!(
            get("nv-rs Fixes", "bMenuCap"),
            None,
            "other fixes stay commented"
        );
        assert_eq!(get("Display", "iPresentInterval"), Some("0"));
        assert_eq!(unused, ["NV_FIXES", "NV_TRACE_LIST"]);
        // Only values changed: the same lines, the original part byte for byte.
        assert_eq!(out.lines().count(), fresh.lines().count());
        assert!(out.ends_with("[Display]\r\nfTestSize=1.5000\r\niPresentInterval=0\r\n"));
        // No settings.ini: Play.exe's defaults, which differ from the sections' only in borderless.
        let (out, unused) = apply_choices(&fresh, None, &seeded.added);
        assert_ne!(out, fresh);
        assert_eq!(out, fresh.replace("bBorderless=0", "bBorderless=1"));
        assert!(unused.is_empty());
        // What Settings.cmd writes for the threads side by side and the drawing thread on.
        let side = Settings::parse("NV_THREADS=real\nNV_RENDER_THREAD=1\n");
        let stepped = "[nv-rs Threads]\r\nbSteppedThreads=1\r\nbRenderThread=0\r\n";
        let (out, unused) = apply_choices(stepped, Some(&side), &[]);
        assert!(out.starts_with("[nv-rs Threads]\r\nbSteppedThreads=0\r\nbRenderThread=1\r\n"));
        assert!(unused.is_empty());
        // Another word chooses nothing and is named.
        let (out, unused) =
            apply_choices(stepped, Some(&Settings::parse("NV_THREADS=slice\n")), &[]);
        assert!(out.starts_with(stepped));
        assert_eq!(unused, ["NV_THREADS"]);
    }

    /// While a settings.ini exists it wins, line by line, over a file that already has the
    /// sections: the rule Settings.cmd follows too. What it does not name stays the file's.
    #[test]
    fn settings_ini_wins_over_a_file_that_has_the_sections() {
        let had = "[nv-rs General]\r\nsPreset=fixes\r\n[nv-rs Display]\r\nbBorderless=0\r\niFrameLimit=30\r\niMenuFrameLimit=45\r\n[nv-rs Threads]\r\nbSteppedThreads=1\r\n";
        let settings =
            Settings::parse("Borderless=1\nPreset=vanilla\nFrameLimit=90\nNV_RENDER_THREAD=0\n");
        let (out, _) = apply_choices(had, Some(&settings), &[]);
        assert_eq!(
            out,
            "[nv-rs General]\r\nsPreset=vanilla\r\n[nv-rs Display]\r\nbBorderless=1\r\niFrameLimit=90\r\niMenuFrameLimit=45\r\n[nv-rs Threads]\r\nbRenderThread=0\r\nbSteppedThreads=1\r\n"
        );
        // No Borderless line: the player's 0 stays (Play.exe's default is only for a key it adds
        // or an empty one); with no settings.ini at all nothing changes.
        let (out, _) = apply_choices(had, Some(&Settings::parse("FrameLimit=90\n")), &[]);
        assert_eq!(out, had.replace("iFrameLimit=30", "iFrameLimit=90"));
        assert_eq!(apply_choices(had, None, &[]).0, had);
        let empty = "[nv-rs Display]\r\nbBorderless=\r\n";
        assert_eq!(
            apply_choices(empty, None, &[]).0,
            "[nv-rs Display]\r\nbBorderless=1\r\n"
        );
        let added = "[nv-rs Display]\r\nbBorderless=0\r\n";
        assert_eq!(
            apply_choices(added, None, &["bBorderless"]).0,
            "[nv-rs Display]\r\nbBorderless=1\r\n"
        );
    }

    /// In play Play.exe takes the six variables out; in a test, which never reads the file, it
    /// passes the file's choices on and leaves what was set before as it is.
    #[test]
    fn the_games_variables_follow_the_file_only_in_a_test() {
        let ini = "[nv-rs General]\nsPreset=vanilla\n[nv-rs Display]\nbBorderless=0\niFrameLimit=90\niMenuFrameLimit=bad\n[nv-rs Threads]\nbSteppedThreads=1\n[nv-rs Fixes]\n;bUncap=1\nbGrabPush=0\nbTopicIndex=2\n";
        let six = [
            "NV_PRESET",
            "NV_BORDERLESS",
            "NV_FRAME_LIMIT",
            "NV_MENU_FRAME_LIMIT",
            "NV_THREADS",
            "NV_RENDER_THREAD",
        ];
        // Play: nothing passed, all six out, whatever else is set.
        let sets: [&[&str]; 3] = [
            &[],
            &["NV_PRESET", "NV_THREADS"],
            &["NV_SAVE_OUT", "NV_D3D9"],
        ];
        for set in sets {
            let play = switches(ini, |name| set.contains(&name));
            assert_eq!(
                play,
                Switches {
                    removed: six.to_vec(),
                    ..Switches::default()
                },
                "{set:?}"
            );
        }
        // A test by a starred variable: the file's values, the play defaults where it has none
        // or one that can't be used, its uncommented fix line; nothing taken out.
        let pairs = |list: &[(&'static str, &str)]| -> Vec<(&'static str, String)> {
            list.iter().map(|(n, v)| (*n, v.to_string())).collect()
        };
        let test = switches(ini, |name| name == "NV_FRAME_LOG");
        assert_eq!(test.test, Some("NV_FRAME_LOG"));
        assert!(test.removed.is_empty() && test.kept.is_empty());
        assert_eq!(
            test.passed,
            pairs(&[
                ("NV_PRESET", "vanilla"),
                ("NV_BORDERLESS", "0"),
                ("NV_FRAME_LIMIT", "90"),
                ("NV_MENU_FRAME_LIMIT", "120"),
                ("NV_THREADS", "stepped"),
                ("NV_RENDER_THREAD", "1"),
                ("NV_GRAB_PUSH", "0"),
            ])
        );
        // A file that says nothing: a play run's values, so the test plays like the player's game.
        let plain = switches("[General]\nx=1\n", |name| name == "NV_SHOT");
        assert_eq!(
            plain.passed,
            pairs(&[
                ("NV_PRESET", "fixes"),
                ("NV_BORDERLESS", "1"),
                ("NV_FRAME_LIMIT", "0"),
                ("NV_MENU_FRAME_LIMIT", "120"),
                ("NV_THREADS", "real"),
                ("NV_RENDER_THREAD", "1"),
            ])
        );
        // A fix's own variable makes a test too; what was set before the start is kept, not
        // passed and not taken out.
        let set = ["NV_UNCAP", "NV_PRESET", "NV_THREADS", "NV_GRAB_PUSH"];
        let pinned = switches(ini, |name| set.contains(&name));
        assert_eq!(pinned.test, Some("NV_UNCAP"));
        assert_eq!(pinned.kept, ["NV_PRESET", "NV_THREADS"]);
        assert!(pinned.removed.is_empty());
        assert_eq!(
            pinned.passed,
            pairs(&[
                ("NV_BORDERLESS", "0"),
                ("NV_FRAME_LIMIT", "90"),
                ("NV_MENU_FRAME_LIMIT", "120"),
                ("NV_RENDER_THREAD", "1"),
            ])
        );
        // Every starred name and every fix's variable is one.
        for name in TEST_SWITCHES
            .iter()
            .copied()
            .chain(rows().filter(|r| r.0 == FIXES_SECTION).map(|r| r.2))
        {
            assert_eq!(switches(ini, |n| n == name).test, Some(name));
        }
    }
}
