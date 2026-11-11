//! The pick of a geometry's triangles (`NiPick`): what the game's ray tests (the crosshair, VATS,
//! decals, the console's picking) run on a tri-based geometry once the ray hits its bound. Each reads
//! the positions (copied corner by corner into its frame for the ray-triangle test) and, for a hit,
//! the texture coordinates, normals and colours of the triangle's corners.
//!
//! Three functions: the general one (`00e9b4f0`, triangles through the data's triangle query), and
//! the two it hands a single-strip `NiTriStrips` (`00e993d0`) and an `NiTriShape` (`00e9a280`). Their
//! x87 steps are the exe's, one instruction at a time, with the values stored to the frame slots the
//! exe stores them to (a frame slot's address goes to the calls by address, so the frames are laid
//! out as the exe's).

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// A pick's results: the record adder (`00e99330`, `thiscall` on the results at pick +0x18, the
/// object; answers the record).
const RECORD_ADD: u32 = 0x00e9_9330;
/// The classes the general pick hands on: `NiTriStrips` (`011f4a20`) and `NiTriShape` (`011f4a40`),
/// and the class test (`0043b300`, `cdecl`: the record, the object).
const TRI_STRIPS_RTTI: u32 = 0x011f_4a20;
const TRI_SHAPE_RTTI: u32 = 0x011f_4a40;
const IS_KIND_OF: u32 = 0x0043_b300;
/// The geometry data's slots: the active triangle count (0xa8, 16 bits) and a triangle's indices
/// (0xac: the triangle and the addresses of its three 16-bit indices); the additional data's slot
/// 0x8c (a byte: whether the pick locks its vertices).
const SLOT_TRIANGLE_COUNT: u32 = 0xa8;
const SLOT_TRIANGLE_INDICES: u32 = 0xac;
const SLOT_LOCKABLE: u32 = 0x8c;
/// The geometry data's lock (`00a67360`, `thiscall`, 1; a byte: release it after) and release
/// (`00a673d0`), and its stream locks, each filling a three-word block (the array, the stride, a
/// byte) the caller zeroes first: the vertices (`00a67410`), the texture coordinates (`00a67720`,
/// the set 0 and the block), the normals (`00a675b0`) and the colours (`00a67660`).
const DATA_LOCK: u32 = 0x00a6_7360;
const DATA_UNLOCK: u32 = 0x00a6_73d0;
const LOCK_VERTICES: u32 = 0x00a6_7410;
const LOCK_TEXTURES: u32 = 0x00a6_7720;
const LOCK_NORMALS: u32 = 0x00a6_75b0;
const LOCK_COLOURS: u32 = 0x00a6_7660;
/// `NiTriStripsData::GetStripLists` (`00a75f10`) and `NiTriShapeData::GetTriList` (`00a7b850`).
const GET_STRIP_LISTS: u32 = 0x00a7_5f10;
const GET_TRI_LIST: u32 = 0x00a7_b850;
/// The ray-triangle test (`00eadb90`, `cdecl`, ten words).
const RAY_HITS_TRIANGLE: u32 = 0x00ea_db90;
/// The point helpers: the matrix times a point (`004b4500`, `thiscall` on the matrix: the out, the
/// point), a point times a matrix (`004b3ae0`, `cdecl`: the out, the point, the matrix), the
/// difference and sum of points (`00439ef0`, `00439e90`, `thiscall`: the out, the other), a point's
/// length (`00457990`, in ST0), the scaled two-float point (`00532fd0`, `cdecl`: the out, the
/// float, the point) and the sum of two (`004dfe30`, `thiscall`), the scaled point (`004a3760`,
/// `cdecl`), the scaled colour (`00532f40`, `cdecl`) and the sum of two (`00532e40`, `thiscall`), a
/// locked colour block's colour (`00534780`, `thiscall`: the vertex, the out), the cross product
/// (`004b3800`, `thiscall`: the out, the other) and the normalize (`004a0c10`, `thiscall`).
const MATRIX_TIMES_POINT: u32 = 0x004b_4500;
const POINT_TIMES_MATRIX: u32 = 0x004b_3ae0;
const POINT_MINUS_POINT: u32 = 0x0043_9ef0;
const POINT_PLUS_POINT: u32 = 0x0043_9e90;
const POINT_LENGTH: u32 = 0x0045_7990;
const POINT2_TIMES_SCALAR: u32 = 0x0053_2fd0;
const POINT2_PLUS_POINT2: u32 = 0x004d_fe30;
const SCALAR_TIMES_POINT: u32 = 0x004a_3760;
const COLOUR_TIMES_SCALAR: u32 = 0x0053_2f40;
const COLOUR_PLUS_COLOUR: u32 = 0x0053_2e40;
const LOCKED_COLOUR_AT: u32 = 0x0053_4780;
const POINT_CROSS: u32 = 0x004b_3800;
const UNITIZE: u32 = 0x004a_0c10;
/// The square root (`00ec6040`: the value in ST0, its root in ST0).
const SQUARE_ROOT: u32 = 0x00ec_6040;
/// The record's defaults when the pick does not return them: the texture coordinates (two words at
/// `011f4980`), the normal (three at `011f426c`), the colour (four at `011a9be0`); the four words read
/// when the object's word +0x20 is null (`011f4288`); the 0.0 the ray's dot product is compared with
/// (`01011d78`).
const DEFAULT_TEXTURE: u32 = 0x011f_4980;
const DEFAULT_NORMAL: u32 = 0x011f_426c;
const DEFAULT_COLOUR: u32 = 0x011a_9be0;
const DEFAULT_BOUND: u32 = 0x011f_4288;
const ZERO: u32 = 0x0101_1d78;

/// `FLD dword [address]`.
fn ld(e: &Engine, address: u32) -> f64 {
    x87_load32(e.mem.u32(address))
}

/// A float memory operand of an x87 arithmetic instruction.
fn op(e: &Engine, address: u32) -> f64 {
    x87_m32(e.mem.u32(address))
}

/// `FSTP dword [address]`.
fn st(e: &mut Engine, address: u32, value: f64) {
    e.mem.set_u32(address, x87_store32(value));
}

/// A value stored as a float and loaded back.
fn stored(value: f64) -> f64 {
    x87_load32(x87_store32(value))
}

/// Copies `count` words.
fn copy_words(e: &mut Engine, from: u32, to: u32, count: u32) {
    for i in 0..count {
        let word = e.mem.u32(from.wrapping_add(4 * i));
        e.mem.set_u32(to.wrapping_add(4 * i), word);
    }
}

/// Writes words from `to` on.
fn write_words(e: &mut Engine, to: u32, words: &[u32]) {
    for (i, word) in (0u32..).zip(words) {
        e.mem.set_u32(to.wrapping_add(4 * i), *word);
    }
}

/// A float argument the exe pushes with `FLD dword [address]; FSTP dword [ESP]`: its bits.
fn pushed_float(e: &Engine, address: u32) -> u32 {
    x87_quiet_bits(e.mem.u32(address))
}

/// Zeroes a lock block (the array, the stride, a byte) as the exe does before a lock call.
fn clear_lock_block(e: &mut Engine, block: u32) {
    e.mem.set_u32(block, 0);
    e.mem.set_u32(block + 4, 0);
    e.mem.set_u8(block + 8, 0);
}

