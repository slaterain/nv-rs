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
//!   `<steam>/steamapps/libraryfolders.vdf` (the newer `"path"` entries and the older flat numbered
//!   ones); Steam itself is in `~/.steam/steam` (usually a link to `~/.local/share/Steam`), in
//!   Flathub's `~/.var/app/com.valvesoftware.Steam/...`, or in the Snap's `~/snap/steam/common/...`.
//! - The game's `Documents\My Games\FalloutNV` (saves, Fallout.ini) is inside a Proton prefix:
//!   `<library>/steamapps/compatdata/<id>/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV`,
//!   the id 22380 or 22490, or a non-Steam shortcut's own id: every prefix there is looked in.
//! - GOG and Epic copies installed with Heroic, and copies installed with Lutris, are found only
//!   where those programs' own files name the folder, and only when that folder really holds the
//!   game (its `Data/FalloutNV.esm`): no folder name is guessed. Anything else: the player names
//!   the folder. Saves are also looked for in plain Wine's `~/.wine`, Bottles', PlayOnLinux's and
//!   CrossOver's prefixes (macOS: CrossOver's bottles).
//! - macOS: the game was never sold for macOS, so no store installs it there: the player copies a
//!   Windows installation's folder over and names it.
//!
//! Names are matched without case (`Data`, `FalloutNV.esm`, `My Games`), and every candidate is
//! tried: a Windows game copied to a Linux disk keeps whatever case its files had.
//!
//! The guard ([`refused`]): nv-rs never writes into the player's own folders ([`never_written`]),
//! and never into anything shaped like a Steam library's game or prefix folder or a Wine prefix's
//! users folder, whatever the lists say ([`by_shape`]). nv-rs's own folder ([`own_folder`]) is the
//! one exception inside any of them, and only as the real folder at its own place (a link there
//! opens nothing up).

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

/// Every entry of `dir` named `name` without case: the one of exactly that name first, then the
/// others in name order. Empty when there is none or `dir` can't be read.
pub fn find_children(dir: &Path, name: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.eq_ignore_ascii_case(name))
        .collect();
    names.sort();
    names.sort_by_key(|n| n != name);
    names.into_iter().map(|n| dir.join(n)).collect()
}

/// The first of [`find_children`].
pub fn find_child(dir: &Path, name: &str) -> Option<PathBuf> {
    find_children(dir, name).into_iter().next()
}

/// Every path `base` joined with `parts` leads to, each part matched without case
/// ([`find_children`]), every candidate followed.
pub fn resolve_all(base: &Path, parts: &[&str]) -> Vec<PathBuf> {
    let mut at = vec![base.to_path_buf()];
    for part in parts {
        at = at.iter().flat_map(|a| find_children(a, part)).collect();
    }
    at
}

/// The first of [`resolve_all`].
pub fn resolve(base: &Path, parts: &[&str]) -> Option<PathBuf> {
    resolve_all(base, parts).into_iter().next()
}

/// Whether `dir` holds the game's master as a file (any case, every candidate).
fn has_master(dir: &Path) -> bool {
    find_children(dir, MAIN_MASTER).iter().any(|f| f.is_file())
}

/// The game's `Data` folder when `folder` is the game's folder or its `Data` (it holds
/// `FalloutNV.esm`, any case): every folder named `Data` without case is tried.
pub fn data_folder(folder: &Path) -> Option<PathBuf> {
    if has_master(folder) {
        return Some(folder.to_path_buf());
    }
    find_children(folder, "Data")
        .into_iter()
        .find(|d| has_master(d))
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

/// A token of a Steam `.vdf` or `.acf` file.
#[derive(Debug, PartialEq)]
enum Token {
    Text(String),
    Open,
    Close,
}

/// The tokens of a `.vdf`: quoted strings (`\\` and `\"` read as `\` and `"`), `{` and `}`;
/// `//` comments and anything else skipped. An unterminated string ends the text.
fn vdf_tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push(Token::Open),
            '}' => out.push(Token::Close),
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => {
                            if let Some(next) = chars.next() {
                                s.push(next);
                            }
                        }
                        other => s.push(other),
                    }
                }
                if !closed {
                    break;
                }
                out.push(Token::Text(s));
            }
            _ => {}
        }
    }
    out
}

