//! `ToggleNoGunWobble`, called by its opcode. Expectations from JIP's command body
//! (functions_jip/jip_fn_actor.h) and the flag's place (`internal/hooks.h`:
//! `kHookActorFlag2_NoGunWobble = 1 << 3`, tested at `[ecx+0x106]`), never from the port's output.

use super::*;
use crate::commands::test_support::*;

const ACTOR: u32 = FORMS + 0x1000;
const PLAIN: u32 = FORMS + 0x1200;
const ACTOR_VT: u32 = FORMS + 0x8000;
const PLAIN_VT: u32 = FORMS + 0x8800;
const RETURN_TRUE: u32 = 0x008d_0360;
const TOGGLE: u32 = 0x28ec;

fn setup() -> Engine {
    let e = engine();
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR_VT + 0x100, RETURN_TRUE);
    e.mem.set_u32(PLAIN, PLAIN_VT);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0047_c850);
    e
}

#[test]
fn no_gun_wobble_answers_and_flips_bit_8_of_jip_flags_2() {
    let mut e = setup();
    assert_eq!(NO_GUN_WOBBLE, 8);
    e.mem.set_u8(ACTOR + 0x105, 0xaa);
    e.mem.set_u8(ACTOR + 0x106, 0x07);
    e.mem.set_u8(ACTOR + 0x107, 0x55);
    let flags2 = |e: &Engine| e.mem.u8(ACTOR + 0x106);
    assert_eq!(call_on(&mut e, TOGGLE, ACTOR, &[]).1, 0.0, "clear");
    assert_eq!(flags2(&e), 0x07, "no argument: no change");
    assert_eq!(
        call_on(&mut e, TOGGLE, ACTOR, &[n(1)]).1,
        0.0,
        "the state before"
    );
    assert_eq!(flags2(&e), 0x0f, "set, the other bits kept");
    assert_eq!(call_on(&mut e, TOGGLE, ACTOR, &[n(1)]).1, 1.0);
    assert_eq!(flags2(&e), 0x0f, "already set: stays");
    assert_eq!(call_on(&mut e, TOGGLE, ACTOR, &[]).1, 1.0);
    assert_eq!(call_on(&mut e, TOGGLE, ACTOR, &[n(0)]).1, 1.0);
    assert_eq!(flags2(&e), 0x07, "cleared");
    assert_eq!(call_on(&mut e, TOGGLE, ACTOR, &[n(0)]).1, 0.0);
    assert_eq!(flags2(&e), 0x07, "already clear: stays");
    assert_eq!(call_on(&mut e, TOGGLE, ACTOR, &[n(-3)]).1, 0.0);
    assert_eq!(flags2(&e), 0x0f, "any non-zero sets");
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0xaa, "jipActorFlags1 untouched");
    assert_eq!(e.mem.u8(ACTOR + 0x107), 0x55, "jipActorFlags3 untouched");
    // Not an actor: no answer, nothing written.
    e.mem.set_u8(PLAIN + 0x106, 0x08);
    assert_eq!(call_on(&mut e, TOGGLE, PLAIN, &[n(0)]).1, 0.0);
    assert_eq!(e.mem.u8(PLAIN + 0x106), 0x08);
    assert_eq!(call_on(&mut e, TOGGLE, 0, &[n(1)]).1, 0.0, "no reference");
}
