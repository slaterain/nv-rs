//! The Settings screen's rows, with no window in them: Settings.ps1's twelve rows by the same
//! names, choices, defaults and keys (`distribution/Settings.ps1`, the census in the launcher
//! track's notes), read from and written to the profile's own `Fallout.ini` and
//! `FalloutPrefs.ini` through the shared code the game's side uses (`game_profile`). A row that is
//! changed writes only its own keys, as Settings.ps1 saves only the options changed in it; "Game's
//! setting" writes nothing. Our keys' defaults are the host's table's (`crates/host/data/
//! game_ini.tsv`), so what the screen shows for a key that is missing is what the game does.

use game_profile::game_ini::set_key;
use game_profile::settings::ini_value;

/// The host's table: section, key, variable, default, note.
const TABLE: &str = include_str!("../../host/data/game_ini.tsv");

/// "Game's setting": keeps what the game's own file says (Settings.ps1's `$own`).
pub const OWN: &str = "Game's setting";

/// The two files of a profile, as text one character a byte (every other byte stays).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Files {
    pub ini: String,
    pub prefs: String,
}

/// A row of the Settings screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Row {
    Renderer,
    DisplayMode,
    Resolution,
    AntiAliasing,
    Anisotropy,
    VSync,
    GameFixes,
    FrameLimit,
    MenuFrameLimit,
    FrameRateCounter,
    GameThreads,
    DrawingThread,
}

/// The rows in Settings.ps1's order.
pub const ROWS: [Row; 12] = [
    Row::Renderer,
    Row::DisplayMode,
    Row::Resolution,
    Row::AntiAliasing,
    Row::Anisotropy,
    Row::VSync,
    Row::GameFixes,
    Row::FrameLimit,
    Row::MenuFrameLimit,
    Row::FrameRateCounter,
    Row::GameThreads,
    Row::DrawingThread,
];

/// The host table's default for one of our keys.
fn default_of(key: &str) -> &'static str {
    TABLE
        .lines()
        .filter(|l| !l.starts_with('#'))
        .find_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 4 && f[1] == key).then_some(f[3])
        })
        .unwrap_or("")
}

/// Our key's value in the profile's Fallout.ini, else the table's default.
fn ours(files: &Files, section: &str, key: &str) -> String {
    ini_value(&files.ini, section, key)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default_of(key))
        .trim()
        .to_string()
}

/// "2x" for a sample count, "Off" for 0, the game's own for anything else.
fn times(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some("0") => "Off".into(),
        Some(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => format!("{n}x"),
        _ => OWN.into(),
    }
}

