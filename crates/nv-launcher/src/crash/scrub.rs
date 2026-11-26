//! What is taken out of a crash report before it is kept or sent: the player's home folder, user
//! name and computer name, save names, e-mail and network addresses, and every absolute path but
//! its part below the game's folder or nv-rs's own. The same on every system: a Windows report
//! read on Linux (or a path written with `/` on Windows) is scrubbed alike.
//!
//! 1. Known folders (the game's, nv-rs's own, the package's, OneDrive's, the home folder),
//!    longest first, matched literally in any case with any run of `\` or `/` between their
//!    parts (so the doubled backslashes of JSON and of Rust's `{:?}` match too): replaced by their
//!    placeholder, the part below kept (`<game>\Data\x.esm`). Below `<home>` and `<onedrive>`
//!    never the part right below it and only the last two parts (`<home>\...\Temp\x.tmp`), and a
//!    Steam account's number is `<id>`. A folder's rest ends at a space, a quote, `;`, `,`, `:`
//!    before a separator, and where another path or a web address starts.
//! 2. Save files (`.fos`, and the extender's `.nvse`): their names hold the character's name, the
//!    place and the play time, so the name goes (`<save>.fos`), in a path, quoted, or in a line
//!    such as `NV_SAVE_OUT: copied ...`; so does a character's own folder under `Saves`.
//! 3. Any other absolute path (`X:\...`, a share `\\host\...` or `//host/...` with its
//!    backslashes doubled or not, `smb://` and `file://` addresses, or `/...` at the start of a
//!    word): `<path>` and its last part only (`<path>\d3d9.dll`). A path goes on over a space
//!    when one of the next five words holds its separator (`C:\Program Files (x86)\x\y.dll`).
//! 4. E-mail addresses (`<e-mail>`), IPv4 and IPv6 addresses (`<ip>`), and Steam accounts written
//!    out (`<id>`): a Steam64 number (`7656119` and ten more digits) and a profile address's
//!    account part (`steamcommunity.com/profiles/<id>`, `steamcommunity.com/id/<id>`); versions,
//!    times and code paths stay.
//! 5. Then, outside the placeholders: the user names and the computer's names become `<user>` and
//!    `<computer>`, in any case: as a whole word or joined to digits, `_` or `-`
//!    (`<user>2.esp`); joined to letters only inside a path or a mod's file name and only when
//!    four characters or longer. Names of one or two characters, and names that are also ordinary
//!    words (`user`, `deck`, `fedora`: [`ORDINARY`]), go only inside paths. Never inside an exe
//!    address (eight hex digits), so a crash's addresses stay for grouping.

use std::path::Path;

/// A character of scrubbed text (and whether it is part of a path), or a placeholder.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Unit {
    Char(char, bool),
    Mark(&'static str),
}

/// A known folder: its parts (empty first part for `/home/x`, two for `\\host\share`) and its
/// placeholder.
struct Root {
    lead: usize,
    parts: Vec<Vec<char>>,
    mark: &'static str,
}

impl Root {
    /// The folder `path`, or none when it names no folder (empty, `/`, `C:\`).
    fn of(path: &str, mark: &'static str) -> Option<Root> {
        let chars: Vec<char> = path.trim().chars().collect();
        let lead = chars.iter().take_while(|&&c| is_separator(c)).count();
        let parts: Vec<Vec<char>> = chars[lead..]
            .split(|&c| is_separator(c))
            .filter(|p| !p.is_empty())
            .map(|p| p.to_vec())
            .collect();
        // A drive alone, or the file system's root, names nothing of the player's.
        let named = parts.len() + usize::from(lead > 0) >= 2
            && !(parts.len() == 1 && parts[0].len() == 2 && parts[0][1] == ':');
        named.then_some(Root { lead, parts, mark })
    }

    fn len(&self) -> usize {
        self.parts.iter().map(Vec::len).sum::<usize>() + self.parts.len() + self.lead
    }

    /// Where this folder's text ends when it starts at `text[at..]`, any case and any run of
    /// separators between its parts, ending on a whole part.
    fn matches(&self, text: &[char], at: usize) -> Option<usize> {
        let mut i = at;
        if self.lead > 0 {
            i = separators(text, i)?;
        }
        for (k, part) in self.parts.iter().enumerate() {
            if k > 0 {
                i = separators(text, i)?;
            }
            for &want in part {
                if !text.get(i).is_some_and(|&c| same(c, want)) {
                    return None;
                }
                i += 1;
            }
        }
        ends_here(&text[i..]).then_some(i)
    }
}

/// A name to take out, its placeholder, and whether it is an ordinary word too.
struct Name {
    chars: Vec<char>,
    mark: &'static str,
    ordinary: bool,
}

/// Names this long or longer go joined to letters too, inside paths and mods' file names.
const JOINED_FROM: usize = 4;

/// User and computer names that are also ordinary words (Windows' and the distributions'
/// defaults, the Steam Deck's user): taken out only inside paths.
pub const ORDINARY: &[&str] = &[
    "admin",
    "administrator",
    "arch",
    "bazzite",
    "cachyos",
    "computer",
    "deck",
    "debian",
    "default",
    "desktop",
    "fedora",
    "game",
    "gamer",
    "guest",
    "home",
    "laptop",
    "linux",
    "localhost",
    "mac",
    "macbook",
    "manjaro",
    "mint",
    "nobara",
    "owner",
    "pc",
    "player",
    "root",
    "steam",
    "steamdeck",
    "test",
    "ubuntu",
    "user",
    "windows",
];

impl Name {
    /// Whether this name starts at `text[i]` (`in_path`: which characters are part of a path, or
    /// of a mod's file name). Joined to digits, `_` or `-` a name goes anywhere; joined to
    /// letters only inside a path (four characters or more: `jtester.esp` in a path, never
    /// `Johnny` in a line for `John`). One or two characters, and ordinary words, go only inside
    /// paths. Never inside an exe address (eight hex digits): `00412345` stays for `12345`.
    fn at(&self, text: &[char], in_path: &[bool], i: usize, file_name: bool) -> bool {
        let n = self.chars.len();
        if text.len() - i < n || !self.chars.iter().zip(&text[i..]).all(|(&a, &b)| same(a, b)) {
            return false;
        }
        let path = in_path[i..i + n].iter().all(|&p| p);
        if (self.ordinary || n <= 2) && !path {
            return false;
        }
        // A mod's file name is no folder of the player's: an ordinary word stays there
        // (`Steam Deck Controls.esp` for the user `deck`).
        if self.ordinary && file_name {
            return false;
        }
        // The word around it (letters and digits).
        let mut start = i;
        while start > 0 && text[start - 1].is_alphanumeric() {
            start -= 1;
        }
        let mut end = i + n;
        while end < text.len() && text[end].is_alphanumeric() {
            end += 1;
        }
        // Inside an exe address, unless the name is that whole word (a staff number `20231234`).
        if end - start == 8 && n < 8 && text[start..end].iter().all(char::is_ascii_hexdigit) {
            return false;
        }
        let letter = |c: Option<&char>| c.is_some_and(|c| c.is_alphabetic());
        let joined_to_letters =
            letter(i.checked_sub(1).map(|k| &text[k])) || letter(text.get(i + n));
        if !joined_to_letters {
            return true;
        }
        path && n >= JOINED_FROM && !self.ordinary
    }
}

/// The scrubber of one report: the folders and names it takes out.
pub struct Scrubber {
    roots: Vec<Root>,
    names: Vec<Name>,
}

/// What a report must not show, as read from the system ([`Known::from_system`]) or made up in a
/// test.
#[derive(Default)]
pub struct Known {
    /// Folders and their placeholders (`<game>`, `<nv-rs>`, `<nv-rs-data>`, `<home>`).
    pub roots: Vec<(String, &'static str)>,
    /// Names and their placeholders (`<user>`, `<computer>`).
    pub words: Vec<(String, &'static str)>,
}

impl Known {
    /// This system's: the home folder and the user's and computer's names as the system gives
    /// them, with the game's folders `games`, the package's (the launcher's own folder)
    /// `program` and nv-rs's own folder `own`.
    pub fn from_system(games: &[&Path], program: &Path, own: &Path) -> Known {
        let mut k = Known::default();
        for game in games {
            k.folder(game, "<game>");
        }
        // Windows' own folder is the package's `userdata`: shown below `<nv-rs>` as it is.
        if !own.starts_with(program) {
            k.folder(own, "<nv-rs-data>");
        }
        k.folder(program, "<nv-rs>");
        let var = |name: &str| std::env::var_os(name).map(|v| v.to_string_lossy().into_owned());
        let mut homes: Vec<String> = ["USERPROFILE", "HOME"]
            .iter()
            .filter_map(|n| var(n))
            .collect();
        if let (Some(drive), Some(path)) = (var("HOMEDRIVE"), var("HOMEPATH")) {
            homes.push(format!("{drive}{path}"));
        }
        for home in &homes {
            let path = Path::new(home);
            k.folder(path, "<home>");
            if let Some(name) = path.file_name() {
                k.words
                    .push((name.to_string_lossy().into_owned(), "<user>"));
            }
            #[cfg(windows)]
            if let Some(short) = crate::play::win::short_path(path) {
                k.folder(&short, "<home>");
            }
        }
        // OneDrive's folders (a work one is named after the company), each its own known folder.
        for name in ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"] {
            if let Some(dir) = var(name) {
                k.folder(Path::new(&dir), "<onedrive>");
            }
        }
        for name in ["USERNAME", "USER", "LOGNAME"] {
            if let Some(user) = var(name) {
                k.words.push((user, "<user>"));
            }
        }
        for computer in super::system::computer_names() {
            k.words.push((computer, "<computer>"));
        }
        k
    }

    /// `path` and, when it differs, its real path (links and junctions followed).
    fn folder(&mut self, path: &Path, mark: &'static str) {
        let text = path.to_string_lossy().into_owned();
        if let Ok(real) = std::fs::canonicalize(path) {
            let real = real.to_string_lossy().into_owned();
            let real = real
                .strip_prefix("\\\\?\\UNC\\")
                .map(|r| format!("\\\\{r}"))
                .or_else(|| real.strip_prefix("\\\\?\\").map(str::to_string))
                .unwrap_or(real);
            if real != text {
                self.roots.push((real, mark));
            }
        }
        self.roots.push((text, mark));
    }
}

impl Scrubber {
    pub fn new(known: &Known) -> Scrubber {
        let mut roots: Vec<Root> = known
            .roots
            .iter()
            .filter_map(|(path, mark)| Root::of(path, mark))
            .collect();
        // Longest first: the game's folder inside the home folder is the game's.
        roots.sort_by_key(|r| std::cmp::Reverse(r.len()));
        let mut names: Vec<Name> = known
            .words
            .iter()
            .map(|(w, mark)| Name {
                chars: w.trim().chars().collect(),
                mark,
                ordinary: ORDINARY.contains(&w.trim().to_lowercase().as_str()),
            })
            .filter(|n| !n.chars.is_empty())
            .collect();
        names.sort_by_key(|n| std::cmp::Reverse(n.chars.len()));
        names.dedup_by(|a, b| {
            a.chars.len() == b.chars.len()
                && a.chars.iter().zip(&b.chars).all(|(&x, &y)| same(x, y))
        });
        Scrubber { roots, names }
    }

    /// `text` with everything above taken out.
    pub fn text(&self, text: &str) -> String {
        self.scrub(text, false)
    }

