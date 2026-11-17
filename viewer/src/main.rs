//! nv-viewer: runs Fallout: New Vegas through the engine port.
//!
//! The player's own FalloutNV.exe (1.4.0.525) is mapped and the game's translated code
//! (`crates/engine`, through `crates/host`) runs its `WinMain` as the main thread; this program
//! only finds the installation, starts the host, gives the game's frames their turn and ends
//! when the game's `main` ends. The game draws in its own window through the real Direct3D 9
//! (`NV_D3D9=real`, docs/adr/0008) and plays its sound through DirectSound (`NV_SOUND=out`);
//! the host reads the keyboard and the mouse from that window.
//!
//! Until 2026-11-03 this was also a hand-written reimplementation of the game drawn with Bevy;
//! that layer was removed (docs/adr/0011-old-layer-removed.md).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const USAGE: &str = "\
nv-viewer - run Fallout: New Vegas through the engine port

USAGE:
    nv-viewer <DATA FOLDER> [OPTIONS]

    DATA FOLDER  the game's Data folder, or the installation folder containing it
                 (FalloutNV.exe 1.4.0.525 is read from the installation)

OPTIONS:
    --movement-missing FILE  write the untranslated functions the game's code reached,
                             with counts, to FILE, its stops to FILE.stops.tsv and the
                             static initializers skipped to FILE.static.tsv (every 5 s
                             and when the game's main ends)
    --movement-probe         for testing: the game's code goes on past functions not
                             translated yet (with zero results) to list more of them
    --check                  only check the installation (FalloutNV.exe is 1.4.0.525) and
                             what the run will find, print it and end: status 0 when the
                             game can start, 1 when not
    -h, --help               print this

Other arguments (the removed viewer's place name, --walk, --physics, --wait, --background,
--new-game, --official, ...) are accepted and ignored, so older launchers keep working.

ENVIRONMENT:
    NV_D3D9=real    the game's own window through the real Direct3D 9 (the play build)
    NV_SOUND=out    the game's sound through DirectSound
    docs/PLAYTESTING.md lists the rest (read by crates/host and crates/engine-core).
";

/// Linux and macOS: what `play.sh` asks the program before a start.
#[cfg(not(windows))]
const FIND_USAGE: &str = "
LINUX AND MACOS:
    nv-viewer --find   print nv-rs's own folder, the game folders found (Steam, Heroic, Lutris)
                       and the game's save folders found in Proton's and Wine's prefixes, one a
                       line, fields split by tabs: own FOLDER / warning TEXT / game STORE FOLDER /
                       saves FROM FOLDER; in each field every byte but ASCII letters, digits and
                       ` /._-+,:@=~()` is written %XX (only read; nothing is written)
";

/// `--find` (Linux and macOS): [`FIND_USAGE`]'s lines (`game_folders::report`), from `HOME`.
#[cfg(not(windows))]
fn find() {
    match game_folders::Home::from_process() {
        Some(home) => print!("{}", game_folders::report(&home)),
        None => {
            eprintln!("error: HOME is not set to a full path, so the player's folders are unknown");
            host::end_process(1);
        }
    }
}

/// The command line.
#[derive(Debug, Default, PartialEq)]
struct Args {
    data: PathBuf,
    movement_missing: Option<PathBuf>,
    movement_probe: bool,
    /// `--check`: check the installation and end.
    check: bool,
    /// The removed viewer's arguments, ignored.
    ignored: Vec<String>,
}

/// `Ok(None)`: `--help`. Each argument as the system gave it (on Linux any bytes: a game folder
/// whose name is not UTF-8 is carried as it is, never turned into text).
fn parse(raw: &[OsString]) -> Result<Option<Args>, String> {
    let mut args = Args::default();
    let mut data = None;
    let mut i = 0;
    while i < raw.len() {
        let arg = raw[i].as_os_str();
        match arg.to_str() {
            Some("-h" | "--help") => return Ok(None),
            Some("--movement-probe") => args.movement_probe = true,
            Some("--check") => args.check = true,
            Some("--movement-missing") => {
                i += 1;
                let file = raw.get(i).ok_or("--movement-missing needs a FILE")?;
                args.movement_missing = Some(PathBuf::from(file));
            }
            _ if data.is_none() && !arg.to_string_lossy().starts_with('-') => {
                data = Some(PathBuf::from(arg));
            }
            _ => args.ignored.push(arg.to_string_lossy().into_owned()),
        }
        i += 1;
    }
    args.data = data.ok_or("the game's Data folder is missing")?;
    Ok(Some(args))
}

/// The command line after the program's name. On Windows as before (`std::env::args`); on Linux
/// and macOS as the system's own strings (`args_os`), so a folder name that is not UTF-8 reaches
/// [`parse`] as it is instead of stopping the program with a panic.
fn command_line() -> Vec<OsString> {
    #[cfg(windows)]
    {
        std::env::args().skip(1).map(OsString::from).collect()
    }
    #[cfg(not(windows))]
    {
        std::env::args_os().skip(1).collect()
    }
}

/// The game's master file, which a Data folder holds.
const MAIN_MASTER: &str = "FalloutNV.esm";

/// The Data folder: `path` itself or its `Data`, whichever holds FalloutNV.esm (any case). Off
/// Windows by the finder's own rule (`game_folders::data_folder`: a `Data` folder in any case,
/// every one tried), which `--find` and play.sh use too, so a game they report is one this takes.
fn find_data_folder(path: &Path) -> Result<PathBuf, String> {
    #[cfg(not(windows))]
    if let Some(data) = game_folders::data_folder(path) {
        return Ok(data);
    }
    #[cfg(windows)]
    for folder in [path.to_path_buf(), path.join("Data")] {
        let has_master = std::fs::read_dir(&folder).is_ok_and(|entries| {
            entries.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(MAIN_MASTER)
            })
        });
        if has_master {
            return Ok(folder);
        }
    }
    Err(format!(
        "neither {} nor a Data folder inside it contains {MAIN_MASTER}",
        path.display()
    ))
}

