//! `InsertNode`, `AttachModel` and `ModelHasBlock`, called by their opcodes, and the map
//! functions the model hooks use. Expectations from JIP's bodies (functions_jip/jip_fn_misc_ref.h:
//! `RegisterInsertObject`, the three commands; internal/hooks.h: `DoInsertNode`,
//! `DoAttachModel`, `DoInsertObjects`; internal/netimmerse.cpp: `NiNode::Create`,
//! `NiAVObject::GetIndex`), never from the port's output. Game functions are stand-ins that
//! record; fixed strings come from a stand-in table that counts.

use super::*;
use crate::commands::test_support::*;
use std::cell::RefCell;

const STATIC: u32 = FORMS + 0x1000;
const REFR: u32 = FORMS + 0x2000;
const MADE: u32 = FORMS + 0x3000;
const PLAIN: u32 = FORMS + 0x4000;
const FORM_VT: u32 = FORMS + 0x5000;
const REFR_VT: u32 = FORMS + 0x5400;
const NODE_VT: u32 = FORMS + 0x5800;
const LEAF_VT: u32 = FORMS + 0x5c00;
const LOADED: u32 = FORMS + 0x6000;
const ROOT: u32 = FORMS + 0x7000;
const SPINE: u32 = FORMS + 0x7400;
const ARM: u32 = FORMS + 0x7800;
const KIDS: u32 = FORMS + 0x8000;
const NEW_NODES: u32 = FORMS + 0x9000;
const STRINGS: u32 = FORMS + 0xa000;
const RETURN_THIS: u32 = 0x0068_15c0;
const BOUND_TRUE: u32 = 0x0dea_2000;
const BOUND_FALSE: u32 = 0x0dea_2004;
const ATTACH_FN: u32 = 0x0dea_2010;
const SET_AT_FN: u32 = 0x0dea_2014;
const REMOVE_FN: u32 = 0x0dea_2018;
const R_STATIC: u16 = 1;
const R_REFR: u16 = 2;
const R_MADE: u16 = 3;
const R_PLAIN: u16 = 4;