/// Whether the pick stops at its first hit (pick type +0 and sort type +4 both 1).
fn stops_at_first_hit(e: &Engine, pick: u32) -> bool {
    e.mem.u32(pick) == 1 && e.mem.u32(pick + 4) == 1
}

/// A record added to the pick's results for `object` (by address: the translations call it plainly).
fn add_record(e: &mut Engine, pick: u32, object: u32) -> u32 {
    e.call(RECORD_ADD, &args![pick + 0x18, object]).u32()
}

/// The frame slots of a hit's barycentric weights: `1 - u - v`, `u` and `v`.
#[derive(Clone, Copy)]
struct Weights {
    w: u32,
    u: u32,
    v: u32,
}

impl Weights {
    /// `1 - u - v` (`FLD u; FADD v; FLD1; FSUBRP`), stored at its slot.
    fn store_first(&self, e: &mut Engine) {
        let weight = x87_sub(1.0, x87_add(ld(e, self.u), op(e, self.v)));
        st(e, self.w, weight);
    }

    /// The three weights as `FLD` loads them (corners 0, 1, 2).
    fn values(&self, e: &Engine) -> [f64; 3] {
        [ld(e, self.w), ld(e, self.u), ld(e, self.v)]
    }

    /// The three weights as pushed float arguments (corners 0, 1, 2).
    fn pushed(&self, e: &Engine) -> [u32; 3] {
        [
            pushed_float(e, self.w),
            pushed_float(e, self.u),
            pushed_float(e, self.v),
        ]
    }
}

/// `c0 * w + c1 * u + c2 * v` over `N` floats a corner, as the picks compute it inline: each product
/// stored as a float (`c * weight`, the last component `weight * c` from a memory operand), then `(c0 w
/// + c1 u) + c2 v`, each sum stored.
fn weighted<const N: usize>(e: &Engine, corners: [u32; 3], weights: [f64; 3]) -> [u32; N] {
    let product = |corner: u32, weight: f64| -> [f64; N] {
        let mut out = [0.0; N];
        for (i, slot) in (0u32..).zip(out.iter_mut()) {
            let at = corner.wrapping_add(4 * i);
            *slot = if (i as usize) + 1 < N {
                stored(x87_mul(ld(e, at), weight))
            } else {
                stored(x87_mul(weight, op(e, at)))
            };
        }
        out
    };
    let far = product(corners[2], weights[2]);
    let middle = product(corners[1], weights[1]);
    let near = product(corners[0], weights[0]);
    let mut out = [0u32; N];
    for (i, slot) in out.iter_mut().enumerate() {
        let sum = stored(x87_add(near[i], middle[i]));
        *slot = x87_store32(x87_add(sum, far[i]));
    }
    out
}

/// The frame slots of a weighted sum through calls: the outs of the third, second and first corners'
/// scaled values, of their partial sum and of the result.
#[derive(Clone, Copy)]
struct CallSlots {
    third: u32,
    second: u32,
    first: u32,
    sum: u32,
    result: u32,
}

/// `c0 * w + c1 * u + c2 * v` through a scaling call (`cdecl`: the out, the float, the corner) and an
/// adding call (`thiscall` on the first part: the out, the other part), in the exe's order. Answers the
/// last call's answer.
fn weighted_by_calls(
    e: &mut Engine,
    times: u32,
    plus: u32,
    corners: [u32; 3],
    weights: [u32; 3],
    slots: CallSlots,
) -> u32 {
    let third = e
        .call(times, &args![slots.third, weights[2], corners[2]])
        .u32();
    let second = e
        .call(times, &args![slots.second, weights[1], corners[1]])
        .u32();
    let first = e
        .call(times, &args![slots.first, weights[0], corners[0]])
        .u32();
    let sum = e.call(plus, &args![first, slots.sum, second]).u32();
    e.call(plus, &args![sum, slots.result, third]).u32()
}

/// The corners' addresses in an array of `stride`-byte elements at `base`.
fn corners_in(base: u32, stride: u32, indices: [u32; 3]) -> [u32; 3] {
    indices.map(|index| index.wrapping_mul(stride).wrapping_add(base))
}

/// The face normal through `004b3800`: the edges `v1 - v0` (to `first_edge`) and `v2 - v0` (to
/// `second_edge`), each coordinate stored in the exe's operand order; the cross product to `out`.
fn face_normal_by_call(
    e: &mut Engine,
    corners: [u32; 3],
    first_edge: u32,
    second_edge: u32,
    out: u32,
) -> u32 {
    let [v0, v1, v2] = corners;
    for i in [0u32, 4, 8] {
        let value = x87_sub(ld(e, v1 + i), ld(e, v0 + i));
        st(e, first_edge + i, value);
    }
    st(e, second_edge, x87_sub(ld(e, v2), ld(e, v0)));
    st(e, second_edge + 4, x87_sub(op(e, v2 + 4), ld(e, v0 + 4)));
    st(e, second_edge + 8, x87_sub(op(e, v2 + 8), ld(e, v0 + 8)));
    e.call(POINT_CROSS, &args![first_edge, out, second_edge])
        .u32()
}

/// The colours of the three corners from the locked colour block `lock` (each into a zeroed colour
/// at `zeroed[i]` through `00534780`), weighted through the colour calls. Answers the result slot.
fn locked_colours(
    e: &mut Engine,
    lock: u32,
    indices: [u32; 3],
    zeroed: [u32; 3],
    weights: [u32; 3],
    slots: CallSlots,
) -> u32 {
    for colour in zeroed {
        write_words(e, colour, &[0; 4]);
    }
    for (index, colour) in indices.into_iter().zip(zeroed) {
        e.call(LOCKED_COLOUR_AT, &args![lock, index, colour]);
    }
    weighted_by_calls(
        e,
        COLOUR_TIMES_SCALAR,
        COLOUR_PLUS_COLOUR,
        zeroed,
        weights,
        slots,
    );
    slots.result
}

/// A hit's normal: copied to `scratch`, normalized and, for coordinate type 1, rotated to world
/// space (`004b4500` into `rotated`); the record's +0x28 gets it.
fn finish_normal(
    e: &mut Engine,
    normal: [u32; 3],
    scratch: u32,
    rotated: u32,
    pick: u32,
    rotate: u32,
    record: u32,
) {
    write_words(e, scratch, &normal);
    e.call(UNITIZE, &args![scratch]);
    if e.mem.u32(pick + 0xc) == 1 {
        let r = e
            .call(MATRIX_TIMES_POINT, &args![rotate, rotated, scratch])
            .u32();
        copy_words(e, r, record.wrapping_add(0x28), 3);
    } else {
        copy_words(e, scratch, record.wrapping_add(0x28), 3);
    }
}

/// The three words at `address`.
fn words3(e: &Engine, address: u32) -> [u32; 3] {
    [0u32, 4, 8].map(|i| e.mem.u32(address.wrapping_add(i)))
}

/// A record's triangle (+0x18) and vertex indices (+0x1a, +0x1c, +0x1e).
fn record_triangle(e: &mut Engine, record: u32, triangle: u32, indices: [u32; 3]) {
    e.mem
        .set_u16(record.wrapping_add(0x18), super::low_half(triangle));
    for (i, index) in (0u32..).zip(indices) {
        e.mem
            .set_u16(record.wrapping_add(0x1a + 2 * i), super::low_half(index));
    }
}

/// The world rotation's rows as the strips' and the shape's picks apply them to the origin (their
/// second and third rows differ in operand order).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rows {
    Strips,
    Shape,
}

