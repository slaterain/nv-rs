//! The model loader's finish of a reference's task on main: `QueuedReference::PostProcess`
//! (`0043fdf0`, slot 0x14 of each of the six reference task vtables), which the loader's queue of
//! finished tasks runs on main once the task's files are in, and the attaches it reaches through the
//! task's slot 0x3c: `QueuedReference::Attach` (`00440ba0`, the plain reference's, the tree's, the
//! actor's and the creature's), `QueuedCharacter`'s (`00441800`, the NPC's) and `QueuedPlayer`'s
//! (`00441c30`), both of which come down to `00440ba0`. In a profiled load window `00440ba0` took
//! 1.62 ms a frame with what it calls; most of it is `TES`'s load of the reference into the scene
//! (`00451ef0`), typed in [`super::scene`] and called here as its typed call.
//!
//! **Which attach runs.** With JIP LN NVSE built in and answering "installed" (the default preset),
//! JIP writes its own attach into the slot 0x3c of four of the six vtables (`QueuedReference`,
//! `QueuedCharacter`, `QueuedCreature`, `QueuedPlayer`); the tree's and the actor's keep the exe's.
//! So the post-process calls its typed attach only when the slot holds the exe's function, and
//! JIP's through the slot. JIP's attach calls the cell's two counters (`00440a10`, `00440a50`) and
//! the queued map's removal (`009062e0`, the lock-free family's) by address, so it reaches their
//! typed bodies too.
//!
//! **Native.** The order of work and every call that does work stay the exe's. Done in place: the
//! task's state test (`00449130`, state 6 is the one `QueuedReference::Cancel` sets), the form flag
//! tests (`00440da0`, `00440d80`), the model's root node (`0043b230`: the counted pointer at +0xc,
//! read through `00559450` as the exe does) and the cell's state (`00450fd0`, the byte at +0x26).
//! Typed here and registered at their addresses: the post-process and the three attaches,
//! `TES::IsCellLoaded` (`004511e0`), the loader's word that a reference is done (`004409e0`: the
//! queued map's slot 0x14, the lock-free family's removal when the slot holds it) and the cell's
//! two counters going down (`00440a10`, `00440a50`). Called by address: the memory context's scope,
//! the cell's lock (`00541ac0`, `00541ae0`) and the model loader's addon nodes (`004489b0`). The
//! load into the scene (`00451ef0`) is this family's. The exe reads the task's reference again for
//! every call that takes it, and so does this.
//!
//! **Which thread.** Main: the loader's queue of finished tasks runs the post-process there.

