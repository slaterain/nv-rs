//! Play on Linux and macOS: the package's `play.sh`, the start every Linux and macOS package has
//! had, run with its output in the launcher's terminal and its questions answered at the
//! terminal's prompt (`NV_RS_ANSWERS_FROM_INPUT=1`, play.sh's own way of asking without a terminal
//! attached). It finds the game, offers to copy the saves, makes the mods ready and starts the
//! game, as it does from a terminal; the launcher only shows it. Its steps move into the launcher
//! in later pull requests, and play.sh stays as the plain-terminal way in.

use crate::session::{self, Event};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;

/// play.sh's line when it starts the game program (`say "Starting the game ($game). Log: $log"`):
/// the launcher's window steps aside then.
pub const STARTING: &str = "Starting the game (";

/// What the player types goes to play.sh's input while it runs.
static INPUT: Mutex<Option<ChildStdin>> = Mutex::new(None);

/// The package's play.sh: beside the launcher's program.
pub fn script() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("play.sh"))
}

/// The window was closed: play.sh's input ends, so a question it asks reads an empty answer (it
/// stops, or leaves the saves where they are) instead of waiting for one that never comes.
pub fn close_input() {
    if let Ok(mut input) = INPUT.lock() {
        *input = None;
    }
}

/// A line the player typed, to play.sh's input (its question's answer).
pub fn type_line(text: &str) {
    if let Ok(mut input) = INPUT.lock() {
        if let Some(pipe) = input.as_mut() {
            let _ = pipe.write_all(format!("{text}\n").as_bytes());
            let _ = pipe.flush();
        }
    }
}

/// Runs play.sh with `args` to its end, its output sent to the window as it comes: play.sh's exit
/// code (127 when it could not start).
pub fn run(script: &Path, args: &[std::ffi::OsString]) -> i32 {
    let started = Command::new("sh")
        .arg(script)
        .args(args)
        .env("NV_RS_ANSWERS_FROM_INPUT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child: Child = match started {
        Ok(c) => c,
        Err(e) => {
            session::send(Event::Output(format!(
                "nv-rs: {} could not be started ({e}): extract the whole archive again, keeping its folders\n",
                script.display()
            )));
            return 127;
        }
    };
    if let Ok(mut input) = INPUT.lock() {
        *input = child.stdin.take();
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
    .map(|pipe| std::thread::spawn(move || forward(pipe)))
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

/// Sends what `pipe` gives to the window, a piece at a time as it comes (a question has no line
/// break at its end), and says when the game starts.
fn forward(mut pipe: Box<dyn Read + Send>) {
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
                if starts_the_game(&String::from_utf8_lossy(&line)) {
                    session::send(Event::GameShown);
                }
                line.clear();
            } else {
                line.push(b);
            }
        }
    }
}

/// Whether play.sh's `line` is the one it writes when it starts the game.
pub fn starts_the_game(line: &str) -> bool {
    line.starts_with(STARTING)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// play.sh still writes the line the launcher steps aside on, as it starts the game.
    #[test]
    fn play_sh_says_when_it_starts_the_game() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../distribution/unix/play.sh"),
        )
        .unwrap();
        assert!(
            script.contains(&format!("say \"{STARTING}$game). Log: $log\"")),
            "play.sh's start line changed"
        );
        assert!(starts_the_game(
            "Starting the game (/games/Fallout New Vegas). Log: /x.log"
        ));
        assert!(!starts_the_game("Found your game (Steam): /games/x"));
    }
}
