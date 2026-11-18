//! Tests of `SwapTextureEx` and `GetLightAmountAtPoint`, each called by its opcode, on scenes
//! built in guest memory (trees from `nif_nodes_tests.rs`). Expectations from JIP's asm
//! (nvse/GameObjects.cpp: `TESObjectREFR::SwapTexture`; internal/jip_core.cpp: `GetLightAmount`,
//! `GetLightAmountAtPoint`; internal/utility.cpp: `NiReplaceObject`, `NiReleaseObject`), worked
//! by hand in single precision; game functions are stand-ins that record their words.

use super::super::nif_nodes::tests::*;
use crate::commands::test_support::{call_on, calls_to, n, r, s, set_ref, z, CALLS};
use engine_core::prelude::*;

const SWAP_TEXTURE_EX: u32 = 0x278b;
const GET_LIGHT_AMOUNT_AT_POINT: u32 = 0x2920;
/// `TES::CreateTextureImage` (`RET 0x10`).
const CREATE_TEXTURE_IMAGE: u32 = 0x0045_68c0;
/// A texture class of the test's: its vtable +4 (the delete) a stand-in.
const TEX_VTBL: u32 = AREA + 0xe_0000;
const TEX_DELETE: u32 = 0x0090_0300;
/// What the texture stand-in answers through its out word (0: none).
const TEX_ANSWER: u32 = AREA + 0xe_0100;
/// `GetAsNiTriBasedGeom` (+0x1C): `00E68810` in `NiTriShape`, `00ACBB70` (0) in `NiLines`.
const NI_LINES: u32 = 0x0109_d55c;
const RETURN_THIS2_FN: u32 = 0x00e6_8810;
const RETURN_NULL_FN: u32 = 0x00ac_bb70;
/// The scene lights' lock and the game's enter and leave.
const LIGHTS_LOCK: u32 = 0x011f_9ea0;
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
/// `*0x011F91C8` (the shadow scene node), `*0x011DEA10` (`TES`).
const SHADOW_SCENE_NODE: u32 = 0x011f_91c8;
const TES_AT: u32 = 0x011d_ea10;

