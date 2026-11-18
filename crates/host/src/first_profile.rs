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
/// desktop's size in the system's scaled pixels and the monitor's in its real pixels, and the
/// card's name (the machine table's: [`table_facts`]).
pub struct Inputs<'a> {
    pub ini: Option<Vec<u8>>,
    pub prefs: Option<Vec<u8>>,
    pub default_ini: Option<Vec<u8>>,
    pub high_ini: Option<Vec<u8>>,
    pub pending: Option<&'a Settings>,
    pub desktop: Option<(u32, u32)>,
    /// The monitor's size in real pixels (`GetAdapterDisplayMode`): the borderless window's.
    pub screen: Option<(u32, u32)>,
    pub device: Option<&'a str>,
}

/// What to write (`None`: leave the file as it is) and the log's lines.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub ini: Option<Vec<u8>>,
    pub prefs: Option<Vec<u8>>,
    pub log: Vec<String>,
    /// `Fallout.ini` has nv-rs's sections, and with them the pending choices (Play.exe's
    /// `Prepared::seeded`: a `settings.ini` may then be moved aside).
    pub seeded: bool,
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

    let mut first_time = false;
    let old_prefs = match &inputs.prefs {
        Some(bytes) => bytes.clone(),
        None => {
            first_time = true;
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
        // The game compares this name with the card's at every start (`004da570`, without case)
        // and, when they differ, shuts its renderer and asks for its launcher; it is written
        // again at every start, so a renamed card (a driver update can rename it) is said here.
        // Only a saved file's name is compared: a first start's text is the High preset's.
        let saved = if first_time {
            None
        } else {
            settings::ini_value(&text_of(&old_prefs), "Display", "sD3DDevice").map(str::to_string)
        };
        if let Some(before) = saved {
            let before = before.trim().trim_matches('"');
            if !before.eq_ignore_ascii_case(device) {
                out.log.push(format!(
                    "FalloutPrefs.ini: the graphics card's name changed from \"{before}\" to \
                     \"{device}\": the new name written"
                ));
            }
        }
        lines.push(("sD3DDevice".into(), format!("\"{device}\"")));
    }
    // The game's window starts at the desktop's size: borderless, the monitor's real pixels (the
    // window covers them: a back buffer in the system's scaled pixels would be stretched and soft
    // on a scaled desktop, the first Linux start's 2194x1234 on 2560x1440, 2026-11-17); framed, the
    // scaled size the window is made in. A FalloutPrefs.ini an earlier start made with the scaled
    // size of a borderless window (exactly that size, the scale not 1) is corrected once: the
    // file then carries [`SIZE_CHECKED`] (written by every start of this build, a fresh file's
    // at once), so a scaled size the player picks later is kept.
    let size = if borderless {
        inputs.screen.or(inputs.desktop)
    } else {
        inputs.desktop
    };
    let checked = size_checked(&text_of(&old_prefs));
    let earlier_scaled = !first_time
        && !checked
        && borderless
        && inputs.screen.is_some()
        && inputs.desktop.is_some()
        && inputs.desktop != inputs.screen
        && prefs_size(&text_of(&old_prefs)) == inputs.desktop;
    if let (Some((w, h)), true) = (size, first_time || earlier_scaled) {
        if !lines.iter().any(|(n, _)| n == "iSize W") {
            if earlier_scaled {
                let (sw, sh) = inputs.desktop.unwrap_or_default();
                out.log.push(format!(
                    "FalloutPrefs.ini: the window's size {sw}x{sh} (the desktop's scaled size an \
                     earlier start set) set to the monitor's {w}x{h}"
                ));
            }
            lines.push(("iSize W".into(), w.to_string()));
            lines.push(("iSize H".into(), h.to_string()));
        }
    }
    let mut new = if lines.is_empty() {
        old_prefs.clone()
    } else {
        bytes_of(&settings::apply_to_prefs(&text_of(&old_prefs), &lines))
    };
    if !checked {
        let (section, key) = SIZE_CHECKED;
        new = bytes_of(&game_ini::set_key(&text_of(&new), section, key, "1"));
    }
    if inputs.prefs.as_ref() != Some(&new) {
        out.prefs = Some(new);
    }
    out.seeded = seeded;
    out
}

