//! How fast the terminal's text types, on the game's own clock. The game types a terminal's text
//! as a queue of lines (`ComputersMenu::AnimatingTextManager`), updated once a frame
//! (`AnimatingTextManager::Update` 007598d0 from `ComputersMenu::DoIdle` 00758470):
//!
//! - A line types (`AnimatingText::Update` 006ffea0) only on a frame at or after its due time:
//!   `(now - due) / ms + 1` characters (whole milliseconds), at most what is left, and its due
//!   time becomes `now + ms`. Its first update types one character. So what a player sees
//!   depends on the frame rate: two characters a frame at 60 frames a second, one at 120.
//! - When a line is done, the next one starts on the next frame's update (the line's done flag
//!   is read at the top of the update), and types its first character there.
//! - In the intro only (the menu's intro state 0, +0xb0), a line that starts sets a wait of 500
//!   ms (011d93d4): nothing types until it has passed.
//!
//! Every screen after the intro types at 1000 / `iComputersDisplayRateMenus` milliseconds a
//! character (the terminal display function 007586e0: 6 ms for the default 150); the intro's lines at their own rates
//! (`crate::intro`). The frames are the game's in menus: the player's "Frame rate limit in
//! menus" (`[nv-rs Display] iMenuFrameLimit`, default 120). With no limit the launcher counts
//! 120 frames a second: the game would then follow the monitor, which the launcher does not
//! guess at (our choice, not the game's).

/// One line of the queue: its characters, its milliseconds a character, and whether it is an
/// intro line (500 ms before it types).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub chars: usize,
    pub ms: u64,
    pub intro: bool,
}

/// The intro's wait before each of its lines types (011d93d4 = now + 500).
pub const INTRO_LINE_WAIT_MS: f64 = 500.0;

/// The frames a second the launcher counts for "No limit" (see the module's text).
pub const NO_LIMIT_FPS: u32 = 120;

/// The frame period, milliseconds, for the menu frame limit `fps` (0: no limit).
pub fn frame_ms(fps: u32) -> f64 {
    1000.0 / f64::from(if fps == 0 { NO_LIMIT_FPS } else { fps })
}

/// What the queue shows `t` milliseconds after it started, frames `frame` milliseconds apart (the
/// first frame at 0): the characters typed of each line, and the time (ms) the last line was
/// done, if it is.
pub fn typed(lines: &[Line], frame: f64, t: f64) -> (Vec<usize>, Option<f64>) {
    let mut out = vec![0usize; lines.len()];
    // The line typing now, the frame its due time comes (None: its first update), the wait.
    let mut current: Option<usize> = None;
    let mut done = false;
    let mut due: Option<f64> = None;
    let mut wait = f64::NEG_INFINITY;
    let mut done_at = 0.0;
    let mut k = 0u64;
    loop {
        // The game's clock counts whole milliseconds.
        let now = (k as f64 * frame).floor();
        if now > t {
            return (out, None);
        }
        k += 1;
        if now <= wait {
            continue;
        }
        // The done line gives way to the next one, which starts on this update.
        if current.is_none() || done {
            let next = current.map_or(0, |c| c + 1);
            if next >= lines.len() {
                return (out, Some(done_at));
            }
            current = Some(next);
            done = false;
            due = None;
            if lines[next].intro {
                wait = now + INTRO_LINE_WAIT_MS;
                continue;
            }
        }
        let i = current.unwrap_or(0);
        let line = lines[i];
        let count = match due {
            None => 1,
            Some(d) if now < d => continue,
            Some(d) => ((now - d) / line.ms.max(1) as f64).floor() as usize + 1,
        };
        out[i] = (out[i] + count).min(line.chars);
        if out[i] >= line.chars {
            done = true;
            done_at = now;
        }
        due = Some(now + line.ms as f64);
    }
}

/// The characters typed in all, the same as [`typed`]'s, summed.
#[cfg(test)]
pub fn typed_chars(lines: &[Line], frame: f64, t: f64) -> usize {
    typed(lines, frame, t).0.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(chars: usize, ms: u64) -> Line {
        Line {
            chars,
            ms,
            intro: false,
        }
    }

    /// At 120 frames a second (8.33 ms) a 6 ms line types one character a frame; at 60 (16.7 ms)
    /// two; at 200 (5 ms) one every other frame. The first character is there at once.
    #[test]
    fn the_typing_follows_the_frame_clock() {
        let l = [line(100, 6)];
        let at = |fps, frames: f64| typed_chars(&l, frame_ms(fps), frames * frame_ms(fps));
        assert_eq!(at(120, 0.0), 1);
        assert_eq!(at(120, 9.0), 10);
        assert_eq!(at(60, 0.0), 1);
        assert_eq!(at(60, 9.0), 19);
        assert_eq!(at(200, 0.0), 1);
        assert_eq!(at(200, 10.0), 6);
        // "No limit" counts as 120.
        assert_eq!(frame_ms(0), frame_ms(120));
    }

    /// A new line starts one frame after the last one's last character, with its first one.
    #[test]
    fn a_line_starts_the_frame_after_the_last_ends() {
        let l = [line(3, 6), line(2, 6)];
        let f = frame_ms(120);
        assert_eq!(typed(&l, f, 2.0 * f).0, [3, 0]);
        assert_eq!(typed(&l, f, 3.0 * f).0, [3, 1]);
        let (shown, end) = typed(&l, f, 10.0 * f);
        assert_eq!(shown, [3, 2]);
        // Done on the fifth frame (frame 4); the frame after it finds no line left.
        assert_eq!(end, Some((4.0 * f).floor()));
        assert_eq!(typed(&l, f, 4.5 * f).1, None);
    }

    /// An intro line waits 500 ms before it types; the next one waits again from its own start.
    #[test]
    fn the_intro_waits_before_each_line() {
        let intro = |chars| Line {
            chars,
            ms: 6,
            intro: true,
        };
        let l = [intro(2), intro(2)];
        let f = frame_ms(120);
        assert_eq!(typed_chars(&l, f, 499.0), 0);
        // The first frame after 500 ms (frame 61, 508 ms) types the first character.
        assert_eq!(typed(&l, f, 61.0 * f).0, [1, 0]);
        assert_eq!(typed(&l, f, 62.0 * f).0, [2, 0]);
        // The second line starts on frame 63 and types from frame 63 + 61.
        assert_eq!(typed(&l, f, 123.0 * f).0, [2, 0]);
        assert_eq!(typed(&l, f, 124.0 * f).0, [2, 1]);
    }

    /// A line of the intro's input rate (50 ms) types one character each 50 ms and a frame.
    #[test]
    fn a_slow_line_waits_for_its_due_frame() {
        let l = [line(5, 50)];
        let f = frame_ms(120);
        // 0: 1; due 50 -> frame 6 (50 ms): 2; due 100 -> frame 12: 3.
        assert_eq!(typed_chars(&l, f, 5.0 * f), 1);
        assert_eq!(typed_chars(&l, f, 6.0 * f), 2);
        assert_eq!(typed_chars(&l, f, 12.0 * f), 3);
    }

    /// Nothing to type: done at once.
    #[test]
    fn an_empty_queue_is_done() {
        assert_eq!(typed(&[], frame_ms(120), 0.0), (vec![], Some(0.0)));
    }
}
