//! The differential test of every typed function of the script_lines family
//! (`engine_typed::script_lines`) against its translation: the runner's line loop, the action
//! list's run, `ParseLine` and `EvaluateLine` with its `BEGIN`, `set`, `if`, `elseif`, `else`,
//! `endif` and command lines, in execute and parse mode, with JIP LN's kept failed scripts on and
//! off. The expressions go through the scripts family (on in every run), so the lines reach the
//! typed evaluator as in play.
//!
//! The script's header (`00500940`) is the scripts family's own getter. What stands in: the casts,
//! the locals' owners and setters, the globals' set, the console and the logs, the command table
//! and a command's and block type's handler, the parameter parser, the temporary report string, the
//! reference's changed test and process, and the CRT's `atol` and `atof`.

use engine as _;
use engine_core::extend::{jip_patches, Hook};
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_core::typed::leaf;
use engine_typed::crt;
use engine_typed::script_lines::{self, runner};
use engine_typed::scripts;

const TABLE_PAGE: u32 = 0x0118_c000;
const OPERATOR_TABLE: u32 = 0x0118_cad0;
const ERROR_NAMES: u32 = 0x0118_c594;
const BLOCK_PAGE: u32 = 0x0118_e000;
const BLOCK_TYPES: u32 = 0x0118_e300;
const SSE2_PAGE: u32 = 0x0127_0000;
const SSE2_AVAILABLE: u32 = 0x0127_0a6c;
const FALSE_PAGE: u32 = 0x0101_2000;
const LINE_PAGE: u32 = 0x011c_a000;
/// The save game's global (SetVariable reads it to report a missing variable).
const SAVE_PAGE: u32 = 0x011d_e000;

const CONFIG: u32 = 0x0300_0000;
const ANSWER_HANDLER: u32 = CONFIG;
const ANSWER_BLOCK: u32 = CONFIG + 4;
const ANSWER_BLOCK_RESULT: u32 = CONFIG + 8;
const ANSWER_DELETED: u32 = CONFIG + 12;
const ANSWER_PROCESS: u32 = CONFIG + 16;
/// Whether a console command runs (the thread's echo byte is written from it in a case's setup).
const ANSWER_CONSOLE: u32 = CONFIG + 20;
/// Whether the debug text is shown (the profiler runs), the vtable +0xf0 slot's answer (a form
/// that takes a variable set), the form a reference variable's id looks up, whether the owner
/// lookups answer no script variables, and the performance counter.
const ANSWER_PROFILE: u32 = CONFIG + 0x90;
const ANSWER_F0: u32 = CONFIG + 0x94;
const LOOKED_UP_FORM: u32 = CONFIG + 0x98;
const ANSWER_NO_OWNER: u32 = CONFIG + 0x9c;
const CLOCK: u32 = CONFIG + 0xa0;
/// A second command record (0x1012): needs a reference, marks references changed.
const DEFINITION2: u32 = CONFIG + 0x300;
/// The numeric id a reference variable (local 3) holds, which looks up `LOOKED_UP_FORM`.
const LOOKED_UP_ID: u32 = 0x7777;
const DEFINITION_FOR: u32 = CONFIG + 24;
const DEFINITION: u32 = CONFIG + 0x40;
/// The commands' and the parameter block's list for `ParseParameters` (12 bytes an entry, the type at
/// +4): types the read does not know (0x3e), so a parse with parameters answers false and stores
/// nothing; a count of 0 answers true.
const PARAMETERS: u32 = CONFIG + 0x800;
const FORM_VTABLE: u32 = CONFIG + 0x100;
const HANDLER: u32 = 0x00ff_3000;
const BLOCK_HANDLER: u32 = 0x00ff_3020;
const NAME_SLOT_FN: u32 = 0x00ff_3010;
const SLOT_F0_FN: u32 = 0x00ff_3030;
const SLOT_48_FN: u32 = 0x00ff_3040;

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

fn number_end(text: &str, point: bool) -> usize {
    text.char_indices()
        .find(|&(i, c)| {
            !(c.is_ascii_digit() || (point && c == '.') || (i == 0 && (c == '-' || c == '+')))
        })
        .map_or(text.len(), |(i, _)| i)
}

fn atol(e: &mut Engine, a: &[u32]) -> Ret {
    let text = String::from_utf8_lossy(&e.mem.cstr(a[0])).to_string();
    let end = number_end(&text, false);
    ret(text[..end].parse::<i64>().unwrap_or(0) as i32 as u32)
}

fn atof(e: &mut Engine, a: &[u32]) -> Ret {
    let text = String::from_utf8_lossy(&e.mem.cstr(a[0])).to_string();
    let end = number_end(&text, true);
    ret_f(text[..end].parse::<f64>().unwrap_or(0.0))
}

