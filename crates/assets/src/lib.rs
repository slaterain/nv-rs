//! Finding game files the way Fallout: New Vegas does.
//!
//! Files come from two places: loose files under the `Data` folder, and
//! the BSA archives the game loads. The game loads the archives listed in
//! its settings first, then, for each active plugin in load order, any
//! archive named after that plugin (`DeadMoney - Main.bsa` for
//! `DeadMoney.esm`). A file in a later archive replaces the same path in an
//! earlier one, and loose files replace archived ones.
//!
//! (The unmodified game only lets a loose file win if it's newer than the
//! archive; every common mod setup turns that check off, so loose files
//! always win here.)
//!
//! ```no_run
//! let plugins = vec!["FalloutNV.esm".to_string(), "DeadMoney.esm".to_string()];
//! let assets = assets::Assets::open("Data", &plugins)?;
//! if let Some(bytes) = assets.read("textures/weapons/1handpistol/10mmpistol.dds")? {
//!     println!("{} bytes", bytes.len());
//! }
//! # Ok::<(), assets::Error>(())
//! ```

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub use bsa::normalize_path;

/// The archives New Vegas loads by default (its `SArchiveList` setting),
/// in order. Used only when no ini file with the setting is found.
pub const DEFAULT_ARCHIVES: [&str; 6] = [
    "Fallout - Textures.bsa",
    "Fallout - Textures2.bsa",
    "Fallout - Meshes.bsa",
    "Fallout - Voices1.bsa",
    "Fallout - Sound.bsa",
    "Fallout - Misc.bsa",
];

/// The archive New Vegas's patches added. The shipped ini files don't list
/// it, but records in FalloutNV.esm use meshes that only it contains (the
/// NCR guard towers and Novac motel pieces among them), so it's loaded
/// right after the listed archives.
pub const PATCH_ARCHIVE: &str = "Update.bsa";

/// Guards against symlink loops while scanning loose files.
const MAX_FOLDER_DEPTH: usize = 32;

#[derive(Debug)]
pub enum Error {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Archive {
        name: String,
        source: bsa::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Archive { name, source } => write!(f, "{name}: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Archive { source, .. } => Some(source),
        }
    }
}

/// Why an archive is loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveReason {
    /// Listed in the game's default archive list.
    Default,
    /// The game's patch archive (see [`PATCH_ARCHIVE`]).
    Patch,
    /// Named after this active plugin.
    Plugin(String),
}

pub struct LoadedArchive {
    pub name: String,
    pub reason: ArchiveReason,
    pub archive: bsa::Archive,
}

/// Where a file comes from.
#[derive(Clone, Copy)]
pub enum Source<'a> {
    Loose(&'a Path),
    Archive {
        name: &'a str,
        archive: &'a bsa::Archive,
        entry: &'a bsa::FileEntry,
    },
}

impl Source<'_> {
    pub fn read(&self) -> Result<Vec<u8>> {
        match *self {
            Source::Loose(path) => std::fs::read(path).map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            }),
            Source::Archive {
                name,
                archive,
                entry,
            } => archive.read(entry).map_err(|source| Error::Archive {
                name: name.to_string(),
                source,
            }),
        }
    }

    /// A short description such as `loose file` or `Fallout - Meshes.bsa`.
    pub fn describe(&self) -> String {
        match self {
            Source::Loose(_) => "loose file".into(),
            Source::Archive { name, .. } => (*name).to_string(),
        }
    }
}

/// The game's own list of archives to load, and where it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveList {
    pub names: Vec<String>,
    /// The ini file the list came from, or a note that the built-in default
    /// was used.
    pub source: String,
}

/// Reads `SArchiveList` from an ini file, if the file has it.
pub fn read_archive_list(ini: &Path) -> Option<Vec<String>> {
    let text = std::fs::read(ini).ok()?;
    let text = String::from_utf8_lossy(&text);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with(';') && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case("SArchiveList"))
        .map(|(_, value)| {
            value
                .split(',')
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(String::from)
                .collect()
        })
}

/// The game's INI settings, as `[Section]` and `key=value` lines from its
/// INI files, later files winning where they share a key (the user's
/// `Fallout.ini`, then `FalloutPrefs.ini`, over the install's
/// `Fallout_default.ini`: [`default_settings_files`]). Names compare without
/// case.
#[derive(Debug, Clone, Default)]
pub struct IniSettings {
    values: std::collections::HashMap<(String, String), String>,
}

