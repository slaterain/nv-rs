//! Tests of the interface commands, at raw offsets: a tile's children (head +4, a node next +0,
//! tile +8), values (data +0x14, count +0x18; a value id +0, number +8, string +0xC), name
//! (+0x20, length +0x24), parent +0x28; a menu tile's vtable `0106ED44` and menu +0x3C; a menu's
//! id +0x20 and click handler at vtable +0xC; the interface manager's mode +0xC, cursor +0x38 and
//! +0x40, active tiles +0xBC and +0xCC, active menu +0xD0, menu stack +0x114; the menus'
//! singletons and fields as the module lists them. The game functions are stand-ins that record.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data;

const IM: u32 = FORMS + 0x1_0000;
const TILES: u32 = FORMS + 0x2_0000;
const NAMES: u32 = FORMS + 0x3_0000;
const VALUES_AT: u32 = FORMS + 0x3_8000;
const MENUS: u32 = FORMS + 0x4_0000;
const MENU_VT: u32 = FORMS + 0x4_f000;
const MAP_VT: u32 = FORMS + 0x5_0000;
const MAP_BUCKETS: u32 = FORMS + 0x5_0100;
const MAP_ITEMS: u32 = FORMS + 0x5_0200;
const MAP_KEYS: u32 = FORMS + 0x5_1000;
const MENU_ARRAY: u32 = FORMS + 0x6_0000;
const OBJ: u32 = FORMS + 0x7_0000;
const ALLOC: u32 = FORMS + 0x8_0000;
const PC: u32 = FORMS + 0x9_0000;
const TILE_VT: u32 = FORMS + 0x9_8000;

const HASH_FN: u32 = 0x0dea_0010;
const EQUAL_FN: u32 = 0x0dea_0020;
const CLICK_FN: u32 = 0x0dea_0030;
const DESTROY_FN: u32 = 0x0dea_0040;
const REFRESH_FN: u32 = 0x00a0_9410;
const FADE_ADD_FN: u32 = 0x00a0_7c60;
const FADE_REMOVE_FN: u32 = 0x00a0_7dc0;
const READ_FILE_FN: u32 = 0x00a0_1b00;
const RENDER_FN: u32 = 0x00a1_ddb0;
const BARTER_REFRESH_FN: u32 = 0x0073_0690;
const LEVEL_UP_CREATE_FN: u32 = 0x0078_4c80;
const LEVEL_UP_PAGE_FN: u32 = 0x0078_5830;
const PURGE_FN: u32 = 0x007f_fe00;
const FORM_ALLOC_FN: u32 = 0x0040_1000;
const REFR_INIT_FN: u32 = 0x0055_a2f0;

/// The trait ids the stand-in map knows.
const TRAITS: [(&[u8], u32); 4] = [
    (b"id", 0xfaa),
    (b"target", 0xfaf),
    (b"clicked", 0xfc7),
    (b"string", 0xfb4),
];

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// `tile(n)`'s address.
fn t(n: u32) -> u32 {
    TILES + 0x100 * n
}

/// Tile `n` named `name` under `parent` (appended to its children).
fn tile(e: &mut Engine, n: u32, parent: u32, name: &[u8]) -> u32 {
    let at = t(n);
    let text = NAMES + 0x40 * n;
    e.mem.write(text, name);
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(at, TILE_VT);
    e.mem.set_u32(at + 0x20, text);
    e.mem.set_u16(at + 0x24, name.len() as u16);
    e.mem.set_u32(at + 0x28, parent);
    if parent != 0 {
        let node = at + 0x80;
        let tail = e.mem.u32(parent + 8);
        e.mem.set_u32(node, 0);
        e.mem.set_u32(node + 4, tail);
        e.mem.set_u32(node + 8, at);
        if tail != 0 {
            e.mem.set_u32(tail, node);
        } else {
            e.mem.set_u32(parent + 4, node);
        }
        e.mem.set_u32(parent + 8, node);
    }
    at
}

/// Values for `tile`, sorted by id: (id, number, string).
fn values(e: &mut Engine, tile: u32, list: &[(u32, f32, Option<&[u8]>)]) -> Vec<u32> {
    let n = (tile - TILES) / 0x100;
    let data = VALUES_AT + 0x100 * n;
    let mut out = Vec::new();
    for (i, (id, num, text)) in list.iter().enumerate() {
        let v = data + 0x40 + 0x20 * i as u32;
        e.mem.set_u32(v, *id);
        e.mem.set_u32(v + 4, tile);
        e.mem.set_f32(v + 8, *num);
        let s = match text {
            Some(text) => {
                let at = v + 0x14;
                e.mem.write(at, text);
                e.mem.set_u8(at + text.len() as u32, 0);
                at
            }
            None => 0,
        };
        e.mem.set_u32(v + 0xc, s);
        e.mem.set_u32(data + 4 * i as u32, v);
        out.push(v);
    }
    e.mem.set_u32(tile + 0x14, data);
    e.mem.set_u32(tile + 0x18, list.len() as u32);
    out
}

/// Menu `id`'s tile `n` (a menu tile) with its menu object.
fn menu(e: &mut Engine, n: u32, id: u32, name: &[u8]) -> (u32, u32) {
    let mt = tile(e, n, t(0), name);
    e.mem.set_u32(mt, 0x0106_ed44);
    let m = MENUS + 0x200 * n;
    e.mem.set_u32(m, MENU_VT);
    e.mem.set_u32(m + 4, mt);
    e.mem.set_u32(m + 0x20, id);
    e.mem.set_u32(mt + 0x3c, m);
    e.mem.set_u32(MENU_ARRAY + 4 * (id - 0x3e9), mt);
    (mt, m)
}

/// The engine: the interface manager, the trait map, the menu array, the player and the
/// stand-ins.
fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011d_8000, 0x7000);
    e.map(0x011f_3000, 0x1000);
    e.map(0x011a_0000, 0x1000);
    e.mem.set_u32(0x011d_8a80, IM);
    e.mem.set_u32(0x011f_350c, MENU_ARRAY);
    e.mem.set_u32(0x011d_ea3c, PC);
    e.mem.set_u32(MENU_VT + 0xc, CLICK_FN);
    e.mem.set_u32(TILE_VT, DESTROY_FN);
    // The trait map: four buckets by the first byte.
    e.mem.set_u32(0x011f_32f4, MAP_VT);
    e.mem.set_u32(0x011f_32f4 + 4, 4);
    e.mem.set_u32(0x011f_32f4 + 8, MAP_BUCKETS);
    e.mem.set_u32(MAP_VT + 4, HASH_FN);
    e.mem.set_u32(MAP_VT + 8, EQUAL_FN);
    for (i, (name, id)) in TRAITS.iter().enumerate() {
        let item = MAP_ITEMS + 0x10 * i as u32;
        let key = MAP_KEYS + 0x20 * i as u32;
        e.mem.write(key, name);
        let bucket = MAP_BUCKETS + 4 * u32::from(name[0] % 4);
        e.mem.set_u32(item, e.mem.u32(bucket));
        e.mem.set_u32(item + 4, key);
        e.mem.set_u32(item + 8, *id);
        e.mem.set_u32(bucket, item);
    }
    e.register(HASH_FN, |e, w| {
        assert_eq!(w[0], 0x011f_32f4);
        eax(u32::from(e.mem.u8(w[1]) % 4))
    });
    e.register(EQUAL_FN, |e, w| {
        let (a, b) = (
            nvse::table::guest_text(e, w[1]),
            nvse::table::guest_text(e, w[2]),
        );
        eax(u32::from(a == b))
    });
    macro_rules! recorder {
        ($($f:expr),*) => {$(
            e.register($f, |_, w| {
                record($f, w);
                eax(0)
            });
        )*};
    }
    recorder!(
        CLICK_FN,
        DESTROY_FN,
        REFRESH_FN,
        FADE_ADD_FN,
        FADE_REMOVE_FN,
        BARTER_REFRESH_FN,
        LEVEL_UP_PAGE_FN,
        PURGE_FN
    );
    e.register(READ_FILE_FN, |e, w| {
        record(READ_FILE_FN, w);
        let path = nvse::table::guest_text(e, w[1]);
        eax(if path == b"good.xml" { t(60) } else { 0 })
    });
    e.register(RENDER_FN, |e, w| {
        record(RENDER_FN, w);
        let name = nvse::table::guest_text(e, w[2]);
        eax(if name == b"Template" { t(61) } else { 0 })
    });
    e.register(LEVEL_UP_CREATE_FN, |e, w| {
        record(LEVEL_UP_CREATE_FN, w);
        e.mem.set_u32(0x011d_9fdc, MENUS + 0x3000);
        eax(0)
    });
    e.mem.set_u32(ALLOC, ALLOC + 0x10);
    e.register(FORM_ALLOC_FN, |e, w| {
        let at = e.mem.u32(ALLOC);
        e.mem.set_u32(ALLOC, at + w[0].next_multiple_of(0x10));
        eax(at)
    });
    e.register(REFR_INIT_FN, |e, w| {
        let n = e.mem.u32(ALLOC + 4) + 1;
        e.mem.set_u32(ALLOC + 4, n);
        e.mem.set_u32(w[0] + 0xc, 0xff00_0000 + n);
        eax(w[0])
    });
    extra_stand_ins(&mut e);
    tile(&mut e, 0, 0, b"root");
    e
}

