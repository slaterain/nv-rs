//! The archives mods come in: their file listing, one file read, and chosen files unpacked into a
//! folder. Used by the FOMOD installer (`crates/fomod` plans which files go where; this does the
//! reading). The same on Windows, Linux and macOS: plain Rust, no system archive library.
//!
//! - The format is told by the first bytes, never the name (`.fomod` files are 7z or zip):
//!   zip `PK\3\4` (or an empty zip's `PK\5\6`), 7z `7z\xBC\xAF\x27\x1C`, RAR `Rar!\x1A\x07`.
//! - zip: Stored, Deflate and Deflate64. 7z: LZMA, LZMA2, BCJ/BCJ2 and PPMd, solid or not. A
//!   solid 7z block decodes from its start, so [`extract`] takes everything wanted in one pass
//!   over the archive, never file by file.
//! - RAR is not read (the lead's decision, 2026-11-17: its one complete reader, RARLab's UnRAR,
//!   has a licence that is not open source); the player is told to repack it as 7z or zip.
//!   Encrypted archives and other methods are refused, in words a player can act on.
//! - Paths inside an archive may use `\` or `/`; they are compared without case, as the game
//!   compares them. Files are written only under the folder given, by the relative paths given:
//!   one with `..`, a drive or a leading separator is refused before anything is written. On a
//!   system whose files are told apart by case (Linux; macOS when so formatted), a folder already
//!   there under another case is used, so `Textures` and `textures` stay one folder.
//! - A downloaded archive may be hostile. Names in it are only ever looked up, never used as paths
//!   (no "zip slip"); links stored in it are never unpacked ([`list`] leaves them out); a link
//!   already in the destination is never written through; a destination with `..`, a `:`, a
//!   leading separator or a Windows device name (`CON`, `COM1`...) is refused on every system;
//!   sizes are checked against [`Limits`] before anything is written, and a file that unpacks
//!   to more than its archive said stops the unpacking (a "bomb"); of two names differing only in
//!   case the first is taken; [`read`] returns at most [`MAX_READ`] bytes.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// An archive's format, from its first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    SevenZip,
    Rar,
    Unknown,
}

/// The format of an archive starting with `head` (its first 8 bytes are enough).
pub fn format_of(head: &[u8]) -> Format {
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        Format::Zip
    } else if head.starts_with(b"7z\xBC\xAF\x27\x1C") {
        Format::SevenZip
    } else if head.starts_with(b"Rar!\x1A\x07") {
        Format::Rar
    } else {
        Format::Unknown
    }
}

/// What went wrong, by what the player can do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// A RAR archive.
    Rar,
    /// Not an archive this reads (or not an archive).
    NotAnArchive,
    /// It needs a password.
    Encrypted,
    /// A compression method not read here.
    Unsupported,
    /// Broken or cut short.
    Damaged,
    /// The file could not be opened, read or written.
    Io,
    /// A destination that would leave the folder, or a link in the way.
    Outside,
    /// More to unpack than the limits allow (room on the drive, or a file far larger than any
    /// mod's): a broken or hostile archive.
    TooBig,
}

#[derive(Debug)]
pub struct ArchiveError {
    pub problem: Problem,
    /// The reader's own words or the path, for the log.
    pub detail: String,
}

impl ArchiveError {
    fn new(problem: Problem, detail: impl Into<String>) -> Self {
        Self {
            problem,
            detail: detail.into(),
        }
    }

