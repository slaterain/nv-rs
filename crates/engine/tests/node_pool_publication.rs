//! Shared node-pool publication through actual render-list releases, invented data only.
//! The pool is also used by UI lists. No game files or renderer are needed.
use engine as _;
use engine_core::{args, exe::Import, real_threads, Engine, Ptr, Ret};
use std::sync::atomic::Ordering;

const CREATE: u32 = 0x00fd_f200;
const WAIT: u32 = 0x00fd_f204;
const WORKER: u32 = 0x0300_0000;
const TEST_ROOT: u32 = 0x0300_1000;
const ENTER_POOL: u32 = 0x0040_fbf0;
const POOL_FREE: u32 = 0x011c_5f58;
const POOL_LOCK: u32 = 0x011c_5f80;
const NODE_COUNT: u32 = 32;
const CYCLES: u32 = 100_000;

fn fail(e: &Engine, p: u32, code: u32) {
    let _ = e.mem.atomic_u32(p + 8).unwrap().compare_exchange(
        0,
        code,
        Ordering::SeqCst,
        Ordering::Relaxed,
    );
}

/// The real acquire, then a synthetic protected payload. The production release's
/// own inlined final unlock must publish these writes; the test does not unlock it.
fn observed_acquire(e: &mut Engine, a: &[u32]) -> Ret {
    assert_eq!(a[0], POOL_LOCK);
    engine_typed::dispatch::refs::lock(e, Ptr::new(a[0]), a[1]);
    let p = e.mem.u32(TEST_ROOT);
    let value = e.mem.atomic_u32(p + 32).unwrap().load(Ordering::Relaxed);
    for off in (32..96).step_by(4) {
        if e.mem.atomic_u32(p + off).unwrap().load(Ordering::Relaxed) != value {
            fail(e, p, 5);
        }
    }
    for off in (32..96).step_by(4) {
        e.mem
            .atomic_u32(p + off)
            .unwrap()
            .store(value + 1, Ordering::Relaxed);
    }
    Ret::default()
}

fn worker(e: &mut Engine, a: &[u32]) -> Ret {
    let control = a[0];
    let p = e.mem.u32(control);
    let list = control + 8; // A distinct PassList/NiTPointerList/accumulator per thread.
    let base = e.mem.u32(p);
    let busy = e.mem.u32(p + 4);
    let case = e.mem.u32(p + 16);
    let mut done = 0;
    for i in 0..CYCLES {
        if e.mem.atomic_u32(p + 8).unwrap().load(Ordering::Relaxed) != 0 {
            break;
        }
        let node = engine_typed::draw_f::helpers::allocate_node(e, Ptr::NULL).addr();
        if node < base || node >= base + 12 * NODE_COUNT || (node - base) % 12 != 0 {
            fail(e, p, 1);
            break;
        }
        let flag = busy + 4 * ((node - base) / 12);
        if e.mem.atomic_u32(flag).unwrap().swap(1, Ordering::SeqCst) != 0 {
            fail(e, p, 2);
            break;
        }
        for off in [0, 4, 8] {
            if e.mem
                .atomic_u32(node + off)
                .unwrap()
                .load(Ordering::Relaxed)
                != 0
            {
                fail(e, p, 3);
            }
        }
        for off in [0, 4, 8] {
            e.mem
                .atomic_u32(node + off)
                .unwrap()
                .store(i + 1, Ordering::Relaxed);
        }
        if i % 256 == 0 {
            std::thread::yield_now();
        }
        for off in [0, 4, 8] {
            if e.mem
                .atomic_u32(node + off)
                .unwrap()
                .load(Ordering::Relaxed)
                != i + 1
            {
                fail(e, p, 4);
            }
        }
        e.mem.set_u32(node, 0); // One-node list, never a second live owner.
        match case {
            0 | 1 => {
                e.mem.set_u32(list + 8, 0);
                e.mem.set_u32(list + 0xc, node);
            }
            2 => {
                e.mem.set_u32(list, node);
                e.mem.set_u32(list + 4, node);
                e.mem.set_u32(list + 8, 1);
            }
            3 => {
                e.mem.set_u32(list + 0xc, node);
                e.mem.set_u32(list + 0x10, node);
                e.mem.set_u32(list + 0x14, 1);
            }
            _ => unreachable!(),
        }
        // Relinquish the test marker BEFORE the real release publishes the node.
        e.mem.atomic_u32(flag).unwrap().store(0, Ordering::Release);
        match case {
            0 => {
                engine_typed::shader::batch::release_spare(e, Ptr::new(list));
            }
            1 => {
                engine::units::bsshader::bsshaderaccumulator::fn_00b63e90(e, Ptr::new(list));
            }
            2 => {
                let item = engine::units::bsshader::run_00b5fd60::fn_00b602d0(e, Ptr::new(list));
                assert_eq!(item, i + 1);
            }
            3 => {
                engine_typed::shader::accumulator::draw_sorted(e, Ptr::new(list));
            }
            _ => unreachable!(),
        }
        done += 1;
    }
    e.mem
        .atomic_u32(p + 12)
        .unwrap()
        .fetch_add(done, Ordering::SeqCst);
    Ret::default()
}

