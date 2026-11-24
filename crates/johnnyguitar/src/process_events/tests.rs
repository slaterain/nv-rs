use super::*;
const AREA: u32 = 0x0300_0000;
const ACTOR: u32 = AREA + 0x1000;
const BASE: u32 = AREA + 0x1200;
const PROCESS: u32 = AREA + 0x2000;
const OTHER_PROCESS: u32 = AREA + 0x2100;
const SCRIPT: u32 = AREA + 0x3000;
const PARENT: u32 = AREA + 0x3400;
const DATA: u32 = AREA + 0x4000;
const RESULT: u32 = DATA + 0x800;
const OFFSET: u32 = RESULT + 8;
const ORIGINALS: [u32; 4] = [0x0088_1d30, 0x0088_2b90, 0x0088_3240, 0x0088_3800];

#[derive(Default)]
struct Observed {
    originals: usize,
    mode: u32,
    args: Vec<(u32, u32, u32, f64, f64)>,
}
fn form(e: &mut Engine, address: u32, kind: u8, id: u32) {
    e.mem.set_u8(address + 4, kind);
    e.mem.set_u32(address + 8, 8);
    e.mem.set_u32(address + 0xc, id);
}
fn script(e: &mut Engine, address: u32, id: u32) {
    let data = e.mem.alloc(0x80);
    let mut bytes = vec![0x1d, 0, 0, 0, 0x10, 0];
    let block = [0x0d, 0, 0, 0, 0, 0, 1, 3, 1, 0, 4, 2, 0, 1, 3, 0, 1];
    bytes.extend_from_slice(&(block.len() as u16).to_le_bytes());
    bytes.extend(block);
    e.mem.write(data, &bytes);
    e.mem.set_u32(address, SCRIPT_VTABLE);
    form(e, address, 0x11, id);
    e.mem.set_u32(address + 0x20, bytes.len() as u32);
    e.mem.set_u32(address + 0x30, data);
}
fn setup(mapped_slots: bool) -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0103_7000, 0x1000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    if mapped_slots {
        e.map(0x0108_4000, 0x7000);
        for first in FIRST_SLOTS {
            for (i, target) in ORIGINALS.into_iter().enumerate() {
                e.mem.set_u32(first + i as u32 * 4, target);
            }
        }
    }
    for target in ORIGINALS {
        e.register(target, |e, w| {
            assert_eq!(w, [ACTOR]);
            e.ext::<Observed>().originals += 1;
            match e.ext::<Observed>().mode {
                1 => {
                    e.mem.set_u32(PROCESS + 0x28, 0);
                }
                2 => {
                    e.mem.set_u32(ACTOR + 0x68, OTHER_PROCESS);
                }
                _ => {}
            }
            Ret {
                eax: 1,
                edx: 0xabc,
                st0: 7.5,
            }
        });
    }
    nvse::table::install(&mut e);
    nvse::functions::install(&mut e);
    install(&mut e);
    e.mem
        .set_u32(nvse::functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    script(&mut e, SCRIPT, 0x0500_3000);
    script(&mut e, PARENT, 0x0500_3400);
    form(&mut e, BASE, 0x2a, 0x0500_1200);
    form(&mut e, ACTOR, 0x3b, 0x0500_1000);
    e.mem.set_u32(ACTOR + 0x20, BASE);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x28, 3);
    e.mem.set_u32(OTHER_PROCESS + 0x28, 0);
    e.register(0x0040_1000, |e, a| e.mem.alloc(a[0]).into_ret());
    e.register(0x0048_39c0, |_, _| 0u32.into_ret());
    e.register(0x0056_af40, |_, _| 0u32.into_ret());
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, a[0]);
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
        list.into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, w| {
        assert_eq!(w[1], 0, "source passes nullptr calling reference");
        assert_eq!(&w[3..], [0, 0]);
        let actor = nvse::args::event_list_variable(e, w[2], 1);
        let old = nvse::args::event_list_variable(e, w[2], 2);
        let new = nvse::args::event_list_variable(e, w[2], 3);
        let values = (
            w[0],
            w[1],
            e.mem.u32(actor + 8),
            e.mem.f64(old + 8),
            e.mem.f64(new + 8),
        );
        e.ext::<Observed>().args.push(values);
        if e.ext::<Observed>().mode == 3 {
            reset(e);
            tick(e);
            assert_eq!(
                e.ext_free::<ProcessEvents>()
                    .lists
                    .lock()
                    .unwrap()
                    .dispatches,
                1
            );
            panic!("stop during reset inside process callback");
        }
        Ret::default()
    });
    e
}
fn register_command(e: &mut Engine, script: u32, add: f64, integer: Option<i32>) {
    let mut values = vec![b'z'];
    values.extend_from_slice(&add.to_le_bytes());
    values.extend_from_slice(&[b'r', 1, 0]);
    let first = e.mem.alloc(16);
    e.mem.set_u32(first + 8, script);
    e.mem.set_u32(PARENT + 0x44, first);
    let count = if let Some(integer) = integer {
        values.push(b'z');
        values.extend_from_slice(&999f64.to_le_bytes()); // ignored flags
        values.extend_from_slice(&[b'r', 2, 0]);
        let second = e.mem.alloc(16);
        e.mem.set_u32(second + 8, BASE);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, second);
        e.mem.set_u32(PARENT + 0x48, node);
        values.push(b'z');
        values.extend_from_slice(&f64::from(integer).to_le_bytes());
        5
    } else {
        e.mem.set_u32(PARENT + 0x48, 0);
        2
    };
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, (values.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, count);
    e.mem.write(DATA + 6, &values);
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
    assert_eq!(e.mem.f64(RESULT), 1234.5, "source does not set result");
}
fn sizes(e: &mut Engine) -> (usize, usize) {
    let lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
    (lists.active.len(), lists.queued.len())
}

