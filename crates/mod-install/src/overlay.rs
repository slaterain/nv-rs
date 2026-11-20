//! The whole game folder "Play with mods" plays from (Play.exe's `userdata\modgame`; on Linux and
//! macOS the port's own folder's `modgame`, for play.sh's `--mods`): the game's own files are
//! never written.
//!
//! # What is in it
//!
//! Every file of the game's folder (its top and its `Data`) under its own name, then each layer's
//! files over them, in order (later wins): the installed mods, then the hand-filled loose `mods`
//! folder. A layer's file or folder whose name differs only in case from one already there takes
//! the name already there, as the game sees one name whatever its case. Left out of each layer: at
//! its top, `plugins.txt`, `READ ME...` files, the install record ([`crate::RECORD`]), mod archives
//! (`.7z`, `.zip`, `.rar`, `.fomod`) and the `installed archives` folder.
//!
//! # How, and why it is safe
//!
//! - Every file is a hard link, another name for the same file: nothing is copied, so a 20 GB
//!   texture mod costs no room and no time. A game file a hard link cannot reach is, on Linux and
//!   macOS, a symbolic link to it instead (another file system, a drive without hard links,
//!   Linux's protected hard links: [`falls_back_to_symlink`]); Windows needs the same NTFS drive,
//!   and says so. A mod's file a hard link cannot reach is copied.
//! - Nothing is ever written through a link. The port's game never writes into its Data (every
//!   file the game makes stays in the run's memory, `engine_platform::files`; mods' own files go to
//!   the port's folder beside the saves, refused inside the game folder, `host::mod_files`). Making
//!   this folder only ever removes a name and makes a new one, and every change goes through
//!   [`crate::fsops::act`] with its two guards: names made of plain parts only, and the real
//!   location of the change inside this folder and outside the game's and every layer's folder.
//!   A link at the folder's own place is removed itself (never followed) and the folder made
//!   again from nothing; folders inside it are checked to be plain folders before a name is made
//!   or removed in them.
//! - Only what changed is redone. What the folder holds is kept beside it ([`state_path`]): each
//!   name with its source, size and modified time, then an `end` line with the count, written
//!   whole (a temporary name, synced, renamed over). A start with the same game and layers reads
//!   the folders' entries (their own sizes and times; a link is resolved only where there is one),
//!   finds everything the same and touches nothing, not even the record: 0.43 s for 100,000 mod
//!   files on the maintainer's Windows PC (a release build; the first build, 100,000 hard links,
//!   about a minute; on Linux and macOS each file's own times are read too, not yet measured).
//!   A name removed by hand (or a folder) is skipped when its source goes. A changed file is relinked alone; a name no longer wanted is removed. A record that
//!   is not whole, or names anything but plain names under `Data/` or one top-level file, is not
//!   trusted: the folder is made again from nothing. A name removed by hand from the folder is
//!   not noticed while its source is unchanged (delete the folder to make it again): looking at
//!   every name each start would cost seconds with a big texture mod.
//! - Folder links: on Windows, a folder link (junction) in the game's Data is left out, as before;
//!   on Linux and macOS folder links in the game's Data are followed once each (a link back to a
//!   folder already walked is not walked again), and a file link is taken as the file it leads to.
//!   Folder links in a layer are never followed (the unpacker never makes them).
//! - The game's Data folder: `Data` exactly, else of the folders named so in another case, the one
//!   holding `FalloutNV.esm`, else the first by name (as `crates/game-folders` picks it).
//!
//! The plugin list: the loose `mods` folder's own `plugins.txt` when there is one, with the
//! installed mods' plugins it does not name added after it (and said: [`Built::added_plugins`]),
//! else [`plugin_list`] of every layer's top-level plugins, starting with every official master
//! (the DLC, Gun Runners' Arsenal, the pre-order packs) in the folder, so none is switched off.
//!
//! # Limits
//!
//! [`MAX_DEPTH`] folders deep, [`MAX_FILES`] files in a layer, in the game's folder and folders
//! walked, [`MAX_HEADER_BYTES`] of a plugin read for its masters: past one the build stops in
//! words. The caps are parameters of [`build_with`] so tests can lower them.

use crate::fsops::{self, act, Guard, Op};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The official masters a mod may name, in the game's load order.
const OFFICIAL: [&str; 9] = [
    "DeadMoney.esm",
    "HonestHearts.esm",
    "OldWorldBlues.esm",
    "LonesomeRoad.esm",
    "GunRunnersArsenal.esm",
    "ClassicPack.esm",
    "MercenaryPack.esm",
    "TribalPack.esm",
    "CaravanPack.esm",
];

