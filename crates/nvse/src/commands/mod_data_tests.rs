//! Tests of the mod-local data, `GetUserTime`, `GetSelfAlt`, `GetCommandOpcode` and `MatchesAnyOf`
//! through the commands' records. The answers pinned are xNVSE's as its source gives them; none of
//! these commands has been run in the real game yet.

use super::super::test_support::*;
use crate::vars::{self, Key, Value, KEY_NUMBER, KEY_STRING};
use engine_core::prelude::*;

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn setup() -> Engine {
    let mut e = engine_with_table();
    e.register(0x00ec_a7f4, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(0x0048_39c0, |_, a| eax(a[0]));
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    // The calling script is mod 5's.
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    e
}

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

/// An expression command's call: the count and each one-token expression; `al` and the result.
fn run(e: &mut Engine, opcode: u32, tokens: &[Vec<u8>]) -> (u32, f64) {
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
    let ok = e
        .call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET])
        .eax;
    (ok & 0xff, e.mem.f64(RESULT))
}

fn get(e: &mut Engine, key: &str) -> f64 {
    run(e, 0x1548, &[text(key)]).1
}

/// The mod-local data: kept per mod, found by key without case (the key text kept is the first
/// one), a number, a form or a string; never an array, an empty key or a call with another
/// number of arguments. `ModLocalDataExists`, `RemoveModLocalData` (1 when there was one).
#[test]
fn mod_local_data_by_key() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x1549, &[text("Count"), num(5.0)]).1, 1.0);
    assert_eq!(get(&mut e, "count"), 5.0);
    assert_eq!(run(&mut e, 0x1549, &[text("COUNT"), num(6.0)]).1, 1.0);
    assert_eq!(get(&mut e, "Count"), 6.0);
    assert_eq!(run(&mut e, 0x1549, &[text("Name"), text("Doc")]).1, 1.0);
    let id = get(&mut e, "name");
    assert_eq!(vars::string(&e, id as u32).unwrap(), b"Doc");
    add_local(&mut e, 1, "who", f64::from_bits(0x0200_1000));
    assert_eq!(run(&mut e, 0x1549, &[text("Who"), var(4, 1)]).1, 1.0);
    assert_eq!(get(&mut e, "who").to_bits() as u32, 0x0200_1000);
    let list = vars::create_array(&mut e, KEY_NUMBER, true, 5);
    add_local(&mut e, 2, "list", f64::from(list));
    assert_eq!(run(&mut e, 0x1549, &[text("List"), var(3, 2)]).1, 0.0);
    assert_eq!(run(&mut e, 0x1549, &[text(""), num(1.0)]).1, 0.0);
    assert_eq!(run(&mut e, 0x1549, &[text("One")]).1, 0.0);
    assert_eq!(run(&mut e, 0x154a, &[text("list")]).1, 0.0);
    assert_eq!(run(&mut e, 0x154a, &[text("NAME")]).1, 1.0);
    assert_eq!(get(&mut e, "nothing"), 0.0);
    // Another mod sees none of it.
    e.mem.set_u32(SCRIPT + 0xc, 0x0600_0d00);
    assert_eq!(run(&mut e, 0x154a, &[text("count")]).1, 0.0);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    // The whole set as a string map, by the keys first given.
    let all = run(&mut e, 0x154c, &[]).1 as u32;
    let map = vars::array(&e, all).unwrap().clone();
    assert_eq!(map.key_type, KEY_STRING);
    let mut keys: Vec<Vec<u8>> = map
        .keys()
        .iter()
        .map(|k| match k {
            Key::Str(s) => s.clone(),
            _ => panic!("a string key"),
        })
        .collect();
    keys.sort();
    assert_eq!(keys, [b"Count".to_vec(), b"Name".to_vec(), b"Who".to_vec()]);
    assert_eq!(vars::array_owning_mod(&e, all), 5);
    assert_eq!(run(&mut e, 0x154b, &[text("count")]).1, 1.0);
    assert_eq!(run(&mut e, 0x154b, &[text("count")]).1, 0.0);
    assert_eq!(run(&mut e, 0x154a, &[text("Count")]).1, 0.0);
}