    /// What the player reads.
    pub fn player_text(&self) -> String {
        let advice = match self.problem {
            Problem::Rar => "This mod is a RAR archive, which nv-rs can't open. Open it with 7-Zip or another archive program, save it again as a .7z or .zip file, and put that in the mods folder instead.",
            Problem::NotAnArchive => "This file is not a .7z or .zip archive. If it is a mod, download it again; if it is another kind of archive, save it again as .7z or .zip.",
            Problem::Encrypted => "This archive is locked with a password. nv-rs can't open locked archives: unpack it with 7-Zip and save it again without a password.",
            Problem::Unsupported => "This archive is packed in a way nv-rs can't read yet. Open it with 7-Zip and save it again as a normal .7z or .zip file.",
            Problem::Damaged => "This archive is damaged or was not downloaded completely. Download it again.",
            Problem::Io => "nv-rs could not read or write a file. Check there is room on the drive and that no other program has the file open, then try again.",
            Problem::Outside => "This mod tries to put a file outside its own folder, so nothing was installed. Please tell the mod's author.",
            Problem::TooBig => "This archive unpacks to more than nv-rs allows or than there is room for on the drive. Free some space and try again; if it still happens, the archive is damaged: download it again.",
        };
        format!("{advice}\n\n({})", self.detail)
    }
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.problem, self.detail)
    }
}

/// One file in an archive: its path as stored (with `\` or `/`), and its unpacked size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub size: u64,
}

/// One part of a path as Windows (and macOS by default) tells names apart: dots and spaces at
/// its end do not count, nor does case (Unicode lower case). The same rule as `crates/fomod`'s.
fn fold_part(part: &str) -> String {
    part.trim_end_matches(['.', ' ']).to_lowercase()
}

/// A path's key: `/` between the parts, none at either end, each part [`fold_part`]ed.
fn key(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .map(fold_part)
        .collect::<Vec<_>>()
        .join("/")
}

/// The most [`read`] returns: installers and their details are a few hundred KB at most.
pub const MAX_READ: u64 = 16 * 1024 * 1024;

/// What [`extract`] may write. The front end sets `total` from the room on the drive; a file is
/// also never more than its archive says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most one file may unpack to (the game's own largest archives are under 2 GB; TTW's LOD
    /// archives a little over).
    pub file: u64,
    /// The most all the wanted files together may unpack to.
    pub total: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file: 16 << 30,
            total: 64 << 30,
        }
    }
}

fn io(path: &Path, e: std::io::Error) -> ArchiveError {
    ArchiveError::new(Problem::Io, format!("{}: {e}", path.display()))
}

fn zip_error(e: zip::result::ZipError) -> ArchiveError {
    use zip::result::ZipError;
    let problem = match &e {
        ZipError::UnsupportedArchive(m) if *m == ZipError::PASSWORD_REQUIRED => Problem::Encrypted,
        ZipError::InvalidPassword => Problem::Encrypted,
        ZipError::UnsupportedArchive(_) | ZipError::CompressionMethodNotSupported(_) => {
            Problem::Unsupported
        }
        ZipError::Io(io) if io.kind() != std::io::ErrorKind::UnexpectedEof => Problem::Io,
        _ => Problem::Damaged,
    };
    ArchiveError::new(problem, format!("zip: {e}"))
}

fn sevenz_error(e: sevenz_rust2::Error) -> ArchiveError {
    use sevenz_rust2::Error as E;
    let problem = match &e {
        E::PasswordRequired | E::MaybeBadPassword(_) => Problem::Encrypted,
        E::UnsupportedCompressionMethod(_) | E::Unsupported(_) | E::ExternalUnsupported => {
            Problem::Unsupported
        }
        E::FileOpen(..) => Problem::Io,
        _ => Problem::Damaged,
    };
    ArchiveError::new(problem, format!("7z: {e}"))
}

fn format_of_file(path: &Path) -> Result<Format, ArchiveError> {
    let mut head = [0u8; 8];
    let mut file = File::open(path).map_err(|e| io(path, e))?;
    let mut got = 0;
    while got < head.len() {
        match file.read(&mut head[got..]).map_err(|e| io(path, e))? {
            0 => break,
            n => got += n,
        }
    }
    Ok(format_of(&head[..got]))
}

/// The format, refusing what this does not read.
fn readable(path: &Path) -> Result<Format, ArchiveError> {
    match format_of_file(path)? {
        Format::Rar => Err(ArchiveError::new(Problem::Rar, path.display().to_string())),
        Format::Unknown => Err(ArchiveError::new(
            Problem::NotAnArchive,
            path.display().to_string(),
        )),
        f => Ok(f),
    }
}