/// The exe's data and the stand-ins.
fn world(e: &mut Engine) {
    for page in [
        TABLE_PAGE, BLOCK_PAGE, SSE2_PAGE, FALSE_PAGE, LINE_PAGE, SAVE_PAGE,
    ] {
        e.map(page, 0x1000);
    }
    e.map(CONFIG, 0x1000);
    let operators: [(u8, &[u8]); 16] = [
        (0, b"("),
        (0, b")"),
        (1, b"&&"),
        (2, b"||"),
        (3, b"<="),
        (3, b"<"),
        (3, b">="),
        (3, b">"),
        (3, b"=="),
        (3, b"!="),
        (4, b"-"),
        (4, b"+"),
        (5, b"*"),
        (5, b"/"),
        (5, b"%"),
        (6, b"~"),
    ];
    for (number, (precedence, text)) in (0u32..).zip(operators) {
        let entry = OPERATOR_TABLE + number * 8;
        e.mem.set_u32(entry, number);
        e.mem.set_u8(entry + 4, precedence);
        e.mem.write(entry + 5, text);
    }
    for code in 0..7 {
        e.mem.set_u32(ERROR_NAMES + code * 4, 0x5000 + code);
    }
    // Block types 0 (no reference needed, a handler), 1 (needs a reference), 2 (no handler),
    // 3 (parameters).
    for (kind, needs, parameters, handler) in [
        (0u32, 0u8, 0u32, BLOCK_HANDLER),
        (1, 1, 0, BLOCK_HANDLER),
        (2, 0, 0, 0),
        (3, 0, PARAMETERS, BLOCK_HANDLER),
    ] {
        let entry = BLOCK_TYPES + kind * 0x28;
        e.mem.set_u8(entry, needs);
        e.mem.set_u32(entry + 4, parameters);
        e.mem.set_u32(entry + 8, handler);
        e.mem.set_u32(entry - 0x10 + 8, 0x10 + kind);
    }
    e.mem.set_u32(SSE2_AVAILABLE, 1);
    e.register(0x00ec_a6bd, atol);
    e.register(0x00ec_a573, atof);
    e.register(0x0044_a670, |e, a| ret(e.mem.cstr(a[0]).len() as u32));
    for log in [0x005b_5e40u32, 0x0070_3c00, 0x005e_2410, 0x0046_dce0] {
        e.register(log, |_, _| ret(0));
    }
    e.register(0x0070_2fc0, |e, _| ret(e.mem.u32(ANSWER_PROFILE)));
    // QueryPerformanceFrequency and QueryPerformanceCounter: 1000 ticks a second, 7 ticks a read.
    e.register(0x00fd_f0a4, |e, a| {
        e.mem.set_u64(a[0], 1000);
        ret(1)
    });
    e.register(0x00fd_f0a0, |e, a| {
        let now = e.mem.u64(CLOCK) + 7;
        e.mem.set_u64(CLOCK, now);
        e.mem.set_u64(a[0], now);
        ret(1)
    });
    e.register(0x0040_13e0, |e, a| ret((e.mem.u32(a[0] + 8) >> 3) & 1));
    e.register(0x0048_3710, |_, _| ret(0));
    e.register(0x005a_bf60, |e, _| ret(e.mem.u32(CONFIG + 0x30)));
    // The locals' deleting destructor (`0041af70`) is the script_lines family's own: its destructor
    // and `operator delete` stand in.
    e.register(0x005a_8bc0, |_, _| ret(0));
    e.register(0x0040_1030, |_, _| ret(0));
    e.register(0x005e_1140, |_, _| ret(0));
    e.register(0x005d_43c0, |e, a| ret(e.mem.u32(a[0] + 0x44)));
    e.register(0x0040_1170, |e, a| {
        ret(if a[0] == 0 {
            0
        } else {
            u32::from(e.mem.u8(a[0] + 4))
        })
    });
    // The cast: a reference type (0x3a..0x40) to a reference, type 0x47 to a quest.
    e.register(0x00ec_43fb, |e, a| {
        let kind = e.mem.u8(a[0] + 4);
        let ok = if a[3] == 0x0118_41cc {
            (0x3a..=0x40).contains(&kind)
        } else {
            kind == 0x47
        };
        ret(if ok { a[0] } else { 0 })
    });
    for owner in [0x0056_73e0u32, 0x0059_e300] {
        e.register(owner, |e, _| {
            ret(if e.mem.u32(ANSWER_NO_OWNER) != 0 {
                0
            } else {
                e.mem.u32(CONFIG + 0x34)
            })
        });
    }
    e.register(0x005a_bed0, |_, _| ret(0x0050_1000));
    e.register(0x0040_77c0, |e, _| ret(e.mem.u32(ANSWER_DELETED)));
    e.register(0x008d_6f30, |e, _| ret(e.mem.u32(ANSWER_PROCESS)));
    e.register(0x0045_0ff0, |_, _| ret(1));
    // The command table: 0x1010 and the counted 0x1059 (both with parameters), 0x1012 (needs a
    // reference, marks references changed); anything else unknown.
    e.register(0x005b_1120, |e, a| {
        ret(if a[0] == e.mem.u32(DEFINITION_FOR) || a[0] == 0x1059 {
            DEFINITION
        } else if a[0] == 0x1012 {
            DEFINITION2
        } else {
            0
        })
    });
    // A reference variable's form lookup: its form for the one id, none for others.
    e.register(0x0048_39c0, |e, a| {
        ret(if a[0] == LOOKED_UP_ID {
            e.mem.u32(LOOKED_UP_FORM)
        } else {
            0
        })
    });
    e.register(0x0040_37b0, |e, a| {
        e.mem.set_u32(a[0], 0x0050_2000);
        ret(a[0])
    });
    e.register(0x0040_6f60, |_, _| ret(0));
    e.register(0x0046_4f30, |_, a| ret(a[0]));
    e.register(0x0040_37d0, |_, _| ret(0));
    e.register(HANDLER, |e, a| {
        e.mem.set_f64(a[6], 6.25);
        ret(e.mem.u32(ANSWER_HANDLER))
    });
    e.register(BLOCK_HANDLER, |e, a| {
        let answer = e.mem.u32(ANSWER_BLOCK_RESULT);
        e.mem.set_f64(a[6], f64::from(answer));
        ret(e.mem.u32(ANSWER_BLOCK))
    });
    e.register(NAME_SLOT_FN, |_, _| ret(0x0050_0000));
    e.register(SLOT_F0_FN, |e, _| ret(e.mem.u32(ANSWER_F0)));
    e.mem.set_u32(ANSWER_F0, 1);
    e.register(SLOT_48_FN, |_, _| ret(0));
    for slot in (0..0x140).step_by(4) {
        e.mem.set_u32(FORM_VTABLE + slot, NAME_SLOT_FN);
    }
    e.mem.set_u32(FORM_VTABLE + 0xf0, SLOT_F0_FN);
    e.mem.set_u32(FORM_VTABLE + 0x48, SLOT_48_FN);
    e.mem.set_u32(DEFINITION_FOR, 0x1010);
    for entry in 0..8 {
        e.mem.set_u32(PARAMETERS + entry * 12 + 4, 0x3e);
    }
    e.mem.set_u32(DEFINITION + 0x14, PARAMETERS);
    e.mem.set_u32(DEFINITION + 0x18, HANDLER);
    e.mem.set_u32(DEFINITION2, 0x0050_3000);
    e.mem.set_u8(DEFINITION2 + 0x10, 1);
    e.mem.set_u32(DEFINITION2 + 0x18, HANDLER);
    e.mem.set_u8(DEFINITION2 + 0x25, 1);
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

fn locals(e: &mut Engine) -> u32 {
    let mut vars = Vec::new();
    // Local 3 is a reference variable: its double's first word holds a numeric form id.
    for (id, value) in [
        (1u32, 2.5f64),
        (2, -4.0),
        (3, f64::from_bits(u64::from(LOOKED_UP_ID))),
    ] {
        let v = e.mem.alloc(0x10);
        e.mem.set_u32(v, id);
        e.mem.set_f64(v + 8, value);
        vars.push(v);
    }
    let l = e.mem.alloc(0x14);
    let head = list(e, &vars);
    e.mem.set_u32(l + 0xc, head);
    l
}

fn form(e: &mut Engine, kind: u8) -> u32 {
    let f = e.mem.alloc(0x40);
    e.mem.set_u32(f, FORM_VTABLE);
    e.mem.set_u8(f + 4, kind);
    e.mem.set_u32(f + 0xc, 0x0012_3456);
    e.mem.set_u32(f + 0x20, 9);
    f
}

/// A runner and a script whose data is `code`: referenced objects 1 a global, 2 a quest, 3 a
/// reference, 4 one with no form, 5 a reference variable not set yet (its local holds 0), 6 a
/// reference variable whose local holds a form's id (looked up to a reference).
struct World {
    runner: u32,
    script: u32,
    data: u32,
    locals: u32,
}

fn script_world(e: &mut Engine, code: &[u8], magic_effect: bool) -> World {
    let locals = locals(e);
    let other = self::locals(e);
    e.mem.set_u32(CONFIG + 0x30, other);
    e.mem.set_u32(CONFIG + 0x34, other);
    let looked_up = form(e, 0x3b);
    e.mem.set_u32(LOOKED_UP_FORM, looked_up);
    // 0xff and 0xfe: reference variables (a variable id, no form yet) of locals 2 and 3.
    let objects: Vec<u32> = [0x06u8, 0x47, 0x3b, 0, 0xff, 0xfe]
        .iter()
        .map(|kind| {
            let o = e.mem.alloc(0x10);
            let f = if *kind == 0 || *kind >= 0xfe {
                0
            } else {
                form(e, *kind)
            };
            e.mem.set_u32(o + 8, f);
            if *kind >= 0xfe {
                e.mem.set_u32(o + 0xc, u32::from(0x101 - u16::from(*kind)));
            }
            o
        })
        .collect();
    let script = e.mem.alloc(0x54);
    e.mem.set_u32(script, FORM_VTABLE);
    e.mem.set_u8(script + 4, 0x11);
    e.mem.set_u32(script + 8, 8);
    e.mem.set_u32(script + 0x1c, objects.len() as u32);
    e.mem.set_u32(script + 0x20, code.len() as u32);
    e.mem.set_u8(script + 0x29, u8::from(magic_effect));
    let data = e.mem.alloc(code.len() as u32 + 0x10);
    e.mem.write(data, code);
    e.mem.set_u32(script + 0x30, data);
    let l = list(e, &objects);
    e.mem.set_u32(script + 0x44, l);
    let runner = e.mem.alloc(0xa4);
    e.mem.set_u32(runner + 8, locals);
    World {
        runner,
        script,
        data,
        locals,
    }
}

/// What the seams' handlers saw, so a word handed to one differently shows in memory.
const SEEN: u32 = CONFIG + 0x80;

/// The extenders' seams in these functions, when `on`: JIP LN's kept failed scripts (its three
/// jumps), the seams before a block type's and a command's handler (xNVSE and JIP zero the
/// result there), and JIP LN's script wait at the run's start and at a block's test (which here
/// makes the test itself for block type 3, as a resumed wait does). xNVSE's wraps over the four
/// functions themselves are not in these runs: a wrap goes over the typed base when the families
/// are registered, which a setup can't do; every call into them stays by address (`e.call`), so a
/// wrap is reached as before.
fn extenders(e: &mut Engine, on: bool) {
    e.set_hook_users(Hook::ScriptWait, u32::from(on));
    if on {
        e.set_jip_patches(
            jip_patches::INSTALLED | jip_patches::FIXES | jip_patches::NO_FAILED_SCRIPT_LOCKS,
        );
        // O7's three and G24's skip of a busy runner's names and log (`0x5E0DD1`).
        for site in [0x005e_0fbe, 0x005e_1a23, 0x005e_1f0b, 0x005e_0dd1] {
            e.on_site_for_test(site, |_, _| Some(1));
        }
        for site in [0x005e_16c3, 0x005e_2329] {
            e.on_site_for_test(site, |e, w| {
                e.mem.set_u32(SEEN, w[0] ^ w[1].rotate_left(8));
                e.mem.set_f64(w[2], 0.0);
                None
            });
        }
        e.on_site_for_test(0x005e_0ec9, |e, w| {
            e.mem.set_u32(SEEN + 4, w[0]);
            e.mem.set_u32(SEEN + 8, w[1]);
            None
        });
        e.on_site_for_test(0x005e_1716, |e, w| {
            if w[1] == 3 {
                let length = e.mem.u32(w[5]);
                e.mem.set_u32(w[5], length.wrapping_add(w[6]));
                return Some(1);
            }
            e.mem.set_u32(SEEN + 12, w[0] ^ w[2] ^ w[3] ^ w[4] << 4);
            None
        });
    }
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

/// Lines for `EvaluateLine`: (opcode, the bytes after the opcode and length words).
const LINES: &[(u32, &[u8])] = &[
    // BEGIN: block types 0, 1, 2, 3 and an invalid one (span 4 words).
    (0x10, b"\x00\x00\x04\x00\x00\x00"),
    (0x10, b"\x01\x00\x04\x00\x00\x00"),
    (0x10, b"\x02\x00\x04\x00\x00\x00"),
    (0x10, b"\x03\x00\x04\x00\x00\x00"),
    (0x10, b"\x30\x00\x04\x00\x00\x00"),
    // set: a float, an integer and a short variable, a global, a reference's and a quest's
    // variable, a missing form, an expression error.
    (0x15, b"f\x01\x00\x06\x00 3 4 +"),
    (0x15, b"l\x02\x00\x06\x00 7 2 /"),
    (0x15, b"s\x01\x00\x08\x00 1.5 2 *"),
    (0x15, b"G\x01\x00\x04\x00 2.5"),
    (0x15, b"r\x03\x00f\x01\x00\x04\x00 1.0"),
    (0x15, b"r\x02\x00s\x02\x00\x02\x00 5"),
    (0x15, b"r\x04\x00f\x01\x00\x02\x00 5"),
    (0x15, b"f\x01\x00\x04\x00 1 +"),
    // if, elseif (true, false, an error), else, endif.
    (0x16, b"\x03\x00\x02\x00 1"),
    (0x16, b"\x03\x00\x02\x00 0"),
    (0x16, b"\x03\x00\x04\x00 1 +"),
    (0x18, b"\x02\x00\x06\x00 s\x01\x00"),
    (0x17, b"\x05\x00"),
    (0x19, b""),
    // end, scriptname, a reference prefix, return.
    (0x11, b""),
    (0x1d, b""),
    (0x1c, b""),
    (0x1e, b""),
    // Commands: known, unknown, the counted one.
    (0x1010, b"\x00\x00"),
    (0x1011, b"\x00\x00"),
    (
        0x1059,
        b"\x02\x00n\x01\x00\x00\x00n\x02\x00\x00\x00\x00\x00\x00\x00",
    ),
];

#[test]
fn script_lines_evaluate_line() {
    let lines: &'static [u32] = (0..LINES.len() as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x005e_1550,
        runner::run_line(Ptr<_>, Ptr<_>, i32, Ptr<_>, Ptr<_>, u32, Ptr<_>, u32, bool, bool) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(lines),
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 1, 3, 6]),
        Input::Flag,
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(640)
    .setup(|e, case| {
        world(e);
        extenders(e, case.word(7) != 0);
        let (opcode, bytes) = LINES[case.word(0) as usize];
        let mut code = vec![0u8; 4];
        code.extend_from_slice(bytes);
        let w = script_world(e, &code, false);
        // The if-stack: depth 0 or 1 with the level open, taken or closed.
        let level = case.word(4);
        e.mem.set_u32(w.runner + 0x20, u32::from(level == 6));
        e.mem.set_u32(w.runner + 0x24, level & 3);
        e.mem.set_u32(w.runner + 0x28, level & 3);
        e.mem.set_u32(ANSWER_HANDLER, case.word(5));
        e.mem.set_u32(ANSWER_BLOCK, case.word(5));
        e.mem.set_u32(ANSWER_BLOCK_RESULT, case.word(6));
        e.mem.set_u32(ANSWER_PROCESS, case.word(6) * 0x0090_0000);
        let reference = if case.word(3) != 0 { form(e, 0x3b) } else { 0 };
        let length = e.mem.alloc(4);
        e.mem.set_u32(length, bytes.len() as u32);
        let skip = e.mem.alloc(4);
        let _ = w.locals;
        vec![
            w.runner,
            w.script,
            opcode,
            reference,
            length,
            4,
            skip,
            7,
            case.word(1),
            case.word(2),
        ]
    })
    .run()
    .assert_same();
}

