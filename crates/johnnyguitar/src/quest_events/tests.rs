use super::*;

const AREA: u32 = 0x0300_0000;
const PARENT: u32 = AREA + 0x1000;
const A: u32 = AREA + 0x2000;
const B: u32 = AREA + 0x3000;
const DATA: u32 = AREA + 0x4000;
const RESULT: u32 = AREA + 0x5000;
const OFFSET: u32 = RESULT + 8;
const STATE: u32 = AREA + 0x6000;
const Q1: u32 = AREA + 0x7000;
const Q2: u32 = AREA + 0x7100;
const PERK: u32 = AREA + 0x7200;
const ACTOR: u32 = AREA + 0x7300;
const PERK_SCRIPT: u32 = AREA + 0x3800;
const SET_ID: u32 = 0x0301_0000;

fn form(e: &mut Engine, p: u32, kind: u8, id: u32) {
    e.mem.set_u8(p + 4, kind);
    e.mem.set_u32(p + 0xc, id);
}

fn udf(e: &mut Engine, script: u32, id: u32) {
    function(e, script, id, &[(1, 4)]);
}

fn function(e: &mut Engine, script: u32, id: u32, params: &[(u16, u8)]) {
    let data = e.mem.alloc(0x80);
    // Actual compiled scn/Begin Function header from nvse/functions_tests.rs.
    let mut bytes = vec![0x1d, 0, 0, 0, 0x10, 0];
    let mut block = vec![0x0d, 0, 0, 0, 0, 0, 1, params.len() as u8];
    for (index, kind) in params {
        block.extend_from_slice(&index.to_le_bytes());
        block.push(*kind);
    }
    bytes.extend_from_slice(&(block.len() as u16).to_le_bytes());
    bytes.extend(block);
    e.mem.write(data, &bytes);
    e.mem.set_u32(script, SCRIPT_VTABLE);
    form(e, script, 0x11, id);
    e.mem.set_u32(script + 0x20, bytes.len() as u32);
    e.mem.set_u32(script + 0x30, data);
}

fn event_list(e: &mut Engine, script: u32) -> u32 {
    let list = e.mem.alloc(0x14);
    e.mem.set_u32(list, script);
    let mut next = 0;
    for index in [3, 2, 1] {
        let local = e.mem.alloc(0x10);
        e.mem.set_u32(local, index);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, local);
        e.mem.set_u32(node + 4, next);
        next = node;
    }
    e.mem.set_u32(list + 0xc, next);
    list
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
    form(&mut e, Q1, 0x47, 0x0500_7000);
    form(&mut e, Q2, 0x47, 0x0500_7100);
    form(&mut e, PERK, 0x56, 0x0500_7200);
    form(&mut e, ACTOR, 0x3b, 0x0500_7300);
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        event_list(e, a[0]).into_ret()
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
        assert_eq!(&a[3..], [0, 0], "no containing reference");
        let local = nvse::args::event_list_variable(e, a[2], 1);
        assert_ne!(local, 0);
        let form_id = e.mem.u32(local + 8);
        if e.mem.u32(STATE + 0x300) != 0 {
            assert_eq!(
                a[1],
                e.mem.u32(STATE + 0x304),
                "perk callback's actor is thisObj"
            );
            assert_eq!(form_id, e.mem.u32(PERK + 0xc));
            let old = nvse::args::event_list_variable(e, a[2], 2);
            let new = nvse::args::event_list_variable(e, a[2], 3);
            let old_value = e.mem.f64(old + 8);
            let new_value = e.mem.f64(new + 8);
            e.mem.set_f64(STATE + 0x200, old_value);
            e.mem.set_f64(STATE + 0x208, new_value);
        } else {
            assert_eq!(a[1], 0, "quest CallFunctionAlt has thisObj0");
            assert_eq!(form_id, e.mem.u32(Q1 + 0xc));
        }
        let count = e.mem.u32(STATE);
        e.mem.set_u32(STATE, count + 1);
        e.mem.set_u32(STATE + 0x40 + 8 * count, a[0]);
        e.mem.set_u32(STATE + 0x44 + 8 * count, form_id);
        match e.mem.u32(STATE + 4) {
            1 if a[0] == A => {
                change(e, COMPLETE, A, 0, false);
                change(e, COMPLETE, B, 0, true);
            }
            2 if a[0] == A => {
                let lambda = e.mem.u32(STATE + 8);
                change(e, COMPLETE, lambda, 0, false);
                change(e, COMPLETE, lambda, Q1, false);
                tick(e);
                assert_eq!(nvse::lambda::saved_count(e, lambda), 3);
            }
            3 => panic!("test stop inside quest callback"),
            _ => {}
        }
        Ret::default()
    });
    e
}