fn ids() -> Vec<u32> {
    calls().into_iter().map(|c| c.0).collect()
}

/// A plain tree: InventoryMenu (tile 1) > Box (2) > Item (3), Item (4); Box > Item:1 is tile 4.
fn tree(e: &mut Engine) -> (u32, u32) {
    let (mt, m) = menu(e, 1, 0x3ea, b"InventoryMenu");
    tile(e, 2, mt, b"Box");
    tile(e, 3, t(2), b"Item");
    tile(e, 4, t(2), b"item");
    (mt, m)
}

fn in_menu_mode(e: &mut Engine) {
    e.mem.set_u32(IM + 0xc, 2);
}

// ---- paths ----

#[test]
fn paths_find_tiles_by_name_index_and_wildcard() {
    let mut e = setup();
    tree(&mut e);
    let loaded = |e: &mut Engine, p: &[u8]| call(e, 0x2608, &[s(p)]).1;
    assert_eq!(loaded(&mut e, b"InventoryMenu"), 1.0, "the menu's own tile");
    assert_eq!(loaded(&mut e, b"inventorymenu/box/item"), 1.0, "no case");
    assert_eq!(loaded(&mut e, b"InventoryMenu\\Box"), 1.0, "a backslash");
    assert_eq!(loaded(&mut e, b"StatsMenu"), 0.0, "no tile for the menu");
    assert_eq!(loaded(&mut e, b"NoMenu/Box"), 0.0);
    assert_eq!(loaded(&mut e, b"InventoryMenu/Nope/Item"), 0.0);
    assert_eq!(
        loaded(&mut e, b"InventoryMenu/Box/id"),
        0.0,
        "a trait, not a tile"
    );
    assert_eq!(
        loaded(&mut e, b"InventoryMenu/Box/:1"),
        0.0,
        "a name starting with the colon"
    );
    // Which tile: through UnloadUIComponent, which destroys the one found.
    let destroyed = |e: &mut Engine, p: &[u8]| {
        CALLS.with(|c| c.borrow_mut().clear());
        call(e, 0x286c, &[s(p)]);
        calls_to(DESTROY_FN)
    };
    assert_eq!(
        destroyed(&mut e, b"InventoryMenu/Box/Item"),
        vec![vec![t(3), 1]]
    );
    assert_eq!(
        destroyed(&mut e, b"InventoryMenu/Box/Item:1"),
        vec![vec![t(4), 1]]
    );
    assert_eq!(
        destroyed(&mut e, b"InventoryMenu/Box/*:1"),
        vec![vec![t(4), 1]]
    );
    assert_eq!(
        destroyed(&mut e, b"InventoryMenu/Box/Item:2"),
        Vec::<Vec<u32>>::new()
    );
    assert_eq!(
        destroyed(&mut e, b"InventoryMenu/Box/x"),
        Vec::<Vec<u32>>::new()
    );
    // A child without a name buffer matches nothing (but `*` takes it).
    e.mem.set_u32(t(3) + 0x20, 0);
    assert_eq!(
        destroyed(&mut e, b"InventoryMenu/Box/Item"),
        vec![vec![t(4), 1]]
    );
    assert_eq!(
        destroyed(&mut e, b"InventoryMenu/Box/*"),
        vec![vec![t(3), 1]]
    );
    // No menu array: nothing.
    e.mem.set_u32(0x011f_350c, 0);
    assert_eq!(loaded(&mut e, b"InventoryMenu"), 0.0);
}

#[test]
fn ui_string_reads_a_trait_through_the_map() {
    let mut e = setup();
    tree(&mut e);
    values(
        &mut e,
        t(3),
        &[
            (0xfaa, 7.0, None),
            (0xfb4, 0.0, Some(b"Hello")),
            (0xfc7, 0.0, None),
        ],
    );
    let get = |e: &mut Engine, p: &[u8]| {
        let res = call(e, 0x2639, &[s(p)]).1;
        string_result(e, res)
    };
    assert_eq!(get(&mut e, b"InventoryMenu/Box/Item/string"), b"Hello");
    assert_eq!(get(&mut e, b"InventoryMenu/Box/Item/id"), b"", "no string");
    assert_eq!(
        get(&mut e, b"InventoryMenu/Box/Item/target"),
        b"",
        "no such value"
    );
    assert_eq!(
        get(&mut e, b"InventoryMenu/Box/Item/unknown"),
        b"",
        "no such trait"
    );
    assert_eq!(
        get(&mut e, b"InventoryMenu/Box/Item"),
        b"",
        "a tile, not a trait"
    );
    assert_eq!(get(&mut e, b"InventoryMenu"), b"", "a menu: no value");
    // A path ending at a trait the tile has is no tile.
    assert_eq!(
        call(&mut e, 0x2608, &[s(b"InventoryMenu/Box/Item/string")]).1,
        0.0
    );
}

#[test]
fn inject_xml_reads_the_file_into_the_tile() {
    let mut e = setup();
    tree(&mut e);
    assert_eq!(
        call(&mut e, 0x2609, &[s(b"InventoryMenu/Box"), s(b"good.xml")]).1,
        1.0
    );
    assert_eq!(calls_to(READ_FILE_FN)[0][0], t(2));
    assert_eq!(
        call(&mut e, 0x2609, &[s(b"InventoryMenu/Box"), s(b"bad.xml")]).1,
        0.0
    );
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(
        call(&mut e, 0x2609, &[s(b"InventoryMenu/No"), s(b"good.xml")]).1,
        0.0
    );
    assert!(calls_to(READ_FILE_FN).is_empty());
}

#[test]
fn click_menu_tile_pokes_clicked_when_targetable() {
    let mut e = setup();
    tree(&mut e);
    let v = values(&mut e, t(3), &[(0xfaf, 1.0, None), (0xfc7, 0.0, None)]);
    let path = s(b"InventoryMenu/Box/Item");
    assert_eq!(
        call(&mut e, 0x28a4, std::slice::from_ref(&path)).1,
        0.0,
        "not in menu mode"
    );
    assert!(calls().is_empty());
    in_menu_mode(&mut e);
    assert_eq!(call(&mut e, 0x28a4, std::slice::from_ref(&path)).1, 1.0);
    assert_eq!(
        calls_to(REFRESH_FN),
        vec![vec![v[1], 1], vec![v[1], 1]],
        "true then false"
    );
    assert_eq!(e.mem.u32(v[1] + 8), 0);
    // Target 0 (or none): no click.
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_f32(v[0] + 8, 0.0);
    assert_eq!(call(&mut e, 0x28a4, std::slice::from_ref(&path)).1, 0.0);
    e.mem.set_f32(v[0] + 8, f32::NAN);
    assert_eq!(call(&mut e, 0x28a4, std::slice::from_ref(&path)).1, 0.0);
    assert!(calls().is_empty());
    // A target but no clicked value: 1, nothing poked.
    e.mem.set_f32(v[0] + 8, 2.0);
    values(&mut e, t(3), &[(0xfaf, 2.0, None)]);
    assert_eq!(call(&mut e, 0x28a4, &[path]).1, 1.0);
    assert!(calls().is_empty());
}

