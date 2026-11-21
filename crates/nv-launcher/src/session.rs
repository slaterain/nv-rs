//! The terminal window and the work behind it talk through this. Play's steps run on a thread of
//! their own (they wait for the graphics and sound tools, then for the game), and every question
//! they used to ask in a message box of Windows' (Play.exe) or in the terminal play.sh was started
//! from goes to the launcher's window instead: a text to read with entries to choose from, a folder
//! to type or drop onto the window, the step the start is at. The window answers on the channel
//! each question carries.
//!
//! With no window (scripted tests set `NV_RS_NO_WINDOWS=1`, as they did for Play.exe) every
//! question gets the answer Play.exe's message boxes gave without a window: the first entry (OK, or
//! Yes), and no folder.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;

/// What the work tells or asks the window.
// Status, StatusDone, Message and Folder come from Play.exe's steps (Windows); on Linux and macOS
// play.sh says and asks everything in its own lines (Output).
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Event {
    /// The step the start is at, for the waiting screen ("Reading your graphics card...").
    Status(String),
    /// No step is going on any more.
    StatusDone,
    /// The game's own window is there: the launcher's window steps aside until the game ends.
    GameShown,
    /// A text to read and the entries to choose from; the reply is the chosen entry's index.
    Message {
        tone: Tone,
        text: String,
        entries: Vec<String>,
        reply: Sender<usize>,
    },
    /// A folder to type at the prompt or drop onto the window; the reply is `None` for Back.
    Folder {
        text: String,
        reply: Sender<Option<PathBuf>>,
    },
    /// Text a program the launcher runs writes (play.sh's lines and questions), as it comes.
    Output(String),
    /// The work has ended with this exit code.
    Ended(i32),
    /// The graphics card the game will draw with, for the start screen.
    Card(crate::app::Card),
    /// The clipboard's line for the paste asked as `id` ([`crate::paste::read_later`]).
    Pasted { id: u64, text: Option<String> },
    /// The terminal's sounds read from the player's game (`crate::sound::load`), and what was
    /// not found, for the log.
    Sounds {
        banks: Vec<(crate::sound_files::Cue, crate::sound::Bank)>,
        said: Vec<String>,
    },
}

/// What a message is, for Windows' message boxes when there is no window (their icon).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Tone {
    Error,
    Warning,
    Info,
    Question,
}

/// The window's end of the channel: set once, when the window is made.
static WINDOW: OnceLock<Box<dyn Fn(Event) -> bool + Send + Sync>> = OnceLock::new();

/// The stop code Play ends with once the question it asks now is answered (none: `i32::MIN`), so
/// a window closed during that question ends the launcher with Play.exe's code.
static STOP_CODE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(i32::MIN);

/// Play is about to ask its last question before it ends with `code`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn set_stop_code(code: i32) {
    STOP_CODE.store(code, std::sync::atomic::Ordering::Relaxed);
}

/// The stop code Play has named, if any.
pub fn stop_code() -> Option<i32> {
    let code = STOP_CODE.load(std::sync::atomic::Ordering::Relaxed);
    (code != i32::MIN).then_some(code)
}

/// From now on events go to `deliver` (the window's event loop; false when it has gone).
pub fn connect(deliver: impl Fn(Event) -> bool + Send + Sync + 'static) {
    let _ = WINDOW.set(Box::new(deliver));
}

/// Sends `event` to the window; false when there is none (or it has gone).
pub fn send(event: Event) -> bool {
    WINDOW.get().is_some_and(|deliver| deliver(event))
}

/// Shows `text` with `entries` and waits for the player's choice: the entry's index. Without a
/// window, or when it goes before the player chose, 0.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn message(text: &str, entries: &[&str]) -> usize {
    message_as(Tone::Question, text, entries)
}

/// [`message`] with its tone.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn message_as(tone: Tone, text: &str, entries: &[&str]) -> usize {
    let (reply, answer) = channel();
    let event = Event::Message {
        tone,
        text: text.to_string(),
        entries: entries.iter().map(|e| e.to_string()).collect(),
        reply,
    };
    if !send(event) {
        return 0;
    }
    answer.recv().unwrap_or(0)
}

/// Asks for a folder and waits: the folder typed or dropped, `None` for Back (and without a
/// window).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn folder(text: &str) -> Option<PathBuf> {
    let (reply, answer) = channel();
    if !send(Event::Folder {
        text: text.to_string(),
        reply,
    }) {
        return None;
    }
    answer.recv().ok().flatten()
}

/// A folder as typed, pasted or dropped: spaces around it and quotes around it taken off, a
/// `file://` link read (its `%XX` undone), as play.sh's `typed_folder` reads one.
pub fn typed_folder(text: &str) -> PathBuf {
    let t = text.trim();
    let t = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(t);
    if let Some(rest) = t.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        let b = rest.as_bytes();
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if b[i] == b'%' {
                if let (Some(h), Some(l)) = (
                    b.get(i + 1).copied().and_then(hex),
                    b.get(i + 2).copied().and_then(hex),
                ) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                    continue;
                }
            }
            out.push(b[i]);
            i += 1;
        }
        return PathBuf::from(String::from_utf8_lossy(&out).into_owned());
    }
    PathBuf::from(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder pasted with quotes, or dropped as a link, is read as the folder.
    #[test]
    fn typed_folders_lose_quotes_and_links() {
        assert_eq!(
            typed_folder("  \"C:\\Games\\Fallout New Vegas\" "),
            PathBuf::from("C:\\Games\\Fallout New Vegas")
        );
        assert_eq!(typed_folder("'/a b/c'"), PathBuf::from("/a b/c"));
        assert_eq!(
            typed_folder("file:///home/me/Fallout%20New%20Vegas"),
            PathBuf::from("/home/me/Fallout New Vegas")
        );
        assert_eq!(typed_folder("file://localhost/x%2"), PathBuf::from("/x%2"));
        assert_eq!(typed_folder("plain"), PathBuf::from("plain"));
    }

    /// The stop code Play names before its last question is kept until a new run clears it.
    #[test]
    fn the_stop_code_is_kept_until_cleared() {
        set_stop_code(i32::MIN);
        assert_eq!(stop_code(), None);
        set_stop_code(7);
        assert_eq!(stop_code(), Some(7));
        set_stop_code(i32::MIN);
        assert_eq!(stop_code(), None);
    }

    /// Without a window every question gets the first entry and no folder, as Play.exe's message
    /// boxes answered with no window.
    #[test]
    fn without_a_window_the_first_entry_is_the_answer() {
        assert_eq!(message("x", &["Yes", "No"]), 0);
        assert_eq!(folder("x"), None);
        assert!(!send(Event::StatusDone));
    }
}
