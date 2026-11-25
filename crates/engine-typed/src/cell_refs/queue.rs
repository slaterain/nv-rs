//! `ModelLoader::QueueReference` (`00444850`): the model loader's queue of a reference's 3D, which
//! [`super::references_after_load`] asks for every reference it brings up (87% of that pass's time
//! in a profiled load window), and the game's other callers (a reference's `Load3D`, the actor and
//! script code) by address.
//!
//! **What the queue is.** The loader (`011c3b3c`) keeps its tasks in two lock-free maps keyed by
//! the reference: the queued references (`+8`) and the references to queue later (`+0xc`). Both are
//! the lock-free family's counted maps: each operation takes the map's own lock (`+0x20`,
//! `0040fbf0`, a spin that yields to the other game threads; one thread may take it again) for that
//! one operation, and works through the calling thread's own interface of the map. The find and the
//! add are two separate operations, as in the exe: another thread may add the same reference
//! between them, and then the add refuses this task (its flag 0 does not replace) and the task
//! pointer lets it go.
//!
//! **Counts and memory.** A task is a counted object (its count at `+8`): the task pointer on the
//! stack counts it when it takes it (`0092c870`, an interlocked increment), the map's node counts
//! it again. When the map refuses the task, the pointer's assignment of null (`006f74f0`, whose
//! release `0044dd60` is an interlocked decrement) drops the last count and deletes the task
//! through its slot 0, inside the context `0x31`. The pointer's release at the end (`0044cbf0`)
//! lets go of whatever it still holds, which deletes only if every other count went meanwhile (a
//! loader thread finished the task and the map let it go between the find or the add and that
//! release). The task is allocated by the game's memory manager (`00401000`), inside the memory
//! context `0x31` (and `0x34` for the player, `0x32` for an NPC or a creature, nested inside it),
//! entered and left by address (`00404eb0`, `00404ee0`) since they set the context the allocator
//! files the block under.
//!
//! **Which thread.** Everything here runs on the calling thread, normally main. With `later` set
//! and the calling thread the loader's owner thread (the word at +0x10 of `011dea0c`) outside menu
//! mode, the task goes into the second map instead of being started; otherwise it is started at
//! once (its slot 0x20, `QueueMe`, which hands its files to the background loaders). The order of
//! work is the exe's: the find, the build, the add, the start, then the cell's counts.
//!
//! **Native.** The game loader's two flags (`0042ce10`, `00444d20`), the base form's type
//! (`00401170`), the reference's flag word (`00444d00`), the task's priority (`0043cc60`: byte 2 of
//! its key at +0x10), the task pointer's set-up with no task (`00528cb0`) and the task it holds
//! (`00559450`) are done in place. The task pointer's assignment (`006f74f0`,
//! `NiPointer<QueuedFile>`'s `operator=`) and release (`0044cbf0`), the count taken (`0092c870`),
//! the cell's two counters (`00444cc0`, `00444ce0`) and `TESObjectREFR::Is3DCritical` (`00572350`)
//! are typed here and registered at their addresses (the first three are the counted-pointer bodies
//! every caller of those addresses shares). The maps' find (`0044bd60`) and add (`00528b60`) are
//! the lock-free family's typed calls when the map's slot holds them; the current thread
//! (`0040fc90`), the interlocked increments (`0040b460`), the owner's thread word (`0044edb0`) and
//! the reference's cell (`008d6f30`) are their families' typed calls. Called by address: the memory
//! manager and its contexts, the task constructors, the task's slots, the release that may delete,
//! menu mode (`00702360`).