    /// A file's name (a mod's plugin) with everything above taken out, read as a path's part: a
    /// name joined to letters goes too (`jtestPatch.esp`).
    pub fn file_name(&self, text: &str) -> String {
        self.scrub(text, true)
    }

    fn scrub(&self, text: &str, all_path: bool) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut units = self.known_folders(&chars);
        if all_path {
            for u in &mut units {
                if let Unit::Char(_, p) = u {
                    *p = true;
                }
            }
        }
        units = runs(units, save_names);
        units = runs(units, character_folders);
        units = runs(units, other_paths);
        units = runs(units, addresses);
        units = runs(units, |text, in_path, _| {
            self.names_in(text, in_path, all_path)
        });
        let mut out = String::with_capacity(text.len());
        for u in units {
            match u {
                Unit::Char(c, _) => out.push(c),
                Unit::Mark(m) => out.push_str(m),
            }
        }
        out
    }

    fn known_folders(&self, text: &[char]) -> Vec<Unit> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < text.len() {
            let starts = i == 0 || !is_name_char(text[i - 1]);
            // OneDrive's folder only as a whole name: `OneDrive - Contoso Ltd2` is a work folder,
            // not the personal `OneDrive` (but `OneDrive (synced)` and `... Ltd (in ...)` are).
            let found = if starts {
                self.roots.iter().find_map(|r| {
                    r.matches(text, i)
                        .filter(|&end| {
                            r.mark != "<onedrive>" || !text[end..].starts_with(&[' ', '-', ' '])
                        })
                        .map(|end| (end, r.mark))
                })
            } else {
                None
            };
            let Some((end, mark)) = found else {
                out.push(Unit::Char(text[i], false));
                i += 1;
                continue;
            };
            out.push(Unit::Mark(mark));
            let home = mark == "<home>" || mark == "<onedrive>";
            // The part below, a path's rest: kept (below the home folder, its last two parts).
            // Below home a part named `OneDrive - <a company>` runs on whatever its name holds
            // (commas, many words), so the company always goes ([`onedrive_part`]).
            let below = if text.get(end).is_some_and(|&c| is_separator(c)) {
                let body = if home { onedrive_part(text, end) } else { end };
                extent(text, end, body, None)
            } else {
                end
            };
            let part = character_folder_in(&text[end..below]);
            let part = if home { last_two(&part) } else { part };
            out.extend(part.into_iter().map(|c| Unit::Char(c, true)));
            i = below;
        }
        out
    }

    fn names_in(&self, text: &[char], in_path: &[bool], file_name: bool) -> Vec<Unit> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < text.len() {
            match self
                .names
                .iter()
                .find(|n| n.at(text, in_path, i, file_name))
            {
                Some(n) => {
                    out.push(Unit::Mark(n.mark));
                    i += n.chars.len();
                }
                None => {
                    out.push(Unit::Char(text[i], in_path[i]));
                    i += 1;
                }
            }
        }
        out
    }
}

/// Each run of characters between placeholders through `pass` (its text, which of its
/// characters are part of a path, and the character before it: a placeholder's last, `>`).
fn runs(
    units: Vec<Unit>,
    mut pass: impl FnMut(&[char], &[bool], Option<char>) -> Vec<Unit>,
) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut text = Vec::new();
    let mut in_path = Vec::new();
    let mut before = None;
    let mut flush = |text: &mut Vec<char>, in_path: &mut Vec<bool>, out: &mut Vec<Unit>, before| {
        if !text.is_empty() {
            out.extend(pass(text, in_path, before));
            text.clear();
            in_path.clear();
        }
    };
    for u in units {
        match u {
            Unit::Char(c, p) => {
                text.push(c);
                in_path.push(p);
            }
            Unit::Mark(m) => {
                flush(&mut text, &mut in_path, &mut out, before);
                out.push(u);
                before = m.chars().last();
            }
        }
    }
    flush(&mut text, &mut in_path, &mut out, before);
    out
}

const BACKSLASH: char = '\\';

fn is_separator(c: char) -> bool {
    c == '\\' || c == '/'
}

/// A run of one or more separators at `text[i..]`: where it ends.
fn separators(text: &[char], i: usize) -> Option<usize> {
    let n = text[i.min(text.len())..]
        .iter()
        .take_while(|&&c| is_separator(c))
        .count();
    (n > 0).then_some(i + n)
}

/// Characters a file or folder name goes on with (a folder `nv-rs` is not the start of
/// `nv-rs2` or `nv-rs.old`).
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '~' | '$')
}

/// Whether a known folder's text may end before `rest`: at a separator, a space, the end, or
/// punctuation (a full stop only at a sentence's end).
fn ends_here(rest: &[char]) -> bool {
    match rest.first() {
        None => true,
        Some(&'.') => rest.get(1).is_none_or(|&c| !is_name_char(c)),
        Some(&c) => !is_name_char(c),
    }
}

fn same(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// Characters that end a path.
fn ends_path(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | '|' | '*' | '?')
}

/// How many words a path goes on over (folders with spaces: `OneDrive - Some Company Ltd`).
const LOOK_AHEAD: usize = 5;

/// Where a path whose name starts at `body` (the whole path at `start`) ends: at a space or a
/// quote, but on over up to five words while one of them holds the separator `sep` (any
/// separator for `None`), the sentence's punctuation after it left out.
fn extent(text: &[char], start: usize, body: usize, sep: Option<char>) -> usize {
    let holds = |w: &[char]| match sep {
        Some(s) => w.contains(&s),
        None => w.iter().any(|&c| is_separator(c)),
    };
    // A word of the path ends at a space or a quote, and at `;` and `,` (lists of paths), and at
    // `:` before a separator (Unix lists).
    let word_end = |from: usize| {
        let mut k = from;
        while k < text.len() {
            let c = text[k];
            // A comma ends it only where a path starts right after (a folder may hold one).
            let path_next = |k: usize| {
                let at = |d: usize| text.get(k + d).copied();
                at(0).is_some_and(is_separator)
                    || (at(0).is_some_and(|c| c.is_ascii_alphabetic())
                        && at(1) == Some(':')
                        && at(2).is_some_and(is_separator))
            };
            let list = c == ';'
                || (c == ',' && path_next(k + 1))
                || (c == ':' && text.get(k + 1).is_some_and(|&d| is_separator(d)));
            if ends_path(c) || list {
                break;
            }
            k += 1;
        }
        k
    };
    let mut end = word_end(body);
    loop {
        let mut k = end;
        let mut extended = None;
        for _ in 0..LOOK_AHEAD {
            if text.get(k) != Some(&' ') {
                break;
            }
            // A run of spaces is one gap (`Jo  Hero\Private`).
            let from = k + text[k..].iter().take_while(|&&c| c == ' ').count();
            let stop = word_end(from);
            // Another path's start ends this one (`... and G:\x`, `... or /tmp/y`). A web address
            // never extends it: its word ends at `:` before `//` (`word_end`), holding no separator.
            let word = &text[from..stop];
            if word.is_empty()
                || is_separator(word[0])
                || (word.len() > 2 && word[0].is_ascii_alphabetic() && word[1] == ':')
            {
                break;
            }
            if holds(word) {
                extended = Some(stop);
                break;
            }
            k = stop;
        }
        match extended {
            Some(stop) => end = stop,
            None => break,
        }
    }
    while end > body {
        let c = text[end - 1];
        let open = text[start..end].iter().filter(|&&x| x == '(').count();
        let close = text[start..end].iter().filter(|&&x| x == ')').count();
        if matches!(c, '.' | ',' | ';' | ':') || (c == ')' && close > open) {
            end -= 1;
        } else {
            break;
        }
    }
    end
}

/// Where a home folder's rest goes on from, its separators at `at`: past a part named
/// `OneDrive - <...>` (a work OneDrive, `OneDrive - Contoso, Ltd`) whatever its name holds, up to
/// its next separator, a quote or `|`, `: `, another path or address ([`starts_other`]) or the
/// line's end; else at `at`.
fn onedrive_part(text: &[char], at: usize) -> usize {
    let name = at + text[at..].iter().take_while(|&&c| is_separator(c)).count();
    let word: Vec<char> = "OneDrive - ".chars().collect();
    let named = text.len() - name >= word.len()
        && word.iter().zip(&text[name..]).all(|(&a, &b)| same(a, b));
    if !named {
        return at;
    }
    let mut k = name + word.len();
    while k < text.len() {
        let c = text[k];
        if is_separator(c)
            || (ends_path(c) && !matches!(c, ' ' | '\''))
            || (c == ':' && text.get(k + 1) == Some(&' '))
            || (c == ' ' && starts_other(text, k + 1))
        {
            break;
        }
        k += 1;
    }
    // Spaces before what ends it are the line's.
    while k > name + word.len() && text[k - 1] == ' ' {
        k -= 1;
    }
    k
}

/// Whether the word at `text[i..]` starts something other than a folder's name: a path (a drive,
/// a separator, a share), an address (`smb://`, `https://`, an e-mail, an IPv4 address) or a
/// stop's call path (`(in A < B)`).
fn starts_other(text: &[char], i: usize) -> bool {
    let word = &text[i..i + text[i..].iter().take_while(|c| !c.is_whitespace()).count()];
    let at = |d: usize| word.get(d).copied();
    at(0).is_some_and(is_separator)
        || (at(0).is_some_and(|c| c.is_ascii_alphabetic())
            && at(1) == Some(':')
            && at(2).is_some_and(is_separator))
        || word.windows(3).any(|w| w == [':', '/', '/'])
        || word.contains(&'@')
        || word == ['(', 'i', 'n']
        || ipv4_from(text, i).is_some()
}

/// A path's rest below the home folder (or OneDrive's): never the part directly below it (a
/// work OneDrive's company, an account's folder), and of the rest only the last two parts:
/// `\...\Temp\x.tmp`, `\...\Desktop`; a rest of one part is `\...`. A Steam account's number (a
/// part of six digits or more) is `<id>`.
fn last_two(part: &[char]) -> Vec<char> {
    let parts: Vec<&[char]> = part
        .split(|&c| is_separator(c))
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return part.to_vec();
    }
    let lead: Vec<char> = part
        .iter()
        .take_while(|&&c| is_separator(c))
        .copied()
        .collect();
    let sep = if lead.is_empty() { vec!['\\'] } else { lead };
    let mut out = sep.clone();
    out.extend("...".chars());
    let from = parts.len().saturating_sub(2).max(1);
    for p in &parts[from..] {
        out.extend(&sep);
        if p.len() >= 6 && p.iter().all(char::is_ascii_digit) {
            out.extend("<id>".chars());
        } else {
            out.extend_from_slice(p);
        }
    }
    if parts.len() > 1 && part.last().is_some_and(|&c| is_separator(c)) {
        out.extend(&sep);
    }
    out
}

/// The length of a save file's extension at `text[i..]` (`.fos`, `.nvse`, any case, not part of
/// a longer word), if one is there.
fn save_extension(text: &[char], i: usize) -> Option<usize> {
    [".fos", ".nvse"].iter().find_map(|ext| {
        let n = ext.chars().count();
        let here = text.len() - i >= n && ext.chars().zip(&text[i..]).all(|(a, &b)| same(a, b));
        (here && !text.get(i + n).is_some_and(|c| c.is_alphanumeric())).then_some(n)
    })
}

/// save_out's word before a save's name (`NV_SAVE_OUT: copied <name>`).
const COPIED: [char; 7] = [' ', 'c', 'o', 'p', 'i', 'e', 'd'];