fn open_zip(path: &Path) -> Result<zip::ZipArchive<File>, ArchiveError> {
    zip::ZipArchive::new(File::open(path).map_err(|e| io(path, e))?).map_err(zip_error)
}

fn open_7z(path: &Path) -> Result<sevenz_rust2::ArchiveReader<File>, ArchiveError> {
    sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty()).map_err(sevenz_error)
}

/// A Unix mode that is a symbolic link.
fn link_mode(mode: u32) -> bool {
    mode & 0o170000 == 0o120000
}

/// Whether a 7z entry is a link: a Windows reparse point, or a Unix symbolic link (7-Zip keeps
/// the Unix mode in the high half of the attributes, marked by 0x8000).
fn sevenz_link(entry: &sevenz_rust2::ArchiveEntry) -> bool {
    if !entry.has_windows_attributes {
        return false;
    }
    let attributes = entry.windows_attributes();
    attributes & 0x400 != 0 || (attributes & 0x8000 != 0 && link_mode(attributes >> 16))
}

/// One entry of an archive as this reads it: (path, size, a plain file).
fn zip_entries(zip: &mut zip::ZipArchive<File>) -> Result<Vec<(String, u64, bool)>, ArchiveError> {
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let file = zip.by_index_raw(i).map_err(zip_error)?;
        let plain =
            !file.is_dir() && !file.is_symlink() && !file.unix_mode().is_some_and(link_mode);
        out.push((
            file.name().map_err(zip_error)?.into_owned(),
            file.size(),
            plain,
        ));
    }
    Ok(out)
}

fn sevenz_entries(reader: &sevenz_rust2::ArchiveReader<File>) -> Vec<(String, u64, bool)> {
    reader
        .archive()
        .files
        .iter()
        .map(|f| {
            let plain = !f.is_directory() && !sevenz_link(f);
            (f.name().to_owned(), f.size(), plain)
        })
        .collect()
}

/// The files in the archive at `path`, in the archive's order. Folders and links are left out:
/// a link is never unpacked.
pub fn list(path: &Path) -> Result<Vec<Entry>, ArchiveError> {
    let entries = match readable(path)? {
        Format::Zip => zip_entries(&mut open_zip(path)?)?,
        _ => sevenz_entries(&open_7z(path)?),
    };
    Ok(entries
        .into_iter()
        .filter(|(_, _, plain)| *plain)
        .map(|(path, size, _)| Entry { path, size })
        .collect())
}

/// Reads at most `limit` bytes; more is [`Problem::TooBig`].
fn read_capped(data: &mut dyn Read, limit: u64, what: &str) -> Result<Vec<u8>, ArchiveError> {
    let mut bytes = Vec::new();
    data.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ArchiveError::new(Problem::Damaged, format!("{what}: {e}")))?;
    if bytes.len() as u64 > limit {
        return Err(ArchiveError::new(
            Problem::TooBig,
            format!("{what}: more than {limit} bytes"),
        ));
    }
    Ok(bytes)
}

/// The bytes of the one file `name` (any case, `\` or `/`) in the archive at `path`, or `None`
/// when it holds no such file. For an installer's `ModuleConfig.xml` (at most [`MAX_READ`]); a
/// big file goes through [`extract`].
pub fn read(path: &Path, name: &str) -> Result<Option<Vec<u8>>, ArchiveError> {
    let wanted = key(name);
    match readable(path)? {
        Format::Zip => {
            let mut zip = open_zip(path)?;
            let entries = zip_entries(&mut zip)?;
            let Some(i) = entries
                .iter()
                .position(|(n, _, plain)| *plain && key(n) == wanted)
            else {
                return Ok(None);
            };
            let mut file = zip.by_index(i).map_err(zip_error)?;
            read_capped(&mut file, MAX_READ, name).map(Some)
        }
        _ => {
            let mut reader = open_7z(path)?;
            let mut out = None;
            let mut failure = None;
            let result = reader.for_each_entries_with_decision(|entry, data| {
                if entry.is_directory() || sevenz_link(entry) || key(entry.name()) != wanted {
                    return Ok(sevenz_rust2::EntryDecision::Continue);
                }
                match read_capped(data, MAX_READ, name) {
                    Ok(bytes) => out = Some(bytes),
                    Err(e) => failure = Some(e),
                }
                Ok(sevenz_rust2::EntryDecision::Stop)
            });
            if let Some(e) = failure {
                return Err(e);
            }
            result.map_err(sevenz_error)?;
            Ok(out)
        }
    }
}

