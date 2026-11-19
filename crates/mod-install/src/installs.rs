//! Installed mods: `mod installs/<NN Name>/`, each with its [`crate::RECORD`].
//!
//! The record is plain UTF-8 text, one `key: value` a line, so a player can read it and a later
//! version can add keys (unknown ones are kept as they are by [`Record::parse`] and written back):
//!
//! ```text
//! name: Gun Pack
//! version: 1.2
//! author: Someone
//! archive: Gun Pack 1.2-1234-1-2.7z
//! archive size: 1048576
//! choice: Options / DLC patches / Honest Hearts
//! file: GunPack.esp
//! file: Meshes/Weapons/gun.nif
//! ```

use crate::fsops::{act, sync_folder, Guard, Held, Op};
use std::path::{Path, PathBuf};

/// One installed mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// Its place in the order (the folder name's number).
    pub number: u32,
    /// The rest of the folder's name.
    pub name: String,
    pub folder: PathBuf,
}

/// What an install recorded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    pub name: String,
    pub version: String,
    pub author: String,
    /// The archive's file name and size (to tell it again when it is put in the mods folder).
    pub archive: String,
    pub archive_size: u64,
    /// The choices made, `Step / Group / Option` each.
    pub choices: Vec<String>,
    /// The files installed, relative, `/` between the parts.
    pub files: Vec<String>,
    /// Lines with keys this version does not know, kept as they are.
    pub other: Vec<String>,
}

impl Record {
    /// The record's text.
    pub fn text(&self) -> String {
        let mut out = String::new();
        let mut line = |key: &str, value: &str| {
            // One line each: a line break in a value would start another key.
            let value: String = value
                .chars()
                .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                .collect();
            out.push_str(&format!("{key}: {value}\n"));
        };
        line("name", &self.name);
        line("version", &self.version);
        line("author", &self.author);
        line("archive", &self.archive);
        line("archive size", &self.archive_size.to_string());
        for c in &self.choices {
            line("choice", c);
        }
        for f in &self.files {
            line("file", f);
        }
        for o in &self.other {
            out.push_str(o);
            out.push('\n');
        }
        out
    }

    /// The record in `text`; lines it cannot read are kept in [`Record::other`].
    pub fn parse(text: &str) -> Record {
        let mut r = Record::default();
        for raw in text.lines() {
            let line = raw.trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(": ") else {
                r.other.push(line.to_owned());
                continue;
            };
            let value = value.to_owned();
            match key {
                "name" => r.name = value,
                "version" => r.version = value,
                "author" => r.author = value,
                "archive" => r.archive = value,
                "archive size" => r.archive_size = value.trim().parse().unwrap_or(0),
                "choice" => r.choices.push(value),
                "file" => r.files.push(value),
                _ => r.other.push(line.to_owned()),
            }
        }
        r
    }
}

/// The most installed mods: the largest mod lists players keep have a few thousand.
pub const MAX_INSTALLS: usize = 10_000;

/// The start of a staging folder's name: where an install unpacks before its record is written and
/// it takes its place. It never starts with a number, so [`installed`] never takes one for a mod
/// (a staging folder left by a killed install is not listed, not played, and cleared by
/// [`clear_staging`]).
pub const STAGING: &str = ".staging-";

/// Whether `folder` holds a complete record (a plain file: the record is written whole, under
/// another name first, and only then given its own).
fn has_record(folder: &Path) -> bool {
    std::fs::symlink_metadata(folder.join(crate::RECORD)).is_ok_and(|m| m.is_file())
}

/// The installed mods in `installs`, in their order: folders named `<number> <name>` holding a
/// complete record ([`crate::RECORD`]); anything else there (a staging folder, a folder without
/// its record) is not a mod and is left alone.
pub fn installed(installs: &Path) -> std::io::Result<Vec<Installed>> {
    Ok(numbered(installs)?
        .into_iter()
        .filter(|i| has_record(&i.folder))
        .collect())
}

/// Every folder in `installs` named `<number> <name>`, with its record or not, in their order.
fn numbered(installs: &Path) -> std::io::Result<Vec<Installed>> {
    let mut folders = 0usize;
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(installs) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        folders += 1;
        if folders > MAX_INSTALLS {
            return Err(std::io::Error::other(format!(
                "{} holds more than {MAX_INSTALLS} mods",
                installs.display()
            )));
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some((number, name)) = file_name.split_once(' ') else {
            continue;
        };
        // Digits only (`parse` would take a leading `+`).
        if !number.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(number) = number.parse::<u32>() else {
            continue;
        };
        out.push(Installed {
            number,
            name: name.to_owned(),
            folder: entry.path(),
        });
    }
    out.sort_by(|a, b| (a.number, &a.name).cmp(&(b.number, &b.name)));
    Ok(out)
}

