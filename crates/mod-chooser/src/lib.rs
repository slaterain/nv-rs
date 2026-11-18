//! The mod installer's text front end: the same on Windows, Linux and macOS, in plain words a
//! player who has never used a terminal can follow (the maintainer's and lead's rules, 2026-11-18):
//! every question says what to type, Enter takes the marked (recommended) choice, and a typo is
//! answered with what was wrong and the same question again: nothing chosen so far is lost. The
//! text is wrapped to fit an 80-column window.
//!
//! It lists the mods installed with nv-rs and the archives waiting in the mods folder. Picking an
//! archive installs it: a FOMOD installer's steps are asked one group at a time (`crates/fomod`),
//! the total to unpack and the room on the drive are shown before the player says yes, the chosen
//! files are unpacked (`crates/mod-archive`) into a staging folder with a progress line about once
//! a second, the record is written, the folder takes its place in `mod installs`, and the archive
//! moves to `installed archives` in the mods folder. An archive with no installer installs its
//! Data layout as it is; a `script.cs` installer is a program nv-rs does not run, and its plain
//! layout is offered when it has one. Picking an installed mod shows its record; `remove` and its
//! number removes it.
//!
//! Every change on disk (clearing a stopped install's staging folder, the record, the staging
//! folder taking its place, keeping the archive, removing a mod) goes through `crates/mod-install`,
//! whose guard forbids the game's folder; the chooser does not start at all when the installs
//! folder or the mods folder is inside the game's folder (through a link too). The game's folder
//! is only read, to answer the installers' questions about which plugins and files the player has.
//!
//! [`run_in_window`] is what nv-rs's program runs for `--install-mods`: every way it ends, on
//! every system, finishes with "Press Enter to close." and waits for that Enter, so a window of
//! its own (Windows starts it in one) never closes before its last words are read.
//!
//! # Limits
//!
//! | Cap | Number | Why that number |
//! |---|---|---|
//! | `MAX_FACT_FILES` | 500,000 files in each folder read for the installers' questions | as `mod_install::overlay::MAX_FILES`, per layer |
//! | `MAX_FACT_DEPTH` | 64 folders deep | as `mod_install::overlay::MAX_DEPTH` |
//! | `MAX_LINE` | 4,096 bytes of a typed line kept | any answer is a few characters |
//! | `MAX_RECORD` | 16 MB of an install record read | a record lists a mod's files: tens of thousands of lines at most |
//! | `ROOM_SPARE` + `ROOM_PER_FILE` | 256 MB and 4 KB a file kept free beyond the unpacked size | the folders, the record, each file's last part-filled block, and room for the system |
//! | the unpacked total | the room on the drive less the spare room (at most `mod_archive::Limits::default`) | a full drive is said as one, before anything is unpacked |
//! | what a 7z decodes | `mod_archive::Limits::default`'s, never lowered for room | decoding writes nothing |
//!
//! One installer runs at a time: a second window says the installer is already open and stops
//! before it changes anything (a lock file in `mod installs`, held open for the whole run, which
//! the system lets go when the program ends however it ends).

use fomod::config::{parse_config, parse_info, FileState};
use fomod::install::{Facts, PlanError, Session};
use fomod::layout::{layout, Layout, Listing};
use mod_archive::{Entry, Limits, Problem};
use mod_install::installs::{self, Installed, Record};
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Where things are, and what the port provides.
#[derive(Debug, Clone)]
pub struct Places {
    /// The game's folder (the one holding Data): only read.
    pub game: PathBuf,
    /// The port's `mod installs` folder.
    pub installs: PathBuf,
    /// The loose `mods` folder the player drops archives into.
    pub loose: PathBuf,
    /// Script extenders the port provides, by name and version (`nvse`, `6.4.9`).
    pub extenders: Vec<(String, String)>,
    /// How the player plays with mods on this system, to end "the mods are used the next time
    /// you ...": `choose "Play with mods"` on Windows, `start nv-rs with ./play.sh --mods` on Linux
    /// and macOS.
    pub play_with_mods: String,
    /// The bytes free on the drive holding a folder ([`room_on_drive`]; the tests give their own).
    pub room: fn(&Path) -> Option<u64>,
}

/// The game's version, for the installers' game version conditions.
const GAME_VERSION: &str = "1.4.0.525";

/// The most files of one folder read for the installers' questions (the game's Data, each
/// installed mod, the loose mods folder: each on its own, as `mod_install::overlay::MAX_FILES`
/// is per layer): past it the install stops in words, never answering from part of the list.
const MAX_FACT_FILES: usize = 500_000;
/// The deepest folder read for them (as `mod_install::overlay::MAX_DEPTH`): past it, the same.
const MAX_FACT_DEPTH: usize = 64;

/// The most bytes of one typed line kept (the rest of a longer line is read and dropped).
const MAX_LINE: usize = 4096;
/// The most bytes of an install record read.
const MAX_RECORD: u64 = 16 << 20;
/// The width the text is wrapped to: an 80-column window with room to spare.
const WIDTH: usize = 78;

/// Room kept free on the drive beyond what a mod unpacks to: the record, the folders, and some
/// for the system. Plus [`ROOM_PER_FILE`] a file, for the part of the last block each file fills.
const ROOM_SPARE: u64 = 256 << 20;
const ROOM_PER_FILE: u64 = 4096;

/// How often a progress line is shown while unpacking.
const PROGRESS_EVERY: Duration = Duration::from_secs(1);

type Out<'a> = &'a mut dyn Write;

/// Whether a line is a path on its own (`C:\...`, `\\server\...`, `/...`, `~/...`): never wrapped,
/// so it can be copied whole, spaces and all.
fn path_line(line: &str) -> bool {
    let t = line.trim_start().as_bytes();
    t.starts_with(b"/")
        || t.starts_with(b"\\\\")
        || t.starts_with(b"~/")
        || (t.len() >= 3
            && t[0].is_ascii_alphabetic()
            && t[1] == b':'
            && matches!(t[2], b'\\' | b'/'))
}

