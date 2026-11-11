//! The tangent space of a geometry (`BSShaderManager::CreateTangentSpaceSimple`): the lighting
//! property's set-up makes it for each geometry it lights, at load. It reads the positions, the
//! texture coordinates and the normals, and gives the data a new normal array three normals long
//! (the normals, then the binormals, then the tangents).

use super::data;
use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_div, x87_load32, x87_m32, x87_mul, x87_store32, x87_sub};
use engine_core::{args, typed_call, Engine};

/// What the tangent space calls: `_strnicmp` (`00ec7ec0`, its answer unused) with "STBB"
/// (`0102bf34`), the new object (`00aa13e0`, the size) and its constructor (`00b66810`, `thiscall`,
/// a word), the fixed string's make (`00a5b690`) and the extra data's lookup and removal
/// (`00a5bdd0`, `00a5be90`, `thiscall`, the address of the name) with the name
/// "Tangent space (binormal & tangent vectors)" (`010ad7a8`), `InterlockedDecrement` (import slot
/// `00fdf1c4`) on a fixed string's count unless it is the empty one (`0109b220`), the memory
/// manager (`011f6238`) and its allocate and free (`00aa3e40`, `00aa4060`), `memcpy` and `memset`,
/// `NiTriStripsData::GetStripLists` (`00a75f10`), the message formatter (`00406d00`) with the
/// no-normals message (`010ad730`) and the vector normalize (`004a0c10`, `thiscall`).
const STRNICMP: u32 = 0x00ec_7ec0;
const STBB: u32 = 0x0102_bf34;
const NEW_OBJECT: u32 = 0x00aa_13e0;
const TANGENT_DATA_CONSTRUCT: u32 = 0x00b6_6810;
const FIXED_STRING_MAKE: u32 = 0x00a5_b690;
const GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
const REMOVE_EXTRA_DATA: u32 = 0x00a5_be90;
const TANGENT_SPACE_NAME: u32 = 0x010a_d7a8;
const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
const EMPTY_FIXED_STRING: u32 = 0x0109_b220;
const MEMORY_MANAGER: u32 = 0x011f_6238;
const ALLOCATE: u32 = 0x00aa_3e40;
const FREE: u32 = 0x00aa_4060;
const MEMCPY: u32 = 0x00ec_44d0;
const MEMSET: u32 = 0x00ec_61c0;
const GET_STRIP_LISTS: u32 = 0x00a7_5f10;
const FORMAT: u32 = 0x0040_6d00;
const NO_NORMALS_MESSAGE: u32 = 0x010a_d730;
const NORMALIZE: u32 = 0x004a_0c10;

/// A geometry's data (`m_spModelData`, +0xb8).
const MODEL_DATA: u32 = 0xb8;

/// `FSTP dword`: the value's float bits.
fn store(value: f64) -> u32 {
    x87_store32(value)
}

/// `FLD dword` of the float at `address`.
fn load(e: &Engine, address: u32) -> f64 {
    x87_load32(e.mem.u32(address))
}

/// A float memory operand at `address`.
fn operand(e: &Engine, address: u32) -> f64 {
    x87_m32(e.mem.u32(address))
}

/// `count * 12` as the code's `MUL`, `SETO`, `NEG`, `OR` gives it: all ones on overflow.
fn twelve_times(count: u32) -> u32 {
    count.saturating_mul(0xc)
}

/// The inlined fixed string release: unless it is the empty string's handle, the count 8 bytes
/// before the text goes down (`InterlockedDecrement`).
fn release_fixed_string(e: &mut Engine, name: u32) {
    if name != e.global::<u32>(EMPTY_FIXED_STRING) {
        e.call(INTERLOCKED_DECREMENT, &args![name.wrapping_sub(8)]);
    }
}

/// The extra data's name made as a fixed string in a stack slot, `call` run with the slot's address,
/// and the name released.
fn with_tangent_space_name(e: &mut Engine, call: impl FnOnce(&mut Engine, u32) -> u32) -> u32 {
    e.with_stack(4, |e, slot| {
        let fixed = e.call(FIXED_STRING_MAKE, &args![TANGENT_SPACE_NAME]).u32();
        e.mem.set_u32(slot.addr(), fixed);
        let answer = call(e, slot.addr());
        let fixed = e.mem.u32(slot.addr());
        release_fixed_string(e, fixed);
        answer
    })
}

