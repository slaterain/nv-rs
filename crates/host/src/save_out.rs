//! `NV_SAVE_OUT=DIR`, for scripted runs and the preview package: the save files the game wrote in
//! this run copied into `DIR`, so a later run can start from them through the main menu (CONTINUE
//! or LOAD) instead of replaying the opening. A save the game deleted in this run is deleted there
//! too. `NV_PREFS_OUT=FILE`: the same for the `FalloutPrefs.ini` the game wrote (the settings
//! changed in its menus).
//!
//! Files the game writes stay in engine memory (`engine_platform::files::Disk`); nothing
//! reaches the disk unless this is asked for. Only files whose name ends in `.fos` (saves) or
//! `.nvse` (the script extender's co-saves) are copied,
//! each under its own name (as its directory entry has it). A file is written to a temporary
//! name first and then renamed, so a reader never sees half of it. A copy is made again whenever
//! the game's bytes change.
//!
//! Main looks at the end of each of its frames, which costs one comparison while no file changed
//! (the platform's change count, `Disk::changes`); a save is written within one frame, so the
//! copy is of the whole file. Main only copies the changed file's bytes (a few megabytes, well
//! under a millisecond); the disk work (writing, renaming, removing) is done on the copy-out's own
//! thread, in order, and a clean end of the run (the game's own exit, `NV_REPEATABLE`'s end)
//! waits for it ([`finish`]), so a save made just before the game closes is kept. Until
//! 2026-11-10 main did it all itself at the 5 s status line: hashing every save written in the run
//! each time, and writing a new one and renaming it inside a frame.
//!
//! `DIR` must be a scratch folder or the package's own. One inside the player's Documents,
//! OneDrive's Documents or AppData is refused, and nothing is written.

use engine::Engine;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// `NV_SAVE_OUT`'s folder, or `None` when it is unset or refused (refused once, with a line).
fn out_dir() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = PathBuf::from(std::env::var_os("NV_SAVE_OUT")?);
        if let Some(why) = refused_here(&dir) {
            warn_visibly(
                &format!(
                    "NV_SAVE_OUT={}: refused ({why}); saves are not copied.",
                    dir.display()
                ),
                &plain_words(why, format!("Your saves are NOT being kept: they will be lost when the game closes. The save folder the game was given is one it must not write to. Start the game through {START}, and report this if you did.")),
            );
            return None;
        }
        for name in clean_parts(&dir) {
            println!("NV_SAVE_OUT: removed {name}, a copy cut short by an earlier run's end");
        }
        Some(dir)
    })
    .as_deref()
}

/// The copy-out's temporary files (`<save>.fos.part`, `<save>.nvse.part`) an earlier run left in
/// `dir` when it was killed part-way through a copy, removed (the save itself, renamed only once
/// complete, is untouched): their names.
pub fn clean_parts(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut removed = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        let part = lower.ends_with(".fos.part") || lower.ends_with(".nvse.part");
        if part && entry.path().is_file() && std::fs::remove_file(entry.path()).is_ok() {
            removed.push(name);
        }
    }
    removed
}

/// What the player starts the game with, for the warnings' words.
#[cfg(windows)]
const START: &str = "Play.exe";
#[cfg(not(windows))]
const START: &str = "play.sh";

/// Why the game may not write into `path` (saves, settings), by this system's rule: on Windows
/// [`refused`] with `USERPROFILE` and [`known_folders`]; on Linux and macOS the player's own
/// folders there (`game_folders::never_written`: Documents, Steam's folders and Proton's
/// prefixes, the game folders the look found, macOS's Library), from `HOME`, and the game folder
/// the port was started with (its `Data` and the folder holding it, [`prepare`]), found by the
/// look or not (a copy given by hand, one inside a prefix the look does not know, every game
/// folder on macOS).
fn refused_here(path: &Path) -> Option<&'static str> {
    #[cfg(windows)]
    {
        let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
        refused(path, profile.as_deref(), &known_folders())
    }
    #[cfg(not(windows))]
    refused_by(guard_list(), path).or_else(|| {
        let started = STARTED_GAME.lock().unwrap_or_else(|p| p.into_inner());
        game_folders::refused(path, &started)
    })
}

/// Off Windows: the game folders the port was started with ([`prepare`]), guarded whole (no
/// exception) whatever the look at the player's folders finds.
#[cfg(not(windows))]
static STARTED_GAME: Mutex<Vec<game_folders::Guarded>> = Mutex::new(Vec::new());

/// Off Windows: the game's `data` folder and the folder holding it (the game's folder) added to
/// [`STARTED_GAME`], as full paths (a relative one taken from the current folder).
#[cfg(not(windows))]
fn guard_started(data: &Path) {
    let data = if data.has_root() {
        data.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(here) => here.join(data),
            Err(_) => return,
        }
    };
    let mut started = STARTED_GAME.lock().unwrap_or_else(|p| p.into_inner());
    for root in [Some(data.as_path()), data.parent()].into_iter().flatten() {
        let guarded = game_folders::Guarded {
            root: root.to_path_buf(),
            except: None,
            system: game_folders::System::this(),
        };
        if !started.contains(&guarded) {
            started.push(guarded);
        }
    }
}

/// Off Windows, for the tests: the plain words of every warning [`warn_visibly`] gave.
#[cfg(all(test, not(windows)))]
static WARNED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Off Windows: why `path` may not be written by the look's `answer`: by the player's folders it
/// found, or, when the look gave no answer (none in time, its thread stopped), always refused.
#[cfg(not(windows))]
fn refused_by(
    answer: Result<&[game_folders::Guarded], &'static str>,
    path: &Path,
) -> Option<&'static str> {
    match answer {
        Ok(never) => game_folders::refused(path, never),
        Err(why) => Some(why),
    }
}

/// Off Windows: the player's folders (`game_folders::never_written`), looked at once, on a thread
/// of their own that [`prepare`] starts before the game's first frame.
#[cfg(not(windows))]
static GUARD: OnceLock<Result<Vec<game_folders::Guarded>, &'static str>> = OnceLock::new();

/// [`prepare`]'s look, until [`guard_list`] takes its answer.
#[cfg(not(windows))]
#[allow(clippy::type_complexity)]
static GUARD_LOOK: Mutex<Option<std::sync::mpsc::Receiver<Option<Vec<game_folders::Guarded>>>>> =
    Mutex::new(None);

/// Starts looking at the player's folders (Linux and macOS) on a thread of its own, so the first
/// frame does not wait on every Steam library and Wine prefix (a sleeping or offline disk can take
/// seconds). Once; nothing on Windows, which asks Windows for its folders. `data` is the game's
/// `Data` folder the port was started with: it and the game's folder are guarded from now on,
/// whether the look finds them or not.
pub fn prepare(data: &Path) {
    #[cfg(not(windows))]
    {
        guard_started(data);
        start_look();
    }
    #[cfg(windows)]
    let _ = data;
}

/// Off Windows: [`prepare`]'s look at the player's folders, started once.
#[cfg(not(windows))]
fn start_look() {
    let mut look = GUARD_LOOK.lock().unwrap_or_else(|p| p.into_inner());
    if look.is_some() || GUARD.get().is_some() {
        return;
    }
    let (send, receive) = std::sync::mpsc::channel();
    let started = std::thread::Builder::new()
        .name("nv player folders".into())
        .spawn(move || {
            let list =
                game_folders::Home::from_process().map(|home| game_folders::never_written(&home));
            let _ = send.send(list);
        });
    if started.is_ok() {
        *look = Some(receive);
    }
}

/// The refusal when [`prepare`]'s look at the player's folders did not answer within
/// [`LOOK_WAIT`]: a disk that does not answer (a sleeping or offline disk, a network share listed
/// as a Steam library), not a save folder of the wrong kind.
#[cfg(not(windows))]
const LOOK_TOO_SLOW: &str =
    "the player's folders could not be looked at in time: a disk did not answer within 10 s";

