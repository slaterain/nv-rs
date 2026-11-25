//! The differential test of every typed function of the cell references family
//! (`engine_typed::cell_refs`) against its translation.
//!
//! The world of `0054bcf0`: a cell whose list of references is built from a seed: none to seven
//! references (now and then a null item), each with a base form (none, one of the two marker forms,
//! or an ordinary one of type 0x23, 0x2a, 0x1e or 0x15 with or without its flag `0x40000000`), form
//! flags `0x800` / `0x20` or none, an extra list whose `ScriptLocals` are absent, fresh or already
//! built, a 3D or none, and one of two vtables: `TESObjectREFR`'s own slots (the exe functions at
//! 0x100, 0x224 and 0x1d0, so the typed side's direct branches run) or stand-ins that answer words
//! kept in the reference. The seed also sets the game loader's word (0 to 4, and 6: its bit 1 is
//! the flag), whether `TES` accepts the cell and whether the cell has a 3D.
//!
//! What stands in: the work the pass hands on (`Load3D`, the cell's scripts and the steps after
//! them, the lock, `TES`'s test and priority, `SetActionFlag`, the cast, the process lists, the
//! attach, the cell's state) and `Script::InitActionList`'s callees. The model loader's queue
//! (`00444850`, this family's) runs as itself on a loader with two stand-in maps, its own callees
//! standing in as in its test below.
//!
//! The finish of a task on main (`0043fdf0` and the attaches it reaches, `00440ba0`) runs on a task,
//! a reference and its cells from a seed, the cell's lock standing in; `TES`'s load runs as itself
//! and hands the reference to a stand-in slot.
//!
//! `TES`'s load of a reference into the scene (`00451ef0`) runs on a reference, its base form, its
//! cell and the source task from a seed, the work it hands on standing in.
//!
//! The reference ID's walk of the scene graph (`004b6dc0`) runs on a tree from a seed (the exe's
//! node and geometry slots and stand-ins), and `NiAVObject::GetProperty` (`00a59d30`) on a
//! geometry's list of properties from a seed.
//!
//! The shadow scene node's add of an object and its lighting update (`00b5eeb0`, `00b5d9f0`) and
//! the add at the tail of a list of counted pointers (`00a5a310`) run on a shadow scene node, its
//! queues, their lock and an object from a seed, the list node pool standing in.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::cell_refs::{self, CELL_REFS};

/// Where the stand-in slots are registered: `VT_FN + slot`.
const VT_FN: u32 = 0x00ff_4000;
/// The answers the doubles give, at fixed words of a page of their own.
const CONTROL: u32 = 0x0300_0000;
const NODE_3D: u32 = CONTROL;
const SCRIPTS_ANSWER: u32 = CONTROL + 4;
const TES_ACCEPTS: u32 = CONTROL + 8;
const PRIORITY: u32 = CONTROL + 0xc;
/// What the script run manager's instance answers (never read).
const MANAGER: u32 = 0x0123_4560;

/// The globals the pass reads.
const TES_POINTER: u32 = 0x011d_ea10;
const SAVE_GAME_POINTER: u32 = 0x011d_e45c;
const GAME_LOADER_POINTER: u32 = 0x011d_df38;
const MODEL_LOADER_POINTER: u32 = 0x011c_3b3c;
const ROOM_MARKER_FORM: u32 = 0x011c_a238;
const MARKER_FORM: u32 = 0x011c_a230;

/// A reference's words the stand-in slots and the cast answer from.
const ANSWER_IS_ACTOR: u32 = 0x100;
const ANSWER_KIND: u32 = 0x104;
const ANSWER_3D: u32 = 0x108;
const ANSWER_CAST: u32 = 0x10c;

/// The callees that stand in.
const DOUBLES: [u32; 19] = [
    0x0054_5cf0, // TESObjectCELL::Load3D
    0x0054_c740, // the cell's scripts
    0x0054_df30, // the test after them
    0x0055_0c60, // its answer's step
    0x0045_3490, // TES's test of the cell
    0x0043_d410, // the vector constructor
    0x00a5_9c60, // the 3D's translation
    0x0054_bc10, // the accepted cell's pass
    0x0054_1ac0, // the cell's lock
    0x0054_1ae0,
    0x0045_8be0, // TES's priority for the cell
    0x005a_c750, // Script::SetActionFlag
    0x00ec_43fb, // __RTDynamicCast
    0x0096_f400, // ProcessLists::RemoveActorFromTempChangeList
    0x0054_8880, // TESObjectCELL::AttachReference3D
    0x0055_35f0, // the flagged cell's step
    0x0045_12a0, // the cell's state
    0x0041_8830, // the extra list's ScriptLocals
    0x005e_24d0, // the script run manager's instance
];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

/// A stand-in slot: the answer its object keeps at `AT`.
fn answer_of<const AT: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    ret(e.mem.u32(a[0] + AT))
}

/// The doubles that count their calls: double `K` counts at `COUNTS + 4 * K` and keeps the last
/// word it was handed (its first past `this` when it has one) at `COUNTS + 0x40 + 4 * K`, so what
/// it was handed shows in memory as well as in the call log.
const COUNTS: u32 = CONTROL + 0x100;
const QUEUED: u32 = 0;
const ATTACHED: u32 = 1;
const ACTION_FLAGS: u32 = 2;
const ACTOR_STEPS: u32 = 3;
const KIND_STEPS: u32 = 4;
const ACTION_LISTS: u32 = 5;

fn counted<const K: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let count = e.mem.u32(COUNTS + 4 * K);
    e.mem.set_u32(COUNTS + 4 * K, count + 1);
    e.mem
        .set_u32(COUNTS + 0x40 + 4 * K, a.get(1).copied().unwrap_or(a[0]));
    Ret::default()
}

/// The model loader's queue: counted, and each reference it was handed kept in order at
/// `QUEUE_LOG`.
const QUEUE_LOG: u32 = CONTROL + 0x400;

fn queue_reference(e: &mut Engine, a: &[u32]) -> Ret {
    let n = e.mem.u32(COUNTS + 4 * QUEUED);
    e.mem.set_u32(QUEUE_LOG + 4 * (n % 0x40), a[1]);
    counted::<QUEUED>(e, a)
}

fn stand_ins(e: &mut Engine) {
    e.map(CONTROL, 0x1000);
    for page in [0x011c_3000, 0x011c_a000, 0x011d_d000, 0x011d_e000] {
        e.map(page, 0x1000);
    }
    for addr in DOUBLES {
        e.register(addr, nothing);
    }
    e.register(0x0054_5cf0, |e, _| ret(e.mem.u32(NODE_3D)));
    e.register(0x0054_df30, |e, _| ret(e.mem.u32(SCRIPTS_ANSWER)));
    e.register(0x0045_3490, |e, _| ret(e.mem.u32(TES_ACCEPTS)));
    e.register(0x0045_8be0, |e, _| ret(e.mem.u32(PRIORITY)));
    e.register(0x00ec_43fb, |e, a| ret(e.mem.u32(a[0] + ANSWER_CAST)));
    e.register(0x0043_d410, |e, a| {
        for i in 0..3 {
            e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
        }
        ret(a[0])
    });
    e.register(0x0045_12a0, |e, a| {
        e.mem.set_u8(a[0] + 0x26, a[1] as u8);
        Ret::default()
    });
    e.register(0x0054_8880, counted::<ATTACHED>);
    e.register(0x005a_c750, counted::<ACTION_FLAGS>);
    // `ScriptLocals` of a list: the word after the list's own.
    e.register(0x0041_8830, |e, a| ret(e.mem.u32(a[0] + 4)));
    e.register(0x005e_24d0, |_, _| ret(MANAGER));
    // The manager's `InitActionList`: the locals get an action list.
    e.register(0x005e_2730, |e, a| {
        e.mem.set_u32(a[3] + 8, a[2]);
        counted::<ACTION_LISTS>(e, a)
    });
    e.register(VT_FN + 0x100, answer_of::<ANSWER_IS_ACTOR>);
    e.register(VT_FN + 0x224, answer_of::<ANSWER_KIND>);
    e.register(VT_FN + 0x1d0, answer_of::<ANSWER_3D>);
    e.register(VT_FN + 0x240, counted::<ACTOR_STEPS>);
    e.register(VT_FN + 0x260, counted::<KIND_STEPS>);
}

/// A small fixed sequence from the seed (a linear congruential generator).
struct Draw(u32);

impl Draw {
    fn next(&mut self, n: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.0 >> 16) % n
    }
}

/// A vtable: `TESObjectREFR`'s slots as the exe holds them, or every slot a stand-in.
fn vtable(e: &mut Engine, own: bool) -> u32 {
    let vt = e.mem.alloc(0x310);
    for slot in [0x100, 0x224, 0x1d0, 0x240, 0x260, 0x308] {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    if own {
        e.mem.set_u32(vt + 0x100, 0x0047_c850);
        e.mem.set_u32(vt + 0x224, 0x0057_25b0);
        e.mem.set_u32(vt + 0x1d0, 0x0043_fcd0);
    }
    vt
}

fn base_form(e: &mut Engine, kind: u8, flags: u32) -> u32 {
    let form = e.mem.alloc(0x50);
    e.mem.set_u8(form + 4, kind);
    e.mem.set_u32(form + 0x48, flags);
    form
}

/// One reference from the draw.
fn reference(e: &mut Engine, d: &mut Draw, vtables: [u32; 2], markers: [u32; 2]) -> u32 {
    let r = e.mem.alloc(0x120);
    let own = d.next(3) != 0;
    e.mem.set_u32(r, vtables[usize::from(own)]);
    e.mem
        .set_u8(r + 4, [0x3a, 0x3d, 0x40, 0x69, 0x3b][d.next(5) as usize]);
    e.mem
        .set_u32(r + 8, [0, 0, 0, 0x800, 0x20][d.next(5) as usize]);
    let has_3d = d.next(3) != 0;
    let base = match d.next(6) {
        0 if !has_3d => 0,
        1 => markers[0],
        2 => markers[1],
        _ => {
            let kind = [0x23, 0x23, 0x2a, 0x1e, 0x15][d.next(5) as usize];
            base_form(e, kind, [0, 0x4000_0000][d.next(2) as usize])
        }
    };
    e.mem.set_u32(r + 0x20, base);
    if base == 0 {
        // No base form: its form flag stops it before the queue, which reads the base form's type
        // (as the exe does: the game's references always have one).
        e.mem.set_u32(r + 8, 0x20);
    }
    // The extra list's `ScriptLocals`: none, fresh, or with an action list.
    let locals = match d.next(3) {
        0 => 0,
        n => {
            let l = e.mem.alloc(0x10);
            e.mem.set_u32(l, 0x0500_0000 + d.next(0x100));
            e.mem.set_u32(l + 8, n - 1);
            l
        }
    };
    e.mem.set_u32(r + 0x48, locals);
    let node = if has_3d { e.mem.alloc(0x10) } else { 0 };
    if own && node != 0 {
        let data = e.mem.alloc(0x20);
        e.mem.set_u32(data + 0x14, node);
        e.mem.set_u32(r + 0x64, data);
    }
    e.mem.set_u32(r + ANSWER_IS_ACTOR, d.next(3) / 2);
    e.mem.set_u32(r + ANSWER_KIND, d.next(2));
    e.mem.set_u32(r + ANSWER_3D, node);
    e.mem
        .set_u32(r + ANSWER_CAST, if d.next(4) == 0 { 0 } else { r });
    r
}

/// The world of a seed: its cell.
fn world(e: &mut Engine, seed: u32) -> u32 {
    stand_ins(e);
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ 0x5bd1);
    e.mem
        .set_u32(NODE_3D, if d.next(2) == 0 { 0 } else { 0x0700_0000 });
    e.mem.set_u32(SCRIPTS_ANSWER, 0xab00 + d.next(2));
    e.mem.set_u32(TES_ACCEPTS, u32::from(seed % 8 == 5));
    e.mem.set_u32(PRIORITY, 0x30 + d.next(4));
    let loader = e.mem.alloc(0x250);
    e.mem
        .set_u32(loader + 0x244, [0, 0, 1, 2, 3, 4, 6][d.next(7) as usize]);
    e.set_global(GAME_LOADER_POINTER, loader);
    e.set_global(TES_POINTER, 0x7000u32);
    e.set_global(SAVE_GAME_POINTER, 0x7100u32);
    // The model loader with two stand-in maps, so its queue (`00444850`) runs as itself.
    queue_doubles(e);
    let model_loader = e.mem.alloc(0x20);
    let queued = test_map(e, false, true);
    let later_map = test_map(e, false, true);
    e.mem.set_u32(model_loader + 8, queued);
    e.mem.set_u32(model_loader + 0xc, later_map);
    e.set_global(MODEL_LOADER_POINTER, model_loader);
    let markers = [base_form(e, 0x20, 0), base_form(e, 0x20, 0)];
    e.set_global(ROOM_MARKER_FORM, markers[0]);
    e.set_global(MARKER_FORM, markers[1]);
    let vtables = [vtable(e, false), vtable(e, true)];
    let cell = e.mem.alloc(0x100);
    let count = d.next(8);
    let mut node = cell + 0xac;
    for i in 0..count {
        let item = if d.next(9) == 0 {
            0
        } else {
            reference(e, &mut d, vtables, markers)
        };
        e.mem.set_u32(node, item);
        if i + 1 < count {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
    }
    cell
}

const SEEDS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
];