impl IniSettings {
    /// Reads the files in order (missing ones are skipped).
    pub fn load(files: &[PathBuf]) -> IniSettings {
        let mut out = IniSettings::default();
        for file in files {
            if let Ok(bytes) = std::fs::read(file) {
                out.add(&String::from_utf8_lossy(&bytes));
            }
        }
        out
    }

    /// Adds one file's text.
    pub fn add(&mut self, text: &str) {
        let mut section = String::new();
        for line in text.lines().map(str::trim) {
            if line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_ascii_lowercase();
            } else if let Some((key, value)) = line.split_once('=') {
                self.values.insert(
                    (section.clone(), key.trim().to_ascii_lowercase()),
                    value.trim().to_string(),
                );
            }
        }
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.values
            .get(&(section.to_ascii_lowercase(), key.to_ascii_lowercase()))
            .map(String::as_str)
    }

    pub fn float(&self, section: &str, key: &str) -> Option<f32> {
        self.get(section, key)?.parse().ok()
    }
}

/// The INI files the game reads its settings from, lowest first: the
/// install's `Fallout_default.ini`, then the user's `Fallout.ini` and
/// `FalloutPrefs.ini` under `Documents\My Games\FalloutNV` (or OneDrive's
/// Documents).
pub fn default_settings_files(data_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Some(game_dir) = data_dir.parent() {
        files.push(game_dir.join("Fallout_default.ini"));
    }
    if let Some(home) = user_home() {
        for documents in [
            home.join("Documents"),
            home.join("OneDrive").join("Documents"),
        ] {
            let dir = documents.join("My Games").join("FalloutNV");
            if dir.join("Fallout.ini").exists() {
                files.push(dir.join("Fallout.ini"));
                files.push(dir.join("FalloutPrefs.ini"));
                break;
            }
        }
    }
    files
}

/// Uses the first candidate ini file that has an archive list, or the
/// built-in default.
pub fn archive_list_from(candidates: &[PathBuf]) -> ArchiveList {
    for ini in candidates {
        if let Some(names) = read_archive_list(ini) {
            return ArchiveList {
                names,
                source: ini.display().to_string(),
            };
        }
    }
    ArchiveList {
        names: DEFAULT_ARCHIVES.iter().map(|s| s.to_string()).collect(),
        source: "built-in default (no ini file with SArchiveList found)".into(),
    }
}

/// Where the game's settings usually are: `Fallout.ini` under
/// `Documents\My Games\FalloutNV` (also checked under OneDrive, which
/// often holds Documents), then `Fallout_default.ini` in the install folder
/// that contains `Data`.
pub fn default_ini_candidates(data_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(home) = user_home() {
        for documents in [
            home.join("Documents"),
            home.join("OneDrive").join("Documents"),
        ] {
            candidates.push(
                documents
                    .join("My Games")
                    .join("FalloutNV")
                    .join("Fallout.ini"),
            );
        }
    }
    if let Some(game_dir) = data_dir.parent() {
        candidates.push(game_dir.join("Fallout_default.ini"));
    }
    candidates
}

/// Windows game installs use `USERPROFILE`; native macOS builds use `HOME`.
/// Prefer the Windows variable when running in a compatibility environment.
fn user_home() -> Option<PathBuf> {
    user_home_from(std::env::var_os("USERPROFILE"), std::env::var_os("HOME"))
}

fn user_home_from(
    userprofile: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    userprofile.or(home).map(PathBuf::from)
}

/// Every file the game can see, with overrides resolved.
pub struct Assets {
    data_dir: PathBuf,
    archives: Vec<LoadedArchive>,
    unused_archives: Vec<String>,
    /// Archives in the folder that aren't loaded, kept open so missing
    /// files can be traced to them.
    inactive: Vec<LoadedArchive>,
    /// Normalized path -> file on disk.
    loose: HashMap<String, PathBuf>,
    /// Normalized path -> (archive index, file index) of the winning copy.
    archived: HashMap<String, (usize, usize)>,
}

fn stem_of(file_name: &str) -> &str {
    file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem)
}

impl Assets {
    /// Opens a `Data` folder using the built-in default archive list.
    /// `plugins` are the active plugin file names in load order; they decide
    /// which extra archives are loaded.
    pub fn open(data_dir: impl AsRef<Path>, plugins: &[String]) -> Result<Self> {
        let list: Vec<String> = DEFAULT_ARCHIVES.iter().map(|s| s.to_string()).collect();
        Self::open_with(data_dir, plugins, &list)
    }