/// Where the save name ending at `ext` starts (not before `from`): a quoted name after the quote
/// that opens it; else after a separator, a double quote, a tab, `<`, `>` or `|`, after `: `, `= `
/// or `copied ` (save_out's `NV_SAVE_OUT: copied <name>`), else at `from`. Save names have
/// spaces in them, and a character's name can have an apostrophe or brackets.
fn save_start(text: &[char], ext: usize, from: usize) -> usize {
    // Quoted (`"Jo Hero: run.fos"`): from the quote that opens it.
    let closing = text.get(
        ext + text[ext..]
            .iter()
            .take_while(|c| c.is_alphanumeric() || **c == '.')
            .count(),
    );
    if let Some(&q) = closing.filter(|&&q| q == '"' || q == '\'') {
        // The quote that opens it starts a word (an apostrophe inside the name does not), and is
        // not escaped (`{:?}` prints a name's own `"` as `\"`).
        let opens = |p: usize| {
            text[p] == q
                && (p == from || !(text[p - 1].is_alphanumeric() || text[p - 1] == BACKSLASH))
        };
        if let Some(open) = (from..ext).rev().find(|&p| opens(p)) {
            return open + 1;
        }
    }
    let mut s = ext;
    while s > from {
        let c = text[s - 1];
        if is_separator(c) || matches!(c, '"' | '`' | '\t' | '\n' | '<' | '>' | '|') {
            break;
        }
        if c == ' ' && s >= 2 {
            let head = &text[..s - 1];
            if matches!(head.last(), Some(':' | '=')) || head.ends_with(&COPIED) {
                break;
            }
        }
        s -= 1;
    }
    // Spaces before the name are the line's, and so are a list's comma and a bracket the name
    // does not close (`(Save 3   Jo Hero.fos, SAVE 3   JO HERO.fos)`).
    loop {
        let unclosed = matches!(text.get(s), Some('(' | '['))
            && !text[s..ext].iter().any(|&c| c == ')' || c == ']');
        if s < ext && (matches!(text[s], ' ' | ',' | ';') || unclosed) {
            s += 1;
        } else {
            break;
        }
    }
    s
}

/// Save names in a run: `<save>` and the extension. A folder of its own under `Saves` (the
/// game's `SLocalSavePath=Saves\<character>\`) is the character's name too: `<save>` as well.
fn save_names(text: &[char], in_path: &[bool], _: Option<char>) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut from = 0;
    // A name starts after the last one's extension at the earliest (and the walk back is short).
    let mut bound = 0;
    let mut i = 0;
    while i < text.len() {
        let Some(n) = save_extension(text, i) else {
            i += 1;
            continue;
        };
        let start = save_start(text, i, bound);
        if start < i {
            let folder = character_folder(text, bound, start);
            let keep_to = folder.map_or(start, |(f, _)| f);
            out.extend((from..keep_to).map(|k| Unit::Char(text[k], in_path[k])));
            if let Some((_, end)) = folder {
                out.push(Unit::Mark("<save>"));
                out.extend((end..start).map(|k| Unit::Char(text[k], in_path[k])));
            }
            out.push(Unit::Mark("<save>"));
            from = i;
        }
        i += n;
        bound = i;
    }
    out.extend((from..text.len()).map(|k| Unit::Char(text[k], in_path[k])));
    out
}

/// The folder a save's name at `start` is in, when it is a folder of its own under `Saves`: its
/// start and end in `text` (at or after `from`).
fn character_folder(text: &[char], from: usize, start: usize) -> Option<(usize, usize)> {
    // A run of separators counts as one (Rust's `{:?}` and JSON double them).
    let back = |mut k: usize| {
        while k > from && is_separator(text[k - 1]) {
            k -= 1;
        }
        k
    };
    let end = back(start);
    if end == start {
        return None;
    }
    let begin = from + text[from..end].iter().rposition(|&c| is_separator(c))? + 1;
    let parent_end = back(begin);
    let parent_begin = from
        + text[from..parent_end]
            .iter()
            .rposition(|&c| is_separator(c))
            .map_or(0, |p| p + 1);
    let parent: String = text[parent_begin..parent_end].iter().collect();
    (parent.eq_ignore_ascii_case("saves") && begin < end).then_some((begin, end))
}

/// A known folder's rest with a character's own save folder (`Saves\<character>\...`) as
/// `<save>`: below the home folder, before its rest is cut to its last two parts (the folder at
/// the rest's end is [`character_folders`]').
fn character_folder_in(part: &[char]) -> Vec<char> {
    let mut parts: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < part.len() {
        while i < part.len() && is_separator(part[i]) {
            i += 1;
        }
        let s = i;
        while i < part.len() && !is_separator(part[i]) {
            i += 1;
        }
        if i > s {
            parts.push((s, i));
        }
    }
    let saves = (0..parts.len()).find(|&k| {
        let (s, e) = parts[k];
        k + 2 < parts.len() && is_saves(&part[s..e])
    });
    let Some(k) = saves else {
        return part.to_vec();
    };
    let (s, e) = parts[k + 1];
    let mut out = part[..s].to_vec();
    out.extend("<save>".chars());
    out.extend_from_slice(&part[e..]);
    out
}

/// Whether a path's part is the game's `Saves` folder (any case).
fn is_saves(part: &[char]) -> bool {
    part.iter()
        .collect::<String>()
        .eq_ignore_ascii_case("saves")
}

/// A character's own save folder where it ends a path, right after its `Saves` (a save in it is
/// `<save>` already: [`save_names`]; one in the middle of a path goes with the path's cut), as
/// the host's `files: <folder> holds ...` lines print it on Linux and macOS: `<save>`, its name
/// up to ` holds `, `: `, `, `, `; `, a quote, another path or address, ` (in ` or the line's end
/// (a name has spaces).
fn character_folders(text: &[char], in_path: &[bool], before: Option<char>) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let prev = if i == 0 { before } else { Some(text[i - 1]) };
        let saves = prev.is_none_or(|c| is_separator(c) || !is_name_char(c))
            && text.len() - i > 5
            && is_saves(&text[i..i + 5])
            && is_separator(text[i + 5]);
        // The path's own separators: those of the word that holds this `Saves`.
        let own = || -> Vec<char> {
            let from = text[..i]
                .iter()
                .rposition(|c| c.is_whitespace())
                .map_or(0, |p| p + 1);
            [BACKSLASH, '/']
                .into_iter()
                .filter(|s| text[from..=i + 5].contains(s))
                .collect()
        };
        let found = saves
            .then(|| folder_at_end(text, i + 5, &own(), false))
            .flatten();
        let Some((s, end)) = found else {
            out.push(Unit::Char(text[i], in_path[i]));
            i += 1;
            continue;
        };
        out.extend((i..s).map(|k| Unit::Char(text[k], in_path[k])));
        out.push(Unit::Mark("<save>"));
        i = end;
    }
    out
}

/// The folder name after the separators at `text[at..]` when it ends its path: its start and
/// end ([`character_folders`]' end rules); none when more of the path follows it, or it names
/// nothing (`Saves\*`, `Saves\.`). `own` holds the path's own separators (a later word with
/// only another kind is the line's: `Saves\Jo Hero 3/4 done`); `nested` when it is the folder
/// after a later `Saves` of the same name's.
fn folder_at_end(text: &[char], at: usize, own: &[char], nested: bool) -> Option<(usize, usize)> {
    let s = at + text[at..].iter().take_while(|&&c| is_separator(c)).count();
    // `Saves\ folder`: no name right after it.
    if text.get(s).is_none_or(|c| c.is_whitespace()) {
        return None;
    }
    let next_is = |k: usize, word: &str| {
        let w: Vec<char> = word.chars().collect();
        text[k..].starts_with(&w)
    };
    // Another character's folder where it ends its own path (`..., Saves/Benny and Saves/Jo
    // Hero holds ...`); not a folder named `Old Saves` with more of the path after it.
    let saves_next = |k: usize| {
        !nested
            && text.len() - k > 5
            && is_saves(&text[k..k + 5])
            && is_separator(text[k + 5])
            && folder_at_end(text, k + 5, own, true).is_some()
    };
    // A later word with a separator, but none of the path's own kind.
    let foreign = |k: usize| {
        let word = text[k..].iter().take_while(|c| !c.is_whitespace());
        let seps: Vec<char> = word.filter(|&&c| is_separator(c)).copied().collect();
        !seps.is_empty() && !seps.iter().any(|c| own.contains(c))
    };
    let mut end = s;
    while end < text.len() {
        let c = text[end];
        // A space inside the name goes on (`Saves\Jo Hero\Private`: the separator then says
        // whether the path goes on); another path's start, an address, ` holds ` or a word
        // with only another kind of separator end it.
        let other = c == ' '
            && (next_is(end, " holds ")
                || starts_other(text, end + 1)
                || saves_next(end + 1)
                || foreign(end + 1));
        if is_separator(c)
            || (ends_path(c) && !matches!(c, ' ' | '\''))
            || (matches!(c, ':' | ',' | ';') && text.get(end + 1) == Some(&' '))
            || other
        {
            break;
        }
        end += 1;
    }
    // Spaces before what ends it are the line's.
    while end > s && text[end - 1] == ' ' {
        end -= 1;
    }
    // More of the path after it: not where it ends.
    let past = end + text[end..].iter().take_while(|&&c| is_separator(c)).count();
    let more = past > end && text.get(past).is_some_and(|&c| !ends_path(c));
    let names = text[s..end].iter().any(|c| c.is_alphanumeric());
    (names && !more).then_some((s, end))
}

/// Whether a `/` (or a `\\` of a share) after `before` opens a path: at the text's start or after
/// a space, a quote, an opening bracket, `=`, `,` or `;` (after `:` only as `path_at` says, so web
/// addresses stay).
fn opens_path(before: Option<char>) -> bool {
    before.is_none_or(|c| {
        c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '(' | '[' | '{' | '=' | ',' | ';')
    })
}

/// The absolute path starting at `text[i]` (whose character before is `before`): its separator,
/// its end, where its name starts, and whether it is a share or an address (whose first part is
/// a host), or none. After `:` (a Unix list, `path:\host`) a path opens with one `/` or a
/// backslash share, never with `//` (web addresses).
fn path_at(text: &[char], i: usize, before: Option<char>) -> Option<(char, usize, usize, bool)> {
    let after_colon = before == Some(':');
    let at = |k: usize| text.get(i + k).copied();
    // A share: two or more of one separator (four backslashes in Rust's `{:?}`), then its host;
    // or the verbatim `\?\UNC\` form, its backslashes doubled or not.
    let run = |from: usize, s: char| {
        text[(i + from).min(text.len())..]
            .iter()
            .take_while(|&&c| c == s)
            .count()
    };
    let share = |s: char| {
        let r = run(0, s);
        if r < 2 {
            return None;
        }
        if at(r).is_some_and(char::is_alphanumeric) {
            return Some(r);
        }
        let r2 = run(r + 1, s);
        let unc = s == BACKSLASH
            && at(r) == Some('?')
            && r2 > 0
            && text[i + r + 1 + r2..].starts_with(&['U', 'N', 'C'])
            && at(r + 1 + r2 + 3) == Some(s);
        // The path's name starts after `UNC` (its `?` would end it).
        unc.then_some(r + 1 + r2 + 3)
    };
    let scheme = ["smb://", "file://"].iter().find_map(|s| {
        let n = s.chars().count();
        // With a host after it (`file:///C:/...` is a local path, found by its drive).
        (text.len() - i > n
            && s.chars().zip(&text[i..]).all(|(a, &b)| same(a, b))
            && text[i + n].is_alphanumeric())
        .then_some(n)
    });
    let (sep, body, host) = if at(0).is_some_and(|c| c.is_ascii_alphabetic())
        && at(1) == Some(':')
        && at(2).is_some_and(is_separator)
        && before.is_none_or(|c| !c.is_alphanumeric())
    {
        (at(2)?, i + 3, false)
    } else if let (Some(r), true) = (share(BACKSLASH), opens_path(before) || after_colon) {
        (BACKSLASH, i + r, true)
    } else if let (Some(r), true) = (share('/'), opens_path(before)) {
        ('/', i + r, true)
    } else if let (Some(n), true) = (scheme, before.is_none_or(|c| !c.is_alphanumeric())) {
        ('/', i + n, true)
    } else if at(0) == Some('/')
        && at(1).is_some_and(|c| !is_separator(c) && !ends_path(c))
        && (opens_path(before) || after_colon)
    {
        return Some(('/', extent(text, i, i + 1, Some('/')), i + 1, false));
    } else {
        return None;
    };
    Some((sep, extent(text, i, body, Some(sep)), body, host))
}

