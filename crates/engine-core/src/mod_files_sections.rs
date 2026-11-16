//! The profile calls past one key (JIP LN's INI commands): a section's lines
//! (`GetPrivateProfileSectionA`), the section names (`GetPrivateProfileSectionNamesA`), a key
//! removed (`WritePrivateProfileStringA` with no value), a section removed
//! (`WritePrivateProfileStructA`/`WritePrivateProfileStringA` with no key) and a section written
//! whole (`WritePrivateProfileSectionA`), as Windows does them (measured on scratch files,
//! 2026-11-16, `build43-scratch/measure/jip-cmds/files-measure`, `ini3-`, `ini4-` and
//! `ini5-2026-11-16.txt`; the tests hold every case). The text rules are plain functions; the
//! calls on a mod's file go through [`super::read_or_error`] and [`super::write_or_error`] (under
//! the kernel lock), so they keep its name rules, its places and Windows' error codes.
//!
//! What Windows does, as these calls see a file:
//! - A line ends at CR LF, a lone CR or a lone LF (each one line end: `x=1\r\r\n` is the line
//!   `x=1` and a blank line).
//! - A line's first character after spaces and tabs makes it a heading (`[`: its name is the
//!   text up to the first `]` or the line's end, without surrounding spaces), a comment (`;`), a
//!   blank line (none), else an entry (with or without `=`; `#` starts no comment).
//! - Before a change Windows trims spaces, tabs and backspaces off the end of the file, then adds
//!   CR LF when it does not end with LF; it writes the file back when it changed it or added that
//!   line end (a trimmed blank last line alone leaves the file as it was).
//! - Only the first section of a name counts.
//! - A section that is the file's last and whose last line is blank reaches the end of the file;
//!   any other ends at its last entry's line end (for a section written whole: its last entry
//!   with `=`), or at its heading's when it has none.

use super::{
    read, read_or_error, trim_spaces, write_or_error, Engine, ERROR_FILE_NOT_FOUND,
    ERROR_PATH_NOT_FOUND,
};

/// One line: where it starts, where its text ends (before the line end) and where the next
/// starts.
#[derive(Clone, Copy, Debug)]
struct Line {
    start: usize,
    text_end: usize,
    end: usize,
}

/// What a line is (its first character after spaces and tabs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Blank,
    Heading,
    Comment,
    Entry,
}

/// A space or a tab: what Windows skips at a line's start.
fn blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

/// The lines of `text`, each line end CR LF, CR or LF.
fn lines(text: &[u8]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let start = at;
        while at < text.len() && text[at] != b'\r' && text[at] != b'\n' {
            at += 1;
        }
        let text_end = at;
        if at < text.len() {
            at += if text[at] == b'\r' && text.get(at + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
        }
        out.push(Line {
            start,
            text_end,
            end: at,
        });
    }
    out
}

/// Where a line's text starts after spaces and tabs.
fn first(text: &[u8], l: Line) -> usize {
    (l.start..l.text_end)
        .find(|&i| !blank(text[i]))
        .unwrap_or(l.text_end)
}

fn kind(text: &[u8], l: Line) -> Kind {
    let at = first(text, l);
    if at >= l.text_end {
        return Kind::Blank;
    }
    match text[at] {
        b'[' => Kind::Heading,
        b';' => Kind::Comment,
        _ => Kind::Entry,
    }
}

/// A heading's name: after `[`, up to the first `]` or the line's end, trimmed.
fn heading_name(text: &[u8], l: Line) -> &[u8] {
    let rest = &text[first(text, l) + 1..l.text_end];
    let name = rest.split(|&b| b == b']').next().unwrap_or(rest);
    trim_spaces(name)
}

/// The first section named `section` (without case or surrounding spaces): its heading's index
/// in `ls` and the index past its last line.
fn find_section(text: &[u8], ls: &[Line], section: &[u8]) -> Option<(usize, usize)> {
    let section = trim_spaces(section);
    let at = ls.iter().position(|&l| {
        kind(text, l) == Kind::Heading && heading_name(text, l).eq_ignore_ascii_case(section)
    })?;
    let end = ls[at + 1..]
        .iter()
        .position(|&l| kind(text, l) == Kind::Heading)
        .map_or(ls.len(), |n| at + 1 + n);
    Some((at, end))
}

/// The file as Windows holds it before a change: spaces, tabs and backspaces off its end, then
/// CR LF added when it does not end with LF (the flag: added, so the file is written back).
fn loaded(old: &[u8]) -> (Vec<u8>, bool) {
    let keep = old
        .iter()
        .rposition(|&b| !matches!(b, b' ' | b'\t' | 0x08))
        .map_or(0, |i| i + 1);
    let mut text = old[..keep].to_vec();
    let added = !text.is_empty() && text.last() != Some(&b'\n');
    if added {
        text.extend_from_slice(b"\r\n");
    }
    (text, added)
}

