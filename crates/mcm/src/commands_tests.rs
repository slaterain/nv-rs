//! Tests of MCM's commands against the DLL's behaviour (`build43-scratch/measure/mcm/NOTES.md`):
//! the registration, the paths each group and index build, the traits found and set (a tile tree
//! in guest memory, the game's trait calls stood in for), the missing ones, the 16-bit numbers,
//! the `Mass` loops, the INI commands over a stand-in of the mod files, and the C runtime's text
//! conversions against the VC++ 2010 runtime's measured answers.

use crate::commands::{atof, fixed4, full_name, mass_indices, COMMANDS};
use crate::menu::{self, path, MENU_ARRAY, MENU_COUNT, SET_FLOAT, SET_STRING, TRAIT_NAME_TO_ID};
use engine_core::mod_files::{self, ModFileCalls};
use engine_core::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// A call's guest data, as JIP's tests lay it out.
const DATA: u32 = 0x0300_0000;
const OFFSET: u32 = 0x0300_0f00;
const RESULT: u32 = 0x0300_0f08;
const SCRIPT: u32 = 0x0300_1000;
const LIST: u32 = 0x0300_2000;
/// The tile tree and the stand-ins' records.
const HEAP: u32 = 0x0400_0000;
const HEAP_TOP: u32 = 0x0400_fff0;
/// The last `Tile::SetString`: tile, id, text (copied).
const LAST_STRING: u32 = 0x0401_0000;
/// How many `Tile::SetFloat` and `Tile::SetString` calls were made.
const SET_CALLS: u32 = 0x0401_0100;

/// Trait ids: the menus' (`&StartMenu;` is 1013) and the others' from their names, without case
/// as the game's trait map compares them (`_stricmp` at `00a0d180`, a lower-cased hash at
/// `00a0d1b0`).
fn id_of(name: &[u8]) -> u32 {
    match name.to_ascii_lowercase().as_slice() {
        b"&startmenu;" => 1013,
        b"&hudmainmenu;" => 1004,
        // The last id the DLL takes (1001 + 0x53).
        b"&lastmenu;" => 1084,
        _ if name.first() == Some(&b'&') => 0x7000,
        _ => {
            0x2000
                + name
                    .iter()
                    .map(|&b| u32::from(b.to_ascii_lowercase()))
                    .sum::<u32>()
                    * 7
        }
    }
}

/// The value of trait `id` on `tile` (0 for none).
fn find_value(e: &Engine, tile: u32, id: u32) -> u32 {
    let (data, count) = (e.mem.u32(tile + 0x14), e.mem.u32(tile + 0x18));
    (0..count)
        .map(|i| e.mem.u32(data + 4 * i))
        .find(|&v| e.mem.u32(v) == id)
        .unwrap_or(0)
}

fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(DATA, 0x1_0000);
    e.map(HEAP, 0x2_0000);
    e.map(MENU_ARRAY & !0xfff, 0x1000);
    e.mem.set_u32(HEAP_TOP, HEAP + 0x100);
    nvse::table::install(&mut e);
    e.register(TRAIT_NAME_TO_ID, |e, a| Ret {
        eax: id_of(&e.mem.cstr(a[0])),
        ..Ret::default()
    });
    e.register(SET_FLOAT, |e, a| {
        let value = find_value(e, a[0], a[1]);
        assert_ne!(value, 0, "SetFloat on a trait the tile lacks");
        e.mem.set_u32(value + 8, a[2]);
        e.mem.set_u32(SET_CALLS, e.mem.u32(SET_CALLS) + 1);
        Ret::default()
    });
    e.register(SET_STRING, |e, a| {
        let text = e.mem.cstr(a[2]);
        e.mem.set_u32(LAST_STRING, a[0]);
        e.mem.set_u32(LAST_STRING + 4, a[1]);
        e.mem.set_cstr(LAST_STRING + 8, &text);
        e.mem.set_u32(SET_CALLS, e.mem.u32(SET_CALLS) + 1);
        Ret::default()
    });
    crate::set_on(&mut e, true);
    crate::install(&mut e);
    e
}

/// `size` zeroed bytes of the test heap.
fn take(e: &mut Engine, size: u32) -> u32 {
    let at = e.mem.u32(HEAP_TOP);
    e.mem.set_u32(HEAP_TOP, at + size.next_multiple_of(16));
    at
}

/// A tile named `name` (none for `None`), with room for 16 values.
fn tile(e: &mut Engine, name: Option<&str>) -> u32 {
    let t = take(e, 0x40);
    if let Some(name) = name {
        let text = take(e, name.len() as u32 + 1);
        e.mem.set_cstr(text, name.as_bytes());
        e.mem.set_u32(t + 0x20, text);
    }
    let values = take(e, 64);
    e.mem.set_u32(t + 0x14, values);
    t
}

/// `child` appended to `parent`'s children (the game's doubly linked list).
fn adopt(e: &mut Engine, parent: u32, child: u32) {
    let node = take(e, 12);
    e.mem.set_u32(node + 8, child);
    let tail = e.mem.u32(parent + 8);
    if tail == 0 {
        e.mem.set_u32(parent + 4, node);
    } else {
        e.mem.set_u32(tail, node);
        e.mem.set_u32(node + 4, tail);
    }
    e.mem.set_u32(parent + 8, node);
}

/// A child of `parent` named `name`.
fn child(e: &mut Engine, parent: u32, name: &str) -> u32 {
    let t = tile(e, Some(name));
    adopt(e, parent, t);
    t
}

