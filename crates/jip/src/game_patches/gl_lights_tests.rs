//! Group GL's game patches, part 2: JIP's light data (H03, H04, H05, H06, H07, H12), from JIP's
//! assembly case by case.

use super::*;
use crate::test_support::engine;
use std::cell::RefCell;

/// A test page for forms, lights, nodes and the editor id map.
const AT: u32 = 0x0660_0000;

thread_local! {
    /// The calls the stand-ins saw: (address, words).
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
    /// The answers `GetRandomInt` gives, in order.
    static RANDOMS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn log(addr: u32, words: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, words.to_vec())));
}

fn calls(addr: u32) -> Vec<Vec<u32>> {
    CALLS.with(|c| {
        c.borrow()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    })
}

fn addresses() -> Vec<u32> {
    CALLS.with(|c| c.borrow().iter().map(|(a, _)| *a).collect())
}

fn randoms(values: &[u32]) {
    RANDOMS.with(|r| *r.borrow_mut() = values.iter().rev().copied().collect());
}

/// JIP installed (its own patches in), the pages mapped, every game function the part calls a
/// logged stand-in: the random numbers from [`randoms`], the allocator a real block, the
/// `ShadowSceneLight` constructor its `this`.
fn world() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    RANDOMS.with(|r| r.borrow_mut().clear());
    let mut e = engine();
    e.map(AT, 0x1_0000);
    for page in [0x011c_4000, 0x011c_5000, 0x011f_6000, 0x011f_9000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    crate::install_as(&mut e, true);
    for addr in [
        LIGHT_DESTRUCTOR,
        SIZED_FREE,
        REMOVE_EXTRA_AT,
        GAME_ANIMATED_LIGHTS,
        LOCK_ENTER,
        LOCK_LEAVE,
        SCENE_ADD_LIGHT,
    ] {
        e.register_double(addr, move |_, a| {
            log(addr, a);
            Ret::default()
        });
    }
    e.register(GET_RANDOM_INT, |_, a| {
        log(GET_RANDOM_INT, a);
        RANDOMS
            .with(|r| r.borrow_mut().pop().expect("a random number"))
            .into_ret()
    });
    e.register(ALLOCATE, |e, a| {
        log(ALLOCATE, a);
        e.mem.alloc(a[0]).into_ret()
    });
    e.register(SCENE_LIGHT_CONSTRUCT, |_, a| {
        log(SCENE_LIGHT_CONSTRUCT, a);
        a[0].into_ret()
    });
    e
}

/// A point light of `size` bytes (its vtable `NiPointLight`'s).
fn point_light(e: &mut Engine) -> u32 {
    let light = e.mem.alloc(POINT_LIGHT_SIZE);
    e.mem.set_u32(light, POINT_LIGHT_VTABLE);
    light
}

/// A node (slot +0xC the shared `return this`) with `children`.
fn node(e: &mut Engine, children: &[u32]) -> u32 {
    let table = e.mem.alloc(0x10);
    e.mem.set_u32(table + 0xc, RETURN_THIS);
    let node = e.mem.alloc(0xb0);
    e.mem.set_u32(node, table);
    let array = e.mem.alloc(4 * children.len().max(1) as u32);
    for (i, &child) in children.iter().enumerate() {
        e.mem.set_u32(array + 4 * i as u32, child);
        if child != 0 {
            e.mem.set_u32(child + PARENT, node);
        }
    }
    e.mem.set_u32(node + CHILDREN, array);
    e.mem.set_u16(node + CHILD_COUNT, children.len() as u16);
    node
}

/// A guest text.
fn text(e: &mut Engine, s: &str) -> u32 {
    let at = e.mem.alloc(s.len() as u32 + 1);
    e.mem.write(at, s.as_bytes());
    e.mem.set_u8(at + s.len() as u32, 0);
    at
}

/// An extra data of `vtable` named `name` holding `value` (+0xC).
fn extra(e: &mut Engine, vtable: u32, name: &str, value: u32) -> u32 {
    let extra = e.mem.alloc(0x10);
    e.mem.set_u32(extra, vtable);
    let name = text(e, name);
    e.mem.set_u32(extra + EXTRA_NAME, name);
    e.mem.set_u32(extra + STRING_EXTRA_TEXT, value);
    extra
}

fn set_extras(e: &mut Engine, object: u32, extras: &[u32]) {
    let array = e.mem.alloc(4 * extras.len().max(1) as u32);
    for (i, &x) in extras.iter().enumerate() {
        e.mem.set_u32(array + 4 * i as u32, x);
    }
    e.mem.set_u32(object + EXTRA_DATA, array);
    e.mem.set_u16(object + EXTRA_COUNT, extras.len() as u16);
}

/// Stand-in functions of the editor id map's vtable.
const MAP_BUCKET: u32 = 0x00e1_0004;
const MAP_EQUAL: u32 = 0x00e1_0008;
const MAP_SET: u32 = 0x00e1_000c;

