//! Tests of the terminal commands: the terminal at the exe's offsets (password +0xA0, difficulty
//! +0xA4, flags +0xA5), references with their extras (terminal state 0x50, lock 0x2A with its
//! data's tries at +0xC, teleport 0x2B with its data's linked door at +0), the new extras' exact
//! blocks and `MarkModified` flags.

use super::*;
use crate::commands::test_support::*;

const TERM: u32 = FORMS + 0x1000;
const NOTE: u32 = FORMS + 0x1100;
const DOOR: u32 = FORMS + 0x1200;
const MISC: u32 = FORMS + 0x1300;
const T_REF: u32 = FORMS + 0x2000;
const D_REF: u32 = FORMS + 0x2100;
const D_OTHER: u32 = FORMS + 0x2200;
const C_REF: u32 = FORMS + 0x2300;
const XSTATE: u32 = FORMS + 0x3000;
const XLOCK: u32 = FORMS + 0x3100;
const LOCK_DATA: u32 = FORMS + 0x3200;
const XTELE: u32 = FORMS + 0x3300;
const TELE_DATA: u32 = FORMS + 0x3400;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    form(&mut e, TERM, 0x17, 0x0000_0100);
    form(&mut e, NOTE, 0x31, 0x0000_0101);
    form(&mut e, DOOR, 0x1c, 0x0000_0102);
    form(&mut e, MISC, 0x1f, 0x0000_0103);
    e.mem.set_u8(TERM + 0xa4, 3);
    for (r, base, id) in [
        (T_REF, TERM, 0x200),
        (D_REF, DOOR, 0x201),
        (D_OTHER, DOOR, 0x202),
        (C_REF, MISC, 0x203),
    ] {
        form(&mut e, r, 0x3a, id);
        e.mem.set_u32(r + 0x20, base);
    }
    for (i, f) in [TERM, NOTE, MISC, T_REF, D_REF, C_REF].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn terminal_lock_level() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2263, &[r(1)]).1, 3.0);
    e.mem.set_u8(TERM + 0xa5, 2);
    assert_eq!(call(&mut e, 0x2263, &[r(1)]).1, -1.0, "unlocked");
    // Other flag bits do not count.
    e.mem.set_u8(TERM + 0xa5, 0xfd);
    assert_eq!(call(&mut e, 0x2263, &[r(1)]).1, 3.0);
    // No argument: the calling reference's base form; none at all: nothing.
    assert_eq!(call_on(&mut e, 0x2263, T_REF, &[]).1, 3.0);
    assert_eq!(call(&mut e, 0x2263, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x2263, &[r(3)]).1, 0.0, "not a terminal");
}

#[test]
fn password_note() {
    let mut e = setup();
    call(&mut e, 0x224f, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x2250, &[r(2), r(1)]);
    assert_eq!(e.mem.u32(TERM + 0xa0), NOTE);
    call_on(&mut e, 0x224f, T_REF, &[]);
    assert_eq!(form_result(&e), 0x101);
    // The note must be a note; the terminal a terminal.
    e.mem.set_u32(TERM + 0xa0, 0);
    call(&mut e, 0x2250, &[r(3), r(1)]);
    assert_eq!(e.mem.u32(TERM + 0xa0), 0);
    let before = e.mem.bytes(MISC, 0xa8);
    call(&mut e, 0x2250, &[r(2), r(3)]);
    assert_eq!(e.mem.bytes(MISC, 0xa8), before);
    call_on(&mut e, 0x2250, T_REF, &[r(2)]);
    assert_eq!(e.mem.u32(TERM + 0xa0), NOTE, "the caller's base");
}

#[test]
fn locked_out_read() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x224b, &[r(4)]).1, 0.0, "no state");
    put_extra(&mut e, T_REF + 0x44, XSTATE, 0x50, 0x0000_0012);
    assert_eq!(call(&mut e, 0x224b, &[r(4)]).1, 2.0, "low 4 bits");
    // Form flag 0x100 first.
    e.mem.set_u32(T_REF + 8, 0x100);
    assert_eq!(call(&mut e, 0x224b, &[r(4)]).1, 3.0);
    // A lock's tries.
    put_extra(&mut e, C_REF + 0x44, XLOCK, 0x2a, LOCK_DATA);
    e.mem.set_u32(LOCK_DATA + 0xc, 0x21);
    assert_eq!(call_on(&mut e, 0x224b, C_REF, &[]).1, 1.0);
    // A door without a lock: its linked door's.
    put_extra(&mut e, D_REF + 0x44, XTELE, 0x2b, TELE_DATA);
    e.mem.set_u32(TELE_DATA, C_REF);
    assert_eq!(call(&mut e, 0x224b, &[r(5)]).1, 1.0);
    e.mem.set_u32(TELE_DATA, D_OTHER);
    assert_eq!(call(&mut e, 0x224b, &[r(5)]).1, 0.0);
}

