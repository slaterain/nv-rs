//! `bhkBlendCollisionObject`: the collision object of a part-keyframed Havok body (an actor's ragdoll
//! bone), which blends its node and its body. Its fields as the code reads them: the node (+8), the
//! flags word (+0xc: bit 0 keyframed, bit 6 the node moved; byte +0xd bit 0 blend the position too),
//! the Havok object (+0x10), a set byte (+0x14), the blend amounts (+0x18, +0x1c, floats), the motion
//! mode it last set (+0x20) and the owner (+0x24).

use super::{body, FAMILY};
use crate::{culling, dispatch, slot_target};
use engine_core::ptr::Ptr;
use engine_core::sse::{addps, mulps, subps};
use engine_core::{args, typed_call, Engine};

/// The update state word of the thread block (`TLS + 0x2c0`): 1, 2 or 3 skip parts of the update.
const TLS_UPDATE_STATE: u32 = 0x2c0;
/// The object's slots: `hktoNiTransform` (+0xa4), the motion change's (+0xa8) and the node's own
/// transform update (+0xb8).
const SLOT_HK_TO_NI: u32 = 0xa4;
const SLOT_MOTION: u32 = 0xa8;
const SLOT_NODE_TRANSFORM: u32 = 0xb8;
/// The body's velocities set (`00561690`, `005617c0`, `thiscall(body, vector)`) from the shared
/// vector at `01267e30`.
const SET_LINEAR_VELOCITY: u32 = 0x0056_1690;
const SET_ANGULAR_VELOCITY: u32 = 0x0056_17c0;
const SHARED_VECTOR: u32 = 0x0126_7e30;
/// The Havok lock (`01267b80`: owner, count) and `bhkCollisionObject::SetMotionType` (`00c65f40`,
/// `thiscall(this, type, object, 0)`).
const LOCK_OBJECT: u32 = 0x0126_7b80;
const SET_MOTION_TYPE: u32 = 0x00c6_5f40;
/// `bhkCollisionObject::hktoNiTransform` (`00c65a80`): by address, a fix may replace it.
const COLLISION_HK_TO_NI: u32 = 0x00c6_5a80;
/// The motion notification (`00c81aa0`, `thiscall`).
const MOTION_NOTIFICATION: u32 = 0x00c8_1aa0;
/// The reciprocal's source (`011afe64`), the position scale (`01267e6c`) and the ones vector
/// (`010ec7f0`).
const RECIPROCAL_SOURCE: u32 = 0x011a_fe64;
const POSITION_SCALE: u32 = 0x0126_7e6c;
const ONES_VECTOR: u32 = 0x010e_c7f0;
/// The blend's callees, all by address: the transform's rotation into a Havok rotation (`0056dff0`),
/// a quaternion from it (`00cb26e0`), the slerp (`00cb2850`), the quaternion's normalisation
/// (`005611c0`), a record from a quaternion (`00cb2d90`), a transform from the record (`00624b10`),
/// a vector to floats (`00458620`) and the body's update from the blend (`00c8de20`).
const TRANSFORM_ROTATION_OUT: u32 = 0x0056_dff0;
const QUATERNION_SET: u32 = 0x00cb_26e0;
const QUATERNION_SET_SLERP: u32 = 0x00cb_2850;
const QUATERNION_STEP: u32 = 0x0056_11c0;
const RECORD_FROM_QUATERNION: u32 = 0x00cb_2d90;
const TRANSFORM_FROM_RECORD: u32 = 0x0062_4b10;
const VECTOR_TO_FLOATS: u32 = 0x0045_8620;
const UPDATE_FROM_BLEND: u32 = 0x00c8_de20;

/// The node's world-data refresh (`00a68bf0`, the culling family's).
fn refresh(e: &mut Engine, node: u32) {
    typed_call!(
        culling::FAMILY,
        0x00a6_8bf0,
        culling::update::update_world_transform(e, Ptr::new(node))
    );
}

