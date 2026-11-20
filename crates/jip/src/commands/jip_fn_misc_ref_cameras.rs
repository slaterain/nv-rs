//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): `AttachExtraCamera` and `ProjectExtraCamera`,
//! a camera of JIP's on a node of a reference's model and the world it sees drawn into a texture
//! (a model's block or an interface image). The rest of the file's commands are in
//! `jip_fn_misc_ref.rs` and the `jip_fn_misc_ref_*` modules.
//!
//! JIP's `s_extraCamerasMap` (named cameras, names case-blind) is [`ExtraCameras`]; before a game
//! loads and at the exit to the main menu every camera comes off its node and is destroyed, and the
//! map empties ([`pre_load`], JIP's `DoPreLoadGameHousekeeping`, after the swapped LOD blocks). The
//! drawing is the game's own ([`super::rendered_textures`]).
//!
//! Threads: on the script's thread, as JIP's; the map has its own lock, not held across a game call.

use super::jip_fn_misc_ref_nodes::block_name;
use super::jip_fn_misc_ref_nodes_w::{set_name, REMOVE_OBJECT_SLOT};
use super::jip_fn_ui::target_component;
use super::nif_nodes as nif;
use super::rendered_textures::{
    generate_rendered_texture, texture_ptr, uncull_high_actors, update_water, TextureParams,
};
use super::{Handlers, FLT_PI_D_180};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::Mutex;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x28fc, attach_extra_camera),
    (0x28fd, project_extra_camera),
];

/// JIP's camera name buffer (`camName[0x40]`) and `ProjectExtraCamera`'s block name (0x80).
const NAME_BUFFER: usize = 0x40;
const BLOCK_BUFFER: usize = 0x80;
/// `NiCamera::Create` (`00A71430`, `cdecl`, no words), `NiNode::AddObject` (vtable +0xDC: the
/// child, 1).
const CAMERA_CREATE: u32 = 0x00a7_1430;
const ADD_OBJECT_SLOT: u32 = 0xdc;
/// `NiAVObject`: its count (+4), parent (+0x18), local scale (+0x64); `NiCamera`: its frustum's view
/// port (+0xDC, l r t b), near (+0xEC) and far (+0xF0) planes, `minNearPlaneDist` (+0xF8),
/// `maxFarNearRatio` (+0xFC), `LODAdjust` (+0x110).
const REF_COUNT: u32 = 4;
const PARENT: u32 = 0x18;
const LOCAL_SCALE: u32 = 0x64;
const VIEW_PORT: u32 = 0xdc;
const NEAR: u32 = 0xec;
const FAR: u32 = 0xf0;
const MIN_NEAR: u32 = 0xf8;
const MAX_FAR_NEAR: u32 = 0xfc;
const LOD_ADJUST: u32 = 0x110;
/// A node's class answers `ADDR_ReturnThis` at vtable +0xC (JIP's `IS_NODE`).
const RETURN_THIS: u32 = 0x0068_15c0;
/// `kVtbl_TileImage` and a `TileImage`'s shader property (+0x40), whose texture is at +0x60.
const TILE_IMAGE_VTABLE: u32 = 0x0106_f01c;
const TILE_SHADER: u32 = 0x40;
const TILE_SHADER_TEXTURE: u32 = 0x60;
/// The game's "HDR on" byte (`011F941E`): a model's texture is drawn in A16B16G16R16F then.
const HDR_ON: u32 = 0x011f_941e;
/// `D3DFMT_X8R8G8B8`, `D3DFMT_L8`, `D3DFMT_A16B16G16R16F`.
const X8R8G8B8: u32 = 22;
const L8: u32 = 50;
const A16B16G16R16F: u32 = 113;
/// The view port's tangent scale (`kFlt2d3`, 2/3) and the level of detail's (0.08).
const TWO_THIRDS: u32 = 0x3f2a_aaab;
const LOD_PER_DEGREE: u32 = 0x3da3_d70a;

/// JIP's `s_extraCamerasMap`: the cameras by name (the key made lower case: JIP's map is
/// case-blind), in the order they were made.
#[derive(Default)]
pub struct ExtraCameras(Mutex<Vec<(Vec<u8>, u32)>>);

fn cameras(e: &mut Engine) -> std::sync::MutexGuard<'_, Vec<(Vec<u8>, u32)>> {
    e.ext_free::<ExtraCameras>()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The camera named `name`, 0 for none.
fn camera_named(e: &mut Engine, name: &[u8]) -> u32 {
    let key = name.to_ascii_lowercase();
    cameras(e)
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(0, |&(_, c)| c)
}

