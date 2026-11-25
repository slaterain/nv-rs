//! What is taken out of a crash report before it is kept or sent: the player's home folder, user
//! name and computer name, and every absolute path but its part below the game's folder or
//! nv-rs's own. The same on every system: a Windows report read on Linux (or a path written with
//! `/` on Windows) is scrubbed alike.
//!
//! 1. Known folders (the game's, nv-rs's own, the package's, the home folder), longest first,
//!    matched literally in any case with any run of `\` or `/` between their parts (so the doubled
//!    backslashes of JSON and of Rust's `{:?}` match too): replaced by their placeholder, the part
//!    below kept (`<home>\AppData\Local\Temp\x.tmp`).
//! 2. Any other absolute path (`X:\...`, `\\host\share\...`, or `/...` at the start of a word):
//!    `<path>` and its last part only (`<path>\d3d9.dll`). A path goes on over a space when one of
//!    the next three words holds its separator (`C:\Program Files (x86)\x\y.dll`).
//! 3. Then, outside the placeholders: the user names and the computer name, as whole words in any
//!    case, become `<user>` and `<computer>`.

use std::path::Path;

/// One piece of scrubbed text: plain text, or a placeholder put in its place.
#[derive(Clone, Debug, PartialEq)]
enum Piece {
    Text(Vec<char>),
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

/// The scrubber of one report: the folders and names it takes out.
pub struct Scrubber {
    roots: Vec<Root>,
    words: Vec<(Vec<char>, &'static str)>,
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
        let mut words: Vec<(Vec<char>, &'static str)> = known
            .words
            .iter()
            .map(|(w, mark)| (w.trim().chars().collect::<Vec<char>>(), *mark))
            .filter(|(w, _)| !w.is_empty())
            .collect();
        words.sort_by_key(|(w, _)| std::cmp::Reverse(w.len()));
        words.dedup_by(|a, b| {
            a.0.len() == b.0.len() && a.0.iter().zip(&b.0).all(|(&x, &y)| same(x, y))
        });
        Scrubber { roots, words }
    }

    /// `text` with everything above taken out.
    pub fn text(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut pieces = self.known_folders(&chars);
        pieces = other_paths(pieces);
        pieces = self.names(pieces);
        let mut out = String::with_capacity(text.len());
        for p in pieces {
            match p {
                Piece::Text(t) => out.extend(t),
                Piece::Mark(m) => out.push_str(m),
            }
        }
        out
    }

    fn known_folders(&self, text: &[char]) -> Vec<Piece> {
        let mut pieces = Vec::new();
        let mut plain = Vec::new();
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
            match found {
                Some((end, mark)) => {
                    push_text(&mut pieces, &mut plain);
                    pieces.push(Piece::Mark(mark));
                    i = end;
                }
                None => {
                    plain.push(text[i]);
                    i += 1;
                }
            }
        }
        push_text(&mut pieces, &mut plain);
        pieces
    }

