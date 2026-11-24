use super::*;
const AREA: u32 = 0x0300_0000;
const A: u32 = AREA + 0x1000;
const B: u32 = AREA + 0x1100;
const PARENT: u32 = AREA + 0x1200;
const OPEN: u32 = AREA + 0x2000;
const DATA: u32 = AREA + 0x3000;
const RESULT: u32 = DATA + 0x800;
const ORIGINAL: u32 = AREA + 0x4000;
const SET_ID: u32 = AREA + 0x4010;
const NATIVE_RESULT: Ret = Ret {
    eax: 0x1234,
    edx: 0x5678,
    st0: 9.25,
};
#[derive(Default)]
struct Seen {
    calls: Vec<(u32, f64)>,
    original: Vec<Vec<u32>>,
    mutate: bool,
    stop: bool,
}
fn udf(e: &mut Engine, script: u32, params: bool) {
    let data = e.mem.alloc(0x80);
    let mut code = vec![0x1d, 0, 0, 0, 0x10, 0];
    let mut block = vec![0x0d, 0, 0, 0, 0, 0, 1, u8::from(params)];
    if params {
        block.extend([1, 0, 1]);
    }
    code.extend((block.len() as u16).to_le_bytes());
    code.extend(block);
    e.mem.write(data, &code);
    e.mem.set_u32(script, SCRIPT_VTABLE);
    e.mem.set_u8(script + 4, 0x11);
    e.mem.set_u32(script + 0xc, 0x0500_0000 + script - AREA);
    e.mem.set_u32(script + 0x20, code.len() as u32);
    e.mem.set_u32(script + 0x30, data);
}
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0103_7000, 0x1000);
    e.map(0x0107_6000, 0x1000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_a000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    nvse::table::install(&mut e);
    nvse::functions::install(&mut e);
    e.mem.set_u32(SLEEP_CLICK_VTABLE, ORIGINAL);
    e.register(ORIGINAL, |e, a| {
        e.ext::<Seen>().original.push(a.to_vec());
        e.mem
            .set_u8(a[0] + IS_REST, e.mem.u8(a[0] + IS_REST).wrapping_add(1));
        NATIVE_RESULT
    });
    e.register(0x005e_23f0, |_, _| false.into_ret());
    install(&mut e);
    e.mem
        .set_u32(nvse::functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    udf(&mut e, A, true);
    udf(&mut e, B, false);
    udf(&mut e, PARENT, false);
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, a[0]);
        let local = e.mem.alloc(0x10);
        e.mem.set_u32(local, 1);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, local);
        e.mem.set_u32(list + 0xc, node);
        list.into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(0x0040_1000, |e, a| e.mem.alloc(a[0]).into_ret());
    e.register(0x0048_39c0, |_, _| 0u32.into_ret());
    e.register(0x005a_a0f0, |e, a| {
        e.mem.set_u32(a[0], SCRIPT_VTABLE);
        e.mem.set_u8(a[0] + 4, 0x11);
        a[0].into_ret()
    });
    for address in [0x0046_4d30, 0x005a_b930, 0x005a_b7f0] {
        e.register(address, |_, _| Ret::default());
    }
    e.mem.set_u32(SCRIPT_VTABLE + 0x128, SET_ID);
    e.register(SET_ID, |e, a| {
        e.mem.set_u32(a[0] + 0xc, a[1]);
        Ret::default()
    });
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
        assert_eq!(a[1], 0);
        assert_eq!(a[3..], [0, 0]);
        let local = nvse::args::event_list_variable(e, a[2], 1);
        let value = e.mem.f64(local + 8);
        e.ext::<Seen>().calls.push((a[0], value));
        if e.ext::<Seen>().mutate {
            e.ext::<Seen>().mutate = false;
            change(e, SLEEP, A, 0, false);
            tick(e);
        }
        assert!(!e.ext::<Seen>().stop, "stop inside callback");
        Ret::default()
    });
    e
}
fn call(e: &mut Engine, op: u32, script: u32, toggle: i32, filter: Option<i32>) {
    let refs = e.mem.alloc(16);
    e.mem.set_u32(refs + 8, script);
    e.mem.set_u32(PARENT + 0x44, refs);
    let mut data = vec![b'n'];
    data.extend(toggle.to_le_bytes());
    data.extend([b'r', 1, 0]);
    data.push(b'n');
    data.extend(0x1234i32.to_le_bytes());
    if let Some(filter) = filter {
        data.push(b'n');
        data.extend(filter.to_le_bytes());
    }
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (data.len() + 2) as u16);
    e.mem
        .set_u16(DATA + 4, if filter.is_some() { 4 } else { 3 });
    e.mem.write(DATA + 6, &data);
    e.mem.set_u32(RESULT + 8, 4);
    e.mem.set_f64(RESULT, 998.5);
    let r = table::record_of(e, op);
    let w = [
        e.mem.u32(r + 0x14),
        DATA,
        0,
        0,
        PARENT,
        0,
        RESULT,
        RESULT + 8,
    ];
    assert_eq!(e.call(e.mem.u32(r + 0x18), &w).eax & 0xff, 1);
    assert_eq!(
        e.mem.f64(RESULT),
        998.5,
        "registration leaves result unchanged"
    );
}
#[test]
fn metadata_unsigned_toggle_exact_script_validation_and_queued_lifecycle() {
    let mut e = setup();
    for (op, name, alias, count) in [
        (
            0x317f,
            "SetJohnnyOnSettingsUpdateEventHandler",
            "SetOnSettingsUpdateEventHandler",
            3,
        ),
        (
            0x3211,
            "SetJohnnyOnSleepWaitEventHandler",
            "SetONSleepWEventHandler",
            4,
        ),
    ] {
        let r = table::record_of(&e, op);
        assert_eq!(table::guest_text(&e, e.mem.u32(r)), name.as_bytes());
        assert_eq!(table::guest_text(&e, e.mem.u32(r + 4)), alias.as_bytes());
        assert_eq!(e.mem.u16(r + 0x12), count);
        let p = e.mem.u32(r + 0x14);
        for (i, (kind, optional)) in [(1, 0), (0x3d, 0), (1, 1), (1, 1)]
            .into_iter()
            .take(count as usize)
            .enumerate()
        {
            assert_eq!(e.mem.u32(p + i as u32 * 12 + 4), kind);
            assert_eq!(e.mem.u32(p + i as u32 * 12 + 8), optional);
        }
    }
    call(&mut e, 0x317f, B, -1, None);
    dispatch(&mut e, SETTINGS, 0);
    assert!(e.ext::<Seen>().calls.is_empty());
    tick(&mut e);
    dispatch(&mut e, SETTINGS, 0);
    assert_eq!(e.ext::<Seen>().calls.len(), 1);
    call(&mut e, 0x317f, B, 1, None);
    tick(&mut e);
    assert_eq!(
        e.ext_get::<Events>().unwrap().lists.lock().unwrap().active[SETTINGS].len(),
        1
    );
    e.mem.set_u32(A, 0);
    call(&mut e, 0x3211, A, 1, Some(0));
    tick(&mut e);
    assert!(e.ext_get::<Events>().unwrap().lists.lock().unwrap().active[SLEEP].is_empty());
    call(&mut e, 0x317f, B, 0, None);
    dispatch(&mut e, SETTINGS, 0);
    assert_eq!(
        e.ext::<Seen>().calls.len(),
        2,
        "deleted active entry still called before Update"
    );
    tick(&mut e);
    dispatch(&mut e, SETTINGS, 0);
    assert_eq!(e.ext::<Seen>().calls.len(), 2);
}
#[test]
fn sleep_virtual_wrapper_preserves_result_calls_original_first_and_only_id_four_dispatches() {
    let mut e = setup();
    call(&mut e, 0x3211, A, 1, Some(2));
    tick(&mut e);
    let target = e.mem.u32(SLEEP_CLICK_VTABLE);
    assert_eq!(e.call(target, &[OPEN, 5, 0x9876]), NATIVE_RESULT);
    assert!(e.ext::<Seen>().calls.is_empty());
    e.mem.set_u8(OPEN + IS_REST, 0);
    assert_eq!(e.call(target, &[OPEN, 4, 0x9876]), NATIVE_RESULT);
    assert_eq!(e.ext::<Seen>().calls, [(A, 0.0)]);
    assert_eq!(
        e.ext::<Seen>().original,
        [vec![OPEN, 5, 0x9876], vec![OPEN, 4, 0x9876]]
    );
    e.call(ORIGINAL, &[OPEN, 4, 0]);
    assert_eq!(
        e.ext::<Seen>().calls.len(),
        1,
        "direct original DoClick emits no duplicate event"
    );
}
#[test]
fn filter_zero_is_wildcard_minus_one_matches_nothing_and_uint8_mode_is_not_normalized() {
    let mut e = setup();
    for filter in [0, 1, 2, -1] {
        call(&mut e, 0x3211, A, 1, Some(filter));
    }
    tick(&mut e);
    dispatch(&mut e, SLEEP, OPEN);
    assert_eq!(e.ext::<Seen>().calls, [(A, 0.0), (A, 0.0)]);
    e.ext::<Seen>().calls.clear();
    e.mem.set_u8(OPEN + IS_REST, 1);
    dispatch(&mut e, SLEEP, OPEN);
    assert_eq!(e.ext::<Seen>().calls, [(A, 0.0), (A, 0.0)]);
    e.ext::<Seen>().calls.clear();
    e.mem.set_u8(OPEN + IS_REST, 7);
    dispatch(&mut e, SLEEP, OPEN);
    assert_eq!(e.ext::<Seen>().calls, [(A, 0.0)]);
}
#[test]
fn getter_has_zero_no_menu_and_exact_byte_plus_one_otherwise() {
    let mut e = setup();
    let r = table::record_of(&e, 0x3236);
    assert_eq!(e.mem.u16(r + 0x12), 0);
    assert_eq!(e.mem.u32(r + 0x14), 0);
    for (menu, value, want) in [
        (0, 0, 0.0),
        (OPEN, 0, 1.0),
        (OPEN, 1, 2.0),
        (OPEN, 255, 256.0),
    ] {
        e.mem.set_u32(MENU, menu);
        e.mem.set_u8(OPEN + IS_REST, value);
        get_state(&mut e, &[0, 0, 0, 0, 0, 0, RESULT, 0]);
        assert_eq!(e.mem.f64(RESULT), want);
    }
}
#[test]
fn nested_update_defers_lambda_capture_release_and_panic_retires_dispatch_guard() {
    let mut e = setup();
    let list = e.mem.alloc(0x14);
    e.mem.set_u32(list, PARENT);
    let local = e.mem.alloc(0x10);
    e.mem.set_u32(local, 1);
    let node = e.mem.alloc(8);
    e.mem.set_u32(node, local);
    e.mem.set_u32(list + 0xc, node);
    let data = e.mem.u32(A + 0x30);
    let size = e.mem.u32(A + 0x20);
    let lambda = nvse::lambda::create(&mut e, DATA + 0x100, data, size, PARENT, list);
    change(&mut e, SLEEP, lambda, 0, true);
    change(&mut e, SLEEP, lambda, 2, true);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 2);
    tick(&mut e);
    change(&mut e, SLEEP, lambda, 0, false);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 2);
    // Remove the second bucket during an in-flight snapshot; Update must retain both.
    e.mem.set_u32(DATA + 0x200, lambda);
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, _| {
        let lambda = e.mem.u32(DATA + 0x200);
        change(e, SLEEP, lambda, 2, false);
        tick(e);
        assert_eq!(nvse::lambda::saved_count(e, lambda), 2);
        panic!("stop inside lambda");
    });
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch(&mut e, SLEEP, OPEN)
    }));
    assert!(stop.is_err());
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 0);
    assert_eq!(
        e.ext_get::<Events>()
            .unwrap()
            .lists
            .lock()
            .unwrap()
            .dispatches,
        0
    );
}

