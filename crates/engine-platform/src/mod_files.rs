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
//! - [`append`]: only the new bytes, added to the folder's copy (made once, whole, when the folder
//!   has none); [`read_head`]: only a file's first bytes. A log a mod adds to every frame costs
//!   the same each frame however long it grows (REVIEW-643).
//! - Writes refuse (`ERROR_ACCESS_DENIED`) a name with a byte above 0x7F or a last part ending in
//!   `.part` (see `writable_as`); every call refuses a part ending in a dot or a space.
//! - The folder a file's name says must exist somewhere (made in this run, in the port's folder,
//!   or in the game's), as Windows answers `ERROR_PATH_NOT_FOUND` otherwise; in the port's folder
//!   it is then made.
//!
//! A `..` takes off the part before it, as Windows resolves a path (`Data\Config\..\x.ini` is
//! `Data\x.ini`, kept and read back under that name). Refused (never read, looked for or written:
//! a read answers as for a file that is not there, `ERROR_FILE_NOT_FOUND`, the existence checks
//! false, a write `ERROR_ACCESS_DENIED`): a name that leaves the game's folder (a `..` past it,
//! another drive, a UNC or device path, another full path), and a part, anywhere in the name,
//! that names a DOS device (`CON`, `PRN`, `AUX`, `NUL`, `COM0`-`COM9`, `LPT0`-`LPT9`, the
//! superscript digits too, in the code page and in UTF-8, `CONIN$`, `CONOUT$`; with any
//! extension). Before every write into a folder of the port's folder, the real path of the part
//! that exists (junctions, `subst`, short names resolved) must lie inside the port folder's real
//! path (checked each time: a folder swapped for a junction after a first write is refused). The
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
    append,
    read_head,
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
    let mods = &mut state!(e).mod_files;
    mods.folder = folder;
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
    /// A part ends in a dot or a space, which Windows takes off when it opens the file (`a.txt.`
    /// is `a.txt`): neither read nor written, so one disk file never has two names in this
    /// run's memory (REVIEW-643).
    Folded,
}

/// A name a write takes ([`relative_as`]), with the rules for writes alone, all
/// `ERROR_ACCESS_DENIED`: a byte above 0x7F anywhere (Windows folds the case of such letters in
/// its own way, which this run's memory, keyed by ASCII case, would not follow: two spellings of
/// one disk file would hold two contents), and a last part ending in `.part` in any case (the
/// name a write goes through, [`new_part`]: a mod's own `x.txt.part` would be taken away by a
/// later write of `x.txt`). Reads of such names are not refused: the disk answers them.
fn writable_as(e: &Engine, name: &[u8], folder_call: bool) -> Result<Vec<u8>, u32> {
    let rel = relative_as(e, name, folder_call).map_err(|_| ERROR_ACCESS_DENIED)?;
    let last = rel.rsplit(|&b| b == b'\\').next().unwrap_or(&rel);
    if !rel.is_ascii() || last.to_ascii_lowercase().ends_with(PART.as_bytes()) {
        return Err(ERROR_ACCESS_DENIED);
    }
    Ok(rel)
}

/// What a write's temporary file adds to the name.
const PART: &str = ".part";

