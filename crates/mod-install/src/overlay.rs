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
//!   this folder only ever removes a name and makes a new one; folders are checked to be plain
//!   folders, never links, before a name is made in them.
//! - Only what changed is redone. What the folder holds is kept beside it ([`state_path`]): each
//!   name with its source, size and modified time. A start with the same game and layers finds
//!   everything the same and touches nothing; a changed file is relinked alone; a name no longer
//!   wanted is removed. Without that record (or with one that cannot be read) the folder is made
//!   again from nothing.
//! - Folder links: on Windows, a folder link (junction) in the game's Data is left out, as before;
//!   on Linux and macOS folder links in the game's Data are followed once each (a link back to a
//!   folder already walked is not walked again), and a file link is taken as the file it leads to.
//!   Folder links in a layer are never followed (the unpacker never makes them).
//! - The game's Data folder: `Data` exactly, else of the folders named so in another case, the one
//!   holding `FalloutNV.esm`, else the first by name (as `crates/game-folders` picks it).
//!
//! The plugin list: the loose `mods` folder's own `plugins.txt` when there is one, with the
//! installed mods' plugins it does not name added after it (and said: [`Built::added_plugins`]),
//! else [`plugin_list`] of every layer's top-level plugins.
//!
//! # Limits
//!
//! [`MAX_DEPTH`] folders deep, [`MAX_FILES`] files in a layer, in the game's folder and folders
//! walked, [`MAX_HEADER_BYTES`] of a plugin read for its masters: past one the build stops in
//! words. The caps are parameters of [`build_with`] so tests can lower them.

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