/// `text` wrapped at [`WIDTH`] columns on spaces: each line keeps its own indent and its
/// continuation lines are indented four more (a hanging indent). A word longer than the width
/// stays whole, and a line that is a path ([`path_line`]) is never wrapped: every message puts a
/// path on a line of its own.
fn wrap(text: &str) -> String {
    let mut out = String::new();
    for line in text.split('\n') {
        if path_line(line) {
            out.push_str(line.trim_end());
            out.push('\n');
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        let hang = indent + 4;
        let mut current = " ".repeat(indent);
        let mut width = indent;
        let mut empty = true;
        for word in line.split(' ').filter(|w| !w.is_empty()) {
            let length = word.chars().count();
            if !empty && width + 1 + length > WIDTH {
                out.push_str(&current);
                out.push('\n');
                current = " ".repeat(hang);
                width = hang;
                empty = true;
            }
            if !empty {
                current.push(' ');
                width += 1;
            }
            current.push_str(word);
            width += length;
            empty = false;
        }
        out.push_str(current.trim_end());
        out.push('\n');
    }
    out
}

/// Writes `text` wrapped ([`wrap`]).
fn say(out: Out, text: &str) -> std::io::Result<()> {
    write!(out, "{}", wrap(text))
}

/// One typed line, trimmed, at most [`MAX_LINE`] bytes kept; bytes that are not UTF-8 become
/// U+FFFD (a typo like any other, answered and asked again). `None` when the input ended.
fn read_answer(input: &mut dyn BufRead) -> std::io::Result<Option<String>> {
    let mut kept = Vec::new();
    let mut any = false;
    loop {
        let buffer = match input.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if buffer.is_empty() {
            break;
        }
        any = true;
        let (part, ended) = match buffer.iter().position(|&b| b == b'\n') {
            Some(i) => (&buffer[..=i], true),
            None => (buffer, false),
        };
        let room = MAX_LINE.saturating_sub(kept.len());
        kept.extend_from_slice(&part[..part.len().min(room)]);
        let used = part.len();
        input.consume(used);
        if ended {
            break;
        }
    }
    if !any {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&kept).trim().to_owned()))
}

/// Asks `question` and reads the answer ([`read_answer`]); `None` when the input ended (taken as
/// stopping, everywhere).
fn ask(input: &mut dyn BufRead, out: Out, question: &str) -> std::io::Result<Option<String>> {
    say(out, question)?;
    write!(out, "> ")?;
    out.flush()?;
    let answer = read_answer(input)?;
    if answer.is_none() {
        writeln!(out)?;
    }
    Ok(answer)
}

/// Numbers typed with spaces or commas between them, each from 1 to `count`, as indices from 0;
/// `Err` with what to type when one is not.
fn numbers(text: &str, count: usize) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for word in text.split([' ', ',']).filter(|w| !w.is_empty()) {
        match word.parse::<usize>() {
            Ok(n) if (1..=count).contains(&n) => out.push(n - 1),
            _ => {
                return Err(format!(
                    "\"{word}\" is not one of the numbers shown. Type a number from 1 to {count}."
                ))
            }
        }
    }
    Ok(out)
}

/// A size as a player reads it: bytes, KB, MB or GB (1,024 each, as Windows shows them).
fn size_text(bytes: u64) -> String {
    const KB: u64 = 1 << 10;
    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    match bytes {
        b if b < KB => format!("{b} bytes"),
        b if b < MB => format!("{:.1} KB", b as f64 / KB as f64),
        b if b < GB => format!("{:.1} MB", b as f64 / MB as f64),
        b => format!("{:.1} GB", b as f64 / GB as f64),
    }
}

/// The bytes free for this user on the drive holding `path` (or its nearest folder that exists);
/// `None` when the system does not say.
pub fn room_on_drive(path: &Path) -> Option<u64> {
    let mut at = path;
    while std::fs::metadata(at).is_err() {
        at = at.parent().filter(|p| !p.as_os_str().is_empty())?;
    }
    free_bytes(at)
}

#[cfg(windows)]
fn free_bytes(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            directory: *const u16,
            free_to_caller: *mut u64,
            total: *mut u64,
            total_free: *mut u64,
        ) -> i32;
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    // SAFETY: `wide` is a NUL-ended UTF-16 path that lives through the call; `free` is a valid
    // u64 to write; the two totals are optional and passed as null.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free)
}

#[cfg(unix)]
#[allow(clippy::useless_conversion)] // the field types differ between Linux and macOS
fn free_bytes(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: statvfs is plain data, valid when zeroed; `name` is a NUL-ended path that lives
    // through the call, and `stats` a valid statvfs to fill.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(name.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    Some(u64::from(stats.f_bavail).saturating_mul(u64::from(stats.f_frsize)))
}

#[cfg(not(any(windows, unix)))]
fn free_bytes(_: &Path) -> Option<u64> {
    None
}

/// `path` as a full path (joined to the current folder when it is relative), to show the player.
fn full_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().map_or_else(|_| path.to_path_buf(), |here| here.join(path))
}

/// "Not enough room on the drive": the mod's unpacked size, the room nv-rs keeps free beyond it,
/// what is free, and how much more is needed.
fn no_room(bytes: u64, spare: u64, free: u64) -> String {
    let needed = bytes.saturating_add(spare);
    format!(
        "Not enough room on the drive: the mod unpacks to {}, and nv-rs keeps {} more free for the system and the folders. {} is free: {} more is needed.",
        size_text(bytes),
        size_text(spare),
        size_text(free),
        size_text(needed.saturating_sub(free))
    )
}

/// The room to keep beyond what `files` files unpack to.
fn spare_for(files: usize) -> u64 {
    ROOM_SPARE.saturating_add(ROOM_PER_FILE.saturating_mul(files as u64))
}

