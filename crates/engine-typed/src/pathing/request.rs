//! The safe straight-line pathing request (Xbox PDB names): its constructor, copy constructor,
//! scalar deleting destructor, `CopyTo`, the target setter as compiled, and the save/load pair.
//!
//! No floats: each function moves words and makes the calls the translations make, by address.

use super::layout::PathingRequestSafeStraightLine;
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The `PathingRequestSafeStraightLine` vtable (by its RTTI), at `0106d414`.
const VTABLE: u32 = 0x0106_d414;
/// `006d3b20`: the destructor body (the `PathingRequestClosePoint` chain).
const DTOR_BODY: u32 = 0x006d_3b20;
/// `00aa1460` (cdecl, two words): frees the 0xbc-byte object.
const FREE: u32 = 0x00aa_1460;
/// `006e3f30` / `006e3fb0`: the `PathingRequestClosePoint` constructor and copy constructor.
const BASE_CTOR: u32 = 0x006e_3f30;
const BASE_COPY_CTOR: u32 = 0x006e_3fb0;
/// `006e2750`: `PathingRequest::CopyTo` (Xbox PDB).
const REQUEST_COPY_TO: u32 = 0x006e_2750;
/// `004036b0`: the other request's word at +0xB8.
const GET_TARGET: u32 = 0x0040_36b0;
/// `006e60c0`: the target setter (registered in this family).
const SET_TARGET: u32 = 0x006e_60c0;
/// `006e5f00` / `006e5f50`: the distances' SaveGame / LoadGame (engine map).
const DISTANCES_SAVE: u32 = 0x006e_5f00;
const DISTANCES_LOAD: u32 = 0x006e_5f50;
/// `00865df0` / `00864e0`: `SaveFormID_ov2` / `LoadFormID_ov2` (engine map).
const SAVE_FORM: u32 = 0x0086_5df0;
const LOAD_FORM_ID: u32 = 0x0086_48e0;
/// `004839c0`: the form of a form id; `00ec43fb`: `__RTDynamicCast`.
const LOOKUP_FORM: u32 = 0x0048_39c0;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// The type descriptors of `TESForm` (`01183028`) and `TESObjectREFR` (`011841cc`).
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_OBJECT_REFR: u32 = 0x0118_41cc;

// Typed from 006e5fd0
/// `PathingRequestSafeStraightLine` scalar deleting destructor (Xbox PDB): runs the destructor
/// body (`006d3b20`) and frees the 0xbc bytes only when `flags & 1` is set. Returns `this`.
pub fn scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(DTOR_BODY, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE, &args![this, 0xbcu32]);
    }
    this
}

// Typed from 006e5fa0
/// `PathingRequestSafeStraightLine::PathingRequestSafeStraightLine` (Xbox PDB): the
/// `PathingRequestClosePoint` base, the vtable, no target. Returns `this`.
pub fn new(
    e: &mut Engine,
    this: Ptr<PathingRequestSafeStraightLine>,
) -> Ptr<PathingRequestSafeStraightLine> {
    e.call(BASE_CTOR, &args![this]);
    e.mem.set_u32(this.addr(), VTABLE);
    e.set(this, PathingRequestSafeStraightLine::pTarget, 0);
    this
}

// Typed from 006e6010
/// The `PathingRequestSafeStraightLine` copy constructor: the `PathingRequestClosePoint` copy
/// constructor, the vtable, then the other's target (`004036b0`). Returns `this`.
pub fn copy_new(
    e: &mut Engine,
    this: Ptr<PathingRequestSafeStraightLine>,
    other: Ptr,
) -> Ptr<PathingRequestSafeStraightLine> {
    e.call(BASE_COPY_CTOR, &args![this, other]);
    e.mem.set_u32(this.addr(), VTABLE);
    let target = e.call(GET_TARGET, &args![other]).u32();
    e.set(this, PathingRequestSafeStraightLine::pTarget, target);
    this
}

// Typed from 006e6080
/// `PathingRequestSafeStraightLine::CopyTo` (Xbox PDB): the `PathingRequest` part
/// (`PathingRequest::CopyTo`, not the close point's), then the target setter on the other with
/// this target (which does not store it).
pub fn copy_to(e: &mut Engine, this: Ptr<PathingRequestSafeStraightLine>, other: Ptr) {
    e.call(REQUEST_COPY_TO, &args![this, other]);
    let target = e.get(this, PathingRequestSafeStraightLine::pTarget);
    e.call(SET_TARGET, &args![other, target]);
}

// Typed from 006e60c0
/// Unnamed (`RET 4`): the target setter as compiled: it copies `pTarget` (+0xB8) onto itself and
/// never reads its argument.
pub fn set_target(e: &mut Engine, this: Ptr<PathingRequestSafeStraightLine>, _unused: u32) {
    let target = e.get(this, PathingRequestSafeStraightLine::pTarget);
    e.set(this, PathingRequestSafeStraightLine::pTarget, target);
}

// Typed from 006e60e0
/// `PathingRequestSafeStraightLine::SaveGame` (Xbox PDB): the distances' save (`006e5f00`), then
/// the target as a form (`SaveFormID_ov2` with 0).
pub fn save_game(e: &mut Engine, this: Ptr<PathingRequestSafeStraightLine>, buffer: u32) {
    e.call(DISTANCES_SAVE, &args![this, buffer]);
    let target = e.get(this, PathingRequestSafeStraightLine::pTarget);
    e.call(SAVE_FORM, &args![buffer, target, 0u32]);
}

// Typed from 006e6110
/// `PathingRequestSafeStraightLine::LoadGame` (Xbox PDB): the distances' load (`006e5f50`), then
/// the target's form id into `pTarget` (`LoadFormID_ov2`; [`init_load_game`] turns it into the
/// reference).
pub fn load_game(e: &mut Engine, this: Ptr<PathingRequestSafeStraightLine>, buffer: u32) {
    e.call(DISTANCES_LOAD, &args![this, buffer]);
    e.call(LOAD_FORM_ID, &args![buffer, this.addr() + 0xb8]);
}

// Typed from 006e6140
/// `PathingRequestSafeStraightLine::InitLoadGame` (Xbox PDB): a loaded target form id becomes
/// the form (`004839c0`) cast to `TESObjectREFR` (`__RTDynamicCast`); 0 stays 0. The buffer
/// argument is not read.
pub fn init_load_game(e: &mut Engine, this: Ptr<PathingRequestSafeStraightLine>, _unused: u32) {
    let form_id = e.get(this, PathingRequestSafeStraightLine::pTarget);
    let target = if form_id != 0 {
        let form = e.call(LOOKUP_FORM, &args![form_id]).u32();
        e.call(
            DYNAMIC_CAST,
            &args![form, 0u32, TYPE_TES_FORM, TYPE_TES_OBJECT_REFR, 0u32],
        )
        .u32()
    } else {
        0
    };
    e.set(this, PathingRequestSafeStraightLine::pTarget, target);
}
