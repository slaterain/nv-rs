//! Made-up folder trees in the system's temporary folder: no store, game or player folder of this
//! machine is read.

use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh empty folder for one test, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "nv-game-folders-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn home(&self, system: System) -> Home {
        Home::new(self.0.join("home"), None, None, system).unwrap()
    }

    /// `rel` (a path with `/`) made, with its parents, as a file holding `text`.
    fn file(&self, rel: &str, text: &str) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn dir(&self, rel: &str) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// A made-up game at `rel` (its `Data/FalloutNV.esm`, the names in the case given).
    fn game(&self, rel: &str, data: &str, master: &str) -> PathBuf {
        self.file(&format!("{rel}/{data}/{master}"), "not the game's file");
        self.0.join(rel)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn vdf_library(path: &Path) -> String {
    format!(
        "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
        path.display().to_string().replace('\\', "\\\\")
    )
}

#[test]
fn steam_library_folders_and_manifests_are_read() {
    let text = r#"
"libraryfolders"
{
	"0" { "path"		"/x/Steam" "apps" { "22380" "123" } }
	"1" { "path"		"/mnt/Games \"Steam\"" }
}"#;
    assert_eq!(
        vdf_values(text, "path"),
        ["/x/Steam", "/mnt/Games \"Steam\""]
    );
    let manifest =
        "\"AppState\"\n{\n\t\"appid\"\t\t\"22380\"\n\t\"installdir\"\t\t\"Fallout New Vegas\"\n}\n";
    assert_eq!(vdf_values(manifest, "InstallDir"), ["Fallout New Vegas"]);
}

#[test]
fn json_and_yaml_values_are_read() {
    let json = r#"{"installed":[{"appName":"1","install_path":"/g/Fallout New Vegas","x":2},
        {"install_path" : "/g/other \"q\" \\ \/end"}, {"install_path": 3}]}"#;
    assert_eq!(
        json_strings(json, "install_path"),
        ["/g/Fallout New Vegas", "/g/other \"q\" \\ /end"]
    );
    assert!(json_strings("{\"install_path\": \"cut", "install_path").is_empty());
    let yaml = "game:\n  exe: /g/FNV/FalloutNV.exe\n  prefix: '/p/fnv'\n  game_path: \"/g/FNV\"\n  exe2: x\n  exe:\n";
    assert_eq!(yaml_values(yaml, "exe"), ["/g/FNV/FalloutNV.exe"]);
    assert_eq!(yaml_values(yaml, "prefix"), ["/p/fnv"]);
    assert_eq!(yaml_values(yaml, "game_path"), ["/g/FNV"]);
}

#[test]
fn a_home_must_be_a_full_path_and_relative_xdg_folders_are_ignored() {
    assert!(Home::new("home".into(), None, None, System::Linux).is_none());
    let s = Scratch::new("home");
    let home = Home::new(
        s.0.join("home"),
        Some("relative".into()),
        Some(s.0.join("cfg")),
        System::Linux,
    )
    .unwrap();
    assert_eq!(home.data_home, None);
    assert_eq!(home.config_home, Some(s.0.join("cfg")));
}

#[test]
fn the_own_folder_follows_xdg_on_linux_and_library_on_macos() {
    let s = Scratch::new("own");
    let home = s.0.join("home");
    let linux = Home::new(home.clone(), None, None, System::Linux).unwrap();
    assert_eq!(own_folder(&linux), home.join(".local/share").join("nv-rs"));
    let moved = Home::new(home.clone(), Some(s.0.join("data")), None, System::Linux).unwrap();
    assert_eq!(own_folder(&moved), s.0.join("data").join("nv-rs"));
    let mac = Home::new(home.clone(), Some(s.0.join("data")), None, System::MacOs).unwrap();
    assert_eq!(
        own_folder(&mac),
        home.join("Library")
            .join("Application Support")
            .join("nv-rs")
    );
}

#[test]
fn names_are_matched_without_case() {
    let s = Scratch::new("case");
    let game = s.game("Games/FNV", "data", "falloutnv.ESM");
    assert_eq!(data_folder(&game), Some(game.join("data")));
    assert_eq!(data_folder(&game.join("data")), Some(game.join("data")));
    assert_eq!(game_folder(&game.join("data")), Some(game.clone()));
    assert_eq!(game_folder(&game), Some(game.clone()));
    assert_eq!(
        resolve(&s.0, &["games", "fnv", "DATA"]),
        Some(game.join("data"))
    );
    // A folder without the master is not the game; nor is a folder named like it.
    s.dir("Games/Empty/Data");
    s.dir("Games/Dir/Data/FalloutNV.esm");
    assert_eq!(data_folder(&s.0.join("Games/Empty")), None);
    assert_eq!(data_folder(&s.0.join("Games/Dir")), None);
    assert_eq!(game_folder(&s.0.join("missing")), None);
}