/// A name every system can hold as a folder name: no `< > : " / \ | ? *` or control characters,
/// no dot or space at the end, not one of Windows' device names, at most 60 characters; `Mod`
/// when nothing is left.
pub fn safe_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    out = out.chars().take(60).collect();
    let trimmed = out.trim_end_matches(['.', ' ']).to_owned();
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if trimmed.is_empty() {
        "Mod".to_owned()
    } else if device {
        format!("{trimmed} mod")
    } else {
        trimmed
    }
}

/// The folder for a new mod named `name`: the number after the last numbered folder (with its
/// record or not, so a new install never meets a leftover's name; two digits at least), then
/// [`safe_name`]. Not made yet.
pub fn next_folder(installs: &Path, name: &str) -> std::io::Result<PathBuf> {
    let next = numbered(installs)?
        .iter()
        .map(|i| i.number)
        .max()
        .map_or(1, |n| n.saturating_add(1));
    Ok(installs.join(format!("{next:02} {}", safe_name(name))))
}

/// Refused: `path` is not directly inside `folder`.
fn not_inside(path: &Path, folder: &Path) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!(
            "{} is not directly inside {}",
            path.display(),
            folder.display()
        ),
    )
}

/// Whether `path` is a name directly inside `folder` (not `.`, `..` or empty).
fn directly_inside(path: &Path, folder: &Path) -> bool {
    path.parent() == Some(folder)
        && path
            .file_name()
            .is_some_and(|n| !n.is_empty() && n != "." && n != "..")
}

/// A guard for changes inside `root`, never inside the game's folder `game` (refused when they
/// are inside one another, either way round).
fn guard_in(root: &Path, game: &Path, extra: &[PathBuf]) -> std::io::Result<Guard> {
    Guard::new(root, &[game.to_path_buf()], extra).map_err(std::io::Error::other)
}

/// Removes an installed mod: its folder and everything in it, through the guarded changes
/// ([`crate::fsops`]), never inside the game's folder `game`. Refused (nothing removed) when the
/// folder is not directly inside `installs` or is a link.
pub fn remove(installs: &Path, mod_folder: &Path, game: &Path) -> std::io::Result<()> {
    if !directly_inside(mod_folder, installs) {
        return Err(not_inside(mod_folder, installs));
    }
    crate::fsops::remove_mod_folder(installs, mod_folder, game)
}

/// The lock file in `installs` that one mod installer holds for its whole run ([`hold_installer`]).
/// It never starts with a number or [`STAGING`]: never listed, played or cleared.
pub const LOCK: &str = ".installer.lock";

/// Takes the installer's lock in `installs` (the folder made when missing), through the guarded
/// changes, never inside the game's folder `game`: `Some` while this run holds it, `None` when
/// another installer window holds it now. A lock left by a program that has ended is free again.
pub fn hold_installer(installs: &Path, game: &Path) -> std::io::Result<Option<Held>> {
    let mut guard = guard_in(installs, game, &[])?;
    act(&guard, Op::MakeDir(installs))?;
    guard.settle().map_err(std::io::Error::other)?;
    crate::fsops::hold(&guard, &installs.join(LOCK))
}

/// What [`clear_staging`] did: how many leftovers it cleared, and each one it could not, with why.
#[derive(Debug, Default)]
pub struct Cleared {
    pub cleared: usize,
    pub left: Vec<(PathBuf, std::io::Error)>,
}

/// Clears every staging folder (named [`STAGING`]...) left in `installs` by an install that was
/// stopped (the window closed, the power cut): each removed whole, a link at its place removed
/// itself (never followed), never inside the game's folder `game`. One that can't be cleared is
/// noted and the rest are still cleared. Only while [`hold_installer`]'s lock is held: another
/// window's staging folder is its install in progress.
pub fn clear_staging(installs: &Path, game: &Path) -> std::io::Result<Cleared> {
    let entries = match std::fs::read_dir(installs) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Cleared::default()),
        Err(e) => return Err(e),
    };
    let mut done = Cleared::default();
    for entry in entries {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with(STAGING) {
            continue;
        }
        if done.cleared + done.left.len() >= MAX_INSTALLS {
            return Err(std::io::Error::other(format!(
                "{} holds more than {MAX_INSTALLS} unfinished installs",
                installs.display()
            )));
        }
        let path = entry.path();
        match guard_in(&path, game, &[]).and_then(|g| act(&g, Op::RemoveTree(&path))) {
            Ok(()) => done.cleared += 1,
            Err(e) => done.left.push((path, e)),
        }
    }
    Ok(done)
}