/// The refusal when [`prepare`]'s look ended without an answer (its thread stopped).
#[cfg(not(windows))]
const LOOK_STOPPED: &str = "the look at the player's folders stopped without an answer";

/// How long the first write waits for [`prepare`]'s look.
#[cfg(not(windows))]
const LOOK_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// The player's folders off Windows ([`prepare`]'s look), waited for at most [`LOOK_WAIT`]: a look
/// that takes longer (a disk that does not answer) or cannot be made refuses every write, said once.
#[cfg(not(windows))]
fn guard_list() -> Result<&'static [game_folders::Guarded], &'static str> {
    GUARD
        .get_or_init(|| {
            start_look();
            let receive = GUARD_LOOK.lock().unwrap_or_else(|p| p.into_inner()).take();
            let answer = wait_for_look(receive, LOOK_WAIT);
            if let Err(why @ (LOOK_TOO_SLOW | LOOK_STOPPED)) = answer {
                println!("NV_SAVE_OUT: {why}; nothing is written this run");
            }
            answer
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(|e| *e)
}

/// [`prepare`]'s look's answer through `receive`, waited for at most `wait`; why every write is
/// refused when there is none.
#[cfg(not(windows))]
#[allow(clippy::type_complexity)]
fn wait_for_look(
    receive: Option<std::sync::mpsc::Receiver<Option<Vec<game_folders::Guarded>>>>,
    wait: std::time::Duration,
) -> Result<Vec<game_folders::Guarded>, &'static str> {
    use std::sync::mpsc::RecvTimeoutError;
    let Some(receive) = receive else {
        return Err("the player's folders could not be looked at (no thread)");
    };
    match receive.recv_timeout(wait) {
        Ok(Some(list)) => Ok(list),
        Ok(None) => Err("HOME is not set to a full path, so the player's folders are unknown"),
        Err(RecvTimeoutError::Timeout) => Err(LOOK_TOO_SLOW),
        Err(RecvTimeoutError::Disconnected) => Err(LOOK_STOPPED),
    }
}

/// The player's words for a refusal `why`: `usual`, but off Windows a look at the player's folders
/// that did not answer in time is said as what it is (a disk that did not answer), with what to do.
fn plain_words(why: &str, usual: String) -> String {
    #[cfg(not(windows))]
    if why == LOOK_TOO_SLOW || why == LOOK_STOPPED {
        return "Your saves and changed settings are NOT being kept this run: they will be lost when the game closes. Before it writes anything, nv-rs looks at your own folders (Steam's libraries, Wine prefixes) so it never writes into them, and that look did not finish: a disk did not answer within 10 s (a sleeping or offline disk, or a network share listed as a Steam library). Wake or mount that disk, or take it out of Steam's library list, and start the game again.".to_owned();
    }
    let _ = why;
    usual
}

/// Writes `bytes` to the temporary file `temporary`. Off Windows a file (or a link) already there
/// is removed first (a link, not where it leads) and the file is made new, so a link planted under
/// the temporary name cannot be written through.
fn write_part(temporary: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(not(windows))]
    {
        use std::io::Write;
        match std::fs::remove_file(temporary) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary)?;
        file.write_all(bytes)
    }
    #[cfg(windows)]
    std::fs::write(temporary, bytes)
}

/// A warning the player can't miss: a line in the log, and a message box (from its own thread,
/// so the game goes on), unless `NV_RS_NO_WINDOWS` is set (scripted runs). Off Windows the line
/// also goes to the error output under a banner (a terminal shows it even when the log goes
/// elsewhere), and the box is the system's own small dialog program when there is one (macOS's
/// `osascript`; on Linux `zenity`, else `kdialog`): none there, the banner is all.
fn warn_visibly(line: &str, plain: &str) {
    println!("{line}");
    #[cfg(all(test, not(windows)))]
    WARNED
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(plain.to_owned());
    #[cfg(not(windows))]
    {
        eprintln!("\n*** nv-rs WARNING ***\n{plain}\n{line}\n");
        // Never from the unit tests, which refuse on purpose.
        if std::env::var_os("NV_RS_NO_WINDOWS").is_none() && !cfg!(test) {
            let text = format!("{plain}\n\n{line}");
            std::thread::spawn(move || {
                for mut command in dialog_commands(&text) {
                    if command
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()
                        .is_ok()
                    {
                        break;
                    }
                }
            });
        }
    }
    #[cfg(windows)]
    if std::env::var_os("NV_RS_NO_WINDOWS").is_none() {
        let text: Vec<u16> = format!("{plain}\n\n{line}")
            .encode_utf16()
            .chain([0])
            .collect();
        let title: Vec<u16> = "nv-rs".encode_utf16().chain([0]).collect();
        std::thread::spawn(move || {
            #[link(name = "user32")]
            extern "system" {
                fn MessageBoxW(
                    window: isize,
                    text: *const u16,
                    title: *const u16,
                    kind: u32,
                ) -> i32;
            }
            // MB_ICONWARNING | MB_SETFOREGROUND | MB_TOPMOST
            // SAFETY: both strings end in a NUL and live through the call.
            unsafe { MessageBoxW(0, text.as_ptr(), title.as_ptr(), 0x30 | 0x1_0000 | 0x4_0000) };
        });
    }
}

/// The dialog programs [`warn_visibly`] tries in order, `text` passed as an argument (never
/// through a shell or a script's text): the first that starts shows it.
#[cfg(any(not(windows), test))]
fn dialog_commands(text: &str) -> Vec<std::process::Command> {
    use std::process::Command;
    if cfg!(target_os = "macos") {
        let mut alert = Command::new("osascript");
        alert
            .args([
                "-e",
                "on run argv",
                "-e",
                "display alert \"nv-rs\" message (item 1 of argv) as warning",
                "-e",
                "end run",
            ])
            .arg(text);
        vec![alert]
    } else {
        let mut zenity = Command::new("zenity");
        zenity
            .args(["--warning", "--no-markup", "--title", "nv-rs", "--text"])
            .arg(text);
        let mut kdialog = Command::new("kdialog");
        kdialog.args(["--title", "nv-rs", "--sorry"]).arg(text);
        vec![zenity, kdialog]
    }
}

/// Off Windows: none (Windows' folder answers do not exist there). Its one caller off Windows is
/// `mod_files`, whose Windows-style check then refuses every folder there (it needs a drive
/// letter): mods' own settings last for a session off Windows until it uses this system's guard.
#[cfg(not(windows))]
pub fn known_folders() -> Vec<PathBuf> {
    Vec::new()
}

/// The player's Documents, AppData and Local AppData as Windows answers for them (wherever they
/// were moved to: OneDrive, another drive). Windows only: off Windows the guard is
/// `game_folders::never_written`, whose folders carry nv-rs's own folder as their exception.
#[cfg(windows)]
pub fn known_folders() -> Vec<PathBuf> {
    {
        #[link(name = "shell32")]
        extern "system" {
            fn SHGetFolderPathW(
                window: isize,
                csidl: i32,
                token: isize,
                flags: u32,
                path: *mut u16,
            ) -> i32;
        }
        let mut out = Vec::new();
        for csidl in [0x05, 0x1a, 0x1c] {
            let mut buffer = vec![0u16; 1024];
            // SAFETY: the buffer is longer than MAX_PATH, as the call asks.
            let hr = unsafe { SHGetFolderPathW(0, csidl, 0, 0, buffer.as_mut_ptr()) };
            if hr == 0 {
                let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
                out.push(PathBuf::from(String::from_utf16_lossy(&buffer[..end])));
            }
        }
        out
    }
}

