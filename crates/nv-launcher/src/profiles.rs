//! The Settings screen's files: both ways of playing's profiles (`game`, then `mods`), each with the
//! game's `Fallout.ini` (nv-rs's `[nv-rs ...]` sections) and `FalloutPrefs.ini`, in nv-rs's own
//! folder (Windows: the package's `userdata`), and the older `settings.ini` stage, kept as
//! the old Settings.cmd kept them (its distribution/Settings.ps1, which this replaced):
//!
//! - a choice goes into a profile's files when its `Fallout.ini` has nv-rs's sections (Play, or
//!   the game's first start, made them ready) and no `settings.ini` waits for it; otherwise into
//!   that `settings.ini`, every option a line, which Play.exe (and, off Windows, the game's first
//!   start) moves into the files: the shared one while it is there, else off Windows the
//!   profile's own (the game's first start makes only the profile it plays ready);
//! - what the screen shows is the game profile's files, `settings.ini`'s lines over them while it
//!   is there (as Play moves it in);
//! - a change writes only its row's keys, in place, in both profiles, the mods profile first and
//!   the game profile last (it is what the screen shows, so after a save that failed half way the
//!   screen still shows what was not saved, and choosing again finishes it);
//! - every file is written whole beside itself first (`NAME.part`) and then takes its place in one
//!   step, its read-only mark kept, so a full disk leaves every file whole; a file that cannot be
//!   read stops the save before anything is written.

use crate::settings::{Files, Row, OWN};
use game_profile::game_ini;
use game_profile::settings::{self as profile_settings, Settings};
use std::path::{Path, PathBuf};

/// One profile's game folder in nv-rs's own folder (`<own>/<name>/Documents/My Games/FalloutNV`).
pub fn profile_dir(own: &Path, name: &str) -> PathBuf {
    own.join(name)
        .join("Documents")
        .join("My Games")
        .join("FalloutNV")
}

/// The Settings screen's files.
pub struct Profiles {
    /// The game profile's folder, then the mods profile's.
    pub dirs: [PathBuf; 2],
    /// `settings.ini` in nv-rs's own folder: the settings for both profiles while it is there.
    pub pending: PathBuf,
    /// Off Windows, each profile's own `settings.ini` (`<own>/<profile>/settings.ini`) for a
    /// profile the game has not made ready yet (play.sh names it to the game for that profile's
    /// start); on Windows `None`: Play.exe makes both profiles ready at once.
    pub own_pending: Option<[PathBuf; 2]>,
}

fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn bytes_of(text: &str) -> Vec<u8> {
    text.chars().map(|c| c as u32 as u8).collect()
}