/// What [`extract`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extracted {
    /// Files written, in the order written.
    pub written: Vec<PathBuf>,
    /// Archive paths asked for that the archive does not hold as a plain file (a link counts as
    /// not held).
    pub missing: Vec<String>,
}

/// Whether a path part is one of Windows' device names (`CON`, `NUL`, `COM1`...), with or without
/// an extension: Windows opens the device for such a name in any folder.
fn device_name(part: &str) -> bool {
    let stem = fold_part(part);
    let stem = stem.split('.').next().unwrap_or_default().trim_end();
    matches!(stem, "con" | "prn" | "aux" | "nul")
        || ((stem.starts_with("com") || stem.starts_with("lpt"))
            && stem.chars().count() == 4
            && stem
                .chars()
                .nth(3)
                .is_some_and(|c| c.is_ascii_digit() || "¹²³".contains(c)))
}

/// Whether `relative` (with `/` or `\`) stays inside the folder it is joined to, on every system:
/// no empty, `.` or `..` part, no leading separator, no `:` (drives and NTFS streams), no Windows
/// device name.
fn stays_inside(relative: &str) -> bool {
    let text = relative.replace('\\', "/");
    !text.is_empty()
        && !text.starts_with('/')
        && text.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p.trim() != ".."
                && !fold_part(p).is_empty()
                && !p.contains(':')
                && !device_name(p)
        })
}

/// `relative` under `folder`, each part an entry already there under another case if there is
/// one. A link on the way, or at the end, is refused: a file is never written through a link.
fn place(folder: &Path, relative: &str) -> Result<PathBuf, ArchiveError> {
    let parts: Vec<&str> = relative.split(['/', '\\']).collect();
    let mut at = folder.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let wanted = fold_part(part);
        let existing = std::fs::read_dir(&at).ok().and_then(|dir| {
            dir.filter_map(Result::ok)
                .find(|e| fold_part(&e.file_name().to_string_lossy()) == wanted)
        });
        match existing {
            Some(entry) => {
                let kind = entry.file_type().map_err(|e| io(&entry.path(), e))?;
                if kind.is_symlink() {
                    return Err(ArchiveError::new(
                        Problem::Outside,
                        format!("{} is a link", entry.path().display()),
                    ));
                }
                if !last && !kind.is_dir() {
                    return Err(ArchiveError::new(
                        Problem::Outside,
                        format!("{} is a file, not a folder", entry.path().display()),
                    ));
                }
                at.push(entry.file_name());
            }
            None => at.push(part),
        }
        if !last {
            std::fs::create_dir_all(&at).map_err(|e| io(&at, e))?;
        }
    }
    Ok(at)
}

