//! Tests of the rest of `Commands_Script.cpp` through the commands' records, on scripts, event
//! lists and references laid out as the game's. The answers pinned are xNVSE's as its source gives
//! them; none of these commands has been run in the real game yet.

use super::super::rtti;
use super::super::test_support::*;
use crate::table::guest_text;
use crate::vars;
use engine_core::prelude::*;
use std::cell::RefCell;

/// A reference and its scriptable base form; scripts (two object scripts, a quest script, a magic
/// script); a script effect; a quest; the event lists of the reference and the quest; the
/// reference's `ExtraScript`; a form that is not a script.
const REF: u32 = 0x0200_0800;
const BASE: u32 = 0x0200_1000;
const OBJ_A: u32 = 0x0200_1800;
const OBJ_B: u32 = 0x0200_2000;
const QUEST_SCRIPT: u32 = 0x0200_2800;
const MAGIC: u32 = 0x0200_3000;
const EFFECT: u32 = 0x0200_3800;
const QUEST: u32 = 0x0200_4000;
const REF_EVENTS: u32 = 0x0200_4800;
const QUEST_EVENTS: u32 = 0x0200_5000;
const EXTRA: u32 = 0x0200_5800;
const PLAIN: u32 = 0x0200_6000;
/// Where variable names, list nodes and locals go, and the bump heap.
const SPACE: u32 = 0x0200_7000;
const HEAP: u32 = 0x0200_a000;
/// `TESScriptableForm` sits at `+0x5C0` in the fake cast: its script at `+0x5C4`.
const SCRIPT_OF: u32 = 0x5c4;
/// The event list `Script::CreateEventList` makes in the fake.
const NEW_EVENTS: u32 = 0x0200_6800;

