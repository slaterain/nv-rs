//! The "Havok reference counts without the global lock" row (`NV_HAVOK_REFS`,
//! `engine_havok::units::havok_sdk::hkreferencedobject`): the game's six reference functions against their lock-free
//! forms. One thread, every sequence of operations: the same counts and the same deletes after each. Real threads: two
//! threads add and remove references on shared objects while a third holds the world's lock (`lockAll`) and changes
//! them as a physics step does; with the row off (the exe's global lock) and on, nothing is lost and nothing is deleted
//! that is still held, and with the row on the last of three concurrent removes deletes each object exactly once.
//!
//! An integration test of its own: it links only what it calls, so it builds where the crate's whole unit-test program
//! does not fit in memory.

use engine_core::hitch::{self, Count};
use engine_core::{args, exe::Import, real_threads, Engine, Ret};
use engine_havok::units::havok_sdk::hkreferencedobject::{lock_free_turn_on, LOCK_FREE_ADDRESSES};
use std::sync::atomic::Ordering;
use std::sync::Mutex;

engine_core::assemble!(engine_havok::units::funcs, engine_gamebryo::units::funcs);

/// The six functions: `addReferenceLockUnchecked`, `removeReferenceLockUnchecked`, `addReference`,
/// `removeReference`, `addReferences`, `removeReferences`; and `lockAll` / `unlockAll`.
const ADD_UNCHECKED: u32 = 0x00c9_03a0;
const REMOVE_UNCHECKED: u32 = 0x00c9_03b0;
const ADD: u32 = 0x00c9_0510;
const REMOVE: u32 = 0x00c9_05b0;
const ADD_MANY: u32 = 0x00c9_0670;
const REMOVE_MANY: u32 = 0x00c9_0740;
const LOCK_ALL: u32 = 0x00c9_0490;
const UNLOCK_ALL: u32 = 0x00c9_04e0;
/// The lock record's constructor (`00c90830`: mode 1, the critical section with a spin count of 4000), the
/// pointer to the record and the TLS index of its owner mark.
const LOCK_RECORD_CTOR: u32 = 0x00c9_0830;
const LOCK_RECORD: u32 = 0x0126_81d4;
const LOCK_TLS_INDEX: u32 = 0x0126_81d8;
/// The import slots the reference functions and the record use, and two of the test's own.
const IMPORTS: [(u32, &str); 9] = [
    (0x00fd_f044, "TlsGetValue"),
    (0x00fd_f048, "TlsAlloc"),
    (0x00fd_f04c, "TlsSetValue"),
    (0x00fd_f05c, "EnterCriticalSection"),
    (0x00fd_f100, "LeaveCriticalSection"),
    (0x00fd_f130, "InitializeCriticalSectionAndSpinCount"),
    (0x00fd_f1d0, "GetCurrentThreadId"),
    (CREATE_THREAD, "CreateThread"),
    (WAIT_ALL, "WaitForMultipleObjects"),
];
const CREATE_THREAD: u32 = 0x00fd_f200;
const WAIT_ALL: u32 = 0x00fd_f204;
/// The test's functions: an object's deleting destructor (slot 0), the two kinds of worker.
const DELETE: u32 = 0x0300_0000;
const REFS_WORKER: u32 = 0x0300_0010;
const STEP_WORKER: u32 = 0x0300_0020;
/// The objects' vtable.
const VTABLE: u32 = 0x0300_1000;

/// The objects: 16 bytes each, the size word 16 (counted), the count, then the deletes the destructor saw.
const OBJECTS: u32 = 8;
const ROUNDS: u32 = 2_000;

/// One test at a time: the hitch counters are the process's.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// An object's deleting destructor: counts the delete in the object's word at `+8` (the object stays).
fn delete(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a[0];
    e.mem
        .atomic_u32(this + 8)
        .expect("aligned")
        .fetch_add(1, Ordering::SeqCst);
    Ret::default()
}