/// The `"key" "value"` pairs of a `.vdf`, read by position (a key, then its value or its block),
/// each with the depth of its block (the top-level object's own pairs are depth 1).
pub fn vdf_pairs(text: &str) -> Vec<(usize, String, String)> {
    let tokens = vdf_tokens(text);
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut at = 0;
    while at < tokens.len() {
        match &tokens[at] {
            Token::Text(key) => match tokens.get(at + 1) {
                Some(Token::Text(value)) => {
                    out.push((depth, key.clone(), value.clone()));
                    at += 2;
                }
                Some(Token::Open) => {
                    depth += 1;
                    at += 2;
                }
                _ => at += 1,
            },
            Token::Open => {
                depth += 1;
                at += 1;
            }
            Token::Close => {
                depth = depth.saturating_sub(1);
                at += 1;
            }
        }
    }
    out
}

/// The value of each `key` pair in a `.vdf`, any case, read by position.
pub fn vdf_values(text: &str, key: &str) -> Vec<String> {
    vdf_pairs(text)
        .into_iter()
        .filter(|(_, k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, _, v)| v)
        .collect()
}

/// The libraries a `libraryfolders.vdf` lists: each `"path"` value (the format since 2021) and each
/// numbered pair right under the top-level object (`"1" "/mnt/games/SteamLibrary"`, the older
/// format). Only full paths.
pub fn library_paths(text: &str) -> Vec<PathBuf> {
    vdf_pairs(text)
        .into_iter()
        .filter(|(depth, key, _)| {
            key.eq_ignore_ascii_case("path")
                || (*depth == 1 && !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(|(_, _, value)| PathBuf::from(value))
        .filter(|p| p.has_root())
        .collect()
}

/// Four hex digits at the front of `chars`, taken only when they are there: otherwise nothing is
/// taken (a broken escape never swallows the value's closing quote).
fn hex4(chars: &mut std::str::Chars<'_>) -> Option<u32> {
    let mut ahead = chars.clone();
    let mut code = 0;
    for _ in 0..4 {
        code = code * 16 + ahead.next()?.to_digit(16)?;
    }
    *chars = ahead;
    Some(code)
}

/// Every string value of `field` in a JSON text (`"field": "value"`), its escapes decoded
/// (`\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t` and `\uXXXX` with surrogate pairs, as Python's
/// json module writes a non-ASCII home folder). A value with a broken escape is left out.
pub fn json_strings(text: &str, field: &str) -> Vec<String> {
    let needle = format!("\"{field}\"");
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(&needle) {
        rest = &rest[at + needle.len()..];
        let Some(after) = rest.trim_start().strip_prefix(':') else {
            continue;
        };
        let Some(after) = after.trim_start().strip_prefix('"') else {
            continue;
        };
        let mut value = String::new();
        let mut chars = after.chars();
        let mut ok = true;
        let mut closed = false;
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    closed = true;
                    break;
                }
                '\\' => match chars.next() {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('r') => value.push('\r'),
                    Some('b') => value.push('\u{8}'),
                    Some('f') => value.push('\u{c}'),
                    Some('u') => {
                        let Some(high) = hex4(&mut chars) else {
                            ok = false;
                            continue;
                        };
                        let code = if (0xd800..0xdc00).contains(&high) {
                            // The low half's `\u` and digits, taken only when all are there.
                            let mut ahead = chars.clone();
                            let low = (ahead.next() == Some('\\') && ahead.next() == Some('u'))
                                .then(|| hex4(&mut ahead))
                                .flatten();
                            if low.is_some() {
                                chars = ahead;
                            }
                            match low {
                                Some(low) if (0xdc00..0xe000).contains(&low) => {
                                    0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
                                }
                                _ => {
                                    ok = false;
                                    continue;
                                }
                            }
                        } else {
                            high
                        };
                        match char::from_u32(code) {
                            Some(c) => value.push(c),
                            None => ok = false,
                        }
                    }
                    Some(other) => value.push(other),
                    None => break,
                },
                other => value.push(other),
            }
        }
        if !closed {
            break;
        }
        rest = chars.as_str();
        if ok {
            out.push(value);
        }
    }
    out
}