/// Every absolute path in a run that is not the rest of a known folder: `<path>` and its last
/// part (and its last separator when it names a folder).
fn other_paths(text: &[char], in_path: &[bool], before: Option<char>) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let prev = if i == 0 { before } else { Some(text[i - 1]) };
        let found = (!in_path[i]).then(|| path_at(text, i, prev)).flatten();
        let Some((sep, end, body, share)) = found else {
            out.push(Unit::Char(text[i], in_path[i]));
            i += 1;
            continue;
        };
        out.push(Unit::Mark("<path>"));
        let path = &text[i..end];
        // A share or an address keeps a last part only past its host (`\host` alone is `<path>`).
        let past_host = text[body..end]
            .split(|&c| is_separator(c))
            .filter(|p| !p.is_empty())
            .count()
            >= 2;
        let last = path
            .split(|&c| is_separator(c))
            .rfind(|p| !p.is_empty())
            .filter(|p| !(p.len() == 2 && p[1] == ':'))
            .filter(|_| !share || past_host);
        if let Some(last) = last {
            out.push(Unit::Char(sep, true));
            if last.len() >= 6 && last.iter().all(char::is_ascii_digit) {
                // A Steam account's number (`userdata\123456789`).
                out.push(Unit::Mark("<id>"));
            } else {
                out.extend(last.iter().map(|&c| Unit::Char(c, true)));
            }
            if path.last().is_some_and(|&c| is_separator(c)) {
                out.push(Unit::Char(sep, true));
            }
        }
        i = end;
    }
    out
}

fn is_email_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

/// An e-mail address starting at `text[i]`: where it ends.
fn email_at(text: &[char], i: usize) -> Option<usize> {
    if i > 0 && is_email_char(text[i - 1]) {
        return None;
    }
    let local = text[i..].iter().take_while(|&&c| is_email_char(c)).count();
    if local == 0 || text.get(i + local) != Some(&'@') {
        return None;
    }
    let from = i + local + 1;
    let mut end = from
        + text[from..]
            .iter()
            .take_while(|&&c| c.is_alphanumeric() || c == '.' || c == '-')
            .count();
    while end > from && text[end - 1] == '.' {
        end -= 1;
    }
    let domain: String = text[from..end].iter().collect();
    let tld = domain.rsplit('.').next().unwrap_or("");
    (domain.contains('.') && tld.len() >= 2 && tld.chars().all(char::is_alphabetic)).then_some(end)
}

/// An IPv4 address (four numbers 0 to 255) starting at `text[i]`: where it ends.
fn ipv4_at(text: &[char], i: usize) -> Option<usize> {
    let joined = |c: char| c.is_alphanumeric() || c == '.' || c == '_';
    if i > 0 && joined(text[i - 1]) {
        return None;
    }
    // A version is no address: the whole word right before it (`version 1.3.250.0`, `driver:
    // 31.0.101.5`; `v1.0.0.1` is joined, above). Not `server 10.0.0.5`.
    let mut k = i;
    while k > 0 && matches!(text[k - 1], ' ' | ':' | '=') {
        k -= 1;
    }
    let word_start = k - text[..k]
        .iter()
        .rev()
        .take_while(|c| c.is_alphabetic())
        .count();
    let word: String = text[word_start..k]
        .iter()
        .collect::<String>()
        .to_lowercase();
    if ["version", "ver", "driver", "build", "v"].contains(&word.as_str()) {
        return None;
    }
    ipv4_from(text, i)
}

/// Four numbers 0 to 255 joined by `.` at `text[i..]`, not part of a longer number: where they
/// end.
fn ipv4_from(text: &[char], i: usize) -> Option<usize> {
    let mut k = i;
    for n in 0..4 {
        if n > 0 {
            if text.get(k) != Some(&'.') {
                return None;
            }
            k += 1;
        }
        let digits = text[k..]
            .iter()
            .take(4)
            .take_while(|c| c.is_ascii_digit())
            .count();
        let number: String = text[k..k + digits].iter().collect();
        if !(1..=3).contains(&digits) || number.parse::<u32>().ok()? > 255 {
            return None;
        }
        k += digits;
    }
    let after_joined = text.get(k).is_some_and(|&c| {
        c.is_alphanumeric()
            || c == '_'
            || (c == '.' && text.get(k + 1).is_some_and(char::is_ascii_digit))
    });
    (!after_joined).then_some(k)
}

