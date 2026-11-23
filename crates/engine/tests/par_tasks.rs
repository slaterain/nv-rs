//! The parallel task manager's split task (`00c44d90`) run by several real game threads at once, as `NV_PAR_TASKS`
//! makes the manager do (the exe runs it on main and one thread): every item is run exactly once, and the end state
//! is the one the exe's two runners leave. No game files.
use engine as _;
use engine_core::{args, exe::Import, real_threads, Engine, Ret};
use std::sync::atomic::Ordering::Relaxed;

const INCREMENT: u32 = 0x00fd_f1c0;
const CREATE: u32 = 0x00fd_f200;
const WAIT: u32 = 0x00fd_f204;
/// The game's split task: each runner takes the next item from the work's shared counter.
const SPLIT: u32 = 0x00c4_4d90;
const RUNNER: u32 = 0x0300_0000;
const ITEM: u32 = 0x0300_0100;
const ITEMS: u32 = 3000;

/// The item function (cdecl(data, item)), as the game's (an actor's Havok sync, a node's update): it works on its
/// own item only, writing a value made from the item into the item's slot, and counts its runs of the item.
fn item(e: &mut Engine, a: &[u32]) -> Ret {
    let (data, i) = (a[0], a[1]);
    let mut v = i.wrapping_mul(0x9e37_79b9);
    for _ in 0..200 {
        v = v.rotate_left(5) ^ v.wrapping_mul(31).wrapping_add(i);
    }
    e.mem.atomic_u32(data + 4 * i).unwrap().store(v, Relaxed);
    e.mem
        .atomic_u32(data + 4 * (ITEMS + i))
        .unwrap()
        .fetch_add(1, Relaxed);
    Ret::default()
}

/// A runner thread (as a parallel task thread's `RunLinearTasks`): the split task on its parameter, the work.
fn runner(e: &mut Engine, a: &[u32]) -> Ret {
    e.call(SPLIT, &args![a[0]]);
    Ret::default()
}

/// The split run by main and `threads` game threads at once: the items' slots and run counts after it.
fn run(threads: u32) -> Vec<u32> {
    let mut e = Engine::new();
    let imports = [
        (INCREMENT, "InterlockedIncrement"),
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
    e.register(RUNNER, runner);
    e.register(ITEM, item);
    e.start_real_clock();
    real_threads::start(&mut e).unwrap();
    let data = e.mem.alloc(8 * ITEMS);
    for k in 0..2 * ITEMS {
        e.mem.set_u32(data + 4 * k, 0);
    }
    // The work as `0087cf10` builds it: the function, its data, the item count, the shared counter.
    let work = e.mem.alloc(16);
    for (k, w) in [ITEM, data, ITEMS, 0].into_iter().enumerate() {
        e.mem.set_u32(work + 4 * k as u32, w);
    }
    let handles = e.mem.alloc(4 * threads);
    for k in 0..threads {
        let h = e
            .call(CREATE, &args![0u32, 0u32, RUNNER, work, 0u32, 0u32])
            .u32();
        assert_ne!(h, 0);
        e.mem.set_u32(handles + 4 * k, h);
    }
    // Main runs items too, as `BSParallelTaskManager::RunTask` does after starting the threads.
    e.call(SPLIT, &args![work]);
    assert_eq!(
        e.call(WAIT, &args![threads, handles, 1u32, 60_000u32])
            .u32(),
        0
    );
    (0..2 * ITEMS).map(|k| e.mem.u32(data + 4 * k)).collect()
}

/// Six runners beside main take the same items as the exe's one: each item exactly once, the same values.
#[test]
fn more_runners_leave_the_same_end_state_as_the_exe_s_one() {
    let one = run(1);
    assert!(
        one[ITEMS as usize..].iter().all(|&n| n == 1),
        "each item once with the exe's one thread"
    );
    for _ in 0..3 {
        let many = run(6);
        assert!(
            many[ITEMS as usize..].iter().all(|&n| n == 1),
            "each item once with six threads"
        );
        assert_eq!(many, one, "the same end state");
    }
}
