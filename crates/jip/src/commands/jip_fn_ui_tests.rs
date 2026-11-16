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
const TRAITS: [(&[u8], u32); 6] = [
    (b"id", 0xfaa),
    (b"target", 0xfaf),
    (b"clicked", 0xfc7),
    (b"string", 0xfb4),
    (b"_enabled", 0xfd0),
    (b"_MarkerIndex", 0xfd1),
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

// ---- package C2 ----

const HUD: u32 = FORMS + 0xa_0000;
const QUEUE_NODES: u32 = FORMS + 0xa_1000;
const SCM: u32 = FORMS + 0xa_2000;
const COLOR_VT: u32 = FORMS + 0xa_3000;
const SET_COLOR_FN: u32 = 0x0dea_0110;
const REFRESH_COLOR_FN: u32 = 0x00a0_b350;
const SPELL: u32 = FORMS + 0xa_4000;
const PIPBOY: u32 = FORMS + 0xa_5000;
const CASTER_VT: u32 = FORMS + 0xa_6000;
const CAST_FN: u32 = 0x0dea_0120;
const IS_TARGET_FN: u32 = 0x0082_2b90;
const DISPEL_FN: u32 = 0x0082_4400;
const SHOW_LIGHT_FN: u32 = 0x007f_a310;
const FONTS: u32 = FORMS + 0xa_7000;
const DIMS_FN: u32 = 0x00a1_b020;
const QUEUE_FN: u32 = 0x0070_52f0;
const C2_LOCALS: u32 = crate::test_support::LIST + 0x100;

/// The HUD with three queued messages (nodes chained from the inline head) and its message
/// tiles 5, 6, 7 with an alpha value of 1 each (tile 7's alpha already 0).
fn queue(e: &mut Engine) -> [u32; 3] {
    e.mem.set_u32(0x011d_96c0, HUD);
    let messages = [
        QUEUE_NODES + 0x100,
        QUEUE_NODES + 0x200,
        QUEUE_NODES + 0x300,
    ];
    let (n1, n2) = (QUEUE_NODES, QUEUE_NODES + 8);
    e.mem.set_u32(HUD + 0x18c, messages[0]);
    e.mem.set_u32(HUD + 0x190, n1);
    e.mem.set_u32(n1, messages[1]);
    e.mem.set_u32(n1 + 4, n2);
    e.mem.set_u32(n2, messages[2]);
    e.mem.set_u32(n2 + 4, 0);
    e.mem.set_u32(HUD + 0x194, 1234);
    for (i, at) in [0x8c, 0x90, 0x94].iter().enumerate() {
        let tl = tile(e, 5 + i as u32, 0, b"msg");
        e.mem.set_u32(HUD + at, tl);
        values(e, tl, &[(0xfa9, if i == 2 { 0.0 } else { 1.0 }, None)]);
    }
    messages
}

#[test]
fn clear_message_queue_cmd() {
    let mut e = setup();
    let messages = queue(&mut e);
    assert_eq!(call(&mut e, 0x2872, &[]).1, 0.0, "no result");
    // Each later node's message then the node, then the first message; the head emptied.
    assert_eq!(
        freed(),
        vec![
            messages[1],
            QUEUE_NODES,
            messages[2],
            QUEUE_NODES + 8,
            messages[0]
        ]
    );
    assert_eq!((e.mem.u32(HUD + 0x18c), e.mem.u32(HUD + 0x190)), (0, 0));
    assert_eq!(e.mem.u32(HUD + 0x194), 0);
    // The alpha values 0, refreshed where they changed (tiles 5 and 6, not 7).
    for n in [5, 6, 7] {
        assert_eq!(value_float(&e, t(n), 0xfa9), 0.0);
    }
    assert_eq!(calls_to(REFRESH_FN).len(), 2);
    // An empty queue (no first message): nothing at all.
    let messages = queue(&mut e);
    e.mem.set_u32(HUD + 0x18c, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2872, &[]);
    assert!(freed().is_empty());
    assert_eq!(e.mem.u32(HUD + 0x194), 1234);
    assert_eq!(value_float(&e, t(5), 0xfa9), 1.0);
    // One message: just it.
    let _ = messages;
    let messages = queue(&mut e);
    e.mem.set_u32(HUD + 0x190, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2872, &[]);
    assert_eq!(freed(), vec![messages[0]]);
    // No HUD: nothing.
    e.mem.set_u32(0x011d_96c0, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2872, &[]);
    assert!(calls().is_empty());
}

/// The system colour manager with `n` colours (nodes at `SCM + 0x100 * (i + 1)`, colour objects
/// at `+0x40` of each, their vtable's `SetColor` recording `(this, colour)`).
fn colors(e: &mut Engine, n: u32) -> Vec<u32> {
    e.mem.set_u32(0x011d_8a88, SCM);
    e.mem.set_u32(COLOR_VT + 8, SET_COLOR_FN);
    e.register(SET_COLOR_FN, |_, w| {
        record(SET_COLOR_FN, w);
        eax(0)
    });
    e.register(REFRESH_COLOR_FN, |_, w| {
        record(REFRESH_COLOR_FN, w);
        eax(0)
    });
    e.mem.set_u32(IM + 0x9c, t(0));
    let mut objects = Vec::new();
    for i in 0..n {
        let node = SCM + 0x100 * (i + 1);
        let next = if i + 1 < n { node + 0x100 } else { 0 };
        e.mem.set_u32(node, next);
        e.mem.set_u32(node + 8, node + 0x40);
        e.mem.set_u32(node + 0x40, COLOR_VT);
        objects.push(node + 0x40);
    }
    e.mem.set_u32(SCM, if n > 0 { SCM + 0x100 } else { 0 });
    objects
}

#[test]
fn set_system_color_cmd() {
    let mut e = setup();
    let objects = colors(&mut e, 5);
    // Type 3: the third colour; r g b's low bytes, alpha 255; then the menus refreshed.
    assert_eq!(
        call(&mut e, 0x2881, &[n(3), n(0x112), n(0x34), n(0x5678)]).1,
        0.0
    );
    assert_eq!(calls_to(SET_COLOR_FN), vec![vec![objects[2], 0x1234_78ff]]);
    assert_eq!(calls_to(REFRESH_COLOR_FN), vec![vec![t(0), 3, 0]]);
    // Each argument in its place: type 1 and 5 are the first and the last.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2881, &[n(1), n(1), n(2), n(3)]);
    call(&mut e, 0x2881, &[n(5), n(0xff), n(0), n(0x80)]);
    assert_eq!(
        calls_to(SET_COLOR_FN),
        vec![vec![objects[0], 0x0102_03ff], vec![objects[4], 0xff00_80ff]]
    );
    // Type 0 or above 5 (6, and -1 as unsigned): nothing.
    CALLS.with(|c| c.borrow_mut().clear());
    for kind in [0, 6, -1] {
        call(&mut e, 0x2881, &[n(kind), n(1), n(2), n(3)]);
    }
    assert!(calls().is_empty());
    // Fewer colours than the type: nothing.
    colors(&mut e, 2);
    call(&mut e, 0x2881, &[n(3), n(1), n(2), n(3)]);
    assert!(calls().is_empty());
    // No manager: nothing.
    e.mem.set_u32(0x011d_8a88, 0);
    call(&mut e, 0x2881, &[n(1), n(1), n(2), n(3)]);
    assert!(calls().is_empty());
}

/// The player's light: `IsSpellTarget` answers the byte at `SPELL + 0x80`.
fn pipboy(e: &mut Engine) {
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(0x011c_358c, SPELL);
    e.mem.set_u32(IM + 0x174, PIPBOY);
    e.mem.set_u32(PC + 0x88, CASTER_VT);
    e.mem.set_u32(CASTER_VT, CAST_FN);
    e.register(IS_TARGET_FN, |e, w| {
        record(IS_TARGET_FN, w);
        eax(0xabcd_ef00 | u32::from(e.mem.u8(SPELL + 0x80)))
    });
    e.register(CAST_FN, |_, w| {
        record(CAST_FN, w);
        eax(0)
    });
    e.register(DISPEL_FN, |_, w| {
        record(DISPEL_FN, w);
        eax(0)
    });
    e.register(SHOW_LIGHT_FN, |_, w| {
        record(SHOW_LIGHT_FN, w);
        eax(0)
    });
}

#[test]
fn toggle_pipboy_light_cmd_toggles() {
    let mut e = setup();
    pipboy(&mut e);
    // Off: 0; asked of the player's MagicTarget with the spell's MagicItem.
    assert_eq!(call(&mut e, 0x28ac, &[]).1, 0.0);
    assert_eq!(
        calls_to(IS_TARGET_FN),
        vec![vec![PC + 0x94, SPELL + 0x18, 1]]
    );
    // On (the low byte only): 1; no argument: no change.
    e.mem.set_u8(SPELL + 0x80, 1);
    assert_eq!(call(&mut e, 0x28ac, &[]).1, 1.0);
    // The same state asked: nothing done.
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(call(&mut e, 0x28ac, &[n(1)]).1, 1.0);
    assert_eq!(ids(), vec![IS_TARGET_FN]);
    // Off asked while on: dispelled, then the effect shown (1, 0, 1) and (0, 0, 1).
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28ac, &[n(0)]);
    assert_eq!(
        calls_to(DISPEL_FN),
        vec![vec![PC + 0x94, SPELL + 0x18, 0, 0]]
    );
    assert_eq!(
        calls_to(SHOW_LIGHT_FN),
        vec![vec![PIPBOY, 1, 0, 1], vec![PIPBOY, 0, 0, 1]]
    );
    assert!(calls_to(CAST_FN).is_empty());
    assert_eq!(
        ids(),
        vec![IS_TARGET_FN, DISPEL_FN, SHOW_LIGHT_FN, SHOW_LIGHT_FN]
    );
    // 2 while on differs (compared whole): cast (the spell, 0), then shown with 2.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28ac, &[n(2)]);
    assert_eq!(calls_to(CAST_FN), vec![vec![PC + 0x88, SPELL, 0]]);
    assert_eq!(
        calls_to(SHOW_LIGHT_FN),
        vec![vec![PIPBOY, 1, 2, 1], vec![PIPBOY, 0, 2, 1]]
    );
    assert!(calls_to(DISPEL_FN).is_empty());
    // On asked while off: cast.
    e.mem.set_u8(SPELL + 0x80, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(call(&mut e, 0x28ac, &[n(1)]).1, 0.0);
    assert_eq!(calls_to(CAST_FN).len(), 1);
    // No Pip-Boy manager: cast, no effect shown.
    e.mem.set_u32(IM + 0x174, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28ac, &[n(1)]);
    assert_eq!(ids(), vec![IS_TARGET_FN, CAST_FN]);
    // No spell: nothing.
    e.mem.set_u32(0x011c_358c, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(call(&mut e, 0x28ac, &[n(1)]).1, 0.0);
    assert!(calls().is_empty());
}

fn c2_locals(e: &mut Engine, n: u32) {
    for i in 0..n {
        let local = C2_LOCALS + 0x100 + i * 0x10;
        e.mem.set_u32(local, i + 1);
        e.mem.set_f64(local + 8, -1.0);
        let node = C2_LOCALS + i * 8;
        e.mem.set_u32(node, local);
        e.mem
            .set_u32(node + 4, if i + 1 < n { node + 8 } else { 0 });
    }
    e.mem.set_u32(crate::test_support::LIST + 0xc, C2_LOCALS);
}

fn local_value(e: &Engine, id: u32) -> f64 {
    e.mem.f64(C2_LOCALS + 0x100 + (id - 1) * 0x10 + 8)
}

/// A script local variable argument (`f` and its id).
fn var(id: u16) -> Vec<u8> {
    let mut b = vec![b'f'];
    b.extend_from_slice(&id.to_le_bytes());
    b
}

#[test]
fn string_ui_dimensions() {
    let mut e = setup();
    c2_locals(&mut e, 3);
    e.mem.set_u32(0x011f_33f8, FONTS);
    e.register(DIMS_FN, |e, w| {
        record(DIMS_FN, w);
        let text = nvse::table::guest_text(e, w[2]);
        assert_eq!(text, b"Hello");
        e.mem.set_f32(w[1], 30.5);
        e.mem.set_f32(w[1] + 4, 12.0);
        e.mem.set_f32(w[1] + 8, 2.0);
        eax(w[1])
    });
    let args = |wrap: f64| [s(b"Hello"), n(3), z(wrap), var(1), var(2), var(3)];
    assert_eq!(call(&mut e, 0x2927, &args(100.0)).1, 0.0, "no result");
    let w = &calls_to(DIMS_FN)[0];
    assert_eq!(
        (w[0], w[3], f32::from_bits(w[4]), w[5]),
        (FONTS, 3, 100.0, 0)
    );
    assert_eq!([1, 2, 3].map(|i| local_value(&e, i)), [30.5, 12.0, 2.0]);
    // A wrap width of 0 or below: FLT_MAX.
    for wrap in [0.0, -5.0] {
        CALLS.with(|c| c.borrow_mut().clear());
        call(&mut e, 0x2927, &args(wrap));
        assert_eq!(f32::from_bits(calls_to(DIMS_FN)[0][4]), f32::MAX);
    }
    // The variables in their order.
    call(
        &mut e,
        0x2927,
        &[s(b"Hello"), n(3), z(1.0), var(3), var(1), var(2)],
    );
    assert_eq!([3, 1, 2].map(|i| local_value(&e, i)), [30.5, 12.0, 2.0]);
    // No font manager: nothing.
    e.mem.set_u32(0x011f_33f8, 0);
    c2_locals(&mut e, 3);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2927, &args(1.0));
    assert!(calls_to(DIMS_FN).is_empty());
    assert_eq!(local_value(&e, 1), -1.0);
}

#[test]
fn message_tokens() {
    // No bar: all of it is the message.
    assert_eq!(
        split_message(b"Hello"),
        (b"Hello".to_vec(), Icon::None, None)
    );
    // An icon path, a sound, then the message.
    assert_eq!(
        split_message(b"icons\\a.dds|$UIPopUp|Hi there"),
        (
            b"Hi there".to_vec(),
            Icon::Text(b"icons\\a.dds".to_vec()),
            Some(b"UIPopUp".to_vec())
        )
    );
    // `#n`: the exe's icons 0 to 6; another `#` token leaves the icon as it was.
    assert_eq!(split_message(b"#0|m").1, Icon::Exe(0x0102_08a0));
    assert_eq!(split_message(b"#6|m").1, Icon::Exe(0x0104_bfe8));
    assert_eq!(split_message(b"#3x|m").1, Icon::Exe(0x0103_0e78));
    assert_eq!(split_message(b"#7|m").1, Icon::None);
    assert_eq!(split_message(b"#|m").1, Icon::None);
    assert_eq!(split_message(b"#/|m").1, Icon::None);
    assert_eq!(split_message(b"a|#9|m").1, Icon::Text(b"a".to_vec()));
    // A later icon replaces an earlier one; runs of bars are one.
    assert_eq!(
        split_message(b"#1||b|||m"),
        (b"m".to_vec(), Icon::Text(b"b".to_vec()), None)
    );
    // A trailing bar: the token before it is the message (nothing after it).
    assert_eq!(
        split_message(b"$s|msg|"),
        (b"msg".to_vec(), Icon::None, Some(b"s".to_vec()))
    );
    // An empty token is an empty icon path.
    assert_eq!(split_message(b"|m").1, Icon::Text(Vec::new()));
    // The message cut to 0x103 bytes (the game's 0x104-byte copy; JIP's own cut is 0x203, which
    // needs its corner message length patch), and the icon path and sound too.
    let long = vec![b'x'; 0x300];
    assert_eq!(split_message(&long).0.len(), 0x103);
    for n in [0x103usize, 0x104, 0x203, 0x204] {
        let mut text = vec![b'i'; n];
        text.push(b'|');
        text.push(b'$');
        text.extend(vec![b's'; n]);
        text.push(b'|');
        text.extend(vec![b'm'; n]);
        let (m, icon, sound) = split_message(&text);
        let fit = n.min(0x103);
        assert_eq!(m, vec![b'm'; fit], "{n}");
        assert_eq!(icon, Icon::Text(vec![b'i'; fit]), "{n}");
        assert_eq!(sound, Some(vec![b's'; fit]), "{n}");
    }
}

#[test]
fn message_ex_alt_queues() {
    let mut e = setup();
    e.register(QUEUE_FN, |e, w| {
        record(QUEUE_FN, w);
        // The texts on the stack: the message, a text icon (not the exe's), the sound.
        let texts: Vec<Vec<u8>> = [w[0], w[2], w[3]]
            .iter()
            .map(|&p| {
                if p == 0 || MESSAGE_ICONS.contains(&p) {
                    b"(none)".to_vec()
                } else {
                    nvse::table::guest_text(e, p)
                }
            })
            .collect();
        CALLS.with(|c| {
            for t in texts {
                let mut words = vec![t.len() as u32];
                words.extend(t.iter().map(|&b| u32::from(b)));
                c.borrow_mut().push((0xfeed, words));
            }
        });
        eax(1)
    });
    let texts = || -> Vec<Vec<u8>> {
        calls_to(0xfeed)
            .into_iter()
            .map(|w| w[1..].iter().map(|&b| b as u8).collect())
            .collect()
    };
    // The seconds (argument 0) as a float; icon type 0, last byte 0. No result.
    assert_eq!(
        call(&mut e, 0x2738, &[z(3.5), s(b"#2|$snd|Hello %g"), z(5.0)]).1,
        0.0
    );
    let w = &calls_to(QUEUE_FN)[0];
    assert_eq!(
        (w[1], w[2], f32::from_bits(w[4]), w[5]),
        (0, 0x0102_5cdc, 3.5, 0)
    );
    assert_eq!(
        texts(),
        vec![b"Hello 5".to_vec(), b"(none)".to_vec(), b"snd".to_vec()]
    );
    // A text icon, no sound.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2738, &[z(1.0), s(b"my.dds|msg")]);
    let w = &calls_to(QUEUE_FN)[0];
    assert_eq!((w[3], f32::from_bits(w[4])), (0, 1.0), "no sound");
    assert_eq!(
        texts(),
        vec![b"msg".to_vec(), b"my.dds".to_vec(), b"(none)".to_vec()]
    );
    // Only a message: no icon, no sound.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2738, &[z(1.0), s(b"plain")]);
    let w = &calls_to(QUEUE_FN)[0];
    assert_eq!((w[2], w[3]), (0, 0));
    assert_eq!(texts()[0], b"plain");
    // A 0x104-byte message, icon path and sound: each handed on in 0x103 bytes and its 0, what
    // the game's queue copies into its 0x104-byte buffers.
    let mut text = vec![b'i'; 0x104];
    text.extend(b"|$");
    text.extend(vec![b's'; 0x104]);
    text.push(b'|');
    text.extend(vec![b'm'; 0x104]);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2738, &[z(1.0), s(&text)]);
    assert_eq!(
        texts(),
        vec![vec![b'm'; 0x103], vec![b'i'; 0x103], vec![b's'; 0x103]]
    );
}

