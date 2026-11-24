use super::*;
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
#[derive(Default)]
struct Output(Vec<Vec<u32>>);
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.register(CONSOLE_ECHO, |e, _| {
        let tls = e.tls();
        u32::from(e.mem.u8(tls + 0x268)).into_ret()
    });
    e.register(PRINT_LINE, |e, args| {
        assert_eq!(table::guest_text(e, args[0]), b"GetJohnnyPatch %d >> %d");
        e.ext::<Output>().0.push(args[1..].to_vec());
        Ret::default()
    });
    e
}
fn execute_id(e: &mut Engine, id: u32) -> f64 {
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, 7);
    e.mem.set_u16(DATA + 4, 1);
    e.mem.set_u8(DATA + 6, b'n');
    e.mem.set_u32(DATA + 7, id);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -1.0);
    let rec = table::record_of(e, OPCODE);
    let args = [e.mem.u32(rec + 0x14), DATA, 0, 0, 0, 0, RESULT, OFFSET];
    assert!(e.call(e.mem.u32(rec + 0x18), &args).bool());
    e.mem.f64(RESULT)
}
fn eval_id(e: &mut Engine, id: u32) -> f64 {
    e.mem.set_f64(RESULT, -1.0);
    let rec = table::record_of(e, OPCODE);
    assert!(e.call(e.mem.u32(rec + 0x20), &[0, id, 0, RESULT]).bool());
    e.mem.f64(RESULT)
}
#[test]
fn actual_record_and_condition_query_actual_enabled_state_without_claiming_unbuilt_flags() {
    let mut e = engine();
    let rec = table::record_of(&e, OPCODE);
    assert_eq!(table::opcode_by_name(&e, b"GetJohnnyPatch"), Some(OPCODE));
    assert_eq!(e.mem.u16(rec + 0x10), 0);
    assert_eq!(e.mem.u16(rec + 0x12), 1);
    let params = e.mem.u32(rec + 0x14);
    assert_eq!((e.mem.u32(params + 4), e.mem.u32(params + 8)), (1, 0));
    assert_eq!(execute_id(&mut e, 1), 0.0);
    engine_core::editor_ids::store(&mut e).set_on(true);
    assert_eq!(execute_id(&mut e, 1), 1.0);
    assert_eq!(eval_id(&mut e, 1), 1.0);
    for id in [0, 2, 3, 4, 5, 6, 7, 8, 9, u32::MAX] {
        assert_eq!(execute_id(&mut e, id), 0.0);
        assert_eq!(eval_id(&mut e, id), 0.0);
    }
    engine_core::editor_ids::store(&mut e).set_on(false);
    assert_eq!(eval_id(&mut e, 1), 0.0);
}
#[test]
fn source_console_echo_preserves_raw_double_varargs_and_invalid_reads_do_not_print() {
    let mut e = engine();
    engine_core::editor_ids::store(&mut e).set_on(true);
    assert_eq!(execute_id(&mut e, 1), 1.0);
    assert!(e.ext::<Output>().0.is_empty());
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
    assert_eq!(execute_id(&mut e, 1), 1.0);
    assert_eq!(e.ext::<Output>().0, [vec![1, 0, 0x3ff0_0000]]);
    e.mem.set_u16(DATA + 4, 1);
    e.mem.set_u8(DATA + 6, 0);
    e.mem.set_u32(OFFSET, 4);
    let rec = table::record_of(&e, OPCODE);
    let args = [e.mem.u32(rec + 0x14), DATA, 0, 0, 0, 0, RESULT, OFFSET];
    assert!(e.call(e.mem.u32(rec + 0x18), &args).bool());
    assert_eq!(e.mem.f64(RESULT), 0.0);
    assert_eq!(e.ext::<Output>().0.len(), 1);
}
#[test]
fn real_thread_views_read_the_same_actual_editor_store() {
    let mut owner = engine();
    let mut peer = engine();
    owner.share_slots_for_test(&mut peer);
    nvse::table::install(&mut peer);
    install(&mut peer);
    engine_core::editor_ids::store(&mut owner).set_on(true);
    assert_eq!(execute_id(&mut peer, 1), 1.0);
    engine_core::editor_ids::store(&mut peer).set_on(false);
    assert_eq!(execute_id(&mut owner, 1), 0.0);
}
