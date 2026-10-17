//! `Havok SDK/run_00e2b070` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! These functions belong to a convex-hull growing routine over two convex
//! shapes (the next unit, `run_00df9840`, drives it); the executable has no
//! names for them and no Xbox PDB entry, so the routines are named by address
//! and the fields below come from this code only.
//!
//! The polytope object (`this` of `00e2d0d0`, `00e2d1b0`, `00e2d440`,
//! `00e2d550`) holds: +0x08 and +0x0c counts, +0x10 the number of faces in
//! use, +0x20 and +0x50 vertex records of 0x40 bytes (the vector at +0, a link
//! at +0x30), +0xde0 a stack of up to 100 recycled face pointers (count at
//! +0x0c) and +0xf70 the faces, 0x50 bytes each: the normal (+0x00), three
//! edge slots of 0x10 bytes (+0x10, +0x20, +0x30), a state word (+0x44) and
//! the plane distance (+0x40). SSE code computes lane by lane in `f32`; x87
//! sums stay unrounded until the `FSTP float`, compares read the status word
//! the way the `TEST AH` pairs do (NaN included).
//!
//! Register conventions: `00e2da40` takes the shape in `ECX`, the matrix in
//! `ESI` and the direction in `EDI` (stack: the second shape and the record,
//! the caller pops them); here those registers are the first three
//! arguments, in that order.

use crate::units::fallout_shared::tesobjectrefr::rsqrt_estimate;

#[allow(unused_imports)]
use crate::prelude::*;

/// Four lanes as raw bits.
type Lanes = [u32; 4];

/// `MOVAPS xmm, [at]`: the 16 bytes at `at` as four words.
fn load_lanes(e: &Engine, at: u32) -> Lanes {
    std::array::from_fn(|lane| e.mem.u32(at.wrapping_add(4 * lane as u32)))
}

/// `MOVAPS [at], xmm`.
fn store_lanes(e: &mut Engine, at: u32, v: Lanes) {
    for (lane, word) in v.into_iter().enumerate() {
        e.mem.set_u32(at.wrapping_add(4 * lane as u32), word);
    }
}

type Vec4 = [f32; 4];

/// Float constants: 0.0, 3.0 and 0.5 (the `RSQRTSS` refinement), the largest
/// value (`0x7f7fffee`) and the most negative one (`0xff7fffee`).
const ZERO: u32 = 0x0101_1d78;
const THREE: u32 = 0x0101_7718;
const HALF: u32 = 0x0101_6248;
const MAX_FLOAT: u32 = 0x0104_da48;
const MIN_FLOAT: u32 = 0x0104_e148;
/// Four sign-bit masks (`0x80000000`).
const SIGN_MASK: u32 = 0x010c_44d0;
/// `double` constants: -1e-10 (as a float widened), 0.9 and 0.99, and 0.5.
const NEG_TINY: u32 = 0x010e_b190;
const NINE_TENTHS: u32 = 0x0106_b9e8;
const SHRINK: u32 = 0x0106_d508;
const HALF_DOUBLE: u32 = 0x0101_1588;
/// The `float` 0.001 the shrinking step of `00e2db70` stops at.
const SMALL_STEP: u32 = 0x0101_7d00;
/// The table of three direction vectors `00e2db70` starts from.
const START_DIRECTIONS: u32 = 0x0126_8370;
/// The table `00e2d440` builds the first four faces from (twelve entries of
/// three words).
const FACE_TABLE: u32 = 0x010e_d0f0;

/// Callees outside this unit, and the functions of this unit that call each
/// other.
const RESET_STATES: u32 = 0x00e6_7fd0;
const FACE_SIDE_TEST: u32 = 0x00e6_3af0;
const NORMALIZE_FACE: u32 = 0x00e2_cfe0;
const SUPPORT_POINT: u32 = 0x00e2_da40;
const WALK_FACES: u32 = 0x00e2_d0d0;

fn load_vec(e: &Engine, addr: u32) -> Vec4 {
    std::array::from_fn(|lane| e.mem.f32(addr + 4 * lane as u32))
}

fn store_vec(e: &mut Engine, addr: u32, v: Vec4) {
    for (lane, value) in v.into_iter().enumerate() {
        e.mem.set_f32(addr + 4 * lane as u32, value);
    }
}

/// A 16-byte copy (`MOVAPS` load and store), bit for bit.
fn copy_vec(e: &mut Engine, from: u32, to: u32) {
    for lane in 0..4 {
        let word = e.mem.u32(from + 4 * lane);
        e.mem.set_u32(to + 4 * lane, word);
    }
}

fn zip_with(a: Vec4, b: Vec4, f: impl Fn(f32, f32) -> f32) -> Vec4 {
    std::array::from_fn(|lane| f(a[lane], b[lane]))
}

/// `z + (y + x)`: the `SHUFPS`/`ADDSS` sum of the first three lanes.
fn sum3(v: Vec4) -> f32 {
    v[2] + (v[1] + v[0])
}

/// `SHUFPS v, v, 0xc9`: the lanes y, z, x, w.
fn yzx(v: Vec4) -> Vec4 {
    [v[1], v[2], v[0], v[3]]
}

/// `SHUFPS v, v, 0xd2`: the lanes z, x, y, w.
fn zxy(v: Vec4) -> Vec4 {
    [v[2], v[0], v[1], v[3]]
}

/// `first.yzx * second.zxy - first.zxy * second.yzx`, the cross product of
/// the SSE sequence (`MULPS`, `MULPS`, `SUBPS`), lane by lane.
fn cross(first: Vec4, second: Vec4) -> Vec4 {
    let plus = zip_with(yzx(first), zxy(second), |a, b| a * b);
    let minus = zip_with(zxy(first), yzx(second), |a, b| a * b);
    zip_with(plus, minus, |a, b| a - b)
}

/// The `RSQRTSS` estimate refined once, with the `CMPEQSS`/`ANDNPS` zero
/// guard: `(half * r) * (three - (sum * r) * r)`, `+0.0` when `sum == 0`.
fn inverse_length(e: &Engine, sum: f32) -> f32 {
    let zero: f32 = e.global(ZERO);
    let three: f32 = e.global(THREE);
    let half: f32 = e.global(HALF);
    let estimate = rsqrt_estimate(sum);
    let step = three - (sum * estimate) * estimate;
    let value = (half * estimate) * step;
    if sum == zero {
        0.0
    } else {
        value
    }
}

/// `vector` with every lane's sign bit flipped (`XORPS` with the sign mask).
fn negated(e: &Engine, vector: Vec4) -> Vec4 {
    std::array::from_fn(|lane| {
        let mask = e.mem.u32(SIGN_MASK + 4 * lane as u32);
        f32::from_bits(vector[lane].to_bits() ^ mask)
    })
}

/// `FLD float; FCHS; FSTP float`: the negation of a value that went through
/// the x87 stack (a signalling NaN comes back quiet).
fn x87_negated(value: f32) -> f32 {
    f32::from_bits(x87_quiet_bits(value.to_bits()) ^ 0x8000_0000)
}

// Translated from 00e2cfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets up the plane of a face (`this` in `ECX`, a face of the polytope):
/// the normal is `(v0 - v1) x (v1 - v2)` of the vectors the three slot
/// pointers at +0x10, +0x20 and +0x30 point to (the SSE cross product, lane
/// by lane), normalised with the refined `RSQRTSS` (zero stays zero) and
/// stored at +0x00; the plane distance, the sum `z + (y + x)` of the unit
/// normal times `v0`, goes to +0x40.
pub fn fn_00e2cfe0(e: &mut Engine, face: Ptr) {
    let v1 = load_vec(e, e.mem.u32(face.addr() + 0x20));
    let v0_address = e.mem.u32(face.addr() + 0x10);
    let v0 = load_vec(e, v0_address);
    let v2 = load_vec(e, e.mem.u32(face.addr() + 0x30));
    let first = zip_with(v0, v1, |a, b| a - b);
    let second = zip_with(v1, v2, |a, b| a - b);
    let normal = cross(first, second);
    let sum = sum3(zip_with(normal, normal, |a, b| a * b));
    let scale = inverse_length(e, sum);
    let unit: Vec4 = normal.map(|lane| scale * lane);
    store_vec(e, face.addr(), unit);
    let v0_again = load_vec(e, v0_address);
    let distance = sum3(zip_with(unit, v0_again, |a, b| a * b));
    e.mem.set_f32(face.addr() + 0x40, x87_quiet(distance));
}

// Translated from 00e2d0d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks the faces around `face` (`this` in `ECX` is the polytope): for each
/// of its three edge slots, whose pointer at +0x18, +0x28, +0x38 leads to the
/// record of the face across it (`record + 0xc`), a neighbour with state 0
/// is classified by `00e63af0` (`point_test` and the polytope's first two
/// words are its arguments) and the state saved: a result of 1 recurses into
/// that neighbour (a result of 2 from the recursion ends this walk), a result
/// of 4 ends the walk with 2; a neighbour in state 2 (just classified or from
/// before) is linked: the record's first word points to a structure whose
/// +0x30 gets the record, `*count` goes up by one and `*last` is set to the
/// record. Afterwards the face's plane distance (+0x40) is set to the largest
/// float and, while the stack of processed faces (+0xde0, count at +0x0c)
/// has fewer than 100 entries, the face is pushed on it. Returns the walk's
/// result, 2 when the stack is full.
pub fn fn_00e2d0d0(
    e: &mut Engine,
    this: Ptr,
    face: Ptr,
    point_test: u32,
    count: Ptr,
    last: Ptr,
) -> i32 {
    let mut result = 0i32;
    let mut slot_pointer = face.addr() + 0x18;
    let mut index = 0;
    loop {
        let record = e.mem.u32(slot_pointer);
        let neighbour = e.mem.u32(record + 0xc);
        let mut state = e.mem.u32(neighbour + 0x44);
        if state == 0 {
            state = e
                .call(
                    FACE_SIDE_TEST,
                    &args![neighbour, point_test, this, this.addr() + 4],
                )
                .u32();
            e.mem.set_u32(neighbour + 0x44, state);
            if state == 1 {
                result = e
                    .call(WALK_FACES, &args![this, neighbour, point_test, count, last])
                    .u32() as i32;
                if result == 2 {
                    break;
                }
            } else if state == 4 {
                result = 2;
                break;
            }
        }
        if state == 2 {
            let target = e.mem.u32(record);
            e.mem.set_u32(target + 0x30, record);
            let counted = e.mem.u32(count.addr());
            e.mem.set_u32(count.addr(), counted.wrapping_add(1));
            e.mem.set_u32(last.addr(), record);
        }
        slot_pointer = slot_pointer.wrapping_add(0x10);
        index += 1;
        if index >= 3 {
            break;
        }
    }
    // The plane distance is reset to the largest float (`FLD`/`FSTP`).
    let largest = x87_quiet(e.global::<f32>(MAX_FLOAT));
    e.mem.set_f32(face.addr() + 0x40, largest);
    let depth = e.mem.i32(this.addr() + 0xc);
    if depth >= 100 {
        return 2;
    }
    e.mem
        .set_u32(this.addr() + 0xde0 + 4 * depth as u32, face.addr());
    e.mem.set_i32(this.addr() + 0xc, depth + 1);
    result
}