/// Trait `name` on `tile` with number `num`: its `Tile::Value`.
fn value(e: &mut Engine, tile: u32, name: &str, num: f32) -> u32 {
    let v = take(e, 0x10);
    e.mem.set_u32(v, id_of(name.as_bytes()));
    e.mem.set_u32(v + 4, tile);
    e.mem.set_f32(v + 8, num);
    let count = e.mem.u32(tile + 0x18);
    let data = e.mem.u32(tile + 0x14);
    e.mem.set_u32(data + 4 * count, v);
    e.mem.set_u32(tile + 0x18, count + 1);
    v
}

/// The start menu in the menu array: its tile.
fn start_menu(e: &mut Engine) -> u32 {
    let menu = tile(e, Some("StartMenu"));
    let array = take(e, 4 * 0x54);
    e.mem.set_u32(MENU_ARRAY, array);
    e.mem.set_u16(MENU_COUNT, 0x54);
    e.mem.set_u32(array + 4 * (1013 - 1001), menu);
    menu
}

/// MCM's tiles as its prefab lays them out, the traits the tests read.
struct Mcm {
    mcm: u32,
    option3: u32,
    mod5: u32,
    sub2: u32,
    title5: u32,
}

fn mcm_tree(e: &mut Engine) -> Mcm {
    let menu = start_menu(e);
    let mcm = child(e, menu, "MCM");
    value(e, mcm, "_MCM", 1.5);
    value(e, mcm, "visible", 0.0);
    let options = child(e, mcm, "MCM_Options");
    value(e, options, "_optionID", 0.0);
    let mut option3 = 0;
    for i in 1..=4 {
        let o = child(e, options, &format!("Option{i}"));
        value(e, o, "_value", i as f32);
        value(e, o, "_enable", 0.0);
        if i == 3 {
            option3 = o;
            let v = child(e, o, "value");
            let text = child(e, v, "text");
            value(e, text, "string", 0.0);
        }
    }
    let list = child(e, mcm, "MCM_ModList");
    value(e, list, "_version", 1.5);
    let mod5 = child(e, list, "Mod5");
    for t in ["_enable", "_modIndex", "_modIndex2"] {
        value(e, mod5, t, 0.0);
    }
    let text = child(e, mod5, "text");
    value(e, text, "string", 0.0);
    value(e, text, "width", 123.0);
    let sub2 = child(e, list, "SubMenu2");
    let text = child(e, sub2, "text");
    value(e, text, "width", 77.0);
    let titles = child(e, mcm, "MCM_ModTitle");
    let title5 = child(e, titles, "Title5");
    value(e, title5, "string", 0.0);
    Mcm {
        mcm,
        option3,
        mod5,
        sub2,
        title5,
    }
}

fn number(e: &mut Engine, path: &str) -> f32 {
    let v = menu::value_at(e, path.as_bytes());
    assert_ne!(v, 0, "{path}");
    e.mem.f32(v + 8)
}

// Compiled arguments, as the script compiler writes them.
fn int(v: i32) -> Vec<u8> {
    let mut b = vec![b'n'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
fn float(v: f64) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
fn s(text: &[u8]) -> Vec<u8> {
    let mut b = (text.len() as u16).to_le_bytes().to_vec();
    b.extend_from_slice(text);
    b
}
/// A reference argument: script reference `index` (from 1), whose form is `form`.
fn r(e: &mut Engine, index: u16, form: u32) -> Vec<u8> {
    let i = u32::from(index);
    let ref_var = 0x0300_a000 + i * 0x10;
    e.mem.set_u32(ref_var + 8, form);
    let node = 0x0300_b000 + i * 8;
    e.mem.set_u32(node, ref_var);
    // The script's reference list (`+0x44`): one node, the variable.
    e.mem.set_u32(SCRIPT + 0x44, ref_var);
    e.mem.set_u32(SCRIPT + 0x48, 0);
    let mut b = vec![b'r'];
    b.extend_from_slice(&index.to_le_bytes());
    b
}

/// Calls `opcode` through its record with `args`: the handler's answer and the result.
fn call(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> (u32, f64) {
    e.mem.set_u16(DATA, opcode as u16);
    let bytes: Vec<u8> = args.concat();
    e.mem.set_u16(DATA + 2, bytes.len() as u16 + 2);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    for (i, b) in bytes.iter().enumerate() {
        e.mem.set_u8(DATA + 6 + i as u32, *b);
    }
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -1.0);
    let record = nvse::table::record_of(e, opcode);
    assert_ne!(record, 0, "{opcode:04x}");
    let params = e.mem.u32(record + 0x14);
    let execute = e.mem.u32(record + 0x18);
    let ok = e
        .call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET])
        .eax
        & 0xff;
    (ok, e.mem.f64(RESULT))
}

