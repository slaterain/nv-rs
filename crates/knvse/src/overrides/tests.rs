use super::*;

fn engine() -> Engine {
    let mut e = crate::tests::engine();
    let aim = nvse::table::guest_str(&mut e, b"Aim");
    e.mem.set_u32(0x0119_77d8 + 17 * 0x24, aim);
    e.register(0x005f_38d0, |_, _| Ret::default());
    e
}
fn files(names: &[&[u8]]) -> Vec<Vec<u8>> {
    names.iter().map(|s| s.to_vec()).collect()
}
fn stack(e: &Engine, kind: usize) -> Vec<Arc<Set>> {
    state(e).lock().unwrap().mods[&(false, 0xff, 17)][kind].clone()
}
#[test]
fn folder_batch_groups_variants_and_disable_removes_the_entire_matching_set() {
    let mut e = engine();
    register_batch(
        &mut e,
        false,
        0,
        files(&[b"One\\Aim.kf", b"Two\\Aim.kf"]),
        true,
        0,
        false,
    )
    .unwrap();
    register_batch(&mut e, false, 0, files(&[b"Three\\Aim.kf"]), true, 0, false).unwrap();
    let sets = stack(&e, 0);
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].paths.len(), 2);
    assert_eq!(sets[1].paths, files(&[b"Three\\Aim.kf"]));
    register_batch(&mut e, false, 0, files(&[b"two\\aim.KF"]), false, 0, false).unwrap();
    assert_eq!(stack(&e, 0).len(), 1);
    assert_eq!(stack(&e, 0)[0].paths, files(&[b"Three\\Aim.kf"]));
}
#[test]
fn existing_path_reorders_stack_without_duplicating_case_insensitive_path() {
    let mut e = engine();
    for name in [b"One\\Aim.kf".as_slice(), b"Two\\Aim.kf", b"Three\\Aim.kf"] {
        register_batch(&mut e, false, 0, files(&[name]), true, 0, false).unwrap();
    }
    register_batch(&mut e, false, 0, files(&[b"ONE\\aim.KF"]), true, 0, false).unwrap();
    let sets = stack(&e, 0);
    assert_eq!(sets.len(), 3);
    assert_eq!(sets[2].paths, files(&[b"One\\Aim.kf"]));
    assert_eq!(sets[0].paths, files(&[b"Two\\Aim.kf"]));
}
#[test]
fn dispatch_allows_callback_mutation_and_unwinds_its_lifetime_guard() {
    let mut e = engine();
    register_batch(&mut e, false, 0, files(&[b"One\\Aim.kf"]), true, 0, false).unwrap();
    let snapshot = stack(&e, 0);
    let stop = catch_unwind(AssertUnwindSafe(|| {
        dispatch(&mut e, |e| {
            register_batch(e, false, 0, files(&[b"One\\Aim.kf"]), false, 0, false).unwrap();
            assert!(stack(e, 0).is_empty());
            assert_eq!(snapshot[0].paths, files(&[b"One\\Aim.kf"]));
            panic!("callback stop");
        })
    }));
    assert!(stop.is_err());
    assert_eq!(state(&e).lock().unwrap().dispatches, 0);
}
#[test]
fn variant_rng_is_native_and_aim_triples_reuse_the_selected_index() {
    let mut e = engine();
    e.register(0x00aa_5230, |e, a| {
        assert_eq!(a, [0x011c_4180, 3]);
        let n = e.mem.u32(0x0300_3000);
        e.mem.set_u32(0x0300_3000, n + 1);
        Ret {
            eax: 2,
            ..Ret::default()
        }
    });
    let set = Set {
        paths: files(&[b"a", b"b", b"c"]),
        condition: 0,
        poll: false,
    };
    assert_eq!(variant(&mut e, &set, 17), b"c");
    assert_eq!(variant(&mut e, &set, 18), b"c");
    assert_eq!(variant(&mut e, &set, 19), b"c");
    assert_eq!(e.mem.u32(0x0300_3000), 1);
    assert_eq!(variant(&mut e, &set, 20), b"c");
    assert_eq!(e.mem.u32(0x0300_3000), 2);
}
#[test]
fn unsupported_variant_is_an_error_without_publishing_a_registry_entry() {
    let mut e = engine();
    assert!(register_batch(
        &mut e,
        false,
        0,
        files(&[b"Aim_order_2.kf"]),
        true,
        0,
        false
    )
    .unwrap_err()
    .contains("unsupported"));
    assert!(state(&e).lock().unwrap().mods.is_empty());
}