/// The limits to unpack `bytes` in `files` files with `free` bytes on the drive: the written
/// total lowered to the room on the drive less the spare room (what a 7z may decode is never
/// lowered: decoding writes nothing); `Err` ([`no_room`]) when it does not fit.
fn room_limits(bytes: u64, files: usize, free: Option<u64>) -> Result<Limits, String> {
    let default = Limits::default();
    let Some(free) = free else {
        return Ok(default);
    };
    let spare = spare_for(files);
    if bytes.saturating_add(spare) > free {
        return Err(no_room(bytes, spare, free));
    }
    Ok(Limits {
        total: default.total.min(free - spare),
        ..default
    })
}

/// The archives waiting in the loose folder (its top level, told by their first bytes; RAR too,
/// to say it can't be read), by name.
fn waiting(loose: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(loose) else {
        return vec![];
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| is_archive(p))
        .collect();
    out.sort();
    out
}

/// Whether the file at `path` is an archive, by its first bytes.
fn is_archive(path: &Path) -> bool {
    let mut head = [0u8; 8];
    let read = std::fs::File::open(path)
        .and_then(|f| {
            let mut got = 0;
            let mut f = f.take(head.len() as u64);
            loop {
                match f.read(&mut head[got..])? {
                    0 => return Ok(got),
                    n => got += n,
                }
                if got == head.len() {
                    return Ok(got);
                }
            }
        })
        .unwrap_or(0);
    !matches!(
        mod_archive::format_of(&head[..read]),
        mod_archive::Format::Unknown
    )
}

/// An installed mod's record (empty when it can't be read).
fn read_record(folder: &Path) -> Record {
    let mut text = String::new();
    if let Ok(file) = std::fs::File::open(folder.join(mod_install::RECORD)) {
        let _ = file.take(MAX_RECORD).read_to_string(&mut text);
    }
    Record::parse(&text)
}

/// "a folder could not be read (why):" and its full path on a line of its own.
fn unreadable(folder: &Path, e: &std::io::Error) -> String {
    format!(
        "a folder could not be read ({e}):\n{}",
        full_path(folder).display()
    )
}

/// What the player has, for the installers' questions: every file of the game's Data, of the
/// installed mods and of the loose mods folder (as "Play with mods" lays it over the game) counts
/// as there and switched on (the port switches on every plugin it plays with).
fn facts(places: &Places) -> Result<Facts, String> {
    facts_with(places, MAX_FACT_FILES, MAX_FACT_DEPTH)
}

/// [`facts`] with these caps (tests lower them), each folder counted on its own; `Err` in words
/// past one, or when a folder can't be read.
fn facts_with(places: &Places, max_files: usize, max_depth: usize) -> Result<Facts, String> {
    let mut plugins = Vec::new();
    // One root: its files, relative, `/` between the parts; a `layer` (an installed mod, the loose
    // mods folder) leaves out what "Play with mods" leaves out at its top (archives, READ ME,
    // plugins.txt, the install record, the kept archives).
    let mut walk = |root: &Path, layer: bool| -> Result<(), String> {
        let mut files = 0usize;
        let mut stack = vec![(root.to_path_buf(), String::new(), 0usize)];
        while let Some((dir, relative, depth)) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound && depth == 0 => continue,
                Err(e) => return Err(unreadable(&dir, &e)),
            };
            for entry in entries {
                let entry = entry.map_err(|e| unreadable(&dir, &e))?;
                files += 1;
                if files > max_files {
                    return Err(format!(
                        "a folder holds more than {max_files} files, more than the installer reads:\n{}",
                        full_path(root).display()
                    ));
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = if relative.is_empty() {
                    name.clone()
                } else {
                    format!("{relative}/{name}")
                };
                let kind = entry
                    .file_type()
                    .map_err(|e| unreadable(&entry.path(), &e))?;
                let top = relative.is_empty();
                if layer
                    && top
                    && (mod_install::overlay::left_out(&name, kind.is_dir())
                        || (kind.is_file() && is_archive(&entry.path())))
                {
                    continue;
                }
                if kind.is_dir() {
                    if depth >= max_depth {
                        return Err(format!(
                            "a folder is more than {max_depth} folders deep, deeper than the installer reads:\n{}",
                            full_path(&entry.path()).display()
                        ));
                    }
                    stack.push((entry.path(), path, depth + 1));
                } else if kind.is_file() {
                    plugins.push((path, FileState::Active));
                }
            }
        }
        Ok(())
    };
    if let Some(data) = mod_install::overlay::game_data(&places.game) {
        walk(&data, false)?;
    }
    let installed = installs::installed(&places.installs)
        .map_err(|e| format!("the installed mods could not be read: {e}"))?;
    for mod_folder in installed {
        walk(&mod_folder.folder, true)?;
    }
    walk(&places.loose, true)?;
    Ok(Facts {
        plugins,
        game_version: Some(GAME_VERSION.to_owned()),
        extenders: places.extenders.clone(),
    })
}

/// Why the chooser must not start: the installs or the mods folder inside the game's folder, or
/// the game's folder inside one of them (through a link too); `None` when they are apart.
fn inside_game(places: &Places) -> Option<String> {
    use mod_install::fsops::real;
    let game = match real(&places.game) {
        Ok(g) => g,
        Err(e) => {
            return Some(format!(
                "The game's folder could not be read ({e}):\n{}",
                full_path(&places.game).display()
            ))
        }
    };
    for (what, folder) in [
        ("The mod installs folder", &places.installs),
        ("The mods folder", &places.loose),
    ] {
        let place = match real(folder) {
            Ok(p) => p,
            Err(e) => {
                return Some(format!(
                    "{what} could not be read ({e}):\n{}",
                    full_path(folder).display()
                ))
            }
        };
        if place.starts_with(&game) || game.starts_with(&place) {
            return Some(format!(
                "{what} is inside the game's folder, or leads there through a link:\n{}\nnv-rs never changes the game's folder, so it can't install mods there. Keep the nv-rs folder (and its mods folder) outside the game's folder, then start this again.",
                full_path(folder).display()
            ));
        }
    }
    None
}