/// `GetModLocalData` tells the running expression what it answered: a form, a string, else a
/// number (also for no value).
#[test]
fn mod_local_data_return_types() {
    let mut e = setup();
    run(&mut e, 0x1549, &[text("s"), text("x")]);
    run(&mut e, 0x1549, &[text("n"), num(2.0)]);
    add_local(&mut e, 1, "who", f64::from_bits(0x0200_1000));
    run(&mut e, 0x1549, &[text("f"), var(4, 1)]);
    let expected = |e: &mut Engine, key: &str| {
        crate::expr::with_evaluator(e, |e| {
            crate::expr::expect_return_type(e, crate::table::Returns::Ambiguous);
            get(e, key);
            crate::expr::expected_return_type(e)
        })
    };
    use crate::table::Returns;
    assert_eq!(expected(&mut e, "s"), Some(Returns::String));
    assert_eq!(expected(&mut e, "f"), Some(Returns::Form));
    assert_eq!(expected(&mut e, "n"), Some(Returns::Default));
    assert_eq!(expected(&mut e, "none"), Some(Returns::Default));
}

/// `GetUserTime`: the local clock as a string map of the calling mod; the day of the week from 1
/// (Sunday).
#[test]
fn user_time_as_a_string_map() {
    let mut e = setup();
    e.register(0x00fd_f0d8, |e, a| {
        for (i, v) in [2026u16, 11, 4, 12, 13, 14, 15, 16].into_iter().enumerate() {
            e.mem.set_u16(a[0] + 2 * i as u32, v);
        }
        eax(0)
    });
    let id = run(&mut e, 0x1547, &[]).1 as u32;
    assert_eq!(vars::array_owning_mod(&e, id), 5);
    let mut got = Vec::new();
    for key in [
        "Year",
        "Month",
        "DayOfWeek",
        "Day",
        "Hour",
        "Minute",
        "Second",
        "Millisecond",
    ] {
        let el = vars::get_element(&mut e, id, &Key::Str(key.as_bytes().to_vec())).unwrap();
        match el.data {
            vars::Data::Number(v) => got.push(v),
            _ => panic!("{key} is a number"),
        }
    }
    assert_eq!(got, [2026.0, 11.0, 5.0, 12.0, 13.0, 14.0, 15.0, 16.0]);
}

/// `GetSelfAlt` and `GetSelfAlt_OLD`: the calling reference's id in the low word, 0 without one.
#[test]
fn self_alt_is_the_caller() {
    let mut e = setup();
    e.mem.set_u32(0x0200_1000 + 0xc, 0x0012_3456);
    for opcode in [0x1669, 0x164a] {
        assert_eq!(
            call(&mut e, opcode, 0x0200_1000, &[]).1.to_bits() as u32,
            0x0012_3456
        );
        assert_eq!(call(&mut e, opcode, 0, &[]).1, 0.0);
    }
}

/// `GetCommandOpcode`: by long or short name without case; 0 for no command, and 0 for a name
/// the console table holds first (xNVSE's one name map holds the console's opcode, which the
/// script table does not have).
#[test]
fn command_opcodes_by_name() {
    let mut e = setup();
    let op = |e: &mut Engine, name: &str| call(e, 0x162c, 0, &[s(name)]).1;
    assert_eq!(op(&mut e, "GetUserTime"), f64::from(0x1547));
    assert_eq!(op(&mut e, "getusertime"), f64::from(0x1547));
    assert_eq!(op(&mut e, "ThisAlt"), f64::from(0x1669));
    assert_eq!(op(&mut e, "NoSuchCommand"), 0.0);
    // The test engine's console records have no names: the empty name is theirs.
    assert_eq!(op(&mut e, ""), 0.0);
    // Two script records with one short name: the lower opcode.
    let short = crate::table::guest_str(&mut e, b"ThisAlt");
    let old = crate::table::record_of(&e, 0x164a);
    e.mem.set_u32(old + 4, short);
    assert_eq!(op(&mut e, "thisalt"), f64::from(0x164a));
    // A console record with a script command's name: the console's, so 0.
    let name = crate::table::guest_str(&mut e, b"GetUserTime");
    e.mem.set_u32(0x0118_e8e0 + 5 * 0x28, name);
    assert_eq!(op(&mut e, "GetUserTime"), 0.0);
}

