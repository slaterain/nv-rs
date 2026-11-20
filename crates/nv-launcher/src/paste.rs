//! Paste at the prompt: Ctrl+V (Cmd+V on macOS) puts the clipboard's text where typing goes (the
//! game's folder, an answer to play.sh): its first line with anything on it ([`line_of`]). An
//! empty clipboard, one holding no text (a picture), or one that cannot be opened at all pastes
//! nothing; it never stops the launcher, and it is read on a thread of its own ([`read_later`]),
//! so a clipboard owner that is slow to answer never holds the window. The system's clipboard is
//! arboard's (Windows, macOS, X11 and Wayland), behind [`Clipboard`] so the rules are tested
//! without one.

/// At most this many characters are pasted: a clipboard holding a huge single line must not make
/// the launcher draw (or send play.sh) all of it.
pub const MAX_CHARS: usize = 4096;

/// Where a paste's text comes from.
pub trait Clipboard {
    /// The clipboard's text, `None` when it has none (or cannot be read).
    fn text(&mut self) -> Option<String>;
}

/// The system's clipboard, opened for one paste.
pub struct System;

impl Clipboard for System {
    fn text(&mut self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }
}

/// What a paste adds at the prompt, from `clipboard` ([`line_of`]).
pub fn pasted(clipboard: &mut dyn Clipboard) -> Option<String> {
    line_of(&clipboard.text()?)
}

/// Invisible format characters some programs add around a path (Explorer's "Copy as path", some
/// editors: direction marks and embeddings, zero-width spaces and joiners, the byte order mark):
/// a path with one fails without a word, so they never get in.
fn invisible(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}')
}

/// The pasted line of `text`: its first line with anything on it, without control or invisible
/// format characters, spaces around it taken off, and the quotes around a whole path taken off
/// ("C:\Games\Some Game" pastes as C:\Games\Some Game); at most [`MAX_CHARS`] characters; `None`
/// when nothing is left.
pub fn line_of(text: &str) -> Option<String> {
    let line = text
        .lines()
        .map(|l| {
            l.chars()
                .filter(|c| !c.is_control() && !invisible(*c))
                .take(MAX_CHARS + 2)
                .collect::<String>()
        })
        .find(|l| !l.trim().is_empty())?;
    let line = line.trim();
    let line = match line.strip_prefix('"').and_then(|l| l.strip_suffix('"')) {
        Some(inner) if !inner.contains('"') => inner.trim(),
        _ => line,
    };
    let line: String = line.chars().take(MAX_CHARS).collect();
    (!line.is_empty()).then_some(line)
}

/// Whether a key pressed with Ctrl (`control`) or the Command key (`command`) is this system's
/// paste: Cmd+V on macOS, Ctrl+V elsewhere, the V by its character (`key`, for layouts like
/// Dvorak that put V elsewhere) or by its place on the keyboard (`physical_v`, for layouts whose
/// letters are not Latin). One key press is one paste, whichever matched.
pub fn is_paste(key: &str, physical_v: bool, control: bool, command: bool) -> bool {
    let held = if cfg!(target_os = "macos") {
        command
    } else {
        control
    };
    held && (key.eq_ignore_ascii_case("v") || physical_v)
}

/// The clipboard read on a thread of its own, its line ([`pasted`]) handed to `answer` there.
pub fn read_later(answer: impl FnOnce(Option<String>) + Send + 'static) {
    std::thread::spawn(move || answer(pasted(&mut System)));
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Option<&'static str>);

    impl Clipboard for Fake {
        fn text(&mut self) -> Option<String> {
            self.0.map(str::to_string)
        }
    }

    /// The first line with something on it, trimmed; nothing from an empty clipboard, one without
    /// text, or one of only spaces and line breaks; no control character gets in.
    #[test]
    fn a_paste_is_the_first_line_trimmed() {
        let paste = |t| pasted(&mut Fake(t));
        assert_eq!(
            paste(Some("  C:\\Games\\Some Game  \r\nsecond line")).as_deref(),
            Some("C:\\Games\\Some Game")
        );
        assert_eq!(
            paste(Some("\n\n  /home/me/game\n")).as_deref(),
            Some("/home/me/game")
        );
        assert_eq!(paste(Some("a\tb\u{7}c")).as_deref(), Some("abc"));
        assert_eq!(paste(Some("")), None);
        assert_eq!(paste(Some(" \r\n \n")), None);
        assert_eq!(paste(Some("\u{7}")), None);
        assert_eq!(paste(None), None);
    }

    /// Invisible format characters never get in, and the quotes around a whole path go.
    #[test]
    fn invisible_characters_and_quotes_go() {
        assert_eq!(line_of("\u{202A}C:\\Games").as_deref(), Some("C:\\Games"));
        assert_eq!(
            line_of("\u{FEFF}\u{200B}/home/me\u{2066}/game\u{2069}").as_deref(),
            Some("/home/me/game")
        );
        assert_eq!(
            line_of("\"C:\\Games\\Some Game\"").as_deref(),
            Some("C:\\Games\\Some Game")
        );
        assert_eq!(
            line_of(" \u{202A}\"D:\\x\"\u{202C} ").as_deref(),
            Some("D:\\x")
        );
        // A quote inside the text is left as it is.
        assert_eq!(line_of("\"a\" \"b\"").as_deref(), Some("\"a\" \"b\""));
        assert_eq!(line_of("\u{200B}\u{FEFF}"), None);
    }

    /// A huge single line is cut to MAX_CHARS characters.
    #[test]
    fn a_huge_line_is_cut() {
        let huge = "x".repeat(1 << 20);
        let line = line_of(&huge).unwrap();
        assert_eq!(line.chars().count(), MAX_CHARS);
        let quoted = format!("\"{}\"", "y".repeat(MAX_CHARS));
        assert_eq!(line_of(&quoted).unwrap().chars().count(), MAX_CHARS);
    }

    /// Ctrl+V pastes on Windows and Linux, Cmd+V on macOS, by the V's character or its place on
    /// the keyboard; V alone, or another key, does not.
    #[test]
    fn the_paste_key_is_the_systems_own() {
        let mac = cfg!(target_os = "macos");
        assert_eq!(is_paste("v", false, true, false), !mac);
        assert_eq!(is_paste("V", false, false, true), mac);
        // A layout whose letters are not Latin: the key in V's place.
        assert!(is_paste("\u{43c}", true, !mac, mac));
        assert!(!is_paste("v", true, false, false));
        assert!(!is_paste("c", false, true, true));
    }
}