thread_local! {
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
    static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn calls() -> Vec<(u32, Vec<u32>)> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn lines() -> Vec<String> {
    LINES.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

fn record(addr: u32, a: &[u32], n: usize) {
    CALLS.with(|c| c.borrow_mut().push((addr, a[..n].to_vec())));
}

fn console_on(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

fn bit(class: u32) -> u64 {
    component(class).0
}

fn form(e: &mut Engine, at: u32, kind: u8, classes: u64) {
    e.mem.set_u8(at + 4, kind);
    e.mem.set_u32(at + 0xc, at);
    set_classes(e, at, classes);
}

/// A bump allocator over `SPACE` for the test's own lists and texts.
fn space(e: &mut Engine, size: u32) -> u32 {
    let next = e.mem.u32(SPACE).max(SPACE + 0x10);
    e.mem.set_u32(SPACE, next + size.next_multiple_of(16));
    next
}

/// A variable `name` of id `id` declared in `script`'s variable list.
fn declare(e: &mut Engine, script: u32, id: u32, name: &str) {
    let info = space(e, 0x20);
    let text = space(e, name.len() as u32 + 1);
    e.mem.write(text, name.as_bytes());
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(info, id);
    e.mem.set_u32(info + 0x18, text);
    push(e, script + 0x4c, info);
}

/// A local of id `id` holding `value` in `events`' local list.
fn local(e: &mut Engine, events: u32, id: u32, value: f64) -> u32 {
    let at = space(e, 0x10);
    e.mem.set_u32(at, id);
    e.mem.set_f64(at + 8, value);
    let node = space(e, 8);
    e.mem.set_u32(node, at);
    e.mem.set_u32(node + 4, e.mem.u32(events + 0xc));
    e.mem.set_u32(events + 0xc, node);
    at
}

/// `item` appended to the `tList` whose head node is at `head`.
fn push(e: &mut Engine, head: u32, item: u32) {
    if e.mem.u32(head) == 0 {
        e.mem.set_u32(head, item);
        return;
    }
    let mut node = head;
    while e.mem.u32(node + 4) != 0 {
        node = e.mem.u32(node + 4);
    }
    let added = space(e, 8);
    e.mem.set_u32(added, item);
    e.mem.set_u32(node + 4, added);
}

fn setup() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    LINES.with(|l| l.borrow_mut().clear());
    let mut e = engine_with_table();
    // The runtime error's globals.
    e.map(0x011c_3000, 0x1000);
    e.register(0x00ec_a7f4, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(0x0071_b160, |_, _| eax(0x0200_f000));
    e.register(0x0071_d0a0, |e, a| {
        let format = guest_text(e, a[1]);
        let list = e.mem.bytes(a[2], 16);
        let line = super::super::printf::ucrt_format(e, &format, &list);
        LINES.with(|l| l.borrow_mut().push(String::from_utf8(line).unwrap()));
        eax(0)
    });
    e.register(0x0040_1000, |e, a| {
        let at = e.mem.u32(HEAP).max(HEAP + 0x10);
        e.mem.set_u32(HEAP, at + a[0].next_multiple_of(16));
        eax(at)
    });
    // `BaseExtraList::GetByType`: the extra at `list + 0x100 + 4 * type`.
    e.register(0x0041_0220, |e, a| eax(e.mem.u32(a[0] + 0x100 + 4 * a[1])));
    e.register(0x0041_0140, |e, a| {
        record(0x0041_0140, a, 2);
        e.mem.set_u32(a[0] + 0x100 + 4 * a[1], 0);
        eax(0)
    });
    e.register(0x0040_ff60, |e, a| {
        record(0x0040_ff60, a, 2);
        let kind = u32::from(e.mem.u8(a[1] + 4));
        e.mem.set_u32(a[0] + 0x100 + 4 * kind, a[1]);
        eax(1)
    });
    e.register(crate::functions::CREATE_EVENT_LIST, |_, a| {
        record(crate::functions::CREATE_EVENT_LIST, a, 1);
        eax(NEW_EVENTS)
    });
    e.register(crate::functions::SCRIPT_EXECUTE, |_, a| {
        record(crate::functions::SCRIPT_EXECUTE, a, 5);
        eax(0x0100)
    });
    let refr = bit(rtti::TES_OBJECT_REFR);
    form(&mut e, REF, 0x3a, refr);
    e.mem.set_u32(REF + 0x20, BASE);
    form(&mut e, BASE, 0x18, bit(rtti::TES_SCRIPTABLE_FORM));
    for (s, kind) in [(OBJ_A, 0u16), (OBJ_B, 0), (QUEST_SCRIPT, 1), (MAGIC, 0x100)] {
        form(&mut e, s, 0x11, bit(rtti::SCRIPT));
        e.mem.set_u16(s + 0x28, kind);
    }
    form(&mut e, EFFECT, 0x10, bit(rtti::EFFECT_SETTING));
    e.mem.set_u32(EFFECT + 0x98, 1);
    form(
        &mut e,
        QUEST,
        0x47,
        bit(rtti::TES_QUEST) | bit(rtti::TES_SCRIPTABLE_FORM),
    );
    e.mem.set_u32(QUEST + 0x5c, QUEST_EVENTS);
    form(&mut e, PLAIN, 0x18, 0);
    // The reference's ExtraScript holds its event list.
    form(&mut e, EXTRA, 0x0d, bit(rtti::EXTRA_SCRIPT));
    e.mem.set_u8(EXTRA + 4, 0x0d);
    e.mem.set_u32(EXTRA + 0x10, REF_EVENTS);
    e.mem.set_u32(REF + 0x44 + 0x100 + 4 * 0x0d, EXTRA);
    e.mem.set_u32(BASE + SCRIPT_OF, OBJ_A);
    e.mem.set_u32(QUEST + SCRIPT_OF, QUEST_SCRIPT);
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    for (i, f) in [OBJ_A, OBJ_B, QUEST_SCRIPT, MAGIC, EFFECT, QUEST, PLAIN, REF]
        .into_iter()
        .enumerate()
    {
        set_script_ref(&mut e, i as u16 + 1, f);
    }
    e
}

/// The script refs of [`setup`].
const R_OBJ_A: u16 = 1;
const R_OBJ_B: u16 = 2;
const R_QUEST_SCRIPT: u16 = 3;
const R_MAGIC: u16 = 4;
const R_EFFECT: u16 = 5;
const R_QUEST: u16 = 6;
const R_PLAIN: u16 = 7;
const R_REF: u16 = 8;

fn low(v: f64) -> u32 {
    v.to_bits() as u32
}

// ---- a form's script ----

/// `IsScripted` and `GetScript`: the form's script (the argument, a reference's base form, the
/// caller's base form; a script effect's when the form is not scriptable); nothing without a form.
#[test]
fn a_forms_script_is_read() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x14b6, REF, &[]).1, 1.0);
    assert_eq!(low(call(&mut e, 0x14b7, REF, &[]).1), OBJ_A);
    assert_eq!(low(call(&mut e, 0x14b7, 0, &[r(R_REF)]).1), OBJ_A);
    assert_eq!(low(call(&mut e, 0x14b7, 0, &[r(R_QUEST)]).1), QUEST_SCRIPT);
    assert_eq!(call(&mut e, 0x14b6, 0, &[r(R_PLAIN)]).1, 0.0);
    assert_eq!(call(&mut e, 0x14b6, 0, &[]).1, 0.0);
    // A script effect's script, only for a script effect.
    e.mem.set_u32(EFFECT + 0x60, MAGIC);
    assert_eq!(low(call(&mut e, 0x14b7, 0, &[r(R_EFFECT)]).1), MAGIC);
    e.mem.set_u32(EFFECT + 0x98, 2);
    assert_eq!(call(&mut e, 0x14b6, 0, &[r(R_EFFECT)]).1, 0.0);
    e.mem.set_u32(BASE + SCRIPT_OF, 0);
    assert_eq!(call(&mut e, 0x14b6, REF, &[]).1, 0.0);
    assert!(calls().is_empty());
}

