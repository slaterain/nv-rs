//! Where mods' own files are kept between sessions (the maintainer, 2026-11-16): a folder of the
//! port's own beside the saves, one per profile, never the game's folder
//! (`engine_platform::mod_files`, which the built-in extenders' file commands use).
//!
//! `NV_MOD_FILES_OUT=DIR` names it; without it, `nv-rs mod files` beside `NV_SAVE_OUT`'s folder
//! (the launcher's `userdata\<game|mods>\Documents\My Games\FalloutNV\Saves` gives
//! `...\My Games\FalloutNV\nv-rs mod files`). With neither, there is none: mods' settings last
//! for the session only, and the start-up log says so in one line. A folder inside the player's
//! Documents or AppData ([`save_out::refused`]) or inside the game's folder, by its text or by the
//! real path it leads to (junctions, `subst`, short names resolved), is refused, with a line, and
//! none is used; so is any folder when the game's folder cannot be told.

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

/// The game's folder for the Data folder `data`: its parent, from the full path (a relative
/// `data` is taken from the process's current folder, as the run opens it).
pub fn game_folder(data: &Path) -> PathBuf {
    let full = if data.is_absolute() {
        data.to_path_buf()
    } else {
        std::env::current_dir().map_or(data.to_path_buf(), |dir| dir.join(data))
    };
    full.parent().map_or(full.clone(), Path::to_path_buf)
}

/// Why `dir` may not keep mods' files: the save folder's refusals ([`save_out::refused`]), or it
/// is the game's folder `game` or inside it, by text (without case, `/` as `\`) or by the real
/// paths both lead to; or the game's folder is not a full path (it cannot be told).
pub fn refused(
    dir: &Path,
    game: &Path,
    profile: Option<&Path>,
    known: &[PathBuf],
) -> Option<&'static str> {
    if let Some(why) = save_out::refused(dir, profile, known) {
        return Some(why);
    }
    if !game.is_absolute() {
        return Some("the game's folder is not known");
    }
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
    let games = [norm(game), norm(&save_out::real_path(game))];
    let dirs = [norm(dir), norm(&save_out::real_path(dir))];
    for game in &games {
        for dir in &dirs {
            if !game.is_empty() && (dir == game || dir.starts_with(&format!("{game}\\"))) {
                return Some("inside the game's folder");
            }
        }
    }
    None
}

/// What [`folder_from`] decided, and the one log line that says so.
#[derive(Debug, PartialEq, Eq)]
pub struct Decision {
    pub folder: Option<PathBuf>,
    pub line: String,
}

/// The folder for a run with these variables (`NV_MOD_FILES_OUT`, `NV_SAVE_OUT`), this game
/// folder, the player's profile folder and Windows' known folders: [`chosen`], checked by
/// [`refused`]. A pure function: [`folder`] gives it the environment.
pub fn folder_from(
    mod_out: Option<PathBuf>,
    save_out: Option<PathBuf>,
    game: &Path,
    profile: Option<&Path>,
    known: &[PathBuf],
) -> Decision {
    let Some(dir) = chosen(mod_out, save_out) else {
        return Decision {
            folder: None,
            line: "mod files: no folder for mods' own files (NV_MOD_FILES_OUT and NV_SAVE_OUT are not set); their settings last for this session only.".to_string(),
        };
    };
    if let Some(why) = refused(&dir, game, profile, known) {
        return Decision {
            folder: None,
            line: format!(
                "mod files: {} refused ({why}); mods' own settings last for this session only.",
                dir.display()
            ),
        };
    }
    Decision {
        line: format!("mod files: mods' own files are kept in {}", dir.display()),
        folder: Some(dir),
    }
}

/// The folder for this run, for the game whose Data folder is `data` ([`game_folder`],
/// [`folder_from`] with the environment); its line printed.
pub fn folder(data: &Path) -> Option<PathBuf> {
    let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
    let decision = folder_from(
        std::env::var_os("NV_MOD_FILES_OUT").map(PathBuf::from),
        std::env::var_os("NV_SAVE_OUT").map(PathBuf::from),
        &game_folder(data),
        profile.as_deref(),
        &save_out::known_folders(),
    );
    println!("{}", decision.line);
    decision.folder
}

#[cfg(test)]
mod tests {
    use super::*;

