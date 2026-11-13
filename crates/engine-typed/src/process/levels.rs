//! The process lists' passes over process levels 3, 2 and 1 (the low, middle-low and middle-high
//! process actors), which `ProcessLists::UpdateProcessLists` runs on the main thread under the
//! process lock. Each walks its level of the process array (the lists' +4), updates the actors that
//! are due by the game hour, and collects the actors that left the level, which it then removes from
//! the level ([`array::remove_actor`]).

use super::array;
use super::FAMILY;
use crate::{dispatch, multibound};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The byte that, when set, skips the per-level updates (`011e0e34`).
const UPDATES_DISABLED: u32 = 0x011e_0e34;
/// The game calendar, which `Calendar::GetHour` (`00867da0`, by address) is called on.
const CALENDAR: u32 = 0x011d_e7b8;
const GET_HOUR: u32 = 0x0086_7da0;
/// The lock (`011f11a0`) the dispatch family's `0040fbf0` enters and `0040fba0` leaves around a
/// process change.
const PROCESS_LOCK: u32 = 0x011f_11a0;
/// The low pass's saved index (`011e0e30`), its time budget setting (`011e0e64`, milliseconds) and
/// the counter's frequency (`011e0e20`, eight bytes).
const SAVED_INDEX: u32 = 0x011e_0e30;
const BUDGET_SETTING: u32 = 0x011e_0e64;
const FREQUENCY: u32 = 0x011e_0e20;
/// `QueryPerformanceCounter` (the import).
const QUERY_COUNTER: u32 = 0x00fd_f0a0;
/// The exe's double constants the hour tests compare with: 1.0 hour for the low pass (`01012070`),
/// 0.3 for the middle-low (`010290d0`), 0.15 for the middle-high (`01070c80`).
const ONE_HOUR: u32 = 0x0101_2070;
const MIDDLE_LOW_INTERVAL: u32 = 0x0102_90d0;
const MIDDLE_HIGH_INTERVAL: u32 = 0x0107_0c80;

/// A `double` constant of the exe.
fn double_at(e: &Engine, address: u32) -> f64 {
    f64::from_bits(e.mem.u64(address))
}

/// The process object of an actor (`008d8520`: the word at +0x68).
fn process_of(e: &mut Engine, actor: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, actor))
}

/// `Calendar::GetHour` (by address), as the exe stores it: a float.
fn game_hour(e: &mut Engine) -> f32 {
    e.call(GET_HOUR, &args![CALENDAR]).f32()
}

// Typed from 007df1f0
/// `thiscall(process)`: the float at +0x20 (`fHourLastProcessed`, Xbox PDB, of a `LowProcess`), in
/// `ST0` by `FLD`, which quiets a signalling NaN. A plain field getter: it reads that word of any
/// object.
pub fn hour_last_processed(e: &mut Engine, this: Ptr) -> f32 {
    f32::from_bits(x87_quiet_bits(e.mem.u32(this.addr().wrapping_add(0x20))))
}

/// The process lock's enter and leave (the dispatch family's).
fn lock(e: &mut Engine) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, Ptr::new(PROCESS_LOCK), 0u32)
    );
}

fn unlock(e: &mut Engine) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fba0,
        dispatch::refs::unlock(e, Ptr::new(PROCESS_LOCK))
    );
}

/// Makes the 8-byte actor list the passes collect actors in: `00401000(8)`, then the constructor
/// `0096a2d0` when the allocation succeeded (0 otherwise).
fn new_actor_list(e: &mut Engine) -> u32 {
    let block = e.call(0x0040_1000, &args![8u32]).u32();
    if block != 0 {
        e.call(0x0096_a2d0, &args![block]).u32()
    } else {
        0
    }
}

/// The actor of a list node, when the node is not null and holds one (`006815c0` gives the node's
/// element).
fn node_actor(e: &mut Engine, node: u32) -> Option<u32> {
    if node == 0 {
        return None;
    }
    let element = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    Some(e.mem.u32(element)).filter(|&actor| actor != 0)
}

/// Under the process lock, deletes the actor's process object (its vtable slot 0 with 1) and clears
/// it (`00407800(actor, 0)`).
fn drop_process(e: &mut Engine, actor: u32) {
    lock(e);
    let process = process_of(e, actor);
    if process != 0 {
        e.vcall(process, 0, &args![1u32]);
    }
    e.call(0x0040_7800, &args![actor, 0u32]);
    unlock(e);
}