/// `RemoveScript` answers the script's id and takes it away; without an argument it also takes
/// the calling reference's `ExtraScript` away (so it stops at once), with one it does not.
#[test]
fn remove_script_stops_the_callers_script() {
    let mut e = setup();
    assert_eq!(low(call(&mut e, 0x14b8, 0, &[r(R_QUEST)]).1), QUEST_SCRIPT);
    assert_eq!(e.mem.u32(QUEST + SCRIPT_OF), 0);
    assert!(calls().is_empty());
    assert_eq!(low(call(&mut e, 0x14b8, REF, &[]).1), OBJ_A);
    assert_eq!(e.mem.u32(BASE + SCRIPT_OF), 0);
    assert_eq!(calls(), [(0x0041_0140, vec![REF + 0x44, 0x0d])]);
    // Without a script: nothing answered, the extra still taken away.
    assert_eq!(call(&mut e, 0x14b8, REF, &[]).1, 0.0);
    assert_eq!(calls().len(), 1);
    e.mem.set_u32(EFFECT + 0x60, MAGIC);
    assert_eq!(low(call(&mut e, 0x14b8, REF, &[r(R_EFFECT)]).1), MAGIC);
    assert_eq!(e.mem.u32(EFFECT + 0x60), 0);
    assert!(calls().is_empty());
}