use super::{base_form, is_actor, kind_test, FAMILY};
use crate::{ai_queries, dispatch, facegen, lockfree, slot_target};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The game loader (a global) and its word of flags: `0042ce10` tests bit 1, `00444d20` bit 3.
const GAME_LOADER_POINTER: u32 = 0x011d_df38;
const GAME_LOADER_FLAGS: u32 = 0x244;
const LOADER_FLAG_2: u32 = 2;
const LOADER_FLAG_8: u32 = 8;
/// The base form type whose references are never queued.
const FORM_TYPE_NOT_QUEUED: u8 = 0xd;
/// The reference's word that `00444d00` tests, and the bit it tests here.
const REFERENCE_FLAGS_C8: u32 = 0xc8;
const REFERENCE_FLAG_2: u32 = 2;
/// The loader's two maps: the queued references and the references to queue later.
const QUEUED_MAP: u32 = 8;
const LATER_MAP: u32 = 0xc;
/// The maps' slots: find (`thiscall(key, holder out)`) and add (`thiscall(key, holder, replace)`),
/// and the lock-free family's functions they hold for the loader's maps.
const SLOT_FIND: u32 = 0x8;
const SLOT_ADD: u32 = 0x10;
const MAP_FIND: u32 = 0x0044_bd60;
const MAP_ADD: u32 = 0x0052_8b60;
/// A task's count, its key (the priority is its byte 2) and its slots: the new priority (0x1c) and
/// `QueueMe` (0x20).
const TASK_COUNT: u32 = 8;
const TASK_KEY: u32 = 0x10;
const SLOT_SET_PRIORITY: u32 = 0x1c;
const SLOT_QUEUE_ME: u32 = 0x20;
/// A counted object's release (`thiscall`): the count's interlocked decrement, and at zero the
/// object's slot 0 with 1.
const RELEASE: u32 = 0x0044_dd60;
/// The memory manager's allocation (`cdecl(size)`) and its context scope (`thiscall(scope, context,
/// 1, source file, line)` .. `thiscall(scope)`), named by the model loader's source file and a
/// line.
const OPERATOR_NEW: u32 = 0x0040_1000;
const SCOPE_BEGIN: u32 = 0x0040_4eb0;
const SCOPE_END: u32 = 0x0040_4ee0;
const SCOPE_MODE: u32 = 1;
const MODEL_LOADER_SOURCE: u32 = 0x0101_6840;
const CONTEXT_QUEUE: u32 = 0x31;
const LINE_QUEUE: u32 = 0xc4e;
const CONTEXT_PLAYER: u32 = 0x34;
const LINE_PLAYER: u32 = 0xc53;
const CONTEXT_ACTOR: u32 = 0x32;
const LINE_NPC: u32 = 0xc6b;
const LINE_CREATURE: u32 = 0xc72;
/// The player (a global) and the task constructors by kind (`thiscall(block, reference, priority)`;
/// the player's `thiscall(block, priority)`), with their blocks' sizes: `QueuedPlayer`,
/// `QueuedTree`, `QueuedCharacter` for an NPC (base form type 0x2a, `TESNPC`), `QueuedCreature` for
/// a creature (0x2b, `TESCreature`) and `QueuedReference` (the vtables they install: `01016d7c`,
/// `01016c5c`, `01016cec`, `01016d34`, `01016ba4`).
const PLAYER_POINTER: u32 = 0x011d_ea3c;
const TASK_PLAYER: u32 = 0x0044_1b20;
const TASK_TREE: u32 = 0x0044_0e50;
const TASK_CHARACTER: u32 = 0x0044_1440;
const TASK_CREATURE: u32 = 0x0044_1920;
const TASK_DEFAULT: u32 = 0x0043_fd40;
const FORM_TYPE_TREE: u8 = 0x25;
const FORM_TYPE_NPC: u8 = 0x2a;
const FORM_TYPE_CREATURE: u8 = 0x2b;
const SIZE_SMALL: u32 = 0x40;
const SIZE_LARGE: u32 = 0x48;
/// The loader's owner thread object (a global) and menu mode (`Interface::IsInMenuMode`).
const OWNER_THREAD_OBJECT: u32 = 0x011d_ea0c;
const IN_MENU_MODE: u32 = 0x0070_2360;
/// The cell's counters: queued references (`iQueuedRefCount`, Xbox PDB) and critical ones.
const CELL_QUEUED_COUNT: u32 = 0xa4;
const CELL_CRITICAL_COUNT: u32 = 0xa0;

/// The game loader's word of flags, read where the exe reads it (once per test).
#[inline]
fn game_loader_flags(e: &mut Engine) -> u32 {
    let loader = e.global::<u32>(GAME_LOADER_POINTER);
    e.mem.u32(loader.wrapping_add(GAME_LOADER_FLAGS))
}

/// The map's find (slot 8): the lock-free family's [`lockfree::map::find`] when the slot holds it.
#[inline]
fn map_find(e: &mut Engine, map: u32, key: u32, holder: u32) -> bool {
    let found = if slot_target(e, map, SLOT_FIND) == MAP_FIND {
        typed_call!(
            lockfree::FAMILY,
            0x0044_bd60,
            lockfree::map::find(e, Ptr::new(map), key, Ptr::new(holder))
        )
    } else {
        e.vcall(map, SLOT_FIND, &args![key, holder]).u8()
    };
    found != 0
}

/// The map's add (slot 0x10): the lock-free family's [`lockfree::map::add`] when the slot holds it.
#[inline]
fn map_add(e: &mut Engine, map: u32, key: u32, holder: u32, replace: u8) -> bool {
    let added = if slot_target(e, map, SLOT_ADD) == MAP_ADD {
        typed_call!(
            lockfree::FAMILY,
            0x0052_8b60,
            lockfree::map::add(e, Ptr::new(map), key, holder, replace)
        )
    } else {
        e.vcall(map, SLOT_ADD, &args![key, holder, u32::from(replace)])
            .u8()
    };
    added != 0
}

