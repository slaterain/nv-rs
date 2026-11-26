//! The package list's functions (`PackageList`, Xbox PDB, the unit `fallout shared/packagelist`
//! and the units beside it: `run_0067e3a0`, `run_0067e440`, `run_0067ec40`), the condition
//! list's append the compiler shares with it (`0067e420`, in `tescondition`), and the schedule
//! window test, each as the exe does it.
//!
//! A package list is a `BSSimpleList<TESPackage *>` whose head node is inside the object that
//! holds it (the AI form's at +0x18): each node is an item word, then the next node. The list is
//! empty when both words of its head are zero. A package (`TESPackage`) has its flags at +0x1c,
//! its procedure type at +0x18, its target at +0x30, its schedule at +0x38 (`PSDT`: the month, a
//! signed byte, then the day of the week, the date and the hour, bytes, and the duration in hours,
//! a word at +4) and its conditions at +0x40.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): a node's item slot (`006815c0`, the node itself) and next node
//! (`00726070`, its word at +4), the list's empty test (`008256d0`, both words of the head zero),
//! a node's item store (`00726c60`, when the word handed is not zero), the count of a list's
//! items (`005ae380`), a package's schedule (`0041d8a0`, +0x38), conditions (`0087eaa0`, +0x40),
//! target (`00671d10`, the word at +0x30), procedure type (`009611e0`, the word at +0x18) and
//! flag tests (`00674e40` 0x8000, `00670f90` 0x400 of the word at +0x1c), the schedule's fields
//! (`00678ca0` the month, `00678cc0` the day of the week, `00678ce0` the date, `0086a460` the
//! hour), a reference's extra data list (`005d43c0`, +0x44) and process (`008d8520`, the word at
//! +0x68), and the word swap (`00401080`, over the runtime's `_byteswap_ulong`). This unit's
//! functions call each other as typed calls, and so are the form lookup and the compile index
//! (this family's), a reference's current package and a package's type (`ai_queries`'), the
//! run-once-a-day list's test (`extra_data`'s), the calendar's day (`ai`'s), the interface's
//! target (`interface`'s) and `_ftol2` (the C runtime's). What stays a call by address: the
//! list's clear, destructor, append and removals, the RTTI cast, the strings, the warning logger
//! (which a fix may replace), the package's flag test, its procedure type's calculation and
//! the per-actor test, the conditions' test, the calendar's month, hour and day of the week, the
//! action flag setter (which a fix may replace), the follower's removal and the days in a month.
//! Every virtual call is made through its slot: the package classes override them.
//!
//! Floats: the schedule window's hour is truncated by `_ftol2` from the `f32` the exe loads
//! (`FLD`); the missed packages' hour is loaded (`FLD`, which quiets a signalling NaN), stepped by
//! one hour with the exe's x87 add under the running thread's control word and stored as the
//! `f32` the exe stores, compared with the schedule's hour as the x87 compare does (an unordered
//! pair fails) and truncated through `_ftol2`; the calendar's hour is passed on as the `f32` the
//! exe stores (`FSTP`).

use super::{tesform, FAMILY};
use crate::{ai, ai_queries, crt, extra_data, interface};
use core::cmp::Ordering::Greater;
use engine_core::x87::{x87_add, x87_load32, x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine, Ptr};

