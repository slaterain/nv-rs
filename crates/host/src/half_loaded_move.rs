//! A half-loaded actor's process level change (`NV_HALF_LOADED_MOVE`), a fix of the `fixes`
//! preset ([`crate::fixes`], docs/FIXES.md): **not 1:1 with FalloutNV.exe**; in the `vanilla`
//! preset nothing here runs.
//!
//! **What the original does.** When a cell's references are loaded with their saved changes
//! (`BGSSaveLoadGame::LoadCell`, `00849b10`, on a loading thread), each changed actor gets two
//! passes. The first (`BGSSaveLoadGame::LoadForm`, `008495d0`) runs the actor's `LoadGame`
//! (`MobileObject::LoadGame` for a form buffer, `00932a00`), whose last step is the process's own
//! (vtable `+0x554`): every *reference* the process holds is stored as its saved form id, to be
//! found later. The second pass (`008492b0`) runs `InitLoadGame` (`00932b70`, then the process's
//! vtable `+0x55c`), which looks the ids up and stores the references (and marks the target
//! targeted, reserves its marker). Between the two, another loading thread can move the same actor
//! to another process level: finishing its 3D it sets its animation (`TESObjectREFR::SetAnimation`,
//! `00572e50`), which moves a middle-low or low actor to the high process (`00881d30`). The move
//! copies the old process into the new one and then works on the new one, and on the way follows
//! the saved ids as objects: the low level's copy (`00907510`) passes the target to `SetTarget`,
//! which marks it targeted through it (`TESObjectREFR::SetTargeted`, the flags at `id + 8`), then
//! `HighProcess::SetLastSeenLocation` (`008f6ea0`, the new process's vtable `+0x48`, from
//! `00882466`) reads the "target"'s vtable from `[id]` and calls through it. For an id such as
//! `0x0015413d` that is a word of low memory taken as a vtable: the original's own crash. The
//! port stopped one read earlier (the maintainer's run of 2026-11-09, "moving too fast": `read of
//! unmapped memory at 00154145 (in 00564db0 < 0091ea40 < 00907510 < 008d0660 < 00881d30 <
//! 00931920)`, on a loading thread). A recording of the original on the same fast route showed the
//! same interleaving (`LoadGame`, the other thread's `SetAnimation` and move to high, then
//! `InitLoadGame`, for one actor), with no target that time.
//!
//! **What this does.** It knows the gap as it is: from the return of the actor's `LoadGame` to
//! the start of its `InitLoadGame` (not the form flag `0x200000`, which `LoadForm` sets before
//! `LoadGame` begins: `LoadGame`'s own level change, `009319c0`, runs while the fields are still
//! references). A level change (`00881d30`, `00882b90`, `00883240`, `00883800`) of an actor in the
//! gap first takes the saved ids out of its process ([`SAVED_FIELDS`], [`SAVED_PACKAGES`]): the
//! fields read 0, the ordinary "no target, no furniture" state, so the whole move runs as the
//! translation's own for an actor without them (as in the recorded race). After it each id goes
//! back where the original's copy would have put it: into the new process at the same place when
//! the copy carries that field from the old level to the new one (and the field is still 0), and
//! nowhere when the copy drops it (the old process is deleted with it, as in the original); with
//! no new process, back into the old one. The second pass then resolves them as in a load without
//! the move: the target is marked and its marker reserved once. A package's id and its "still an
//! id" flag (bit 0 of `+0x14`, which `ActorPackage::MoveTo` leaves behind) and its target (`+8`)
//! go with the package. If the second pass reaches the actor while its move still runs, it puts
//! the ids back first; the move then puts back nothing.

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::fallout_ai::actor::{fn_00881d30, fn_00882b90, fn_00883240, fn_00883800};
use engine::units::fallout_ai::mobileobject::{fn_00932b70, mobile_object_load_game_ov2};
use std::collections::HashMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// The four process level changes (to the high, low, middle-low and middle-high process).
pub const LEVEL_CHANGES: [u32; 4] = [0x0088_1d30, 0x0088_2b90, 0x0088_3240, 0x0088_3800];
/// `MobileObject::LoadGame` and `InitLoadGame` for a form buffer (Xbox PDB `_ov2`).
pub const LOAD_GAME: u32 = 0x0093_2a00;
pub const INIT_LOAD_GAME: u32 = 0x0093_2b70;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "half-loaded actor's level change",
    env: "NV_HALF_LOADED_MOVE",
    addresses: &[
        LEVEL_CHANGES[0],
        LEVEL_CHANGES[1],
        LEVEL_CHANGES[2],
        LEVEL_CHANGES[3],
        LOAD_GAME,
        INIT_LOAD_GAME,
    ],
    summary: "an actor moved to another AI level while its saved data loads keeps its saved references for the second pass instead of following them",
    install,
};