/// Arguments that do not read (a global argument with no global behind it, a variable with no
/// local): no game call, no variable written; SetSystemColor without the interface manager sets
/// the colour and refreshes nothing.
#[test]
fn c2_unreadable_arguments_and_no_interface_manager() {
    let mut e = setup();
    c2_locals(&mut e, 3);
    e.mem.set_u32(0x011f_33f8, FONTS);
    e.register(DIMS_FN, |_, w| {
        record(DIMS_FN, w);
        eax(w[1])
    });
    e.register(QUEUE_FN, |_, w| {
        record(QUEUE_FN, w);
        eax(1)
    });
    let bad = || vec![b'G', 9, 0];
    call(
        &mut e,
        0x2927,
        &[s(b"Hi"), n(1), bad(), var(1), var(2), var(3)],
    );
    call(
        &mut e,
        0x2927,
        &[s(b"Hi"), n(1), z(1.0), var(1), var(2), var(9)],
    );
    assert!(calls_to(DIMS_FN).is_empty());
    assert_eq!([1, 2, 3].map(|i| local_value(&e, i)), [-1.0; 3]);
    call(&mut e, 0x2738, &[bad(), s(b"Hello")]);
    assert!(calls_to(QUEUE_FN).is_empty());
    // SetSystemColor with no interface manager.
    let objects = colors(&mut e, 2);
    e.mem.set_u32(0x011d_8a80, 0);
    call(&mut e, 0x2881, &[n(2), n(1), n(2), n(3)]);
    assert_eq!(calls_to(SET_COLOR_FN), vec![vec![objects[1], 0x0102_03ff]]);
    assert!(calls_to(REFRESH_COLOR_FN).is_empty());
}

