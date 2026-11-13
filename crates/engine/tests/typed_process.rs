//! The differential test of every typed function of the process family (`engine_typed::process`)
//! against its translation. The world: the process lists with their process array (at +4) of four
//! levels, the actors in it (objects with vtables whose slots log and answer a word, form flags,
//! the byte at +0x87, a process object with its level word, hour and own vtable), the game hour, the
//! performance counter and the time budget.
//!
//! What stands in: `Calendar::GetHour`, the performance counter, `LowProcess::ComputeLastTimeProcessed`,
//! the process type, `TESObjectREFR::RunScript` and the other actor calls, the objects' slots. The
//! actor list the passes collect in, the heap and the 64-bit multiply and divide run translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::process::{self, array, levels};
use engine_typed::{dispatch, multibound};

const VT_FN: u32 = 0x00ff_2000;
const VT_SLOTS: u32 = 0x400;
const LOG: u32 = 0x0300_0000;
const MAX_CALLS: usize = 20_000;

const ONE: u32 = 0x3f80_0000;
const MINUS_ONE: u32 = 0xbf80_0000;

/// Where the stand-ins find what they answer: the game hour (a float), the counter (eight bytes)
/// and its step (eight bytes).
const HOUR: u32 = LOG + 0x3e00;
const COUNTER: u32 = LOG + 0x3e10;
const STEP: u32 = LOG + 0x3e18;
/// The low pass's saved index, the updates-disabled byte, the budget setting and the frequency.
const SAVED_INDEX: u32 = 0x011e_0e30;
const UPDATES_DISABLED: u32 = 0x011e_0e34;
const BUDGET: u32 = 0x011e_0e68;
const FREQUENCY: u32 = 0x011e_0e20;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn ret_f(st0: f64) -> Ret {
    Ret {
        st0,
        ..Ret::default()
    }
}

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    if n >= 0x7c0 {
        return;
    }
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

fn words(e: &mut Engine, at: u32, w: &[u32]) {
    for (i, v) in (0u32..).zip(w) {
        e.mem.set_u32(at + 4 * i, *v);
    }
}

fn block(e: &mut Engine, size: u32, w: &[u32]) -> u32 {
    let a = e.mem.alloc(size.max(4 * w.len() as u32));
    words(e, a, w);
    a
}

/// A stand-in that logs its words under `TAG` and answers the word at `LOG + 0x3f00 + 4 * (TAG &
/// 0x3f)`.
fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1_0000, *w);
    }
    ret(e.mem.u32(LOG + 0x3f00 + 4 * (TAG & 0x3f)))
}

fn set_logged_answer(e: &mut Engine, tag: u32, answer: u32) {
    e.mem.set_u32(LOG + 0x3f00 + 4 * (tag & 0x3f), answer);
}

/// The stand-in for any slot: logs `this`, the slot and its words, answers the object's answer word
/// (+0xf8 of the vtable block: the vtable's answer).
fn any_slot<const SLOT: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a.first().copied().unwrap_or(0);
    log(e, 0xff00 | SLOT, this);
    for w in a.iter().skip(1) {
        log(e, 0xfffe, *w);
    }
    let vt = e.mem.u32(this);
    ret(e.mem.u32(vt + VT_SLOTS))
}

/// An object of `size` bytes with its own vtable of stand-in slots answering `answer`.
fn object(e: &mut Engine, size: u32, answer: u32) -> u32 {
    let vt = e.mem.alloc(VT_SLOTS + 0x40);
    for slot in (0..VT_SLOTS).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    e.mem.set_u32(vt + VT_SLOTS, answer);
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    o
}

