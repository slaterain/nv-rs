//! A package's schedule (`PSDT`, the 8 bytes at +0x38 of `TESPackage`) and the functions of the
//! units around it that read and write one (`run_00678a80`, `run_00678cc0`, `run_0067fcb0`), each
//! as the exe does it, with the three-word swap that sits in the same unit.
//!
//! The schedule is the month (+0, a signed byte, -1 for any), the day of the week (+1, -1 for
//! any), the date (+2, 0 for any), the hour (+3, -1 for any) and the duration in hours (the word
//! at +4).
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): the package's schedule (`0041d8a0`, +0x38), the schedule's fields
//! (`00678ca0` the month and `0086a460` the hour, besides this unit's own day of the week and
//! date), its setters (`005407b0` the month, this unit's own for the other three bytes, and
//! `006ecd40` the word at +4), the duration (`00726070`, the word at +4), the big-endian byte
//! (`00401500`, `011c54ba`) and the word swaps (`0060ce80` on the word at +4, `00401080`, over the
//! runtime's `_byteswap_ulong`). The chunk's write is this family's typed call. Every byte the
//! exe compares through `MOVSX` is compared as the signed byte it is.
//!
//! No floats.

use super::{tesform, FAMILY};
use engine_core::typed_call;
use engine_core::Engine;

/// A package's schedule.
const SCHEDULE: u32 = 0x38;
const MONTH: u32 = 0;
const DAY_OF_WEEK: u32 = 1;
const DATE: u32 = 2;
const HOUR: u32 = 3;
const DURATION: u32 = 4;
/// The record's byte order flag (the files are big-endian).
const BIG_ENDIAN: u32 = 0x011c_54ba;
/// The chunk ("PSDT") and its size.
const CHUNK_SCHEDULE: u32 = 0x5444_5350;
const SCHEDULE_SIZE: u32 = 8;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

fn field(e: &Engine, schedule: u32, offset: u32) -> i8 {
    e.mem.i8(at(schedule, offset))
}

fn swap_word(e: &mut Engine, at: u32) {
    let word = e.mem.u32(at);
    e.mem.set_u32(at, word.swap_bytes());
}

// Typed from 00678a80
/// The order of two packages' schedules (`this` a package, `other` a schedule): by the month, the
/// date, the day of the week and the hour in turn, the first that decides answers. A field
/// decides unless both are "any" (for the date: this one 0, the other -1) or they are equal: this
/// one "any" (-1) answers 1, the other "any" -1, else -1 when this one is lower and 1 otherwise.
/// 0 when none decides.
pub fn compare(e: &mut Engine, this: u32, other: u32) -> i32 {
    let mine = at(this, SCHEDULE);
    for (offset, unset) in [(MONTH, -1), (DATE, 0), (DAY_OF_WEEK, -1), (HOUR, -1)] {
        let (a, b) = (field(e, mine, offset), field(e, other, offset));
        if (a == unset && b == -1) || a == b {
            continue;
        }
        return if a == -1 || (b != -1 && a >= b) {
            1
        } else {
            -1
        };
    }
    0
}

// Typed from 00678cc0
/// The schedule's day of the week (+1, signed).
pub fn day_of_week(e: &mut Engine, this: u32) -> i8 {
    field(e, this, DAY_OF_WEEK)
}

// Typed from 00678ce0
/// The schedule's date (+2).
pub fn date(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(at(this, DATE))
}

// Typed from 0067fcb0
/// The schedule's constructor: any month, day of the week and hour, date 0, no duration. Returns
/// `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u8(at(this, MONTH), 0xff);
    e.mem.set_u8(at(this, DAY_OF_WEEK), 0xff);
    e.mem.set_u8(at(this, DATE), 0);
    e.mem.set_u8(at(this, HOUR), 0xff);
    e.mem.set_u32(at(this, DURATION), 0);
    this
}

// Typed from 0067fcf0
/// The schedule's assignment from `source`: its four bytes one by one, then the duration. Returns
/// `this`.
pub fn assign(e: &mut Engine, this: u32, source: u32) -> u32 {
    for offset in [MONTH, DAY_OF_WEEK, DATE, HOUR] {
        let byte = e.mem.u8(at(source, offset));
        e.mem.set_u8(at(this, offset), byte);
    }
    let duration = e.mem.u32(at(source, DURATION));
    e.mem.set_u32(at(this, DURATION), duration);
    this
}

// Typed from 0067fd40
/// The schedule set from `source` (nothing for a null one), as the exe's setters do it: the day
/// of the week, the month, the date, the hour, then the duration.
pub fn set_from(e: &mut Engine, this: u32, source: u32) {
    if source == 0 {
        return;
    }
    for offset in [DAY_OF_WEEK, MONTH, DATE, HOUR] {
        let byte = e.mem.u8(at(source, offset));
        e.mem.set_u8(at(this, offset), byte);
    }
    let duration = e.mem.u32(at(source, DURATION));
    e.mem.set_u32(at(this, DURATION), duration);
}

// Typed from 0067fdb0
/// The schedule's day of the week set.
pub fn set_day_of_week(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(at(this, DAY_OF_WEEK), value);
}

// Typed from 0067fdd0
/// The schedule's date set.
pub fn set_date(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(at(this, DATE), value);
}

// Typed from 0067fdf0
/// The schedule's hour set.
pub fn set_hour(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(at(this, HOUR), value);
}

// Typed from 0067fe10
/// Whether this schedule differs from `other` (true for a null one): the day of the week, the
/// month, the date, the hour and the duration compared in that order.
pub fn differs(e: &mut Engine, this: u32, other: u32) -> bool {
    if other == 0 {
        return true;
    }
    [DAY_OF_WEEK, MONTH, DATE, HOUR]
        .into_iter()
        .any(|offset| field(e, this, offset) != field(e, other, offset))
        || e.mem.u32(at(this, DURATION)) != e.mem.u32(at(other, DURATION))
}

// Typed from 0067fec0
/// The schedule written as its `PSDT` chunk, its duration byte-swapped around the write when the
/// records are big-endian.
pub fn save(e: &mut Engine, this: u32) {
    if e.mem.u8(BIG_ENDIAN) != 0 {
        swap_word(e, at(this, DURATION));
    }
    typed_call!(
        FAMILY,
        0x0048_5990,
        tesform::add_chunk_data(e, CHUNK_SCHEDULE, this, SCHEDULE_SIZE)
    );
    if e.mem.u8(BIG_ENDIAN) != 0 {
        swap_word(e, at(this, DURATION));
    }
}

// Typed from 0067ff10
/// The three words at +4, +8 and +0xc byte-swapped (`00401080` each; the weapon's save and load
/// swap their chunk with it).
pub fn swap_three_words(e: &mut Engine, this: u32) {
    for offset in [4, 8, 0xc] {
        swap_word(e, at(this, offset));
    }
}