/// An interlocked increment of the word at `at` (`0040b460`).
#[inline]
fn count(e: &mut Engine, at: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_b460,
        dispatch::refs::increment(e, Ptr::new(at))
    );
}

/// The task pointer at `holder` given `object` ([`task_pointer_assign`]).
#[inline]
fn assign(e: &mut Engine, holder: u32, object: u32) {
    typed_call!(FAMILY, 0x006f_74f0, task_pointer_assign(e, holder, object));
}

// Typed from 006f74f0
/// `NiPointer<QueuedFile>::operator=` from an object (Xbox PDB, `thiscall(holder, object)`): when
/// the holder holds another object, the held one is released (`0044dd60`), `object` stored and,
/// read back not null, counted (`0092c870`). Answers `this`.
pub fn task_pointer_assign(e: &mut Engine, this: u32, object: u32) -> u32 {
    let held = e.mem.u32(this);
    if held != object {
        if held != 0 {
            e.call(RELEASE, &args![held]);
        }
        e.mem.set_u32(this, object);
        let taken = e.mem.u32(this);
        if taken != 0 {
            typed_call!(FAMILY, 0x0092_c870, add_ref(e, taken));
        }
    }
    this
}

// Typed from 0092c870
/// A counted object's count taken (`thiscall`): the interlocked increment of its count at +8.
pub fn add_ref(e: &mut Engine, this: u32) {
    count(e, this.wrapping_add(TASK_COUNT));
}

// Typed from 0044cbf0
/// A counted pointer's release (`thiscall(holder)`): the held object, if any, released (`0044dd60`,
/// which deletes it through its slot 0 at the last count).
pub fn task_pointer_release(e: &mut Engine, this: u32) {
    let held = e.mem.u32(this);
    if held != 0 {
        e.call(RELEASE, &args![held]);
    }
}

// Typed from 00444cc0
/// The cell's count of queued references (`iQueuedRefCount`, Xbox PDB, +0xa4) one more
/// (`thiscall(cell)`, an interlocked increment).
pub fn count_queued(e: &mut Engine, this: u32) {
    count(e, this.wrapping_add(CELL_QUEUED_COUNT));
}

// Typed from 00444ce0
/// The cell's count of critical queued references (`iCriticalQueuedRefCount`, Xbox PDB, +0xa0) one
/// more (`thiscall(cell)`, an interlocked increment).
pub fn count_critical(e: &mut Engine, this: u32) {
    count(e, this.wrapping_add(CELL_CRITICAL_COUNT));
}

// Typed from 00572350
/// `TESObjectREFR::Is3DCritical` (Xbox PDB, `thiscall(reference)`): whether the reference's slot
/// 0x100 (whether it is an actor) says no.
pub fn is_3d_critical(e: &mut Engine, this: u32) -> bool {
    !is_actor(e, this)
}

/// `body` inside the memory context `context` (named by the model loader's source file and `line`),
/// its scope a 4-byte local.
#[inline]
pub(super) fn in_context(e: &mut Engine, context: u32, line: u32, body: impl FnOnce(&mut Engine)) {
    e.with_stack(4, |e, scope| {
        e.call(
            SCOPE_BEGIN,
            &args![scope, context, SCOPE_MODE, MODEL_LOADER_SOURCE, line],
        );
        body(e);
        e.call(SCOPE_END, &args![scope]);
    });
}

/// A task of `size` bytes from the memory manager, built by `make` (the constructor by address,
/// with the block first) and given to the task pointer; a block of 0 gives 0.
fn build(e: &mut Engine, holder: u32, size: u32, make: impl FnOnce(&mut Engine, u32) -> u32) {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    let object = if block == 0 { 0 } else { make(e, block) };
    assign(e, holder, object);
}

/// The reference's task, by its kind, into the task pointer at `holder`.
fn build_task(e: &mut Engine, holder: u32, reference: u32, priority: u32) {
    if reference == e.global::<u32>(PLAYER_POINTER) {
        in_context(e, CONTEXT_PLAYER, LINE_PLAYER, |e| {
            build(e, holder, SIZE_LARGE, |e, block| {
                e.call(TASK_PLAYER, &args![block, priority]).u32()
            });
        });
        return;
    }
    let base = base_form(e, reference);
    match e.mem.u8(base.wrapping_add(4)) {
        FORM_TYPE_TREE => build(e, holder, SIZE_SMALL, |e, block| {
            e.call(TASK_TREE, &args![block, reference, priority]).u32()
        }),
        FORM_TYPE_NPC => in_context(e, CONTEXT_ACTOR, LINE_NPC, |e| {
            build(e, holder, SIZE_LARGE, |e, block| {
                e.call(TASK_CHARACTER, &args![block, reference, priority])
                    .u32()
            });
        }),
        FORM_TYPE_CREATURE => in_context(e, CONTEXT_ACTOR, LINE_CREATURE, |e| {
            build(e, holder, SIZE_SMALL, |e, block| {
                e.call(TASK_CREATURE, &args![block, reference, priority])
                    .u32()
            });
        }),
        _ => build(e, holder, SIZE_SMALL, |e, block| {
            e.call(TASK_DEFAULT, &args![block, reference, priority])
                .u32()
        }),
    }
}