#[test]
fn steam_finds_the_game_in_any_library_and_the_proton_saves() {
    let s = Scratch::new("steam");
    let home = s.home(System::Linux);
    // Steam itself in ~/.local/share/Steam, a second library elsewhere with the game (names in
    // another case) and its manifest, Proton's prefix with saves.
    let lib2 = s.dir("mnt/games/SteamLibrary");
    s.dir("mnt/games/SteamLibrary/steamapps");
    s.file(
        "home/.local/share/Steam/steamapps/libraryfolders.vdf",
        &vdf_library(&lib2),
    );
    let game = s.game(
        "mnt/games/SteamLibrary/steamapps/common/fallout new vegas",
        "Data",
        "FalloutNV.esm",
    );
    s.file(
        "mnt/games/SteamLibrary/steamapps/appmanifest_22380.acf",
        "\"AppState\" { \"installdir\" \"Fallout New Vegas\" }",
    );
    let saves = s.dir(
        "mnt/games/SteamLibrary/steamapps/compatdata/22380/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV/Saves",
    );
    s.dir("mnt/games/SteamLibrary/steamapps/compatdata/22380/pfx/drive_c/users/Public/Documents/My Games/FalloutNV");
    assert_eq!(steam_roots(&home), [s.0.join("home/.local/share/Steam")]);
    assert_eq!(
        steam_libraries(&home),
        [s.0.join("home/.local/share/Steam"), lib2.clone()]
    );
    assert_eq!(
        games(&home),
        [Found {
            store: "Steam",
            folder: game.clone()
        }]
    );
    assert_eq!(
        save_folders(&home),
        [Saves {
            from: "Steam (Proton)",
            folder: saves.parent().unwrap().to_path_buf()
        }]
    );
    let report = report(&home);
    let lines: Vec<&str> = report.lines().collect();
    assert_eq!(lines.len(), 3, "{report}");
    assert!(lines[0].starts_with("own\t"));
    // (The folder as found: on Windows, where these tests also run, its text mixes `/` and `\`.)
    let found = &games(&home)[0].folder;
    assert_eq!(
        lines[1],
        format!("game\tSteam\t{}", escape(&path_bytes(found)))
    );
    assert_eq!(found, &game);
    assert!(lines[2].starts_with("saves\tSteam (Proton)\t"));
    // macOS: no store installs the game there, and no Proton.
    let mac = s.home(System::MacOs);
    assert!(games(&mac).is_empty());
    assert!(save_folders(&mac).is_empty());
}

#[test]
fn the_flathub_steam_is_looked_in_and_a_folder_without_the_game_is_left_out() {
    let s = Scratch::new("flatpak");
    let home = s.home(System::Linux);
    let game = s.game(
        "home/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/Fallout New Vegas",
        "Data",
        "FalloutNV.esm",
    );
    // An uninstalled copy's empty folder in the native Steam.
    s.dir("home/.steam/steam/steamapps/common/Fallout New Vegas/Data");
    assert_eq!(
        games(&home),
        [Found {
            store: "Steam",
            folder: game
        }]
    );
}

/// `~/.steam/steam` is a link to `~/.local/share/Steam` on most systems: one library, one game.
#[cfg(unix)]
#[test]
fn steams_link_to_itself_is_one_library() {
    let s = Scratch::new("link");
    let home = s.home(System::Linux);
    let game = s.game(
        "home/.local/share/Steam/steamapps/common/Fallout New Vegas",
        "Data",
        "FalloutNV.esm",
    );
    s.dir("home/.steam");
    std::os::unix::fs::symlink(
        s.0.join("home/.local/share/Steam"),
        s.0.join("home/.steam/steam"),
    )
    .unwrap();
    assert_eq!(steam_libraries(&home).len(), 1);
    assert_eq!(games(&home).len(), 1);
    assert_eq!(
        std::fs::canonicalize(&games(&home)[0].folder).unwrap(),
        std::fs::canonicalize(&game).unwrap()
    );
}

#[test]
fn heroic_and_lutris_installs_are_found_where_their_files_name_them() {
    let s = Scratch::new("heroic");
    let home = s.home(System::Linux);
    let gog = s.game("Games/Heroic/Fallout New Vegas", "Data", "FalloutNV.esm");
    let other = s.dir("Games/Heroic/Another Game");
    s.file(
        "home/.config/heroic/gog_store/installed.json",
        &format!(
            "{{\"installed\":[{{\"install_path\":\"{}\"}},{{\"install_path\":\"{}\"}}]}}",
            json_text(&other),
            json_text(&gog)
        ),
    );
    let prefix = s.dir("Games/Heroic/Prefixes/FNV");
    let saves =
        s.dir("Games/Heroic/Prefixes/FNV/drive_c/users/player/Documents/My Games/FalloutNV");
    s.file(
        "home/.config/heroic/GamesConfig/1454587428.json",
        &format!(
            "{{\"1454587428\":{{\"winePrefix\":\"{}\"}}}}",
            json_text(&prefix)
        ),
    );
    let lutris = s.game("Games/lutris-fnv/drive_c/FNV", "Data", "FalloutNV.esm");
    s.file(
        "home/.local/share/lutris/games/fallout-new-vegas-1.yml",
        &format!(
            "game:\n  exe: {}\n  prefix: {}\n",
            lutris.join("FalloutNV.exe").display(),
            s.0.join("Games/lutris-fnv").display()
        ),
    );
    let found = games(&home);
    assert_eq!(
        found,
        [
            Found {
                store: "Heroic",
                folder: gog
            },
            Found {
                store: "Lutris",
                folder: lutris
            }
        ]
    );
    assert_eq!(
        save_folders(&home),
        [Saves {
            from: "Wine prefix",
            folder: saves
        }]
    );
}

