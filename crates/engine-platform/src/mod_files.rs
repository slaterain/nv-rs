//! Mods' own files: the files a script extender's commands read and write for a mod (MCM's
//! `Data\Config\<mod>.ini`, later JIP LN's INI and file commands), kept between sessions in a
//! folder of the port's own beside the saves (the maintainer, 2026-11-16), never in the game's
//! folder. The extender crates reach these through the core's seam
//! (`engine_core::mod_files`, installed with the layer: [`CALLS`]), which also holds the profile
//! text rules (`ini_value`, `written_profile`).
//!
//! A name is given as the game would pass it to Windows: relative to the game's folder (the
//! current directory, `Data\Config\MCM.ini`), or a full path inside it. The folder keeps that
//! relative name: `<folder>\Data\Config\MCM.ini`.
//! - [`read`]: what a mod wrote in this run (in memory), else the folder's copy (from an earlier
//!   session), else the game's own (the run's files, then the player's Data, read only).
//! - [`write`]: into the run's memory, and into the folder when there is one (written to a
//!   `.part` name first and then renamed, so a reader never sees half of it). Without a folder
//!   (no `NV_SAVE_OUT` or `NV_MOD_FILES_OUT`) a write holds for the session only.
//!
//! A name that leaves the game's folder (`..`, a drive, another full path) is never written, here
//! or anywhere; its reads go to the run's files as any other read. The host chooses the folder
//! (`host::mod_files`, refused inside the player's Documents and AppData and the game's folder)
//! and hands it over with [`set_folder`].
//!
//! Commands run on main, so nothing here takes a lock of its own; the state is the platform
//! layer's (`Kernel::platform`), beside the run's files.

use crate::{files, Engine};
use engine_core::mod_files::ModFileCalls;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The calls the core's seam reaches (`engine_core::mod_files::install`, with the layer).
pub static CALLS: ModFileCalls = ModFileCalls {
    read,
    exists,
    directory_exists,
    create_directory,
    write,
};

/// The mod files of this run: the folder they are kept in and what mods wrote in this run.
#[derive(Debug, Default)]
pub struct ModFiles {
    /// The port's folder for mods' files (none: writes last for the session).
    pub folder: Option<PathBuf>,
    /// Files written in this run, by [`relative`] name (ASCII lower case, `\`).
    pub written: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Folders made in this run ([`create_directory`]), by the same keys.
    pub folders: BTreeSet<Vec<u8>>,
}

/// The folder mods' files are kept in from now on (the host's choice; `None`: none).
pub fn set_folder(e: &mut Engine, folder: Option<PathBuf>) {
    state!(e).mod_files.folder = folder;
}

/// The folder mods' files are kept in, if any.
pub fn folder(e: &Engine) -> Option<PathBuf> {
    state_ref!(e).mod_files.folder.clone()
}

