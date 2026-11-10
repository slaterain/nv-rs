//! The thread-race row (`host::thread_races`, docs/FIXES.md "Thread races") with real OS threads:
//! the render helpers (`engine::real_threads::fork_join`) and the caller run the same exe
//! function side by side on one shared memory, in rounds that start together.
//!
//! - The thread's scrap heap: every participant makes its first `GetThreadScrapHeap` (`00aa42e0`)
//!   call of the round at once, into a thread scrap-heap map of one bucket (the exe's has 37: one
//!   bucket makes every insert meet, the race is the same). The translation's `SetAt` links each
//!   new node by reading and writing the bucket's head with no lock, so some rounds lose a node;
//!   with the row none does.
//! - The face-gen depth: every participant takes and releases the face-gen lock in a loop
//!   (`00652140`, `00652190`); the original counts down after the release with a plain read and
//!   write, so counts are lost and the depth ends away from 0; with the row it ends at 0.
//!
//! One test: the helper pool is one per process, made from the first engine that forks. The
//! vanilla halves only report how often the race struck (a race need not strike in a given run);
//! the row's halves must never fail.

use engine::prelude::*;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering::SeqCst};

const HELPERS: usize = 3;
const PARTICIPANTS: usize = HELPERS + 1;

const GET_THREAD_SCRAP_HEAP: u32 = 0x00aa_42e0;
const GET_CURRENT_THREAD_ID: u32 = 0x00fd_f1d0;
const ALLOCATE: u32 = 0x00aa_3e40;
const SCRAP_HEAP_CTOR: u32 = 0x00aa_53f0;
const MAP_INIT: u32 = 0x00aa_47e0;
/// The map's virtual functions, as the exe's table (`010a25f8`) holds them: destructor, hash,
/// compare, set value, free value, new item.
const MAP_SLOTS: [u32; 6] = [
    0x00aa_4bb0,
    0x00a0_d260,
    0x00ae_8dc0,
    0x00a1_e720,
    0x00aa_4a50,
    0x00aa_8b00,
];
const MAP: u32 = 0x114;

const FACE_GEN_ENTER: u32 = 0x0065_2140;
const FACE_GEN_LEAVE: u32 = 0x0065_2190;
const FACE_GEN_DEPTH: u32 = 0x011d_59dc;
const LOCK_TAKE: u32 = 0x0045_38e0;
const LOCK_RELEASE: u32 = 0x0082_f1f0;

thread_local! {
    /// The game thread id this OS thread answers `GetCurrentThreadId` with.
    static ID: Cell<u32> = const { Cell::new(0) };
}

/// The face-gen lock's stand-in: a spin lock.
static LOCK: AtomicBool = AtomicBool::new(false);

/// A barrier for the participants of a round, reusable (a generation per pass).
struct Barrier {
    waiting: AtomicUsize,
    generation: AtomicUsize,
}

impl Barrier {
    const fn new() -> Self {
        Barrier {
            waiting: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
        }
    }

    fn wait(&self) {
        let generation = self.generation.load(SeqCst);
        if self.waiting.fetch_add(1, SeqCst) + 1 == PARTICIPANTS {
            self.waiting.store(0, SeqCst);
            self.generation.fetch_add(1, SeqCst);
        } else {
            while self.generation.load(SeqCst) == generation {
                std::hint::spin_loop();
            }
        }
    }
}

static BARRIER: Barrier = Barrier::new();

fn stand_ins(e: &mut Engine) {
    e.register(GET_CURRENT_THREAD_ID, |_, _| ID.with(Cell::get).into_ret());
    e.register(ALLOCATE, |e, a| e.mem.alloc(a[1]).into_ret());
    e.register(SCRAP_HEAP_CTOR, |_, a| a[0].into_ret());
    e.register(LOCK_TAKE, |_, _| {
        while LOCK
            .compare_exchange_weak(false, true, SeqCst, SeqCst)
            .is_err()
        {
            std::hint::spin_loop();
        }
        Ret::default()
    });
    e.register(LOCK_RELEASE, |_, _| {
        LOCK.store(false, SeqCst);
        Ret::default()
    });
}

