//! The cell-entry wait and the cells loaded ahead (`NV_CELL_ENTRY_WAIT`), a fix of the `fixes`
//! preset ([`crate::fixes`], docs/FIXES.md): **not 1:1 with FalloutNV.exe**; in the `vanilla`
//! preset nothing here runs.
//!
//! **What the original does.** When the player's parent cell changes while the model loader
//! (`011c3b3c`) still has queued references of the new cell (`00447950`: the cell's count at
//! `+0xa0` is positive), `PlayerCharacter::Update` (`0094391c`..`00943964`) re-prioritises every
//! queued reference (`00446b50`: the entered cell's 3D-critical ones to priority 0, the others to
//! `TES::GetCellPriority`, 1 near and 3 far) and then waits in the IO manager for priority 0
//! (`00c3dfa0`). In the same crossing `TES::UpdateCurrentGridCell` has queued the row of cells
//! three cells ahead of the cell just left (`004528ba`, `00452920`, `0045294a`, through
//! `005283c0`): a pre-load into the cell buffer, which the grid itself only needs at its next
//! shift (and there it cancels whatever is still queued, `005281f0`, and loads what is missing
//! itself). The exterior cell loader queues each of those tasks at priority 0 (its constructor
//! `00527c00` gives `00440540` a 0), and the walk only knows the model loader's references. So
//! the wait for the entered cell also waits for the whole row, loaded one cell after another on
//! the one loader thread, and in the key order of the priority-0 bucket the entered cell's own
//! work stands behind it: 45 ms in the base game, about 150 ms with large plugins, for 1.5 ms of
//! the entered cell's own loads (measure/smooth-200/BORDERS.md, runs fd-dg1 and fd-dg2).
//!
//! **What this does.** Registered over the walk (`00446b50`). With a cell (the cell entry; the
//! walk's other caller, `0094cfd6`, passes none and waits for nothing) it runs the translation's
//! walk and then walks the exterior cell loader's own tasks (the loader at `011c9618` is their
//! map; the same iteration `00528110` and `005282d0` make) the way the walk treats references:
//! a task whose priority byte (`0043cc60`) differs from the wanted one is requeued through its
//! own virtual (slot `0x1c`, `00440660`, `IOManager::RequeueTask` `00c3df40`). Wanted is the
//! priority of the task's cell, 1 or 3. A queued task has no cell yet (its data's cell word,
//! `+0xc`, is filled when it runs, `00528620`), so the priority is `TES::GetCellPriority`'s own
//! arithmetic (`00458ca8`..`00458db0`) on the task's coordinates ([`cell_priority_at`]; the
//! function itself needs a cell and keeps a one-cell cache, `011c3f00` and `011c3f04`, which the
//! row leaves as it is). A task at the entered cell's own coordinates is left at 0 and waited
//! for, as in the original. The manager's requeue refuses a task that is running or done
//! (`00c3e690`: state 3 and up), so only a task still queued moves; one already running when the
//! player crosses is waited for as before.
//!
//! The wait itself is the original's. It then covers the entered cell's priority-0 loads and
//! their post-processing, which is what the code after it was given before too; the row of cells
//! ahead loads right after, in the background, at the priority its references get.

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::fallout_shared::modelloader_p3::fn_00446b50;

/// The model loader's walk over its queued references at a cell entry.
pub const WALK: u32 = 0x0044_6b50;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "cell entry waits for its own cell",
    env: "NV_CELL_ENTRY_WAIT",
    addresses: &[WALK],
    summary: "crossing into a new outdoor cell no longer pauses for the row of cells loaded ahead: they load in the background, after what the cell entered needs",
    install,
};

