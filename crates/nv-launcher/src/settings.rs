//! The Settings screen's rows, with no window in them: the twelve rows of the old Settings.cmd
//! (its distribution/Settings.ps1) by the same names, choices, defaults and keys (the census in
//! the launcher track's notes), read from and written to the profile's own `Fallout.ini` and
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
    // The Testing list (the lead's pick, DESIGN.md: its own list under Settings, so the main
    // list stays Settings.cmd's twelve): switches still being proven, each one key of ours.
    AmbientOcclusion,
    SunShadows,
    SunLightFromSky,
    SunShadowsCharacters,
    JipBuiltIn,
}

/// The Testing list's rows: the new lighting (Vulkan only, off by default) and JIP LN built in (on
/// by default with the fixes preset since the turn-on; "Off" here turns it off).
pub const TESTING: [Row; 5] = [
    Row::AmbientOcclusion,
    Row::SunShadows,
    Row::SunLightFromSky,
    Row::SunShadowsCharacters,
    Row::JipBuiltIn,
];

impl Row {
    /// A Testing row's own key: its section and name in the host's table.
    fn switch(self) -> Option<(&'static str, &'static str)> {
        Some(match self {
            Row::AmbientOcclusion => ("nv-rs Display", "bAmbientOcclusion"),
            Row::SunShadows => ("nv-rs Display", "bSunShadows"),
            Row::SunLightFromSky => ("nv-rs Display", "bSunLightFromSky"),
            Row::SunShadowsCharacters => ("nv-rs Display", "bSunShadowsCharacters"),
            Row::JipBuiltIn => ("nv-rs Fixes", "bJIPInstalled"),
            _ => return None,
        })
    }

    /// A Testing row's variable in the host's table (its settings.ini line's name).
    pub fn variable(self) -> Option<&'static str> {
        let (_, key) = self.switch()?;
        TABLE.lines().filter(|l| !l.starts_with('#')).find_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 4 && f[1] == key).then_some(f[2])
        })
    }
}

/// The Renderer row's choice for the game's own renderer (wgpu): on Windows Vulkan, recommended
/// over the original's Direct3D 9; on Linux Vulkan and on macOS Metal, the only renderer there.
pub const OWN_RENDERER: &str = if cfg!(windows) {
    "Vulkan (recommended)"
} else if cfg!(target_os = "macos") {
    "Metal"
} else {
    "Vulkan"
};

/// The original's renderer, a choice on Windows only (the real Direct3D 9 is a Windows API).
pub const DIRECT3D_9: &str = "Direct3D 9 (the original's)";

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
/// A Testing row's default as the game plays it: an `[nv-rs Fixes]` key follows the preset (off
/// under `sPreset=vanilla`, the host's rule for a fix row without its line), any other key the
/// host table's default.
fn testing_default(files: &Files, section: &str, key: &str) -> &'static str {
    if section == "nv-rs Fixes"
        && ours(files, "nv-rs General", "sPreset").eq_ignore_ascii_case("vanilla")
    {
        "0"
    } else {
        default_of(key)
    }
}