/// The masters a plugin's header names (`MAST` fields of its `TES4` record).
pub fn masters(file: &[u8]) -> Vec<String> {
    if file.get(..4) != Some(b"TES4") || file.len() < 24 {
        return vec![];
    }
    let size = u32::from_le_bytes([file[4], file[5], file[6], file[7]]) as usize;
    let end = (24 + size).min(file.len());
    let mut at = 24;
    let mut out = Vec::new();
    while at + 6 <= end {
        let length = u16::from_le_bytes([file[at + 4], file[at + 5]]) as usize;
        if &file[at..at + 4] == b"MAST" {
            if let Some(name) = file.get(at + 6..at + 6 + length) {
                let name = name.split(|&b| b == 0).next().unwrap_or(name);
                out.push(String::from_utf8_lossy(name).into_owned());
            }
        }
        at += 6 + length;
    }
    out
}

/// The plugin list for these mod plugins: every official master `present` in the game's Data
/// (names in lower case) or named by a mod, in the official order (so the DLC stay on whatever
/// the mods name); then the mods' `.esm`, then their `.esp` (the game orders the load by file
/// date; this only says which are on).
pub fn plugin_list(present: &BTreeSet<String>, plugins: &[(String, Vec<u8>)]) -> String {
    let needed: BTreeSet<String> = plugins
        .iter()
        .flat_map(|(_, header)| masters(header))
        .map(|m| m.to_ascii_lowercase())
        .collect();
    let mut names: Vec<&str> = OFFICIAL
        .iter()
        .copied()
        .filter(|m| {
            let m = m.to_ascii_lowercase();
            present.contains(&m) || needed.contains(&m)
        })
        .collect();
    let official: BTreeSet<String> = names.iter().map(|n| n.to_ascii_lowercase()).collect();
    let by_ext = |ext: &str| -> Vec<&str> {
        let mut v: Vec<&str> = plugins
            .iter()
            .map(|(n, _)| n.as_str())
            .filter(|n| {
                let n = n.to_ascii_lowercase();
                n.ends_with(ext) && !official.contains(&n)
            })
            .collect();
        v.sort_by_key(|n| n.to_ascii_lowercase());
        v
    };
    names.extend(by_ext(".esm"));
    names.extend(by_ext(".esp"));
    let mut out = String::new();
    for name in names {
        out.push_str(name);
        out.push_str("\r\n");
    }
    out
}

/// What building the folder did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Built {
    /// Game files linked in this build (not those already there from the last one).
    pub linked: u64,
    /// The mods' files in the folder, all layers together.
    pub mod_files: usize,
    /// Names (re)made in this build: linked or copied, game's and mods'.
    pub made: u64,
    /// Names removed in this build (no longer wanted, or changed).
    pub removed: u64,
    /// Mods' files copied because a hard link could not reach them.
    pub copied: u64,
    /// A script extender's plugin folder or a DLL among the mods: not used.
    pub extender_plugins: bool,
    /// Installed mods' plugins added after the loose folder's own `plugins.txt`, which did not
    /// name them.
    pub added_plugins: Vec<String>,
    /// Names left out because no system can hold them safely (a control character, a `:` or
    /// ``, `.` or `..`): hand-placed files the game could not have used either.
    pub skipped: Vec<String>,
}

/// What stopped the folder being made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Hard links across two drives (Windows `ERROR_NOT_SAME_DEVICE`).
    OtherDrive,
    /// A drive that has no hard links (FAT, exFAT, a network share).
    NoLinks,
    /// Another program has the file open (sharing or lock violation; busy).
    Locked,
    /// Refused: a read-only file or folder, rights, an antivirus.
    Denied,
    /// No room on the drive.
    Full,
    Other,
}

/// A failed build: which step and path, and what that means.
#[derive(Debug)]
pub struct ModError {
    pub kind: Failure,
    pub step: String,
}

