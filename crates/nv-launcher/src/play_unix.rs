//! Play on Linux and macOS: the package's `play.sh`, the start every Linux and macOS package has
//! had, run with its output in the launcher's terminal and its questions answered at the
//! terminal's prompt (`NV_RS_ANSWERS_FROM_INPUT=1`, play.sh's own way of asking without a terminal
//! attached; `crate::piped` carries both). It finds the game, offers to copy the saves, makes the
//! mods ready and starts the game (or, with `--install-mods`, runs the mod installer), as it does
//! from a terminal; the launcher only shows it. Its steps move into the launcher in later pull
//! requests, and play.sh stays as the plain-terminal way in. When the game it started ends badly,
//! the launcher keeps a crash report (`crate::crash`) and says where.

use crate::session::{self, Event};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

/// play.sh's line when it starts the game program (`say "Starting the game ($game). Log: $log"`):
/// the launcher's window steps aside then.
pub const STARTING: &str = "Starting the game (";

/// The game play.sh started in this run: its folder and log (from its start line), and when.
static STARTED: Mutex<Option<(PathBuf, PathBuf, SystemTime)>> = Mutex::new(None);

/// The package's play.sh: beside the launcher's program.
pub fn script() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("play.sh"))
}

fn take_started() -> Option<(PathBuf, PathBuf, SystemTime)> {
    STARTED.lock().ok().and_then(|mut s| s.take())
}

/// Runs play.sh with `args` to its end, its output sent to the window as it comes: play.sh's exit
/// code (127 when it could not start). When the game it started ended badly, a crash report.
pub fn run(script: &Path, args: &[std::ffi::OsString]) -> i32 {
    take_started();
    let mut command = Command::new("sh");
    command
        .arg(script)
        .args(args)
        .env("NV_RS_ANSWERS_FROM_INPUT", "1");
    let code = crate::piped::run(command, &script.display().to_string(), said);
    if let Some((game, log, started)) = take_started() {
        let mods = args.iter().any(|a| a == "--mods");
        if let Some(line) = crash_report(script, &game, &log, started, code, mods) {
            session::send(Event::Output(line));
        }
    }
    code
}

/// The crash report of a game run that ended with `code` (play.sh's status, the program's), and
/// the line that says where it is; none for a good end.
fn crash_report(
    script: &Path,
    game: &Path,
    log: &Path,
    started: SystemTime,
    code: i32,
    mods: bool,
) -> Option<String> {
    // play.sh's log is `<own>/logs/game-<time>.log`.
    let own = log.parent()?.parent()?;
    let program = script.parent()?;
    // The game as found (with mods the start line names nv-rs's install of it), from the file
    // play.sh remembers it in.
    let mut games = vec![game.to_path_buf()];
    if let Ok(found) = std::fs::read(own.join("game-folder.txt")) {
        let found = String::from_utf8_lossy(&found)
            .trim_end_matches('\n')
            .to_string();
        if !found.is_empty() {
            games.push(PathBuf::from(found));
        }
    }
    let kept = crate::crash::after_run(&crate::crash::Run {
        status: Some(code),
        shell: crate::crash::Shell::Unix,
        log,
        started,
        mods,
        own,
        program,
        games,
        table_card: None,
    })?;
    Some(match kept {
        Ok(folder) => format!(
            "\nnv-rs kept a crash report of this, with your user name and folder names taken out:\n{}\nAttach its two files when you report it.\n",
            folder.display()
        ),
        Err(why) => format!("\nnv-rs could not keep a crash report: {why}\n"),
    })
}

/// A whole line of play.sh's: the window steps aside when it says the game starts, and the
/// game's folder and log are kept for a crash report.
fn said(line: &str) {
    if starts_the_game(line) {
        if let (Some(started), Ok(mut kept)) = (started_game(line), STARTED.lock()) {
            *kept = Some((started.0, started.1, SystemTime::now()));
        }
        session::send(Event::GameShown);
    }
}

/// Whether play.sh's `line` is the one it writes when it starts the game.
pub fn starts_the_game(line: &str) -> bool {
    line.starts_with(STARTING)
}

/// The game's folder and log named by play.sh's start line.
pub fn started_game(line: &str) -> Option<(PathBuf, PathBuf)> {
    let rest = line.strip_prefix(STARTING)?;
    let (game, log) = rest.rsplit_once("). Log: ")?;
    Some((PathBuf::from(game), PathBuf::from(log.trim_end())))
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
        assert_eq!(
            started_game(
                "Starting the game (/games/Fallout (GOG) New Vegas). Log: /own/logs/game-1.log"
            ),
            Some((
                PathBuf::from("/games/Fallout (GOG) New Vegas"),
                PathBuf::from("/own/logs/game-1.log")
            ))
        );
        assert_eq!(started_game("Found your game (Steam): /games/x"), None);
    }

    /// A game run play.sh started that ended on a signal leaves a crash report in nv-rs's own
    /// folder (the log's grandparent); a good end leaves none.
    #[test]
    fn a_crashed_run_leaves_a_report_beside_its_log() {
        let own =
            std::env::temp_dir().join(format!("nv-launcher-unix-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&own);
        std::fs::create_dir_all(own.join("logs")).unwrap();
        let log = own.join("logs").join("game-1.log");
        std::fs::write(&log, "a line of the game's\n").unwrap();
        let script = own.join("play.sh");
        let game = own.join("game");
        let now = SystemTime::now();
        assert_eq!(crash_report(&script, &game, &log, now, 0, false), None);
        let line = crash_report(&script, &game, &log, now, 139, false).unwrap();
        assert!(line.contains("kept a crash report"), "{line}");
        let reports: Vec<_> = std::fs::read_dir(own.join("reports"))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(reports.len(), 1);
        let body = std::fs::read_to_string(reports[0].path().join("report.txt")).unwrap();
        assert!(body.contains("signal 11"), "{body}");
        let _ = std::fs::remove_dir_all(&own);
    }
}