/// The whole chooser in a window of its own (nv-rs's program with `--install-mods`): [`run`], or
/// `places`' error, then on every way it ends, "Press Enter to close." and one line read (the
/// input ending closes at once). The exit code: 0 when it ran, 1 when it stopped on an error, 2
/// when it could not start (no folders, folders inside the game's, another installer open).
pub fn run_in_window(input: &mut dyn BufRead, out: Out, places: Result<Places, String>) -> i32 {
    let code = match places {
        Err(why) => {
            let _ = say(out, &why);
            2
        }
        Ok(places) => match run(input, out, &places) {
            Ok(code) => code,
            Err(e) => {
                let _ = say(
                    out,
                    &format!("\nThe mod installer stopped: {e}\nNothing more was changed."),
                );
                1
            }
        },
    };
    let _ = writeln!(out);
    let _ = write!(out, "Press Enter to close.");
    let _ = out.flush();
    let _ = read_answer(input);
    let _ = writeln!(out);
    code
}

/// What a line typed at the list asks for.
enum ListAnswer {
    Finish,
    Pick(usize),
    Remove(usize),
    /// Not understood: what to say.
    Again(String),
}

fn list_answer(answer: &str, total: usize) -> ListAnswer {
    let lower = answer.to_lowercase();
    let mut words = lower.split_whitespace();
    let Some(first) = words.next() else {
        return ListAnswer::Finish;
    };
    let one = |text: &str| match numbers(text, total) {
        Ok(p) if p.len() == 1 => Ok(p[0]),
        Ok(p) if p.is_empty() => Err(format!(
            "Type the number after \"{first}\", like {first} 1."
        )),
        Ok(_) => Err(format!("Type one number from 1 to {total}.")),
        Err(why) => Err(why),
    };
    match first {
        "q" | "quit" | "exit" | "done" | "finish" => ListAnswer::Finish,
        "remove" | "uninstall" | "delete" => {
            match one(&words.collect::<Vec<_>>().join(" ")) {
                Ok(n) => ListAnswer::Remove(n),
                Err(why) => ListAnswer::Again(why),
            }
        }
        w if w.starts_with(|c: char| c.is_ascii_digit()) => match one(&lower) {
            Ok(n) => ListAnswer::Pick(n),
            Err(why) => ListAnswer::Again(why),
        },
        w => ListAnswer::Again(format!(
            "\"{w}\" is not something this understands. Type a number, or remove and a number (like remove 1), or just press Enter to finish."
        )),
    }
}

/// The front end: the list, then what the player picks, until they press Enter on the list. One
/// installer at a time ([`installs::hold_installer`], held for the whole run, the same on every
/// system). The exit code as [`run_in_window`]'s: 0 when it ran, 2 when it could not start.
pub fn run(input: &mut dyn BufRead, out: Out, places: &Places) -> std::io::Result<i32> {
    writeln!(out, "nv-rs mods")?;
    writeln!(out, "==========")?;
    if let Some(why) = inside_game(places) {
        say(out, &format!("\n{why}\nNothing was changed."))?;
        return Ok(2);
    }
    let _held = match installs::hold_installer(&places.installs, &places.game) {
        Ok(Some(held)) => held,
        Ok(None) => {
            say(
                out,
                "\nThe mod installer is already open in another window. Finish there (or close that window), then start this again.\nNothing was changed.",
            )?;
            return Ok(2);
        }
        Err(e) => {
            say(
                out,
                &format!(
                    "\nThe mod installer could not start: its folder could not be used ({e}):\n{}\nNothing was changed.",
                    full_path(&places.installs).display()
                ),
            )?;
            return Ok(2);
        }
    };
    clear_leftovers(out, places)?;
    choose(input, out, places)?;
    Ok(0)
}

/// Clears what stopped installs left (said, with any that could not be cleared); the installer
/// carries on either way, since a leftover is never listed or played.
fn clear_leftovers(out: Out, places: &Places) -> std::io::Result<()> {
    match installs::clear_staging(&places.installs, &places.game) {
        Ok(done) if done.cleared == 0 && done.left.is_empty() => Ok(()),
        Ok(done) => {
            let stopped = "\nAn install that was stopped before it finished (the window closed, or the power went) had left its unpacked files behind.";
            if done.left.is_empty() {
                return say(
                    out,
                    &format!("{stopped} They were cleared: that mod was not installed, and you can install it again."),
                );
            }
            say(
                out,
                &format!(
                    "{stopped} {} of {} were cleared; these could not be (they are never used as mods, and the next start tries again):",
                    done.cleared,
                    done.cleared + done.left.len()
                ),
            )?;
            for (path, e) in &done.left {
                writeln!(out, "{}", full_path(path).display())?;
                say(out, &format!("    ({e})"))?;
            }
            Ok(())
        }
        Err(e) => say(
            out,
            &format!(
                "\nThe mod installs folder could not be checked for what stopped installs leave behind ({e}). Such leftovers are never used as mods; the installer carries on."
            ),
        ),
    }
}

