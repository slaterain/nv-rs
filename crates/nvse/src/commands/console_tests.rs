//! Tests of `Commands_Console.cpp`, the print, string and type commands of
//! `Commands_Scripting.cpp`, `IsFormValid` and `IsReference` (`Commands_Script.cpp`) and
//! `Update3D` (`Commands_MiscRef.cpp`), through the commands' records. The answers pinned are
//! xNVSE's as its source gives them; none of these commands has been run in the real game yet.

use super::super::rtti;
use super::super::test_support::*;
use crate::table::guest_text;
use crate::vars::{self, Key, Value, KEY_NUMBER, KEY_STRING};
use engine_core::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// The console (`ConsoleManager`, its output file's name at `+0x810`) and the echo byte.
const CONSOLE: u32 = 0x0200_f000;
const ECHO: u32 = 0x011f_158c;
/// The game's `toupper`, `LookupFormByID`, the file calls (`CreateFileA`, `SetFilePointer`,
/// `WriteFile`, `CloseHandle`), the model loader and its `QueueReference`.
const GAME_TOUPPER: u32 = 0x00ec_a7f4;
const LOOKUP_FORM: u32 = 0x0048_39c0;
const CREATE_FILE_A: u32 = 0x00fd_f088;
const SET_FILE_POINTER: u32 = 0x00fd_f240;
const WRITE_FILE: u32 = 0x00fd_f0dc;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;
const MODEL_LOADER: u32 = 0x011c_3b3c;
const QUEUE_REFERENCE: u32 = 0x0044_4850;

thread_local! {
    static LINES: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static FILES: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
    static HANDLES: RefCell<HashMap<u32, Vec<u8>>> = RefCell::new(HashMap::new());
    static CALLS: RefCell<Vec<Vec<u32>>> = const { RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The console's lines since the last call, each as the game's `Print` formats it.
fn lines() -> Vec<String> {
    LINES.with(|l| {
        std::mem::take(&mut *l.borrow_mut())
            .into_iter()
            .map(|b| String::from_utf8(b).unwrap())
            .collect()
    })
}

fn setup() -> Engine {
    LINES.with(|l| l.borrow_mut().clear());
    FILES.with(|f| f.borrow_mut().clear());
    CALLS.with(|c| c.borrow_mut().clear());
    let mut e = engine_with_table();
    e.map(ECHO & !0xfff, 0x1000);
    e.map(MODEL_LOADER & !0xfff, 0x1000);
    e.register(GAME_TOUPPER, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(LOOKUP_FORM, |_, a| {
        eax(if (0x0200_0000..0x0200_f000).contains(&a[0]) {
            a[0]
        } else {
            0
        })
    });
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    // A form's `GetName` (its editor id): none.
    e.register(0x0060_1000, |_, _| eax(0));
    e.register(0x0071_d0a0, |e, a| {
        let format = guest_text(e, a[1]);
        let list = e.mem.bytes(a[2], 16);
        let line = super::super::printf::ucrt_format(e, &format, &list);
        LINES.with(|l| l.borrow_mut().push(line));
        eax(0)
    });
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    // The calling script is mod 5's.
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    e
}

fn console_on(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

// ---- expressions ----

fn var(var_type: u8, idx: u16) -> Vec<u8> {
    let mut b = vec![b'V', var_type, 0, 0];
    b.extend_from_slice(&idx.to_le_bytes());
    b
}
fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
fn text(t: &str) -> Vec<u8> {
    let mut b = vec![b'S'];
    b.extend_from_slice(&(t.len() as u16).to_le_bytes());
    b.extend_from_slice(t.as_bytes());
    b
}

/// An expression command's call: the count and each one-token expression.
fn run(e: &mut Engine, opcode: u32, tokens: &[Vec<u8>]) -> f64 {
    let mut bytes = vec![tokens.len() as u8];
    for t in tokens {
        bytes.extend_from_slice(&(t.len() as u16 + 2).to_le_bytes());
        bytes.extend_from_slice(t);
    }
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    e.mem.write(DATA + 4, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = crate::table::record_of(e, opcode);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    e.call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET]);
    e.mem.f64(RESULT)
}

/// A string variable's text.
fn string_of(e: &Engine, id: f64) -> String {
    String::from_utf8(vars::string(e, id as u32).expect("a string")).unwrap()
}

/// A form at `at` (its id its address), with a name (`TESFullName` at `+0x30`) when given.
fn form(e: &mut Engine, at: u32, name: Option<&str>) -> u32 {
    e.mem.set_u8(at + 4, 0x18);
    e.mem.set_u32(at + 0xc, at);
    let vtable = at + 0x400;
    e.mem.set_u32(at, vtable);
    e.mem.set_u32(vtable + 0x130, 0x0060_1000);
    if let Some(name) = name {
        set_classes(e, at, component(rtti::TES_FULL_NAME).0);
        let text = at + 0x200;
        e.mem.write(text, name.as_bytes());
        e.mem.set_u8(text + name.len() as u32, 0);
        let full = at + component(rtti::TES_FULL_NAME).1;
        e.mem.set_u32(full + 4, text);
        e.mem.set_u16(full + 8, name.len() as u16);
    }
    at
}

// ---- Commands_Console.cpp ----

/// `PrintToConsole` prints its formatted line and answers 1; a line of 512 or more is refused
/// with a line saying so. Before the first frame nothing prints (the answer is still 1).
#[test]
fn print_to_console_prints_the_formatted_line() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x14a0, 0, &[s("got %g"), z(2.5)]), (1, 1.0));
    assert_eq!(lines(), ["got 2.5"]);
    let long = "x".repeat(600);
    assert_eq!(call(&mut e, 0x14a0, 0, &[s(&long)]), (1, 0.0));
    assert_eq!(
        lines(),
        ["PrintToConsole >> Max length exceeded (512 chars)"]
    );
    let just_under = "y".repeat(511);
    assert_eq!(call(&mut e, 0x14a0, 0, &[s(&just_under)]).1, 1.0);
    assert_eq!(lines(), [just_under]);
    let at_limit = "z".repeat(512);
    assert_eq!(call(&mut e, 0x14a0, 0, &[s(&at_limit)]), (1, 0.0));
    assert_eq!(
        lines(),
        ["PrintToConsole >> Max length exceeded (512 chars)"]
    );
    e.ext::<crate::Nvse>().main_thread_recorded = false;
    assert_eq!(call(&mut e, 0x14a0, 0, &[s("early")]), (1, 1.0));
    assert!(lines().is_empty());
}