/// The list's clear (`00470470`), its destructor (`0046ffb0`), its append of the item a word
/// holds (`00905820`), its first node's removal (`0063f7b0`) and the removal after a node of the
/// item a word holds (`00905330`), each with `ECX` = the list or node.
const LIST_CLEAR: u32 = 0x0047_0470;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_APPEND: u32 = 0x0090_5820;
const LIST_POP_FRONT: u32 = 0x0063_f7b0;
const LIST_REMOVE_AFTER: u32 = 0x0090_5330;
/// `__RTDynamicCast` (cdecl: object, vftable delta, source type, target type, is-reference).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_PACKAGE: u32 = 0x0118_46a0;
const TYPE_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const TYPE_ACTOR: u32 = 0x0118_46d4;
/// `BSString` on the stack (8 bytes): its constructor, its set (text, 0), its destructor.
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
const STRING_SET: u32 = 0x0040_37f0;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
/// The warning logger (cdecl: format, words), which a fix may replace: called by address.
const WARNING: u32 = 0x005b_5e40;
/// `MASTERFILE: Could not find Package (%08X) for %s.` and "UNKNOWN form".
const TEXT_NO_PACKAGE: u32 = 0x0106_8c94;
const TEXT_UNKNOWN_FORM: u32 = 0x0106_8cc8;
/// `"%s Picked Package %s (%s)"`, `sprintf` (cdecl), the line's buffer size and the console's
/// print of a line (cdecl: the line).
const TEXT_PICKED_PACKAGE: u32 = 0x0106_8cd8;
const SPRINTF: u32 = 0x00ec_623a;
const LINE_SIZE: u32 = 0x134;
const CONSOLE_PRINT: u32 = 0x0070_3c00;
/// A text's own copy (`00464f30`: the text, 0), a reference's name (`0055d520`) and the package
/// types' names (a table of text pointers).
const TEXT_COPY: u32 = 0x0046_4f30;
const REFERENCE_NAME: u32 = 0x0055_d520;
const PACKAGE_TYPE_NAMES: u32 = 0x0119_bcb0;
/// The package's vtable slots: whether it may run for a reference (one word), its name.
const SLOT_MAY_RUN: u32 = 0x138;
const SLOT_NAME: u32 = 0x130;
/// The process's vtable slots: the object it acts on, the package's end (reference, package),
/// the next package's start (0).
const SLOT_PROCESS_OBJECT: u32 = 0x128;
const SLOT_PROCESS_PACKAGE_END: u32 = 0x59c;
const SLOT_PROCESS_PACKAGE_START: u32 = 0x5a8;
/// The package's flag test with a word (`00678d40`), its procedure type's calculation
/// (`TESPackage::CalculateProcedureType`, `006777b0`: package, value), the reference's value it
/// takes (`00569b80`) and the per-actor test (`00677760`: package, actor).
const PACKAGE_FLAG_TEST: u32 = 0x0067_8d40;
const PACKAGE_CALCULATE_PROCEDURE: u32 = 0x0067_77b0;
const REFERENCE_PROCEDURE_VALUE: u32 = 0x0056_9b80;
const PACKAGE_ACTOR_TEST: u32 = 0x0067_7760;
/// `PackageTarget::GetTargReference` (`00680020`) and `TESCondition`'s test (`00680c30`:
/// conditions, reference, target reference).
const TARGET_REFERENCE: u32 = 0x0068_0020;
const CONDITIONS_HOLD: u32 = 0x0068_0c30;
/// `ExtraDataList::RemoveFollower` (`00422690`: list, reference).
const REMOVE_FOLLOWER: u32 = 0x0042_2690;
/// `Script::SetActionFlag` (cdecl: package, extra list, flag), which a fix may replace: called by
/// address.
const SET_ACTION_FLAG: u32 = 0x005a_c750;
const ACTION_ENDED: u32 = 0x800;
const ACTION_STARTED: u32 = 0x200;
/// The output list's test for an item a word holds (`005f65d0`).
const LIST_CONTAINS: u32 = 0x005f_65d0;
/// The days in a month (cdecl, a `u16`).
const DAYS_IN_MONTH: u32 = 0x004b_10d0;
/// The calendar (`011de7b8`) and its month (`00867d20`), hour (`00867da0`, `ST0`) and day of the
/// week (`00867ef0`).
const CALENDAR: u32 = 0x011d_e7b8;
const CALENDAR_MONTH: u32 = 0x0086_7d20;
const CALENDAR_HOUR: u32 = 0x0086_7da0;
const CALENDAR_DAY_OF_WEEK: u32 = 0x0086_7ef0;
/// The `double`s the missed packages' hour steps by and ends a day past (the exe's read-only
/// `0101_2070` and `0103_5838`, 1.0 and 23.0).
const ONE_HOUR: f64 = 1.0;
const LAST_HOUR: f64 = 23.0;

