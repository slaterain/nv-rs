//! The cut (step 2, piece 1: `measure/scope-1111/PLAN.md`): as the world draw's first half starts, what the draw
//! will read that the frame's tail may change goes into the value packet ([`super::capture`]).
//!
//! The pass lists the draw walks are walked once here, the way the draw walks them: each batch renderer of the
//! world accumulator, its pass groups (through the pass map), its register group and alpha holders with their own
//! lists and renderers, its two alpha group lists of sorted groups and its grouped alphas. For each pass's
//! geometry the packet takes:
//! - the world transform (`m_kWorld`, +0x68: rotation, translate, scale, 0x34 bytes), which the tail's scene
//!   updates (`00a68bf0`) rewrite;
//! - the world bound it points at (+0x20; the empty bound `011f4288` for none), which `00a7fdd0` rewrites;
//! - with a skin instance (+0xbc), its root's and each bone's world transform (the root +0x10; the bone list
//!   +0x14, as many as the skin data's +0x44 counts);
//! - its shader property's per-frame values (+0x24 to +0x38: the second flags, the alpha and fade alpha, the
//!   distance from the camera at +0x34), which the tail's other accumulations set again;
//! - for particles (a geometry whose immediate draw is `NiParticles`'), its data's active count (+0x50) and the
//!   arrays the quads are built from (positions, colours, sizes and their scales, angles, the extra bytes, the
//!   point indices), for as many points as the data's vertex count slot answers;
//!
//! and the geometry is held (a reference) until the join. For each pass, its lights: the count (+9) and array
//! (+0xc) and each light's values (`ShadowSceneLight` +0xd0 and its flags +0xf4/+0xf5; through its light +0xf8,
//! the light's world transform, its dimmer, colours and radius +0xc4 to +0xe4 and, for a light that is not a point
//! light, its direction +0xf0). For the frame: the camera (+0x68 to its end at +0x114: its world transform, the
//! world-to-camera matrix, the frustum and the port), the accumulator's +0x38 and +0x3a (which main clears after
//! the second half), the fog index (`011f91c4`, written by the tail's culling) with its scene node's fog property
//! (+0x20 to +0x34, +0x60), and the time values the shaders animate with (`011f940c`, four floats, written before
//! the next frame's input; `011f94ac`). The lights and the time values are read by the shader family (value
//! packet B routes those reads).
//!
//! The light lists the set-ups walk are copied too: each drawn lighting property's list (+0x60, its nodes and their
//! lights: the light getters `draw::lights` walk it), the water property's own list (+0x128, its count +0x130), the
//! scene node's and each accumulator's shadow scene node's light (the accumulator's +0x194, the node's +0xe0) and the
//! scene node's camera position (+0x1e4). With the own state, each pass light's clip-plane cache (+0x1a8 to +0x208)
//! is taken as the draw's own: the shadow clip planes' set-ups write it in main's light.

use super::{capture, detach, packet, shapes};
use crate::shader::layout::BSShaderAccumulator;
use engine_core::ptr::Ptr;
use engine_core::{AddrSet, Engine};

/// An object's world transform (`m_kWorld`): rotation, translate, scale.
pub(crate) const WORLD: (u32, u32) = (0x68, 0x9c);
/// A bound: its centre and radius.
pub(crate) const BOUND: (u32, u32) = (0, 0x10);
/// A shader property's per-frame values: the second flags, the alpha, the fade alpha, two words, the distance.
pub(crate) const PROPERTY: (u32, u32) = (0x24, 0x38);
/// The camera (`NiCamera`, 0x114 bytes: its factory `00a71430` allocates that): its world transform and all it
/// adds (the world-to-camera matrix, the frustum and port the getters `004a0d10` and `0045bbe0` point into).
pub(crate) const CAMERA: (u32, u32) = (0x68, 0x114);
/// The accumulator's two bytes main clears after the world draw: the reflection copy (+0x38) and the depth
/// passes (+0x3a).
pub(crate) const ACC_REFLECTION: (u32, u32) = (0x38, 0x39);
pub(crate) const ACC_DEPTH_PASSES: (u32, u32) = (0x3a, 0x3b);
/// The accumulator's draw-time settings the draw reads and never writes (set before the cut by whoever set the
/// accumulator up; copied so that main may change them meanwhile): the radius byte (+0x31), the split, shadow-group
/// and late-alpha bytes and the decal flag (+0x164 to +0x16c), the batch renderers, the cursor arrays' pointers and
/// the renderer count (+0x174 to +0x18c), the shadow scene node (+0x194: the tall grass's light and the shadow projection's
/// test read it), the render mode (+0x19c). The cursor arrays themselves (what +0x17c to
/// +0x188 point to) are written and read by the world draw alone: they stay in guest memory (the depth-of-field
/// re-render `00875e40`, their only other user, waits for the draw: piece 8).
pub(crate) const ACC_SETTINGS: [(u32, u32); 5] = [
    (0x31, 0x32),
    (0x164, 0x16d),
    (0x174, 0x190),
    (0x194, 0x198),
    (0x19c, 0x1a0),
];
/// An accumulator's shadow scene node (+0x194, in [`ACC_SETTINGS`]); the shader accumulator (`011f95ec`); a shadow scene
/// node's camera position (+0x1e4, three floats).
pub(crate) const ACC_SHADOW_NODE: (u32, u32) = (0x194, 0x198);
const SHADER_ACCUMULATOR: u32 = 0x011f_95ec;
pub(crate) const SCENE_CAMERA: (u32, u32) = (0x1e4, 0x1f0);
/// `NiAVObject`'s empty bound (`011f4288`).
const NULL_BOUND: u32 = 0x011f_4288;
/// The fog index byte (`011f91c4`), the shadow scene nodes (`011f91c8`) and a node's fog property (+0x134);
/// the fog property's values the passes read (+0x20 to +0x34, +0x60).
const FOG_INDEX: u32 = 0x011f_91c4;
const SHADOW_SCENE_NODES: u32 = 0x011f_91c8;
const FOG_VALUES: (u32, u32) = (0x20, 0x34);
const FOG_POWER: (u32, u32) = (0x60, 0x64);
/// The shaders' time values: four floats (`011f940c`) and the start time (`011f94ac`).
const GAME_TIMES: u32 = 0x011f_940c;
const START_TIME: u32 = 0x011f_94ac;
/// A geometry's immediate draw (its slot 0xe0): the particles' (`NiParticles::RenderImmediate`, `00a9b1f0`, the four
/// particle classes' vtables) reaches the points draw, which builds quads from the particle arrays.
const SLOT_RENDER_IMMEDIATE: u32 = 0xe0;
const PARTICLES_RENDER_IMMEDIATE: u32 = 0x00a9_b1f0;
/// A light (`ShadowSceneLight`): its weight (+0xd0) and +0xd4, its flags (+0xf4, +0xf5) and its light (+0xf8, set
/// once); the light's dimmer, ambient and diffuse colours and radius (+0xc4 to +0xe4), +0xe4 and +0xe8 (the light
/// set-up `00b70820` reads all of these) and its direction (+0xf0).
const SHADOW_LIGHT: (u32, u32) = (0xd0, 0xd8);
const SHADOW_LIGHT_FLAGS: (u32, u32) = (0xf4, 0xfc);
/// A pass light's matrix (+0x10, the cube projection `00b7b730` multiplies by), its byte +0xed and its six clip
/// planes (+0x144, which `00b7be00` transforms); its clip-plane cache (+0x1a8, made +0x208) is draw-written: piece 4.
const SHADOW_LIGHT_MATRIX: (u32, u32) = (0x10, 0x50);
const SHADOW_LIGHT_ED: (u32, u32) = (0xec, 0xee);
const SHADOW_LIGHT_PLANES: (u32, u32) = (0x144, 0x1a4);
/// A pass light's index (`u16` +0x110: 0xff for none) and its bytes +0xec and +0xed (the first active light's getters read
/// +0x110 and +0xec), its light's flags byte (+0x30: bit 0 app-culled), a lighting property's light list (+0x60).
const SHADOW_LIGHT_INDEX: (u32, u32) = (0x110, 0x112);
const LIGHT_FLAGS: (u32, u32) = (0x30, 0x31);
const PROPERTY_LIGHTS: (u32, u32) = (0x60, 0x64);
const LIGHT_VALUES: (u32, u32) = (0xc4, 0xec);
const LIGHT_DIRECTION: (u32, u32) = (0xf0, 0xfc);