/// A YAML double-quoted scalar's text after its opening quote (several lines joined with `\n`):
/// the value (every escape YAML has decoded: `\\ \" \/ \  \n \t \r \0 \a \b \e \f \v \N \_ \L \P
/// \xNN \uNNNN \UNNNNNNNN`, the ones PyYAML writes included, a line break folded to a space, one
/// ending in `\` joined without one), or `None` when it never closes or an escape is broken or
/// unknown.
fn yaml_double_quoted(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\n' => {
                while chars.peek().is_some_and(|c| *c == ' ' || *c == '\t') {
                    chars.next();
                }
                out.push(' ');
            }
            '\\' => {
                fn code(
                    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
                    n: usize,
                ) -> Option<char> {
                    let digits: String = chars.take(n).collect();
                    if digits.len() != n {
                        return None;
                    }
                    u32::from_str_radix(&digits, 16)
                        .ok()
                        .and_then(char::from_u32)
                }
                match chars.next()? {
                    'n' => out.push('\n'),
                    't' | '\t' => out.push('\t'),
                    'r' => out.push('\r'),
                    '0' => out.push('\0'),
                    'a' => out.push('\u{7}'),
                    'b' => out.push('\u{8}'),
                    'e' => out.push('\u{1b}'),
                    'f' => out.push('\u{c}'),
                    'v' => out.push('\u{b}'),
                    'N' => out.push('\u{85}'),
                    '_' => out.push('\u{a0}'),
                    'L' => out.push('\u{2028}'),
                    'P' => out.push('\u{2029}'),
                    'x' => out.push(code(&mut chars, 2)?),
                    'u' => out.push(code(&mut chars, 4)?),
                    'U' => out.push(code(&mut chars, 8)?),
                    '\n' => {
                        while chars.peek().is_some_and(|c| *c == ' ' || *c == '\t') {
                            chars.next();
                        }
                    }
                    other @ (' ' | '"' | '/' | '\\') => out.push(other),
                    // Not an escape YAML has: the value is broken.
                    _ => return None,
                }
            }
            other => out.push(other),
        }
    }
    None
}

/// The value of each `key:` of a YAML text (Lutris's game files): a plain value with the lines
/// after it that are indented deeper folded on with spaces (PyYAML's folding of long values), a
/// single-quoted one (`''` read as `'`), or a double-quoted one with its escapes decoded (PyYAML
/// writes a non-ASCII home folder as `\xF6` and the like), over as many lines as it takes.
pub fn yaml_values(text: &str, key: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    fn indent(line: &str) -> usize {
        line.len() - line.trim_start().len()
    }
    let mut out = Vec::new();
    for (i, &line) in lines.iter().enumerate() {
        let Some(value) = line
            .trim_start()
            .strip_prefix(key)
            .and_then(|r| r.strip_prefix(':'))
        else {
            continue;
        };
        let value = value.trim();
        let deeper: Vec<&str> = lines[i + 1..]
            .iter()
            .copied()
            .take_while(|l: &&str| !l.trim().is_empty() && indent(l) > indent(line))
            .map(str::trim)
            .collect();
        let found = if let Some(quoted) = value.strip_prefix('"') {
            let all = std::iter::once(quoted)
                .chain(deeper.iter().copied())
                .collect::<Vec<_>>()
                .join("\n");
            yaml_double_quoted(&all)
        } else if let Some(quoted) = value.strip_prefix('\'') {
            let all = std::iter::once(quoted)
                .chain(deeper.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            let mut s = String::new();
            let mut chars = all.chars().peekable();
            let mut closed = None;
            while let Some(c) = chars.next() {
                if c == '\'' {
                    if chars.peek() == Some(&'\'') {
                        chars.next();
                        s.push('\'');
                    } else {
                        closed = Some(s.clone());
                        break;
                    }
                } else {
                    s.push(c);
                }
            }
            closed
        } else {
            let mut s = value.to_string();
            for more in &deeper {
                s.push(' ');
                s.push_str(more);
            }
            Some(s)
        };
        if let Some(found) = found.filter(|f| !f.is_empty()) {
            out.push(found);
        }
    }
    out
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

/// Where Steam keeps itself, whether there or not: `~/.steam/steam` and `~/.steam/root`,
/// `~/.local/share/Steam`, the Flathub Steam's and the Snap Steam's.
fn steam_places(home: &Home) -> Vec<PathBuf> {
    let flatpak = home.home.join(".var/app/com.valvesoftware.Steam");
    let snap = home.home.join("snap/steam/common");
    vec![
        home.home.join(".steam/steam"),
        home.home.join(".steam/root"),
        home.home.join(".local/share/Steam"),
        flatpak.join(".local/share/Steam"),
        flatpak.join(".steam/steam"),
        flatpak.join("data/Steam"),
        snap.join(".local/share/Steam"),
        snap.join(".steam/steam"),
    ]
}

/// Whole folders of Steam and of the Wine tools whose prefixes hold Windows games and their saves
/// (guarded whole): Steam's `~/.steam`, Flathub's and the Snap's Steam, plain Wine's `~/.wine`,
/// CrossOver's `~/.cxoffice`, PlayOnLinux's `~/.PlayOnLinux`, Bottles' (its own and Flathub's).
fn whole_folders(home: &Home) -> Vec<PathBuf> {
    [
        ".steam",
        ".var/app/com.valvesoftware.Steam",
        "snap/steam",
        ".wine",
        ".cxoffice",
        ".PlayOnLinux",
        ".var/app/com.usebottles.bottles",
    ]
    .iter()
    .map(|rel| home.home.join(rel))
    .chain([home.data().join("bottles")])
    .collect()
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

/// The folders in `dir`, in name order.
fn folders_in(dir: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    folders
}

/// Heroic's configuration folders: its own and the Flathub Heroic's.
fn heroic_configs(home: &Home) -> Vec<PathBuf> {
    vec![
        home.config().join("heroic"),
        home.home
            .join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ]
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

/// A game folder found, with who installed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// "Steam", "Heroic" or "Lutris".
    pub store: &'static str,
    /// The game's folder (the one holding `Data`).
    pub folder: PathBuf,
}

/// Saves found: the game's `My Games/FalloutNV` folder in a prefix, and whose prefix it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saves {
    /// "Steam (Proton)" or "Wine prefix".
    pub from: &'static str,
    /// The `My Games/FalloutNV` folder (its `Saves` holds the saves).
    pub folder: PathBuf,
}

/// One look at the player's folders: everything the finder and the guard need, each store's files
/// read once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scan {
    /// Steam's folders that exist, each once.
    pub steam_roots: Vec<PathBuf>,
    /// Every Steam library that exists, each once (the Steam folders first).
    pub libraries: Vec<PathBuf>,
    /// The game folders found, Steam's first, each once.
    pub games: Vec<Found>,
    /// The game's `My Games/FalloutNV` folders found in prefixes, each once.
    pub saves: Vec<Saves>,
    /// The Wine prefixes Heroic and Lutris name, and the Wine tools' prefixes that exist.
    pub prefixes: Vec<PathBuf>,
    /// What could not be read, for the player (a Steam library list that is there but unreadable).
    pub problems: Vec<String>,
}

/// The game's `My Games/FalloutNV` folders inside a Wine prefix: one for each user in
/// `drive_c/users` but `Public`, every case variant tried, those that are folders.
pub fn prefix_saves(prefix: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for users in resolve_all(prefix, &["drive_c", "users"]) {
        for user in folders_in(&users) {
            let public = user
                .file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("public"));
            if public {
                continue;
            }
            out.extend(
                resolve_all(&user, &["Documents", "My Games", "FalloutNV"])
                    .into_iter()
                    .filter(|f| f.is_dir()),
            );
        }
    }
    out
}