/// Lines for `ParseLine`: an ordinary line, a reference prefix to an object with a reference, to
/// one without a form, to a reference variable not set yet and to one holding a form's id, past
/// the count.
const PARSED: &[&[u8]] = &[
    b"\x15\x00\x06\x00",
    b"\x1c\x00\x03\x00\x10\x10\x02\x00",
    b"\x1c\x00\x04\x00\x10\x10\x02\x00",
    b"\x1c\x00\x05\x00\x10\x10\x02\x00",
    b"\x1c\x00\x06\x00\x10\x10\x02\x00",
    b"\x1c\x00\x09\x00\x11\x10\x02\x00",
    b"\x1c\x00\x02\x00\x10\x10\x02\x00",
];

#[test]
fn script_lines_parse_line() {
    let parsed: &'static [u32] = (0..PARSED.len() as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x005e_13a0,
        runner::read_line(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, bool) -> bool
    ))
    .common()
    .inputs(&[Input::Of(parsed), Input::Flag, Input::Flag])
    .setup(|e, case| {
        world(e);
        let w = script_world(e, PARSED[case.word(0) as usize], false);
        let work = e.mem.alloc(16);
        e.mem.set_u32(work + 12, case.word(2) * 0x1234);
        vec![
            w.runner,
            w.script,
            work,
            work + 4,
            work + 8,
            work + 12,
            case.word(1),
        ]
    })
    .run()
    .assert_same();
}

