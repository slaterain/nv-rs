//! The differential test of the script functions xNVSE wraps that every command line goes
//! through (`engine_typed::script_lines::params`) and the locals' action flag set they reach
//! (`engine_typed::scripts::run`), against their translations: a command's parameter read over
//! every kind of parameter and every referenced-object type with forms of every kind it tests,
//! compiled or not, with xNVSE's inline-expression seam; the script event mark with JIP LN's three
//! seams on and off; the locals' deleting destructor; the action list's lookup and flag set.
//!
//! What stands in: the parameter types' table, the form type and compiled-flag reads, the cast,
//! the container test, the form's +0xe8 slot, the referenced-object list, the copy, the log, the
//! extra data list's locals, the locals' destructor and `operator delete`.

use engine as _;
use engine_core::abi::AbiFn;
use engine_core::extend::Hook;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_core::typed::leaf;
use engine_typed::crt;
use engine_typed::script_lines::{self, params};
use engine_typed::scripts::{self, run};

const CONFIG: u32 = 0x0300_0000;
const ANSWER_COMPILED: u32 = CONFIG;
const ANSWER_LOCALS: u32 = CONFIG + 4;
const SEEN: u32 = CONFIG + 0x10;
const FORM_VTABLE: u32 = CONFIG + 0x100;
const SLOT_E8_FN: u32 = 0x00ff_3050;
const NAME_SLOT_FN: u32 = 0x00ff_3010;
/// The parameter types' table (8 bytes a type: whether it is a referenced object).
const TYPE_TABLE: u32 = 0x0118_cdd5;
const TABLE_PAGE: u32 = 0x0118_c000;
const SSE2_PAGE: u32 = 0x0127_0000;
const SSE2_AVAILABLE: u32 = 0x0127_0a6c;
const LINE_PAGE: u32 = 0x011c_a000;
/// The referenced-object types `ParseParameters` knows, and one it does not (0x2e).
const REFERENCE_TYPES: &[u32] = &[
    3, 4, 6, 7, 9, 0xb, 0xe, 0x14, 0x15, 0x19, 0x1f, 0x23, 0x32, 0x35, 0x38, 0x39, 0x3d, 0x2e,
];
/// The referenced objects' form types (0: no form): a reference, a quest, a container type, a
/// weapon-like type, a 0x55, an 8, a 0x39, a 0x2b, a quest of another kind (0x47).
const OBJECT_KINDS: &[u8] = &[0x3b, 0x19, 0x27, 0x2a, 0x55, 8, 0x39, 0x2b, 0x47, 0];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The exe's data and the stand-ins.
fn world(e: &mut Engine) {
    // The types' table runs past its first page for the high types.
    e.map(TABLE_PAGE, 0x2000);
    for page in [SSE2_PAGE, LINE_PAGE] {
        e.map(page, 0x1000);
    }
    e.map(CONFIG, 0x1000);
    e.mem.set_u32(SSE2_AVAILABLE, 1);
    for kind in REFERENCE_TYPES {
        e.mem.set_u8(TYPE_TABLE + kind * 8, 1);
    }
    e.register(0x0040_1170, |e, a| {
        ret(if a[0] == 0 {
            0
        } else {
            u32::from(e.mem.u8(a[0] + 4))
        })
    });
    e.register(0x0040_13e0, |e, _| ret(e.mem.u32(ANSWER_COMPILED)));
    // The cast: target 7 takes references, 11 the 0x2a kind, 31 the 0x55 kind.
    e.register(0x00ec_43fb, |e, a| {
        let kind = if a[0] == 0 { 0 } else { e.mem.u8(a[0] + 4) };
        let ok = match a[3] {
            0x0118_3060 => kind == 0x3b,
            0x0118_3140 => kind == 0x2a,
            0x0118_37c4 => kind == 0x55,
            _ => false,
        };
        ret(if ok { a[0] } else { 0 })
    });
    e.register(0x0048_1f30, |_, a| {
        ret(u32::from(a[0] == 0x27 || a[0] == 0x2a))
    });
    e.register(SLOT_E8_FN, |e, a| {
        ret(u32::from(e.mem.u8(a[0] + 4) == 0x27))
    });
    e.register(NAME_SLOT_FN, |_, _| ret(0x0050_0000));
    for slot in (0..0x140).step_by(4) {
        e.mem.set_u32(FORM_VTABLE + slot, NAME_SLOT_FN);
    }
    e.mem.set_u32(FORM_VTABLE + 0xe8, SLOT_E8_FN);
    e.register(0x005d_43c0, |e, a| ret(e.mem.u32(a[0] + 0x44)));
    e.register(0x005b_5e40, |e, a| {
        let h = e.mem.u32(SEEN) ^ a[0] ^ a.get(1).copied().unwrap_or(0).rotate_left(9);
        e.mem.set_u32(SEEN, h);
        ret(0)
    });
    e.register(0x0041_8830, |e, _| ret(e.mem.u32(ANSWER_LOCALS)));
    e.register(0x005a_8bc0, |e, a| {
        e.mem.set_u32(SEEN + 4, a[0]);
        ret(0)
    });
    e.register(0x0040_1030, |e, a| {
        e.mem.set_u32(SEEN + 8, a[0]);
        ret(0)
    });
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

/// A script with a referenced object for each of [`OBJECT_KINDS`] (1-based); a quest's +0x74 list
/// has a second node for the 0x19 kind.
fn script(e: &mut Engine) -> u32 {
    let objects: Vec<u32> = OBJECT_KINDS
        .iter()
        .map(|&kind| {
            let o = e.mem.alloc(0x10);
            if kind != 0 {
                let f = e.mem.alloc(0x80);
                e.mem.set_u32(f, FORM_VTABLE);
                e.mem.set_u8(f + 4, kind);
                if kind == 0x19 {
                    e.mem.set_u32(f + 0x78, 0x0070_0000);
                }
                e.mem.set_u32(o + 8, f);
            }
            o
        })
        .collect();
    let s = e.mem.alloc(0x54);
    e.mem.set_u32(s, FORM_VTABLE);
    e.mem.set_u8(s + 4, 0x11);
    e.mem.set_u32(s + 0x1c, objects.len() as u32);
    let l = list(e, &objects);
    e.mem.set_u32(s + 0x44, l);
    s
}

/// A parameter list of `types` (12 bytes an entry, the type at +4).
fn parameter_list(e: &mut Engine, types: &[u32]) -> u32 {
    let p = e.mem.alloc(12 * types.len() as u32 + 4);
    for (i, kind) in types.iter().enumerate() {
        e.mem.set_u32(p + 12 * i as u32 + 4, *kind);
    }
    p
}

/// Four output slots, each big enough for a string.
fn outputs(e: &mut Engine) -> Vec<u32> {
    (0..4)
        .map(|i| {
            let o = e.mem.alloc(0x210);
            e.mem.set_u32(o, 0x5555_0000 + i);
            o
        })
        .collect()
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&script_lines::SCRIPT_LINES)
            .family(&scripts::SCRIPTS)
            .family(&crt::CRT)
            .family(&leaf::LEAVES)
            .allow_nan_payloads()
    }
}

