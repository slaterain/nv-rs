//! Every call a built-in extender (xNVSE `crates/nvse`, JIP LN `crates/jip`, UI Organizer
//! `crates/uio`, MCM `crates/mcm`, lStewieAl's Tweaks `crates/stewie`) makes to one of the game's
//! functions by a constant address passes as many argument words as that function's translation
//! takes: a call with too few stops the game ("call has N argument words, needs more"), one with
//! too many hands a later argument to nobody. Here because this crate registers every translation.
//! (A player's modded game stopped that way in xNVSE's lambdas, whose two list copies passed two
//! words of three, when this test read the jip crate only.)
//!
//! The test reads each crate's source (not its tests: `_tests.rs` files and `#[cfg(test)]` items
//! are left out): each `.call(NAME, &args![..])` or `.call(NAME, &[..])` whose callee is a
//! `const NAME: u32` of the crate, an address written in place (`0x005a_b930`) or one of two such
//! in an `if`, and each call of a helper that hands its `u32` parameter on as the callee
//! (`fn call_with_text(e, f: u32, ..)` calling `e.call(f, ..)`) with such a callee. The words are
//! counted by each item's type as its text gives it: a literal's suffix (a float literal without
//! one is an `f64`), a cast, a method or path named for its type (`.u32()`, `e.mem.f64(at)`,
//! `f64::from(x)`), `.st0`, a constant, a field of the crates' structs, a `let` or parameter of
//! the calling function. An item whose type the text does not give counts as one word (their
//! number is printed). Each callee's word count is probed on a fresh engine: the translation is
//! called with 0, 1, 2, ... zero words until its argument reads stop asking for more (`entry!`
//! reads every argument before the body runs, an entry that asserts its count says so, and a
//! nested call stops the probe before it reads its own). A callee nobody has translated is listed,
//! not failed (a call to it stops the game whatever it passes), but its calls must agree.
//!
//! The calls the reader cannot check by itself are listed in [`UNREAD`] with their words checked
//! by hand: calls through a vtable slot (the target depends on the object's class), calls to an
//! address read at run time (a hook's original, a menu's handler, a command's routine), and word
//! lists built elsewhere (`w`, `&words`). A call of that kind that is not listed fails, so each
//! new one is looked at once; a listed list written in place must still pass the listed words,
//! and a listed constant callee is probed for them. For calls through a vtable, the ignored test
//! below probes listed translations (`NV_PROBE`), and `NV_JIP_SRC` (`NV_NVSE_SRC`, `NV_UIO_SRC`,
//! ...) points a crate's test at another copy of its source (an open pull request's branch,
//! exported).

use engine as _;
use engine_core::prelude::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// The extender crates the test reads.
const CRATES: [&str; 5] = ["jip", "nvse", "uio", "mcm", "stewie"];

/// One direct call in a crate's source.
#[derive(Debug)]
struct Site {
    at: String,
    callee: String,
    addr: u32,
    words: usize,
    /// The list's last item is a literal 0 (`ShowMessageBox`'s closing word).
    zero_last: bool,
}

/// A call the reader could not check by itself.
#[derive(Debug)]
struct Unread {
    at: String,
    file: String,
    callee: String,
    why: String,
    /// The words its list passes, when the list is written in place.
    words: Option<usize>,
    /// The callee's address, when it is a constant.
    addr: Option<u32>,
}

/// The words a call listed in [`UNREAD`] passes, counted by hand.
#[derive(Clone, Copy, Debug)]
enum Words {
    /// Always this many.
    N(usize),
    /// This many or more (a list that grows with its data).
    AtLeast(usize),
}

impl Words {
    fn takes(self, n: usize) -> bool {
        match self {
            Words::N(k) => n == k,
            Words::AtLeast(k) => n >= k,
        }
    }
    fn least(self) -> usize {
        match self {
            Words::N(k) | Words::AtLeast(k) => k,
        }
    }
}

use Words::{AtLeast, N};

/// One row of [`UNREAD`]: (crate, file, callee as written, the words it passes, how they were
/// checked).
type Row = (&'static str, &'static str, &'static str, Words, &'static str);

