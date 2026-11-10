//! The queued node updates: with more than one thread, the AI threads queue the scene graphs they
//! changed (the pending list at `011ca174`, under its lock at `011ca1a0`), and main runs them after
//! the frame is shown (`Main::PostSwapProcess`, `008705d0`): each queued node's geometries get their
//! update (the update's entry, `00a59c60`, a Rust call while the scene family is on), and a node
//! that still has a parent and is flagged as waiting tells its reference's cell about it.
//!
//! The three helpers are called as the translation calls them, as plain Rust calls inside this
//! file (docs/ENGINE_CRATE.md, "Calls": one unit), so nothing at their addresses sees those calls.

use super::FAMILY;
use crate::dispatch;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The pending list and its lock.
const PENDING_LIST: u32 = 0x011c_a174;
const PENDING_LOCK: u32 = 0x011c_a1a0;
/// The list's `IsEmpty` (`004a4460`) and `RemoveHead` (`004ee8a0`), both `thiscall`, no words.
const LIST_IS_EMPTY: u32 = 0x004a_4460;
const LIST_REMOVE_HEAD: u32 = 0x004e_e8a0;
/// The smart pointer's copy into a slot (`00559a40`, `thiscall(slot, from)`).
const SLOT_COPY: u32 = 0x0055_9a40;
/// `TESObjectREFR::FindReferenceFor3D` (`0056f930`, cdecl, the node), the reference's parent cell
/// (`008d6f30`, `thiscall`) and the cell's update for a reference (`0054a070`, `thiscall(cell,
/// reference, 1, 0)`).
const FIND_REFERENCE_FOR_3D: u32 = 0x0056_f930;
const PARENT_CELL: u32 = 0x008d_6f30;
const CELL_UPDATE_FOR_REFERENCE: u32 = 0x0054_a070;
/// An object's flag bits set or cleared (`0043b370`, `thiscall(object, set, mask)`).
const SET_FLAG_BITS: u32 = 0x0043_b370;
/// The flag of a node whose reference waits for its cell's update.
const WAITING_FLAG: u32 = 0x0040_0000;
/// The update data's constructor (`0043d410`, `thiscall(data, time, 1, 0)`), its size and the time
/// it is given (`011c3c08`).
const UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
const UPDATE_DATA_SIZE: u32 = 12;
const UPDATE_TIME: u32 = 0x011c_3c08;
/// Vtable slots: the object as a node (+0x0c, null for any other class), its parent (+0x10) and
/// the object as a geometry (+0x30, null for any other class).
const SLOT_AS_NODE: u32 = 0x0c;
const SLOT_PARENT: u32 = 0x10;
const SLOT_AS_GEOMETRY: u32 = 0x30;

// Typed from 00552570
/// The queued node updates (cdecl, no words): under the pending list's lock, while the list is not
/// empty, its head is copied into a held slot and removed, the node's geometries are updated
/// ([`update_geometries`]), and a node that still has a parent (slot +0x10) and the waiting flag
/// ([`waiting`]) has its reference (`FindReferenceFor3D`) tell its parent cell about it
/// (`0054a070` with 1 and 0) and the flag cleared ([`set_waiting`]); the slot is released.
pub fn run_queued(e: &mut Engine) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, Ptr::new(PENDING_LOCK), 0u32)
    );
    while !e.call(LIST_IS_EMPTY, &args![PENDING_LIST]).bool() {
        e.with_stack(4, |e, slot| {
            let head = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, PENDING_LIST));
            e.call(SLOT_COPY, &args![slot, head]);
            e.call(LIST_REMOVE_HEAD, &args![PENDING_LIST]);
            let node = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
            update_geometries(e, Ptr::new(node));
            let node = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
            if node != 0 {
                let node = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
                if e.vcall(node, SLOT_PARENT, &[]).u32() != 0 {
                    let node = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
                    if waiting(e, Ptr::new(node)) {
                        let node = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
                        let reference = e.call(FIND_REFERENCE_FOR_3D, &args![node]).u32();
                        if reference != 0 && e.call(PARENT_CELL, &args![reference]).u32() != 0 {
                            let cell = e.call(PARENT_CELL, &args![reference]).u32();
                            e.call(
                                CELL_UPDATE_FOR_REFERENCE,
                                &args![cell, reference, 1u32, 0u32],
                            );
                            let node =
                                typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
                            set_waiting(e, Ptr::new(node), false);
                        }
                    }
                }
            }
            typed_call!(
                dispatch::FAMILY,
                0x0045_cec0,
                dispatch::refs::release_held(e, slot)
            );
        });
    }
    typed_call!(
        dispatch::FAMILY,
        0x0040_fba0,
        dispatch::refs::unlock(e, Ptr::new(PENDING_LOCK))
    );
}

// Typed from 005526b0
/// Whether the node has the waiting flag (`thiscall`): its flag word's `0x400000` (`00456630`).
pub fn waiting(e: &mut Engine, this: Ptr) -> bool {
    typed_call!(
        leaf::FAMILY,
        0x0045_6630,
        leaf::flags_at_30_any(e, this, WAITING_FLAG)
    )
}

// Typed from 005526d0
/// Sets or clears the node's waiting flag (`thiscall`, one word): `0043b370` with the word and
/// the flag.
pub fn set_waiting(e: &mut Engine, this: Ptr, set: bool) {
    e.call(SET_FLAG_BITS, &args![this, set, WAITING_FLAG]);
}

// Typed from 005526f0
/// The geometries' update under a node (cdecl, one word): a node (slot +0x0c answers it as a node)
/// walks its children (`0043b480`, `0043b4a0`) and recurses; an object that is a geometry (slot
/// +0x30) gets the update's entry (`00a59c60`) with update data made of the time at `011c3c08`, 1
/// and 0 (`0043d410`). A null node does nothing.
pub fn update_geometries(e: &mut Engine, node: Ptr) {
    if node.is_null() {
        return;
    }
    let as_node = e.vcall(node.addr(), SLOT_AS_NODE, &[]).u32();
    if as_node != 0 {
        let count = typed_call!(
            dispatch::FAMILY,
            0x0043_b480,
            dispatch::scene::child_count(e, Ptr::new(as_node))
        );
        for index in 0..u32::from(count) {
            let child = typed_call!(
                dispatch::FAMILY,
                0x0043_b4a0,
                dispatch::scene::child(e, Ptr::new(as_node), index)
            );
            update_geometries(e, child);
        }
    } else {
        let geometry = e.vcall(node.addr(), SLOT_AS_GEOMETRY, &[]).u32();
        if geometry != 0 {
            let time = e.global::<f32>(UPDATE_TIME);
            e.with_stack(UPDATE_DATA_SIZE, |e, data| {
                e.call(UPDATE_DATA_CONSTRUCT, &args![data, time, 1u32, 0u32]);
                typed_call!(
                    FAMILY,
                    0x00a5_9c60,
                    super::walk::update(e, Ptr::new(geometry), data.addr())
                );
            });
        }
    }
}
