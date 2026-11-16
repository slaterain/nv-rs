//! Mods' own files: the files a script extender's commands read and write for a mod (MCM's
//! `Data\Config\<mod>.ini`, later JIP LN's INI and file commands), kept between sessions in a
//! folder of the port's own beside the saves (the maintainer, 2026-11-16), never in the game's
//! folder. The extender crates reach these through the core's seam (`engine_core::mod_files`,
//! installed with the layer: [`CALLS`]), which also holds the profile text rules (`ini_value`,
//! `written_profile`) and takes the kernel lock around each call: these functions reach the
//! platform layer's state, which other threads change under that lock.
//!
//! A name is given as the game would pass it to Windows: relative to the game's folder (the
//! current directory, `Data\Config\MCM.ini`), or a full path inside it. The folder keeps that
//! relative name: `<folder>\Data\Config\MCM.ini`.
//! - [`read`]: what a mod wrote in this run (in memory), else the folder's copy (from an earlier
//!   session), else the game's own (the run's files, then the player's Data, read only). Only a
//!   copy that is not there passes on to the next place: one that is there but cannot be read
//!   fails the read.
//! - [`write`]: into the folder when there is one (a `.part` name first, then renamed, so a reader
//!   never sees half of it), then into the run's memory; a failed folder write fails the call and
//!   changes nothing. Without a folder (no `NV_SAVE_OUT` or `NV_MOD_FILES_OUT`) a write lives in
//!   the run's memory only: mods' settings last for the session, and nothing touches the disk.
//! - The folder a file's name says must exist somewhere (made in this run, in the port's folder,
//!   or in the game's), as Windows answers `ERROR_PATH_NOT_FOUND` otherwise; in the port's folder
//!   it is then made.
//!
//! Refused (never read, looked for or written: a read fails with `ERROR_ACCESS_DENIED`, the
//! existence checks answer false): a name that leaves the game's folder (`..`, another drive, a
//! UNC or device path, another full path), and a part that names a DOS device (`CON`, `PRN`, `AUX`, `NUL`, `COM0`-`COM9`, `LPT0`-`LPT9`, the
//! superscript digits too, `CONIN$`, `CONOUT$`; with any extension). Before the first write into
//! a folder of the port's folder, the real path of the part that exists (junctions, `subst`, short
//! names resolved) must lie inside the port folder's real path; the folders checked are kept. The
//! host chooses the folder (`host::mod_files`, refused inside the player's Documents and AppData
//! and the game's folder) and hands it over with [`set_folder`].

use crate::{files, Engine};
use engine_core::mod_files::{
    ModFileCalls, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND,
    ERROR_PATH_NOT_FOUND,
};
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
    /// Folders of the port's folder whose real path was found inside it ([`confined`]).
    pub checked: BTreeSet<PathBuf>,
}

/// The folder mods' files are kept in from now on (the host's choice; `None`: none).
pub fn set_folder(e: &mut Engine, folder: Option<PathBuf>) {
    let mods = &mut state!(e).mod_files;
    mods.folder = folder;
    mods.checked.clear();
}

/// The folder mods' files are kept in, if any.
pub fn folder(e: &Engine) -> Option<PathBuf> {
    state_ref!(e).mod_files.folder.clone()
}

/// Why a name is not a mod file's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotMine {
    /// It leaves the game's folder (or names no file): neither read nor written.
    Outside,
    /// A part names a DOS device: neither read nor written.
    Device,
}

/// Whether one part of a name is a DOS device's name: the text before its first `.`, without
/// trailing spaces and dots, compared without case (the superscript digits as the ANSI code
/// page has them, `0xB9`, `0xB2`, `0xB3`).
pub fn is_device(part: &[u8]) -> bool {
    let base = part.split(|&b| b == b'.').next().unwrap_or(part);
    let end = base
        .iter()
        .rposition(|&b| b != b' ' && b != b'.')
        .map_or(0, |i| i + 1);
    let base = base[..end].to_ascii_uppercase();
    match base.as_slice() {
        b"CON" | b"PRN" | b"AUX" | b"NUL" | b"CONIN$" | b"CONOUT$" => true,
        [b'C', b'O', b'M', d] | [b'L', b'P', b'T', d] => {
            d.is_ascii_digit() || matches!(d, 0xb9 | 0xb2 | 0xb3)
        }
        _ => false,
    }
}

