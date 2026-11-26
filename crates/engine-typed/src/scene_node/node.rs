//! The shadow scene node's (`ShadowSceneNode`, Xbox PDB) own passes: the sun light set
//! (`SetSunLight`), the occlusion planes' visibility for a camera (`UpdateOcclusionPlaneVisibility`),
//! a light listed (`00b5eca0`) and the queued lights removed (`RemoveQueuedLight`, both overloads),
//! the light value at a point (`00b5b4d0`), the dynamic shadow maps drawn
//! (`UpdateDynamicShadows`), the destructor and its deleting form, and the five-byte jump to the
//! counted list's `RemoveAll`.
//!
//! Done in place: the `NiPointer` counts, the culling process's planes (`NiPlane`'s constructor
//! `00a69940`: the zero point and a zero constant) and its destructor (`00a693e0`: the base
//! vtable stored back), the renderer's word +0xa8 (its getter `00b6d120`), the sun light's colour
//! peaks (`FCOM` choices of loaded floats).
//! Typed calls: the culling process's frustum (`00a694a0`, the culling family's), the light's
//! constructor, `SetLight`, luminance at a point and depth map draw (the lights family's), the
//! object update (`00a59c60`, the scene family's), the shader accumulator (`00b4f5c0`, the render
//! family's), the counted list's add (`00a5a310`, the cell references family's) and this family's
//! own functions. What stays by address: the allocations and frees, `NiDirectionalLight`'s,
//! `NiCullingProcess`'s and `BSCubeMapCamera`'s constructors, the occlusion plane's two tests, the
//! light update (`00b5d300`) and cull (`00b9ef30`), the lists' `RemovePos` and `Remove`, the
//! renderer's map calls, the rendering system's `00ba3130`, the scene graph slot release, the
//! compound frustum's and `NiNode`'s destructors, and the deleting destructor slots.
//!
//! **Floats.** The light value sum (`00b5b4d0`) runs on main and on the AI threads (the actor's
//! light value), so it is written over [`Real`]: each light's value added to the running `float`
//! at the thread's precision and stored. The colour peaks only compare loaded floats (a signalling
//! NaN chosen comes back quiet, as the exe's `FLD` makes it).

use super::lists::{
    self, add_tail_held, assign, element_of, head_of, next_of, ni_light_of, release, AV_FLAGS,
    LIGHT_CULL, LIGHT_LIST, LIGHT_QUEUE_ADD, LIGHT_QUEUE_REMOVE, LIGHT_QUEUE_UPDATE, LIGHT_UPDATE,
    NEW_OBJECT, PORTAL_GRAPH, RENDERER, SHADOW_LIGHT_LIST, SHADOW_SCENE_LIGHT_SIZE,
    WORLD_TRANSLATE,
};
use super::FAMILY;
use crate::lights::{self, scene_frustum, scene_light, ShadowSceneLight};
use crate::native::{float_precision, Real};
use crate::{culling, render, scene};
use engine_core::ptr::Ptr;
use engine_core::types::NiTPointerList;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The node's members (Xbox PDB names): `kShadowLightActiveFenceObject` and
/// `kShadowLightFadingInFenceObject` (+0xd8, +0xdc), `pSunLight` (+0xe0), `lObjectQueueUpdate`
/// (+0x108), `lObjectQueueUpdateActiveOnly` (+0x114), `cSceneGraphIndex` (+0x120, a byte),
/// `pWindowReflectionEnvMap` (+0x124), `spCubeMapCamera` (+0x128), `spFog` (+0x134) and the
/// compound frustum (+0x138).
const ACTIVE_FENCE_OBJECT: u32 = 0xd8;
const FADING_FENCE_OBJECT: u32 = 0xdc;
const SUN_LIGHT: u32 = 0xe0;
const OBJECT_QUEUE_UPDATE: u32 = 0x108;
const OBJECT_QUEUE_UPDATE_ACTIVE_ONLY: u32 = 0x114;
const SCENE_GRAPH_INDEX: u32 = 0x120;
const WINDOW_REFLECTION_MAP: u32 = 0x124;
const CUBE_MAP_CAMERA: u32 = 0x128;
const FOG: u32 = 0x134;
const COMPOUND_FRUSTUM: u32 = 0x138;
/// A shadow scene light's `bCastShadow` (+0xec), `bDynamicLight` (+0xfc), `usFrustumCull` (+0x110,
/// 16 bits) and its two arrays the portal graph fills (+0x214 count +0x218, +0x224 count +0x228).
const CAST_SHADOW: u32 = 0xec;
const DYNAMIC_LIGHT: u32 = 0xfc;
const FRUSTUM_CULL: u32 = 0x110;
const LIGHT_ROOMS: u32 = 0x214;
const LIGHT_ROOMS_COUNT: u32 = 0x218;
const LIGHT_PORTALS: u32 = 0x224;
const LIGHT_PORTALS_COUNT: u32 = 0x228;
/// The portal graph's word +0x28 (set when its rooms hold lights) and its occlusion planes' list
/// head (+8); a room's light array (+0xdc, count +0xe4); a portal's object (+0x104) and that
/// object's light array (+0xb0: buffer +0xb4, count +0xb8); an occlusion plane's state (+0x40).
const GRAPH_HAS_LIGHTS: u32 = 0x28;
const GRAPH_PLANES: u32 = 8;
const ROOM_LIGHTS: u32 = 0xdc;
const ROOM_LIGHTS_COUNT: u32 = 0xe4;
const PORTAL_OBJECT: u32 = 0x104;
const OBJECT_LIGHTS: u32 = 0xb0;
const PLANE_STATE: u32 = 0x40;
/// The deleting destructor slot of a shadow scene light (+0, a flag word).
const SLOT_DELETING_DESTRUCTOR: u32 = 0;