impl ModError {
    /// What the player reads: the step that failed, then what to do about that kind of failure.
    pub fn player_text(&self) -> String {
        let no_links = if cfg!(windows) {
            "For mods nv-rs makes links to the game's files, never copies, and this drive can't hold links: it needs a normal Windows (NTFS) drive. Move the nv-rs folder to an NTFS drive that the game is on, or play without mods."
        } else {
            "For mods nv-rs makes links to the game's files, never copies, and neither a hard nor a symbolic link could be made there. Move nv-rs's folder to a drive with a Linux or macOS file system, or play without mods."
        };
        let advice = match self.kind {
            Failure::OtherDrive => "For mods nv-rs makes links to the game's files, never copies, and a link can't reach another drive. Move the nv-rs folder to the drive the game is on, or play without mods.",
            Failure::NoLinks => no_links,
            Failure::Locked => "Another program has this file open: the game or its launcher still running, a mod manager, or an antivirus scanning it. Close it, wait a moment and try again.",
            Failure::Denied => "The system did not allow it: the file or folder may be read-only, or your account lacks the rights, or an antivirus or controlled folder access blocks it. Clear \"Read-only\" (on Windows in its Properties), or allow nv-rs, and try again.",
            Failure::Full => "There is no room left on the drive. Free some space, or play without mods: the links take little room, but a mod's file that can't be linked is copied.",
            Failure::Other => "Please report this if it keeps happening.",
        };
        format!(
            "nv-rs could not make its own copy of the game's folder for mods.\n\nWhat failed: {}\n\n{advice}",
            self.step
        )
    }
}

/// The kind of failure for this system's error code, for `linking` a file or any other step.
#[cfg(windows)]
pub fn classify(code: Option<i32>, linking: bool) -> Failure {
    match code {
        Some(17) => Failure::OtherDrive,
        Some(1 | 50) if linking => Failure::NoLinks,
        Some(32 | 33) => Failure::Locked,
        Some(5) => Failure::Denied,
        Some(39 | 112) => Failure::Full,
        _ => Failure::Other,
    }
}

/// The kind of failure for this system's error code (Linux's and macOS's `errno`), for `linking`
/// a file or any other step. A link that only a symbolic link can make never fails the build
/// ([`falls_back_to_symlink`]); failing both is [`Failure::NoLinks`].
#[cfg(not(windows))]
pub fn classify(code: Option<i32>, linking: bool) -> Failure {
    match code {
        Some(c) if linking && falls_back_to_symlink(c) => Failure::NoLinks,
        Some(1 | 13 | 30) => Failure::Denied,
        Some(16 | 26) => Failure::Locked,
        Some(28 | 69) => Failure::Full,
        _ => Failure::Other,
    }
}

/// Whether a hard link that failed with this `errno` is made as a symbolic link instead (Linux
/// and macOS): another file system (`EXDEV`), a file system without hard links or a file another
/// account owns under Linux's protected hard links (`EPERM`), not supported (`ENOTSUP`,
/// `EOPNOTSUPP`: 95 on Linux, 45 and 102 on macOS), too many links to the file (`EMLINK`).
pub fn falls_back_to_symlink(code: i32) -> bool {
    matches!(code, 18 | 1 | 95 | 31) || (cfg!(target_os = "macos") && matches!(code, 45 | 102))
}

/// How the folder's hard links are made: a seam so tests can make them fail as a drive would.
/// Every change still goes through [`fsops::act`].
pub trait Linker {
    fn hard_link(&self, guard: &Guard, from: &Path, to: &Path) -> std::io::Result<()>;
}

/// The system's own hard links.
pub struct System;

impl Linker for System {
    fn hard_link(&self, guard: &Guard, from: &Path, to: &Path) -> std::io::Result<()> {
        act(guard, Op::HardLink { from, to })
    }
}

/// Links the game's file `from` as `to`: a hard link; on Linux and macOS a symbolic link to it
/// when a hard link cannot be made there ([`falls_back_to_symlink`]).
fn link_game_file(
    guard: &Guard,
    linker: &dyn Linker,
    from: &Path,
    to: &Path,
) -> std::io::Result<()> {
    match linker.hard_link(guard, from, to) {
        Err(e) if cfg!(unix) && e.raw_os_error().is_some_and(falls_back_to_symlink) => {
            act(guard, Op::Symlink { from, to })
        }
        other => other,
    }
}

/// The caps: [`MAX_DEPTH`], [`MAX_FILES`], [`MAX_HEADER_BYTES`] unless a test lowers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    pub depth: usize,
    pub files: usize,
    pub header_bytes: u64,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            depth: MAX_DEPTH,
            files: MAX_FILES,
            header_bytes: MAX_HEADER_BYTES,
        }
    }
}

/// The deepest a folder may be inside the game's Data or a mod's folder: the game's deepest Data
/// paths have under 12 parts.
pub const MAX_DEPTH: usize = 64;

