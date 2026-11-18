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

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
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
    /// A destination that would leave the folder.
    Outside,
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

/// A path's key: `/` between the parts, none at either end, ASCII lower case.
fn key(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect::<Vec<_>>()
        .join("/")
        .to_ascii_lowercase()
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

/// The files in the archive at `path` (folders left out), in the archive's order.
pub fn list(path: &Path) -> Result<Vec<Entry>, ArchiveError> {
    match readable(path)? {
        Format::Zip => {
            let mut zip = open_zip(path)?;
            let mut out = Vec::new();
            for i in 0..zip.len() {
                let file = zip.by_index_raw(i).map_err(zip_error)?;
                if file.is_dir() {
                    continue;
                }
                let name = file.name().map_err(zip_error)?.into_owned();
                out.push(Entry {
                    path: name,
                    size: file.size(),
                });
            }
            Ok(out)
        }
        _ => {
            let reader = open_7z(path)?;
            Ok(reader
                .archive()
                .files
                .iter()
                .filter(|f| !f.is_directory())
                .map(|f| Entry {
                    path: f.name().to_owned(),
                    size: f.size(),
                })
                .collect())
        }
    }
}

/// The bytes of the one file `name` (any case, `\` or `/`) in the archive at `path`, or `None`
/// when it holds no such file. For an installer's `ModuleConfig.xml`; a big file goes through
/// [`extract`].
pub fn read(path: &Path, name: &str) -> Result<Option<Vec<u8>>, ArchiveError> {
    let wanted = key(name);
    match readable(path)? {
        Format::Zip => {
            let mut zip = open_zip(path)?;
            for i in 0..zip.len() {
                let found = {
                    let file = zip.by_index_raw(i).map_err(zip_error)?;
                    !file.is_dir() && key(&file.name().map_err(zip_error)?) == wanted
                };
                if found {
                    let mut file = zip.by_index(i).map_err(zip_error)?;
                    let mut out = Vec::new();
                    file.read_to_end(&mut out)
                        .map_err(|e| ArchiveError::new(Problem::Damaged, format!("zip: {e}")))?;
                    return Ok(Some(out));
                }
            }
            Ok(None)
        }
        _ => {
            let mut reader = open_7z(path)?;
            let mut out = None;
            reader
                .for_each_entries_with_decision(|entry, data| {
                    if entry.is_directory() || key(entry.name()) != wanted {
                        return Ok(sevenz_rust2::EntryDecision::Continue);
                    }
                    let mut bytes = Vec::new();
                    data.read_to_end(&mut bytes)
                        .map_err(|e| sevenz_rust2::Error::Io(e, "reading".into()))?;
                    out = Some(bytes);
                    Ok(sevenz_rust2::EntryDecision::Stop)
                })
                .map_err(sevenz_error)?;
            Ok(out)
        }
    }
}

/// What [`extract`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extracted {
    /// Files written, in the order written.
    pub written: Vec<PathBuf>,
    /// Archive paths asked for that the archive does not hold.
    pub missing: Vec<String>,
}

/// Whether `relative` (with `/` or `\`) stays inside the folder it is joined to.
fn stays_inside(relative: &str) -> bool {
    let text = relative.replace('\\', "/");
    !text.is_empty()
        && !text.starts_with('/')
        && text.as_bytes().get(1) != Some(&b':')
        && text
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains(':'))
}

/// `relative` under `folder`, each part a folder already there under another case if there is one.
fn place(folder: &Path, relative: &str) -> Result<PathBuf, ArchiveError> {
    let parts: Vec<&str> = relative.split(['/', '\\']).collect();
    let mut at = folder.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let existing = std::fs::read_dir(&at).ok().and_then(|dir| {
            dir.filter_map(Result::ok)
                .find(|e| {
                    e.file_name().to_string_lossy().eq_ignore_ascii_case(part)
                        && (last || e.file_type().is_ok_and(|t| t.is_dir()))
                })
                .map(|e| e.file_name())
        });
        match existing {
            Some(name) => at.push(name),
            None => at.push(part),
        }
        if !last {
            std::fs::create_dir_all(&at).map_err(|e| io(&at, e))?;
        }
    }
    Ok(at)
}

/// Unpacks from the archive at `path` each wanted file: (path in the archive, `\` or `/`, any
/// case; path under `folder`, relative). One archive file may go to several places. Every
/// destination is checked before anything is written; a later file for the same destination
/// replaces an earlier one, as the plan has already decided.
pub fn extract(
    path: &Path,
    wanted: &[(String, String)],
    folder: &Path,
) -> Result<Extracted, ArchiveError> {
    if let Some((_, bad)) = wanted.iter().find(|(_, d)| !stays_inside(d)) {
        return Err(ArchiveError::new(Problem::Outside, bad.clone()));
    }
    let mut by_source: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (source, destination) in wanted {
        by_source.entry(key(source)).or_default().push(destination);
    }
    let format = readable(path)?;
    std::fs::create_dir_all(folder).map_err(|e| io(folder, e))?;
    let mut done = Extracted::default();
    let mut found: BTreeMap<String, bool> = by_source.keys().map(|k| (k.clone(), false)).collect();
    // Writes one archive file to its destinations; the first written, the rest copied from it.
    let write =
        |source_key: &str, data: &mut dyn Read, done: &mut Extracted| -> Result<(), ArchiveError> {
            let destinations = &by_source[source_key];
            let first = place(folder, destinations[0])?;
            let mut file = File::create(&first).map_err(|e| io(&first, e))?;
            std::io::copy(data, &mut file)
                .map_err(|e| ArchiveError::new(Problem::Damaged, format!("{source_key}: {e}")))?;
            file.flush().map_err(|e| io(&first, e))?;
            drop(file);
            done.written.push(first.clone());
            for destination in &destinations[1..] {
                let other = place(folder, destination)?;
                std::fs::copy(&first, &other).map_err(|e| io(&other, e))?;
                done.written.push(other);
            }
            Ok(())
        };
    match format {
        Format::Zip => {
            let mut zip = open_zip(path)?;
            for i in 0..zip.len() {
                let name = {
                    let file = zip.by_index_raw(i).map_err(zip_error)?;
                    if file.is_dir() {
                        continue;
                    }
                    key(&file.name().map_err(zip_error)?)
                };
                if let Some(seen) = found.get_mut(&name) {
                    *seen = true;
                    let mut file = zip.by_index(i).map_err(zip_error)?;
                    write(&name, &mut file, &mut done)?;
                }
            }
        }
        _ => {
            let mut reader = open_7z(path)?;
            let mut failure = None;
            let result = reader.for_each_entries(|entry, data| {
                if entry.is_directory() {
                    return Ok(true);
                }
                let name = key(entry.name());
                if let Some(seen) = found.get_mut(&name) {
                    *seen = true;
                    if let Err(e) = write(&name, data, &mut done) {
                        failure = Some(e);
                        return Err(sevenz_rust2::Error::Other("stopped".into()));
                    }
                }
                Ok(true)
            });
            if let Some(e) = failure {
                return Err(e);
            }
            result.map_err(sevenz_error)?;
        }
    }
    done.missing = wanted
        .iter()
        .filter(|(s, _)| !found[&key(s)])
        .map(|(s, _)| s.clone())
        .collect();
    done.missing.dedup();
    Ok(done)
}

#[cfg(test)]
mod tests;