// Typed from 00b54b60
/// `BSShaderManager::CreateTangentSpaceSimple` (engine map; `cdecl`, the geometry). Compares the
/// geometry's name with "STBB" (the answer unused) and makes a 0x14-byte object (`00b66810` with 1).
/// When the geometry has the tangent space extra data, the object gets copies of its two halves (+0xc
/// the second, +0x10 the first, 12 bytes a vertex), the extra data is removed and the object is
/// answered. Otherwise, for a triangle shape (slot 0x1c) with normals, each triangle's tangent and
/// binormal (from the positions and the texture coordinates, through `1 / (du1 * dv2 - du2 * dv1)`)
/// are added into its three vertices' sums (a strip reads every other triangle with the winding
/// flipped; triangles with a repeated vertex are skipped); each vertex's tangent is made orthogonal to
/// its normal and the binormal their cross product (both normalized, `004a0c10`); the data gets the
/// normals followed by the binormals and tangents (`NiGeometryData::ReplaceData`, method 0x1000), the
/// old normals and the sums are freed, the object is deleted and 0 answered. Without normals a message
/// is formatted into a stack buffer and 0 answered (the object kept). The exe's exception frame and
/// stack cookie are not part of this.
pub fn create_tangent_space(e: &mut Engine, geometry: u32) -> u32 {
    let name = e.mem.u32(geometry + 8);
    if name != 0 {
        e.call(STRNICMP, &args![name, STBB, 4u32]);
    }
    let block = e.call(NEW_OBJECT, &args![0x14u32]).u32();
    let object = if block != 0 {
        e.call(TANGENT_DATA_CONSTRUCT, &args![block, 1u32]).u32()
    } else {
        0
    };
    let existing = with_tangent_space_name(e, |e, slot| {
        e.call(GET_EXTRA_DATA, &args![geometry, slot]).u32()
    });
    if existing != 0 {
        let data = e.mem.u32(geometry + MODEL_DATA);
        let count = u32::from(e.mem.u16(data + 8));
        let first = e.mem.u32(existing + 0xc);
        let size = count * 12;
        let second = first.wrapping_add(size);
        let copy = e
            .call(ALLOCATE, &args![MEMORY_MANAGER, twelve_times(count)])
            .u32();
        e.mem.set_u32(object + 0xc, copy);
        let copy = e
            .call(ALLOCATE, &args![MEMORY_MANAGER, twelve_times(count)])
            .u32();
        e.mem.set_u32(object + 0x10, copy);
        let destination = e.mem.u32(object + 0xc);
        e.call(MEMCPY, &args![destination, second, size]);
        let destination = e.mem.u32(object + 0x10);
        e.call(MEMCPY, &args![destination, first, size]);
        with_tangent_space_name(e, |e, slot| {
            e.call(REMOVE_EXTRA_DATA, &args![geometry, slot]);
            0
        });
        return object;
    }
    if e.vcall(geometry, 0x1c, &args![]).u32() == 0 {
        return 0;
    }
    let shape = e.vcall(geometry, 0x1c, &args![]).u32();
    let data = e.mem.u32(shape + MODEL_DATA);
    let triangles = e.mem.u16(data + 0x40);
    let normals = e.mem.u32(data + 0x24);
    let count = u32::from(e.mem.u16(data + 8));
    let vertices = e.mem.u32(data + 0x20);
    let coordinates = e.mem.u32(data + 0x2c);
    let strips = e.vcall(shape, 0x20, &args![]).u32() != 0;
    let mut strip = 0u32;
    if strips {
        let data = e.mem.u32(shape + MODEL_DATA);
        strip = e.call(GET_STRIP_LISTS, &args![data]).u32();
    }
    if normals == 0 {
        e.with_stack(0x100, |e, buffer| {
            e.call(FORMAT, &args![buffer, 0xffu32, NO_NORMALS_MESSAGE]);
        });
        return 0;
    }
    let sums = e
        .call(ALLOCATE, &args![MEMORY_MANAGER, twelve_times(count * 2)])
        .u32();
    let binormal_sums = sums.wrapping_add(count * 12);
    e.call(MEMSET, &args![sums, 0u32, count * 24]);
    let output = e
        .call(ALLOCATE, &args![MEMORY_MANAGER, twelve_times(count * 3)])
        .u32();
    e.call(MEMCPY, &args![output, normals, count * 12]);
    let binormals_out = output.wrapping_add(count * 12);
    let tangents_out = output.wrapping_add(count * 24);
    // The stack words a triangle's indices are read into (the triangle query writes their low halves).
    e.with_stack(0xc, |e, indices| {
        let slot0 = indices.addr();
        let slot1 = slot0 + 4;
        let slot2 = slot0 + 8;
        let mut flip = false;
        let mut triangle = 0u32;
        let mut cursor = strip.wrapping_add(4);
        if triangles == 0 {
            return;
        }
        loop {
            if strips {
                let a = u32::from(e.mem.u16(cursor.wrapping_sub(4)));
                let b = u32::from(e.mem.u16(cursor.wrapping_sub(2)));
                let c = u32::from(e.mem.u16(cursor));
                e.mem.set_u32(slot0, a);
                e.mem.set_u32(slot1, b);
                e.mem.set_u32(slot2, c);
                if flip {
                    e.mem.set_u32(slot2, a);
                    e.mem.set_u32(slot0, c);
                }
                flip = !flip;
            } else {
                let data = e.mem.u32(shape + MODEL_DATA);
                e.vcall(data, 0xac, &args![triangle, slot0, slot1, slot2]);
            }
            let i0 = e.mem.u16(slot0);
            let i1 = e.mem.u16(slot1);
            let i2 = e.mem.u16(slot2);
            if i0 != i1 && i1 != i2 && i0 != i2 {
                add_triangle(
                    e,
                    [u32::from(i0), u32::from(i1), u32::from(i2)],
                    vertices,
                    coordinates,
                    sums,
                    binormal_sums,
                );
            }
            cursor = cursor.wrapping_add(2);
            triangle = triangle.wrapping_add(1);
            if super::low_half(triangle) >= triangles {
                break;
            }
        }
    });
    for index in 0..count {
        let normal = normals.wrapping_add(12 * index);
        let sum = sums.wrapping_add(12 * index);
        let tangent = tangents_out.wrapping_add(12 * index);
        let binormal = binormals_out.wrapping_add(12 * index);
        let along = x87_add(
            x87_add(
                x87_mul(load(e, normal), operand(e, sum)),
                x87_mul(load(e, normal + 4), operand(e, sum + 4)),
            ),
            x87_mul(load(e, normal + 8), operand(e, sum + 8)),
        );
        let along = x87_load32(store(along));
        let part_x = store(x87_mul(load(e, normal), along));
        let part_y = store(x87_mul(load(e, normal + 4), along));
        let part_z = store(x87_mul(along, operand(e, normal + 8)));
        let tangent_x = store(x87_sub(load(e, sum), x87_m32(part_x)));
        let tangent_y = store(x87_sub(load(e, sum + 4), x87_m32(part_y)));
        let tangent_z = store(x87_sub(load(e, sum + 8), x87_m32(part_z)));
        e.mem.set_u32(tangent, tangent_x);
        e.mem.set_u32(tangent + 4, tangent_y);
        e.mem.set_u32(tangent + 8, tangent_z);
        let cross_x = store(x87_sub(
            x87_mul(load(e, tangent + 8), operand(e, normal + 4)),
            x87_mul(load(e, normal + 8), operand(e, tangent + 4)),
        ));
        let cross_y = store(x87_sub(
            x87_mul(load(e, normal + 8), operand(e, tangent)),
            x87_mul(load(e, tangent + 8), operand(e, normal)),
        ));
        let cross_z = store(x87_sub(
            x87_mul(load(e, normal), operand(e, tangent + 4)),
            x87_mul(load(e, normal + 4), operand(e, tangent)),
        ));
        e.mem.set_u32(binormal, cross_x);
        e.mem.set_u32(binormal + 4, cross_y);
        e.mem.set_u32(binormal + 8, cross_z);
        e.call(NORMALIZE, &args![tangent]);
        e.call(NORMALIZE, &args![binormal]);
    }
    e.call(FREE, &args![MEMORY_MANAGER, sums]);
    let data = e.mem.u32(geometry + MODEL_DATA);
    let old_normals = e.mem.u32(data + 0x24);
    e.call(FREE, &args![MEMORY_MANAGER, old_normals]);
    let flags = u32::from(e.mem.u16(data + 0xc) & 1);
    let coordinates = e.mem.u32(data + 0x2c);
    let colours = e.mem.u32(data + 0x28);
    let vertices = e.mem.u32(data + 0x20);
    typed_call!(
        FAMILY,
        0x00a6_6bb0,
        data::replace_data(
            e,
            Ptr::new(data),
            count,
            vertices,
            output,
            colours,
            coordinates,
            flags,
            0x1000
        )
    );
    if object != 0 {
        e.vcall(object, 0, &args![1u32]);
    }
    0
}