/// The path the system really means: links, junctions, `subst` and short names resolved, a part
/// that does not exist yet kept after the real path of the longest part that does.
pub(crate) fn real_path(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path.to_path_buf();
    loop {
        if let Ok(real) = std::fs::canonicalize(&at) {
            let mut out = real;
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

/// Why `dir` may not receive saves: it lies inside one of the player's own folders under
/// `profile` (Documents, OneDrive's Documents, AppData) or inside one of the `known` folders
/// (Windows' own answer for Documents and AppData, wherever they were moved to), by its text or by
/// the real path it leads to (a junction or a drive made with `subst` doesn't get round this).
/// Compared without case, `/` read as `\`.
/// The rest of OneDrive is not the game's: a Desktop kept there is where many people unzip things.
pub fn refused(dir: &Path, profile: Option<&Path>, known: &[PathBuf]) -> Option<&'static str> {
    let norm = |p: &Path| {
        let text = p.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
        let text = match text.strip_prefix("\\\\?\\unc\\") {
            Some(rest) => format!("\\\\{rest}"),
            None => text
                .strip_prefix("\\\\?\\")
                .map_or(text.clone(), str::to_owned),
        };
        text.trim_end_matches('\\').to_owned()
    };
    let real_dir = norm(&real_path(dir));
    let dir = norm(dir);
    if !dir.contains(':') && !dir.starts_with("\\\\") {
        return Some("not a full path");
    }
    if dir.split('\\').any(|part| part == "..") {
        return Some("a `..` in the path");
    }
    let Some(profile) = profile else {
        return Some("USERPROFILE is not set, so the player's folders are unknown");
    };
    let profile = norm(profile);
    let mut roots: Vec<String> = ["documents", "onedrive\\documents", "appdata"]
        .iter()
        .map(|player| format!("{profile}\\{player}"))
        .collect();
    for folder in known {
        roots.push(norm(folder));
        roots.push(norm(&real_path(folder)));
    }
    for root in roots {
        for place in [&dir, &real_dir] {
            if *place == root || place.starts_with(&format!("{root}\\")) {
                return Some("inside the player's own folders");
            }
        }
    }
    None
}

/// `NV_PREFS_OUT`'s file, or `None` when it is unset or refused (refused once, with a line).
fn prefs_file() -> Option<&'static Path> {
    static FILE: OnceLock<Option<PathBuf>> = OnceLock::new();
    FILE.get_or_init(|| {
        let file = PathBuf::from(std::env::var_os("NV_PREFS_OUT")?);
        if let Some(why) = refused_here(&file) {
            warn_visibly(
                &format!(
                    "NV_PREFS_OUT={}: refused ({why}); settings are not copied.",
                    file.display()
                ),
                &plain_words(why, format!("Your changed game settings are NOT being kept: they will be lost when the game closes. The settings file the game was given is in a folder it must not write to. Start the game through {START}, and report this if you did.")),
            );
            return None;
        }
        Some(file)
    })
    .as_deref()
}

/// A save or its script extender co-save (`.fos`, `.nvse`, ADR-0016 §10).
fn is_save(key: &[u8]) -> bool {
    key.ends_with(b".fos") || key.ends_with(b".nvse")
}

/// The game's `FalloutPrefs.ini` (what `NV_PREFS_OUT` copies).
fn is_prefs(key: &[u8]) -> bool {
    key.ends_with(b"\\falloutprefs.ini")
}

/// One piece of the copy-out's disk work, made on main from the run's files ([`jobs`]) and done
/// on the copy-out's own thread ([`run`]), in the order made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// `bytes` into `target`: written to `temporary` first and then renamed, so a reader never
    /// sees half of it; `dir` made first when given. `key` is the file's key in the run's files;
    /// `copied` the name the log line gives (none for the settings file).
    Write {
        key: Vec<u8>,
        dir: Option<PathBuf>,
        target: PathBuf,
        temporary: PathBuf,
        bytes: Vec<u8>,
        copied: Option<String>,
    },
    /// `target` removed when it is a file (a save the game deleted); `name` for the log line.
    Remove { target: PathBuf, name: String },
}

/// What the copy-out has handed on so far ([`jobs`]): the run's change count at the last look,
/// the change stamp of each file's bytes handed on, and the deleted saves whose removal was
/// handed on.
#[derive(Debug, Default)]
pub struct Watch {
    seen: Option<u64>,
    done: HashMap<Vec<u8>, u64>,
    removed: HashSet<Vec<u8>>,
    /// The name each save was written under in this run, by key (the key is lower case; the
    /// copy keeps the game's own case, which a Linux disk compares exactly).
    names: HashMap<Vec<u8>, String>,
}

impl Watch {
    /// A copy that failed is made again at the next look ([`copy_saves`] with `look_again`).
    pub fn failed(&mut self, key: &[u8]) {
        self.done.remove(key);
        self.seen = None;
    }
}

/// The jobs for what changed in the run's files since the last look: each save (into `dir`) and
/// the settings file (into `prefs`) whose bytes changed, and each save the game deleted that is
/// not there again (removed from `dir`). Nothing when no file changed since the last look, which
/// is one comparison (`Disk::changes`): this runs on main every frame. The bytes are copied here
/// (the game may write the file again); the disk work is the jobs'.
pub fn jobs(e: &Engine, watch: &mut Watch, dir: Option<&Path>, prefs: Option<&Path>) -> Vec<Job> {
    let disk = &engine_platform::state(&e.kernel.platform).disk;
    if watch.seen == Some(disk.changes) {
        return vec![];
    }
    watch.seen = Some(disk.changes);
    let mut jobs = Vec::new();
    // The first settings file, as the copy-out always took it.
    let prefs_key = prefs.and_then(|_| disk.files.keys().find(|key| is_prefs(key)));
    for (key, &stamp) in &disk.stamps {
        let save = dir.is_some() && is_save(key);
        if !save && prefs_key != Some(key) {
            continue;
        }
        if watch.done.get(key) == Some(&stamp) {
            continue;
        }
        let Some(bytes) = disk.files.get(key) else {
            continue;
        };
        watch.done.insert(key.clone(), stamp);
        watch.removed.remove(key);
        let job = match (save, dir, prefs) {
            (true, Some(dir), _) => {
                let name = key.rsplit(|&b| b == b'\\').next().unwrap_or(key);
                let name = String::from_utf8_lossy(name).into_owned();
                let name = disk
                    .entries
                    .get(key)
                    .map_or(name, |entry| entry.name.clone());
                watch.names.insert(key.clone(), name.clone());
                Job::Write {
                    key: key.clone(),
                    dir: Some(dir.to_path_buf()),
                    target: dir.join(&name),
                    temporary: dir.join(format!("{name}.part")),
                    bytes: bytes.clone(),
                    copied: Some(name),
                }
            }
            (_, _, Some(file)) => Job::Write {
                key: key.clone(),
                dir: None,
                target: file.to_path_buf(),
                temporary: file.with_extension("ini.part"),
                bytes: bytes.clone(),
                copied: None,
            },
            _ => continue,
        };
        jobs.push(job);
    }
    if let Some(dir) = dir {
        for key in &disk.deleted {
            if !is_save(key) || disk.files.contains_key(key) || watch.removed.contains(key) {
                continue;
            }
            watch.removed.insert(key.clone());
            watch.done.remove(key);
            let name = key.rsplit(|&b| b == b'\\').next().unwrap_or(key);
            let name = String::from_utf8_lossy(name).into_owned();
            // Off Windows the name the copy was written under in this run (the key is lower
            // case, and Linux compares names exactly); a copy from an earlier run is found by
            // the delete itself ([`run_checked`]). Windows compares without case: as before.
            let written = watch.names.remove(key);
            #[cfg(not(windows))]
            let name = written.unwrap_or(name);
            #[cfg(windows)]
            let _ = written;
            jobs.push(Job::Remove {
                target: dir.join(&name),
                name,
            });
        }
    }
    jobs
}

/// Does `job`'s disk work: the log line when there is one (`Ok`), or the line saying why a
/// write failed (`Err`).
pub fn run(job: &Job) -> Result<Option<String>, String> {
    #[cfg(not(windows))]
    let _forget = ForgetFolder(job);
    // Off Windows the guard is asked again at write time; on Windows nothing is (the folder was
    // checked once, at the start, as before).
    #[cfg(not(windows))]
    let check = |path: &Path| {
        let why = refused_here(path);
        if let Some(why) = why {
            warn_refused_mid_run(path, why);
        }
        why
    };
    #[cfg(windows)]
    let check = |_: &Path| -> Option<&'static str> { None };
    run_checked(job, &check)
}

