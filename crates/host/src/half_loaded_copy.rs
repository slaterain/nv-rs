//! A half-loaded actor's process copied to another level: the saved reference ids are carried
//! through untouched (docs/FIXES.md, "Where every preset differs from the original"). Installed
//! in **both presets**: the original's own instructions on this path write into memory the port
//! does not have, and the port must not stop there.
//!
//! **What the original does.** When a cell's references are loaded with their saved changes
//! (`BGSSaveLoadGame::LoadCell`, `00849b10`, from `00545cf0`), each changed form gets two
//! passes. The first (`BGSSaveLoadGame::LoadForm`, `008495d0`) sets the form's flag `0x200000`
//! (`0046a010(1)` at `008496d2`) and runs its `LoadGame` (form vtable `+0x5c`); for an actor that
//! reads its AI process's saved data (`MobileObject::LoadGame`, `00932a00`, then the process's
//! vtable `+0x554`), and every *reference* the process holds is stored as its saved form id, to be
//! found later: `LowProcess` (`00910630`) the target `+0x40`, `+0x44`, `+0x48`, `+0x54` and the
//! generic locations; `MiddleHighProcess` (`009278d0` resolves them) `+0x140` (the furniture),
//! `+0x158`, `+0xc8`, `+0x160`, `+0x164`, `+0x1bc`. The second pass (`008492b0`, at `0054665d`)
//! runs each form's `InitLoadGame` (vtable `+0x64`), which turns those ids into references
//! (`LowProcess::InitLoadGame`, `00910820`: `SetTarget` of the target found by its id), and then
//! clears the flag (`0046a010(0)` at `008494d8`). Between the two, the flag says "this form's saved
//! fields are still ids", and the game's own process code tests it (`00576d30`: the move to the
//! high process `00881d30` at `00881f34`, the process lists' updates, `SetTargetForPackage`).
//!
//! In that gap another thread can move the same actor to another process level: the loading
//! thread finishing the actor's 3D sets its animation (`TESObjectREFR::SetAnimation`, `00572e50`;
//! a middle-low or low actor then `ChangeProcessLevel(0)` at `00572ef0`), and the transition
//! (`00881d30`; also `00882b90`, `00883240`, `00883800`) builds the new process and copies the old
//! one into it (`BaseProcess::Copy`, `008d0660`, vtable `+4`). The copy passes the saved fields
//! on as they are, and the second pass then resolves them in the new process just as it would have
//! in the old one. But on the way three calls read through a field as if it were a reference
//! already: the low level's copy (`00907510`) gives the target to the new process's `SetTarget`
//! (vtable `+0x12c`: `0091ea40` or `00911e80`), which marks it targeted
//! (`TESObjectREFR::SetTargeted`, `00564db0`: the form flags at `+8` read and written back with
//! `0x40000`) and may reserve one of its markers (`TESObjectREFR::GetFirstFreeMarkerIndex`
//! `005682c0`, `SetMarkerReserved` `00568150`); `MiddleHighProcess::MiddleHighCopy` (`00914c60`)
//! marks the furniture targeted; and the reset of an old high process (`SetTarget(0)`, at the start
//! of `Copy`) frees the old target's and furniture's reserved markers
//! (`MiddleHighProcess::ClearReservation`, `00914b40`). With a saved id such as `0x0015413d` the
//! original reads and writes a bit at `id + 8` (`0x00154145`): low memory a 32-bit Windows process
//! usually has mapped (its first heaps and stacks), so it changes a bit of something unrelated and
//! carries on. The port has nothing mapped there and stopped (the maintainer's play run of
//! 2026-11-09, "moving too fast": `read of unmapped memory at 00154145 (in 00564db0 < 0091ea40 <
//! 00907510 < 008d0660 < 00881d30 < 00931920)`, on the loading thread).
//!
//! **What this does.** Only while a process transition of an actor with the flag `0x200000` runs
//! its `Copy`: `SetTargeted` and `SetMarkerReserved` do nothing and `GetFirstFreeMarkerIndex`
//! answers "none" (-1, its own answer for a reference without a free marker). Everything else of
//! `SetTarget`, `ClearReservation` and the level copies runs as translated, so the new process
//! holds the saved ids exactly as the original's does and the second pass resolves, marks and
//! reserves them in the same way. Every other call of the three, and every other transition,
//! is the translation's own. Nothing a player sees differs: in the original the marks went to the
//! wrong memory, and the real target is marked by the second pass in both.