#[test]
fn script_params_reference_types() {
    let objects: &'static [u32] = (0..=OBJECT_KINDS.len() as u32 + 1)
        .collect::<Vec<_>>()
        .leak();
    Diff::new((0x005a_ccb0, params::parse_parameters_entry as AbiFn))
        .common()
        .inputs(&[
            Input::Of(REFERENCE_TYPES),
            Input::Of(objects),
            Input::Flag,
            Input::Flag,
        ])
        .max_cases(900)
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(ANSWER_COMPILED, case.word(2));
            let s = script(e);
            let p = parameter_list(e, &[case.word(0)]);
            // A count of 1, the marker (an `r`, or a wrong one) and the object's index.
            let code = e.mem.alloc(0x20);
            e.mem.set_u16(code, 1);
            e.mem
                .set_u8(code + 2, if case.word(3) != 0 { b'r' } else { b'q' });
            e.mem.set_u16(code + 3, case.word(1) as u16);
            let offset = e.mem.alloc(4);
            let mut words = vec![p, code, offset, 0x0071_0000, 0x0072_0000, s, 0];
            words.extend(outputs(e));
            words
        })
        .run()
        .assert_same();
}

/// Lists of plain parameters and their data (after the count).
const PLAIN: &[(&[u32], &[u8])] = &[
    (&[0], b"\x03\x00abc"),
    (&[1], b"n\x07\x00\x00\x00"),
    (&[0x17], b"z\x00\x00\x00\x00\x00\x00\x04\xc0"),
    (&[2], b"z\x00\x00\x00\x00\x00\x00\xe0\x3f"),
    (&[2], b"Q\x00\x00"),
    (&[5, 8, 0x20], b"\xfe\xff\x09\x34\x12"),
    (&[0xa, 0x12, 0x1c], b"\x01\x00\x02\x80\xff\x7f"),
    (
        &[0x29, 0x33, 0x34, 0x37],
        b"\x01\x00\x02\x00\x03\x00\x04\x00",
    ),
    (&[0x50], b"\x00\x00"),
    (&[0, 1], b"\x00\x00n\x01\x00\x00\x00"),
    (&[1, 0], b"\xff\xff\x01\x00"),
];

