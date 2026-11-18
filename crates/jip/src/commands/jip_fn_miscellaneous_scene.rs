//! `functions_jip/jip_fn_miscellaneous.h` (JIP LN NVSE): its two scene-graph commands,
//! `SwapTextureEx` (a texture of a geometry block of a reference's model replaced by a file's)
//! and `GetLightAmountAtPoint` (how lit a point is, 0 to 100, from the sun and the scene's
//! lights). The rest of the file's commands are in `jip_fn_miscellaneous.rs`.
//!
//! `SwapTextureEx` (JIP's `TESObjectREFR::SwapTexture`, nvse/GameObjects.cpp), every offset and
//! call checked in the exe (Ghidra, FalloutNV.exe 1.4.0.525):
//! - the path: `Textures\` + the argument + `.dds` in JIP's `path[0x80]` (an argument of 0x73
//!   bytes or more runs past it: the crate's buffer rule, [`super::buffer_arg`]);
//! - the block: JIP's `GetNiNode` root as seen ([`nif::ni_node`]), the block of that name below
//!   it (`NiNode::GetBlock`: none for an empty name), kept when its vtable +0x1C answers
//!   `00E68810` (`GetAsNiTriBasedGeom`: `NiTriShape` `0109D454` and `NiTriStrips` `0109CD44`
//!   only; `NiLines` and `NiGeometry` answer 0);
//! - its shader property (+0xA8), of a type (+0x1C) whose bit is set in `0x200100` (`bt`, so the
//!   type taken modulo 32: 8, `BSShaderPPLightingProperty`, or 21, `BSShaderNoLightingProperty`,
//!   whose constructor `00B6FC90` writes 0x15 there);
//! - the texture: `TES::CreateTextureImage` (`004568C0`, `stdcall` (path, &texture, 0, 0),
//!   `RET 0x10`); none made, nothing more;
//! - the slot: for type 8 the word at +0xAC + 4 * index (the lighting property's six texture
//!   arrays, one per kind, engine-bs `bsshaderpplightingproperty.rs`: that kind's first entry is
//!   replaced; an index past 5 reads the words after the arrays, as JIP does, unbounded); for
//!   type 21 +0x60 (its constructor zeroes it), after the word +0x38 is cleared;
//! - JIP's `NiReplaceObject` (the slot's old texture counted down and freed at 0, the new one
//!   counted up: [`crate::game_patches::gx::replace_object`]) then `NiReleaseObject` on the new
//!   texture (the load's own count down; freed through its vtable +4 at 0).
//!
//! `GetLightAmountAtPoint` (JIP's `GetLightAmountAtPoint` and `GetLightAmount`,
//! internal/jip_core.cpp; the comparisons' widths read in JIP's 57.30 DLL: `cmp byte`):
//! - the sun's `ShadowSceneLight` (`ShadowSceneNode::pSunLight`, +0xE0 of `*0x011F91C8`); none:
//!   0 (and nothing else);
//! - in an interior (`TES` `*0x011DEA10` +0x34 not 0): the sun light's (`+0xF8`) largest
//!   ambient channel (+0xC8..) plus its largest diffuse channel (+0xD4..); in an exterior: 0
//!   when no grid cell holds the point ([`super::havok_refs::grid_cell_at`]), else the sun's own
//!   amount;
//! - a light's amount: 0 when its `ShadowSceneLight` casts shadows (byte +0xEC is 1) or the
//!   light is culled (flag byte +0x30 bit 0); for a point light (byte +0xF4) the distance from
//!   its world position (+0x8C) over its radius (+0xE0), 0 at 1 or past, else the factor
//!   `1 - d * d`; then the largest diffuse channel times the dimmer (+0xC4) times the factor;
//!   the distance `sqrt((dx² + dy²) + dz²)` (JIP's two `haddps`), all in single precision;
//! - then, under the scene lights' lock (`0x011F9EA0`, the game's `0040FBF0` / `0040FBA0`, as
//!   JIP's `LightCS`), each light of the scene's list (`+0xB4`: a node's next +0, light +8)
//!   added while the sum is below 1.0 (a NaN sum walks on);
//! - the sum kept at 0 or above (`maxss`: NaN becomes 0), times 100, at most 100.
//!
//! Deliberate differences, where JIP reads through null or garbage: a missing `TES`, sun light,
//! light or slot counts as nothing (no amount, no replace; the texture made is still released);
//! the scene list walk stops at an unreadable node and after `MAX_WALK` nodes; a texture
//! index's slot word that cannot be read is no slot.
//!
//! Threads: on the script's thread, as JIP's; the scene list is read under the game's lock as
//! JIP does, the texture slots without one (as JIP).