/// Actual registration record/typed argument decoder; flags are supplied but ignored.
fn command_call(
    e: &mut Engine,
    opcode: u32,
    script: u32,
    add: f64,
    flags: Option<f64>,
    filter: Option<u32>,
) {
    let mut values = vec![b'z'];
    values.extend_from_slice(&add.to_le_bytes());
    values.extend_from_slice(&[b'r', 1, 0]);
    let refs = e.mem.alloc(16);
    e.mem.set_u32(refs + 8, script);
    e.mem.set_u32(PARENT + 0x44, refs);
    let mut count = 2;
    if let Some(flags) = flags {
        count += 1;
        values.push(b'z');
        values.extend_from_slice(&flags.to_le_bytes());
    }
    if let Some(filter) = filter {
        assert!(flags.is_some());
        let next = e.mem.alloc(16);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(next + 8, filter);
        e.mem.set_u32(entry, next);
        e.mem.set_u32(PARENT + 0x48, entry);
        count += 1;
        values.extend_from_slice(&[b'r', 2, 0]);
    } else {
        e.mem.set_u32(PARENT + 0x48, 0);
    }
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, (values.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, count);
    e.mem.write(DATA + 6, &values);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.5);
    let record = table::record_of(e, opcode);
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

fn sizes(e: &mut Engine) -> ([usize; 3], [usize; 3]) {
    let lists = e.ext_free::<QuestEvents>().lists.lock().unwrap();
    (
        std::array::from_fn(|i| lists.active[i].len()),
        std::array::from_fn(|i| lists.queued[i].len()),
    )
}