/// The frame slots of the model-space ray.
#[derive(Clone, Copy)]
struct ModelSlots {
    difference: u32,
    inverse: u32,
    origin: u32,
    direction: u32,
}

/// The ray in model space: `origin - translation` (+0x24 after the rotation) and the direction
/// through the world rotation (transposed), times `1 / scale` (+0x30), each value stored as a float
/// in the exe's order.
fn model_ray(e: &mut Engine, o: u32, dir: u32, rotate: u32, slots: ModelSlots, rows: Rows) {
    let at = |offset: u32| rotate.wrapping_add(offset);
    let d = slots.difference;
    st(e, d, x87_sub(ld(e, o), op(e, at(0x24))));
    st(e, d + 4, x87_sub(ld(e, o.wrapping_add(4)), op(e, at(0x28))));
    st(e, d + 8, x87_sub(ld(e, o.wrapping_add(8)), op(e, at(0x2c))));
    st(e, slots.inverse, x87_div(1.0, ld(e, at(0x30))));
    let dy = ld(e, d + 4);
    let a = x87_mul(dy, op(e, at(0xc)));
    let dx = ld(e, d);
    let b = x87_mul(ld(e, at(0)), dx);
    let s = x87_add(b, a);
    let dz = ld(e, d + 8);
    let s = x87_add(s, x87_mul(dz, op(e, at(0x18))));
    st(e, d, s);
    let t = x87_add(x87_mul(dx, op(e, at(0x4))), x87_mul(dy, op(e, at(0x10))));
    let t = match rows {
        Rows::Strips => x87_add(t, x87_mul(ld(e, at(0x1c)), dz)),
        Rows::Shape => x87_add(t, x87_mul(dz, op(e, at(0x1c)))),
    };
    st(e, d + 4, t);
    let u = match rows {
        Rows::Strips => {
            let c = x87_mul(dx, ld(e, at(0x8)));
            let f = x87_mul(dy, ld(e, at(0x14)));
            let u = x87_add(f, c);
            x87_add(u, x87_mul(dz, op(e, at(0x20))))
        }
        Rows::Shape => {
            let f = x87_mul(dy, ld(e, at(0x14)));
            let c = x87_mul(dx, ld(e, at(0x8)));
            let u = x87_add(c, f);
            x87_add(x87_mul(dz, ld(e, at(0x20))), u)
        }
    };
    st(e, d + 8, u);
    let inverse = ld(e, slots.inverse);
    for i in [0u32, 4, 8] {
        st(e, slots.origin + i, x87_mul(ld(e, d + i), inverse));
    }
    let a = x87_mul(ld(e, dir.wrapping_add(4)), op(e, at(0xc)));
    let b = x87_mul(ld(e, at(0)), op(e, dir));
    let s = x87_add(a, b);
    let s = x87_add(s, x87_mul(ld(e, at(0x18)), op(e, dir.wrapping_add(8))));
    st(e, d, s);
    let a = x87_mul(ld(e, dir), op(e, at(0x4)));
    let b = x87_mul(ld(e, dir.wrapping_add(4)), op(e, at(0x10)));
    let t = x87_add(a, b);
    let t = x87_add(t, x87_mul(ld(e, at(0x1c)), op(e, dir.wrapping_add(8))));
    st(e, d + 4, t);
    let a = x87_mul(ld(e, at(0x14)), op(e, dir.wrapping_add(4)));
    let b = x87_mul(ld(e, at(0x8)), op(e, dir));
    let u = x87_add(a, b);
    let u = x87_add(u, x87_mul(ld(e, at(0x20)), op(e, dir.wrapping_add(8))));
    st(e, d + 8, u);
    st(e, slots.direction, x87_mul(ld(e, d), inverse));
    st(e, slots.direction + 4, x87_mul(ld(e, d + 4), inverse));
    st(e, slots.direction + 8, x87_mul(inverse, op(e, d + 8)));
}

/// The data's vertex lock as the picks start: the block at `lock` zeroed; without positions and
/// with additional data whose slot 0x8c answers true (a byte), the lock (`00a67360` with 1; its byte
/// answer kept) and the vertices' lock into the block.
fn lock_vertices(e: &mut Engine, data: u32, vertices: u32, lock: u32) -> bool {
    let mut locked = false;
    if vertices == 0 {
        let additional = e.mem.u32(data.wrapping_add(0x30));
        if additional != 0
            && super::low_byte(e.vcall(additional, SLOT_LOCKABLE, &args![]).u32()) != 0
        {
            locked = super::low_byte(e.call(DATA_LOCK, &args![data, 1u32]).u32()) != 0;
            e.call(LOCK_VERTICES, &args![data, lock]);
        }
    }
    locked
}

/// The pick of a geometry with no triangles to test: a record whose distance is `|translation -
/// origin|` (the differences stored at `differences`, the square at `square`, the square root). The
/// squares of x and y are added as `x^2 + y^2` for the strips and `y^2 + x^2` for the shape.
/// Answers true.
fn record_distance_to_translation(
    e: &mut Engine,
    o: u32,
    p: u32,
    obj: u32,
    differences: u32,
    square: u32,
    rows: Rows,
) -> bool {
    let record = add_record(e, p, obj);
    for i in [0u32, 4, 8] {
        let value = x87_sub(ld(e, obj.wrapping_add(0x8c + i)), op(e, o.wrapping_add(i)));
        st(e, differences + i, value);
    }
    let x = ld(e, differences);
    let y = ld(e, differences + 4);
    let z = ld(e, differences + 8);
    let (xx, yy) = match rows {
        Rows::Strips => {
            let yy = x87_mul(y, y);
            (x87_mul(x, x), yy)
        }
        Rows::Shape => {
            let xx = x87_mul(x, x);
            (xx, x87_mul(y, y))
        }
    };
    let sum = match rows {
        Rows::Strips => x87_add(xx, yy),
        Rows::Shape => x87_add(yy, xx),
    };
    st(e, square, x87_add(sum, x87_mul(z, z)));
    let root = e.call(SQUARE_ROOT, &args![ld(e, square)]).f64();
    st(e, square, root);
    let distance = x87_quiet_bits(e.mem.u32(square));
    e.mem.set_u32(record.wrapping_add(0x14), distance);
    true
}

/// The ray-triangle test of the corner copies (`00eadb90`): the cull byte (pick +0x10) goes out in
/// the low byte of the frame word at `cull_slot`, as the exe pushes it.
#[allow(clippy::too_many_arguments)]
fn test_triangle(
    e: &mut Engine,
    p: u32,
    model: ModelSlots,
    corners: [u32; 3],
    cull_slot: u32,
    point: u32,
    distance: u32,
    weights: Weights,
) -> bool {
    let cull = e.mem.u8(p + 0x10);
    e.mem.set_u8(cull_slot, cull);
    let cull_word = e.mem.u32(cull_slot);
    let answer = e
        .call(
            RAY_HITS_TRIANGLE,
            &args![
                model.origin,
                model.direction,
                corners[0],
                corners[1],
                corners[2],
                cull_word,
                point,
                distance,
                weights.u,
                weights.v
            ],
        )
        .u32();
    super::low_byte(answer) != 0
}