    /// Opens a `Data` folder with an explicit archive list (see
    /// [`archive_list_from`]).
    pub fn open_with(
        data_dir: impl AsRef<Path>,
        plugins: &[String],
        archive_list: &[String],
    ) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let io_error = |source| Error::Io {
            path: data_dir.clone(),
            source,
        };
        let mut bsa_files: Vec<String> = std::fs::read_dir(&data_dir)
            .map_err(io_error)?
            .flatten()
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.to_ascii_lowercase().ends_with(".bsa"))
            .collect();
        bsa_files.sort_by_key(|n| n.to_ascii_lowercase());

        // Pick archives in load order.
        let mut chosen: Vec<(String, ArchiveReason)> = Vec::new();
        let take =
            |name: &str, chosen: &mut Vec<(String, ArchiveReason)>, reason: ArchiveReason| {
                let already = chosen.iter().any(|(n, _)| n.eq_ignore_ascii_case(name));
                if !already {
                    chosen.push((name.to_string(), reason));
                }
            };
        for wanted in archive_list {
            if let Some(found) = bsa_files.iter().find(|n| n.eq_ignore_ascii_case(wanted)) {
                take(found, &mut chosen, ArchiveReason::Default);
            }
        }
        if let Some(found) = bsa_files
            .iter()
            .find(|n| n.eq_ignore_ascii_case(PATCH_ARCHIVE))
        {
            take(found, &mut chosen, ArchiveReason::Patch);
        }
        for plugin in plugins {
            let stem = stem_of(plugin).to_ascii_lowercase();
            for name in &bsa_files {
                let lower = name.to_ascii_lowercase();
                let own =
                    lower == format!("{stem}.bsa") || lower.starts_with(&format!("{stem} - "));
                if own {
                    take(name, &mut chosen, ArchiveReason::Plugin(plugin.clone()));
                }
            }
        }
        let unused_archives: Vec<String> = bsa_files
            .iter()
            .filter(|n| !chosen.iter().any(|(c, _)| c == *n))
            .cloned()
            .collect();
        // Unloaded archives that can't be read are simply left out.
        let inactive = unused_archives
            .iter()
            .filter_map(|name| {
                let archive = bsa::Archive::open(data_dir.join(name)).ok()?;
                Some(LoadedArchive {
                    name: name.clone(),
                    reason: ArchiveReason::Default,
                    archive,
                })
            })
            .collect();

        let mut archives = Vec::with_capacity(chosen.len());
        for (name, reason) in chosen {
            let archive =
                bsa::Archive::open(data_dir.join(&name)).map_err(|source| Error::Archive {
                    name: name.clone(),
                    source,
                })?;
            archives.push(LoadedArchive {
                name,
                reason,
                archive,
            });
        }

        let mut archived = HashMap::new();
        for (ai, loaded) in archives.iter().enumerate() {
            for (fi, file) in loaded.archive.files().iter().enumerate() {
                archived.insert(file.path.clone(), (ai, fi));
            }
        }

        let mut loose = HashMap::new();
        scan_loose(&data_dir, "", 0, &mut loose)?;

        Ok(Self {
            data_dir,
            archives,
            unused_archives,
            inactive,
            loose,
            archived,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Loaded archives, lowest priority first.
    pub fn archives(&self) -> &[LoadedArchive] {
        &self.archives
    }

    /// Archives in the folder that the game doesn't load with these plugins.
    pub fn unused_archives(&self) -> &[String] {
        &self.unused_archives
    }

    /// Names of archives the game doesn't load that contain this path; a
    /// quick way to explain a "missing" file.
    pub fn in_unused_archives(&self, path: &str) -> Vec<&str> {
        let key = normalize_path(path);
        self.inactive
            .iter()
            .filter(|a| a.archive.find(&key).is_some())
            .map(|a| a.name.as_str())
            .collect()
    }

    pub fn loose_count(&self) -> usize {
        self.loose.len()
    }

    /// Number of distinct paths visible to the game.
    pub fn len(&self) -> usize {
        self.archived.len()
            + self
                .loose
                .keys()
                .filter(|k| !self.archived.contains_key(*k))
                .count()
    }

    pub fn is_empty(&self) -> bool {
        self.archived.is_empty() && self.loose.is_empty()
    }

    fn archived_source(&self, ai: usize, fi: usize) -> Source<'_> {
        let loaded = &self.archives[ai];
        Source::Archive {
            name: &loaded.name,
            archive: &loaded.archive,
            entry: &loaded.archive.files()[fi],
        }
    }

    /// Where the game would load a file from, if anywhere.
    pub fn locate(&self, path: &str) -> Option<Source<'_>> {
        let key = normalize_path(path);
        if let Some(p) = self.loose.get(&key) {
            return Some(Source::Loose(p));
        }
        self.archived
            .get(&key)
            .map(|&(ai, fi)| self.archived_source(ai, fi))
    }

    pub fn contains(&self, path: &str) -> bool {
        let key = normalize_path(path);
        self.loose.contains_key(&key) || self.archived.contains_key(&key)
    }

    /// Reads a file, or returns `None` if the game can't see it.
    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>> {
        self.locate(path).map(|s| s.read()).transpose()
    }

    /// Every copy of a file, highest priority (the one used) first.
    pub fn versions(&self, path: &str) -> Vec<Source<'_>> {
        let key = normalize_path(path);
        let mut out = Vec::new();
        if let Some(p) = self.loose.get(&key) {
            out.push(Source::Loose(p));
        }
        for loaded in self.archives.iter().rev() {
            if let Some(entry) = loaded.archive.find(&key) {
                out.push(Source::Archive {
                    name: &loaded.name,
                    archive: &loaded.archive,
                    entry,
                });
            }
        }
        out
    }

    /// Every visible path (each once), in no particular order.
    pub fn paths(&self) -> impl Iterator<Item = &str> + '_ {
        self.archived.keys().map(String::as_str).chain(
            self.loose
                .keys()
                .filter(|k| !self.archived.contains_key(*k))
                .map(String::as_str),
        )
    }
}

