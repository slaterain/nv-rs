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
    let vt = e.mem.alloc(0x300);
    for slot in [0x100, 0x224, 0x1d0, 0x240, 0x260] {
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
// type (0xd, the tree, creature and actor types, two others) or the player, the reference's class
// (`TESObjectREFR`'s own slots 0x100 and 0x224, or stand-ins answering words kept in the
// reference), its word at +0xc8, a cell or none; the loader's two maps with their slots the
// lock-free family's addresses or stand-ins (the same doubles behind both: a small table of keys
// and tasks), the reference already queued (its task's priority byte the same or not), the
// allocation refused, the add refused, the owner thread and menu mode. The priority is one of four
// (one above a byte), `later` both ways.

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

/// A task as a constructor leaves it: the task vtable, no count, the priority in its key's byte 2.
fn new_task(e: &mut Engine, block: u32, kind: u32, reference: u32, priority: u32) {
    let vt = e.mem.alloc(0x40);
    for slot in [0, 0x1c, 0x20] {
        e.mem.set_u32(vt + slot, TASK_VT_FN + slot);
    }
    e.mem.set_u32(block, vt);
    e.mem.set_u32(block + 8, 0);
    e.mem
        .set_u32(block + 0x10, (priority & 0xff) << 16 | 0x1234);
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
    let kind = [0xd, 0x25, 0x2a, 0x2b, 0x1e, 0x15, 0x25, 0x2a][d.next(8) as usize];
    let base = base_form(e, kind, 0);
    e.mem.set_u32(r + 0x20, base);
    e.mem.set_u32(r + 0xc8, [0, 2, 3, 4][d.next(4) as usize]);
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
            if e.mem.u32(queued + MAP_LAST_FLAG) == 0 {
                // No add: an early end, or the reference found (its task's count back to the
                // map's alone).
                let flags = e.mem.u32(e.global::<u32>(GAME_LOADER_POINTER) + 0x244);
                let base = e.mem.u32(r + 0x20);
                let kind = if e.mem.u32(e.mem.u32(r) + 0x224) == 0x0057_25b0 {
                    let k = e.mem.u8(r + 4);
                    (0x3d..=0x40).contains(&k) || k == 0x69
                } else {
                    e.mem.u32(r + ANSWER_KIND) != 0
                };
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
            hit(["player", "tree", "creature", "actor", "other"]
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
        "creature",
        "actor",
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
