//! Tests of `AttachLine`, called by its opcode on trees built by `nif_nodes_tests.rs`.
//! Expectations from JIP's command body (functions_jip/jip_fn_misc_ref.h), `NiLines::Create`,
//! `NiGeometry::AddProperty`, `NiAVObject::AssignGeometryProps`, `NiObjectNET::SetName`
//! (internal/netimmerse.cpp) and `TESObjectREFR::GetNode2` (nvse/GameObjects.cpp); the game
//! functions are stand-ins at their exe addresses that record their words in order.

use super::super::nif_nodes::tests::*;
use crate::commands::test_support::{allocations, call_on, calls, calls_to, s, z, CALLS};
use crate::game_patches::gp;
use engine_core::prelude::*;

const ATTACH_LINE: u32 = 0x2919;
const ADD_OBJECT: u32 = 0x00a5_ed10;
const NI_ALLOCATE: u32 = 0x00aa_13e0;
const LINES_CONSTRUCT: u32 = 0x00a7_46e0;
const NO_LIGHTING_CONSTRUCT: u32 = 0x00b6_fc90;
const ALPHA_CREATE: u32 = 0x00a5_ceb0;
const LIST_NODE_NEW: u32 = 0x0043_a010;
const ASSIGN_1: u32 = 0x00a5_a040;
const ASSIGN_2: u32 = 0x00b5_7bd0;
const ADD_STRING: u32 = 0x00a5_b690;
const CLONING_CONSTRUCT: u32 = 0x004a_d0c0;
const CLONING_DESTRUCT: u32 = 0x004a_d1d0;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;
/// The test's class words: the line's vtable (its +0x48 `CreateClone`, +0x68 `ProcessClone`).
const LINES_VT: u32 = AREA + 0xe_0000;
const ALPHA_VT: u32 = AREA + 0xe_0400;
const CREATE_CLONE_FN: u32 = 0x0090_0600;
const PROCESS_CLONE_FN: u32 = 0x0090_0610;
/// `1 / 255.0F` as JIP's code holds it.
fn colour_scale() -> f32 {
    f32::from_bits(0x3b80_8081)
}

