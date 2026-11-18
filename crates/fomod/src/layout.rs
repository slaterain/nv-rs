//! Where things are in a mod archive, from its list of files (paths with `/` or `\`).
//!
//! The listing may come straight from an archive: folder entries (`Wrapper/`, or a bare
//! `Wrapper` from a 7z) are dropped, so only files are left ([`Listing`]). Names are compared as
//! Windows and macOS compare them ([`fold`]); `__MACOSX` folders (left by Macs' zip tool) are not
//! looked at.
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

/// An archive's files, indexed once: by [`fold`]ed path, each with the listing's own spelling.
#[derive(Debug, Clone, Default)]
pub struct Listing<'a> {
    files: BTreeMap<String, &'a str>,
}

impl<'a> Listing<'a> {
    /// The files of `listing`: folder entries (ending in `/` or `\`, or a name another entry is
    /// inside of), `__MACOSX` and empty paths left out; of two entries that fold to one path, the
    /// first.
    pub fn new(listing: &'a [String]) -> Self {
        let mut files: BTreeMap<String, &'a str> = BTreeMap::new();
        for given in listing {
            if given.ends_with(['/', '\\']) {
                continue;
            }
            let key = fold(given);
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
        Self { files }
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
}

/// The folder (normalised, the listing's case) holding `fomod`, by the rule above.
fn fomod_prefix(listing: &Listing) -> Option<String> {
    let paths: Vec<(Vec<String>, Vec<String>)> = listing
        .files()
        .map(|(key, given)| {
            let own: Vec<String> = normalise(given).split('/').map(str::to_owned).collect();
            (key.split('/').map(str::to_owned).collect(), own)
        })
        .collect();
    let mut base: Vec<String> = Vec::new();
    let mut spelled: Vec<String> = Vec::new();
    loop {
        // The names right below `base`: (folded) -> (own spelling, is a folder).
        let mut tops: BTreeMap<&str, (&str, bool)> = BTreeMap::new();
        for (key, own) in &paths {
            if key.len() <= base.len() || key[..base.len()] != base[..] {
                continue;
            }
            let at = base.len();
            let folder = key.len() > at + 1;
            let entry = tops.entry(&key[at]).or_insert((&own[at], folder));
            entry.1 |= folder;
        }
        if tops.get("fomod").is_some_and(|(_, folder)| *folder) {
            return Some(spelled.join("/"));
        }
        let mut only = tops.into_iter();
        match (only.next(), only.next()) {
            (Some((key, (own, true))), None) => {
                base.push(key.to_owned());
                spelled.push(own.to_owned());
            }
            _ => return None,
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

/// The shallowest folder (normalised, the listing's case; `""` the top) that holds a plugin or
/// archive, or one of the game's Data folders, or is itself named `Data`, leaving out `fomod`
/// folders; `None` when there is none. On a tie the first in folded order.
pub fn data_folder(listing: &[String]) -> Option<String> {
    let index = Listing::new(listing);
    let mut best: Option<(usize, String)> = None;
    let mut offer = |parts: &[String]| {
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
        let own: Vec<String> = normalise(given).split('/').map(str::to_owned).collect();
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

/// What the archive with this listing holds.
pub fn layout(listing: &[String]) -> Layout {
    let index = Listing::new(listing);
    let prefix = fomod_prefix(&index);
    let found = |name: &str| {
        prefix
            .as_deref()
            .and_then(|p| index.file(&join(p, name)))
            .map(str::to_owned)
    };
    let info = found("fomod/info.xml");
    if let Some(prefix) = &prefix {
        if let Some(config) = found("fomod/ModuleConfig.xml") {
            return Layout::Installer {
                prefix: prefix.clone(),
                config,
                info,
            };
        }
        if let Some(script) = found("fomod/script.cs") {
            return Layout::Script {
                prefix: prefix.clone(),
                script,
                data: data_folder(listing),
            };
        }
    }
    match data_folder(listing) {
        Some(data) => Layout::Plain { data, info },
        None => Layout::Unknown,
    }
}
