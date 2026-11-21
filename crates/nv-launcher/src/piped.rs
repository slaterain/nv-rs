//! A program the launcher runs with its lines in the terminal's text area and its questions
//! answered at the terminal's prompt: play.sh on Linux and macOS (`crate::play_unix`), and the mod
//! installer (the game program's `--install-mods`, the text chooser of `crates/mod-chooser`) on
//! every system. What the player types at the prompt goes to the program's input, a line at a time;
//! what it writes comes to the window as it comes (a question has no line break at its end).

use crate::session::{self, Event};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;

/// What the player types goes to the running program's input.
static INPUT: Mutex<Option<ChildStdin>> = Mutex::new(None);

/// The window was closed: the program's input ends, so a question it asks reads an empty answer
/// (play.sh stops or leaves the saves where they are; the mod installer finishes) instead of
/// waiting for one that never comes.
pub fn close_input() {
    if let Ok(mut input) = INPUT.lock() {
        *input = None;
    }
}

/// A line the player typed, to the running program's input (its question's answer).
pub fn type_line(text: &str) {
    if let Ok(mut input) = INPUT.lock() {
        if let Some(pipe) = input.as_mut() {
            let _ = pipe.write_all(format!("{text}\n").as_bytes());
            let _ = pipe.flush();
        }
    }
}

/// Runs `command` to its end, its output (and errors) sent to the window as they come, each whole
/// line also given to `on_line`: its exit code (127 when it could not start, `what` then named in
/// the window's line).
pub fn run(mut command: Command, what: &str, on_line: fn(&str)) -> i32 {
    let started = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child: Child = match started {
        Ok(c) => c,
        Err(e) => {
            session::send(Event::Output(format!(
                "nv-rs: {what} could not be started ({e}): extract the whole archive again, keeping its folders\n"
            )));
            return 127;
        }
    };
    // With no window (scripted runs) nobody will type: the program's questions read an empty
    // answer at once, as play.sh and the installer do with their input closed.
    if session::has_window() {
        if let Ok(mut input) = INPUT.lock() {
            *input = child.stdin.take();
        }
    } else {
        drop(child.stdin.take());
    }
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|pipe| std::thread::spawn(move || forward(pipe, on_line)))
    .collect();
    for r in readers {
        let _ = r.join();
    }
    let status = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    close_input();
    status
}

/// Sends what `pipe` gives to the window, a piece at a time as it comes, and each whole line to
/// `on_line`.
fn forward(mut pipe: Box<dyn Read + Send>, on_line: fn(&str)) {
    let mut buffer = [0u8; 4096];
    let mut line = Vec::new();
    loop {
        let n = match pipe.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        session::send(Event::Output(
            String::from_utf8_lossy(&buffer[..n]).into_owned(),
        ));
        for &b in &buffer[..n] {
            if b == b'\n' {
                on_line(&String::from_utf8_lossy(&line));
                line.clear();
            } else {
                line.push(b);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn heard(line: &str) {
        LINES
            .lock()
            .unwrap()
            .push(line.trim_end_matches('\r').to_string());
    }

    /// A program's whole lines (its output and its errors) reach `on_line`, its exit code comes
    /// back, and with no window its input is closed at once (a question reads an empty answer and
    /// the program goes on): every system, through its own shell.
    #[test]
    fn lines_and_the_exit_code_come_back_and_no_window_closes_the_input() {
        #[cfg(windows)]
        let mut c = {
            let mut c = Command::new("cmd");
            c.args(["/C", "echo one& set /p a=& echo two 1>&2& exit 3"]);
            c
        };
        #[cfg(not(windows))]
        let mut c = {
            let mut c = Command::new("sh");
            c.args(["-c", "echo one; read a; echo two >&2; exit 3"]);
            c
        };
        c.env("NV_RS_PIPED_TEST", "1");
        assert_eq!(run(c, "the test's shell", heard), 3);
        let lines = LINES.lock().unwrap().clone();
        assert!(lines.contains(&"one".to_string()), "{lines:?}");
        assert!(lines.iter().any(|l| l.trim() == "two"), "{lines:?}");
    }

    /// A program that cannot be started ends as 127.
    #[test]
    fn a_program_that_cannot_start_is_127() {
        let c = Command::new("nv-rs-no-such-program-here");
        assert_eq!(run(c, "nothing", |_| {}), 127);
    }
}
