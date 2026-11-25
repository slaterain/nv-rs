//! `TESHavokUtilities::ConsolidateSimIslands` (Xbox PDB): a reference's rigid bodies that share the
//! first body's Havok world have their simulation islands merged into the first body's. The
//! after-draw stage's per-actor step (`009781d0`, the process family) calls it for each actor of the
//! high process level, and `008885e0` for the player; its loop over the bodies made about 1,600
//! calls by address a frame on the crowded Goodsprings street (the array element and the island).
//!
//! The body array is a Havok pointer array (`hkArray`, 12 bytes: data, size, capacity and flags)
//! on the stack; its element (`00629ab0`: the address `data + 4 * index`) is read in place, the
//! size and each body's world (`00726070`, `0044ddc0`: words +4 and +8) are leaf getters, the
//! island (`009ad610`: word +0xcc) and the two calls on the world around the loop (`00483710`, an
//! empty body) are the dispatch family's, all as typed calls. What stays by address: the array's
//! construction and destruction (`006299a0`, `00629b40`), the gathering of the bodies (`00c7be20`
//! from the ragdoll controller, else `00c80930` from the reference's 3D, its slot +0x1d0) and the
//! merge (`hkpWorldOperationUtil::mergeIslands`, `00cb5570`).
//!
//! No float maths. As in the translation, the exception frame's unwind (which destroys the array
//! when a callee throws) is behaviour only.

use crate::dispatch;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The body array's constructor and destructor (`thiscall(array)`), by address.
const ARRAY_CONSTRUCT: u32 = 0x0062_99a0;
const ARRAY_DESTRUCT: u32 = 0x0062_9b40;
/// `bhkRagdollController::GetRBForBones` (`thiscall(controller, array)`) and
/// `bhkUtilFunctions::GatherRigidBodies` (`cdecl(node, array)`), by address.
const GET_RB_FOR_BONES: u32 = 0x00c7_be20;
const GATHER_RIGID_BODIES: u32 = 0x00c8_0930;
/// `hkpWorldOperationUtil::mergeIslands` (`cdecl(world, first, other)`), by address.
const MERGE_ISLANDS: u32 = 0x00cb_5570;
/// The reference's ragdoll controller (+0xac) and its slot of its 3D (+0x1d0).
const RAGDOLL_CONTROLLER: u32 = 0xac;
const SLOT_GET_3D: u32 = 0x1d0;

/// The array's size (`00726070`), as the exe compares it: signed.
fn size(e: &mut Engine, bodies: Ptr) -> i32 {
    let size: u32 = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, bodies));
    size as i32
}

/// The body at `index` (`00629ab0`'s element, read in place: `data + 4 * index`).
fn body_at(e: &Engine, bodies: Ptr, index: i32) -> u32 {
    let slot = e
        .mem
        .u32(bodies.addr())
        .wrapping_add((index as u32).wrapping_mul(4));
    e.mem.u32(slot)
}

/// A body's world (`0044ddc0`: its word +8).
fn world_of(e: &mut Engine, body: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, body))
}

/// A body's simulation island (`009ad610`: its word +0xcc).
fn island_of(e: &mut Engine, body: u32) -> u32 {
    let island: Ptr = typed_call!(
        dispatch::FAMILY,
        0x009a_d610,
        dispatch::refs::word_cc(e, Ptr::new(body))
    );
    island.addr()
}

// Typed from 0062c430
/// `TESHavokUtilities::ConsolidateSimIslands(reference)` (`cdecl`): with a reference, its rigid
/// bodies gathered into a local array (from its ragdoll controller at +0xac, else from its 3D); with
/// any, and the first body in a world, between the world's two `00483710` calls, every other body
/// (the size read again each step) that is in the same world and in another island than the first
/// body (both islands read again each step: a merge can change either) has its island merged into
/// the first one's. The array destroyed at the end.
pub fn consolidate_sim_islands(e: &mut Engine, reference: Ptr) {
    if reference.is_null() {
        return;
    }
    e.with_stack(12, |e, bodies| {
        e.call(ARRAY_CONSTRUCT, &args![bodies]);
        let controller = e.mem.u32(reference.addr() + RAGDOLL_CONTROLLER);
        if controller != 0 {
            e.call(GET_RB_FOR_BONES, &args![controller, bodies]);
        } else {
            let node = e.vcall(reference.addr(), SLOT_GET_3D, &[]).u32();
            e.call(GATHER_RIGID_BODIES, &args![node, bodies]);
        }
        if size(e, bodies) != 0 {
            let first = body_at(e, bodies, 0);
            let world = world_of(e, first);
            if world != 0 {
                typed_call!(
                    dispatch::FAMILY,
                    0x0048_3710,
                    dispatch::refs::nothing_1(e, world)
                );
                let mut index = 1i32;
                while index < size(e, bodies) {
                    let first = body_at(e, bodies, 0);
                    let first_island = island_of(e, first);
                    let other = body_at(e, bodies, index);
                    let other_island = island_of(e, other);
                    let other = body_at(e, bodies, index);
                    if world == world_of(e, other) && first_island != other_island {
                        let other = body_at(e, bodies, index);
                        let first = body_at(e, bodies, 0);
                        e.call(MERGE_ISLANDS, &args![world, first, other]);
                    }
                    index = index.wrapping_add(1);
                }
                typed_call!(
                    dispatch::FAMILY,
                    0x0048_3710,
                    dispatch::refs::nothing_1(e, world)
                );
            }
        }
        e.call(ARRAY_DESTRUCT, &args![bodies]);
    });
}
