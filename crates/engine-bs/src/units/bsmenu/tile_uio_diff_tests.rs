//! UI Organizer's XML reader (crate `uio`, `uio::build::parse`, translated from UIO 2.30's
//! `ResolveXMLFileHook`/`ResolveTrait`) against the game's own reader it replaces
//! ([`tile_parse_file`], `00A01E20`, with `Tile::TileTemplate::AddPair` [`template_add_pair`]):
//! the same text through both, on engines with the same doubles for what lies outside both (the
//! heap, the text table and its user traits, the game settings, `BSStringT`, the list nodes), and
//! the items of every template they make compared (type, number, text, id).
//!
//! The texts are as both readers get them in the game: after `Menu::CompressAndStoreXML`, so with
//! no white space between tags. They are written by hand in the shape of the vanilla menus (no
//! game file is shipped). Trait and action ids are this test's text table's.
//!
//! Where UIO's reader is EXPECTED to differ from the game's (each one read in UIO's source,
//! `ui_organizer.h` 2.30):
//!
//! 1. The text of a start or end item (trait 4/5, action 2/3, tile end 7): the game copies the
//!    tag's text into every item (`00A0B950`); UIO's `ResolveTrait` sets no text for an id
//!    (data type 3), so a new item has none and one from the pool keeps its old text. Not
//!    compared ([`same_items`]).
//! 2. The text of a number: the game sets it empty (`EMPTY_NAME`), UIO frees it (null). Both read
//!    as no text; compared as equal.
//! 3. A trait constant `&name;` as a value: the game keeps its text with its number (`&true;`,
//!    2); UIO's `ResolveTrait` takes it as a number (data type 1) and frees the text
//!    ([`a_trait_constant_keeps_its_text_only_in_the_games_reader`]).
//! 4. A self-closed tag with no attribute `<x/>`: the game makes its start and its end; UIO's scan
//!    sees only the `/` and makes the end ([`a_self_closed_tag_with_no_attribute`]).
//! 5. A game setting `&-fName;` / `&-bName;`: the game (`Interface::TestConstantForGameSettings`,
//!    `007073D0`) prints a float with `"%i"` (the low word of the double: 0 for most) and a bool
//!    as `&true;` / `&false;`; UIO's `GetGMSTValue` gives the float itself and nothing for a bool
//!    ([`float_and_bool_settings`]). `&-sName;` and `&-iName;` agree.
//! 6. The id of a value item: the game's is the number's integer, UIO's `eNone`. A value is folded
//!    into its trait in every vanilla shape, so it is seen only in a value outside a trait.

use super::*;

/// The doubles' map functions (free addresses).
const TEXT_HASH: u32 = 0x0600_0004;
const TEXT_EQUAL: u32 = 0x0600_0008;
const INT_HASH: u32 = 0x0600_0104;
const INT_EQUAL: u32 = 0x0600_0108;
/// UIO's user traits by id.
const TRAITS_BY_ID: u32 = 0x011f_3308;
const BUCKETS: u32 = 37;
/// UIO's game settings owner (`*0x11C8048`, the map at +0x10C).
const SETTINGS_OWNER: u32 = 0x011c_8048;
const SETTINGS_MAP: u32 = 0x10c;

/// The text table this test gives both readers (the uio crate's test names, and more in the
/// game's ranges: tile types 901-908, actions 2000-2025, traits 4001-4125).
const NAMES: &[(&str, u32)] = &[
    ("rect", 0x385),
    ("image", 0x386),
    ("text", 0x387),
    ("menu", 0x38c),
    ("template", 999),
    ("name", 0xbba),
    ("src", 0xbbb),
    ("trait", 0xbbc),
    ("x", 0xfa1),
    ("y", 0xfa2),
    ("visible", 0xfa3),
    ("width", 0xfa5),
    ("height", 0xfa6),
    ("alpha", 0xfa9),
    ("filename", 0xfb9),
    ("string", 0xfdf),
    ("font", 0xfe0),
    ("justify", 0xfe1),
    ("copy", 0x7d0),
    ("add", 0x7d1),
    ("sub", 0x7d2),
    ("and", 0x7e2),
    ("not", 0x7e4),
    ("&true;", 2),
    ("&false;", 1),
    ("&left;", 1),
    ("&center;", 2),
];

fn lower(e: &Engine, at: u32) -> Vec<u8> {
    e.mem.cstr(at).to_ascii_lowercase()
}

