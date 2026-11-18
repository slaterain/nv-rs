//! Where things are in a mod archive, from its list of files (paths with `/` or `\`).
//!
//! The `fomod` folder is found as Mod Organizer 2 finds it: at the top, else, when the top holds
//! exactly one entry and that is a folder, inside it, and so on down (archives often wrap
//! everything in `ModName v1.2\`). Names are compared without case; `__MACOSX` folders (left by
//! Macs' zip tool) are not looked at. File sources in the installer are relative to the folder
//! holding `fomod` (the prefix).
//!
//! An archive without an installer, or with a C# `script.cs` installer this port does not run,
//! may still hold a plain Data layout: [`data_folder`] finds the shallowest folder that looks like
//! a Data folder (holds a plugin or archive, or one of the game's Data folders), as managers do.

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

/// `a` and `b` joined with `/` (either may be empty).
pub fn join(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_owned(),
        (_, true) => a.to_owned(),
        _ => format!("{a}/{b}"),
    }
}

fn skipped(path: &str) -> bool {
    path.split('/').any(|p| p.eq_ignore_ascii_case("__MACOSX"))
}

/// The listing's entries as (normalised, lower case, as given), without `__MACOSX`.
fn entries(listing: &[String]) -> Vec<(String, String, &str)> {
    listing
        .iter()
        .map(|e| {
            let n = normalise(e);
            let lower = n.to_ascii_lowercase();
            (n, lower, e.as_str())
        })
        .filter(|(n, _, _)| !n.is_empty() && !skipped(n))
        .collect()
}

/// The folder (normalised, original case) holding `fomod`, by the rule above.
fn fomod_prefix(entries: &[(String, String, &str)]) -> Option<String> {
    let mut base = String::new();
    loop {
        let under = |path: &str| -> Option<String> {
            if base.is_empty() {
                Some(path.to_owned())
            } else {
                path.strip_prefix(&format!("{}/", base.to_ascii_lowercase()))
                    .map(str::to_owned)
            }
        };
        // The entries below `base`, as (first part, whether it is a folder).
        let mut tops: Vec<(String, bool)> = Vec::new();
        for (_, lower, _) in entries {
            let Some(rest) = under(lower) else { continue };
            let (first, is_folder) = match rest.find('/') {
                Some(at) => (&rest[..at], true),
                None => (rest.as_str(), false),
            };
            if first == "fomod" && is_folder {
                return Some(base);
            }
            if !tops.iter().any(|(t, f)| t == first && *f == is_folder) {
                tops.push((first.to_owned(), is_folder));
            }
        }
        match tops.as_slice() {
            [(_, true)] => {
                // The one folder, in the listing's own case.
                let first = entries
                    .iter()
                    .find_map(|(n, lower, _)| {
                        under(lower)?;
                        let skip = if base.is_empty() { 0 } else { base.len() + 1 };
                        let rest = &n[skip..];
                        rest.find('/').map(|at| rest[..at].to_owned())
                    })
                    .unwrap_or_default();
                base = join(&base, &first);
            }
            _ => return None,
        }
    }
}

/// The listing's own spelling of the file `path` (normalised, any case).
fn spelled(entries: &[(String, String, &str)], path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    entries
        .iter()
        .find(|(_, l, _)| *l == lower)
        .map(|(_, _, given)| (*given).to_owned())
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
/// folders; `None` when there is none. On a tie the first in the listing's order.
pub fn data_folder(listing: &[String]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    let mut offer = |parts: &[&str]| {
        let depth = parts.len();
        if best.as_ref().map_or(true, |(d, _)| depth < *d) {
            best = Some((depth, parts.join("/")));
        }
    };
    for (n, _, _) in entries(listing) {
        let parts: Vec<&str> = n.split('/').collect();
        if parts.iter().any(|p| p.eq_ignore_ascii_case("fomod")) {
            continue;
        }
        let last = parts.len() - 1;
        let name = parts[last].to_ascii_lowercase();
        if DATA_FILES.iter().any(|ext| name.ends_with(ext)) {
            offer(&parts[..last]);
        }
        for k in 0..last {
            let part = parts[k].to_ascii_lowercase();
            if part == "data" {
                offer(&parts[..=k]);
            } else if DATA_FOLDERS.contains(&part.as_str()) {
                offer(&parts[..k]);
            }
        }
    }
    best.map(|(_, path)| path)
}

/// What the archive with this listing holds.
pub fn layout(listing: &[String]) -> Layout {
    let all = entries(listing);
    let prefix = fomod_prefix(&all);
    let info = prefix
        .as_deref()
        .and_then(|p| spelled(&all, &join(p, "fomod/info.xml")));
    if let Some(prefix) = &prefix {
        if let Some(config) = spelled(&all, &join(prefix, "fomod/ModuleConfig.xml")) {
            return Layout::Installer {
                prefix: prefix.clone(),
                config,
                info,
            };
        }
        if let Some(script) = spelled(&all, &join(prefix, "fomod/script.cs")) {
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
