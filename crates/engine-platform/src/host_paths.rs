//! The game's Windows paths over this system's folders on Linux and macOS (decision 3 of the
//! Linux and macOS track, the Wine way).
//!
//! The game sees this system's folders on one drive, `Z:` (`engine_core::exe::game_view`): it
//! is started in `Z:\home\me\Games\Fallout New Vegas`, and every path it builds from there is a
//! Windows path, `\` between the parts, in whatever case the game, its INI files or a mod wrote
//! the names. Windows finds a file whatever the case of its name; Linux's file systems (and macOS's
//! when formatted case-sensitive) do not. So a path the file calls are given becomes this system's
//! path here ([`host_path`]):
//! - `Z:` is taken off, and each `\` becomes `/`. Another drive letter has no folder here: it
//!   stops the run, naming the drive.
//! - Each part is then looked up in its folder as Windows compares names: without case, by
//!   NTFS's upper-casing (`super::files`' `ntfs_key`). A part whose name is there as written is
//!   taken as it is; otherwise the folder's names that match it without case are put in byte
//!   order and the first is taken. When a folder holds more than one name for the part (two
//!   names that differ only in case, which Windows cannot hold in one folder), the log says
//!   which was taken, once per folder and name.
//! - `.` and `..` are taken away as Windows takes them, by the text of the path (a `..` goes up
//!   from the part before it, links or not).
//! - From the first part that is in no form in its folder, the rest is kept as written: the file
//!   is not there, and the call answers as for any missing file.
//!
//! A path with no drive (one the host gave: a test's scratch folder, `/tmp/...`) is treated the
//! same from its `/` on; a relative one is left relative, `\` as `/`.
//!
//! The folders' names are kept between calls ([`Listings`]) and read again when a folder's
//! modification time changes (a name added, removed or renamed changes it on these systems).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

/// What stops a path from being one of this system's.
#[derive(Debug, PartialEq, Eq)]
pub enum NotHere {
    /// A drive other than `Z:`: no folder of this system is on it.
    OtherDrive(char),
    /// A drive with no `\` after it (`Z:x`, relative to that drive's current folder): the game's
    /// own calls are given full paths (`super::files::full_path`), so this is not provided.
    DriveRelative,
}

/// The game's path `game` (its text, `\` or `/` between parts) as this system's path, before the
/// names' case is looked up: `Z:` taken off, `\` as `/`.
pub fn unix_text(game: &str) -> Result<String, NotHere> {
    let b = game.as_bytes();
    let rest = if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        if !b[0].eq_ignore_ascii_case(&engine_core::exe::HOST_DRIVE) {
            return Err(NotHere::OtherDrive(b[0].to_ascii_uppercase() as char));
        }
        let rest = &game[2..];
        if !rest.starts_with(['\\', '/']) {
            return Err(NotHere::DriveRelative);
        }
        rest
    } else {
        game
    };
    Ok(rest.replace('\\', "/"))
}

/// Whether two names are the same name to Windows: equal after NTFS's upper-casing.
pub fn same_name(a: &str, b: &str) -> bool {
    let fold = |s: &str| super::files::ntfs_key(&s.encode_utf16().collect::<Vec<u16>>());
    fold(a) == fold(b)
}

/// The path `path` (absolute, `/` between parts) with each part's name as its folder holds it.
/// `exists` says whether a path is there as written; `names` lists a folder's names (`None` for
/// no such folder). The second value: the folders and names where more than one name matched
/// (for the log).
pub fn resolve(
    path: &str,
    exists: &mut dyn FnMut(&str) -> bool,
    names: &mut dyn FnMut(&str) -> Option<Vec<String>>,
) -> (String, Vec<(String, String, Vec<String>)>) {
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
    let mut out = String::new();
    let mut several = Vec::new();
    let mut lost = false;
    for part in parts {
        let folder = if out.is_empty() { "/" } else { out.as_str() }.to_string();
        let mut next = format!("{out}/{part}");
        if !lost && !exists(&next) {
            match names(&folder) {
                Some(listed) => {
                    let mut matching: Vec<String> =
                        listed.into_iter().filter(|n| same_name(n, part)).collect();
                    matching.sort();
                    match matching.first() {
                        Some(first) => {
                            next = format!("{out}/{first}");
                            if matching.len() > 1 {
                                several.push((folder, part.to_string(), matching.clone()));
                            }
                        }
                        None => lost = true,
                    }
                }
                None => lost = true,
            }
        }
        out = next;
    }
    if out.is_empty() {
        out.push('/');
    }
    (out, several)
}

/// The folders' names read so far, each with the folder's modification time when read.
type Listings = HashMap<String, (std::time::SystemTime, Vec<String>)>;

/// The names of the folder `folder` on this system (`None` if there is no such folder), from
/// [`LISTINGS`] while its modification time is the one read with them.
fn folder_names(folder: &str) -> Option<Vec<String>> {
    static LISTINGS: Mutex<Option<Listings>> = Mutex::new(None);
    let modified = std::fs::metadata(folder).ok()?.modified().ok()?;
    let mut kept = LISTINGS.lock().unwrap_or_else(|p| p.into_inner());
    let kept = kept.get_or_insert_with(HashMap::new);
    if let Some((when, names)) = kept.get(folder) {
        if *when == modified {
            return Some(names.clone());
        }
    }
    let names: Vec<String> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    kept.insert(folder.to_string(), (modified, names.clone()));
    Some(names)
}