// A package's fields.
const PROCEDURE_TYPE: u32 = 0x18;
const PACKAGE_FLAGS: u32 = 0x1c;
const TARGET: u32 = 0x30;
const SCHEDULE: u32 = 0x38;
const CONDITIONS: u32 = 0x40;
const FLAG_MISSABLE: u32 = 4;
const FLAG_ONCE_PER_DAY: u32 = 0x400;
const FLAG_NEVER_TO_RUN: u32 = 0x8000;
// A reference's extra data list and process.
const REFERENCE_EXTRA: u32 = 0x44;
const REFERENCE_PROCESS: u32 = 0x68;
// The schedule's fields.
const MONTH: u32 = 0;
const DAY_OF_WEEK: u32 = 1;
const DATE: u32 = 2;
const HOUR: u32 = 3;
const DURATION: u32 = 4;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// Whether the list whose head is `list` is empty (`008256d0`: both words of the head zero).
fn is_empty(e: &Engine, list: u32) -> bool {
    e.mem.u32(at(list, 4)) == 0 && e.mem.u32(list) == 0
}

/// A node's item (the word in its slot, `006815c0`) and its next node (`00726070`).
fn item_of(e: &Engine, node: u32) -> u32 {
    e.mem.u32(node)
}

fn next_of(e: &Engine, node: u32) -> u32 {
    e.mem.u32(at(node, 4))
}

fn schedule_month(e: &Engine, schedule: u32) -> i32 {
    i32::from(e.mem.i8(at(schedule, MONTH)))
}

fn schedule_day_of_week(e: &Engine, schedule: u32) -> i32 {
    i32::from(e.mem.i8(at(schedule, DAY_OF_WEEK)))
}

fn schedule_date(e: &Engine, schedule: u32) -> i32 {
    i32::from(e.mem.i8(at(schedule, DATE)))
}

fn schedule_hour(e: &Engine, schedule: u32) -> i32 {
    i32::from(e.mem.i8(at(schedule, HOUR)))
}

fn schedule_duration(e: &Engine, schedule: u32) -> i32 {
    e.mem.i32(at(schedule, DURATION))
}

fn package_flag(e: &Engine, package: u32, flag: u32) -> bool {
    e.mem.u32(at(package, PACKAGE_FLAGS)) & flag != 0
}

fn calendar_day(e: &mut Engine) -> i32 {
    let day = typed_call!(
        ai::FAMILY,
        0x0086_7d60,
        ai::middle_low_process::calendar_day(e, Ptr::new(CALENDAR))
    );
    i32::from(i8::from_le_bytes([day.to_le_bytes()[0]]))
}

/// `_ftol2` of `value`, its low word as the exe takes it from `EAX`.
fn truncate(e: &mut Engine, value: f64) -> i32 {
    let whole = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, value));
    i32::from_ne_bytes(crt::low_word(whole).to_ne_bytes())
}

// Typed from 0067e3a0
/// A package list's destructor: [`free`], then the list's destructor.
pub fn destruct(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0067_e400, free(e, this));
    e.call(LIST_DESTRUCT, &args![this]);
}

// Typed from 0067e400
/// `PackageList::FreePackageList` (Xbox PDB): the list cleared (its head is the object itself).
pub fn free(e: &mut Engine, this: u32) {
    e.call(LIST_CLEAR, &args![this]);
}