/// A file as text, one character a byte; a missing one is empty (any other failure stops).
fn read(path: &Path) -> std::io::Result<String> {
    match std::fs::read(path) {
        Ok(b) => Ok(text_of(&b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(std::io::Error::new(
            e.kind(),
            format!("{}: {e}", path.display()),
        )),
    }
}

/// The error `e` naming `path`.
fn naming(path: &Path, e: std::io::Error) -> std::io::Error {
    std::io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}

/// `bytes` written whole beside `path` (`NAME.part`), not yet in its place.
fn write_part(path: &Path, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let _ = std::fs::remove_file(&part);
    if let Err(e) = std::fs::write(&part, bytes) {
        let _ = std::fs::remove_file(&part);
        return Err(naming(path, e));
    }
    Ok(part)
}

/// Every file of `files` (a path and its new bytes) written whole beside itself first, and only
/// when all of them are, each put in its place: a full disk or a refused file leaves every file
/// as it was. A file that cannot take its place (held open by another program) stops there,
/// named; the ones before it are in place and the rest are as they were, their `.part` files gone.
fn write_all(files: &[(PathBuf, Vec<u8>)]) -> std::io::Result<()> {
    let mut parts = Vec::new();
    for (path, bytes) in files {
        match write_part(path, bytes) {
            Ok(part) => parts.push(part),
            Err(e) => {
                for part in &parts {
                    let _ = std::fs::remove_file(part);
                }
                return Err(e);
            }
        }
    }
    for (i, ((path, _), part)) in files.iter().zip(&parts).enumerate() {
        if let Err(e) = put_in_place(path, part) {
            for part in &parts[i + 1..] {
                let _ = std::fs::remove_file(part);
            }
            return Err(e);
        }
    }
    Ok(())
}

/// `part` (from [`write_part`]) put in place of `path` in one step; a read-only mark is lifted
/// for that step and put back. A failure names the file and leaves no `.part` behind.
fn put_in_place(path: &Path, part: &Path) -> std::io::Result<()> {
    replace_with(path, part).map_err(|e| {
        let _ = std::fs::remove_file(part);
        naming(path, e)
    })
}

// `set_readonly(false)` clears the read-only attribute (Windows) or gives the owner write (Unix).
#[allow(clippy::permissions_set_readonly_false)]
fn replace_with(path: &Path, part: &Path) -> std::io::Result<()> {
    let read_only = std::fs::metadata(path).is_ok_and(|m| m.permissions().readonly());
    // Windows refuses to replace a read-only file; elsewhere the folder decides, and the new file
    // gets the mark below.
    if read_only && cfg!(windows) {
        let mut p = std::fs::metadata(path)?.permissions();
        p.set_readonly(false);
        std::fs::set_permissions(path, p)?;
    }
    let renamed = std::fs::rename(part, path);
    if read_only {
        if let Ok(m) = std::fs::metadata(path) {
            let mut p = m.permissions();
            p.set_readonly(true);
            let _ = std::fs::set_permissions(path, p);
        }
    }
    renamed
}

/// settings.ini's names and values for what `files` show (Settings.ps1's `Get-Choices`).
pub fn settings_lines(files: &Files) -> Vec<(&'static str, String)> {
    let now = |row: Row| row.current(files);
    let pick = |row: Row, yes: &str, a: &str, b: &str| {
        if now(row).starts_with(yes) {
            a.to_string()
        } else {
            b.to_string()
        }
    };
    let limit = |row: Row| match now(row).as_str() {
        "No limit" => "0".to_string(),
        v => v.to_string(),
    };
    vec![
        (
            "Renderer",
            pick(Row::Renderer, "Direct3D 9", "d3d9", "vulkan"),
        ),
        ("Borderless", pick(Row::DisplayMode, "Window", "0", "1")),
        ("Resolution", now(Row::Resolution)),
        ("AntiAliasing", now(Row::AntiAliasing)),
        ("Anisotropy", now(Row::Anisotropy)),
        ("VSync", now(Row::VSync)),
        (
            "Preset",
            pick(Row::GameFixes, "Vanilla", "vanilla", "fixes"),
        ),
        ("FrameLimit", limit(Row::FrameLimit)),
        ("MenuFrameLimit", limit(Row::MenuFrameLimit)),
        (
            "NV_SHOW_FPS",
            pick(Row::FrameRateCounter, "Shown", "1", "0"),
        ),
        (
            "NV_THREADS",
            pick(Row::GameThreads, "One at a time", "stepped", "real"),
        ),
        (
            "NV_RENDER_THREAD",
            pick(Row::DrawingThread, "Main thread", "0", "1"),
        ),
    ]
}

/// The screen's row and choice that a settings.ini line names (the opposite of
/// [`settings_lines`]): the tests choose by those names, as the old Settings.cmd tests did.
#[cfg(test)]
pub fn choice_for(name: &str, value: &str) -> Option<(Row, String)> {
    let two = |row: Row, yes: &str, a: &str, b: &str| {
        Some((row, if value == yes { a } else { b }.to_string()))
    };
    let limit = |row: Row| {
        Some((
            row,
            if value == "0" { "No limit" } else { value }.to_string(),
        ))
    };
    match name {
        "Renderer" => two(
            Row::Renderer,
            "d3d9",
            "Direct3D 9 (the original's)",
            crate::settings::OWN_RENDERER,
        ),
        "Borderless" => two(Row::DisplayMode, "0", "Window", "Borderless fullscreen"),
        "Resolution" => Some((Row::Resolution, value.to_string())),
        "AntiAliasing" => Some((Row::AntiAliasing, value.to_string())),
        "Anisotropy" => Some((Row::Anisotropy, value.to_string())),
        "VSync" => Some((Row::VSync, value.to_string())),
        "Preset" => two(
            Row::GameFixes,
            "vanilla",
            "Vanilla (the original game)",
            "Fixes (recommended)",
        ),
        "FrameLimit" => limit(Row::FrameLimit),
        "MenuFrameLimit" => limit(Row::MenuFrameLimit),
        "NV_SHOW_FPS" => two(
            Row::FrameRateCounter,
            "1",
            "Shown (F11 hides it)",
            "Hidden (F11 shows it)",
        ),
        "NV_THREADS" => two(
            Row::GameThreads,
            "stepped",
            "One at a time (slower)",
            "Side by side (recommended)",
        ),
        "NV_RENDER_THREAD" => two(
            Row::DrawingThread,
            "0",
            "Main thread (slower)",
            "Own thread (recommended)",
        ),
        _ => None,
    }
}

/// The names Settings.ps1 knew: a settings.ini line by another name is kept as it is.
const KNOWN: [&str; 12] = [
    "Renderer",
    "Borderless",
    "Resolution",
    "AntiAliasing",
    "Anisotropy",
    "VSync",
    "Preset",
    "FrameLimit",
    "MenuFrameLimit",
    "NV_SHOW_FPS",
    "NV_THREADS",
    "NV_RENDER_THREAD",
];

/// Lines Settings.ps1 dropped: switches the game makes itself from the lines above.
const DROPPED: [&str; 4] = [
    "NV_PRESET",
    "NV_FRAME_LIMIT",
    "NV_MENU_FRAME_LIMIT",
    "NV_FIXES",
];

impl Profiles {
    pub fn at(own: &Path) -> Profiles {
        Profiles {
            dirs: [profile_dir(own, "game"), profile_dir(own, "mods")],
            pending: own.join("settings.ini"),
            // Off Windows the game's first start makes one profile ready and moves settings.ini
            // into that one only (host first_profile), so the other keeps its own file, which
            // play.sh names to the game for that profile's start.
            own_pending: (!cfg!(windows))
                .then(|| [own.join("game"), own.join("mods")].map(|d| d.join("settings.ini"))),
        }
    }

    fn files(dir: &Path) -> std::io::Result<Files> {
        Ok(Files {
            ini: read(&dir.join("Fallout.ini"))?,
            prefs: read(&dir.join("FalloutPrefs.ini"))?,
        })
    }

    /// The settings.ini that profile `i` takes its choices from at its next start: the shared one
    /// while it is there (it is the settings for both), else (off Windows) the profile's own.
    fn pending_for(&self, i: usize) -> &Path {
        match &self.own_pending {
            Some(own) if !self.pending.exists() => &own[i],
            _ => &self.pending,
        }
    }

    fn pending_at(path: &Path) -> std::io::Result<Option<(String, Settings)>> {
        match std::fs::read(path) {
            Ok(b) => {
                let text = profile_settings::decode(&b);
                let parsed = Settings::parse(&text);
                Ok(Some((text, parsed)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(naming(path, e)),
        }
    }

    /// Off Windows a shared settings.ini (from an older package, or made before the per-profile
    /// files) would be moved by the game's first start into the profile played first only: at the
    /// first save it is split into each profile's own file (over one that is there already, its lines
    /// winning line by line, as play.sh and the screen let the shared file win), and then renamed aside as Play renames a moved one, so the choice reaches both.
    fn split_shared(&self) -> std::io::Result<()> {
        let Some(own) = &self.own_pending else {
            return Ok(());
        };
        let raw = match std::fs::read(&self.pending) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(naming(&self.pending, e)),
        };
        let mut writes = Vec::new();
        for path in own {
            let bytes = match std::fs::read(path) {
                Ok(own) => bytes_of(&shared_over(&text_of(&raw), &text_of(&own))),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => raw.clone(),
                Err(e) => return Err(naming(path, e)),
            };
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| naming(dir, e))?;
            }
            writes.push((path.clone(), bytes));
        }
        write_all(&writes)?;
        let moved = profile_settings::migrated_name(&self.pending);
        std::fs::rename(&self.pending, &moved).map_err(|e| naming(&self.pending, e))
    }

    /// Whether profile `i` takes a choice in its files: they have nv-rs's sections (Play, or the
    /// game's first start, made them ready) and no settings.ini waits for it.
    fn ready(&self, i: usize) -> std::io::Result<bool> {
        Ok(Self::pending_at(self.pending_for(i))?.is_none()
            && game_ini::has_sections(&Self::files(&self.dirs[i])?.ini))
    }

    /// Whether the choices go into the files: every profile made ready, no settings.ini.
    pub fn in_files(&self) -> std::io::Result<bool> {
        Ok(self.ready(0)? && self.ready(1)?)
    }

    /// What the screen shows: the game profile's files, its settings.ini's lines over them while
    /// it is there, each line on its own (one the game cannot use is left out, the others kept).
    pub fn shown(&self) -> std::io::Result<Files> {
        let game = Self::files(&self.dirs[0])?;
        Ok(match Self::pending_at(self.pending_for(0))? {
            None => game,
            Some((_, pending)) => {
                let pending = usable(&pending);
                Files {
                    ini: game_ini::apply_choices(&game.ini, Some(&pending)).0,
                    prefs: profile_settings::apply_to_prefs(&game.prefs, &pending.prefs_lines()),
                }
            }
        })
    }

    /// `row` set to `choice`: in place in each profile made ready, and in the settings.ini of each
    /// one that is not (all its options a line, the choice made, the lines of other names kept);
    /// on Windows, while either profile is not ready, in the shared settings.ini only.
    /// The mods profile first, the game profile (what the screen shows) last; every file written
    /// whole beside itself before any takes its place. Nothing is written when a file cannot be
    /// read.
    pub fn save(&self, row: Row, choice: &str) -> std::io::Result<()> {
        self.split_shared()?;
        let shown = self.shown()?;
        // Windows (one shared settings.ini): while a profile is not ready, the choices go into
        // settings.ini only, and Play.exe moves them into both at its next start.
        let shared_only = self.own_pending.is_none() && !self.in_files()?;
        let mut writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();
        for i in [1, 0] {
            if !shared_only && self.ready(i)? {
                let dir = &self.dirs[i];
                let files = Self::files(dir)?;
                let new = row.apply(&files, choice);
                if new.prefs != files.prefs && dir.join("FalloutPrefs.ini").exists() {
                    writes.push((dir.join("FalloutPrefs.ini"), bytes_of(&new.prefs)));
                }
                if new.ini != files.ini {
                    writes.push((dir.join("Fallout.ini"), bytes_of(&new.ini)));
                }
                continue;
            }
            let path = self.pending_for(i).to_path_buf();
            if writes.iter().any(|(p, _)| *p == path) {
                continue;
            }
            let kept: Vec<String> = match Self::pending_at(&path)? {
                Some((text, _)) => text
                    .lines()
                    .filter(|l| {
                        l.split_once('=').is_some_and(|(name, _)| {
                            let name = name.trim();
                            !name.starts_with('#')
                                && !name.starts_with(';')
                                && !is_one_of(name, &KNOWN)
                                && !is_one_of(name, &DROPPED)
                                && !row.variable().is_some_and(|v| v.eq_ignore_ascii_case(name))
                        })
                    })
                    .map(str::to_string)
                    .collect(),
                None => Vec::new(),
            };
            let chosen = row.apply(&shown, choice);
            let mut lines = vec![
                "# nv-rs settings (written by nv-rs's launcher; Play moves them into the game's files)"
                    .to_string(),
            ];
            let own_row = name_of(row);
            lines.extend(
                settings_lines(&chosen)
                    .into_iter()
                    .map(|(name, value)| match own_row {
                        // A row set to "Game's setting" says so (the game's own file decides).
                        Some(n) if n == name && choice == OWN => format!("{name}={OWN}"),
                        // Off Windows the game's own renderer, whatever a profile holds: the
                        // real Direct3D 9 is a Windows API, so it is never passed on.
                        _ if name == "Renderer" && self.own_pending.is_some() => {
                            format!("{name}=vulkan")
                        }
                        _ => format!("{name}={value}"),
                    }),
            );
            // A table row: its own switch's line, which Play (or the game's first start) moves
            // into the key. A fix row has one only when chosen here, or when a Fallout.ini this
            // settings.ini reaches holds the key (only a line can change it there: then it hands
            // it back to the preset); with none it follows the preset. The shared file reaches
            // both profiles, a profile's own only its own, each read as it is before this save
            // (a profile that is not ready takes nothing in place).
            let reached = if path == self.pending {
                &self.dirs[..]
            } else {
                std::slice::from_ref(&self.dirs[i])
            };
            let held = reached
                .iter()
                .any(|d| Self::files(d).is_ok_and(|f| row.chosen_here(&f).is_some()));
            if let Some(line) = row.pending_line(&chosen, held) {
                lines.push(line);
            }
            lines.extend(kept);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| naming(dir, e))?;
            }
            writes.push((path, (lines.join("\r\n") + "\r\n").into_bytes()));
        }
        write_all(&writes)
    }
}