/// `SetScript`: an object script on a scriptable form (the old script answered; without the second
/// argument the caller gets a new `ExtraScript` with its own event list); a quest script only on a
/// quest (the script's quest set to it); a magic script only on a script effect; an object script
/// on an effect, or a form that is not a script, nothing.
#[test]
fn set_script_by_kind() {
    let mut e = setup();
    e.mem.set_u32(OBJ_B + 0x40, 0x1234);
    assert_eq!(low(call(&mut e, 0x14b9, REF, &[r(R_OBJ_B)]).1), OBJ_A);
    assert_eq!(e.mem.u32(BASE + SCRIPT_OF), OBJ_B);
    assert_eq!(e.mem.u32(OBJ_B + 0x40), 0, "not a quest");
    let made = calls();
    assert_eq!(made.len(), 3, "{made:x?}");
    assert_eq!(made[0], (0x0041_0140, vec![REF + 0x44, 0x0d]));
    assert_eq!(made[1], (crate::functions::CREATE_EVENT_LIST, vec![OBJ_B]));
    let extra = made[2].1[1];
    assert_eq!(made[2], (0x0040_ff60, vec![REF + 0x44, extra]));
    assert_eq!(e.mem.u32(extra), 0x0101_5914);
    assert_eq!(e.mem.u8(extra + 4), 0x0d);
    assert_eq!(e.mem.u32(extra + 0xc), OBJ_B);
    assert_eq!(e.mem.u32(extra + 0x10), NEW_EVENTS);
    // With the form given, no ExtraScript.
    assert_eq!(
        low(call(&mut e, 0x14b9, REF, &[r(R_OBJ_A), r(R_REF)]).1),
        OBJ_B
    );
    assert!(calls().is_empty());
    // A quest script: on the quest, the script's quest is the quest.
    assert_eq!(
        low(call(&mut e, 0x14b9, 0, &[r(R_QUEST_SCRIPT), r(R_QUEST)]).1),
        QUEST_SCRIPT
    );
    assert_eq!(e.mem.u32(QUEST + SCRIPT_OF), QUEST_SCRIPT);
    assert_eq!(e.mem.u32(QUEST_SCRIPT + 0x40), QUEST);
    // On a form that is not a quest: the old script answered, nothing set.
    assert_eq!(
        low(call(&mut e, 0x14b9, 0, &[r(R_QUEST_SCRIPT), r(R_REF)]).1),
        OBJ_A
    );
    assert_eq!(e.mem.u32(BASE + SCRIPT_OF), OBJ_A);
    // A magic script: not on a scriptable form; on an effect.
    assert_eq!(call(&mut e, 0x14b9, 0, &[r(R_MAGIC), r(R_REF)]).1, 0.0);
    e.mem.set_u32(EFFECT + 0x60, OBJ_B);
    assert_eq!(
        low(call(&mut e, 0x14b9, 0, &[r(R_MAGIC), r(R_EFFECT)]).1),
        OBJ_B
    );
    assert_eq!(e.mem.u32(EFFECT + 0x60), MAGIC);
    // An object script on an effect, a form that is not a script: nothing.
    assert_eq!(call(&mut e, 0x14b9, 0, &[r(R_OBJ_A), r(R_EFFECT)]).1, 0.0);
    assert_eq!(e.mem.u32(EFFECT + 0x60), MAGIC);
    assert_eq!(call(&mut e, 0x14b9, 0, &[r(R_PLAIN), r(R_REF)]).1, 0.0);
    assert_eq!(e.mem.u32(BASE + SCRIPT_OF), OBJ_A);
    assert!(calls().is_empty());
}

// ---- variables by name ----

/// The variables of the caller's script (in its event list) and of a quest's: by name without
/// case; `HasVariable` checks the declaration only; a local missing from the event list reads 0.
#[test]
fn variables_are_read_by_name() {
    let mut e = setup();
    declare(&mut e, OBJ_A, 1, "Count");
    declare(&mut e, OBJ_A, 2, "Target");
    declare(&mut e, OBJ_A, 3, "Missing");
    local(&mut e, REF_EVENTS, 1, 7.5);
    let target = local(&mut e, REF_EVENTS, 2, 0.0);
    e.mem.set_u32(target + 8, PLAIN);
    e.mem.set_u32(target + 12, 0xdead);
    declare(&mut e, QUEST_SCRIPT, 1, "stage");
    local(&mut e, QUEST_EVENTS, 1, 30.0);
    assert_eq!(call(&mut e, 0x152e, REF, &[s("count")]).1, 7.5);
    assert_eq!(call(&mut e, 0x152e, REF, &[s("COUNT")]).1, 7.5);
    assert_eq!(call(&mut e, 0x152e, REF, &[s("missing")]).1, 0.0);
    assert_eq!(call(&mut e, 0x152e, REF, &[s("nothing")]).1, 0.0);
    assert_eq!(call(&mut e, 0x152e, REF, &[s("Stage"), r(R_QUEST)]).1, 30.0);
    assert_eq!(call(&mut e, 0x152e, 0, &[s("count")]).1, 0.0);
    // `GetRefVariable` takes the low word only.
    assert_eq!(
        call(&mut e, 0x1530, REF, &[s("target")]).1.to_bits(),
        u64::from(PLAIN)
    );
    assert_eq!(call(&mut e, 0x152f, REF, &[s("missing")]).1, 1.0);
    assert_eq!(call(&mut e, 0x152f, REF, &[s("nothing")]).1, 0.0);
    assert_eq!(call(&mut e, 0x152f, 0, &[s("count")]).1, 0.0);
    // No event list on the reference: nothing, not even `HasVariable`.
    e.mem.set_u32(EXTRA + 0x10, 0);
    assert_eq!(call(&mut e, 0x152f, REF, &[s("count")]).1, 0.0);
}