// Typed from 0067e420
/// `BSSimpleList<TESConditionItem_P>::AddTail` (Xbox PDB; the package list's append shares the
/// body): `item` appended through the stack word that holds it.
pub fn add_tail(e: &mut Engine, this: u32, item: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        e.call(LIST_APPEND, &args![this, slot]);
    });
}

// Typed from 0067e440
/// A package list's copy: this list freed, then each item of `other` up to a null node appended
/// (none when `other` is empty).
pub fn copy(e: &mut Engine, this: u32, other: u32) {
    typed_call!(FAMILY, 0x0067_e400, free(e, this));
    if is_empty(e, other) {
        return;
    }
    let mut node = other;
    while node != 0 {
        let item = item_of(e, node);
        typed_call!(FAMILY, 0x0067_e420, add_tail(e, this, item));
        node = next_of(e, node);
    }
}

// Typed from 0067e4a0
/// Whether two package lists differ: an empty list differs from a non-empty one; otherwise the
/// items are compared pair by pair while both nodes are set and hold an item, and the lists differ
/// at a different pair or when either has a node left.
pub fn lists_differ(e: &mut Engine, this: u32, other: u32) -> bool {
    if is_empty(e, this) {
        return !is_empty(e, other);
    }
    let (mut mine, mut theirs) = (this, other);
    loop {
        if mine == 0 || theirs == 0 {
            break;
        }
        let (my_item, their_item) = (item_of(e, mine), item_of(e, theirs));
        if my_item == 0 || their_item == 0 {
            break;
        }
        if my_item != their_item {
            return true;
        }
        mine = next_of(e, mine);
        theirs = next_of(e, theirs);
    }
    mine != 0 || theirs != 0
}

// Typed from 0067e560
/// `PackageList::InitItem` (Xbox PDB; `owner` the form that holds the list, or null): each node's
/// form id (given the owner's compile index when there is an owner) looked up and cast to a
/// package. A package is stored in its node, which becomes the previous node; otherwise a warning
/// names the id and the owner (its name slot fills a string; "UNKNOWN form" without one) and the
/// node is dropped: after the previous node when there is one (the walk going on from the
/// previous node's next), else from the front.
pub fn init_item(e: &mut Engine, this: u32, owner: u32) {
    let mut node = this;
    let mut previous = 0u32;
    let file = if owner != 0 {
        typed_call!(FAMILY, 0x0048_4e60, tesform::get_file(e, owner, -1))
    } else {
        0
    };
    while node != 0 && !is_empty(e, node) {
        let id = e.with_stack(4, |e, slot| {
            let id = item_of(e, node);
            e.mem.set_u32(slot.addr(), id);
            if owner != 0 {
                let slot = slot.addr();
                typed_call!(
                    FAMILY,
                    0x0048_5d50,
                    tesform::add_compile_index(e, slot, file)
                );
            }
            e.mem.u32(slot.addr())
        });
        let form = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id));
        let package = e
            .call(
                DYNAMIC_CAST,
                &args![form, 0u32, TYPE_TES_FORM, TYPE_TES_PACKAGE, 0u32],
            )
            .u32();
        if package != 0 {
            e.mem.set_u32(node, package);
            previous = node;
            node = next_of(e, node);
            continue;
        }
        e.with_stack(8, |e, name| {
            e.call(STRING_CONSTRUCT, &args![name]);
            if owner != 0 {
                e.vcall(owner, 0x90, &args![name]);
            } else {
                e.call(STRING_SET, &args![name, TEXT_UNKNOWN_FORM, 0u32]);
            }
            let text = e.mem.u32(name.addr());
            e.call(WARNING, &args![TEXT_NO_PACKAGE, id, text]);
            if previous != 0 {
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), id);
                    e.call(LIST_REMOVE_AFTER, &args![previous, slot]);
                });
                node = next_of(e, previous);
            } else {
                e.call(LIST_POP_FRONT, &args![node]);
            }
            e.call(STRING_DESTRUCT, &args![name]);
        });
    }
}

