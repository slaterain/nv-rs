//! A face geometry's morphs over its locked vertex array: the base offsets copied in, then each
//! morph added with its weight. The locked array is a strided cursor `{base, stride, packed}` (the
//! geometry data's lock, `00a67410`, fills it); a vertex is three floats at `base + index * stride`.
//!
//! Native floats: every result here is one operation on two floats, stored as a float (the point's
//! scale `0045bb20`, add `0063c8a0` and difference `00439ef0`), so it is plain `f32` on every thread
//! (the same float the exe stores at either precision; see [`super`]).

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The object's geometry data (`005495f0`), called once a morph, whose vertex count the morph reads
/// in place (`004a8ae0`, `AX`: the data's 16-bit word at +8, [`DATA_VERTEX_COUNT`]). The locked
/// array's start into a cursor (`00a67410`: data; cursor).
const OBJECT_GEOMETRY: u32 = 0x0054_95f0;
const GEOMETRY_BEGIN: u32 = 0x00a6_7410;
/// The morph extra data (`NiObjectNET::GetExtraData`, `00a5bdd0`, by address as `00653430` calls
/// it) with the extra data's name (`011d5b8c`), the data's test (`0049ec60`) and `MarkAsChanged`
/// (`00a67090`).
const GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
const EXTRA_DATA_NAME: u32 = 0x011d_5b8c;
const HAS_DATA: u32 = 0x0049_ec60;
const MARK_AS_CHANGED: u32 = 0x00a6_7090;
/// The geometry's data pointer (+0xb8) and the data's 16-bit vertex count (+8): what the face's
/// vertex count (`00456650`) reads, which the base copy asks before every vertex.
const GEOMETRY_DATA: u32 = 0xb8;
const DATA_VERTEX_COUNT: u32 = 0x8;

/// `BSFaceGenMorph::ApplyMorph`'s weight test: above 0.0 and not above 1.0, as the exe compares the
/// float with its doubles (`01012060`, `01012070`; a NaN passes, as there).
#[inline(always)]
#[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN-aware as the exe.
fn weight_in_range(weight: f32) -> bool {
    !(weight <= 0.0) && !(weight > 1.0)
}

/// The point at `at`.
#[inline(always)]
fn point_at(e: &Engine, at: u32) -> [f32; 3] {
    [
        e.mem.f32(at),
        e.mem.f32(at.wrapping_add(4)),
        e.mem.f32(at.wrapping_add(8)),
    ]
}

/// `point += offset * weight`, each axis: the product stored as a float (`0045bb20`), then the
/// sum (`0063c8a0`), written back at `at`.
#[inline(always)]
fn add_scaled(e: &mut Engine, at: u32, offset: [f32; 3], weight: f32) {
    let point = point_at(e, at);
    for ((p, o), k) in point.iter().zip(offset).zip(0u32..) {
        let sum = p + weight * o;
        e.mem.set_f32(at.wrapping_add(4 * k), sum);
    }
}

/// The locked vertex array of a geometry data: its start and stride (`00a67410` into a zeroed
/// cursor on the guest stack, as the exe's cursor constructor `00534550` leaves it; the lock
/// writes nothing while the buffers are not locked).
fn locked_array(e: &mut Engine, geometry: u32) -> (u32, u32) {
    e.with_stack(0xc, |e, cursor| {
        let c = cursor.addr();
        for k in 0..3u32 {
            e.mem.set_u32(c + 4 * k, 0);
        }
        e.call(GEOMETRY_BEGIN, &args![geometry, c]);
        (e.mem.u32(c), e.mem.u32(c + 4))
    })
}

// Typed from 00660860
/// `BSFaceGenMorphDifferential::ApplyMorph` (`thiscall(morph, object, unused, weight)`, `RET 0xc`):
/// with an object and a weight in range, each of the first `min(vertex count, morph count)`
/// vertices of the object's locked array gets `weight` times its offset (the morph's array at +4,
/// 12 bytes each; its count at +8) added.
pub fn differential_apply(e: &mut Engine, this: Ptr, object: u32, _unused: u32, weight: f32) {
    if object == 0 || !weight_in_range(weight) {
        return;
    }
    let geometry = e.call(OBJECT_GEOMETRY, &args![object]).u32();
    let (mut at, stride) = locked_array(e, geometry);
    let count = e.mem.u32(this.addr().wrapping_add(8));
    let vertices = u32::from(e.mem.u16(geometry.wrapping_add(DATA_VERTEX_COUNT)));
    let n = vertices.min(count);
    let mut offset = e.mem.u32(this.addr().wrapping_add(4));
    for _ in 0..n {
        let delta = point_at(e, offset);
        add_scaled(e, at, delta, weight);
        at = at.wrapping_add(stride);
        offset = offset.wrapping_add(0xc);
    }
}

