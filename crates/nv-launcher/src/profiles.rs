//! The Settings screen's files: both ways of playing's profiles (`game`, then `mods`), each with the
//! game's `Fallout.ini` (nv-rs's `[nv-rs ...]` sections) and `FalloutPrefs.ini`, in nv-rs's own
//! folder (Windows: the package's `userdata`), and the older `settings.ini` stage, kept as
//! Settings.cmd kept them (`distribution/Settings.ps1`, which this replaces):
//!
//! - the choices go into the files only when every profile's `Fallout.ini` has nv-rs's sections
//!   (Play made them ready) and there is no `settings.ini`; otherwise into `settings.ini`, every
//!   option a line, which Play.exe (and, off Windows, the game's first start) moves into the files;
//! - what the screen shows is the game profile's files, `settings.ini`'s lines over them while it
//!   is there (as Play moves it in);
//! - a change writes only its row's keys, in place, in both profiles, the mods profile first and
//!   the game profile last (it is what the screen shows, so after a save that failed half way the
//!   screen still shows what was not saved, and choosing again finishes it);
//! - every file is written whole beside itself first (`NAME.part`) and then takes its place in one
//!   step, its read-only mark kept, so a full disk leaves every file whole; a file that cannot be
//!   read stops the save before anything is written.

use crate::settings::{Files, Row};
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
    /// `settings.ini` in nv-rs's own folder.
    pub pending: PathBuf,
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

/// `bytes` in place of `path`: written beside it first, then put in its place in one step; a
/// read-only mark is lifted for that step and put back.
// `set_readonly(false)` clears the read-only attribute (Windows) or gives the owner write (Unix).
#[allow(clippy::permissions_set_readonly_false)]
fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let _ = std::fs::remove_file(&part);
    std::fs::write(&part, bytes)?;
    let read_only = std::fs::metadata(path).is_ok_and(|m| m.permissions().readonly());
    // Windows refuses to replace a read-only file; elsewhere the folder decides, and the new file
    // gets the mark below.
    if read_only && cfg!(windows) {
        let mut p = std::fs::metadata(path)?.permissions();
        p.set_readonly(false);
        std::fs::set_permissions(path, p)?;
    }
    let renamed = std::fs::rename(&part, path);
    if read_only {
        if let Ok(m) = std::fs::metadata(path) {
            let mut p = m.permissions();
            p.set_readonly(true);
            let _ = std::fs::set_permissions(path, p);
        }
    }
    if renamed.is_err() {
        let _ = std::fs::remove_file(&part);
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
        }
    }

    fn files(dir: &Path) -> std::io::Result<Files> {
        Ok(Files {
            ini: read(&dir.join("Fallout.ini"))?,
            prefs: read(&dir.join("FalloutPrefs.ini"))?,
        })
    }

    fn pending_settings(&self) -> std::io::Result<Option<(String, Settings)>> {
        match std::fs::read(&self.pending) {
            Ok(b) => {
                let text = profile_settings::decode(&b);
                let parsed = Settings::parse(&text);
                Ok(Some((text, parsed)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", self.pending.display()),
            )),
        }
    }

    /// Whether the choices go into the files: every profile made ready, no settings.ini.
    pub fn in_files(&self) -> std::io::Result<bool> {
        if self.pending_settings()?.is_some() {
            return Ok(false);
        }
        for dir in &self.dirs {
            if !game_ini::has_sections(&Self::files(dir)?.ini) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// What the screen shows: the game profile's files, settings.ini's lines over them while it
    /// is there.
    pub fn shown(&self) -> std::io::Result<Files> {
        let game = Self::files(&self.dirs[0])?;
        Ok(match self.pending_settings()? {
            None => game,
            Some((_, pending)) => Files {
                ini: game_ini::apply_choices(&game.ini, Some(&pending)).0,
                prefs: profile_settings::apply_to_prefs(&game.prefs, &pending.prefs_lines()),
            },
        })
    }

    /// `row` set to `choice`: in both profiles' files, or (before Play made them ready, or while a
    /// settings.ini is there) in settings.ini. Nothing is written when a file cannot be read.
    pub fn save(&self, row: Row, choice: &str) -> std::io::Result<()> {
        if self.in_files()? {
            // Every file read first: one that cannot be read stops the save before any write.
            let mut each = Vec::new();
            for dir in self.dirs.iter().rev() {
                each.push((dir.clone(), Self::files(dir)?));
            }
            for (dir, files) in each {
                let new = row.apply(&files, choice);
                if new.prefs != files.prefs && dir.join("FalloutPrefs.ini").exists() {
                    write_whole(&dir.join("FalloutPrefs.ini"), &bytes_of(&new.prefs))?;
                }
                if new.ini != files.ini {
                    write_whole(&dir.join("Fallout.ini"), &bytes_of(&new.ini))?;
                }
            }
            return Ok(());
        }
        // settings.ini: every option a line, the choice made, the lines of other names kept.
        let shown = self.shown()?;
        let kept: Vec<String> = match self.pending_settings()? {
            Some((text, _)) => text
                .lines()
                .filter(|l| {
                    l.split_once('=').is_some_and(|(name, _)| {
                        let name = name.trim();
                        !name.starts_with('#')
                            && !name.starts_with(';')
                            && !KNOWN.contains(&name)
                            && !DROPPED.contains(&name)
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
        lines.extend(
            settings_lines(&chosen)
                .into_iter()
                .map(|(name, value)| format!("{name}={value}")),
        );
        lines.extend(kept);
        if let Some(dir) = self.pending.parent() {
            std::fs::create_dir_all(dir)?;
        }
        write_whole(&self.pending, (lines.join("\r\n") + "\r\n").as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::OWN;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nv-launcher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const SECTIONS: &str = "[Display]\r\niPresentInterval=1\r\n[nv-rs Display]\r\n;the renderer\r\nsRenderer=vulkan\r\niFrameLimit=0\r\n[nv-rs General]\r\nsPreset=fixes\r\n";
    const PREFS: &str = "[Display]\r\niSize W=1920\r\niSize H=1080\r\niMultiSample=4\r\n";

    fn ready(own: &Path) -> Profiles {
        let p = Profiles::at(own);
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
        p.save(Row::Renderer, "Direct3D 9 (the original's)")
            .unwrap();
        p.save(Row::Resolution, "2560x1440").unwrap();
        for dir in &p.dirs {
            let ini = std::fs::read_to_string(dir.join("Fallout.ini")).unwrap();
            assert_eq!(ini, SECTIONS.replace("sRenderer=vulkan", "sRenderer=d3d9"));
            let prefs = std::fs::read_to_string(dir.join("FalloutPrefs.ini")).unwrap();
            assert_eq!(prefs, PREFS.replace("1920", "2560").replace("1080", "1440"));
            assert!(!dir.join("Fallout.ini.part").exists());
        }
        let shown = p.shown().unwrap();
        assert_eq!(Row::Renderer.current(&shown), "Direct3D 9 (the original's)");
        // "Game's setting" writes nothing.
        p.save(Row::AntiAliasing, OWN).unwrap();
        let prefs = std::fs::read_to_string(p.dirs[1].join("FalloutPrefs.ini")).unwrap();
        assert!(prefs.contains("iMultiSample=4"));
        assert!(!p.pending.exists());
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
        assert!(p
            .save(Row::Renderer, "Direct3D 9 (the original's)")
            .is_err());
        assert_eq!(
            std::fs::read(p.dirs[1].join("Fallout.ini")).unwrap(),
            before
        );
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
    }
}