// Translated from 00e2d1b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the faces that close the hole left when the faces of a visible
/// region were removed (`this` in `ECX` is the polytope): `count` faces are
/// taken, first from the stack of recycled faces (+0xde0, count at +0x0c),
/// then new ones from the end of the face array (+0xf70 + 0x50 * the number
/// in use at +0x10, which grows). The edges of the hole form a linked list
/// starting at `edge_list`: an edge record holds its first vertex pointer
/// (+0x00) and a slot (+0x04) leading to the pointer of its second vertex,
/// whose record's +0x30 gives the next edge (and is cleared). Face `i`,
/// built on the edge `i`, joins that edge's two vertices and `point`; the
/// edge slot, the face's three slots (back pointers, links to the neighbour
/// face `i + 1`, wrapping to the first) and its plane (the unit normal of
/// `(vb - va) x (va - point)` from the SSE cross product with the refined
/// `RSQRTSS`, the distance `z + (y + x)` of the normal times `vb`) are
/// filled in. The call returns 2 early when the list ends too soon or a face
/// has a zero normal. Afterwards the states of all faces (+0x44) and the
/// first +0x08 vertex records (+0x50, 0x40 apart) are cleared; returns 2
/// when the last edge reached is not `edge_list` (the hole did not close),
/// else 0.
pub fn fn_00e2d1b0(e: &mut Engine, this: Ptr, count: i32, edge_list: Ptr, point: Ptr) -> u32 {
    let t = this.addr();
    // A scratch array of face pointers on the stack (up to 100).
    e.with_stack(0x200, |e, scratch| {
        let local = scratch.addr();
        let mut top = e.mem.i32(t + 0xc).wrapping_sub(1);
        let mut filled = 0u32;
        let mut remaining = count;
        if count != 0 {
            let mut slot = t
                .wrapping_add(0xde0)
                .wrapping_add((top as u32).wrapping_mul(4));
            loop {
                if top < 0 {
                    break;
                }
                let face = e.mem.u32(slot);
                let depth = e.mem.i32(t + 0xc);
                e.mem.set_i32(t + 0xc, depth.wrapping_sub(1));
                e.mem.set_u32(local + 4 * filled, face);
                remaining = remaining.wrapping_sub(1);
                filled += 1;
                top = top.wrapping_sub(1);
                slot = slot.wrapping_sub(4);
                if remaining == 0 {
                    break;
                }
            }
        }
        if remaining > 0 {
            let in_use = e.mem.i32(t + 0x10);
            let mut offset = (in_use as u32).wrapping_mul(0x50);
            for k in 0..remaining as u32 {
                e.mem.set_u32(
                    local + 4 * (filled + k),
                    offset.wrapping_add(t).wrapping_add(0xf70),
                );
                offset = offset.wrapping_add(0x50);
            }
            e.mem.set_i32(t + 0x10, in_use.wrapping_add(remaining));
        }

        let mut edge = edge_list.addr();
        let mut index = 0i32;
        if count > 0 {
            loop {
                if edge == 0 {
                    return 2;
                }
                let face = e.mem.u32(local + 4 * index as u32);
                let next_index = index + 1;
                let next_face = if next_index < count {
                    e.mem.u32(local + 4 * next_index as u32)
                } else {
                    e.mem.u32(local)
                };
                e.mem.set_u32(face + 0x1c, face);
                e.mem.set_u32(face + 0x2c, face);
                e.mem.set_u32(face + 0x3c, face);
                let second_slot = face + 0x20;
                e.mem.set_u32(face + 0x14, second_slot);
                let third_slot = face + 0x30;
                e.mem.set_u32(face + 0x24, third_slot);
                let first_slot = face + 0x10;
                e.mem.set_u32(face + 0x34, first_slot);
                let second_pointer = e.mem.u32(e.mem.u32(edge + 4));
                let first_pointer = e.mem.u32(edge);
                e.mem.set_u32(first_slot, second_pointer);
                e.mem.set_u32(third_slot, point.addr());
                e.mem.set_u32(face + 0x20, first_pointer);
                e.mem.set_u32(edge + 8, first_slot);
                e.mem.set_u32(face + 0x18, edge);
                e.mem.set_u32(next_face + 0x28, third_slot);
                e.mem.set_u32(face + 0x38, next_face + 0x20);

                let second = load_vec(e, second_pointer);
                let first = load_vec(e, first_pointer);
                let apex = load_vec(e, point.addr());
                let along = zip_with(second, first, |a, b| a - b);
                let back = zip_with(first, apex, |a, b| a - b);
                let normal = cross(along, back);
                let sum = sum3(zip_with(normal, normal, |a, b| a * b));
                if sum == 0.0 {
                    return 2;
                }
                let scale = inverse_length(e, sum);
                let unit: Vec4 = normal.map(|lane| scale * lane);
                let distance = sum3(zip_with(second, unit, |a, b| a * b));
                e.mem.set_f32(face + 0x40, x87_quiet(distance));
                store_vec(e, face, unit);
                let next_slot = e.mem.u32(edge + 4);
                let vertex = e.mem.u32(next_slot);
                edge = e.mem.u32(vertex + 0x30);
                e.mem.set_u32(vertex + 0x30, 0);
                index = next_index;
                if index >= count {
                    break;
                }
            }
        }
        for face in 0..e.mem.i32(t + 0x10).max(0) as u32 {
            e.mem.set_u32(t + 0xfb4 + 0x50 * face, 0);
        }
        for vertex in 0..e.mem.i32(t + 8).max(0) as u32 {
            e.mem.set_u32(t + 0x50 + 0x40 * vertex, 0);
        }
        if edge_list.addr() != edge {
            2
        } else {
            0
        }
    })
}

// Translated from 00e2d440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets up the first four faces of the polytope (`this` in `ECX`): the face
/// count (+0x10) becomes 4 and each of the four faces (+0xf70, 0x50 bytes
/// apart) gets its slot links and, from the twelve-entry table at
/// `010ed0f0`, the vertex pointers of its three slots (`entry[0] + this +
/// 0x20`, and `entry[2] + entry[1] + this + 0xf80` for the pointer in +8);
/// state 0. The last face (+0x1060) is given its plane (`00e2cfe0`); if the
/// first two vertices (+0x20 and +0x60) satisfy `(v20 - v60) . normal > 0`
/// for that normal, the vertex pointers of three slots are redirected to the
/// vertices at +0x60 and +0x20 (+0x1020, +0xfd0, +0xf80 get +0x60; +0x1070,
/// +0xfe0, +0xfa0 get +0x20). Then all four faces get their planes.
pub fn fn_00e2d440(e: &mut Engine, this: Ptr) {
    let t = this.addr();
    e.mem.set_u32(t + 0x10, 4);
    let mut table = FACE_TABLE;
    let end = t + 0x10b0;
    let mut face = t + 0xf70;
    while face != end {
        e.mem.set_u32(face + 0x14, face + 0x20);
        e.mem.set_u32(face + 0x24, face + 0x30);
        let mut slot = face + 0x10;
        let limit = face + 0x40;
        e.mem.set_u32(face + 0x44, 0);
        e.mem.set_u32(face + 0x34, slot);
        while slot < limit {
            e.mem.set_u32(slot + 0xc, face);
            let first = e.mem.u32(table);
            e.mem
                .set_u32(slot, first.wrapping_add(t).wrapping_add(0x20));
            let third = e.mem.u32(table + 8).wrapping_add(e.mem.u32(table + 4));
            slot += 0x10;
            e.mem
                .set_u32(slot - 8, third.wrapping_add(t).wrapping_add(0xf80));
            table = table.wrapping_add(0xc);
        }
        face += 0x50;
    }
    let last_face = t + 0x1060;
    e.call(NORMALIZE_FACE, &args![last_face]);
    let v60 = t + 0x60;
    let v20 = t + 0x20;
    let difference = zip_with(load_vec(e, v20), load_vec(e, v60), |a, b| a - b);
    let normal = load_vec(e, last_face);
    let side = sum3(zip_with(difference, normal, |a, b| a * b));
    if side > 0.0 {
        e.mem.set_u32(t + 0x1020, v60);
        e.mem.set_u32(t + 0xfd0, v60);
        e.mem.set_u32(t + 0xf80, v60);
        e.mem.set_u32(last_face + 0x10, v20);
        e.mem.set_u32(t + 0xfe0, v20);
        e.mem.set_u32(t + 0xfa0, v20);
    }
    for k in 0..4 {
        e.call(NORMALIZE_FACE, &args![t + 0xf70 + 0x50 * k]);
    }
}

// Translated from 00e2d550 (decompiled, FalloutNV.exe 1.4.0.525)
/// Hill-climbs over the faces of the polytope (`this` in `ECX`) from `face`:
/// resets the per-face states (`00e67fd0`), then repeatedly marks the
/// current face as state 3 and looks through its three slots, skipping the
/// one it was entered by. For slot `k` with vertices `p` (its pointer at
/// +0x00) and `q` (the pointer in the slot at +0x04), `d = q - p`, `f = (d x
/// normal) . p`, the slot counts as a way out when `-1e-10 * d.d > f * |f|`
/// (the constant is a `double`, the product stays unrounded) and the starting
/// face's plane distance (+0x40) is not above 0 or 0.9 times it is at most the
/// neighbour's. The walk moves to the neighbour across the slot (`slot + 8`
/// leads to its record, `+0xc` to the face), ending after that move when
/// the neighbour already has state 3. Returns the face it stopped at.
pub fn fn_00e2d550(e: &mut Engine, this: Ptr, face: Ptr) -> Ptr {
    e.call(RESET_STATES, &args![this]);
    let tiny: f64 = e.global(NEG_TINY);
    let nine_tenths: f64 = e.global(NINE_TENTHS);
    let mut current = face.addr();
    let mut excluded = 0u32;
    let mut done = false;
    loop {
        if done {
            return Ptr::new(current);
        }
        e.mem.set_u32(current + 0x44, 3);
        let mut slot_index = 0u32;
        let mut slot = current + 0x10;
        while slot_index < 3 {
            if slot != excluded {
                let p = load_vec(e, e.mem.u32(slot));
                let q_address = e.mem.u32(e.mem.u32(slot + 4));
                let q = load_vec(e, q_address);
                let d = zip_with(q, p, |a, b| a - b);
                let d_squared = sum3(zip_with(d, d, |a, b| a * b));
                let normal = load_vec(e, current);
                let crossed = cross(d, normal);
                let f = sum3(zip_with(crossed, p, |a, b| a * b));
                let bound = f as f64 * f.abs() as f64;
                let threshold = (d_squared as f64 * tiny) as f32;
                if threshold as f64 > bound {
                    let neighbour = e.mem.u32(e.mem.u32(slot + 8) + 0xc);
                    let own = e.mem.f32(face.addr() + 0x40);
                    let theirs = e.mem.f32(neighbour + 0x40);
                    let positive = 0.0 < own;
                    let nearer = own as f64 * nine_tenths > theirs as f64;
                    if !positive || !nearer {
                        excluded = e.mem.u32(current + 0x18 + 0x10 * slot_index);
                        done = e.mem.u32(neighbour + 0x44) == 3;
                        current = neighbour;
                        break;
                    }
                }
            }
            slot_index += 1;
            slot += 0x10;
        }
        if slot_index == 3 {
            return Ptr::new(current);
        }
    }
}

// Translated from 00e2da40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Support point of the pair of shapes along a direction (custom
/// convention): `shape_a` is `ECX`, `matrix` `ESI` (four column vectors: the
/// rotation and the translation of `shape_b` relative to `shape_a`) and
/// `direction` `EDI`; the stack arguments are `shape_b` and `record`.
///
/// Asks `shape_a` (virtual call, slot +0x38) for its vertex along `-direction`
/// and `shape_b` for its vertex along the direction turned by the transpose
/// of the rotation, moves that vertex by the transform (`(y * c1 + x * c0) +
/// (z * c2 + c3)` per lane) and takes `s = (va - vb') . direction` (sum `z +
/// (y + x)`). If the record's float at +0x2c is below `s` (ordered), the
/// record is updated: +0x00 the vertex of `shape_a`, +0x10 the vertex of
/// `shape_b` (before the transform), +0x20 the direction, +0x2c `s`.
pub fn fn_00e2da40(
    e: &mut Engine,
    shape_a: Ptr,
    matrix: Ptr,
    direction: Ptr,
    shape_b: Ptr,
    record: Ptr,
) {
    e.with_stack(0x40, |e, frame| {
        let flipped = frame.addr();
        let rotated = frame.addr() + 0x10;
        let vertex_b = frame.addr() + 0x20;
        let vertex_a = frame.addr() + 0x30;
        let direction_now = load_vec(e, direction.addr());
        let backwards = negated(e, direction_now);
        store_vec(e, flipped, backwards);
        e.vcall(shape_a.addr(), 0x38, &args![flipped, vertex_a]);

        let c0 = load_vec(e, matrix.addr());
        let c1 = load_vec(e, matrix.addr() + 0x10);
        let c2 = load_vec(e, matrix.addr() + 0x20);
        let d = load_vec(e, direction.addr());
        // The transposed rows: the x, y, z of each column, with lane 3 taken
        // from c2's y and w.
        let row_x = [c0[0], c1[0], c2[0], c2[1]];
        let row_y = [c0[1], c1[1], c2[1], c2[3]];
        let row_z = [c0[2], c1[2], c2[2], c2[3]];
        let turned: Vec4 = std::array::from_fn(|lane| {
            (d[1] * row_y[lane] + d[0] * row_x[lane]) + d[2] * row_z[lane]
        });
        store_vec(e, rotated, turned);
        e.vcall(shape_b.addr(), 0x38, &args![rotated, vertex_b]);

        let b = load_vec(e, vertex_b);
        let c0 = load_vec(e, matrix.addr());
        let c1 = load_vec(e, matrix.addr() + 0x10);
        let c2 = load_vec(e, matrix.addr() + 0x20);
        let c3 = load_vec(e, matrix.addr() + 0x30);
        let direction_after = load_vec(e, direction.addr());
        let moved: Vec4 = std::array::from_fn(|lane| {
            (b[1] * c1[lane] + b[0] * c0[lane]) + (b[2] * c2[lane] + c3[lane])
        });
        let a = load_vec(e, vertex_a);
        let gap = zip_with(a, moved, |x, y| x - y);
        let score = sum3(zip_with(gap, direction_after, |x, y| x * y));
        // `FCOMP`: updates only when the record's float is below the score.
        if e.mem.f32(record.addr() + 0x2c) < score {
            store_vec(e, record.addr() + 0x20, direction_after);
            store_vec(e, record.addr(), a);
            store_vec(e, record.addr() + 0x10, b);
            e.mem.set_f32(record.addr() + 0x2c, x87_quiet(score));
        }
    });
}