/// A Testing row's value: its key in the profile's Fallout.ini, else [`testing_default`].
fn testing_value(files: &Files, section: &str, key: &str) -> String {
    ini_value(&files.ini, section, key)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| testing_default(files, section, key))
        .trim()
        .to_string()
}

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
            Row::AmbientOcclusion => "Ambient occlusion (new lighting)",
            Row::SunShadows => "Sun shadows (new lighting)",
            Row::SunLightFromSky => "Sun light from the sun and moon",
            Row::SunShadowsCharacters => "Characters' sun shadows",
            Row::JipBuiltIn => "JIP LN built in",
        }
    }

    /// Its choices, the recommended one first where Settings.ps1 marks one; `desktop` is the
    /// main monitor's size for the resolution row. A value the list lacks (written by hand) is
    /// kept as a choice of its own, as Settings.ps1 keeps it.
    pub fn choices(self, files: &Files, desktop: (u32, u32)) -> Vec<String> {
        self.choices_on(files, desktop, cfg!(windows))
    }

    /// [`Row::choices`] as on Windows (`windows`) or not.
    pub fn choices_on(self, files: &Files, desktop: (u32, u32), windows: bool) -> Vec<String> {
        let fixed: &[&str] = match self {
            Row::Renderer if windows => &[OWN_RENDERER, DIRECT3D_9],
            // Off Windows the game's own renderer only: the real Direct3D 9 is a Windows API.
            Row::Renderer => &[OWN_RENDERER],
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
            // The Testing rows: the default the game plays with first.
            r => match r.switch() {
                Some((section, key)) if testing_default(files, section, key) == "1" => {
                    &["On", "Off"]
                }
                _ => &["Off", "On"],
            },
        };
        let mut list: Vec<String> = if self == Row::Resolution {
            // The monitor's own size first, when it is known (never 0x0).
            let native =
                (desktop.0 > 0 && desktop.1 > 0).then(|| format!("{}x{}", desktop.0, desktop.1));
            let mut l = vec![OWN.to_string()];
            for s in [
                native.as_deref().unwrap_or("3840x2160"),
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
        // A profile that holds Direct3D 9 off Windows shows it as its value, never as a choice.
        let refused = self == Row::Renderer && now == DIRECT3D_9 && !windows;
        if !list.contains(&now) && !refused {
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
                    DIRECT3D_9.into()
                } else {
                    OWN_RENDERER.into()
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
            r => match r.switch() {
                Some((section, key)) if testing_value(files, section, key) == "1" => "On".into(),
                _ => "Off".into(),
            },
        }
    }

    /// Whether `choice` may be saved for this row on this system: never Direct3D 9 off Windows, and
    /// never a resolution with a side of 0 (or one that is not a number).
    pub fn allows(self, choice: &str) -> bool {
        self.allows_on(choice, cfg!(windows))
    }

    /// [`Row::allows`] as on Windows (`windows`) or not.
    pub fn allows_on(self, choice: &str, windows: bool) -> bool {
        match self {
            Row::Renderer => windows || !choice.starts_with("Direct3D 9"),
            Row::Resolution => choice
                .split_once('x')
                .is_some_and(|(w, h)| [w, h].iter().all(|v| v.parse::<u32>().is_ok_and(|n| n > 0))),
            _ => true,
        }
    }

    /// The files with `choice` written into this row's keys (and nothing else changed); the
    /// files as they are for "Game's setting", the choice already in them, or one this system
    /// does not allow ([`Row::allows`]).
    pub fn apply(self, files: &Files, choice: &str) -> Files {
        if choice == OWN || choice == self.current(files) || !self.allows(choice) {
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
            Row::AmbientOcclusion
            | Row::SunShadows
            | Row::SunLightFromSky
            | Row::SunShadowsCharacters
            | Row::JipBuiltIn => {
                if let Some((section, key)) = self.switch() {
                    ini(section, key, if choice == "On" { "1" } else { "0" });
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
                OWN_RENDERER,
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
        let r = Row::Renderer.apply(&f, DIRECT3D_9);
        if cfg!(windows) {
            assert_eq!(r.ini, f.ini.replace("sRenderer=vulkan", "sRenderer=d3d9"));
        } else {
            assert_eq!(r, f, "no Direct3D 9 off Windows");
        }
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
        assert_eq!(Row::Renderer.apply(&f, OWN_RENDERER), f);
        // A resolution with a side of 0 is never written.
        assert_eq!(Row::Resolution.apply(&f, "0x0"), f);
        assert_eq!(Row::Resolution.apply(&f, "1280x0"), f);
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
        assert_eq!(Row::Renderer.current(&f), OWN_RENDERER);
        // No monitor size known: no 0x0 among the choices.
        let res = Row::Resolution.choices(&f, (0, 0));
        assert!(
            res.iter()
                .all(|r| !r.starts_with("0x") && !r.ends_with("x0")),
            "{res:?}"
        );
    }

    /// The Renderer row offers the original's Direct3D 9 on Windows only; elsewhere the game's own
    /// renderer by its system's name (Vulkan, Metal on macOS) is the one choice, and a profile that
    /// already holds d3d9 there shows it as its value, never offered, and choosing the one choice
    /// moves it off.
    #[test]
    fn direct3d_9_is_a_choice_on_windows_only() {
        let plain = files("[nv-rs Display]\r\nsRenderer=vulkan\r\n", "");
        let held = files("[nv-rs Display]\r\nsRenderer=d3d9\r\n", "");
        let size = (1920, 1080);
        assert_eq!(Row::Renderer.current(&held), DIRECT3D_9);
        // Windows' rule, and every other system's, checked on any system.
        assert_eq!(
            Row::Renderer.choices_on(&plain, size, true),
            [OWN_RENDERER, DIRECT3D_9]
        );
        assert!(Row::Renderer.allows_on(DIRECT3D_9, true));
        assert_eq!(
            Row::Renderer.choices_on(&plain, size, false),
            [OWN_RENDERER]
        );
        assert_eq!(Row::Renderer.choices_on(&held, size, false), [OWN_RENDERER]);
        assert!(!Row::Renderer.allows_on(DIRECT3D_9, false));
        assert!(Row::Renderer.allows_on(OWN_RENDERER, false));
        let moved = Row::Renderer.apply(&held, OWN_RENDERER);
        assert_eq!(Row::Renderer.current(&moved), OWN_RENDERER);
        if cfg!(windows) {
            assert_eq!(OWN_RENDERER, "Vulkan (recommended)");
        } else {
            assert_eq!(Row::Renderer.apply(&plain, DIRECT3D_9), plain);
            let name = if cfg!(target_os = "macos") {
                "Metal"
            } else {
                "Vulkan"
            };
            assert_eq!(OWN_RENDERER, name);
        }
    }

    /// The Testing rows: each one key of ours, shown as the host table's default when missing
    /// (the new lighting off, the sun light from the sky and characters' shadows on with it, JIP
    /// on with the fixes preset), the default offered first, and a change writes only that key.
    #[test]
    fn the_testing_rows_are_one_key_each() {
        let f = files("[nv-rs Display]\r\nsRenderer=vulkan\r\n", "");
        let shown: Vec<String> = TESTING.iter().map(|r| r.current(&f)).collect();
        assert_eq!(shown, ["Off", "Off", "On", "On", "On"]);
        assert_eq!(Row::AmbientOcclusion.choices(&f, (1, 1)), ["Off", "On"]);
        assert_eq!(Row::SunLightFromSky.choices(&f, (1, 1)), ["On", "Off"]);
        let r = Row::SunShadows.apply(&f, "On");
        assert_eq!(r.ini, set_key(&f.ini, "nv-rs Display", "bSunShadows", "1"));
        assert_eq!(Row::SunShadows.current(&r), "On");
        assert_eq!(Row::JipBuiltIn.choices(&f, (1, 1)), ["On", "Off"]);
        let r = Row::JipBuiltIn.apply(&f, "Off");
        assert_eq!(r.ini, set_key(&f.ini, "nv-rs Fixes", "bJIPInstalled", "0"));
        assert_eq!(Row::JipBuiltIn.current(&r), "Off");
        // Under the vanilla preset with no key, JIP is off as the game plays it: the row shows
        // Off, offers Off first, and On writes the key.
        let v = files("[nv-rs General]\r\nsPreset=vanilla\r\n", "");
        assert_eq!(Row::JipBuiltIn.current(&v), "Off");
        assert_eq!(Row::JipBuiltIn.choices(&v, (1, 1)), ["Off", "On"]);
        let r = Row::JipBuiltIn.apply(&v, "On");
        assert_eq!(r.ini, set_key(&v.ini, "nv-rs Fixes", "bJIPInstalled", "1"));
        assert_eq!(Row::JipBuiltIn.current(&r), "On");
        assert_eq!(Row::JipBuiltIn.apply(&v, "Off"), v, "already off");
        // The display rows keep the table's default under vanilla.
        assert_eq!(Row::SunLightFromSky.current(&v), "On");
        assert_eq!(r.prefs, f.prefs);
        assert_eq!(Row::SunLightFromSky.apply(&f, "On"), f);
        assert!(TESTING.iter().all(|r| !ROWS.contains(r)));
    }
}
