//! Where things are in a mod archive, from its list of files (paths with `/` or `\`).
//!
//! The listing may come straight from an archive: folder entries (`Wrapper/`, or a bare
//! `Wrapper` from a 7z) are dropped, so only files are left ([`Listing`]). Names are compared as
//! Windows and macOS compare them ([`fold`]); `__MACOSX` folders (left by Macs' zip tool) are not
//! looked at. A listing beyond any mod's ([`MAX_ENTRIES`] files, paths of more than
//! [`MAX_PATH_PARTS`] parts or [`MAX_PATH_BYTES`] bytes) is refused as not a mod's, so the work on
//! it stays bounded: it is indexed once, and every lookup after that is a search in that index.
//!
//! The `fomod` folder is found as Mod Organizer 2 finds it: at the top, else, when the top holds
//! exactly one name and that is a folder, inside it, and so on down (archives often wrap
//! everything in `ModName v1.2\`). File sources in the installer are relative to the folder
//! holding `fomod` (the prefix).
//!
//! An archive without an installer, or with a C# `script.cs` installer this port does not run,
//! may still hold a plain Data layout: [`data_folder`] finds the shallowest folder that looks like
//! a Data folder (holds a plugin or archive, or one of the game's Data folders), as managers do.

use std::collections::BTreeMap;

/// The most files a listing may hold: a big texture mod has tens of thousands.
pub const MAX_ENTRIES: usize = 500_000;

/// The most parts a path may have: the game's deepest Data paths have under 12.
pub const MAX_PATH_PARTS: usize = 64;

/// The most bytes a path may have: Windows' own limit for most programs is 260 characters.
pub const MAX_PATH_BYTES: usize = 1_024;

/// Why a listing is not taken as a mod's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingError(pub String);

impl std::fmt::Display for ListingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the archive's file list is not a mod's: {}", self.0)
    }
}

/// What an archive holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    /// An XML installer: the prefix and the listing's own spelling of its two files.
    Installer {
        prefix: String,
        config: String,
        info: Option<String>,
    },
    /// A `fomod\script.cs` installer (a program; not run) and the Data layout beside it, if any.
    Script {
        prefix: String,
        script: String,
        data: Option<String>,
    },
    /// No installer: files to install as they are laid out from `data` down (`""` the top).
    Plain { data: String, info: Option<String> },
    /// Nothing that looks like a mod.
    Unknown,
}

/// The path with `/` between its parts and no `/` at either end.
pub fn normalise(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// One part of a path as Windows (and macOS by default) tells names apart: dots and spaces at
/// its end do not count, nor does case (Unicode lower case).
pub fn fold_part(part: &str) -> String {
    part.trim_end_matches(['.', ' ']).to_lowercase()
}

/// A path's key: [`normalise`]d, each part [`fold_part`]ed.
pub fn fold(path: &str) -> String {
    normalise(path)
        .split('/')
        .map(fold_part)
        .collect::<Vec<_>>()
        .join("/")
}

/// `a` and `b` joined with `/` (either may be empty).
pub fn join(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_owned(),
        (_, true) => a.to_owned(),
        _ => format!("{a}/{b}"),
    }
}

/// The keys of `files` under the folder key `prefix` (`""` or ending in `/`): the first and the
/// last, in key order.
fn first_and_last<'m, V>(
    files: &'m BTreeMap<String, V>,
    prefix: &str,
) -> Option<(&'m String, &'m String)> {
    let first = files
        .range(prefix.to_owned()..)
        .next()
        .filter(|(k, _)| k.starts_with(prefix))?
        .0;
    // Every key under `prefix` sorts before `prefix` with its last `/` raised to `0`.
    let last = match prefix.strip_suffix('/') {
        Some(stem) => files.range(..format!("{stem}0")).next_back()?.0,
        None => files.keys().next_back()?,
    };
    Some((first, last))
}

/// An archive's files, indexed once: by [`fold`]ed path, each with the listing's own spelling.
#[derive(Debug, Clone, Default)]
pub struct Listing<'a> {
    files: BTreeMap<String, &'a str>,
}

impl<'a> Listing<'a> {
    /// The files of `listing`: folder entries (ending in `/` or `\`, or a name another entry is
    /// inside of), `__MACOSX` and empty paths left out; of two entries that fold to one path, the
    /// first. Refused past [`MAX_ENTRIES`], [`MAX_PATH_PARTS`] or [`MAX_PATH_BYTES`].
    pub fn new(listing: &'a [String]) -> Result<Self, ListingError> {
        if listing.len() > MAX_ENTRIES {
            return Err(ListingError(format!(
                "{} entries (a mod has at most {MAX_ENTRIES})",
                listing.len()
            )));
        }
        let mut files: BTreeMap<String, &'a str> = BTreeMap::new();
        for given in listing {
            if given.len() > MAX_PATH_BYTES {
                return Err(ListingError(format!(
                    "a path of {} bytes (at most {MAX_PATH_BYTES})",
                    given.len()
                )));
            }
            if given.ends_with(['/', '\\']) {
                continue;
            }
            let key = fold(given);
            if key.split('/').count() > MAX_PATH_PARTS {
                return Err(ListingError(format!(
                    "a path {} folders deep (at most {MAX_PATH_PARTS})",
                    key.split('/').count()
                )));
            }
            if key.is_empty() || key.split('/').any(|p| p == "__macosx") {
                continue;
            }
            files.entry(key).or_insert(given.as_str());
        }
        let folders: Vec<String> = files
            .keys()
            .filter(|k| {
                let under = format!("{k}/");
                files
                    .range(under.clone()..)
                    .next()
                    .is_some_and(|(next, _)| next.starts_with(&under))
            })
            .cloned()
            .collect();
        for folder in folders {
            files.remove(&folder);
        }
        Ok(Self { files })
    }

