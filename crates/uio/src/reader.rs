//! UI Organizer's file reader (`GetXMLFileData`, `ResolveXMLIncludes`, `ui_organizer.h` (UIO)):
//! the text `Tile::ReadFile` parses once UIO hooks it: a file's text (UIO's patched copy for the
//! files its lists target), its `<include>` tags replaced by the prefab files' texts, read the same
//! way.

use crate::hash::{begins_ci, str_hash};
use std::collections::HashMap;

/// The patched files, by UIO's hash of the path (`s_cachedBaseFiles`' entries that `UIOLoad`
/// made).
#[derive(Clone, Debug)]
pub struct Patched {
    files: HashMap<u32, PatchedFile>,
    /// The bytes of the texts held ([`Patched::keep`] keeps no more than `limit`).
    held: usize,
    limit: usize,
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

/// The most bytes of patched texts kept with their includes expanded (64 MiB). Past it an
/// expanded text is not kept: the file keeps its patched text and is expanded again at each read
/// (the same text; UIO keeps every expanded text, with no limit).
pub const MAX_HELD: usize = 64 << 20;

impl Default for Patched {
    fn default() -> Self {
        Patched::with_limit(MAX_HELD)
    }
}

/// A patched file as one read is served it ([`Patched::serve`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Served {
    /// Its includes already expanded.
    Expanded(Vec<u8>),
    /// The patched text, its includes still to expand.
    Unexpanded(Vec<u8>),
}

impl Patched {
    /// No patched file, keeping at most `limit` bytes of texts ([`MAX_HELD`]).
    pub fn with_limit(limit: usize) -> Self {
        Patched {
            files: HashMap::new(),
            held: 0,
            limit,
        }
    }

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
        if let std::collections::hash_map::Entry::Vacant(v) = self.files.entry(hash) {
            self.held += text.len();
            v.insert(PatchedFile {
                text,
                resolved: false,
                once,
                used: false,
            });
        }
    }

    /// The patched file of `hash` for one read: a copy of its text (the file of a `once` entry
    /// handed over, at its first read only); `None` when there is none to serve. The caller holds
    /// no part of the map while it reads on (another thread may read the same files meanwhile).
    pub fn serve(&mut self, hash: u32) -> Option<Served> {
        let f = self.files.get_mut(&hash)?;
        if f.once && f.used {
            return None;
        }
        f.used = true;
        if f.once {
            let text = std::mem::take(&mut f.text);
            self.held -= text.len();
            return Some(Served::Unexpanded(text));
        }
        Some(if f.resolved {
            Served::Expanded(f.text.clone())
        } else {
            Served::Unexpanded(f.text.clone())
        })
    }

    /// Keeps `text` as the file's text with its includes expanded, for later reads: not for a
    /// `once` file (read once), not when it is expanded already (another read was first), and not
    /// past [`MAX_HELD`] (the file keeps its patched text).
    pub fn keep(&mut self, hash: u32, text: Vec<u8>) {
        let Some(f) = self.files.get_mut(&hash) else {
            return;
        };
        if f.once || f.resolved {
            return;
        }
        let held = self.held - f.text.len() + text.len();
        if held > self.limit {
            return;
        }
        self.held = held;
        f.text = text;
        f.resolved = true;
    }

    /// Whether the file of `path` is kept with its includes expanded.
    pub fn expanded(&self, path: &[u8]) -> bool {
        self.files.get(&str_hash(path)).is_some_and(|f| f.resolved)
    }

    /// The bytes of texts held.
    pub fn held(&self) -> usize {
        self.held
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

/// The longest path UIO's path buffers hold with their ending 0 (`kPrefabsPath[0x80]` for an
/// include, `prefabsPath[0x80]` and `menusPath[0x80]` for a list's paths and `#file`): a longer
/// one overruns them in UIO. Here such a path names no file (a deliberate difference).
pub const UIO_PATH: usize = 0x7f;

/// How deep includes are followed. UIO has no limit: a file that includes itself (or a loop of
/// them) runs it out of stack. Here such an include gives nothing (a deliberate difference).
pub const MAX_DEPTH: u32 = 64;
/// The most text a read makes (64 MiB): an include whose text would not fit in what is left gives
/// nothing, and the read fetches no further include. UIO has no limit and runs out of memory on
/// files that include each other many times over (a deliberate difference).
pub const MAX_TEXT: usize = 64 << 20;
/// The most files one read reads for its includes; past it an include gives nothing (files that
/// include each other many times over without text would otherwise be read without end; a
/// deliberate difference).
pub const MAX_READS: u32 = 10_000;

/// The game's read of a file ([`Source::raw`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fetch {
    /// No such file.
    Missing,
    /// Longer than the room asked for (not copied).
    TooLarge,
    Text(Vec<u8>),
}