#[test]
fn registrations_have_exact_records_and_script_guard_and_ignore_flags() {
    let mut e = setup();
    for (opcode, long, short) in [
        (
            0x314c,
            "SetJohnnyOnDyingEventHandler",
            "SetOnDyingEventHandler",
        ),
        (
            0x3158,
            "SetJohnnySeenDataEventHandler",
            "SetSeenDataEventHandler",
        ),
        (
            0x314d,
            "SetJohnnyOnStartQuestEventHandler",
            "SetOnStartQuestEventHandler",
        ),
        (
            0x314e,
            "SetJohnnyOnStopQuestEventHandler",
            "SetOnStopQuestEventHandler",
        ),
        (
            0x318e,
            "SetJohnnyOnRemovePerkEventHandler",
            "SetOnRemovePerkEventHandler",
        ),
        (
            0x317c,
            "SetJohnnyOnCompleteQuestEventHandler",
            "SetOnCompleteQuestEventHandler",
        ),
        (
            0x317d,
            "SetJohnnyOnFailQuestEventHandler",
            "SetOnFailQuestEventHandler",
        ),
        (
            0x318d,
            "SetJohnnyOnAddPerkEventHandler",
            "SetOnAddPerkEventHandler",
        ),
    ] {
        let rec = table::record_of(&e, opcode);
        assert_eq!(table::guest_text(&e, e.mem.u32(rec)), long.as_bytes());
        assert_eq!(table::opcode_by_name(&e, short.as_bytes()), Some(opcode));
        assert_eq!(e.mem.u16(rec + 0x10), 0);
        assert_eq!(e.mem.u16(rec + 0x12), 4);
        assert_eq!(e.mem.u32(rec + 0x20), 0);
        let params = e.mem.u32(rec + 0x14);
        for (i, (name, kind, optional)) in [
            ("setOrRemove", 1, 0),
            ("Script", 0x3d, 0),
            ("flags", 1, 1),
            ("Form", 0x3d, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let slot = params + 12 * i as u32;
            assert_eq!(table::guest_text(&e, e.mem.u32(slot)), name.as_bytes());
            assert_eq!(e.mem.u32(slot + 4), kind);
            assert_eq!(e.mem.u32(slot + 8), optional);
        }
    }
    command_call(&mut e, 0x317c, A, -1.0, Some(0x7fff as f64), Some(Q1));
    command_call(&mut e, 0x317d, B, 1.0, None, None);
    assert_eq!(sizes(&mut e), ([0, 0, 0], [1, 1, 0]));
    e.mem.set_u32(A, SCRIPT_VTABLE + 4);
    command_call(&mut e, 0x317c, A, 1.0, None, None);
    assert_eq!(sizes(&mut e).1, [1, 1, 0]);
    e.mem.set_u32(A, SCRIPT_VTABLE);
    assert!(!crate::plugin().listed);
    assert_eq!(e.mem.u32(STATE), 0);
    tick(&mut e);
    dispatch_complete(&mut e, Q1);
    dispatch_failed(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 2);
}

#[test]
fn raw_filter_identity_dedup_and_removal_have_original_tick_order() {
    let mut e = setup();
    change(&mut e, COMPLETE, A, 0, true);
    change(&mut e, COMPLETE, A, 0, true);
    change(&mut e, COMPLETE, A, 0, false); // Cannot cancel pending registrations.
    dispatch_complete(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 0);
    tick(&mut e);
    change(&mut e, COMPLETE, A, 0, false);
    change(&mut e, COMPLETE, A, 0, true); // Deleted-but-active still deduplicates.
    dispatch_complete(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 1);
    tick(&mut e);
    assert_eq!(sizes(&mut e), ([0, 0, 0], [0, 0, 0]));
    // Same expanded ID but different raw pointer is a distinct registration.
    e.mem.set_u32(Q2 + 0xc, e.mem.u32(Q1 + 0xc));
    change(&mut e, COMPLETE, A, Q1, true);
    change(&mut e, COMPLETE, A, Q2, true);
    tick(&mut e);
    dispatch_complete(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 3);
    change(&mut e, COMPLETE, A, Q1, false);
    tick(&mut e);
    dispatch_complete(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 4);
}

#[test]
fn filters_cache_nested_lists_and_reference_bases_and_empty_sets_are_wildcards() {
    let mut e = setup();
    let outer = e.mem.alloc(0x30);
    let inner = e.mem.alloc(0x30);
    let reference = e.mem.alloc(0x80);
    form(&mut e, outer, 0x55, 0x0600_0001);
    form(&mut e, inner, 0x55, 0x0600_0002);
    form(&mut e, reference, 0x3a, 0x0600_0003);
    e.mem.set_u32(reference + 8, 8); // Initialized reference.
    e.mem.set_u32(reference + 0x20, Q1);
    let list_tail = e.mem.alloc(8);
    e.mem.set_u32(list_tail, inner);
    e.mem.set_u32(outer + 0x1c, list_tail); // Null head item with a next node is not empty.
    e.mem.set_u32(inner + 0x18, reference);
    change(&mut e, COMPLETE, A, outer, true);
    e.mem.set_u32(reference + 0x20, Q2);
    e.mem.set_u32(inner + 0x18, Q2); // Registration must keep its earlier Q1 cache.
    tick(&mut e);
    dispatch_complete(&mut e, Q1);
    let before = e.mem.u32(STATE);
    dispatch_complete(&mut e, Q2);
    assert_eq!(e.mem.u32(STATE), before);
    // STAT is deliberately rejected by FilterForm; the resulting empty set means any valid quest.
    let stat = e.mem.alloc(0x20);
    form(&mut e, stat, 0x20, 0x0600_0004);
    change(&mut e, FAILED, B, stat, true);
    tick(&mut e);
    dispatch_failed(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), before + 1);
    for flags in [1 << 5, 1 << 14, 1 << 21] {
        e.mem.set_u32(Q1 + 8, flags);
        dispatch_failed(&mut e, Q1);
        assert_eq!(e.mem.u32(STATE), before + 1);
    }
    e.mem.set_u32(Q1 + 8, 0);
    e.mem.set_u32(Q1 + 0xc, 0);
    dispatch_failed(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), before + 1);
    dispatch_failed(&mut e, 0);
}

