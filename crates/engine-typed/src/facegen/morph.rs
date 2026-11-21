//! A face geometry's morphs over its locked vertex array: the base offsets copied in, then each
//! morph added with its weight. The locked array is a strided cursor `{base, stride, packed}` (the
//! geometry data's lock, `00a67410`, fills it); a vertex is three floats at `base + index * stride`.

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{
    x87_load32, x87_m32, x87_mul_store32, x87_quiet_bits, x87_store32, x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// The exe's double constants the weight is compared with: 0.0 (`01012060`) and 1.0 (`01012070`).
const ZERO: u32 = 0x0101_2060;
const ONE: u32 = 0x0101_2070;
/// The object's geometry data (`005495f0`), its vertex count (`004a8ae0`, `AX`), the locked
/// array's start into a cursor (`00a67410`: data; cursor).
const OBJECT_GEOMETRY: u32 = 0x0054_95f0;
const GEOMETRY_VERTEX_COUNT: u32 = 0x004a_8ae0;
const GEOMETRY_BEGIN: u32 = 0x00a6_7410;
/// The face geometry's vertex count (`00456650`, 16 bits), its morph extra data
/// (`NiObjectNET::GetExtraData`, `00a5bdd0`, by address as `00653430` calls it) with the extra
/// data's name (`011d5b8c`), the data's test (`0049ec60`) and `MarkAsChanged` (`00a67090`).
const FACE_VERTEX_COUNT: u32 = 0x0045_6650;
const GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
const EXTRA_DATA_NAME: u32 = 0x011d_5b8c;
const HAS_DATA: u32 = 0x0049_ec60;
const MARK_AS_CHANGED: u32 = 0x00a6_7090;

/// `BSFaceGenMorph::ApplyMorph`'s weight test: above 0.0 and not above 1.0, as the exe compares the
/// float with its doubles (a NaN passes, as there).
fn weight_in_range(e: &Engine, weight: u32) -> bool {
    let w = x87_m32(weight);
    !(w <= e.global::<f64>(ZERO)) && !(w > e.global::<f64>(ONE))
}

/// Three words copied.
#[inline(always)]
fn copy3(e: &mut Engine, to: u32, from: u32) {
    for k in 0..3u32 {
        let w = e.mem.u32(from.wrapping_add(4 * k));
        e.mem.set_u32(to.wrapping_add(4 * k), w);
    }
}

/// `point += offset * weight`, each axis: the product stored as a float at the running thread's
/// precision (`0045bb20`), the sum as a double rounded to a float (`0063c8a0`); the point's words
/// written back at `to`.
#[inline(always)]
fn add_scaled(e: &mut Engine, to: u32, point: [u32; 3], offset: u32, weight: u32) {
    // The scaled offset, z first as the exe computes it.
    let z = x87_mul_store32(weight, e.mem.u32(offset.wrapping_add(8)));
    let y = x87_mul_store32(weight, e.mem.u32(offset.wrapping_add(4)));
    let x = x87_mul_store32(weight, e.mem.u32(offset));
    let scaled = [x, y, z];
    for k in 0..3usize {
        let sum = f64::from(f32::from_bits(point[k])) + f64::from(f32::from_bits(scaled[k]));
        #[allow(clippy::cast_possible_truncation)]
        let sum = sum as f32;
        e.mem.set_u32(to.wrapping_add(4 * k as u32), sum.to_bits());
    }
}

/// A point's three words at `at`.
#[inline(always)]
fn point_at(e: &Engine, at: u32) -> [u32; 3] {
    [
        e.mem.u32(at),
        e.mem.u32(at.wrapping_add(4)),
        e.mem.u32(at.wrapping_add(8)),
    ]
}

/// The locked vertex array of `object`'s geometry data: its start and stride (`00a67410` into a
/// cursor on the guest stack, as the exe makes it).
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
    if object == 0 || !weight_in_range(e, weight.to_bits()) {
        return;
    }
    let weight = x87_quiet_bits(weight.to_bits());
    let geometry = e.call(OBJECT_GEOMETRY, &args![object]).u32();
    let (base, stride) = locked_array(e, geometry);
    let count = e.mem.u32(this.addr().wrapping_add(8));
    let vertices = u32::from(e.call(GEOMETRY_VERTEX_COUNT, &args![geometry]).u16());
    let n = vertices.min(count);
    let offsets = e.mem.u32(this.addr().wrapping_add(4));
    let mut at = base;
    for i in 0..n {
        let point = point_at(e, at);
        add_scaled(e, at, point, offsets.wrapping_add(i.wrapping_mul(0xc)), weight);
        at = at.wrapping_add(stride);
    }
}