#[test]
fn json_folder_skips_an_unreadable_kf_without_splitting_variants() {
    let mut e = engine();
    e.register(0x00af_df20, |_, _| Ret::default());
    let animations = files(&[b"One\\Aim.kf", b"Missing.kf", b"Two\\Aim.kf"]);
    let error = register_batch(&mut e, false, 0, animations.clone(), true, 0, false).unwrap_err();
    assert!(error.contains("Missing.kf"));
    assert_eq!(stack(&e, 0)[0].paths, files(&[b"One\\Aim.kf"]));

    let mut e = engine();
    e.register(0x00af_df20, |_, _| Ret::default());
    let entry = paths::JsonEntry {
        folder: b"TestFolder".to_vec(),
        form: 0,
        condition: 0,
        poll: false,
        priority: 0,
    };
    register_json_folder(&mut e, &entry, false, animations);
    let sets = stack(&e, 0);
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].paths, files(&[b"One\\Aim.kf", b"Two\\Aim.kf"]));
}

const NOTE: u32 = 0x0300_8000;
const SCRIPT_VT: u32 = 0x0103_7094;
fn actor_fixture(e: &mut Engine) -> (u32, u32) {
    let actor = e.mem.alloc(0x800);
    let animation = e.mem.alloc(0x140);
    let base = e.mem.alloc(0x200);
    let process = e.mem.alloc(0x200);
    let process_vt = e.mem.alloc(0x200);
    let mover = e.mem.alloc(0x40);
    let mover_vt = e.mem.alloc(0x40);
    let av_vt = e.mem.alloc(0x20);
    e.mem.set_u32(actor + 0xc, 0x0500_0014);
    e.mem.set_u32(base + 0xc, 0x0500_0007);
    e.mem.set_u32(actor + 0x20, base);
    e.mem.set_u32(actor + 0x68, process);
    e.mem.set_u32(process, process_vt);
    e.mem.set_u32(actor + 0x190, mover);
    e.mem.set_u32(mover, mover_vt);
    e.mem.set_u32(actor + 0xa4, av_vt);
    e.mem.set_u32(animation + 4, actor);
    e.mem.set_u32(process_vt + 0x148, NOTE + 0x100);
    e.mem.set_u32(mover_vt + 0x20, NOTE + 0x104);
    e.mem.set_u32(av_vt + 8, NOTE + 0x108);
    e.register(NOTE + 0x100, |_, _| 0u32.into_ret());
    e.register(NOTE + 0x104, |_, _| 0u32.into_ret());
    e.register(NOTE + 0x108, |_, _| 100u32.into_ret());
    e.register(0x00ec_43fb, |_, a| {
        if a[3] == rtti::ACTOR {
            a[0].into_ret()
        } else {
            0u32.into_ret()
        }
    });
    e.register(0x008b_a3e0, |_, _| 0u32.into_ret());
    e.register(0x008b_a410, |_, _| 0u32.into_ret());
    (actor, animation)
}
fn udf(e: &mut Engine, value: f64) -> u32 {
    nvse::functions::install(e);
    e.mem
        .set_u32(nvse::functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    let script = e.mem.alloc(0x80);
    let data = e.mem.alloc(0x100);
    e.mem.write(
        data,
        &[0x1d, 0, 0, 0, 0x10, 0, 8, 0, 0x0d, 0, 0, 0, 0, 0, 1, 0],
    );
    let mut expression = vec![b'Z'];
    expression.extend(value.to_le_bytes());
    let body = [
        vec![1],
        ((expression.len() + 2) as u16).to_le_bytes().to_vec(),
        expression,
    ]
    .concat();
    e.mem.write(data + 0x40, &body);
    e.mem.set_u32(script, SCRIPT_VT);
    e.mem.set_u8(script + 4, 0x11);
    e.mem.set_u32(script + 0xc, 0x0500_0100);
    e.mem.set_u32(script + 0x20, 0x40 + body.len() as u32);
    e.mem.set_u32(script + 0x30, data);
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, a[0]);
        list.into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    // This is the real UserFunction/SetFunctionValue boundary, with only the
    // game's Script::Execute entry replaced. No compiler success is simulated.
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
        assert_eq!(a[1], e.mem.u32(NOTE));
        e.mem.set_u32(NOTE + 4, e.mem.u32(NOTE + 4) + 1);
        if e.mem.u8(NOTE + 8) != 0 {
            register_batch(
                e,
                false,
                0,
                files(&[b"Conditional\\Aim.kf"]),
                false,
                0,
                false,
            )
            .unwrap();
            assert_eq!(state(e).lock().unwrap().dispatches, 1);
        }
        let record = nvse::table::record_of(e, 0x1546);
        let execute = e.mem.u32(record + 0x18);
        let params = e.mem.u32(record + 0x14);
        let data = e.mem.u32(a[0] + 0x30);
        e.with_stack(12, |e, f| {
            e.mem.set_u32(f.addr() + 8, 0x40);
            e.call(
                execute,
                &[params, data, a[1], a[3], a[0], a[2], f.addr(), f.addr() + 8],
            );
        });
        Ret::default()
    });
    script
}
#[test]
fn actual_user_function_value_controls_selection_and_can_remove_its_snapshot() {
    let mut e = engine();
    let (actor, animation) = actor_fixture(&mut e);
    e.mem.set_u32(NOTE, actor);
    let script = udf(&mut e, 1.0);
    register_batch(
        &mut e,
        false,
        0,
        files(&[b"Fallback\\Aim.kf"]),
        true,
        0,
        false,
    )
    .unwrap();
    register_batch(
        &mut e,
        false,
        0,
        files(&[b"Conditional\\Aim.kf"]),
        true,
        script,
        false,
    )
    .unwrap();
    e.mem.set_u8(NOTE + 8, 1);
    let selected = dispatch(&mut e, |e| select(e, animation, 17)).unwrap();
    assert_eq!(selected.paths, files(&[b"Conditional\\Aim.kf"]));
    assert_eq!(e.mem.u32(NOTE + 4), 1);
    assert_eq!(
        dispatch(&mut e, |e| select(e, animation, 17))
            .unwrap()
            .paths,
        files(&[b"Fallback\\Aim.kf"])
    );
    assert_eq!(state(&e).lock().unwrap().dispatches, 0);
}
#[test]
fn real_false_condition_falls_back_and_actor_specific_override_precedes_global() {
    let mut e = engine();
    let (actor, animation) = actor_fixture(&mut e);
    e.mem.set_u32(NOTE, actor);
    let script = udf(&mut e, 0.0);
    register_batch(
        &mut e,
        false,
        0,
        files(&[b"Fallback\\Aim.kf"]),
        true,
        0,
        false,
    )
    .unwrap();
    register_batch(
        &mut e,
        false,
        0,
        files(&[b"Conditional\\Aim.kf"]),
        true,
        script,
        false,
    )
    .unwrap();
    assert_eq!(
        dispatch(&mut e, |e| select(e, animation, 17))
            .unwrap()
            .paths,
        files(&[b"Fallback\\Aim.kf"])
    );
    register_batch(
        &mut e,
        false,
        actor,
        files(&[b"Actor\\Aim.kf"]),
        true,
        0,
        false,
    )
    .unwrap();
    assert_eq!(
        dispatch(&mut e, |e| select(e, animation, 17))
            .unwrap()
            .paths,
        files(&[b"Actor\\Aim.kf"])
    );
    assert_eq!(e.mem.u32(NOTE + 4), 1);
}
#[test]
fn actual_command_record_extracts_required_arguments_and_rejects_malformed_payload() {
    let mut e = engine();
    let record = nvse::table::record_of(&e, 0x3922);
    let params = e.mem.u32(record + 0x14);
    assert_eq!(e.mem.u16(record + 0x10), 0);
    assert_eq!(e.mem.u16(record + 0x12), 5);
    for (i, (kind, optional)) in [(1, 0), (1, 0), (0, 0), (1, 1), (0x3d, 1)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(e.mem.u32(params + i as u32 * 12 + 4), kind);
        assert_eq!(e.mem.u32(params + i as u32 * 12 + 8), optional);
    }
    let path = b"Global\\Aim.kf";
    let mut data = vec![b'n'];
    data.extend(0i32.to_le_bytes());
    data.push(b'n');
    data.extend(1i32.to_le_bytes());
    data.extend((path.len() as u16).to_le_bytes());
    data.extend(path);
    let addr = 0x0300_0000;
    let result = addr + 0x1000;
    let offset = result + 8;
    for (index, (count, payload, want)) in [(3, data.as_slice(), 1.0), (3, b"x".as_slice(), 0.0)]
        .into_iter()
        .enumerate()
    {
        let addr = addr + index as u32 * 0x100;
        e.mem.set_u16(addr, 0x3922);
        e.mem.set_u16(addr + 2, (payload.len() + 2) as u16);
        e.mem.set_u16(addr + 4, count);
        e.mem.write(addr + 6, payload);
        e.mem.set_u32(offset, 4);
        e.mem.set_f64(result, 99.0);
        e.call(
            e.mem.u32(record + 0x18),
            &[params, addr, 0, 0, 0, 0, result, offset],
        );
        assert_eq!(e.mem.f64(result), want);
    }
    assert_eq!(stack(&e, 0)[0].paths, files(&[path]));
}