/// The list and what the player picks, until they press Enter on the list.
fn choose(input: &mut dyn BufRead, out: Out, places: &Places) -> std::io::Result<()> {
    loop {
        let installed = match installs::installed(&places.installs) {
            Ok(i) => i,
            Err(e) => {
                return say(
                    out,
                    &format!("\nThe installed mods could not be read: {e}\nNothing was changed."),
                )
            }
        };
        let records: Vec<Record> = installed.iter().map(|m| read_record(&m.folder)).collect();
        let archives = waiting(&places.loose);
        writeln!(out)?;
        if installed.is_empty() && archives.is_empty() {
            let folder = full_path(&places.loose);
            say(
                out,
                "No mods yet. Put a mod's archive (.7z, .zip or .fomod, as you downloaded it) in this folder:",
            )?;
            writeln!(out, "{}", folder.display())?;
            return say(out, "Then start this again.");
        }
        if !installed.is_empty() {
            writeln!(out, "Installed with nv-rs:")?;
            for (i, m) in installed.iter().enumerate() {
                say(out, &format!("  {}) {}", i + 1, m.name))?;
            }
        }
        if !archives.is_empty() {
            writeln!(out, "Waiting in the mods folder, not installed yet:")?;
            for (i, a) in archives.iter().enumerate() {
                let mut line = format!(
                    "  {}) {}",
                    installed.len() + i + 1,
                    a.file_name().unwrap_or_default().to_string_lossy()
                );
                if let Some(m) = installed_as(a, &installed, &records) {
                    line.push_str(&format!(" (installed already, as {})", m.name));
                }
                say(out, &line)?;
            }
        }
        let total = installed.len() + archives.len();
        let Some(answer) = ask(
            input,
            out,
            "\nType a number and press Enter. A waiting archive's number installs it; an installed mod's number shows what it installed.\nTo remove an installed mod, type remove and its number (like remove 1).\nJust press Enter to finish.",
        )?
        else {
            return Ok(());
        };
        match list_answer(&answer, total) {
            ListAnswer::Finish => {
                return say(
                    out,
                    &format!(
                        "Done. The installed mods are used the next time you {}.",
                        places.play_with_mods
                    ),
                )
            }
            ListAnswer::Again(why) => say(out, &why)?,
            ListAnswer::Pick(n) if n < installed.len() => {
                show_record(out, &installed[n], &records[n])?
            }
            ListAnswer::Remove(n) if n < installed.len() => {
                remove_flow(input, out, places, &installed[n])?
            }
            ListAnswer::Remove(_) => say(
                out,
                "That one is not installed: only mods from the \"Installed with nv-rs\" list can be removed.",
            )?,
            ListAnswer::Pick(n) => {
                let archive = &archives[n - installed.len()];
                if let Some(m) = installed_as(archive, &installed, &records) {
                    let again = ask(
                        input,
                        out,
                        &format!(
                            "\nThis archive was installed already, as {}. Install a second copy of it? Type yes and press Enter to install it again, or just press Enter to leave it.",
                            m.name
                        ),
                    )?;
                    if !again.as_deref().is_some_and(is_yes) {
                        say(out, "Left as it is: nothing was installed.")?;
                        continue;
                    }
                }
                install_flow(input, out, places, archive)?
            }
        }
    }
}

/// Whether `answer` is a yes.
fn is_yes(answer: &str) -> bool {
    answer.eq_ignore_ascii_case("yes") || answer.eq_ignore_ascii_case("y")
}

/// The installed mod `archive` was installed as (its record names the same archive file name and
/// size), if any.
fn installed_as<'a>(
    archive: &Path,
    installed: &'a [Installed],
    records: &[Record],
) -> Option<&'a Installed> {
    let name = archive.file_name()?.to_string_lossy();
    let size = std::fs::metadata(archive).ok()?.len();
    installed
        .iter()
        .zip(records)
        .find(|(_, r)| r.archive == name && r.archive_size == size)
        .map(|(m, _)| m)
}

fn show_record(out: Out, chosen: &Installed, record: &Record) -> std::io::Result<()> {
    say(out, &format!("\n{} {}", chosen.name, record.version))?;
    if !record.author.is_empty() {
        say(out, &format!("  by {}", record.author))?;
    }
    if !record.archive.is_empty() {
        say(out, &format!("  from {}", record.archive))?;
    }
    for choice in &record.choices {
        say(out, &format!("  chosen: {choice}"))?;
    }
    say(out, &format!("  {} files", record.files.len()))
}

fn remove_flow(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    chosen: &Installed,
) -> std::io::Result<()> {
    let answer = ask(
        input,
        out,
        &format!(
            "\nRemove {}? Its files are deleted from nv-rs's own folder (your game folder is not touched).\nType yes and press Enter to remove it, or just press Enter to keep it.",
            chosen.name
        ),
    )?;
    if answer.as_deref().is_some_and(is_yes) {
        match installs::remove(&places.installs, &chosen.folder, &places.game) {
            Ok(()) => say(out, &format!("Removed {}.", chosen.name))?,
            Err(e) => say(out, &format!("{} could not be removed: {e}", chosen.name))?,
        }
    } else {
        say(out, &format!("Kept {}.", chosen.name))?;
    }
    Ok(())
}

/// What an install puts where: (path in the archive, path in the mod's folder).
type Wanted = Vec<(String, String)>;