// Translated from 00e2db70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Refines the pair of closest support points of two shapes (cdecl, six
/// stack arguments: the shapes `shape_a` and `shape_b`, the `matrix` of
/// `00e2da40`, and the outputs `out_a`, `out_b` and `out_c`).
///
/// A record of `00e2da40` starts with the float +0x2c at the lowest value
/// (`0104e148`). The three directions of the table at `01268370` and their
/// negations seed it (two calls each). Then, with a step `s` that starts at
/// 1.0, every round shrinks it (`s = s * 0.99`, the `double` at `0106d508`),
/// takes the record's direction `d`, builds a vector `u` perpendicular to
/// it (two lanes picked from the smallest absolute components, one negated),
/// the unit `un = u / |u|` (refined `RSQRTSS`), the cross product `d x un`,
/// and the four directions `d + s un`, `d - s un`, `s (d x un) + d` and
/// `d - s (d x un)`, each scaled by the same normalising factor (that of
/// the first); each goes through `00e2da40`. The step is then halved and the
/// loop goes on while it exceeds 0.001. Finally `out_a` gets the record's
/// +0x00 vector, `out_b` its +0x10 vector, and `out_c` the direction (+0x00,
/// lane 3 being the score), the +0x00 vector at +0x10 and the score at +0x20.
pub fn fn_00e2db70(
    e: &mut Engine,
    shape_a: Ptr,
    shape_b: Ptr,
    matrix: Ptr,
    out_a: Ptr,
    out_b: Ptr,
    out_c: Ptr,
) {
    e.with_stack(0x100, |e, frame| {
        let base = frame.addr();
        let record = Ptr::<()>::new(base + 0x40);
        let start = x87_quiet(e.global::<f32>(MIN_FLOAT));
        e.mem.set_f32(base + 0x6c, start);
        for k in 0..3u32 {
            copy_vec(e, START_DIRECTIONS + 0x10 * k, base + 0x20);
            e.call(
                SUPPORT_POINT,
                &args![shape_a, matrix, base + 0x20, shape_b, record],
            );
            let flipped = negated(e, load_vec(e, base + 0x20));
            store_vec(e, base + 0x20, flipped);
            e.call(
                SUPPORT_POINT,
                &args![shape_a, matrix, base + 0x20, shape_b, record],
            );
        }

        let shrink: f64 = e.global(SHRINK);
        let half: f64 = e.global(HALF_DOUBLE);
        let mut step = 1.0f32;
        loop {
            step = (step as f64 * shrink) as f32;
            let d = load_vec(e, base + 0x60);
            let abs_x = d[0].abs();
            let abs_y = d[1].abs();
            let abs_z = d[2].abs();
            let mut first = 1usize;
            let mut second = 2usize;
            let mut third = 0usize;
            let mut smallest = abs_x;
            if abs_x > abs_y {
                smallest = abs_y;
                first = 0;
                third = 1;
            }
            if smallest > abs_z {
                second = third;
            }
            let mut u = [0.0f32; 4];
            u[first] = x87_quiet(d[second]);
            u[second] = x87_negated(d[first]);
            let u_sum = sum3(zip_with(u, u, |a, b| a * b));
            let u_scale = inverse_length(e, u_sum);
            let unit_u: Vec4 = u.map(|lane| u_scale * lane);
            let scaled_u: Vec4 = unit_u.map(|lane| step * lane);
            let crossed = cross(d, unit_u);
            let scaled_cross: Vec4 = crossed.map(|lane| lane * step);
            let plus = zip_with(d, scaled_u, |a, b| a + b);
            let plus_sum = sum3(zip_with(plus, plus, |a, b| a * b));
            let scale = inverse_length(e, plus_sum);
            let directions: [Vec4; 4] = [
                plus.map(|lane| scale * lane),
                zip_with(d, scaled_u, |a, b| a - b).map(|lane| lane * scale),
                zip_with(scaled_cross, d, |a, b| a + b).map(|lane| lane * scale),
                zip_with(d, scaled_cross, |a, b| a - b).map(|lane| lane * scale),
            ];
            let slots = [0xb0u32, 0x90, 0xa0, 0xc0];
            for (direction, slot) in directions.iter().zip(slots) {
                store_vec(e, base + slot, *direction);
            }
            for slot in slots {
                e.call(
                    SUPPORT_POINT,
                    &args![shape_a, matrix, base + slot, shape_b, record],
                );
            }
            step = (step as f64 * half) as f32;
            // `FCOM`: the loop goes on only while the step is above the limit.
            let limit: f32 = e.global(SMALL_STEP);
            let go_on = step > limit;
            if !go_on {
                break;
            }
        }
        copy_vec(e, base + 0x40, out_a.addr());
        copy_vec(e, base + 0x50, out_b.addr());
        let best = x87_quiet(e.mem.f32(base + 0x6c));
        e.mem.set_f32(out_c.addr() + 0x20, best);
        copy_vec(e, base + 0x60, out_c.addr());
        copy_vec(e, base + 0x40, out_c.addr() + 0x10);
    });
}

// Translated from 00e2b9b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map: keeps the `store`'s best entries by key. `store` (the third word)
/// is a list of 0x60-byte entries: `+8` the array, `+0xc` the write pointer, `+0x10` the
/// capacity, `+0x14` the number in use, `+0x18` an override pointer. `candidate` has the float
/// key at `+0x10`. While the list has room the entry at the write pointer is used (the pointer
/// advances by 0x60, the count by one). When it is full, the entry with the largest key (the
/// first of equals; compares as `FCOMPP`: a NaN never wins) is the victim, and the function
/// returns when its key is not strictly above the candidate's (a NaN also returns). The entry
/// gets the candidate's key (through the x87 stack, `+0x10`), its first 16 bytes (`+0`), its
/// word at `+0x14` and, at `+0x50`, the override pointer or, when it is zero, the last node of
/// the chain that starts at `chain` (the nodes link through `+0xc`). Finally the chain's length
/// `n` (the number of links after `chain`) is counted, the entry's word `n` at `+0x20` becomes
/// `0xffffffff` and the words `n - 1` down to 0 get `[node + 4]` of the chain's first `n`
/// nodes. The function takes three stack words and pops none (`RET`) and returns nothing.
pub fn fn_00e2b9b0(e: &mut Engine, chain: Ptr, candidate: Ptr, store: Ptr) {
    let (chain, candidate, store) = (chain.addr(), candidate.addr(), store.addr());
    let count = e.mem.i32(store + 0x14);
    let entry = if count < e.mem.i32(store + 0x10) {
        let entry = e.mem.u32(store + 0xc);
        e.mem.set_i32(store + 0x14, count.wrapping_add(1));
        e.mem.set_u32(store + 0xc, entry.wrapping_add(0x60));
        entry
    } else {
        let mut best = e.mem.u32(store + 8);
        if count > 1 {
            let mut other = best.wrapping_add(0x60);
            let mut remaining = count - 1;
            loop {
                let other_key = x87_load32(e.mem.u32(other + 0x10));
                let best_key = x87_load32(e.mem.u32(best + 0x10));
                if best_key < other_key {
                    best = other;
                }
                other = other.wrapping_add(0x60);
                remaining -= 1;
                if remaining == 0 {
                    break;
                }
            }
        }
        let new_key = x87_load32(e.mem.u32(candidate + 0x10));
        let best_key = x87_load32(e.mem.u32(best + 0x10));
        if best_key.partial_cmp(&new_key) != Some(std::cmp::Ordering::Greater) {
            return;
        }
        best
    };
    let key = x87_quiet_bits(e.mem.u32(candidate + 0x10));
    e.mem.set_u32(entry + 0x10, key);
    for lane in 0..4 {
        let word = e.mem.u32(candidate + 4 * lane);
        e.mem.set_u32(entry + 4 * lane, word);
    }
    let word = e.mem.u32(candidate + 0x14);
    e.mem.set_u32(entry + 0x14, word);
    let override_pointer = e.mem.u32(store + 0x18);
    if override_pointer != 0 {
        e.mem.set_u32(entry + 0x50, override_pointer);
    } else {
        let mut last = chain;
        let mut link = e.mem.u32(chain + 0xc);
        if link != 0 {
            loop {
                last = link;
                link = e.mem.u32(last + 0xc);
                if link == 0 {
                    break;
                }
            }
        }
        e.mem.set_u32(entry + 0x50, last);
    }
    let mut length: u32 = 0;
    let mut link = e.mem.u32(chain + 0xc);
    if link != 0 {
        loop {
            link = e.mem.u32(link + 0xc);
            length += 1;
            if link == 0 {
                break;
            }
        }
    }
    e.mem.set_u32(entry + length * 4 + 0x20, 0xffff_ffff);
    let mut index = length as i32 - 1;
    if index >= 0 {
        let mut node = chain;
        let mut slot = entry
            .wrapping_add((index as u32).wrapping_mul(4))
            .wrapping_add(0x20);
        loop {
            let word = e.mem.u32(node + 4);
            e.mem.set_u32(slot, word);
            node = e.mem.u32(node + 0xc);
            index -= 1;
            slot = slot.wrapping_sub(4);
            if index < 0 {
                break;
            }
        }
    }
}

// Translated from 00e2baa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map: initialises the record `object` from the record `source` and a
/// word `extra`: `1.0f` (through the x87 stack) at `+0x14`, the byte 0 at `+0x18`, `source`
/// itself at `+0x1c` and `+0x20`, `[source + 0x30]` at `+0x24` and `+0x28`, `[source + 0x24]` at
/// `+0x50`, and at `+0x54` the word `extra + 0x10` when the byte at `source + 0x20` is not zero
/// and `extra` is not zero, else 0. The function takes three stack words and pops none (`RET`)
/// and returns nothing.
pub fn fn_00e2baa0(e: &mut Engine, object: Ptr, source: Ptr, extra: u32) {
    let (object, source) = (object.addr(), source.addr());
    e.mem.set_u32(object + 0x14, 0x3f80_0000);
    e.mem.set_u8(object + 0x18, 0);
    let first = e.mem.u32(source + 0x30);
    e.mem.set_u32(object + 0x24, first);
    let second = e.mem.u32(source + 0x30);
    e.mem.set_u32(object + 0x20, source);
    e.mem.set_u32(object + 0x28, second);
    let third = e.mem.u32(source + 0x24);
    e.mem.set_u32(object + 0x1c, source);
    e.mem.set_u32(object + 0x50, third);
    let selected = if e.mem.i8(source + 0x20) != 0 {
        extra
    } else {
        0
    };
    if selected != 0 {
        e.mem.set_u32(object + 0x54, selected.wrapping_add(0x10));
    } else {
        e.mem.set_u32(object + 0x54, 0);
    }
}

// Translated from 00e2baf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map: transposes eight vectors into six (pure SSE shuffles, nothing is
/// computed). `a` .. `h` are the vectors at the eight pointers (stack words 2 to 9) and `tail`
/// the tenth word: `out[0]` = x of `a`, `c`, `e`, `g`; `out[1]` = their y; `out[2]` = their z;
/// `out[3]` = x of `b`, `d`, `f`, `h`; `out[4]` = their y; `out[5]` = their z; and the word at
/// `out + 0x60` is `tail`. All eight vectors are loaded before anything is stored. The function
/// takes ten stack words and pops none (`RET`) and returns nothing.
#[allow(clippy::too_many_arguments)]
pub fn fn_00e2baf0(
    e: &mut Engine,
    out: Ptr,
    a: Ptr,
    b: Ptr,
    c: Ptr,
    d: Ptr,
    e_vector: Ptr,
    f: Ptr,
    g: Ptr,
    h: Ptr,
    tail: u32,
) {
    let load = |e: &Engine, at: Ptr| -> [u32; 4] {
        std::array::from_fn(|lane| e.mem.u32(at.addr() + 4 * lane as u32))
    };
    let (a, b, c, d) = (load(e, a), load(e, b), load(e, c), load(e, d));
    let (e_lanes, f, g, h) = (load(e, e_vector), load(e, f), load(e, g), load(e, h));
    let rows = [
        [a[0], c[0], e_lanes[0], g[0]],
        [a[1], c[1], e_lanes[1], g[1]],
        [a[2], c[2], e_lanes[2], g[2]],
        [b[0], d[0], f[0], h[0]],
        [b[1], d[1], f[1], h[1]],
        [b[2], d[2], f[2], h[2]],
    ];
    for (row, lanes) in rows.into_iter().enumerate() {
        for (lane, word) in lanes.into_iter().enumerate() {
            e.mem
                .set_u32(out.addr() + 0x10 * row as u32 + 4 * lane as u32, word);
        }
    }
    e.mem.set_u32(out.addr() + 0x60, tail);
}