/// Registers [`any_slot`] at every slot address (one function per slot, so the log names it).
fn slots(e: &mut Engine) {
    macro_rules! each {
        ($($s:literal)*) => { $( e.register(VT_FN + $s, any_slot::<$s>); )* };
    }
    each!(0x0 0x10 0x28 0x100 0x1ec 0x25c 0x2ec);
    for s in (0..VT_SLOTS).step_by(4) {
        if ![0x0, 0x10, 0x28, 0x100, 0x1ec, 0x25c, 0x2ec].contains(&s) {
            e.register(VT_FN + s, any_slot::<0xff>);
        }
    }
}

/// The exe's data and the stand-ins.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [
        0x0101_2000,
        0x0102_9000,
        0x0107_0000,
        0x011d_e000,
        0x011e_0000,
        0x011f_1000,
        0x0120_2000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    e.mem.set_u64(0x0102_90d0, 0.3f64.to_bits());
    e.mem.set_u64(0x0107_0c80, 0.15f64.to_bits());
    e.mem.set_u64(0x0101_a6b0, (-1.0f64).to_bits());
    // The lock's imports: the interlocked compare-exchange and the thread id.
    e.register(0x00fd_f1bc, |e, a| {
        let old = e.mem.u32(a[0]);
        if old == a[2] {
            e.mem.set_u32(a[0], a[1]);
        }
        ret(old)
    });
    e.register(0x00fd_f1d0, |_, _| ret(0x1234));
    // `QueryPerformanceCounter`: the counter moved on by its step, written to the word given.
    e.register(0x00fd_f0a0, |e, a| {
        let now = e.mem.u64(COUNTER).wrapping_add(e.mem.u64(STEP));
        e.mem.set_u64(COUNTER, now);
        e.mem.set_u64(a[0], now);
        log(e, 0xf0a0, now as u32);
        ret(1)
    });
    // `Calendar::GetHour`: the hour float.
    e.register(0x0086_7da0, |e, a| {
        log(e, 0x7da0, a[0]);
        ret_f(f64::from(e.mem.f32(HOUR)))
    });
    // The process type of an actor: its word +0xf0.
    e.register(0x0093_1850, |e, a| {
        log(e, 0x1850, a[0]);
        ret(e.mem.u32(a[0] + 0xf0))
    });
    e.register(0x0090_7650, logged::<0x01>);
    e.register(0x0092_f190, logged::<0x02>);
    e.register(0x005a_c740, logged::<0x03>);
    e.register(0x0056_5870, logged::<0x04>);
    e.register(0x004f_8960, logged::<0x05>);
    e.register(0x0049_3bb0, logged::<0x06>);
    e.register(0x0040_7800, logged::<0x07>);
    slots(e);
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&process::PROCESS)
            .family(&dispatch::DISPATCH)
            .family(&multibound::MULTIBOUND)
            .family(&leaf::LEAVES)
            // The translation's 64-bit multiply and divide of the time budget, which the typed pass
            // computes natively.
            .ignore_calls(&[0x00ec_c300, 0x00ec_c250])
            .max_calls(MAX_CALLS)
    }
}

/// A process array of `levels` (each level's actors, in order) with `spare` empty slots after them,
/// its cursors from `cursors`.
fn process_array(e: &mut Engine, levels: [&[u32]; 4], spare: u32, cursors: [u32; 4]) -> u32 {
    let a = object(e, 0x40, 0);
    let count: u32 = levels.iter().map(|l| l.len() as u32).sum::<u32>() + spare;
    let elements = e.mem.alloc(4 * count.max(1));
    let mut index = 0u32;
    for (level, actors) in (0u32..).zip(levels) {
        e.mem.set_u32(a + 0x10 + 4 * level, index);
        for &actor in actors {
            e.mem.set_u32(elements + 4 * index, actor);
            index += 1;
        }
        e.mem.set_u32(a + 0x20 + 4 * level, index);
        e.mem.set_u32(a + 0x30 + 4 * level, cursors[level as usize]);
    }
    e.mem.set_u32(a + 4, elements);
    e.mem.set_u16(a + 8, count as u16);
    e.mem.set_u16(a + 0xa, index as u16);
    let non_null = levels
        .iter()
        .flat_map(|l| l.iter())
        .filter(|&&x| x != 0)
        .count();
    e.mem.set_u16(a + 0xc, non_null as u16);
    a
}