/// The records as the DLL registers them: names, help texts with their quotes, parameter lists,
/// no condition; xNVSE's list names the plugin.
#[test]
fn the_commands_are_registered_as_the_dll_registers_them() {
    let e = engine();
    let names = |at: u32| nvse::table::guest_text(&e, e.mem.u32(at));
    for (i, c) in COMMANDS.iter().enumerate() {
        assert_eq!(c.opcode, 0x21c0 + i as u32);
        let record = nvse::table::record_of(&e, c.opcode);
        assert_eq!(names(record), c.name.as_bytes());
        assert_eq!(names(record + 4), c.short.as_bytes());
        assert_eq!(names(record + 0xc), c.help.as_bytes());
        assert_eq!(e.mem.u16(record + 0x12) as usize, c.params.len());
        assert_eq!(e.mem.u32(record + 0x20), 0, "no condition eval");
        let params = e.mem.u32(record + 0x14);
        for (k, p) in c.params.iter().enumerate() {
            let at = params + 12 * k as u32;
            assert_eq!(
                nvse::table::guest_text(&e, e.mem.u32(at)),
                p.name.as_bytes()
            );
            assert_eq!(e.mem.u32(at + 4), p.kind);
            assert_eq!(e.mem.u32(at + 8), u32::from(p.optional));
        }
    }
    assert_eq!(COMMANDS[0].short, "GetModINI");
    assert_eq!(COMMANDS[4].params.len(), 24);
    assert_eq!(COMMANDS[6].params.len(), 25);
    assert_eq!(COMMANDS[7].params[4].kind, 0x15);
    let plugins = &e.ext_get::<nvse::Nvse>().unwrap().plugins.built_ins;
    let me = plugins.iter().find(|p| p.file == "MCM.dll").unwrap();
    assert_eq!(
        (me.name, me.version, me.opcode_base),
        ("MCM Extensions", 1, 0x21c0)
    );
}

/// Off, nothing is registered: the opcodes stay as without the DLL.
#[test]
fn off_nothing_is_registered() {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    nvse::table::install(&mut e);
    crate::install(&mut e);
    assert!(!e.ext::<crate::Mcm>().installed);
    let record = nvse::table::record_of(&e, 0x21c2);
    if record != 0 {
        assert_eq!(e.mem.u32(record + 0x18), 0, "no execute");
    }
    assert!(e
        .ext_get::<nvse::Nvse>()
        .unwrap()
        .plugins
        .built_ins
        .iter()
        .all(|p| p.file != "MCM.dll"));
}

/// `GetMenuPath`: each group's place, the item index where the group has items, signed.
#[test]
fn paths_by_group_and_index() {
    let cases: &[(i32, i32, &str)] = &[
        (0, 0, "StartMenu/MCM/_MCM"),
        (0, 7, "StartMenu/MCM/_MCM"),
        (1, 0, "StartMenu/MCM/MCM_Options/_MCM"),
        (1, 3, "StartMenu/MCM/MCM_Options/Option3/_MCM"),
        (1, -2, "StartMenu/MCM/MCM_Options/Option-2/_MCM"),
        (2, 9, "StartMenu/MCM/MCM_Scale/_MCM"),
        (3, 0, "StartMenu/MCM/MCM_List/_MCM"),
        (3, 12, "StartMenu/MCM/MCM_List/Item12/_MCM"),
        (4, 1, "StartMenu/MCM/MCM_Trigger/_MCM"),
        (5, 0, "StartMenu/MCM/MCM_ModList/_MCM"),
        (5, 1, "StartMenu/MCM/MCM_ModList/Mod1/_MCM"),
        (5, 23, "StartMenu/MCM/MCM_ModList/Mod23/_MCM"),
        (5, 24, "StartMenu/MCM/MCM_ModList/SubMenu1/_MCM"),
        (5, 30, "StartMenu/MCM/MCM_ModList/SubMenu7/_MCM"),
        (5, -4, "StartMenu/MCM/MCM_ModList/Mod-4/_MCM"),
        (6, 1, "StartMenu/MCM/_MCM"),
        (9, 1, "StartMenu/MCM/MCM_Info/_MCM"),
        (15, 1, "StartMenu/MCM/MCM_Input/_MCM"),
        (17, 1, "StartMenu/MCM/MCM_Images/_MCM"),
        (-1, 1, "StartMenu/MCM/_MCM"),
    ];
    for &(kind, index, want) in cases {
        assert_eq!(
            path(kind, index, b"_MCM"),
            want.as_bytes(),
            "{kind} {index}"
        );
    }
}