/// Whole scripts for the two loops: a block with lines, a skipped `if`, a failing parse, a return.
const SCRIPTS: &[&[u8]] = &[
    b"\x10\x00\x06\x00\x00\x00\x00\x00\x00\x00\x15\x00\x06\x00f\x01\x00\x02\x00 5\x11\x00\x00\x00",
    b"\x16\x00\x06\x00\x01\x00\x02\x00 0\x15\x00\x06\x00f\x01\x00\x02\x00 5\x19\x00\x00\x00",
    b"\x1e\x00\x00\x00\x15\x00\x06\x00f\x01\x00\x02\x00 5",
    b"\x1c\x00\x04\x00\x10\x10\x02\x00\x00\x00",
    b"\x10\x10\x02\x00\x00\x00\x11\x10\x02\x00\x00\x00",
    // A command that needs a reference and marks references changed (the loop answers true).
    b"\x12\x10\x02\x00\x00\x00",
    // An invalid block type (the line fails).
    b"\x10\x00\x06\x00\x30\x00\x04\x00\x00\x00",
    // A set through a reference variable holding a form's id.
    b"\x15\x00\x0a\x00r\x06\x00f\x01\x00\x02\x00 5",
];

#[test]
fn script_lines_run() {
    let scripts_: &'static [u32] = (0..SCRIPTS.len() as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x005e_0d20,
        runner::run_lines(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, bool, u8, u8, f32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(scripts_),
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(512)
    .setup(|e, case| {
        world(e);
        extenders(e, case.word(5) != 0);
        let w = script_world(e, SCRIPTS[case.word(0) as usize], case.word(3) != 0);
        e.mem.set_u32(ANSWER_HANDLER, 1);
        e.mem.set_u32(ANSWER_BLOCK, 1);
        e.mem.set_u32(ANSWER_BLOCK_RESULT, 1);
        if case.word(4) != 0 {
            e.mem.set_u32(w.runner + 0x14, w.script);
        }
        let locals = if case.word(1) != 0 { w.locals } else { 0 };
        if case.word(3) != 0 {
            let effect = e.mem.alloc(8);
            e.mem.set_u32(w.locals + 0x10, effect);
            let other = e.mem.u32(CONFIG + 0x30);
            e.mem.set_u32(other + 0x10, effect);
        }
        // A null reference is how every quest script runs.
        let reference = if case.word(6) != 0 { 0 } else { form(e, 0x3b) };
        vec![
            w.runner,
            w.script,
            reference,
            locals,
            0x0070_0000,
            case.word(2),
            1,
            0,
            0x3f00_0000,
        ]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x005e_1160,
        runner::build_action_list(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&[Input::Of(scripts_), Input::Flag, Input::Flag])
    .setup(|e, case| {
        world(e);
        let w = script_world(e, SCRIPTS[case.word(0) as usize], false);
        if case.word(1) != 0 {
            e.mem.set_u32(w.runner + 0x14, w.script);
        }
        let reference = if case.word(2) != 0 { 0 } else { form(e, 0x3b) };
        vec![w.runner, w.script, reference, w.locals]
    })
    .run()
    .assert_same();
}

/// The line loop's rarer paths (the review of #867): the magic effect's block made on the first
/// run (none in the locals), the profiler's timing with the debug text shown, a null reference,
/// with the extenders' seams on and off, every script above.
#[test]
fn script_lines_run_effect_block_and_profiler() {
    let scripts_: &'static [u32] = (0..SCRIPTS.len() as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x005e_0d20,
        runner::run_lines(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, bool, u8, u8, f32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(scripts_),
        Input::Flag,
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(256)
    .setup(|e, case| {
        world(e);
        extenders(e, case.word(3) != 0);
        // 0: no magic effect; 1: a magic effect with its block made; 2: one without (made now).
        let magic = case.word(2);
        let w = script_world(e, SCRIPTS[case.word(0) as usize], magic != 0);
        e.mem.set_u32(ANSWER_HANDLER, 1);
        e.mem.set_u32(ANSWER_BLOCK, 1);
        e.mem.set_u32(ANSWER_BLOCK_RESULT, 1);
        e.mem.set_u32(ANSWER_PROFILE, case.word(4));
        if magic == 1 {
            let effect = e.mem.alloc(8);
            e.mem.set_u32(w.locals + 0x10, effect);
        }
        let reference = if case.word(1) != 0 { 0 } else { form(e, 0x3b) };
        vec![
            w.runner,
            w.script,
            reference,
            w.locals,
            0x0070_0000,
            0,
            1,
            1,
            0x3e80_0000,
        ]
    })
    .run()
    .assert_same();
}

/// `EvaluateLine`'s set and command paths the first test does not reach (the review of #867): a
/// set through a reference variable not set yet and through one holding a form's id, on a form with
/// no script variables (a global), on a reference whose owner answers none, with no runner locals,
/// a deleted target, a form whose +0xf0 slot says no; the command that needs a reference and marks
/// references changed; the counted command's operands; the console's echo.
const GAP_LINES: &[(u32, &[u8])] = &[
    (0x15, b"r\x05\x00f\x01\x00\x02\x00 5"),
    (0x15, b"r\x06\x00f\x01\x00\x02\x00 5"),
    (0x15, b"r\x06\x00l\x02\x00\x02\x00 7"),
    (0x15, b"r\x01\x00f\x01\x00\x02\x00 5"),
    (0x15, b"r\x03\x00f\x01\x00\x04\x00 1.0"),
    (0x15, b"r\x02\x00s\x02\x00\x02\x00 5"),
    (0x15, b"f\x01\x00\x06\x00 3 4 +"),
    (0x15, b"l\x02\x00\x06\x00 7 2 /"),
    (0x15, b"G\x01\x00\x04\x00 2.5"),
    (0x1012, b"\x00\x00"),
    (
        0x1059,
        b"\x02\x00n\x01\x00\x00\x00n\x02\x00\x00\x00\x00\x00\x00\x00",
    ),
];

#[test]
fn script_lines_evaluate_set_and_command_paths() {
    let lines: &'static [u32] = (0..GAP_LINES.len() as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x005e_1550,
        runner::run_line(Ptr<_>, Ptr<_>, i32, Ptr<_>, Ptr<_>, u32, Ptr<_>, u32, bool, bool) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(lines),
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 1, 2]),
        Input::Flag,
    ])
    .max_cases(1100)
    .setup(|e, case| {
        world(e);
        let (opcode, bytes) = GAP_LINES[case.word(0) as usize];
        let mut code = vec![0u8; 4];
        code.extend_from_slice(bytes);
        let w = script_world(e, &code, false);
        e.mem.set_u32(ANSWER_HANDLER, 1);
        e.mem.set_u32(ANSWER_PROCESS, 0);
        if case.word(3) != 0 {
            e.mem.set_u32(w.runner + 8, 0);
        }
        e.mem.set_u32(ANSWER_NO_OWNER, case.word(4));
        // 0: an ordinary target; 1: a deleted one; 2: a form whose +0xf0 slot says no.
        e.mem.set_u32(ANSWER_DELETED, u32::from(case.word(5) == 1));
        e.mem.set_u32(ANSWER_F0, u32::from(case.word(5) != 2));
        if case.word(6) != 0 {
            e.mem.set_u32(ANSWER_CONSOLE, 1);
            let tls = e.tls();
            e.mem.set_u8(tls + 0x268, 1);
        }
        let reference = if case.word(2) != 0 { form(e, 0x3b) } else { 0 };
        let length = e.mem.alloc(4);
        e.mem.set_u32(length, bytes.len() as u32);
        let skip = e.mem.alloc(4);
        vec![
            w.runner,
            w.script,
            opcode,
            reference,
            length,
            4,
            skip,
            7,
            case.word(1),
            0,
        ]
    })
    .run()
    .assert_same();
}