#[test]
fn callback_changes_are_deferred_and_do_not_hold_the_state_lock() {
    let mut e = setup();
    change(&mut e, COMPLETE, A, 0, true);
    tick(&mut e);
    e.mem.set_u32(STATE + 4, 1);
    dispatch_complete(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 1);
    assert_eq!(sizes(&mut e), ([1, 0, 0], [1, 0, 0]));
    tick(&mut e);
    dispatch_complete(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 2);
    assert_eq!(e.mem.u32(STATE + 0x48), B);
}

fn lambda(e: &mut Engine) -> u32 {
    let list = event_list(e, PARENT);
    let data = e.mem.u32(B + 0x30);
    nvse::lambda::create(e, DATA + 0x100, data, 19, PARENT, list)
}

#[test]
fn lambda_retention_is_per_raw_filter_and_bucket_and_nested_tick_keeps_the_snapshot() {
    let mut e = setup();
    let lambda = lambda(&mut e);
    change(&mut e, COMPLETE, A, 0, true);
    change(&mut e, COMPLETE, lambda, 0, true);
    change(&mut e, COMPLETE, lambda, 0, true);
    change(&mut e, COMPLETE, lambda, Q1, true);
    change(&mut e, FAILED, lambda, 0, true);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 3);
    tick(&mut e);
    e.mem.set_u32(STATE + 4, 2);
    e.mem.set_u32(STATE + 8, lambda);
    dispatch_complete(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 3);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 1);
    change(&mut e, FAILED, lambda, 0, false);
    tick(&mut e);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 0);
    assert!(e
        .ext_free::<QuestEvents>()
        .lists
        .lock()
        .unwrap()
        .retired
        .is_empty());
}

#[test]
fn panic_cleanup_releases_the_snapshot_counter_and_load_messages_do_not_flush_quests() {
    let mut e = setup();
    change(&mut e, COMPLETE, A, 0, true);
    tick(&mut e);
    for message in [
        nvse::plugins::msg::PRE_LOAD_GAME,
        nvse::plugins::msg::NEW_GAME,
    ] {
        nvse::plugins::dispatch(&mut e, message, 0, 0);
        assert_eq!(sizes(&mut e).0, [1, 0, 0]);
    }
    e.mem.set_u32(STATE + 4, 3);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dispatch_complete(
            &mut e, Q1
        )))
        .is_err()
    );
    assert_eq!(
        e.ext_free::<QuestEvents>().lists.lock().unwrap().dispatches,
        0
    );
    change(&mut e, COMPLETE, A, 0, false);
    tick(&mut e);
    assert_eq!(sizes(&mut e).0, [0, 0, 0]);
}

#[test]
fn add_perk_forwards_raw_promoted_integer_words_through_actual_alt_udf_extraction() {
    // JohnnyEvents passes ints through EventBase::CallScript unchanged, although
    // Alt interprets numeric words as f32. Cover both actual UDF numeric kinds;
    // do not silently repair this ABI mismatch to logical perk rank values.
    for rank_kind in [1, 0] {
        let mut e = setup();
        function(
            &mut e,
            PERK_SCRIPT,
            0x0500_3800,
            &[(1, 4), (2, rank_kind), (3, rank_kind)],
        );
        e.mem.set_u32(STATE + 0x300, 1);
        e.mem.set_u32(STATE + 0x304, ACTOR);
        command_call(&mut e, 0x318d, PERK_SCRIPT, 1.0, Some(123.0), Some(PERK));
        dispatch_add_perk(&mut e, ACTOR, PERK, 1);
        assert_eq!(e.mem.u32(STATE), 0, "add waits for the update tick");
        tick(&mut e);
        for rank in [0u8, 1, 2, 255] {
            dispatch_add_perk(&mut e, ACTOR, PERK, rank);
            let interpret = |word| {
                let value = f64::from(f32::from_bits(word));
                if rank_kind == 1 {
                    value.floor()
                } else {
                    value
                }
            };
            let expected_old = interpret((i32::from(rank) - 1) as u32);
            let actual_old = e.mem.f64(STATE + 0x200);
            if expected_old.is_nan() {
                assert!(actual_old.is_nan(), "raw FFFFFFFF must reach Alt as NaN");
            } else {
                assert_eq!(actual_old, expected_old);
            }
            assert_eq!(e.mem.f64(STATE + 0x208), interpret(u32::from(rank)));
        }
        assert_eq!(e.mem.u32(STATE), 4);
        dispatch_add_perk(&mut e, ACTOR, 0, 1);
        assert_eq!(e.mem.u32(STATE), 4, "source explicitly rejects null perk");
        // There is no actor validity/type gate in OnAddPerk; it is the UDF calling object.
        e.mem.set_u32(STATE + 0x304, 0);
        dispatch_add_perk(&mut e, 0, PERK, 0);
        assert_eq!(e.mem.u32(STATE), 5);
        assert!(e.mem.f64(STATE + 0x200).is_nan());
    }
}

