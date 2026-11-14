//! Tests of the run-time compile entry, the function files and `CompileScript`, from xNVSE 6.4.9's
//! source (`GameScript.cpp`, `CachedScripts.cpp`, `Commands_Script.cpp`): the game's compiler,
//! constructors and file calls are stand-ins that record what they were given, on the command
//! table's engine; a folder of files stands in for the game's working folder.

use super::*;
use crate::commands::test_support::{engine_with_table, DATA, LIST, OFFSET, RESULT, SCRIPT};
use std::cell::RefCell;
use std::collections::BTreeMap;

const AREA: u32 = 0x0600_0000;
const HEAP_TOP: u32 = AREA;
const HEAP: u32 = AREA + 0x1_0000;
const CONSOLE: u32 = AREA + 0x100;
const COMPILER: u32 = AREA + 0x200;
const VTABLE: u32 = AREA + 0x400;
const HANDLER: u32 = AREA + 0x800;
const NEXT_ID: u32 = 0xff00_0801;
const SET_REF_ID_FN: u32 = 0x7eff_c000;
const SET_EDITOR_ID_FN: u32 = 0x7eff_c004;
const TOUPPER: u32 = 0x00ec_a7f4;
const CONSOLE_PRINT: u32 = 0x0071_d0a0;
const SET_FILE_POINTER: u32 = 0x00fd_f240;
const WRITE_FILE: u32 = 0x00fd_f0dc;

/// What one compile saw: the buffer's name, the text, the partial flag, the data's first byte,
/// the data offset, the run-time mode, the current script, the script's text pointer (the same
/// as the buffer's), and whether form ids were off.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    name: String,
    text: String,
    partial: u8,
    first: u8,
    offset: u32,
    mode: u32,
    current_is_script: bool,
    same_text: bool,
}

#[derive(Default)]
struct Log {
    compiles: Vec<Seen>,
    /// Every call of interest, in order: `ctor-buffer`, `ctor-script`, `assign <n>`,
    /// `dtor-buffer`, `dtor-script`, `free <addr>`, `ref-id <id> <flag>`, `editor-id <name>`.
    calls: Vec<String>,
    console: Vec<String>,
    /// The fake folder: files by path (lower case, `\`), and the names a search answers in each
    /// folder, in order, with whether each is a folder.
    files: BTreeMap<String, Vec<u8>>,
    listings: BTreeMap<String, Vec<(String, bool)>>,
    /// Open handles: their path and read position; open searches: what is left to answer.
    handles: BTreeMap<u32, (String, usize)>,
    searches: BTreeMap<u32, Vec<(String, bool)>>,
    next_handle: u32,
    /// Bytes written to files (`nvse.log`).
    written: BTreeMap<String, Vec<u8>>,
}

thread_local! {
    static LOG: RefCell<Log> = RefCell::new(Log::default());
}