/// The DLL's lookup: separators of both kinds, empty parts skipped, `name:n`, `*`, names without
/// case, a child without a name, a trait only as the last part, the menu's checks.
#[test]
fn the_lookup_follows_the_dlls_rules() {
    let mut e = engine();
    let t = mcm_tree(&mut e);
    assert_eq!(number(&mut e, "StartMenu/MCM/_MCM"), 1.5);
    assert_eq!(number(&mut e, r"StartMenu\MCM\_MCM"), 1.5);
    assert_eq!(number(&mut e, "StartMenu//MCM///_MCM"), 1.5);
    assert_eq!(
        number(&mut e, "startmenu/mcm/mcm_options/OPTION3/_value"),
        3.0
    );
    assert_eq!(number(&mut e, "StartMenu/MCM/MCM_Options/*:2/_value"), 3.0);
    assert_eq!(number(&mut e, "StartMenu/MCM/MCM_Options/*/_value"), 1.0);
    // `*` only alone; a colon first is part of the name.
    assert_eq!(
        menu::value_at(&mut e, b"StartMenu/MCM/MCM_Options/O*/_value"),
        0
    );
    assert_eq!(menu::value_at(&mut e, b"StartMenu/MCM/:1/_MCM"), 0);
    // A trait not last, a path that ends at a tile, a missing trait, no menu.
    assert_eq!(menu::value_at(&mut e, b"StartMenu/MCM/nope/_MCM"), 0);
    // A part that names a trait but not last is nothing too.
    assert_eq!(menu::value_at(&mut e, b"StartMenu/MCM/_MCM/x"), 0);
    // A colon first is part of the name: a child named `:1` is found by that name.
    let colon = child(&mut e, t.mcm, ":1");
    value(&mut e, colon, "_c", 4.0);
    assert_eq!(number(&mut e, "StartMenu/MCM/:1/_c"), 4.0);
    // The menu ids the DLL takes: up to 1001 + 0x53 (with the array that long).
    let array = e.mem.u32(MENU_ARRAY);
    let last = tile(&mut e, Some("LastMenu"));
    value(&mut e, last, "_m", 2.0);
    e.mem.set_u32(array + 4 * 0x53, last);
    assert_eq!(number(&mut e, "LastMenu/_m"), 2.0);
    assert_eq!(menu::value_at(&mut e, b"StartMenu/MCM/MCM_Options"), 0);
    assert_eq!(menu::value_at(&mut e, b"StartMenu/MCM/_none"), 0);
    assert_eq!(menu::value_at(&mut e, b"HUDMainMenu/MCM/_MCM"), 0);
    assert_eq!(menu::value_at(&mut e, b"StartMenu"), 0);
    assert_eq!(menu::value_at(&mut e, b"Nothing/MCM/_MCM"), 0);
    // A child without a name matches nothing (the DLL crashes there) and the walk goes on.
    let nameless = tile(&mut e, None);
    adopt(&mut e, t.mcm, nameless);
    let late = child(&mut e, t.mcm, "Late");
    value(&mut e, late, "_x", 9.0);
    assert_eq!(number(&mut e, "StartMenu/MCM/Late/_x"), 9.0);
    // The menu array's count bounds the id.
    e.mem.set_u16(MENU_COUNT, 12);
    assert_eq!(menu::value_at(&mut e, b"StartMenu/MCM/_MCM"), 0);
    e.mem.set_u16(MENU_COUNT, 0x54);
    // A path of 16 KB or more finds nothing.
    let mut long = b"StartMenu/MCM/_MCM".to_vec();
    long.resize(menu::PATH_BUFFER, b'/');
    assert_eq!(menu::value_at(&mut e, &long), 0);
    let _ = (t.option3, t.mod5, t.sub2, t.title5);
}

/// `GetMCMFloat`: the number, 0 for a missing trait; `SetMCMFloat`: set, 0 without a value,
/// nothing set when missing; every call answers true.
#[test]
fn get_and_set_mcm_float() {
    let mut e = engine();
    mcm_tree(&mut e);
    assert_eq!(
        call(&mut e, 0x21c2, &[int(0), int(0), s(b"_MCM")]),
        (1, 1.5)
    );
    assert_eq!(
        call(&mut e, 0x21c2, &[int(1), int(3), s(b"_value")]),
        (1, 3.0)
    );
    assert_eq!(
        call(&mut e, 0x21c2, &[int(5), int(0), s(b"_version")]),
        (1, 1.5)
    );
    assert_eq!(
        call(&mut e, 0x21c2, &[int(1), int(9), s(b"_value")]),
        (1, 0.0)
    );
    assert_eq!(
        call(&mut e, 0x21c3, &[int(0), int(0), s(b"visible"), float(1.0)]).0,
        1
    );
    assert_eq!(number(&mut e, "StartMenu/MCM/visible"), 1.0);
    call(&mut e, 0x21c3, &[int(0), int(0), s(b"visible")]);
    assert_eq!(number(&mut e, "StartMenu/MCM/visible"), 0.0);
    let calls = e.mem.u32(SET_CALLS);
    call(&mut e, 0x21c3, &[int(1), int(9), s(b"_value"), float(5.0)]);
    assert_eq!(e.mem.u32(SET_CALLS), calls, "a missing trait is not set");
    // The child and the grandchild in their places: group 1, item 3 (a swap would be group 3).
    call(&mut e, 0x21c3, &[int(1), int(3), s(b"_value"), float(8.0)]);
    assert_eq!(
        number(&mut e, "StartMenu/MCM/MCM_Options/Option3/_value"),
        8.0
    );
    assert_eq!(
        number(&mut e, "StartMenu/MCM/MCM_Options/Option1/_value"),
        1.0
    );
}

/// `SetMCMString`: the formatted text into the trait; `SetMCMStringMass` from max down to min.
#[test]
fn set_mcm_string_and_mass() {
    let mut e = engine();
    let t = mcm_tree(&mut e);
    let (ok, _) = call(
        &mut e,
        0x21c4,
        &[
            int(1),
            int(3),
            s(b"value/text/string"),
            s(br"MCM\Check1.dds"),
        ],
    );
    assert_eq!(ok, 1);
    assert_eq!(e.mem.cstr(LAST_STRING + 8), br"MCM\Check1.dds");
    let text = menu::component(&mut e, t.option3, b"value/text/string")
        .unwrap()
        .0;
    assert_eq!(e.mem.u32(LAST_STRING), text);
    assert_eq!(
        e.mem.u32(LAST_STRING + 4),
        id_of(b"string"),
        "the trait's id"
    );
    // Mass over the mod list: Mod5 (index 5) and SubMenu2 (index 25) have no `_x`; Title not in it.
    let calls = e.mem.u32(SET_CALLS);
    call(
        &mut e,
        0x21c6,
        &[int(5), int(4), int(6), s(b"text/string"), s(b"hi")],
    );
    assert_eq!(e.mem.u32(SET_CALLS), calls + 1, "only Mod5 has the trait");
    assert_eq!(e.mem.cstr(LAST_STRING + 8), b"hi");
}