/// A memory manager with a thread scrap-heap map of one bucket.
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

/// The nodes in the map's one bucket.
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

/// Rounds in which a participant's node went missing from the map.
fn scrap_heap_rounds(e: &mut Engine, manager: u32, rounds: usize) -> usize {
    let lost = AtomicUsize::new(0);
    let job = |w: &mut Engine, number: usize| {
        ID.with(|id| id.set(0x100 + number as u32));
        for _ in 0..rounds {
            BARRIER.wait();
            assert_ne!(w.call(GET_THREAD_SCRAP_HEAP, &args![manager]).u32(), 0);
            BARRIER.wait();
            if number == 0 {
                if nodes(w, manager) != PARTICIPANTS {
                    lost.fetch_add(1, SeqCst);
                }
                // The next round starts with an empty map, and the row's threads look again.
                let table = w.mem.u32(manager + MAP + 8);
                w.mem.set_u32(table, 0);
                w.mem.set_u32(manager + MAP + 0xc, 0);
                host::thread_races::forget_found_heaps();
            }
            BARRIER.wait();
        }
    };
    assert!(engine::real_threads::fork_join(e, HELPERS, &job));
    lost.into_inner()
}

/// The face-gen depth after every participant took and released the lock `turns` times.
fn face_gen_depth(e: &mut Engine, turns: usize) -> u32 {
    e.mem.set_u32(FACE_GEN_DEPTH, 0);
    let job = |w: &mut Engine, _: usize| {
        BARRIER.wait();
        for _ in 0..turns {
            w.call(FACE_GEN_ENTER, &args![]);
            w.call(FACE_GEN_LEAVE, &args![]);
        }
        BARRIER.wait();
    };
    assert!(engine::real_threads::fork_join(e, HELPERS, &job));
    e.mem.u32(FACE_GEN_DEPTH)
}

#[test]
fn threads_meeting_lose_nothing_with_the_row() {
    let mut e = Engine::new();
    if engine::real_threads::MOST_HELPERS < HELPERS
        || std::thread::available_parallelism().map_or(1, |n| n.get()) < 2
    {
        eprintln!("skipped: needs two cores and {HELPERS} helpers");
        return;
    }
    stand_ins(&mut e);
    // The face-gen mapping of the exe's data: tests run without the exe.
    e.mem.map(FACE_GEN_DEPTH & !0xfff, 0x1000);

    const ROUNDS: usize = 20_000;
    const TURNS: usize = 200_000;
    // The original (no row): reported, not required.
    let manager_a = manager(&mut e);
    let lost = scrap_heap_rounds(&mut e, manager_a, ROUNDS);
    let depth = face_gen_depth(&mut e, TURNS);
    eprintln!(
        "vanilla: {lost} of {ROUNDS} rounds lost a scrap heap; the face-gen depth ended at {} \
         after {} takes and releases",
        depth as i32,
        TURNS * PARTICIPANTS
    );

    host::thread_races::install(&mut e);
    let counted = AtomicU32::new(0);
    let manager_b = manager(&mut e);
    assert_eq!(scrap_heap_rounds(&mut e, manager_b, ROUNDS), 0);
    // A thread's second call finds its heap without the map: the same heap.
    ID.with(|id| id.set(0x100));
    let first = e.call(GET_THREAD_SCRAP_HEAP, &args![manager_b]).u32();
    assert_eq!(
        e.call(GET_THREAD_SCRAP_HEAP, &args![manager_b]).u32(),
        first
    );
    counted.store(face_gen_depth(&mut e, TURNS), SeqCst);
    assert_eq!(counted.load(SeqCst), 0, "the face-gen depth with the row");
}
