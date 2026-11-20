//! The game's terminal intro, shown once, on the launcher's first start with the game's look (the
//! lead's pick, DESIGN.md: "on the very first start only, skipped by any key"), typed as
//! `ComputersMenu::DisplayIntro` 00758ad0 types it into its intro text (the menu's tile at +0x4c):
//!
//! 1. `sHackingIntro01` (the welcome line) at 1000 /
//!    `iComputersDisplayRateMenus` milliseconds a character (150: 6 ms);
//! 2. `"%s%s"` of the prompt `"> "` (01072118) and `sComputersLogon` (the logon line) at 1000 /
//!    `iHackingInputRate` (20: 50 ms), as if typed;
//! 3. `sHackingHeader2` (the password line) at the menus' rate;
//! 4. the prompt and the password as stars, at the input rate: `BGSTerminal::GetHackMiniGameValue`
//!    00501270 stars, `(random & 1) + lock level * 2 + 4` (at most 12). The launcher's terminal has
//!    no lock of its own; it types the count of the game's lowest level (0): 4 or 5.
//!
//! Then `DoIdle` 00758490 waits 2000 ms once the text is in and `FinishIntro` 00758d30 clears it
//! and types the terminal's own screen (the start screen). The division is the game's, in whole
//! milliseconds; a rate of 0 is 1000 ms, as there.

/// `iComputersDisplayRateMenus`' default (00f6db90: 0x96) and `iHackingInputRate`'s (00f6e220:
/// 0x14), characters a second. No GMST in FalloutNV.esm sets them.
pub const DISPLAY_RATE_MENUS: u32 = 150;
pub const HACKING_INPUT_RATE: u32 = 20;
/// `DoIdle`'s wait between the typed intro and `FinishIntro`, milliseconds.
pub const END_PAUSE_MS: u64 = 2000;

/// The milliseconds a character as `DisplayIntro` works them out from a rate.
pub fn ms_per_char(rate: u32) -> u64 {
    1000u32.checked_div(rate).map_or(1000, u64::from)
}

/// The intro's four lines, each with its milliseconds a character.
pub struct Intro {
    pub lines: Vec<(String, u64)>,
}

impl Intro {
    /// From the game's strings (`welcome`, `logon`, `password_header`); `odd` is the random bit of
    /// the password's length.
    pub fn new(welcome: &str, logon: &str, password_header: &str, odd: bool) -> Intro {
        let menus = ms_per_char(DISPLAY_RATE_MENUS);
        let input = ms_per_char(HACKING_INPUT_RATE);
        let stars = (usize::from(odd) + 4).min(12);
        Intro {
            lines: vec![
                (welcome.to_string(), menus),
                (format!("> {logon}"), input),
                (password_header.to_string(), menus),
                (format!("> {}", "*".repeat(stars)), input),
            ],
        }
    }

    /// The milliseconds until the last character is in.
    pub fn typing_ms(&self) -> u64 {
        self.lines
            .iter()
            .map(|(l, ms)| l.chars().count() as u64 * ms)
            .sum()
    }

    /// The milliseconds until the start screen follows (`FinishIntro`).
    pub fn total_ms(&self) -> u64 {
        self.typing_ms() + END_PAUSE_MS
    }

    /// The lines as typed `elapsed` milliseconds after the start: one after the other, each at its
    /// own rate.
    pub fn typed(&self, elapsed: u64) -> Vec<String> {
        let mut left = elapsed;
        let mut out = Vec::new();
        for (line, ms) in &self.lines {
            let n = line.chars().count() as u64;
            let shown = (left / ms).min(n);
            out.push(line.chars().take(shown as usize).collect());
            if shown < n {
                break;
            }
            left -= n * ms;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intro() -> Intro {
        Intro::new(
            "HELLO FROM A MADE UP TERMINAL FOR TESTING",
            "SIGN IN NOW",
            "TYPE YOUR WORD NOW",
            false,
        )
    }

    /// The rates in whole milliseconds as the game divides them: 6 for menus' 150, 50 for 20.
    #[test]
    fn the_rates_are_the_games() {
        assert_eq!(ms_per_char(150), 6);
        assert_eq!(ms_per_char(20), 50);
        assert_eq!(ms_per_char(0), 1000);
        let i = intro();
        assert_eq!(i.lines[1], ("> SIGN IN NOW".to_string(), 50));
        assert_eq!(i.lines[3].0, "> ****");
        assert_eq!(Intro::new("a", "b", "c", true).lines[3].0, "> *****");
        // 41 + 18 characters at 6 ms, 13 + 6 at 50 ms.
        assert_eq!(i.typing_ms(), 59 * 6 + 19 * 50);
        assert_eq!(i.total_ms(), i.typing_ms() + 2000);
    }

    /// The lines come one after the other, each at its own rate.
    #[test]
    fn the_lines_are_typed_in_order() {
        let i = intro();
        assert_eq!(i.typed(0), [""]);
        assert_eq!(i.typed(6 * 5), ["HELLO"]);
        let first = 41 * 6;
        assert_eq!(
            i.typed(first)[..2],
            ["HELLO FROM A MADE UP TERMINAL FOR TESTING", ""]
        );
        assert_eq!(i.typed(first + 50 * 3)[1], "> S");
        let all = i.typed(i.typing_ms());
        assert_eq!(all.len(), 4);
        assert_eq!(all[3], "> ****");
        assert_eq!(i.typed(u64::MAX / 2), all);
    }
}