#[test]
fn gradual_floats_by_argument_count() {
    let mut e = setup();
    tree(&mut e);
    let v = values(&mut e, t(3), &[(0xfaa, 3.0, None)]);
    let p = || s(b"InventoryMenu/Box/Item/id");
    call(&mut e, 0x28cb, &[p(), z(1.0), z(5.0), z(2.0), n(3)]);
    assert_eq!(
        calls_to(FADE_ADD_FN),
        vec![vec![
            t(3),
            0xfaa,
            1.0f32.to_bits(),
            5.0f32.to_bits(),
            2.0f32.to_bits(),
            5
        ]]
    );
    for (mode, want) in [(0, 0), (1, 4), (2, 1), (4, 6), (5, 0), (-1, 0)] {
        CALLS.with(|c| c.borrow_mut().clear());
        call(&mut e, 0x28cb, &[p(), z(1.0), z(5.0), z(2.0), n(mode)]);
        assert_eq!(calls_to(FADE_ADD_FN)[0][5], want, "mode {mode}");
    }
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28cb, &[p(), z(1.0), z(5.0), z(2.0)]);
    assert_eq!(calls_to(FADE_ADD_FN)[0][5], 0, "four arguments: mode 0");
    // Fewer: the fade ends, and with a start value the value is set (refreshed).
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28cb, &[p(), z(9.0)]);
    assert_eq!(ids(), vec![FADE_REMOVE_FN, REFRESH_FN]);
    assert_eq!(e.mem.f32(v[0] + 8), 9.0);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28cb, &[p(), z(9.0), z(1.0)]);
    assert_eq!(ids(), vec![FADE_REMOVE_FN], "the same value: no refresh");
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28cb, &[p(), z(f64::NAN)]);
    assert_eq!(ids(), vec![FADE_REMOVE_FN], "a NaN: nothing set");
    assert_eq!(e.mem.f32(v[0] + 8), 9.0);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28cb, &[p()]);
    assert_eq!(calls_to(FADE_REMOVE_FN), vec![vec![t(3), 0xfaa]]);
    assert_eq!(ids(), vec![FADE_REMOVE_FN]);
    // A path without a value: nothing.
    CALLS.with(|c| c.borrow_mut().clear());
    call(
        &mut e,
        0x28cb,
        &[s(b"InventoryMenu"), z(1.0), z(2.0), z(3.0)],
    );
    call(
        &mut e,
        0x28cb,
        &[s(b"InventoryMenu/Box/Item/target"), z(1.0)],
    );
    assert!(calls().is_empty());
}

#[test]
fn templates_add_a_named_tile() {
    let mut e = setup();
    let (_, m) = tree(&mut e);
    let made = tile(&mut e, 61, 0, b"");
    string(&mut e, made + 0x20, None);
    let add = |e: &mut Engine, text: &[u8]| call(e, 0x28c6, &[s(text)]).1;
    assert_eq!(add(&mut e, b"InventoryMenu/Box|Template|NewName"), 1.0);
    assert_eq!(calls_to(RENDER_FN)[0][..2], [m, t(2)]);
    assert_eq!(calls_to(RENDER_FN)[0][3], 0);
    assert_eq!(text_at(&e, made + 0x20), b"NewName");
    // The menu's own tile, `||` as one delimiter, no new name.
    CALLS.with(|c| c.borrow_mut().clear());
    string(&mut e, made + 0x20, Some(b"Keep"));
    assert_eq!(add(&mut e, b"InventoryMenu||Template"), 1.0);
    assert_eq!(calls_to(RENDER_FN)[0][1], t(1));
    assert_eq!(text_at(&e, made + 0x20), b"Keep");
    // The name is the rest, `|`s and all.
    assert_eq!(add(&mut e, b"InventoryMenu|Template|a|b"), 1.0);
    assert_eq!(text_at(&e, made + 0x20), b"a|b");
    // No template, a trait path, an unknown menu or tile, a template the menu lacks.
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(add(&mut e, b"InventoryMenu/Box"), 0.0);
    assert_eq!(add(&mut e, b"InventoryMenu/Box|"), 0.0);
    assert_eq!(add(&mut e, b"InventoryMenu/Box/id|Template"), 0.0);
    assert_eq!(add(&mut e, b"NoMenu/Box|Template"), 0.0);
    assert_eq!(add(&mut e, b"NoMenu|Template"), 0.0);
    assert_eq!(add(&mut e, b"InventoryMenu/No/x|Template"), 0.0);
    assert!(calls_to(RENDER_FN).is_empty());
    assert_eq!(add(&mut e, b"InventoryMenu|Other"), 0.0);
    // A menu tile without its menu: nothing.
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(t(1) + 0x3c, 0);
    assert_eq!(add(&mut e, b"InventoryMenu|Template"), 0.0);
    assert!(calls_to(RENDER_FN).is_empty());
}

#[test]
fn menu_buttons_click_through_the_vtable() {
    let mut e = setup();
    let (_, m) = tree(&mut e);
    values(&mut e, t(3), &[(0xfaa, 12.9, None)]);
    let click = |e: &mut Engine, args: &[Vec<u8>]| {
        CALLS.with(|c| c.borrow_mut().clear());
        let res = call(e, 0x2687, args).1;
        (res, calls_to(CLICK_FN))
    };
    assert_eq!(
        click(&mut e, &[s(b"InventoryMenu/Box/Item")]),
        (0.0, vec![])
    );
    in_menu_mode(&mut e);
    assert_eq!(
        click(&mut e, &[s(b"InventoryMenu/Box/Item")]),
        (1.0, vec![vec![m, 12, t(3)]])
    );
    assert_eq!(
        click(&mut e, &[s(b"InventoryMenu/Box/Item:1"), n(2)]),
        (1.0, vec![vec![m, u32::MAX, t(4)]; 2]),
        "no id: -1"
    );
    assert_eq!(
        click(&mut e, &[s(b"InventoryMenu/Box/Item"), n(0)]),
        (0.0, vec![])
    );
    assert_eq!(
        click(&mut e, &[s(b"InventoryMenu#5")]),
        (1.0, vec![vec![m, 5, 0]])
    );
    assert_eq!(
        click(&mut e, &[s(b"InventoryMenu#0")]),
        (1.0, vec![vec![m, 0, 0]]),
        "tile id 0"
    );
    assert_eq!(click(&mut e, &[s(b"InventoryMenu#-1")]), (0.0, vec![]));
    assert_eq!(click(&mut e, &[s(b"InventoryMenu/Box#5")]), (0.0, vec![]));
    assert_eq!(click(&mut e, &[s(b"Nope/Box")]), (0.0, vec![]));
    // A tile under no menu tile.
    let lone = tile(&mut e, 9, 0, b"Lone");
    e.mem.set_u32(t(2) + 0x28, lone);
    e.mem.set_u32(t(3) + 0x28, t(2));
    // (the path still goes through the menu's children)
    assert_eq!(
        click(&mut e, &[s(b"InventoryMenu/Box/Item")]),
        (0.0, vec![])
    );
}

/// The click handler as a menu's own that closes the menu at the third click: the menu's
/// `g_tileMenuArray` entry cleared (`by_tile`), else its shown byte (as `Menu::~Menu` does both).
fn closing_click(e: &mut Engine, by_tile: bool) {
    if by_tile {
        e.register(CLICK_FN, |e, w| {
            record(CLICK_FN, w);
            if CALLS.with(|c| c.borrow().len()) == 3 {
                e.mem.set_u32(MENU_ARRAY + 4, 0);
            }
            eax(0)
        });
    } else {
        e.register(CLICK_FN, |e, w| {
            record(CLICK_FN, w);
            if CALLS.with(|c| c.borrow().len()) == 3 {
                e.mem.set_u8(0x011f_308f + 0x3ea, 0);
            }
            eax(0)
        });
    }
}

#[test]
fn menu_button_clicks_with_the_size_caps_row() {
    let count = |e: &mut Engine, times: i32| {
        CALLS.with(|c| c.borrow_mut().clear());
        let res = call(e, 0x2687, &[s(b"InventoryMenu#1"), n(times)]).1;
        (res, calls_to(CLICK_FN).len())
    };
    let mut e = setup();
    tree(&mut e);
    in_menu_mode(&mut e);
    e.mem.set_u8(0x011f_308f + 0x3ea, 1);
    // Vanilla (the row off): every click, as JIP, past the row's cap and through a closed menu.
    assert_eq!(count(&mut e, 70_000), (1.0, 70_000));
    closing_click(&mut e, true);
    assert_eq!(
        count(&mut e, 10),
        (1.0, 10),
        "vanilla clicks on after the close"
    );
    // The row on: -1 (4,294,967,295) and 2^31 stop at the cap.
    let mut e = setup();
    tree(&mut e);
    in_menu_mode(&mut e);
    e.mem.set_u8(0x011f_308f + 0x3ea, 1);
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::SIZE_CAPS, true);
    assert_eq!(count(&mut e, -1), (1.0, 65_536));
    assert_eq!(count(&mut e, i32::MIN), (1.0, 65_536));
    assert_eq!(count(&mut e, 65_535), (1.0, 65_535));
    assert_eq!(count(&mut e, 2), (1.0, 2));
    // ... and once the menu is closed: its tile entry gone, or its shown byte cleared.
    closing_click(&mut e, true);
    assert_eq!(count(&mut e, -1), (1.0, 3));
    e.mem.set_u32(MENU_ARRAY + 4, t(1));
    closing_click(&mut e, false);
    assert_eq!(count(&mut e, 10), (1.0, 3));
    // Another menu tile in the entry is not this menu's.
    e.mem.set_u8(0x011f_308f + 0x3ea, 1);
    e.register(CLICK_FN, |e, w| {
        record(CLICK_FN, w);
        e.mem.set_u32(MENU_ARRAY + 4, t(7));
        eax(0)
    });
    assert_eq!(count(&mut e, 10), (1.0, 1));
}