use super::nif_nodes as nif;
use super::Handlers;
use crate::game_patches::gx::{refs_add, replace_object};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x278b, swap_texture_ex),
    (0x2920, get_light_amount_at_point),
];

/// JIP's `blockName[0x40]`, `path[0x80]` with `Textures\` (9 bytes) before the argument and
/// `.dds` (4 bytes and its 0) after it: an argument of 0x73 bytes or more runs past the buffer.
const NAME_BUFFER: usize = 0x40;
const PATH_ROOM: usize = 0x80 - 9 - 4;
/// A geometry block's vtable slot +0x1C (`GetAsNiTriBasedGeom`) and the answer of the tri-based
/// classes (`00E68810`, returns `this`).
const AS_TRI_BASED_SLOT: u32 = 0x1c;
const RETURN_THIS2: u32 = 0x00e6_8810;
/// `NiGeometry`'s shader property (+0xA8); a shader property's type (+0x1C), the types JIP
/// takes (`bt 0x200100`), the lighting property's texture lists (+0xAC), the no-lighting
/// property's word +0x38 and texture +0x60.
const SHADER_PROPERTY: u32 = 0xa8;
const SHADER_TYPE: u32 = 0x1c;
const SWAP_TYPES: u32 = 0x0020_0100;
const LIGHTING: u32 = 8;
const LIGHTING_TEXTURES: u32 = 0xac;
const NO_LIGHTING_38: u32 = 0x38;
const NO_LIGHTING_TEXTURE: u32 = 0x60;
/// `TES::CreateTextureImage` (`004568C0`).
const CREATE_TEXTURE_IMAGE: u32 = 0x0045_68c0;
/// An `NiRefObject`'s vtable delete (+4, no argument: `00CFCC20` in `NiNode`'s, `RET`).
const DELETE_SLOT: u32 = 0x04;

/// `ShadowSceneNode` (`*0x011F91C8`): the sun (+0xE0), the lights list's first node (+0xB4).
const SHADOW_SCENE_NODE: u32 = 0x011f_91c8;
const SSN_SUN: u32 = 0xe0;
const SSN_LIGHTS: u32 = 0xb4;
/// `ShadowSceneLight`: casts shadows (byte +0xEC), point light (byte +0xF4), the light (+0xF8).
const SSL_CASTS_SHADOW: u32 = 0xec;
const SSL_POINT: u32 = 0xf4;
const SSL_LIGHT: u32 = 0xf8;
/// `NiLight` / `NiPointLight`: flags (+0x30), world translation (+0x8C), dimmer (+0xC4),
/// ambient (+0xC8), diffuse (+0xD4), radius (+0xE0).
const LIGHT_FLAGS: u32 = 0x30;
const LIGHT_WORLD_POS: u32 = 0x8c;
const LIGHT_DIMMER: u32 = 0xc4;
const LIGHT_AMBIENT: u32 = 0xc8;
const LIGHT_DIFFUSE: u32 = 0xd4;
const LIGHT_RADIUS: u32 = 0xe0;
/// The scene lights' lock and the game's enter (`(lock, 0)`) and leave.
const SCENE_LIGHTS_LOCK: u32 = 0x011f_9ea0;
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
/// The most list nodes walked.
const MAX_WALK: u32 = 1 << 16;

