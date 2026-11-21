use super::*;

const AREA: u32 = 0x0300_0000;
const PARENT: u32 = AREA + 0x1000;
const A: u32 = AREA + 0x2000;
const B: u32 = AREA + 0x3000;
const DATA: u32 = AREA + 0x4000;
const RESULT: u32 = AREA + 0x5000;
const OFFSET: u32 = RESULT + 8;
const STATE: u32 = AREA + 0x6000;
const VT: u32 = 0x0103_7094;
const SET_ID: u32 = 0x0301_0000;

fn udf(e: &mut Engine, script: u32, id: u32) {
    let data = e.mem.alloc(0x80);
    // xNVSE's no-parameter Begin Function, as functions_tests.rs builds it.
    e.mem.write(
        data,
        &[0x1d, 0, 0, 0, 0x10, 0, 8, 0, 0x0d, 0, 0, 0, 0, 0, 1, 0],
    );
    e.mem.set_u32(script, VT);
    e.mem.set_u8(script + 4, 0x11);
    e.mem.set_u32(script + 0xc, id);
    e.mem.set_u32(script + 0x20, 16);
    e.mem.set_u32(script + 0x30, data);
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
    crate::install(&mut e);
    e.mem
        .set_u32(nvse::functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    udf(&mut e, A, 0x0500_2000);
    udf(&mut e, B, 0x0500_3000);
    udf(&mut e, PARENT, 0x0500_1000);
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        let p = e.mem.alloc(0x14);
        e.mem.set_u32(p, a[0]);
        p.into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(0x0040_1000, |e, a| e.mem.alloc(a[0]).into_ret());
    e.register(0x0048_39c0, |_, _| 0u32.into_ret());
    e.register(0x005a_a0f0, |e, a| {
        e.mem.set_u32(a[0], VT);
        e.mem.set_u8(a[0] + 4, 0x11);
        a[0].into_ret()
    });
    for address in [0x0046_4d30, 0x005a_b930, 0x005a_b7f0] {
        e.register(address, |_, _| Ret::default());
    }
    e.mem.set_u32(VT + 0x128, SET_ID);
    e.register(SET_ID, |e, a| {
        e.mem.set_u32(a[0] + 0xc, a[1]);
        Ret::default()
    });
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
        assert_eq!(&a[1..2], [0], "render UDF has no calling reference");
        assert_eq!(&a[3..], [0, 0], "nor containing reference");
        let count = e.mem.u32(STATE);
        e.mem.set_u32(STATE, count + 1);
        e.mem.set_u32(STATE + 0x40 + 4 * count, a[0]);
        match e.mem.u32(STATE + 4) {
            1 if a[0] == A => {
                change(e, A, false, 0);
                change(e, B, true, 0);
            }
            2 if a[0] == A => {
                let lambda = e.mem.u32(STATE + 8);
                change(e, lambda, false, 0);
                tick(e);
                assert_eq!(nvse::lambda::saved_count(e, lambda), 2);
            }
            3 => panic!("test stop inside render UDF"),
            _ => {}
        }
        Ret::default()
    });
    e
}

fn command_call(e: &mut Engine, script: u32, values: &[f64]) {
    let refs = e.mem.alloc(0x10);
    e.mem.set_u32(refs + 8, script);
    e.mem.set_u32(PARENT + 0x44, refs);
    let mut args = vec![b'z'];
    args.extend_from_slice(&values[0].to_le_bytes());
    args.extend_from_slice(&[b'r', 1, 0]);
    for value in &values[1..] {
        args.push(b'z');
        args.extend_from_slice(&value.to_le_bytes());
    }
    e.mem.set_u16(DATA, 0x319f);
    e.mem.set_u16(DATA + 2, (args.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, (values.len() + 1) as u16);
    e.mem.write(DATA + 6, &args);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.5);
    let record = nvse::table::record_of(e, 0x319f);
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
    assert_eq!(e.call(e.mem.u32(record + 0x18), &words).eax & 0xff, 1);
    assert_eq!(e.mem.f64(RESULT), 1234.5, "JG leaves the result unchanged");
}

fn sizes(e: &mut Engine) -> ([usize; 3], [usize; 3]) {
    let lists = e.ext_free::<Render>().lists.lock().unwrap();
    (
        std::array::from_fn(|i| lists.active[i].len()),
        std::array::from_fn(|i| lists.queued[i].len()),
    )
}

#[test]
fn actual_command_uses_fourth_flags_and_keeps_its_result() {
    let mut e = setup();
    // Third argument 4 is ignored; default fourth argument selects game + menu.
    command_call(&mut e, A, &[-1., 4.]);
    assert_eq!(sizes(&mut e), ([0, 0, 0], [0, 1, 1]));
    // Script form required, without requiring Begin Function at registration.
    e.mem.set_u32(B, VT + 4); // Right type byte, wrong concrete Script vtable.
    command_call(&mut e, B, &[1., 0., 4.]);
    assert_eq!(sizes(&mut e).1, [0, 1, 1]);
    e.mem.set_u32(B, VT);
    command_call(&mut e, B, &[1., 0., 5.]);
    assert_eq!(sizes(&mut e).1, [1, 1, 1]);
}