/// The reference's cell (`008d6f30`, the word at +0x40).
#[inline]
pub(super) fn parent_cell(e: &mut Engine, reference: u32) -> u32 {
    typed_call!(
        ai_queries::FAMILY,
        0x008d_6f30,
        ai_queries::queries::word_at_40(e, reference)
    )
}

// Typed from 00444850
/// `ModelLoader::QueueReference` (Xbox PDB), `thiscall(loader, reference, priority, later)`, `ret
/// 0xc`. Nothing happens when the game loader's flag 2 is set and its flag 8 clear, when the base
/// form has type `0xd`, or when the reference's slot 0x224 says yes and its word at +0xc8 has the
/// bit 2. A task already in the queued map only gets the new priority (its slot 0x1c) when its own
/// differs. Otherwise, inside the memory context `0x31`, the task is built by the reference's kind
/// (the player in the context `0x34`; base form type `0x25`; `0x2a` and `0x2b` in the context
/// `0x32`; any other) and added to the queued map (flag 0); refused, the task pointer is cleared.
/// Added, it goes to the map of later references (flag 1) when `later` is set, the thread is the
/// loader's owner and the game is not in menu mode, and is started (its slot 0x20) otherwise; then
/// the reference's cell, if any, counts it, and counts it as critical too when the reference is not
/// an actor by its slot 0x100.
pub fn queue_reference(e: &mut Engine, this: u32, reference: u32, priority: u32, later: u8) {
    if game_loader_flags(e) & LOADER_FLAG_2 != 0 && game_loader_flags(e) & LOADER_FLAG_8 == 0 {
        return;
    }
    let base = base_form(e, reference);
    if e.mem.u8(base.wrapping_add(4)) == FORM_TYPE_NOT_QUEUED {
        return;
    }
    if kind_test(e, reference)
        && e.mem.u32(reference.wrapping_add(REFERENCE_FLAGS_C8)) & REFERENCE_FLAG_2 != 0
    {
        return;
    }
    e.with_stack(4, |e, holder| {
        let holder = holder.addr();
        e.mem.set_u32(holder, 0);
        let queued = e.mem.u32(this.wrapping_add(QUEUED_MAP));
        if map_find(e, queued, reference, holder) {
            let task = e.mem.u32(holder);
            let own = (e.mem.u32(task.wrapping_add(TASK_KEY)) >> 16) & 0xff;
            if own != priority {
                let task = e.mem.u32(holder);
                e.vcall(task, SLOT_SET_PRIORITY, &args![priority]);
            }
        } else {
            in_context(e, CONTEXT_QUEUE, LINE_QUEUE, |e| {
                build_task(e, holder, reference, priority);
                let queued = e.mem.u32(this.wrapping_add(QUEUED_MAP));
                if !map_add(e, queued, reference, holder, 0) {
                    assign(e, holder, 0);
                    return;
                }
                let mut on_owner_thread = false;
                if later != 0 {
                    let thread = typed_call!(
                        dispatch::FAMILY,
                        0x0040_fc90,
                        dispatch::refs::current_thread(e)
                    );
                    let owner_object = e.global::<u32>(OWNER_THREAD_OBJECT);
                    let owner = typed_call!(
                        facegen::FAMILY,
                        0x0044_edb0,
                        facegen::keyframe::word_at_10(e, Ptr::new(owner_object))
                    );
                    if thread == owner && !e.call(IN_MENU_MODE, &args![]).bool() {
                        on_owner_thread = true;
                    }
                }
                if on_owner_thread {
                    let later_map = e.mem.u32(this.wrapping_add(LATER_MAP));
                    map_add(e, later_map, reference, holder, 1);
                } else {
                    let task = e.mem.u32(holder);
                    e.vcall(task, SLOT_QUEUE_ME, &args![]);
                }
                if parent_cell(e, reference) != 0 {
                    let cell = parent_cell(e, reference);
                    typed_call!(FAMILY, 0x0044_4cc0, count_queued(e, cell));
                    if typed_call!(FAMILY, 0x0057_2350, is_3d_critical(e, reference)) {
                        let cell = parent_cell(e, reference);
                        typed_call!(FAMILY, 0x0044_4ce0, count_critical(e, cell));
                    }
                }
            });
        }
        typed_call!(FAMILY, 0x0044_cbf0, task_pointer_release(e, holder));
    });
}