/// How one of the sums is added, as the code orders the operands: `FLD value; FADD [sum]`
/// (`ValueAndOperand`), the sum loaded and the value added into it (`SumAndValue`), or the sum loaded
/// and added into the value's register (`ValueAndSum`). Only which of two NaNs comes out tells them
/// apart.
#[derive(Clone, Copy)]
enum Addition {
    ValueAndOperand,
    SumAndValue,
    ValueAndSum,
}

/// The float at `at` plus `value`, stored back as a float.
fn add_to(e: &mut Engine, at: u32, value: f64, mode: Addition) {
    let sum = match mode {
        Addition::ValueAndOperand => x87_add(value, operand(e, at)),
        Addition::SumAndValue => x87_add(load(e, at), value),
        Addition::ValueAndSum => x87_add(value, load(e, at)),
    };
    e.mem.set_u32(at, store(sum));
}

/// One triangle: its tangent and binormal from the edge vectors and the texture coordinates'
/// differences (in the code's x87 order, each stored as a float), added to its three vertices' sums.
fn add_triangle(
    e: &mut Engine,
    [i0, i1, i2]: [u32; 3],
    vertices: u32,
    coordinates: u32,
    sums: u32,
    binormal_sums: u32,
) {
    let v0 = vertices.wrapping_add(12 * i0);
    let v1 = vertices.wrapping_add(12 * i1);
    let v2 = vertices.wrapping_add(12 * i2);
    let edge = |e: &Engine, to: u32, offset: u32| {
        store(x87_sub(load(e, to + offset), operand(e, v0 + offset)))
    };
    let e1x = edge(e, v1, 0);
    let e2x = edge(e, v2, 0);
    let e1y = edge(e, v1, 4);
    let e2y = edge(e, v2, 4);
    let e1z = edge(e, v1, 8);
    let e2z = edge(e, v2, 8);
    let uv0 = coordinates.wrapping_add(8 * i0);
    let uv1 = coordinates.wrapping_add(8 * i1);
    let uv2 = coordinates.wrapping_add(8 * i2);
    let du1 = x87_load32(store(x87_sub(load(e, uv1), operand(e, uv0))));
    let du2 = x87_load32(store(x87_sub(load(e, uv2), operand(e, uv0))));
    let dv1 = x87_load32(store(x87_sub(load(e, uv1 + 4), operand(e, uv0 + 4))));
    let dv2 = x87_load32(store(x87_sub(load(e, uv2 + 4), operand(e, uv0 + 4))));
    let determinant = x87_sub(x87_mul(dv2, du1), x87_mul(dv1, du2));
    let inverse = x87_load32(store(x87_div(1.0, determinant)));
    let tangent_x = store(x87_mul(
        x87_sub(x87_mul(dv2, x87_m32(e1x)), x87_mul(dv1, x87_m32(e2x))),
        inverse,
    ));
    let tangent_y = store(x87_mul(
        x87_sub(x87_mul(dv2, x87_m32(e1y)), x87_mul(dv1, x87_m32(e2y))),
        inverse,
    ));
    let e1z = x87_load32(e1z);
    let e2z = x87_load32(e2z);
    let tangent_z = store(x87_mul(
        x87_sub(x87_mul(dv2, e1z), x87_mul(dv1, e2z)),
        inverse,
    ));
    let binormal_x = store(x87_mul(
        x87_sub(x87_mul(du1, x87_m32(e2x)), x87_mul(du2, x87_m32(e1x))),
        inverse,
    ));
    let binormal_y = store(x87_mul(
        x87_sub(x87_mul(du1, x87_m32(e2y)), x87_mul(du2, x87_m32(e1y))),
        inverse,
    ));
    let binormal_z = store(x87_mul(
        inverse,
        x87_sub(x87_mul(e2z, du1), x87_mul(du2, e1z)),
    ));
    let tangent = [tangent_x, tangent_y, tangent_z].map(x87_load32);
    let binormal = [binormal_x, binormal_y, binormal_z].map(x87_load32);
    use Addition::{SumAndValue as S, ValueAndOperand as A, ValueAndSum as C};
    let t0 = sums.wrapping_add(12 * i0);
    let t1 = sums.wrapping_add(12 * i1);
    let t2 = sums.wrapping_add(12 * i2);
    for (at, modes) in [(t0, [A, S, S]), (t1, [A, S, A]), (t2, [C, A, A])] {
        for (axis, (value, mode)) in (0u32..).zip(tangent.iter().zip(modes)) {
            add_to(e, at + 4 * axis, *value, mode);
        }
    }
    let b0 = binormal_sums.wrapping_add(12 * i0);
    let b1 = binormal_sums.wrapping_add(12 * i1);
    let b2 = binormal_sums.wrapping_add(12 * i2);
    for (at, modes) in [(b0, [S, A, S]), (b1, [S, A, S]), (b2, [C, A, A])] {
        for (axis, (value, mode)) in (0u32..).zip(binormal.iter().zip(modes)) {
            add_to(e, at + 4 * axis, *value, mode);
        }
    }
}