// ---------------------------------------------------------------------------------------------
// 00b5c7c0: the jump to the counted list's RemoveAll.

// Typed from 00b5c7c0
/// A five-byte jump to the counted list's `RemoveAll` (`00e74d40`), `this` passed on.
pub fn remove_all_jump(e: &mut Engine, this: Ptr<NiTPointerList>) {
    typed_call!(FAMILY, 0x00e7_4d40, lists::remove_all(e, this));
}

// ---------------------------------------------------------------------------------------------
// 00b5aac0: the sun light.

/// `NiDirectionalLight`'s constructor (`00a75c20`, no stack words, 0xfc bytes) and the colour its
/// ambient and diffuse colours (+0xc8, +0xd4) take (`011f4998`, three floats); an `NiLight`'s
/// revision word (+0xa8).
const DIRECTIONAL_LIGHT_CONSTRUCT: u32 = 0x00a7_5c20;
const DIRECTIONAL_LIGHT_SIZE: u32 = 0xfc;
const SUN_COLOUR: u32 = 0x011f_4998;
const LIGHT_AMBIENT: u32 = 0xc8;
const LIGHT_DIFFUSE: u32 = 0xd4;
const LIGHT_REVISION: u32 = 0xa8;

/// A new `ShadowSceneLight` (0x250 bytes, constructed), or 0 when the allocation fails.
fn new_shadow_light(e: &mut Engine) -> u32 {
    let memory = e.call(NEW_OBJECT, &args![SHADOW_SCENE_LIGHT_SIZE]).u32();
    if memory == 0 {
        return 0;
    }
    typed_call!(
        lights::FAMILY,
        0x00b9_fda0,
        scene_light::construct(e, Ptr::new(memory))
    )
    .addr()
}

/// Three words copied.
fn copy_three(e: &mut Engine, to: u32, from: u32) {
    for offset in [0u32, 4, 8] {
        let word = e.mem.u32(from.wrapping_add(offset));
        e.mem.set_u32(to.wrapping_add(offset), word);
    }
}

// Typed from 00b5aac0
/// `ShadowSceneNode::SetSunLight` (Xbox PDB, `thiscall(node, light)`, `ret 4`): the sun light
/// deleted (its slot 0 with 1) and `pSunLight` cleared; a new `ShadowSceneLight` (null when its
/// allocation fails) for the given `NiLight`, or without one for a new `NiDirectionalLight` (null
/// when its allocation fails) whose ambient and diffuse colours become the sun colour and whose
/// revision goes up twice; the new light becomes `pSunLight` and is set to that `NiLight`.
pub fn set_sun_light(e: &mut Engine, this: Ptr, light: u32) {
    let node = this.addr();
    let sun = e.mem.u32(node.wrapping_add(SUN_LIGHT));
    if sun != 0 {
        e.vcall(sun, SLOT_DELETING_DESTRUCTOR, &args![1u32]);
        e.mem.set_u32(node.wrapping_add(SUN_LIGHT), 0);
    }
    let light = if light != 0 {
        light
    } else {
        let memory = e.call(NEW_OBJECT, &args![DIRECTIONAL_LIGHT_SIZE]).u32();
        let directional = if memory != 0 {
            e.call(DIRECTIONAL_LIGHT_CONSTRUCT, &args![memory]).u32()
        } else {
            0
        };
        copy_three(e, directional.wrapping_add(LIGHT_AMBIENT), SUN_COLOUR);
        copy_three(e, directional.wrapping_add(LIGHT_DIFFUSE), SUN_COLOUR);
        let revision = directional.wrapping_add(LIGHT_REVISION);
        let raised = e.mem.u32(revision).wrapping_add(2);
        e.mem.set_u32(revision, raised);
        directional
    };
    let shadow_light = new_shadow_light(e);
    e.mem.set_u32(node.wrapping_add(SUN_LIGHT), shadow_light);
    typed_call!(
        lights::FAMILY,
        0x00b9_dda0,
        scene_light::set_light(e, Ptr::new(shadow_light), Ptr::new(light))
    );
}

