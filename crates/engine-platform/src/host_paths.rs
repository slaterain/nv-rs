//! The game's Windows paths over this system's folders on Linux and macOS (decision 3 of the
//! Linux and macOS track, the Wine way).
//!
//! The game sees this system's folders on one drive, `Z:` (`engine_core::exe::game_view`): it
//! is started in `Z:\home\me\Games\Fallout New Vegas`, and every path it builds from there is a
//! Windows path, `\` between the parts, in whatever case the game, its INI files or a mod wrote
//! the names. Windows finds a file whatever the case of its name; Linux's file systems (and macOS's
//! when formatted case-sensitive) do not. So a path the file calls are given becomes this system's
//! path here ([`host_path`]):
//! - `Z:` is taken off, and each `\` becomes `/`. Another drive letter is a drive this system
//!   does not have: everything on it is missing, as on a PC without that drive (the game's own
//!   paths start on `Z:`; a test or an INI value may name `C:`). Such a path becomes one under
//!   [`ABSENT_DRIVES`], which is never there, and the log says so once per drive.
//! - The parts are then read as Windows reads a path ([`win32_parts`], measured with
//!   `GetFullPathNameW` and `CreateFileW` on scratch files, Windows 11 26200, 2026-11-16):
//!   `.` and `..` by the text of the path (a `..` goes up from the part before it, links or
//!   not); one `.` at the end of a folder's name dropped (`sub.\y.txt` is `sub\y.txt`, `sub..\`
//!   stays); the dots and spaces at the end of the last name dropped (`x.dds. ` is `x.dds`); a
//!   separator at the end kept, so a file named with one is not opened (Windows answers
//!   `ERROR_DIRECTORY`). A last name `NUL` (Windows' null device) is not provided: the run
//!   stops. A `:` in a name (a file's stream) answers as missing.
//! - A name of this system's that Windows cannot hold (a `:` or `\` in it, a dot or space at its
//!   end, `NUL`: [`windows_cannot_hold`]) is not shown to the game by the listing, and the game
//!   cannot open it (the rules above read its name as another one, or as a stream).
//! - Each part is then looked up in its folder as Windows compares names: without case, by
//!   NTFS's upper-casing (`super::files`' `ntfs_key`). A part whose name is there as written is
//!   tried first; then the folder's other names that match it without case, in byte order. The
//!   first that holds the rest of the path is taken ([`resolve`]): a folder that is two folders
//!   here (`Data/Textures` and `Data/textures`, as two mods unpacked by hand leave them; one
//!   folder to Windows) gives its files from either. When a folder holds more than one name for
//!   the part, the log says which was taken, once per folder and name. On a file system that
//!   ignores case (macOS's by default) a name there in another case is that folder's entry, and
//!   no other is tried.
//! - When no such path holds every part, the one that holds the most is taken and the rest is
//!   kept as written: the file is not there, and the call answers as for any missing file.
//!
//! A folder's listing ([`host_folders`]) is every folder the path names in this way, as one.
//!
//! A path with no drive (one the host gave: a test's scratch folder, `/tmp/...`) is treated the
//! same from its `/` on; a relative one is left relative, `\` as `/`.
//!
//! The folders' names are kept between calls ([`Listings`]) while the folder's modification
//! time stays the one read with them (a name added, removed or renamed changes it on these
//! systems); a listing read within [`SETTLED`] after its folder's last change is not kept, so a
//! change in the same tick of the file system's clock is never missed. That is what keeps them
//! current, after anyone's writes; [`forget`] (the save copy-out's, once it runs off Windows) is
//! a fallback for a file system whose clock does not. They are kept by folder, not by spelling
//! ([`folder_key`]), at most [`MAX_FOLDERS`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// What stops a path from being one of this system's.
#[derive(Debug, PartialEq, Eq)]
pub enum NotHere {
    /// A drive other than `Z:`: no folder of this system is on it.
    OtherDrive(char),
    /// A drive with a name but no `\` after it (`Z:x`, relative to that drive's current folder):
    /// stops the run. The game's file calls do not give one: they make every name full first
    /// (`super::files::full_path` puts `Z:x` in the current folder, where its `:` makes it a
    /// stream, [`NotHere::Stream`]). A bare `Z:` is the drive's root ([`unix_text`]).
    DriveRelative,
    /// A network or device path (`\\server\share`, `\\.\`, `\\?\` before anything but a drive's
    /// path): not provided off Windows, and never laid over this system's root.
    Unc,
    /// The last name `NUL` (any case, dots and spaces after it): Windows' null device in every
    /// folder (measured: `D:\x\NUL` is `\\.\NUL`; `CON`, `AUX`, `COM1` and `nul.txt` are plain
    /// names on Windows 11). Not provided.
    Device,
    /// A `:` in a name: a file's alternate stream to Windows (`x.dds:s`, `x.dds::$DATA`). No
    /// stream is provided: the path answers as a missing file ([`host_path`]), never as the file
    /// of this system that holds a `:` in its name.
    Stream,
}

/// The game's path `game` (its text, `\` or `/` between parts) as this system's path, before the
/// names are read as Windows reads them: `Z:` taken off, `\` as `/`. A bare `Z:` is the root,
/// `/`: the folder the file calls cut a name at the root to (`Z:\nv.ini`'s folder when they
/// look whether it is there, `Z:\*`'s when they list it). `\\?\Z:\x` is `Z:\x` (the long-path
/// form of the same file to Windows).
pub fn unix_text(game: &str) -> Result<String, NotHere> {
    if let Some(rest) = game.strip_prefix(r"\\?\") {
        let r = rest.as_bytes();
        if r.len() >= 3 && r[0].is_ascii_alphabetic() && r[1] == b':' && r[2] == b'\\' {
            return unix_text(rest);
        }
    }
    let b = game.as_bytes();
    if b.len() >= 2 && b[0] == b'\\' && matches!(b[1], b'\\' | b'/') {
        return Err(NotHere::Unc);
    }
    let rest = if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        if !b[0].eq_ignore_ascii_case(&engine_core::exe::HOST_DRIVE) {
            return Err(NotHere::OtherDrive(b[0].to_ascii_uppercase() as char));
        }
        let rest = &game[2..];
        if rest.is_empty() {
            return Ok("/".to_string());
        }
        if !rest.starts_with(['\\', '/']) {
            return Err(NotHere::DriveRelative);
        }
        rest
    } else {
        game
    };
    Ok(rest.replace('\\', "/"))
}

/// Whether a name of this system's is one Windows cannot hold in a folder, so the game is never
/// shown it ([`hidden_line`], the listing off Windows) and cannot open it: a `:` (a stream to
/// Windows) or a `\` (a separator), a `.` or a space at its end (Windows drops them, so the
/// name would open another file), or `NUL` (the null device).
pub fn windows_cannot_hold(name: &str) -> bool {
    name.contains([':', '\\']) || name.ends_with(['.', ' ']) || name.eq_ignore_ascii_case("NUL")
}

/// The most folders whose hidden names ([`windows_cannot_hold`]) the log names.
pub const MAX_HIDDEN_SAID: usize = 16;

