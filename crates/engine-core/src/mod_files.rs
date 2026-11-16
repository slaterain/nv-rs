//! Mods' own files, as the built-in script extenders reach them (MCM's `GetModINI`/`SetModINI`,
//! later JIP LN's INI and file commands): the seam between those crates, which do not depend on
//! the platform layer, and the platform layer's `mod_files`, which keeps a mod's writes in a
//! folder of the port's own beside the saves (the maintainer, 2026-11-16) and reads them back
//! before the game's own Data.
//!
//! The platform installs its calls ([`install`]) with the layer; without it (an extender crate's
//! own tests, a program without the platform) a read finds nothing and a write writes nothing
//! ([`ERROR_NOT_SUPPORTED`]). The profile text rules are here too, as plain functions every side
//! can use: [`ini_value`] (`GetPrivateProfileStringA`'s reading) and [`written_profile`]
//! (`WritePrivateProfileStringA`'s writing), both measured on scratch files.
//!
//! A name is given as the game passes it to Windows: relative to the game's folder
//! (`Data\Config\MCM.ini`), or a full path inside it. The game's own profile imports
//! (`GetPrivateProfileStringA` and the others the exe calls) do not come here: only the
//! extenders' commands look in the port's folder first.
//!
//! Locking: the calls reach the platform layer's state (`Kernel::platform`), which with real
//! threads other threads change under the kernel lock; each wrapper here holds it
//! ([`crate::real_threads::kernel_guard`]) for the call. The calls make no game call.

use crate::Engine;

/// Windows' error codes the calls answer (winerror.h).
pub const ERROR_FILE_NOT_FOUND: u32 = 2;
pub const ERROR_PATH_NOT_FOUND: u32 = 3;
pub const ERROR_ACCESS_DENIED: u32 = 5;
pub const ERROR_NOT_SUPPORTED: u32 = 50;
pub const ERROR_ALREADY_EXISTS: u32 = 183;

/// The platform layer's calls ([`install`]). Errors are Windows' codes for what the same call
/// would answer there.
pub struct ModFileCalls {
    /// The file's bytes: written in this run, else the port's folder's copy, else the game's.
    /// `ERROR_FILE_NOT_FOUND` (or `_PATH_NOT_FOUND`) when there is none; any other error when a
    /// copy is there but cannot be read (a later place is not looked at then);
    /// `ERROR_ACCESS_DENIED` for a name outside the game's folder or a device, which is never
    /// read.
    pub read: fn(&Engine, &[u8]) -> Result<Vec<u8>, u32>,
    /// Whether the file exists in one of [`ModFileCalls::read`]'s places (not a folder), from
    /// the file system's attributes: its bytes are not read. False for a name `read` refuses.
    pub exists: fn(&Engine, &[u8]) -> bool,
    /// Whether the folder exists (made in this run, in the port's folder, or the game's); one
    /// trailing separator is allowed, as Windows allows it. False for a name `read` refuses.
    pub directory_exists: fn(&Engine, &[u8]) -> bool,
    /// `CreateDirectoryA` for a mod's folder (one trailing separator allowed):
    /// `ERROR_ALREADY_EXISTS` when it exists, `ERROR_PATH_NOT_FOUND` when its parent exists
    /// nowhere, `ERROR_ACCESS_DENIED` for a name outside the game's folder.
    pub create_directory: fn(&mut Engine, &[u8]) -> Result<(), u32>,
    /// The file written whole (the port's folder and the run): `ERROR_PATH_NOT_FOUND` when its
    /// folder exists nowhere, `ERROR_ACCESS_DENIED` for a folder, a name outside the game's
    /// folder or a port folder that refused the write.
    pub write: fn(&mut Engine, &[u8], &[u8]) -> Result<(), u32>,
}

static CALLS: std::sync::OnceLock<&'static ModFileCalls> = std::sync::OnceLock::new();

/// Installs the platform layer's calls (the first call wins: one layer a program).
pub fn install(calls: &'static ModFileCalls) {
    let _ = CALLS.set(calls);
}

