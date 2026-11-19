//! `AttachExtraCamera`, `ProjectExtraCamera` and the cameras' pre-load step on trees built by
//! `nif_nodes_tests.rs`; every expectation from JIP's bodies (functions_jip/jip_fn_misc_ref.h,
//! internal/netimmerse.cpp, internal/Ni_types.cpp, nvse/GameData.cpp, internal/serialization.h).
//! The game functions are stand-ins at their exe addresses (or, for a vtable slot the test writes,
//! an address of its own), reached through the real vtable slots, and record the words they get.

use super::super::nif_nodes::tests::*;
use super::super::rendered_textures::tests::*;
use super::*;
use crate::commands::test_support::{call_on, calls, calls_to, n, s, z, CALLS};

/// `NiNode::AddObject` (vtable +0xDC; a stand-in address of the test's), `RemoveObject` (+0xE8:
/// `00A5D8F0`), the camera's deleting destructor (`NiCamera` vtable +0; the test's), the game's
/// fixed-string maker `00A5B690`.
const ADD_OBJECT: u32 = 0x00de_00dc;
const REMOVE_OBJECT: u32 = 0x00a5_d8f0;
const CAMERA_DELETE: u32 = 0x00de_0000;
const ADD_STRING: u32 = 0x00a5_b690;