#[test]
fn lambda_capture_refs_are_per_set_and_retired_only_after_real_udf_returns() {
    let mut e = engine();
    let (actor, animation) = actor_fixture(&mut e);
    e.mem.set_u32(NOTE, actor);
    let parent = udf(&mut e, 1.0);
    e.map(0x0103_7000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    e.register(0x0040_1000, |e, a| e.mem.alloc(a[0]).into_ret());
    e.register(0x0048_39c0, |_, _| 0u32.into_ret());
    e.register(0x005a_a0f0, |e, a| {
        e.mem.set_u32(a[0], SCRIPT_VT);
        e.mem.set_u8(a[0] + 4, 0x11);
        a[0].into_ret()
    });
    for address in [0x0046_4d30, 0x005a_b930, 0x005a_b7f0] {
        e.register(address, |_, _| Ret::default());
    }
    e.mem.set_u32(SCRIPT_VT + 0x128, NOTE + 0x120);
    e.register(NOTE + 0x120, |e, a| {
        e.mem.set_u32(a[0] + 0xc, a[1]);
        Ret::default()
    });
    let list = e.mem.alloc(0x14);
    e.mem.set_u32(list, parent);
    let data = e.mem.u32(parent + 0x30);
    let lambda = nvse::lambda::create(&mut e, NOTE + 0x200, data, 0x80, parent, list);
    assert!(nvse::lambda::is_lambda(&e, lambda));
    register_batch(
        &mut e,
        false,
        0,
        files(&[b"Conditional\\Aim.kf"]),
        true,
        lambda,
        false,
    )
    .unwrap();
    register_batch(
        &mut e,
        false,
        0,
        files(&[b"Other\\Aim.kf"]),
        true,
        lambda,
        false,
    )
    .unwrap();
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 2);
    // Snapshot the first set; removing both while inside dispatch must retain
    // its captured list until the actual UserFunction call has completed.
    let snapshot = stack(&e, 0);
    dispatch(&mut e, |e| {
        register_batch(
            e,
            false,
            0,
            files(&[b"Conditional\\Aim.kf", b"Other\\Aim.kf"]),
            false,
            0,
            false,
        )
        .unwrap();
        assert_eq!(nvse::lambda::saved_count(e, lambda), 2);
        assert_eq!(condition(e, snapshot[0].condition, actor), Some(1.0));
        assert_eq!(nvse::lambda::saved_count(e, lambda), 2);
        assert!(select(e, animation, 17).is_none());
    });
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 0);
    assert!(state(&e).lock().unwrap().retired.is_empty());
}

