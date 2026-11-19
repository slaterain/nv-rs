//! Every call a built-in extender (xNVSE `crates/nvse`, JIP LN `crates/jip`, UI Organizer
//! `crates/uio`, MCM `crates/mcm`, lStewieAl's Tweaks `crates/stewie`) makes to one of the game's
//! functions by a constant address passes as many argument words as that function's translation
//! takes: a call with too few stops the game ("call has N argument words, needs more"), one with
//! too many hands a later argument to nobody. Here because this crate registers every translation.
//! (A player's modded game stopped that way in xNVSE's lambdas, whose two list copies passed two
//! words of three, when this test read the jip crate only.)
//!
//! The test reads each crate's source (not its tests): each `.call(NAME, &args![..])` or
//! `.call(NAME, &[..])` whose callee is a `const NAME: u32` of the crate or an address written in
//! place (`0x005a_b930`), the words counted from the list (one per item; two for an item written as
//! an `f64`, `u64` or `i64` literal). Each callee's word count is probed on a fresh engine: the
//! translation is called with 0, 1, 2, ... zero words until its argument reads stop asking for more
//! (`entry!` reads every argument before the body runs; a nested call stops the probe before it
//! reads its own). A callee nobody has translated is listed, not failed (a call to it stops the
//! game whatever it passes).
//!
//! Not covered (the sweep of 2026-11-15 checked jip's by hand): calls through a vtable slot
//! (`vcall`: the target depends on the object's class), calls to an address read at run time (a
//! hook's original, a menu's handler, a callee passed as a parameter), and word lists built
//! elsewhere (`w`, `&words`). For those, the ignored test below probes listed translations
//! (`NV_PROBE`), and `NV_JIP_SRC` (`NV_NVSE_SRC`, `NV_UIO_SRC`, ...) points a crate's test at
//! another copy of its source (an open pull request's branch, exported).

