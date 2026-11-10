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
        let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
        if let Some(why) = refused(&dir, profile.as_deref(), &known_folders()) {
            warn_visibly(
                &format!(
                    "NV_SAVE_OUT={}: refused ({why}); saves are not copied.",
                    dir.display()
                ),
                "Your saves are NOT being kept: they will be lost when the game closes. The save folder the game was given is one it must not write to. Start the game through Play.exe, and report this if you did.",
            );
            return None;
        }
        Some(dir)
    })
    .as_deref()
}

/// A warning the player can't miss: a line in the log, and on Windows a message box (from its own
/// thread, so the game goes on), unless `NV_RS_NO_WINDOWS` is set (scripted runs).
fn warn_visibly(line: &str, plain: &str) {
    println!("{line}");
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
    #[cfg(not(windows))]
    let _ = plain;
}

/// The player's Documents, AppData and Local AppData as Windows answers for them (wherever they
/// were moved to: OneDrive, another drive); empty off Windows.
pub fn known_folders() -> Vec<PathBuf> {
    #[cfg(windows)]
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
    #[cfg(not(windows))]
    Vec::new()
}

/// The path the system really means: links, junctions, `subst` and short names resolved, a part
/// that does not exist yet kept after the real path of the longest part that does.
fn real_path(path: &Path) -> PathBuf {
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
        let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
        if let Some(why) = refused(&file, profile.as_deref(), &known_folders()) {
            warn_visibly(
                &format!(
                    "NV_PREFS_OUT={}: refused ({why}); settings are not copied.",
                    file.display()
                ),
                "Your changed game settings are NOT being kept: they will be lost when the game closes. The settings file the game was given is in a folder it must not write to. Start the game through Play.exe, and report this if you did.",
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
    match job {
        Job::Write {
            dir,
            target,
            temporary,
            bytes,
            copied,
            ..
        } => {
            let written = dir
                .as_deref()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(temporary, bytes))
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
            let removed = target.is_file() && std::fs::remove_file(target).is_ok();
            Ok(removed.then(|| format!("NV_SAVE_OUT: copied {name} (deleted)")))
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

/// The copy-out's thread, started with the first job: it does the jobs in the order sent, so the
/// disk work (writing a save of several megabytes, renaming it, removing one) never holds up a
/// frame.
fn writer() -> &'static std::sync::mpsc::Sender<Message> {
    static SENDER: OnceLock<std::sync::mpsc::Sender<Message>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (send, receive) = std::sync::mpsc::channel::<Message>();
        std::thread::Builder::new()
            .name("nv save out".into())
            .spawn(move || {
                for message in receive {
                    match message {
                        Message::Job(job) => match run(&job) {
                            Ok(Some(line)) => println!("{line}"),
                            Ok(None) => {}
                            Err(line) => {
                                println!("{line}");
                                if let Job::Write { key, .. } = job {
                                    FAILED.lock().unwrap_or_else(|p| p.into_inner()).push(key);
                                }
                            }
                        },
                        Message::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })
            .expect("the copy-out's thread");
        send
    })
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
        let _ = send.send(Message::Job(job));
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
        let bytes = |jobs: &[Job]| match &jobs[..] {
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
}