// Typed from 00c817d0
/// `bhkBlendCollisionObject::UpdateWorldData` (`thiscall`, a word not read): nothing in update state
/// 3. A set byte +0x14 is cleared after refreshing the node's world data (when there is a node). Else,
/// unless the Havok object's word +0x10 has bit 6: the node-moved bit 6 of the flags byte moves the
/// body to the node (`NitohkTransform`) and clears that bit of the word (in update state 2 the world
/// data is refreshed instead); the keyframed bit 0 runs `hktoNiTransform` (slot +0xa4, directly when
/// it is this class's) when there is a Havok object, the blend amount is below 1.0 or the motion mode
/// is 1, and ends; every other case refreshes the node's world data.
pub fn update_world_data(e: &mut Engine, this: Ptr, _unused: u32) {
    let me = this.addr();
    let slot = e.tls() + TLS_UPDATE_STATE;
    let state = e.mem.u32(slot);
    if state == 3 {
        return;
    }
    if e.mem.u8(me + 0x14) != 0 {
        let target = e.mem.u32(me + 8);
        if target != 0 {
            refresh(e, target);
        }
        e.mem.set_u8(me + 0x14, 0);
        return;
    }
    let object = e.mem.u32(me + 0x10);
    let object_flag = object != 0 && (e.mem.u32(object + 0x10) >> 6) & 1 != 0;
    if !object_flag {
        let flags = e.mem.u8(me + 0xc);
        if (flags >> 6) & 1 != 0 {
            if state != 2 {
                typed_call!(FAMILY, 0x00c6_5b00, body::ni_to_hk(e, this));
                let word = e.mem.u16(me + 0xc);
                e.mem.set_u16(me + 0xc, word & 0xffbf);
                return;
            }
        } else if flags & 1 != 0 {
            let scale = e.mem.f32(me + 0x18);
            if object != 0 || 1.0 > scale || e.mem.u32(me + 0x20) == 1 {
                if slot_target(e, me, SLOT_HK_TO_NI) == 0x00c8_1e30 {
                    typed_call!(FAMILY, 0x00c8_1e30, hk_to_ni(e, this));
                } else {
                    e.vcall(me, SLOT_HK_TO_NI, &args![]);
                }
            }
            return;
        }
    }
    let target = e.mem.u32(me + 8);
    if target != 0 {
        refresh(e, target);
    }
}

// Typed from 00c81e30
/// `bhkBlendCollisionObject::hktoNiTransform` (`thiscall`): picks a target motion mode and a blend
/// mode from the blend amount (+0x18), the second amount (+0x1c), the position flag and the update
/// state: amount 0.0 gives target 1 (in state 1 only the node's world data is refreshed) and blend
/// mode 1 when the second amount is 0.0 or the flag is set, else 0; 1.0 gives target 4 and blend mode
/// 1 in state 2, else 2; any other amount target 1 and blend mode 1 in state 2, else 0. A changed
/// motion mode is set (slot +0xa8 when it was 4, else the body's velocities from the shared vector),
/// under the Havok lock, by `SetMotionType`, and remembered. Blend mode 0 blends the body into the
/// node ([`blend_into_node`]), 1 runs `bhkCollisionObject::hktoNiTransform` by address, 2 slot +0xa8.
/// After a change, an owner whose word +0x18 is 0 gets the motion notification.
pub fn hk_to_ni(e: &mut Engine, this: Ptr) {
    let me = this.addr();
    let source = e.global::<f32>(RECIPROCAL_SOURCE);
    let node = e.mem.u32(me + 8);
    let flag = e.mem.u8(me + 0xd) & 1 != 0;
    let hk = e.mem.u32(me + 0x10);
    let state_word = e.tls() + TLS_UPDATE_STATE;
    let first = e.mem.f32(me + 0x18);
    let (target, mode) = if first == 0.0 {
        if e.mem.u32(state_word) == 1 {
            refresh(e, node);
            return;
        }
        let second = e.mem.f32(me + 0x1c);
        (1u32, u32::from(second == 0.0 || flag))
    } else if 1.0 == first {
        let state = e.mem.u32(state_word);
        (4, if state == 2 { 1 } else { 2 })
    } else {
        let state = e.mem.u32(state_word);
        (1, u32::from(state == 2))
    };
    let current = e.mem.u32(me + 0x20);
    let mut changed = false;
    if current != target {
        if current == 4 {
            e.vcall(me, SLOT_MOTION, &args![]);
        } else if hk != 0 {
            e.call(SET_LINEAR_VELOCITY, &args![hk, SHARED_VECTOR]);
            e.call(SET_ANGULAR_VELOCITY, &args![hk, SHARED_VECTOR]);
        }
        typed_call!(
            dispatch::FAMILY,
            0x0040_fbf0,
            dispatch::refs::lock(e, Ptr::new(LOCK_OBJECT), 0u32)
        );
        e.call(SET_MOTION_TYPE, &args![this, target, hk, 0u32]);
        let count = e.global::<u32>(LOCK_OBJECT + 4).wrapping_sub(1);
        e.set_global(LOCK_OBJECT + 4, count);
        if count == 0 {
            e.set_global(LOCK_OBJECT, 0u32);
        }
        e.mem.set_u32(me + 0x20, target);
        changed = true;
    }
    match mode {
        0 => blend_into_node(e, me, node, hk, flag, source),
        1 => {
            e.call(COLLISION_HK_TO_NI, &args![this]);
        }
        2 => {
            e.vcall(me, SLOT_MOTION, &args![]);
        }
        _ => {}
    }
    if changed {
        let owner = e.mem.u32(me + 0x24);
        if owner != 0 && e.mem.u32(owner + 0x18) == 0 {
            e.call(MOTION_NOTIFICATION, &args![this]);
        }
    }
}

/// Four float lanes at `at`.
fn lanes(e: &Engine, at: u32) -> [f32; 4] {
    [0u32, 4, 8, 12].map(|k| e.mem.f32(at + k))
}