/// Debug mode is a bit per mod: `SetDebugMode` without a mod sets the calling script's,
/// `GetDebugMode` reads it, `DebugPrint` and `PrintDebug` print only while it is on; mods 0 and
/// 0xFF are never toggled; in console mode both commands say what they did.
#[test]
fn debug_mode_is_per_mod() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x14a3, 0, &[]).1, 0.0);
    call(&mut e, 0x14a1, 0, &[s("hidden")]);
    run(&mut e, 0x15bd, &[text("hidden too")]);
    assert!(lines().is_empty());
    call(&mut e, 0x14a2, 0, &[n(1)]);
    assert_eq!(call(&mut e, 0x14a3, 0, &[]).1, 1.0);
    assert_eq!(call(&mut e, 0x14a3, 0, &[n(5)]).1, 1.0);
    assert_eq!(call(&mut e, 0x14a3, 0, &[n(6)]).1, 0.0);
    assert_eq!(call(&mut e, 0x14a1, 0, &[s("shown %g"), z(3.0)]), (1, 1.0));
    run(&mut e, 0x15bd, &[text("shown too")]);
    assert_eq!(lines(), ["shown 3", "shown too"]);
    call(&mut e, 0x14a2, 0, &[n(0)]);
    call(&mut e, 0x14a1, 0, &[s("off again")]);
    assert!(lines().is_empty());
    for index in [0, 0xff] {
        call(&mut e, 0x14a2, 0, &[n(1), n(index)]);
        assert_eq!(call(&mut e, 0x14a3, 0, &[n(index)]).1, 0.0);
    }
    // Another mod's bit, and the same bit of the next field (mod 0x27 is field 1, bit 7).
    call(&mut e, 0x14a2, 0, &[n(1), n(0x27)]);
    assert_eq!(call(&mut e, 0x14a3, 0, &[n(0x27)]).1, 1.0);
    assert_eq!(call(&mut e, 0x14a3, 0, &[n(0x07)]).1, 0.0);
    console_on(&mut e);
    call(&mut e, 0x14a2, 0, &[n(1), n(7)]);
    call(&mut e, 0x14a3, 0, &[n(7)]);
    call(&mut e, 0x14a2, 0, &[n(0), n(7)]);
    call(&mut e, 0x14a3, 0, &[n(7)]);
    assert_eq!(
        lines(),
        [
            "Debug statements toggled on for mod 07",
            "Debug statements on for mod 07",
            "Debug statements toggled off for mod 07",
            "Debug statements off for mod 07",
        ]
    );
}