/// Off Windows: a write or a delete refused at write time (nv-rs's save folder turned into a
/// link while the game runs, a look at the player's folders that did not answer): said to the
/// player as visibly as the refusal at the start, once a run (the log has every one).
#[cfg(not(windows))]
fn warn_refused_mid_run(path: &Path, why: &str) {
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| {
        warn_visibly(
            &format!("NV_SAVE_OUT: {}: refused at write time ({why})", path.display()),
            &plain_words(why, format!("Your saves and changed settings are NOT being kept from now on: they will be lost when the game closes. nv-rs's save folder changed while the game was running, so that it now leads into one of your own folders (Steam's, Wine's or your Documents), which nv-rs never writes into. Put nv-rs's own folder back as a plain folder (not a link) and start the game again through {START}.")),
        );
    });
}

/// [`run`], with `check` saying why a path may not be written or deleted (`None`: it may).
///
/// Asked for the file and its temporary name just before the disk work, so a folder link made
/// after the start (the save folder swapped for a link into the player's Proton saves) is
/// refused. What this cannot close: a folder link swapped in after the check and before the file
/// is opened (or deleted) would be followed. That window is not only microseconds: the check
/// looks up where every guarded folder really is (one look per Steam library, prefix and saves
/// folder), so with a library on a sleeping disk it lasts as long as that disk takes to answer.
/// Closing it needs the folder opened once and the file made relative to it, which Rust's
/// standard library does not offer (no new dependency was added for it). To use it, another
/// program of the player's own would have to replace nv-rs's Saves folder (or one above it) with
/// a link into the player's own saves inside that window; nothing nv-rs or play.sh does makes
/// such a link. The player READ MEs say so. Not detected at all: a BIND MOUNT of one of the
/// player's folders (a Proton prefix's or Documents' saves) onto nv-rs's own folder. The guard
/// compares where paths lead (links resolved), and a bind mount is not a link: the same folder
/// shows under a second path that resolves to itself. It was left out on purpose: making one
/// needs administrator rights (`mount --bind`, an fstab line) or a deliberate tool (bindfs),
/// nothing nv-rs or play.sh does makes one, and the unit tests (which run without those rights)
/// could not make one to test a check of folder identity, so it would ship untested. The player READ MEs say
/// this too. A link planted at the temporary
/// name is never written through (`write_part`), and the rename replaces a link at the save's
/// own name instead of following it.
fn run_checked(
    job: &Job,
    check: &dyn Fn(&Path) -> Option<&'static str>,
) -> Result<Option<String>, String> {
    match job {
        Job::Write {
            dir,
            target,
            temporary,
            bytes,
            copied,
            ..
        } => {
            if let Some(why) = check(target).or_else(|| check(temporary)) {
                let which = if copied.is_some() {
                    "NV_SAVE_OUT"
                } else {
                    "NV_PREFS_OUT"
                };
                return Err(format!(
                    "{which}: {} not copied: refused ({why})",
                    target.display()
                ));
            }
            let written = dir
                .as_deref()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| write_part(temporary, bytes))
                .and_then(|()| std::fs::rename(temporary, target));
            match (written, copied) {
                (Ok(()), Some(name)) => Ok(Some(format!("NV_SAVE_OUT: copied {name}"))),
                (Ok(()), None) => Ok(None),
                (Err(err), Some(_)) => Err(format!(
                    "NV_SAVE_OUT: {} not copied: {err}",
                    target.display()
                )),
                (Err(err), None) => Err(format!(
                    "NV_PREFS_OUT: {} not copied: {err}",
                    target.display()
                )),
            }
        }
        Job::Remove { target, name } => {
            // A deletion is a write into the folder: the same check (a folder link made after
            // the start cannot lead it into a player's folder).
            if let Some(why) = check(target) {
                return Err(format!(
                    "NV_SAVE_OUT: {} not deleted: refused ({why})",
                    target.display()
                ));
            }
            // Off Windows only a plain file is deleted (not a link at the save's name); a copy
            // whose name differs only in case (written by an earlier run: the game's key is lower
            // case, Linux compares names exactly) is found as the one such plain file.
            #[cfg(not(windows))]
            let (target, name) = match same_but_case(target) {
                Some(found) => {
                    if let Some(why) = check(&found) {
                        return Err(format!(
                            "NV_SAVE_OUT: {} not deleted: refused ({why})",
                            found.display()
                        ));
                    }
                    let shown = found
                        .file_name()
                        .map_or_else(|| name.clone(), |n| n.to_string_lossy().into_owned());
                    (found, shown)
                }
                None => (target.clone(), name.clone()),
            };
            #[cfg(not(windows))]
            let file = std::fs::symlink_metadata(&target).is_ok_and(|m| m.is_file());
            #[cfg(windows)]
            let file = target.is_file();
            let removed = file && std::fs::remove_file(target).is_ok();
            Ok(removed.then(|| format!("NV_SAVE_OUT: copied {name} (deleted)")))
        }
    }
}

/// Off Windows: when nothing is at `target`, the one plain file in its folder whose name is
/// `target`'s without ASCII case (the game's key for a save is lower case; its copy keeps the
/// game's own case); `None` when something is at `target`, or there is no such file, or more
/// than one.
#[cfg(not(windows))]
fn same_but_case(target: &Path) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    if std::fs::symlink_metadata(target).is_ok() {
        return None;
    }
    let (dir, name) = (target.parent()?, target.file_name()?);
    let mut found = std::fs::read_dir(dir).ok()?.flatten().filter(|entry| {
        entry
            .file_name()
            .as_bytes()
            .eq_ignore_ascii_case(name.as_bytes())
    });
    let only = found.next()?;
    if found.next().is_some() {
        return None;
    }
    let path = only.path();
    std::fs::symlink_metadata(&path)
        .is_ok_and(|m| m.is_file())
        .then_some(path)
}

/// Off Windows, when a job's disk work is done (dropped at the end of [`run`]): the names of the
/// job's folder kept for the game's paths are read again at their next look
/// (`engine_platform::host_paths::forget`), whatever the folder's time says.
#[cfg(not(windows))]
struct ForgetFolder<'a>(&'a Job);

#[cfg(not(windows))]
impl Drop for ForgetFolder<'_> {
    fn drop(&mut self) {
        let (Job::Write { target, .. } | Job::Remove { target, .. }) = self.0;
        if let Some(folder) = target.parent() {
            engine_platform::host_paths::forget(folder);
        }
    }
}

/// A message to the copy-out's thread.
enum Message {
    Job(Job),
    /// Answers once every job sent before it is done.
    Flush(std::sync::mpsc::SyncSender<()>),
}

/// The keys whose copy failed on the copy-out's thread, for [`copy_saves`] to make again.
static FAILED: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Does one job on the copy-out's thread or, when that is gone, on the caller's: its line
/// printed, a failed write's key kept for the next look. A panic in it (the disk work, the
/// printing) is caught, said, and counted as a failed write, so the thread goes on.
fn do_job(job: &Job) {
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(job)));
    let failed = match done {
        Ok(Ok(Some(line))) => {
            println!("{line}");
            false
        }
        Ok(Ok(None)) => false,
        Ok(Err(line)) => {
            println!("{line}");
            true
        }
        Err(_) => {
            println!("NV_SAVE_OUT: a copy stopped with a panic; it is made again at the next look");
            true
        }
    };
    if let (true, Job::Write { key, .. }) = (failed, job) {
        FAILED
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(key.clone());
    }
}