/// `GetStringVariable`: the string the variable holds as a new string; "" for a missing variable
/// or a number that names no string; no string at all without a script.
#[test]
fn string_variables_are_read_and_set() {
    let mut e = setup();
    let id = vars::add_string(&mut e, 5, b"hello", false);
    declare(&mut e, OBJ_A, 1, "text");
    declare(&mut e, OBJ_A, 2, "nostring");
    local(&mut e, REF_EVENTS, 1, f64::from(id));
    local(&mut e, REF_EVENTS, 2, 9999.0);
    let got = call(&mut e, 0x167c, REF, &[s("text")]).1;
    assert_ne!(got as u32, id);
    assert_eq!(vars::string(&e, got as u32).unwrap(), b"hello");
    let got = call(&mut e, 0x167c, REF, &[s("other")]).1;
    assert_eq!(vars::string(&e, got as u32).unwrap(), b"");
    let got = call(&mut e, 0x167c, REF, &[s("nostring")]).1;
    assert_eq!(vars::string(&e, got as u32).unwrap(), b"");
    assert_eq!(call(&mut e, 0x167c, 0, &[s("text")]).1, 0.0);
    // `SetStringVariable` sets the string the variable holds; nothing for one that holds none.
    call(&mut e, 0x167d, REF, &[s("TEXT"), s("bye")]);
    assert_eq!(vars::string(&e, id).unwrap(), b"bye");
    call(&mut e, 0x167d, REF, &[s("nostring"), s("x")]);
    assert!(vars::string(&e, 9999).is_none());
}

/// `SetVariable` (a float, widened) and `SetRefVariable` (the id, then a zero word); nothing for a
/// variable that is not declared or has no local.
#[test]
fn variables_are_set_by_name() {
    let mut e = setup();
    declare(&mut e, OBJ_A, 1, "count");
    declare(&mut e, OBJ_A, 2, "target");
    declare(&mut e, QUEST_SCRIPT, 1, "stage");
    declare(&mut e, QUEST_SCRIPT, 2, "nolocal");
    let count = local(&mut e, REF_EVENTS, 1, 0.0);
    let target = local(&mut e, REF_EVENTS, 2, 0.0);
    let stage = local(&mut e, QUEST_EVENTS, 1, 0.0);
    e.mem.set_u32(target + 12, 0xdead);
    call(&mut e, 0x15be, REF, &[s("count"), z(0.1)]);
    assert_eq!(e.mem.f64(count + 8), f64::from(0.1f32));
    call(&mut e, 0x15be, REF, &[s("STAGE"), z(40.0), r(R_QUEST)]);
    assert_eq!(e.mem.f64(stage + 8), 40.0);
    call(&mut e, 0x15be, REF, &[s("nolocal"), z(1.0), r(R_QUEST)]);
    call(&mut e, 0x15bf, REF, &[s("target"), r(R_PLAIN)]);
    assert_eq!(e.mem.f64(target + 8).to_bits(), u64::from(PLAIN));
    call(&mut e, 0x15be, 0, &[s("count"), z(5.0)]);
    assert_eq!(e.mem.f64(count + 8), f64::from(0.1f32));
}

/// `GetArrayVariable` outside an expression: a runtime error, and the line stops (`al` 0).
#[test]
fn get_array_variable_needs_an_expression() {
    let mut e = setup();
    declare(&mut e, OBJ_A, 1, "list");
    local(&mut e, REF_EVENTS, 1, 3.0);
    // Before the first frame the error is not printed (its text names the script).
    e.ext::<crate::Nvse>().main_thread_recorded = false;
    assert_eq!(call(&mut e, 0x1531, REF, &[s("list")]).0, 0);
    // Inside an expression it reads as `GetVariable`.
    let got = crate::expr::with_evaluator(&mut e, |e| call(e, 0x1531, REF, &[s("list")]));
    assert_eq!(got, (1, 3.0));
}

// ---- whole scripts ----

/// `CompareScripts` reads its scripts as inventory objects, so a script never passes the read:
/// always 0, even for the same script.
#[test]
fn compare_scripts_never_reads_a_script() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x1532, 0, &[r(R_OBJ_A), r(R_OBJ_A)]), (1, 0.0));
}