/// The pass against its translation, over the seeds' worlds and both values of `flag`.
#[test]
fn references_after_load() {
    Diff::new(entry!(
        0x0054_bcf0,
        cell_refs::references_after_load(u32, u8)
    ))
    .family(&CELL_REFS)
    .family(&leaf::LEAVES)
    .goes_by_address(&[0x0047_c850, 0x0043_fcd0])
    // The model loader's queue runs as itself: its other families' calls by address, its in-place
    // calls ignored.
    .goes_by_address(&[0x0040_b460, 0x008d_6f30])
    .ignore_calls(QUEUE_INLINED)
    // Read in place: the game loader's flag, the extra list, the form type, the base form's flag
    // test and its two steps, the cell's list.
    .ignore_calls(&[
        0x0042_ce10,
        0x005d_43c0,
        0x0040_1170,
        0x0045_2440,
        0x0045_2460,
        0x0062_4700,
        0x0096_04f0,
    ])
    .inputs(&[Input::Of(SEEDS), Input::Flag])
    .setup(|e, case| vec![world(e, case.word(0)), case.word(1)])
    .run()
    .assert_same();
}

/// The worlds the seeds make reach every branch the pass has, checked on the translation (so a
/// change to the generator that loses one shows here, not as a weaker comparison): a cell `TES`
/// accepts, references queued, attached, given their action list and flag, actors' and the other
/// kind's steps, a list whose last reference is brought up in both walks, a queued reference with a
/// 3D whose base form type is above 0x23 with the flag `0x40000000` (attached: only type 0x23 keeps
/// it out), and a game loader word of 4 or more with its bit 1 clear (the action lists set up).
#[test]
fn the_seeds_reach_every_branch() {
    let mut reached = [0u32; 6];
    let (mut accepted, mut twice, mut above_23, mut high_word) = (0, 0, 0, 0);
    for &seed in SEEDS {
        let mut e = Engine::new_untyped_for_test();
        let cell = world(&mut e, seed);
        // The pass's queue calls counted by a double here (the queue has its own test below).
        e.register(0x0044_4850, queue_reference);
        e.call(0x0054_bcf0, &args![cell, 0u32]);
        if e.mem.u32(TES_ACCEPTS) != 0 {
            accepted += 1;
            continue;
        }
        for (k, n) in reached.iter_mut().enumerate() {
            *n += e.mem.u32(COUNTS + 4 * k as u32);
        }
        // The last item of the list, ordinary, queued in both walks: the exe counts the pass before
        // it judges the item.
        let mut node = cell + 0xac;
        while e.mem.u32(node + 4) != 0 {
            node = e.mem.u32(node + 4);
        }
        let last = e.mem.u32(node);
        let queued = e.mem.u32(COUNTS + 4 * QUEUED);
        let times = (0..queued)
            .filter(|i| e.mem.u32(QUEUE_LOG + 4 * i) == last)
            .count();
        twice += u32::from(last != 0 && times == 2);
        for i in 0..queued {
            let item = e.mem.u32(QUEUE_LOG + 4 * i);
            let base = e.mem.u32(item + 0x20);
            above_23 += u32::from(
                base != 0
                    && e.mem.u8(base + 4) > 0x23
                    && e.mem.u32(base + 0x48) & 0x4000_0000 != 0
                    && e.mem.u32(item + ANSWER_3D) != 0,
            );
        }
        let word = e.mem.u32(e.global::<u32>(GAME_LOADER_POINTER) + 0x244);
        high_word +=
            u32::from(word >= 4 && word & 2 == 0 && e.mem.u32(COUNTS + 4 * ACTION_FLAGS) > 0);
    }
    assert!(accepted > 0, "no seed has TES accept the cell");
    assert!(
        above_23 > 0,
        "no seed queues a type above 0x23 with the flag and a 3D"
    );
    assert!(
        high_word > 0,
        "no seed has a game loader word of 4 or more with bit 1 clear"
    );
    assert!(
        twice > 0,
        "no seed ends its list with a reference queued in both walks"
    );
    for (k, n) in reached.iter().enumerate() {
        assert!(*n > 0, "no seed reaches double {k}: {reached:?}");
    }
}

/// `Script::InitActionList` against its translation: no list, no locals, locals with an action
/// list, fresh locals.
#[test]
fn init_action_list() {
    Diff::new(entry!(0x005a_c190, cell_refs::init_action_list(u32, u32)))
        .family(&CELL_REFS)
        .inputs(&[Input::Of(&[0x0600_0000, 0]), Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            stand_ins(e);
            let list = e.mem.alloc(0x10);
            let locals = match case.word(1) {
                0 => 0,
                n => {
                    let l = e.mem.alloc(0x10);
                    e.mem.set_u32(l, 0x0500_0000 + n);
                    e.mem.set_u32(l + 8, u32::from(n == 1));
                    l
                }
            };
            e.mem.set_u32(list + 4, locals);
            let list = if case.word(1) == 3 { 0 } else { list };
            vec![case.word(0), list]
        })
        .run()
        .assert_same();
}

/// `TESObjectREFR`'s slot 0x224 against its translation, every type byte.
#[test]
fn kind_3d_to_40_or_69() {
    const KINDS: &[u32] = &[
        0, 1, 0x1e, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x68, 0x69, 0x6a, 0x7f, 0x80,
        0xff,
    ];
    Diff::new(entry!(
        0x0057_25b0,
        cell_refs::kind_3d_to_40_or_69(u32) -> bool
    ))
    .family(&CELL_REFS)
    .ignore_calls(&[0x0040_1170])
    .inputs(&[Input::Of(KINDS)])
    .setup(|e, case| {
        let r = e.mem.alloc(0x10);
        e.mem.set_u32(r + 4, 0xaabb_cc00 | case.word(0));
        vec![r]
    })
    .run()
    .assert_same();
}

// The model loader's queue of a reference (`00444850`).
//
// Its world, from a seed: the game loader's word (its flags 2 and 8), the reference's base form
// type (0xd, one below it, the tree, NPC and creature types, two others) or the player, the
// reference's class (`TESObjectREFR`'s own slots 0x100 and 0x224, or stand-ins answering words
// kept in the reference), its word at +0xc8 (with bit 0 alone and a bit past the low word too), a
// cell or none; the loader's two maps with their slots the lock-free family's addresses or
// stand-ins (the same doubles behind both: a small table of keys and tasks), the reference already
// queued (its task's priority byte the same or not), the allocation refused, the add refused, the
// owner thread and menu mode. The priority is one of four (one above a byte), `later` both ways.

/// The queue's answers, on a page of their own.
const Q_CONTROL: u32 = 0x0310_0000;
const Q_ALLOC_FAILS: u32 = Q_CONTROL;
const Q_MENU_MODE: u32 = Q_CONTROL + 4;
const Q_SCOPE_DEPTH: u32 = Q_CONTROL + 8;
const Q_ALLOCATED: u32 = Q_CONTROL + 0xc;
/// What the current thread is, in the queue's world.
const Q_THREAD: u32 = 0x77;
/// Where the stand-in slots of the maps and the tasks are registered.
const MAP_VT_FN: u32 = 0x00ff_6000;
const TASK_VT_FN: u32 = 0x00ff_7000;
/// The player and the loader's owner thread object (globals).
const PLAYER_POINTER: u32 = 0x011d_ea3c;
const OWNER_THREAD_OBJECT: u32 = 0x011d_ea0c;

/// The test's map: its vtable, the number of keys, the add's answer, the last replace flag, then
/// up to eight (key, task) pairs.
const MAP_COUNT: u32 = 4;
const MAP_ADD_ANSWER: u32 = 8;
const MAP_LAST_FLAG: u32 = 0xc;
const MAP_PAIRS: u32 = 0x10;

/// The test's task: its vtable, count (+8), key (+0x10), then what its doubles keep.
const TASK_KIND: u32 = 0x2c;
const TASK_NEW_PRIORITY: u32 = 0x30;
const TASK_QUEUED: u32 = 0x34;
const TASK_DELETED: u32 = 0x38;

/// The map's find: the task of the key into the holder, counted, as the lock-free map's find does.
fn map_find(e: &mut Engine, a: &[u32]) -> Ret {
    let (map, key, holder) = (a[0], a[1], a[2]);
    for i in 0..e.mem.u32(map + MAP_COUNT) {
        if e.mem.u32(map + MAP_PAIRS + 8 * i) == key {
            let task = e.mem.u32(map + MAP_PAIRS + 8 * i + 4);
            e.mem.set_u32(holder, task);
            e.mem.set_u32(task + 8, e.mem.u32(task + 8) + 1);
            return ret(1);
        }
    }
    ret(0)
}

/// The map's add: refused when the map says so or the holder is empty; else the pair kept and the
/// task counted.
fn map_add(e: &mut Engine, a: &[u32]) -> Ret {
    let (map, key, holder, flag) = (a[0], a[1], a[2], a[3]);
    e.mem.set_u32(map + MAP_LAST_FLAG, 0x100 | flag);
    let task = e.mem.u32(holder);
    let n = e.mem.u32(map + MAP_COUNT);
    if e.mem.u32(map + MAP_ADD_ANSWER) == 0 || task == 0 || n == 8 {
        return ret(0);
    }
    e.mem.set_u32(map + MAP_PAIRS + 8 * n, key);
    e.mem.set_u32(map + MAP_PAIRS + 8 * n + 4, task);
    e.mem.set_u32(map + MAP_COUNT, n + 1);
    e.mem.set_u32(task + 8, e.mem.u32(task + 8) + 1);
    ret(1)
}

/// A task as a constructor leaves it: the task vtable, no count, the priority in its key's byte 2
/// (bytes 0, 1 and 3 other words, as a real key keeps: the priority's setter `004405d0` replaces
/// byte 2 alone).
fn new_task(e: &mut Engine, block: u32, kind: u32, reference: u32, priority: u32) {
    let vt = e.mem.alloc(0x40);
    for slot in [0, 0x1c, 0x20] {
        e.mem.set_u32(vt + slot, TASK_VT_FN + slot);
    }
    e.mem.set_u32(block, vt);
    e.mem.set_u32(block + 8, 0);
    e.mem
        .set_u32(block + 0x10, (priority & 0xff) << 16 | 0x5a00_1234);
    e.mem.set_u32(block + 0x14, 0x8000_0001);
    e.mem.set_u32(block + 0x28, reference);
    e.mem.set_u32(block + TASK_KIND, kind);
}

fn constructor<const KIND: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    // The player's takes the priority only.
    let (reference, priority) = if KIND == 0 { (0, a[1]) } else { (a[1], a[2]) };
    new_task(e, a[0], KIND, reference, priority);
    ret(a[0])
}

fn queue_stand_ins(e: &mut Engine) {
    for page in [0x011c_3000, 0x011d_d000, 0x011d_e000] {
        e.map(page, 0x1000);
    }
    queue_doubles(e);
}

