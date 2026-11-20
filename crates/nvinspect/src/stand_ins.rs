//! Test stand-ins and their word counts: a warning list, not a failure.
//!
//! A stand-in registered with `e.register(addr, f)` or `e.register_double(addr, f)` takes any
//! number of argument words, so a translation that hands that call a word too few or too many
//! still passes its test (two players' games stopped that way). Registered with
//! `e.register_words(addr, n, f)` / `e.register_double_words(addr, n, f)` it holds every call in
//! debug builds to `n` words (`this` included). This module lists the stand-ins in the crates'
//! test code that carry no count yet, with the count the real translation at that address takes
//! where one exists (its `entry!(addr, name(types))`): those can be counted by script; a
//! stand-in at an address no entry has (a made-up vtable slot, a function not translated yet)
//! needs its count from the exe (`vcall-words --words ADDR` for a function, or the table slot).
//!
//! What it does not do: it cannot tell a right count from a wrong one, and it does not see
//! stand-ins whose address is not a number or a constant written in their file (a variable, a
//! loop over addresses).

use std::collections::HashMap;
use std::path::Path;

use crate::vcall_words::{
    game_code, inside, mounted_paths, pieces, rust_files, test_file, test_spans,
};

/// One stand-in of the test code.
#[derive(Clone, Debug, PartialEq)]
pub struct StandIn {
    pub file: String,
    pub line: usize,
    /// The address, when it is a number or a constant of the file (or a `pub` constant of the
    /// tree with one value).
    pub addr: Option<u32>,
    /// Registered with a word count already.
    pub counted: bool,
    /// The words the translation at `addr` takes, from its `entry!`.
    pub entry_words: Option<u32>,
}

/// The words of an `entry!`'s parameter types, when every type is one the check reads.
fn type_words(types: &str) -> Option<u32> {
    pieces(types)
        .iter()
        .map(|ty| {
            let ty = ty.trim();
            let last = ty.rsplit("::").next().unwrap_or(ty);
            match last {
                "u32" | "i32" | "f32" | "bool" | "u8" | "i8" | "u16" | "i16" => Some(1),
                "f64" | "u64" | "i64" => Some(2),
                _ if last.starts_with("Ptr") => Some(1),
                _ => {
                    let array = ty.strip_prefix('[')?.strip_suffix(']')?;
                    let (element, count) = array.split_once(';')?;
                    (element.trim() == "u32").then(|| count.trim().parse().ok())?
                }
            }
        })
        .sum()
}

/// A number written in place: hex or decimal, with underscores.
fn literal(text: &str) -> Option<u32> {
    let text = text.trim().replace('_', "");
    match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

/// Every `entry!(ADDR, name(types) ..)` of `text`: address and words (`None` for a type the
/// check does not read).
pub fn entries(text: &str, into: &mut HashMap<u32, Option<u32>>) {
    for (at, _) in text.match_indices("entry!(") {
        let Some(args) = inside(text, at + "entry!(".len()) else {
            continue;
        };
        let parts = pieces(args);
        let (Some(addr), Some(signature)) = (parts.first().and_then(|a| literal(a)), parts.get(1))
        else {
            continue;
        };
        let words = signature
            .find('(')
            .and_then(|open| inside(signature, open + 1))
            .and_then(type_words);
        into.entry(addr)
            .and_modify(|w| {
                if *w != words {
                    *w = None;
                }
            })
            .or_insert(words);
    }
}

/// The `u32` constants of `text` written as a number (a name with two values left out).
fn constants(text: &str, public_only: bool, into: &mut HashMap<String, Option<u32>>) {
    for (at, _) in text.match_indices("const ") {
        let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        if public_only && !text[line_start..at].trim_start().starts_with("pub") {
            continue;
        }
        let rest = &text[at + "const ".len()..];
        let Some((name, rest)) = rest.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let Some(rest) = rest.trim_start().strip_prefix("u32") else {
            continue;
        };
        let Some(value) = rest
            .trim_start()
            .strip_prefix('=')
            .and_then(|v| v.split_once(';'))
            .and_then(|(v, _)| literal(v))
        else {
            continue;
        };
        into.entry(name.to_string())
            .and_modify(|v| {
                if *v != Some(value) {
                    *v = None;
                }
            })
            .or_insert(Some(value));
    }
}

/// The stand-ins in the test code of the file `rel` (its `text`; `mounted` when a
/// `#[cfg(test)] mod NAME;` mounts it), their addresses resolved with the file's constants and
/// then `public` (the tree's `pub` constants).
pub fn stand_ins(
    rel: &str,
    text: &str,
    mounted: bool,
    public: &HashMap<String, Option<u32>>,
    entry_words: &HashMap<u32, Option<u32>>,
) -> Vec<StandIn> {
    let (code, _) = game_code(rel, text, mounted);
    let mut own = HashMap::new();
    constants(text, false, &mut own);
    let mut out = Vec::new();
    for kind in [
        ".register(",
        ".register_double(",
        ".register_words(",
        ".register_double_words(",
    ] {
        for (at, _) in text.match_indices(kind) {
            // Test code only: blanked in the game code.
            if code.as_bytes().get(at) != Some(&b' ') {
                continue;
            }
            let Some(args) = inside(text, at + kind.len()) else {
                continue;
            };
            let Some(first) = pieces(args).into_iter().next() else {
                continue;
            };
            let name = first.rsplit("::").next().unwrap_or(&first).trim();
            let addr = literal(&first).or_else(|| {
                own.get(name)
                    .copied()
                    .flatten()
                    .or_else(|| public.get(name).copied().flatten())
            });
            out.push(StandIn {
                file: rel.to_string(),
                line: 1 + text[..at].matches('\n').count(),
                addr,
                counted: kind.ends_with("_words("),
                entry_words: addr.and_then(|a| entry_words.get(&a).copied().flatten()),
            });
        }
    }
    out.sort_by_key(|s| s.line);
    out
}

/// The stand-ins of every crate under `crates` (paths relative to it).
pub fn census(crates: &Path) -> Vec<StandIn> {
    let mut files = Vec::new();
    rust_files(crates, &mut files);
    let sources: Vec<(String, String)> = files
        .iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(path).ok()?;
            let rel = path
                .strip_prefix(crates)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            Some((rel, text))
        })
        .collect();
    let (mut entry_words, mut public) = (HashMap::new(), HashMap::new());
    let (mut test_paths, mut test_dirs) = (Vec::new(), Vec::new());
    for (rel, text) in &sources {
        entries(text, &mut entry_words);
        constants(text, true, &mut public);
        for mount in test_spans(text).1 {
            let (paths, dir) = mounted_paths(rel, &mount);
            test_paths.extend(paths);
            test_dirs.push(format!("{dir}/"));
        }
    }
    sources
        .iter()
        .flat_map(|(rel, text)| {
            let mounted = test_file(rel)
                || test_paths.contains(rel)
                || test_dirs.iter().any(|d| rel.starts_with(d.as_str()));
            stand_ins(rel, text, mounted, &public, &entry_words)
        })
        .collect()
}