/// The cameras, for the tests.
pub fn extra_cameras(e: &mut Engine) -> Vec<(Vec<u8>, u32)> {
    cameras(e).clone()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_AttachExtraCamera_Execute.
/// `ref.AttachExtraCamera name attach [node] [firstPerson]`: with `attach`, the camera of that name
/// (made the first time: counted 2, named, scale 0, near 5, far 353840, nearest near plane 1, far to
/// near ratio 70768, level of detail 1) put under the reference's node (`GetNifBlock`, which must be
/// a node) unless it is there; result 1. Without, the named camera taken off its node, destroyed
/// and forgotten; result 1. An empty name: nothing.
fn attach_extra_camera(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, NAME_BUFFER, "AttachExtraCamera");
    if name.is_empty() {
        return handled(true);
    }
    if read.int(1, 0) != 0 {
        let node_name = block_name(e, &read, 2, "AttachExtraCamera");
        let target = nif::nif_block(e, a.this_obj, read.int(3, 0), &node_name);
        if target == 0 || e.mem.u32(e.mem.u32(target).wrapping_add(0xc)) != RETURN_THIS {
            return handled(true);
        }
        let mut camera = camera_named(e, &name);
        if camera == 0 {
            camera = e.call(CAMERA_CREATE, &args![]).u32();
            if camera == 0 {
                // The game always makes one (JIP writes through it unchecked).
                return handled(true);
            }
            e.mem.set_u32(camera + REF_COUNT, 2);
            set_name(e, camera, &name);
            e.mem.set_f32(camera + LOCAL_SCALE, 0.0);
            e.mem.set_f32(camera + NEAR, 5.0);
            e.mem.set_f32(camera + FAR, 353_840.0);
            e.mem.set_f32(camera + MIN_NEAR, 1.0);
            e.mem.set_f32(camera + MAX_FAR_NEAR, 70_768.0);
            e.mem.set_f32(camera + LOD_ADJUST, 1.0);
            cameras(e).push((name.to_ascii_lowercase(), camera));
        }
        if e.mem.u32(camera + PARENT) != target {
            e.vcall(target, ADD_OBJECT_SLOT, &args![camera, 1u32]);
        }
        a.set_result(e, 1.0);
    } else {
        let key = name.to_ascii_lowercase();
        let camera = {
            let mut list = cameras(e);
            match list.iter().position(|(k, _)| *k == key) {
                Some(at) => list.remove(at).1,
                None => 0,
            }
        };
        if camera != 0 {
            destroy(e, camera);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

/// A camera off its node (`RemoveObject`) and destroyed (its deleting destructor, vtable +0, 1).
fn destroy(e: &mut Engine, camera: u32) {
    let parent = e.mem.u32(camera + PARENT);
    if parent != 0 {
        e.vcall(parent, REMOVE_OBJECT_SLOT, &args![camera]);
    }
    e.vcall(camera, 0, &args![1u32]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiViewport::SetFOV (asm).
/// The view port for a field of view of `fov` radians: `v = tan(fov) * 2/3` (JIP's `Tan`), then
/// left `-v`, right `v`, top `v`, bottom `-v`.
fn set_fov(e: &mut Engine, view_port: u32, fov: f32) {
    let v = super::ln_fn_math::tan(fov) * f32::from_bits(TWO_THIRDS);
    for (i, x) in [-v, v, v, -v].iter().enumerate() {
        e.mem.set_f32(view_port + 4 * i as u32, *x);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_ProjectExtraCamera_Execute.
/// `[ref.]ProjectExtraCamera name block fov [size] [flags]`: the named camera, when it is on a node,
/// draws the world into a `size` x `size` texture (`size & 0xFF0`, 0x100 when not given; 0: nothing)
/// that takes the place of a texture: `*path`, an interface image's (`TileImage` with a shader;
/// flag 1 grey, L8), else the calling reference's geometry block's (A16B16G16R16F while the game draws
/// in HDR, else X8R8G8B8). A new field of view (in degrees, its bits differ from the scale's, where
/// JIP keeps it) sets the view port and the level of detail (`fov * 0.08`). Before the draw the high
/// actors are shown to it and the water updated for it. Result 1 when drawn.
fn project_extra_camera(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, NAME_BUFFER, "ProjectExtraCamera");
    let block = super::buffer_arg(e, &read, 1, BLOCK_BUFFER, "ProjectExtraCamera");
    let fov = read.float(2, 0.0);
    let pixel_size = read.int(3, 0x100) & 0xff0;
    let flags = read.int(4, 0);
    let camera = camera_named(e, &name);
    if camera == 0 || e.mem.u32(camera + PARENT) == 0 || pixel_size == 0 {
        return handled(true);
    }
    let mut format = X8R8G8B8;
    let mut slot = 0;
    if block.first() == Some(&b'*') {
        if let Some((tile, _)) = target_component(e, &block[1..], false) {
            let shader = e.mem.u32(tile + TILE_SHADER);
            if e.mem.u32(tile) == TILE_IMAGE_VTABLE && shader != 0 {
                slot = shader + TILE_SHADER_TEXTURE;
                if flags & 1 != 0 {
                    format = L8;
                }
            }
        }
    } else if a.this_obj != 0 {
        slot = texture_ptr(e, a.this_obj, &block);
        if e.mem.u8(HDR_ON) != 0 {
            format = A16B16G16R16F;
        }
    }
    if slot == 0 {
        return handled(true);
    }
    if e.mem.u32(camera + LOCAL_SCALE) != fov.to_bits() {
        e.mem.set_f32(camera + LOCAL_SCALE, fov);
        set_fov(e, camera + VIEW_PORT, fov * FLT_PI_D_180);
        e.mem
            .set_f32(camera + LOD_ADJUST, fov * f32::from_bits(LOD_PER_DEGREE));
    }
    uncull_high_actors(e);
    update_water(e, camera);
    let params = TextureParams {
        width: pixel_size,
        height: pixel_size,
        format,
        render_mode: 0,
        bg_color_mask: 0xffff_ffff,
    };
    generate_rendered_texture(e, camera, params, slot);
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h:
// DoPreLoadGameHousekeeping (the extra cameras' step, after the swapped LOD blocks).
/// Before a game loads and at the exit to the main menu: every extra camera off its node and
/// destroyed, the map emptied.
pub fn pre_load(e: &mut Engine) {
    let list = std::mem::take(&mut *cameras(e));
    for (_, camera) in list {
        destroy(e, camera);
    }
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_cameras_tests.rs"]
mod tests;
