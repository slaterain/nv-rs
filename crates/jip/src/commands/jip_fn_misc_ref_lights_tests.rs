//! Tests of `AttachLight` and `RemoveLight`, called by their opcodes on trees built by
//! `nif_nodes_tests.rs`. Expectations from JIP's command bodies (functions_jip/jip_fn_misc_ref.h),
//! `CreatePointLight` (internal/hooks.h), `NiObjectNET::SetName` (internal/netimmerse.cpp) and
//! `TESObjectREFR::GetNode2` (nvse/GameObjects.cpp); the game functions are stand-ins at their exe
//! addresses that record their words in order.

use super::super::nif_nodes::tests::*;
use crate::commands::test_support::{call_on, calls, form, r, s, set_ref, z, CALLS, FORMS};
use crate::game_patches::gl::lights::{active_point_lights, ActivePointLights, POINT_LIGHT_VTABLE};
use engine_core::prelude::*;
use nvse::plugins::{dispatch, msg};

const ATTACH: u32 = 0x2879;
const REMOVE: u32 = 0x287a;
const ADD_OBJECT: u32 = 0x00a5_ed10;
const REMOVE_OBJECT: u32 = 0x00a5_d8f0;
const ADD_STRING: u32 = 0x00a5_b690;
const CREATE: u32 = 0x00a7_d6e0;
const LIGHT: u32 = FORMS + 0x1000;
const OTHER: u32 = FORMS + 0x1200;
/// A point light's flags (+0x30, +0x33), `resetTraits` (+0x9E), `extraFlags` (+0x9F), form (+0xE8).
const FLAGS_LOW: u32 = 0x30;
const FLAGS_HIGH: u32 = 0x33;
const RESET: u32 = 0x9e;
const EXTRA: u32 = 0x9f;
const BASE: u32 = 0xe8;