fn record(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn clear() {
    CALLS.with(|c| c.borrow_mut().clear());
}

fn setup() -> Engine {
    let mut e = scene_engine();
    e.map(0x011d_e000, 0x1000);
    e.map(0x011f_9000, 0x1000);
    e.mem.set_u32(NI_TRI_SHAPE + 0x1c, RETURN_THIS2_FN);
    e.mem.set_u32(NI_LINES + 0x18, RETURN_THIS2_FN);
    e.mem.set_u32(NI_LINES + 0x1c, RETURN_NULL_FN);
    e.mem.set_u32(TEX_VTBL + 4, TEX_DELETE);
    e.register(TEX_DELETE, |_, w| {
        record(TEX_DELETE, w);
        eax(0)
    });
    e.register(CREATE_TEXTURE_IMAGE, |e, w| {
        // The path's text after the words; the out word set to the answer.
        let text = nvse::table::guest_text(e, w[0]);
        let mut words = w.to_vec();
        words.extend(text.iter().map(|&b| u32::from(b)));
        record(CREATE_TEXTURE_IMAGE, &words);
        let tex = e.mem.u32(TEX_ANSWER);
        e.mem.set_u32(w[1], tex);
        eax(0)
    });
    e.register(LOCK_ENTER, |_, w| {
        record(LOCK_ENTER, w);
        eax(0)
    });
    e.register(LOCK_LEAVE, |_, w| {
        record(LOCK_LEAVE, w);
        eax(0)
    });
    e
}

// ---- SwapTextureEx ----

/// A texture object (count `count`).
fn texture(e: &mut Engine, count: u32) -> u32 {
    let t = object(e, TEX_VTBL, None);
    e.mem.set_u32(t + 4, count);
    t
}

/// A shader property of type `kind` set on `block` (+0xA8).
fn property(e: &mut Engine, block: u32, kind: u32) -> u32 {
    let p = object(e, 0, None);
    e.mem.set_u32(p + 0x1c, kind);
    e.mem.set_u32(block + 0xa8, p);
    p
}

struct Swap {
    e: Engine,
    refr: u32,
    shape: u32,
    lines: u32,
    root: u32,
}

fn swap_scene() -> Swap {
    let mut e = setup();
    let shape = shape(&mut e, b"Body");
    let lines = object(&mut e, NI_LINES, Some(b"Lines"));
    let root = node(&mut e, b"Root", &[shape, lines]);
    let refr = reference(&mut e, 0x1234, false, root);
    set_ref(&mut e, 1, refr);
    Swap {
        e,
        refr,
        shape,
        lines,
        root,
    }
}

fn swap(e: &mut Engine, block: &[u8], file: &[u8], index: Option<i32>) {
    let mut args = vec![r(1), s(block), s(file)];
    if let Some(i) = index {
        args.push(n(i));
    }
    call_on(e, SWAP_TEXTURE_EX, 0, &args);
}

/// The texture image calls' paths.
fn loaded_paths() -> Vec<Vec<u8>> {
    calls_to(CREATE_TEXTURE_IMAGE)
        .into_iter()
        .map(|w| w[4..].iter().map(|&b| b as u8).collect())
        .collect()
}

#[test]
fn swap_texture_ex_replaces_a_lighting_propertys_list_entry() {
    let mut sc = swap_scene();
    let e = &mut sc.e;
    let p = property(e, sc.shape, 8);
    // Texture lists: kind 0 and kind 1, each first entry an old texture.
    let old0 = texture(e, 1);
    let old1 = texture(e, 1);
    let list0 = object(e, 0, None);
    let list1 = object(e, 0, None);
    e.mem.set_u32(list0, old0);
    e.mem.set_u32(list1, old1);
    e.mem.set_u32(p + 0xac, list0);
    e.mem.set_u32(p + 0xb0, list1);
    let new = texture(e, 1);
    e.mem.set_u32(TEX_ANSWER, new);
    swap(e, b"Body", b"armor\\new", Some(1));
    assert_eq!(loaded_paths(), vec![b"Textures\\armor\\new.dds".to_vec()]);
    let call = &calls_to(CREATE_TEXTURE_IMAGE)[0];
    assert_eq!(call[2..4], [0, 0], "(path, &texture, 0, 0)");
    assert_eq!(e.mem.u32(list1), new, "kind 1's first entry");
    assert_eq!(e.mem.u32(list0), old0, "kind 0 untouched");
    assert_eq!(
        e.mem.u32(new + 4),
        1,
        "counted up by the slot, down by the release"
    );
    assert_eq!(e.mem.u32(old1 + 4), 0);
    assert_eq!(
        calls_to(TEX_DELETE),
        vec![vec![old1]],
        "the old texture freed"
    );
    // No index: kind 0.
    clear();
    let newer = texture(e, 1);
    e.mem.set_u32(TEX_ANSWER, newer);
    swap(e, b"Body", b"x", None);
    assert_eq!(e.mem.u32(list0), newer);
    assert_eq!(calls_to(TEX_DELETE), vec![vec![old0]]);
}

#[test]
fn swap_texture_ex_on_a_no_lighting_property_clears_its_word_and_sets_its_texture() {
    let mut sc = swap_scene();
    let e = &mut sc.e;
    let p = property(e, sc.shape, 21);
    e.mem.set_u32(p + 0x38, 0xdead_beef);
    let new = texture(e, 1);
    e.mem.set_u32(TEX_ANSWER, new);
    swap(e, b"Body", b"a", Some(3));
    assert_eq!(e.mem.u32(p + 0x38), 0);
    assert_eq!(e.mem.u32(p + 0x60), new);
    assert_eq!(e.mem.u32(new + 4), 1);
    assert!(
        calls_to(TEX_DELETE).is_empty(),
        "an empty slot frees nothing"
    );
    // The type taken modulo 32 for the test (40 = 8 + 32 passes), its low byte for the kind
    // (40 is not 8: the no-lighting path).
    clear();
    let q = property(e, sc.shape, 40);
    e.mem.set_u32(q + 0x38, 7);
    let other = texture(e, 1);
    e.mem.set_u32(TEX_ANSWER, other);
    swap(e, b"Body", b"a", None);
    assert_eq!(e.mem.u32(q + 0x60), other);
    assert_eq!(e.mem.u32(q + 0x38), 0);
}

#[test]
fn swap_texture_ex_the_same_texture_again_only_drops_the_loads_count() {
    let mut sc = swap_scene();
    let e = &mut sc.e;
    let p = property(e, sc.shape, 21);
    let held = texture(e, 2);
    e.mem.set_u32(p + 0x60, held);
    e.mem.set_u32(TEX_ANSWER, held);
    swap(e, b"Body", b"a", None);
    assert_eq!(e.mem.u32(p + 0x60), held);
    assert_eq!(e.mem.u32(held + 4), 1);
    assert!(calls_to(TEX_DELETE).is_empty());
    // A load whose count falls to 0 at the release is freed.
    clear();
    let lone = texture(e, 0);
    e.mem.set_u32(TEX_ANSWER, lone);
    e.mem.set_u32(p + 0x60, lone);
    swap(e, b"Body", b"a", None);
    assert_eq!(calls_to(TEX_DELETE), vec![vec![lone]]);
}

#[test]
fn swap_texture_ex_skips_what_jip_skips() {
    let mut sc = swap_scene();
    let e = &mut sc.e;
    let new = texture(e, 1);
    e.mem.set_u32(TEX_ANSWER, new);
    // Not tri-based geometry (NiLines), a node, an empty or missing name: nothing loaded.
    property(e, sc.lines, 21);
    property(e, sc.root, 21);
    swap(e, b"Lines", b"a", None);
    swap(e, b"Root", b"a", None);
    swap(e, b"", b"a", None);
    swap(e, b"Missing", b"a", None);
    // No property; a type whose bit is clear (9, 20, 22).
    swap(e, b"Body", b"a", None);
    for kind in [9, 20, 22] {
        property(e, sc.shape, kind);
        swap(e, b"Body", b"a", None);
    }
    assert!(calls_to(CREATE_TEXTURE_IMAGE).is_empty());
    // No texture made: nothing written.
    let p = property(e, sc.shape, 21);
    e.mem.set_u32(p + 0x38, 5);
    e.mem.set_u32(TEX_ANSWER, 0);
    swap(e, b"Body", b"a", None);
    assert_eq!(calls_to(CREATE_TEXTURE_IMAGE).len(), 1);
    assert_eq!(e.mem.u32(p + 0x38), 5);
    assert_eq!(e.mem.u32(p + 0x60), 0);
    // A lighting property whose list word is null: the load released, nothing replaced.
    clear();
    let q = property(e, sc.shape, 8);
    let lone = texture(e, 1);
    e.mem.set_u32(TEX_ANSWER, lone);
    swap(e, b"Body", b"a", Some(2));
    assert_eq!(e.mem.u32(q + 0xb4), 0);
    assert_eq!(calls_to(TEX_DELETE), vec![vec![lone]]);
    let _ = sc.refr;
}

#[test]
fn swap_texture_ex_uses_the_seen_root() {
    let mut e = setup();
    let third = shape(&mut e, b"Body");
    let first = shape(&mut e, b"Body");
    let root = node(&mut e, b"Root", &[third]);
    let first_root = node(&mut e, b"First", &[first]);
    let p = player(&mut e, root, first_root, false);
    set_ref(&mut e, 1, p);
    let p1 = property(&mut e, first, 21);
    let p3 = property(&mut e, third, 21);
    let new = texture(&mut e, 1);
    e.mem.set_u32(TEX_ANSWER, new);
    swap(&mut e, b"Body", b"a", None);
    assert_eq!(e.mem.u32(p1 + 0x60), new, "first person: its own node");
    assert_eq!(e.mem.u32(p3 + 0x60), 0);
}

// ---- GetLightAmountAtPoint ----

/// A light (`NiLight`, 0x100 bytes) and its `ShadowSceneLight` (0x100 bytes).
fn light(
    e: &mut Engine,
    point: bool,
    pos: [f32; 3],
    radius: f32,
    diffuse: [f32; 3],
    dimmer: f32,
) -> u32 {
    let l = object(e, 0, None);
    for (i, v) in pos.into_iter().enumerate() {
        e.mem.set_f32(l + 0x8c + 4 * i as u32, v);
    }
    e.mem.set_f32(l + 0xe0, radius);
    for (i, v) in diffuse.into_iter().enumerate() {
        e.mem.set_f32(l + 0xd4 + 4 * i as u32, v);
    }
    e.mem.set_f32(l + 0xc4, dimmer);
    let d = object(e, 0, None);
    e.mem.set_u8(d + 0xf4, u8::from(point));
    e.mem.set_u32(d + 0xf8, l);
    d
}

/// The scene node with `sun` and the lights list (`+0xB4`) holding `lights` (0 an empty item).
fn scene(e: &mut Engine, sun: u32, lights: &[u32]) -> u32 {
    let ssn = object(e, 0, None);
    e.mem.set_u32(ssn + 0xe0, sun);
    let mut next = 0;
    for &l in lights.iter().rev() {
        let item = object(e, 0, None);
        e.mem.set_u32(item, next);
        e.mem.set_u32(item + 8, l);
        next = item;
    }
    e.mem.set_u32(ssn + 0xb4, next);
    e.mem.set_u32(SHADOW_SCENE_NODE, ssn);
    ssn
}

/// `TES` with the interior word and a 5x5 grid centred on (0, 0) holding `cell` at the centre.
fn tes(e: &mut Engine, interior: u32, cell: u32) -> u32 {
    let t = object(e, 0, None);
    e.mem.set_u32(t + 0x34, interior);
    let grid = object(e, 0, None);
    let cells = object(e, 0, None);
    e.mem.set_u32(grid + 4, 0);
    e.mem.set_u32(grid + 8, 0);
    e.mem.set_u32(grid + 0xc, 5);
    e.mem.set_u32(grid + 0x10, cells);
    // Index `y' + x' + (x' << 2)` with x' = y' = 2: 12.
    e.mem.set_u32(cells + 12 * 4, cell);
    e.mem.set_u32(t + 8, grid);
    e.mem.set_u32(TES_AT, t);
    t
}

fn amount(e: &mut Engine, p: [f64; 3]) -> f64 {
    call_on(
        e,
        GET_LIGHT_AMOUNT_AT_POINT,
        0,
        &[z(p[0]), z(p[1]), z(p[2])],
    )
    .1
}

#[test]
fn light_amount_in_an_interior_adds_the_suns_ambient_and_the_point_lights() {
    let mut e = setup();
    tes(&mut e, 1, 0);
    let sun = light(&mut e, false, [0.0; 3], 0.0, [0.2, 0.05, 0.1], 9.0);
    let sun_light = e.mem.u32(sun + 0xf8);
    for (i, v) in [0.1f32, 0.3, 0.2].into_iter().enumerate() {
        e.mem.set_f32(sun_light + 0xc8 + 4 * i as u32, v);
    }
    // A point light 5 away with radius 10: d 0.5, factor 0.75; largest diffuse 0.4, dimmer 0.5.
    let near = light(&mut e, true, [3.0, 4.0, 0.0], 10.0, [0.1, 0.4, 0.3], 0.5);
    // One 10 away (d 1.0: none), one casting shadows, one culled: none.
    let far = light(&mut e, true, [0.0, 0.0, 10.0], 10.0, [1.0; 3], 1.0);
    let shadowed = light(&mut e, false, [0.0; 3], 0.0, [1.0; 3], 1.0);
    e.mem.set_u8(shadowed + 0xec, 1);
    let culled = light(&mut e, false, [0.0; 3], 0.0, [1.0; 3], 1.0);
    let culled_light = e.mem.u32(culled + 0xf8);
    e.mem.set_u8(culled_light + 0x30, 1);
    scene(&mut e, sun, &[0, near, far, shadowed, culled]);
    // (0.2 + 0.3) + 0.4 * 0.5 * (1 - 0.5 * 0.5), each step a float; times 100.
    let sum = (0.2f32 + 0.3f32) + 0.4f32 * 0.5f32 * (1.0f32 - 0.5f32 * 0.5f32);
    assert_eq!(amount(&mut e, [0.0; 3]), f64::from(sum * 100.0));
    assert_eq!(calls_to(LOCK_ENTER), vec![vec![LIGHTS_LOCK, 0]]);
    assert_eq!(calls_to(LOCK_LEAVE), vec![vec![LIGHTS_LOCK]]);
    // A culled sun light is still the interior's ambient and diffuse.
    clear();
    e.mem.set_u8(sun_light + 0x30, 1);
    assert_eq!(amount(&mut e, [0.0; 3]), f64::from(sum * 100.0));
}

#[test]
fn light_amount_stops_adding_at_one_and_keeps_to_0_to_100() {
    let mut e = setup();
    tes(&mut e, 1, 0);
    let sun = light(&mut e, false, [0.0; 3], 0.0, [0.5, 0.0, 0.0], 1.0);
    let bright = light(&mut e, false, [0.0; 3], 0.0, [0.6, 0.0, 0.0], 1.0);
    // Past 1.0 the walk stops: this light would take the sum below 0.
    let negative = light(&mut e, false, [0.0; 3], 0.0, [1.0, 0.0, 0.0], -10.0);
    scene(&mut e, sun, &[bright, negative]);
    assert_eq!(amount(&mut e, [0.0; 3]), 100.0);
    // Without the bright one the negative light is added: kept at 0.
    scene(&mut e, sun, &[negative]);
    assert_eq!(amount(&mut e, [0.0; 3]), 0.0);
    // A NaN sum walks on and ends at 0 (`maxss` with 0).
    let sun_light = e.mem.u32(sun + 0xf8);
    // The last channel NaN: `maxss` keeps it (a NaN first channel would give way to the next).
    e.mem.set_f32(sun_light + 0xdc, f32::NAN);
    scene(&mut e, sun, &[bright, bright]);
    assert_eq!(amount(&mut e, [0.0; 3]), 0.0);
}

#[test]
fn light_amount_in_an_exterior_needs_a_loaded_cell_and_takes_the_suns_light() {
    let mut e = setup();
    let cell = object(&mut e, 0, None);
    tes(&mut e, 0, cell);
    // A directional sun: its largest diffuse times its dimmer.
    let sun = light(&mut e, false, [0.0; 3], 0.0, [0.25, 0.5, 0.125], 0.5);
    scene(&mut e, sun, &[]);
    assert_eq!(
        amount(&mut e, [100.0, 100.0, 0.0]),
        f64::from(0.5f32 * 0.5 * 100.0)
    );
    // Outside the grid (x' = 2 + (20000 >> 12) = 6, not below 5): 0, the lock not taken.
    clear();
    assert_eq!(amount(&mut e, [20000.0, 0.0, 0.0]), 0.0);
    assert!(calls_to(LOCK_ENTER).is_empty());
    // A sun casting shadows gives nothing in an exterior.
    e.mem.set_u8(sun + 0xec, 1);
    assert_eq!(amount(&mut e, [0.0; 3]), 0.0);
}

#[test]
fn light_amount_without_a_sun_is_0_and_takes_no_lock() {
    let mut e = setup();
    tes(&mut e, 1, 0);
    e.mem.set_u32(SHADOW_SCENE_NODE, 0);
    assert_eq!(amount(&mut e, [0.0; 3]), 0.0);
    scene(&mut e, 0, &[]);
    assert_eq!(amount(&mut e, [0.0; 3]), 0.0);
    assert!(calls_to(LOCK_ENTER).is_empty());
}
