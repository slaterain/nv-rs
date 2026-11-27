//! The grid shift waits only for running cell loads (`NV_GRID_SHIFT_WAIT`), a fix of the `fixes`
//! preset ([`crate::fixes`]): **not 1:1 with FalloutNV.exe**; in the `vanilla` preset nothing here
//! runs.
//!
//! **What the original does.** When the player crosses into a new outdoor cell,
//! `TES::UpdateCurrentGridCell` (`00452580`) settles the exterior cell loader's queued cells (the
//! row it loads ahead) before it shifts the grid: `00528110` post-processes every loader task that
//! has finished (state 5: its slot `0x14`), `ExteriorCellLoader::TryCancelTasks` (`005281f0`)
//! cancels the queued and finished ones and answers how many refused because a loader thread is
//! running them (states 3 and 4), and when any refused, `005282d0` (`00452b4b`) walks the loader's
//! tasks again. For each it waits in `005289b0`, a Sleep(0) loop while the task's state is 3 or 5,
//! then post-processes it if its state is 5 (`005287e0`). A loader thread moves a task it runs from
//! 3 (the load, the task's slot 4 through `00c3fc80`) to 4 (handing it to the post-process queue,
//! `00c3fca0`) to 5; nothing but main moves a finished task on (its post-process is main's, in the
//! walk itself or in `IOManager`'s queue, `00c3dbf0`), and main is the thread waiting. So a task
//! that reaches 5 before the wait looks at it, or between two of its looks, is waited on for good:
//! main spins in `005289b0` and the game freezes with every other thread idle. That needs a cell
//! load still running at a grid shift, which takes moving across cells faster than the loader
//! keeps up (the maintainer's speed-20000 flight froze in 1 of about 4 runs: the frame log stopped,
//! and stacks taken of the frozen process show main in `005289b0`'s loop under `005282d0` and
//! `UpdateCurrentGridCell`, using one processor, every game thread waiting for work). The exe's
//! own code is the same (`005289b0`: `CMP [task+0xc],3`, `JZ` to the sleep, `CMP [task+0xc],5`,
//! `JNZ` out); the post-process test right after the wait shows the wait was meant for the
//! running states.
//!
//! **What this does.** Registered over `005282d0`: the same walk with the same calls
//! ([`fn_005282d0_waiting_while_running`]), whose wait for each task sleeps while its state is 3
//! or 4, the two states a loader thread moves on, and never on 5. A finished task then goes
//! straight to the walk's own post-process, as one that finished during the wait does in the
//! original; a task still loading is waited for as before, now until it has finished, and then
//! post-processed there too (the exe's wait ended at 4 and left it to `IOManager`'s queue on a
//! later frame, where it is post-processed the same way). `005282d0` has no other caller.

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::fallout_shared::exteriorcellloader::fn_005282d0_waiting_while_running;

/// The exterior cell loader's walk over the tasks `TryCancelTasks` left (`005282d0`).
pub const FINISH_LEFT_TASKS: u32 = 0x0052_82d0;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "grid shift waits only for running cell loads",
    env: "NV_GRID_SHIFT_WAIT",
    addresses: &[FINISH_LEFT_TASKS],
    summary: "crossing into new outdoor cells as a cell load finishes (moving very fast) no longer freezes the game: the grid shift waits only for cell loads still running and finishes a finished one at once, instead of waiting for it to finish itself",
    install,
};