/// The copy-out's thread, started with the first job: it does the jobs in the order sent, so the
/// disk work (writing a save of several megabytes, renaming it, removing one) never holds up a
/// frame. Its start registers [`flush`] with the platform layer, which runs it before it ends the
/// process itself (the game's window closed: `engine_platform::on_exit`).
fn writer() -> &'static std::sync::mpsc::Sender<Message> {
    static SENDER: OnceLock<std::sync::mpsc::Sender<Message>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (send, receive) = std::sync::mpsc::channel::<Message>();
        let started = std::thread::Builder::new()
            .name("nv save out".into())
            .spawn(move || {
                for message in receive {
                    match message {
                        Message::Job(job) => do_job(&job),
                        Message::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            });
        if let Err(why) = started {
            // No thread: the jobs are done on the caller's ([`hand`]), as the receiver is gone.
            println!("NV_SAVE_OUT: the copy-out's thread could not start ({why})");
        }
        engine_platform::on_exit(flush);
        send
    })
}

/// Hands `job` to the copy-out's thread through `send`, or, when that thread is gone (it never
/// started, or it ended), does it here, saying so once: no copy is dropped unsaid.
fn hand(send: &std::sync::mpsc::Sender<Message>, job: Job) {
    if let Err(std::sync::mpsc::SendError(Message::Job(job))) = send.send(Message::Job(job)) {
        static SAID: std::sync::Once = std::sync::Once::new();
        SAID.call_once(|| {
            println!(
                "NV_SAVE_OUT: the copy-out's thread is gone; saves are copied on the main thread from now on"
            )
        });
        do_job(&job);
    }
}

/// Whether the copy-out's thread was started (a job was sent).
static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `NV_SAVE_OUT` and `NV_PREFS_OUT`, each main frame: what changed in the run's files since the
/// last call handed to the copy-out's thread ([`jobs`]); nothing at all while no file changed.
/// `look_again` (the 5 s status line): copies that failed are made again.
pub fn copy_saves(e: &Engine, look_again: bool) {
    let (dir, prefs) = (out_dir(), prefs_file());
    if dir.is_none() && prefs.is_none() {
        return;
    }
    static WATCH: Mutex<Option<Watch>> = Mutex::new(None);
    let mut watch = WATCH.lock().unwrap_or_else(|p| p.into_inner());
    let watch = watch.get_or_insert_with(Watch::default);
    if look_again {
        for key in std::mem::take(&mut *FAILED.lock().unwrap_or_else(|p| p.into_inner())) {
            watch.failed(&key);
        }
    }
    let jobs = jobs(e, watch, dir, prefs);
    if jobs.is_empty() {
        return;
    }
    STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
    let send = writer();
    for job in jobs {
        hand(send, job);
    }
}

/// Waits until every job handed to the copy-out's thread is done (the run is ending).
pub fn flush() {
    if !STARTED.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let (done, wait) = std::sync::mpsc::sync_channel(1);
    if writer().send(Message::Flush(done)).is_ok() {
        let _ = wait.recv();
    }
}