    fn names(&self, pieces: Vec<Piece>) -> Vec<Piece> {
        let mut out = Vec::new();
        for piece in pieces {
            let Piece::Text(text) = piece else {
                out.push(piece);
                continue;
            };
            let mut plain = Vec::new();
            let mut i = 0;
            while i < text.len() {
                let starts = i == 0 || !is_word_char(text[i - 1]);
                let found = starts
                    .then(|| {
                        self.words.iter().find(|(w, _)| {
                            text.len() - i >= w.len()
                                && w.iter().zip(&text[i..]).all(|(&a, &b)| same(a, b))
                                && !text.get(i + w.len()).is_some_and(|&c| is_word_char(c))
                        })
                    })
                    .flatten();
                match found {
                    Some((w, mark)) => {
                        push_text(&mut out, &mut plain);
                        out.push(Piece::Mark(mark));
                        i += w.len();
                    }
                    None => {
                        plain.push(text[i]);
                        i += 1;
                    }
                }
            }
            push_text(&mut out, &mut plain);
        }
        out
    }
}

fn push_text(pieces: &mut Vec<Piece>, plain: &mut Vec<char>) {
    if !plain.is_empty() {
        pieces.push(Piece::Text(std::mem::take(plain)));
    }
}

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

/// Characters a name is part of (a user name is a whole word only).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
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

/// Characters that end a path that is not a known folder.
fn ends_path(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | '|' | '*' | '?')
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
    let (sep, body) = if at(0).is_some_and(|c| c.is_ascii_alphabetic())
        && at(1) == Some(':')
        && at(2).is_some_and(is_separator)
        && before.is_none_or(|c| !c.is_alphanumeric())
    {
        (at(2)?, i + 3)
    } else if at(0) == Some('\\')
        && at(1) == Some('\\')
        && (at(2).is_some_and(char::is_alphanumeric)
            || text[i..].starts_with(&['\\', '\\', '?', '\\', 'U', 'N', 'C', '\\']))
        && opens_path(before)
    {
        ('\\', i + 2)
    } else if at(0) == Some('/')
        && at(1).is_some_and(|c| !is_separator(c) && !ends_path(c))
        && opens_path(before)
    {
        ('/', i + 1)
    } else {
        return None;
    };
    let word_end = |from: usize| from + text[from..].iter().take_while(|&&c| !ends_path(c)).count();
    let mut end = word_end(body);
    // Folders with spaces: on over up to three words while one of them holds the separator.
    loop {
        let mut k = end;
        let mut extended = None;
        for _ in 0..3 {
            if text.get(k) != Some(&' ') {
                break;
            }
            let start = k + 1;
            let stop = word_end(start);
            if stop == start {
                break;
            }
            if text[start..stop].contains(&sep) {
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
    // Punctuation after the path is the sentence's.
    while end > body {
        let c = text[end - 1];
        let open = text[i..end].iter().filter(|&&x| x == '(').count();
        let close = text[i..end].iter().filter(|&&x| x == ')').count();
        if matches!(c, '.' | ',' | ';' | ':') || (c == ')' && close > open) {
            end -= 1;
        } else {
            break;
        }
    }
    Some((sep, end))
}

/// Every absolute path in the plain pieces: `<path>` and its last part.
fn other_paths(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out = Vec::new();
    // The character before the next one: a placeholder's last (`>`), so `<home>/x` stays whole.
    let mut before: Option<char> = None;
    for piece in pieces {
        let Piece::Text(text) = piece else {
            out.push(piece);
            before = Some('>');
            continue;
        };
        let mut plain = Vec::new();
        let mut i = 0;
        while i < text.len() {
            let prev = if i == 0 { before } else { Some(text[i - 1]) };
            match path_at(&text, i, prev) {
                Some((sep, end)) => {
                    push_text(&mut out, &mut plain);
                    out.push(Piece::Mark("<path>"));
                    let last = text[i..end]
                        .split(|&c| is_separator(c))
                        .rfind(|p| !p.is_empty())
                        .filter(|p| !(p.len() == 2 && p[1] == ':'));
                    if let Some(last) = last {
                        let mut tail = vec![sep];
                        tail.extend_from_slice(last);
                        out.push(Piece::Text(tail));
                    }
                    i = end;
                }
                None => {
                    plain.push(text[i]);
                    i += 1;
                }
            }
        }
        before = text.last().copied();
        push_text(&mut out, &mut plain);
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
            "temp <home>\\AppData\\Local\\Temp\\x.tmp"
        );
        assert_eq!(s.text("d:/profiles/jo tester/Desktop"), "<home>/Desktop");
        // Doubled backslashes (JSON, Rust's {:?}) and a mix of both separators.
        assert_eq!(
            s.text("path: \"D:\\\\Profiles\\\\Jo Tester\\\\x.ini\""),
            "path: \"<home>\\\\x.ini\""
        );
        assert_eq!(s.text("D:\\Profiles/Jo Tester\\/y"), "<home>\\/y");
        // The verbatim form Windows answers for a real path.
        assert_eq!(
            s.text("\\\\?\\D:\\Profiles\\Jo Tester\\z"),
            "\\\\?\\<home>\\z"
        );
        assert_eq!(s.text("/home/jtest/.cache/x"), "<home>/.cache/x");
        // Another folder that only starts like it is not it (its name is still taken out).
        assert_eq!(s.text("/home/jtest2/x"), "<path>/x");
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
        assert_eq!(s.text("(C:\\x\\y.log)"), "(<path>\\y.log)");
        assert_eq!(s.text("C:\\"), "<path>");
        // Not paths: web addresses, relative source paths, fractions.
        let kept = "see https://github.com/slaterain/nv-rs/issues; panicked at nv-rs/crates/host/src/lib.rs:12:5; 1/2";
        assert_eq!(s.text(kept), kept);
    }

    #[test]
    fn user_and_computer_names_go_as_whole_words_in_any_case() {
        let s = player();
        assert_eq!(
            s.text("hello JTEST, jo tester on JT-DESK"),
            "hello <user>, <user> on <computer>"
        );
        assert_eq!(s.text("jtester and xjtest stay"), "jtester and xjtest stay");
        assert_eq!(s.text("mail jtest@example.org"), "mail <user>@example.org");
        // In a path that is no known folder: the path goes, so does the name in its last part.
        assert_eq!(s.text("G:\\x\\jtest.txt"), "<path>\\<user>.txt");
    }

    #[test]
    fn a_name_inside_a_placeholder_is_left_alone() {
        let s = Scrubber::new(&Known {
            roots: vec![("/home/home".into(), "<home>")],
            words: vec![("home".into(), "<user>"), ("path".into(), "<user>")],
        });
        assert_eq!(s.text("/home/home/x and home"), "<home>/x and <user>");
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
                assert_eq!(scrubbed, "at <home>/x");
            }
        }
    }
}