#[test]
fn locked_out_set_on_a_terminal() {
    let mut e = setup();
    // State 0 with no state: nothing made.
    call(&mut e, 0x224c, &[n(0), r(4)]);
    assert!(allocations().is_empty() && calls_to(ADD_CHANGE_FN).is_empty());
    let block = heap_next(&e);
    call(&mut e, 0x224c, &[n(2), r(4)]);
    assert_eq!(allocations(), vec![0x10]);
    assert_eq!(e.mem.u32(block), 0x0101_5190);
    assert_eq!(e.mem.u32(block + 4), 0x50, "the type as a whole word");
    assert_eq!(e.mem.u32(block + 8), 0);
    assert_eq!(e.mem.u8(block + 0xc), 2);
    assert_eq!(e.mem.u8(block + 0xd), 3, "the terminal's difficulty");
    assert_eq!(e.mem.u16(block + 0xe), 0);
    assert_eq!(calls_to(ADD_EXTRA_FN), vec![vec![T_REF + 0x44, block]]);
    assert_eq!(
        calls_to(ADD_CHANGE_FN),
        vec![vec![SAVE_LOAD, T_REF, 0x8000_0000, 0]]
    );
    // Set again: the same state, no new block; 3 is refused.
    call(&mut e, 0x224c, &[n(1), r(4)]);
    assert_eq!(e.mem.u8(block + 0xc), 1);
    assert_eq!(allocations().len(), 1);
    call(&mut e, 0x224c, &[n(3), r(4)]);
    assert_eq!(e.mem.u8(block + 0xc), 1);
}

#[test]
fn locked_out_set_on_locks_and_doors() {
    let mut e = setup();
    put_extra(&mut e, C_REF + 0x44, XLOCK, 0x2a, LOCK_DATA);
    e.mem.set_u32(LOCK_DATA + 0xc, 0xff);
    call(&mut e, 0x224c, &[n(2), r(6)]);
    assert_eq!(e.mem.u32(LOCK_DATA + 0xc), 2, "the whole word");
    assert_eq!(
        calls_to(ADD_CHANGE_FN),
        vec![vec![SAVE_LOAD, C_REF, 0x1000, 0]]
    );
    // A door with no lock, linked to C_REF: C_REF's lock.
    put_extra(&mut e, D_REF + 0x44, XTELE, 0x2b, TELE_DATA);
    e.mem.set_u32(TELE_DATA, C_REF);
    call(&mut e, 0x224c, &[n(1), r(5)]);
    assert_eq!(e.mem.u32(LOCK_DATA + 0xc), 1);
    // Linked to a door with no lock: a new lock on the LINKED door.
    e.mem.set_u32(TELE_DATA, D_OTHER);
    let block = heap_next(&e);
    call(&mut e, 0x224c, &[n(2), r(5)]);
    assert_eq!(allocations(), vec![0x10, 0x14]);
    assert_eq!(e.mem.u32(block), 0x0101_589c);
    assert_eq!(e.mem.u32(block + 4), 0x2a);
    let data = e.mem.u32(block + 0xc);
    assert_eq!(data, block + 0x10);
    assert_eq!(e.mem.u32(data + 0xc), 2);
    assert_eq!(e.mem.bytes(data, 0xc), vec![0; 0xc], "zeroed");
    assert_eq!(e.mem.u32(data + 0x10), 0);
    assert_eq!(calls_to(ADD_EXTRA_FN), vec![vec![D_OTHER + 0x44, block]]);
    // State 0 on a reference with nothing: no lock made.
    let fresh = FORMS + 0x6000;
    form(&mut e, fresh, 0x3a, 0x204);
    e.mem.set_u32(fresh + 0x20, MISC);
    let n_before = allocations().len();
    call_on(&mut e, 0x224c, fresh, &[n(0)]);
    assert!(e.mem.u32(fresh + 0x48) == 0, "no extra added");
    assert_eq!(allocations().len(), n_before);
}
