//! Mods' own files, as the built-in script extenders reach them (MCM's `GetModINI`/`SetModINI`,
//! later JIP LN's INI and file commands): the seam between those crates, which do not depend on
//! the platform layer, and the platform layer's `mod_files`, which keeps a mod's writes in a
//! folder of the port's own beside the saves (the maintainer, 2026-11-16) and reads them back
//! before the game's own Data.
//!
//! The platform installs its calls ([`install`]) with the layer; without it (an extender crate's
//! own tests, a program without the platform) a read finds nothing and a write writes nothing.
//! The profile text rules are here too, as plain functions every side can use: [`ini_value`]
//! (`GetPrivateProfileStringA`'s reading) and [`written_profile`] (`WritePrivateProfileStringA`'s
//! writing, measured on scratch files).
//!
//! A name is given as the game passes it to Windows: relative to the game's folder
//! (`Data\Config\MCM.ini`), or a full path inside it.

use crate::Engine;

/// The platform layer's calls ([`install`]).
pub struct ModFileCalls {
    /// The file's bytes: written in this run, else the port's folder's copy, else the game's.
    pub read: fn(&Engine, &[u8]) -> Option<Vec<u8>>,
    /// Whether the file exists in one of [`ModFileCalls::read`]'s places.
    pub exists: fn(&Engine, &[u8]) -> bool,
    /// Whether the folder exists (made in this run, in the port's folder, or the game's).
    pub directory_exists: fn(&Engine, &[u8]) -> bool,
    /// `CreateDirectoryA` for a mod's folder; false for a name outside the game's folder.
    pub create_directory: fn(&mut Engine, &[u8]) -> bool,
    /// The file written whole (the run and the port's folder); false: nothing written.
    pub write: fn(&mut Engine, &[u8], &[u8]) -> bool,
}

static CALLS: std::sync::OnceLock<&'static ModFileCalls> = std::sync::OnceLock::new();

/// Installs the platform layer's calls (the first call wins: one layer a program).
pub fn install(calls: &'static ModFileCalls) {
    let _ = CALLS.set(calls);
}

/// [`ModFileCalls::read`]; `None` without the platform layer.
pub fn read(e: &Engine, name: &[u8]) -> Option<Vec<u8>> {
    CALLS.get().and_then(|c| (c.read)(e, name))
}

/// [`ModFileCalls::exists`]; false without the platform layer.
pub fn exists(e: &Engine, name: &[u8]) -> bool {
    CALLS.get().is_some_and(|c| (c.exists)(e, name))
}

/// [`ModFileCalls::directory_exists`]; false without the platform layer.
pub fn directory_exists(e: &Engine, name: &[u8]) -> bool {
    CALLS.get().is_some_and(|c| (c.directory_exists)(e, name))
}

/// [`ModFileCalls::create_directory`]; false without the platform layer.
pub fn create_directory(e: &mut Engine, name: &[u8]) -> bool {
    CALLS.get().is_some_and(|c| (c.create_directory)(e, name))
}

/// [`ModFileCalls::write`]; false without the platform layer.
pub fn write(e: &mut Engine, name: &[u8], bytes: &[u8]) -> bool {
    CALLS.get().is_some_and(|c| (c.write)(e, name, bytes))
}

/// `GetPrivateProfileStringA(section, key, ..., name)` over [`read`]: the value as Windows reads
/// it ([`ini_value`]), `None` when the file, the section or the key is not there.
pub fn profile_string(e: &Engine, name: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    ini_value(&read(e, name)?, section, key)
}

/// `WritePrivateProfileStringA(section, key, value, name)` over [`read`] and [`write`]: the
/// file as Windows rewrites it ([`written_profile`]). False, writing nothing, when the file's
/// folder exists nowhere ([`directory_exists`]; Windows answers `ERROR_PATH_NOT_FOUND`), the
/// name is outside the game's folder, or there is no platform layer.
pub fn write_profile_string(
    e: &mut Engine,
    name: &[u8],
    section: &[u8],
    key: &[u8],
    value: &[u8],
) -> bool {
    let folder_end = name.iter().rposition(|&b| b == b'\\' || b == b'/');
    if let Some(at) = folder_end {
        if !directory_exists(e, &name[..at]) {
            return false;
        }
    }
    let old = read(e, name).unwrap_or_default();
    let new = written_profile(&old, section, key, value);
    write(e, name, &new)
}

/// ASCII white space taken off both ends.
pub fn trim_spaces(s: &[u8]) -> &[u8] {
    let start = s
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

/// The value of `key` in section `section` of the INI text `text` (names compared without case,
/// spaces around names and values trimmed, one pair of surrounding quotes taken off), as
/// `GetPrivateProfileStringA` finds it: only in the first section of that name, as Windows reads
/// a file with a repeated section name (measured on scratch files, 2026-11-09; the platform's
/// `ini.rs` tests hold the answers). A comment after a value is part of it (`350\t;note` reads
/// `350\t;note`, measured 2026-11-16).
pub fn ini_value(text: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    let section = trim_spaces(section);
    let mut inside = false;
    for line in text.split(|&b| b == b'\n') {
        let line = trim_spaces(line);
        if let Some(rest) = line.strip_prefix(b"[") {
            // The first section of the name ends here: Windows reads no later one.
            if inside {
                return None;
            }
            let name = rest.split(|&b| b == b']').next().unwrap_or(rest);
            inside = trim_spaces(name).eq_ignore_ascii_case(section);
            continue;
        }
        if !inside {
            continue;
        }
        let Some(at) = line.iter().position(|&b| b == b'=') else {
            continue;
        };
        if trim_spaces(&line[..at]).eq_ignore_ascii_case(key) {
            let value = trim_spaces(&line[at + 1..]);
            let value = match value {
                [b'"', middle @ .., b'"'] => middle,
                _ => value,
            };
            return Some(value.to_vec());
        }
    }
    None
}

/// What Windows ends every line it writes with, and a last line that had none (not blank).
const CRLF: &[u8] = b"\r\n";

/// A byte of a blank last line (one with no line end): a write drops such a line (measured: a
/// space, a tab, a backspace; a lone CR is kept).
fn blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | 0x08)
}