/// The most files (and folders walked) one layer, or the game's folder, may hold: a big texture
/// mod has tens of thousands, the game's own Data a few hundred loose files and its archives.
pub const MAX_FILES: usize = 500_000;

/// The most bytes of a plugin read for its masters: the header record comes first and is small.
pub const MAX_HEADER_BYTES: u64 = 1 << 20;

fn other(text: String) -> std::io::Error {
    std::io::Error::other(text)
}

const ARCHIVES: [&str; 4] = [".7z", ".zip", ".rar", ".fomod"];

/// Whether the top-level entry `name` of a layer stays out of the game's folder (the mod chooser
/// leaves the same out of what it counts as the player's files).
pub fn left_out(name: &str, is_dir: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    if is_dir {
        return lower == crate::installs::KEPT;
    }
    lower == "plugins.txt"
        || lower.starts_with("read me")
        || lower == crate::RECORD
        || ARCHIVES.iter().any(|a| lower.ends_with(a))
}

/// Where a name in the folder comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Source {
    path: PathBuf,
    /// A game file (linked, symbolic link allowed) or a mod's (linked, else copied).
    game: bool,
    size: u64,
    modified: u128,
}

fn nanos(meta: &std::fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos())
}

/// A source from a folder entry: its own size and time (no extra look at the file, except for a
/// link, which is taken as the file it leads to).
fn source(entry: &std::fs::DirEntry, game: bool) -> std::io::Result<Option<Source>> {
    let kind = entry.file_type()?;
    let (path, meta) = if kind.is_symlink() {
        let Ok(real) = std::fs::canonicalize(entry.path()) else {
            return Ok(None);
        };
        let meta = std::fs::metadata(&real)?;
        (real, meta)
    } else {
        (entry.path(), entry.metadata()?)
    };
    if !meta.is_file() {
        return Ok(None);
    }
    Ok(Some(Source {
        path,
        game,
        size: meta.len(),
        modified: nanos(&meta),
    }))
}

/// A name's key: Unicode lower case (Windows compares names without case, beyond ASCII too).
fn key_of(spelled: &str) -> String {
    spelled.to_lowercase()
}

/// The folder's wanted names: by [`key_of`], (own spelling, source); and its folders by key,
/// (own spelling).
#[derive(Default)]
struct Wanted {
    files: BTreeMap<String, (String, Source)>,
    folders: BTreeMap<String, String>,
    /// Names left out because no system can hold them safely ([`fsops::valid_part`]).
    skipped: Vec<String>,
}

impl Wanted {
    /// The spelling of `relative` (parts joined by `/`): each folder as already named, else as
    /// given; the file's name as already there, else as given.
    fn spell(&mut self, relative: &[String]) -> String {
        let mut spelled = String::new();
        let mut key = String::new();
        for (i, part) in relative.iter().enumerate() {
            if !key.is_empty() {
                key.push('/');
                spelled.push('/');
            }
            key.push_str(&part.to_lowercase());
            let last = i + 1 == relative.len();
            let own = if last {
                self.files
                    .get(&key)
                    .map(|(name, _)| name.rsplit('/').next().unwrap_or(name).to_owned())
                    .unwrap_or_else(|| part.clone())
            } else {
                self.folders.entry(key.clone()).or_insert_with(|| {
                    let mut s = spelled.clone();
                    s.push_str(part);
                    s
                });
                let full = &self.folders[&key];
                full.rsplit('/').next().unwrap_or(full).to_owned()
            };
            spelled.push_str(&own);
        }
        spelled
    }

    fn put(&mut self, relative: &[String], source: Source) {
        if !relative.iter().all(|p| fsops::valid_part(p)) {
            self.skipped.push(relative.join("/"));
            return;
        }
        let spelled = self.spell(relative);
        self.files.insert(key_of(&spelled), (spelled, source));
    }
}

/// The entries of `dir`, sorted by name, so the first of two case-alike names is the same on
/// every machine.
fn sorted_entries(dir: &Path) -> std::io::Result<Vec<std::fs::DirEntry>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