/// Looks at the player's folders once ([`Scan`]). macOS: no store installs the game there, so no
/// game is looked for; saves only in CrossOver's bottles.
pub fn scan(home: &Home) -> Scan {
    let mut scan = Scan::default();
    if home.system == System::MacOs {
        let bottles = home
            .home
            .join("Library/Application Support/CrossOver/Bottles");
        scan.prefixes = folders_in(&bottles);
        scan.saves = saves_in(&[], &scan.prefixes);
        return scan;
    }
    // Steam: its folders, their library lists, the game in each library.
    scan.steam_roots = distinct(
        steam_places(home)
            .into_iter()
            .filter(|p| p.join("steamapps").is_dir())
            .collect(),
    );
    let mut libraries = Vec::new();
    for root in &scan.steam_roots {
        libraries.push(root.clone());
        for file in ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"] {
            let path = root.join(file);
            if !path.exists() {
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => libraries.extend(library_paths(&String::from_utf8_lossy(&bytes))),
                Err(e) => scan
                    .problems
                    .push(format!("{} could not be read ({e})", path.display())),
            }
        }
    }
    scan.libraries = distinct(
        libraries
            .into_iter()
            .filter(|l| l.join("steamapps").is_dir())
            .collect(),
    );
    let mut steam = Vec::new();
    for library in &scan.libraries {
        let apps = library.join("steamapps");
        for app in STEAM_APPS {
            if let Ok(text) = std::fs::read_to_string(apps.join(format!("appmanifest_{app}.acf"))) {
                for dir in vdf_values(&text, "installdir") {
                    steam.extend(resolve_all(&apps, &["common", dir.as_str()]));
                }
            }
        }
        steam.extend(resolve_all(&apps, &["common", "Fallout New Vegas"]));
    }
    // Heroic (GOG, Epic) and legendary: every install_path their lists name.
    let mut heroic = Vec::new();
    let mut lists = Vec::new();
    for config in heroic_configs(home) {
        lists.push(config.join("gog_store/installed.json"));
        lists.push(config.join("legendaryConfig/legendary/installed.json"));
    }
    lists.push(home.config().join("legendary/installed.json"));
    for list in lists {
        if let Ok(text) = std::fs::read_to_string(&list) {
            heroic.extend(
                json_strings(&text, "install_path")
                    .into_iter()
                    .map(PathBuf::from),
            );
        }
    }
    // Lutris: each game file's exe (a relative one under its game_path) and game_path.
    let mut lutris = Vec::new();
    let mut named_prefixes = Vec::new();
    for file in lutris_files(home) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let game_path = yaml_values(&text, "game_path")
            .into_iter()
            .map(PathBuf::from)
            .find(|p| p.has_root());
        for exe in yaml_values(&text, "exe") {
            let exe = PathBuf::from(exe);
            let exe = match (&game_path, exe.has_root()) {
                (_, true) => exe,
                (Some(base), false) => base.join(exe),
                (None, false) => continue,
            };
            lutris.extend(exe.parent().map(Path::to_path_buf));
        }
        lutris.extend(game_path);
        named_prefixes.extend(yaml_values(&text, "prefix").into_iter().map(PathBuf::from));
    }
    for config in heroic_configs(home) {
        for file in files_in(&config.join("GamesConfig"), "json") {
            if let Ok(text) = std::fs::read_to_string(&file) {
                named_prefixes.extend(
                    json_strings(&text, "winePrefix")
                        .into_iter()
                        .map(PathBuf::from),
                );
            }
        }
    }
    let mut seen = Vec::new();
    for (store, folders) in [("Steam", steam), ("Heroic", heroic), ("Lutris", lutris)] {
        for folder in folders {
            if !folder.has_root() || data_folder(&folder).is_none() {
                continue;
            }
            let key = std::fs::canonicalize(&folder).unwrap_or_else(|_| folder.clone());
            if !seen.contains(&key) {
                seen.push(key);
                scan.games.push(Found { store, folder });
            }
        }
    }
    // Prefixes: every Proton prefix in every library (the game's ids and non-Steam shortcuts'),
    // the ones Heroic and Lutris name, and the Wine tools' own.
    let proton: Vec<PathBuf> = scan
        .libraries
        .iter()
        .flat_map(|l| folders_in(&l.join("steamapps/compatdata")))
        .map(|id| id.join("pfx"))
        .collect();
    let mut prefixes: Vec<PathBuf> = named_prefixes
        .into_iter()
        .filter(|p| p.has_root())
        .collect();
    prefixes.push(home.home.join(".wine"));
    prefixes.extend(folders_in(&home.home.join(".cxoffice")));
    prefixes.extend(folders_in(&home.home.join(".PlayOnLinux/wineprefix")));
    prefixes.extend(folders_in(&home.data().join("bottles/bottles")));
    prefixes.extend(folders_in(
        &home
            .home
            .join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
    ));
    scan.prefixes = prefixes;
    scan.saves = saves_in(&proton, &scan.prefixes);
    scan
}