/// The text of an INI file after `WritePrivateProfileStringA(section, key, value)`, as Windows
/// writes it (measured on scratch files, 2026-11-16, `build43-scratch/measure/mcm/ini-measure`,
/// `ini-2026-11-16.txt` and `ini2-2026-11-16.txt`):
/// - lines end at LF; each line the file had keeps its bytes and its end (an LF-only file stays
///   so); a last line with no end gets CR LF, unless it is blank (spaces, tabs, backspaces), when
///   it is dropped (a new file MCM made as one backspace byte becomes the section alone);
/// - the section is the first heading (`[name]`, the text up to the first `]`) whose name matches
///   without case or surrounding spaces; only that one is looked in;
/// - in it, the first line `key=...` whose key matches the same way (lines starting with `;` are
///   not keys) keeps everything up to `=`, then the new value, then the white space (spaces, tabs,
///   CR) that ended the old value; the old value's leading blanks and a comment after it go;
/// - a new key goes right after the section's last key line (after its heading when it has none),
///   before comments, other lines and blank lines, as `key=value` CR LF;
/// - a new section goes at the end, `[section]` CR LF `key=value` CR LF, with no blank line;
/// - names and the value are written as given.
pub fn written_profile(old: &[u8], section: &[u8], key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut lines: Vec<(&[u8], bool)> = Vec::new();
    let mut rest = old;
    while !rest.is_empty() {
        match rest.iter().position(|&b| b == b'\n') {
            Some(at) => {
                lines.push((&rest[..at], true));
                rest = &rest[at + 1..];
            }
            None => {
                lines.push((rest, false));
                rest = &[];
            }
        }
    }
    if let Some(&(last, false)) = lines.last() {
        if last.iter().all(|&b| blank(b)) {
            lines.pop();
        }
    }
    let (section_name, key_name) = (trim_spaces(section), trim_spaces(key));
    let mut heading: Option<usize> = None;
    let mut last_key: Option<usize> = None;
    let mut replace: Option<(usize, usize)> = None;
    for (i, &(line, _)) in lines.iter().enumerate() {
        let t = trim_spaces(line);
        if let Some(name) = t.strip_prefix(b"[") {
            if heading.is_some() {
                break;
            }
            let name = name.split(|&b| b == b']').next().unwrap_or(name);
            if trim_spaces(name).eq_ignore_ascii_case(section_name) {
                heading = Some(i);
            }
            continue;
        }
        if heading.is_none() || t.first() == Some(&b';') {
            continue;
        }
        if let Some(eq) = line.iter().position(|&b| b == b'=') {
            if trim_spaces(&line[..eq]).eq_ignore_ascii_case(key_name) {
                replace = Some((i, eq));
                break;
            }
            last_key = Some(i);
        }
    }
    let new_line: Vec<u8> = [key, b"=", value].concat();
    let mut out = Vec::with_capacity(old.len() + new_line.len() + section.len() + 8);
    let push = |out: &mut Vec<u8>, line: &[u8], ended: bool| {
        out.extend_from_slice(line);
        out.extend_from_slice(if ended { b"\n" } else { CRLF });
    };
    for (i, &(line, ended)) in lines.iter().enumerate() {
        match replace {
            Some((at, eq)) if at == i => {
                let old_value = &line[eq + 1..];
                let end = old_value
                    .iter()
                    .rposition(|&b| !matches!(b, b' ' | b'\t' | b'\r'))
                    .map_or(0, |p| p + 1);
                let mut changed = line[..eq + 1].to_vec();
                changed.extend_from_slice(value);
                changed.extend_from_slice(&old_value[end..]);
                push(&mut out, &changed, ended);
            }
            _ => push(&mut out, line, ended),
        }
        if replace.is_none() && heading.is_some() && last_key.or(heading) == Some(i) {
            out.extend_from_slice(&new_line);
            out.extend_from_slice(CRLF);
        }
    }
    if heading.is_none() {
        out.push(b'[');
        out.extend_from_slice(section);
        out.push(b']');
        out.extend_from_slice(CRLF);
        out.extend_from_slice(&new_line);
        out.extend_from_slice(CRLF);
    }
    out
}

#[path = "mod_files_sections.rs"]
mod sections;
pub use sections::{
    profile_list, profile_section, profile_section_names, remove_profile_key,
    remove_profile_section, removed_key, removed_section, section_lines, section_names,
    write_profile_section, written_section,
};

#[cfg(test)]
#[path = "mod_files_tests.rs"]
mod tests;
