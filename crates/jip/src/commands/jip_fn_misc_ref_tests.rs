//! Tests of `jip_fn_misc_ref`'s two getters, on the state the core track's setters write: a
//! form's `jipFormFlags6` byte (+6, bit 1 `kHookFormFlag6_NoPCActivation`) and the disabled form
//! types (`ui_hooks.rs`' `activation_disabled_types`).

use super::*;
use crate::commands::test_support::*;

const OBJ: u32 = FORMS + 0x100;
const REFR: u32 = FORMS + 0x200;
const VTBL: u32 = FORMS + 0x6000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, OBJ, 0x1f, 0x0000_0101);
    form(&mut e, REFR, 0x3a, 0x0000_0102);
    set_ref(&mut e, 1, OBJ);
    // `kParamType_BoundObject`: xNVSE's reader asks `IsBoundObject`.
    e.mem.set_u32(OBJ, VTBL);
    e.mem
        .set_u32(VTBL + nvse::vtable::IS_BOUND_OBJECT, 0x0050_1000);
    e.register(0x0050_1000, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call_on(e, opcode, this, args).1
}

/// The form given, else the calling reference; only bit 1 of the byte counts.
#[test]
fn interaction_disabled() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x27c8, 0, &[]), 0.0, "no form, no reference");
    assert_eq!(run(&mut e, 0x27c8, REFR, &[]), 0.0);
    e.mem.set_u8(REFR + 6, 0xfd);
    assert_eq!(run(&mut e, 0x27c8, REFR, &[]), 0.0, "every other bit");
    e.mem.set_u8(REFR + 6, 0x02);
    assert_eq!(run(&mut e, 0x27c8, REFR, &[]), 1.0);
    assert_eq!(
        run(&mut e, 0x27c8, REFR, &[r(1)]),
        0.0,
        "the form given, not the reference"
    );
    e.mem.set_u8(OBJ + 6, 0x02);
    e.mem.set_u8(REFR + 6, 0);
    assert_eq!(run(&mut e, 0x27c8, REFR, &[r(1)]), 1.0);
    assert_eq!(run(&mut e, 0x27c8, 0, &[r(1)]), 1.0);
    e.mem.set_u8(OBJ + 5, 0xff);
    e.mem.set_u8(OBJ + 6, 0);
    assert_eq!(run(&mut e, 0x27c8, 0, &[r(1)]), 0.0, "byte 5 is another's");
}

/// A filter type (`kValidFilterForm`) below `kFormType_Max` (121) the setter disabled: 1.
#[test]
fn interaction_disabled_types() {
    let mut e = setup();
    assert!(crate::ln_events::VALID_FILTER_FORM[0x1f]);
    assert!(!crate::ln_events::VALID_FILTER_FORM[0]);
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(0x1f)]), 0.0);
    e.ext::<Jip>().ui_hooks.activation_disabled_types[0x1f] = true;
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(0x1f)]), 1.0);
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(0x1e)]), 0.0);
    // Not a filter type: 0 even when its slot is set.
    e.ext::<Jip>().ui_hooks.activation_disabled_types[0] = true;
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(0)]), 0.0);
    // The last type and past the table.
    let last = 120usize;
    e.ext::<Jip>().ui_hooks.activation_disabled_types[last] = true;
    let want = if crate::ln_events::VALID_FILTER_FORM[last] {
        1.0
    } else {
        0.0
    };
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(120)]), want);
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(121)]), 0.0);
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(-1)]), 0.0);
}

/// Through the core track's setter: the getters read what it wrote.
#[test]
fn reads_what_the_setters_write() {
    let mut e = setup();
    call_on(&mut e, 0x27c9, REFR, &[n(1)]);
    assert_eq!(run(&mut e, 0x27c8, REFR, &[]), 1.0);
    call_on(&mut e, 0x27c9, REFR, &[n(0)]);
    assert_eq!(run(&mut e, 0x27c8, REFR, &[]), 0.0);
    call(&mut e, 0x27cb, &[n(1), n(0x1f)]);
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(0x1f)]), 1.0);
    call(&mut e, 0x27cb, &[n(0), n(0x1f)]);
    assert_eq!(run(&mut e, 0x27ca, 0, &[n(0x1f)]), 0.0);
}