/// The game's path `game` as this system's path ([`unix_text`], then [`resolve`] against this
/// system's folders, a folder holding two names for one part said once in the log). A drive
/// other than `Z:` and a drive-relative path stop the run, as a call not provided.
pub fn host_path(game: &str) -> PathBuf {
    let text = match unix_text(game) {
        Ok(text) => text,
        Err(NotHere::OtherDrive(d)) => panic!(
            "the game's path {game:?}: drive {d}: is not provided off Windows (this system's \
             folders are the game's drive {}:)",
            engine_core::exe::HOST_DRIVE as char
        ),
        Err(NotHere::DriveRelative) => {
            panic!("the game's path {game:?}: a path relative to a drive is not provided")
        }
    };
    if !text.starts_with('/') {
        return PathBuf::from(text);
    }
    let (found, several) = resolve(
        &text,
        &mut |p| std::fs::symlink_metadata(p).is_ok(),
        &mut folder_names,
    );
    if !several.is_empty() {
        static SAID: Mutex<Option<HashSet<(String, String)>>> = Mutex::new(None);
        let mut said = SAID.lock().unwrap_or_else(|p| p.into_inner());
        let said = said.get_or_insert_with(HashSet::new);
        for (folder, part, names) in several {
            if said.insert((folder.clone(), part.to_ascii_lowercase())) {
                eprintln!(
                    "files: {folder} holds {} names for the game's {part:?} ({}); the first, \
                     {:?}, is taken",
                    names.len(),
                    names.join(", "),
                    names[0]
                );
            }
        }
    }
    PathBuf::from(found)
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
            ("/home/me/Fallout New Vegas/Data", vec!["Meshes", "Fallout.esm"]),
            ("/home/me/Fallout New Vegas/data", vec!["readme.txt"]),
            ("/home/me/Fallout New Vegas/Data/Meshes", vec!["Armor", "Ägypten"]),
        ])
    }

    /// [`resolve`] over [`tree`]: whether a path is there as written, and a folder's names.
    fn resolved(path: &str) -> (String, Vec<(String, String, Vec<String>)>) {
        let t = tree();
        let listed = |p: &str| -> Option<Vec<String>> {
            t.get(p).map(|v| v.iter().map(|s| s.to_string()).collect())
        };
        let exists = |p: &str| -> bool {
            let (folder, name) = p.rsplit_once('/').unwrap();
            let folder = if folder.is_empty() { "/" } else { folder };
            listed(folder).is_some_and(|n| n.iter().any(|x| x == name))
        };
        resolve(path, &mut { exists }, &mut { listed })
    }

    #[test]
    fn drive_z_is_this_systems_root_and_other_drives_are_not_here() {
        assert_eq!(
            unix_text(r"Z:\home\me\Fallout New Vegas\Data\Fallout.esm").unwrap(),
            "/home/me/Fallout New Vegas/Data/Fallout.esm"
        );
        assert_eq!(unix_text(r"z:/home\me").unwrap(), "/home/me");
        assert_eq!(unix_text(r"Z:\").unwrap(), "/");
        assert_eq!(unix_text(r"C:\Games\x").unwrap_err(), NotHere::OtherDrive('C'));
        assert_eq!(unix_text(r"d:\x").unwrap_err(), NotHere::OtherDrive('D'));
        assert_eq!(unix_text(r"Z:Data\x").unwrap_err(), NotHere::DriveRelative);
        assert_eq!(unix_text(r"/tmp/scratch\a.txt").unwrap(), "/tmp/scratch/a.txt");
        assert_eq!(unix_text(r"Data\Meshes").unwrap(), "Data/Meshes");
    }

    #[test]
    fn the_game_view_and_back_is_the_same_folder() {
        let host = "/home/me/Fallout New Vegas";
        assert_eq!(unix_text(&engine_core::exe::game_view(host)).unwrap(), host);
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

    #[test]
    fn a_name_written_as_it_is_is_taken_as_it_is() {
        let (path, several) = resolved("/home/me/Fallout New Vegas/data/readme.txt");
        assert_eq!(path, "/home/me/Fallout New Vegas/data/readme.txt");
        assert!(several.is_empty());
    }

    #[test]
    fn two_names_for_one_part_take_the_first_in_byte_order_and_say_so() {
        let (path, several) = resolved("/home/me/Fallout New Vegas/DATA/Fallout.esm");
        assert_eq!(path, "/home/me/Fallout New Vegas/Data/Fallout.esm");
        assert_eq!(
            several,
            [(
                "/home/me/Fallout New Vegas".to_string(),
                "DATA".to_string(),
                vec!["Data".to_string(), "data".to_string()]
            )]
        );
    }

    #[test]
    fn from_a_missing_part_on_the_rest_is_kept_as_written() {
        assert_eq!(
            resolved("/home/me/fallout new vegas/Data/Textures/Armor/x.dds").0,
            "/home/me/Fallout New Vegas/Data/Textures/Armor/x.dds"
        );
        assert_eq!(resolved("/nowhere/Data").0, "/nowhere/Data");
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
    }

    #[test]
    fn same_name_is_windows_comparison() {
        assert!(same_name("Fallout.ESM", "fallout.esm"));
        assert!(same_name("Ägypten", "äGYPTEN"));
        assert!(!same_name("lodz.txt", "\u{142}odz.txt"));
        assert!(!same_name("a.txt", "a.txt "));
    }
}
