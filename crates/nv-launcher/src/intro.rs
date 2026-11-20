//! The game's terminal intro, shown once, on the launcher's first start with the game's look (the
//! lead's pick, DESIGN.md: "on the very first start only"), typed as
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
//! The first line types at once; after each line ends the next waits 500 ms (the intro state of
//! `AnimatingTextManager::Update`, `crate::pacing`): three waits between the four lines. The lines
//! type on the game's frame clock. Then `DoIdle` 00758490, on the
//! frame after the last line is done, waits 2000 ms, and `FinishIntro` 00758d30 clears the text
//! and types the terminal's own screen (the start screen). The division is the game's, in whole
//! milliseconds; a rate of 0 is 1000 ms, as there.
//!
//! The game finishes its intro at once on a click on the terminal's background (`DoClick`
//! 00757f70, tile 13, intro state below 2). The launcher also lets Esc or Enter finish it: a
//! practical extra, not the game's (a keyboard-only player has no background to click).

use crate::pacing::{self, Line};

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

/// The intro's four lines, each with its milliseconds a character, and the frames it types on
/// (milliseconds apart: [`pacing::frame_ms`]).
pub struct Intro {
    pub lines: Vec<(String, u64)>,
    pub frame: f64,
}

impl Intro {
    /// From the game's strings (`welcome`, `logon`, `password_header`); `odd` is the random bit of
    /// the password's length; `frame` the frame period.
    pub fn new(welcome: &str, logon: &str, password_header: &str, odd: bool, frame: f64) -> Intro {
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
            frame,
        }
    }

    /// The lines as the text manager's queue: intro lines, each at its own rate; the prompt
    /// lines (`> ` and the logon, `> ` and the stars) typed as if by a player.
    pub fn queue(&self) -> Vec<Line> {
        self.lines
            .iter()
            .map(|(l, ms)| Line {
                chars: l.chars().count(),
                ms: *ms,
                intro: true,
                keyed: l.starts_with('>'),
            })
            .collect()
    }

    /// The milliseconds until the last character is in.
    pub fn typing_ms(&self) -> u64 {
        pacing::typed(&self.queue(), self.frame, f64::MAX)
            .1
            .unwrap_or(0.0) as u64
    }

    /// The milliseconds until the start screen follows (`FinishIntro`): the frame after the
    /// last character finds the queue empty and `DoIdle` waits 2000 ms from there.
    pub fn total_ms(&self) -> u64 {
        (self.typing_ms() as f64 + self.frame).floor() as u64 + END_PAUSE_MS
    }

    /// The lines as typed `elapsed` milliseconds after the start.
    pub fn typed(&self, elapsed: u64) -> Vec<String> {
        let (shown, _) = pacing::typed(&self.queue(), self.frame, elapsed as f64);
        let mut out = Vec::new();
        for ((line, _), n) in self.lines.iter().zip(shown) {
            out.push(line.chars().take(n).collect());
            if n < line.chars().count() {
                break;
            }
        }
        out
    }
}

/// Whether a press finishes the intro: Esc or Enter, not a held key's repeat (the launcher's
/// extra; any other key does nothing, as in the game). A click finishes it as in the game, in
/// the window's own handler.
pub fn key_finishes(press: crate::app::Press, repeat: bool) -> bool {
    use crate::app::Press;
    !repeat && matches!(press, Press::Escape | Press::Enter)
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
            pacing::frame_ms(120),
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
        assert_eq!(
            Intro::new("a", "b", "c", true, pacing::frame_ms(120)).lines[3].0,
            "> *****"
        );
    }

    /// The first line types at once, the others each after a 500 ms wait; the lines come one
    /// after the other, and the start screen follows 2000 ms (and a frame) after the last
    /// character.
    #[test]
    fn the_lines_type_in_order_with_waits_between() {
        let i = intro();
        // The first frame types the first character, then one a frame (33 ms is the fifth frame).
        assert_eq!(i.typed(0), ["H"]);
        assert_eq!(i.typed(33), ["HELLO"]);
        let all = i.typed(i.typing_ms());
        assert_eq!(all.len(), 4);
        assert_eq!(all[3], "> ****");
        assert_eq!(i.typed(u64::MAX / 2), all);
        // Three waits of 500 ms, and the lines' characters: more than these, less than four waits.
        let chars = 41 * 6 + 13 * 50 + 18 * 6 + 6 * 50;
        assert!(i.typing_ms() > 3 * 500 + chars, "{}", i.typing_ms());
        assert!(i.typing_ms() < 4 * 500 + chars, "{}", i.typing_ms());
        assert_eq!(i.total_ms(), (i.typing_ms() as f64 + i.frame) as u64 + 2000);
    }

    /// Esc or Enter finishes the intro (the launcher's extra); other keys, and a held key's
    /// repeats, do not.
    #[test]
    fn only_esc_or_enter_finish_the_intro() {
        use crate::app::Press;
        assert!(key_finishes(Press::Escape, false));
        assert!(key_finishes(Press::Enter, false));
        assert!(!key_finishes(Press::Escape, true));
        assert!(!key_finishes(Press::Enter, true));
        for p in [
            Press::Other,
            Press::Up,
            Press::Click,
            Press::Modifier,
            Press::Left,
        ] {
            assert!(!key_finishes(p, false), "{p:?}");
        }
    }
}