/// `Actor`'s process (`pCurrentProcess`, the word `00881d30` reads as `param_1[0x1a]`) and the
/// process's level word (`0045cd60`: 0 high, 1 middle-high, 2 middle-low, 3 low).
const ACTOR_PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;

/// A process field the first pass leaves as a saved id: its offset, the lowest level that has
/// it (`level <= has`), and whether the level copy (`BaseProcess::Copy`, `008d0660`) takes it
/// from the old process to the new one.
struct SavedField {
    offset: u32,
    has: i32,
    copied: bool,
}

const fn field(offset: u32, has: i32, copied: bool) -> SavedField {
    SavedField {
        offset,
        has,
        copied,
    }
}

/// The process words each level's `LoadGame` (form buffer) stores as saved ids and its
/// `InitLoadGame` resolves. Low (`00910630` / `00910820`): the target `+0x40` (copied by
/// `00907510` through `SetTarget`), `+0x44`, `+0x48`, `+0x54` (not copied). Middle-low
/// (`0092eaa0`): none. Middle-high (`00927000` / `009278d0`): `+0x158` and the furniture `+0x140`
/// and `+0x164` (copied by `MiddleHighCopy`, `00914c60`), `+0x160`, `+0x1bc` (not copied). High
/// (`008fce90` / `008fdbd0`): `+0x30c`, `+0x2a4`, `+0x3f0`, `+0x41c`, `+0x370`, `+0x2ac` and the six
/// head-tracking targets `+0x3f8`..`+0x40c` (`HighProcess::HighCopy`, `008d9fa0`, runs only from a
/// high process to a high one, which no level change makes). The id lists (`+0x6c` low; `+0xc8`,
/// the active effects `+0x1b8` and the queued items `+0x230` middle-high; `+0x38c`, `+0x394`,
/// `+0x264`, the detection states high) stay where they are (module documentation of the PR).
const SAVED_FIELDS: &[SavedField] = &[
    field(0x40, 3, true),
    field(0x44, 3, false),
    field(0x48, 3, false),
    field(0x54, 3, false),
    field(0x140, 1, true),
    field(0x158, 1, true),
    field(0x164, 1, true),
    field(0x160, 1, false),
    field(0x1bc, 1, false),
    field(0x2a4, 0, false),
    field(0x2ac, 0, false),
    field(0x30c, 0, false),
    field(0x370, 0, false),
    field(0x3f0, 0, false),
    field(0x3f8, 0, false),
    field(0x3fc, 0, false),
    field(0x400, 0, false),
    field(0x404, 0, false),
    field(0x408, 0, false),
    field(0x40c, 0, false),
    field(0x41c, 0, false),
];

/// The packages in a process (`ActorPackage`, `008c6730` / `008c6970`): the current one at `+4`
/// (moved by the low copy) and the middle-high run-once one at `+0xe4` (moved by
/// `MiddleHighCopy`). Their saved ids: the target `+8`, and the package `+0` when bit 0 of `+0x14`
/// says it is still an id (a created package).
const SAVED_PACKAGES: &[SavedField] = &[field(0x04, 3, true), field(0xe4, 1, true)];
const PACKAGE_TARGET: u32 = 8;
const PACKAGE_FLAGS: u32 = 0x14;
const PACKAGE_STILL_AN_ID: u32 = 1;

