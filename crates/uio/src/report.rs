//! UI Organizer's start-up report in the play log (nv-rs's own: UIO 2.30 writes its own
//! `ui_organizer.log` with the same facts, which the built-in leaves out): which lists were read,
//! what they add and where, and why each entry left out was left out, in plain words. Printed once
//! when the lists have been read ([`startup_lines`]) and once when the first file with additions is
//! read ([`first_read_line`]). The row off, UIO never loads and nothing is printed.

use crate::lists::SUPPORTED_NAME;

/// What became of one entry of a list (UIO's own log line for it, `UIOLoad`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Put in the lists ("+ Adding to").
    Added,
    /// The same path was already listed for that tile ("File already added").
    AlreadyAdded,
    /// `Data\menus\prefabs\<path>` is not a loose file ("File not found").
    PrefabMissing,
    /// `@file` with no tile, and the file is not below `prefabs\` ("Injecting @ EoF is only
    /// allowed with prefab files").
    EndOfFileNotPrefab,
    /// A target that is not one of UIO's 33 menu names ("Invalid target menu").
    UnknownMenu,
    /// `@file` longer than UIO's path buffer (nv-rs: passed over, where UIO overruns it).
    TargetTooLong,
    /// The condition line answered false ("Conditions evaluated as FALSE").
    ConditionFalse,
}

/// One entry of a list: its path line's parts, its condition line, what became of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The list file's name (`supported.txt`, or the public list's file name).
    pub list: Vec<u8>,
    /// The file to include, below `Data\menus\prefabs\`.
    pub path: Vec<u8>,
    /// The target as written: a menu name, or `@` and a file below `Data\menus\`.
    pub target: Vec<u8>,
    /// The target file (`Data\...`); empty when the entry stopped before it was known.
    pub file: Vec<u8>,
    /// The tile ("" = the end of the file).
    pub tile: Vec<u8>,
    /// The condition line as evaluated (a public list's spaces made tabs); empty when not read.
    pub condition: Vec<u8>,
    pub outcome: Outcome,
}

/// One list file UIO tried to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListRead {
    pub name: Vec<u8>,
    pub opened: bool,
}

/// What the lists' scan saw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub lists: Vec<ListRead>,
    pub entries: Vec<Entry>,
}

/// What went wrong after the lists, while patching a target file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The game could not read the target file: its additions were not put in.
    Unreadable { file: Vec<u8>, additions: usize },
    /// The tile was not found in the file: its additions were not put in.
    TileMissing {
        file: Vec<u8>,
        tile: Vec<u8>,
        additions: usize,
    },
}

/// The most "not added" lines printed; the rest are counted in one line.
pub const MAX_SKIPPED_LINES: usize = 40;

const PREFIX: &str = "UI Organizer: ";

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// A target file as UIO's log names it: without `Data\Menus\` (UIO's `menuFile + 11`).
fn short_file(file: &[u8]) -> String {
    let head = b"data\\menus\\";
    if file.len() > head.len() && file[..head.len()].eq_ignore_ascii_case(head) {
        text(&file[head.len()..])
    } else {
        text(file)
    }
}