fn rec(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn setup() -> Engine {
    let mut e = scene_engine();
    // `BSFadeNode`'s slot +0xDC lies on the page after the one the scene maps.
    e.map(0x010a_9000, 0x1000);
    e.mem.set_u32(NI_NODE + 0xdc, ADD_OBJECT);
    e.mem.set_u32(BS_FADE_NODE + 0xdc, ADD_OBJECT);
    e.mem.set_u32(LINES_VT + 0x48, CREATE_CLONE_FN);
    e.mem.set_u32(LINES_VT + 0x68, PROCESS_CLONE_FN);
    e.register(ADD_OBJECT, |_, w| {
        rec(ADD_OBJECT, w);
        eax(0)
    });
    e.register(NI_ALLOCATE, |e, w| {
        rec(NI_ALLOCATE, w);
        eax(object(e, 0, None))
    });
    e.register(LINES_CONSTRUCT, |e, w| {
        rec(LINES_CONSTRUCT, w);
        e.mem.set_u32(w[0], LINES_VT);
        eax(w[0])
    });
    e.register(NO_LIGHTING_CONSTRUCT, |_, w| {
        rec(NO_LIGHTING_CONSTRUCT, w);
        eax(w[0])
    });
    e.register(ALPHA_CREATE, |e, w| {
        rec(ALPHA_CREATE, w);
        let a = object(e, ALPHA_VT, None);
        e.mem.set_u16(a + 0x18, 0xec);
        eax(a)
    });
    e.register(LIST_NODE_NEW, |e, w| {
        rec(LIST_NODE_NEW, w);
        eax(object(e, 0, None))
    });
    e.register(ASSIGN_1, |_, w| {
        rec(ASSIGN_1, w);
        eax(0)
    });
    e.register(ASSIGN_2, |_, w| {
        rec(ASSIGN_2, w);
        eax(0)
    });
    e.register(ADD_STRING, |e, w| {
        let text = nvse::table::guest_text(e, w[0]);
        let mut words = w.to_vec();
        words.extend(text.iter().map(|&b| u32::from(b)));
        rec(ADD_STRING, &words);
        let block = object(e, 0, None);
        let at = block + 0x10;
        e.mem.write(at, &text);
        e.mem.set_u32(at - 8, 1);
        eax(at)
    });
    e.register(CLONING_CONSTRUCT, |_, w| {
        rec(CLONING_CONSTRUCT, w);
        eax(w[0])
    });
    e.register(CLONING_DESTRUCT, |_, w| {
        rec(CLONING_DESTRUCT, w);
        eax(0)
    });
    e.register(CREATE_CLONE_FN, |e, w| {
        rec(CREATE_CLONE_FN, w);
        eax(object(e, LINES_VT, Some(b"Copy")))
    });
    e.register(PROCESS_CLONE_FN, |_, w| {
        rec(PROCESS_CLONE_FN, w);
        eax(0)
    });
    e
}

fn attach(e: &mut Engine, this: u32, node: &[u8], line: &[u8], length: f64, c: [f64; 4]) -> f64 {
    CALLS.with(|x| x.borrow_mut().clear());
    let args = [
        s(node),
        s(line),
        z(length),
        z(c[0]),
        z(c[1]),
        z(c[2]),
        z(c[3]),
    ];
    call_on(e, ATTACH_LINE, this, &args).1
}

/// The order of the game calls the line's making makes (allocations included).
fn sequence() -> Vec<u32> {
    calls().into_iter().map(|c| c.0).collect()
}

#[test]
fn attach_line_makes_jips_line_and_adds_it_under_the_node() {
    let mut e = setup();
    let bip = node(&mut e, b"Bip01 Head", &[]);
    let root = node(&mut e, b"Root", &[bip]);
    let refr = reference(&mut e, 0x1234, false, root);
    let result = attach(
        &mut e,
        refr,
        b"Bip01 Head",
        b"Laser",
        50.0,
        [255.0, 0.0, 127.5, 51.0],
    );
    assert_eq!(result, 1.0);
    assert_eq!(
        sequence(),
        vec![
            HEAP_ALLOCATE,
            HEAP_ALLOCATE,
            HEAP_ALLOCATE,
            NI_ALLOCATE,
            LINES_CONSTRUCT,
            ALPHA_CREATE,
            LIST_NODE_NEW,
            NI_ALLOCATE,
            NO_LIGHTING_CONSTRUCT,
            LIST_NODE_NEW,
            ASSIGN_1,
            ASSIGN_2,
            ADD_STRING,
            ADD_OBJECT,
        ]
    );
    assert_eq!(allocations(), vec![2, 0x20, 0x18]);
    assert_eq!(calls_to(NI_ALLOCATE), vec![vec![0xc4], vec![0x80]]);
    let construct = &calls_to(LINES_CONSTRUCT)[0];
    let lines = construct[0];
    let (vertices, colours, connections) = (construct[2], construct[3], construct[7]);
    assert_eq!(construct[1], 2);
    assert_eq!(construct[4..7], [0, 0, 0]);
    // Connections: (1, 0).
    assert_eq!(e.mem.u16(connections), 1);
    // Two vertices: (0, 0, 0) and (0, 1, 0).
    let v: Vec<f32> = (0..6).map(|i| e.mem.f32(vertices + 4 * i)).collect();
    assert_eq!(v, vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    // The colour twice, each channel times 1 / 255.0F.
    let k = colour_scale();
    let want = [255.0 * k, 0.0 * k, 127.5 * k, 51.0 * k];
    for copy in 0..2 {
        let got: Vec<u32> = (0..4)
            .map(|i| e.mem.f32(colours + 0x10 * copy + 4 * i).to_bits())
            .collect();
        assert_eq!(got, want.map(f32::to_bits).to_vec());
    }
    // The property list: the shader first, then the alpha property; both counted up.
    let head = e.mem.u32(lines + 0x24);
    let tail = e.mem.u32(lines + 0x28);
    assert_eq!(e.mem.u32(lines + 0x2c), 2);
    let shader = e.mem.u32(head + 8);
    let alpha = e.mem.u32(tail + 8);
    assert_eq!(calls_to(NO_LIGHTING_CONSTRUCT)[0][0], shader);
    assert_eq!(e.mem.u32(head), tail, "the first node's next");
    assert_eq!(e.mem.u32(tail + 4), head, "the last node's previous");
    assert_eq!(e.mem.u32(shader + 4), 1);
    assert_eq!(e.mem.u32(alpha + 4), 1);
    assert_eq!(e.mem.u16(alpha + 0x18), 0x10ed, "JIP's alpha flags");
    assert_eq!(calls_to(LIST_NODE_NEW), vec![vec![lines], vec![lines]]);
    // `UpdateProperties` takes the line alone (`this`); `PrepareObjectRecurse` four words.
    assert_eq!(calls_to(ASSIGN_1), vec![vec![lines]]);
    assert_eq!(calls_to(ASSIGN_2), vec![vec![lines, 0, 0, 0]]);
    // Named, its scale the length, added under the node.
    let name = &calls_to(ADD_STRING)[0];
    assert_eq!(
        name[1..].iter().map(|&b| b as u8).collect::<Vec<_>>(),
        b"Laser"
    );
    assert_eq!(nvse::table::guest_text(&e, e.mem.u32(lines + 8)), b"Laser");
    assert_eq!(e.mem.f32(lines + 0x64), 50.0);
    assert_eq!(calls_to(ADD_OBJECT), vec![vec![bip, lines, 1]]);
}

#[test]
fn attach_line_makes_the_alpha_property_once() {
    let mut e = setup();
    let root = node(&mut e, b"Root", &[]);
    let refr = reference(&mut e, 0x1234, false, root);
    attach(&mut e, refr, b"", b"A", 1.0, [0.0; 4]);
    let first = calls_to(LINES_CONSTRUCT)[0][0];
    let alpha = e.mem.u32(e.mem.u32(first + 0x28) + 8);
    assert_eq!(
        calls_to(ADD_OBJECT),
        vec![vec![root, first, 1]],
        "an empty name: the root"
    );
    attach(&mut e, refr, b"", b"B", 1.0, [0.0; 4]);
    assert!(
        calls_to(ALPHA_CREATE).is_empty(),
        "kept from the first line"
    );
    let second = calls_to(LINES_CONSTRUCT)[0][0];
    assert_eq!(e.mem.u32(e.mem.u32(second + 0x28) + 8), alpha);
    assert_eq!(e.mem.u32(alpha + 4), 2);
}

#[test]
fn attach_line_only_under_a_node_of_the_loaded_root() {
    let mut e = setup();
    let geom = shape(&mut e, b"Shape");
    let root = node(&mut e, b"Root", &[geom]);
    let refr = reference(&mut e, 0x1234, false, root);
    // A geometry block (GetNode2 keeps only a node), a missing block: nothing made.
    assert_eq!(attach(&mut e, refr, b"Shape", b"L", 1.0, [0.0; 4]), 0.0);
    assert!(calls().is_empty());
    assert_eq!(attach(&mut e, refr, b"Missing", b"L", 1.0, [0.0; 4]), 0.0);
    assert!(calls().is_empty());
    let unloaded = reference(&mut e, 0x99, false, 0);
    assert_eq!(attach(&mut e, unloaded, b"", b"L", 1.0, [0.0; 4]), 0.0);
    assert!(calls().is_empty());
}

#[test]
fn attach_line_for_the_player_also_copies_it_under_the_first_person_node() {
    let mut e = setup();
    let bip3 = node(&mut e, b"Bip01 Head", &[]);
    let root = node(&mut e, b"Root", &[bip3]);
    let bip1 = node(&mut e, b"Bip01 Head", &[]);
    let camera_first = node(&mut e, b"Camera1st", &[]);
    let jip_first = node(&mut e, b"JIPFirst", &[bip1]);
    // In first person: GetNode2 still takes the loaded (third-person) root.
    let p = player(&mut e, root, camera_first, false);
    // No first-person node made yet: the third-person line only.
    attach(&mut e, p, b"Bip01 Head", b"L", 1.0, [0.0; 4]);
    let lines = calls_to(LINES_CONSTRUCT)[0][0];
    assert_eq!(calls_to(ADD_OBJECT), vec![vec![bip3, lines, 1]]);
    assert!(calls_to(CLONING_CONSTRUCT).is_empty());
    // With JIP's first-person node: a copy under its node of that name.
    gp::set_first_person_node_for_test(&mut e, jip_first);
    attach(&mut e, p, b"Bip01 Head", b"L", 1.0, [0.0; 4]);
    let lines = calls_to(LINES_CONSTRUCT)[0][0];
    let clone = calls_to(CREATE_CLONE_FN);
    assert_eq!(clone.len(), 1);
    assert_eq!(clone[0][0], lines, "the line cloned");
    let added = calls_to(ADD_OBJECT);
    assert_eq!(added.len(), 2);
    assert_eq!(added[0], vec![bip3, lines, 1]);
    assert_eq!(added[1][0], bip1);
    assert_ne!(added[1][1], lines);
    assert_eq!(added[1][2], 1);
    // No node of that name under the first-person node: the third-person line only.
    attach(&mut e, p, b"Root", b"L", 1.0, [0.0; 4]);
    assert_eq!(calls_to(ADD_OBJECT).len(), 1);
    // Another reference: no copy.
    let other = reference(&mut e, 0x77, false, root);
    attach(&mut e, other, b"Bip01 Head", b"L", 1.0, [0.0; 4]);
    assert_eq!(calls_to(ADD_OBJECT).len(), 1);
    assert!(calls_to(CREATE_CLONE_FN).is_empty());
}