/// The tail the three passes share: every actor collected in `list` is removed from process level
/// `level` ([`array::remove_actor`]); an actor with form flag 0x800 (`00440da0`) loses its process
/// object; the list is then emptied (`00470470`) and deleted (`004702f0(list, 1)`, when not null).
/// With `use_slot_2ec` (the middle-high pass), an actor whose vtable slot `0x2ec` answers true
/// instead gets its own vtable slot `0x10` with 1 under the lock.
fn remove_collected(e: &mut Engine, this: Ptr, list: u32, level: u32, use_slot_2ec: bool) {
    let array_at = Ptr::new(this.addr().wrapping_add(4));
    let mut node = list;
    while let Some(actor) = node_actor(e, node) {
        typed_call!(
            FAMILY,
            0x0096_aaf0,
            array::remove_actor(e, array_at, actor, level as i32)
        );
        if use_slot_2ec && e.vcall(actor, 0x2ec, &args![]).bool() {
            lock(e);
            if actor != 0 {
                e.vcall(actor, 0x10, &args![1u32]);
            }
            unlock(e);
        } else if typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, actor)) {
            drop_process(e, actor);
        }
        node = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node));
    }
    e.call(0x0047_0470, &args![list]);
    if list != 0 {
        e.call(0x0047_02f0, &args![list, 1u32]);
    }
}

/// The update of an actor whose process says it may run (`process` vtable slot `0x1ec`): the
/// actor's vtable slot `0x25c` with `time` (through the x87, which quiets a signalling NaN), its
/// script (`TESObjectREFR::RunScript`, `00565870`) when `Script::GetProcessScripts` (`005ac740`) is
/// true, and `004f8960` on the actor when its vtable slot `0x100` is true.
fn run_actor_update(e: &mut Engine, actor: u32, process: u32, time: f32) {
    if !e.vcall(process, 0x1ec, &args![]).bool() {
        return;
    }
    e.vcall(actor, 0x25c, &args![x87_quiet_bits(time.to_bits())]);
    if e.call(0x005a_c740, &args![]).bool() {
        e.call(0x0056_5870, &args![actor]);
    }
    if actor != 0 && e.vcall(actor, 0x100, &args![]).bool() {
        e.call(0x004f_8960, &args![actor]);
    }
}

/// The actor at `index` of the process array (the lists' +4).
fn actor_at(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let array_at = Ptr::new(this.addr().wrapping_add(4));
    typed_call!(FAMILY, 0x0096_8670, array::actor_at(e, array_at, index))
}

/// A level's first and end indexes.
fn level_range(e: &mut Engine, this: Ptr, level: u32) -> (u32, u32) {
    let array_at = Ptr::new(this.addr().wrapping_add(4));
    let begin = typed_call!(FAMILY, 0x005d_e0f0, array::level_head(e, array_at, level));
    let end = typed_call!(FAMILY, 0x005b_e5c0, array::level_tail(e, array_at, level));
    (begin, end)
}