fn setup() -> Engine {
    let mut e = scene_engine();
    e.mem.set_u32(NI_NODE + ADD_OBJECT_SLOT, ADD_OBJECT);
    e.mem.set_u32(NI_NODE + 0xe8, REMOVE_OBJECT);
    e.mem.set_u32(NI_CAMERA, CAMERA_DELETE);
    e.register(ADD_OBJECT, |e, w| {
        record(ADD_OBJECT, w);
        e.mem.set_u32(w[1] + 0x18, w[0]);
        Ret::default()
    });
    e.register(REMOVE_OBJECT, |e, w| {
        record(REMOVE_OBJECT, w);
        e.mem.set_u32(w[1] + 0x18, 0);
        Ret::default()
    });
    e.register(CAMERA_DELETE, |_, w| {
        record(CAMERA_DELETE, w);
        Ret::default()
    });
    e.register(CAMERA_CREATE, |e, w| {
        record(CAMERA_CREATE, w);
        // A camera is 0x114 bytes (its level of detail at +0x110): a zeroed block of 0x200.
        let camera = fixed(e, &[0u8; 0x200]);
        e.mem.set_u32(camera, NI_CAMERA);
        Ret {
            eax: camera,
            ..Ret::default()
        }
    });
    // A fixed string of the text with room for its count before it.
    e.register(ADD_STRING, |e, w| {
        let text: Vec<u8> = (0..0x40)
            .map(|i| e.mem.u8(w[0] + i))
            .take_while(|&b| b != 0)
            .collect();
        let mut bytes = vec![0u8; 8];
        bytes.extend_from_slice(&text);
        let at = fixed(e, &bytes) + 8;
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    call_on(e, opcode, this, args).1
}

/// A reference with the tree `Root { Body { Head (shape) } }`: the reference, Root, Body, Head.
fn scene(e: &mut Engine) -> (u32, u32, u32, u32) {
    let head = shape(e, b"Head");
    let body = node(e, b"Body", &[head]);
    let root = node(e, b"Root", &[body]);
    (reference(e, 0x1234, false, root), root, body, head)
}

fn name_of(e: &Engine, object: u32) -> Vec<u8> {
    let at = e.mem.u32(object + NAME);
    (0..0x40)
        .map(|i| e.mem.u8(at + i))
        .take_while(|&b| b != 0)
        .collect()
}

// ---- AttachExtraCamera 0x28FC ----

/// The first attach makes the camera (counted 2, named, JIP's frustum and level of detail) and
/// puts it under the node; the name is case-blind; the same node again changes nothing; another
/// node moves it there.
#[test]
fn attach_makes_the_camera_once_and_moves_it() {
    let mut e = setup();
    let (refr, root, body, _) = scene(&mut e);
    assert_eq!(
        run(&mut e, 0x28fc, refr, &[s(b"MyCam"), n(1), s(b"Body")]),
        1.0
    );
    let list = extra_cameras(&mut e);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, b"mycam".to_vec());
    let camera = list[0].1;
    assert_eq!(calls_to(CAMERA_CREATE), vec![Vec::<u32>::new()]);
    assert_eq!(e.mem.u32(camera + 4), 2);
    assert_eq!(name_of(&e, camera), b"MyCam".to_vec());
    assert_eq!(e.mem.f32(camera + 0x64), 0.0);
    assert_eq!(e.mem.f32(camera + 0xec), 5.0);
    assert_eq!(e.mem.f32(camera + 0xf0), 353_840.0);
    assert_eq!(e.mem.f32(camera + 0xf8), 1.0);
    assert_eq!(e.mem.f32(camera + 0xfc), 70_768.0);
    assert_eq!(e.mem.f32(camera + 0x110), 1.0);
    assert_eq!(calls_to(ADD_OBJECT), vec![vec![body, camera, 1]]);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(
        run(&mut e, 0x28fc, refr, &[s(b"MYCAM"), n(1), s(b"Body")]),
        1.0
    );
    assert!(calls().is_empty(), "made once, already there");
    assert_eq!(run(&mut e, 0x28fc, refr, &[s(b"mycam"), n(1)]), 1.0);
    assert_eq!(
        calls_to(ADD_OBJECT),
        vec![vec![root, camera, 1]],
        "no node name: the root"
    );
    assert_eq!(extra_cameras(&mut e).len(), 1);
}

/// Nothing for an empty name, a block that is not a node, a missing block or no 3D.
#[test]
fn attach_needs_a_name_and_a_node() {
    let mut e = setup();
    let (refr, _, _, _) = scene(&mut e);
    let bare = reference(&mut e, 0x1235, false, 0);
    for (this, args) in [
        (refr, vec![s(b""), n(1), s(b"Body")]),
        (refr, vec![s(b"Cam"), n(1), s(b"Head")]),
        (refr, vec![s(b"Cam"), n(1), s(b"Nowhere")]),
        (bare, vec![s(b"Cam"), n(1)]),
        (refr, vec![s(b""), n(0)]),
    ] {
        assert_eq!(run(&mut e, 0x28fc, this, &args), 0.0);
    }
    assert!(calls().is_empty());
    assert!(extra_cameras(&mut e).is_empty());
}

/// Detaching: off its node, destroyed (the deleting destructor with 1) and forgotten; a name not
/// in the map: nothing.
#[test]
fn detach_destroys_the_camera() {
    let mut e = setup();
    let (refr, _, body, _) = scene(&mut e);
    run(&mut e, 0x28fc, refr, &[s(b"Cam"), n(1), s(b"Body")]);
    let camera = extra_cameras(&mut e)[0].1;
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(run(&mut e, 0x28fc, refr, &[s(b"CAM"), n(0)]), 1.0);
    assert_eq!(
        calls(),
        vec![
            (REMOVE_OBJECT, vec![body, camera]),
            (CAMERA_DELETE, vec![camera, 1])
        ]
    );
    assert!(extra_cameras(&mut e).is_empty());
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(run(&mut e, 0x28fc, refr, &[s(b"Cam"), n(0)]), 0.0);
    assert!(calls().is_empty());
}

/// Before a game loads: every camera off its node and destroyed, in the map's order, the map
/// emptied.
#[test]
fn pre_load_destroys_every_camera() {
    let mut e = setup();
    let (refr, root, body, _) = scene(&mut e);
    run(&mut e, 0x28fc, refr, &[s(b"One"), n(1), s(b"Body")]);
    run(&mut e, 0x28fc, refr, &[s(b"Two"), n(1)]);
    let list = extra_cameras(&mut e);
    let (one, two) = (list[0].1, list[1].1);
    CALLS.with(|c| c.borrow_mut().clear());
    pre_load(&mut e);
    assert_eq!(
        calls(),
        vec![
            (REMOVE_OBJECT, vec![body, one]),
            (CAMERA_DELETE, vec![one, 1]),
            (REMOVE_OBJECT, vec![root, two]),
            (CAMERA_DELETE, vec![two, 1]),
        ]
    );
    assert!(extra_cameras(&mut e).is_empty());
}

/// The step runs in JIP's housekeeping before a game loads and at the exit to the main menu.
#[test]
fn the_housekeeping_messages_destroy_the_cameras() {
    for kind in [
        nvse::plugins::msg::PRE_LOAD_GAME,
        nvse::plugins::msg::EXIT_TO_MAIN_MENU,
    ] {
        let mut e = setup();
        let (refr, _, body, _) = scene(&mut e);
        // The other housekeeping steps read the player (`*011DEA3C`): this reference stands in.
        if !e.mem.is_mapped(0x011d_e000) {
            e.map(0x011d_e000, 0x1000);
        }
        e.mem.set_u32(0x011d_ea3c, refr);
        run(&mut e, 0x28fc, refr, &[s(b"Cam"), n(1), s(b"Body")]);
        let camera = extra_cameras(&mut e)[0].1;
        CALLS.with(|c| c.borrow_mut().clear());
        nvse::plugins::dispatch(&mut e, kind, 0, 0);
        assert_eq!(calls_to(REMOVE_OBJECT), vec![vec![body, camera]], "{kind}");
        assert_eq!(calls_to(CAMERA_DELETE), vec![vec![camera, 1]], "{kind}");
        assert!(extra_cameras(&mut e).is_empty(), "{kind}");
    }
}

// ---- ProjectExtraCamera 0x28FD ----

/// A camera named `Cam` on Body, the head lit with its texture slot; the camera and the slot.
fn ready(e: &mut Engine) -> (u32, u32, u32) {
    drawing(e);
    let (refr, _, _, head) = scene(e);
    run(e, 0x28fc, refr, &[s(b"Cam"), n(1), s(b"Body")]);
    let slot = object(e, NI_NODE, None);
    shaded(e, head, 8, slot);
    CALLS.with(|c| c.borrow_mut().clear());
    (refr, extra_cameras(e)[0].1, slot)
}

/// The draw: a 0x100 texture in X8R8G8B8, the world culling process made once (0x348 bytes, the
/// accumulator at +0xC8 with `0x63, 1, 0x2F7` on the shadow scene), the frame with a full clear,
/// the passes with the scene's byte 1 and the mask kept (a background mask of all ones), the new
/// texture in the slot (counted up, the old one let go), the rendered texture let go. The field of
/// view: the scale keeps it, the view port is `(-v, v, v, -v)` with `v = tan(fov) * 2/3`, the level
/// of detail `fov * 0.08`. Result 1.
#[test]
fn project_draws_into_the_blocks_texture() {
    let mut e = setup();
    let (refr, camera, slot) = ready(&mut e);
    assert_eq!(
        run(&mut e, 0x28fd, refr, &[s(b"Cam"), s(b"Head"), z(75.0)]),
        1.0
    );
    let culling = ALLOC_AT;
    let accumulator = ALLOC_AT + 0xc8;
    let manager_words = vec![MANAGER_AT, 0, 0x100, 0x100, 22, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        calls(),
        vec![
            (CREATE_RENDERED_TEXTURE, manager_words),
            (NI_ALLOC, vec![0x348]),
            (CULLING_PROCESS_CTOR, vec![culling, 0]),
            (ACCUMULATOR_CTOR, vec![accumulator, 0x63, 1, 0x2f7]),
            (BEGIN_FRAME, vec![7, GROUP]),
            (CULL_SCENE, vec![camera, SCENE_AT, culling, 1, MASK]),
            (DRAW_SCENE, vec![camera, accumulator, 1, MASK]),
            (END_FRAME, vec![]),
            (TEXTURE_DELETE, vec![OLD_AT]),
            (RENDERED_DELETE, vec![RENDERED_AT]),
        ]
    );
    assert_eq!(e.mem.u32(slot), MADE_AT);
    assert_eq!(e.mem.u32(MADE_AT + 4), 1);
    assert_eq!(e.mem.u32(RENDERED_AT + 4), 0, "counted up, then let go");
    assert_eq!(e.mem.u8(SCENE_AT + 0x130), 0);
    assert_eq!(e.mem.u32(RENDERER_AT + 0x5e0), MASK);
    assert_eq!(e.mem.u32(accumulator + 4), 2);
    assert_eq!(e.mem.u32(accumulator + 0x194), SCENE_AT);
    assert_eq!(e.mem.u32(accumulator + 0x19c), 0);
    assert_eq!(e.mem.u8(accumulator + 0x16c), 1);
    assert_eq!(e.mem.u32(culling + 0xc4), accumulator);
    assert_eq!(e.mem.u32(culling + 0x90), 3);
    assert_eq!(e.mem.f32(camera + 0x64), 75.0);
    let v = crate::commands::ln_fn_math::tan(75.0 * crate::commands::FLT_PI_D_180)
        * f32::from_bits(0x3f2a_aaab);
    let port: Vec<f32> = (0..4).map(|i| e.mem.f32(camera + 0xdc + 4 * i)).collect();
    assert_eq!(port, vec![-v, v, v, -v]);
    assert_eq!(
        e.mem.f32(camera + 0x110),
        75.0 * f32::from_bits(0x3da3_d70a)
    );
    // The same field of view again: the view port left as it is; the culling process kept.
    e.mem.set_f32(camera + 0xdc, 9.0);
    CALLS.with(|c| c.borrow_mut().clear());
    run(
        &mut e,
        0x28fd,
        refr,
        &[s(b"Cam"), s(b"Head"), z(75.0), n(0x123)],
    );
    assert_eq!(e.mem.f32(camera + 0xdc), 9.0);
    assert!(calls_to(NI_ALLOC).is_empty());
    assert_eq!(
        calls_to(CREATE_RENDERED_TEXTURE)[0][2..5],
        [0x120, 0x120, 22],
        "the size's low bits off"
    );
}

/// HDR on: A16B16G16R16F for a model's texture.
#[test]
fn project_draws_in_hdr_when_the_game_does() {
    let mut e = setup();
    let (refr, _, _) = ready(&mut e);
    e.mem.set_u8(HDR, 1);
    run(&mut e, 0x28fd, refr, &[s(b"Cam"), s(b"Head"), z(60.0)]);
    assert_eq!(calls_to(CREATE_RENDERED_TEXTURE)[0][4], 113);
}

/// An interface image (`*` and its path, here the HUD menu's own tile): its shader's texture takes
/// the draw, in X8R8G8B8 whatever the game's HDR, or L8 (grey) with flag 1. A tile that is not an
/// image, or an image without a shader: nothing.
#[test]
fn project_draws_into_an_interface_image() {
    let mut e = setup();
    let (refr, _, _) = ready(&mut e);
    e.mem.set_u8(HDR, 1);
    let image = hud_tile(&mut e, 0x0106_f01c);
    let shader = object(&mut e, NI_NODE, None);
    e.mem.set_u32(image + 0x40, shader);
    e.mem.set_u32(shader + 0x60, OLD_AT);
    let path = s(b"*HUDMainMenu");
    assert_eq!(
        run(&mut e, 0x28fd, refr, &[s(b"Cam"), path.clone(), z(60.0)]),
        1.0
    );
    assert_eq!(calls_to(CREATE_RENDERED_TEXTURE)[0][4], 22);
    assert_eq!(e.mem.u32(shader + 0x60), MADE_AT);
    CALLS.with(|c| c.borrow_mut().clear());
    let grey = [s(b"Cam"), path.clone(), z(60.0), n(0x100), n(1)];
    assert_eq!(run(&mut e, 0x28fd, refr, &grey), 1.0);
    assert_eq!(calls_to(CREATE_RENDERED_TEXTURE)[0][4], 50);
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(image, 0x0106_ed44);
    assert_eq!(run(&mut e, 0x28fd, refr, &grey), 0.0, "a menu tile");
    e.mem.set_u32(image, 0x0106_f01c);
    e.mem.set_u32(image + 0x40, 0);
    assert_eq!(run(&mut e, 0x28fd, refr, &grey), 0.0, "no shader");
    assert!(calls().is_empty());
}

/// Nothing for a camera not made, a camera off its node, a size below 0x10, a block without a
/// texture place or a tile path with no image.
#[test]
fn project_needs_a_camera_on_a_node_and_a_texture() {
    let mut e = setup();
    let (refr, camera, _) = ready(&mut e);
    for args in [
        vec![s(b"Other"), s(b"Head"), z(60.0)],
        vec![s(b"Cam"), s(b"Head"), z(60.0), n(0xf)],
        vec![s(b"Cam"), s(b"Body"), z(60.0)],
        vec![s(b"Cam"), s(b"Nowhere"), z(60.0)],
        vec![s(b"Cam"), s(b"*NoMenu/NoTile"), z(60.0)],
    ] {
        assert_eq!(run(&mut e, 0x28fd, refr, &args), 0.0);
    }
    e.mem.set_u32(camera + 0x18, 0);
    assert_eq!(
        run(&mut e, 0x28fd, refr, &[s(b"Cam"), s(b"Head"), z(60.0)]),
        0.0
    );
    assert!(calls().is_empty());
}

/// Before the draw: every high actor's root loses its "culled" bit, and with water the update
/// runs for the camera with the two settings words 0 and the byte 1, all put back after.
#[test]
fn project_shows_high_actors_and_updates_water() {
    let mut e = setup();
    let (refr, camera, _) = ready(&mut e);
    let actor_root = object(&mut e, NI_NODE, None);
    e.mem.set_u32(actor_root + 0x30, 0x0010_0003);
    let actor = reference(&mut e, 0x2000, true, actor_root);
    let next = object(&mut e, NI_NODE, None);
    e.mem.set_u32(HIGH_ACTOR_LIST, actor);
    e.mem.set_u32(HIGH_ACTOR_LIST + 4, next);
    e.mem.set_u32(next, 0);
    e.mem.set_u32(next + 4, 0);
    e.mem.set_u32(TES_WORD, TES_AT);
    e.mem.set_u32(TES_AT + 0x64, WATER_AT);
    e.mem.set_u32(WATER_AT + 0x3c, 1);
    e.mem.set_u32(WATER_WORDS, 0x11);
    e.mem.set_u32(WATER_WORDS + 0x50, 0x22);
    run(&mut e, 0x28fd, refr, &[s(b"Cam"), s(b"Head"), z(60.0)]);
    assert_eq!(e.mem.u32(actor_root + 0x30), 3);
    assert_eq!(
        calls_to(WATER_UPDATE),
        vec![vec![WATER_AT, camera, 0, 1, 0, 0]]
    );
    assert_eq!(e.mem.u8(EXTRA_CAMERA), 0);
    assert_eq!(e.mem.u32(WATER_WORDS), 0x11);
    assert_eq!(e.mem.u32(WATER_WORDS + 0x50), 0x22);
    // No water in the list: no update.
    e.mem.set_u32(WATER_AT + 0x3c, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    run(&mut e, 0x28fd, refr, &[s(b"Cam"), s(b"Head"), z(60.0)]);
    assert!(calls_to(WATER_UPDATE).is_empty());
}