// ---- a menu's target and its selected item ----

const Q: u32 = FORMS + 0xa_0000;

/// The form result of `GetMenuTargetRef id` (0 for none: the result is cleared first).
fn menu_target(e: &mut Engine, id: u32) -> u32 {
    e.mem.set_f64(crate::test_support::RESULT, 0.0);
    call(e, 0x2685, &[n(id as i32)]);
    form_result(e)
}

/// The form result of `GetSelectedItemRef`.
fn selected_item(e: &mut Engine) -> u32 {
    e.mem.set_f64(crate::test_support::RESULT, 0.0);
    call(e, 0x270b, &[]);
    form_result(e)
}

/// A list box at `at`: list items (tile, object) at `items_at`, the highlighted tile `selected`.
fn list_box(e: &mut Engine, at: u32, items_at: u32, items: &[(u32, u32)], selected: u32) {
    for (i, (tile, object)) in items.iter().enumerate() {
        let item = items_at + 0x10 * i as u32;
        e.mem.set_u32(item, *tile);
        e.mem.set_u32(item + 4, *object);
        let node = if i == 0 {
            at + 4
        } else {
            items_at + 0x100 + 8 * i as u32
        };
        let next = if i + 1 < items.len() {
            items_at + 0x100 + 8 * (i as u32 + 1)
        } else {
            0
        };
        e.mem.set_u32(node, item);
        e.mem.set_u32(node + 4, next);
    }
    e.mem.set_u32(at + 0x10, selected);
}