/// The ids taken out of one process.
#[derive(Debug, Default, Clone)]
struct Parked {
    /// The process they came from.
    process: u32,
    /// (offset, id) of the fields.
    fields: Vec<(u32, u32)>,
    /// (package offset, the package's id when it is still one, its target id).
    packages: Vec<(u32, Option<u32>, u32)>,
}

#[derive(Debug)]
enum Gap {
    /// `LoadGame` returned; `InitLoadGame` has not begun.
    Open,
    /// A level change runs with the ids taken out.
    Moving(Parked),
}

type Key = (usize, u32);

fn gaps() -> MutexGuard<'static, HashMap<Key, Gap>> {
    static GAPS: OnceLock<Mutex<HashMap<Key, Gap>>> = OnceLock::new();
    GAPS.get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn level(e: &Engine, process: u32) -> i32 {
    e.mem.u32(process.wrapping_add(PROCESS_LEVEL)) as i32
}

/// Takes the saved ids out of `process` (they read 0 after).
fn park(e: &mut Engine, process: u32) -> Parked {
    let mut parked = Parked {
        process,
        ..Parked::default()
    };
    let at = level(e, process);
    for f in SAVED_FIELDS.iter().filter(|f| at <= f.has) {
        let id = e.mem.u32(process + f.offset);
        if id != 0 {
            parked.fields.push((f.offset, id));
            e.mem.set_u32(process + f.offset, 0);
        }
    }
    for p in SAVED_PACKAGES.iter().filter(|p| at <= p.has) {
        let package = process + p.offset;
        let flags = e.mem.u32(package + PACKAGE_FLAGS);
        let id = (flags & PACKAGE_STILL_AN_ID != 0).then(|| e.mem.u32(package));
        let target = e.mem.u32(package + PACKAGE_TARGET);
        if id.is_some() || target != 0 {
            parked.packages.push((p.offset, id, target));
            if id.is_some() {
                e.mem.set_u32(package, 0);
                e.mem
                    .set_u32(package + PACKAGE_FLAGS, flags & !PACKAGE_STILL_AN_ID);
            }
            e.mem.set_u32(package + PACKAGE_TARGET, 0);
        }
    }
    parked
}

/// Puts the ids back into `process`: every one when it is the process they came from, else only
/// those the level copy takes from their level to `process`'s; never over a word that is no
/// longer 0.
fn unpark(e: &mut Engine, parked: &Parked, process: u32) {
    if process == 0 {
        return;
    }
    let same = process == parked.process;
    let to = level(e, process);
    let from = if same { to } else { level(e, parked.process) };
    let goes = |offset: u32, table: &[SavedField]| {
        table
            .iter()
            .find(|f| f.offset == offset)
            .is_some_and(|f| same || (f.copied && from <= f.has && to <= f.has))
    };
    for &(offset, id) in &parked.fields {
        if goes(offset, SAVED_FIELDS) && e.mem.u32(process + offset) == 0 {
            e.mem.set_u32(process + offset, id);
        }
    }
    for &(offset, id, target) in &parked.packages {
        if !goes(offset, SAVED_PACKAGES) {
            continue;
        }
        let package = process + offset;
        if let Some(id) = id {
            if e.mem.u32(package) == 0 {
                e.mem.set_u32(package, id);
                let flags = e.mem.u32(package + PACKAGE_FLAGS);
                e.mem
                    .set_u32(package + PACKAGE_FLAGS, flags | PACKAGE_STILL_AN_ID);
            }
        }
        if target != 0 && e.mem.u32(package + PACKAGE_TARGET) == 0 {
            e.mem.set_u32(package + PACKAGE_TARGET, target);
        }
    }
}