/// The queue's doubles, and the page of their answers.
fn queue_doubles(e: &mut Engine) {
    e.map(Q_CONTROL, 0x1000);
    // The interlocked increment and decrement, plainly.
    e.register(0x0040_b460, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x0040_19a0, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    for addr in [0x0044_bd60, MAP_VT_FN + 8] {
        e.register(addr, map_find);
    }
    for addr in [0x0052_8b60, MAP_VT_FN + 0x10] {
        e.register(addr, map_add);
    }
    e.register(0x0044_1b20, constructor::<0>);
    e.register(0x0044_0e50, constructor::<1>);
    e.register(0x0044_1440, constructor::<2>);
    e.register(0x0044_1920, constructor::<3>);
    e.register(0x0043_fd40, constructor::<4>);
    // The memory manager: a block, or none when the world says so.
    e.register(0x0040_1000, |e, a| {
        if e.mem.u32(Q_ALLOC_FAILS) != 0 {
            return ret(0);
        }
        e.mem.set_u32(Q_ALLOCATED, e.mem.u32(Q_ALLOCATED) + a[0]);
        ret(e.mem.alloc(a[0]))
    });
    // The memory contexts: their depth kept, each scope given the context it entered.
    e.register(0x0040_4eb0, |e, a| {
        e.mem.set_u32(Q_SCOPE_DEPTH, e.mem.u32(Q_SCOPE_DEPTH) + 1);
        e.mem.set_u32(a[0], a[1]);
        ret(a[0])
    });
    e.register(0x0040_4ee0, |e, a| {
        e.mem.set_u32(Q_SCOPE_DEPTH, e.mem.u32(Q_SCOPE_DEPTH) - 1);
        e.mem.set_u32(a[0], 0xdead);
        Ret::default()
    });
    e.register(0x0040_fc90, |_, _| ret(Q_THREAD));
    e.register(0x0070_2360, |e, _| ret(e.mem.u32(Q_MENU_MODE)));
    e.register(TASK_VT_FN, |e, a| {
        e.mem.set_u32(a[0] + TASK_DELETED, 0x100 | a[1]);
        ret(a[0])
    });
    e.register(TASK_VT_FN + 0x1c, |e, a| {
        e.mem.set_u32(a[0] + TASK_NEW_PRIORITY, 0x100 | a[1]);
        Ret::default()
    });
    e.register(TASK_VT_FN + 0x20, |e, a| {
        let n = e.mem.u32(a[0] + TASK_QUEUED);
        e.mem.set_u32(a[0] + TASK_QUEUED, n + 1);
        Ret::default()
    });
    e.register(VT_FN + 0x100, answer_of::<ANSWER_IS_ACTOR>);
    e.register(VT_FN + 0x224, answer_of::<ANSWER_KIND>);
}

/// A map with the lock-free family's slots or stand-ins, and its add's answer.
fn test_map(e: &mut Engine, own: bool, accepts: bool) -> u32 {
    let vt = e.mem.alloc(0x40);
    let (find, add) = if own {
        (0x0044_bd60, 0x0052_8b60)
    } else {
        (MAP_VT_FN + 8, MAP_VT_FN + 0x10)
    };
    e.mem.set_u32(vt + 8, find);
    e.mem.set_u32(vt + 0x10, add);
    let map = e.mem.alloc(MAP_PAIRS + 8 * 8);
    e.mem.set_u32(map, vt);
    e.mem.set_u32(map + MAP_ADD_ANSWER, u32::from(accepts));
    map
}

/// The queue's world of a seed and `later`: the call's words (loader, reference, priority, later).
fn queue_world(e: &mut Engine, seed: u32, later: u32) -> Vec<u32> {
    queue_stand_ins(e);
    let mut d = Draw(seed.wrapping_mul(0x85eb_ca6b) ^ 0x2c1b);
    let game_loader = e.mem.alloc(0x250);
    e.mem
        .set_u32(game_loader + 0x244, [0, 1, 8, 10, 2, 6][d.next(6) as usize]);
    e.set_global(GAME_LOADER_POINTER, game_loader);
    let priority = [0, 0x32, 0xff, 0x132][d.next(4) as usize];
    // The reference.
    let r = e.mem.alloc(0x120);
    let vt = vtable(e, d.next(2) == 0);
    e.mem.set_u32(r, vt);
    e.mem
        .set_u8(r + 4, [0x3a, 0x3d, 0x40, 0x69][d.next(4) as usize]);
    let kind = [0xd, 0xc, 0x25, 0x2a, 0x2b, 0x1e, 0x15, 0x25, 0x2a][d.next(9) as usize];
    let base = base_form(e, kind, 0);
    e.mem.set_u32(r + 0x20, base);
    e.mem
        .set_u32(r + 0xc8, [0, 1, 2, 3, 4, 0x1_0002][d.next(6) as usize]);
    e.mem.set_u32(r + ANSWER_IS_ACTOR, d.next(2));
    e.mem.set_u32(r + ANSWER_KIND, d.next(2));
    let cell = if d.next(4) == 0 {
        0
    } else {
        e.mem.alloc(0x100)
    };
    e.mem.set_u32(r + 0x40, cell);
    let player = if d.next(6) == 0 { r } else { 0x7300 };
    e.set_global(PLAYER_POINTER, player);
    // The loader and its maps.
    let maps_own = d.next(2) == 0;
    let queued = test_map(e, maps_own, d.next(5) != 0);
    let later_map = test_map(e, maps_own, d.next(2) == 0);
    if d.next(4) == 0 {
        // Already queued: a task whose priority byte is the call's or not, counted by the map.
        let task = e.mem.alloc(0x48);
        let byte = if d.next(2) == 0 {
            priority
        } else {
            priority + 7
        };
        new_task(e, task, 9, r, byte);
        e.mem.set_u32(task + 8, 1);
        e.mem.set_u32(queued + MAP_PAIRS, r);
        e.mem.set_u32(queued + MAP_PAIRS + 4, task);
        e.mem.set_u32(queued + MAP_COUNT, 1);
    }
    let loader = e.mem.alloc(0x20);
    e.mem.set_u32(loader + 8, queued);
    e.mem.set_u32(loader + 0xc, later_map);
    e.mem.set_u32(Q_ALLOC_FAILS, u32::from(d.next(8) == 0));
    e.mem.set_u32(Q_MENU_MODE, u32::from(d.next(3) == 0));
    let owner = e.mem.alloc(0x20);
    let owner_thread = if d.next(2) == 0 { Q_THREAD } else { 0x78 };
    e.mem.set_u32(owner + 0x10, owner_thread);
    e.set_global(OWNER_THREAD_OBJECT, owner);
    vec![loader, r, priority, later]
}

/// The calls `00444850`'s typed version does in place.
const QUEUE_INLINED: &[u32] = &[
    0x0042_ce10, // the game loader's flag 2
    0x0040_1170, // the form type
    0x0052_8cb0, // the task pointer's set-up
    0x0055_9450, // the task the pointer holds
    0x0043_cc60, // the task's priority
];

/// The queue against its translation, over the seeds' worlds and both values of `later`.
#[test]
fn model_loader_queue_reference() {
    Diff::new(entry!(
        0x0044_4850,
        cell_refs::queue::queue_reference(u32, u32, u32, u8)
    ))
    .family(&CELL_REFS)
    .family(&leaf::LEAVES)
    // The other families' typed calls: the maps' find and add, the current thread, the interlocked
    // increment, the owner's thread word, the reference's cell, the class's false body.
    .goes_by_address(&[
        0x0044_bd60,
        0x0052_8b60,
        0x0040_fc90,
        0x0040_b460,
        0x0044_edb0,
        0x008d_6f30,
        0x0047_c850,
    ])
    .ignore_calls(QUEUE_INLINED)
    .inputs(&[Input::Of(SEEDS), Input::Flag])
    .setup(|e, case| queue_world(e, case.word(0), case.word(1)))
    .run()
    .assert_same();
}

/// The queue's worlds reach every branch it has, checked on the translation: each of the three
/// early ends, a task found with the same and with another priority, each kind's task, an
/// allocation refused, an add refused (the task deleted), the later map, a started task, a cell
/// counting a reference and a critical one.
#[test]
fn the_queue_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeMap::<&str, u32>::new();
    for &seed in SEEDS {
        for later in [0, 1] {
            let mut e = Engine::new_untyped_for_test();
            let words = queue_world(&mut e, seed, later);
            let (loader, r, priority) = (words[0], words[1], words[2]);
            let queued = e.mem.u32(loader + 8);
            let later_map = e.mem.u32(loader + 0xc);
            let before = e.mem.u32(queued + MAP_COUNT);
            let old = e.mem.u32(queued + MAP_PAIRS + 4);
            e.call(0x0044_4850, &args![loader, r, priority, later]);
            assert_eq!(e.mem.u32(Q_SCOPE_DEPTH), 0, "seed {seed}");
            let mut hit = |name| *reached.entry(name).or_default() += 1;
            let base = e.mem.u32(r + 0x20);
            let kind = if e.mem.u32(e.mem.u32(r) + 0x224) == 0x0057_25b0 {
                let k = e.mem.u8(r + 4);
                (0x3d..=0x40).contains(&k) || k == 0x69
            } else {
                e.mem.u32(r + ANSWER_KIND) != 0
            };
            if e.mem.u32(queued + MAP_LAST_FLAG) == 0 {
                // No add: an early end, or the reference found (its task's count back to the
                // map's alone).
                let flags = e.mem.u32(e.global::<u32>(GAME_LOADER_POINTER) + 0x244);
                hit(if flags & 2 != 0 && flags & 8 == 0 {
                    "loader flag"
                } else if e.mem.u8(base + 4) == 0xd {
                    "type 0xd"
                } else if kind && e.mem.u32(r + 0xc8) & 2 != 0 {
                    "slot 0x224 and +0xc8"
                } else {
                    assert_eq!((before, e.mem.u32(old + 8)), (1, 1), "seed {seed}");
                    if e.mem.u32(old + TASK_NEW_PRIORITY) != 0 {
                        "found, another priority"
                    } else {
                        "found, same priority"
                    }
                });
                continue;
            }
            // Past the early ends on their other side: a type below 0xd, and the bit at +0xc8
            // next to the one tested.
            if e.mem.u8(base + 4) == 0xc {
                hit("type 0xc queued");
            }
            if kind && e.mem.u32(r + 0xc8) == 1 {
                hit("slot 0x224, +0xc8 bit 0 alone, queued");
            }
            if e.mem.u32(Q_ALLOC_FAILS) != 0 {
                hit("allocation refused");
                continue;
            }
            let count = e.mem.u32(queued + MAP_COUNT);
            if count == before {
                hit("add refused");
                continue;
            }
            let task = e.mem.u32(queued + MAP_PAIRS + 8 * (count - 1) + 4);
            hit(["player", "tree", "npc", "creature", "other"]
                [e.mem.u32(task + TASK_KIND) as usize]);
            if e.mem.u32(later_map + MAP_LAST_FLAG) != 0 {
                hit("later map");
            } else if e.mem.u32(task + TASK_QUEUED) != 0 {
                hit("started");
            }
            let cell = e.mem.u32(r + 0x40);
            if cell != 0 && e.mem.u32(cell + 0xa4) == 1 {
                hit("cell counts");
                if e.mem.u32(cell + 0xa0) == 1 {
                    hit("critical");
                }
            }
        }
    }
    for name in [
        "loader flag",
        "type 0xd",
        "slot 0x224 and +0xc8",
        "found, same priority",
        "found, another priority",
        "allocation refused",
        "add refused",
        "player",
        "tree",
        "npc",
        "creature",
        "type 0xc queued",
        "slot 0x224, +0xc8 bit 0 alone, queued",
        "other",
        "later map",
        "started",
        "cell counts",
        "critical",
    ] {
        assert!(
            reached.contains_key(name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}

/// A task of the queue's doubles, its count `count`.
fn counted_task(e: &mut Engine, count: u32) -> u32 {
    let t = e.mem.alloc(0x48);
    new_task(e, t, 4, 0, 0x32);
    e.mem.set_u32(t + 8, count);
    t
}

/// The task pointer's assignment against its translation: the held task none, one or another, the
/// new one none, the same or another, the first task's count 1 (released to its deletion) or 2.
#[test]
fn task_pointer_assign() {
    Diff::new(entry!(
        0x006f_74f0,
        cell_refs::queue::task_pointer_assign(u32, u32) -> u32
    ))
    .family(&CELL_REFS)
    .goes_by_address(&[0x0040_b460])
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[1, 2]),
    ])
    .setup(|e, case| {
        queue_stand_ins(e);
        let first = counted_task(e, case.word(2));
        let second = counted_task(e, 1);
        let pick = |k: u32| [0, first, second][k as usize];
        let holder = e.mem.alloc(4);
        e.mem.set_u32(holder, pick(case.word(0)));
        vec![holder, pick(case.word(1))]
    })
    .run()
    .assert_same();
}

/// The task pointer's release against its translation: empty, or a task of count 1 (deleted) or 2.
#[test]
fn task_pointer_release() {
    Diff::new(entry!(
        0x0044_cbf0,
        cell_refs::queue::task_pointer_release(u32)
    ))
    .family(&CELL_REFS)
    .inputs(&[Input::Of(&[0, 1, 2])])
    .setup(|e, case| {
        queue_stand_ins(e);
        let task = match case.word(0) {
            0 => 0,
            n => counted_task(e, n),
        };
        let holder = e.mem.alloc(4);
        e.mem.set_u32(holder, task);
        vec![holder]
    })
    .run()
    .assert_same();
}

/// The count taken against its translation.
#[test]
fn add_ref() {
    Diff::new(entry!(0x0092_c870, cell_refs::queue::add_ref(u32)))
        .family(&CELL_REFS)
        .goes_by_address(&[0x0040_b460])
        .inputs(&[Input::Of(&[0, 1, 0xffff_ffff])])
        .setup(|e, case| {
            queue_stand_ins(e);
            vec![counted_task(e, case.word(0))]
        })
        .run()
        .assert_same();
}

/// The cell's two counters against their translations.
#[test]
fn count_queued() {
    Diff::new(entry!(0x0044_4cc0, cell_refs::queue::count_queued(u32)))
        .family(&CELL_REFS)
        .goes_by_address(&[0x0040_b460])
        .inputs(&[Input::Of(&[0, 5])])
        .setup(|e, case| {
            queue_stand_ins(e);
            let cell = e.mem.alloc(0x100);
            e.mem.set_u32(cell + 0xa0, case.word(0));
            e.mem.set_u32(cell + 0xa4, case.word(0) + 1);
            vec![cell]
        })
        .run()
        .assert_same();
}

#[test]
fn count_critical() {
    Diff::new(entry!(0x0044_4ce0, cell_refs::queue::count_critical(u32)))
        .family(&CELL_REFS)
        .goes_by_address(&[0x0040_b460])
        .inputs(&[Input::Of(&[0, 5])])
        .setup(|e, case| {
            queue_stand_ins(e);
            let cell = e.mem.alloc(0x100);
            e.mem.set_u32(cell + 0xa0, case.word(0));
            e.mem.set_u32(cell + 0xa4, case.word(0) + 1);
            vec![cell]
        })
        .run()
        .assert_same();
}

/// `TESObjectREFR::Is3DCritical` against its translation: the class's own slot 0x100 (the false
/// body), or a stand-in answering no or yes.
#[test]
fn is_3d_critical() {
    Diff::new(entry!(
        0x0057_2350,
        cell_refs::queue::is_3d_critical(u32) -> bool
    ))
    .family(&CELL_REFS)
    .goes_by_address(&[0x0047_c850])
    .inputs(&[Input::Of(&[0, 1, 2])])
    .setup(|e, case| {
        queue_stand_ins(e);
        let r = e.mem.alloc(0x120);
        let vt = vtable(e, case.word(0) == 0);
        e.mem.set_u32(r, vt);
        e.mem
            .set_u32(r + ANSWER_IS_ACTOR, u32::from(case.word(0) == 2));
        vec![r]
    })
    .run()
    .assert_same();
}

// The finish of a reference's task on main (`0043fdf0` and the attaches it reaches, `00440ba0`).
//
// Its world, from a seed: the task's state (6 ends it at once), its memory context and its model
// (none, one with a root node, one without); the reference's class (`TESObjectREFR`'s own slot
// 0x100, the false body, or a stand-in answering a word kept in the reference: its 3D critical or
// not), its form flags (none, 0x800, 0x20, both, others), its parent cell (none or one, the cell's
// load state 0 to 7, its two counts 0, 1 or 5, so the floor at 0 is reached); the loader's map of
// queued references (its slot 0x14 the lock-free family's removal or a stand-in); and whether
// `TES`'s load moves the reference into another cell (the cell read again after it is that one).

/// The attach's answers and what its doubles keep, on a page of their own.
const A_CONTROL: u32 = 0x0320_0000;
const A_LOCK_DEPTH: u32 = A_CONTROL;
const A_LOCKED: u32 = A_CONTROL + 4;
const A_UNLOCKED: u32 = A_CONTROL + 8;
const A_ADDON_NODES: u32 = A_CONTROL + 0xc;
const A_LOADED: u32 = A_CONTROL + 0x18;
const A_REMOVED: u32 = A_CONTROL + 0x2c;
const A_MOVE_TO: u32 = A_CONTROL + 0x34;
const A_ATTACHED_BY_SLOT: u32 = A_CONTROL + 0x38;