/// The exterior cell loader (the pointer at `011c9618`; the object is the map of its tasks).
const CELL_LOADER: u32 = 0x011c_9618;
/// The loader map's walk, as `00528110` and `005282d0` make it: the iterator (`00528c70`, ended
/// by `00528800`), its done test, the next entry (key and counted pointer out), the counted
/// pointer's start and release, and the pointer's word.
const ITERATOR_CONSTRUCT: u32 = 0x0052_8c70;
const ITERATOR_DESTRUCT: u32 = 0x0052_8800;
const ITERATOR_IS_DONE: u32 = 0x006e_bde0;
const MAP_NEXT_ENTRY: u32 = 0x0090_6570;
const TASK_POINTER_CONSTRUCT: u32 = 0x0052_8cb0;
const TASK_POINTER_DESTRUCT: u32 = 0x0044_cbf0;
const TASK_POINTER_GET: u32 = 0x0055_9450;
/// An exterior cell task's data (`005285c0`): the cell's x at `+0`, its y at `+4`.
const TASK_DATA: u32 = 0x1c;
/// The priority byte of a task's key (`0043cc60`), and the task's requeue virtual (`00440660`
/// for an exterior cell task).
const TASK_PRIORITY: u32 = 0x0043_cc60;
const SLOT_REQUEUE: u32 = 0x1c;
/// A cell's interior flag and an exterior cell's coordinates (`TESObjectCELL::GetDataX` and
/// `GetDataY`, Xbox PDB).
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
/// `TES` (`011dea10`) and its grid centre (`iCurrentGridX`, `iCurrentGridY`, Xbox PDB).
const TES: u32 = 0x011d_ea10;
const GRID_X: u32 = 0x24;
const GRID_Y: u32 = 0x28;
/// The `uGridsToLoad` setting and the reader of a setting's value (`0043d4d0`).
const GRIDS_TO_LOAD: u32 = 0x011c_63cc;
const SETTING_VALUE: u32 = 0x0043_d4d0;
/// The C runtime's `abs` (`00ec7d40`).
const ABSOLUTE_VALUE: u32 = 0x00ec_7d40;

fn half_grid(e: &mut Engine) -> i32 {
    let value = e.call(SETTING_VALUE, &args![GRIDS_TO_LOAD]).u32();
    (e.mem.u32(value) >> 1) as i32
}

/// The priority `TES::GetCellPriority` (`00458be0`) gives an exterior cell at `x`, `y` when it
/// works it out (its branch for a cell with no position given, `00458ca8`..`00458db0`): the
/// cell's distance from the grid centre along each axis, the larger of the two as signed numbers
/// (`00458d21`: not the larger distance), its size times 10 plus 10 as a byte, 1 up to 20 and 3
/// above. Read with the same calls; the function's cache of the last cell asked is neither read
/// nor written.
pub fn cell_priority_at(e: &mut Engine, x: i32, y: i32) -> u32 {
    let tes = e.global::<u32>(TES);
    let half = half_grid(e);
    let grid_x = e.mem.u32(tes.wrapping_add(GRID_X)) as i32;
    let offset_x = x.wrapping_sub(grid_x.wrapping_sub(half));
    let half = half_grid(e);
    let grid_y = e.mem.u32(tes.wrapping_add(GRID_Y)) as i32;
    let offset_y = y.wrapping_sub(grid_y.wrapping_sub(half));
    let half = half_grid(e);
    let across = offset_x.wrapping_sub(half);
    let half = half_grid(e);
    let down = offset_y.wrapping_sub(half);
    let farther = if across > down { across } else { down };
    let distance = e.call(ABSOLUTE_VALUE, &args![farther]).i32();
    let scaled = distance.wrapping_mul(10).wrapping_add(10) as u8;
    if scaled > 0x14 {
        3
    } else {
        1
    }
}

/// Gives every exterior cell task but the entered cell's own the priority of its cell, through
/// the task's own requeue (module documentation).
pub fn requeue_cells_ahead(e: &mut Engine, entered: Ptr) {
    let loader = e.global::<u32>(CELL_LOADER);
    if loader == 0 {
        return;
    }
    // An interior cell has no place in the grid: no exterior task is its own.
    let own = if e.call(CELL_IS_INTERIOR, &args![entered]).bool() {
        None
    } else {
        let x = e.call(CELL_GET_DATA_X, &args![entered]).i32();
        let y = e.call(CELL_GET_DATA_Y, &args![entered]).i32();
        Some((x, y))
    };
    e.with_stack(16, |e, iterator| {
        e.call(ITERATOR_CONSTRUCT, &args![iterator]);
        while !e.call(ITERATOR_IS_DONE, &args![iterator]).bool() {
            e.with_stack(8, |e, item| {
                e.call(TASK_POINTER_CONSTRUCT, &args![item, 0u32]);
                let found = e
                    .call(
                        MAP_NEXT_ENTRY,
                        &args![loader, iterator, item.addr() + 4, item, 1u32],
                    )
                    .bool();
                if found {
                    let task = e.call(TASK_POINTER_GET, &args![item]).u32();
                    let data = e.mem.u32(task.wrapping_add(TASK_DATA));
                    let place = (e.mem.u32(data) as i32, e.mem.u32(data.wrapping_add(4)) as i32);
                    if Some(place) != own {
                        let wanted = cell_priority_at(e, place.0, place.1);
                        let current = u32::from(e.call(TASK_PRIORITY, &args![task]).u8());
                        if current != wanted {
                            e.vcall(task, SLOT_REQUEUE, &args![wanted]);
                        }
                    }
                }
                e.call(TASK_POINTER_DESTRUCT, &args![item]);
            });
        }
        e.call(ITERATOR_DESTRUCT, &args![iterator]);
    });
}