fn with_log<T>(f: impl FnOnce(&mut Log) -> T) -> T {
    LOG.with(|l| f(&mut l.borrow_mut()))
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn note(s: String) {
    with_log(|l| l.calls.push(s));
}

fn key(path: &[u8]) -> String {
    String::from_utf8_lossy(path)
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn heap(e: &mut Engine, a: &[u32]) -> Ret {
    let at = e.mem.u32(HEAP_TOP).max(HEAP);
    e.mem
        .set_u32(HEAP_TOP, at + a[0].max(1).next_multiple_of(16));
    e.mem.write(at, &vec![0; a[0] as usize]);
    eax(at)
}

/// The stand-in compiler: records the buffer and answers false for a text containing `bad`.
fn compile_stand_in(e: &mut Engine, a: &[u32]) -> Ret {
    assert_eq!(a[0], COMPILER);
    let (script, buf) = (a[1], a[2]);
    let text_at = e.mem.u32(buf + sb::TEXT);
    let text = crate::table::guest_text(e, text_at);
    let name = crate::compiler::string_cstr(e, buf + sb::NAME);
    let data = e.mem.u32(buf + sb::DATA);
    let seen = Seen {
        name: String::from_utf8_lossy(&name).into_owned(),
        text: String::from_utf8_lossy(&text).into_owned(),
        partial: e.mem.u8(buf + sb::PARTIAL),
        first: e.mem.u8(data),
        offset: e.mem.u32(buf + sb::DATA_OFFSET),
        mode: e.mem.u32(buf + 8),
        current_is_script: e.mem.u32(buf + sb::CURRENT_SCRIPT) == script,
        same_text: e.mem.u32(script + sc::TEXT) == text_at,
    };
    with_log(|l| l.compiles.push(seen));
    eax(u32::from(!text.windows(3).any(|w| w == b"bad")))
}

/// The engine: the command table, the stand-ins and a console whose compiler is [`COMPILER`].
fn engine() -> Engine {
    LOG.with(|l| *l.borrow_mut() = Log::default());
    let mut e = engine_with_table();
    e.map(AREA, 0x10_0000);
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(DATA_HANDLER, HANDLER);
    e.mem.set_u32(CONSOLE, COMPILER);
    e.mem.set_u32(VTABLE + SET_REF_ID, SET_REF_ID_FN);
    e.mem.set_u32(VTABLE + SET_EDITOR_ID, SET_EDITOR_ID_FN);
    e.ext::<Nvse>().main_thread_recorded = true;
    e.register(FORM_HEAP_ALLOCATE, heap);
    e.register(FORM_HEAP_FREE, |_, a| {
        note(format!("free {:x}", a[0]));
        Ret::default()
    });
    e.register(BUFFER_CONSTRUCTOR, |e, a| {
        let data = heap(e, &[0x4000]).eax;
        e.mem.set_u32(a[0] + sb::DATA, data);
        note("ctor-buffer".into());
        eax(a[0])
    });
    e.register(BUFFER_DESTRUCTOR, |_, a| {
        note(format!("dtor-buffer {:x}", a[0]));
        Ret::default()
    });
    e.register(SCRIPT_CONSTRUCTOR, |e, a| {
        e.mem.set_u32(a[0], VTABLE);
        note("ctor-script".into());
        eax(a[0])
    });
    e.register(SCRIPT_DESTRUCTOR, |_, a| {
        note(format!("dtor-script {:x}", a[0]));
        Ret::default()
    });
    e.register(DISABLE_ASSIGN_FORM_IDS, |_, a| {
        assert_eq!(a[0], HANDLER);
        note(format!("assign-off {}", a[1]));
        Ret::default()
    });
    e.register(NEXT_FREE_FORM_ID, |_, a| {
        assert_eq!(a[0], HANDLER);
        eax(NEXT_ID)
    });
    e.register(SET_REF_ID_FN, |e, a| {
        e.mem.set_u32(a[0] + 0xc, a[1]);
        note(format!("ref-id {:x} {}", a[1], a[2]));
        Ret::default()
    });
    e.register(SET_EDITOR_ID_FN, |e, a| {
        let name = crate::table::guest_text(e, a[1]);
        note(format!("editor-id {}", String::from_utf8_lossy(&name)));
        eax(1)
    });
    e.register(CONSOLE_MANAGER, |_, _| eax(CONSOLE));
    e.register(SCRIPT_COMPILE, compile_stand_in);
    e.register(TOUPPER, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(CONSOLE_PRINT, |e, a| {
        let format = crate::table::guest_text(e, a[1]);
        let text = if format == b"%s" {
            crate::table::guest_text(e, e.mem.u32(a[2]))
        } else {
            format
        };
        with_log(|l| l.console.push(String::from_utf8_lossy(&text).into_owned()));
        Ret::default()
    });
    file_calls(&mut e);
    e
}

/// The file calls over the fake folder.
fn file_calls(e: &mut Engine) {
    e.register(CREATE_FILE_A, |e, a| {
        let path = key(&crate::table::guest_text(e, a[0]));
        let disposition = a[4];
        with_log(|l| {
            l.next_handle += 1;
            let h = 0x100 + l.next_handle;
            if disposition == OPEN_EXISTING {
                if !l.files.contains_key(&path) {
                    return eax(INVALID_HANDLE_VALUE);
                }
            } else if disposition == 2 || !l.written.contains_key(&path) {
                l.written.insert(path.clone(), Vec::new());
            }
            l.handles.insert(h, (path, 0));
            eax(h)
        })
    });
    e.register(READ_FILE, |e, a| {
        let (h, buf, count, read) = (a[0], a[1], a[2], a[3]);
        let chunk = with_log(|l| {
            let (path, pos) = l.handles.get(&h).cloned().unwrap();
            let bytes = l.files.get(&path).cloned().unwrap_or_default();
            let end = (pos + count as usize).min(bytes.len());
            l.handles.insert(h, (path, end));
            bytes[pos..end].to_vec()
        });
        e.mem.write(buf, &chunk);
        e.mem.set_u32(read, chunk.len() as u32);
        eax(1)
    });
    e.register(WRITE_FILE, |e, a| {
        let bytes = e.mem.bytes(a[1], a[2]);
        with_log(|l| {
            let path = l.handles[&a[0]].0.clone();
            l.written.entry(path).or_default().extend(bytes);
        });
        eax(1)
    });
    e.register(SET_FILE_POINTER, |_, _| eax(0));
    e.register(CLOSE_HANDLE, |_, a| {
        with_log(|l| l.handles.remove(&a[0]));
        eax(1)
    });
    e.register(FIND_FIRST_FILE_A, |e, a| {
        let pattern = key(&crate::table::guest_text(e, a[0]));
        let dir = pattern.strip_suffix('*').expect("a folder search").to_string();
        let Some(mut entries) = with_log(|l| l.listings.get(&dir).cloned()) else {
            return eax(INVALID_HANDLE_VALUE);
        };
        entries.insert(0, ("..".into(), true));
        entries.insert(0, (".".into(), true));
        let first = entries.remove(0);
        write_found(e, a[1], &first);
        with_log(|l| {
            l.next_handle += 1;
            let h = 0x900 + l.next_handle;
            l.searches.insert(h, entries);
            eax(h)
        })
    });
    e.register(FIND_NEXT_FILE_A, |e, a| {
        let next = with_log(|l| {
            let rest = l.searches.get_mut(&a[0]).unwrap();
            (!rest.is_empty()).then(|| rest.remove(0))
        });
        match next {
            Some(found) => {
                write_found(e, a[1], &found);
                eax(1)
            }
            None => eax(0),
        }
    });
    e.register(FIND_CLOSE, |_, a| {
        with_log(|l| l.searches.remove(&a[0]));
        eax(1)
    });
}

fn write_found(e: &mut Engine, data: u32, (name, folder): &(String, bool)) {
    e.mem.set_u32(
        data,
        if *folder {
            FILE_ATTRIBUTE_DIRECTORY
        } else {
            0x20
        },
    );
    e.mem.write(data + FIND_DATA_NAME, name.as_bytes());
    e.mem
        .set_u8(data + FIND_DATA_NAME + name.len() as u32, 0);
}

/// Puts a file (or, with `None`, a folder) at `path` below the game's folder (no separator at
/// its end), listed in its folder after the ones put there before when that folder was put.
fn put(path: &str, bytes: Option<&[u8]>) {
    let k = key(path.as_bytes());
    let at = k.rfind('\\').map_or(0, |at| at + 1);
    let (dir, name) = (k[..at].to_string(), path[at..].to_string());
    with_log(|l| {
        if let Some(listing) = l.listings.get_mut(&dir) {
            listing.push((name, bytes.is_none()));
        }
        match bytes {
            Some(b) => {
                l.files.insert(k, b.to_vec());
            }
            None => {
                l.listings.entry(format!("{k}\\")).or_default();
            }
        }
    });
}

/// The function folder, `data/nvse/user_defined_functions`.
const BASE: &str = "data/nvse/user_defined_functions";

fn calls() -> Vec<String> {
    with_log(|l| std::mem::take(&mut l.calls))
}
fn compiles() -> Vec<Seen> {
    with_log(|l| std::mem::take(&mut l.compiles))
}
fn console() -> Vec<String> {
    with_log(|l| std::mem::take(&mut l.console))
}
fn written(path: &str) -> String {
    with_log(|l| String::from_utf8_lossy(l.written.get(path).map_or(&[][..], |v| v)).into_owned())
}

/// `CompileScriptEx` sets the buffer up as xNVSE does (the name, the text in both the buffer and
/// the script, a partial script, the fake `scn`, offset 4, the console's mode, the script as the
/// current one), with form ids off only while the script is made; a compiled script with
/// `assignFormID` gets the next free id, and the buffer goes after; the text pointers are cleared
/// after the compile.
#[test]
fn compile_script_ex_sets_up_the_buffer() {
    let mut e = engine();
    let script = compile_script_ex(&mut e, b"Begin Function {}\nEnd\0ignored", Some(b"MyUdf"), true);
    assert_ne!(script, 0);
    assert_eq!(
        compiles(),
        vec![Seen {
            name: "MyUdf".into(),
            text: "Begin Function {}\nEnd".into(),
            partial: 1,
            first: 0x1d,
            offset: 4,
            mode: 1,
            current_is_script: true,
            same_text: true,
        }]
    );
    let c = calls();
    let buffer_dtor = c.iter().position(|s| s.starts_with("dtor-buffer")).unwrap();
    assert_eq!(
        &c[..4],
        &[
            "ctor-buffer".to_string(),
            "assign-off 1".into(),
            "ctor-script".into(),
            "assign-off 0".into()
        ]
    );
    let ref_id = c.iter().position(|s| s == &format!("ref-id {NEXT_ID:x} 1")).unwrap();
    assert!(ref_id < buffer_dtor, "{c:?}");
    assert!(!c.iter().any(|s| s.starts_with("dtor-script")), "{c:?}");
    assert_eq!(e.mem.u32(script + 0xc), NEXT_ID);
    assert_eq!(e.mem.u32(script + sc::TEXT), 0);
}

/// Without a name the buffer is `nvse_partial_script_<n>`, counted from 1 over the run; without
/// `assignFormID` no id is given.
#[test]
fn unnamed_scripts_are_counted() {
    let mut e = engine();
    assert_ne!(compile_script(&mut e, b"x"), 0);
    assert_ne!(compile_script(&mut e, b"y"), 0);
    let names: Vec<String> = compiles().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["nvse_partial_script_1", "nvse_partial_script_2"]);
    assert!(!calls().iter().any(|s| s.starts_with("ref-id")));
}

/// A script that does not compile is destroyed (the script, then the buffer) and 0 is answered.
#[test]
fn a_failed_compile_destroys_the_script_first() {
    let mut e = engine();
    assert_eq!(compile_script_ex(&mut e, b"bad", None, true), 0);
    let c = calls();
    let script = c.iter().position(|s| s.starts_with("dtor-script")).unwrap();
    let buffer = c.iter().position(|s| s.starts_with("dtor-buffer")).unwrap();
    assert!(script < buffer, "{c:?}");
    assert!(!c.iter().any(|s| s.starts_with("ref-id")));
}

/// With no console (xNVSE's fix #281) nothing compiles.
#[test]
fn no_console_no_compile() {
    let mut e = engine();
    e.register(CONSOLE_MANAGER, |_, _| eax(0));
    assert_eq!(compile_script(&mut e, b"x"), 0);
    assert!(compiles().is_empty());
    e.register(CONSOLE_MANAGER, |_, _| eax(CONSOLE));
    e.mem.set_u32(CONSOLE, 0);
    assert_eq!(compile_script(&mut e, b"x"), 0);
    assert!(compiles().is_empty());
}

/// `CompileExpression`: the text inside `SetFunctionValue (...)`; a text naming
/// `SetFunctionValue` (without case) is the body, `%r` and `%R` made line breaks.
#[test]
fn compile_expression_builds_a_function() {
    let mut e = engine();
    compile_expression(&mut e, b"1 + 2");
    compile_expression(&mut e, b"let x := 1%rsetfunctionvalue x%R");
    compile_expression(&mut e, b"SetFunctionValu 3");
    let texts: Vec<String> = compiles().into_iter().map(|s| s.text).collect();
    assert_eq!(
        texts,
        [
            "begin function{}\nSetFunctionValue (1 + 2)\nend\n",
            "begin function{}\nlet x := 1\nsetfunctionvalue x\n\nend\n",
            "begin function{}\nSetFunctionValue (SetFunctionValu 3)\nend\n",
        ]
    );
}

/// `std::ifstream` in text mode: `\r\n` to `\n`, a lone `\r` kept, Ctrl+Z ends the text.
#[test]
fn files_read_in_text_mode() {
    assert_eq!(text_mode(b"a\r\nb\rc\n"), b"a\nb\rc\n");
    assert_eq!(text_mode(b"a\r\n\r\n"), b"a\n\n");
    assert_eq!(text_mode(b"ab\x1acd"), b"ab");
    assert_eq!(text_mode(b"x\r"), b"x\r");
    assert_eq!(text_mode(b""), b"");
}

/// `path::stem` and `path::extension`.
#[test]
fn stems_and_extensions() {
    let cases: [(&[u8], &[u8], &[u8]); 8] = [
        (b"a.txt", b"a", b".txt"),
        (b"a.b.gek", b"a.b", b".gek"),
        (b".txt", b".txt", b""),
        (b"a.", b"a", b"."),
        (b"a", b"a", b""),
        (b".", b".", b""),
        (b"..", b"..", b""),
        (b"..x", b".", b".x"),
    ];
    for (name, stem, extension) in cases {
        assert_eq!(stem_and_extension(name), (stem, extension), "{name:?}");
    }
    assert_eq!(file_name(b"data/nvse/user_defined_functions/sub\\x.txt"), b"x.txt");
    assert_eq!(file_name(b"x"), b"x");
}

/// The first frame's compile: every file with an extension below the folder, sub-folders right
/// after their entry, each named `nvseRuntimeScript<count><stem>` (the count is the cache's
/// size: a failed file adds nothing), cached by its path below the folder (a sub-folder's after
/// a `\`), its editor id set; a failed file named on the console and in `nvse.log`, a compiled
/// one in `nvse.log`.
#[test]
fn the_function_files_compile_at_the_first_frame() {
    let mut e = engine();
    put(BASE, None);
    put(&format!("{BASE}/First.txt"), Some(b"Begin Function {}\r\nEnd\r\n"));
    put(&format!("{BASE}/noext"), Some(b"Begin Function {}\nEnd\n"));
    put(&format!("{BASE}/Sub"), None);
    put(&format!("{BASE}/Sub/inner.gek"), Some(b"inner"));
    put(&format!("{BASE}/bad.txt"), Some(b"bad"));
    put(&format!("{BASE}/last.txt"), Some(b"last"));
    cache_all_scripts(&mut e);
    let seen: Vec<(String, String)> = compiles().into_iter().map(|s| (s.name, s.text)).collect();
    assert_eq!(
        seen,
        [
            ("nvseRuntimeScript0First".to_string(), "Begin Function {}\nEnd\n".to_string()),
            ("nvseRuntimeScript1inner".into(), "inner".into()),
            ("nvseRuntimeScript2bad".into(), "bad".into()),
            ("nvseRuntimeScript2last".into(), "last".into()),
        ]
    );
    let editor_ids: Vec<String> = calls()
        .into_iter()
        .filter(|c| c.starts_with("editor-id"))
        .collect();
    assert_eq!(
        editor_ids,
        [
            "editor-id nvseRuntimeScript0First",
            "editor-id nvseRuntimeScript1inner",
            "editor-id nvseRuntimeScript2last"
        ]
    );
    let mut keys: Vec<Vec<u8>> = e
        .ext::<Nvse>()
        .compile_script
        .cached
        .values()
        .map(|(k, _)| k.clone())
        .collect();
    keys.sort();
    assert_eq!(keys, [b"First.txt".to_vec(), b"Sub\\inner.gek".to_vec(), b"last.txt".to_vec()]);
    assert_ne!(cached(&mut e, b"first.TXT"), 0, "found without case");
    assert_ne!(cached(&mut e, b"sub\\INNER.gek"), 0);
    assert_eq!(cached(&mut e, b"Sub/inner.gek"), 0, "the command makes '/' a '\\' first");
    assert_eq!(
        console(),
        ["xNVSE: Failed to precompile script file at data/nvse/user_defined_functions/bad.txt"]
    );
    assert_eq!(
        written("nvse.log"),
        "xNVSE: script file successfully precompiled at data/nvse/user_defined_functions/First.txt\r\n\
         xNVSE: script file successfully precompiled at data/nvse/user_defined_functions/Sub\\inner.gek\r\n\
         xNVSE: Failed to precompile script file at data/nvse/user_defined_functions/bad.txt\r\n\
         xNVSE: script file successfully precompiled at data/nvse/user_defined_functions/last.txt\r\n"
    );
}

/// No folder: nothing compiles, nothing is printed.
#[test]
fn no_folder_no_files() {
    let mut e = engine();
    cache_all_scripts(&mut e);
    assert!(compiles().is_empty());
    assert!(console().is_empty());
    assert!(e.ext::<Nvse>().compile_script.cached.is_empty());
}

/// The cache is xNVSE's map: a path of a hash already there keeps its first key text and takes
/// the new script.
#[test]
fn the_cache_keeps_the_first_key() {
    let mut e = engine();
    cache(&mut e, b"A.txt", 1);
    cache(&mut e, b"a.TXT", 2);
    let s = &e.ext::<Nvse>().compile_script.cached;
    assert_eq!(s.len(), 1);
    assert_eq!(s.values().next().unwrap(), &(b"A.txt".to_vec(), 2));
}

fn string_token(t: &str) -> Vec<u8> {
    let mut b = vec![b'S'];
    b.extend_from_slice(&(t.len() as u16).to_le_bytes());
    b.extend_from_slice(t.as_bytes());
    b
}

fn number_token(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// `CompileScript`'s call with one-token expressions: its result's low word.
fn run(e: &mut Engine, tokens: &[Vec<u8>]) -> u32 {
    let mut bytes = vec![tokens.len() as u8];
    for t in tokens {
        bytes.extend_from_slice(&(t.len() as u16 + 2).to_le_bytes());
        bytes.extend_from_slice(t);
    }
    e.mem.set_u16(DATA, OP_COMPILE_SCRIPT as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    e.mem.write(DATA + 4, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1.5);
    let record = crate::table::record_of(e, OP_COMPILE_SCRIPT);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    let ok = e
        .call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET])
        .eax;
    assert_eq!(ok & 0xff, 1);
    assert_eq!(e.mem.u32(RESULT + 4), 0, "the result's high word is cleared first");
    e.mem.u32(RESULT)
}

/// `CompileScript path [recompile]`: the cached script's id (the path's `/` read as `\`,
/// without case); a file not cached is compiled and cached now; `recompile` compiles again and
/// replaces it; an empty path, a missing file or one that does not compile gives 0.
#[test]
fn compile_script_finds_or_compiles_the_file() {
    let mut e = engine();
    let base = format!("{BASE}/");
    put(BASE, None);
    put(&format!("{base}Sub"), None);
    put(&format!("{base}Sub/f.txt"), Some(b"one"));
    cache_all_scripts(&mut e);
    compiles();
    let first = run(&mut e, &[string_token("sub/F.TXT")]);
    assert_eq!(first, NEXT_ID);
    assert!(compiles().is_empty(), "the cached script");
    // A file made after the first frame: compiled now, under the path as given.
    put(&format!("{base}late.txt"), Some(b"late"));
    assert_eq!(run(&mut e, &[string_token("late.txt")]), NEXT_ID);
    let seen = compiles();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].name, "nvseRuntimeScript1late");
    assert_ne!(cached(&mut e, b"late.txt"), 0);
    // Recompiled: compiled again even though cached.
    assert_eq!(
        run(&mut e, &[string_token("late.txt"), number_token(1.0)]),
        NEXT_ID
    );
    assert_eq!(compiles()[0].name, "nvseRuntimeScript2late");
    assert_eq!(e.ext::<Nvse>().compile_script.cached.len(), 2);
    // Recompile false: the cache.
    assert_eq!(
        run(&mut e, &[string_token("late.txt"), number_token(0.0)]),
        NEXT_ID
    );
    assert!(compiles().is_empty());
    // Missing, no extension, empty, failing.
    assert_eq!(run(&mut e, &[string_token("missing.txt")]), 0);
    put(&format!("{base}noext"), Some(b"x"));
    assert_eq!(run(&mut e, &[string_token("noext")]), 0);
    assert_eq!(run(&mut e, &[string_token("")]), 0);
    put(&format!("{base}bad.txt"), Some(b"bad"));
    assert_eq!(run(&mut e, &[string_token("bad.txt")]), 0);
    assert_eq!(cached(&mut e, b"bad.txt"), 0);
    assert!(console().is_empty(), "the command prints nothing");
}