/// Simple actor stand-ins for the array tests: distinct words, some repeated.
fn ids(n: u32, k: u32) -> Vec<u32> {
    (0..n)
        .map(|i| {
            if (i + k) % 7 == 3 {
                0
            } else {
                0x0400_0000 + 0x10 * ((i * 3 + k) % 11)
            }
        })
        .collect()
}

/// The array of an array test from `w` = [the levels' sizes pattern, the actors' pattern, the
/// cursors' pattern].
fn array_world(e: &mut Engine, pattern: u32, k: u32, cursors: u32) -> u32 {
    let sizes: [u32; 4] = match pattern {
        0 => [2, 3, 4, 5],
        1 => [0, 1, 0, 6],
        2 => [5, 0, 0, 0],
        _ => [0, 0, 0, 0],
    };
    let all = ids(sizes.iter().sum(), k);
    let mut start = 0usize;
    let mut parts: Vec<&[u32]> = Vec::new();
    for s in sizes {
        parts.push(&all[start..start + s as usize]);
        start += s as usize;
    }
    let c = match cursors {
        0 => [0, 0, 0, 0],
        1 => [1, 3, 9, 13],
        _ => [7, 0, 40, 2],
    };
    process_array(e, [parts[0], parts[1], parts[2], parts[3]], 2, c)
}

#[test]
fn process_array_getters() {
    // A level's head and tail, each level and one past (which reads the next table's word).
    let head_or_tail = |d: Diff| {
        d.common()
            .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2, 3, 4])])
            .setup(|e, case| {
                stand_ins(e);
                vec![array_world(e, case.word(0), 1, 1), case.word(1)]
            })
            .run()
            .assert_same();
    };
    head_or_tail(Diff::new(entry!(
        0x005d_e0f0,
        array::level_head(Ptr<_>, u32) -> u32
    )));
    head_or_tail(Diff::new(entry!(
        0x005b_e5c0,
        array::level_tail(Ptr<_>, u32) -> u32
    )));
    // The actor at each index, an empty slot among them.
    Diff::new(entry!(0x0096_8670, array::actor_at(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 3, 5, 13])])
        .setup(|e, case| {
            stand_ins(e);
            vec![array_world(e, case.word(0), 2, 0), case.word(1)]
        })
        .run()
        .assert_same();
    // The element store: below the count over a null or non-null element, at and past the count; a
    // null or non-null value.
    Diff::new(entry!(0x0096_ae90, array::set_element(Ptr<_>, u32, Ptr<_>)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1]),
            Input::Of(&[0, 3, 10, 14, 15]),
            Input::Of(&[0, 0x0400_0040]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            let a = array_world(e, case.word(0), 0, 0);
            let value = block(e, 4, &[case.word(2)]);
            vec![a, case.word(1), value]
        })
        .run()
        .assert_same();
    // The level of each index, none past the last level.
    Diff::new(entry!(0x0096_ab20, array::level_of(Ptr<_>, u32) -> i32))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2, 4, 5, 9, 13, 14, 20]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            vec![array_world(e, case.word(0), 0, 0), case.word(1)]
        })
        .run()
        .assert_same();
    // The cursors corrected: inside, past and before their levels; empty levels.
    Diff::new(entry!(0x0096_ac30, array::correct_cursors(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            stand_ins(e);
            vec![array_world(e, case.word(0), 0, case.word(1))]
        })
        .run()
        .assert_same();
}