/// An IPv6 address starting at `text[i]` (hex groups of at most four digits with `::`, or all
/// eight): where it ends. Times (`12:34:56`) and Rust paths (`a::b`) are not.
fn ipv6_at(text: &[char], i: usize) -> Option<usize> {
    let joined = |c: char| c.is_alphanumeric() || c == '_' || c == ':' || c == '.';
    if i > 0 && joined(text[i - 1]) {
        return None;
    }
    let n = text[i..]
        .iter()
        .take_while(|&&c| c.is_ascii_hexdigit() || c == ':')
        .count();
    let end = i + n;
    if text
        .get(end)
        .is_some_and(|&c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    let run: String = text[i..end].iter().collect();
    let groups: Vec<&str> = run.split(':').collect();
    let fits = groups.iter().all(|g| g.len() <= 4)
        && run.chars().any(|c| c.is_ascii_hexdigit())
        && !run.contains(":::")
        && (run.contains("::") || groups.len() == 8);
    // Code, not an address: two names of hex letters (`face::add`, `d3d9::d3d9`), unless the first
    // is an address's own start: link-local `fe80`-`febf`, unique local `fc`/`fd`, multicast `ff`,
    // or a global `2xxx`/`3xxx` (`fe80::beef` is an address).
    let named: Vec<&&str> = groups.iter().filter(|g| !g.is_empty()).collect();
    let address_start = |g: &str| {
        let g = g.to_ascii_lowercase();
        g.len() == 4
            && (["fe8", "fe9", "fea", "feb"]
                .iter()
                .any(|p| g.starts_with(p))
                || ["fc", "fd", "ff"].iter().any(|p| g.starts_with(p))
                || g.starts_with(['2', '3']))
    };
    let code = named.len() == 2
        && !address_start(named[0])
        && named
            .iter()
            .all(|g| g.chars().any(|c| c.is_ascii_alphabetic()));
    if !fits || n < 3 || code {
        return None;
    }
    // An IPv4 address at its end (`::ffff:192.168.1.20`).
    if text.get(end) == Some(&'.') {
        let last = i + run.rfind(':').map_or(0, |p| p + 1);
        if let Some(v4) = ipv4_from(text, last) {
            return Some(v4);
        }
    }
    Some(end)
}

/// A Steam account's number written out at `text[i]`: a Steam64 id (`7656119` and ten more
/// digits, as every account's is), not part of a longer word or number: where it ends.
fn steam_id_at(text: &[char], i: usize) -> Option<usize> {
    let joined = |c: &char| c.is_alphanumeric() || *c == '_';
    let end = i + 17;
    let digits = text.get(i..end)?;
    let id = digits.iter().all(char::is_ascii_digit)
        && digits.starts_with(&['7', '6', '5', '6', '1', '1', '9'])
        && !(i > 0 && joined(&text[i - 1]))
        && !text.get(end).is_some_and(joined);
    id.then_some(end)
}

/// A Steam profile's address at `text[i]`, `steamcommunity.com/profiles/<number>` or
/// `steamcommunity.com/id/<name>` (any case): where its account part starts and ends.
fn steam_profile_at(text: &[char], i: usize) -> Option<(usize, usize)> {
    let from = ["steamcommunity.com/profiles/", "steamcommunity.com/id/"]
        .iter()
        .find_map(|lead| {
            let from = i + lead.len();
            let at = text.get(i..from)?;
            at.iter()
                .zip(lead.chars())
                .all(|(&c, l)| same(c, l))
                .then_some(from)
        })?;
    let n = text[from..]
        .iter()
        .take_while(|&&c| c.is_alphanumeric() || c == '_' || c == '-')
        .count();
    (n > 0).then_some((from, from + n))
}

/// E-mail and network addresses and Steam accounts in a run.
fn addresses(text: &[char], in_path: &[bool], _: Option<char>) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if let Some((from, end)) = steam_profile_at(text, i) {
            out.extend((i..from).map(|k| Unit::Char(text[k], in_path[k])));
            out.push(Unit::Mark("<id>"));
            i = end;
        } else if let Some(end) = email_at(text, i) {
            out.push(Unit::Mark("<e-mail>"));
            i = end;
        } else if let Some(end) = steam_id_at(text, i) {
            out.push(Unit::Mark("<id>"));
            i = end;
        } else if let Some(end) = ipv4_at(text, i).or_else(|| ipv6_at(text, i)) {
            out.push(Unit::Mark("<ip>"));
            i = end;
        } else {
            out.push(Unit::Char(text[i], in_path[i]));
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up player: home `D:\Profiles\Jo Tester` (Windows) or `/home/jtest`, user names
    /// `jtest` and `Jo Tester`, computer `JT-DESK`, the game in `E:\Steam\steamapps\common\Fallout
    /// New Vegas`, nv-rs in `F:\Games\nv-rs`.
    fn player() -> Scrubber {
        Scrubber::new(&Known {
            roots: vec![
                (
                    "E:\\Steam\\steamapps\\common\\Fallout New Vegas".into(),
                    "<game>",
                ),
                ("F:\\Games\\nv-rs".into(), "<nv-rs>"),
                ("/home/jtest/.local/share/nv-rs".into(), "<nv-rs-data>"),
                ("D:\\Profiles\\Jo Tester".into(), "<home>"),
                ("/home/jtest".into(), "<home>"),
            ],
            words: vec![
                ("jtest".into(), "<user>"),
                ("Jo Tester".into(), "<user>"),
                ("JT-DESK".into(), "<computer>"),
            ],
        })
    }

    #[test]
    fn the_home_folder_goes_in_any_case_and_with_any_separators() {
        let s = player();
        assert_eq!(
            s.text("temp D:\\Profiles\\Jo Tester\\AppData\\Local\\Temp\\x.tmp"),
            "temp <home>\\...\\Temp\\x.tmp"
        );
        // The part right below home never stays.
        assert_eq!(s.text("d:/profiles/jo tester/Desktop"), "<home>/...");
        // Doubled backslashes (JSON, Rust's {:?}) and a mix of both separators.
        assert_eq!(
            s.text("path: \"D:\\\\Profiles\\\\Jo Tester\\\\x.ini\""),
            "path: \"<home>\\\\...\""
        );
        assert_eq!(s.text("D:\\Profiles/Jo Tester\\/y/z"), "<home>\\/...\\/z");
        // The verbatim form Windows answers for a real path.
        assert_eq!(
            s.text("\\\\?\\D:\\Profiles\\Jo Tester\\z\\w"),
            "\\\\?\\<home>\\...\\w"
        );
        assert_eq!(s.text("/home/jtest/.cache/x"), "<home>/.../x");
        // Another folder that only starts like it is not it; the name in it still goes.
        assert_eq!(s.text("/home/jtest2/x"), "<path>/x");
        assert_eq!(s.text("in /home/jtest2"), "in <path>/<user>2");
    }

    /// A Mac player, as [`Known::from_system`] finds one: a home in the Mac's users folder
    /// (`/Users/<name>`, here `jtest`), nv-rs's own folder `~/Library/Application Support/nv-rs`,
    /// the game in a Steam folder of the home: every one taken out, and another user's home too.
    /// (The paths are put together here: the repository's hygiene test refuses a user's home
    /// written out, made-up ones too.)
    #[test]
    fn a_mac_players_folders_go() {
        let mac = |rest: &str| format!("/{}/{rest}", "Users");
        let s = Scrubber::new(&Known {
            roots: vec![
                (
                    mac("jtest/Library/Application Support/Steam/steamapps/common/Fallout New Vegas"),
                    "<game>",
                ),
                (mac("jtest/Library/Application Support/nv-rs"), "<nv-rs-data>"),
                (mac("jtest/Downloads/nv-rs-preview-macos-arm64"), "<nv-rs>"),
                (mac("jtest"), "<home>"),
            ],
            words: vec![("jtest".into(), "<user>")],
        });
        let log = format!(
            "Log: {}",
            mac("jtest/Library/Application Support/nv-rs/logs/game.log")
        );
        assert_eq!(s.text(&log), "Log: <nv-rs-data>/logs/game.log");
        assert_eq!(
            s.text(&mac(
                "jtest/Library/Application Support/Steam/steamapps/common/Fallout New Vegas/Data/x.esm"
            )),
            "<game>/Data/x.esm"
        );
        assert_eq!(
            s.text(&format!(
                "at {}",
                mac("jtest/Downloads/nv-rs-preview-macos-arm64/app/nv-viewer")
            )),
            "at <nv-rs>/app/nv-viewer"
        );
        let desktop = mac("jtest/Desktop/notes");
        assert_eq!(s.text(&desktop), "<home>/.../notes");
        // Another user's home: no part of it stays but its last.
        assert_eq!(s.text(&mac("someone/Documents/x")), "<path>/x");
        for line in [s.text(&log), s.text(&desktop), s.text("jtest's Mac")] {
            assert!(!line.contains("jtest"), "{line}");
        }
    }

    /// Below the home folder (and OneDrive's) never the part right below it (a work OneDrive
    /// names the employer) and only the last two parts; a Steam account's number is `<id>`.
    #[test]
    fn below_the_home_folder_only_its_last_two_parts_stay() {
        let s = player();
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd\\Documents\\notes\\x.txt read"),
            "<home>\\...\\notes\\x.txt read"
        );
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd\\Desktop"),
            "<home>\\...\\Desktop"
        );
        assert_eq!(
            s.text("/home/jtest/.steam/steam/userdata/123456789/config/x.vdf"),
            "<home>/.../config/x.vdf"
        );
        assert_eq!(
            s.text("/home/jtest/.local/share/Steam/userdata/123456789/"),
            "<home>/.../userdata/<id>/"
        );
        assert_eq!(s.text("/home/jtest/a/b/"), "<home>/.../b/");
        assert_eq!(s.text("/home/jtest/a/b/c/"), "<home>/.../b/c/");
        // OneDrive's folder is known as it is, however many words its name has.
        let k = Scrubber::new(&Known {
            roots: vec![
                (
                    "D:\\Profiles\\Jo Tester\\OneDrive - The University of Somewhere".into(),
                    "<onedrive>",
                ),
                ("D:\\Profiles\\Jo Tester".into(), "<home>"),
            ],
            words: Vec::new(),
        });
        assert_eq!(
            k.text("D:\\Profiles\\Jo Tester\\OneDrive - The University of Somewhere\\Documents\\My Games\\FalloutNV\\x.ini"),
            "<onedrive>\\...\\FalloutNV\\x.ini"
        );
        assert_eq!(
            k.text("at D:\\Profiles\\Jo Tester\\OneDrive - The University of Somewhere"),
            "at <onedrive>"
        );
    }

    /// A known folder's rest ends where a list goes on (`;`, `,`) or a web address starts.
    #[test]
    fn a_known_folders_rest_ends_at_a_list_or_an_address() {
        let s = player();
        assert_eq!(
            s.text("F:\\Games\\nv-rs\\userdata\\x;D:\\Private Stuff\\tax 2026\\y.pdf"),
            "<nv-rs>\\userdata\\x;<path>\\y.pdf"
        );
        assert_eq!(
            s.text("file:///D:/Profiles/Jo Tester/x/y and https://jtest.example.com/x"),
            "file:///<home>/.../y and https://<user>.example.com/x"
        );
    }

    #[test]
    fn the_game_and_nv_rs_folders_keep_their_part_below() {
        let s = player();
        assert_eq!(
            s.text(
                "data E:\\Steam\\steamapps\\common\\Fallout New Vegas\\Data\\FalloutNV.esm read"
            ),
            "data <game>\\Data\\FalloutNV.esm read"
        );
        assert_eq!(
            s.text("log F:\\Games\\nv-rs\\userdata\\logs\\game-1.log."),
            "log <nv-rs>\\userdata\\logs\\game-1.log."
        );
        assert_eq!(
            s.text("Log: /home/jtest/.local/share/nv-rs/logs/game.log"),
            "Log: <nv-rs-data>/logs/game.log"
        );
        assert_eq!(s.text("F:\\Games\\nv-rs.old\\x"), "<path>\\x");
    }

    /// A save's name holds the character's name, the place and the play time: it goes, in
    /// save_out's lines and in paths, the extension kept.
    #[test]
    fn save_names_go() {
        let s = player();
        assert_eq!(
            s.text("NV_SAVE_OUT: copied Save 12   Jo Hero  Goodsprings  01 02 03.fos"),
            "NV_SAVE_OUT: copied <save>.fos"
        );
        assert_eq!(
            s.text("NV_SAVE_OUT: copied Save 12   Jo Hero  Goodsprings  01 02 03.nvse (deleted)"),
            "NV_SAVE_OUT: copied <save>.nvse (deleted)"
        );
        assert_eq!(
            s.text("F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Save 3   Jo Hero  Primm  00 10 59.fos"),
            "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save>.fos"
        );
        assert_eq!(
            s.text("loading \"Save 4 Jo Hero.FOS\" now"),
            "loading \"<save>.FOS\" now"
        );
        assert_eq!(
            s.text("in G:\\x\\Save 5 Jo.fos"),
            "in <path>\\x\\<save>.fos"
        );
        // Not a save: another extension.
        assert_eq!(s.text("the fossil.fosx stays"), "the fossil.fosx stays");
        // A character's name with an apostrophe or brackets, a quoted name with a colon.
        assert_eq!(
            s.text("NV_SAVE_OUT: copied systemsave D'Arcy Tester.fos"),
            "NV_SAVE_OUT: copied <save>.fos"
        );
        assert_eq!(s.text("Jo's (best) run.fos"), "<save>.fos");
        assert_eq!(
            s.text("hidden \"Jo Hero: run.fos\" here"),
            "hidden \"<save>.fos\" here"
        );
        // A character's own folder under Saves (SLocalSavePath=Saves\<character>\).
        assert_eq!(
            s.text("F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Jo Hero\\Save 1.fos"),
            "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save>\\<save>.fos"
        );
        // Many in one long line: each its own.
        let many = "a.fos ".repeat(2000);
        assert_eq!(s.text(&many), "<save>.fos ".repeat(2000));
    }

    #[test]
    fn other_absolute_paths_keep_only_their_last_part() {
        let s = player();
        assert_eq!(
            s.text("loaded C:\\Windows\\System32\\vulkan-1.dll, ok"),
            "loaded <path>\\vulkan-1.dll, ok"
        );
        assert_eq!(
            s.text("from C:\\Program Files (x86)\\Some Tool\\tool.exe (version 2)"),
            "from <path>\\tool.exe (version 2)"
        );
        assert_eq!(
            s.text("mounted at /mnt/big disk/games/x.esp"),
            "mounted at <path>/x.esp"
        );
        assert_eq!(
            s.text("share \\\\server\\share\\a b\\c.txt"),
            "share <path>\\c.txt"
        );
        assert_eq!(s.text("share //nas-1/games/x.esp"), "share <path>/x.esp");
        // Rust's {:?} doubles a share's backslashes, its verbatim form too; smb:// and file://.
        assert_eq!(
            s.text("open \"\\\\\\\\JT-NAS\\\\Games\\\\Mods\\\\x.esp\""),
            "open \"<path>\\x.esp\""
        );
        assert_eq!(
            s.text("open \"\\\\\\\\?\\\\UNC\\\\JT-NAS\\\\Games\\\\y.esp\""),
            "open \"<path>\\y.esp\""
        );
        assert_eq!(s.text("at smb://jt-nas/games"), "at <path>/games");
        assert_eq!(s.text("at file://JT-NAS/games/x.esp"), "at <path>/x.esp");
        assert_eq!(s.text("(C:\\x\\y.log)"), "(<path>\\y.log)");
        assert_eq!(s.text("C:\\"), "<path>");
        assert_eq!(s.text("in D:\\a\\b\\"), "in <path>\\b\\");
        // Not paths: web addresses, relative source paths, fractions.
        let kept = "see https://github.com/slaterain/nv-rs/issues; panicked at nv-rs/crates/host/src/lib.rs:12:5; 1/2";
        assert_eq!(s.text(kept), kept);
    }

    #[test]
    fn e_mail_and_network_addresses_go() {
        let s = player();
        assert_eq!(
            s.text("mail jo.tester+fnv@mail.example.org, then"),
            "mail <e-mail>, then"
        );
        assert_eq!(s.text("from 192.168.1.20:8080 ok"), "from <ip>:8080 ok");
        assert_eq!(s.text("v6 fe80::1ff:fe23:4567:890a here"), "v6 <ip> here");
        assert_eq!(s.text("[2001:db8::1]"), "[<ip>]");
        assert_eq!(
            s.text("1:2:3:4:5:6:7:8 is eight groups"),
            "<ip> is eight groups"
        );
        // Not addresses: versions with a number over 255, times, Rust paths, a lone @.
        let kept = "FalloutNV.exe 1.4.0.525, build 26200.7171, at 12:34:56, std::fs::read, a @ b";
        assert_eq!(s.text(kept), kept);
        // An IPv4 address inside IPv6, an e-mail address that is not ASCII.
        assert_eq!(s.text("from ::ffff:192.168.1.20 now"), "from <ip> now");
        assert_eq!(
            s.text("mail j\u{f6}ran.tester@ex\u{e4}mple.se, ok"),
            "mail <e-mail>, ok"
        );
        // Versions with every number 255 or less, and code paths of hex letters, stay.
        let kept = "Vulkan version 1.3.250.0, driver 31.0.101.5, v1.0.0.1, face::add, d3d9::d3d9";
        assert_eq!(s.text(kept), kept);
    }

    /// A Steam account written out, as a player might in an F12 note: its Steam64 number, or its
    /// profile's address with the number or the account's own name.
    #[test]
    fn steam_accounts_written_out_go() {
        let s = player();
        assert_eq!(
            s.text("my steam is 76561198012345678, add me"),
            "my steam is <id>, add me"
        );
        assert_eq!(
            s.text("https://steamcommunity.com/profiles/76561198012345678/ and steamcommunity.com/id/Some_Handle-2?l=en"),
            "https://steamcommunity.com/profiles/<id>/ and steamcommunity.com/id/<id>?l=en"
        );
        // An account named like the user: `<id>`, in any case.
        assert_eq!(
            s.text("HTTPS://SteamCommunity.com/ID/jtest."),
            "HTTPS://SteamCommunity.com/ID/<id>."
        );
        // Not an account: shorter or longer numbers, one joined to a letter, an address with no
        // account part or another page.
        let kept = "frame 76561198 at 1765611980123456789, x76561198012345678, steamcommunity.com/id/ and https://steamcommunity.com/market";
        assert_eq!(s.text(kept), kept);
    }

    #[test]
    fn user_and_computer_names_go_in_any_case_also_joined() {
        let s = player();
        assert_eq!(
            s.text("hello JTEST, jo tester on JT-DESK"),
            "hello <user>, <user> on <computer>"
        );
        // Joined to digits or '_' anywhere; joined to letters only in a path or a mod's name.
        assert_eq!(
            s.text("jtester and xjtest, jtest2 and jtest_tweaks.esp"),
            "jtester and xjtest, <user>2 and <user>_tweaks.esp"
        );
        assert_eq!(s.text("G:\\x\\xjtestPatch.esp"), "<path>\\x<user>Patch.esp");
        assert_eq!(s.file_name("jtesterPatch.esp"), "<user>erPatch.esp");
        assert_eq!(s.text("mail jtest@example"), "mail <user>@example");
        // In a path's last part, a two-word name too.
        assert_eq!(s.text("G:\\x\\jtest.txt"), "<path>\\<user>.txt");
        assert_eq!(s.text("G:\\Backups\\Jo Tester.txt"), "<path>\\<user>.txt");
    }

    /// Short names: three characters as a word or joined to digits and '_', never to letters;
    /// one or two characters only inside paths. A name never goes inside an exe address.
    #[test]
    fn short_names_and_exe_addresses() {
        let bob = Scrubber::new(&Known {
            roots: Vec::new(),
            words: vec![("bob".into(), "<user>")],
        });
        assert_eq!(
            bob.text("bob2.esp bob_tweaks BobsPatch xXbobXx, bob."),
            "<user>2.esp <user>_tweaks BobsPatch xXbobXx, <user>."
        );
        assert_eq!(bob.text("in G:\\x\\bob2"), "in <path>\\<user>2");
        let jo = Scrubber::new(&Known {
            roots: Vec::new(),
            words: vec![("jo".into(), "<user>"), ("a".into(), "<user>")],
        });
        let stop = "jolly jo: a call the platform does not provide";
        assert_eq!(jo.text(stop), stop);
        assert_eq!(
            jo.text("G:\\x\\jo2.esp and G:\\y\\a"),
            "<path>\\<user>2.esp and <path>\\<user>"
        );
        let hex = Scrubber::new(&Known {
            roots: Vec::new(),
            words: vec![("12345".into(), "<user>"), ("cafe".into(), "<user>")],
        });
        let stop = "Game's main stopped in frame 3: x (in 00412345 < 0040cafe)";
        assert_eq!(hex.text(stop), stop);
        assert_eq!(hex.text("a cafe, 12345."), "a <user>, <user>.");
    }

    /// Names that are ordinary words (Fedora's host name, Windows' "User", the Deck's "deck") go
    /// only inside paths; the system's own words stay.
    #[test]
    fn ordinary_word_names_go_only_inside_paths() {
        let s = Scrubber::new(&Known {
            roots: vec![("F:\\Games\\nv-rs".into(), "<nv-rs>")],
            words: vec![
                ("deck".into(), "<user>"),
                ("fedora".into(), "<computer>"),
                ("User".into(), "<user>"),
            ],
        });
        assert_eq!(
            s.text("Fedora Linux 40 on a Steam Deck, user settings"),
            "Fedora Linux 40 on a Steam Deck, user settings"
        );
        assert_eq!(
            s.text("F:\\Games\\nv-rs\\mods\\deck\\x.esp and G:\\x\\user"),
            "<nv-rs>\\mods\\<user>\\x.esp and <path>\\<user>"
        );
    }

    #[test]
    fn a_name_inside_a_placeholder_is_left_alone() {
        let s = Scrubber::new(&Known {
            roots: vec![("/srv/home".into(), "<home>")],
            words: vec![("srvx".into(), "<user>"), ("path".into(), "<user>")],
        });
        assert_eq!(s.text("/srv/home/x and path"), "<home>/... and <user>");
        assert_eq!(s.text("/opt/x/y"), "<path>/y");
    }

    #[test]
    fn nothing_known_still_takes_out_absolute_paths() {
        let s = Scrubber::new(&Known::default());
        assert_eq!(s.text("plain text"), "plain text");
        assert_eq!(s.text("at /srv/a/b.log now"), "at <path>/b.log now");
    }

    #[test]
    fn a_drive_or_the_root_alone_is_no_known_folder() {
        assert!(Root::of("C:\\", "<home>").is_none());
        assert!(Root::of("/", "<home>").is_none());
        assert!(Root::of("", "<home>").is_none());
        assert!(Root::of("/home", "<home>").is_some());
        assert!(Root::of("C:\\x", "<home>").is_some());
    }

    #[test]
    fn this_systems_names_are_found() {
        let k = Known::from_system(
            &[],
            Path::new("/no/such/program"),
            Path::new("/no/such/own"),
        );
        // The test's own process has a home folder on every system the checks run on.
        assert!(k.roots.iter().any(|(_, m)| *m == "<home>"), "{:?}", k.roots);
        assert!(k.words.iter().any(|(_, m)| *m == "<user>"));
        let s = Scrubber::new(&k);
        for (home, mark) in &k.roots {
            if *mark == "<home>" && Root::of(home, mark).is_some() {
                let scrubbed = s.text(&format!("at {home}/x"));
                assert_eq!(scrubbed, "at <home>/...");
            }
        }
    }

    /// REVIEW-1042's examples: lists of paths, a folder with a comma, eight-hex user names, the
    /// version word only as the whole word before a number, link-local addresses, a character's
    /// save folder below home and with doubled separators, quoted names with an apostrophe,
    /// host-only shares and addresses, a share after ':', OneDrive's company folders, Steam ids
    /// anywhere, and ordinary-word names in mods' file names.
    #[test]
    fn the_review_of_1042s_examples() {
        let s = player();
        // Lists: ';' opens the next path, ':' too with one '/' or a backslash share.
        assert_eq!(
            s.text("C:\\a\\b;\\\\JT-NAS\\Private\\y.pdf"),
            "<path>\\b;<path>\\y.pdf"
        );
        assert_eq!(
            s.text("/srv/a:/srv/private/tax 2026/y.pdf"),
            "<path>/a:<path>/y.pdf"
        );
        assert_eq!(
            s.text("LD_LIBRARY_PATH=/opt/a/lib:/mnt/Private Stuff/lib"),
            "LD_LIBRARY_PATH=<path>/lib:<path>/lib"
        );
        assert_eq!(
            s.text("/home/jtest/x:/media/jtest-backup/Private/tax.pdf"),
            "<home>/...:<path>/tax.pdf"
        );
        assert_eq!(
            s.text("F:\\Games\\nv-rs\\userdata\\x;\\\\JT-NAS\\Private\\y.pdf"),
            "<nv-rs>\\userdata\\x;<path>\\y.pdf"
        );
        assert_eq!(
            s.text("/home/jtest/.local/share/nv-rs/x;/srv/private/y.pdf"),
            "<nv-rs-data>/x;<path>/y.pdf"
        );
        // A folder with a comma.
        assert_eq!(s.text("G:\\Backups\\Tester, Jo\\x.txt"), "<path>\\x.txt");
        // An eight-hex-digit user name goes; exe addresses stay.
        let num = Scrubber::new(&Known {
            roots: Vec::new(),
            words: vec![("20231234".into(), "<user>")],
        });
        assert_eq!(
            num.text("hello 20231234, 20231234_patch.esp, 20231234-pc"),
            "hello <user>, <user>_patch.esp, <user>-pc"
        );
        assert_eq!(
            num.text("(in 00a1b2c3 < 00400000)"),
            "(in 00a1b2c3 < 00400000)"
        );
        // The version word only as the whole word right before the number.
        assert_eq!(
            s.text("connected to server 192.168.1.20"),
            "connected to server <ip>"
        );
        assert_eq!(
            s.text("observer 10.0.0.5, build server at 10.1.2.3"),
            "observer <ip>, build server at <ip>"
        );
        let kept = "version 1.3.250.0, driver: 31.0.101.5";
        assert_eq!(s.text(kept), kept);
        // Link-local and unique-local addresses with a short end.
        assert_eq!(s.text("at fe80::beef and fd00::cafe"), "at <ip> and <ip>");
        // A character's own save folder below home, and with doubled separators.
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Alexxander\\quicksave.fos"),
            "<home>\\...\\<save>\\<save>.fos"
        );
        assert_eq!(
            s.text("\"F:\\\\Games\\\\nv-rs\\\\userdata\\\\game\\\\Documents\\\\My Games\\\\FalloutNV\\\\Saves\\\\Jo Hero\\\\quicksave.fos\""),
            "\"<nv-rs>\\\\userdata\\\\game\\\\Documents\\\\My Games\\\\FalloutNV\\\\Saves\\\\<save>\\\\<save>.fos\""
        );
        // Quoted names with an apostrophe (GNU tools quote names that way).
        assert_eq!(s.text("removed 'Jo's run.fos'"), "removed '<save>.fos'");
        assert_eq!(
            s.text("removed 'D'Arcy Tester.fos'"),
            "removed '<save>.fos'"
        );
        // A share or an address that is only a host keeps nothing; a share after ':' is found.
        assert_eq!(s.text("on \\\\JT-NAS now"), "on <path> now");
        assert_eq!(s.text("on \"\\\\\\\\JT-NAS\""), "on \"<path>\"");
        assert_eq!(
            s.text("at smb://jt-nas and file://JT-NAS"),
            "at <path> and <path>"
        );
        assert_eq!(s.text("path:\\\\JT-NAS\\Private\\x"), "path:<path>\\x");
        // OneDrive's company folder without OneDrive's variables, with many words or a comma.
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd"),
            "<home>\\..."
        );
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Royal Melbourne Institute of Technology\\Documents\\My Games\\FalloutNV\\x.ini"),
            "<home>\\...\\FalloutNV\\x.ini"
        );
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Smith, Jones & Co\\Desktop\\x.txt"),
            "<home>\\...\\Desktop\\x.txt"
        );
        // The personal OneDrive is not the start of a work one.
        let od = Scrubber::new(&Known {
            roots: vec![
                ("D:\\Profiles\\Jo Tester\\OneDrive".into(), "<onedrive>"),
                ("D:\\Profiles\\Jo Tester".into(), "<home>"),
            ],
            words: Vec::new(),
        });
        assert_eq!(
            od.text("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd2\\x"),
            "<home>\\...\\x"
        );
        assert_eq!(
            od.text("D:\\Profiles\\Jo Tester\\OneDrive\\Documents\\x"),
            "<onedrive>\\...\\x"
        );
        // A Steam account's number outside home too.
        assert_eq!(
            s.text("C:\\Program Files (x86)\\Steam\\userdata\\123456789"),
            "<path>\\<id>"
        );
        // Ordinary-word names stay in mods' file names.
        let deck = Scrubber::new(&Known {
            roots: Vec::new(),
            words: vec![("deck".into(), "<user>"), ("User".into(), "<user>")],
        });
        assert_eq!(
            deck.file_name("Steam Deck Controls.esp"),
            "Steam Deck Controls.esp"
        );
        assert_eq!(
            deck.file_name("User Interface Tweaks.esp"),
            "User Interface Tweaks.esp"
        );
    }

    /// REVIEW-1042-delta's examples: a work OneDrive's name ends before what the line says
    /// next, OneDrive's own folders are found with words after them, and a character's save
    /// folder goes where the host's `files:` lines print it alone.
    #[test]
    fn the_second_review_of_1042s_examples() {
        let s = player();
        // Without OneDrive's variables: the company's part ends at `: `, before another path, an
        // address or a call path; what follows stays.
        assert_eq!(
            s.text("could not write D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd: access denied (in 00a1b2c3 < 00400000)"),
            "could not write <home>\\...: access denied (in 00a1b2c3 < 00400000)"
        );
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd and G:\\Private\\Tax 2026\\Smith family\\return.pdf"),
            "<home>\\... <path>\\return.pdf"
        );
        assert_eq!(
            s.text("copied D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd to D:\\Backup\\Jo Tester\\x.zip"),
            "copied <home>\\... <path>\\x.zip"
        );
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd and /srv/private/smith/y.pdf"),
            "<home>\\... <path>/y.pdf"
        );
        assert_eq!(
            s.text("mail jo.tester@example.com about D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd, ip 192.168.1.20"),
            "mail <e-mail> about <home>\\... <ip>"
        );
        assert_eq!(
            s.text("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd see https://example.com/x"),
            "<home>\\... https://example.com/x"
        );
        for (text, want) in [
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd mailed jo@example.com",
                "<home>\\... <e-mail>",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd to \\\\JT-NAS\\Private\\x.zip",
                "<home>\\... <path>\\x.zip",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd to smb://jt-nas/private/x.zip",
                "<home>\\... <path>/x.zip",
            ),
            (
                "could not read D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd (in 00a1b2c3 < 00400000)",
                "could not read <home>\\... (in 00a1b2c3 < 00400000)",
            ),
        ] {
            assert_eq!(s.text(text), want, "{text}");
        }
        assert_eq!(
            s.text(
                "D:\\Profiles\\Jo Tester\\OneDrive is full; see G:\\Private\\Smith\\notes\\x.txt"
            ),
            "<home>\\... is full; see <path>\\x.txt"
        );
        // With OneDrive's variables: its folders are whole names with words after them.
        let od = Scrubber::new(&Known {
            roots: vec![
                (
                    "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd".into(),
                    "<onedrive>",
                ),
                ("D:\\Profiles\\Jo Tester\\OneDrive".into(), "<onedrive>"),
                ("D:\\Profiles\\Jo Tester".into(), "<home>"),
            ],
            words: Vec::new(),
        });
        for (text, want) in [
            (
                "could not read D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd (in 00a1b2c3 < 00400000)",
                "could not read <onedrive> (in 00a1b2c3 < 00400000)",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd and G:\\Private\\Tax 2026\\Smith family\\return.pdf",
                "<onedrive> and <path>\\return.pdf",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd: access denied",
                "<onedrive>: access denied",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive is full; see G:\\Private\\Smith\\notes\\x.txt",
                "<onedrive> is full; see <path>\\x.txt",
            ),
            (
                "Documents are in D:\\Profiles\\Jo Tester\\OneDrive (synced), 3 GB free; see G:\\Private\\Smith\\notes\\x.txt",
                "Documents are in <onedrive> (synced), 3 GB free; see <path>\\x.txt",
            ),
            (
                "could not read D:\\Profiles\\Jo Tester\\OneDrive (in 00a1b2c3 < 00400000)",
                "could not read <onedrive> (in 00a1b2c3 < 00400000)",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd2\\x",
                "<home>\\...\\x",
            ),
        ] {
            assert_eq!(od.text(text), want, "{text}");
        }
        // A character's save folder at a path's end: the host's `files:` lines (Linux, macOS),
        // below home, in nv-rs's folder and in other paths; a save right after `Saves` stays one.
        assert_eq!(
            s.text("files: /home/jtest/.local/share/nv-rs/game/Documents/My Games/FalloutNV/Saves/Alexxander holds 2 names for the game's \"save 3   jo hero.fos\" (Save 3   Jo Hero.fos, SAVE 3   JO HERO.fos); \"Save 3   Jo Hero.fos\" is taken"),
            "files: <nv-rs-data>/game/Documents/My Games/FalloutNV/Saves/<save> holds 2 names for the game's \"<save>.fos\" (<save>.fos, <save>.fos); \"<save>.fos\" is taken"
        );
        assert_eq!(
            s.text("files: /home/jtest/.local/share/nv-rs/game/Documents/My Games/FalloutNV/Saves/Alexxander holds \"Jo Hero: run.fos\", a name Windows cannot hold in a folder"),
            "files: <nv-rs-data>/game/Documents/My Games/FalloutNV/Saves/<save> holds \"<save>.fos\", a name Windows cannot hold in a folder"
        );
        // `{:?}` escapes a name's own quotes.
        assert_eq!(
            s.text("files: /home/jtest/.local/share/nv-rs/game/Documents/My Games/FalloutNV/Saves holds \"Jo \\\"Ace\\\" run.fos\", a name Windows cannot hold in a folder"),
            "files: <nv-rs-data>/game/Documents/My Games/FalloutNV/Saves holds \"<save>.fos\", a name Windows cannot hold in a folder"
        );
        for (text, want) in [
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Alexxander",
                "<home>\\...\\Saves\\<save>",
            ),
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Alexxander\\",
                "<home>\\...\\Saves\\<save>\\",
            ),
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Save 1.fos",
                "<home>\\...\\Saves\\<save>.fos",
            ),
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Save 1.fos -> backup\\x",
                "<home>\\...\\Saves\\<save>.fos -> backup\\x",
            ),
            (
                "F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Alexxander holds 2 names",
                "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save> holds 2 names",
            ),
            ("G:\\x\\Saves\\Alexxander", "<path>\\Saves\\<save>"),
            (
                "G:\\x\\Saves\\Jo Hero holds 2 names",
                "<path>\\Saves\\<save> holds 2 names",
            ),
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Jo Hero\\x.txt",
                "<home>\\...\\<save>\\x.txt",
            ),
            (
                "/srv/nv/Saves/Alexxander holds 2 names",
                "<path>/Saves/<save> holds 2 names",
            ),
        ] {
            assert_eq!(s.text(text), want, "{text}");
        }
    }

    /// Every review example for the made-up player (REVIEW-1035-delta to REVIEW-1042-delta2),
    /// each alone and all in one long mixed line, so a later fix cannot reopen an old leak or let
    /// one case's rule reach into the next.
    #[test]
    fn every_review_example_holds_alone_and_in_one_long_line() {
        let s = player();
        let cases: &[(&str, &str)] = &[
            // Lists of paths, a comma in a folder.
            ("C:\\a\\b;\\\\JT-NAS\\Private\\y.pdf", "<path>\\b;<path>\\y.pdf"),
            ("/srv/a:/srv/private/tax 2026/y.pdf", "<path>/a:<path>/y.pdf"),
            (
                "LD_LIBRARY_PATH=/opt/a/lib:/mnt/Private Stuff/lib",
                "LD_LIBRARY_PATH=<path>/lib:<path>/lib",
            ),
            (
                "/home/jtest/x:/media/jtest-backup/Private/tax.pdf",
                "<home>/...:<path>/tax.pdf",
            ),
            (
                "F:\\Games\\nv-rs\\userdata\\x;\\\\JT-NAS\\Private\\y.pdf",
                "<nv-rs>\\userdata\\x;<path>\\y.pdf",
            ),
            (
                "/home/jtest/.local/share/nv-rs/x;/srv/private/y.pdf",
                "<nv-rs-data>/x;<path>/y.pdf",
            ),
            ("G:\\Backups\\Tester, Jo\\x.txt", "<path>\\x.txt"),
            // Addresses and versions.
            ("connected to server 192.168.1.20", "connected to server <ip>"),
            (
                "observer 10.0.0.5, build server at 10.1.2.3",
                "observer <ip>, build server at <ip>",
            ),
            (
                "version 1.3.250.0, driver: 31.0.101.5",
                "version 1.3.250.0, driver: 31.0.101.5",
            ),
            ("at fe80::beef and fd00::cafe", "at <ip> and <ip>"),
            // Saves and characters' folders.
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Alexxander\\quicksave.fos",
                "<home>\\...\\<save>\\<save>.fos",
            ),
            (
                "\"F:\\\\Games\\\\nv-rs\\\\userdata\\\\game\\\\Documents\\\\My Games\\\\FalloutNV\\\\Saves\\\\Jo Hero\\\\quicksave.fos\"",
                "\"<nv-rs>\\\\userdata\\\\game\\\\Documents\\\\My Games\\\\FalloutNV\\\\Saves\\\\<save>\\\\<save>.fos\"",
            ),
            ("removed 'Jo's run.fos'", "removed '<save>.fos'"),
            ("removed 'D'Arcy Tester.fos'", "removed '<save>.fos'"),
            (
                "NV_SAVE_OUT: copied Save 3   Jo Hero.fos, Save 4   Jo Hero.fos; Save 5   Jo Hero.fos",
                "NV_SAVE_OUT: copied <save>.fos, <save>.fos; <save>.fos",
            ),
            (
                "NV_SAVE_OUT: copied Jo's (best) run.fos",
                "NV_SAVE_OUT: copied <save>.fos",
            ),
            (
                "files: /home/jtest/.local/share/nv-rs/game/Documents/My Games/FalloutNV/Saves/Alexxander holds \"Jo Hero: run.fos\", a name Windows cannot hold in a folder",
                "files: <nv-rs-data>/game/Documents/My Games/FalloutNV/Saves/<save> holds \"<save>.fos\", a name Windows cannot hold in a folder",
            ),
            (
                "files: /home/jtest/.local/share/nv-rs/game/Documents/My Games/FalloutNV/Saves holds \"Jo \\\"Ace\\\" run.fos\", a name Windows cannot hold in a folder",
                "files: <nv-rs-data>/game/Documents/My Games/FalloutNV/Saves holds \"<save>.fos\", a name Windows cannot hold in a folder",
            ),
            (
                "files: /srv/nv/Saves/Alexxander, Saves/Benny and Saves/Jo Hero holds 2 names",
                "files: <path>/Saves/<save>, Saves/<save> Saves/<save> holds 2 names",
            ),
            (
                "F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Jo Hero: access denied",
                "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save>: access denied",
            ),
            (
                "F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Jo Hero and more",
                "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save>",
            ),
            (
                "F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Save 3   Jo Hero  Primm.bak copied",
                "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save>",
            ),
            (
                "F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\*.fos and Saves\\* and Saves\\.",
                "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save>.fos and Saves\\* and Saves\\.",
            ),
            (
                "/home/jtest/.local/share/nv-rs/game/Documents/My Games/FalloutNV/Saves/Jo Hero",
                "<nv-rs-data>/game/Documents/My Games/FalloutNV/Saves/<save>",
            ),
            (
                "could not read F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Jo Hero (in 00a1b2c3 < 00400000)",
                "could not read <nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save> (in 00a1b2c3 < 00400000)",
            ),
            (
                "G:\\x\\Saves\\Jo Hero mailed jo@example.com",
                "<path>\\Saves\\<save> <e-mail>",
            ),
            (
                "NV_SAVE_OUT=D:\\Backup\\Saves\\Jo Hero: refused (a link); saves are not copied.",
                "NV_SAVE_OUT=<path>\\Saves\\<save>: refused (a link); saves are not copied.",
            ),
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Alexxander\\",
                "<home>\\...\\Saves\\<save>\\",
            ),
            // A character's folder in the middle of a path goes with the path's cut.
            (
                "E:\\Backup\\FalloutNV\\Saves\\Alexxander\\Private Stuff\\Tax 2026\\x.txt",
                "<path>\\x.txt",
            ),
            (
                "E:\\Backup\\FalloutNV\\Saves\\Alexxander\\old\\Smith family\\Save 1.fos",
                "<path>\\Smith family\\<save>.fos",
            ),
            ("/srv/backup/Saves/Alexxander/private/smith/y.txt", "<path>/y.txt"),
            (
                "\\\\JT-NAS\\Games\\Saves\\Alexxander\\Private\\x.txt",
                "<path>\\x.txt",
            ),
            (
                "\"E:\\\\Backup\\\\Saves\\\\Alexxander\\\\Private\\\\x.txt\"",
                "\"<path>\\x.txt\"",
            ),
            ("E:\\Backup\\Saves\\Alexxander\\x.txt", "<path>\\x.txt"),
            // ... and with a space in its name (a character's name), in every form.
            ("E:\\Backup\\Saves\\Jo Hero\\Private\\x.txt", "<path>\\x.txt"),
            (
                "E:\\Backup\\Saves\\Jo Hero Old\\Private Stuff\\Tax 2026\\x.txt",
                "<path>\\x.txt",
            ),
            ("/srv/backup/Saves/Jo Hero/private/smith/y.txt", "<path>/y.txt"),
            (
                "\\\\JT-NAS\\Games\\Saves\\Jo Hero\\Private\\x.txt",
                "<path>\\x.txt",
            ),
            (
                "\"E:\\\\Backup\\\\Saves\\\\Jo Hero\\\\Private\\\\x.txt\"",
                "\"<path>\\x.txt\"",
            ),
            (
                "/mnt/games/SteamLibrary/steamapps/compatdata/22380/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV/Saves/Jo Hero/Backup/old/Save 1.fos",
                "<path>/old/<save>.fos",
            ),
            (
                "files: /mnt/games/x/Saves/Jo Hero holds 2 names for the game's \"save 3   jo hero.fos\" (Save 3   Jo Hero.fos, SAVE 3   JO HERO.fos); \"Save 3   Jo Hero.fos\" is taken",
                "files: <path>/Saves/<save> holds 2 names for the game's \"<save>.fos\" (<save>.fos, <save>.fos); \"<save>.fos\" is taken",
            ),
            // A folder named like `Saves` under Saves, other separators, spaces in a run.
            ("E:\\Backup\\Saves\\Jo Saves\\Private\\x.txt", "<path>\\x.txt"),
            (
                "E:\\Backup\\Saves\\Old Saves\\Private\\Tax 2026\\x.txt",
                "<path>\\x.txt",
            ),
            (
                "E:\\Backup\\Saves\\Old Saves\\Jo Hero\\Backup\\Save 1.fos",
                "<path>\\Backup\\<save>.fos",
            ),
            (
                "E:\\Backup\\Saves\\Old Saves\\Jo Hero\\Save 3   Jo Hero.fos",
                "<path>\\Saves\\<save> Saves\\<save>\\<save>.fos",
            ),
            (
                "E:\\Backup\\Saves\\Jo Hero Old Saves Copy\\Private\\x.txt",
                "<path>\\x.txt",
            ),
            (
                "E:\\Backup\\Saves\\Jo Hero Saves\\x and E:\\Backup\\Saves\\Jo Saves",
                "<path>\\Saves\\<save> Saves\\<save> <path>\\Saves\\<save>",
            ),
            (
                "E:\\Backup\\Saves\\Jo Hero saves/x.txt",
                "<path>\\Saves\\<save> saves/<save>",
            ),
            ("E:\\Backup\\Saves\\Jo  Hero\\Private\\x.txt", "<path>\\x.txt"),
            ("E:/Backup/Saves/Jo Hero/Private/x.txt", "<path>/x.txt"),
            ("E:\\Backup/Saves/Jo Hero\\Private\\x.txt", "<path>\\x.txt"),
            ("smb://jt-nas/games/Saves/Jo Hero/private/x.txt", "<path>/x.txt"),
            (
                "file:///E:/Backup/Saves/Jo Hero/Private/x.txt",
                "file:///<path>/x.txt",
            ),
            ("E:\\Backup\\Saves\\Jo Hero copied to Backups\\x", "<path>\\x"),
            // A later word with only another kind of separator is the line's.
            (
                "E:\\Backup\\Saves\\Jo Hero 3/4 done",
                "<path>\\Saves\\<save> 3/4 done",
            ),
            ("E:\\Backup\\Saves\\Jo Hero (1/2)", "<path>\\Saves\\<save> (1/2)"),
            (
                "E:\\Backup\\Saves\\Jo Hero and/or x",
                "<path>\\Saves\\<save> and/or x",
            ),
            ("/srv/nv/Saves/Jo Hero 3/4 done", "<path>/4 done"),
            (
                "F:\\Games\\nv-rs\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\Jo Hero 3/4 done",
                "<nv-rs>\\userdata\\game\\Documents\\My Games\\FalloutNV\\Saves\\<save>/4 done",
            ),
            (
                "D:\\Profiles\\Jo Tester\\Documents\\My Games\\FalloutNV\\Saves\\Jo Hero 3/4 done",
                "<home>\\...\\<save>\\4 done",
            ),
            // A character's folder at a path's end: where its name ends.
            ("E:\\Backup\\Saves\\Hero, Jo", "<path>\\Saves\\<save>, Jo"),
            ("E:\\Backup\\Saves\\Jo Hero; then x", "<path>\\Saves\\<save>; then x"),
            (
                "E:\\Backup\\Saves\\Jo Hero\\ and more",
                "<path>\\Saves\\<save>\\ and more",
            ),
            (
                "E:\\Backup\\Saves\\Jo Hero 192.168.1.20",
                "<path>\\Saves\\<save> <ip>",
            ),
            (
                "E:\\Backup\\Saves\\Jo Hero\tnext field",
                "<path>\\Saves\\<save>\tnext field",
            ),
            ("E:\\Backup\\Saves\\O'Brien Hero, ok", "<path>\\Saves\\<save>, ok"),
            (
                "Game's main stopped in frame 12: could not open E:\\Backup\\Saves\\Jo Hero (in 00a1b2c3 < 00400000)",
                "Game's main stopped in frame 12: could not open <path>\\Saves\\<save> (in 00a1b2c3 < 00400000)",
            ),
            // Relative forms stay as main has them; `Saves\` before a space names nothing.
            (
                "Saves\\Jo Hero\\Private\\x.txt and saves/Jo Hero/notes/y.txt",
                "Saves\\Jo Hero\\Private\\x.txt and saves/Jo Hero/notes/y.txt",
            ),
            (
                "the Saves\\ folder, Saves\\Jo Hero",
                "the Saves\\ folder, Saves\\<save>",
            ),
            // Hosts and shares.
            ("on \\\\JT-NAS now", "on <path> now"),
            ("on \"\\\\\\\\JT-NAS\"", "on \"<path>\""),
            ("at smb://jt-nas and file://JT-NAS", "at <path> and <path>"),
            ("path:\\\\JT-NAS\\Private\\x", "path:<path>\\x"),
            // OneDrive without its variables.
            ("D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd", "<home>\\..."),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd is full",
                "<home>\\...",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Royal Melbourne Institute of Technology\\Documents\\My Games\\FalloutNV\\x.ini",
                "<home>\\...\\FalloutNV\\x.ini",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Smith, Jones & Co\\Desktop\\x.txt",
                "<home>\\...\\Desktop\\x.txt",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - AT@T Services\\Documents\\x.ini",
                "<home>\\...\\Documents\\x.ini",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso (in liquidation) Ltd\\Documents\\x.ini",
                "<home>\\...\\Documents\\x.ini",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd - Archive\\x.ini",
                "<home>\\...\\x.ini",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - 10.0.0.5 Holdings\\x.ini",
                "<home>\\...\\x.ini",
            ),
            (
                "\"D:\\\\Profiles\\\\Jo Tester\\\\OneDrive - Contoso Ltd\": access denied",
                "\"<home>\\\\...\": access denied",
            ),
            (
                "could not write D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd: access denied (in 00a1b2c3 < 00400000)",
                "could not write <home>\\...: access denied (in 00a1b2c3 < 00400000)",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd and G:\\Private\\Tax 2026\\Smith family\\return.pdf",
                "<home>\\... <path>\\return.pdf",
            ),
            (
                "mail jo.tester@example.com about D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd, ip 192.168.1.20",
                "mail <e-mail> about <home>\\... <ip>",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive is full; see G:\\Private\\Smith\\notes\\x.txt",
                "<home>\\... is full; see <path>\\x.txt",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd   | next",
                "<home>\\...   | next",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd> x",
                "<home>\\...> x",
            ),
            (
                "D:\\Profiles\\Jo Tester\\OneDrive - Contoso Ltd\tfield",
                "<home>\\...\tfield",
            ),
            // Account numbers; what stays.
            (
                "C:\\Program Files (x86)\\Steam\\userdata\\123456789",
                "<path>\\<id>",
            ),
            (
                "my steam is steamcommunity.com/profiles/76561198012345678, add me",
                "my steam is steamcommunity.com/profiles/<id>, add me",
            ),
            (
                "E:\\Steam\\steamapps\\common\\Fallout New Vegas\\Data\\Saves\\x.esp",
                "<game>\\Data\\Saves\\<save>",
            ),
            (
                "read of 1a2b3c00 not mapped (in 00412345 < 0040cafe)",
                "read of 1a2b3c00 not mapped (in 00412345 < 0040cafe)",
            ),
            (
                "see https://github.com/slaterain/nv-rs/issues at 12:34:56",
                "see https://github.com/slaterain/nv-rs/issues at 12:34:56",
            ),
        ];
        for (text, want) in cases {
            assert_eq!(s.text(text), *want, "{text}");
        }
        let line: Vec<&str> = cases.iter().map(|(t, _)| *t).collect();
        let want: Vec<&str> = cases.iter().map(|(_, w)| *w).collect();
        assert_eq!(s.text(&line.join(" | ")), want.join(" | "));
    }
}