// Typed from 00e993d0
/// The pick of a single-strip `NiTriStrips` (unnamed in the engine map; `cdecl`: the ray's origin and
/// direction, the pick, the geometry; the general pick hands it these). The ray goes into model space;
/// the data gives the triangle count and maybe locks its vertices. Without strip lists, or without
/// vertices, a record gets the distance to the world translation and the answer is true. Otherwise
/// each triangle `i` of the strip (indices `i`, `i + 1`, `i + 2`, the first two swapped for odd `i`; a
/// triangle whose first two or last two indices are equal is skipped) is tested against the model ray
/// (culling with the pick's front-only byte). A hit adds a record: the point (back to world space for
/// coordinate type 1), the distance, the triangle and its indices; the texture coordinates, normal
/// and colour as the pick asks (locked arrays inline, the data's arrays through the point and colour
/// calls, else the defaults). A pick that stops at its first hit answers true there. The data lock is
/// released on those returns; answers whether any triangle was hit.
pub fn pick_strips(e: &mut Engine, origin: Ptr, direction: Ptr, pick: Ptr, object: Ptr) -> bool {
    let (o, dir, p, obj) = (origin.addr(), direction.addr(), pick.addr(), object.addr());
    e.with_stack(0x2fc, |e, frame| {
        let f = frame.addr();
        let rotate = obj.wrapping_add(0x68);
        let model = ModelSlots {
            difference: f + 0x20,
            inverse: f + 0x40,
            origin: f + 0x1d0,
            direction: f + 0x1c4,
        };
        model_ray(e, o, dir, rotate, model, Rows::Strips);
        e.mem.set_u8(f + 0x1b, 0);
        let data = e.mem.u32(obj.wrapping_add(0xb8));
        let count = e.vcall(data, SLOT_TRIANGLE_COUNT, &args![]).u32() & 0xffff;
        let data = e.mem.u32(obj.wrapping_add(0xb8));
        e.mem.set_u32(f + 0x38, data);
        let vertex_lock = f + 0xc0;
        clear_lock_block(e, vertex_lock);
        let vertices = e.mem.u32(data.wrapping_add(0x20));
        let locked = lock_vertices(e, data, vertices, vertex_lock);
        let strips = e.call(GET_STRIP_LISTS, &args![data]).u32();
        if strips == 0 || (vertices == 0 && e.mem.u32(vertex_lock) == 0) {
            return record_distance_to_translation(e, o, p, obj, f + 0x7c, f + 0x3c, Rows::Strips);
        }
        let weights = Weights {
            w: f + 0x1c,
            u: f + 0x30,
            v: f + 0x2c,
        };
        let copies = [f + 0x90, f + 0x9c, f + 0xa8];
        let mut triangle = 0u32;
        while super::low_half(triangle) < super::low_half(count) {
            let at = strips.wrapping_add((triangle & 0xffff) * 2);
            let (first, second) = if triangle & 1 != 0 {
                (e.mem.u16(at.wrapping_add(2)), e.mem.u16(at))
            } else {
                (e.mem.u16(at), e.mem.u16(at.wrapping_add(2)))
            };
            let third = e.mem.u16(at.wrapping_add(4));
            if first != second && second != third {
                let indices = [u32::from(first), u32::from(second), u32::from(third)];
                let sources = if vertices == 0 {
                    let stride = e.mem.u32(vertex_lock + 4);
                    corners_in(e.mem.u32(vertex_lock), stride, indices)
                } else {
                    corners_in(vertices, 12, indices)
                };
                for (source, copy) in sources.into_iter().zip(copies) {
                    copy_words(e, source, copy, 3);
                }
                let point = f + 0xcc;
                if test_triangle(e, p, model, copies, f + 0x134, point, f + 0x114, weights) {
                    e.mem.set_u8(f + 0x1b, 1);
                    let record = add_record(e, p, obj);
                    if e.mem.u32(p + 0xc) == 1 {
                        let r = e
                            .call(MATRIX_TIMES_POINT, &args![rotate, f + 0x21c, point])
                            .u32();
                        st(e, f + 0x118, ld(e, rotate.wrapping_add(0x30)));
                        let scale = ld(e, f + 0x118);
                        let x = stored(x87_mul(ld(e, r), scale));
                        let y = x87_store32(x87_mul(ld(e, r.wrapping_add(4)), scale));
                        let z = x87_store32(x87_mul(scale, op(e, r.wrapping_add(8))));
                        let x = x87_store32(x87_add(x, op(e, rotate.wrapping_add(0x24))));
                        let y = x87_store32(x87_add(ld(e, rotate.wrapping_add(0x28)), x87_m32(y)));
                        let z = x87_store32(x87_add(ld(e, rotate.wrapping_add(0x2c)), x87_m32(z)));
                        write_words(e, point, &[x, y, z]);
                    }
                    copy_words(e, point, record.wrapping_add(8), 3);
                    let distance = x87_quiet_bits(e.mem.u32(f + 0x114));
                    e.mem.set_u32(record.wrapping_add(0x14), distance);
                    record_triangle(e, record, triangle, indices);
                    weights.store_first(e);
                    strips_record_texture(e, f, p, obj, record, indices, weights);
                    strips_record_normal(e, f, p, obj, record, indices, weights);
                    strips_record_colour(e, f, p, obj, record, indices, weights);
                    if stops_at_first_hit(e, p) {
                        if locked {
                            let data = e.mem.u32(f + 0x38);
                            e.call(DATA_UNLOCK, &args![data]);
                        }
                        return true;
                    }
                }
            }
            triangle = triangle.wrapping_add(1);
        }
        if locked {
            let data = e.mem.u32(f + 0x38);
            e.call(DATA_UNLOCK, &args![data]);
        }
        e.mem.u8(f + 0x1b) != 0
    })
}

/// The strips pick's texture coordinates of a hit (its frame `f`; the data at +0x38).
fn strips_record_texture(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    let out = record.wrapping_add(0x20);
    if e.mem.u8(p + 0x30) == 0 {
        copy_words(e, DEFAULT_TEXTURE, out, 2);
        return;
    }
    let lock = f + 0xb4;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x38);
    e.call(LOCK_TEXTURES, &args![data, 0u32, lock]);
    let base = e.mem.u32(lock);
    if base != 0 {
        let corners = corners_in(base, e.mem.u32(lock + 4), indices);
        let texture: [u32; 2] = weighted(e, corners, weights.values(e));
        write_words(e, out, &texture);
        return;
    }
    let data = e.mem.u32(obj.wrapping_add(0xb8));
    let textures = e.mem.u32(data.wrapping_add(0x2c));
    if textures == 0 {
        copy_words(e, DEFAULT_TEXTURE, out, 2);
        return;
    }
    let slots = CallSlots {
        third: f + 0x20c,
        second: f + 0x204,
        first: f + 0x214,
        sum: f + 0x1fc,
        result: f + 0x194,
    };
    let pushed = weights.pushed(e);
    weighted_by_calls(
        e,
        POINT2_TIMES_SCALAR,
        POINT2_PLUS_POINT2,
        corners_in(textures, 8, indices),
        pushed,
        slots,
    );
    copy_words(e, slots.result, out, 2);
}