thread_local! {
    static TEXTS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static NODE_CALLS: RefCell<Vec<(u32, u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

fn node_calls() -> Vec<(u32, u32, Vec<u32>)> {
    NODE_CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// The fixed string of `text` (0x40 apart, the count at -8, the length at -4).
fn s_of(text: &[u8]) -> u32 {
    TEXTS.with(|t| {
        let i = t.borrow().iter().position(|x| x == text).expect("made");
        STRINGS + 8 + 0x40 * i as u32
    })
}

fn count(e: &Engine, text: &[u8]) -> u32 {
    e.mem.u32(s_of(text) - 8)
}

/// A node at `at` named `name` (a fixed string made with no count) with `kids`.
fn node(e: &mut Engine, at: u32, name: &[u8], kids: &[u32]) {
    e.mem.set_u32(at, NODE_VT);
    let n = make_string(e, name);
    e.mem.set_u32(at + 8, n);
    let data = KIDS + (at - ROOT) / 4;
    e.mem.set_u32(at + 0xa0, data);
    e.mem.set_u16(at + 0xa6, kids.len() as u16);
    for (i, &k) in kids.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, k);
        e.mem.set_u32(k + 0x18, at);
    }
}

fn make_string(e: &mut Engine, text: &[u8]) -> u32 {
    let at = TEXTS.with(|t| {
        let mut t = t.borrow_mut();
        let i = t.iter().position(|x| x == text).unwrap_or_else(|| {
            t.push(text.to_vec());
            t.len() - 1
        });
        STRINGS + 8 + 0x40 * i as u32
    });
    e.mem.write(at, text);
    e.mem.set_u8(at + text.len() as u32, 0);
    e.mem.set_u32(at - 4, text.len() as u32);
    at
}

fn setup() -> Engine {
    TEXTS.with(|t| t.borrow_mut().clear());
    NODE_CALLS.with(|c| c.borrow_mut().clear());
    files::fresh();
    let mut e = engine();
    e.map(0x011c_3000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    // No opened archive (`GetFileArchived` walks none), its path hash a stand-in.
    e.map(0x011f_8000, 0x1000);
    e.register(0x00af_d270, |_, _| Ret::default());
    // Forms: a static (bound), a reference to it with 3D, a reference made in game, a plain
    // form (not bound).
    for (f, vt, id) in [
        (STATIC, FORM_VT, 0x0000_1001),
        (REFR, REFR_VT, 0x0000_2001),
        (MADE, REFR_VT, 0xff00_0001),
        (PLAIN, FORM_VT + 0x200, 0x0000_3001),
    ] {
        // Type 0x20 (a static): not an inventory type.
        form(&mut e, f, 0x20, id);
        e.mem.set_u32(f, vt);
    }
    e.mem.set_u32(FORM_VT + 0xe8, BOUND_TRUE);
    e.mem.set_u32(FORM_VT + 0x200 + 0xe8, BOUND_FALSE);
    e.mem.set_u32(REFR_VT + 0xf0, 0x008d_0360);
    e.mem.set_u32(REFR + 0x20, STATIC);
    e.mem.set_u32(MADE + 0x20, STATIC);
    e.mem.set_u32(REFR + 0x64, LOADED);
    e.mem.set_u32(MADE + 0x64, LOADED);
    e.mem.set_u32(LOADED + 0x14, ROOT);
    e.register(BOUND_TRUE, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e.register(BOUND_FALSE, |_, _| Ret::default());
    // Nodes: ROOT "Scene Root" > SPINE "Bip01 Spine" > ARM (a leaf, "Arm").
    e.mem.set_u32(NODE_VT + 0xc, RETURN_THIS);
    e.mem.set_u32(NODE_VT + 0xdc, ATTACH_FN);
    e.mem.set_u32(NODE_VT + 0xe0, SET_AT_FN);
    e.mem.set_u32(NODE_VT + 0xe8, REMOVE_FN);
    e.mem.set_u32(LEAF_VT + 0xc, 0x0dea_2020);
    node(&mut e, ARM, b"Arm", &[]);
    e.mem.set_u32(ARM, LEAF_VT);
    node(&mut e, SPINE, b"Bip01 Spine", &[ARM]);
    node(&mut e, ROOT, b"Scene Root", &[SPINE]);
    for f in [ATTACH_FN, SET_AT_FN, REMOVE_FN] {
        e.register(f, |_, _| Ret::default());
    }
    e.register(ATTACH_FN, |_, w| {
        NODE_CALLS.with(|c| c.borrow_mut().push((ATTACH_FN, w[0], w[1..].to_vec())));
        Ret::default()
    });
    e.register(SET_AT_FN, |_, w| {
        NODE_CALLS.with(|c| c.borrow_mut().push((SET_AT_FN, w[0], w[1..].to_vec())));
        Ret::default()
    });
    e.register(REMOVE_FN, |_, w| {
        NODE_CALLS.with(|c| c.borrow_mut().push((REMOVE_FN, w[0], w[1..].to_vec())));
        Ret::default()
    });
    // The string table: one count more for each ask.
    e.register(0x00a5_b690, |e, w| {
        let text = nvse::table::guest_text(e, w[0]);
        let at = make_string(e, &text);
        let c = e.mem.u32(at - 8);
        e.mem.set_u32(at - 8, c + 1);
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    // `NiNode::Create`'s game half: new empty nodes 0x100 apart.
    e.register(0x00a5_f030, |e, _| {
        let n = e.mem.u32(NEW_NODES);
        e.mem.set_u32(NEW_NODES, n + 1);
        let at = NEW_NODES + 0x100 * (n + 1);
        e.mem.set_u32(at, NODE_VT);
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    set_ref(&mut e, R_STATIC, STATIC);
    set_ref(&mut e, R_REFR, REFR);
    set_ref(&mut e, R_MADE, MADE);
    set_ref(&mut e, R_PLAIN, PLAIN);
    // Registered with their patches (H08, H10).
    assert_ne!(crate::test_support::execute_of(&e, 0x28d3), 0);
    e
}

fn insert(e: &mut Engine, r_form: u16, mode: i32, text: &[u8]) -> f64 {
    call_on(e, 0x28d3, 0, &[r(r_form), n(mode), s(text)]).1
}

fn attach(e: &mut Engine, r_form: u16, mode: i32, text: &[u8]) -> f64 {
    call_on(e, 0x28d4, 0, &[r(r_form), n(mode), s(text)]).1
}

// ---- the maps ----

#[test]
fn entries_are_kept_per_form_node_and_object() {
    let mut e = setup();
    assert_eq!(insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA"), 1.0);
    assert_eq!(e.mem.u8(STATIC + 6), INSERT_NODE, "the form flagged");
    assert!(any_insert_objects(&mut e));
    assert_eq!(
        insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA"),
        0.0,
        "already there"
    );
    assert_eq!(insert(&mut e, R_STATIC, 1, b"Bip01 Spine|^NewB"), 1.0);
    assert_eq!(
        insert(&mut e, R_STATIC, 1, b"Root"),
        1.0,
        "no node name: the root"
    );
    let names = node_names(&mut e, true, STATIC).unwrap();
    assert_eq!(names.len(), 2);
    let spine = names
        .iter()
        .find(|(k, _)| *k == s_of(b"Bip01 Spine"))
        .unwrap();
    assert_eq!(
        spine.1,
        vec![(b(b"NewA"), s_of(b"NewA")), (b(b"^NewB"), s_of(b"NewB")),],
        "ordered by a byte compare (`N` before `^`), the put-in names without `^`"
    );
    assert_eq!(
        names.iter().find(|(k, _)| *k == 0).unwrap().1,
        vec![(b(b"Root"), s_of(b"Root"))]
    );
    // The node key holds one count; the setup's own string held none.
    assert_eq!(count(&e, b"Bip01 Spine"), 1);
    assert_eq!(count(&e, b"NewA"), 1);
    // Removed one by one: the counts given back, the flag cleared with the last.
    assert_eq!(insert(&mut e, R_STATIC, 0, b"Bip01 Spine|NewA"), 1.0);
    assert_eq!(count(&e, b"NewA"), 0);
    assert_eq!(
        insert(&mut e, R_STATIC, 0, b"Bip01 Spine|NewA"),
        0.0,
        "gone already"
    );
    assert_eq!(insert(&mut e, R_STATIC, 0, b"Bip01 Spine|^NewB"), 1.0);
    assert_eq!(count(&e, b"Bip01 Spine"), 0, "the empty node's key goes");
    assert_eq!(e.mem.u8(STATIC + 6), INSERT_NODE);
    assert_eq!(insert(&mut e, R_STATIC, 0, b"Root"), 1.0);
    assert_eq!(e.mem.u8(STATIC + 6), 0);
    assert!(!any_insert_objects(&mut e));
    assert!(node_names(&mut e, true, STATIC).is_none());
}

fn b(t: &[u8]) -> Vec<u8> {
    t.to_vec()
}

#[test]
fn refused_requests() {
    let mut e = setup();
    assert_eq!(insert(&mut e, R_PLAIN, 1, b"X"), 0.0, "not a bound object");
    assert_eq!(
        insert(&mut e, R_STATIC, 1, b"Spine|"),
        0.0,
        "no object name"
    );
    assert_eq!(
        insert(&mut e, R_MADE, 1, b"X"),
        0.0,
        "made in game: only mode 3"
    );
    assert!(node_names(&mut e, true, MADE).is_none());
    assert_eq!(
        attach(&mut e, R_STATIC, 1, b"none.nif"),
        0.0,
        "no model file"
    );
    assert_eq!(
        attach(&mut e, R_STATIC, 1, b"*_L"),
        0.0,
        "a suffix with no end"
    );
    assert!(node_names(&mut e, false, STATIC).is_none());
    files::put("data\\meshes\\gun.nif", b"x");
    assert_eq!(attach(&mut e, R_STATIC, 1, b"gun.nif"), 1.0);
    assert_eq!(
        attach(&mut e, R_STATIC, 1, b"*_L*gun.nif"),
        1.0,
        "the path after the suffix"
    );
    assert_eq!(e.mem.u8(STATIC + 6), ATTACH_MODEL);
    let names = node_names(&mut e, false, STATIC).unwrap();
    assert_eq!(
        names[0].1,
        vec![(b(b"*_L*gun.nif"), 0), (b(b"gun.nif"), 0)],
        "no model made yet"
    );
}

// ---- applied now ----

#[test]
fn a_node_inserted_under_its_block_now() {
    let mut e = setup();
    assert_eq!(insert(&mut e, R_REFR, 3, b"Bip01 Spine|NewA"), 1.0);
    let made = NEW_NODES + 0x100;
    assert_eq!(node_calls(), vec![(ATTACH_FN, SPINE, vec![made, 1])]);
    assert_eq!(e.mem.u32(made + 8), s_of(b"NewA"), "named");
    assert_eq!(count(&e, b"NewA"), 2, "the map's and the node's");
    assert_eq!(e.mem.u8(made + 0x33), 0x80, "JIP's block");
    // The root has a block of that name already (the made node isn't a child in this stand-in;
    // name ARM so): nothing.
    assert_eq!(insert(&mut e, R_REFR, 3, b"Bip01 Spine|Arm"), 1.0);
    assert!(node_calls().is_empty());
    // A leaf target: nothing.
    assert_eq!(insert(&mut e, R_REFR, 3, b"Arm|Other"), 1.0);
    assert!(node_calls().is_empty());
}

#[test]
fn a_parent_node_takes_the_targets_place() {
    let mut e = setup();
    assert_eq!(insert(&mut e, R_REFR, 3, b"Arm|^Holder"), 1.0);
    let made = NEW_NODES + 0x100;
    assert_eq!(
        node_calls(),
        vec![
            // `GetIndex`: 1 (JIP counts from the last down, the place plus one).
            (SET_AT_FN, SPINE, vec![1, made]),
            (ATTACH_FN, made, vec![ARM, 1]),
        ]
    );
    // Already the target's parent: nothing.
    assert_eq!(insert(&mut e, R_REFR, 3, b"Arm|^Bip01 Spine"), 1.0);
    assert!(node_calls().is_empty());
    // The root itself: nothing.
    assert_eq!(insert(&mut e, R_REFR, 3, b"^Top"), 1.0);
    assert!(node_calls().is_empty());
}

#[test]
fn removing_takes_the_block_off_now() {
    let mut e = setup();
    insert(&mut e, R_REFR, 1, b"Bip01 Spine|Arm");
    assert_eq!(insert(&mut e, R_REFR, 2, b"Bip01 Spine|Arm"), 1.0);
    assert_eq!(node_calls(), vec![(REMOVE_FN, SPINE, vec![ARM])]);
    assert!(node_names(&mut e, true, REFR).is_none());
}

#[test]
fn a_reference_made_in_game_only_changes_its_3d() {
    let mut e = setup();
    assert_eq!(insert(&mut e, R_MADE, 3, b"Bip01 Spine|NewA"), 1.0);
    assert_eq!(node_calls().len(), 1);
    assert!(node_names(&mut e, true, MADE).is_none(), "nothing kept");
    assert_eq!(count(&e, b"NewA"), 1, "only the node's count");
    assert_eq!(
        count(&e, b"Bip01 Spine"),
        0,
        "the temporary name given back"
    );
}

// ---- the hooks' function ----

#[test]
fn do_insert_objects_applies_a_flagged_forms_entries() {
    let mut e = setup();
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA");
    insert(&mut e, R_STATIC, 1, b"Missing|NewB");
    do_insert_objects(&mut e, STATIC, 0, ROOT);
    let made = NEW_NODES + 0x100;
    assert_eq!(
        node_calls(),
        vec![(ATTACH_FN, SPINE, vec![made, 1])],
        "a missing node: passed"
    );
    // Without the flag: nothing.
    e.mem.set_u8(STATIC + 6, 0);
    do_insert_objects(&mut e, PLAIN, STATIC, ROOT);
    assert!(node_calls().is_empty());
}

// ---- ModelHasBlock ----

#[test]
fn model_has_block_looks_in_the_3d_and_the_maps() {
    let mut e = setup();
    let has = |e: &mut Engine, rf: u16, name: &[u8]| call_on(e, 0x28df, 0, &[r(rf), s(name)]).1;
    assert_eq!(has(&mut e, R_REFR, b"Arm"), 1.0, "in the reference's 3D");
    assert_eq!(has(&mut e, R_REFR, b"Scene Root"), 1.0, "the root itself");
    assert_eq!(has(&mut e, R_REFR, b"Leg"), 0.0);
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|^Holder");
    assert_eq!(
        has(&mut e, R_STATIC, b"Holder"),
        1.0,
        "the base's `^` entry"
    );
    assert_eq!(
        has(&mut e, R_REFR, b"Holder"),
        1.0,
        "through the reference's base form"
    );
    assert_eq!(has(&mut e, R_STATIC, b"^Holder"), 1.0, "the key itself");
    assert_eq!(has(&mut e, R_STATIC, b"Holde"), 0.0, "a whole key only");
    insert(&mut e, R_REFR, 1, b"Leg");
    assert_eq!(
        has(&mut e, R_REFR, b"Leg"),
        1.0,
        "the reference's own entry"
    );
}

// ---- AttachModel now ----

const MODEL: u32 = FORMS + 0xc000;
const MODEL_ROOT_NODE: u32 = FORMS + 0xc400;
const COPY: u32 = FORMS + 0xd000;
const COPY_CHILD: u32 = FORMS + 0xd400;
const UPDATE_FN: u32 = 0x0dea_2030;

/// The model loader's map finds `gun.nif`: a model (count 0) whose root has one child; its
/// copy (the clone stand-in answers COPY, named as the root, with one child named "Barrel").
fn model_setup(e: &mut Engine) {
    e.mem.set_u32(0x011c_3b3c, FORMS + 0xc300);
    e.mem.set_u32(FORMS + 0xc300, FORMS + 0xc310);
    e.register(0x0044_93d0, |e, w| {
        let path = nvse::table::guest_text(e, w[1]);
        let found = path.eq_ignore_ascii_case(b"gun.nif");
        e.mem.set_u32(w[2], if found { MODEL } else { 0 });
        Ret {
            eax: u32::from(found),
            ..Ret::default()
        }
    });
    e.mem.set_u32(MODEL + 0xc, MODEL_ROOT_NODE);
    node(e, MODEL_ROOT_NODE, b"GunRoot", &[]);
    e.mem.set_u32(NODE_VT + 0x48, 0x0dea_2040);
    e.mem.set_u32(NODE_VT + 0x68, 0x0dea_2044);
    e.mem.set_u32(NODE_VT + 0xc0, UPDATE_FN);
    for f in [0x004a_d0c0, 0x004a_d1d0, 0x0dea_2044, UPDATE_FN] {
        e.register(f, |_, _| Ret::default());
    }
    e.register(0x0dea_2040, |e, _| {
        node(e, COPY_CHILD, b"Barrel", &[]);
        e.mem.set_u32(COPY_CHILD, LEAF_VT);
        node(e, COPY, b"GunRoot", &[COPY_CHILD]);
        Ret {
            eax: COPY,
            ..Ret::default()
        }
    });
    files::put(r"data\meshes\gun.nif", b"x");
}

#[test]
fn a_model_attached_now_and_its_name_kept() {
    let mut e = setup();
    model_setup(&mut e);
    assert_eq!(attach(&mut e, R_REFR, 3, b"Bip01 Spine|gun.nif"), 1.0);
    assert_eq!(node_calls(), vec![(ATTACH_FN, SPINE, vec![COPY, 1])]);
    assert_eq!(e.mem.u32(MODEL + 8), 1, "the model's count raised from 0");
    assert_eq!(
        e.mem.u8(MODEL_ROOT_NODE + 0x33),
        0x40,
        "its lights made ready once"
    );
    assert_eq!(e.mem.u8(COPY + 0x33), 0x80, "JIP's block");
    let names = node_names(&mut e, false, REFR).unwrap();
    assert_eq!(
        names[0].1,
        vec![(b(b"gun.nif"), s_of(b"GunRoot"))],
        "the copy's root name kept"
    );
    // A suffix: every block of the copy renamed.
    assert_eq!(attach(&mut e, R_REFR, 3, b"*_L*gun.nif"), 1.0);
    assert_eq!(
        nvse::table::guest_text(&e, e.mem.u32(COPY + 8)),
        b"GunRoot_L"
    );
    assert_eq!(
        nvse::table::guest_text(&e, e.mem.u32(COPY_CHILD + 8)),
        b"Barrel_L"
    );
    assert_eq!(
        node_calls(),
        vec![(ATTACH_FN, ROOT, vec![COPY, 1])],
        "no node name: the root"
    );
}

// ---- ReloadEquippedModels ----

const CHAR: u32 = FORMS + 0xe000;
const CHAR_VT: u32 = FORMS + 0xe400;
const BIPED: u32 = FORMS + 0xe800;
const PROCESS: u32 = FORMS + 0xeb00;
const WEAPON: u32 = FORMS + 0xec00;
const SLOT_NODE: u32 = FORMS + 0xed00;

#[test]
fn reload_equipped_models_reloads_the_slots() {
    let mut e = setup();
    form(&mut e, CHAR, 0x3b, 0x0000_5001);
    e.mem.set_u32(CHAR, CHAR_VT);
    e.mem.set_u32(CHAR_VT + 0x218, BOUND_TRUE);
    e.mem.set_u32(CHAR + 0x20, STATIC);
    e.mem.set_u32(CHAR + 0x64, LOADED);
    e.mem.set_u32(CHAR + 0x1b4, BIPED);
    e.mem.set_u32(CHAR + 0x68, PROCESS);
    e.mem.set_u32(BIPED + 0x2b0, CHAR);
    // Slot 3 holds a node under SPINE; slot 5 a weapon without flag 0x10.
    e.mem.set_u32(BIPED + 0x2c + 3 * 0x10 + 8, SLOT_NODE);
    e.mem.set_u32(SLOT_NODE + 0x18, SPINE);
    e.mem.set_u32(BIPED + 0x2c + 5 * 0x10, WEAPON);
    for f in [0x0060_6540, 0x004a_c1e0, 0x0049_9240] {
        e.register(f, |_, w| {
            CALLS.with(|c| c.borrow_mut().push((0xbeef, w.to_vec())));
            Ret::default()
        });
    }
    e.mem.set_u32(NODE_VT + 0xa4, UPDATE_FN);
    e.register(UPDATE_FN, |_, _| Ret::default());
    let anim = FORMS + 0xef00;
    e.mem.set_u32(PROCESS + 0x1c0, anim);
    assert_eq!(call_on(&mut e, 0x280d, CHAR, &[n(3)]), (1, 0.0));
    assert_eq!(
        e.mem.u32(BIPED + 0x2c + 3 * 0x10 + 8),
        0,
        "the slot's node taken"
    );
    assert_eq!(node_calls(), vec![(REMOVE_FN, SPINE, vec![SLOT_NODE])]);
    let reloads: Vec<Vec<u32>> = calls()
        .into_iter()
        .filter(|c| c.0 == 0xbeef)
        .map(|c| c.1)
        .collect();
    assert_eq!(
        reloads,
        vec![vec![STATIC, CHAR, BIPED, 0], vec![BIPED, 0], vec![anim, 0]],
        "the models loaded again, the biped updated, the animation data refreshed"
    );
    // The Pip-Boy's slot and past 19: nothing.
    e.mem.set_u32(BIPED + 0x2c + 3 * 0x10 + 8, SLOT_NODE);
    let before = calls().len();
    call_on(&mut e, 0x280d, CHAR, &[n(6)]);
    call_on(&mut e, 0x280d, CHAR, &[n(20)]);
    assert!(node_calls().is_empty());
    assert_eq!(calls().len(), before, "nothing reloaded");
    // Not in high process: nothing.
    e.mem.set_u32(PROCESS + 0x28, 1);
    call_on(&mut e, 0x280d, CHAR, &[]);
    assert!(node_calls().is_empty());
}

/// A put-in name already kept is left as it is (the copy's root name is taken only for an empty
/// slot, `cmp dword ptr [eax], 0`).
#[test]
fn an_attached_models_name_only_fills_an_empty_slot() {
    let mut e = setup();
    model_setup(&mut e);
    let kept = make_string(&mut e, b"Kept");
    let mut slot = kept;
    let made = do_attach_model(&mut e, SPINE, b"gun.nif", &mut slot, ROOT);
    assert_eq!(made, COPY);
    assert_eq!(slot, kept);
    let mut empty = 0;
    do_attach_model(&mut e, SPINE, b"gun.nif", &mut empty, ROOT);
    assert_eq!(empty, s_of(b"GunRoot"));
}

// ---- RefreshAnimData (Actor::RefreshAnimData) ----

#[test]
fn refresh_anim_data_refreshes_and_blends() {
    let mut e = setup();
    const PC: u32 = FORMS + 0xf000;
    const PC_VT: u32 = FORMS + 0xf400;
    const ANIM: u32 = FORMS + 0xf800;
    const ANIM_1ST: u32 = FORMS + 0xfa00;
    form(&mut e, PC, 0x3b, 0x14);
    e.mem.set_u32(PC, PC_VT);
    e.mem.set_u32(PC_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(PC + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x1c0, ANIM);
    e.mem.set_u32(PC + 0x690, ANIM_1ST);
    for f in [0x0049_9240, 0x0049_94f0] {
        e.register(f, |_, w| {
            CALLS.with(|c| c.borrow_mut().push((0xa0a0, w.to_vec())));
            Ret::default()
        });
    }
    let anim_calls = || -> Vec<Vec<u32>> {
        CALLS.with(|c| {
            let v = c
                .borrow()
                .iter()
                .filter(|x| x.0 == 0xa0a0)
                .map(|x| x.1.clone())
                .collect();
            c.borrow_mut().retain(|x| x.0 != 0xa0a0);
            v
        })
    };
    // The data's word alone does not blend: the process's byte is 0.
    e.mem.set_u32(ANIM + 0xf0, 1);
    assert_eq!(call_on(&mut e, 0x290c, PC, &[]), (1, 0.0));
    assert_eq!(
        anim_calls(),
        vec![vec![ANIM, 0], vec![ANIM_1ST, 0]],
        "the player's two"
    );
    // The process byte and the data's word: blended first (the first-person data's own word).
    e.mem.set_u8(PROCESS + 0x135, 1);
    e.mem.set_u32(ANIM + 0xf0, 1);
    call_on(&mut e, 0x290c, PC, &[]);
    assert_eq!(
        anim_calls(),
        vec![vec![ANIM, 4, 0], vec![ANIM, 0], vec![ANIM_1ST, 0]]
    );
    // Not the player: only its own; not high process or no data: nothing.
    e.mem.set_u32(PC + 0xc, 0x15);
    call_on(&mut e, 0x290c, PC, &[]);
    assert_eq!(anim_calls().len(), 2);
    e.mem.set_u32(PROCESS + 0x28, 1);
    call_on(&mut e, 0x290c, PC, &[]);
    assert!(anim_calls().is_empty());
}

// ---- ReloadEquippedModels: the player, the re-equipped weapon, every slot ----

const PCHAR: u32 = FORMS + 0x1_0000;
const PBIPED: u32 = FORMS + 0x1_0400;
const PBIPED_1ST: u32 = FORMS + 0x1_0800;
const PNODE_1ST: u32 = FORMS + 0x1_0c00;
const PPROCESS: u32 = FORMS + 0x1_1000;
const PPROCESS_VT: u32 = FORMS + 0x1_1400;
const PWEAPON: u32 = FORMS + 0x1_1800;
const PENTRY: u32 = FORMS + 0x1_1c00;
const PXLIST: u32 = FORMS + 0x1_1d00;
const PXDATA: u32 = FORMS + 0x1_1e00;
const BIP01: u32 = FORMS + 0x1_2000;
const BACKPACK: u32 = FORMS + 0x1_2100;
const SLOT_NODES: u32 = FORMS + 0x1_3000;
const QUEUE_EQUIP_FN: u32 = 0x0dea_2050;

/// The player (`0x14`) with 3D, a third- and a first-person biped (the second's node
/// `PNODE_1ST`), in high process, its weapon in slot 5 (flags `0x10 | 4`: re-equipped, scoped),
/// the process's weapon entry that weapon with one extra data list `PXDATA`; Bip01 with a
/// "Backpack" node. Every game call recorded under its address.
fn player_scene() -> Engine {
    let mut e = setup();
    form(&mut e, PCHAR, 0x3b, 0x14);
    e.mem.set_u32(PCHAR, CHAR_VT);
    e.mem.set_u32(CHAR_VT + 0x218, BOUND_TRUE);
    e.mem.set_u32(PCHAR + 0x20, STATIC);
    e.mem.set_u32(PCHAR + 0x64, LOADED);
    e.mem.set_u32(PCHAR + 0x1b4, PBIPED);
    e.mem.set_u32(PCHAR + 0x68C, PBIPED_1ST);
    e.mem.set_u32(PCHAR + 0x694, PNODE_1ST);
    e.mem.set_u32(PCHAR + 0x68, PPROCESS);
    e.mem.set_u32(PPROCESS, PPROCESS_VT);
    e.mem.set_u32(PPROCESS_VT + 0x178, QUEUE_EQUIP_FN);
    for b in [PBIPED, PBIPED_1ST] {
        e.mem.set_u32(b + 0x2b0, PCHAR);
    }
    node(&mut e, PNODE_1ST, b"1st", &[]);
    form(&mut e, PWEAPON, 0x28, 0x0000_6001);
    e.mem.set_u8(PWEAPON + 0x100, 0x14);
    e.mem.set_u32(PBIPED + 0x2c + 5 * 0x10, PWEAPON);
    e.mem.set_u32(PPROCESS + 0x114, PENTRY);
    e.mem.set_u32(PENTRY, PXLIST);
    e.mem.set_u32(PXLIST, PXDATA);
    e.mem.set_u32(PENTRY + 8, PWEAPON);
    e.mem.set_u32(PBIPED, BIP01);
    node(&mut e, BACKPACK, b"Backpack", &[]);
    node(&mut e, BIP01, b"Bip01", &[BACKPACK]);
    // Slots 0, 3 and 6 hold nodes under SPINE.
    for slot in [0u32, 3, 6] {
        let n = SLOT_NODES + 0x40 * slot;
        e.mem.set_u32(PBIPED + 0x2c + slot * 0x10 + 8, n);
        e.mem.set_u32(n + 0x18, SPINE);
    }
    macro_rules! stand_ins {
        ($($f:expr),*) => {$(
            e.register($f, |_, w| {
                CALLS.with(|c| c.borrow_mut().push(($f, w.to_vec())));
                Ret::default()
            });
        )*};
    }
    stand_ins!(
        0x0060_6540u32,
        0x004a_c1e0,
        0x0049_9240,
        0x0077_f270,
        0x0077_f2f0,
        QUEUE_EQUIP_FN,
        UPDATE_FN
    );
    e.mem.set_u32(NODE_VT + 0xa4, UPDATE_FN);
    e
}

fn of(addr: u32) -> Vec<Vec<u32>> {
    calls()
        .into_iter()
        .filter(|c| c.0 == addr)
        .map(|c| c.1)
        .collect()
}

#[test]
fn the_players_weapon_slot_reloads_both_views_and_reequips() {
    let mut e = player_scene();
    assert_eq!(call_on(&mut e, 0x280d, PCHAR, &[n(5)]), (1, 0.0));
    assert_eq!(
        node_calls(),
        vec![(REMOVE_FN, BIP01, vec![BACKPACK])],
        "the backpack node taken first (slot 5 has no node)"
    );
    assert_eq!(
        of(0x0060_6540),
        vec![
            vec![STATIC, PCHAR, PBIPED, 0],
            vec![STATIC, PCHAR, PBIPED_1ST, 0]
        ],
        "both bipeds' models loaded again"
    );
    assert_eq!(of(0x004a_c1e0), vec![vec![PBIPED, 0], vec![PBIPED_1ST, 0]]);
    let updates = of(UPDATE_FN);
    assert_eq!(updates.len(), 2);
    assert_eq!(
        (updates[0][0], updates[0][2]),
        (ROOT, 0),
        "the root's downward update"
    );
    assert_eq!(
        (updates[1][0], updates[1][2]),
        (PNODE_1ST, 0),
        "then the first-person node's"
    );
    assert_eq!(
        of(0x0077_f270),
        vec![Vec::<u32>::new()],
        "the HUD's weapon refresh"
    );
    assert_eq!(
        of(0x0077_f2f0),
        vec![vec![PWEAPON + 0x1e8]],
        "the scope with the weapon's model"
    );
    assert_eq!(
        of(QUEUE_EQUIP_FN),
        vec![vec![PPROCESS, PCHAR, 1, PWEAPON, 1, PXDATA, 1, 0, 1, 0, 0]],
        "equipped again: the actor, true, the weapon, 1, its first list, true, false, true, 0, false"
    );
    for slot in [0u32, 3, 6] {
        let n = SLOT_NODES + 0x40 * slot;
        assert_eq!(
            e.mem.u32(PBIPED + 0x2c + slot * 0x10 + 8),
            n,
            "slot {slot} left"
        );
    }
}

#[test]
fn no_argument_reloads_every_slot_but_the_pipboys() {
    let mut e = player_scene();
    // The weapon entry is of another weapon: no equip again.
    e.mem.set_u32(PENTRY + 8, PWEAPON + 0x40);
    call_on(&mut e, 0x280d, PCHAR, &[]);
    for slot in [0u32, 3] {
        assert_eq!(
            e.mem.u32(PBIPED + 0x2c + slot * 0x10 + 8),
            0,
            "slot {slot} taken"
        );
    }
    assert_eq!(
        e.mem.u32(PBIPED + 0x2c + 6 * 0x10 + 8),
        SLOT_NODES + 0x40 * 6,
        "the Pip-Boy's slot (mask 0xFFFBF) left"
    );
    let removed: Vec<_> = node_calls().into_iter().map(|c| c.2[0]).collect();
    assert_eq!(removed, vec![BACKPACK, SLOT_NODES, SLOT_NODES + 0xc0]);
    assert!(
        of(QUEUE_EQUIP_FN).is_empty(),
        "another weapon's entry: not equipped again"
    );
    assert_eq!(
        of(0x0077_f270).len(),
        1,
        "the weapon slot is among them: HUD refreshed"
    );
}

#[test]
fn another_slot_of_the_player_leaves_the_hud() {
    let mut e = player_scene();
    call_on(&mut e, 0x280d, PCHAR, &[n(3)]);
    assert!(of(0x0077_f270).is_empty());
    assert!(of(QUEUE_EQUIP_FN).is_empty());
    assert_eq!(
        node_calls(),
        vec![(REMOVE_FN, SPINE, vec![SLOT_NODES + 0xc0])],
        "no backpack"
    );
}

// ---- ModelHasBlock: a base form's model file ----

#[test]
fn model_has_block_loads_a_base_forms_model() {
    let mut e = setup();
    const MODEL_FORM: u32 = FORMS + 0x1_4000;
    const MODEL_TEXT: u32 = FORMS + 0x1_4100;
    const LOADED_ROOT: u32 = FORMS + 0x1_4200;
    const LOADED_KID: u32 = FORMS + 0x1_4300;
    // A static (type 0x20, its model component at word 0x10 by `kBaseComponentOffsets`).
    let model = super::super::model_component(&e, STATIC);
    assert_ne!(model, 0, "statics have a model");
    let _ = MODEL_FORM;
    e.mem.write(MODEL_TEXT, b"box.nif\0");
    e.mem.set_u32(model + 4, MODEL_TEXT);
    node(&mut e, LOADED_KID, b"Lid", &[]);
    node(&mut e, LOADED_ROOT, b"BoxRoot", &[LOADED_KID]);
    e.mem.set_u32(0x011c_3b3c, FORMS + 0x1_4400);
    e.register(0x0044_7080, |e, w| {
        let mut words = w.to_vec();
        words[1] = 0;
        CALLS.with(|c| c.borrow_mut().push((0x0044_7080, words)));
        assert_eq!(nvse::table::guest_text(e, w[1]), b"box.nif");
        Ret {
            eax: LOADED_ROOT,
            ..Ret::default()
        }
    });
    let has = |e: &mut Engine, rf: u16, name: &[u8]| call_on(e, 0x28df, 0, &[r(rf), s(name)]).1;
    assert_eq!(has(&mut e, R_STATIC, b"Lid"), 1.0);
    assert_eq!(
        of(0x0044_7080),
        vec![vec![FORMS + 0x1_4400, 0, 0, 1, 0, 0, 1]],
        "the loader, the path (checked), 0, 1, 0, 0, 1"
    );
    assert_eq!(
        has(&mut e, R_STATIC, b"Hinge"),
        0.0,
        "a name the file has not"
    );
    // A reference with 3D: its 3D only, the file never loaded.
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(has(&mut e, R_REFR, b"Hinge"), 0.0);
    assert!(of(0x0044_7080).is_empty());
}

// ---- With the model hooks: H10 (biped parts, add-ons) and H08 (the clone test) ----

/// H10's biped part seam with `InsertNode`: a part whose form has entries gets them in its clone
/// (here the scene's root, which takes the model root's name first), and the clone is named
/// `"<slot name> <form id>"`; a form with none is only named.
#[test]
fn h10_a_biped_part_gets_its_forms_entries_and_jips_name() {
    let mut e = setup();
    e.map(0x0118_8000, 0x1000);
    let slot_name = FORMS + 0xf000;
    e.mem.write(slot_name, b"UpperBody\0");
    e.mem.set_u32(0x0118_8b98 + 4 * 2, slot_name);
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA");
    node_calls();
    let seam = crate::game_patches::gh2::BIPED_PART_NAME.addr();
    // The form STATIC, the clone ROOT, the model's root ROOT, slot 2, third person.
    assert_eq!(e.site(seam, &[STATIC, ROOT, ROOT, 2, 0]), Some(1));
    let made = NEW_NODES + 0x100;
    assert_eq!(node_calls(), vec![(ATTACH_FN, SPINE, vec![made, 1])]);
    assert_eq!(e.mem.u32(ROOT + 8), s_of(b"UpperBody 00001001"));
    // A plain form: only the name.
    assert_eq!(e.site(seam, &[PLAIN, ARM, ARM, 2, 0]), Some(1));
    assert!(node_calls().is_empty());
}

/// H10's add-on seam with `InsertNode`: the clone's rotation is the identity and its translation
/// 0, and a form with entries gets them in the clone.
#[test]
fn h10_an_add_on_gets_its_forms_entries_and_a_reset_transform() {
    let mut e = setup();
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA");
    node_calls();
    for at in (0x34..0x64).step_by(4) {
        e.mem.set_f32(ROOT + at, 7.0);
    }
    let seam = crate::game_patches::gh2::ADD_ON_TRANSFORM.addr();
    assert_eq!(e.site(seam, &[ROOT, STATIC, 0]), Some(1));
    let floats: Vec<f32> = (0..12).map(|i| e.mem.f32(ROOT + 0x34 + 4 * i)).collect();
    assert_eq!(
        floats,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(
        node_calls(),
        vec![(ATTACH_FN, SPINE, vec![NEW_NODES + 0x100, 1])]
    );
}

/// H08: a form or a reference with entries is cloned for each placed object (the clone seam
/// answers 1); without entries the game's checks go on (0).
#[test]
fn h08_a_form_with_entries_is_cloned() {
    let mut e = setup();
    let seam = crate::game_patches::gl::lights::CLONE_3D.addr();
    assert_eq!(e.site(seam, &[0, STATIC, 0]), Some(0));
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA");
    assert_eq!(e.site(seam, &[0, STATIC, 0]), Some(1), "the form");
    assert_eq!(e.site(seam, &[0, PLAIN, STATIC]), Some(1), "the reference");
    assert_eq!(e.site(seam, &[0, PLAIN, 0]), Some(0));
}

/// A request kept for the main loop (asked on another thread) runs only while its kept id still
/// names the very form that was asked: the id in no form map (a load dropped it): nothing done and
/// the request gone; another form holding the id now (a reference rebuilt elsewhere, or a made-in-
/// game id reused): that one gets nothing either, and the old pointer is never read; the same form
/// under its id: it runs.
#[test]
fn a_kept_request_runs_only_for_the_same_form_under_its_id() {
    let mut e = setup();
    game_globals(&mut e);
    let keep = |e: &mut Engine, key: u32, form: u32, id: u32| {
        let req = Request {
            form,
            mode: 1,
            flag: INSERT_NODE,
            text: b"Bip01 Spine|NewA".to_vec(),
        };
        maps(e).pending.insert(key, (req, id));
    };
    // The id is in no form map: dropped.
    keep(&mut e, 7, STATIC, 0x0000_1001);
    run_pending(&mut e, 7);
    assert!(maps(&mut e).pending.is_empty(), "taken");
    assert_eq!(e.mem.u8(STATIC + 6), 0, "nothing done to the old pointer");
    assert!(node_names(&mut e, true, STATIC).is_none());
    // The id now names another form (a made-in-game id reused by a new reference after a load).
    let other = FORMS + 0xb000;
    form(&mut e, other, 0x20, 0xff00_0002);
    e.mem.set_u32(other, FORM_VT);
    in_form_map(&mut e, other, 0xff00_0002, 0);
    keep(&mut e, 8, STATIC, 0xff00_0002);
    run_pending(&mut e, 8);
    assert_eq!(
        e.mem.u8(other + 6),
        0,
        "a different form with the kept id: nothing"
    );
    assert!(node_names(&mut e, true, other).is_none());
    assert!(node_names(&mut e, true, STATIC).is_none());
    // The same form under its id: run.
    in_form_map(&mut e, STATIC, 0x0000_1001, 0);
    keep(&mut e, 9, STATIC, 0x0000_1001);
    run_pending(&mut e, 9);
    assert_eq!(e.mem.u8(STATIC + 6), INSERT_NODE);
    assert!(node_names(&mut e, true, STATIC).is_some());
    // An unknown key: nothing.
    run_pending(&mut e, 99);
}

// ---- H10's name counts and its light and node conditions ----

const LIGHT: u32 = FORMS + 0xc000;
const LIGHT_FORM: u32 = FORMS + 0xc400;
const LIT_NODE: u32 = FORMS + 0x7c00;

/// A node `LIT_NODE` ("Lamp", marked by JIP's light hooks: byte +0x33 bit 0x20) holding one of
/// JIP's point lights (`LIGHT`, marked, with a form); the slot names' page with slot 2 "UpperBody".
fn lit_scene(e: &mut Engine) {
    e.map(0x0118_8000, 0x1000);
    let slot_name = FORMS + 0xf000;
    e.mem.write(slot_name, b"UpperBody\0");
    e.mem.set_u32(0x0118_8b98 + 4 * 2, slot_name);
    e.mem
        .set_u32(LIGHT, crate::game_patches::gl::lights::POINT_LIGHT_VTABLE);
    e.mem.set_u8(LIGHT + 0x33, 0x20);
    e.mem.set_u32(LIGHT + 0xe8, LIGHT_FORM);
    node(e, LIT_NODE, b"Lamp", &[LIGHT]);
    e.mem.set_u8(LIT_NODE + 0x33, 0x20);
}

/// Whether `LIGHT` was listed (its done mark, byte +0x33 bit 0x40, set).
fn lit(e: &Engine) -> bool {
    e.mem.u8(LIGHT + 0x33) & 0x40 != 0
}

/// H10's rename keeps the fixed strings' counts as JIP's `SetName` does (its `lock`-ed adds): the
/// new name holds one count, the old name one fewer; naming a block the name it has leaves the
/// count as it was. With entries the clone first takes the model root's name with one count more
/// (JIP's `lock inc`; the clone's own name is overwritten, its count kept).
#[test]
fn h10_the_names_counts_after_a_rename() {
    let mut e = setup();
    lit_scene(&mut e);
    let seam = crate::game_patches::gh2::BIPED_PART_NAME.addr();
    let lamp = s_of(b"Lamp");
    e.mem.set_u32(lamp - 8, 5);
    // A plain form (no entries), first person (no lights): only the name.
    assert_eq!(e.site(seam, &[PLAIN, LIT_NODE, ROOT, 2, 1]), Some(1));
    let named = e.mem.u32(LIT_NODE + 8);
    assert_eq!(named, s_of(b"UpperBody 00003001"));
    assert_eq!(
        count(&e, b"UpperBody 00003001"),
        1,
        "the new name: one count"
    );
    assert_eq!(e.mem.u32(lamp - 8), 4, "the old name: one fewer");
    // The same name again: unchanged.
    assert_eq!(e.site(seam, &[PLAIN, LIT_NODE, ROOT, 2, 1]), Some(1));
    assert_eq!(e.mem.u32(LIT_NODE + 8), named);
    assert_eq!(count(&e, b"UpperBody 00003001"), 1);
    // With entries: the root's name taken with one count more, then the part named; the root
    // name's count back where it was and the part name's old string not given back.
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA");
    let root_name = s_of(b"Scene Root");
    e.mem.set_u32(root_name - 8, 3);
    let held = count(&e, b"UpperBody 00003001");
    assert_eq!(e.site(seam, &[STATIC, LIT_NODE, ROOT, 2, 1]), Some(1));
    assert_eq!(
        e.mem.u32(root_name - 8),
        3,
        "+1 taken, -1 given back by the rename"
    );
    assert_eq!(
        count(&e, b"UpperBody 00003001"),
        held,
        "the overwritten name's count kept"
    );
    assert_eq!(e.mem.u32(LIT_NODE + 8), s_of(b"UpperBody 00001001"));
}

/// H10's biped part conditions: a node clone marked by the light hooks is lit in third person,
/// not in first person; a clone that is not a node is only named (no entries, no lights).
#[test]
fn h10_a_biped_parts_lights_and_node_test() {
    let seam = crate::game_patches::gh2::BIPED_PART_NAME.addr();
    let mut e = setup();
    lit_scene(&mut e);
    assert_eq!(e.site(seam, &[PLAIN, LIT_NODE, ROOT, 2, 0]), Some(1));
    assert!(lit(&e), "third person: lit");

    let mut e = setup();
    lit_scene(&mut e);
    assert_eq!(e.site(seam, &[PLAIN, LIT_NODE, ROOT, 2, 1]), Some(1));
    assert!(!lit(&e), "first person: not lit");

    let mut e = setup();
    lit_scene(&mut e);
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA");
    node_calls();
    // ARM is a leaf: its slot +0xC is not the shared `return this`.
    e.mem.set_u8(ARM + 0x33, 0x20);
    assert_eq!(e.site(seam, &[STATIC, ARM, ROOT, 2, 0]), Some(1));
    assert!(node_calls().is_empty(), "not a node: no entries");
    assert_eq!(e.mem.u32(ARM + 8), s_of(b"UpperBody 00001001"), "named");
}

/// H10's add-on conditions: lit unless its root override is JIP's first-person node; with form 0
/// only the transform is reset (no entries, no lights).
#[test]
fn h10_an_add_ons_lights_and_form_zero() {
    let seam = crate::game_patches::gh2::ADD_ON_TRANSFORM.addr();
    let first = FORMS + 0xe000;
    let mut e = setup();
    lit_scene(&mut e);
    crate::game_patches::gp::set_first_person_node_for_test(&mut e, first);
    assert_eq!(e.site(seam, &[LIT_NODE, PLAIN, first]), Some(1));
    assert!(!lit(&e), "under the first-person node: not lit");

    let mut e = setup();
    lit_scene(&mut e);
    crate::game_patches::gp::set_first_person_node_for_test(&mut e, first);
    assert_eq!(e.site(seam, &[LIT_NODE, PLAIN, first + 0x100]), Some(1));
    assert!(lit(&e), "another root: lit");

    let mut e = setup();
    lit_scene(&mut e);
    insert(&mut e, R_STATIC, 1, b"Bip01 Spine|NewA");
    node_calls();
    for at in (0x34..0x64).step_by(4) {
        e.mem.set_f32(LIT_NODE + at, 7.0);
    }
    assert_eq!(e.site(seam, &[LIT_NODE, 0, 0]), Some(1));
    assert!(!lit(&e), "form 0: no lights");
    assert!(node_calls().is_empty(), "form 0: no entries");
    assert_eq!(e.mem.f32(LIT_NODE + 0x34), 1.0, "the transform reset");
    assert_eq!(e.mem.f32(LIT_NODE + 0x38), 0.0);
}

// ---- The queued reference's attach (group GP): a reference loaded later gets its entries ----

thread_local! {
    /// At each node attach: the light's byte +0x33 then (whether the light step ran before).
    static LIGHT_AT_ATTACH: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A queued reference task for `refr` (flags `0x820`: the cell's load is skipped, as for a
/// reference already loaded), its cell and its loader stand-ins, the root `ROOT` holding `SPINE`
/// and `LIGHT` (the root marked by the light hooks), and an attach stand-in that records the
/// light's mark at each attach.
fn queued(e: &mut Engine, refr: u32) -> u32 {
    lit_scene(e);
    node(e, ROOT, b"Scene Root", &[SPINE, LIGHT]);
    e.mem.set_u8(ROOT + 0x33, 0x20);
    let task = FORMS + 0xd000;
    let cell = FORMS + 0xd100;
    let loader = FORMS + 0xd200;
    e.mem.set_u32(task + 0x28, refr);
    e.mem.set_u32(refr + 8, e.mem.u32(refr + 8) | 0x820);
    e.mem.set_u32(refr + 0x40, cell);
    e.mem.set_u32(0x011c_3b3c, loader);
    for f in [0x0090_62e0, 0x0044_0a10, 0x0044_0a50] {
        e.register(f, |_, _| Ret::default());
    }
    LIGHT_AT_ATTACH.with(|l| l.borrow_mut().clear());
    e.register(ATTACH_FN, |e, w| {
        LIGHT_AT_ATTACH.with(|l| l.borrow_mut().push(e.mem.u8(LIGHT + 0x33)));
        NODE_CALLS.with(|c| c.borrow_mut().push((ATTACH_FN, w[0], w[1..].to_vec())));
        Ret::default()
    });
    task
}

/// The names of the nodes attached, in order (the inserted nodes' names).
fn attached_names(e: &Engine) -> Vec<(u32, u32)> {
    node_calls()
        .into_iter()
        .filter(|c| c.0 == ATTACH_FN)
        .map(|(_, parent, w)| (parent, e.mem.u32(w[0] + 8)))
        .collect()
}

/// A reference and its base with entries: both forms' entries go under the loaded 3D, the
/// reference's first (JIP's `DoInsertObjects(refr, base)`), before the point-light step.
#[test]
fn a_loaded_reference_gets_its_and_its_bases_entries_before_the_lights() {
    let mut e = setup();
    insert(&mut e, R_REFR, 1, b"NewR");
    insert(&mut e, R_STATIC, 1, b"NewB");
    let task = queued(&mut e, REFR);
    node_calls();
    crate::game_patches::gp::queued_reference(&mut e, task);
    assert_eq!(
        attached_names(&e),
        vec![(ROOT, s_of(b"NewR")), (ROOT, s_of(b"NewB"))],
        "the reference's, then the base's, under the loaded 3D"
    );
    assert_eq!(
        LIGHT_AT_ATTACH.with(|l| l.borrow().clone()),
        vec![0x20, 0x20],
        "attached before the light step"
    );
    assert!(lit(&e), "then the lights");
}

/// The player (`+0xC` 0x14) with a first-person node (`+0x694`): his own entries go under both
/// roots; entries only on his base go under the loaded 3D only (JIP's first-person call passes
/// the player alone).
#[test]
fn the_player_gets_his_entries_in_his_first_person_node_too() {
    let first = FORMS + 0xe000;
    let mut e = setup();
    insert(&mut e, R_REFR, 1, b"NewR");
    let task = queued(&mut e, REFR);
    node(&mut e, first, b"1st", &[]);
    e.mem.set_u32(REFR + 0xc, 0x14);
    e.mem.set_u32(REFR + 0x694, first);
    node_calls();
    crate::game_patches::gp::queued_reference(&mut e, task);
    assert_eq!(
        attached_names(&e),
        vec![(ROOT, s_of(b"NewR")), (first, s_of(b"NewR"))]
    );

    let mut e = setup();
    insert(&mut e, R_STATIC, 1, b"NewB");
    let task = queued(&mut e, REFR);
    node(&mut e, first, b"1st", &[]);
    e.mem.set_u32(REFR + 0xc, 0x14);
    e.mem.set_u32(REFR + 0x694, first);
    node_calls();
    crate::game_patches::gp::queued_reference(&mut e, task);
    assert_eq!(attached_names(&e), vec![(ROOT, s_of(b"NewB"))]);
}

/// No base (JIP's `GetBaseForm2` answers 0): nothing goes in, even with entries on the reference.
#[test]
fn a_loaded_reference_with_no_base_gets_nothing() {
    let mut e = setup();
    insert(&mut e, R_REFR, 1, b"NewR");
    let task = queued(&mut e, REFR);
    e.mem.set_u32(REFR + 0x20, 0);
    node_calls();
    crate::game_patches::gp::queued_reference(&mut e, task);
    assert!(attached_names(&e).is_empty());
    assert!(lit(&e), "the light step still runs");
}