fn publication(case: u32) {
    // Direct calls below exercise the typed release itself. Keep its other typed
    // callees by address so the unrelated batch-end draw can be a no-op double.
    let mut e = Engine::new_untyped_for_test();
    e.mem.map(0x00fd_f000, 0x1000);
    e.mem.map(0x011c_5000, 0x1000);
    e.mem.map(0x0300_0000, 0x2000);
    let imports = [
        (0x00fd_f1d0, "GetCurrentThreadId"),
        (0x00fd_f1bc, "InterlockedCompareExchange"),
        (0x00fd_f104, "Sleep"),
        (CREATE, "CreateThread"),
        (WAIT, "WaitForMultipleObjects"),
    ]
    .map(|(slot, name)| Import {
        slot,
        dll: "KERNEL32.dll".into(),
        name: name.into(),
    });
    assert_eq!(e.register_imports(&imports), imports.len());
    e.register(WORKER, worker);
    e.register(ENTER_POOL, observed_acquire);
    e.register(0x00b9_88e0, |_, _| Ret::default()); // No pass to end, no renderer.
    e.register(0x0043_a080, |_, _| {
        panic!("unexpected refill of a 32-node pool")
    });
    e.start_real_clock();
    real_threads::start(&mut e).unwrap();
    let base = e.mem.alloc(NODE_COUNT * 12);
    let busy = e.mem.alloc(NODE_COUNT * 4);
    let p = e.mem.alloc(128);
    e.mem.set_u32(TEST_ROOT, p);
    e.mem.set_u32(p, base);
    e.mem.set_u32(p + 4, busy);
    e.mem.set_u32(p + 16, case);
    for i in 0..NODE_COUNT {
        e.mem.set_u32(
            base + 12 * i,
            if i + 1 < NODE_COUNT {
                base + 12 * (i + 1)
            } else {
                0
            },
        );
    }
    e.mem.set_u32(POOL_FREE, base);
    let handles = e.mem.alloc(16);
    for i in 0..4 {
        let control = e.mem.alloc(8 + 0x1ac);
        e.mem.set_u32(control, p);
        let h = e
            .call(CREATE, &args![0u32, 0u32, WORKER, control, 0u32, 0u32])
            .u32();
        assert_ne!(h, 0);
        e.mem.set_u32(handles + 4 * i, h);
    }
    assert_eq!(
        e.call(WAIT, &args![4u32, handles, 1u32, 60_000u32]).u32(),
        0
    );
    assert_eq!(e.mem.u32(p + 8), 0,
        "case {case}: 1=invalid node, 2=duplicate owner, 3=uncleared node, 4=payload overwrite, 5=incomplete lock publication");
    assert_eq!(e.mem.u32(p + 12), 4 * CYCLES);
    let acquisitions = if case < 2 { 3 } else { 2 };
    assert_eq!(e.mem.u32(p + 32), 4 * CYCLES * acquisitions);
    let mut nodes = std::collections::HashSet::new();
    let mut cursor = e.mem.u32(POOL_FREE);
    while cursor != 0 {
        assert!(cursor >= base && cursor < base + 12 * NODE_COUNT && (cursor - base) % 12 == 0);
        assert!(
            nodes.insert(cursor),
            "case {case}: duplicate/cyclic free node"
        );
        cursor = e.mem.u32(cursor);
    }
    assert_eq!(nodes.len(), NODE_COUNT as usize);
    for i in 0..NODE_COUNT {
        assert_eq!(e.mem.u32(busy + 4 * i), 0);
    }
    assert_eq!(e.mem.u32(POOL_LOCK), 0);
    assert_eq!(e.mem.u32(POOL_LOCK + 4), 0);
}

#[test]
fn render_list_releases_publish_the_shared_node_pool() {
    // Serialized: each Engine reserves a 4 GB guest address space. These cover
    // the nested outer final decrement and a one-level final decrement in both
    // translated and typed render paths.
    for case in 0..4 {
        publication(case);
    }
}
