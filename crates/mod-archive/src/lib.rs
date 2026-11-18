//! The archives mods come in: their file listing, one file read, and chosen files unpacked into a
//! folder. Used by the FOMOD installer (`crates/fomod` plans which files go where; this does the
//! reading). The same on Windows, Linux and macOS: plain Rust, no system archive library.
//!
//! - The format is told by the first bytes, never the name (`.fomod` files are 7z or zip):
//!   zip `PK\3\4` (or an empty zip's `PK\5\6`), 7z `7z\xBC\xAF\x27\x1C`, RAR `Rar!\x1A\x07`.
//! - zip: Stored, Deflate and Deflate64. 7z: LZMA, LZMA2, BCJ/BCJ2 and PPMd, solid or not. A
//!   solid 7z block decodes from its start, so [`extract`] takes everything wanted in one pass
//!   over the archive, never file by file, and decodes no further than the last wanted file of
//!   each block; a block with nothing wanted is not decoded.
//! - RAR is not read (the lead's decision, 2026-11-17: its one complete reader, RARLab's UnRAR,
//!   has a licence that is not open source); the player is told to repack it as 7z or zip.
//!   Password-locked archives and other methods are refused, in words a player can act on.
//! - Names are compared as Windows and macOS compare them: case and dots or spaces at a name's
//!   end do not count, and Unicode names are taken in one normal form (NFC), so a name typed one
//!   way finds the file stored the other way.
//! - Files are written only under the folder given, by the relative paths given (see below).
//!
//! # Hostile archives
//!
//! - Names in an archive are only ever looked up, never used as paths (no "zip slip"); links
//!   stored in it are never listed or unpacked.
//! - A destination is checked before anything is written, on every system: no `..`, no leading
//!   separator, no `:` (drives, NTFS streams), no Windows device name (`CON`, `CONIN$`, `COM1`...),
//!   no character Windows cannot hold (`< > " | ? *`, control characters), no 8.3 short-name
//!   shape (`~` and a digit, which Windows may resolve to another name). Dots and spaces at a
//!   name's end are dropped when a folder or file is made, as Windows drops them.
//! - Nothing is written through a link: every folder used is checked to be a plain folder just
//!   before a file goes in it, each file is written under a new temporary name (made only if no
//!   such name exists, so never through a link) and renamed over its final name only when it is
//!   complete and of its declared size.
//! - Sizes are checked against [`Limits`] before anything is decoded or written: the wanted
//!   files', and for a 7z the bytes to decode to reach them. A 7z is opened with a memory limit
//!   ([`MAX_DECODER_MEMORY`]) for its header and its decoders.
//! - On any failure the error carries the files already written ([`ArchiveError::written`]), and
//!   no partial file is left. The installer unpacks each mod into a fresh folder of its own and
//!   deletes it on a failure (`crates/mod-install`), so a failed install leaves nothing behind.
//!
//! # Limits
//!
//! Every count from the archive has a cap far above any mod's; past one the archive is refused
//! in words ([`Problem::NotAMod`], [`Problem::TooBig`]), never unpacked by half. Every lookup is
//! indexed: wanted files by name, each destination folder's names read once ([`Placer`]).
//!
//! | Cap | Number | Why that number |
//! |---|---|---|
//! | [`MAX_ENTRIES`] | 500,000 entries (and wanted files) | a big texture mod has tens of thousands |
//! | [`MAX_PATH_BYTES`] | 1,024 bytes a path | Windows' own limit for most programs is 260 characters |
//! | [`MAX_NAME_BYTES`] | 255 bytes a name | the file systems' own limit |
//! | [`MAX_PATH_PARTS`] | 64 parts | the game's deepest Data paths have under 12 |
//! | [`MAX_READ`] | 16 MB | installers and their details are a few hundred KB |
//! | [`MAX_DECODER_MEMORY`] | 2 GB | above the largest dictionaries 7-Zip makes for mods |
//! | [`Limits::file`] | 16 GB by default | the game's largest archives are about 2 GB |
//! | [`Limits::total`] | 64 GB by default, for writing and for decoding; the front end lowers it to the room on the drive | the largest mods are tens of GB |
//! | a file's declared size | as the archive says | a file that unpacks to more stops the unpacking |

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

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
    /// More to unpack or decode than the limits allow (room on the drive, or far more than any
    /// mod's): a broken or hostile archive.
    TooBig,
    /// A file list beyond any mod's: more entries than [`MAX_ENTRIES`], or a name longer or deeper
    /// than [`MAX_PATH_BYTES`], [`MAX_NAME_BYTES`] or [`MAX_PATH_PARTS`].
    NotAMod,
}