/// The tree with the list guard row on (the walks of lists that run back into themselves).
fn guarded_tree() -> Engine {
    let mut e = setup();
    tree(&mut e);
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::LIST_GUARD, true);
    e
}

#[test]
fn child_list_running_back_ends() {
    let mut e = guarded_tree();
    // Box's last child node points back to its first: a name it lacks is not found.
    e.mem.set_u32(t(4) + 0x80, t(3) + 0x80);
    assert_eq!(
        call(&mut e, 0x2608, &[s(b"InventoryMenu/Box/Nope/x")]).1,
        0.0
    );
    assert_eq!(
        call(&mut e, 0x2608, &[s(b"InventoryMenu/Box/Item")]).1,
        1.0,
        "found before the loop"
    );
}

#[test]
fn trait_bucket_running_back_ends() {
    let mut e = guarded_tree();
    values(&mut e, t(3), &[(0xfb4, 0.0, Some(b"Hello"))]);
    // A map bucket whose item points to itself ("target" is bucket 0's only item; 'x' hashes there).
    e.mem.set_u32(MAP_ITEMS + 0x10, MAP_ITEMS + 0x10);
    let res = call(&mut e, 0x2639, &[s(b"InventoryMenu/Box/Item/xq")]).1;
    assert_eq!(string_result(&e, res), b"");
    let res = call(&mut e, 0x2639, &[s(b"InventoryMenu/Box/Item/string")]).1;
    assert_eq!(string_result(&e, res), b"Hello", "another bucket");
}

#[test]
fn parent_chain_running_back_ends() {
    let mut e = guarded_tree();
    // A parent chain that runs in a circle without a menu tile: no menu, no click.
    in_menu_mode(&mut e);
    e.mem.set_u32(t(2) + 0x28, t(3));
    e.mem.set_u32(t(3) + 0x28, t(2));
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(call(&mut e, 0x2687, &[s(b"InventoryMenu/Box/Item")]).1, 0.0);
    assert!(calls_to(CLICK_FN).is_empty());
}

#[test]
fn parent_menu_found_within_65536_steps() {
    // Item's parents: a chain of plain tiles with a menu tile at the end. The walk looks at
    // Item and 65,535 parents: a menu tile as the 65,535th parent is found, as the 65,536th not.
    const CHAIN: u32 = 0x0500_0000;
    let at = |i: u32| CHAIN + 0x40 * i;
    let mut e = setup();
    let (_, m) = tree(&mut e);
    e.map(CHAIN, 0x41_0000);
    in_menu_mode(&mut e);
    for i in 0..65_536 {
        e.mem.set_u32(at(i), TILE_VT);
        e.mem.set_u32(at(i) + 0x28, at(i + 1));
    }
    e.mem.set_u32(t(3) + 0x28, at(0));
    let click = |e: &mut Engine| {
        CALLS.with(|c| c.borrow_mut().clear());
        let res = call(e, 0x2687, &[s(b"InventoryMenu/Box/Item")]).1;
        (res, calls_to(CLICK_FN).len())
    };
    for (last, found) in [(65_534u32, true), (65_535, false)] {
        e.mem.set_u32(at(last), 0x0106_ed44);
        e.mem.set_u32(at(last) + 0x3c, m);
        let want = if found { (1.0, 1) } else { (0.0, 0) };
        assert_eq!(click(&mut e), want, "menu tile as parent {}", last + 1);
        e.mem.set_u32(at(last), TILE_VT);
    }
}

#[test]
fn trait_compare_reads_its_low_byte() {
    let mut e = setup();
    tree(&mut e);
    e.register(EQUAL_FN, |e, w| {
        let (a, b) = (
            nvse::table::guest_text(e, w[1]),
            nvse::table::guest_text(e, w[2]),
        );
        eax(if a == b { 0x1234_5601 } else { 0xffff_ff00 })
    });
    values(
        &mut e,
        t(3),
        &[(0xfb4, 0.0, Some(b"S")), (0xfc7, 0.0, Some(b"C"))],
    );
    // Bucket 3 holds "string" (first) and "clicked": "clicked" is compared with "string" first.
    let get = |e: &mut Engine, p: &[u8]| {
        let res = call(e, 0x2639, &[s(p)]).1;
        string_result(e, res)
    };
    assert_eq!(get(&mut e, b"InventoryMenu/Box/Item/clicked"), b"C");
    assert_eq!(get(&mut e, b"InventoryMenu/Box/Item/string"), b"S");
}

/// `$name` as the script's string variable holding `text`: the script's variable list (`+0x4C`:
/// an info with its id +0 and name +0x18) and the event list's (`+0xC`: a variable with its id +0
/// and value +8).
fn dollar(e: &mut Engine, text: &[u8]) -> Vec<u8> {
    use crate::test_support::{LIST, SCRIPT};
    let id = nvse::vars::add_string(e, 0, text, false);
    let (info, name, node, local) = (0x0300_3000, 0x0300_3040, 0x0300_3080, 0x0300_30c0);
    e.mem.write(name, b"big\0");
    e.mem.set_u32(info, 1);
    e.mem.set_u32(info + 0x18, name);
    e.mem.set_u32(SCRIPT + 0x4c, info);
    e.mem.set_u32(SCRIPT + 0x50, 0);
    e.mem.set_u32(local, 1);
    e.mem.set_f64(local + 8, f64::from(id));
    e.mem.set_u32(node, local);
    e.mem.set_u32(node + 4, 0);
    e.mem.set_u32(LIST + 0xc, node);
    s(b"$big")
}

/// A text over 1 MiB (more than the guest stack holds).
fn big() -> Vec<u8> {
    vec![b'x'; (1 << 20) + 16]
}

#[test]
fn trait_name_over_a_mebibyte_goes_cut_to_jips_buffer() {
    let mut e = guarded_tree();
    // A trait name: the map's hash and compare get 255 bytes.
    e.register(HASH_FN, |e, w| {
        record(HASH_FN, &[nvse::table::guest_text(e, w[1]).len() as u32]);
        eax(0)
    });
    let mut path = b"InventoryMenu/Box/Item/".to_vec();
    path.extend_from_slice(&big());
    let arg = dollar(&mut e, &path);
    let res = call(&mut e, 0x2639, &[arg]).1;
    assert_eq!(string_result(&e, res), b"");
    assert_eq!(calls_to(HASH_FN), vec![vec![0xff]]);
}

#[test]
fn xml_name_over_a_mebibyte_goes_cut_to_jips_buffer() {
    let mut e = guarded_tree();
    // An XML file name: 127 bytes.
    e.register(READ_FILE_FN, |e, w| {
        record(
            READ_FILE_FN,
            &[nvse::table::guest_text(e, w[1]).len() as u32],
        );
        eax(0)
    });
    let arg = dollar(&mut e, &big());
    assert_eq!(call(&mut e, 0x2609, &[s(b"InventoryMenu/Box"), arg]).1, 0.0);
    assert_eq!(calls_to(READ_FILE_FN), vec![vec![0x7f]]);
}

#[test]
fn template_name_over_a_mebibyte_goes_cut_to_jips_buffer() {
    let mut e = guarded_tree();
    // A template's name: 255 bytes.
    e.register(RENDER_FN, |e, w| {
        record(RENDER_FN, &[nvse::table::guest_text(e, w[2]).len() as u32]);
        eax(0)
    });
    let id = nvse::vars::add_string(&mut e, 0, &big(), false);
    assert_eq!(
        call(&mut e, 0x28c6, &[s(b"InventoryMenu|%z"), z(f64::from(id))]).1,
        0.0
    );
    assert_eq!(calls_to(RENDER_FN), vec![vec![0xff]]);
}