// ---------------------------------------------------------------------------------------------
// 00b5ac90: the occlusion planes.

/// `NiCullingProcess`'s constructor (`00a69400`, `ret 4`) and the base vtable its destructor
/// (`00a693e0`) stores back (`0109c28c`); the local's layout as the exe has it: the camera
/// (+0xc), the frustum's planes (+0x2c), six more planes (+0x90) and their active mask (+0xf0),
/// 0xf4 bytes. A camera's frustum (+0xdc); the zero point (`011f426c`).
const CULLER_CONSTRUCT: u32 = 0x00a6_9400;
const CULLER_VTABLE: u32 = 0x0109_c28c;
const CULLER_SIZE: u32 = 0xf4;
const CULLER_CAMERA: u32 = 0xc;
const CULLER_PLANES: u32 = 0x2c;
const CULLER_MORE_PLANES: u32 = 0x90;
const CULLER_ACTIVE_PLANES: u32 = 0xf0;
const CAMERA_FRUSTUM: u32 = 0xdc;
const ZERO_POINT: u32 = 0x011f_426c;
/// `BSOcclusionPlane::WithinFrustum` (`00c33cf0`, `ret 0x14`: the planes, 0, a point) and
/// `ViewerOnCorrectSide` (`00c358a0`, `ret 0xc`: a point), each a byte.
const PLANE_WITHIN_FRUSTUM: u32 = 0x00c3_3cf0;
const PLANE_VIEWER_ON_CORRECT_SIDE: u32 = 0x00c3_58a0;

/// The three words of a point.
fn three(e: &Engine, at: u32) -> [u32; 3] {
    [0u32, 4, 8].map(|offset| e.mem.u32(at.wrapping_add(offset)))
}

// Typed from 00b5ac90
/// `ShadowSceneNode::UpdateOcclusionPlaneVisibility` (Xbox PDB, `thiscall(node, camera)`, `ret
/// 4`): only with a portal graph. A local culling process for the camera's frustum (its six more
/// planes the zero point and zero, all six active); each occlusion plane of the graph's list gets
/// the state 1 when it is within that frustum and the camera's world position is on its correct
/// side, else 2. The local's vtable is the base one again at the end.
pub fn update_occlusion_plane_visibility(e: &mut Engine, this: Ptr, camera: u32) {
    let graph = e.mem.u32(this.addr().wrapping_add(PORTAL_GRAPH));
    if graph == 0 {
        return;
    }
    e.with_stack(CULLER_SIZE, |e, local| {
        let culler = local.addr();
        e.call(CULLER_CONSTRUCT, &args![culler, 0u32]);
        e.mem.set_u32(culler + CULLER_CAMERA, camera);
        typed_call!(
            culling::FAMILY,
            0x00a6_94a0,
            culling::process::set_frustum(
                e,
                Ptr::new(culler),
                Ptr::new(camera.wrapping_add(CAMERA_FRUSTUM))
            )
        );
        // The graph's word again, as the exe reads it after the frustum is set.
        let graph = e.mem.u32(this.addr().wrapping_add(PORTAL_GRAPH));
        let mut item = e.mem.u32(graph.wrapping_add(GRAPH_PLANES));
        let [x, y, z] = three(e, ZERO_POINT);
        for plane in 0..6u32 {
            let at = culler + CULLER_MORE_PLANES + 0x10 * plane;
            e.mem.set_u32(at, x);
            e.mem.set_u32(at + 4, y);
            e.mem.set_u32(at + 8, z);
            e.mem.set_u32(at + 0xc, 0);
        }
        e.mem.set_u32(culler + CULLER_ACTIVE_PLANES, 0x3f);
        while item != 0 {
            let plane = element_of(e, item);
            item = next_of(e, item);
            if plane == 0 {
                continue;
            }
            let [x, y, z] = three(e, ZERO_POINT);
            let within = e
                .call(
                    PLANE_WITHIN_FRUSTUM,
                    &args![plane, culler + CULLER_PLANES, 0u32, x, y, z],
                )
                .u8();
            let [px, py, pz] = three(e, camera.wrapping_add(WORLD_TRANSLATE));
            let side = e
                .call(PLANE_VIEWER_ON_CORRECT_SIDE, &args![plane, px, py, pz])
                .u8();
            let state = if within & side == 0 { 2 } else { 1 };
            e.mem.set_u32(plane.wrapping_add(PLANE_STATE), state);
        }
        e.mem.set_u32(culler, CULLER_VTABLE);
    });
}

// ---------------------------------------------------------------------------------------------
// 00b5eca0: a light listed.