/// Fills the open packet for the world draw of `acc` with `camera`: the frame's values ([`take_frame`]), then each
/// listed pass's lights and geometry, each geometry held. The lists are read through the drawing thread's list
/// view ([`detach`]).
pub fn fill(e: &mut Engine, camera: u32, acc: Ptr<BSShaderAccumulator>) {
    take_frame(e, camera, acc);
    let mut passes = Vec::new();
    let mut renderers = AddrSet::default();
    for i in 0..packet::renderer_count(e, acc) {
        walk_renderer(
            e,
            packet::renderer(e, acc, i).addr(),
            &mut renderers,
            &mut passes,
        );
    }
    // The draw's own state (piece 3), when no listed pass's shader class may reach a translation that touches it.
    let reason = super::statics::decide(e, &passes);
    super::statics::count(reason);
    if reason.is_none() {
        super::statics::take(e, acc);
        for &pass in &passes {
            own_clip_plane_caches(e, pass);
        }
    }
    let mut geometries = AddrSet::default();
    let mut nodes = AddrSet::default();
    for pass in passes {
        take_lights(e, pass, &mut nodes);
        let geometry = packet::pass_geometry(e, pass);
        if geometry != 0 && geometries.insert(geometry) {
            take_geometry(e, geometry, &mut nodes);
            capture::hold(e, geometry);
        }
    }
}

/// The frame's values: the camera, the accumulator's two bytes, the empty bound, the fog index with its fog
/// property, the time values.
pub fn take_frame(e: &Engine, camera: u32, acc: Ptr<BSShaderAccumulator>) {
    capture::take(e, camera, CAMERA.0, CAMERA.1);
    capture::take(e, acc.addr(), ACC_REFLECTION.0, ACC_REFLECTION.1);
    capture::take(e, acc.addr(), ACC_DEPTH_PASSES.0, ACC_DEPTH_PASSES.1);
    for (from, to) in ACC_SETTINGS {
        capture::take(e, acc.addr(), from, to);
    }
    capture::take(e, NULL_BOUND, BOUND.0, BOUND.1);
    capture::take(e, FOG_INDEX, 0, 1);
    // The first scene node's light (+0xe0), the camera light the set-ups read (the node itself, the scene's, is set at
    // load: read in place).
    if let Some(scene) = peek(e, SHADOW_SCENE_NODES).filter(|&n| n != 0) {
        take_node_light(e, scene);
        // Its camera position (+0x1e4, three floats), which lighting 3.0's set-ups read.
        capture::take(e, scene, SCENE_CAMERA.0, SCENE_CAMERA.1);
    }
    // Each accumulator's shadow scene node's light (+0x194, then the node's +0xe0), as the tall grass's light set-up
    // reads it: the drawn accumulator's, and the shader accumulator's (`011f95ec`, the set-ups' `GetAccumulator`)
    // with its +0x194 when it is another.
    take_shadow_node(e, acc.addr());
    if let Some(held) = peek(e, SHADER_ACCUMULATOR).filter(|&a| a != 0 && a != acc.addr()) {
        capture::take(e, held, ACC_SHADOW_NODE.0, ACC_SHADOW_NODE.1);
        take_shadow_node(e, held);
    }
    let index = u32::from(e.global::<u8>(FOG_INDEX));
    let node = e.global::<u32>(SHADOW_SCENE_NODES.wrapping_add(index.wrapping_mul(4)));
    if node != 0 {
        let fog = e.global::<u32>(node.wrapping_add(0x134));
        capture::take(e, fog, FOG_VALUES.0, FOG_VALUES.1);
        capture::take(e, fog, FOG_POWER.0, FOG_POWER.1);
    }
    capture::take(e, GAME_TIMES, 0, 0x10);
    capture::take(e, START_TIME, 0, 4);
}