fn attach_stand_ins(e: &mut Engine) {
    queue_stand_ins(e);
    e.map(A_CONTROL, 0x1000);
    // The cell's lock: its depth kept, and the cells it was taken and let go for.
    e.register(0x0054_1ac0, |e, a| {
        e.mem.set_u32(A_LOCK_DEPTH, e.mem.u32(A_LOCK_DEPTH) + 1);
        e.mem.set_u32(A_LOCKED, a[0]);
        Ret::default()
    });
    e.register(0x0054_1ae0, |e, a| {
        e.mem
            .set_u32(A_LOCK_DEPTH, e.mem.u32(A_LOCK_DEPTH).wrapping_sub(1));
        e.mem.set_u32(A_UNLOCKED, a[0]);
        Ret::default()
    });
    // ModelLoader::LoadAddonNodes (loader, reference, node), counted.
    e.register(0x0044_89b0, |e, a| {
        e.mem.set_u32(A_ADDON_NODES, e.mem.u32(A_ADDON_NODES) + 1);
        e.mem.set_u32(A_ADDON_NODES + 4, a[1]);
        e.mem.set_u32(A_ADDON_NODES + 8, a[2]);
        Ret::default()
    });
    // TES's load of the reference (`00451ef0`, this family's) runs as itself: the reference's slot
    // 0x224 says so (its type 0x3d, or the stand-in's word), so the load hands it to its slot 0x308,
    // which counts and moves the reference into the world's other cell when the world says so.
    e.register(VT_FN + 0x308, |e, a| {
        e.mem.set_u32(A_LOADED, e.mem.u32(A_LOADED) + 1);
        e.mem.set_u32(A_LOADED + 4, a[0]);
        let to = e.mem.u32(A_MOVE_TO);
        if to != 0 {
            e.mem.set_u32(a[0] + 0x40, to);
        }
        Ret::default()
    });
    // The queued map's removal (by its address or a stand-in slot), counted with the key.
    for addr in [0x0090_62e0, MAP_VT_FN + 0x14] {
        e.register(addr, |e, a| {
            e.mem.set_u32(A_REMOVED, e.mem.u32(A_REMOVED) + 1);
            e.mem.set_u32(A_REMOVED + 4, a[1]);
            ret(1)
        });
    }
    // A task's attach slot that holds none of the exe's attaches.
    e.register(TASK_VT_FN + 0x3c, |e, a| {
        e.mem.set_u32(A_ATTACHED_BY_SLOT, a[0]);
        Ret::default()
    });
}

/// A cell of the draw: its load state and its two counts.
fn attach_cell(e: &mut Engine, d: &mut Draw) -> u32 {
    let cell = e.mem.alloc(0x100);
    e.mem.set_u8(cell + 0x26, d.next(8) as u8);
    e.mem.set_u32(cell + 0xa4, [0, 1, 5][d.next(3) as usize]);
    e.mem.set_u32(cell + 0xa0, [0, 1, 5][d.next(3) as usize]);
    cell
}

/// The attach's world of a seed, with the task's slot 0x3c given `attach`: the task.
fn attach_world(e: &mut Engine, seed: u32, attach: u32) -> u32 {
    attach_stand_ins(e);
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ 0x51ed);
    // The reference and its cells.
    let r = e.mem.alloc(0x120);
    let vt = vtable(e, d.next(2) == 0);
    e.mem.set_u32(r, vt);
    // `TES`'s load hands the reference to its slot 0x308: the type for the exe's slot 0x224, the
    // word for a stand-in's.
    e.mem.set_u8(r + 4, 0x3d);
    e.mem.set_u32(r + ANSWER_KIND, 1);
    e.mem.set_u32(
        r + 8,
        [0, 0x800, 0x20, 0x820, 0x10, 0x1000][d.next(6) as usize],
    );
    e.mem.set_u32(r + ANSWER_IS_ACTOR, d.next(2));
    let cell = if d.next(5) == 0 {
        0
    } else {
        attach_cell(e, &mut d)
    };
    e.mem.set_u32(r + 0x40, cell);
    if d.next(5) == 0 {
        let other = attach_cell(e, &mut d);
        e.mem.set_u32(A_MOVE_TO, other);
    }
    // The task.
    let task_vt = e.mem.alloc(0x48);
    e.mem.set_u32(task_vt + 0x3c, attach);
    let task = e.mem.alloc(0x48);
    e.mem.set_u32(task, task_vt);
    e.mem.set_u32(task + 8, 1);
    e.mem.set_u32(task + 0xc, [0, 5, 6, 3][d.next(4) as usize]);
    e.mem
        .set_u32(task + 0x18, [0x31, 0x32, 7][d.next(3) as usize]);
    e.mem.set_u32(task + 0x28, r);
    let model = match d.next(3) {
        0 => 0,
        n => {
            let model = e.mem.alloc(0x20);
            let node = if n == 1 { e.mem.alloc(0x40) } else { 0 };
            e.mem.set_u32(model + 0xc, node);
            model
        }
    };
    e.mem.set_u32(task + 0x30, model);
    // The globals: TES and the model loader with its map of queued references.
    let tes = e.mem.alloc(0x40);
    e.set_global(TES_POINTER, tes);
    let map_vt = e.mem.alloc(0x40);
    let remove = if d.next(2) == 0 {
        0x0090_62e0
    } else {
        MAP_VT_FN + 0x14
    };
    e.mem.set_u32(map_vt + 0x14, remove);
    let map = e.mem.alloc(0x40);
    e.mem.set_u32(map, map_vt);
    let loader = e.mem.alloc(0x20);
    e.mem.set_u32(loader + 8, map);
    e.set_global(MODEL_LOADER_POINTER, loader);
    task
}

/// The calls the attach's typed version does in place: the task's state test, and the form type the
/// reference class's slot 0x224 (`005725b0`, reached in `TES`'s load) reads in place.
const ATTACH_INLINED: &[u32] = &[0x0044_9130, 0x0040_1170];

/// The other families' typed calls the attach makes: the reference's cell, the class's false body,
/// the interlocked decrement, the map's removal.
const ATTACH_BY_ADDRESS: &[u32] = &[0x008d_6f30, 0x0047_c850, 0x0040_19a0, 0x0090_62e0];

/// `QueuedReference::Attach` against its translation, over the seeds' worlds.
#[test]
fn queued_reference_attach() {
    Diff::new(entry!(0x0044_0ba0, cell_refs::attach::attach(u32)))
        .family(&CELL_REFS)
        .family(&leaf::LEAVES)
        .goes_by_address(ATTACH_BY_ADDRESS)
        .ignore_calls(ATTACH_INLINED)
        .inputs(&[Input::Of(SEEDS)])
        .setup(|e, case| vec![attach_world(e, case.word(0), 0x0044_0ba0)])
        .run()
        .assert_same();
}

/// The character's and the player's attaches against their translations (each comes down to
/// `00440ba0`).
#[test]
fn character_and_player_attach() {
    Diff::new(entry!(
        0x0044_1800,
        cell_refs::attach::character_attach(u32)
    ))
    .family(&CELL_REFS)
    .family(&leaf::LEAVES)
    .goes_by_address(ATTACH_BY_ADDRESS)
    .ignore_calls(ATTACH_INLINED)
    .inputs(&[Input::Of(&SEEDS[..24])])
    .setup(|e, case| vec![attach_world(e, case.word(0), 0x0044_1800)])
    .run()
    .assert_same();
    Diff::new(entry!(0x0044_1c30, cell_refs::attach::player_attach(u32)))
        .family(&CELL_REFS)
        .family(&leaf::LEAVES)
        .goes_by_address(ATTACH_BY_ADDRESS)
        .ignore_calls(ATTACH_INLINED)
        .inputs(&[Input::Of(&SEEDS[..24])])
        .setup(|e, case| vec![attach_world(e, case.word(0), 0x0044_1c30)])
        .run()
        .assert_same();
}

/// `QueuedReference::PostProcess` against its translation: the task's slot 0x3c holding each of the
/// exe's three attaches, or something else (JIP's attach is one), which is called through the slot.
#[test]
fn queued_reference_post_process() {
    Diff::new(entry!(0x0043_fdf0, cell_refs::attach::post_process(u32)))
        .family(&CELL_REFS)
        .family(&leaf::LEAVES)
        .goes_by_address(ATTACH_BY_ADDRESS)
        .ignore_calls(ATTACH_INLINED)
        .inputs(&[
            Input::Of(&SEEDS[..32]),
            Input::Of(&[0x0044_0ba0, 0x0044_1800, 0x0044_1c30, TASK_VT_FN + 0x3c]),
        ])
        .setup(|e, case| vec![attach_world(e, case.word(0), case.word(1))])
        .run()
        .assert_same();
}

/// The attach's worlds reach every branch it has, checked on the translation: a task in state 6, a
/// flagged reference with a cell (locked, not loaded), a reference with no cell, a cell not loaded,
/// a cell loaded with a model and its root node, with a model without one, with no model, a
/// reference moved to another cell by the load (the other cell's lock let go), a critical and a
/// non-critical 3D, each count floored at 0, and the map's removal by either slot.
#[test]
fn the_attach_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeMap::<&str, u32>::new();
    for &seed in SEEDS {
        let mut e = Engine::new_untyped_for_test();
        let task = attach_world(&mut e, seed, 0x0044_0ba0);
        let r = e.mem.u32(task + 0x28);
        let cell = e.mem.u32(r + 0x40);
        let counts = if cell == 0 {
            (0, 0)
        } else {
            (e.mem.u32(cell + 0xa4), e.mem.u32(cell + 0xa0))
        };
        let map = e.mem.u32(e.global::<u32>(MODEL_LOADER_POINTER) + 8);
        let by_own_slot = e.mem.u32(e.mem.u32(map) + 0x14) == 0x0090_62e0;
        e.call(0x0044_0ba0, &args![task]);
        assert_eq!(e.mem.u32(Q_SCOPE_DEPTH), 0, "seed {seed}");
        let mut hit = |name| *reached.entry(name).or_default() += 1;
        if e.mem.u32(task + 0xc) == 6 {
            assert_eq!(e.mem.u32(A_REMOVED), 0, "seed {seed}");
            hit("state 6");
            continue;
        }
        assert_eq!(e.mem.u32(A_REMOVED), 1, "seed {seed}");
        hit(if by_own_slot {
            "removed by the map's own slot"
        } else {
            "removed by a stand-in slot"
        });
        let flagged = e.mem.u32(r + 8) & 0x820 != 0;
        if cell == 0 {
            hit("no cell");
            continue;
        }
        assert_eq!(e.mem.u32(A_LOCKED), cell, "seed {seed}");
        let loaded = e.mem.u32(A_LOADED) != 0;
        if flagged {
            assert!(!loaded, "seed {seed}");
            hit("flagged");
        } else if !loaded {
            hit("cell not loaded");
        } else {
            let model = e.mem.u32(task + 0x30);
            hit(if model == 0 {
                "loaded, no model"
            } else if e.mem.u32(model + 0xc) != 0 {
                "loaded, a model with a root node"
            } else {
                "loaded, a model without one"
            });
        }
        let now = e.mem.u32(r + 0x40);
        if now != cell {
            assert_eq!(e.mem.u32(A_UNLOCKED), now, "seed {seed}");
            hit("moved by the load");
            continue;
        }
        assert_eq!(e.mem.u32(A_LOCK_DEPTH), 0, "seed {seed}");
        let critical = e.mem.u32(cell + 0xa0) != counts.1 || (counts.1 == 0 && critical_3d(&e, r));
        hit(if critical { "critical" } else { "not critical" });
        if counts.0 == 0 {
            hit("queued count floored");
        }
        if critical && counts.1 == 0 {
            hit("critical count floored");
        }
    }
    for name in [
        "state 6",
        "removed by the map's own slot",
        "removed by a stand-in slot",
        "no cell",
        "flagged",
        "cell not loaded",
        "loaded, no model",
        "loaded, a model with a root node",
        "loaded, a model without one",
        "moved by the load",
        "critical",
        "not critical",
        "queued count floored",
        "critical count floored",
    ] {
        assert!(
            reached.contains_key(name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}

/// Whether the reference's 3D is critical in the attach's world: its slot 0x100 (the class's false
/// body, or the stand-in's word) says it is no actor.
fn critical_3d(e: &Engine, r: u32) -> bool {
    let vt = e.mem.u32(r);
    e.mem.u32(vt + 0x100) == 0x0047_c850 || e.mem.u32(r + ANSWER_IS_ACTOR) == 0
}

/// `TES::IsCellLoaded` against its translation: no cell, each load state, and the flag as the game
/// passes it: 0 or 1, and a process level up to 3 (`MobileObject::ChangeProcessLevel`, `00931920`,
/// pushes its level byte as the flag), with a word whose low byte is 0 (the flag is a byte).
#[test]
fn tes_is_cell_loaded() {
    Diff::new(entry!(
        0x0045_11e0,
        cell_refs::attach::is_cell_loaded(u32, u32, u8) -> bool
    ))
    .family(&CELL_REFS)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 0xff, 0x100]),
        Input::Of(&[0, 1, 2, 3, 0x100]),
    ])
    .setup(|e, case| {
        let tes = e.mem.alloc(0x40);
        let cell = match case.word(0) {
            0x100 => 0,
            state => {
                let cell = e.mem.alloc(0x100);
                e.mem.set_u8(cell + 0x26, state as u8);
                cell
            }
        };
        vec![tes, cell, case.word(1)]
    })
    .run()
    .assert_same();
}