/// Unpacks from the archive at `path` each wanted file: (path in the archive, `\` or `/`, any
/// case; path under `folder`, relative). One archive file may go to several places.
///
/// Before anything is written: every destination is checked ([`stays_inside`]), and the wanted
/// files' sizes as the archive gives them against `limits`. While writing: no file is written
/// through a link, and a file that unpacks to more than its archive said, or more than the
/// limits, stops the unpacking ([`Problem::TooBig`]). Of two entries whose names differ only in
/// case, the first is taken. A 7z is read only as far as the last wanted file.
pub fn extract(
    path: &Path,
    wanted: &[(String, String)],
    folder: &Path,
    limits: Limits,
) -> Result<Extracted, ArchiveError> {
    if let Some((_, bad)) = wanted.iter().find(|(_, d)| !stays_inside(d)) {
        return Err(ArchiveError::new(Problem::Outside, bad.clone()));
    }
    let mut by_source: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (source, destination) in wanted {
        by_source.entry(key(source)).or_default().push(destination);
    }
    let format = readable(path)?;
    let mut zip = None;
    let mut sevenz = None;
    let entries = match format {
        Format::Zip => zip_entries(zip.insert(open_zip(path)?))?,
        _ => sevenz_entries(sevenz.insert(open_7z(path)?)),
    };
    // Which entry each wanted file is (the first plain file of its name), and what it all comes
    // to as the archive says.
    let mut chosen: BTreeMap<String, usize> = BTreeMap::new();
    for (i, (name, _, plain)) in entries.iter().enumerate() {
        let k = key(name);
        if *plain && by_source.contains_key(&k) {
            chosen.entry(k).or_insert(i);
        }
    }
    let mut total: u64 = 0;
    for (k, &i) in &chosen {
        let size = entries[i].1;
        if size > limits.file {
            return Err(ArchiveError::new(
                Problem::TooBig,
                format!("{}: {size} bytes", entries[i].0),
            ));
        }
        total = total.saturating_add(size.saturating_mul(by_source[k].len() as u64));
    }
    if total > limits.total {
        return Err(ArchiveError::new(
            Problem::TooBig,
            format!("{total} bytes to unpack, {} allowed", limits.total),
        ));
    }
    let by_index: BTreeMap<usize, String> = chosen.iter().map(|(k, &i)| (i, k.clone())).collect();
    std::fs::create_dir_all(folder).map_err(|e| io(folder, e))?;
    let mut done = Extracted::default();
    // Writes one archive file to its destinations; the first written, the rest copied from it.
    let write =
        |index: usize, data: &mut dyn Read, done: &mut Extracted| -> Result<(), ArchiveError> {
            let source_key = &by_index[&index];
            let (name, size, _) = &entries[index];
            let destinations = &by_source[source_key];
            let first = place(folder, destinations[0])?;
            let mut file = File::create(&first).map_err(|e| io(&first, e))?;
            let written = std::io::copy(&mut data.take(size.saturating_add(1)), &mut file)
                .map_err(|e| ArchiveError::new(Problem::Damaged, format!("{name}: {e}")))?;
            drop(file);
            if written > *size {
                let _ = std::fs::remove_file(&first);
                return Err(ArchiveError::new(
                    Problem::TooBig,
                    format!("{name} unpacks to more than the {size} bytes its archive says"),
                ));
            }
            done.written.push(first.clone());
            for destination in &destinations[1..] {
                let other = place(folder, destination)?;
                std::fs::copy(&first, &other).map_err(|e| io(&other, e))?;
                done.written.push(other);
            }
            Ok(())
        };
    if let Some(zip) = zip.as_mut() {
        for &index in by_index.keys() {
            let mut file = zip.by_index(index).map_err(zip_error)?;
            write(index, &mut file, &mut done)?;
        }
    } else if let Some(reader) = sevenz.as_mut() {
        let last = by_index.keys().next_back().copied();
        let mut index = 0usize;
        let mut failure = None;
        let result = reader.for_each_entries_with_decision(|_, data| {
            let at = index;
            index += 1;
            if by_index.contains_key(&at) {
                if let Err(e) = write(at, data, &mut done) {
                    failure = Some(e);
                    return Ok(sevenz_rust2::EntryDecision::Stop);
                }
            }
            Ok(if Some(at) >= last {
                sevenz_rust2::EntryDecision::Stop
            } else {
                sevenz_rust2::EntryDecision::Continue
            })
        });
        if let Some(e) = failure {
            return Err(e);
        }
        result.map_err(sevenz_error)?;
    }
    done.missing = wanted
        .iter()
        .filter(|(s, _)| !chosen.contains_key(&key(s)))
        .map(|(s, _)| s.clone())
        .collect();
    done.missing.dedup();
    Ok(done)
}

#[cfg(test)]
mod tests;
