//! Installed mods: `mod installs/<NN Name>/`, each with its [`crate::RECORD`].
//!
//! The record is plain UTF-8 text, one `key: value` a line, so a player can read it and a later
//! version can add keys (unknown ones are kept as they are by [`Record::parse`] and written back):
//!
//! ```text
//! name: Gun Pack
//! version: 1.2
//! author: Someone
//! archive: Gun Pack 1.2-1234-1-2.7z
//! archive size: 1048576
//! choice: Options / DLC patches / Honest Hearts
//! file: GunPack.esp
//! file: Meshes/Weapons/gun.nif
//! ```

use std::path::{Path, PathBuf};

/// One installed mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// Its place in the order (the folder name's number).
    pub number: u32,
    /// The rest of the folder's name.
    pub name: String,
    pub folder: PathBuf,
}

/// What an install recorded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    pub name: String,
    pub version: String,
    pub author: String,
    /// The archive's file name and size (to tell it again when it is put in the mods folder).
    pub archive: String,
    pub archive_size: u64,
    /// The choices made, `Step / Group / Option` each.
    pub choices: Vec<String>,
    /// The files installed, relative, `/` between the parts.
    pub files: Vec<String>,
    /// Lines with keys this version does not know, kept as they are.
    pub other: Vec<String>,
}

impl Record {
    /// The record's text.
    pub fn text(&self) -> String {
        let mut out = String::new();
        let mut line = |key: &str, value: &str| {
            // One line each: a line break in a value would start another key.
            let value: String = value
                .chars()
                .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                .collect();
            out.push_str(&format!("{key}: {value}\n"));
        };
        line("name", &self.name);
        line("version", &self.version);
        line("author", &self.author);
        line("archive", &self.archive);
        line("archive size", &self.archive_size.to_string());
        for c in &self.choices {
            line("choice", c);
        }
        for f in &self.files {
            line("file", f);
        }
        for o in &self.other {
            out.push_str(o);
            out.push('\n');
        }
        out
    }

    /// The record in `text`; lines it cannot read are kept in [`Record::other`].
    pub fn parse(text: &str) -> Record {
        let mut r = Record::default();
        for raw in text.lines() {
            let line = raw.trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(": ") else {
                r.other.push(line.to_owned());
                continue;
            };
            let value = value.to_owned();
            match key {
                "name" => r.name = value,
                "version" => r.version = value,
                "author" => r.author = value,
                "archive" => r.archive = value,
                "archive size" => r.archive_size = value.trim().parse().unwrap_or(0),
                "choice" => r.choices.push(value),
                "file" => r.files.push(value),
                _ => r.other.push(line.to_owned()),
            }
        }
        r
    }
}

/// The most installed mods: the largest mod lists players keep have a few thousand.
pub const MAX_INSTALLS: usize = 10_000;

/// The installed mods in `installs`, in their order: folders named `<number> <name>`; anything
/// else there is not a mod and is left alone.
pub fn installed(installs: &Path) -> std::io::Result<Vec<Installed>> {
    let mut folders = 0usize;
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(installs) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        folders += 1;
        if folders > MAX_INSTALLS {
            return Err(std::io::Error::other(format!(
                "{} holds more than {MAX_INSTALLS} mods",
                installs.display()
            )));
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some((number, name)) = file_name.split_once(' ') else {
            continue;
        };
        let Ok(number) = number.parse::<u32>() else {
            continue;
        };
        out.push(Installed {
            number,
            name: name.to_owned(),
            folder: entry.path(),
        });
    }
    out.sort_by(|a, b| (a.number, &a.name).cmp(&(b.number, &b.name)));
    Ok(out)
}

/// A name every system can hold as a folder name: no `< > : " / \ | ? *` or control characters,
/// no dot or space at the end, not one of Windows' device names, at most 60 characters; `Mod`
/// when nothing is left.
pub fn safe_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    out = out.chars().take(60).collect();
    let trimmed = out.trim_end_matches(['.', ' ']).to_owned();
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if trimmed.is_empty() {
        "Mod".to_owned()
    } else if device {
        format!("{trimmed} mod")
    } else {
        trimmed
    }
}

/// The folder for a new mod named `name`: the number after the last one (two digits at least),
/// then [`safe_name`]. Not made yet.
pub fn next_folder(installs: &Path, name: &str) -> std::io::Result<PathBuf> {
    let next = installed(installs)?
        .iter()
        .map(|i| i.number)
        .max()
        .map_or(1, |n| n + 1);
    Ok(installs.join(format!("{next:02} {}", safe_name(name))))
}

/// Removes an installed mod: its folder and everything in it. Refused (nothing removed) when the
/// folder is not directly inside `installs`.
pub fn remove(installs: &Path, mod_folder: &Path) -> std::io::Result<()> {
    let inside = mod_folder.parent() == Some(installs)
        && mod_folder
            .file_name()
            .is_some_and(|n| !n.is_empty() && n != "." && n != "..");
    if !inside {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "{} is not a mod's folder in {}",
                mod_folder.display(),
                installs.display()
            ),
        ));
    }
    crate::fsops::remove_mod_folder(installs, mod_folder)
}

/// The folders "Play with mods" lays over the game, in order: each installed mod's, then the loose
/// `mods` folder.
pub fn layers(installs: &Path, loose: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = installed(installs)?.into_iter().map(|i| i.folder).collect();
    out.push(loose.to_path_buf());
    Ok(out)
}
