//! `ProjectUITile` on a reference built by `nif_nodes_tests.rs`, with the drawing's stand-ins
//! (`rendered_textures_tests.rs`). Expectations from JIP's `Cmd_ProjectUITile_Execute` and
//! `BSTextureManager::GenerateRenderedUITexture` (internal/netimmerse.cpp).

use super::super::nif_nodes::tests::*;
use super::super::rendered_textures::tests::*;
use super::*;
use crate::commands::test_support::{call_on, calls, calls_to, s, z, CALLS};

/// The cached screen ratio and `iSize H`'s value (`jip_fn_ui_cursor.rs`).
const SCREEN_RATIO: u32 = 0x011d_8a48;
const SIZE_H: u32 = 0x011c_718c + 4;

fn run(e: &mut Engine, this: u32, args: &[Vec<u8>]) -> f64 {
    call_on(e, 0x2925, this, args).1
}

/// A reference whose `Screen` block has a lighting shader with its texture slot; the interface's
/// root node (hidden bit set) at the manager's +0x80; a 720 line screen at ratio 1. The
/// reference, the slot and the root.
fn ready(e: &mut Engine) -> (u32, u32, u32) {
    drawing(e);
    let screen = shape(e, b"Screen");
    let root = node(e, b"Root", &[screen]);
    let refr = reference(e, 0x1234, false, root);
    let slot = object(e, NI_NODE, None);
    shaded(e, screen, 8, slot);
    let ui_root = object(e, NI_NODE, None);
    e.mem.set_u8(ui_root + 0x30, 0x31);
    e.mem.set_u32(IM_AT + 0x80, ui_root);
    e.mem.set_f32(SCREEN_RATIO, 1.0);
    e.mem.set_u32(SIZE_H, 720);
    for i in 0..4 {
        e.mem.set_f32(UI_CAMERA_AT + 0xdc + 4 * i, 7.0);
    }
    CALLS.with(|c| c.borrow_mut().clear());
    (refr, slot, ui_root)
}

fn f(v: f32) -> u32 {
    v.to_bits()
}

/// The whole interface (an empty path) drawn into the block's texture: an A8R8G8B8 texture of the
/// area's rounded size, the interface's culling process made once (an accumulator of 0x348 bytes
/// with `0x64, 1, 0x2F7`, counted 2, in tile mode, the process after it at +0x280, all passing),
/// the frame begun with a back buffer clear (no scene being drawn), the passes with the mask 0, the
/// interface camera's view port the area (x, x + width, 2 * 360 - y, that less the height) and the
/// node shown; after it the node's hidden bit, the mask and the view port back, the new texture in
/// the slot. Result 1.
#[test]
fn project_ui_tile_draws_the_interface_into_the_texture() {
    let mut e = setup_engine();
    let (refr, slot, ui_root) = ready(&mut e);
    assert_eq!(
        run(
            &mut e,
            refr,
            &[s(b""), s(b"Screen"), z(10.0), z(20.0), z(300.4), z(200.5)]
        ),
        1.0
    );
    let accumulator = ALLOC_AT;
    let culling = ALLOC_AT + 0x280;
    let x = 10.0f32;
    let top = 360.0f32 + 360.0 - 20.0;
    let port = [f(x), f(300.4 + x), f(top), f(top - 200.5)];
    let mut cull = vec![UI_CAMERA_AT, ui_root, culling, 0, 0];
    cull.extend(port);
    cull.push(0x30);
    let mut draw = vec![UI_CAMERA_AT, accumulator, 0, 0];
    draw.extend(port);
    assert_eq!(
        calls(),
        vec![
            (
                CREATE_RENDERED_TEXTURE,
                vec![MANAGER_AT, 0, 300, 200, 21, 0, 0, 0, 0, 0, 0]
            ),
            (NI_ALLOC, vec![0x348]),
            (ACCUMULATOR_CTOR, vec![accumulator, 0x64, 1, 0x2f7]),
            (CULLING_PROCESS_CTOR, vec![culling, 0]),
            (BEGIN_FRAME, vec![1, GROUP]),
            (CULL_SCENE, cull),
            (DRAW_SCENE, draw),
            (END_FRAME, vec![]),
            (TEXTURE_DELETE, vec![OLD_AT]),
            (RENDERED_DELETE, vec![RENDERED_AT]),
        ]
    );
    assert_eq!(e.mem.u32(slot), MADE_AT);
    assert_eq!(e.mem.u8(ui_root + 0x30), 0x31);
    assert_eq!(e.mem.u32(RENDERER_AT + 0x5e0), MASK);
    for i in 0..4 {
        assert_eq!(e.mem.f32(UI_CAMERA_AT + 0xdc + 4 * i), 7.0);
    }
    assert_eq!(e.mem.u32(accumulator + 4), 2);
    assert_eq!(e.mem.u32(accumulator + 0x19c), 0xa);
    assert_eq!(e.mem.u32(culling + 0xc4), accumulator);
    assert_eq!(e.mem.u32(culling + 0x90), 1);
    // Again, with a scene being drawn: the culling process kept, no frame begun; a shown node
    // stays shown.
    e.mem.set_u32(RENDERER_AT + 0x200, 1);
    e.mem.set_u8(ui_root + 0x30, 0x30);
    CALLS.with(|c| c.borrow_mut().clear());
    run(
        &mut e,
        refr,
        &[s(b""), s(b"Screen"), z(0.0), z(0.0), z(64.0), z(64.0)],
    );
    assert!(calls_to(NI_ALLOC).is_empty());
    assert!(calls_to(BEGIN_FRAME).is_empty());
    assert_eq!(e.mem.u8(ui_root + 0x30), 0x30);
}

/// A tile path: that tile's node is drawn (`GetTargetComponent`; here the HUD menu's own tile),
/// not the whole interface; a tile without a node: nothing.
#[test]
fn project_ui_tile_draws_the_tiles_node() {
    let mut e = setup_engine();
    let (refr, _, _) = ready(&mut e);
    let tile = hud_tile(&mut e, 0x0106_ed44);
    let tile_node = object(&mut e, NI_NODE, None);
    e.mem.set_u32(tile + 0x2c, tile_node);
    let args = [
        s(b"HUDMainMenu"),
        s(b"Screen"),
        z(0.0),
        z(0.0),
        z(64.0),
        z(64.0),
    ];
    assert_eq!(run(&mut e, refr, &args), 1.0);
    assert_eq!(calls_to(CULL_SCENE)[0][1], tile_node);
    e.mem.set_u32(tile + 0x2c, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(run(&mut e, refr, &args), 0.0);
    assert!(calls().is_empty());
}

/// Nothing without a texture place, for a tile path that names no tile, or with no root node.
#[test]
fn project_ui_tile_needs_a_texture_and_a_node() {
    let mut e = setup_engine();
    let (refr, _, _) = ready(&mut e);
    let area = [z(0.0), z(0.0), z(64.0), z(64.0)];
    let with = |path: &[u8], block: &[u8]| {
        let mut args = vec![s(path), s(block)];
        args.extend(area.iter().cloned());
        args
    };
    assert_eq!(run(&mut e, refr, &with(b"", b"Root")), 0.0);
    assert_eq!(run(&mut e, refr, &with(b"", b"Nowhere")), 0.0);
    assert_eq!(run(&mut e, refr, &with(b"NoMenu/NoTile", b"Screen")), 0.0);
    e.mem.set_u32(IM_AT + 0x80, 0);
    assert_eq!(run(&mut e, refr, &with(b"", b"Screen")), 0.0);
    assert!(calls().is_empty());
}

fn setup_engine() -> Engine {
    scene_engine()
}
