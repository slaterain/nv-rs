//! The per-actor loops of the after-draw stage (`Main::UpdateNonRenderSafeAITasks`, `0086f6a0`) that
//! walk the high-process actors after the frame is shown: the projectiles' and explosions' step
//! (`0096c240`), the detection lists merged into each character's process (`0096c710`, with the
//! merge `008f63e0`, which reaches `HighProcess::GetDetectionState`, `008f6650`, the dispatch family's, directly) and the per-actor
//! ragdoll and process step (`009781d0`). Each is the translation's loop with the process array's
//! accessors, the leaf getters and the detection lookup called directly; the actors' own slots and
//! the heavy callees (the ragdoll's update, the process slots) stay by address.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address.
//! No float maths but moves (`009781d0`'s frame time is passed on as the caller gave it).

use super::{array, FAMILY};
use crate::dispatch;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The player (`011dea3c`).
const PLAYER: u32 = 0x011d_ea3c;
/// Actor slots: `IsActor` (+0x100), the 3D node (+0x1d0), the package test (+0x22c with 0), the
/// object kind tests (+0x220, +0x224), the projectile step (+0x314), the frame step (+0x34c).
const SLOT_IS_ACTOR: u32 = 0x100;
const SLOT_GET_NODE: u32 = 0x1d0;
const SLOT_0X220: u32 = 0x220;
const SLOT_0X224: u32 = 0x224;
const SLOT_0X22C: u32 = 0x22c;
const SLOT_0X314: u32 = 0x314;
const SLOT_0X34C: u32 = 0x34c;
/// The process's slot of `GetDetectionState` (+0x504; `HighProcess`'s `008f6650`).
const SLOT_DETECTION_STATE: u32 = 0x504;
const GET_DETECTION_STATE: u32 = 0x008f_6650;
/// The process's lists (`HighProcess`, PC offsets): detected actors (+0x25c), actors who detect me
/// (+0x260), the detection pass's thread list (+0x268) and its temporary list (+0x26c).
const DETECTED_ACTOR_LIST: u32 = 0x25c;
const WHO_DETECTS_ME_LIST: u32 = 0x260;
const THREAD_DETECT_LIST: u32 = 0x268;
const TEMP_WHO_DETECTS_ME_LIST: u32 = 0x26c;
/// `operator new` / `delete`, `DetectionState`'s constructor (0x24 bytes), its copy (`008d6fc0`),
/// the list's append (`005ae3d0`, `(list, &item)`) and clear (`00470470`).
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const DETECTION_STATE_CONSTRUCTOR: u32 = 0x0066_e590;
const DETECTION_STATE_COPY: u32 = 0x008d_6fc0;
const LIST_APPEND: u32 = 0x005a_e3d0;
const LIST_CLEAR: u32 = 0x0047_0470;
/// The actor's slot of its package of type 0x12 (+0x428) and the package's flag store (`008f6630`).
const SLOT_PACKAGE_0X12: u32 = 0x428;
const PACKAGE_FLAG: u32 = 0x008f_6630;

/// The process array (the lists' +4), a level's head and tail and the object at an index.
fn array_at(this: Ptr) -> Ptr {
    Ptr::new(this.addr().wrapping_add(4))
}
fn head(e: &mut Engine, this: Ptr, level: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x005d_e0f0,
        array::level_head(e, array_at(this), level)
    )
}
fn tail(e: &mut Engine, this: Ptr, level: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x005b_e5c0,
        array::level_tail(e, array_at(this), level)
    )
}
fn object_at(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0096_8670,
        dispatch::refs::array_item(e, array_at(this), index)
    )
}

/// The process object of an actor (`008d8520`, the word at +0x68).
fn process_of(e: &mut Engine, actor: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, actor))
}