/// The loader's word that a reference is done against its translation: the map's slot 0x14 the
/// lock-free family's removal (by its address here) or a stand-in.
#[test]
fn loader_reference_done() {
    Diff::new(entry!(
        0x0044_09e0,
        cell_refs::attach::reference_done(u32, u32)
    ))
    .family(&CELL_REFS)
    .goes_by_address(&[0x0090_62e0])
    .inputs(&[Input::Flag])
    .setup(|e, case| {
        let task = attach_world(e, 7, 0x0044_0ba0);
        let map = e.mem.u32(e.global::<u32>(MODEL_LOADER_POINTER) + 8);
        let remove = if case.word(0) != 0 {
            0x0090_62e0
        } else {
            MAP_VT_FN + 0x14
        };
        e.mem.set_u32(e.mem.u32(map) + 0x14, remove);
        vec![
            e.global::<u32>(MODEL_LOADER_POINTER),
            e.mem.u32(task + 0x28),
        ]
    })
    .run()
    .assert_same();
}

/// The world of the cell's two counters going down: the counter from 0 (floored), 1, 5 or with
/// its top bit set, the other counter beside it.
trait CountDownCase {
    fn count_down_case(self) -> Diff;
}

impl CountDownCase for Diff {
    fn count_down_case(self) -> Diff {
        self.family(&CELL_REFS)
            .goes_by_address(&[0x0040_19a0])
            .inputs(&[Input::Of(&[0, 1, 5, 0x8000_0000, 0x8000_0001])])
            .setup(|e, case| {
                queue_stand_ins(e);
                let cell = e.mem.alloc(0x100);
                e.mem.set_u32(cell + 0xa0, case.word(0));
                e.mem.set_u32(cell + 0xa4, case.word(0) ^ 1);
                vec![cell]
            })
    }
}

/// The cell's count of queued references going down against its translation.
#[test]
fn cell_queued_count_goes_down() {
    Diff::new(entry!(
        0x0044_0a10,
        cell_refs::attach::queued_count_down(u32)
    ))
    .count_down_case()
    .run()
    .assert_same();
}

/// The cell's count of critical queued references going down against its translation.
#[test]
fn cell_critical_count_goes_down() {
    Diff::new(entry!(
        0x0044_0a50,
        cell_refs::attach::critical_count_down(u32)
    ))
    .count_down_case()
    .run()
    .assert_same();
}

// `TES`'s load of a reference into the scene (`00451ef0`).
//
// Its world, from a seed: the reference's class (`TESObjectREFR`'s own slots, an actor's with the
// true body at 0x100, an `Explosion`'s with it at 0x220, or stand-ins answering words kept in the
// reference) and its type byte (what the exe's slot 0x224 tests), its form flags (seen from a
// distance, `0x80000`, the particles' `0x1000000`, `0x20`, `0x800`), its base form (type 0x2c,
// 0x2d, 0x23 with or without its flag `0x40000000`, 0x1e, 0x25 and others; seen from a distance or
// not; cast or not), its loaded data (none, or one with a 3D or none, a count from -1 to 2, flags),
// the 3D's parent; the cell's state and its count of references seen from a distance (0xffff too:
// it wraps); the source task (none, with a node or not, its slot 0x40 the empty body or a
// stand-in); the game loader's word (its flag, and bit 3 beside it), the player, the large cell's
// byte, the cast's sizes, its radius and the reference's scale (the product just below 5 that 24
// bits round to 5, one past 2^63, which two seeds always take, a NaN); JIP's patches and its
// handler's answer; now and then a null reference. The reference ID's walk runs on a small scene
// graph ([`scene_graph`]).

/// A reference's words the load's stand-ins answer from (past the pass's): its scale (a float's
/// bits), its slot 0x220, its own `Load3D`, JIP's handler.
const ANSWER_SCALE: u32 = 0x110;
const ANSWER_OTHER: u32 = 0x114;
const ANSWER_LOAD_3D: u32 = 0x118;
const ANSWER_JIP: u32 = 0x11c;
/// A cast's radius, its slot 0x1bc's answer (a float's bits).
const CAST_RADIUS: u32 = 0x100;
/// The node JIP's handler answers.
const JIP_NODE: u32 = 0x0712_0000;

/// The globals the load reads: the shadow scene nodes, the large cell's byte and its limit.
const SHADOW_SCENE_NODES: u32 = 0x011f_91c8;
const LARGE_STEP_ON: u32 = 0x0119_b8c8;
const LARGE_LIMIT: u32 = 0x0101_79e0;

/// The work the load hands on, standing in (the call log holds each call with its words). The
/// reference ID's walk of the scene graph (`004b6dc0`) is this family's and runs as itself on
/// [`scene_graph`]'s objects; so do the shadow scene node's add and lighting update (`00b5eeb0`,
/// `00b5d9f0`) on its queues, their lock and the list node pool of [`lighting_stand_ins`].
const SCENE_DOUBLES: [u32; 8] = [
    0x0056_5210, // TESActorBase::SetStartsDead
    0x00b5_cbd0, // the shadow scene node's actors' add
    0x0056_f700, // the step after the attach
    0x0057_8170, // the master particle addon nodes' removal
    0x0057_8060, // and addition
    0x0056_5730, // TESObjectREFR::InitScript
    0x008c_4640, // Actor::UpdateAlpha
    0x0054_7ad0, // the large cell's step
];

/// The scene graph the load's walk (`004b6dc0`) meets, on JIP's node's page: JIP's node is a node
/// (the exe's `NiNode` slots 0xc and 0x18) with one child, a geometry (the exe's geometry slots)
/// whose list holds one shade property (its slot 0x8c the exe's `3`) with a word at +0x1c the walk
/// accepts, so the ID is stored at its +0x70. The world's other nodes are geometries holding the
/// same list.
const NODE_VTABLE: u32 = JIP_NODE + 0x800;
const GEOMETRY_VTABLE: u32 = JIP_NODE + 0x900;
const PROPERTY_VTABLE: u32 = JIP_NODE + 0xa00;
const JIP_GEOMETRY: u32 = JIP_NODE + 0x200;
const PROPERTY_LIST: u32 = JIP_NODE + 0x300;
const JIP_CHILDREN: u32 = JIP_NODE + 0x400;
const SHADE_PROPERTY: u32 = JIP_NODE + 0x500;

fn scene_graph(e: &mut Engine) {
    e.map(JIP_NODE, 0x1000);
    e.mem.set_u32(NODE_VTABLE + 0xc, 0x0068_15c0);
    e.mem.set_u32(NODE_VTABLE + 0x18, 0x00ac_bb70);
    e.mem.set_u32(GEOMETRY_VTABLE + 0xc, 0x00ac_bb70);
    e.mem.set_u32(GEOMETRY_VTABLE + 0x18, 0x00e6_8810);
    e.mem.set_u32(PROPERTY_VTABLE + 0x8c, 0x00a6_a000);
    e.mem.set_u32(SHADE_PROPERTY, PROPERTY_VTABLE);
    e.mem.set_u32(SHADE_PROPERTY + 0x1c, 1);
    e.mem.set_u32(PROPERTY_LIST + 8, SHADE_PROPERTY);
    e.mem.set_u32(JIP_GEOMETRY, GEOMETRY_VTABLE);
    e.mem.set_u32(JIP_GEOMETRY + 0x24, PROPERTY_LIST);
    e.mem.set_u32(JIP_NODE, NODE_VTABLE);
    e.mem.set_u32(JIP_NODE + 0xa0, JIP_CHILDREN);
    e.mem.set_u16(JIP_NODE + 0xa6, 1);
    e.mem.set_u32(JIP_CHILDREN, JIP_GEOMETRY);
}

/// A float's bits in ST0.
fn float_ret(bits: u32) -> Ret {
    Ret {
        st0: f64::from(f32::from_bits(bits)),
        ..Ret::default()
    }
}

/// The reference's 3D set (`00570f70(reference, node)`): kept where both the exe's slot 0x1d0 (the
/// loaded data's +0x14) and the stand-in's word read it.
fn set_3d(e: &mut Engine, a: &[u32]) -> Ret {
    let data = e.mem.u32(a[0] + 0x64);
    if data != 0 {
        e.mem.set_u32(data + 0x14, a[1]);
    }
    e.mem.set_u32(a[0] + ANSWER_3D, a[1]);
    Ret::default()
}

/// JIP's handler at `0x45234E`: the reference's word, when it has one.
fn jip_on_load_actor(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let node = e.mem.u32(a[0] + ANSWER_JIP);
    (node != 0).then_some(node)
}

fn scene_stand_ins(e: &mut Engine) {
    use engine_core::extend::{Owner, SiteSlot};
    stand_ins(e);
    for page in [0x011f_9000, 0x0119_b000, 0x0101_7000] {
        e.map(page, 0x1000);
    }
    for addr in SCENE_DOUBLES {
        e.register(addr, nothing);
    }
    scene_graph(e);
    // The shadow scene node (its queues empty) and what its queue meets.
    e.map(0x0333_0000, 0x1000);
    lighting_stand_ins(e);
    e.register(0x0057_0f70, set_3d);
    e.register(0x0056_7400, |e, a| {
        float_ret(e.mem.u32(a[0] + ANSWER_SCALE))
    });
    e.register(VT_FN + 0x1bc, |e, a| {
        float_ret(e.mem.u32(a[0] + CAST_RADIUS))
    });
    e.register(VT_FN + 0x220, answer_of::<ANSWER_OTHER>);
    e.register(VT_FN + 0x1c8, answer_of::<ANSWER_LOAD_3D>);
    e.register(VT_FN + 0x1f4, |_, a| ret(a[0] + 0x30));
    for slot in [0x304, 0x308, 0x40, 0xe8] {
        e.register(VT_FN + slot, nothing);
    }
    e.on_site(SiteSlot::new(Owner::Jip, 0x0045_234e), jip_on_load_actor);
    e.set_global(SHADOW_SCENE_NODES, 0x0333_0000u32);
    e.set_global(LARGE_LIMIT, 200.0f64);
}