/// The game's Data folder: `Data` exactly, else of the case-alike ones (sorted) the one holding
/// `FalloutNV.esm` in any case, else the first; `None` when there is none.
pub fn game_data(game: &Path) -> Option<PathBuf> {
    let entries = sorted_entries(game).ok()?;
    if let Some(exact) = entries
        .iter()
        .find(|e| e.file_name() == "Data" && e.path().is_dir())
    {
        return Some(exact.path());
    }
    let alike: Vec<PathBuf> = entries
        .iter()
        .filter(|e| {
            e.file_name().to_string_lossy().eq_ignore_ascii_case("data") && e.path().is_dir()
        })
        .map(|e| e.path())
        .collect();
    let has_master = |d: &PathBuf| {
        std::fs::read_dir(d).is_ok_and(|mut entries| {
            entries.any(|e| {
                e.is_ok_and(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("FalloutNV.esm")
                })
            })
        })
    };
    alike
        .iter()
        .find(|d| has_master(d))
        .or_else(|| alike.first())
        .cloned()
}

/// Walks the game's Data into `wanted` under `Data/`. Windows: links (junctions, symbolic links)
/// are left out, as before. Linux and macOS: a folder link is followed once (a folder already
/// walked is not walked again), a file link is taken as the file it leads to.
fn walk_game(
    dir: &Path,
    relative: &mut Vec<String>,
    wanted: &mut Wanted,
    caps: Caps,
    seen: &mut BTreeSet<PathBuf>,
    count: &mut usize,
) -> std::io::Result<()> {
    if relative.len() > caps.depth {
        return Err(other(format!(
            "{} is more than {} folders deep (a link that loops back?)",
            dir.display(),
            caps.depth
        )));
    }
    for entry in sorted_entries(dir)? {
        *count += 1;
        if *count > caps.files {
            return Err(other(format!(
                "the game's folder holds more than {} files and folders",
                caps.files
            )));
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() && cfg!(windows) {
            continue;
        }
        relative.push(entry.file_name().to_string_lossy().into_owned());
        let is_dir = if kind.is_symlink() {
            entry.path().is_dir()
        } else {
            kind.is_dir()
        };
        if is_dir {
            let real = if kind.is_symlink() {
                std::fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path())
            } else {
                entry.path()
            };
            if !kind.is_symlink() || seen.insert(real.clone()) {
                walk_game(&real, relative, wanted, caps, seen, count)?;
            }
        } else if let Some(s) = source(&entry, true)? {
            wanted.put(relative, s);
        }
        relative.pop();
    }
    Ok(())
}

/// A layer's files into `wanted` under `Data/` (by [`left_out`]); folder links never followed.
/// Collects the layer's top-level plugins.
fn walk_layer(
    root: &Path,
    dir: &Path,
    relative: &mut Vec<String>,
    wanted: &mut Wanted,
    caps: Caps,
    count: &mut usize,
    plugins: &mut Vec<(String, PathBuf)>,
) -> std::io::Result<()> {
    if relative.len() > caps.depth {
        return Err(other(format!(
            "{} is more than {} folders deep",
            dir.display(),
            caps.depth
        )));
    }
    let top = relative.len() == 1;
    for entry in sorted_entries(dir)? {
        let kind = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if top && left_out(&name, kind.is_dir()) {
            continue;
        }
        *count += 1;
        if *count > caps.files {
            return Err(other(format!(
                "{} holds more than {} files",
                root.display(),
                caps.files
            )));
        }
        relative.push(name.clone());
        if kind.is_dir() {
            walk_layer(root, &entry.path(), relative, wanted, caps, count, plugins)?;
        } else if let Some(s) = source(&entry, false)? {
            let lower = name.to_ascii_lowercase();
            if top && (lower.ends_with(".esm") || lower.ends_with(".esp")) {
                plugins.push((name.clone(), s.path.clone()));
            }
            wanted.put(relative, s);
        }
        relative.pop();
    }
    Ok(())
}

/// The record of what the folder holds, beside it: `target` with `.nv-rs-state` added.
pub fn state_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".nv-rs-state");
    target.with_file_name(name)
}

fn state_temporary(target: &Path) -> PathBuf {
    let mut name = state_path(target)
        .file_name()
        .unwrap_or_default()
        .to_os_string();
    name.push(".part");
    target.with_file_name(name)
}

const STATE_FIRST_LINE: &str = "nv-rs modgame 2";

/// The record's text: a line a name, then `end` and the count. Every name in it is a valid one
/// ([`Wanted::put`] takes no other), so no name can make another line.
fn state_text(files: &BTreeMap<String, (String, Source)>) -> String {
    let mut out = String::from(STATE_FIRST_LINE);
    out.push('\n');
    for (spelled, s) in files.values() {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            if s.game { "g" } else { "m" },
            s.size,
            s.modified,
            s.path.to_string_lossy(),
            spelled
        ));
    }
    out.push_str(&format!("end\t{}\n", files.len()));
    out
}

