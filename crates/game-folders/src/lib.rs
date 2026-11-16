//! The player's game and saves on Linux and macOS, the folders nv-rs must never write to there,
//! and nv-rs's own data folder (where its saves, settings and logs go).
//!
//! Everything here is a plain function over a [`Home`] (the home folder and the XDG variables), so
//! the tests run on made-up folder trees on any system. Nothing here writes anything: the stores'
//! files and the game's folders are only read. On Windows the launcher (`scripts/launcher`,
//! `find.rs`) finds the game through the registry instead; this crate is not used there.
//!
//! Where a Linux player's game is (docs: the Linux/macOS census, "Run-time matters"):
//! - Steam for Linux keeps the Windows game in a Steam library:
//!   `<library>/steamapps/common/Fallout New Vegas`, the libraries listed in
//!   `<steam>/steamapps/libraryfolders.vdf`; Steam itself is in `~/.steam/steam` (usually a link to
//!   `~/.local/share/Steam`), or, installed from Flathub, in
//!   `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`.
//! - The game's `Documents\My Games\FalloutNV` (saves, Fallout.ini) is inside the Proton prefix:
//!   `<library>/steamapps/compatdata/<app id>/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV`.
//!   The app id is 22380, or 22490 (the copy sold in some regions; the Windows launcher looks for
//!   both).
//! - GOG and Epic copies installed with Heroic, and copies installed with Lutris, are found only
//!   where those programs' own files name the folder, and only when that folder really holds the
//!   game (its `Data/FalloutNV.esm`): no folder name is guessed. Anything else: the player names
//!   the folder.
//! - macOS: the game was never sold for macOS, so no store installs it there: the player copies a
//!   Windows installation's folder over and names it.
//!
//! Names are matched without case (`Data`, `FalloutNV.esm`, `My Games`): a Windows game copied to
//! a Linux disk keeps whatever case its files had.

use std::path::{Component, Path, PathBuf};

/// Steam's app ids of Fallout: New Vegas (the main one and the one sold in some regions), as the
/// Windows launcher's finder has them.
pub const STEAM_APPS: [u32; 2] = [22380, 22490];

/// The game's master file, which its `Data` folder holds.
pub const MAIN_MASTER: &str = "FalloutNV.esm";

/// Which system's folders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum System {
    Linux,
    MacOs,
}

impl System {
    /// The system this program was built for (Linux's rules for every system but macOS).
    pub fn this() -> System {
        if cfg!(target_os = "macos") {
            System::MacOs
        } else {
            System::Linux
        }
    }
}

/// The player's home folder and the variables that move folders out of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Home {
    pub home: PathBuf,
    /// `XDG_DATA_HOME`, when set to a full path (the XDG rule: a relative one is ignored).
    pub data_home: Option<PathBuf>,
    /// `XDG_CONFIG_HOME`, the same.
    pub config_home: Option<PathBuf>,
    pub system: System,
}

impl Home {
    /// From this process's `HOME`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME`; `None` when `HOME` is
    /// not set or not a full path.
    pub fn from_process() -> Option<Home> {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
        Home::new(
            var("HOME")?,
            var("XDG_DATA_HOME"),
            var("XDG_CONFIG_HOME"),
            System::this(),
        )
    }

    /// `None` when `home` is not a full path; an XDG folder that is not one is ignored.
    pub fn new(
        home: PathBuf,
        data_home: Option<PathBuf>,
        config_home: Option<PathBuf>,
        system: System,
    ) -> Option<Home> {
        let full = |p: &PathBuf| p.has_root();
        if !full(&home) {
            return None;
        }
        Some(Home {
            home,
            data_home: data_home.filter(full),
            config_home: config_home.filter(full),
            system,
        })
    }

    /// `$XDG_CONFIG_HOME`, else `~/.config`.
    fn config(&self) -> PathBuf {
        self.config_home
            .clone()
            .unwrap_or_else(|| self.home.join(".config"))
    }

    /// `$XDG_DATA_HOME`, else `~/.local/share`.
    fn data(&self) -> PathBuf {
        self.data_home
            .clone()
            .unwrap_or_else(|| self.home.join(".local").join("share"))
    }
}

/// nv-rs's own folder: its saves, settings, logs and the game folder it remembers. Linux:
/// `$XDG_DATA_HOME/nv-rs`, else `~/.local/share/nv-rs`; macOS: `~/Library/Application Support/nv-rs`.
pub fn own_folder(home: &Home) -> PathBuf {
    match home.system {
        System::Linux => home.data().join("nv-rs"),
        System::MacOs => home
            .home
            .join("Library")
            .join("Application Support")
            .join("nv-rs"),
    }
}

