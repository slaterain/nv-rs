//! The thread-race row under the stepped host (the #414 reviewer's demonstration, kept as a test):
//! a time slice that ends at the map look-up the row makes under its lock. Before the fix the main
//! thread's first call then waited on its OS thread for that lock while the holder waited for the
//! engine (a hang: this test stops the process with code 86 after 10 s). `control` runs the same
//! steps on the translation alone, first.

use engine::prelude::*;
use engine::units::bscore::memorymanager::memory_manager_get_thread_scrap_heap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::SeqCst};
use std::sync::Arc;

const GET_THREAD_SCRAP_HEAP: u32 = 0x00aa_42e0;
const GET_CURRENT_THREAD_ID: u32 = 0x00fd_f1d0;
const ALLOCATE: u32 = 0x00aa_3e40;
const SCRAP_HEAP_CTOR: u32 = 0x00aa_53f0;
const MAP_INIT: u32 = 0x00aa_47e0;
const MAP_SLOTS: [u32; 6] = [
    0x00aa_4bb0,
    0x00a0_d260,
    0x00ae_8dc0,
    0x00a1_e720,
    0x00aa_4a50,
    0x00aa_8b00,
];
const MAP: u32 = 0x114;
const THREAD_START: u32 = 0x00ab_0000;

static MANAGER: AtomicU32 = AtomicU32::new(0);

fn stand_ins(e: &mut Engine) {
    e.register(GET_CURRENT_THREAD_ID, |e, _| {
        engine::thread_host::current_id(e).into_ret()
    });
    e.register(ALLOCATE, |e, a| e.mem.alloc(a[1]).into_ret());
    e.register(SCRAP_HEAP_CTOR, |_, a| a[0].into_ret());
}

fn manager(e: &mut Engine) -> u32 {
    let manager = e.mem.alloc(0x140);
    e.call(MAP_INIT, &args![manager + MAP, 1u32]);
    let vtable = e.mem.alloc(4 * MAP_SLOTS.len() as u32);
    for (i, slot) in MAP_SLOTS.iter().enumerate() {
        e.mem.set_u32(vtable + 4 * i as u32, *slot);
    }
    e.mem.set_u32(manager + MAP, vtable);
    manager
}

fn nodes(e: &Engine, manager: u32) -> usize {
    let table = e.mem.u32(manager + MAP + 8);
    let mut node = e.mem.u32(table);
    let mut count = 0;
    while node != 0 {
        count += 1;
        node = e.mem.u32(node);
    }
    count
}

fn row(e: &mut Engine, a: &[u32]) -> Ret {
    host::thread_races::thread_scrap_heap(e, a[0]).into_ret()
}

fn translation(e: &mut Engine, a: &[u32]) -> Ret {
    memory_manager_get_thread_scrap_heap(e, Ptr::new(a[0])).into_ret()
}

fn thread_start(e: &mut Engine, _: &[u32]) -> Ret {
    let m = MANAGER.load(SeqCst);
    let heap = e.call(GET_THREAD_SCRAP_HEAP, &args![m]).u32();
    assert_ne!(heap, 0);
    Ret::default()
}

fn stepped(with_row: bool) {
    let mut e = Engine::new();
    stand_ins(&mut e);
    let getter: engine::AbiFn = if with_row { row } else { translation };
    e.register(GET_THREAD_SCRAP_HEAP, getter);
    e.register(THREAD_START, thread_start);
    let m = manager(&mut e);
    MANAGER.store(m, SeqCst);
    // Stepped threads, a slice that ends at whatever call comes next (`NV_THREADS=slice`; the
    // default scheme's hard cap ends a turn the same way), three calls long: the game thread's
    // start, the scrap-heap getter and GetCurrentThreadId; its fourth call is the map look-up.
    e.kernel.threads.scheme = engine::thread_host::Scheme::SLICE;
    let id = engine::thread_host::create(&mut e, 0x500, THREAD_START, 0, false);
    engine::thread_host::start_slicing(&mut e, 3);
    engine::thread_host::run_until(
        &mut e,
        engine::units::platform::signalled,
        &mut |e: &mut Engine| e.kernel.threads.preempted >= 1,
    );
    let state = engine::thread_host::by_handle(&mut e, 0x500).map(|t| t.state.clone());
    eprintln!(
        "game thread {id} gave its turn back at its fourth call: {state:?}; map nodes {}",
        nodes(&e, m)
    );
    let done = Arc::new(AtomicBool::new(false));
    let watch = done.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(10));
        if !watch.load(SeqCst) {
            eprintln!(
                "RESULT stepped (row {with_row}): HUNG: the main thread's first scrap-heap call \
                 has not returned after 10 s (it waits on its OS thread while the other thread \
                 waits for the engine)"
            );
            std::process::exit(86);
        }
    });
    let heap = e.call(GET_THREAD_SCRAP_HEAP, &args![m]).u32();
    done.store(true, SeqCst);
    eprintln!("main's first call returned {heap:08x}");
    engine::thread_host::run_threads(&mut e, engine::units::platform::signalled);
    eprintln!(
        "RESULT stepped (row {with_row}): finished, map nodes {}",
        nodes(&e, m)
    );
}

/// One test: the two runs share the manager static and the process exit on a hang.
#[test]
fn stepped_turn_ends_inside_the_map_lock() {
    stepped(false);
    stepped(true);
}
