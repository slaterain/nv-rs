//! The whole game folder "Play with mods" plays from (Play.exe's `userdata\modgame`; play.sh's
//! own folder's `modgame` on Linux and macOS): the game's own files are never written.
//!
//! Every file of the game's folder (its top and its `Data`) is linked there: a hard link, another
//! name for the same file. Where a hard link cannot reach (Linux and macOS: the game on another
//! file system, `EXDEV`), a symbolic link instead; Windows needs the same drive (symbolic links
//! there need rights a player does not have), and says so. Then each layer's files are copied
//! over, in order (later wins), the link removed first so a copy never writes through it. A file
//! or folder already there under another case is the one replaced or filled, as the game sees
//! one name whatever its case (Linux's file systems would otherwise keep both).
//!
//! Left out of each layer: at its top, `plugins.txt`, `READ ME...` files, the install record
//! ([`crate::RECORD`]), mod archives (`.7z`, `.zip`, `.rar`, `.fomod`: the installer's input, not
//! the game's) and the `installed archives` folder.
//!
//! The plugin list: the last layer's own `plugins.txt` as it is when there is one (the loose
//! `mods` folder's, as before), else [`plugin_list`] of every layer's top-level plugins.

use std::collections::BTreeSet;
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

/// What building the folder found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Built {
    pub linked: u64,
    pub mod_files: usize,
    /// A script extender's plugin folder or a DLL among the mods: not used.
    pub extender_plugins: bool,
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
        let advice = match self.kind {
            Failure::OtherDrive => "For mods nv-rs makes links to the game's files, never copies, and a link can't reach another drive. Move the nv-rs folder to the drive the game is on, or play without mods.",
            Failure::NoLinks => "For mods nv-rs makes links to the game's files, never copies, and this drive can't hold links: it needs a normal Windows (NTFS) drive. Move the nv-rs folder to an NTFS drive that the game is on, or play without mods.",
            Failure::Locked => "Another program has this file open: the game or its launcher still running, a mod manager, or an antivirus scanning it. Close it, wait a moment and try again.",
            Failure::Denied => "The system did not allow it: the file or folder may be read-only, or your account lacks the rights, or an antivirus or controlled folder access blocks it. Clear \"Read-only\" (on Windows in its Properties), or allow nv-rs, and try again.",
            Failure::Full => "There is no room left on the drive. Free some space, or play without mods: the links take little room, but the mods' files are copied.",
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

/// The kind of failure for this system's error code (Linux's and macOS's `errno`, the values both
/// share), for `linking` a file or any other step. A link across file systems (`EXDEV`) never
/// fails the build: a symbolic link is made instead.
#[cfg(not(windows))]
pub fn classify(code: Option<i32>, linking: bool) -> Failure {
    match code {
        Some(1 | 13 | 30) => Failure::Denied,
        Some(16 | 26) => Failure::Locked,
        Some(28 | 69) => Failure::Full,
        Some(18) if linking => Failure::OtherDrive,
        _ => Failure::Other,
    }
}

/// Links the game's file `from` as `to`: a hard link; on Linux and macOS a symbolic link when the
/// game is on another file system.
fn link(from: &Path, to: &Path) -> std::io::Result<()> {
    match std::fs::hard_link(from, to) {
        #[cfg(unix)]
        Err(e) if e.raw_os_error() == Some(18) => std::os::unix::fs::symlink(from, to),
        other => other,
    }
}

fn link_tree(from: &Path, to: &Path) -> std::io::Result<u64> {
    std::fs::create_dir_all(to)?;
    let mut count = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        // A link to a folder is followed, as the game would follow it.
        let path = entry.path();
        if path.is_dir() {
            count += link_tree(&path, &target)?;
        } else if path.is_file() {
            link(&path, &target)?;
            count += 1;
        }
    }
    Ok(count)
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

/// Files under `folder`, relative to it, by [`left_out`].
fn layer_files(folder: &Path, relative: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let top = relative.as_os_str().is_empty();
    for entry in std::fs::read_dir(folder.join(relative))? {
        let entry = entry?;
        let name = entry.file_name();
        let path = relative.join(&name);
        let is_dir = entry.path().is_dir();
        if top && left_out(&name.to_string_lossy(), is_dir) {
            continue;
        }
        if is_dir {
            layer_files(folder, &path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// `relative` under `base`, each part an entry already there under another case if there is
/// one; the folders on the way made.
fn place(base: &Path, relative: &Path) -> std::io::Result<PathBuf> {
    let parts: Vec<_> = relative.components().collect();
    let mut at = base.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        let part = part.as_os_str();
        let last = i + 1 == parts.len();
        let wanted = part.to_string_lossy();
        let existing = std::fs::read_dir(&at).ok().and_then(|dir| {
            dir.filter_map(Result::ok)
                .find(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&wanted)
                        && (last || e.path().is_dir())
                })
                .map(|e| e.file_name())
        });
        at.push(existing.as_deref().unwrap_or(part));
        if !last {
            std::fs::create_dir_all(&at)?;
        }
    }
    Ok(at)
}

/// Makes `target` (removed first) from the game's folder `game` and the `layers` over it (see the
/// module's text), and writes the plugin list to `plugins_txt`.
pub fn build(
    game: &Path,
    layers: &[PathBuf],
    target: &Path,
    plugins_txt: &Path,
) -> Result<Built, ModError> {
    let describe = |what: &str, path: &Path, e: std::io::Error| ModError {
        kind: classify(e.raw_os_error(), what == "linking"),
        step: format!("{what} {}: {e}", path.display()),
    };
    // A symbolic link left from an earlier build is removed, never followed.
    if target.exists() || target.symlink_metadata().is_ok() {
        std::fs::remove_dir_all(target).map_err(|e| describe("removing", target, e))?;
    }
    std::fs::create_dir_all(target).map_err(|e| describe("making", target, e))?;
    let mut built = Built::default();
    for entry in std::fs::read_dir(game).map_err(|e| describe("reading", game, e))? {
        let entry = entry.map_err(|e| describe("reading", game, e))?;
        let path = entry.path();
        if path.is_file() {
            link(&path, &target.join(entry.file_name()))
                .map_err(|e| describe("linking", &path, e))?;
            built.linked += 1;
        }
    }
    // The game's Data folder in whatever case it has (a Linux copy may say "data").
    let game_data = place(game, Path::new("Data")).map_err(|e| describe("reading", game, e))?;
    let data = target.join("Data");
    built.linked += link_tree(&game_data, &data).map_err(|e| describe("linking", &game_data, e))?;
    let mut plugins: Vec<(String, Vec<u8>)> = Vec::new();
    for layer in layers {
        std::fs::create_dir_all(layer).map_err(|e| describe("making", layer, e))?;
        let mut files = Vec::new();
        layer_files(layer, Path::new(""), &mut files).map_err(|e| describe("reading", layer, e))?;
        for relative in &files {
            let source = layer.join(relative);
            let destination = place(&data, relative).map_err(|e| describe("making", &data, e))?;
            // The link goes first: a copy over it would write into the game's own file.
            if destination.symlink_metadata().is_ok() {
                std::fs::remove_file(&destination)
                    .map_err(|e| describe("replacing", &destination, e))?;
            }
            std::fs::copy(&source, &destination).map_err(|e| describe("copying", &source, e))?;
            let text = relative
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            if text.contains("nvse/") || text.ends_with(".dll") {
                built.extender_plugins = true;
            }
            let top = relative.components().count() == 1;
            if top && (text.ends_with(".esm") || text.ends_with(".esp")) {
                let name = relative.to_string_lossy().into_owned();
                let header = std::fs::read(&source).map_err(|e| describe("reading", &source, e))?;
                plugins.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
                plugins.push((name, header));
            }
            built.mod_files += 1;
        }
    }
    let own = layers.last().map(|l| l.join("plugins.txt"));
    let list = match own.and_then(|p| std::fs::read(p).ok()) {
        Some(own) => own,
        None => plugin_list(&plugins).into_bytes(),
    };
    if let Some(parent) = plugins_txt.parent() {
        std::fs::create_dir_all(parent).map_err(|e| describe("making", parent, e))?;
    }
    std::fs::write(plugins_txt, list).map_err(|e| describe("writing", plugins_txt, e))?;
    Ok(built)
}
