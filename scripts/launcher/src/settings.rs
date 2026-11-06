//! `userdata\settings.ini`, which Settings.cmd writes and the launcher reads: `Name=Value` lines.
//! Names the launcher doesn't know are kept by Settings.cmd and ignored here, so a new setting can
//! join without a new launcher. A name starting with `NV_` goes to the game's environment as it is,
//! which is how a switch of the game's own (a fixes preset, say) can join Settings later.
use std::collections::BTreeMap;

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
                map.insert(name.trim().to_string(), value.trim().to_string());
            }
        }
        Settings(map)
    }

    fn get(&self, name: &str) -> &str {
        self.0.get(name).map_or("", String::as_str)
    }

    /// Borderless window: on unless the file says otherwise (the way the game has been played most).
    pub fn borderless(&self) -> bool {
        self.get("Borderless") != "0"
    }

    /// The game's environment switches from the file (`NV_...=...`).
    pub fn environment(&self) -> Vec<(&str, &str)> {
        self.0
            .iter()
            .filter(|(k, _)| k.starts_with("NV_"))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// The `FalloutPrefs.ini` lines these settings set, by the game's own names; `desktop` is the
    /// primary monitor's size, used when no resolution was chosen and the prefs have none.
    pub fn prefs_lines(&self) -> Vec<(String, String)> {
        let mut set = Vec::new();
        let times = |v: &str| match v {
            "Off" => Some(0u32),
            v => v.strip_suffix('x').and_then(|n| n.parse().ok()),
        };
        if self.borderless() {
            set.push(("bFull Screen".into(), "0".into()));
        }
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
        match self.get("VSync") {
            "On" => set.push(("iPresentInterval".into(), "1".into())),
            "Off" => set.push(("iPresentInterval".into(), "0".into())),
            _ => {}
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
    fn the_file_is_read_and_unknown_names_do_no_harm() {
        let s = Settings::parse(
            "# a comment\nBorderless=1\nResolution=2560x1440\nAntiAliasing=8x\nAnisotropy=Off\nVSync=Game launcher's\nSomethingNew=7\nNV_FIXES=vnv\n",
        );
        assert!(s.borderless());
        assert_eq!(
            s.prefs_lines(),
            lines(&[
                ("bFull Screen", "0"),
                ("iSize W", "2560"),
                ("iSize H", "1440"),
                ("iMultiSample", "8"),
                ("iMaxAnisotropy", "0"),
            ])
        );
        assert_eq!(s.environment(), [("NV_FIXES", "vnv")]);
        assert!(Settings::parse("").borderless());
        assert!(!Settings::parse("Borderless=0").borderless());
        assert!(Settings::parse("Borderless=0\nResolution=Game launcher's")
            .prefs_lines()
            .is_empty());
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
}
