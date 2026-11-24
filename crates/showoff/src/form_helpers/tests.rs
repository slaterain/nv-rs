use super::*;
use nvse::vars::{Key, Value};
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const EVENT: u32 = DATA + 0x3000;
const LOCAL: u32 = DATA + 0x3100;
const LOCAL_NODE: u32 = DATA + 0x3200;
const REF: u32 = DATA + 0x4000;
const LIST: u32 = DATA + 0x5000;
const A: u32 = DATA + 0x6000;
const B: u32 = A + 0x100;
const C: u32 = B + 0x100;
const VT: u32 = DATA + 0x7000;
const HEAP: u32 = DATA + 0x8000;
#[derive(Default)]
struct Heap(u32);
fn ret(n: u32) -> Ret {
    Ret {
        eax: n,
        ..Ret::default()
    }
}
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(DATA, 0x20000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(SCRIPT + 0xc, 0x0100_1234);
    e.mem.set_u32(SCRIPT + 0x44, REF);
    e.mem.set_u32(REF + 8, LIST);
    e.mem.set_u32(EVENT + 0xc, LOCAL_NODE);
    e.mem.set_u32(LOCAL_NODE, LOCAL);
    e.mem.set_u32(LOCAL, 1);
    e.mem.set_u32(LIST + 0xc, 0x0100_1111);
    e.mem.set_u32(LIST, VT);
    e.mem.set_u8(LIST + 4, 0x55);
    for (form, id) in [(A, 0x0100_2222), (B, 0x0100_3333), (C, 0x0100_4444)] {
        e.mem.set_u32(form + 0xc, id);
        e.mem.set_u32(form, VT);
    }
    e.register(LOOKUP_FORM, |_, a| {
        ret(match a[0] {
            0x0100_1111 => LIST,
            0x0100_2222 => A,
            0x0100_3333 => B,
            0x0100_4444 => C,
            _ => 0,
        })
    });
    e.register(0x00ec_43fb, |_, a| {
        assert_eq!(a[1], 0);
        assert_eq!(a[2], rtti::TES_FORM);
        assert_eq!(a[3], rtti::BGS_LIST_FORM);
        assert_eq!(a[4], 0);
        ret(if a[0] == LIST { LIST } else { 0 })
    });
    e.register(0x00ec_a7f4, |_, a| {
        ret(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.ext::<Heap>().0 = HEAP;
    e.register(0x0040_1000, |e, a| {
        let at = e.ext::<Heap>().0;
        e.ext::<Heap>().0 += a[0].div_ceil(4) * 4;
        ret(at)
    });
    e
}
fn array(e: &mut Engine, key_type: u8, entries: &[(Key, Value)]) -> Vec<u8> {
    let id = vars::create_array(e, key_type, key_type == vars::KEY_NUMBER, 1);
    for (key, value) in entries {
        assert!(vars::set_element(e, id, key, value.clone()));
    }
    e.mem.set_f64(LOCAL + 8, f64::from(id));
    vec![b'V', 3, 0, 0, 1, 0]
}
fn number(n: f64) -> Vec<u8> {
    [vec![b'Z'], n.to_le_bytes().to_vec()].concat()
}
fn run(e: &mut Engine, args: &[Vec<u8>]) -> f64 {
    let mut body = vec![args.len() as u8];
    for arg in args {
        body.extend_from_slice(&((arg.len() + 2) as u16).to_le_bytes());
        body.extend_from_slice(arg);
    }
    e.mem.set_u16(DATA, 0x3d1c);
    e.mem.set_u16(DATA + 2, body.len() as u16);
    e.mem.write(DATA + 4, &body);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -123.0);
    let rec = table::record_of(e, 0x3d1c);
    let words = [
        e.mem.u32(rec + 0x14),
        DATA,
        0,
        0,
        SCRIPT,
        EVENT,
        RESULT,
        OFFSET,
    ];
    assert!(e.call(e.mem.u32(rec + 0x18), &words).bool());
    e.mem.f64(RESULT)
}
fn base(e: &mut Engine, form: u32) -> f64 {
    e.mem.set_u32(REF + 8, form);
    e.mem.set_u16(DATA, 0x3cd4);
    e.mem.set_u16(DATA + 2, 5);
    e.mem.set_u16(DATA + 4, 1);
    e.mem.write(DATA + 6, &[b'r', 1, 0]);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -123.0);
    let rec = table::record_of(e, 0x3cd4);
    let words = [
        e.mem.u32(rec + 0x14),
        DATA,
        0,
        0,
        SCRIPT,
        EVENT,
        RESULT,
        OFFSET,
    ];
    assert!(e.call(e.mem.u32(rec + 0x18), &words).bool());
    e.mem.f64(RESULT)
}
fn contents(e: &Engine) -> Vec<u32> {
    let mut out = Vec::new();
    let mut at = LIST + 0x18;
    while at != 0 {
        let item = e.mem.u32(at);
        if item != 0 {
            out.push(item);
        }
        at = e.mem.u32(at + 4);
    }
    out
}
fn pair(form: u32) -> (Key, Value) {
    (Key::Number(0.0), Value::Form(form))
}

#[test]
fn exact_records_aliases_parameters_and_return_types() {
    let e = engine();
    for (op, name, short, spec) in [
        (0x3cd4, "IsBaseForm", "IsBasedForm", &[(0x3d, false)][..]),
        (
            0x3d1c,
            "ListAddArray",
            "AddArrayToFormList",
            &[(0x8008, false), (0x8040, false), (0x8001, true)][..],
        ),
    ] {
        assert_eq!(table::opcode_by_name(&e, name.as_bytes()), Some(op));
        assert_eq!(table::opcode_by_name(&e, short.as_bytes()), Some(op));
        assert_eq!(table::return_type(&e, op), Returns::Default);
        let rec = table::record_of(&e, op);
        assert_eq!(e.mem.u16(rec + 0x10), 0);
        assert_eq!(e.mem.u16(rec + 0x12), spec.len() as u16);
        assert_eq!(e.mem.u32(rec + 0x20), 0);
        let params = e.mem.u32(rec + 0x14);
        for (i, (kind, optional)) in spec.iter().enumerate() {
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 4), *kind);
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 8), u32::from(*optional));
        }
    }
}
#[test]
fn base_form_is_exact_virtual_function_address_not_type_or_called_result() {
    let mut e = engine();
    e.mem.set_u32(VT + 0xf0, RETURN_TRUE);
    e.mem.set_u8(A + 4, 0x28);
    assert_eq!(
        base(&mut e, A),
        0.0,
        "even a weapon type with the reference marker"
    );
    e.mem.set_u32(VT + 0xf0, DATA + 0x18000);
    e.register(DATA + 0x18000, |_, _| {
        panic!("IS_REFERENCE must not invoke virtual code")
    });
    e.mem.set_u8(A + 4, 0x3a);
    assert_eq!(
        base(&mut e, A),
        1.0,
        "even reference type with different vfunc"
    );
    assert_eq!(base(&mut e, 0), 0.0);
}
#[test]
fn appends_form_values_in_array_order_skips_other_and_missing_values_and_keeps_duplicates() {
    let mut e = engine();
    e.mem.set_u32(LIST + 0x18, C);
    e.mem.set_u32(LIST + 0x20, 1);
    e.mem.set_u32(LIST + 8, 0x1234);
    let entries = [
        pair(0x0100_2222),
        (Key::Number(1.0), Value::Number(7.0)),
        (Key::Number(2.0), Value::Form(0x0100_9999)),
        (Key::Number(3.0), Value::Form(0x0100_3333)),
        (Key::Number(4.0), Value::Form(0x0100_2222)),
    ];
    let token = array(&mut e, vars::KEY_NUMBER, &entries);
    assert_eq!(run(&mut e, &[vec![b'R', 1, 0], token]), 1.0);
    assert_eq!(contents(&e), [C, A, B, A]);
    assert_eq!(
        e.mem.u32(LIST + 0x20),
        1,
        "requested unsigned -2 is not added-object index"
    );
    assert_eq!(
        e.mem.u32(LIST + 8),
        0x1234,
        "source does not mark the enclosing form modified"
    );
}
#[test]
fn explicit_index_reverses_input_and_counts_using_requested_index() {
    let mut e = engine();
    e.mem.set_u32(LIST + 0x18, C);
    e.mem.set_u32(LIST + 0x20, 1);
    let token = array(
        &mut e,
        vars::KEY_NUMBER,
        &[
            pair(0x0100_2222),
            (Key::Number(1.0), Value::Form(0x0100_3333)),
        ],
    );
    assert_eq!(run(&mut e, &[vec![b'R', 1, 0], token, number(0.0)]), 1.0);
    assert_eq!(contents(&e), [A, B, C]);
    assert_eq!(e.mem.u32(LIST + 0x20), 3);
    let before = contents(&e);
    let token = array(&mut e, vars::KEY_NUMBER, &[pair(0x0100_2222)]);
    assert_eq!(run(&mut e, &[vec![b'R', 1, 0], token, number(-1.0)]), 0.0);
    assert_eq!(contents(&e), before);
    assert_eq!(e.mem.u32(LIST + 0x20), 3);
    // The requested index equal to numAddedObjects is outside the counted range.
    e.mem.set_u32(LIST + 0x20, 2);
    let token = array(&mut e, vars::KEY_NUMBER, &[pair(0x0100_2222)]);
    assert_eq!(run(&mut e, &[vec![b'R', 1, 0], token, number(2.0)]), 1.0);
    assert_eq!(e.mem.u32(LIST + 0x20), 2);
}
#[test]
fn wrong_target_missing_array_and_empty_array_leave_success_default_and_list_unchanged() {
    let mut e = engine();
    let token = array(&mut e, vars::KEY_NUMBER, &[pair(0x0100_2222)]);
    e.mem.set_u32(REF + 8, A);
    assert_eq!(run(&mut e, &[vec![b'R', 1, 0], token]), 1.0);
    assert!(contents(&e).is_empty());
    e.mem.set_u32(REF + 8, LIST);
    e.mem.set_f64(LOCAL + 8, 0.0);
    assert_eq!(
        run(&mut e, &[vec![b'R', 1, 0], vec![b'V', 3, 0, 0, 1, 0]]),
        1.0
    );
    let token = array(&mut e, vars::KEY_STRING, &[]);
    assert_eq!(run(&mut e, &[vec![b'R', 1, 0], token]), 1.0);
    assert!(contents(&e).is_empty());
}