/// The marker in `FalloutPrefs.ini` (nv-rs's own profile folder, never the game's folders) that
/// the window's first size has been looked at by a start that gives a borderless window the
/// monitor's real pixels: section `[nv-rs]` (the game reads no such section), key
/// `bWindowSizeChecked=1`. With it, the one-time correction of a scaled size never runs again.
const SIZE_CHECKED: (&str, &str) = ("nv-rs", "bWindowSizeChecked");

/// Whether a `FalloutPrefs.ini`'s text carries [`SIZE_CHECKED`].
fn size_checked(text: &str) -> bool {
    let (section, key) = SIZE_CHECKED;
    settings::ini_value(text, section, key).is_some_and(|v| v.trim() == "1")
}

/// The `iSize W` and `iSize H` of a `FalloutPrefs.ini`'s text (keys without case), when both are
/// there.
fn prefs_size(text: &str) -> Option<(u32, u32)> {
    let value = |key: &str| {
        text.lines().find_map(|line| {
            let (k, v) = line.split_once('=')?;
            k.trim()
                .eq_ignore_ascii_case(key)
                .then(|| v.trim().parse::<u32>().ok())
                .flatten()
        })
    };
    value("iSize W").zip(value("iSize H"))
}

/// What a first start takes from the machine table ([`table_facts`]).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// The desktop's size (`GetSystemMetrics` 0 and 1, in the scaled pixels the game sees).
    pub desktop: Option<(u32, u32)>,
    /// The monitor's size in real pixels (`GetAdapterDisplayMode`'s width and height).
    pub screen: Option<(u32, u32)>,
    /// The card's name (`GetAdapterIdentifier`'s description).
    pub device: Option<String>,
}

/// The [`Facts`] in a machine table's text.
pub fn table_facts(table: &str) -> Facts {
    let mut w = None;
    let mut h = None;
    let mut screen = None;
    let mut device = None;
    let bytes_of_hex = |bytes: &str| -> Vec<u8> {
        (0..bytes.len() / 2)
            .filter_map(|i| u8::from_str_radix(bytes.get(2 * i..2 * i + 2)?, 16).ok())
            .collect()
    };
    for line in table.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["GetSystemMetrics", "0", v] => w = u32::from_str_radix(v, 16).ok(),
            ["GetSystemMetrics", "1", v] => h = u32::from_str_radix(v, 16).ok(),
            ["GetAdapterDisplayMode", "0", "0", bytes] => {
                let b = bytes_of_hex(bytes);
                let word =
                    |at: usize| Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?));
                screen = word(0).zip(word(4)).filter(|&(w, h)| w > 0 && h > 0);
            }
            ["GetAdapterIdentifier", "0 0", "0", bytes] => {
                let b = bytes_of_hex(bytes);
                let name = b.get(512..1024).unwrap_or_default();
                let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                device = Some(String::from_utf8_lossy(&name[..end]).into_owned());
            }
            _ => {}
        }
    }
    Facts {
        desktop: w.zip(h),
        screen,
        device: device.filter(|d| !d.is_empty()),
    }
}