// ---- SwapTextureEx ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::SwapTexture
// (asm).
/// `block`'s shader property when the block is tri-based geometry and the property's type takes
/// a swap; `None` otherwise.
fn swap_property(e: &Engine, block: u32) -> Option<u32> {
    let vtbl = nif::ptr(e, block)?;
    if nif::rd32(e, vtbl.wrapping_add(AS_TRI_BASED_SLOT)) != Some(RETURN_THIS2) {
        return None;
    }
    let property = nif::ptr(e, block.wrapping_add(SHADER_PROPERTY))?;
    let kind = nif::rd32(e, property.wrapping_add(SHADER_TYPE))?;
    if SWAP_TYPES >> (kind & 31) & 1 == 0 {
        return None;
    }
    Some(property)
}

/// `TESObjectREFR::SwapTexture` (JIP): see the module's doc.
fn swap_texture(e: &mut Engine, refr: u32, name: &[u8], path: &[u8], index: u32) {
    let root = nif::ni_node(e, refr);
    if root == 0 {
        return;
    }
    let block = nif::get_block(e, root, name);
    if block == 0 {
        return;
    }
    let Some(property) = swap_property(e, block) else {
        return;
    };
    let texture = super::with_guest_text(e, path, |e, at| {
        e.with_stack(4, |e, out| {
            let out = out.addr();
            e.mem.set_u32(out, 0);
            e.call(CREATE_TEXTURE_IMAGE, &args![at, out, 0u32, 0u32]);
            e.mem.u32(out)
        })
    });
    if texture == 0 {
        return;
    }
    // `cmp byte ptr [property+0x1C], 8`: the type's low byte.
    let slot = if e.mem.u8(property.wrapping_add(SHADER_TYPE)) == LIGHTING as u8 {
        nif::ptr(
            e,
            property
                .wrapping_add(LIGHTING_TEXTURES)
                .wrapping_add(index.wrapping_mul(4)),
        )
    } else {
        e.mem.set_u32(property.wrapping_add(NO_LIGHTING_38), 0);
        Some(property.wrapping_add(NO_LIGHTING_TEXTURE))
    };
    if let Some(slot) = slot {
        replace_object(e, slot, texture);
    }
    release_object(e, texture);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: NiReleaseObject.
/// `object`'s count down; at 0 or below it is deleted (its vtable +4, no argument).
fn release_object(e: &mut Engine, object: u32) {
    if refs_add(e, object, -1) <= 0 {
        let vtable = e.mem.u32(object);
        let delete = e.mem.u32(vtable.wrapping_add(DELETE_SLOT));
        e.call(delete, &[object]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SwapTextureEx_Execute.
/// A reference, a block's name, a texture path below `Textures\` without `.dds`, and the texture
/// index (0 when not given): the block's texture swapped. No result.
fn swap_texture_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let refr = read.form(0);
    let name = super::buffer_arg(e, &read, 1, NAME_BUFFER, "SwapTextureEx");
    let file = super::buffer_arg(e, &read, 2, PATH_ROOM, "SwapTextureEx");
    let index = read.int(3, 0);
    if refr == 0 {
        return handled(true);
    }
    let path = [b"Textures\\".as_slice(), &file, b".dds"].concat();
    swap_texture(e, refr, &name, &path, index);
    handled(true)
}

// ---- GetLightAmountAtPoint ----

/// `maxss a, b` and `minss a, b`: `a` when it is the larger (smaller), else `b` (NaN gives `b`).
fn maxss(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}
fn minss(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

/// The largest of the three channels at `at`, in JIP's order (`maxss` twice).
fn largest(e: &Engine, at: u32) -> Option<f32> {
    let c = nif::floats3(e, at)?;
    Some(maxss(maxss(c[0], c[1]), c[2]))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetLightAmount (asm).
/// How much `data` (a `ShadowSceneLight`) lights `pos`; 0 for a light it cannot read.
fn light_amount(e: &Engine, data: u32, pos: [f32; 3]) -> f32 {
    if nif::rd8(e, data.wrapping_add(SSL_CASTS_SHADOW)) == Some(1) {
        return 0.0;
    }
    let Some(light) = nif::ptr(e, data.wrapping_add(SSL_LIGHT)) else {
        return 0.0;
    };
    if nif::rd8(e, light.wrapping_add(LIGHT_FLAGS)).map_or(true, |f| f & 1 != 0) {
        return 0.0;
    }
    let mut factor = 1.0f32;
    if nif::rd8(e, data.wrapping_add(SSL_POINT)).is_some_and(|b| b != 0) {
        let (Some(at), Some(radius)) = (
            nif::floats3(e, light.wrapping_add(LIGHT_WORLD_POS)),
            nif::rdf32(e, light.wrapping_add(LIGHT_RADIUS)),
        ) else {
            return 0.0;
        };
        let d: [f32; 3] = std::array::from_fn(|i| at[i] - pos[i]);
        let distance = ((d[0] * d[0] + d[1] * d[1]) + (d[2] * d[2])).sqrt() / radius;
        // `comiss` and `jnb`: 1.0 or past (not a NaN) is no light.
        if distance >= 1.0 {
            return 0.0;
        }
        factor = 1.0 - distance * distance;
    }
    let (Some(diffuse), Some(dimmer)) = (
        largest(e, light.wrapping_add(LIGHT_DIFFUSE)),
        nif::rdf32(e, light.wrapping_add(LIGHT_DIMMER)),
    ) else {
        return 0.0;
    };
    diffuse * dimmer * factor
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetLightAmountAtPoint
// (asm).
/// How lit `pos` is, 0 to 100: see the module's doc.
pub(crate) fn light_amount_at_point(e: &mut Engine, pos: [f32; 3]) -> f32 {
    let Some(scene) = nif::ptr(e, SHADOW_SCENE_NODE) else {
        return 0.0;
    };
    let Some(sun) = nif::ptr(e, scene.wrapping_add(SSN_SUN)) else {
        return 0.0;
    };
    let Some(tes) = nif::ptr(e, super::havok_refs::TES) else {
        return 0.0;
    };
    let interior =
        nif::rd32(e, tes.wrapping_add(super::havok_refs::TES_INTERIOR)).is_some_and(|c| c != 0);
    let mut sum = if interior {
        let Some(light) = nif::ptr(e, sun.wrapping_add(SSL_LIGHT)) else {
            return 0.0;
        };
        let (Some(ambient), Some(diffuse)) = (
            largest(e, light.wrapping_add(LIGHT_AMBIENT)),
            largest(e, light.wrapping_add(LIGHT_DIFFUSE)),
        ) else {
            return 0.0;
        };
        diffuse + ambient
    } else {
        if super::havok_refs::grid_cell_at(e, tes, pos[0], pos[1]) == 0 {
            return 0.0;
        }
        light_amount(e, sun, pos)
    };
    e.call(LOCK_ENTER, &args![SCENE_LIGHTS_LOCK, 0u32]);
    let mut node = nif::rd32(e, scene.wrapping_add(SSN_LIGHTS)).unwrap_or(0);
    let mut walked = 0;
    // `comiss sum, 1.0` and `jnb`: a NaN sum walks on.
    while (sum < 1.0 || sum.is_nan()) && node != 0 && walked < MAX_WALK {
        walked += 1;
        let (Some(next), Some(data)) = (nif::rd32(e, node), nif::rd32(e, node.wrapping_add(8)))
        else {
            break;
        };
        node = next;
        if data != 0 {
            sum += light_amount(e, data, pos);
        }
    }
    let amount = minss(maxss(sum, 0.0) * 100.0, 100.0);
    e.call(LOCK_LEAVE, &args![SCENE_LIGHTS_LOCK]);
    amount
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetLightAmountAtPoint_Execute.
/// Three floats, a point: how lit it is, 0 to 100.
fn get_light_amount_at_point(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let pos = [read.float(0, 0.0), read.float(1, 0.0), read.float(2, 0.0)];
        let amount = light_amount_at_point(e, pos);
        a.set_result(e, f64::from(amount));
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_miscellaneous_scene_tests.rs"]
mod tests;
