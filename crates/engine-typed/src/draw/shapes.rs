//! The geometries' immediate draws and the geometry data getters the draws ask.
//!
//! The batch renderer's standard draw hands each plain geometry to its vtable slot 0xe0
//! (`OnlyRenderImmediate`, Xbox PDB): the triangle shape's ([`tri_shape_render_immediate`]), the
//! segmented shape's, which first merges its enabled segments ([`segmented_render_immediate`]), and
//! the particles' ([`particles_render_immediate`]); the triangle strips' is the dispatch family's. Each
//! is `NiGeometry::RenderImmediate` (the dispatch family's `00a80610`) and, while the renderer draws
//! immediately, the renderer's draw for the geometry's kind: slot 0x1b4 the shape draw with its partial
//! records, 0x1bc the camera-facing points ([`super::packed`]).
//!
//! The data getters (a data's vertex and triangle counts, its strip lists, its RTTI, and the folded
//! "false" a shader declaration answers) are what the packed draws, the stream set-up and the packer
//! ask through a data's or a declaration's vtable, about 2,800 times a frame in the crowded street:
//! [`vertex_count`] and its siblings call them directly when the slot holds them.

use super::layout::{BSSegmentedTriShape, Segment};
use super::{packed, packet, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The renderer's slots for a shape's draw (0x1b4) and for points (0x1bc).
const SLOT_DRAW_SHAPE: u32 = 0x1b4;
const SLOT_DRAW_POINTS: u32 = 0x1bc;

/// Whether the renderer draws now: its draw mode (+0x200) is 1 or 2 and its immediate byte (+0x208)
/// is 1.
#[inline(always)]
fn draws_now(e: &Engine, renderer: u32) -> bool {
    let mode = packet::renderer_draw_mode(e, renderer);
    (mode == 1 || mode == 2) && packet::renderer_immediate(e, renderer) == 1
}

/// `NiGeometry::RenderImmediate` (the dispatch family's `00a80610`): the renderer's property state
/// becomes the geometry's, its controllers' `OnPreDisplay` run.
#[inline(always)]
fn base_render_immediate(e: &mut Engine, this: Ptr, renderer: Ptr) {
    typed_call!(
        crate::dispatch::FAMILY,
        0x00a8_0610,
        crate::dispatch::scene::geometry_render_immediate(e, this, renderer)
    );
}

// Typed from 00a74600
/// `NiTriShape::OnlyRenderImmediate` (Xbox PDB), `thiscall(this, renderer)`: `00a80610`, then while
/// the renderer draws now its slot 0x1b4 with the shape (`NiDX9Renderer`'s shape draw,
/// [`packed::draw_shape`], directly when the slot holds it).
pub fn tri_shape_render_immediate(e: &mut Engine, this: Ptr, renderer: Ptr) {
    base_render_immediate(e, this, renderer);
    if !draws_now(e, renderer.addr()) {
        return;
    }
    if crate::slot_target(e, renderer.addr(), SLOT_DRAW_SHAPE) == 0x00e7_45a0 {
        typed_call!(FAMILY, 0x00e7_45a0, packed::draw_shape(e, renderer, this));
    } else {
        e.vcall(renderer.addr(), SLOT_DRAW_SHAPE, &args![this]);
    }
}

// Typed from 00a9b1f0
/// `NiParticles::RenderImmediate` (Xbox PDB), `thiscall(this, renderer)`: `00a80610`, then while the
/// renderer draws now its slot 0x1bc with the particles (`NiDX9Renderer`'s points draw,
/// [`packed::draw_points`], directly when the slot holds it).
pub fn particles_render_immediate(e: &mut Engine, this: Ptr, renderer: Ptr) {
    base_render_immediate(e, this, renderer);
    if !draws_now(e, renderer.addr()) {
        return;
    }
    if crate::slot_target(e, renderer.addr(), SLOT_DRAW_POINTS) == 0x00e7_0140 {
        typed_call!(FAMILY, 0x00e7_0140, packed::draw_points(e, renderer, this));
    } else {
        e.vcall(renderer.addr(), SLOT_DRAW_POINTS, &args![this]);
    }
}

/// Segment `index` of the shape's array, the array read each time as the exe reads it.
#[inline(always)]
fn segment(e: &Engine, this: Ptr<BSSegmentedTriShape>, index: u32) -> Ptr<Segment> {
    let segments = e.get(this, BSSegmentedTriShape::pSegments);
    Ptr::new(segments.wrapping_add(index.wrapping_mul(0x14)))
}

// Typed from 00a87040
/// `BSSegmentedTriShape::MergeEnabledSegments` (Xbox PDB), `thiscall(this)`: when the segments
/// changed (+0xcc), from the last segment back, each one's state after the merge. The last keeps its
/// own enabled byte and count when it has primitives, else is off. Each earlier one merges with the
/// next (the next off after the merge, this one on, its merged count its own plus the next's, and with
/// no primitives of its own the next's start index) when both are on (the next by its state after the
/// merge) or when it has no primitives and the next is on; otherwise it keeps its own enabled byte and
/// count when it has primitives, or is off. The changed flag is cleared. The segments' enabled bytes
/// are the game's (the packet); what the merge writes is the draw's.
pub fn merge_enabled_segments(e: &mut Engine, this: Ptr<BSSegmentedTriShape>) {
    if e.get(this, BSSegmentedTriShape::bSegmentsChanged) == 0 {
        return;
    }
    let count = e.get(this, BSSegmentedTriShape::iNumSegments);
    let last = segment(e, this, count.wrapping_sub(1));
    let primitives = e.get(last, Segment::iNumPrimitives);
    if primitives == 0 {
        e.set(last, Segment::bEnabledPostMerge, 0);
    } else {
        e.set(last, Segment::iNumPrimitivesMerged, primitives);
        let enabled = e.get(last, Segment::bEnabled);
        e.set(last, Segment::bEnabledPostMerge, enabled);
    }
    let mut index = e
        .get(this, BSSegmentedTriShape::iNumSegments)
        .wrapping_sub(2) as i32;
    while index >= 0 {
        let at = index as u32;
        let current = segment(e, this, at);
        let next = Ptr::<Segment>::new(current.addr().wrapping_add(0x14));
        let enabled = e.get(current, Segment::bEnabled);
        let next_on = e.get(next, Segment::bEnabledPostMerge) != 0;
        let own = e.get(current, Segment::iNumPrimitives);
        if next_on && (enabled != 0 || own == 0) {
            e.set(next, Segment::bEnabledPostMerge, 0);
            let current = segment(e, this, at);
            e.set(current, Segment::bEnabledPostMerge, 1);
            let current = segment(e, this, at);
            let next = Ptr::<Segment>::new(current.addr().wrapping_add(0x14));
            let merged = e
                .get(next, Segment::iNumPrimitivesMerged)
                .wrapping_add(e.get(current, Segment::iNumPrimitives));
            e.set(current, Segment::iNumPrimitivesMerged, merged);
            let current = segment(e, this, at);
            if e.get(current, Segment::iNumPrimitives) == 0 {
                let next = Ptr::<Segment>::new(current.addr().wrapping_add(0x14));
                let start = e.get(next, Segment::iStartIndex);
                e.set(current, Segment::iStartIndex, start);
            }
        } else if own != 0 {
            e.set(current, Segment::bEnabledPostMerge, enabled);
            let current = segment(e, this, at);
            let own = e.get(current, Segment::iNumPrimitives);
            e.set(current, Segment::iNumPrimitivesMerged, own);
        } else {
            e.set(current, Segment::bEnabledPostMerge, 0);
        }
        index -= 1;
    }
    e.set(this, BSSegmentedTriShape::bSegmentsChanged, 0);
}

// Typed from 00a87130
/// `BSSegmentedTriShape::OnlyRenderImmediate` (Xbox PDB), `thiscall(this, renderer)`: the enabled
/// segments merged, then the triangle shape's immediate draw (a jump with the same words).
pub fn segmented_render_immediate(e: &mut Engine, this: Ptr<BSSegmentedTriShape>, renderer: Ptr) {
    merge_enabled_segments(e, this);
    typed_call!(
        FAMILY,
        0x00a7_4600,
        tri_shape_render_immediate(e, Ptr::new(this.addr()), renderer)
    );
}

// ---- The geometry data getters. ----

/// A data's vtable slots: the vertex count (0x90), the triangle count (0xa8), the RTTI (8).
pub(crate) const SLOT_VERTEX_COUNT: u32 = 0x90;
pub(crate) const SLOT_TRIANGLE_COUNT: u32 = 0xa8;
pub(crate) const SLOT_RTTI: u32 = 8;

// Typed from 00a669e0
/// `NiGeometryData::GetActiveVertexCount` (Xbox PDB), `thiscall(this)`: the vertex count (+8).
pub fn active_vertex_count(e: &mut Engine, this: u32) -> u16 {
    packet::data_vertices(e, this)
}

// Typed from 00a923a0
/// `NiParticlesData::GetActiveVertexCount` (Xbox PDB), `thiscall(this)`: the active particle count
/// (+0x50).
pub fn particles_active_vertex_count(e: &mut Engine, this: u32) -> u16 {
    packet::active_particles(e, this)
}

// Typed from 00a75f20
/// `NiTriBasedGeomData::GetActiveTriangleCount` (Xbox PDB), `thiscall(this)`: the triangle count
/// (+0x40).
pub fn active_triangle_count(e: &mut Engine, this: u32) -> u16 {
    packet::data_triangles(e, this)
}

// Typed from 00a923b0
/// `NiTriStripsDynamicData::GetActiveTriangleCount` (Xbox PDB), `thiscall(this)`: the active
/// triangle count (+0x52).
pub fn strips_dynamic_active_triangle_count(e: &mut Engine, this: u32) -> u16 {
    packet::active_triangles(e, this)
}

// Typed from 00a75f10
/// `NiTriStripsData::GetStripLists` (Xbox PDB), `thiscall(this)`: the strips' index array (+0x4c).
pub fn strip_lists(e: &mut Engine, this: u32) -> u32 {
    packet::strip_lists_of(e, this)
}

/// `NiTriStripsData::ms_RTTI` and `NiTriStripsDynamicData::ms_RTTI` (their `NiRTTI` records).
const TRI_STRIPS_DATA_RTTI: u32 = 0x011f_4a70;
const TRI_STRIPS_DYNAMIC_DATA_RTTI: u32 = 0x011f_5e8c;

// Typed from 00a75f30
/// `NiTriStripsData::GetRTTI` (Xbox PDB): the class's `NiRTTI` record (`011f4a70`).
pub fn strips_data_rtti(_e: &mut Engine) -> u32 {
    TRI_STRIPS_DATA_RTTI
}

// Typed from 00a92470
/// `NiTriStripsDynamicData::GetRTTI` (Xbox PDB): the class's `NiRTTI` record (`011f5e8c`).
pub fn strips_dynamic_data_rtti(_e: &mut Engine) -> u32 {
    TRI_STRIPS_DYNAMIC_DATA_RTTI
}

// Typed from 0047c850
/// A folded body many classes' slots and callers share (a shader declaration's slot 0x8c among
/// them): false, `this` not read.
pub fn answers_false(_e: &mut Engine, _this: u32) -> bool {
    false
}

/// A data's vertex count through its slot 0x90: the plain data's (`00a669e0`) or the particles'
/// (`00a923a0`) directly when the slot holds it, any other through the vtable.
#[inline(always)]
pub(crate) fn vertex_count(e: &mut Engine, data: u32) -> u16 {
    match crate::slot_target(e, data, SLOT_VERTEX_COUNT) {
        0x00a6_69e0 => typed_call!(FAMILY, 0x00a6_69e0, active_vertex_count(e, data)),
        0x00a9_23a0 => typed_call!(FAMILY, 0x00a9_23a0, particles_active_vertex_count(e, data)),
        _ => e.vcall(data, SLOT_VERTEX_COUNT, &[]).u16(),
    }
}

/// A data's triangle count through its slot 0xa8: the triangle data's (`00a75f20`) or the dynamic
/// strips' (`00a923b0`) directly when the slot holds it, any other through the vtable. Answers the
/// slot's word.
#[inline(always)]
pub(crate) fn triangle_count(e: &mut Engine, data: u32) -> u16 {
    match crate::slot_target(e, data, SLOT_TRIANGLE_COUNT) {
        0x00a7_5f20 => typed_call!(FAMILY, 0x00a7_5f20, active_triangle_count(e, data)),
        0x00a9_23b0 => typed_call!(
            FAMILY,
            0x00a9_23b0,
            strips_dynamic_active_triangle_count(e, data)
        ),
        _ => e.vcall(data, SLOT_TRIANGLE_COUNT, &[]).u16(),
    }
}

/// A data's RTTI through its slot 8: the strip data's (`00a75f30`) or the dynamic strips'
/// (`00a92470`) directly when the slot holds it, any other through the vtable.
#[inline(always)]
pub(crate) fn rtti_of(e: &mut Engine, data: u32) -> u32 {
    match crate::slot_target(e, data, SLOT_RTTI) {
        0x00a7_5f30 => typed_call!(FAMILY, 0x00a7_5f30, strips_data_rtti(e); words data),
        0x00a9_2470 => typed_call!(FAMILY, 0x00a9_2470, strips_dynamic_data_rtti(e); words data),
        _ => e.vcall(data, SLOT_RTTI, &[]).u32(),
    }
}

/// A word-answering slot with no stack words that the folded "false" (`0047c850`) fills in many
/// classes: directly when the slot holds it, any other target through the vtable.
#[inline(always)]
pub(crate) fn asks(e: &mut Engine, this: u32, slot: u32) -> bool {
    if crate::slot_target(e, this, slot) == 0x0047_c850 {
        typed_call!(FAMILY, 0x0047_c850, answers_false(e, this))
    } else {
        e.vcall(this, slot, &[]).bool()
    }
}
