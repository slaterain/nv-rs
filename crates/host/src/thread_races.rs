//! Thread races (`NV_THREAD_RACES`), a row of the `fixes` preset ([`crate::fixes`],
//! docs/FIXES.md, "Thread races"): three places where FalloutNV.exe 1.4.0.525 shares state
//! between its threads without the lock or the locked instruction it needs. The port runs the
//! game's threads side by side (`NV_THREADS`, real threads in play), so the original's races are
//! the port's too. In the `vanilla` preset nothing here runs. Found by lStewieAl's Tweaks
//! (its `ThreadSafeThreadScrapHeap`, `ConsolePrintMultithreadingFix` and
//! `patchFixFaceGenRaceDataRace`); each is built here from the exe.
//!
//! **The thread's scrap heap** (`MemoryManager::GetThreadScrapHeap`, `00aa42e0`). Each thread's
//! scrap heap is found in a map keyed by the thread's id (the memory manager's +0x114, 37
//! buckets). The first call on a thread allocates a heap and inserts it
//! (`BSMapBase::SetAt`, `00aa4860`): read the bucket's head, link the new node in front, write the
//! head back, count one more; no lock. Two threads inserting into one bucket at once can both read
//! the old head, and one node is lost: that thread's next call makes a second heap, and a block
//! taken from the first is then freed into the second. Nothing removes an entry (only the exit
//! teardown, `00aa3a40`, walks and frees them). Here each OS thread keeps its heap once found
//! (keyed on the run, the game thread's id and a generation the teardown moves on), and the map's
//! look-up and insert run under one lock, so the map is filled as in the original and the
//! teardown still frees every heap. A call after the first is a thread-local read, where the
//! original makes an import call and a map look-up through three virtual calls.
//!
//! **The debug texts and the console** (the critical section at `011f33dc`). `DebugText`'s queue
//! is handled and timed out on the main thread (`DebugText::HandleQueuedPrints`, `00a0e770`;
//! `00a0df80`), and those take the section only while the debug texts are shown (TDT); a
//! `DebugText::Print` (`00a0f8b0`) or a console print (`0071d0a0`, what scripts and the
//! extender's commands print with) takes none, from any thread. Here the four run whole under
//! that section. It is recursive, so their own takes inside stay as they are.
//!
//! **The face-gen depth** (`011d59dc`). `00652140` takes the face-gen lock (`011d5a9c`) and counts
//! the depth up; `00652160` tries it and counts up when it got it; `00652190` releases it and only
//! then counts down, outside the lock, with a plain read and write. A release on one thread and
//! a take on another at the same moment lose one of the two counts. The memory-freeing step
//! (`00866a90`, case 2) empties the face-gen model cache only when the depth is exactly 1 after its
//! own take, so a depth that drifted up keeps the cache full for good (Tweaks' author: "lead to
//! infinite loading for some saves"), and one that drifted down empties it under another thread's
//! feet. Here the counts are locked increments and decrements (atomics), at the same
//! points. (Tweaks also leaves out the lock in three of the model map's functions; the exe gives
//! no reason that this is safe, so that part is not taken.)

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::bscore::memorymanager::fn_00aa3a40;
use engine::units::bsmenu::debugtext::{
    debug_text_handle_queued_prints, debug_text_print, fn_00a0df80,
};
use engine::units::fallout_interface::menuconsole::fn_0071d0a0;
use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