/// Where a section's lines end for a change: the end of the file when it is the last section and
/// the file's last line is blank; else the end of its last line `counts`, else its heading's.
fn section_end(
    text: &[u8],
    ls: &[Line],
    (at, end): (usize, usize),
    counts: impl Fn(&[u8], Line) -> bool,
) -> usize {
    if end == ls.len() && ls.last().is_some_and(|&l| kind(text, l) == Kind::Blank) {
        return text.len();
    }
    ls[at + 1..end]
        .iter()
        .rev()
        .find(|&&l| counts(text, l))
        .map_or(ls[at].end, |l| l.end)
}

/// An entry with `=`.
fn key_line(text: &[u8], l: Line) -> bool {
    kind(text, l) == Kind::Entry && text[l.start..l.text_end].contains(&b'=')
}

/// The lines of section `section` of `text` as `GetPrivateProfileSectionA` gives them: each entry
/// in order, `key=value` with the spaces and tabs around both taken off (quotes kept), a line
/// without `=` trimmed; `None` when there is no such section.
pub fn section_lines(text: &[u8], section: &[u8]) -> Option<Vec<Vec<u8>>> {
    let ls = lines(text);
    let (at, end) = find_section(text, &ls, section)?;
    Some(
        ls[at + 1..end]
            .iter()
            .filter(|&&l| kind(text, l) == Kind::Entry)
            .map(|&l| {
                let line = &text[l.start..l.text_end];
                match line.iter().position(|&b| b == b'=') {
                    Some(eq) => {
                        [trim_spaces(&line[..eq]), b"=", trim_spaces(&line[eq + 1..])].concat()
                    }
                    None => trim_spaces(line).to_vec(),
                }
            })
            .collect(),
    )
}

/// Every heading's name in `text` as `GetPrivateProfileSectionNamesA` gives them: in order,
/// repeats and empty names (`[]`) included.
pub fn section_names(text: &[u8]) -> Vec<Vec<u8>> {
    let ls = lines(text);
    ls.iter()
        .filter(|&&l| kind(text, l) == Kind::Heading)
        .map(|&l| heading_name(text, l).to_vec())
        .collect()
}

/// A list of texts as Windows fills a buffer of `size` bytes with it (the section and section
/// name calls): each text and a 0, then one more 0; the count is the bytes before that last 0.
/// When that does not fit with a byte to spare (a list of `n` bytes needs `n + 2`), the first
/// `size - 2` bytes and two 0s, counted `size - 2` (one 0 for a buffer of one byte, nothing for
/// none).
pub fn profile_list(items: &[Vec<u8>], size: usize) -> (Vec<u8>, usize) {
    let mut all: Vec<u8> = Vec::new();
    for item in items {
        all.extend_from_slice(item);
        all.push(0);
    }
    if size >= all.len() + 2 {
        let count = all.len();
        all.push(0);
        return (all, count);
    }
    match size {
        0 => (Vec::new(), 0),
        1 => (vec![0], 0),
        _ => {
            let mut out = all[..size - 2].to_vec();
            out.extend_from_slice(&[0, 0]);
            (out, size - 2)
        }
    }
}

/// `text` after `WritePrivateProfileStringA(section, key, NULL)`: the first entry `key=...` of the
/// section (keys compared as reads do) taken out from its first character after spaces and tabs
/// through its line end (the spaces before it stay). `None`: Windows leaves the file as it was
/// (no such key, or an empty key name, and nothing to add at the end).
pub fn removed_key(old: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    let (text, added) = loaded(old);
    let key = trim_spaces(key);
    let ls = lines(&text);
    let found = (!key.is_empty())
        .then(|| find_section(&text, &ls, section))
        .flatten()
        .and_then(|(at, end)| {
            ls[at + 1..end].iter().copied().find(|&l| {
                let line = &text[l.start..l.text_end];
                key_line(&text, l)
                    && line
                        .iter()
                        .position(|&b| b == b'=')
                        .is_some_and(|eq| trim_spaces(&line[..eq]).eq_ignore_ascii_case(key))
            })
        });
    match found {
        Some(l) => Some([&text[..first(&text, l)], &text[l.end..]].concat()),
        None => added.then_some(text),
    }
}