/// The installed calls, `None` without the platform layer.
fn calls() -> Option<&'static ModFileCalls> {
    CALLS.get().copied()
}

/// [`ModFileCalls::read`] under the kernel lock; `ERROR_NOT_SUPPORTED` without the platform.
pub fn read_or_error(e: &Engine, name: &[u8]) -> Result<Vec<u8>, u32> {
    let _kernel = crate::real_threads::kernel_guard(e);
    calls().map_or(Err(ERROR_NOT_SUPPORTED), |c| (c.read)(e, name))
}

/// [`read_or_error`]'s bytes; `None` for any error.
pub fn read(e: &Engine, name: &[u8]) -> Option<Vec<u8>> {
    read_or_error(e, name).ok()
}

/// [`ModFileCalls::exists`] under the kernel lock; false without the platform layer.
pub fn exists(e: &Engine, name: &[u8]) -> bool {
    let _kernel = crate::real_threads::kernel_guard(e);
    calls().is_some_and(|c| (c.exists)(e, name))
}

/// [`ModFileCalls::directory_exists`] under the kernel lock; false without the platform layer.
pub fn directory_exists(e: &Engine, name: &[u8]) -> bool {
    let _kernel = crate::real_threads::kernel_guard(e);
    calls().is_some_and(|c| (c.directory_exists)(e, name))
}

/// [`ModFileCalls::create_directory`] under the kernel lock.
pub fn create_directory_or_error(e: &mut Engine, name: &[u8]) -> Result<(), u32> {
    let _kernel = crate::real_threads::kernel_guard(e);
    calls().map_or(Err(ERROR_NOT_SUPPORTED), |c| (c.create_directory)(e, name))
}

/// [`create_directory_or_error`] as `CreateDirectoryA`'s answer (true: made).
pub fn create_directory(e: &mut Engine, name: &[u8]) -> bool {
    create_directory_or_error(e, name).is_ok()
}

/// [`ModFileCalls::write`] under the kernel lock.
pub fn write_or_error(e: &mut Engine, name: &[u8], bytes: &[u8]) -> Result<(), u32> {
    let _kernel = crate::real_threads::kernel_guard(e);
    calls().map_or(Err(ERROR_NOT_SUPPORTED), |c| (c.write)(e, name, bytes))
}

/// [`write_or_error`]: true when written.
pub fn write(e: &mut Engine, name: &[u8], bytes: &[u8]) -> bool {
    write_or_error(e, name, bytes).is_ok()
}

/// `GetPrivateProfileStringA(section, key, ..., name)` over [`read`]: the value as Windows reads
/// it ([`ini_value`]), `None` when the file, the section or the key is not there (or the file
/// cannot be read, when Windows also answers the default).
pub fn profile_string(e: &Engine, name: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    ini_value(&read(e, name)?, section, key)
}

/// `WritePrivateProfileStringA(section, key, value, name)` over [`read_or_error`] and
/// [`write_or_error`]: the file as Windows rewrites it ([`written_profile`]); a missing file
/// starts empty. A file that is there but cannot be read is not written (so a kept file is never
/// replaced by one made from nothing), with one line in the log; the error comes back, as it
/// does when the folder exists nowhere or the write fails.
pub fn write_profile_string(
    e: &mut Engine,
    name: &[u8],
    section: &[u8],
    key: &[u8],
    value: &[u8],
) -> Result<(), u32> {
    let old = match read_or_error(e, name) {
        Ok(bytes) => bytes,
        Err(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) => Vec::new(),
        Err(code) => {
            println!(
                "mod files: {} could not be read (Windows error {code}); not written, so it stays as it is",
                String::from_utf8_lossy(name)
            );
            return Err(code);
        }
    };
    let new = written_profile(&old, section, key, value);
    write_or_error(e, name, &new)
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
/// spaces around names and values trimmed, one pair of the same surrounding quotes, single or
/// double, taken off), as
/// `GetPrivateProfileStringA` finds it: only in the first section of that name, as Windows reads
/// a file with a repeated section name (measured on scratch files, 2026-11-09; the platform's
/// `ini.rs` tests hold the answers). A comment after a value is part of it (`350\t;note` reads
/// `350\t;note`, measured 2026-11-16). A line ends at LF or at CR, alone or together
/// (`[A]\rx=1\ry=2\r` reads y as 2: `jip-cmds/files-measure/ini4-2026-11-16.txt`, `STRING cr
/// only`; `ini5-...`, `SECTION cr cr lf` and `lf cr`).
pub fn ini_value(text: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    let section = trim_spaces(section);
    let mut inside = false;
    for line in text.split(|&b| b == b'\n' || b == b'\r') {
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
            // One pair of the same quote, single or double, taken off (measured 2026-11-16,
            // `ini3-2026-11-16.txt`): `"1"` and `'1'` read 1, `""` reads nothing, a lone `"` or
            // `"1'` stays, `"a"b"` reads `a"b`, `"1" ;c` stays whole.
            let value = match value {
                [q @ (b'"' | b'\x27'), middle @ .., last] if q == last => middle,
                _ => value,
            };
            return Some(value.to_vec());
        }
    }
    None
}