fn json_text(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

#[test]
fn the_xdg_documents_folder_is_read_from_user_dirs() {
    let s = Scratch::new("xdg");
    let home = s.home(System::Linux);
    assert_eq!(xdg_documents(&home), None);
    s.file(
        "home/.config/user-dirs.dirs",
        "# comment\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_DOCUMENTS_DIR=\"$HOME/Dokumente\"\n",
    );
    assert_eq!(
        xdg_documents(&home),
        Some(s.0.join("home").join("Dokumente"))
    );
    let full = s.0.join("elsewhere");
    s.file(
        "home/.config/user-dirs.dirs",
        &format!("XDG_DOCUMENTS_DIR=\"{}\"\n", full.display()),
    );
    assert_eq!(xdg_documents(&home), Some(full));
    // Review of #630 (round 2): `/` and the home folder named in full are xdg-user-dirs' "none".
    for none in ["/".to_string(), s.0.join("home").display().to_string()] {
        s.file(
            "home/.config/user-dirs.dirs",
            &format!("XDG_DOCUMENTS_DIR=\"{none}\"\n"),
        );
        assert_eq!(xdg_documents(&home), None, "{none}");
    }
}

#[test]
fn the_players_folders_are_refused_and_the_own_folder_taken_on_linux() {
    let s = Scratch::new("guard-linux");
    let home = s.home(System::Linux);
    let lib2 = s.dir("mnt/SteamLibrary");
    s.dir("mnt/SteamLibrary/steamapps");
    s.file(
        "home/.local/share/Steam/steamapps/libraryfolders.vdf",
        &vdf_library(&lib2),
    );
    let game = s.game(
        "mnt/SteamLibrary/steamapps/common/Fallout New Vegas",
        "Data",
        "FalloutNV.esm",
    );
    s.file(
        "home/.config/user-dirs.dirs",
        "XDG_DOCUMENTS_DIR=\"$HOME/Dokumente\"\n",
    );
    let never = never_written(&home);
    let h = s.0.join("home");
    for dir in [
        h.join("Documents/My Games/FalloutNV/Saves"),
        h.join("DOCUMENTS"),
        h.join("Dokumente/x"),
        h.join(".steam/steam/steamapps/compatdata/22380/pfx/drive_c"),
        h.join(".local/share/Steam/userdata"),
        h.join(".var/app/com.valvesoftware.Steam/data"),
        lib2.join("steamapps/compatdata/22380/pfx/drive_c/users/steamuser/Documents"),
        game.join("Data"),
        PathBuf::from("relative/saves"),
        h.join("x/../Documents"),
    ] {
        assert!(refused(&dir, &never).is_some(), "{}", dir.display());
    }
    for dir in [
        own_folder(&home).join("Saves"),
        h.join("DocumentsScratch"),
        h.join("Games/nv-rs"),
        s.0.join("mnt/other"),
    ] {
        assert_eq!(refused(&dir, &never), None, "{}", dir.display());
    }
}

#[test]
fn the_players_folders_are_refused_and_the_own_folder_taken_on_macos() {
    let s = Scratch::new("guard-mac");
    let home = s.home(System::MacOs);
    let never = never_written(&home);
    let h = s.0.join("home");
    for dir in [
        h.join("Documents/My Games/FalloutNV"),
        h.join("Library/Preferences"),
        h.join("library/application support/CrossOver/Bottles"),
        h.join("Library/Application Support/nv-rs-other"),
    ] {
        assert!(refused(&dir, &never).is_some(), "{}", dir.display());
    }
    for dir in [
        own_folder(&home),
        own_folder(&home).join("Saves"),
        h.join("Library/Application Support/NV-RS/Saves"),
        h.join("Games/FalloutNV"),
    ] {
        assert_eq!(refused(&dir, &never), None, "{}", dir.display());
    }
}

/// A link that leads into the player's folders is refused by where it leads.
#[cfg(unix)]
#[test]
fn a_link_into_the_players_folders_is_refused() {
    let s = Scratch::new("guard-link");
    let home = s.home(System::Linux);
    let documents = s.dir("home/Documents");
    std::os::unix::fs::symlink(&documents, s.0.join("innocent")).unwrap();
    let never = never_written(&home);
    assert!(refused(&s.0.join("innocent/Saves"), &never).is_some());
    assert_eq!(refused(&s.0.join("elsewhere"), &never), None);
}

/// Review of #630: nv-rs's own folder is never taken away by a broad player folder: an XDG
/// Documents folder that is the home folder ("$HOME/", xdg-user-dirs' "none"), XDG_DATA_HOME inside
/// the Flathub Steam's folder, a Lutris prefix that holds the own folder.
#[test]
fn the_own_folder_stays_writable_inside_broad_player_folders() {
    let s = Scratch::new("own-broad");
    let h = s.0.join("home");
    s.file(
        "home/.config/user-dirs.dirs",
        "XDG_DOCUMENTS_DIR=\"$HOME/\"\n",
    );
    let home = s.home(System::Linux);
    assert_eq!(xdg_documents(&home), None);
    let never = never_written(&home);
    assert_eq!(refused(&own_folder(&home).join("Saves"), &never), None);
    assert!(refused(&h.join("Documents/x"), &never).is_some());
    // Started from the Flathub Steam: XDG_DATA_HOME inside its folder.
    let flatpak = Home::new(
        h.clone(),
        Some(h.join(".var/app/com.valvesoftware.Steam/data")),
        None,
        System::Linux,
    )
    .unwrap();
    let never = never_written(&flatpak);
    assert_eq!(refused(&own_folder(&flatpak).join("Saves"), &never), None);
    // Review of #630 (round 2): on Linux names that differ in case are other folders, so the
    // exception is the own folder named exactly.
    for other in ["NV-RS/Saves", "Nv-Rs/Saves"] {
        let dir = h.join(".var/app/com.valvesoftware.Steam/data").join(other);
        assert!(refused(&dir, &never).is_some(), "{}", dir.display());
    }
    assert!(refused(
        &h.join(".var/app/com.valvesoftware.Steam/data/Steam"),
        &never
    )
    .is_some());
    assert!(!own_folder_warnings(&flatpak, &never).is_empty());
    // A Lutris prefix that is the whole data folder.
    s.file(
        "home/.local/share/lutris/games/x.yml",
        &format!("game:\n  prefix: {}\n", h.join(".local/share").display()),
    );
    let home = s.home(System::Linux);
    let never = never_written(&home);
    assert_eq!(refused(&own_folder(&home).join("Saves"), &never), None);
    assert!(refused(&h.join(".local/share/other"), &never).is_some());
    // A prefix trick is not the own folder.
    assert!(refused(&h.join(".local/share/nv-rs-other"), &never).is_some());
}

/// A link AT nv-rs's own folder opens nothing up: on macOS a link from it to ~/Library leaves
/// ~/Library refused; a link planted inside it to a player folder is refused by where it leads.
#[cfg(unix)]
#[test]
fn a_link_at_or_inside_the_own_folder_opens_nothing() {
    let s = Scratch::new("own-link");
    let home = s.home(System::MacOs);
    let library = s.dir("home/Library");
    s.dir("home/Library/Application Support");
    std::os::unix::fs::symlink(&library, own_folder(&home)).unwrap();
    let never = never_written(&home);
    assert!(refused(&own_folder(&home).join("Preferences"), &never).is_some());
    assert!(refused(&library.join("Preferences"), &never).is_some());
    assert!(!own_folder_warnings(&home, &never).is_empty());
    let linux = s.home(System::Linux);
    let own = own_folder(&linux);
    std::fs::create_dir_all(&own).unwrap();
    let documents = s.dir("home/Documents");
    std::os::unix::fs::symlink(&documents, own.join("planted")).unwrap();
    let never = never_written(&linux);
    assert!(refused(&own.join("planted/x.fos"), &never).is_some());
    assert_eq!(refused(&own.join("Saves/x.fos"), &never), None);
    // Review of #630 (round 2): a link inside a player's folder that leads into the own folder is
    // still the player's folder by its name: the exception needs the place inside the own folder
    // by its text too.
    std::os::unix::fs::symlink(&own, documents.join("to-own")).unwrap();
    assert!(refused(&documents.join("to-own/Saves"), &never).is_some());
}

/// Review of #630 (round 5): a link inside nv-rs's own folder to a place no list names (a mounted
/// Windows partition's Documents with the player's real saves) is refused, and `--find` says why;
/// a plain folder in the own folder is allowed, and so is the own folder moved to another disk
/// (the own folder itself a link).
#[cfg(unix)]
#[test]
fn a_link_out_of_the_own_folder_to_an_unlisted_place_is_refused() {
    let s = Scratch::new("own-leaves");
    let home = s.home(System::Linux);
    let own = own_folder(&home);
    std::fs::create_dir_all(own.join("game")).unwrap();
    let windows = s.dir("mnt/c/Users/x/Documents");
    std::fs::create_dir_all(windows.join("My Games/FalloutNV/Saves")).unwrap();
    let never = never_written(&home);
    // Before the link: the own folder's saves are allowed, the mounted partition is not guarded.
    let saves = own.join("game/Documents/My Games/FalloutNV/Saves/x.fos");
    assert_eq!(refused(&saves, &never), None);
    assert_eq!(
        refused(&windows.join("My Games/FalloutNV/Saves/x.fos"), &never),
        None
    );
    std::os::unix::fs::symlink(&windows, own.join("game/Documents")).unwrap();
    assert_eq!(refused(&saves, &never), Some(LEAVES_OWN));
    assert_eq!(
        refused(&own.join("game/Documents"), &never),
        Some(LEAVES_OWN)
    );
    assert_eq!(refused(&own.join("logs/x.log"), &never), None);
    let why = own_folder_refusal(&home, &never).expect("--find refuses the own folder");
    assert!(why.contains("a link to a place outside it"), "{why}");
    // The own folder moved to another disk (a link AT it): its places are still its own.
    let moved = Home::new(s.0.join("home2"), None, None, System::Linux).unwrap();
    let moved_own = own_folder(&moved);
    std::fs::create_dir_all(moved_own.parent().unwrap()).unwrap();
    let disk2 = s.dir("disk2/nv-rs");
    std::os::unix::fs::symlink(&disk2, &moved_own).unwrap();
    let never = never_written(&moved);
    assert_eq!(refused(&moved_own.join("game/Saves/x.fos"), &never), None);
    assert_eq!(own_folder_refusal(&moved, &never), None);
}

/// Review of #630 (round 2): a plainly named link that leads into a Steam library's prefixes is
/// refused by where it leads, though no list names that library.
#[cfg(unix)]
#[test]
fn a_plain_link_into_a_libraries_prefixes_is_refused() {
    let s = Scratch::new("shape-link");
    let pfx = s.dir("mnt/Lib/steamapps/compatdata/22380/pfx");
    std::os::unix::fs::symlink(&pfx, s.0.join("innocent")).unwrap();
    let empty = Home::new(s.0.join("nobody"), None, None, System::Linux).unwrap();
    let never = never_written(&empty);
    assert!(refused(&s.0.join("innocent/x"), &never).is_some());
    assert_eq!(refused(&s.0.join("mnt/Lib/notes"), &never), None);
}

/// Review of #630 (round 1, owed): a guarded folder that is itself a link (Documents kept on
/// another disk) is guarded where it leads too.
#[cfg(unix)]
#[test]
fn a_guarded_folder_that_is_a_link_is_guarded_where_it_leads() {
    let s = Scratch::new("root-link");
    let elsewhere = s.dir("disk2/Documents");
    s.dir("home");
    std::os::unix::fs::symlink(&elsewhere, s.0.join("home/Documents")).unwrap();
    let home = s.home(System::Linux);
    let never = never_written(&home);
    assert!(refused(&elsewhere.join("My Games/FalloutNV/Saves"), &never).is_some());
    assert!(refused(&s.0.join("home/Documents/x"), &never).is_some());
    assert_eq!(refused(&s.0.join("disk2/other"), &never), None);
}

/// Review of #630 (round 2): when the own folder itself is refused, the warning names the cause
/// and the folder, not a guess: a `..` in XDG_DATA_HOME, a path shaped like Steam's folders.
#[test]
fn the_own_folders_warning_names_the_cause() {
    let s = Scratch::new("own-cause");
    let h = s.0.join("home");
    let dotted = Home::new(
        h.clone(),
        Some(h.join(".local/../data")),
        None,
        System::Linux,
    )
    .unwrap();
    let never = never_written(&dotted);
    let warnings = own_folder_warnings(&dotted, &never).join("\n");
    assert!(warnings.contains("`..`"), "{warnings}");
    assert!(warnings.contains("nv-rs's own folder"), "{warnings}");
    assert!(!warnings.contains("link"), "{warnings}");
    let shaped = Home::new(
        h.clone(),
        Some(s.0.join("lib/steamapps/common/x")),
        None,
        System::Linux,
    )
    .unwrap();
    let never = never_written(&shaped);
    let warnings = own_folder_warnings(&shaped, &never).join("\n");
    assert!(warnings.contains("Steam library"), "{warnings}");
    assert!(!warnings.contains("link"), "{warnings}");
}

/// The Snap Steam, plain Wine, Bottles, PlayOnLinux and CrossOver: guarded as whole folders, saves
/// found in them; and anything shaped like a library's or a prefix's folders is refused even when
/// no list names it.
#[test]
fn other_steams_and_wine_tools_are_guarded_and_their_saves_found() {
    let s = Scratch::new("wine-tools");
    let h = s.0.join("home");
    let snap_game = s.game(
        "home/snap/steam/common/.local/share/Steam/steamapps/common/Fallout New Vegas",
        "Data",
        "FalloutNV.esm",
    );
    let snap_saves = s.dir(
        "home/snap/steam/common/.local/share/Steam/steamapps/compatdata/22380/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV",
    );
    // A non-Steam shortcut's prefix (its own id).
    let shortcut = s.dir(
        "home/snap/steam/common/.local/share/Steam/steamapps/compatdata/3123456789/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV",
    );
    let wine = s.dir("home/.wine/drive_c/users/player/Documents/My Games/FalloutNV");
    let bottles = s.dir(
        "home/.var/app/com.usebottles.bottles/data/bottles/bottles/FNV/drive_c/users/player/Documents/My Games/FalloutNV",
    );
    let pol =
        s.dir("home/.PlayOnLinux/wineprefix/FNV/drive_c/users/player/Documents/My Games/FalloutNV");
    let cx = s.dir("home/.cxoffice/FNV/drive_c/users/crossover/Documents/My Games/FalloutNV");
    let home = s.home(System::Linux);
    let scan = scan(&home);
    assert_eq!(
        scan.games,
        [Found {
            store: "Steam",
            folder: snap_game
        }]
    );
    let found: Vec<&PathBuf> = scan.saves.iter().map(|s| &s.folder).collect();
    for want in [&snap_saves, &shortcut, &wine, &cx, &pol, &bottles] {
        assert!(found.contains(&want), "{} in {found:?}", want.display());
    }
    let never = never_written_from(&home, &scan);
    for dir in [
        h.join("snap/steam/x"),
        h.join(".wine/x"),
        h.join(".cxoffice/x"),
        h.join(".PlayOnLinux/x"),
        h.join(".var/app/com.usebottles.bottles/x"),
        h.join(".local/share/bottles/x"),
    ] {
        assert!(refused(&dir, &never).is_some(), "{}", dir.display());
    }
    // By shape: a library on a disk nobody listed, a prefix nobody named.
    let empty = Home::new(s.0.join("nobody"), None, None, System::Linux).unwrap();
    let never = never_written(&empty);
    for dir in [
        s.0.join("mnt/Lib/SteamApps/compatdata/22380/pfx/x"),
        s.0.join("mnt/Lib/steamapps/COMMON/Fallout New Vegas"),
        s.0.join("games/prefix/drive_c/Users/me/Documents"),
    ] {
        assert!(refused(&dir, &never).is_some(), "{}", dir.display());
    }
    assert_eq!(refused(&s.0.join("mnt/Lib/notes"), &never), None);
}

/// The older flat library list, a "path" that is a value, relative paths; a 22490 manifest naming
/// another folder; two folders differing only in case, the empty one first.
#[test]
fn library_lists_manifests_and_case_variants() {
    let flat = "\"LibraryFolders\"\n{\n\t\"TimeNextStatsReport\"\t\"1\"\n\t\"1\"\t\"/mnt/games/SteamLibrary\"\n\t\"2\"\t\"relative/lib\"\n}\n";
    assert_eq!(
        library_paths(flat),
        [PathBuf::from("/mnt/games/SteamLibrary")]
    );
    let labelled = "\"libraryfolders\" { \"0\" { \"label\" \"path\" \"path\" \"/x/Steam\" \"contentid\" \"5\" } }";
    assert_eq!(library_paths(labelled), [PathBuf::from("/x/Steam")]);
    assert!(library_paths("\"libraryfolders\" { \"0\" { \"path\" \"/cut").is_empty());
    let s = Scratch::new("variants");
    let home = s.home(System::Linux);
    s.dir("home/.steam/root/steamapps/common/Fallout New Vegas/Data");
    let real = s.game(
        "home/.steam/root/steamapps/common/FALLOUT NEW VEGAS",
        "data",
        "falloutnv.esm",
    );
    let pcr = s.game(
        "home/.steam/root/steamapps/common/FNV PCR",
        "Data",
        "FalloutNV.esm",
    );
    s.file(
        "home/.steam/root/steamapps/appmanifest_22490.acf",
        "\"AppState\" { \"installdir\" \"FNV PCR\" }",
    );
    let found: Vec<PathBuf> = games(&home).into_iter().map(|f| f.folder).collect();
    assert!(found.contains(&pcr), "{found:?}");
    // A disk that ignores case (Windows, macOS) holds one folder for both names.
    if !s
        .0
        .join("home/.steam/root/steamapps/common/fallout new vegas")
        .exists()
    {
        assert!(found.contains(&real), "{found:?}");
        // Two Data folders, the empty one named exactly.
        let both = s.dir("both");
        s.dir("both/Data");
        s.file("both/data/FalloutNV.esm", "x");
        assert_eq!(data_folder(&both), Some(both.join("data")));
    }
}

/// legendary's \u escapes (a non-ASCII home folder) and PyYAML's \x escapes and folded lines.
#[test]
fn store_files_escapes_are_decoded() {
    let json =
        r#"{"install_path": "/home/j\u00f6rg/Games/FNV", "x": "\ud83d\ude00", "y": "\ud83d"}"#;
    assert_eq!(json_strings(json, "install_path"), ["/home/jörg/Games/FNV"]);
    assert_eq!(json_strings(json, "x"), ["\u{1f600}"]);
    assert!(json_strings(json, "y").is_empty());
    let yaml = "game:\n  exe: \"/home/j\\xF6rg/Games/a very long folder name that PyYAML\n    folds onto the next line/FalloutNV.exe\"\n  game_path: /home/a plain\n    value folded\n  prefix: 'it''s'\nsystem: {}\n";
    assert_eq!(
        yaml_values(yaml, "exe"),
        ["/home/jörg/Games/a very long folder name that PyYAML folds onto the next line/FalloutNV.exe"]
    );
    assert_eq!(
        yaml_values(yaml, "game_path"),
        ["/home/a plain value folded"]
    );
    assert_eq!(yaml_values(yaml, "prefix"), ["it's"]);
}

/// Review of #630 (round 2): a broken `\u` escape or a lone high surrogate leaves its value out
/// without swallowing the closing quote, so the next value is still read.
#[test]
fn a_broken_json_escape_does_not_hide_the_next_value() {
    for broken in [r"\ud83d", r"\u12", r"\ud83d\u12", r"\ud83dA"] {
        let json = format!(
            r#"{{"install_path": "/a{broken}", "install_path": "/b", "winePrefix": "/games/fnv-prefix"}}"#
        );
        assert_eq!(json_strings(&json, "install_path"), ["/b"], "{json}");
        assert_eq!(
            json_strings(&json, "winePrefix"),
            ["/games/fnv-prefix"],
            "{json}"
        );
    }
}

/// Review of #630 (round 2): every escape PyYAML writes (`\_` for a no-break space, `\N`, `\L`,
/// `\P`, `\a`, `\b`, `\e`, `\f`, `\v`) and its `\`-continued lines; an escape YAML has not is a
/// broken value.
#[test]
fn every_yaml_escape_pyyaml_writes_is_decoded() {
    let yaml = "game:\n  game_path: \"/home/x\\_y/a\\N\\L\\P\\a\\b\\e\\f\\v\\\n    \\ z\"\n  exe: \"/q\\k\"\n";
    assert_eq!(
        yaml_values(yaml, "game_path"),
        ["/home/x\u{a0}y/a\u{85}\u{2028}\u{2029}\u{7}\u{8}\u{1b}\u{c}\u{b} z"]
    );
    assert!(yaml_values(yaml, "exe").is_empty());
}

/// --find's fields are escaped: a tab, a line break, a quote, a percent sign and a non-UTF-8 byte
/// in a folder's name cannot make or change a record.
#[test]
fn the_report_cannot_be_forged() {
    assert_eq!(escape(b"/a b/c-d_e.f"), "/a b/c-d_e.f");
    assert_eq!(escape(b"a\tb\nc\"d%e\xff"), "a%09b%0Ac%22d%25e%FF");
    assert_eq!(escape(b"Steam (Proton)"), "Steam (Proton)");
}

/// The same with real folders: a Heroic prefix whose name holds a line break and a forged record.
#[cfg(unix)]
#[test]
fn a_folder_name_with_a_line_break_stays_one_field() {
    let s = Scratch::new("forge");
    let prefix = s.0.join("p\ngame\tSteam\t/q");
    std::fs::create_dir_all(prefix.join("drive_c/users/x/Documents/My Games/FalloutNV")).unwrap();
    s.file(
        "home/.config/heroic/GamesConfig/1.json",
        &format!(
            "{{\"1\":{{\"winePrefix\":\"{}\"}}}}",
            prefix
                .display()
                .to_string()
                .replace('\n', "\\n")
                .replace('\t', "\\t")
        ),
    );
    let home = s.home(System::Linux);
    let report = report(&home);
    let lines: Vec<&str> = report.lines().collect();
    assert!(lines.iter().all(|l| !l.starts_with("game")), "{report}");
    assert_eq!(
        lines.iter().filter(|l| l.starts_with("saves\t")).count(),
        1,
        "{report}"
    );
    assert!(report.contains("p%0Agame%09Steam%09"), "{report}");
}

/// A name that is not UTF-8 keeps its bytes (Linux allows any bytes; macOS's disks do not).
#[cfg(target_os = "linux")]
#[test]
fn a_name_that_is_not_utf8_keeps_its_bytes() {
    use std::os::unix::ffi::OsStrExt;
    let s = Scratch::new("bytes");
    let name = std::ffi::OsStr::from_bytes(b"Fallout \xe9");
    let game =
        s.0.join("home/.local/share/Steam/steamapps/common")
            .join(name);
    std::fs::create_dir_all(game.join("Data")).unwrap();
    std::fs::write(game.join("Data/FalloutNV.esm"), b"x").unwrap();
    s.file(
        "home/.local/share/Steam/steamapps/appmanifest_22380.acf",
        "\"AppState\" { \"installdir\" \"Fallout \\u00e9\" }",
    );
    let home = s.home(System::Linux);
    // Not found by a manifest that names it in UTF-8, but escaped byte for byte when printed.
    assert!(escape(&path_bytes(&game)).ends_with("Fallout %E9"));
    assert!(!report(&home).contains('\u{fffd}'));
    // Review of #630 (round 2): a game that IS found under a home folder whose name is not UTF-8
    // is reported byte for byte (own, game and saves lines), never as lossy text.
    let odd = s.0.join(std::ffi::OsStr::from_bytes(b"h\xe9me"));
    let found = odd.join(".local/share/Steam/steamapps/common/Fallout New Vegas");
    std::fs::create_dir_all(found.join("Data")).unwrap();
    std::fs::write(found.join("Data/FalloutNV.esm"), b"x").unwrap();
    std::fs::create_dir_all(odd.join(".local/share/Steam/steamapps/compatdata/22380/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV")).unwrap();
    let home = Home::new(odd.clone(), None, None, System::Linux).unwrap();
    let report = report(&home);
    let want = escape(&path_bytes(&found));
    assert!(want.contains("h%E9me"), "{want}");
    assert!(
        report.lines().any(|l| l == format!("game\tSteam\t{want}")),
        "{report}"
    );
    assert!(
        report
            .lines()
            .any(|l| l.starts_with("saves\t") && l.contains("h%E9me")),
        "{report}"
    );
    assert!(report.starts_with(&format!("own\t{}", escape(&path_bytes(&own_folder(&home))))));
    assert!(
        !report.contains('\u{fffd}') && !report.contains("%EF%BF%BD"),
        "{report}"
    );
}

/// Review of #630 (round 2): every byte a shell or `printf %b` reads specially is escaped (a
/// backslash above all: play.sh decodes with `printf %b`), and a run of high bytes keeps each.
#[test]
fn every_byte_a_shell_reads_specially_is_escaped() {
    for b in b"\\$`'\"*?!&;|<>[]{}#\r\0\t\n%".iter().copied() {
        assert_eq!(escape(&[b]), format!("%{b:02X}"), "byte {b:#04x}");
    }
    assert_eq!(escape(b"\xc3\xa9\xff\x80"), "%C3%A9%FF%80");
    for b in b" /._-+,:@=~()aZ09".iter().copied() {
        assert_eq!(escape(&[b]), (b as char).to_string());
    }
}

/// Review of #631: --find says in a record of its own when nv-rs's own folder may not be written
/// (play.sh stops before its first write), checked at the deepest folder play.sh writes: a `..`
/// in XDG_DATA_HOME, or a link at the own folder's `game` folder into Documents.
#[test]
fn a_refused_own_folder_gets_a_record_of_its_own() {
    let s = Scratch::new("refused-record");
    let h = s.0.join("home");
    let plain = s.home(System::Linux);
    assert!(!report(&plain).lines().any(|l| l.starts_with("refused\t")));
    let dotted = Home::new(
        h.clone(),
        Some(h.join(".local/../data")),
        None,
        System::Linux,
    )
    .unwrap();
    let report_text = report(&dotted);
    let line = report_text
        .lines()
        .find(|l| l.starts_with("refused\t"))
        .unwrap_or_else(|| panic!("{report_text}"));
    assert!(line.contains("nv-rs%27s own folder"), "{line}");
    assert!(!report_text
        .lines()
        .any(|l| l.starts_with("warning\t") && l.contains("cannot be written")));
    #[cfg(unix)]
    {
        let documents = s.dir("home/Documents");
        let own = own_folder(&plain);
        std::fs::create_dir_all(&own).unwrap();
        std::os::unix::fs::symlink(&documents, own.join("game")).unwrap();
        assert!(
            report(&plain).lines().any(|l| l.starts_with("refused\t")),
            "{}",
            report(&plain)
        );
    }
}

/// Review of #630 (round 4): a manifest's `installdir` names one folder inside the library's
/// `steamapps/common`, never a path: one that climbs out (`../../outside`), a full path to a
/// game elsewhere, or one with a `..` inside it is not followed, though each leads to a game.
#[test]
fn a_manifests_installdir_cannot_lead_out_of_steamapps_common() {
    let s = Scratch::new("installdir");
    let home = s.home(System::Linux);
    let library = s.0.join("home/.local/share/Steam");
    let common = s.dir("home/.local/share/Steam/steamapps/common");
    // Where each installdir would lead if it were joined as a path.
    let climbed = s.game("home/.local/share/Steam/outside", "Data", "FalloutNV.esm");
    let absolute = s.game("elsewhere/FNV", "Data", "FalloutNV.esm");
    s.dir("home/.local/share/Steam/steamapps/common/common");
    let dotted = s.game(
        "home/.local/share/Steam/steamapps/common/x",
        "Data",
        "FalloutNV.esm",
    );
    let manifest = |app: u32, dir: &str| {
        s.file(
            &format!("home/.local/share/Steam/steamapps/appmanifest_{app}.acf"),
            &format!(
                "\"AppState\" {{ \"installdir\" \"{}\" }}",
                dir.replace('\\', "\\\\")
            ),
        );
    };
    // Each case alone, so one followed path cannot hide behind another.
    let cases = [
        ("../../outside".to_string(), climbed),
        (absolute.display().to_string(), absolute.clone()),
        ("common/../x".to_string(), dotted),
    ];
    let mut followed = Vec::new();
    for (dir, there) in &cases {
        assert!(data_folder(there).is_some(), "{there:?}");
        manifest(22380, dir);
        let found = games(&home);
        if !found.is_empty() {
            followed.push(format!("installdir {dir:?} was followed: {found:?}"));
        }
    }
    assert!(followed.is_empty(), "{followed:#?}");
    // A plain name is followed (the test's library is read at all).
    manifest(22380, "FNV Game");
    let plain = s.game(
        "home/.local/share/Steam/steamapps/common/FNV Game",
        "Data",
        "FalloutNV.esm",
    );
    assert_eq!(
        games(&home),
        [Found {
            store: "Steam",
            folder: plain
        }]
    );
    assert!(common.is_dir() && library.is_dir());
}