/// An engine with the record, the imports and the test's functions; the row on when `lock_free`.
fn engine(lock_free: bool) -> Engine {
    let mut e = Engine::new();
    e.map(0x0100_0000, 0x0030_0000);
    e.map(0x00fd_f000, 0x1000);
    e.map(VTABLE, 0x1000);
    let imports = IMPORTS.map(|(slot, name)| Import {
        slot,
        dll: "KERNEL32.dll".into(),
        name: name.into(),
    });
    assert_eq!(e.register_imports(&imports), imports.len());
    e.register(DELETE, delete);
    e.register(REFS_WORKER, refs_worker);
    e.register(STEP_WORKER, step_worker);
    e.mem.set_u32(VTABLE, DELETE);
    let index = e.call(0x00fd_f048, &[]).u32();
    e.mem.set_u32(LOCK_TLS_INDEX, index);
    let record = e.mem.alloc(0x30);
    e.mem.set_u16(record + 4, 0x30);
    e.call(LOCK_RECORD_CTOR, &args![record]);
    assert_eq!(e.mem.u32(record + 8), 1, "the lock mode the game has");
    e.mem.set_u32(LOCK_RECORD, record);
    if lock_free {
        lock_free_turn_on(&mut e);
    }
    e
}

/// `n` objects with `count` references each, and the array of their pointers.
fn objects(e: &mut Engine, n: u32, count: u16) -> (Vec<u32>, u32) {
    let array = e.mem.alloc(4 * n);
    let list = (0..n)
        .map(|i| {
            let object = e.mem.alloc(16);
            assert_eq!(object % 4, 0);
            e.mem.set_u32(object, VTABLE);
            e.mem.set_u16(object + 4, 16);
            e.mem.set_u16(object + 6, count);
            e.mem.set_u32(object + 8, 0);
            e.mem.set_u32(array + 4 * i, object);
            object
        })
        .collect();
    (list, array)
}

/// (count, deletes) of each object.
fn state(e: &Engine, objects: &[u32]) -> Vec<(u16, u32)> {
    objects
        .iter()
        .map(|&o| (e.mem.u16(o + 6), e.mem.u32(o + 8)))
        .collect()
}

/// A worker's block: the array, the number of objects, the rounds, and whether it removes its own reference at the end.
fn block(e: &mut Engine, array: u32, n: u32, rounds: u32, last: bool) -> u32 {
    let p = e.mem.alloc(16);
    for (i, word) in (0u32..).zip([array, n, rounds, u32::from(last)]) {
        e.mem.set_u32(p + 4 * i, word);
    }
    p
}

/// Adds and removes references the way main and the game's other threads do: one at a time (`addReference`,
/// `removeReference`) and every eighth round all at once (`addReferences`, `removeReferences`).
fn refs_worker(e: &mut Engine, a: &[u32]) -> Ret {
    let p = a[0];
    let (array, n, rounds, last) = (
        e.mem.u32(p),
        e.mem.u32(p + 4),
        e.mem.u32(p + 8),
        e.mem.u32(p + 12),
    );
    for round in 0..rounds {
        if round % 8 == 7 {
            e.call(ADD_MANY, &args![array, n, 4u32]);
            e.call(REMOVE_MANY, &args![array, n, 4u32]);
            continue;
        }
        for i in 0..n {
            let object = e.mem.u32(array + 4 * i);
            e.call(ADD, &args![object]);
            e.call(REMOVE, &args![object]);
        }
    }
    if last != 0 {
        for i in 0..n {
            let object = e.mem.u32(array + 4 * i);
            e.call(REMOVE, &args![object]);
        }
    }
    Ret::default()
}