// ---- the interface manager ----

#[test]
fn cursor_and_top_menu() {
    let mut e = setup();
    e.mem.set_f32(IM + 0x38, 640.5);
    e.mem.set_f32(IM + 0x40, 360.25);
    assert_eq!(call(&mut e, 0x2638, &[vec![b'X']]).1, 640.5);
    assert_eq!(call(&mut e, 0x2638, &[vec![b'Y']]).1, 360.25);
    assert_eq!(call(&mut e, 0x2638, &[vec![b'Z']]).1, 360.25, "not X: y");
    let top = |e: &mut Engine| call(e, 0x2681, &[]).1;
    assert_eq!(top(&mut e), 0.0, "game mode");
    e.mem.set_u32(IM + 0xc, 0x0102);
    e.mem.set_u32(IM + 0x114, 0x3f0);
    e.mem.set_u32(IM + 0x118, 0x41d);
    assert_eq!(
        top(&mut e),
        f64::from(0x41d),
        "the mode's low byte; the stack's top"
    );
    e.mem.set_u32(IM + 0x118, 0);
    assert_eq!(top(&mut e), f64::from(0x3f0));
    e.mem.set_u32(MENUS + 0x20, 0x3eb);
    e.mem.set_u32(IM + 0xd0, MENUS);
    assert_eq!(top(&mut e), f64::from(0x3eb), "the menu under the pointer");
    e.mem.set_u32(IM + 0xd0, 0);
    // The Pip-Boy: the shown one of its menus, in JIP's order.
    e.mem.set_u32(IM + 0x114, 1);
    assert_eq!(top(&mut e), 0.0);
    for (id, want) in [
        (0x425u32, 0x425u32),
        (0x40b, 0x40b),
        (0x3eb, 0x3eb),
        (0x3ff, 0x3ff),
        (0x3ea, 0x3ea),
    ] {
        e.mem.set_u8(0x011f_308f + id, 1);
        assert_eq!(top(&mut e), f64::from(want), "{id:x}");
    }
    e.mem.set_u32(IM + 0xc, 0x0201);
    assert_eq!(top(&mut e), 0.0, "the mode's low byte below 2");
    e.mem.set_u32(IM + 0xc, 1);
    assert_eq!(top(&mut e), 0.0);
}

#[test]
fn active_tile_name_path_and_id() {
    let mut e = setup();
    tree(&mut e);
    let name = |e: &mut Engine| {
        let r = call(e, 0x2682, &[]).1;
        string_result(e, r)
    };
    let full = |e: &mut Engine| {
        let r = call(e, 0x2683, &[]).1;
        string_result(e, r)
    };
    let id = |e: &mut Engine| call(e, 0x2684, &[]).1;
    assert_eq!(
        (name(&mut e), full(&mut e), id(&mut e)),
        (vec![], vec![], -1.0)
    );
    e.mem.set_u32(IM + 0xbc, t(4));
    assert_eq!(full(&mut e), b"InventoryMenu/Box/item");
    assert_eq!(name(&mut e), b"item");
    assert_eq!(id(&mut e), -1.0, "no id value");
    values(&mut e, t(3), &[(0xfaa, -3.7, None)]);
    e.mem.set_u32(IM + 0xcc, t(3));
    assert_eq!(full(&mut e), b"InventoryMenu/Box/Item", "activeTile first");
    assert_eq!(id(&mut e), -3.0, "truncated");
    values(&mut e, t(3), &[(0xfaa, f32::NAN, None)]);
    assert_eq!(id(&mut e), f64::from(i32::MIN));
    e.mem.set_u32(IM + 0xcc, t(1));
    assert_eq!(full(&mut e), b"InventoryMenu");
    assert_eq!(name(&mut e), b"", "no slash");
    // A tile not under a menu tile: nothing (JIP runs past its array).
    let lone = tile(&mut e, 9, 0, b"Lone");
    e.mem.set_u32(IM + 0xcc, lone);
    assert_eq!(full(&mut e), b"");
}

#[test]
fn menu_stack_full_reads_on_inside_the_manager() {
    let mut e = setup();
    e.mem.set_u32(IM + 0xc, 2);
    // All ten entries set, the word after them 0: the tenth.
    for i in 0..10 {
        e.mem.set_u32(IM + 0x114 + 4 * i, 0x3f0 + i);
    }
    assert_eq!(call(&mut e, 0x2681, &[]).1, f64::from(0x3f9));
    // Every word after them set too (and past the manager): the last word inside it.
    for at in (0x13c..0x700).step_by(4) {
        e.mem.set_u32(IM + at, 0x1_0000 + at);
    }
    assert_eq!(call(&mut e, 0x2681, &[]).1, f64::from(0x1_0578));
}

/// A chain of `count` tiles named `L`, each under the one before, under the menu tile.
fn deep_chain(e: &mut Engine, count: u32) -> Vec<u32> {
    let (mut parent, mut out) = (t(1), Vec::new());
    for i in 0..count {
        parent = tile(e, 100 + i, parent, b"L");
        out.push(parent);
    }
    out
}

#[test]
fn active_tile_path_within_jips_24_entries() {
    let mut e = setup();
    menu(&mut e, 1, 0x3ea, b"InventoryMenu");
    let chain = deep_chain(&mut e, 24);
    let full = |e: &mut Engine| {
        let r = call(e, 0x2683, &[]).1;
        string_result(e, r)
    };
    let path = |levels: usize| {
        let mut p = b"InventoryMenu".to_vec();
        for _ in 0..levels {
            p.extend_from_slice(b"/L");
        }
        p
    };
    // JIP's `parents[24]` holds the tile and 23 above it: 22 and 23 levels under the menu tile
    // give the path, 24 run past the array (empty here).
    for (levels, want) in [(22, path(22)), (23, path(23)), (24, Vec::new())] {
        e.mem.set_u32(IM + 0xcc, chain[levels - 1]);
        assert_eq!(full(&mut e), want, "{levels} levels");
    }
    e.mem.set_u32(IM + 0xcc, chain[22]);
    let r = call(&mut e, 0x2682, &[]).1;
    assert_eq!(string_result(&e, r), b"L");
}

#[test]
fn active_tile_missing_from_its_parents_children() {
    let mut e = setup();
    tree(&mut e);
    let full = |e: &mut Engine| {
        let r = call(e, 0x2683, &[]).1;
        string_result(e, r)
    };
    let name = |e: &mut Engine| {
        let r = call(e, 0x2682, &[]).1;
        string_result(e, r)
    };
    // Box's child by its parent word only: JIP reads through the list's end; empty here.
    let ghost = tile(&mut e, 9, 0, b"Ghost");
    e.mem.set_u32(ghost + 0x28, t(2));
    e.mem.set_u32(IM + 0xcc, ghost);
    assert_eq!((full(&mut e), name(&mut e)), (vec![], vec![]));
    // The list's back links run in a circle without it (the list guard row ends the walk).
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::LIST_GUARD, true);
    e.mem.set_u32(t(3) + 0x80 + 4, t(4) + 0x80);
    assert_eq!(full(&mut e), b"");
    // Found, but its earlier siblings of the same name run in a circle: empty too.
    e.mem.write(NAMES + 0x40 * 3, b"item");
    e.mem.set_u32(IM + 0xcc, t(4));
    assert_eq!(full(&mut e), b"");
    // Without the circle: the sibling counted.
    e.mem.set_u32(t(3) + 0x80 + 4, 0);
    assert_eq!(full(&mut e), b"InventoryMenu/Box/item:1");
}

#[test]
fn char_gen_menus() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x282a, &[]).1, 0.0);
    let record = nvse::table::record_of(&e, 0x282a);
    let eval = e.mem.u32(record + 0x20);
    for (n, id) in [(1, 0x418), (2, 0x424), (3, 0x432), (4, 0x43c)] {
        let mut e = setup();
        menu(&mut e, n, id, b"x");
        assert_eq!(call(&mut e, 0x282a, &[]).1, 1.0, "{id:x}");
        e.mem.set_f64(RESULT, 0.0);
        e.call(eval, &[0, 0, 0, RESULT]);
        assert_eq!(e.mem.f64(RESULT), 1.0);
    }
    menu(&mut e, 5, 0x3ea, b"x");
    e.mem.set_f64(RESULT, 0.0);
    e.call(eval, &[0, 0, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 0.0);
}

// ---- messages ----

