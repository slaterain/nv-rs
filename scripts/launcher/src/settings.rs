//! `userdata\settings.ini`: `Name=Value` lines Settings.cmd wrote before the player's options
//! moved into the game's `Fallout.ini` (and wrote until Play.exe's first start). Play.exe moves
//! it into the profiles' files once (`game_ini.rs`) and renames it `settings.ini.migrated`.
//! `Preset`, `FrameLimit` and `MenuFrameLimit` are checked as the game would read them
//! ([`Settings::game_switches`]); names the launcher doesn't know are passed over.
use std::collections::BTreeMap;

/// Menus' frames a second at most with no `MenuFrameLimit=` line: the game's default.
const MENU_FRAME_LIMIT: u32 = 120;

/// The file's text as the game decodes it (host `fixes.rs`): a UTF-8 byte order mark dropped,
/// UTF-16 (what PowerShell 5.1's `>` writes) decoded, anything else read as UTF-8 with bad bytes
/// replaced. The launcher passes the preset on, so it must read the same lines the game would.
pub fn decode(bytes: &[u8]) -> String {
    let utf16 = |rest: &[u8], big: bool| {
        let units: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| {
                if big {
                    u16::from_be_bytes(c)
                } else {
                    u16::from_le_bytes(c)
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xff, 0xfe, rest @ ..] => utf16(rest, false),
        [0xfe, 0xff, rest @ ..] => utf16(rest, true),
        [0xef, 0xbb, 0xbf, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// The value of `key` in `section` of an INI text, found as the game finds it (host
/// `game_ini::value`): only in the first section of that name (Windows reads no later one,
/// measured), names compared without case, spaces trimmed, one pair of quotes taken off, the
/// first match wins; a `;` line never matches.
pub fn ini_value<'a>(text: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix('[') {
            // The first section of the name ends here.
            if inside {
                return None;
            }
            let name = rest.split(']').next().unwrap_or(rest);
            inside = name.trim().eq_ignore_ascii_case(section.trim());
            continue;
        }
        let Some((name, value)) = line.split_once('=').filter(|_| inside) else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(key) {
            let value = value.trim();
            return Some(
                value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(value),
            );
        }
    }
    None
}

/// The file's settings.
#[derive(Debug, Default, PartialEq)]
pub struct Settings(pub BTreeMap<String, String>);

impl Settings {
    pub fn parse(text: &str) -> Settings {
        let mut map = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some((name, value)) = line.split_once('=') {
                // The last line of a name wins, its case aside, as the game and Settings.ps1 read it.
                let name = name.trim();
                map.retain(|k: &String, _| !k.eq_ignore_ascii_case(name));
                map.insert(name.to_string(), value.trim().to_string());
            }
        }
        Settings(map)
    }

    fn get(&self, name: &str) -> &str {
        self.0.get(name).map_or("", String::as_str)
    }

    /// The `Borderless=` line's choice: `0` is the window as the game makes it, anything else the
    /// borderless window. `None` with no line or an empty one: nothing was chosen here.
    pub fn borderless(&self) -> Option<bool> {
        match self.get_any_case("Borderless").map(str::trim) {
            None | Some("") => None,
            Some("0") => Some(false),
            Some(_) => Some(true),
        }
    }

    /// A setting by its name compared without case, as the game reads `Preset` and `FrameLimit`.
    fn get_any_case(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// The game's fixes preset and frame limits as its switches, `NV_PRESET` (`fixes` or `vanilla`),
    /// `NV_FRAME_LIMIT` and `NV_MENU_FRAME_LIMIT` (frames a second, 0 for no limit), from
    /// `Preset=`, `FrameLimit=` and `MenuFrameLimit=`: the fixes, no limit and menus at 120 (the
    /// game's own default, host `fixes.rs`) when a line is missing or empty. A value the game
    /// wouldn't take is an error the player reads, never a silent default.
    pub fn game_switches(&self) -> Result<[(&'static str, String); 3], String> {
        let preset = match self.get_any_case("Preset").map(str::trim) {
            None | Some("") => "fixes".to_string(),
            Some(v) if v.eq_ignore_ascii_case("fixes") => "fixes".to_string(),
            Some(v) if v.eq_ignore_ascii_case("vanilla") => "vanilla".to_string(),
            Some(v) => return Err(format!("Preset={v} (it can be fixes or vanilla)")),
        };
        let limit = match self.get_any_case("FrameLimit").map(str::trim) {
            None | Some("") => 0,
            Some(v) => v
                .parse::<u32>()
                .map_err(|_| format!("FrameLimit={v} (it can be a number, 0 for no limit)"))?,
        };
        let menu = match self.get_any_case("MenuFrameLimit").map(str::trim) {
            None | Some("") => MENU_FRAME_LIMIT,
            Some(v) => v
                .parse::<u32>()
                .map_err(|_| format!("MenuFrameLimit={v} (it can be a number, 0 for no limit)"))?,
        };
        Ok([
            ("NV_PRESET", preset),
            ("NV_FRAME_LIMIT", limit.to_string()),
            ("NV_MENU_FRAME_LIMIT", menu.to_string()),
        ])
    }

    /// The `FalloutPrefs.ini` lines these settings set, by the game's own names: resolution,
    /// anti-aliasing and anisotropic filtering. The borderless window's `bFull Screen=0` is not
    /// among them: Play.exe sets it at every start from what the profile's `Fallout.ini` says
    /// once these settings are in it (`prepare_profile`), so the two can't disagree.
    pub fn prefs_lines(&self) -> Vec<(String, String)> {
        let mut set = Vec::new();
        let times = |v: &str| match v {
            "Off" => Some(0u32),
            v => v.strip_suffix('x').and_then(|n| n.parse().ok()),
        };
        if let Some((w, h)) = self.get("Resolution").split_once('x') {
            if w.parse::<u32>().is_ok() && h.parse::<u32>().is_ok() {
                set.push(("iSize W".into(), w.into()));
                set.push(("iSize H".into(), h.into()));
            }
        }
        if let Some(n) = times(self.get("AntiAliasing")) {
            set.push(("iMultiSample".into(), n.to_string()));
        }
        if let Some(n) = times(self.get("Anisotropy")) {
            set.push(("iMaxAnisotropy".into(), n.to_string()));
        }
        set
    }
}

/// `prefs` (a `FalloutPrefs.ini`) with these lines set: each replaced where its name stands, and
/// those it doesn't have added to `[Display]` (the section made when missing). Line ends stay.
pub fn apply_to_prefs(prefs: &str, lines: &[(String, String)]) -> String {
    let mut missing: Vec<&(String, String)> = lines.iter().collect();
    let mut out = String::with_capacity(prefs.len() + 64);
    for line in prefs.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let end = &line[body.len()..];
        let set = body
            .split_once('=')
            .and_then(|(name, _)| lines.iter().find(|(n, _)| n == name));
        match set {
            Some((name, value)) => {
                out.push_str(name);
                out.push('=');
                out.push_str(value);
                out.push_str(end);
                missing.retain(|(n, _)| n != name);
            }
            None => out.push_str(line),
        }
    }
    if missing.is_empty() {
        return out;
    }
    let mut added = String::new();
    for (name, value) in &missing {
        added.push_str(name);
        added.push('=');
        added.push_str(value);
        added.push_str("\r\n");
    }
    if let Some(at) = out.find("[Display]") {
        let after = out[at..].find('\n').map_or(out.len(), |n| at + n + 1);
        if after == out.len() && !out.ends_with('\n') {
            out.push_str("\r\n");
            out.push_str(&added);
        } else {
            out.insert_str(after, &added);
        }
    } else {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str("\r\n");
        }
        out.push_str("[Display]\r\n");
        out.push_str(&added);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &[(&str, &str)]) -> Vec<(String, String)> {
        s.iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn a_key_is_found_as_the_game_finds_it() {
        let ini =
            " [ NV-RS display ]\n ;iFrameLimit=9\n IFRAMELIMIT = \"5\" \n[Other]\niFrameLimit=7\n";
        assert_eq!(ini_value(ini, "nv-rs Display", "iFrameLimit"), Some("5"));
        assert_eq!(ini_value(ini, "Other", "iFrameLimit"), Some("7"));
        assert_eq!(ini_value(ini, "nv-rs Display", "bBorderless"), None);
        // A repeated section name: only the first section counts, as Windows reads it
        // (measure/ini/repeated-sections.md); an empty first one hides the later one too.
        let ini = "[A]\r\nx=1\r\n[B]\r\nz=3\r\n[a]\r\ny=2\r\nx=9\r\n[ A ]\r\nw=6\r\n";
        assert_eq!(ini_value(ini, "A", "x"), Some("1"));
        assert_eq!(ini_value(ini, "A", "y"), None);
        assert_eq!(ini_value(ini, "A", "w"), None);
        assert_eq!(ini_value(ini, " A ", "x"), Some("1"));
        assert_eq!(ini_value("[A]\r\n[A]\r\nx=1\r\n", "A", "x"), None);
        assert_eq!(ini_value("[A]\r\nx=\r\n[A]\r\nx=1\r\n", "A", "x"), Some(""));
    }

    #[test]
    fn the_file_is_read_and_unknown_names_do_no_harm() {
        // V-sync is not a FalloutPrefs.ini line: the game reads iPresentInterval from Fallout.ini.
        let s = Settings::parse(
            "# a comment\nBorderless=1\nResolution=2560x1440\nAntiAliasing=8x\nAnisotropy=Off\nVSync=On\nSomethingNew=7\nNV_FIXES=vnv\n",
        );
        assert_eq!(s.borderless(), Some(true));
        // The borderless window's bFull Screen=0 is not set from here: Play.exe sets it from the
        // profile's Fallout.ini.
        assert_eq!(
            s.prefs_lines(),
            lines(&[
                ("iSize W", "2560"),
                ("iSize H", "1440"),
                ("iMultiSample", "8"),
                ("iMaxAnisotropy", "0"),
            ])
        );
        // No Borderless line, or an empty one, chooses nothing.
        assert_eq!(Settings::parse("").borderless(), None);
        assert_eq!(Settings::parse("Borderless=\n").borderless(), None);
        assert_eq!(Settings::parse("borderless = 0").borderless(), Some(false));
        assert_eq!(Settings::parse("Borderless=yes").borderless(), Some(true));
        assert!(Settings::parse("Borderless=1\nResolution=Game launcher's")
            .prefs_lines()
            .is_empty());
    }

    fn switches(text: &str) -> Result<Vec<(&'static str, String)>, String> {
        Settings::parse(text).game_switches().map(Vec::from)
    }

    fn pair(preset: &str, limit: &str) -> Vec<(&'static str, String)> {
        with_menu(preset, limit, "120")
    }

    fn with_menu(preset: &str, limit: &str, menu: &str) -> Vec<(&'static str, String)> {
        vec![
            ("NV_PRESET", preset.to_string()),
            ("NV_FRAME_LIMIT", limit.to_string()),
            ("NV_MENU_FRAME_LIMIT", menu.to_string()),
        ]
    }

    #[test]
    fn the_menu_frame_limit_becomes_the_games_switch() {
        assert_eq!(
            switches("MenuFrameLimit=\n"),
            Ok(with_menu("fixes", "0", "120"))
        );
        assert_eq!(
            switches("FrameLimit=144\nMenuFrameLimit=60\n"),
            Ok(with_menu("fixes", "144", "60"))
        );
        assert_eq!(
            switches("menuframelimit = 0\n"),
            Ok(with_menu("fixes", "0", "0"))
        );
        let e = switches("MenuFrameLimit=fast\n").unwrap_err();
        assert!(e.contains("MenuFrameLimit=fast"), "{e}");
    }

    #[test]
    fn preset_and_frame_limit_become_the_games_switches() {
        // Nothing chosen, or an empty line: the fixes and no limit.
        assert_eq!(switches(""), Ok(pair("fixes", "0")));
        assert_eq!(switches("Preset=\nFrameLimit=\n"), Ok(pair("fixes", "0")));
        // What Settings.cmd writes.
        assert_eq!(
            switches("Preset=vanilla\nFrameLimit=144\n"),
            Ok(pair("vanilla", "144"))
        );
        assert_eq!(
            switches("Preset=fixes\nFrameLimit=0\n"),
            Ok(pair("fixes", "0"))
        );
        // Written by hand: case and spaces as the game takes them.
        assert_eq!(
            switches("preset = Vanilla \nframelimit= 60\n"),
            Ok(pair("vanilla", "60"))
        );
        // A value the game would refuse stops here, with the line named.
        let e = switches("Preset=vanila\n").unwrap_err();
        assert!(e.contains("Preset=vanila"), "{e}");
        let e = switches("FrameLimit=60fps\n").unwrap_err();
        assert!(e.contains("FrameLimit=60fps"), "{e}");
        assert!(switches("FrameLimit=-1\n").is_err());
        // A stale NV_FIXES line chooses nothing: the preset stays the fixes.
        assert_eq!(switches("NV_FIXES=vanilla\n"), Ok(pair("fixes", "0")));
    }

    #[test]
    fn the_file_is_decoded_as_the_game_decodes_it() {
        let text = "Preset=vanilla\r\n";
        let mut utf16 = vec![0xff, 0xfe];
        utf16.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        let mut big = vec![0xfe, 0xff];
        big.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
        let mut marked = vec![0xef, 0xbb, 0xbf];
        marked.extend(text.bytes());
        for bytes in [utf16, big, marked, text.as_bytes().to_vec()] {
            assert_eq!(
                Settings::parse(&decode(&bytes)).game_switches().unwrap()[0].1,
                "vanilla"
            );
        }
    }

    #[test]
    fn prefs_keep_everything_else() {
        let prefs = "[Display]\r\niSize W=800\r\nbFull Screen=1\r\n[Audio]\r\nfVolume=1\r\n";
        let out = apply_to_prefs(prefs, &lines(&[("iSize W", "1920"), ("bFull Screen", "0")]));
        assert_eq!(
            out,
            "[Display]\r\niSize W=1920\r\nbFull Screen=0\r\n[Audio]\r\nfVolume=1\r\n"
        );
    }

    #[test]
    fn what_prefs_lack_goes_into_display() {
        let out = apply_to_prefs(
            "[Audio]\r\nfVolume=1\r\n[Display]\r\niSize W=800\r\n",
            &lines(&[("iSize H", "600")]),
        );
        assert_eq!(
            out,
            "[Audio]\r\nfVolume=1\r\n[Display]\r\niSize H=600\r\niSize W=800\r\n"
        );
        let out = apply_to_prefs("", &lines(&[("iSize H", "600")]));
        assert_eq!(out, "[Display]\r\niSize H=600\r\n");
        let out = apply_to_prefs("[Audio]\nfVolume=1", &lines(&[("iSize H", "600")]));
        assert_eq!(out, "[Audio]\nfVolume=1\r\n[Display]\r\niSize H=600\r\n");
    }

    #[test]
    fn the_last_line_of_a_name_wins_whatever_its_case() {
        let s = Settings::parse("Preset=fixes\npreset=vanilla\n");
        assert_eq!(s.game_switches().unwrap()[0].1, "vanilla");
        let s = Settings::parse("preset=vanila\nPreset=fixes\nFrameLimit=60\nframelimit=0\n");
        let sw = s.game_switches().unwrap();
        assert_eq!((sw[0].1.as_str(), sw[1].1.as_str()), ("fixes", "0"));
    }
}
