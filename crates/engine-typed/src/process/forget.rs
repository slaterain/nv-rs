//! The process lists' clean-up when a reference goes away: `ProcessLists::NullOutTargetRef`
//! (`0096f600`) and the shorter clean-up a reference's destructor runs (`00976680`). Each walks every
//! object of the process array up to the tail of level 3 and every object of the temp list, and for
//! each actor (and its followers) clears whatever mentions the reference: its followers, head-track
//! target, package targets and locations, combat, dialogue and furniture targets, flee target. So a
//! cell unload or a fast travel, which frees hundreds of references, makes these walks hundreds of
//! times over every actor: in the load window of 2026-11-10 they were 151 and 82 ms of one cell load
//! (a profiled run; measure/stewie-eo/TARGETS.md, row 2).
//!
//! lStewieAl's Engine Optimizations (its Tweaks' `Inlines`, the process part) makes the same walks
//! with the getters read in place. Here likewise: an object's extra data list (`005d43c0`: the object
//! plus 0x44) and the word at +0x70 (`004fd380`) are read in place, the process array's tail and
//! elements and the extra data getters (followers, head-track target, package data, package target,
//! the lookup itself) are the process and dispatch families' Rust functions. Every other call (the
//! virtual slots of the actors and their processes, the package and combat functions) is made in the
//! exe's order with the exe's words. The plugin also skips `Actor::IsSpectator` before the created
//! package's clean-up in `0096f600` (it jumps into the clean-up whatever the answer); that changes
//! what runs, so it is not done here.
//!
//! Main thread only in the game (the reference's destructor and the cell unload run there or under the
//! process lock); no float maths.

use super::{array, FAMILY};
use crate::dispatch;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The player, a global.
const PLAYER: u32 = 0x011d_ea3c;
/// The object `00976680` first tells about the reference (`00992920`).
const OBSERVER: u32 = 0x011f_1958;
/// The process lists' process array, their alive actor list and their temp list.
const MOB_PROCESS_ARRAY: u32 = 0x04;
const ALIVE_ACTOR_LIST: u32 = 0x80;
const TEMP_SHOULD_MOVE_LIST: u32 = 0x78;
/// The process type of a package (`0041ca90`).
const PACKAGE_TYPE: u32 = 0x0041_ca90;
/// A list's removal of the value a word points at (`00905330`).
const LIST_REMOVE: u32 = 0x0090_5330;
/// The extra data list's removal of a follower (`00422690`).
const EXTRA_REMOVE_FOLLOWER: u32 = 0x0042_2690;
/// The word the furniture marker's reset passes (`0098ddd0`'s argument).
const FURNITURE_ARGUMENT: u32 = 0x011f_426c;
/// The actors close to the player: their removal (`0096e6f0`).
const REMOVE_CLOSE_TO_PLAYER: u32 = 0x0096_e6f0;

/// An object's extra data list: `005d43c0`, the object plus 0x44.
fn extra_list(object: u32) -> u32 {
    object.wrapping_add(0x44)
}

/// The word at +0x70 (`004fd380`): an actor's head-track target here.
fn word_70(e: &mut Engine, object: u32) -> u32 {
    e.mem.u32(object.wrapping_add(0x70))
}

fn is_actor(e: &mut Engine, object: u32) -> bool {
    e.vcall(object, 0x100, &[]).bool()
}

fn actor_process(e: &mut Engine, actor: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, actor))
}

fn followers(e: &mut Engine, object: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0042_2700,
        dispatch::extra::followers(e, Ptr::new(extra_list(object)))
    )
    .addr()
}

fn array_tail(e: &mut Engine, array: u32, level: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x005b_e5c0,
        array::level_tail(e, Ptr::new(array), level)
    )
}

fn array_object(e: &mut Engine, array: u32, index: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0096_8670,
        dispatch::refs::array_item(e, Ptr::new(array), index)
    )
}

fn node_value(e: &mut Engine, node: u32) -> u32 {
    let item = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(item)
}

fn node_next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

