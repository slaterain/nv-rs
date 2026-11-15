//! UI Organizer's file reader (`GetXMLFileData`, `ResolveXMLIncludes`, `ui_organizer.h` (UIO)):
//! the text `Tile::ReadFile` parses once UIO hooks it: a file's text (UIO's patched copy for the
//! files its lists target), its `<include>` tags replaced by the prefab files' texts, read the same
//! way.

use crate::hash::{begins_ci, str_hash};
use std::collections::HashMap;

/// The patched files, by UIO's hash of the path (`s_cachedBaseFiles`' entries that `UIOLoad`
/// made).
#[derive(Clone, Debug, Default)]
pub struct Patched {
    files: HashMap<u32, PatchedFile>,
}

#[derive(Clone, Debug)]
struct PatchedFile {
    /// The patched text; once read, with its includes expanded.
    text: Vec<u8>,
    resolved: bool,
    /// Read again from the file after its first read (see [`Patched::insert`]).
    once: bool,
    /// Already read once (for a `once` file).
    used: bool,
}

/// The menu files whose parse UIO does not keep (`UIOLoad`: `s_cachedParsedData` entries made
/// with `cached = false`): globals and the six menus the game keeps open.
const NOT_KEPT: [&[u8]; 7] = [
    b"Data\\Menus\\globals.xml",
    b"Data\\Menus\\Main\\hud_main_menu.xml",
    b"Data\\Menus\\Main\\inventory_menu.xml",
    b"Data\\Menus\\item_mod_menu.xml",
    b"Data\\Menus\\Main\\map_menu.xml",
    b"Data\\Menus\\repair_menu.xml",
    b"Data\\Menus\\Main\\stats_menu.xml",
];