/// Runs the level change `f` of `actor` with its saved ids taken out when it is in the gap.
pub fn level_change<R>(e: &mut Engine, actor: u32, f: impl FnOnce(&mut Engine) -> R) -> R {
    let key = (e.kernel.id(), actor);
    let process = e.mem.u32(actor.wrapping_add(ACTOR_PROCESS));
    let mine = {
        let mut gaps = gaps();
        match gaps.get_mut(&key) {
            Some(gap @ Gap::Open) if process != 0 => {
                *gap = Gap::Moving(park(e, process));
                true
            }
            _ => false,
        }
    };
    if !mine {
        return f(e);
    }
    // Put back on the way out whatever happens inside (a guest exception or a stop unwinds).
    let result = catch_unwind(AssertUnwindSafe(|| f(e)));
    let parked = {
        let mut gaps = gaps();
        match gaps.get_mut(&key) {
            Some(gap @ Gap::Moving(_)) => match std::mem::replace(gap, Gap::Open) {
                Gap::Moving(parked) => Some(parked),
                Gap::Open => None,
            },
            // The second pass took them back already.
            _ => None,
        }
    };
    if let Some(parked) = parked {
        let now = e.mem.u32(actor.wrapping_add(ACTOR_PROCESS));
        unpark(e, &parked, now);
    }
    match result {
        Ok(value) => value,
        Err(stop) => resume_unwind(stop),
    }
}

/// The actor's `LoadGame` returned: its process holds saved ids until `InitLoadGame`.
pub fn loaded(e: &Engine, actor: u32) {
    gaps().insert((e.kernel.id(), actor), Gap::Open);
}

/// The actor's `InitLoadGame` begins: the gap closes; ids a running level change took out go back
/// into the actor's process first, for this pass to resolve.
pub fn initialising(e: &mut Engine, actor: u32) {
    let gap = gaps().remove(&(e.kernel.id(), actor));
    if let Some(Gap::Moving(parked)) = gap {
        let process = e.mem.u32(actor.wrapping_add(ACTOR_PROCESS));
        let parked = Parked { process, ..parked };
        unpark(e, &parked, process);
    }
}