/// A shadow scene node's light (+0xe0) and that light's values ([`take_light`]).
fn take_node_light(e: &Engine, node: u32) {
    capture::take(e, node, 0xe0, 0xe4);
    if let Some(light) = peek(e, node.wrapping_add(0xe0)) {
        take_light(e, light, &mut AddrSet::default());
    }
}

/// The light of accumulator `acc`'s shadow scene node (+0x194), when it has one.
fn take_shadow_node(e: &Engine, acc: u32) {
    if let Some(node) = peek(e, acc.wrapping_add(ACC_SHADOW_NODE.0)).filter(|&n| n != 0) {
        take_node_light(e, node);
    }
}

/// What the cut takes for the accumulators' shadow scene nodes and the scene's ([`take_frame`]'s part): for the
/// checks of the set-ups that read them.
pub fn take_shadow_nodes(e: &Engine, acc: u32) {
    capture::take(e, acc, ACC_SHADOW_NODE.0, ACC_SHADOW_NODE.1);
    take_shadow_node(e, acc);
    if let Some(scene) = peek(e, SHADOW_SCENE_NODES).filter(|&n| n != 0) {
        take_node_light(e, scene);
        capture::take(e, scene, SCENE_CAMERA.0, SCENE_CAMERA.1);
    }
}

/// What the cut takes for the camera: for the checks of the paths that read it.
pub fn take_camera(e: &Engine, camera: u32) {
    capture::take(e, camera, CAMERA.0, CAMERA.1);
}

/// What the cut takes for one geometry ([`take_geometry`]), not held: for the checks of the paths that draw one.
pub fn take_geometry_values(e: &mut Engine, geometry: u32) {
    take_geometry(e, geometry, &mut AddrSet::default());
}

/// What the cut takes for one pass's lights ([`take_lights`]): for the checks of the paths that set them.
pub fn take_pass_lights(e: &Engine, pass: u32) {
    take_lights(e, pass, &mut AddrSet::default());
}

/// What the cut takes for one pass light ([`take_light`]): for the checks of the paths that read a pass's first light.
pub fn take_one_light(e: &Engine, shadow: u32) {
    take_light(e, shadow, &mut AddrSet::default());
}

/// What the cut takes for one lighting property's light list ([`take_light_list`]) and its shader values: for the
/// checks of the paths that look for its first active light.
pub fn take_property_lights(e: &Engine, property: u32) {
    capture::take(e, property, PROPERTY.0, PROPERTY.1);
    take_light_list(e, property, &mut AddrSet::default());
}

/// Every pass in batch renderer `renderer`'s lists and in its holders' and sorted groups' lists, into `passes`
/// (a renderer once).
fn walk_renderer(e: &Engine, renderer: u32, seen: &mut AddrSet<u32>, passes: &mut Vec<u32>) {
    if renderer == 0 || !seen.insert(renderer) {
        return;
    }
    // The pass groups the pass map names: five lists each.
    let groups = e.global::<u32>(renderer.wrapping_add(0x08));
    let hash_size = e.global::<u32>(renderer.wrapping_add(0x10));
    let buckets = e.global::<u32>(renderer.wrapping_add(0x14));
    if groups != 0 && buckets != 0 {
        for bucket in 0..hash_size {
            let mut item = e.global::<u32>(buckets.wrapping_add(bucket.wrapping_mul(4)));
            while item != 0 {
                let index = e.global::<u32>(item.wrapping_add(8));
                let group = groups.wrapping_add(index.wrapping_mul(0x68));
                for list in 0..5u32 {
                    walk_list(e, group.wrapping_add(list * 0x14), passes);
                }
                item = e.global::<u32>(item);
            }
        }
    }
    // The register group and alpha holders.
    for slot in (0x3cu32..=0x74).step_by(4) {
        walk_holder(
            e,
            e.global::<u32>(renderer.wrapping_add(slot)),
            seen,
            passes,
        );
    }
    // The alpha group lists' sorted groups (each a holder) and the grouped alphas.
    for at in [0x78u32, 0x84] {
        let mut node = detach::word(e, renderer.wrapping_add(at));
        while node != 0 {
            walk_holder(e, e.global::<u32>(node.wrapping_add(8)), seen, passes);
            node = e.global::<u32>(node);
        }
    }
    for at in [0x90u32, 0x94] {
        walk_holder(e, detach::word(e, renderer.wrapping_add(at)), seen, passes);
    }
}

/// A holder's own list (+0xc) and its batch renderer (+8).
fn walk_holder(e: &Engine, holder: u32, seen: &mut AddrSet<u32>, passes: &mut Vec<u32>) {
    if holder == 0 {
        return;
    }
    walk_list(e, holder.wrapping_add(0xc), passes);
    walk_renderer(e, e.global::<u32>(holder.wrapping_add(8)), seen, passes);
}

/// The passes of a pass list (its head +4; each node's next +0, pass +8).
fn walk_list(e: &Engine, list: u32, passes: &mut Vec<u32>) {
    let mut node = detach::word(e, list.wrapping_add(4));
    while node != 0 {
        let pass = e.global::<u32>(node.wrapping_add(8));
        if pass != 0 {
            passes.push(pass);
        }
        node = e.global::<u32>(node);
    }
}