/// A file as a first start reads it: its bytes, `None` when it is not there, or why it could not
/// be read (any error but "not found": such a file is never replaced, as Play.exe does).
fn read_file(path: &std::path::Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// How [`first_start_with`] writes a file (the bytes to the path) and moves `settings.ini`.
pub type Write = dyn Fn(&std::path::Path, Vec<u8>) -> Result<(), String>;
pub type Rename = dyn Fn(&std::path::Path, &std::path::Path) -> Result<(), String>;

/// [`first_start`] with its writes and its move of `settings.ini` given (`write`, `rename`), for
/// the tests: the profile in `profile` (`My Games\FalloutNV`), the game in `game_folder`, the
/// pending choices in `settings` (a `settings.ini`, when there is one), the machine table's
/// `table`. A file that was missing and could not be written stops the start (the error): without
/// `FalloutPrefs.ini` naming the card the game asks for its launcher and stops in the renderer
/// (docs/PLAYTESTING.md), and a game with no `Fallout.ini` would not have nv-rs's sections. A file
/// that was there and could not be updated is said in the log; the game reads it as it is.
/// `settings.ini` is moved aside (Play.exe's rule, `game_profile::settings::migrated_name`) once
/// both files hold its choices, so it is applied once.
pub fn first_start_with(
    profile: &std::path::Path,
    game_folder: &std::path::Path,
    settings_file: Option<&std::path::Path>,
    table: &str,
    write: &Write,
    rename: &Rename,
) -> Result<Vec<String>, String> {
    let ini_path = profile.join("Fallout.ini");
    let prefs_path = profile.join("FalloutPrefs.ini");
    let ini = read_file(&ini_path);
    let prefs = read_file(&prefs_path);
    let pending = settings_file
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| Settings::parse(&settings::decode(&b)));
    let facts = table_facts(table);
    let read = |file: &Result<Option<Vec<u8>>, String>| file.as_ref().ok().cloned().flatten();
    let outcome = prepare(&Inputs {
        ini: read(&ini),
        prefs: read(&prefs),
        default_ini: std::fs::read(game_folder.join("Fallout_default.ini")).ok(),
        high_ini: std::fs::read(game_folder.join("high.ini")).ok(),
        pending: pending.as_ref(),
        desktop: facts.desktop,
        screen: facts.screen,
        device: facts.device.as_deref(),
    });
    let mut log = outcome.log.clone();
    let mut all_written = true;
    for (path, before, bytes) in [
        (&ini_path, &ini, outcome.ini),
        (&prefs_path, &prefs, outcome.prefs),
    ] {
        if let Err(why) = before {
            log.push(format!("could not be read, so left as it is: {why}"));
            all_written = false;
            continue;
        }
        let Some(bytes) = bytes else { continue };
        if let Err(why) = write(path, bytes) {
            if matches!(before, Ok(None)) {
                return Err(format!(
                    "nv-rs could not make the game's first {} ({why}). The game needs it to start \
                     (without it the game asks for its own launcher and stops). Make sure nv-rs's \
                     own folder can be written, then start again.",
                    path.display()
                ));
            }
            log.push(format!("{}: not updated: {why}", path.display()));
            all_written = false;
        }
    }
    if let (Some(file), true, true) = (
        settings_file,
        pending.is_some(),
        outcome.seeded && all_written,
    ) {
        let moved = settings::migrated_name(file);
        match rename(file, &moved) {
            Ok(()) => log.push(format!(
                "{} moved into the files and renamed {}",
                file.display(),
                moved.display()
            )),
            Err(why) => log.push(format!(
                "{}: not renamed ({why}): the next start moves it in again",
                file.display()
            )),
        }
    }
    Ok(log)
}