use engine::prelude::*;
use engine::units::fallout_ai::actor::{fn_00881d30, fn_00882b90, fn_00883240, fn_00883800};
use engine::units::fallout_ai::baseprocess::base_process_copy;
use engine::units::fallout_shared::tesobjectrefr_p2::{
    tes_object_refr_get_first_free_marker_index, tes_object_refr_set_marker_reserved,
    tes_object_refr_set_targeted,
};
use std::cell::Cell;

/// The four process transitions (move to the high, middle-high, middle-low and low process).
pub const TRANSITIONS: [u32; 4] = [0x0088_1d30, 0x0088_2b90, 0x0088_3240, 0x0088_3800];
/// `BaseProcess::Copy` (Xbox PDB), every process level's vtable `+4`.
pub const PROCESS_COPY: u32 = 0x008d_0660;
/// `TESObjectREFR::SetTargeted`, `SetMarkerReserved` and `GetFirstFreeMarkerIndex` (Xbox PDB).
pub const SET_TARGETED: u32 = 0x0056_4db0;
pub const SET_MARKER_RESERVED: u32 = 0x0056_8150;
pub const GET_FIRST_FREE_MARKER_INDEX: u32 = 0x0056_82c0;
/// The form flag `BGSSaveLoadGame::LoadForm` sets before a form's `LoadGame` and the second pass
/// clears after its `InitLoadGame` (`0046a010`; tested by `00576d30`), in the form's flags at `+8`.
pub const SAVED_FIELDS_UNRESOLVED: u32 = 0x0020_0000;
const FORM_FLAGS: u32 = 8;

thread_local! {
    /// The innermost transition running on this thread is of an actor with the flag.
    static TRANSITION_OF_HALF_LOADED: Cell<bool> = const { Cell::new(false) };
    /// That transition's `Copy` is running (a depth: the copy calls no transition today).
    static COPYING_SAVED_FIELDS: Cell<u32> = const { Cell::new(0) };
}

/// Whether a call made now reads a field of a half-loaded actor's process that is still a saved id.
fn copying_saved_fields() -> bool {
    COPYING_SAVED_FIELDS.with(Cell::get) != 0
}

/// Runs `f` as a transition of `actor` (its flag read at the start, as `00881d30` reads it).
pub fn in_transition<R>(e: &mut Engine, actor: u32, f: impl FnOnce(&mut Engine) -> R) -> R {
    let half_loaded = e.mem.u32(actor.wrapping_add(FORM_FLAGS)) & SAVED_FIELDS_UNRESOLVED != 0;
    let outer = TRANSITION_OF_HALF_LOADED.with(|t| t.replace(half_loaded));
    // A copy under way stays a copy only inside its own transition.
    let depth = COPYING_SAVED_FIELDS.with(|c| c.replace(0));
    let result = f(e);
    COPYING_SAVED_FIELDS.with(|c| c.set(depth));
    TRANSITION_OF_HALF_LOADED.with(|t| t.set(outer));
    result
}

/// The first copies of the run are said in the log, and the first ones whose target is still a
/// saved id (the case that stopped the port): a play report then shows the gap was met.
fn note_copy(e: &Engine, old_process: u32) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SAID: AtomicU32 = AtomicU32::new(0);
    static SAID_WITH_TARGET: AtomicU32 = AtomicU32::new(0);
    if old_process == 0 {
        return;
    }
    let target = e.mem.u32(old_process.wrapping_add(0x40));
    let said = if target == 0 {
        &SAID
    } else {
        &SAID_WITH_TARGET
    };
    if said.fetch_add(1, Ordering::Relaxed) >= 5 {
        return;
    }
    eprintln!(
        "half-loaded actor's process copied to another level: its saved ids carried through \
         (the target word {target:08x})"
    );
}