#[test]
fn unlisted_listener_updates_events_with_editor_ids_off_and_keeps_them_across_load() {
    let mut e = setup();
    assert!(!crate::on(&e));
    let plugins = nvse::plugins::built_ins(&e);
    assert_eq!(plugins.len(), 1);
    assert!(!plugins[0].listed);
    command_call(&mut e, A, &[1.]);
    assert_eq!(sizes(&mut e).0, [0, 0, 0]);
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);
    assert_eq!(sizes(&mut e), ([0, 1, 1], [0, 0, 0]));
    for message in [
        nvse::plugins::msg::PRE_LOAD_GAME,
        nvse::plugins::msg::NEW_GAME,
    ] {
        nvse::plugins::dispatch(&mut e, message, 0, 0);
        assert_eq!(sizes(&mut e).0, [0, 1, 1]);
    }
    assert_eq!(e.site(SITES[GAME].addr(), &[]), None);
    assert_eq!(e.mem.u32(STATE), 1);
}

#[test]
fn every_mode_mask_selects_only_its_original_buckets() {
    for flags in 0..8 {
        let mut e = setup();
        command_call(&mut e, A, &[1., 0., f64::from(flags)]);
        tick(&mut e);
        let expected = [
            flags & 2 == 0 && flags & 4 != 0,
            flags & 2 == 0 && flags & 4 == 0,
            flags & 1 == 0,
        ];
        for (bucket, selected) in expected.into_iter().enumerate() {
            let before = e.mem.u32(STATE);
            assert_eq!(e.site(SITES[bucket].addr(), &[]), None);
            assert_eq!(e.mem.u32(STATE) - before, u32::from(selected));
        }
    }
}

#[test]
fn pending_dedup_and_active_removal_are_deferred_to_the_tick() {
    let mut e = setup();
    change(&mut e, A, true, 0);
    change(&mut e, A, true, 0);
    change(&mut e, A, false, 0); // Pending registration is not removed by JG.
    assert_eq!(sizes(&mut e).1, [0, 1, 1]);
    game(&mut e, &[]);
    assert_eq!(e.mem.u32(STATE), 0);
    tick(&mut e);
    change(&mut e, A, false, 0);
    change(&mut e, A, true, 0); // Deleted-but-active entry still deduplicates.
    game(&mut e, &[]);
    menu(&mut e, &[]);
    assert_eq!(e.mem.u32(STATE), 2);
    tick(&mut e);
    assert_eq!(sizes(&mut e), ([0, 0, 0], [0, 0, 0]));
    assert_eq!(
        e.ext_free::<Render>().active_mask.load(Ordering::Acquire),
        0
    );
}

#[test]
fn callbacks_can_register_and_remove_without_a_held_state_lock() {
    let mut e = setup();
    e.mem.set_u32(STATE + 4, 1);
    change(&mut e, A, true, 0);
    tick(&mut e);
    game(&mut e, &[]);
    assert_eq!(e.mem.u32(STATE), 1);
    assert_eq!(sizes(&mut e).1, [0, 1, 1]);
    tick(&mut e);
    game(&mut e, &[]);
    assert_eq!(e.mem.u32(STATE), 2);
    assert_eq!(e.mem.u32(STATE + 0x44), B);
}

fn lambda(e: &mut Engine) -> u32 {
    let list = e.mem.alloc(0x14);
    e.mem.set_u32(list, PARENT);
    let data = e.mem.u32(B + 0x30);
    nvse::lambda::create(e, DATA + 0x100, data, 16, PARENT, list)
}

#[test]
fn lambda_captures_are_per_bucket_and_snapshot_lives_through_nested_tick() {
    let mut e = setup();
    let lambda = lambda(&mut e);
    change(&mut e, A, true, 0);
    change(&mut e, lambda, true, 0);
    change(&mut e, lambda, true, 0);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 2);
    tick(&mut e);
    e.mem.set_u32(STATE + 4, 2);
    e.mem.set_u32(STATE + 8, lambda);
    game(&mut e, &[]);
    assert_eq!(
        e.mem.u32(STATE),
        2,
        "snapshot lambda still runs after nested tick removes it"
    );
    assert_eq!(e.mem.u32(STATE + 0x44), lambda);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 0);
    assert!(e
        .ext_free::<Render>()
        .lists
        .lock()
        .unwrap()
        .retired
        .is_empty());
}

#[test]
fn a_stopped_udf_does_not_leave_the_dispatch_counter_held() {
    let mut e = setup();
    change(&mut e, A, true, 0);
    tick(&mut e);
    e.mem.set_u32(STATE + 4, 3);
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| game(&mut e, &[])));
    assert!(stopped.is_err());
    assert_eq!(e.ext_free::<Render>().lists.lock().unwrap().dispatches, 0);
    change(&mut e, A, false, 0);
    tick(&mut e);
    assert_eq!(sizes(&mut e).0, [0, 0, 0]);
}