/// Whether one part of a name is a DOS device's name: the text before its first `.`, without
/// trailing spaces and dots, compared without case (the superscript digits as the ANSI code
/// page has them, `0xB9`, `0xB2`, `0xB3`, and in UTF-8, `C2 B9`, `C2 B2`, `C2 B3`).
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
        // The same superscripts spelt in UTF-8 (`C2 B9`...), which the file layer reads as UTF-8
        // when the part is valid UTF-8 (`files::real_path`): the same device (REVIEW-642b).
        [b'C', b'O', b'M', 0xc2, d] | [b'L', b'P', b'T', 0xc2, d] => {
            matches!(d, 0xb9 | 0xb2 | 0xb3)
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
    // A device name in any part refuses the name, also in a part a later `..` takes off (so no
    // path to a device is ever opened, whatever the `..`s do).
    if rest
        .split(|&b| b == b'\\')
        .any(|part| !matches!(part, b"" | b"." | b"..") && is_device(part))
    {
        return Err(NotMine::Device);
    }
    // `..` takes off the part before it, as Windows resolves a path (`Data\Config\..\x.ini` is
    // `Data\x.ini`); one that would leave the game's folder refuses the name.
    let mut parts: Vec<&[u8]> = Vec::new();
    for part in rest.split(|&b| b == b'\\') {
        match part {
            b"" | b"." => {}
            b".." => {
                if parts.pop().is_none() {
                    return Err(NotMine::Outside);
                }
            }
            _ if part.ends_with(b".") || part.ends_with(b" ") => return Err(NotMine::Folded),
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
/// A name [`relative`] refuses reads nothing, as a file that is not there:
/// `ERROR_FILE_NOT_FOUND` (an extender command then answers as for a missing file).
pub fn read(e: &Engine, name: &[u8]) -> Result<Vec<u8>, u32> {
    assert_locked(e);
    let rel = relative(e, name).map_err(|_| ERROR_FILE_NOT_FOUND)?;
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
/// resolved), so nothing is made through a link that leads out. Checked again on every call (no
/// folder is trusted for the session: one swapped for a junction later is refused, REVIEW-642b).
fn confined(folder: &Path, target: &Path) -> Result<(), u32> {
    std::fs::create_dir_all(folder).map_err(|err| code_of(&err))?;
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
    Ok(())
}

/// The `.part` file a write goes through, made new: a plain file left there (an earlier write
/// that stopped) is taken away first; anything else of that name (a link, a junction, a folder)
/// refuses the write, `ERROR_ACCESS_DENIED`, and is left as it is. Made with `create_new`, so an
/// entry that appears meanwhile is never opened (no link is ever written through).
fn new_part(part: &std::ffi::OsStr) -> Result<std::fs::File, u32> {
    match std::fs::symlink_metadata(part) {
        Ok(meta) if meta.file_type().is_file() => {
            std::fs::remove_file(part).map_err(|err| code_of(&err))?;
        }
        Ok(_) => {
            println!(
                "mod files: {} is not a plain file; nothing written through it",
                Path::new(part).display()
            );
            return Err(ERROR_ACCESS_DENIED);
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(code_of(&err)),
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(part)
        .map_err(|err| code_of(&err))
}

/// `CreateDirectoryA` for a mod's folder (one trailing separator allowed): made in the port's
/// folder when there is one, and recorded in this run. `ERROR_ALREADY_EXISTS` when it exists,
/// `ERROR_PATH_NOT_FOUND` when its parent exists nowhere, `ERROR_ACCESS_DENIED` for a name
/// outside the game's folder or a device, or the port folder's own error; nothing made then.
pub fn create_directory(e: &mut Engine, name: &[u8]) -> Result<(), u32> {
    assert_locked(e);
    let rel = writable_as(e, name, true)?;
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
        if let Err(code) = confined(&folder, &target) {
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
    let rel = writable_file(e, name)?;
    write_rel(e, &rel, bytes)
}

/// A file name a write or an append takes ([`writable_as`]), whose own name is not a folder's
/// (`ERROR_ACCESS_DENIED`) and whose folder exists somewhere (else `ERROR_PATH_NOT_FOUND`).
fn writable_file(e: &Engine, name: &[u8]) -> Result<Vec<u8>, u32> {
    let rel = writable_as(e, name, false)?;
    if folder_exists(e, &rel) {
        return Err(ERROR_ACCESS_DENIED);
    }
    if let Some(parent) = parent_of(&rel) {
        if !folder_exists(e, parent) {
            return Err(ERROR_PATH_NOT_FOUND);
        }
    }
    Ok(rel)
}

/// [`write`] of a name [`writable_file`] took.
fn write_rel(e: &mut Engine, rel: &[u8], bytes: &[u8]) -> Result<(), u32> {
    if let Some(folder) = state_ref!(e).mod_files.folder.clone() {
        let target = kept_path(&folder, rel);
        let parent = target.parent().unwrap_or(&folder).to_path_buf();
        let mut part = target.clone().into_os_string();
        part.push(PART);
        let mut made = false;
        let done = confined(&folder, &parent)
            .and_then(|()| new_part(&part))
            .and_then(|mut file| {
                made = true;
                use std::io::Write as _;
                file.write_all(bytes)
                    .and_then(|()| {
                        drop(file);
                        std::fs::rename(&part, &target)
                    })
                    .map_err(|err| code_of(&err))
            });
        if let Err(code) = done {
            // Only a `.part` this call made is taken away.
            if made {
                let _ = std::fs::remove_file(&part);
            }
            println!(
                "mod files: {} not written (Windows error {code})",
                target.display()
            );
            return Err(code);
        }
    }
    state!(e).mod_files.written.insert(key(rel), bytes.to_vec());
    Ok(())
}

/// `fopen(name, "ab")` and one write of `bytes`: only the new bytes are written. With a port
/// folder, the folder's copy is opened for appending (after the same names, folders and real
/// path checks as [`write`], made each time), and this run's memory forgets its whole copy (the
/// disk's answers from then on, so nothing whole is kept or rewritten); a copy that is a link or
/// a reparse point is refused, `ERROR_ACCESS_DENIED`, and nothing is written through it. When the
/// folder has no copy yet, it is made once, whole, through [`write`]: what [`read`] answers (this
/// run's write, else the game's copy; none: empty) and then the bytes. Without a folder the
/// run's copy grows in memory (seeded once the same way). A failed append leaves the file as it
/// was (a partly written end is cut off again) and fails with Windows' code: a copy that is there
/// but cannot be opened (held by another program, unreadable) is never replaced.
pub fn append(e: &mut Engine, name: &[u8], bytes: &[u8]) -> Result<(), u32> {
    assert_locked(e);
    let rel = writable_file(e, name)?;
    let k = key(&rel);
    let Some(folder) = state_ref!(e).mod_files.folder.clone() else {
        if !state_ref!(e).mod_files.written.contains_key(&k) {
            let seed = seed(e, &rel)?;
            state!(e).mod_files.written.insert(k.clone(), seed);
        }
        if let Some(copy) = state!(e).mod_files.written.get_mut(&k) {
            copy.extend_from_slice(bytes);
        }
        return Ok(());
    };
    let target = kept_path(&folder, &rel);
    let parent = target.parent().unwrap_or(&folder).to_path_buf();
    let done = confined(&folder, &parent).and_then(|()| appended_kept(&target, bytes));
    match done {
        // Appended on disk: the disk's copy answers from now on.
        Ok(true) => {
            state!(e).mod_files.written.remove(&k);
            Ok(())
        }
        // No copy in the folder yet: made once, whole.
        Ok(false) => {
            let mut whole = seed(e, &rel)?;
            whole.extend_from_slice(bytes);
            write_rel(e, &rel, &whole)
        }
        Err(code) => {
            println!(
                "mod files: {} not appended to (Windows error {code})",
                target.display()
            );
            Err(code)
        }
    }
}

/// The folder's copy `target` appended to when it is a plain file (true); false when it is not
/// there; anything else of that name (a link, a junction) refuses the append.
fn appended_kept(target: &Path, bytes: &[u8]) -> Result<bool, u32> {
    match std::fs::symlink_metadata(target) {
        Ok(meta) if meta.file_type().is_file() => append_to(target, bytes).map(|()| true),
        Ok(_) => {
            println!(
                "mod files: {} is not a plain file; nothing written through it",
                target.display()
            );
            Err(ERROR_ACCESS_DENIED)
        }
        Err(err) if missing(code_of(&err)) => Ok(false),
        Err(err) => Err(code_of(&err)),
    }
}

/// What an append starts from when the port's folder has no copy: [`read`]'s answer (this
/// run's write, else the game's copy), empty when there is none; a copy that is there but cannot
/// be read fails the append (it is never replaced by one made from nothing).
fn seed(e: &Engine, rel: &[u8]) -> Result<Vec<u8>, u32> {
    match read(e, rel) {
        Ok(bytes) => Ok(bytes),
        Err(code) if missing(code) => Ok(Vec::new()),
        Err(code) => Err(code),
    }
}

/// The folder's copy `target` (a plain file just looked at) opened for appending and `bytes`
/// added. On Windows the open does not follow a reparse point and one is refused (so a link put
/// there after the look is not written through); elsewhere the look before stands for it. A write
/// that fails cuts the file back to its old length.
fn append_to(target: &Path, bytes: &[u8]) -> Result<(), u32> {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.append(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_FLAG_OPEN_REPARSE_POINT: the entry itself, not where a link leads.
        options.custom_flags(0x0020_0000);
    }
    let mut file = options.open(target).map_err(|err| code_of(&err))?;
    let meta = file.metadata().map_err(|err| code_of(&err))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        // FILE_ATTRIBUTE_REPARSE_POINT
        if meta.file_attributes() & 0x400 != 0 {
            println!(
                "mod files: {} is a link; nothing written through it",
                target.display()
            );
            return Err(ERROR_ACCESS_DENIED);
        }
    }
    if !meta.is_file() {
        return Err(ERROR_ACCESS_DENIED);
    }
    let old = meta.len();
    if let Err(err) = file.write_all(bytes).and_then(|()| file.flush()) {
        let _ = file.set_len(old);
        return Err(code_of(&err));
    }
    Ok(())
}

/// The first `max` bytes of the mod file `name`, from [`read`]'s places with its answers; only
/// those bytes are read from the disk.
pub fn read_head(e: &Engine, name: &[u8], max: usize) -> Result<Vec<u8>, u32> {
    assert_locked(e);
    let rel = relative(e, name).map_err(|_| ERROR_FILE_NOT_FOUND)?;
    let mods = &state_ref!(e).mod_files;
    if let Some(bytes) = mods.written.get(&key(&rel)) {
        return Ok(bytes[..bytes.len().min(max)].to_vec());
    }
    if let Some(folder) = &mods.folder {
        match files::read_head_of(&kept_path(folder, &rel), max) {
            Ok(bytes) => return Ok(bytes),
            Err(err) if missing(code_of(&err)) => {}
            Err(err) => return Err(code_of(&err)),
        }
    }
    files::read_head_or_error(e, &files::full_path(e, &rel), max)
}

#[cfg(test)]
#[path = "mod_files_tests.rs"]
mod tests;