// Typed from 0096c240
/// The projectiles' and explosions' step (`thiscall(lists, unused)`, `RET 4`): for each object of
/// level 0 (the end asked again each step) that is not null and for which `00576d30`, `00440d80` and
/// `00440da0` are all false: slot +0x224 true gets slot +0x314; otherwise slot +0x220 true runs
/// `009ae6e0` on it.
pub fn projectiles_step(e: &mut Engine, this: Ptr, _unused: u32) {
    let mut index = head(e, this, 0);
    while index < tail(e, this, 0) {
        let object = object_at(e, this, index);
        if object != 0
            && !typed_call!(leaf::FAMILY, 0x0057_6d30, leaf::form_flag_200000(e, object))
            && !typed_call!(leaf::FAMILY, 0x0044_0d80, leaf::form_flag_20(e, object))
            && !typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, object))
        {
            if e.vcall(object, SLOT_0X224, &args![]).bool() {
                e.vcall(object, SLOT_0X314, &args![]);
            } else if e.vcall(object, SLOT_0X220, &args![]).bool() {
                e.call(0x009a_e6e0, &args![object]);
            }
        }
        index = index.wrapping_add(1);
    }
}

// Typed from 0096c710
/// The detection lists merged (`thiscall(lists)`): `008f63e0(process, actor)` for the player when it
/// has a process object, then for every character (slot +0x100) of level 0.
pub fn merge_detection(e: &mut Engine, this: Ptr) {
    let player = e.mem.u32(PLAYER);
    if process_of(e, player) != 0 {
        let player = e.mem.u32(PLAYER);
        let process = process_of(e, player);
        typed_call!(
            FAMILY,
            0x008f_63e0,
            merge_detection_lists(e, Ptr::new(process), Ptr::new(player))
        );
    }
    let mut index = head(e, this, 0);
    while index < tail(e, this, 0) {
        let actor = object_at(e, this, index);
        if actor != 0 && e.vcall(actor, SLOT_IS_ACTOR, &args![]).bool() {
            let process = process_of(e, actor);
            typed_call!(
                FAMILY,
                0x008f_63e0,
                merge_detection_lists(e, Ptr::new(process), Ptr::new(actor))
            );
        }
        index = index.wrapping_add(1);
    }
}

/// The item of a list node and the next node.
fn list_item(e: &mut Engine, node: u32) -> u32 {
    let slot = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(slot)
}
fn list_next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

/// The process's `DetectionState` for `(actor, which)` through its slot +0x504: the typed lookup
/// directly when the slot holds `HighProcess`'s.
fn detection_state(e: &mut Engine, process: u32, actor: u32, which: u32) -> u32 {
    if crate::slot_target(e, process, SLOT_DETECTION_STATE) == GET_DETECTION_STATE {
        typed_call!(
            dispatch::FAMILY,
            0x008f_6650,
            dispatch::refs::detection_state(e, Ptr::new(process), actor, which as i32)
        )
        .addr()
    } else {
        e.vcall(process, SLOT_DETECTION_STATE, &args![actor, which])
            .u32()
    }
}

/// A new `DetectionState` (null when the allocation fails).
fn new_detection_state(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![0x24u32]).u32();
    if block == 0 {
        0
    } else {
        e.call(DETECTION_STATE_CONSTRUCTOR, &args![block]).u32()
    }
}

/// Appends `item` to `list` (the list function takes the item's address).
fn list_append(e: &mut Engine, list: u32, item: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        e.call(LIST_APPEND, &args![list, slot]);
    });
}

/// One of the two merges: every state of the list at `from` copied into the process's state for
/// its actor (`which`), or a new one appended to the list at `into`; each source freed, the list
/// emptied. True when a state was merged.
fn merge_list(e: &mut Engine, this: Ptr, from: u32, into: u32, which: u32) -> bool {
    let mut merged = false;
    let mut node = e.mem.u32(this.addr() + from);
    while node != 0 {
        let source = list_item(e, node);
        if source == 0 {
            break;
        }
        let actor = e.mem.u32(source);
        let mut target = detection_state(e, this.addr(), actor, which);
        if target == 0 {
            target = new_detection_state(e);
            let list = e.mem.u32(this.addr() + into);
            list_append(e, list, target);
        }
        e.call(DETECTION_STATE_COPY, &args![target, source]);
        merged = true;
        e.call(OPERATOR_DELETE, &args![source]);
        node = list_next(e, node);
    }
    let list = e.mem.u32(this.addr() + from);
    e.call(LIST_CLEAR, &args![list]);
    merged
}

