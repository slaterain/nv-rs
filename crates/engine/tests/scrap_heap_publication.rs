//! Headless stress of the scrap manager's real buffer-list lock; no game files.
use engine as _;
use engine_core::{args, exe::Import, real_threads, Engine, Ret};
use std::sync::atomic::Ordering;
const CREATE: u32 = 0x00fd_f200;
const WAIT: u32 = 0x00fd_f204;
const WORKER: u32 = 0x0300_0000;
fn worker(e: &mut Engine, a: &[u32]) -> Ret {
    let p = a[0];
    let manager = e.mem.u32(p);
    let busy = e.mem.u32(p + 4);
    e.with_stack(4, |e, size| {
        for _ in 0..100_000 {
            if e.mem.atomic_u32(p + 8).unwrap().load(Ordering::Relaxed) != 0 {
                break;
            }
            e.mem.set_u32(size.addr(), 0x10000);
            let b = e.call(0x00aa_59b0, &args![manager, size]).u32();
            let id = e.mem.u32(b);
            if id >= 8 {
                e.mem.atomic_u32(p + 8).unwrap().store(3, Ordering::Relaxed);
                break;
            }
            if e.mem
                .atomic_u32(busy + 4 * id)
                .unwrap()
                .swap(1, Ordering::SeqCst)
                != 0
            {
                e.mem.atomic_u32(p + 8).unwrap().store(1, Ordering::Relaxed);
                break;
            }
            let first = e.mem.atomic_u32(b + 64).unwrap().load(Ordering::Relaxed);
            for off in (64..128).step_by(4) {
                if e.mem.atomic_u32(b + off).unwrap().load(Ordering::Relaxed) != first {
                    e.mem.atomic_u32(p + 8).unwrap().store(2, Ordering::Relaxed);
                }
                e.mem
                    .atomic_u32(b + off)
                    .unwrap()
                    .store(first + 1, Ordering::Relaxed);
            }
            e.mem
                .atomic_u32(busy + 4 * id)
                .unwrap()
                .store(0, Ordering::Release);
            e.call(0x00aa_5b70, &args![manager, b, 0x10000u32]);
        }
    });
    Ret::default()
}
#[test]
fn scrap_buffers_are_never_handed_to_two_threads() {
    let mut e = Engine::new();
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
    e.mem.map(0x00fd_f000, 0x1000);
    assert_eq!(e.register_imports(&imports), imports.len());
    e.register(WORKER, worker);
    e.start_real_clock();
    real_threads::start(&mut e).unwrap();
    let manager = e.mem.alloc(0x240);
    let p = e.mem.alloc(16);
    let busy = e.mem.alloc(32);
    e.mem.set_u32(p, manager);
    e.mem.set_u32(p + 4, busy);
    for i in 0..8 {
        let b = e.mem.alloc(0x10000);
        e.mem.set_u32(b, i);
        e.mem.set_u32(manager + i * 8, b);
        e.mem.set_u32(manager + i * 8 + 4, 0x10000);
    }
    e.mem.set_u32(manager + 0x200, 8);
    e.mem.set_u32(manager + 0x204, 8 * 0x10000);
    let handles = e.mem.alloc(16);
    for i in 0..4 {
        let h = e
            .call(CREATE, &args![0u32, 0u32, WORKER, p, 0u32, 0u32])
            .u32();
        assert_ne!(h, 0);
        e.mem.set_u32(handles + 4 * i, h);
    }
    assert_eq!(
        e.call(WAIT, &args![4u32, handles, 1u32, 60_000u32]).u32(),
        0
    );
    assert_eq!(
        e.mem.u32(p + 8),
        0,
        "no duplicate or incompletely published buffer"
    );
    assert_eq!(e.mem.u32(manager + 0x200), 8);
    assert_eq!(e.mem.u32(manager + 0x204), 8 * 0x10000);
    let ids: std::collections::HashSet<_> = (0..8)
        .map(|i| e.mem.u32(e.mem.u32(manager + 8 * i)))
        .collect();
    assert_eq!(ids.len(), 8, "all buffers returned exactly once");
}
