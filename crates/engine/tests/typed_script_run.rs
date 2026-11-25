//! The differential test of the scripts family's run functions (`engine_typed::scripts::run`)
//! against their translations: `Script::Run` with its quest delays, the magic effect's three runs,
//! the run manager's instance, run and action list build, a fresh runner and its busy test, the
//! forgetting of a run's referenced objects, and the locals' variable set and action flags' clear.
//!
//! The script's header (`00500940`) and the elapsed time (`0084d030`, the frame timer's float at
//! +0xc) are the family's own getters, each with a test of its own here. What stands in: the quest
//! delay setting and a quest's delay, the referenced-object list, the run count's interlocked add
//! and subtract, the allocator and its free, the fill, the manager's constructor and the exit
//! queue, the runner's line loop and the action list's run (xNVSE's wrapped pair: by address in
//! play too), the log, the save game's loading test and form, the form lookup, the cast and the
//! names.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_core::typed::leaf;
use engine_typed::scripts::{self, run};

const CONFIG: u32 = 0x0300_0000;
/// The frame timer (`011f6394`) whose float at +0xc is the elapsed time.
const ELAPSED_TIMER: u32 = 0x011f_6394;
/// What the stand-ins answer: the quest's delay (a double), the line loop's result, whether a save
/// loads, the form being loaded's header, the looked-up form, the cast, the name, whether the
/// allocator answers null.
const ANSWER_QUEST_DELAY: u32 = CONFIG + 8;
const ANSWER_RUN: u32 = CONFIG + 0x10;
const ANSWER_LOADING: u32 = CONFIG + 0x14;
const ANSWER_HEADER: u32 = CONFIG + 0x18;
const ANSWER_FORM: u32 = CONFIG + 0x1c;
const ANSWER_CAST: u32 = CONFIG + 0x20;
const ANSWER_NAME: u32 = CONFIG + 0x24;
const ANSWER_NO_BLOCK: u32 = CONFIG + 0x28;
/// The quest delay setting's float, and what the line loop saw (so a word handed on differently
/// shows in memory).
const SETTING: u32 = CONFIG + 0x30;
const SEEN: u32 = CONFIG + 0x40;
const FORM_VTABLE: u32 = CONFIG + 0x100;
const NAME_SLOT_FN: u32 = 0x00ff_3010;
/// The manager, its constructed flag and the run count; the save game's global.
const MANAGER: u32 = 0x011c_aed0;
const MANAGER_PAGE: u32 = 0x011c_a000;
const RUN_DEPTH: u32 = 0x011c_aedc;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const SAVE_PAGE: u32 = 0x011d_e000;
const TIME_PAGE: u32 = 0x011f_6000;

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

