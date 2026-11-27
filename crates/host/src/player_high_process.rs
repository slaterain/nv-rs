//! The player keeps the high process (`NV_PLAYER_HIGH_PROCESS`), a fix of the `fixes` preset
//! ([`crate::fixes`]): **not 1:1 with FalloutNV.exe**; in the `vanilla` preset nothing here runs.
//!
//! **What the original does.** Every actor's AI runs in one of four process levels: high (a
//! `HighProcess`, 0x46c bytes), middle-high (a `MiddleHighProcess`, 0x25c bytes), middle-low and
//! low. `MobileObject::GetDesiredProcessLevel` (`009334b0`) picks the level from the actor's cell:
//! high in a cell that is attached (`TES::IsCellLoaded(cell, 1)`, state 5 or 6), middle-high in one
//! that is loaded but not attached yet (`IsCellLoaded(cell, 0)` only, states 2 to 4), lower
//! further out. It asks the same of the player. When the player enters a cell faster than the
//! background loads attach it (flying or running very fast across cells), `TESObjectCELL::
//! AddReference` (`00548230`) queues the player on the process lists' temporary change list
//! (`ProcessLists::AddActorToTempChangeList`, `0096e870`, which asks `009334b0`: 1), the same
//! frame's `ProcessLists::ChangeProcessLevelTempList` (`0096eb40`) calls the player's
//! `UpdateProcessLevel` (`008860d0`, Actor's, kept by `PlayerCharacter`'s vtable), and that moves
//! the player to the middle-high process (`00883800`: a new 0x25c-byte process, the high one
//! deleted). Nothing in the exe keeps the player out of it. The same frame's after-draw detection
//! merge (`0096c710`) then merges the player's detection lists (`008f63e0`), which reads the thread
//! list at `+0x268`: past the end of a middle-high process, a word of whatever block lies after it,
//! taken as a list node and read through. Recorded in the original on the maintainer's speed-20000
//! flight (2026-11-26): the player's one entry into a cell in state 3 among 127 entries, the
//! desired level 1, `00883800` for the player, then `008f63e0` on its new process reading
//! `0x80187ffe` at `+0x268`, and the original ended with an access violation inside it. The port
//! stopped in the same place (the maintainer's crash of 2026-11-26, a float read as a pointer).
//!
//! **What this does.** Registered over `009334b0`: for the player it answers 0 (the high
//! process) and runs nothing else; every other actor gets the translation's answer. Every caller
//! that reaches the function by address then agrees that the player belongs in the high process:
//! `AddActorToTempChangeList` no longer queues the player for a level change (its current level
//! is the desired one), `UpdateProcessLevel` returns before any change, and `MoveRefToNewSpace`'s
//! unload branch (desired 3) never picks the player. The player is never in the process lists'
//! array, so their own level updates never reach it. Besides the cell that is loaded but not
//! attached, the original also answers lower for the player with no 3D outside a save load and
//! with no cell at all; the row answers high there too. Each of those could only move the player
//! down, which the game does not survive. The three direct calls inside `mobileobject.rs`
//! (`00932b70` and `00932d60`, run while a mobile object is loaded from a save) do not come
//! through here and keep the translation's answer; the last of them runs for the player on every
//! save load, where the original answers high (saves load fine in the original).

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::fallout_ai::mobileobject::{
    mobile_object_get_desired_process_level, MobileObject,
};

/// `MobileObject::GetDesiredProcessLevel` (Xbox PDB).
pub const GET_DESIRED_PROCESS_LEVEL: u32 = 0x0093_34b0;
/// The player (the `PlayerCharacter` pointer).
const PLAYER: u32 = 0x011d_ea3c;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "the player keeps the high process",
    env: "NV_PLAYER_HIGH_PROCESS",
    addresses: &[GET_DESIRED_PROCESS_LEVEL],
    summary: "the player entering a cell that is loaded but not attached yet (moving very fast across cells) stays in the high AI process instead of being moved to the middle-high one, which crashed the game",
    install,
};

/// Registers the row at [`GET_DESIRED_PROCESS_LEVEL`].
pub fn install(e: &mut Engine) {
    let (at, f) = engine::entry!(0x0093_34b0, desired_process_level(Ptr<MobileObject>) -> i32);
    e.register(at, f);
}