#[test]
fn add_perk_filters_the_perk_not_actor_and_uses_the_shared_deferred_lifecycle() {
    let mut e = setup();
    function(&mut e, PERK_SCRIPT, 0x0500_3800, &[(1, 4), (2, 1), (3, 1)]);
    e.mem.set_u32(STATE + 0x300, 1);
    e.mem.set_u32(STATE + 0x304, ACTOR);
    e.mem.set_u32(ACTOR + 8, 8);
    let actor_base = e.mem.alloc(0x30);
    form(&mut e, actor_base, 0x2a, 0x0600_0099);
    e.mem.set_u32(ACTOR + 0x20, actor_base);
    command_call(&mut e, 0x318d, PERK_SCRIPT, 1.0, Some(0.0), Some(ACTOR));
    tick(&mut e);
    dispatch_add_perk(&mut e, ACTOR, PERK, 1);
    assert_eq!(
        e.mem.u32(STATE),
        0,
        "actor filter must not match the actor payload"
    );
    command_call(&mut e, 0x318d, PERK_SCRIPT, -1.0, Some(0.0), Some(PERK));
    command_call(&mut e, 0x318d, PERK_SCRIPT, 0.0, Some(0.0), Some(PERK));
    tick(&mut e); // Removing pending did not cancel it.
    assert_eq!(sizes(&mut e).0, [0, 0, 2]);
    dispatch_add_perk(&mut e, ACTOR, PERK, 2);
    assert_eq!(e.mem.u32(STATE), 1);
    command_call(&mut e, 0x318d, PERK_SCRIPT, 0.0, Some(0.0), Some(PERK));
    command_call(&mut e, 0x318d, PERK_SCRIPT, 1.0, Some(0.0), Some(PERK));
    dispatch_add_perk(&mut e, ACTOR, PERK, 2);
    assert_eq!(
        e.mem.u32(STATE),
        2,
        "marked-deleted entry still dispatches before tick"
    );
    tick(&mut e);
    dispatch_add_perk(&mut e, ACTOR, PERK, 2);
    assert_eq!(
        e.mem.u32(STATE),
        2,
        "re-add did not resurrect deleted active entry"
    );
}

#[test]
fn add_perk_lambda_captures_follow_each_raw_filter_and_survive_load_messages() {
    let mut e = setup();
    function(&mut e, PERK_SCRIPT, 0x0500_3800, &[(1, 4), (2, 1), (3, 1)]);
    e.mem.set_u32(STATE + 0x300, 1);
    e.mem.set_u32(STATE + 0x304, ACTOR);
    let list = event_list(&mut e, PARENT);
    let data = e.mem.u32(PERK_SCRIPT + 0x30);
    let size = e.mem.u32(PERK_SCRIPT + 0x20);
    let lambda = nvse::lambda::create(&mut e, DATA + 0x200, data, size, PARENT, list);
    change(&mut e, ADD_PERK, lambda, 0, true);
    change(&mut e, ADD_PERK, lambda, PERK, true);
    change(&mut e, ADD_PERK, lambda, PERK, true);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 2);
    tick(&mut e);
    for message in [
        nvse::plugins::msg::PRE_LOAD_GAME,
        nvse::plugins::msg::NEW_GAME,
    ] {
        nvse::plugins::dispatch(&mut e, message, 0, 0);
        assert_eq!(nvse::lambda::saved_count(&e, lambda), 2);
    }
    dispatch_add_perk(&mut e, ACTOR, PERK, 1);
    assert_eq!(e.mem.u32(STATE), 2);
    change(&mut e, ADD_PERK, lambda, 0, false);
    tick(&mut e);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 1);
    dispatch_add_perk(&mut e, ACTOR, PERK, 1);
    assert_eq!(e.mem.u32(STATE), 3);
    change(&mut e, ADD_PERK, lambda, PERK, false);
    tick(&mut e);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 0);
    assert_eq!(sizes(&mut e).0, [0, 0, 0]);
}