/// A geometry's values: its world transform, its bound, its skin's root and bones, its shader property's values,
/// its particles.
fn take_geometry(e: &mut Engine, geometry: u32, nodes: &mut AddrSet<u32>) {
    capture::take(e, geometry, WORLD.0, WORLD.1);
    let bound = e.global::<u32>(geometry.wrapping_add(0x20));
    capture::take(e, bound, BOUND.0, BOUND.1);
    take_skin(e, packet::skin_instance(e, geometry), nodes);
    let property = packet::shader_property(e, geometry);
    capture::take(e, property, PROPERTY.0, PROPERTY.1);
    if lighting_property(e, property) {
        take_light_list(e, property, nodes);
    }
    let data = packet::model_data(e, geometry);
    if data != 0
        && crate::slot_target(e, geometry, SLOT_RENDER_IMMEDIATE) == PARTICLES_RENDER_IMMEDIATE
    {
        take_particles(e, data);
    }
    take_group_c_arrays(e, geometry, property, data);
}

/// The tall grass shader (`010b8980`), the no-lighting shader (`010bb230`) and the SpeedTree leaf shader
/// (`010b9190`), by vtable (a geometry's shader +0xc0); the SpeedTree leaf property (`010b9700`, made by `00bb3ba0`) is
/// told by its slots ([`leaf_slots`]).
const TALL_GRASS_SHADER: u32 = 0x010b_8980;
const NO_LIGHTING_SHADER: u32 = 0x010b_b230;
const LEAF_SHADER: u32 = 0x010b_9190;
/// The water shader (`010af9e0`, its slot 0x7c `00b864d0`), whose set-up walks the water property's own light list.
const WATER_SHADER: u32 = 0x010a_f9e0;
/// A geometry's shader.
const GEOMETRY_SHADER: u32 = 0xc0;

/// Whether `property`'s slots 0xfc and 0x100 are the SpeedTree leaf property's (`00bb3c80`, `00bb3c90`: its leaf data
/// and the leaves `[[+0x88]+8]`), as the reader tests them: an override in those slots is respected.
fn leaf_slots(e: &Engine, property: u32) -> bool {
    let Some(vtable) = peek(e, property) else {
        return false;
    };
    peek(e, vtable.wrapping_add(0xfc)) == Some(0x00bb_3c80)
        && peek(e, vtable.wrapping_add(0x100)) == Some(0x00bb_3c90)
}

/// The arrays Group C's set-ups copy into their constants whole (`_memcpy`), by the geometry's shader:
/// - tall grass (`00baaf40`): the property's instance data (+0x7c): its counts (`u16` +0x10, +0x12), its array
///   (+0x14) and that array's count x 0x10 bytes;
/// - no-lighting (`00bc6fa0`): the model data's sub-texture array (+0x68) and count (`u8` +0x6c), count x 0x10 bytes;
/// - SpeedTree leaves (`00bb2760`): the leaf property's slot 0x100 answer (`00bb3c90`: its leaf data +0x88, then
///   that data's +8, null without data), 0x300 bytes; read here without the call, for that class only.
///
/// Read without a stop at an unmapped word, as the light walk.
fn take_group_c_arrays(e: &Engine, geometry: u32, property: u32, data: u32) {
    let Some(shader) = peek(e, geometry.wrapping_add(GEOMETRY_SHADER)) else {
        return;
    };
    if shader == 0 {
        return;
    }
    match peek(e, shader) {
        Some(TALL_GRASS_SHADER) if property != 0 => {
            capture::take(e, property, 0x7c, 0x80);
            let Some(instances) = peek(e, property.wrapping_add(0x7c)) else {
                return;
            };
            if instances == 0 {
                return;
            }
            capture::take(e, instances, 0x10, 0x18);
            let (Some(counts), Some(array)) = (
                peek(e, instances.wrapping_add(0x10)),
                peek(e, instances.wrapping_add(0x14)),
            ) else {
                return;
            };
            capture::take(e, array, 0, (counts & 0xffff) << 4);
        }
        Some(NO_LIGHTING_SHADER) if data != 0 => {
            capture::take(e, data, 0x68, 0x6d);
            let (Some(array), Some(count)) = (
                peek(e, data.wrapping_add(0x68)),
                peek_u8(e, data.wrapping_add(0x6c)),
            ) else {
                return;
            };
            capture::take(e, array, 0, u32::from(count) << 4);
        }
        Some(WATER_SHADER) if property != 0 => take_water_lights(e, property),
        Some(LEAF_SHADER) if property != 0 && leaf_slots(e, property) => {
            capture::take(e, property, 0x88, 0x8c);
            let Some(leaf_data) = peek(e, property.wrapping_add(0x88)) else {
                return;
            };
            if leaf_data == 0 {
                return;
            }
            capture::take(e, leaf_data, 8, 0xc);
            if let Some(leaves) = peek(e, leaf_data.wrapping_add(8)) {
                capture::take(e, leaves, 0, 0x300);
            }
        }
        _ => {}
    }
}

/// A skin instance's root's (+0x10) and bones' world transforms (the bone list +0x14, as many as its data +8
/// counts at +0x44).
fn take_skin(e: &Engine, skin: u32, nodes: &mut AddrSet<u32>) {
    if skin == 0 {
        return;
    }
    take_node(e, e.global::<u32>(skin.wrapping_add(0x10)), nodes);
    let data = e.global::<u32>(skin.wrapping_add(8));
    let bones = e.global::<u32>(skin.wrapping_add(0x14));
    if data != 0 && bones != 0 {
        for bone in 0..e.global::<u32>(data.wrapping_add(0x44)) {
            take_node(
                e,
                e.global::<u32>(bones.wrapping_add(bone.wrapping_mul(4))),
                nodes,
            );
        }
    }
}

/// What the cut takes for one skin instance ([`take_skin`]): for the checks of the bone fill.
pub fn take_skin_values(e: &Engine, skin: u32) {
    take_skin(e, skin, &mut AddrSet::default());
}

/// A node's world transform, once.
fn take_node(e: &Engine, node: u32, nodes: &mut AddrSet<u32>) {
    if node != 0 && nodes.insert(node) {
        capture::take(e, node, WORLD.0, WORLD.1);
    }
}