/// The run ends cleanly: what changed since the last frame handed on, and all of it written.
pub fn finish(e: &Engine) {
    copy_saves(e, true);
    flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_players_folders_are_refused_and_scratch_folders_taken() {
        let profile = Path::new(r"C:\Users\<name>");
        for dir in [
            r"C:\Users\<name>\Documents\My Games\FalloutNV\Saves",
            r"c:/users/<name>/onedrive/documents/x",
            r"C:\Users\<name>\AppData\Local\FalloutNV",
            r"C:\Users\<name>\Documents",
            "relative\\saves",
            r"C:\Users\<name>\nv-re\..\Documents\My Games",
        ] {
            assert!(
                refused(Path::new(dir), Some(profile), &[]).is_some(),
                "{dir}"
            );
        }
        assert!(refused(Path::new(r"D:\scratch"), None, &[]).is_some());
        for dir in [
            r"C:\Users\<name>\nv-re\work\build43-scratch\docs-port\My Games\FalloutNV\Saves",
            r"C:\Users\<name>\DocumentsScratch",
            r"C:\Users\<name>\OneDrive\Desktop\nv-rs\userdata\Documents\My Games\FalloutNV\Saves",
            r"D:\scratch",
        ] {
            assert_eq!(refused(Path::new(dir), Some(profile), &[]), None, "{dir}");
        }
    }

    /// The warning's dialog gets the text as one argument of its own, quotes and all, never inside
    /// a script's or a shell's text.
    #[test]
    fn the_warning_dialog_gets_the_text_as_one_argument() {
        let text = "a \"quoted\" line; $(not run) `x`\n\nsecond";
        let commands = dialog_commands(text);
        assert!(!commands.is_empty());
        for command in &commands {
            let args: Vec<_> = command.get_args().collect();
            assert_eq!(args.last().copied(), Some(std::ffi::OsStr::new(text)));
            assert_eq!(args.iter().filter(|a| **a == text).count(), 1);
        }
    }

    /// Documents moved away from the profile (OneDrive, another drive) is still the player's
    /// Documents, and so is a junction that leads into it.
    #[test]
    fn a_redirected_documents_and_a_junction_into_it_are_refused() {
        let profile = Path::new(r"C:\Users\<name>");
        let moved = vec![PathBuf::from(r"D:\Moved\Documents")];
        let saves = Path::new(r"D:\Moved\Documents\My Games\FalloutNV\Saves");
        assert!(refused(saves, Some(profile), &moved).is_some());
        assert!(refused(Path::new("d:/moved/documents"), Some(profile), &moved).is_some());
        for fine in [r"D:\Moved\DocumentsScratch", r"D:\Moved\Other"] {
            assert_eq!(
                refused(Path::new(fine), Some(profile), &moved),
                None,
                "{fine}"
            );
        }
        #[cfg(windows)]
        {
            let base =
                std::env::temp_dir().join(format!("nv-save-junction-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            let documents = base.join("Moved Documents");
            std::fs::create_dir_all(&documents).unwrap();
            let link = base.join("innocent");
            let made = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&link)
                .arg(&documents)
                .output()
                .unwrap();
            assert!(made.status.success(), "{made:?}");
            let known = vec![documents.clone()];
            assert!(refused(&link.join("Saves"), Some(profile), &known).is_some());
            assert!(refused(&documents.join("Saves"), Some(profile), &known).is_some());
            assert_eq!(
                refused(&base.join("elsewhere"), Some(profile), &known),
                None
            );
            let _ = std::fs::remove_dir_all(&base);
        }
    }

    /// The jobs of one look, done at once (as the copy-out's thread does them, in order): the log
    /// lines.
    fn look(
        e: &Engine,
        watch: &mut Watch,
        dir: Option<&Path>,
        prefs: Option<&Path>,
    ) -> Vec<String> {
        jobs(e, watch, dir, prefs)
            .iter()
            .filter_map(|job| run(job).unwrap())
            .collect()
    }

    /// `key` with `bytes` among the run's files, stamped as a write of the game's stamps it.
    fn put(e: &mut Engine, key: &[u8], bytes: &[u8]) {
        let disk = &mut engine_platform::state_mut(&mut e.kernel.platform).disk;
        disk.files.insert(key.to_vec(), bytes.to_vec());
        disk.changed(key);
    }

    #[test]
    fn new_and_changed_saves_are_copied_once_each() {
        let dir = std::env::temp_dir().join(format!("nv-save-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut e = Engine::new();
        let saves = br"c:\scratch\my games\falloutnv\saves\";
        let key = |name: &str| [saves.as_slice(), name.as_bytes()].concat();
        put(&mut e, &key("quicksave.fos"), b"FO3SAVEGAME one");
        put(&mut e, &key("falloutprefs.ini"), b"x");
        let mut watch = Watch::default();
        assert_eq!(
            look(&e, &mut watch, Some(&dir), None),
            vec!["NV_SAVE_OUT: copied quicksave.fos".to_string()]
        );
        assert_eq!(
            std::fs::read(dir.join("quicksave.fos")).unwrap(),
            b"FO3SAVEGAME one"
        );
        assert!(!dir.join("falloutprefs.ini").exists());
        assert!(!dir.join("quicksave.fos.part").exists());
        assert!(look(&e, &mut watch, Some(&dir), None).is_empty());
        put(&mut e, &key("quicksave.fos"), b"FO3SAVEGAME two");
        assert_eq!(
            look(&e, &mut watch, Some(&dir), None),
            vec!["NV_SAVE_OUT: copied quicksave.fos".to_string()]
        );
        assert_eq!(
            std::fs::read(dir.join("quicksave.fos")).unwrap(),
            b"FO3SAVEGAME two"
        );
        // The script extender's co-save goes with it (ADR-0016 §10).
        put(&mut e, &key("quicksave.nvse"), b"NVSE");
        assert_eq!(
            look(&e, &mut watch, Some(&dir), None),
            vec!["NV_SAVE_OUT: copied quicksave.nvse".to_string()]
        );
        assert_eq!(std::fs::read(dir.join("quicksave.nvse")).unwrap(), b"NVSE");
        // A change to a file that is not a save hands on nothing.
        put(&mut e, &key("note.txt"), b"x");
        assert!(look(&e, &mut watch, Some(&dir), None).is_empty());
        // A failed copy is made again once said so.
        watch.failed(&key("quicksave.fos"));
        assert_eq!(
            look(&e, &mut watch, Some(&dir), None),
            vec!["NV_SAVE_OUT: copied quicksave.fos".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The bytes a job carries are the run's bytes when the look was made: the game writing the
    /// file again before the copy-out's thread gets to it changes neither the copy nor the next
    /// look's job, which carries the new bytes.
    /// Linux and macOS: a save the copy-out writes, and one it removes, are seen by the game's
    /// next look in that folder even when the folder's time does not show the change (it is put
    /// back here, as a file system whose clock does not move would leave it).
    #[cfg(not(windows))]
    #[test]
    fn the_copy_outs_writes_are_seen_by_the_games_next_look() {
        use engine_platform::host_paths::folder_matches;
        let dir = std::env::temp_dir().join(format!("nv-rs-copy-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Old.fos"), b"old").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let set_time = || {
            std::fs::File::open(&dir)
                .unwrap()
                .set_modified(old)
                .unwrap()
        };
        set_time();
        let folder = dir.to_string_lossy().into_owned();
        assert_eq!(folder_matches(&folder, "NEW.FOS"), Some(vec![]));
        let write = Job::Write {
            key: b"new.fos".to_vec(),
            dir: Some(dir.clone()),
            target: dir.join("New.fos"),
            temporary: dir.join("New.fos.part"),
            bytes: b"new".to_vec(),
            copied: Some("New.fos".to_string()),
        };
        run(&write).unwrap();
        set_time();
        assert_eq!(
            folder_matches(&folder, "new.fos"),
            Some(vec!["New.fos".to_string()])
        );
        let remove = Job::Remove {
            target: dir.join("Old.fos"),
            name: "Old.fos".to_string(),
        };
        run(&remove).unwrap();
        set_time();
        assert_eq!(folder_matches(&folder, "OLD.FOS"), Some(vec![]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_job_keeps_the_bytes_of_its_look() {
        let mut e = Engine::new();
        let key = br"c:\scratch\saves\quicksave.fos".to_vec();
        put(&mut e, &key, b"first");
        let dir = PathBuf::from(r"c:\nowhere");
        let mut watch = Watch::default();
        let first = jobs(&e, &mut watch, Some(&dir), None);
        put(&mut e, &key, b"second");
        let second = jobs(&e, &mut watch, Some(&dir), None);
        let bytes = |jobs: &[Job]| match jobs {
            [Job::Write { bytes, .. }] => bytes.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(bytes(&first), b"first");
        assert_eq!(bytes(&second), b"second");
    }

    #[test]
    fn a_save_the_game_deleted_goes_and_its_settings_are_copied() {
        let dir = std::env::temp_dir().join(format!("nv-save-del-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut e = Engine::new();
        let saves = br"c:\scratch\my games\falloutnv\";
        let key = |name: &str| [saves.as_slice(), name.as_bytes()].concat();
        {
            let disk = &mut engine_platform::state_mut(&mut e.kernel.platform).disk;
            for name in [r"saves\old.fos", r"saves\made again.fos", r"saves\note.txt"] {
                disk.deleted.insert(key(name));
                disk.gone(&key(name));
            }
        }
        put(&mut e, &key(r"saves\made again.fos"), &[1]);
        put(&mut e, &key("falloutprefs.ini"), b"one");
        let saves_dir = dir.join("saves");
        std::fs::create_dir_all(&saves_dir).unwrap();
        for name in ["old.fos", "made again.fos", "kept.fos"] {
            std::fs::write(saves_dir.join(name), b"x").unwrap();
        }
        let prefs = dir.join("FalloutPrefs.ini");
        let mut watch = Watch::default();
        let mut lines = look(&e, &mut watch, Some(&saves_dir), Some(&prefs));
        lines.sort();
        assert_eq!(
            lines,
            vec![
                "NV_SAVE_OUT: copied made again.fos".to_string(),
                "NV_SAVE_OUT: copied old.fos (deleted)".to_string()
            ]
        );
        assert!(!saves_dir.join("old.fos").exists());
        assert_eq!(
            std::fs::read(saves_dir.join("made again.fos")).unwrap(),
            [1]
        );
        assert!(saves_dir.join("kept.fos").exists());
        assert_eq!(std::fs::read(&prefs).unwrap(), b"one");
        std::fs::remove_file(&prefs).unwrap();
        // The same bytes are not copied again, and a removal is made once.
        std::fs::write(saves_dir.join("old.fos"), b"x").unwrap();
        put(&mut e, &key(r"saves\note.txt"), b"y");
        assert!(look(&e, &mut watch, Some(&saves_dir), Some(&prefs)).is_empty());
        assert!(!prefs.exists(), "the same bytes are not copied again");
        assert!(saves_dir.join("old.fos").exists(), "removed once only");
        // Settings alone, without a save folder.
        put(&mut e, &key("falloutprefs.ini"), b"two");
        assert!(look(&e, &mut watch, None, Some(&prefs)).is_empty());
        assert_eq!(std::fs::read(&prefs).unwrap(), b"two");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The thread: jobs sent are done by the time [`flush`] answers, in the order sent (a save
    /// written and then removed is gone). What a run that ends right after a save relies on.
    #[test]
    fn a_flush_waits_for_every_job_sent() {
        let dir = std::env::temp_dir().join(format!("nv-save-flush-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let big = vec![7u8; 8 << 20];
        let write = |name: &str, bytes: Vec<u8>| Job::Write {
            key: name.as_bytes().to_vec(),
            dir: Some(dir.clone()),
            target: dir.join(name),
            temporary: dir.join(format!("{name}.part")),
            bytes,
            copied: Some(name.to_string()),
        };
        STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
        for job in [
            write("a.fos", big.clone()),
            write("b.fos", b"b".to_vec()),
            Job::Remove {
                target: dir.join("b.fos"),
                name: "b.fos".into(),
            },
        ] {
            writer().send(Message::Job(job)).unwrap();
        }
        flush();
        assert_eq!(std::fs::read(dir.join("a.fos")).unwrap(), big);
        assert!(!dir.join("b.fos").exists());
        assert!(!dir.join("a.fos.part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
    /// A save handed to the copy-out's thread just before the platform layer ends the process
    /// (the game's window closed) is on disk once the exit hooks have run: the thread's start
    /// registered the flush (`engine_platform::on_exit`).
    #[test]
    fn the_platforms_exit_hook_finishes_the_copies_handed_on() {
        let dir = std::env::temp_dir().join(format!("nv-save-exit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let bytes = vec![5u8; 6 << 20];
        let job = Job::Write {
            key: b"exit.fos".to_vec(),
            dir: Some(dir.clone()),
            target: dir.join("exit.fos"),
            temporary: dir.join("exit.fos.part"),
            bytes: bytes.clone(),
            copied: Some("exit.fos".into()),
        };
        STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
        hand(writer(), job);
        engine_platform::run_exit_hooks();
        assert_eq!(std::fs::read(dir.join("exit.fos")).unwrap(), bytes);
        assert!(!dir.join("exit.fos.part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With the copy-out's thread gone (its receiver dropped), a job is done on the caller's
    /// thread at once: no copy is dropped.
    #[test]
    fn with_the_thread_gone_a_copy_is_made_on_the_callers_thread() {
        let dir = std::env::temp_dir().join(format!("nv-save-gone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (send, receive) = std::sync::mpsc::channel::<Message>();
        drop(receive);
        hand(
            &send,
            Job::Write {
                key: b"gone.fos".to_vec(),
                dir: Some(dir.clone()),
                target: dir.join("gone.fos"),
                temporary: dir.join("gone.fos.part"),
                bytes: b"kept".to_vec(),
                copied: Some("gone.fos".into()),
            },
        );
        assert_eq!(std::fs::read(dir.join("gone.fos")).unwrap(), b"kept");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A copy an earlier run's kill cut short leaves `<save>.part`: removed at the next start,
    /// the saves and other files left.
    #[test]
    fn copies_cut_short_by_an_earlier_kill_are_removed() {
        let dir = std::env::temp_dir().join(format!("nv-save-part-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["a.fos", "a.fos.part", "b.NVSE.part", "notes.part", "c.fos"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let mut removed = clean_parts(&dir);
        removed.sort();
        assert_eq!(
            removed,
            vec!["a.fos.part".to_string(), "b.NVSE.part".to_string()]
        );
        for kept in ["a.fos", "notes.part", "c.fos"] {
            assert!(dir.join(kept).exists(), "{kept}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Review of #630 (round 2): the guard off Windows at write and delete time. Made-up folders
    /// in the temporary folder: `home/Documents/My Games/FalloutNV/Saves` stands for the player's
    /// own saves (a guarded folder), `own/Saves` for nv-rs's own save folder.
    #[cfg(unix)]
    mod guard_at_write_time {
        use super::super::*;
        use game_folders::Guarded;

        struct Scratch(PathBuf);

        impl Scratch {
            fn new(name: &str) -> Scratch {
                let dir = std::env::temp_dir()
                    .join(format!("nv-save-guard-{name}-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&dir);
                std::fs::create_dir_all(&dir).unwrap();
                // The real path, so the guard's look (links resolved) and the text agree.
                Scratch(std::fs::canonicalize(&dir).unwrap())
            }

            /// The player's saves folder (guarded), with `x.fos` in it, and nv-rs's own saves.
            fn folders(&self) -> (PathBuf, PathBuf, Vec<Guarded>) {
                let players = self.0.join("home/Documents/My Games/FalloutNV/Saves");
                std::fs::create_dir_all(&players).unwrap();
                std::fs::write(players.join("x.fos"), b"the player's own save").unwrap();
                let own = self.0.join("own/Saves");
                std::fs::create_dir_all(&own).unwrap();
                let never = vec![Guarded {
                    root: self.0.join("home/Documents"),
                    except: Some(self.0.join("own")),
                    system: game_folders::System::this(),
                }];
                (players, own, never)
            }
        }

        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        fn write(dir: &Path, name: &str, bytes: &[u8]) -> Job {
            Job::Write {
                key: name.as_bytes().to_vec(),
                dir: Some(dir.to_path_buf()),
                target: dir.join(name),
                temporary: dir.join(format!("{name}.part")),
                bytes: bytes.to_vec(),
                copied: Some(name.to_string()),
            }
        }

        /// The save folder swapped for a link into the player's saves after the start: the write
        /// is refused and the player's save is untouched; before the swap it went through.
        #[test]
        fn a_write_through_a_folder_link_made_after_the_start_is_refused() {
            let s = Scratch::new("write");
            let (players, own, never) = s.folders();
            let check = |p: &Path| game_folders::refused(p, &never);
            let job = write(&own, "x.fos", b"nv-rs's copy");
            assert_eq!(
                run_checked(&job, &check),
                Ok(Some("NV_SAVE_OUT: copied x.fos".to_string()))
            );
            std::fs::remove_dir_all(&own).unwrap();
            std::os::unix::fs::symlink(&players, &own).unwrap();
            let refused = run_checked(&job, &check).unwrap_err();
            assert!(refused.contains("refused"), "{refused}");
            assert_eq!(
                std::fs::read(players.join("x.fos")).unwrap(),
                b"the player's own save"
            );
            assert!(!players.join("x.fos.part").exists());
        }

        /// A link planted at the temporary name is not written through: it is replaced by a new
        /// file, and the file it led to keeps its bytes.
        #[test]
        fn a_link_planted_at_the_temporary_name_is_not_written_through() {
            let s = Scratch::new("part");
            let (_, own, never) = s.folders();
            let victim = s.0.join("victim.txt");
            std::fs::write(&victim, b"not nv-rs's").unwrap();
            std::os::unix::fs::symlink(&victim, own.join("y.fos.part")).unwrap();
            let check = |p: &Path| game_folders::refused(p, &never);
            assert_eq!(
                run_checked(&write(&own, "y.fos", b"the copy"), &check),
                Ok(Some("NV_SAVE_OUT: copied y.fos".to_string()))
            );
            assert_eq!(std::fs::read(&victim).unwrap(), b"not nv-rs's");
            assert_eq!(std::fs::read(own.join("y.fos")).unwrap(), b"the copy");
            assert!(std::fs::symlink_metadata(own.join("y.fos.part")).is_err());
        }

        /// A deletion through a folder link made after the start is refused: the player's save
        /// the link leads to is still there. A link at the save's own name is not deleted, and
        /// neither is the file it leads to; a plain file is.
        #[test]
        fn a_delete_through_a_folder_link_made_after_the_start_is_refused() {
            let s = Scratch::new("delete");
            let (players, own, never) = s.folders();
            let check = |p: &Path| game_folders::refused(p, &never);
            let remove = |name: &str| Job::Remove {
                target: own.join(name),
                name: name.to_string(),
            };
            std::fs::write(own.join("plain.fos"), b"x").unwrap();
            assert_eq!(
                run_checked(&remove("plain.fos"), &check),
                Ok(Some("NV_SAVE_OUT: copied plain.fos (deleted)".to_string()))
            );
            assert!(!own.join("plain.fos").exists());
            let elsewhere = s.0.join("elsewhere.fos");
            std::fs::write(&elsewhere, b"kept").unwrap();
            std::os::unix::fs::symlink(&elsewhere, own.join("linked.fos")).unwrap();
            assert_eq!(run_checked(&remove("linked.fos"), &check), Ok(None));
            assert!(std::fs::symlink_metadata(own.join("linked.fos")).is_ok());
            assert_eq!(std::fs::read(&elsewhere).unwrap(), b"kept");
            std::fs::remove_dir_all(&own).unwrap();
            std::os::unix::fs::symlink(&players, &own).unwrap();
            let refused = run_checked(&remove("x.fos"), &check).unwrap_err();
            assert!(refused.contains("not deleted: refused"), "{refused}");
            assert_eq!(
                std::fs::read(players.join("x.fos")).unwrap(),
                b"the player's own save"
            );
        }

        /// A look at the player's folders that never answers refuses after the wait, saying it was
        /// a disk that did not answer (not a wrong save folder); one whose thread stopped says
        /// that; an answer is taken.
        #[test]
        fn a_look_that_never_answers_refuses_and_says_so() {
            let (send, receive) = std::sync::mpsc::channel();
            let wait = std::time::Duration::from_millis(50);
            assert_eq!(wait_for_look(Some(receive), wait), Err(LOOK_TOO_SLOW));
            drop(send);
            let words = plain_words(LOOK_TOO_SLOW, "usual".into());
            assert!(words.contains("a disk did not answer"), "{words}");
            assert!(!words.contains("must not write to"), "{words}");
            let (send, receive) = std::sync::mpsc::channel::<Option<Vec<Guarded>>>();
            drop(send);
            assert_eq!(wait_for_look(Some(receive), wait), Err(LOOK_STOPPED));
            assert!(wait_for_look(None, wait).is_err());
            let (send, receive) = std::sync::mpsc::channel();
            send.send(Some(vec![])).unwrap();
            assert_eq!(wait_for_look(Some(receive), wait), Ok(vec![]));
            assert_eq!(plain_words("a `..` in the path", "usual".into()), "usual");
        }

        /// Review of #630 (round 3): the production path, as the game's copy-out runs it ([`run`]
        /// with the process's own look at the player's folders): a write into nv-rs's own folder
        /// goes through; once that folder is swapped for a link into a Proton prefix's saves, a
        /// write and a delete are refused and the player's save is untouched.
        #[test]
        fn the_production_check_refuses_a_write_and_a_delete_through_a_swapped_folder() {
            let s = Scratch::new("production");
            let players = s.0.join(
                "lib/steamapps/compatdata/22380/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV/Saves",
            );
            std::fs::create_dir_all(&players).unwrap();
            std::fs::write(players.join("x.fos"), b"the player's own save").unwrap();
            let own = s.0.join("own/Saves");
            std::fs::create_dir_all(&own).unwrap();
            assert_eq!(
                run(&write(&own, "first.fos", b"kept")),
                Ok(Some("NV_SAVE_OUT: copied first.fos".to_string()))
            );
            assert_eq!(std::fs::read(own.join("first.fos")).unwrap(), b"kept");
            std::fs::remove_dir_all(&own).unwrap();
            std::os::unix::fs::symlink(&players, &own).unwrap();
            let refused = run(&write(&own, "x.fos", b"nv-rs's copy")).unwrap_err();
            assert!(refused.contains("refused"), "{refused}");
            let refused = run(&Job::Remove {
                target: own.join("x.fos"),
                name: "x.fos".into(),
            })
            .unwrap_err();
            assert!(refused.contains("not deleted: refused"), "{refused}");
            assert_eq!(
                std::fs::read(players.join("x.fos")).unwrap(),
                b"the player's own save"
            );
            assert!(!players.join("x.fos.part").exists());
            // Review of #630 (round 4): the refusals mid-run were said to the player, once.
            let warned: Vec<String> = WARNED
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .iter()
                .filter(|w| w.contains("NOT being kept from now on"))
                .cloned()
                .collect();
            assert_eq!(warned.len(), 1, "{warned:?}");
            // Review of #630 (round 4): the guard's LIST (from HOME, as prepare reads it) is in
            // use: the player's Documents saves have no Steam or Wine shape, so only the list can
            // refuse them; a scratch folder is allowed. Nothing is written.
            let home = game_folders::Home::from_process().expect("HOME is a full path");
            let documents = home.home.join("Documents/My Games/FalloutNV/Saves/x.fos");
            assert!(refused_here(&documents).is_some(), "{documents:?}");
            assert_eq!(refused_here(&s.0.join("own/Saves/y.fos")), None);
        }

        /// Review of #630 (round 4): the game folder the port was started with is guarded, though
        /// the look at the player's folders did not find it (a copy given by hand): its `Data` and
        /// the folder holding it; a folder beside it is not.
        #[test]
        fn the_game_folder_started_with_is_guarded_unfound() {
            let s = Scratch::new("started");
            let data = s.0.join("game/Data");
            std::fs::create_dir_all(&data).unwrap();
            let in_game = s.0.join("game/Saves/x.fos");
            let in_data = data.join("x.fos");
            let beside = s.0.join("own/Saves/x.fos");
            assert_eq!(refused_here(&in_game), None);
            assert_eq!(refused_here(&in_data), None);
            prepare(&data);
            assert!(refused_here(&in_game).is_some());
            assert!(refused_here(&in_data).is_some());
            assert_eq!(refused_here(&beside), None);
        }

        /// Review of #630 (round 3): a look with no answer refuses every path; an answer is
        /// used; the first write waits a few seconds for it, not none and not forever.
        #[test]
        fn a_look_without_an_answer_refuses_every_path() {
            let s = Scratch::new("no-answer");
            let plain = s.0.join("own/Saves/x.fos");
            assert_eq!(refused_by(Err(LOOK_TOO_SLOW), &plain), Some(LOOK_TOO_SLOW));
            assert_eq!(refused_by(Err(LOOK_STOPPED), &plain), Some(LOOK_STOPPED));
            assert_eq!(refused_by(Ok(&[]), &plain), None);
            let never = [Guarded {
                root: s.0.join("own"),
                except: None,
                system: game_folders::System::this(),
            }];
            assert!(refused_by(Ok(&never), &plain).is_some());
            assert!(
                LOOK_WAIT >= std::time::Duration::from_secs(5),
                "{LOOK_WAIT:?}"
            );
            assert!(
                LOOK_WAIT <= std::time::Duration::from_secs(30),
                "{LOOK_WAIT:?}"
            );
        }

        /// Review of #630 (round 3): on Linux a save the game deletes is deleted from nv-rs's
        /// folder under the name its copy was written with (the game's key is lower case, the
        /// copy keeps the game's case, Linux compares names exactly), and not a file that differs
        /// from it only in case.
        #[cfg(target_os = "linux")]
        #[test]
        fn a_deleted_save_goes_under_the_name_it_was_written_with() {
            let s = Scratch::new("case-delete");
            let dir = s.0.join("Saves");
            let mut e = Engine::new();
            let key = br"c:\scratch\my games\falloutnv\saves\save 3 x.fos".to_vec();
            super::put(&mut e, &key, b"FO3SAVEGAME");
            engine_platform::state_mut(&mut e.kernel.platform)
                .disk
                .entries
                .insert(
                    key.clone(),
                    engine_platform::files::Entry {
                        name: "Save 3 X.fos".into(),
                        short: None,
                        attributes: 0x20,
                        size: 11,
                        created: 0,
                        accessed: 0,
                        written: 0,
                    },
                );
            let mut watch = Watch::default();
            assert_eq!(
                super::look(&e, &mut watch, Some(&dir), None),
                ["NV_SAVE_OUT: copied Save 3 X.fos".to_string()]
            );
            // Another file differing only in case (not the one written): left alone.
            std::fs::write(dir.join("SAVE 3 X.FOS"), b"another").unwrap();
            {
                let disk = &mut engine_platform::state_mut(&mut e.kernel.platform).disk;
                disk.files.remove(&key);
                disk.entries.remove(&key);
                disk.deleted.insert(key.clone());
                disk.gone(&key);
            }
            assert_eq!(
                super::look(&e, &mut watch, Some(&dir), None),
                ["NV_SAVE_OUT: copied Save 3 X.fos (deleted)".to_string()]
            );
            assert!(std::fs::symlink_metadata(dir.join("Save 3 X.fos")).is_err());
            assert_eq!(std::fs::read(dir.join("SAVE 3 X.FOS")).unwrap(), b"another");
        }

        /// Review of #630 (round 3): a copy an earlier run wrote (its name in the game's case) is
        /// found by the delete without case, when it is the only such plain file.
        #[cfg(target_os = "linux")]
        #[test]
        fn a_copy_from_an_earlier_run_is_deleted_whatever_its_case() {
            let s = Scratch::new("case-earlier");
            let dir = s.0.join("Saves");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("Save 4 Y.fos"), b"x").unwrap();
            let never: Vec<Guarded> = vec![];
            let check = |p: &Path| game_folders::refused(p, &never);
            assert_eq!(
                run_checked(
                    &Job::Remove {
                        target: dir.join("save 4 y.fos"),
                        name: "save 4 y.fos".into(),
                    },
                    &check
                ),
                Ok(Some(
                    "NV_SAVE_OUT: copied Save 4 Y.fos (deleted)".to_string()
                ))
            );
            assert!(std::fs::symlink_metadata(dir.join("Save 4 Y.fos")).is_err());
        }
    }
}