#[test]
fn process_array_find() {
    // Each actor of the pattern (its first place found from each level's start, or none), a null
    // actor, one not in the array; each level and the signed index -1.
    Diff::new(entry!(0x0096_abc0, array::find(Ptr<_>, u32, i32) -> u32))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[
                0,
                0x0400_0000,
                0x0400_0030,
                0x0400_00a0,
                0x0400_0050,
                0x0500_0000,
            ]),
            Input::Of(&[0, 1, 2, 3, 0xffff_ffff]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            let a = array_world(e, case.word(0), 1, 0);
            // The actor not in the array sits in the first spare slot past the last level's end,
            // where the search must not look.
            let elements = e.mem.u32(a + 4);
            let end = e.mem.u32(a + 0x2c);
            e.mem.set_u32(elements + 4 * end, 0x0500_0000);
            vec![a, case.word(1), case.word(2)]
        })
        .run()
        .assert_same();
    // A long array whose element block crosses page ends (the typed search reads a page at a time):
    // the actor before, at and after the first page end, at the end, not there, and the block running
    // into an unmapped page with no match (both stop there).
    Diff::new(entry!(0x0096_abc0, array::find(Ptr<_>, u32, i32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5])])
        .expect_stops(|case| case.word(0) == 5)
        .setup(|e, case| {
            stand_ins(e);
            e.map(0x0500_0000, 0x3000);
            let a = object(e, 0x40, 0);
            // 1,100 elements from 0x0500_0ff8: indexes 0 and 1 in the first page, 2.. in the next two.
            let elements = 0x0500_0ff8;
            let n = if case.word(0) == 5 { 4_000 } else { 1_100 };
            for i in 0..1_100u32.min((0x0500_3000 - elements) / 4) {
                e.mem.set_u32(elements + 4 * i, 0x0600_0000 + i);
            }
            words(e, a + 0x10, &[0, 0, 0, 0, 0, 0, 0, n]);
            e.mem.set_u32(a + 4, elements);
            let actor = match case.word(0) {
                0 => 0x0600_0001,
                1 => 0x0600_0002,
                2 => 0x0600_0003,
                3 => 0x0600_0000 + 1_099,
                _ => 0x0700_0000,
            };
            vec![a, actor, 3]
        })
        .run()
        .assert_same();
}

#[test]
fn process_array_remove() {
    // The element at each index removed (its level found or none), the cursors corrected after.
    Diff::new(entry!(0x0096_aa40, array::remove_at(Ptr<_>, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2, 4, 5, 9, 13, 14, 0xffff_ffff]),
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            vec![array_world(e, case.word(0), 1, case.word(2)), case.word(1)]
        })
        .run()
        .assert_same();
    // An actor removed from a level: found in it, found in a later level, not there, null.
    Diff::new(entry!(0x0096_aaf0, array::remove_actor(Ptr<_>, u32, i32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 0x0400_0000, 0x0400_0030, 0x0400_00a0, 0x0500_0000]),
            Input::Of(&[0, 1, 2, 3]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            vec![
                array_world(e, case.word(0), 1, 1),
                case.word(1),
                case.word(2),
            ]
        })
        .run()
        .assert_same();
}

#[test]
fn process_hour_last_processed() {
    // A number, -1, a quiet and a signalling NaN.
    Diff::new(entry!(
        0x007d_f1f0,
        levels::hour_last_processed(Ptr<_>) -> f32
    ))
    .common()
    .inputs(&[Input::Of(&[ONE, MINUS_ONE, 0x7fc0_0000, 0x7f80_0001])])
    .setup(|e, case| {
        stand_ins(e);
        vec![block(e, 0x30, &[0, 0, 0, 0, 0, 0, 0, 0, case.word(0)])]
    })
    .run()
    .assert_same();
}