fn skipped(e: &mut Engine, object: u32) -> bool {
    typed_call!(leaf::FAMILY, 0x0057_6d30, leaf::form_flag_200000(e, object))
}

fn process_level(e: &mut Engine, process: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0045_cd60, leaf::word_at_28(e, process))
}

/// Removes `value` from a list (`00905330` takes the address of a local holding it).
fn list_remove_value(e: &mut Engine, list: u32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(LIST_REMOVE, &args![list, slot.addr()]);
    });
}

/// The player's followers lose an actor `target` (both functions start so).
fn leave_player_followers(e: &mut Engine, player: u32, target: u32) {
    if followers(e, player) != 0 {
        let record = followers(e, player);
        if e.mem.u32(record + 0xc) != 0 {
            let record = followers(e, player);
            let list = e.mem.u32(record + 0xc);
            list_remove_value(e, list, target);
        }
    }
    if actor_process(e, player) != 0 {
        let process = actor_process(e, player);
        e.vcall(process, 0x2c4, &args![target, 3u32]);
    }
}

/// Clears a package-like object's location reference when it is `target`.
fn clear_location_reference(e: &mut Engine, package: u32, target: u32) {
    if package == 0 {
        return;
    }
    if e.call(0x0055_b980, &args![package]).u32() != 0 {
        let location = e.call(0x0055_b980, &args![package]).u32();
        if e.call(0x0067_f390, &args![location]).u32() == target {
            let location = e.call(0x0055_b980, &args![package]).u32();
            e.call(0x0067_f3c0, &args![location, 0u32]);
        }
    }
}

/// `0096f600`'s package end for `high` when its package target is `target`.
fn end_package_if_target(e: &mut Engine, high: u32, process: u32, target: u32) {
    let holder = e.vcall(process, 0x20c, &[]).u32();
    let mut hit = e.call(0x0088_1650, &args![high]).u32() == target;
    if !hit
        && holder != 0
        && typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, holder)) != 0
    {
        let inner = typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, holder));
        hit = e.call(0x0068_0020, &args![inner]).u32() == target;
    }
    if !hit {
        return;
    }
    if e.call(0x0093_36c0, &args![high]).bool() {
        e.vcall(high, 0x288, &[]);
    } else {
        let package = e.call(0x0093_44a0, &args![high]).u32();
        if e.call(0x0067_4d40, &args![package]).bool() {
            e.call(0x0088_1680, &args![high, 0u32]);
        } else {
            e.vcall(process, 0x234, &[]);
            e.vcall(process, 0x214, &[]);
        }
    }
}

/// The furniture marker's reset when the process's furniture is `target`.
fn release_furniture(e: &mut Engine, high: u32, process: u32, target: u32) {
    if e.vcall(process, 0x4c8, &[]).u32() != target {
        return;
    }
    let marker = e.vcall(process, 0x4d4, &[]).u32();
    e.call(0x00c5_4550, &args![marker, 0.0f32]);
    let marker = e.vcall(process, 0x4d4, &[]).u32();
    e.call(0x0056_8ab0, &args![marker, 0u32]);
    let marker = e.vcall(process, 0x4d4, &[]).u32();
    e.call(0x0098_ddd0, &args![marker, FURNITURE_ARGUMENT]);
    let other = actor_process(e, high);
    let marker = e.vcall(other, 0x4d4, &[]).u32();
    e.vcall(process, 0x540, &args![0u32, 0x7fu32, marker]);
}

/// The flee target: a package of type 0x16 whose target is `target` is told (`00994ef0`), then
/// `Actor::RemoveFleeTarget`.
fn forget_flee_target(e: &mut Engine, high: u32, target: u32) {
    let package = e.call(0x0093_44a0, &args![high]).u32();
    let mut flee = 0u32;
    if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x16 {
        flee = package;
    }
    if flee != 0 && e.call(0x005e_3fc0, &args![flee]).u32() == target {
        e.call(0x0099_4ef0, &args![flee, 0u32]);
    }
    e.call(0x008a_6730, &args![high, target]);
}

