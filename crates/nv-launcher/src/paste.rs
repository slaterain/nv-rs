//! Paste at the prompt: Ctrl+V (Cmd+V on macOS) puts the clipboard's text where typing goes (the
//! game's folder, an answer to play.sh): its first line with anything on it, spaces around it
//! taken off, never a control character. An empty clipboard, one holding no text (a picture), or
//! one that cannot be opened at all pastes nothing; it never stops the launcher. The system's
//! clipboard is arboard's (Windows, macOS, X11 and Wayland), behind [`Clipboard`] so the rule is
//! tested without one.

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

/// What a paste adds at the prompt: the clipboard's first line that has anything on it, trimmed,
/// without control characters; `None` when there is nothing to add.
pub fn pasted(clipboard: &mut dyn Clipboard) -> Option<String> {
    let text = clipboard.text()?;
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    let line: String = line.chars().filter(|c| !c.is_control()).collect();
    (!line.is_empty()).then_some(line)
}

/// Whether the key `key` pressed with Ctrl (`control`) or the Command key (`command`) is this
/// system's paste: Cmd+V on macOS, Ctrl+V elsewhere.
pub fn is_paste(key: &str, control: bool, command: bool) -> bool {
    let held = if cfg!(target_os = "macos") {
        command
    } else {
        control
    };
    held && key.eq_ignore_ascii_case("v")
}

/// A key pressed at the prompt: when it is this system's paste ([`is_paste`]), what `clipboard`
/// gives ([`pasted`]) is added to `typed` and `true` is returned (the key types nothing itself,
/// whatever its text, even when the clipboard has nothing to give); any other key: `false`.
pub fn on_key(
    typed: &mut String,
    key: &str,
    control: bool,
    command: bool,
    clipboard: &mut dyn Clipboard,
) -> bool {
    if !is_paste(key, control, command) {
        return false;
    }
    if let Some(text) = pasted(clipboard) {
        typed.push_str(&text);
    }
    true
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

    /// Ctrl+V pastes on Windows and Linux, Cmd+V on macOS; V alone, or another key, does not.
    #[test]
    fn the_paste_key_is_the_systems_own() {
        let mac = cfg!(target_os = "macos");
        assert_eq!(is_paste("v", true, false), !mac);
        assert_eq!(is_paste("V", false, true), mac);
        assert!(!is_paste("v", false, false));
        assert!(!is_paste("c", true, true));
    }

    /// The paste key adds the clipboard's line after what is typed and types nothing itself (also
    /// with an empty clipboard); another key is left to the typing.
    #[test]
    fn the_paste_key_adds_the_line_after_what_is_typed() {
        let mac = cfg!(target_os = "macos");
        let (control, command) = (!mac, mac);
        let mut typed = String::from("D:\\");
        assert!(on_key(
            &mut typed,
            "v",
            control,
            command,
            &mut Fake(Some("Games\\Some Game\n"))
        ));
        assert_eq!(typed, "D:\\Games\\Some Game");
        assert!(on_key(&mut typed, "V", control, command, &mut Fake(None)));
        assert_eq!(typed, "D:\\Games\\Some Game");
        assert!(!on_key(&mut typed, "v", false, false, &mut Fake(Some("x"))));
        assert_eq!(typed, "D:\\Games\\Some Game");
    }
}