/// The exe's data and the stand-ins.
fn world(e: &mut Engine) {
    for page in [MANAGER_PAGE, SAVE_PAGE, TIME_PAGE] {
        e.map(page, 0x1000);
    }
    e.map(CONFIG, 0x1000);
    e.register(0x0040_3e20, |_, a| ret(a[0] - 0x011c_ac68 + SETTING));
    e.register(0x0079_2760, |e, _| ret_f(e.mem.f64(ANSWER_QUEST_DELAY)));
    e.register(0x005d_43c0, |e, a| ret(e.mem.u32(a[0] + 0x44)));
    e.register(0x0040_b460, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x0040_19a0, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x0040_1000, |e, a| {
        ret(if e.mem.u32(ANSWER_NO_BLOCK) != 0 {
            0
        } else {
            e.mem.alloc(a[0])
        })
    });
    e.register(0x0040_1030, |_, _| ret(0));
    e.register(0x0040_3d30, |e, a| {
        for i in 0..a[2] {
            e.mem.set_u8(a[0] + i, a[1] as u8);
        }
        ret(a[0])
    });
    e.register(0x0096_a2d0, |e, a| {
        e.mem.set_u32(a[0], 0);
        e.mem.set_u32(a[0] + 4, 0);
        ret(a[0])
    });
    e.register(0x00ec_658f, |_, _| ret(0));
    // The line loop and the action list's run: what they were handed, and the answer.
    e.register(0x005e_0d20, |e, a| {
        let mut h = 0u32;
        for w in a.iter().take(9) {
            h = h.rotate_left(5) ^ w;
        }
        let seen = e.mem.u32(SEEN);
        e.mem.set_u32(SEEN, seen.rotate_left(7) ^ h);
        e.mem.set_u32(SEEN + 4, e.mem.u32(MANAGER + 4));
        ret(e.mem.u32(ANSWER_RUN))
    });
    e.register(0x005e_1160, |e, a| {
        let h = a[0] ^ a[1].rotate_left(8) ^ a[2].rotate_left(16) ^ a[3].rotate_left(24);
        e.mem.set_u32(SEEN + 8, h);
        ret(1)
    });
    e.register(0x005b_5e40, |e, a| {
        let mut h = e.mem.u32(SEEN + 12);
        for w in a.iter().take(6) {
            h = h.rotate_left(3) ^ w;
        }
        e.mem.set_u32(SEEN + 12, h);
        ret(0)
    });
    e.register(0x0047_c850, |e, _| ret(e.mem.u32(ANSWER_LOADING)));
    e.register(0x004f_d3c0, |e, _| ret(e.mem.u32(ANSWER_HEADER)));
    e.register(0x0048_39c0, |e, _| ret(e.mem.u32(ANSWER_FORM)));
    e.register(0x00ec_43fb, |e, _| ret(e.mem.u32(ANSWER_CAST)));
    e.register(0x0055_d520, |e, _| ret(e.mem.u32(ANSWER_NAME)));
    e.register(0x0048_2720, |e, _| ret(e.mem.u32(ANSWER_NAME) ^ 0x10));
    e.register(NAME_SLOT_FN, |_, _| ret(0x0050_0000));
    for slot in (0..0x140).step_by(4) {
        e.mem.set_u32(FORM_VTABLE + slot, NAME_SLOT_FN);
    }
}

fn list(e: &mut Engine, items: &[u32]) -> u32 {
    if items.is_empty() {
        return e.mem.alloc(8);
    }
    let mut next = 0;
    for item in items.iter().rev() {
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, *item);
        e.mem.set_u32(node + 4, next);
        next = node;
    }
    next
}

/// A script (a form whose header sits at +0x18): data size `size`, quest script or not, the delay
/// and buffer, the owner quest; referenced objects with and without a variable id.
fn script(e: &mut Engine, size: u32, quest: bool, delay: f32, owner: u32) -> u32 {
    let s = e.mem.alloc(0x54);
    e.mem.set_u32(s, FORM_VTABLE);
    e.mem.set_u8(s + 4, 0x11);
    e.mem.set_u32(s + 0x20, size);
    e.mem.set_u8(s + 0x28, u8::from(quest));
    e.mem.set_f32(s + 0x38, delay);
    e.mem.set_f32(s + 0x3c, 0.25);
    e.mem.set_u32(s + 0x40, owner);
    let objects: Vec<u32> = [(0x0070_0000u32, 7u32), (0x0070_0100, 0), (0, 3)]
        .iter()
        .map(|&(form, id)| {
            let o = e.mem.alloc(0x10);
            e.mem.set_u32(o + 8, form);
            e.mem.set_u32(o + 0xc, id);
            o
        })
        .collect();
    let l = list(e, &objects);
    e.mem.set_u32(s + 0x44, l);
    s
}

/// The TLS block's last referenced-object search's script.
fn last_search(e: &mut Engine, script: u32) {
    let tls = e.tls();
    e.mem.set_u32(tls + 0x288, script);
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&scripts::SCRIPTS)
            .family(&leaf::LEAVES)
            .allow_nan_payloads()
    }
}

const DELAYS: &[u32] = &[0, 0x3f80_0000, 0x4120_0000, 0xbf80_0000];