/// The package extra data's target (type 0x19, +0x14) cleared when it is `target`.
fn clear_extra_record_reference(e: &mut Engine, high: u32, target: u32) {
    let record = typed_call!(
        dispatch::FAMILY,
        0x0041_0220,
        dispatch::extra::get_extra_data(e, Ptr::new(extra_list(high)), 0x19)
    )
    .addr();
    if record != 0 && e.mem.u32(record + 0x14) == target {
        e.mem.set_u32(record + 0x14, 0);
    }
}

/// The address the process's slot 0x3d0 is compared with: the reference's second base, or 0.
fn second_base_of(target: u32) -> u32 {
    if target == 0 {
        0
    } else {
        target + 0x94
    }
}

/// The next follower of the extra data's follower list (0 at the end or for one that is not an
/// actor), the list moved on.
fn next_follower(e: &mut Engine, list: &mut u32) -> u32 {
    if *list != 0 && node_value(e, *list) != 0 {
        let mut high = 0u32;
        let follower = node_value(e, *list);
        if is_actor(e, follower) {
            high = node_value(e, *list);
        }
        *list = node_next(e, *list);
        high
    } else {
        0
    }
}

/// The head-track target cleared when it is `target`, and the process told.
fn clear_head_track(e: &mut Engine, high: u32, process: u32, target: u32) {
    if word_70(e, high) == target {
        e.call(0x0057_bd60, &args![high, 0u32]);
        if process != 0 {
            e.vcall(process, 0x310, &args![0u32]);
            e.vcall(process, 0x4a0, &[]);
        }
    }
}

/// `0096f600`'s first loop, for an actor of the array and then its followers.
fn forget_in_array_actor(e: &mut Engine, high: u32, list: u32, target: u32, flag: u8) {
    let player = e.mem.u32(PLAYER);
    let mut high = high;
    let mut list = list;
    while high != 0 {
        e.call(0x008b_3200, &args![high, target]);
        let extra = extra_list(high);
        if extra != 0
            && typed_call!(
                dispatch::FAMILY,
                0x0041_9ea0,
                dispatch::extra::head_track_target(e, Ptr::new(extra))
            ) == target
        {
            e.call(0x0041_9dc0, &args![extra, 0u32]);
        }
        let process = actor_process(e, high);
        clear_head_track(e, high, process, target);
        if process == 0 {
            return;
        }
        if (process_level(e, process) as i32) < 2 {
            e.vcall(process, 0x2c4, &args![target, 3u32]);
            e.vcall(process, 0x260, &args![target]);
            e.vcall(process, 0x7b0, &args![target]);
            let record = e.vcall(process, 0xf8, &args![player]).u32();
            if record != 0 && e.mem.u32(record + 0x18) == target {
                e.vcall(process, 0x100, &[]);
            }
            if e.vcall(process, 0x68c, &[]).u32() == target {
                e.vcall(process, 0x26c, &[]);
            }
            if e.vcall(process, 0x484, &[]).u32() == target {
                e.vcall(process, 0x488, &args![0u32]);
            }
            if is_actor(e, target) {
                e.vcall(process, 0xc0, &args![target]);
            }
            let package = e.vcall(process, 0x20c, &[]).u32();
            if package != 0 {
                let holder = e.vcall(process, 0x278, &[]).u32();
                if holder != 0
                    && typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, holder)) == target
                {
                    e.call(0x0088_1680, &args![high, 0u32]);
                } else if e.call(PACKAGE_TYPE, &args![package]).u32() == 0x1c
                    && e.call(0x008d_80e0, &args![package]).u32() == target
                {
                    e.vcall(high, 0x288, &[]);
                }
            }
        }
        e.vcall(process, 0x47c, &args![target]);
        clear_extra_record_reference(e, high, target);
        if e.call(0x008a_6650, &args![high, 0u32]).bool() {
            forget_flee_target(e, high, target);
        } else if e.call(0x008a_6170, &args![high]).bool() && is_actor(e, target) {
            let package = e.call(0x0093_44a0, &args![high]).u32();
            let mut created = 0u32;
            if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x18 {
                created = package;
            }
            if created != 0 {
                e.call(0x009f_8420, &args![created, target]);
            }
        }
        e.vcall(process, 0x8, &args![target]);
        let extra = extra_list(high);
        if extra != 0 {
            let package_data = typed_call!(
                dispatch::FAMILY,
                0x0042_12b0,
                dispatch::extra::actor_package_data(e, Ptr::new(extra))
            );
            if package_data != 0 {
                e.vcall(package_data, 0x4, &args![target]);
            }
            if typed_call!(
                dispatch::FAMILY,
                0x0041_cb70,
                dispatch::extra::package_target(e, Ptr::new(extra))
            ) == target
            {
                e.call(0x0041_cab0, &args![extra, 0u32]);
            }
        }
        if is_actor(e, target) && e.vcall(process, 0x3d0, &[]).u32() == second_base_of(target) {
            e.vcall(process, 0x3d4, &args![0u32]);
        }
        if process_level(e, process) == 0 {
            e.vcall(process, 0xac, &[]);
        }
        if word_70(e, high) == target {
            e.call(0x0057_bd60, &args![high, 0u32]);
        }
        if flag != 0 {
            let package = e.call(0x0093_44a0, &args![high]).u32();
            clear_location_reference(e, package, target);
            let current = e.vcall(process, 0x22c, &[]).u32();
            clear_location_reference(e, current, target);
        }
        let controller = e.vcall(high, 0x428, &[]).u32();
        if controller != 0 {
            e.call(0x0097_f9c0, &args![controller, target]);
        }
        if e.vcall(process, 0x128, &[]).u32() == target {
            e.vcall(process, 0x12c, &args![0u32]);
        }
        if e.vcall(high, 0x2c8, &[]).u32() == target {
            e.call(0x0088_1620, &args![high, 0u32]);
        }
        end_package_if_target(e, high, process, target);
        e.vcall(process, 0x664, &args![target]);
        release_furniture(e, high, process, target);
        high = next_follower(e, &mut list);
    }
}