/// `MemoryManager::GetThreadScrapHeap` and the memory manager's teardown (`00aa3a40`).
pub const GET_THREAD_SCRAP_HEAP: u32 = 0x00aa_42e0;
pub const MEMORY_MANAGER_TEARDOWN: u32 = 0x00aa_3a40;
/// `DebugText::HandleQueuedPrints`, the debug texts' time-out (`00a0df80`), `DebugText::Print`
/// (Xbox PDB) and the console print (`0071d0a0`).
pub const HANDLE_QUEUED_PRINTS: u32 = 0x00a0_e770;
pub const DEBUG_TEXT_TIMERS: u32 = 0x00a0_df80;
pub const DEBUG_TEXT_PRINT: u32 = 0x00a0_f8b0;
pub const CONSOLE_PRINT: u32 = 0x0071_d0a0;
/// Take the face-gen lock, try it, release it.
pub const FACE_GEN_ENTER: u32 = 0x0065_2140;
pub const FACE_GEN_TRY: u32 = 0x0065_2160;
pub const FACE_GEN_LEAVE: u32 = 0x0065_2190;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "thread races",
    env: "NV_THREAD_RACES",
    addresses: &[
        GET_THREAD_SCRAP_HEAP,
        MEMORY_MANAGER_TEARDOWN,
        HANDLE_QUEUED_PRINTS,
        DEBUG_TEXT_TIMERS,
        DEBUG_TEXT_PRINT,
        CONSOLE_PRINT,
        FACE_GEN_ENTER,
        FACE_GEN_TRY,
        FACE_GEN_LEAVE,
    ],
    summary:
        "the scrap heap of a new thread, the debug texts and console prints, and the face-gen \
              lock's depth are safe when threads meet",
    install,
};

/// Registers the row's nine functions.
pub fn install(e: &mut Engine) {
    e.register(GET_THREAD_SCRAP_HEAP, thread_scrap_heap_entry);
    e.register(MEMORY_MANAGER_TEARDOWN, teardown_entry);
    e.register(HANDLE_QUEUED_PRINTS, handle_queued_prints_entry);
    e.register(DEBUG_TEXT_TIMERS, debug_text_timers_entry);
    e.register(DEBUG_TEXT_PRINT, debug_text_print_entry);
    e.register(CONSOLE_PRINT, console_print_entry);
    e.register(FACE_GEN_ENTER, |e, _| {
        face_gen_enter(e);
        Ret::default()
    });
    e.register(FACE_GEN_TRY, |e, _| face_gen_try(e).into_ret());
    e.register(FACE_GEN_LEAVE, |e, _| {
        face_gen_leave(e);
        Ret::default()
    });
}

// ---------------------------------------------------------------------------------------------
// The thread's scrap heap
// ---------------------------------------------------------------------------------------------

/// The memory manager's singleton (`011f6238`: `00aa42e0` allocates the heap from it, not from
/// `this`), the import slot of `GetCurrentThreadId`, `Allocate` (`00aa3e40`), the map's look-up
/// (`00a1e380`: key, value out-parameter; true when found) and `SetAt` (`00aa4860`), and
/// `ScrapHeap::ScrapHeap` (`00aa53f0`), as `00aa42e0` calls them.
const MEMORY_MANAGER: u32 = 0x011f_6238;
const GET_CURRENT_THREAD_ID: u32 = 0x00fd_f1d0;
const ALLOCATE: u32 = 0x00aa_3e40;
const MAP_GET_AT: u32 = 0x00a1_e380;
const MAP_SET_AT: u32 = 0x00aa_4860;
const SCRAP_HEAP_CTOR: u32 = 0x00aa_53f0;
/// Where the thread scrap-heap map is in the memory manager (`kThreadScrapHeapMap`).
const THREAD_SCRAP_HEAP_MAP: u32 = 0x114;

/// The map's look-up and insert, one thread at a time (every run of the process: a thread takes
/// it once, at its first call).
static MAP_LOCK: Mutex<()> = Mutex::new(());

/// Moves on at every teardown of a memory manager (and in tests, [`forget_found_heaps`]): a heap
/// found before it is not used again.
static GENERATION: AtomicU32 = AtomicU32::new(0);

/// A heap this OS thread found, and what it was found for.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Found {
    /// The run ([`engine::units::platform::KernelRef::id`]), the game thread's id, the memory
    /// manager and [`GENERATION`].
    run: usize,
    thread: u32,
    manager: u32,
    generation: u32,
}

thread_local! {
    /// The scrap heap this OS thread found last. Each game thread runs on its own OS thread
    /// (stepped and real threads alike), and so does each render helper.
    static FOUND: Cell<Option<(Found, u32)>> = const { Cell::new(None) };
}

fn map_lock() -> MutexGuard<'static, ()> {
    MAP_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn thread_scrap_heap_entry(e: &mut Engine, a: &[u32]) -> Ret {
    thread_scrap_heap(e, a[0]).into_ret()
}