/// [`GET_DESIRED_PROCESS_LEVEL`] with the row: 0 (the high process) for the player, the
/// translation's answer for everyone else.
pub fn desired_process_level(e: &mut Engine, this: Ptr<MobileObject>) -> i32 {
    let player = e.global::<u32>(PLAYER);
    if player != 0 && this.addr() == player {
        return 0;
    }
    mobile_object_get_desired_process_level(e, this)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The test player, another actor in the same cell, their process and the vtables (every
    /// slot an address nothing is registered at, which answers 0, but the actors' `+0x24c`).
    const THE_PLAYER: u32 = 0x0010_0000;
    const OTHER_ACTOR: u32 = 0x0010_1000;
    const PROCESS: u32 = 0x0010_2000;
    const ACTOR_VTABLE: u32 = 0x0010_3000;
    const PROCESS_VTABLE: u32 = 0x0010_4000;
    const OPEN_SLOTS: u32 = 0x0150_0000;
    const CELL: u32 = 0x0020_0000;
    const TES: u32 = 0x0021_0000;
    /// `Actor::UpdateProcessLevel`, the move to the middle-high process (the actor's vtable
    /// `+0x24c`), and the leaves `009334b0` asks.
    const UPDATE_PROCESS_LEVEL: u32 = 0x0088_60d0;
    const MOVE_TO_MIDDLE_HIGH: u32 = 0x0088_3800;
    const SLOT_MOVE_TO_MIDDLE_HIGH: u32 = 0x24c;
    /// `009334b0`'s first test (true: no sleep check), the actor's cell, and its first "active"
    /// test (true: the cell tests decide).
    const SKIP_SLEEP_CHECK: u32 = 0x0047_c850;
    const ACTOR_CELL: u32 = 0x008d_6f30;
    const ACTIVE_TEST: u32 = 0x0042_ce10;
    const IS_CELL_LOADED: u32 = 0x0045_11e0;
    /// The process's level word (`0045cd60` reads `+0x28`): 0 high.
    const PROCESS_LEVEL: u32 = 0x28;
    const ACTOR_PROCESS: u32 = 0x68;

    struct World {
        e: Box<Engine>,
        moves: Rc<RefCell<Vec<u32>>>,
    }

    /// The player and another actor, both in a high process, in a cell that is loaded but not
    /// attached (`TES::IsCellLoaded(cell, 1)` false, `(cell, 0)` true); `row` installs the row.
    fn world(row: bool) -> World {
        let mut e = Box::new(Engine::new());
        e.on_open = Some(Rc::new(RefCell::new(
            |_: &mut Engine, _: u32, _: &[u32]| Some(Ret::default()),
        )));
        for global in [0x011d_e45c, PLAYER, 0x011d_df38, 0x011d_ea10, 0x011c_3b3c] {
            e.map(global, 4);
        }
        e.mem.set_u32(PLAYER, THE_PLAYER);
        e.mem.set_u32(0x011d_ea10, TES);
        for actor in [THE_PLAYER, OTHER_ACTOR] {
            e.map(actor, 0x100);
            e.mem.set_u32(actor, ACTOR_VTABLE);
            e.mem.set_u32(actor + ACTOR_PROCESS, PROCESS);
        }
        e.map(PROCESS, 0x100);
        e.mem.set_u32(PROCESS, PROCESS_VTABLE);
        e.mem.set_u32(PROCESS + PROCESS_LEVEL, 0);
        for vtable in [ACTOR_VTABLE, PROCESS_VTABLE] {
            e.map(vtable, 0x800);
            for slot in (0..0x800).step_by(4) {
                e.mem.set_u32(vtable + slot, OPEN_SLOTS + slot);
            }
        }
        e.mem
            .set_u32(ACTOR_VTABLE + SLOT_MOVE_TO_MIDDLE_HIGH, MOVE_TO_MIDDLE_HIGH);
        e.register_double(SKIP_SLEEP_CHECK, |_, _| 1u32.into_ret());
        e.register_double(ACTOR_CELL, |_, _| CELL.into_ret());
        e.register_double(ACTIVE_TEST, |_, _| 1u32.into_ret());
        e.register_double(IS_CELL_LOADED, |_, args| {
            assert_eq!(&args[..2], &[TES, CELL]);
            (args[2] == 0).into_ret()
        });
        let moves: Rc<RefCell<Vec<u32>>> = Rc::default();
        let seen = moves.clone();
        e.register_double(MOVE_TO_MIDDLE_HIGH, move |_, args| {
            seen.borrow_mut().push(args[0]);
            1u32.into_ret()
        });
        if row {
            install(&mut e);
        }
        World { e, moves }
    }

    fn desired(w: &mut World, actor: u32) -> u32 {
        w.e.call(GET_DESIRED_PROCESS_LEVEL, &args![actor]).u32()
    }

    #[test]
    fn with_the_row_the_player_in_a_loaded_cell_not_attached_keeps_the_high_process() {
        let mut w = world(true);
        assert_eq!(desired(&mut w, THE_PLAYER), 0);
        assert!(w.e.call(UPDATE_PROCESS_LEVEL, &args![THE_PLAYER]).bool());
        assert!(
            w.moves.borrow().is_empty(),
            "no move to the middle-high process"
        );
    }

    #[test]
    fn without_the_row_the_player_is_moved_to_the_middle_high_process_as_in_the_original() {
        let mut w = world(false);
        assert_eq!(desired(&mut w, THE_PLAYER), 1);
        assert!(w.e.call(UPDATE_PROCESS_LEVEL, &args![THE_PLAYER]).bool());
        assert_eq!(*w.moves.borrow(), vec![THE_PLAYER]);
    }

    #[test]
    fn with_the_row_other_actors_still_get_the_translations_level() {
        let mut w = world(true);
        assert_eq!(desired(&mut w, OTHER_ACTOR), 1);
        assert!(w.e.call(UPDATE_PROCESS_LEVEL, &args![OTHER_ACTOR]).bool());
        assert_eq!(*w.moves.borrow(), vec![OTHER_ACTOR]);
    }

    #[test]
    fn with_no_player_yet_nothing_is_taken_for_the_player() {
        let mut w = world(true);
        w.e.mem.set_u32(PLAYER, 0);
        assert_eq!(desired(&mut w, THE_PLAYER), 1);
        assert_eq!(desired(&mut w, 0), 1, "a null reference is not the player");
    }
}