#[test]
fn menu_target_refs_of_the_menus_own_fields() {
    let mut e = setup();
    let target = form(&mut e, Q, 0x3b, 0x0000_6001);
    // Not open, outside the menu ids: none.
    assert_eq!(menu_target(&mut e, 0x3f0), 0);
    assert_eq!(menu_target(&mut e, 0x3e8), 0);
    assert_eq!(menu_target(&mut e, 0x43d), 0);
    // A menu tile without a menu: none.
    let (mt, m) = menu(&mut e, 20, 0x3f0, b"ContainerMenu");
    e.mem.set_u32(m + 0x74, target);
    e.mem.set_u32(mt + 0x3c, 0);
    assert_eq!(menu_target(&mut e, 0x3f0), 0);
    // Container +0x74, lock pick +0x6C, barter +0x80, computers +0xAC, companion wheel +0x68,
    // recipe +0x60, caravan +0x100: each menu's own reference.
    let fields = [
        (0x3f0, 0x74),
        (0x3f6, 0x6c),
        (0x41d, 0x80),
        (0x421, 0xac),
        (0x433, 0x68),
        (0x435, 0x60),
        (0x43b, 0x100),
    ];
    for (i, (id, field)) in fields.into_iter().enumerate() {
        let (_, m) = menu(&mut e, 21 + 2 * i as u32, id, b"Menu");
        assert_eq!(menu_target(&mut e, id), 0, "{id:x} empty");
        e.mem.set_u32(m + field, target);
        assert_eq!(menu_target(&mut e, id), 0x0000_6001, "{id:x}");
    }
    // A menu id this command does not know: none.
    let (_, m) = menu(&mut e, 40, 0x3ea, b"InventoryMenu");
    e.mem.set_u32(m + 0x74, target);
    assert_eq!(menu_target(&mut e, 0x3ea), 0);
    // The VATS target and the repair vendor: the game's globals, whatever the menu holds.
    e.map(0x011f_2000, 0x1000);
    menu(&mut e, 42, 0x420, b"VATSMenu");
    menu(&mut e, 44, 0x422, b"RepairServicesMenu");
    assert_eq!(menu_target(&mut e, 0x420), 0);
    assert_eq!(menu_target(&mut e, 0x422), 0);
    e.mem.set_u32(0x011f_21cc, target);
    assert_eq!(menu_target(&mut e, 0x420), 0x0000_6001);
    e.mem.set_u32(0x011d_a7f4, target);
    assert_eq!(menu_target(&mut e, 0x422), 0x0000_6001);
}

