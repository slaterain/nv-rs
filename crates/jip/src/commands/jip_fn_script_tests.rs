//! Tests of `jip_fn_script` on scripts at the exe's offsets (data length +0x20, data +0x30, the
//! quest delay counter +0x38), quests (script +0x1C), references with an `ExtraScript` (type
//! 0x0D: script +0xC, event list +0x10), compiled script data built by hand in the shape
//! `ScriptBlockIterator` and `Cmd_DisableScriptedActivate_Execute` read, and the world hooks'
//! state (`jipFormFlags5`/`6`, the disabled events, the waits). The game's `SetActionFlag`
//! (`005A8E20`) and `Script::Run` (`005AC1E0`) are recording stand-ins.

use super::*;
use crate::commands::test_support::*;
use crate::world_hooks::ScriptWait;

const QUEST: u32 = FORMS + 0x100;
const QUEST2: u32 = FORMS + 0x180;
const SCRIPT_A: u32 = FORMS + 0x200;
const SCRIPT_B: u32 = FORMS + 0x300;
const ACTI: u32 = FORMS + 0x400;
const REFR: u32 = FORMS + 0x500;
const REFR2: u32 = FORMS + 0x600;
const NPC_FORM: u32 = FORMS + 0x700;
const XSCRIPT: u32 = FORMS + 0x800;
const XSCRIPT2: u32 = FORMS + 0x880;
const EVENTS: u32 = FORMS + 0x900;
const EVENTS2: u32 = FORMS + 0x980;
const DATA_A: u32 = FORMS + 0x1000;
const DATA_B: u32 = FORMS + 0x1400;
const XDL: u32 = FORMS + 0x1800;
const TEMP_REF: u32 = FORMS + 0x1900;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    form(&mut e, QUEST, 0x47, 0x0100_0100);
    form(&mut e, QUEST2, 0x47, 0x0100_0180);
    form(&mut e, SCRIPT_A, 0x11, 0x0100_0200);
    form(&mut e, SCRIPT_B, 0x11, 0x0100_0300);
    form(&mut e, ACTI, 0x15, 0x0100_0400);
    form(&mut e, REFR, 0x3a, 0x0100_0500);
    form(&mut e, REFR2, 0x3a, 0x0100_0600);
    form(&mut e, NPC_FORM, 0x2a, 0x0100_0700);
    e.mem.set_u32(REFR + 0x20, ACTI);
    e.mem.set_u32(REFR2 + 0x20, ACTI);
    for (i, f) in [QUEST, SCRIPT_A, ACTI, SCRIPT_B, NPC_FORM, QUEST2]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.register(SET_ACTION_FLAG, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((SET_ACTION_FLAG, w.to_vec())));
        Ret {
            eax: e.mem.u32(w[0] + 0x40),
            ..Ret::default()
        }
    });
    e.register(SCRIPT_RUN, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((SCRIPT_RUN, w.to_vec())));
        Ret::default()
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call_on(e, opcode, this, args).1
}

/// Compiled data: `ScriptName` (the word 0x1D), then each block as the compiler writes it:
/// `Begin` (0x10, length 6, the type, the 32-bit length of the rest: 4 + its lines), its lines,
/// `End` (the word 0x11). The data's length.
fn script_data(e: &mut Engine, at: u32, blocks: &[(u16, Vec<u8>)]) -> u32 {
    let mut d = 0x1du32.to_le_bytes().to_vec();
    for (kind, lines) in blocks {
        d.extend_from_slice(&0x10u16.to_le_bytes());
        d.extend_from_slice(&6u16.to_le_bytes());
        d.extend_from_slice(&kind.to_le_bytes());
        d.extend_from_slice(&(4 + lines.len() as u32).to_le_bytes());
        d.extend_from_slice(lines);
        d.extend_from_slice(&0x11u32.to_le_bytes());
    }
    e.mem.write(at, &d);
    d.len() as u32
}

