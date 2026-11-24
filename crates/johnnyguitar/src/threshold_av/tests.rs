use super::*;
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const ACTOR: u32 = DATA + 0x2000;
const VT: u32 = DATA + 0x3000;
const PREDICATE: u32 = DATA + 0x4000;
const NAME: u32 = DATA + 0x5000;
#[derive(Default)]
struct State {
    actor: bool,
    value: f64,
    predicates: usize,
    queries: Vec<u32>,
    prints: Vec<Vec<u32>>,
    names: Vec<u32>,
}
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(ACTOR, VT);
    e.mem.set_u32(VT + IS_ACTOR, PREDICATE);
    e.mem.write(NAME, b"Sneak\0");
    e.ext::<State>().actor = true;
    e.ext::<State>().value = 42.25;
    e.register(PREDICATE, |e, a| {
        assert_eq!(a, [ACTOR]);
        let s = e.ext::<State>();
        s.predicates += 1;
        u32::from(s.actor).into_ret()
    });
    e.register(GET_CLAMPED, |e, a| {
        assert_eq!(a[0], ACTOR + 0xa4);
        e.ext::<State>().queries.push(a[1]);
        e.ext::<State>().value.into_ret()
    });
    e.register(ECHO, |e, _| {
        let tls = e.tls();
        u32::from(e.mem.u8(tls + 0x268)).into_ret()
    });
    e.register(GET_AV_NAME, |e, a| {
        e.ext::<State>().names.push(a[0]);
        NAME.into_ret()
    });
    e.register(PRINT, |e, a| {
        assert_eq!(table::guest_text(e, a[0]), b"GetThresholdAV %s >> %.2f");
        e.ext::<State>().prints.push(a[1..].to_vec());
        Ret::default()
    });
    e
}
fn execute_call(e: &mut Engine, av: u16) -> f64 {
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, 4);
    e.mem.set_u16(DATA + 4, 1);
    e.mem.set_u16(DATA + 6, av);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -1.0);
    let rec = table::record_of(e, OPCODE);
    let args = [e.mem.u32(rec + 0x14), DATA, ACTOR, 0, 0, 0, RESULT, OFFSET];
    assert!(e.call(e.mem.u32(rec + 0x18), &args).bool());
    e.mem.f64(RESULT)
}
fn eval_call(e: &mut Engine, this: u32, av: u32) -> f64 {
    e.mem.set_f64(RESULT, -1.0);
    let rec = table::record_of(e, OPCODE);
    assert!(e.call(e.mem.u32(rec + 0x20), &[this, av, 0, RESULT]).bool());
    e.mem.f64(RESULT)
}
#[test]
fn real_actor_value_parameter_and_condition_delegate_to_actual_float_clamped_getter() {
    let mut e = engine();
    let rec = table::record_of(&e, OPCODE);
    assert_eq!(table::opcode_by_name(&e, b"GetThresholdAV"), Some(OPCODE));
    assert_eq!(e.mem.u16(rec + 0x10), 1);
    assert_eq!(e.mem.u16(rec + 0x12), 1);
    let p = e.mem.u32(rec + 0x14);
    assert_eq!((e.mem.u32(p + 4), e.mem.u32(p + 8)), (5, 0));
    assert_eq!(table::return_type(&e, OPCODE), Returns::Default);
    assert_eq!(execute_call(&mut e, 41), 42.25);
    assert_eq!(e.ext::<State>().predicates, 2);
    assert_eq!(e.ext::<State>().queries, [41]);
    assert_eq!(eval_call(&mut e, ACTOR, 0xffff_ffff), 42.25);
    assert_eq!(e.ext::<State>().queries, [41, u32::MAX]);
    assert_eq!(e.ext::<State>().predicates, 3);
}
#[test]
fn float_precision_null_conditions_nonactors_and_tls_console_echo_follow_source() {
    let mut e = engine();
    e.ext::<State>().value = 1.23456789012345;
    let expected = f64::from(1.23456789012345f64 as f32);
    assert_eq!(execute_call(&mut e, 41), expected);
    assert!(e.ext::<State>().prints.is_empty());
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
    assert_eq!(execute_call(&mut e, 41), expected);
    let bits = expected.to_bits();
    assert_eq!(
        e.ext::<State>().prints,
        [vec![NAME, bits as u32, (bits >> 32) as u32]]
    );
    assert_eq!(e.ext::<State>().names, [41]);
    let calls = e.ext::<State>().predicates;
    assert_eq!(eval_call(&mut e, 0, 41), 0.0);
    assert_eq!(e.ext::<State>().predicates, calls);
    e.ext::<State>().actor = false;
    let queries = e.ext::<State>().queries.len();
    assert_eq!(execute_call(&mut e, 41), 0.0);
    assert_eq!(eval_call(&mut e, ACTOR, 41), 0.0);
    assert_eq!(e.ext::<State>().queries.len(), queries);
    assert_eq!(e.ext::<State>().prints.len(), 1);
}
