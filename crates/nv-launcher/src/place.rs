//! Where things are, for the start screen: nv-rs's own folder (the launcher's own file goes there),
//! the game remembered or found (only for the look and the status line: Play checks it), the play
//! profile's `Fallout.ini` (the renderer it chooses), whether there are mods to play with, and
//! whether nv-rs may write in its own folder at all. Windows: the package's `userdata`, Play's own
//! checks (`play::place`); Linux and macOS: nv-rs's own folder and the game as play.sh finds them
//! (`game_folders`, the finder `nv-viewer --find` answers play.sh with).

use std::path::{Path, PathBuf};

pub struct Place {
    /// nv-rs's own folder: Windows `userdata` in the package, Linux `~/.local/share/nv-rs`, macOS
    /// `~/Library/Application Support/nv-rs`.
    pub own: PathBuf,
    /// The game's folder: the one remembered, else the first a store names (no check run).
    pub game: Option<PathBuf>,
    /// The play profile's `Fallout.ini`.
    pub fallout_ini: PathBuf,
    /// Windows: the card the last start's adapter table names (the real Direct3D 9's card).
    pub adapter_card: Option<String>,
    /// Mods are installed, or put in the mods folder by hand.
    pub mods: bool,
    /// Why nv-rs must not write in its own folder, in the player's words, and Play.exe's stop
    /// code for it (`None`: it may).
    pub refused: Option<(String, i32)>,
}

/// The intro was shown at this start and is still to be noted: on Windows only once Play has
/// passed its own first-write checks with the game's folder (nothing is written into the package
/// before), elsewhere at once into nv-rs's own folder when that may be written.
static INTRO_TO_NOTE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The intro was shown (or skipped): noted at once ([`note_intro_shown`]) where `now` (Linux and
/// macOS, the own folder not refused), else kept until Play's checks pass
/// ([`note_intro_if_shown`]).
pub fn intro_seen(own: &Path, now: bool) -> std::io::Result<()> {
    if now {
        note_intro_shown(own)
    } else {
        INTRO_TO_NOTE.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

/// The intro is over at the start screen (the window's `end_intro`): nothing where nv-rs may not
/// write in its own folder (`refused`); else noted at once on Linux and macOS, and on Windows kept
/// for Play's first write (the package gets nothing before Play's own checks).
pub fn intro_ended(own: &Path, refused: bool) -> std::io::Result<()> {
    if refused {
        return Ok(());
    }
    intro_seen(own, cfg!(not(windows)))
}

/// Play's first write has passed its checks: the intro seen at this start is noted now.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn note_intro_if_shown(own: &Path) -> std::io::Result<()> {
    if INTRO_TO_NOTE.swap(false, std::sync::atomic::Ordering::Relaxed) {
        note_intro_shown(own)?;
    }
    Ok(())
}

/// Whether the mods folder `dir` holds anything but its READ ME.
pub fn has_mods(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            !e.file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .starts_with("read me")
        })
    })
}

/// The launcher's own file in nv-rs's own folder: what it remembers between starts.
pub fn launcher_ini(own: &Path) -> PathBuf {
    own.join("launcher.ini")
}

/// Whether the game's intro was shown (the launcher's own file says `bIntroShown=1`).
pub fn intro_shown(own: &Path) -> bool {
    std::fs::read(launcher_ini(own)).is_ok_and(|bytes| {
        let text: String = bytes.iter().map(|&b| b as char).collect();
        game_profile::settings::ini_value(&text, "Launcher", "bIntroShown")
            .is_some_and(|v| v.trim() == "1")
    })
}

/// Notes that the intro was shown, keeping the file's other lines. Never through a link, and only
/// into a folder nv-rs may write ([`Place::refused`] is checked by the caller).
pub fn note_intro_shown(own: &Path) -> std::io::Result<()> {
    let file = launcher_ini(own);
    if std::fs::symlink_metadata(&file).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(std::io::Error::other("the launcher's file is a link"));
    }
    let old: String = std::fs::read(&file)
        .unwrap_or_default()
        .iter()
        .map(|&b| b as char)
        .collect();
    let text = game_profile::game_ini::set_key(&old, "Launcher", "bIntroShown", "1");
    std::fs::create_dir_all(own)?;
    let part = own.join("launcher.ini.part");
    let _ = std::fs::remove_file(&part);
    std::fs::write(
        &part,
        text.chars().map(|c| c as u32 as u8).collect::<Vec<u8>>(),
    )?;
    std::fs::rename(&part, &file)
}