/// Removes one staging folder of `installs` after a failed install (another window's install
/// keeps its own), through the guarded changes, never inside the game's folder `game`.
pub fn discard_staging(installs: &Path, staging: &Path, game: &Path) -> std::io::Result<()> {
    let ours = directly_inside(staging, installs)
        && staging
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with(STAGING));
    if !ours {
        return Err(not_inside(staging, installs));
    }
    act(&guard_in(staging, game, &[])?, Op::RemoveTree(staging))
}

/// The staging folder for the mod folder `folder` (from [`next_folder`]).
pub fn staging_for(folder: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(STAGING);
    name.push(folder.file_name().unwrap_or_default());
    folder.with_file_name(name)
}

/// Makes a fresh, empty staging folder for the mod folder `folder` inside `installs` (made when
/// missing), a leftover of that name cleared first; all through the guarded changes, never inside
/// the game's folder `game`. Where to unpack.
pub fn make_staging(installs: &Path, folder: &Path, game: &Path) -> std::io::Result<PathBuf> {
    if !directly_inside(folder, installs) {
        return Err(not_inside(folder, installs));
    }
    let staging = staging_for(folder);
    let mut guard = guard_in(installs, game, &[])?;
    act(&guard, Op::MakeDir(installs))?;
    guard.settle().map_err(std::io::Error::other)?;
    act(&guard_in(&staging, game, &[])?, Op::RemoveTree(&staging))?;
    act(&guard, Op::MakeDir(&staging))?;
    Ok(staging)
}

/// Gives an unpacked staging folder its place: `record` written in it whole (under another name
/// first, so a record is there complete or not at all), then the folder renamed to `folder`
/// (nothing may be at that name). Both directly inside `installs`, never in the game's folder
/// `game`. Until the rename the mod is not listed or played ([`installed`]).
pub fn finish(
    installs: &Path,
    staging: &Path,
    folder: &Path,
    record: &Record,
    game: &Path,
) -> std::io::Result<()> {
    for place in [staging, folder] {
        if !directly_inside(place, installs) {
            return Err(not_inside(place, installs));
        }
    }
    let meta = std::fs::symlink_metadata(staging)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(std::io::Error::other(format!(
            "{} is not a plain folder",
            staging.display()
        )));
    }
    let record_path = staging.join(crate::RECORD);
    let temporary = staging.join(format!(".{}.part", crate::RECORD));
    let guard = guard_in(installs, game, std::slice::from_ref(&record_path))?;
    act(
        &guard,
        Op::WriteWhole {
            to: &record_path,
            temporary: &temporary,
            bytes: record.text().as_bytes(),
        },
    )?;
    // Linux and macOS: the record's name on the disk before the folder takes its place, and the
    // folder's new name after (the unpacked files were synced as they were written).
    sync_folder(staging)?;
    act(
        &guard,
        Op::Rename {
            from: staging,
            to: folder,
        },
    )?;
    sync_folder(installs)
}

/// The folder in the loose mods folder where installed archives are kept (not offered again, not
/// laid over the game).
pub const KEPT: &str = "installed archives";

/// The most names tried for a kept archive (`Name.zip`, `Name (2).zip`...).
const MAX_KEPT_NAMES: u32 = 1_000;

/// Moves an installed archive (directly inside the loose mods folder `loose`) into [`KEPT`] there
/// (made when missing): under its own name, or `Name (2).ext` and so on when that is taken, never
/// over anything; through the guarded changes, never inside the game's folder `game`. Where it
/// went.
pub fn keep_archive(loose: &Path, archive: &Path, game: &Path) -> std::io::Result<PathBuf> {
    if !directly_inside(archive, loose) {
        return Err(not_inside(archive, loose));
    }
    let guard = guard_in(loose, game, &[])?;
    let kept = loose.join(KEPT);
    match std::fs::symlink_metadata(&kept) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => act(&guard, Op::MakeDir(&kept))?,
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    let given = Path::new(archive.file_name().unwrap_or_default());
    let stem = given
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let extension = given
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1..=MAX_KEPT_NAMES {
        let to = if n == 1 {
            kept.join(given)
        } else {
            kept.join(format!("{stem} ({n}){extension}"))
        };
        match std::fs::symlink_metadata(&to) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                act(
                    &guard,
                    Op::Rename {
                        from: archive,
                        to: &to,
                    },
                )?;
                return Ok(to);
            }
            Err(e) => return Err(e),
            Ok(_) => {}
        }
    }
    Err(std::io::Error::other(format!(
        "{} holds {MAX_KEPT_NAMES} archives named like {}",
        kept.display(),
        given.display()
    )))
}

/// The folders "Play with mods" lays over the game, in order: each installed mod's (only those
/// with a complete record: never a staging folder or a half-finished one), then the loose `mods`
/// folder.
pub fn layers(installs: &Path, loose: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = installed(installs)?.into_iter().map(|i| i.folder).collect();
    out.push(loose.to_path_buf());
    Ok(out)
}