/// The renderer's shadow map calls (`thiscall` on the renderer): a new map (`00b6e110`, `ret
/// 0x14`: the word at `011f9508`, 0x2a, 0, 0, 0) and a map of its own (`00b6d3f0`, no stack
/// words); its word +0xa8 (what its getter `00b6d120` answers: read in place);
/// `BSRenderedTexture::StopOffscreen` (`00b6b260`) and the slot +0xac its answer's vtable holds.
const RENDERER_NEW_MAP: u32 = 0x00b6_e110;
const RENDERER_MAP_WORD: u32 = 0x011f_9508;
const RENDERER_WORD_A8: u32 = 0xa8;
const RENDERER_SHADOW_MAP: u32 = 0x00b6_d3f0;
const STOP_OFFSCREEN: u32 = 0x00b6_b260;
const SLOT_TARGET_SET: u32 = 0xac;
/// The byte the exe reads at absolute address 0xed (`CMP byte ptr [0xed], 0`).
const ABSOLUTE_0XED: u32 = 0xed;

// Typed from 00b5eca0
/// A light added to `lLightList` (`thiscall(node, light)`, `ret 4`): a listed light with the same
/// `NiLight` takes the new light's `bDynamicLight`, is updated (`00b5d300`) and is answered.
/// Otherwise the light is appended through an `NiPointer` temporary; with `bCastShadow` it gets a
/// shadow map (`00b5aee0`): when the byte at absolute address 0xed is set (as the exe reads it), a
/// new map from the renderer, its `StopOffscreen` target's slot +0xac called with the renderer's
/// word +0xa8 (its getter `00b6d120` read in place, so nothing runs between the vtable's read and
/// the slot's); otherwise a map of the renderer's own (`00b6d3f0`). Then it is updated and 0 is
/// answered.
pub fn add_listed_light(e: &mut Engine, this: Ptr, light: u32) -> u32 {
    let node = this.addr();
    let ni_light = ni_light_of(e, light);
    let mut item = head_of(e, node, LIGHT_LIST);
    while item != 0 {
        let listed = element_of(e, item);
        item = next_of(e, item);
        if listed != 0 && ni_light_of(e, listed) == ni_light {
            let dynamic = e.mem.u8(light.wrapping_add(DYNAMIC_LIGHT));
            e.mem.set_u8(listed.wrapping_add(DYNAMIC_LIGHT), dynamic);
            e.call(LIGHT_UPDATE, &args![node, listed]);
            return listed;
        }
    }
    add_tail_held(e, node.wrapping_add(LIGHT_LIST), light, false);
    if e.mem.u8(light.wrapping_add(CAST_SHADOW)) != 0 {
        let map = if e.mem.u8(ABSOLUTE_0XED) != 0 {
            let renderer = e.mem.u32(RENDERER);
            let word = e.mem.u32(RENDERER_MAP_WORD);
            let map = e
                .call(
                    RENDERER_NEW_MAP,
                    &args![renderer, word, 0x2au32, 0u32, 0u32, 0u32],
                )
                .u32();
            let target = e.call(STOP_OFFSCREEN, &args![map]).u32();
            let renderer = e.mem.u32(RENDERER);
            let value = e.mem.u32(renderer.wrapping_add(RENDERER_WORD_A8));
            e.vcall(target, SLOT_TARGET_SET, &args![value]);
            map
        } else {
            let renderer = e.mem.u32(RENDERER);
            e.call(RENDERER_SHADOW_MAP, &args![renderer]).u32()
        };
        typed_call!(
            FAMILY,
            0x00b5_aee0,
            lists::set_shadow_map(e, Ptr::new(light), map)
        );
    }
    e.call(LIGHT_UPDATE, &args![node, light]);
    0
}

// ---------------------------------------------------------------------------------------------
// 00b5d040, 00b5ef90: the queued lights removed.

/// `NiTPointerList`'s `RemovePos` (`00b9efa0`, `ret 8`: the out `NiPointer`, the position) and
/// `Remove` (`00b9fcc0`, `ret 8`: the out `NiPointer`, the element's pointer) (engine map).
const LIST_REMOVE_POS: u32 = 0x00b9_efa0;
const LIST_REMOVE: u32 = 0x00b9_fcc0;
/// The array search's compare (`00b5a980`): a word against the value's.
const ARRAY_COMPARE: u32 = 0x00b5_a980;

/// The list a light is kept in: `lShadowLightList` for a light with `bCastShadow`, else `lLightList`.
fn list_of(e: &Engine, node: u32, light: u32) -> u32 {
    let list = if e.mem.u8(light.wrapping_add(CAST_SHADOW)) != 0 {
        SHADOW_LIGHT_LIST
    } else {
        LIGHT_LIST
    };
    node.wrapping_add(list)
}

/// A list removal whose out `NiPointer` is the argument word itself (`word`, at a stack slot as
/// the exe's own argument slot): the removed light let go after.
fn remove_into_word(e: &mut Engine, callee: u32, list: u32, word: u32, argument: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), word);
        e.call(callee, &args![list, slot.addr(), argument]);
        let removed = e.mem.u32(slot.addr());
        if removed != 0 {
            release(e, removed);
        }
    });
}