#[test]
fn menu_target_dialog_hacking_level_up_trait_and_map() {
    let mut e = setup();
    // The dialogue partner; a talking activator's talking actor when it has one.
    let base = form(&mut e, Q, 0x2a, 0x0000_6010);
    let partner = form(&mut e, Q + 0x100, 0x3b, 0x0000_6011);
    let actor = form(&mut e, Q + 0x200, 0x3c, 0x0000_6012);
    e.mem.set_u32(partner + 0x20, base);
    let (_, dialog) = menu(&mut e, 20, 0x3f1, b"DialogMenu");
    assert_eq!(menu_target(&mut e, 0x3f1), 0);
    e.mem.set_u32(dialog + 0x80, partner);
    assert_eq!(
        menu_target(&mut e, 0x3f1),
        0x0000_6011,
        "not a talking activator"
    );
    e.mem.set_u8(base + 4, 0x16);
    assert_eq!(
        menu_target(&mut e, 0x3f1),
        0x0000_6011,
        "a talking activator with no talking actor"
    );
    put_extra(&mut e, partner + 0x44, Q + 0x300, 0x55, actor);
    assert_eq!(menu_target(&mut e, 0x3f1), 0x0000_6012);
    e.mem.set_u8(base + 4, 0x2a);
    assert_eq!(
        menu_target(&mut e, 0x3f1),
        0x0000_6011,
        "a talking actor extra, not a talking activator"
    );
    e.mem.set_u8(base + 4, 0x16);
    // Hacking: the terminal once the guess tiles' count (its low byte) is set, else the
    // interface manager's activate reference.
    let terminal = form(&mut e, Q + 0x400, 0x3b, 0x0000_6020);
    let activate = form(&mut e, Q + 0x500, 0x3b, 0x0000_6021);
    let (_, hacking) = menu(&mut e, 22, 0x41f, b"HackingMenu");
    e.mem.set_u32(hacking + 0x198, terminal);
    e.mem.set_u32(IM + 0xfc, activate);
    e.mem.set_u32(hacking + 0x17c, 0x100);
    assert_eq!(menu_target(&mut e, 0x41f), 0x0000_6021);
    e.mem.set_u32(hacking + 0x17c, 1);
    assert_eq!(menu_target(&mut e, 0x41f), 0x0000_6020);
    // Level up: the highlighted perk on the perk page (1) only; trait: the highlighted perk.
    let perk1 = form(&mut e, Q + 0x600, 0x56, 0x0000_6030);
    let perk2 = form(&mut e, Q + 0x700, 0x56, 0x0000_6031);
    let (_, level_up) = menu(&mut e, 24, 0x403, b"LevelUpMenu");
    let items = [(t(70), perk1), (t(71), perk2)];
    list_box(&mut e, level_up + 0x94, Q + 0x1000, &items, t(71));
    assert_eq!(menu_target(&mut e, 0x403), 0, "the skills page");
    e.mem.set_u32(level_up + 0x28, 2);
    assert_eq!(menu_target(&mut e, 0x403), 0, "page 2");
    e.mem.set_u32(level_up + 0x28, 1);
    assert_eq!(menu_target(&mut e, 0x403), 0x0000_6031);
    e.mem.set_u32(level_up + 0x94 + 0x10, t(72));
    assert_eq!(menu_target(&mut e, 0x403), 0, "a tile no item has");
    e.mem.set_u32(level_up + 0x94 + 0x10, 0);
    assert_eq!(menu_target(&mut e, 0x403), 0, "nothing highlighted");
    let (_, traits) = menu(&mut e, 26, 0x43c, b"TraitMenu");
    list_box(&mut e, traits + 0x54, Q + 0x1200, &items, t(70));
    assert_eq!(menu_target(&mut e, 0x43c), 0x0000_6030);
    // Map: the marker tile's `_MarkerIndex` (truncated) picks the marker list's item.
    let markers = [
        form(&mut e, Q + 0x800, 0x3b, 0x0000_6040),
        form(&mut e, Q + 0x900, 0x3b, 0x0000_6041),
    ];
    let (_, map) = menu(&mut e, 28, 0x3ff, b"MapMenu");
    e.mem.set_u32(map + 0xd8, markers[0]);
    e.mem.set_u32(map + 0xdc, Q + 0x1400);
    e.mem.set_u32(Q + 0x1400, markers[1]);
    assert_eq!(menu_target(&mut e, 0x3ff), 0, "no marker tile");
    let marker_tile = tile(&mut e, 73, 0, b"marker");
    e.mem.set_u32(map + 0xd4, marker_tile);
    assert_eq!(menu_target(&mut e, 0x3ff), 0, "no _MarkerIndex");
    let v = values(&mut e, marker_tile, &[(0xfd1, 1.75, None)])[0];
    assert_eq!(menu_target(&mut e, 0x3ff), 0x0000_6041);
    for (num, want) in [
        (0.99, 0x0000_6040),
        (2.0, 0),
        (-0.5, 0x0000_6040),
        (-1.0, 0),
        (f32::NAN, 0),
    ] {
        e.mem.set_f32(v + 8, num);
        assert_eq!(menu_target(&mut e, 0x3ff), want, "{num}");
    }
}

