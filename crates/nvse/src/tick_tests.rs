//! Tests of the tick and the delayed calls (I-8).

use super::*;
use crate::commands::test_support::*;

const AREA: u32 = 0x0400_0000;
const FN_SCRIPT: u32 = AREA;
const FN_DATA: u32 = AREA + 0x100;
/// Counts of the function's runs, and its argument as it last saw it.
const RUNS: u32 = AREA + 0x7f00;
const SEEN: u32 = AREA + 0x7f08;
const LOCALS: u32 = AREA + 0x6000;

/// `Script::Execute`: counts the run and notes local 1.
fn execute(e: &mut Engine, a: &[u32]) -> Ret {
    let n = e.mem.u32(RUNS);
    e.mem.set_u32(RUNS, n + 1);
    let local = crate::args::event_list_variable(e, a[2], 1);
    let v = if local != 0 {
        e.mem.f64(local + 8)
    } else {
        -1.0
    };
    e.mem.set_f64(SEEN, v);
    Ret::default()
}

/// One event list with local 1.
fn create_event_list(e: &mut Engine, a: &[u32]) -> Ret {
    let list = LOCALS;
    e.mem.set_u32(list, a[0]);
    e.mem.set_u32(list + 0xc, LOCALS + 0x20);
    e.mem.set_u32(LOCALS + 0x20, LOCALS + 0x40);
    e.mem.set_u32(LOCALS + 0x24, 0);
    e.mem.set_u32(LOCALS + 0x40, 1);
    Ret {
        eax: list,
        ..Ret::default()
    }
}

fn one(_: &mut Engine, _: &[u32]) -> Ret {
    Ret {
        st0: 1.0,
        ..Ret::default()
    }
}

fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

fn lookup(e: &mut Engine, a: &[u32]) -> Ret {
    let eax = if a[0] != 0 && a[0] == e.mem.u32(FN_SCRIPT + 0xc) {
        FN_SCRIPT
    } else {
        0
    };
    Ret {
        eax,
        ..Ret::default()
    }
}

fn upper(_: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: u32::from((a[0] as u8).to_ascii_uppercase()),
        ..Ret::default()
    }
}

fn setup() -> Engine {
    let mut e = engine_with_table();
    e.map(AREA, 0x1_0000);
    for page in [TIME_GLOBAL, START_MENU, 0x011c_3f2c] {
        e.map(page & !0xfff, 0x1000);
    }
    e.register(0x00ec_a7f4, upper);
    e.register(LOOKUP_FORM, lookup);
    e.register(VATS_TIME_MULT, one);
    e.register(MENU_MODE, nothing);
    e.register(CONSOLE_INSTANCE, nothing);
    e.register(functions::SCRIPT_EXECUTE, execute);
    e.register(functions::CREATE_EVENT_LIST, create_event_list);
    e.register(functions::EVENT_LIST_DESTROY, nothing);
    // A function script taking one float (local 1), `CallAfterSeconds 2 fn 0 7`.
    let mut d = vec![
        0x1d, 0, 0, 0, 0x10, 0, 11, 0, 0x0d, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0,
    ];
    d.resize(0x20, 0);
    e.mem.write(FN_DATA, &d);
    e.mem.set_u32(FN_SCRIPT + 0xc, 0x0500_0e00);
    e.mem.set_u8(FN_SCRIPT + 4, 0x11);
    e.mem.set_u32(FN_SCRIPT + 0x30, FN_DATA);
    e.mem.set_u32(FN_SCRIPT + 0x20, d.len() as u32);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    functions::install(&mut e);
    install(&mut e);
    e
}

fn frame(e: &mut Engine, seconds: f32) {
    e.mem.set_f32(TIME_GLOBAL + 0xc, seconds);
    e.site(TICK.addr(), &[]);
}

fn expression(tokens: &[Vec<u8>]) -> Vec<u8> {
    let body = tokens.concat();
    let mut b = (body.len() as u16 + 2).to_le_bytes().to_vec();
    b.extend(body);
    b
}
fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
/// The function, as reference 1 of the calling script.
fn form(_: u32) -> Vec<u8> {
    vec![b'R', 1, 0]
}

/// Runs an expression-format command with `args` as its expressions.
fn run(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    let mut bytes = vec![args.len() as u8];
    for a in args {
        bytes.extend(expression(std::slice::from_ref(a)));
    }
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    e.mem.write(DATA + 4, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = crate::table::record_of(e, opcode);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    e.call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET]);
    e.mem.f64(RESULT)
}