#[test]
fn script_lines_data_is_read_where_the_runner_says() {
    // A guard for the fixtures: the script's data word (+0x30) is what the lines read.
    let mut e = Engine::new();
    world(&mut e);
    let w = script_world(&mut e, b"\x11\x00\x00\x00", false);
    assert_eq!(e.mem.u32(w.script + 0x30), w.data);
}

/// A busy runner reads its running script's word once: here the asked script's name (the first of
/// the log line's two) clears the word, as another thread finishing that script in the same runner
/// did (a loading thread's action list beside main's run, the speed-20000 trip, 2026-11-26). The
/// exe reads the word again and would read through 0; the log line names the script that was
/// running, and nothing runs. Both the action list's build and the line loop.
#[test]
fn script_lines_busy_runner_reads_the_running_script_once() {
    const RUNNER_OF_TEST: u32 = CONFIG + 0xc0;
    const LOGGED: u32 = CONFIG + 0xc4;
    const CLEARING_NAME_FN: u32 = 0x00ff_3050;
    const ASKED_NAME: u32 = 0x0050_1000;
    let mut e = Engine::new_untyped_for_test();
    world(&mut e);
    e.register(CLEARING_NAME_FN, |e, _| {
        let runner = e.mem.u32(RUNNER_OF_TEST);
        e.mem.set_u32(runner + 0x14, 0);
        ret(ASKED_NAME)
    });
    e.register(0x005b_5e40, |e, a| {
        e.mem.set_u32(LOGGED, a[1]);
        e.mem.set_u32(LOGGED + 4, a[2]);
        ret(0)
    });
    let w = script_world(&mut e, b"\x11\x00\x00\x00", false);
    // The asked script's own vtable: the form vtable with the clearing name.
    let vtable = e.mem.alloc(0x140);
    for slot in (0..0x140).step_by(4) {
        let f = e.mem.u32(FORM_VTABLE + slot);
        e.mem.set_u32(vtable + slot, f);
    }
    e.mem.set_u32(vtable + 0x130, CLEARING_NAME_FN);
    e.mem.set_u32(w.script, vtable);
    let running = form(&mut e, 0x11);
    e.mem.set_u32(RUNNER_OF_TEST, w.runner);
    let busy = |e: &mut Engine| {
        e.mem.set_u32(w.runner + 0x14, running);
        e.mem.set_u32(LOGGED, 0);
        e.mem.set_u32(LOGGED + 4, 0);
    };

    busy(&mut e);
    assert!(!runner::build_action_list(
        &mut e,
        Ptr::new(w.runner),
        Ptr::new(w.script),
        Ptr::NULL,
        Ptr::new(w.locals),
    ));
    assert_eq!(e.mem.u32(LOGGED), 0x0050_0000, "the running script's name");
    assert_eq!(e.mem.u32(LOGGED + 4), ASKED_NAME);
    assert_eq!(e.mem.u32(w.runner + 0x14), 0, "not taken");

    busy(&mut e);
    assert!(!runner::run_lines(
        &mut e,
        Ptr::new(w.runner),
        Ptr::new(w.script),
        Ptr::NULL,
        Ptr::new(w.locals),
        Ptr::new(0x0070_0000),
        false,
        1,
        0,
        0.5,
    ));
    assert_eq!(e.mem.u32(LOGGED), 0x0050_0000, "the running script's name");
    assert_eq!(e.mem.u32(LOGGED + 4), ASKED_NAME);
    assert_eq!(e.mem.u32(w.runner + 0x14), 0, "not taken");
}

