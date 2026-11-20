//! Tests of `ReloadCharController`, called by its opcode. Expectations from JIP's command body
//! (functions_jip/jip_fn_actor.h); the virtual slots are stand-ins that record their words in
//! order.

use crate::commands::test_support::*;
use engine_core::prelude::*;

const RELOAD_CHAR_CONTROLLER: u32 = 0x28dd;
const ACTOR: u32 = FORMS + 0x1000;
const PROCESS: u32 = FORMS + 0x2000;
const ACTOR_VT: u32 = FORMS + 0x8000;
const PLAIN_VT: u32 = FORMS + 0x8800;
const PROCESS_VT: u32 = FORMS + 0x9000;
/// `IS_ACTOR`'s answer (vtable +0x100).
const RETURN_TRUE: u32 = 0x008d_0360;
/// Stand-ins in the vtables: the process's +0x290 and the actor's +0x1C4.
const SET_CONTROLLER_FN: u32 = 0x0090_0400;
const INIT_HAVOK_FN: u32 = 0x0090_0410;

fn rec(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn setup() -> Engine {
    let mut e = engine();
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR_VT + 0x100, RETURN_TRUE);
    e.mem.set_u32(ACTOR_VT + 0x1c4, INIT_HAVOK_FN);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0090_0500);
    e.mem.set_u32(ACTOR + 0xc, 0x1234);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS, PROCESS_VT);
    e.mem.set_u32(PROCESS_VT + 0x290, SET_CONTROLLER_FN);
    e.mem.set_u32(PROCESS + 0x28, 0);
    e.register(SET_CONTROLLER_FN, |_, w| {
        rec(SET_CONTROLLER_FN, w);
        Ret::default()
    });
    e.register(INIT_HAVOK_FN, |_, w| {
        rec(INIT_HAVOK_FN, w);
        Ret::default()
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32) {
    CALLS.with(|c| c.borrow_mut().clear());
    call_on(e, opcode, this, &[]);
}

fn order() -> Vec<(u32, Vec<u32>)> {
    calls()
}

#[test]
fn reload_char_controller_drops_the_controller_then_sets_up_havok() {
    let mut e = setup();
    for level in [0, 1] {
        e.mem.set_u32(PROCESS + 0x28, level);
        run(&mut e, RELOAD_CHAR_CONTROLLER, ACTOR);
        assert_eq!(
            order(),
            vec![
                (SET_CONTROLLER_FN, vec![PROCESS, 0]),
                (INIT_HAVOK_FN, vec![ACTOR]),
            ]
        );
    }
    // A low process (2, 3), none, not an actor: nothing.
    for level in [2, 3, 0xffff_ffff] {
        e.mem.set_u32(PROCESS + 0x28, level);
        run(&mut e, RELOAD_CHAR_CONTROLLER, ACTOR);
        assert!(order().is_empty());
    }
    e.mem.set_u32(PROCESS + 0x28, 0);
    e.mem.set_u32(ACTOR + 0x68, 0);
    run(&mut e, RELOAD_CHAR_CONTROLLER, ACTOR);
    assert!(order().is_empty());
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(ACTOR, PLAIN_VT);
    run(&mut e, RELOAD_CHAR_CONTROLLER, ACTOR);
    assert!(order().is_empty());
}