/// `SetConsoleEcho` answers the echo as it was and sets the byte; `GetConsoleEcho` reads it.
/// The console's output file: `HasConsoleOutputFilename`, and its name as a string variable.
#[test]
fn console_echo_and_output_file() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x15b1, 0, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x15b2, 0, &[n(1)]).1, 0.0);
    assert_eq!(e.mem.u8(ECHO), 1);
    assert_eq!(call(&mut e, 0x15b1, 0, &[]).1, 1.0);
    assert_eq!(call(&mut e, 0x15b2, 0, &[n(0)]).1, 1.0);
    assert_eq!(e.mem.u8(ECHO), 0);
    assert_eq!(call(&mut e, 0x15f4, 0, &[]).1, 0.0);
    let empty = call(&mut e, 0x15f5, 0, &[]).1;
    assert_eq!(string_of(&e, empty), "");
    e.mem.write(CONSOLE + 0x810, b"scof.txt\0");
    assert_eq!(call(&mut e, 0x15f4, 0, &[]).1, 1.0);
    let name = call(&mut e, 0x15f5, 0, &[]).1;
    assert_eq!(string_of(&e, name), "scof.txt");
}

// ---- Commands_Scripting.cpp ----

/// `Print` prints the expression's string representation: a string as it is, a number in
/// `%g`, a form as `[id: .., edid: .., name: ..]`, an array as `[..]` (a map with its keys, a
/// number key in `%f`, a string key quoted); a line of 512 or more in pieces of 500 marked
/// " ...".
#[test]
fn print_shows_the_string_representation() {
    let mut e = setup();
    run(&mut e, 0x1541, &[text("plain")]);
    run(&mut e, 0x1541, &[num(0.1)]);
    run(&mut e, 0x1541, &[num(1e20)]);
    let named = form(&mut e, 0x0200_1000, Some("Stimpak"));
    let nameless = form(&mut e, 0x0200_2000, None);
    add_local(&mut e, 1, "f1", f64::from_bits(u64::from(named)));
    add_local(&mut e, 2, "f2", f64::from_bits(u64::from(nameless)));
    run(&mut e, 0x1541, &[var(4, 1)]);
    run(&mut e, 0x1541, &[var(4, 2)]);
    assert_eq!(
        lines(),
        [
            "plain",
            "0.1",
            "1e+20",
            "[id: 2001000, edid: \"\", name: \"Stimpak\"]",
            "[id: 2002000, edid: \"\", name: \"<no name>\"]",
        ]
    );
    let inner = vars::create_array(&mut e, KEY_NUMBER, true, 5);
    vars::set_element(&mut e, inner, &Key::Number(0.0), Value::Str(b"in".to_vec()));
    let list = vars::create_array(&mut e, KEY_NUMBER, true, 5);
    for (i, v) in [
        Value::Number(1.5),
        Value::Str(b"two".to_vec()),
        Value::Form(named),
        Value::Form(0x0700_0000),
        Value::Array(inner),
        Value::Array(9999),
    ]
    .into_iter()
    .enumerate()
    {
        vars::set_element(&mut e, list, &Key::Number(i as f64), v);
    }
    let map = vars::create_array(&mut e, KEY_NUMBER, false, 5);
    vars::set_element(&mut e, map, &Key::Number(2.0), Value::Number(4.0));
    vars::set_element(&mut e, map, &Key::Number(-1.5), Value::Str(b"x".to_vec()));
    let strings = vars::create_array(&mut e, KEY_STRING, false, 5);
    vars::set_element(
        &mut e,
        strings,
        &Key::Str(b"b".to_vec()),
        Value::Number(1.0),
    );
    vars::set_element(
        &mut e,
        strings,
        &Key::Str(b"a".to_vec()),
        Value::Number(2.0),
    );
    let empty = vars::create_array(&mut e, KEY_NUMBER, true, 5);
    for (i, id) in [list, map, strings, empty].into_iter().enumerate() {
        add_local(&mut e, 10 + i as u16, "arr", f64::from(id));
        run(&mut e, 0x1541, &[var(3, 10 + i as u16)]);
    }
    assert_eq!(
        lines(),
        [
            "[1.5, \"two\", [id: 2001000, edid: \"\", name: \"Stimpak\"], null, [\"in\"], invalid array]",
            "[-1.500000: \"x\", 2.000000: 4]",
            "[\"a\": 2, \"b\": 1]",
            "[]",
        ]
    );
    let long: String = (0..1010)
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    run(&mut e, 0x1541, &[text(&long)]);
    let out = lines();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], format!("{} ...", &long[..500]));
    assert_eq!(out[1], format!("{} ...", &long[500..1000]));
    assert_eq!(out[2], long[1000..]);
}