#[test]
fn command_record_real_decoding_script_guard_ignored_flags_and_default_match_none() {
    let mut e = setup(false);
    let record = table::record_of(&e, OPCODE);
    assert_eq!(
        table::opcode_by_name(&e, b"SetOnProcessLevelChangeEventHandler"),
        Some(OPCODE)
    );
    assert_eq!(e.mem.u16(record + 0x10), 0);
    assert_eq!(e.mem.u16(record + 0x12), 5);
    let params = e.mem.u32(record + 0x14);
    for (i, (kind, optional)) in [(1, 0), (0x3d, 0), (1, 1), (0x3d, 1), (1, 1)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            (
                e.mem.u32(params + i as u32 * 12 + 4),
                e.mem.u32(params + i as u32 * 12 + 8)
            ),
            (kind, optional)
        );
    }
    register_command(&mut e, BASE, 1.0, None);
    assert_eq!(sizes(&mut e), (0, 0));
    register_command(&mut e, SCRIPT, -1.0, None);
    register_command(&mut e, SCRIPT, 1.0, None);
    assert_eq!(
        sizes(&mut e),
        (0, 1),
        "uint set/remove and duplicate identity"
    );
    tick(&mut e);
    dispatch(&mut e, ACTOR, 3, 0);
    assert!(
        e.ext::<Observed>().args.is_empty(),
        "empty integer set is match-none"
    );
    register_command(&mut e, SCRIPT, 1.0, Some(0));
    assert_eq!(sizes(&mut e), (1, 1));
    tick(&mut e);
    dispatch(&mut e, ACTOR, 3, 0);
    assert_eq!(
        e.ext::<Observed>().args,
        [(SCRIPT, 0, 0x0500_1000, 0.0, 0.0)],
        "source uint args3/0 are raw varargs, Alt interprets their float bits and floors"
    );
}

#[test]
fn sixteen_virtual_slots_preserve_originals_order_cached_process_and_direct_calls() {
    let mut e = setup(true);
    change(&mut e, SCRIPT, BASE, 0, true);
    tick(&mut e);
    e.ext::<Observed>().mode = 1;
    for (i, first) in FIRST_SLOTS.into_iter().enumerate() {
        for (j, original) in ORIGINALS.into_iter().enumerate() {
            let slot = first + j as u32 * 4;
            let index = i * 4 + j;
            assert_eq!(e.mem.u32(slot), ROUTINES + index as u32 * 4);
            assert_eq!(
                e.ext_free::<ProcessEvents>().originals[index].load(Ordering::Acquire),
                original
            );
            e.mem.set_u32(PROCESS + 0x28, 3);
            let result = e.call(e.mem.u32(slot), &[ACTOR]);
            assert_eq!((result.eax, result.edx, result.st0), (1, 0xabc, 7.5));
            assert_eq!(e.ext::<Observed>().args.len(), index + 1);
            assert_eq!(
                e.ext::<Observed>().originals,
                index + 1,
                "callback after original"
            );
        }
    }
    let saved = e.mem.u32(FIRST_SLOTS[0]);
    install(&mut e);
    assert_eq!(e.mem.u32(FIRST_SLOTS[0]), saved, "install once");
    e.mem.set_u32(PROCESS + 0x28, 3);
    e.call(ORIGINALS[0], &[ACTOR]);
    assert_eq!(
        e.ext::<Observed>().args.len(),
        16,
        "direct calls remain unwrapped"
    );
    e.mem.set_u32(PROCESS + 0x28, 3);
    e.ext::<Observed>().mode = 2;
    e.call(saved, &[ACTOR]);
    assert_eq!(e.mem.u32(ACTOR + 0x68), OTHER_PROCESS);
    assert_eq!(
        e.ext::<Observed>().args.len(),
        16,
        "must reread captured process, not replacement"
    );
    e.mem.set_u32(ACTOR + 0x68, 0);
    e.ext::<Observed>().mode = 0;
    e.call(saved, &[ACTOR]);
    assert_eq!(
        e.ext::<Observed>().originals,
        19,
        "null process still calls original exactly once"
    );
    assert_eq!(e.ext::<Observed>().args.len(), 16);
}