/// A particle data's active count and, for as many points as the draw will take (the data's vertex count slot,
/// as the draw asks it), its arrays.
fn take_particles(e: &mut Engine, data: u32) {
    capture::take(e, data, 0x50, 0x52);
    let count = u32::from(shapes::vertex_count(e, data));
    let arrays = packet::particle_arrays(e, data);
    for (array, size) in [
        (arrays.positions, 0xc),
        (arrays.colors, 0x10),
        (arrays.sizes, 4),
        (arrays.scales, 4),
        (arrays.angles, 4),
        (arrays.bytes, 1),
        (packet::point_indices(e, data), 2),
    ] {
        capture::take(e, array, 0, count.wrapping_mul(size));
    }
}

/// A pass's lights: their count (+9) and array (+0xc), and each light's values, once a light. At least two entries:
/// the light direction (`00b7c7b0`) reads the first light when the count is 0, and lighting 3.0 the second when it is 1.
fn take_lights(e: &Engine, pass: u32, nodes: &mut AddrSet<u32>) {
    capture::take(e, pass, 9, 0xa);
    capture::take(e, pass, 0xc, 0x10);
    // Read without a stop at an unmapped word: the cut never stops where the draw would not.
    let (Some(count), Some(lights)) = (
        peek_u8(e, pass.wrapping_add(9)),
        peek(e, pass.wrapping_add(0xc)),
    ) else {
        return;
    };
    if lights == 0 {
        return;
    }
    let listed = u32::from(count).max(2);
    capture::take(e, lights, 0, listed.wrapping_mul(4));
    for i in 0..listed {
        let Some(shadow) = peek(e, lights.wrapping_add(i * 4)) else {
            return;
        };
        take_light(e, shadow, nodes);
    }
}

/// The clip-plane caches of a pass's lights (as many as [`take_lights`] reads), taken as the draw's own state: a pass
/// light's six transformed planes and the byte that says they are made (+0x1a8 to +0x208), which the shadow clip
/// planes' set-ups (`00b7be00`; lighting 3.0's `00bbef90` through `00b71ad0`) write in main's light and read back.
/// Only with the rest of the own state (the cut's decision, [`super::statics`]).
pub fn own_clip_plane_caches(e: &Engine, pass: u32) {
    let (Some(count), Some(lights)) = (
        peek_u8(e, pass.wrapping_add(9)),
        peek(e, pass.wrapping_add(0xc)),
    ) else {
        return;
    };
    if lights == 0 {
        return;
    }
    for i in 0..u32::from(count).max(2) {
        match peek(e, lights.wrapping_add(i * 4)) {
            Some(0) => {}
            Some(shadow) => own_clip_plane_cache(e, shadow),
            None => return,
        }
    }
}

/// One pass light's clip-plane cache as the draw's own state ([`own_clip_plane_caches`]): for the checks.
pub fn own_clip_plane_cache(e: &Engine, shadow: u32) {
    capture::own(e, shadow, CLIP_PLANE_CACHE.0, CLIP_PLANE_CACHE.1);
}

/// A pass light's clip-plane cache: six planes (+0x1a8) and the byte that says they are made (+0x208).
const CLIP_PLANE_CACHE: (u32, u32) = (
    crate::shader::geometry::CLIP_PLANE_CACHE,
    crate::shader::geometry::CLIP_PLANE_CACHE + crate::shader::geometry::CLIP_PLANES_MADE + 1,
);

/// A pass light's (`ShadowSceneLight`) values and its light's, once a light (a null light takes nothing).
fn take_light(e: &Engine, shadow: u32, nodes: &mut AddrSet<u32>) {
    if shadow == 0 || !nodes.insert(shadow) {
        return;
    }
    capture::take(e, shadow, SHADOW_LIGHT.0, SHADOW_LIGHT.1);
    capture::take(e, shadow, SHADOW_LIGHT_FLAGS.0, SHADOW_LIGHT_FLAGS.1);
    capture::take(e, shadow, SHADOW_LIGHT_MATRIX.0, SHADOW_LIGHT_MATRIX.1);
    capture::take(e, shadow, SHADOW_LIGHT_ED.0, SHADOW_LIGHT_ED.1);
    capture::take(e, shadow, SHADOW_LIGHT_PLANES.0, SHADOW_LIGHT_PLANES.1);
    capture::take(e, shadow, SHADOW_LIGHT_INDEX.0, SHADOW_LIGHT_INDEX.1);
    let Some(light) = peek(e, shadow.wrapping_add(0xf8)) else {
        return;
    };
    if light != 0 && nodes.insert(light) {
        capture::take(e, light, WORLD.0, WORLD.1);
        capture::take(e, light, LIGHT_VALUES.0, LIGHT_VALUES.1);
        capture::take(e, light, LIGHT_FLAGS.0, LIGHT_FLAGS.1);
        // Its direction too, a point light's included: the light direction (`00b7c7b0`) reads the pass's first light
        // whatever its kind.
        capture::take(e, light, LIGHT_DIRECTION.0, LIGHT_DIRECTION.1);
    }
}

/// The vtables of the lighting property (`010ae7b0`, stored by its constructor `00b71720`) and of every class built
/// on it, found from the exe by the constructors that call it and theirs: `010ae0d0` (`00b66f50`), `010adea8`
/// (`00b60cb0`), `010adad0` (`00b57f90`), `010b95c0` (`00bb3660`), `010b9828` (`00bb4080`); on `00b66f50`:
/// `010b8330` (`00ba8630`), `010b9490` (`00bb3500`), `010b9910` (`00bb4520`), `010babf8` (`00bc3bb0`); on
/// `00bb3660`: `010b9700` (the SpeedTree leaf property, `00bb3ba0`); on `00bb3500`: `010b9338` (`00bb3070`); on
/// `00bb4520`: `010bcb60` (`00c05880`). Only these keep a light list at +0x60: the other shader properties keep
/// something else there (the no-lighting property its base texture, the sky and water values their set-ups read,
/// the particle and bolt properties their instance and segment data).
const LIGHTING_PROPERTY_VTABLES: [u32; 13] = [
    0x010a_e7b0,
    0x010a_e0d0,
    0x010a_dea8,
    0x010a_dad0,
    0x010b_95c0,
    0x010b_9828,
    0x010b_8330,
    0x010b_9490,
    0x010b_9910,
    0x010b_abf8,
    0x010b_9700,
    0x010b_9338,
    0x010b_cb60,
];