// Translated from 00e2b070 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map (the same function as `00e2b510` with other frame offsets and the
/// callee `00d1d530`). Arguments (cdecl, four stack words, nothing popped): `owner` (nothing
/// happens when it is null), a `source` record, a word `value` and a four-float `vector`. It
/// builds a `0x3a4`-byte stack frame: `00d1bb50` (ten stack words, the last `0xf`) converts eight
/// vectors of `source` (at `+0x00`, `+0x10`, `+0x30`, `+0x40`, `+0x60`, `+0x70`, `+0x90` and
/// `+0xa0`) into the frame; four 0x80-byte local objects get their vtable words (`010dbacc`,
/// `010dbae4`, `010dbad8`, `010dbaf8`), `1.0f`, `value` and records copied from `source`
/// (`+0xc0..+0xd4` and the words of the four sub-records); a four-pointer array refers to
/// them; the vector is copied from `vector`. Then `00d1d530` (cdecl, six words: `owner`,
/// `[owner + 0x10]`, 0, the converted block, the pointer array and the vector copy) runs, four
/// words of the frame go to `source + 0xe0..+0xec` and the vector copy back to `vector`
/// (interleaved by `UNPCKLPS`); the closing loop writes the vtable words of four destroyed
/// locals into the frame that is about to be discarded.
///
/// A register-level translation of straight-line code: each statement ends with the address of
/// the instruction it comes from; `frame` is the address of what the exe's `ESP` points at after
/// its prologue (the saved registers) and the offsets are relative to it, taking the words
/// still pushed for the unpopped cdecl calls into account.
#[allow(unused_assignments)]
pub fn fn_00e2b070(e: &mut Engine, owner: Ptr, source: Ptr, value: u32, vector: Ptr) {
    let (arg0, arg1, arg2, arg3) = (owner.addr(), source.addr(), value, vector.addr());
    if arg0 == 0 {
        return;
    }
    e.with_stack(0x420, |e, buffer| {
        let frame = buffer.addr() + 0x60;
        let (mut eax, mut ecx, mut edx, mut esi, mut edi) = (0u32, 0u32, 0u32, 0u32, 0u32);
        let ebx = 0u32;
        let mut xmm = [[0u32; 4]; 8];
        let mut st0 = 0u32;
        esi = arg1; // 00e2b08a
        let pushed0 = 0xf;
        e.mem.set_u32(frame - 0x4, pushed0); // 00e2b08d
        edx = esi.wrapping_add(0xa0); // 00e2b08f
        let pushed1 = edx;
        e.mem.set_u32(frame - 0x8, pushed1); // 00e2b095
        edi = esi.wrapping_add(0x90); // 00e2b096
        let pushed2 = edi;
        e.mem.set_u32(frame - 0xc, pushed2); // 00e2b09c
        edx = esi.wrapping_add(0x70); // 00e2b09d
        let pushed3 = edx;
        e.mem.set_u32(frame - 0x10, pushed3); // 00e2b0a0
        eax = esi.wrapping_add(0x60); // 00e2b0a1
        let pushed4 = eax;
        e.mem.set_u32(frame - 0x14, pushed4); // 00e2b0a4
        eax = esi.wrapping_add(0x40); // 00e2b0a5
        let pushed5 = eax;
        e.mem.set_u32(frame - 0x18, pushed5); // 00e2b0a8
        ecx = esi.wrapping_add(0x30); // 00e2b0a9
        let pushed6 = ecx;
        e.mem.set_u32(frame - 0x1c, pushed6); // 00e2b0ac
        eax = esi.wrapping_add(0x10); // 00e2b0ad
        let pushed7 = eax;
        e.mem.set_u32(frame - 0x20, pushed7); // 00e2b0b0
        ecx = frame + 0x340; // 00e2b0b1
        let pushed8 = esi;
        e.mem.set_u32(frame - 0x24, pushed8); // 00e2b0b8
        let pushed9 = ecx;
        e.mem.set_u32(frame - 0x28, pushed9); // 00e2b0b9
        e.call(
            0x00d1bb50,
            &args![
                pushed9, pushed8, pushed7, pushed6, pushed5, pushed4, pushed3, pushed2, pushed1,
                pushed0
            ],
        ); // 00e2b0ba
        st0 = 0x3f80_0000; // 00e2b0bf
        xmm[0] = load_lanes(e, esi); // 00e2b0c1
        e.mem.set_u32(frame + 0x144, st0); // 00e2b0c4
        eax = 0x10dbacc; // 00e2b0cb
        e.mem.set_u32(frame + 0x1c4, st0); // 00e2b0d0
        edx = 0x10dbae4; // 00e2b0d7
        e.mem.set_u32(frame + 0x244, st0); // 00e2b0dc
        e.mem.set_u32(frame + 0x13c, eax); // 00e2b0e3
        e.mem.set_u32(frame + 0x2c4, st0); // 00e2b0ea
        e.mem.set_u32(frame + 0x1bc, eax); // 00e2b0f1
        e.mem.set_u32(frame + 0x23c, eax); // 00e2b0f8
        e.mem.set_u32(frame + 0x2bc, eax); // 00e2b0ff
        eax = e.mem.u32(esi.wrapping_add(0xc8)); // 00e2b106
        ecx = 0x10dbad8; // 00e2b10c
        e.mem.set_u32(frame + 0x60, eax); // 00e2b111
        e.mem.set_u32(frame + 0x154, eax); // 00e2b118
        e.mem.set_u32(frame + 0x158, eax); // 00e2b11f
        eax = e.mem.u32(esi.wrapping_add(0x24)); // 00e2b126
        e.mem.set_u32(frame + 0x134, edx); // 00e2b129
        e.mem.set_u32(frame + 0x1b4, edx); // 00e2b130
        e.mem.set_u32(frame + 0x234, edx); // 00e2b137
        e.mem.set_u32(frame + 0x2b4, edx); // 00e2b13e
        edx = u32::from(e.mem.u8(esi.wrapping_add(0x20))); // 00e2b145
        e.mem.set_u32(frame + 0x54, eax); // 00e2b149
        eax = (eax & 0xffff_ff00) | u32::from(e.mem.u8(esi.wrapping_add(0xc4))); // 00e2b14d
        store_lanes(e, frame + 0x30, xmm[0]); // 00e2b153
        xmm[0] = load_lanes(e, esi.wrapping_add(0x10)); // 00e2b158
        e.mem.set_u32(frame + 0x138, ecx); // 00e2b15c
        e.mem.set_u32(frame + 0x1b8, ecx); // 00e2b163
        e.mem.set_u32(frame + 0x238, ecx); // 00e2b16a
        e.mem.set_u32(frame + 0x2b8, ecx); // 00e2b171
        ecx = arg2; // 00e2b178
        e.mem.set_u8(frame + 0x6c, ebx as u8); // 00e2b17b
        e.mem.set_u8(frame + 0xac, ebx as u8); // 00e2b182
        e.mem.set_u8(frame + 0xec, ebx as u8); // 00e2b189
        e.mem.set_u8(frame + 0x12c, ebx as u8); // 00e2b190
        e.mem.set_u32(frame + 0x130, 0x10dbaf8); // 00e2b197
        e.mem.set_u32(frame + 0x180, ebx); // 00e2b1a2
        e.mem.set_u32(frame + 0x184, ebx); // 00e2b1a9
        e.mem.set_u32(frame + 0x190, ebx); // 00e2b1b0
        e.mem.set_u8(frame + 0x148, ebx as u8); // 00e2b1b7
        e.mem.set_u32(frame + 0x1b0, 0x10dbaf8); // 00e2b1be
        e.mem.set_u32(frame + 0x200, ebx); // 00e2b1c9
        e.mem.set_u32(frame + 0x204, ebx); // 00e2b1d0
        e.mem.set_u32(frame + 0x210, ebx); // 00e2b1d7
        e.mem.set_u8(frame + 0x1c8, ebx as u8); // 00e2b1de
        e.mem.set_u32(frame + 0x230, 0x10dbaf8); // 00e2b1e5
        e.mem.set_u32(frame + 0x280, ebx); // 00e2b1f0
        e.mem.set_u32(frame + 0x284, ebx); // 00e2b1f7
        e.mem.set_u32(frame + 0x290, ebx); // 00e2b1fe
        e.mem.set_u8(frame + 0x248, ebx as u8); // 00e2b205
        e.mem.set_u32(frame + 0x2b0, 0x10dbaf8); // 00e2b20c
        e.mem.set_u32(frame + 0x300, ebx); // 00e2b217
        e.mem.set_u32(frame + 0x304, ebx); // 00e2b21e
        e.mem.set_u32(frame + 0x310, ebx); // 00e2b225
        e.mem.set_u8(frame + 0x2c8, ebx as u8); // 00e2b22c
        store_lanes(e, frame + 0x40, xmm[0]); // 00e2b233
        e.mem.set_u8(frame + 0x50, edx as u8); // 00e2b238
        e.mem.set_u8(frame + 0x6d, eax as u8); // 00e2b23c
        e.mem.set_u32(frame + 0x64, 0x1); // 00e2b243
        e.mem.set_u32(frame + 0x68, ebx); // 00e2b24e
        edx = frame + 0x30; // 00e2b255
        e.mem.set_u32(frame + 0x150, edx); // 00e2b259
        e.mem.set_u32(frame + 0x140, ecx); // 00e2b260
        ecx = edx; // 00e2b267
        edx = frame + 0x138; // 00e2b269
        e.mem.set_u32(frame + 0x20, edx); // 00e2b270
        edx = e.mem.u32(esi.wrapping_add(0xcc)); // 00e2b274
        e.mem.set_u32(frame + 0xa0, edx); // 00e2b27a
        e.mem.set_u32(frame + 0x1d4, edx); // 00e2b281
        e.mem.set_u32(frame + 0x1d8, edx); // 00e2b288
        edx = u32::from(e.mem.u8(esi.wrapping_add(0x50))); // 00e2b28f
        e.mem.set_u8(frame + 0x90, edx as u8); // 00e2b293
        edx = e.mem.u32(esi.wrapping_add(0x54)); // 00e2b29a
        e.mem.set_u32(frame + 0x94, edx); // 00e2b29d
        edx = arg2; // 00e2b2a4
        e.mem.set_u32(frame + 0x1c0, edx); // 00e2b2a7
        xmm[0] = load_lanes(e, esi.wrapping_add(0x30)); // 00e2b2ae
        edx = frame + 0x70; // 00e2b2b2
        e.mem.set_u32(frame + 0x1d0, edx); // 00e2b2b9
        e.mem.set_u32(frame + 0x1cc, edx); // 00e2b2c0
        edx = frame + 0x1b8; // 00e2b2c7
        e.mem.set_u32(frame + 0x24, edx); // 00e2b2ce
        edx = e.mem.u32(esi.wrapping_add(0xd0)); // 00e2b2d2
        e.mem.set_u32(frame + 0xe0, edx); // 00e2b2d8
        e.mem.set_u32(frame + 0x254, edx); // 00e2b2df
        e.mem.set_u32(frame + 0x258, edx); // 00e2b2e6
        edx = u32::from(e.mem.u8(esi.wrapping_add(0x80))); // 00e2b2ed
        e.mem.set_u8(frame + 0xd0, edx as u8); // 00e2b2f4
        edx = e.mem.u32(esi.wrapping_add(0x84)); // 00e2b2fb
        e.mem.set_u32(frame + 0xd4, edx); // 00e2b301
        edx = arg2; // 00e2b308
        e.mem.set_u32(frame + 0x240, edx); // 00e2b30b
        edx = frame + 0xb0; // 00e2b312
        e.mem.set_u32(frame + 0x250, edx); // 00e2b319
        e.mem.set_u32(frame + 0x24c, edx); // 00e2b320
        edx = frame + 0x238; // 00e2b327
        e.mem.set_u32(frame + 0x28, edx); // 00e2b32e
        edx = e.mem.u32(esi.wrapping_add(0xd4)); // 00e2b332
        e.mem.set_u32(frame + 0x120, edx); // 00e2b338
        e.mem.set_u32(frame + 0x2d4, edx); // 00e2b33f
        e.mem.set_u32(frame + 0x2d8, edx); // 00e2b346
        edx = u32::from(e.mem.u8(edi.wrapping_add(0x20))); // 00e2b34d
        e.mem.set_u32(frame + 0x14c, ecx); // 00e2b351
        ecx = arg3; // 00e2b358
        store_lanes(e, frame + 0x70, xmm[0]); // 00e2b35b
        xmm[0] = load_lanes(e, esi.wrapping_add(0x40)); // 00e2b363
        st0 = x87_quiet_bits(e.mem.u32(ecx)); // 00e2b367
        store_lanes(e, frame + 0x80, xmm[0]); // 00e2b369
        xmm[0] = load_lanes(e, esi.wrapping_add(0x60)); // 00e2b371
        e.mem.set_u32(frame + 0x10, st0); // 00e2b375
        st0 = x87_quiet_bits(e.mem.u32(ecx.wrapping_add(0x4))); // 00e2b379
        e.mem.set_u8(frame + 0x110, edx as u8); // 00e2b37c
        edx = e.mem.u32(edi.wrapping_add(0x24)); // 00e2b383
        e.mem.set_u32(frame + 0x14, st0); // 00e2b386
        st0 = x87_quiet_bits(e.mem.u32(ecx.wrapping_add(0x8))); // 00e2b38a
        store_lanes(e, frame + 0xb0, xmm[0]); // 00e2b38d
        xmm[0] = load_lanes(e, esi.wrapping_add(0x70)); // 00e2b395
        e.mem.set_u32(frame + 0x18, st0); // 00e2b399
        st0 = x87_quiet_bits(e.mem.u32(ecx.wrapping_add(0xc))); // 00e2b39d
        e.mem.set_u8(frame + 0xad, eax as u8); // 00e2b3a0
        e.mem.set_u8(frame + 0xed, eax as u8); // 00e2b3a7
        e.mem.set_u32(frame + 0x1c, st0); // 00e2b3ae
        e.mem.set_u8(frame + 0x12d, eax as u8); // 00e2b3b2
        eax = arg2; // 00e2b3b9
        store_lanes(e, frame + 0xc0, xmm[0]); // 00e2b3bc
        xmm[0] = load_lanes(e, edi); // 00e2b3c4
        e.mem.set_u32(frame + 0x114, edx); // 00e2b3c7
        edx = frame + 0xf0; // 00e2b3ce
        e.mem.set_u32(frame + 0x2c0, eax); // 00e2b3d5
        store_lanes(e, frame + 0xf0, xmm[0]); // 00e2b3dc
        xmm[0] = load_lanes(e, edi.wrapping_add(0x10)); // 00e2b3e4
        eax = edx; // 00e2b3e8
        e.mem.set_u32(frame + 0xa4, 0x1); // 00e2b3ea
        e.mem.set_u32(frame + 0xa8, ebx); // 00e2b3f5
        e.mem.set_u32(frame + 0xe4, 0x1); // 00e2b3fc
        e.mem.set_u32(frame + 0xe8, ebx); // 00e2b407
        store_lanes(e, frame + 0x100, xmm[0]); // 00e2b40e
        e.mem.set_u32(frame + 0x124, 0x1); // 00e2b416
        e.mem.set_u32(frame + 0x128, ebx); // 00e2b421
        e.mem.set_u32(frame + 0x2d0, edx); // 00e2b428
        e.mem.set_u32(frame + 0x2cc, eax); // 00e2b42f
        edx = e.mem.u32(esi.wrapping_add(0xc0)); // 00e2b436
        eax = e.mem.u32(edx.wrapping_mul(4).wrapping_add(0x11b3830)); // 00e2b43c
        ecx = frame + 0x2b8; // 00e2b443
        e.mem.set_u32(frame + 0x2c, ecx); // 00e2b44a
        ecx = frame + 0x10; // 00e2b44e
        let pushed10 = ecx;
        e.mem.set_u32(frame - 0x2c, pushed10); // 00e2b452
        e.mem.set_u32(frame + 0x3a0, eax); // 00e2b453
        edx = frame + 0x20; // 00e2b45a
        let pushed11 = edx;
        e.mem.set_u32(frame - 0x30, pushed11); // 00e2b45e
        eax = frame + 0x340; // 00e2b45f
        let pushed12 = eax;
        e.mem.set_u32(frame - 0x34, pushed12); // 00e2b466
        eax = arg0; // 00e2b467
        ecx = e.mem.u32(eax.wrapping_add(0x10)); // 00e2b46a
        let pushed13 = ebx;
        e.mem.set_u32(frame - 0x38, pushed13); // 00e2b46d
        let pushed14 = ecx;
        e.mem.set_u32(frame - 0x3c, pushed14); // 00e2b46e
        let pushed15 = eax;
        e.mem.set_u32(frame - 0x40, pushed15); // 00e2b46f
        e.call(
            0x00d1d530,
            &args![pushed15, pushed14, pushed13, pushed12, pushed11, pushed10],
        ); // 00e2b470
        xmm[0] = [e.mem.u32(frame + 0x10), 0, 0, 0]; // 00e2b475
        xmm[1] = [e.mem.u32(frame + 0x14), 0, 0, 0]; // 00e2b47b
        xmm[2] = [e.mem.u32(frame + 0x18), 0, 0, 0]; // 00e2b481
        xmm[3] = [e.mem.u32(frame + 0x1c), 0, 0, 0]; // 00e2b487
        edx = e.mem.u32(frame + 0x68); // 00e2b48d
        eax = e.mem.u32(frame + 0xa8); // 00e2b494
        ecx = e.mem.u32(frame + 0xe8); // 00e2b49b
        e.mem.set_u32(esi.wrapping_add(0xe0), edx); // 00e2b4a2
        edx = e.mem.u32(frame + 0x128); // 00e2b4a8
        e.mem.set_u32(esi.wrapping_add(0xe4), eax); // 00e2b4af
        eax = arg3; // 00e2b4b5
        e.mem.set_u32(esi.wrapping_add(0xe8), ecx); // 00e2b4b8
        e.mem.set_u32(esi.wrapping_add(0xec), edx); // 00e2b4be
        xmm[0] = [xmm[0][0], xmm[2][0], xmm[0][1], xmm[2][1]]; // 00e2b4c4
        xmm[1] = [xmm[1][0], xmm[3][0], xmm[1][1], xmm[3][1]]; // 00e2b4c7
        xmm[0] = [xmm[0][0], xmm[1][0], xmm[0][1], xmm[1][1]]; // 00e2b4ca
        store_lanes(e, eax, xmm[0]); // 00e2b4cd
                                     // ADD ESP,0x40 00e2b4d0
                                     // The exe's closing loop: four stores of vtable words into the dead locals.
        ecx = ebx.wrapping_add(3);
        eax = frame + 0x338;
        edi = 0x010c_cfac;
        esi = 0x010c_cf8c;
        edx = 0x010c_3bb8;
        loop {
            eax = eax.wrapping_sub(0x80);
            ecx = ecx.wrapping_sub(1);
            e.mem.set_u32(eax.wrapping_add(4), edi);
            e.mem.set_u32(eax, 0x010c_cfa0);
            e.mem.set_u32(eax.wrapping_sub(4), esi);
            e.mem.set_u32(eax.wrapping_sub(8), edx);
            if (ecx as i32) < 0 {
                break;
            }
        }
    });
}

