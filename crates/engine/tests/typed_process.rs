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
use engine_typed::process::{self, array, forget, levels};
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
        0x0101_a000,
        0x0102_9000,
        0x0107_0000,
        0x011a_3000,
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
    // The game's hour and date, which `LowProcess::ComputeLastTimeProcessed` copies.
    e.mem.set_f32(0x011e_0358, 13.5);
    e.mem.set_u32(0x011a_39c4, 4);
    e.mem.set_u8(0x011e_0360, 19);
    e.mem.set_u32(0x011e_035c, 2281);
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
    // `operator new` and `operator delete` (the actor list's nodes).
    e.register(0x0040_1000, |e, a| {
        log(e, 0x1000, a[0]);
        ret(e.mem.alloc(a[0].max(4)))
    });
    e.register(0x0040_1030, logged::<0x08>);
    e.register(0x0092_f190, logged::<0x02>);
    e.register(0x005a_c740, logged::<0x03>);
    e.register(0x0056_5870, logged::<0x04>);
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
            // The AI queries family's functions the clean-up and the passes call: by address here,
            // to the stand-ins these worlds give them; their own differential tests are in
            // `typed_ai_queries.rs`.
            .goes_by_address(&[
                0x0041_ca90,
                0x0049_3bb0,
                0x0055_b980,
                0x0067_f390,
                0x0093_44a0,
                0x008a_6650,
                0x0088_15a0,
            ])
            // The translation's 64-bit multiply and divide of the time budget, which the typed pass
            // computes natively.
            .ignore_calls(&[0x00ec_c300, 0x00ec_c250])
            // The element address the exe asks for each element its search reads; the typed search
            // reads the element block a page at a time.
            .ignore_calls(&[0x0087_7a30])
            // The character's word +0x108 the exe reads into a flag it never reads, which the typed
            // actor update leaves out.
            .ignore_calls(&[0x004f_8960])
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
    Diff::new(entry!(0x005d_e0f0, array::level_head(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2, 3, 4])])
        .setup(|e, case| {
            stand_ins(e);
            vec![array_world(e, case.word(0), 1, 1), case.word(1)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x005b_e5c0, array::level_tail(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2, 3, 4])])
        .setup(|e, case| {
            stand_ins(e);
            vec![array_world(e, case.word(0), 1, 1), case.word(1)]
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
    // into an unmapped page with no match (both stop there), or with a match in its last mapped page:
    // at index 2,000 and at index 2,049, the last mapped word (0x0500_2ffc).
    Diff::new(entry!(0x0096_abc0, array::find(Ptr<_>, u32, i32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7])])
        .expect_stops(|case| case.word(0) == 5)
        .setup(|e, case| {
            stand_ins(e);
            e.map(0x0500_0000, 0x3000);
            let a = object(e, 0x40, 0);
            // Elements from 0x0500_0ff8 to the end of the mapping (2,050 of them): indexes 0 and 1 in
            // the first page, 2.. in the next two; the array 1,100 long, or 4,000 (past the mapping).
            let elements = 0x0500_0ff8;
            let n = if case.word(0) >= 5 { 4_000 } else { 1_100 };
            for i in 0..(0x0500_3000 - elements) / 4 {
                e.mem.set_u32(elements + 4 * i, 0x0600_0000 + i);
            }
            words(e, a + 0x10, &[0, 0, 0, 0, 0, 0, 0, n]);
            e.mem.set_u32(a + 4, elements);
            let actor = match case.word(0) {
                0 => 0x0600_0001,
                1 => 0x0600_0002,
                2 => 0x0600_0003,
                3 => 0x0600_0000 + 1_099,
                6 => 0x0600_0000 + 2_000,
                7 => 0x0600_0000 + 2_049,
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
fn actor(e: &mut Engine, kind: u32, level: u32, last: u32, collected_answer: u32) -> u32 {
    if kind == 0 {
        return 0;
    }
    let bits = kind.saturating_sub(5);
    // A collected actor's slots (0x2ec among them, which the middle-high removal asks) answer
    // `collected_answer`.
    let answer = if (2..5).contains(&kind) {
        collected_answer
    } else {
        (bits >> 1) & 1
    };
    let a = object(e, 0x100, answer);
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
    // Only the low process has actors without a process object (the middle ones always have one).
    if bits & 8 == 0 || kind < 5 || level != 3 {
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
                i % 2,
            )
        })
        .collect();
    let others: Vec<u32> = (0..2).map(|i| actor(e, 5 + i, level ^ 2, ONE, 0)).collect();
    let mut levels: [&[u32]; 4] = [&others, &[], &[], &[]];
    levels[level as usize] = &actors;
    let a = process_array(e, levels, 4, [0; 4]);
    let lists = block(e, 0x100, &[]);
    let bytes = e.mem.bytes(a, 0x40);
    e.mem.write(lists + 4, &bytes);
    lists
}

/// The game hour and last hours the pass cases try: the hour (12.0, 1.0, NaN); then the last hours
/// (-1; with the hour 12.0: an hour before, half an hour, more than an hour, after it, the hour
/// itself, 0.2 before (between the middle passes' 0.15 and 0.3) and 0.1 before (below both)).
const HOURS: [u32; 3] = [0x4140_0000, 0x3f80_0000, 0x7fc0_0000];
const LASTS: [u32; 8] = [
    MINUS_ONE,
    0x4130_0000,
    0x4138_0000,
    0x4100_0000,
    0x4150_0000,
    0x4140_0000,
    0x413c_cccd,
    0x413e_6666,
];

#[test]
fn process_update_low() {
    // The actors' kinds pattern; the saved index (0, below the level's head, inside, past the end);
    // `all` (0, 1, and 0x100: only its low byte counts); updates disabled;
    // the hour; `time` (0, 1, NaN); the counter's step (no budget, or one that ends the walk early).
    Diff::new(entry!(0x0096_b050, levels::update_low(Ptr<_>, f32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 3, 7]),
            Input::Of(&[0, 1, 5, 40]),
            Input::Of(&[0, 1, 0x100]),
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
                // Below level 3's head (the level-0 actors before it): kept, as in the exe.
                1 => 1,
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
                let a = actor(e, 5 + 1 + 4, level, last, 0);
                let lists = block(e, 0x100, &[]);
                let one = [a];
                let arr = process_array(
                    e,
                    match level {
                        3 => [&[], &[], &[], &one],
                        2 => [&[], &[], &one, &[]],
                        _ => [&[], &one, &[], &[]],
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

// ---- The clean-up when a reference goes away (`0096f600`, `00976680`) ----

/// The exe functions the clean-up calls by address, each a stand-in that logs its words and answers
/// from [`FORGET_ANSWERS`] by its index here.
const FORGET_CALLEES: [u32; 38] = [
    0x0055_b980,
    0x0067_f390,
    0x0067_f3c0,
    0x0088_1650,
    0x0068_0020,
    0x0093_36c0,
    0x0093_44a0,
    0x0067_4d40,
    0x0088_1680,
    0x00c5_4550,
    0x0056_8ab0,
    0x0098_ddd0,
    0x0041_ca90,
    0x005e_3fc0,
    0x0099_4ef0,
    0x008a_6730,
    0x0057_bd60,
    0x008b_3200,
    0x0041_9dc0,
    0x008d_80e0,
    0x008a_6650,
    0x008a_6170,
    0x009f_8420,
    0x0041_cab0,
    0x0097_f9c0,
    0x0088_1620,
    0x0087_4480,
    0x005e_3fa0,
    0x0042_2690,
    0x0090_5330,
    0x0067_2fc0,
    0x0067_6140,
    0x0067_1d30,
    0x0088_1510,
    0x008c_4f10,
    0x0099_2920,
    0x009c_4c80,
    0x009b_28a0,
];
/// Where each callee's answer is.
const FORGET_ANSWERS: u32 = LOG + 0x3c00;
/// A slot that answers 0 (a non-actor's "is it an actor"), past the stand-in slots.
const NOT_AN_ACTOR: u32 = 0x00ff_3000;
/// The array slot `009c4c80`'s stand-in empties when it is called (0: none), then the array's
/// first element and the address of the slot's level's tail (the next two words).
const LEAVES_ITS_SLOT: u32 = LOG + 0x3e70;

fn forget_callee<const I: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x5000 | I, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, 0x5100 | I, *w);
    }
    ret(e.mem.u32(FORGET_ANSWERS + 4 * I))
}

fn forget_callees(e: &mut Engine) {
    e.register(NOT_AN_ACTOR, |e, a| {
        log(e, 0x3000, a.first().copied().unwrap_or(0));
        ret(0)
    });
    macro_rules! each {
        ($($i:literal)*) => { $( e.register(FORGET_CALLEES[$i], forget_callee::<$i>); )* };
    }
    each!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32
        33 34 35 36 37);
    // `009c4c80` (index 36): also takes the object out of the array slot [`LEAVES_ITS_SLOT`] names,
    // once, as the array's removal (`0096aa40`) does: the level's tail one lower, the element at the
    // new tail moved into the slot (unless the slot is that last one) and the new tail's slot emptied.
    e.register(0x009c_4c80, |e, a| {
        let slot = e.mem.u32(LEAVES_ITS_SLOT);
        if slot != 0 {
            let elements = e.mem.u32(LEAVES_ITS_SLOT + 4);
            let tail = e.mem.u32(LEAVES_ITS_SLOT + 8);
            let last_index = e.mem.u32(tail) - 1;
            e.mem.set_u32(tail, last_index);
            let last = elements + 4 * last_index;
            if slot != last {
                let moved = e.mem.u32(last);
                e.mem.set_u32(slot, moved);
            }
            e.mem.set_u32(last, 0);
            e.mem.set_u32(LEAVES_ITS_SLOT, 0);
        }
        forget_callee::<36>(e, a)
    });
}

/// An object of 0x200 bytes whose slots all answer `answer`, its extra data list (at +0x44) empty.
/// Its vtable goes on to slot 0x7fc (the clean-up calls slots up to 0x7b0): the slots past the
/// answer word at +0x400 are stand-ins too ([`forget_high_slots`]).
fn forget_object(e: &mut Engine, answer: u32) -> u32 {
    let vt = e.mem.alloc(0x800);
    for slot in (0..0x800).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    e.mem.set_u32(vt + VT_SLOTS, answer);
    let o = e.mem.alloc(0x200);
    e.mem.set_u32(o, vt);
    o
}

/// The stand-ins for the slots past the answer word.
fn forget_high_slots(e: &mut Engine) {
    for s in (VT_SLOTS + 4..0x800).step_by(4) {
        e.register(VT_FN + s, any_slot::<0xfd>);
    }
}

/// Gives `object`'s extra data list the extra data of `kinds` (a type and its words at +0xc and
/// +0x14), with the type bitmap set.
fn give_extra(e: &mut Engine, object: u32, kinds: &[(u32, u32, u32)]) {
    let list = object + 0x44;
    let mut next = 0;
    for &(kind, c, x14) in kinds.iter().rev() {
        let extra = block(e, 0x18, &[0x0101_0000, kind, next, c, 0, x14]);
        next = extra;
        let byte = list + 8 + (kind >> 3);
        let flags = e.mem.u8(byte) | 1 << (kind & 7);
        e.mem.set_u8(byte, flags);
    }
    e.mem.set_u32(list + 4, next);
}

/// An actor of the clean-up's world: its slots answer `slot_answer`, its word at +0x70 and +0xc0
/// the target or not (`k` bits 0 and 1), its process (unless `k` bit 2) with slots answering
/// `process_answer` and level `level`.
fn forget_actor(
    e: &mut Engine,
    target: u32,
    decoy: u32,
    (slot_answer, process_answer, level, k): (u32, u32, u32, u32),
) -> u32 {
    let a = forget_object(e, slot_answer);
    e.mem
        .set_u32(a + 0x70, if k & 1 != 0 { target } else { decoy });
    e.mem.set_u32(a + 0xc0, if k & 2 != 0 { target } else { 0 });
    if k & 4 == 0 {
        let p = forget_object(e, process_answer);
        e.mem.set_u32(p + 0x28, level);
        e.mem.set_u32(a + 0x68, p);
    }
    a
}

/// The clean-up's world from the case: `pattern` picks which objects stand in the array and what
/// their slots answer; `answers` which callees answer 0, a decoy object or the target; `which` the
/// target (an actor, a non-actor, or none). Returns the lists and the target.
fn forget_world(e: &mut Engine, pattern: u32, answers: u32, which: u32) -> (u32, u32) {
    stand_ins(e);
    forget_callees(e);
    forget_high_slots(e);
    // The extra data's dirty count and lock.
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(0x011c_38e4, 1);
    // A decoy whose slots answer 0: the "something, not the target" answer, its low byte not 0 so
    // that it is true as a byte too.
    let base = forget_object(e, 0);
    let decoy = base + 0x11;
    let vt = e.mem.u32(base);
    e.mem.set_u32(decoy, vt);
    // The target: an actor (its slots answer itself) or a non-actor (they answer 0).
    // Placed off its block's start, as the decoy is, so its address's low byte (what a slot's
    // answer is as a byte) is not 0 whatever the heap gives: as an actor, its "is it an actor" is true.
    let target_base = forget_object(e, 0);
    let target = target_base + 0x21;
    let vt = e.mem.u32(target_base);
    e.mem.set_u32(target, vt);
    assert_ne!(target & 0xff, 0, "the target's answer is true as a byte");
    if which == 0 {
        e.mem.set_u32(vt + VT_SLOTS, target);
    }
    for i in 0..FORGET_CALLEES.len() as u32 {
        let pick = (answers >> ((i * 7) % 15)) & 3;
        let answer = match pick {
            1 => decoy,
            2 | 3 => target,
            _ => 0,
        };
        e.mem.set_u32(FORGET_ANSWERS + 4 * i, answer);
    }
    // The decoy's word a record's +0x18 is read from.
    e.mem
        .set_u32(decoy + 0x18, if answers & 1 != 0 { target } else { 0 });
    let p_answer = if pattern & 1 != 0 { target } else { decoy };
    let s_answer = if pattern & 2 != 0 { target } else { decoy };
    let follower = forget_actor(e, target, decoy, (decoy, p_answer, 3, 1));
    let lead = forget_actor(
        e,
        target,
        decoy,
        (s_answer, p_answer, pattern % 3, pattern >> 2),
    );
    // With pattern bit 32 the lead's slot 0x2c8 is the actor classes' own (`008815a0`): the
    // clean-up's current-target test takes its typed branch (the call itself by address here, to
    // the translation; its direct run is in `typed_ai_queries`).
    if pattern & 32 != 0 {
        let vt = e.mem.u32(lead);
        e.mem.set_u32(vt + 0x2c8, 0x0088_15a0);
    }
    // The lead's extra data: followers (a list of the follower and a non-actor), head track, package
    // data (an object whose slots log), package target.
    let non_actor = forget_object(e, 0);
    let second = block(e, 8, &[non_actor, 0]);
    let first = block(e, 8, &[follower, second]);
    let package_data = forget_object(e, 0);
    give_extra(
        e,
        lead,
        &[
            (0x1d, first, 0),
            (0x46, if pattern & 8 != 0 { target } else { 0 }, 0),
            (0x70, package_data, 0),
            (0x19, 0, if answers & 2 != 0 { target } else { decoy }),
        ],
    );
    let plain = forget_actor(e, target, decoy, (decoy, decoy, 1, 0));
    let skipped = forget_actor(e, target, decoy, (target, target, 0, 0));
    e.mem.set_u32(skipped + 8, 0x0020_0000);
    let other = forget_object(e, if pattern & 4 != 0 { target } else { 0 });
    // The words around the one the clean-up clears in a non-actor (`009707f0`: +0xfc).
    words(e, other + 0xf8, &[0x1111, 0x2222, 0x3333]);
    // A non-actor: its slot 0x100 (is it an actor) answers 0 whatever its other slots answer.
    let vt = e.mem.u32(other);
    e.mem.set_u32(vt + 0x100, NOT_AN_ACTOR);
    // The array's levels; the temp list holds the follower, the non-actor and the skipped one.
    let one_plain = [plain];
    let lead_level = [lead];
    let hole = [0];
    let three = [other, skipped, follower];
    let lead_plain = [lead, plain];
    let others = [other];
    let skipped_level = [skipped];
    let other_lead = [other, lead];
    let levels: [&[u32]; 4] = match pattern % 4 {
        0 => [&one_plain, &lead_level, &hole, &three],
        1 => [&lead_plain, &[], &[], &others],
        2 => [&[], &[], &skipped_level, &other_lead],
        // The array one actor long: the temp list's loop starts from index 1.
        3 if pattern & 8 != 0 => [&lead_level, &[], &[], &[]],
        _ => [&lead_level, &others, &[], &[]],
    };
    let a = process_array(e, levels, 2, [0; 4]);
    let lists = e.mem.alloc(0x1_0400);
    let bytes = e.mem.bytes(a, 0x40);
    e.mem.write(lists + 4, &bytes);
    // With pattern bit 16, the non-actor told about the target (`009c4c80`) is flagged 0x800 and
    // leaves the array while it is told (its level's last element moves into its slot, as
    // `0096aa40` moves it), so `00976680` looks at that slot again.
    if pattern & 16 != 0 {
        let flags = e.mem.u32(other + 8) | 0x800;
        e.mem.set_u32(other + 8, flags);
        let elements = e.mem.u32(a + 4);
        let count = u32::from(e.mem.u16(a + 0xa));
        if let Some(i) = (0..count).find(|i| e.mem.u32(elements + 4 * i) == other) {
            // Level 3's tail in the lists' copy of the array (+4, then +0x20 + 4 * level).
            let level_tail = lists + 4 + 0x20 + 4 * 3;
            words(
                e,
                LEAVES_ITS_SLOT,
                &[elements + 4 * i, elements, level_tail],
            );
        }
    }
    let tail = block(e, 8, &[skipped, 0]);
    let middle = block(e, 8, &[non_actor, tail]);
    words(e, lists + 0x78, &[follower, middle]);
    // The alive actor list's head; the actors close to the player: the target among them.
    words(e, lists + 0x80, &[target, 0]);
    words(e, lists + 0x88, &[decoy, target]);
    // The player, whose followers the target may leave; the object `00976680` first tells.
    let player = forget_actor(e, target, decoy, (decoy, p_answer, 0, pattern & 1));
    give_extra(e, player, &[(0x1d, first, 0)]);
    e.mem.set_u32(0x011d_ea3c, player);
    e.mem.set_u32(0x011f_1958, decoy);
    (lists, target)
}

/// The world patterns, the callees' answers, the target (an actor or not; the exe calls the
/// target's slots, so it is never null).
fn forget_inputs() -> Vec<Input> {
    vec![
        Input::Of(&[0, 1, 2, 3, 5, 6, 7, 9, 12, 15, 17, 20, 26, 31, 33, 46]),
        Input::Of(&[0, 0x5a5a, 0xffff, 0x1234, 0x8001]),
        Input::Of(&[0, 1]),
    ]
}

/// The calls the clean-up does in place: an object's extra data list (`005d43c0`, +0x44) and its
/// word at +0x70 (`004fd380`).
const FORGET_IN_PLACE: &[u32] = &[0x005d_43c0, 0x004f_d380];

#[test]
fn process_null_out_target_ref() {
    let mut inputs = forget_inputs();
    inputs.push(Input::Flag);
    Diff::new(entry!(
        0x0096_f600,
        forget::null_out_target_ref(Ptr<_>, u32, u8)
    ))
    .common()
    // The extra data cache's memset wrapper, a C runtime family member: translated on both sides.
    .goes_by_address(&[0x0040_3d30])
    .ignore_calls(FORGET_IN_PLACE)
    .max_cases(320)
    .inputs(&inputs)
    .setup(|e, case| {
        let (lists, target) = forget_world(e, case.word(0), case.word(1), case.word(2));
        vec![lists, target, case.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn process_reference_destructed() {
    Diff::new(entry!(
        0x0097_6680,
        forget::reference_destructed(Ptr<_>, u32)
    ))
    .common()
    .goes_by_address(&[0x0040_3d30])
    .ignore_calls(FORGET_IN_PLACE)
    // The actors close to the player, which the translation calls as a Rust function.
    .translation_calls_plainly(&[0x0096_e6f0])
    .max_cases(160)
    .inputs(&forget_inputs())
    .setup(|e, case| {
        let (lists, target) = forget_world(e, case.word(0), case.word(1), case.word(2));
        vec![lists, target]
    })
    .run()
    .assert_same();
}

/// The clean-up's world at a size like a town's: `n` actors of the low process level, each with a
/// process and an extra data list of a few entries (none of them the followers), none of them
/// mentioning the target, as for nearly every reference a cell unload frees.
fn forget_crowd(e: &mut Engine, n: u32) -> (u32, u32) {
    stand_ins(e);
    forget_callees(e);
    forget_high_slots(e);
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(0x011c_38e4, 1);
    let base = forget_object(e, 0);
    let decoy = base + 0x11;
    let vt = e.mem.u32(base);
    e.mem.set_u32(decoy, vt);
    let target = forget_object(e, 0);
    for i in 0..FORGET_CALLEES.len() as u32 {
        e.mem.set_u32(FORGET_ANSWERS + 4 * i, 0);
    }
    let actors: Vec<u32> = (0..n)
        .map(|i| {
            let a = forget_actor(e, target, decoy, (decoy, decoy, 3, 0));
            give_extra(e, a, &[(0x0b, i, 0), (0x46, 0, 0), (0x70 + i % 2, 0, 0)]);
            a
        })
        .collect();
    let a = process_array(e, [&[], &[], &[], &actors], 2, [0; 4]);
    let lists = e.mem.alloc(0x1_0400);
    let bytes = e.mem.bytes(a, 0x40);
    e.mem.write(lists + 4, &bytes);
    let player = forget_actor(e, target, decoy, (decoy, decoy, 0, 0));
    e.mem.set_u32(0x011d_ea3c, player);
    e.mem.set_u32(0x011f_1958, decoy);
    (lists, target)
}

/// The time the clean-up takes for one reference over 200 low-process actors, the translations alone
/// against the game's set-up (the families on): 5 rounds of 20 calls each, alternating, the best
/// round of each printed. Run on demand (`--ignored --nocapture`, best in a release build): a timing,
/// not a check. The actors' own slots and the package and combat functions are cheap stand-ins here;
/// in the game they are the translations' work, which the typed clean-up still calls, so the share
/// saved in the game is smaller than here.
#[test]
#[ignore]
fn process_forget_speed() {
    const ACTORS: u32 = 200;
    const CALLS: u32 = 20;
    let mut engines = [
        ("translation", Engine::new_untyped_for_test()),
        ("typed", Engine::new()),
    ];
    for address in [0x0096_f600u32, 0x0097_6680] {
        let mut best = [u128::MAX; 2];
        let worlds: Vec<(u32, u32)> = engines
            .iter_mut()
            .map(|(_, e)| forget_crowd(e, ACTORS))
            .collect();
        for _ in 0..5 {
            for (i, (_, e)) in engines.iter_mut().enumerate() {
                let (lists, target) = worlds[i];
                let started = std::time::Instant::now();
                for _ in 0..CALLS {
                    e.call(address, &[lists, target, 0]);
                }
                best[i] = best[i].min(started.elapsed().as_micros());
            }
        }
        for (i, (name, _)) in engines.iter().enumerate() {
            println!(
                "{address:08x} {name}: {} us for {CALLS} calls over {ACTORS} actors (best of 5)",
                best[i]
            );
        }
    }
}

// ----- 0096c970: the actors' postponed bone updates (NV_PAR_BONES off) -------------------------

/// Where this test's own slot functions live: `IsActor` (+0x100: the object's word +0x10 low byte),
/// the animation (+0x1e4: its word +0x14) and the 3D node (+0x1d0: its word +0x18).
const BONES_FN: u32 = 0x00ff_6000;
/// The bool setting `bTaskletActorSceneGraphUpdates`'s value word, which the setting getter
/// (`00408d60`, a stand-in here) answers the address of.
const TASKLET_VALUE: u32 = LOG + 0x3d00;

fn bones_slot<const SLOT: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a[0];
    log(e, 0xb000 | SLOT, this);
    ret(match SLOT {
        0x100 => e.mem.u32(this + 0x10) & 0xff,
        0x1e4 => e.mem.u32(this + 0x14),
        _ => e.mem.u32(this + 0x18),
    })
}

/// An actor of the bones pass from `kind`: 0 null; 1 not an actor; 2 an actor without an animation;
/// 3.. an actor with an animation whose `UpdateQueuedScenegraph` (a stand-in) answers bit 0 of
/// `kind - 3`, asking for its lighting by bit 1.
fn bones_actor(e: &mut Engine, kind: u32) -> u32 {
    if kind == 0 {
        return 0;
    }
    let vt = e.mem.alloc(0x400);
    for slot in [0x100u32, 0x1e4, 0x1d0] {
        e.mem.set_u32(vt + slot, BONES_FN + slot);
    }
    let a = e.mem.alloc(0x160);
    e.mem.set_u32(a, vt);
    e.mem.set_u32(a + 0x10, u32::from(kind != 1));
    e.mem.set_u32(a + 0x18, 0x0500_0000 + kind);
    if kind >= 3 {
        let bits = kind - 3;
        let animation = e.mem.alloc(0x100);
        e.mem.set_u32(animation + 0x40, bits & 1);
        e.mem.set_u32(a + 0x14, animation);
        e.mem.set_u8(a + 0x14e, ((bits >> 1) & 1) as u8);
    }
    a
}

fn bones_stand_ins(e: &mut Engine) {
    e.register(BONES_FN + 0x100, bones_slot::<0x100>);
    e.register(BONES_FN + 0x1e4, bones_slot::<0x1e4>);
    e.register(BONES_FN + 0x1d0, bones_slot::<0x1d0>);
    e.register(0x0040_8d60, |e, a| {
        log(e, 0x8d60, a[0]);
        ret(TASKLET_VALUE)
    });
    // `UpdateQueuedScenegraph`: logs the animation, answers its word +0x40.
    e.register(0x0049_3930, |e, a| {
        log(e, 0x3930, a[0]);
        ret(e.mem.u32(a[0] + 0x40))
    });
    // The scrap array: construct, push (the count at +8 up by one), destruct; the task's build and run.
    e.register(0x0097_89e0, |e, a| {
        log(e, 0x89e0, 0);
        e.mem.set_u32(a[0] + 8, 0);
        ret(a[0])
    });
    e.register(0x007c_b2e0, |e, a| {
        let pushed = e.mem.u32(a[1]);
        log(e, 0xb2e0, pushed);
        let n = e.mem.u32(a[0] + 8);
        e.mem.set_u32(a[0] + 8, n + 1);
        ret(0)
    });
    e.register(0x0097_8a80, |e, _| {
        log(e, 0x8a80, 0);
        ret(0)
    });
    e.register(0x0087_cf10, |e, a| {
        log(e, 0xcf10, a[1]);
        log(e, 0xcf11, a[3]);
        ret(a[0])
    });
    e.register(0x00c4_4c90, logged::<0x10>);
    // The shadow scene nodes' table (`00450b80` reads it; a family member, so not a stand-in).
    e.map(0x011f_9000, 0x1000);
    e.register(0x00b5_d9f0, logged::<0x11>);
    e.register(0x0049_6960, logged::<0x12>);
}

#[test]
fn process_parallel_update_animation_scenegraph() {
    engine_typed::process::bones::set_for_tests("off", 3);
    // The actors' kinds pattern, the game's own parallel switch, how many actors (none, one, many).
    Diff::new(entry!(
        0x0096_c970,
        process::bones::parallel_update_animation_scenegraph(Ptr<_>)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 3, 5]), Input::Flag, Input::Of(&[0, 1, 13])])
    .setup(|e, case| {
        stand_ins(e);
        bones_stand_ins(e);
        e.mem.set_u8(TASKLET_VALUE, case.word(1) as u8);
        e.mem.set_u32(0x011f_91c8, 0x0600_0000);
        e.mem.set_u32(0x0120_2df4, 0x0700_0000);
        let n = case.word(2);
        let k = case.word(0);
        let all: Vec<u32> = (0..n).map(|i| bones_actor(e, (i * 3 + k) % 7)).collect();
        // Levels 0 and 1 hold them (the pass walks level 0's head to level 1's tail); one actor of
        // level 2 after them is not walked.
        let split = all.len() / 2;
        let other = [bones_actor(e, 4)];
        let a = process_array(e, [&all[..split], &all[split..], &other, &[]], 2, [0; 4]);
        let lists = block(e, 0x100, &[]);
        let bytes = e.mem.bytes(a, 0x40);
        e.mem.write(lists + 4, &bytes);
        vec![lists]
    })
    .run()
    .assert_same();
}