// Typed from 00b5d040
/// `ShadowSceneNode::RemoveQueuedLight` (Xbox PDB, `thiscall(node, pointer, position)`, `ret 8`:
/// an `NiPointer`'s address and a list position). With a portal graph whose word +0x28 is set:
/// the light is taken out of the light array of each room in its first array (`00b5a930` with
/// the compare `00b5a980`, then `00b5ade0` with 1) and out of the light array of each portal's
/// object in its second (a search of the buffer, then `00b5ade0`). Then the position is removed
/// from the light's list (`RemovePos`, its out `NiPointer` the position's own word) and the
/// removed pointer let go.
pub fn remove_queued_light(e: &mut Engine, this: Ptr, pointer: Ptr, position: u32) {
    let node = this.addr();
    let holder = pointer.addr();
    let graph = e.mem.u32(node.wrapping_add(PORTAL_GRAPH));
    if graph != 0 && e.mem.u32(graph.wrapping_add(GRAPH_HAS_LIGHTS)) != 0 {
        // The light is read again after each array's step (`MOV EAX,[EBP]`): the arrays' code
        // runs in between.
        let mut light = e.mem.u32(holder);
        let mut index = 0u32;
        while index < e.mem.u32(light.wrapping_add(LIGHT_ROOMS_COUNT)) {
            let rooms = e.mem.u32(light.wrapping_add(LIGHT_ROOMS));
            let room = e.mem.u32(rooms.wrapping_add(index.wrapping_mul(4)));
            if room != 0 && e.mem.u32(room.wrapping_add(ROOM_LIGHTS_COUNT)) != 0 {
                let array = Ptr::new(room.wrapping_add(ROOM_LIGHTS));
                let found = typed_call!(
                    FAMILY,
                    0x00b5_a930,
                    lists::find(e, array, holder, 0u32, ARRAY_COMPARE)
                );
                if found != u32::MAX {
                    typed_call!(FAMILY, 0x00b5_ade0, lists::remove_at(e, array, found, 1u8));
                }
            }
            light = e.mem.u32(holder);
            index = index.wrapping_add(1);
        }
        let mut index = 0u32;
        while index < e.mem.u32(light.wrapping_add(LIGHT_PORTALS_COUNT)) {
            let portals = e.mem.u32(light.wrapping_add(LIGHT_PORTALS));
            let portal = e.mem.u32(portals.wrapping_add(index.wrapping_mul(4)));
            if portal != 0 {
                let object = e.mem.u32(portal.wrapping_add(PORTAL_OBJECT));
                let array = object.wrapping_add(OBJECT_LIGHTS);
                let count = e.mem.u32(array.wrapping_add(8));
                if count != 0 {
                    let buffer = e.mem.u32(array.wrapping_add(4));
                    let found = (0..count).find(|&slot| {
                        buffer != 0 && e.mem.u32(buffer.wrapping_add(slot.wrapping_mul(4))) == light
                    });
                    if let Some(slot) = found {
                        typed_call!(
                            FAMILY,
                            0x00b5_ade0,
                            lists::remove_at(e, Ptr::new(array), slot, 1u8)
                        );
                    }
                }
            }
            light = e.mem.u32(holder);
            index = index.wrapping_add(1);
        }
    }
    let light = e.mem.u32(holder);
    let list = list_of(e, node, light);
    remove_into_word(e, LIST_REMOVE_POS, list, position, position);
}

// Typed from 00b5ef90
/// `ShadowSceneNode::RemoveQueuedLight` (Xbox PDB, the overload of one word, `thiscall(node,
/// pointer)`, `ret 4`). A light of another portal graph (its +0x240) is removed from its list by
/// `Remove` (the out `NiPointer` the argument's own word) and let go after. A light of this graph
/// is looked up in its list and, when found, removed by `RemoveQueuedLight` (`00b5d040`) with the
/// item as the position.
pub fn remove_queued_light_of(e: &mut Engine, this: Ptr, pointer: Ptr) {
    let node = this.addr();
    let holder = pointer.addr();
    let light = e.mem.u32(holder);
    let list = list_of(e, node, light);
    let graph = e.mem.u32(light.wrapping_add(lists::LIGHT_PORTAL_GRAPH));
    if graph != e.mem.u32(node.wrapping_add(PORTAL_GRAPH)) {
        remove_into_word(e, LIST_REMOVE, list, holder, holder);
        return;
    }
    let mut item = e.mem.u32(list);
    while item != 0 && element_of(e, item) != light {
        item = next_of(e, item);
    }
    if item != 0 {
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item);
            typed_call!(
                FAMILY,
                0x00b5_d040,
                remove_queued_light(e, this, pointer, slot.addr())
            );
        });
    }
}

// ---------------------------------------------------------------------------------------------
// 00b5b4d0: the light value at a point.