/// `SetMCMFloatMass`: every index from max down to min; none when max < min; at most
/// `MASS_STEPS`.
#[test]
fn set_mcm_float_mass() {
    let mut e = engine();
    mcm_tree(&mut e);
    call(
        &mut e,
        0x21c5,
        &[int(1), int(1), int(4), s(b"_enable"), float(2.0)],
    );
    for i in 1..=4 {
        assert_eq!(
            number(
                &mut e,
                &format!("StartMenu/MCM/MCM_Options/Option{i}/_enable")
            ),
            2.0
        );
    }
    call(
        &mut e,
        0x21c5,
        &[int(1), int(4), int(1), s(b"_enable"), float(7.0)],
    );
    assert_eq!(
        number(&mut e, "StartMenu/MCM/MCM_Options/Option1/_enable"),
        2.0
    );
    // The child (group 1) differs from min: Option1 untouched, Options 2 to 4 set.
    call(
        &mut e,
        0x21c5,
        &[int(1), int(2), int(4), s(b"_value"), float(9.0)],
    );
    let value =
        |e: &mut Engine, i: u32| number(e, &format!("StartMenu/MCM/MCM_Options/Option{i}/_value"));
    assert_eq!(value(&mut e, 1), 1.0);
    for i in 2..=4 {
        assert_eq!(value(&mut e, i), 9.0, "Option{i}");
    }
    // No value: the traits become 0.
    call(&mut e, 0x21c5, &[int(1), int(1), int(2), s(b"_enable")]);
    for i in 1..=2 {
        assert_eq!(
            number(
                &mut e,
                &format!("StartMenu/MCM/MCM_Options/Option{i}/_enable")
            ),
            0.0
        );
    }
    // A range far longer than the tiles: every Option still reached.
    call(
        &mut e,
        0x21c5,
        &[int(1), int(1), int(100_001), s(b"_enable"), float(6.0)],
    );
    for i in 1..=4 {
        assert_eq!(
            number(
                &mut e,
                &format!("StartMenu/MCM/MCM_Options/Option{i}/_enable")
            ),
            6.0,
            "Option{i}"
        );
    }
    assert_eq!(
        mass_indices(&mut e, 1, 1, 4),
        vec![4, 3, 2, 1],
        "max down to min"
    );
    assert_eq!(mass_indices(&mut e, 1, 3, 3), vec![3]);
    assert!(mass_indices(&mut e, 1, 4, 1).is_empty());
    let steps = crate::commands::MASS_STEPS as i32;
    assert_eq!(mass_indices(&mut e, 1, 0, steps - 1).len(), steps as usize);
    // Longer: only the indices that can name a tile, from the top, at either end of the range.
    for (min, max) in [
        (i32::MIN, i32::MAX),
        (-100_000, 4),
        (0, 100_000),
        (1, steps + 1),
    ] {
        let want: Vec<i32> = [4, 3, 2, 1, 0]
            .into_iter()
            .filter(|i| (min..=max).contains(i))
            .collect();
        assert_eq!(mass_indices(&mut e, 1, min, max), want, "{min}..{max}");
    }
    // Group 5: `Mod5` and `SubMenu2` (index 25); a group without an index: once.
    assert_eq!(mass_indices(&mut e, 5, i32::MIN, i32::MAX), vec![25, 5, 0]);
    assert_eq!(mass_indices(&mut e, 2, i32::MIN, i32::MAX), vec![i32::MAX]);
    // An item number written otherwise names nothing (`Option03` is not `Option3`).
    let options = menu::value_at(&mut e, b"StartMenu/MCM/MCM_Options/_optionID");
    let options = e.mem.u32(options + 4);
    child(&mut e, options, "Option07");
    assert_eq!(
        mass_indices(&mut e, 1, i32::MIN, i32::MAX),
        vec![4, 3, 2, 1, 0]
    );
}