/// Whether a record's name is one the build makes: valid parts only, and either `Data/...` or one
/// top-level file name.
fn trusted_name(spelled: &str) -> bool {
    fsops::valid_relative(spelled)
        && (!spelled.contains('/') || spelled.starts_with("Data/"))
        && !spelled.eq_ignore_ascii_case("Data")
}

/// The record read back: (spelled name, source) by key; `None` when it is not a whole, valid
/// record (its first line, a valid name on every line, an `end` line with the right count last).
fn parse_state(text: &str) -> Option<BTreeMap<String, (String, Source)>> {
    let mut lines = text.lines();
    if lines.next()? != STATE_FIRST_LINE {
        return None;
    }
    let mut out = BTreeMap::new();
    let mut ended = None;
    for line in lines {
        if ended.is_some() {
            return None;
        }
        let mut f = line.split('\t');
        let first = f.next()?;
        if first == "end" {
            ended = Some(f.next()?.parse::<usize>().ok()?);
            continue;
        }
        let game = match first {
            "g" => true,
            "m" => false,
            _ => return None,
        };
        let size = f.next()?.parse().ok()?;
        let modified = f.next()?.parse().ok()?;
        let path = PathBuf::from(f.next()?);
        let spelled = f.next()?.to_owned();
        if f.next().is_some() || !trusted_name(&spelled) {
            return None;
        }
        out.insert(
            key_of(&spelled),
            (
                spelled,
                Source {
                    path,
                    game,
                    size,
                    modified,
                },
            ),
        );
    }
    (ended? == out.len()).then_some(out)
}

/// The first [`Caps::header_bytes`] of a plugin: enough for its header record and masters.
fn header(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut out = Vec::new();
    std::fs::File::open(path)?
        .take(limit)
        .read_to_end(&mut out)?;
    Ok(out)
}

/// The path of the valid name `relative` under `target`, each of its folders checked to be a
/// plain folder (never a link); with `make`, the folders missing are made (through [`act`]);
/// without, `None` when one is missing (nothing is there).
fn plain_folders(
    guard: &Guard,
    target: &Path,
    relative: &str,
    make: bool,
) -> std::io::Result<Option<PathBuf>> {
    if !fsops::valid_relative(relative) {
        return Err(other(format!("{relative:?} is not a plain name")));
    }
    let mut at = target.to_path_buf();
    let parts: Vec<&str> = relative.split('/').collect();
    for part in &parts[..parts.len() - 1] {
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(other(format!(
                    "{} is not a plain folder (a link?)",
                    at.display()
                )))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && make => {
                act(guard, Op::MakeDir(&at))?
            }
            // A folder on the way gone (removed by hand): nothing there to remove.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        }
    }
    Ok(Some(at.join(parts[parts.len() - 1])))
}

/// The failure of the step `what` on `path`, in words.
fn describe(what: &str, path: &Path, e: std::io::Error) -> ModError {
    ModError {
        kind: classify(e.raw_os_error(), what == "linking"),
        step: format!("{what} {}: {e}", path.display()),
    }
}

/// Makes `target` from the game's folder `game` and the `layers` over it (see the module's text),
/// redoing only what changed since the last build, and writes the plugin list to `plugins_txt`.
pub fn build(
    game: &Path,
    layers: &[PathBuf],
    target: &Path,
    plugins_txt: &Path,
) -> Result<Built, ModError> {
    build_with(game, layers, target, plugins_txt, Caps::default(), &System)
}