/// The saves folders in Proton's prefixes (`proton`) and the other prefixes, each once.
fn saves_in(proton: &[PathBuf], others: &[PathBuf]) -> Vec<Saves> {
    let mut out: Vec<Saves> = Vec::new();
    let mut seen = Vec::new();
    let sources = proton
        .iter()
        .map(|p| ("Steam (Proton)", p))
        .chain(others.iter().map(|p| ("Wine prefix", p)));
    for (from, prefix) in sources {
        for folder in prefix_saves(prefix) {
            let key = std::fs::canonicalize(&folder).unwrap_or_else(|_| folder.clone());
            if !seen.contains(&key) {
                seen.push(key);
                out.push(Saves { from, folder });
            }
        }
    }
    out
}

/// Every game folder found ([`scan`]).
pub fn games(home: &Home) -> Vec<Found> {
    scan(home).games
}

/// Every saves folder found ([`scan`]). Only read, never written: nv-rs offers to COPY the saves
/// out of them on the player's yes (the maintainer, 2026-11-15: "Yes, copy only").
pub fn save_folders(home: &Home) -> Vec<Saves> {
    scan(home).saves
}

/// Every Steam library that exists ([`scan`]).
pub fn steam_libraries(home: &Home) -> Vec<PathBuf> {
    scan(home).libraries
}