/// `0096f600`'s second loop (the temp list), for an actor and then its followers; `array_index` is
/// the index the first loop ended with (the detection removal only below 2).
fn forget_in_temp_actor(e: &mut Engine, high: u32, list: u32, target: u32, array_index: u32) {
    let mut high = high;
    let mut list = list;
    while high != 0 {
        let process = actor_process(e, high);
        clear_head_track(e, high, process, target);
        if process == 0 {
            return;
        }
        if array_index < 2 {
            e.vcall(process, 0x2c4, &args![target, 3u32]);
        }
        e.vcall(process, 0x47c, &args![target]);
        clear_extra_record_reference(e, high, target);
        if e.call(0x008a_6650, &args![high, 0u32]).bool() {
            forget_flee_target(e, high, target);
        }
        if is_actor(e, target) && actor_process(e, high) != 0 {
            let marker = second_base_of(target);
            let current = actor_process(e, high);
            if e.vcall(current, 0x3d0, &[]).u32() == marker {
                let current = actor_process(e, high);
                e.vcall(current, 0x3d4, &args![0u32]);
            }
        }
        if process_level(e, process) == 0 {
            e.vcall(process, 0xac, &[]);
        }
        if word_70(e, high) == target {
            e.call(0x0057_bd60, &args![high, 0u32]);
        }
        let package = e.call(0x0093_44a0, &args![high]).u32();
        clear_location_reference(e, package, target);
        let current = e.vcall(process, 0x22c, &[]).u32();
        clear_location_reference(e, current, target);
        let controller = e.vcall(high, 0x428, &[]).u32();
        if controller != 0 {
            e.call(0x0097_f9c0, &args![controller, target]);
        }
        if e.vcall(process, 0x128, &[]).u32() == target {
            e.vcall(process, 0x12c, &args![0u32]);
        }
        if e.vcall(high, 0x2c8, &[]).u32() == target {
            e.call(0x0088_1620, &args![high, 0u32]);
        }
        end_package_if_target(e, high, process, target);
        e.vcall(process, 0x664, &args![target]);
        release_furniture(e, high, process, target);
        high = next_follower(e, &mut list);
    }
}