/// Whether `property` is a lighting property (its vtable one of [`LIGHTING_PROPERTY_VTABLES`]), the only kind
/// whose +0x60 is a light list.
fn lighting_property(e: &Engine, property: u32) -> bool {
    property != 0 && peek(e, property).is_some_and(|v| LIGHTING_PROPERTY_VTABLES.contains(&v))
}

/// The most nodes of a lighting property's light list the cut walks (a list is a handful of lights).
const LIGHT_LIST_LIMIT: u32 = 64;

/// A lighting property's light list (+0x60): its head, each node's next and light words (+0, +8) and each light's
/// values, as the first active light's getters (`draw::lights`) and the set-ups after them read them.
fn take_light_list(e: &Engine, property: u32, nodes: &mut AddrSet<u32>) {
    if property == 0 {
        return;
    }
    capture::take(e, property, PROPERTY_LIGHTS.0, PROPERTY_LIGHTS.1);
    let mut node = peek(e, property.wrapping_add(PROPERTY_LIGHTS.0)).unwrap_or(0);
    for _ in 0..LIGHT_LIST_LIMIT {
        if node == 0 {
            return;
        }
        capture::take(e, node, 0, 0xc);
        if let Some(light) = peek(e, node.wrapping_add(8)) {
            take_light(e, light, nodes);
        }
        node = peek(e, node).unwrap_or(0);
    }
}

/// The water property's light list (`00b864d0` reads it): its first node (+0x128) and count (+0x130), each node's next
/// and light words (+0, +8) and each light's world transform and values (its position +0x8c, dimmer +0xc4, colour
/// +0xd4 and radius +0xe0: the lights are `NiLight`s, not pass lights).
pub fn take_water_lights(e: &Engine, property: u32) {
    capture::take(e, property, WATER_LIGHTS.0, WATER_LIGHTS.1);
    capture::take(e, property, WATER_LIGHT_COUNT.0, WATER_LIGHT_COUNT.1);
    let mut node = peek(e, property.wrapping_add(WATER_LIGHTS.0)).unwrap_or(0);
    for _ in 0..LIGHT_LIST_LIMIT {
        if node == 0 {
            return;
        }
        capture::take(e, node, 0, 0xc);
        if let Some(light) = peek(e, node.wrapping_add(8)).filter(|&l| l != 0) {
            capture::take(e, light, WORLD.0, WORLD.1);
            capture::take(e, light, LIGHT_VALUES.0, LIGHT_VALUES.1);
        }
        node = peek(e, node).unwrap_or(0);
    }
}

/// The water property's light list head (+0x128) and light count (+0x130).
const WATER_LIGHTS: (u32, u32) = (0x128, 0x12c);
const WATER_LIGHT_COUNT: (u32, u32) = (0x130, 0x134);

/// The word at `addr`, `None` when it is not mapped.
fn peek(e: &Engine, addr: u32) -> Option<u32> {
    let mut b = [0u8; 4];
    e.mem
        .try_read(addr, &mut b)
        .ok()
        .map(|()| u32::from_le_bytes(b))
}

/// The byte at `addr`, `None` when it is not mapped.
fn peek_u8(e: &Engine, addr: u32) -> Option<u8> {
    let mut b = [0u8; 1];
    e.mem.try_read(addr, &mut b).ok().map(|()| b[0])
}

/// The run's packet reports added up ([`join`]): what a POISON run's check reads.
static TOTALS: std::sync::Mutex<capture::Report> = std::sync::Mutex::new(capture::Report {
    misses: 0,
    reads: 0,
    written: 0,
    ranges: 0,
    bytes: 0,
    held: 0,
    owned: 0,
    kept_main: 0,
});

/// The cut, as the world draw's first half starts with a scene renderer and a camera: with `NV_DRAW_THREAD` set
/// the packet is opened and filled ([`fill`]); with `poison`, scribbled. A packet still open (a draw that did not
/// end) is closed first.
pub fn cut(e: &mut Engine, camera: u32, acc: Ptr<BSShaderAccumulator>) {
    let mode = capture::mode();
    if mode == capture::Mode::Off {
        return;
    }
    join(e);
    if !capture::begin(e) {
        return;
    }
    fill(e, camera, acc);
    if mode == capture::Mode::Poison {
        // The two scribbles take turns, cut by cut: a byte the draw writes with one is caught by the other.
        let cuts = CUTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let scribble = if cuts % 2 == 0 {
            capture::SCRIBBLE
        } else {
            capture::SCRIBBLE_OTHER
        };
        capture::poison(e, scribble);
    }
}

/// The cuts made with POISON, for the scribbles' turns.
static CUTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How often (in joins) the run's totals are printed after the first: about every ten seconds of play.
const TOTALS_EVERY: u64 = 600;