/// Steam's folders that exist ([`scan`]).
pub fn steam_roots(home: &Home) -> Vec<PathBuf> {
    scan(home).steam_roots
}

/// The XDG Documents folder from `user-dirs.dirs` (`XDG_DOCUMENTS_DIR="$HOME/..."` or a full
/// path), when it names one. One that is the home folder itself or `/` is xdg-user-dirs' way of
/// saying there is no Documents folder: `None`.
pub fn xdg_documents(home: &Home) -> Option<PathBuf> {
    let text = std::fs::read_to_string(home.config().join("user-dirs.dirs")).ok()?;
    let line = text
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("XDG_DOCUMENTS_DIR="))?;
    let value = line.trim().trim_matches('"');
    let path = if let Some(rest) = value.strip_prefix("$HOME") {
        let rest = rest.trim_start_matches('/');
        if rest.is_empty() {
            return None;
        }
        home.home.join(rest)
    } else {
        PathBuf::from(value)
    };
    let off = parts(&path).is_empty()
        || parts(&path) == parts(&home.home)
        || path.components().all(|c| matches!(c, Component::RootDir));
    (path.has_root() && !off).then_some(path)
}

/// A folder nv-rs never writes into, but for nv-rs's own folder when it lies inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guarded {
    pub root: PathBuf,
    /// nv-rs's own folder ([`own_folder`]): taken only as the real folder at its own place.
    pub except: Option<PathBuf>,
    /// Whose rule names the exception: on Linux its names must match exactly (`NV-RS` is another
    /// folder there); macOS's disks ignore case, so there they are compared without it. The
    /// guarded folder itself is compared without case on both (refusing more, never less).
    pub system: System,
}

/// The player's own folders, which nv-rs never writes into (saves, settings, anything), each with
/// nv-rs's own folder as its one exception: what Documents and AppData are on Windows. Linux:
/// `~/Documents` and the XDG Documents folder; Steam's and the Wine tools' whole folders
/// ([`whole_folders`]), every Steam folder and library (the game, Proton's prefixes with the
/// player's Proton saves), every game folder, prefix and saves folder found. macOS: `~/Documents`
/// and `~/Library` (CrossOver's bottles and their saves found too). On macOS a prefix outside
/// `~/Library` (plain Wine's `~/.wine`, a Game Porting Toolkit or Homebrew Wine prefix, Heroic's
/// `~/Games/Heroic/Prefixes`) is not named: it is left to the `drive_c/users` shape ([`by_shape`],
/// which refuses every prefix's users folder, where the saves are). The game folder the port was
/// started with is guarded by the host whether this finds it or not (`host::save_out::prepare`).
pub fn never_written(home: &Home) -> Vec<Guarded> {
    never_written_from(home, &scan(home))
}

/// [`never_written`] from a [`scan`] already made.
pub fn never_written_from(home: &Home, scan: &Scan) -> Vec<Guarded> {
    let mut roots = vec![home.home.join("Documents")];
    match home.system {
        System::MacOs => roots.push(home.home.join("Library")),
        System::Linux => {
            roots.extend(xdg_documents(home));
            roots.extend(whole_folders(home));
            roots.extend(steam_places(home));
            roots.extend(scan.libraries.iter().cloned());
            roots.extend(scan.games.iter().map(|f| f.folder.clone()));
        }
    }
    roots.extend(scan.saves.iter().map(|s| s.folder.clone()));
    roots.extend(scan.prefixes.iter().cloned());
    let own = own_folder(home);
    roots
        .into_iter()
        .map(|root| Guarded {
            root,
            except: Some(own.clone()),
            system: home.system,
        })
        .collect()
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
/// refuses a little more than it must, never less), the Windows verbatim prefix (`\\?\`, which
/// canonicalize gives there) left out.
fn parts(path: &Path) -> Vec<String> {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| match c {
            Component::Prefix(p) => match p.kind() {
                std::path::Prefix::VerbatimDisk(d) | std::path::Prefix::Disk(d) => {
                    format!("{}:", (d as char).to_ascii_lowercase())
                }
                _ => c.as_os_str().to_string_lossy().to_lowercase(),
            },
            _ => c.as_os_str().to_string_lossy().to_lowercase(),
        })
        .collect()
}