/// One actor of the passes' world from its kind (the bits as below) for `level`, with the process's
/// last hour `last`:
/// - kind 0: null; 1: form flag 0x200000 (skipped); 2: form flag 0x800 (collected; loses its
///   process); 3: form flag 0x20 (collected); 4: another level (collected); 5..: an actor of the
///   level: (kind - 5) bit 0 the byte at +0x87, bit 1 its slots answer true, bit 2 its process's
///   slots answer true, bit 3 no process object.
fn actor(e: &mut Engine, kind: u32, level: u32, last: u32) -> u32 {
    if kind == 0 {
        return 0;
    }
    let bits = kind.saturating_sub(5);
    let a = object(e, 0x100, (bits >> 1) & 1);
    let flags = match kind {
        1 => 0x0020_0000,
        2 => 0x800,
        3 => 0x20,
        _ => 0,
    };
    e.mem.set_u32(a + 8, flags);
    e.mem.set_u8(a + 0x87, (bits & 1) as u8);
    let own_level = if kind == 4 { level ^ 1 } else { level };
    e.mem.set_u32(a + 0xf0, own_level);
    if bits & 8 == 0 || kind < 5 {
        let p = object(e, 0x40, (bits >> 2) & 1);
        e.mem.set_u32(p + 0x28, own_level);
        e.mem.set_u32(p + 0x20, last);
        e.mem.set_u32(a + 0x68, p);
    }
    a
}

/// The process lists of a pass case: `n` actors of `level` from the kinds pattern `k`, the last
/// hours from `lasts`, between some actors of the other levels.
fn lists_world(e: &mut Engine, level: u32, n: u32, k: u32, lasts: &[u32]) -> u32 {
    let actors: Vec<u32> = (0..n)
        .map(|i| {
            let kind = (i * 5 + k) % 21;
            actor(
                e,
                kind,
                level,
                lasts[(i as usize + k as usize) % lasts.len()],
            )
        })
        .collect();
    let others: Vec<u32> = (0..2).map(|i| actor(e, 5 + i, level ^ 2, ONE)).collect();
    let mut levels: [&[u32]; 4] = [&others, &[], &[], &[]];
    levels[level as usize] = &actors;
    let a = process_array(e, levels, 4, [0; 4]);
    let lists = block(e, 0x100, &[]);
    let bytes = e.mem.bytes(a, 0x40);
    e.mem.write(lists + 4, &bytes);
    lists
}

/// The game hour and last hours the pass cases try: the hour; then the last hours (-1, the hour
/// itself, an hour before, more than an interval before, after it).
const HOURS: [u32; 3] = [0x4140_0000, 0x3f80_0000, 0x7fc0_0000];
const LASTS: [u32; 5] = [
    MINUS_ONE,
    0x4130_0000,
    0x4138_0000,
    0x4100_0000,
    0x4150_0000,
];

#[test]
fn process_update_low() {
    // The actors' kinds pattern; the saved index (0, inside, past the end); `all`; updates disabled;
    // the hour; `time` (0, 1, NaN); the counter's step (no budget, or one that ends the walk early).
    Diff::new(entry!(0x0096_b050, levels::update_low(Ptr<_>, f32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 3, 7]),
            Input::Of(&[0, 5, 40]),
            Input::Flag,
            Input::Flag,
            Input::Of(&HOURS),
            Input::Of(&[0, ONE, 0x7fc0_0000]),
            Input::Of(&[0, 1]),
        ])
        .max_cases(256)
        .setup(|e, case| {
            stand_ins(e);
            let lists = lists_world(e, 3, 23, case.word(0), &LASTS);
            let begin = e.mem.u32(lists + 4 + 0x1c);
            let saved = match case.word(1) {
                0 => 0,
                5 => begin + 5,
                _ => 40,
            };
            e.mem.set_u32(SAVED_INDEX, saved);
            e.mem.set_u8(UPDATES_DISABLED, case.word(3) as u8);
            e.mem.set_u32(HOUR, case.word(4));
            // A budget of 3 ms at a frequency of 1,000,000: 3,000 ticks; a step of 1,000 ends the
            // walk at the third check.
            e.mem.set_u32(BUDGET, 3);
            e.mem.set_u64(FREQUENCY, 1_000_000);
            e.mem
                .set_u64(STEP, if case.word(6) == 0 { 0 } else { 1_000 });
            set_logged_answer(e, 0x03, 1);
            vec![lists, case.word(5), case.word(2)]
        })
        .run()
        .assert_same();
}