// Typed from 0067e700
/// Whether `hour` (truncated by `_ftol2`) falls in the window of `duration` hours from `start`:
/// at `start` itself; for a window past hour 23, after `start` or before its end less 24, and
/// before its end; otherwise strictly between `start` and the end. Signed compares.
pub fn in_schedule_window(e: &mut Engine, start: i32, duration: i32, hour: f32) -> bool {
    let hour = truncate(e, x87_load32(hour.to_bits()));
    if hour == start {
        return true;
    }
    let end = start.wrapping_add(duration);
    if end > 0x17 {
        (hour > start || hour < end.wrapping_sub(0x18)) && hour < end
    } else {
        hour > start && hour < end
    }
}

/// Whether `FindCurrentPackage` passes over `package` before its schedule: unless it may run for
/// the reference (its slot) and is neither never to run nor once a day, a package once a day
/// passes only when it is not in the reference's run-once-a-day list.
fn skipped_before_schedule(e: &mut Engine, package: u32, reference: u32) -> bool {
    let considered = e.vcall(package, SLOT_MAY_RUN, &args![reference]).bool()
        && !package_flag(e, package, FLAG_NEVER_TO_RUN)
        && !package_flag(e, package, FLAG_ONCE_PER_DAY);
    if considered {
        return false;
    }
    if !package_flag(e, package, FLAG_ONCE_PER_DAY) {
        return true;
    }
    let list = at(reference, REFERENCE_EXTRA);
    typed_call!(
        extra_data::FAMILY,
        0x0041_d8c0,
        extra_data::packages::is_in_run_once_packages(e, list, package)
    )
}

/// The schedule's tests in `FindCurrentPackage`: true when the package may run now. A window
/// past hour 24 also takes the day and the month before today's (the package's flag test with 1
/// standing for the day of the week).
fn schedule_allows(e: &mut Engine, package: u32, schedule: u32) -> bool {
    let mut late_flag = false;
    let mut day_before = false;
    let mut month_before = false;
    if schedule_hour(e, schedule).wrapping_add(schedule_duration(e, schedule)) > 0x18 {
        late_flag = e.call(PACKAGE_FLAG_TEST, &args![package, 1u32]).bool();
        let mut day = calendar_day(e).wrapping_sub(1);
        if day < 0 {
            day = 0x1e;
        }
        day_before = schedule_date(e, schedule) == day;
        let mut month = e
            .call(CALENDAR_MONTH, &args![CALENDAR])
            .i32()
            .wrapping_sub(1);
        if month < 0 {
            month = 0xc;
        }
        month_before = schedule_month(e, schedule) == month;
    }
    let month = schedule_month(e, schedule);
    if month != -1 && month != e.call(CALENDAR_MONTH, &args![CALENDAR]).i32() && !month_before {
        return false;
    }
    let date = schedule_date(e, schedule);
    if date != 0 && date != calendar_day(e) && !day_before {
        return false;
    }
    if schedule_day_of_week(e, schedule) != -1
        && !e.call(PACKAGE_FLAG_TEST, &args![package, 0u32]).bool()
        && !late_flag
    {
        return false;
    }
    let start = schedule_hour(e, schedule);
    if start != -1 {
        let hour = e.call(CALENDAR_HOUR, &args![CALENDAR]).f64();
        let hour = f32::from_bits(x87_store32(hour));
        let duration = schedule_duration(e, schedule);
        if !typed_call!(
            FAMILY,
            0x0067_e700,
            in_schedule_window(e, start, duration, hour)
        ) {
            return false;
        }
    }
    true
}

/// The package's conditions with the reference and its target's reference (0 without a target).
fn conditions_hold(e: &mut Engine, package: u32, reference: u32) -> bool {
    let target = e.mem.u32(at(package, TARGET));
    let target_reference = if target != 0 {
        e.call(TARGET_REFERENCE, &args![target]).u32()
    } else {
        0
    };
    e.call(
        CONDITIONS_HOLD,
        &args![at(package, CONDITIONS), reference, target_reference],
    )
    .bool()
}