// Translated from 00e2b510 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map (the same function as `00e2b070` with other frame offsets and the
/// callee `00d1d490`). Arguments (cdecl, four stack words, nothing popped): `owner` (nothing
/// happens when it is null), a `source` record, a word `value` and a four-float `vector`. It
/// builds a `0x3a4`-byte stack frame: `00d1bb50` (ten stack words, the last `0xf`) converts eight
/// vectors of `source` (at `+0x00`, `+0x10`, `+0x30`, `+0x40`, `+0x60`, `+0x70`, `+0x90` and
/// `+0xa0`) into the frame; four 0x80-byte local objects get their vtable words (`010dbacc`,
/// `010dbae4`, `010dbad8`, `010dbaf8`), `1.0f`, `value` and records copied from `source`
/// (`+0xc0..+0xd4` and the words of the four sub-records); a four-pointer array refers to
/// them; the vector is copied from `vector`. Then `00d1d490` (cdecl, six words: `owner`,
/// `[owner + 0x10]`, 0, the converted block, the pointer array and the vector copy) runs, four
/// words of the frame go to `source + 0xe0..+0xec` and the vector copy back to `vector`
/// (interleaved by `UNPCKLPS`); the closing loop writes the vtable words of four destroyed
/// locals into the frame that is about to be discarded.
///
/// A register-level translation of straight-line code: each statement ends with the address of
/// the instruction it comes from; `frame` is the address of what the exe's `ESP` points at after
/// its prologue (the saved registers) and the offsets are relative to it, taking the words
/// still pushed for the unpopped cdecl calls into account.
#[allow(unused_assignments)]
pub fn fn_00e2b510(e: &mut Engine, owner: Ptr, source: Ptr, value: u32, vector: Ptr) {
    let (arg0, arg1, arg2, arg3) = (owner.addr(), source.addr(), value, vector.addr());
    if arg0 == 0 {
        return;
    }
    e.with_stack(0x420, |e, buffer| {
        let frame = buffer.addr() + 0x60;
        let (mut eax, mut ecx, mut edx, mut esi, mut edi) = (0u32, 0u32, 0u32, 0u32, 0u32);
        let ebx = 0u32;
        let mut xmm = [[0u32; 4]; 8];
        let mut st0 = 0u32;
        esi = arg1; // 00e2b52a
        let pushed0 = 0xf;
        e.mem.set_u32(frame - 0x4, pushed0); // 00e2b52d
        edx = esi.wrapping_add(0xa0); // 00e2b52f
        let pushed1 = edx;
        e.mem.set_u32(frame - 0x8, pushed1); // 00e2b535
        edi = esi.wrapping_add(0x90); // 00e2b536
        let pushed2 = edi;
        e.mem.set_u32(frame - 0xc, pushed2); // 00e2b53c
        edx = esi.wrapping_add(0x70); // 00e2b53d
        let pushed3 = edx;
        e.mem.set_u32(frame - 0x10, pushed3); // 00e2b540
        eax = esi.wrapping_add(0x60); // 00e2b541
        let pushed4 = eax;
        e.mem.set_u32(frame - 0x14, pushed4); // 00e2b544
        eax = esi.wrapping_add(0x40); // 00e2b545
        let pushed5 = eax;
        e.mem.set_u32(frame - 0x18, pushed5); // 00e2b548
        ecx = esi.wrapping_add(0x30); // 00e2b549
        let pushed6 = ecx;
        e.mem.set_u32(frame - 0x1c, pushed6); // 00e2b54c
        eax = esi.wrapping_add(0x10); // 00e2b54d
        let pushed7 = eax;
        e.mem.set_u32(frame - 0x20, pushed7); // 00e2b550
        ecx = frame + 0x340; // 00e2b551
        let pushed8 = esi;
        e.mem.set_u32(frame - 0x24, pushed8); // 00e2b558
        let pushed9 = ecx;
        e.mem.set_u32(frame - 0x28, pushed9); // 00e2b559
        e.call(
            0x00d1bb50,
            &args![
                pushed9, pushed8, pushed7, pushed6, pushed5, pushed4, pushed3, pushed2, pushed1,
                pushed0
            ],
        ); // 00e2b55a
        st0 = 0x3f80_0000; // 00e2b55f
        xmm[0] = load_lanes(e, esi); // 00e2b561
        e.mem.set_u32(frame + 0x144, st0); // 00e2b564
        eax = 0x10dbacc; // 00e2b56b
        e.mem.set_u32(frame + 0x1c4, st0); // 00e2b570
        edx = 0x10dbae4; // 00e2b577
        e.mem.set_u32(frame + 0x244, st0); // 00e2b57c
        e.mem.set_u32(frame + 0x13c, eax); // 00e2b583
        e.mem.set_u32(frame + 0x2c4, st0); // 00e2b58a
        e.mem.set_u32(frame + 0x1bc, eax); // 00e2b591
        e.mem.set_u32(frame + 0x23c, eax); // 00e2b598
        e.mem.set_u32(frame + 0x2bc, eax); // 00e2b59f
        eax = e.mem.u32(esi.wrapping_add(0xc8)); // 00e2b5a6
        ecx = 0x10dbad8; // 00e2b5ac
        e.mem.set_u32(frame + 0x60, eax); // 00e2b5b1
        e.mem.set_u32(frame + 0x154, eax); // 00e2b5b8
        e.mem.set_u32(frame + 0x158, eax); // 00e2b5bf
        eax = e.mem.u32(esi.wrapping_add(0x24)); // 00e2b5c6
        e.mem.set_u32(frame + 0x134, edx); // 00e2b5c9
        e.mem.set_u32(frame + 0x1b4, edx); // 00e2b5d0
        e.mem.set_u32(frame + 0x234, edx); // 00e2b5d7
        e.mem.set_u32(frame + 0x2b4, edx); // 00e2b5de
        edx = u32::from(e.mem.u8(esi.wrapping_add(0x20))); // 00e2b5e5
        e.mem.set_u32(frame + 0x54, eax); // 00e2b5e9
        eax = (eax & 0xffff_ff00) | u32::from(e.mem.u8(esi.wrapping_add(0xc4))); // 00e2b5ed
        store_lanes(e, frame + 0x30, xmm[0]); // 00e2b5f3
        xmm[0] = load_lanes(e, esi.wrapping_add(0x10)); // 00e2b5f8
        e.mem.set_u32(frame + 0x138, ecx); // 00e2b5fc
        e.mem.set_u32(frame + 0x1b8, ecx); // 00e2b603
        e.mem.set_u32(frame + 0x238, ecx); // 00e2b60a
        e.mem.set_u32(frame + 0x2b8, ecx); // 00e2b611
        ecx = arg2; // 00e2b618
        e.mem.set_u8(frame + 0x6c, ebx as u8); // 00e2b61b
        e.mem.set_u8(frame + 0xac, ebx as u8); // 00e2b622
        e.mem.set_u8(frame + 0xec, ebx as u8); // 00e2b629
        e.mem.set_u8(frame + 0x12c, ebx as u8); // 00e2b630
        e.mem.set_u32(frame + 0x130, 0x10dbaf8); // 00e2b637
        e.mem.set_u32(frame + 0x180, ebx); // 00e2b642
        e.mem.set_u32(frame + 0x184, ebx); // 00e2b649
        e.mem.set_u32(frame + 0x190, ebx); // 00e2b650
        e.mem.set_u8(frame + 0x148, ebx as u8); // 00e2b657
        e.mem.set_u32(frame + 0x1b0, 0x10dbaf8); // 00e2b65e
        e.mem.set_u32(frame + 0x200, ebx); // 00e2b669
        e.mem.set_u32(frame + 0x204, ebx); // 00e2b670
        e.mem.set_u32(frame + 0x210, ebx); // 00e2b677
        e.mem.set_u8(frame + 0x1c8, ebx as u8); // 00e2b67e
        e.mem.set_u32(frame + 0x230, 0x10dbaf8); // 00e2b685
        e.mem.set_u32(frame + 0x280, ebx); // 00e2b690
        e.mem.set_u32(frame + 0x284, ebx); // 00e2b697
        e.mem.set_u32(frame + 0x290, ebx); // 00e2b69e
        e.mem.set_u8(frame + 0x248, ebx as u8); // 00e2b6a5
        e.mem.set_u32(frame + 0x2b0, 0x10dbaf8); // 00e2b6ac
        e.mem.set_u32(frame + 0x300, ebx); // 00e2b6b7
        e.mem.set_u32(frame + 0x304, ebx); // 00e2b6be
        e.mem.set_u32(frame + 0x310, ebx); // 00e2b6c5
        e.mem.set_u8(frame + 0x2c8, ebx as u8); // 00e2b6cc
        store_lanes(e, frame + 0x40, xmm[0]); // 00e2b6d3
        e.mem.set_u8(frame + 0x50, edx as u8); // 00e2b6d8
        e.mem.set_u8(frame + 0x6d, eax as u8); // 00e2b6dc
        e.mem.set_u32(frame + 0x64, 0x1); // 00e2b6e3
        e.mem.set_u32(frame + 0x68, ebx); // 00e2b6ee
        edx = frame + 0x30; // 00e2b6f5
        e.mem.set_u32(frame + 0x150, edx); // 00e2b6f9
        e.mem.set_u32(frame + 0x140, ecx); // 00e2b700
        ecx = edx; // 00e2b707
        edx = frame + 0x13c; // 00e2b709
        e.mem.set_u32(frame + 0x20, edx); // 00e2b710
        edx = e.mem.u32(esi.wrapping_add(0xcc)); // 00e2b714
        e.mem.set_u32(frame + 0xa0, edx); // 00e2b71a
        e.mem.set_u32(frame + 0x1d4, edx); // 00e2b721
        e.mem.set_u32(frame + 0x1d8, edx); // 00e2b728
        edx = u32::from(e.mem.u8(esi.wrapping_add(0x50))); // 00e2b72f
        e.mem.set_u8(frame + 0x90, edx as u8); // 00e2b733
        edx = e.mem.u32(esi.wrapping_add(0x54)); // 00e2b73a
        e.mem.set_u32(frame + 0x94, edx); // 00e2b73d
        edx = arg2; // 00e2b744
        e.mem.set_u32(frame + 0x1c0, edx); // 00e2b747
        xmm[0] = load_lanes(e, esi.wrapping_add(0x30)); // 00e2b74e
        edx = frame + 0x70; // 00e2b752
        e.mem.set_u32(frame + 0x1d0, edx); // 00e2b759
        e.mem.set_u32(frame + 0x1cc, edx); // 00e2b760
        edx = frame + 0x1bc; // 00e2b767
        e.mem.set_u32(frame + 0x24, edx); // 00e2b76e
        edx = e.mem.u32(esi.wrapping_add(0xd0)); // 00e2b772
        e.mem.set_u32(frame + 0xe0, edx); // 00e2b778
        e.mem.set_u32(frame + 0x254, edx); // 00e2b77f
        e.mem.set_u32(frame + 0x258, edx); // 00e2b786
        edx = u32::from(e.mem.u8(esi.wrapping_add(0x80))); // 00e2b78d
        e.mem.set_u8(frame + 0xd0, edx as u8); // 00e2b794
        edx = e.mem.u32(esi.wrapping_add(0x84)); // 00e2b79b
        e.mem.set_u32(frame + 0xd4, edx); // 00e2b7a1
        edx = arg2; // 00e2b7a8
        e.mem.set_u32(frame + 0x240, edx); // 00e2b7ab
        edx = frame + 0xb0; // 00e2b7b2
        e.mem.set_u32(frame + 0x250, edx); // 00e2b7b9
        e.mem.set_u32(frame + 0x24c, edx); // 00e2b7c0
        edx = frame + 0x23c; // 00e2b7c7
        e.mem.set_u32(frame + 0x28, edx); // 00e2b7ce
        edx = e.mem.u32(esi.wrapping_add(0xd4)); // 00e2b7d2
        e.mem.set_u32(frame + 0x120, edx); // 00e2b7d8
        e.mem.set_u32(frame + 0x2d4, edx); // 00e2b7df
        e.mem.set_u32(frame + 0x2d8, edx); // 00e2b7e6
        edx = u32::from(e.mem.u8(edi.wrapping_add(0x20))); // 00e2b7ed
        e.mem.set_u32(frame + 0x14c, ecx); // 00e2b7f1
        ecx = arg3; // 00e2b7f8
        store_lanes(e, frame + 0x70, xmm[0]); // 00e2b7fb
        xmm[0] = load_lanes(e, esi.wrapping_add(0x40)); // 00e2b803
        st0 = x87_quiet_bits(e.mem.u32(ecx)); // 00e2b807
        store_lanes(e, frame + 0x80, xmm[0]); // 00e2b809
        xmm[0] = load_lanes(e, esi.wrapping_add(0x60)); // 00e2b811
        e.mem.set_u32(frame + 0x10, st0); // 00e2b815
        st0 = x87_quiet_bits(e.mem.u32(ecx.wrapping_add(0x4))); // 00e2b819
        e.mem.set_u8(frame + 0x110, edx as u8); // 00e2b81c
        edx = e.mem.u32(edi.wrapping_add(0x24)); // 00e2b823
        e.mem.set_u32(frame + 0x14, st0); // 00e2b826
        st0 = x87_quiet_bits(e.mem.u32(ecx.wrapping_add(0x8))); // 00e2b82a
        store_lanes(e, frame + 0xb0, xmm[0]); // 00e2b82d
        xmm[0] = load_lanes(e, esi.wrapping_add(0x70)); // 00e2b835
        e.mem.set_u32(frame + 0x18, st0); // 00e2b839
        st0 = x87_quiet_bits(e.mem.u32(ecx.wrapping_add(0xc))); // 00e2b83d
        e.mem.set_u8(frame + 0xad, eax as u8); // 00e2b840
        e.mem.set_u8(frame + 0xed, eax as u8); // 00e2b847
        e.mem.set_u32(frame + 0x1c, st0); // 00e2b84e
        e.mem.set_u8(frame + 0x12d, eax as u8); // 00e2b852
        eax = arg2; // 00e2b859
        store_lanes(e, frame + 0xc0, xmm[0]); // 00e2b85c
        xmm[0] = load_lanes(e, edi); // 00e2b864
        e.mem.set_u32(frame + 0x114, edx); // 00e2b867
        edx = frame + 0xf0; // 00e2b86e
        e.mem.set_u32(frame + 0x2c0, eax); // 00e2b875
        store_lanes(e, frame + 0xf0, xmm[0]); // 00e2b87c
        xmm[0] = load_lanes(e, edi.wrapping_add(0x10)); // 00e2b884
        eax = edx; // 00e2b888
        e.mem.set_u32(frame + 0xa4, 0x1); // 00e2b88a
        e.mem.set_u32(frame + 0xa8, ebx); // 00e2b895
        e.mem.set_u32(frame + 0xe4, 0x1); // 00e2b89c
        e.mem.set_u32(frame + 0xe8, ebx); // 00e2b8a7
        store_lanes(e, frame + 0x100, xmm[0]); // 00e2b8ae
        e.mem.set_u32(frame + 0x124, 0x1); // 00e2b8b6
        e.mem.set_u32(frame + 0x128, ebx); // 00e2b8c1
        e.mem.set_u32(frame + 0x2d0, edx); // 00e2b8c8
        e.mem.set_u32(frame + 0x2cc, eax); // 00e2b8cf
        edx = e.mem.u32(esi.wrapping_add(0xc0)); // 00e2b8d6
        eax = e.mem.u32(edx.wrapping_mul(4).wrapping_add(0x11b3830)); // 00e2b8dc
        ecx = frame + 0x2bc; // 00e2b8e3
        e.mem.set_u32(frame + 0x2c, ecx); // 00e2b8ea
        ecx = frame + 0x10; // 00e2b8ee
        let pushed10 = ecx;
        e.mem.set_u32(frame - 0x2c, pushed10); // 00e2b8f2
        e.mem.set_u32(frame + 0x3a0, eax); // 00e2b8f3
        edx = frame + 0x20; // 00e2b8fa
        let pushed11 = edx;
        e.mem.set_u32(frame - 0x30, pushed11); // 00e2b8fe
        eax = frame + 0x340; // 00e2b8ff
        let pushed12 = eax;
        e.mem.set_u32(frame - 0x34, pushed12); // 00e2b906
        eax = arg0; // 00e2b907
        ecx = e.mem.u32(eax.wrapping_add(0x10)); // 00e2b90a
        let pushed13 = ebx;
        e.mem.set_u32(frame - 0x38, pushed13); // 00e2b90d
        let pushed14 = ecx;
        e.mem.set_u32(frame - 0x3c, pushed14); // 00e2b90e
        let pushed15 = eax;
        e.mem.set_u32(frame - 0x40, pushed15); // 00e2b90f
        e.call(
            0x00d1d490,
            &args![pushed15, pushed14, pushed13, pushed12, pushed11, pushed10],
        ); // 00e2b910
        xmm[0] = [e.mem.u32(frame + 0x10), 0, 0, 0]; // 00e2b915
        xmm[1] = [e.mem.u32(frame + 0x14), 0, 0, 0]; // 00e2b91b
        xmm[2] = [e.mem.u32(frame + 0x18), 0, 0, 0]; // 00e2b921
        xmm[3] = [e.mem.u32(frame + 0x1c), 0, 0, 0]; // 00e2b927
        edx = e.mem.u32(frame + 0x68); // 00e2b92d
        eax = e.mem.u32(frame + 0xa8); // 00e2b934
        ecx = e.mem.u32(frame + 0xe8); // 00e2b93b
        e.mem.set_u32(esi.wrapping_add(0xe0), edx); // 00e2b942
        edx = e.mem.u32(frame + 0x128); // 00e2b948
        e.mem.set_u32(esi.wrapping_add(0xe4), eax); // 00e2b94f
        eax = arg3; // 00e2b955
        e.mem.set_u32(esi.wrapping_add(0xe8), ecx); // 00e2b958
        e.mem.set_u32(esi.wrapping_add(0xec), edx); // 00e2b95e
        xmm[0] = [xmm[0][0], xmm[2][0], xmm[0][1], xmm[2][1]]; // 00e2b964
        xmm[1] = [xmm[1][0], xmm[3][0], xmm[1][1], xmm[3][1]]; // 00e2b967
        xmm[0] = [xmm[0][0], xmm[1][0], xmm[0][1], xmm[1][1]]; // 00e2b96a
        store_lanes(e, eax, xmm[0]); // 00e2b96d
                                     // ADD ESP,0x40 00e2b970
                                     // The exe's closing loop: four stores of vtable words into the dead locals.
        ecx = ebx.wrapping_add(3);
        eax = frame + 0x338;
        edi = 0x010c_cfac;
        esi = 0x010c_cf8c;
        edx = 0x010c_3bb8;
        loop {
            eax = eax.wrapping_sub(0x80);
            ecx = ecx.wrapping_sub(1);
            e.mem.set_u32(eax.wrapping_add(4), edi);
            e.mem.set_u32(eax, 0x010c_cfa0);
            e.mem.set_u32(eax.wrapping_sub(4), esi);
            e.mem.set_u32(eax.wrapping_sub(8), edx);
            if (ecx as i32) < 0 {
                break;
            }
        }
    });
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00e2cfe0, fn_00e2cfe0(Ptr)),
        entry!(0x00e2d0d0, fn_00e2d0d0(Ptr, Ptr, u32, Ptr, Ptr) -> i32),
        entry!(0x00e2d1b0, fn_00e2d1b0(Ptr, i32, Ptr, Ptr) -> u32),
        entry!(0x00e2d440, fn_00e2d440(Ptr)),
        entry!(0x00e2d550, fn_00e2d550(Ptr, Ptr) -> Ptr),
        entry!(0x00e2da40, fn_00e2da40(Ptr, Ptr, Ptr, Ptr, Ptr)),
        entry!(0x00e2db70, fn_00e2db70(Ptr, Ptr, Ptr, Ptr, Ptr, Ptr)),
        entry!(0x00e2_b070, fn_00e2b070(Ptr, Ptr, u32, Ptr)),
        entry!(0x00e2_b510, fn_00e2b510(Ptr, Ptr, u32, Ptr)),
        entry!(0x00e2_b9b0, fn_00e2b9b0(Ptr, Ptr, Ptr)),
        entry!(0x00e2_baa0, fn_00e2baa0(Ptr, Ptr, u32)),
        entry!(
            0x00e2_baf0,
            fn_00e2baf0(Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, u32)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x0100_0000, 0x0030_0000);
        e.set_global(ZERO, 0.0f32);
        e.set_global(THREE, 3.0f32);
        e.set_global(HALF, 0.5f32);
        e.set_global(MAX_FLOAT, f32::from_bits(0x7f7f_ffee));
        e.set_global(MIN_FLOAT, f32::from_bits(0xff7f_ffee));
        for lane in 0..4 {
            e.mem.set_u32(SIGN_MASK + 4 * lane, 0x8000_0000);
        }
        e.set_global(NEG_TINY, -1e-10f32 as f64);
        e.set_global(NINE_TENTHS, 0.9f32 as f64);
        e.set_global(SHRINK, 0.99f32 as f64);
        e.set_global(HALF_DOUBLE, 0.5f64);
        e.set_global(SMALL_STEP, 0.001f32);
        e
    }

    fn vector_at(e: &mut Engine, v: Vec4) -> u32 {
        let address = e.mem.alloc(16);
        store_vec(e, address, v);
        address
    }

    fn assert_close(actual: &[f32], expected: &[f32]) {
        for (a, b) in actual.iter().zip(expected) {
            assert!((a - b).abs() < 1e-4, "{actual:?} vs {expected:?}");
        }
    }

    /// A face whose three slots point at the given vertices.
    fn face_on(e: &mut Engine, vertices: [Vec4; 3]) -> u32 {
        let face = e.mem.alloc(0x50);
        for (k, v) in vertices.into_iter().enumerate() {
            let at = vector_at(e, v);
            e.mem.set_u32(face + 0x10 + 0x10 * k as u32, at);
        }
        face
    }

    #[test]
    fn face_plane_is_the_unit_normal_and_its_distance() {
        let mut e = engine();
        let face = face_on(
            &mut e,
            [
                [1.0, 0.0, 5.0, 0.0],
                [0.0, 0.0, 3.0, 0.0],
                [0.0, 1.0, 3.0, 0.0],
            ],
        );
        e.call(NORMALIZE_FACE, &args![face]);
        let root = 5.0f32.sqrt();
        assert_close(&load_vec(&e, face)[..3], &[2.0 / root, 0.0, -1.0 / root]);
        assert_close(&[e.mem.f32(face + 0x40)], &[-3.0 / root]);
    }

    #[test]
    fn face_plane_of_coincident_vertices_is_zero() {
        let mut e = engine();
        let same = [1.0, 2.0, 3.0, 0.0];
        let face = face_on(&mut e, [same, same, same]);
        e.call(NORMALIZE_FACE, &args![face]);
        assert_eq!(load_vec(&e, face), [0.0; 4]);
        assert_eq!(e.mem.f32(face + 0x40), 0.0);
    }

    /// A face with three records leading to three neighbour faces with the
    /// given states; returns (face, records, neighbours).
    fn ring(e: &mut Engine, states: [u32; 3]) -> (u32, [u32; 3], [u32; 3]) {
        let face = e.mem.alloc(0x50);
        let mut records = [0; 3];
        let mut neighbours = [0; 3];
        for k in 0..3 {
            let neighbour = e.mem.alloc(0x50);
            e.mem.set_u32(neighbour + 0x44, states[k]);
            let target = e.mem.alloc(0x40);
            let record = e.mem.alloc(0x10);
            e.mem.set_u32(record, target);
            e.mem.set_u32(record + 0xc, neighbour);
            e.mem.set_u32(face + 0x18 + 0x10 * k as u32, record);
            records[k] = record;
            neighbours[k] = neighbour;
        }
        (face, records, neighbours)
    }

    #[test]
    fn walk_links_neighbours_that_are_in_state_two() {
        let mut e = engine();
        let this = e.mem.alloc(0x1000);
        let (face, records, _) = ring(&mut e, [2, 5, 2]);
        let counter = e.mem.alloc(8);
        let result = e
            .call(WALK_FACES, &args![this, face, 7u32, counter, counter + 4])
            .u32() as i32;
        assert_eq!(result, 0);
        assert_eq!(e.mem.u32(counter), 2);
        assert_eq!(e.mem.u32(counter + 4), records[2]);
        for k in [0usize, 2] {
            let target = e.mem.u32(records[k]);
            assert_eq!(e.mem.u32(target + 0x30), records[k]);
        }
        let untouched = e.mem.u32(records[1]);
        assert_eq!(e.mem.u32(untouched + 0x30), 0);
        // The face is reset and pushed on the stack of processed faces.
        assert_eq!(e.mem.f32(face + 0x40), f32::from_bits(0x7f7f_ffee));
        assert_eq!(e.mem.u32(this + 0xc), 1);
        assert_eq!(e.mem.u32(this + 0xde0), face);
    }

    #[test]
    fn walk_classifies_fresh_neighbours_and_stops_on_a_four() {
        let mut e = engine();
        let this = e.mem.alloc(0x1000);
        let (face, records, neighbours) = ring(&mut e, [0, 0, 0]);
        let results = [3u32, 2, 4];
        let seen: std::rc::Rc<std::cell::RefCell<Vec<Vec<u32>>>> = Default::default();
        let log = seen.clone();
        e.register_double(FACE_SIDE_TEST, move |_, a| {
            log.borrow_mut().push(a.to_vec());
            let index = log.borrow().len() - 1;
            Ret {
                eax: results[index],
                ..Ret::default()
            }
        });
        let counter = e.mem.alloc(8);
        let result = e
            .call(WALK_FACES, &args![this, face, 7u32, counter, counter + 4])
            .u32() as i32;
        assert_eq!(result, 2);
        assert_eq!(seen.borrow()[0], vec![neighbours[0], 7, this, this + 4]);
        assert_eq!(seen.borrow().len(), 3);
        // The result is saved in each neighbour's state.
        assert_eq!(e.mem.u32(neighbours[0] + 0x44), 3);
        assert_eq!(e.mem.u32(neighbours[1] + 0x44), 2);
        assert_eq!(e.mem.u32(neighbours[2] + 0x44), 4);
        // The state-2 neighbour was linked; the one that answered 4 was not.
        assert_eq!(e.mem.u32(counter), 1);
        assert_eq!(e.mem.u32(counter + 4), records[1]);
    }

    #[test]
    fn walk_recurses_into_a_neighbour_that_answers_one() {
        let mut e = engine();
        let this = e.mem.alloc(0x1000);
        let (face, _, neighbours) = ring(&mut e, [0, 5, 5]);
        // The neighbour's own neighbours are all finished (state 5).
        let (inner, _, _) = ring(&mut e, [5, 5, 5]);
        // Make the first record's neighbour the inner ring's face.
        let record = e.mem.u32(face + 0x18);
        e.mem.set_u32(record + 0xc, inner);
        e.register_double(FACE_SIDE_TEST, move |_, a| {
            assert_eq!(a[0], inner);
            Ret {
                eax: 1,
                ..Ret::default()
            }
        });
        let counter = e.mem.alloc(8);
        let result = e
            .call(WALK_FACES, &args![this, face, 7u32, counter, counter + 4])
            .u32() as i32;
        assert_eq!(result, 0);
        assert_eq!(e.mem.u32(inner + 0x44), 1);
        // The inner face was walked first and pushed first.
        assert_eq!(e.mem.u32(this + 0xc), 2);
        assert_eq!(e.mem.u32(this + 0xde0), inner);
        assert_eq!(e.mem.u32(this + 0xde4), face);
        let _ = neighbours;
    }

    #[test]
    fn walk_returns_two_when_the_face_stack_is_full() {
        let mut e = engine();
        let this = e.mem.alloc(0x1000);
        e.mem.set_u32(this + 0xc, 100);
        let (face, _, _) = ring(&mut e, [5, 5, 5]);
        let counter = e.mem.alloc(8);
        let result = e
            .call(WALK_FACES, &args![this, face, 7u32, counter, counter + 4])
            .u32() as i32;
        assert_eq!(result, 2);
        assert_eq!(e.mem.u32(this + 0xc), 100);
        assert_eq!(e.mem.f32(face + 0x40), f32::from_bits(0x7f7f_ffee));
    }

    /// A polytope with three vertex records and a closed list of two edges
    /// A->B and B->C around the apex `point`; returns (this, edge0, point).
    fn hole(e: &mut Engine, apex: Vec4) -> (u32, u32, u32, [u32; 3]) {
        let this = e.mem.alloc(0x1200);
        e.mem.set_u32(this + 8, 2);
        let a = e.mem.alloc(0x40);
        let b = e.mem.alloc(0x40);
        let c = e.mem.alloc(0x40);
        store_vec(e, a, [0.0, 0.0, 0.0, 0.0]);
        store_vec(e, b, [1.0, 0.0, 0.0, 0.0]);
        store_vec(e, c, [0.0, 1.0, 0.0, 0.0]);
        let mut edges = [0; 2];
        for (k, (from, to)) in [(a, b), (b, c)].into_iter().enumerate() {
            let edge = e.mem.alloc(0x10);
            let cell = e.mem.alloc(4);
            e.mem.set_u32(edge, from);
            e.mem.set_u32(edge + 4, cell);
            e.mem.set_u32(cell, to);
            edges[k] = edge;
        }
        e.mem.set_u32(b + 0x30, edges[1]);
        e.mem.set_u32(c + 0x30, edges[0]);
        let point = vector_at(e, apex);
        (this, edges[0], point, [a, b, c])
    }

    #[test]
    fn closing_a_hole_builds_a_ring_of_faces() {
        let mut e = engine();
        let (this, edge, point, [a, b, c]) = hole(&mut e, [0.0, 0.0, 1.0, 0.0]);
        let result = e.call(0x00e2_d1b0, &args![this, 2i32, edge, point]).u32();
        assert_eq!(result, 0);
        let face0 = this + 0xf70;
        let face1 = this + 0xfc0;
        assert_eq!(e.mem.u32(this + 0x10), 2);
        // Face 0 sits on the edge A->B, face 1 on B->C.
        assert_eq!(e.mem.u32(face0 + 0x10), b);
        assert_eq!(e.mem.u32(face0 + 0x20), a);
        assert_eq!(e.mem.u32(face0 + 0x30), point);
        assert_eq!(e.mem.u32(face0 + 0x14), face0 + 0x20);
        assert_eq!(e.mem.u32(face0 + 0x24), face0 + 0x30);
        assert_eq!(e.mem.u32(face0 + 0x34), face0 + 0x10);
        assert_eq!(e.mem.u32(face0 + 0x1c), face0);
        assert_eq!(e.mem.u32(face0 + 0x18), edge);
        assert_eq!(e.mem.u32(edge + 8), face0 + 0x10);
        assert_eq!(e.mem.u32(face1 + 0x10), c);
        assert_eq!(e.mem.u32(face1 + 0x20), b);
        // Neighbour links, the last face wrapping to the first.
        assert_eq!(e.mem.u32(face0 + 0x38), face1 + 0x20);
        assert_eq!(e.mem.u32(face1 + 0x28), face0 + 0x30);
        assert_eq!(e.mem.u32(face1 + 0x38), face0 + 0x20);
        assert_eq!(e.mem.u32(face0 + 0x28), face1 + 0x30);
        // The planes and the cleared links.
        assert_close(&load_vec(&e, face0)[..3], &[0.0, 1.0, 0.0]);
        assert_close(&[e.mem.f32(face0 + 0x40)], &[0.0]);
        let third = 1.0 / 3.0f32.sqrt();
        assert_close(&load_vec(&e, face1)[..3], &[-third, -third, -third]);
        assert_close(&[e.mem.f32(face1 + 0x40)], &[-third]);
        assert_eq!(e.mem.u32(b + 0x30), 0);
        assert_eq!(e.mem.u32(c + 0x30), 0);
    }

    #[test]
    fn closing_a_hole_takes_recycled_faces_first() {
        let mut e = engine();
        let (this, edge, point, [_, b, _]) = hole(&mut e, [0.0, 0.0, 1.0, 0.0]);
        let recycled = e.mem.alloc(0x50);
        e.mem.set_u32(this + 0xc, 1);
        e.mem.set_u32(this + 0xde0, recycled);
        let result = e.call(0x00e2_d1b0, &args![this, 2i32, edge, point]).u32();
        assert_eq!(result, 0);
        assert_eq!(e.mem.u32(this + 0xc), 0);
        // One face came off the stack, one is new.
        assert_eq!(e.mem.u32(this + 0x10), 1);
        assert_eq!(e.mem.u32(recycled + 0x10), b);
        assert_eq!(e.mem.u32(edge + 8), recycled + 0x10);
    }

    #[test]
    fn closing_a_hole_stops_at_a_degenerate_face() {
        let mut e = engine();
        // The apex coincides with the first vertex: a zero normal.
        let (this, edge, point, _) = hole(&mut e, [0.0, 0.0, 0.0, 0.0]);
        let result = e.call(0x00e2_d1b0, &args![this, 2i32, edge, point]).u32();
        assert_eq!(result, 2);
        // The edge list running out ends the call with 2 as well.
        let result = e.call(0x00e2_d1b0, &args![this, 1i32, 0u32, point]).u32();
        assert_eq!(result, 2);
    }

    #[test]
    fn closing_a_hole_reports_an_open_ring() {
        let mut e = engine();
        let (this, edge, point, [_, _, c]) = hole(&mut e, [0.0, 0.0, 1.0, 0.0]);
        // Only the first edge is used: the walk ends on edge 1, not edge 0.
        let result = e.call(0x00e2_d1b0, &args![this, 1i32, edge, point]).u32();
        assert_eq!(result, 2);
        let _ = c;
    }

    #[test]
    fn first_faces_are_built_from_the_table() {
        let mut e = engine();
        for j in 0..12u32 {
            e.mem.set_u32(FACE_TABLE + 12 * j, 0x40 * (j % 4));
            e.mem.set_u32(FACE_TABLE + 12 * j + 4, 0x10 * j);
            e.mem.set_u32(FACE_TABLE + 12 * j + 8, 0x20);
        }
        let this = e.mem.alloc(0x1200);
        store_vec(&mut e, this + 0x20, [0.0, 0.0, 1.0, 0.0]);
        store_vec(&mut e, this + 0x60, [0.0; 4]);
        let faces: std::rc::Rc<std::cell::RefCell<Vec<u32>>> = Default::default();
        let log = faces.clone();
        let last = this + 0x1060;
        e.register_double(NORMALIZE_FACE, move |e, a| {
            log.borrow_mut().push(a[0]);
            if a[0] == last {
                store_vec(e, a[0], [0.0, 0.0, 1.0, 0.0]);
            }
            Ret::default()
        });
        e.call(0x00e2_d440, &args![this]);
        assert_eq!(e.mem.u32(this + 0x10), 4);
        // Face 1 (+0xfc0), slot 2: entry 5 of the table.
        let face = this + 0xfc0;
        assert_eq!(e.mem.u32(face + 0x20), this + 0x20);
        assert_eq!(e.mem.u32(face + 0x28), 0x20 + 0x10 * 4 + this + 0xf80);
        assert_eq!(e.mem.u32(face + 0x2c), face);
        assert_eq!(e.mem.u32(face + 0x14), face + 0x20);
        assert_eq!(e.mem.u32(face + 0x24), face + 0x30);
        assert_eq!(e.mem.u32(face + 0x34), face + 0x10);
        assert_eq!(e.mem.u32(face + 0x44), 0);
        // The last face's normal faces the first vertex: three slots are redirected.
        assert_eq!(e.mem.u32(this + 0x1020), this + 0x60);
        assert_eq!(e.mem.u32(this + 0xfd0), this + 0x60);
        assert_eq!(e.mem.u32(this + 0xf80), this + 0x60);
        assert_eq!(e.mem.u32(last + 0x10), this + 0x20);
        assert_eq!(e.mem.u32(this + 0xfe0), this + 0x20);
        assert_eq!(e.mem.u32(this + 0xfa0), this + 0x20);
        assert_eq!(
            *faces.borrow(),
            vec![
                last,
                this + 0xf70,
                this + 0xfc0,
                this + 0x1010,
                this + 0x1060
            ]
        );
    }

    #[test]
    fn first_faces_keep_their_vertices_when_the_normal_points_away() {
        let mut e = engine();
        let this = e.mem.alloc(0x1200);
        store_vec(&mut e, this + 0x20, [0.0, 0.0, -1.0, 0.0]);
        store_vec(&mut e, this + 0x60, [0.0; 4]);
        let last = this + 0x1060;
        e.register_double(NORMALIZE_FACE, move |e, a| {
            if a[0] == last {
                store_vec(e, a[0], [0.0, 0.0, 1.0, 0.0]);
            }
            Ret::default()
        });
        e.call(0x00e2_d440, &args![this]);
        assert_eq!(e.mem.u32(this + 0x1020), e.mem.u32(this + 0x1020));
        assert_ne!(e.mem.u32(this + 0x1020), this + 0x60);
        assert_ne!(e.mem.u32(this + 0xfd0), this + 0x60);
    }
}