fn hash(text: &[u8]) -> u32 {
    text.iter().fold(0u32, |h, c| {
        h.wrapping_mul(31)
            .wrapping_add(c.to_ascii_lowercase() as u32)
    }) % BUCKETS
}

/// A text map (`NiTMap`: vtable, bucket count, buckets, count) at `map`.
fn make_map(e: &mut Engine, map: u32) {
    let vtable = e.mem.alloc(0x20);
    e.mem.set_u32(vtable + 4, TEXT_HASH);
    e.mem.set_u32(vtable + 8, TEXT_EQUAL);
    e.mem.set_u32(map, vtable);
    e.mem.set_u32(map + 4, BUCKETS);
    let buckets = e.mem.alloc(BUCKETS * 4);
    e.mem.set_u32(map + 8, buckets);
    e.mem.set_u32(map + 0xc, 0);
}

fn map_put(e: &mut Engine, map: u32, key: &[u8], data: u32) {
    let k = e.mem.alloc(key.len() as u32 + 1);
    e.mem.set_cstr(k, key);
    let slot = e.mem.u32(map + 8) + hash(key) * 4;
    let entry = e.mem.alloc(0xc);
    e.mem.set_u32(entry, e.mem.u32(slot));
    e.mem.set_u32(entry + 4, k);
    e.mem.set_u32(entry + 8, data);
    e.mem.set_u32(slot, entry);
    e.mem.set_u32(map + 0xc, e.mem.u32(map + 0xc) + 1);
}

fn map_find(e: &Engine, map: u32, key: &[u8]) -> Option<u32> {
    let key = key.to_ascii_lowercase();
    let mut entry = e.mem.u32(e.mem.u32(map + 8) + hash(&key) * 4);
    while entry != 0 {
        if lower(e, e.mem.u32(entry + 4)) == key {
            return Some(e.mem.u32(entry + 8));
        }
        entry = e.mem.u32(entry);
    }
    None
}

/// `BSStringT::Set` as the game's does it: a copy of the text (or none).
fn string_set(e: &mut Engine, string: u32, text: u32) {
    if text == 0 {
        e.mem.set_u32(string, 0);
        e.mem.set_u32(string + 4, 0);
        return;
    }
    let t = e.mem.cstr(text);
    let copy = e.mem.alloc(t.len() as u32 + 1);
    e.mem.set_cstr(copy, &t);
    e.mem.set_u32(string, copy);
    e.mem.set_u16(string + 4, t.len() as u16);
    e.mem.set_u16(string + 6, t.len() as u16);
}

