//! JIP LN NVSE's rendered textures (`internal/netimmerse.cpp`: `BSTextureManager::
//! GenerateRenderedTexture`, `GenerateRenderedUITexture`, `GetGRTCullingProcess`), its texture
//! slot lookup (`nvse/GameObjects.cpp`: `TESObjectREFR::GetTexturePtr`) and the two scene steps
//! `ProjectExtraCamera` runs first (`ProcessManager::UnCullHighActors`, `WaterManager::UpdateEx`).
//! The drawing is the game's own: a rendered texture from the texture manager (`00B6D170`), the
//! frame and its accumulate and draw passes (`00B6B8D0`, `00B6BEE0`, `00B6C0D0`, `00B6B790`), which
//! the port's renderer draws like the game's own rendered textures (water, the Pip-Boy screen).
//!
//! JIP's image space pass after the draw (`isEffect` up to `IS_SHADER_NOISE_NORMAL_MAP`) is not
//! here: every caller in JIP 57.30 passes `IS_EFFECT_NONE` (`ProjectExtraCamera`'s argument is
//! commented out in its body, the minimap passes none), so the branch never runs.
//!
//! Threads: on the script's thread. JIP's culling process for the world is per thread
//! (`thread_local`), kept here per engine; the one for the interface is one for the run, as JIP's
//! function-local static.

use super::nif_nodes::{get_block, ni_node};
use crate::game_patches::gx::{refs_add, replace_object};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// `*BSTEXTUREMANAGER` (`011F91A8`), `*NIDX9RENDERER` (`011F4748`), `g_shadowSceneNode`
/// (`*011F91C8`), the interface manager (`*011D8A80`).
const TEXTURE_MANAGER: u32 = 0x011f_91a8;
const RENDERER: u32 = 0x011f_4748;
const SHADOW_SCENE_NODE: u32 = 0x011f_91c8;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
/// The game calls: a rendered texture (`thiscall` on the manager: 0, width, height, format, six
/// 0s), the frame begun with its clear (`cdecl`: flags, the texture's render target group), the
/// accumulate pass (`cdecl`: camera, scene, culling process) and the draw pass (`cdecl`: camera,
/// accumulator; JIP leaves the culling process on the stack after them, which it does not read),
/// the frame ended; `Ni_Alloc` (`cdecl` size), the culling process (`thiscall`: 0) and the shader
/// accumulator (`thiscall`: 3 words) constructors.
const CREATE_RENDERED_TEXTURE: u32 = 0x00b6_d170;
const BEGIN_FRAME: u32 = 0x00b6_b8d0;
const CULL_SCENE: u32 = 0x00b6_bee0;
const DRAW_SCENE: u32 = 0x00b6_c0d0;
const END_FRAME: u32 = 0x00b6_b790;
const NI_ALLOC: u32 = 0x00aa_13e0;
const CULLING_PROCESS_CTOR: u32 = 0x004a_0eb0;
const ACCUMULATOR_CTOR: u32 = 0x00b6_60d0;
/// `kClrFlag_All`, `kClrFlag_BackBuffer`; `D3DFMT_A8R8G8B8`; `kRndrMode_Tiles`;
/// `kCull_IgnoreMultiBounds`, `kCull_AllPass`.
const CLEAR_ALL: u32 = 7;
const CLEAR_BACK_BUFFER: u32 = 1;
const A8R8G8B8: u32 = 21;
const RENDER_MODE_TILES: u32 = 0xa;
const CULL_IGNORE_MULTI_BOUNDS: u32 = 3;
const CULL_ALL_PASS: u32 = 1;
/// A rendered texture's render target group (+8) and its texture (+0x30); a culling process's
/// accumulator (+0xC4) and cull mode (+0x90); an accumulator's scene (+0x194), render mode
/// (+0x19C) and "background" byte (+0x16C); the scene's byte +0x130; the renderer's clear mask
/// (+0x5E0) and "scene active" word (+0x200); a camera's frustum view port (+0xDC, 16 bytes); a
/// node's flags (+0x30, bit 0 hidden).
const TEXTURE_TARGET_GROUP: u32 = 8;
const TEXTURE_NI_TEXTURE: u32 = 0x30;
const CULLING_ACCUMULATOR: u32 = 0xc4;
const CULLING_MODE: u32 = 0x90;
const ACCUMULATOR_SCENE: u32 = 0x194;
const ACCUMULATOR_RENDER_MODE: u32 = 0x19c;
const ACCUMULATOR_BACKGROUND: u32 = 0x16c;
const SCENE_RENDERING_TEXTURE: u32 = 0x130;
const RENDERER_CLEAR_MASK: u32 = 0x5e0;
const RENDERER_SCENE_ACTIVE: u32 = 0x200;
const CAMERA_VIEW_PORT: u32 = 0xdc;
const NODE_FLAGS: u32 = 0x30;