/// `text` after `WritePrivateProfileStructA(section, NULL, ...)` (and
/// `WritePrivateProfileStringA(section, NULL, NULL)`): the section taken out from its heading's
/// `[` through its last entry's line end (comments and blank lines after it stay, unless the
/// section reaches the end of the file). `None`: Windows leaves the file as it was.
pub fn removed_section(old: &[u8], section: &[u8]) -> Option<Vec<u8>> {
    let (text, added) = loaded(old);
    let ls = lines(&text);
    match find_section(&text, &ls, section) {
        Some(found) => {
            let from = first(&text, ls[found.0]);
            let to = section_end(&text, &ls, found, |t, l| kind(t, l) == Kind::Entry);
            Some([&text[..from], &text[to..]].concat())
        }
        None => added.then_some(text),
    }
}

/// `text` after `WritePrivateProfileSectionA(section, lines)`: the lines after the section's
/// heading through its last entry with `=` (entries without `=` after it, comments and blank
/// lines stay) replaced by `lines`, each as given and CR LF; a section not there goes at the end,
/// `[section]` CR LF and the lines.
pub fn written_section(old: &[u8], section: &[u8], new_lines: &[Vec<u8>]) -> Vec<u8> {
    let (text, _) = loaded(old);
    let ls = lines(&text);
    let mut block = Vec::new();
    for line in new_lines {
        block.extend_from_slice(line);
        block.extend_from_slice(b"\r\n");
    }
    match find_section(&text, &ls, section) {
        Some(found) => {
            let from = ls[found.0].end;
            let to = section_end(&text, &ls, found, key_line);
            [&text[..from], &block[..], &text[to..]].concat()
        }
        None => [&text[..], b"[", section, b"]\r\n", &block[..]].concat(),
    }
}

/// The file's bytes before a change, as [`super::write_profile_string`] takes them: `None` when
/// it is not there (`ERROR_FILE_NOT_FOUND` or `_PATH_NOT_FOUND`: Windows makes it, and the
/// write then answers when its folder exists nowhere); any other error comes back, with one line
/// in the log, and nothing is written (a kept file is never replaced by one made from nothing).
fn before_change(e: &Engine, name: &[u8]) -> Result<Option<Vec<u8>>, u32> {
    match read_or_error(e, name) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) => Ok(None),
        Err(code) => {
            println!(
                "mod files: {} could not be read (Windows error {code}); not written, so it stays as it is",
                String::from_utf8_lossy(name)
            );
            Err(code)
        }
    }
}

/// `GetPrivateProfileSectionA(section, ..., name)` over [`read`]: [`section_lines`]; `None` when
/// the file or the section is not there.
pub fn profile_section(e: &Engine, name: &[u8], section: &[u8]) -> Option<Vec<Vec<u8>>> {
    section_lines(&read(e, name)?, section)
}

/// `GetPrivateProfileSectionNamesA(..., name)` over [`read`]: [`section_names`]; `None` when the
/// file is not there.
pub fn profile_section_names(e: &Engine, name: &[u8]) -> Option<Vec<Vec<u8>>> {
    Some(section_names(&read(e, name)?))
}

/// `WritePrivateProfileStringA(section, key, NULL, name)` over [`read_or_error`] and
/// [`write_or_error`] ([`removed_key`]). Ok, as Windows answers, also when the key is not there;
/// a file not there is made empty (measured). The error, writing nothing, when the file is there
/// but cannot be read, its folder exists nowhere, or the write is refused.
pub fn remove_profile_key(
    e: &mut Engine,
    name: &[u8],
    section: &[u8],
    key: &[u8],
) -> Result<(), u32> {
    match before_change(e, name)? {
        None => write_or_error(e, name, b""),
        Some(old) => match removed_key(&old, section, key) {
            Some(new) => write_or_error(e, name, &new),
            None => Ok(()),
        },
    }
}

/// `WritePrivateProfileStructA(section, NULL, NULL, 0, name)` over [`read_or_error`] and
/// [`write_or_error`] ([`removed_section`]): as [`remove_profile_key`] answers.
pub fn remove_profile_section(e: &mut Engine, name: &[u8], section: &[u8]) -> Result<(), u32> {
    match before_change(e, name)? {
        None => write_or_error(e, name, b""),
        Some(old) => match removed_section(&old, section) {
            Some(new) => write_or_error(e, name, &new),
            None => Ok(()),
        },
    }
}

/// `WritePrivateProfileSectionA(section, lines, name)` over [`read_or_error`] and
/// [`write_or_error`] ([`written_section`]; a file not there is made). The error, writing
/// nothing, as [`remove_profile_key`].
pub fn write_profile_section(
    e: &mut Engine,
    name: &[u8],
    section: &[u8],
    new_lines: &[Vec<u8>],
) -> Result<(), u32> {
    let old = before_change(e, name)?.unwrap_or_default();
    let new = written_section(&old, section, new_lines);
    write_or_error(e, name, &new)
}

#[cfg(test)]
#[path = "mod_files_sections_tests.rs"]
mod tests;