#[test]
fn menu_target_items_as_inventory_references() {
    let mut e = setup();
    let item = form(&mut e, Q, 0x28, 0x0000_6101);
    let chest = form(&mut e, Q + 0x100, 0x3a, 0x0000_6102);
    let merchant = form(&mut e, Q + 0x200, 0x3b, 0x0000_6103);
    let shop = form(&mut e, Q + 0x300, 0x3a, 0x0000_6104);
    // Each selection global holds its own item, so a read of the wrong global shows.
    let item_c = form(&mut e, Q + 0x900, 0x28, 0x0000_6105);
    let item_b = form(&mut e, Q + 0xa00, 0x28, 0x0000_6106);
    let xd = Q + 0x400;
    let inv = entry(&mut e, Q + 0x500, item, 3, xd);
    let cnt = entry(&mut e, Q + 0x600, item_c, 2, 0);
    let brt = entry(&mut e, Q + 0x700, item_b, 1, 0);
    let target = |e: &mut Engine, id: u32| {
        let r = menu_target(e, id);
        if r == 0 {
            None
        } else {
            Some(inv_ref(e, r))
        }
    };
    menu(&mut e, 20, 0x3f8, b"QuantityMenu");
    e.mem.set_u32(0x011d_9ea8, inv);
    e.mem.set_u32(0x011d_8fa8, brt);
    // No inventory shown, no container or barter menu: none.
    assert_eq!(target(&mut e, 0x3f8), None);
    // The barter menu's item: the player's, or the merchant container's when the right list is
    // the current one (none for a merchant without one).
    let barter = Q + 0x2000;
    e.mem.set_u32(0x011d_8fa4, barter);
    assert_eq!(target(&mut e, 0x3f8), Some((PC, item_b, 0)));
    e.mem.set_u32(barter + 0x80, merchant);
    e.mem.set_u32(barter + 0x108, barter + 0xd8);
    assert_eq!(target(&mut e, 0x3f8), None);
    put_extra(&mut e, merchant + 0x44, Q + 0x800, 0x3c, shop);
    assert_eq!(target(&mut e, 0x3f8), Some((shop, item_b, 0)));
    e.mem.set_u32(barter + 0x108, barter + 0xa8);
    assert_eq!(target(&mut e, 0x3f8), Some((PC, item_b, 0)));
    // The container menu before the barter menu: in the container when its right list is the
    // current one.
    let container = Q + 0x2200;
    e.mem.set_u32(0x011d_93f8, container);
    e.mem.set_u32(0x011d_93fc, cnt);
    e.mem.set_u32(container + 0x74, chest);
    assert_eq!(target(&mut e, 0x3f8), Some((PC, item_c, 0)));
    e.mem.set_u32(container + 0xf8, container + 0xc8);
    assert_eq!(target(&mut e, 0x3f8), Some((chest, item_c, 0)));
    // The inventory shown before both: the inventory's item, the player's.
    e.mem.set_u8(0x011f_308f + 0x3ea, 1);
    assert_eq!(target(&mut e, 0x3f8), Some((PC, item, xd)));
    // Repair and item mod: their items, the player's; none without one.
    menu(&mut e, 22, 0x40b, b"RepairMenu");
    menu(&mut e, 24, 0x425, b"ItemModMenu");
    assert_eq!(target(&mut e, 0x40b), None);
    assert_eq!(target(&mut e, 0x425), None);
    e.mem.set_u32(0x011d_a760, cnt);
    e.mem.set_u32(0x011d_9f58, brt);
    assert_eq!(target(&mut e, 0x40b), Some((PC, item_c, 0)));
    let r = menu_target(&mut e, 0x425);
    assert_eq!(inv_ref(&mut e, r), (PC, item_b, 0));
    // Without a player: none.
    e.mem.set_u32(0x011d_ea3c, 0);
    assert_eq!(target(&mut e, 0x40b), None);
}