/// Linux and macOS: nv-rs's own folder and the game, as play.sh finds them.
#[cfg(not(windows))]
pub fn place() -> Place {
    match game_folders::Home::from_process() {
        Some(home) => place_from(&home, std::env::var_os("NV_RS_GAME").map(PathBuf::from)),
        None => Place {
            own: PathBuf::new(),
            game: None,
            fallout_ini: PathBuf::new(),
            adapter_card: None,
            mods: false,
            refused: Some((
                "HOME is not set to a full path, so nv-rs has no folder of its own".into(),
                1,
            )),
        },
    }
}

/// [`place`] for the home folder `home` (Linux's and macOS's rules; it runs on every system, so
/// its test does): the game named (`NV_RS_GAME`), then the folder play.sh remembers
/// (`game-folder.txt`), then the first copy a store names.
#[cfg_attr(windows, allow(dead_code))]
pub fn place_from(home: &game_folders::Home, named: Option<PathBuf>) -> Place {
    use game_folders as gf;
    let own = gf::own_folder(home);
    // play.sh stops with status 1 on the same refusal.
    let refused = gf::own_folder_refusal(home, &gf::never_written(home)).map(|why| (why, 1));
    let remembered = std::fs::read(own.join("game-folder.txt"))
        .ok()
        .map(|b| {
            String::from_utf8_lossy(&b)
                .trim_end_matches('\n')
                .to_string()
        })
        .map(PathBuf::from);
    let game = named
        .into_iter()
        .chain(remembered)
        .chain(gf::games(home).into_iter().map(|f| f.folder))
        .find_map(|folder| gf::game_folder(&folder));
    let fallout_ini = own
        .join("game")
        .join("Documents")
        .join("My Games")
        .join("FalloutNV")
        .join("Fallout.ini");
    let mods = has_mods(&own.join("my mods")) || has_mods(&own.join("mod installs"));
    Place {
        own,
        game,
        fallout_ini,
        adapter_card: None,
        mods,
        refused,
    }
}

#[cfg(windows)]
pub fn place() -> Place {
    crate::play::place()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The intro is noted in the launcher's own file, and its other lines stay.
    #[test]
    fn the_intro_is_noted_once_and_other_lines_stay() {
        let dir = std::env::temp_dir().join(format!("nv-launcher-ini-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!intro_shown(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(launcher_ini(&dir), "[Launcher]\r\nsOther=x\r\n").unwrap();
        assert!(!intro_shown(&dir));
        note_intro_shown(&dir).unwrap();
        assert!(intro_shown(&dir));
        let text = std::fs::read_to_string(launcher_ini(&dir)).unwrap();
        assert!(text.contains("sOther=x"), "{text}");
        assert!(!dir.join("launcher.ini.part").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Linux and macOS: the game play.sh remembers in nv-rs's own folder, the play profile's
    /// Fallout.ini there, and the mods folders there ("my mods", "mod installs").
    #[test]
    fn linux_and_macos_read_nv_rs_own_folder() {
        use game_folders::{Home, System};
        let dir = std::env::temp_dir().join(format!("nv-launcher-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let home = Home::new(dir.join("home"), None, None, System::Linux).unwrap();
        let own = dir.join("home/.local/share/nv-rs");
        let game = dir.join("games/Fallout New Vegas");
        std::fs::create_dir_all(game.join("Data")).unwrap();
        std::fs::write(game.join("Data/FalloutNV.esm"), b"x").unwrap();
        std::fs::create_dir_all(own.join("my mods")).unwrap();
        std::fs::write(own.join("game-folder.txt"), format!("{}\n", game.display())).unwrap();
        let p = place_from(&home, None);
        assert_eq!(p.own, own);
        assert_eq!(p.game.as_deref(), Some(game.as_path()));
        assert_eq!(
            p.fallout_ini,
            own.join("game/Documents/My Games/FalloutNV/Fallout.ini")
        );
        assert!(!p.mods);
        assert_eq!(p.adapter_card, None);
        std::fs::write(own.join("my mods/a.esp"), b"x").unwrap();
        assert!(place_from(&home, None).mods);
        // A game named by NV_RS_GAME comes first; a folder without the game is passed over.
        let p = place_from(&home, Some(dir.join("no game here")));
        assert_eq!(p.game.as_deref(), Some(game.as_path()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A mods folder with only its READ ME has nothing to play with.
    #[test]
    fn a_mods_folder_with_only_its_read_me_is_empty() {
        let dir = std::env::temp_dir().join(format!("nv-launcher-mods-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!has_mods(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("READ ME.txt"), "x").unwrap();
        assert!(!has_mods(&dir));
        std::fs::write(dir.join("a.esp"), "x").unwrap();
        assert!(has_mods(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