#[test]
fn source_start_stop_pointer_bool_bug_emits_start_even_for_stop_calls() {
    let mut e = setup();
    command_call(&mut e, 0x314d, A, 1.0, Some(99.0), Some(Q1));
    command_call(&mut e, 0x314e, B, 1.0, Some(0.0), Some(Q1));
    dispatch_source_start_stop(&mut e, Q1);
    assert_eq!(e.mem.u32(STATE), 0, "registrations apply at Update");
    tick(&mut e);
    // Each real native enable/disable call reaches the same source-bug hook.
    for _enabled in [true, false] {
        dispatch_source_start_stop(&mut e, Q1);
    }
    assert_eq!(e.mem.u32(STATE), 2);
    assert_eq!(e.mem.u32(STATE + 0x40), A);
    assert_eq!(e.mem.u32(STATE + 0x48), A);
    let lists = e.ext_free::<QuestEvents>().lists.lock().unwrap();
    assert_eq!(
        lists.active[STOP].len(),
        1,
        "real Stop registration survives; source never dispatches it"
    );
}

#[test]
fn remove_perk_checks_teammate_rank_and_passes_actor_and_only_perk() {
    let mut e = setup();
    function(&mut e, PERK_SCRIPT, 0x0500_3800, &[(1, 4)]);
    e.mem.set_u32(STATE + 0x300, 1);
    e.mem.set_u32(STATE + 0x304, ACTOR);
    let vtable = e.mem.alloc(0x500);
    e.mem.set_u32(ACTOR, vtable);
    e.mem.set_u32(vtable + 0x4a0, SET_ID + 4);
    e.register(SET_ID + 4, |e, a| {
        assert_eq!(&a[..2], &[ACTOR, PERK]);
        e.mem.set_u32(STATE + 0x320, a[2]);
        e.mem.u32(STATE + 0x324).into_ret()
    });
    command_call(
        &mut e,
        0x318e,
        PERK_SCRIPT,
        1.0,
        Some(0x7fff as f64),
        Some(PERK),
    );
    tick(&mut e);
    dispatch_remove_perk(&mut e, ACTOR, 0, true);
    dispatch_remove_perk(&mut e, ACTOR, PERK, false);
    assert_eq!(e.mem.u32(STATE), 0);
    e.mem.set_u32(STATE + 0x324, 0x100);
    dispatch_remove_perk(&mut e, ACTOR, PERK, false);
    assert_eq!(e.mem.u32(STATE), 0, "rank is AL, not the whole EAX");
    e.mem.set_u32(STATE + 0x324, 1);
    dispatch_remove_perk(&mut e, ACTOR, PERK, true);
    assert_eq!(e.mem.u32(STATE + 0x320), 1);
    assert_eq!(e.mem.u32(STATE), 1);
    assert_eq!(e.mem.u32(STATE + 0x44), e.mem.u32(PERK + 0xc));
    command_call(&mut e, 0x318e, PERK_SCRIPT, 0.0, Some(0.0), Some(PERK));
    dispatch_remove_perk(&mut e, ACTOR, PERK, false);
    assert_eq!(e.mem.u32(STATE), 2, "deleted entries remain until Update");
    tick(&mut e);
    dispatch_remove_perk(&mut e, ACTOR, PERK, false);
    assert_eq!(e.mem.u32(STATE), 2);
}

mod water_events;