#[test]
fn selected_item_of_the_menu_on_top() {
    let mut e = setup();
    let item = form(&mut e, Q, 0x28, 0x0000_6201);
    let chest = form(&mut e, Q + 0x100, 0x3a, 0x0000_6202);
    let merchant = form(&mut e, Q + 0x200, 0x3b, 0x0000_6203);
    let shop = form(&mut e, Q + 0x300, 0x3a, 0x0000_6204);
    let perk = form(&mut e, Q + 0x400, 0x56, 0x0000_6205);
    let (quest, note, challenge) = (
        form(&mut e, Q + 0x500, 0x47, 0x0000_6206),
        form(&mut e, Q + 0x600, 0x31, 0x0000_6207),
        form(&mut e, Q + 0x700, 0x71, 0x0000_6208),
    );
    let sel = entry(&mut e, Q + 0x800, item, 2, 0);
    let listed = entry(&mut e, Q + 0x900, item, 5, 0);
    // The container and barter selections hold their own items, so a read of the wrong global
    // shows.
    let item_c = form(&mut e, Q + 0xa00, 0x28, 0x0000_6209);
    let item_b = form(&mut e, Q + 0xb00, 0x28, 0x0000_620a);
    let sel_c = entry(&mut e, Q + 0xc00, item_c, 3, 0);
    let sel_b = entry(&mut e, Q + 0xd00, item_b, 4, 0);
    let top = Q + 0x3000;
    let on_top = |e: &mut Engine, id: u32| {
        e.mem.set_u32(IM + 0xc, 2);
        e.mem.set_u32(IM + 0xd0, top);
        e.mem.set_u32(top + 0x20, id);
    };
    let selected = |e: &mut Engine| {
        let r = selected_item(e);
        if r == 0 {
            None
        } else {
            Some(inv_ref(e, r))
        }
    };
    // Not in menu mode: none.
    let inv = Q + 0x4000;
    e.mem.set_u32(0x011d_9ea4, inv);
    e.mem.set_u32(inv + 0xb8 + 0x10, t(80));
    e.mem.set_u32(0x011d_9ea8, sel);
    assert_eq!(selected_item(&mut e), 0);
    // Inventory: its selection when its item list has a highlight; no menu object: none.
    on_top(&mut e, 0x3ea);
    assert_eq!(selected(&mut e), Some((PC, item, 0)));
    e.mem.set_u32(inv + 0xb8 + 0x10, 0);
    assert_eq!(selected_item(&mut e), 0, "nothing highlighted");
    e.mem.set_u32(inv + 0xb8 + 0x10, t(80));
    e.mem.set_u32(0x011d_9ea4, 0);
    assert_eq!(selected_item(&mut e), 0, "no inventory menu");
    // Stats: the highlighted perk rank's perk.
    on_top(&mut e, 0x3eb);
    let stats = Q + 0x4400;
    e.mem.set_u32(0x011d_ace0, stats);
    let rank = Q + 0x4a00;
    e.mem.set_u32(rank, perk);
    list_box(&mut e, stats + 0x1e0, Q + 0x4c00, &[(t(81), rank)], 0);
    assert_eq!(selected_item(&mut e), 0);
    e.mem.set_u32(stats + 0x1e0 + 0x10, t(81));
    assert_eq!(selected_item(&mut e), 0x0000_6205);
    // Container: its selection, in the container when the right list has the highlight.
    on_top(&mut e, 0x3f0);
    let cnt = Q + 0x5000;
    e.mem.set_u32(0x011d_93f8, cnt);
    e.mem.set_u32(cnt + 0x74, chest);
    e.mem.set_u32(0x011d_93fc, sel_c);
    assert_eq!(selected_item(&mut e), 0);
    e.mem.set_u32(cnt + 0x98 + 0x10, t(82));
    assert_eq!(selected(&mut e), Some((PC, item_c, 0)));
    e.mem.set_u32(cnt + 0x98 + 0x10, 0);
    e.mem.set_u32(cnt + 0xc8 + 0x10, t(82));
    assert_eq!(selected(&mut e), Some((chest, item_c, 0)));
    // Barter: likewise, in the merchant's container. Its selection is set only now, after the
    // container steps.
    on_top(&mut e, 0x41d);
    let brt = Q + 0x5400;
    e.mem.set_u32(0x011d_8fa4, brt);
    e.mem.set_u32(brt + 0x80, merchant);
    e.mem.set_u32(0x011d_8fa8, sel_b);
    assert_eq!(selected_item(&mut e), 0);
    e.mem.set_u32(brt + 0xa8 + 0x10, t(83));
    assert_eq!(selected(&mut e), Some((PC, item_b, 0)));
    e.mem.set_u32(brt + 0xd8 + 0x10, t(83));
    assert_eq!(selected(&mut e), None, "a merchant without a container");
    put_extra(&mut e, merchant + 0x44, Q + 0x5800, 0x3c, shop);
    assert_eq!(selected(&mut e), Some((shop, item_b, 0)));
    // Repair, repair services, item mod: the highlighted list item's entry, the player's.
    for (id, singleton, list) in [
        (0x40b, 0x011d_a75c, 0x5c),
        (0x422, 0x011d_a7f0, 0x68),
        (0x425, 0x011d_9f54, 0x60),
    ] {
        on_top(&mut e, id);
        let m = Q + 0x6000 + 0x400 * (id - 0x40b);
        e.mem.set_u32(singleton, m);
        let items_at = Q + 0x9000 + 0x400 * (id - 0x40b);
        list_box(&mut e, m + list, items_at, &[(t(84), listed)], 0);
        assert_eq!(selected_item(&mut e), 0, "{id:x}");
        e.mem.set_u32(m + list + 0x10, t(84));
        let r = selected_item(&mut e);
        assert_eq!(inv_ref(&mut e, r), (PC, item, 0), "{id:x}");
    }
    // Map: the quest, else the note, else the challenge list; each needs its highlight and an
    // enabled container tile.
    on_top(&mut e, 0x3ff);
    let map = Q + 0xc000;
    e.mem.set_u32(0x011d_a368, map);
    let lists = [
        (0x130, quest, 90),
        (0x160, note, 92),
        (0x1f0, challenge, 94),
    ];
    let mut enabled = Vec::new();
    for (i, (list, form, n)) in lists.into_iter().enumerate() {
        let items_at = Q + 0xd000 + 0x400 * i as u32;
        list_box(&mut e, map + list, items_at, &[(t(n), form)], t(n));
        let parent = tile(&mut e, n + 1, 0, b"list");
        e.mem.set_u32(map + list + 0xc, parent);
        enabled.push(values(&mut e, parent, &[(0xfd0, 0.0, None)])[0]);
    }
    assert_eq!(selected_item(&mut e), 0, "none enabled");
    e.mem.set_f32(enabled[2] + 8, 1.0);
    assert_eq!(selected_item(&mut e), 0x0000_6208);
    e.mem.set_f32(enabled[1] + 8, -2.0);
    assert_eq!(selected_item(&mut e), 0x0000_6207);
    e.mem.set_f32(enabled[0] + 8, 1.0);
    assert_eq!(selected_item(&mut e), 0x0000_6206);
    // A highlighted, enabled quest list ends the chain even when no item has its highlighted
    // tile: the note list (highlighted and enabled too) is not looked at.
    e.mem.set_u32(map + 0x130 + 0x10, t(96));
    assert_eq!(
        selected_item(&mut e),
        0,
        "the quest list highlighted on a tile no item has"
    );
    e.mem.set_u32(map + 0x130 + 0x10, 0);
    assert_eq!(
        selected_item(&mut e),
        0x0000_6207,
        "the quest list not highlighted"
    );
    e.mem.set_u32(map + 0x160 + 0xc, 0);
    assert_eq!(selected_item(&mut e), 0x0000_6208, "no note list tile");
    // Another menu on top: none.
    on_top(&mut e, 0x3f1);
    assert_eq!(selected_item(&mut e), 0);
}

// ---- the world map ----

/// A worldspace's map data: NW and SE cells (x, y), scale, offset, min and max coords.
struct MapData {
    cells: [i16; 4],
    scale: f32,
    offset: [f32; 2],
    bounds: [f32; 4],
}

