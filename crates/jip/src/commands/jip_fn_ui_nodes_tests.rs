//! Tests of `AttachUIXML` and `AttachUIComponent` on trees built by `nif_nodes_tests.rs`, each
//! called by its opcode. Expectations from JIP's command bodies (functions_jip/jip_fn_ui.h) and
//! `TESObjectREFR::GetNode` (nvse/GameObjects.cpp); the game functions are stand-ins at their exe
//! addresses that record their words.

use super::super::nif_nodes::tests::*;
use crate::commands::test_support::{call_on, calls_to, s, CALLS};
use engine_core::prelude::*;

/// `NiNode::AddObject`, vtable +0xDC of `NiNode` and `BSFadeNode`: `00A5ED10` (`RET 8`).
const ADD_OBJECT: u32 = 0x00a5_ed10;
/// `Tile::ReadFile` `00A01B00` (`RET 4`).
const READ_FILE: u32 = 0x00a0_1b00;
/// `Menu::CompressAndStoreXML` (what UIO's inject calls first).
const COMPRESS: u32 = 0x00a1_ce70;
/// `Menu::XMLHash`'s `SetAt` and `RemoveAt`.
const XML_SET_AT: u32 = 0x00a0_c900;
const XML_REMOVE_AT: u32 = 0x00a1_e240;
/// `g_HUDMainMenu`.
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
const ATTACH_UI_XML: u32 = 0x28fe;
const ATTACH_UI_COMPONENT: u32 = 0x28ff;

fn record(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The HUD's menu and tile, the tile `ReadFile` makes (with a node) and one without a node.
struct Scene {
    hud_tile: u32,
    made: u32,
    made_node: u32,
    bare: u32,
    root: u32,
    bip: u32,
    first: u32,
    first_bip: u32,
}

/// What `ReadFile`'s stand-in makes: `good.xml` and UIO's `jip_temp.xml` a tile with a node,
/// `bare.xml` a tile without one, anything else none.
const MADE_AT: u32 = AREA + 0xd_0000;
const BARE_AT: u32 = AREA + 0xd_0100;

fn setup() -> (Engine, Scene) {
    let mut e = scene_engine();
    // `BSFadeNode`'s slot +0xDC lies on the page after the one the scene maps.
    e.map(0x010a_9000, 0x1000);
    e.map(0x011d_9000, 0x1000);
    e.mem.set_u32(NI_NODE + 0xdc, ADD_OBJECT);
    e.mem.set_u32(BS_FADE_NODE + 0xdc, ADD_OBJECT);
    e.register(ADD_OBJECT, |_, w| {
        record(ADD_OBJECT, w);
        eax(0)
    });
    e.register(READ_FILE, |e, w| {
        record(READ_FILE, w);
        let path = nvse::table::guest_text(e, w[1]);
        eax(match path.as_slice() {
            b"good.xml" | b"jip_temp.xml" => MADE_AT,
            b"bare.xml" => BARE_AT,
            _ => 0,
        })
    });
    let menu = object(&mut e, 0, None);
    let hud_tile = object(&mut e, 0, None);
    e.mem.set_u32(menu + 4, hud_tile);
    e.mem.set_u32(HUD_MAIN_MENU, menu);
    let made_node = node(&mut e, b"TileNode", &[]);
    for i in 0..0x10 {
        e.mem.set_u32(MADE_AT + 4 * i, 0);
        e.mem.set_u32(BARE_AT + 4 * i, 0);
    }
    // `Tile::spModel` (+0x2C, engine-bs `tile.rs`).
    e.mem.set_u32(MADE_AT + 0x2c, made_node);
    let geom = shape(&mut e, b"Shape");
    let bip = node(&mut e, b"Bip01 Head", &[]);
    let root = node(&mut e, b"Root", &[geom, bip]);
    let first_bip = node(&mut e, b"Bip01 Head", &[]);
    let first = node(&mut e, b"First", &[first_bip]);
    (
        e,
        Scene {
            hud_tile,
            made: MADE_AT,
            made_node,
            bare: BARE_AT,
            root,
            bip,
            first,
            first_bip,
        },
    )
}

fn added() -> Vec<Vec<u32>> {
    calls_to(ADD_OBJECT)
}

fn clear() {
    CALLS.with(|c| c.borrow_mut().clear());
}

#[test]
fn attach_ui_xml_adds_the_new_tiles_node_under_the_named_node() {
    let (mut e, sc) = setup();
    let refr = reference(&mut e, 0x1234, false, sc.root);
    let run = |e: &mut Engine, name: &[u8], xml: &[u8]| {
        call_on(e, ATTACH_UI_XML, refr, &[s(name), s(xml)]).1
    };
    assert_eq!(run(&mut e, b"Bip01 Head", b"good.xml"), 1.0);
    let reads = calls_to(READ_FILE);
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0][0], sc.hud_tile, "read under the HUD's tile");
    assert_eq!(added(), vec![vec![sc.bip, sc.made_node, 1]]);
    // An empty name: the root.
    clear();
    assert_eq!(run(&mut e, b"", b"good.xml"), 1.0);
    assert_eq!(added(), vec![vec![sc.root, sc.made_node, 1]]);
    // A geometry block, a missing block: nothing read.
    clear();
    assert_eq!(run(&mut e, b"Shape", b"good.xml"), 0.0);
    assert_eq!(run(&mut e, b"Nowhere", b"good.xml"), 0.0);
    assert!(calls_to(READ_FILE).is_empty());
    assert!(added().is_empty());
    // A tile without a node, no tile: read, nothing added.
    assert_eq!(run(&mut e, b"Bip01 Head", b"bare.xml"), 0.0);
    assert_eq!(run(&mut e, b"Bip01 Head", b"bad.xml"), 0.0);
    assert_eq!(calls_to(READ_FILE).len(), 2);
    assert!(added().is_empty());
    assert_eq!(e.mem.u32(sc.bare + 0x2c), 0);
    let _ = sc.made;
}