#[test]
fn combined_process_and_sleep_virtual_installations_keep_distinct_originals() {
    let mut e = setup();
    const PROCESS_SLOT: u32 = 0x0108_ac7c;
    const PROCESS_ORIGINAL: u32 = AREA + 0x4020;
    e.map(0x0108_a000, 0x1000);
    for offset in [0, 4, 8, 12] {
        e.mem.set_u32(PROCESS_SLOT + offset, PROCESS_ORIGINAL);
    }
    e.register(PROCESS_ORIGINAL, |e, a| {
        e.mem.set_u32(DATA + 0x300, e.mem.u32(DATA + 0x300) + 1);
        assert_eq!(a, [OPEN, 77]);
        Ret {
            eax: 0xfeed,
            edx: 0xabcd,
            st0: -3.0,
        }
    });
    crate::process_events::install(&mut e);
    let process = e.mem.u32(PROCESS_SLOT);
    let sleep = e.mem.u32(SLEEP_CLICK_VTABLE);
    assert_ne!(
        process, sleep,
        "combined modules must not overwrite registered host routine addresses"
    );
    assert_eq!(
        e.call(process, &[OPEN, 77]),
        Ret {
            eax: 0xfeed,
            edx: 0xabcd,
            st0: -3.0
        }
    );
    assert_eq!(e.call(sleep, &[OPEN, 4, 0]), NATIVE_RESULT);
    assert_eq!(e.mem.u32(DATA + 0x300), 1);
    assert_eq!(e.ext::<Seen>().original, [vec![OPEN, 4, 0]]);
}
