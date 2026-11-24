use super::*;
const AREA: u32 = 0x0300_0000;
const ACTOR: u32 = AREA + 0x1000;
const OWNER: u32 = ACTOR + 0xa4;
const VTABLE: u32 = AREA + 0x2000;
const SCRIPT: u32 = AREA + 0x3000;
const PARENT: u32 = AREA + 0x3400;
const DATA: u32 = AREA + 0x4000;
const RESULT: u32 = DATA + 0x800;
const OFFSET: u32 = RESULT + 8;
#[derive(Default)]
struct Seen {
    clamped: f32,
    form: u32,
    reads: Vec<u32>,
    calls: Vec<(u32, f64, f64, f64)>,
    stop: bool,
}
fn script(e: &mut Engine, p: u32, id: u32) {
    let data = e.mem.alloc(0x80);
    let mut code = vec![0x1d, 0, 0, 0, 0x10, 0];
    let block = [0x0d, 0, 0, 0, 0, 0, 1, 3, 1, 0, 1, 2, 0, 0, 3, 0, 0];
    code.extend_from_slice(&(block.len() as u16).to_le_bytes());
    code.extend(block);
    e.mem.write(data, &code);
    e.mem.set_u32(p, SCRIPT_VTABLE);
    e.mem.set_u8(p + 4, 0x11);
    e.mem.set_u32(p + 0xc, id);
    e.mem.set_u32(p + 0x20, code.len() as u32);
    e.mem.set_u32(p + 0x30, data);
}
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0103_7000, 0x1000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    nvse::table::install(&mut e);
    nvse::functions::install(&mut e);
    install(&mut e);
    e.mem
        .set_u32(nvse::functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(OWNER, VTABLE);
    e.mem.set_u32(VTABLE + 0xc, AREA + 0x5000);
    e.mem.set_u32(VTABLE + 0x24, AREA + 0x5010);
    e.ext::<Seen>().form = ACTOR;
    e.ext::<Seen>().clamped = 2.75;
    e.register(AREA + 0x5000, |e, w| {
        assert_eq!(w, [OWNER, 10]);
        e.ext::<Seen>().reads.push(0xc);
        e.ext::<Seen>().clamped.into_ret()
    });
    e.register(AREA + 0x5010, |e, w| {
        assert_eq!(w, [OWNER]);
        e.ext::<Seen>().reads.push(0x24);
        e.ext::<Seen>().form.into_ret()
    });
    script(&mut e, SCRIPT, 0x0500_3000);
    script(&mut e, PARENT, 0x0500_3400);
    e.register(0x0040_1000, |e, w| e.mem.alloc(w[0]).into_ret());
    e.register(0x0048_39c0, |_, _| 0u32.into_ret());
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, w| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, w[0]);
        let mut next = 0;
        for i in [3, 2, 1] {
            let local = e.mem.alloc(0x10);
            e.mem.set_u32(local, i);
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, local);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        e.mem.set_u32(list + 0xc, next);
        list.into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, w| {
        assert_eq!(w[1], 0);
        assert_eq!(&w[3..], [0, 0]);
        let values = std::array::from_fn::<_, 3, _>(|i| {
            let local = nvse::args::event_list_variable(e, w[2], i as u32 + 1);
            e.mem.f64(local + 8)
        });
        e.ext::<Seen>()
            .calls
            .push((w[0], values[0], values[1], values[2]));
        if e.ext::<Seen>().stop {
            panic!("stop inside AV callback");
        }
        Ret::default()
    });
    e.register(0x0066_ee50, |e, w| {
        assert!(SET_MODE.with(Cell::get));
        let mut payload = w.to_vec();
        payload.push(123);
        notify(e, &payload);
        Ret::default()
    });
    e
}
fn command_call(e: &mut Engine, target: u32, flags: Option<i32>, index: Option<i32>) {
    let first = e.mem.alloc(16);
    e.mem.set_u32(first + 8, target);
    e.mem.set_u32(PARENT + 0x44, first);
    let mut data = vec![b'z'];
    data.extend_from_slice(&1f64.to_le_bytes());
    data.extend_from_slice(&[b'r', 1, 0]);
    let mut count = 2;
    for value in [flags, index].into_iter().flatten() {
        data.push(b'z');
        data.extend_from_slice(&f64::from(value).to_le_bytes());
        count += 1;
    }
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, (data.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, count);
    e.mem.write(DATA + 6, &data);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.5);
    let record = table::record_of(e, OPCODE);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        0,
        0,
        PARENT,
        0,
        RESULT,
        OFFSET,
    ];
    assert!(e.call(e.mem.u32(record + 0x18), &words).bool());
    assert_eq!(e.mem.f64(RESULT), 1234.5);
}
fn sizes(e: &mut Engine) -> (usize, usize) {
    let lists = e.ext_free::<Events>().0.lock().unwrap();
    (lists.active.len(), lists.queued.len())
}
fn notify_values(e: &mut Engine, previous: f32, change: f32, info: u32) {
    assert_eq!(
        e.site(
            NOTIFY.addr(),
            &[OWNER, 10, previous.to_bits(), change.to_bits(), 0, info]
        ),
        None
    );
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 0.000001, "{a} != {b}");
}
#[test]
fn registration_and_decoder_keep_exact_player_command_alias_defaults_and_index_contract() {
    let mut e = setup();
    assert_eq!(
        table::opcode_by_name(&e, b"SetJohnnyOnActorValueChangeEventHandler"),
        Some(OPCODE)
    );
    assert_eq!(
        table::opcode_by_name(&e, b"SetOnNPCActorValueChangeEventHandler"),
        None
    );
    let record = table::record_of(&e, OPCODE);
    assert_eq!(e.mem.u16(record + 0x10), 0);
    assert_eq!(e.mem.u16(record + 0x12), 4);
    let params = e.mem.u32(record + 0x14);
    for (i, (kind, opt)) in [(1, 0), (0x3d, 0), (1, 1), (1, 1)].into_iter().enumerate() {
        assert_eq!(
            (
                e.mem.u32(params + i as u32 * 12 + 4),
                e.mem.u32(params + i as u32 * 12 + 8)
            ),
            (kind, opt)
        );
    }
    command_call(&mut e, ACTOR, None, None);
    assert_eq!(sizes(&mut e), (0, 0));
    command_call(&mut e, SCRIPT, None, None);
    command_call(&mut e, SCRIPT, Some(0), Some(77));
    command_call(&mut e, SCRIPT, Some(0), Some(-2));
    assert_eq!(sizes(&mut e), (0, 2));
    tick(&mut e);
    notify_values(&mut e, 1.0, 2.0, 123);
    assert!(
        e.ext::<Seen>().calls.is_empty(),
        "negative filters are not wildcards"
    );
    command_call(&mut e, SCRIPT, Some(1), Some(10));
    command_call(&mut e, SCRIPT, Some(3), Some(10));
    assert_eq!(sizes(&mut e), (2, 1));
    tick(&mut e);
    notify_values(&mut e, 1.2, 2.3, 123);
    assert_eq!(e.ext::<Seen>().calls.len(), 1);
    let (_, index, old, new) = e.ext::<Seen>().calls[0];
    assert_eq!(index, 0.0, "raw AV index goes through Alt float-bit ABI");
    close(old, 1.2);
    close(new, 3.5);
    assert_eq!(e.ext::<Seen>().reads, [0xc, 0x24, 0xc, 0x24]);
}
#[test]
fn flags_preserve_floor_fractional_clamp_and_outer_integer_change_gate() {
    for (flags, old, new) in [(0, 1.0, 3.0), (1, 1.2, 3.5), (2, 1.0, 2.0), (3, 1.2, 2.75)] {
        let mut e = setup();
        change(&mut e, SCRIPT, ACTOR, 10, flags, true);
        tick(&mut e);
        notify_values(&mut e, 1.2, 2.3, 123);
        let (_, _, actual_old, actual_new) = e.ext::<Seen>().calls[0];
        close(actual_old, old);
        close(actual_new, new);
        notify_values(&mut e, 1.2, 0.4, 123);
        assert_eq!(
            e.ext::<Seen>().calls.len(),
            1,
            "no-floor flag does not bypass outer floor-change predicate"
        );
        e.ext::<Seen>().clamped = 1.9;
        notify_values(&mut e, 1.2, 2.3, 123);
        assert_eq!(
            e.ext::<Seen>().calls.len(),
            if flags & 2 != 0 { 1 } else { 2 }
        );
    }
}
#[test]
fn info_and_player_selection_preserve_native_reads_and_do_not_claim_npc_callbacks() {
    let mut e = setup();
    change(&mut e, SCRIPT, ACTOR, 10, 0, true);
    tick(&mut e);
    notify_values(&mut e, 1.0, 2.0, 0);
    assert!(e.ext::<Seen>().reads.is_empty());
    e.ext::<Seen>().form = 0;
    notify_values(&mut e, 1.0, 2.0, 123);
    assert_eq!(e.ext::<Seen>().reads, [0xc, 0x24]);
    assert!(e.ext::<Seen>().calls.is_empty());
    e.ext::<Seen>().form = ACTOR + 0x200;
    notify_values(&mut e, 1.0, 2.0, 123);
    assert!(e.ext::<Seen>().calls.is_empty());
    assert_eq!(floor_integer(f32::NAN), i32::MIN);
    assert_eq!(floor_integer(f32::INFINITY), i32::MIN);
    assert_eq!(floor_integer(-1.2), -2);
}
#[test]
fn set_scope_restores_nested_mode_and_unwinds_after_callback_stop() {
    let mut e = setup();
    change(&mut e, SCRIPT, ACTOR, 10, 0, true);
    tick(&mut e);
    assert!(!SET_MODE.with(Cell::get));
    assert_eq!(
        e.site(SET.addr(), &[OWNER, 10, 10f32.to_bits(), 2f32.to_bits(), 0]),
        Some(0)
    );
    assert!(!SET_MODE.with(Cell::get));
    assert_eq!(
        e.ext::<Seen>().calls[0].3,
        2.0,
        "SET new is replacement, not old+delta"
    );
    SET_MODE.with(|v| v.set(true));
    e.site(SET.addr(), &[OWNER, 10, 10f32.to_bits(), 2f32.to_bits(), 0]);
    assert!(SET_MODE.with(Cell::get));
    SET_MODE.with(|v| v.set(false));
    e.ext::<Seen>().stop = true;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        e.site(SET.addr(), &[OWNER, 10, 10f32.to_bits(), 2f32.to_bits(), 0]);
    }))
    .is_err());
    assert!(!SET_MODE.with(Cell::get));
    assert_eq!(e.ext_free::<Events>().0.lock().unwrap().dispatches, 0);
}
#[test]
fn queue_remove_tick_order_and_no_load_reset_keep_source_lifecycle() {
    let mut e = setup();
    change(&mut e, SCRIPT, ACTOR, 10, 0, true);
    change(&mut e, SCRIPT, ACTOR, 10, 0, false);
    tick(&mut e);
    assert_eq!(sizes(&mut e), (1, 0));
    change(&mut e, SCRIPT, ACTOR, 10, 0, false);
    notify_values(&mut e, 1.0, 2.0, 123);
    assert_eq!(
        e.ext::<Seen>().calls.len(),
        1,
        "deleted remains active until Update"
    );
    tick(&mut e);
    assert_eq!(sizes(&mut e), (0, 0));
    change(&mut e, SCRIPT, ACTOR, 10, 0, true);
    tick(&mut e);
    for kind in [
        nvse::plugins::msg::PRE_LOAD_GAME,
        nvse::plugins::msg::NEW_GAME,
    ] {
        nvse::plugins::dispatch(&mut e, kind, 0, 0);
        assert_eq!(sizes(&mut e), (1, 0));
    }
    for slot in OLD_READS {
        assert_eq!(e.site(slot.addr(), &[ACTOR, 10, 0]), Some(0));
    }
}