// Typed from 008f63e0
/// The detection pass's lists merged into the process (`thiscall(process, owner)`, `RET 4`): the
/// thread list (+0x268) into the detected actors (+0x25c, `GetDetectionState(actor, 0)`); when a
/// state was merged, `owner`'s package of type 0x12 (slot +0x428), when there is one, gets its flag
/// set (`008f6630` with 1); then the temporary list (+0x26c) into the actors who detect me (+0x260,
/// `GetDetectionState(actor, 1)`).
pub fn merge_detection_lists(e: &mut Engine, this: Ptr, owner: Ptr) {
    let merged = merge_list(e, this, THREAD_DETECT_LIST, DETECTED_ACTOR_LIST, 0);
    if merged {
        let package = e.vcall(owner.addr(), SLOT_PACKAGE_0X12, &args![]).u32();
        if package != 0 {
            e.call(PACKAGE_FLAG, &args![package, 1u32]);
        }
    }
    merge_list(e, this, TEMP_WHO_DETECTS_ME_LIST, WHO_DETECTS_ME_LIST, 1);
}

// Typed from 009781d0
/// The per-actor ragdoll and process step (`thiscall(lists, delta)`): for each object of level 0 (the
/// end asked again each step): an actor that `00576d30` does not skip gets `008ba600`, its process's
/// slot +0x104, the ragdoll update of its object at +0xb0 (`00978400`, then `00ca2ad0` with the
/// actor's 3D node and a value from `00931ed0` and `004a3a20`; `00ca1410`), `ConsolidateSimIslands`
/// (`0062c430`) unless its slot +0x22c(0) is true, its slot +0x34c(delta, the player resting) and its
/// process's slots +0x500(actor, 0), +0x170(actor) and +0x1d8(actor); another object with slot
/// +0x220 gets `009b17a0(delta)`. Then `00978890(delta)`, and the player's process slot +0x170.
pub fn ragdoll_step(e: &mut Engine, this: Ptr, delta: f32) {
    let mut index = 0u32;
    while index < tail(e, this, 0) {
        let object = object_at(e, this, index);
        if object != 0
            && e.vcall(object, SLOT_IS_ACTOR, &args![]).bool()
            && !typed_call!(leaf::FAMILY, 0x0057_6d30, leaf::form_flag_200000(e, object))
        {
            actor_step(e, object, delta);
        } else if object != 0 && e.vcall(object, SLOT_0X220, &args![]).bool() {
            e.call(0x009b_17a0, &args![object, delta]);
        }
        index = index.wrapping_add(1);
    }
    e.call(0x0097_8890, &args![this, delta]);
    let player = e.mem.u32(PLAYER);
    if process_of(e, player) != 0 {
        let process = process_of(e, player);
        e.vcall(process, 0x170, &args![player]);
    }
}

/// One actor of [`ragdoll_step`].
fn actor_step(e: &mut Engine, actor: u32, delta: f32) {
    e.call(0x008b_a600, &args![actor]);
    let process = process_of(e, actor);
    e.vcall(process, 0x104, &args![]);
    let ragdoll = e.mem.u32(actor + 0xb0);
    // `00978400`: the ragdoll controller's flag byte at +0x18.
    if e.call(0x0097_8400, &args![ragdoll]).bool() {
        e.with_stack(0x10, |e, buffer| {
            let source = e.call(0x0093_1ed0, &args![actor, buffer.addr()]).u32();
            let value = e.call(0x004a_3a20, &args![source]).u32();
            let node = e.vcall(actor, SLOT_GET_NODE, &args![]).u32();
            let ragdoll = e.mem.u32(actor + 0xb0);
            e.call(0x00ca_2ad0, &args![ragdoll, node, value]);
        });
    }
    let ragdoll = e.mem.u32(actor + 0xb0);
    e.call(0x00ca_1410, &args![ragdoll]);
    if !e.vcall(actor, SLOT_0X22C, &args![0u32]).bool() {
        e.call(0x0062_c430, &args![actor]);
    }
    let player = e.mem.u32(PLAYER);
    let resting = e.call(0x0094_df60, &args![player]).u8();
    e.vcall(actor, SLOT_0X34C, &args![delta, u32::from(resting)]);
    let process = process_of(e, actor);
    if process != 0 {
        e.vcall(process, 0x500, &args![actor, 0u32]);
        e.vcall(process, 0x170, &args![actor]);
        e.vcall(process, 0x1d8, &args![actor]);
    }
}