/// An actor's follower list (the followers extra data's list at +0xc), or 0.
fn follower_list(e: &mut Engine, high: u32) -> u32 {
    let record = if high != 0 { followers(e, high) } else { 0 };
    if record != 0 {
        e.mem.u32(record + 0xc)
    } else {
        0
    }
}

// Typed from 0096f600
/// `ProcessLists::NullOutTargetRef` (`thiscall(lists, target, flag)`): forgets `target` in the
/// player, in every object of the process array up to the tail of level 3 and in every object of the
/// temp list; with `flag` also the package locations that point at it.
pub fn null_out_target_ref(e: &mut Engine, this: Ptr, target: u32, flag: u8) {
    let array = this.addr().wrapping_add(MOB_PROCESS_ARRAY);
    let player = e.mem.u32(PLAYER);

    if is_actor(e, target) {
        leave_player_followers(e, player, target);
        // `0096e2f0`: the alive actor list loses it.
        list_remove_value(e, this.addr().wrapping_add(ALIVE_ACTOR_LIST), target);
    }
    if word_70(e, player) == target {
        e.call(0x0057_bd60, &args![player, 0u32]);
    }
    if actor_process(e, player) != 0 {
        let process = actor_process(e, player);
        if e.vcall(process, 0x68c, &[]).u32() == target {
            let process = actor_process(e, player);
            e.vcall(process, 0x26c, &[]);
        }
        let process = actor_process(e, player);
        if e.vcall(process, 0x484, &[]).u32() == target {
            let process = actor_process(e, player);
            e.vcall(process, 0x488, &args![0u32]);
        }
        let process = actor_process(e, player);
        let record = e.vcall(process, 0xf8, &args![player]).u32();
        if record != 0 && e.mem.u32(record + 0x18) == target && actor_process(e, player) != 0 {
            let process = actor_process(e, player);
            e.vcall(process, 0x100, &[]);
        }
    }
    if is_actor(e, target) {
        e.vcall(player, 0x468, &args![target]);
    }

    // The objects of the process array.
    let mut index = 0u32;
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        if object != 0 && !skipped(e, object) {
            let mut high = 0u32;
            let mut other = 0u32;
            if is_actor(e, object) {
                high = object;
            } else if e.vcall(object, 0x224, &[]).bool() {
                other = object;
            }
            let list = follower_list(e, high);
            if high != 0 && followers(e, high) != 0 && is_actor(e, target) {
                e.call(EXTRA_REMOVE_FOLLOWER, &args![extra_list(high), target]);
            }
            if other != 0 && e.call(0x0087_4480, &args![other]).u32() == target {
                // `009707f0`: the word at +0xfc.
                e.mem.set_u32(other.wrapping_add(0xfc), 0);
            }
            if high == 0 {
                if e.call(0x005e_3fa0, &args![object]).u32() != 0
                    && target == e.call(0x005e_3fa0, &args![object]).u32()
                {
                    e.vcall(object, 0x288, &[]);
                }
            } else {
                forget_in_array_actor(e, high, list, target, flag);
            }
        }
        index = index.wrapping_add(1);
    }

    // The objects of the temp list.
    let mut node = this.addr().wrapping_add(TEMP_SHOULD_MOVE_LIST);
    while node != 0 && !typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, node)) {
        let item = node_value(e, node);
        if item != 0 && !skipped(e, item) {
            let mut high = 0u32;
            let mut other = 0u32;
            let object = node_value(e, node);
            if is_actor(e, object) {
                high = node_value(e, node);
            } else {
                let object = node_value(e, node);
                if e.vcall(object, 0x224, &[]).bool() {
                    other = node_value(e, node);
                }
            }
            let list = follower_list(e, high);
            if other != 0 && e.call(0x0087_4480, &args![other]).u32() == target {
                e.mem.set_u32(other.wrapping_add(0xfc), 0);
            }
            forget_in_temp_actor(e, high, list, target, index);
        }
        node = node_next(e, node);
    }
}