// Typed from 0067e780
/// `PackageList::FindCurrentPackage` (Xbox PDB; `reference` or null): the first package of the
/// list that may run for the reference now, or the reference's current package (0 without a
/// reference) when none does, the list is empty or a null item ends it. A package is passed over
/// before its schedule, by its schedule (month, date, day of the week, the hour window) or by its
/// conditions. For the package found: a "Picked Package" line on the console when the reference
/// is the interface's target; its procedure type calculated when unset; and when it replaces
/// another current package: a type-1 package's follower removed from the actor its process acts
/// on, the old package's end flagged and run through the process, and the new package's start
/// flagged.
pub fn find_current_package(e: &mut Engine, this: u32, reference: u32) -> u32 {
    let current = if reference != 0 {
        typed_call!(
            ai_queries::FAMILY,
            0x0093_44a0,
            ai_queries::queries::current_package(e, reference)
        )
    } else {
        0
    };
    if is_empty(e, this) {
        return current;
    }
    let mut node = this;
    while node != 0 {
        let package = item_of(e, node);
        if package == 0 {
            return current;
        }
        if !skipped_before_schedule(e, package, reference) {
            let schedule = at(package, SCHEDULE);
            if schedule != 0
                && schedule_allows(e, package, schedule)
                && conditions_hold(e, package, reference)
            {
                picked(e, package, reference, current);
                return package;
            }
        }
        node = next_of(e, node);
    }
    current
}

/// A package's type (`0041ca90`).
fn package_type(e: &mut Engine, package: u32) -> i32 {
    typed_call!(
        ai_queries::FAMILY,
        0x0041_ca90,
        ai_queries::queries::package_type(e, package)
    )
}

/// What `FindCurrentPackage` does with the package it picked.
fn picked(e: &mut Engine, package: u32, reference: u32, current: u32) {
    let target = typed_call!(
        interface::FAMILY,
        0x0070_3180,
        interface::interface_cpp::interface_get_target_refr(e)
    );
    if target == reference {
        let name = e.vcall(package, SLOT_NAME, &[]).u32();
        let package_name = e.call(TEXT_COPY, &args![name, 0u32]).u32();
        let kind = package_type(e, package) as u32;
        let kind_text = e
            .mem
            .u32(PACKAGE_TYPE_NAMES.wrapping_add(kind.wrapping_mul(4)));
        let kind_name = e.call(TEXT_COPY, &args![kind_text, 0u32]).u32();
        let reference_name = e.call(REFERENCE_NAME, &args![reference]).u32();
        e.with_stack(LINE_SIZE, |e, line| {
            e.call(
                SPRINTF,
                &args![
                    line,
                    TEXT_PICKED_PACKAGE,
                    reference_name,
                    kind_name,
                    package_name
                ],
            );
            e.call(CONSOLE_PRINT, &args![line]);
        });
    }
    if e.mem.i32(at(package, PROCEDURE_TYPE)) == -1 {
        let value = e.call(REFERENCE_PROCEDURE_VALUE, &args![reference]).u32();
        e.call(PACKAGE_CALCULATE_PROCEDURE, &args![package, value]);
    }
    if current == 0 || current == package {
        if package != 0 && package != current {
            let list = at(reference, REFERENCE_EXTRA);
            e.call(SET_ACTION_FLAG, &args![package, list, ACTION_STARTED]);
        }
        return;
    }
    if package_type(e, current) == 1 {
        let process = e.mem.u32(at(reference, REFERENCE_PROCESS));
        let object = e.vcall(process, SLOT_PROCESS_OBJECT, &[]).u32();
        let actor = e
            .call(
                DYNAMIC_CAST,
                &args![object, 0u32, TYPE_TES_OBJECT_REFR, TYPE_ACTOR, 0u32],
            )
            .u32();
        if actor != 0 {
            e.call(
                REMOVE_FOLLOWER,
                &args![at(actor, REFERENCE_EXTRA), reference],
            );
        }
    }
    let list = at(reference, REFERENCE_EXTRA);
    e.call(SET_ACTION_FLAG, &args![current, list, ACTION_ENDED]);
    let process = e.mem.u32(at(reference, REFERENCE_PROCESS));
    e.vcall(
        process,
        SLOT_PROCESS_PACKAGE_END,
        &args![reference, current],
    );
    let process = e.mem.u32(at(reference, REFERENCE_PROCESS));
    e.vcall(process, SLOT_PROCESS_PACKAGE_START, &args![0u32]);
    if package != 0 {
        e.call(SET_ACTION_FLAG, &args![package, list, ACTION_STARTED]);
    }
}