/// Indexes files in subfolders of `Data` (files directly in `Data` are
/// plugins and archives, not assets).
fn scan_loose(
    dir: &Path,
    prefix: &str,
    depth: usize,
    out: &mut HashMap<String, PathBuf>,
) -> Result<()> {
    if depth > MAX_FOLDER_DEPTH {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}\\{name}")
        };
        if kind.is_dir() {
            scan_loose(&entry.path(), &relative, depth + 1, out)?;
        } else if depth > 0 {
            out.insert(relative, entry.path());
        }
    }
    Ok(())
}

/// Normalizes a texture path as written in a mesh: case and slashes, a
/// leading `data\`, and the implied `textures\` folder (paths sometimes
/// omit it, or include an artist's full path before it).
pub fn texture_path(reference: &str) -> String {
    folder_relative(reference, "textures")
}

/// Normalizes a model path as written in a plugin record, which is
/// relative to `meshes\`.
pub fn mesh_path(reference: &str) -> String {
    folder_relative(reference, "meshes")
}

fn folder_relative(reference: &str, folder: &str) -> String {
    let path = normalize_path(reference.trim());
    let marker = format!("{folder}\\");
    if path.starts_with(&marker) {
        return path;
    }
    match path.find(&format!("\\{marker}")) {
        Some(i) => path[i + 1..].to_string(),
        None => format!("{marker}{path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_texture_and_mesh_references() {
        assert_eq!(
            texture_path("Textures\\Weapons\\Gun.DDS"),
            "textures\\weapons\\gun.dds"
        );
        assert_eq!(
            texture_path("weapons/gun.dds"),
            "textures\\weapons\\gun.dds"
        );
        assert_eq!(texture_path("Data\\Textures\\x.dds"), "textures\\x.dds");
        assert_eq!(
            texture_path("C:\\Projects\\Fallout\\Data\\Textures\\Clutter\\x.dds"),
            "textures\\clutter\\x.dds"
        );
        assert_eq!(
            mesh_path("Weapons\\1HandPistol\\10mm.NIF"),
            "meshes\\weapons\\1handpistol\\10mm.nif"
        );
        assert_eq!(mesh_path("meshes\\a.nif"), "meshes\\a.nif");
    }

    #[test]
    fn home_discovery_falls_back_to_macos_home() {
        assert_eq!(
            user_home_from(None, Some("player-home".into())),
            Some(PathBuf::from("player-home"))
        );
        assert_eq!(
            user_home_from(Some("windows-home".into()), Some("mac-home".into())),
            Some(PathBuf::from("windows-home"))
        );
    }
}
