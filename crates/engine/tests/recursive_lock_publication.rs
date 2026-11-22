//! Publication through the game's recursive lock on real host threads, with invented data only.
use engine as _;
use engine_core::{args, exe::Import, real_threads, Engine, Ret};
use std::sync::atomic::Ordering;

const CREATE: u32 = 0x00fd_f200;
const WAIT: u32 = 0x00fd_f204;
const WORKER: u32 = 0x0300_0000;

fn worker(e: &mut Engine, args: &[u32]) -> Ret {
    let p = args[0];
    let typed = e.mem.u32(p + 8) != 0;
    for _ in 0..100_000 {
        if typed {
            engine_typed::dispatch::refs::lock(e, engine_core::Ptr::new(p), 0);
        } else {
            engine::units::fallout_shared::run_0040fbf0::fn_0040fbf0(
                e,
                engine_core::Ptr::new(p),
                0,
            );
        }
        let value = e.mem.atomic_u32(p + 64).unwrap().load(Ordering::Relaxed);
        for offset in (64..128).step_by(4) {
            if e.mem
                .atomic_u32(p + offset)
                .unwrap()
                .load(Ordering::Relaxed)
                != value
            {
                e.mem
                    .atomic_u32(p + 128)
                    .unwrap()
                    .store(1, Ordering::Relaxed);
            }
        }
        for offset in (64..128).step_by(4) {
            e.mem
                .atomic_u32(p + offset)
                .unwrap()
                .store(value + 1, Ordering::Relaxed);
        }
        if typed {
            engine_typed::dispatch::refs::unlock(e, engine_core::Ptr::new(p));
        } else {
            engine::units::fallout_shared::run_0040fba0::fn_0040fba0(e, engine_core::Ptr::new(p));
        }
    }
    Ret::default()
}

fn publication(typed: bool) {
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
    let p = e.mem.alloc(256);
    e.mem.set_u32(p + 8, u32::from(typed));
    let handles = e.mem.alloc(16);
    for i in 0..4 {
        let h = e
            .call(CREATE, &args![0u32, 0u32, WORKER, p, 0u32, 0u32])
            .u32();
        assert_ne!(h, 0);
        e.mem.set_u32(handles + i * 4, h);
    }
    assert_eq!(
        e.call(WAIT, &args![4u32, handles, 1u32, 60_000u32]).u32(),
        0
    );
    assert_eq!(
        e.mem.u32(p + 128),
        0,
        "all protected words published together; typed={typed}"
    );
    assert_eq!(e.mem.u32(p + 64), 400_000);
}

#[test]
fn translated_unlock_publishes_protected_writes() {
    publication(false);
}
#[test]
fn typed_unlock_publishes_protected_writes() {
    publication(true);
}