fn give_script(e: &mut Engine, script: u32, data: u32, length: u32) {
    e.mem.set_u32(script + 0x30, data);
    e.mem.set_u32(script + 0x20, length);
}

/// A line: the opcode, its length, its bytes.
fn line(op: u16, args: &[u8]) -> Vec<u8> {
    let mut v = op.to_le_bytes().to_vec();
    v.extend_from_slice(&(args.len() as u16).to_le_bytes());
    v.extend_from_slice(args);
    v
}

#[test]
fn quest_delay() {
    let mut e = setup();
    e.mem.set_f32(SCRIPT_A + 0x38, 2.5);
    assert_eq!(run(&mut e, 0x2622, 0, &[r(1)]), 0.0, "no script");
    e.mem.set_u32(QUEST + 0x1c, SCRIPT_A);
    assert_eq!(run(&mut e, 0x2622, 0, &[r(1)]), 2.5);
    e.mem.set_f32(SCRIPT_A + 0x38, 0.1);
    assert_eq!(
        run(&mut e, 0x2622, 0, &[r(1)]),
        f64::from(0.1f32),
        "a float"
    );
    run(&mut e, 0x2623, 0, &[r(1), z(-3.25)]);
    assert_eq!(e.mem.f32(SCRIPT_A + 0x38), -3.25);
    run(&mut e, 0x2623, 0, &[r(1), z(1e40)]);
    assert_eq!(
        e.mem.f32(SCRIPT_A + 0x38),
        f32::INFINITY,
        "(float) of a huge double"
    );
    // Another quest has no script: nothing written anywhere.
    e.mem.set_f32(SCRIPT_A + 0x38, 7.0);
    run(&mut e, 0x2623, 0, &[r(6), z(1.0)]);
    assert_eq!(e.mem.f32(SCRIPT_A + 0x38), 7.0);
    // Not a quest: the reader refuses it.
    assert_eq!(run(&mut e, 0x2622, 0, &[r(2)]), 0.0);
    run(&mut e, 0x2623, 0, &[r(2), z(9.0)]);
    assert_eq!(e.mem.f32(SCRIPT_A + 0x38), 7.0);
}

fn disabled_events(e: &mut Engine) -> &mut std::collections::BTreeMap<u32, u32> {
    &mut e.ext::<Jip>().world_hooks.disabled_events
}

/// A masked event by the form's flag (bit 2) and its mask; "OnActivate" by its flag (bit 3);
/// names compared without case; unknown names and the whole text as one name.
#[test]
fn script_event_disabled() {
    let mut e = setup();
    let ask = |e: &mut Engine, name: &[u8]| run(e, 0x27e5, 0, &[r(3), s(name)]);
    assert_eq!(ask(&mut e, b"OnAdd"), 0.0);
    disabled_events(&mut e).insert(ACTI, 0x1 | 0x80);
    assert_eq!(ask(&mut e, b"OnAdd"), 0.0, "the flag is not set");
    e.mem.set_u8(ACTI + 6, 0x04);
    assert_eq!(ask(&mut e, b"OnAdd"), 1.0);
    assert_eq!(ask(&mut e, b"onhit"), 1.0, "any case");
    assert_eq!(ask(&mut e, b"OnEquip"), 0.0, "not in the mask");
    assert_eq!(ask(&mut e, b"OnActivate"), 0.0);
    assert_eq!(ask(&mut e, b"OnAdd OnHit"), 0.0, "one name, not a list");
    assert_eq!(ask(&mut e, b"Nothing"), 0.0);
    e.mem.set_u8(ACTI + 6, 0x08);
    assert_eq!(ask(&mut e, b"OnActivate"), 1.0);
    assert_eq!(
        ask(&mut e, b"OnAdd"),
        0.0,
        "bit 3 alone does not do for a mask"
    );
    disabled_events(&mut e).clear();
    e.mem.set_u8(ACTI + 6, 0x04);
    assert_eq!(ask(&mut e, b"OnAdd"), 0.0, "flag without an entry");
    // OnReset is the top bit.
    disabled_events(&mut e).insert(ACTI, 0x8000_0000);
    assert_eq!(ask(&mut e, b"OnReset"), 1.0);
}