/// A worldspace at `at` with `map`, its parent and whether it uses the parent's map data.
fn world(e: &mut Engine, at: u32, map: &MapData, parent: u32, parents_map: bool) -> u32 {
    form(e, at, 0x41, at);
    for (i, c) in map.cells.into_iter().enumerate() {
        e.mem.set_u16(at + 0x88 + 2 * i as u32, c as u16);
    }
    e.mem.set_f32(at + 0x90, map.scale);
    e.mem.set_f32(at + 0x94, map.offset[0]);
    e.mem.set_f32(at + 0x98, map.offset[1]);
    e.mem.set_u32(at + 0x9c, 0xdead_beef);
    for (i, c) in map.bounds.into_iter().enumerate() {
        e.mem.set_f32(at + 0xa0 + 4 * i as u32, c);
    }
    e.mem.set_u32(at + 0x70, parent);
    e.mem
        .set_u16(at + 0x4e, if parents_map { 0xff04 } else { 0xfffb });
    at
}

/// JIP's numbers for a map with `cells` (after its `paddw`: NW y and SE x plus 1), the position
/// already moved: (moved - NW * 4096) * rcpss(span * 4096) * 0.796875 + 0.1015625.
fn expected(cells: [i16; 4], moved: [f32; 2]) -> [f64; 2] {
    let nw = [i32::from(cells[0]), i32::from(cells[1].wrapping_add(1))];
    let se = [i32::from(cells[2].wrapping_add(1)), i32::from(cells[3])];
    [0, 1].map(|i| {
        let min = (nw[i] << 12) as f32;
        let size = processor_rcp(((se[i] - nw[i]) << 12) as f32) * 0.796_875;
        f64::from((moved[i] - min) * size + 0.101_562_5)
    })
}

#[test]
fn world_map_pos_mults_of_a_reference() {
    let mut e = setup();
    c2_locals(&mut e, 2);
    let refr = form(&mut e, Q, 0x3b, 0x0000_6301);
    e.mem.set_f32(refr + 0x30, 12_345.5);
    e.mem.set_f32(refr + 0x34, -6_789.25);
    let pos = |e: &mut Engine| {
        call_on(e, 0x2730, refr, &[var(1), var(2)]);
        [local_value(e, 1), local_value(e, 2)]
    };
    // No cell (and no persistent cell): the variables stay.
    assert_eq!(pos(&mut e), [-1.0, -1.0]);
    // An interior cell (no worldspace): they stay.
    let cell = Q + 0x100;
    e.mem.set_u32(refr + 0x40, cell);
    assert_eq!(pos(&mut e), [-1.0, -1.0]);
    // A root world, scale 1: the position plus the offset.
    let root_map = MapData {
        cells: [-26, 25, 24, -24],
        scale: 1.0,
        offset: [100.0, -50.0],
        bounds: [1.0, 2.0, 3.0, 4.0],
    };
    let root = world(&mut e, Q + 0x400, &root_map, 0, false);
    e.mem.set_u32(cell + 0xc0, root);
    let want = expected(root_map.cells, [12_345.5 + 100.0, -6_789.25 - 50.0]);
    assert_eq!(pos(&mut e), want);
    // The variables in their order.
    call_on(&mut e, 0x2730, refr, &[var(2), var(1)]);
    assert_eq!([local_value(&e, 2), local_value(&e, 1)], want);
    // A child world using its parent's map: the root's cells, the child's own scale and offset
    // (scale not 1: (min + max) * (0.5 - scale / 2) + offset, then position * scale + that).
    let child_map = MapData {
        cells: [0, 0, 1, -1],
        scale: 0.25,
        offset: [10.0, 20.0],
        // x and y sums differ (4000 and 6000), so a swapped axis shows.
        bounds: [-1000.0, -3000.0, 5000.0, 9000.0],
    };
    let child = world(&mut e, Q + 0x600, &child_map, root, true);
    e.mem.set_u32(cell + 0xc0, child);
    let half = 0.5f32 - 0.25 * 0.5;
    let moved = [
        12_345.5f32 * 0.25 + ((-1000.0f32 + 5000.0) * half + 10.0),
        -6_789.25f32 * 0.25 + ((-3000.0f32 + 9000.0) * half + 20.0),
    ];
    assert_eq!(pos(&mut e), expected(root_map.cells, moved));
    // Two levels up: the grandparent's map when both pass it up; the first world that keeps its
    // own map stops the walk.
    let middle_map = MapData {
        cells: [-2, 3, 5, -6],
        scale: 1.0,
        offset: [0.0; 2],
        bounds: [0.0; 4],
    };
    let middle = world(&mut e, Q + 0x800, &middle_map, root, true);
    e.mem.set_u32(child + 0x70, middle);
    assert_eq!(pos(&mut e), expected(root_map.cells, moved));
    e.mem.set_u16(middle + 0x4e, 0xfffb);
    assert_eq!(pos(&mut e), expected(middle_map.cells, moved));
    // The child keeping its own map: its own cells.
    e.mem.set_u16(child + 0x4e, 0);
    assert_eq!(pos(&mut e), expected(child_map.cells, moved));
    // A persistent reference (no parent cell): its `ExtraPersistentCell`'s cell.
    e.mem.set_u32(refr + 0x40, 0);
    let persistent = Q + 0x1000;
    e.mem.set_u32(persistent + 0xc0, root);
    put_extra(&mut e, refr + 0x44, Q + 0x1100, 0x0c, persistent);
    assert_eq!(pos(&mut e), want);
    // A persistent cell extra that holds no cell: the variables stay (JIP's `GetParentWorld`
    // tests it for null).
    e.mem.set_f64(C2_LOCALS + 0x108, -1.0);
    e.mem.set_f64(C2_LOCALS + 0x108 + 0x10, -1.0);
    e.mem.set_u32(Q + 0x1100 + 0xc, 0);
    assert_eq!(pos(&mut e), [-1.0, -1.0]);
    // Not on a reference: nothing.
    e.mem.set_f64(C2_LOCALS + 0x108, 7.0);
    call(&mut e, 0x2730, &[var(1), var(2)]);
    assert_eq!(local_value(&e, 1), 7.0);
}
