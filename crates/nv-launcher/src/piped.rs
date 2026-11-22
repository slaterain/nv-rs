//! A program the launcher runs with its lines in the terminal's text area and its questions
//! answered at the terminal's prompt: play.sh on Linux and macOS (`crate::play_unix`), and the mod
//! installer (the game program's `--install-mods`, the text chooser of `crates/mod-chooser`) on
//! every system. What the player types at the prompt goes to the program's input, a line at a time;
//! what it writes comes to the window as it comes (a question has no line break at its end).

use crate::session::{self, Event};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// What the player types goes to the running program's input.
static INPUT: Mutex<Option<ChildStdin>> = Mutex::new(None);

/// A launcher window that takes typed lines is there ([`window_takes_lines`]): set only by the
/// window once it shows, cleared when Play goes on in Windows' message boxes instead.
static WINDOW: AtomicBool = AtomicBool::new(false);

/// The window was closed since the work started ([`close_input`], [`reopen`]).
static CLOSED: AtomicBool = AtomicBool::new(false);

/// Whether a launcher window takes typed lines from now on.
pub fn window_takes_lines(yes: bool) {
    WINDOW.store(yes, Ordering::Relaxed);
}

/// Whether a launcher window takes typed lines now (else a program that asks questions runs in
/// a console of its own, or with its input closed).
pub fn window_lines() -> bool {
    WINDOW.load(Ordering::Relaxed)
}

/// Whether a started program keeps its input for the player's lines: only with a window that
/// takes them and not closed since the work started.
pub fn keeps_input(window: bool, closed: bool) -> bool {
    window && !closed
}

/// A new work starts: the program it runs keeps its input again (unless the window closes first).
pub fn reopen() {
    if let Ok(_input) = INPUT.lock() {
        CLOSED.store(false, Ordering::Relaxed);
    }
}

/// The window was closed: the program's input ends, so a question it asks reads an empty answer
/// (play.sh stops or leaves the saves where they are; the mod installer finishes) instead of
/// waiting for one that never comes; a program not started yet starts with its input closed.
pub fn close_input() {
    if let Ok(mut input) = INPUT.lock() {
        CLOSED.store(true, Ordering::Relaxed);
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
    // With no window that takes lines, or one closed already, nobody will type: the program's
    // questions read an empty answer at once, as play.sh and the installer do with their input
    // closed. Checked and stored under the lock [`close_input`] takes, so a close cannot slip
    // between.
    if let Ok(mut input) = INPUT.lock() {
        if keeps_input(window_lines(), CLOSED.load(Ordering::Relaxed)) {
            *input = child.stdin.take();
        } else {
            drop(child.stdin.take());
        }
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
    if let Ok(mut input) = INPUT.lock() {
        *input = None;
    }
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

    /// The tests change the run-wide flags: one at a time.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn heard(line: &str) {
        LINES
            .lock()
            .unwrap()
            .push(line.trim_end_matches('\r').to_string());
    }

    /// Each system's own shell: prints a line, reads one (a question), prints one more on its
    /// errors, ends with 3.
    fn asking_shell() -> Command {
        #[cfg(windows)]
        let c = {
            let mut c = Command::new("cmd");
            c.args(["/C", "echo one& set /p a=& echo two 1>&2& exit 3"]);
            c
        };
        #[cfg(not(windows))]
        let c = {
            let mut c = Command::new("sh");
            c.args(["-c", "echo one; read a; echo two >&2; exit 3"]);
            c
        };
        c
    }

    /// A program's whole lines (its output and its errors) reach `on_line`, its exit code comes
    /// back, and with no window that takes lines its input is closed at once (a question reads
    /// an empty answer and the program goes on): every system, through its own shell.
    #[test]
    fn lines_and_the_exit_code_come_back_and_no_window_closes_the_input() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        window_takes_lines(false);
        reopen();
        assert_eq!(run(asking_shell(), "the test's shell", heard), 3);
        let lines = LINES.lock().unwrap().clone();
        assert!(lines.contains(&"one".to_string()), "{lines:?}");
        assert!(lines.iter().any(|l| l.trim() == "two"), "{lines:?}");
    }

    /// A window closed before the program starts (Play's work not spawned yet): the program
    /// starts with its input closed and does not wait for a line nobody sends; a new work keeps
    /// its input again.
    #[test]
    fn a_window_closed_before_the_start_closes_the_input() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        window_takes_lines(true);
        close_input();
        assert_eq!(run(asking_shell(), "the test's shell", |_| {}), 3);
        window_takes_lines(false);
        assert!(keeps_input(true, false));
        assert!(!keeps_input(true, true));
        assert!(!keeps_input(false, false));
        reopen();
        assert!(!CLOSED.load(Ordering::Relaxed));
    }

    /// A program that cannot be started ends as 127.
    #[test]
    fn a_program_that_cannot_start_is_127() {
        let c = Command::new("nv-rs-no-such-program-here");
        assert_eq!(run(c, "nothing", |_| {}), 127);
    }
}