// Typed from 00660a90
/// `BSFaceGenMorphStatistical::ApplyMorph` (`thiscall(morph, object, base, weight)`, `RET 0xc`):
/// with an object, a weight in range and a basis (the base's word +0xc), each indexed vertex (the
/// morph's index array at +4, its count at +8, read again each step) moves by `weight` times the
/// difference between basis row `first + i` (the morph's +0xc) and the vertex's own row.
pub fn statistical_apply(e: &mut Engine, this: Ptr, object: u32, base: u32, weight: f32) {
    if object == 0 || !weight_in_range(weight) {
        return;
    }
    if typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, base)) == 0 {
        return;
    }
    let basis = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, base));
    let geometry = e.call(OBJECT_GEOMETRY, &args![object]).u32();
    let (start, stride) = locked_array(e, geometry);
    // The data's vertex count (+8, 16 bits), read in place: an index past it is a vertex the base
    // copy does not reset, so a head with one is never left out by the exact skip.
    let vertices = u32::from(e.mem.u16(geometry.wrapping_add(8)));
    let t = this.addr();
    let mut i = 0u32;
    while i < e.mem.u32(t.wrapping_add(8)) {
        let indices = e.mem.u32(t.wrapping_add(4));
        let index = e.mem.u32(indices.wrapping_add(i.wrapping_mul(4)));
        // A vertex past the count, which the base copy does not reset: this head is never left out.
        if index >= vertices {
            super::apply::skip::note_past_count();
        }
        let at = start.wrapping_add(index.wrapping_mul(stride));
        let row = e
            .mem
            .u32(t.wrapping_add(0xc))
            .wrapping_add(i)
            .wrapping_mul(0xc)
            .wrapping_add(basis);
        let own = index.wrapping_mul(0xc).wrapping_add(basis);
        // The difference (`00439ef0`), each axis stored as a float.
        let (r, o) = (point_at(e, row), point_at(e, own));
        let difference = [r[0] - o[0], r[1] - o[1], r[2] - o[2]];
        add_scaled(e, at, difference, weight);
        i += 1;
    }
}

// Typed from 00653340
/// The base copy (cdecl(geometry, cursor)): with the geometry's morph extra data and a cursor whose
/// first word is set, the extra data's offsets (its word +0xc, 12 bytes each) are copied into the
/// cursor's vertices, one a stride, for the geometry's vertex count (`00456650`: the data's 16-bit
/// count, read in place); a geometry with data (`0049ec60`) then has it marked changed
/// (`MarkAsChanged` with 1).
pub fn copy_base(e: &mut Engine, geometry: u32, cursor: u32) {
    if geometry == 0 {
        return;
    }
    let name = e.mem.u32(EXTRA_DATA_NAME);
    let extra = e.call(GET_EXTRA_DATA, &args![geometry, name]).u32();
    if extra == 0 {
        return;
    }
    let mut at = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, cursor));
    if at == 0 {
        return;
    }
    let stride = e.mem.u32(cursor.wrapping_add(4));
    let mut offset = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, extra));
    let data = e.mem.u32(geometry.wrapping_add(GEOMETRY_DATA));
    let count = u32::from(e.mem.u16(data.wrapping_add(DATA_VERTEX_COUNT)));
    for _ in 0..count {
        for k in 0..3u32 {
            let w = e.mem.u32(offset.wrapping_add(4 * k));
            e.mem.set_u32(at.wrapping_add(4 * k), w);
        }
        at = at.wrapping_add(stride);
        offset = offset.wrapping_add(0xc);
    }
    if e.call(HAS_DATA, &args![geometry]).u32() != 0 {
        let data = e.call(OBJECT_GEOMETRY, &args![geometry]).u32();
        e.call(MARK_AS_CHANGED, &args![data, 1u32]);
    }
}