/// The strips pick's normal of a hit (its frame `f`; the corner copies at +0x90, +0x9c, +0xa8).
fn strips_record_normal(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    if e.mem.u8(p + 0x31) == 0 {
        copy_words(e, DEFAULT_NORMAL, record.wrapping_add(0x28), 3);
        return;
    }
    let lock = f + 0xd8;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x38);
    e.call(LOCK_NORMALS, &args![data, lock]);
    let base = e.mem.u32(lock);
    let normal = if base != 0 {
        let corners = corners_in(base, e.mem.u32(lock + 4), indices);
        weighted::<3>(e, corners, weights.values(e))
    } else {
        let smooth = if e.mem.u8(p + 0x32) != 0 {
            let data = e.mem.u32(obj.wrapping_add(0xb8));
            e.mem.u32(data.wrapping_add(0x24))
        } else {
            0
        };
        let answer = if smooth != 0 {
            let slots = CallSlots {
                third: f + 0x270,
                second: f + 0x24c,
                first: f + 0x240,
                sum: f + 0x228,
                result: f + 0x258,
            };
            let pushed = weights.pushed(e);
            weighted_by_calls(
                e,
                SCALAR_TIMES_POINT,
                POINT_PLUS_POINT,
                corners_in(smooth, 12, indices),
                pushed,
                slots,
            )
        } else {
            let copies = [f + 0x90, f + 0x9c, f + 0xa8];
            face_normal_by_call(e, copies, f + 0x1ac, f + 0x1b8, f + 0x234)
        };
        words3(e, answer)
    };
    let rotate = obj.wrapping_add(0x68);
    finish_normal(e, normal, f + 0x44, f + 0x264, p, rotate, record);
}

/// The strips pick's colour of a hit (its frame `f`).
fn strips_record_colour(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    let out = record.wrapping_add(0x34);
    if e.mem.u8(p + 0x33) == 0 {
        copy_words(e, DEFAULT_COLOUR, out, 4);
        return;
    }
    let lock = f + 0x20;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x38);
    e.call(LOCK_COLOURS, &args![data, lock]);
    if e.mem.u32(lock) != 0 {
        let slots = CallSlots {
            third: f + 0x27c,
            second: f + 0x29c,
            first: f + 0x2bc,
            sum: f + 0x2dc,
            result: f + 0x1ec,
        };
        let pushed = weights.pushed(e);
        let zeroed = [f + 0x174, f + 0x19c, f + 0x184];
        let result = locked_colours(e, lock, indices, zeroed, pushed, slots);
        copy_words(e, result, out, 4);
        return;
    }
    let data = e.mem.u32(obj.wrapping_add(0xb8));
    let colours = e.mem.u32(data.wrapping_add(0x28));
    if colours == 0 {
        copy_words(e, DEFAULT_COLOUR, out, 4);
        return;
    }
    let slots = CallSlots {
        third: f + 0x28c,
        second: f + 0x2ac,
        first: f + 0x2ec,
        sum: f + 0x2cc,
        result: f + 0x1dc,
    };
    let pushed = weights.pushed(e);
    weighted_by_calls(
        e,
        COLOUR_TIMES_SCALAR,
        COLOUR_PLUS_COLOUR,
        corners_in(colours, 16, indices),
        pushed,
        slots,
    );
    copy_words(e, slots.result, out, 4);
}

// Typed from 00e9a280
/// The pick of an `NiTriShape` (unnamed in the engine map; `cdecl`, the four words of the strips'
/// pick), as the strips' pick with these differences: the triangles are the triangle list's (three
/// indices each, none skipped); the corners come from the vertex lock when the data was locked, else
/// from the positions; the hit point goes to world space through the rotation inline; the texture
/// coordinates and smooth normals are weighted inline and the face normal computed inline; only the
/// data's own colours go through the colour calls, the locked ones are weighted inline.
pub fn pick_shape(e: &mut Engine, origin: Ptr, direction: Ptr, pick: Ptr, object: Ptr) -> bool {
    let (o, dir, p, obj) = (origin.addr(), direction.addr(), pick.addr(), object.addr());
    e.with_stack(0x2e0, |e, frame| {
        let f = frame.addr();
        let rotate = obj.wrapping_add(0x68);
        let model = ModelSlots {
            difference: f + 0x18,
            inverse: f + 0x38,
            origin: f + 0x26c,
            direction: f + 0x278,
        };
        model_ray(e, o, dir, rotate, model, Rows::Shape);
        e.mem.set_u8(f + 0x13, 0);
        let data = e.mem.u32(obj.wrapping_add(0xb8));
        let count = e.vcall(data, SLOT_TRIANGLE_COUNT, &args![]).u32() & 0xffff;
        let data = e.mem.u32(obj.wrapping_add(0xb8));
        e.mem.set_u32(f + 0x34, data);
        let vertex_lock = f + 0x130;
        clear_lock_block(e, vertex_lock);
        let vertices = e.mem.u32(data.wrapping_add(0x20));
        let locked = lock_vertices(e, data, vertices, vertex_lock);
        let list = e.call(GET_TRI_LIST, &args![data]).u32();
        if list == 0 || (vertices == 0 && e.mem.u32(vertex_lock) == 0) {
            return record_distance_to_translation(e, o, p, obj, f + 0x9c, f + 0x2c, Rows::Shape);
        }
        let weights = Weights {
            w: f + 0x14,
            u: f + 0x30,
            v: f + 0x28,
        };
        let copies = [f + 0x100, f + 0x118, f + 0x10c];
        let mut triangle = 0u32;
        while super::low_half(triangle) < super::low_half(count) {
            let at = list.wrapping_add((triangle & 0xffff) * 6);
            let indices = [0u32, 2, 4].map(|i| u32::from(e.mem.u16(at.wrapping_add(i))));
            let sources = if locked {
                let stride = e.mem.u32(vertex_lock + 4);
                corners_in(e.mem.u32(vertex_lock), stride, indices)
            } else {
                corners_in(vertices, 12, indices)
            };
            for (source, copy) in sources.into_iter().zip(copies) {
                copy_words(e, source, copy, 3);
            }
            let point = f + 0x124;
            if test_triangle(e, p, model, copies, f + 0x1c0, point, f + 0x1e8, weights) {
                e.mem.set_u8(f + 0x13, 1);
                let record = add_record(e, p, obj);
                if e.mem.u32(p + 0xc) == 1 {
                    shape_point_to_world(e, f, rotate, point);
                }
                copy_words(e, point, record.wrapping_add(8), 3);
                let distance = x87_quiet_bits(e.mem.u32(f + 0x1e8));
                e.mem.set_u32(record.wrapping_add(0x14), distance);
                record_triangle(e, record, triangle, indices);
                weights.store_first(e);
                shape_record_texture(e, f, p, obj, record, indices, weights);
                shape_record_normal(e, f, p, obj, record, indices, weights);
                shape_record_colour(e, f, p, obj, record, indices, weights);
                if stops_at_first_hit(e, p) {
                    if locked {
                        let data = e.mem.u32(f + 0x34);
                        e.call(DATA_UNLOCK, &args![data]);
                    }
                    return true;
                }
            }
            triangle = triangle.wrapping_add(1);
        }
        if locked {
            let data = e.mem.u32(f + 0x34);
            e.call(DATA_UNLOCK, &args![data]);
        }
        e.mem.u8(f + 0x13) != 0
    })
}