/// What Windows ends every line it writes with, and a last line that had none (not blank).
const CRLF: &[u8] = b"\r\n";

/// A byte a write trims off the end of the file before it splits it into lines (measured: a
/// space, a tab, a backspace; a CR is kept). So a last line with no end loses its trailing
/// blanks, and is dropped when nothing else is left.
fn blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | 0x08)
}

/// `text` as Windows' profile writer splits it into lines: each line's bytes and the bytes that
/// ended it. An LF ends a line (a CR right before it stays in the line, so a CR LF file is
/// written back as it was); a CR followed by any other byte ends a line too; a CR at the very
/// end does not (`ini-measure/ini-cr-2026-11-16.txt`: `[A]\rx=1\ry=2\r` with x=5 is
/// `[A]\rx=5\ry=2\r\r\n`; `jip-cmds/files-measure/ini4-2026-11-16.txt` and `ini5-...`, the same
/// lines through Windows' other profile writes; `x=1\r\r\ny=2` and `\n\ry=2` keep the extra line).
fn profile_lines(text: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, &b) in text.iter().enumerate() {
        let ends = match b {
            b'\n' => true,
            b'\r' => text.get(i + 1).is_some_and(|&next| next != b'\n'),
            _ => false,
        };
        if ends {
            lines.push((&text[start..i], &text[i..=i]));
            start = i + 1;
        }
    }
    if start < text.len() {
        lines.push((&text[start..], &[][..]));
    }
    lines
}

/// The text of an INI file after `WritePrivateProfileStringA(section, key, value)`, as Windows
/// writes it (measured on scratch files, 2026-11-16, `build43-scratch/measure/mcm/ini-measure`,
/// `ini-2026-11-16.txt` and `ini2-2026-11-16.txt`):
/// - lines end at LF, or at a CR not followed by LF ([`profile_lines`]); each line the file had
///   keeps its bytes and its end (an LF-only or CR-only file stays so); spaces, tabs and
///   backspaces at the very end of the file are dropped first, and a last line with no end then
///   gets CR LF (a new file MCM made as one backspace byte becomes the section alone; `x=1  `
///   becomes `x=1` CR LF; `x=1\r ` becomes `x=1\r` CR LF: `ini-measure/ini-cr-2026-11-16.txt`,
///   and `jip-cmds/files-measure/ini5-2026-11-16.txt`, the `trailing spaces no eol` cases);
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
    // Blanks at the very end go before the text is split (`x=1\r ` is `x=1\r`, whose CR then
    // ends no line).
    let kept = old.iter().rposition(|&b| !blank(b)).map_or(0, |p| p + 1);
    let lines = profile_lines(&old[..kept]);
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
    let push = |out: &mut Vec<u8>, line: &[u8], ended: &[u8]| {
        out.extend_from_slice(line);
        out.extend_from_slice(if ended.is_empty() { CRLF } else { ended });
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

#[cfg(test)]
#[path = "mod_files_tests.rs"]
mod tests;