impl Patched {
    /// `s_cachedBaseFiles.Emplace(path, prefab)` after a file is patched (an entry already there
    /// is kept, as `Emplace` keeps it). What later reads see, as UIO serves it: a patched prefab
    /// (`Data\menus\prefabs\...`, UIO's main cache) at every read; a patched menu at its first
    /// read through UIO's reader and at every later one through UIO's parse cache, except the
    /// seven menus whose parse UIO does not keep ([`NOT_KEPT`]), which a later read takes from the
    /// file again (UIO empties the entry after the first read).
    pub fn insert(&mut self, path: &[u8], text: Vec<u8>) {
        let prefab = begins_ci(path.get(11..).unwrap_or_default(), b"prefabs\\");
        let hash = str_hash(path);
        let once = !prefab && NOT_KEPT.iter().any(|m| str_hash(m) == hash);
        self.files.entry(hash).or_insert(PatchedFile {
            text,
            resolved: false,
            once,
            used: false,
        });
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// `data\menus\prefabs\` (`kPrefabsPath`, UIO): where an include's file is read from.
const PREFABS: &[u8] = b"data\\menus\\prefabs\\";

/// How deep includes are followed. UIO has no limit: a file that includes itself (or a loop of
/// them) runs it out of stack. Here such an include gives nothing (a deliberate difference).
pub const MAX_DEPTH: u32 = 64;
/// The most text a read makes (64 MiB): an include whose text would not fit in what is left gives
/// nothing. UIO has no limit and runs out of memory on files that include each other many times
/// over (a deliberate difference).
pub const MAX_TEXT: usize = 64 << 20;
/// The most files one read reads for its includes; past it an include gives nothing (files that
/// include each other many times over without text would otherwise be read without end).
pub const MAX_READS: u32 = 10_000;

/// The reader: the patched files and the game's own read of a file (`Menu::CompressAndStoreXML
/// (path, NULL)`, 00a1ce70: null when the file is missing).
pub struct Reader<'a> {
    pub patched: &'a mut Patched,
    pub raw: &'a mut dyn FnMut(&[u8]) -> Option<Vec<u8>>,
    /// Files read so far for this read's includes ([`MAX_READS`]).
    reads: u32,
}

impl<'a> Reader<'a> {
    pub fn new(patched: &'a mut Patched, raw: &'a mut dyn FnMut(&[u8]) -> Option<Vec<u8>>) -> Self {
        Reader {
            patched,
            raw,
            reads: 0,
        }
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: GetXMLFileData.
    /// A file's text as UIO's reader gives it: the patched text ([`Patched::insert`]) or the
    /// game's read, with its includes expanded; `None` when the file is missing (UIO answers an
    /// empty text there).
    pub fn file(&mut self, path: &[u8]) -> Option<Vec<u8>> {
        self.reads = 0;
        self.file_at(path, 0, MAX_TEXT)
    }

    /// [`Reader::file`] at an include `depth`, with `room` bytes left for its includes.
    fn file_at(&mut self, path: &[u8], depth: u32, room: usize) -> Option<Vec<u8>> {
        let hash = str_hash(path);
        if let Some(f) = self.patched.files.get_mut(&hash) {
            if !(f.once && f.used) {
                f.used = true;
                if !f.resolved {
                    let text = std::mem::take(&mut f.text);
                    let text = self.resolve(text, depth, room);
                    let f = self.patched.files.get_mut(&hash).expect("still there");
                    f.text = text;
                    f.resolved = true;
                }
                return self.patched.files.get(&hash).map(|f| f.text.clone());
            }
        }
        let text = (self.raw)(path)?;
        Some(self.resolve(text, depth, room))
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: ResolveXMLIncludes.
    /// `text` with each `<include` replaced as UIO does: the tag ends at the first `>` from its
    /// 21st byte on (so a short tag takes in the next tag); the path is between the first `"` from
    /// its 14th byte and the next `"`; a path under 5 bytes, or no quotes, drops the tag; else the
    /// tag is replaced by the text of `data\menus\prefabs\<path>` read by [`Reader::file`] (its
    /// includes expanded in turn; a missing file gives nothing). A tag with no `>` after it ends
    /// the expansion there (the rest is kept as it is). `room`: the bytes left for included text
    /// ([`MAX_TEXT`]).
    pub fn resolve(&mut self, text: Vec<u8>, depth: u32, room: usize) -> Vec<u8> {
        let mut d = text;
        d.push(0);
        let mut out: Vec<u8> = Vec::new();
        let mut found = false;
        let mut cur = 0usize;
        while let Some(tag) = find(&d, cur, b"<include") {
            // UIO searches for the `>` from 20 bytes in, also past the text's end; past the end
            // there is nothing here.
            let Some(close) = find_chr(&d, tag + 0x14, b'>') else {
                break;
            };
            found = true;
            out.extend_from_slice(&d[cur..tag]);
            cur = close + 1;
            d[close] = 0;
            let Some(q) = find_chr(&d, tag + 0xd, b'"') else {
                continue;
            };
            let start = q + 1;
            let Some(q2) = find_chr(&d, start, b'"') else {
                continue;
            };
            if q2 - start < 5 {
                continue;
            }
            let path = [PREFABS, &d[start..q2]].concat();
            if depth < MAX_DEPTH && out.len() < room && self.reads < MAX_READS {
                self.reads += 1;
                if let Some(t) = self.file_at(&path, depth + 1, room - out.len()) {
                    if out.len() + t.len() <= room {
                        out.extend_from_slice(&t);
                    }
                }
            }
        }
        if !found {
            d.pop();
            return d;
        }
        let rest = d[cur..]
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(d.len() - cur);
        out.extend_from_slice(&d[cur..cur + rest]);
        out
    }
}
/// `FindChr` (UIO) on a C string: the first `c` at or after `from`, stopping at a 0 (nothing from
/// past the end).
fn find_chr(d: &[u8], from: usize, c: u8) -> Option<usize> {
    let mut i = from;
    while i < d.len() && d[i] != 0 {
        if d[i] == c {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `strstr` on the C string at `from`.
fn find(d: &[u8], from: usize, pat: &[u8]) -> Option<usize> {
    let end = from + d.get(from..)?.iter().position(|&c| c == 0)?;
    d[from..end]
        .windows(pat.len())
        .position(|w| w == pat)
        .map(|p| from + p)
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