/// An engine both readers run on: the game's reader's own callees from this file (the storage,
/// its templates, `AddPair` and its item pool) translated, the rest stood in.
fn engine(settings: &[(&str, u32)]) -> Engine {
    let (mut e, _, _) = super::tests::pair_engine();
    e.map(0x011c_8000, 0x1000);
    for (addr, f) in funcs() {
        if [0x00a0_a410, 0x00a0_ac80, 0x00a0_ae70, 0x00a0_af10].contains(&addr) {
            e.register(addr, f);
        }
    }
    // The game's heap (UIO's `GAME_HEAP_ALLOC`/`FREE` too).
    e.register(MEMORY_ALLOCATE, |e, a| e.mem.alloc(a[1].max(1)).into_ret());
    e.register(MEMORY_DEALLOCATE, |e, a| {
        if a[1] != 0 {
            e.mem.free(a[1]);
        }
        Ret::default()
    });
    e.register(STRING_SET, |e, a| {
        string_set(e, a[0], a[1]);
        Ret::default()
    });
    e.register(STRING_ASSIGN, |e, a| {
        let text = e.mem.u32(a[1]);
        string_set(e, a[0], text);
        Ret::default()
    });
    e.register(TEMPLATE_ITEM_CONSTRUCT, |e, a| {
        let item = a[0];
        e.mem.set_u32(item, a[1]);
        e.mem.set_u32(item + 4, a[2]);
        e.mem.set_u32(item + 8, 0);
        e.mem.set_u32(item + 0xc, 0);
        string_set(e, item + 8, a[3]);
        e.mem.set_u32(item + 0x10, a[4]);
        e.mem.set_u32(item + 0x14, a[5]);
        item.into_ret()
    });
    // A template: its name copied (a fixed string in the game), its owner, an empty list.
    e.register(TEMPLATE_CONSTRUCT, |e, a| {
        let name = if a[1] == 0 {
            0
        } else {
            let t = e.mem.cstr(a[1]);
            let copy = e.mem.alloc(t.len() as u32 + 1);
            e.mem.set_cstr(copy, &t);
            copy
        };
        e.mem.set_u32(a[0], name);
        e.mem.set_u32(a[0] + 4, a[2]);
        a[0].into_ret()
    });
    // The text table (`0x11F32F4`), which UIO walks itself and the game asks through its find
    // (`00A1E380`) and add (`00A0C900`); a user name starts with `_` or `&_` (`00A0BB00`).
    e.register(TEXT_HASH, |e, a| hash(&e.mem.cstr(a[1])).into_ret());
    e.register(TEXT_EQUAL, |e, a| {
        ((lower(e, a[1]) == lower(e, a[2])) as u32).into_ret()
    });
    make_map(&mut e, TEXT_TABLE);
    // The user traits by id (`0x11F3308`, integer keys), which UIO fills itself (the game through
    // its trait extra data map).
    e.register(INT_HASH, |_, a| (a[1] % BUCKETS).into_ret());
    e.register(INT_EQUAL, |_, a| ((a[1] == a[2]) as u32).into_ret());
    make_map(&mut e, TRAITS_BY_ID);
    let vtable = e.mem.u32(TRAITS_BY_ID);
    e.mem.set_u32(vtable + 4, INT_HASH);
    e.mem.set_u32(vtable + 8, INT_EQUAL);
    for (name, id) in NAMES {
        map_put(&mut e, TEXT_TABLE, name.as_bytes(), *id);
    }
    e.register(TEXT_TABLE_FIND, |e, a| {
        let key = e.mem.cstr(a[1]);
        match map_find(e, a[0], &key) {
            Some(id) => {
                e.mem.set_u32(a[2], id);
                1u32.into_ret()
            }
            None => 0u32.into_ret(),
        }
    });
    e.register(TEXT_TABLE_ADD, |e, a| {
        let key = e.mem.cstr(a[1]);
        if map_find(e, a[0], &key).is_none() {
            map_put(e, a[0], &key, a[2]);
        }
        Ret::default()
    });
    e.register(TEXT_IS_USER_NAME, |e, a| {
        let t = e.mem.cstr(a[0]);
        ((t.first() == Some(&b'_') || t.starts_with(b"&_")) as u32).into_ret()
    });
    e.set_global(TEXT_TABLE_USER_NEXT_ID, 10000i32);
    // The game settings: one map, read by UIO directly and by the game's
    // `TestConstantForGameSettings` (`007073D0`, as translated in engine-fallout-interface:
    // `&` and `-` skipped, a `;` cut; `s` the text, `i` and `f` printed with `"%i"` (a float
    // as a double: its low word), `b` `&true;` / `&false;`).
    let owner = e.mem.alloc(0x200);
    e.mem.set_u32(SETTINGS_OWNER, owner);
    make_map(&mut e, owner + SETTINGS_MAP);
    for (name, value) in settings {
        let setting = e.mem.alloc(0xc);
        // A text setting's value is a text made from the number given.
        let value = if name.starts_with('s') {
            let text = format!("Text {value}");
            let at = e.mem.alloc(text.len() as u32 + 1);
            e.mem.set_cstr(at, text.as_bytes());
            at
        } else {
            *value
        };
        e.mem.set_u32(setting + 4, value);
        map_put(&mut e, owner + SETTINGS_MAP, name.as_bytes(), setting);
    }
    e.register(INTERFACE_TEST_CONSTANT_FOR_GAME_SETTINGS, |e, a| {
        let mut name = e.mem.cstr(a[0]);
        if name.first() == Some(&b'&') {
            name.remove(0);
        }
        if name.first() == Some(&b'-') {
            name.remove(0);
        }
        if name.last() == Some(&b';') {
            name.pop();
        }
        let map = e.mem.u32(SETTINGS_OWNER) + SETTINGS_MAP;
        let Some(setting) = map_find(e, map, &name) else {
            return 0u32.into_ret();
        };
        let value = e.mem.u32(setting + 4);
        let text = match name.first().map(|c| c | 0x20) {
            Some(b's') => e.mem.cstr(value),
            Some(b'i') => (value as i32).to_string().into_bytes(),
            Some(b'f') => {
                let double = f32::from_bits(value) as f64;
                (double.to_bits() as u32 as i32).to_string().into_bytes()
            }
            Some(b'b') => {
                if value & 0xff == 1 {
                    b"&true;".to_vec()
                } else {
                    b"&false;".to_vec()
                }
            }
            _ => return 0u32.into_ret(),
        };
        e.mem.set_cstr(a[1], &text);
        1u32.into_ret()
    });
    e
}