/// The warning list: one line per stand-in without a count (with the entry's count when one
/// exists), and a summary line per crate.
pub fn warnings(list: &[StandIn]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut per_crate: std::collections::BTreeMap<&str, [usize; 4]> = Default::default();
    for s in list {
        let krate = s.file.split('/').next().unwrap_or("");
        let counts = per_crate.entry(krate).or_default();
        counts[0] += 1;
        if s.counted {
            counts[1] += 1;
            continue;
        }
        let at = s
            .addr
            .map_or_else(|| "?".to_string(), |a| format!("{a:08x}"));
        let what = match (s.addr, s.entry_words) {
            (None, _) => {
                counts[3] += 1;
                "no word count; its address is not a number or a constant".to_string()
            }
            (Some(_), Some(n)) => {
                counts[2] += 1;
                format!("no word count; its entry takes {n}")
            }
            (Some(_), None) => {
                counts[3] += 1;
                "no word count; no entry: take it from the exe (vcall-words --words ADDR)"
                    .to_string()
            }
        };
        lines.push(format!("stand-in\t{}:{}\t{at}\t{what}", s.file, s.line));
    }
    for (krate, [all, counted, by_entry, other]) in per_crate {
        lines.push(format!(
            "summary\t{krate}\t{all} stand-ins: {counted} counted, {by_entry} to count from their \
             entry, {other} from the exe or by hand"
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stand_ins_are_listed_with_the_count_their_entry_takes() {
        let mut entry_words = HashMap::new();
        entries(
            "vec![entry!(0x0040_1000, get(Ptr<Foo>, u32, f64) -> u32), entry!(0x0040_2000, odd(Thing))]",
            &mut entry_words,
        );
        assert_eq!(entry_words[&0x0040_1000], Some(4));
        assert_eq!(
            entry_words[&0x0040_2000], None,
            "a type the check does not read"
        );
        let text = "const GET: u32 = 0x0040_1000;
fn game(e: &mut Engine) { e.register(GET, f); }
#[cfg(test)]
mod tests {
    const FAKE: u32 = 0x7400_0010;
    fn t(e: &mut Engine, at: u32) {
        e.register(GET, f);
        e.register_double(FAKE, |_, _| Ret::default());
        e.register_words(GET, 4, f);
        e.register(at, f);
    }
}
";
        let list = stand_ins("k/src/a.rs", text, false, &HashMap::new(), &entry_words);
        let got: Vec<(usize, Option<u32>, bool, Option<u32>)> = list
            .iter()
            .map(|s| (s.line, s.addr, s.counted, s.entry_words))
            .collect();
        assert_eq!(
            got,
            [
                (7, Some(0x0040_1000), false, Some(4)),
                (8, Some(0x7400_0010), false, None),
                (9, Some(0x0040_1000), true, Some(4)),
                (10, None, false, None),
            ]
        );
        let lines = warnings(&list);
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert!(lines[0].contains("its entry takes 4"));
        assert!(lines[1].contains("take it from the exe"));
        assert_eq!(
            lines[3],
            "summary\tk\t4 stand-ins: 1 counted, 1 to count from their entry, 2 from the exe or by hand"
        );
    }
}
