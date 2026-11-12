//! Tests of `Commands_UI.cpp` through the commands' records, on tiles laid out as the game's
//! (children in its doubly linked list, trait values sorted by id). The answers pinned are
//! xNVSE's as its source gives them; none of these commands has been run in the real game yet.

use super::super::test_support::*;
use super::*;
use std::cell::RefCell;

/// Where the fakes keep things: the menus' tile array, the interface manager, the tiles.
const ARRAY: u32 = 0x0200_0100;
const MANAGER: u32 = 0x0200_0400;
const TILES: u32 = 0x0200_1000;
/// The game's `toupper` (the case table).
const GAME_TOUPPER: u32 = 0x00ec_a7f4;

/// The trait ids the fake `TraitNameToID` knows.
const TRAITS: &[(&str, u32)] = &[
    ("x", 0x0fa1),
    ("y", 0x0fa2),
    ("locus", 0x0fa8),
    ("listindex", 0x0fb0),
    ("systemcolor", 0x0fc0),
    ("val", 0x1000),
    ("width", 0x1001),
    ("string", 0x1002),
    ("_y", 0x1003),
    ("height", 0x1004),
    ("num", 0x1005),
];

thread_local! {
    /// Calls of the two setters: (tile, trait id, number or text).
    static SET: RefCell<Vec<(u32, u32, String)>> = const { RefCell::new(Vec::new()) };
    static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static CALLED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    /// The tiles' values: (tile, [(id, value address)]).
    static NEXT: RefCell<u32> = const { RefCell::new(0) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn alloc(size: u32) -> u32 {
    NEXT.with(|n| {
        let mut n = n.borrow_mut();
        let at = TILES + *n;
        *n += size.next_multiple_of(16);
        at
    })
}

fn setup() -> Engine {
    SET.with(|s| s.borrow_mut().clear());
    LINES.with(|l| l.borrow_mut().clear());
    CALLED.with(|c| c.borrow_mut().clear());
    NEXT.with(|n| *n.borrow_mut() = 0);
    let mut e = engine_with_table();
    for page in [TILE_MENU_ARRAY, INTERFACE_MANAGER, VATS_CAMERA] {
        e.map(page & !0xfff, 0x1000);
    }
    e.mem.set_u32(TILE_MENU_ARRAY, ARRAY);
    e.mem.set_u32(INTERFACE_MANAGER, MANAGER);
    e.register(GAME_TOUPPER, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(TRAIT_NAME_TO_ID, |e, a| {
        let name = guest_text(e, a[0]);
        let id = TRAITS
            .iter()
            .find(|(n, _)| n.as_bytes() == name.as_slice())
            .map_or(0xffff, |&(_, id)| id);
        eax(id)
    });
    e.register(SET_FLOAT_VALUE, |e, a| {
        // The last argument makes the value stick on traits the menu computes.
        assert_eq!(a[3], 1, "SetFloatValue propagates");
        let num = f32::from_bits(a[2]);
        SET.with(|s| s.borrow_mut().push((a[0], a[1], format!("{num}"))));
        // The tile's value follows, as the game's setter makes it.
        let value = value_of(e, a[0], a[1]);
        if value != 0 {
            e.mem.set_f32(value + VALUE_NUM, num);
        }
        eax(0)
    });
    e.register(SET_STRING_VALUE, |e, a| {
        assert_eq!(a[3], 1, "SetStringValue propagates");
        let text = String::from_utf8(guest_text(e, a[2])).unwrap();
        SET.with(|s| s.borrow_mut().push((a[0], a[1], text)));
        eax(0)
    });
    e.register(0x0071_b160, |_, _| eax(0x0200_f000));
    e.register(0x0071_d0a0, |e, a| {
        let format = guest_text(e, a[1]);
        let list = e.mem.bytes(a[2], 16);
        let line = super::super::printf::ucrt_format(e, &format, &list);
        LINES.with(|l| l.borrow_mut().push(String::from_utf8(line).unwrap()));
        eax(0)
    });
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    e
}

fn set() -> Vec<(u32, u32, String)> {
    SET.with(|s| std::mem::take(&mut *s.borrow_mut()))
}

fn lines() -> Vec<String> {
    LINES.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

fn console_on(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

/// A tile named `name` under `parent` (appended to its children), with `values` (trait name,
/// number); `str_values` hold strings.
fn tile(
    e: &mut Engine,
    parent: u32,
    name: &str,
    values: &[(&str, f32)],
    str_values: &[(&str, &str)],
) -> u32 {
    let at = alloc(0x40);
    let text = alloc(name.len() as u32 + 1);
    e.mem.write(text, name.as_bytes());
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(at + NAME, text);
    e.mem.set_u16(at + NAME + 4, name.len() as u16);
    e.mem.set_u32(at + PARENT, parent);
    let mut all: Vec<(u32, u32)> = Vec::new();
    for &(trait_name, num) in values {
        let id = TRAITS.iter().find(|(n, _)| *n == trait_name).unwrap().1;
        let value = alloc(0x14);
        e.mem.set_u32(value, id);
        e.mem.set_u32(value + VALUE_TILE, at);
        e.mem.set_f32(value + VALUE_NUM, num);
        all.push((id, value));
    }
    for &(trait_name, s) in str_values {
        let id = TRAITS.iter().find(|(n, _)| *n == trait_name).unwrap().1;
        let value = alloc(0x14);
        let text = alloc(s.len() as u32 + 1);
        e.mem.write(text, s.as_bytes());
        e.mem.set_u8(text + s.len() as u32, 0);
        e.mem.set_u32(value, id);
        e.mem.set_u32(value + VALUE_TILE, at);
        e.mem.set_u32(value + VALUE_STR, text);
        all.push((id, value));
    }
    all.sort_unstable();
    let data = alloc(4 * all.len() as u32 + 4);
    for (i, &(_, value)) in all.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, value);
    }
    e.mem.set_u32(at + VALUES, data);
    e.mem.set_u32(at + VALUE_COUNT, all.len() as u32);
    if parent != 0 {
        let node = alloc(12);
        e.mem.set_u32(node + 8, at);
        let tail = e.mem.u32(parent + CHILDREN + 4);
        e.mem.set_u32(node + 4, tail);
        if tail == 0 {
            e.mem.set_u32(parent + CHILDREN, node);
        } else {
            e.mem.set_u32(tail, node);
        }
        e.mem.set_u32(parent + CHILDREN + 4, node);
    }
    at
}

/// A menu's tile, put in the menus' array at its id.
fn menu(e: &mut Engine, id: u32, name: &str, values: &[(&str, f32)]) -> u32 {
    let at = tile(e, 0, name, values, &[]);
    e.mem.set_u32(ARRAY + 4 * (id - MENU_MIN), at);
    at
}

/// `GetUIFloat` reads a trait by `Menu/tile/.../trait` (the menu's name without case): the
/// trait's number, -999 and "Trait not found" in console mode when it is not there, and a line
/// with the number in console mode.
#[test]
fn get_ui_float_reads_a_trait_by_path() {
    let mut e = setup();
    let hud = menu(&mut e, 0x3ec, "HUDMainMenu", &[]);
    let a = tile(&mut e, hud, "a", &[], &[]);
    tile(&mut e, a, "b", &[("val", 2.5)], &[]);
    let path = s("hudmainmenu/a/b/val");
    assert_eq!(call(&mut e, 0x14a4, 0, std::slice::from_ref(&path)).1, 2.5);
    assert_eq!(
        call(&mut e, 0x14a4, 0, &[s("HUDMainMenu/a/b/width")]).1,
        -999.0
    );
    assert_eq!(
        call(&mut e, 0x14a4, 0, &[s("HUDMainMenu/a/c/val")]).1,
        -999.0
    );
    assert_eq!(
        call(&mut e, 0x14a4, 0, &[s("NoSuchMenu/a/b/val")]).1,
        -999.0
    );
    assert_eq!(call(&mut e, 0x14a4, 0, &[s("HUDMainMenu")]).1, -999.0);
    console_on(&mut e);
    call(&mut e, 0x14a4, 0, &[path]);
    call(&mut e, 0x14a4, 0, &[s("HUDMainMenu/a/b/width")]);
    assert_eq!(lines(), ["GetUIFloat >> 2.5000", "Trait not found"]);
}

/// The commands without `Alt` find a tile's `n`-th child of a name in the order xNVSE's list type
/// walks the game's list (the first child, then the last back to the second); the `Alt` forms in
/// the children's order. `*` matches any child.
#[test]
fn the_old_lookup_walks_the_children_out_of_order() {
    let mut e = setup();
    let hud = menu(&mut e, 0x3ec, "HUDMainMenu", &[]);
    for v in [1.0, 2.0, 3.0, 4.0] {
        tile(&mut e, hud, "dup", &[("val", v)], &[]);
    }
    let get = |e: &mut Engine, opcode: u32, path: &str| call(e, opcode, 0, &[s(path)]).1;
    assert_eq!(get(&mut e, 0x14a4, "HUDMainMenu/dup/val"), 1.0);
    assert_eq!(get(&mut e, 0x14a4, "HUDMainMenu/dup:1/val"), 4.0);
    assert_eq!(get(&mut e, 0x14a4, "HUDMainMenu/dup:2/val"), 3.0);
    assert_eq!(get(&mut e, 0x14a4, "HUDMainMenu/dup:3/val"), 2.0);
    assert_eq!(get(&mut e, 0x14a4, "HUDMainMenu/*:1/val"), 4.0);
    assert_eq!(get(&mut e, 0x14a4, "HUDMainMenu/dup:4/val"), -999.0);
    assert_eq!(get(&mut e, 0x161f, "HUDMainMenu/dup:1/val"), 2.0);
    assert_eq!(get(&mut e, 0x161f, "HUDMainMenu/dup:3/val"), 4.0);
    assert_eq!(get(&mut e, 0x161f, "HUDMainMenu/*:2/val"), 3.0);
    // A name that starts with the colon names nothing.
    assert_eq!(get(&mut e, 0x161f, "HUDMainMenu/:1/val"), -999.0);
}

/// The setters: `SetUIFloat`, `SetUIString`, `SetUIStringEx` (a formatted string) on the trait's
/// tile, propagating; nothing without the trait. The `Alt` forms and `ModUIFloat` answer 1 when
/// they set it.
#[test]
fn the_setters_set_the_traits_tile() {
    let mut e = setup();
    let hud = menu(&mut e, 0x3ec, "HUDMainMenu", &[]);
    let a = tile(&mut e, hud, "a", &[("val", 1.5)], &[("string", "old")]);
    assert_eq!(
        call(&mut e, 0x14a5, 0, &[s("HUDMainMenu/a/val"), z(4.0)]),
        (1, 0.0)
    );
    call(&mut e, 0x14a6, 0, &[s("HUDMainMenu/a/string"), s("new")]);
    call(
        &mut e,
        0x14e2,
        0,
        &[s("HUDMainMenu/a/string"), s("%g pts"), z(5.0)],
    );
    // A format with more specifiers than the counted arguments sets nothing (the bytes after
    // them, here another number, are not read).
    call_counted(
        &mut e,
        0x14e2,
        3,
        &[s("HUDMainMenu/a/string"), s("%g and %g"), z(5.0), z(7.0)],
    );
    call(&mut e, 0x14a5, 0, &[s("HUDMainMenu/a/width"), z(4.0)]);
    assert_eq!(
        set(),
        [
            (a, 0x1000, "4".to_string()),
            (a, 0x1002, "new".to_string()),
            (a, 0x1002, "5 pts".to_string()),
        ]
    );
    assert_eq!(
        call(&mut e, 0x1620, 0, &[s("HUDMainMenu/a/val"), z(2.0)]),
        (1, 1.0)
    );
    assert_eq!(
        call(&mut e, 0x163a, 0, &[s("HUDMainMenu/a/val"), z(0.5)]),
        (1, 1.0)
    );
    assert_eq!(
        call(
            &mut e,
            0x1621,
            0,
            &[s("HUDMainMenu/a/string"), s("%g left"), z(3.0)]
        ),
        (1, 1.0)
    );
    assert_eq!(
        call(&mut e, 0x163a, 0, &[s("HUDMainMenu/a/width"), z(1.0)]),
        (1, 0.0)
    );
    assert_eq!(
        set(),
        [
            (a, 0x1000, "2".to_string()),
            (a, 0x1000, "2.5".to_string()),
            (a, 0x1002, "3 left".to_string()),
        ]
    );
}

/// `GetUIFloatInherited`: `x`, `y` and `depth` add up over the path's tiles that have `locus`
/// set (the menu's own left out with the second argument) and the last tile's own;
/// `systemcolor` is the nearest that has one; any other trait reads as `GetUIFloatAlt`; -999
/// for a path that is not there.
#[test]
fn inherited_traits_add_up_along_the_path() {
    let mut e = setup();
    let hud = menu(
        &mut e,
        0x3ec,
        "HUDMainMenu",
        &[("x", 10.0), ("locus", 1.0), ("systemcolor", 1.0)],
    );
    let p = tile(
        &mut e,
        hud,
        "p",
        &[("x", 5.0), ("locus", 1.0), ("systemcolor", 2.0)],
        &[],
    );
    let q = tile(&mut e, p, "q", &[("x", 100.0), ("locus", 0.0)], &[]);
    tile(&mut e, q, "c", &[("x", 2.0), ("width", 7.0)], &[]);
    let get =
        |e: &mut Engine, path: &str, exclude: i32| call(e, 0x168b, 0, &[s(path), n(exclude)]).1;
    assert_eq!(get(&mut e, "HUDMainMenu/p/q/c/x", 0), 17.0);
    assert_eq!(get(&mut e, "HUDMainMenu/p/q/c/x", 1), 7.0);
    assert_eq!(get(&mut e, "HUDMainMenu/p/q/c/systemcolor", 0), 2.0);
    assert_eq!(get(&mut e, "HUDMainMenu/p/q/c/width", 0), 7.0);
    assert_eq!(get(&mut e, "HUDMainMenu/p/r/c/x", 0), -999.0);
    assert_eq!(get(&mut e, "HUDMainMenu/p/q/c/", 0), -999.0);
}

/// `NormalizeItemName`: a count in brackets and the spaces before it go, then a final `+`.
#[test]
fn item_names_lose_counts_and_pluses() {
    for (from, to) in [
        ("9mm Pistol (5)", "9mm Pistol"),
        ("9mm Pistol+", "9mm Pistol"),
        ("Rifle   (12)", "Rifle"),
        ("Stimpak (10)+", "Stimpak (10)"),
        ("Note (a)", "Note (a)"),
        ("(5)", ""),
        ("a)", "a)"),
        ("+", "+"),
        ("", ""),
    ] {
        let mut s = from.as_bytes().to_vec();
        normalize_item_name(&mut s);
        assert_eq!(s, to.as_bytes(), "{from}");
    }
}

/// `SortUIListBox` sets each shown item's `_y` to the heights of the items before it in order:
/// by the keys of the spec (`@` normalized names, `-` reversed, `#` numeric), items whose keys
/// are equal in the order xNVSE met them (the first child, then the last back to the second:
/// MSVC's sort keeps their order below 33 items); hidden items (`listindex` under 0, or none)
/// are left alone.
#[test]
fn list_boxes_are_sorted_by_their_keys() {
    let mut e = setup();
    let inv = menu(&mut e, 0x3ea, "InventoryMenu", &[]);
    let list = tile(&mut e, inv, "list", &[], &[]);
    let item = |e: &mut Engine, name: &str, index: f32, height: f32| {
        tile(
            e,
            list,
            "item",
            &[("listindex", index), ("_y", -1.0), ("height", height)],
            &[("string", name)],
        )
    };
    let b1 = item(&mut e, "Beta (2)", 0.0, 10.0);
    let a = item(&mut e, "Alpha", 1.0, 20.0);
    let hidden = item(&mut e, "Aardvark", -1.0, 30.0);
    let b2 = item(&mut e, "Beta+", 2.0, 40.0);
    let c = item(&mut e, "Charlie", 3.0, 50.0);
    // A NaN `listindex` is shown (only one under 0 hides an item).
    let d = item(&mut e, "Delta", f32::NAN, 5.0);
    // Met as b1, d, c, b2, hidden (skipped), a.
    call(&mut e, 0x14d6, 0, &[s("InventoryMenu/list"), s("@string")]);
    let y = |e: &Engine, t: u32| value_num(e, value_of(e, t, 0x1003));
    assert_eq!(
        [
            y(&e, a),
            y(&e, b1),
            y(&e, b2),
            y(&e, c),
            y(&e, d),
            y(&e, hidden)
        ],
        [0.0, 20.0, 30.0, 70.0, 120.0, -1.0]
    );
    call(&mut e, 0x14d6, 0, &[s("InventoryMenu/list"), s("-@string")]);
    assert_eq!(
        [y(&e, d), y(&e, c), y(&e, b1), y(&e, b2), y(&e, a)],
        [0.0, 5.0, 55.0, 65.0, 105.0]
    );
    // A destination and increment of the spec's own.
    call(
        &mut e,
        0x14d6,
        0,
        &[s("InventoryMenu/list"), s("#height$x$height")],
    );
    let x: Vec<String> = set()
        .into_iter()
        .filter(|&(_, id, _)| id == 0x0fa1)
        .map(|(_, _, v)| v)
        .collect();
    assert!(x.is_empty(), "no item has an x trait: {x:?}");
}

/// `SortUIListBox` by numbers: a string trait read as a number with `#` ("9" before "25" before
/// "100", not as text), a float trait, a float trait reversed; positions written to the spec's
/// own destination with `_y` left alone. A key through a child (`a/num`) is looked up whole for
/// every item (xNVSE finds it only for the first; see the module's notes).
#[test]
fn list_boxes_sort_by_numbers_into_their_own_destination() {
    let mut e = setup();
    let inv = menu(&mut e, 0x3ea, "InventoryMenu", &[]);
    let list = tile(&mut e, inv, "list", &[], &[]);
    let item = |e: &mut Engine, text: &str, num: f32, height: f32| {
        let t = tile(
            e,
            list,
            "item",
            &[
                ("listindex", 0.0),
                ("_y", -1.0),
                ("width", -1.0),
                ("num", num),
                ("height", height),
            ],
            &[("string", text)],
        );
        tile(e, t, "a", &[("num", num)], &[]);
        t
    };
    let hundred = item(&mut e, "100", 2.0, 1.0);
    let twenty_five = item(&mut e, "25", 3.0, 10.0);
    let nine = item(&mut e, "9", 1.0, 100.0);
    let all = [hundred, twenty_five, nine];
    let read = |e: &Engine, id: u32| all.map(|t| value_num(e, value_of(e, t, id)));
    // By the strings as numbers: 9, 25, 100.
    call(
        &mut e,
        0x14d6,
        0,
        &[s("InventoryMenu/list"), s("#string$width")],
    );
    assert_eq!(read(&e, 0x1001), [110.0, 100.0, 0.0]);
    assert_eq!(read(&e, 0x1003), [-1.0, -1.0, -1.0]);
    // By the float trait: 1 (nine), 2 (hundred), 3 (twenty_five).
    call(
        &mut e,
        0x14d6,
        0,
        &[s("InventoryMenu/list"), s("num$width")],
    );
    assert_eq!(read(&e, 0x1001), [100.0, 101.0, 0.0]);
    // Reversed: 3, 2, 1.
    call(
        &mut e,
        0x14d6,
        0,
        &[s("InventoryMenu/list"), s("-num$width")],
    );
    assert_eq!(read(&e, 0x1001), [10.0, 0.0, 11.0]);
    // The same order through a child's trait, for every item.
    call(
        &mut e,
        0x14d6,
        0,
        &[s("InventoryMenu/list"), s("a/num$width")],
    );
    assert_eq!(read(&e, 0x1001), [100.0, 101.0, 0.0]);
    assert_eq!(read(&e, 0x1003), [-1.0, -1.0, -1.0]);
}

/// A known menu that is not open (its slot in the menus' array 0) is the most common missing
/// tile: the readers answer -999 (`GetUIFloat` says "Trait not found" in console mode, as
/// `SetUIFloat` does, the others print nothing), the setters set nothing (the `Alt` ones answer
/// 0), and `SortUIListBox` sets nothing.
#[test]
fn a_closed_menu_has_no_traits() {
    let mut e = setup();
    console_on(&mut e);
    assert_eq!(call(&mut e, 0x14a4, 0, &[s("StartMenu/x/y")]).1, -999.0);
    assert_eq!(
        call(&mut e, 0x14a5, 0, &[s("StartMenu/x/y"), z(1.0)]),
        (1, 0.0)
    );
    assert_eq!(lines(), ["Trait not found", "Trait not found"]);
    assert_eq!(call(&mut e, 0x161f, 0, &[s("StartMenu/x/y")]).1, -999.0);
    assert_eq!(
        call(&mut e, 0x168b, 0, &[s("StartMenu/x/y"), n(0)]).1,
        -999.0
    );
    assert!(lines().is_empty());
    assert_eq!(
        call(&mut e, 0x1620, 0, &[s("StartMenu/x/y"), z(1.0)]),
        (1, 0.0)
    );
    assert_eq!(
        call(&mut e, 0x1621, 0, &[s("StartMenu/x/y"), s("text")]),
        (1, 0.0)
    );
    call(&mut e, 0x14d6, 0, &[s("StartMenu/x"), s("num")]);
    assert!(set().is_empty());
}

/// Paths written with backslashes, and a child named in another case, find the same tiles.
#[test]
fn backslashes_and_case_in_paths() {
    let mut e = setup();
    let hud = menu(&mut e, 0x3ec, "HUDMainMenu", &[("x", 1.0), ("locus", 1.0)]);
    let a = tile(&mut e, hud, "a", &[], &[]);
    tile(&mut e, a, "b", &[("val", 2.5), ("x", 3.0)], &[]);
    for v in [1.0, 2.0] {
        tile(&mut e, hud, "dup", &[("val", v)], &[]);
    }
    assert_eq!(
        call(&mut e, 0x14a4, 0, &[s("HUDMainMenu\\a\\b\\val")]).1,
        2.5
    );
    assert_eq!(
        call(&mut e, 0x161f, 0, &[s("HUDMainMenu\\DUP:1\\val")]).1,
        2.0
    );
    assert_eq!(
        call(&mut e, 0x168b, 0, &[s("HUDMainMenu\\a\\b\\x"), n(0)]).1,
        4.0
    );
}

/// `std::sort` as MSVC's library sorts: what is ordered ends in order, and equal elements keep
/// their order below 33 (insertion sort); over 32 the partitions move them as MSVC's do.
#[test]
fn the_sort_is_msvcs() {
    let mut v: Vec<u32> = (0..200).map(|i| (i * 7919) % 211).collect();
    msvc_sort(&mut v, &|a: &u32, b: &u32| a < b);
    assert!(v.windows(2).all(|w| w[0] <= w[1]));
    // Below 33, ties keep their order.
    let mut pairs: Vec<(u32, u32)> = (0..32).map(|i| (i % 3, i)).collect();
    msvc_sort(&mut pairs, &|a: &(u32, u32), b: &(u32, u32)| a.0 < b.0);
    let expected: Vec<(u32, u32)> = (0..3)
        .flat_map(|k| (0..32).filter(move |i| i % 3 == k).map(move |i| (k, i)))
        .collect();
    assert_eq!(pairs, expected);
    // Heap sort when the partitions go bad (a depth of 0 from the start).
    let mut w: Vec<u32> = (0..100).rev().collect();
    let len = w.len();
    sort_unchecked(&mut w, 0, len, 0, &|a: &u32, b: &u32| a < b);
    assert_eq!(w, (0..100).collect::<Vec<u32>>());
}

/// `PrintActiveTile` prints the highlighted tile's full name, its parents first with a backslash
/// each; without one a line saying so.
#[test]
fn print_active_tile_names_the_tile() {
    let mut e = setup();
    let hud = menu(&mut e, 0x3ec, "HUDMainMenu", &[]);
    let a = tile(&mut e, hud, "a", &[], &[]);
    let b = tile(&mut e, a, "b", &[], &[]);
    call(&mut e, 0x163e, 0, &[]);
    e.mem.set_u32(MANAGER + ACTIVE_TILE, b);
    call(&mut e, 0x163e, 0, &[]);
    assert_eq!(lines(), ["Could not read active tile", "HUDMainMenu\\a\\b"]);
}

/// `EndVATScam` takes the VATS targets back one by one, then accepts in the limb or zoom mode;
/// `ShowLevelUpMenu` calls the game's.
#[test]
fn vats_ends_and_the_level_up_menu_shows() {
    let mut e = setup();
    e.mem.set_u32(VATS_CAMERA, 2);
    e.register(VATS_UNCLICK, |e, a| {
        assert_eq!(a[0], VATS_CAMERA);
        CALLED.with(|c| c.borrow_mut().push(VATS_UNCLICK));
        let left = e.mem.u32(VATS_CAMERA) - 1;
        e.mem.set_u32(VATS_CAMERA, left);
        eax(0)
    });
    e.register(VATS_ACCEPT, |_, _| {
        CALLED.with(|c| c.borrow_mut().push(VATS_ACCEPT));
        eax(0)
    });
    e.register(SHOW_LEVEL_UP_MENU, |_, _| {
        CALLED.with(|c| c.borrow_mut().push(SHOW_LEVEL_UP_MENU));
        eax(0)
    });
    e.mem.set_u32(VATS_CAMERA + VATS_MODE, 1);
    call(&mut e, 0x15b5, 0, &[]);
    e.mem.set_u32(VATS_CAMERA + VATS_MODE, 2);
    call(&mut e, 0x15b5, 0, &[]);
    call(&mut e, 0x161e, 0, &[]);
    assert_eq!(
        CALLED.with(|c| c.borrow().clone()),
        [VATS_UNCLICK, VATS_UNCLICK, VATS_ACCEPT, SHOW_LEVEL_UP_MENU]
    );
}

/// MSVC's own `std::sort` (Visual Studio 2022's `<algorithm>`, built for x86 from
/// `measure/xnvse-rest/sortcases.cpp`) on index lists with many ties (`sort_cases.txt`: the
/// count, the key's modulus and multiplier, MSVC's order): the same orders here.
#[test]
fn the_sort_matches_msvcs_own() {
    for line in include_str!("sort_cases.txt")
        .lines()
        .filter(|l| !l.is_empty())
    {
        let (head, order) = line.split_once('|').unwrap();
        let p: Vec<usize> = head.split(' ').map(|x| x.parse().unwrap()).collect();
        let (n, m, mul) = (p[0], p[1], p[2]);
        let keys: Vec<usize> = (0..n).map(|i| (i * mul) % m).collect();
        let mut v: Vec<usize> = (0..n).collect();
        msvc_sort(&mut v, &|&a: &usize, &b: &usize| keys[a] < keys[b]);
        let want: Vec<usize> = order
            .split(',')
            .map(|x| x.trim().parse().unwrap())
            .collect();
        assert_eq!(v, want, "{head}");
    }
}

/// [`call`] with the arguments' count word set to `count`, whatever follows the counted
/// arguments left in the data (as the next bytes of a compiled script are).
fn call_counted(e: &mut Engine, opcode: u32, count: u16, args: &[Vec<u8>]) -> (u32, f64) {
    let bytes: Vec<u8> = args.concat();
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16 + 2);
    e.mem.set_u16(DATA + 4, count);
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = crate::table::record_of(e, opcode);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    let ok = e
        .call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET])
        .eax;
    (ok & 0xff, e.mem.f64(RESULT))
}
