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
    assert_eq!(lines[1], format!("game\tSteam\t{}", game.display()));
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