use super::queue::{in_context, is_3d_critical, parent_cell};
use super::{scene, CELL_LOCK_ENTER, CELL_LOCK_LEAVE, FAMILY, MODEL_LOADER_POINTER, TES_POINTER};
use crate::{dispatch, lockfree, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The task's slot `Attach` and the exe's three functions found there: `QueuedReference`'s (in the
/// plain reference's, the tree's, the actor's and the creature's vtables), `QueuedCharacter`'s and
/// `QueuedPlayer`'s.
const SLOT_ATTACH: u32 = 0x3c;
const ATTACH: u32 = 0x0044_0ba0;
const CHARACTER_ATTACH: u32 = 0x0044_1800;
const PLAYER_ATTACH: u32 = 0x0044_1c30;
/// The task's state (`QueuedFile`), the state whose task is left alone, its memory context, its
/// reference and its model (`spModel`, a counted pointer).
const TASK_STATE: u32 = 0xc;
const STATE_CANCELLED: u32 = 6;
const TASK_CONTEXT: u32 = 0x18;
const TASK_REFERENCE: u32 = 0x28;
const TASK_MODEL: u32 = 0x30;
/// The line of the model loader's source file the attach's memory context names.
const LINE_ATTACH: u32 = 0x804;
/// A form's flags (+8): the two that keep a reference from being loaded here.
const FORM_FLAGS: u32 = 8;
const FORM_FLAG_800: u32 = 0x800;
const FORM_FLAG_20: u32 = 0x20;
/// A model's root node (a counted pointer at +0xc).
const MODEL_NODE: u32 = 0xc;
/// `ModelLoader::LoadAddonNodes` (`(loader, reference, node)`).
const LOAD_ADDON_NODES: u32 = 0x0044_89b0;
/// The loader's map of queued references (+8), its slot 0x14 and the lock-free family's removal it
/// holds for the loader's map.
const QUEUED_MAP: u32 = 8;
const SLOT_REMOVE: u32 = 0x14;
const MAP_REMOVE: u32 = 0x0090_62e0;
/// The cell's load state (`cCellState`, the byte `00450fd0` answers) and its two counters.
const CELL_STATE: u32 = 0x26;
const CELL_QUEUED_COUNT: u32 = 0xa4;
const CELL_CRITICAL_COUNT: u32 = 0xa0;

// Typed from 0043fdf0
/// `QueuedReference::PostProcess` (Xbox PDB, `thiscall(task)`): the task's slot 0x3c (`Attach`). The
/// exe's three attaches are their typed calls when the slot holds them; anything else (JIP's
/// attach) through the slot.
pub fn post_process(e: &mut Engine, this: u32) {
    match slot_target(e, this, SLOT_ATTACH) {
        ATTACH => typed_call!(FAMILY, 0x0044_0ba0, attach(e, this)),
        CHARACTER_ATTACH => typed_call!(FAMILY, 0x0044_1800, character_attach(e, this)),
        PLAYER_ATTACH => typed_call!(FAMILY, 0x0044_1c30, player_attach(e, this)),
        _ => {
            e.vcall(this, SLOT_ATTACH, &args![]);
        }
    }
}

// Typed from 00441c30
/// `QueuedPlayer`'s attach (slot 0x3c, `thiscall(task)`): `QueuedCharacter`'s ([`character_attach`]).
pub fn player_attach(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0044_1800, character_attach(e, this));
}

// Typed from 00441800
/// `QueuedCharacter`'s attach (slot 0x3c, `thiscall(task)`): `QueuedReference::Attach`
/// ([`attach`]).
pub fn character_attach(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0044_0ba0, attach(e, this));
}

/// The task's reference (`pRef`), read where the exe reads it.
#[inline]
fn task_reference(e: &Engine, task: u32) -> u32 {
    e.mem.u32(task.wrapping_add(TASK_REFERENCE))
}

/// The form flags of the task's reference.
#[inline]
fn form_flags(e: &Engine, task: u32) -> u32 {
    e.mem.u32(task_reference(e, task).wrapping_add(FORM_FLAGS))
}

/// The object a counted pointer at `at` holds (`00559450`).
#[inline]
fn held(e: &mut Engine, at: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, at))
}