#[test]
fn script_params_plain_types() {
    let lists: &'static [u32] = (0..PLAIN.len() as u32).collect::<Vec<_>>().leak();
    Diff::new((0x005a_ccb0, params::parse_parameters_entry as AbiFn))
        .common()
        .inputs(&[Input::Of(lists), Input::Flag, Input::Flag, Input::Flag])
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(ANSWER_COMPILED, case.word(1));
            // xNVSE's inline expression: its seam answers when on.
            if case.word(2) != 0 {
                e.on_site_for_test(0x005a_cd0d, |e, w| {
                    e.mem.set_u32(SEEN + 12, w[0] ^ w[1]);
                    Some(1)
                });
            }
            let s = script(e);
            let (types, data) = PLAIN[case.word(0) as usize];
            let p = if case.word(3) != 0 {
                0
            } else {
                parameter_list(e, types)
            };
            let code = e.mem.alloc(0x40);
            e.mem.set_u16(code, types.len() as u16);
            e.mem.write(code + 2, data);
            let offset = e.mem.alloc(4);
            let mut words = vec![p, code, offset, 0x0071_0000, 0x0072_0000, s, 0];
            words.extend(outputs(e));
            words
        })
        .run()
        .assert_same();
}

/// Locals with an action list: entries for forms 0x0070_0000 and none (the "any form" one).
fn locals(e: &mut Engine, with_list: bool, with_any: bool) -> u32 {
    let l = e.mem.alloc(0x14);
    if with_list {
        let mut actions = Vec::new();
        for form in [0x0070_0000u32, 0x0070_0100, 0] {
            if form == 0 && !with_any {
                continue;
            }
            let a = e.mem.alloc(8);
            e.mem.set_u32(a, form);
            e.mem.set_u32(a + 4, 0x10);
            actions.push(a);
        }
        let head = list(e, &actions);
        e.mem.set_u32(l + 8, head);
    }
    l
}

#[test]
fn script_params_action_flags() {
    Diff::new(entry!(
        0x005a_c750,
        params::set_action_flag(u32, Ptr<_>, u32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0x0070_0000, 0x0070_0200]),
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let l = locals(e, true, case.word(5) != 0);
        e.mem
            .set_u32(ANSWER_LOCALS, if case.word(2) != 0 { l } else { 0 });
        // JIP LN's seams: 0 none, 1 on and passing, 2 on and answering.
        let mark = case.word(3);
        e.set_hook_users(Hook::MarkScriptEvent, u32::from(mark != 0));
        e.set_hook_users(Hook::MergeEventMask, u32::from(case.word(4) != 0));
        e.set_hook_users(Hook::OnHitEvent, u32::from(mark != 0));
        if mark == 2 {
            e.on_site_for_test(0x005a_c763, |_, w| Some(w[2] & 1));
        } else if mark == 1 {
            e.on_site_for_test(0x005a_c763, |_, _| None);
        }
        if case.word(4) == 2 {
            e.on_site_for_test(0x005a_c782, |_, w| Some(u32::from(w[1] != 0)));
        } else if case.word(4) == 1 {
            e.on_site_for_test(0x005a_c782, |_, _| None);
        }
        e.on_site_for_test(0x005a_c78a, |e, w| {
            e.mem.set_u32(SEEN + 16, w[0] ^ w[1] ^ w[2] ^ w[3] << 4);
            None
        });
        let extra = if case.word(1) != 0 { 0x0073_0000 } else { 0 };
        vec![case.word(0), extra, 0x41]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x005a_8e20,
        run::set_locals_action_flag(Ptr<_>, u32, u32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0x0070_0000, 0x0070_0200, 0]),
        Input::Flag,
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let l = locals(e, case.word(1) != 0, case.word(2) != 0);
        vec![l, case.word(0), 0x82]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x005a_90b0,
        run::find_action(Ptr<_>, u32, bool) -> Ptr<_>
    ))
    .common()
    .inputs(&[
        Input::Of(&[0x0070_0000, 0x0070_0100, 0x0070_0200, 0]),
        Input::Flag,
        Input::Flag,
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let l = locals(e, case.word(2) != 0, case.word(3) != 0);
        vec![l, case.word(0), case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn script_params_delete_locals() {
    Diff::new(entry!(
        0x0041_af70,
        params::delete_locals(Ptr<_>, u32) -> Ptr<_>
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 3])])
    .setup(|e, case| {
        world(e);
        let l = locals(e, true, true);
        vec![l, case.word(0)]
    })
    .run()
    .assert_same();
}
