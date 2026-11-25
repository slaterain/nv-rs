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
//! 4. E-mail addresses (`<e-mail>`) and IPv4 and IPv6 addresses (`<ip>`); versions, times and
//!    code paths stay.
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
    fn at(&self, text: &[char], in_path: &[bool], i: usize) -> bool {
        let n = self.chars.len();
        if text.len() - i < n || !self.chars.iter().zip(&text[i..]).all(|(&a, &b)| same(a, b)) {
            return false;
        }
        let path = in_path[i..i + n].iter().all(|&p| p);
        if (self.ordinary || n <= 2) && !path {
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
        if end - start == 8 && text[start..end].iter().all(char::is_ascii_hexdigit) {
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
        units = runs(units, other_paths);
        units = runs(units, addresses);
        units = runs(units, |text, in_path, _| self.names_in(text, in_path));
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
            let found = if starts {
                self.roots
                    .iter()
                    .find_map(|r| r.matches(text, i).map(|end| (end, r.mark)))
            } else {
                None
            };
            let Some((end, mark)) = found else {
                out.push(Unit::Char(text[i], false));
                i += 1;
                continue;
            };
            out.push(Unit::Mark(mark));
            // The part below, a path's rest: kept (below the home folder, its last two parts).
            let below = if text.get(end).is_some_and(|&c| is_separator(c)) {
                extent(text, end, end, None)
            } else {
                end
            };
            let part = &text[end..below];
            let part = if mark == "<home>" || mark == "<onedrive>" {
                last_two(part)
            } else {
                part.to_vec()
            };
            out.extend(part.into_iter().map(|c| Unit::Char(c, true)));
            i = below;
        }
        out
    }

    fn names_in(&self, text: &[char], in_path: &[bool]) -> Vec<Unit> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < text.len() {
            match self.names.iter().find(|n| n.at(text, in_path, i)) {
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
            let list = c == ';'
                || c == ','
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
            let from = k + 1;
            let stop = word_end(from);
            // Another path's start ends this one (`... and G:\x`, `... or /tmp/y`).
            let word = &text[from..stop];
            let url = word.windows(3).any(|w| w == [':', '/', '/']);
            if word.is_empty()
                || url
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
        if let Some(open) = text[from..ext].iter().rposition(|&c| c == q) {
            return from + open + 1;
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
    // Spaces before the name are the line's.
    while s < ext && text[s] == ' ' {
        s += 1;
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
    let end = start
        .checked_sub(1)
        .filter(|&e| e >= from && is_separator(text[e]))?;
    let begin = from + text[from..end].iter().rposition(|&c| is_separator(c))? + 1;
    let parent_end = begin.checked_sub(1)?;
    let parent_begin = from
        + text[from..parent_end]
            .iter()
            .rposition(|&c| is_separator(c))
            .map_or(0, |p| p + 1);
    let parent: String = text[parent_begin..parent_end].iter().collect();
    (parent.eq_ignore_ascii_case("saves") && begin < end).then_some((begin, end))
}

/// Whether a `/` (or a `\\` of a share) after `before` opens a path: at the text's start or after
/// a space, a quote, an opening bracket, `=` or `,` (not after `:`, so web addresses stay).
fn opens_path(before: Option<char>) -> bool {
    before.is_none_or(|c| {
        c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '(' | '[' | '{' | '=' | ',')
    })
}

/// The absolute path starting at `text[i]` (whose character before is `before`): its separator
/// and end, or none.
fn path_at(text: &[char], i: usize, before: Option<char>) -> Option<(char, usize)> {
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
    let (sep, body) = if at(0).is_some_and(|c| c.is_ascii_alphabetic())
        && at(1) == Some(':')
        && at(2).is_some_and(is_separator)
        && before.is_none_or(|c| !c.is_alphanumeric())
    {
        (at(2)?, i + 3)
    } else if let (Some(r), true) = (share(BACKSLASH), opens_path(before)) {
        (BACKSLASH, i + r)
    } else if let (Some(r), true) = (share('/'), opens_path(before)) {
        ('/', i + r)
    } else if let (Some(n), true) = (scheme, before.is_none_or(|c| !c.is_alphanumeric())) {
        ('/', i + n)
    } else if at(0) == Some('/')
        && at(1).is_some_and(|c| !is_separator(c) && !ends_path(c))
        && opens_path(before)
    {
        ('/', i + 1)
    } else {
        return None;
    };
    Some((sep, extent(text, i, body, Some(sep))))
}

/// Every absolute path in a run that is not the rest of a known folder: `<path>` and its last
/// part (and its last separator when it names a folder).
fn other_paths(text: &[char], in_path: &[bool], before: Option<char>) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let prev = if i == 0 { before } else { Some(text[i - 1]) };
        let found = (!in_path[i]).then(|| path_at(text, i, prev)).flatten();
        let Some((sep, end)) = found else {
            out.push(Unit::Char(text[i], in_path[i]));
            i += 1;
            continue;
        };
        out.push(Unit::Mark("<path>"));
        let path = &text[i..end];
        let last = path
            .split(|&c| is_separator(c))
            .rfind(|p| !p.is_empty())
            .filter(|p| !(p.len() == 2 && p[1] == ':'));
        if let Some(last) = last {
            out.push(Unit::Char(sep, true));
            out.extend(last.iter().map(|&c| Unit::Char(c, true)));
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
    // A version is no address: `version 1.3.250.0`, `driver 31.0.101.5`, `v1.0.0.1`.
    let before: String = text[i.saturating_sub(12)..i]
        .iter()
        .collect::<String>()
        .to_lowercase();
    if ["version", "driver", "build", "ver "]
        .iter()
        .any(|w| before.contains(w))
        || before.ends_with('v')
    {
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
    // Code, not an address: two names of hex letters (`face::add`, `d3d9::d3d9`).
    let named: Vec<&&str> = groups.iter().filter(|g| !g.is_empty()).collect();
    let code = named.len() == 2
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

/// E-mail and network addresses in a run.
fn addresses(text: &[char], in_path: &[bool], _: Option<char>) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if let Some(end) = email_at(text, i) {
            out.push(Unit::Mark("<e-mail>"));
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
}