/// `name` relative to the game's folder, `\` for `/`, without `.` parts. With `folder_call`
/// (the directory calls) one trailing separator is allowed, as Windows allows it.
pub fn relative_as(e: &Engine, name: &[u8], folder_call: bool) -> Result<Vec<u8>, NotMine> {
    let mut name: Vec<u8> = name
        .iter()
        .map(|&b| if b == b'/' { b'\\' } else { b })
        .collect();
    if folder_call && name.len() > 1 && name.ends_with(b"\\") {
        name.pop();
    }
    let mut rest: &[u8] = &name;
    if files::is_full_path(&name) {
        let mut game = e.kernel.current_directory.clone();
        if !game.ends_with(b"\\") {
            game.push(b'\\');
        }
        if name.len() <= game.len() || !name[..game.len()].eq_ignore_ascii_case(&game) {
            return Err(NotMine::Outside);
        }
        rest = &name[game.len()..];
    }
    if rest.starts_with(b"\\") || rest.contains(&b':') || rest.ends_with(b"\\") {
        return Err(NotMine::Outside);
    }
    let mut parts: Vec<&[u8]> = Vec::new();
    for part in rest.split(|&b| b == b'\\') {
        match part {
            b"" | b"." => {}
            b".." => return Err(NotMine::Outside),
            _ if is_device(part) => return Err(NotMine::Device),
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        return Err(NotMine::Outside);
    }
    Ok(parts.join(&b'\\'))
}

/// [`relative_as`] for a file.
pub fn relative(e: &Engine, name: &[u8]) -> Result<Vec<u8>, NotMine> {
    relative_as(e, name, false)
}

/// The key of a relative name in [`ModFiles`]: lower case.
fn key(rel: &[u8]) -> Vec<u8> {
    rel.to_ascii_lowercase()
}

/// The folder part of a relative name, if it has one.
fn parent_of(rel: &[u8]) -> Option<&[u8]> {
    rel.iter().rposition(|&b| b == b'\\').map(|at| &rel[..at])
}

/// The folder's copy of a relative name.
fn kept_path(folder: &Path, rel: &[u8]) -> PathBuf {
    folder.join(files::host_path(rel))
}

/// A host file call's failure as Windows' code (`ERROR_ACCESS_DENIED` when it gives none).
fn code_of(err: &std::io::Error) -> u32 {
    if let Some(code) = err.raw_os_error() {
        return code as u32;
    }
    match err.kind() {
        std::io::ErrorKind::NotFound => ERROR_FILE_NOT_FOUND,
        _ => ERROR_ACCESS_DENIED,
    }
}

/// Whether a failure says the file is not there (so the next place is looked at).
fn missing(code: u32) -> bool {
    code == ERROR_FILE_NOT_FOUND || code == ERROR_PATH_NOT_FOUND
}

fn assert_locked(e: &Engine) {
    debug_assert!(
        engine_core::real_threads::kernel_lock_held(e),
        "mod files reached without the kernel lock (engine_core::mod_files takes it)"
    );
}

/// The bytes of the mod file `name`: written in this run, else the folder's copy, else the
/// game's (the run's files, then the player's, read only). A copy that is there but cannot be
/// read (access denied, held by another program, a folder of that name) fails with its error.
/// A name [`relative`] refuses reads nothing: `ERROR_ACCESS_DENIED`.
pub fn read(e: &Engine, name: &[u8]) -> Result<Vec<u8>, u32> {
    assert_locked(e);
    let rel = relative(e, name).map_err(|_| ERROR_ACCESS_DENIED)?;
    let mods = &state_ref!(e).mod_files;
    if let Some(bytes) = mods.written.get(&key(&rel)) {
        return Ok(bytes.clone());
    }
    if let Some(folder) = &mods.folder {
        match std::fs::read(kept_path(folder, &rel)) {
            Ok(bytes) => return Ok(bytes),
            Err(err) if missing(code_of(&err)) => {}
            Err(err) => return Err(code_of(&err)),
        }
    }
    files::read_whole_or_error(e, &files::full_path(e, &rel))
}

/// Whether the mod file `name` exists in [`read`]'s places, from attributes alone (the file's
/// bytes are not read): a file, not a folder; a game file the run deleted is not there. A name
/// [`relative`] refuses is not looked for: false.
pub fn exists(e: &Engine, name: &[u8]) -> bool {
    assert_locked(e);
    let is_file = |a: Option<u32>| a.is_some_and(|a| a & files::FILE_ATTRIBUTE_DIRECTORY == 0);
    let Ok(rel) = relative(e, name) else {
        return false;
    };
    let mods = &state_ref!(e).mod_files;
    if mods.written.contains_key(&key(&rel)) {
        return true;
    }
    if let Some(folder) = &mods.folder {
        // What is there in the port's folder answers (a folder there hides the game's file, as
        // it fails the read).
        if let Ok(meta) = std::fs::metadata(kept_path(folder, &rel)) {
            return meta.is_file();
        }
    }
    is_file(files::attributes(e, &files::full_path(e, &rel)))
}

/// [`directory_exists`] of a relative name.
fn folder_exists(e: &Engine, rel: &[u8]) -> bool {
    let mods = &state_ref!(e).mod_files;
    if mods.folders.contains(&key(rel)) {
        return true;
    }
    if let Some(folder) = &mods.folder {
        if kept_path(folder, rel).is_dir() {
            return true;
        }
    }
    files::is_directory(e, &files::full_path(e, rel))
}

/// Whether the folder `name` exists: made in this run, in the port's folder, or in the game's
/// (one trailing separator allowed). A name [`relative_as`] refuses is not looked for: false.
pub fn directory_exists(e: &Engine, name: &[u8]) -> bool {
    assert_locked(e);
    relative_as(e, name, true).is_ok_and(|rel| folder_exists(e, &rel))
}

/// Makes `target`'s folders in the port's folder `folder`, after checking that the real path of
/// the part that exists lies inside `folder`'s real path (junctions, `subst` and short names
/// resolved), so nothing is made through a link that leads out; the folders checked are kept.
fn confined(e: &mut Engine, folder: &Path, target: &Path) -> Result<(), u32> {
    std::fs::create_dir_all(folder).map_err(|err| code_of(&err))?;
    if state_ref!(e).mod_files.checked.contains(target) {
        return Ok(());
    }
    let root = std::fs::canonicalize(folder).map_err(|err| code_of(&err))?;
    let mut existing = target.to_path_buf();
    while !existing.exists() {
        if !existing.pop() {
            return Err(ERROR_ACCESS_DENIED);
        }
    }
    let real = std::fs::canonicalize(&existing).map_err(|err| code_of(&err))?;
    if !real.starts_with(&root) {
        println!(
            "mod files: {} leads outside {} ({}); nothing written there",
            existing.display(),
            folder.display(),
            real.display()
        );
        return Err(ERROR_ACCESS_DENIED);
    }
    std::fs::create_dir_all(target).map_err(|err| code_of(&err))?;
    state!(e).mod_files.checked.insert(target.to_path_buf());
    Ok(())
}

/// `CreateDirectoryA` for a mod's folder (one trailing separator allowed): made in the port's
/// folder when there is one, and recorded in this run. `ERROR_ALREADY_EXISTS` when it exists,
/// `ERROR_PATH_NOT_FOUND` when its parent exists nowhere, `ERROR_ACCESS_DENIED` for a name
/// outside the game's folder or a device, or the port folder's own error; nothing made then.
pub fn create_directory(e: &mut Engine, name: &[u8]) -> Result<(), u32> {
    assert_locked(e);
    let rel = relative_as(e, name, true).map_err(|_| ERROR_ACCESS_DENIED)?;
    if folder_exists(e, &rel) {
        return Err(ERROR_ALREADY_EXISTS);
    }
    if let Some(parent) = parent_of(&rel) {
        if !folder_exists(e, parent) {
            return Err(ERROR_PATH_NOT_FOUND);
        }
    }
    if let Some(folder) = state_ref!(e).mod_files.folder.clone() {
        let target = kept_path(&folder, &rel);
        if let Err(code) = confined(e, &folder, &target) {
            println!(
                "mod files: {} not made (Windows error {code})",
                target.display()
            );
            return Err(code);
        }
    }
    state!(e).mod_files.folders.insert(key(&rel));
    Ok(())
}

/// Writes the mod file `name` whole: into the port's folder when there is one, then into the
/// run's memory. `ERROR_PATH_NOT_FOUND` when its folder exists nowhere, `ERROR_ACCESS_DENIED` for
/// a folder of that name, a name outside the game's folder or a device, or the port folder's own
/// error (disk full, access denied): nothing written then, and no `.part` left.
pub fn write(e: &mut Engine, name: &[u8], bytes: &[u8]) -> Result<(), u32> {
    assert_locked(e);
    let rel = relative(e, name).map_err(|_| ERROR_ACCESS_DENIED)?;
    if folder_exists(e, &rel) {
        return Err(ERROR_ACCESS_DENIED);
    }
    if let Some(parent) = parent_of(&rel) {
        if !folder_exists(e, parent) {
            return Err(ERROR_PATH_NOT_FOUND);
        }
    }
    if let Some(folder) = state_ref!(e).mod_files.folder.clone() {
        let target = kept_path(&folder, &rel);
        let parent = target.parent().unwrap_or(&folder).to_path_buf();
        let mut part = target.clone().into_os_string();
        part.push(".part");
        let done = confined(e, &folder, &parent).and_then(|()| {
            std::fs::write(&part, bytes)
                .and_then(|()| std::fs::rename(&part, &target))
                .map_err(|err| code_of(&err))
        });
        if let Err(code) = done {
            let _ = std::fs::remove_file(&part);
            println!(
                "mod files: {} not written (Windows error {code})",
                target.display()
            );
            return Err(code);
        }
    }
    state!(e)
        .mod_files
        .written
        .insert(key(&rel), bytes.to_vec());
    Ok(())
}

#[cfg(test)]
#[path = "mod_files_tests.rs"]
mod tests;