/// `MatchesAnyOf` compares by the first value's kind: numbers within 0.0001 (as floats), strings
/// without case, forms by id, arrays by keys and elements; 0 without a match.
#[test]
fn matches_any_of_by_the_first_kind() {
    let mut e = setup();
    let m = |e: &mut Engine, toks: &[Vec<u8>]| run(e, 0x166f, toks).1;
    assert_eq!(m(&mut e, &[num(1.0), num(2.0), num(1.00001)]), 1.0);
    assert_eq!(m(&mut e, &[num(1.0), num(2.0), num(1.001)]), 0.0);
    assert_eq!(m(&mut e, &[text("Doc"), text("x"), text("DOC")]), 1.0);
    assert_eq!(m(&mut e, &[text("Doc"), text("Do")]), 0.0);
    add_local(&mut e, 1, "a", f64::from_bits(0x0200_1000));
    add_local(&mut e, 2, "b", f64::from_bits(0x0200_2000));
    add_local(&mut e, 3, "c", f64::from_bits(0x0200_1000));
    assert_eq!(m(&mut e, &[var(4, 1), var(4, 2), var(4, 3)]), 1.0);
    assert_eq!(m(&mut e, &[var(4, 1), var(4, 2)]), 0.0);
    let arrays: Vec<u32> = (0..3)
        .map(|i| {
            let id = vars::create_array(&mut e, KEY_NUMBER, true, 5);
            vars::set_element(
                &mut e,
                id,
                &Key::Number(0.0),
                Value::Number(f64::from(i % 2)),
            );
            id
        })
        .collect();
    for (i, id) in arrays.iter().enumerate() {
        add_local(&mut e, 10 + i as u16, "arr", f64::from(*id));
    }
    assert_eq!(m(&mut e, &[var(3, 10), var(3, 11), var(3, 12)]), 1.0);
    assert_eq!(m(&mut e, &[var(3, 10), var(3, 11)]), 0.0);
    // An array that does not exist matches nothing (xNVSE would stop the game).
    add_local(&mut e, 13, "gone", 9999.0);
    assert_eq!(m(&mut e, &[var(3, 10), var(3, 13)]), 0.0);
}

/// `set s to GetModLocalData "name"` (a classic set line) with a string kept: the command's own
/// evaluator is open when the string goes out, so the answer is a new temporary string, and the
/// variable's own string is left as it was (xNVSE's `ExpressionEvaluator` lives through the
/// command).
#[test]
fn a_set_statement_gets_a_temporary_string() {
    let mut e = setup();
    run(&mut e, 0x1549, &[text("name"), text("Doc")]);
    let id = vars::add_string(&mut e, 5, b"old", false);
    add_local(&mut e, 3, "s", f64::from(id));
    // The statement, the variable, the expression's length, a space, the command token, then the
    // command's own arguments: the count and one expression.
    let mut line = vec![0x15, 0, 0, 0, b's', 3, 0, 0, 0, b' ', b'X'];
    line.extend_from_slice(&0x1548u16.to_le_bytes());
    let key = text("name");
    let mut args = vec![1u8];
    args.extend_from_slice(&(key.len() as u16 + 2).to_le_bytes());
    args.extend(key);
    line.extend_from_slice(&(args.len() as u16).to_le_bytes());
    line.extend(args);
    let at = DATA + 0x100;
    e.mem.write(at, &line);
    let record = crate::table::record_of(&e, 0x1548);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    e.mem.set_u32(OFFSET, 5);
    e.mem.set_f64(RESULT, 0.0);
    e.call(
        execute,
        &[params, at + 10, 0, 0, SCRIPT, LIST, RESULT, OFFSET],
    );
    let got = e.mem.f64(RESULT) as u32;
    assert_ne!(got, id);
    assert_eq!(vars::string(&e, got).unwrap(), b"Doc");
    assert_eq!(vars::string(&e, id).unwrap(), b"old");
    vars::clean_strings(&mut e);
    assert!(vars::string(&e, got).is_none(), "a temporary string");
    assert_eq!(vars::string(&e, id).unwrap(), b"old");
}