/// The middle passes' case setup: `level`'s actors from the pattern, updates disabled, the hour.
fn middle_setup(e: &mut Engine, case: &Case, level: u32) -> Vec<u32> {
    stand_ins(e);
    let lists = lists_world(e, level, 23, case.word(0), &LASTS);
    e.mem.set_u8(UPDATES_DISABLED, case.word(1) as u8);
    e.mem.set_u32(HOUR, case.word(2));
    set_logged_answer(e, 0x06, case.word(4));
    vec![lists, case.word(3), 0x55]
}

/// The middle passes' inputs: the kinds pattern, updates disabled, the hour, `time`, `00493bb0`'s
/// answer.
fn middle_inputs() -> Vec<Input> {
    vec![
        Input::Of(&[0, 3, 7]),
        Input::Flag,
        Input::Of(&HOURS),
        Input::Of(&[0, ONE, 0x7fc0_0000]),
        Input::Flag,
    ]
}

#[test]
fn process_update_middle_low() {
    Diff::new(entry!(
        0x0096_b470,
        levels::update_middle_low(Ptr<_>, f32, u32)
    ))
    .common()
    .inputs(&middle_inputs())
    .max_cases(256)
    .setup(|e, case| middle_setup(e, case, 2))
    .run()
    .assert_same();
}

#[test]
fn process_update_middle_high() {
    Diff::new(entry!(
        0x0096_b810,
        levels::update_middle_high(Ptr<_>, f32, u32)
    ))
    .common()
    .inputs(&middle_inputs())
    .max_cases(256)
    .setup(|e, case| middle_setup(e, case, 1))
    .run()
    .assert_same();
}

#[test]
fn process_hour_rounding() {
    // One actor due by the hours: the hour and last hour whose difference, one float subtraction at
    // the main thread's precision, reaches the pass's interval while the exact difference falls
    // short (1.0: 0x3f800000 - 0x2edbe6ff; 0.3: 0x3e99999a - 0x324e288f; 0.15: 0x3e19999a -
    // 0x31ce288f), so the hours computed in double precision miss the update.
    for (level, hour, last) in [
        (3u32, 0x3f80_0000u32, 0x2edb_e6ffu32),
        (2, 0x3e99_999a, 0x324e_288f),
        (1, 0x3e19_999a, 0x31ce_288f),
    ] {
        let f = match level {
            3 => entry!(0x0096_b050, levels::update_low(Ptr<_>, f32, u32)),
            2 => entry!(0x0096_b470, levels::update_middle_low(Ptr<_>, f32, u32)),
            _ => entry!(0x0096_b810, levels::update_middle_high(Ptr<_>, f32, u32)),
        };
        Diff::new(f)
            .common()
            .inputs(&[Input::Of(&[0])])
            .setup(move |e, _| {
                stand_ins(e);
                // An actor of the level with the byte at +0x87, its process's slots true.
                let a = actor(e, 5 + 1 + 4, level, last);
                let lists = block(e, 0x100, &[]);
                let arr = process_array(
                    e,
                    match level {
                        3 => [&[], &[], &[], &[a]],
                        2 => [&[], &[], &[a], &[]],
                        _ => [&[], &[a], &[], &[]],
                    },
                    1,
                    [0; 4],
                );
                let bytes = e.mem.bytes(arr, 0x40);
                e.mem.write(lists + 4, &bytes);
                e.mem.set_u32(HOUR, hour);
                set_logged_answer(e, 0x03, 0);
                vec![lists, 0, 1]
            })
            .run()
            .assert_same();
    }
}