#[test]
#[should_panic(expected = "runs past JIP's 64-byte buffer")]
fn script_event_name_past_the_buffer() {
    let mut e = setup();
    run(&mut e, 0x27e5, 0, &[r(3), s(&[b'a'; 64])]);
}

/// The calling reference's script's event list gets the event's mask with the filter form
/// (`SetActionFlag`); its answer is the result. "OnActivate" (mask 0), unknown names, no
/// script or no event list: no call.
#[test]
fn fake_script_event() {
    let mut e = setup();
    let ev = |e: &mut Engine, name: &[u8], filter: Option<u16>| {
        CALLS.with(|c| c.borrow_mut().clear());
        let mut a = vec![s(name)];
        if let Some(i) = filter {
            a.push(r(i));
        }
        let res = run(e, 0x27e7, REFR, &a);
        (res, calls_to(SET_ACTION_FLAG))
    };
    assert_eq!(ev(&mut e, b"OnAdd", None), (0.0, vec![]), "no ExtraScript");
    put_extra(&mut e, REFR + 0x44, XSCRIPT, 0x0d, SCRIPT_A);
    e.mem.set_u32(XSCRIPT + 0x10, 0);
    assert_eq!(ev(&mut e, b"OnAdd", None), (0.0, vec![]), "no event list");
    e.mem.set_u32(XSCRIPT + 0x10, EVENTS);
    e.mem.set_u32(EVENTS + 0x40, 1);
    assert_eq!(ev(&mut e, b"OnAdd", None), (1.0, vec![vec![EVENTS, 0, 1]]));
    assert_eq!(
        ev(&mut e, b"ondrop", Some(3)),
        (1.0, vec![vec![EVENTS, ACTI, 4]])
    );
    e.mem.set_u32(EVENTS + 0x40, 0x100);
    assert_eq!(
        ev(&mut e, b"OnHit", None),
        (0.0, vec![vec![EVENTS, 0, 0x80]]),
        "the answer's low byte"
    );
    assert_eq!(ev(&mut e, b"OnActivate", None), (0.0, vec![]));
    assert_eq!(ev(&mut e, b"Unknown", None), (0.0, vec![]));
}

/// An inventory reference: its stack's own extra data list is the one looked in, not the
/// temporary reference's.
#[test]
fn fake_script_event_on_an_inventory_reference() {
    let mut e = setup();
    e.register(0x0040_1000, |_, _| Ret {
        eax: TEMP_REF,
        ..Ret::default()
    });
    e.register(0x0055_a2f0, |e, w| {
        e.mem.set_u32(w[0] + 0xc, 0xff00_0800);
        Ret::default()
    });
    put_extra(&mut e, XDL, XSCRIPT, 0x0d, SCRIPT_A);
    e.mem.set_u32(XSCRIPT + 0x10, EVENTS);
    e.mem.set_u32(EVENTS + 0x40, 1);
    let refr = nvse::invref::create(
        &mut e,
        REFR2,
        nvse::invref::Data {
            item: ACTI,
            entry: 0,
            xdata: XDL,
        },
        false,
    );
    assert_eq!(refr, TEMP_REF);
    // The temporary reference's own copy now leads to another script's events.
    e.mem.set_u32(TEMP_REF + 0x48, 0);
    put_extra(&mut e, TEMP_REF + 0x44, XSCRIPT2, 0x0d, SCRIPT_B);
    e.mem.set_u32(XSCRIPT2 + 0x10, EVENTS2);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(run(&mut e, 0x27e7, TEMP_REF, &[s(b"OnEquip")]), 1.0);
    assert_eq!(calls_to(SET_ACTION_FLAG), vec![vec![EVENTS, 0, 2]]);
}