#[test]
fn messages_disabled_flags_and_time() {
    let mut e = setup();
    let msg = form(&mut e, OBJ, 0x62, 0x0000_1001);
    let msg2 = form(&mut e, OBJ + 0x100, 0x62, 0x0000_1002);
    let other = form(&mut e, OBJ + 0x200, 0x1f, 0x0000_1003);
    let list = form(&mut e, OBJ + 0x300, 0x55, 0x0000_1004);
    e.mem.set_u32(list + 0x18, msg2);
    e.mem.set_u32(list + 0x1c, OBJ + 0x400);
    e.mem.set_u32(OBJ + 0x400, other);
    e.mem.set_u32(OBJ + 0x404, 0);
    for (i, f) in [msg, msg2, other, list].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    assert_eq!(e.hook_users(Hook::MessageDisabled), 0);
    call(&mut e, 0x2784, &[r(1), n(1)]);
    assert_eq!(e.mem.u8(msg + 6), 1);
    assert_eq!(e.hook_users(Hook::MessageDisabled), 1);
    assert_eq!(call(&mut e, 0x2785, &[r(1)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2785, &[r(2)]).1, 0.0);
    call(&mut e, 0x2784, &[r(4), n(5)]);
    assert_eq!(
        (e.mem.u8(msg2 + 6), e.mem.u8(other + 6)),
        (1, 0),
        "a list: its messages"
    );
    call(&mut e, 0x2784, &[r(3), n(1)]);
    assert_eq!(e.mem.u8(other + 6), 0, "not a message");
    e.mem.set_u8(msg + 6, 0x81);
    call(&mut e, 0x2784, &[r(1), n(0)]);
    assert_eq!(e.mem.u8(msg + 6), 0x80, "only its bit");
    assert_eq!(call(&mut e, 0x2785, &[r(1)]).1, 0.0);
    // Flags and the display time.
    e.mem.set_u32(msg + 0x38, 0xaabb_cc02);
    assert_eq!(call(&mut e, 0x2787, &[r(1)]).1, 2.0, "the low byte");
    call(&mut e, 0x2788, &[r(1), n(3)]);
    assert_eq!(e.mem.u32(msg + 0x38), 0xaabb_cc03);
    call(&mut e, 0x2788, &[r(1), n(4)]);
    call(&mut e, 0x2788, &[r(1), n(-1)]);
    assert_eq!(e.mem.u32(msg + 0x38), 0xaabb_cc03, "above 3: nothing");
    call(&mut e, 0x278f, &[r(1), n(-7)]);
    assert_eq!(e.mem.u32(msg + 0x3c), -7i32 as u32);
}

// ---- the menus' data ----

#[test]
fn item_filters() {
    let mut e = setup();
    let filter = |e: &mut Engine, args: &[Vec<u8>]| call(e, 0x2686, args).1;
    assert_eq!(filter(&mut e, &[n(0x3ea)]), -1.0, "no menu");
    assert_eq!(filter(&mut e, &[n(0x3f0)]), -1.0);
    assert_eq!(filter(&mut e, &[n(0x41d)]), -1.0);
    let (inv, cont, barter) = (MENUS, MENUS + 0x200, MENUS + 0x400);
    e.mem.set_u32(0x011d_9ea4, inv);
    e.mem.set_u32(0x011d_93f8, cont);
    e.mem.set_u32(0x011d_8fa4, barter);
    e.mem.set_u32(inv + 0x84, 3);
    e.mem.set_u32(cont + 0x8c, 4);
    e.mem.set_u32(cont + 0x90, 5);
    e.mem.set_u32(barter + 0x9c, 6);
    e.mem.set_u32(barter + 0xa0, 7);
    assert_eq!(filter(&mut e, &[n(0x3ea)]), 4.0, "plus 1");
    assert_eq!(filter(&mut e, &[n(0x3f0)]), 4.0);
    assert_eq!(filter(&mut e, &[n(0x3f0), n(1)]), 5.0);
    assert_eq!(filter(&mut e, &[n(0x41d)]), 6.0);
    assert_eq!(filter(&mut e, &[n(0x41d), n(2)]), 7.0);
    assert_eq!(filter(&mut e, &[n(0x3eb)]), -1.0, "another menu");
    e.mem.set_u32(inv + 0x84, u32::MAX);
    assert_eq!(filter(&mut e, &[n(0x3ea)]), 0.0);
}

#[test]
fn barter_caps_and_prices() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2711, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x27fa, &[n(1)]).1, 0.0);
    call(&mut e, 0x27fb, &[n(1), z(2.0)]);
    assert!(calls().is_empty(), "no menu");
    let barter = MENUS;
    e.mem.set_u32(0x011d_8fa4, barter);
    e.mem.set_u32(barter + 0x90, -5i32 as u32);
    assert_eq!(call(&mut e, 0x2711, &[]).1, -5.0);
    e.mem.set_f32(barter + 0x94, 1.5);
    e.mem.set_f32(barter + 0x98, 0.25);
    assert_eq!(call(&mut e, 0x27fa, &[n(0)]).1, 1.5);
    assert_eq!(call(&mut e, 0x27fa, &[n(9)]).1, 0.25);
    call(&mut e, 0x27fb, &[n(1), z(0.75)]);
    assert_eq!(
        (e.mem.f32(barter + 0x94), e.mem.f32(barter + 0x98)),
        (1.5, 0.75)
    );
    call(&mut e, 0x27fb, &[n(0), z(3.0)]);
    assert_eq!(e.mem.f32(barter + 0x94), 3.0);
    assert_eq!(calls_to(BARTER_REFRESH_FN), vec![vec![1], vec![1]]);
}

#[test]
fn recipe_menu() {
    let mut e = setup();
    let recipe = form(&mut e, OBJ, 0x6a, 0x0000_2001);
    let category = form(&mut e, OBJ + 0x100, 0x6b, 0x0000_2002);
    e.mem.set_u32(0x011d_8e94, recipe);
    assert_eq!(call(&mut e, 0x2712, &[]).0, 1);
    assert_eq!(form_result(&e), 0, "no menu");
    let m = MENUS;
    e.mem.set_u32(0x011d_8e90, m);
    call(&mut e, 0x2712, &[]);
    assert_eq!(form_result(&e), 0x2001);
    e.mem.set_u32(0x011d_8e94, 0);
    call(&mut e, 0x2712, &[]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x2713, &[]);
    assert_eq!(form_result(&e), 0, "no category");
    e.mem.set_u32(m + 0x64, category);
    call(&mut e, 0x2713, &[]);
    assert_eq!(form_result(&e), 0x2002);
    e.mem.set_u32(0x011d_8f08 + 8, 9);
    assert_eq!(call(&mut e, 0x2715, &[]).1, 9.0);
}

#[test]
fn font_files() {
    let mut e = setup();
    let get = |e: &mut Engine, id: i32| {
        let r = call(e, 0x274b, &[n(id)]).1;
        string_result(e, r)
    };
    assert_eq!(get(&mut e, 1), b"", "no manager");
    let manager = OBJ;
    e.mem.set_u32(0x011f_33f8, manager);
    for (id, at) in [(1u32, 0u32), (8, 0x1c), (10, 0x24), (89, 0x160)] {
        let font = OBJ + 0x1000 + 0x100 * id;
        let path = font + 0x80;
        let text = format!("Fonts\\f{id}.fnt");
        e.mem.write(path, text.as_bytes());
        e.mem.set_u32(font + 4, path);
        e.mem.set_u32(manager + at, font);
        assert_eq!(get(&mut e, id as i32), text.as_bytes());
    }
    e.mem.set_u32(manager + 0x20, OBJ + 0x1000 + 0x100);
    assert_eq!(get(&mut e, 9), b"", "font 9 is the byte after the eight");
    e.mem.set_u32(manager + 0x164, OBJ + 0x1000 + 0x100);
    assert_eq!(get(&mut e, 90), b"");
    assert_eq!(get(&mut e, 0), b"");
    e.mem.set_u32(manager + 4, 0);
    assert_eq!(get(&mut e, 2), b"", "no font");
    let font = OBJ + 0x1000 + 0x100;
    e.mem.set_u32(font + 4, 0);
    assert_eq!(get(&mut e, 1), b"", "no path");
}

