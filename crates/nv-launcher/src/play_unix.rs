//! Play on Linux and macOS: the package's `play.sh`, the start every Linux and macOS package has
//! had, run with its output in the launcher's terminal and its questions answered at the
//! terminal's prompt (`NV_RS_ANSWERS_FROM_INPUT=1`, play.sh's own way of asking without a terminal
//! attached; `crate::piped` carries both). It finds the game, offers to copy the saves, makes the
//! mods ready and starts the game (or, with `--install-mods`, runs the mod installer), as it does
//! from a terminal; the launcher only shows it. Its steps move into the launcher in later pull
//! requests, and play.sh stays as the plain-terminal way in.

use crate::session::{self, Event};
use std::path::{Path, PathBuf};
use std::process::Command;

/// play.sh's line when it starts the game program (`say "Starting the game ($game). Log: $log"`):
/// the launcher's window steps aside then.
pub const STARTING: &str = "Starting the game (";

/// The package's play.sh: beside the launcher's program.
pub fn script() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("play.sh"))
}

/// Runs play.sh with `args` to its end, its output sent to the window as it comes: play.sh's exit
/// code (127 when it could not start).
pub fn run(script: &Path, args: &[std::ffi::OsString]) -> i32 {
    let mut command = Command::new("sh");
    command
        .arg(script)
        .args(args)
        .env("NV_RS_ANSWERS_FROM_INPUT", "1");
    crate::piped::run(command, &script.display().to_string(), said)
}

/// A whole line of play.sh's: the window steps aside when it says the game starts.
fn said(line: &str) {
    if starts_the_game(line) {
        session::send(Event::GameShown);
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
