//! JohnnyGuitar NVSE's world-to-screen projection (`JG/internal/JG/WorldToScreen.cpp`): at the end
//! of every `BSSceneGraph::SetCameraFOV` (`00C52020`, JG's jump at `0xC5244A`) it copies the world
//! scene graph's camera (its local and world positions, its world-to-camera matrix and its
//! viewport) when the call was for that camera at the player's world field of view; `WorldToScreen`
//! and `JGLegacyWorldToScreen` project a point with the copy, `GetCameraTranslation` reads its
//! positions. The copy is the game's state: one for the run, in a lock-free slot.

use engine_core::prelude::*;
use std::sync::Mutex;

/// JG's `ZERO_TOLERANCE` (`0.0000099999997f`).
pub const ZERO_TOLERANCE: f32 = 0.000_009_999_999_7;

/// `WorldToScreen::CameraParams`: the camera's world and local positions, its world-to-camera
/// matrix (row by row) and its viewport (left, right, top, bottom).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraParams {
    pub world: [f32; 3],
    pub local: [f32; 3],
    pub world_to_cam: [[f32; 4]; 4],
    pub port: [f32; 4],
}

/// JG's `kCameraData` (zero until the first copy, as JG's static).
#[derive(Default)]
pub struct CameraData(pub Mutex<CameraParams>);

/// `NiCamera`'s fields (JohnnyGuitar's `Gamebryo/NiCamera.hpp`, `NiAVObject.hpp`): the local and
/// world transforms' translations (`m_kLocal` +0x34, `m_kWorld` +0x68, each `m_kTranslate` +0x24),
/// `m_aafWorldToCam` (+0x9C) and `m_kPort` (+0x100).
const LOCAL_TRANSLATE: u32 = 0x34 + 0x24;
const WORLD_TRANSLATE: u32 = 0x68 + 0x24;
const WORLD_TO_CAM: u32 = 0x9c;
const PORT: u32 = 0x100;

/// `TESMain::GetWorldSceneGraph` (`0045C670`), `SceneGraph::spCamera` (+0xAC, as `SetCameraFOV`
/// reads its own camera); the player (`*011DEA3C`) and its world field of view
/// (`PlayerCharacter::GetWorldFOV`, `00710AB0`, `thiscall`).
const WORLD_SCENE_GRAPH: u32 = 0x0045_c670;
const SCENE_CAMERA: u32 = 0xac;
const PLAYER: u32 = 0x011d_ea3c;
const PLAYER_WORLD_FOV: u32 = 0x0071_0ab0;

fn f32s<const N: usize>(e: &Engine, at: u32) -> [f32; N] {
    std::array::from_fn(|i| e.mem.f32(at + 4 * i as u32))
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/internal/JG/WorldToScreen.cpp:
// CopyNiCamera (NiCameraGetAltHook's call).
/// The seam at the end of `SetCameraFOV` (words: the camera it set, the field of view's bits):
/// with a world scene graph and a player, the copy of that camera when it is the scene graph's
/// and the field of view is the player's world one (within [`ZERO_TOLERANCE`]). The exe goes on.
pub fn camera_site(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (camera, fov) = (w[0], f32::from_bits(w[1]));
    let graph = e.call(WORLD_SCENE_GRAPH, &args![]).u32();
    let player = e.mem.u32(PLAYER);
    if graph == 0 || player == 0 {
        return None;
    }
    let world_fov = e.call(PLAYER_WORLD_FOV, &args![player]).f32();
    if camera != e.mem.u32(graph + SCENE_CAMERA) || (fov - world_fov).abs() > ZERO_TOLERANCE {
        return None;
    }
    let mut copy = CameraParams {
        local: f32s(e, camera + LOCAL_TRANSLATE),
        world: f32s(e, camera + WORLD_TRANSLATE),
        port: f32s(e, camera + PORT),
        ..CameraParams::default()
    };
    for (r, row) in copy.world_to_cam.iter_mut().enumerate() {
        *row = f32s(e, camera + WORLD_TO_CAM + 16 * r as u32);
    }
    *e.ext_free::<CameraData>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = copy;
    None
}

/// The copy as it is now.
pub fn camera(e: &mut Engine) -> CameraParams {
    *e.ext_free::<CameraData>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/internal/JG/WorldToScreen.cpp:
// WorldToScreenPoint3.
/// The point projected with the camera: `Some` of the screen point (x, y from 0 to 1 on the
/// viewport, z the depth over w) when it is on screen in front; otherwise `None` of the point,
/// pulled onto the screen's edge for `offscreen` 0 (a square) or 1 (a circle), as computed for 2
/// and up. `w` zero: `None` of x and y as projected and z untouched (0).
pub fn project(
    c: &CameraParams,
    p: [f32; 3],
    offscreen: i32,
    zero_tolerance: f32,
) -> (bool, [f32; 3]) {
    let m = &c.world_to_cam;
    let row = |r: usize| p[0] * m[r][0] + p[1] * m[r][1] + p[2] * m[r][2] + m[r][3];
    let fw = row(3);
    let mut out = [row(0), row(1), 0.0];
    if fw == 0.0 {
        return (false, out);
    }
    let mut inv_w = 1.0 / fw;
    out[2] = row(2) * inv_w;
    let mut behind = false;
    if !(fw > zero_tolerance) {
        behind = true;
        inv_w *= -1.0;
    }
    out[0] *= inv_w;
    out[1] *= inv_w;
    let [left, right, top, bottom] = c.port;
    out[0] *= (right - left) * 0.5;
    out[1] *= (top - bottom) * 0.5;
    out[0] += (right + left) * 0.5;
    out[1] += (top + bottom) * 0.5;
    out[1] = top - out[1];
    if out[0] >= left && out[0] <= right && out[1] >= bottom && out[1] <= top && !behind {
        return (true, out);
    }
    if offscreen < 2 {
        let (mut x2, mut y2) = (right / 2.0, top / 2.0);
        let d = ((out[0] - x2) * (out[0] - x2) + (out[1] - y2) * (out[1] - y2)).sqrt();
        let r = y2 / d;
        out[0] = r * out[0] + (1.0 - r) * x2;
        out[1] = r * out[1] + (1.0 - r) * y2;
        if offscreen == 0 {
            x2 = out[0] - 0.5;
            y2 = out[1] - 0.5;
            let divider = if y2 * y2 > 0.125 { y2.abs() } else { x2.abs() };
            out[0] = 0.5 * ((x2 / divider) + 1.0);
            out[1] = 0.5 * ((y2 / divider) + 1.0);
        }
        // C's `a > 0.f ? (a < 1.f ? a : 1.f) : 0.f` (a NaN gives 0).
        let clamp01 = |a: f32| {
            if a > 0.0 {
                if a < 1.0 {
                    a
                } else {
                    1.0
                }
            } else {
                0.0
            }
        };
        out[0] = clamp01(out[0]);
        out[1] = clamp01(out[1]);
    }
    (false, out)
}