/// Installs `archive`: the right flow for what it holds.
fn install_flow(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    archive: &Path,
) -> std::io::Result<()> {
    let file_name = archive
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    say(out, &format!("\n{file_name}"))?;
    let entries = match mod_archive::list(archive) {
        Ok(e) => e,
        Err(e) => return say(out, &e.player_text()),
    };
    let listing: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
    let found = match layout(&listing) {
        Ok(l) => l,
        Err(e) => return say(out, &format!("This mod can't be installed: {e}.")),
    };
    let stem = Path::new(&file_name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let read_info = |info: &Option<String>| {
        info.as_deref()
            .and_then(|path| mod_archive::read(archive, path).ok().flatten())
            .map(|bytes| parse_info(&bytes))
            .unwrap_or_default()
    };
    match found {
        Layout::Installer {
            prefix,
            config,
            info,
        } => {
            let info = read_info(&info);
            let bytes = match mod_archive::read(archive, &config) {
                Ok(Some(b)) => b,
                Ok(None) => return say(out, "The installer file could not be found in it."),
                Err(e) => return say(out, &e.player_text()),
            };
            let config = match parse_config(&bytes) {
                Ok(c) => c,
                Err(e) => return say(out, &format!("This mod's installer can't be read: {e}.")),
            };
            let name = first_of(&[&info.name, &config.name, &stem]);
            let facts = match facts(places) {
                Ok(f) => f,
                // No full stop: the reason ends with a path on a line of its own.
                Err(why) => return say(out, &format!("This mod can't be installed: {why}")),
            };
            let mut session = match Session::new(&config, &facts) {
                Ok(s) => s,
                Err(why) => return say(out, &format!("This mod can't be installed: {why}.")),
            };
            say(out, &format!("{name} {}", info.version))?;
            if !info.author.is_empty() {
                say(out, &format!("by {}", info.author))?;
            }
            let Some((wanted, choices, limits)) = ask_steps(
                input,
                out,
                places,
                &mut session,
                &entries,
                &listing,
                &prefix,
            )?
            else {
                return Ok(());
            };
            let record = Record {
                name: name.clone(),
                version: info.version.clone(),
                author: info.author.clone(),
                choices,
                ..Record::default()
            };
            install(out, places, archive, &name, wanted, record, limits)
        }
        Layout::Script { data, .. } => {
            say(
                out,
                "This mod's installer is a small program (script.cs). nv-rs does not run programs that come with a download.",
            )?;
            match data {
                Some(data) => plain_flow(input, out, places, archive, &entries, &data, &stem),
                None => say(
                    out,
                    "It has no files laid out as in the game's Data folder either. Install it with a mod manager and copy the result into the mods folder instead.",
                ),
            }
        }
        Layout::Plain { data, info } => {
            let info = read_info(&info);
            let name = first_of(&[&info.name, &stem]);
            plain_flow(input, out, places, archive, &entries, &data, &name)
        }
        Layout::Unknown => say(
            out,
            "nv-rs can't tell how to install this: it has no installer and no files laid out as in the game's Data folder (.esp, .esm, .bsa, Meshes, Textures...).",
        ),
    }
}

fn first_of(names: &[&str]) -> String {
    names
        .iter()
        .find(|n| !n.trim().is_empty())
        .map_or_else(|| "Mod".to_owned(), |n| n.trim().to_owned())
}

/// What the player said to the summary before installing.
enum Confirm {
    Yes(Limits),
    Back,
    Stop,
    /// Refused (no room on the drive): already said.
    Refused,
}

/// The summary before installing (the files, their unpacked size, the room on the drive) and the
/// question: yes installs; with `can_go_back`, Enter alone (or anything else) goes back to the
/// choices and q stops, else anything but yes leaves it.
fn confirm(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    entries: &[Entry],
    wanted: &Wanted,
    can_go_back: bool,
) -> std::io::Result<Confirm> {
    let (files, bytes) = mod_archive::wanted_size(entries, wanted);
    let free = (places.room)(&places.installs);
    let room = match free {
        Some(f) => format!("Free on the drive: {}.", size_text(f)),
        None => "How much room is free on the drive could not be read.".to_owned(),
    };
    say(
        out,
        &format!(
            "\n{files} files will be installed: {} unpacked. {room}",
            size_text(bytes)
        ),
    )?;
    let limits = match room_limits(bytes, files, free) {
        Ok(l) => l,
        Err(why) => {
            say(
                out,
                &format!(
                    "{why}\nNothing was installed. Free some space on the drive, then try again."
                ),
            )?;
            return Ok(Confirm::Refused);
        }
    };
    let question = if can_go_back {
        "Type yes and press Enter to install it.\nPress Enter alone to go back to the choices, or type q to stop."
    } else {
        "Type yes and press Enter to install it, or just press Enter to leave it."
    };
    Ok(match ask(input, out, question)? {
        None => Confirm::Stop,
        Some(a) if is_yes(&a) => Confirm::Yes(limits),
        Some(a) if !can_go_back || a.eq_ignore_ascii_case("q") => Confirm::Stop,
        Some(a) if a.is_empty() => Confirm::Back,
        Some(a) => {
            say(
                out,
                &format!("\"{a}\" is not yes or q: back to the choices."),
            )?;
            Confirm::Back
        }
    })
}

/// An archive without an installer: its files from `data` down, as they are, after a yes.
fn plain_flow(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    archive: &Path,
    entries: &[Entry],
    data: &str,
    name: &str,
) -> std::io::Result<()> {
    let listing: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
    let index = match Listing::new(&listing) {
        Ok(l) => l,
        Err(e) => return say(out, &format!("This mod can't be installed: {e}.")),
    };
    let wanted: Wanted = index
        .under(data)
        .into_iter()
        .map(|(rest, given)| (given.to_owned(), rest))
        .collect();
    say(
        out,
        &format!("{name} has no installer: its files are installed as they are laid out."),
    )?;
    match confirm(input, out, places, entries, &wanted, false)? {
        Confirm::Yes(limits) => {
            let record = Record {
                name: name.to_owned(),
                ..Record::default()
            };
            install(out, places, archive, name, wanted, record, limits)
        }
        Confirm::Refused => Ok(()),
        Confirm::Back | Confirm::Stop => say(out, "Left as it is: nothing was installed."),
    }
}

/// The steps, one group at a time, until the player says yes to the summary (`Some`: what to
/// unpack, the choices made and the limits) or stops (`None`, already said).
fn ask_steps(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    session: &mut Session,
    entries: &[Entry],
    listing: &[String],
    prefix: &str,
) -> std::io::Result<Option<(Wanted, Vec<String>, Limits)>> {
    let stopped = |out: Out| say(out, "Stopped: nothing was installed.");
    let mut history: Vec<usize> = Vec::new();
    let mut step = session.next_step(None);
    let mut from_last = false;
    if step.is_none() {
        say(out, "This installer has no choices to make.")?;
    }
    loop {
        while let Some(index) = step {
            match ask_step(input, out, session, index, history.len() + 1, from_last)? {
                StepAnswer::Next => {
                    history.push(index);
                    step = session.next_step(Some(index));
                    from_last = false;
                }
                StepAnswer::Back => match history.pop() {
                    Some(previous) => {
                        step = Some(previous);
                        from_last = true;
                    }
                    None => {
                        say(
                            out,
                            "This is the first question: there is nothing before it.",
                        )?;
                        from_last = false;
                    }
                },
                StepAnswer::Stop => {
                    stopped(out)?;
                    return Ok(None);
                }
            }
        }
        let plan = match session.plan(listing, prefix) {
            Ok(plan) => plan,
            Err(PlanError::Choices(problems)) => {
                for (_, why) in &problems {
                    say(out, &format!("Not yet: {why}."))?;
                }
                step = problems.first().map(|(s, _)| *s);
                if let Some(s) = step {
                    history.retain(|&h| h < s);
                }
                from_last = false;
                continue;
            }
            Err(e) => {
                say(out, &format!("This mod can't be installed: {e}."))?;
                return Ok(None);
            }
        };
        if !plan.missing.is_empty() {
            say(
                out,
                &format!(
                    "\n{} files the installer names are not in the archive; the mod may still work.",
                    plan.missing.len()
                ),
            )?;
        }
        if !plan.refused.is_empty() {
            say(
                out,
                &format!(
                    "{} files would go outside the game's Data folder and are left out.",
                    plan.refused.len()
                ),
            )?;
        }
        let wanted: Wanted = plan
            .copies
            .into_iter()
            .map(|c| (c.source, c.destination))
            .collect();
        match confirm(input, out, places, entries, &wanted, true)? {
            Confirm::Yes(limits) => return Ok(Some((wanted, choices(session), limits))),
            Confirm::Refused => return Ok(None),
            Confirm::Stop => {
                stopped(out)?;
                return Ok(None);
            }
            Confirm::Back => {
                step = history.pop();
                from_last = true;
                if step.is_none() {
                    say(out, "This installer has no choices to go back to.")?;
                }
            }
        }
    }
}

enum StepAnswer {
    Next,
    Back,
    Stop,
}

/// The rule of a group, in words: what to type.
fn rule_text(kind: fomod::config::GroupType) -> &'static str {
    use fomod::config::GroupType as G;
    match kind {
        G::SelectExactlyOne => "Choose one: type its number and press Enter.",
        G::SelectAtMostOne => {
            "Choose one, or none: type its number, or 0 for none, and press Enter."
        }
        G::SelectAtLeastOne => {
            "Choose at least one: type their numbers with spaces between them (like 1 3) and press Enter."
        }
        G::SelectAny => {
            "Choose any: type their numbers with spaces between them (like 1 3), or 0 for none, and press Enter."
        }
        G::SelectAll => "All of these are installed: just press Enter.",
    }
}

