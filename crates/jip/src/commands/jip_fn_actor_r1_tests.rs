//! `GetWheelDisabled` and `SetWheelDisabled`, called by their opcodes. Expectations from JIP's
//! command bodies (functions_jip/jip_fn_actor.h) and the flag's place (`internal/hooks.h`:
//! `kHookActorFlag1_DisableWheel = 1 << 7`; `patches_game.h` tests it at `[ecx+0x105]`), never
//! from the port's output.

use super::*;
use crate::commands::test_support::*;

const ACTOR: u32 = FORMS + 0x1000;
const PLAIN: u32 = FORMS + 0x1200;
const ACTOR_VT: u32 = FORMS + 0x8000;
const PLAIN_VT: u32 = FORMS + 0x8800;
/// `IS_ACTOR`'s test: the vtable's +0x100 slot is `Actor::IsActor`'s `return true` (`008D0360`).
const RETURN_TRUE: u32 = 0x008d_0360;
const GET: u32 = 0x2728;
const SET: u32 = 0x2729;

fn setup() -> Engine {
    let e = engine();
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR_VT + 0x100, RETURN_TRUE);
    e.mem.set_u32(PLAIN, PLAIN_VT);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0047_c850);
    e
}

#[test]
fn the_flag_is_bit_7_of_jip_flags_1() {
    assert_eq!(DISABLE_WHEEL, 1 << 7);
}

#[test]
fn get_answers_the_flag() {
    let mut e = setup();
    e.mem.set_u8(ACTOR + 0x105, 0x7f);
    assert_eq!(
        call_on(&mut e, GET, ACTOR, &[]).1,
        0.0,
        "every other bit set"
    );
    e.mem.set_u8(ACTOR + 0x105, 0x80);
    assert_eq!(call_on(&mut e, GET, ACTOR, &[]).1, 1.0);
    e.mem.set_u8(PLAIN + 0x105, 0x80);
    assert_eq!(call_on(&mut e, GET, PLAIN, &[]).1, 0.0, "not an actor");
    assert_eq!(call_on(&mut e, GET, 0, &[]).1, 0.0, "no reference");
}

#[test]
fn set_flips_only_when_asked_differs() {
    let mut e = setup();
    e.mem.set_u8(ACTOR + 0x104, 0x11);
    e.mem.set_u8(ACTOR + 0x105, 0x05);
    e.mem.set_u8(ACTOR + 0x106, 0x22);
    let flags1 = |e: &Engine| e.mem.u8(ACTOR + 0x105);
    assert_eq!(call_on(&mut e, SET, ACTOR, &[n(1)]), (1, 0.0), "no result");
    assert_eq!(flags1(&e), 0x85, "set, the other bits kept");
    call_on(&mut e, SET, ACTOR, &[n(9)]);
    assert_eq!(
        flags1(&e),
        0x85,
        "already set: stays (any non-zero disables)"
    );
    call_on(&mut e, SET, ACTOR, &[n(0)]);
    assert_eq!(flags1(&e), 0x05, "cleared");
    call_on(&mut e, SET, ACTOR, &[n(0)]);
    assert_eq!(flags1(&e), 0x05, "already clear: stays");
    call_on(&mut e, SET, ACTOR, &[n(-1)]);
    assert_eq!(flags1(&e), 0x85);
    assert_eq!(e.mem.u8(ACTOR + 0x104), 0x11, "the byte before untouched");
    assert_eq!(e.mem.u8(ACTOR + 0x106), 0x22, "jipActorFlags2 untouched");
    // A line with no argument data (the compiler never makes one for a required parameter):
    // the read succeeds and JIP's output is its uninitialised stack word; the port reads 0.
    call_on(&mut e, SET, ACTOR, &[]);
    assert_eq!(flags1(&e), 0x05);
    // Not an actor: nothing written.
    e.mem.set_u8(PLAIN + 0x105, 0);
    call_on(&mut e, SET, PLAIN, &[n(1)]);
    assert_eq!(e.mem.u8(PLAIN + 0x105), 0);
    assert_eq!(call_on(&mut e, SET, 0, &[n(1)]), (1, 0.0), "no reference");
}