/// The joins made, for [`TOTALS_EVERY`].
static JOINS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The join, as the world draw's second half ends: the packet closed ([`capture::end`]) and its report added to
/// the run's [`totals`] and kept as this thread's [`last_report`]. Nothing without an open packet.
pub fn join(e: &mut Engine) {
    if !capture::is_open(e) {
        return;
    }
    let report = capture::end(e);
    LAST.with(|last| last.set(Some(report)));
    let mut totals = TOTALS.lock().unwrap_or_else(|p| p.into_inner());
    totals.misses += report.misses;
    totals.reads += report.reads;
    totals.written += report.written;
    totals.ranges += report.ranges;
    totals.bytes += report.bytes;
    totals.held += report.held;
    totals.owned += report.owned;
    totals.kept_main += report.kept_main;
    let joins = JOINS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if joins % TOTALS_EVERY == 0 {
        // With the switch on, the run's coverage: a miss is a read the cut left out (the first few are printed
        // as they happen).
        eprintln!(
            "nv-draw packet: {} joins: misses {}, reads {}, written {}, ranges {}, bytes {}, held {}, owned {}, kept main {}",
            joins + 1,
            totals.misses,
            totals.reads,
            totals.written,
            totals.ranges,
            totals.bytes,
            totals.held,
            totals.owned,
            totals.kept_main
        );
        let (owned, kept, reason) = super::statics::report();
        eprintln!(
            "nv-draw own state: {owned} cuts took it, {kept} kept it in guest memory (first reason {reason:#010x})"
        );
    }
}

thread_local! {
    static LAST: std::cell::Cell<Option<capture::Report>> = const { std::cell::Cell::new(None) };
}

/// The report of the last packet [`join`] closed on this thread, taken (`None` after).
pub fn last_report() -> Option<capture::Report> {
    LAST.with(std::cell::Cell::take)
}