/// `ToString`: a string, a number in `%g`, a form's name ("<no name>" without one); `TypeOf`:
/// "Number", "String", "Form", "Array", "Map", "StringMap" or "<Bad Array>".
#[test]
fn to_string_and_type_of() {
    let mut e = setup();
    let id = run(&mut e, 0x1540, &[text("abc")]);
    assert_eq!(string_of(&e, id), "abc");
    let id = run(&mut e, 0x1540, &[num(2.25)]);
    assert_eq!(string_of(&e, id), "2.25");
    let named = form(&mut e, 0x0200_1000, Some("Stimpak"));
    let nameless = form(&mut e, 0x0200_2000, None);
    add_local(&mut e, 1, "f1", f64::from_bits(u64::from(named)));
    add_local(&mut e, 2, "f2", f64::from_bits(u64::from(nameless)));
    let id = run(&mut e, 0x1540, &[var(4, 1)]);
    assert_eq!(string_of(&e, id), "Stimpak");
    let id = run(&mut e, 0x1540, &[var(4, 2)]);
    assert_eq!(string_of(&e, id), "<no name>");
    let packed = vars::create_array(&mut e, KEY_NUMBER, true, 5);
    let map = vars::create_array(&mut e, KEY_NUMBER, false, 5);
    let strings = vars::create_array(&mut e, KEY_STRING, false, 5);
    for (i, array) in [packed, map, strings, 9999].into_iter().enumerate() {
        add_local(&mut e, 10 + i as u16, "arr", f64::from(array));
    }
    let mut types = Vec::new();
    for tok in [
        num(1.0),
        text("s"),
        var(4, 1),
        var(3, 10),
        var(3, 11),
        var(3, 12),
        var(3, 13),
    ] {
        let id = run(&mut e, 0x1543, &[tok]);
        types.push(string_of(&e, id));
    }
    assert_eq!(
        types,
        [
            "Number",
            "String",
            "Form",
            "Array",
            "Map",
            "StringMap",
            "<Bad Array>"
        ]
    );
}

/// `PrintF` appends the string and a new line to the file it names (text mode: `\r\n`, inside
/// the string too); `PrintDebugF` only in debug mode.
#[test]
fn print_f_appends_to_the_file() {
    let mut e = setup();
    e.register(CREATE_FILE_A, |e, a| {
        let path = guest_text(e, a[0]);
        FILES.with(|f| {
            f.borrow_mut().entry(path.clone()).or_default();
        });
        let h = HANDLES.with(|h| {
            let mut h = h.borrow_mut();
            let id = 0x100 + h.len() as u32;
            h.insert(id, path);
            id
        });
        eax(h)
    });
    e.register(WRITE_FILE, |e, a| {
        let path = HANDLES.with(|h| h.borrow()[&a[0]].clone());
        let bytes = e.mem.bytes(a[1], a[2]);
        FILES.with(|f| f.borrow_mut().get_mut(&path).unwrap().extend(bytes));
        e.mem.set_u32(a[3], a[2]);
        eax(1)
    });
    e.register(SET_FILE_POINTER, |_, _| eax(0));
    e.register(CLOSE_HANDLE, |_, _| eax(1));
    run(&mut e, 0x15f6, &[text("log.txt"), text("one\ntwo")]);
    run(&mut e, 0x15f6, &[text("log.txt"), text("three")]);
    run(&mut e, 0x15f7, &[text("log.txt"), text("debug off")]);
    call(&mut e, 0x14a2, 0, &[n(1)]);
    run(&mut e, 0x15f7, &[text("log.txt"), text("debug on")]);
    let file = FILES.with(|f| f.borrow()[&b"log.txt".to_vec()].clone());
    assert_eq!(file, b"one\r\ntwo\r\nthree\r\ndebug on\r\n");
}