/// A reference class's vtable for the load: 0 `TESObjectREFR`'s own slots, 1 an actor's (the true
/// body at 0x100), 2 an `Explosion`'s (the true body at 0x220), 3 every slot a stand-in.
fn scene_vtable(e: &mut Engine, class: u32) -> u32 {
    let vt = e.mem.alloc(0x310);
    for slot in [
        0x100, 0x1c8, 0x1d0, 0x1f4, 0x220, 0x224, 0x260, 0x304, 0x308,
    ] {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    if class != 3 {
        let (is_actor, other) = match class {
            1 => (0x008d_0360, 0x0047_c850),
            2 => (0x0047_c850, 0x008d_0360),
            _ => (0x0047_c850, 0x0047_c850),
        };
        e.mem.set_u32(vt + 0x100, is_actor);
        e.mem.set_u32(vt + 0x220, other);
        e.mem.set_u32(vt + 0x224, 0x0057_25b0);
        e.mem.set_u32(vt + 0x1d0, 0x0043_fcd0);
        if class == 0 {
            e.mem.set_u32(vt + 0x1f4, 0x0043_6aa0);
        }
    }
    vt
}

/// A node (a geometry of [`scene_graph`]'s), culled (its flag 1, which `AddObject` leaves out) or
/// not, with a parent (whose slot 0xe8 takes it) or none.
fn scene_node(e: &mut Engine, d: &mut Draw, culled: bool) -> u32 {
    let node = e.mem.alloc(0x40);
    e.mem.set_u32(node, GEOMETRY_VTABLE);
    e.mem.set_u32(node + 0x24, PROPERTY_LIST);
    e.mem.set_u8(node + 0x30, u8::from(culled));
    if d.next(3) != 0 {
        let vt = e.mem.alloc(0x100);
        e.mem.set_u32(vt + 0xe8, VT_FN + 0xe8);
        let parent = e.mem.alloc(0x20);
        e.mem.set_u32(parent, vt);
        e.mem.set_u32(node + 0x18, parent);
    }
    node
}

/// The load's world of a seed: its words (`TES`, the reference, the cell, the source, `flag`).
fn scene_world(e: &mut Engine, seed: u32, flag: u32) -> Vec<u32> {
    use engine_core::extend::jip_patches::{NPC_PERKS, NPC_WEAPON_MODS};
    scene_stand_ins(e);
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ 0x5ce4);
    // A quarter of the seeds' nodes culled.
    let culled = seed % 4 == 3;
    e.set_jip_patches([0, 0, NPC_PERKS, NPC_WEAPON_MODS][d.next(4) as usize]);
    e.mem.set_u8(LARGE_STEP_ON, u8::from(d.next(4) != 0));
    let loader = e.mem.alloc(0x250);
    // The game loader's word: its flag (bit 1) or not, with bit 0 or bit 2 alone too, and a third
    // of the seeds with bit 3 as well (8 alone, 9, 0xa, 0xc, 0xe), which the flag's test must not
    // take for its own.
    let high = if seed % 3 == 1 { 8 } else { 0 };
    e.mem.set_u32(
        loader + 0x244,
        [0, 0, 1, 2, 4, 6][d.next(6) as usize] | high,
    );
    e.set_global(GAME_LOADER_POINTER, loader);
    // The reference: its class, type byte, flags, form id and position.
    let class = d.next(4);
    let vt = scene_vtable(e, class);
    let r = e.mem.alloc(0x140);
    e.mem.set_u32(r, vt);
    e.mem
        .set_u8(r + 4, if d.next(10) == 0 { 0x3d } else { 0x3b });
    e.mem.set_u32(
        r + 8,
        [
            0,
            0,
            0x8000,
            0x0008_8000,
            0x0100_0000,
            0x0108_8000,
            0x20,
            0x800,
        ][d.next(8) as usize],
    );
    e.mem.set_u32(r + 0xc, 0x0001_2300 + seed);
    for (i, value) in [1.5f32, -2.0, 3.25].into_iter().enumerate() {
        e.mem.set_f32(r + 0x30 + 4 * i as u32, value);
    }
    // The base form, and its cast for type 0x25: the sizes, the radius, the reference's scale.
    let base = e.mem.alloc(0x120);
    // The seed takes part, so every type meets every other draw (the sequence alone pairs them).
    let kind = [0x2c, 0x2d, 0x23, 0x23, 0x1e, 0x25, 0x25, 0x25, 0x15, 0x2a]
        [((d.next(10) + seed) % 10) as usize];
    e.mem.set_u8(base + 4, kind);
    e.mem.set_u32(base + 8, [0, 0, 0x8000][d.next(3) as usize]);
    e.mem
        .set_u32(base + 0x48, [0, 0x4000_0000][d.next(2) as usize]);
    e.mem.set_u32(r + 0x20, base);
    let cast = if d.next(5) == 0 {
        0
    } else {
        let vt = e.mem.alloc(0x200);
        e.mem.set_u32(vt + 0x1bc, VT_FN + 0x1bc);
        let cast = e.mem.alloc(0x120);
        e.mem.set_u32(cast, vt);
        let (first, second) = [
            (300.0f32, 250.0f32),
            (300.0, 250.0),
            (200.0, 300.0),
            (300.0, f32::NAN),
        ][d.next(4) as usize];
        e.mem.set_f32(cast + 0x8c, first);
        e.mem.set_f32(cast + 0x90, second);
        cast
    };
    e.mem.set_u32(base + ANSWER_CAST, cast);
    let (radius, scale) = [
        (2.5f32, 2.0f32),
        (5.0 / 3.0, 3.0),
        (1_099_511_627_776.0, 1_073_741_824.0),
        (-2.5, 3.0),
        (f32::NAN, 1.0),
    ][d.next(5) as usize];
    if cast != 0 {
        e.mem.set_u32(cast + CAST_RADIUS, radius.to_bits());
    }
    e.mem.set_u32(r + ANSWER_SCALE, scale.to_bits());
    // The extra list's `ScriptLocals` (the word after the list's own): none, fresh or built.
    let locals = match d.next(3) {
        0 => 0,
        n => {
            let l = e.mem.alloc(0x10);
            e.mem.set_u32(l, 0x0500_0000 + d.next(0x100));
            e.mem.set_u32(l + 8, n - 1);
            l
        }
    };
    e.mem.set_u32(r + 0x48, locals);
    // The 3D, the loaded data with its count and flags.
    let node = if d.next(4) == 0 {
        0
    } else {
        scene_node(e, &mut d, culled)
    };
    let data = if d.next(5) == 0 {
        0
    } else {
        let data = e.mem.alloc(0x20);
        e.mem
            .set_u32(data + 4, [-1i32, 0, 1, 2][d.next(4) as usize] as u32);
        e.mem
            .set_u32(data + 0x10, [0, 2, 6, 4, 2][d.next(5) as usize]);
        e.mem.set_u32(data + 0x14, node);
        data
    };
    e.mem.set_u32(r + 0x64, data);
    // The stand-ins' answers.
    e.mem.set_u32(r + ANSWER_IS_ACTOR, d.next(2));
    e.mem.set_u32(r + ANSWER_KIND, u32::from(d.next(10) == 0));
    e.mem.set_u32(r + ANSWER_OTHER, u32::from(d.next(10) == 0));
    e.mem.set_u32(r + ANSWER_3D, node);
    let own_3d = if d.next(4) == 0 {
        0
    } else {
        scene_node(e, &mut d, culled)
    };
    e.mem.set_u32(r + ANSWER_LOAD_3D, own_3d);
    e.mem
        .set_u32(r + ANSWER_JIP, if d.next(2) == 0 { 0 } else { JIP_NODE });
    // The cell.
    let cell = e.mem.alloc(0x100);
    e.mem
        .set_u8(cell + 0x26, [0, 2, 4, 5, 6, 6, 5, 7][d.next(8) as usize]);
    e.mem
        .set_u16(cell + 0xaa, [0, 5, 0xffff][d.next(3) as usize]);
    // The source task.
    let source = if d.next(3) == 0 {
        0
    } else {
        let vt = e.mem.alloc(0x48);
        e.mem
            .set_u32(vt + 0x40, [0x0048_3710, VT_FN + 0x40][d.next(2) as usize]);
        let task = e.mem.alloc(0x48);
        e.mem.set_u32(task, vt);
        let model_node = if d.next(3) == 0 {
            0
        } else {
            scene_node(e, &mut d, culled)
        };
        e.mem.set_u32(task + 0x34, model_node);
        task
    };
    // The player: the reference now and then.
    let player = if d.next(6) == 0 { r } else { 0x0700_1000 };
    e.set_global(PLAYER_POINTER, player);
    // Four seeds take the large cell's step whatever their draws: a plain reference (the exe's
    // slots) with no form flags, a type 0x25 base form, a cast whose sizes pass; two of them the
    // radius 2^40 and the scale 2^30 (the product past 2^63), two a NaN radius. The exe's store
    // gives the integer indefinite for both, whose low word is 0, where a saturating cast would
    // give 0xffffffff past 2^63 (and 0 for the NaN).
    if seed % 24 == 21 {
        let past = seed % 48 == 21;
        let vt = scene_vtable(e, 0);
        e.mem.set_u32(r, vt);
        e.mem.set_u8(r + 4, 0x3b);
        e.mem.set_u32(r + 8, 0);
        e.mem.set_u8(base + 4, 0x25);
        e.mem.set_u8(LARGE_STEP_ON, 1);
        let vt = e.mem.alloc(0x200);
        e.mem.set_u32(vt + 0x1bc, VT_FN + 0x1bc);
        let cast = e.mem.alloc(0x120);
        e.mem.set_u32(cast, vt);
        e.mem.set_f32(cast + 0x8c, 300.0);
        e.mem.set_f32(cast + 0x90, 250.0);
        let (radius, scale) = if past {
            (1_099_511_627_776.0f32, 1_073_741_824.0f32)
        } else {
            (f32::NAN, 1.0)
        };
        e.mem.set_u32(cast + CAST_RADIUS, radius.to_bits());
        e.mem.set_u32(base + ANSWER_CAST, cast);
        e.mem.set_u32(r + ANSWER_SCALE, scale.to_bits());
    }
    let tes = e.mem.alloc(0x40);
    let r = if seed % 32 == 31 { 0 } else { r };
    vec![tes, r, cell, source, flag]
}

/// The calls the load's typed version does in place: the form type, the game loader's flag, the
/// extra data and the extra data list, the base form's flag test, the large cell's byte and the
/// cast's sizes, the two empty bodies, the reference's flag tests, the loaded data's count, the
/// source's node, the node's parent.
const SCENE_INLINED: &[u32] = &[
    0x0040_1170,
    0x0042_ce10,
    0x0043_0830,
    0x0045_2440,
    0x0045_2480,
    0x0045_bb80,
    0x0048_3710,
    0x0045_34f0,
    0x0056_4e60,
    0x0056_51e0,
    0x0057_b200,
    0x005d_43c0,
    0x005d_8710,
    0x0096_11e0,
];

/// `TES`'s load of a reference into the scene against its translation, over the seeds' worlds and
/// `flag` 0, 1 and a word whose low byte is 0.
#[test]
fn tes_load_reference() {
    Diff::new(entry!(
        0x0045_1ef0,
        cell_refs::scene::load_reference(u32, u32, u32, u32, u8)
    ))
    .family(&CELL_REFS)
    .family(&leaf::LEAVES)
    .goes_by_address(&[0x0047_c850, 0x0043_fcd0])
    // The walk's node child count and child (the dispatch family's), on JIP's node.
    .goes_by_address(&[0x0043_b480, 0x0043_b4a0])
    // The shadow scene node's queue lock (the dispatch family's) and list node pool (the draw F
    // family's).
    .goes_by_address(&[0x0040_fbf0, 0x0043_a010])
    .ignore_calls(SCENE_INLINED)
    // What the walk does in place.
    .ignore_calls(WALK_INLINED)
    .inputs(&[Input::Of(SEEDS), Input::Of(&[0, 1, 0x100])])
    .setup(|e, case| scene_world(e, case.word(0), case.word(1)))
    .run()
    .assert_same();
}

/// The actor classes' true body against its translation.
#[test]
fn answers_true() {
    Diff::new(entry!(0x008d_0360, cell_refs::answers_true(u32) -> bool))
        .family(&CELL_REFS)
        .inputs(&[Input::Of(&[0, 0x0700_0000])])
        .run()
        .assert_same();
}

/// The branches [`the_scene_seeds_reach_every_branch`] asks the seeds to reach.
const SCENE_BRANCHES: &[&str] = &[
    "a null reference",
    "handed to slot 0x308",
    "handed to slot 0x304",
    "its distant 3D given up",
    "not loaded: form flag 0x20 or 0x800",
    "off the temp-change list",
    "the source's node set as its 3D",
    "its own Load3D",
    "the source's slot 0x40",
    "the source's empty body",
    "a type 0x2c or 0x2d node to its parent",
    "put in the scene",
    "kept from the attach",
    "an actor's node",
    "attached",
    "particles removed",
    "particles added",
    "queued by AddObject",
    "a culled node left out by AddObject",
    "lighting updated",
    "type 0x1e's step",
    "the action list set up",
    "the action list set up under a word of 8 alone",
    "the action list set up under bit 0",
    "the action list set up under bit 2 without bit 1",
    "the game loader's flag",
    "counted as seen from a distance",
    "the distant count wraps",
    "the large cell's step",
    "a product just below 5",
    "a product past 2^63",
    "a NaN product",
    "an actor's alpha",
    "an ID set on the scene graph",
    "the player's ID not set",
    "JIP's node",
    "an ID stored in the shade property",
];