/// `00976680`'s clean-up of a created package (not of type 0x12) whose target or location is
/// `target`.
fn clear_created_package_references(e: &mut Engine, package: u32, high: u32, target: u32) {
    if package == 0
        || !e.call(0x0067_4d40, &args![package]).bool()
        || e.call(PACKAGE_TYPE, &args![package]).u32() == 0x12
    {
        return;
    }
    if typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, package)) != 0 {
        let inner = typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, package));
        if e.call(0x0068_0020, &args![inner]).u32() == target {
            e.call(0x0067_2fc0, &args![package, 0u32]);
        }
    }
    if e.call(0x0067_6140, &args![package, high]).u32() == target {
        e.call(0x0067_1d30, &args![package, 0u32]);
    }
}

/// `00976680`'s package end when the package target is `target` (the slot asked again each step).
fn end_package_when_targeted(e: &mut Engine, high: u32, process: u32, target: u32) {
    let mut hit = e.call(0x0088_1650, &args![high]).u32() == target;
    if !hit && e.vcall(process, 0x20c, &[]).u32() != 0 {
        let holder = e.vcall(process, 0x20c, &[]).u32();
        if typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, holder)) != 0 {
            let holder = e.vcall(process, 0x20c, &[]).u32();
            let inner = typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, holder));
            hit = e.call(0x0068_0020, &args![inner]).u32() == target;
        }
    }
    if !hit {
        return;
    }
    if e.call(0x0093_36c0, &args![high]).bool() {
        e.vcall(high, 0x288, &[]);
        return;
    }
    if e.call(0x0093_44a0, &args![high]).u32() != 0 {
        let package = e.call(0x0093_44a0, &args![high]).u32();
        if e.call(0x0067_4d40, &args![package]).bool() {
            e.call(0x0088_1680, &args![high, 0u32]);
            return;
        }
    }
    e.vcall(process, 0x234, &[]);
    e.vcall(process, 0x214, &[]);
}

/// The clean-up both loops of `00976680` end with.
fn forget_actor_tail(e: &mut Engine, high: u32, process: u32, target: u32) {
    clear_extra_record_reference(e, high, target);
    if e.mem.u32(high + 0xc0) == target {
        e.mem.set_u32(high + 0xc0, 0);
    }
    if e.call(0x008a_6650, &args![high, 0u32]).bool() {
        forget_flee_target(e, high, target);
    }
    if e.call(0x0049_3bb0, &args![high]).bool() {
        let controller = e.vcall(high, 0x428, &[]).u32();
        if controller != 0 {
            e.call(0x0097_f9c0, &args![controller, target]);
        }
    }
    e.call(EXTRA_REMOVE_FOLLOWER, &args![extra_list(high), target]);
    if e.vcall(process, 0x128, &[]).u32() == target {
        e.vcall(process, 0x12c, &args![0u32]);
    }
    if e.vcall(high, 0x2c8, &[]).u32() == target {
        e.call(0x0088_1620, &args![high, 0u32]);
    }
}

/// The packages of slot 0x27c of the process and `00881510` of the actor.
fn forget_packages(e: &mut Engine, high: u32, target: u32) {
    let process = actor_process(e, high);
    let current = e.vcall(process, 0x27c, &[]).u32();
    let set_as_current = e.call(0x0088_1510, &args![high]).u32();
    clear_created_package_references(e, current, high, target);
    clear_created_package_references(e, set_as_current, high, target);
}