/// [`build`] with these caps and this way of making hard links.
pub fn build_with(
    game: &Path,
    layers: &[PathBuf],
    target: &Path,
    plugins_txt: &Path,
    caps: Caps,
    linker: &dyn Linker,
) -> Result<Built, ModError> {
    let state = state_path(target);
    let state_part = state_temporary(target);
    let mut forbidden = vec![game.to_path_buf()];
    forbidden.extend(layers.iter().cloned());
    let refusal = |why: String| ModError {
        kind: Failure::Other,
        step: format!("refused: {why}"),
    };
    let mut guard = Guard::new(
        target,
        &forbidden,
        &[state.clone(), plugins_txt.to_path_buf()],
    )
    .map_err(refusal)?;
    // What the folder should hold.
    let mut wanted = Wanted::default();
    for entry in sorted_entries(game).map_err(|e| describe("reading", game, e))? {
        let kind = entry
            .file_type()
            .map_err(|e| describe("reading", game, e))?;
        if kind.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(s) = source(&entry, true).map_err(|e| describe("reading", game, e))? {
                wanted.put(&[name], s);
            }
        }
    }
    let game_data = game_data(game).ok_or_else(|| ModError {
        kind: Failure::Other,
        step: format!("finding the game's Data folder in {}", game.display()),
    })?;
    let mut seen = BTreeSet::new();
    let mut count = 0usize;
    walk_game(
        &game_data,
        &mut vec!["Data".to_owned()],
        &mut wanted,
        caps,
        &mut seen,
        &mut count,
    )
    .map_err(|e| describe("reading", &game_data, e))?;
    let mut built = Built::default();
    let mut layer_plugins: Vec<Vec<(String, PathBuf)>> = Vec::new();
    for layer in layers {
        if !layer.exists() {
            act(&guard, Op::MakeDirAll(layer)).map_err(|e| describe("making", layer, e))?;
            // Its real location learnt now it is there: the guard is never weaker.
            guard.settle().map_err(refusal)?;
        }
        let mut plugins = Vec::new();
        let mut layer_count = 0usize;
        walk_layer(
            layer,
            layer,
            &mut vec!["Data".to_owned()],
            &mut wanted,
            caps,
            &mut layer_count,
            &mut plugins,
        )
        .map_err(|e| describe("reading", layer, e))?;
        layer_plugins.push(plugins);
    }
    built.skipped = std::mem::take(&mut wanted.skipped);
    // The official masters in the folder (the game's own, or a mod's copy): kept on.
    let present: BTreeSet<String> = OFFICIAL
        .iter()
        .map(|m| m.to_ascii_lowercase())
        .filter(|m| wanted.files.contains_key(&key_of(&format!("Data/{m}"))))
        .collect();
    for (spelled, s) in wanted.files.values() {
        if !s.game {
            built.mod_files += 1;
            let text = spelled.to_ascii_lowercase();
            if text.contains("/nvse/") || text.ends_with(".dll") {
                built.extender_plugins = true;
            }
        }
    }
    // What it holds now: the record, when the folder is a plain folder (never a link at its
    // place) and the record is whole and valid.
    let plain_target =
        std::fs::symlink_metadata(target).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink());
    let previous = if plain_target {
        std::fs::read_to_string(&state)
            .ok()
            .and_then(|t| parse_state(&t))
    } else {
        None
    };
    let from_nothing = previous.is_none();
    let previous = previous.unwrap_or_default();
    if from_nothing {
        // A link at the folder's place, or the folder, removed: the link itself, never followed.
        act(&guard, Op::RemoveTree(target)).map_err(|e| describe("removing", target, e))?;
    }
    act(&guard, Op::MakeDir(target)).map_err(|e| describe("making", target, e))?;
    guard.settle().map_err(refusal)?;
    // What changes: names gone, and names new, changed or missing.
    let gone: Vec<&String> = previous
        .iter()
        .filter(|(key, _)| !wanted.files.contains_key(*key))
        .map(|(_, (spelled, _))| spelled)
        .collect();
    let mut redo: Vec<&String> = Vec::new();
    for (key, (spelled, s)) in &wanted.files {
        let same = previous
            .get(key)
            .is_some_and(|(old_spelled, old)| old == s && old_spelled == spelled);
        if !same {
            redo.push(key);
        }
    }
    if gone.is_empty() && redo.is_empty() && !from_nothing {
        // Unchanged: the folder and its record are left as they are.
        write_plugins(
            &guard,
            &present,
            plugins_txt,
            &layer_plugins,
            caps,
            layers
                .last()
                .and_then(|l| std::fs::read(l.join("plugins.txt")).ok()),
            &mut built,
        )?;
        return Ok(built);
    }
    // The record goes first: a build stopped part-way is made again from nothing next time.
    match std::fs::symlink_metadata(&state) {
        Ok(_) => act(
            &guard_for_state(&state).map_err(refusal)?,
            Op::RemoveName(&state),
        )
        .map_err(|e| describe("removing", &state, e))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(describe("removing", &state, e)),
    }
    for spelled in gone {
        let Some(path) = plain_folders(&guard, target, spelled, false)
            .map_err(|e| describe("removing", target, e))?
        else {
            continue;
        };
        act(&guard, Op::RemoveName(&path)).map_err(|e| describe("removing", &path, e))?;
        built.removed += 1;
    }
    for key in redo {
        let (spelled, s) = &wanted.files[key];
        if let Some((old_spelled, _)) = previous.get(key) {
            if let Some(old) = plain_folders(&guard, target, old_spelled, false)
                .map_err(|e| describe("replacing", target, e))?
            {
                act(&guard, Op::RemoveName(&old)).map_err(|e| describe("replacing", &old, e))?;
                built.removed += 1;
            }
        }
        let to = plain_folders(&guard, target, spelled, true)
            .map_err(|e| describe("making", target, e))?
            .ok_or_else(|| describe("making", target, other("a folder vanished".into())))?;
        // The name goes first: making it again never writes into what was there.
        act(&guard, Op::RemoveName(&to)).map_err(|e| describe("replacing", &to, e))?;
        if s.game {
            link_game_file(&guard, linker, &s.path, &to)
                .map_err(|e| describe("linking", &s.path, e))?;
            built.linked += 1;
        } else if linker.hard_link(&guard, &s.path, &to).is_err() {
            act(
                &guard,
                Op::Copy {
                    from: &s.path,
                    to: &to,
                },
            )
            .map_err(|e| describe("copying", &s.path, e))?;
            built.copied += 1;
        }
        built.made += 1;
    }
    let text = state_text(&wanted.files);
    act(
        &guard,
        Op::WriteWhole {
            to: &state,
            temporary: &state_part,
            bytes: text.as_bytes(),
        },
    )
    .map_err(|e| describe("writing", &state, e))?;
    write_plugins(
        &guard,
        &present,
        plugins_txt,
        &layer_plugins,
        caps,
        layers
            .last()
            .and_then(|l| std::fs::read(l.join("plugins.txt")).ok()),
        &mut built,
    )?;
    Ok(built)
}