fn rec(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn clear() {
    CALLS.with(|x| x.borrow_mut().clear());
}

/// A reference whose root `Root` holds the node `Lamp` (no children); the light form (reference
/// 1, id 0x1A2B3) and a form of another type (reference 2).
fn setup() -> (Engine, u32, u32) {
    let mut e = scene_engine();
    let lamp = node(&mut e, b"Lamp", &[]);
    let root = node(&mut e, b"Root", &[lamp]);
    let refr = reference(&mut e, 0x501, false, root);
    form(&mut e, LIGHT, 0x1e, 0x0001_a2b3);
    form(&mut e, OTHER, 0x1f, 0x0000_0a1f);
    set_ref(&mut e, 1, LIGHT);
    set_ref(&mut e, 2, OTHER);
    e.mem.set_u32(NI_NODE + 0xdc, ADD_OBJECT);
    e.mem.set_u32(NI_NODE + 0xe8, REMOVE_OBJECT);
    e.register(ADD_OBJECT, |_, w| {
        rec(ADD_OBJECT, w);
        eax(0)
    });
    e.register(REMOVE_OBJECT, |_, w| {
        rec(REMOVE_OBJECT, w);
        eax(0)
    });
    e.register(CREATE, |e, w| {
        rec(CREATE, w);
        eax(object(e, POINT_LIGHT_VTABLE, None))
    });
    e.register(ADD_STRING, |e, w| {
        let text = nvse::table::guest_text(e, w[0]);
        eax(fixed(e, &text))
    });
    clear();
    (e, refr, lamp)
}

fn name_of(e: &Engine, object: u32) -> Vec<u8> {
    nvse::table::guest_text(e, e.mem.u32(object + NAME))
}

fn attach(e: &mut Engine, refr: u32, node: &[u8], light: u16, offset: &[f64]) -> f64 {
    clear();
    let mut args = vec![s(node), r(light)];
    args.extend(offset.iter().map(|&v| z(v)));
    call_on(e, ATTACH, refr, &args).1
}

fn remove(e: &mut Engine, refr: u32, node: &[u8]) -> f64 {
    clear();
    call_on(e, REMOVE, refr, &[s(node)]).1
}

/// The (node, light) of the `AddObject` call the last command made.
fn added() -> (u32, u32) {
    let add = calls()
        .into_iter()
        .find(|c| c.0 == ADD_OBJECT)
        .expect("an AddObject call");
    assert_eq!(add.1.len(), 3);
    (add.1[0], add.1[1])
}

fn translation(e: &Engine, light: u32) -> Vec<f32> {
    (0..3)
        .map(|i| e.mem.f32(light + LOCAL_TRANSLATE + 4 * i))
        .collect()
}

/// A new light: made, named "PtLight " and the form's id in upper-case hexadecimal, added under
/// the node (`AddObject` (light, 1)), the offset its local translation, JIP's marks set (its form,
/// reset, the script mark, `0x60` and listed once); result 1.
#[test]
fn attach_light_makes_names_adds_and_lists_a_light() {
    let (mut e, refr, lamp) = setup();
    assert_eq!(attach(&mut e, refr, b"Lamp", 1, &[1.5, -2.0, 8.25]), 1.0);
    let log = calls();
    assert_eq!(log.len(), 2, "{log:x?}");
    assert_eq!(log[0], (CREATE, vec![]));
    let (node, light) = added();
    assert_eq!(log[1], (ADD_OBJECT, vec![lamp, light, 1]));
    assert_eq!(node, lamp);
    assert_eq!(e.mem.u32(light), POINT_LIGHT_VTABLE);
    assert_eq!(name_of(&e, light), b"PtLight 1A2B3");
    assert_eq!(e.mem.u32(light + BASE), LIGHT);
    assert_eq!(e.mem.u8(light + RESET), 1);
    assert_eq!(e.mem.u8(light + EXTRA), 0x80);
    assert_eq!(e.mem.u8(light + FLAGS_HIGH), 0x60);
    assert_eq!(e.mem.u8(light + FLAGS_LOW), 0, "not off by default");
    assert_eq!(translation(&e, light), vec![1.5, -2.0, 8.25]);
    assert_eq!(active_point_lights(&mut e), vec![light]);
}

/// Without an offset the translation is 0, 0, 0 (JIP's zeroed vector), with one value only x is
/// set; an empty node name is the root (`GetNode2`).
#[test]
fn attach_light_without_an_offset_and_on_the_root() {
    let (mut e, refr, _) = setup();
    let root = e.mem.u32(e.mem.u32(refr + LOADED_DATA) + ROOT_NODE);
    assert_eq!(attach(&mut e, refr, b"", 1, &[]), 1.0);
    let (node, light) = added();
    assert_eq!(node, root);
    assert_eq!(translation(&e, light), vec![0.0, 0.0, 0.0]);
    attach(&mut e, refr, b"Lamp", 1, &[4.0]);
    let (_, light) = added();
    assert_eq!(
        translation(&e, light),
        vec![4.0, 0.0, 0.0],
        "y and z not given"
    );
}

/// The node's first point light child is used again (null slots and other blocks skipped): no new
/// light, no add, its name kept; a light not listed yet is listed and marked `0x60`; a second
/// attach marks it to be set again and does not list it twice.
#[test]
fn attach_light_reuses_the_nodes_first_point_light() {
    let (mut e, refr, lamp) = setup();
    let bulb = shape(&mut e, b"Bulb");
    let first = object(&mut e, POINT_LIGHT_VTABLE, Some(b"Mesh light"));
    let second = object(&mut e, POINT_LIGHT_VTABLE, Some(b"Second"));
    set_children(&mut e, lamp, &[0, bulb, first, second]);
    e.mem.set_u8(first + FLAGS_HIGH, 0x01);
    assert_eq!(attach(&mut e, refr, b"Lamp", 1, &[3.0, 2.0, 1.0]), 1.0);
    assert!(calls().is_empty(), "nothing made or added: {:x?}", calls());
    assert_eq!(name_of(&e, first), b"Mesh light");
    assert_eq!(e.mem.u32(first + BASE), LIGHT);
    assert_eq!(e.mem.u8(first + FLAGS_HIGH), 0x61);
    assert_eq!(e.mem.u8(first + EXTRA), 0x80);
    assert_eq!(translation(&e, first), vec![3.0, 2.0, 1.0]);
    assert_eq!(e.mem.u32(second + BASE), 0, "only the first");
    assert_eq!(active_point_lights(&mut e), vec![first]);
    e.mem.set_u8(first + RESET, 0);
    attach(&mut e, refr, b"Lamp", 1, &[]);
    assert!(calls().is_empty());
    assert_eq!(e.mem.u8(first + RESET), 1);
    assert_eq!(e.mem.u8(first + FLAGS_HIGH), 0x61);
    assert_eq!(active_point_lights(&mut e), vec![first], "listed once");
}

/// A form "off by default" (+0xA8 bit 0x20) hides the light (+0x30 bit 0); a later attach of a
/// form that is not leaves it hidden (JIP only ORs the bit in).
#[test]
fn attach_light_hides_an_off_by_default_light_and_never_shows_it() {
    let (mut e, refr, lamp) = setup();
    e.mem.set_u32(LIGHT + 0xa8, 0x20);
    attach(&mut e, refr, b"Lamp", 1, &[]);
    let (_, light) = added();
    assert_eq!(e.mem.u8(light + FLAGS_LOW), 1);
    set_children(&mut e, lamp, &[light]);
    e.mem.set_u32(LIGHT + 0xa8, 0);
    attach(&mut e, refr, b"Lamp", 1, &[]);
    assert!(calls().is_empty(), "the same light");
    assert_eq!(e.mem.u8(light + FLAGS_LOW), 1, "still hidden");
    // The hidden bit joins the light's other flag bits (JIP's OR).
    let mesh = object(&mut e, POINT_LIGHT_VTABLE, Some(b"Mesh light"));
    e.mem.set_u8(mesh + FLAGS_LOW, 0x04);
    set_children(&mut e, lamp, &[mesh]);
    e.mem.set_u32(LIGHT + 0xa8, 0x20);
    attach(&mut e, refr, b"Lamp", 1, &[]);
    assert_eq!(e.mem.u8(mesh + FLAGS_LOW), 0x05);
}

/// No node of that name, no 3D, a form that is not a light, or a game that makes no light:
/// nothing made or listed, no result.
#[test]
fn attach_light_needs_a_node_a_light_form_and_a_light() {
    let (mut e, refr, _) = setup();
    assert_eq!(attach(&mut e, refr, b"Nowhere", 1, &[]), 0.0);
    assert!(calls().is_empty());
    assert_eq!(attach(&mut e, refr, b"Lamp", 2, &[]), 0.0, "not a light");
    assert!(calls().is_empty());
    e.register(CREATE, |_, w| {
        rec(CREATE, w);
        eax(0)
    });
    assert_eq!(attach(&mut e, refr, b"Lamp", 1, &[]), 0.0);
    assert_eq!(
        calls(),
        vec![(CREATE, vec![])],
        "nothing added after no light"
    );
    assert!(active_point_lights(&mut e).is_empty());
    set_root(&mut e, refr, 0);
    assert_eq!(attach(&mut e, refr, b"", 1, &[]), 0.0, "no 3D");
    assert!(calls().is_empty());
}

/// `RemoveLight`: the node's first point light child with the script mark (0x80) taken out
/// (`RemoveObject`, vtable +0xE8); unmarked lights, other blocks with the bit and null slots are
/// passed over; one per call; no node or no marked light, nothing. No result.
#[test]
fn remove_light_takes_out_the_first_marked_point_light() {
    let (mut e, refr, lamp) = setup();
    let bulb = shape(&mut e, b"Bulb");
    let plain = object(&mut e, POINT_LIGHT_VTABLE, Some(b"Plain"));
    let marked_a = object(&mut e, POINT_LIGHT_VTABLE, Some(b"A"));
    let marked_b = object(&mut e, POINT_LIGHT_VTABLE, Some(b"B"));
    e.mem.set_u8(bulb + EXTRA, 0x80);
    e.mem.set_u8(plain + EXTRA, 0x7f);
    e.mem.set_u8(marked_a + EXTRA, 0x80);
    e.mem.set_u8(marked_b + EXTRA, 0x81);
    set_children(&mut e, lamp, &[0, bulb, plain, marked_a, marked_b]);
    assert_eq!(remove(&mut e, refr, b"Lamp"), 0.0);
    assert_eq!(calls(), vec![(REMOVE_OBJECT, vec![lamp, marked_a])]);
    set_children(&mut e, lamp, &[plain, marked_b]);
    remove(&mut e, refr, b"Lamp");
    assert_eq!(calls(), vec![(REMOVE_OBJECT, vec![lamp, marked_b])]);
    set_children(&mut e, lamp, &[plain, bulb]);
    remove(&mut e, refr, b"Lamp");
    assert!(calls().is_empty(), "no marked light");
    set_children(&mut e, lamp, &[marked_a]);
    remove(&mut e, refr, b"Nowhere");
    assert!(calls().is_empty(), "no node");
}

/// A light `AttachLight` put on a node is the one `RemoveLight` takes out (its script mark).
#[test]
fn an_attached_light_is_removed_again() {
    let (mut e, refr, lamp) = setup();
    attach(&mut e, refr, b"Lamp", 1, &[]);
    let (_, light) = added();
    set_children(&mut e, lamp, &[light]);
    remove(&mut e, refr, b"Lamp");
    assert_eq!(calls(), vec![(REMOVE_OBJECT, vec![lamp, light])]);
}

/// `GetNode2` takes only a node: a geometry block of that name is no place for a light.
#[test]
fn a_geometry_block_is_not_a_node() {
    let (mut e, refr, lamp) = setup();
    let shade = shape(&mut e, b"Shade");
    set_children(&mut e, lamp, &[shade]);
    assert_eq!(attach(&mut e, refr, b"Shade", 1, &[]), 0.0);
    assert!(calls().is_empty(), "{:x?}", calls());
    remove(&mut e, refr, b"Shade");
    assert!(calls().is_empty());
    assert!(active_point_lights(&mut e).is_empty());
}

/// Before a game loads and at the exit to the main menu (JIP's `DoPreLoadGameHousekeeping`):
/// every listed light with the script mark and a parent comes off it (`RemoveObject`); a listed
/// light without the mark and a marked one without a parent are left; the list stays as it is.
#[test]
fn script_lights_come_off_their_nodes_before_a_load() {
    for kind in [msg::PRE_LOAD_GAME, msg::EXIT_TO_MAIN_MENU] {
        let (mut e, refr, lamp) = setup();
        // The other housekeeping steps read the player (`*011DEA3C`): this reference stands in.
        if !e.mem.is_mapped(0x011d_e000) {
            e.map(0x011d_e000, 0x1000);
        }
        e.mem.set_u32(0x011d_ea3c, refr);
        attach(&mut e, refr, b"Lamp", 1, &[]);
        let (_, attached) = added();
        set_children(&mut e, lamp, &[attached]);
        let plain = object(&mut e, POINT_LIGHT_VTABLE, Some(b"Plain"));
        let orphan = object(&mut e, POINT_LIGHT_VTABLE, Some(b"Orphan"));
        e.mem.set_u8(orphan + EXTRA, 0x80);
        let _holder = node(&mut e, b"Holder", &[plain]);
        e.ext_free::<ActivePointLights>()
            .0
            .lock()
            .unwrap()
            .extend([plain, orphan]);
        clear();
        dispatch(&mut e, kind, 0, 0);
        let removed: Vec<_> = calls()
            .into_iter()
            .filter(|c| c.0 == REMOVE_OBJECT)
            .collect();
        assert_eq!(
            removed,
            vec![(REMOVE_OBJECT, vec![lamp, attached])],
            "{kind}"
        );
        assert_eq!(
            active_point_lights(&mut e),
            vec![attached, plain, orphan],
            "{kind}: the list stays"
        );
    }
}