/// `00aa42e0` with the row: the heap this thread found before, or the original's look-up and
/// insert under [`MAP_LOCK`].
pub fn thread_scrap_heap(e: &mut Engine, manager: u32) -> u32 {
    let key = Found {
        run: e.kernel.id(),
        thread: engine::thread_host::current_id(e),
        manager,
        generation: GENERATION.load(Ordering::Acquire),
    };
    if let Some((found, heap)) = FOUND.with(Cell::get) {
        if found == key {
            return heap;
        }
    }
    let heap = look_up_or_insert(e, manager);
    // A null heap (a failed allocation) is looked up again next time, as the original does.
    if heap != 0 {
        FOUND.with(|f| f.set(Some((key, heap))));
    }
    heap
}

/// The original's steps (`00aa42e0`): the thread's id, the look-up into a stack word cleared
/// first, and on a miss a 0x10-byte block from the singleton, the heap built in it (0 when the
/// allocation failed) and stored with `SetAt`; the look-up and the store under [`MAP_LOCK`]. The
/// heap is built outside the lock (its buffer comes from the scrap-heap manager, which takes a
/// lock of its own, a place where another game thread may run), and only this thread inserts its
/// own id, so no other insert can come between.
fn look_up_or_insert(e: &mut Engine, manager: u32) -> u32 {
    let thread = e.call(GET_CURRENT_THREAD_ID, &args![]).u32();
    let map = manager + THREAD_SCRAP_HEAP_MAP;
    e.with_stack(4, |e, heap_at| {
        let heap_at = heap_at.addr();
        e.mem.set_u32(heap_at, 0);
        {
            let _map = map_lock();
            e.call(MAP_GET_AT, &args![map, thread, heap_at]);
        }
        if e.mem.u32(heap_at) == 0 {
            let block = e.call(ALLOCATE, &args![MEMORY_MANAGER, 0x10u32]).u32();
            let heap = if block != 0 {
                e.call(SCRAP_HEAP_CTOR, &args![block]).u32()
            } else {
                0
            };
            e.mem.set_u32(heap_at, heap);
            let _map = map_lock();
            e.call(MAP_SET_AT, &args![map, thread, heap]);
        }
        e.mem.u32(heap_at)
    })
}

/// The heaps found so far are not used again: each thread looks its heap up in the map at its
/// next call.
pub fn forget_found_heaps() {
    GENERATION.fetch_add(1, Ordering::AcqRel);
}

fn teardown_entry(e: &mut Engine, a: &[u32]) -> Ret {
    forget_found_heaps();
    fn_00aa3a40(e, Ptr::new(a[0]));
    Ret::default()
}

// ---------------------------------------------------------------------------------------------
// The debug texts and the console
// ---------------------------------------------------------------------------------------------

/// The debug texts' critical section and the import slots that take and leave it.
const DEBUG_TEXT_SECTION: u32 = 0x011f_33dc;
const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;

/// Runs `f` inside the debug texts' section, and leaves it however `f` ends (a guest exception
/// or a stop unwinds through here).
fn in_debug_text_section<R>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> R) -> R {
    e.call(ENTER_CRITICAL_SECTION, &args![DEBUG_TEXT_SECTION]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(e)));
    e.call(LEAVE_CRITICAL_SECTION, &args![DEBUG_TEXT_SECTION]);
    result.unwrap_or_else(|stop| std::panic::resume_unwind(stop))
}

fn handle_queued_prints_entry(e: &mut Engine, a: &[u32]) -> Ret {
    in_debug_text_section(e, |e| {
        debug_text_handle_queued_prints(e, Ptr::new(a[0]));
    });
    Ret::default()
}

fn debug_text_timers_entry(e: &mut Engine, _: &[u32]) -> Ret {
    in_debug_text_section(e, fn_00a0df80);
    Ret::default()
}

fn debug_text_print_entry(e: &mut Engine, a: &[u32]) -> Ret {
    in_debug_text_section(e, |e| {
        debug_text_print(
            e,
            Ptr::new(a[0]),
            a[1],
            f32::from_bits(a[2]),
            f32::from_bits(a[3]),
            a[4] as i32,
            a[5] as i32,
            f32::from_bits(a[6]),
            a[7] as i32,
            Ptr::new(a[8]),
        )
    })
    .into_ret()
}