/// The calls the reader cannot check by itself, each checked by hand ([`Row`]). A call listed
/// twice in a file is listed twice. What "a command's words" are: the eight of `COMMAND_ARGS`
/// (`paramInfo`, `scriptData`, `thisObj`, `containingObj`, `scriptObj`, `eventList`, `result`,
/// `opcodeOffsetPtr`), which the game's script runner passes to every command's routine.
const UNREAD: &[Row] = &[
    // JIP LN.
    (
        "jip",
        "commands/jip_fn_actor_p2.rs",
        "KILL_ACTOR_EXECUTE",
        N(8),
        "the command's own words, handed to the game's KillActor routine",
    ),
    (
        "jip",
        "commands/jip_fn_actor_p2.rs",
        "ATTACH_ASH_PILE",
        N(8),
        "the command's own words, handed to the game's AttachAshPile routine",
    ),
    (
        "jip",
        "commands/jip_fn_actor_p2.rs",
        "e.mem.u32(vtbl + VT_IS_CHARACTER)",
        N(1),
        "a vtable slot taking only `this`",
    ),
    (
        "jip",
        "commands/jip_fn_actor_p2.rs",
        "function",
        AtLeast(2),
        "a queued command's routine: the reference, 0, then the task's own words",
    ),
    (
        "jip",
        "commands/jip_fn_ccc.rs",
        "SAY_TO",
        N(8),
        "the command's own words, padded with zeros to eight",
    ),
    (
        "jip",
        "commands/jip_fn_misc_ref.rs",
        "f",
        N(1),
        "a vtable slot (GetAnimData) taking only `this`",
    ),
    (
        "jip",
        "commands/jip_fn_misc_ref.rs",
        "destroy",
        N(2),
        "the reference's destructor: `this` and the free flag",
    ),
    (
        "jip",
        "commands/jip_fn_misc_ref_insert.rs",
        "f",
        N(1),
        "a vtable slot (IsBoundObject) taking only `this`",
    ),
    (
        "jip",
        "commands/jip_fn_misc_ref_nodes.rs",
        "f",
        N(1),
        "a vtable slot (IsBoundObject) taking only `this`",
    ),
    (
        "jip",
        "commands/jip_fn_miscellaneous_scene.rs",
        "delete",
        N(1),
        "a vtable slot taking only `this`",
    ),
    (
        "jip",
        "commands/jip_fn_ui.rs",
        "hash",
        N(2),
        "the trait map's hash routine from its vtable: the map and the key",
    ),
    (
        "jip",
        "commands/jip_fn_ui.rs",
        "equal",
        N(3),
        "the trait map's compare routine from its vtable: the map and two keys",
    ),
    (
        "jip",
        "commands/jip_fn_ui.rs",
        "destroy",
        N(2),
        "the tile's destructor: `this` and the free flag",
    ),
    (
        "jip",
        "commands/jip_fn_ui.rs",
        "handler",
        N(3),
        "the menu's click handler from its vtable: the menu, the tile's id and the tile",
    ),
    (
        "jip",
        "commands/jip_fn_ui.rs",
        "SHOW_MESSAGE_BOX",
        AtLeast(9),
        "MessageBoxExAlt's box: eight words, its buttons, then the closing 0",
    ),
    (
        "jip",
        "commands/jip_fn_utility.rs",
        "e.mem.u32(vtbl + 4)",
        N(2),
        "the editor-id map's hash routine from its vtable: the map and the key",
    ),
    (
        "jip",
        "commands/jip_fn_utility.rs",
        "equal",
        N(3),
        "the editor-id map's compare routine from its vtable: the map and two keys",
    ),
    (
        "jip",
        "game_patches/gx.rs",
        "delete",
        N(1),
        "a vtable slot taking only `this`",
    ),
    (
        "jip",
        "ui_hooks.rs",
        "original",
        N(3),
        "the hooked menu's own click routine: the menu, the tile's id and the tile",
    ),
    // xNVSE.
    (
        "nvse",
        "commands/game.rs",
        "SHOW_MESSAGE_BOX",
        N(19),
        "MessageBoxEx's box: eight words, ten buttons, the closing 0",
    ),
    (
        "nvse",
        "commands/game.rs",
        "OUTPUT_LOCAL_MAP_PICTURES",
        N(8),
        "the command's own words, handed to the game's OutputLocalMapPictures",
    ),
    (
        "nvse",
        "commands/game.rs",
        "ADD_SPELL",
        N(8),
        "the command's own words, handed to the game's AddSpell",
    ),
    (
        "nvse",
        "compiler.rs",
        "parse",
        N(4),
        "a command's parse routine from its record: the count, parameters, line and buffer",
    ),
    (
        "nvse",
        "expr.rs",
        "execute",
        N(8),
        "a command's routine from its record: a command's words",
    ),
    (
        "nvse",
        "functions.rs",
        "execute",
        N(8),
        "a command's routine from its record: a command's words",
    ),
    // UI Organizer.
    (
        "uio",
        "build.rs",
        "hash",
        N(2),
        "the tile map's hash routine from its vtable: the map and the key",
    ),
    (
        "uio",
        "build.rs",
        "equal",
        N(3),
        "the tile map's compare routine from its vtable: the map and two keys",
    ),
    (
        "uio",
        "build.rs",
        "hash",
        N(2),
        "the tile map's hash routine, on insert",
    ),
    (
        "uio",
        "build.rs",
        "equal",
        N(3),
        "the tile map's compare routine, on insert",
    ),
];

/// The source folder of extender crate `name` (`jip`, `nvse`, ...); `NV_<NAME>_SRC=FOLDER`
/// (`NV_JIP_SRC`, `NV_NVSE_SRC`, ...) reads another copy instead (a sweep of an open pull
/// request's files, exported from its branch).
fn crate_src(name: &str) -> PathBuf {
    std::env::var_os(format!("NV_{}_SRC", name.to_ascii_uppercase()))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(name)
                .join("src")
        })
}

/// A crate's source files, without its test files.
fn sources(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("extender source folder") {
            let path = entry.expect("folder entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|x| x == "rs") {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if !name.ends_with("_tests.rs") && name != "test_support.rs" {
                    out.push(path);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

/// A crate's files as (path from its source folder, code).
fn files_of(root: &Path) -> Vec<(String, String)> {
    sources(root)
        .into_iter()
        .map(|p| {
            let name = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (name, code_of(&std::fs::read_to_string(&p).expect("source")))
        })
        .collect()
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Spaces for `from..to` (line breaks kept, so offsets keep their lines).
fn blank(out: &mut [u8], from: usize, to: usize) {
    let to = to.min(out.len());
    for c in out[from.min(to)..to].iter_mut() {
        if *c != b'\n' {
            *c = b' ';
        }
    }
}

/// The text with comments and the insides of strings and char literals blanked (same length, so
/// offsets keep their lines), and every `#[cfg(test)]` item blanked.
fn code_of(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let after_ident = i > 0 && is_ident(b[i - 1]);
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            let end = b[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(b.len(), |k| i + k);
            blank(&mut out, i, end);
            i = end;
            continue;
        }
        // A raw string: `r"..."`, `r#"..."#`, `br"..."`.
        let raw =
            c == b'r' && (!after_ident || (b[i - 1] == b'b' && (i < 2 || !is_ident(b[i - 2]))));
        if raw {
            let hashes = b[i + 1..].iter().take_while(|&&c| c == b'#').count();
            if b.get(i + 1 + hashes) == Some(&b'"') {
                let start = i + 2 + hashes;
                let mut j = start;
                while j < b.len()
                    && !(b[j] == b'"' && b[j + 1..].iter().take(hashes).all(|&c| c == b'#'))
                {
                    j += 1;
                }
                blank(&mut out, start, j);
                i = j + 1 + hashes;
                continue;
            }
        }
        if c == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            blank(&mut out, start, j);
            i = j + 1;
            continue;
        }
        // A char literal (`'"'`, `'\''`, `'\u{7f}'`), not a lifetime.
        if c == b'\'' && i + 2 < b.len() {
            let close = if b[i + 1] == b'\\' {
                b[i + 3..]
                    .iter()
                    .position(|&c| c == b'\'')
                    .map(|k| i + 3 + k)
            } else if b[i + 2] == b'\'' {
                Some(i + 2)
            } else {
                None
            };
            if let Some(close) = close {
                blank(&mut out, i + 1, close);
                i = close + 1;
                continue;
            }
        }
        i += 1;
    }
    let mut from = 0;
    const TEST: &[u8] = b"#[cfg(test)]";
    while let Some(k) = out[from..].windows(TEST.len()).position(|w| w == TEST) {
        let start = from + k;
        let end = item_end(&out, start + TEST.len());
        blank(&mut out, start, end);
        from = end;
    }
    String::from_utf8(out).expect("utf-8 source")
}

/// Where the item starting at `at` ends (after its attributes): its `;`, its `,` (a field or an
/// array element), or the `}` that closes its first block.
fn item_end(b: &[u8], mut at: usize) -> usize {
    loop {
        while at < b.len() && b[at].is_ascii_whitespace() {
            at += 1;
        }
        if b[at..].starts_with(b"#[") {
            let mut depth = 0;
            while at < b.len() {
                match b[at] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            at += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                at += 1;
            }
            continue;
        }
        break;
    }
    let head = String::from_utf8_lossy(&b[at..(at + 24).min(b.len())]).into_owned();
    let is_fn = [
        "fn ",
        "pub fn ",
        "pub(crate) fn ",
        "const fn ",
        "unsafe fn ",
    ]
    .iter()
    .any(|p| head.starts_with(p));
    let mut depth = 0i32;
    for (i, &c) in b.iter().enumerate().skip(at) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' => depth -= 1,
            b'}' => {
                depth -= 1;
                if depth <= 0 {
                    return i + 1;
                }
            }
            b';' if depth == 0 => return i + 1,
            b',' if depth == 0 && !is_fn => return i + 1,
            _ => {}
        }
    }
    b.len()
}

