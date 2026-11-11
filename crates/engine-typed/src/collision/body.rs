//! `bhkCollisionObject::NitohkTransform` (moving a Havok body to its node), the quaternion of a
//! rotation matrix it needs, and the body's linear velocity it reads. A collision object
//! (`bhkCollisionObject`, 0x14 bytes, Xbox PDB) holds its node (+8), its flags (+0xc, bit 6 move the
//! body) and its Havok body (+0x10); the body's motion type is the byte +0xe8 of its Havok object
//! (+8), 5 without one.

use super::FAMILY;
use crate::{culling, dispatch};
use engine_core::ptr::Ptr;
use engine_core::x87::x87_sub;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32,
};
use engine_core::{args, typed_call, Engine};

/// The Havok world of a body (`00c87a40`), the object test (`0047b470`) and the motion type test
/// (`00c8cce0`), the hard keyframe (`00c8e160`, `(body, translation, quaternion, lock)`), the
/// quaternion's normalisation (`005611c0`), all by address.
const GET_HK_WORLD: u32 = 0x00c8_7a40;
const OBJECT_TEST: u32 = 0x0047_b470;
const MOTION_TYPE_TEST: u32 = 0x00c8_cce0;
const APPLY_HARD_KEYFRAME: u32 = 0x00c8_e160;
const NORMALIZE_QUATERNION: u32 = 0x0056_11c0;
/// The body's velocities set (`00561690`, `005617c0`, `thiscall(body, vector)`), by address.
const SET_LINEAR_VELOCITY: u32 = 0x0056_1690;
const SET_ANGULAR_VELOCITY: u32 = 0x0056_17c0;
/// The Havok lock (`01267b80`: owner, count), the zero vector (`01267e30`), the velocity epsilon
/// (`01017d00`) and the absolute-value mask (`01017d10`).
const LOCK: u32 = 0x0126_7b80;
const ZERO_VECTOR: u32 = 0x0126_7e30;
const VELOCITY_EPSILON: u32 = 0x0101_7d00;
const ABS_MASK: u32 = 0x0101_7d10;
/// The shared zero vector a body without an entity answers (`00458b20`) and the vector's finish
/// (`0045c650`), by address.
const ZERO_VECTOR_GETTER: u32 = 0x0045_8b20;
const VECTOR_FINISH: u32 = 0x0045_c650;
/// `NiQuaternion::FromRotation`'s constants: 1.0 and 0.5 (doubles at `01012070`, `01011588`) and the
/// next-index table (`011a9e98`).
const ONE_DOUBLE: u32 = 0x0101_2070;
const HALF_DOUBLE: u32 = 0x0101_1588;
const NEXT_INDEX: u32 = 0x011a_9e98;

/// Whether every lane of the vector at `vector` is within the epsilon of the zero vector's (the
/// exe's SSE compare of the masked difference).
fn velocity_within_epsilon(e: &Engine, vector: u32, lanes: u32) -> bool {
    let epsilon = e.mem.f32(VELOCITY_EPSILON);
    (0..lanes).all(|lane| {
        let difference = e.mem.f32(vector + 4 * lane) - e.mem.f32(ZERO_VECTOR + 4 * lane);
        let magnitude = f32::from_bits(difference.to_bits() & e.mem.u32(ABS_MASK + 4 * lane));
        magnitude <= epsilon
    })
}

/// Releases the Havok lock in line: the count (+4) lowered, the owner cleared at 0.
fn release_lock(e: &mut Engine) {
    let count = e.mem.u32(LOCK + 4).wrapping_sub(1);
    e.mem.set_u32(LOCK + 4, count);
    if count == 0 {
        e.mem.set_u32(LOCK, 0);
    }
}

/// Moves the body to the node: the position (`004a3e00` on the node's translation), the rotation as
/// the quaternion `(x, y, z, w)` of the `NiQuaternion` `(w, x, y, z)` at `quaternion` (each word
/// through `FLD`/`FSTP`), normalised (`005611c0`), then, under the Havok lock (released in line), the
/// body's slot +0xe8 with both.
fn move_body(e: &mut Engine, body: u32, translation: u32, quaternion: u32) {
    e.with_stack(0x20, |e, vectors| {
        let position = vectors.addr();
        let rotation = position + 0x10;
        typed_call!(
            dispatch::FAMILY,
            0x004a_3e00,
            dispatch::math::havok_vector(e, Ptr::new(position), Ptr::new(translation))
        );
        for (target, source) in [(0, 4), (4, 8), (8, 0xc), (0xc, 0)] {
            let bits = x87_quiet_bits(e.mem.u32(quaternion + source));
            e.mem.set_u32(rotation + target, bits);
        }
        e.call(NORMALIZE_QUATERNION, &args![rotation]);
        typed_call!(
            dispatch::FAMILY,
            0x0040_fbf0,
            dispatch::refs::lock(e, Ptr::new(LOCK), 0u32)
        );
        e.vcall(body, 0xe8, &args![position, rotation]);
    });
    release_lock(e);
}