// Typed from 0067efd0
/// Whether the package may be missed (bit 2 of its flags): what the missed packages collect.
pub fn is_missable(e: &mut Engine, this: u32) -> bool {
    package_flag(e, this, FLAG_MISSABLE)
}

/// What the missed packages' tests read at one hour step.
struct Step {
    actor: u32,
    out: u32,
    day: i32,
    month: i32,
    hour_bits: u32,
    current: u32,
    /// The stack word holding the package (handed by address).
    slot: u32,
}

/// The tests a package of the list passes before it is added to the missed packages, in the
/// exe's order: missable, the per-actor test, a schedule that has begun by the step's month, date,
/// the calendar's day of the week and hour, not the actor's current package and not yet in `out`.
fn qualifies(e: &mut Engine, package: u32, step: &Step) -> bool {
    if !typed_call!(FAMILY, 0x0067_efd0, is_missable(e, package)) {
        return false;
    }
    if !e
        .call(PACKAGE_ACTOR_TEST, &args![package, step.actor])
        .bool()
    {
        return false;
    }
    let schedule = at(package, SCHEDULE);
    if schedule == 0 {
        return false;
    }
    let month = schedule_month(e, schedule);
    if month != -1 && month > step.month {
        return false;
    }
    let date = schedule_date(e, schedule);
    if date != 0 && date > step.day {
        return false;
    }
    let day_of_week = schedule_day_of_week(e, schedule);
    if day_of_week != -1 && day_of_week > e.call(CALENDAR_DAY_OF_WEEK, &args![CALENDAR]).i32() {
        return false;
    }
    let hour = schedule_hour(e, schedule);
    // The x87 compare of the step's hour with the schedule's: only a greater hour passes (not a
    // lower or equal one, nor an unordered pair).
    if hour != -1 && x87_load32(step.hour_bits).partial_cmp(&f64::from(hour)) != Some(Greater) {
        return false;
    }
    if package == step.current {
        return false;
    }
    !e.call(LIST_CONTAINS, &args![step.out, step.slot]).bool()
}