#[test]
fn vats_targets() {
    let mut e = setup();
    let a = form(&mut e, OBJ, 0x3b, 0x0000_3001);
    let b = form(&mut e, OBJ + 0x100, 0x3b, 0x0000_3002);
    let (ta, tb, tn) = (OBJ + 0x200, OBJ + 0x300, OBJ + 0x400);
    e.mem.set_u32(ta, a);
    e.mem.set_u32(tb, b);
    e.mem.set_u32(tn, 0);
    e.mem.set_u32(0x011d_b150, ta);
    e.mem.set_u32(0x011d_b154, OBJ + 0x500);
    e.mem.set_u32(OBJ + 0x500, tn);
    e.mem.set_u32(OBJ + 0x504, OBJ + 0x508);
    e.mem.set_u32(OBJ + 0x508, tb);
    e.mem.set_u32(OBJ + 0x50c, OBJ + 0x510);
    e.mem.set_u32(OBJ + 0x510, 0);
    e.mem.set_u32(OBJ + 0x514, 0);
    assert_eq!(call(&mut e, 0x2827, &[]).1, 0.0, "no menu");
    e.mem.set_u32(0x011d_b0d4, MENUS);
    let id = call(&mut e, 0x2827, &[]).1;
    assert_eq!(
        array_data(&e, id),
        vec![Data::Form(0x3001), Data::Form(0x3002)]
    );
    e.mem.set_u32(ta, 0);
    e.mem.set_u32(tb, 0);
    assert_eq!(call(&mut e, 0x2827, &[]).1, 0.0, "none: no array");
}

#[test]
fn level_up_menu_with_points() {
    let mut e = setup();
    call(&mut e, 0x28e1, &[n(17)]);
    let m = MENUS + 0x3000;
    assert_eq!(e.mem.u32(m + 0x5c), 17);
    assert_eq!(ids(), vec![LEVEL_UP_CREATE_FN, LEVEL_UP_PAGE_FN]);
    assert_eq!(calls_to(LEVEL_UP_PAGE_FN), vec![vec![m, 0]]);
    // No menu made: only the create.
    let mut e = setup();
    e.register(LEVEL_UP_CREATE_FN, |_, w| {
        record(LEVEL_UP_CREATE_FN, w);
        eax(0)
    });
    call(&mut e, 0x28e1, &[n(17)]);
    assert_eq!(ids(), vec![LEVEL_UP_CREATE_FN]);
}

// ---- inventory references ----

/// A changes entry for `item` (count `count`, its first extra data list `xdata` or none).
fn entry(e: &mut Engine, at: u32, item: u32, count: i32, xdata: u32) -> u32 {
    if xdata != 0 {
        e.mem.set_u32(at + 0x10, xdata);
        e.mem.set_u32(at + 0x14, 0);
        e.mem.set_u32(at, at + 0x10);
    } else {
        e.mem.set_u32(at, 0);
    }
    e.mem.set_u32(at + 4, count as u32);
    e.mem.set_u32(at + 8, item);
    at
}

/// The inventory reference made for a reference id: (container, item, extra data list).
fn inv_ref(e: &mut Engine, id: u32) -> (u32, u32, u32) {
    let r = nvse::invref::get(e, id).expect("an inventory reference");
    (r.container, r.data.item, r.data.xdata)
}

#[test]
fn barter_offers_as_inventory_references() {
    let mut e = setup();
    let item1 = form(&mut e, OBJ, 0x28, 0x0000_4001);
    let item2 = form(&mut e, OBJ + 0x100, 0x1f, 0x0000_4002);
    let merchant = form(&mut e, OBJ + 0x200, 0x3b, 0x0000_4003);
    let chest = form(&mut e, OBJ + 0x300, 0x3a, 0x0000_4004);
    let xd = OBJ + 0x400;
    let e1 = entry(&mut e, OBJ + 0x500, item1, 2, xd);
    let e2 = entry(&mut e, OBJ + 0x600, item2, 1, 0);
    let barter = MENUS;
    assert_eq!(call(&mut e, 0x270e, &[n(0)]).1, 0.0, "no menu");
    e.mem.set_u32(0x011d_8fa4, barter);
    assert_eq!(call(&mut e, 0x270e, &[n(0)]).1, 0.0, "no merchant");
    e.mem.set_u32(barter + 0x80, merchant);
    e.mem.set_u32(barter + 0x10c, e1);
    e.mem.set_u32(barter + 0x110, OBJ + 0x700);
    e.mem.set_u32(OBJ + 0x700, e2);
    e.mem.set_u32(OBJ + 0x704, 0);
    let id = call(&mut e, 0x270e, &[n(0)]).1;
    let refs = array_data(&e, id);
    assert_eq!(refs.len(), 2);
    let ids: Vec<u32> = refs
        .iter()
        .map(|d| match d {
            Data::Form(f) => *f,
            _ => panic!("a form"),
        })
        .collect();
    assert_eq!(inv_ref(&mut e, ids[0]), (PC, item1, xd));
    assert_eq!(inv_ref(&mut e, ids[1]), (PC, item2, 0));
    // The merchant's side: in its merchant container (none: 0).
    e.mem.set_u32(barter + 0x114, e2);
    let id = call(&mut e, 0x270e, &[n(1)]).1;
    let Data::Form(r) = array_data(&e, id)[0] else {
        panic!("a form")
    };
    assert_eq!(inv_ref(&mut e, r), (0, item2, 0));
    let xm = OBJ + 0x800;
    put_extra(&mut e, merchant + 0x44, xm, 0x3c, chest);
    let id = call(&mut e, 0x270e, &[n(1)]).1;
    let Data::Form(r) = array_data(&e, id)[0] else {
        panic!("a form")
    };
    assert_eq!(inv_ref(&mut e, r).0, chest);
    // An empty offer: no array.
    e.mem.set_u32(barter + 0x114, 0);
    assert_eq!(call(&mut e, 0x270e, &[n(1)]).1, 0.0);
}

#[test]
fn menu_item_lists_as_a_map() {
    let mut e = setup();
    let item1 = form(&mut e, OBJ, 0x28, 0x0000_5001);
    let item2 = form(&mut e, OBJ + 0x100, 0x1f, 0x0000_5002);
    let item3 = form(&mut e, OBJ + 0x200, 0x1f, 0x0000_5003);
    let chest = form(&mut e, OBJ + 0x300, 0x3a, 0x0000_5004);
    let e1 = entry(&mut e, OBJ + 0x500, item1, 1, 0);
    let e2 = entry(&mut e, OBJ + 0x600, item2, 3, 0);
    let e3 = entry(&mut e, OBJ + 0x700, item3, 1, 0);
    // List box items: e1, (no object), e2 filtered, e3; a null item between.
    let li = |e: &mut Engine, at: u32, obj: u32, filtered: u8| {
        e.mem.set_u32(at, 0);
        e.mem.set_u32(at + 4, obj);
        e.mem.set_u8(at + 8, filtered);
        at
    };
    let items = [
        li(&mut e, OBJ + 0x900, e1, 0),
        li(&mut e, OBJ + 0x910, 0, 0),
        0,
        li(&mut e, OBJ + 0x920, e2, 1),
        li(&mut e, OBJ + 0x930, e3, 0),
    ];
    let list_at = |e: &mut Engine, head: u32| {
        for (i, it) in items.iter().enumerate() {
            let node = if i == 0 {
                head
            } else {
                OBJ + 0xa00 + 8 * i as u32
            };
            let next = if i + 1 < items.len() {
                OBJ + 0xa00 + 8 * (i as u32 + 1)
            } else {
                0
            };
            e.mem.set_u32(node, *it);
            e.mem.set_u32(node + 4, next);
        }
    };
    let refs = |e: &mut Engine, args: &[Vec<u8>]| -> Option<Vec<(f64, u32, u32)>> {
        let id = call(e, 0x2935, args).1;
        if id == 0.0 {
            return None;
        }
        let nvse::vars::Container::NumMap(v) = &nvse::vars::array(e, id as u32)?.elements else {
            panic!("a map by number")
        };
        let entries: Vec<(f64, Data)> = v.iter().map(|(k, el)| (*k, el.data.clone())).collect();
        Some(
            entries
                .into_iter()
                .map(|(k, v)| {
                    let Data::Form(r) = v else { panic!("forms") };
                    let (container, item, _) = inv_ref(e, r);
                    (k, container, item)
                })
                .collect(),
        )
    };
    assert_eq!(refs(&mut e, &[]), None, "no menu on top");
    // The inventory menu on top.
    in_menu_mode(&mut e);
    e.mem.set_u32(IM + 0x114, 0x3ea);
    assert_eq!(refs(&mut e, &[]), None, "no inventory menu");
    let inv = MENUS;
    e.mem.set_u32(0x011d_9ea4, inv);
    list_at(&mut e, inv + 0xb8 + 4);
    assert_eq!(
        refs(&mut e, &[]),
        Some(vec![(0.0, PC, item1), (2.0, PC, item3)]),
        "filtered left out but counted"
    );
    assert_eq!(
        refs(&mut e, &[n(1)]),
        Some(vec![(0.0, PC, item1), (1.0, PC, item2), (2.0, PC, item3)])
    );
    // The container menu: the other side in the container.
    let cont = MENUS + 0x400;
    e.mem.set_u32(0x011d_93f8, cont);
    e.mem.set_u32(IM + 0x114, 0x3f0);
    list_at(&mut e, cont + 0xc8 + 4);
    e.mem.set_u32(cont + 0x98 + 4, 0);
    e.mem.set_u32(cont + 0x98 + 8, 0);
    assert_eq!(
        refs(&mut e, &[n(0), n(0)]),
        Some(vec![]),
        "an empty list: an empty map"
    );
    assert_eq!(refs(&mut e, &[n(0), n(1)]), None, "no container");
    e.mem.set_u32(cont + 0x74, chest);
    assert_eq!(
        refs(&mut e, &[n(0), n(1)]),
        Some(vec![(0.0, chest, item1), (2.0, chest, item3)])
    );
    // Repair (+0x5C) and repair services (+0x68); barter's other side.
    for (id, singleton, list) in [
        (0x40bu32, 0x011d_a75cu32, 0x5cu32),
        (0x422, 0x011d_a7f0, 0x68),
        (0x41d, 0x011d_8fa4, 0xa8),
    ] {
        let m = MENUS + 0x800 + 0x200 * (id & 0xf);
        e.mem.set_u32(singleton, m);
        e.mem.set_u32(IM + 0x114, id);
        list_at(&mut e, m + list + 4);
        assert_eq!(refs(&mut e, &[n(0)]).map(|v| v.len()), Some(2), "{id:x}");
    }
    let barter = MENUS + 0x800 + 0x200 * 0xd;
    list_at(&mut e, barter + 0xd8 + 4);
    assert_eq!(refs(&mut e, &[n(0), n(1)]), None, "no merchant");
    let merchant = form(&mut e, OBJ + 0xc00, 0x3b, 0x0000_5005);
    e.mem.set_u32(barter + 0x80, merchant);
    put_extra(&mut e, merchant + 0x44, OBJ + 0xd00, 0x3c, chest);
    assert_eq!(refs(&mut e, &[n(0), n(1)]).map(|v| v[0].1), Some(chest));
    e.mem.set_u32(IM + 0x114, 0x3eb);
    assert_eq!(refs(&mut e, &[]), None, "another menu");
}