// Typed from 0096b050
/// The pass over process level 3 (`thiscall(lists, time, all)`, `RET 8`). It runs from the index
/// saved at `011e0e30` (reset to the level's first index when 0 or at or past the level's end) to
/// the level's end, saving the next index after each actor. Unless `all` (its low byte) is set,
/// every fifth index checks the time budget: the deadline is `QueryPerformanceCounter` plus
/// `budget * frequency / 1000` (the exe's `__allmul` and `__alldiv`, signed 64-bit) with the budget
/// the word `0043d4d0(011e0e64)` points to and the frequency at `011e0e20`; past it the loop stops.
/// An actor with a process object and without form flag 0x200000 is collected for removal when it
/// has form flag 0x800 or 0x20 or its process level (`0045cd60`) is not 3; otherwise, unless updates
/// are disabled (`011e0e34`), an actor without the byte at +0x87 and one whose last processed hour
/// is -1 get `LowProcess::ComputeLastTimeProcessed` (`00907650`) on the process; the others update
/// when the hours passed since (a float) are at least 1.0, negative, or `time` is not 0. The
/// collected actors leave level 3.
pub fn update_low(e: &mut Engine, this: Ptr, time: f32, all: u32) {
    let list = new_actor_list(e);
    let deadline = e.with_stack(8, |e, counter| {
        e.call(QUERY_COUNTER, &args![counter]);
        let start = e.mem.u64(counter.addr());
        let budget_slot = typed_call!(
            multibound::FAMILY,
            0x0043_d4d0,
            multibound::getters::setting_value(e, Ptr::new(BUDGET_SETTING))
        );
        let budget = i64::from(e.mem.u32(budget_slot));
        let frequency = e.mem.u64(FREQUENCY) as i64;
        let ticks = budget.wrapping_mul(frequency).wrapping_div(1000);
        (ticks as u64).wrapping_add(start)
    });
    let array_at = Ptr::new(this.addr().wrapping_add(4));
    let end = typed_call!(FAMILY, 0x005b_e5c0, array::level_tail(e, array_at, 3));
    let saved = e.mem.u32(SAVED_INDEX);
    if saved == 0 || saved >= end {
        let first = typed_call!(FAMILY, 0x005d_e0f0, array::level_head(e, array_at, 3));
        e.mem.set_u32(SAVED_INDEX, first);
    }
    let hour = game_hour(e);
    let mut index = e.mem.u32(SAVED_INDEX);
    e.with_stack(0xc, |e, locals| {
        let actor_slot = locals.addr();
        let now = actor_slot + 4;
        while index < end {
            let actor = actor_at(e, this, index);
            e.mem.set_u32(actor_slot, actor);
            let mut out_of_time = false;
            if all & 0xff == 0 && index % 5 == 0 {
                e.call(QUERY_COUNTER, &args![now]);
                out_of_time = e.mem.u64(now) as i64 >= deadline as i64;
            }
            if actor != 0 {
                low_actor(e, actor, actor_slot, list, hour, time);
            }
            index = index.wrapping_add(1);
            e.mem.set_u32(SAVED_INDEX, index);
            if out_of_time {
                break;
            }
        }
    });
    remove_collected(e, this, list, 3, false);
}

/// One actor of [`update_low`]'s loop (the actor is also in the word at `actor_slot`).
fn low_actor(e: &mut Engine, actor: u32, actor_slot: u32, list: u32, hour: f32, time: f32) {
    let process = process_of(e, actor);
    if process == 0 || typed_call!(leaf::FAMILY, 0x0057_6d30, leaf::form_flag_200000(e, actor)) {
        return;
    }
    if typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, actor))
        || typed_call!(leaf::FAMILY, 0x0044_0d80, leaf::form_flag_20(e, actor))
        || typed_call!(leaf::FAMILY, 0x0045_cd60, leaf::word_at_28(e, process)) != 3
    {
        e.call(0x005a_e3d0, &args![list, actor_slot]);
        return;
    }
    if e.mem.u8(UPDATES_DISABLED) != 0 {
        return;
    }
    if typed_call!(leaf::FAMILY, 0x0043_7bb0, leaf::byte_at_87(e, actor)) == 0 {
        e.call(0x0090_7650, &args![process]);
        return;
    }
    let last = typed_call!(
        FAMILY,
        0x007d_f1f0,
        hour_last_processed(e, Ptr::new(process))
    );
    if f64::from(last) == -1.0 {
        e.call(0x0090_7650, &args![process]);
        return;
    }
    // `FSUB` at the main thread's 24-bit precision, stored as a float; `FCOMP` with the doubles.
    let passed = hour - last;
    if f64::from(passed) >= double_at(e, ONE_HOUR) || passed < 0.0 || time != 0.0 {
        run_actor_update(e, actor, process, time);
    }
}

/// The hour test of the middle-low and middle-high passes once both hours are read (floats): -1 as
/// the last hour runs `0092f190` on the actor; otherwise the actor updates (with a fresh process
/// read) when `hour - last` is at least the double at `interval`, the hour is before the last one,
/// or `time` is not 0.
fn timed_update(e: &mut Engine, actor: u32, time: f32, interval: u32, last: f32, hour: f32) {
    if f64::from(last) == -1.0 {
        e.call(0x0092_f190, &args![actor]);
        return;
    }
    if f64::from(hour - last) >= double_at(e, interval) || hour < last || time != 0.0 {
        let process = process_of(e, actor);
        run_actor_update(e, actor, process, time);
    }
}

