//! Where mods' own files are kept between sessions (the maintainer, 2026-11-16): a folder of the
//! port's own beside the saves, one per profile, never the game's folder
//! (`engine_platform::mod_files`, which the built-in extenders' file commands use).
//!
//! `NV_MOD_FILES_OUT=DIR` names it; without it, `nv-rs mod files` beside `NV_SAVE_OUT`'s folder
//! (the launcher's `userdata\<game|mods>\Documents\My Games\FalloutNV\Saves` gives
//! `...\My Games\FalloutNV\nv-rs mod files`). With neither, there is none: a mod's writes last
//! for the session. A folder inside the player's Documents or AppData ([`save_out::refused`]) or
//! inside the game's folder is refused, with a line, and none is used.

use crate::save_out;
use std::path::{Path, PathBuf};

/// The name of the folder beside the saves.
pub const BESIDE_SAVES: &str = "nv-rs mod files";

/// The folder `NV_MOD_FILES_OUT` (`mod_out`) or `NV_SAVE_OUT` (`save_out`) gives, before any
/// check: the first as it is, else the second's parent with [`BESIDE_SAVES`].
pub fn chosen(mod_out: Option<PathBuf>, save_out: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(dir) = mod_out.filter(|d| !d.as_os_str().is_empty()) {
        return Some(dir);
    }
    let saves = save_out.filter(|d| !d.as_os_str().is_empty())?;
    let beside = saves.parent().unwrap_or(&saves).join(BESIDE_SAVES);
    Some(beside)
}

/// Why `dir` may not keep mods' files: the save folder's refusals ([`save_out::refused`]), or it
/// is the game's folder `game` or inside it (compared without case, `/` as `\`).
pub fn refused(
    dir: &Path,
    game: &Path,
    profile: Option<&Path>,
    known: &[PathBuf],
) -> Option<&'static str> {
    if let Some(why) = save_out::refused(dir, profile, known) {
        return Some(why);
    }
    let norm = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    };
    let (dir, game) = (norm(dir), norm(game));
    if !game.is_empty() && (dir == game || dir.starts_with(&format!("{game}\\"))) {
        return Some("inside the game's folder");
    }
    None
}

/// The folder for this run (the environment, [`chosen`], checked by [`refused`]; a refusal said
/// once), for the game whose folder is `game`.
pub fn folder(game: &Path) -> Option<PathBuf> {
    let dir = chosen(
        std::env::var_os("NV_MOD_FILES_OUT").map(PathBuf::from),
        std::env::var_os("NV_SAVE_OUT").map(PathBuf::from),
    )?;
    let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
    if let Some(why) = refused(&dir, game, profile.as_deref(), &save_out::known_folders()) {
        println!(
            "mod files: {} refused ({why}); mods' own files last for this session only.",
            dir.display()
        );
        return None;
    }
    println!("mod files: mods' own files are kept in {}", dir.display());
    Some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_is_named_or_beside_the_saves() {
        let saves = PathBuf::from(r"C:\nv\userdata\mods\Documents\My Games\FalloutNV\Saves");
        assert_eq!(
            chosen(None, Some(saves.clone())).unwrap(),
            PathBuf::from(r"C:\nv\userdata\mods\Documents\My Games\FalloutNV\nv-rs mod files")
        );
        assert_eq!(
            chosen(Some(PathBuf::from(r"D:\keep")), Some(saves)).unwrap(),
            PathBuf::from(r"D:\keep")
        );
        assert_eq!(chosen(None, None), None);
        assert_eq!(chosen(Some(PathBuf::new()), None), None);
    }

    /// The player's folders and the game's folder are refused; the package's and scratch
    /// folders are taken.
    #[test]
    fn the_players_folders_and_the_game_folder_are_refused() {
        let profile = Path::new(r"C:\Users\<name>");
        let game = Path::new(r"C:\Games\Steam\steamapps\common\Fallout New Vegas");
        for dir in [
            r"C:\Users\<name>\Documents\My Games\FalloutNV\nv-rs mod files",
            r"C:\Users\<name>\AppData\Local\x",
            r"c:/games/steam/steamapps/common/fallout new vegas/Data/Config",
            r"C:\Games\Steam\steamapps\common\Fallout New Vegas",
            r"relative\folder",
        ] {
            assert!(
                refused(Path::new(dir), game, Some(profile), &[]).is_some(),
                "{dir}"
            );
        }
        for dir in [
            r"C:\nv\userdata\mods\Documents\My Games\FalloutNV\nv-rs mod files",
            r"C:\Users\<name>\OneDrive\Desktop\nv-rs-play-new\docs-mods\nv-rs mod files",
            r"C:\Games\Steam\steamapps\common\Fallout New Vegas Other",
        ] {
            assert_eq!(
                refused(Path::new(dir), game, Some(profile), &[]),
                None,
                "{dir}"
            );
        }
    }
}