impl Row {
    /// Its name on the screen (Settings.ps1's labels).
    pub fn label(self) -> &'static str {
        match self {
            Row::Renderer => "Renderer",
            Row::DisplayMode => "Display mode",
            Row::Resolution => "Resolution",
            Row::AntiAliasing => "Anti-aliasing",
            Row::Anisotropy => "Anisotropic filtering",
            Row::VSync => "V-sync",
            Row::GameFixes => "Game fixes",
            Row::FrameLimit => "Frame rate limit",
            Row::MenuFrameLimit => "Frame rate limit in menus",
            Row::FrameRateCounter => "Frame rate counter",
            Row::GameThreads => "Game threads",
            Row::DrawingThread => "Drawing thread",
        }
    }

    /// Its choices, the recommended one first where Settings.ps1 marks one; `desktop` is the
    /// main monitor's size for the resolution row. A value the list lacks (written by hand) is
    /// kept as a choice of its own, as Settings.ps1 keeps it.
    pub fn choices(self, files: &Files, desktop: (u32, u32)) -> Vec<String> {
        let fixed: &[&str] = match self {
            Row::Renderer => &["Vulkan (recommended)", "Direct3D 9 (the original's)"],
            Row::DisplayMode => &["Borderless fullscreen", "Window"],
            Row::Resolution => &[],
            Row::AntiAliasing => &[OWN, "Off", "2x", "4x", "8x"],
            Row::Anisotropy => &[OWN, "Off", "2x", "4x", "8x", "15x"],
            Row::VSync => &[OWN, "On", "Off"],
            Row::GameFixes => &["Fixes (recommended)", "Vanilla (the original game)"],
            Row::FrameLimit => &["No limit", "60", "90", "120", "144", "165", "240"],
            Row::MenuFrameLimit => &["No limit", "30", "60", "90", "120", "144", "165", "240"],
            Row::FrameRateCounter => &["Hidden (F11 shows it)", "Shown (F11 hides it)"],
            Row::GameThreads => &["Side by side (recommended)", "One at a time (slower)"],
            Row::DrawingThread => &["Own thread (recommended)", "Main thread (slower)"],
        };
        let mut list: Vec<String> = if self == Row::Resolution {
            let native = format!("{}x{}", desktop.0, desktop.1);
            let mut l = vec![OWN.to_string()];
            for s in [
                native.as_str(),
                "3840x2160",
                "2560x1440",
                "1920x1080",
                "1600x900",
                "1280x720",
            ] {
                if !l.iter().any(|x| x == s) {
                    l.push(s.to_string());
                }
            }
            l
        } else {
            fixed.iter().map(|s| s.to_string()).collect()
        };
        let now = self.current(files);
        if !list.contains(&now) {
            list.push(now);
        }
        list
    }

    /// The choice the files say now.
    pub fn current(self, files: &Files) -> String {
        let display = |key: &str| ini_value(&files.prefs, "Display", key);
        match self {
            Row::Renderer => {
                if ours(files, "nv-rs Display", "sRenderer").eq_ignore_ascii_case("d3d9") {
                    "Direct3D 9 (the original's)".into()
                } else {
                    "Vulkan (recommended)".into()
                }
            }
            Row::DisplayMode => {
                if ours(files, "nv-rs Display", "bBorderless") == "0" {
                    "Window".into()
                } else {
                    "Borderless fullscreen".into()
                }
            }
            Row::Resolution => match (display("iSize W"), display("iSize H")) {
                (Some(w), Some(h))
                    if [w, h]
                        .iter()
                        .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())) =>
                {
                    format!("{w}x{h}")
                }
                _ => OWN.into(),
            },
            Row::AntiAliasing => times(display("iMultiSample")),
            Row::Anisotropy => times(display("iMaxAnisotropy")),
            Row::VSync => match ini_value(&files.ini, "Display", "iPresentInterval").map(str::trim)
            {
                Some("1") => "On".into(),
                Some("0") => "Off".into(),
                _ => OWN.into(),
            },
            Row::GameFixes => {
                if ours(files, "nv-rs General", "sPreset").eq_ignore_ascii_case("vanilla") {
                    "Vanilla (the original game)".into()
                } else {
                    "Fixes (recommended)".into()
                }
            }
            Row::FrameLimit => match ours(files, "nv-rs Display", "iFrameLimit").as_str() {
                "0" => "No limit".into(),
                v => v.into(),
            },
            Row::MenuFrameLimit => match ours(files, "nv-rs Display", "iMenuFrameLimit").as_str() {
                "0" => "No limit".into(),
                v => v.into(),
            },
            Row::FrameRateCounter => {
                if ours(files, "nv-rs Display", "bShowFrameRate") == "1" {
                    "Shown (F11 hides it)".into()
                } else {
                    "Hidden (F11 shows it)".into()
                }
            }
            Row::GameThreads => {
                if ours(files, "nv-rs Threads", "bSteppedThreads") == "1" {
                    "One at a time (slower)".into()
                } else {
                    "Side by side (recommended)".into()
                }
            }
            Row::DrawingThread => {
                if ours(files, "nv-rs Threads", "bRenderThread") == "0" {
                    "Main thread (slower)".into()
                } else {
                    "Own thread (recommended)".into()
                }
            }
        }
    }

    /// The files with `choice` written into this row's keys (and nothing else changed); the
    /// files as they are for "Game's setting" or the choice already in them.
    pub fn apply(self, files: &Files, choice: &str) -> Files {
        if choice == OWN || choice == self.current(files) {
            return files.clone();
        }
        let mut out = files.clone();
        let mut ini = |section: &str, key: &str, value: &str| {
            out.ini = set_key(&out.ini, section, key, value);
        };
        let first = |s: &str| s.split_whitespace().next().unwrap_or("").to_string();
        match self {
            Row::Renderer => ini(
                "nv-rs Display",
                "sRenderer",
                if choice.starts_with("Direct3D 9") {
                    "d3d9"
                } else {
                    "vulkan"
                },
            ),
            Row::DisplayMode => ini(
                "nv-rs Display",
                "bBorderless",
                if choice == "Window" { "0" } else { "1" },
            ),
            Row::VSync => ini(
                "Display",
                "iPresentInterval",
                if choice == "On" { "1" } else { "0" },
            ),
            Row::GameFixes => ini(
                "nv-rs General",
                "sPreset",
                if choice.starts_with("Vanilla") {
                    "vanilla"
                } else {
                    "fixes"
                },
            ),
            Row::FrameLimit | Row::MenuFrameLimit => {
                let key = if self == Row::FrameLimit {
                    "iFrameLimit"
                } else {
                    "iMenuFrameLimit"
                };
                let value = if choice == "No limit" {
                    "0".to_string()
                } else {
                    first(choice)
                };
                ini("nv-rs Display", key, &value)
            }
            Row::FrameRateCounter => ini(
                "nv-rs Display",
                "bShowFrameRate",
                if choice.starts_with("Shown") {
                    "1"
                } else {
                    "0"
                },
            ),
            Row::GameThreads => ini(
                "nv-rs Threads",
                "bSteppedThreads",
                if choice.starts_with("One at a time") {
                    "1"
                } else {
                    "0"
                },
            ),
            Row::DrawingThread => ini(
                "nv-rs Threads",
                "bRenderThread",
                if choice.starts_with("Main thread") {
                    "0"
                } else {
                    "1"
                },
            ),
            Row::Resolution => {
                if let Some((w, h)) = choice.split_once('x') {
                    out.prefs = set_key(&out.prefs, "Display", "iSize W", w);
                    out.prefs = set_key(&out.prefs, "Display", "iSize H", h);
                }
            }
            Row::AntiAliasing | Row::Anisotropy => {
                let key = if self == Row::AntiAliasing {
                    "iMultiSample"
                } else {
                    "iMaxAnisotropy"
                };
                let value = if choice == "Off" {
                    "0"
                } else {
                    choice.trim_end_matches('x')
                };
                out.prefs = set_key(&out.prefs, "Display", key, value);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(ini: &str, prefs: &str) -> Files {
        Files {
            ini: ini.into(),
            prefs: prefs.into(),
        }
    }

    /// With none of our keys, each row shows the host table's default, as the game plays it.
    #[test]
    fn missing_keys_show_the_tables_defaults() {
        let f = files("", "");
        let shown: Vec<String> = ROWS.iter().map(|r| r.current(&f)).collect();
        assert_eq!(
            shown,
            [
                "Vulkan (recommended)",
                "Borderless fullscreen",
                OWN,
                OWN,
                OWN,
                OWN,
                "Fixes (recommended)",
                "No limit",
                "120",
                "Hidden (F11 shows it)",
                "Side by side (recommended)",
                "Own thread (recommended)",
            ]
        );
    }

    /// A change writes only that row's keys, in place; every other byte stays.
    #[test]
    fn a_change_writes_its_own_keys_only() {
        let f = files(
            "[Display]\r\niPresentInterval=1\r\n[nv-rs Display]\r\n;note\r\nsRenderer=vulkan\r\niFrameLimit=0\r\n",
            "[Display]\r\niSize W=1920\r\niSize H=1080\r\niMultiSample=4\r\n",
        );
        let r = Row::Renderer.apply(&f, "Direct3D 9 (the original's)");
        assert_eq!(r.ini, f.ini.replace("sRenderer=vulkan", "sRenderer=d3d9"));
        assert_eq!(r.prefs, f.prefs);
        let r = Row::FrameLimit.apply(&f, "144");
        assert_eq!(r.ini, f.ini.replace("iFrameLimit=0", "iFrameLimit=144"));
        let r = Row::VSync.apply(&f, "Off");
        assert_eq!(
            r.ini,
            f.ini.replace("iPresentInterval=1", "iPresentInterval=0")
        );
        let r = Row::Resolution.apply(&f, "2560x1440");
        assert_eq!(
            r.prefs,
            f.prefs.replace("1920", "2560").replace("1080", "1440")
        );
        let r = Row::AntiAliasing.apply(&f, "Off");
        assert_eq!(r.prefs, f.prefs.replace("iMultiSample=4", "iMultiSample=0"));
        assert_eq!(Row::AntiAliasing.current(&r), "Off");
        // "Game's setting", and the value already there, write nothing.
        assert_eq!(Row::Anisotropy.apply(&f, OWN), f);
        assert_eq!(Row::Renderer.apply(&f, "Vulkan (recommended)"), f);
    }

    /// A value written by hand is kept as a choice of its own.
    #[test]
    fn a_hand_written_value_is_a_choice() {
        let f = files(
            "[nv-rs Display]\nsRenderer=VULKAN\niFrameLimit=75\n",
            "[Display]\niSize W=1366\niSize H=768\n",
        );
        assert!(Row::FrameLimit
            .choices(&f, (2560, 1440))
            .contains(&"75".to_string()));
        let res = Row::Resolution.choices(&f, (2560, 1440));
        assert_eq!(res[0], OWN);
        assert_eq!(res[1], "2560x1440");
        assert_eq!(res.last().map(String::as_str), Some("1366x768"));
        assert_eq!(Row::Renderer.current(&f), "Vulkan (recommended)");
    }
}