/// Whether `inner` is `outer` or lies inside it, by their parts.
fn inside(inner: &[String], outer: &[String]) -> bool {
    !outer.is_empty() && inner.len() >= outer.len() && inner[..outer.len()] == *outer
}

/// Whether `parts` (without case) hold a Steam library's game or prefix folder
/// (`steamapps/common`, `steamapps/compatdata`) or a Wine prefix's users folder (`drive_c/users`):
/// refused whatever the lists say, so a library or prefix nobody listed is guarded too.
pub fn by_shape(parts: &[String]) -> bool {
    parts.windows(2).any(|pair| {
        matches!(
            (pair[0].as_str(), pair[1].as_str()),
            ("steamapps", "common") | ("steamapps", "compatdata") | ("drive_c", "users")
        )
    })
}

/// `path`'s parts exactly as named (each part's bytes, [`escape`]d so that no two names give the
/// same text): Linux's rule, where names differing only in case are different folders.
/// (A Windows drive, met only when the tests run there, is [`parts`]'s.)
fn exact_parts(path: &Path) -> Vec<String> {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| match c {
            Component::Prefix(_) => parts(Path::new(c.as_os_str())).concat(),
            _ => escape(&path_bytes(Path::new(c.as_os_str()))),
        })
        .collect()
}

/// Whether the place (`dir`, really `real_dir`) lies inside nv-rs's own folder `own` as the real
/// folder at its own place: `own`'s real path must be its parent's real path plus its own name (a
/// link AT the own folder opens nothing up), and the place must be inside it both by text and by
/// where it leads (a link inside it to elsewhere is not inside it). Names compared by `system`'s
/// rule: exactly on Linux, without case on macOS.
fn excepted(dir: &Path, real_dir: &Path, own: &Path, system: System) -> bool {
    let (Some(parent), Some(name)) = (own.parent(), own.file_name()) else {
        return false;
    };
    let key = match system {
        System::Linux => exact_parts,
        System::MacOs => parts,
    };
    let own_real = key(&real_path(own));
    own_real == key(&real_path(parent).join(name))
        && inside(&key(dir), &key(own))
        && inside(&key(real_dir), &own_real)
}

/// Why nv-rs may not write into `dir` (Linux and macOS): it is not a full path, it has a `..`, its
/// parts are shaped like a Steam library's or a Wine prefix's ([`by_shape`]), or it lies inside one
/// of `never` ([`never_written`]) but not inside that one's exception, by its text or by the real
/// path it leads to (a link doesn't get round this), parts compared without case.
pub fn refused(dir: &Path, never: &[Guarded]) -> Option<&'static str> {
    if !dir.has_root() {
        return Some("not a full path");
    }
    if dir.components().any(|c| matches!(c, Component::ParentDir)) {
        return Some(PARENT);
    }
    let real_dir = real_path(dir);
    let text = parts(dir);
    let real = parts(&real_dir);
    if by_shape(&text) || by_shape(&real) {
        return Some(SHAPED);
    }
    for guarded in never {
        let roots = [parts(&guarded.root), parts(&real_path(&guarded.root))];
        if !roots.iter().any(|r| inside(&text, r) || inside(&real, r)) {
            continue;
        }
        if let Some(own) = &guarded.except {
            if excepted(dir, &real_dir, own, guarded.system) {
                continue;
            }
        }
        return Some("inside the player's own folders");
    }
    None
}

/// [`refused`]'s reason for a path with a `..` in it.
const PARENT: &str = "a `..` in the path";

/// [`refused`]'s reason for a path shaped like a Steam library's or a Wine prefix's folders.
const SHAPED: &str = "inside a Steam library's or a Wine prefix's folders";

/// The player folders nv-rs's own folder lies inside (its exception keeps it writable, but the
/// player should know), and why the own folder itself would be refused, in plain words with what
/// to do: a `..` in its path, a path shaped like Steam's or Wine's folders, or (the one cause
/// left) a link at it or above it leading elsewhere.
pub fn own_folder_warnings(home: &Home, never: &[Guarded]) -> Vec<String> {
    let mut out = own_folder_notes(home, never);
    out.extend(own_folder_refusal(home, never));
    out
}