// Typed from 00c65b00
/// `bhkCollisionObject::NitohkTransform` (`thiscall`): moves the Havok body to the node. Nothing
/// without a node (its world data is refreshed first, `00a68bf0`) or a body; a body of motion type 5
/// whose Havok world (`00c87a40`) is set does nothing, nor one without a Havok object. The node's
/// rotation becomes a quaternion ([`quaternion_from_rotation`]). Then: the object's bit 6 moves the
/// body ([`move_body`]); a node whose flags (+0x30) have none of bits 3, 4, 22, 27 compares the body's
/// position (slot +0xd4) with its translation: within 1.0 (squared, NaN near) the body's linear and
/// angular velocities ([`linear_velocity`], `00560e10`) are zeroed (`00561690`, `005617c0`) unless
/// within the epsilon, and it ends; otherwise (and with those flag bits) the body is hard keyframed
/// (`00c8e160`) when `0047b470` or `00c8cce0` answer true, and moved otherwise.
pub fn ni_to_hk(e: &mut Engine, this: Ptr) {
    let node = e.mem.u32(this.addr() + 8);
    let body = e.mem.u32(this.addr() + 0x10);
    if node == 0 {
        return;
    }
    typed_call!(
        culling::FAMILY,
        0x00a6_8bf0,
        culling::update::update_world_transform(e, Ptr::new(node))
    );
    if body == 0 {
        return;
    }
    let object = e.mem.u32(body + 8);
    let motion_type = if object != 0 {
        u32::from(e.mem.u8(object + 0xe8))
    } else {
        5
    };
    if motion_type == 5 && e.call(GET_HK_WORLD, &args![body]).u32() != 0 {
        return;
    }
    if e.mem.u32(body + 8) == 0 {
        return;
    }
    let rotation = node + 0x68;
    let translation = node + 0x8c;
    // The quaternion (16 bytes) and the word holding the motion type.
    e.with_stack(0x20, |e, frame| {
        let quaternion = frame.addr();
        let type_word = quaternion + 0x10;
        e.mem.set_u32(type_word, motion_type);
        typed_call!(
            FAMILY,
            0x00a6_df40,
            quaternion_from_rotation(e, Ptr::new(quaternion), Ptr::new(rotation))
        );
        let flags = e.mem.u8(this.addr() + 0xc);
        if (flags >> 6) & 1 != 0 {
            move_body(e, body, translation, quaternion);
            return;
        }
        let node_flags = e.mem.u32(node + 0x30);
        let flagged = [0x16, 0x1b, 3, 4]
            .into_iter()
            .any(|bit| (node_flags >> bit) & 1 != 0);
        if !flagged {
            let squared_distance = e.with_stack(0x20, |e, vectors| {
                let wanted = vectors.addr();
                let current = wanted + 0x10;
                typed_call!(
                    dispatch::FAMILY,
                    0x004a_3e00,
                    dispatch::math::havok_vector(e, Ptr::new(wanted), Ptr::new(translation))
                );
                e.vcall(body, 0xd4, &args![current]);
                let squares: [f32; 4] = [0u32, 4, 8, 12].map(|lane| {
                    let difference = e.mem.f32(wanted + lane) - e.mem.f32(current + lane);
                    difference * difference
                });
                squares[2] + (squares[1] + squares[0])
            });
            let far = squared_distance > 1.0;
            if !far {
                // Near: zero the velocities that are not already (nearly) zero.
                let linear = typed_call!(FAMILY, 0x0056_0d80, linear_velocity(e, Ptr::new(body)));
                if !velocity_within_epsilon(e, linear.addr(), 4) {
                    e.call(SET_LINEAR_VELOCITY, &args![body, ZERO_VECTOR]);
                }
                let angular = typed_call!(
                    dispatch::FAMILY,
                    0x0056_0e10,
                    dispatch::scene::body_vector(e, Ptr::new(body))
                );
                if !velocity_within_epsilon(e, angular.addr(), 4) {
                    e.call(SET_ANGULAR_VELOCITY, &args![body, ZERO_VECTOR]);
                }
                return;
            }
        }
        if e.call(OBJECT_TEST, &args![this]).bool()
            || e.call(MOTION_TYPE_TEST, &args![type_word]).bool()
        {
            e.call(
                APPLY_HARD_KEYFRAME,
                &args![body, translation, quaternion, LOCK],
            );
            return;
        }
        move_body(e, body, translation, quaternion);
    });
}

/// `FLD dword [a]`.
fn ld(e: &Engine, a: u32) -> f64 {
    x87_load32(e.mem.u32(a))
}

/// A float memory operand at `a`.
fn m32(e: &Engine, a: u32) -> f64 {
    x87_m32(e.mem.u32(a))
}

/// `FSTP dword [a]`.
fn fst(e: &mut Engine, a: u32, v: f64) {
    e.mem.set_u32(a, x87_store32(v));
}

/// `FSTP dword tmp; FLD dword tmp`: the value rounded to a float and loaded back.
fn spill(v: f64) -> f64 {
    x87_load32(x87_store32(v))
}