/// The shape pick's hit point taken to world space in place: the rotation's rows (not transposed)
/// applied inline, times the scale (+0x30, stored at `f + 0x3c`), plus the translation (+0x24 after
/// the rotation).
fn shape_point_to_world(e: &mut Engine, f: u32, rotate: u32, point: u32) {
    let at = |offset: u32| rotate.wrapping_add(offset);
    let py = ld(e, point + 4);
    let a = x87_mul(py, op(e, at(0x4)));
    let px = ld(e, point);
    let b = x87_mul(ld(e, at(0)), px);
    let s = x87_add(b, a);
    let pz = ld(e, point + 8);
    let s = stored(x87_add(s, x87_mul(pz, op(e, at(0x8)))));
    let t = x87_add(x87_mul(px, op(e, at(0xc))), x87_mul(py, op(e, at(0x10))));
    let t = x87_store32(x87_add(t, x87_mul(pz, op(e, at(0x14)))));
    let c = x87_mul(px, ld(e, at(0x18)));
    let d = x87_mul(py, ld(e, at(0x1c)));
    let u = x87_add(d, c);
    let u = x87_store32(x87_add(u, x87_mul(pz, op(e, at(0x20)))));
    st(e, f + 0x3c, ld(e, at(0x30)));
    let scale = ld(e, f + 0x3c);
    let x = x87_store32(x87_mul(s, scale));
    let y = x87_store32(x87_mul(x87_load32(t), scale));
    let z = x87_store32(x87_mul(scale, x87_m32(u)));
    let x = x87_store32(x87_add(ld(e, at(0x24)), x87_m32(x)));
    let y = x87_store32(x87_add(ld(e, at(0x28)), x87_m32(y)));
    let z = x87_store32(x87_add(ld(e, at(0x2c)), x87_m32(z)));
    write_words(e, point, &[x, y, z]);
}

/// The shape pick's texture coordinates of a hit (its frame `f`; the data at +0x34).
fn shape_record_texture(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    let out = record.wrapping_add(0x20);
    if e.mem.u8(p + 0x30) == 0 {
        copy_words(e, DEFAULT_TEXTURE, out, 2);
        return;
    }
    let lock = f + 0x18;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x34);
    e.call(LOCK_TEXTURES, &args![data, 0u32, lock]);
    let base = e.mem.u32(lock);
    let corners = if base != 0 {
        corners_in(base, e.mem.u32(lock + 4), indices)
    } else {
        let data = e.mem.u32(obj.wrapping_add(0xb8));
        let textures = e.mem.u32(data.wrapping_add(0x2c));
        if textures == 0 {
            copy_words(e, DEFAULT_TEXTURE, out, 2);
            return;
        }
        corners_in(textures, 8, indices)
    };
    let texture: [u32; 2] = weighted(e, corners, weights.values(e));
    write_words(e, out, &texture);
}

/// The shape pick's normal of a hit (its frame `f`; the corner copies at +0x100, +0x118, +0x10c).
fn shape_record_normal(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    if e.mem.u8(p + 0x31) == 0 {
        copy_words(e, DEFAULT_NORMAL, record.wrapping_add(0x28), 3);
        return;
    }
    let lock = f + 0x148;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x34);
    e.call(LOCK_NORMALS, &args![data, lock]);
    let base = e.mem.u32(lock);
    let corners = if base != 0 {
        Some(corners_in(base, e.mem.u32(lock + 4), indices))
    } else {
        let normals = if e.mem.u8(p + 0x32) != 0 {
            let data = e.mem.u32(obj.wrapping_add(0xb8));
            e.mem.u32(data.wrapping_add(0x24))
        } else {
            0
        };
        (normals != 0).then(|| corners_in(normals, 12, indices))
    };
    let normal = match corners {
        Some(corners) => weighted::<3>(e, corners, weights.values(e)),
        None => shape_face_normal(e, [f + 0x100, f + 0x118, f + 0x10c]),
    };
    let rotate = obj.wrapping_add(0x68);
    finish_normal(e, normal, f + 0x44, f + 0x294, p, rotate, record);
}

/// The shape pick's face normal `(v1 - v0) x (v2 - v0)`, inline: each edge coordinate stored, then
/// each cross product coordinate the difference of two products in the exe's order.
fn shape_face_normal(e: &Engine, corners: [u32; 3]) -> [u32; 3] {
    let [v0, v1, v2] = corners;
    let [e1x, e1y, e1z] = [0u32, 4, 8].map(|i| stored(x87_sub(ld(e, v1 + i), ld(e, v0 + i))));
    let e2x = stored(x87_sub(ld(e, v2), ld(e, v0)));
    let e2y = stored(x87_sub(op(e, v2 + 4), ld(e, v0 + 4)));
    let e2z = stored(x87_sub(op(e, v2 + 8), ld(e, v0 + 8)));
    let x = x87_sub(x87_mul(e2z, e1y), x87_mul(e2y, e1z));
    let y = x87_sub(x87_mul(e1z, e2x), x87_mul(e2z, e1x));
    let z = x87_sub(x87_mul(e1x, e2y), x87_mul(e2x, e1y));
    [x87_store32(x), x87_store32(y), x87_store32(z)]
}

/// The shape pick's colour of a hit (its frame `f`).
fn shape_record_colour(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    let out = record.wrapping_add(0x34);
    if e.mem.u8(p + 0x33) == 0 {
        copy_words(e, DEFAULT_COLOUR, out, 4);
        return;
    }
    let lock = f + 0x13c;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x34);
    e.call(LOCK_COLOURS, &args![data, lock]);
    if e.mem.u32(lock) != 0 {
        let zeroed = [f + 0x154, f + 0x174, f + 0x164];
        for colour in zeroed {
            write_words(e, colour, &[0; 4]);
        }
        for (index, colour) in indices.into_iter().zip(zeroed) {
            e.call(LOCKED_COLOUR_AT, &args![lock, index, colour]);
        }
        let colour: [u32; 4] = weighted(e, zeroed, weights.values(e));
        write_words(e, out, &colour);
        return;
    }
    let data = e.mem.u32(obj.wrapping_add(0xb8));
    let colours = e.mem.u32(data.wrapping_add(0x28));
    if colours == 0 {
        copy_words(e, DEFAULT_COLOUR, out, 4);
        return;
    }
    let slots = CallSlots {
        third: f + 0x2a0,
        second: f + 0x2c0,
        first: f + 0x2d0,
        sum: f + 0x2b0,
        result: f + 0x284,
    };
    let pushed = weights.pushed(e);
    weighted_by_calls(
        e,
        COLOUR_TIMES_SCALAR,
        COLOUR_PLUS_COLOUR,
        corners_in(colours, 16, indices),
        pushed,
        slots,
    );
    copy_words(e, slots.result, out, 4);
}