/// The player folders nv-rs's own folder lies inside (its exception keeps it writable, but the
/// player should know).
pub fn own_folder_notes(home: &Home, never: &[Guarded]) -> Vec<String> {
    let own = own_folder(home);
    let text = parts(&own);
    let real = parts(&real_path(&own));
    let mut out = Vec::new();
    for g in never {
        let root = [parts(&g.root), parts(&real_path(&g.root))];
        if root.iter().any(|r| inside(&text, r) || inside(&real, r)) {
            out.push(format!(
                "nv-rs's own folder {} lies inside {}, one of your own folders: nv-rs writes only into its own folder there",
                own.display(),
                g.root.display()
            ));
        }
    }
    out
}

/// The deepest folder play.sh writes into: the game's own save folder inside nv-rs's own folder
/// (`game/Documents/My Games/FalloutNV/Saves`, where the game finds its saves).
pub fn own_saves_folder(home: &Home) -> PathBuf {
    own_folder(home)
        .join("game")
        .join("Documents")
        .join("My Games")
        .join("FalloutNV")
        .join("Saves")
}

/// Why nv-rs's own folder may not be written at all, in plain words with the folder's name and
/// what to do (`--find`'s `refused` record: play.sh stops before its first write): checked at
/// the deepest folder written ([`own_saves_folder`]), so a link anywhere along the way counts.
pub fn own_folder_refusal(home: &Home, never: &[Guarded]) -> Option<String> {
    let own = own_folder(home);
    let why = refused(&own_saves_folder(home), never)?;
    {
        let set = match home.system {
            System::Linux => "XDG_DATA_HOME (or HOME)",
            System::MacOs => "HOME",
        };
        let cause = match why {
            PARENT => format!(
                "its path has a `..` in it, which nv-rs never writes through. Set {set} to a plain full path without `..`"
            ),
            SHAPED => format!(
                "its path lies inside what looks like a Steam library's or a Wine prefix's folders (steamapps/common, steamapps/compatdata or drive_c/users), which nv-rs never writes into. Set {set} to a folder outside those"
            ),
            _ => format!(
                "it leads into one of your own folders (it, or a folder above it, is a link to elsewhere). Make it a plain folder, or set {set} elsewhere"
            ),
        };
        Some(format!(
            "nv-rs's own folder {} cannot be written, so saves will NOT be kept: {cause}",
            own.display()
        ))
    }
}

/// Bytes kept as they are by [`escape`]: ASCII letters and digits, space and `/._-+,:@=~()`.
fn safe(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b" /._-+,:@=~()".contains(&b)
}

/// A field of `--find`'s output: every byte of `bytes` outside the safe set ([`safe`]) written
/// `%XX` (two upper-case hex digits), so no field holds a tab, a line break or anything a shell
/// reads specially, and a name that is not UTF-8 keeps its bytes. `play.sh` decodes it.
pub fn escape(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        if safe(b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A path's own bytes (on Linux and macOS exactly; elsewhere, where this only runs in tests, its
/// text).
pub fn path_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().into_owned().into_bytes()
    }
}

/// What `nv-viewer --find` prints, one record a line, its fields split by a tab, each path
/// [`escape`]d: `own<TAB>FOLDER` (nv-rs's own folder), `warning<TAB>TEXT` for each thing the
/// player should know (the own folder inside one of their folders, a Steam library list that
/// could not be read), `refused<TAB>TEXT` when nv-rs's own folder may not be written at all
/// ([`own_folder_refusal`]: play.sh stops before its first write and shows TEXT), `game<TAB>STORE<TAB>FOLDER` for each game found, `saves<TAB>FROM<TAB>FOLDER`
/// for each saves folder found. Read by `play.sh`.
pub fn report(home: &Home) -> String {
    let scan = scan(home);
    let never = never_written_from(home, &scan);
    let mut out = format!("own\t{}\n", escape(&path_bytes(&own_folder(home))));
    if let Some(why) = own_folder_refusal(home, &never) {
        out.push_str(&format!("refused\t{}\n", escape(why.as_bytes())));
    }
    for warning in own_folder_notes(home, &never)
        .into_iter()
        .chain(scan.problems.iter().cloned())
    {
        out.push_str(&format!("warning\t{}\n", escape(warning.as_bytes())));
    }
    for found in &scan.games {
        out.push_str(&format!(
            "game\t{}\t{}\n",
            found.store,
            escape(&path_bytes(&found.folder))
        ));
    }
    for saves in &scan.saves {
        out.push_str(&format!(
            "saves\t{}\t{}\n",
            escape(saves.from.as_bytes()),
            escape(&path_bytes(&saves.folder))
        ));
    }
    out
}

#[cfg(test)]
mod tests;