/// `--movement-missing`'s lists, each rewritten whole: the untranslated addresses reached
/// (`path`), the stops (`.stops.tsv`) and the static initializers skipped (`.static.tsv`).
fn write_lists(host: &host::Host, path: &Path) {
    let _ = std::fs::write(path, host.movement.missing_tsv(&|_| None));
    let _ = std::fs::write(path.with_extension("stops.tsv"), host.movement.stops_tsv());
    let _ = std::fs::write(path.with_extension("static.tsv"), host.static_skips_tsv());
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn SetProcessDpiAwarenessContext(context: isize) -> i32;
}

fn main() {
    // With the game's own window (`NV_D3D9=real`, `wgpu` or `real,wgpu`, or a launcher's play
    // run, which draws by its `Fallout.ini`'s `sRenderer` without the variable) the process is
    // DPI-unaware, as FalloutNV.exe is (no `dpiAware` in its manifest): Windows scales the game's
    // window on a display above 100% from the start, as it does the original's (playtest,
    // 2026-10-30). Any backend named, as the window's test (`d3d9real::wanted`; a value that one
    // refuses draws nothing, and the awareness doesn't matter then).
    #[cfg(windows)]
    if std::env::var("NV_D3D9").is_ok_and(|v| !v.trim().is_empty())
        || host::fixes::is_play_run(|name| std::env::var(name).ok())
    {
        // DPI_AWARENESS_CONTEXT_UNAWARE.
        // SAFETY: no pointers; a failure leaves the awareness as it was.
        unsafe {
            SetProcessDpiAwarenessContext(-1);
        }
    }
    let raw = command_line();
    #[cfg(not(windows))]
    if raw.first().map(OsString::as_os_str) == Some(std::ffi::OsStr::new("--find")) {
        find();
        return;
    }
    let args = match parse(&raw) {
        Ok(Some(args)) => args,
        Ok(None) => {
            print!("{USAGE}");
            #[cfg(not(windows))]
            print!("{FIND_USAGE}");
            return;
        }
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            host::end_process(2);
        }
    };
    if !args.ignored.is_empty() {
        println!(
            "Ignored (the removed viewer's arguments): {}",
            args.ignored.join(" ")
        );
    }
    let data = match find_data_folder(&args.data) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("error: {e}");
            host::end_process(1);
        }
    };
    if args.check {
        match host::check_installation(&data) {
            Ok(lines) => {
                for line in lines {
                    println!("{line}");
                }
                return;
            }
            Err(e) => {
                eprintln!("error: {e}");
                host::end_process(1);
            }
        }
    }
    // The game's own code runs on the player's FalloutNV.exe, read from the same installation
    // as Data: any build but 1.4.0.525 is refused here, with the reason (2026-10-13). Starting
    // runs main's first frame. What differs from the exe: the fixes preset unless the settings or
    // the environment say otherwise (docs/FIXES.md).
    // `--movement-missing` and `--movement-probe` make the run a test: its threads run stepped
    // (the recorder and the probe are the main thread's engine's) unless `NV_THREADS` says, and
    // the player's Fallout.ini options are not read.
    let command_line_test = args.movement_missing.is_some() || args.movement_probe;
    let choice = match host::fixes::Choice::read_with(command_line_test) {
        Ok(choice) => choice,
        Err(e) => {
            eprintln!("error: {e}");
            host::end_process(1);
        }
    };
    let mut game = match host::Host::start_with(&data, &choice) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("error: {e}");
            host::end_process(1);
        }
    };
    println!("{}", game.describe());
    game.movement.probe = args.movement_probe;
    // The run ends where the game's `main` ends its frames (`host::Host::at_main_end`), as the
    // original's process does: with main's answer when it returned, with the game's own code
    // when it called `ExitProcess`, and with status 3 when it stopped (a fault, a function not
    // translated, a call the platform does not provide), so the window closes and the host's line
    // naming the end is the log's last. `--movement-missing`'s lists are written first.
    let lists = args.movement_missing.clone();
    game.at_main_end = Some(Box::new(
        move |game: &host::Host, end: host::player_move::MainEnd| {
            if let Some(path) = &lists {
                write_lists(game, path);
            }
            // What the platform layer runs before it ends a run itself (the window closed): the
            // game's own Quit ends here, and its last pipelines are written too.
            host::run_exit_hooks();
            host::end_process(end.status());
        },
    ));
    // An end in the start-up.
    game.end_if_main_ended();
    // Main's frames. The keyboard and the mouse come from the game's window
    // (`Host::set_keys_down`, `Host::mouse`); scripted runs add `NV_MOVE_KEYS` and
    // `NV_MOVE_MOUSE`. Every 5 s the status line (the heartbeat the launchers watch) and the
    // lists.
    let mut last: Option<Instant> = None;
    let mut frame_log = host::frame_log::FrameLog::from_env();
    loop {
        game.set_keys_down(&[]);
        game.mouse([0; 3], [false; 8]);
        if !game.main_frame() {
            break;
        }
        // `NV_REPEATABLE=FRAMES` (a test mode): the run ends after FRAMES of main's frames.
        if game.repeatable_end() {
            host::end_process(0);
        }
        if let Some(log) = &mut frame_log {
            log.record(&mut game.engine);
        }
        if last.map_or(true, |at| at.elapsed() >= Duration::from_secs(5)) {
            last = Some(Instant::now());
            println!("{}", game.movement.summary());
            if let Some(path) = &args.movement_missing {
                write_lists(&game, path);
            }
        }
    }
    // No frame and no end: the game's main never ran as the main thread.
    if let Some(path) = &args.movement_missing {
        write_lists(&game, path);
    }
    eprintln!(
        "error: the game's main is not running ({})",
        game.movement.summary()
    );
    host::end_process(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(a: &[&str]) -> Vec<OsString> {
        a.iter().map(OsString::from).collect()
    }

    /// Review of #630 (round 2): a game folder whose name is not UTF-8 (Linux allows any bytes)
    /// is carried as it is, byte for byte, and its check ends with a plain message, not a panic.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_folder_name_that_is_not_utf8_is_carried_as_it_is() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let folder = OsString::from_vec(b"/x/Fallout \xe9".to_vec());
        let raw = vec![folder.clone(), OsString::from("--check")];
        let args = parse(&raw).unwrap().unwrap();
        assert_eq!(args.data.as_os_str().as_bytes(), b"/x/Fallout \xe9");
        assert!(args.check);
        let message = find_data_folder(&args.data).unwrap_err();
        assert!(message.contains("FalloutNV.esm"), "{message}");
        // An argument that is not UTF-8 and is not the folder is kept among the ignored ones.
        let raw = vec![folder, OsString::from_vec(b"--odd\xff".to_vec())];
        assert_eq!(parse(&raw).unwrap().unwrap().ignored.len(), 1);
    }

    /// Review of #630 (round 4): on Linux a game whose `Data` folder, master and exe were
    /// lower-cased (copied by a tool that does that) is one `--find` reports, and the program takes
    /// it too: its Data folder and its exe are found, by the same rule as the finder's.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_game_the_finder_reports_in_another_case_is_taken() {
        let game = std::env::temp_dir().join(format!("nv-viewer-case-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&game);
        std::fs::create_dir_all(game.join("data")).unwrap();
        std::fs::write(game.join("data/falloutnv.esm"), b"not the game's file").unwrap();
        std::fs::write(game.join("falloutnv.exe"), b"not the game's file").unwrap();
        let reported = game_folders::game_folder(&game);
        let data = find_data_folder(&game);
        let exe = host::game_exe(&game.join("data"));
        let _ = std::fs::remove_dir_all(&game);
        assert_eq!(reported, Some(game.clone()));
        assert_eq!(data, Ok(game.join("data")));
        assert_eq!(exe, game.join("falloutnv.exe"));
    }

    /// play.sh's command line still parses: the Data folder, the old flags ignored.
    #[test]
    fn old_launch_lines_still_parse() {
        let raw = strings(&[
            "C:\\Games\\Data",
            "GSDocMitchellHouse",
            "--background",
            "--walk",
            "--physics",
            "engine",
            "--wait",
            "100000",
            "--movement-missing",
            "m.tsv",
        ]);
        let args = parse(&raw).unwrap().unwrap();
        assert_eq!(args.data, PathBuf::from("C:\\Games\\Data"));
        assert_eq!(args.movement_missing, Some(PathBuf::from("m.tsv")));
        assert!(!args.movement_probe);
        assert_eq!(args.ignored.len(), 7);
    }

    #[test]
    fn help_and_missing_data() {
        assert_eq!(parse(&strings(&["--help"])), Ok(None));
        assert!(parse(&strings(&["--movement-probe"])).is_err());
        assert!(parse(&strings(&["--movement-missing"])).is_err());
        assert!(
            parse(&strings(&["--check", r"C:\Games\Data"]))
                .unwrap()
                .unwrap()
                .check
        );
    }
}