fn tile_name(tile: &[u8]) -> String {
    if tile.is_empty() {
        "the end of the file".to_string()
    } else {
        format!("tile {}", text(tile))
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Why a condition line answered false, in plain words: a lone `#file` or `?plugin` (with or
/// without `!`) is named; anything else is quoted.
pub fn condition_reason(condition: &[u8]) -> String {
    let tokens: Vec<&[u8]> = condition
        .split(|&c| c == b'\t')
        .filter(|t| !t.is_empty())
        .collect();
    if let [token] = tokens.as_slice() {
        let (negated, t) = match token.split_first() {
            Some((b'!', rest)) => (true, rest),
            _ => (false, *token),
        };
        match t.split_first() {
            Some((b'#', file)) => {
                let is = if negated { "is" } else { "is not" };
                return format!("Prefabs\\{} {is} installed", text(file));
            }
            Some((b'?', plugin)) => {
                let is = if negated { "is" } else { "is not" };
                return format!("{} {is} in plugins.txt", text(plugin));
            }
            _ => {}
        }
    }
    let shown: Vec<String> = tokens.iter().copied().map(text).collect();
    format!("its condition is false ({})", shown.join(" "))
}

/// Why `entry` was not added, in plain words (`None` for an added one).
pub fn reason(entry: &Entry) -> Option<String> {
    Some(match entry.outcome {
        Outcome::Added => return None,
        Outcome::AlreadyAdded => format!(
            "it is already added to {} at {}",
            short_file(&entry.file),
            tile_name(&entry.tile)
        ),
        Outcome::PrefabMissing => format!("Prefabs\\{} is not installed", text(&entry.path)),
        Outcome::EndOfFileNotPrefab => format!(
            "an addition at the end of a file must go into a Prefabs file, not {}",
            text(&entry.target)
        ),
        Outcome::UnknownMenu => format!("{} is not a menu UIO knows", text(&entry.target)),
        Outcome::TargetTooLong => format!("the target {} is too long", text(&entry.target)),
        Outcome::ConditionFalse => condition_reason(&entry.condition),
    })
}

/// The report printed once the lists are read: the lists read (and those that would not open),
/// the additions by target file and tile, each problem met while patching, and each entry not
/// added with its reason (at most [`MAX_SKIPPED_LINES`], then a count).
pub fn startup_lines(scan: &Scan, problems: &[Problem]) -> Vec<String> {
    let mut out = Vec::new();

    let opened: Vec<String> = scan
        .lists
        .iter()
        .filter(|l| l.opened)
        .map(|l| text(&l.name))
        .collect();
    let mut first = if opened.is_empty() {
        format!("{PREFIX}read no list file")
    } else {
        format!(
            "{PREFIX}read {}: {}",
            plural(opened.len(), "list file", "list files"),
            opened.join(", ")
        )
    };
    for l in scan.lists.iter().filter(|l| !l.opened) {
        if l.name == SUPPORTED_NAME {
            first.push_str(
                "; supported.txt could not be opened (UIO's own list of the UI mods it knows, \
                 Data\\uio\\supported.txt in the UIO download)",
            );
        } else {
            first.push_str(&format!("; {} could not be opened", text(&l.name)));
        }
    }
    out.push(first);

    // The additions, grouped by target file and tile in the order first met.
    let mut groups: Vec<(&[u8], &[u8], Vec<String>)> = Vec::new();
    for e in scan.entries.iter().filter(|e| e.outcome == Outcome::Added) {
        let at = groups.iter().position(|(f, t, _)| {
            f.eq_ignore_ascii_case(&e.file) && t.eq_ignore_ascii_case(&e.tile)
        });
        match at {
            Some(i) => groups[i].2.push(text(&e.path)),
            None => groups.push((e.file.as_slice(), e.tile.as_slice(), vec![text(&e.path)])),
        }
    }
    let added: usize = groups.iter().map(|g| g.2.len()).sum();
    if added == 0 {
        out.push(format!("{PREFIX}no additions"));
    } else {
        let parts: Vec<String> = groups
            .iter()
            .map(|(f, t, paths)| {
                format!(
                    "{} at {}: {}",
                    short_file(f),
                    tile_name(t),
                    paths.join(", ")
                )
            })
            .collect();
        out.push(format!(
            "{PREFIX}{}: {}",
            plural(added, "addition", "additions"),
            parts.join("; ")
        ));
    }

    for p in problems {
        out.push(match p {
            Problem::Unreadable { file, additions } => format!(
                "{PREFIX}{} could not be read, so its {} not put in",
                short_file(file),
                plural(*additions, "addition was", "additions were")
            ),
            Problem::TileMissing {
                file,
                tile,
                additions,
            } => format!(
                "{PREFIX}{} has no tile {}, so {} there not put in",
                short_file(file),
                text(tile),
                plural(*additions, "addition was", "additions were")
            ),
        });
    }

    let skipped: Vec<String> = scan
        .entries
        .iter()
        .filter_map(|e| {
            reason(e).map(|why| {
                format!(
                    "{PREFIX}{}: {} not added: {why}",
                    text(&e.list),
                    text(&e.path)
                )
            })
        })
        .collect();
    let more = skipped.len().saturating_sub(MAX_SKIPPED_LINES);
    out.extend(skipped.into_iter().take(MAX_SKIPPED_LINES));
    if more > 0 {
        out.push(format!(
            "{PREFIX}and {} not added",
            plural(more, "more entry", "more entries")
        ));
    }
    out
}

/// The line printed when the first file with additions is read (the game building a menu, or a
/// menu including a patched prefab).
pub fn first_read_line(file: &[u8], additions: usize) -> String {
    format!(
        "{PREFIX}first file with additions read: {} ({})",
        short_file(file),
        plural(additions, "addition", "additions")
    )
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