/// A guard that may remove only the record beside the folder.
fn guard_for_state(state: &Path) -> Result<Guard, String> {
    Guard::new(state.parent().unwrap_or(state), &[], &[])
}

/// The plugin list (see the module's text), written whole.
fn write_plugins(
    guard: &Guard,
    present: &BTreeSet<String>,
    plugins_txt: &Path,
    layer_plugins: &[Vec<(String, PathBuf)>],
    caps: Caps,
    own_list: Option<Vec<u8>>,
    built: &mut Built,
) -> Result<(), ModError> {
    let headers = |plugins: &[(String, PathBuf)]| -> Result<Vec<(String, Vec<u8>)>, ModError> {
        plugins
            .iter()
            .map(|(name, path)| {
                header(path, caps.header_bytes)
                    .map(|h| (name.clone(), h))
                    .map_err(|e| describe("reading", path, e))
            })
            .collect()
    };
    // By name without case: a later layer's plugin of the same name replaces an earlier one's.
    let by_name = |lists: &[Vec<(String, PathBuf)>]| -> Vec<(String, PathBuf)> {
        let mut all: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
        for (name, path) in lists.iter().flatten() {
            all.insert(name.to_lowercase(), (name.clone(), path.clone()));
        }
        all.into_values().collect()
    };
    let list = match own_list {
        Some(mut own) => {
            let named: BTreeSet<String> = String::from_utf8_lossy(&own)
                .lines()
                .map(|l| l.trim().to_lowercase())
                .collect();
            let installed: Vec<(String, PathBuf)> =
                by_name(&layer_plugins[..layer_plugins.len().saturating_sub(1)])
                    .into_iter()
                    .filter(|(n, _)| !named.contains(&n.to_lowercase()))
                    .collect();
            if !installed.is_empty() {
                let extra = plugin_list(&BTreeSet::new(), &headers(&installed)?);
                for name in extra.lines() {
                    if named.contains(&name.to_lowercase()) {
                        continue;
                    }
                    if !own.is_empty() && !own.ends_with(b"\n") {
                        own.extend_from_slice(b"\r\n");
                    }
                    own.extend_from_slice(name.as_bytes());
                    own.extend_from_slice(b"\r\n");
                    built.added_plugins.push(name.to_owned());
                }
            }
            own
        }
        None => plugin_list(present, &headers(&by_name(layer_plugins))?).into_bytes(),
    };
    let mut temporary = plugins_txt.as_os_str().to_os_string();
    temporary.push(".part");
    if let Some(parent) = plugins_txt.parent() {
        act(guard, Op::MakeDirAll(parent)).map_err(|e| describe("making", parent, e))?;
    }
    act(
        guard,
        Op::WriteWhole {
            to: plugins_txt,
            temporary: Path::new(&temporary),
            bytes: &list,
        },
    )
    .map_err(|e| describe("writing", plugins_txt, e))
}