/// One step, group by group, from its first group (or its last, coming back to it): b goes back
/// one group, and from the first group to the step before.
fn ask_step(
    input: &mut dyn BufRead,
    out: Out,
    session: &mut Session,
    index: usize,
    number: usize,
    from_last: bool,
) -> std::io::Result<StepAnswer> {
    use fomod::config::GroupType as G;
    let view = session.view(index);
    let title = if view.name.is_empty() {
        "Choices".to_owned()
    } else {
        view.name.clone()
    };
    say(out, &format!("\nStep {number}: {title}"))?;
    let mut group = if from_last {
        view.groups.len().saturating_sub(1)
    } else {
        0
    };
    while group < view.groups.len() {
        let current = session.view(index).groups[group].clone();
        say(out, &format!("\n  {}", current.name))?;
        for (i, option) in current.options.iter().enumerate() {
            let mark = if option.selected { "[x]" } else { "[ ]" };
            let tag = match option.kind {
                fomod::config::PluginType::Required => "  (needed)",
                fomod::config::PluginType::Recommended => "  (recommended)",
                fomod::config::PluginType::NotUsable => "  (can't be chosen here)",
                _ => "",
            };
            say(out, &format!("    {}) {mark} {}{tag}", i + 1, option.name))?;
        }
        let question = format!(
            "{}\nEnter alone keeps the ones marked [x].\n?1 = read about option 1, b = back, q = stop.",
            rule_text(current.kind)
        );
        let Some(answer) = ask(input, out, &question)? else {
            return Ok(StepAnswer::Stop);
        };
        let lower = answer.to_lowercase();
        if lower == "q" || lower == "quit" || lower == "stop" {
            return Ok(StepAnswer::Stop);
        }
        if lower == "b" || lower == "back" {
            if group == 0 {
                return Ok(StepAnswer::Back);
            }
            group -= 1;
            continue;
        }
        if let Some(n) = answer.strip_prefix('?') {
            match numbers(n.trim(), current.options.len()) {
                Ok(p) if p.len() == 1 => {
                    let option = &current.options[p[0]];
                    let text = if option.description.trim().is_empty() {
                        "(no description)"
                    } else {
                        option.description.trim()
                    };
                    say(out, &format!("\n  {}:", option.name))?;
                    for paragraph in text.lines() {
                        say(out, &format!("    {}", paragraph.trim()))?;
                    }
                }
                _ => say(
                    out,
                    &format!(
                        "Type ? and one number from 1 to {}, like ?1.",
                        current.options.len()
                    ),
                )?,
            }
            continue;
        }
        let chosen = if answer.is_empty() {
            Ok(session.chosen(index, group))
        } else if answer == "0" {
            Ok(vec![])
        } else {
            numbers(&answer, current.options.len())
        };
        match chosen {
            Err(why) => say(out, &why)?,
            Ok(chosen)
                if chosen.len() > 1
                    && matches!(current.kind, G::SelectExactlyOne | G::SelectAtMostOne) =>
            {
                say(out, "Only one can be chosen here: type one number.")?
            }
            Ok(chosen) => match session.choose(index, group, &chosen) {
                Ok(()) => group += 1,
                Err(why) => say(out, &format!("Not that: {why}."))?,
            },
        }
    }
    Ok(StepAnswer::Next)
}