/// The log line for a name the listing of `folder` does not show the game
/// ([`windows_cannot_hold`]): once per folder, for the first [`MAX_HIDDEN_SAID`] folders; the
/// line of the last of those says no more are named.
pub fn hidden_line(folder: &Path, name: &str) -> Option<String> {
    static SAID: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);
    let mut said = SAID.lock().unwrap_or_else(|p| p.into_inner());
    let said = said.get_or_insert_with(HashSet::new);
    if said.len() >= MAX_HIDDEN_SAID || !said.insert(folder.to_path_buf()) {
        return None;
    }
    let more = if said.len() == MAX_HIDDEN_SAID {
        " (no more folders are named)"
    } else {
        ""
    };
    Some(format!(
        "files: {} holds {name:?}, a name Windows cannot hold in a folder (':', '\\', a dot or \
         space at its end, or NUL): the game is not shown it and cannot open it{more}",
        folder.display()
    ))
}

/// A name as Windows compares it: NTFS's upper-casing of its UTF-16 units.
fn fold(name: &str) -> Vec<u16> {
    super::files::ntfs_key(&name.encode_utf16().collect::<Vec<u16>>())
}

/// Whether two names are the same name to Windows: equal after NTFS's upper-casing.
pub fn same_name(a: &str, b: &str) -> bool {
    fold(a) == fold(b)
}

/// The names of the absolute path `path` (`/` between them) as Windows reads them, and whether it
/// ends in a separator (see the module's list): `.` and `..` by the text, one `.` at the end of
/// a folder's name dropped, the dots and spaces at the end of the last name dropped (a last name
/// that is nothing else is dropped, leaving the separator before it at the end).
pub fn win32_parts(path: &str) -> Result<(Vec<&str>, bool), NotHere> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let (rest, last) = path.rsplit_once('/').unwrap_or(("", path));
    // `x/` ends in a separator; `x/.` and `x/..` do not (measured: `x.dds\.` is `x.dds`).
    let mut trailing = last.is_empty() && !rest.is_empty();
    let count = parts.len();
    for (at, part) in parts.iter_mut().enumerate() {
        let name: &str = part;
        if at + 1 < count || trailing {
            if name.ends_with('.') && !name.ends_with("..") {
                *part = &name[..name.len() - 1];
            }
        } else {
            *part = name.trim_end_matches(['.', ' ']);
        }
    }
    if parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
        trailing = true;
    }
    if parts.iter().any(|p| p.contains(':')) {
        return Err(NotHere::Stream);
    }
    if !trailing && parts.last().is_some_and(|p| p.eq_ignore_ascii_case("NUL")) {
        return Err(NotHere::Device);
    }
    let trailing = trailing && !parts.is_empty();
    Ok((parts, trailing))
}

/// A folder that held more than one name for one of a path's parts: the folder, the part as the
/// game wrote it, the names (byte order) and the one taken (for the log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Several {
    pub folder: String,
    pub part: String,
    pub names: Vec<String>,
    pub taken: String,
}

/// The most case-equal names tried for one part, and the most folders tried for one path: a
/// bound on the work for a path whose folders hold many names that differ only in case.
pub const MAX_TWINS: usize = 8;
pub const MAX_STEPS: usize = 64;

/// How [`resolve`] and [`folders`] look at this system: whether a path is there as written, and
/// the names of a folder that match a part without case, in byte order (`None`: no such folder).
pub struct Look<'a> {
    pub exists: &'a mut dyn FnMut(&str) -> bool,
    pub matches: &'a mut dyn FnMut(&str, &str) -> Option<Vec<String>>,
    steps: usize,
}

impl<'a> Look<'a> {
    pub fn new(
        exists: &'a mut dyn FnMut(&str) -> bool,
        matches: &'a mut dyn FnMut(&str, &str) -> Option<Vec<String>>,
    ) -> Self {
        Self {
            exists,
            matches,
            steps: 0,
        }
    }

    /// The folder `out`'s names for `part` other than the name as written when that is there
    /// (`exact`), in byte order and at most [`MAX_TWINS`]; with all of the folder's names for the
    /// part (`None`: no such folder).
    fn next_names(
        &mut self,
        out: &str,
        part: &str,
        exact: bool,
    ) -> (Vec<String>, Option<Vec<String>>) {
        let folder = if out.is_empty() { "/" } else { out };
        let Some(names) = (self.matches)(folder, part) else {
            return (vec![], None);
        };
        // The name as written is there, yet the folder does not hold it so: a file system that
        // ignores case (macOS's by default), where the folder's one name for it IS that entry.
        // No other is tried (each would walk the same folder again).
        if exact && !names.iter().any(|n| n == part) {
            return (vec![], Some(names));
        }
        let others = names
            .iter()
            .filter(|n| !(exact && *n == part))
            .take(MAX_TWINS)
            .cloned()
            .collect();
        (others, Some(names))
    }
}

/// The deepest of the paths from `out` over `rest`: how many of `rest` it holds, the path (the
/// parts it does not hold as written) and the folders that held several names on the way.
fn walk(out: String, rest: &[&str], look: &mut Look) -> (usize, String, Vec<Several>) {
    let Some((&part, after)) = rest.split_first() else {
        return (0, out, vec![]);
    };
    let as_written = |out: &str| (0, format!("{out}/{}", rest.join("/")), vec![]);
    let exact = format!("{out}/{part}");
    let there = (look.exists)(&exact);
    let mut best: Option<(usize, String, Vec<Several>, String)> = None;
    if there {
        look.steps += 1;
        let (found, path, several) = walk(exact, after, look);
        if found == after.len() {
            return (found + 1, path, several);
        }
        best = Some((found + 1, path, several, part.to_string()));
    }
    if look.steps >= MAX_STEPS {
        return best.map_or_else(|| as_written(&out), |(f, p, s, _)| (f, p, s));
    }
    let (others, names) = look.next_names(&out, part, there);
    for name in others {
        if look.steps >= MAX_STEPS {
            break;
        }
        look.steps += 1;
        let (found, path, several) = walk(format!("{out}/{name}"), after, look);
        if best.as_ref().map_or(true, |b| found + 1 > b.0) {
            best = Some((found + 1, path, several, name));
        }
        if found == after.len() {
            break;
        }
    }
    let Some((found, path, mut several, taken)) = best else {
        return as_written(&out);
    };
    if let Some(names) = names.filter(|n| n.len() > 1) {
        let folder = if out.is_empty() { "/".to_string() } else { out };
        several.insert(
            0,
            Several {
                folder,
                part: part.to_string(),
                names,
                taken,
            },
        );
    }
    (found, path, several)
}

/// The path `path` (absolute, `/` between parts) with each part's name as its folder holds it
/// (see the module's list), and the folders where more than one name matched a part (for the
/// log).
pub fn resolve(path: &str, look: &mut Look) -> Result<(String, Vec<Several>), NotHere> {
    let (parts, trailing) = win32_parts(path)?;
    let (_, mut out, several) = walk(String::new(), &parts, look);
    if out.is_empty() || trailing {
        out.push('/');
    }
    Ok((out, several))
}