// ---- terminal models ----

#[test]
fn terminal_models_set_replace_and_remove() {
    let mut e = setup();
    let term = form(&mut e, OBJ, 0x17, 0x0000_6001);
    let term2 = form(&mut e, OBJ + 0x100, 0x17, 0x0000_6002);
    let other = form(&mut e, OBJ + 0x200, 0x1f, 0x0000_6003);
    let list = form(&mut e, OBJ + 0x300, 0x55, 0x0000_6004);
    e.mem.set_u32(list + 0x18, term2);
    e.mem.set_u32(list + 0x1c, OBJ + 0x400);
    e.mem.set_u32(OBJ + 0x400, other);
    e.mem.set_u32(OBJ + 0x404, 0);
    for (i, f) in [term, term2, other, list].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    let models = |e: &mut Engine| e.ext::<Jip>().ui_hooks.terminal_models.clone();
    let first = heap_next(&e);
    call(&mut e, 0x2822, &[r(1), s(b"a.nif")]);
    assert_eq!(allocations(), vec![6]);
    let held = models(&mut e)[&term];
    assert_eq!(nvse::table::guest_text(&e, held), b"a.nif");
    assert_eq!(held, first);
    assert_eq!(e.hook_users(Hook::SetTerminalModel), 1);
    // Replaced: the old text freed; in use, the model released and the game's path back.
    e.ext::<Jip>().ui_hooks.terminal_model_default = 0x0101_1584;
    e.mem.set_u32(0x011a_0bb0, first);
    call(&mut e, 0x2822, &[r(1), s(b"bb.nif")]);
    assert_eq!(freed(), vec![first]);
    assert_eq!(calls_to(PURGE_FN).len(), 1);
    assert_eq!(e.mem.u32(0x011a_0bb0), 0x0101_1584);
    let second = models(&mut e)[&term];
    assert_eq!(nvse::table::guest_text(&e, second), b"bb.nif");
    // A list: its terminals only.
    call(&mut e, 0x2822, &[r(4), s(b"c.nif")]);
    assert_eq!(models(&mut e).len(), 2);
    call(&mut e, 0x2822, &[r(3), s(b"d.nif")]);
    assert!(!models(&mut e).contains_key(&other));
    // Removed (not in use: no release).
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2822, &[r(1)]);
    assert_eq!(freed(), vec![second]);
    assert!(calls_to(PURGE_FN).is_empty());
    assert_eq!(e.hook_users(Hook::SetTerminalModel), 1, "one left");
    call(&mut e, 0x2822, &[r(1)]);
    call(&mut e, 0x2822, &[r(2), s(b"")]);
    assert!(models(&mut e).is_empty());
    assert_eq!(e.hook_users(Hook::SetTerminalModel), 0);
}

/// JIP's helpers: tokens, slashes.
#[test]
fn helpers() {
    assert_eq!(next_token(b"a||b|c", b'|'), (&b"a"[..], &b"b|c"[..]));
    assert_eq!(next_token(b"abc", b'|'), (&b"abc"[..], &b""[..]));
    assert_eq!(next_token(b"a|", b'|'), (&b"a"[..], &b""[..]));
    assert_eq!(slash_pos(b"ab\\c/d"), Some(2));
    assert_eq!(slash_pos(b"abc"), None);
}

const COMPRESS_FN: u32 = 0x00a1_ce70;

thread_local! {
    /// The text the built-in UIO handed to each `Tile::ReadFile` of `jip_temp.xml`.
    static INJECTED: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[test]
fn inject_component_goes_through_the_built_in_uio_only() {
    let mut e = setup();
    tree(&mut e);
    let inject = |e: &mut Engine, path: &[u8], xml: &[u8]| call(e, 0x260a, &[s(path), s(xml)]).1;
    // `Menu::CompressAndStoreXML(name, text)`: the text as the game's storage.
    e.register(COMPRESS_FN, |e, w| {
        record(COMPRESS_FN, w);
        let text = nvse::table::guest_text(e, w[1]);
        let data = e.mem.alloc(text.len() as u32 + 1);
        e.mem.write(data, &[text.as_slice(), b"\0"].concat());
        let storage = e.mem.alloc(8);
        e.mem.set_u32(storage, text.len() as u32);
        e.mem.set_u32(storage + 4, data);
        eax(storage)
    });
    e.register(READ_FILE_FN, |e, w| {
        record(READ_FILE_FN, w);
        let path = nvse::table::guest_text(e, w[1]);
        if path != b"jip_temp.xml" {
            return eax(0);
        }
        let text = e.ext::<uio::Uio>().extern_text.clone().unwrap_or_default();
        INJECTED.with(|i| i.borrow_mut().push(text));
        eax(t(62))
    });
    // UIO off: JIP would write `jip_temp.xml` into the game's folder; here nothing is done.
    assert_eq!(inject(&mut e, b"InventoryMenu/Box", b"<rect/>"), 0.0);
    assert!(calls_to(READ_FILE_FN).is_empty());
    assert!(calls_to(COMPRESS_FN).is_empty());
    // On: the text read into the tile through UIO, under `jip_temp.xml`.
    uio::set_on(&mut e, true);
    uio::install(&mut e);
    assert_eq!(
        inject(&mut e, b"InventoryMenu/Box", br#"<rect name="x"/>"#),
        1.0
    );
    assert_eq!(calls_to(READ_FILE_FN)[0][0], t(2));
    assert_eq!(
        INJECTED.with(|i| i.borrow().clone()),
        vec![br#"<rect name="x"/>"#.to_vec()]
    );
    assert!(e.ext::<uio::Uio>().extern_text.is_none());
    // A path ending at a trait, an unknown tile, an empty text: nothing read.
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(inject(&mut e, b"InventoryMenu/Box/id", b"<rect/>"), 0.0);
    assert_eq!(inject(&mut e, b"InventoryMenu/No", b"<rect/>"), 0.0);
    assert_eq!(inject(&mut e, b"InventoryMenu/Box", b""), 0.0);
    assert!(calls_to(READ_FILE_FN).is_empty());
}