/// Registers the row at [`FINISH_LEFT_TASKS`].
pub fn install(e: &mut Engine) {
    let (at, f) = engine::entry!(0x0052_82d0, fn_005282d0_waiting_while_running(Ptr));
    e.register(at, f);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// The loader (the walk's map), one task in it, the task's vtable and its post-process
    /// double (slot `0x14`), the IO manager global and a page for them.
    const PAGE: u32 = 0x0600_0000;
    const LOADER: u32 = 0x0600_0800;
    const TASK: u32 = 0x0600_0100;
    const TASK_VTABLE: u32 = 0x0600_0300;
    const POST_PROCESS: u32 = 0x0600_0400;
    const IO_MANAGER: u32 = 0x0120_2d98;
    const SLEEP: u32 = 0x0040_fca0;
    const TASK_STATE: u32 = 0x0c;

    struct Walk {
        e: Engine,
        sleeps: Rc<Cell<u32>>,
        posts: Rc<Cell<u32>>,
    }

    /// The walk's map holds one task in `state`; its iteration, holder and release are doubles
    /// (the walk's end test answers false once, the removal hands out the task), the state test
    /// is the translation, and Sleep counts and cancels the task (state 6) at its 1000th call so
    /// that the exe's endless wait ends in a test. `row` installs the row; without it the
    /// translation is at `005282d0`.
    fn walk(row: bool, state: u32) -> Walk {
        let mut e = Engine::new();
        for page in [0x0120_2000, PAGE] {
            e.map(page, 0x1000);
        }
        e.mem.set_u32(IO_MANAGER, 0x0600_0200);
        let ends = Rc::new(Cell::new(0u32));
        e.register_double(0x006e_bde0, move |_, _| {
            ends.set(ends.get() + 1);
            (ends.get() > 1).into_ret()
        });
        for leaf in [0x0052_8cb0, 0x0044_cbf0] {
            e.register_double(leaf, |_, _| Ret::default());
        }
        e.register_double(0x0090_6570, |e, a| {
            e.mem.set_u32(a[3], TASK);
            1u32.into_ret()
        });
        e.register_double(0x0055_9450, |e, a| e.mem.u32(a[0]).into_ret());
        let (at, f) = engine::entry!(
            0x0052_87e0,
            engine::units::fallout_shared::exteriorcellloader::fn_005287e0(Ptr) -> bool
        );
        e.register(at, f);
        e.put_vtable(TASK_VTABLE, &[0, 0, 0, 0, 0, POST_PROCESS]);
        e.mem.set_u32(TASK, TASK_VTABLE);
        e.mem.set_u32(TASK + TASK_STATE, state);
        let posts = Rc::new(Cell::new(0u32));
        let seen = posts.clone();
        e.register_double(POST_PROCESS, move |_, _| {
            seen.set(seen.get() + 1);
            Ret::default()
        });
        let sleeps = Rc::new(Cell::new(0u32));
        let slept = sleeps.clone();
        e.register_double(SLEEP, move |e, _| {
            slept.set(slept.get() + 1);
            if slept.get() >= 1000 {
                e.mem.set_u32(TASK + TASK_STATE, 6);
            }
            Ret::default()
        });
        if row {
            install(&mut e);
        } else {
            let (at, f) = engine::entry!(
                0x0052_82d0,
                engine::units::fallout_shared::exteriorcellloader::fn_005282d0(Ptr)
            );
            e.register(at, f);
        }
        Walk { e, sleeps, posts }
    }

    fn run(w: &mut Walk) -> (u32, u32) {
        w.e.call(FINISH_LEFT_TASKS, &args![LOADER]);
        (w.sleeps.get(), w.posts.get())
    }

    #[test]
    fn with_the_row_a_finished_task_is_post_processed_without_waiting() {
        assert_eq!(run(&mut walk(true, 5)), (0, 1));
    }

    #[test]
    fn without_the_row_the_walk_waits_on_a_finished_task_as_in_the_original() {
        // The original never ends this wait; the test's Sleep cancels the task at 1000.
        assert_eq!(run(&mut walk(false, 5)), (1000, 0));
    }

    #[test]
    fn with_the_row_a_task_whose_load_never_ends_is_still_waited_for() {
        // State 3 stays (nothing moves it but the test's cancel at the 1000th sleep).
        assert_eq!(run(&mut walk(true, 3)), (1000, 0));
        assert_eq!(run(&mut walk(true, 4)), (1000, 0));
    }

    #[test]
    fn the_rows_name_and_address() {
        assert_eq!(FIX.addresses, &[0x0052_82d0]);
        assert_eq!(FIX.env, "NV_GRID_SHIFT_WAIT");
    }
}