/// Every path that holds all of `path`'s parts (absolute, `/` between them), in the order
/// [`resolve`] tries them (so the first is the one it takes), at most [`MAX_TWINS`] and within
/// [`MAX_STEPS`]; empty when none does.
pub fn folders(path: &str, look: &mut Look) -> Result<Vec<String>, NotHere> {
    fn chains(out: String, rest: &[&str], look: &mut Look, all: &mut Vec<String>) {
        let Some((&part, after)) = rest.split_first() else {
            all.push(if out.is_empty() { "/".to_string() } else { out });
            return;
        };
        let exact = format!("{out}/{part}");
        let there = (look.exists)(&exact);
        let mut names = if there {
            vec![part.to_string()]
        } else {
            vec![]
        };
        names.extend(look.next_names(&out, part, there).0);
        for name in names {
            if all.len() >= MAX_TWINS || look.steps >= MAX_STEPS {
                return;
            }
            look.steps += 1;
            chains(format!("{out}/{name}"), after, look, all);
        }
    }
    let (parts, _) = win32_parts(path)?;
    let mut all = Vec::new();
    chains(String::new(), &parts, look, &mut all);
    Ok(all)
}

/// A listing read within this time after its folder's last change is not kept: a file system's
/// clock moves in ticks (1 to 10 ms on Linux, 1 s on HFS+, 2 s on FAT), so another change in the
/// same tick would leave the time as it was and the kept names stale. A folder whose time is
/// after the reading (unpacked from an archive made in a time zone ahead, a share whose clock
/// is ahead) is kept: any change to it now gives it the time of now, which is not its time.
pub const SETTLED: Duration = Duration::from_secs(2);

/// The most folders whose names are kept; past it the one kept longest ago is dropped.
pub const MAX_FOLDERS: usize = 4096;

/// A folder's names, each group of names Windows takes as one ([`fold`]) in byte order, with the
/// folder's modification time when read and when it was kept (for the bound).
struct Listing {
    modified: SystemTime,
    by_name: HashMap<Vec<u16>, Vec<String>>,
    kept: u64,
}

impl Listing {
    fn new(modified: SystemTime, names: Vec<String>) -> Self {
        let mut by_name: HashMap<Vec<u16>, Vec<String>> = HashMap::new();
        for name in names {
            by_name.entry(fold(&name)).or_default().push(name);
        }
        for group in by_name.values_mut() {
            group.sort();
        }
        Self {
            modified,
            by_name,
            kept: 0,
        }
    }

    fn matching(&self, key: &[u16]) -> Vec<String> {
        self.by_name.get(key).cloned().unwrap_or_default()
    }
}

/// The folders' names read so far ([`folder_matches`]), by the folder's key ([`folder_key`]).
#[derive(Default)]
pub struct Listings {
    folders: HashMap<String, Listing>,
    next: u64,
}

impl Listings {
    /// The names of `folder` that match `key` when its names were kept at the modification time
    /// `modified`.
    fn get(&self, folder: &str, modified: SystemTime, key: &[u16]) -> Option<Vec<String>> {
        let listing = self.folders.get(folder)?;
        (listing.modified == modified).then(|| listing.matching(key))
    }

    /// Keeps `listing` for `folder` unless it was read within [`SETTLED`] after its last change,
    /// `now` being when it was read; at most [`MAX_FOLDERS`] folders, the one kept longest ago
    /// dropped for a new one.
    fn keep(&mut self, folder: &str, mut listing: Listing, now: SystemTime) {
        if matches!(now.duration_since(listing.modified), Ok(age) if age < SETTLED) {
            self.folders.remove(folder);
            return;
        }
        if self.folders.len() >= MAX_FOLDERS && !self.folders.contains_key(folder) {
            let oldest = self
                .folders
                .iter()
                .min_by_key(|(_, l)| l.kept)
                .map(|(k, _)| k.clone());
            if let Some(oldest) = oldest {
                self.folders.remove(&oldest);
            }
        }
        listing.kept = self.next;
        self.next += 1;
        self.folders.insert(folder.to_string(), listing);
    }
}

static LISTINGS: Mutex<Option<Listings>> = Mutex::new(None);

/// The key a folder's names are kept under: on Linux and macOS the folder itself (its device
/// and number), so every spelling of it (another case on macOS, a link, `..`) is one folder kept
/// once and forgotten once; elsewhere (the tests on Windows) its path.
fn folder_key(folder: &str, meta: &std::fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = folder;
        format!("#{}:{}", meta.dev(), meta.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        folder.to_string()
    }
}

/// The names of the folder `folder` on this system that match `part` without case, in byte order
/// (`None` if there is no such folder), from [`LISTINGS`] while the folder's modification time is
/// the one read with them. The lock is held only to look and to keep, not while reading.
pub fn folder_matches(folder: &str, part: &str) -> Option<Vec<String>> {
    let key = fold(part);
    let meta = std::fs::metadata(folder).ok()?;
    let at = folder_key(folder, &meta);
    let modified = meta.modified().ok();
    if let Some(modified) = modified {
        let kept = LISTINGS.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(found) = kept.as_ref().and_then(|l| l.get(&at, modified, &key)) {
            return Some(found);
        }
    }
    let now = SystemTime::now();
    let names: Vec<String> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    let Some(modified) = modified else {
        return Some(Listing::new(SystemTime::UNIX_EPOCH, names).matching(&key));
    };
    let listing = Listing::new(modified, names);
    let found = listing.matching(&key);
    let mut kept = LISTINGS.lock().unwrap_or_else(|p| p.into_inner());
    kept.get_or_insert_with(Listings::default)
        .keep(&at, listing, now);
    Some(found)
}

/// The port wrote into or removed from the folder `folder` (this system's path, any spelling of
/// it on Linux and macOS: [`folder_key`]): its kept names are read again at the next look,
/// whatever its modification time says. The folder's time is what keeps the names current
/// ([`SETTLED`]); this is for a file system whose clock does not.
pub fn forget(folder: &Path) {
    let Ok(meta) = std::fs::metadata(folder) else {
        return;
    };
    let at = folder_key(&folder.to_string_lossy(), &meta);
    let mut kept = LISTINGS.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(kept) = kept.as_mut() {
        kept.folders.remove(&at);
    }
}

/// Where a drive this system does not have lies: a folder that is never there (Debian's policy
/// keeps `/nonexistent` absent, for exactly this use; macOS has none by default), so every path
/// on such a drive answers as a missing file or folder.
pub const ABSENT_DRIVES: &str = "/nonexistent/nv-rs drive";

/// [`ABSENT_DRIVES`]' path for `game` on the drive `drive`: `/nonexistent/nv-rs drive C/...`.
pub fn absent_drive_path(drive: char, game: &str) -> PathBuf {
    PathBuf::from(format!(
        "{ABSENT_DRIVES} {drive}{}",
        game.get(2..).unwrap_or("").replace('\\', "/")
    ))
}

/// The log line for a path on the drive `drive`, which this system does not have: `Some` the first
/// time in the run for that drive, `None` after (the line is said once per drive, never silently
/// skipped).
pub fn absent_drive_line(drive: char) -> Option<String> {
    static SAID: Mutex<Option<HashSet<char>>> = Mutex::new(None);
    let mut said = SAID.lock().unwrap_or_else(|p| p.into_inner());
    said.get_or_insert_with(HashSet::new)
        .insert(drive)
        .then(|| {
            format!(
            "files: drive {drive}: is not on this system (its folders are the game's drive {}:): \
             everything on it answers as missing, as on a PC without that drive",
            engine_core::exe::HOST_DRIVE as char
        )
        })
}