#[test]
fn attach_ui_xml_uses_the_seen_root_and_needs_the_hud() {
    let (mut e, sc) = setup();
    // The player in first person: the first-person node's block.
    let p = player(&mut e, sc.root, sc.first, false);
    assert_eq!(
        call_on(
            &mut e,
            ATTACH_UI_XML,
            p,
            &[s(b"Bip01 Head"), s(b"good.xml")]
        )
        .1,
        1.0
    );
    assert_eq!(added(), vec![vec![sc.first_bip, sc.made_node, 1]]);
    // In third person: the root's.
    clear();
    e.mem.set_u8(p + IS_3RD_PERSON, 1);
    assert_eq!(
        call_on(
            &mut e,
            ATTACH_UI_XML,
            p,
            &[s(b"Bip01 Head"), s(b"good.xml")]
        )
        .1,
        1.0
    );
    assert_eq!(added(), vec![vec![sc.bip, sc.made_node, 1]]);
    // No loaded 3D: nothing.
    clear();
    let unloaded = reference(&mut e, 0x99, false, 0);
    assert_eq!(
        call_on(&mut e, ATTACH_UI_XML, unloaded, &[s(b""), s(b"good.xml")]).1,
        0.0
    );
    // No HUD menu yet: nothing read.
    e.mem.set_u32(HUD_MAIN_MENU, 0);
    let refr = reference(&mut e, 0x1234, false, sc.root);
    assert_eq!(
        call_on(
            &mut e,
            ATTACH_UI_XML,
            refr,
            &[s(b"Bip01 Head"), s(b"good.xml")]
        )
        .1,
        0.0
    );
    assert!(calls_to(READ_FILE).is_empty());
    assert!(added().is_empty());
}

#[test]
fn attach_ui_component_attaches_with_the_uio_row_on_or_off() {
    let (mut e, sc) = setup();
    e.register(COMPRESS, |e, w| {
        record(COMPRESS, w);
        let text = nvse::table::guest_text(e, w[1]);
        let data = e.mem.alloc(text.len() as u32 + 1);
        e.mem.write(data, &[text.as_slice(), b"\0"].concat());
        let storage = e.mem.alloc(8);
        e.mem.set_u32(storage, text.len() as u32);
        e.mem.set_u32(storage + 4, data);
        eax(storage)
    });
    let refr = reference(&mut e, 0x1234, false, sc.root);
    let run = |e: &mut Engine, name: &[u8], xml: &[u8]| {
        call_on(e, ATTACH_UI_COMPONENT, refr, &[s(name), s(xml)]).1
    };
    // UIO off: JIP's own path (its `jip_temp.xml` read with `Tile::ReadFile` under the HUD's
    // tile), the text handed over through `Menu::XMLHash`, no file: the same tile attached.
    e.register(XML_SET_AT, |_, w| {
        record(XML_SET_AT, w);
        eax(0)
    });
    e.register(XML_REMOVE_AT, |_, w| {
        record(XML_REMOVE_AT, w);
        eax(1)
    });
    assert_eq!(run(&mut e, b"Bip01 Head", b"<rect/>"), 1.0);
    let reads = calls_to(READ_FILE);
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0][0], sc.hud_tile);
    assert_eq!(calls_to(COMPRESS).len(), 1);
    assert_eq!(calls_to(XML_SET_AT).len(), 1);
    assert_eq!(calls_to(XML_REMOVE_AT).len(), 1);
    assert_eq!(added(), vec![vec![sc.bip, sc.made_node, 1]]);
    clear();
    uio::set_on(&mut e, true);
    uio::install(&mut e);
    assert_eq!(run(&mut e, b"Bip01 Head", br#"<rect name="x"/>"#), 1.0);
    let reads = calls_to(READ_FILE);
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0][0], sc.hud_tile, "made under the HUD's tile");
    assert_eq!(added(), vec![vec![sc.bip, sc.made_node, 1]]);
    // A geometry block or no such block: nothing made; an empty text: no tile.
    clear();
    assert_eq!(run(&mut e, b"Shape", b"<rect/>"), 0.0);
    assert_eq!(run(&mut e, b"Nowhere", b"<rect/>"), 0.0);
    assert!(calls_to(COMPRESS).is_empty());
    assert_eq!(run(&mut e, b"Bip01 Head", b""), 0.0);
    assert!(added().is_empty());
}