/// The choices made, `Step / Group / Option` each, for the record.
fn choices(session: &Session) -> Vec<String> {
    let mut out = Vec::new();
    let mut step = session.next_step(None);
    while let Some(i) = step {
        let view = session.view(i);
        for group in &view.groups {
            for option in group.options.iter().filter(|o| o.selected) {
                out.push(format!("{} / {} / {}", view.name, group.name, option.name));
            }
        }
        step = session.next_step(Some(i));
    }
    out
}

/// Unpacks into a staging folder beside the installed ones (cleared on a failure), with a progress
/// line about once a second; writes the record and gives the folder its place; keeps the archive
/// in `installed archives`. Every change through `mod-install`'s guarded functions.
fn install(
    out: Out,
    places: &Places,
    archive: &Path,
    name: &str,
    wanted: Wanted,
    mut record: Record,
    limits: Limits,
) -> std::io::Result<()> {
    let folder = match installs::next_folder(&places.installs, name) {
        Ok(f) => f,
        Err(e) => {
            return say(
                out,
                &format!("Nothing was installed: the mod installs folder could not be read: {e}"),
            )
        }
    };
    let staging = match installs::make_staging(&places.installs, &folder, &places.game) {
        Ok(s) => s,
        Err(e) => {
            return say(
                out,
                &format!("Nothing was installed: a folder for it could not be made: {e}"),
            )
        }
    };
    let discard = |out: Out| -> std::io::Result<()> {
        if let Err(e) = installs::discard_staging(&places.installs, &staging, &places.game) {
            say(
                out,
                &format!(
                    "(Its unpacked files could not all be cleared: {e}. They are cleared the next time this starts.)"
                ),
            )?;
        }
        Ok(())
    };
    let file_name = archive
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    say(
        out,
        &format!(
            "\nInstalling {name}. A large mod can take several minutes: please don't close this window until it says it is done."
        ),
    )?;
    out.flush()?;
    let mut last: Option<Instant> = None;
    let mut all_written = false;
    // When making sure of the files on the disk began, and whether that was said.
    let mut saving: Option<(Instant, bool)> = None;
    let mut report = |p: mod_archive::Progress| {
        #[cfg(test)]
        tests::on_progress(&staging, p);
        let due = last.map_or(true, |t| t.elapsed() >= PROGRESS_EVERY);
        if p.saving > 0 {
            // Said only when it lasts over a second, then about once a second.
            let (since, said) = saving.get_or_insert((Instant::now(), false));
            if since.elapsed() >= PROGRESS_EVERY && (!*said || due) {
                if !*said {
                    let _ = writeln!(out, "Making sure the files are saved on the disk...");
                    *said = true;
                }
                last = Some(Instant::now());
                let _ = writeln!(out, "  {} of {} files saved", p.saved, p.saving);
                let _ = out.flush();
            }
            return;
        }
        let last_file = p.files == p.files_total && !all_written;
        if due || last_file {
            all_written |= p.files == p.files_total;
            last = Some(Instant::now());
            // A solid 7z decodes the options not chosen on its way to the chosen ones.
            let past = if p.read > 0 {
                format!(" (and {} of other files read through)", size_text(p.read))
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "  {} of {} files, {} of {}{past}",
                p.files,
                p.files_total,
                size_text(p.bytes),
                size_text(p.bytes_total)
            );
            let _ = out.flush();
        }
    };
    let extracted = mod_archive::extract_with(archive, &wanted, &staging, limits, &mut report);
    let done = match extracted {
        Ok(done) => done,
        Err(e) => {
            discard(out)?;
            // "No room" only for the drive really filling up while writing: the written total
            // was checked against the room before the yes, and decoding writes nothing.
            if e.problem == Problem::NoRoom {
                let (files, bytes) = {
                    let listed = mod_archive::list(archive).unwrap_or_default();
                    mod_archive::wanted_size(&listed, &wanted)
                };
                let free = (places.room)(&places.installs).unwrap_or(0);
                return say(
                    out,
                    &format!(
                        "Nothing was installed.\n{}\nFree some space on the drive, then try again.",
                        no_room(bytes, spare_for(files), free)
                    ),
                );
            }
            return say(out, &format!("Nothing was installed.\n{}", e.player_text()));
        }
    };
    // Every file it wrote still there (another program could have removed one): the record lists
    // only what the folder really holds.
    if let Some(gone) = done
        .written
        .iter()
        .find(|p| !std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file()))
    {
        discard(out)?;
        return say(
            out,
            &format!(
                "Nothing was installed: a file it had unpacked is gone (did another program remove it?):\n{}",
                full_path(gone).display()
            ),
        );
    }
    record.archive = file_name;
    record.archive_size = std::fs::metadata(archive).map_or(0, |m| m.len());
    record.files = wanted.iter().map(|(_, d)| d.replace('\\', "/")).collect();
    if let Err(e) = installs::finish(&places.installs, &staging, &folder, &record, &places.game) {
        discard(out)?;
        return say(out, &format!("Nothing was installed: {e}"));
    }
    say(
        out,
        &format!(
            "Installed {name} ({} files). It is used the next time you {}.",
            done.written.len(),
            places.play_with_mods
        ),
    )?;
    match installs::keep_archive(&places.loose, archive, &places.game) {
        Ok(kept) => {
            say(out, "The archive was moved to:")?;
            writeln!(out, "{}", kept.display())?;
        }
        Err(e) => say(
            out,
            &format!(
                "(The archive could not be moved to \"{}\" in the mods folder: {e}. You can move or delete it yourself.)",
                installs::KEPT
            ),
        )?,
    }
    Ok(())
}

#[cfg(test)]
mod tests;