/// The game's editor id map with one bucket (every text in it), its compare a text compare,
/// its set storing the key and the form; `entries` are (text, form), the first at the head.
fn editor_map(e: &mut Engine, entries: &[(&str, u32)]) -> u32 {
    let table = e.mem.alloc(0x10);
    e.mem.set_u32(table + 4, MAP_BUCKET);
    e.mem.set_u32(table + 8, MAP_EQUAL);
    e.mem.set_u32(table + 0xc, MAP_SET);
    let map = e.mem.alloc(0x14);
    e.mem.set_u32(map, table);
    let buckets = e.mem.alloc(4);
    e.mem.set_u32(map + 4, 1);
    e.mem.set_u32(map + 8, buckets);
    let mut head = 0;
    for &(key, form) in entries.iter().rev() {
        let entry = e.mem.alloc(0xc);
        let key = text(e, key);
        e.mem.set_u32(entry, head);
        e.mem.set_u32(entry + 4, key);
        e.mem.set_u32(entry + 8, form);
        head = entry;
    }
    e.mem.set_u32(buckets, head);
    e.mem.set_u32(map + 0xc, entries.len() as u32);
    e.mem.set_u32(EDITOR_IDS_MAP, map);
    e.register(MAP_BUCKET, |_, a| {
        log(MAP_BUCKET, a);
        Ret::default()
    });
    e.register(MAP_EQUAL, |e, a| {
        let read = |e: &Engine, mut at: u32| {
            let mut out = Vec::new();
            while e.mem.u8(at) != 0 {
                out.push(e.mem.u8(at));
                at += 1;
            }
            out
        };
        u32::from(read(e, a[1]) == read(e, a[2])).into_ret()
    });
    // The map keeps its own copy of the text (the node's key).
    e.register(MAP_SET, |e, a| {
        log(MAP_SET, a);
        let mut len = 0;
        while e.mem.u8(a[2] + len) != 0 {
            len += 1;
        }
        let copy = e.mem.alloc(len + 1);
        for i in 0..=len {
            let b = e.mem.u8(a[2] + i);
            e.mem.set_u8(copy + i, b);
        }
        e.mem.set_u32(a[1] + 4, copy);
        e.mem.set_u32(a[1] + 8, a[3]);
        Ret::default()
    });
    e.register(MAP_NEW_NODE, |e, a| {
        log(MAP_NEW_NODE, a);
        e.mem.alloc(0xc).into_ret()
    });
    map
}

// ---- rows and install ----

