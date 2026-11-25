//! Whether crash reports may be sent: the player's answer, asked once at the first start (send
//! them automatically, ask each time, never), kept in the launcher's own file (`launcher.ini`,
//! `[Launcher] sCrashReports`) and changed in Settings. Until the player answers, nothing is ever
//! sent: an unanswered launcher keeps its reports on this PC only, as "never" does.

use std::path::Path;

/// The launcher's own file's key for the answer.
const KEY: &str = "sCrashReports";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Consent {
    /// Not asked yet, or the question was left with "Not now": nothing is sent.
    Unanswered,
    /// Each report is sent as soon as it is made.
    Send,
    /// Each report is shown first, with Send and Don't send.
    Ask,
    /// Reports stay on this PC.
    Never,
}

/// The three answers, in the order the question shows them (the tests' list).
#[cfg(test)]
pub const ANSWERS: [Consent; 3] = [Consent::Send, Consent::Ask, Consent::Never];

impl Consent {
    /// The answer a value of the launcher's file names (any case); anything else is no answer.
    pub fn from_value(value: Option<&str>) -> Consent {
        match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            Some("send") => Consent::Send,
            Some("ask") => Consent::Ask,
            Some("never") => Consent::Never,
            _ => Consent::Unanswered,
        }
    }

    /// The value written for this answer.
    pub fn value(self) -> &'static str {
        match self {
            Consent::Unanswered => "",
            Consent::Send => "send",
            Consent::Ask => "ask",
            Consent::Never => "never",
        }
    }

    /// The Settings row's words for it.
    pub fn label(self) -> &'static str {
        match self {
            Consent::Unanswered => "Not answered",
            Consent::Send => "Send automatically",
            Consent::Ask => "Ask each time",
            Consent::Never => "Never send",
        }
    }
}

/// The answer kept in nv-rs's own folder `own`.
pub fn read(own: &Path) -> Consent {
    let text: String = std::fs::read(crate::place::launcher_ini(own))
        .unwrap_or_default()
        .iter()
        .map(|&b| b as char)
        .collect();
    Consent::from_value(game_profile::settings::ini_value(&text, "Launcher", KEY))
}

/// `answer` kept in nv-rs's own folder `own`: at once where `now`, else held until Play's first
/// write (Windows: `crate::place::write_held_keys`).
pub fn keep(own: &Path, answer: Consent, now: bool) -> std::io::Result<()> {
    crate::place::set_key_when_allowed(own, now, KEY, answer.value())
}

/// `answer` kept in nv-rs's own folder `own` at once (the launcher's file's other lines stay):
/// the tests' shorthand for [`keep`].
#[cfg(test)]
pub fn write(own: &Path, answer: Consent) -> std::io::Result<()> {
    crate::place::set_launcher_key(own, KEY, answer.value())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_read_in_any_case_and_anything_else_is_no_answer() {
        assert_eq!(Consent::from_value(Some("send")), Consent::Send);
        assert_eq!(Consent::from_value(Some(" ASK ")), Consent::Ask);
        assert_eq!(Consent::from_value(Some("Never")), Consent::Never);
        assert_eq!(Consent::from_value(Some("yes")), Consent::Unanswered);
        assert_eq!(Consent::from_value(Some("")), Consent::Unanswered);
        assert_eq!(Consent::from_value(None), Consent::Unanswered);
        for a in ANSWERS {
            assert_eq!(Consent::from_value(Some(a.value())), a);
        }
    }

    /// The answer is kept in the launcher's own file, its other lines stay; none there is no
    /// answer.
    #[test]
    fn the_answer_is_kept_in_the_launchers_file() {
        let own = std::env::temp_dir().join(format!("nv-launcher-consent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&own);
        assert_eq!(read(&own), Consent::Unanswered);
        std::fs::create_dir_all(&own).unwrap();
        std::fs::write(
            crate::place::launcher_ini(&own),
            "[Launcher]\r\nbSounds=0\r\n",
        )
        .unwrap();
        assert_eq!(read(&own), Consent::Unanswered);
        write(&own, Consent::Ask).unwrap();
        assert_eq!(read(&own), Consent::Ask);
        write(&own, Consent::Never).unwrap();
        assert_eq!(read(&own), Consent::Never);
        assert!(!crate::place::sounds_on(&own));
        // Held (Windows before Play's first write): nothing on disk until the held keys go.
        keep(&own, Consent::Send, false).unwrap();
        assert_eq!(read(&own), Consent::Never);
        crate::place::write_held_keys(&own).unwrap();
        assert_eq!(read(&own), Consent::Send);
        keep(&own, Consent::Ask, true).unwrap();
        assert_eq!(read(&own), Consent::Ask);
        // A first answer held, then a change written at once (Settings): the change stays, the
        // held answer is dropped (Play's first write later must not bring it back).
        keep(&own, Consent::Send, false).unwrap();
        keep(&own, Consent::Never, true).unwrap();
        crate::place::write_held_keys(&own).unwrap();
        assert_eq!(read(&own), Consent::Never);
        let _ = std::fs::remove_dir_all(&own);
    }
}