/// JIP's `TextureParams`: width, height, Direct3D format, render mode, background colour mask.
#[derive(Clone, Copy, Debug)]
pub struct TextureParams {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub render_mode: u32,
    pub bg_color_mask: u32,
}

fn counted_up(e: &mut Engine, object: u32) {
    refs_add(e, object, 1);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: NiReleaseObject.
/// `object`'s count down; at 0 or below it is deleted (its vtable +4).
pub(crate) fn release(e: &mut Engine, object: u32) {
    if refs_add(e, object, -1) <= 0 {
        let vtable = e.mem.u32(object);
        let delete = e.mem.u32(vtable.wrapping_add(4));
        e.call(delete, &[object]);
    }
}

fn new_rendered_texture(e: &mut Engine, width: u32, height: u32, format: u32) -> u32 {
    let manager = e.mem.u32(TEXTURE_MANAGER);
    let texture = e
        .call(
            CREATE_RENDERED_TEXTURE,
            &args![manager, 0u32, width, height, format, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32],
        )
        .u32();
    if texture != 0 {
        counted_up(e, texture);
    }
    texture
}

/// JIP's culling process for the world, one per thread.
#[derive(Default)]
pub struct WorldCulling(u32);

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: GetGRTCullingProcess.
/// The thread's culling process with its shader accumulator after it (0x348 bytes: the process,
/// then at +0xC8 the accumulator made with `0x63, 1, 0x2F7`, counted 2, on the shadow scene),
/// made the first time, its cull mode "ignore multi bounds".
fn world_culling(e: &mut Engine) -> u32 {
    let kept = e.ext::<WorldCulling>().0;
    if kept != 0 {
        return kept;
    }
    let block = e.call(NI_ALLOC, &args![0x348u32]).u32();
    let culling = e.call(CULLING_PROCESS_CTOR, &args![block, 0u32]).u32();
    let accumulator = e
        .call(
            ACCUMULATOR_CTOR,
            &args![culling + 0xc8, 0x63u32, 1u32, 0x2f7u32],
        )
        .u32();
    e.mem.set_u32(accumulator + 4, 2);
    let scene = e.mem.u32(SHADOW_SCENE_NODE);
    e.mem.set_u32(accumulator + ACCUMULATOR_SCENE, scene);
    e.mem.set_u32(culling + CULLING_ACCUMULATOR, accumulator);
    e.mem
        .set_u32(culling + CULLING_MODE, CULL_IGNORE_MULTI_BOUNDS);
    e.ext::<WorldCulling>().0 = culling;
    culling
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// BSTextureManager::GenerateRenderedTexture (asm).
/// The world seen from `camera` drawn into a new rendered texture of `params`, which then takes
/// the place of the texture at `slot` (JIP's `NiReplaceObject`: the new one counted up, the old one
/// let go). On the way: the scene's byte +0x130 set for the draw, the accumulator's render mode, the
/// renderer's clear mask ANDed with the background mask (the accumulator's byte +0x16C its "not
/// 0"), the frame begun with a full clear, the accumulate and draw passes, the frame ended, the
/// mask and the byte put back. Nothing (the slot as it was) when the game makes no texture.
pub fn generate_rendered_texture(e: &mut Engine, camera: u32, params: TextureParams, slot: u32) {
    let texture = new_rendered_texture(e, params.width, params.height, params.format);
    // nv-rs's own: the game made no texture (JIP goes on and calls through it): nothing drawn,
    // the slot keeps its texture.
    if texture == 0 {
        return;
    }
    let culling = world_culling(e);
    let accumulator = culling + 0xc8;
    let scene = e.mem.u32(accumulator + ACCUMULATOR_SCENE);
    e.mem.set_u8(scene + SCENE_RENDERING_TEXTURE, 1);
    e.mem
        .set_u32(accumulator + ACCUMULATOR_RENDER_MODE, params.render_mode);
    let mask_at = e.mem.u32(RENDERER) + RENDERER_CLEAR_MASK;
    let old_mask = e.mem.u32(mask_at);
    let mask = old_mask & params.bg_color_mask;
    e.mem.set_u32(mask_at, mask);
    e.mem
        .set_u8(accumulator + ACCUMULATOR_BACKGROUND, u8::from(mask != 0));
    let group = e.mem.u32(texture + TEXTURE_TARGET_GROUP);
    e.call(BEGIN_FRAME, &args![CLEAR_ALL, group]);
    e.call(CULL_SCENE, &args![camera, scene, culling]);
    e.call(DRAW_SCENE, &args![camera, accumulator]);
    e.call(END_FRAME, &args![]);
    e.mem.set_u8(scene + SCENE_RENDERING_TEXTURE, 0);
    e.mem.set_u32(mask_at, old_mask);
    let made = e.mem.u32(texture + TEXTURE_NI_TEXTURE);
    replace_object(e, slot, made);
    release(e, texture);
}

/// JIP's culling process for the interface, one for the run.
#[derive(Default)]
pub struct UiCulling(AtomicU32);

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// BSTextureManager::GenerateRenderedUITexture (asm).
/// The interface tiles under `node` drawn, as the area `area` (x, y, width, height) of the screen,
/// into a new A8R8G8B8 rendered texture of the area's size (its width and height made integers,
/// rounded), which then takes the place of the texture at `slot`. On the way: JIP's culling process
/// for the interface (made the first time: an accumulator of 0x348 bytes made with `0x64, 1,
/// 0x2F7`, counted 2, in tile mode, with the culling process after it at +0x280, all passing), the
/// renderer's clear mask 0 for the draw, the frame begun with a back buffer clear unless a scene is
/// being drawn, the interface camera's view port the area (x, x + width, 2 * the half screen
/// height - y, that less the height), the node shown for the draw, the passes; then the node's
/// hidden bit, the mask and the view port put back. Nothing (the slot as it was) when the game makes
/// no texture.
pub fn generate_rendered_ui_texture(
    e: &mut Engine,
    node: u32,
    area: [f32; 4],
    slot: u32,
    half_height: f32,
) {
    let width = cvtps2dq(area[2]);
    let height = cvtps2dq(area[3]);
    let texture = new_rendered_texture(e, width, height, A8R8G8B8);
    // nv-rs's own, as in [`generate_rendered_texture`]: no texture (an area that rounds to 0),
    // nothing drawn.
    if texture == 0 {
        return;
    }
    let renderer = e.mem.u32(RENDERER);
    let mut culling = e.ext_free::<UiCulling>().0.load(Ordering::Acquire);
    if culling == 0 {
        let block = e.call(NI_ALLOC, &args![0x348u32]).u32();
        let accumulator = e
            .call(ACCUMULATOR_CTOR, &args![block, 0x64u32, 1u32, 0x2f7u32])
            .u32();
        e.mem.set_u32(accumulator + 4, 2);
        e.mem
            .set_u32(accumulator + ACCUMULATOR_RENDER_MODE, RENDER_MODE_TILES);
        culling = e
            .call(CULLING_PROCESS_CTOR, &args![accumulator + 0x280, 0u32])
            .u32();
        e.mem.set_u32(culling + CULLING_ACCUMULATOR, accumulator);
        e.mem.set_u32(culling + CULLING_MODE, CULL_ALL_PASS);
        e.ext_free::<UiCulling>()
            .0
            .store(culling, Ordering::Release);
    }
    let accumulator = e.mem.u32(culling + CULLING_ACCUMULATOR);
    let ui_scene = e.mem.u32(e.mem.u32(INTERFACE_MANAGER) + 4);
    let camera = e.mem.u32(ui_scene + 0xac);
    let old_mask = e.mem.u32(renderer + RENDERER_CLEAR_MASK);
    e.mem.set_u32(renderer + RENDERER_CLEAR_MASK, 0);
    if e.mem.u32(renderer + RENDERER_SCENE_ACTIVE) == 0 {
        let group = e.mem.u32(texture + TEXTURE_TARGET_GROUP);
        e.call(BEGIN_FRAME, &args![CLEAR_BACK_BUFFER, group]);
    }
    let view_at = camera + CAMERA_VIEW_PORT;
    let saved: [u32; 4] = std::array::from_fn(|i| e.mem.u32(view_at + 4 * i as u32));
    let top = half_height + half_height - area[1];
    let view = [area[0], area[2] + area[0], top, top - area[3]];
    for (i, v) in view.iter().enumerate() {
        e.mem.set_f32(view_at + 4 * i as u32, *v);
    }
    let flags = e.mem.u8(node + NODE_FLAGS);
    let was_hidden = flags & 1;
    e.mem.set_u8(node + NODE_FLAGS, flags & 0xfe);
    e.call(CULL_SCENE, &args![camera, node, culling]);
    e.call(DRAW_SCENE, &args![camera, accumulator]);
    e.call(END_FRAME, &args![]);
    let flags = e.mem.u8(node + NODE_FLAGS);
    e.mem.set_u8(node + NODE_FLAGS, flags | was_hidden);
    e.mem.set_u32(renderer + RENDERER_CLEAR_MASK, old_mask);
    for (i, v) in saved.iter().enumerate() {
        e.mem.set_u32(view_at + 4 * i as u32, *v);
    }
    let made = e.mem.u32(texture + TEXTURE_NI_TEXTURE);
    replace_object(e, slot, made);
    release(e, texture);
}

/// `cvtps2dq` with the default rounding, a lane as `cvtss2si` takes it: to the nearest, halves to
/// even; 0x80000000 for NaN or out of range.
fn cvtps2dq(v: f32) -> u32 {
    crate::game_patches::gd::cvtss2si(v) as u32
}

/// A tri-based geometry block's class answers `ADDR_ReturnThis2` at vtable +0x1C (`GetTexturePtr`'s
/// own test; JIP's `IS_GEOMETRY` looks at +0x18, which every geometry answers); its
/// shader property (+0xA8) and that one's type byte (+0x1C): 8 `BSShaderPPLightingProperty` (its
/// texture slots' pointer at +0xAC), 0x15 `TileShaderProperty` (its texture at +0x60).
const RETURN_THIS2: u32 = 0x00e6_8810;
const GEOMETRY_SHADER: u32 = 0xa8;
const SHADER_TYPE: u32 = 0x1c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetTexturePtr
// (asm).
/// The place of the texture of the reference's geometry block named `name`: a lighting shader's
/// first texture slot (the pointer at +0xAC), or a tile shader's texture (+0x60). 0 for an empty
/// name, no 3D, no such block, a block that is not tri-based geometry, no shader or another kind.
/// The root is `GetNiNode`'s: for the player in first person the first person node.
pub fn texture_ptr(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    if name.is_empty() {
        return 0;
    }
    let root = ni_node(e, refr);
    if root == 0 {
        return 0;
    }
    let block = get_block(e, root, name);
    if block == 0 || e.mem.u32(e.mem.u32(block).wrapping_add(0x1c)) != RETURN_THIS2 {
        return 0;
    }
    let shader = e.mem.u32(block + GEOMETRY_SHADER);
    if shader == 0 {
        return 0;
    }
    match e.mem.u8(shader + SHADER_TYPE) {
        8 => e.mem.u32(shader + 0xac),
        0x15 => shader + 0x60,
        _ => 0,
    }
}

/// The process manager's high actors list (`011E0F00`: data +0, next +4); an actor's loaded data
/// (+0x64) and its root (+0x14); the "culled" flag bit (`0x100000`) of a node's flags (+0x30).
const HIGH_ACTORS: u32 = 0x011e_0f00;
const CULLED: u32 = 0x0010_0000;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameProcess.h:
// ProcessManager::UnCullHighActors (inlined in Cmd_ProjectExtraCamera_Execute).
/// Every high-process actor's root loses its "culled" bit, so another camera draws them.
pub fn uncull_high_actors(e: &mut Engine) {
    let mut entry = HIGH_ACTORS;
    while entry != 0 {
        let actor = e.mem.u32(entry);
        if actor != 0 {
            let loaded = e.mem.u32(actor + 0x64);
            if loaded != 0 {
                let root = e.mem.u32(loaded + 0x14);
                if root != 0 {
                    let flags = e.mem.u32(root + NODE_FLAGS);
                    e.mem.set_u32(root + NODE_FLAGS, flags & !CULLED);
                }
            }
        }
        entry = e.mem.u32(entry + 4);
    }
}

/// `g_TES` (`*011DEA10`) and its water manager (+0x64): its water list (+0x3C); the two settings
/// words the update clears for itself (`011C7B70` and +0x50) and the "in an extra camera" byte
/// (`011F91C5`); the update (`004E21B0`, `thiscall`: the camera, 0).
const TES: u32 = 0x011d_ea10;
const WATER_SETTINGS: u32 = 0x011c_7b70;
const EXTRA_CAMERA_BYTE: u32 = 0x011f_91c5;
const WATER_UPDATE: u32 = 0x004e_21b0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameData.cpp: WaterManager::UpdateEx (asm).
/// With water to update: the water's reflections for `camera`, with the two settings words 0 and
/// the byte `011F91C5` 1 for the call, then put back.
pub fn update_water(e: &mut Engine, camera: u32) {
    let tes = e.mem.u32(TES);
    if tes == 0 {
        return;
    }
    let water = e.mem.u32(tes + 0x64);
    if water == 0 || e.mem.u32(water + 0x3c) == 0 {
        return;
    }
    let first = e.mem.u32(WATER_SETTINGS);
    e.mem.set_u32(WATER_SETTINGS, 0);
    let second = e.mem.u32(WATER_SETTINGS + 0x50);
    e.mem.set_u32(WATER_SETTINGS + 0x50, 0);
    e.mem.set_u8(EXTRA_CAMERA_BYTE, 1);
    e.call(WATER_UPDATE, &args![water, camera, 0u32]);
    e.mem.set_u8(EXTRA_CAMERA_BYTE, 0);
    e.mem.set_u32(WATER_SETTINGS + 0x50, second);
    e.mem.set_u32(WATER_SETTINGS, first);
}

#[cfg(test)]
#[path = "rendered_textures_tests.rs"]
pub(crate) mod tests;