/// `name` in `dir`: the entry of exactly that name, else the first (in name order) that matches
/// it without case. `None` when there is none or `dir` can't be read.
/// The name returned is the one the folder lists (a disk that ignores case answers for any case,
/// so the listing is read, not asked).
pub fn find_child(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.eq_ignore_ascii_case(name))
        .collect();
    names.sort();
    let exact = names.iter().find(|n| *n == name);
    exact.or(names.first()).map(|n| dir.join(n))
}

/// `base` joined with each of `parts`, each matched without case ([`find_child`]).
pub fn resolve(base: &Path, parts: &[&str]) -> Option<PathBuf> {
    parts
        .iter()
        .try_fold(base.to_path_buf(), |at, part| find_child(&at, part))
}

/// The game's `Data` folder when `folder` is the game's folder or its `Data` (it holds
/// `FalloutNV.esm`, any case).
pub fn data_folder(folder: &Path) -> Option<PathBuf> {
    if find_child(folder, MAIN_MASTER).is_some_and(|f| f.is_file()) {
        return Some(folder.to_path_buf());
    }
    let data = find_child(folder, "Data")?;
    find_child(&data, MAIN_MASTER)
        .is_some_and(|f| f.is_file())
        .then_some(data)
}

/// The game's folder (the one holding `Data`) for a folder the player named: the game's folder
/// itself, or its `Data`.
pub fn game_folder(named: &Path) -> Option<PathBuf> {
    let data = data_folder(named)?;
    if data == named {
        // `named` is Data itself: its parent is the game's folder.
        data.parent().map(Path::to_path_buf)
    } else {
        Some(named.to_path_buf())
    }
}

/// The quoted strings of a Steam `.vdf` or `.acf` file, in order, `\\` read as `\`.
pub fn vdf_strings(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    if let Some(next) = chars.next() {
                        s.push(next);
                    }
                }
                other => s.push(other),
            }
        }
        out.push(s);
    }
    out
}

/// The value after each `key` in a `.vdf`: its `"key" "value"` pairs, any case.
pub fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let strings = vdf_strings(text);
    strings
        .windows(2)
        .filter(|pair| pair[0].eq_ignore_ascii_case(key))
        .map(|pair| pair[1].clone())
        .collect()
}

/// Every string value of `field` in a JSON text (`"field": "value"`), escapes read (`\\`, `\"`,
/// `\/`; others kept as the character after the backslash). Enough for the stores' own files,
/// which hold paths.
pub fn json_strings(text: &str, field: &str) -> Vec<String> {
    let needle = format!("\"{field}\"");
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(&needle) {
        rest = &rest[at + needle.len()..];
        let after = rest.trim_start();
        let Some(after) = after.strip_prefix(':') else {
            continue;
        };
        let after = after.trim_start();
        let Some(after) = after.strip_prefix('"') else {
            continue;
        };
        let mut value = String::new();
        let mut chars = after.char_indices();
        let mut end = None;
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => {
                    end = Some(i + 1);
                    break;
                }
                '\\' => {
                    if let Some((_, next)) = chars.next() {
                        value.push(next);
                    }
                }
                other => value.push(other),
            }
        }
        let Some(end) = end else { break };
        out.push(value);
        rest = &after[end..];
    }
    out
}

/// The value of each `key:` line of a YAML text (Lutris's game files), quotes taken off.
pub fn yaml_values(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            (!value.is_empty()).then(|| value.to_string())
        })
        .collect()
}

/// `paths` without the ones that lead to a folder already listed (by the real folder: Steam's
/// `~/.steam/steam` is usually a link to `~/.local/share/Steam`), in order.
fn distinct(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut out = Vec::new();
    for path in paths {
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !seen.contains(&key) {
            seen.push(key);
            out.push(path);
        }
    }
    out
}

/// Where Steam keeps itself, whether there or not: `~/.steam/steam`, `~/.local/share/Steam` and
/// the Flathub Steam's.
fn steam_places(home: &Home) -> Vec<PathBuf> {
    let flatpak = home.home.join(".var/app/com.valvesoftware.Steam");
    vec![
        home.home.join(".steam/steam"),
        home.home.join(".steam/root"),
        home.home.join(".local/share/Steam"),
        flatpak.join(".local/share/Steam"),
        flatpak.join(".steam/steam"),
        flatpak.join("data/Steam"),
    ]
}