/// The load's worlds reach every branch it has, checked on the translation (under the other
/// threads' word; the Diff runs both).
#[test]
fn the_scene_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeMap::<&str, u32>::new();
    for &seed in SEEDS {
        for flag in [0u8, 1] {
            let mut e = Engine::new_untyped_for_test();
            let w = scene_world(&mut e, seed, u32::from(flag));
            let (r, cell) = (w[1], w[2]);
            let distant_before = e.mem.u16(cell + 0xaa);
            e.call_log = Some(Vec::new());
            e.call(0x0045_1ef0, &args![w[0], r, cell, w[3], flag]);
            let log = e.call_log.take().unwrap_or_default();
            let words = |addr: u32| -> Vec<Vec<u32>> {
                log.iter()
                    .filter(|(a, _)| *a == addr)
                    .map(|(_, w)| w.clone())
                    .collect()
            };
            let called = |addr: u32| !words(addr).is_empty();
            let mut hit = |name| *reached.entry(name).or_default() += 1;
            if r == 0 {
                hit("a null reference");
                continue;
            }
            if called(VT_FN + 0x308) {
                hit("handed to slot 0x308");
                continue;
            }
            if called(VT_FN + 0x304) {
                hit("handed to slot 0x304");
                continue;
            }
            if called(0x0056_5210) {
                hit("its distant 3D given up");
            }
            if !called(0x0056_5730) {
                hit("not loaded: form flag 0x20 or 0x800");
                continue;
            }
            let base = e.mem.u32(r + 0x20);
            let kind = e.mem.u8(base + 4);
            for (addr, name) in [
                (0x0096_f400, "off the temp-change list"),
                (VT_FN + 0x1c8, "its own Load3D"),
                (VT_FN + 0x40, "the source's slot 0x40"),
                (0x0048_3710, "the source's empty body"),
                (0x00b5_eeb0, "put in the scene"),
                (0x00b5_cbd0, "an actor's node"),
                (0x0054_8880, "attached"),
                (0x0057_8170, "particles removed"),
                (0x0057_8060, "particles added"),
                (0x0045_34f0, "type 0x1e's step"),
                (0x005a_c750, "the action list set up"),
                (0x008c_4640, "an actor's alpha"),
                (0x004b_6dc0, "an ID set on the scene graph"),
            ] {
                if called(addr) {
                    hit(name);
                }
            }
            if words(0x0057_0f70).iter().any(|w| w[1] != 0) {
                hit("the source's node set as its 3D");
            }
            // `AddObject` queues a node that is not culled; the load's own update queues it again.
            let added = words(0x00b5_eeb0);
            let queued_by_add = added
                .iter()
                .filter(|w| e.mem.u8(w[1] + 0x30) & 1 == 0)
                .count();
            if queued_by_add > 0 {
                hit("queued by AddObject");
            }
            if queued_by_add < added.len() {
                hit("a culled node left out by AddObject");
            }
            if words(0x00b5_d9f0).len() > queued_by_add {
                hit("lighting updated");
            }
            if (kind == 0x2c || kind == 0x2d) && called(VT_FN + 0xe8) {
                hit("a type 0x2c or 0x2d node to its parent");
            }
            if called(0x00b5_eeb0) && !called(0x0054_8880) {
                hit("kept from the attach");
            }
            if !called(0x005a_c750) {
                hit("the game loader's flag");
            }
            let word = e.mem.u32(e.global::<u32>(GAME_LOADER_POINTER) + 0x244);
            if word == 8 && called(0x005a_c750) {
                hit("the action list set up under a word of 8 alone");
            }
            if word & 3 == 1 && called(0x005a_c750) {
                hit("the action list set up under bit 0");
            }
            if word & 6 == 4 && called(0x005a_c750) {
                hit("the action list set up under bit 2 without bit 1");
            }
            let distant_after = e.mem.u16(cell + 0xaa);
            if distant_after != distant_before {
                hit(if distant_after == 0 {
                    "the distant count wraps"
                } else {
                    "counted as seen from a distance"
                });
            }
            for step in words(0x0054_7ad0) {
                hit("the large cell's step");
                let scale = f32::from_bits(e.mem.u32(r + ANSWER_SCALE));
                if scale == 3.0 && step[4] == 4 {
                    hit("a product just below 5");
                }
                let cast = e.mem.u32(base + ANSWER_CAST);
                let radius = f32::from_bits(e.mem.u32(cast + CAST_RADIUS));
                if scale == 1_073_741_824.0 && radius == 1_099_511_627_776.0 && step[4] == 0 {
                    hit("a product past 2^63");
                }
                if radius.is_nan() && step[4] == 0 {
                    hit("a NaN product");
                }
            }
            if called(0x008c_4640) && !called(0x004b_6dc0) {
                hit("the player's ID not set");
            }
            if words(0x004b_6dc0).iter().any(|w| w[0] == JIP_NODE) {
                hit("JIP's node");
            }
            if e.mem.u32(SHADE_PROPERTY + 0x70) == e.mem.u32(r + 0xc) {
                hit("an ID stored in the shade property");
            }
        }
    }
    for name in SCENE_BRANCHES {
        assert!(
            reached.contains_key(name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}

// The reference ID's walk of the scene graph (`004b6dc0`) and `NiAVObject::GetProperty`
// (`00a59d30`).
//
// Their world, from a seed: a tree below a root (now and then none), at most five levels deep, of
// objects of five sorts: a node of the exe's `NiNode` slots (0xc the object itself, 0x18 none) with
// none to four children (a null child now and then); a geometry of the exe's geometry slots (0x18
// the object itself, 0xc none) with a list of none to four properties (null items too); an object
// of stand-in slots answering words kept in it (its 0x18 a word that is not the object, or 0 with
// its 0xc the object itself or 0); a stand-in node whose slot 0xc answers another node, which holds
// the children walked, while its own children (never walked) hold a geometry the walk would store
// in; and a light or a camera (the exe's slots: 0 at both), whose words where a node keeps its
// children (never walked) hold such a geometry too. Each property is of a class whose slot 0x8c
// holds one of the exe's eleven constant bodies (the shade property's most often) or a stand-in
// answering a word kept in it (3, 2, 7, -1), with its word at +0x1c from 0, 1, 5, 0xc, 0xd, -1 and
// 0x8000_0000.

/// Where the walk's stand-in slots are registered: `WALK_VT_FN + slot`.
const WALK_VT_FN: u32 = 0x00ff_8000;
/// An object's words its stand-in slots answer: 0x18 and 0xc.
const ANSWER_GEOMETRY: u32 = 0xb0;
const ANSWER_NODE: u32 = 0xb4;
/// A property's word its stand-in slot 0x8c answers.
const ANSWER_PROPERTY_KIND: u32 = 0x78;
/// The exe's bodies in a property's slot 0x8c, the shade property's three times.
const KIND_BODIES: [u32; 13] = [
    0x00ac_bb70,
    0x00b6_f470,
    0x00c1_ba40,
    0x00a6_a000,
    0x00a6_a000,
    0x00a6_a000,
    0x0081_b6c0,
    0x00e9_e670,
    0x00dc_9bc0,
    0x00a9_50e0,
    0x00dc_9e60,
    0x00a5_f500,
    0x00e0_9620,
];

/// The vtables of a walk world: the exe's node and geometry slots, stand-ins, and one per property
/// class (the exe's bodies, then the stand-in's).
struct WalkVtables {
    node: u32,
    geometry: u32,
    light: u32,
    stand_in: u32,
    properties: Vec<u32>,
}

fn walk_vtable(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
    let vt = e.mem.alloc(0x90);
    for &(slot, target) in slots {
        e.mem.set_u32(vt + slot, target);
    }
    vt
}

fn walk_vtables(e: &mut Engine) -> WalkVtables {
    e.register(WALK_VT_FN + 0xc, answer_of::<ANSWER_NODE>);
    e.register(WALK_VT_FN + 0x18, answer_of::<ANSWER_GEOMETRY>);
    e.register(WALK_VT_FN + 0x8c, answer_of::<ANSWER_PROPERTY_KIND>);
    let node = walk_vtable(e, &[(0xc, 0x0068_15c0), (0x18, 0x00ac_bb70)]);
    let geometry = walk_vtable(e, &[(0xc, 0x00ac_bb70), (0x18, 0x00e6_8810)]);
    // The exe's lights and camera (`NiPointLight`, `NiSpotLight`, `NiDirectionalLight`,
    // `NiCamera`): 0 at both slots.
    let light = walk_vtable(e, &[(0xc, 0x00ac_bb70), (0x18, 0x00ac_bb70)]);
    let stand_in = walk_vtable(e, &[(0xc, WALK_VT_FN + 0xc), (0x18, WALK_VT_FN + 0x18)]);
    let mut properties: Vec<u32> = KIND_BODIES
        .iter()
        .map(|&body| walk_vtable(e, &[(0x8c, body)]))
        .collect();
    properties.push(walk_vtable(e, &[(0x8c, WALK_VT_FN + 0x8c)]));
    WalkVtables {
        node,
        geometry,
        light,
        stand_in,
        properties,
    }
}

/// An object of the walk's world: its vtable; its children array at +0x9c (the items at +0xa0, the
/// count a 16-bit word at +0xa6) and its property list's head (+0x24) filled in by the caller.
fn walk_object(e: &mut Engine, vt: u32) -> u32 {
    let o = e.mem.alloc(0xc0);
    e.mem.set_u32(o, vt);
    o
}

fn set_children(e: &mut Engine, node: u32, children: &[u32]) {
    let n = children.len() as u32;
    let items = e.mem.alloc(4 * n.max(1));
    for (i, &child) in (0u32..).zip(children) {
        e.mem.set_u32(items + 4 * i, child);
    }
    e.mem.set_u32(node + 0xa0, items);
    e.mem.set_u16(node + 0xa6, n as u16);
}

/// A list of none to four properties (a null item now and then) for `object`.
fn walk_properties(e: &mut Engine, d: &mut Draw, vts: &WalkVtables, object: u32) {
    let classes = vts.properties.len() as u32;
    let mut link = object + 0x24;
    for _ in 0..d.next(5) {
        let node = e.mem.alloc(0xc);
        e.mem.set_u32(link, node);
        link = node;
        if d.next(6) == 0 {
            continue;
        }
        let p = e.mem.alloc(0x80);
        e.mem.set_u32(p, vts.properties[d.next(classes) as usize]);
        e.mem.set_u32(
            p + 0x1c,
            [0, 1, 5, 0xc, 0xd, 0xffff_ffff, 0x8000_0000][d.next(7) as usize],
        );
        e.mem.set_u32(p + 0x70, 0x5a5a_0000 + d.next(0x100));
        e.mem.set_u32(
            p + ANSWER_PROPERTY_KIND,
            [3, 2, 7, 0xffff_ffff][d.next(4) as usize],
        );
        e.mem.set_u32(node + 8, p);
    }
}

/// A list holding one shade property the walk accepts (its word at +0x1c 1) for `object`.
fn shade_list(e: &mut Engine, vts: &WalkVtables, object: u32) {
    let node = e.mem.alloc(0xc);
    let p = e.mem.alloc(0x80);
    e.mem.set_u32(p, vts.properties[3]);
    e.mem.set_u32(p + 0x1c, 1);
    e.mem.set_u32(node + 8, p);
    e.mem.set_u32(object + 0x24, node);
}

/// None to four children below a node at `depth` (a null one now and then).
fn walk_children(e: &mut Engine, d: &mut Draw, vts: &WalkVtables, depth: u32) -> Vec<u32> {
    let n = d.next(5);
    let mut out = Vec::new();
    for _ in 0..n {
        let child = if d.next(7) == 0 {
            0
        } else {
            walk_tree(e, d, vts, depth + 1)
        };
        out.push(child);
    }
    out
}

/// An object at `depth` and what is below it.
fn walk_tree(e: &mut Engine, d: &mut Draw, vts: &WalkVtables, depth: u32) -> u32 {
    let sort = if depth >= 4 { d.next(2) } else { d.next(8) };
    match sort {
        // A geometry of the exe's slots.
        0 => {
            let o = walk_object(e, vts.geometry);
            walk_properties(e, d, vts, o);
            o
        }
        // Stand-in slots: a geometry by a word that is not the object, or 0 with the object (or 0)
        // as a node.
        1 => {
            let o = walk_object(e, vts.stand_in);
            let geometry = [0, 0, 1, 0x0700_0000][d.next(4) as usize];
            e.mem.set_u32(o + ANSWER_GEOMETRY, geometry);
            walk_properties(e, d, vts, o);
            if geometry == 0 && d.next(2) == 0 && depth < 4 {
                e.mem.set_u32(o + ANSWER_NODE, o);
                let below = walk_children(e, d, vts, depth);
                set_children(e, o, &below);
            }
            o
        }
        // A stand-in node answering another node: that one's children are walked, its own never.
        2 => {
            let o = walk_object(e, vts.stand_in);
            let other = walk_object(e, vts.node);
            let below = walk_children(e, d, vts, depth);
            set_children(e, other, &below);
            e.mem.set_u32(o + ANSWER_NODE, other);
            let decoy = walk_object(e, vts.geometry);
            walk_properties(e, d, vts, decoy);
            set_children(e, o, &[decoy]);
            o
        }
        // A light or a camera (the exe's slots: 0 at both): never walked, though its words at +0xa0
        // and +0xa6 hold a decoy geometry whose shade property the walk would store in.
        7 => {
            let o = walk_object(e, vts.light);
            let decoy = walk_object(e, vts.geometry);
            shade_list(e, vts, decoy);
            set_children(e, o, &[decoy]);
            o
        }
        // A node of the exe's slots.
        _ => {
            let o = walk_object(e, vts.node);
            let below = walk_children(e, d, vts, depth);
            set_children(e, o, &below);
            o
        }
    }
}

/// The walk's world of a seed: its root (null for one seed in twelve).
fn walk_world(e: &mut Engine, seed: u32) -> u32 {
    let vts = walk_vtables(e);
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ 0x6dc0);
    let root = walk_tree(e, &mut d, &vts, 0);
    if seed % 12 == 7 {
        0
    } else {
        root
    }
}

/// The bodies the typed walk and `GetProperty` do in place: `004a2c20` and its `00441110`,
/// `0057bd60`, the slot bodies (0, and the object itself) and the property kinds' constants.
const WALK_INLINED: &[u32] = &[
    0x004a_2c20,
    0x0044_1110,
    0x0057_bd60,
    0x00ac_bb70,
    0x0068_15c0,
    0x00e6_8810,
    0x00b6_f470,
    0x00c1_ba40,
    0x00a6_a000,
    0x0081_b6c0,
    0x00e9_e670,
    0x00dc_9bc0,
    0x00a9_50e0,
    0x00dc_9e60,
    0x00a5_f500,
    0x00e0_9620,
];

/// The reference ID's walk against its translation, over the seeds' trees and two IDs.
#[test]
fn set_reference_id_on_scenegraph() {
    Diff::new(entry!(
        0x004b_6dc0,
        cell_refs::scenegraph::set_reference_id_on_scenegraph(u32, u32)
    ))
    .family(&CELL_REFS)
    .family(&engine_typed::dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .ignore_calls(WALK_INLINED)
    .inputs(&[Input::Of(SEEDS), Input::Of(&[0x0001_2345, 0])])
    .setup(|e, case| vec![walk_world(e, case.word(0)), case.word(1)])
    .run()
    .assert_same();
}

const PROPERTY_SEEDS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
];

/// `GetProperty` against its translation: a geometry's list from a seed, and every kind below 7
/// (the exe asks 4 too, in `00b587c0`), at it, past it and negative.
#[test]
fn get_property() {
    Diff::new(entry!(
        0x00a5_9d30,
        cell_refs::scenegraph::get_property(u32, i32) -> u32
    ))
    .family(&CELL_REFS)
    .ignore_calls(WALK_INLINED)
    .inputs(&[
        Input::Of(PROPERTY_SEEDS),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 0xd, 0xffff_ffff, 0x8000_0000]),
    ])
    .setup(|e, case| {
        let vts = walk_vtables(e);
        let mut d = Draw(case.word(0).wrapping_mul(0x9e37_79b9) ^ 0x9d30);
        let o = walk_object(e, vts.geometry);
        walk_properties(e, &mut d, &vts, o);
        vec![o, case.word(1)]
    })
    .run()
    .assert_same();
}