/// `ResetAllVariables`: every local of the caller's event list set to 0, the number answered;
/// without a caller, the calling script's.
#[test]
fn reset_all_variables_on_the_caller_or_the_script() {
    let mut e = setup();
    let a = local(&mut e, REF_EVENTS, 1, 3.0);
    let b = local(&mut e, REF_EVENTS, 2, 4.0);
    assert_eq!(call(&mut e, 0x1533, REF, &[]).1, 2.0);
    assert_eq!([e.mem.f64(a + 8), e.mem.f64(b + 8)], [0.0, 0.0]);
    let own = add_local(&mut e, 1, "x", 9.0);
    assert_eq!(call(&mut e, 0x1533, 0, &[]).1, 1.0);
    assert_eq!(e.mem.f64(own + 8), 0.0);
    // A calling reference without an ExtraScript: nothing reset, the calling script's local kept.
    e.mem.set_f64(own + 8, 9.0);
    e.mem.set_u32(REF + 0x44 + 0x100 + 4 * 0x0d, 0);
    assert_eq!(call(&mut e, 0x1533, REF, &[]).1, 0.0);
    assert_eq!(e.mem.f64(own + 8), 9.0);
}

/// A literal reference (`varIdx` 0) and a reference variable in a script's reference list.
fn ref_entry(e: &mut Engine, script: u32, form: u32, var_idx: u32) {
    let var = space(e, 0x10);
    e.mem.set_u32(var + 8, form);
    e.mem.set_u32(var + 0xc, var_idx);
    push(e, script + 0x44, var);
}

/// `GetNumExplicitRefs` counts the literal references (the form's script, else the caller's); the
/// console prints the count. `GetNthExplicitRef` answers the FIRST literal reference for any
/// index, and nothing for -1 (which never ends in xNVSE).
#[test]
fn explicit_references() {
    let mut e = setup();
    ref_entry(&mut e, OBJ_B, 0, 3);
    ref_entry(&mut e, OBJ_B, QUEST, 0);
    ref_entry(&mut e, OBJ_B, PLAIN, 0);
    ref_entry(&mut e, OBJ_B, REF, 2);
    assert_eq!(call(&mut e, 0x1534, 0, &[r(R_OBJ_B)]).1, 2.0);
    assert_eq!(call(&mut e, 0x1534, REF, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x1534, 0, &[r(R_PLAIN)]).1, 0.0);
    for index in [0, 1, 2, 7] {
        let got = call(&mut e, 0x1535, 0, &[n(index), r(R_OBJ_B)]).1;
        assert_eq!(low(got), QUEST, "index {index}");
    }
    assert_eq!(call(&mut e, 0x1535, 0, &[n(-1), r(R_OBJ_B)]).1, 0.0);
    assert_eq!(call(&mut e, 0x1535, REF, &[n(0)]).1, 0.0);
    console_on(&mut e);
    call(&mut e, 0x1534, 0, &[r(R_OBJ_B)]);
    call(&mut e, 0x1535, 0, &[n(1), r(R_OBJ_B)]);
    assert_eq!(
        lines(),
        [
            "GetNumExplicitRefs >> 2",
            "GetNthExplicitRef >> <no name> (02004000)"
        ]
    );
}

/// `RunScript` runs the form's script once on the caller with no event list (a scriptable form's,
/// a script effect's, or a script given itself) and answers what `Execute` answers (its `al`).
#[test]
fn run_script_runs_it_once() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x1536, REF, &[r(R_QUEST)]).1, 0.0);
    assert_eq!(
        calls(),
        [(
            crate::functions::SCRIPT_EXECUTE,
            vec![QUEST_SCRIPT, REF, 0, 0, 0]
        )]
    );
    call(&mut e, 0x1536, 0, &[r(R_OBJ_B)]);
    assert_eq!(calls()[0].1[..2], [OBJ_B, 0]);
    call(&mut e, 0x1536, 0, &[r(R_PLAIN)]);
    assert!(calls().is_empty());
    e.register(crate::functions::SCRIPT_EXECUTE, |_, _| eax(1));
    assert_eq!(call(&mut e, 0x1536, REF, &[r(R_REF)]).1, 1.0);
}