/// `RunScriptLine2`: a temporary script on the stack, marked temporary, given the text, run on the
/// console's compiler with the object, destroyed; its answer. With no console it does not run.
#[test]
fn run_script_line_runs_a_temporary_script() {
    let mut e = engine();
    e.register(MARK_AS_TEMPORARY, |_, _| {
        note("temporary".into());
        Ret::default()
    });
    e.register(SCRIPT_SET_TEXT, |e, a| {
        let t = crate::table::guest_text(e, a[1]);
        note(format!("text {}", String::from_utf8_lossy(&t)));
        Ret::default()
    });
    e.register(SCRIPT_RUN, |_, a| {
        note(format!("run {:x} {} {:x}", a[1], a[2], a[3]));
        eax(1)
    });
    assert!(run_script_line(&mut e, b"player.additem f 1\0x", 0x14));
    let c = calls();
    assert_eq!(
        c,
        [
            "ctor-script".to_string(),
            "temporary".into(),
            "text player.additem f 1".into(),
            format!("run {COMPILER:x} 1 14"),
            c[4].clone(),
        ]
    );
    assert!(c[4].starts_with("dtor-script"));
    e.register(CONSOLE_MANAGER, |_, _| eax(0));
    assert!(!run_script_line2(&mut e, b"x", 0, true));
    assert!(calls().is_empty());
}