#[test]
fn queue_removal_and_reset_keep_source_update_order_and_pending_adds() {
    let mut e = setup(false);
    change(&mut e, SCRIPT, BASE, 0, true);
    change(&mut e, SCRIPT, BASE, 0, false);
    tick(&mut e);
    assert_eq!(sizes(&mut e), (1, 0), "remove does not erase pending adds");
    change(&mut e, SCRIPT, BASE, 0, false);
    dispatch(&mut e, ACTOR, 3, 0);
    assert_eq!(
        e.ext::<Observed>().args.len(),
        1,
        "marked callbacks run until Update"
    );
    change(&mut e, SCRIPT, BASE, 1, true);
    reset(&mut e);
    assert_eq!(sizes(&mut e), (0, 1), "Reset flushes active only");
    tick(&mut e);
    assert_eq!(sizes(&mut e), (1, 0));
    dispatch(&mut e, ACTOR, 3, 0);
    assert_eq!(
        e.ext::<Observed>().args.len(),
        1,
        "new-level filter enforced"
    );
    dispatch(&mut e, ACTOR, 3, 1);
    assert_eq!(e.ext::<Observed>().args.len(), 2);
    change(&mut e, SCRIPT, BASE, 1, false);
    tick(&mut e);
    assert_eq!(sizes(&mut e), (0, 0));
}

#[test]
fn form_integer_filter_caches_lists_reference_bases_and_rejects_only_xmarker() {
    let mut e = setup(false);
    let list = AREA + 0x5000;
    let xmarker = AREA + 0x5100;
    let static_form = AREA + 0x5200;
    let reference = AREA + 0x5300;
    form(&mut e, list, 0x55, 0x0500_5000);
    form(&mut e, xmarker, 0x20, 0x3b);
    form(&mut e, static_form, 0x20, 0x0500_5200);
    form(&mut e, reference, 0x3b, 0x0500_5300);
    e.mem.set_u32(reference + 0x20, BASE);
    e.mem.set_u32(list + 0x18, reference);
    change(&mut e, SCRIPT, list, 0, true);
    e.mem.set_u32(list + 0x18, static_form);
    tick(&mut e);
    assert!(matches(&mut e, &HashSet::from([0x0500_1200]), ACTOR));
    dispatch(&mut e, ACTOR, 3, 0);
    assert_eq!(
        e.ext::<Observed>().args.len(),
        1,
        "list expansion snapshot retained"
    );
    let mut ids = HashSet::new();
    expand(&e, static_form, &mut ids);
    expand(&e, xmarker, &mut ids);
    assert_eq!(ids, HashSet::from([0x0500_5200]));
    e.mem.set_u32(ACTOR + 8, 8 | (1 << 5));
    dispatch(&mut e, ACTOR, 3, 0);
    assert_eq!(
        e.ext::<Observed>().args.len(),
        1,
        "deleted actor rejected even matching cached base"
    );
}

#[test]
fn nested_reset_and_stop_release_snapshot_accounting_without_holding_callback_lock() {
    let mut e = setup(false);
    change(&mut e, SCRIPT, BASE, 0, true);
    tick(&mut e);
    e.ext::<Observed>().mode = 3;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch(&mut e, ACTOR, 3, 0);
    }))
    .is_err());
    assert_eq!(sizes(&mut e), (0, 0));
    let lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
    assert_eq!(lists.dispatches, 0);
    assert!(lists.retired.is_empty());
    drop(lists);
    dispatch(&mut e, ACTOR, 3, 0);
    assert_eq!(e.ext::<Observed>().args.len(), 1);
}

#[test]
fn reference_filter_checks_leveled_template_then_original_base_after_ordinary_base() {
    let mut e = setup(false);
    let other = AREA + 0x6000;
    let extra = AREA + 0x6100;
    form(&mut e, other, 0x2a, 0x0500_6000);
    e.mem.set_u32(ACTOR + 0x20, other);
    e.register(0x0056_af40, |_, w| {
        assert_eq!(w, [ACTOR]);
        1u32.into_ret()
    });
    e.register(0x0041_0220, |_, w| {
        assert_eq!(w, [ACTOR + 0x44, 0x2e]);
        (AREA + 0x6100).into_ret()
    });
    let ids = HashSet::from([0x0500_1200]);
    e.mem.set_u32(extra + 0x10, BASE);
    assert!(matches(&mut e, &ids, ACTOR));
    e.mem.set_u32(extra + 0x10, 0);
    e.mem.set_u32(extra + 0xc, BASE);
    assert!(matches(&mut e, &ids, ACTOR));
    e.mem.set_u32(extra + 0xc, 0);
    assert!(!matches(&mut e, &ids, ACTOR));
    assert!(matches(&mut e, &HashSet::from([0x0500_1000]), ACTOR));
    assert!(matches(&mut e, &HashSet::new(), ACTOR));
}
