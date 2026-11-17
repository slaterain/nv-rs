//! The first start's profile files on Linux and macOS, where there is no Play.exe to prepare them
//! (review of #687): the game's `Fallout.ini` with nv-rs's own sections and the player's pending
//! choices, and `FalloutPrefs.ini` from the game's High preset at the desktop's size, borderless,
//! naming the card. The same steps and the same code as Play.exe's `prepare_profile`
//! (`scripts/launcher` main.rs; the sections and lines from the shared `game_profile` crate), with
//! one difference: Play.exe first copies the player's own Windows files from their `My Games`; off
//! Windows the starting point is the game's `Fallout_default.ini` and `high.ini` (only read from
//! the game's folder). Both files live in nv-rs's own folder (the game's Documents there,
//! `engine_platform::machine_table::game_folders`) and are written only through the save guard
//! (`crate::save_out`). A file already there (one the player edited) keeps every byte outside the
//! lines this adds: `Fallout.ini` gets only the sections or keys it lacks, `FalloutPrefs.ini` is
//! made only when missing and then only the play lines are set.
// The pure part runs in the tests on every system; only Linux and macOS use it in a run.
#![cfg_attr(windows, allow(dead_code))]

use game_profile::game_ini;
use game_profile::settings::{self, Settings};

/// A file's bytes as text, one character per byte (as Play.exe reads them).
fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// [`text_of`]'s text as the file's bytes again.
fn bytes_of(text: &str) -> Vec<u8> {
    text.chars().map(|c| c as u32 as u8).collect()
}

/// What a first start reads: the profile's two files as they are (`None`: missing), the game's
/// `Fallout_default.ini` and `high.ini`, the player's pending choices (`settings.ini`), the
/// desktop's size and the card's name (the machine table's).
pub struct Inputs<'a> {
    pub ini: Option<Vec<u8>>,
    pub prefs: Option<Vec<u8>>,
    pub default_ini: Option<Vec<u8>>,
    pub high_ini: Option<Vec<u8>>,
    pub pending: Option<&'a Settings>,
    pub desktop: Option<(u32, u32)>,
    pub device: Option<&'a str>,
}

/// What to write (`None`: leave the file as it is) and the log's lines.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub ini: Option<Vec<u8>>,
    pub prefs: Option<Vec<u8>>,
    pub log: Vec<String>,
}

/// Play.exe's `prepare_profile` steps on [`Inputs`].
pub fn prepare(inputs: &Inputs) -> Outcome {
    let mut out = Outcome::default();
    // An empty file is no file: the game's defaults are taken, as at a first start.
    let start = match &inputs.ini {
        Some(bytes) if !bytes.is_empty() => bytes.clone(),
        _ => inputs
            .default_ini
            .clone()
            .filter(|b| !b.is_empty())
            .unwrap_or_default(),
    };
    let mut seeded = false;
    let mut text = text_of(&start);
    if start.is_empty() {
        out.log.push(
            "Fallout.ini: none to start from (no Fallout_default.ini in the game's folder): none made"
                .into(),
        );
    } else if let Some(s) = game_ini::with_sections(&text) {
        text = game_ini::apply_choices(&s.text, inputs.pending).0;
        seeded = true;
        if s.fresh {
            out.log
                .push("Fallout.ini: nv-rs's own sections added".into());
        } else if !s.added.is_empty() {
            out.log.push(format!(
                "Fallout.ini: nv-rs's own sections completed: {} added",
                s.added.join(", ")
            ));
        }
    } else {
        out.log
            .push("Fallout.ini is UTF-16 text: left as it is, without nv-rs's own sections".into());
    }
    let ini = bytes_of(&text);
    if !ini.is_empty() && inputs.ini.as_ref() != Some(&ini) {
        out.ini = Some(ini.clone());
    }

    let mut first_time_size = None;
    let old_prefs = match &inputs.prefs {
        Some(bytes) => bytes.clone(),
        None => {
            // The game's window starts at the desktop's size.
            first_time_size = inputs.desktop;
            out.log
                .push("FalloutPrefs.ini: none yet: starting from the game's High preset".into());
            inputs.high_ini.clone().unwrap_or_default()
        }
    };
    let mut lines = inputs
        .pending
        .map(Settings::prefs_lines)
        .unwrap_or_default();
    let borderless = if seeded {
        game_ini::borderless(&settings::decode(&ini))
    } else {
        inputs
            .pending
            .and_then(Settings::borderless)
            .unwrap_or_else(|| game_ini::borderless(""))
    };
    if borderless {
        lines.push(("bFull Screen".into(), "0".into()));
    }
    if let Some(device) = inputs.device {
        lines.push(("sD3DDevice".into(), format!("\"{device}\"")));
    }
    if let Some((w, h)) = first_time_size {
        if !lines.iter().any(|(n, _)| n == "iSize W") {
            lines.push(("iSize W".into(), w.to_string()));
            lines.push(("iSize H".into(), h.to_string()));
        }
    }
    let new = if lines.is_empty() {
        old_prefs.clone()
    } else {
        bytes_of(&settings::apply_to_prefs(&text_of(&old_prefs), &lines))
    };
    if inputs.prefs.as_ref() != Some(&new) {
        out.prefs = Some(new);
    }
    out
}