fn waits(e: &mut Engine) -> &mut std::collections::BTreeMap<u32, ScriptWait> {
    &mut e.ext::<Jip>().world_hooks.script_waits
}

/// The quest given, else the calling reference: its flag (bit 3 of `jipFormFlags5`).
#[test]
fn waiting_scripts() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x2811, 0, &[]), 0.0, "neither");
    assert_eq!(run(&mut e, 0x2811, REFR, &[]), 0.0);
    e.mem.set_u8(REFR + 5, 0xf7);
    assert_eq!(run(&mut e, 0x2811, REFR, &[]), 0.0, "every other bit");
    e.mem.set_u8(REFR + 5, 0x08);
    assert_eq!(run(&mut e, 0x2811, REFR, &[]), 1.0);
    assert_eq!(run(&mut e, 0x2811, REFR, &[r(1)]), 0.0, "the quest given");
    e.mem.set_u8(QUEST + 5, 0x08);
    assert_eq!(run(&mut e, 0x2811, 0, &[r(1)]), 1.0);
    e.mem.set_u8(QUEST + 6, 0x08);
    e.mem.set_u8(QUEST + 5, 0);
    assert_eq!(
        run(&mut e, 0x2811, 0, &[r(1)]),
        0.0,
        "byte 6 is not the flag"
    );
    // StopScriptWaiting: the wait cut to one block, only with the flag.
    let wait = ScriptWait {
        owner: REFR,
        iterations: 9,
        ..ScriptWait::default()
    };
    waits(&mut e).insert(REFR, wait);
    waits(&mut e).insert(
        QUEST,
        ScriptWait {
            owner: QUEST,
            iterations: 5,
            ..wait
        },
    );
    run(&mut e, 0x2812, REFR, &[]);
    assert_eq!(waits(&mut e)[&REFR].iterations, 1);
    run(&mut e, 0x2812, REFR, &[r(1)]);
    assert_eq!(waits(&mut e)[&QUEST].iterations, 5, "the quest has no flag");
    e.mem.set_u8(QUEST + 5, 0x08);
    run(&mut e, 0x2812, REFR, &[r(1)]);
    assert_eq!(waits(&mut e)[&QUEST].iterations, 1);
    // A flag with no wait, and no owner at all: nothing.
    e.mem.set_u8(REFR2 + 5, 0x08);
    run(&mut e, 0x2812, REFR2, &[]);
    run(&mut e, 0x2812, 0, &[]);
    assert_eq!(waits(&mut e).len(), 2);
}

fn blocks(e: &mut Engine, script: u32) -> Vec<DisabledBlock> {
    e.ext::<Jip>()
        .script
        .disabled_blocks
        .get(&script)
        .cloned()
        .unwrap_or_default()
}

/// Script A: GameMode (0) at +8, OnActivate (2) at +0x16, GameMode at +0x24 (each block
/// 0xE bytes with its 2-byte line... the type bytes' places counted below).
fn block_script(e: &mut Engine) -> [u32; 3] {
    let len = script_data(
        e,
        DATA_A,
        &[
            (0, line(0x1001, &[])),
            (2, line(0x1002, &[])),
            (0, line(0x1003, &[])),
        ],
    );
    give_script(e, SCRIPT_A, DATA_A, len);
    // Each block: Begin 10 bytes, a 4-byte line, End 4 bytes: 18 bytes from +4.
    [DATA_A + 8, DATA_A + 8 + 18, DATA_A + 8 + 36]
}