/// Four float lanes to `at`.
fn set_lanes(e: &mut Engine, at: u32, value: [f32; 4]) {
    for (k, lane) in (0u32..).zip(value) {
        e.mem.set_f32(at + 4 * k, lane);
    }
}

/// The reciprocal `FLD1; FDIVRP` makes, stored as a float; 1.0 for a zero source. Native maths
/// (ADR-0014): one division of floats stored at once, so plain `f32` gives the exe's float on either
/// thread's word (a double quotient rounded to a float is the correctly rounded float quotient).
fn reciprocal(source: f32) -> f32 {
    if source == 0.0 {
        1.0
    } else {
        1.0 / source
    }
}

/// The scaled position's lane: one product of floats stored as a float, plain `f32` on either word.
fn scaled_lane(value: f32, scale: f32) -> f32 {
    value * scale
}

/// Blend mode 0 of [`hk_to_ni`]: the node's world data refreshed; the node's translation scaled
/// (+0x00 of the exe's 0xd0-byte frame), its rotation as a Havok rotation (+0xa0) and a quaternion
/// (+0x30); the body's position (+0x50) and rotation (+0x40) read (slots +0xd4, +0xd8); the position
/// blended `(1 - amount) * body + amount * node` lane by lane as the exe's SSE (+0x20) and the
/// rotation slerped (+0x10, normalised); the position (blended with the flag, the node's without it)
/// and the quaternion make a record (+0x60) that becomes the node's transform and translation, the
/// node's slot +0xb8 runs in update state 0, and the body is updated from the blend (`00c8de20`).
fn blend_into_node(e: &mut Engine, this: u32, node: u32, hk: u32, flag: bool, source: f32) {
    refresh(e, node);
    let transform = node + 0x68;
    let translation = transform + 0x24;
    let scale = e.global::<f32>(POSITION_SCALE);
    let state_word = e.tls() + TLS_UPDATE_STATE;
    e.with_stack(0xd0, |e, frame| {
        let f = frame.addr();
        let scaled = f;
        let blended_quaternion = f + 0x10;
        let blended_position = f + 0x20;
        let node_quaternion = f + 0x30;
        let havok_quaternion = f + 0x40;
        let havok_position = f + 0x50;
        let record = f + 0x60;
        let buffer = f + 0xa0;
        for lane in 0..3u32 {
            let value = e.mem.f32(translation + 4 * lane);
            e.mem.set_f32(scaled + 4 * lane, scaled_lane(value, scale));
        }
        e.mem.set_f32(scaled + 0xc, 0.0);
        e.call(TRANSFORM_ROTATION_OUT, &args![buffer, transform]);
        e.call(QUATERNION_SET, &args![node_quaternion, buffer]);
        e.vcall(hk, 0xd4, &args![havok_position]);
        e.vcall(hk, 0xd8, &args![havok_quaternion]);
        let blend = e.mem.f32(this + 0x18);
        let ones = lanes(e, ONES_VECTOR);
        let position = lanes(e, havok_position);
        let own = lanes(e, scaled);
        // The exe's SSE in its operand order, so where both operands are NaN the destination's NaN
        // is kept: SUBPS ones, the splatted blend (00c82089); MULPS that, the Havok position
        // (00c8208c); MULPS the scaled position, the blend (00c8209d); ADDPS the first, the second
        // (00c820a1).
        let splat = [blend; 4];
        let havok_side = mulps(subps(ones, splat), position);
        let mixed = addps(havok_side, mulps(own, splat));
        set_lanes(e, blended_position, mixed);
        let blend_bits = e.mem.u32(this + 0x18);
        e.call(
            QUATERNION_SET_SLERP,
            &args![
                blended_quaternion,
                havok_quaternion,
                node_quaternion,
                blend_bits
            ],
        );
        e.call(QUATERNION_STEP, &args![blended_quaternion]);
        let chosen = if flag { blended_position } else { scaled };
        for lane in 0..4u32 {
            let value = e.mem.u32(chosen + 4 * lane);
            e.mem.set_u32(record + 0x30 + 4 * lane, value);
        }
        e.call(RECORD_FROM_QUATERNION, &args![record, blended_quaternion]);
        e.call(TRANSFORM_FROM_RECORD, &args![transform, record]);
        e.call(VECTOR_TO_FLOATS, &args![translation, record + 0x30]);
        if e.mem.u32(state_word) == 0 {
            e.vcall(this, SLOT_NODE_TRANSFORM, &args![transform]);
        }
        let inverse = reciprocal(source);
        let second = if flag { 1.0f32 } else { e.mem.f32(this + 0x1c) };
        e.call(
            UPDATE_FROM_BLEND,
            &args![
                hk,
                blended_position,
                blended_quaternion,
                inverse.to_bits(),
                second.to_bits()
            ],
        );
    });
}