/// The desktop's size (`GetSystemMetrics` 0 and 1, in the scaled pixels the game sees) and the
/// card's name (`GetAdapterIdentifier`'s description) in a machine table's text.
pub fn table_facts(table: &str) -> (Option<(u32, u32)>, Option<String>) {
    let mut w = None;
    let mut h = None;
    let mut device = None;
    for line in table.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["GetSystemMetrics", "0", v] => w = u32::from_str_radix(v, 16).ok(),
            ["GetSystemMetrics", "1", v] => h = u32::from_str_radix(v, 16).ok(),
            ["GetAdapterIdentifier", "0 0", "0", bytes] => {
                let b: Vec<u8> = (0..bytes.len() / 2)
                    .filter_map(|i| u8::from_str_radix(bytes.get(2 * i..2 * i + 2)?, 16).ok())
                    .collect();
                let name = b.get(512..1024).unwrap_or_default();
                let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                device = Some(String::from_utf8_lossy(&name[..end]).into_owned());
            }
            _ => {}
        }
    }
    (w.zip(h), device.filter(|d| !d.is_empty()))
}

/// Off Windows, at a play run's start (after the renderer choice and the machine table): the
/// profile files in `documents` (`My Games\FalloutNV`) prepared from the game in `game_folder`
/// and the table's facts, written through the save guard. Each step's line is said in the log;
/// a refused or failed write is said and the start goes on (the game then makes its own files).
#[cfg(not(windows))]
pub fn first_start(documents: &std::path::Path, game_folder: &std::path::Path, table: &str) {
    let profile = documents.join("My Games").join("FalloutNV");
    let read = |p: &std::path::Path| std::fs::read(p).ok();
    let pending = crate::fixes::settings_file()
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| Settings::parse(&settings::decode(&b)));
    let (desktop, device) = table_facts(table);
    let outcome = prepare(&Inputs {
        ini: read(&profile.join("Fallout.ini")),
        prefs: read(&profile.join("FalloutPrefs.ini")),
        default_ini: read(&game_folder.join("Fallout_default.ini")),
        high_ini: read(&game_folder.join("high.ini")),
        pending: pending.as_ref(),
        desktop,
        device: device.as_deref(),
    });
    for line in &outcome.log {
        eprintln!("first start: {line}");
    }
    for (name, bytes) in [
        ("Fallout.ini", outcome.ini),
        ("FalloutPrefs.ini", outcome.prefs),
    ] {
        let Some(bytes) = bytes else { continue };
        let target = profile.join(name);
        let job = crate::save_out::Job::Write {
            key: Vec::new(),
            dir: Some(profile.clone()),
            temporary: profile.join(format!("{name}.part")),
            target,
            bytes,
            copied: None,
        };
        if let Err(why) = crate::save_out::run(&job) {
            eprintln!("first start: {name}: {why}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_INI: &[u8] = b"[General]\r\nsLanguage=ENGLISH\r\n[Display]\r\nbDoTallGrass=1\r\n";
    const HIGH_INI: &[u8] = b"[Display]\r\niSize W=1280\r\niSize H=720\r\nbFull Screen=1\r\n";

    fn first(ini: Option<&[u8]>, prefs: Option<&[u8]>) -> Outcome {
        prepare(&Inputs {
            ini: ini.map(<[u8]>::to_vec),
            prefs: prefs.map(<[u8]>::to_vec),
            default_ini: Some(DEFAULT_INI.to_vec()),
            high_ini: Some(HIGH_INI.to_vec()),
            pending: None,
            desktop: Some((2560, 1440)),
            device: Some("AMD Radeon RX 7800 XT (RADV NAVI32)"),
        })
    }

    /// A first start makes both files: the game's defaults with nv-rs's sections, and the High
    /// preset at the desktop's size, borderless, naming the card.
    #[test]
    fn a_first_start_makes_both_files() {
        let out = first(None, None);
        let ini = text_of(out.ini.as_deref().unwrap());
        assert!(ini.contains("[General]\r\nsLanguage=ENGLISH\r\n"), "{ini}");
        assert!(game_ini::has_sections(&ini), "{ini}");
        let prefs = text_of(out.prefs.as_deref().unwrap());
        for line in [
            "iSize W=2560",
            "iSize H=1440",
            "bFull Screen=0",
            "sD3DDevice=\"AMD Radeon RX 7800 XT (RADV NAVI32)\"",
        ] {
            assert!(prefs.contains(line), "{line}: {prefs}");
        }
        assert!(!prefs.contains("iSize W=1280"), "{prefs}");
    }

    /// A file the player edited is not replaced: Fallout.ini already with the sections is left
    /// as it is; FalloutPrefs.ini keeps its own size, only the play lines are set.
    #[test]
    fn an_existing_players_file_is_not_overwritten() {
        let made = first(None, None);
        let ini = made.ini.unwrap();
        let mut edited = ini.clone();
        edited.extend_from_slice(b"[Player]\r\nmine=1\r\n");
        let prefs = b"[Display]\r\niSize W=1920\r\niSize H=1080\r\nbFull Screen=0\r\nsD3DDevice=\"AMD Radeon RX 7800 XT (RADV NAVI32)\"\r\nfGamma=1.2\r\n";
        let again = first(Some(&edited), Some(prefs));
        assert_eq!(again.ini, None, "the edited Fallout.ini stays as it is");
        assert_eq!(
            again.prefs, None,
            "the edited FalloutPrefs.ini stays as it is"
        );
        // A FalloutPrefs.ini with only the size the player chose: the size stays, the play
        // lines come in.
        let partial = b"[Display]\r\niSize W=1920\r\niSize H=1080\r\n";
        let out = first(Some(&edited), Some(partial));
        let text = text_of(out.prefs.as_deref().unwrap());
        assert!(
            text.contains("iSize W=1920") && !text.contains("2560"),
            "{text}"
        );
        assert!(text.contains("bFull Screen=0"), "{text}");
    }

    /// No game defaults: no Fallout.ini is made (as Play.exe), the High preset still is.
    #[test]
    fn without_the_games_defaults_no_fallout_ini_is_made() {
        let out = prepare(&Inputs {
            ini: None,
            prefs: None,
            default_ini: None,
            high_ini: None,
            pending: None,
            desktop: None,
            device: None,
        });
        assert_eq!(out.ini, None);
        assert!(out.log[0].contains("none made"));
    }

    #[test]
    fn the_tables_facts_are_read() {
        let mut id = vec![0u8; 1100];
        id[512..516].copy_from_slice(b"Card");
        let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
        let table = format!(
            "GetSystemMetrics\t0\t800\nGetSystemMetrics\t1\t480\nGetAdapterIdentifier\t0 0\t0\t{hex}\n"
        );
        assert_eq!(
            table_facts(&table),
            (Some((2048, 1152)), Some("Card".into()))
        );
        assert_eq!(table_facts(""), (None, None));
    }
}