#[test]
fn script_blocks_disabled() {
    let mut e = setup();
    let at = block_script(&mut e);
    let get = |e: &mut Engine, kind: i32| run(e, 0x2814, 0, &[r(2), n(kind)]);
    let set = |e: &mut Engine, disable: i32, kind: Option<i32>| {
        let mut a = vec![r(2), n(disable)];
        if let Some(k) = kind {
            a.push(n(k));
        }
        run(e, 0x2815, 0, &a);
    };
    let types = |e: &Engine| at.map(|a| e.mem.u8(a));
    assert_eq!(types(&e), [0, 2, 0]);
    set(&mut e, 1, Some(0));
    assert_eq!(types(&e), [0x0e, 2, 0x0e]);
    assert_eq!(
        blocks(&mut e, SCRIPT_A),
        vec![
            DisabledBlock {
                block_type: 0,
                type_at: at[0]
            },
            DisabledBlock {
                block_type: 0,
                type_at: at[2]
            },
        ]
    );
    assert_eq!(get(&mut e, 0), 1.0);
    assert_eq!(get(&mut e, 2), 0.0);
    // Already disabled: nothing more.
    set(&mut e, 1, Some(0));
    assert_eq!(blocks(&mut e, SCRIPT_A).len(), 2);
    set(&mut e, 1, Some(2));
    assert_eq!(types(&e), [0x0e, 0x0e, 0x0e]);
    // Enabling one type gives its blocks back and keeps the script's entry.
    set(&mut e, 0, Some(0));
    assert_eq!(types(&e), [0, 0x0e, 0]);
    assert_eq!(
        blocks(&mut e, SCRIPT_A),
        vec![DisabledBlock {
            block_type: 2,
            type_at: at[1]
        }]
    );
    // A type with none disabled: nothing.
    set(&mut e, 0, Some(5));
    assert_eq!(blocks(&mut e, SCRIPT_A).len(), 1);
    // The last one goes: the entry goes.
    set(&mut e, 0, Some(2));
    assert_eq!(types(&e), [0, 2, 0]);
    assert!(!e
        .ext::<Jip>()
        .script
        .disabled_blocks
        .contains_key(&SCRIPT_A));
    // A type the script has no block of: no entry made.
    set(&mut e, 1, Some(7));
    assert!(!e
        .ext::<Jip>()
        .script
        .disabled_blocks
        .contains_key(&SCRIPT_A));
    // Negative: disabling does nothing; enabling gives every block back and drops the entry.
    set(&mut e, 1, None);
    set(&mut e, 1, Some(-5));
    assert!(blocks(&mut e, SCRIPT_A).is_empty());
    set(&mut e, 1, Some(2));
    set(&mut e, 1, Some(0));
    set(&mut e, 0, None);
    assert_eq!(types(&e), [0, 2, 0]);
    assert!(!e
        .ext::<Jip>()
        .script
        .disabled_blocks
        .contains_key(&SCRIPT_A));
    // Enabling all of a script with none disabled (JIP's `Remove` on an iterator that found
    // nothing): nothing changes.
    set(&mut e, 0, None);
    assert_eq!(types(&e), [0, 2, 0]);
    // Past 0x25 and wrong forms: nothing.
    set(&mut e, 1, Some(0x26));
    assert!(blocks(&mut e, SCRIPT_A).is_empty());
    assert_eq!(get(&mut e, 0x26), 0.0);
    run(&mut e, 0x2815, 0, &[r(3), n(1), n(0)]);
    assert_eq!(types(&e), [0, 2, 0]);
    assert_eq!(run(&mut e, 0x2814, 0, &[r(3), n(0)]), 0.0);
}