// Typed from 00440ba0
/// `QueuedReference::Attach` (Xbox PDB, `thiscall(task)`), inside the task's memory context (the
/// model loader's line `0x804`): nothing for a task in state 6. Otherwise the reference's parent
/// cell, if any, takes its lock. A reference without form flag `0x800` or `0x20` whose cell
/// `TES::IsCellLoaded` (flag 0) says is loaded gets its addon nodes (`ModelLoader::LoadAddonNodes`
/// with the model's root node, when the task has a model) and is loaded into the scene by `TES`
/// (`00451ef0` with the task as the source). Then, either way, the loader's queued map lets the
/// reference go ([`reference_done`]), and the parent cell, if any, counts one queued reference
/// fewer (and one critical fewer when the reference's 3D is critical) and lets its lock go.
pub fn attach(e: &mut Engine, this: u32) {
    let context = e.mem.u32(this.wrapping_add(TASK_CONTEXT));
    in_context(e, context, LINE_ATTACH, |e| {
        if e.mem.u32(this.wrapping_add(TASK_STATE)) == STATE_CANCELLED {
            return;
        }
        let skip =
            form_flags(e, this) & FORM_FLAG_800 != 0 || form_flags(e, this) & FORM_FLAG_20 != 0;
        if parent_cell(e, task_reference(e, this)) != 0 {
            let cell = parent_cell(e, task_reference(e, this));
            e.call(CELL_LOCK_ENTER, &args![cell]);
        }
        if !skip {
            let cell = parent_cell(e, task_reference(e, this));
            let tes = e.global::<u32>(TES_POINTER);
            if typed_call!(FAMILY, 0x0045_11e0, is_cell_loaded(e, tes, cell, 0)) {
                let model_slot = this.wrapping_add(TASK_MODEL);
                if held(e, model_slot) != 0 {
                    let model = held(e, model_slot);
                    let node = held(e, model.wrapping_add(MODEL_NODE));
                    let reference = task_reference(e, this);
                    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
                    e.call(LOAD_ADDON_NODES, &args![loader, reference, node]);
                }
                let cell = parent_cell(e, task_reference(e, this));
                let reference = task_reference(e, this);
                let tes = e.global::<u32>(TES_POINTER);
                typed_call!(
                    FAMILY,
                    0x0045_1ef0,
                    scene::load_reference(e, tes, reference, cell, this, 0)
                );
            }
        }
        let reference = task_reference(e, this);
        let loader = e.global::<u32>(MODEL_LOADER_POINTER);
        typed_call!(FAMILY, 0x0044_09e0, reference_done(e, loader, reference));
        if parent_cell(e, task_reference(e, this)) != 0 {
            let cell = parent_cell(e, task_reference(e, this));
            typed_call!(FAMILY, 0x0044_0a10, queued_count_down(e, cell));
            let reference = task_reference(e, this);
            if typed_call!(FAMILY, 0x0057_2350, is_3d_critical(e, reference)) {
                let cell = parent_cell(e, task_reference(e, this));
                typed_call!(FAMILY, 0x0044_0a50, critical_count_down(e, cell));
            }
            let cell = parent_cell(e, task_reference(e, this));
            e.call(CELL_LOCK_LEAVE, &args![cell]);
        }
    });
}

// Typed from 004511e0
/// `TES::IsCellLoaded` (Xbox PDB, `thiscall(tes, cell, flag)`, `ret 8`; `this` is not read): false
/// for no cell; by the cell's load state, 2 to 4 answer whether `flag` is 0, 5 and 6 answer true,
/// any other false.
pub fn is_cell_loaded(e: &mut Engine, _this: u32, cell: u32, flag: u8) -> bool {
    if cell == 0 {
        return false;
    }
    match e.mem.u8(cell.wrapping_add(CELL_STATE)) {
        2..=4 => flag == 0,
        5 | 6 => true,
        _ => false,
    }
}

// Typed from 004409e0
/// The model loader's word that a reference's task is over (`thiscall(loader, reference)`, `ret
/// 4`): the slot 0x14 of the loader's map of queued references with the reference, which for the
/// loader's map is the lock-free family's removal of the key ([`lockfree::map::remove`]), its typed
/// call when the slot holds it.
pub fn reference_done(e: &mut Engine, this: u32, reference: u32) {
    let map = e.mem.u32(this.wrapping_add(QUEUED_MAP));
    if slot_target(e, map, SLOT_REMOVE) == MAP_REMOVE {
        typed_call!(
            lockfree::FAMILY,
            0x0090_62e0,
            lockfree::map::remove(e, Ptr::new(map), reference)
        );
    } else {
        e.vcall(map, SLOT_REMOVE, &args![reference]);
    }
}

/// The cell's counter at `at` one less (an interlocked decrement, `004019a0`), kept from going below
/// 0 as the exe reads it back.
#[inline]
fn count_down(e: &mut Engine, at: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_19a0,
        dispatch::refs::decrement(e, Ptr::new(at))
    );
    if e.mem.i32(at) < 0 {
        e.mem.set_u32(at, 0);
    }
}

// Typed from 00440a10
/// The cell's count of queued references (`iQueuedRefCount`, Xbox PDB, +0xa4) one less
/// (`thiscall(cell)`), never below 0.
pub fn queued_count_down(e: &mut Engine, this: u32) {
    count_down(e, this.wrapping_add(CELL_QUEUED_COUNT));
}

// Typed from 00440a50
/// The cell's count of critical queued references (`iCriticalQueuedRefCount`, Xbox PDB, +0xa0) one
/// less (`thiscall(cell)`), never below 0.
pub fn critical_count_down(e: &mut Engine, this: u32) {
    count_down(e, this.wrapping_add(CELL_CRITICAL_COUNT));
}