/// One item: type, number, text (none for a null pointer), id.
#[derive(Clone, Debug, PartialEq)]
struct Item {
    kind: i32,
    num: f32,
    text: Option<String>,
    id: u32,
}

fn template_items(e: &Engine, template: u32) -> Vec<Item> {
    let mut out = vec![];
    let mut node = e.mem.u32(template + 8);
    while node != 0 {
        let at = e.mem.u32(node + 8);
        let data = e.mem.u32(at + 8);
        out.push(Item {
            kind: e.mem.i32(at),
            num: e.mem.f32(at + 4),
            text: (data != 0).then(|| String::from_utf8_lossy(&e.mem.cstr(data)).into_owned()),
            id: e.mem.u32(at + 0x10),
        });
        node = e.mem.u32(node);
    }
    out
}

/// Every template of a storage: the main one, then the sub-templates in list order, each with
/// its name.
fn templates(e: &Engine, storage: u32) -> Vec<(Option<String>, Vec<Item>)> {
    let mut out = vec![(None, template_items(e, e.mem.u32(storage)))];
    let mut node = storage + 4;
    while node != 0 && e.mem.u32(node) != 0 {
        let t = e.mem.u32(node);
        let name = String::from_utf8_lossy(&e.mem.cstr(e.mem.u32(t))).into_owned();
        out.push((Some(name), template_items(e, t)));
        node = e.mem.u32(node + 4);
    }
    out
}

/// The game's reader on `text` (`Tile::ParseFile` with a buffer).
fn games(text: &str, settings: &[(&str, u32)]) -> Vec<(Option<String>, Vec<Item>)> {
    let mut e = engine(settings);
    let path = e.mem.alloc(16);
    e.mem.set_cstr(path, b"diff.xml");
    let buffer = e.mem.alloc(text.len() as u32 + 1);
    e.mem.set_cstr(buffer, text.as_bytes());
    let storage = tile_parse_file(&mut e, Ptr::new(path), Ptr::new(buffer), text.len() as u32);
    assert!(!storage.is_null(), "the game's reader refused {text:?}");
    assert_eq!(e.global::<u8>(XML_ERROR_FLAG), 0, "{text:?}");
    templates(&e, storage.addr())
}

/// UIO's reader on `text`.
fn uios(text: &str, settings: &[(&str, u32)]) -> Vec<(Option<String>, Vec<Item>)> {
    let mut e = engine(settings);
    let storage = uio::build::parse(&mut e, text.as_bytes());
    assert_ne!(storage, 0);
    templates(&e, storage)
}

/// The items as compared: texts of start and end items left out (expected difference 1), an
/// empty text read as none (2).
fn same_items(t: &[(Option<String>, Vec<Item>)]) -> Vec<(Option<String>, Vec<Item>)> {
    t.iter()
        .map(|(name, items)| {
            let items = items
                .iter()
                .map(|i| Item {
                    text: if [2, 3, 4, 5, 7].contains(&i.kind) {
                        None
                    } else {
                        i.text.clone().filter(|t| !t.is_empty())
                    },
                    ..i.clone()
                })
                .collect();
            (name.clone(), items)
        })
        .collect()
}

fn assert_same(text: &str) {
    let settings = [("sHello", 7), ("iSeven", 7)];
    let game = games(text, &settings);
    let uio = uios(text, &settings);
    assert!(
        !game.iter().all(|(_, i)| i.is_empty()),
        "nothing parsed: {text:?}"
    );
    assert_eq!(same_items(&uio), same_items(&game), "{text:?}");
}