// Typed from 0067ec40
/// `TESAIForm::GetMissedPackages`' list walk (`this` the list; the third word is never read): from
/// the day and month the actor's process answers (its slots +0x30 and +0x34; +0x38, the year, is
/// asked and never used) and `hour`, hour by hour until the day ends the walk, every package of
/// the list up to a null item that [`qualifies`] (its procedure type calculated when unset) added
/// to `out`. An empty list ends the walk; past hour 23 the day goes on (into the next month by the
/// days in the month, a month past the year's last into the first); reaching the calendar's day
/// and month at or past its hour ends it too. Then `out` is bubble-sorted by the schedules'
/// order (`00678a80`): up to its item count passes, each swapping the items of a node and of
/// every node from it on that are out of order, until a pass swaps none.
pub fn missed_packages(e: &mut Engine, this: u32, actor: u32, out: u32, _unused: u32, hour: f32) {
    let source = e.mem.u32(at(actor, REFERENCE_PROCESS));
    let mut day = e.vcall(source, 0x30, &args![]).i32();
    let source = e.mem.u32(at(actor, REFERENCE_PROCESS));
    let mut month = e.vcall(source, 0x34, &args![]).i32();
    let source = e.mem.u32(at(actor, REFERENCE_PROCESS));
    e.vcall(source, 0x38, &args![]);
    let mut hour_bits = x87_quiet_bits(hour.to_bits());
    let calendar_day = calendar_day(e);
    let calendar_month = e.call(CALENDAR_MONTH, &args![CALENDAR]).i32();
    let mut current = 0u32;

    e.with_stack(4, |e, slot| {
        let slot = slot.addr();
        while day != 0 {
            if actor != 0 {
                current = typed_call!(
                    ai_queries::FAMILY,
                    0x0093_44a0,
                    ai_queries::queries::current_package(e, actor)
                );
            }
            if is_empty(e, this) {
                day = 0;
                continue;
            }
            e.mem.set_u32(slot, 0);
            let mut node = this;
            while node != 0 {
                let package = item_of(e, node);
                e.mem.set_u32(slot, package);
                if package == 0 {
                    break;
                }
                let step = Step {
                    actor,
                    out,
                    day,
                    month,
                    hour_bits,
                    current,
                    slot,
                };
                if qualifies(e, package, &step) {
                    if e.mem.i32(at(package, PROCEDURE_TYPE)) == -1 {
                        let value = e.call(REFERENCE_PROCEDURE_VALUE, &args![actor]).u32();
                        e.call(PACKAGE_CALCULATE_PROCEDURE, &args![package, value]);
                    }
                    e.call(LIST_APPEND, &args![out, slot]);
                }
                node = next_of(e, node);
            }

            hour_bits = x87_store32(x87_add(x87_load32(hour_bits), ONE_HOUR));
            if x87_load32(hour_bits) > LAST_HOUR {
                day = day.wrapping_add(1);
                hour_bits = 0;
                let days = i32::from(
                    e.call(
                        DAYS_IN_MONTH,
                        &args![i32::from(i8::from_le_bytes([month.to_le_bytes()[0]]))],
                    )
                    .u16(),
                );
                if day > days {
                    day = day.wrapping_sub(days);
                    month = month.wrapping_add(1);
                    if month >= 12 {
                        month = month.wrapping_sub(12);
                    }
                }
            }
            if day == calendar_day && month == calendar_month {
                let step_hour = truncate(e, x87_load32(hour_bits));
                let now = e.call(CALENDAR_HOUR, &args![CALENDAR]).f64();
                if step_hour >= truncate(e, now) {
                    day = 0;
                }
            }
        }
    });

    if out == 0 {
        return;
    }
    sort_by_schedule(e, out);
}

/// The missed packages' bubble sort of `out`.
fn sort_by_schedule(e: &mut Engine, out: u32) {
    let mut count = 0u32;
    let mut node = out;
    while node != 0 {
        if item_of(e, node) != 0 {
            count = count.wrapping_add(1);
        }
        node = next_of(e, node);
    }
    let mut outer = out;
    let mut swapped = true;
    while count != 0 && swapped {
        swapped = false;
        let mut node = outer;
        while node != 0 {
            let mine = item_of(e, outer);
            let theirs = item_of(e, node);
            let schedule = at(theirs, SCHEDULE);
            if typed_call!(
                FAMILY,
                0x0067_8a80,
                super::schedule::compare(e, mine, schedule)
            ) == 1
            {
                // `00726c60` on each node: the word stored when it is not zero.
                if theirs != 0 {
                    e.mem.set_u32(outer, theirs);
                }
                if mine != 0 {
                    e.mem.set_u32(node, mine);
                }
                swapped = true;
            }
            node = next_of(e, node);
        }
        outer = next_of(e, outer);
        count = count.wrapping_sub(1);
    }
}
