//! Tests of the faction commands, at the raw offset of the faction loader (`005FCB70`: the
//! reputation +0x38).

use crate::commands::test_support::*;

const FACT: u32 = FORMS + 0x1000;
const REPU: u32 = FORMS + 0x2000;
const MISC: u32 = FORMS + 0x3000;

#[test]
fn the_reputation() {
    let mut e = engine();
    form(&mut e, FACT, 0x08, 0x0000_fac7);
    form(&mut e, REPU, 0x68, 0x0000_4e90);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [FACT, REPU, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    call(&mut e, 0x2628, &[r(1)]);
    assert_eq!(form_result(&e), 0, "none");
    call(&mut e, 0x2629, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(FACT + 0x38), REPU);
    call(&mut e, 0x2628, &[r(1)]);
    assert_eq!(form_result(&e), 0x4e90);
    // A misc item is refused by the reader (`kParamType_Reputation`): nothing changes.
    call(&mut e, 0x2629, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(FACT + 0x38), REPU);
    call(&mut e, 0x2629, &[r(1)]);
    assert_eq!(e.mem.u32(FACT + 0x38), 0, "cleared");
}