/// What the reader reads through. Each call is whole: nothing of the patched files is held
/// between them.
pub trait Source {
    /// The patched file of `hash` for this read ([`Patched::serve`]).
    fn serve(&mut self, hash: u32) -> Option<Served>;
    /// The patched file's expanded text for later reads ([`Patched::keep`]).
    fn keep(&mut self, hash: u32, text: Vec<u8>);
    /// The game's own read of a file (`Menu::CompressAndStoreXML(path, NULL)`, 00a1ce70: null
    /// when the file is missing), when its text fits in `room` bytes.
    fn raw(&mut self, path: &[u8], room: usize) -> Fetch;
}

/// The reader for one read of a file.
pub struct Reader<'a> {
    source: &'a mut dyn Source,
    /// Files read so far for this read's includes ([`MAX_READS`]).
    reads: u32,
    /// A limit refused an include in the expansion under way (its text is then not kept).
    capped: bool,
    /// The read's text or read limit was reached: no further include is fetched.
    exhausted: bool,
}

impl<'a> Reader<'a> {
    pub fn new(source: &'a mut dyn Source) -> Self {
        Reader {
            source,
            reads: 0,
            capped: false,
            exhausted: false,
        }
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: GetXMLFileData.
    /// A file's text as UIO's reader gives it: the patched text ([`Patched::insert`]) or the
    /// game's read, with its includes expanded; `None` when the file is missing (UIO answers an
    /// empty text there).
    pub fn file(&mut self, path: &[u8]) -> Option<Vec<u8>> {
        self.reads = 0;
        self.capped = false;
        self.exhausted = false;
        self.file_at(path, 0, MAX_TEXT)
    }

    /// [`Reader::file`] at an include `depth`, with `room` bytes left for its text and includes.
    /// A patched file expanded with no limit reached is kept expanded for later reads; one that
    /// met a limit is expanded again at the next read (where it may meet none).
    fn file_at(&mut self, path: &[u8], depth: u32, room: usize) -> Option<Vec<u8>> {
        let hash = str_hash(path);
        match self.source.serve(hash) {
            Some(Served::Expanded(text)) => return Some(text),
            Some(Served::Unexpanded(text)) => {
                let outer = std::mem::replace(&mut self.capped, false);
                let text = self.resolve(text, depth, room);
                let capped = self.capped;
                self.capped |= outer;
                if !capped {
                    self.source.keep(hash, text.clone());
                }
                return Some(text);
            }
            None => {}
        }
        match self.source.raw(path, room) {
            Fetch::Text(text) => Some(self.resolve(text, depth, room)),
            Fetch::Missing => None,
            Fetch::TooLarge => {
                self.stop();
                None
            }
        }
    }

    /// The read's limit reached: no further include is fetched.
    fn stop(&mut self) {
        self.exhausted = true;
        self.capped = true;
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: ResolveXMLIncludes.
    /// `text` with each `<include` replaced as UIO does: the tag ends at the first `>` from its
    /// 21st byte on (so a short tag takes in the next tag); the path is between the first `"` from
    /// its 14th byte and the next `"`; a path under 5 bytes, or no quotes, drops the tag; else the
    /// tag is replaced by the text of `data\menus\prefabs\<path>` read by [`Reader::file`] (its
    /// includes expanded in turn; a missing file, or a path longer than UIO's buffer
    /// ([`UIO_PATH`]), gives nothing). A tag with no `>` after it ends the expansion there (the
    /// rest is kept as it is). `room`: the bytes left for the text made ([`MAX_TEXT`]).
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
            if PREFABS.len() + (q2 - start) > UIO_PATH || self.exhausted {
                continue;
            }
            if depth >= MAX_DEPTH {
                self.capped = true;
                continue;
            }
            if self.reads >= MAX_READS || out.len() >= room {
                self.stop();
                continue;
            }
            self.reads += 1;
            let path = [PREFABS, &d[start..q2]].concat();
            if let Some(t) = self.file_at(&path, depth + 1, room - out.len()) {
                if out.len() + t.len() <= room {
                    out.extend_from_slice(&t);
                } else {
                    self.stop();
                }
            }
        }
        if !found {
            d.pop();
            return d;
        }
        let rest = find_chr(&d, cur, 0).unwrap_or(d.len()) - cur;
        out.extend_from_slice(&d[cur..cur + rest]);
        out
    }
}

/// `FindChr` (UIO) on a C string: the first `c` at or after `from`, stopping at a 0 (nothing from
/// past the end). `c` = 0 finds the 0 that ends the string.
fn find_chr(d: &[u8], from: usize, c: u8) -> Option<usize> {
    let mut i = from;
    while i < d.len() {
        if d[i] == c {
            return Some(i);
        }
        if d[i] == 0 {
            return None;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
thread_local! {
    /// The bytes [`find`] has looked at (the tests' work count).
    pub(crate) static FIND_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// `strstr` on the C string at `from`: a forward search that stops at the first match or the
/// first 0 (`pat` has no 0, so a match never runs past one).
fn find(d: &[u8], from: usize, pat: &[u8]) -> Option<usize> {
    let mut i = from;
    while i < d.len() && d[i] != 0 {
        #[cfg(test)]
        FIND_WORK.with(|w| w.set(w.get() + 1));
        if d[i..].starts_with(pat) {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