/// Off Windows, at a play run's start (after the renderer choice and the machine table, before the
/// game reads its INI files): the profile files in `documents` (`My Games\FalloutNV`) prepared
/// from the game in `game_folder` and the table's facts ([`first_start_with`]), written through
/// the save guard (`save_out::run`: a refused path is never written). Each step is said in the
/// log; the error stops the start.
#[cfg(not(windows))]
pub fn first_start(
    documents: &std::path::Path,
    game_folder: &std::path::Path,
    table: &str,
) -> Result<(), String> {
    let profile = documents.join("My Games").join("FalloutNV");
    let settings_file = crate::fixes::settings_file();
    let write = |target: &std::path::Path, bytes: Vec<u8>| -> Result<(), String> {
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let job = crate::save_out::Job::Write {
            key: Vec::new(),
            dir: target.parent().map(std::path::Path::to_path_buf),
            temporary: target.with_file_name(format!("{name}.part")),
            target: target.to_path_buf(),
            bytes,
            copied: None,
        };
        crate::save_out::run(&job).map(|_| ())
    };
    let rename = |from: &std::path::Path, to: &std::path::Path| -> Result<(), String> {
        match crate::save_out::refused_to_write(to) {
            Some(why) => Err(format!("refused ({why})")),
            None => std::fs::rename(from, to).map_err(|e| e.to_string()),
        }
    };
    let log = first_start_with(
        &profile,
        game_folder,
        settings_file.as_deref(),
        table,
        &write,
        &rename,
    )?;
    for line in log {
        eprintln!("first start: {line}");
    }
    Ok(())
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
            screen: Some((2560, 1440)),
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

    /// A card whose name changed since the last start (a driver update can rename it) is said in
    /// the log and the new name written; the same name in another case (the game compares without
    /// case) is not.
    #[test]
    fn a_renamed_card_is_said_and_written() {
        let renamed = |before: &str| {
            let prefs = format!(
                "[Display]\r\nsD3DDevice=\"{before}\"\r\n[nv-rs]\r\nbWindowSizeChecked=1\r\n"
            );
            first(None, Some(prefs.as_bytes()))
        };
        let out = renamed("AMD Radeon RX 7800 XT (RADV GFX1101)");
        assert!(
            out.log.iter().any(|l| l.contains(
                "changed from \"AMD Radeon RX 7800 XT (RADV GFX1101)\" to \"AMD Radeon RX 7800 XT (RADV NAVI32)\""
            )),
            "{:?}",
            out.log
        );
        let prefs = text_of(out.prefs.as_deref().unwrap());
        assert!(
            prefs.contains("sD3DDevice=\"AMD Radeon RX 7800 XT (RADV NAVI32)\""),
            "{prefs}"
        );
        let out = renamed("amd radeon rx 7800 xt (radv navi32)");
        assert!(
            !out.log.iter().any(|l| l.contains("changed from")),
            "{:?}",
            out.log
        );
    }

    /// A first start (no FalloutPrefs.ini yet) compares no name, even when the High preset it
    /// starts from names another card.
    #[test]
    fn a_first_start_says_no_renamed_card() {
        let out = prepare(&Inputs {
            ini: None,
            prefs: None,
            default_ini: Some(DEFAULT_INI.to_vec()),
            high_ini: Some(b"[Display]\r\nsD3DDevice=\"Some Older Card\"\r\n".to_vec()),
            pending: None,
            desktop: Some((2560, 1440)),
            screen: Some((2560, 1440)),
            device: Some("AMD Radeon RX 7800 XT (RADV NAVI32)"),
        });
        assert!(
            !out.log.iter().any(|l| l.contains("changed from")),
            "{:?}",
            out.log
        );
        let prefs = text_of(out.prefs.as_deref().unwrap());
        assert!(
            prefs.contains("sD3DDevice=\"AMD Radeon RX 7800 XT (RADV NAVI32)\""),
            "{prefs}"
        );
    }

    /// A file the player edited is not replaced: Fallout.ini already with the sections is left
    /// as it is; FalloutPrefs.ini keeps its own size, only the play lines are set.
    #[test]
    fn an_existing_players_file_is_not_overwritten() {
        let made = first(None, None);
        let ini = made.ini.unwrap();
        let mut edited = ini.clone();
        edited.extend_from_slice(b"[Player]\r\nmine=1\r\n");
        let prefs = b"[Display]\r\niSize W=1920\r\niSize H=1080\r\nbFull Screen=0\r\nsD3DDevice=\"AMD Radeon RX 7800 XT (RADV NAVI32)\"\r\nfGamma=1.2\r\n[nv-rs]\r\nbWindowSizeChecked=1\r\n";
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
            screen: None,
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
            Facts {
                desktop: Some((2048, 1152)),
                screen: None,
                device: Some("Card".into())
            }
        );
        assert_eq!(table_facts(""), Facts::default());
    }

    /// The facts from the lines the machine table really writes (a 2x monitor, a card).
    #[test]
    fn the_facts_come_from_the_real_table_lines() {
        use engine_platform::machine_table::display::{card_lines, display_lines};
        use engine_platform::machine_table::{Card, Monitor};
        let monitor = Monitor {
            size: (3840, 2160),
            scale: 2.0,
            refresh_millihertz: Some(60_000),
            modes: Vec::new(),
            primary: true,
        };
        let card = Card {
            name: "Intel(R) Arc(tm) A770 Graphics (DG2)".into(),
            vendor: 0x8086,
            device: 0x56a0,
            driver: "Intel open-source Mesa driver".into(),
            driver_info: "Mesa 25.0.7".into(),
            sample_counts: vec![1, 2, 4, 8],
            bc: true,
            float32_blendable: true,
        };
        let table = card_lines(&card) + &display_lines(&[monitor], &card.name).unwrap();
        assert_eq!(
            table_facts(&table),
            Facts {
                desktop: Some((1920, 1080)),
                screen: Some((3840, 2160)),
                device: Some(card.name.clone())
            }
        );
    }

    /// A desktop scaled by a non-whole factor (the first Linux start's: 2560x1440 at about 1.17,
    /// read by the game as 2194x1234): borderless, the window's size is the monitor's real pixels;
    /// framed, the scaled size the window is made in; a FalloutPrefs.ini an earlier start made at the
    /// scaled size is corrected once, one the player set to another size is not.
    #[test]
    fn a_scaled_desktop_gives_the_borderless_window_its_real_pixels() {
        use engine_platform::machine_table::display::display_lines;
        use engine_platform::machine_table::Monitor;
        let monitor = Monitor {
            size: (2560, 1440),
            scale: 2560.0 / 2194.0,
            refresh_millihertz: Some(165_000),
            modes: Vec::new(),
            primary: true,
        };
        let facts = table_facts(&display_lines(&[monitor], "card").unwrap());
        assert_eq!(facts.desktop, Some((2194, 1234)));
        assert_eq!(facts.screen, Some((2560, 1440)));
        let inputs = |prefs: Option<&[u8]>, pending: Option<&Settings>| -> Outcome {
            prepare(&Inputs {
                ini: None,
                prefs: prefs.map(<[u8]>::to_vec),
                default_ini: Some(DEFAULT_INI.to_vec()),
                high_ini: Some(HIGH_INI.to_vec()),
                pending,
                desktop: facts.desktop,
                screen: facts.screen,
                device: Some("card"),
            })
        };
        let prefs = text_of(inputs(None, None).prefs.as_deref().unwrap());
        assert!(
            prefs.contains("iSize W=2560") && prefs.contains("iSize H=1440"),
            "{prefs}"
        );
        assert!(prefs.contains("bFull Screen=0"), "{prefs}");
        // A fresh file is marked at once: nothing to correct later.
        assert!(size_checked(&prefs), "{prefs}");
        let framed = Settings::parse("Borderless=0\n");
        let prefs = text_of(inputs(None, Some(&framed)).prefs.as_deref().unwrap());
        assert!(
            prefs.contains("iSize W=2194") && prefs.contains("iSize H=1234"),
            "{prefs}"
        );
        // The volunteer's file from the first start: corrected, with a line.
        let earlier = b"[Display]\r\niSize W=2194\r\niSize H=1234\r\nbFull Screen=0\r\n";
        let out = inputs(Some(earlier), None);
        let prefs = text_of(out.prefs.as_deref().unwrap());
        assert!(
            prefs.contains("iSize W=2560") && prefs.contains("iSize H=1440"),
            "{prefs}"
        );
        assert!(out
            .log
            .iter()
            .any(|l| l.contains("set to the monitor's 2560x1440")));
        assert!(size_checked(&prefs), "{prefs}");
        // Once marked, the scaled size is the player's choice (picked later in the game or by
        // hand): kept on every later start, the file unchanged.
        let chosen = b"[Display]\r\niSize W=2194\r\niSize H=1234\r\nbFull Screen=0\r\nsD3DDevice=\"card\"\r\n[nv-rs]\r\nbWindowSizeChecked=1\r\n";
        let out = inputs(Some(chosen), None);
        assert_eq!(out.prefs, None, "{:?}", out.prefs.as_deref().map(text_of));
        assert!(!out.log.iter().any(|l| l.contains("set to the monitor's")));
        // A size of the player's own: kept.
        let own = b"[Display]\r\niSize W=1920\r\niSize H=1080\r\nbFull Screen=0\r\n";
        let out = inputs(Some(own), None);
        let prefs = out.prefs.as_deref().map_or_else(|| text_of(own), text_of);
        assert!(prefs.contains("iSize W=1920"), "{prefs}");
        assert!(!prefs.contains("2560"), "{prefs}");
    }

    /// The player's pending choices: their resolution beats the desktop's size, and with the
    /// window as the game makes it (Borderless=0) no `bFull Screen=0` is set.
    #[test]
    fn pending_choices_go_in() {
        let pending = Settings::parse("Resolution=1920x1080\nBorderless=0\n");
        let out = prepare(&Inputs {
            ini: None,
            prefs: None,
            default_ini: Some(DEFAULT_INI.to_vec()),
            high_ini: Some(HIGH_INI.to_vec()),
            pending: Some(&pending),
            desktop: Some((2560, 1440)),
            screen: Some((2560, 1440)),
            device: Some("card"),
        });
        assert!(out.seeded);
        let ini = text_of(out.ini.as_deref().unwrap());
        assert!(!game_ini::borderless(&ini), "{ini}");
        let prefs = text_of(out.prefs.as_deref().unwrap());
        assert!(
            prefs.contains("iSize W=1920") && prefs.contains("iSize H=1080"),
            "{prefs}"
        );
        assert!(!prefs.contains("2560"), "{prefs}");
        assert!(
            prefs.contains("bFull Screen=1") && !prefs.contains("bFull Screen=0"),
            "{prefs}"
        );
    }

    /// A scratch profile and game folder for [`first_start_with`].
    fn scratch(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("nv-first-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let profile = root.join("profile");
        let game = root.join("game");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("Fallout_default.ini"), DEFAULT_INI).unwrap();
        std::fs::write(game.join("high.ini"), HIGH_INI).unwrap();
        (root, profile, game)
    }

    fn plain_write(path: &std::path::Path, bytes: Vec<u8>) -> Result<(), String> {
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    fn plain_rename(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
        std::fs::rename(from, to).map_err(|e| e.to_string())
    }

    /// A refused write of a file that is missing at a first start stops the start, naming it; of
    /// a file already there, it is said and the start goes on.
    #[test]
    fn a_refused_first_write_stops_the_start() {
        let (root, profile, game) = scratch("refused");
        let refuse_prefs = |path: &std::path::Path, bytes: Vec<u8>| {
            if path.ends_with("FalloutPrefs.ini") {
                Err("refused (a link out of nv-rs's folder)".to_string())
            } else {
                plain_write(path, bytes)
            }
        };
        let err =
            first_start_with(&profile, &game, None, "", &refuse_prefs, &plain_rename).unwrap_err();
        assert!(
            err.contains("FalloutPrefs.ini") && err.contains("refused"),
            "{err}"
        );
        // Once it is there, a refused update only goes into the log.
        std::fs::write(profile.join("FalloutPrefs.ini"), b"[Display]\r\n").unwrap();
        let log =
            first_start_with(&profile, &game, None, "", &refuse_prefs, &plain_rename).unwrap();
        assert!(log.iter().any(|l| l.contains("not updated")), "{log:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A file that cannot be read (here a folder by its name) is never replaced.
    #[test]
    fn an_unreadable_file_is_left_as_it_is() {
        let (root, profile, game) = scratch("unreadable");
        std::fs::create_dir_all(profile.join("Fallout.ini")).unwrap();
        let log = first_start_with(&profile, &game, None, "", &plain_write, &plain_rename).unwrap();
        assert!(
            log.iter().any(|l| l.contains("could not be read")),
            "{log:?}"
        );
        assert!(profile.join("Fallout.ini").is_dir());
        assert!(profile.join("FalloutPrefs.ini").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// settings.ini is moved aside once both files hold its choices: applied once.
    #[test]
    fn settings_ini_is_applied_once() {
        let (root, profile, game) = scratch("settings");
        let settings_file = root.join("settings.ini");
        std::fs::write(&settings_file, b"Resolution=1600x900\n").unwrap();
        let log = first_start_with(
            &profile,
            &game,
            Some(&settings_file),
            "",
            &plain_write,
            &plain_rename,
        )
        .unwrap();
        assert!(!settings_file.exists(), "{log:?}");
        assert!(root.join("settings.ini.migrated").is_file());
        let prefs = std::fs::read(profile.join("FalloutPrefs.ini")).unwrap();
        assert!(text_of(&prefs).contains("iSize W=1600"));
        // The player changes the size later; the next start does not apply settings.ini again.
        let changed = text_of(&prefs).replace("iSize W=1600", "iSize W=1280");
        std::fs::write(profile.join("FalloutPrefs.ini"), bytes_of(&changed)).unwrap();
        first_start_with(
            &profile,
            &game,
            Some(&settings_file),
            "",
            &plain_write,
            &plain_rename,
        )
        .unwrap();
        let again = text_of(&std::fs::read(profile.join("FalloutPrefs.ini")).unwrap());
        assert!(again.contains("iSize W=1280"), "{again}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