/// `FLD a; FLD b; FCOM; TEST AH,5; JP` of two loaded floats: `a` when `b < a`, else `b` (a NaN
/// too), as the loaded value's bits (a signalling NaN quiet).
fn larger(a: u32, b: u32) -> u32 {
    let kept = if f32::from_bits(b) < f32::from_bits(a) {
        a
    } else {
        b
    };
    x87_quiet_bits(kept)
}

/// The largest component of a colour (three floats): `(g, b)`'s larger, then `r`'s against it.
fn colour_peak(e: &Engine, colour: u32) -> u32 {
    let [r, g, b] = three(e, colour);
    larger(r, larger(g, b))
}

// Typed from 00b5b4d0
/// The light value at a point (`thiscall(node, x, y, z, count, ambient, diffuse, excluded)`, `ret
/// 0x1c`; from `CalculateLightValue`): each light of `lLightList` adds its luminance at the point
/// (`00b9bc50`, the excluded `NiLight` passed on) to a running `float`, the count going up after
/// each light that leaves it above 0.0; the sun light's `NiLight` gives the largest component of
/// its ambient and of its diffuse colour. Answers the sum.
#[allow(clippy::too_many_arguments)]
pub fn light_value_at(
    e: &mut Engine,
    this: Ptr,
    x: u32,
    y: u32,
    z: u32,
    count: Ptr,
    ambient: Ptr,
    diffuse: Ptr,
    excluded: u32,
) -> f32 {
    let sum = if float_precision() {
        light_sum::<f32>(e, this.addr(), [x, y, z], count.addr(), excluded)
    } else {
        light_sum::<f64>(e, this.addr(), [x, y, z], count.addr(), excluded)
    };
    let sun = e.mem.u32(this.addr().wrapping_add(SUN_LIGHT));
    let ni_light = ni_light_of(e, sun);
    let peak = colour_peak(e, ni_light.wrapping_add(LIGHT_AMBIENT));
    e.mem.set_u32(ambient.addr(), peak);
    let peak = colour_peak(e, ni_light.wrapping_add(LIGHT_DIFFUSE));
    e.mem.set_u32(diffuse.addr(), peak);
    f32::from_bits(x87_quiet_bits(sum))
}

fn light_sum<R: Real>(e: &mut Engine, node: u32, at: [u32; 3], count: u32, excluded: u32) -> u32 {
    let [x, y, z] = at;
    let mut sum = R::ZERO.store();
    let mut item = head_of(e, node, LIGHT_LIST);
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light == 0 {
            continue;
        }
        let value = typed_call!(
            lights::FAMILY,
            0x00b9_bc50,
            scene_light::luminance_at(e, Ptr::new(light), x, y, z, excluded)
        );
        sum = (R::load(value.to_bits()) + R::load(sum)).store();
        if R::ZERO < R::load(sum) {
            let n = e.mem.u32(count).wrapping_add(1);
            e.mem.set_u32(count, n);
        }
    }
    sum
}

// ---------------------------------------------------------------------------------------------
// 00b5b880: the dynamic shadows.

/// The thread's word +0x2b4 (0x20 meanwhile); `BSCubeMapCamera`'s constructor (`00bd8290`, `ret
/// 4`, 0x140 bytes), its word +0x114 and its held object (+0x130); the current shadow light
/// (`011f9174`); the rendering system (`01200088`) and its thread stage call (`00ba3130`, `ret 8`:
/// 1 and the stage; unnamed beside `MTRenderingSystem::SetThreadStage`, Xbox PDB).
const TLS_WORD: u32 = 0x2b4;
const CUBE_CAMERA_CONSTRUCT: u32 = 0x00bd_8290;
const CUBE_CAMERA_SIZE: u32 = 0x140;
const CUBE_CAMERA_WORD: u32 = 0x114;
const CUBE_CAMERA_HELD: u32 = 0x130;
const CURRENT_SHADOW_LIGHT: u32 = 0x011f_9174;
const RENDERING_SYSTEM: u32 = 0x0120_0088;
const RENDERING_SYSTEM_STAGE: u32 = 0x00ba_3130;