#[cfg(test)]
mod tests_a05 {
    use crate::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Calls = Rc<RefCell<Vec<(u32, Vec<u32>)>>>;

    /// A 0x60-byte-entry list: `count` entries in use with the given keys, capacity `capacity`.
    /// Returns (store, entries).
    fn store(e: &mut Engine, keys: &[f32], capacity: i32, count: i32) -> (u32, u32) {
        let store = e.mem.alloc(0x20);
        let entries = e.mem.alloc(0x60 * 4);
        for (index, key) in keys.iter().enumerate() {
            e.mem.set_f32(entries + 0x60 * index as u32 + 0x10, *key);
        }
        e.mem.set_u32(store + 8, entries);
        e.mem.set_u32(store + 0xc, entries + 0x60 * count as u32);
        e.mem.set_i32(store + 0x10, capacity);
        e.mem.set_i32(store + 0x14, count);
        (store, entries)
    }

    /// A chain of three nodes linked through +0xc whose +4 words are 0x111, 0x222, 0x333.
    fn chain(e: &mut Engine) -> [u32; 3] {
        let nodes = [e.mem.alloc(0x10), e.mem.alloc(0x10), e.mem.alloc(0x10)];
        for (index, node) in nodes.iter().enumerate() {
            e.mem.set_u32(node + 4, 0x111 * (index as u32 + 1));
            e.mem
                .set_u32(node + 0xc, nodes.get(index + 1).copied().unwrap_or(0));
        }
        nodes
    }

