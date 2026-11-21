//! Tests of `fn_math`'s commands, called through their records as `EvaluateLine` calls them.

use super::*;
use crate::test_support::*;
use crate::world_to_screen;

fn result(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, r) = call(e, opcode, args);
    assert_eq!(ok, 1);
    r
}

/// Clamp, Remap, Lerp and Sign in floats, as JohnnyGuitar computes them.
#[test]
fn the_arithmetic_commands() {
    let mut e = engine(true);
    assert_eq!(result(&mut e, CLAMP, &[num(5.0), num(0.0), num(2.0)]), 2.0);
    assert_eq!(result(&mut e, CLAMP, &[num(-1.0), num(0.0), num(2.0)]), 0.0);
    assert_eq!(result(&mut e, CLAMP, &[num(1.5), num(0.0), num(2.0)]), 1.5);
    assert_eq!(
        result(&mut e, REMAP, &[num(5.0), num(0.0), num(10.0), num(100.0), num(200.0)]),
        150.0
    );
    // In floats: (1 - 0.1) * 0 + 0.1 * 1 rounds as an f32 does.
    let lerp = result(&mut e, LERP, &[num(0.0), num(1.0), num(0.1)]);
    assert_eq!(lerp, f64::from((1.0f32 - 0.1f32) * 0.0 + 0.1f32 * 1.0));
    assert_eq!(result(&mut e, SIGN, &[num(-3.0)]), -1.0);
    assert_eq!(result(&mut e, SIGN, &[num(3.0)]), 1.0);
    assert_eq!(result(&mut e, SIGN, &[num(0.0)]), 0.0);
}

/// GetRGBColor packs three components of at most 255; one above gives 0.
#[test]
fn rgb_color_packs_three_components() {
    let mut e = engine(true);
    assert_eq!(
        result(&mut e, GET_RGB_COLOR, &[int(0x12), int(0x34), int(0x56)]),
        f64::from(0x0012_3456u32)
    );
    assert_eq!(result(&mut e, GET_RGB_COLOR, &[int(256), int(0), int(0)]), 0.0);
}

/// The projection: a point straight ahead lands in the middle; one behind is pulled to the edge
/// (0 and 1 for the square), and the answer is 0.
#[test]
fn a_point_is_projected_with_the_camera_copy() {
    let mut c = world_to_screen::CameraParams::default();
    // Camera looking down +y: x to the right, z up, w = y.
    c.world_to_cam = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
    ];
    c.port = [0.0, 1.0, 1.0, 0.0];
    let (on, p) = world_to_screen::project(&c, [0.0, 10.0, 0.0], 0, world_to_screen::ZERO_TOLERANCE);
    assert!(on);
    assert_eq!((p[0], p[1]), (0.5, 0.5));
    let (on, p) = world_to_screen::project(&c, [0.0, -10.0, 1.0], 0, world_to_screen::ZERO_TOLERANCE);
    assert!(!on);
    assert!((0.0..=1.0).contains(&p[0]) && (0.0..=1.0).contains(&p[1]));
}

/// Not installed: no command has a handler (as without the DLL).
#[test]
fn not_installed_nothing_answers() {
    let e = engine(false);
    assert!(e.script_tables().max_opcode <= LERP);
}

/// The camera copy at the end of `SetCameraFOV`: taken only for the world scene graph's camera at
/// the player's world field of view; WorldToScreen then projects with it.
#[test]
fn the_camera_is_copied_for_the_world_camera_only() {
    let mut e = engine(true);
    let graph = e.mem.alloc(0x100);
    let camera = e.mem.alloc(0x120);
    e.mem.set_u32(graph + 0xac, camera);
    e.register_double(0x0045_c670, move |_, _| graph.into_ret());
    let player = e.mem.alloc(0x700);
    e.mem.set_f32(player + 0x670, 75.0);
    if !e.mem.is_mapped(0x011d_ea3c) {
        e.map(0x011d_e000, 0x1000);
    }
    e.mem.set_u32(0x011d_ea3c, player);
    e.register(0x0071_0ab0, |e, a| e.mem.f32(a[0] + 0x670).into_ret());
    // The camera: world position (1, 2, 3), a viewport of (0, 1, 1, 0).
    e.mem.set_f32(camera + 0x68 + 0x24, 1.0);
    e.mem.set_f32(camera + 0x68 + 0x28, 2.0);
    e.mem.set_f32(camera + 0x68 + 0x2c, 3.0);
    e.mem.set_f32(camera + 0x104, 1.0);
    e.mem.set_f32(camera + 0x108, 1.0);
    // Another field of view: no copy.
    e.site(0x00c5_244a, &[camera, 90.0f32.to_bits()]);
    assert_eq!(world_to_screen::camera(&mut e), world_to_screen::CameraParams::default());
    // Another camera: no copy.
    let other = e.mem.alloc(0x120);
    e.site(0x00c5_244a, &[other, 75.0f32.to_bits()]);
    assert_eq!(world_to_screen::camera(&mut e), world_to_screen::CameraParams::default());
    e.site(0x00c5_244a, &[camera, 75.0f32.to_bits()]);
    let copy = world_to_screen::camera(&mut e);
    assert_eq!(copy.world, [1.0, 2.0, 3.0]);
    assert_eq!(copy.port, [0.0, 1.0, 1.0, 0.0]);
}
