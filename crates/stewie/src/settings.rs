//! Tweaks' settings: every key its code reads (`data/settings.tsv`), the port's defaults, and the
//! player's `nvse_stewie_tweaks.ini` over them.
//!
//! The port's defaults are VNV's values (the lead's choice, 2026-11-18; docs/FIXES.md says how this
//! differs from Tweaks): Tweaks itself writes every key its INI lacks into the file with its code's
//! default and uses that, so with no file a real install runs with the code's defaults, which turn
//! off almost everything VNV players have on. Here a player's file overlays VNV's values key by key:
//! a key it has wins, a key it lacks keeps VNV's value. With the switch `bMissingKeysAsTweaks=1`
//! (`[nv-rs Stewie]` in Fallout.ini, for a complete file brought from a real install) a key the file
//! lacks takes the code's default instead, as with Tweaks. The port never writes the file.
//!
//! A value is read as Tweaks reads it ([`crate::ini`]); a value Tweaks would refuse (a number it
//! cannot read, or Tweaks' own "missing" mark -19997) counts as missing, as `ModifyTweakIfPresent`
//! counts it, so the default under it stands.

use crate::ini;
use std::collections::HashMap;

/// The table: section, key, Tweaks' code default, VNV's value.
pub const TABLE: &str = include_str!("../data/settings.tsv");

/// How Tweaks reads a key from more than one file (`data/key_rules.tsv`): rule (`main` or `or`),
/// section, key.
pub const KEY_RULES: &str = include_str!("../data/key_rules.tsv");

/// How a key is read across the main INI and the overlay files (`Tweaks\INIs\*.ini`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Read from the main INI only (inside `if (isDefaultINI)`): overlay files' values are ignored.
    MainOnly,
    /// `GET_TWEAKH`: each file's value is ORed in, so an overlay can turn it on, never off.
    Or,
}

/// The rule of every key [`KEY_RULES`] names, by lowercase section and key; every other key: the
/// last file that has it wins.
pub fn rules() -> HashMap<(Vec<u8>, Vec<u8>), Rule> {
    KEY_RULES
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            assert_eq!(f.len(), 3, "key_rules.tsv: {line}");
            let rule = match f[0] {
                "main" => Rule::MainOnly,
                "or" => Rule::Or,
                other => panic!("key_rules.tsv: rule {other}"),
            };
            (key(f[1], f[2]), rule)
        })
        .collect()
}

/// Tweaks' `MISSING_INI_VAL` (`SettingUtils.cpp`): the default `ModifyTweakIfPresent` passes to
/// `GetLongValue`, so a value that reads as it counts as missing.
pub const MISSING: i32 = -19997;

/// One row of [`TABLE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setting {
    pub section: &'static str,
    pub key: &'static str,
    /// Tweaks' code default; `None` for one its source writes as an expression (settled when the
    /// row that reads it is built; the table keeps the expression after a `?`).
    pub code: Option<&'static str>,
    /// VNV's value; `None` when VNV's INI has no such key.
    pub vnv: Option<&'static str>,
}

impl Setting {
    /// The port's default: VNV's value, else the code's.
    pub fn port_default(&self) -> Option<&'static str> {
        self.vnv.or(self.code)
    }
}

/// The rows of [`TABLE`]. `#` lines are notes.
pub fn table() -> Vec<Setting> {
    TABLE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|line| {
            let f: Vec<&'static str> = line.split('\t').collect();
            assert_eq!(f.len(), 4, "settings.tsv: {line}");
            Setting {
                section: f[0],
                key: f[1],
                code: (!f[2].starts_with('?')).then_some(f[2]),
                vnv: (f[3] != "-").then_some(f[3]),
            }
        })
        .collect()
}

type Key = (Vec<u8>, Vec<u8>);

fn key(section: &str, key: &str) -> Key {
    (
        section.as_bytes().to_ascii_lowercase(),
        key.as_bytes().to_ascii_lowercase(),
    )
}

/// What one file did, for the start-up log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRead {
    pub name: String,
    /// Keys of the table it has.
    pub keys: usize,
    /// Of those, the ones whose value differs from the port's default under it.
    pub changed: usize,
}

/// The settings a run reads: the files' values over the defaults.
#[derive(Clone, Debug, Default)]
pub struct Settings {
    /// The defaults: VNV's values, or with the switch the code's, by lowercase section and key.
    base: HashMap<Key, &'static str>,
    /// The files' values, the first file first (a later one wins), each with whether it is the
    /// main INI (an overlay's main-only keys are left out).
    files: Vec<(bool, HashMap<Key, Vec<u8>>)>,
    /// The keys whose [`Rule`] is not "the last file wins".
    rules: HashMap<Key, Rule>,
    /// What each file did.
    pub read: Vec<FileRead>,
}