use engine as _;
use engine_core::prelude::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// One direct call in a crate's source.
#[derive(Debug)]
struct Site {
    at: String,
    callee: String,
    addr: u32,
    words: usize,
}

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
        for entry in std::fs::read_dir(dir).expect("jip source folder") {
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

/// The text with comments blanked (same length, so offsets keep their lines) and an inline test
/// module cut off.
fn code_of(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        if in_str {
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            if b[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if b[i] == b'"' {
            in_str = true;
        } else if b[i] == b'\'' && i + 2 < b.len() && (b[i + 2] == b'\'' || b[i + 1] == b'\\') {
            // A char literal ('"', '\''): skip it whole.
            let end = b[i + 1..].iter().skip(1).position(|&c| c == b'\'');
            if let Some(end) = end {
                i += end + 3;
                continue;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    let mut code = String::from_utf8(out).expect("utf-8 source");
    if let Some(at) = code.find("\n#[cfg(test)]\nmod ") {
        code.truncate(at);
    }
    code
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
            if !ident
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                || !tail.starts_with("u32")
            {
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

/// Every direct call by a constant address in the crate whose source is at `root`, and the calls
/// the test could not read (where, why).
fn sites(root: &Path) -> (Vec<Site>, Vec<String>) {
    let files: Vec<(String, String)> = sources(root)
        .into_iter()
        .map(|p| {
            let name = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (name, code_of(&std::fs::read_to_string(&p).expect("source")))
        })
        .collect();
    let consts = consts(&files);
    let mut out = Vec::new();
    let mut skipped = Vec::new();
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
            let Some(args) = call_args(code, open) else {
                skipped.push(format!("{where_}: unbalanced"));
                continue;
            };
            let parts = items(args);
            if parts.len() != 2 {
                skipped.push(format!("{where_}: {} arguments", parts.len()));
                continue;
            }
            let callee = parts[0].rsplit("::").next().unwrap().trim().to_string();
            let written = literal_address(&callee);
            if written.is_none()
                && (callee.is_empty()
                    || !callee
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
            {
                skipped.push(format!("{where_}: callee `{}` read at run time", parts[0]));
                continue;
            }
            let addr = match (written, consts.get(&callee).map(Vec::as_slice)) {
                (Some(at), _) => at,
                (None, Some(values)) => match values.iter().find(|(f, _)| f == name) {
                    Some(&(_, v)) => v,
                    None if values.iter().all(|(_, v)| *v == values[0].1) => values[0].1,
                    None => {
                        skipped.push(format!("{where_}: `{callee}` has several values"));
                        continue;
                    }
                },
                (None, None) => {
                    skipped.push(format!("{where_}: `{callee}` not a constant of the crate"));
                    continue;
                }
            };
            let list = parts[1].trim();
            let inner = list
                .strip_prefix("&args!")
                .or_else(|| list.strip_prefix('&'))
                .map(str::trim)
                .and_then(|l| l.strip_prefix('['))
                .and_then(|l| l.strip_suffix(']'));
            let Some(inner) = inner else {
                skipped.push(format!(
                    "{where_}: `{callee}` words built elsewhere (`{list}`)"
                ));
                continue;
            };
            let words = items(inner)
                .iter()
                .map(|item| {
                    if ["f64", "u64", "i64"].iter().any(|s| item.ends_with(s)) {
                        2
                    } else {
                        1
                    }
                })
                .sum();
            out.push(Site {
                at: where_,
                callee,
                addr,
                words,
            });
        }
    }
    (out, skipped)
}

/// The callees with a variable argument list (`cdecl`, `...`), and the words they always take: a call
/// passes at least those (the rest follow the format or the entry point's tables; the sweep checked
/// them by hand).
const VARIADIC: [(u32, usize); 3] = [
    // `BGSEntryPoint::HandleEntryPoint`: the entry point and the actor, then the filters and the
    // function's arguments.
    (0x005e_58f0, 2),
    // The console's formatted print: the format, then its words.
    (0x0070_3c00, 1),
    // `ShowMessageBox`: the text, callback and options (nine words, the first button among them),
    // then the other buttons and the closing 0 (its entry asserts the nine: xNVSE's
    // `DisplayMessage` passes ten, `MessageBoxEx` nineteen).
    (0x0070_3e80, 9),
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
        let short = msg == format!("call has {k} argument words, needs more")
            || msg.starts_with(&format!("index out of bounds: the len is {k} but"));
        if !short || BODY_REACHED.with(Cell::get) {
            return Some(k);
        }
    }
    None
}

/// One crate's check at a time: the probe silences the process's panic hook while it runs, and
/// two crates' checks swapping it at once could leave it silent for a failing test's message.
static PROBING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Extender crate `name`'s direct calls (at least `at_least` of them, so a source reader that
/// stopped finding calls fails) each pass the words their callee's translation and typed form take.
fn check(name: &str, at_least: usize) {
    let (sites, skipped) = sites(&crate_src(name));
    assert!(
        sites.len() >= at_least,
        "{name}: the source reader found only {} calls",
        sites.len()
    );
    let _turn = PROBING.lock().unwrap_or_else(|e| e.into_inner());
    let registered = Engine::new_untyped_for_test();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut probed: BTreeMap<u32, (Option<usize>, Option<usize>)> = BTreeMap::new();
    let mut wrong = Vec::new();
    for s in &sites {
        if let Some(&(_, least)) = VARIADIC.iter().find(|(a, _)| *a == s.addr) {
            if s.words < least {
                wrong.push(format!(
                    "{}: {} ({:08x}) passes {} words, it takes at least {least}",
                    s.at, s.callee, s.addr, s.words
                ));
            }
            continue;
        }
        if registered.is_registered(s.addr) && !probed.contains_key(&s.addr) {
            let translated = arity(Engine::new_untyped_for_test, s.addr);
            let typed = arity(Engine::new, s.addr);
            probed.insert(s.addr, (translated, typed));
        }
    }
    std::panic::set_hook(previous);
    let mut open = Vec::new();
    for s in sites
        .iter()
        .filter(|s| VARIADIC.iter().all(|(a, _)| *a != s.addr))
    {
        match probed.get(&s.addr) {
            None => open.push(format!("{} {} ({:08x})", s.at, s.callee, s.addr)),
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
    println!(
        "{name}: {} direct calls, {} callees probed; {} not translated: {open:#?}; not read: {skipped:#?}",
        sites.len(),
        probed.len(),
        open.len()
    );
    assert!(wrong.is_empty(), "{name}: wrong word counts: {wrong:#?}");
}

#[test]
fn jip_passes_each_game_function_the_words_it_takes() {
    check("jip", 50);
}

/// xNVSE's lambdas called the game's two list copies with two words of three: a player's modded
/// game stopped at the first inline function a script gave `ar_ForEach`.
#[test]
fn nvse_passes_each_game_function_the_words_it_takes() {
    check("nvse", 50);
}

#[test]
fn uio_passes_each_game_function_the_words_it_takes() {
    check("uio", 1);
}

#[test]
fn mcm_passes_each_game_function_the_words_it_takes() {
    check("mcm", 1);
}

/// lStewieAl's Tweaks calls no game function by address yet (its rows run inside the
/// translations): the check runs, so a call added later is read.
#[test]
fn stewie_passes_each_game_function_the_words_it_takes() {
    check("stewie", 0);
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