/// Registers the four level changes and the two passes' ends.
pub fn install(e: &mut Engine) {
    e.register(LEVEL_CHANGES[0], |e, a| {
        level_change(e, a[0], |e| fn_00881d30(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(LEVEL_CHANGES[1], |e, a| {
        level_change(e, a[0], |e| fn_00882b90(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(LEVEL_CHANGES[2], |e, a| {
        level_change(e, a[0], |e| fn_00883240(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(LEVEL_CHANGES[3], |e, a| {
        level_change(e, a[0], |e| fn_00883800(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(LOAD_GAME, |e, a| {
        mobile_object_load_game_ov2(e, Ptr::new(a[0]), Ptr::new(a[1]));
        loaded(e, a[0]);
        Ret::default()
    });
    e.register(INIT_LOAD_GAME, |e, a| {
        initialising(e, a[0]);
        fn_00932b70(e, Ptr::new(a[0]), Ptr::new(a[1]));
        Ret::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::units::fallout_ai::actorpackage::actor_package_move_to;
    use engine::units::fallout_ai::highprocess_p4::high_process_set_last_seen_location;
    use engine::units::fallout_ai::lowprocess::{fn_00907510, low_process_init_load_game_ov2};
    use engine::units::fallout_ai::middlehighprocess::middle_high_process_set_target;
    use engine::units::fallout_shared::tesobjectrefr_p2::tes_object_refr_set_targeted;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The saved id in the maintainer's stop (`0x00154145 - 8`), a package target's id and a
    /// created package's id.
    const SAVED_ID: u32 = 0x0015_413d;
    const PACKAGE_TARGET_ID: u32 = 0x0010_2030;
    const CREATED_PACKAGE_ID: u32 = 0xff00_0801;
    /// The test vtables (every slot an address nothing is registered at, which answers 0,
    /// except the slots set below) and the test functions.
    const LOW_VTABLE: u32 = 0x00d0_0000;
    const HIGH_VTABLE: u32 = 0x00d1_0000;
    const ACTOR_VTABLE: u32 = 0x00d2_0000;
    const TARGET_VTABLE: u32 = 0x00d3_0000;
    const OPEN_SLOTS: u32 = 0x00e0_0000;
    const TEST_COPY: u32 = 0x00d8_0004;
    const TEST_GET_TARGET: u32 = 0x00d8_0128;
    const TEST_IS_A_FOLLOWER: u32 = 0x00d8_0360;
    const TEST_RUNNING_PACKAGE: u32 = 0x00d8_0278;
    const TEST_CURRENT_PACKAGE: u32 = 0x00d8_022c;
    const TEST_LOCATION: u32 = 0x00d8_01f4;
    /// The exe's functions on the path, run as translated (or as the exe's own one-liners).
    const MOVE_TO_HIGH: u32 = 0x0088_1d30;
    const LOW_COPY: u32 = 0x0090_7510;
    const MIDDLE_HIGH_SET_TARGET: u32 = 0x0091_ea40;
    const SET_TARGETED: u32 = 0x0056_4db0;
    const SET_MARKER_RESERVED: u32 = 0x0056_8150;
    const GET_FIRST_FREE_MARKER_INDEX: u32 = 0x0056_82c0;
    const SET_LAST_SEEN_LOCATION: u32 = 0x008f_6ea0;
    const LOW_INIT_LOAD_GAME: u32 = 0x0091_0820;
    const PACKAGE_MOVE_TO: u32 = 0x008c_6130;
    const STORE_AT_8: u32 = 0x0040_3550;
    const OPERATOR_NEW: u32 = 0x0040_1000;
    const HIGH_PROCESS_CONSTRUCTOR: u32 = 0x008d_7510;
    const ACTOR_PROCESS_GETTER: u32 = 0x008d_8520;
    const PROCESS_LEVEL_GETTER: u32 = 0x0045_cd60;
    const FLAGS_GETTER: u32 = 0x0044_ddc0;
    const REFERENCE_FLOAT_POINTER: u32 = 0x0043_0830;
    const LOOKUP_FORM_BY_ID: u32 = 0x0048_39c0;
    const DYNAMIC_CAST: u32 = 0x00ec_43fb;
    const RESERVES_TARGET: u32 = 0x0067_8d00;
    /// `TESObjectREFR::SetTargeted`'s flag, and the form flag `LoadForm` sets for the whole
    /// load (wider than the gap: module documentation).
    const TARGETED: u32 = 0x4_0000;
    const LOADING_FLAG: u32 = 0x0020_0000;

    type Log = Rc<RefCell<Vec<u32>>>;

    struct World {
        e: Box<Engine>,
        log: Log,
        actor: u32,
        old: u32,
        target: u32,
        package: u32,
    }

    /// An actor whose old low process holds `target_word` at `+0x40` and its current package
    /// `package_words` at `+0`, `+8`, `+0x14`; a target reference with flags; the move to high
    /// with the low level's copy, `SetTarget`, `SetLastSeenLocation`, the follower check (the
    /// actor is a follower) and the second pass's low `InitLoadGame` translated; every other
    /// call answers 0. `fix` installs the row (else the move to high is the translation's).
    fn world(fix: bool, target_word: u32, package_words: [u32; 3]) -> World {
        let mut e = Box::new(Engine::new());
        let log: Log = Rc::default();
        let open = log.clone();
        e.on_open = Some(Rc::new(RefCell::new(
            move |_: &mut Engine, addr: u32, _: &[u32]| {
                open.borrow_mut().push(addr);
                Some(Ret::default())
            },
        )));
        e.map(0x0100_0000, 0x4_0000);
        e.map(0x011c_0000, 0x6_0000);
        // The save/load game, the save/load record, the player and the data handler: blank
        // objects (their flags read as clear).
        for global in [0x011d_df38u32, 0x011d_e45c, 0x011d_ea3c, 0x011c_3f2c] {
            let object = e.mem.alloc(0x1000);
            e.mem.set_u32(global, object);
        }
        for vtable in [LOW_VTABLE, HIGH_VTABLE, ACTOR_VTABLE, TARGET_VTABLE] {
            e.map(vtable, 0x1000);
            for slot in (0..0x1000).step_by(4) {
                e.mem.set_u32(vtable + slot, OPEN_SLOTS + slot);
            }
        }
        if fix {
            install(&mut e);
        } else {
            e.register(MOVE_TO_HIGH, |e, a| {
                fn_00881d30(e, Ptr::new(a[0])).into_ret()
            });
        }
        e.register(LOW_COPY, |e, a| {
            fn_00907510(e, Ptr::new(a[0]), Ptr::new(a[1]));
            Ret::default()
        });
        // `BaseProcess::Copy` of a low process into a high one: only the low level copies.
        e.register(TEST_COPY, |e, a| e.call(LOW_COPY, &args![a[0], a[1]]));
        e.register(MIDDLE_HIGH_SET_TARGET, |e, a| {
            middle_high_process_set_target(e, Ptr::new(a[0]), a[1]);
            Ret::default()
        });
        let marks = log.clone();
        e.register_double(SET_TARGETED, move |e, a| {
            marks.borrow_mut().push(SET_TARGETED);
            tes_object_refr_set_targeted(e, Ptr::new(a[0]), a[1] as u8);
            Ret::default()
        });
        e.register(SET_LAST_SEEN_LOCATION, |e, a| {
            high_process_set_last_seen_location(e, Ptr::new(a[0]));
            Ret::default()
        });
        e.register(LOW_INIT_LOAD_GAME, |e, a| {
            low_process_init_load_game_ov2(e, Ptr::new(a[0]), a[1]);
            Ret::default()
        });
        e.register(PACKAGE_MOVE_TO, |e, a| {
            actor_package_move_to(e, Ptr::new(a[0]), Ptr::new(a[1]));
            Ret::default()
        });
        e.register(STORE_AT_8, |e, a| {
            e.mem.set_u32(a[0] + 8, a[1]);
            Ret::default()
        });
        e.register(FLAGS_GETTER, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(OPERATOR_NEW, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(HIGH_PROCESS_CONSTRUCTOR, |e, a| {
            e.mem.set_u32(a[0], HIGH_VTABLE);
            e.mem.set_u32(a[0] + 0x1a8, u32::MAX);
            a[0].into_ret()
        });
        e.register(ACTOR_PROCESS_GETTER, |e, a| {
            e.mem.u32(a[0] + ACTOR_PROCESS).into_ret()
        });
        e.register(PROCESS_LEVEL_GETTER, |e, a| {
            e.mem.u32(a[0] + PROCESS_LEVEL).into_ret()
        });
        let one = e.mem.alloc(4);
        e.mem.set_f32(one, 1.0);
        e.register_double(REFERENCE_FLOAT_POINTER, move |_, _| one.into_ret());
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        e.register(RESERVES_TARGET, |_, _| 1u32.into_ret());
        for (slot, function) in [
            (4, TEST_COPY),
            (0x48, SET_LAST_SEEN_LOCATION),
            (0x12c, MIDDLE_HIGH_SET_TARGET),
            (0x128, TEST_GET_TARGET),
            (0x360, TEST_IS_A_FOLLOWER),
            (0x278, TEST_RUNNING_PACKAGE),
            (0x22c, TEST_CURRENT_PACKAGE),
            (0x55c, LOW_INIT_LOAD_GAME),
        ] {
            e.mem.set_u32(HIGH_VTABLE + slot, function);
        }
        e.register(TEST_GET_TARGET, |e, a| e.mem.u32(a[0] + 0x40).into_ret());
        e.register(TEST_IS_A_FOLLOWER, |_, _| 1u32.into_ret());
        e.register(TEST_RUNNING_PACKAGE, |_, a| (a[0] + 4).into_ret());
        let reserving = e.mem.alloc(0x20);
        e.register_double(TEST_CURRENT_PACKAGE, move |_, _| reserving.into_ret());
        let target = e.mem.alloc(0x40);
        e.mem.set_u32(target, TARGET_VTABLE);
        e.mem.set_u32(target + 8, 0x10);
        e.mem.set_u32(TARGET_VTABLE + 0x1f4, TEST_LOCATION);
        let location = e.mem.alloc(12);
        e.register_double(TEST_LOCATION, move |_, _| location.into_ret());
        e.register_double(LOOKUP_FORM_BY_ID, move |_, a| {
            (if a[0] == SAVED_ID { target } else { 0 }).into_ret()
        });
        let actor = e.mem.alloc(0x200);
        e.mem.set_u32(actor, ACTOR_VTABLE);
        e.mem.set_u32(actor + 8, LOADING_FLAG);
        let old = e.mem.alloc(0x400);
        e.mem.set_u32(old, LOW_VTABLE);
        e.mem.set_u32(old + PROCESS_LEVEL, 3);
        e.mem.set_u32(old + 0x40, target_word);
        let package = old + 4;
        e.mem.set_u32(package, package_words[0]);
        e.mem.set_u32(package + PACKAGE_TARGET, package_words[1]);
        e.mem.set_u32(package + PACKAGE_FLAGS, package_words[2]);
        e.mem.set_u32(actor + ACTOR_PROCESS, old);
        World {
            e,
            log,
            actor,
            old,
            target,
            package,
        }
    }

    impl World {
        fn process(&self) -> u32 {
            self.e.mem.u32(self.actor + ACTOR_PROCESS)
        }
        fn count(&self, addr: u32) -> usize {
            self.log.borrow().iter().filter(|&&a| a == addr).count()
        }
        /// The move to high as the loading thread's `SetAnimation` makes it.
        fn move_to_high(&mut self) {
            let actor = self.actor;
            self.e.call(MOVE_TO_HIGH, &args![actor]);
        }
        /// The second pass on the actor: the gap closes, then the process's `InitLoadGame`.
        fn second_pass(&mut self) {
            let actor = self.actor;
            initialising(&mut self.e, actor);
            let process = self.process();
            self.e.vcall(process, 0x55c, &args![0u32]);
        }
    }

    fn stop_text(stop: Box<dyn std::any::Any + Send>) -> String {
        stop.downcast_ref::<String>()
            .cloned()
            .or_else(|| stop.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()
    }

    #[test]
    fn vanilla_the_move_in_the_gap_follows_the_saved_target_and_stops() {
        let mut w = world(false, SAVED_ID, [0, 0, 0]);
        let actor = w.actor;
        loaded(&w.e, actor);
        let stop = catch_unwind(AssertUnwindSafe(|| w.move_to_high())).unwrap_err();
        let text = stop_text(stop);
        assert!(text.contains("00154145"), "{text}");
        gaps().remove(&(w.e.kernel.id(), actor));
    }

    #[test]
    fn the_move_in_the_gap_leaves_the_target_to_the_second_pass() {
        let mut w = world(true, SAVED_ID, [0, 0, 0]);
        let actor = w.actor;
        loaded(&w.e, actor);
        w.move_to_high();
        let new = w.process();
        assert_ne!(new, w.old);
        // The move saw no target: nothing marked, located or reserved through the id; the old
        // process keeps nothing; the new one holds the id for the second pass.
        assert_eq!(w.count(SET_TARGETED), 0);
        assert_eq!(w.count(SET_MARKER_RESERVED), 0);
        assert_eq!(w.e.mem.u32(w.old + 0x40), 0);
        assert_eq!(w.e.mem.u32(new + 0x40), SAVED_ID);
        w.second_pass();
        // Resolved, marked once, its marker reserved once.
        assert_eq!(w.e.mem.u32(new + 0x40), w.target);
        assert_eq!(w.count(SET_TARGETED), 1);
        assert_eq!(w.e.mem.u32(w.target + 8), 0x10 | TARGETED);
        assert_eq!(w.count(GET_FIRST_FREE_MARKER_INDEX), 1);
        assert_eq!(w.count(SET_MARKER_RESERVED), 1);
        assert!(!gaps().contains_key(&(w.e.kernel.id(), actor)));
    }

    #[test]
    fn a_move_before_the_ids_are_written_is_the_translations() {
        // `LoadGame`'s own level change: the flag set, the gap not open, a real target.
        for fix in [false, true] {
            let mut w = world(fix, 0, [0, 0, 0]);
            let (target, old) = (w.target, w.old);
            w.e.mem.set_u32(old + 0x40, target);
            w.move_to_high();
            let new = w.process();
            assert_eq!(w.e.mem.u32(new + 0x40), target, "fix {fix}");
            assert_eq!(w.count(SET_TARGETED), 1, "fix {fix}");
            assert_eq!(w.e.mem.u32(target + 8), 0x10 | TARGETED, "fix {fix}");
            assert_eq!(w.count(SET_MARKER_RESERVED), 1, "fix {fix}");
        }
    }

    #[test]
    fn a_created_package_and_its_target_go_with_the_package() {
        let mut w = world(
            true,
            0,
            [CREATED_PACKAGE_ID, PACKAGE_TARGET_ID, PACKAGE_STILL_AN_ID],
        );
        let actor = w.actor;
        loaded(&w.e, actor);
        w.move_to_high();
        let package = w.process() + 4;
        assert_eq!(w.e.mem.u32(package), CREATED_PACKAGE_ID);
        assert_eq!(w.e.mem.u32(package + PACKAGE_TARGET), PACKAGE_TARGET_ID);
        assert_eq!(
            w.e.mem.u32(package + PACKAGE_FLAGS) & PACKAGE_STILL_AN_ID,
            PACKAGE_STILL_AN_ID
        );
        assert_eq!(w.e.mem.u32(w.package), 0);
        assert_eq!(w.e.mem.u32(w.package + PACKAGE_TARGET), 0);
        initialising(&mut w.e, actor);
    }

    #[test]
    fn fields_the_copy_drops_are_dropped_and_a_move_that_keeps_the_process_keeps_them() {
        let mut w = world(true, SAVED_ID, [0, 0, 0]);
        let (actor, old) = (w.actor, w.old);
        w.e.mem.set_u32(old + 0x44, 0x0012_0001);
        loaded(&w.e, actor);
        // A level change that keeps the process (the actor already at that level).
        level_change(&mut w.e, actor, |e| {
            assert_eq!(e.mem.u32(old + 0x40), 0);
            assert_eq!(e.mem.u32(old + 0x44), 0);
        });
        assert_eq!(w.e.mem.u32(old + 0x40), SAVED_ID);
        assert_eq!(w.e.mem.u32(old + 0x44), 0x0012_0001);
        // The move to high: the low copy takes the target, not +0x44.
        w.move_to_high();
        let new = w.process();
        assert_eq!(w.e.mem.u32(new + 0x40), SAVED_ID);
        assert_eq!(w.e.mem.u32(new + 0x44), 0);
        initialising(&mut w.e, actor);
    }

    #[test]
    fn a_stop_inside_the_move_puts_the_ids_back() {
        let mut w = world(true, SAVED_ID, [0, 0, 0]);
        let (actor, old) = (w.actor, w.old);
        loaded(&w.e, actor);
        let stop = catch_unwind(AssertUnwindSafe(|| {
            level_change(&mut w.e, actor, |_| panic!("a guest exception"))
        }));
        assert!(stop.is_err());
        assert_eq!(w.e.mem.u32(old + 0x40), SAVED_ID);
        // The gap is still open: the next move takes them out again.
        level_change(&mut w.e, actor, |e| assert_eq!(e.mem.u32(old + 0x40), 0));
        initialising(&mut w.e, actor);
    }

    #[test]
    fn the_second_pass_during_the_move_takes_the_ids_back_first() {
        let mut w = world(true, SAVED_ID, [0, 0, 0]);
        let (actor, old) = (w.actor, w.old);
        loaded(&w.e, actor);
        level_change(&mut w.e, actor, |e| {
            initialising(e, actor);
            assert_eq!(e.mem.u32(old + 0x40), SAVED_ID);
            // The pass resolves it here; the move puts nothing back over it.
            e.mem.set_u32(old + 0x40, 0x0000_7000);
        });
        assert_eq!(w.e.mem.u32(old + 0x40), 0x0000_7000);
        assert!(!gaps().contains_key(&(w.e.kernel.id(), actor)));
    }

    #[test]
    fn the_load_and_init_entries_open_and_close_the_gap() {
        let mut w = world(true, 0, [0, 0, 0]);
        let actor = w.actor;
        let key = (w.e.kernel.id(), actor);
        assert!(!gaps().contains_key(&key));
        loaded(&w.e, actor);
        assert!(matches!(gaps().get(&key), Some(Gap::Open)));
        initialising(&mut w.e, actor);
        assert!(!gaps().contains_key(&key));
    }
}