/// Runs `f` as a transition's `Copy`.
pub fn in_copy<R>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> R) -> R {
    let saved = TRANSITION_OF_HALF_LOADED.with(Cell::get);
    if saved {
        COPYING_SAVED_FIELDS.with(|c| c.set(c.get() + 1));
    }
    let result = f(e);
    if saved {
        COPYING_SAVED_FIELDS.with(|c| c.set(c.get() - 1));
    }
    result
}

/// Registers the transitions, the copy and the three reference calls (every preset).
pub fn install(e: &mut Engine) {
    e.register(TRANSITIONS[0], |e, a| {
        in_transition(e, a[0], |e| fn_00881d30(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(TRANSITIONS[1], |e, a| {
        in_transition(e, a[0], |e| fn_00882b90(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(TRANSITIONS[2], |e, a| {
        in_transition(e, a[0], |e| fn_00883240(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(TRANSITIONS[3], |e, a| {
        in_transition(e, a[0], |e| fn_00883800(e, Ptr::new(a[0]))).into_ret()
    });
    e.register(PROCESS_COPY, |e, a| {
        let (this, other) = (a[0], a[1]);
        if TRANSITION_OF_HALF_LOADED.with(Cell::get) {
            note_copy(e, other);
        }
        in_copy(e, |e| base_process_copy(e, Ptr::new(this), Ptr::new(other)));
        Ret::default()
    });
    e.register(SET_TARGETED, |e, a| {
        if !copying_saved_fields() {
            tes_object_refr_set_targeted(e, Ptr::new(a[0]), a[1] as u8);
        }
        Ret::default()
    });
    e.register(SET_MARKER_RESERVED, |e, a| {
        if !copying_saved_fields() {
            tes_object_refr_set_marker_reserved(e, Ptr::new(a[0]), a[1], a[2] as u8);
        }
        Ret::default()
    });
    e.register(GET_FIRST_FREE_MARKER_INDEX, |e, a| {
        if copying_saved_fields() {
            (-1i32).into_ret()
        } else {
            tes_object_refr_get_first_free_marker_index(e, Ptr::new(a[0]), a[1] as u8).into_ret()
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::units::fallout_ai::lowprocess::fn_00907510;
    use engine::units::fallout_ai::middlehighprocess::middle_high_process_set_target;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    /// The saved id in the maintainer's stop (`0x00154145 - 8`).
    const SAVED_ID: u32 = 0x0015_413d;
    /// `MiddleHighProcess::SetTarget` (the high process's vtable `+0x12c`).
    const MIDDLE_HIGH_SET_TARGET: u32 = 0x0091_ea40;
    /// The low level's copy and what it calls besides `SetTarget`: the hour (set),
    /// `ActorPackage::MoveTo`, the date and the modifier list's copy; and the hour's getter.
    const LOW_COPY: u32 = 0x0090_7510;
    const LOW_COPY_CALLS: [u32; 4] = [0x0069_3d50, 0x008c_6130, 0x0059_bb30, 0x0093_7160];
    const GET_HOUR: u32 = 0x007d_f1f0;
    /// The word store at `+8` (`00403550`): the running package's target and a form's flags.
    const STORE_AT_8: u32 = 0x0040_3550;
    /// `MiddleHighProcess::ClearReservation`'s data-handler and save/load flag tests, their
    /// globals, and the package's "reserves its target" test.
    const DATA_HANDLER_TEST: u32 = 0x0042_26e0;
    const SAVE_LOAD_TEST: u32 = 0x0042_ce10;
    const DATA_HANDLER: u32 = 0x011c_3f2c;
    const SAVE_LOAD_GAME: u32 = 0x011d_df38;
    const RESERVES_TARGET: u32 = 0x0067_8d00;
    /// The new process's vtable slots (test functions): the running package, the current
    /// package, the "no target" call.
    const RUNNING_PACKAGE: u32 = 0x00c0_0278;
    const CURRENT_PACKAGE: u32 = 0x00c0_022c;
    const NO_TARGET: u32 = 0x00c0_0294;

    struct World {
        e: Box<Engine>,
        actor: u32,
        old: u32,
        new: u32,
        holder: u32,
    }

    /// An actor (flag `0x200000` when `half_loaded`), its old process with `target` at `+0x40`,
    /// and a new high process (the middle-high `SetTarget`, a running package `holder`, and a
    /// current package that reserves its target when `reserving`).
    fn world(guard: bool, half_loaded: bool, target: u32, reserving: bool) -> World {
        let mut e = Box::new(Engine::new());
        e.call_log = Some(Vec::new());
        if guard {
            install(&mut e);
        } else {
            e.register(SET_TARGETED, |e, a| {
                tes_object_refr_set_targeted(e, Ptr::new(a[0]), a[1] as u8);
                Ret::default()
            });
        }
        for page in [DATA_HANDLER, SAVE_LOAD_GAME] {
            e.map(page & !0xfff, 0x1000);
        }
        e.register(LOW_COPY, |e, a| {
            fn_00907510(e, Ptr::new(a[0]), Ptr::new(a[1]));
            Ret::default()
        });
        e.register(MIDDLE_HIGH_SET_TARGET, |e, a| {
            middle_high_process_set_target(e, Ptr::new(a[0]), a[1]);
            Ret::default()
        });
        let stubs =
            LOW_COPY_CALLS
                .into_iter()
                .chain([NO_TARGET, DATA_HANDLER_TEST, SAVE_LOAD_TEST]);
        for address in stubs {
            e.register(address, |_, _| Ret::default());
        }
        e.register(GET_HOUR, |_, _| Ret {
            st0: 8.0,
            ..Ret::default()
        });
        e.register(STORE_AT_8, |e, a| {
            e.mem.set_u32(a[0] + 8, a[1]);
            Ret::default()
        });
        e.register(RESERVES_TARGET, |_, _| 1u32.into_ret());
        let holder = e.mem.alloc(0x10);
        e.register_double(RUNNING_PACKAGE, move |_, _| holder.into_ret());
        let package = if reserving { e.mem.alloc(0x10) } else { 0 };
        e.register_double(CURRENT_PACKAGE, move |_, _| package.into_ret());
        let actor = e.mem.alloc(0x200);
        if half_loaded {
            e.mem.set_u32(actor + FORM_FLAGS, SAVED_FIELDS_UNRESOLVED);
        }
        let old = e.mem.alloc(0x400);
        e.mem.set_u32(old + 0x40, target);
        let new = e.mem.alloc(0x400);
        let vtable = e.mem.alloc(0x800);
        e.mem.set_u32(new, vtable);
        e.mem.set_u32(vtable + 0x12c, MIDDLE_HIGH_SET_TARGET);
        e.mem.set_u32(vtable + 0x278, RUNNING_PACKAGE);
        e.mem.set_u32(vtable + 0x22c, CURRENT_PACKAGE);
        e.mem.set_u32(vtable + 0x294, NO_TARGET);
        e.mem.set_u32(new + 0x1a8, u32::MAX);
        World {
            e,
            actor,
            old,
            new,
            holder,
        }
    }

    fn calls_to(e: &Engine, callee: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(address, _)| *address == callee)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The low level's copy as a transition of the actor makes it.
    fn copy_in_transition(w: &mut World) {
        let (actor, new, old) = (w.actor, w.new, w.old);
        in_transition(&mut w.e, actor, |e| {
            in_copy(e, |e| e.call(LOW_COPY, &args![new, old]));
        });
    }

    #[test]
    fn without_the_guard_the_copy_stops_where_the_maintainers_run_did() {
        let mut w = world(false, true, SAVED_ID, false);
        let (new, old) = (w.new, w.old);
        let stop = catch_unwind(AssertUnwindSafe(|| {
            w.e.call(LOW_COPY, &args![new, old]);
        }))
        .unwrap_err();
        let text = stop
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| stop.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        assert!(text.contains("00154145"), "{text}");
    }

    #[test]
    fn a_half_loaded_actors_copy_carries_the_saved_target_through() {
        let mut w = world(true, true, SAVED_ID, false);
        copy_in_transition(&mut w);
        // The new process and its running package hold the id, as the original's do, for the
        // second pass to resolve; nothing read through it, and no "no target" call.
        assert_eq!(w.e.mem.u32(w.new + 0x40), SAVED_ID);
        assert_eq!(w.e.mem.u32(w.holder + 8), SAVED_ID);
        assert!(calls_to(&w.e, NO_TARGET).is_empty());
        assert_eq!(calls_to(&w.e, SET_TARGETED), vec![vec![SAVED_ID, 1]]);
    }

    #[test]
    fn a_reserving_package_gets_no_marker_through_the_saved_id() {
        let mut w = world(true, true, SAVED_ID, true);
        copy_in_transition(&mut w);
        assert_eq!(w.e.mem.u32(w.new + 0x40), SAVED_ID);
        // "No free marker": the slot the original fills from the id's memory stays empty.
        assert_eq!(w.e.mem.u32(w.new + 0x1a8), u32::MAX);
        assert_eq!(calls_to(&w.e, GET_FIRST_FREE_MARKER_INDEX).len(), 1);
        assert_eq!(calls_to(&w.e, SET_MARKER_RESERVED).len(), 1);
    }

    #[test]
    fn a_loaded_actors_copy_marks_its_target_as_the_translation_does() {
        let mut w = world(true, false, 0, false);
        let target = w.e.mem.alloc(0x40);
        w.e.mem.set_u32(target + 8, 0x10);
        let old = w.old;
        w.e.mem.set_u32(old + 0x40, target);
        copy_in_transition(&mut w);
        assert_eq!(w.e.mem.u32(w.new + 0x40), target);
        assert_eq!(w.e.mem.u32(target + 8), 0x10 | 0x4_0000);
    }

    #[test]
    fn outside_the_copy_a_half_loaded_actors_calls_are_the_translations() {
        let mut w = world(true, true, 0, false);
        let target = w.e.mem.alloc(0x40);
        let (actor, new) = (w.actor, w.new);
        // In the transition but not in its copy (a target the transition picks itself).
        in_transition(&mut w.e, actor, |e| {
            e.vcall(new, 0x12c, &args![target]);
        });
        assert_eq!(w.e.mem.u32(target + 8), 0x4_0000);
    }

    #[test]
    fn the_reset_of_a_half_loaded_high_process_frees_no_marker_through_an_id() {
        let mut w = world(true, true, 0, false);
        let new = w.new;
        // A high process being reset (`SetTarget(0)` at the start of `Copy`) with a saved target,
        // its reserved marker slot and a saved furniture id.
        w.e.mem.set_u32(new + 0x40, SAVED_ID);
        w.e.mem.set_u32(new + 0x1a8, 2);
        w.e.mem.set_u32(new + 0x140, SAVED_ID + 1);
        let actor = w.actor;
        in_transition(&mut w.e, actor, |e| {
            in_copy(e, |e| e.vcall(new, 0x12c, &args![0u32]));
        });
        assert_eq!(w.e.mem.u32(new + 0x40), 0);
        assert_eq!(w.e.mem.u32(new + 0x1a8), u32::MAX);
        assert_eq!(calls_to(&w.e, NO_TARGET), vec![vec![new, 0]]);
        assert_eq!(calls_to(&w.e, SET_MARKER_RESERVED).len(), 2);
    }

    #[test]
    fn a_transition_inside_a_copy_is_not_a_copy() {
        let mut w = world(true, true, 0, false);
        let loaded_actor = w.e.mem.alloc(0x20);
        let actor = w.actor;
        in_transition(&mut w.e, actor, |e| {
            in_copy(e, |e| {
                assert!(copying_saved_fields());
                in_transition(e, loaded_actor, |_| assert!(!copying_saved_fields()));
                assert!(copying_saved_fields());
            })
        });
        assert!(!copying_saved_fields());
    }
}