// Typed from 00e9b4f0
/// The pick of a tri-based geometry (unnamed in the engine map; `cdecl`: the ray's origin and
/// direction, the pick, the geometry; the pick's walk calls it once the ray hits the geometry's
/// bound). With the pick's intersect type (+8) 0 or a skin instance (+0xbc): the four words at the
/// object's +0x20 (`011f4288` when null) are a centre, and when `(centre - origin) . direction` is
/// not below 0.0 (unordered counts as not below) a record gets `|centre - origin|` as its distance and,
/// for intersect type 1, the point `origin + direction * distance`; answers true, else false.
/// Otherwise an object whose class chain reaches `NiTriStrips` with one strip is picked by
/// [`pick_strips`], else one of class `NiTriShape` by [`pick_shape`]. The rest: the ray goes into model
/// space through `004b3ae0`; the data maybe locks its vertices; without vertices a record gets the
/// distance to the world translation and the answer is true (the data lock not released there).
/// Otherwise each triangle of the active count comes from the data's triangle query and is tested and
/// recorded as in the strips' pick (the point back to world space through `004b4500`, the scale and the
/// translation). Without a colour lock and without data colours the record's colour is left as it is.
pub fn pick_geometry(e: &mut Engine, origin: Ptr, direction: Ptr, pick: Ptr, object: Ptr) -> bool {
    let (o, dir, p, obj) = (origin.addr(), direction.addr(), pick.addr(), object.addr());
    e.with_stack(0x2f4, |e, frame| {
        let f = frame.addr();
        if e.mem.u32(p + 8) == 0 || e.mem.u32(obj.wrapping_add(0xbc)) != 0 {
            return pick_centre(e, f, o, dir, p, obj);
        }
        let mut rtti = e.vcall(obj, 8, &args![]).u32();
        let mut strips = false;
        while rtti != 0 {
            if rtti == TRI_STRIPS_RTTI {
                strips = true;
                break;
            }
            rtti = e.mem.u32(rtti.wrapping_add(4));
        }
        if strips {
            let data = e.mem.u32(obj.wrapping_add(0xb8));
            if e.mem.u16(data.wrapping_add(0x44)) == 1 {
                return typed_call!(
                    FAMILY,
                    0x00e9_93d0,
                    pick_strips(e, origin, direction, pick, object)
                );
            }
        } else if super::low_byte(e.call(IS_KIND_OF, &args![TRI_SHAPE_RTTI, obj]).u32()) != 0 {
            return typed_call!(
                FAMILY,
                0x00e9_a280,
                pick_shape(e, origin, direction, pick, object)
            );
        }
        pick_triangles(e, f, o, dir, p, obj)
    })
}

/// The general pick by the centre at the object's +0x20 (frame `f`).
fn pick_centre(e: &mut Engine, f: u32, o: u32, dir: u32, p: u32, obj: u32) -> bool {
    let mut centre = e.mem.u32(obj.wrapping_add(0x20));
    if centre == 0 {
        centre = DEFAULT_BOUND;
    }
    copy_words(e, centre, f + 0x168, 4);
    for i in [0u32, 4, 8] {
        let value = x87_sub(ld(e, f + 0x168 + i), op(e, o.wrapping_add(i)));
        st(e, f + 0x38 + i, value);
    }
    let a = x87_mul(ld(e, dir.wrapping_add(4)), op(e, f + 0x3c));
    let b = x87_mul(ld(e, f + 0x38), op(e, dir));
    let s = x87_add(a, b);
    let s = x87_add(s, x87_mul(ld(e, dir.wrapping_add(8)), op(e, f + 0x40)));
    st(e, f + 0x24, s);
    if ld(e, f + 0x24) < op(e, ZERO) {
        return false;
    }
    let record = add_record(e, p, obj);
    let x = ld(e, f + 0x38);
    let y = ld(e, f + 0x3c);
    let z = ld(e, f + 0x40);
    let xx = x87_mul(x, x);
    let yy = x87_mul(y, y);
    let sum = x87_add(yy, xx);
    st(e, f + 0x24, x87_add(x87_mul(z, z), sum));
    let root = e.call(SQUARE_ROOT, &args![ld(e, f + 0x24)]).f64();
    st(e, f + 0x24, root);
    let distance = x87_quiet_bits(e.mem.u32(f + 0x24));
    e.mem.set_u32(record.wrapping_add(0x14), distance);
    if e.mem.u32(p + 8) == 1 {
        let t = x87_load32(distance);
        let px = x87_store32(x87_mul(t, op(e, dir)));
        let py = x87_store32(x87_mul(ld(e, dir.wrapping_add(4)), t));
        let pz = x87_store32(x87_mul(t, op(e, dir.wrapping_add(8))));
        let x = x87_store32(x87_add(ld(e, o), x87_m32(px)));
        let y = x87_store32(x87_add(ld(e, o.wrapping_add(4)), x87_m32(py)));
        let z = x87_store32(x87_add(x87_load32(pz), op(e, o.wrapping_add(8))));
        write_words(e, record.wrapping_add(8), &[x, y, z]);
    }
    true
}

/// The general pick of the triangles from the data's triangle query (frame `f`).
fn pick_triangles(e: &mut Engine, f: u32, o: u32, dir: u32, p: u32, obj: u32) -> bool {
    let rotate = obj.wrapping_add(0x68);
    let at = |offset: u32| rotate.wrapping_add(offset);
    // The model ray through `004b3ae0`: the origin at +0x1d4, the direction at +0x1c8.
    let difference = f + 0x48;
    st(e, difference, x87_sub(ld(e, o), op(e, at(0x24))));
    st(
        e,
        difference + 4,
        x87_sub(ld(e, o.wrapping_add(4)), op(e, at(0x28))),
    );
    st(
        e,
        difference + 8,
        x87_sub(ld(e, o.wrapping_add(8)), op(e, at(0x2c))),
    );
    st(e, f + 0x28, x87_div(1.0, ld(e, at(0x30))));
    let model_origin = f + 0x1d4;
    let model_direction = f + 0x1c8;
    for (vector, out, temporary) in [
        (difference, model_origin, f + 0x20),
        (dir, model_direction, f + 0x28),
    ] {
        let inverse = ld(e, f + 0x28);
        let r = e
            .call(POINT_TIMES_MATRIX, &args![f + 0xc4, vector, rotate])
            .u32();
        st(e, f + 0x44, x87_mul(ld(e, r.wrapping_add(4)), inverse));
        st(e, temporary, x87_mul(ld(e, r.wrapping_add(8)), inverse));
        st(e, out, x87_mul(inverse, op(e, r)));
        st(e, out + 4, ld(e, f + 0x44));
        st(e, out + 8, ld(e, temporary));
    }
    let model = ModelSlots {
        difference,
        inverse: f + 0x28,
        origin: model_origin,
        direction: model_direction,
    };
    e.mem.set_u8(f + 0x1f, 0);
    let data = e.mem.u32(obj.wrapping_add(0xb8));
    let count = e.vcall(data, SLOT_TRIANGLE_COUNT, &args![]).u32() & 0xffff;
    let data = e.mem.u32(obj.wrapping_add(0xb8));
    let vertex_lock = f + 0xd0;
    clear_lock_block(e, vertex_lock);
    let vertices = e.mem.u32(data.wrapping_add(0x20));
    e.mem.set_u8(f + 0x1e, 0);
    e.mem.set_u32(f + 0x2c, data);
    let locked = lock_vertices(e, data, vertices, vertex_lock);
    if vertices == 0 && e.mem.u32(vertex_lock) == 0 {
        let record = add_record(e, p, obj);
        e.call(
            POINT_MINUS_POINT,
            &args![obj.wrapping_add(0x8c), f + 0xc4, o],
        );
        let length = e.call(POINT_LENGTH, &args![f + 0xc4]).f64();
        st(e, record.wrapping_add(0x14), length);
        return true;
    }
    let weights = Weights {
        w: f + 0x20,
        u: f + 0x30,
        v: f + 0x34,
    };
    let copies = [f + 0xa0, f + 0x94, f + 0x88];
    let mut triangle = 0u32;
    while super::low_half(triangle) < super::low_half(count) {
        let data = e.mem.u32(obj.wrapping_add(0xb8));
        e.vcall(
            data,
            SLOT_TRIANGLE_INDICES,
            &args![triangle, f + 0x10, f + 0x14, f + 0x18],
        );
        let indices = [0x10u32, 0x14, 0x18].map(|slot| u32::from(e.mem.u16(f + slot)));
        let sources = if vertices == 0 {
            let stride = e.mem.u32(vertex_lock + 4);
            corners_in(e.mem.u32(vertex_lock), stride, indices)
        } else {
            corners_in(vertices, 12, indices)
        };
        for (source, copy) in sources.into_iter().zip(copies) {
            copy_words(e, source, copy, 3);
        }
        let point = f + 0xac;
        if test_triangle(e, p, model, copies, f + 0x128, point, f + 0x24, weights) {
            e.mem.set_u8(f + 0x1f, 1);
            let record = add_record(e, p, obj);
            if e.mem.u32(p + 0xc) == 1 {
                let r = e
                    .call(MATRIX_TIMES_POINT, &args![rotate, f + 0x238, point])
                    .u32();
                st(e, f + 0x20, ld(e, at(0x30)));
                let scale = ld(e, f + 0x20);
                let x = x87_store32(x87_mul(scale, op(e, r)));
                let y = x87_store32(x87_mul(ld(e, r.wrapping_add(4)), scale));
                let z = x87_store32(x87_mul(scale, op(e, r.wrapping_add(8))));
                let x = x87_store32(x87_add(x87_load32(x), op(e, at(0x24))));
                let y = x87_store32(x87_add(ld(e, at(0x28)), x87_m32(y)));
                let z = x87_store32(x87_add(ld(e, at(0x2c)), x87_m32(z)));
                write_words(e, point, &[x, y, z]);
            }
            copy_words(e, point, record.wrapping_add(8), 3);
            let distance = x87_quiet_bits(e.mem.u32(f + 0x24));
            e.mem.set_u32(record.wrapping_add(0x14), distance);
            record_triangle(e, record, triangle, indices);
            weights.store_first(e);
            triangles_record_texture(e, f, p, obj, record, indices, weights);
            triangles_record_normal(e, f, p, obj, record, indices, weights);
            triangles_record_colour(e, f, p, obj, record, indices, weights);
            if stops_at_first_hit(e, p) {
                if locked {
                    let data = e.mem.u32(f + 0x2c);
                    e.call(DATA_UNLOCK, &args![data]);
                }
                return true;
            }
        }
        triangle = triangle.wrapping_add(1);
    }
    if locked {
        let data = e.mem.u32(f + 0x2c);
        e.call(DATA_UNLOCK, &args![data]);
    }
    e.mem.u8(f + 0x1f) != 0
}