// Typed from 00b5b880
/// `ShadowSceneNode::UpdateDynamicShadows` (Xbox PDB, `thiscall(node, word)`, `ret 4`): with the
/// thread's word +0x2b4 at 0x20 meanwhile: a cube map camera made when there is none (a new
/// `BSCubeMapCamera`, null when the allocation fails, assigned to `spCubeMapCamera`), its word
/// +0x114 cleared. With shadow lights: each with an `NiLight`: a culled `NiLight` (flag 1) gets
/// the light's cull (`00b9ef30`); otherwise the `NiLight` is updated (zero data) and, unless
/// `usFrustumCull` is 0xff, the light becomes the current shadow light and draws its depth map
/// (`MTRenderFrustumDepthMap` with the word and the running count). Then the rendering system's
/// `00ba3130` (1, index) for the indices from count + 0x11 up to 0x14. The camera's held object
/// let go and cleared.
pub fn update_dynamic_shadows(e: &mut Engine, this: Ptr, word: u32) {
    let node = this.addr();
    let tls = e.tls();
    let saved = e.mem.u32(tls + TLS_WORD);
    e.mem.set_u32(tls + TLS_WORD, 0x20);
    if e.mem.u32(node.wrapping_add(CUBE_MAP_CAMERA)) == 0 {
        let memory = e.call(NEW_OBJECT, &args![CUBE_CAMERA_SIZE]).u32();
        let camera = if memory != 0 {
            e.call(CUBE_CAMERA_CONSTRUCT, &args![memory, 0u32]).u32()
        } else {
            0
        };
        assign(e, node.wrapping_add(CUBE_MAP_CAMERA), camera);
    }
    let camera = e.mem.u32(node.wrapping_add(CUBE_MAP_CAMERA));
    e.mem.set_u32(camera.wrapping_add(CUBE_CAMERA_WORD), 0);
    let list = node.wrapping_add(SHADOW_LIGHT_LIST);
    if e.mem.u32(list.wrapping_add(8)) != 0 {
        let mut drawn = 0u32;
        let mut item = e.mem.u32(list);
        while item != 0 {
            let light = element_of(e, item);
            item = next_of(e, item);
            if light == 0 {
                continue;
            }
            let ni_light = ni_light_of(e, light);
            if ni_light == 0 {
                continue;
            }
            if e.mem.u32(ni_light.wrapping_add(AV_FLAGS)) & 1 != 0 {
                e.call(LIGHT_CULL, &args![light]);
                continue;
            }
            e.with_stack(0xc, |e, data| {
                e.mem.set_u32(data.addr(), 0);
                for byte in 4..9u32 {
                    e.mem.set_u8(data.addr() + byte, 0);
                }
                typed_call!(
                    scene::FAMILY,
                    0x00a5_9c60,
                    scene::walk::update(e, Ptr::new(ni_light), data.addr())
                );
            });
            if e.mem.u16(light.wrapping_add(FRUSTUM_CULL)) != 0xff {
                e.mem.set_u32(CURRENT_SHADOW_LIGHT, light);
                typed_call!(
                    lights::FAMILY,
                    0x00b9_f780,
                    scene_frustum::render_frustum_depth_map(
                        e,
                        Ptr::<ShadowSceneLight>::new(light),
                        word,
                        drawn
                    )
                );
                drawn = drawn.wrapping_add(1);
            }
        }
        // `LEA ESI,[EBP + 0x11]; CMP ESI,0x15; JGE`: a signed index.
        let mut index = drawn.wrapping_add(0x11) as i32;
        while index < 0x15 {
            e.call(
                RENDERING_SYSTEM_STAGE,
                &args![RENDERING_SYSTEM, 1u32, index as u32],
            );
            index += 1;
        }
    }
    let camera = e.mem.u32(node.wrapping_add(CUBE_MAP_CAMERA));
    let slot = camera.wrapping_add(CUBE_CAMERA_HELD);
    let held = e.mem.u32(slot);
    if held != 0 {
        release(e, held);
        e.mem.set_u32(slot, 0);
    }
    e.mem.set_u32(tls + TLS_WORD, saved);
}

// ---------------------------------------------------------------------------------------------
// 00b5e5a0, 00b5f980: the destructors.

/// The `ShadowSceneNode` vtable (`010adcf8`); the scene graph's nodes (`011f91c8`, one word per
/// index) and the release of a slot (`00b4f2f0`, `cdecl`: the index, 0); the accumulator's scene
/// node (+0x194); the renderer's reflection map hand-back (`00b6da10`, `ret 4`); the compound
/// frustum's destructor (`00c47770`) and `NiNode`'s (`00a5e930`); the sized free (`00aa1460`,
/// `cdecl`: the block, 0x200).
const SHADOW_SCENE_NODE_VTABLE: u32 = 0x010a_dcf8;
const SCENE_GRAPH_NODES: u32 = 0x011f_91c8;
const SCENE_GRAPH_RELEASE: u32 = 0x00b4_f2f0;
const ACCUMULATOR_SCENE_NODE: u32 = 0x194;
const RENDERER_RELEASE_REFLECTION: u32 = 0x00b6_da10;
const COMPOUND_FRUSTUM_DESTRUCT: u32 = 0x00c4_7770;
const NI_NODE_DESTRUCT: u32 = 0x00a5_e930;
const SIZED_FREE: u32 = 0x00aa_1460;
const SHADOW_SCENE_NODE_SIZE: u32 = 0x200;

/// The shader accumulator (`BSShaderManager::GetAccumulator`, the render family's).
fn accumulator(e: &mut Engine) -> u32 {
    typed_call!(
        render::FAMILY,
        0x00b4_f5c0,
        render::passes::shader_accumulator(e)
    )
}

/// An `NiPointer` member let go and cleared (nothing when null).
fn clear_member(e: &mut Engine, slot: u32) {
    let held = e.mem.u32(slot);
    if held != 0 {
        release(e, held);
        e.mem.set_u32(slot, 0);
    }
}