// ---- The game's own compiled scripts (opt-in) ---------------------------------------------------

/// The real scripts' world on top of [`world`]: every block type (0 to 0x25) with a handler that
/// applies, and a record for every command opcode (0x1000 up) whose handler succeeds.
fn real_world(e: &mut Engine) {
    for kind in 0..0x26u32 {
        let entry = BLOCK_TYPES + kind * 0x28;
        e.mem.set_u32(entry + 8, BLOCK_HANDLER);
        e.mem.set_u32(entry - 0x10 + 8, 0x10 + kind);
    }
    e.register(0x005b_1120, |_, a| {
        ret(if a[0] >= 0x1000 { DEFINITION } else { 0 })
    });
    e.mem.set_u32(ANSWER_HANDLER, 1);
    e.mem.set_u32(ANSWER_BLOCK, 1);
    e.mem.set_u32(ANSWER_BLOCK_RESULT, 1);
    // What the operand reader asks of the real operands: a form by id, a global's value, the
    // container's reference pointer and inventory item (none here), the item's locals.
    e.register(0x0048_39c0, |_, _| ret(0));
    e.register(0x0052_6ac0, |e, a| {
        ret_f(if a[0] == 0 {
            0.0
        } else {
            f64::from(f32::from_bits(e.mem.u32(a[0] + 0x24)))
        })
    });
    for nothing in [0x0041_c8d0u32, 0x0057_6260, 0x004b_dea0, 0x0044_59e0] {
        e.register(nothing, |_, _| ret(0));
    }
}