/// The walk with the row on: the translation's, then, at a cell entry, the cells ahead.
pub fn walk(e: &mut Engine, loader: Ptr, cell: Ptr) {
    fn_00446b50(e, loader, cell);
    if !cell.is_null() {
        requeue_cells_ahead(e, cell);
    }
}

/// Registers the walk.
pub fn install(e: &mut Engine) {
    e.register(WALK, |e, a| {
        walk(e, Ptr::new(a[0]), Ptr::new(a[1]));
        Ret::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// `ExteriorCellLoaderTask`'s vtable (`0102d028`) and its eight slots in the exe: the
    /// destructor, run, finish, cancel, description, post-process, the key maker and the requeue.
    const TASK_VTABLE: u32 = 0x0102_d028;
    const TASK_SLOTS: [u32; 8] = [
        0x0052_7c40,
        0x0052_7cb0,
        0x0052_7ce0,
        0x0052_7d80,
        0x0052_7df0,
        0x0052_7d00,
        0x0043_db70,
        0x0044_0660,
    ];
    const TASK_CONSTRUCT: u32 = 0x0052_7c00;
    const TASK_STATE: u32 = 0x0c;
    const TASK_KEY: u32 = 0x10;
    /// The IO manager (`01202d98`), its real members used here (the bucket of a key `00c3e3a0`,
    /// the add `00c3de80`) and the test's stand-ins for its map and its thread.
    const IO_MANAGER: u32 = 0x0120_2d98;
    const MANAGER_BUCKET: u32 = 0x00c3_e3a0;
    const MANAGER_ADD: u32 = 0x00c3_de80;
    const MANAGER_VTABLE: u32 = 0x00d0_0000;
    const TEST_INSERT: u32 = 0x00d8_0010;
    const TEST_REMOVE: u32 = 0x00d8_0014;
    const TEST_THREAD_OF: u32 = 0x00d8_0058;
    const SIGNAL_THREAD: u32 = 0x0044_2600;
    const MANAGER_COUNTS: u32 = 0x54;
    const MANAGER_TOTAL: u32 = 0x58;
    const MANAGER_MODE: u32 = 0x68;
    const GET_CELL_PRIORITY: u32 = 0x0045_8be0;
    const PRIORITY_CACHE: u32 = 0x011c_3f00;
    const PRIORITY_CACHE_CELL: u32 = 0x011c_3f04;
    /// A type-3 key's bucket is its priority times 7 (`00c3e3a0`).
    const BUCKET_OF_PRIORITY_1: u32 = 7;
    const BUCKET_OF_PRIORITY_3: u32 = 21;

    type Keys = Rc<RefCell<Vec<u64>>>;

    struct World {
        e: Box<Engine>,
        model_loader: u32,
        manager: u32,
        /// The keys the manager's map was asked to take out and to take in.
        removed: Keys,
        inserted: Keys,
        tasks: Rc<RefCell<Vec<u32>>>,
    }

    /// The game's globals, an IO manager whose map and thread are stand-ins (its bucket rule,
    /// add, requeue and counts are the translations), an exterior cell loader and a model loader
    /// whose maps hand out the test's tasks and nothing, the grid centre at (-10, 1) with
    /// `uGridsToLoad` 5. `fix` installs the row.
    fn world(fix: bool) -> World {
        let mut e = Box::new(Engine::new());
        e.map(0x011c_0000, 0x6_0000);
        e.put_vtable(TASK_VTABLE, &TASK_SLOTS);
        e.register(0x0040_1000, |e, a| e.mem.alloc(a[0]).into_ret());

        let manager = e.mem.alloc(0x84);
        let mut slots = [0u32; 0x17];
        slots[0x10 / 4] = TEST_INSERT;
        slots[0x14 / 4] = TEST_REMOVE;
        slots[0x24 / 4] = MANAGER_BUCKET;
        slots[0x48 / 4] = MANAGER_ADD;
        slots[0x58 / 4] = TEST_THREAD_OF;
        e.put_vtable(MANAGER_VTABLE, &slots);
        e.mem.set_u32(manager, MANAGER_VTABLE);
        let counts = e.mem.alloc(4 * 0x78);
        e.mem.set_u32(manager + MANAGER_COUNTS, counts);
        let threads = e.mem.alloc(4);
        e.mem.set_u32(manager + 0x50, threads);
        e.mem.set_u32(manager + MANAGER_MODE, 6);
        e.mem.set_u32(IO_MANAGER, manager);
        let removed: Keys = Rc::default();
        let inserted: Keys = Rc::default();
        let log = removed.clone();
        e.register_double(TEST_REMOVE, move |_, a| {
            log.borrow_mut().push(u64::from(a[1]) | u64::from(a[2]) << 32);
            1u32.into_ret()
        });
        let log = inserted.clone();
        e.register_double(TEST_INSERT, move |_, a| {
            log.borrow_mut().push(u64::from(a[1]) | u64::from(a[2]) << 32);
            1u32.into_ret()
        });
        e.register(TEST_THREAD_OF, |_, _| 0u32.into_ret());
        e.register(SIGNAL_THREAD, |_, _| Ret::default());

        let tes = e.mem.alloc(0x100);
        e.mem.set_u32(tes + GRID_X, -10i32 as u32);
        e.mem.set_u32(tes + GRID_Y, 1);
        e.mem.set_u32(TES, tes);
        e.mem.set_u32(GRIDS_TO_LOAD + 4, 5);

        // The two maps: the cell loader's hands out the tasks, each with a count taken as the
        // real map's iteration takes one; the model loader's has no queued reference.
        let cell_loader = e.mem.alloc(0x40);
        e.mem.set_u32(CELL_LOADER, cell_loader);
        let model_loader = e.mem.alloc(0x10);
        let references = e.mem.alloc(0x40);
        e.mem.set_u32(model_loader + 8, references);
        let tasks: Rc<RefCell<Vec<u32>>> = Rc::default();
        let handed = Rc::new(Cell::new(0usize));
        let list = tasks.clone();
        e.register_double(MAP_NEXT_ENTRY, move |e, a| {
            let next = if a[0] == cell_loader {
                list.borrow().get(handed.get()).copied()
            } else {
                None
            };
            let Some(task) = next else {
                handed.set(0);
                let flags = e.mem.u8(a[1] + 0x0c);
                e.mem.set_u8(a[1] + 0x0c, flags | 2);
                return 0u32.into_ret();
            };
            handed.set(handed.get() + 1);
            let data = e.mem.u32(task + TASK_DATA);
            let key = e.mem.u32(data) << 16 | e.mem.u32(data + 4) & 0xffff;
            e.mem.set_u32(a[2], key);
            e.mem.set_u32(a[3], task);
            let count = e.mem.u32(task + 8);
            e.mem.set_u32(task + 8, count + 1);
            1u32.into_ret()
        });
        if fix {
            install(&mut e);
        }
        World {
            e,
            model_loader,
            manager,
            removed,
            inserted,
            tasks,
        }
    }

    impl World {
        /// An exterior cell task for the cell at `x`, `y`, made by the exe's constructor
        /// (priority 0), held by the loader's map and queued with the manager; then put into
        /// `state` (1 queued, 3 running, 5 done).
        fn task(&mut self, x: i32, y: i32, state: u32) -> u32 {
            let e = &mut self.e;
            let data = e.mem.alloc(0x14);
            e.mem.set_u32(data, x as u32);
            e.mem.set_u32(data + 4, y as u32);
            let task = e.mem.alloc(0x20);
            let loader = e.mem.u32(CELL_LOADER);
            e.call(TASK_CONSTRUCT, &args![task, loader, data]);
            // The map's own count of the task.
            e.mem.set_u32(task + 8, 1);
            e.vcall(self.manager, 0x48, &args![task]);
            assert_eq!(e.mem.u32(task + TASK_STATE), 1);
            e.mem.set_u32(task + TASK_STATE, state);
            self.tasks.borrow_mut().push(task);
            task
        }

        /// An exterior cell at `x`, `y`, or an interior cell.
        fn cell(&mut self, place: Option<(i32, i32)>) -> u32 {
            let e = &mut self.e;
            let cell = e.mem.alloc(0x100);
            match place {
                Some((x, y)) => {
                    let data = e.mem.alloc(8);
                    e.mem.set_u32(data, x as u32);
                    e.mem.set_u32(data + 4, y as u32);
                    e.mem.set_u32(cell + 0x48, data);
                }
                None => e.mem.set_u8(cell + 0x24, 1),
            }
            cell
        }

        fn enter(&mut self, cell: u32) {
            self.e.call(WALK, &args![self.model_loader, cell]);
        }

        fn key(&self, task: u32) -> u64 {
            self.e.mem.u64(task + TASK_KEY)
        }

        fn priority(&self, task: u32) -> u32 {
            (self.key(task) >> 16) as u32 & 0xff
        }

        fn state(&self, task: u32) -> u32 {
            self.e.mem.u32(task + TASK_STATE)
        }

        fn bucket(&self, index: u32) -> u32 {
            let counts = self.e.mem.u32(self.manager + MANAGER_COUNTS);
            self.e.mem.u32(counts + 4 * index)
        }
    }

    /// The player crosses west from (-10, 1) into (-11, 1): the row three cells ahead of the
    /// cell left is x = -13, y = -1..3, queued at priority 0.
    fn row(w: &mut World) -> Vec<u32> {
        (-1..=3).map(|y| w.task(-13, y, 1)).collect()
    }

    #[test]
    fn the_queued_cells_ahead_get_their_cells_priority() {
        let mut w = world(true);
        let row = row(&mut w);
        let own = w.task(-11, 1, 1);
        let running = w.task(-12, 1, 3);
        let done = w.task(-12, 2, 5);
        let before: Vec<u64> = w.tasks.borrow().iter().map(|t| w.key(*t)).collect();
        assert!(w.tasks.borrow().iter().all(|t| w.priority(*t) == 0));
        assert_eq!(w.bucket(0), 8);
        w.e.mem.set_u32(PRIORITY_CACHE, 0x1234);
        w.e.mem.set_u32(PRIORITY_CACHE_CELL, 0x5678);
        let queued_before = w.inserted.borrow().len();
        let entered = w.cell(Some((-11, 1)));

        w.enter(entered);

        // The row: 3 at its two ends and 1 between, as GetCellPriority answers for those cells
        // (the larger of -3 across and -2..2 down, signed); each still queued, under a new key.
        let after: Vec<u32> = row.iter().map(|t| w.priority(*t)).collect();
        assert_eq!(after, [3, 1, 1, 1, 3]);
        assert!(row.iter().all(|t| w.state(*t) == 1));
        // Taken out of the manager's map under the old keys, in the map's order, and put back.
        assert_eq!(*w.removed.borrow(), before[..5]);
        let requeued = w.inserted.borrow()[queued_before..].to_vec();
        let now: Vec<u64> = row.iter().map(|t| w.key(*t)).collect();
        assert_eq!(requeued, now);
        // The manager's counts follow: 5 left the priority-0 bucket the wait looks at.
        assert_eq!(w.bucket(0), 3);
        assert_eq!(w.bucket(BUCKET_OF_PRIORITY_1), 3);
        assert_eq!(w.bucket(BUCKET_OF_PRIORITY_3), 2);
        assert_eq!(w.e.mem.u32(w.manager + MANAGER_TOTAL), 8);
        // The entered cell's own task, a running one and a finished one: as they were.
        for (task, state, index) in [(own, 1, 5), (running, 3, 6), (done, 5, 7)] {
            assert_eq!(w.key(task), before[index]);
            assert_eq!(w.state(task), state);
        }
        // Every count taken while walking was given back.
        assert!(w.tasks.borrow().iter().all(|t| w.e.mem.u32(*t + 8) == 1));
        // GetCellPriority's cache was not touched.
        assert_eq!(w.e.mem.u32(PRIORITY_CACHE), 0x1234);
        assert_eq!(w.e.mem.u32(PRIORITY_CACHE_CELL), 0x5678);
    }

    /// Vanilla: the walk is the translation's, and no cell task moves.
    #[test]
    fn without_the_row_nothing_changes() {
        let mut w = world(false);
        row(&mut w);
        w.task(-11, 1, 1);
        w.task(-12, 1, 3);
        let before: Vec<u64> = w.tasks.borrow().iter().map(|t| w.key(*t)).collect();
        let queued_before = w.inserted.borrow().len();
        let entered = w.cell(Some((-11, 1)));
        w.enter(entered);
        let after: Vec<u64> = w.tasks.borrow().iter().map(|t| w.key(*t)).collect();
        assert_eq!(after, before);
        assert!(w.removed.borrow().is_empty());
        assert_eq!(w.inserted.borrow().len(), queued_before);
        assert_eq!(w.bucket(0), 7);
    }

    /// The walk's other caller (`0094cfd6`) passes no cell and waits for nothing: the cell tasks
    /// stay. An interior cell entered has no task of its own: every queued one gets its
    /// priority, one at (0, 0), which an interior's missing coordinates read as, included.
    #[test]
    fn no_cell_and_an_interior_cell() {
        let mut w = world(true);
        let row = row(&mut w);
        let origin = w.task(0, 0, 1);
        w.enter(0);
        assert!(w.removed.borrow().is_empty());
        assert!(w.tasks.borrow().iter().all(|t| w.priority(*t) == 0));

        let interior = w.cell(None);
        w.enter(interior);
        let after: Vec<u32> = row.iter().map(|t| w.priority(*t)).collect();
        assert_eq!(after, [3, 1, 1, 1, 3]);
        assert_eq!(w.priority(origin), 3);
        assert_eq!(w.removed.borrow().len(), 6);
    }

    /// A second entry finds the row at its priorities and requeues nothing; a task that then
    /// runs is left alone by a later entry that would want another priority.
    #[test]
    fn a_task_at_its_priority_or_running_is_not_requeued() {
        let mut w = world(true);
        let row = row(&mut w);
        let entered = w.cell(Some((-11, 1)));
        w.enter(entered);
        assert_eq!(w.removed.borrow().len(), 5);
        let keys: Vec<u64> = row.iter().map(|t| w.key(*t)).collect();
        w.enter(entered);
        assert_eq!(w.removed.borrow().len(), 5);
        // The loader thread takes the first (state 3); the grid centre moves into the row, at
        // (-13, 2), where every cell of it would be priority 1: the two ends want another.
        w.e.mem.set_u32(row[0] + TASK_STATE, 3);
        let tes = w.e.mem.u32(TES);
        w.e.mem.set_u32(tes + GRID_X, -13i32 as u32);
        w.e.mem.set_u32(tes + GRID_Y, 2);
        w.enter(entered);
        assert_eq!(w.key(row[0]), keys[0]);
        assert_eq!(w.state(row[0]), 3);
        assert_eq!(w.priority(row[4]), 1);
        assert_eq!(w.removed.borrow().len(), 6);
    }

    /// [`cell_priority_at`] is `TES::GetCellPriority`'s answer for a cell at those coordinates,
    /// for every cell around the centre and both grid sizes (the function's cache emptied before
    /// each question, so it works the answer out).
    #[test]
    fn the_priority_is_get_cell_prioritys() {
        let mut w = world(false);
        let tes = w.e.mem.u32(TES);
        for grid in [5u32, 7] {
            w.e.mem.set_u32(GRIDS_TO_LOAD + 4, grid);
            for x in -15..=-5 {
                for y in -5..=7 {
                    let cell = w.cell(Some((x, y)));
                    w.e.mem.set_u32(PRIORITY_CACHE_CELL, 0);
                    let games = w.e.call(GET_CELL_PRIORITY, &args![tes, cell, 0u32]).u32();
                    assert_eq!(cell_priority_at(&mut w.e, x, y), games, "{x}, {y}");
                }
            }
        }
        // Far enough for the byte to wrap (25 cells: 260 as a byte is 4), as the game's does.
        let cell = w.cell(Some((15, 1)));
        w.e.mem.set_u32(PRIORITY_CACHE_CELL, 0);
        let games = w.e.call(GET_CELL_PRIORITY, &args![tes, cell, 0u32]).u32();
        assert_eq!((cell_priority_at(&mut w.e, 15, 1), games), (1, 1));
    }
}
