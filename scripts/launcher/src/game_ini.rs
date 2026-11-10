//! nv-rs's own sections in a profile's `Fallout.ini` (host `game_ini.rs`; measure/ini/DESIGN.md).
//!
//! Play.exe puts the four `[nv-rs ...]` sections (`distribution/nv-rs-sections.ini`) at the top of
//! each profile's `Fallout.ini` once, and moves a `settings.ini` from before into the files once
//! ([`apply_choices`]); after that Settings.cmd edits the files in place. The text is handled one
//! character per byte, so every byte outside the lines written comes back as it was, line ends
//! included.
use crate::settings::{ini_value, Settings};

/// The four sections, with a note above each key and the fix lines commented out.
pub const SECTIONS: &str = include_str!("../../../distribution/nv-rs-sections.ini");

/// The host's table of our keys: section, key, the variable it replaces, default, note.
const TABLE: &str = include_str!("../../../crates/host/data/game_ini.tsv");

/// Our sections' names: a file with any of them already has our keys.
const SECTION_NAMES: [&str; 4] = [
    "nv-rs General",
    "nv-rs Display",
    "nv-rs Threads",
    "nv-rs Fixes",
];

/// `(section, key, variable)` for each of our keys, in the table's order.
fn rows() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    TABLE
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?, f.next()?, f.next()?))
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

/// Whether `text` already has one of our sections.
pub fn has_sections(text: &str) -> bool {
    text.lines().filter_map(section_of).any(|name| {
        SECTION_NAMES
            .iter()
            .any(|ours| ours.eq_ignore_ascii_case(name))
    })
}

/// `text` with our sections at the top and an empty line after them (after a UTF-8 byte order
/// mark, if the file starts with one), in the file's own line ends; the rest byte for byte as it
/// was. A text that has them already comes back unchanged.
pub fn with_sections(text: &str) -> String {
    if has_sections(text) {
        return text.to_string();
    }
    let eol = line_end(text);
    let (mark, rest) = match text.strip_prefix("\u{ef}\u{bb}\u{bf}") {
        Some(rest) => ("\u{ef}\u{bb}\u{bf}", rest),
        None => ("", text),
    };
    let mut out = String::with_capacity(SECTIONS.len() + text.len() + 8);
    out.push_str(mark);
    for line in SECTIONS.lines() {
        out.push_str(line);
        out.push_str(eol);
    }
    if !rest.is_empty() {
        out.push_str(eol);
        out.push_str(rest);
    }
    out
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

/// `fallout_ini` with the player's choices from `settings` (a `settings.ini` from before, or an
/// empty one: Play.exe's own defaults) in our keys, and V-sync in `[Display] iPresentInterval`,
/// where the original reads it. `fresh`: our sections were added just now, so their values are only
/// the defaults and every choice goes in; otherwise a key the file already sets keeps its value
/// (the file wins, as it did over settings.ini). Also the names of settings.ini's lines that
/// are not used any more. `settings` must have passed [`Settings::game_switches`].
pub fn apply_choices(fallout_ini: &str, settings: &Settings, fresh: bool) -> (String, Vec<String>) {
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
    for (section, key, variable) in rows() {
        let value = match variable {
            "NV_PRESET" => line("Preset").and(switch(variable)),
            "NV_FRAME_LIMIT" => line("FrameLimit").and(switch(variable)),
            "NV_MENU_FRAME_LIMIT" => line("MenuFrameLimit").and(switch(variable)),
            "NV_BORDERLESS" => Some(if settings.borderless() { "1" } else { "0" }.to_string()),
            "NV_THREADS" => line(variable)
                .filter(|v| v.eq_ignore_ascii_case("stepped"))
                .map(|_| "1".to_string()),
            _ => line(variable).and_then(flag),
        };
        if variable != "NV_BORDERLESS" && value.is_some() {
            used.push(variable);
        }
        let Some(value) = value else {
            continue;
        };
        if fresh || ini_value(&text, section, key).is_none_or(str::is_empty) {
            text = set_key(&text, section, key, &value);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Made-up text in the shape of a game's INI file (not the game's own).
    const MADE_UP: &str = "[General]\r\nsTestName=Abc\r\niTestCount=3\r\n\r\n[Display]\r\nfTestSize=1.5000\r\niPresentInterval=1\r\n";

    #[test]
    fn the_sections_go_on_top_once_and_the_rest_stays() {
        let out = with_sections(MADE_UP);
        assert!(out.starts_with("; nv-rs's own options."));
        assert!(out.ends_with(&format!("\r\n\r\n{MADE_UP}")));
        assert_eq!(&out[out.len() - MADE_UP.len()..], MADE_UP);
        // CRLF throughout, the sections' lines too.
        assert!(!out.replace("\r\n", "").contains('\n'));
        assert_eq!(out.matches("[nv-rs General]").count(), 1);
        assert_eq!(with_sections(&out), out, "a second time changes nothing");
        // A file with LF ends gets LF lines; one with a byte order mark keeps it first.
        let lf = MADE_UP.replace("\r\n", "\n");
        let out = with_sections(&lf);
        assert!(!out.contains('\r') && out.ends_with(&format!("\n\n{lf}")));
        let marked = format!("\u{ef}\u{bb}\u{bf}{MADE_UP}");
        let out = with_sections(&marked);
        assert!(out.starts_with("\u{ef}\u{bb}\u{bf}; nv-rs's own options."));
        assert!(out.ends_with(&format!("\r\n\r\n{MADE_UP}")));
        // A file that names one of the sections (any case) already has them.
        let own = "[General]\r\n[ NV-RS display ]\r\nbBorderless=1\r\n";
        assert_eq!(with_sections(own), own);
        // Nothing to put them before.
        assert_eq!(
            with_sections(""),
            SECTIONS.replace("\r\n", "\n").replace('\n', "\r\n")
        );
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
        let fresh = with_sections(MADE_UP);
        let settings = Settings::parse(
            "Borderless=1\nPreset=vanilla\nFrameLimit=90\nMenuFrameLimit=\nVSync=Off\nNV_THREADS=stepped\nNV_RENDER_THREAD=0\nNV_UNCAP=0\nNV_TRACE_LIST=x\nNV_FIXES=vnv\nResolution=1920x1080\n",
        );
        let (out, unused) = apply_choices(&fresh, &settings, true);
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
        let (out, unused) = apply_choices(&fresh, &Settings::default(), true);
        assert_eq!(out, fresh.replace("bBorderless=0", "bBorderless=1"));
        assert!(unused.is_empty());
        // Sections the file had before: what it sets stays, what it lacks comes from settings.ini.
        let had = "[nv-rs General]\r\nsPreset=fixes\r\n[nv-rs Display]\r\nbBorderless=0\r\n";
        let (out, _) = apply_choices(had, &settings, false);
        assert_eq!(ini_value(&out, "nv-rs General", "sPreset"), Some("fixes"));
        assert_eq!(ini_value(&out, "nv-rs Display", "bBorderless"), Some("0"));
        assert_eq!(ini_value(&out, "nv-rs Display", "iFrameLimit"), Some("90"));
        assert!(out.starts_with(
            "[nv-rs General]\r\nsPreset=fixes\r\n[nv-rs Display]\r\niFrameLimit=90\r\nbBorderless=0\r\n"
        ));
    }
}