/// The top-level items of a comma-separated list.
fn items(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in list.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The arguments of the call whose `(` is at `open`.
fn call_args(code: &str, open: usize) -> Option<&str> {
    let b = code.as_bytes();
    let mut depth = 0;
    for (i, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&code[open + 1..i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_const_name(text: &str) -> bool {
    !text.is_empty()
        && text.starts_with(|c: char| c.is_ascii_uppercase())
        && text
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn is_name(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(|c: char| c.is_ascii_digit())
        && text.bytes().all(is_ident)
}

/// `const NAME: u32 = 0x...;` values of every file, by name (a name given two values is dropped
/// unless the calling file has its own).
fn consts(files: &[(String, String)]) -> BTreeMap<String, Vec<(String, u32)>> {
    let mut out: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for (name, code) in files {
        let mut rest = code.as_str();
        while let Some(at) = rest.find("const ") {
            rest = &rest[at + 6..];
            let Some((ident, tail)) = rest.split_once(':') else {
                break;
            };
            let ident = ident.trim();
            let tail = tail.trim_start();
            if !is_const_name(ident) || !tail.starts_with("u32") {
                continue;
            }
            let Some((_, value)) = tail.split_once('=') else {
                continue;
            };
            let Some((value, _)) = value.split_once(';') else {
                continue;
            };
            let value = value.trim().replace('_', "");
            let parsed = match value.strip_prefix("0x") {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => value.parse().ok(),
            };
            if let Some(v) = parsed {
                out.entry(ident.to_string())
                    .or_default()
                    .push((name.clone(), v));
            }
        }
    }
    out
}

/// An address written in place (`0x005a_b930`), or none.
fn literal_address(text: &str) -> Option<u32> {
    let hex = text.strip_prefix("0x")?.replace('_', "");
    let hex = hex.strip_suffix("u32").unwrap_or(&hex);
    u32::from_str_radix(hex, 16).ok()
}

/// The words an argument of type `ty` takes, when the reader knows the type.
fn type_words(ty: &str) -> Option<usize> {
    let ty = ty.trim();
    match ty {
        "f64" | "u64" | "i64" => Some(2),
        "u32" | "i32" | "f32" | "u16" | "i16" | "u8" | "i8" | "bool" => Some(1),
        "[u32; 16]" => Some(16),
        _ if ty.starts_with("Ptr<") => Some(1),
        _ => None,
    }
}

/// What the reader knows of the crates' types: each constant's type, and each struct field's
/// words by name (a name whose fields take different words is left out).
#[derive(Default)]
struct Types {
    consts: BTreeMap<String, Option<usize>>,
    fields: BTreeMap<String, Option<usize>>,
}

impl Types {
    fn of(files: &[(String, String)]) -> Types {
        fn add(map: &mut BTreeMap<String, Option<usize>>, name: &str, words: Option<usize>) {
            map.entry(name.to_string())
                .and_modify(|w| {
                    if *w != words {
                        *w = None;
                    }
                })
                .or_insert(words);
        }
        let mut t = Types::default();
        for (_, code) in files {
            for key in ["const ", "static "] {
                let mut rest = code.as_str();
                while let Some(at) = rest.find(key) {
                    rest = &rest[at + key.len()..];
                    let Some((ident, tail)) = rest.split_once(':') else {
                        break;
                    };
                    let ident = ident.trim();
                    if !is_const_name(ident) {
                        continue;
                    }
                    let ty = tail.split(['=', ';']).next().unwrap_or("");
                    add(&mut t.consts, ident, type_words(ty));
                }
            }
            let mut from = 0;
            while let Some(k) = code[from..].find("struct ") {
                let at = from + k + 7;
                from = at;
                if at > 7 && is_ident(code.as_bytes()[at - 8]) {
                    continue;
                }
                let rest = &code[at..];
                let Some(open) = rest.find(['{', ';', '(']) else {
                    break;
                };
                if !rest[open..].starts_with('{') {
                    continue;
                }
                let Some(body) = call_args(code, at + open) else {
                    continue;
                };
                for field in items(body) {
                    let mut field = field.as_str();
                    while field.starts_with("#[") {
                        field = field.split_once(']').map_or("", |(_, r)| r).trim_start();
                    }
                    for vis in ["pub(crate) ", "pub(super) ", "pub "] {
                        field = field.strip_prefix(vis).unwrap_or(field);
                    }
                    if let Some((name, ty)) = field.split_once(':') {
                        if is_name(name.trim()) {
                            add(&mut t.fields, name.trim(), type_words(ty));
                        }
                    }
                }
            }
        }
        t
    }
}

/// The function around offset `at`: its name, its parameters' text, where it starts, and whether
/// it is visible outside its file.
struct Fun<'a> {
    name: &'a str,
    params: &'a str,
    start: usize,
    public: bool,
}

fn fn_around(code: &str, at: usize) -> Option<Fun<'_>> {
    let b = code.as_bytes();
    let mut k = at;
    let start = loop {
        let found = code[..k].rfind("fn ")?;
        if found == 0 || !is_ident(b[found - 1]) {
            break found;
        }
        k = found;
    };
    let rest = &code[start + 3..];
    let len = rest.bytes().position(|c| !is_ident(c))?;
    let open = start + 3 + rest.find('(')?;
    let line = code[..start].rfind('\n').map_or(0, |n| n + 1);
    Some(Fun {
        name: &rest[..len],
        params: call_args(code, open)?,
        start,
        public: code[line..start].contains("pub"),
    })
}

/// An expression's top-level operands: `Err(())` for a comparison or a logical operation (a
/// `bool`), else the operands of its arithmetic and bit operations (the whole text for none).
fn operands(text: &str) -> Result<Vec<&str>, ()> {
    const LOGIC: [&[u8]; 8] = [
        b" == ", b" != ", b" < ", b" > ", b" <= ", b" >= ", b" && ", b" || ",
    ];
    const ARITH: [&[u8]; 10] = [
        b" + ", b" - ", b" * ", b" / ", b" % ", b" | ", b" & ", b" ^ ", b" << ", b" >> ",
    ];
    let b = text.as_bytes();
    let mut depth = 0i32;
    let mut out = Vec::new();
    let mut from = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b' ' if depth == 0 => {
                if LOGIC.iter().any(|op| b[i..].starts_with(op)) {
                    return Err(());
                }
                if let Some(op) = ARITH.iter().find(|op| b[i..].starts_with(op)) {
                    out.push(&text[from..i]);
                    i += op.len();
                    from = i;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&text[from..]);
    Ok(out)
}

/// The words argument `item` takes, by the type its text gives, or none when the text does not
/// say. `scope` is the calling function's text before the call (for its `let`s), `params` its
/// parameters.
fn item_words(item: &str, scope: &str, params: &str, types: &Types, depth: u32) -> Option<usize> {
    const NAMED: [&str; 11] = [
        "f64", "u64", "i64", "f32", "u32", "i32", "u16", "i16", "u8", "i8", "bool",
    ];
    let item = item.trim();
    if depth > 4 || item.is_empty() {
        return None;
    }
    // A cast: `x as f64`.
    for t in NAMED {
        if item
            .strip_suffix(t)
            .is_some_and(|head| head.ends_with(" as "))
        {
            return type_words(t);
        }
    }
    // A literal: an integer is one word unless its suffix says otherwise, a float two (`f64`
    // unless it says `f32`).
    let digits = item.strip_prefix('-').unwrap_or(item);
    if digits.starts_with(|c: char| c.is_ascii_digit()) {
        let suffix = NAMED.iter().find(|t| {
            digits.strip_suffix(**t).is_some_and(|h| {
                !h.is_empty() && (!digits.starts_with("0x") || t.starts_with(['u', 'i']))
            })
        });
        return match suffix {
            Some(t) => type_words(t),
            None if !digits.starts_with("0x")
                && digits.contains(['.', 'e', 'E'])
                && digits
                    .bytes()
                    .all(|c| c.is_ascii_digit() || b"._eE-+".contains(&c)) =>
            {
                Some(2)
            }
            None if digits
                .bytes()
                .all(|c| c.is_ascii_hexdigit() || b"_x".contains(&c)) =>
            {
                Some(1)
            }
            None => None,
        };
    }
    if item == "true" || item == "false" {
        return Some(1);
    }
    if item.ends_with(".st0") {
        return Some(2);
    }
    if item.ends_with(".eax") || item.ends_with(".edx") {
        return Some(1);
    }
    // `!x`, `-x`, `(x)`.
    if let Some(inner) = item.strip_prefix('!').or_else(|| item.strip_prefix('-')) {
        return item_words(inner, scope, params, types, depth + 1);
    }
    if item.starts_with('(') && call_args(item, 0).map(str::len) == Some(item.len() - 2) {
        return item_words(&item[1..item.len() - 1], scope, params, types, depth + 1);
    }
    match operands(item) {
        Err(()) => return Some(1),
        Ok(parts) if parts.len() > 1 => {
            return parts
                .iter()
                .find_map(|p| item_words(p, scope, params, types, depth + 1));
        }
        Ok(_) => {}
    }
    // A method or path named for its type: `.u32()`, `e.mem.f64(at)`, `f64::from(x)`,
    // `e.global::<u32>(X)`, `Ptr::new(at)`; `to_bits` takes its receiver's.
    if item.ends_with(')') {
        let b = item.as_bytes();
        let mut depth_ = 0;
        let mut open = None;
        for i in (0..b.len()).rev() {
            match b[i] {
                b')' | b']' | b'}' => depth_ += 1,
                b'(' | b'[' | b'{' => {
                    depth_ -= 1;
                    if depth_ == 0 {
                        open = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let head = &item[..open?];
        if let Some(t) = head.strip_suffix('>').and_then(|h| h.rsplit_once("::<")) {
            return type_words(t.1);
        }
        let name_at = head
            .bytes()
            .rposition(|c| !is_ident(c))
            .map_or(0, |k| k + 1);
        let name = &head[name_at..];
        if NAMED.contains(&name) {
            return type_words(name);
        }
        let before = &head[..name_at];
        if let Some(path) = before.strip_suffix("::") {
            let ty = &path[path
                .bytes()
                .rposition(|c| !is_ident(c))
                .map_or(0, |k| k + 1)..];
            if ty == "Ptr" {
                return Some(1);
            }
            return type_words(ty);
        }
        if name == "to_bits" {
            let receiver = before.strip_suffix('.')?;
            return item_words(receiver, scope, params, types, depth + 1).map(|w| w.min(2));
        }
        return None;
    }
    let last = item.rsplit("::").next().unwrap_or(item);
    if is_const_name(last) {
        return types.consts.get(last).copied().flatten();
    }
    // A field: `a.this_obj`, `cut.message`.
    if let Some((_, field)) = item.rsplit_once('.') {
        if item.split('.').all(is_name) {
            return types.fields.get(field).copied().flatten();
        }
        return None;
    }
    if !is_name(item) {
        return None;
    }
    // The last `let` of the name before the call, else a parameter of the function.
    if let Some(l) = last_let(scope, item) {
        if let Some(ty) = l.ty {
            return type_words(ty);
        }
        return item_words(l.value?, &scope[..l.at], params, types, depth + 1);
    }
    for p in items(params) {
        if let Some((name, ty)) = p.split_once(':') {
            let name = name.trim();
            if name.strip_prefix("mut ").unwrap_or(name).trim() == item {
                return type_words(ty);
            }
        }
    }
    None
}

/// Where the first of `stops` outside brackets is in `text` (its end when none is).
fn top_level_stop(text: &str, stops: &[u8]) -> usize {
    let mut depth = 0i32;
    for (i, c) in text.bytes().enumerate() {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0 && stops.contains(&c) => return i,
            _ => {}
        }
    }
    text.len()
}

/// A `let` of a name ([`last_let`]): its type and its value as written, whether it is `mut`, and
/// where it is.
struct Let<'a> {
    ty: Option<&'a str>,
    value: Option<&'a str>,
    mutable: bool,
    at: usize,
}

/// The last `let` of `name` in `scope` (the calling function's text before the call).
fn last_let<'a>(scope: &'a str, name: &str) -> Option<Let<'a>> {
    let b = scope.as_bytes();
    let mut k = scope.len();
    while let Some(at) = scope[..k].rfind(name) {
        k = at;
        let end = at + name.len();
        if (at > 0 && is_ident(b[at - 1])) || b.get(end).is_some_and(|&c| is_ident(c)) {
            continue;
        }
        let head = scope[..at].trim_end();
        let (head, mutable) = match head.strip_suffix("mut") {
            Some(h) if h.ends_with(char::is_whitespace) => (h.trim_end(), true),
            _ => (head, false),
        };
        if !head.ends_with("let") || (head.len() > 3 && is_ident(head.as_bytes()[head.len() - 4])) {
            continue;
        }
        let rest = scope[end..].trim_start();
        let (ty, rest) = match rest.strip_prefix(':') {
            Some(t) => {
                let stop = top_level_stop(t, b"=;");
                (Some(t[..stop].trim()), &t[stop..])
            }
            None => (None, rest),
        };
        let value = rest
            .strip_prefix('=')
            .map(|v| v[..top_level_stop(v, b";")].trim());
        return Some(Let {
            ty,
            value,
            mutable,
            at,
        });
    }
    None
}

/// The callees a callee's text names: a constant, an address written in place, or one of two
/// such in an `if` (an error saying why when it names none).
fn resolve(
    text: &str,
    file: &str,
    consts: &BTreeMap<String, Vec<(String, u32)>>,
) -> Result<Vec<(String, u32)>, String> {
    let text = text.trim();
    if let Some(cond) = text.strip_prefix("if ") {
        let pair = cond
            .strip_suffix('}')
            .and_then(|c| c.rsplit_once("} else {"))
            .and_then(|(l, b)| Some((l.rsplit_once('{')?.1, b)));
        if let Some((a, b)) = pair {
            let mut out = resolve(a, file, consts)?;
            out.extend(resolve(b, file, consts)?);
            return Ok(out);
        }
        return Err(format!("callee `{text}` read at run time"));
    }
    let name = text.rsplit("::").next().unwrap().trim();
    if let Some(at) = literal_address(name) {
        return Ok(vec![(name.to_string(), at)]);
    }
    if !is_const_name(name) {
        return Err(format!("callee `{text}` read at run time"));
    }
    match consts.get(name).map(Vec::as_slice) {
        Some(values) => match values.iter().find(|(f, _)| f == file) {
            Some(&(_, v)) => Ok(vec![(name.to_string(), v)]),
            None if values.iter().all(|(_, v)| *v == values[0].1) => {
                Ok(vec![(name.to_string(), values[0].1)])
            }
            None => Err(format!("`{name}` has several values")),
        },
        None => Err(format!("`{name}` not a constant of the crate")),
    }
}

/// The items of a word list written in place (`&args![..]`, `&[..]`, `&[x; N]`), or held by a
/// `let` of the calling function (`scope`) that fixes its length (an array, or an `args!` or
/// `vec!` that is not `mut`); none when it is built elsewhere (`w`, a `vec!` pushed to).
fn list_items(list: &str, scope: &str) -> Option<Vec<String>> {
    let list = list.trim();
    let bare = list.strip_prefix('&').unwrap_or(list).trim();
    let inner = ["args!", "vec!", ""]
        .iter()
        .find_map(|m| bare.strip_prefix(m)?.trim().strip_prefix('['))
        .and_then(|l| l.strip_suffix(']'));
    let Some(inner) = inner else {
        let held = last_let(scope, bare).filter(|_| is_name(bare))?;
        let value = held.value?;
        if value.starts_with('[') || !held.mutable {
            return list_items(value, &scope[..held.at]);
        }
        return None;
    };
    let parts = items(inner);
    if let [one] = parts.as_slice() {
        if let Some((x, n)) = one.split_once(';') {
            return Some(vec![x.trim().to_string(); n.trim().parse().ok()?]);
        }
    }
    Some(parts)
}

/// A function that hands its `u32` parameter on as the callee of a call with a word list
/// written in place: each call of it with a constant there is a call of that constant.
struct Helper {
    file: String,
    name: String,
    public: bool,
    index: usize,
    words: usize,
    zero_last: bool,
    inner: Unread,
}

/// What one crate's reading found.
#[derive(Default)]
struct Reading {
    sites: Vec<Site>,
    unread: Vec<Unread>,
    /// Items counted as one word without their type read.
    untyped: usize,
    items: usize,
}

/// Every direct call by a constant address in the crate whose source is at `root`, and the calls
/// the test could not read.
fn sites(root: &Path, types: &Types) -> Reading {
    let files = files_of(root);
    let consts = consts(&files);
    let mut r = Reading::default();
    let mut helpers: Vec<Helper> = Vec::new();
    for (name, code) in &files {
        for (at, _) in code.match_indices(".call") {
            let tail = &code[at + 5..];
            // `.call(` and `.call_as(` / `.call_as::<T>(`; not `.call_next`, `.call1`, `.vcall`.
            let open = if tail.starts_with('(') {
                at + 5
            } else if let Some(t) = tail.strip_prefix("_as") {
                match t.find('(') {
                    Some(k) if t[..k].trim().is_empty() || t[..k].trim().starts_with("::<") => {
                        at + 8 + k
                    }
                    _ => continue,
                }
            } else {
                continue;
            };
            let line = code[..at].matches('\n').count() + 1;
            let where_ = format!("{name}:{line}");
            let unread = |callee: &str, why: String, words, addr| Unread {
                at: where_.clone(),
                file: name.clone(),
                callee: callee.to_string(),
                why,
                words,
                addr,
            };
            let Some(args) = call_args(code, open) else {
                r.unread
                    .push(unread("", "unbalanced".to_string(), None, None));
                continue;
            };
            let parts = items(args);
            // `Engine::call` takes the callee and the words: a `.call` with another count is
            // another type's method (a parser's, a lambda pass's), not a call into the game.
            if parts.len() != 2 {
                continue;
            }
            let fun = fn_around(code, at);
            let (scope, params) = fun
                .as_ref()
                .map_or(("", ""), |f| (&code[f.start..at], f.params));
            let list = list_items(&parts[1], scope).map(|list| {
                let mut words = 0;
                for item in &list {
                    let w = item_words(item, scope, params, types, 0);
                    r.items += 1;
                    if w.is_none() {
                        r.untyped += 1;
                    }
                    words += w.unwrap_or(1);
                }
                let zero_last = list
                    .last()
                    .is_some_and(|l| ["0", "0u32", "0_u32"].contains(&l.as_str()));
                (words, zero_last)
            });
            let callee = parts[0].as_str();
            // A callee held by a `let` that is not `mut`: its value's callees.
            let resolved = resolve(callee, name, &consts).or_else(|why| {
                let held = last_let(scope, callee).filter(|l| is_name(callee) && !l.mutable);
                held.and_then(|l| resolve(l.value?, name, &consts).ok())
                    .ok_or(why)
            });
            match (resolved, list) {
                (Ok(callees), Some((words, zero_last))) => {
                    for (callee, addr) in callees {
                        r.sites.push(Site {
                            at: where_.clone(),
                            callee,
                            addr,
                            words,
                            zero_last,
                        });
                    }
                }
                (Ok(callees), None) => r.unread.push(unread(
                    &callees[0].0,
                    format!("words built elsewhere (`{}`)", parts[1]),
                    None,
                    Some(callees[0].1),
                )),
                (Err(why), list) => {
                    let inner = unread(callee, why, list.map(|l| l.0), None);
                    let param = fun.as_ref().and_then(|f| {
                        let index = items(f.params).iter().position(|p| {
                            p.split_once(':').is_some_and(|(n, t)| {
                                n.trim().strip_prefix("mut ").unwrap_or(n.trim()) == callee
                                    && t.trim() == "u32"
                            })
                        })?;
                        Some((f, index))
                    });
                    match (param, list) {
                        (Some((f, index)), Some((words, zero_last))) if is_name(callee) => helpers
                            .push(Helper {
                                file: name.clone(),
                                name: f.name.to_string(),
                                public: f.public,
                                index,
                                words,
                                zero_last,
                                inner,
                            }),
                        _ => r.unread.push(inner),
                    }
                }
            }
        }
    }
    // The helpers' calls.
    for h in helpers {
        let mut found = 0;
        for (name, code) in files.iter().filter(|(f, _)| h.public || *f == h.file) {
            let b = code.as_bytes();
            for (at, _) in code.match_indices(&format!("{}(", h.name)) {
                if at > 0 && (is_ident(b[at - 1]) || b[at - 1] == b'.') {
                    continue;
                }
                if code[..at].trim_end().ends_with("fn") {
                    continue;
                }
                let Some(args) = call_args(code, at + h.name.len()) else {
                    continue;
                };
                let Some(callee) = items(args).into_iter().nth(h.index) else {
                    continue;
                };
                found += 1;
                let line = code[..at].matches('\n').count() + 1;
                let where_ = format!("{name}:{line}");
                match resolve(&callee, name, &consts) {
                    Ok(callees) => {
                        for (callee, addr) in callees {
                            r.sites.push(Site {
                                at: where_.clone(),
                                callee,
                                addr,
                                words: h.words,
                                zero_last: h.zero_last,
                            });
                        }
                    }
                    Err(why) => r.unread.push(Unread {
                        at: where_,
                        file: name.clone(),
                        callee,
                        why: format!("{why}, passed to `{}`", h.name),
                        words: Some(h.words),
                        addr: None,
                    }),
                }
            }
        }
        if found == 0 {
            r.unread.push(h.inner);
        }
    }
    r
}

/// The callees with a variable argument list (`cdecl`, `...`): the words they always take, and
/// whether the list ends with a 0. A call passes at least those (the rest follow the format or
/// the entry point's tables; the sweep checked them by hand).
const VARIADIC: [(u32, usize, bool); 3] = [
    // `BGSEntryPoint::HandleEntryPoint`: the entry point and the actor, then the filters and the
    // function's arguments.
    (0x005e_58f0, 2, false),
    // The console's formatted print: the format, then its words.
    (0x0070_3c00, 1, false),
    // `ShowMessageBox`: the text, callback and options (nine words, the first button among them),
    // then the other buttons and the closing 0 (its entry asserts the nine: xNVSE's
    // `DisplayMessage` passes ten, `MessageBoxEx` nineteen). With no button the 0 is the ninth.
    (0x0070_3e80, 9, true),
];

thread_local! {
    /// Set by the probe's call hook when the probed function's body calls another.
    static BODY_REACHED: Cell<bool> = const { Cell::new(false) };
}

/// The probe's call hook: a call made while another runs (the probed function's body) stops it
/// before that callee reads its own arguments.
fn stop_nested(e: &mut Engine, _addr: u32, returning: bool) {
    if !returning && !e.call_stack.is_empty() {
        BODY_REACHED.with(|b| b.set(true));
        panic!("arity probe: the body was reached");
    }
}

/// Whether a probe's stop with `msg` after `k` words says the call was short of words: the
/// reads' own message, an index past the words, or an entry's count it asserts ("00703e80 needs
/// 9 argument words").
fn short_of_words(msg: &str, k: usize) -> bool {
    if msg == format!("call has {k} argument words, needs more")
        || msg.starts_with(&format!("index out of bounds: the len is {k} but"))
    {
        return true;
    }
    if !msg.contains("argument words") {
        return false;
    }
    let mut rest = msg;
    while let Some(at) = rest.find("needs ") {
        rest = &rest[at + 6..];
        let n: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if n.parse::<usize>().is_ok_and(|n| k < n) {
            return true;
        }
    }
    false
}

/// How many argument words the function registered at `addr` reads before its body runs (the
/// first count that does not stop on a missing word), up to 32.
fn arity(make: fn() -> Engine, addr: u32) -> Option<usize> {
    for k in 0..=32usize {
        let mut e = make();
        e.set_call_hook(Some(stop_nested));
        BODY_REACHED.with(|b| b.set(false));
        let words = vec![0u32; k];
        let result = catch_unwind(AssertUnwindSafe(|| {
            e.call(addr, &words);
        }));
        let Err(payload) = result else {
            return Some(k);
        };
        let msg = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        if !short_of_words(&msg, k) || BODY_REACHED.with(Cell::get) {
            return Some(k);
        }
    }
    None
}

/// One crate's check at a time: the probe silences the process's panic hook while it runs, and
/// two crates' checks swapping it at once could leave it silent for a failing test's message.
static PROBING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Pairs crate `name`'s unread calls with [`UNREAD`]'s rows: the problems (an unlisted call, a
/// listed one that is gone or passes other words) and the pairs.
fn match_unread<'a>(name: &str, unread: &'a [Unread]) -> (Vec<String>, Vec<(&'a Unread, Words)>) {
    let mut rows: Vec<(&Row, bool)> = UNREAD
        .iter()
        .filter(|r| r.0 == name)
        .map(|r| (r, false))
        .collect();
    let mut problems = Vec::new();
    let mut pairs = Vec::new();
    for u in unread {
        let same = |r: &Row| r.1 == u.file && r.2 == u.callee;
        let pick = rows
            .iter()
            .position(|(r, used)| !used && same(r) && u.words.map_or(true, |n| r.3.takes(n)))
            .or_else(|| rows.iter().position(|(r, used)| !used && same(r)));
        match pick {
            Some(k) => {
                rows[k].1 = true;
                let words = rows[k].0 .3;
                if let Some(n) = u.words.filter(|&n| !words.takes(n)) {
                    problems.push(format!(
                        "{}: `{}` passes {n} words, UNREAD lists {words:?}",
                        u.at, u.callee
                    ));
                }
                pairs.push((u, words));
            }
            None => problems.push(format!(
                "{}: {} (not in UNREAD: check its words by hand, then list it as \
                 (\"{name}\", \"{}\", \"{}\", {}, \"...\"))",
                u.at,
                u.why,
                u.file,
                u.callee,
                u.words.map_or("N(?)".to_string(), |n| format!("N({n})"))
            )),
        }
    }
    for (r, used) in rows {
        if !used {
            problems.push(format!(
                "UNREAD lists `{}` in {}, which the reader no longer finds there: remove the row",
                r.2, r.1
            ));
        }
    }
    (problems, pairs)
}

/// Extender crate `name`'s direct calls (at least `at_least` of them, so a source reader that
/// stopped finding calls fails) each pass the words their callee's translation and typed form take.
fn check(name: &str, at_least: usize) {
    let all: Vec<(String, String)> = CRATES
        .iter()
        .flat_map(|c| files_of(&crate_src(c)))
        .collect();
    let types = Types::of(&all);
    let reading = sites(&crate_src(name), &types);
    let sites = &reading.sites;
    assert!(
        sites.len() >= at_least,
        "{name}: the source reader found only {} calls",
        sites.len()
    );
    let (mut wrong, pairs) = match_unread(name, &reading.unread);
    let _turn = PROBING.lock().unwrap_or_else(|e| e.into_inner());
    let registered = Engine::new_untyped_for_test();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut probed: BTreeMap<u32, (Option<usize>, Option<usize>)> = BTreeMap::new();
    let variadic = |addr: u32| VARIADIC.iter().find(|v| v.0 == addr);
    let addrs = sites
        .iter()
        .map(|s| s.addr)
        .chain(pairs.iter().filter_map(|(u, _)| u.addr));
    for addr in addrs {
        if variadic(addr).is_none() && registered.is_registered(addr) && !probed.contains_key(&addr)
        {
            let translated = arity(Engine::new_untyped_for_test, addr);
            let typed = arity(Engine::new, addr);
            probed.insert(addr, (translated, typed));
        }
    }
    std::panic::set_hook(previous);
    let mut open = Vec::new();
    let mut untranslated: BTreeMap<u32, Vec<&Site>> = BTreeMap::new();
    for s in sites {
        if let Some(&(_, least, zero_last)) = variadic(s.addr) {
            if s.words < least {
                wrong.push(format!(
                    "{}: {} ({:08x}) passes {} words, it takes at least {least}",
                    s.at, s.callee, s.addr, s.words
                ));
            }
            if zero_last && !s.zero_last {
                wrong.push(format!(
                    "{}: {} ({:08x}) passes a list that does not end with 0",
                    s.at, s.callee, s.addr
                ));
            }
            continue;
        }
        match probed.get(&s.addr) {
            None => {
                open.push(format!("{} {} ({:08x})", s.at, s.callee, s.addr));
                untranslated.entry(s.addr).or_default().push(s);
            }
            Some(&(translated, typed)) => {
                for (form, n) in [("translation", translated), ("typed form", typed)] {
                    if n != Some(s.words) {
                        wrong.push(format!(
                            "{}: {} ({:08x}) passes {} words, its {form} takes {n:?}",
                            s.at, s.callee, s.addr, s.words
                        ));
                    }
                }
            }
        }
    }
    // A callee nobody has translated: its calls agree with each other.
    for (addr, calls) in &untranslated {
        if calls.iter().any(|s| s.words != calls[0].words) {
            let counts: Vec<String> = calls
                .iter()
                .map(|s| format!("{} {}", s.at, s.words))
                .collect();
            wrong.push(format!(
                "{addr:08x} (not translated) is called with different word counts: {counts:?}"
            ));
        }
    }
    // A listed call to a constant callee whose words are built elsewhere: the callee takes the
    // listed words.
    for (u, words) in &pairs {
        let Some(addr) = u.addr else { continue };
        if let Some(&(_, least, _)) = variadic(addr) {
            if words.least() < least {
                wrong.push(format!(
                    "{}: UNREAD lists {words:?} for `{}` ({addr:08x}), it takes at least {least}",
                    u.at, u.callee
                ));
            }
            continue;
        }
        let Some(&(translated, typed)) = probed.get(&addr) else {
            open.push(format!("{} {} ({addr:08x})", u.at, u.callee));
            continue;
        };
        for (form, n) in [("translation", translated), ("typed form", typed)] {
            if !matches!(words, Words::N(k) if n == Some(*k)) {
                wrong.push(format!(
                    "{}: UNREAD lists {words:?} for `{}` ({addr:08x}), its {form} takes {n:?}",
                    u.at, u.callee
                ));
            }
        }
    }
    println!(
        "{name}: {} direct calls, {} callees probed, {} calls listed as checked by hand; {} of {} \
         items counted as one word without their type read; {} not translated: {open:#?}",
        sites.len(),
        probed.len(),
        pairs.len(),
        reading.untyped,
        reading.items,
        open.len()
    );
    assert!(wrong.is_empty(), "{name}: wrong word counts: {wrong:#?}");
}

#[test]
fn jip_passes_each_game_function_the_words_it_takes() {
    check("jip", 300);
}

/// xNVSE's lambdas called the game's two list copies with two words of three: a player's modded
/// game stopped at the first inline function a script gave `ar_ForEach`.
#[test]
fn nvse_passes_each_game_function_the_words_it_takes() {
    check("nvse", 200);
}

#[test]
fn uio_passes_each_game_function_the_words_it_takes() {
    check("uio", 30);
}

#[test]
fn mcm_passes_each_game_function_the_words_it_takes() {
    check("mcm", 4);
}

/// lStewieAl's Tweaks calls no game function by address yet (its rows run inside the
/// translations): the check runs, so a call added later is read.
#[test]
fn stewie_passes_each_game_function_the_words_it_takes() {
    check("stewie", 0);
}

/// The reader's own rules, on made-up source.
#[test]
fn the_reader_counts_words_by_type_and_lists_what_it_cannot_read() {
    let types = Types::of(&[(
        "a.rs".to_string(),
        code_of(
            "pub struct A { pub this_obj: u32, pub(crate) scale: f64 }\nconst HALF: f64 = 0.5;",
        ),
    )]);
    let w = |item: &str, scope: &str, params: &str| item_words(item, scope, params, &types, 0);
    assert_eq!(w("1.5", "", ""), Some(2), "a float literal is an f64");
    assert_eq!(w("1.5f32", "", ""), Some(1));
    assert_eq!(w("0xffu8", "", ""), Some(1));
    assert_eq!(w("2u64", "", ""), Some(2));
    assert_eq!(w("7", "", ""), Some(1));
    assert_eq!(w("x as f64", "", ""), Some(2));
    assert_eq!(w("f64::from(x)", "", ""), Some(2));
    assert_eq!(w("e.mem.f64(at)", "", ""), Some(2));
    assert_eq!(w("e.call(F, &[]).st0", "", ""), Some(2));
    assert_eq!(w("e.global::<u64>(X)", "", ""), Some(2));
    assert_eq!(w("HALF", "", ""), Some(2));
    assert_eq!(w("a.scale", "", ""), Some(2));
    assert_eq!(w("a.this_obj", "", ""), Some(1));
    assert_eq!(w("d", "let d = r.f64();\n", ""), Some(2), "a let's value");
    assert_eq!(w("d", "let d: f64 = 0.0;\nlet x = 1;", ""), Some(2));
    assert_eq!(w("d", "", "e: &mut Engine, d: f64"), Some(2), "a parameter");
    assert_eq!(w("d * 2.0", "", ""), Some(2));
    assert_eq!(
        w("d", "let d = 1.0;\nlet d = d as u32;", ""),
        Some(1),
        "the last let"
    );
    assert_eq!(w("refr", "let refr = somewhere();", ""), None);

    let code = String::from(
        "const F: u32 = 0x0040_0000;\nconst G: u32 = 0x0040_1000;\n\
         fn helper(e: &mut Engine, f: u32, x: u32) -> u32 { e.call(f, &args![x, 0u32]).u32() }\n\
         fn run(e: &mut Engine, w: &[u32], up: bool) {\n\
         helper(e, F, 1);\n\
         e.call(if up { F } else { G }, &args![1.5]);\n\
         e.call(F, w);\n\
         let f = e.mem.u32(8);\n\
         e.call(f, &[0; 3]);\n\
         e.call(G, \"a, b\", 3);\n\
         let g = if up { F } else { G };\n\
         e.call(g, &args![1u32]);\n\
         let ws = [1u32, 2, 3];\n\
         e.call(G, &ws);\n\
         let mut grown = vec![1u32];\n\
         grown.push(0);\n\
         e.call(G, &grown);\n\
         }\n\
         #[cfg(test)]\nfn only_in_tests(e: &mut Engine) { e.call(F, &[]); }\n",
    );
    let dir = std::env::temp_dir().join(format!("arity-reader-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.rs"), &code).unwrap();
    let r = sites(&dir, &types);
    std::fs::remove_dir_all(&dir).unwrap();
    let found: Vec<(&str, usize)> = r
        .sites
        .iter()
        .map(|s| (s.callee.as_str(), s.words))
        .collect();
    assert_eq!(
        found,
        [("F", 2), ("G", 2), ("F", 1), ("G", 1), ("G", 3), ("F", 2)],
        "{r:?}",
        r = r.sites
    );
    let unread: Vec<(&str, Option<usize>, Option<u32>)> = r
        .unread
        .iter()
        .map(|u| (u.callee.as_str(), u.words, u.addr))
        .collect();
    assert_eq!(
        unread,
        [
            ("F", None, Some(0x0040_0000)),
            ("f", Some(3), None),
            ("G", None, Some(0x0040_1000))
        ],
        "{:?}",
        r.unread
    );
    assert!(short_of_words("00703e80 needs 9 argument words, got 3", 3));
    assert!(!short_of_words("00703e80 needs 9 argument words, got 9", 9));
    assert!(!short_of_words("some other stop", 0));
}

/// A sweep's tool, not a check: `NV_PROBE=addr:words,addr:words,...` (hex addresses, the words a
/// caller passes, `this` included) probes each listed translation the same way and fails naming
/// those that take another count (a vtable slot's targets, read from the exe, for instance).
#[test]
#[ignore = "a sweep's tool: needs NV_PROBE"]
fn listed_translations_take_the_listed_words() {
    let list = std::env::var("NV_PROBE").expect("NV_PROBE=addr:words,...");
    let _turn = PROBING.lock().unwrap_or_else(|e| e.into_inner());
    let registered = Engine::new_untyped_for_test();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut report = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (addr, words) = item.split_once(':').expect("addr:words");
        let addr = u32::from_str_radix(addr.trim_start_matches("0x"), 16).expect("hex address");
        let words: usize = words.parse().expect("a word count");
        let found = if registered.is_registered(addr) {
            arity(Engine::new_untyped_for_test, addr)
        } else {
            None
        };
        if found != Some(words) {
            report.push(format!("{addr:08x}: passed {words}, takes {found:?}"));
        }
    }
    std::panic::set_hook(previous);
    assert!(report.is_empty(), "{report:#?}");
}