#[derive(Debug)]
pub struct ArchiveError {
    pub problem: Problem,
    /// The reader's own words or the path, for the log.
    pub detail: String,
    /// The files [`extract`] had already written when it stopped, complete ones only: the caller
    /// removes them (or the whole fresh folder) to leave nothing behind.
    pub written: Vec<PathBuf>,
}

impl ArchiveError {
    fn new(problem: Problem, detail: impl Into<String>) -> Self {
        Self {
            problem,
            detail: detail.into(),
            written: vec![],
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
            Problem::NotAMod => "This archive's list of files is not a mod's: far too many files, or names far too long or too deeply nested. Download it again; if it still happens, it is not a mod nv-rs can install.",
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
/// its end do not count, nor does case (Unicode lower case), in one Unicode normal form (NFC).
fn fold_part(part: &str) -> String {
    part.trim_end_matches(['.', ' '])
        .nfc()
        .collect::<String>()
        .to_lowercase()
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

/// The most entries an archive may hold: a big texture mod has tens of thousands.
pub const MAX_ENTRIES: usize = 500_000;

/// The most bytes a path in an archive, or a destination, may have (Windows' own limit for most
/// programs is 260 characters).
pub const MAX_PATH_BYTES: usize = 1_024;

/// The most bytes one part of a destination may have (the file systems' own limit, 255).
pub const MAX_NAME_BYTES: usize = 255;

/// The most parts a path may have: the game's deepest Data paths have under 12.
pub const MAX_PATH_PARTS: usize = 64;

/// The most memory a 7z's header and decoders may ask for: above the largest dictionaries 7-Zip
/// makes for mods (its `-mx9` uses 64 MB to 1.5 GB).
pub const MAX_DECODER_MEMORY: usize = 2 << 30;

/// The most [`read`] returns: installers and their details are a few hundred KB at most.
pub const MAX_READ: u64 = 16 * 1024 * 1024;

/// Refuses a file list beyond the caps above.
fn check_entries(entries: &[(String, u64, bool)]) -> Result<(), ArchiveError> {
    if entries.len() > MAX_ENTRIES {
        return Err(ArchiveError::new(
            Problem::NotAMod,
            format!("{} entries (at most {MAX_ENTRIES})", entries.len()),
        ));
    }
    for (name, _, _) in entries {
        if name.len() > MAX_PATH_BYTES || name.split(['/', '\\']).count() > MAX_PATH_PARTS {
            return Err(ArchiveError::new(
                Problem::NotAMod,
                format!("a name of {} bytes", name.len()),
            ));
        }
    }
    Ok(())
}

/// What [`extract`] may write, and how much a 7z may decode to reach it. The front end sets
/// `total` from the room on the drive; a file is also never more than its archive says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most one file may unpack to (the game's own largest archives are under 2 GB; TTW's LOD
    /// archives a little over).
    pub file: u64,
    /// The most all the wanted files together may unpack to, and the most a 7z may decode to
    /// reach them.
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

/// 7-Zip's password method, which this build has no decoder for: a locked archive.
const SEVENZ_AES: &str = "AES256_SHA256";

fn sevenz_error(e: sevenz_rust2::Error) -> ArchiveError {
    use sevenz_rust2::Error as E;
    let problem = match &e {
        E::PasswordRequired | E::MaybeBadPassword(_) => Problem::Encrypted,
        E::UnsupportedCompressionMethod(m) if m.contains(SEVENZ_AES) => Problem::Encrypted,
        E::UnsupportedCompressionMethod(_) | E::Unsupported(_) | E::ExternalUnsupported => {
            Problem::Unsupported
        }
        E::MaxMemLimited { .. } => Problem::TooBig,
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

thread_local! {
    /// The memory limit 7z readers are opened with, in KB: always [`MAX_DECODER_MEMORY`], except
    /// in this crate's tests, which lower it to show the limit is applied.
    static DECODER_MEMORY_KB: std::cell::Cell<usize> =
        const { std::cell::Cell::new(MAX_DECODER_MEMORY / 1024) };
}

/// A 7z opened with the memory limit for its header and decoders.
fn open_7z(path: &Path) -> Result<sevenz_rust2::ArchiveReader<File>, ArchiveError> {
    let file = File::open(path).map_err(|e| io(path, e))?;
    sevenz_rust2::ArchiveReader::new_with_memory_limit(
        file,
        sevenz_rust2::Password::empty(),
        DECODER_MEMORY_KB.with(|kb| kb.get()),
    )
    .map_err(sevenz_error)
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
    if zip.len() > MAX_ENTRIES {
        return Err(ArchiveError::new(
            Problem::NotAMod,
            format!("{} entries (at most {MAX_ENTRIES})", zip.len()),
        ));
    }
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
    check_entries(&out)?;
    Ok(out)
}

fn sevenz_entries(
    reader: &sevenz_rust2::ArchiveReader<File>,
) -> Result<Vec<(String, u64, bool)>, ArchiveError> {
    if reader.archive().files.len() > MAX_ENTRIES {
        return Err(ArchiveError::new(
            Problem::NotAMod,
            format!(
                "{} entries (at most {MAX_ENTRIES})",
                reader.archive().files.len()
            ),
        ));
    }
    let out: Vec<(String, u64, bool)> = reader
        .archive()
        .files
        .iter()
        .map(|f| {
            let plain = !f.is_directory() && !sevenz_link(f);
            (f.name().to_owned(), f.size(), plain)
        })
        .collect();
    check_entries(&out)?;
    Ok(out)
}

/// The files in the archive at `path`, in the archive's order. Folders and links are left out:
/// a link is never unpacked.
pub fn list(path: &Path) -> Result<Vec<Entry>, ArchiveError> {
    let entries = match readable(path)? {
        Format::Zip => zip_entries(&mut open_zip(path)?)?,
        _ => sevenz_entries(&open_7z(path)?)?,
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

/// A 7z's plan to decode some of its files: the order the reader hands entries over in (each
/// block's files with data, block by block, then the files without data), the last wanted file
/// of each block, and the bytes to decode to reach the wanted files.
struct SevenzPlan {
    order: Vec<usize>,
    block_of: BTreeMap<usize, usize>,
    last_wanted: BTreeMap<usize, usize>,
    decode: u64,
}

fn sevenz_plan(
    archive: &sevenz_rust2::Archive,
    wanted: &std::collections::BTreeSet<usize>,
) -> SevenzPlan {
    let blocks = &archive.stream_map.file_block_index;
    let mut order: Vec<usize> = (0..archive.files.len())
        .filter(|&i| archive.files[i].has_stream && blocks[i].is_some())
        .collect();
    order.sort_by_key(|&i| (blocks[i], i));
    order.extend(
        (0..archive.files.len()).filter(|&i| blocks[i].is_none() || !archive.files[i].has_stream),
    );
    let mut last_wanted = BTreeMap::new();
    for &i in wanted {
        if let Some(Some(b)) = blocks.get(i) {
            let last = last_wanted.entry(*b).or_insert(i);
            *last = (*last).max(i);
        }
    }
    let decode = order
        .iter()
        .filter(|&&i| {
            archive.files[i].has_stream
                && blocks[i].is_some_and(|b| last_wanted.get(&b).is_some_and(|&last| i <= last))
        })
        .map(|&i| archive.files[i].size)
        .fold(0u64, u64::saturating_add);
    let block_of = (0..archive.files.len())
        .filter_map(|i| blocks[i].map(|b| (i, b)))
        .collect();
    SevenzPlan {
        order,
        block_of,
        last_wanted,
        decode,
    }
}

/// The bytes of the one file `name` (any case, `\` or `/`) in the archive at `path`, or `None`
/// when it holds no such file (found from the listing, without decoding anything). For an
/// installer's `ModuleConfig.xml` (at most [`MAX_READ`]); a big file goes through [`extract`]. A 7z
/// decodes no more than [`Limits::default`]'s total to reach it.
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
            let entries = sevenz_entries(&reader)?;
            let Some(target) = entries
                .iter()
                .position(|(n, _, plain)| *plain && key(n) == wanted)
            else {
                return Ok(None);
            };
            if entries[target].1 > MAX_READ {
                return Err(ArchiveError::new(
                    Problem::TooBig,
                    format!("{name}: {} bytes", entries[target].1),
                ));
            }
            if reader.archive().is_file_encrypted(target) {
                return Err(ArchiveError::new(Problem::Encrypted, name.to_owned()));
            }
            let plan = sevenz_plan(reader.archive(), &[target].into_iter().collect());
            let budget = Limits::default().total;
            if plan.decode > budget {
                return Err(ArchiveError::new(
                    Problem::TooBig,
                    format!("{} bytes to decode to reach {name}", plan.decode),
                ));
            }
            let mut out = None;
            let mut failure = None;
            let mut next = 0usize;
            let result = reader.for_each_entries_with_decision(|_, data| {
                let index = plan.order[next];
                next += 1;
                if index != target {
                    return Ok(
                        if plan_block_done(&plan, reader_block(&plan, index), index) {
                            sevenz_rust2::EntryDecision::SkipBlock
                        } else {
                            sevenz_rust2::EntryDecision::Continue
                        },
                    );
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

/// The block of file `index` in a plan's order (`usize::MAX` for a file without data).
fn reader_block(plan: &SevenzPlan, index: usize) -> usize {
    plan.block_of.get(&index).copied().unwrap_or(usize::MAX)
}

/// Whether nothing more is wanted from `block` once file `index` of it is reached.
fn plan_block_done(plan: &SevenzPlan, block: usize, index: usize) -> bool {
    plan.last_wanted
        .get(&block)
        .map_or(true, |&last| index > last)
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

/// Whether a path part is one of Windows' device names (`CON`, `NUL`, `CONIN$`, `COM1`...), with
/// or without an extension: Windows opens the device for such a name in any folder.
fn device_name(part: &str) -> bool {
    let stem = fold_part(part);
    let stem = stem.split('.').next().unwrap_or_default().trim_end();
    matches!(
        stem,
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$" | "clock$"
    ) || ((stem.starts_with("com") || stem.starts_with("lpt"))
        && stem.chars().count() == 4
        && stem
            .chars()
            .nth(3)
            .is_some_and(|c| c.is_ascii_digit() || "¹²³".contains(c)))
}

/// Whether a part has the shape of a Windows 8.3 short name (`~` then a digit), which Windows may
/// resolve to another file than the one meant.
fn short_name_shape(part: &str) -> bool {
    part.as_bytes()
        .windows(2)
        .any(|w| w[0] == b'~' && w[1].is_ascii_digit())
}

/// Whether `relative` (with `/` or `\`) stays inside the folder it is joined to and names what
/// every system can hold: see the module's text, and within [`MAX_PATH_BYTES`],
/// [`MAX_PATH_PARTS`] and [`MAX_NAME_BYTES`].
fn stays_inside(relative: &str) -> bool {
    let text = relative.replace('\\', "/");
    !text.is_empty()
        && text.len() <= MAX_PATH_BYTES
        && text.split('/').count() <= MAX_PATH_PARTS
        && !text.starts_with('/')
        && text.split('/').all(|p| {
            !p.is_empty()
                && p.len() <= MAX_NAME_BYTES
                && p != "."
                && p.trim() != ".."
                && !fold_part(p).is_empty()
                && !p.contains(':')
                && !p.chars().any(|c| c < ' ' || "<>\"|?*".contains(c))
                && !device_name(p)
                && !short_name_shape(p)
        })
}

/// What is known of one entry in a folder: its own name, and whether it is a folder or a link.
#[derive(Clone)]
struct Seen {
    name: std::ffi::OsString,
    folder: bool,
    link: bool,
}

/// Places relative paths under one folder. Each folder's names are read once and kept, so
/// placing many files in one folder costs one look each, not a reading of the folder each.
struct Placer<'f> {
    folder: &'f Path,
    folders: BTreeMap<PathBuf, BTreeMap<String, Seen>>,
}

impl<'f> Placer<'f> {
    fn new(folder: &'f Path) -> Self {
        Self {
            folder,
            folders: BTreeMap::new(),
        }
    }

    /// The names in `dir` by [`fold_part`]; the first of two that fold alike.
    fn names(&mut self, dir: &Path) -> Result<&mut BTreeMap<String, Seen>, ArchiveError> {
        if !self.folders.contains_key(dir) {
            let mut names = BTreeMap::new();
            match std::fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry.map_err(|e| io(dir, e))?;
                        let kind = entry.file_type().map_err(|e| io(&entry.path(), e))?;
                        names
                            .entry(fold_part(&entry.file_name().to_string_lossy()))
                            .or_insert(Seen {
                                name: entry.file_name(),
                                folder: kind.is_dir(),
                                link: kind.is_symlink(),
                            });
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io(dir, e)),
            }
            self.folders.insert(dir.to_path_buf(), names);
        }
        Ok(self.folders.get_mut(dir).expect("just read"))
    }

    /// `relative` under the folder, each part an entry already there under another case if there
    /// is one, a new part made without the dots and spaces at its end (as Windows makes it); the
    /// folders on the way made. A link on the way, or at the end, is refused: a file is never
    /// written through a link.
    fn place(&mut self, relative: &str) -> Result<PathBuf, ArchiveError> {
        let parts: Vec<&str> = relative.split(['/', '\\']).collect();
        let mut at = self.folder.to_path_buf();
        for (i, part) in parts.iter().enumerate() {
            let last = i + 1 == parts.len();
            let wanted = fold_part(part);
            let here = at.clone();
            let names = self.names(&here)?;
            let next = match names.get(&wanted) {
                Some(seen) => {
                    if seen.link {
                        return Err(ArchiveError::new(
                            Problem::Outside,
                            format!("{} is a link", here.join(&seen.name).display()),
                        ));
                    }
                    if !last && !seen.folder {
                        return Err(ArchiveError::new(
                            Problem::Outside,
                            format!(
                                "{} is a file, not a folder",
                                here.join(&seen.name).display()
                            ),
                        ));
                    }
                    here.join(&seen.name)
                }
                None => {
                    let own: String = part.trim_end_matches(['.', ' ']).nfc().collect();
                    names.insert(
                        wanted,
                        Seen {
                            name: own.clone().into(),
                            folder: !last,
                            link: false,
                        },
                    );
                    here.join(own)
                }
            };
            if !last {
                std::fs::create_dir_all(&next).map_err(|e| io(&next, e))?;
            }
            at = next;
        }
        Ok(at)
    }

    /// Checks, just before a file is written in `dir`, that `dir` and every folder from the
    /// destination folder to it is a plain folder, not a link made since it was looked at.
    fn plain_folders(&self, dir: &Path) -> Result<(), ArchiveError> {
        let mut at = Some(dir);
        while let Some(folder) = at {
            let meta = std::fs::symlink_metadata(folder).map_err(|e| io(folder, e))?;
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err(ArchiveError::new(
                    Problem::Outside,
                    format!("{} is a link", folder.display()),
                ));
            }
            if folder == self.folder {
                break;
            }
            at = folder.parent();
        }
        Ok(())
    }
}

/// Writes `data` (at most `size` bytes, else [`Problem::TooBig`]) as `target`: under a new
/// temporary name beside it, made only if no such name exists (so never through a link), then
/// renamed over `target` once complete. On a failure the temporary file is removed.
fn write_whole(
    placer: &Placer,
    target: &Path,
    data: &mut dyn Read,
    size: u64,
    what: &str,
) -> Result<(), ArchiveError> {
    let dir = target.parent().unwrap_or(placer.folder);
    placer.plain_folders(dir)?;
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = (0..100u32)
        .map(|n| dir.join(format!(".{name}.nv-rs-part{n}")))
        .find_map(|path| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .ok()
                .map(|file| (path, file))
        });
    let Some((temporary, mut file)) = temporary else {
        return Err(ArchiveError::new(
            Problem::Io,
            format!(
                "{}: no temporary name could be made beside it",
                target.display()
            ),
        ));
    };
    let result = (|| {
        let written = std::io::copy(&mut data.take(size.saturating_add(1)), &mut file)
            .map_err(|e| ArchiveError::new(Problem::Damaged, format!("{what}: {e}")))?;
        file.flush().map_err(|e| io(&temporary, e))?;
        if written > size {
            return Err(ArchiveError::new(
                Problem::TooBig,
                format!("{what} unpacks to more than the {size} bytes its archive says"),
            ));
        }
        if written < size {
            return Err(ArchiveError::new(
                Problem::Damaged,
                format!("{what} unpacks to {written} of the {size} bytes its archive says"),
            ));
        }
        Ok(())
    })();
    drop(file);
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temporary);
        return Err(e);
    }
    placer.plain_folders(dir)?;
    std::fs::rename(&temporary, target).map_err(|e| {
        let _ = std::fs::remove_file(&temporary);
        io(target, e)
    })
}

/// Unpacks from the archive at `path` each wanted file: (path in the archive, `\` or `/`, any
/// case; path under `folder`, relative). One archive file may go to several places.
///
/// Before anything is written: every destination is checked ([`stays_inside`]), the wanted files'
/// sizes as the archive gives them against `limits`, and for a 7z the bytes to decode to reach
/// them. While writing: see the module's text. Of two entries whose names differ only in case,
/// the first is taken. On a failure, the error's [`ArchiveError::written`] lists the complete
/// files already written. Unpack into a fresh folder and delete it on a failure.
pub fn extract(
    path: &Path,
    wanted: &[(String, String)],
    folder: &Path,
    limits: Limits,
) -> Result<Extracted, ArchiveError> {
    if wanted.len() > MAX_ENTRIES {
        return Err(ArchiveError::new(
            Problem::NotAMod,
            format!("{} files wanted (at most {MAX_ENTRIES})", wanted.len()),
        ));
    }
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
        _ => sevenz_entries(sevenz.insert(open_7z(path)?))?,
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
    let mut plan = None;
    if let Some(reader) = sevenz.as_ref() {
        if let Some(&i) = by_index
            .keys()
            .find(|&&i| reader.archive().is_file_encrypted(i))
        {
            return Err(ArchiveError::new(Problem::Encrypted, entries[i].0.clone()));
        }
        let made = sevenz_plan(reader.archive(), &by_index.keys().copied().collect());
        if made.decode > limits.total {
            return Err(ArchiveError::new(
                Problem::TooBig,
                format!("{} bytes to decode, {} allowed", made.decode, limits.total),
            ));
        }
        plan = Some(made);
    }
    std::fs::create_dir_all(folder).map_err(|e| io(folder, e))?;
    let mut done = Extracted::default();
    let mut placer = Placer::new(folder);
    // Writes one archive file to its destinations; the first written, the rest copied from it.
    let mut write =
        |index: usize, data: &mut dyn Read, done: &mut Extracted| -> Result<(), ArchiveError> {
            let source_key = &by_index[&index];
            let (name, size, _) = &entries[index];
            let destinations = &by_source[source_key];
            let first = placer.place(destinations[0])?;
            write_whole(&placer, &first, data, *size, name)?;
            done.written.push(first.clone());
            for destination in &destinations[1..] {
                let other = placer.place(destination)?;
                let mut copy = File::open(&first).map_err(|e| io(&first, e))?;
                write_whole(&placer, &other, &mut copy, *size, name)?;
                done.written.push(other);
            }
            Ok(())
        };
    let outcome: Result<(), ArchiveError> = if let Some(zip) = zip.as_mut() {
        (|| {
            for &index in by_index.keys() {
                let mut file = zip.by_index(index).map_err(zip_error)?;
                write(index, &mut file, &mut done)?;
            }
            Ok(())
        })()
    } else if let (Some(reader), Some(plan)) = (sevenz.as_mut(), plan.as_ref()) {
        // Each entry the reader hands over is the next one of the plan's order, so the file is
        // known by its index, never by its name alone.
        let mut next = 0usize;
        let mut left = by_index.len();
        let mut failure = None;
        let result = reader.for_each_entries_with_decision(|_, data| {
            if left == 0 {
                return Ok(sevenz_rust2::EntryDecision::Stop);
            }
            let index = plan.order[next];
            next += 1;
            if !by_index.contains_key(&index) {
                return Ok(if plan_block_done(plan, reader_block(plan, index), index) {
                    sevenz_rust2::EntryDecision::SkipBlock
                } else {
                    sevenz_rust2::EntryDecision::Continue
                });
            }
            if let Err(e) = write(index, data, &mut done) {
                failure = Some(e);
                return Ok(sevenz_rust2::EntryDecision::Stop);
            }
            left -= 1;
            Ok(if left == 0 {
                sevenz_rust2::EntryDecision::Stop
            } else if plan_block_done(plan, reader_block(plan, index), index) {
                sevenz_rust2::EntryDecision::SkipBlock
            } else {
                sevenz_rust2::EntryDecision::Continue
            })
        });
        match failure {
            Some(e) => Err(e),
            None => result.map_err(sevenz_error),
        }
    } else {
        Ok(())
    };
    if let Err(mut e) = outcome {
        e.written = std::mem::take(&mut done.written);
        return Err(e);
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