/// Steam's folders on this system that exist, each once.
pub fn steam_roots(home: &Home) -> Vec<PathBuf> {
    distinct(
        steam_places(home)
            .into_iter()
            .filter(|p| p.join("steamapps").is_dir())
            .collect(),
    )
}

/// Every Steam library: each Steam folder and the libraries its `libraryfolders.vdf` lists (in
/// `steamapps`, or the older `config`), each once, those that exist.
pub fn steam_libraries(home: &Home) -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    for root in steam_roots(home) {
        libraries.push(root.clone());
        for file in ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"] {
            if let Ok(text) = std::fs::read_to_string(root.join(file)) {
                libraries.extend(vdf_values(&text, "path").into_iter().map(PathBuf::from));
            }
        }
    }
    distinct(
        libraries
            .into_iter()
            .filter(|l| l.join("steamapps").is_dir())
            .collect(),
    )
}

/// The game's folders Steam's libraries hold: the folder each app's manifest names
/// (`appmanifest_<id>.acf`'s `installdir`) and `common/Fallout New Vegas`, matched without case.
/// Only folders that hold the game.
pub fn steam_games(home: &Home) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for library in steam_libraries(home) {
        let apps = library.join("steamapps");
        for app in STEAM_APPS {
            if let Ok(text) = std::fs::read_to_string(apps.join(format!("appmanifest_{app}.acf"))) {
                for dir in vdf_values(&text, "installdir") {
                    out.extend(resolve(&apps, &["common", &dir]));
                }
            }
        }
        out.extend(resolve(&apps, &["common", "Fallout New Vegas"]));
    }
    distinct(
        out.into_iter()
            .filter(|f| data_folder(f).is_some())
            .collect(),
    )
}

/// Heroic's configuration folders: its own and the Flathub Heroic's.
fn heroic_configs(home: &Home) -> Vec<PathBuf> {
    vec![
        home.config().join("heroic"),
        home.home
            .join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ]
}

/// The `.json` (or other `extension`) files in `dir`, in name order.
fn files_in(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case(extension))
        })
        .collect();
    files.sort();
    files
}

/// Games installed with Heroic (GOG and Epic) or with the Epic tool it uses (legendary): every
/// `install_path` their install lists name. Only folders that hold the game.
pub fn heroic_games(home: &Home) -> Vec<PathBuf> {
    let mut lists = Vec::new();
    for config in heroic_configs(home) {
        lists.push(config.join("gog_store/installed.json"));
        lists.push(config.join("legendaryConfig/legendary/installed.json"));
    }
    lists.push(home.config().join("legendary/installed.json"));
    let mut out = Vec::new();
    for list in lists {
        if let Ok(text) = std::fs::read_to_string(&list) {
            out.extend(
                json_strings(&text, "install_path")
                    .into_iter()
                    .map(PathBuf::from),
            );
        }
    }
    distinct(
        out.into_iter()
            .filter(|f| data_folder(f).is_some())
            .collect(),
    )
}

/// Lutris's game files (`games/*.yml`) in its own folders and the Flathub Lutris's.
fn lutris_files(home: &Home) -> Vec<PathBuf> {
    let flatpak = home.home.join(".var/app/net.lutris.Lutris");
    [
        home.config().join("lutris/games"),
        home.data().join("lutris/games"),
        flatpak.join("config/lutris/games"),
        flatpak.join("data/lutris/games"),
    ]
    .iter()
    .flat_map(|dir| files_in(dir, "yml"))
    .collect()
}

/// Games installed with Lutris: the folder of each game file's `exe` (a full path) and its
/// `game_path`. Only folders that hold the game.
pub fn lutris_games(home: &Home) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for file in lutris_files(home) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for exe in yaml_values(&text, "exe") {
            let exe = PathBuf::from(exe);
            if exe.has_root() {
                out.extend(exe.parent().map(Path::to_path_buf));
            }
        }
        out.extend(
            yaml_values(&text, "game_path")
                .into_iter()
                .map(PathBuf::from),
        );
    }
    distinct(
        out.into_iter()
            .filter(|f| data_folder(f).is_some())
            .collect(),
    )
}

/// A game folder found, with who installed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// "Steam", "Heroic" or "Lutris".
    pub store: &'static str,
    /// The game's folder (the one holding `Data`).
    pub folder: PathBuf,
}

