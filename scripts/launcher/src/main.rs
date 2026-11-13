//! Play.exe, the preview package's launcher: finds the player's game (Steam, GOG and Epic, read
//! only, or it asks), checks it is 1.4.0.525, makes this PC's tables, keeps every file the game
//! writes in the package's own folder and starts the game, saying plainly in a window what went
//! wrong when something does. Nothing is ever written outside the package's folder.
//!
//!   Play.exe                  plays
//!   Play.exe --mods           plays with the mods in the `mods` folder
//!   Play.exe --import-saves   copies the saves from Documents\My Games\FalloutNV (read only)
//!   Play.exe --check          everything up to starting the game, then says what it found
//!   --game FOLDER             the game's folder, instead of looking for it
//!   --from FOLDER             with --import-saves: where to copy saves from
//!
//! Scripted tests set `NV_RS_NO_WINDOWS=1` (no window shown; every message is in
//! `userdata\logs\launcher.log` either way), `NV_RS_REAL_DOCUMENTS=DIR` (where the player's
//! Documents are read from), `NV_RS_NO_SHORT_PATHS=1`, `NV_RS_NO_STORES=1` (look in no store) and `NV_RS_SYSTEM_FOLDER=DIR`.
#![windows_subsystem = "windows"]

mod find;
mod game_ini;
mod mods;
mod settings;
mod tables;
mod ui;
mod win;

use settings::Settings;
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Why the launcher stops: what the player reads, and the launcher's exit code.
#[derive(Debug)]
struct Stop {
    code: i32,
    message: String,
}

fn stop<T>(code: i32, message: impl Into<String>) -> Result<T, Stop> {
    Err(Stop {
        code,
        message: message.into(),
    })
}

const STOP_LOCATION: i32 = 2;
const STOP_NO_GAME: i32 = 3;
const STOP_WRONG_GAME: i32 = 4;
const STOP_DIRECTX: i32 = 5;
const STOP_TABLES: i32 = 6;
const STOP_GAME_STOPPED: i32 = 7;
const STOP_MODS: i32 = 8;
const STOP_BLOCKED: i32 = 9;
const STOP_HUNG: i32 = 10;
const STOP_SETTINGS: i32 = 11;

/// How long the game program's `--check` may take (about a second here, a little longer with a
/// slow sound device query).
const CHECK_LIMIT: Duration = Duration::from_secs(60);

const ISSUES: &str = "https://github.com/slaterain/nv-rs/issues";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The package's folders.
struct Package {
    root: PathBuf,
    game_exe: PathBuf,
    tools: PathBuf,
    userdata: PathBuf,
    machine: PathBuf,
    logs: PathBuf,
    reports: PathBuf,
    settings: PathBuf,
    remembered: PathBuf,
    /// Log lines kept in memory until the package's place is checked ([`clear_for_writing`]): a
    /// refusal leaves the disk untouched.
    held: RefCell<Vec<String>>,
    writing: Cell<bool>,
}

impl Package {
    fn at(root: PathBuf) -> Package {
        let userdata = root.join("userdata");
        Package {
            game_exe: root.join("app\\FalloutNV.exe"),
            tools: root.join("app\\tools"),
            machine: userdata.join("machine"),
            logs: userdata.join("logs"),
            reports: userdata.join("reports"),
            settings: userdata.join("settings.ini"),
            remembered: userdata.join("game-folder.txt"),
            held: RefCell::new(Vec::new()),
            writing: Cell::new(false),
            userdata,
            root,
        }
    }

    fn log(&self, text: &str) {
        let (when, _) = win::now();
        let line = format!("{when} {text}");
        if self.writing.get() {
            self.append(&line);
        } else {
            self.held.borrow_mut().push(line);
        }
    }

    fn append(&self, line: &str) {
        if std::fs::create_dir_all(&self.logs).is_ok() {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.logs.join("launcher.log"))
            {
                let _ = writeln!(file, "{line}");
            }
        }
    }

    /// From here on the log goes to its file, with what was kept so far first.
    fn start_writing(&self) {
        self.writing.set(true);
        let held = std::mem::take(&mut *self.held.borrow_mut());
        for line in held {
            self.append(&line);
        }
    }
}

/// The game's own folders in the package, one set for each way of playing.
struct Profile {
    documents: PathBuf,
    roaming: PathBuf,
    local: PathBuf,

    saves: PathBuf,
    prefs: PathBuf,
    table: PathBuf,
}

impl Profile {
    fn at(userdata: &Path, name: &str) -> Profile {
        let base = userdata.join(name);
        let documents = base.join("Documents");
        let game_documents = documents.join("My Games\\FalloutNV");
        Profile {
            roaming: base.join("AppData\\Roaming"),
            local: base.join("AppData\\Local"),
            saves: game_documents.join("Saves"),
            prefs: game_documents.join("FalloutPrefs.ini"),
            table: base.join("adapter.tsv"),

            documents,
        }
    }

    /// The profile's `Fallout.ini`, beside its `FalloutPrefs.ini`.
    fn fallout_ini(&self) -> PathBuf {
        self.prefs.with_file_name("Fallout.ini")
    }

