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

// ---- lane E: persistence, primitives, moves, animations, positions and angles ----

mod lane_e {
    use super::super::*;
    use crate::commands::test_support::*;
    use crate::test_support::LIST;

    const MISC: u32 = FORMS + 0x100;
    const ACTI: u32 = FORMS + 0x180;
    const REFR: u32 = FORMS + 0x200;
    const DEST: u32 = FORMS + 0x300;
    const CONT: u32 = FORMS + 0x380;
    const CELL_AT: u32 = FORMS + 0x400;
    const WORLD_AT: u32 = FORMS + 0x480;
    const PROC: u32 = FORMS + 0x800;
    const ANIM: u32 = FORMS + 0xc00;
    const ANIM2: u32 = FORMS + 0xc80;
    const LOADED: u32 = FORMS + 0xd00;
    const NODE: u32 = FORMS + 0xe00;
    const X1: u32 = FORMS + 0x2000;
    const X2: u32 = FORMS + 0x2040;
    const PRIM: u32 = FORMS + 0x3500;
    const HUD: u32 = FORMS + 0x4000;
    const REF_VTBL: u32 = FORMS + 0x6400;
    const ACTOR_VTBL: u32 = FORMS + 0x6800;
    const CONT_VTBL: u32 = FORMS + 0x6c00;
    /// Stand-ins: `GetAnimData` answers [`ANIM`], the flags test the word at [`FLAGS_ANSWER`].
    const GET_ANIM_DATA_FN: u32 = 0x0050_7000;
    const DESTROY_FN: u32 = 0x0050_7100;
    const FLAGS_TEST_FN: u32 = 0x0041_b3a0;
    const FLAGS_ANSWER: u32 = FORMS + 0x7000;
    const LOCALS: u32 = LIST + 0x100;

    const SET_PERSISTS_FN: u32 = 0x0056_5480;
    const FADE_FN: u32 = 0x008f_e9e0;
    const MOVE_TO_FN: u32 = 0x005c_cb20;
    const HUD_INFO_FN: u32 = 0x0077_5a00;
    const PLAYER_ADD_FN: u32 = 0x0095_3ff0;
    const PICKED_UP_FN: u32 = 0x0057_2230;
    const CONT_ADD_FN: u32 = 0x0057_4b30;
    const SET_3D_FN: u32 = 0x0057_02e0;
    const QUEUE_FN: u32 = 0x0044_4850;

    /// A recorded stand-in answering 0 at `$addr`.
    macro_rules! rec {
        ($e:expr, $addr:expr) => {
            $e.register($addr, |_, w| {
                CALLS.with(|c| c.borrow_mut().push(($addr, w.to_vec())));
                Ret::default()
            })
        };
    }

    fn setup() -> Engine {
        let mut e = engine();
        extra_stand_ins(&mut e);
        e.map(0x011d_9000, 0x1000);
        e.map(0x011c_3000, 0x1000);
        form(&mut e, MISC, 0x1f, 0x0000_0101);
        form(&mut e, ACTI, 0x15, 0x0000_0181);
        form(&mut e, REFR, 0x3a, 0x0000_0102);
        form(&mut e, DEST, 0x3a, 0x0000_0103);
        form(&mut e, CONT, 0x1b, 0x0000_0104);
        form(&mut e, CELL_AT, 0x39, 0x0000_0105);
        form(&mut e, WORLD_AT, 0x41, 0x0000_0106);
        e.mem.set_u32(REFR + 0x20, MISC);
        e.mem.set_u32(DEST + 0x20, CONT);
        for vt in [REF_VTBL, ACTOR_VTBL] {
            e.mem.set_u32(vt + 0xf0, 0x008d_0360);
            e.mem.set_u32(vt + 0x1e4, GET_ANIM_DATA_FN);
            e.mem.set_u32(vt + 0x10, DESTROY_FN);
        }
        e.mem.set_u32(ACTOR_VTBL + 0x100, 0x008d_0360);
        for f in [REFR, DEST] {
            e.mem.set_u32(f, REF_VTBL);
        }
        e.mem.set_u32(CONT, CONT_VTBL);
        set_ref(&mut e, 1, DEST);
        e.register(GET_ANIM_DATA_FN, |_, w| {
            CALLS.with(|c| c.borrow_mut().push((GET_ANIM_DATA_FN, w.to_vec())));
            Ret {
                eax: ANIM,
                ..Ret::default()
            }
        });
        e.register(FLAGS_TEST_FN, |e, w| {
            CALLS.with(|c| c.borrow_mut().push((FLAGS_TEST_FN, w.to_vec())));
            Ret {
                eax: e.mem.u32(FLAGS_ANSWER),
                ..Ret::default()
            }
        });
        rec!(e, SET_PERSISTS_FN);
        rec!(e, FADE_FN);
        rec!(e, MOVE_TO_FN);
        rec!(e, HUD_INFO_FN);
        rec!(e, PLAYER_ADD_FN);
        rec!(e, PICKED_UP_FN);
        rec!(e, CONT_ADD_FN);
        rec!(e, DESTROY_FN);
        rec!(e, SET_3D_FN);
        rec!(e, QUEUE_FN);
        e
    }

    fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
        call_on(e, opcode, this, args).1
    }

    /// Script variables `1..=n` in the event list, each 0x10 bytes (id, data at +8), set to -1.
    fn locals(e: &mut Engine, n: u32) {
        for i in 0..n {
            let local = LOCALS + 0x100 + i * 0x10;
            e.mem.set_u32(local, i + 1);
            e.mem.set_f64(local + 8, -1.0);
            let node = LOCALS + i * 8;
            e.mem.set_u32(node, local);
            e.mem
                .set_u32(node + 4, if i + 1 < n { node + 8 } else { 0 });
        }
        e.mem.set_u32(LIST + 0xc, LOCALS);
    }

    fn var(e: &Engine, id: u32) -> f64 {
        e.mem.f64(LOCALS + 0x100 + (id - 1) * 0x10 + 8)
    }

    fn vars(e: &Engine, n: u32) -> Vec<f64> {
        (1..=n).map(|i| var(e, i)).collect()
    }

    /// A script variable argument (`f` and its id).
    fn v(id: u16) -> Vec<u8> {
        let mut b = vec![b'f'];
        b.extend_from_slice(&id.to_le_bytes());
        b
    }

    fn set3(e: &mut Engine, at: u32, v: [f32; 3]) {
        for (i, x) in v.into_iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, x);
        }
    }

    fn get3(e: &Engine, at: u32) -> [f32; 3] {
        [e.mem.f32(at), e.mem.f32(at + 4), e.mem.f32(at + 8)]
    }

    fn set_matrix(e: &mut Engine, at: u32, m: [f32; 9]) {
        for (i, x) in m.into_iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, x);
        }
    }

    /// The game's switch only when the argument (zero or not) differs from the flag (0x400).
    #[test]
    fn persistent() {
        let mut e = setup();
        run(&mut e, 0x2600, REFR, &[n(0)]);
        assert!(
            calls_to(SET_PERSISTS_FN).is_empty(),
            "already not persistent"
        );
        run(&mut e, 0x2600, REFR, &[n(5)]);
        assert_eq!(calls_to(SET_PERSISTS_FN), vec![vec![REFR, 1]]);
        e.mem.set_u32(REFR + 8, 0x400);
        run(&mut e, 0x2600, REFR, &[n(1)]);
        assert_eq!(calls_to(SET_PERSISTS_FN).len(), 1, "already persistent");
        run(&mut e, 0x2600, REFR, &[n(0)]);
        assert_eq!(calls_to(SET_PERSISTS_FN)[1], vec![REFR, 0]);
        e.mem.set_u32(REFR + 8, 0xffff_fbff);
        run(&mut e, 0x2600, REFR, &[n(0)]);
        assert_eq!(calls_to(SET_PERSISTS_FN).len(), 2, "only bit 0x400 counts");
    }

    /// -1 without a primitive; the old size (twice the half) as the result, the new half-size on
    /// the axis or, for a sphere (type 2), on all three.
    #[test]
    fn primitive_bound_set() {
        let mut e = setup();
        let set =
            |e: &mut Engine, axis: u8, val: f64| call_on(e, 0x267a, REFR, &[vec![axis], z(val)]).1;
        assert_eq!(set(&mut e, b'X', 4.0), -1.0);
        put_extra(&mut e, REFR + 0x44, X1, 0x6b, 0);
        assert_eq!(set(&mut e, b'X', 4.0), -1.0, "no primitive");
        e.mem.set_u32(X1 + 0xc, PRIM);
        e.mem.set_u32(PRIM + 4, 1);
        set3(&mut e, PRIM + 0x18, [1.5, 0.1, -3.0]);
        assert_eq!(set(&mut e, b'Y', 5.0), f64::from(0.1f32) * 2.0);
        assert_eq!(get3(&e, PRIM + 0x18), [1.5, 2.5, -3.0]);
        assert_eq!(set(&mut e, b'Z', 1.0), -6.0);
        assert_eq!(e.mem.f32(PRIM + 0x20), 0.5);
        e.mem.set_u32(PRIM + 4, 3);
        assert_eq!(set(&mut e, b'X', 9.0), 3.0);
        assert_eq!(get3(&e, PRIM + 0x18), [4.5, 2.5, 0.5], "a plane: one axis");
        e.mem.set_u32(PRIM + 4, 2);
        assert_eq!(set(&mut e, b'X', 7.0), 9.0);
        assert_eq!(
            get3(&e, PRIM + 0x18),
            [3.5, 3.5, 3.5],
            "a sphere: all three"
        );
        assert_eq!(set(&mut e, b'W', 1.0), -1.0, "not an axis: nothing");
        assert_eq!(get3(&e, PRIM + 0x18), [3.5, 3.5, 3.5]);
        assert!(calls_to(SET_3D_FN).is_empty(), "no 3D: no reload");
    }

    /// The 3D reload runs for a reference with 3D (`overrides::ref_update_3d`: the game's
    /// `Set3D` and the model loader's queue).
    #[test]
    fn primitive_bound_reloads_3d() {
        let mut e = setup();
        put_extra(&mut e, REFR + 0x44, X1, 0x6b, PRIM);
        e.mem.set_u32(PRIM + 4, 1);
        e.mem.set_u32(REFR + 0x64, LOADED);
        e.mem.set_u32(LOADED + 0x14, NODE);
        call_on(&mut e, 0x267a, REFR, &[vec![b'X'], z(2.0)]);
        assert_eq!(calls_to(SET_3D_FN), vec![vec![REFR, 0, 1]]);
        assert_eq!(calls_to(QUEUE_FN).len(), 1);
        assert_eq!(e.mem.u8(REFR + 0x5f) & 1, 1);
    }

    fn added() -> u32 {
        calls_to(ADD_EXTRA_FN).last().map_or(0, |w| w[1])
    }

    fn clear_extras(e: &mut Engine) {
        e.mem.set_u32(REFR + 0x44 + 4, 0);
        e.mem.set_u8(REFR + 0x44 + 8 + 0x6b / 8, 0);
    }

    /// Types 1 to 3 on base 0x21 or an activator (not a plane), no primitive yet: the extra added
    /// with no primitive, then the zeroed primitive of the type's size, vtable, type and halves.
    #[test]
    fn primitive_added() {
        let mut e = setup();
        let add = |e: &mut Engine, kind: i32| {
            call_on(e, 0x26f2, REFR, &[n(kind), z(2.0), z(4.0), z(6.0)]).1
        };
        assert_eq!(add(&mut e, 1), 0.0, "a misc item base");
        e.mem.set_u32(REFR + 0x20, ACTI);
        assert_eq!(add(&mut e, 0), 0.0, "type 0");
        assert_eq!(add(&mut e, 4), 0.0, "type 4");
        assert_eq!(add(&mut e, 3), 0.0, "an activator's plane");
        assert!(calls_to(ADD_EXTRA_FN).is_empty());
        let before = heap_next(&e);
        assert_eq!(add(&mut e, 1), 1.0);
        let x = added();
        assert_eq!((e.mem.u32(x), e.mem.u8(x + 4)), (0x0101_51b4, 0x6b));
        let p = e.mem.u32(x + 0xc);
        assert_eq!(
            p,
            before + 0x10,
            "the extra's block first, then the primitive's"
        );
        assert_eq!((e.mem.u32(p), e.mem.u32(p + 4)), (0x0101_e8c4, 1));
        assert_eq!(get3(&e, p + 0x18), [1.0, 2.0, 3.0]);
        for off in (0x8..0x18).chain(0x24..0x4c).step_by(4) {
            assert_eq!(e.mem.u32(p + off), 0, "zeroed at {off:#x}");
        }
        assert_eq!(
            allocations().last().copied(),
            Some(0x4c),
            "a box: 0x4C bytes"
        );
        assert_eq!(add(&mut e, 2), 0.0, "already has one");
        // A sphere and a plane on base 0x21: the free heap filled with a sentinel first, so the
        // zeroing is seen to cover exactly the 0x34-byte block. The stand-in allocator fills the
        // block it hands out with 0xCD up to the next 8 bytes, so p+0x34 (the first word past
        // the block) keeps 0xCDCDCDCD and the words after it keep the sentinel.
        const SENTINEL: u32 = 0xa5a5_a5a5;
        let fill = |e: &mut Engine| {
            let at = heap_next(e);
            for i in (0..0x100).step_by(4) {
                e.mem.set_u32(at + i, SENTINEL);
            }
        };
        let zeroed_block = |e: &Engine, p: u32, what: &str| {
            for off in (0x8..0x18).chain(0x24..0x34).step_by(4) {
                assert_eq!(e.mem.u32(p + off), 0, "{what}: zeroed at {off:#x}");
            }
            assert_eq!(e.mem.u32(p + 0x34), 0xcdcd_cdcd, "{what}: p+0x34 untouched");
            for off in (0x38..0x50).step_by(4) {
                assert_eq!(e.mem.u32(p + off), SENTINEL, "{what}: untouched at {off:#x}");
            }
        };
        form(&mut e, MISC, 0x1f, 0x21);
        e.mem.set_u32(REFR + 0x20, MISC);
        clear_extras(&mut e);
        fill(&mut e);
        assert_eq!(add(&mut e, 2), 1.0);
        let p = e.mem.u32(added() + 0xc);
        assert_eq!((e.mem.u32(p), e.mem.u32(p + 4)), (0x0101_ea64, 2));
        assert_eq!(get3(&e, p + 0x18), [1.0, 1.0, 1.0]);
        assert_eq!(allocations().last().copied(), Some(0x34));
        zeroed_block(&e, p, "a sphere");
        clear_extras(&mut e);
        fill(&mut e);
        assert_eq!(add(&mut e, 3), 1.0);
        let p = e.mem.u32(added() + 0xc);
        assert_eq!((e.mem.u32(p), e.mem.u32(p + 4)), (0x0101_e75c, 3));
        assert_eq!(get3(&e, p + 0x18), [1.0, 1.0, 3.0], "a plane's middle is 1");
        assert_eq!(allocations().last().copied(), Some(0x34));
        zeroed_block(&e, p, "a plane");
        // A sphere on an activator: only the plane is refused there.
        e.mem.set_u32(REFR + 0x20, ACTI);
        clear_extras(&mut e);
        assert_eq!(add(&mut e, 2), 1.0, "an activator's sphere");
        let p = e.mem.u32(added() + 0xc);
        assert_eq!((e.mem.u32(p), e.mem.u32(p + 4)), (0x0101_ea64, 2));
        e.mem.set_u32(REFR + 0x20, 0);
        clear_extras(&mut e);
        assert_eq!(add(&mut e, 1), 0.0, "no base");
    }

    /// An actor with a high process (level 0): the delay stored and the game's fade; else the
    /// game's plain move.
    #[test]
    fn fade_delay() {
        let mut e = setup();
        run(&mut e, 0x280e, REFR, &[r(1), z(2.5)]);
        assert_eq!(calls_to(MOVE_TO_FN), vec![vec![REFR, DEST, 0, 0, 0]]);
        e.mem.set_u32(REFR, ACTOR_VTBL);
        run(&mut e, 0x280e, REFR, &[r(1), z(2.5)]);
        assert_eq!(calls_to(MOVE_TO_FN).len(), 2, "an actor with no process");
        e.mem.set_u32(REFR + 0x68, PROC);
        e.mem.set_u32(PROC + 0x28, 1);
        run(&mut e, 0x280e, REFR, &[r(1), z(2.5)]);
        assert_eq!(calls_to(MOVE_TO_FN).len(), 3, "a low process");
        assert_eq!(e.mem.f32(PROC + 0x3ec), 0.0);
        e.mem.set_u32(PROC + 0x28, 0);
        run(&mut e, 0x280e, REFR, &[r(1), z(2.5)]);
        assert_eq!(calls_to(MOVE_TO_FN).len(), 3);
        assert_eq!(e.mem.f32(PROC + 0x3ec), 2.5);
        assert_eq!(calls_to(FADE_FN), vec![vec![PROC, REFR, DEST, 0, 0, 0]]);
    }

    fn groups(e: &mut Engine, at: u32, ids: [u16; 8]) {
        for (i, id) in ids.into_iter().enumerate() {
            e.mem.set_u16(at + 0x4c + 2 * i as u32, id);
        }
    }

    /// JIP's table by the low byte (245 and above skipped); category 4 and up ignore sub-type and
    /// flags; 0 sub-type or flags match any.
    #[test]
    fn anim_playing() {
        let mut e = setup();
        let play = |e: &mut Engine, args: &[Vec<u8>]| run(e, 0x283f, REFR, args);
        // 0x0107 = group 7: {2, 1, 1}; 0xF5 = 245: skipped; group 0: {1, 1, 0}.
        groups(&mut e, ANIM, [0xf5, 0x0107, 0, 0, 0, 0, 0, 0]);
        assert_eq!(play(&mut e, &[n(2)]), 1.0);
        assert_eq!(play(&mut e, &[n(2), n(1)]), 1.0);
        assert_eq!(play(&mut e, &[n(2), n(2)]), 0.0, "another sub-type");
        assert_eq!(play(&mut e, &[n(2), n(0), n(1)]), 1.0);
        assert_eq!(play(&mut e, &[n(2), n(0), n(6)]), 0.0, "no flag shared");
        assert_eq!(
            play(&mut e, &[n(2), n(0), n(0x101)]),
            1.0,
            "a whole word of flags"
        );
        assert_eq!(play(&mut e, &[n(1), n(1)]), 1.0);
        assert_eq!(
            play(&mut e, &[n(1), n(0), n(1)]),
            0.0,
            "group 0 has no flags"
        );
        assert_eq!(play(&mut e, &[n(3)]), 0.0);
        // 0xB5 = 181: {4, 0, 0}; 0xF4 = 244: {2, 5, 0}; 0xFF: skipped.
        groups(
            &mut e,
            ANIM,
            [0xb5, 0xf4, 0xf5, 0xff, 0xff, 0xff, 0xff, 0xff],
        );
        assert_eq!(
            play(&mut e, &[n(4), n(9), n(9)]),
            1.0,
            "category 4: the rest ignored"
        );
        assert_eq!(play(&mut e, &[n(2), n(5)]), 1.0, "the last entry");
        assert_eq!(play(&mut e, &[n(5)]), 0.0);
        // A group in the eighth slot counts.
        groups(
            &mut e,
            ANIM,
            [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x11],
        );
        assert_eq!(
            play(&mut e, &[n(3), n(0), n(0)]),
            1.0,
            "group 17: {{3, 0, 0}}"
        );
        assert!(calls_to(GET_ANIM_DATA_FN).iter().all(|w| w == &vec![REFR]));
    }

    /// The player's point of view: only with a high process; 1 its first-person data, else its
    /// process's; another reference with a point of view uses `GetAnimData`.
    #[test]
    fn anim_playing_pov() {
        let mut e = setup();
        groups(&mut e, ANIM, [0; 8]);
        groups(&mut e, ANIM2, [0x11; 8]);
        let play = |e: &mut Engine, pov: i32| run(e, 0x283f, REFR, &[n(3), n(0), n(0), n(pov)]);
        assert_eq!(play(&mut e, 1), 0.0, "not the player: GetAnimData");
        assert_eq!(calls_to(GET_ANIM_DATA_FN).len(), 1);
        form(&mut e, REFR, 0x3b, 0x14);
        assert_eq!(play(&mut e, 1), 0.0, "no process");
        e.mem.set_u32(REFR + 0x68, PROC);
        e.mem.set_u32(PROC + 0x28, 2);
        assert_eq!(play(&mut e, 1), 0.0, "a low process");
        e.mem.set_u32(PROC + 0x28, 0);
        assert_eq!(play(&mut e, 1), 0.0, "no first-person data");
        e.mem.set_u32(REFR + 0x690, ANIM2);
        assert_eq!(play(&mut e, 1), 1.0);
        assert_eq!(play(&mut e, 2), 0.0, "the process's: none");
        e.mem.set_u32(PROC + 0x1c0, ANIM2);
        assert_eq!(play(&mut e, 2), 1.0);
        assert_eq!(calls_to(GET_ANIM_DATA_FN).len(), 1);
        assert_eq!(play(&mut e, 0), 0.0, "no point of view: GetAnimData");
        assert_eq!(calls_to(GET_ANIM_DATA_FN).len(), 2);
    }

    /// Into the player: the player's add; into another container: picked up, added, destroyed
    /// unless flag 1, a file, or the flags test says otherwise.
    #[test]
    fn to_container() {
        let mut e = setup();
        let mv = |e: &mut Engine, args: &[Vec<u8>]| {
            CALLS.with(|c| c.borrow_mut().clear());
            run(e, 0x284a, REFR, args);
        };
        mv(&mut e, &[r(1)]);
        assert_eq!(calls_to(CONT_ADD_FN), vec![vec![DEST, REFR, 1, 0, 0]]);
        assert_eq!(calls_to(PICKED_UP_FN), vec![vec![REFR]]);
        assert_eq!(calls_to(FLAGS_TEST_FN), vec![vec![REFR + 0x44, 2]]);
        assert_eq!(calls_to(DESTROY_FN), vec![vec![REFR, 1]]);
        assert!(calls_to(REMOVE_BY_TYPE_FN).is_empty());
        assert!(calls_to(HUD_INFO_FN).is_empty());
        let order = || calls().into_iter().map(|c| c.0).collect::<Vec<_>>();
        assert_eq!(
            order(),
            vec![PICKED_UP_FN, CONT_ADD_FN, FLAGS_TEST_FN, DESTROY_FN],
            "picked up, added, the flags test, destroyed: in that order"
        );
        e.mem.set_u32(FLAGS_ANSWER, 0x100);
        mv(&mut e, &[r(1)]);
        assert_eq!(calls_to(DESTROY_FN).len(), 1, "only the low byte answers");
        e.mem.set_u32(FLAGS_ANSWER, 1);
        mv(&mut e, &[r(1)]);
        assert!(calls_to(DESTROY_FN).is_empty(), "the flags test");
        e.mem.set_u32(FLAGS_ANSWER, 0);
        e.mem.set_u32(REFR + 0x10, MISC);
        mv(&mut e, &[r(1)]);
        assert!(calls_to(DESTROY_FN).is_empty(), "from a file");
        e.mem.set_u32(REFR + 0x10, 0);
        e.mem.set_u32(REFR + 8, 1);
        mv(&mut e, &[r(1)]);
        assert!(calls_to(DESTROY_FN).is_empty(), "flag 1");
        e.mem.set_u32(REFR + 8, 0);
        // Under the crosshair, the owner dropped.
        e.mem.set_u32(0x011d_96c0, HUD);
        e.mem.set_u32(HUD + 0x1b8, DEST);
        mv(&mut e, &[r(1), n(1)]);
        assert!(
            calls_to(HUD_INFO_FN).is_empty(),
            "another reference under the crosshair"
        );
        e.mem.set_u32(HUD + 0x1b8, REFR);
        mv(&mut e, &[r(1), n(1)]);
        assert_eq!(calls_to(HUD_INFO_FN), vec![vec![0, 0, 0]]);
        assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![REFR + 0x44, 0x21]]);
        assert_eq!(calls_to(PICKED_UP_FN).len(), 1);
        let at = |addr: u32| order().iter().position(|&a| a == addr).unwrap_or(usize::MAX);
        assert!(
            at(HUD_INFO_FN) < at(REMOVE_BY_TYPE_FN) && at(REMOVE_BY_TYPE_FN) < at(PICKED_UP_FN),
            "the crosshair cleared, then the owner removed, then picked up: {:x?}",
            order()
        );
        // Into the player.
        form(&mut e, DEST, 0x3b, 0x14);
        mv(&mut e, &[r(1)]);
        assert_eq!(calls_to(PLAYER_ADD_FN), vec![vec![DEST, REFR, 1, 0]]);
        assert!(calls_to(CONT_ADD_FN).is_empty());
        assert!(calls_to(PICKED_UP_FN).is_empty());
        assert!(calls_to(DESTROY_FN).is_empty());
        // Not an item: nothing.
        form(&mut e, DEST, 0x3a, 0x103);
        e.mem.set_u32(REFR + 0x20, ACTI);
        mv(&mut e, &[r(1)]);
        assert!(calls().is_empty(), "not an item");
        // A target base with no container; one answering through its vtable (+0xF8).
        e.mem.set_u32(REFR + 0x20, MISC);
        e.mem.set_u32(MISC, CONT_VTBL);
        e.mem.set_u32(DEST + 0x20, MISC);
        mv(&mut e, &[r(1)]);
        assert!(calls().is_empty(), "no container");
        e.mem.set_u32(CONT_VTBL + 0xf8, 0x008d_0360);
        mv(&mut e, &[r(1)]);
        assert_eq!(calls_to(CONT_ADD_FN).len(), 1);
        e.mem.set_u32(DEST + 0x20, 0);
        mv(&mut e, &[r(1)]);
        assert!(calls().is_empty(), "no target base");
    }

    #[test]
    fn pos_ex() {
        let mut e = setup();
        locals(&mut e, 3);
        set3(&mut e, REFR + 0x30, [1.5, -2.25, 1e10]);
        assert_eq!(run(&mut e, 0x28f8, REFR, &[v(1), v(2), v(3)]), 0.0);
        assert_eq!(vars(&e, 3), vec![1.5, -2.25, f64::from(1e10f32)]);
        set3(&mut e, REFR + 0x30, [0.1, 7.0, 0.0]);
        run(&mut e, 0x28f8, REFR, &[v(3), v(3), v(1)]);
        assert_eq!(var(&e, 3), 7.0, "y after x into the same variable");
        assert_eq!(var(&e, 1), 0.0);
        set3(&mut e, REFR + 0x30, [5.0, 5.0, 5.0]);
        run(&mut e, 0x28f8, REFR, &[v(1), v(2), v(9)]);
        assert_eq!(
            var(&e, 1),
            0.0,
            "a missing variable: the read fails, nothing written"
        );
    }

    /// JIP's FromEulerPRY from its asm's lanes: sines are `Cos_V3` of pi/2 less the angles.
    fn jip_matrix(p: f32, r: f32, y: f32) -> [f32; 9] {
        use crate::commands::ln_fn_math::cos_v3;
        let h = std::f32::consts::FRAC_PI_2;
        let [s0, s1, s2] = cos_v3([h - p, h - r, h - y]);
        let [c0, c1, c2] = cos_v3([p, r, y]);
        [
            c1 * c2,
            c1 * s2,
            -s1,
            s0 * s1 * c2 - c0 * s2,
            s0 * s1 * s2 + c0 * c2,
            c1 * s0,
            c0 * s1 * c2 + s0 * s2,
            c0 * s1 * s2 - s0 * c2,
            c1 * c0,
        ]
    }

    /// The matrix is JIP's and a rotation: close to the textbook one for pitch, roll, yaw.
    #[test]
    fn matrix_from_angles() {
        for (p, r, y) in [
            (0.3f32, -1.1f32, 2.0f32),
            (0.0, 0.0, 0.0),
            (-3.0, 0.5, -0.25),
        ] {
            let m = from_euler_pry([p, r, y]);
            assert_eq!(m.map(f32::to_bits), jip_matrix(p, r, y).map(f32::to_bits));
            let (p, r, y) = (f64::from(p), f64::from(r), f64::from(y));
            let want = [
                r.cos() * y.cos(),
                r.cos() * y.sin(),
                -r.sin(),
                p.sin() * r.sin() * y.cos() - p.cos() * y.sin(),
                p.sin() * r.sin() * y.sin() + p.cos() * y.cos(),
                r.cos() * p.sin(),
                p.cos() * r.sin() * y.cos() + p.sin() * y.sin(),
                p.cos() * r.sin() * y.sin() - p.sin() * y.cos(),
                r.cos() * p.cos(),
            ];
            for (a, b) in m.iter().zip(want) {
                assert!((f64::from(*a) - b).abs() < 1e-5, "{m:?}");
            }
        }
    }

    /// Rows times the vector, ((a0 + a1) + (a2 + a3)) with a3 the next float times 0: an
    /// infinite word after a row makes its answer NaN.
    #[test]
    fn matrix_times_vector() {
        let m = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        assert_eq!(
            multiply_vector(&m, 0.0, [1.0, 10.0, 100.0]),
            [321.0, 654.0, 987.0]
        );
        let r = multiply_vector(&m, f32::INFINITY, [1.0, 1.0, 1.0]);
        assert_eq!(r[..2], [6.0, 15.0]);
        assert!(r[2].is_nan(), "the word past the last row");
        let mut m2 = m;
        m2[3] = f32::NAN;
        let r = multiply_vector(&m2, 0.0, [1.0, 1.0, 1.0]);
        assert!(r[0].is_nan(), "the next row's first");
        let mut m3 = m;
        m3[6] = f32::INFINITY;
        assert!(multiply_vector(&m3, 0.0, [1.0, 1.0, 1.0])[1].is_nan());
        // The pairs: (1 + 1e8) + (-1e8 + 0) = 0 in floats, where 1 + (1e8 + -1e8) is 1.
        let m4 = [1.0, 1e8, -1e8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(multiply_vector(&m4, 0.0, [1.0, 1.0, 1.0])[0], 0.0);
        let m5 = [1e8, -1e8, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(multiply_vector(&m5, 0.0, [1.0, 1.0, 1.0])[0], 1.0);
        // -0 in the last pair: -0 + (+0 * 0 = +0) = +0.
        let m6 = [0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(multiply_vector(&m6, 0.0, [-0.0, 0.0, 0.0])[0].to_bits(), 0);
    }

    #[test]
    fn angles_from_matrix() {
        use crate::commands::ln_fn_math::{asin, atan2};
        let m = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        assert_eq!(
            to_euler_pry_inv(&m),
            [atan2(0.8, 0.9), asin(-0.7), atan2(0.4, 0.1)]
        );
    }

    #[test]
    fn angle_ex() {
        let mut e = setup();
        locals(&mut e, 3);
        let deg = "57.29578018".parse::<f32>().unwrap();
        assert_eq!(FLT_180_D_PI.to_bits(), deg.to_bits());
        set3(&mut e, REFR + 0x24, [0.5, -1.0, 3.0]);
        run(&mut e, 0x28f9, REFR, &[v(1), v(2), v(3)]);
        let want = [0.5f32 * deg, -deg, 3.0 * deg].map(f64::from).to_vec();
        assert_eq!(vars(&e, 3), want);
        run(&mut e, 0x28f9, REFR, &[v(1), v(2), v(3), n(0)]);
        assert_eq!(vars(&e, 3), want, "0: not local");
        // Local, no 3D: the matrix of the angles.
        run(&mut e, 0x28f9, REFR, &[v(1), v(2), v(3), n(1)]);
        let a = to_euler_pry_inv(&jip_matrix(0.5, -1.0, 3.0)).map(|x| f64::from(x * deg));
        assert_eq!(vars(&e, 3), a.to_vec());
        // Loaded data with no root node: the same.
        e.mem.set_u32(REFR + 0x64, LOADED);
        run(&mut e, 0x28f9, REFR, &[v(1), v(2), v(3), n(1)]);
        assert_eq!(vars(&e, 3), a.to_vec());
        // Local with 3D: the root node's world rotation.
        e.mem.set_u32(LOADED + 0x14, NODE);
        let m = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        set_matrix(&mut e, NODE + 0x68, m);
        set_matrix(&mut e, NODE + 0x34, [0.9; 9]);
        run(&mut e, 0x28f9, REFR, &[v(1), v(2), v(3), n(1)]);
        let b = to_euler_pry_inv(&m).map(|x| f64::from(x * deg));
        assert_eq!(vars(&e, 3), b.to_vec());
        // The player: its angles' matrix even with 3D.
        form(&mut e, REFR, 0x3b, 0x14);
        run(&mut e, 0x28f9, REFR, &[v(1), v(2), v(3), n(2)]);
        assert_eq!(vars(&e, 3), a.to_vec());
    }

    #[test]
    fn translated_pos() {
        let mut e = setup();
        locals(&mut e, 3);
        set3(&mut e, REFR + 0x24, [0.0, 0.0, 1.0]);
        set3(&mut e, REFR + 0x30, [100.0, 200.0, 300.0]);
        let args = [z(10.0), z(20.0), z(30.0), v(1), v(2), v(3)];
        assert_eq!(run(&mut e, 0x2909, REFR, &args), 1.0);
        let m = jip_matrix(0.0, 0.0, 1.0);
        let row = |i: usize| (m[3 * i] * 10.0 + m[3 * i + 1] * 20.0) + m[3 * i + 2] * 30.0;
        let want = [row(0) + 100.0, row(1) + 200.0, row(2) + 300.0].map(f64::from);
        assert_eq!(vars(&e, 3), want.to_vec());
        // With 3D: the node's world rotation and translation (the local transform not used).
        e.mem.set_u32(REFR + 0x64, LOADED);
        e.mem.set_u32(LOADED + 0x14, NODE);
        set_matrix(
            &mut e,
            NODE + 0x68,
            [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 2.0],
        );
        set_matrix(&mut e, NODE + 0x34, [5.0; 9]);
        set3(&mut e, NODE + 0x58, [9.0, 9.0, 9.0]);
        set3(&mut e, NODE + 0x8c, [1.0, 2.0, 3.0]);
        run(&mut e, 0x2909, REFR, &args);
        assert_eq!(vars(&e, 3), vec![-19.0, 12.0, 63.0]);
        // An infinite translation x: the last row's fourth lane reads it (NaN).
        e.mem.set_f32(NODE + 0x8c, f32::INFINITY);
        run(&mut e, 0x2909, REFR, &args);
        assert_eq!(var(&e, 1), f64::INFINITY);
        assert_eq!(var(&e, 2), 12.0);
        assert!(var(&e, 3).is_nan());
        // A missing variable: nothing, result 0.
        e.mem.set_f32(NODE + 0x8c, 1.0);
        let missing = [z(1.0), z(1.0), z(1.0), v(1), v(2), v(7)];
        assert_eq!(run(&mut e, 0x2909, REFR, &missing), 0.0);
        assert!(var(&e, 3).is_nan(), "unchanged");
    }

    #[test]
    fn editor_position() {
        let mut e = setup();
        locals(&mut e, 6);
        let deg = FLT_180_D_PI;
        let six = [v(1), v(2), v(3), v(4), v(5), v(6)];
        let get = |e: &mut Engine, args: &[Vec<u8>]| {
            e.mem.set_u32(RESULT, 0);
            call_on(e, 0x292a, REFR, args);
            form_result(e)
        };
        assert_eq!(get(&mut e, &six), 0, "no extras");
        put_extra(&mut e, REFR + 0x44, X2, 0x0f, 0);
        set3(&mut e, X2 + 0xc, [1.0, 2.0, 3.0]);
        set3(&mut e, X2 + 0x18, [0.5, 0.25, -1.0]);
        assert_eq!(get(&mut e, &six), 0, "no starting cell");
        assert_eq!(var(&e, 1), -1.0);
        put_extra(&mut e, REFR + 0x44, X1, 0x49, CELL_AT);
        assert_eq!(get(&mut e, &six), 0x105);
        let rot = [0.5 * deg, 0.25 * deg, -deg].map(f64::from);
        assert_eq!(vars(&e, 6), vec![1.0, 2.0, 3.0, rot[0], rot[1], rot[2]]);
        locals(&mut e, 6);
        assert_eq!(get(&mut e, &six[..3]), 0x105);
        assert_eq!(var(&e, 3), 3.0);
        assert_eq!(var(&e, 4), -1.0, "three arguments: no angles");
        locals(&mut e, 6);
        assert_eq!(get(&mut e, &six[..5]), 0x105);
        assert_eq!(var(&e, 4), -1.0, "five arguments: no angles");
        // A worldspace: its persistent cell, or nothing without one.
        e.mem.set_u32(X1 + 0xc, WORLD_AT);
        locals(&mut e, 6);
        assert_eq!(get(&mut e, &six), 0);
        assert_eq!(var(&e, 1), -1.0);
        e.mem.set_u32(WORLD_AT + 0x34, CELL_AT);
        assert_eq!(get(&mut e, &six), 0x105);
        e.mem.set_u32(X1 + 0xc, MISC);
        locals(&mut e, 6);
        assert_eq!(get(&mut e, &six), 0, "not a cell");
        assert_eq!(var(&e, 1), -1.0);
        e.mem.set_u32(X1 + 0xc, 0);
        assert_eq!(get(&mut e, &six), 0, "no cell");
        // The starting position missing.
        e.mem.set_u32(X1 + 0xc, CELL_AT);
        e.mem.set_u32(X1 + 8, 0);
        assert_eq!(get(&mut e, &six), 0, "no starting position");
        // An actor: its own place, angles 0, 0, z.
        e.mem.set_u32(REFR, ACTOR_VTBL);
        set3(&mut e, REFR + 0x160, [7.0, 8.0, 9.0]);
        e.mem.set_f32(REFR + 0x16c, 2.0);
        assert_eq!(get(&mut e, &six), 0, "no starting world or cell");
        e.mem.set_u32(REFR + 0x170, CELL_AT);
        assert_eq!(get(&mut e, &six), 0x105);
        assert_eq!(
            vars(&e, 6),
            vec![7.0, 8.0, 9.0, 0.0, 0.0, f64::from(2.0 * deg)]
        );
    }
}