// Typed from 00660a90
/// `BSFaceGenMorphStatistical::ApplyMorph` (`thiscall(morph, object, base, weight)`, `RET 0xc`):
/// with an object, a weight in range and a basis (the base's word +0xc), each indexed vertex (the
/// morph's index array at +4, its count at +8, read again each step) moves by `weight` times the
/// difference between basis row `first + i` (the morph's +0xc) and the vertex's own row.
pub fn statistical_apply(e: &mut Engine, this: Ptr, object: u32, base: u32, weight: f32) {
    if object == 0 || !weight_in_range(e, weight.to_bits()) {
        return;
    }
    if typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, base)) == 0 {
        return;
    }
    let weight = x87_quiet_bits(weight.to_bits());
    let basis = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, base));
    let geometry = e.call(OBJECT_GEOMETRY, &args![object]).u32();
    let (start, stride) = locked_array(e, geometry);
    let t = this.addr();
    let mut i = 0u32;
    while i < e.mem.u32(t.wrapping_add(8)) {
        let indices = e.mem.u32(t.wrapping_add(4));
        let index = e.mem.u32(indices.wrapping_add(i.wrapping_mul(4)));
        let at = start.wrapping_add(index.wrapping_mul(stride));
        let point = point_at(e, at);
        let row = e
            .mem
            .u32(t.wrapping_add(0xc))
            .wrapping_add(i)
            .wrapping_mul(0xc)
            .wrapping_add(basis);
        let own = index.wrapping_mul(0xc).wrapping_add(basis);
        // The difference (`00439ef0`): each axis `FLD row; FSUB own` at the running thread's
        // precision, stored as a float.
        let difference: [u32; 3] = std::array::from_fn(|k| {
            let k = 4 * k as u32;
            x87_store32(x87_sub(
                x87_load32(e.mem.u32(row.wrapping_add(k))),
                x87_m32(e.mem.u32(own.wrapping_add(k))),
            ))
        });
        e.with_stack(0xc, |e, local| {
            let d = local.addr();
            for (k, w) in (0u32..).zip(difference) {
                e.mem.set_u32(d + 4 * k, w);
            }
            add_scaled(e, at, point, d, weight);
        });
        i += 1;
    }
}

// Typed from 00653340
/// The base copy (cdecl(geometry, cursor)): with the geometry's morph extra data and a cursor whose
/// first word is set, the extra data's offsets (its word +0xc, 12 bytes each) are copied into the
/// cursor's vertices, one a stride, for the geometry's vertex count (`00456650`); a geometry with data
/// (`0049ec60`) then has it marked changed (`MarkAsChanged` with 1).
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
    let count = u32::from(e.call(FACE_VERTEX_COUNT, &args![geometry]).u16());
    for _ in 0..count {
        copy3(e, at, offset);
        at = at.wrapping_add(stride);
        offset = offset.wrapping_add(0xc);
    }
    if e.call(HAS_DATA, &args![geometry]).u32() != 0 {
        let data = e.call(OBJECT_GEOMETRY, &args![geometry]).u32();
        e.call(MARK_AS_CHANGED, &args![data, 1u32]);
    }
}