    fn candidate(e: &mut Engine, key: f32) -> u32 {
        let candidate = e.mem.alloc(0x20);
        for word in 0..4u32 {
            e.mem.set_u32(candidate + 4 * word, 0xc0de_0000 + word);
        }
        e.mem.set_f32(candidate + 0x10, key);
        e.mem.set_u32(candidate + 0x14, 0xfeed);
        candidate
    }

    /// With room the entry at the write pointer is used; the candidate's words, the last chain
    /// node and the two chain words (plus the terminator) end up in it.
    #[test]
    fn fn_00e2b9b0_fills_the_next_free_entry() {
        let mut e = Engine::new();
        let (store, entries) = store(&mut e, &[], 3, 0);
        let nodes = chain(&mut e);
        let candidate = candidate(&mut e, 2.5);
        e.call(0x00e2_b9b0, &args![nodes[0], candidate, store]);
        assert_eq!(e.mem.i32(store + 0x14), 1);
        assert_eq!(e.mem.u32(store + 0xc), entries + 0x60);
        assert_eq!(e.mem.f32(entries + 0x10), 2.5);
        for word in 0..4u32 {
            assert_eq!(e.mem.u32(entries + 4 * word), 0xc0de_0000 + word);
        }
        assert_eq!(e.mem.u32(entries + 0x14), 0xfeed);
        assert_eq!(e.mem.u32(entries + 0x50), nodes[2]);
        assert_eq!(e.mem.u32(entries + 0x20), 0x222);
        assert_eq!(e.mem.u32(entries + 0x24), 0x111);
        assert_eq!(e.mem.u32(entries + 0x28), 0xffff_ffff);
    }