/// `00976680`'s work for an actor (`high`, with its process) at `index` of the array (or, in the
/// temp list's loop, the array's end index): `tails` are the tails of levels 1 and 0 to compare with,
/// or none for the temp list, which compares the index with 2 and 1 and has no `0x7b0` call.
fn forget_destructed_in_actor(
    e: &mut Engine,
    high: u32,
    target: u32,
    index: u32,
    array: Option<u32>,
) {
    let process = actor_process(e, high);
    clear_head_track(e, high, process, target);
    if process == 0 {
        return;
    }
    forget_packages(e, high, target);
    let below_1 = match array {
        Some(array) => index < array_tail(e, array, 1),
        None => index < 2,
    };
    if below_1 {
        e.vcall(process, 0x2c4, &args![target, 3u32]);
        end_package_when_targeted(e, high, process, target);
        let topic = e.vcall(target, 0x198, &[]).u32();
        e.call(0x008c_4f10, &args![high, topic]);
        let below_0 = match array {
            Some(array) => index < array_tail(e, array, 0),
            None => index < 1,
        };
        if below_0 {
            e.vcall(process, 0x664, &args![target]);
        }
    }
    e.vcall(process, 0x47c, &args![target]);
    if array.is_some() {
        e.vcall(process, 0x7b0, &args![target]);
    }
    forget_actor_tail(e, high, process, target);
}

/// A non-actor of slot 0x224 (`009c4c80`) or 0x220 (`009b28a0`) told about `target`.
fn tell_non_actor(e: &mut Engine, object: u32, target: u32) {
    if e.vcall(object, 0x224, &[]).bool() {
        e.call(0x009c_4c80, &args![object, target]);
    } else if e.vcall(object, 0x220, &[]).bool() {
        e.call(0x009b_28a0, &args![object, target]);
    }
}

// Typed from 00976680
/// The clean-up a reference's destructor runs (`thiscall(lists, target)`): after `00992920` on the
/// object at `011f1958`, an actor `target` leaves the player's followers and detection and the
/// player's head-track target; every object of the array up to the tail of level 3 that is not
/// skipped is cleaned (an actor through its packages, detection and targets, a non-actor through
/// `009c4c80` or `009b28a0`), then every actor of the temp list (where a skipped one or a non-actor
/// makes the last object of the first loop told again, as the exe does), then the actors close to
/// the player lose it.
pub fn reference_destructed(e: &mut Engine, this: Ptr, target: u32) {
    let array = this.addr().wrapping_add(MOB_PROCESS_ARRAY);
    let observer = e.mem.u32(OBSERVER);
    e.call(0x0099_2920, &args![observer, target]);
    let player = e.mem.u32(PLAYER);
    if is_actor(e, target) {
        leave_player_followers(e, player, target);
        if word_70(e, player) == target {
            e.call(0x0057_bd60, &args![player, 0u32]);
        }
    }

    let mut index = 0u32;
    let mut last_object = 0u32;
    while index < array_tail(e, array, 3) {
        last_object = array_object(e, array, index);
        let object = last_object;
        if object != 0 && !skipped(e, object) {
            if is_actor(e, object) {
                forget_destructed_in_actor(e, object, target, index, Some(array));
            } else if e.vcall(object, 0x224, &[]).bool() {
                e.call(0x009c_4c80, &args![object, target]);
                if typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, object))
                    && array_object(e, array, index) != object
                {
                    index = index.wrapping_sub(1);
                }
            } else if e.vcall(object, 0x220, &[]).bool() {
                e.call(0x009b_28a0, &args![object, target]);
            }
        }
        index = index.wrapping_add(1);
    }

    let mut node = this.addr().wrapping_add(TEMP_SHOULD_MOVE_LIST);
    while node != 0 && !typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, node)) {
        let item = node_value(e, node);
        let high = if is_actor(e, item) {
            node_value(e, node)
        } else {
            0
        };
        if high == 0 || skipped(e, high) {
            tell_non_actor(e, last_object, target);
        } else {
            forget_destructed_in_actor(e, high, target, index, None);
        }
        node = node_next(e, node);
    }
    e.call(REMOVE_CLOSE_TO_PLAYER, &args![this, target]);
}