/// `PrintVar` prints `name: value` for each variable (a number in `%g`, a string in quotes);
/// nothing when the first argument is not a variable. `DebugPrintVar` only in debug mode.
#[test]
fn print_var_names_each_variable() {
    let mut e = setup();
    add_local(&mut e, 1, "count", 3.0);
    let id = vars::add_string(&mut e, 5, b"abc", false);
    add_local(&mut e, 2, "label", f64::from(id));
    run(&mut e, 0x1626, &[var(0, 1), var(2, 2)]);
    assert_eq!(lines(), ["count: 3", "label: \"abc\""]);
    run(&mut e, 0x1626, &[num(1.0), var(0, 1)]);
    assert!(lines().is_empty());
    run(&mut e, 0x165c, &[var(0, 1)]);
    assert!(lines().is_empty());
    call(&mut e, 0x14a2, 0, &[n(1)]);
    run(&mut e, 0x165c, &[var(0, 1)]);
    assert_eq!(lines(), ["count: 3"]);
}

// ---- Commands_Script.cpp ----

/// A reference at `at` (form type `0x3A`) of base form `base`.
fn reference(e: &mut Engine, at: u32, base: u32) -> u32 {
    e.mem.set_u8(at + 4, 0x3a);
    e.mem.set_u32(at + 0xc, at);
    e.mem.set_u32(at + 0x20, base);
    set_classes(e, at, component(rtti::TES_OBJECT_REFR).0);
    at
}

/// `IsFormValid`: 1 for a form given, or for the calling reference's base form; without either
/// 0 and no line. In console mode it says which.
#[test]
fn is_form_valid_checks_the_form_or_the_caller() {
    let mut e = setup();
    let base = form(&mut e, 0x0200_1000, None);
    let refr = reference(&mut e, 0x0200_3000, base);
    set_script_ref(&mut e, 1, base);
    assert_eq!(call(&mut e, 0x14ba, 0, &[r(1)]), (1, 1.0));
    assert_eq!(call(&mut e, 0x14ba, 0, &[]), (1, 0.0));
    assert_eq!(call(&mut e, 0x14ba, refr, &[]), (1, 1.0));
    let baseless = reference(&mut e, 0x0200_4000, 0);
    console_on(&mut e);
    assert_eq!(call(&mut e, 0x14ba, baseless, &[]), (1, 0.0));
    assert_eq!(call(&mut e, 0x14ba, 0, &[r(1)]), (1, 1.0));
    assert_eq!(call(&mut e, 0x14ba, 0, &[]), (1, 0.0));
    assert_eq!(lines(), ["Invalid Form", "Valid Form!"]);
}

/// `IsReference`: 1 when the argument reads as a reference, 0 for any other form; the console
/// says which either way.
#[test]
fn is_reference_takes_only_references() {
    let mut e = setup();
    let base = form(&mut e, 0x0200_1000, None);
    let refr = reference(&mut e, 0x0200_3000, base);
    set_script_ref(&mut e, 1, refr);
    set_script_ref(&mut e, 2, base);
    console_on(&mut e);
    assert_eq!(call(&mut e, 0x14bb, 0, &[r(1)]).1, 1.0);
    assert_eq!(call(&mut e, 0x14bb, 0, &[r(2)]).1, 0.0);
    assert_eq!(lines(), ["IsReference", "Not reference!"]);
}

// ---- Commands_MiscRef.cpp ----

/// `Update3D`: the calling reference's 3D let go (`Set3D(NULL, true)`) and queued to load again,
/// and 1; without a reference 0 and no call.
#[test]
fn update_3d_reloads_the_reference() {
    let mut e = setup();
    let refr = reference(&mut e, 0x0200_3000, 0x0200_1000);
    let vtable = 0x0200_6000;
    e.mem.set_u32(refr, vtable);
    e.mem.set_u32(vtable + 0x1cc, 0x0060_2000);
    e.register_double(0x0060_2000, |_, a| {
        CALLS.with(|c| c.borrow_mut().push(a[..3].to_vec()));
        Ret::default()
    });
    e.mem.set_u32(MODEL_LOADER, 0x0200_7000);
    e.register_double(QUEUE_REFERENCE, |_, a| {
        CALLS.with(|c| c.borrow_mut().push(a[..4].to_vec()));
        Ret::default()
    });
    assert_eq!(call(&mut e, 0x152d, 0, &[]), (1, 0.0));
    assert!(CALLS.with(|c| c.borrow().is_empty()));
    assert_eq!(call(&mut e, 0x152d, refr, &[]), (1, 1.0));
    assert_eq!(
        CALLS.with(|c| c.borrow().clone()),
        [vec![refr, 0, 1], vec![0x0200_7000, refr, 1, 0]]
    );
}