#[test]
fn both_readers_make_the_same_items_for_vanilla_shapes() {
    for text in [
        // A tile with a name and number traits.
        r#"<rect name="a"><x>5</x><y>-1.5</y><alpha>0</alpha></rect>"#,
        // Nested tiles; traits built from links (`<copy src trait/>`) and actions inside them.
        concat!(
            r#"<menu name="M"><rect name="r"><visible><copy src="parent()" trait="visible"/>"#,
            r#"</visible><x><copy src="me()" trait="y"/><add>4</add><sub>1.25</sub></x>"#,
            r#"<image name="i"><width><copy src="parent()" trait="width"/></width></image>"#,
            r#"</rect></menu>"#
        ),
        // Text values, a setting's text and number, a file name.
        concat!(
            r#"<text name="t"><string>&-sHello;</string><font>&-iSeven;</font>"#,
            r#"<filename>interface\hud\a.dds</filename><string>Hello there</string></text>"#
        ),
        // A template definition and its close, then a tile after it; two definitions.
        concat!(
            r#"<template name="T"><rect name="t"><x>1</x></rect></template>"#,
            r#"<rect name="u"></rect><template name="U"><image name="v"></image></template>"#
        ),
        // A self-closed tag with attributes, and a tile closed by its own tag.
        r#"<rect name="s"><image name="i"/><text name="j"></text></rect>"#,
        // User traits (`_name`, `_name_3`), as values and as link targets.
        concat!(
            r#"<rect name="w"><_user>3</_user><_count_3>7</_count_3>"#,
            r#"<x><copy src="me()" trait="_user"/></x></rect>"#
        ),
        // Shapes of the vanilla HUD and menus: a root menu, nested tiles, actions inside
        // actions (`<and>`, `<not>`), a link to a user trait of another tile.
        concat!(
            r#"<menu name="HUDMainMenu"><rect name="Root"><x>0</x><y>0</y>"#,
            r#"<width><copy src="screen()" trait="width"/></width>"#,
            r#"<text name="t1"><string>&-sHello;</string><font>3</font>"#,
            r#"<visible><copy src="parent()" trait="visible"/><and><copy src="Root" "#,
            r#"trait="_on"/></and><not>0</not></visible></text></rect></menu>"#
        ),
        concat!(
            r#"<rect name="list"><height><copy src="child(item)" trait="height"/>"#,
            r#"<add>8</add></height><image name="item"><filename>a.dds</filename>"#,
            r#"<width>32</width><height>32</height><x><copy src="sibling()" trait="x"/>"#,
            r#"<add><copy src="sibling()" trait="width"/></add></x></image></rect>"#
        ),
    ] {
        assert_same(text);
    }
}

/// Expected difference 3: `&true;` as a value.
#[test]
fn a_trait_constant_keeps_its_text_only_in_the_games_reader() {
    let text = "<visible>&true;</visible>";
    let game = games(text, &[]);
    let uio = uios(text, &[]);
    let item = |text: Option<&str>| Item {
        kind: 8,
        num: 2.0,
        text: text.map(str::to_string),
        id: 0xfa3,
    };
    assert_eq!(game[0].1, vec![item(Some("&true;"))]);
    assert_eq!(uio[0].1, vec![item(None)]);
}

/// Expected difference 4: `<x/>`.
#[test]
fn a_self_closed_tag_with_no_attribute() {
    let text = "<x/>";
    let kinds =
        |t: &[(Option<String>, Vec<Item>)]| -> Vec<i32> { t[0].1.iter().map(|i| i.kind).collect() };
    assert_eq!(kinds(&games(text, &[])), vec![4, 5]);
    assert_eq!(kinds(&uios(text, &[])), vec![5]);
}

/// Expected difference 5: float and bool settings.
#[test]
fn float_and_bool_settings() {
    let settings = [("fHalf", 0.5f32.to_bits()), ("bFlag", 1)];
    let text = "<x>&-fHalf;</x><y>&-bFlag;</y>";
    let game = games(text, &settings);
    let uio = uios(text, &settings);
    let num = |t: &[(Option<String>, Vec<Item>)]| -> Vec<(i32, f32)> {
        t[0].1.iter().map(|i| (i.kind, i.num)).collect()
    };
    // The game: 0.5 printed with "%i" is "0"; the bool's `&true;` is the constant 2.
    assert_eq!(num(&game), vec![(8, 0.0), (8, 2.0)]);
    // UIO: the float itself; nothing for the bool (its start and end stay).
    assert_eq!(num(&uio), vec![(8, 0.5), (4, 4002.0), (5, 4002.0)]);
}

/// Expected difference 6: a value outside a trait keeps its own id.
#[test]
fn a_value_outside_a_trait() {
    let text = r#"<rect name="a">5</rect>"#;
    let game = games(text, &[]);
    let uio = uios(text, &[]);
    let ids = |t: &[(Option<String>, Vec<Item>)]| -> Vec<(i32, u32)> {
        t[0].1.iter().map(|i| (i.kind, i.id)).collect()
    };
    assert_eq!(ids(&game), vec![(6, 0x385), (0xbb9, 5), (7, 0x385)]);
    assert_eq!(
        ids(&uio),
        vec![(6, 0x385), (0xbb9, 0x8000_0000), (7, 0x385)]
    );
}