/// `NV_DATA="...\Data" cargo test -p engine --test typed_script_lines -- --ignored real_scda`:
/// `FalloutNV.esm`'s compiled scripts (up to `NV_SCRIPTS`, default 400) run whole through the line
/// loop, typed against translated, the extenders' seams on and off, with given and fresh locals,
/// and with a null reference (as quest scripts run).
/// Nothing of the game's is stored; referenced objects beyond the world's four answer as missing
/// (such a line fails as in the game with a missing reference), every command succeeds.
#[test]
#[ignore = "needs the player's FalloutNV.esm (set NV_DATA)"]
fn real_scda_scripts() {
    use esm::{FourCC, Plugin};
    let data = std::path::PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA"));
    let limit: usize = std::env::var("NV_SCRIPTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400);
    let plugin = Plugin::open(data.join("FalloutNV.esm")).expect("FalloutNV.esm");
    let mut all = Vec::new();
    for entry in plugin.records_of_type(FourCC(*b"SCPT")) {
        let record = plugin.record(entry).expect("SCPT");
        if let Some(code) = record.get(FourCC(*b"SCDA")) {
            if !code.data.is_empty() {
                all.push(code.data.to_vec());
            }
        }
        if all.len() >= limit {
            break;
        }
    }
    assert!(!all.is_empty(), "no scripts found");
    let count = all.len();
    let all: &'static [Vec<u8>] = all.leak();
    let indices: &'static [u32] = (0..count as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x005e_0d20,
        runner::run_lines(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, bool, u8, u8, f32) -> bool
    ))
    .common()
    .inputs(&[Input::Of(indices), Input::Flag, Input::Of(&[0, 1, 2])])
    .max_cases(count * 6)
    .setup(move |e, case| {
        world(e);
        real_world(e);
        extenders(e, case.word(1) != 0);
        let w = script_world(e, &all[case.word(0) as usize], false);
        // 0: fresh locals; 1: given locals; 2: given locals and a null reference (a quest script).
        let locals = if case.word(2) != 0 { w.locals } else { 0 };
        let reference = if case.word(2) == 2 { 0 } else { form(e, 0x3b) };
        vec![
            w.runner,
            w.script,
            reference,
            locals,
            0x0070_0000,
            0,
            0,
            0,
            0,
        ]
    })
    .run()
    .assert_same();
    eprintln!("{count} real scripts compared");
}