/// Type 0x25 is the last one taken; JIP's quirk with type 0x0E (a disabled block's type):
/// disabling 0x0E also takes the blocks already disabled, and giving those back writes 0x0E.
#[test]
fn script_blocks_edges() {
    let mut e = setup();
    let len = script_data(
        &mut e,
        DATA_A,
        &[(0x25, line(0x1001, &[])), (0, line(0x1002, &[]))],
    );
    give_script(&mut e, SCRIPT_A, DATA_A, len);
    let at = [DATA_A + 8, DATA_A + 8 + 18];
    run(&mut e, 0x2815, 0, &[r(2), n(1), n(0x25)]);
    assert_eq!(e.mem.u8(at[0]), 0x0e);
    assert_eq!(run(&mut e, 0x2814, 0, &[r(2), n(0x25)]), 1.0);
    run(&mut e, 0x2815, 0, &[r(2), n(1), n(0x0e)]);
    assert_eq!(
        blocks(&mut e, SCRIPT_A),
        vec![
            DisabledBlock {
                block_type: 0x25,
                type_at: at[0]
            },
            DisabledBlock {
                block_type: 0x0e,
                type_at: at[0]
            },
        ]
    );
    run(&mut e, 0x2815, 0, &[r(2), n(0), n(0x25)]);
    assert_eq!(e.mem.u8(at[0]), 0x25);
    run(&mut e, 0x2815, 0, &[r(2), n(0), n(0x0e)]);
    assert_eq!(e.mem.u8(at[0]), 0x0e, "JIP writes the 0x0E it kept");
    assert!(blocks(&mut e, SCRIPT_A).is_empty());
}

/// A walk that stops: data not starting with `ScriptName`, a block whose `End` is not where its
/// length says, and a length that ends the walk early.
#[test]
fn script_blocks_walk() {
    let mut e = setup();
    let len = script_data(
        &mut e,
        DATA_A,
        &[(3, line(0x1001, &[])), (4, line(0x1002, &[]))],
    );
    give_script(&mut e, SCRIPT_A, DATA_A, len);
    let has = |e: &mut Engine, kind: i32| run(e, 0x2828, 0, &[n(kind), r(2)]);
    assert_eq!(
        (has(&mut e, 3), has(&mut e, 4), has(&mut e, 5)),
        (1.0, 1.0, 0.0)
    );
    e.mem.set_u32(SCRIPT_A + 0x20, 18 + 4);
    assert_eq!(
        has(&mut e, 4),
        0.0,
        "the length ends the walk after the first"
    );
    assert_eq!(has(&mut e, 3), 1.0);
    e.mem.set_u32(SCRIPT_A + 0x20, len);
    e.mem.set_u32(DATA_A + 4 + 18 + 18 - 4, 0x12);
    assert_eq!(has(&mut e, 4), 0.0, "no End: the walk stops");
    assert_eq!(has(&mut e, 3), 1.0);
    e.mem.set_u32(DATA_A, 0x1c);
    assert_eq!(has(&mut e, 3), 0.0, "not compiled data");
    e.mem.set_u32(SCRIPT_A + 0x30, 0);
    assert_eq!(has(&mut e, 3), 0.0, "no data");
}

/// HasScriptBlock: a script form itself; another form's scriptable component (an activator's
/// at +0x5C, `kBaseComponentOffsets`); the calling reference's base form; boundaries.
#[test]
fn has_script_block() {
    let mut e = setup();
    let len = script_data(&mut e, DATA_B, &[(0x25, line(0x1001, &[]))]);
    give_script(&mut e, SCRIPT_B, DATA_B, len);
    let has = |e: &mut Engine, this: u32, a: &[Vec<u8>]| run(e, 0x2828, this, a);
    assert_eq!(has(&mut e, 0, &[n(0x25), r(4)]), 1.0);
    assert_eq!(has(&mut e, 0, &[n(0x26), r(4)]), 0.0, "past 0x25");
    assert_eq!(
        has(&mut e, 0, &[n(0x25), r(3)]),
        0.0,
        "the activator has no script"
    );
    e.mem.set_u32(ACTI + 0x5c + 4, SCRIPT_B);
    assert_eq!(has(&mut e, 0, &[n(0x25), r(3)]), 1.0);
    assert_eq!(has(&mut e, REFR, &[n(0x25)]), 1.0, "the reference's base");
    assert_eq!(has(&mut e, 0, &[n(0x25)]), 0.0, "no form, no reference");
    // A form type with no scriptable component (a quest has one; a script form is itself).
    form(&mut e, NPC_FORM, 0x36, 0x0100_0700);
    assert_eq!(has(&mut e, 0, &[n(0x25), r(5)]), 0.0);
    // A disabled block reads as 0x0E.
    run(&mut e, 0x2815, 0, &[r(4), n(1), n(0x25)]);
    assert_eq!(has(&mut e, 0, &[n(0x25), r(4)]), 0.0);
    assert_eq!(has(&mut e, 0, &[n(0x0e), r(4)]), 1.0);
}