/// Whether an actor of the middle passes left its level: form flag 0x800 or 0x20, or a process type
/// (`MobileObject::GetCurrentProcessType`, `00931850`, by address) other than `level`.
fn left_level(e: &mut Engine, actor: u32, level: u32) -> bool {
    typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, actor))
        || typed_call!(leaf::FAMILY, 0x0044_0d80, leaf::form_flag_20(e, actor))
        || e.call(0x0093_1850, &args![actor]).u32() != level
}

// Typed from 0096b470
/// The pass over process level 2 (`thiscall(lists, time, unused)`, `RET 8`). For each actor of the
/// level (not null, without form flag 0x200000): collected for removal when it left the level;
/// otherwise, unless updates are disabled, a character (vtable slot `0x100`) without the byte at
/// +0x87 and without `00493bb0` gets its process's vtable slot `0x28`; the others take the hour test
/// with 0.3 hour. The collected actors leave level 2.
pub fn update_middle_low(e: &mut Engine, this: Ptr, time: f32, _unused: u32) {
    let list = new_actor_list(e);
    let (begin, end) = level_range(e, this, 2);
    e.with_stack(4, |e, actor_slot| {
        let actor_slot = actor_slot.addr();
        for index in begin..end {
            let actor = actor_at(e, this, index);
            e.mem.set_u32(actor_slot, actor);
            if actor != 0
                && !typed_call!(leaf::FAMILY, 0x0057_6d30, leaf::form_flag_200000(e, actor))
            {
                middle_low_actor(e, actor, actor_slot, list, time);
            }
        }
    });
    remove_collected(e, this, list, 2, false);
}

/// One actor of [`update_middle_low`]'s loop.
fn middle_low_actor(e: &mut Engine, actor: u32, actor_slot: u32, list: u32, time: f32) {
    let character = e.vcall(actor, 0x100, &args![]).bool();
    if left_level(e, actor, 2) {
        e.call(0x005a_e3d0, &args![list, actor_slot]);
        return;
    }
    if e.mem.u8(UPDATES_DISABLED) != 0 {
        return;
    }
    if typed_call!(leaf::FAMILY, 0x0043_7bb0, leaf::byte_at_87(e, actor)) == 0
        && character
        && !e.call(0x0049_3bb0, &args![actor]).bool()
    {
        let process = process_of(e, actor);
        e.vcall(process, 0x28, &args![]);
        return;
    }
    let process = process_of(e, actor);
    let last = typed_call!(
        FAMILY,
        0x007d_f1f0,
        hour_last_processed(e, Ptr::new(process))
    );
    let hour = game_hour(e);
    timed_update(e, actor, time, MIDDLE_LOW_INTERVAL, last, hour);
}

// Typed from 0096b810
/// The pass over process level 1 (`thiscall(lists, time, unused)`, `RET 8`). For each actor of the
/// level (not null, without form flag 0x200000): collected for removal when it left the level;
/// otherwise, unless updates are disabled, the last processed hour and the game hour are read, the
/// actor's vtable slot `0x100` is asked (its answer is not used), and the hour test runs with 0.15
/// hour. The collected actors leave level 1; one whose vtable slot `0x2ec` is true gets its slot
/// `0x10` with 1 under the lock instead of losing its process.
pub fn update_middle_high(e: &mut Engine, this: Ptr, time: f32, _unused: u32) {
    let list = new_actor_list(e);
    let (begin, end) = level_range(e, this, 1);
    e.with_stack(4, |e, actor_slot| {
        let actor_slot = actor_slot.addr();
        for index in begin..end {
            let actor = actor_at(e, this, index);
            e.mem.set_u32(actor_slot, actor);
            if actor == 0
                || typed_call!(leaf::FAMILY, 0x0057_6d30, leaf::form_flag_200000(e, actor))
            {
                continue;
            }
            if left_level(e, actor, 1) {
                e.call(0x005a_e3d0, &args![list, actor_slot]);
            } else if e.mem.u8(UPDATES_DISABLED) == 0 {
                let process = process_of(e, actor);
                let last = typed_call!(
                    FAMILY,
                    0x007d_f1f0,
                    hour_last_processed(e, Ptr::new(process))
                );
                let hour = game_hour(e);
                e.vcall(actor, 0x100, &args![]);
                timed_update(e, actor, time, MIDDLE_HIGH_INTERVAL, last, hour);
            }
        }
    });
    remove_collected(e, this, list, 1, true);
}