#[test]
fn scripts_run() {
    Diff::new(entry!(
        0x005a_c1e0,
        run::script_run(Ptr<_>, u32, u32, u32, u8) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[2, 4, 5, 0x40]),
        Input::Flag,
        Input::Of(DELAYS),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(768)
    .setup(|e, case| {
        world(e);
        // Owner quest: none, one whose delay is above 0, one at 0 (the stand-in answers the same
        // double for any quest, so case 2 sets it to 0).
        let owner = if case.word(3) == 0 { 0 } else { 0x0071_0000 };
        let s = script(
            e,
            case.word(0),
            case.word(1) != 0,
            f32::from_bits(case.word(2)),
            owner,
        );
        // The frame timer's elapsed seconds (`0084d030` reads its float at +0xc).
        e.mem.set_f32(ELAPSED_TIMER + 0xc, 2.5);
        e.mem.set_f64(
            ANSWER_QUEST_DELAY,
            if case.word(3) == 2 { 0.0 } else { 1.75 },
        );
        e.mem
            .set_f32(SETTING, if case.word(5) != 0 { 3.0 } else { 0.0 });
        e.mem.set_u32(ANSWER_RUN, case.word(6));
        if case.word(7) != 0 {
            last_search(e, s);
        }
        vec![s, 0x0072_0000, 0x0073_0000, 0x0074_0000, case.word(4)]
    })
    .run()
    .assert_same();
}

#[test]
fn scripts_run_effects() {
    Diff::new(entry!(0x005a_c340, run::run_effect_start(Ptr<_>, u32, u32)))
        .common()
        .inputs(&[Input::Flag, Input::Flag])
        .setup(|e, case| {
            world(e);
            let s = script(e, 8, false, 0.0, 0);
            e.mem.set_u32(ANSWER_RUN, case.word(0));
            e.mem.set_u32(RUN_DEPTH, case.word(1));
            vec![s, 0x0072_0000, 0x0073_0000]
        })
        .run()
        .assert_same();
    Diff::new(entry!(
        0x005a_c380,
        run::run_effect_finish(Ptr<_>, u32, u32)
    ))
    .common()
    .inputs(&[Input::Flag, Input::Flag])
    .setup(|e, case| {
        world(e);
        let s = script(e, 8, false, 0.0, 0);
        e.mem.set_u32(ANSWER_RUN, case.word(0));
        e.mem.set_u32(RUN_DEPTH, case.word(1));
        vec![s, 0x0072_0000, 0x0073_0000]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x005a_c3c0,
        run::run_effect_update(Ptr<_>, u32, u32, f32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 0x3e80_0000, 0x7f80_0001, 0xff80_0000]),
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let s = script(e, 8, false, 0.0, 0);
        e.mem.set_u32(RUN_DEPTH, case.word(1));
        vec![s, 0x0072_0000, 0x0073_0000, case.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn scripts_run_manager() {
    Diff::new(entry!(
        0x005e_2590,
        run::manager_run(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, u8, u8, u8, f32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 5]),
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 0x3f00_0000, 0x7f80_0001]),
        Input::Flag,
    ])
    .max_cases(256)
    .setup(|e, case| {
        world(e);
        e.mem.set_u32(RUN_DEPTH, case.word(0));
        if case.word(1) != 0 {
            let cached = e.mem.alloc(0xa4);
            e.mem.set_u32(MANAGER, cached);
        }
        e.mem.set_u32(MANAGER + 4, 0x0075_0000);
        e.mem.set_u32(ANSWER_NO_BLOCK, case.word(2));
        e.mem.set_u32(ANSWER_RUN, case.word(3));
        let s = script(e, 8, false, 0.0, 0);
        vec![
            MANAGER,
            s,
            0x0072_0000,
            0x0073_0000,
            0x0074_0000,
            case.word(5),
            1,
            0,
            case.word(4),
        ]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x005e_24d0, run::manager_instance() -> Ptr<_>))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(0x011c_aed8, case.word(0));
            e.mem.set_u32(MANAGER, 0x1234);
            vec![]
        })
        .run()
        .assert_same();
    Diff::new(entry!(
        0x005e_2730,
        run::manager_init_action_list(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&[Input::Flag, Input::Flag, Input::Flag])
    .setup(|e, case| {
        world(e);
        if case.word(0) != 0 {
            let cached = e.mem.alloc(0xa4);
            e.mem.set_u32(MANAGER, cached);
            e.mem.set_u32(cached + 0x14, case.word(1) * 0x0076_0000);
        }
        // A failed allocation only with a cached runner: without one the exe's busy test reads
        // through the null runner too.
        e.mem.set_u32(ANSWER_NO_BLOCK, case.word(2) & case.word(0));
        vec![MANAGER, 0x0072_0000, 0x0073_0000, 0x0074_0000]
    })
    .run()
    .assert_same();
}

#[test]
fn scripts_runner_construct_and_busy() {
    Diff::new(entry!(0x005e_0c60, run::construct_runner(Ptr<_>) -> Ptr<_>))
        .common()
        .inputs(&[Input::Of(&[0, 0xa5a5_a5a5])])
        .setup(|e, case| {
            world(e);
            let r = e.mem.alloc(0xa4);
            for i in (0..0xa4).step_by(4) {
                e.mem.set_u32(r + i, case.word(0) ^ i);
            }
            vec![r]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x005e_2850, run::runner_is_busy(Ptr<_>) -> bool))
        .common()
        .inputs(&[Input::Of(&[0, 1, 0x8000_0000])])
        .setup(|e, case| {
            world(e);
            let r = e.mem.alloc(0xa4);
            e.mem.set_u32(r + 0x14, case.word(0));
            e.mem.set_u32(r + 0x10, 0xffff_ffff);
            vec![r]
        })
        .run()
        .assert_same();
}

#[test]
fn scripts_forget_referenced_objects() {
    Diff::new(entry!(0x005a_e2e0, run::forget_referenced_objects(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            world(e);
            let s = script(e, 8, false, 0.0, 0);
            match case.word(0) {
                0 => last_search(e, s),
                1 => last_search(e, s + 4),
                _ => {
                    // An empty list.
                    let empty = list(e, &[]);
                    e.mem.set_u32(s + 0x44, empty);
                }
            }
            vec![s]
        })
        .run()
        .assert_same();
}

/// Locals with variables 1, 2 and 9 (values set), an action list of three entries, and a master
/// script or none.
fn locals(e: &mut Engine, master: bool) -> u32 {
    let mut vars = Vec::new();
    for (id, value) in [(1u32, 2.5f64), (2, -4.0), (9, 0.0)] {
        let v = e.mem.alloc(0x18);
        e.mem.set_u32(v, id);
        e.mem.set_f64(v + 8, value);
        vars.push(v);
    }
    let l = e.mem.alloc(0x14);
    let head = list(e, &vars);
    e.mem.set_u32(l + 0xc, head);
    let mut actions = Vec::new();
    for (form, flags) in [(0x0070_0000u32, 3u32), (0, 1), (0x0070_0100, 0xffff_ffff)] {
        let a = e.mem.alloc(8);
        e.mem.set_u32(a, form);
        e.mem.set_u32(a + 4, flags);
        actions.push(a);
    }
    let action_list = list(e, &actions);
    e.mem.set_u32(l + 8, action_list);
    if master {
        let s = script(e, 8, false, 0.0, 0);
        e.mem.set_u32(l, s);
    }
    l
}

#[test]
fn scripts_set_variable_and_action_flags() {
    Diff::new(entry!(0x005a_9290, run::set_variable(Ptr<_>, u32, f64)))
        .common()
        .inputs(&[
            Input::Of(&[1, 2, 9, 5]),
            // The double's high word (its low word is 1): 1.5, a tiny number, a signalling NaN with
            // and without its sign.
            Input::Of(&[0x3ff8_0000, 0, 0x7ff0_0000, 0xfff4_0000]),
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2]),
            Input::Flag,
        ])
        .max_cases(768)
        .setup(|e, case| {
            world(e);
            let l = locals(e, case.word(2) != 0);
            let save = e.mem.alloc(0x200);
            e.mem.set_u32(save + 0x1c4, 0x0050_1000);
            e.mem.set_u32(SAVE_LOAD_GAME, save);
            e.mem.set_u32(ANSWER_LOADING, case.word(3));
            let header = if case.word(4) != 0 {
                let h = e.mem.alloc(8);
                e.mem.set_u32(h, 0x0001_2345);
                h
            } else {
                0
            };
            e.mem.set_u32(ANSWER_HEADER, header);
            let (form, cast) = match case.word(5) {
                0 => (0, 0),
                1 => (0x0077_0000, 0),
                _ => (0x0077_0000, 0x0077_0000),
            };
            e.mem.set_u32(ANSWER_FORM, form);
            e.mem.set_u32(ANSWER_CAST, cast);
            e.mem.set_u32(ANSWER_NAME, case.word(6) * 0x0050_2000);
            // The double: high word from the input, low word a pattern (a signalling NaN's payload).
            vec![l, case.word(0), 0x0000_0001, case.word(1)]
        })
        .run()
        .assert_same();
    // The empty-list exits: a variable list whose first node has no item (the list of a script
    // with no variables) or none at all, with and without a master script, while a save loads or
    // not.
    Diff::new(entry!(0x005a_9290, run::set_variable(Ptr<_>, u32, f64)))
        .common()
        .inputs(&[
            Input::Of(&[1, 2]),
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            world(e);
            let l = locals(e, case.word(1) != 0);
            let variables = if case.word(0) == 1 { list(e, &[]) } else { 0 };
            e.mem.set_u32(l + 0xc, variables);
            let save = e.mem.alloc(0x200);
            e.mem.set_u32(save + 0x1c4, 0x0050_1000);
            e.mem.set_u32(SAVE_LOAD_GAME, save);
            e.mem.set_u32(ANSWER_LOADING, case.word(2));
            let header = if case.word(3) != 0 {
                let h = e.mem.alloc(8);
                e.mem.set_u32(h, 0x0001_2345);
                h
            } else {
                0
            };
            e.mem.set_u32(ANSWER_HEADER, header);
            let (form, cast) = match case.word(4) {
                0 => (0, 0),
                1 => (0x0077_0000, 0),
                _ => (0x0077_0000, 0x0077_0000),
            };
            e.mem.set_u32(ANSWER_FORM, form);
            e.mem.set_u32(ANSWER_CAST, cast);
            e.mem.set_u32(ANSWER_NAME, 0x0050_2000);
            vec![l, 1, 0, 0x3ff8_0000]
        })
        .run()
        .assert_same();
    // The action list: whole, none, a first node with no item (an empty list), an item-0 node in
    // the middle (the walk stops there).
    Diff::new(entry!(0x005a_8ea0, run::clear_action_flags(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            world(e);
            let l = locals(e, false);
            match case.word(0) {
                1 => e.mem.set_u32(l + 8, 0),
                2 => {
                    let empty = list(e, &[]);
                    e.mem.set_u32(l + 8, empty);
                }
                3 => {
                    let head = e.mem.u32(l + 8);
                    let second = e.mem.u32(head + 4);
                    e.mem.set_u32(second, 0);
                }
                _ => {}
            }
            vec![l]
        })
        .run()
        .assert_same();
}

/// Words a getter hands back whole, and an object's address near the top of memory (the sum wraps).
const WORDS: &[u32] = &[0, 1, 0x0012_3456, 0x7fff_ffff, 0x8000_0000, 0xffff_ffe0];

#[test]
fn scripts_plus_18() {
    Diff::new(entry!(0x0050_0940, run::plus_18(Ptr) -> u32))
        .common()
        .inputs(&[Input::Of(WORDS)])
        .run()
        .assert_same();
}

/// The floats at +0xc: zeros, a subnormal, ordinary values, the largest float, an infinity, a quiet
/// NaN and a signalling one (the `FLD` quiets it), with their signs.
const FLOATS: &[u32] = &[
    0,
    0x8000_0000,
    1,
    0x3f80_0000,
    0xc020_0000,
    0x7f7f_ffff,
    0xff80_0000,
    0x7fc0_1234,
    0x7f80_0001,
    0xffa0_0000,
];

#[test]
fn scripts_float_at_c() {
    Diff::new(entry!(0x0084_d030, run::float_at_c(Ptr) -> f64))
        .common()
        .inputs(&[Input::Of(FLOATS)])
        .setup(|e, case| {
            let object = e.mem.alloc(0x10);
            e.mem.set_u32(object + 0xc, case.word(0));
            vec![object]
        })
        .run()
        .assert_same();
}