/// JIP's `InitJIPHooks` addresses, in address order: H06 `0x447155`, H04 `0x465488`, H03's
/// twelve, `Clone3D`'s `0x50EF46`, H12's two calls and H05 `0xA68D6B`; H12's wrap `0x50DE20`.
#[test]
fn gl2_rows_are_jips_addresses() {
    assert_eq!(
        SITES,
        [
            0x0044_7155,
            0x0046_5488,
            0x0050_da73,
            0x0050_ef46,
            0x0079_2dc0,
            0x007c_6f86,
            0x007f_c4da,
            0x007f_d49f,
            0x0080_ea39,
            0x0081_c095,
            0x008c_7c4e,
            0x008c_7e18,
            0x009c_6cb6,
            0x009c_6d3d,
            0x00a6_8d6b,
            0x00a7_d6e2,
            0x00a7_d7c3,
            0x00b5_cd2a,
        ]
    );
    assert!(SITES.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(super::super::AREA.jip_wraps, [0x0050_de20]);
    assert_eq!(
        [GET_EDITOR_ID_SLOT, SET_EDITOR_ID_SLOT, DESTROY_SLOT],
        [0x0102_8ee4 + 0x130, 0x0102_8ee4 + 0x134, 0x0109_dd0c]
    );
}

/// JIP not installed: no seam answers, the wrap is not made (the exe's `0050de20` runs), the
/// three slots keep what they held. Installed (the fixes row off): every seam answers, the slots
/// hold the routines, the wrap is JIP's. (The lead's decision, review of #700: JIP's light data is
/// JIP's own behaviour, in with JIP installed in every preset, the original-behaviour one too.)
#[test]
fn gl2_goes_in_with_jip_only() {
    for (installed, row) in [(false, true), (true, false)] {
        let mut e = engine();
        e.register(GAME_ANIMATED_LIGHTS, |_, _| Ret::default());
        crate::install_as(&mut e, installed);
        crate::game_patches::set_row(&mut e, row);
        for &site in &SITES {
            assert_eq!(
                e.site(site, &[0, 0, 0, 0, 0]).is_some(),
                installed,
                "{site:08x}"
            );
        }
        let expected = |routine: u32| if installed { routine } else { 0 };
        assert_eq!(
            e.mem.u32(GET_EDITOR_ID_SLOT),
            expected(GET_EDITOR_ID_ROUTINE)
        );
        assert_eq!(
            e.mem.u32(SET_EDITOR_ID_SLOT),
            expected(SET_EDITOR_ID_ROUTINE)
        );
        assert_eq!(e.mem.u32(DESTROY_SLOT), expected(DESTROY_ROUTINE));
        let form = 0x0118_e000;
        e.mem.set_u32(form + FORM_FLAGS, 0);
        let answer = e.call(0x0050_de20, &args![form, form, 0u32]).u32();
        assert_eq!(answer, if installed { 0 } else { 0x5a60 });
    }
}

// ---- H03, H05, H04 sizes ----

/// H03: each of the twelve places answers 0x110; H05 answers (the word copied); H04 0xCC.
#[test]
fn h03_h05_h04_answer_jips_sizes() {
    let mut e = world();
    for slot in POINT_LIGHT_SIZES {
        assert_eq!(e.site(slot.addr(), &[]), Some(0x110), "{:08x}", slot.addr());
    }
    assert_eq!(e.site(CLONE_WORD.addr(), &[]), Some(1));
    assert_eq!(e.site(LIGHT_FORM_SIZE.addr(), &[]), Some(0xcc));
}

// ---- H04 editor ids ----

/// `GetEditorID` through the light form's vtable slot: the word +0xC8, or the empty text.
#[test]
fn h04_get_editor_id_answers_the_kept_text() {
    let mut e = world();
    let form = e.mem.alloc(LIGHT_FORM_BYTES);
    assert_eq!(
        e.call(GET_EDITOR_ID_ROUTINE, &args![form]).u32(),
        EMPTY_TEXT
    );
    e.mem.set_u32(form + FORM_EDITOR_ID, 0x1234_5678);
    assert_eq!(
        e.call(GET_EDITOR_ID_ROUTINE, &args![form]).u32(),
        0x1234_5678
    );
}

/// `SetEditorID`: a new node at the head of the text's bucket (the old head after it), the
/// count up by one, the map's set with (node, text, form), the node's key at +0xC8; true. No
/// map: true and nothing done.
#[test]
fn h04_set_editor_id_registers_the_form() {
    let mut e = world();
    let old = e.mem.alloc(4);
    let map = editor_map(&mut e, &[("Other", old)]);
    let head = e.mem.u32(e.mem.u32(map + 8));
    let form = e.mem.alloc(LIGHT_FORM_BYTES);
    let name = text(&mut e, "MyLamp");
    assert_eq!(e.call(SET_EDITOR_ID_ROUTINE, &args![form, name]).u32(), 1);
    assert_eq!(calls(MAP_NEW_NODE), vec![vec![form]]);
    assert_eq!(calls(MAP_BUCKET), vec![vec![map, name]]);
    let node = e.mem.u32(e.mem.u32(map + 8));
    assert_ne!(node, head);
    assert_eq!(e.mem.u32(node), head);
    assert_eq!(e.mem.u32(map + 0xc), 2);
    assert_eq!(calls(MAP_SET), vec![vec![map, node, name, form]]);
    let key = e.mem.u32(node + 4);
    assert_ne!(key, name, "the map's copy");
    assert_eq!(e.mem.u32(form + FORM_EDITOR_ID), key);
    // Found by its editor id afterwards (the newest first).
    assert_eq!(editor_id_lookup_at(&mut e, name), form);

    let mut e = world();
    e.mem.set_u32(EDITOR_IDS_MAP, 0);
    let form = e.mem.alloc(LIGHT_FORM_BYTES);
    assert_eq!(
        e.call(SET_EDITOR_ID_ROUTINE, &args![form, 0x77u32]).u32(),
        1
    );
    assert_eq!(e.mem.u32(form + FORM_EDITOR_ID), 0);
    assert!(calls(MAP_NEW_NODE).is_empty());
}

/// With JohnnyGuitar's editor ids in (their row on), JIP leaves H04 out, as it does with
/// JohnnyGuitar loaded (`if (!GetModuleHandle("johnnyguitar"))`): the light form keeps the
/// game's size (the site does not answer) and the light class's two editor-id slots stay the
/// exe's, for JohnnyGuitar's getter and setter. JIP's lookup by name asks their store.
#[test]
fn h04_left_out_with_johnnyguitars_editor_ids() {
    let mut e = engine();
    e.register(GAME_ANIMATED_LIGHTS, |_, _| Ret::default());
    engine_core::editor_ids::store(&mut e).set_on(true);
    crate::install_as(&mut e, true);
    assert_eq!(e.site(LIGHT_FORM_SIZE.addr(), &[]), None);
    assert_eq!(e.mem.u32(GET_EDITOR_ID_SLOT), 0);
    assert_eq!(e.mem.u32(SET_EDITOR_ID_SLOT), 0);
    assert_eq!(
        e.mem.u32(DESTROY_SLOT),
        DESTROY_ROUTINE,
        "the rest of JIP's lights stay"
    );
    assert!(e.site(CLONE_WORD.addr(), &[]).is_some());
    e.map(AT, 0x1_0000);
    let form = e.mem.alloc(LIGHT_FORM_BYTES);
    let store = engine_core::editor_ids::store(&mut e);
    store.add_name(form, b"MyLamp");
    store.map_insert(form, b"MyLamp");
    let lower = text(&mut e, "mylamp");
    assert_eq!(editor_id_lookup_at(&mut e, lower), form);
}

// ---- H07 the destructor ----

/// The destructor body always; 0x110 bytes freed with flag bit 0 for a light not of JIP's; a
/// light of JIP's (`0x20000000`) or no bit 0: not freed. Answers the light.
#[test]
fn h07_frees_0x110_bytes_but_not_jips_lights() {
    for (flags, jips, freed) in [
        (1u32, false, true),
        (3, false, true),
        (0, false, false),
        (2, false, false),
        (1, true, false),
    ] {
        let mut e = world();
        let light = point_light(&mut e);
        if jips {
            e.mem.set_u8(light + FLAGS_HIGH, 0x20);
        }
        let answer = e.call(DESTROY_ROUTINE, &args![light, flags]).u32();
        assert_eq!(answer, light);
        assert_eq!(calls(LIGHT_DESTRUCTOR), vec![vec![light]]);
        assert_eq!(
            calls(SIZED_FREE),
            if freed {
                vec![vec![light, 0x110]]
            } else {
                vec![]
            },
            "{flags} {jips}"
        );
    }
}

// ---- H06 the model's lights ----

/// A model's root: a light whose last `LIGH_EDID` string extra names a form's editor id is tied
/// to the form and marked with each parent up to the root; the extra removed (its index); a
/// light in a child node too; a light with an unknown id or a `LIGH_EDID` extra that is not a
/// string: the extra removed, nothing tied; a light without one: untouched. The root is marked
/// done and a second load changes nothing.
#[test]
fn h06_ties_the_tagged_lights_to_their_forms() {
    let mut e = world();
    let lamp = e.mem.alloc(LIGHT_FORM_BYTES);
    editor_map(&mut e, &[("Lamp01", lamp)]);
    let tied = point_light(&mut e);
    let other = extra(&mut e, 0x0109_0000, "Other", 0);
    let first = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", 0);
    let lamp_text = text(&mut e, "Lamp01");
    let last = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", lamp_text);
    set_extras(&mut e, tied, &[first, other, last]);
    let deep = point_light(&mut e);
    let deep_text = text(&mut e, "Lamp01");
    let deep_extra = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", deep_text);
    set_extras(&mut e, deep, &[deep_extra]);
    let unknown = point_light(&mut e);
    let unknown_text = text(&mut e, "Nobody");
    let unknown_extra = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", unknown_text);
    set_extras(&mut e, unknown, &[unknown_extra]);
    let wrong = point_light(&mut e);
    let wrong_text = text(&mut e, "Lamp01");
    let wrong_extra = extra(&mut e, 0x0109_0000, "LIGH_EDID", wrong_text);
    set_extras(&mut e, wrong, &[wrong_extra]);
    let plain = point_light(&mut e);
    let plain_extra = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDIDX", lamp_text);
    set_extras(&mut e, plain, &[plain_extra]);
    let inner = node(&mut e, &[deep]);
    let middle = node(&mut e, &[inner, 0]);
    let root = node(&mut e, &[tied, middle, unknown, wrong, plain]);

    assert_eq!(e.site(LOAD_NIF.addr(), &[root]), Some(0));
    assert_eq!(e.mem.u8(root + FLAGS_HIGH), 0x40 | 0x20);
    assert_eq!(e.mem.u32(tied + BASE_LIGHT), lamp);
    assert_eq!(e.mem.u8(tied + FLAGS_HIGH), 0x20);
    assert_eq!(e.mem.u32(deep + BASE_LIGHT), lamp);
    assert_eq!(e.mem.u8(deep + FLAGS_HIGH), 0x20);
    assert_eq!(e.mem.u8(inner + FLAGS_HIGH), 0x20);
    assert_eq!(e.mem.u8(middle + FLAGS_HIGH), 0x20);
    for light in [unknown, wrong, plain] {
        assert_eq!(e.mem.u32(light + BASE_LIGHT), 0, "{light:x}");
        assert_eq!(e.mem.u8(light + FLAGS_HIGH), 0, "{light:x}");
    }
    assert_eq!(
        calls(REMOVE_EXTRA_AT),
        vec![
            vec![tied, 2],
            vec![deep, 0],
            vec![unknown, 0],
            vec![wrong, 0]
        ]
    );

    // Done once: a second load of the same root does nothing.
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(tied + BASE_LIGHT, 0);
    assert_eq!(e.site(LOAD_NIF.addr(), &[root]), Some(0));
    assert!(calls(REMOVE_EXTRA_AT).is_empty());
    assert_eq!(e.mem.u32(tied + BASE_LIGHT), 0);
    // No 3D: nothing.
    assert_eq!(e.site(LOAD_NIF.addr(), &[0]), Some(0));
}

/// With JohnnyGuitar's editor ids in (their row on), JohnnyGuitar's InitLights: every form has
/// a name, so a light is tied only to a light form (type 0x1E). A static owning the `LIGH_EDID`
/// name leaves the light untied (the extra still removed); a light form's name ties. With the
/// store off, JIP's own: the same name ties to whatever form the game's map gives.
#[test]
fn h06_with_johnnyguitar_ties_only_to_a_light_form() {
    let mut e = world();
    engine_core::editor_ids::store(&mut e).set_on(true);
    let stat = e.mem.alloc(LIGHT_FORM_BYTES);
    e.mem.set_u8(stat + 4, 0x20);
    let lamp = e.mem.alloc(LIGHT_FORM_BYTES);
    e.mem.set_u8(lamp + 4, 0x1e);
    let store = engine_core::editor_ids::store(&mut e);
    for (form, name) in [(stat, &b"Shared01"[..]), (lamp, &b"Lamp01"[..])] {
        store.add_name(form, name);
        store.map_insert(form, name);
    }
    let by_static = point_light(&mut e);
    let shared = text(&mut e, "Shared01");
    let x = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", shared);
    set_extras(&mut e, by_static, &[x]);
    let by_lamp = point_light(&mut e);
    let lamp_text = text(&mut e, "lamp01");
    let y = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", lamp_text);
    set_extras(&mut e, by_lamp, &[y]);
    let root = node(&mut e, &[by_static, by_lamp]);
    assert_eq!(e.site(LOAD_NIF.addr(), &[root]), Some(0));
    assert_eq!(
        e.mem.u32(by_static + BASE_LIGHT),
        0,
        "a static is not a light"
    );
    assert_eq!(e.mem.u8(by_static + FLAGS_HIGH), 0);
    assert_eq!(e.mem.u32(by_lamp + BASE_LIGHT), lamp);
    assert_eq!(e.mem.u8(by_lamp + FLAGS_HIGH), 0x20);
    assert_eq!(
        calls(REMOVE_EXTRA_AT),
        vec![vec![by_static, 0], vec![by_lamp, 0]]
    );

    // The store off: JIP's own tie, to any form of the game's map.
    let mut e = world();
    let stat = e.mem.alloc(LIGHT_FORM_BYTES);
    e.mem.set_u8(stat + 4, 0x20);
    editor_map(&mut e, &[("Shared01", stat)]);
    let light = point_light(&mut e);
    let shared = text(&mut e, "Shared01");
    let x = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", shared);
    set_extras(&mut e, light, &[x]);
    let root = node(&mut e, &[light]);
    assert_eq!(e.site(LOAD_NIF.addr(), &[root]), Some(0));
    assert_eq!(e.mem.u32(light + BASE_LIGHT), stat);
}

/// The parents are marked up to the root and no further, and stop at one marked already.
#[test]
fn h06_marks_parents_up_to_the_root_only() {
    let mut e = world();
    let lamp = e.mem.alloc(LIGHT_FORM_BYTES);
    editor_map(&mut e, &[("Lamp01", lamp)]);
    let light = point_light(&mut e);
    let id = text(&mut e, "Lamp01");
    let x = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", id);
    set_extras(&mut e, light, &[x]);
    let inner = node(&mut e, &[light]);
    let root = node(&mut e, &[inner]);
    let above = node(&mut e, &[root]);
    assert_eq!(e.site(LOAD_NIF.addr(), &[root]), Some(0));
    assert_eq!(e.mem.u8(inner + FLAGS_HIGH), 0x20);
    assert_eq!(e.mem.u8(root + FLAGS_HIGH), 0x60);
    assert_eq!(e.mem.u8(above + FLAGS_HIGH), 0, "not past the root");

    // A parent marked already stops the walk.
    let mut e = world();
    editor_map(&mut e, &[("Lamp01", lamp)]);
    let light = point_light(&mut e);
    let id = text(&mut e, "Lamp01");
    let x = extra(&mut e, STRING_EXTRA_VTABLE, "LIGH_EDID", id);
    set_extras(&mut e, light, &[x]);
    let inner = node(&mut e, &[light]);
    let middle = node(&mut e, &[inner]);
    let root = node(&mut e, &[middle]);
    e.mem.set_u8(middle + FLAGS_HIGH, 0x20);
    assert_eq!(e.site(LOAD_NIF.addr(), &[root]), Some(0));
    assert_eq!(e.mem.u8(inner + FLAGS_HIGH), 0x20);
    assert_eq!(e.mem.u8(root + FLAGS_HIGH), 0x40, "the walk stopped below");
}

// ---- H06's companion: Clone3D ----

/// `Clone3D`'s seam: a model marked `0x20000000` (byte +0x33 bit 0x20) answers 1 (cloned), other
/// bits of the byte or none answer 0 (the exe's checks), no model 0.
#[test]
fn clone_3d_clones_a_model_with_jips_lights() {
    let mut e = world();
    // A form and a reference with no `InsertNode`/`AttachModel` entries (H08: their flags clear).
    let (form, reference) = (e.mem.alloc(0x10), e.mem.alloc(0x10));
    for (byte, answer) in [(0x20u8, 1u32), (0x60, 1), (0x40, 0), (0xdf, 0), (0, 0)] {
        let model = node(&mut e, &[]);
        e.mem.set_u8(model + FLAGS_HIGH, byte);
        assert_eq!(
            e.site(CLONE_3D.addr(), &[model, form, reference]),
            Some(answer),
            "{byte:02x}"
        );
    }
    assert_eq!(e.site(CLONE_3D.addr(), &[0, form, 0]), Some(0));
}

// ---- the list ----

/// A light form: radius, colour bytes, flags, fade.
fn light_form(e: &mut Engine, radius: u32, colour: u32, flags: u32, fade: f32) -> u32 {
    let form = e.mem.alloc(LIGHT_FORM_BYTES);
    e.mem.set_u32(form + FORM_RADIUS, radius);
    e.mem.set_u32(form + FORM_COLOUR, colour);
    e.mem.set_u32(form + FORM_FLAGS, flags);
    e.mem.set_f32(form + FORM_FADE, fade);
    form
}

/// What `SetLightProperties` leaves: the dimmer the fade, ambient and the four words at +0xF0
/// zero, the colour over 255, the radius as a float.
fn assert_properties(e: &Engine, light: u32, form: u32) {
    assert_eq!(e.mem.u32(light + DIMMER), e.mem.u32(form + FORM_FADE));
    for at in [0xc8u32, 0xcc, 0xd0, 0xf0, 0xf4, 0xf8, 0xfc] {
        assert_eq!(e.mem.u32(light + at), 0, "{at:x}");
    }
    let colour = e.mem.u32(form + FORM_COLOUR);
    let mult = f32::from_bits(0x3b80_8081);
    for (i, shift) in [0u32, 8, 16].into_iter().enumerate() {
        let byte = ((colour >> shift) & 0xff) as f32;
        assert_eq!(
            e.mem.f32(light + DIFFUSE + 4 * i as u32),
            byte * mult,
            "{i}"
        );
    }
    let radius = (e.mem.u32(form + FORM_RADIUS) | (colour >> 24)) as i32 as f32;
    assert_eq!(e.mem.f32(light + 0xe0), radius);
}

/// The 3D's lights of JIP's, in order and through child nodes: each marked listed, hidden when
/// the form is off by default (0x20), set from the form, appended; a light listed already, one
/// not marked, or one with no form: left alone.
#[test]
fn add_point_lights_lists_jips_lights() {
    let mut e = world();
    let on = light_form(&mut e, 300, 0x00ff_8040, 0, 0.75);
    let off = light_form(&mut e, 256, 0x0100_0000, 0x20, 1.5);
    let first = point_light(&mut e);
    let second = point_light(&mut e);
    let listed = point_light(&mut e);
    let unmarked = point_light(&mut e);
    let formless = point_light(&mut e);
    for (light, form, flags) in [
        (first, on, 0x20u8),
        (second, off, 0x20),
        (listed, on, 0x60),
        (unmarked, on, 0),
        (formless, 0, 0x20),
    ] {
        e.mem.set_u32(light + BASE_LIGHT, form);
        e.mem.set_u8(light + FLAGS_HIGH, flags);
        e.mem.set_u32(light + 0xc8, 0x5555);
    }
    let inner = node(&mut e, &[second]);
    e.mem.set_u8(inner + FLAGS_HIGH, 0x20);
    let skipped = node(&mut e, &[unmarked]);
    let root = node(
        &mut e,
        &[first, inner, listed, skipped, unmarked, formless, 0],
    );
    add_point_lights(&mut e, root);
    assert_eq!(active_point_lights(&mut e), vec![first, second]);
    assert_eq!(e.mem.u8(first + FLAGS_HIGH), 0x60);
    assert_eq!(e.mem.u8(second + FLAGS_HIGH), 0x60);
    assert_eq!(e.mem.u8(first + FLAGS) & 1, 0);
    assert_eq!(e.mem.u8(second + FLAGS) & 1, 1, "off by default: hidden");
    assert_properties(&e, first, on);
    assert_properties(&e, second, off);
    assert_eq!(e.mem.f32(first + 0xe0), 300.0);
    assert_eq!(e.mem.f32(second + 0xe0), 257.0, "radius | colour byte 3");
    for light in [listed, unmarked, formless] {
        assert_eq!(e.mem.u32(light + 0xc8), 0x5555, "{light:x}");
    }
    assert_eq!(e.mem.u8(unmarked + FLAGS_HIGH), 0);
    assert_eq!(e.mem.u8(formless + FLAGS_HIGH), 0x20);
}

// ---- H12 the animation ----

const SECONDS: f32 = 0.02;

/// A light for the animation with its dimmer, progress, translation (and scale 1.0) and colour.
fn animated(e: &mut Engine, dimmer: f32, progress: f32) -> u32 {
    let light = point_light(e);
    e.mem.set_f32(light + DIMMER, dimmer);
    e.mem.set_f32(light + DIMMER + 4, progress);
    for (i, v) in [10.0f32, 20.0, 30.0, 1.0].into_iter().enumerate() {
        e.mem.set_f32(light + LOCAL_TRANSLATE + 4 * i as u32, v);
    }
    for (i, v) in [0.5f32, 0.25, 1.0, 300.0].into_iter().enumerate() {
        e.mem.set_f32(light + DIFFUSE + 4 * i as u32, v);
    }
    light
}

fn animation_world() -> Engine {
    let e = world();
    e.mem.set_f32(FRAME_SECONDS, SECONDS);
    e
}

fn f4(e: &Engine, at: u32) -> [f32; 4] {
    std::array::from_fn(|i| e.mem.f32(at + 4 * i as u32))
}

/// Flicker (0x8, normal speed 7.5): the first frame saves the translation (+0x100, lane 3 0)
/// and sets `extraFlags` bit 1, picks the dimmer's change from the first random and the
/// offset from the next three (z, y, x), and moves both by one step; the scale +0x64 becomes
/// 0.0 (lane 3). A frame within the leg moves on with no random; a progress past 1.0 starts a
/// new leg from the saved translation.
#[test]
fn h12_flicker_moves_the_dimmer_and_the_light() {
    let mut e = animation_world();
    let form = light_form(&mut e, 0, 0, 0x8, 0.8);
    let light = animated(&mut e, 0.5, 0.0);
    randoms(&[0x25, 0x10, 0x7f, 0x83]);
    animate(&mut e, form, light);
    let step = SECONDS * 7.5;
    let change =
        (0x25 as f32 * f32::from_bits(0x3b9c_09c1) + f32::from_bits(0x3f2c_cccd)) * 0.8 - 0.5;
    let tenth = f32::from_bits(0x3dcc_cccd);
    // Saved = the translation at the start, so the offset is the pick alone.
    let offset = [
        (3 - 63) as f32 * tenth - 10.0 + 10.0,
        (127 - 63) as f32 * tenth - 20.0 + 20.0,
        (16 - 63) as f32 * tenth - 30.0 + 30.0,
        0.0,
    ];
    assert_eq!(calls(GET_RANDOM_INT).len(), 4);
    assert!(calls(GET_RANDOM_INT)
        .iter()
        .all(|w| w[..] == [GAME_RNG, 0xffff_ffff]));
    assert_eq!(e.mem.u8(light + EXTRA_FLAGS), 1);
    assert_eq!(f4(&e, light + SAVED), [10.0, 20.0, 30.0, 0.0]);
    assert_eq!(f4(&e, light + OFFSET), offset);
    assert_eq!(e.mem.f32(light + DIMMER + 8), change);
    assert_eq!(e.mem.f32(light + DIMMER + 4), step);
    assert_eq!(e.mem.f32(light + DIMMER), 0.5 + change * step);
    let moved = [
        10.0 + offset[0] * step,
        20.0 + offset[1] * step,
        30.0 + offset[2] * step,
        0.0,
    ];
    assert_eq!(f4(&e, light + LOCAL_TRANSLATE), moved);
    assert_eq!(e.mem.u32(light + 0x64), 0, "the scale lane is 0.0");

    // Within the leg: no random, the same change and offset.
    CALLS.with(|c| c.borrow_mut().clear());
    animate(&mut e, form, light);
    assert!(calls(GET_RANDOM_INT).is_empty());
    assert_eq!(e.mem.f32(light + DIMMER + 4), step + step);
    assert_eq!(
        e.mem.f32(light + DIMMER),
        (0.5 + change * step) + change * step
    );
    assert_eq!(
        e.mem.f32(light + LOCAL_TRANSLATE),
        moved[0] + offset[0] * step
    );

    // Past 1.0: a new leg toward the saved point.
    e.mem.set_f32(light + DIMMER + 4, 0.99);
    let at = f4(&e, light + LOCAL_TRANSLATE);
    let dimmer = e.mem.f32(light + DIMMER);
    randoms(&[0x3f, 0x3f, 0x3f, 0x3f]);
    animate(&mut e, form, light);
    let change =
        (0x3f as f32 * f32::from_bits(0x3b9c_09c1) + f32::from_bits(0x3f2c_cccd)) * 0.8 - dimmer;
    assert_eq!(e.mem.f32(light + DIMMER + 8), change);
    assert_eq!(e.mem.f32(light + DIMMER + 4), step);
    let offset = [
        0.0 * tenth - at[0] + 10.0,
        0.0 * tenth - at[1] + 20.0,
        0.0 * tenth - at[2] + 30.0,
        0.0,
    ];
    assert_eq!(f4(&e, light + OFFSET), offset);
    assert_eq!(e.mem.f32(light + LOCAL_TRANSLATE), at[0] + offset[0] * step);
}

/// Slow flicker (0x40): the step is the seconds times 4.2857 (`0x40892492`).
#[test]
fn h12_slow_flicker_steps_slower() {
    let mut e = animation_world();
    let form = light_form(&mut e, 0, 0, 0x40, 1.0);
    let light = animated(&mut e, 0.5, 0.25);
    animate(&mut e, form, light);
    assert_eq!(
        e.mem.f32(light + DIMMER + 4),
        0.25 + SECONDS * f32::from_bits(0x4089_2492)
    );
}

/// Pulse (0x80, step 0.125; 0x100 slow, 0.0625): between a quarter of the fade and the fade
/// the dimmer moves in the direction kept in +0xD0's sign; at or above the fade, or at or below
/// a quarter of it, the direction turns first.
#[test]
fn h12_pulse_turns_at_the_ends() {
    for (flags, speed) in [(0x80u32, 0.125f32), (0x100, 0.0625)] {
        let step = SECONDS * speed;
        for (dimmer, direction, turned) in [
            (0.5f32, 0u32, false),
            (0.5, 0x8000_0000, false),
            (1.0, 0, true),
            (1.25, 0x8000_0000, true),
            (0.25, 0x8000_0000, true),
            (0.1, 0, true),
        ] {
            let mut e = animation_world();
            let form = light_form(&mut e, 0, 0, flags, 1.0);
            let light = animated(&mut e, dimmer, 0.0);
            e.mem.set_u32(light + 0xd0, direction);
            animate(&mut e, form, light);
            let now = if turned {
                direction ^ 0x8000_0000
            } else {
                direction
            };
            assert_eq!(e.mem.u32(light + 0xd0), now, "{flags:x} {dimmer}");
            let moved = if now != 0 { -step } else { step };
            assert_eq!(
                e.mem.f32(light + DIMMER),
                dimmer + moved,
                "{flags:x} {dimmer}"
            );
            assert!(calls(GET_RANDOM_INT).is_empty());
            assert_eq!(f4(&e, light + LOCAL_TRANSLATE), [10.0, 20.0, 30.0, 1.0]);
        }
    }
}

/// JIP's colour shift (0x800, step 4.0; 0x1000 slow, 2.0): the first frame saves the colour
/// (+0x100, lane 3 0) and moves toward the saved colour times `(random & 0x7F) / 127` per
/// channel (randoms b, g, r in that order of calls); the radius +0xE0 is left alone.
#[test]
fn h12_colour_shift_moves_the_colour() {
    for (flags, speed) in [(0x800u32, 4.0f32), (0x1000, 2.0)] {
        let mut e = animation_world();
        let form = light_form(&mut e, 0, 0, flags, 1.0);
        let light = animated(&mut e, 0.5, 0.0);
        randoms(&[0x7f, 0x80, 0x40]);
        animate(&mut e, form, light);
        let step = SECONDS * speed;
        let k = f32::from_bits(0x3c01_0204);
        let offset = [
            0x40 as f32 * k * 0.5 - 0.5,
            0.0 * k * 0.25 - 0.25,
            127.0 * k * 1.0 - 1.0,
            0.0,
        ];
        assert_eq!(f4(&e, light + SAVED), [0.5, 0.25, 1.0, 0.0]);
        assert_eq!(f4(&e, light + OFFSET), offset);
        assert_eq!(e.mem.f32(light + DIMMER + 4), step);
        assert_eq!(
            f4(&e, light + DIFFUSE),
            [
                0.5 + offset[0] * step,
                0.25 + offset[1] * step,
                1.0 + offset[2] * step,
                300.0
            ],
            "{flags:x}"
        );
        assert_eq!(e.mem.f32(light + DIMMER), 0.5, "the dimmer stays");
        assert_eq!(e.mem.u8(light + EXTRA_FLAGS), 0);
    }
}

/// H12's wrap of `0050de20`: a form with no animated flag does nothing; with one, the data's
/// light is animated; no light: nothing. The exe's function never runs; answers 0.
#[test]
fn h12_wrap_animates_the_datas_light() {
    let mut e = animation_world();
    let light = animated(&mut e, 0.5, 0.5);
    let data = e.mem.alloc(8);
    e.mem.set_u32(data, light);
    let quiet = light_form(&mut e, 0, 0, 0x2 | 0x20 | 0x200 | 0x400, 1.0);
    assert_eq!(e.call(0x0050_de20, &args![quiet, data, 0u32]).u32(), 0);
    assert_eq!(e.mem.f32(light + DIMMER), 0.5);
    assert_eq!(
        e.mem.f32(light + DIMMER + 4),
        0.5,
        "not animated: the progress stays"
    );
    let pulse = light_form(&mut e, 0, 0, 0x80, 1.0);
    assert_eq!(e.call(0x0050_de20, &args![pulse, data, 0u32]).u32(), 0);
    assert_eq!(e.mem.f32(light + DIMMER), 0.5 + SECONDS * 0.125);
    e.mem.set_u32(data, 0);
    assert_eq!(e.call(0x0050_de20, &args![pulse, data, 0u32]).u32(), 0);
}

/// H12 at the two task sequences: the exe's animated lights first (`00455640` with the `TES`
/// word), then, with the scene lights' lock held once, the list from the last: a light with no
/// parent freed (0x110 bytes) and taken out once its count is 0 (the last moved into its
/// place), kept while counted; a hidden one skipped; a marked one set from its form again and
/// its translation and scale put back; a light held only by its parent put into the scene's
/// lights (a new `ShadowSceneLight`) and not animated; the others animated when their form
/// says so. Answers 0.
#[test]
fn h12_updates_jips_lights() {
    let mut e = animation_world();
    let scene = e.mem.alloc(0x200);
    e.mem.set_u32(scene + SCENE_1E0, 0x0abc);
    e.mem.set_u32(SHADOW_SCENE_NODE, scene);
    let pulse = light_form(&mut e, 400, 0x0020_4060, 0x80, 1.0);
    let steady = light_form(&mut e, 400, 0x0020_4060, 0, 1.0);
    let parent = e.mem.alloc(0x40);
    let make = |e: &mut Engine, has_parent: bool, count: u32, form: u32| {
        let light = animated(e, 0.5, 0.5);
        if has_parent {
            e.mem.set_u32(light + PARENT, parent);
        }
        e.mem.set_u32(light + REF_COUNT, count);
        e.mem.set_u32(light + BASE_LIGHT, form);
        light
    };
    let gone = make(&mut e, false, 0, pulse);
    let detached = make(&mut e, false, 2, pulse);
    let hidden = make(&mut e, true, 3, pulse);
    e.mem.set_u8(hidden + FLAGS, 1);
    let fresh = make(&mut e, true, 1, pulse);
    e.mem.set_u8(fresh + RESET_TRAITS, 1);
    e.mem.set_u8(fresh + EXTRA_FLAGS, 0x81);
    for (i, v) in [1.0f32, 2.0, 3.0, 4.0].into_iter().enumerate() {
        e.mem.set_f32(fresh + SAVED + 4 * i as u32, v);
    }
    let pulsing = make(&mut e, true, 3, pulse);
    let still = make(&mut e, true, 3, steady);
    lights_list(&mut e).extend([gone, detached, hidden, fresh, pulsing, still]);
    // Another thread adds a light while the step runs (here: at the scene add): kept after the list.
    let late = make(&mut e, true, 3, steady);
    // The constructed `ShadowSceneLight` has bytes of its own after +0xFC: kept.
    e.register(SCENE_LIGHT_CONSTRUCT, |e, a| {
        log(SCENE_LIGHT_CONSTRUCT, a);
        e.mem.set_u32(a[0] + SCENE_LIGHT_FC, 0x5c5d_5e00);
        a[0].into_ret()
    });
    e.register_double(SCENE_ADD_LIGHT, move |e, a| {
        log(SCENE_ADD_LIGHT, a);
        lights_list(e).push(late);
        Ret::default()
    });

    assert_eq!(e.site(ANIMATED_LIGHTS[1].addr(), &[0x0d00]), Some(0));
    assert_eq!(calls(GAME_ANIMATED_LIGHTS), vec![vec![0x0d00]]);
    assert_eq!(calls(LOCK_ENTER), vec![vec![SCENE_LIGHTS_LOCK, 0]]);
    assert_eq!(calls(LOCK_LEAVE), vec![vec![SCENE_LIGHTS_LOCK]]);
    let order = addresses();
    assert_eq!(order.first(), Some(&GAME_ANIMATED_LIGHTS));
    assert_eq!(order.last(), Some(&LOCK_LEAVE));
    assert_eq!(calls(SIZED_FREE), vec![vec![gone, 0x110]]);
    assert_eq!(
        active_point_lights(&mut e),
        vec![still, detached, hidden, fresh, pulsing, late]
    );
    // Hidden: untouched.
    assert_eq!(e.mem.f32(hidden + DIMMER), 0.5);
    // Marked: set again, the translation and scale back, both marks cleared, added.
    assert_properties(&e, fresh, pulse);
    assert_eq!(e.mem.u8(fresh + RESET_TRAITS), 0);
    assert_eq!(e.mem.u8(fresh + EXTRA_FLAGS), 0x80);
    assert_eq!(f4(&e, fresh + LOCAL_TRANSLATE), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(calls(ALLOCATE), vec![vec![0x250]]);
    let scene_light = calls(SCENE_LIGHT_CONSTRUCT)[0][0];
    assert_eq!(calls(SCENE_ADD_LIGHT), vec![vec![scene, scene_light]]);
    assert_eq!(e.mem.u16(scene_light + SCENE_LIGHT_POINT), 1);
    assert_eq!(e.mem.u32(scene_light + SCENE_LIGHT_LIGHT), fresh);
    assert_eq!(e.mem.u8(scene_light + SCENE_LIGHT_FC), 1);
    assert_eq!(
        e.mem.u32(scene_light + SCENE_LIGHT_FC),
        0x5c5d_5e01,
        "one byte written, +0xFD..+0xFF kept"
    );
    assert_eq!(e.mem.u32(scene_light + SCENE_LIGHT_240), 0x0abc);
    assert_eq!(e.mem.u32(fresh + REF_COUNT), 2);
    assert_eq!(e.mem.f32(fresh + DIMMER), 1.0, "set, not animated");
    // Counted: animated by its form; a form with no animation: untouched.
    assert_eq!(e.mem.f32(pulsing + DIMMER), 0.5 + SECONDS * 0.125);
    assert_eq!(e.mem.f32(still + DIMMER), 0.5);
    assert_eq!(e.mem.f32(detached + DIMMER), 0.5);

    // Next frame: the added light is animated now; the detached one freed once let go.
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(detached + REF_COUNT, 0);
    assert_eq!(e.site(ANIMATED_LIGHTS[0].addr(), &[0x0d04]), Some(0));
    assert_eq!(calls(SIZED_FREE), vec![vec![detached, 0x110]]);
    assert!(calls(SCENE_ADD_LIGHT).is_empty());
    assert_eq!(
        e.mem.f32(fresh + DIMMER),
        1.0 - SECONDS * 0.125,
        "at the fade: turned"
    );
    assert_eq!(
        active_point_lights(&mut e),
        vec![still, late, hidden, fresh, pulsing]
    );
}

/// An empty list: the exe's animated lights only, no lock.
#[test]
fn h12_with_no_lights_only_the_exes_step() {
    let mut e = animation_world();
    assert_eq!(e.site(ANIMATED_LIGHTS[0].addr(), &[0x0d00]), Some(0));
    assert_eq!(addresses(), vec![GAME_ANIMATED_LIGHTS]);
}

/// No scene node: a light held only by its parent is not added (JIP would read through null).
#[test]
fn h12_without_a_scene_node_adds_nothing() {
    let mut e = animation_world();
    e.mem.set_u32(SHADOW_SCENE_NODE, 0);
    let form = light_form(&mut e, 1, 0, 0, 1.0);
    let light = animated(&mut e, 0.5, 0.5);
    let parent = e.mem.alloc(0x40);
    e.mem.set_u32(light + PARENT, parent);
    e.mem.set_u32(light + REF_COUNT, 1);
    e.mem.set_u32(light + BASE_LIGHT, form);
    lights_list(&mut e).push(light);
    assert_eq!(e.site(ANIMATED_LIGHTS[0].addr(), &[0x0d00]), Some(0));
    assert!(calls(ALLOCATE).is_empty());
    assert_eq!(e.mem.u32(light + REF_COUNT), 1);
    assert_eq!(active_point_lights(&mut e), vec![light]);
}