/// `name` relative to the game's folder, `\` for `/`, without `.` parts: `None` for a name that
/// leaves it (a part `..`, a drive, a root, a full path elsewhere) or names no file.
pub fn relative(e: &Engine, name: &[u8]) -> Option<Vec<u8>> {
    let name: Vec<u8> = name
        .iter()
        .map(|&b| if b == b'/' { b'\\' } else { b })
        .collect();
    let mut rest: &[u8] = &name;
    if files::is_full_path(&name) {
        let mut game = e.kernel.current_directory.clone();
        if !game.ends_with(b"\\") {
            game.push(b'\\');
        }
        if name.len() <= game.len() || !name[..game.len()].eq_ignore_ascii_case(&game) {
            return None;
        }
        rest = &name[game.len()..];
    }
    if rest.starts_with(b"\\") || rest.contains(&b':') || rest.ends_with(b"\\") {
        return None;
    }
    let mut parts: Vec<&[u8]> = Vec::new();
    for part in rest.split(|&b| b == b'\\') {
        match part {
            b"" | b"." => {}
            b".." => return None,
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join(&b'\\'))
}

/// The key of a relative name in [`ModFiles`]: lower case.
fn key(rel: &[u8]) -> Vec<u8> {
    rel.to_ascii_lowercase()
}

/// The folder's copy of a relative name.
fn kept_path(folder: &Path, rel: &[u8]) -> PathBuf {
    folder.join(files::host_path(rel))
}

/// The bytes of the mod file `name`: written in this run, else the folder's copy, else the game's
/// (the run's files, then the player's, read only); `None` when there is none.
pub fn read(e: &Engine, name: &[u8]) -> Option<Vec<u8>> {
    let Some(rel) = relative(e, name) else {
        return files::read_whole(e, &files::full_path(e, name));
    };
    let mods = &state_ref!(e).mod_files;
    if let Some(bytes) = mods.written.get(&key(&rel)) {
        return Some(bytes.clone());
    }
    if let Some(folder) = &mods.folder {
        if let Ok(bytes) = std::fs::read(kept_path(folder, &rel)) {
            return Some(bytes);
        }
    }
    files::read_whole(e, &files::full_path(e, &rel))
}

/// Whether the mod file `name` exists ([`read`]'s places).
pub fn exists(e: &Engine, name: &[u8]) -> bool {
    let Some(rel) = relative(e, name) else {
        return files::read_whole(e, &files::full_path(e, name)).is_some();
    };
    let mods = &state_ref!(e).mod_files;
    if mods.written.contains_key(&key(&rel)) {
        return true;
    }
    if let Some(folder) = &mods.folder {
        if kept_path(folder, &rel).is_file() {
            return true;
        }
    }
    files::read_whole(e, &files::full_path(e, &rel)).is_some()
}

/// Whether the folder `name` exists: made in this run, in the port's folder, or in the game's.
pub fn directory_exists(e: &Engine, name: &[u8]) -> bool {
    let Some(rel) = relative(e, name) else {
        return files::is_directory(e, &files::full_path(e, name));
    };
    let mods = &state_ref!(e).mod_files;
    if mods.folders.contains(&key(&rel)) {
        return true;
    }
    if let Some(folder) = &mods.folder {
        if kept_path(folder, &rel).is_dir() {
            return true;
        }
    }
    files::is_directory(e, &files::full_path(e, &rel))
}

/// `CreateDirectoryA` for a mod's folder: recorded in this run, and made in the port's folder
/// when there is one; false for a name outside the game's folder (nothing made).
pub fn create_directory(e: &mut Engine, name: &[u8]) -> bool {
    let Some(rel) = relative(e, name) else {
        return false;
    };
    let folder = state_ref!(e).mod_files.folder.clone();
    state!(e).mod_files.folders.insert(key(&rel));
    if let Some(folder) = folder {
        let target = kept_path(&folder, &rel);
        if let Err(err) = std::fs::create_dir_all(&target) {
            println!("mod files: {} not made: {err}", target.display());
        }
    }
    true
}

/// Writes the mod file `name` whole: into the run's memory, and into the port's folder when
/// there is one. False (nothing written) for a name outside the game's folder. A folder write
/// that fails is said in the log; the run keeps the bytes.
pub fn write(e: &mut Engine, name: &[u8], bytes: &[u8]) -> bool {
    let Some(rel) = relative(e, name) else {
        return false;
    };
    let folder = state_ref!(e).mod_files.folder.clone();
    state!(e)
        .mod_files
        .written
        .insert(key(&rel), bytes.to_vec());
    if let Some(folder) = folder {
        let target = kept_path(&folder, &rel);
        let mut part = target.clone().into_os_string();
        part.push(".part");
        let done = target
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&part, bytes))
            .and_then(|()| std::fs::rename(&part, &target));
        if let Err(err) = done {
            println!("mod files: {} not written: {err}", target.display());
        }
    }
    true
}

#[cfg(test)]
#[path = "mod_files_tests.rs"]
mod tests;