    /// The listing's spelling of the file at `path` (any case, either separator).
    pub fn file(&self, path: &str) -> Option<&'a str> {
        self.files.get(&fold(path)).copied()
    }

    /// The files inside the folder `path`: each as (its path below the folder, in the listing's
    /// own spelling; the listing's spelling of the whole path).
    pub fn under(&self, path: &str) -> Vec<(String, &'a str)> {
        let key = fold(path);
        let depth = if key.is_empty() {
            0
        } else {
            key.split('/').count()
        };
        let start = if key.is_empty() {
            String::new()
        } else {
            format!("{key}/")
        };
        self.files
            .range(start.clone()..)
            .take_while(|(k, _)| k.starts_with(&start))
            .map(|(_, given)| {
                let own = normalise(given);
                let rest = own.split('/').skip(depth).collect::<Vec<_>>().join("/");
                (rest, *given)
            })
            .collect()
    }

    /// Every file: (folded path, the listing's spelling), in folded order.
    pub fn files(&self) -> impl Iterator<Item = (&String, &&'a str)> {
        self.files.iter()
    }

    /// The folder (normalised, the listing's case) holding `fomod`, by the rule above: one lookup
    /// in the index per folder level, never a pass over every file per level.
    fn fomod_prefix(&self) -> Option<String> {
        let mut prefix = String::new();
        let mut spelled: Vec<String> = Vec::new();
        loop {
            let fomod = format!("{prefix}fomod/");
            if first_and_last(&self.files, &fomod).is_some() {
                return Some(spelled.join("/"));
            }
            // Exactly one name below `prefix`, and a folder: the first and the last key under it
            // share that name and its `/` (the keys between them do too, being sorted).
            let (first, last) = first_and_last(&self.files, &prefix)?;
            let rest = &first[prefix.len()..];
            let name = &rest[..rest.find('/')?];
            let folder = format!("{prefix}{name}/");
            if !last.starts_with(&folder) {
                return None;
            }
            let own = normalise(self.files[first]);
            spelled.push(own.split('/').nth(spelled.len())?.to_owned());
            prefix = folder;
        }
    }
}

/// The game's Data folders a mod's files go in (lower case).
const DATA_FOLDERS: &[&str] = &[
    "meshes",
    "textures",
    "sound",
    "music",
    "menus",
    "shaders",
    "video",
    "fonts",
    "lodsettings",
    "trees",
    "nvse",
];

/// Plugin and archive extensions that mark a Data folder.
const DATA_FILES: &[&str] = &[".esm", ".esp", ".bsa"];

fn data_folder_in(index: &Listing) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    let mut offer = |parts: &[&str]| {
        let depth = parts.len();
        if best.as_ref().map_or(true, |(d, _)| depth < *d) {
            best = Some((depth, parts.join("/")));
        }
    };
    for (key, given) in index.files() {
        let keys: Vec<&str> = key.split('/').collect();
        if keys.contains(&"fomod") {
            continue;
        }
        let own_path = normalise(given);
        let own: Vec<&str> = own_path.split('/').collect();
        let last = keys.len() - 1;
        if DATA_FILES.iter().any(|ext| keys[last].ends_with(ext)) {
            offer(&own[..last]);
        }
        for k in 0..last {
            if keys[k] == "data" {
                offer(&own[..=k]);
            } else if DATA_FOLDERS.contains(&keys[k]) {
                offer(&own[..k]);
            }
        }
    }
    best.map(|(_, path)| path)
}

/// The shallowest folder (normalised, the listing's case; `""` the top) that holds a plugin or
/// archive, or one of the game's Data folders, or is itself named `Data`, leaving out `fomod`
/// folders; `None` when there is none. On a tie the first in folded order.
pub fn data_folder(listing: &[String]) -> Result<Option<String>, ListingError> {
    Ok(data_folder_in(&Listing::new(listing)?))
}

/// What the archive with this listing holds, or why the listing is not a mod's.
pub fn layout(listing: &[String]) -> Result<Layout, ListingError> {
    let index = Listing::new(listing)?;
    let prefix = index.fomod_prefix();
    let found = |name: &str| {
        prefix
            .as_deref()
            .and_then(|p| index.file(&join(p, name)))
            .map(str::to_owned)
    };
    let info = found("fomod/info.xml");
    if let Some(prefix) = &prefix {
        if let Some(config) = found("fomod/ModuleConfig.xml") {
            return Ok(Layout::Installer {
                prefix: prefix.clone(),
                config,
                info,
            });
        }
        if let Some(script) = found("fomod/script.cs") {
            return Ok(Layout::Script {
                prefix: prefix.clone(),
                script,
                data: data_folder_in(&index),
            });
        }
    }
    Ok(match data_folder_in(&index) {
        Some(data) => Layout::Plain { data, info },
        None => Layout::Unknown,
    })
}