/// The general pick's texture coordinates of a hit (its frame `f`; the data at +0x2c).
fn triangles_record_texture(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    let out = record.wrapping_add(0x20);
    if e.mem.u8(p + 0x30) == 0 {
        copy_words(e, DEFAULT_TEXTURE, out, 2);
        return;
    }
    let lock = f + 0x138;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x2c);
    e.call(LOCK_TEXTURES, &args![data, 0u32, lock]);
    let base = e.mem.u32(lock);
    if base != 0 {
        let corners = corners_in(base, e.mem.u32(lock + 4), indices);
        let texture: [u32; 2] = weighted(e, corners, weights.values(e));
        write_words(e, out, &texture);
        return;
    }
    let data = e.mem.u32(obj.wrapping_add(0xb8));
    let textures = e.mem.u32(data.wrapping_add(0x2c));
    if textures == 0 {
        copy_words(e, DEFAULT_TEXTURE, out, 2);
        return;
    }
    let slots = CallSlots {
        third: f + 0x200,
        second: f + 0x210,
        first: f + 0x208,
        sum: f + 0x218,
        result: f + 0x184,
    };
    let pushed = weights.pushed(e);
    weighted_by_calls(
        e,
        POINT2_TIMES_SCALAR,
        POINT2_PLUS_POINT2,
        corners_in(textures, 8, indices),
        pushed,
        slots,
    );
    copy_words(e, slots.result, out, 2);
}

/// The general pick's normal of a hit (its frame `f`; the corner copies at +0xa0, +0x94, +0x88).
fn triangles_record_normal(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    if e.mem.u8(p + 0x31) == 0 {
        copy_words(e, DEFAULT_NORMAL, record.wrapping_add(0x28), 3);
        return;
    }
    let lock = f + 0xdc;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x2c);
    e.call(LOCK_NORMALS, &args![data, lock]);
    let base = e.mem.u32(lock);
    let normal = if base != 0 {
        let corners = corners_in(base, e.mem.u32(lock + 4), indices);
        weighted::<3>(e, corners, weights.values(e))
    } else {
        let smooth = if e.mem.u8(p + 0x32) != 0 {
            let data = e.mem.u32(obj.wrapping_add(0xb8));
            e.mem.u32(data.wrapping_add(0x24))
        } else {
            0
        };
        let answer = if smooth != 0 {
            let slots = CallSlots {
                third: f + 0x220,
                second: f + 0x250,
                first: f + 0x244,
                sum: f + 0x25c,
                result: f + 0x268,
            };
            let pushed = weights.pushed(e);
            weighted_by_calls(
                e,
                SCALAR_TIMES_POINT,
                POINT_PLUS_POINT,
                corners_in(smooth, 12, indices),
                pushed,
                slots,
            )
        } else {
            let copies = [f + 0xa0, f + 0x94, f + 0x88];
            face_normal_by_call(e, copies, f + 0xc4, f + 0x1bc, f + 0x22c)
        };
        words3(e, answer)
    };
    let rotate = obj.wrapping_add(0x68);
    finish_normal(e, normal, f + 0x38, f + 0x168, p, rotate, record);
}

/// The general pick's colour of a hit (its frame `f`). Without a colour lock and without data colours
/// the record's colour is not written.
fn triangles_record_colour(
    e: &mut Engine,
    f: u32,
    p: u32,
    obj: u32,
    record: u32,
    indices: [u32; 3],
    weights: Weights,
) {
    let out = record.wrapping_add(0x34);
    if e.mem.u8(p + 0x33) == 0 {
        copy_words(e, DEFAULT_COLOUR, out, 4);
        return;
    }
    let lock = f + 0xb8;
    clear_lock_block(e, lock);
    let data = e.mem.u32(f + 0x2c);
    e.call(LOCK_COLOURS, &args![data, lock]);
    if e.mem.u32(lock) != 0 {
        let slots = CallSlots {
            third: f + 0x274,
            second: f + 0x294,
            first: f + 0x2b4,
            sum: f + 0x2d4,
            result: f + 0x1f0,
        };
        let pushed = weights.pushed(e);
        let zeroed = [f + 0x18c, f + 0x19c, f + 0x1ac];
        let result = locked_colours(e, lock, indices, zeroed, pushed, slots);
        copy_words(e, result, out, 4);
        return;
    }
    let data = e.mem.u32(obj.wrapping_add(0xb8));
    let colours = e.mem.u32(data.wrapping_add(0x28));
    if colours == 0 {
        return;
    }
    let slots = CallSlots {
        third: f + 0x284,
        second: f + 0x2a4,
        first: f + 0x2e4,
        sum: f + 0x2c4,
        result: f + 0x1e0,
    };
    let pushed = weights.pushed(e);
    weighted_by_calls(
        e,
        COLOUR_TIMES_SCALAR,
        COLOUR_PLUS_COLOUR,
        corners_in(colours, 16, indices),
        pushed,
        slots,
    );
    copy_words(e, slots.result, out, 4);
}