/// In the calling reference's script, the first OnActivate block's Activate lines (0x100D, also
/// one wrapped in an expression line 0x1C) become 0x2215 and back; lines of other blocks, other
/// opcodes and later blocks are left alone.
#[test]
fn scripted_activate() {
    let mut e = setup();
    let wrapped = {
        let mut v = 0x1cu16.to_le_bytes().to_vec();
        v.extend_from_slice(&0x0009u16.to_le_bytes());
        v.extend(line(0x100d, &[1]));
        v
    };
    let mut on_activate = line(0x100d, &[0xaa, 0xbb]);
    on_activate.extend(wrapped.clone());
    on_activate.extend(line(0x100e, &[]));
    let len = script_data(
        &mut e,
        DATA_A,
        &[
            (0, line(0x100d, &[])),
            (2, on_activate),
            (2, line(0x100d, &[])),
        ],
    );
    give_script(&mut e, SCRIPT_A, DATA_A, len);
    let before = e.mem.bytes(DATA_A, len);
    // No ExtraScript, no event list: nothing.
    run(&mut e, 0x282b, REFR, &[n(1)]);
    put_extra(&mut e, REFR + 0x44, XSCRIPT, 0x0d, SCRIPT_A);
    e.mem.set_u32(XSCRIPT + 0x10, 0);
    run(&mut e, 0x282b, REFR, &[n(1)]);
    assert_eq!(e.mem.bytes(DATA_A, len), before);
    e.mem.set_u32(XSCRIPT + 0x10, EVENTS);
    run(&mut e, 0x282b, REFR, &[n(1)]);
    let mut want = before.clone();
    // The block of type 0: +4, 10 + 4 + 4 bytes; the OnActivate block from +22: its lines from
    // +32: the plain line at +32, the expression line at +38 (its inner opcode at +42), then
    // 0x100E at +47.
    want[32..34].copy_from_slice(&0x2215u16.to_le_bytes());
    want[42..44].copy_from_slice(&0x2215u16.to_le_bytes());
    assert_eq!(e.mem.bytes(DATA_A, len), want);
    run(&mut e, 0x282b, REFR, &[n(1)]);
    assert_eq!(
        e.mem.bytes(DATA_A, len),
        want,
        "already disabled: nothing more"
    );
    run(&mut e, 0x282b, REFR, &[n(0)]);
    assert_eq!(e.mem.bytes(DATA_A, len), before);
}

/// The calling reference's script run on its event list (`Script::Run(ref, events, 0, 0)`).
#[test]
fn execute_script() {
    let mut e = setup();
    run(&mut e, 0x283a, REFR, &[]);
    put_extra(&mut e, REFR + 0x44, XSCRIPT, 0x0d, 0);
    e.mem.set_u32(XSCRIPT + 0x10, EVENTS);
    run(&mut e, 0x283a, REFR, &[]);
    e.mem.set_u32(XSCRIPT + 0xc, SCRIPT_A);
    e.mem.set_u32(XSCRIPT + 0x10, 0);
    run(&mut e, 0x283a, REFR, &[]);
    assert!(calls_to(SCRIPT_RUN).is_empty());
    e.mem.set_u32(XSCRIPT + 0x10, EVENTS);
    run(&mut e, 0x283a, REFR, &[]);
    assert_eq!(
        calls_to(SCRIPT_RUN),
        vec![vec![SCRIPT_A, REFR, EVENTS, 0, 0]]
    );
}