/// Every game folder found, Steam's first, each once. Empty on macOS, where no store installs
/// the game: the player names the folder.
pub fn games(home: &Home) -> Vec<Found> {
    if home.system == System::MacOs {
        return vec![];
    }
    let mut out: Vec<Found> = Vec::new();
    let mut seen = Vec::new();
    for (store, folders) in [
        ("Steam", steam_games(home)),
        ("Heroic", heroic_games(home)),
        ("Lutris", lutris_games(home)),
    ] {
        for folder in folders {
            let key = std::fs::canonicalize(&folder).unwrap_or_else(|_| folder.clone());
            if !seen.contains(&key) {
                seen.push(key);
                out.push(Found { store, folder });
            }
        }
    }
    out
}

/// The game's `My Games/FalloutNV` folders inside a Wine prefix (Proton's, Heroic's, Lutris's):
/// one for each user in `drive_c/users` but `Public`, those that exist.
pub fn prefix_saves(prefix: &Path) -> Vec<PathBuf> {
    let Some(users) = resolve(prefix, &["drive_c", "users"]) else {
        return vec![];
    };
    let mut names: Vec<PathBuf> = std::fs::read_dir(&users)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            !e.file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("public")
        })
        .map(|e| e.path())
        .collect();
    names.sort();
    names
        .iter()
        .filter_map(|user| resolve(user, &["Documents", "My Games", "FalloutNV"]))
        .filter(|f| f.is_dir())
        .collect()
}

/// Where Proton keeps each Steam library's prefixes for the game: `steamapps/compatdata/<app
/// id>/pfx`, whether there or not.
fn proton_prefixes(home: &Home) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for library in steam_libraries(home) {
        for app in STEAM_APPS {
            out.push(library.join(format!("steamapps/compatdata/{app}/pfx")));
        }
    }
    out
}

/// The Wine prefixes Heroic's game settings (`GamesConfig/*.json`, `winePrefix`) and Lutris's
/// game files (`prefix`) name, whatever game they are for (only the ones holding the game's
/// saves count, [`save_folders`]).
fn other_prefixes(home: &Home) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for config in heroic_configs(home) {
        for file in files_in(&config.join("GamesConfig"), "json") {
            if let Ok(text) = std::fs::read_to_string(&file) {
                out.extend(
                    json_strings(&text, "winePrefix")
                        .into_iter()
                        .map(PathBuf::from),
                );
            }
        }
    }
    for file in lutris_files(home) {
        if let Ok(text) = std::fs::read_to_string(&file) {
            out.extend(yaml_values(&text, "prefix").into_iter().map(PathBuf::from));
        }
    }
    out.retain(|p| p.has_root());
    out
}

/// Saves found: the game's `My Games/FalloutNV` folder in a prefix, and whose prefix it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saves {
    /// "Steam (Proton)", "Heroic" or "Lutris".
    pub from: &'static str,
    /// The `My Games/FalloutNV` folder (its `Saves` holds the saves).
    pub folder: PathBuf,
}

/// Every `My Games/FalloutNV` folder found in Proton's prefixes for the game and in the prefixes
/// Heroic and Lutris name, each once. Only read, never written: nv-rs offers to COPY the saves
/// out of them on the player's yes (the maintainer, 2026-11-15: "Yes, copy only").
pub fn save_folders(home: &Home) -> Vec<Saves> {
    if home.system == System::MacOs {
        return vec![];
    }
    let mut out: Vec<Saves> = Vec::new();
    let mut seen = Vec::new();
    let mut add = |from: &'static str, folder: PathBuf| {
        let key = std::fs::canonicalize(&folder).unwrap_or_else(|_| folder.clone());
        if !seen.contains(&key) {
            seen.push(key);
            out.push(Saves { from, folder });
        }
    };
    for prefix in proton_prefixes(home) {
        for folder in prefix_saves(&prefix) {
            add("Steam (Proton)", folder);
        }
    }
    for prefix in other_prefixes(home) {
        for folder in prefix_saves(&prefix) {
            add("Wine prefix", folder);
        }
    }
    out
}

/// The XDG Documents folder from `user-dirs.dirs` (`XDG_DOCUMENTS_DIR="$HOME/..."` or a full
/// path), when it names one.
pub fn xdg_documents(home: &Home) -> Option<PathBuf> {
    let text = std::fs::read_to_string(home.config().join("user-dirs.dirs")).ok()?;
    let line = text
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("XDG_DOCUMENTS_DIR="))?;
    let value = line.trim().trim_matches('"');
    if let Some(rest) = value.strip_prefix("$HOME") {
        let rest = rest.trim_start_matches('/');
        return Some(if rest.is_empty() {
            home.home.clone()
        } else {
            home.home.join(rest)
        });
    }
    let path = PathBuf::from(value);
    path.has_root().then_some(path)
}