/// `SetMCMModList`: each number its low 16 bits, signed; the name only with a reference; a
/// reference's base form's full name, `<no name>` without one.
#[test]
fn set_mcm_mod_list() {
    let mut e = engine();
    let t = mcm_tree(&mut e);
    // DYNAMIC_CAST (`0x00EC43FB`) stood in for: a reference casts to `TESObjectREFR`, a cell to
    // `TESObjectCELL`; a full name at `+0x18` for a cell, `+0x30` for other forms with a name.
    e.register(0x00ec_43fb, |e, a| {
        let (form, to) = (a[0], a[3]);
        let kind = e.mem.u8(form + 4);
        let eax = match to {
            0x0118_41cc if (0x3a..=0x3c).contains(&kind) => form,
            0x0118_3fb4 if kind == 0x39 => form,
            0x0118_3158 if kind == 0x39 => form + 0x18,
            0x0118_3158 if e.mem.u32(form + 0x34) != 0 => form + 0x30,
            _ => 0,
        };
        Ret {
            eax,
            ..Ret::default()
        }
    });
    // The name parameter is type 0x15 (an object), which xNVSE's reader (the exe's `ExtractArgs`
    // as xNVSE replaces it, which the DLL calls) takes only for a bound object: the forms'
    // vtables answer `IsBoundObject` (`+0xE8`), a base object 1, a reference 0.
    const BOUND: u32 = 0x0060_0000;
    const NOT_BOUND: u32 = 0x0060_0004;
    e.register(BOUND, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e.register(NOT_BOUND, |_, _| Ret::default());
    let (vt_bound, vt_refr) = (take(&mut e, 0x200), take(&mut e, 0x200));
    e.mem.set_u32(vt_bound + 0xe8, BOUND);
    e.mem.set_u32(vt_refr + 0xe8, NOT_BOUND);
    let base = take(&mut e, 0x40);
    e.mem.set_u32(base, vt_bound);
    e.mem.set_u8(base + 4, 0x28);
    let name = take(&mut e, 16);
    e.mem.set_cstr(name, b"My Mod");
    e.mem.set_u32(base + 0x34, name);
    e.mem.set_u16(base + 0x38, 6);
    let refr = take(&mut e, 0x40);
    e.mem.set_u32(refr, vt_refr);
    e.mem.set_u8(refr + 4, 0x3a);
    e.mem.set_u32(refr + 0x20, base);
    // A reference is refused by the reader: the command sets nothing.
    let calls = e.mem.u32(SET_CALLS);
    let rr = r(&mut e, 1, refr);
    assert_eq!(
        call(&mut e, 0x21c7, &[int(5), int(1), int(2), int(3), rr]).0,
        1
    );
    assert_eq!(e.mem.u32(SET_CALLS), calls, "nothing set for a reference");
    // 65541 is 5 in 16 bits; enable 65537 is 1; index 65535 is -1.
    let rb = r(&mut e, 1, base);
    call(
        &mut e,
        0x21c7,
        &[int(65541), int(65537), int(65535), int(3), rb],
    );
    assert_eq!(
        number(&mut e, "StartMenu/MCM/MCM_ModList/Mod5/_enable"),
        1.0
    );
    assert_eq!(
        number(&mut e, "StartMenu/MCM/MCM_ModList/Mod5/_modIndex"),
        -1.0
    );
    assert_eq!(
        number(&mut e, "StartMenu/MCM/MCM_ModList/Mod5/_modIndex2"),
        3.0
    );
    assert_eq!(e.mem.cstr(LAST_STRING + 8), b"My Mod");
    assert_eq!(e.mem.u32(LAST_STRING), t.title5, "the title last");
    // Without a reference only `_enable`.
    let calls = e.mem.u32(SET_CALLS);
    call(&mut e, 0x21c7, &[int(5), int(0)]);
    assert_eq!(e.mem.u32(SET_CALLS), calls + 1);
    assert_eq!(
        number(&mut e, "StartMenu/MCM/MCM_ModList/Mod5/_enable"),
        0.0
    );
    // Names: no full name, an empty one, a cell without a name in a worldspace.
    let plain = take(&mut e, 0x40);
    e.mem.set_u8(plain + 4, 0x28);
    assert_eq!(full_name(&mut e, plain), b"<no name>");
    // A full name whose text is there but empty (length 0).
    let empty = take(&mut e, 0x40);
    e.mem.set_u8(empty + 4, 0x28);
    let etext = take(&mut e, 16);
    e.mem.set_cstr(etext, b"Stale");
    e.mem.set_u32(empty + 0x34, etext);
    assert_eq!(full_name(&mut e, empty), b"<no name>");
    assert_eq!(full_name(&mut e, refr), b"My Mod");
    let world = take(&mut e, 0x40);
    e.mem.set_u8(world + 4, 0x41);
    let wname = take(&mut e, 16);
    e.mem.set_cstr(wname, b"Mojave");
    e.mem.set_u32(world + 0x34, wname);
    e.mem.set_u16(world + 0x38, 6);
    let cell = take(&mut e, 0x100);
    e.mem.set_u8(cell + 4, 0x39);
    e.mem.set_u32(cell + 0xc0, world);
    assert_eq!(full_name(&mut e, cell), b"Mojave");
    let cname = take(&mut e, 16);
    e.mem.set_cstr(cname, b"Goodsprings");
    e.mem.set_u32(cell + 0x1c, cname);
    e.mem.set_u16(cell + 0x20, 11);
    assert_eq!(full_name(&mut e, cell), b"Goodsprings");
    let _ = (t.mod5, t.sub2);
}

/// `GetMCMListWidth`: 1 the mod's text width, 2 the sub-menu's, else 0; 16-bit numbers.
#[test]
fn get_mcm_list_width() {
    let mut e = engine();
    mcm_tree(&mut e);
    assert_eq!(call(&mut e, 0x21c8, &[int(1), int(5)]), (1, 123.0));
    assert_eq!(call(&mut e, 0x21c8, &[int(2), int(2)]), (1, 77.0));
    assert_eq!(
        call(&mut e, 0x21c8, &[int(0x10001), int(0x10005)]),
        (1, 123.0)
    );
    assert_eq!(call(&mut e, 0x21c8, &[int(3), int(5)]), (1, 0.0));
    assert_eq!(call(&mut e, 0x21c8, &[int(1), int(6)]), (1, 0.0));
}

/// The mod files the INI commands use, stood in for (this crate's test program has no platform
/// layer): files and folders by lower-case name, shared by the tests (each its own mod name).
static FILES: Mutex<BTreeMap<Vec<u8>, Vec<u8>>> = Mutex::new(BTreeMap::new());
static FOLDERS: Mutex<BTreeSet<Vec<u8>>> = Mutex::new(BTreeSet::new());
/// Every name the commands handed the mod files, as given (all the tests' calls).
static NAMES: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// The stand-in's key of a name (lower case, `\` for `/`); the name is recorded.
fn k(name: &[u8]) -> Vec<u8> {
    NAMES.lock().unwrap().push(name.to_vec());
    name.to_ascii_lowercase()
        .iter()
        .map(|&b| if b == b'/' { b'\\' } else { b })
        .collect()
}

static STAND_IN: ModFileCalls = ModFileCalls {
    read: |_, name| {
        FILES
            .lock()
            .unwrap()
            .get(&k(name))
            .cloned()
            .ok_or(mod_files::ERROR_FILE_NOT_FOUND)
    },
    exists: |_, name| FILES.lock().unwrap().contains_key(&k(name)),
    directory_exists: |_, name| {
        let name = k(name);
        let name = name.strip_suffix(b"\\").unwrap_or(&name).to_vec();
        FOLDERS.lock().unwrap().contains(&name)
    },
    create_directory: |_, name| {
        let name = k(name);
        let name = name.strip_suffix(b"\\").unwrap_or(&name).to_vec();
        if FOLDERS.lock().unwrap().insert(name) {
            Ok(())
        } else {
            Err(mod_files::ERROR_ALREADY_EXISTS)
        }
    },
    write: |_, name, bytes| {
        FILES.lock().unwrap().insert(k(name), bytes.to_vec());
        Ok(())
    },
};

fn ini_engine() -> Engine {
    mod_files::install(&STAND_IN);
    engine()
}

fn file(name: &str) -> Option<Vec<u8>> {
    FILES.lock().unwrap().get(&k(name.as_bytes())).cloned()
}

/// `GetModINI`: the value of `Data\Config\<mod>.ini`, `atof` of it cut to 31 bytes, as a float;
/// "0" when missing; 0 for a missing section or key, a part of 100 bytes, separators only.
#[test]
fn get_mod_ini() {
    let mut e = ini_engine();
    FILES.lock().unwrap().insert(
        k(br"Data\Config\GetTest.ini"),
        b"[List]\r\nMaxWidth=350\t;Max width of mod list.\r\n[Messages]\r\nInitial=1\r\nLong=1234567890123456789012345678901234\r\nF=0.1\r\n".to_vec(),
    );
    let get = |e: &mut Engine, text: &[u8]| call(e, 0x21c0, &[s(text)]);
    assert_eq!(get(&mut e, b"GetTest/List/MaxWidth"), (1, 350.0));
    assert_eq!(get(&mut e, br"GetTest\Messages:Initial"), (1, 1.0));
    assert_eq!(get(&mut e, b"//GetTest//Messages//Initial//"), (1, 1.0));
    assert_eq!(get(&mut e, b"GetTest/Messages/F"), (1, f64::from(0.1f32)));
    // 31 bytes of the value: 1234567890123456789012345678901.
    assert_eq!(
        get(&mut e, b"GetTest/Messages/Long"),
        (1, f64::from(1234567890123456789012345678901f64 as f32))
    );
    assert_eq!(get(&mut e, b"GetTest/Messages/Missing"), (1, 0.0));
    assert_eq!(get(&mut e, b"NoSuchMod/A/B"), (1, 0.0));
    assert_eq!(get(&mut e, b"GetTest/Messages"), (1, 0.0));
    assert_eq!(get(&mut e, b"GetTest"), (1, 0.0));
    assert_eq!(get(&mut e, b"///"), (1, 0.0));
    let long = [b"GetTest/".as_slice(), &[b'x'; 100], b"/Initial"].concat();
    assert_eq!(get(&mut e, &long), (1, 0.0));
    // 99 bytes fit the DLL's 100-byte buffers; 100 do not, even when the file has that section.
    let mut text = file(r"Data\Config\GetTest.ini").unwrap();
    for n in [99, 100] {
        text.extend_from_slice(&[b"[".as_slice(), &vec![b'y'; n], b"]\r\nK=7\r\n"].concat());
    }
    FILES
        .lock()
        .unwrap()
        .insert(k(br"Data\Config\GetTest.ini"), text);
    for (n, want) in [(99, 7.0), (100, 0.0)] {
        let path = [b"GetTest/".as_slice(), &vec![b'y'; n], b"/K"].concat();
        assert_eq!(get(&mut e, &path), (1, want), "{n} bytes");
    }
}

/// `SetModINI`: `Data\Config` made, a new file started as one backspace byte and written as
/// Windows writes (the backspace gone), the value as `%.4f`; an existing file kept in order.
#[test]
fn set_mod_ini() {
    let mut e = ini_engine();
    let set = |e: &mut Engine, text: &[u8], v: f64| call(e, 0x21c1, &[s(text), float(v)]).0;
    assert_eq!(set(&mut e, b"SetTest/Messages/Initial", 1.0), 1);
    assert!(FOLDERS.lock().unwrap().contains(&k(br"Data\Config")));
    assert_eq!(
        file(r"Data\Config\SetTest.ini").unwrap(),
        b"[Messages]\r\nInitial=1.0000\r\n"
    );
    set(&mut e, b"SetTest/List/MaxWidth", 350.0);
    set(&mut e, b"settest/messages/initial", 0.03125);
    assert_eq!(
        file(r"Data\Config\SetTest.ini").unwrap(),
        b"[Messages]\r\nInitial=0.0313\r\n[List]\r\nMaxWidth=350.0000\r\n"
    );
    // Read back through GetModINI.
    assert_eq!(
        call(&mut e, 0x21c0, &[s(b"SetTest/List/MaxWidth")]),
        (1, 350.0)
    );
    // A missing key writes nothing.
    set(&mut e, b"SetTest2/Messages", 1.0);
    assert_eq!(file(r"Data\Config\SetTest2.ini"), None);
}

/// Whatever a script writes, the INI commands hand the mod files only `Data\Config\` or
/// `Data\Config\<one part>.ini`: `/`, `\` and `:` cut the text into parts before a name is made,
/// so `..`, a drive, a stream or a deeper folder never reaches a name. A device's name
/// (`Data\Config\CON.ini`) is refused by the platform's mod files (`engine-platform`
/// `mod_files::relative`, its test `names_outside_the_game_folder_and_devices_are_refused` has
/// that very name).
#[test]
fn set_mod_ini_names_stay_in_data_config() {
    let mut e = ini_engine();
    for text in [
        &b"../../x/S/K"[..],
        b"..\\..\\x\\S\\K",
        b"C:/x/S/K",
        b"C:\\x\\S\\K",
        b"//server/share/S/K",
        b"CON/S/K",
        b"x:stream/S/K",
        b"Data/Config/Deep/S/K",
    ] {
        call(&mut e, 0x21c1, &[s(text), float(1.0)]);
        call(&mut e, 0x21c0, &[s(text)]);
    }
    let names = NAMES.lock().unwrap().clone();
    assert!(names.iter().any(|n| n == br"Data\Config\CON.ini"));
    assert!(names.iter().any(|n| n == br"Data\Config\...ini"));
    for name in &names {
        let rest = name
            .strip_prefix(br"Data\Config\")
            .unwrap_or_else(|| panic!("{:?}", String::from_utf8_lossy(name)));
        let one_part = rest.is_empty()
            || (rest.ends_with(b".ini") && !rest.iter().any(|&b| matches!(b, b'\\' | b'/' | b':')));
        assert!(one_part, "{:?}", String::from_utf8_lossy(name));
    }
}

/// `%.4f` as the VC++ 2010 runtime writes it (measured with `msvcr100.dll`, 2026-11-16).
#[test]
fn fixed4_matches_the_vc2010_runtime() {
    let measured: &[(u64, &str)] = &[
        (0f64.to_bits(), "0.0000"),
        ((-0f64).to_bits(), "-0.0000"),
        (1f64.to_bits(), "1.0000"),
        (350f64.to_bits(), "350.0000"),
        (0.03125f64.to_bits(), "0.0313"),
        (0.09375f64.to_bits(), "0.0938"),
        ((-0.03125f64).to_bits(), "-0.0313"),
        (2.15625f64.to_bits(), "2.1563"),
        (0.00005f64.to_bits(), "0.0001"),
        (0.12345f64.to_bits(), "0.1235"),
        (123456789f64.to_bits(), "123456789.0000"),
        (1e20f64.to_bits(), "100000000000000000000.0000"),
        (1e-5f64.to_bits(), "0.0000"),
        (2.675f64.to_bits(), "2.6750"),
        (1.00005f64.to_bits(), "1.0001"),
        (f64::from(1.23455f32).to_bits(), "1.2345"),
        (1234.56785f64.to_bits(), "1234.5678"),
        (9.99995f64.to_bits(), "10.0000"),
        (0.00015f64.to_bits(), "0.0001"),
        (
            3.4028234663852886e38f64.to_bits(),
            "340282346638528860000000000000000000000.0000",
        ),
        // The two-step rounding: below the tie exactly, above it at 17 digits.
        (0xbfff_264c_2f83_7b4a, "-1.9469"),
        (0x401e_f8fc_5048_16f0, "7.7432"),
        (0x3fef_be0d_ed28_8ce7, "0.9920"),
        (0x3f97_318f_c504_816f, "0.0227"),
        (0x7ff0_0000_0000_0000, "1.#INF"),
        (0xfff0_0000_0000_0000, "-1.#INF"),
        (0xfff8_0000_0000_0000, "-1.#IND"),
        (0x7ff8_0000_0000_0000, "1.#QNB"),
        (0xfff8_0000_0000_0001, "-1.#QNB"),
    ];
    for &(bits, want) in measured {
        assert_eq!(
            String::from_utf8(fixed4(f64::from_bits(bits))).unwrap(),
            want,
            "{bits:016x}"
        );
    }
}

/// `atof` as the VC++ 2010 runtime reads (measured with `msvcr100.dll`, 2026-11-16).
#[test]
fn atof_matches_the_vc2010_runtime() {
    let measured: &[(&[u8], f64)] = &[
        (b"350", 350.0),
        (b"350\t;Max width", 350.0),
        (b";Max width", 0.0),
        (b" 12.5x", 12.5),
        (b"1e3", 1000.0),
        (b"1e", 1.0),
        (b".5", 0.5),
        (b"-.5e1", -5.0),
        (b"+7", 7.0),
        (b"abc", 0.0),
        (b"", 0.0),
        (b"0x10", 0.0),
        (b"inf", 0.0),
        (b"nan", 0.0),
        (b"1.#INF", 1.0),
        (b"  -3", -3.0),
        (b"1d2", 100.0),
        (b"3.4028236e38", 3.4028236e38),
        (b"2.000000000000000111", 2.0),
        (b"1,5", 1.0),
    ];
    for &(text, want) in measured {
        assert_eq!(atof(text), want, "{:?}", String::from_utf8_lossy(text));
    }
    assert_eq!(atof(b"1e400"), f64::INFINITY);
}