/// `GetCurrentScript`: the calling script's id in the result's low word.
#[test]
fn current_script() {
    let mut e = setup();
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    assert_eq!(low(call(&mut e, 0x1537, 0, &[]).1), 0x0500_0d00);
}

// ---- the ternary operator ----

fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// An expression command's call: the count, then each expression with its length word; `al`,
/// the result, and where the offset word ended.
fn run_expr(e: &mut Engine, opcode: u32, this_obj: u32, tokens: &[Vec<u8>]) -> (u32, f64, u32) {
    let mut bytes = vec![tokens.len() as u8];
    for t in tokens {
        bytes.extend_from_slice(&(t.len() as u16 + 2).to_le_bytes());
        bytes.extend_from_slice(t);
    }
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    e.mem.write(DATA + 4, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = crate::table::record_of(e, opcode);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    let ok = e
        .call(
            execute,
            &[params, DATA, this_obj, 0, SCRIPT, LIST, RESULT, OFFSET],
        )
        .eax;
    (ok & 0xff, e.mem.f64(RESULT), e.mem.u32(OFFSET))
}

/// `Ternary`: only the branch the condition picks is evaluated, the other stepped over (an
/// expression that cannot be evaluated is harmless there); the offset word ends past all three
/// either way. Not three expressions: 0.
#[test]
fn ternary_evaluates_only_the_branch_it_picks() {
    let mut e = setup();
    // An operator without operands: evaluating it fails.
    let broken = vec![crate::expr::op::ADD];
    let end = 4 + (9 + 2) * 2 + (1 + 2);
    assert_eq!(
        run_expr(&mut e, 0x166e, 0, &[num(1.0), num(5.0), broken.clone()]),
        (1, 5.0, end)
    );
    assert_eq!(
        run_expr(&mut e, 0x166e, 0, &[num(0.0), broken.clone(), num(7.0)]),
        (1, 7.0, end)
    );
    // The picked branch failing: the result stays 0.
    assert_eq!(
        run_expr(&mut e, 0x166e, 0, &[num(1.0), broken, num(7.0)]).1,
        0.0
    );
    assert_eq!(run_expr(&mut e, 0x166e, 0, &[num(1.0), num(5.0)]).1, 0.0);
}

/// A function script `at` (id its address) taking no parameters.
fn function_script(e: &mut Engine, at: u32) {
    let data = at + 0x100;
    let mut d = vec![0x1d, 0, 0, 0, 0x10, 0];
    let block = vec![0x0d, 0, 0, 0, 0, 0, 1, 0];
    d.extend_from_slice(&(block.len() as u16).to_le_bytes());
    d.extend(block);
    d.resize(0x40, 0);
    e.mem.write(data, &d);
    e.mem.set_u32(at + 0x30, data);
    e.mem.set_u32(at + 0x20, d.len() as u32);
}

/// `TernaryUDF`: the first value's truth picks the function (the second or third argument),
/// called on the calling reference; a pick that is not a function calls nothing.
#[test]
fn ternary_udf_calls_the_function_it_picks() {
    let mut e = setup();
    e.register(0x0048_39c0, |_, a| eax(a[0]));
    e.register(crate::functions::EVENT_LIST_DESTROY, |_, _| eax(0));
    crate::functions::install(&mut e);
    function_script(&mut e, OBJ_A);
    function_script(&mut e, OBJ_B);
    let refer = |i: u16| {
        let mut b = vec![b'R'];
        b.extend_from_slice(&i.to_le_bytes());
        b
    };
    let ran = |e: &mut Engine, cond: f64, f: u16, g: u16| {
        calls();
        run_expr(e, 0x1639, REF, &[num(cond), refer(f), refer(g)]);
        calls()
            .into_iter()
            .filter(|(addr, _)| *addr == crate::functions::SCRIPT_EXECUTE)
            .map(|(_, a)| (a[0], a[1]))
            .collect::<Vec<_>>()
    };
    assert_eq!(ran(&mut e, 1.0, R_OBJ_A, R_OBJ_B), [(OBJ_A, REF)]);
    assert_eq!(ran(&mut e, 0.0, R_OBJ_A, R_OBJ_B), [(OBJ_B, REF)]);
    assert!(ran(&mut e, 1.0, R_PLAIN, R_OBJ_B).is_empty());
}