/// A folder nv-rs never writes into, but for one folder inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guarded {
    pub root: PathBuf,
    /// nv-rs's own folder when it lies inside `root` (macOS: `~/Library/Application Support/nv-rs`
    /// inside `~/Library`).
    pub except: Option<PathBuf>,
}

/// The player's own folders, which nv-rs never writes into (saves, settings, anything): what
/// Documents and AppData are on Windows. Linux: `~/Documents` and the XDG Documents folder; every
/// Steam folder and library (the game, Proton's prefixes with the player's Proton saves), the
/// Flathub Steam's whole folder, every game folder and Wine prefix with the game's saves found.
/// macOS: `~/Documents` and `~/Library` (but nv-rs's own folder in it, [`own_folder`]).
pub fn never_written(home: &Home) -> Vec<Guarded> {
    let plain = |root: PathBuf| Guarded { root, except: None };
    let mut out = vec![plain(home.home.join("Documents"))];
    match home.system {
        System::MacOs => out.push(Guarded {
            root: home.home.join("Library"),
            except: Some(own_folder(home)),
        }),
        System::Linux => {
            out.extend(xdg_documents(home).map(plain));
            out.push(plain(home.home.join(".steam")));
            out.push(plain(home.home.join(".var/app/com.valvesoftware.Steam")));
            out.extend(steam_places(home).into_iter().map(plain));
            out.extend(steam_libraries(home).into_iter().map(plain));
            out.extend(games(home).into_iter().map(|f| plain(f.folder)));
            for saves in save_folders(home) {
                out.push(plain(saves.folder));
            }
            out.extend(other_prefixes(home).into_iter().map(plain));
        }
    }
    out
}

/// The path the system really means: links resolved, a part that does not exist yet kept after
/// the real path of the longest part that does.
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

/// `path`'s parts, each without case (macOS's disks compare names without case; on Linux this
/// refuses a little more than it must, never less).
fn parts(path: &Path) -> Vec<String> {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

/// Whether `inner` is `outer` or lies inside it, by their parts.
fn inside(inner: &[String], outer: &[String]) -> bool {
    !outer.is_empty() && inner.len() >= outer.len() && inner[..outer.len()] == *outer
}

/// Why nv-rs may not write into `dir` (Linux and macOS): it is not a full path, it has a `..`,
/// or it lies inside one of `never` ([`never_written`]) but not inside that one's exception, by
/// its text or by the real path it leads to (a link doesn't get round this), parts compared
/// without case. The same rule as Windows' guard (host `save_out::refused`).
pub fn refused(dir: &Path, never: &[Guarded]) -> Option<&'static str> {
    if !dir.has_root() {
        return Some("not a full path");
    }
    if dir.components().any(|c| matches!(c, Component::ParentDir)) {
        return Some("a `..` in the path");
    }
    let places = [parts(dir), parts(&real_path(dir))];
    for guarded in never {
        let excepts: Vec<Vec<String>> = guarded
            .except
            .iter()
            .flat_map(|e| [parts(e), parts(&real_path(e))])
            .collect();
        for root in [parts(&guarded.root), parts(&real_path(&guarded.root))] {
            for place in &places {
                if inside(place, &root) && !excepts.iter().any(|e| inside(place, e)) {
                    return Some("inside the player's own folders");
                }
            }
        }
    }
    None
}

/// What `nv-viewer --find` prints, one line each, fields split by a tab: `own<TAB>FOLDER` (nv-rs's
/// own folder), `game<TAB>STORE<TAB>FOLDER` for each game found, `saves<TAB>FROM<TAB>FOLDER` for
/// each saves folder found. Read by `play.sh`.
pub fn report(home: &Home) -> String {
    let mut out = format!("own\t{}\n", own_folder(home).display());
    for found in games(home) {
        out.push_str(&format!(
            "game\t{}\t{}\n",
            found.store,
            found.folder.display()
        ));
    }
    for saves in save_folders(home) {
        out.push_str(&format!(
            "saves\t{}\t{}\n",
            saves.from,
            saves.folder.display()
        ));
    }
    out
}

#[cfg(test)]
mod tests;