/// Changes references as a physics step does: holding the world's lock (`lockAll`), with the unchecked functions.
fn step_worker(e: &mut Engine, a: &[u32]) -> Ret {
    let p = a[0];
    let (array, n, rounds, last) = (
        e.mem.u32(p),
        e.mem.u32(p + 4),
        e.mem.u32(p + 8),
        e.mem.u32(p + 12),
    );
    for _ in 0..rounds {
        e.call(LOCK_ALL, &[]);
        for i in 0..n {
            let object = e.mem.u32(array + 4 * i);
            e.call(ADD_UNCHECKED, &args![object]);
            e.call(REMOVE_UNCHECKED, &args![object]);
        }
        e.call(UNLOCK_ALL, &[]);
    }
    if last != 0 {
        e.call(LOCK_ALL, &[]);
        for i in 0..n {
            let object = e.mem.u32(array + 4 * i);
            e.call(REMOVE_UNCHECKED, &args![object]);
        }
        e.call(UNLOCK_ALL, &[]);
    }
    Ret::default()
}

/// Two reference workers and one stepping worker on real threads, `rounds` each, then main joins them.
fn run_threads(e: &mut Engine, array: u32, rounds: u32, last: bool) {
    let handles = e.mem.alloc(12);
    for (i, worker) in (0u32..).zip([REFS_WORKER, REFS_WORKER, STEP_WORKER]) {
        let p = block(e, array, OBJECTS, rounds, last);
        let h = e
            .call(CREATE_THREAD, &args![0u32, 0u32, worker, p, 0u32, 0u32])
            .u32();
        assert_ne!(h, 0);
        e.mem.set_u32(handles + 4 * i, h);
    }
    assert_eq!(
        e.call(WAIT_ALL, &args![3u32, handles, 1u32, 120_000u32])
            .u32(),
        0,
        "the workers end"
    );
}

/// One thread, every sequence of the six functions on two twin objects, one through the exe's functions and one
/// through the row's: after each operation the two have the same count and the same deletes. Sequences of up to five
/// operations from counts 0, 1, 2 and 0xFFFF (the edges: a delete at 1, 0 going to 0xFFFF, 0xFFFF wrapping to 0),
/// and an object of size 0 that is never counted.
#[test]
fn every_sequence_ends_with_the_exe_s_counts_and_deletes() {
    let _one = serial();
    let mut exe = engine(false);
    let mut free = engine(true);
    const OPS: [u32; 6] = [
        ADD_UNCHECKED,
        REMOVE_UNCHECKED,
        ADD,
        REMOVE,
        ADD_MANY,
        REMOVE_MANY,
    ];
    let mut sequences = 0;
    for start in [0u16, 1, 2, 0xffff] {
        for size in [16u16, 0] {
            for length in 1..=5u32 {
                for code in 0..6u32.pow(length) {
                    let (a, array_a) = objects(&mut exe, 1, start);
                    let (b, array_b) = objects(&mut free, 1, start);
                    exe.mem.set_u16(a[0] + 4, size);
                    free.mem.set_u16(b[0] + 4, size);
                    let mut rest = code;
                    for step in 0..length {
                        let op = OPS[(rest % 6) as usize];
                        rest /= 6;
                        let (wa, wb) = if op == ADD_MANY || op == REMOVE_MANY {
                            (args![array_a, 1u32, 4u32], args![array_b, 1u32, 4u32])
                        } else {
                            (args![a[0]], args![b[0]])
                        };
                        exe.call(op, &wa);
                        free.call(op, &wb);
                        assert_eq!(
                            state(&exe, &a),
                            state(&free, &b),
                            "from {start:#x} (size {size}), step {step} of sequence {code} ({length} long): {op:08x}"
                        );
                    }
                    sequences += 1;
                }
            }
        }
    }
    assert_eq!(sequences, 8 * (6 + 36 + 216 + 1296 + 7776));
}