/// The plugin list for these mod plugins: the official masters they name, their `.esm`, their
/// `.esp` (the game orders the load by file date; this only says which are on).
pub fn plugin_list(plugins: &[(String, Vec<u8>)]) -> String {
    let needed: BTreeSet<String> = plugins
        .iter()
        .flat_map(|(_, header)| masters(header))
        .map(|m| m.to_ascii_lowercase())
        .collect();
    let mut names: Vec<&str> = OFFICIAL
        .iter()
        .copied()
        .filter(|m| needed.contains(&m.to_ascii_lowercase()))
        .collect();
    let by_ext = |ext: &str| -> Vec<&str> {
        let mut v: Vec<&str> = plugins
            .iter()
            .map(|(n, _)| n.as_str())
            .filter(|n| n.to_ascii_lowercase().ends_with(ext))
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

/// How the folder's names are made: a seam so tests can make hard links fail as a drive would.
pub trait Linker {
    fn hard_link(&self, from: &Path, to: &Path) -> std::io::Result<()>;
}

/// The system's own hard links.
pub struct System;

impl Linker for System {
    fn hard_link(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::hard_link(from, to)
    }
}

/// Links the game's file `from` as `to`: a hard link; on Linux and macOS a symbolic link to it
/// when a hard link cannot be made there ([`falls_back_to_symlink`]).
fn link_game_file(linker: &dyn Linker, from: &Path, to: &Path) -> std::io::Result<()> {
    match linker.hard_link(from, to) {
        #[cfg(unix)]
        Err(e) if e.raw_os_error().is_some_and(falls_back_to_symlink) => {
            std::os::unix::fs::symlink(from, to)
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

/// Whether the top-level entry `name` of a layer stays out of the game's folder.
fn left_out(name: &str, is_dir: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    if is_dir {
        return lower == "installed archives";
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

fn source(path: PathBuf, game: bool) -> std::io::Result<Source> {
    let meta = std::fs::metadata(&path)?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Ok(Source {
        path,
        game,
        size: meta.len(),
        modified,
    })
}

/// The folder's wanted names: by name without ASCII case, (own spelling, source); and its
/// folders by name without case, (own spelling).
#[derive(Default)]
struct Wanted {
    files: BTreeMap<String, (String, Source)>,
    folders: BTreeMap<String, String>,
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
            key.push_str(&part.to_ascii_lowercase());
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
        let spelled = self.spell(relative);
        self.files
            .insert(spelled.to_ascii_lowercase(), (spelled, source));
    }
}

/// The entries of `dir`, sorted by name, so the first of two case-alike names is the same on
/// every machine (the exact one is taken first by the caller where it matters).
fn sorted_entries(dir: &Path) -> std::io::Result<Vec<std::fs::DirEntry>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

/// The game's Data folder: `Data` exactly, else of the case-alike ones (sorted) the one holding
/// `FalloutNV.esm` in any case, else the first; `None` when there is none.
pub fn game_data(game: &Path) -> Option<PathBuf> {
    let exact = game.join("Data");
    if exact.is_dir()
        && std::fs::read_dir(game)
            .is_ok_and(|mut d| d.any(|e| e.is_ok_and(|e| e.file_name() == "Data")))
    {
        return Some(exact);
    }
    let alike: Vec<PathBuf> = sorted_entries(game)
        .ok()?
        .into_iter()
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
        let mut path = entry.path();
        let mut is_dir = kind.is_dir();
        let is_file;
        if kind.is_symlink() {
            if cfg!(windows) {
                continue;
            }
            let Ok(real) = std::fs::canonicalize(&path) else {
                continue;
            };
            is_dir = real.is_dir();
            is_file = real.is_file();
            path = real;
        } else {
            is_file = kind.is_file();
        }
        relative.push(entry.file_name().to_string_lossy().into_owned());
        if is_dir {
            let real = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if seen.insert(real) {
                walk_game(&path, relative, wanted, caps, seen, count)?;
            }
        } else if is_file {
            wanted.put(relative, source(path, true)?);
        }
        relative.pop();
    }
    Ok(())
}

/// A layer's files into `wanted` under `Data/` (by [`left_out`]); folder links never followed.
/// Returns the layer's top-level plugins' names.
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
        } else if kind.is_file() || (kind.is_symlink() && entry.path().is_file()) {
            let path = std::fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path());
            let lower = name.to_ascii_lowercase();
            if top && (lower.ends_with(".esm") || lower.ends_with(".esp")) {
                plugins.push((name.clone(), path.clone()));
            }
            wanted.put(relative, source(path, false)?);
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

const STATE_FIRST_LINE: &str = "nv-rs modgame 1";

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
    out
}

/// The record read back: (spelled name, source) by name without case; `None` when it is not one.
fn parse_state(text: &str) -> Option<BTreeMap<String, (String, Source)>> {
    let mut lines = text.lines();
    if lines.next()? != STATE_FIRST_LINE {
        return None;
    }
    let mut out = BTreeMap::new();
    for line in lines {
        let mut f = line.split('\t');
        let game = match f.next()? {
            "g" => true,
            "m" => false,
            _ => return None,
        };
        let size = f.next()?.parse().ok()?;
        let modified = f.next()?.parse().ok()?;
        let path = PathBuf::from(f.next()?);
        let spelled = f.next()?.to_owned();
        out.insert(
            spelled.to_ascii_lowercase(),
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
    Some(out)
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

/// The path of `relative` under `target`, each of its folders checked to be a plain folder (never
/// a link) before a name is made or removed in it; with `make`, the folders missing are made.
fn plain_folders(target: &Path, relative: &str, make: bool) -> std::io::Result<PathBuf> {
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && make => std::fs::create_dir(&at)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(at.join(parts[parts.len() - 1]))
            }
            Err(e) => return Err(e),
        }
    }
    Ok(at.join(parts[parts.len() - 1]))
}

/// Removes the name `path` (a file or a link; never what a link leads to); gone already is fine.
fn remove_name(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
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
    let describe = |what: &str, path: &Path, e: std::io::Error| ModError {
        kind: classify(e.raw_os_error(), what == "linking"),
        step: format!("{what} {}: {e}", path.display()),
    };
    // What the folder should hold.
    let mut wanted = Wanted::default();
    let mut count = 0usize;
    for entry in sorted_entries(game).map_err(|e| describe("reading", game, e))? {
        let kind = entry
            .file_type()
            .map_err(|e| describe("reading", game, e))?;
        if kind.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            wanted.put(
                &[name],
                source(entry.path(), true).map_err(|e| describe("reading", &entry.path(), e))?,
            );
        }
    }
    let game_data = game_data(game).ok_or_else(|| ModError {
        kind: Failure::Other,
        step: format!("finding the game's Data folder in {}", game.display()),
    })?;
    let mut seen = BTreeSet::new();
    seen.insert(std::fs::canonicalize(&game_data).unwrap_or_else(|_| game_data.clone()));
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
        std::fs::create_dir_all(layer).map_err(|e| describe("making", layer, e))?;
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
    for (spelled, s) in wanted.files.values() {
        if !s.game {
            built.mod_files += 1;
            let text = spelled.to_ascii_lowercase();
            if text.contains("/nvse/") || text.ends_with(".dll") {
                built.extender_plugins = true;
            }
        }
    }
    // What it holds now: the record, when the folder is there and the record reads.
    let state = state_path(target);
    let previous = if target.is_dir() {
        std::fs::read_to_string(&state)
            .ok()
            .and_then(|t| parse_state(&t))
    } else {
        None
    };
    let previous = match previous {
        Some(p) => p,
        None => {
            // From nothing: a link left in an old folder is removed, never followed.
            if target.symlink_metadata().is_ok() {
                std::fs::remove_dir_all(target).map_err(|e| describe("removing", target, e))?;
            }
            BTreeMap::new()
        }
    };
    std::fs::create_dir_all(target).map_err(|e| describe("making", target, e))?;
    // The record goes first: a build stopped part-way is made again from nothing next time.
    let _ = std::fs::remove_file(&state);
    for (key, (spelled, _)) in &previous {
        if !wanted.files.contains_key(key) {
            remove_name(
                &plain_folders(target, spelled, false)
                    .map_err(|e| describe("removing", target, e))?,
            )
            .map_err(|e| describe("removing", target, e))?;
            built.removed += 1;
        }
    }
    for (key, (spelled, s)) in &wanted.files {
        if let Some((old_spelled, old)) = previous.get(key) {
            if old == s && old_spelled == spelled {
                continue;
            }
            remove_name(
                &plain_folders(target, old_spelled, false)
                    .map_err(|e| describe("replacing", target, e))?,
            )
            .map_err(|e| describe("replacing", target, e))?;
            built.removed += 1;
        }
        let to = plain_folders(target, spelled, true).map_err(|e| describe("making", target, e))?;
        // The name goes first: making it again never writes into what was there.
        remove_name(&to).map_err(|e| describe("replacing", &to, e))?;
        if s.game {
            link_game_file(linker, &s.path, &to).map_err(|e| describe("linking", &s.path, e))?;
            built.linked += 1;
        } else if linker.hard_link(&s.path, &to).is_err() {
            std::fs::copy(&s.path, &to).map_err(|e| describe("copying", &s.path, e))?;
            built.copied += 1;
        }
        built.made += 1;
    }
    std::fs::write(&state, state_text(&wanted.files))
        .map_err(|e| describe("writing", &state, e))?;
    // The plugin list.
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
    let mut all: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
    for plugins in &layer_plugins {
        for (name, path) in plugins {
            all.insert(name.to_ascii_lowercase(), (name.clone(), path.clone()));
        }
    }
    let own = layers.last().map(|l| l.join("plugins.txt"));
    let list = match own.and_then(|p| std::fs::read(p).ok()) {
        Some(mut own) => {
            let named: BTreeSet<String> = String::from_utf8_lossy(&own)
                .lines()
                .map(|l| l.trim().to_ascii_lowercase())
                .collect();
            let installed: Vec<(String, PathBuf)> = layer_plugins
                [..layer_plugins.len().saturating_sub(1)]
                .iter()
                .flatten()
                .filter(|(n, _)| !named.contains(&n.to_ascii_lowercase()))
                .map(|(n, p)| (n.to_ascii_lowercase(), (n.clone(), p.clone())))
                .collect::<BTreeMap<String, (String, PathBuf)>>()
                .into_values()
                .collect();
            if !installed.is_empty() {
                let extra = plugin_list(&headers(&installed)?);
                for name in String::from_utf8_lossy(extra.as_bytes()).lines() {
                    if named.contains(&name.to_ascii_lowercase()) {
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
        None => plugin_list(&headers(&all.into_values().collect::<Vec<_>>())?).into_bytes(),
    };
    if let Some(parent) = plugins_txt.parent() {
        std::fs::create_dir_all(parent).map_err(|e| describe("making", parent, e))?;
    }
    std::fs::write(plugins_txt, list).map_err(|e| describe("writing", plugins_txt, e))?;
    Ok(built)
}
