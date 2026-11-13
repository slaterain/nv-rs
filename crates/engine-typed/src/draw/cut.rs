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
    capture::take(e, NULL_BOUND, BOUND.0, BOUND.1);
    capture::take(e, FOG_INDEX, 0, 1);
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
    let data = packet::model_data(e, geometry);
    if data != 0
        && crate::slot_target(e, geometry, SLOT_RENDER_IMMEDIATE) == PARTICLES_RENDER_IMMEDIATE
    {
        take_particles(e, data);
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

/// A pass's lights: their count (+9) and array (+0xc), and each light's values, once a light.
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
    capture::take(e, lights, 0, u32::from(count).wrapping_mul(4));
    for i in 0..u32::from(count) {
        let Some(shadow) = peek(e, lights.wrapping_add(i * 4)) else {
            return;
        };
        if shadow == 0 || !nodes.insert(shadow) {
            continue;
        }
        capture::take(e, shadow, SHADOW_LIGHT.0, SHADOW_LIGHT.1);
        capture::take(e, shadow, SHADOW_LIGHT_FLAGS.0, SHADOW_LIGHT_FLAGS.1);
        let (Some(light), Some(point)) = (
            peek(e, shadow.wrapping_add(0xf8)),
            peek_u8(e, shadow.wrapping_add(0xf4)),
        ) else {
            continue;
        };
        if light != 0 && nodes.insert(light) {
            capture::take(e, light, WORLD.0, WORLD.1);
            capture::take(e, light, LIGHT_VALUES.0, LIGHT_VALUES.1);
            // A direction only for a light that is not a point light (the flag +0xf4), as the set-up reads it.
            if point == 0 {
                capture::take(e, light, LIGHT_DIRECTION.0, LIGHT_DIRECTION.1);
            }
        }
    }
}

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
}