/// A shared settings.ini (`shared`) laid over a profile's own (`own`): all of the shared file's
/// lines, then the own file's lines of the names the shared file does not have (names compared
/// without case, as the game reads them); the own file's comments go.
fn shared_over(shared: &str, own: &str) -> String {
    let name = |l: &str| {
        l.split_once('=')
            .map(|(n, _)| n.trim().to_ascii_lowercase())
            .filter(|n| !n.starts_with('#') && !n.starts_with(';'))
    };
    let named: Vec<String> = shared.lines().filter_map(name).collect();
    let mut lines: Vec<&str> = shared.lines().collect();
    lines.extend(
        own.lines()
            .filter(|l| name(l).is_some_and(|n| !named.contains(&n))),
    );
    lines.join("\r\n") + "\r\n"
}

/// Whether `name` is one of `names`, its case aside (as the game and Settings.ps1 read them).
fn is_one_of(name: &str, names: &[&str]) -> bool {
    names.iter().any(|n| n.eq_ignore_ascii_case(name))
}

/// settings.ini's name for one of the twelve rows.
fn name_of(row: Row) -> Option<&'static str> {
    crate::settings::ROWS
        .iter()
        .position(|r| *r == row)
        .map(|i| KNOWN[i])
}

/// `pending`'s lines the game can use, each on its own: one that cannot be read (a limit that is
/// not a number, a preset that is neither) is left out and the others stay (one bad line must
/// not turn a Vanilla choice into Fixes).
fn usable(pending: &Settings) -> Settings {
    Settings(
        pending
            .0
            .iter()
            .filter(|(k, v)| {
                let one = Settings([((*k).clone(), (*v).clone())].into_iter().collect());
                one.game_switches().is_ok()
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nv-launcher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const SECTIONS: &str = "[Display]\r\niPresentInterval=1\r\n[nv-rs Display]\r\n;the renderer\r\nsRenderer=vulkan\r\niFrameLimit=0\r\n[nv-rs General]\r\nsPreset=fixes\r\n";
    const PREFS: &str = "[Display]\r\niSize W=1920\r\niSize H=1080\r\niMultiSample=4\r\n";

    /// Both profiles made ready, by Windows' rule (one shared settings.ini; Play.exe makes both
    /// profiles ready at once), whatever system the tests run on.
    fn ready(own: &Path) -> Profiles {
        let mut p = Profiles::at(own);
        p.own_pending = None;
        for dir in &p.dirs {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("Fallout.ini"), SECTIONS).unwrap();
            std::fs::write(dir.join("FalloutPrefs.ini"), PREFS).unwrap();
        }
        p
    }

    /// With both profiles made ready, a choice changes only its own keys, in both profiles, in
    /// place; every other byte stays.
    #[test]
    fn a_choice_changes_its_own_keys_in_both_profiles() {
        let own = scratch("both");
        let p = ready(&own);
        assert!(p.in_files().unwrap());
        p.save(Row::FrameLimit, "144").unwrap();
        p.save(Row::Resolution, "2560x1440").unwrap();
        for dir in &p.dirs {
            let ini = std::fs::read_to_string(dir.join("Fallout.ini")).unwrap();
            assert_eq!(ini, SECTIONS.replace("iFrameLimit=0", "iFrameLimit=144"));
            let prefs = std::fs::read_to_string(dir.join("FalloutPrefs.ini")).unwrap();
            assert_eq!(prefs, PREFS.replace("1920", "2560").replace("1080", "1440"));
            assert!(!dir.join("Fallout.ini.part").exists());
        }
        let shown = p.shown().unwrap();
        assert_eq!(Row::FrameLimit.current(&shown), "144");
        // "Game's setting" writes nothing.
        p.save(Row::AntiAliasing, OWN).unwrap();
        let prefs = std::fs::read_to_string(p.dirs[1].join("FalloutPrefs.ini")).unwrap();
        assert!(prefs.contains("iMultiSample=4"));
        assert!(!p.pending.exists());
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// A fix row chosen in both profiles made ready: its key in place in each file; chosen back
    /// to the preset's value, each file byte for byte as it was (the generated note again). Before
    /// Play made them ready, the same choices go into settings.ini as the row's own line, which
    /// Play moves into the key, and choosing the preset's value again takes the line out; a line
    /// of another fix stays.
    #[test]
    fn a_fix_row_is_its_own_key_and_hands_back_to_the_preset() {
        let own = scratch("fix-row");
        let p = ready(&own);
        let fixes = "[nv-rs Fixes]\r\n; Uncapped.\r\n;bUncap=1\r\n";
        let text = format!("{SECTIONS}{fixes}");
        for dir in &p.dirs {
            std::fs::write(dir.join("Fallout.ini"), &text).unwrap();
        }
        let uncap = Row::of("bUncap").unwrap();
        p.save(uncap, "Off").unwrap();
        for dir in &p.dirs {
            let ini = std::fs::read_to_string(dir.join("Fallout.ini")).unwrap();
            assert_eq!(ini, text.replace(";bUncap=1", "bUncap=0"));
        }
        assert_eq!(uncap.current(&p.shown().unwrap()), "Off");
        p.save(uncap, "On").unwrap();
        for dir in &p.dirs {
            assert_eq!(
                std::fs::read_to_string(dir.join("Fallout.ini")).unwrap(),
                text
            );
        }
        std::fs::remove_dir_all(&own).unwrap();
        // Not ready (Windows' rule: one shared settings.ini).
        let own = scratch("fix-row-pending");
        let mut p = Profiles::at(&own);
        p.own_pending = None;
        std::fs::create_dir_all(own.join("game")).unwrap();
        std::fs::write(&p.pending, "Preset=fixes\r\nNV_GRAB_PUSH=0\r\n").unwrap();
        assert!(!p.in_files().unwrap());
        p.save(uncap, "Off").unwrap();
        let pending = std::fs::read_to_string(&p.pending).unwrap();
        assert!(pending.contains("\r\nNV_UNCAP=0\r\n"), "{pending}");
        assert!(pending.contains("\r\nNV_GRAB_PUSH=0\r\n"), "{pending}");
        assert_eq!(uncap.current(&p.shown().unwrap()), "Off");
        let moved = game_ini::apply_choices(&text, Some(&usable(&Settings::parse(&pending)))).0;
        assert_eq!(
            profile_settings::ini_value(&moved, "nv-rs Fixes", "bUncap"),
            Some("0")
        );
        p.save(uncap, "On").unwrap();
        let pending = std::fs::read_to_string(&p.pending).unwrap();
        assert!(!pending.contains("NV_UNCAP"), "{pending}");
        assert!(pending.contains("\r\nNV_GRAB_PUSH=0\r\n"), "{pending}");
        assert_eq!(uncap.current(&p.shown().unwrap()), "On");
        // A profile's own file that holds the key: the line hands it back to the preset (a 0 or 1
        // would pin it), so the move makes it the generated note again, and a later preset
        // change moves it as it moves the other profile.
        std::fs::create_dir_all(&p.dirs[1]).unwrap();
        let held = text.replace(";bUncap=1", "bUncap=0");
        std::fs::write(p.dirs[1].join("Fallout.ini"), &held).unwrap();
        p.save(uncap, "Off").unwrap();
        p.save(uncap, "On").unwrap();
        let pending = std::fs::read_to_string(&p.pending).unwrap();
        assert!(pending.contains("\r\nNV_UNCAP=preset\r\n"), "{pending}");
        assert_eq!(uncap.current(&p.shown().unwrap()), "On");
        p.save(Row::GameFixes, "Vanilla (the original game)")
            .unwrap();
        let pending = usable(&Settings::parse(
            &std::fs::read_to_string(&p.pending).unwrap(),
        ));
        for start in [&held, &text] {
            let moved = game_ini::apply_choices(start, Some(&pending)).0;
            let files = Files {
                ini: moved,
                prefs: String::new(),
            };
            assert_eq!(uncap.chosen_here(&files), None, "{}", files.ini);
            assert_eq!(uncap.current(&files), "Off");
        }
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// Before Play made both profiles ready (one has no sections), the choices go into
    /// settings.ini, every option a line, and the files are left as they are; while a
    /// settings.ini is there it is the settings, its lines shown over the files and lines of other
    /// names kept.
    #[test]
    fn before_play_made_the_files_ready_the_choices_go_into_settings_ini() {
        let own = scratch("pending");
        let p = ready(&own);
        std::fs::write(p.dirs[1].join("Fallout.ini"), "[General]\r\n").unwrap();
        assert!(!p.in_files().unwrap());
        p.save(Row::FrameLimit, "144").unwrap();
        let text = std::fs::read_to_string(&p.pending).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with('#'));
        assert_eq!(lines.len(), 13, "{text}");
        assert!(lines.contains(&"FrameLimit=144"), "{text}");
        assert!(lines.contains(&"Renderer=vulkan"), "{text}");
        assert!(lines.contains(&"NV_THREADS=real"), "{text}");
        assert_eq!(
            std::fs::read_to_string(p.dirs[0].join("Fallout.ini")).unwrap(),
            SECTIONS
        );
        // Now settings.ini is there: it is the settings, even with both files ready.
        std::fs::write(p.dirs[1].join("Fallout.ini"), SECTIONS).unwrap();
        let mut with_other = std::fs::read_to_string(&p.pending).unwrap();
        with_other.push_str("SomethingElse=7\r\nNV_PRESET=vanilla\r\n");
        std::fs::write(&p.pending, &with_other).unwrap();
        assert!(!p.in_files().unwrap());
        assert_eq!(Row::FrameLimit.current(&p.shown().unwrap()), "144");
        p.save(Row::GameFixes, "Vanilla (the original game)")
            .unwrap();
        let text = std::fs::read_to_string(&p.pending).unwrap();
        assert!(text.contains("Preset=vanilla\r\n") && text.contains("FrameLimit=144\r\n"));
        assert!(text.contains("SomethingElse=7"), "{text}");
        assert!(!text.contains("NV_PRESET"), "{text}");
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// A read-only file stays read-only after its change; a file that cannot be read stops the
    /// save before anything is written.
    #[test]
    fn a_read_only_file_keeps_its_mark_and_an_unreadable_one_stops_the_save() {
        let own = scratch("marks");
        let p = ready(&own);
        let ini = p.dirs[0].join("Fallout.ini");
        let mut perms = std::fs::metadata(&ini).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&ini, perms).unwrap();
        p.save(Row::VSync, "Off").unwrap();
        assert!(std::fs::metadata(&ini).unwrap().permissions().readonly());
        assert!(std::fs::read_to_string(&ini)
            .unwrap()
            .contains("iPresentInterval=0"));
        let mut perms = std::fs::metadata(&ini).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        std::fs::set_permissions(&ini, perms).unwrap();
        // The game profile's Fallout.ini is a folder: unreadable; the mods profile is not touched.
        std::fs::remove_file(&ini).unwrap();
        std::fs::create_dir_all(&ini).unwrap();
        let before = std::fs::read(p.dirs[1].join("Fallout.ini")).unwrap();
        let e = p.save(Row::FrameLimit, "60").unwrap_err();
        assert!(e.to_string().contains("Fallout.ini"), "{e}");
        assert_eq!(
            std::fs::read(p.dirs[1].join("Fallout.ini")).unwrap(),
            before
        );
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// Off Windows only the profile the game started first is ready: a choice goes into its files
    /// in place, and into the other profile's own settings.ini (every option a line; a table row
    /// as its own switch's line), which that profile's first start moves in; nothing shared is
    /// written. Once both are ready, both take it in place.
    #[test]
    fn with_only_the_game_profile_ready_the_other_keeps_its_own_file() {
        let own = scratch("one-ready");
        let mut p = Profiles::at(&own);
        p.own_pending = Some([own.join("game"), own.join("mods")].map(|d| d.join("settings.ini")));
        std::fs::create_dir_all(&p.dirs[0]).unwrap();
        std::fs::write(p.dirs[0].join("Fallout.ini"), SECTIONS).unwrap();
        std::fs::write(p.dirs[0].join("FalloutPrefs.ini"), PREFS).unwrap();
        assert!(!p.in_files().unwrap());
        p.save(Row::FrameLimit, "90").unwrap();
        p.save(Row::of("bAmbientOcclusion").unwrap(), "On").unwrap();
        let game = std::fs::read_to_string(p.dirs[0].join("Fallout.ini")).unwrap();
        assert!(game.contains("iFrameLimit=90"), "{game}");
        assert!(game.contains("bAmbientOcclusion=1"), "{game}");
        let mods = std::fs::read_to_string(own.join("mods").join("settings.ini")).unwrap();
        assert!(mods.contains("\r\nFrameLimit=90\r\n"), "{mods}");
        assert!(mods.contains("\r\nNV_AO=1\r\n"), "{mods}");
        assert!(!p.pending.exists(), "no shared settings.ini");
        assert!(!own.join("game").join("settings.ini").exists());
        // The mods profile's file, read as the game reads it, has both choices.
        let pending = Settings::parse(&mods);
        let (ini, _) = game_ini::apply_choices(SECTIONS, Some(&usable(&pending)));
        let get = |s, k| profile_settings::ini_value(&ini, s, k).map(str::to_string);
        assert_eq!(
            get("nv-rs Display", "bAmbientOcclusion").as_deref(),
            Some("1")
        );
        assert_eq!(get("nv-rs Display", "iFrameLimit").as_deref(), Some("90"));
        // The mods profile made ready (its first start moved the file in): in place from then on.
        std::fs::create_dir_all(&p.dirs[1]).unwrap();
        std::fs::write(p.dirs[1].join("Fallout.ini"), SECTIONS).unwrap();
        std::fs::remove_file(own.join("mods").join("settings.ini")).unwrap();
        assert!(p.in_files().unwrap());
        p.save(Row::FrameLimit, "120").unwrap();
        for dir in &p.dirs {
            let ini = std::fs::read_to_string(dir.join("Fallout.ini")).unwrap();
            assert!(ini.contains("iFrameLimit=120"), "{ini}");
        }
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// The off-Windows rule (each profile's own settings.ini).
    fn per_profile(own: &Path) -> Profiles {
        let mut p = Profiles::at(own);
        p.own_pending = Some([own.join("game"), own.join("mods")].map(|d| d.join("settings.ini")));
        p
    }

    /// Off Windows with only the game profile ready, a fix turned off and back on follows the
    /// preset in both: the game profile's file has the generated note again, and the mods
    /// profile's own settings.ini no line for it (the game profile's file held the key before the
    /// save, which is not the mods profile's); a later preset change then moves both alike. Where
    /// the mods profile's own file holds the key, its line hands it back to the preset.
    #[test]
    fn off_windows_a_fix_handed_back_follows_the_preset_in_both_profiles() {
        let own = scratch("fix-per-profile");
        let p = per_profile(&own);
        let text = format!("{SECTIONS}[nv-rs Fixes]\r\n; Uncapped.\r\n;bUncap=1\r\n");
        std::fs::create_dir_all(&p.dirs[0]).unwrap();
        std::fs::write(p.dirs[0].join("Fallout.ini"), &text).unwrap();
        let uncap = Row::of("bUncap").unwrap();
        let mods = own.join("mods").join("settings.ini");
        let mods_profile = |start: &str| {
            let pending = usable(&Settings::parse(&std::fs::read_to_string(&mods).unwrap()));
            Files {
                ini: game_ini::apply_choices(start, Some(&pending)).0,
                prefs: String::new(),
            }
        };
        p.save(uncap, "Off").unwrap();
        assert_eq!(uncap.current(&mods_profile(&text)), "Off");
        p.save(uncap, "On").unwrap();
        let game = std::fs::read_to_string(p.dirs[0].join("Fallout.ini")).unwrap();
        assert_eq!(game, text);
        let lines = std::fs::read_to_string(&mods).unwrap();
        assert!(!lines.contains("NV_UNCAP"), "{lines}");
        // A later preset change: both profiles follow it, the mods profile once its first start
        // moves its file in.
        for (preset, now) in [
            ("Vanilla (the original game)", "Off"),
            ("Fixes (recommended)", "On"),
        ] {
            p.save(Row::GameFixes, preset).unwrap();
            let game = Files {
                ini: std::fs::read_to_string(p.dirs[0].join("Fallout.ini")).unwrap(),
                prefs: String::new(),
            };
            for files in [game, mods_profile(&text)] {
                assert_eq!(uncap.chosen_here(&files), None, "{}", files.ini);
                assert_eq!(uncap.current(&files), now, "{preset}");
            }
        }
        // The mods profile's own Fallout.ini holds the key: its line hands it back.
        std::fs::create_dir_all(&p.dirs[1]).unwrap();
        let held = text.replace(";bUncap=1", "bUncap=0");
        std::fs::write(p.dirs[1].join("Fallout.ini"), &held).unwrap();
        p.save(uncap, "Off").unwrap();
        p.save(uncap, "On").unwrap();
        let lines = std::fs::read_to_string(&mods).unwrap();
        assert!(lines.contains("\r\nNV_UNCAP=preset\r\n"), "{lines}");
        assert_eq!(uncap.chosen_here(&mods_profile(&held)), None);
        assert_eq!(uncap.current(&mods_profile(&held)), "On");
        assert!(!own.join("game").join("settings.ini").exists());
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// Off Windows a d3d9 the game profile holds is never passed on: the other profile's own
    /// settings.ini says the game's own renderer, whatever row was saved.
    #[test]
    fn off_windows_d3d9_is_never_copied_into_the_other_profile() {
        let own = scratch("no-d3d9");
        let p = per_profile(&own);
        std::fs::create_dir_all(&p.dirs[0]).unwrap();
        let held = SECTIONS.replace("sRenderer=vulkan", "sRenderer=d3d9");
        std::fs::write(p.dirs[0].join("Fallout.ini"), &held).unwrap();
        p.save(Row::FrameLimit, "60").unwrap();
        let mods = std::fs::read_to_string(own.join("mods").join("settings.ini")).unwrap();
        assert!(mods.contains("\r\nRenderer=vulkan\r\n"), "{mods}");
        assert!(!mods.contains("d3d9"), "{mods}");
        assert!(mods.contains("\r\nFrameLimit=60\r\n"), "{mods}");
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// Off Windows a shared settings.ini is split at the first save into each profile's own file
    /// (the game's first start would move it into one profile only), the choice in both, and the
    /// shared one renamed aside; over a profile's own file that is there already, the shared
    /// file's lines win line by line and the own file's other lines stay.
    #[test]
    fn off_windows_a_shared_settings_ini_reaches_both_profiles() {
        let own = scratch("split");
        let p = per_profile(&own);
        std::fs::write(&p.pending, "Preset=vanilla\r\nOther=1\r\n").unwrap();
        p.save(Row::FrameLimit, "60").unwrap();
        for name in ["game", "mods"] {
            let text = std::fs::read_to_string(own.join(name).join("settings.ini")).unwrap();
            assert!(text.contains("\r\nFrameLimit=60\r\n"), "{name}: {text}");
            assert!(text.contains("\r\nPreset=vanilla\r\n"), "{name}: {text}");
            assert!(text.contains("\r\nOther=1\r\n"), "{name}: {text}");
        }
        assert!(!p.pending.exists());
        assert!(own.join("settings.ini.migrated").is_file());
        // Again, with both own files there: the shared file's lines win line by line (as play.sh
        // and the screen let it win), the own files' lines of other names stay.
        std::fs::write(&p.pending, "preset=fixes\r\n").unwrap();
        p.save(Row::MenuFrameLimit, "90").unwrap();
        for name in ["game", "mods"] {
            let text = std::fs::read_to_string(own.join(name).join("settings.ini")).unwrap();
            assert!(text.contains("\r\nPreset=fixes\r\n"), "{name}: {text}");
            assert!(!text.contains("vanilla"), "{name}: {text}");
            assert!(text.contains("\r\nMenuFrameLimit=90\r\n"), "{name}: {text}");
            assert!(text.contains("\r\nFrameLimit=60\r\n"), "{name}: {text}");
            assert!(text.contains("\r\nOther=1\r\n"), "{name}: {text}");
        }
        assert!(!p.pending.exists());
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// settings.ini's names are matched without case, as the game reads them: a hand-written
    /// `framelimit=` or `renderer=` line is replaced by the choice, not kept after it to win.
    #[test]
    fn settings_ini_names_are_matched_without_case() {
        let own = scratch("case");
        let mut p = Profiles::at(&own);
        p.own_pending = None;
        let hand = "framelimit=45\r\nrenderer=vulkan\r\nnv_preset=vanilla\r\nOther=1\r\n";
        std::fs::write(&p.pending, hand).unwrap();
        p.save(Row::FrameLimit, "60").unwrap();
        let text = std::fs::read_to_string(&p.pending).unwrap();
        assert!(!text.contains("framelimit=45"), "{text}");
        assert!(!text.contains("renderer=vulkan"), "{text}");
        assert!(!text.contains("nv_preset"), "{text}");
        assert!(text.contains("\r\nFrameLimit=60\r\n"), "{text}");
        assert!(text.contains("\r\nOther=1\r\n"), "{text}");
        assert_eq!(Row::FrameLimit.current(&p.shown().unwrap()), "60");
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// One line the game cannot use (a limit that is not a number) leaves the other lines as they
    /// are: a Vanilla choice stays Vanilla on the screen and through the next save, and the bad
    /// line is replaced by what the screen shows.
    #[test]
    fn one_bad_line_does_not_drop_the_others() {
        let own = scratch("bad-line");
        let mut p = Profiles::at(&own);
        p.own_pending = None;
        let text = "Preset=vanilla\r\nFrameLimit=abc\r\nMenuFrameLimit=60\r\n";
        std::fs::write(&p.pending, text).unwrap();
        let shown = p.shown().unwrap();
        assert_eq!(
            Row::GameFixes.current(&shown),
            "Vanilla (the original game)"
        );
        assert_eq!(Row::MenuFrameLimit.current(&shown), "60");
        p.save(Row::VSync, "Off").unwrap();
        let text = std::fs::read_to_string(&p.pending).unwrap();
        assert!(text.contains("\r\nPreset=vanilla\r\n"), "{text}");
        assert!(text.contains("\r\nMenuFrameLimit=60\r\n"), "{text}");
        assert!(!text.contains("abc"), "{text}");
        assert!(Settings::parse(&text).game_switches().is_ok(), "{text}");
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// A row set to "Game's setting" in settings.ini says so (the game's own file decides), not
    /// the value it had.
    #[test]
    fn game_setting_is_written_into_settings_ini() {
        let own = scratch("own-line");
        let mut p = Profiles::at(&own);
        p.own_pending = None;
        std::fs::write(&p.pending, "AntiAliasing=4x\r\nResolution=1920x1080\r\n").unwrap();
        p.save(Row::AntiAliasing, OWN).unwrap();
        let text = std::fs::read_to_string(&p.pending).unwrap();
        assert!(
            text.contains(&format!("\r\nAntiAliasing={OWN}\r\n")),
            "{text}"
        );
        assert!(text.contains("\r\nResolution=1920x1080\r\n"), "{text}");
        assert_eq!(Row::AntiAliasing.current(&p.shown().unwrap()), OWN);
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// Every file of a save is written whole beside itself before any takes its place: when one
    /// cannot be written, no profile has changed and no `.part` is left.
    #[test]
    fn a_save_that_cannot_write_one_file_changes_none() {
        let own = scratch("all-or-none");
        let p = ready(&own);
        // The game profile's Fallout.ini.part is a folder: its part cannot be written.
        std::fs::create_dir_all(p.dirs[0].join("Fallout.ini.part").join("x")).unwrap();
        let read_all = || -> Vec<Vec<u8>> {
            p.dirs
                .iter()
                .map(|d| std::fs::read(d.join("Fallout.ini")).unwrap())
                .collect()
        };
        let before = read_all();
        assert!(p.save(Row::FrameLimit, "60").is_err());
        assert_eq!(read_all(), before);
        assert!(!p.dirs[1].join("Fallout.ini.part").exists());
        std::fs::remove_dir_all(&own).unwrap();
    }

    /// settings.ini's lines say what the files show, by Settings.ps1's names and values.
    #[test]
    fn settings_ini_lines_say_what_the_files_show() {
        let files = Files {
            ini: SECTIONS.into(),
            prefs: PREFS.into(),
        };
        let lines = settings_lines(&files);
        let get = |n: &str| lines.iter().find(|(k, _)| *k == n).unwrap().1.clone();
        assert_eq!(get("Renderer"), "vulkan");
        assert_eq!(get("Borderless"), "1");
        assert_eq!(get("Resolution"), "1920x1080");
        assert_eq!(get("AntiAliasing"), "4x");
        assert_eq!(get("VSync"), "On");
        assert_eq!(get("Preset"), "fixes");
        assert_eq!(get("FrameLimit"), "0");
        assert_eq!(get("MenuFrameLimit"), "120");
        assert_eq!(get("NV_THREADS"), "real");
        assert_eq!(get("NV_RENDER_THREAD"), "1");
        assert_eq!(lines.len(), KNOWN.len());
        // Every line names its row's choice back (the tests' way of choosing).
        for (name, value) in &lines {
            let (row, choice) = choice_for(name, value).unwrap();
            assert_eq!(row.current(&files), choice, "{name}");
        }
        assert!(choice_for("SomethingElse", "1").is_none());
    }
}