    /// A non-zero override pointer replaces the last-node search; a chain without links gives
    /// only the terminator at word 0.
    #[test]
    fn fn_00e2b9b0_uses_the_override_pointer() {
        let mut e = Engine::new();
        let (store, entries) = store(&mut e, &[], 3, 0);
        e.mem.set_u32(store + 0x18, 0x4242);
        let lone = e.mem.alloc(0x10);
        let candidate = candidate(&mut e, 1.0);
        e.call(0x00e2_b9b0, &args![lone, candidate, store]);
        assert_eq!(e.mem.u32(entries + 0x50), 0x4242);
        assert_eq!(e.mem.u32(entries + 0x20), 0xffff_ffff);
    }

    /// A chain without links and no override: the entry's last node is the chain's own node.
    #[test]
    fn fn_00e2b9b0_single_node_chain_is_its_own_last_node() {
        let mut e = Engine::new();
        let (store, entries) = store(&mut e, &[], 3, 0);
        let lone = e.mem.alloc(0x10);
        let candidate = candidate(&mut e, 1.0);
        e.call(0x00e2_b9b0, &args![lone, candidate, store]);
        assert_eq!(e.mem.u32(entries + 0x50), lone);
    }

    /// A full list replaces its largest key (here entry 1) when the candidate's key is smaller.
    #[test]
    fn fn_00e2b9b0_replaces_the_largest_key_when_full() {
        let mut e = Engine::new();
        let (store, entries) = store(&mut e, &[1.0, 5.0, 3.0], 3, 3);
        let nodes = chain(&mut e);
        let candidate = candidate(&mut e, 2.0);
        e.call(0x00e2_b9b0, &args![nodes[0], candidate, store]);
        assert_eq!(e.mem.f32(entries + 0x10), 1.0);
        assert_eq!(e.mem.f32(entries + 0x60 + 0x10), 2.0);
        assert_eq!(e.mem.f32(entries + 0xc0 + 0x10), 3.0);
        assert_eq!(e.mem.i32(store + 0x14), 3);
        assert_eq!(e.mem.u32(entries + 0x60 + 0x50), nodes[2]);
    }

    /// A full list is left alone when the candidate's key is not smaller than the largest
    /// (greater, equal or a NaN).
    #[test]
    fn fn_00e2b9b0_keeps_a_full_list_when_the_candidate_is_not_smaller() {
        for key in [9.0f32, 5.0, f32::NAN] {
            let mut e = Engine::new();
            let (store, entries) = store(&mut e, &[1.0, 5.0, 3.0], 3, 3);
            let nodes = chain(&mut e);
            let candidate = candidate(&mut e, key);
            e.call(0x00e2_b9b0, &args![nodes[0], candidate, store]);
            assert_eq!(e.mem.f32(entries + 0x60 + 0x10), 5.0, "key {key}");
            assert_eq!(e.mem.u32(entries + 0x60 + 0x50), 0);
        }
    }

    /// A NaN key in the list never counts as larger: the first entry stays the victim.
    #[test]
    fn fn_00e2b9b0_a_nan_in_the_list_is_not_a_victim() {
        let mut e = Engine::new();
        let (store, entries) = store(&mut e, &[4.0, f32::NAN, 3.0], 3, 3);
        let nodes = chain(&mut e);
        let candidate = candidate(&mut e, 1.0);
        e.call(0x00e2_b9b0, &args![nodes[0], candidate, store]);
        assert_eq!(e.mem.f32(entries + 0x10), 1.0);
    }

    /// The fields are copied, the pointer words depend on the byte at +0x20 and the extra word.
    #[test]
    fn fn_00e2baa0_initialises_the_record() {
        let mut e = Engine::new();
        let object = e.mem.alloc(0x60);
        e.mem.write(object, &[0x55; 0x60]);
        let source = e.mem.alloc(0x40);
        e.mem.set_u32(source + 0x30, 0x3030);
        e.mem.set_u32(source + 0x24, 0x2424);
        e.mem.set_u8(source + 0x20, 1);
        e.call(0x00e2_baa0, &args![object, source, 0x100u32]);
        assert_eq!(e.mem.f32(object + 0x14), 1.0);
        assert_eq!(e.mem.u8(object + 0x18), 0);
        assert_eq!(e.mem.u32(object + 0x1c), source);
        assert_eq!(e.mem.u32(object + 0x20), source);
        assert_eq!(e.mem.u32(object + 0x24), 0x3030);
        assert_eq!(e.mem.u32(object + 0x28), 0x3030);
        assert_eq!(e.mem.u32(object + 0x50), 0x2424);
        assert_eq!(e.mem.u32(object + 0x54), 0x110);
        assert_eq!(e.mem.u32(object + 0x58), 0x5555_5555);
        // A zero byte, or a zero extra word, gives 0.
        e.mem.set_u8(source + 0x20, 0);
        e.call(0x00e2_baa0, &args![object, source, 0x100u32]);
        assert_eq!(e.mem.u32(object + 0x54), 0);
        e.mem.set_u8(source + 0x20, 0x80);
        e.call(0x00e2_baa0, &args![object, source, 0x100u32]);
        assert_eq!(e.mem.u32(object + 0x54), 0x110);
        e.call(0x00e2_baa0, &args![object, source, 0u32]);
        assert_eq!(e.mem.u32(object + 0x54), 0);
    }

    /// Eight vectors with lane values 10 * index + lane become six rows.
    #[test]
    fn fn_00e2baf0_transposes_eight_vectors_into_six() {
        let mut e = Engine::new();
        let vectors: Vec<u32> = (0..8u32)
            .map(|index| {
                let at = e.mem.alloc(0x10);
                for lane in 0..4u32 {
                    e.mem.set_u32(at + 4 * lane, 10 * index + lane);
                }
                at
            })
            .collect();
        let out = e.mem.alloc(0x80);
        e.mem.write(out, &[0x77; 0x80]);
        let mut words = vec![out];
        words.extend(vectors.iter().copied());
        words.push(0x1234);
        e.call(0x00e2_baf0, &words);
        let rows: Vec<Vec<u32>> = (0..6u32)
            .map(|row| {
                (0..4u32)
                    .map(|lane| e.mem.u32(out + 0x10 * row + 4 * lane))
                    .collect()
            })
            .collect();
        // a, c, e, g are vectors 0, 2, 4, 6; b, d, f, h are 1, 3, 5, 7.
        assert_eq!(rows[0], vec![0, 20, 40, 60]);
        assert_eq!(rows[1], vec![1, 21, 41, 61]);
        assert_eq!(rows[2], vec![2, 22, 42, 62]);
        assert_eq!(rows[3], vec![10, 30, 50, 70]);
        assert_eq!(rows[4], vec![11, 31, 51, 71]);
        assert_eq!(rows[5], vec![12, 32, 52, 72]);
        assert_eq!(e.mem.u32(out + 0x60), 0x1234);
        assert_eq!(e.mem.u32(out + 0x64), 0x7777_7777);
    }

    /// The frame builders: the two big callees are doubles; `00d1d530` (or `00d1d490`) checks
    /// what the frame holds when it is called and changes four frame words and the vector copy,
    /// which the function then writes back.
    fn check_frame_builder(address: u32, final_callee: u32, pointer_shift: u32) {
        let mut e = Engine::new();
        e.map(0x0100_0000, 0x0030_0000);
        let calls: Calls = Rc::new(RefCell::new(Vec::new()));
        let first_calls = Rc::clone(&calls);
        e.register_double(0x00d1_bb50, move |_, a| {
            first_calls.borrow_mut().push((0x00d1_bb50, a.to_vec()));
            Ret::default()
        });
        let owner = e.mem.alloc(0x20);
        e.mem.set_u32(owner + 0x10, 0xabc0);
        let source = e.mem.alloc(0x100);
        for word in 0..0x3d {
            e.mem.set_u32(source + 4 * word, 0x1000 + word);
        }
        // The byte flags and the table index the function reads.
        e.mem.set_u8(source + 0x20, 7);
        e.mem.set_u8(source + 0xc4, 9);
        e.mem.set_u32(source + 0xc0, 2);
        e.set_global(0x011b_3830 + 8, 0x7e57u32);
        let vector = e.mem.alloc(0x10);
        for (lane, value) in [1.0f32, 2.0, 3.0, 4.0].into_iter().enumerate() {
            e.mem.set_f32(vector + 4 * lane as u32, value);
        }
        let last_calls = Rc::clone(&calls);
        e.register_double(final_callee, move |e, a| {
            last_calls.borrow_mut().push((final_callee, a.to_vec()));
            let frame = a[3] - 0x340;
            // The record vectors copied from the source, the vector copy and the sources of
            // the written-back words.
            for word in 0..4u32 {
                assert_eq!(e.mem.u32(frame + 0x30 + 4 * word), 0x1000 + word);
                assert_eq!(e.mem.u32(frame + 0x40 + 4 * word), 0x1004 + word);
            }
            for (lane, value) in [1.0f32, 2.0, 3.0, 4.0].into_iter().enumerate() {
                assert_eq!(e.mem.f32(a[5] + 4 * lane as u32), value);
            }
            assert_eq!(a[4], frame + 0x20);
            assert_eq!(a[5], frame + 0x10);
            let objects: Vec<u32> = (0..4u32).map(|slot| e.mem.u32(a[4] + 4 * slot)).collect();
            let expected: Vec<u32> = [0x138u32, 0x1b8, 0x238, 0x2b8]
                .iter()
                .map(|offset| frame + offset + pointer_shift)
                .collect();
            assert_eq!(objects, expected);
            // The table word at the end of the frame (`011b3830 + 4 * [source + 0xc0]`).
            assert_eq!(e.mem.u32(frame + 0x3a0), 0x7e57);
            for (offset, word) in [
                (0x68u32, 0xa1u32),
                (0xa8, 0xa2),
                (0xe8, 0xa3),
                (0x128, 0xa4),
            ] {
                e.mem.set_u32(frame + offset, word);
            }
            for (lane, value) in [5.0f32, 6.0, 7.0, 8.0].into_iter().enumerate() {
                e.mem.set_f32(a[5] + 4 * lane as u32, value);
            }
            Ret::default()
        });
        e.call(address, &args![owner, source, 0x7777u32, vector]);
        let calls = calls.borrow();
        assert_eq!(calls.len(), 2);
        let (first_address, first) = &calls[0];
        assert_eq!(*first_address, 0x00d1_bb50);
        assert_eq!(
            first[1..],
            [
                source,
                source + 0x10,
                source + 0x30,
                source + 0x40,
                source + 0x60,
                source + 0x70,
                source + 0x90,
                source + 0xa0,
                0xf
            ]
        );
        let (_, last) = &calls[1];
        assert_eq!(last[..3], [owner, 0xabc0, 0]);
        assert_eq!(first[0], last[3]);
        for (offset, word) in [(0xe0u32, 0xa1u32), (0xe4, 0xa2), (0xe8, 0xa3), (0xec, 0xa4)] {
            assert_eq!(e.mem.u32(source + offset), word);
        }
        let result: Vec<f32> = (0..4u32).map(|lane| e.mem.f32(vector + 4 * lane)).collect();
        assert_eq!(result, vec![5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn fn_00e2b070_builds_the_frame_and_writes_the_results_back() {
        check_frame_builder(0x00e2_b070, 0x00d1_d530, 0);
    }

    #[test]
    fn fn_00e2b510_builds_the_frame_and_writes_the_results_back() {
        check_frame_builder(0x00e2_b510, 0x00d1_d490, 4);
    }

    /// A null owner does nothing at all.
    #[test]
    fn frame_builders_do_nothing_for_a_null_owner() {
        let mut e = Engine::new();
        e.call_log = Some(vec![]);
        e.call(0x00e2_b070, &args![0u32, 0x1000u32, 0u32, 0x2000u32]);
        e.call(0x00e2_b510, &args![0u32, 0x1000u32, 0u32, 0x2000u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }
}