impl Settings {
    /// The defaults with the `files` (name and bytes, in the order they overlay) over them;
    /// `as_tweaks`: a key the files lack takes the code's default, as with Tweaks.
    pub fn new(files: &[(&str, &[u8])], as_tweaks: bool) -> Settings {
        Settings::with_main(files, true, as_tweaks)
    }

    /// As [`Settings::new`]; `main`: the first file is the main INI (`nvse_stewie_tweaks.ini`) and
    /// the others are overlay files (`Tweaks\INIs\*.ini`), each key read by its [`Rule`]. Without a
    /// main INI (`main` false) every file is an overlay over the defaults.
    pub fn with_main(files: &[(&str, &[u8])], main: bool, as_tweaks: bool) -> Settings {
        let table = table();
        let base: HashMap<Key, &'static str> = table
            .iter()
            .filter_map(|s| {
                let value = if as_tweaks { s.code } else { s.port_default() };
                value.map(|v| (key(s.section, s.key), v))
            })
            .collect();
        let mut settings = Settings {
            base,
            files: Vec::new(),
            rules: rules(),
            read: Vec::new(),
        };
        for (i, (name, bytes)) in files.iter().enumerate() {
            let is_main = main && i == 0;
            let mut values = ini::values(bytes);
            if !is_main {
                values.retain(|k, _| settings.rules.get(k) != Some(&Rule::MainOnly));
            }
            let mut keys = 0;
            let mut changed = 0;
            for s in &table {
                let Some(value) = values.get(&key(s.section, s.key)) else {
                    continue;
                };
                keys += 1;
                let under = settings.raw(s.section, s.key);
                if under.as_deref() != Some(value.as_slice()) {
                    changed += 1;
                }
            }
            settings.files.push((is_main, values));
            settings.read.push(FileRead {
                name: name.to_string(),
                keys,
                changed,
            });
        }
        settings
    }

    /// The text of `key` in `section`: the last file that has it, else the default (`None`: neither,
    /// or a code default not settled yet).
    pub fn raw(&self, section: &str, name: &str) -> Option<Vec<u8>> {
        let k = key(section, name);
        self.files
            .iter()
            .rev()
            .find_map(|(_, f)| f.get(&k).cloned())
            .or_else(|| self.base.get(&k).map(|v| v.as_bytes().to_vec()))
    }

    /// An integer setting as `ModifyTweakIfPresent` reads it: the last file whose value reads as a
    /// number (and not as [`MISSING`]), else the default's, else `default`.
    pub fn long(&self, section: &str, name: &str, default: i32) -> i32 {
        let k = key(section, name);
        if self.rules.get(&k) == Some(&Rule::Or) {
            return self.long_ored(&k, default);
        }
        for (_, f) in self.files.iter().rev() {
            match ini::long_value(f.get(&k).map(Vec::as_slice), MISSING) {
                MISSING => continue,
                n => return n,
            }
        }
        self.base_long(&k, default)
    }

    /// A `GET_TWEAKH` key: the main INI's value (read as `GetLongValue(.., 0)`, so one that is no
    /// number counts 0) when it has the key, else the default's; then each overlay file's value
    /// ORed in.
    fn long_ored(&self, k: &Key, default: i32) -> i32 {
        let (main, overlays) = match self.files.split_first() {
            Some(((true, f), rest)) => (f.get(k), rest),
            _ => (None, self.files.as_slice()),
        };
        let first = match main {
            Some(v) => ini::long_value(Some(v), 0),
            None => self.base_long(k, default),
        };
        overlays.iter().fold(first, |value, (_, f)| {
            value | ini::long_value(f.get(k).map(Vec::as_slice), 0)
        })
    }

    /// The default under every file: the table's value, else `default`.
    fn base_long(&self, k: &Key, default: i32) -> i32 {
        match ini::long_value(self.base.get(k).map(|v| v.as_bytes()), MISSING) {
            MISSING => default,
            n => n,
        }
    }

    /// The start-up log's line for each file read.
    pub fn log_lines(&self) -> Vec<String> {
        self.read
            .iter()
            .map(|r| {
                format!(
                    "lStewieAl's Tweaks: read {}: {} of its {} keys changed a value",
                    r.name, r.changed, r.keys
                )
            })
            .collect()
    }
}