thread_local! {
    static BOXES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    static HEARD: std::cell::RefCell<Vec<(u32, Vec<u8>, u32)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn boxes() -> Vec<String> {
    BOXES.with(|b| std::mem::take(&mut *b.borrow_mut()))
}

fn heard(e: &mut Engine, m: &crate::plugins::Message) {
    let text = crate::table::guest_text(e, m.data);
    HEARD.with(|h| h.borrow_mut().push((m.kind, text, m.len)));
}

/// A built-in plugin "TestPlugin" of version 7 with a listener, and one not listed; the message
/// box records its text; the calling script's editor id is "MyScript".
fn with_plugins(e: &mut Engine) {
    BOXES.with(|b| b.borrow_mut().clear());
    HEARD.with(|h| h.borrow_mut().clear());
    let plugin = |name, listed, base| crate::plugins::BuiltIn {
        file: name,
        name,
        version: 7,
        opcode_base: base,
        listed,
        listener: Some(heard),
        save: None,
        load: None,
        pre_load: None,
        new_game: None,
    };
    crate::plugins::register(e, plugin("TestPlugin", true, 0x9000));
    crate::plugins::register(e, plugin("Hidden", false, 0x9100));
    e.register(0x0070_3e80, |e, a| {
        let text = String::from_utf8(crate::table::guest_text(e, a[0])).unwrap();
        assert_eq!(crate::table::guest_text(e, a[8]), b"Ok");
        assert_eq!(a[3], 0x005b_4a70);
        BOXES.with(|b| b.borrow_mut().push(text));
        eax(1)
    });
    // No console: the "not found" line goes nowhere.
    e.register(0x0071_b160, |_, _| eax(0));
    let vtable = 0x0300_9000;
    e.mem.set_u32(SCRIPT, vtable);
    e.mem.set_u32(vtable + 0x130, 0x0060_3000);
    e.register(0x0060_3000, |e, _| {
        eax(crate::table::guest_str(e, b"MyScript"))
    });
}

/// `PluginVersion`: a plugin that is missing, or older than asked, or an xNVSE older than asked,
/// shows one message box and stops the line; a plugin name warned of once (in any case) is not
/// warned of again. A requirement met shows nothing.
#[test]
fn plugin_version_warns_once() {
    let mut e = setup();
    with_plugins(&mut e);
    assert_eq!(run(&mut e, 0x1676, &[text("TestPlugin"), num(7.0)]).0, 1);
    assert_eq!(
        run(&mut e, 0x1676, &[text("nvse"), num(f64::from(0x0604_0090))]).0,
        1
    );
    assert!(boxes().is_empty());
    assert_eq!(run(&mut e, 0x1676, &[text("Missing"), num(1.0)]).0, 0);
    assert_eq!(run(&mut e, 0x1676, &[text("Hidden"), num(1.0)]).0, 0);
    assert_eq!(run(&mut e, 0x1676, &[text("MISSING"), num(1.0)]).0, 1);
    assert_eq!(run(&mut e, 0x1676, &[text("testplugin"), num(8.0)]).0, 0);
    assert_eq!(
        run(&mut e, 0x1676, &[text("NVSE"), num(f64::from(0x0700_0010))]).0,
        0
    );
    assert_eq!(
        boxes(),
        [
            "A script ('MyScript') requires the plugin named 'Missing' but it was not found. Your game will NOT work correctly. Please locate the plugin.",
            "A script ('MyScript') requires the plugin named 'Hidden' but it was not found. Your game will NOT work correctly. Please locate the plugin.",
            "A script ('MyScript') requires the plugin 'testplugin' >= '8' (You have version 7).\nYour game will NOT work correctly. Please update the plugin.",
            "A script ('MyScript') requires xNVSE version >= 7.0.1 (You have version 6.4.9).\nYour game will NOT work correctly. Please update xNVSE.",
        ]
    );
    // One argument: nothing.
    assert_eq!(run(&mut e, 0x1676, &[text("Other")]).0, 1);
    assert!(boxes().is_empty());
}

/// `ReloadPluginConfig`: the message (25, the name, its length with the 0) to the named loaded
/// plugin only, and 1; any other name prints "not found" and answers 0.
#[test]
fn reload_plugin_config_tells_that_plugin() {
    let mut e = setup();
    with_plugins(&mut e);
    assert_eq!(call(&mut e, 0x167a, 0, &[s("testplugin")]), (1, 1.0));
    assert_eq!(
        HEARD.with(|h| h.borrow().clone()),
        [(25, b"testplugin".to_vec(), 11)]
    );
    assert_eq!(call(&mut e, 0x167a, 0, &[s("Hidden")]), (1, 0.0));
    assert_eq!(HEARD.with(|h| h.borrow().len()), 1);
}