#[test]
fn polling_false_condition_replays_fitted_native_group_and_removes_stale_track() {
    let mut e = engine();
    let (actor, animation) = actor_fixture(&mut e);
    e.mem.set_u32(NOTE, actor);
    let actor_vt = e.mem.alloc(0x240);
    e.mem.set_u32(actor, actor_vt);
    e.mem.set_u32(actor_vt + 0x22c, NOTE + 0x128);
    e.register(NOTE + 0x128, |_, a| {
        assert_eq!(a[1], 1);
        false.into_ret()
    });
    let process = e.mem.u32(actor + 0x68);
    let vt = e.mem.u32(process);
    e.mem.set_u32(vt + 0x1b8, NOTE + 0x12c);
    e.mem.set_u32(process + 0x1c0, animation);
    e.register(NOTE + 0x12c, |e, a| e.mem.u32(a[0] + 0x1c0).into_ret());
    e.register(0x0048_39c0, |e, a| {
        let actor = e.mem.u32(NOTE);
        assert_eq!(a, [e.mem.u32(actor + 0xc)]);
        actor.into_ret()
    });
    let sequence = e.mem.alloc(0x80);
    let group = e.mem.alloc(0x40);
    e.mem.set_u32(sequence + 0x74, group);
    e.mem.set_u16(group + 0x10, 17);
    e.mem.set_u32(sequence + 0x44, 1);
    e.mem.set_u32(animation + 0xe0, sequence);
    let script = udf(&mut e, 0.0);
    let set = Set {
        paths: files(&[b"Conditional\\Aim.kf"]),
        condition: script,
        poll: true,
    };
    let key = track(&mut e, animation, actor, 17, &set);
    state(&e)
        .lock()
        .unwrap()
        .polled
        .get_mut(&key)
        .unwrap()
        .sequence = sequence;
    e.register(0x0089_7910, |e, a| {
        assert_eq!(
            a,
            [
                e.mem.u32(NOTE),
                17,
                0,
                0,
                e.mem.u32(e.mem.u32(e.mem.u32(NOTE) + 0x68) + 0x1c0)
            ]
        );
        0x111u32.into_ret()
    });
    e.register(0x0049_4740, |e, a| {
        assert_eq!(a[1..], [0x111, 1, u32::MAX, u32::MAX]);
        e.mem.set_u32(NOTE + 12, e.mem.u32(NOTE + 12) + 1);
        Ret::default()
    });
    tick(&mut e);
    assert_eq!(e.mem.u32(NOTE + 12), 1);
    assert_eq!(e.mem.u32(NOTE + 4), 1);
    e.mem.set_u32(sequence + 0x44, 0);
    e.mem.set_u32(sequence + 0x24, 1);
    tick(&mut e);
    assert!(state(&e).lock().unwrap().polled.is_empty());
    assert_eq!(e.mem.u32(NOTE + 4), 1);
}