/// What a refused path stops the run with.
fn refused(game: &str, why: NotHere) -> ! {
    match why {
        NotHere::OtherDrive(_) => unreachable!("another drive is an absent one"),
        NotHere::DriveRelative => {
            panic!("the game's path {game:?}: a path relative to a drive is not provided")
        }
        NotHere::Unc => panic!(
            "the game's path {game:?}: network and device paths (\\\\server, \\\\?\\) are not \
             provided off Windows"
        ),
        NotHere::Device => {
            panic!("the game's path {game:?}: Windows' null device (NUL) is not provided")
        }
        NotHere::Stream => unreachable!("a stream answers as missing"),
    }
}

/// Where a path naming a file's stream lies ([`NotHere::Stream`]): a folder that is never there,
/// so it answers as missing.
pub const NO_STREAMS: &str = "/nonexistent/nv-rs streams";

/// This system's path for `game`, whose names read as `text` were refused for `why`: a stream is
/// missing ([`NO_STREAMS`], said once in the run); the others stop the run ([`refused`]).
fn not_here(game: &str, text: &str, why: NotHere) -> PathBuf {
    if why != NotHere::Stream {
        refused(game, why);
    }
    static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
        eprintln!(
            "files: the game's path {game:?} names a file's stream (a ':' in a name): no stream \
             is provided, it answers as missing (said once)"
        );
    }
    PathBuf::from(format!("{NO_STREAMS}{text}"))
}

/// The game's path `game` as this system's text ([`unix_text`]), or the path it is when it is on
/// a drive this system does not have (said once per drive).
fn host_text(game: &str) -> Result<String, PathBuf> {
    match unix_text(game) {
        Ok(text) => Ok(text),
        Err(NotHere::OtherDrive(d)) => {
            if let Some(line) = absent_drive_line(d) {
                eprintln!("{line}");
            }
            Err(absent_drive_path(d, game))
        }
        Err(why) => refused(game, why),
    }
}

/// The log lines for the folders that held several names for a part: one per folder and name
/// (the name as Windows compares it) in the run, none for one said before.
fn several_lines(several: Vec<Several>) -> Vec<String> {
    if several.is_empty() {
        return vec![];
    }
    /// A folder and a part's name as Windows compares it.
    type Said = HashSet<(String, Vec<u16>)>;
    static SAID: Mutex<Option<Said>> = Mutex::new(None);
    let mut said = SAID.lock().unwrap_or_else(|p| p.into_inner());
    let said = said.get_or_insert_with(HashSet::new);
    several
        .into_iter()
        .filter(|s| said.insert((s.folder.clone(), fold(&s.part))))
        .map(|s| {
            format!(
                "files: {} holds {} names for the game's {:?} ({}); {:?} is taken",
                s.folder,
                s.names.len(),
                s.part,
                s.names.join(", "),
                s.taken
            )
        })
        .collect()
}

/// The game's path `game` as this system's path ([`unix_text`], then [`resolve`] against this
/// system's folders, a folder holding two names for one part said once in the log). A drive
/// other than `Z:` is absent ([`absent_drive_path`], said once per drive), a stream missing
/// ([`NO_STREAMS`]); the other paths [`NotHere`] names stop the run, as calls not provided.
pub fn host_path(game: &str) -> PathBuf {
    let text = match host_text(game) {
        Ok(text) => text,
        Err(absent) => return absent,
    };
    if !text.starts_with('/') {
        return PathBuf::from(text);
    }
    let mut exists = |p: &str| std::fs::symlink_metadata(p).is_ok();
    let mut matches = folder_matches;
    let (found, several) = match resolve(&text, &mut Look::new(&mut exists, &mut matches)) {
        Ok(resolved) => resolved,
        Err(why) => return not_here(game, &text, why),
    };
    for line in several_lines(several) {
        eprintln!("{line}");
    }
    PathBuf::from(found)
}

/// Every folder of this system that the game's folder `game` names ([`folders`]: the ones whose
/// names differ only in case, which are one folder to Windows), the one [`host_path`] takes
/// first; when none is there, that path alone (the listing answers as for a missing folder).
pub fn host_folders(game: &str) -> Vec<PathBuf> {
    let text = match host_text(game) {
        Ok(text) => text,
        Err(absent) => return vec![absent],
    };
    if !text.starts_with('/') {
        return vec![PathBuf::from(text)];
    }
    let mut exists = |p: &str| std::fs::symlink_metadata(p).is_ok();
    let mut matches = folder_matches;
    let all = match folders(&text, &mut Look::new(&mut exists, &mut matches)) {
        Ok(all) => all,
        Err(why) => return vec![not_here(game, &text, why)],
    };
    if all.is_empty() {
        return vec![host_path(game)];
    }
    all.into_iter().map(PathBuf::from).collect()
}

/// The name of the file or folder `path` (this system's, as [`host_path`] made it) as its folder
/// holds it: the name itself when the folder holds it so (Linux, where [`host_path`] took it
/// from the folder), else the first in byte order that matches it without case (macOS's file
/// systems that ignore case, where a name is there in any case); `None` when there is none.
pub fn stored_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let folder = path.parent()?.to_str()?;
    let folder = if folder.is_empty() { "/" } else { folder };
    stored_of(&folder_matches(folder, name)?, name)
}