fn console_print_entry(e: &mut Engine, a: &[u32]) -> Ret {
    in_debug_text_section(e, |e| {
        fn_0071d0a0(e, Ptr::new(a[0]), a[1], a[2]);
    });
    Ret::default()
}

// ---------------------------------------------------------------------------------------------
// The face-gen depth
// ---------------------------------------------------------------------------------------------

/// The face-gen lock, its depth, its take (`004538e0`, `thiscall` with one word), try
/// (`0044b130`) and release (`0082f1f0`).
const FACE_GEN_LOCK: u32 = 0x011d_5a9c;
const FACE_GEN_DEPTH: u32 = 0x011d_59dc;
const LOCK_TAKE: u32 = 0x0045_38e0;
const LOCK_TRY: u32 = 0x0044_b130;
const LOCK_RELEASE: u32 = 0x0082_f1f0;

/// Counts the depth one up or down with a locked instruction, whatever the threads scheme (the
/// render helpers run beside a stepped game thread too). An unmapped word faults on the plain
/// access, as the original's does.
fn count_depth(e: &mut Engine, up: bool) {
    if let Some(word) = e.mem.atomic_u32(FACE_GEN_DEPTH) {
        if up {
            word.fetch_add(1, Ordering::SeqCst);
        } else {
            word.fetch_sub(1, Ordering::SeqCst);
        }
        return;
    }
    let depth = e.mem.u32(FACE_GEN_DEPTH);
    let depth = if up {
        depth.wrapping_add(1)
    } else {
        depth.wrapping_sub(1)
    };
    e.mem.set_u32(FACE_GEN_DEPTH, depth);
}

/// `00652140` with the depth counted up by a locked increment.
pub fn face_gen_enter(e: &mut Engine) {
    e.call(LOCK_TAKE, &args![FACE_GEN_LOCK, 0u32]);
    count_depth(e, true);
}

/// `00652160` with the depth counted up by a locked increment when the lock was got (the try's
/// answer exactly 1, as the original compares it).
pub fn face_gen_try(e: &mut Engine) -> u8 {
    let taken = e.call(LOCK_TRY, &args![FACE_GEN_LOCK]).u8();
    if taken == 1 {
        count_depth(e, true);
    }
    taken
}

/// `00652190` with the depth counted down by a locked decrement, after the release as in the
/// original.
pub fn face_gen_leave(e: &mut Engine) {
    e.call(LOCK_RELEASE, &args![FACE_GEN_LOCK]);
    count_depth(e, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calls(e: &Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.clone().unwrap()
    }

    /// With the row the four debug-text functions run whole inside the section: its take is the
    /// first call and its leave the last, around the translation's own calls. Without the row
    /// (the translations alone) the print and the console print take no section at all.
    #[test]
    fn debug_text_functions_run_inside_the_section() {
        for fix in [false, true] {
            let mut e = Engine::new();
            if fix {
                install(&mut e);
            }
            e.register(ENTER_CRITICAL_SECTION, |_, _| Ret::default());
            e.register(LEAVE_CRITICAL_SECTION, |_, _| Ret::default());
            // The console print returns at once when the console is up and not shown.
            e.register(0x0070_5ea0, |_, _| 1u32.into_ret());
            e.register(0x0070_edf0, |_, _| 0u32.into_ret());
            e.call_log = Some(vec![]);
            e.call(CONSOLE_PRINT, &args![0x1000u32, 0u32, 0u32]);
            let log = calls(&e);
            let sections = log
                .iter()
                .filter(|(at, _)| *at == ENTER_CRITICAL_SECTION || *at == LEAVE_CRITICAL_SECTION)
                .count();
            if fix {
                assert_eq!(log[1], (ENTER_CRITICAL_SECTION, vec![DEBUG_TEXT_SECTION]));
                assert_eq!(
                    log.last().unwrap(),
                    &(LEAVE_CRITICAL_SECTION, vec![DEBUG_TEXT_SECTION])
                );
                assert_eq!(sections, 2, "{log:x?}");
            } else {
                assert_eq!(sections, 0, "{log:x?}");
            }
        }
    }
}