/// The first frame records the main thread; `CallAfterSeconds 2 fn 0 7` runs once, when the
/// summed game seconds reach 2, with its argument.
#[test]
fn call_after_seconds_runs_once_when_due() {
    let mut e = setup();
    set_script_ref(&mut e, 1, FN_SCRIPT);
    assert_eq!(
        run(&mut e, 0x1641, &[num(2.0), form(0), num(0.0), num(7.0)]),
        1.0
    );
    frame(&mut e, 0.5);
    assert!(e.ext::<Nvse>().main_thread_recorded);
    frame(&mut e, 1.0);
    assert_eq!(e.mem.u32(RUNS), 0);
    frame(&mut e, 0.5);
    assert_eq!(e.mem.u32(RUNS), 1);
    assert_eq!(e.mem.f64(SEEN), 7.0);
    frame(&mut e, 5.0);
    assert_eq!(e.mem.u32(RUNS), 1);
    assert!(e.ext::<Nvse>().tick.after.is_empty());
}

/// `CallAfterFrames 2 fn` runs on the second frame; `CallForSeconds 1 fn` every frame until the
/// second is over.
#[test]
fn frames_and_for_seconds() {
    let mut e = setup();
    set_script_ref(&mut e, 1, FN_SCRIPT);
    run(&mut e, 0x164e, &[num(2.0), form(0)]);
    frame(&mut e, 0.1);
    assert_eq!(e.mem.u32(RUNS), 0);
    frame(&mut e, 0.1);
    assert_eq!(e.mem.u32(RUNS), 1);
    run(&mut e, 0x1642, &[num(1.0), form(0)]);
    for _ in 0..4 {
        frame(&mut e, 0.3);
    }
    // 0.2 + 1.0 = 1.2: frames at 0.5, 0.8, 1.1 run, the one at 1.4 drops it.
    assert_eq!(e.mem.u32(RUNS), 4);
    assert!(e.ext::<Nvse>().tick.for_seconds.is_empty());
}

fn var(var_type: u8, idx: u16) -> Vec<u8> {
    let mut b = vec![b'V', var_type, 0, 0];
    b.extend_from_slice(&idx.to_le_bytes());
    b
}

/// Review item 3: an array argument is held as `SelfOwningArrayElement` holds it (a reference
/// from the array's own mod), so the tick's clean-up before the delayed calls keeps it; the
/// reference goes with the call.
#[test]
fn array_arguments_are_held_until_the_call() {
    let mut e = setup();
    set_script_ref(&mut e, 1, FN_SCRIPT);
    // The function takes an array (type 3) as local 1.
    e.mem.set_u8(FN_DATA + 0x12, 3);
    let arr = crate::vars::create_array(&mut e, crate::vars::KEY_NUMBER, true, 5);
    add_local(&mut e, 7, "aArg", f64::from(arr));
    run(&mut e, 0x1641, &[num(1.0), form(0), num(0.0), var(3, 7)]);
    frame(&mut e, 0.5);
    assert!(
        crate::vars::array_exists(&e, arr),
        "held across the clean-up"
    );
    assert_eq!(crate::vars::array(&e, arr).unwrap().refs, vec![5]);
    frame(&mut e, 0.5);
    assert_eq!(e.mem.u32(RUNS), 1);
    assert_eq!(
        e.mem.f64(SEEN),
        f64::from(arr),
        "the function got the array"
    );
    frame(&mut e, 0.1);
    assert!(
        !crate::vars::array_exists(&e, arr),
        "released with the call"
    );
}

/// Review item 2 (the passes): a function that clears the delayed calls while the tick walks
/// them (a load started from a script) leaves the pass safe.
#[test]
fn a_call_that_clears_the_lists_is_safe() {
    fn clearing(e: &mut Engine, _: &[u32]) -> Ret {
        let n = e.mem.u32(RUNS);
        e.mem.set_u32(RUNS, n + 1);
        clear_delayed_calls(e);
        Ret::default()
    }
    let mut e = setup();
    e.register(functions::SCRIPT_EXECUTE, clearing);
    set_script_ref(&mut e, 1, FN_SCRIPT);
    for _ in 0..3 {
        run(&mut e, 0x164e, &[num(1.0), form(0)]);
        run(&mut e, 0x1641, &[num(0.0), form(0)]);
    }
    frame(&mut e, 0.1);
    assert_eq!(e.mem.u32(RUNS), 1, "the first call cleared the rest");
    assert!(e.ext::<Nvse>().tick.after_frames.is_empty());
}
