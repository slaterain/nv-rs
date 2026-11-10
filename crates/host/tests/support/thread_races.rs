//! What the two thread-race tests share (`thread_races.rs` with the row installed,
//! `thread_races_vanilla.rs` without it; one per test binary, since the render helpers' pool is one
//! per process and takes the function table when it is made): the render helpers
//! (`engine::real_threads::fork_join`) and the caller run the same exe functions side by side on one
//! shared memory, in rounds that start together.
//!
//! - The thread's scrap heap: every participant makes its first `GetThreadScrapHeap` (`00aa42e0`)
//!   call of the round at once, into a thread scrap-heap map of one bucket (the exe's has 37: one
//!   bucket makes every insert meet; the race is the same). The original's `SetAt` links each new
//!   node by reading and writing the bucket's head with no lock, so some rounds lose a node.
//! - The face-gen depth: every participant takes and releases the face-gen lock in a loop
//!   (`00652140`, `00652190`); the original counts down after the release with a plain read and
//!   write, so counts are lost and the depth ends away from 0.
//!
//! Bounded: three helpers, and none (the test is skipped) below four cores, rounds stop at a time limit, and a barrier that waits more than
//! [`BARRIER_LIMIT`] stops the test with a message.

use engine::prelude::*;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::time::{Duration, Instant};

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

/// The longest a participant waits at a barrier before the test stops.
pub const BARRIER_LIMIT: Duration = Duration::from_secs(60);

thread_local! {
    /// The game thread id this OS thread answers `GetCurrentThreadId` with.
    static ID: Cell<u32> = const { Cell::new(0) };
}

/// The face-gen lock's stand-in: a spin lock.
static LOCK: AtomicBool = AtomicBool::new(false);

/// A reusable barrier for `n` participants.
struct Barrier {
    n: AtomicUsize,
    waiting: AtomicUsize,
    generation: AtomicUsize,
}

static BARRIER: Barrier = Barrier {
    n: AtomicUsize::new(0),
    waiting: AtomicUsize::new(0),
    generation: AtomicUsize::new(0),
};

impl Barrier {
    fn wait(&self) {
        let generation = self.generation.load(SeqCst);
        if self.waiting.fetch_add(1, SeqCst) + 1 == self.n.load(SeqCst) {
            self.waiting.store(0, SeqCst);
            self.generation.fetch_add(1, SeqCst);
            return;
        }
        let start = Instant::now();
        while self.generation.load(SeqCst) == generation {
            assert!(
                start.elapsed() < BARRIER_LIMIT,
                "a participant waited {BARRIER_LIMIT:?} at the barrier"
            );
            std::thread::yield_now();
        }
    }
}

/// What a run of both races counted.
pub struct Counts {
    pub participants: usize,
    pub rounds: usize,
    pub lost: usize,
    pub turns: usize,
    pub depth: i32,
}

/// The helpers beside the caller: three, or none (the test is skipped) below four cores: limited
/// to two or three processors the four threads had not finished after 90 s (the PR 414 review).
pub fn helpers() -> usize {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    if cores < 4 {
        0
    } else {
        3
    }
}

fn stand_ins(e: &mut Engine) {
    e.register(GET_CURRENT_THREAD_ID, |_, _| ID.with(Cell::get).into_ret());
    e.register(ALLOCATE, |e, a| e.mem.alloc(a[1]).into_ret());
    e.register(SCRAP_HEAP_CTOR, |_, a| a[0].into_ret());
    e.register(LOCK_TAKE, |_, _| {
        while LOCK
            .compare_exchange_weak(false, true, SeqCst, SeqCst)
            .is_err()
        {
            std::thread::yield_now();
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

/// Runs up to `rounds` scrap-heap rounds and `turns` face-gen takes per participant, within
/// `limit` for the rounds. `row` installs the thread-race row first (before the first fork, so
/// the helpers' function table has it). None when the machine has one core.
pub fn run(row: bool, rounds: usize, turns: usize, limit: Duration) -> Option<Counts> {
    let helpers = helpers();
    if helpers == 0 {
        eprintln!("skipped: fewer than four cores");
        return None;
    }
    let participants = helpers + 1;
    BARRIER.n.store(participants, SeqCst);
    let mut e = Engine::new();
    stand_ins(&mut e);
    if row {
        host::thread_races::install(&mut e);
    }
    // The face-gen depth's page of the exe's data: tests run without the exe.
    e.mem.map(FACE_GEN_DEPTH & !0xfff, 0x1000);

    let manager = manager(&mut e);
    let lost = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let go_on = AtomicBool::new(true);
    let start = Instant::now();
    let job = |w: &mut Engine, number: usize| {
        ID.with(|id| id.set(0x100 + number as u32));
        loop {
            if number == 0 {
                let round = done.load(SeqCst);
                go_on.store(round < rounds && start.elapsed() < limit, SeqCst);
            }
            BARRIER.wait();
            if !go_on.load(SeqCst) {
                break;
            }
            assert_ne!(w.call(GET_THREAD_SCRAP_HEAP, &args![manager]).u32(), 0);
            BARRIER.wait();
            if number == 0 {
                if nodes(w, manager) != participants {
                    lost.fetch_add(1, SeqCst);
                }
                // The next round starts with an empty map, and the row's threads look again.
                let table = w.mem.u32(manager + MAP + 8);
                w.mem.set_u32(table, 0);
                w.mem.set_u32(manager + MAP + 0xc, 0);
                host::thread_races::forget_found_heaps();
                done.fetch_add(1, SeqCst);
            }
            BARRIER.wait();
        }
    };
    assert!(engine::real_threads::fork_join(&mut e, helpers, &job));

    e.mem.set_u32(FACE_GEN_DEPTH, 0);
    let job = |w: &mut Engine, _: usize| {
        BARRIER.wait();
        for _ in 0..turns {
            w.call(FACE_GEN_ENTER, &args![]);
            w.call(FACE_GEN_LEAVE, &args![]);
        }
        BARRIER.wait();
    };
    assert!(engine::real_threads::fork_join(&mut e, helpers, &job));
    let counts = Counts {
        participants,
        rounds: done.into_inner(),
        lost: lost.into_inner(),
        turns: turns * participants,
        depth: e.mem.u32(FACE_GEN_DEPTH) as i32,
    };
    eprintln!(
        "{}: {} threads; {} of {} rounds lost a scrap heap; the face-gen depth ended at {} after \
         {} takes and releases",
        if row { "row" } else { "vanilla" },
        counts.participants,
        counts.lost,
        counts.rounds,
        counts.depth,
        counts.turns
    );
    Some(counts)
}