    fn make(&self) -> std::io::Result<()> {
        for dir in [&self.saves, &self.roaming, &self.local.join("FalloutNV")] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

/// `\\?\` taken off the front of what Windows answers for a real path.
fn without_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        PathBuf::from(format!("\\\\{rest}"))
    } else if let Some(rest) = text.strip_prefix("\\\\?\\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

/// The path Windows really means: junctions, symbolic links, `subst` and mapped drives and 8.3
/// short names resolved. A part that doesn't exist (yet) is kept as it is, after the real path of
/// the longest part that does.
fn real_path(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path.to_path_buf();
    loop {
        if let Ok(real) = std::fs::canonicalize(&at) {
            let mut out = without_verbatim(real);
            for part in rest.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (at.file_name().map(|n| n.to_owned()), at.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                at = parent.to_path_buf();
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Lower case, `\` only, no trailing `\`, links and short names resolved ([`real_path`]): for
/// comparing paths.
fn folder_key(path: &Path) -> String {
    real_path(path)
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// Whether `inner` is `outer` or lies inside it (both [`folder_key`]s).
fn is_inside(inner: &str, outer: &str) -> bool {
    inner == outer || inner.starts_with(&format!("{outer}\\"))
}

/// The longest name the game makes under Documents (a save, up to 60 characters) must fit the
/// 259 characters its paths have. An error text when `documents`' path is too long.
fn path_too_long(documents_bytes: usize) -> bool {
    documents_bytes + "\\My Games\\FalloutNV\\Saves\\".len() + 60 >= 260
}

/// Where the player's real Documents are (only read).
fn real_documents() -> Option<PathBuf> {
    std::env::var_os("NV_RS_REAL_DOCUMENTS")
        .map(PathBuf::from)
        .or_else(|| win::known_folder(5))
}

/// The player's folders under `profile` (`%USERPROFILE%`) by their text, wherever Windows says
/// Documents is: Documents, OneDrive's Documents and AppData. The game refuses to keep saves there
/// (host `save_out::refused`), so the launcher refuses the same places. The rest of OneDrive is
/// allowed: a Desktop kept there is where many people unzip things.
fn profile_folders(profile: &Path) -> [(PathBuf, &'static str); 3] {
    [
        (profile.join("Documents"), "Documents"),
        (profile.join("OneDrive\\Documents"), "Documents"),
        (profile.join("AppData"), "AppData"),
    ]
}

/// Which of the player's `folders` the package's folder `root` is in or holds, by the real paths.
fn player_folder_of(root: &Path, folders: &[(PathBuf, &'static str)]) -> Option<&'static str> {
    let root = folder_key(root);
    folders.iter().find_map(|(folder, what)| {
        let key = folder_key(folder);
        (is_inside(&root, &key) || is_inside(&key, &root)).then_some(*what)
    })
}

/// Why the package can't run from where it is.
fn check_location(package: &Package) -> Result<(), Stop> {
    let root = folder_key(&package.root);
    let move_it = "Move the whole nv-rs folder somewhere else, for example C:\\Games\\nv-rs, and start Play.exe again.";
    let temporary = std::env::var_os("TEMP").map(|t| folder_key(Path::new(&t)));
    if temporary.is_some_and(|t| is_inside(&root, &t)) {
        return stop(
            STOP_LOCATION,
            format!(
                "nv-rs is running from a temporary folder, which usually means from inside the ZIP.\n\nExtract the whole ZIP to a folder first (for example C:\\Games\\nv-rs), then start Play.exe there.\n\nThis folder was:\n{}",
                package.root.display()
            ),
        );
    }
    if !package.game_exe.is_file() || !package.tools.join("d3d9-capture.exe").is_file() {
        return stop(
            STOP_BLOCKED,
            format!(
                "A part of nv-rs is missing here: {}\n\nExtract the whole ZIP again. If it is still missing, your antivirus may have removed it: the program is new and not signed (see READ ME.txt).",
                if package.game_exe.is_file() { "app\\tools" } else { "app\\FalloutNV.exe" }
            ),
        );
    }
    let mut folders: Vec<(PathBuf, &str)> = [
        (real_documents(), "Documents"),
        (win::known_folder(0x1a), "AppData"),
        (win::known_folder(0x1c), "AppData"),
    ]
    .into_iter()
    .filter_map(|(folder, what)| Some((folder?, what)))
    .collect();
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        folders.extend(profile_folders(Path::new(&profile)));
    }
    if let Some(what) = player_folder_of(&package.root, &folders) {
        return stop(
            STOP_LOCATION,
            format!(
                "nv-rs is inside your {what} folder, and it keeps away from the game's own files there.\n\n{move_it}\n\nThis folder was:\n{}",
                package.root.display()
            ),
        );
    }
    // The game's own paths (and the tools' start folder) must fit, before anything runs here.
    let documents = package.userdata.join("game\\Documents");
    let length = guest_bytes(&package.root).map_or(package.root.as_os_str().len(), |bytes| {
        bytes.len() + documents.as_os_str().len() - package.root.as_os_str().len()
    });
    if path_too_long(length) {
        return stop(
            STOP_LOCATION,
            format!(
                "This folder's path is too long for the game:\n{}\n\nMove the whole nv-rs folder to a shorter path, for example C:\\Games\\nv-rs.",
                package.root.display()
            ),
        );
    }
    Ok(())
}

/// Whether the package's folder is the game's folder or is inside it or the other way round, by
/// the real paths.
fn inside_game(root: &Path, game: &Path) -> bool {
    let (root, game) = (folder_key(root), folder_key(game));
    is_inside(&root, &game) || is_inside(&game, &root)
}

/// Every check of where the package is that needs the game's folder, then the first write: the
/// folder test and the log. Nothing is written before this passes, and a refusal leaves the disk
/// as it was. `game` is the folder found; with `None` (nothing found, or only saves to import) the
/// folders the game could be in (the last one used, Steam, GOG and Epic) are compared instead.
fn clear_for_writing(package: &Package, game: Option<&Path>) -> Result<(), Stop> {
    let move_it = "Move the whole nv-rs folder somewhere else, for example C:\\Games\\nv-rs, and start Play.exe again.";
    let mut folders: Vec<PathBuf> = Vec::new();
    match game {
        Some(game) => folders.push(game.to_path_buf()),
        None => {
            if let Ok(text) = std::fs::read_to_string(&package.remembered) {
                folders.push(PathBuf::from(text.trim()));
            }
            folders.extend(find::candidates().into_iter().map(|(_, place)| place));
        }
    }
    for folder in &folders {
        if inside_game(&package.root, folder) {
            return stop(
                STOP_LOCATION,
                format!(
                    "nv-rs is inside your game's folder. It must never write there, so it won't run from there.\n\n{move_it}\n\nThis folder was:\n{}",
                    package.root.display()
                ),
            );
        }
    }
    // Writable: not under Program Files without rights, not on a read-only drive.
    let test = package.root.join(".nv-rs-write-test");
    let written = std::fs::create_dir_all(&package.userdata)
        .and_then(|()| std::fs::write(&test, b"x"))
        .and_then(|()| std::fs::remove_file(&test));
    if let Err(e) = written {
        return stop(
            STOP_LOCATION,
            format!(
                "nv-rs can't write in its own folder, where it keeps your saves and settings ({e}).\n\nIt doesn't need administrator rights and shouldn't be given them. {move_it}\n\nThis folder was:\n{}",
                package.root.display()
            ),
        );
    }
    package.start_writing();
    Ok(())
}

/// The game's folder and `Data` folder.
#[derive(Clone)]
struct Game {
    folder: PathBuf,
    data: PathBuf,
}

/// What a program printed, or why it didn't finish.
enum Limited {
    Done(Output),
    TimedOut,
    Failed(std::io::Error),
}

/// Runs `command` to its end, but no longer than `limit`: a program that hangs (a sound or graphics
/// driver can) is ended, so the player gets a message instead of a window that never goes.
fn run_limited(command: &mut Command, limit: Duration) -> Limited {
    let mut child = match command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return Limited::Failed(e),
    };
    fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    }
    let (out, err) = (drain(child.stdout.take()), drain(child.stderr.take()));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Limited::TimedOut;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(e) => return Limited::Failed(e),
        }
    };
    Limited::Done(Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}

/// What the player reads when the game's program can't be opened for reading (a lock or no
/// access), or `None` when it can.
fn exe_read_problem(exe: &Path) -> Option<String> {
    let e = std::fs::File::open(exe).err()?;
    let why = match e.raw_os_error() {
        Some(32 | 33) => "Another program has it open: the game or its launcher still running, or an antivirus scanning it. Close them, wait a moment and start Play.exe again.",
        Some(5) => "Windows does not let this account read it. The folder's permissions may be restrictive, or an antivirus or Windows' \"controlled folder access\" blocks it. Allow Play.exe, or give your account read access to the game's folder.",
        _ => "Windows can't read it.",
    };
    Some(format!(
        "nv-rs can't read the game's program, FalloutNV.exe ({e}):\n{}\n\n{why}",
        exe.display()
    ))
}

/// What `app\FalloutNV.exe --check` says about this copy of the game: its lines, or why it can't run.
fn check_game(package: &Package, game: &Game) -> Result<Vec<String>, Stop> {
    let output = run_limited(
        Command::new(&package.game_exe)
            .arg("--check")
            .arg(&game.data),
        CHECK_LIMIT,
    );
    let output = match output {
        Limited::Done(o) => o,
        Limited::TimedOut => {
            package.log("--check: did not finish");
            return stop(
                STOP_HUNG,
                format!(
                    "nv-rs's check of your game did not finish in {} seconds, so it was ended. This usually means the sound or graphics driver is stuck answering.\n\nRestart the PC and try again. If it happens again, set the sound output to another device (or stereo) in Windows' sound settings, and report it at {ISSUES}.",
                    CHECK_LIMIT.as_secs()
                ),
            );
        }
        Limited::Failed(e) => {
            return stop(
                STOP_BLOCKED,
                format!(
                    "Windows or your antivirus did not let nv-rs start its program ({e}).\n\nThe program is new and not signed, so this can happen: see READ ME.txt. Program:\n{}",
                    package.game_exe.display()
                ),
            )
        }
    };
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect());
    }
    let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
    package.log(&format!("--check: {details}"));
    let exe = game.folder.join("FalloutNV.exe");
    if !exe.is_file() {
        return stop(
            STOP_WRONG_GAME,
            format!(
                "There is no FalloutNV.exe in this game folder:\n{}\n\nnv-rs needs the installed game, version 1.4.0.525.",
                game.folder.display()
            ),
        );
    }
    if let Some(message) = exe_read_problem(&exe) {
        return stop(STOP_BLOCKED, message);
    }
    if details.contains("error: ") && !details.contains("1.4.0.525") {
        return stop(
            STOP_BLOCKED,
            format!(
                "nv-rs's own program did not run ({details}).\n\nThe program is new and not signed, so Windows or an antivirus can block it: see READ ME.txt.\n\nPlease report this at {ISSUES}"
            ),
        );
    }
    let version = win::file_version(&exe);
    let found = version.as_deref().unwrap_or("unknown");
    if version.as_deref() == Some("1.4.0.525") {
        return stop(
            STOP_WRONG_GAME,
            format!(
                "Your game says it is version 1.4.0.525, but its program isn't the build nv-rs was made from (the Steam one). Some GOG, Epic and regional copies, and programs changed by patch tools, differ.\n\nnv-rs can't run on this copy yet, because every address it uses belongs to that one build. Please tell us which edition you have: {ISSUES}\n\nGame folder:\n{}",
                game.folder.display()
            ),
        );
    }
    stop(
        STOP_WRONG_GAME,
        format!(
            "Your game is version {found}, and nv-rs needs version 1.4.0.525 (the last update, with all patches installed).\n\nUpdate the game through the store you bought it from, then start Play.exe again.\n\nGame folder:\n{}",
            game.folder.display()
        ),
    )
}

/// A game whose program passes the check, from `--game`, the remembered folder, the stores or the
/// player. Nothing there is written.
fn find_game(package: &Package, explicit: Option<&Path>) -> Result<(Game, Vec<String>), Stop> {
    let game_at = |folder: &Path| -> Option<Game> {
        let data = find::data_folder(folder)?;
        let folder = data.parent()?.to_path_buf();
        Some(Game { folder, data })
    };
    if let Some(path) = explicit {
        let Some(game) = game_at(path) else {
            return stop(
                STOP_NO_GAME,
                format!(
                    "This folder has no FalloutNV.esm, in itself or in a Data folder inside it:\n{}",
                    path.display()
                ),
            );
        };
        let lines = check_game(package, &game)?;
        return Ok((game, lines));
    }
    let mut places: Vec<(&str, PathBuf)> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(&package.remembered) {
        places.push(("the last time", PathBuf::from(text.trim())));
    }
    places.extend(find::candidates());
    let mut problem: Option<Stop> = None;
    for (from, place) in &places {
        let Some(game) = game_at(place) else {
            continue;
        };
        package.log(&format!(
            "game found through {from}: {}",
            game.folder.display()
        ));
        match check_game(package, &game) {
            Ok(lines) => return Ok((game, lines)),
            Err(why) => {
                package.log(&why.message);
                problem.get_or_insert(why);
            }
        }
    }
    let first = match problem {
        Some(why) if why.code == STOP_BLOCKED || why.code == STOP_HUNG => return Err(why),
        Some(why) => {
            ui::error(&why.message);
            Some(why)
        }
        None => {
            ui::info("nv-rs could not find Fallout: New Vegas on this PC. It looks in Steam, GOG and Epic.\n\nIn the next window, choose the game's folder: the one with FalloutNV.exe and a Data folder in it.");
            None
        }
    };
    loop {
        let Some(picked) = ui::pick_folder(
            "Choose your Fallout: New Vegas folder (with FalloutNV.exe and Data in it)",
        ) else {
            return Err(first.unwrap_or(Stop {
                code: STOP_NO_GAME,
                message: "nv-rs can't start without the game. It needs your own installed copy of Fallout: New Vegas, version 1.4.0.525: Steam, GOG or Epic.".into(),
            }));
        };
        let Some(game) = game_at(&picked) else {
            ui::error(&format!(
                "That folder has no FalloutNV.esm, in itself or in a Data folder inside it:\n{}\n\nChoose the folder Fallout: New Vegas is installed in.",
                picked.display()
            ));
            continue;
        };
        match check_game(package, &game) {
            Ok(lines) => return Ok((game, lines)),
            Err(why) if why.code == STOP_BLOCKED || why.code == STOP_HUNG => return Err(why),
            Err(why) => {
                package.log(&why.message);
                ui::error(&why.message);
            }
        }
    }
}

/// The bytes the game's code gets for one of the package's folders. Its paths go back to Windows
/// as UTF-8 (`engine-platform` files.rs), and it reads them one byte at a time under a one-byte
/// code page. So: plain letters as they are; else the 8.3 name, when Windows has a plain one; else
/// UTF-8 on a one-byte code page. On a Chinese, Japanese or Korean code page the game's C runtime
/// would read UTF-8 as double-byte text, which can swallow a `\`: `None`.
fn guest_bytes(path: &Path) -> Option<Vec<u8>> {
    let text = path.to_str()?;
    if text.is_ascii() {
        return Some(text.as_bytes().to_vec());
    }
    if std::env::var_os("NV_RS_NO_SHORT_PATHS").is_none() {
        if let Some(short) = win::short_path(path) {
            if let Some(short) = short.to_str().filter(|s| s.is_ascii()) {
                return Some(short.as_bytes().to_vec());
            }
        }
    }
    if matches!(win::ansi_code_page(), 932 | 936 | 949 | 950 | 1361 | 54936) {
        return None;
    }
    Some(text.as_bytes().to_vec())
}

/// A file's bytes as text, one character per byte, so writing it back with [`bytes_of`] gives
/// every byte not changed back as it was.
fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// [`text_of`]'s text as the file's bytes again.
fn bytes_of(text: &str) -> Vec<u8> {
    text.chars().map(|c| c as u32 as u8).collect()
}

/// [`tables::write_whole`], keeping the file's read-only mark: the mark is lifted for the write
/// and put back on the new file.
// Windows only: `set_readonly(false)` clears the read-only attribute.
#[allow(clippy::permissions_set_readonly_false)]
fn write_keeping_mark(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let read_only = std::fs::metadata(path).is_ok_and(|m| m.permissions().readonly());
    if read_only {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(path, permissions)?;
    }
    let written = tables::write_whole(path, bytes);
    if read_only {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(path, permissions)?;
    }
    written
}

/// What a start did with one profile's `Fallout.ini` ([`prepare_profile`]).
struct Prepared {
    /// nv-rs's sections are in the file, and with them a `settings.ini` that is being moved in.
    seeded: bool,
}

/// A profile's `Fallout.ini` and `FalloutPrefs.ini`: the player's own copied once, read only from
/// `real` (their `My Games\FalloutNV`: the settings the game's launcher made). nv-rs's own sections
/// go into `Fallout.ini` (`game_ini.rs`): the whole block once, and what a file with only some of
/// them lacks. The player's choices go into them: `pending`, a `settings.ini` that is being moved
/// into the files, whose lines win. Play.exe has no values of its own: what nothing chose is the
/// generated sections' value, the game's default (the borderless window among them).
///
/// Two files get no sections and are said in the log: one with nothing to start from (a file
/// holding only our block would stop the player's own from being copied at a later start), and a
/// UTF-16 file, which is left as it is.
///
/// On a PC where the game's own launcher has never run, this does what it leaves behind: `Fallout.ini`
/// from the game's `Fallout_default.ini` and `FalloutPrefs.ini` from its High preset (`high.ini`), both
/// only read from the game's folder. `sD3DDevice` always names this PC's card: the game hands over to
/// its launcher when it doesn't (`004da570`), and here there is no launcher to hand over to.
fn prepare_profile(
    package: &Package,
    profile: &Profile,
    real: Option<&Path>,
    pending: Option<&Settings>,
    game_folder: &Path,
    device: Option<&str>,
) -> std::io::Result<Prepared> {
    let read_real = |name: &str| real.and_then(|d| std::fs::read(d.join(name)).ok());
    let ini = profile.fallout_ini();
    let old = match std::fs::read(&ini) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    // An empty file is no file: the player's own or the game's defaults are copied, as at a
    // first start.
    let start = match &old {
        Some(bytes) if !bytes.is_empty() => bytes.clone(),
        _ => read_real("Fallout.ini")
            .filter(|bytes| !bytes.is_empty())
            .or_else(|| std::fs::read(game_folder.join("Fallout_default.ini")).ok())
            .unwrap_or_default(),
    };
    let mut prepared = Prepared { seeded: false };
    let mut text = text_of(&start);
    if start.is_empty() {
        package.log(&format!(
            "no Fallout.ini to start from (none of the player's, no Fallout_default.ini in the game's folder): none made at {}",
            ini.display()
        ));
    } else if let Some(seeded) = game_ini::with_sections(&text) {
        text = game_ini::apply_choices(&seeded.text, pending).0;
        prepared = Prepared { seeded: true };
        if seeded.fresh {
            package.log(&format!("nv-rs's own sections added to {}", ini.display()));
        } else if !seeded.added.is_empty() {
            package.log(&format!(
                "nv-rs's own sections completed in {}: {} added",
                ini.display(),
                seeded.added.join(", ")
            ));
        }
    } else {
        package.log(&format!(
            "{} is UTF-16 text: left as it is, without nv-rs's own sections",
            ini.display()
        ));
    }
    let bytes = bytes_of(&text);
    if !bytes.is_empty() && old.as_ref() != Some(&bytes) {
        write_keeping_mark(&ini, &bytes)?;
    }

    let mut first_time_size = None;
    if !profile.prefs.is_file() {
        if let Some(bytes) = read_real("FalloutPrefs.ini") {
            tables::write_whole(&profile.prefs, &bytes)?;
            package.log("copied the player's FalloutPrefs.ini (read only) for the first start");
        } else {
            // The game's window starts at the desktop's size.
            first_time_size = Some(ui::desktop_size());
            let preset = std::fs::read(game_folder.join("high.ini")).unwrap_or_default();
            tables::write_whole(&profile.prefs, &preset)?;
            package.log("no FalloutPrefs.ini to copy: starting from the game's High preset");
        }
    }
    let mut lines = pending.map(Settings::prefs_lines).unwrap_or_default();
    // A borderless window is the game's windowed mode, at every start: by what the file says now
    // (a settings.ini's choice already in it), read as the game reads it, and the game's default
    // when it says nothing usable (`game_ini::borderless`), so Play.exe and the game agree. A
    // file without our sections can't say: then the choice passed to the game decides
    // (`game_ini::switches`), settings.ini's or the same default.
    let borderless = if prepared.seeded {
        game_ini::borderless(&settings::decode(&bytes))
    } else {
        pending
            .and_then(Settings::borderless)
            .unwrap_or_else(|| game_ini::borderless(""))
    };
    if borderless {
        lines.push(("bFull Screen".into(), "0".into()));
    }
    if let Some(device) = device {
        lines.push(("sD3DDevice".into(), format!("\"{device}\"")));
    }
    if let Some((w, h)) = first_time_size {
        if !lines.iter().any(|(n, _)| n == "iSize W") {
            lines.push(("iSize W".into(), w.to_string()));
            lines.push(("iSize H".into(), h.to_string()));
        }
    }
    if lines.is_empty() {
        return Ok(prepared);
    }
    let old: Vec<u8> = std::fs::read(&profile.prefs).unwrap_or_default();
    let new = settings::apply_to_prefs(&text_of(&old), &lines);
    let bytes = bytes_of(&new);
    if bytes != old {
        write_keeping_mark(&profile.prefs, &bytes)?;
    }
    Ok(prepared)
}

/// The name a moved `settings.ini` gets: `settings.ini.migrated`, or with the first free number
/// after it (`settings.ini.migrated.2`, ...) when an earlier one is there. An earlier one is
/// never replaced.
fn migrated_name(settings: &Path) -> PathBuf {
    let mut base = settings.as_os_str().to_owned();
    base.push(".migrated");
    let mut name = PathBuf::from(&base);
    let mut number = 1u32;
    while name.symlink_metadata().is_ok() {
        number += 1;
        let mut numbered = base.clone();
        numbered.push(format!(".{number}"));
        name = PathBuf::from(numbered);
    }
    name
}

/// Both profiles' files made ready ([`prepare_profile`]), so Settings.cmd finds them to edit; a
/// `settings.ini` goes into both and is renamed `settings.ini.migrated` ([`migrated_name`]).
///
/// While a `settings.ini` exists it is the settings for both tools: Settings.cmd reads and
/// writes it, and its lines win here over what the files say. So when it can't be renamed, or a
/// profile's file couldn't take our sections, it stays, the log says so, and the next start
/// moves it in again: the same result every time.
fn prepare_files(
    package: &Package,
    profiles: [&Profile; 2],
    real: Option<&Path>,
    game_folder: &Path,
    device: Option<&str>,
) -> Result<(), Stop> {
    let pending = match std::fs::read(&package.settings) {
        Ok(bytes) => Some(Settings::parse(&settings::decode(&bytes))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return stop(
                STOP_SETTINGS,
                format!(
                    "nv-rs can't read its settings ({e}). Close any program that has it open, or remove it, and start Play.exe again. The file is:\n{}",
                    package.settings.display()
                ),
            )
        }
    };
    if let Some(settings) = &pending {
        settings.game_switches().map_err(|why| Stop {
            code: STOP_SETTINGS,
            message: format!(
                "nv-rs's settings have a line it can't use: {why}.\n\nOpen Settings.cmd, choose again and click Save. The file is:\n{}",
                package.settings.display()
            ),
        })?;
    }
    // settings.ini's lines that are not used any more: the same whatever file they go into.
    let unused = pending
        .as_ref()
        .map(|s| game_ini::apply_choices("", Some(s)).1)
        .unwrap_or_default();
    let mut all_seeded = true;
    for profile in profiles {
        let done = profile.make().and_then(|()| {
            prepare_profile(
                package,
                profile,
                real,
                pending.as_ref(),
                game_folder,
                device,
            )
        });
        match done {
            Ok(prepared) => {
                all_seeded &= prepared.seeded;
            }
            Err(e) => {
                return stop(
                    STOP_LOCATION,
                    format!(
                        "nv-rs could not write the game's settings ({e}):\n{}",
                        profile.fallout_ini().display()
                    ),
                )
            }
        }
    }
    if pending.is_none() {
        return Ok(());
    }
    if !unused.is_empty() {
        package.log(&format!(
            "settings.ini's lines not used any more (left in the file): {}",
            unused.join(", ")
        ));
    }
    if !all_seeded {
        package.log(
            "settings.ini stays where it is: a Fallout.ini could not take its choices (said above). Settings.cmd keeps reading and writing settings.ini, and Play.exe moves it in again at the next start",
        );
        return Ok(());
    }
    let moved = migrated_name(&package.settings);
    let name = moved
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match std::fs::rename(&package.settings, &moved) {
        Ok(()) => package.log(&format!(
            "settings.ini's choices are now in the game's Fallout.ini and FalloutPrefs.ini; the old file is {name}"
        )),
        Err(e) => package.log(&format!(
            "settings.ini's choices are in the game's Fallout.ini and FalloutPrefs.ini, but the file could not be renamed to {name} ({e}). It stays the settings while it is there: Settings.cmd keeps reading and writing it, and Play.exe moves it in again at every start, over what the game's files say. Close whatever has it open, or remove it"
        )),
    }
    Ok(())
}

/// The newest logs stay; older game logs go (only `game-*.log` in the package's own folder).
fn tidy_logs(logs: &Path) {
    let Ok(entries) = std::fs::read_dir(logs) else {
        return;
    };
    let mut names: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("game-") && n.ends_with(".log"))
        })
        .collect();
    names.sort();
    for old in names.iter().rev().skip(10) {
        let _ = std::fs::remove_file(old);
    }
}

/// What the player reads when the game's run ended badly, from the log's end.
fn plain_end(tail: &str, status: Option<i32>) -> String {
    let lines: Vec<&str> = tail.lines().filter(|l| !l.trim().is_empty()).collect();
    let last = lines
        .iter()
        .rev()
        .find(|l| {
            l.contains("Game's main stopped") || l.contains("panicked") || l.starts_with("error:")
        })
        .or(lines.last())
        .copied()
        .unwrap_or("(the log is empty)");
    let has = |what: &str| tail.contains(what);
    let what = if has("the sound table disagrees") || has("the sound table has") {
        "Sound on this PC answers differently from what nv-rs knows, and it stops rather than guess. Changing the sound output device, or setting it to stereo, may get past this."
    } else if has("d3dx9_38.dll") {
        "DirectX 9's D3DX library (d3dx9_38.dll) was missing or did something unexpected. Installing Microsoft's \"DirectX End-User Runtimes (June 2010)\" may help."
    } else if has("was not captured in the adapter table")
        || has("is not in the machine table")
        || has("adapter table")
    {
        "The game asked your graphics card something this PC's table has no answer for, so it stopped. That is a gap in nv-rs, and your report is what fills it."
    } else if has("CreateDevice answered") {
        "Your graphics card refused the game's display settings. Try Settings.cmd: Window, a lower resolution, anti-aliasing off."
    } else if has("the game's window") && has("not provided") {
        "The game asked Windows for something nv-rs doesn't provide yet."
    } else {
        "This preview has bugs left, and the game's code stopped at one of them."
    };
    // Drawn on Vulkan (wgpu): the original's Direct3D 9 is one setting away.
    let renderer = if has("NV_D3D9=wgpu: drawing with") || has("NV_D3D9 wgpu: the game's device") {
        "\n\nIt was drawn on Vulkan. If it stops again there, Settings.cmd's Renderer \"Direct3D 9\" draws the game as the original does."
    } else {
        ""
    };
    format!(
        "The game stopped (status {}).\n\n{what}{renderer}\n\nPlease report it at {ISSUES} and attach the log named below. It may show your Windows user name in folder paths.\n\nLast line of the log:\n{last}",
        status.map_or("unknown".to_string(), |s| s.to_string())
    )
}

/// The last 64 KB of the log, as text.
fn log_tail(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    let from = bytes.len().saturating_sub(64 * 1024);
    String::from_utf8_lossy(&bytes[from..]).into_owned()
}

struct Options {
    mods: bool,
    import: bool,
    check_only: bool,
    game: Option<PathBuf>,
    from: Option<PathBuf>,
}

fn parse_options(raw: &[String]) -> Options {
    let mut options = Options {
        mods: false,
        import: false,
        check_only: false,
        game: std::env::var_os("NV_RS_GAME").map(PathBuf::from),
        from: None,
    };
    let mut args = raw.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mods" => options.mods = true,
            "--import-saves" => options.import = true,
            "--check" => options.check_only = true,
            "--game" => options.game = args.next().map(PathBuf::from),
            "--from" => options.from = args.next().map(PathBuf::from),
            _ => {}
        }
    }
    options
}

/// `--import-saves`: the player's saves copied into the package, never the other way, never over a
/// save already there.
fn import_saves(package: &Package, profile: &Profile, from: Option<&Path>) -> Result<i32, Stop> {
    let source = match from {
        Some(path) => path.to_path_buf(),
        None => match real_documents() {
            Some(docs) => docs.join("My Games\\FalloutNV\\Saves"),
            None => {
                return stop(
                    STOP_LOCATION,
                    "Windows did not say where your Documents folder is.",
                )
            }
        },
    };
    let saves: Vec<PathBuf> = std::fs::read_dir(&source)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("fos"))
                })
                .collect()
        })
        .unwrap_or_default();
    if saves.is_empty() {
        let text = format!("No saves were found in:\n{}", source.display());
        package.log(&text);
        ui::info(&text);
        return Ok(0);
    }
    if !ui::ask(&format!(
        "Copy {} saves into nv-rs's own folder?\n\nFrom (only read, nothing there changes):\n{}\n\nTo:\n{}\n\nA save already there stays as it is. A save made with mods needs those mods: use Play with mods for it.",
        saves.len(),
        source.display(),
        profile.saves.display()
    )) {
        return Ok(0);
    }
    profile.make().map_err(|e| Stop {
        code: STOP_LOCATION,
        message: format!("{}: {e}", profile.saves.display()),
    })?;
    let (mut copied, mut kept) = (0, 0);
    for save in &saves {
        let Some(name) = save.file_name() else {
            continue;
        };
        let target = profile.saves.join(name);
        if target.exists() {
            kept += 1;
            continue;
        }
        let mut temporary = target.as_os_str().to_owned();
        temporary.push(".part");
        // `copy` only reads the source, and keeps its date (the game sorts saves by date).
        let done =
            std::fs::copy(save, &temporary).and_then(|_| std::fs::rename(&temporary, &target));
        match done {
            Ok(()) => copied += 1,
            Err(e) => package.log(&format!("{}: {e}", save.display())),
        }
    }
    let text = format!("Saves copied: {copied}. Already there, left as they were: {kept}.");
    package.log(&format!("import from {}: {text}", source.display()));
    ui::info(&text);
    Ok(0)
}

fn run(package: &Package, options: &Options) -> Result<i32, Stop> {
    check_location(package)?;
    let profile = Profile::at(
        &package.userdata,
        if options.mods { "mods" } else { "game" },
    );
    if options.import {
        clear_for_writing(package, None)?;
        return import_saves(package, &profile, options.from.as_deref());
    }
    if win::another_is_running() {
        return stop(
            STOP_LOCATION,
            "nv-rs is already running. Only one game at a time.",
        );
    }
    let status = ui::Status::show("Looking for Fallout: New Vegas...");
    let result = play(package, options, &profile, &status);
    status.close();
    result
}

fn play(
    package: &Package,
    options: &Options,
    profile: &Profile,
    status: &ui::Status,
) -> Result<i32, Stop> {
    let (mut game, check_lines) = find_game(package, options.game.as_deref())?;
    // The first write of the run: the game's folder is known and the package is not in it.
    clear_for_writing(package, Some(&game.folder))?;
    for line in &check_lines {
        package.log(line);
    }
    let _ = std::fs::write(
        &package.remembered,
        game.folder.to_string_lossy().as_bytes(),
    );
    // DirectX 9's D3DX is not part of Windows; the game's installer adds it.
    let system = std::env::var_os("NV_RS_SYSTEM_FOLDER")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("SystemRoot").map(|r| PathBuf::from(r).join("System32")))
        .unwrap_or_else(|| PathBuf::from("C:\\Windows\\System32"));
    if !system.join("d3dx9_38.dll").is_file() {
        return stop(
            STOP_DIRECTX,
            "DirectX 9's D3DX library (64-bit d3dx9_38.dll) is not on this PC. Windows doesn't include it, and nv-rs uses it for the game's shaders and textures.\n\nInstall \"DirectX End-User Runtimes (June 2010)\" from Microsoft's download site, then start Play.exe again. It is safe alongside newer DirectX.",
        );
    }

    profile.make().map_err(|e| Stop {
        code: STOP_LOCATION,
        message: format!(
            "nv-rs could not make its folders ({e}):\n{}",
            package.userdata.display()
        ),
    })?;
    std::fs::create_dir_all(&package.machine).map_err(|e| Stop {
        code: STOP_LOCATION,
        message: format!("{}: {e}", package.machine.display()),
    })?;

    // Mods: a whole install of links and copies in the package.
    if options.mods {
        status.set("Getting the mods ready...");
        let install = package.userdata.join("modgame");
        let plugins = profile.local.join("FalloutNV\\plugins.txt");
        let built =
            mods::build_install(&game.folder, &package.root.join("mods"), &install, &plugins)
                .map_err(|e| Stop {
                    code: STOP_MODS,
                    message: e.player_text(),
                })?;
        package.log(&format!(
            "mods: {} files linked, {} mod files",
            built.linked, built.mod_files
        ));
        if built.extender_plugins {
            ui::warning("Some of your mods include plugins for xNVSE or other script extenders. Those plugins are not used yet, and mods that need them may not work or may stop the game.");
        }
        game = Game {
            data: install.join("Data"),
            folder: install,
        };
    }

    // The tables.
    status.set("Reading your graphics card...");
    let tables_log = package.machine.join("tables.log");
    let adapter = package.machine.join("d3d9_adapter.tsv");
    let game_exe = game.folder.join("FalloutNV.exe");
    let table = match tables::make_adapter_table(&package.tools, &game_exe, &tables_log) {
        Ok(bytes) => {
            tables::write_whole(&adapter, &bytes).map_err(|e| Stop {
                code: STOP_TABLES,
                message: format!("{}: {e}", adapter.display()),
            })?;
            bytes
        }
        Err(why) => {
            package.log(&format!("the graphics table was not made: {why}"));
            match std::fs::read(&adapter) {
                Ok(bytes) if !why.contains("hardware") => {
                    package.log("using the last good graphics table");
                    bytes
                }
                _ => {
                    return stop(
                        STOP_TABLES,
                        format!(
                            "nv-rs could not read your graphics card: {why}.\n\nThe game needs hardware Direct3D 9 from a graphics driver on this PC. Over Remote Desktop, or with no driver installed, there is none. Update the graphics driver and try again.\n\nIf it still happens, please report it at {ISSUES} with this file:\n{}",
                            tables_log.display()
                        ),
                    )
                }
            }
        }
    };

    status.set("Reading your sound device. The first time this takes about 20 seconds...");
    let sound = package.machine.join("dsound.tsv");
    let fingerprint = tables::sound_fingerprint();
    let stamp = package.machine.join("dsound.machine.txt");
    match tables::make_sound_table(&package.tools, &sound, &stamp, &fingerprint, &tables_log) {
        Ok(made) => {
            package.log(if made {
                "the sound table was made"
            } else {
                "the sound table is still right"
            });
            if made {
                sound_notes(package, &sound, &check_lines);
            }
        }
        Err(why) if sound.is_file() => {
            package.log(&format!(
                "the sound table was not made again ({why}): using the last one"
            ));
        }
        Err(why) => {
            package.log(&format!("the sound table was not made: {why}"));
            // What DirectSound answers without a device (DSERR_NODRIVER): the game's own way of
            // going on without sound.
            let _ = tables::write_whole(&sound, b"DsCreate\t\t88780078\t\nDsEnd\n");
            let _ = std::fs::remove_file(&stamp);
            ui::warning(&format!(
                "nv-rs could not read your sound device ({why}), so the game will start without sound.\n\nIf you have sound normally, please report this at {ISSUES} and attach:\n{}",
                tables_log.display()
            ));
        }
    }

    // The game's folders in the package.
    let refuse_path = |path: &Path| {
        Stop {
        code: STOP_LOCATION,
        message: format!(
            "The game can't use this folder's path, because of the letters in it on this Windows' language setting:\n{}\n\nMove the whole nv-rs folder to a path with plain letters only, for example C:\\Games\\nv-rs.",
            path.display()
        ),
    }
    };
    let documents =
        guest_bytes(&profile.documents).ok_or_else(|| refuse_path(&profile.documents))?;
    let roaming = guest_bytes(&profile.roaming).ok_or_else(|| refuse_path(&profile.roaming))?;
    let local = guest_bytes(&profile.local).ok_or_else(|| refuse_path(&profile.local))?;
    if path_too_long(documents.len()) {
        return stop(
            STOP_LOCATION,
            format!(
                "This folder's path is too long for the game:\n{}\n\nMove the whole nv-rs folder to a shorter path, for example C:\\Games\\nv-rs.",
                package.root.display()
            ),
        );
    }
    let data_argument = match game.data.to_str() {
        Some(text) if text.is_ascii() => game.data.clone(),
        _ => match guest_bytes(&game.data) {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(text) => PathBuf::from(text),
                Err(_) => game.data.clone(),
            },
            None => return Err(refuse_path(&game.data)),
        },
    };
    let device = tables::adapter_name(&table);
    let table = tables::with_folders(&table, &documents, &roaming, &local).map_err(|why| Stop {
        code: STOP_TABLES,
        message: format!(
            "nv-rs could not prepare the game's folders: {why}. Please report this at {ISSUES}."
        ),
    })?;
    tables::write_whole(&profile.table, &table).map_err(|e| Stop {
        code: STOP_TABLES,
        message: format!("{}: {e}", profile.table.display()),
    })?;

    // Both ways of playing get their files, so Settings.cmd can edit them; the profile not played
    // takes the game's own folder's defaults (the mods install's are the same files).
    let other = Profile::at(
        &package.userdata,
        if options.mods { "game" } else { "mods" },
    );
    let real = real_documents().map(|d| d.join("My Games\\FalloutNV"));
    prepare_files(
        package,
        [profile, &other],
        real.as_deref(),
        &game.folder,
        device.as_deref(),
    )?;

    if options.check_only {
        let mut text = format!("Game: {}\n", game.folder.display());
        for line in &check_lines {
            text.push_str(line);
            text.push('\n');
        }
        text.push_str("\nTables made. Everything is ready to play.");
        package.log(&text);
        ui::info(&text);
        return Ok(0);
    }

    // The run.
    status.set("Starting the game...");
    let (_, stamp_text) = win::now();
    tidy_logs(&package.logs);
    let log_path = package.logs.join(format!("game-{stamp_text}.log"));
    let log_file = std::fs::File::create(&log_path).map_err(|e| Stop {
        code: STOP_LOCATION,
        message: format!("{}: {e}", log_path.display()),
    })?;
    let log_clone = log_file.try_clone().map_err(|e| Stop {
        code: STOP_LOCATION,
        message: format!("{}: {e}", log_path.display()),
    })?;
    // The player's options are in the profile's Fallout.ini, which the game reads itself (host
    // `game_ini.rs`): in play no variable is passed for them, so nothing here can beat the file
    // (`pass_switches`, below).
    let mut command = Command::new(&package.game_exe);
    command
        .arg(&data_argument)
        .current_dir(&package.userdata)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_clone))
        .creation_flags(CREATE_NO_WINDOW)
        .env("NV_SOUND", "out")
        .env("NV_D3D9_ADAPTER", &profile.table)
        .env("NV_DSOUND_TABLE", &sound)
        .env("NV_D3DX_CACHE", package.machine.join("d3dx-cache"))
        // wgpu's compiled pipelines (Vulkan, the default renderer), beside the other machine files.
        // Who draws is the profile's Fallout.ini's `sRenderer` (the game reads it; `switches`).
        .env("NV_WGPU_CACHE", package.machine.join("wgpu-cache"))
        .env("NV_SAVE_OUT", &profile.saves)
        .env("NV_PREFS_OUT", &profile.prefs)
        .env("NV_REPORT_DIR", &package.reports)
        .env("NV_REPORT_LOG", &log_path)
        .env("NV_SETTINGS", &package.settings);

    let fallout_ini = text_of(&std::fs::read(profile.fallout_ini()).unwrap_or_default());
    // A settings.ini still there (its choices could not go into the files) is read the same way
    // `prepare_files` read it, which already stopped the start for a line the game can't use.
    let pending = std::fs::read(&package.settings)
        .ok()
        .map(|bytes| Settings::parse(&settings::decode(&bytes)));
    let switches = game_ini::switches(&fallout_ini, pending.as_ref(), |name| {
        std::env::var_os(name).is_some()
    });
    if let Some(line) = pass_switches(&mut command, &switches) {
        package.log(&line);
    }
    let key = |section: &str, key: &str| {
        settings::ini_value(&fallout_ini, section, key)
            .unwrap_or("")
            .to_string()
    };
    package.log(&format!(
        "starting the game on {}; Fallout.ini: preset {}, renderer {}, frame limit {}, menu frame limit {}; log {}",
        data_argument.display(),
        key("nv-rs General", "sPreset"),
        key("nv-rs Display", "sRenderer"),
        key("nv-rs Display", "iFrameLimit"),
        key("nv-rs Display", "iMenuFrameLimit"),
        log_path.display()
    ));
    let mut child = command.spawn().map_err(|e| Stop {
        code: STOP_BLOCKED,
        message: format!(
            "Windows or your antivirus did not let nv-rs start its program ({e}).\n\nThe program is new and not signed, so this can happen: see READ ME.txt. Program:\n{}",
            package.game_exe.display()
        ),
    })?;
    let mut waiting_for_window = true;
    let exit = loop {
        match child.try_wait() {
            Ok(Some(code)) => break code,
            Ok(None) => {}
            Err(e) => {
                package.log(&format!("waiting for the game: {e}"));
                break child.wait().map_err(|e| Stop {
                    code: STOP_GAME_STOPPED,
                    message: e.to_string(),
                })?;
            }
        }
        if waiting_for_window && ui::game_window_is_there() {
            waiting_for_window = false;
            status.close();
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    status.close();
    package.log(&format!("the game ended with status {:?}", exit.code()));
    if exit.success() {
        return Ok(0);
    }
    let message = plain_end(&log_tail(&log_path), exit.code());
    package.log(&message);
    let text = format!(
        "{message}\n\nLog:\n{}\n\nOpen the logs folder?",
        log_path.display()
    );
    if ui::windows_allowed() && ui::ask(&text) {
        ui::open_folder(&package.logs);
    }
    Ok(STOP_GAME_STOPPED)
}

/// The game's variables for the player's options on its command ([`game_ini::switches`]).
///
/// In play the eight that would beat the profile's `Fallout.ini` are taken out, not even one left
/// from Play.exe's own environment. When a variable set before the start makes the run a test,
/// which never reads the file, nothing is taken out and the file's choices are passed on, so the
/// run plays like the player's game; the line for the log says which variable did it, what was
/// passed and what was kept. When the played profile's file has none of our sections, the
/// player's choices (settings.ini's, else the play defaults) are passed in play too, and the
/// line says so.
fn pass_switches(command: &mut Command, switches: &game_ini::Switches) -> Option<String> {
    for name in &switches.removed {
        command.env_remove(name);
    }
    for (name, value) in &switches.passed {
        command.env(name, value);
    }
    let passed: Vec<String> = switches
        .passed
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    let list = |names: &[String]| {
        if names.is_empty() {
            "none".to_string()
        } else {
            names.join(", ")
        }
    };
    let kept: Vec<String> = switches.kept.iter().map(|name| name.to_string()).collect();
    let Some(test) = switches.test else {
        return switches.no_sections.then(|| {
            format!(
                "the played profile's Fallout.ini has no nv-rs sections (said above), so the game can't read the player's settings there: passed as variables instead, from settings.ini where it has a line, else the defaults: {}",
                list(&passed)
            )
        });
    };
    Some(format!(
        "{test} was set before Play.exe started, which makes this run a test: the game does not read Fallout.ini's nv-rs settings in a test. Passed on from the file (from settings.ini when the file has no nv-rs sections), so the game plays as the player's does: {}. Set before the start and kept as they were: {}",
        list(&passed),
        list(&kept)
    ))
}

/// What the sound table and the game's own check say about sound, once when the table is made.
fn sound_notes(package: &Package, sound: &Path, check_lines: &[String]) {
    let table = std::fs::read_to_string(sound).unwrap_or_default();
    if tables::sound_state(&table) == Ok(tables::Sound::NoDevice) {
        package.log("no sound device");
        ui::warning("Windows reports no sound device, so the game will start without sound.\n\nPlug in or turn on a sound device and start Play.exe again: nv-rs notices the change and reads the new device.");
        return;
    }
    for line in check_lines {
        if let Some((_, why)) = line.split_once("not provided: ") {
            package.log(&format!("sound layout: {why}"));
            ui::warning(&format!(
                "The game will have no sound on your current sound device:\n{why}.\n\nIn Windows' sound settings, set the device to stereo, 5.1 or 7.1, or choose another device, then start Play.exe again."
            ));
        }
    }
}

fn main() {
    let raw: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let options = parse_options(&raw);
    let root = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let package = Package::at(root);
    package.log(&format!(
        "Play.exe {:?} in {}; Windows {}; code page {}",
        raw,
        package.root.display(),
        win::windows_build(),
        win::ansi_code_page()
    ));
    match run(&package, &options) {
        Ok(code) => std::process::exit(code),
        Err(why) => {
            // A stop before the first write keeps the disk untouched, unless the package's place is
            // fine (nothing found, wrong version...): then the log is for the player's report.
            if !package.writing.get() && why.code != STOP_LOCATION {
                let _ = clear_for_writing(&package, None);
            }
            package.log(&format!("stopped ({}): {}", why.code, why.message));
            ui::error(&why.message);
            std::process::exit(why.code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_compare_without_case_and_whole_folders_only() {
        let key = |p: &str| folder_key(Path::new(p));
        assert!(is_inside(
            &key("C:/Games/FNV/nv-rs"),
            &key("c:\\games\\fnv\\")
        ));
        assert!(is_inside(&key("C:\\Games\\FNV"), &key("C:\\Games\\fnv")));
        assert!(!is_inside(&key("C:\\Games\\FNV2"), &key("C:\\Games\\FNV")));
    }

    #[test]
    fn a_long_path_is_refused_before_the_game_runs_out_of_room() {
        assert!(!path_too_long(170));
        assert!(path_too_long(180));
    }

    #[test]
    fn the_players_message_names_what_happened() {
        let tail = "Game code: ok\nsomething\nGame's main stopped in frame 12: open function 00123456\nmore\n";
        let text = plain_end(tail, Some(3));
        assert!(text.contains("status 3"));
        assert!(text.contains("Game's main stopped in frame 12"));
        assert!(plain_end(
            "panicked: the sound table disagrees with the layer",
            Some(3)
        )
        .contains("Sound on this PC"));
        assert!(plain_end("IDirect3D9::CheckDeviceFormat(0 1) was not captured in the adapter table: not provided", Some(3))
            .contains("graphics card"));
        assert!(plain_end(
            "NV_D3D9=real: CreateDevice answered 0x8876086a: nothing is forwarded",
            None
        )
        .contains("Settings.cmd"));
        assert!(plain_end("", None).contains("the log is empty"));
        // Drawn on Vulkan: the fallback renderer is named; on the real Direct3D 9 it is not.
        let vulkan =
            "NV_D3D9=wgpu: drawing with a card (Vulkan)\nGame's main stopped in frame 9: x\n";
        assert!(plain_end(vulkan, Some(3)).contains("Renderer \"Direct3D 9\""));
        assert!(!plain_end(tail, Some(3)).contains("Renderer"));
    }

    #[test]
    fn arguments_are_read() {
        let raw: Vec<String> = [
            "--mods",
            "--game",
            "D:\\x",
            "--import-saves",
            "--from",
            "E:\\y",
            "--other",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let o = parse_options(&raw);
        assert!(o.mods && o.import && !o.check_only);
        assert_eq!(o.game.unwrap(), PathBuf::from("D:\\x"));
        assert_eq!(o.from.unwrap(), PathBuf::from("E:\\y"));
    }

    #[test]
    fn only_old_game_logs_go() {
        let dir = std::env::temp_dir().join(format!("nv-rs-logs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..14 {
            std::fs::write(dir.join(format!("game-2026{i:02}.log")), b"x").unwrap();
        }
        std::fs::write(dir.join("launcher.log"), b"x").unwrap();
        tidy_logs(&dir);
        assert!(!dir.join("game-202600.log").exists());
        assert!(!dir.join("game-202603.log").exists());
        assert!(dir.join("game-202604.log").exists());
        assert!(dir.join("launcher.log").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nv-rs-launcher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `link` made a junction to `target` (`mklink /J`, no rights needed).
    fn junction(link: &Path, target: &Path) {
        let made = Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
    }

    #[test]
    fn a_junction_and_a_short_name_are_the_same_place() {
        let dir = scratch("junction");
        let real = dir.join("Real Game Folder");
        std::fs::create_dir_all(real.join("nv-rs")).unwrap();
        let link = dir.join("link");
        junction(&link, &real);
        assert_eq!(folder_key(&link), folder_key(&real));
        assert!(inside_game(&real.join("nv-rs"), &link));
        assert!(inside_game(&link.join("nv-rs"), &real));
        assert!(!inside_game(&dir.join("elsewhere"), &real));
        // The 8.3 name, when this drive has one.
        if let Some(short) = win::short_path(&real) {
            assert_eq!(folder_key(&short), folder_key(&real));
        }
        // A part that doesn't exist yet follows the real path of the part that does.
        assert_eq!(
            folder_key(&link.join("not yet")),
            format!("{}\\not yet", folder_key(&real))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_refusal_before_the_first_write_leaves_the_disk_untouched() {
        let dir = scratch("untouched");
        let game = dir.join("game");
        let root = game.join("nv-rs");
        std::fs::create_dir_all(&root).unwrap();
        let link = dir.join("link");
        junction(&link, &game);
        let package = Package::at(root.clone());
        package.log("held in memory");
        for found in [game.clone(), link.clone()] {
            let why = clear_for_writing(&package, Some(&found)).err().unwrap();
            assert_eq!(why.code, STOP_LOCATION);
            assert!(why.message.contains("inside your game's folder"));
        }
        package.log("still in memory");
        assert!(!package.writing.get());
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            0,
            "nothing written"
        );
        // Where it is fine, the first write comes after the checks, with the kept lines first.
        let other = dir.join("other game");
        std::fs::create_dir_all(&other).unwrap();
        clear_for_writing(&package, Some(&other)).unwrap();
        package.log("written");
        let log = std::fs::read_to_string(package.logs.join("launcher.log")).unwrap();
        let lines: Vec<&str> = log
            .lines()
            .map(|l| l.split_once(' ').unwrap().1.split_once(' ').unwrap().1)
            .collect();
        assert_eq!(lines, ["held in memory", "still in memory", "written"]);
        assert!(!root.join(".nv-rs-write-test").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_program_that_hangs_is_ended_and_one_that_ends_is_read() {
        let mut slow = Command::new("cmd");
        slow.args(["/c", "ping -n 30 127.0.0.1 >nul"]);
        let start = Instant::now();
        assert!(matches!(
            run_limited(&mut slow, Duration::from_millis(700)),
            Limited::TimedOut
        ));
        assert!(start.elapsed() < Duration::from_secs(10));
        let mut quick = Command::new("cmd");
        quick.args(["/c", "echo hello"]);
        match run_limited(&mut quick, Duration::from_secs(30)) {
            Limited::Done(o) => {
                assert!(o.status.success());
                assert!(String::from_utf8_lossy(&o.stdout).contains("hello"));
            }
            _ => panic!("did not finish"),
        }
        assert!(matches!(
            run_limited(
                &mut Command::new("C:\\no\\such\\program.exe"),
                Duration::from_secs(5)
            ),
            Limited::Failed(_)
        ));
    }

    #[test]
    fn a_game_program_that_is_locked_says_so() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = scratch("locked");
        let exe = dir.join("FalloutNV.exe");
        std::fs::write(&exe, b"x").unwrap();
        assert!(exe_read_problem(&exe).is_none());
        let held = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&exe)
            .unwrap();
        let text = exe_read_problem(&exe).unwrap();
        assert!(text.contains("Another program has it open"), "{text}");
        assert!(!text.contains("antivirus blocks nv-rs"));
        drop(held);
        let text = exe_read_problem(&dir.join("missing.exe")).unwrap();
        assert!(text.contains("can't read"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A scratch package (`userdata`) and game folder with made-up INI text (not the game's own),
    /// and the player's `My Games\FalloutNV` with `Fallout.ini` when `player` is given.
    struct Files {
        dir: PathBuf,
        package: Package,
        game: PathBuf,
        real: PathBuf,
    }

    const DEFAULT_INI: &[u8] = b"[General]\r\nsTestName=Default\r\n\r\n[Display]\r\niPresentInterval=1\r\nfTest=1.0000\r\n";

    impl Files {
        fn new(name: &str, player: Option<&[u8]>) -> Files {
            let dir = scratch(name);
            let game = dir.join("game");
            std::fs::create_dir_all(&game).unwrap();
            std::fs::write(game.join("Fallout_default.ini"), DEFAULT_INI).unwrap();
            std::fs::write(game.join("high.ini"), b"[Display]\r\niSize W=800\r\n").unwrap();
            let real = dir.join("Documents\\My Games\\FalloutNV");
            std::fs::create_dir_all(&real).unwrap();
            if let Some(bytes) = player {
                std::fs::write(real.join("Fallout.ini"), bytes).unwrap();
            }
            let package = Package::at(dir.join("nv-rs"));
            Files {
                package,
                game,
                real,
                dir,
            }
        }

        fn profiles(&self) -> [Profile; 2] {
            ["game", "mods"].map(|name| Profile::at(&self.package.userdata, name))
        }

        fn start(&self) -> Result<(), Stop> {
            let [game, mods] = self.profiles();
            prepare_files(
                &self.package,
                [&game, &mods],
                Some(&self.real),
                &self.game,
                Some("Test Card"),
            )
        }

        fn ini(&self, profile: usize) -> Vec<u8> {
            std::fs::read(self.profiles()[profile].fallout_ini()).unwrap()
        }

        fn prefs(&self, profile: usize) -> String {
            text_of(&std::fs::read(&self.profiles()[profile].prefs).unwrap())
        }

        /// What the launcher logged so far (kept in memory: these tests never open the log).
        fn log(&self) -> String {
            self.package.held.borrow().join("\n")
        }

        /// The moved settings.ini files, by name.
        fn migrated(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.package.userdata)
                .unwrap()
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with("settings.ini."))
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Files {
        // Windows only: this clears the read-only attribute.
        #[allow(clippy::permissions_set_readonly_false)]
        fn drop(&mut self) {
            for profile in self.profiles() {
                for file in [profile.fallout_ini(), profile.prefs.clone()] {
                    if let Ok(m) = std::fs::metadata(&file) {
                        let mut p = m.permissions();
                        p.set_readonly(false);
                        let _ = std::fs::set_permissions(&file, p);
                    }
                }
            }
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// The sections as generated (the game's defaults: Play.exe changes no value) in the line ends
    /// `eol`, then an empty line.
    fn our_block(eol: &str) -> Vec<u8> {
        let mut out = String::new();
        for line in game_ini::SECTIONS.lines() {
            out.push_str(line);
            out.push_str(eol);
        }
        out.push_str(eol);
        out.into_bytes()
    }

    #[test]
    fn a_new_players_file_from_fallout_default_ini_gets_the_sections_once() {
        let files = Files::new("seed-default", None);
        files.start().unwrap();
        for profile in 0..2 {
            let ini = files.ini(profile);
            let mut want = our_block("\r\n");
            want.extend_from_slice(DEFAULT_INI);
            assert_eq!(
                ini, want,
                "the default file byte for byte under the sections"
            );
            // A new player starts borderless: the generated sections' own line, from the table.
            assert_eq!(
                settings::ini_value(&text_of(&ini), "nv-rs Display", "bBorderless"),
                Some("1")
            );
            let prefs = files.prefs(profile);
            assert!(prefs.contains("bFull Screen=0") && prefs.contains("sD3DDevice=\"Test Card\""));
        }
        // A second start changes nothing.
        let before = (files.ini(0), files.ini(1), files.prefs(0));
        files.start().unwrap();
        assert_eq!((files.ini(0), files.ini(1), files.prefs(0)), before);
        assert!(!files.package.settings.exists());
    }

    /// A `bBorderless` line that chooses nothing (empty, or a word the game can't use) is the
    /// game's default, borderless, as the host reads it: Play.exe leaves the line as it is and
    /// keeps the game's own full-screen mode off for it, as for 1; Settings.cmd shows it as
    /// borderless, so saving that changes no byte, and choosing Window writes 0, which then stays
    /// and gets no such push.
    #[test]
    fn a_borderless_line_that_says_nothing_takes_the_games_default() {
        for (name, value) in [("empty", ""), ("word", "yes")] {
            let ini = text_of(&our_block("\r\n"))
                .replace("bBorderless=1", &format!("bBorderless={value}"))
                + "[General]\r\nx=1\r\n";
            assert!(ini.contains(&format!("\r\nbBorderless={value}\r\n")));
            let files = Files::new(&format!("borderless-{name}"), Some(ini.as_bytes()));
            files.start().unwrap();
            for profile in 0..2 {
                assert_eq!(text_of(&files.ini(profile)), ini);
                assert!(files.prefs(profile).contains("bFull Screen=0"), "{name}");
            }
            settings_window(&files, "");
            settings_window(&files, "Borderless=1");
            assert_eq!(text_of(&files.ini(0)), ini, "{name}");
            settings_window(&files, "Borderless=0");
            let chosen = text_of(&files.ini(0));
            assert_eq!(
                chosen,
                ini.replace(&format!("bBorderless={value}\r\n"), "bBorderless=0\r\n")
            );
            files.start().unwrap();
            assert_eq!(text_of(&files.ini(0)), chosen);
            assert_eq!(text_of(&files.ini(1)), chosen);
        }
        // A file with 0 gets no push: a fresh profile's prefs stay without the key.
        let ini = text_of(&our_block("\r\n")).replace("bBorderless=1", "bBorderless=0");
        let files = Files::new("borderless-off", Some(ini.as_bytes()));
        files.start().unwrap();
        assert!(!files.prefs(0).contains("bFull Screen"));
    }

    #[test]
    fn the_players_own_file_is_copied_once_with_the_sections_and_its_line_ends() {
        // LF ends and bytes that are not UTF-8 come through as they were.
        let player = b"[General]\nsTestName=Caf\xe9\n[Display]\nfTest=2.0000\n";
        let files = Files::new("seed-player", Some(player));
        files.start().unwrap();
        let mut want = our_block("\n");
        want.extend_from_slice(player);
        assert_eq!(files.ini(0), want);
        // The player's file changes later: the profile's copy is not made again.
        std::fs::write(files.real.join("Fallout.ini"), b"[General]\r\nsOther=1\r\n").unwrap();
        files.start().unwrap();
        assert_eq!(files.ini(0), want);
        assert_eq!(
            std::fs::read(files.real.join("Fallout.ini")).unwrap(),
            b"[General]\r\nsOther=1\r\n"
        );
    }

    #[test]
    fn a_file_that_has_the_sections_is_left_as_it_is() {
        // The whole block, with the player's own values in it.
        let mut player = text_of(&our_block("\r\n"))
            .replace("sPreset=fixes", "sPreset=vanilla")
            .replace("bBorderless=1", "bBorderless=0")
            .into_bytes();
        player.extend_from_slice(b"[General]\r\nx=1\r\n");
        let files = Files::new("seed-has", Some(&player));
        files.start().unwrap();
        assert_eq!(files.ini(0), player);
        assert!(!files.prefs(0).contains("bFull Screen"));
        assert!(!files.log().contains("completed"));
    }

    /// A file with one section of ours made by hand (or left half made by an older tool): the
    /// rest of the block is added, the borderless window as for a new file, and nothing that was
    /// there changes. No state leaves a file half seeded.
    #[test]
    fn a_file_with_one_hand_made_section_gets_the_rest_of_the_block() {
        let player = b"[General]\r\nsTestName=Hand\r\n\r\n[nv-rs Display]\r\niFrameLimit=60\r\n";
        let files = Files::new("seed-hand", Some(player));
        files.start().unwrap();
        for profile in 0..2 {
            let ini = text_of(&files.ini(profile));
            assert!(ini.starts_with(&text_of(player)), "{ini}");
            let get = |s: &str, k: &str| settings::ini_value(&ini, s, k).map(str::to_string);
            assert_eq!(get("nv-rs Display", "iFrameLimit").as_deref(), Some("60"));
            assert_eq!(get("nv-rs Display", "bBorderless").as_deref(), Some("1"));
            assert_eq!(
                get("nv-rs Display", "iMenuFrameLimit").as_deref(),
                Some("120")
            );
            assert_eq!(get("nv-rs General", "sPreset").as_deref(), Some("fixes"));
            assert_eq!(get("nv-rs Threads", "bRenderThread").as_deref(), Some("1"));
            assert!(ini.contains("\r\n\r\n[nv-rs Fixes]\r\n; Each fix follows sPreset."));
            assert!(ini.ends_with(";bNVSEListGuard=1\r\n"));
            assert!(files.prefs(profile).contains("bFull Screen=0"));
        }
        assert!(files.log().contains("nv-rs's own sections completed in"));
        // A second start changes nothing.
        let before = (files.ini(0), files.ini(1), files.prefs(0));
        files.start().unwrap();
        assert_eq!((files.ini(0), files.ini(1), files.prefs(0)), before);
    }

    /// A profile from before the sections (the published preview's), and Settings.cmd opened
    /// before Play.exe: it must not write a key into that Fallout.ini, which Play.exe would then
    /// take for one that has the sections. It writes settings.ini, and Play.exe puts the whole
    /// block and the choices into both profiles.
    #[test]
    fn settings_cmd_on_a_file_from_before_the_sections_leaves_it_to_play_exe() {
        let files = Files::new("old-profile", None);
        let [game, _] = files.profiles();
        game.make().unwrap();
        let old_ini = b"[General]\r\nsTestName=Old\r\n[Display]\r\niPresentInterval=1\r\n";
        let old_prefs = "[Display]\r\niSize W=800\r\niSize H=600\r\n";
        std::fs::write(game.fallout_ini(), old_ini).unwrap();
        std::fs::write(&game.prefs, old_prefs).unwrap();
        settings_window(&files, "FrameLimit=60");
        assert_eq!(files.ini(0), old_ini);
        assert_eq!(files.prefs(0), old_prefs);
        let written = std::fs::read_to_string(&files.package.settings).unwrap();
        assert!(
            written.contains("\r\nFrameLimit=60\r\n") && written.contains("\r\nBorderless=1\r\n"),
            "{written}"
        );
        files.start().unwrap();
        for (profile, rest) in [&old_ini[..], DEFAULT_INI].into_iter().enumerate() {
            let ini = text_of(&files.ini(profile));
            assert!(ini.starts_with("; nv-rs's own options."), "{ini}");
            assert!(
                ini.ends_with(&text_of(rest)),
                "the file's own part as it was"
            );
            let get = |s: &str, k: &str| settings::ini_value(&ini, s, k).map(str::to_string);
            assert_eq!(get("nv-rs Display", "iFrameLimit").as_deref(), Some("60"));
            assert_eq!(get("nv-rs Display", "bBorderless").as_deref(), Some("1"));
            assert!(ini.contains("\r\n;bUncap=1\r\n"), "the fix lines too");
            assert!(files.prefs(profile).contains("bFull Screen=0"));
        }
        assert!(!files.package.settings.exists());
        // From here on Settings.cmd edits the files.
        settings_window(&files, "FrameLimit=90");
        assert!(!files.package.settings.exists());
        for profile in 0..2 {
            let ini = text_of(&files.ini(profile));
            assert_eq!(
                settings::ini_value(&ini, "nv-rs Display", "iFrameLimit"),
                Some("90")
            );
        }
    }

    /// With no Fallout.ini of the player's and no Fallout_default.ini, no file is made: one that
    /// held only our block would stop the player's own from being copied at a later start. A
    /// settings.ini then stays until there is a file for its choices.
    #[test]
    fn with_nothing_to_start_from_no_fallout_ini_is_made() {
        let files = Files::new("seed-nothing", None);
        std::fs::remove_file(files.game.join("Fallout_default.ini")).unwrap();
        std::fs::create_dir_all(&files.package.userdata).unwrap();
        std::fs::write(&files.package.settings, "FrameLimit=60\r\n").unwrap();
        files.start().unwrap();
        let [game, mods] = files.profiles();
        assert!(!game.fallout_ini().exists() && !mods.fallout_ini().exists());
        assert!(game.prefs.is_file(), "the prefs are made as before");
        assert!(files.package.settings.exists());
        assert!(files.log().contains("no Fallout.ini to start from"));
        assert!(files.log().contains("settings.ini stays where it is"));
        // The player's file appears later (the game's own launcher ran): now it is copied, an
        // empty file in the profile counting as none, and the settings go in.
        let player = b"[General]\r\nsTestName=Later\r\n";
        std::fs::write(files.real.join("Fallout.ini"), player).unwrap();
        std::fs::write(game.fallout_ini(), b"").unwrap();
        files.start().unwrap();
        for profile in 0..2 {
            let ini = text_of(&files.ini(profile));
            assert!(ini.ends_with(&text_of(player)));
            assert_eq!(
                settings::ini_value(&ini, "nv-rs Display", "iFrameLimit"),
                Some("60")
            );
        }
        assert!(!files.package.settings.exists());
    }

    /// A file that starts with a UTF-8 byte order mark gets the block at its end, every byte of
    /// the player's where it was (`game_ini::with_sections` says why).
    #[test]
    fn a_file_with_a_byte_order_mark_gets_the_sections_at_its_end() {
        let mut player = vec![0xef, 0xbb, 0xbf];
        player.extend_from_slice(b"[General]\r\nsTestName=Marked\r\n[Display]\r\nfTest=2.0000\r\n");
        let files = Files::new("seed-marked", Some(&player));
        files.start().unwrap();
        let block = our_block("\r\n");
        let mut want = player.clone();
        want.extend_from_slice(b"\r\n");
        want.extend_from_slice(&block[..block.len() - 2]);
        assert_eq!(files.ini(0), want);
        files.start().unwrap();
        assert_eq!(files.ini(0), want, "a second start changes nothing");
    }

    /// A UTF-16 file is copied and left as it is, the log says so, and a settings.ini stays: its
    /// choices have no file to go into.
    #[test]
    fn a_utf16_file_is_left_as_it_is() {
        let mut player = vec![0xff, 0xfe];
        player.extend(
            "[General]\r\nsTestName=Wide\r\n"
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        );
        let files = Files::new("seed-utf16", Some(&player));
        std::fs::create_dir_all(&files.package.userdata).unwrap();
        std::fs::write(&files.package.settings, "FrameLimit=60\r\n").unwrap();
        files.start().unwrap();
        assert_eq!(files.ini(0), player);
        assert_eq!(files.ini(1), player);
        assert!(files.log().contains("is UTF-16 text: left as it is"));
        assert!(files.package.settings.exists());
        files.start().unwrap();
        assert_eq!(files.ini(0), player);
    }

    #[test]
    fn a_read_only_file_gets_the_sections_and_stays_read_only() {
        let files = Files::new("seed-read-only", None);
        let [game, _] = files.profiles();
        game.make().unwrap();
        std::fs::write(game.fallout_ini(), DEFAULT_INI).unwrap();
        let mut p = std::fs::metadata(game.fallout_ini()).unwrap().permissions();
        p.set_readonly(true);
        std::fs::set_permissions(game.fallout_ini(), p).unwrap();
        files.start().unwrap();
        let mut want = our_block("\r\n");
        want.extend_from_slice(DEFAULT_INI);
        assert_eq!(files.ini(0), want);
        assert!(std::fs::metadata(game.fallout_ini())
            .unwrap()
            .permissions()
            .readonly());
        assert!(!std::fs::metadata(game.prefs.clone())
            .unwrap()
            .permissions()
            .readonly());
    }

    #[test]
    fn settings_ini_moves_into_both_profiles_once() {
        let files = Files::new("migrate", None);
        std::fs::create_dir_all(&files.package.userdata).unwrap();
        let old = "Borderless=0\r\nResolution=1920x1080\r\nAntiAliasing=4x\r\nVSync=Off\r\nPreset=vanilla\r\nFrameLimit=60\r\nMenuFrameLimit=0\r\nNV_THREADS=stepped\r\nNV_SOMETHING=1\r\n";
        std::fs::write(&files.package.settings, old).unwrap();
        files.start().unwrap();
        let mut moved = files.package.settings.as_os_str().to_owned();
        moved.push(".migrated");
        assert!(!files.package.settings.exists());
        assert_eq!(std::fs::read_to_string(&moved).unwrap(), old);
        for profile in 0..2 {
            let ini = text_of(&files.ini(profile));
            let get = |s: &str, k: &str| settings::ini_value(&ini, s, k).map(str::to_string);
            assert_eq!(get("nv-rs General", "sPreset").as_deref(), Some("vanilla"));
            assert_eq!(get("nv-rs Display", "bBorderless").as_deref(), Some("0"));
            assert_eq!(get("nv-rs Display", "iFrameLimit").as_deref(), Some("60"));
            assert_eq!(
                get("nv-rs Display", "iMenuFrameLimit").as_deref(),
                Some("0")
            );
            assert_eq!(
                get("nv-rs Threads", "bSteppedThreads").as_deref(),
                Some("1")
            );
            assert_eq!(get("Display", "iPresentInterval").as_deref(), Some("0"));
            assert!(ini.ends_with("[Display]\r\niPresentInterval=0\r\nfTest=1.0000\r\n"));
            let prefs = files.prefs(profile);
            assert!(prefs.contains("iSize W=1920") && prefs.contains("iMultiSample=4"));
            assert!(!prefs.contains("iPresentInterval") && !prefs.contains("bFull Screen"));
        }
        // Next start: nothing moves again; a key the player edits in the file stays.
        let [game, _] = files.profiles();
        let edited = text_of(&files.ini(0)).replace("iFrameLimit=60", "iFrameLimit=144");
        std::fs::write(game.fallout_ini(), bytes_of(&edited)).unwrap();
        let before = files.prefs(0);
        files.start().unwrap();
        assert_eq!(text_of(&files.ini(0)), edited);
        assert_eq!(files.prefs(0), before);
    }

    #[test]
    fn a_settings_line_the_game_would_refuse_stops_before_anything_moves() {
        let files = Files::new("migrate-bad", None);
        std::fs::create_dir_all(&files.package.userdata).unwrap();
        std::fs::write(&files.package.settings, "Preset=vanila\r\n").unwrap();
        let why = files.start().unwrap_err();
        assert_eq!(why.code, STOP_SETTINGS);
        assert!(why.message.contains("Preset=vanila"));
        assert!(files.package.settings.exists());
        assert!(!files.profiles()[0].fallout_ini().exists());
    }

    /// A file of the player's with the sections (Play.exe made it ready) and a settings.ini put
    /// back beside it: while settings.ini is there it is the settings, for Settings.cmd and for
    /// Play.exe alike. Settings.cmd shows the files with settings.ini's lines over them and writes
    /// settings.ini; Play.exe moves it in, its lines winning, a key no line names keeping the
    /// file's value.
    #[test]
    fn settings_ini_beside_files_that_have_the_sections_wins_in_both_tools() {
        let files = Files::new("both-present", None);
        files.start().unwrap();
        let [game, mods] = files.profiles();
        for profile in [&game, &mods] {
            let ini = text_of(&std::fs::read(profile.fallout_ini()).unwrap())
                .replace("iMenuFrameLimit=120", "iMenuFrameLimit=45")
                .replace("iFrameLimit=0", "iFrameLimit=30");
            std::fs::write(profile.fallout_ini(), bytes_of(&ini)).unwrap();
        }
        std::fs::write(&files.package.settings, "FrameLimit=60\r\nBorderless=0\r\n").unwrap();
        let before = (files.ini(0), files.ini(1));
        // Settings.cmd, saving what it shows: settings.ini over the files, into settings.ini.
        settings_window(&files, "");
        assert_eq!(
            (files.ini(0), files.ini(1)),
            before,
            "the files are not edited"
        );
        let written = std::fs::read_to_string(&files.package.settings).unwrap();
        for line in [
            "FrameLimit=60",
            "Borderless=0",
            "MenuFrameLimit=45",
            "Preset=fixes",
        ] {
            assert!(
                written.contains(&format!("\r\n{line}\r\n")),
                "{line}: {written}"
            );
        }
        files.start().unwrap();
        for profile in 0..2 {
            let ini = text_of(&files.ini(profile));
            let get = |s: &str, k: &str| settings::ini_value(&ini, s, k).map(str::to_string);
            assert_eq!(get("nv-rs Display", "iFrameLimit").as_deref(), Some("60"));
            assert_eq!(get("nv-rs Display", "bBorderless").as_deref(), Some("0"));
            assert_eq!(
                get("nv-rs Display", "iMenuFrameLimit").as_deref(),
                Some("45")
            );
        }
        assert!(!files.package.settings.exists());
        assert_eq!(files.migrated(), ["settings.ini.migrated"]);
    }

    /// A settings.ini that can't be renamed (another program holds it open): its choices go in,
    /// the log says clearly that it stays, and it stays the settings: the next start moves it in
    /// again, over a key edited in the file meanwhile, the same result every time.
    #[test]
    fn a_settings_ini_that_cannot_be_renamed_stays_the_settings() {
        use std::os::windows::fs::OpenOptionsExt;
        let files = Files::new("rename-fails", None);
        std::fs::create_dir_all(&files.package.userdata).unwrap();
        std::fs::write(&files.package.settings, "FrameLimit=60\r\n").unwrap();
        // Readable by others, not to be renamed or deleted while this is open.
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&files.package.settings)
            .unwrap();
        files.start().unwrap();
        assert!(files.package.settings.exists());
        assert!(files.migrated().is_empty());
        assert!(
            files
                .log()
                .contains("but the file could not be renamed to settings.ini.migrated"),
            "{}",
            files.log()
        );
        let frame_limit = |p: usize| {
            settings::ini_value(&text_of(&files.ini(p)), "nv-rs Display", "iFrameLimit")
                .map(str::to_string)
        };
        assert_eq!(frame_limit(0).as_deref(), Some("60"));
        assert_eq!(frame_limit(1).as_deref(), Some("60"));
        // Settings.cmd, meanwhile, writes settings.ini, not the files.
        drop(held);
        settings_window(&files, "MenuFrameLimit=90");
        let written = std::fs::read_to_string(&files.package.settings).unwrap();
        assert!(
            written.contains("\r\nFrameLimit=60\r\n")
                && written.contains("\r\nMenuFrameLimit=90\r\n")
        );
        let [game, _] = files.profiles();
        let edited = text_of(&files.ini(0)).replace("iFrameLimit=60", "iFrameLimit=144");
        std::fs::write(game.fallout_ini(), bytes_of(&edited)).unwrap();
        files.start().unwrap();
        assert_eq!(
            frame_limit(0).as_deref(),
            Some("60"),
            "settings.ini still wins"
        );
        assert!(!files.package.settings.exists());
        assert_eq!(files.migrated(), ["settings.ini.migrated"]);
    }

    /// A second settings.ini (restored from a backup, or an old userdata copied over) moves in
    /// too, and the earlier moved file is kept as it was: the new one gets a number.
    #[test]
    fn a_second_migration_keeps_the_first_moved_file() {
        let files = Files::new("migrate-twice", None);
        std::fs::create_dir_all(&files.package.userdata).unwrap();
        std::fs::write(&files.package.settings, "FrameLimit=60\r\n").unwrap();
        files.start().unwrap();
        std::fs::write(&files.package.settings, "FrameLimit=90\r\n").unwrap();
        files.start().unwrap();
        std::fs::write(&files.package.settings, "FrameLimit=120\r\n").unwrap();
        files.start().unwrap();
        assert_eq!(
            files.migrated(),
            [
                "settings.ini.migrated",
                "settings.ini.migrated.2",
                "settings.ini.migrated.3"
            ]
        );
        let read = |name: &str| std::fs::read_to_string(files.package.userdata.join(name)).unwrap();
        assert_eq!(read("settings.ini.migrated"), "FrameLimit=60\r\n");
        assert_eq!(read("settings.ini.migrated.2"), "FrameLimit=90\r\n");
        let ini = text_of(&files.ini(1));
        assert_eq!(
            settings::ini_value(&ini, "nv-rs Display", "iFrameLimit"),
            Some("120")
        );
        assert!(files
            .log()
            .contains("the old file is settings.ini.migrated.3"));
    }

    /// The variables on the game's own command (`pass_switches` on a `Command`, as `play` makes
    /// it). In play the eight are taken out and nothing else is said; with a test variable set
    /// before Play.exe, the file's choices are set on the command, nothing is taken out, the one
    /// set before is left to come through as it was, and the log line names all of it.
    #[test]
    fn the_games_command_gets_the_files_choices_only_in_a_test() {
        let eight = [
            "NV_PRESET",
            "NV_BORDERLESS",
            "NV_D3D9",
            "NV_FRAME_LIMIT",
            "NV_MENU_FRAME_LIMIT",
            "NV_SHOW_FPS",
            "NV_THREADS",
            "NV_RENDER_THREAD",
        ];
        let ini = text_of(&our_block("\r\n"))
            .replace("sPreset=fixes", "sPreset=vanilla")
            .replace("iFrameLimit=0", "iFrameLimit=90")
            .replace(";bGrabPush=1", "bGrabPush=0");
        let envs = |command: &Command| -> Vec<(String, Option<String>)> {
            command
                .get_envs()
                .map(|(k, v)| {
                    (
                        k.to_string_lossy().into_owned(),
                        v.map(|v| v.to_string_lossy().into_owned()),
                    )
                })
                .collect()
        };
        let mut play = Command::new("game.exe");
        play.env("NV_SETTINGS", "settings.ini");
        let line = pass_switches(&mut play, &game_ini::switches(&ini, None, |_| false));
        assert_eq!(line, None);
        let mut want: Vec<(String, Option<String>)> =
            eight.iter().map(|name| (name.to_string(), None)).collect();
        want.push(("NV_SETTINGS".into(), Some("settings.ini".into())));
        want.sort();
        let mut got = envs(&play);
        got.sort();
        assert_eq!(got, want);

        let mut test = Command::new("game.exe");
        let inherited = |name: &str| name == "NV_FRAME_LOG" || name == "NV_THREADS";
        let line = pass_switches(&mut test, &game_ini::switches(&ini, None, inherited)).unwrap();
        let mut got = envs(&test);
        got.sort();
        let mut want: Vec<(String, Option<String>)> = [
            ("NV_PRESET", "vanilla"),
            ("NV_BORDERLESS", "1"),
            ("NV_D3D9", "wgpu"),
            ("NV_FRAME_LIMIT", "90"),
            ("NV_MENU_FRAME_LIMIT", "120"),
            ("NV_SHOW_FPS", "0"),
            ("NV_RENDER_THREAD", "1"),
            ("NV_GRAB_PUSH", "0"),
        ]
        .iter()
        .map(|(n, v)| (n.to_string(), Some(v.to_string())))
        .collect();
        want.sort();
        assert_eq!(
            got, want,
            "NV_THREADS is not on the command: it comes through as set"
        );
        assert!(
            line.starts_with("NV_FRAME_LOG was set before Play.exe started"),
            "{line}"
        );
        assert!(
            line.contains("NV_PRESET=vanilla, NV_BORDERLESS=1, NV_D3D9=wgpu, NV_FRAME_LIMIT=90"),
            "{line}"
        );
        assert!(line.ends_with("kept as they were: NV_THREADS"), "{line}");
    }

    /// Settings.ps1 on the three shapes measured last (measure/ini/repeated-sections.md): a key
    /// before any heading (in no section: not read, not written), a heading with text after its
    /// `]` (still `[Display]`) and a key twice in that section (the first line is the one read
    /// and written). It shows what Windows reads and writes as `set_key` does; FalloutPrefs.ini's
    /// resolution goes into its first `[Display]` the same way, from Play.exe and Settings.cmd.
    #[test]
    fn settings_cmd_reads_and_writes_as_windows_on_odd_lines() {
        let files = Files::new("settings-odd", None);
        files.start().unwrap();
        let [game, mods] = files.profiles();
        let ini = format!(
            "iPresentInterval=0\r\n{}",
            text_of(&files.ini(0)).replace(
                "[Display]\r\niPresentInterval=1\r\n",
                "[Display] ; tweaks\r\niPresentInterval=1\r\nipresentinterval=0\r\n"
            )
        );
        assert!(ini.contains("[Display] ; tweaks\r\n"));
        assert_eq!(
            settings::ini_value(&ini, "Display", "iPresentInterval"),
            Some("1")
        );
        let prefs = "[Display]\r\niSize W=800\r\n[Display]\r\niSize W=640\r\niSize H=480\r\n";
        for profile in [&game, &mods] {
            std::fs::write(profile.fallout_ini(), bytes_of(&ini)).unwrap();
            std::fs::write(&profile.prefs, prefs).unwrap();
        }
        // The window shows V-sync On (the first line): choosing On writes nothing.
        settings_window(&files, "VSync=On");
        assert_eq!(text_of(&files.ini(0)), ini);
        settings_window(&files, "VSync=Off;Resolution=1920x1080");
        let want = game_ini::set_key(&ini, "Display", "iPresentInterval", "0");
        assert!(want.starts_with("iPresentInterval=0\r\n"));
        assert!(want.contains("[Display] ; tweaks\r\niPresentInterval=0\r\nipresentinterval=0\r\n"));
        let mut want_prefs = prefs.to_string();
        for (key, value) in [("iSize W", "1920"), ("iSize H", "1080")] {
            want_prefs = game_ini::set_key(&want_prefs, "Display", key, value);
        }
        assert!(want_prefs.ends_with("[Display]\r\niSize W=640\r\niSize H=480\r\n"));
        for profile in 0..2 {
            assert_eq!(text_of(&files.ini(profile)), want);
            assert_eq!(files.prefs(profile), want_prefs);
        }
        // Play.exe's own writer gives the same FalloutPrefs.ini lines.
        assert_eq!(
            settings::apply_to_prefs(
                prefs,
                &[
                    ("iSize W".to_string(), "1920".to_string()),
                    ("iSize H".to_string(), "1080".to_string())
                ]
            ),
            want_prefs
        );
    }

    /// Settings.ps1 on a file with a repeated `[Display]` section (a hand-edited file): it shows
    /// and changes the first section's key only, as Windows reads and writes it, the same lines as
    /// `set_key`; the later section stays byte for byte.
    #[test]
    fn settings_cmd_uses_the_first_section_of_a_repeated_name() {
        let files = Files::new("settings-repeated", None);
        files.start().unwrap();
        let [game, mods] = files.profiles();
        // The first [Display] has no iPresentInterval; a later [Display] says 0, which Windows
        // never reads. So the game's V-sync is its default, and the window shows the game's
        // setting: choosing Off writes the key into the first section, the later one untouched.
        let ini = text_of(&files.ini(0)).replace("iPresentInterval=1\r\n", "")
            + "\r\n[Display]\r\niPresentInterval=0\r\n";
        assert_eq!(
            settings::ini_value(&ini, "Display", "iPresentInterval"),
            None
        );
        for profile in [&game, &mods] {
            std::fs::write(profile.fallout_ini(), bytes_of(&ini)).unwrap();
        }
        settings_window(&files, "");
        assert_eq!(
            text_of(&files.ini(0)),
            ini,
            "nothing chosen, nothing written"
        );
        settings_window(&files, "VSync=Off");
        let want = game_ini::set_key(&ini, "Display", "iPresentInterval", "0");
        assert!(want.ends_with("\r\n[Display]\r\niPresentInterval=0\r\n"));
        assert_eq!(
            settings::ini_value(&want, "Display", "iPresentInterval"),
            Some("0")
        );
        for profile in 0..2 {
            assert_eq!(text_of(&files.ini(profile)), want);
        }
    }

    /// Settings.ps1 on a file it must change in place whatever its shape: LF line ends, a byte
    /// that is not UTF-8, a key the player commented out and no `[Display]` section. Its lines are
    /// the ones Play.exe's own `set_key` makes, in Settings.ps1's order; every other byte stays.
    #[test]
    fn settings_cmd_edits_any_shape_of_file_as_play_exe_would() {
        let files = Files::new("settings-shapes", None);
        files.start().unwrap();
        let [game, mods] = files.profiles();
        let ini = text_of(&our_block("\n")).replace("\niFrameLimit=0\n", "\n;iFrameLimit=0\n")
            + "[General]\nsTestName=Caf\u{e9}\n";
        for profile in [&game, &mods] {
            std::fs::write(profile.fallout_ini(), bytes_of(&ini)).unwrap();
        }
        let prefs_before = (files.prefs(0), files.prefs(1));
        settings_window(&files, "VSync=Off;FrameLimit=60;NV_RENDER_THREAD=0");
        let mut want = ini.clone();
        for (section, key, value) in [
            ("Display", "iPresentInterval", "0"),
            ("nv-rs Display", "iFrameLimit", "60"),
            ("nv-rs Threads", "bRenderThread", "0"),
        ] {
            want = game_ini::set_key(&want, section, key, value);
        }
        assert!(want.ends_with("sTestName=Caf\u{e9}\n[Display]\niPresentInterval=0\n"));
        assert!(!want.contains('\r'));
        for profile in 0..2 {
            assert_eq!(text_of(&files.ini(profile)), want);
        }
        assert_eq!((files.prefs(0), files.prefs(1)), prefs_before);
    }

    /// A save that fails at its last step leaves the file whole and as it was, leaves no `.part`
    /// file behind and says so; saving again once it is free finishes the job. Here the game
    /// profile's Fallout.ini is held open by another program that lets others read and write it
    /// but not replace or delete it: an in-place write (the old way) would go through and could be
    /// cut off half way, while the `.part` file is written in full and only its last step, taking
    /// the file's place, fails.
    #[test]
    fn a_failed_save_leaves_the_file_whole_and_says_so() {
        use std::os::windows::fs::OpenOptionsExt;
        let files = Files::new("settings-fails", None);
        files.start().unwrap();
        let [game, mods] = files.profiles();
        let before = files.ini(0);
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1 | 2)
            .open(game.fallout_ini())
            .unwrap();
        let (saved, errors) = run_settings(&files, "FrameLimit=60");
        assert!(!saved, "the script ends with a failure");
        assert!(errors.contains("The settings were not saved."), "{errors}");
        // The path's own ASCII end: the scratch folder above it may have letters the error
        // stream's code page shows otherwise.
        assert!(
            errors.contains("game\\Documents\\My Games\\FalloutNV\\Fallout.ini"),
            "{errors}"
        );
        drop(held);
        assert_eq!(files.ini(0), before, "the game profile's file as it was");
        for profile in [&game, &mods] {
            let mut part = profile.fallout_ini().into_os_string();
            part.push(".part");
            assert!(!Path::new(&part).exists(), "no .part left");
        }
        settings_window(&files, "FrameLimit=60");
        for profile in 0..2 {
            assert_eq!(
                settings::ini_value(
                    &text_of(&files.ini(profile)),
                    "nv-rs Display",
                    "iFrameLimit"
                ),
                Some("60")
            );
        }
    }

    /// The played profile's Fallout.ini can't take our sections (UTF-16 here; or there was
    /// nothing to make it from): settings.ini stays, its unused lines are named, the game's own
    /// full-screen mode is kept off by settings.ini's choice, and the game gets the player's
    /// choices as variables, the defaults for what settings.ini doesn't say. Settings.cmd keeps
    /// writing settings.ini for it.
    #[test]
    fn a_profile_without_the_sections_gets_the_choices_as_variables() {
        let mut player = vec![0xff, 0xfe];
        player.extend(
            "[General]\r\nsTestName=Wide\r\n"
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        );
        let files = Files::new("unseeded-play", Some(&player));
        std::fs::create_dir_all(&files.package.userdata).unwrap();
        std::fs::write(
            &files.package.settings,
            "NV_THREADS=stepped\r\nFrameLimit=60\r\nNV_SOMETHING=1\r\n",
        )
        .unwrap();
        files.start().unwrap();
        assert_eq!(files.ini(0), player);
        assert!(files.package.settings.exists());
        assert!(
            files.log().contains(
                "settings.ini's lines not used any more (left in the file): NV_SOMETHING"
            ),
            "{}",
            files.log()
        );
        assert!(
            files.prefs(0).contains("bFull Screen=0"),
            "no Borderless line: the default"
        );
        let pending = Settings::parse(&settings::decode(
            &std::fs::read(&files.package.settings).unwrap(),
        ));
        let mut command = Command::new("game.exe");
        let switches = game_ini::switches(&text_of(&files.ini(0)), Some(&pending), |_| false);
        let line = pass_switches(&mut command, &switches).unwrap();
        assert!(line.starts_with("the played profile's Fallout.ini has no nv-rs sections"));
        let mut got: Vec<(String, Option<String>)> = command
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        got.sort();
        let mut want: Vec<(String, Option<String>)> = [
            ("NV_PRESET", "fixes"),
            ("NV_BORDERLESS", "1"),
            ("NV_D3D9", "wgpu"),
            ("NV_FRAME_LIMIT", "60"),
            ("NV_MENU_FRAME_LIMIT", "120"),
            ("NV_SHOW_FPS", "0"),
            ("NV_THREADS", "stepped"),
            ("NV_RENDER_THREAD", "1"),
        ]
        .iter()
        .map(|(n, v)| (n.to_string(), Some(v.to_string())))
        .collect();
        want.sort();
        assert_eq!(got, want);
        // Settings.cmd: Window chosen goes into settings.ini, and the next start keeps the game's
        // full-screen mode as the game's files say (no push).
        settings_window(&files, "Borderless=0");
        assert_eq!(files.ini(0), player);
        let written = std::fs::read_to_string(&files.package.settings).unwrap();
        assert!(
            written.contains("\r\nBorderless=0\r\n") && written.contains("\r\nFrameLimit=60\r\n")
        );
        let [game, _] = files.profiles();
        std::fs::remove_file(&game.prefs).unwrap();
        files.start().unwrap();
        assert!(!files.prefs(0).contains("bFull Screen"));
    }

    /// Settings.cmd's Renderer: Direct3D 9 chosen goes into both profiles' `sRenderer`, Vulkan back
    /// again; the generated sections start at the game's default, Vulkan.
    #[test]
    fn settings_cmd_sets_the_renderer() {
        let files = Files::new("settings-renderer", None);
        files.start().unwrap();
        let renderer = |files: &Files, profile| {
            settings::ini_value(&text_of(&files.ini(profile)), "nv-rs Display", "sRenderer")
                .map(str::to_string)
        };
        for (choose, want) in [
            ("", "vulkan"),
            ("Renderer=d3d9", "d3d9"),
            ("Renderer=vulkan", "vulkan"),
        ] {
            if !choose.is_empty() {
                settings_window(&files, choose);
            }
            for profile in 0..2 {
                assert_eq!(renderer(&files, profile).as_deref(), Some(want), "{choose}");
            }
        }
    }

    /// Settings.cmd edits the files in place only when every profile's Fallout.ini has our
    /// sections: with the mods profile's file missing it writes settings.ini, and Play.exe puts
    /// the choice into both profiles, not its defaults into one.
    #[test]
    fn settings_cmd_edits_the_files_only_when_every_profile_has_the_sections() {
        let files = Files::new("one-profile", None);
        files.start().unwrap();
        let [_, mods] = files.profiles();
        std::fs::remove_file(mods.fallout_ini()).unwrap();
        let game_before = files.ini(0);
        settings_window(&files, "FrameLimit=90");
        assert_eq!(files.ini(0), game_before, "not edited in place");
        assert!(std::fs::read_to_string(&files.package.settings)
            .unwrap()
            .contains("\r\nFrameLimit=90\r\n"));
        files.start().unwrap();
        for profile in 0..2 {
            assert_eq!(
                settings::ini_value(
                    &text_of(&files.ini(profile)),
                    "nv-rs Display",
                    "iFrameLimit"
                ),
                Some("90")
            );
        }
        assert!(!files.package.settings.exists());
    }

    /// A file Settings.cmd can't read stops it with the message, before it writes anything: it
    /// must not take the file for one without the settings and save defaults over the player's
    /// choices.
    #[test]
    fn a_file_settings_cmd_cannot_read_stops_it_before_anything_is_written() {
        use std::os::windows::fs::OpenOptionsExt;
        let files = Files::new("settings-unreadable", None);
        files.start().unwrap();
        let [game, _] = files.profiles();
        let before = (files.ini(0), files.ini(1), files.prefs(0), files.prefs(1));
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(game.fallout_ini())
            .unwrap();
        let (saved, errors) = run_settings(&files, "FrameLimit=60");
        drop(held);
        assert!(!saved);
        assert!(errors.contains("The settings were not saved."), "{errors}");
        assert!(!files.package.settings.exists(), "no settings.ini written");
        assert_eq!(
            (files.ini(0), files.ini(1), files.prefs(0), files.prefs(1)),
            before
        );
    }

    /// distribution/Settings.ps1 run in the scratch package with no window (`NV_RS_NO_WINDOWS`),
    /// `choose` its choices (`NV_RS_SETTINGS_CHOOSE`): how it ended and what it wrote to its
    /// error stream.
    fn run_settings(files: &Files, choose: &str) -> (bool, String) {
        let script = files.package.root.join("Settings.ps1");
        std::fs::create_dir_all(&files.package.root).unwrap();
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../distribution/Settings.ps1"),
            &script,
        )
        .unwrap();
        let mut command = Command::new("powershell");
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&script)
            .env("NV_RS_NO_WINDOWS", "1")
            .env("NV_RS_SETTINGS_CHOOSE", choose);
        // A PowerShell that hangs fails the test instead of holding it (and CI) for hours.
        let out = match run_limited(&mut command, Duration::from_secs(120)) {
            Limited::Done(out) => out,
            Limited::TimedOut => panic!("Settings.ps1 did not end within 2 minutes"),
            Limited::Failed(e) => panic!("Windows PowerShell could not start: {e}"),
        };
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        (out.status.success(), own_errors(&said))
    }

    /// What a PowerShell script itself wrote to the error stream. On a PC where Windows
    /// PowerShell has not run before, PowerShell's own "Preparing modules for first use" progress
    /// arrives there too, as a CLIXML document with no error record in it: that is not the
    /// script's: that block is taken out and the rest kept. A block with an error record stays.
    fn own_errors(said: &str) -> String {
        let Some(rest) = said.strip_prefix("#< CLIXML") else {
            return said.to_string();
        };
        let end = rest
            .find("</Objs>")
            .map_or(rest.len(), |at| at + "</Objs>".len());
        if rest[..end].contains("S=\"Error\"") {
            return said.to_string();
        }
        rest[end..].trim().to_string()
    }

    /// [`run_settings`], which must save without a word.
    fn settings_window(files: &Files, choose: &str) {
        let (saved, errors) = run_settings(files, choose);
        assert!(saved && errors.is_empty(), "{errors}");
    }

    #[test]
    fn powershells_own_progress_is_not_a_script_error() {
        let progress = "#< CLIXML\r\n<Objs Version=\"1.1.0.1\" xmlns=\"http://schemas.microsoft.com/powershell/2004/04\"><Obj S=\"progress\" RefId=\"0\"><TN RefId=\"0\"><T>System.Management.Automation.PSCustomObject</T></TN><MS><I64 N=\"SourceId\">1</I64><PR N=\"Record\"><AV>Preparing modules for first use.</AV></PR></MS></Obj></Objs>";
        assert_eq!(own_errors(progress), "");
        let error = "#< CLIXML\r\n<Objs><S S=\"Error\">boom</S></Objs>";
        assert_eq!(own_errors(error), error);
        assert_eq!(
            own_errors("The settings were not saved."),
            "The settings were not saved."
        );
        assert_eq!(own_errors(""), "");
        // The script's own words after the block are kept.
        assert_eq!(
            own_errors(&format!("{progress}\r\nThe settings were not saved.")),
            "The settings were not saved."
        );
    }

    #[test]
    fn settings_cmd_before_the_first_start_writes_settings_ini_for_play_exe_to_move() {
        let files = Files::new("settings-first", None);
        settings_window(
            &files,
            "Preset=vanilla;FrameLimit=60;NV_RENDER_THREAD=0;NV_SHOW_FPS=1",
        );
        let written = std::fs::read_to_string(&files.package.settings).unwrap();
        assert!(
            written.contains("\r\nPreset=vanilla\r\n") && written.contains("\r\nFrameLimit=60\r\n")
        );
        files.start().unwrap();
        let ini = text_of(&files.ini(1));
        assert_eq!(
            settings::ini_value(&ini, "nv-rs General", "sPreset"),
            Some("vanilla")
        );
        assert_eq!(
            settings::ini_value(&ini, "nv-rs Display", "iFrameLimit"),
            Some("60")
        );
        assert_eq!(
            settings::ini_value(&ini, "nv-rs Threads", "bRenderThread"),
            Some("0")
        );
        assert_eq!(
            settings::ini_value(&ini, "nv-rs Display", "bShowFrameRate"),
            Some("1")
        );
        assert!(!files.package.settings.exists());
    }

    #[test]
    fn settings_cmd_changes_only_its_own_lines_in_both_profiles() {
        let files = Files::new("settings-files", None);
        files.start().unwrap();
        let [game, mods] = files.profiles();
        // Saving what it shows changes no byte.
        let before: Vec<(Vec<u8>, String)> =
            (0..2).map(|p| (files.ini(p), files.prefs(p))).collect();
        settings_window(&files, "");
        let after: Vec<(Vec<u8>, String)> =
            (0..2).map(|p| (files.ini(p), files.prefs(p))).collect();
        assert_eq!(after, before);
        assert!(!files.package.settings.exists());
        // A read-only mark stays.
        let mut p = std::fs::metadata(mods.fallout_ini()).unwrap().permissions();
        p.set_readonly(true);
        std::fs::set_permissions(mods.fallout_ini(), p).unwrap();
        settings_window(
            &files,
            "Borderless=0;Preset=vanilla;FrameLimit=144;VSync=Off;Resolution=1280x720;AntiAliasing=Off;NV_SHOW_FPS=1;NV_THREADS=stepped",
        );
        for (profile, (ini, prefs)) in before.iter().enumerate() {
            // The same lines as Play.exe's own set_key makes, everything else as it was.
            let mut want = text_of(ini);
            for (section, key, value) in [
                ("nv-rs Display", "bBorderless", "0"),
                ("Display", "iPresentInterval", "0"),
                ("nv-rs General", "sPreset", "vanilla"),
                ("nv-rs Display", "iFrameLimit", "144"),
                ("nv-rs Display", "bShowFrameRate", "1"),
                ("nv-rs Threads", "bSteppedThreads", "1"),
            ] {
                want = game_ini::set_key(&want, section, key, value);
            }
            assert_eq!(text_of(&files.ini(profile)), want);
            let mut want = prefs.clone();
            for (key, value) in [
                ("iSize W", "1280"),
                ("iSize H", "720"),
                ("iMultiSample", "0"),
            ] {
                want = game_ini::set_key(&want, "Display", key, value);
            }
            assert_eq!(files.prefs(profile), want);
        }
        assert!(std::fs::metadata(mods.fallout_ini())
            .unwrap()
            .permissions()
            .readonly());
        assert!(!std::fs::metadata(game.fallout_ini())
            .unwrap()
            .permissions()
            .readonly());
    }

    #[test]
    fn onedrive_documents_is_refused_as_the_game_refuses_it() {
        let profile = scratch("profile");
        let folders = profile_folders(&profile);
        let inside = |path: PathBuf| player_folder_of(&path, &folders);
        assert_eq!(
            inside(profile.join("OneDrive\\Documents\\nv-rs")),
            Some("Documents")
        );
        assert_eq!(
            inside(profile.join("onedrive/documents/Games/nv-rs")),
            Some("Documents")
        );
        assert_eq!(inside(profile.join("Documents\\nv-rs")), Some("Documents"));
        assert_eq!(
            inside(profile.join("AppData\\Local\\nv-rs")),
            Some("AppData")
        );
        // The rest of OneDrive is not the game's, nor a name that only starts the same.
        assert_eq!(inside(profile.join("OneDrive\\Desktop\\nv-rs")), None);
        assert_eq!(inside(profile.join("OneDrive\\Documents2\\nv-rs")), None);
        assert_eq!(inside(profile.join("Games\\nv-rs")), None);
        // A junction into OneDrive's Documents is the same place.
        std::fs::create_dir_all(profile.join("OneDrive\\Documents")).unwrap();
        junction(&profile.join("link"), &profile.join("OneDrive\\Documents"));
        assert_eq!(inside(profile.join("link\\nv-rs")), Some("Documents"));
        let _ = std::fs::remove_dir_all(&profile);
    }
}