    // Windows only: it builds the game's names with backslashes and drive letters, which the platform's
    // file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
    // as part of a name elsewhere; starting the game there is refused today).
    #[cfg(windows)]
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
    // Windows only: it builds the game's names with backslashes and drive letters, which the platform's
    // file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
    // as part of a name elsewhere; starting the game there is refused today).
    #[cfg(windows)]
    #[test]
    fn the_players_folders_and_the_game_folder_are_refused() {
        let profile = Path::new(r"C:\Users\<name>");
        let game = Path::new(r"C:\Games\Steam\steamapps\common\Fallout New Vegas");
        for dir in [
            r"C:\Users\<name>\Documents\My Games\FalloutNV\nv-rs mod files",
            r"C:\Users\<name>\AppData\Local\x",
            r"c:/games/steam/steamapps/common/fallout new vegas/Data/Config",
            r"C:\Games\Steam\steamapps\common\Fallout New Vegas",
            r"\\?\C:\Games\Steam\steamapps\common\Fallout New Vegas\x",
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
        // A game folder that is not a full path: nothing is taken.
        assert!(refused(Path::new(r"D:\keep"), Path::new(""), Some(profile), &[]).is_some());
    }

    /// A scratch folder removed when the test ends, however it ends.
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir(self.0.join("link"));
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The game's folder by another spelling (the real path's `\\?\` form) and through a
    /// junction is refused as the game's folder.
    #[cfg(windows)]
    #[test]
    fn another_spelling_of_the_game_folder_is_refused() {
        let base =
            Scratch(std::env::temp_dir().join(format!("nv-mod-files-host-{}", std::process::id())));
        let game = base.0.join("Fallout New Vegas");
        std::fs::create_dir_all(game.join("Data")).unwrap();
        let profile = Path::new(r"C:\Users\<name>");
        let real = std::fs::canonicalize(&game).unwrap();
        assert!(refused(&real.join("x"), &game, Some(profile), &[]).is_some());
        let link = base.0.join("link");
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&game)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
        assert!(refused(&link.join("mods"), &game, Some(profile), &[]).is_some());
        assert_eq!(
            refused(&base.0.join("elsewhere"), &game, Some(profile), &[]),
            None
        );
    }

    /// [`folder_from`]: refused folders give none (with the line saying so), as does no variable;
    /// a good folder is taken.
    // Windows only: it builds the game's names with backslashes and drive letters, which the platform's
    // file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
    // as part of a name elsewhere; starting the game there is refused today).
    #[cfg(windows)]
    #[test]
    fn folder_from_answers_none_for_refused_folders() {
        let profile = Path::new(r"C:\Users\<name>");
        let game = Path::new(r"C:\Games\FNV");
        let known = [PathBuf::from(r"D:\Moved\Documents")];
        for dir in [
            r"C:\Users\<name>\Documents\x",
            r"C:\Users\<name>\AppData\Roaming\x",
            r"D:\Moved\Documents\x",
            r"C:\Games\FNV",
            r"C:\Games\FNV\Data\Config",
        ] {
            let d = folder_from(Some(PathBuf::from(dir)), None, game, Some(profile), &known);
            assert_eq!(d.folder, None, "{dir}");
            assert!(d.line.contains("refused"), "{}", d.line);
        }
        let d = folder_from(None, None, game, Some(profile), &known);
        assert_eq!(d.folder, None);
        assert!(d.line.contains("this session only"));
        let d = folder_from(
            None,
            Some(PathBuf::from(
                r"D:\nv\userdata\game\Documents\My Games\FalloutNV\Saves",
            )),
            game,
            Some(profile),
            &known,
        );
        assert_eq!(
            d.folder,
            Some(PathBuf::from(
                r"D:\nv\userdata\game\Documents\My Games\FalloutNV\nv-rs mod files"
            ))
        );
    }

    /// The game's start-up INI set-up (`00876ad0`, translated) with a game folder whose
    /// `FALLOUT.INI` says `bUseMyGamesDirectory=0`: through the platform's
    /// `GetPrivateProfileIntA` (which reads `.\FALLOUT.INI` from the game's folder, as Windows
    /// does) the key answers 1, so the game takes its My Games branch: its INI and save folders
    /// are the Documents ones, not `.\` (the game's folder).
    #[test]
    fn the_start_up_keeps_the_my_games_folders() {
        use engine::{Engine, Ptr, Ret};
        let base = Scratch(
            std::env::temp_dir().join(format!("nv-mod-files-startup-{}", std::process::id())),
        );
        std::fs::create_dir_all(&base.0).unwrap();
        std::fs::write(
            base.0.join("FALLOUT.INI"),
            b"[General]\r\nbUseMyGamesDirectory=0\r\n",
        )
        .unwrap();
        std::fs::write(
            base.0.join("Custom.ini"),
            b"[General]\r\nbUseMyGamesDirectory=0\r\n",
        )
        .unwrap();
        let mut e = Engine::new();
        e.kernel.current_directory = base.0.to_string_lossy().replace('/', "\\").into_bytes();
        // The exe's strings and globals the function reads.
        e.map(0x0105_d000, 0x1000);
        e.map(0x0108_2000, 0x1000);
        e.map(0x011a_2000, 0x1000);
        e.map(0x0300_0000, 0x1000);
        e.mem.set_cstr(0x0105_d548, b"General");
        e.mem.set_cstr(0x0108_2dac, b"bUseMyGamesDirectory");
        e.mem.set_cstr(0x0108_2dc4, b".\\%s");
        e.mem.set_cstr(0x0108_2d84, b".\\");
        e.mem.set_cstr(0x0108_2da0, b"\\My Games");
        e.mem.set_cstr(0x0108_2d94, b"\\My Games");
        e.mem.set_cstr(0x0108_2d88, b"\\FalloutNV\\");
        e.mem.set_cstr(0x0108_2d70, b"Fallout_default.ini");
        e.mem.set_cstr(0x0300_0800, b"Custom.ini");
        e.mem.set_cstr(0x0300_0810, b"FALLOUT.INI");
        e.mem.set_u32(0x011a_2ff4, 0x0300_0800);
        e.mem.set_u32(0x011a_2ff0, 0x0300_0810);
        // The two folder buffers (`0086d480`, `004dc110`) and the helpers around the reads.
        const INI_DIR: u32 = 0x0300_0100;
        const SAVE_DIR: u32 = 0x0300_0300;
        e.register(0x0086_d480, |_, _| Ret {
            eax: INI_DIR,
            ..Ret::default()
        });
        e.register(0x004d_c110, |_, _| Ret {
            eax: SAVE_DIR,
            ..Ret::default()
        });
        e.register(0x0040_6d00, |e, a| {
            let name = e.mem.cstr(a[3]);
            e.mem.set_cstr(a[0], &[b".\\".as_slice(), &name].concat());
            Ret::default()
        });
        e.register(0x0040_6d30, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(0x0040_6d50, |e, a| {
            let text = [e.mem.cstr(a[0]), e.mem.cstr(a[2])].concat();
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(0x00fd_f274, |e, a| {
            e.mem.set_cstr(a[4], br"C:\Docs");
            Ret::default()
        });
        for (addr, eax) in [
            (0x00fd_f0b8u32, 1u32),
            (0x00fd_f0d4, 1),
            (0x00fd_f1b8, 1),
            (0x00ec_bf05, 0),
            (0x00c3_bb80, 0x0300_0800),
            (0x00c3_bbd0, 0x0300_0800),
            (0x00c3_bc20, 0x0300_0800),
        ] {
            let f: engine::AbiFn = match eax {
                0 => |_, _| Ret::default(),
                1 => |_, _| Ret {
                    eax: 1,
                    ..Ret::default()
                },
                _ => |_, _| Ret {
                    eax: 0x0300_0800,
                    ..Ret::default()
                },
            };
            e.register(addr, f);
        }
        e.register(0x00fd_f0c4, |e, a| Ret {
            eax: engine_platform::ini::get_private_profile_int_a(
                e,
                Ptr::new(a[0]),
                Ptr::new(a[1]),
                a[2],
                Ptr::new(a[3]),
            ),
            ..Ret::default()
        });
        let buffer = 0x0300_0500;
        e.call(0x0087_6ad0, &[buffer]);
        assert_eq!(e.mem.cstr(INI_DIR), br"C:\Docs\My Games");
        assert_eq!(e.mem.cstr(SAVE_DIR), br"C:\Docs\My Games\FalloutNV\");
        assert_ne!(e.mem.cstr(SAVE_DIR), b".\\");
    }

    /// The host gives the game's folder as Data's parent, from the full path.
    // Windows only: it builds the game's names with backslashes and drive letters, which the platform's
    // file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
    // as part of a name elsewhere; starting the game there is refused today).
    #[cfg(windows)]
    #[test]
    fn the_game_folder_is_datas_parent() {
        assert_eq!(
            game_folder(Path::new(r"C:\Games\FNV\Data")),
            PathBuf::from(r"C:\Games\FNV")
        );
        let relative = game_folder(Path::new("Data"));
        assert!(relative.is_absolute());
        assert_eq!(relative, std::env::current_dir().unwrap());
    }
}