/// The square root (`00ec6040`, the crt family's) of `x`.
fn root(e: &mut Engine, x: f64) -> f64 {
    typed_call!(
        crate::crt::FAMILY,
        0x00ec_6040,
        crate::crt::math::ci_sqrt(e, x)
    )
}

// Typed from 00a6df40
/// `NiQuaternion::FromRotation` (`thiscall(this, matrix)`, row-major floats): with a positive trace
/// the root is `sqrt(trace + 1)`; otherwise the largest diagonal element `i` (with `j` and `k` from the
/// exe's next-index table) gives the root `sqrt(m_ii - m_jj - m_kk + 1)`, and the vector components
/// are written through a local table of pointers to `x`, `y`, `z`, indexed as the exe indexes it.
pub fn quaternion_from_rotation(e: &mut Engine, this: Ptr, matrix: Ptr) {
    let q = this.addr();
    let m = matrix.addr();
    let at = |n: u32| m.wrapping_add(n.wrapping_mul(4));
    let s1 = x87_add(ld(e, at(0)), m32(e, at(4)));
    let trace = spill(x87_add(s1, m32(e, at(8))));
    // FCOM against 0; TEST AH,0x41: below, equal or unordered take the other branch.
    if trace > 0.0 {
        let x = spill(x87_add(trace, e.mem.f64(ONE_DOUBLE)));
        let r = spill(spill(root(e, x)));
        let h = e.mem.f64(HALF_DOUBLE);
        fst(e, q, x87_mul(r, h));
        let k = spill(x87_div(h, r));
        let d1 = x87_sub(ld(e, at(7)), m32(e, at(5)));
        fst(e, q + 4, x87_mul(d1, k));
        let d2 = x87_sub(ld(e, at(2)), m32(e, at(6)));
        fst(e, q + 8, x87_mul(d2, k));
        let d3 = x87_sub(ld(e, at(3)), m32(e, at(1)));
        fst(e, q + 0xc, x87_mul(k, d3));
        return;
    }
    // FCOMPP; TEST AH,0x5; JP: only "below" picks the other element.
    let mut i = 0u32;
    if ld(e, at(0)) < ld(e, at(4)) {
        i = 1;
    }
    let diag = |n: u32| m.wrapping_add(n << 4);
    if ld(e, diag(i)) < ld(e, at(8)) {
        i = 2;
    }
    let j = e.mem.u32(NEXT_INDEX.wrapping_add(i.wrapping_mul(4)));
    let k = e.mem.u32(NEXT_INDEX.wrapping_add(j.wrapping_mul(4)));
    let a = x87_sub(ld(e, diag(i)), m32(e, diag(j)));
    let b = x87_sub(a, m32(e, diag(k)));
    let x = spill(x87_add(b, e.mem.f64(ONE_DOUBLE)));
    let r = spill(spill(root(e, x)));
    let index = |row: u32, column: u32| at(row.wrapping_mul(3).wrapping_add(column));
    e.with_stack(12, |e, slots| {
        let slots = slots.addr();
        e.mem.set_u32(slots, q + 4);
        e.mem.set_u32(slots + 4, q + 8);
        e.mem.set_u32(slots + 8, q + 0xc);
        let slot = |n: u32| slots.wrapping_add(n.wrapping_mul(4));
        let h = e.mem.f64(HALF_DOUBLE);
        let qi = e.mem.u32(slot(i));
        let half_root = spill(x87_mul(r, h));
        fst(e, qi, half_root);
        let kf = spill(x87_div(h, r));
        let wd = x87_sub(ld(e, index(k, j)), m32(e, index(j, k)));
        fst(e, q, x87_mul(wd, kf));
        let sj = x87_add(ld(e, index(i, j)), m32(e, index(j, i)));
        let qj = e.mem.u32(slot(j));
        fst(e, qj, spill(x87_mul(sj, kf)));
        let sk = x87_add(ld(e, index(i, k)), m32(e, index(k, i)));
        let qk = e.mem.u32(slot(k));
        fst(e, qk, spill(x87_mul(kf, sk)));
    });
}

// Typed from 00560d80
/// A reference's linear velocity (`thiscall`): with a character proxy (`004ae750`) its entity's
/// ([`linear_velocity_of`]), otherwise the shared zero vector (`00458b20`).
pub fn linear_velocity(e: &mut Engine, this: Ptr) -> Ptr {
    let entity = typed_call!(
        dispatch::FAMILY,
        0x004a_e750,
        dispatch::scene::character_proxy(e, this)
    );
    if entity == 0 {
        e.call(ZERO_VECTOR_GETTER, &args![]).ptr()
    } else {
        linear_velocity_of(e, Ptr::new(entity))
    }
}

// Typed from 00560dc0
/// An entity's linear velocity (`thiscall`): its part at +0xe0 (`009d9f40`), finished by `0045c650`.
pub fn linear_velocity_of(e: &mut Engine, this: Ptr) -> Ptr {
    let part = typed_call!(
        dispatch::FAMILY,
        0x009d_9f40,
        dispatch::scene::part_e0(e, this)
    );
    e.call(VECTOR_FINISH, &args![part]).ptr()
}