/// The reports of every packet closed so far, added up.
pub fn totals() -> capture::Report {
    *TOTALS.lock().unwrap_or_else(|p| p.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pass whose light count is 0 still has its first two lights copied (the light direction reads the first,
    /// lighting 3.0 the second): their +0xd4, matrix, planes and byte +0xed, and the array's two words.
    #[test]
    fn a_pass_without_lights_still_gives_its_first_two() {
        let mut e = Engine::new();
        let pass = e.mem.alloc(0x10);
        let lights = e.mem.alloc(8);
        let shadows = [e.mem.alloc(0x200), e.mem.alloc(0x200)];
        e.mem.set_u32(pass + 0xc, lights);
        for (k, shadow) in (0u32..).zip(shadows) {
            e.mem.set_u32(lights + 4 * k, shadow);
            e.mem.set_u32(shadow + 0xd4, 0x40 + k);
            e.mem.set_u32(shadow + 0x10, 0x50 + k);
            e.mem.set_u32(shadow + 0x144, 0x60 + k);
            e.mem.set_u8(shadow + 0xed, 1);
        }
        assert!(capture::begin(&e));
        take_pass_lights(&e, pass);
        let read: Vec<u32> = shadows
            .iter()
            .flat_map(|&s| {
                [
                    capture::word(&e, s, 0xd4),
                    capture::word(&e, s, 0x10),
                    capture::word(&e, s, 0x144),
                    u32::from(capture::byte(&e, s, 0xed)),
                ]
            })
            .chain([capture::word(&e, lights, 0), capture::word(&e, lights, 4)])
            .collect();
        let report = capture::end(&mut e);
        assert_eq!(
            read,
            [0x40, 0x50, 0x60, 1, 0x41, 0x51, 0x61, 1, shadows[0], shadows[1]]
        );
        assert_eq!(report.misses, 0);
    }

    /// The light set-up (`00b70820`) reads a pass light's +0xd4 and its light's +0xe4 and +0xe8: the cut copies
    /// them, so a read of them from the packet misses nothing.
    #[test]
    fn a_pass_lights_values_the_set_up_reads_are_copied() {
        let mut e = Engine::new();
        let pass = e.mem.alloc(0x10);
        let lights = e.mem.alloc(4);
        let shadow = e.mem.alloc(0x100);
        let light = e.mem.alloc(0x100);
        e.mem.set_u8(pass + 9, 1);
        e.mem.set_u32(pass + 0xc, lights);
        e.mem.set_u32(lights, shadow);
        e.mem.set_u32(shadow + 0xf8, light);
        e.mem.set_u8(shadow + 0xf4, 1);
        e.mem.set_u32(shadow + 0xd4, 0x11);
        e.mem.set_u32(light + 0xe4, 0x22);
        e.mem.set_u32(light + 0xe8, 0x33);
        assert!(capture::begin(&e));
        take_pass_lights(&e, pass);
        // Main changes them after the cut: the draw still reads the cut's values.
        e.mem.set_u32(shadow + 0xd4, 0);
        e.mem.set_u32(light + 0xe4, 0);
        e.mem.set_u32(light + 0xe8, 0);
        let read = [
            capture::word(&e, shadow, 0xd0),
            capture::word(&e, shadow, 0xd4),
            capture::word(&e, light, 0xe0),
            capture::word(&e, light, 0xe4),
            capture::word(&e, light, 0xe8),
        ];
        let report = capture::end(&mut e);
        assert_eq!(read, [0, 0x11, 0, 0x22, 0x33]);
        assert_eq!(report.misses, 0);
    }

    /// A geometry (with a vtable, a bound, its shader property and model data) whose shader has `vtable`:
    /// (geometry, property, model data).
    fn group_c_geometry(e: &mut Engine, vtable: u32) -> (u32, u32, u32) {
        let geometry = e.mem.alloc(0x100);
        let geometry_vtable = e.mem.alloc(0x100);
        let shader = e.mem.alloc(0x10);
        let property = e.mem.alloc(0x100);
        let data = e.mem.alloc(0x100);
        e.mem.set_u32(geometry, geometry_vtable);
        let bound = e.mem.alloc(0x10);
        e.mem.set_u32(geometry + 0x20, bound);
        e.mem.set_u32(geometry + 0xa8, property);
        e.mem.set_u32(geometry + 0xb8, data);
        e.mem.set_u32(geometry + GEOMETRY_SHADER, shader);
        e.mem.set_u32(shader, vtable);
        (geometry, property, data)
    }

    /// An array of `words` words holding 0x100, 0x101, ..
    fn words(e: &mut Engine, words: u32) -> u32 {
        let at = e.mem.alloc(4 * words);
        for i in 0..words {
            e.mem.set_u32(at + 4 * i, 0x100 + i);
        }
        at
    }

    /// The words of `[at, at + 4 * n)` read from the open packet after main cleared them in guest memory, and
    /// the packet's misses.
    fn read_after_clear(e: &mut Engine, at: u32, n: u32) -> (Vec<u32>, u64) {
        for i in 0..n {
            e.mem.set_u32(at + 4 * i, 0);
        }
        let read = (0..n).map(|i| capture::word(e, at, 4 * i)).collect();
        (read, capture::end(e).misses)
    }

    /// The tall grass's instance copy (`00baaf40`) reads the instance data's counts and array and the array's
    /// count x 0x10 bytes: the cut copies them all.
    #[test]
    fn the_grass_instances_are_copied() {
        let mut e = Engine::new();
        let (geometry, property, _) = group_c_geometry(&mut e, TALL_GRASS_SHADER);
        let instances = e.mem.alloc(0x40);
        let array = words(&mut e, 12);
        e.mem.set_u32(property + 0x7c, instances);
        e.mem.set_u32(instances + 0x10, 0x0002_0003);
        e.mem.set_u32(instances + 0x14, array);
        assert!(capture::begin(&e));
        take_geometry_values(&mut e, geometry);
        e.mem.set_u32(instances + 0x10, 0);
        let counts = capture::word(&e, instances, 0x10);
        let (read, misses) = read_after_clear(&mut e, array, 12);
        assert_eq!(counts, 0x0002_0003);
        assert_eq!(read, (0x100..0x10c).collect::<Vec<u32>>());
        assert_eq!(misses, 0);
    }

    /// The no-lighting shader's sub-texture copy (`00bc6fa0`): the model data's array and count, count x 0x10.
    #[test]
    fn the_no_lighting_subtextures_are_copied() {
        let mut e = Engine::new();
        let (geometry, _, data) = group_c_geometry(&mut e, NO_LIGHTING_SHADER);
        let array = words(&mut e, 8);
        e.mem.set_u32(data + 0x68, array);
        e.mem.set_u8(data + 0x6c, 2);
        assert!(capture::begin(&e));
        take_geometry_values(&mut e, geometry);
        let (read, misses) = read_after_clear(&mut e, array, 8);
        assert_eq!(read, (0x100..0x108).collect::<Vec<u32>>());
        assert_eq!(misses, 0);
    }

    /// A lighting property's light list (+0x60) is copied with its nodes and lights; another property's +0x60 (the
    /// no-lighting property's base texture here) is left alone: nothing of it, or of what it points to, is taken.
    #[test]
    fn only_a_lighting_propertys_light_list_is_copied() {
        let mut e = Engine::new();
        let (geometry, property, _) = group_c_geometry(&mut e, 0);
        let node = e.mem.alloc(0xc);
        let shadow = e.mem.alloc(0x200);
        let light = e.mem.alloc(0x100);
        e.mem.set_u32(property, 0x010a_e0d0);
        e.mem.set_u32(property + 0x60, node);
        e.mem.set_u32(node + 8, shadow);
        e.mem.set_u32(shadow + 0xf8, light);
        e.mem.set_u16(shadow + 0x110, 2);
        e.mem.set_u8(light + 0x30, 1);
        assert!(capture::begin(&e));
        take_geometry_values(&mut e, geometry);
        e.mem.set_u32(property + 0x60, 0);
        let read = [
            capture::word(&e, property, 0x60),
            capture::word(&e, node, 8),
            capture::word(&e, shadow, 0xf8),
            u32::from(capture::byte(&e, light, 0x30)),
        ];
        assert_eq!(read, [node, shadow, light, 1]);
        assert_eq!(capture::end(&mut e).misses, 0);
        // The no-lighting property (its vtable not a lighting property's): +0x60 holds a texture.
        let (geometry, property, _) = group_c_geometry(&mut e, NO_LIGHTING_SHADER);
        let texture = e.mem.alloc(0x40);
        let texture_vtable = e.mem.alloc(0x40);
        e.mem.set_u32(property, 0x010b_b2b0);
        e.mem.set_u32(property + 0x60, texture);
        e.mem.set_u32(texture, texture_vtable);
        e.mem.set_u32(texture + 8, light);
        assert!(capture::begin(&e));
        take_geometry_values(&mut e, geometry);
        let _ = capture::word(&e, property, 0x60);
        let _ = capture::word(&e, texture, 0);
        assert_eq!(capture::end(&mut e).misses, 2);
    }

    /// The SpeedTree leaves (`00bb2760`): the leaf property's slot 0x100 answer, its leaf data's +8, 0x300 bytes.
    #[test]
    fn the_speedtree_leaves_are_copied() {
        let mut e = Engine::new();
        let (geometry, property, _) = group_c_geometry(&mut e, LEAF_SHADER);
        let leaf_data = e.mem.alloc(0x10);
        let leaves = words(&mut e, 0xc0);
        let vtable = e.mem.alloc(0x104);
        e.mem.set_u32(vtable + 0xfc, 0x00bb_3c80);
        e.mem.set_u32(vtable + 0x100, 0x00bb_3c90);
        e.mem.set_u32(property, vtable);
        e.mem.set_u32(property + 0x88, leaf_data);
        e.mem.set_u32(leaf_data + 8, leaves);
        assert!(capture::begin(&e));
        take_geometry_values(&mut e, geometry);
        let (read, misses) = read_after_clear(&mut e, leaves, 0xc0);
        assert_eq!(read, (0x100..0x1c0).collect::<Vec<u32>>());
        assert_eq!(misses, 0);
        // Another property class: its slot 0x100 is not read without the call, so nothing is taken.
        let (geometry, property, _) = group_c_geometry(&mut e, LEAF_SHADER);
        e.mem.set_u32(property + 0x88, leaf_data);
        assert!(capture::begin(&e));
        take_geometry_values(&mut e, geometry);
        let _ = capture::word(&e, leaves, 0);
        assert_eq!(capture::end(&mut e).misses, 1);
    }
}