/// Main's reference changes that take the global lock are counted for the hitch line (the exe's functions); with the
/// row none takes it.
#[test]
fn main_s_locked_reference_changes_are_counted_and_the_row_takes_none() {
    let _one = serial();
    for lock_free in [false, true] {
        let mut e = engine(lock_free);
        let (list, array) = objects(&mut e, 2, 1);
        let before = hitch::snapshot();
        for &object in &list {
            e.call(ADD, &args![object]);
            e.call(REMOVE, &args![object]);
        }
        e.call(ADD_MANY, &args![array, 2u32, 4u32]);
        e.call(REMOVE_MANY, &args![array, 2u32, 4u32]);
        let d = hitch::since(&hitch::snapshot(), &before);
        let locks = d[Count::HavokRefLocks as usize];
        if lock_free {
            assert_eq!(locks, 0, "the row takes no lock");
        } else {
            assert_eq!(locks, 4 + 2, "one for each change, one for each batch");
        }
        assert_eq!(state(&e, &list), vec![(1, 0); 2]);
    }
}

/// Real threads, the references balanced: two threads add and remove references (one at a time and in batches)
/// while a third adds and removes them holding the world's lock, as a step does. Every count ends where it started and
/// nothing is deleted, with the exe's lock and with the row; main's waits for the lock are the exe's only.
#[test]
fn shared_references_on_real_threads_lose_nothing() {
    let _one = serial();
    for lock_free in [false, true] {
        let mut e = engine(lock_free);
        e.start_real_clock();
        real_threads::start(&mut e).unwrap();
        let (list, array) = objects(&mut e, OBJECTS, 1);
        run_threads(&mut e, array, ROUNDS, false);
        assert_eq!(
            state(&e, &list),
            vec![(1, 0); OBJECTS as usize],
            "lock_free={lock_free}"
        );
    }
}

/// Real threads, each holding one of the three references: their last removes run at once. With the row exactly one of
/// them deletes each object, and the count ends at 0. With the exe's functions the count ends at 0 too, and an object
/// is deleted at most once: two removes that both see a count of 2 can both take the locked path, and then none
/// deletes (the exe's own unlocked test of a count of 1, `00c905b0`), which the row does not copy.
#[test]
fn the_last_of_three_concurrent_removes_deletes_once() {
    let _one = serial();
    for lock_free in [false, true] {
        let mut e = engine(lock_free);
        e.start_real_clock();
        real_threads::start(&mut e).unwrap();
        for _ in 0..20 {
            let (list, array) = objects(&mut e, OBJECTS, 3);
            run_threads(&mut e, array, 50, true);
            for (i, (count, deletes)) in state(&e, &list).into_iter().enumerate() {
                assert_eq!(count, 0, "object {i}, lock_free={lock_free}");
                if lock_free {
                    assert_eq!(deletes, 1, "object {i}");
                } else {
                    assert!(deletes <= 1, "object {i}: deleted {deletes} times");
                }
            }
        }
    }
}

/// The row replaces exactly the six functions it names, and the batch functions reach the same per-object work.
#[test]
fn the_row_replaces_the_six_reference_functions() {
    let _one = serial();
    assert_eq!(
        LOCK_FREE_ADDRESSES,
        [
            ADD_UNCHECKED,
            REMOVE_UNCHECKED,
            ADD,
            REMOVE,
            ADD_MANY,
            REMOVE_MANY
        ]
    );
    let mut e = engine(true);
    let (list, array) = objects(&mut e, 3, 1);
    e.call(ADD_MANY, &args![array, 3u32, 4u32]);
    assert_eq!(state(&e, &list), vec![(2, 0); 3]);
    e.call(REMOVE_MANY, &args![array, 3u32, 4u32]);
    e.call(REMOVE_MANY, &args![array, 3u32, 4u32]);
    assert_eq!(state(&e, &list), vec![(0, 1); 3]);
    // lockAll and unlockAll still take the section: the depth word counts the holds.
    let record = e.mem.u32(LOCK_RECORD);
    e.call(LOCK_ALL, &[]);
    e.call(LOCK_ALL, &[]);
    assert_eq!(e.mem.u32(record + 0xc), 2);
    e.call(UNLOCK_ALL, &[]);
    e.call(UNLOCK_ALL, &[]);
    assert_eq!(e.mem.u32(record + 0xc), 0);
}
