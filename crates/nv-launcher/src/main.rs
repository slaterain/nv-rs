//! nv-rs's launcher: one window in the look of the game's own terminal (read from the player's
//! installed game; the port's own plain look before the game is found), from which the player
//! plays. On Windows it is the package's Play.exe and does what Play.exe did (crate::play); on
//! Linux and macOS it is `nv-rs` and runs the package's play.sh in its terminal (crate::play_unix).
//!
//!   nv-rs                       the start screen (the game's intro before it, the first time)
//!   nv-rs --mods | --import-saves | --install-mods | --check | --game FOLDER | --from FOLDER
//!                               Play.exe's actions (Windows) and play.sh's (Linux, macOS), at once
//!
//! For pictures and tests (nothing is written but the capture, no sound):
//!   nv-rs --capture FILE.png [--game FOLDER] [--profile FALLOUT.INI] [--plain] [--size WxH]
//!         [--intro-at MS]       the start screen (or the intro MS milliseconds in) drawn off screen
//!   nv-rs --game-card           the card and API the game will draw with on Vulkan, one line
//!
//! `NV_RS_NO_WINDOWS=1` (scripted tests): no window at all; the action runs as Play.exe and
//! play.sh ran it, plain play when none is named.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod canvas;
mod font;
mod game;
mod gpu;
mod intro;
mod place;
#[cfg(windows)]
mod play;
#[cfg_attr(windows, allow(dead_code))]
mod play_unix;
mod png;
mod screen;
mod session;
// The Settings screen's rows (its screen comes with the next pull request); the start screen
// reads the renderer row.
#[allow(dead_code)]
mod settings;

use std::path::PathBuf;

/// The options for pictures and tests; every other argument is the action's (Play.exe's or
/// play.sh's).
pub struct Capture {
    pub file: PathBuf,
    pub game: Option<PathBuf>,
    pub profile: Option<PathBuf>,
    pub size: (u32, u32),
    pub plain: bool,
    pub intro_at: Option<u64>,
}

fn capture_options(raw: &[String]) -> Result<Option<Capture>, String> {
    if !raw.iter().any(|a| a == "--capture") {
        return Ok(None);
    }
    let mut c = Capture {
        file: PathBuf::new(),
        game: std::env::var_os("NV_RS_GAME").map(PathBuf::from),
        profile: None,
        size: (screen::WIDTH, screen::HEIGHT),
        plain: false,
        intro_at: None,
    };
    let mut args = raw.iter();
    while let Some(a) = args.next() {
        let mut value = |what: &str| args.next().cloned().ok_or(format!("{a} needs {what}"));
        match a.as_str() {
            "--capture" => c.file = value("a file")?.into(),
            "--game" => c.game = Some(value("a folder")?.into()),
            "--profile" => c.profile = Some(value("a Fallout.ini")?.into()),
            "--size" => {
                let s = value("WIDTHxHEIGHT")?;
                let (w, h) = s.split_once('x').ok_or("--size needs WIDTHxHEIGHT")?;
                c.size = (
                    w.parse().map_err(|_| "--size: not a number")?,
                    h.parse().map_err(|_| "--size: not a number")?,
                );
            }
            "--intro-at" => {
                c.intro_at = Some(
                    value("milliseconds")?
                        .parse()
                        .map_err(|_| "--intro-at: not a number")?,
                )
            }
            "--plain" => c.plain = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Some(c))
}

/// The card line for `--game-card`: name, a tab, the API.
fn game_card_line() -> String {
    match gpu::game_card() {
        Some((name, api)) => format!("{name}\t{}", gpu::api_name(api)),
        None => String::new(),
    }
}

/// Runs the action named by `raw` (Play.exe's or play.sh's arguments) with no window: its exit
/// code.
fn without_window(raw: &[String]) -> i32 {
    #[cfg(windows)]
    {
        play::run_play(raw)
    }
    #[cfg(not(windows))]
    {
        let Some(script) = play_unix::script() else {
            eprintln!("nv-rs: the folder of this program is not known");
            return 1;
        };
        match std::process::Command::new("sh")
            .arg(&script)
            .args(raw)
            .status()
        {
            Ok(s) => s.code().unwrap_or(1),
            Err(e) => {
                eprintln!("nv-rs: {} could not be started: {e}", script.display());
                127
            }
        }
    }
}

fn main() {
    let raw: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    if raw.first().map(String::as_str) == Some("--game-card") {
        println!("{}", game_card_line());
        return;
    }
    match capture_options(&raw) {
        Ok(Some(c)) => {
            if let Err(e) = app::capture(&c) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            return;
        }
        Ok(None) => {}
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
    if std::env::var_os("NV_RS_NO_WINDOWS").is_some_and(|v| v == "1") {
        std::process::exit(without_window(&raw));
    }
    std::process::exit(app::run(raw));
}
