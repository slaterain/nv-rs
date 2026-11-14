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

// ---- batch 12: reference data ----

mod data {
    use super::super::*;
    use crate::commands::test_support::*;
    use nvse::vars::Data;

    const MISC: u32 = FORMS + 0x100;
    const ACTI: u32 = FORMS + 0x180;
    const REFR: u32 = FORMS + 0x200;
    const REFR2: u32 = FORMS + 0x300;
    const DOOR_BASE: u32 = FORMS + 0x400;
    const OTHER_DOOR: u32 = FORMS + 0x500;
    const PWATER: u32 = FORMS + 0x600;
    const WATER_FORM: u32 = FORMS + 0x680;
    const PLAYER_AT: u32 = FORMS + 0x700;
    const MANAGER: u32 = FORMS + 0x1000;
    const X1: u32 = FORMS + 0x2000;
    const X2: u32 = FORMS + 0x2040;
    const NODES: u32 = FORMS + 0x3000;
    const TDATA: u32 = FORMS + 0x3400;
    const PRIM: u32 = FORMS + 0x3500;
    const VTBL: u32 = FORMS + 0x6000;
    const REF_VTBL: u32 = FORMS + 0x6400;

    fn setup() -> Engine {
        let mut e = engine();
        extra_stand_ins(&mut e);
        e.map(0x011d_8000, 0x1000);
        form(&mut e, MISC, 0x1f, 0x0000_0101);
        form(&mut e, ACTI, 0x15, 0x0000_0181);
        form(&mut e, REFR, 0x3a, 0x0000_0102);
        form(&mut e, REFR2, 0x3b, 0x0000_0103);
        form(&mut e, DOOR_BASE, 0x1c, 0x0000_0104);
        form(&mut e, OTHER_DOOR, 0x3a, 0x0000_0105);
        form(&mut e, PWATER, 0x23, 0x0000_0106);
        form(&mut e, WATER_FORM, 0x4e, 0x0000_0107);
        e.mem.set_u32(REFR + 0x20, MISC);
        e.mem.set_u32(REFR2 + 0x20, ACTI);
        // References' vtables answer `IsReference` (the reader's object reference check).
        e.mem.set_u32(REF_VTBL + 0xf0, 0x008d_0360);
        for f in [REFR, REFR2, OTHER_DOOR] {
            e.mem.set_u32(f, REF_VTBL);
        }
        e.mem.set_u32(WATER_FORM, VTBL);
        e.mem.set_u32(VTBL + 0x130, 0x0050_3000);
        e.register(0x0050_3000, |_, _| Ret::default());
        set_ref(&mut e, 1, MISC);
        set_ref(&mut e, 2, REFR2);
        set_ref(&mut e, 3, ACTI);
        e.mem.set_u32(0x011d_ea3c, PLAYER_AT);
        e.mem.set_u32(0x011d_8a80, MANAGER);
        e
    }

    fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
        call_on(e, opcode, this, args).1
    }

    fn node(e: &mut Engine, at: u32, data: u32, next: u32) {
        e.mem.set_u32(at, data);
        e.mem.set_u32(at + 4, next);
    }

    #[test]
    fn is_item() {
        let mut e = setup();
        assert_eq!(run(&mut e, 0x262d, 0, &[r(1)]), 1.0);
        assert_eq!(run(&mut e, 0x262d, REFR, &[]), 1.0, "the reference's base");
        assert_eq!(run(&mut e, 0x262d, REFR2, &[]), 0.0, "an activator base");
        assert_eq!(run(&mut e, 0x262d, 0, &[]), 0.0, "neither");
        // The reader takes only inventory objects: an activator is refused, nothing written.
        e.mem.set_f64(RESULT, 7.0);
        assert_eq!(call_on(&mut e, 0x262d, REFR, &[r(3)]).1, 0.0);
    }

    #[test]
    fn ref_type_and_its_echo() {
        let mut e = setup();
        assert_eq!(run(&mut e, 0x2775, REFR, &[]), 58.0);
        assert_eq!(
            run(&mut e, 0x2775, REFR, &[r(2)]),
            59.0,
            "the reference given"
        );
        assert_eq!(run(&mut e, 0x2775, 0, &[]), 0.0);
        let (res, lines) = echoed(&mut e, 0x2775, REFR, &[]);
        assert_eq!((res, lines), (58.0, vec![b"GetRefType >> 58".to_vec()]));
        let (_, lines) = echoed(&mut e, 0x2775, 0, &[]);
        assert!(lines.is_empty(), "no reference: no echo");
    }

    /// Lists with an empty first node, empty items skipped, the array only when there is one.
    #[test]
    fn teammates_and_children() {
        let mut e = setup();
        assert_eq!(run(&mut e, 0x26ce, 0, &[]), 0.0, "an empty list: no array");
        node(&mut e, PLAYER_AT + 0x5fc, 0, NODES);
        node(&mut e, NODES, REFR2, NODES + 8);
        node(&mut e, NODES + 8, 0, NODES + 0x10);
        node(&mut e, NODES + 0x10, REFR, 0);
        let id = run(&mut e, 0x26ce, 0, &[]);
        assert_eq!(
            array_data(&e, id),
            vec![Data::Form(0x103), Data::Form(0x102)]
        );
        // Enable children (0x38) and linked children (0x52) of the calling reference.
        assert_eq!(run(&mut e, 0x2635, REFR, &[]), 0.0);
        put_extra(&mut e, REFR + 0x44, X1, 0x38, 0);
        e.mem.set_u32(X1 + 0x10, 0);
        assert_eq!(run(&mut e, 0x2635, REFR, &[]), 0.0, "empty children");
        node(&mut e, X1 + 0xc, OTHER_DOOR, 0);
        let id = run(&mut e, 0x2635, REFR, &[]);
        assert_eq!(array_data(&e, id), vec![Data::Form(0x105)]);
        assert_eq!(run(&mut e, 0x2636, REFR, &[]), 0.0, "no linked children");
        put_extra(&mut e, REFR + 0x44, X2, 0x52, REFR);
        node(&mut e, X2 + 0xc, REFR, NODES + 8);
        let id = run(&mut e, 0x2636, REFR, &[]);
        assert_eq!(
            array_data(&e, id),
            vec![Data::Form(0x102), Data::Form(0x102)]
        );
    }

    #[test]
    fn teleport_door() {
        let mut e = setup();
        let door = |e: &mut Engine| {
            e.mem.set_u32(RESULT, 0);
            call_on(e, 0x27f9, REFR, &[]);
            form_result(e)
        };
        put_extra(&mut e, REFR + 0x44, X1, 0x2b, TDATA);
        e.mem.set_u32(TDATA, OTHER_DOOR);
        assert_eq!(door(&mut e), 0, "not a door");
        e.mem.set_u32(REFR + 0x20, DOOR_BASE);
        assert_eq!(door(&mut e), 0x105);
        e.mem.set_u32(TDATA, 0);
        assert_eq!(door(&mut e), 0, "no linked door");
        e.mem.set_u32(X1 + 0xc, 0);
        assert_eq!(door(&mut e), 0, "no data");
        e.mem.set_u32(REFR + 0x20, 0);
        assert_eq!(door(&mut e), 0, "no base");
    }

    /// The flag (bit 22) is the result; it flips only for a game-made (0xFF), non-persistent
    /// reference whose flag differs from the argument (compared as a whole: 2 differs from 1),
    /// marking it changed with 1.
    #[test]
    fn purge_on_unload() {
        let mut e = setup();
        assert_eq!(run(&mut e, 0x2931, REFR, &[]), 0.0);
        run(&mut e, 0x2931, REFR, &[n(1)]);
        assert_eq!(e.mem.u32(REFR + 8), 0, "not made in the game");
        e.mem.set_u8(REFR + 0xf, 0xff);
        e.mem.set_u32(REFR + 8, 0x400);
        run(&mut e, 0x2931, REFR, &[n(1)]);
        assert_eq!(e.mem.u32(REFR + 8), 0x400, "persistent");
        e.mem.set_u32(REFR + 8, 0x8);
        assert_eq!(run(&mut e, 0x2931, REFR, &[n(1)]), 0.0, "the old state");
        assert_eq!(e.mem.u32(REFR + 8), 0x40_0008);
        assert_eq!(calls_to(ADD_CHANGE_FN), vec![vec![SAVE_LOAD, REFR, 1, 0]]);
        assert_eq!(run(&mut e, 0x2931, REFR, &[n(1)]), 1.0);
        assert_eq!(e.mem.u32(REFR + 8), 0x40_0008, "the same: nothing");
        run(&mut e, 0x2931, REFR, &[n(2)]);
        assert_eq!(e.mem.u32(REFR + 8), 0x8, "2 is not 1: flipped off");
        run(&mut e, 0x2931, REFR, &[n(0)]);
        assert_eq!(e.mem.u32(REFR + 8), 0x8);
        assert_eq!(calls_to(ADD_CHANGE_FN).len(), 2);
    }

    #[test]
    fn primitives() {
        let mut e = setup();
        let bound = |e: &mut Engine, axis: u8| call_on(e, 0x2679, REFR, &[axis_arg(axis)]).1;
        assert_eq!(run(&mut e, 0x26f1, REFR, &[]), 0.0);
        assert_eq!(bound(&mut e, b'X'), -1.0);
        put_extra(&mut e, REFR + 0x44, X1, 0x6b, 0);
        assert_eq!(run(&mut e, 0x26f1, REFR, &[]), 0.0, "no primitive");
        assert_eq!(bound(&mut e, b'Y'), -1.0);
        e.mem.set_u32(X1 + 0xc, PRIM);
        e.mem.set_f32(PRIM + 0x18, 1.5);
        e.mem.set_f32(PRIM + 0x1c, 0.1);
        e.mem.set_f32(PRIM + 0x20, -3.0);
        assert_eq!(run(&mut e, 0x26f1, REFR, &[]), 1.0);
        assert_eq!(bound(&mut e, b'X'), 3.0);
        assert_eq!(bound(&mut e, b'Y'), f64::from(0.1f32) * 2.0);
        assert_eq!(bound(&mut e, b'Z'), -6.0);
    }

    /// An axis argument as the compiler writes it (the letter).
    fn axis_arg(axis: u8) -> Vec<u8> {
        vec![axis]
    }

    #[test]
    fn crosshair_water() {
        let mut e = setup();
        let water = |e: &mut Engine| {
            e.mem.set_u32(RESULT, 0);
            call(e, 0x2813, &[]);
            form_result(e)
        };
        assert_eq!(water(&mut e), 0);
        e.mem.set_u32(MANAGER + 0xfc, REFR);
        assert_eq!(water(&mut e), 0, "not placeable water");
        e.mem.set_u32(REFR + 0x20, PWATER);
        assert_eq!(water(&mut e), 0, "no water form");
        e.mem.set_u32(PWATER + 0x4c, WATER_FORM);
        assert_eq!(water(&mut e), 0x107);
        let (_, lines) = echoed(&mut e, 0x2813, 0, &[]);
        assert_eq!(lines, vec![b"GetCrosshairWater >> 107".to_vec()]);
        e.mem.set_u32(MANAGER + 0xfc, 0);
        let (_, lines) = echoed(&mut e, 0x2813, 0, &[]);
        assert_eq!(lines, vec![b"GetCrosshairWater >> 0".to_vec()]);
    }
}