/// An `NiPointer` member's destructor: the object let go, the word kept.
fn destroy_member(e: &mut Engine, slot: u32) {
    let held = e.mem.u32(slot);
    if held != 0 {
        release(e, held);
    }
}

/// The counted list's `RemoveAll` on the node's list at `offset`.
fn empty_list(e: &mut Engine, node: u32, offset: u32) {
    typed_call!(
        FAMILY,
        0x00e7_4d40,
        lists::remove_all(e, Ptr::new(node.wrapping_add(offset)))
    );
}

// Typed from 00b5e5a0
/// The `ShadowSceneNode` destructor (`thiscall(node)`): the vtable set; the scene graph slot
/// released when the node is the one at its index; the accumulator's scene node cleared when it is
/// this node; every light queued for removal (`00b5d180`); the fence objects let go and cleared;
/// the sun light deleted (its slot 0 with 1); the window reflection map handed to the renderer,
/// let go and cleared; the light queues emptied; no portal graph; the cube map camera let go and
/// cleared; the compound frustum destroyed; then the members' destructors in reverse order (the
/// fog, the camera and map pointers, the object and light queues, the fence objects, the shadow
/// light and light lists) and `NiNode`'s.
pub fn destruct(e: &mut Engine, this: Ptr) {
    let node = this.addr();
    e.mem.set_u32(node, SHADOW_SCENE_NODE_VTABLE);
    let index = u32::from(e.mem.u8(node.wrapping_add(SCENE_GRAPH_INDEX)));
    if e.mem
        .u32(SCENE_GRAPH_NODES.wrapping_add(index.wrapping_mul(4)))
        == node
    {
        e.call(SCENE_GRAPH_RELEASE, &args![index, 0u32]);
    }
    // The accumulator asked three times, as the exe calls it.
    if accumulator(e) != 0 {
        let at = accumulator(e).wrapping_add(ACCUMULATOR_SCENE_NODE);
        if e.mem.u32(at) == node {
            let at = accumulator(e).wrapping_add(ACCUMULATOR_SCENE_NODE);
            e.mem.set_u32(at, 0);
        }
    }
    typed_call!(FAMILY, 0x00b5_d180, lists::queue_all_removals(e, this));
    clear_member(e, node.wrapping_add(ACTIVE_FENCE_OBJECT));
    clear_member(e, node.wrapping_add(FADING_FENCE_OBJECT));
    let sun = e.mem.u32(node.wrapping_add(SUN_LIGHT));
    if sun != 0 {
        e.vcall(sun, SLOT_DELETING_DESTRUCTOR, &args![1u32]);
    }
    let map = e.mem.u32(node.wrapping_add(WINDOW_REFLECTION_MAP));
    if map != 0 {
        let renderer = e.mem.u32(RENDERER);
        e.call(RENDERER_RELEASE_REFLECTION, &args![renderer, map]);
    }
    clear_member(e, node.wrapping_add(WINDOW_REFLECTION_MAP));
    for list in [LIGHT_QUEUE_ADD, LIGHT_QUEUE_REMOVE, LIGHT_QUEUE_UPDATE] {
        empty_list(e, node, list);
    }
    e.mem.set_u32(node.wrapping_add(PORTAL_GRAPH), 0);
    clear_member(e, node.wrapping_add(CUBE_MAP_CAMERA));
    e.call(
        COMPOUND_FRUSTUM_DESTRUCT,
        &args![node.wrapping_add(COMPOUND_FRUSTUM)],
    );
    for member in [FOG, CUBE_MAP_CAMERA, WINDOW_REFLECTION_MAP] {
        destroy_member(e, node.wrapping_add(member));
    }
    for list in [
        OBJECT_QUEUE_UPDATE_ACTIVE_ONLY,
        OBJECT_QUEUE_UPDATE,
        LIGHT_QUEUE_UPDATE,
        LIGHT_QUEUE_REMOVE,
        LIGHT_QUEUE_ADD,
    ] {
        empty_list(e, node, list);
    }
    for member in [FADING_FENCE_OBJECT, ACTIVE_FENCE_OBJECT] {
        destroy_member(e, node.wrapping_add(member));
    }
    empty_list(e, node, SHADOW_LIGHT_LIST);
    empty_list(e, node, LIGHT_LIST);
    e.call(NI_NODE_DESTRUCT, &args![node]);
}

// Typed from 00b5f980
/// The `ShadowSceneNode` deleting destructor (`thiscall(node, flags)`, `ret 4`): the destructor,
/// then the 0x200-byte block freed when bit 0 is set; answers the node.
pub fn delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x00b5_e5a0, destruct(e, this));
    if flags & 1 != 0 {
        e.call(SIZED_FREE, &args![this.addr(), SHADOW_SCENE_NODE_SIZE]);
    }
    this
}