/// Of a folder's names that match `name` without case (byte order), the one it is stored as:
/// `name` itself when there, else the first.
fn stored_of(names: &[String], name: &str) -> Option<String> {
    names.iter().find(|n| *n == name).or(names.first()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up folder tree: each folder's names.
    fn tree() -> HashMap<&'static str, Vec<&'static str>> {
        HashMap::from([
            ("/", vec!["home", "tmp"]),
            ("/home", vec!["me"]),
            ("/home/me", vec!["Fallout New Vegas"]),
            (
                "/home/me/Fallout New Vegas",
                vec!["Data", "FalloutNV.exe", "data"],
            ),
            (
                "/home/me/Fallout New Vegas/Data",
                vec!["Meshes", "Fallout.esm", "Textures", "textures", "both.txt"],
            ),
            (
                "/home/me/Fallout New Vegas/data",
                vec!["readme.txt", "both.txt"],
            ),
            (
                "/home/me/Fallout New Vegas/Data/Meshes",
                vec!["Armor", "Ägypten"],
            ),
            ("/home/me/Fallout New Vegas/Data/Textures", vec!["Clutter"]),
            ("/home/me/Fallout New Vegas/Data/Textures/Clutter", vec![]),
            ("/home/me/Fallout New Vegas/Data/textures", vec!["armor"]),
            (
                "/home/me/Fallout New Vegas/Data/textures/armor",
                vec!["x.dds"],
            ),
        ])
    }

    /// [`tree`]'s names of `folder` that match `part` without case, in byte order.
    fn tree_matches(folder: &str, part: &str) -> Option<Vec<String>> {
        let t = tree();
        let mut names: Vec<String> = t
            .get(folder)?
            .iter()
            .filter(|n| same_name(n, part))
            .map(|s| s.to_string())
            .collect();
        names.sort();
        Some(names)
    }

    fn tree_exists(p: &str) -> bool {
        let (folder, name) = p.rsplit_once('/').unwrap();
        let folder = if folder.is_empty() { "/" } else { folder };
        tree().get(folder).is_some_and(|n| n.contains(&name))
    }

    /// [`resolve`] over [`tree`], and how many folders it listed.
    fn resolved_counting(path: &str) -> (String, Vec<Several>, usize) {
        let mut listed = 0;
        let mut exists = tree_exists;
        let mut matches = |f: &str, p: &str| {
            listed += 1;
            tree_matches(f, p)
        };
        let (path, several) = resolve(path, &mut Look::new(&mut exists, &mut matches)).unwrap();
        (path, several, listed)
    }

    fn resolved(path: &str) -> (String, Vec<Several>) {
        let (path, several, _) = resolved_counting(path);
        (path, several)
    }

    fn folders_of(path: &str) -> Vec<String> {
        let mut exists = tree_exists;
        let mut matches = tree_matches;
        folders(path, &mut Look::new(&mut exists, &mut matches)).unwrap()
    }

    #[test]
    fn drive_z_is_this_systems_root_and_other_drives_are_not_here() {
        assert_eq!(
            unix_text(r"Z:\home\me\Fallout New Vegas\Data\Fallout.esm").unwrap(),
            "/home/me/Fallout New Vegas/Data/Fallout.esm"
        );
        assert_eq!(unix_text(r"z:/home\me").unwrap(), "/home/me");
        assert_eq!(unix_text(r"Z:\").unwrap(), "/");
        assert_eq!(
            unix_text(r"C:\Games\x").unwrap_err(),
            NotHere::OtherDrive('C')
        );
        assert_eq!(unix_text(r"d:\x").unwrap_err(), NotHere::OtherDrive('D'));
        assert_eq!(unix_text(r"Z:Data\x").unwrap_err(), NotHere::DriveRelative);
        // A bare `Z:` (the folder of `Z:\nv.ini` as the file calls cut it, of `Z:\*` listed) is
        // the root.
        assert_eq!(unix_text("Z:").unwrap(), "/");
        assert_eq!(unix_text("z:").unwrap(), "/");
        assert_eq!(unix_text("C:").unwrap_err(), NotHere::OtherDrive('C'));
        assert_eq!(host_path("Z:"), PathBuf::from("/"));
        assert_eq!(
            unix_text(r"/tmp/scratch\a.txt").unwrap(),
            "/tmp/scratch/a.txt"
        );
        assert_eq!(unix_text(r"Data\Meshes").unwrap(), "Data/Meshes");
    }

    /// Another drive is one this system does not have: its paths lie under a folder that is never
    /// there, so they answer as missing.
    #[test]
    fn another_drive_is_an_absent_drive() {
        assert_eq!(
            absent_drive_path('C', r"C:\Games\FNV\Data\x.esp"),
            PathBuf::from("/nonexistent/nv-rs drive C/Games/FNV/Data/x.esp")
        );
        assert_eq!(
            absent_drive_path('Q', r"q:\"),
            PathBuf::from("/nonexistent/nv-rs drive Q/")
        );
    }

    /// Another drive is never laid over this system's root (Z:'s): `C:\home` is not `/home`, and
    /// [`host_path`] takes it there (no folder of this system is looked at).
    #[test]
    fn another_drive_is_not_this_systems_root() {
        let p = absent_drive_path('C', r"C:\home\me");
        assert!(p.to_string_lossy().starts_with(ABSENT_DRIVES), "{p:?}");
        assert_ne!(p, PathBuf::from("/home/me"));
        assert!(!p.to_string_lossy().starts_with("/home"));
        assert_eq!(host_path(r"C:\home\me"), p);
        assert_eq!(host_folders(r"c:\home\me"), [p]);
    }

    /// The absent drive's line is said the first time for each drive, and not again.
    #[test]
    fn the_absent_drive_line_is_said_once_per_drive() {
        let first = absent_drive_line('K').expect("said the first time");
        assert!(
            first.contains("drive K:") && first.contains("missing"),
            "{first}"
        );
        assert_eq!(absent_drive_line('K'), None);
        assert_eq!(absent_drive_line('k'.to_ascii_uppercase()), None);
        assert!(absent_drive_line('J').is_some());
    }

    /// Network and device paths are refused, not mapped onto this system's root.
    #[test]
    fn network_and_device_paths_are_refused() {
        assert_eq!(unix_text(r"\\server\share\x"), Err(NotHere::Unc));
        assert_eq!(unix_text(r"\\?\UNC\server\share\x"), Err(NotHere::Unc));
        assert_eq!(unix_text(r"\\?\Volume{x}\y"), Err(NotHere::Unc));
        assert_eq!(unix_text(r"\\.\PhysicalDrive0"), Err(NotHere::Unc));
        assert_eq!(unix_text(r"\\.\Z:\x"), Err(NotHere::Unc));
        // The long-path form of a drive's path is that path.
        assert_eq!(unix_text(r"\\?\Z:\tmp\x").unwrap(), "/tmp/x");
        assert_eq!(unix_text(r"\\?\C:\x"), Err(NotHere::OtherDrive('C')));
        assert_eq!(unix_text(r"\/x"), Err(NotHere::Unc));
        // A host path with a doubled `/` is still this system's.
        assert_eq!(unix_text("//tmp/x").unwrap(), "//tmp/x");
    }

    #[test]
    #[should_panic(expected = "network and device paths")]
    fn a_network_path_stops_the_run() {
        host_path(r"\\server\share\x");
    }

    #[test]
    fn the_game_view_and_back_is_the_same_folder() {
        let host = "/home/me/Fallout New Vegas";
        assert_eq!(unix_text(&engine_core::exe::game_view(host)).unwrap(), host);
    }

    /// Windows' reading of a path's names (measured with `GetFullPathNameW` and `CreateFileW`,
    /// Windows 11 26200, 2026-11-16; `D` a scratch folder holding `x.dds` and `sub\y.txt`).
    #[test]
    fn names_are_read_as_windows_reads_them() {
        fn parts(p: &str) -> (Vec<&str>, bool) {
            win32_parts(p).unwrap()
        }
        // The last name loses its dots and spaces: `x.dds `, `x.dds.`, `x.dds. . `, `x.dds..`.
        for p in ["/D/x.dds ", "/D/x.dds.", "/D/x.dds. . ", "/D/x.dds.."] {
            assert_eq!(parts(p), (vec!["D", "x.dds"], false), "{p:?}");
        }
        // A folder's name loses one dot at its end, not two and not a space.
        assert_eq!(parts("/D/sub./y.txt"), (vec!["D", "sub", "y.txt"], false));
        assert_eq!(parts("/D/a.b./x"), (vec!["D", "a.b", "x"], false));
        assert_eq!(
            parts("/D/sub../y.txt"),
            (vec!["D", "sub..", "y.txt"], false)
        );
        assert_eq!(parts("/D/sub /y.txt"), (vec!["D", "sub ", "y.txt"], false));
        assert_eq!(parts("/D/.../x.dds"), (vec!["D", "...", "x.dds"], false));
        // A separator at the end stays (`x.dds\` fails with `ERROR_DIRECTORY`); `sub.\` is `sub\`.
        assert_eq!(parts("/D/x.dds/"), (vec!["D", "x.dds"], true));
        assert_eq!(parts("/D/sub./"), (vec!["D", "sub"], true));
        assert_eq!(parts("/D/sub/y.txt./"), (vec!["D", "sub", "y.txt"], true));
        // A last name of dots and spaces only goes, its separator staying: `sub\...`,
        // `sub\. `, `sub\.. ` are `sub\`; `y.txt\. ` is `y.txt\`.
        for p in ["/D/sub/...", "/D/sub/. ", "/D/sub/.. "] {
            assert_eq!(parts(p), (vec!["D", "sub"], true), "{p:?}");
        }
        assert_eq!(parts("/D/sub/y.txt/. "), (vec!["D", "sub", "y.txt"], true));
        // `.` and `..` by the text: `x.dds\.` is `x.dds`, `sub\..` is `D`.
        assert_eq!(parts("/D/x.dds/."), (vec!["D", "x.dds"], false));
        assert_eq!(parts("/D/sub/.."), (vec!["D"], false));
        assert_eq!(
            parts("/D/sub/y.txt/../y.txt"),
            (vec!["D", "sub", "y.txt"], false)
        );
        assert_eq!(
            parts("/D/sub./x /y.txt "),
            (vec!["D", "sub", "x ", "y.txt"], false)
        );
        // A space or dot before a name stays: ` x.dds`, `x .dds`.
        assert_eq!(parts("/D/ x.dds"), (vec!["D", " x.dds"], false));
        assert_eq!(parts("/D/x .dds"), (vec!["D", "x .dds"], false));
        assert_eq!(parts("/"), (vec![], false));
    }

    /// `NUL` as the last name is the null device in every folder; the other old device names
    /// are plain names on Windows 11 (measured); a `:` is a stream.
    #[test]
    fn the_null_device_and_streams_are_not_provided() {
        for p in [
            "/D/NUL",
            "/D/nul",
            "/D/Nul",
            "/D/NUL.",
            "/D/NUL ",
            "/D/sub/NUL",
        ] {
            assert_eq!(win32_parts(p), Err(NotHere::Device), "{p:?}");
        }
        for p in [
            "/D/nul.txt",
            "/D/CON",
            "/D/AUX.dds",
            "/D/COM1",
            "/D/LPT1.x",
            "/D/NUL/x",
            "/D/nul./",
        ] {
            assert!(win32_parts(p).is_ok(), "{p:?}");
        }
        for p in [
            "/D/x.dds:s",
            "/D/x.dds::$DATA",
            "/D/sub/y.txt:s:$DATA",
            "/D/Z:x/y",
        ] {
            assert_eq!(win32_parts(p), Err(NotHere::Stream), "{p:?}");
        }
    }

    #[test]
    #[should_panic(expected = "null device")]
    fn the_null_device_stops_the_run() {
        host_path(r"Z:\tmp\NUL");
    }

    /// A stream (also a `:` the current folder's name holds, `Z:x` made full) is missing, never
    /// the file of this system named with a `:`.
    #[test]
    fn a_stream_answers_as_missing() {
        assert_eq!(
            host_path(r"Z:\tmp\x.dds:s"),
            PathBuf::from("/nonexistent/nv-rs streams/tmp/x.dds:s")
        );
        assert_eq!(
            host_folders(r"Z:\tmp\a:b"),
            [PathBuf::from("/nonexistent/nv-rs streams/tmp/a:b")]
        );
        assert!(host_path(r"Z:\home\me\Z:x").starts_with(NO_STREAMS));
    }

    /// Names Windows cannot hold in a folder are kept from the game.
    #[test]
    fn names_windows_cannot_hold_are_known() {
        for name in [
            "Mod: Part 2.esp",
            r"a\b.esp",
            "readme.txt ",
            "readme.txt.",
            "...",
            "NUL",
            "nul",
        ] {
            assert!(windows_cannot_hold(name), "{name:?}");
        }
        for name in [
            "readme.txt",
            "nul.txt",
            "CON",
            ".hidden",
            " x.esp",
            "x .esp",
        ] {
            assert!(!windows_cannot_hold(name), "{name:?}");
        }
    }

    /// A hidden name is said once per folder, for at most [`MAX_HIDDEN_SAID`] folders.
    #[test]
    fn hidden_names_are_said_once_per_folder_and_capped() {
        let first = hidden_line(Path::new("/hidden-0"), "a:b").expect("said");
        assert!(
            first.contains("/hidden-0") && first.contains("\"a:b\""),
            "{first}"
        );
        assert_eq!(hidden_line(Path::new("/hidden-0"), "c:d"), None);
        let mut said = 1;
        for i in 1..MAX_HIDDEN_SAID + 5 {
            said += usize::from(hidden_line(Path::new(&format!("/hidden-{i}")), "x.").is_some());
        }
        // Other tests may have said some of the 16 already; never more than 16.
        assert!(said <= MAX_HIDDEN_SAID, "{said}");
        assert_eq!(hidden_line(Path::new("/hidden-last"), "x."), None);
    }

    #[test]
    fn each_part_is_found_without_case() {
        assert_eq!(
            resolved("/HOME/ME/fallout new vegas/DATA/meshes/ARMOR/x.nif").0,
            "/home/me/Fallout New Vegas/Data/Meshes/Armor/x.nif"
        );
        // Outside ASCII, by NTFS's upper-casing: ä and Ä are one letter to Windows.
        assert_eq!(
            resolved("/home/me/Fallout New Vegas/Data/Meshes/äGYPTEN").0,
            "/home/me/Fallout New Vegas/Data/Meshes/Ägypten"
        );
    }

    /// A name written as the folder holds it is taken without listing the folder.
    #[test]
    fn a_name_written_as_it_is_is_taken_as_it_is() {
        let (path, several, listed) =
            resolved_counting("/home/me/Fallout New Vegas/data/readme.txt");
        assert_eq!(path, "/home/me/Fallout New Vegas/data/readme.txt");
        assert!(several.is_empty());
        assert_eq!(listed, 0);
    }

    #[test]
    fn two_names_for_one_part_take_the_first_in_byte_order_and_say_so() {
        let (path, several) = resolved("/home/me/Fallout New Vegas/DATA/Fallout.esm");
        assert_eq!(path, "/home/me/Fallout New Vegas/Data/Fallout.esm");
        assert_eq!(
            several,
            [Several {
                folder: "/home/me/Fallout New Vegas".to_string(),
                part: "DATA".to_string(),
                names: vec!["Data".to_string(), "data".to_string()],
                taken: "Data".to_string(),
            }]
        );
    }

    /// Two folders whose names differ only in case are one folder to Windows: a file only in the
    /// second is found by any spelling, even one that names the first exactly.
    #[test]
    fn a_file_in_the_other_of_two_twin_folders_is_found() {
        let (path, several) = resolved("/home/me/Fallout New Vegas/DATA/readme.txt");
        assert_eq!(path, "/home/me/Fallout New Vegas/data/readme.txt");
        assert_eq!(several.len(), 1);
        assert_eq!(several[0].taken, "data");
        assert_eq!(
            resolved("/home/me/Fallout New Vegas/Data/README.TXT").0,
            "/home/me/Fallout New Vegas/data/readme.txt"
        );
        // Two levels: `Textures` is there as written, but `Armor\x.dds` only under `textures`.
        assert_eq!(
            resolved("/home/me/Fallout New Vegas/Data/Textures/Armor/x.dds").0,
            "/home/me/Fallout New Vegas/Data/textures/armor/x.dds"
        );
        // In both: the first in [`resolve`]'s order, the name as written first.
        assert_eq!(
            resolved("/home/me/Fallout New Vegas/data/both.txt").0,
            "/home/me/Fallout New Vegas/data/both.txt"
        );
        assert_eq!(
            resolved("/home/me/Fallout New Vegas/DATA/both.txt").0,
            "/home/me/Fallout New Vegas/Data/both.txt"
        );
    }

    /// The listing of a folder that is several here is all of them, in [`resolve`]'s order.
    #[test]
    fn twin_folders_are_listed_together_in_the_order_they_are_tried() {
        assert_eq!(
            folders_of("/home/me/Fallout New Vegas/DATA"),
            [
                "/home/me/Fallout New Vegas/Data",
                "/home/me/Fallout New Vegas/data"
            ]
        );
        assert_eq!(
            folders_of("/home/me/Fallout New Vegas/data"),
            [
                "/home/me/Fallout New Vegas/data",
                "/home/me/Fallout New Vegas/Data"
            ]
        );
        assert_eq!(
            folders_of("/home/me/fallout new vegas/data/TEXTURES"),
            [
                "/home/me/Fallout New Vegas/Data/Textures",
                "/home/me/Fallout New Vegas/Data/textures"
            ]
        );
        assert_eq!(
            folders_of("/home/me/Fallout New Vegas/Data/Meshes"),
            ["/home/me/Fallout New Vegas/Data/Meshes"]
        );
        assert!(folders_of("/home/me/Fallout New Vegas/Sound").is_empty());
        assert_eq!(folders_of("/"), ["/"]);
    }

    /// When no path holds every part, the one holding the most is taken and the rest kept as
    /// written.
    #[test]
    fn from_a_missing_part_on_the_rest_is_kept_as_written() {
        assert_eq!(
            resolved("/home/me/fallout new vegas/Data/Textures/Armor/y.dds").0,
            "/home/me/Fallout New Vegas/Data/textures/armor/y.dds"
        );
        assert_eq!(
            resolved("/home/me/fallout new vegas/Data/Sound/x.wav").0,
            "/home/me/Fallout New Vegas/Data/Sound/x.wav"
        );
        assert_eq!(resolved("/nowhere/Data").0, "/nowhere/Data");
    }

    /// A folder holding many names that differ only in case is tried at most [`MAX_TWINS`]
    /// times per part, and a path at most [`MAX_STEPS`] times.
    #[test]
    fn the_search_over_twin_folders_is_bounded() {
        let mut steps = 0;
        let mut exists = |_: &str| false;
        let mut matches = |_: &str, part: &str| {
            steps += 1;
            // Every folder holds 20 names for every part but the last, which is nowhere.
            Some(if part == "f" {
                vec![]
            } else {
                (0..20).map(|i| format!("{part}{i:02}")).collect()
            })
        };
        let (path, _) = resolve("/a/b/c/d/e/f", &mut Look::new(&mut exists, &mut matches)).unwrap();
        // At most one folder listed per step and one for the start (MAX_STEPS is 64): without
        // the bound it would be 8 names at each of 5 levels, tens of thousands.
        assert!(steps <= 65, "{steps}");
        assert_eq!(path, "/a00/b00/c00/d00/e00/f");
    }

    /// A file system that ignores case (macOS's by default): every spelling of a name is there.
    /// A missing file under folders the game spells otherwise is looked for once per folder,
    /// not again under each folder's stored spelling.
    #[test]
    fn a_file_system_that_ignores_case_walks_each_folder_once() {
        let folders: HashMap<&str, Vec<&str>> = HashMap::from([
            ("", vec!["Games"]),
            ("/Games", vec!["Fallout New Vegas"]),
            ("/Games/Fallout New Vegas", vec!["Data"]),
            ("/Games/Fallout New Vegas/Data", vec!["Textures"]),
            ("/Games/Fallout New Vegas/Data/Textures", vec!["Armor"]),
            (
                "/Games/Fallout New Vegas/Data/Textures/Armor",
                vec!["x.dds"],
            ),
        ]);
        // The stored path of `p` in any case.
        let stored = |p: &str| -> Option<String> {
            let mut at = String::new();
            for part in p.split('/').filter(|s| !s.is_empty()) {
                let name = folders
                    .get(at.as_str())?
                    .iter()
                    .find(|n| same_name(n, part))?;
                at = format!("{at}/{name}");
            }
            Some(at)
        };
        let (mut exists_calls, mut listings) = (0, 0);
        let mut exists = |p: &str| {
            exists_calls += 1;
            stored(p).is_some()
        };
        let mut matches = |folder: &str, part: &str| {
            listings += 1;
            let at = stored(folder)?;
            let held = folders.get(at.as_str())?;
            Some(
                held.iter()
                    .filter(|n| same_name(n, part))
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>(),
            )
        };
        let (path, several) = resolve(
            "/games/fallout new vegas/data/textures/armor/missing.dds",
            &mut Look::new(&mut exists, &mut matches),
        )
        .unwrap();
        assert_eq!(
            path,
            "/games/fallout new vegas/data/textures/armor/missing.dds"
        );
        assert!(several.is_empty());
        // One look at each of the 6 names, one listing of each of the 6 folders.
        assert_eq!((exists_calls, listings), (6, 6));
        // A file that is there opens with no listing at all.
        let (mut exists_calls, mut listings) = (0, 0);
        let mut exists = |p: &str| {
            exists_calls += 1;
            stored(p).is_some()
        };
        let mut matches = |_: &str, _: &str| {
            listings += 1;
            None
        };
        resolve(
            "/GAMES/fallout new vegas/DATA/textures/armor/X.DDS",
            &mut Look::new(&mut exists, &mut matches),
        )
        .unwrap();
        assert_eq!((exists_calls, listings), (6, 0));
    }

    #[test]
    fn dots_go_by_the_text_of_the_path() {
        assert_eq!(
            resolved("/home/./me/Fallout New Vegas/Data/../data/./README.TXT").0,
            "/home/me/Fallout New Vegas/data/readme.txt"
        );
        assert_eq!(resolved("/../home").0, "/home");
        assert_eq!(resolved("/").0, "/");
        assert_eq!(resolved("//home//me").0, "/home/me");
        assert_eq!(resolved("/home/me/").0, "/home/me/");
        assert_eq!(
            resolved("/home/me./FALLOUT NEW VEGAS. ").0,
            "/home/me/Fallout New Vegas"
        );
    }

    #[test]
    fn same_name_is_windows_comparison() {
        assert!(same_name("Fallout.ESM", "fallout.esm"));
        assert!(same_name("Ägypten", "äGYPTEN"));
        assert!(!same_name("lodz.txt", "\u{142}odz.txt"));
        assert!(!same_name("a.txt", "a.txt "));
    }

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// A folder's names are kept while its time is the one read with them, grouped by Windows'
    /// comparison in byte order.
    #[test]
    fn kept_names_answer_while_the_folder_time_stays() {
        let mut kept = Listings::default();
        kept.keep(
            "/d",
            Listing::new(at(100), names(&["b.txt", "B.TXT", "a.txt", "b.TXT"])),
            at(200),
        );
        assert_eq!(
            kept.get("/d", at(100), &fold("b.Txt")),
            Some(names(&["B.TXT", "b.TXT", "b.txt"]))
        );
        assert_eq!(kept.get("/d", at(100), &fold("c.txt")), Some(vec![]));
        assert_eq!(kept.get("/d", at(101), &fold("a.txt")), None);
        assert_eq!(kept.get("/e", at(100), &fold("a.txt")), None);
    }

    /// A listing read within [`SETTLED`] of its folder's change is not kept (another change in
    /// that tick would not move the time), and replaces none.
    #[test]
    fn a_listing_read_just_after_a_change_is_not_kept() {
        let mut kept = Listings::default();
        kept.keep("/d", Listing::new(at(100), names(&["a"])), at(100));
        assert_eq!(kept.get("/d", at(100), &fold("a")), None);
        kept.keep("/d", Listing::new(at(100), names(&["a"])), at(102));
        assert_eq!(kept.get("/d", at(100), &fold("a")), Some(names(&["a"])));
        kept.keep("/d", Listing::new(at(103), names(&["b"])), at(104));
        assert_eq!(kept.get("/d", at(100), &fold("a")), None);
        assert_eq!(kept.get("/d", at(103), &fold("b")), None);
        // A folder whose time is after the reading (unpacked with times ahead) is kept: a change
        // now would give it the time of now.
        kept.keep("/d", Listing::new(at(110), names(&["b"])), at(104));
        assert_eq!(kept.get("/d", at(110), &fold("b")), Some(names(&["b"])));
    }

    /// At most [`MAX_FOLDERS`] folders are kept, the one kept longest ago dropped first.
    #[test]
    fn the_kept_folders_are_bounded() {
        let mut kept = Listings::default();
        for i in 0..MAX_FOLDERS + 10 {
            kept.keep(&format!("/{i}"), Listing::new(at(1), names(&["a"])), at(10));
            assert!(kept.folders.len() <= MAX_FOLDERS);
        }
        assert_eq!(kept.folders.len(), MAX_FOLDERS);
        for gone in 0..10 {
            assert_eq!(kept.get(&format!("/{gone}"), at(1), &fold("a")), None);
        }
        for still in [10, 11, MAX_FOLDERS, MAX_FOLDERS + 9] {
            assert_eq!(
                kept.get(&format!("/{still}"), at(1), &fold("A")),
                Some(names(&["a"])),
                "{still}"
            );
        }
        // A folder kept again when full replaces itself, dropping nothing.
        let before = kept.folders.len();
        kept.keep(
            &format!("/{}", MAX_FOLDERS + 9),
            Listing::new(at(2), names(&["b"])),
            at(10),
        );
        assert_eq!(kept.folders.len(), before);
    }

    /// Sets the folder `dir`'s modification time.
    fn set_folder_time(dir: &Path, t: SystemTime) {
        #[cfg(windows)]
        let file = {
            use std::os::windows::fs::OpenOptionsExt;
            // FILE_WRITE_ATTRIBUTES, FILE_FLAG_BACKUP_SEMANTICS (a folder's handle).
            std::fs::OpenOptions::new()
                .access_mode(0x100)
                .custom_flags(0x0200_0000)
                .open(dir)
        };
        #[cfg(not(windows))]
        let file = std::fs::File::open(dir);
        file.unwrap().set_modified(t).unwrap();
    }

    /// This system's folders: a name added is found at the next look (the folder's time moves),
    /// and a change the time does not show is found after the port says it wrote there.
    #[test]
    fn a_real_folders_names_follow_its_changes() {
        let dir = std::env::temp_dir().join(format!("nv-rs-listings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let folder = dir.to_str().unwrap().to_string();
        std::fs::write(dir.join("Foo.txt"), b"").unwrap();
        let old = SystemTime::now() - Duration::from_secs(3600);
        set_folder_time(&dir, old);
        assert_eq!(
            folder_matches(&folder, "FOO.TXT"),
            Some(names(&["Foo.txt"]))
        );
        assert_eq!(folder_matches(&folder, "BAR.TXT"), Some(vec![]));
        // A name added: the folder's time moves, the names are read again.
        std::fs::write(dir.join("Bar.txt"), b"").unwrap();
        assert_eq!(
            folder_matches(&folder, "bar.txt"),
            Some(names(&["Bar.txt"]))
        );
        // A name added with the time put back (as within one tick of the clock): the kept names
        // answer, until the port says it wrote there.
        set_folder_time(&dir, old);
        assert_eq!(
            folder_matches(&folder, "FOO.TXT"),
            Some(names(&["Foo.txt"]))
        );
        std::fs::write(dir.join("Baz.txt"), b"").unwrap();
        set_folder_time(&dir, old);
        assert_eq!(folder_matches(&folder, "BAZ.TXT"), Some(vec![]));
        forget(&dir);
        assert_eq!(
            folder_matches(&folder, "BAZ.TXT"),
            Some(names(&["Baz.txt"]))
        );
        // Linux and macOS: the folder is kept as itself, so another spelling of it is forgotten
        // too. (The 2-second rule is tested with set times: a_listing_read_just_after_...)
        #[cfg(unix)]
        {
            std::fs::write(dir.join("Qux.txt"), b"").unwrap();
            set_folder_time(&dir, old);
            assert_eq!(folder_matches(&folder, "QUX.TXT"), Some(vec![]));
            forget(&dir.join("."));
            assert_eq!(
                folder_matches(&folder, "qux.TXT"),
                Some(names(&["Qux.txt"]))
            );
        }
        assert_eq!(
            folder_matches(&dir.join("none").to_string_lossy(), "x"),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A folder's stored name: the name itself when held so, else the first that matches.
    #[test]
    fn the_stored_name_is_the_name_itself_else_the_first() {
        let held = names(&["Armor.nif", "armor.nif"]);
        assert_eq!(stored_of(&held, "armor.nif").as_deref(), Some("armor.nif"));
        assert_eq!(stored_of(&held, "Armor.nif").as_deref(), Some("Armor.nif"));
        assert_eq!(stored_of(&held, "ARMOR.NIF").as_deref(), Some("Armor.nif"));
        assert_eq!(stored_of(&[], "x"), None);
    }

    /// A folder's two names for a part are said once per folder and name, whatever its case.
    #[test]
    fn two_names_are_said_once_per_folder_and_name() {
        let several = |folder: &str, part: &str| Several {
            folder: folder.to_string(),
            part: part.to_string(),
            names: names(&["Data", "data"]),
            taken: "Data".to_string(),
        };
        let first = several_lines(vec![several("/said-once", "DATA")]);
        assert_eq!(
            first,
            [
                r#"files: /said-once holds 2 names for the game's "DATA" (Data, data); "Data" is taken"#
            ]
        );
        assert!(several_lines(vec![several("/said-once", "data")]).is_empty());
        // By Windows' comparison, outside ASCII too.
        assert_eq!(
            several_lines(vec![several("/said-once", "ÄGYPTEN")]).len(),
            1
        );
        assert!(several_lines(vec![several("/said-once", "ägypten")]).is_empty());
        assert_eq!(several_lines(vec![several("/said-twice", "data")]).len(), 1);
    }

    #[test]
    fn the_stored_name_is_the_folders() {
        let dir = std::env::temp_dir().join(format!("nv-rs-stored-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Armor.NIF"), b"").unwrap();
        assert_eq!(
            stored_name(&dir.join("Armor.NIF")).as_deref(),
            Some("Armor.NIF")
        );
        assert_eq!(
            stored_name(&dir.join("armor.nif")).as_deref(),
            Some("Armor.NIF")
        );
        assert_eq!(stored_name(&dir.join("other.nif")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