/// The walk's worlds reach what it does, checked on the translation: a null root, an ID stored, a
/// shade property's word at +0x1c of 0xc (accepted), 0xd and negative (refused), a geometry with no
/// shade property, a material property ahead of the shade property, the stand-in slots, a stand-in
/// node answering another node, a null child and a null property item, three of the exe's property
/// kinds asked, a tree three levels deep, a light or camera met under a node.
#[test]
fn the_walk_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeMap::<&str, u32>::new();
    for &seed in SEEDS {
        let mut e = Engine::new_untyped_for_test();
        let root = walk_world(&mut e, seed);
        let mut hit = |name| *reached.entry(name).or_default() += 1;
        if root == 0 {
            hit("a null root");
            continue;
        }
        e.call_log = Some(Vec::new());
        e.call(0x004b_6dc0, &args![root, 0x0001_2345u32]);
        let log = e.call_log.take().unwrap_or_default();
        let called = |addr: u32| log.iter().any(|(a, _)| *a == addr);
        for (addr, name) in [
            (0x0057_bd60, "an ID stored"),
            (WALK_VT_FN + 0x18, "a stand-in slot 0x18"),
            (WALK_VT_FN + 0xc, "a stand-in slot 0xc"),
            (WALK_VT_FN + 0x8c, "a stand-in property kind"),
            (0x00c1_ba40, "a material property asked"),
            (0x0081_b6c0, "a stencil property asked"),
            (0x00e0_9620, "a fog property asked"),
        ] {
            if called(addr) {
                hit(name);
            }
        }
        // What each GetProperty of the walk met in its object's list.
        for (addr, words) in &log {
            if *addr == WALK_VT_FN + 0xc {
                let answer = e.mem.u32(words[0] + ANSWER_NODE);
                if answer != 0 && answer != words[0] {
                    hit("a stand-in node answering another node");
                }
            }
            if *addr != 0x00a5_9d30 {
                continue;
            }
            let mut node = e.mem.u32(words[0] + 0x24);
            let (mut material_first, mut shade) = (false, None);
            while node != 0 {
                let p = e.mem.u32(node + 8);
                node = e.mem.u32(node);
                if p == 0 {
                    hit("a null property item");
                    continue;
                }
                let body = e.mem.u32(e.mem.u32(p) + 0x8c);
                if shade.is_none() && body == 0x00c1_ba40 {
                    material_first = true;
                }
                if shade.is_none() && body == 0x00a6_a000 {
                    shade = Some(p);
                }
            }
            let Some(p) = shade else {
                hit("a geometry with no shade property");
                continue;
            };
            if material_first {
                hit("a material property ahead of the shade property");
            }
            match e.mem.u32(p + 0x1c) {
                0xc => hit("a shade property's word 0xc"),
                0xd => hit("a shade property's word 0xd"),
                0xffff_ffff | 0x8000_0000 => hit("a shade property's negative word"),
                _ => {}
            }
        }
        // The tree's shape, through the exe's nodes: a null child, the depth, a light or a camera.
        let mut stack = vec![(root, 0u32)];
        while let Some((o, depth)) = stack.pop() {
            if depth >= 3 {
                hit("three levels deep");
            }
            let vt = e.mem.u32(o);
            if e.mem.u32(vt + 0xc) == 0x00ac_bb70 && e.mem.u32(vt + 0x18) == 0x00ac_bb70 {
                hit("a light or camera (both slots 0)");
            }
            if e.mem.u32(vt + 0xc) != 0x0068_15c0 {
                continue;
            }
            for i in 0..u32::from(e.mem.u16(o + 0xa6)) {
                let child = e.mem.u32(e.mem.u32(o + 0xa0) + 4 * i);
                if child == 0 {
                    hit("a null child");
                } else {
                    stack.push((child, depth + 1));
                }
            }
        }
    }
    for name in [
        "a null root",
        "an ID stored",
        "a stand-in slot 0x18",
        "a stand-in slot 0xc",
        "a stand-in property kind",
        "a material property asked",
        "a stencil property asked",
        "a fog property asked",
        "a null property item",
        "a geometry with no shade property",
        "a material property ahead of the shade property",
        "a shade property's word 0xc",
        "a shade property's word 0xd",
        "a shade property's negative word",
        "a stand-in node answering another node",
        "a null child",
        "three levels deep",
        "a light or camera (both slots 0)",
    ] {
        assert!(
            reached.contains_key(name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}

// The shadow scene node's queue of the objects whose lighting is to be redone:
// `ShadowSceneNode::AddObject` (`00b5eeb0`), `UpdateObjectLighting` (`00b5d9f0`) and the add at the
// tail of a list of counted pointers (`00a5a310`) that puts an object in it.
//
// Their world, from a seed: a shadow scene node whose two queues are each empty or hold one to
// three items (their count the items', or a word about to wrap); the queues' lock free or already
// held by this thread (once or twice); an object (now and then none) whose vtable's slot 4 (its
// delete) marks it, its count at +4 0, 1, 5 or about to wrap (then the temporary's release deletes
// it), its flags at +0x30 with bit 0 (culled) or not; the list node pool's next node, its links
// junk or clear and its old element none, the object itself, or another object counted once
// (deleted when let go) or three times. JIP's patches are all on for half the seeds, off for the
// others: nothing of JIP's is reached, and the same code runs either way.

/// Where the objects' delete slot is registered, and where it marks an object.
const LIGHT_VT_FN: u32 = 0x00ff_c000;
const DELETED: u32 = 0x3c;
/// The thread the current thread's import answers, and the queues' lock (its owner, then its
/// count).
const LOCK_THREAD: u32 = 0x1234;
const OBJECT_QUEUE_LOCK: u32 = 0x011f_9ec0;
/// The list node pool's words for its next node: its links and its old element.
const POOL_LINKS: u32 = CONTROL + 0x800;
const POOL_ELEMENT: u32 = CONTROL + 0x804;

/// What the queue meets: the interlocked imports and the current thread (plainly), the list node
/// pool (`0043a010`: a fresh node with the world's words, as a pool node left with words in it
/// would be) and the objects' delete.
fn lighting_stand_ins(e: &mut Engine) {
    e.register(0x00fd_f1c0, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x00fd_f1c4, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x00fd_f1bc, |e, a| {
        let old = e.mem.u32(a[0]);
        if old == a[2] {
            e.mem.set_u32(a[0], a[1]);
        }
        ret(old)
    });
    e.register(0x00fd_f1d0, |_, _| ret(LOCK_THREAD));
    e.register(0x0043_a010, |e, _| {
        let node = e.mem.alloc(0xc);
        let links = e.mem.u32(POOL_LINKS);
        e.mem.set_u32(node, links);
        e.mem.set_u32(node + 4, links);
        let element = e.mem.u32(POOL_ELEMENT);
        e.mem.set_u32(node + 8, element);
        ret(node)
    });
    e.register(LIGHT_VT_FN + 4, |e, a| {
        let n = e.mem.u32(a[0] + DELETED);
        e.mem.set_u32(a[0] + DELETED, n + 1);
        Ret::default()
    });
}

/// A counted object of the queue's world: its delete slot, its count and its flags.
fn lighting_object(e: &mut Engine, count: u32, flags: u8) -> u32 {
    let vt = e.mem.alloc(0x10);
    e.mem.set_u32(vt + 4, LIGHT_VT_FN + 4);
    let o = e.mem.alloc(0x40);
    e.mem.set_u32(o, vt);
    e.mem.set_u32(o + 4, count);
    e.mem.set_u8(o + 0x30, flags);
    o
}

/// A queue of none to three items (their elements words never read), its count the items' or a
/// word about to wrap.
fn lighting_queue(e: &mut Engine, d: &mut Draw, list: u32) {
    let n = d.next(4);
    let (mut head, mut tail) = (0, 0);
    for i in 0..n {
        let item = e.mem.alloc(0xc);
        e.mem.set_u32(item + 4, tail);
        e.mem.set_u32(item + 8, 0x0600_0000 + i);
        if tail == 0 {
            head = item;
        } else {
            e.mem.set_u32(tail, item);
        }
        tail = item;
    }
    e.mem.set_u32(list, head);
    e.mem.set_u32(list + 4, tail);
    e.mem
        .set_u32(list + 8, if d.next(4) == 0 { 0xffff_ffff } else { n });
}

/// The queue's world of a seed: the shadow scene node, the object (0 now and then) and the pointer
/// holding it (a word of its own, for the list's add).
fn lighting_world(e: &mut Engine, seed: u32) -> [u32; 3] {
    e.map(CONTROL, 0x1000);
    e.map(0x011f_9000, 0x1000);
    lighting_stand_ins(e);
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ 0xd9f0);
    e.set_jip_patches(if seed % 2 == 0 { 0 } else { u32::MAX });
    let node = e.mem.alloc(0x200);
    lighting_queue(e, &mut d, node + 0x108);
    lighting_queue(e, &mut d, node + 0x114);
    let (owner, count) = [(0, 0), (0, 0), (LOCK_THREAD, 1), (LOCK_THREAD, 2)][d.next(4) as usize];
    e.mem.set_u32(OBJECT_QUEUE_LOCK, owner);
    e.mem.set_u32(OBJECT_QUEUE_LOCK + 4, count);
    let object = if seed % 16 == 5 {
        0
    } else {
        let count = [0, 1, 5, 0xffff_ffff][d.next(4) as usize];
        let flags = [0, 1, 2, 3, 0xfe, 0xff][d.next(6) as usize];
        lighting_object(e, count, flags)
    };
    let links = [0, 0x5a5a_5a5a][d.next(2) as usize];
    e.mem.set_u32(POOL_LINKS, links);
    let element = match d.next(6) {
        3 => object,
        4 => lighting_object(e, 1, 0),
        5 => lighting_object(e, 3, 0),
        _ => 0,
    };
    e.mem.set_u32(POOL_ELEMENT, element);
    let pointer = e.mem.alloc(4);
    e.mem.set_u32(pointer, object);
    [node, object, pointer]
}

/// `ShadowSceneNode::AddObject` against its translation, over the seeds' worlds.
#[test]
fn shadow_scene_add_object() {
    Diff::new(entry!(
        0x00b5_eeb0,
        cell_refs::lighting::add_object(u32, u32)
    ))
    .family(&CELL_REFS)
    .family(&engine_typed::dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    // The list node pool (the draw F family's), standing in.
    .goes_by_address(&[0x0043_a010])
    .inputs(&[Input::Of(SEEDS)])
    .setup(|e, case| {
        let [node, object, _] = lighting_world(e, case.word(0));
        vec![node, object]
    })
    .run()
    .assert_same();
}

/// `ShadowSceneNode::UpdateObjectLighting` against its translation, over the seeds' worlds and the
/// flag 0, 1, 2, 0xff and a word whose low byte is 0.
#[test]
fn shadow_scene_update_object_lighting() {
    Diff::new(entry!(
        0x00b5_d9f0,
        cell_refs::lighting::update_object_lighting(u32, u32, u8)
    ))
    .family(&CELL_REFS)
    .family(&engine_typed::dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .goes_by_address(&[0x0043_a010])
    .inputs(&[Input::Of(SEEDS), Input::Of(&[0, 1, 2, 0xff, 0x100])])
    .setup(|e, case| {
        let [node, object, _] = lighting_world(e, case.word(0));
        vec![node, object, case.word(1)]
    })
    .run()
    .assert_same();
}

/// The add at the tail of a list of counted pointers against its translation: either queue of the
/// seeds' worlds and the pointer holding the object (or none).
#[test]
fn counted_list_add_tail() {
    Diff::new(entry!(
        0x00a5_a310,
        cell_refs::lighting::list_add_tail(u32, u32)
    ))
    .family(&CELL_REFS)
    .goes_by_address(&[0x0043_a010])
    .inputs(&[Input::Of(SEEDS), Input::Of(&[0x108, 0x114])])
    .setup(|e, case| {
        let [node, _, pointer] = lighting_world(e, case.word(0));
        vec![node + case.word(1), pointer]
    })
    .run()
    .assert_same();
}

/// The queue's worlds reach what it does, checked on the translation: a null object, a culled one
/// left out by `AddObject`, each queue, the flag's byte read alone, an empty queue and one with a
/// tail, a count that wraps, the lock taken anew and taken again by its owner, the node's old
/// element let go (and deleted), the same element kept, the temporary's release deleting the
/// object, junk links in the pool's node, JIP's patches on and off.
#[test]
fn the_lighting_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeMap::<&str, u32>::new();
    for &seed in SEEDS {
        for flag in [0u32, 1, 0x100] {
            let mut e = Engine::new_untyped_for_test();
            let [node, object, _] = lighting_world(&mut e, seed);
            let mut hit = |name| *reached.entry(name).or_default() += 1;
            hit(if e.jip_patches() == 0 {
                "JIP's patches off"
            } else {
                "JIP's patches on"
            });
            if object == 0 {
                hit("a null object");
                continue;
            }
            let culled = e.mem.u8(object + 0x30) & 1 != 0;
            let queue = if flag & 0xff == 0 { 0x108 } else { 0x114 };
            let (head, count) = (e.mem.u32(node + queue), e.mem.u32(node + queue + 8));
            let element = e.mem.u32(POOL_ELEMENT);
            let element_count = if element == 0 {
                0
            } else {
                e.mem.u32(element + 4)
            };
            let owner = e.mem.u32(OBJECT_QUEUE_LOCK);
            let links = e.mem.u32(POOL_LINKS);
            e.call_log = Some(Vec::new());
            if flag == 0 {
                e.call(0x00b5_eeb0, &args![node, object]);
            } else {
                e.call(0x00b5_d9f0, &args![node, object, flag]);
            }
            let log = e.call_log.take().unwrap_or_default();
            if flag == 0 && culled {
                assert!(!log.iter().any(|(a, _)| *a == 0x00b5_d9f0));
                hit("a culled object left out by AddObject");
                continue;
            }
            hit(if queue == 0x108 {
                "queued for an update"
            } else {
                "queued for an update of the active lights only"
            });
            if flag == 0x100 {
                hit("the flag's byte read alone");
            }
            hit(if head == 0 {
                "an empty queue's head set"
            } else {
                "linked after the tail"
            });
            if count == 0xffff_ffff {
                hit("the queue's count wraps");
            }
            hit(if owner == 0 {
                "the lock taken anew and left"
            } else {
                "the lock taken again by its owner"
            });
            if links != 0 {
                hit("junk links in the pool's node");
            }
            if element == object {
                hit("the same element kept");
            } else if element != 0 {
                hit(if element_count == 1 {
                    "the old element let go and deleted"
                } else {
                    "the old element let go"
                });
            }
            if e.mem.u32(object + DELETED) != 0 {
                hit("the temporary's release deletes the object");
            }
        }
    }
    for name in [
        "JIP's patches off",
        "JIP's patches on",
        "a null object",
        "a culled object left out by AddObject",
        "queued for an update",
        "queued for an update of the active lights only",
        "the flag's byte read alone",
        "an empty queue's head set",
        "linked after the tail",
        "the queue's count wraps",
        "the lock taken anew and left",
        "the lock taken again by its owner",
        "junk links in the pool's node",
        "the same element kept",
        "the old element let go and deleted",
        "the old element let go",
        "the temporary's release deletes the object",
    ] {
        assert!(
            reached.contains_key(name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}
